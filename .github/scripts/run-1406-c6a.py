#!/usr/bin/env python3
"""#1406 C6a Linux 外层监督候选；PENDING，禁止实际运行。

资源来源及 guardian 合同见 c6a-contract-recovery-v2.md。所有大文件处理、
git、验证与归档在持有进程组身份的 worker 内执行；主控保留 WNOWAIT leader。
只发行本文件与 issue-1406-c6a.yml，不依赖另一个准备件。
"""
from __future__ import annotations

import argparse
import array
import ctypes
from dataclasses import dataclass
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from typing import Literal, TypedDict, cast

CONTRACT_READY = True
REVIEWED_SOURCE = '90d301f97f5fba5bd7a215d08b4352796988baf2'
REVIEWED_TREE = 'c03e57f7984d7d8b9e588ed3a02ae0d7c36ba830'
SEMANTICS_READY = True
CANDIDATE_SOURCE = '90d301f97f5fba5bd7a215d08b4352796988baf2'
CANDIDATE_TREE = 'c03e57f7984d7d8b9e588ed3a02ae0d7c36ba830'
BINDER_SNAPSHOT_SHA256 = "9f8a6a40a112d91982fe0093e0fdb2f39dd60c757032a16a0ee6c34dd59dca25"
# 上述准备件只作接口参考，未导入；内嵌语义候选仍须独审后才可打开READY。
CI_FILES = frozenset({".github/scripts/run-1406-c6a.py", ".github/workflows/issue-1406-c6a.yml"})
ENDPOINTS = {"build": 1200, "runtime": 1830, "verify": 1920, "archive": 2040, "final": 2070}
GIB, MIB = 1024 ** 3, 1024 ** 2
RESOURCE_LIMITS = {"initial_free": 8 * GIB, "initial_available": 3 * GIB,
                   "build_rss": 5 * GIB, "runtime_rss": 2 * GIB,
                   "min_free": GIB, "min_available": 512 * MIB, "max_growth": 6 * GIB}
POLL_SECONDS = 0.25
TERM_SECONDS, KILL_SECONDS = 3.0, 3.0
# 主任务须给同阶段回收留位置；不增加任一原阶段上限。
REAP_RESERVE = TERM_SECONDS + KILL_SECONDS + POLL_SECONDS
BINARIES = ("e_economic_session", "b_chong_session")
CANCELLATION = {"count": 0, "first_signal": None}
SEALED_CONTEXT = None
ANCHOR_SCHEMA = "c6a-clock/1"
ANCHOR_OPS: tuple[ClockOp, ...] = ("work", "we5-case", "we5-q", "we7-case", "we7-q")
ANCHOR_BYTES = 1024


ClockOp = Literal["work", "we5-case", "we5-q", "we7-case", "we7-q"]
CaseName = Literal["we5", "we7"]


class WorkAnchor(TypedDict):
    t0: float
    work_deadline: float
    final_deadline: float


class CaseAnchor(TypedDict):
    started_at: float
    case_deadline: float
    work_deadline: float


class QAnchor(CaseAnchor):
    deadline: float


ClockAnchor = WorkAnchor | CaseAnchor | QAnchor


class ClockRequest(TypedDict):
    schema: str
    run_id: str
    seq: int
    op: ClockOp


class ClockResponse(ClockRequest):
    anchor: ClockAnchor


class ClockRow(TypedDict):
    request: ClockRequest
    response: ClockResponse
    received: float
    issued: float
    sent: float | None


class PendingClockResponse(TypedDict):
    raw: bytes
    row: ClockRow
    send_deadline: float


class ClockTranscript(TypedDict):
    schema: str
    run_id: str
    driver_pid: int | None
    started: float | None
    outer_deadline: float | None
    rows: list[ClockRow]
    eof: bool
    failure: str | None
    complete: bool


# 可空字段描述尚未 arm、尚未发送或失败状态，不能当作成功。
# 验证器输入穿过 JSON 信任边界，故入口为 object，校验后才收窄。
CaseStartupAnchors = dict[CaseName, CaseAnchor]


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def clock() -> float:
    return time.clock_gettime(time.CLOCK_MONOTONIC)


def remember_signal(number, _frame=None):
    CANCELLATION["count"] += 1
    if CANCELLATION["first_signal"] is None:
        CANCELLATION["first_signal"] = number
    if SEALED_CONTEXT is not None:
        # 最终收据已发布且无进程所有权动作在途；封口后取消仍强制非零退出。
        try:
            save(SEALED_CONTEXT / "CANCELLED-AFTER-SEAL.json", {"signals": dict(CANCELLATION), "accepted": False})
        finally:
            raise SystemExit(1)


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def parse_json(raw):
    def pairs(values):
        out = {}
        for key, value in values:
            require(key not in out, "JSON duplicate key")
            out[key] = value
        return out
    def number(value):
        result = float(value)
        require(math.isfinite(result), "JSON nonfinite number")
        return result
    return json.loads(raw, object_pairs_hook=pairs, parse_float=number,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def small_json(path):
    # 主线程只读取有限收据；大内容解析留给阶段 worker。
    with Path(path).open("rb") as stream:
        raw = stream.read(16385)
    require(len(raw) <= 16384, "receipt exceeds 16 KiB")
    return parse_json(raw)


def save(path, value):
    path = Path(path)
    temporary = path.with_name("." + path.name + ".tmp")
    with temporary.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, allow_nan=False, sort_keys=True)
        stream.write("\n")
    try:
        os.link(temporary, path)  # 新文件、同卷原子发布，禁止覆盖。
    finally:
        temporary.unlink()


def regular_files(root, *, special=None, base=None, exclude=None, excluded=None):
    base = Path(root) if base is None else base
    require(stat.S_ISDIR(Path(root).lstat().st_mode), "not a plain directory")
    with os.scandir(root) as iterator:
        entries = sorted(iterator, key=lambda entry: entry.name)
    for entry in entries:
        relative = str(Path(entry.path).relative_to(base))
        # 排除当前封口/探针命名空间必须先于DirEntry.stat；发布者可随时unlink临时名。
        if exclude is not None and exclude(relative):
            if excluded is not None:
                excluded.append(relative)
            continue
        mode = entry.stat(follow_symlinks=False).st_mode
        if stat.S_ISDIR(mode):
            yield from regular_files(Path(entry.path), special=special, base=base,
                                     exclude=exclude, excluded=excluded)
        elif stat.S_ISREG(mode):
            yield Path(entry.path)
        elif stat.S_ISSOCK(mode):
            if special is not None:
                special.append({"path": str(Path(entry.path).relative_to(base)), "mode": mode, "socket": True})
        else:
            raise RuntimeError("symlink or unsupported evidence entry: " + entry.path)


def snapshot(groups, filesystem):
    """仅 worker 读 /proc；父监督器给此观测同阶段绝对截止。"""
    rows = []
    page = os.sysconf("SC_PAGE_SIZE")
    for path in Path("/proc").iterdir():
        if not path.name.isdigit():
            continue
        try:
            fields = (path / "stat").read_text().rsplit(") ", 1)[1].split()
        except (FileNotFoundError, ProcessLookupError):
            continue
        pgid = int(fields[2])
        if pgid in groups or pgid == os.getpgrp():
            rows.append({"pid": int(path.name), "pgid": pgid, "state": fields[0],
                         "rss": int(fields[21]) * page})
    available = None
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            available = int(line.split()[1]) * 1024
    require(available is not None, "missing MemAvailable")
    return {"live": any(row["state"] != "Z" and row["pgid"] in groups for row in rows),
            "rss": sum(row["rss"] for row in rows),
            "free": shutil.disk_usage(filesystem).free, "available": available}


def resources_ok(sample, initial_free, rss_limit):
    require(sample["free"] >= RESOURCE_LIMITS["min_free"], "disk reserve")
    require(initial_free - sample["free"] <= RESOURCE_LIMITS["max_growth"], "filesystem growth")
    require(sample["available"] >= RESOURCE_LIMITS["min_available"], "available memory")
    require(sample["rss"] <= rss_limit, "group RSS")


def anchor_packet(sock: socket.socket) -> bytes:
    """一个有界packet；收到意外SCM_RIGHTS也必须关闭，不能泄漏描述符。"""
    raw, ancillary, flags, _ = sock.recvmsg(ANCHOR_BYTES, socket.CMSG_SPACE(16 * array.array("i").itemsize))
    for level, kind, data in ancillary:
        if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
            values = array.array("i")
            values.frombytes(data[:len(data) - len(data) % values.itemsize])
            for fd in values:
                os.close(fd)
    require(not ancillary and not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC), "invalid clock packet ancillary/truncation")
    return raw


def anchor_encode(value: ClockRequest | ClockResponse) -> bytes:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    require(len(raw) <= ANCHOR_BYTES, "clock packet oversized")
    return raw


class ClockBroker:
    """父进程唯一签发者；tick不阻塞、不写盘，每次至多一收一发。"""
    def __init__(self, parent: socket.socket, child: socket.socket, run_id: str) -> None:
        self.parent = parent
        self.child: socket.socket | None = child
        self.run_id = run_id
        self.parent.setblocking(False)
        self.child.setblocking(False)
        self.parent.set_inheritable(False)
        self.child.set_inheritable(False)
        self.started: float | None = None
        self.pid: int | None = None
        self.outer_deadline: float | None = None
        self.rows: list[ClockRow] = []
        self.pending: PendingClockResponse | None = None
        self.case_end: float | None = None
        self.work_end: float | None = None
        self.final_end: float | None = None
        self.failure: str | None = None
        self.eof, self.closed = False, False

    @classmethod
    def create(cls) -> ClockBroker:
        parent, child = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        try:
            return cls(parent, child, os.urandom(16).hex())
        except BaseException:
            parent.close()
            child.close()
            raise

    def arm(self, started: float, outer_deadline: float) -> None:
        require(self.started is None, "clock work anchor already armed")
        self.started, self.outer_deadline = started, outer_deadline
        self.work_end, self.final_end = started + 600, started + 630
        self.case_end = self.work_end

    def child_args(self) -> list[str]:
        if self.started is None or self.child is None:
            raise RuntimeError("clock broker not armed")
        return ["--anchor-fd", str(self.child.fileno()), "--anchor-run-id", self.run_id,
                "--anchor-work-deadline", repr(self.work_end)]

    def child_closed(self) -> None:
        if self.child is not None:
            self.child.close()
            self.child = None

    def reject(self, error: Exception | str) -> None:
        if self.failure is None:
            self.failure = str(error)

    def tick(self) -> None:
        if self.closed or self.failure or self.eof:
            return
        try:
            require(not CANCELLATION["count"], "clock cancelled")
            if (self.started is None or self.pid is None or self.outer_deadline is None
                    or self.case_end is None or self.work_end is None or self.final_end is None):
                raise RuntimeError("clock owner not registered")
            now = clock()
            require(now < self.outer_deadline, "clock outer deadline")
            # 最后一次应答发出后只等EOF，不把Q的10秒误当整个case工作期限。
            if self.pending is not None:
                require(now < self.pending["send_deadline"], "clock pending response expired")
            else:
                try:
                    raw = anchor_packet(self.parent)
                except BlockingIOError:
                    raw = None
                if raw == b"":
                    require(len(self.rows) == len(ANCHOR_OPS) and all(r["sent"] is not None for r in self.rows),
                            "clock premature EOF")
                    self.eof = True
                    return
                if raw is not None:
                    require(len(self.rows) < len(ANCHOR_OPS), "clock extra request")
                    index = len(self.rows)
                    request = parse_json(raw)
                    expected: ClockRequest = {"schema": ANCHOR_SCHEMA, "run_id": self.run_id,
                                "seq": index + 1, "op": ANCHOR_OPS[index]}
                    require(type(request) is dict and request == expected and type(request.get("seq")) is int,
                            "clock order/schema/run mismatch")
                    issued = clock()
                    require(not CANCELLATION["count"] and issued < min(self.case_end, self.work_end, self.outer_deadline),
                            "clock expired previous budget or cancelled")
                    op = ANCHOR_OPS[index]
                    anchor: ClockAnchor
                    if op == "work":
                        anchor = {"t0": self.started, "work_deadline": self.work_end, "final_deadline": self.final_end}
                        send_end = self.work_end
                    elif op.endswith("-case"):
                        self.case_end = min(self.work_end, issued + 300)
                        anchor = {"started_at": issued, "case_deadline": self.case_end, "work_deadline": self.work_end}
                        send_end = self.case_end
                    else:
                        send_end = min(issued + 10, self.case_end, self.work_end)
                        anchor = {"started_at": issued, "deadline": send_end,
                                  "case_deadline": self.case_end, "work_deadline": self.work_end}
                    response: ClockResponse = {**expected, "anchor": anchor}
                    row: ClockRow = {"request": cast(ClockRequest, request), "response": response, "received": now, "issued": issued, "sent": None}
                    self.rows.append(row)
                    self.pending = {"raw": anchor_encode(response), "row": row,
                                    "send_deadline": min(send_end, self.outer_deadline)}
            if self.pending is not None:
                require(not CANCELLATION["count"] and clock() < self.pending["send_deadline"], "clock send expired/cancelled")
                try:
                    count = self.parent.send(self.pending["raw"])
                except BlockingIOError:
                    return
                require(count == len(self.pending["raw"]), "clock partial seqpacket send")
                sent = clock()
                self.pending["row"]["sent"] = sent
                require(not CANCELLATION["count"] and sent < self.pending["send_deadline"],
                        "clock response late/cancelled")
                self.pending = None
        except Exception as error:
            self.reject(error)

    def complete(self) -> bool:
        return self.failure is None and self.eof and len(self.rows) == 5 and self.pending is None

    def close(self) -> None:
        self.child_closed()
        if not self.closed:
            self.parent.close()
            self.closed = True

    def transcript(self) -> ClockTranscript:
        return {"schema": ANCHOR_SCHEMA, "run_id": self.run_id, "driver_pid": self.pid,
                "started": self.started, "outer_deadline": self.outer_deadline, "rows": self.rows,
                "eof": self.eof, "failure": self.failure, "complete": self.complete()}


def clock_mapping(value: object, detail: str) -> dict[str, object]:
    if type(value) is not dict or not all(type(key) is str for key in value):
        raise RuntimeError(detail)
    return cast(dict[str, object], value)


def finite_clock_number(value: object) -> float:
    if type(value) not in (int, float):
        raise RuntimeError("clock nonnumeric value")
    number = cast(float, value)
    require(math.isfinite(number), "clock nonfinite value")
    return number


def verify_clock_transcript(value: object, started: float, finished: float) -> CaseStartupAnchors:
    """只读父进程保存的见证，独算期限；不读取driver proof来补锚。"""
    transcript = clock_mapping(value, "clock transcript type")
    require(set(transcript) == {"schema", "run_id", "driver_pid", "started", "outer_deadline", "rows",
                                "eof", "failure", "complete"}, "clock transcript fields")
    require(transcript["schema"] == ANCHOR_SCHEMA and transcript["failure"] is None
            and transcript["complete"] is True and transcript["eof"] is True, "clock transcript incomplete")
    started, finished = finite_clock_number(started), finite_clock_number(finished)
    origin = finite_clock_number(transcript["started"])
    outer_deadline = finite_clock_number(transcript["outer_deadline"])
    require(origin == started < finished < outer_deadline, "clock runtime bounds")
    pid, run_id = transcript["driver_pid"], transcript["run_id"]
    if type(pid) is not int or pid <= 0 or type(run_id) is not str or not re.fullmatch("[0-9a-f]{32}", run_id):
        raise RuntimeError("clock owner/run")
    raw_rows = transcript["rows"]
    if type(raw_rows) is not list or len(raw_rows) != 5:
        raise RuntimeError("clock row count")
    rows = cast(list[object], raw_rows)
    work_end, case_end, previous = started + 600, started + 600, started
    result: CaseStartupAnchors = {}
    for index, raw_row in enumerate(rows):
        row = clock_mapping(raw_row, "clock row type")
        require(set(row) == {"request", "response", "received", "issued", "sent"}, "clock row fields")
        received, issued, sent = (finite_clock_number(row[key]) for key in ("received", "issued", "sent"))
        require(previous <= received <= issued <= sent < finished, "clock row timing")
        expected: ClockRequest = {"schema": ANCHOR_SCHEMA, "run_id": run_id, "seq": index + 1, "op": ANCHOR_OPS[index]}
        request = clock_mapping(row["request"], "clock request type")
        require(request == expected and type(request.get("seq")) is int, "clock request identity")
        require(issued < min(case_end, work_end), "clock previous deadline")
        anchor: ClockAnchor
        if index == 0:
            anchor = {"t0": started, "work_deadline": work_end, "final_deadline": started + 630}
            end = work_end
        elif index in (1, 3):
            case_end = min(work_end, issued + 300)
            anchor = {"started_at": issued, "case_deadline": case_end, "work_deadline": work_end}
            end = case_end
        else:
            end = min(issued + 10, case_end, work_end)
            anchor = {"started_at": issued, "deadline": end, "case_deadline": case_end, "work_deadline": work_end}
            result["we5" if index == 2 else "we7"] = {"started_at": issued, "case_deadline": case_end, "work_deadline": work_end}
        response = clock_mapping(row["response"], "clock response type")
        require(type(response.get("seq")) is int, "clock response sequence type")
        require(response == {**expected, "anchor": anchor} and sent < min(end, outer_deadline),
                "clock response/deadline mismatch")
        for number in clock_mapping(response["anchor"], "clock anchor type").values():
            finite_clock_number(number)
        previous = sent
    return result


@dataclass
class Owned:
    process: object
    name: str
    started: float
    exited: float | None = None
    identity_lost: bool = False
    argv: object = None
    cleanup: dict | None = None


class LinuxSupervisor:
    """单线程唯一wait者；信号handler只记状态，OWNED保留到最后一次组信号之后。"""
    def __init__(self, root, artifacts, filesystem, env, t0, initial_free=None):
        self.root, self.artifacts, self.filesystem = Path(root), Path(artifacts), Path(filesystem)
        self.env, self.t0, self.initial_free = dict(env), t0, initial_free
        self.owned, self.serial = {}, 0
        self.receipts = []
        self.active_broker: ClockBroker | None = None

    def end(self, stage):
        return self.t0 + ENDPOINTS[stage]

    def observe_exit(self, owned):
        require(not owned.identity_lost and owned.process.returncode is None, "lost/reaped leader")
        try:
            result = os.waitid(os.P_PID, owned.process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        except ChildProcessError:
            owned.identity_lost = True
            raise RuntimeError("leader ownership lost; no group signals allowed")
        if result is not None and owned.exited is None:
            owned.exited = clock()
        return result

    def spawn(self, name: str, argv: list[str], *, broker: ClockBroker | None = None,
              deadline: float | None = None) -> Owned:
        self.serial += 1
        name = f"{self.serial:05d}-{name}"
        started = clock()
        pass_fds: tuple[int, ...] = ()
        if broker is not None:
            if deadline is None or broker.child is None:
                raise RuntimeError("clock spawn requires deadline and child endpoint")
            broker.arm(started, deadline)
            pass_fds = (broker.child.fileno(),)
            argv = [*argv, *broker.child_args()]
        with (self.artifacts / (name + ".stdout")).open("xb") as out, (self.artifacts / (name + ".stderr")).open("xb") as err:
            try:
                proc = subprocess.Popen(argv, cwd=self.root, env=self.env, stdin=subprocess.DEVNULL,
                                        stdout=out, stderr=err, start_new_session=True,
                                        close_fds=True, pass_fds=pass_fds)
                owned = Owned(proc, name, started, argv=argv)
                self.owned[proc.pid] = owned
                if broker is not None:
                    broker.pid = proc.pid
            finally:
                if broker is not None:
                    broker.child_closed()
        return owned

    def clock_tick(self) -> None:
        if self.active_broker is not None:
            self.active_broker.tick()
            require(self.active_broker.failure is None, "clock broker: " + str(self.active_broker.failure))

    def send(self, owned, number, events):
        # WNOWAIT是所有权证明；不能用poll()、名字或历史PGID代替。
        self.observe_exit(owned)
        try:
            os.killpg(owned.process.pid, number)
            events.append({"signal": int(number), "at": clock(), "pgid": owned.process.pid})
            return True
        except ProcessLookupError:
            events.append({"signal_esrch": int(number), "at": clock()})
            return False

    def wait_group(self, owned, result, wait_end):
        proc = owned.process
        try:
            require(not owned.identity_lost and proc.returncode is None, "lost/reaped leader")
            while True:
                remaining = wait_end - clock()
                require(remaining > 0, "cleanup deadline before wait")
                try:
                    proc.wait(timeout=remaining)
                    break
                except InterruptedError:
                    continue
            # 已reap以后只观察/收养后代，绝不再向此历史组发信号。
            while clock() < wait_end:
                try:
                    child, status = os.waitpid(-proc.pid, os.WNOHANG)
                except ChildProcessError:
                    child = 0
                except InterruptedError:
                    continue
                if child:
                    result["events"].append({"reaped_descendant": child, "status": status})
                    continue
                try:
                    os.killpg(proc.pid, 0)
                except ProcessLookupError:
                    result["complete"] = True
                    self.owned.pop(proc.pid, None)
                    break
                except InterruptedError:
                    continue
                remaining = wait_end - clock()
                if remaining > 0:
                    try:
                        time.sleep(min(POLL_SECONDS, remaining))
                    except InterruptedError:
                        continue
            require(result["complete"], "group remains or cleanup deadline")
        except Exception as error:
            result["errors"].append(str(error))

    def reap(self, owned, deadline, *, graceful):
        if owned.cleanup is not None:
            return owned.cleanup  # 只复用该Owned实例的收据，不重复wait或猜历史PID。
        targets = [owned] if graceful else [item for item in self.owned.values() if item.cleanup is None]
        cohort = [{"name": item.name, "pid": item.process.pid} for item in targets]
        term_end = min(deadline, clock() + TERM_SECONDS)
        for item in targets:
            item.cleanup = {"name": item.name, "pid": item.process.pid,
                "complete": False, "forced": not graceful, "events": [], "errors": [],
                "deadline": deadline, "cohort": cohort}
        if not graceful:
            # 嵌套probe失败时连同outer统一回收，共享同一TERM/KILL窗，不逐组续租。
            for item in targets:
                result = item.cleanup
                try:
                    require(clock() < deadline, "cleanup deadline before TERM")
                    self.send(item, signal.SIGTERM, result["events"])
                except Exception as error:
                    result["errors"].append(str(error))
            while True:
                remaining = term_end - clock()
                if remaining <= 0:
                    break
                try:
                    time.sleep(min(POLL_SECONDS, remaining))
                except InterruptedError:
                    continue  # 信号中断不跳过KILL，仍核同一绝对截止。
            for item in targets:
                try:
                    require(clock() < deadline, "cleanup deadline before KILL")
                    self.send(item, signal.SIGKILL, item.cleanup["events"])
                except Exception as error:
                    item.cleanup["errors"].append(str(error))
        wait_end = min(deadline, clock() + KILL_SECONDS)
        for item in targets:
            self.wait_group(item, item.cleanup, wait_end)
            if item.cleanup["errors"]:
                item.cleanup["error"] = "; ".join(item.cleanup["errors"])
        return owned.cleanup

    def probe(self, groups, deadline, *, allow_cancel=True):
        # 观测helper独立组；受当前阶段截止且不递归监测。小收据不带完整进程清单。
        output = self.artifacts / f"probe-{self.serial + 1:05d}.json"
        request = {"groups": list(groups), "filesystem": str(self.filesystem), "output": str(output)}
        # 保守收紧观测helper：活动最多一个原采样周期，回收仍包含在原阶段截止。
        probe_end = min(deadline, clock() + REAP_RESERVE + POLL_SECONDS)
        receipt = self.run("resources", self.worker_command("resources", request), probe_end,
                           monitor=False, allow_cancel=allow_cancel)
        sample = small_json(output)
        require(receipt["exit"] == 0, "resource helper failed")
        return sample

    def worker_command(self, operation, request):
        path = self.artifacts / f"request-{self.serial + 1:05d}-{operation}.json"
        save(path, request)
        return [sys.executable, "-B", str(Path(__file__).resolve()), "--worker", operation, str(path)]

    def run(self, name, argv, deadline, *, monitor=True, allow_cancel=True, rss_limit=None,
            broker: ClockBroker | None = None):
        require(clock() < deadline - REAP_RESERVE, "insufficient same-stage cleanup reserve")
        if allow_cancel:
            require(not CANCELLATION["count"], "cancelled before spawn")
        if broker is not None:
            require(self.active_broker is None and name == "runtime", "only runtime owns clock broker")
            self.active_broker = broker
        try:
            owned = self.spawn(name, argv, broker=broker, deadline=deadline) if broker is not None else self.spawn(name, argv)
        except BaseException:
            if broker is not None:
                broker.close()
                self.active_broker = None
            raise
        active_end = deadline - REAP_RESERVE
        receipt = {"name": owned.name, "argv": owned.argv or argv, "started": owned.started, "deadline": deadline,
                   "exit": None, "exited": None, "failure": None, "cleanup": None}
        graceful = False
        try:
            while True:
                require(clock() < active_end, "phase active deadline")
                if allow_cancel:
                    require(not CANCELLATION["count"], "cancelled")
                # probe递归run(monitor=False)也推进同一有界状态机，不阻塞资源监督。
                self.clock_tick()
                if monitor:
                    # probe共享该绝对期限，不从观测完成时重新打开窗口。
                    sample = self.probe([owned.process.pid], deadline, allow_cancel=allow_cancel)
                    resources_ok(sample, self.initial_free, rss_limit or RESOURCE_LIMITS["runtime_rss"])
                    # 记录resource helper输出，保留完整采样序列；仅本组无活成员时正常reap。
                    if self.observe_exit(owned) is not None:
                        # 第一次采样可能发生在leader退出之前；退出后再观察后代，避免旧样本误判。
                        sample = self.probe([owned.process.pid], deadline, allow_cancel=allow_cancel)
                        resources_ok(sample, self.initial_free, rss_limit or RESOURCE_LIMITS["runtime_rss"])
                        graceful = not sample["live"]
                        require(graceful, "leader exited with live descendants")
                        if broker is not None:
                            self.clock_tick()  # 收最终EOF；不是等待/续租。
                            require(broker.complete(), "clock handshake incomplete at runtime exit")
                        break
                elif self.observe_exit(owned) is not None:
                    # 资源helper仅纯Python读取，不派子进程；退出后允许正常reap。
                    graceful = True
                    if broker is not None:
                        self.clock_tick()
                        require(broker.complete(), "clock handshake incomplete at runtime exit")
                    break
                time.sleep(min(POLL_SECONDS if monitor else 0.01, max(0.0, active_end - clock())))
        except Exception as error:
            receipt["failure"] = str(error)
        finally:
            if broker is not None:
                if receipt["failure"] is not None:
                    broker.reject(receipt["failure"])
                # 先停签发再回收；cleanup不会为了完成握手阻塞TERM/KILL/最后wait。
                broker.close()
                self.active_broker = None
            receipt["cleanup"] = self.reap(owned, deadline, graceful=graceful)
            receipt["exit"], receipt["exited"] = owned.process.returncode, owned.exited
            receipt["finished"] = clock()
            receipt["signals"] = dict(CANCELLATION)
            if broker is not None:
                if CANCELLATION["count"]:
                    broker.reject("cancelled during runtime cleanup")
                receipt["clock_transcript"] = broker.transcript()
            receipt["pass"] = (receipt["failure"] is None and receipt["exit"] == 0
                and receipt["cleanup"]["complete"] and not receipt["cleanup"]["forced"]
                and receipt["finished"] < deadline and (not allow_cancel or not CANCELLATION["count"]))
            save(self.artifacts / (owned.name + ".receipt.json"), receipt)
            self.receipts.append(receipt)
        require(receipt["pass"], "supervised stage failed: " + owned.name)
        return receipt


def identity(root):
    def git(*args):
        # 本worker继承外层进程组；整个git helper仍受同一阶段监督。
        return subprocess.check_output(["git", *args], cwd=root, text=True).strip()
    require(REVIEWED_SOURCE != "PENDING" and REVIEWED_TREE != "PENDING", "source pending")
    head = git("rev-parse", "HEAD")
    require(head == os.environ.get("GITHUB_SHA") == os.environ.get("EXPECTED_SHA"), "workflow SHA mismatch")
    require(not git("status", "--porcelain", "--untracked-files=all"), "dirty checkout")
    require(git("rev-parse", REVIEWED_SOURCE + "^{tree}") == REVIEWED_TREE, "reviewed source tree")
    subprocess.run(["git", "merge-base", "--is-ancestor", REVIEWED_SOURCE, head], cwd=root, check=True)
    require(set(git("diff", "--name-only", REVIEWED_SOURCE, head).splitlines()) <= CI_FILES, "unreviewed source delta")
    return {"source_sha": head, "source_tree": git("rev-parse", "HEAD^{tree}")}


def driver_module(root):
    sys.path.insert(0, str(Path(root) / "s_session/tests"))
    import tb03c_query_runtime
    return tb03c_query_runtime


def build_evidence(root, binaries, command, exit_code, source):
    current = identity(root)
    require(current == source and type(exit_code) is int and exit_code == 0, "build identity/exit")
    driver = driver_module(root)
    evidence = {**current, "command": command, "exit_code": exit_code,
                "source_files": {name: digest(Path(root) / name) for name in driver.SOURCE_FILES},
                "binaries": {name: digest(Path(binaries) / name) for name in BINARIES}}
    driver.verify_build(evidence, current["source_sha"], current["source_tree"],
                        evidence["source_files"], evidence["binaries"])
    for name, expected in driver.INPUT_HASHES.items():
        require(digest(Path(root) / "s_session/tests" / name) == expected, "frozen input changed")
    return evidence


SEMANTICS_SOURCE = '"""#1406 C6a 离线语义候选。只读取；不启动产品、不签最终 PASS。\n\n可整体嵌入发行 runner 的独立 namespace，入口 verify_runtime。\n"""\nfrom __future__ import annotations\n\nimport ast\nimport base64\nimport re\nimport builtins\nimport copy\nimport io\nimport importlib.machinery\nimport pathlib\nimport hashlib\nimport importlib.util\nimport json\nimport locale\nimport math\nfrom pathlib import Path\nimport signal\nimport sys\nimport types\nfrom unittest.mock import patch\n\n\ndef require(value, message):\n    if not value:\n        raise ValueError(message)\n\n\ndef equal(actual, expected, message):\n    require(actual == expected, message)\n\n\ndef finite(value):\n    return type(value) in (int, float) and math.isfinite(value)\n\n\ndef strict_log_json(raw):\n    def pairs(items):\n        result = {}\n        for key, value in items:\n            require(key not in result, \'日志重复键\')\n            result[key] = value\n        return result\n    def number(value):\n        parsed = float(value)\n        require(math.isfinite(parsed), \'日志非有限数\')\n        return parsed\n    def constant(value):\n        raise ValueError(\'日志非法数值 \' + value)\n    return json.loads(raw, object_pairs_hook=pairs, parse_float=number, parse_constant=constant)\n\n\ndef digest_file(path):\n    return hashlib.sha256(Path(path).read_bytes()).hexdigest()\n\n\ndef load_driver(checkout, source_hashes):\n    """先核全部执行/数据字节，再在不碰全局 import 缓存的私有模块图执行。"""\n    checkout = Path(checkout).resolve()\n    buffers = {}\n    def freeze(relative, wanted):\n        require(type(relative) is str and not Path(relative).is_absolute() and\n                \'..\' not in Path(relative).parts, \'源码路径越界\')\n        path = (checkout / relative).resolve()\n        require(path.is_relative_to(checkout), \'源码 symlink 越界\')\n        raw = path.read_bytes()\n        equal(hashlib.sha256(raw).hexdigest(), wanted, \'执行前冻结字节摘要不同 \' + relative)\n        buffers[path] = raw\n        return raw\n    for relative, wanted in source_hashes.items():\n        freeze(relative, wanted)\n    driver_path = checkout / \'s_session/tests/tb03c_query_runtime.py\'\n    require(driver_path in buffers, \'缺 driver 源码锚\')\n    tree = ast.parse(buffers[driver_path], filename=str(driver_path))\n    constants = {}\n    for node in tree.body:\n        if isinstance(node, ast.Assign):\n            for target in node.targets:\n                if isinstance(target, ast.Name) and target.id in (\'SOURCE_FILES\', \'INPUT_HASHES\'):\n                    constants[target.id] = ast.literal_eval(node.value)\n    equal(set(source_hashes), set(constants[\'SOURCE_FILES\']), \'精确 11 源码集合\')\n    require(len(source_hashes) == 11, \'源码数量非 11\')\n    for relative, wanted in constants[\'INPUT_HASHES\'].items():\n        freeze(\'s_session/tests/\' + relative, wanted)\n    paths = {\'c6a_bound_driver\': driver_path,\n             \'s_socket_client\': checkout / \'s_session/s_socket_client.py\',\n             \'tb03c_oracle\': checkout / \'s_session/tests/tb03c_oracle.py\'}\n    # 所有冻结可执行 oracle 一并先核、先审 import，含 importlib 动态加载者。\n    executable = {driver_path, paths[\'s_socket_client\']} | {\n        checkout / \'s_session/tests\' / relative\n        for relative in constants[\'INPUT_HASHES\'] if relative.endswith(\'.py\')}\n    standard = sys.stdlib_module_names\n    for path in executable:\n        require(path in buffers, \'递归执行依赖未冻结\')\n        parsed = ast.parse(buffers[path], filename=str(path))\n        for node in ast.walk(parsed):\n            if isinstance(node, ast.Import):\n                names = [alias.name for alias in node.names]\n            elif isinstance(node, ast.ImportFrom):\n                require(node.level == 0, \'未绑定相对递归 import\')\n                names = [node.module]\n            else:\n                continue\n            for name in names:\n                require(name.split(\'.\')[0] in standard or name in paths,\n                        \'未绑定递归执行依赖 \' + name)\n    private_modules = {}\n    private_sys = types.ModuleType(\'sys\')\n    private_sys.__dict__.update(vars(sys))\n    private_sys.path = list(sys.path)\n    private_sys.modules = dict(sys.modules)\n\n    class FrozenPath(type(Path())):\n        def open(self, mode=\'r\', buffering=-1, encoding=None, errors=None, newline=None):\n            raw = buffers.get(Path(self).resolve())\n            if raw is not None and mode in (\'r\', \'rb\', \'rt\'):\n                if \'b\' in mode:\n                    return io.BytesIO(raw)\n                return io.StringIO(raw.decode(locale.getencoding() if encoding in (None, \'locale\') else encoding, errors or \'strict\'), newline=newline)\n            return super().open(mode, buffering, encoding, errors, newline)\n    private_pathlib = types.ModuleType(\'pathlib\')\n    private_pathlib.__dict__.update(vars(pathlib))\n    private_pathlib.Path = FrozenPath\n\n    def execute(module, path):\n        require(path in executable, \'动态执行路径未冻结\')\n        module.__file__, module.__package__ = str(path), \'\'\n        module.__dict__[\'__builtins__\'] = {**vars(builtins), \'__import__\': isolated_import}\n        exec(compile(buffers[path], str(path), \'exec\'), module.__dict__)\n        return module\n\n    class FrozenLoader:\n        def __init__(self, path):\n            self.path = path\n        def create_module(self, spec):\n            return None\n        def exec_module(self, module):\n            execute(module, self.path)\n    def frozen_spec(name, location, *args, **kwargs):\n        require(not args and not kwargs, \'动态加载参数未绑定\')\n        path = Path(location).resolve()\n        require(path in executable, \'动态执行依赖未冻结\')\n        return importlib.machinery.ModuleSpec(name, FrozenLoader(path), origin=str(path))\n    private_util = types.ModuleType(\'importlib.util\')\n    private_util.__dict__.update(vars(importlib.util))\n    private_util.spec_from_file_location = frozen_spec\n    private_importlib = types.ModuleType(\'importlib\')\n    private_importlib.__dict__.update(vars(importlib))\n    private_importlib.util = private_util\n\n    def isolated_import(name, globals=None, locals=None, fromlist=(), level=0):\n        require(level == 0, \'隔离模块不允许未绑定相对 import\')\n        if name in paths:\n            if name not in private_modules:\n                module = types.ModuleType(name)\n                private_modules[name] = module\n                execute(module, paths[name])\n            return private_modules[name]\n        if name == \'sys\':\n            return private_sys\n        if name == \'pathlib\':\n            return private_pathlib\n        if name in (\'importlib\', \'importlib.util\'):\n            return private_util if name.endswith(\'.util\') and fromlist else private_importlib\n        require(name.split(\'.\')[0] in standard, \'执行期未绑定递归依赖 \' + name)\n        return builtins.__import__(name, globals, locals, fromlist, level)\n    return isolated_import(\'c6a_bound_driver\')\n\n\ndef binder_for(driver):\n    namespace = {\'driver\': driver}\n    exec(BINDER_CODE, namespace)\n    return types.SimpleNamespace(**namespace)\n\n\ndef expected_steps(driver, seed, plan, oracle):\n    """完全从冻结原件枚举 20+21 个主写请求，不从运行回执取期望。"""\n    rows = {\'we5\': [], \'we7\': []}\n    recorded = {}\n    for index, step in enumerate(plan[\'steps\']):\n        domain, payload = driver.we5_payload(step, recorded)\n        rows[\'we5\'].append({\'message\': f\'step-{index:03d}\', \'domain\': domain,\n                            \'payload\': payload, \'outcome\': step[\'outcome\']})\n        if step[\'op\'] == \'capture\' and step[\'outcome\'] == \'Recorded\':\n            event = driver.event_at(payload[\'raw_utf8\'], payload[\'json_pointer\'])\n            recorded[driver.fact_key(event)] = {\'fact_hash\': driver.fact_hash(payload[\'raw_utf8\'], payload[\'json_pointer\'])}\n    ledger = oracle.IndependentLedger()\n    raw = (driver.HERE / \'fixtures/tb03c_allocation/RAW.json\').read_text()\n    def add(domain, payload, outcome):\n        rows[\'we7\'].append({\'message\': f"we7-{len(rows[\'we7\']):03d}", \'domain\': domain,\n                            \'payload\': payload, \'outcome\': outcome})\n    for index, event in enumerate(ledger.raw[\'events\']):\n        pointer, key = f\'/events/{index}\', driver.fact_key(event)\n        fact_hash = driver.fact_hash(raw, pointer)\n        external = event[\'source\'][\'external_fact_id\']\n        add(\'E\', {\'op\': \'CaptureExternal\', \'raw_utf8\': raw, \'json_pointer\': pointer,\n                  \'token\': \'t-\' + external, \'source_event_time\': event[\'source\'][\'source_event_time\']}, \'Recorded\')\n        if index < 3:\n            add(\'B:\' + event[\'book_identity\'][\'chong_id\'], {\'op\': \'ApplyAllocation\', \'fact_key\': key,\n                \'allocation_id\': event[\'allocation_id\'], \'expected_fact_hash\': fact_hash}, \'Applied\')\n        else:\n            add(\'E\', {\'op\': \'ResolveEvidence\', \'evidence_id\': \'resolve-\' + key,\n                     \'covered\': [{\'fact_key\': key, \'fact_hash\': fact_hash, \'token\': \'t-\' + external}],\n                     \'frontier\': {\'source_namespace\': driver.SOURCE[\'source_namespace\'],\n                                  \'source_epoch\': driver.SOURCE[\'source_epoch\'],\n                                  \'cursor\': event[\'source\'][\'source_cursor\']},\n                     \'dependency_version\': \'testonly-deps/1\'}, \'Resolved\')\n            for participant in ledger.manifest[\'participants\']:\n                allocation = \'netted:\' + driver.digest([key, event[\'allocation_id\'], ledger.manifest[\'manifest_hash\'], participant])\n                add(\'B:\' + participant[\'chong_id\'], {\'op\': \'ApplyAllocation\', \'fact_key\': key,\n                    \'allocation_id\': allocation, \'expected_fact_hash\': fact_hash}, \'Applied\')\n    equal([len(rows[k]) for k in (\'we5\', \'we7\')], [20, 21], \'固定完整 41 步\')\n    return rows\n\n\nclass ProcessLedger:\n    def __init__(self, root, binder, provenance, allowed):\n        self.root, self.binder, self.allowed = root, binder, allowed\n        path = binder.regular_file(root, \'processes.jsonl\')\n        self.allowed.add(\'processes.jsonl\')\n        self.rows = [strict_log_json(line) for line in path.read_bytes().splitlines()]\n        binder.verify_processes(self.rows)\n        self.start, self.end = provenance[\'driver_started_monotonic\'], provenance[\'driver_finished_monotonic\']\n        require(finite(self.start) and finite(self.end) and self.start <= self.end, \'外层 driver 区间非法\')\n        for row in self.rows:\n            require(self.start <= row[\'monotonic\'] <= self.end, \'子进程事件越出外层 driver 区间\')\n        self.spawns = [row for row in self.rows if row[\'event\'] == \'spawn\']\n        self.position = 0\n        self.active = {}\n        for row in self.rows:\n            if row[\'event\'] == \'spawn\':\n                self.active[row[\'pid\']] = row\n            elif row[\'event\'] == \'reaped\':\n                original = self.active.pop(row[\'pid\'])\n                for key in (\'stdout\', \'stderr\'):\n                    equal(row[key], original[key], \'收割原件归属\')\n                original[\'reaped\'] = row\n        require(not self.active, \'未回收子进程\')\n\n    def consume(self, command, label):\n        require(self.position < len(self.spawns), \'缺具名进程\')\n        row = self.spawns[self.position]\n        self.position += 1\n        equal(row[\'command\'], command, \'具名进程命令顺序 \' + label)\n        for stream in (\'stdout\', \'stderr\'):\n            expected = f\'{self.position:04d}-{label}.{stream}\'\n            equal(row[stream], expected, \'具名进程原件 \' + label)\n            self.binder.regular_file(self.root, expected)\n            self.allowed.add(expected)\n        equal(row[\'reaped\'][\'label\'], label, \'收割 label\')\n        return row, self.binder.regular_file(self.root, row[\'stdout\']).read_bytes()\n\n\ndef page_uint(value, maximum=4194304):\n    require(type(value) is str and re.fullmatch(r\'0|[1-9][0-9]*\', value) is not None,\n            \'分页整数必须是规范文本\')\n    require(len(value) <= 20 and int(value) <= maximum, \'分页整数超界\')\n    return int(value)\n\n\ndef page_document(document, descriptor, query, epochs, manifest, digest):\n    """独立核对重建文档和固定切面；不调用生产分页验证器。"""\n    kind = document.get(\'kind\')\n    equal(kind, descriptor[\'document_kind\'], \'分页逻辑类型\')\n    fixed = {}\n    def cut(value, domain):\n        require(type(value) is dict, \'cut 类型\')\n        equal(set(value), {\'domain_id\', \'commit_seq\', \'root_hash\'}, \'cut 精确字段\')\n        equal(value[\'domain_id\'], domain, \'cut 域\')\n        require(type(value[\'root_hash\']) is str and re.fullmatch(\'[0-9a-f]{64}\', value[\'root_hash\']),\n                \'cut root_hash 必须为小写64hex字符串\')\n        return page_uint(value[\'commit_seq\'], 2**64 - 1)\n    def image(row, domain):\n        seq = cut(row[\'cut\'], domain)\n        equal(row[\'image_hash\'], digest(row[\'image\']), \'重建图像摘要\')\n        stamp = page_uint(row[\'commit_ns\'], 2**64 - 1)\n        return seq, stamp\n    def changes(rows, base, end, domain):\n        require(type(rows) is list and len(rows) <= 32, \'变化数上限\')\n        previous = -1 if base is None else cut(base, domain)\n        stamp = -1\n        for row in rows:\n            seq, now = image(row, domain)\n            equal(seq, previous + 1, \'变化连续序号\')\n            require(now >= stamp, \'变化时间倒退\')\n            previous, stamp = seq, now\n        equal(cut(end, domain), previous, \'变化终点\')\n        if rows:\n            equal(rows[-1][\'cut\'], end, \'变化末cut\')\n        else:\n            require(base is not None, \'无创世变化\')\n            equal(end, base, \'空变化固定cut\')\n    if kind in (\'EconomicView\', \'EconomicChanges\'):\n        domain = document[\'domain_id\']\n        equal(set(epochs), {domain}, \'owner 域集合\')\n        equal(document[\'writer_epoch\'], epochs[domain], \'owner writer epoch\')\n        fixed[domain] = document[\'cut\']\n        if kind == \'EconomicView\':\n            equal(query[\'op\'], \'ReadView\', \'owner query类型\')\n            _, stamp = image(document, domain)\n            if query[\'cut\'] is not None:\n                equal(document[\'cut\'], query[\'cut\'], \'指定历史cut\')\n            if query[\'as_known_ns\'] is not None:\n                require(stamp <= page_uint(query[\'as_known_ns\'], 2**64-1), \'获知时间越界\')\n        else:\n            equal(query[\'op\'], \'Watch\', \'owner watch类型\')\n            changes(document[\'changes\'], query[\'after_cut\'], document[\'cut\'], domain)\n    else:\n        require(kind in (\'EconomicSnapshot\', \'EconomicWatch\'), \'未知文档类型\')\n        equal((document[\'session_id\'], document[\'session_generation\'], document[\'manifest_hash\']),\n              (manifest[\'session_id\'], manifest[\'session_generation\'], digest(manifest)), \'公共身份\')\n        equal(set(document[\'domains\']), set(epochs), \'公共域集合\')\n        for domain, row in document[\'domains\'].items():\n            require(row[\'status\'] in (\'Available\', \'Unavailable\'), \'公共可用状态\')\n            fixed[domain] = None\n            if row[\'status\'] == \'Unavailable\':\n                continue\n            if kind == \'EconomicSnapshot\':\n                image(row, domain)\n                fixed[domain] = row[\'cut\']\n                if query[\'cuts\'] is not None:\n                    equal(row[\'cut\'], query[\'cuts\'][domain], \'公共指定cut\')\n                if query[\'snapshot_token\'] is None and query[\'as_known_ns\'] is not None:\n                    require(page_uint(row[\'commit_ns\'], 2**64-1) <= page_uint(query[\'as_known_ns\'], 2**64-1),\n                            \'公共获知时间越界\')\n            else:\n                equal(row[\'base_cut\'], query[\'cuts\'][domain], \'watch base\')\n                changes(row[\'changes\'], row[\'base_cut\'], row[\'next_cut\'], domain)\n                fixed[domain] = row[\'next_cut\']\n        if kind == \'EconomicSnapshot\':\n            equal(query[\'op\'], \'snapshot\', \'snapshot query\')\n            equal(document[\'completeness\'], \'CompleteVector\' if all(v is not None for v in fixed.values())\n                  else \'PartialVector\', \'向量完整度\')\n            body = {k: v for k, v in document.items() if k != \'snapshot_token\'}\n            equal(document[\'snapshot_token\'], \'economic-snapshot-\' + digest(body), \'snapshot token摘要\')\n            if query[\'snapshot_token\'] is not None:\n                equal(document[\'snapshot_token\'], query[\'snapshot_token\'], \'原token重读\')\n            else:\n                equal(document[\'mode\'], \'AsKnown\' if query[\'cuts\'] is not None or query[\'as_known_ns\'] is not None\n                      else \'Current\', \'snapshot mode\')\n                equal(document[\'as_known_ns\'], query[\'as_known_ns\'], \'snapshot as known\')\n        else:\n            equal(query[\'op\'], \'watch\', \'watch query\')\n    require(type(descriptor[\'fixed_cuts\']) is dict, \'描述符fixed_cuts类型\')\n    equal(set(descriptor[\'fixed_cuts\']), set(epochs), \'描述符fixed_cuts完整域集合\')\n    for domain, value in descriptor[\'fixed_cuts\'].items():\n        if value is not None:\n            cut(value, domain)\n    if query.get(\'cuts\') is not None:\n        equal(set(query[\'cuts\']), set(epochs), \'请求cuts完整域集合\')\n        for domain, value in query[\'cuts\'].items():\n            cut(value, domain)\n    equal(descriptor[\'fixed_cuts\'], fixed, \'描述符固定切面投影\')\n\n\ndef verify_internal_document(driver, records, domain, selector, manifest, epochs):\n    """消费一组 owner Open/Page/Close；独立重组，不调用生产分页器。"""\n    require(records, \'Q内部缺owner逻辑读\')\n    request, opened = records.pop(0)\n    payload = request[\'payload\']\n    equal(set(payload), {\'schema_revision\', \'op\', \'query\', \'requested_remaining_ms\'}, \'内部Open字段\')\n    equal((payload[\'schema_revision\'], payload[\'op\'], payload[\'query\']),\n          (\'economic-read-pages/1\', \'EconomicReadOpen\', selector), \'内部Open完整选择器\')\n    remaining = page_uint(payload[\'requested_remaining_ms\'], 30000)\n    require(remaining > 0, \'内部Open剩余预算\')\n    equal(set(opened), {\'schema_revision\', \'kind\', \'descriptor\', \'descriptor_sha256\'}, \'内部Open响应字段\')\n    equal((opened[\'schema_revision\'], opened[\'kind\']),\n          (\'economic-read-pages/1\', \'EconomicReadOpened\'), \'内部Open成功\')\n    desc, dh = opened[\'descriptor\'], opened[\'descriptor_sha256\']\n    equal(set(desc), {\'schema_revision\', \'transfer_id\', \'session_id\', \'session_generation\',\n        \'manifest_hash\', \'producer_id\', \'producer_epoch\', \'query_sha256\', \'document_kind\',\n        \'fixed_cuts\', \'owner_epochs\', \'document_bytes\', \'document_sha256\', \'chunk_bytes\', \'page_count\'},\n        \'内部描述符字段\')\n    for key, value in {\'schema_revision\': \'economic-read-pages/1\', \'session_id\': manifest[\'session_id\'],\n        \'session_generation\': manifest[\'session_generation\'], \'manifest_hash\': driver.digest(manifest),\n        \'producer_id\': domain + \':\' + manifest[\'session_id\'], \'producer_epoch\': epochs[domain],\n        \'query_sha256\': driver.digest(selector), \'owner_epochs\': {domain: epochs[domain]},\n        \'chunk_bytes\': \'262144\'}.items():\n        equal(desc[key], value, \'内部描述符绑定 \' + key)\n    require(type(desc[\'transfer_id\']) is str and re.fullmatch(\'[0-9a-f]{32}\', desc[\'transfer_id\']),\n            \'内部transfer身份\')\n    equal(dh, driver.digest(desc), \'内部描述符hash\')\n    total = page_uint(desc[\'document_bytes\'])\n    require(total > 0, \'内部正文为空\')\n    count = (total + 262143) // 262144\n    equal(page_uint(desc[\'page_count\'], 16), count, \'内部页数\')\n    raw = bytearray()\n    for _ in range(count):\n        require(records, \'Q内部缺Page\')\n        req, reply = records.pop(0)\n        p = req[\'payload\']\n        ms = page_uint(p.get(\'requested_remaining_ms\'), 30000)\n        require(0 < ms <= remaining, \'内部预算复活\')\n        remaining = ms\n        equal(p, {\'schema_revision\': \'economic-read-pages/1\', \'op\': \'EconomicReadPage\',\n            \'transfer_id\': desc[\'transfer_id\'], \'descriptor_sha256\': dh,\n            \'offset\': str(len(raw)), \'requested_remaining_ms\': str(ms)}, \'内部Page完整请求\')\n        equal(set(reply), {\'schema_revision\', \'kind\', \'transfer_id\', \'descriptor_sha256\', \'offset\',\n            \'raw_byte_count\', \'chunk_sha256\', \'data_base64\', \'next_offset\', \'done\'}, \'内部Page字段\')\n        for key, value in {\'schema_revision\': \'economic-read-pages/1\', \'kind\': \'EconomicReadPage\',\n            \'transfer_id\': desc[\'transfer_id\'], \'descriptor_sha256\': dh, \'offset\': str(len(raw))}.items():\n            equal(reply[key], value, \'内部Page绑定\')\n        fragment = base64.b64decode(reply[\'data_base64\'], validate=True)\n        equal(base64.b64encode(fragment).decode(\'ascii\'), reply[\'data_base64\'], \'内部base64规范编码\')\n        equal(len(fragment), min(262144, total - len(raw)), \'内部片长度\')\n        equal(page_uint(reply[\'raw_byte_count\']), len(fragment), \'内部片原长度\')\n        equal(reply[\'chunk_sha256\'], hashlib.sha256(fragment).hexdigest(), \'内部片hash\')\n        raw.extend(fragment)\n        require(type(reply[\'done\']) is bool, \'内部done类型\')\n        equal(reply[\'done\'], len(raw) == total, \'内部done\')\n        equal(reply[\'next_offset\'], None if len(raw) == total else str(len(raw)), \'内部next offset\')\n    require(records, \'Q内部缺Close\')\n    req, reply = records.pop(0)\n    equal(req[\'payload\'], {\'schema_revision\': \'economic-read-pages/1\', \'op\': \'EconomicReadClose\',\n        \'transfer_id\': desc[\'transfer_id\'], \'descriptor_sha256\': dh}, \'内部Close完整请求\')\n    equal(reply, {\'schema_revision\': \'economic-read-pages/1\', \'kind\': \'EconomicReadClosed\',\n        \'transfer_id\': desc[\'transfer_id\'], \'descriptor_sha256\': dh}, \'内部Close成功\')\n    equal(hashlib.sha256(raw).hexdigest(), desc[\'document_sha256\'], \'内部完整文档hash\')\n    document = driver.decode_frame(raw)\n    equal(driver.canonical(document), bytes(raw), \'内部canonical完整字节\')\n    page_document(document, desc, selector, {domain: epochs[domain]}, manifest, driver.digest)\n    return document\n\n\ndef replay_case_class(driver, binder):\n    class ReplayCase(driver.Case):\n        def __init__(self, name, output, input_manifest, process, provenance, schedule, allowed):\n            require(\'case_startup_anchors\' in provenance and name in provenance[\'case_startup_anchors\'],\n                    \'缺独立 Q startup 锚 \' + name)\n            self.clock_anchors = provenance[\'case_startup_anchors\'][name]\n            equal(set(self.clock_anchors), {\'started_at\', \'case_deadline\', \'work_deadline\'}, \'独立 Q startup 锚字段\')\n            require(all(finite(v) for v in self.clock_anchors.values()), \'独立 Q startup 锚时钟非法\')\n            self.name, self.output, self.work = name, output / name, output / name / \'state\'\n            self.root, self.process, self.provenance, self.schedule, self.allowed = output, process, provenance, schedule, allowed\n            # *** 关键修复：直接读取文件中的 manifest，不从 input_manifest 构造 ***\n            # 文件中的 manifest 包含 ProducerCase 在 super().__init__() 之前修改的所有字段：\n            # - \'session_id\': \'tb1406-c6a-\' + name\n            # - \'e\': {}\n            # - 每个 node 的 \'db\' 和 \'socket\'\n            # - 追加的 authorized_sources（完整版，包含 EconomicRead* operations）\n            self.manifest = self.read_json(\'state/manifest.json\')\n            # 保存原件，用于后续验证\n            file_manifest = copy.deepcopy(self.manifest)\n            self.nodes = {\'E\': self.manifest[\'e\'], **{\'B:\' + n[\'chong_id\']: n for n in self.manifest[\'chongs\']}}\n            for index, node in enumerate(self.nodes.values()):\n                self.read_bytes(f\'state/{index}.db\')\n            # 验证：文件中的 manifest 必须与我们期望的一致\n            # 期望的 manifest = input_manifest + ProducerCase 的修改\n            expected_manifest = copy.deepcopy(input_manifest)\n            expected_manifest[\'session_id\'] = \'tb1406-c6a-\' + name\n            expected_manifest[\'e\'] = {}\n            expected_nodes = {\'E\': expected_manifest[\'e\'], **{\'B:\' + n[\'chong_id\']: n for n in expected_manifest[\'chongs\']}}\n            for index, node in enumerate(expected_nodes.values()):\n                node.update(db=str(self.work / f\'{index}.db\'), socket=str(self.work / f\'{index}.sock\'))\n            expected_manifest[\'authorized_sources\'].append({**driver.QUERY_SOURCE, \'operations\': [\'ReadView\', \'Watch\',\n                \'EconomicReadOpen\', \'EconomicReadPage\', \'EconomicReadClose\']})\n            equal(file_manifest, expected_manifest, \'独立启动 manifest\')\n            self.binaries = Path(provenance[\'binaries_dir\'])\n            self.epochs, self.requests = {}, {}\n            # 初始化父类 d.Case.paged 需要的序列计数器（参考 tb03c_query_runtime.py:600）\n            self.sequence, self.query_sequence, self.logical_sequence = 0, 0, 0\n            self.business, self.last_views, self.pending = [], None, None\n            self.startup = False\n            # 分页由下面独立的离线核验实现；禁止调用生产组装 helper 或伪时钟。\n            self.budget = types.SimpleNamespace(until=lambda end: (_ for _ in ()).throw(\n                ValueError(\'离线分页不可借用生产截止判据\')))\n            self.last_logical_time = process.start\n            self.server_processes = []\n            self.internal_messages = set()\n            self.q_attempt = 0\n\n        def read_bytes(self, relative):\n            path = binder.regular_file(self.output, relative)\n            self.allowed.add(f\'{self.name}/{relative}\')\n            return path.read_bytes()\n\n        def read_json(self, relative):\n            self.read_bytes(relative)\n            return binder.load(binder.regular_file(self.output, relative))\n\n        def wire(self, address, request, *, http=False, limit=None, deadline=None, startup_ready=None):\n            require(startup_ready is None, \'离线 wire 不得另开启动循环\')\n            self.sequence += 1\n            label = f\'{self.sequence:04d}\'\n            transport = self.read_json(label + \'-transport.json\')\n            equal(set(transport), {\'request\', \'response\', \'request_sha256\', \'response_sha256\',\n                                   \'response_bytes\', \'http\', \'address\'}, \'业务 transport 精确字段\')\n            require(transport[\'http\'] is http, \'逐序 transport 角色\')\n            equal(transport[\'address\'], list(address) if http else address, \'逐序 transport 地址\')\n            prefix = \'http\' if http else \'owner\'\n            for kind in (\'request\', \'response\'):\n                equal(transport[kind], f\'{label}.{prefix}-{kind}.bin\', \'逐序原件名\')\n            sent, received = self.read_bytes(transport[\'request\']), self.read_bytes(transport[\'response\'])\n            equal(sent, request, \'具名请求原字节\')\n            for kind, raw in ((\'request\', sent), (\'response\', received)):\n                equal(transport[kind + \'_sha256\'], hashlib.sha256(raw).hexdigest(), \'逐序 wire hash\')\n            require(type(transport[\'response_bytes\']) is int, \'响应长度类型\')\n            equal(transport[\'response_bytes\'], len(received), \'响应长度\')\n            maximum = driver.FRAME_BYTES + 16384 if http else driver.FRAME_BYTES\n            require(len(sent) <= driver.FRAME_BYTES and len(received) <= maximum, \'原 frame 字节界\')\n            if http:\n                body = received.partition(b\'\\r\\n\\r\\n\')[2]\n                require(len(body) <= int(driver.QUERY_RESOURCES[\'max_reply_bytes\']), \'Q 原响应容量\')\n            return received\n\n        def q_owner_scope(self, public, physical):\n            self.q_attempt += 1\n            name = f\'q-owner-wire/attempt-{self.q_attempt:06d}/\'\n            equal(self.read_bytes(name + \'public-request.json\'), driver.canonical(public), \'Q内部公共请求绑定\')\n            begin, end = self.read_json(name + \'begin.json\'), self.read_json(name + \'end.json\')\n            equal(set(begin), {\'schema_revision\', \'attempt\', \'public_request_sha256\', \'started_at\', \'deadline\'}, \'Q内部scope开始字段\')\n            equal(set(end), {\'attempt\', \'rpc_count\', \'wire_bytes\', \'error_class\', \'observed_at\'}, \'Q内部scope结束字段\')\n            require(type(begin[\'attempt\']) is int and type(end[\'attempt\']) is int, \'Q内部attempt类型\')\n            equal((begin[\'attempt\'], end[\'attempt\']), (self.q_attempt, self.q_attempt), \'Q内部逐次attempt绑定\')\n            equal((begin[\'schema_revision\'], begin[\'public_request_sha256\']),\n                  (\'c6a-q-owner-wire/1\', driver.digest(public)), \'Q内部scope身份\')\n            require(all(finite(v) for v in (begin[\'started_at\'], begin[\'deadline\'], end[\'observed_at\'])),\n                    \'Q内部scope时钟\')\n            require(physical[\'started_at\'] <= begin[\'started_at\'] <= end[\'observed_at\'] <= physical[\'observed_at\']\n                    and end[\'observed_at\'] < min(begin[\'deadline\'], physical[\'deadline\']), \'Q内部scope迟到或越界\')\n            require(begin[\'deadline\'] <= begin[\'started_at\'] + 30, \'Q内部scope不得另开长窗\')\n            # 由冻结资源合同推出结构上界，不新增产品诊断拒绝帽。\n            maximum_document = min(4194304, int(driver.QUERY_RESOURCES[\'max_cached_bytes\']))\n            pages_per_document = (maximum_document + 262143) // 262144\n            maximum_rpc = int(driver.QUERY_RESOURCES[\'max_domains\']) * (\n                2 + int(driver.QUERY_RESOURCES[\'max_changes\'])) * (2 + pages_per_document)\n            require(type(end[\'rpc_count\']) is int and 0 <= end[\'rpc_count\'] <= maximum_rpc, \'Q内部rpc计数\')\n            equal(end[\'error_class\'], None, \'Q内部scope失败\')\n            grouped = {domain: [] for domain in self.nodes}\n            previous = begin[\'started_at\']\n            last_domain = dict.fromkeys(self.nodes, previous)\n            total, logical_deadline = 0, None\n            for index in range(1, end[\'rpc_count\'] + 1):\n                prefix = name + f\'rpc-{index:04d}\'\n                start, finish = self.read_json(prefix + \'-begin.json\'), self.read_json(prefix + \'-end.json\')\n                equal(set(start), {\'index\', \'address\', \'timeout_ms\', \'max_frame_bytes\', \'started_at\',\n                    \'deadline\', \'logical_deadline\', \'request_sha256\', \'request_bytes\'}, \'Q内部RPC开始字段\')\n                equal(set(finish), {\'transport\', \'error_class\', \'started_send\', \'observed_at\',\n                    \'response_bytes\', \'response_sha256\'}, \'Q内部RPC结束字段\')\n                equal(start[\'index\'], index, \'Q内部连续序号\')\n                equal(start[\'max_frame_bytes\'], 1048576, \'Q内部原帧帽\')\n                require(type(start[\'timeout_ms\']) is int and 0 < start[\'timeout_ms\'] <= 30000, \'Q内部输送timeout\')\n                require(all(finite(v) for v in (start[\'started_at\'], start[\'deadline\'],\n                        start[\'logical_deadline\'], finish[\'observed_at\'])), \'Q内部RPC时钟类型\')\n                require(previous <= start[\'started_at\'] <= finish[\'observed_at\'] <= end[\'observed_at\']\n                        and finish[\'observed_at\'] < start[\'deadline\'] <= start[\'logical_deadline\']\n                        <= begin[\'deadline\'], \'Q内部RPC迟到或期限不属于scope\')\n                require(start[\'deadline\'] <= start[\'started_at\'] + start[\'timeout_ms\']/1000,\n                        \'Q内部单次输送期限变长\')\n                previous = start[\'started_at\']\n                if logical_deadline is None:\n                    logical_deadline = start[\'logical_deadline\']\n                equal(start[\'logical_deadline\'], logical_deadline, \'Q内部逻辑读共同截止不可续窗\')\n                sent, received = self.read_bytes(prefix + \'-request.bin\'), self.read_bytes(prefix + \'-response.bin\')\n                for side, raw, meta in ((\'request\', sent, start), (\'response\', received, finish)):\n                    require(0 < len(raw) <= 1048576, \'Q内部原帧大小\')\n                    equal(meta[side + \'_bytes\'], len(raw), \'Q内部原帧长度\')\n                    equal(meta[side + \'_sha256\'], hashlib.sha256(raw).hexdigest(), \'Q内部原帧摘要\')\n                    total += len(raw)\n                equal((finish[\'transport\'], finish[\'error_class\'], finish[\'started_send\']),\n                      (\'Received\', None, True), \'Q内部失败输送不得算成功\')\n                req, rep = driver.decode_frame(sent), driver.decode_frame(received)\n                equal(sent, driver.canonical(req) + b\'\\n\', \'Q内部请求canonical原字节\')\n                require(received.endswith(b\'\\n\') and received.count(b\'\\n\') == 1, \'Q内部单响应帧\')\n                equal(set(req), {\'schema_revision\', \'session_id\', \'session_generation\', \'source_namespace\',\n                    \'source_epoch\', \'producer_id\', \'producer_epoch\', \'message_id\', \'payload_hash\',\n                    \'causal_refs\', \'payload\'}, \'Q内部请求公共头\')\n                for key, expected in {**driver.QUERY_SOURCE, \'schema_revision\': public[\'schema_revision\'],\n                    \'session_id\': self.manifest[\'session_id\'], \'session_generation\': self.manifest[\'session_generation\'],\n                    \'causal_refs\': []}.items():\n                    equal(req[key], expected, \'Q内部请求身份 \' + key)\n                require(type(req[\'message_id\']) is str and re.fullmatch(\'query-[0-9a-f]{32}\', req[\'message_id\']),\n                        \'Q内部随机请求身份格式\')\n                require(req[\'message_id\'] not in self.internal_messages, \'Q内部请求身份重复\')\n                self.internal_messages.add(req[\'message_id\'])\n                equal(req[\'payload_hash\'], driver.digest(req[\'payload\']), \'Q内部请求内容绑定\')\n                domains = [d for d, node in self.nodes.items() if node[\'socket\'] == start[\'address\']]\n                equal(len(domains), 1, \'Q内部具名owner地址\')\n                domain = domains[0]\n                if req[\'payload\'].get(\'op\') != \'EconomicReadClose\':\n                    ms = page_uint(req[\'payload\'].get(\'requested_remaining_ms\'), 30000)\n                    require(0 < ms <= int((logical_deadline - last_domain[domain]) * 1000),\n                            \'Q内部wire预算超过原共同截止\')\n                require(start[\'started_at\'] >= last_domain[domain], \'同域RPC顺序重叠\')\n                last_domain[domain] = finish[\'observed_at\']\n                value = driver.validate_reply(req, rep, producer=domain, producer_epoch=self.epochs[domain])\n                grouped[domain].append((req, value))\n            equal(end[\'wire_bytes\'], total, \'Q内部scope字节账\')\n            require(total <= end[\'rpc_count\'] * 2 * driver.FRAME_BYTES, \'Q内部scope逐帧字节账\')\n            return grouped\n\n        def q_owner_projection(self, query, document, grouped):\n            if query[\'op\'] == \'snapshot\' and query[\'snapshot_token\'] is not None:\n                require(all(not rows for rows in grouped.values()), \'固定token不得新增owner读取\')\n                return\n            for domain, rows in grouped.items():\n                actual = document[\'domains\'][domain]\n                equal(actual[\'status\'], \'Available\', \'正常轨Q域必须完整可用\')\n                if query[\'op\'] == \'snapshot\':\n                    selector = {\'op\': \'ReadView\', \'cut\': None if query[\'cuts\'] is None else query[\'cuts\'][domain],\n                                \'as_known_ns\': query[\'as_known_ns\']}\n                    view = verify_internal_document(driver, rows, domain, selector, self.manifest, self.epochs)\n                    equal(actual, {\'status\': \'Available\', **{k: view[k] for k in (\'cut\', \'commit_ns\', \'image\', \'image_hash\')}},\n                          \'Q公共snapshot必须逐字来自内部owner文档\')\n                else:\n                    selector = {\'op\': \'ReadView\', \'cut\': query[\'cuts\'][domain], \'as_known_ns\': None}\n                    base = verify_internal_document(driver, rows, domain, selector, self.manifest, self.epochs)\n                    changes = verify_internal_document(driver, rows, domain,\n                        {\'op\': \'Watch\', \'after_cut\': query[\'cuts\'][domain]}, self.manifest, self.epochs)\n                    equal(actual, {\'status\': \'Available\', \'base_cut\': query[\'cuts\'][domain],\n                                   \'next_cut\': changes[\'cut\'], \'changes\': changes[\'changes\']},\n                          \'Q公共watch必须逐字来自内部owner文档\')\n                    stamp = page_uint(base[\'commit_ns\'], 2**64-1)\n                    for change in changes[\'changes\']:\n                        view = verify_internal_document(driver, rows, domain,\n                            {\'op\': \'ReadView\', \'cut\': change[\'cut\'], \'as_known_ns\': None},\n                            self.manifest, self.epochs)\n                        equal({k: view[k] for k in (\'cut\', \'commit_ns\', \'image\', \'image_hash\')}, change,\n                              \'Q内部变化必须经过同cut全图回查\')\n                        now = page_uint(change[\'commit_ns\'], 2**64-1)\n                        require(now >= stamp, \'Q内部变化早于基点\')\n                        stamp = now\n                require(not rows, \'多余Q内部owner RPC\')\n\n        def paged(self, query, *, domain, message=None, deadline=None):\n            self.logical_sequence += 1\n            logical_id = f\'logical-{self.logical_sequence:04d}\'\n            proof = self.read_json(logical_id + \'-pagination.json\')\n            expected_fields = {\'schema_revision\', \'logical_id\', \'domain\', \'logical_message\', \'selector\',\n                \'query_sha256\', \'session_id\', \'session_generation\', \'manifest_hash\', \'producer_id\',\n                \'producer_epoch\', \'owner_epochs\', \'started_at\', \'deadline\', \'case_deadline\',\n                \'work_deadline\', \'physical\', \'status\', \'payload\', \'document_bytes\', \'document_sha256\',\n                \'observed_at\'}\n            equal(set(proof), expected_fields, \'分页证明完整字段\')\n            epochs = dict(self.epochs) if domain == \'Q\' else {domain: self.epochs[domain]}\n            epoch = \'1\' if domain == \'Q\' else self.epochs[domain]\n            identity = {\'schema_revision\': \'c6a-logical-pages/1\', \'logical_id\': logical_id,\n                \'domain\': domain, \'logical_message\': message, \'selector\': query,\n                \'query_sha256\': driver.digest(query), \'session_id\': self.manifest[\'session_id\'],\n                \'session_generation\': self.manifest[\'session_generation\'], \'manifest_hash\': driver.digest(self.manifest),\n                \'producer_id\': domain + \':\' + self.manifest[\'session_id\'], \'producer_epoch\': epoch,\n                \'owner_epochs\': epochs, \'status\': \'COMPLETE\'}\n            for key, value in identity.items():\n                equal(proof[key], value, \'分页证明绑定 \' + key)\n            start, end, observed = proof[\'started_at\'], proof[\'deadline\'], proof[\'observed_at\']\n            require(all(finite(v) for v in (start, end, observed)), \'分页逻辑时钟类型\')\n            equal(proof[\'case_deadline\'], self.clock_anchors[\'case_deadline\'], \'分页case原截止\')\n            equal(proof[\'work_deadline\'], self.clock_anchors[\'work_deadline\'], \'分页work原截止\')\n            expected_end = min(start + 30, proof[\'case_deadline\'], proof[\'work_deadline\'],\n                               deadline if deadline is not None else proof[\'work_deadline\'])\n            equal(end, expected_end, \'分页30秒唯一截止\')\n            require(self.last_logical_time <= start <= observed < end and observed <= self.process.end,\n                    \'分页迟到或时钟逆序\')\n            require(self.process.spawns and self.process.spawns[self.process.position-1][\'monotonic\'] <= start,\n                    \'分页早于所属服务启动\')\n            physical = proof[\'physical\']\n            require(type(physical) is list and 3 <= len(physical) <= 18, \'分页物理帧数\')\n            previous, remaining, descriptor, descriptor_hash = start, 30000, None, None\n            assembled = bytearray()\n            public_scopes = []\n            for index, row in enumerate(physical, 1):\n                equal(set(row), {\'index\', \'sequence\', \'op\', \'remaining_ms\', \'started_at\', \'deadline\',\n                    \'observed_at\', \'transport\', \'request_envelope_sha256\', \'request_identity\',\n                    \'response_envelope_sha256\', \'response_identity\'}, \'分页物理索引字段\')\n                require(type(row[\'index\']) is int and type(row[\'sequence\']) is int, \'分页索引类型\')\n                equal((row[\'index\'], row[\'sequence\'], row[\'transport\']),\n                      (index, self.sequence + 1, f\'{self.sequence+1:04d}-transport.json\'), \'分页连续物理序列\')\n                require(finite(row[\'started_at\']) and finite(row[\'observed_at\']) and\n                        previous <= row[\'started_at\'] <= row[\'observed_at\'] <= observed and row[\'observed_at\'] < end,\n                        \'分页物理时钟或晚到\')\n                equal(row[\'deadline\'], end, \'分页不可续窗\')\n                ms = row[\'remaining_ms\']\n                require(type(ms) is int and 1 <= ms <= remaining and ms <= int((end-previous)*1000),\n                        \'分页剩余预算不可复活\')\n                remaining = ms\n                # helper 先计算wire预算，再计算fetch超时；两者取时点不同。\n                # 只从原帧读取该非确定值，并在原截止和前页观察时刻内限制。\n                transport = self.read_json(row[\'transport\'])\n                sent = self.read_bytes(transport[\'request\'])\n                encoded = sent.partition(b\'\\r\\n\\r\\n\')[2] if domain == \'Q\' else sent\n                original = driver.decode_frame(encoded)\n                wire_ms = None\n                if index != len(physical):\n                    wire_ms = page_uint(original[\'payload\'][\'requested_remaining_ms\'], 30000)\n                    require(ms <= wire_ms <= int((end-previous)*1000), \'wire剩余预算越界\')\n                if index == 1:\n                    payload = {\'schema_revision\': \'economic-read-pages/1\', \'op\': \'EconomicReadOpen\',\n                               \'query\': query, \'requested_remaining_ms\': str(wire_ms)}\n                elif index == len(physical):\n                    payload = {\'schema_revision\': \'economic-read-pages/1\', \'op\': \'EconomicReadClose\',\n                               \'transfer_id\': descriptor[\'transfer_id\'], \'descriptor_sha256\': descriptor_hash}\n                else:\n                    payload = {\'schema_revision\': \'economic-read-pages/1\', \'op\': \'EconomicReadPage\',\n                        \'transfer_id\': descriptor[\'transfer_id\'], \'descriptor_sha256\': descriptor_hash,\n                        \'offset\': str(len(assembled)), \'requested_remaining_ms\': str(wire_ms)}\n                equal(row[\'op\'], payload[\'op\'], \'分页op顺序\')\n                evidence = {}\n                if domain == \'Q\':\n                    response = self.http_frame(\'/api/economic/read\', payload, evidence=evidence)\n                    public_scopes.append((original, row))\n                else:\n                    response = self.owner_frame(domain, payload, logical_id + \'-\' + str(index), evidence=evidence)\n                for key, value in evidence.items():\n                    equal(row[key], value, \'分页原信封索引 \' + key)\n                equal(response.get(\'schema_revision\'), \'economic-read-pages/1\', \'分页响应版本\')\n                if index == 1:\n                    equal(set(response), {\'schema_revision\', \'kind\', \'descriptor\', \'descriptor_sha256\'}, \'Open字段\')\n                    equal(response[\'kind\'], \'EconomicReadOpened\', \'Open成功\')\n                    descriptor, descriptor_hash = response[\'descriptor\'], response[\'descriptor_sha256\']\n                    equal(set(descriptor), {\'schema_revision\', \'transfer_id\', \'session_id\', \'session_generation\',\n                        \'manifest_hash\', \'producer_id\', \'producer_epoch\', \'query_sha256\', \'document_kind\',\n                        \'fixed_cuts\', \'owner_epochs\', \'document_bytes\', \'document_sha256\', \'chunk_bytes\', \'page_count\'},\n                        \'描述符字段\')\n                    for key in (\'session_id\', \'session_generation\', \'manifest_hash\', \'producer_id\',\n                                \'producer_epoch\', \'query_sha256\', \'owner_epochs\'):\n                        equal(descriptor[key], identity[key], \'描述符可信绑定 \' + key)\n                    equal(descriptor[\'schema_revision\'], \'economic-read-pages/1\', \'描述符版本\')\n                    require(type(descriptor[\'transfer_id\']) is str and\n                            re.fullmatch(\'[0-9a-f]{32}\', descriptor[\'transfer_id\']), \'随机transfer格式\')\n                    require(type(descriptor[\'document_sha256\']) is str and\n                            re.fullmatch(\'[0-9a-f]{64}\', descriptor[\'document_sha256\']), \'正文hash格式\')\n                    equal(descriptor_hash, driver.digest(descriptor), \'描述符hash\')\n                    total = page_uint(descriptor[\'document_bytes\'])\n                    require(total > 0, \'正文为空\')\n                    equal(descriptor[\'chunk_bytes\'], \'262144\', \'固定stride\')\n                    equal(page_uint(descriptor[\'page_count\'], 16), (total + 262143)//262144, \'页数\')\n                    equal(len(physical), (total + 262143)//262144 + 2, \'物理帧完整性\')\n                elif index == len(physical):\n                    equal(response, {\'schema_revision\': \'economic-read-pages/1\', \'kind\': \'EconomicReadClosed\',\n                        \'transfer_id\': descriptor[\'transfer_id\'], \'descriptor_sha256\': descriptor_hash}, \'Close必须成功\')\n                else:\n                    equal(set(response), {\'schema_revision\', \'kind\', \'transfer_id\', \'descriptor_sha256\', \'offset\',\n                        \'raw_byte_count\', \'chunk_sha256\', \'data_base64\', \'next_offset\', \'done\'}, \'Page字段\')\n                    for key in (\'transfer_id\', \'descriptor_sha256\', \'offset\'):\n                        equal(response[key], payload[key], \'Page绑定 \' + key)\n                    equal(response[\'kind\'], \'EconomicReadPage\', \'Page成功\')\n                    count = min(262144, total - len(assembled))\n                    equal(page_uint(response[\'raw_byte_count\']), count, \'页原字节数\')\n                    require(type(response[\'data_base64\']) is str, \'base64类型\')\n                    fragment = base64.b64decode(response[\'data_base64\'], validate=True)\n                    equal(base64.b64encode(fragment).decode(\'ascii\'), response[\'data_base64\'], \'base64规范编码\')\n                    equal(len(fragment), count, \'片长度\')\n                    equal(hashlib.sha256(fragment).hexdigest(), response[\'chunk_sha256\'], \'片hash\')\n                    assembled.extend(fragment)\n                    require(type(response[\'done\']) is bool, \'done类型\')\n                    equal(response[\'done\'], len(assembled) == total, \'done精确边界\')\n                    equal(response[\'next_offset\'], None if len(assembled) == total else str(len(assembled)),\n                          \'next_offset边界\')\n                previous = row[\'observed_at\']\n            equal(len(assembled), total, \'完整文档长度\')\n            equal(hashlib.sha256(assembled).hexdigest(), descriptor[\'document_sha256\'], \'完整文档hash\')\n            document = driver.decode_frame(bytes(assembled))\n            equal(driver.canonical(document), bytes(assembled), \'完整canonical字节\')\n            page_document(document, descriptor, query, epochs, self.manifest, driver.digest)\n            if domain == \'Q\':\n                for index, (public, row) in enumerate(public_scopes):\n                    internal = self.q_owner_scope(public, row)\n                    if index == 0:\n                        self.q_owner_projection(query, document, internal)\n                    else:\n                        require(all(not frames for frames in internal.values()), \'Q Page/Close不得新增owner RPC\')\n            equal(proof[\'payload\'], logical_id + \'-payload.json\', \'逻辑文档原件名\')\n            equal(self.read_bytes(proof[\'payload\']), bytes(assembled), \'保存原件与独立重组逐字相等\')\n            require(type(proof[\'document_bytes\']) is int, \'证明长度类型\')\n            equal(proof[\'document_bytes\'], total, \'证明长度\')\n            equal(proof[\'document_sha256\'], descriptor[\'document_sha256\'], \'证明hash\')\n            self.last_logical_time = observed\n            return document\n\n        def http(self, path, payload=None, *, deadline=None, startup_ready=None):\n            reply = super().http(path, payload, deadline=deadline, startup_ready=startup_ready)\n            equal((reply[\'session_id\'], reply[\'session_generation\'], reply[\'manifest_hash\']),\n                  (self.manifest[\'session_id\'], self.manifest[\'session_generation\'], driver.digest(self.manifest)),\n                  \'公共响应 manifest/session 身份\')\n            if path.endswith(\'/snapshot\'):\n                fields = {\'kind\', \'session_id\', \'session_generation\', \'manifest_hash\', \'mode\',\n                          \'as_known_ns\', \'domains\', \'completeness\', \'snapshot_token\'}\n                equal(set(reply), fields, \'公共 snapshot 完整字段\')\n                body = {k: v for k, v in reply.items() if k != \'snapshot_token\'}\n                equal(reply[\'snapshot_token\'], \'economic-snapshot-\' + driver.digest(body), \'公共固定 token 身份\')\n                if payload[\'snapshot_token\'] is None:\n                    equal(reply[\'mode\'], \'AsKnown\' if payload[\'cuts\'] is not None or payload[\'as_known_ns\'] is not None else \'Current\', \'公共模式\')\n                    equal(reply[\'as_known_ns\'], payload[\'as_known_ns\'], \'公共获知时刻\')\n            elif path.endswith(\'/watch\'):\n                equal(set(reply), {\'kind\', \'session_id\', \'session_generation\', \'manifest_hash\', \'domains\'}, \'公共 watch 完整字段\')\n            return reply\n\n        def owner(self, domain, payload, message, *, deadline=None):\n            first = message not in self.requests\n            if message.startswith((\'step-\', \'we7-\')) and first:\n                require(len(self.business) < len(self.schedule), \'多余业务步骤\')\n                expected = self.schedule[len(self.business)]\n                equal((domain, payload, message), (expected[\'domain\'], expected[\'payload\'], expected[\'message\']), \'独立 41 步请求顺序\')\n                self.business.append(message)\n            reply = super().owner(domain, payload, message, deadline=deadline)\n            if message.startswith((\'step-\', \'we7-\')) and first:\n                equal(driver.outcome(reply), self.schedule[len(self.business)-1][\'outcome\'], \'独立步骤 outcome\')\n                if reply[\'kind\'] in (\'Recorded\', \'Resolved\', \'Applied\'):\n                    self.pending = (domain, copy.deepcopy(payload), copy.deepcopy(reply), copy.deepcopy(self.last_views))\n            return reply\n\n        def observe(self):\n            result = super().observe()\n            if self.pending:\n                domain, payload, receipt, previous = self.pending\n                require(previous is not None, \'写前 owner 缺失\')\n                equal(receipt[\'cut\'], result[domain][\'cut\'], \'成功回执 cut 对 after owner\')\n                if payload[\'op\'] == \'CaptureExternal\':\n                    binder.verify_capture(payload, receipt, previous[\'E\'], result[\'E\'])\n                self.pending = None\n            self.last_views = copy.deepcopy(result)\n            return result\n\n        def start(self):\n            first_init = None\n            for domain in self.nodes:\n                init, raw = self.process.consume(self.args(domain, \'init\'), self.name + \'-init\')\n                if first_init is None:\n                    first_init = init\n                reply = driver.decode_frame(raw)\n                equal((reply[\'kind\'], reply[\'domain_id\'], reply[\'writer_epoch\']), (\'Initialized\', domain, \'1\'), \'正常轨 init 身份\')\n                self.epochs[domain] = reply[\'writer_epoch\']\n                serve, _ = self.process.consume(self.args(domain, \'serve\'), self.name + \'-serve\')\n                require(init[\'reaped\'][\'monotonic\'] <= serve[\'monotonic\'], \'init 未退出即 serve\')\n                self.server_processes.append(serve)\n                equal(self.owner(domain, {\'op\': \'ReadView\', \'cut\': None, \'as_known_ns\': None}, \'ready-\' + domain)[\'kind\'],\n                      \'EconomicView\', \'owner 启动完整就绪\')\n            config = {\'schema_revision\': \'economic-query/1\', \'manifest\': \'manifest.json\',\n                      \'manifest_hash\': driver.digest(self.manifest), \'query_source\': driver.QUERY_SOURCE,\n                      \'producer_epoch\': \'1\', \'expected_epochs\': self.epochs, \'resources\': driver.QUERY_RESOURCES}\n            equal(self.read_json(\'state/query.json\'), config, \'Q 原配置\')\n            row = self.process.spawns[self.process.position]\n            command = row[\'command\']\n            require(len(command) == 19, \'Q argv 长度\')\n            require(command[9] == \'--port\', \'Q port 位置\')\n            self.port = int(command[10])\n            self.structural_db = self.work / \'UNUSED-STRUCTURE.db\'\n            expected_command = [self.provenance[\'python_executable\'], \'-B\', str(driver.ROOT / \'s_readonly_server.py\'),\n                \'--db\', str(self.structural_db), \'--browser\', str(self.work / \'UNUSED-BROWSER.html\'),\n                \'--host\', \'127.0.0.1\', \'--port\', str(self.port), \'--resource-config\',\n                str(driver.HERE / \'fixtures/tb01c/query-resources.json\'), \'--producer-epoch\', \'1\',\n                \'--economic-config\', str(self.work / \'query.json\'),\n                \'--economic-wire-evidence\', str(self.output / \'q-owner-wire\')]\n            q, _ = self.process.consume(expected_command, self.name + \'-q\')\n            self.server_processes.append(q)\n            first = self.sequence + 1\n            shared = None\n            while True:\n                self.sequence += 1\n                transport = self.read_json(f\'{self.sequence:04d}-transport.json\')\n                attempt = transport[\'startup_ready_attempt\']\n                for kind in (\'request\', \'response\'):\n                    raw = self.read_bytes(transport[kind])\n                    maximum = driver.FRAME_BYTES + 16384 if kind == \'response\' else driver.FRAME_BYTES\n                    require(len(raw) <= maximum, \'Q startup 原 frame 字节界\')\n                if shared is None:\n                    shared = attempt\n                if attempt[\'phase\'] == \'complete\':\n                    break\n                require(attempt[\'phase\'] == \'connect\', \'Q startup 未成功且非连接拒绝\')\n            clock_anchors = self.clock_anchors\n            work_start = clock_anchors[\'work_deadline\'] - driver.WORK_SECONDS\n            self.work_start = work_start\n            require(self.process.start <= work_start <= self.process.spawns[0][\'monotonic\'], \'driver t0 原来源区间\')\n            require(self.server_processes[-2][\'monotonic\'] <= clock_anchors[\'started_at\'] <= q[\'monotonic\'] <= shared[\'attempt_started_at\'], \'Q 启动同源时间次序\')\n            require(clock_anchors[\'case_deadline\'] <= min(clock_anchors[\'work_deadline\'], first_init[\'monotonic\'] + driver.CASE_SECONDS), \'case 期限续租\')\n            require(clock_anchors[\'case_deadline\'] > first_init[\'monotonic\'], \'case 初始期限已过\')\n            self.read_bytes(\'Q-STARTUP.json\')\n            self.read_bytes(\'port.json\')\n            binder.verify_q_startup(self.output, manifest=self.manifest, port=self.port,\n                started_at=clock_anchors[\'started_at\'], case_deadline=clock_anchors[\'case_deadline\'], work_deadline=clock_anchors[\'work_deadline\'],\n                first_sequence=first, success_sequence=self.sequence)\n            proof = self.read_json(\'Q-STARTUP.json\')\n            require(proof[\'validated_at\'] <= self.process.end, \'Q 核验越出 driver 生命期\')\n            for serve in self.server_processes:\n                require(serve[\'reaped\'][\'monotonic\'] >= proof[\'validated_at\'], \'Q 就绪前 owner 已退出\')\n            require(not self.structural_db.exists(), \'经济路由产生结构 DB\')\n            self.startup = True\n\n        def finish(self):\n            equal(self.business, [row[\'message\'] for row in self.schedule], \'完整业务步骤\')\n            require(self.pending is None and self.startup, \'步骤未闭合\')\n            for serve in self.server_processes:\n                require(serve[\'reaped\'][\'returncode\'] in (0, -signal.SIGTERM), \'服务异常退出\')\n            require(not self.structural_db.exists(), \'经济路由产生结构 DB\')\n    return ReplayCase\n\n\ndef verify_runtime(output: Path, build_evidence: dict, provenance: dict) -> dict:\n    """离线重放完整 41 步，成功只返回 VERIFIED_C6A_SEMANTICS。\n\n    provenance 由外层持有；函数不将 driver RESULT 自述提升为外部事实。\n    本函数所有 I/O 仍必须在外层硬截止/资源监督 helper 中运行。\n    """\n    required = {\'checkout_root\', \'binaries_dir\', \'python_executable\', \'source_sha\', \'source_tree\',\n                \'source_files\', \'binaries_before\', \'binaries_after\', \'driver_started_monotonic\',\n                \'driver_finished_monotonic\', \'artifact_manifest_sha256\', \'case_startup_anchors\'}\n    require(required <= set(provenance), \'缺独立外层 provenance\')\n    output = Path(output).resolve()\n    driver = load_driver(provenance[\'checkout_root\'], provenance[\'source_files\'])\n    binder = binder_for(driver)\n    driver.verify_build(build_evidence, provenance[\'source_sha\'], provenance[\'source_tree\'],\n                        provenance[\'source_files\'], provenance[\'binaries_before\'])\n    equal(provenance[\'binaries_before\'], provenance[\'binaries_after\'], \'运行期间二进制改变\')\n    inventory = binder.verify_inventory(output, output / \'ARTIFACTS.json\', provenance[\'artifact_manifest_sha256\'])\n    actual = set()\n    for path in output.rglob(\'*\'):\n        require(not path.is_symlink(), \'原件树禁止符号链接\')\n        if path.is_file():\n            actual.add(path.relative_to(output).as_posix())\n    equal(actual - {\'ARTIFACTS.json\', \'RESULT.json\'}, inventory, \'ARTIFACTS 全普通文件精确覆盖\')\n    for marker in (\'CANCELLED-AFTER-SEAL.json\', \'FINALIZATION-REJECTED.json\', \'FINALIZATION-FAILED.json\'):\n        require(marker not in actual, \'取消/迟到/封口失败原件\')\n    receipt = binder.load(binder.regular_file(output, \'RESULT.json\'))\n    equal(set(receipt), {\'status\', \'scope\', \'results\', \'primary_error\', \'cleanup_error\', \'cancellation\',\n                        \'candidate_only\', \'outer_acceptance_requires\', \'remaining_children\', \'work_seconds\',\n                        \'final_seconds\', \'elapsed\', \'artifact_manifest_sha256\', \'not_proven\'}, \'driver RESULT 完整字段\')\n    equal(receipt[\'scope\'], \'TestOnly WE5/WE7 public Q normal\', \'driver scope\')\n    equal(receipt[\'outer_acceptance_requires\'], [\'exit_zero\', \'no_cancellation\', \'no_failure_markers\',\n          \'complete_verified_originals\', \'group_reaped_with_owned_identity\'], \'外层责任不可删除\')\n    equal(receipt[\'not_proven\'], [\'complete C6\', \'FIN/2\', \'real S/gamma\', \'per_leg success\',\n          \'C7 recovery\', \'frontend\', \'1394 readiness\'], \'未证实边界不可删除\')\n    equal(receipt[\'status\'], \'CANDIDATE_C6A_NORMAL_ONLY\', \'driver 仅候选正常终态\')\n    require(receipt[\'candidate_only\'] is True and receipt[\'primary_error\'] is None and\n            receipt[\'cleanup_error\'] is None and receipt[\'cancellation\'] is None and\n            receipt[\'remaining_children\'] == [], \'driver 候选含失败\')\n    equal(receipt[\'artifact_manifest_sha256\'], provenance[\'artifact_manifest_sha256\'], \'候选清单身份\')\n    equal((receipt[\'work_seconds\'], receipt[\'final_seconds\']), (600, 630), \'driver 原时限\')\n    require(finite(receipt[\'elapsed\']) and 0 <= receipt[\'elapsed\'] < 630, \'driver 晚候选\')\n    seed, plan, oracle = driver.load_inputs()\n    schedule = expected_steps(driver, seed, plan, oracle)\n    source = binder.load(binder.regular_file(output, \'SOURCE.json\'))\n    equal(source, {\'base\': driver.BASE, \'base_tree\': driver.BASE_TREE, \'build\': build_evidence,\n        \'inputs\': driver.INPUT_HASHES, \'q_resource_sha256\': driver.sha(driver.HERE / \'fixtures/tb01c/query-resources.json\'),\n        \'limits\': {\'work\': 600, \'per_case\': 300, \'final\': 630, \'startup\': 10, \'exchange\': 30, \'cli\': 30,\n                   \'term_grace\': 3, \'kill_grace\': 3}, \'hard_io_deadline\': False}, \'完整 SOURCE 原合同\')\n    allowed = {\'SOURCE.json\'}\n    processes = ProcessLedger(output, binder, provenance, allowed)\n    for command, label, expected in (([\'git\', \'rev-parse\', \'HEAD\'], \'source-head\', provenance[\'source_sha\']),\n            ([\'git\', \'rev-parse\', \'HEAD^{tree}\'], \'source-tree\', provenance[\'source_tree\']),\n            ([\'git\', \'status\', \'--porcelain\', \'--untracked-files=all\'], \'source-clean\', \'\')):\n        row, raw = processes.consume(command, label)\n        equal(raw, (expected + \'\\n\').encode() if expected else b\'\', \'源身份 CLI 原件 \' + label)\n    ReplayCase = replay_case_class(driver, binder)\n    def check_saved(path, value):\n        path = Path(path)\n        relative = path.relative_to(output).as_posix()\n        actual_value = binder.load(binder.regular_file(output, relative))\n        equal(driver.canonical(actual_value), driver.canonical(value), \'公共/独立 oracle 保存原件 \' + relative)\n        allowed.add(relative)\n    results, traces = [], {}\n    clock_origin = None\n    with patch.object(driver, \'save\', check_saved):\n        for name in (\'we5\', \'we7\'):\n            manifest = seed[\'manifest\'] if name == \'we5\' else driver.we7_manifest(seed, oracle)\n            case = ReplayCase(name, output, manifest, processes, provenance, schedule[name], allowed)\n            result = driver.execute_we5(case, plan) if name == \'we5\' else driver.execute_we7(case, oracle)\n            results.append(result)\n            if clock_origin is None:\n                clock_origin = case.work_start\n            equal(case.work_start, clock_origin, \'两 case 必须同一 driver t0\')\n            traces[name] = {\'steps\': len(case.business), \'transports\': case.sequence}\n            for i in range(len(case.nodes)):\n                for suffix in (\'.db\', \'.db-wal\', \'.db-shm\'):\n                    allowed.add(f\'{name}/state/{i}{suffix}\')\n    require(processes.rows[-1][\'monotonic\'] <= clock_origin + receipt[\'elapsed\'] <= processes.end, \'候选 elapsed 与进程账同源\')\n    equal(processes.position, len(processes.spawns), \'多余子进程\')\n    equal(receipt[\'results\'], results, \'内部候选必须对应外层完整重放结果\')\n    require(inventory <= allowed, \'未消费或非合同原件 \' + repr(sorted(inventory - allowed)))\n    return {\'status\': \'VERIFIED_C6A_SEMANTICS\', \'candidate_only\': True, \'steps\': 41,\n            \'cases\': traces, \'results\': results, \'artifact_manifest_sha256\': provenance[\'artifact_manifest_sha256\'],\n            \'startup_clock_basis\': \'独立外层 case_startup_anchors；缺锚拒绝，禁止 proof/transport 反取\',\n            \'not_proven\': [\'outer guardian/resource/deadline/cancellation acceptance\', \'complete C6\', \'full #1323 acceptance\']}\n\n\nBINDER_CODE = \'import hashlib, json, math, errno, signal\\nfrom pathlib import Path, PurePosixPath\\nINNER_LIMITS={"startup":10}\\ndef require(condition, message):\\n    if not condition:\\n        raise ValueError(message)\\n\\ndef equal(actual, expected, message):\\n    driver.equal(actual, expected, message)\\n\\ndef load(path):\\n    # 日志含 monotonic 小数；仍拒绝重复键及 NaN/Infinity。\\n    def unique(pairs):\\n        result = {}\\n        for key, value in pairs:\\n            require(key not in result, "原件 JSON 重复键")\\n            result[key] = value\\n        return result\\n    def bad_number(value):\\n        raise ValueError("原件 JSON 非有限数值：" + value)\\n    def finite_float(value):\\n        number = float(value)\\n        require(math.isfinite(number), "原件 JSON 非有限数值：" + value)\\n        return number\\n    return json.loads(Path(path).read_bytes(), object_pairs_hook=unique,\\n                      parse_constant=bad_number, parse_float=finite_float)\\n\\ndef regular_file(root, name):\\n    require(type(name) is str, "原件路径须为字符串")\\n    root, relative = Path(root).resolve(), PurePosixPath(name)\\n    require(name == relative.as_posix(), "原件路径不是规范 POSIX 相对路径")\\n    require(not relative.is_absolute() and relative.parts and\\n            all(p not in ("..", ".") for p in relative.parts), "原件路径越界")\\n    path = root\\n    for component in relative.parts:\\n        path = path / component\\n        require(not path.is_symlink(), "原件禁止符号链接")\\n    require(path.is_file() and path.resolve().is_relative_to(root), "原件不是普通文件")\\n    return path\\n\\ndef verify_inventory(root, manifest, expected_manifest_sha):\\n    """验证已保存清单，不重新生成清单给伪造文件背书。调用方另核覆盖范围。"""\\n    equal(driver.sha(manifest), expected_manifest_sha, "原清单身份")\\n    rows = load(manifest)["files"]\\n    require(type(rows) is list and rows, "缺原件清单")\\n    seen = set()\\n    for row in rows:\\n        equal(set(row), {"path", "bytes", "sha256"}, "原件条目字段")\\n        path = regular_file(root, row["path"])\\n        require(row["path"] not in seen, "原件清单重复路径")\\n        seen.add(row["path"])\\n        require(type(row["bytes"]) is int and row["bytes"] >= 0, "原件长度非法")\\n        equal(path.stat().st_size, row["bytes"], "原件长度")\\n        equal(driver.sha(path), row["sha256"], "原件摘要")\\n    return seen\\n\\ndef verify_capture(payload, receipt, before, after):\\n    """H3：Capture 原件/事实及 Recorded cut 必须指向同一次 after owner。\\n\\n    只处理本轨首次成功 Capture。精确重送由外层独立比较原回执与不变图像。\\n    不实现金额 reducer，不从回执或 owner 反向生成原事实预期。\\n    """\\n    equal(payload["op"], "CaptureExternal", "Capture 操作")\\n    equal(receipt["kind"], "Recorded", "Capture 回执类型")\\n    equal(receipt["domain_id"], "E", "Capture 回执角色")\\n    event = driver.event_at(payload["raw_utf8"], payload["json_pointer"])\\n    key = driver.fact_key(event)\\n    expected = {"fact_key": key,\\n                "fact_hash": driver.fact_hash(payload["raw_utf8"], payload["json_pointer"]),\\n                "raw_utf8": payload["raw_utf8"], "json_pointer": payload["json_pointer"],\\n                "raw_sha256": hashlib.sha256(payload["raw_utf8"].encode()).hexdigest(),\\n                "event": event, "token": payload["token"],\\n                "source_event_time": payload["source_event_time"]}\\n    for owner in (before, after):\\n        equal(owner["cut"]["domain_id"], "E", "Capture owner 域")\\n        equal(owner["image_hash"], driver.digest(owner["image"]), "owner 图像摘要")\\n    equal(receipt["fact_key"], key, "Capture 回执事实")\\n    equal(receipt["fact_hash"], expected["fact_hash"], "Capture 回执事实内容")\\n    equal(receipt["token"], payload["token"], "Capture 回执 token")\\n    equal(receipt["cut"], after["cut"], "Recorded cut 未绑定 after owner")\\n    equal(int(after["cut"]["commit_seq"]), int(before["cut"]["commit_seq"]) + 1,\\n          "首次 Capture 单次提交")\\n    require(not any(row["fact_key"] == key for row in before["image"]["recorded"]),\\n            "首次 Capture 已有原事实")\\n    records = [row for row in after["image"]["recorded"] if row["fact_key"] == key]\\n    equal(len(records), 1, "Capture after 缺事实或重复")\\n    for field, value in expected.items():\\n        equal(records[0][field], value, "Capture after 原件/事实 " + field)\\n\\ndef verify_q_startup(root, *, manifest, port, started_at, case_deadline,\\n                     work_deadline, first_sequence, success_sequence):\\n    """核真实 driver 的 Q-STARTUP/transport/wire 原件，不执行网络操作。\\n\\n    参数必须来自调用者可信的 manifest、端口分配、同一单调时钟和完整序列账本，\\n    不得从待验 proof/transport 反取。外层仍须证明来源、清单覆盖与监督真实性。\\n    """\\n    for value in (started_at, case_deadline, work_deadline):\\n        require(type(value) in (int, float) and math.isfinite(value), "Q startup 可信时钟非法")\\n    require(type(port) is int and 0 < port <= 65535, "Q startup 可信端口非法")\\n    require(type(first_sequence) is int and type(success_sequence) is int and\\n            0 < first_sequence <= success_sequence, "Q startup 可信序列非法")\\n    deadline = min(started_at + INNER_LIMITS["startup"], case_deadline, work_deadline)\\n    require(started_at < deadline, "Q startup 可信期限已过")\\n    shared = {"schema_revision": "c6a-q-startup/1", "started_at": started_at,\\n              "deadline": deadline, "case_deadline": case_deadline, "work_deadline": work_deadline}\\n    proof = load(regular_file(root, "Q-STARTUP.json"))\\n    equal(set(proof), set(shared) | {"first_sequence", "success_sequence", "validated_at",\\n                                    "success_response_sha256"}, "Q startup proof 字段")\\n    for key, expected in shared.items():\\n        if key != "schema_revision":\\n            require(type(proof[key]) in (int, float) and math.isfinite(proof[key]),\\n                    "Q startup proof 时钟类型")\\n        equal(proof[key], expected, "Q startup 可信锚 " + key)\\n    for key, expected in (("first_sequence", first_sequence), ("success_sequence", success_sequence)):\\n        require(type(proof[key]) is int, "Q startup proof 序列类型")\\n        equal(proof[key], expected, "Q startup 首末序列")\\n    validated = proof["validated_at"]\\n    require(type(validated) in (int, float) and math.isfinite(validated) and\\n            started_at <= validated < deadline, "Q startup 晚到或非法核验时间")\\n    port_record = load(regular_file(root, "port.json"))\\n    equal(port_record, {"host": "127.0.0.1", "port": port, "released_before_child_bind": True,\\n                        "collision_policy": "FAIL without touching occupant"}, "Q startup 端口原件")\\n    request = (f"GET /api/economic/ready HTTP/1.0\\\\r\\\\nHost: 127.0.0.1:{port}\\\\r\\\\n"\\n               "Content-Type: application/json\\\\r\\\\nContent-Length: 0\\\\r\\\\nConnection: close\\\\r\\\\n\\\\r\\\\n").encode()\\n    previous = started_at\\n    ready = None\\n    for sequence in range(first_sequence, success_sequence + 1):\\n        label = f"{sequence:04d}"\\n        transport = load(regular_file(root, label + "-transport.json"))\\n        equal(set(transport), {"request", "response", "request_sha256", "response_sha256",\\n                               "response_bytes", "http", "address", "startup_ready_attempt"},\\n              "Q startup transport 字段")\\n        require(transport["http"] is True, "Q startup 必须 HTTP")\\n        equal(transport["address"], ["127.0.0.1", port], "Q startup host/port")\\n        for kind in ("request", "response"):\\n            equal(transport[kind], label + f".http-{kind}.bin", "Q startup 具名原件")\\n        raw_request = regular_file(root, transport["request"]).read_bytes()\\n        raw_response = regular_file(root, transport["response"]).read_bytes()\\n        for kind, raw in (("request", raw_request), ("response", raw_response)):\\n            equal(transport[kind + "_sha256"], hashlib.sha256(raw).hexdigest(), "Q startup 原件摘要")\\n        equal(raw_request, request, "Q startup 显式零 body ready GET")\\n        require(type(transport["response_bytes"]) is int, "Q startup 响应长度类型")\\n        equal(transport["response_bytes"], len(raw_response), "Q startup 响应长度")\\n        attempt = transport["startup_ready_attempt"]\\n        equal(set(attempt), set(shared) | {"sequence", "attempt_started_at", "observed_at", "phase",\\n                                           "exception_class", "exception_errno", "connect_completed", "send_entered"},\\n              "Q startup attempt 字段")\\n        for key, expected in shared.items():\\n            if key != "schema_revision":\\n                require(type(attempt[key]) in (int, float) and math.isfinite(attempt[key]),\\n                        "Q startup attempt 时钟类型")\\n            equal(attempt[key], expected, "Q startup attempt 原期限 " + key)\\n        require(type(attempt["sequence"]) is int, "Q startup attempt 序列类型")\\n        equal(attempt["sequence"], sequence, "Q startup 连续序列")\\n        begin, observed = attempt["attempt_started_at"], attempt["observed_at"]\\n        require(all(type(v) in (int, float) and math.isfinite(v) for v in (begin, observed)),\\n                "Q startup attempt 时钟非法")\\n        require(previous <= begin <= observed <= validated and observed < deadline,\\n                "Q startup attempt 逆序或晚到")\\n        previous = observed\\n        if sequence < success_sequence:\\n            equal((attempt["phase"], attempt["exception_class"], attempt["exception_errno"]),\\n                  ("connect", "ConnectionRefusedError", errno.ECONNREFUSED), "Q startup 非连接拒绝不可重试")\\n            require(attempt["connect_completed"] is False and attempt["send_entered"] is False,\\n                    "Q startup 重试已连接或已发送")\\n            equal(raw_response, b"", "Q startup 重试必须零响应")\\n        else:\\n            equal((attempt["phase"], attempt["exception_class"], attempt["exception_errno"]),\\n                  ("complete", None, None), "Q startup 成功阶段")\\n            require(attempt["connect_completed"] is True and attempt["send_entered"] is True,\\n                    "Q startup 成功未完成连接发送")\\n            equal(proof["success_response_sha256"], transport["response_sha256"], "Q startup 成功原件绑定")\\n            ready = driver.parse_http(raw_response)\\n    equal(ready["manifest_hash"], driver.digest(manifest), "Q startup manifest 身份")\\n    equal(ready["book_revision"], "tb03c-book/1", "Q startup book 合同")\\n    equal((ready["session_id"], ready["session_generation"], ready["producer_epoch"]),\\n          (manifest["session_id"], manifest["session_generation"], "1"), "Q startup 会话/epoch 身份")\\n    domains = ["E"] + ["B:" + node["chong_id"] for node in manifest["chongs"]]\\n    require(type(ready["domains"]) is list and all(type(d) is str for d in ready["domains"]),\\n            "Q startup domains 类型")\\n    equal(sorted(ready["domains"]), sorted(domains), "Q startup owner 集合")\\n    return ready\\n\\ndef verify_processes(events):\\n    """按 spawn 实例闭合，拒绝没有 TERM 原件的 -15；CLI 永须 exit0。"""\\n    active = {}\\n    spawned = 0\\n    previous = -math.inf\\n    for event in events:\\n        now = event["monotonic"]\\n        require(type(now) in (int, float) and math.isfinite(now) and now >= previous,\\n                "进程原件时序非法")\\n        previous = now\\n        pid, kind = event["pid"], event["event"]\\n        require(type(pid) is int and pid > 0, "进程身份非法")\\n        if kind == "spawn":\\n            require(pid not in active, "进程实例重叠")\\n            require(event["direct_child"] is True and event["inherits_driver_group"] is True,\\n                    "进程未绑定 driver 组")\\n            command = event["command"]\\n            require(type(command) is list and command and all(type(x) is str for x in command),\\n                    "缺进程命令")\\n            active[pid] = {"command": command, "signals": []}\\n            spawned += 1\\n        else:\\n            require(pid in active, "未知或已回收进程")\\n            child = active[pid]\\n            if kind == "signal":\\n                require(event["returncode_before"] is None, "已退出进程不得发信号")\\n                child["signals"].append(event["signal"])\\n            elif kind == "reaped":\\n                code, command = event["returncode"], child["command"]\\n                require(type(code) is int, "退出码非法")\\n                service = (len(command) > 1 and command[1] == "serve") or any(\\n                    Path(arg).name == "s_readonly_server.py" for arg in command)\\n                require(code == 0 or (service and code == -signal.SIGTERM and\\n                                     child["signals"] == [signal.SIGTERM]),\\n                        "非零 CLI 或缺 TERM 原件的退出")\\n                require(signal.SIGKILL not in child["signals"], "正常轨出现 KILL 升级")\\n                del active[pid]\\n            else:\\n                require(kind in ("wait", "wait-timeout", "signal-esrch"), "未知进程事件")\\n    require(spawned > 0 and not active, "进程清单为空或未全部回收")\'\n'
SEMANTICS_SHA256 = '76a8386d0fa750a53e0e9c4800131d6da552996e8715769149c28fdcf624fc56'

def verify_runtime(output, build, provenance):
    """内嵌独立离线候选；READY仍由独立评审门控制。"""
    require(hashlib.sha256(SEMANTICS_SOURCE.encode()).hexdigest() == SEMANTICS_SHA256, "embedded semantics bytes changed")
    namespace = {"__name__": "c6a_offline_semantics"}
    exec(compile(SEMANTICS_SOURCE, "<c6a-offline-semantics>", "exec"), namespace)
    return namespace["verify_runtime"](output, build, provenance)


def inventory(root, exclude=None):
    special, rows, excluded = [], [], []
    for path in regular_files(root, special=special, exclude=exclude, excluded=excluded):
        relative = str(path.relative_to(root))
        rows.append({"path": relative, "bytes": path.stat().st_size, "sha256": digest(path)})
    return {"files": rows, "non_regular_entries": special, "excluded_observed_files": excluded}


def finalization_file(name, sequence):
    # 仅证据根的当前封口/资源监督文件；raw-evidence内同名文件永不排除。
    if "/" in name:
        return False
    # 仅save()精确形状的临时名；不能把一般隐藏文件或raw-evidence中的同名原件放宽。
    if name.startswith(".") and name.endswith(".tmp"):
        name = name[1:-4]
    if name in {"FILES.json", "FILES.SEAL.json", "RESULT.json", "LATE-OR-CANCELLED.json", "CANCELLED-AFTER-SEAL.json"}:
        return True
    match = re.match(r"^(?:probe-|request-)?([0-9]{5,})[-.]", name)
    return match is not None and int(match.group(1)) >= sequence


def worker(operation, request):
    require(sys.platform == "linux" and os.environ.get("GITHUB_ACTIONS") == "true", "remote Linux workers only")
    if operation == "resources":
        value = snapshot(set(request["groups"]), request["filesystem"])
        save(request["output"], value)
    elif operation == "identity":
        save(request["output"], identity(Path(request["root"])))
    elif operation == "build-evidence":
        value = build_evidence(Path(request["root"]), Path(request["binaries"]), request["command"],
                               request["exit"], request["source"])
        save(request["output"], value)
    elif operation == "verify":
        root, output = Path(request["root"]), Path(request["runtime"])
        build = parse_json(Path(request["build"]).read_bytes())
        source = identity(root)
        require(source == {key: build[key] for key in ("source_sha", "source_tree")}, "post-runtime source identity")
        after = {name: digest(Path(request["binaries"]) / name) for name in BINARIES}
        require(after == build["binaries"], "post-runtime binary hash")
        rows = inventory(output)
        outer_manifest = Path(request["output"]).with_name("RUNTIME-INVENTORY.json")
        save(outer_manifest, rows)
        provenance = {**source, "source_files": build["source_files"], "binaries_before": build["binaries"],
            "binaries_after": after, "driver_started_monotonic": request["started"],
            "driver_finished_monotonic": request["finished"], "checkout_root": str(root),
            "binaries_dir": request["binaries"], "python_executable": request["python"],
            "artifact_manifest_sha256": digest(output / "ARTIFACTS.json"),
            "outer_inventory_sha256": digest(outer_manifest)}
        provenance["case_startup_anchors"] = verify_clock_transcript(request["clock_transcript"], request["started"], request["finished"])
        provenance["clock_transcript"] = request["clock_transcript"]
        receipt = parse_json((output / "RESULT.json").read_bytes())
        # Driver receipt在finalize中保存elapsed/原额度；工作起点仍以外层见证为准。
        require((receipt["work_seconds"], receipt["final_seconds"]) == (600, 630), "runtime limits changed")
        elapsed = receipt["elapsed"]
        require(type(elapsed) in (int, float) and math.isfinite(elapsed) and 0 <= elapsed < 630
                and request["started"] + elapsed <= request["finished"], "runtime elapsed/work anchor mismatch")
        value = verify_runtime(output, build, provenance)
        require(value.get("status") == "VERIFIED_C6A_SEMANTICS", "semantic result missing")
        save(request["output"], value)
    elif operation == "archive":
        source, destination = Path(request["runtime"]), Path(request["raw"])
        require(source == destination, "runtime must already be in upload scope; never move live evidence")
        if source.exists():
            require(source.stat().st_dev == destination.parent.stat().st_dev, "raw evidence must stay on filesystem")
        if destination.exists():
            save(Path(request["manifest"]), inventory(destination))
            with tarfile.open(request["tar"], "x:gz") as archive:
                for path in regular_files(destination):
                    archive.add(path, arcname=str(path.relative_to(destination)), recursive=False)
        save(request["output"], {"preserved": destination.exists(), "moved": False,
                                 "sealed_after_group_reap": destination.exists()})
    elif operation == "inventory":
        sequence = request["finalization_sequence"]
        value = inventory(Path(request["artifacts"]), lambda name: finalization_file(name, sequence))
        # 完整原件全部上传；只有根层封口及其正在写的资源日志不作自引用哈希。
        value["excluded_finalization_sequence_from"] = sequence
        value["exclusion_rule"] = "root final receipts and supervisor/probe/request files with sequence >= threshold; never raw-evidence"
        save(request["manifest"], value)
        save(request["output"], {"manifest_sha256": digest(request["manifest"]), "files": len(value["files"])})
    else:
        raise RuntimeError("unknown helper")


def activate_linux():
    require(sys.platform == "linux", "Linux required")
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "SIGCHLD must be default")
    libc = ctypes.CDLL(None, use_errno=True)
    require(libc.prctl(36, 1, 0, 0, 0) == 0, "PR_SET_CHILD_SUBREAPER failed")
    require(hasattr(os, "WNOWAIT"), "WNOWAIT required")
    signal.signal(signal.SIGTERM, remember_signal)
    signal.signal(signal.SIGINT, remember_signal)


def real_run():
    global SEALED_CONTEXT
    require(CONTRACT_READY and SEMANTICS_READY, "PENDING: contract/semantics not independently reviewed")
    require(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("GITHUB_EVENT_NAME") == "push"
            and os.environ.get("GITHUB_REF") == "refs/heads/validate/1406-c6a", "dedicated push only")
    activate_linux()
    t0 = int(os.environ["C6A_T0_NS"]) / 1_000_000_000
    require(math.isfinite(t0) and 0 <= clock() - t0 < ENDPOINTS["build"] - REAP_RESERVE, "invalid original t0")
    root, temp = Path(os.environ["GITHUB_WORKSPACE"]).resolve(), Path(os.environ["RUNNER_TEMP"]).resolve()
    artifacts = temp / "1406-c6a-artifacts"
    artifacts.mkdir(mode=0o700)
    scratch = Path(tempfile.mkdtemp(prefix="1406-c6a-", dir=temp))
    # driver自行创建output；从首次创建就处于always上传范围，scratch只放编译target。
    runtime, binaries = artifacts / "raw-evidence", scratch / "target/debug"
    env = dict(os.environ, CARGO_TARGET_DIR=str(scratch / "target"), CARGO_INCREMENTAL="0", PYTHONDONTWRITEBYTECODE="1")
    supervisor = LinuxSupervisor(root, artifacts, scratch, env, t0)
    result = {"status": "FAIL_C6A", "source": None, "scratch": str(scratch), "errors": [],
              "runtime": str(runtime), "runtime_upload_relative_path": "raw-evidence",
              "raw_evidence_state": "UNSEALED", "delivery": "NOT_VERIFIED_REMOTE_UPLOAD"}
    try:
        require(artifacts.stat().st_dev == scratch.stat().st_dev, "evidence filesystem differs")
        initial = supervisor.probe([], supervisor.end("build"))
        require(initial["free"] >= RESOURCE_LIMITS["initial_free"]
                and initial["available"] >= RESOURCE_LIMITS["initial_available"], "initial resources")
        supervisor.initial_free = initial["free"]
        save(artifacts / "CONFIGURATION.json", {"t0": t0, "endpoints": ENDPOINTS,
             "resources": RESOURCE_LIMITS, "poll_seconds": POLL_SECONDS, "initial": initial})
        source_path = artifacts / "SOURCE.json"
        supervisor.run("identity", supervisor.worker_command("identity", {"root": str(root), "output": str(source_path)}), supervisor.end("build"))
        source = small_json(source_path)
        result["source"] = source
        command = ["cargo", "build", "--manifest-path", str(root / "rust/Cargo.toml"), "--locked",
                   "--features", "s_session", "--jobs", "2", "--bin", BINARIES[0], "--bin", BINARIES[1]]
        build = supervisor.run("build", command, supervisor.end("build"), rss_limit=RESOURCE_LIMITS["build_rss"])
        evidence_path = artifacts / "build-evidence.json"
        request = {"root": str(root), "binaries": str(binaries), "command": command,
                   "exit": build["exit"], "source": source, "output": str(evidence_path)}
        supervisor.run("build-evidence", supervisor.worker_command("build-evidence", request), supervisor.end("build"))
        command = [sys.executable, "-B", str(root / "s_session/tests/tb03c_query_runtime.py"),
                   "--binaries", str(binaries), "--build-evidence", str(evidence_path), "--output", str(runtime)]
        broker = ClockBroker.create()
        try:
            executed = supervisor.run("runtime", command, supervisor.end("runtime"), broker=broker)
        finally:
            broker.close()
        request = {"root": str(root), "runtime": str(runtime), "build": str(evidence_path), "binaries": str(binaries),
                   "started": executed["started"], "finished": executed["exited"], "python": sys.executable,
                   "clock_transcript": executed["clock_transcript"],
                   "output": str(artifacts / "SEMANTICS.json")}
        supervisor.run("verify", supervisor.worker_command("verify", request), supervisor.end("verify"))
        result["status"] = "CANDIDATE_C6A"
    except Exception as error:
        result["errors"].append(str(error))
    finally:
        # 原件始终在上传面。未回收时不移动、不哈希正在写入的原件，不称完整封存。
        try:
            if supervisor.owned:
                result["raw_evidence_state"] = "UNSEALED_LIVE_OR_UNKNOWN"
            require(not supervisor.owned, "unreaped groups: uploaded raw evidence may still change; unsealed failure")
            request = {"runtime": str(runtime), "raw": str(artifacts / "raw-evidence"),
                       "manifest": str(artifacts / "RAW-INVENTORY.json"), "tar": str(artifacts / "runtime-evidence.tar.gz"),
                       "output": str(artifacts / "ARCHIVE.json")}
            supervisor.run("archive", supervisor.worker_command("archive", request), supervisor.end("archive"), allow_cancel=False)
            archive_receipt = small_json(artifacts / "ARCHIVE.json")
            if result["status"] == "CANDIDATE_C6A":
                require(archive_receipt["sealed_after_group_reap"], "successful runtime has no sealed raw evidence")
            result["raw_evidence_state"] = "SEALED_AFTER_REAP" if archive_receipt["sealed_after_group_reap"] else "NOT_CREATED"
        except Exception as error:
            result["errors"].append("archive: " + str(error))
        try:
            require(not supervisor.owned, "cannot seal with unowned groups")
            request = {"artifacts": str(artifacts), "manifest": str(artifacts / "FILES.json"),
                       "output": str(artifacts / "FILES.SEAL.json"), "finalization_sequence": supervisor.serial + 1}
            supervisor.run("inventory", supervisor.worker_command("inventory", request), supervisor.end("final"), allow_cancel=False)
            result["manifest"] = small_json(artifacts / "FILES.SEAL.json")
        except Exception as error:
            result["errors"].append("seal: " + str(error))
        result["cancelled"], result["unreaped"] = dict(CANCELLATION), sorted(supervisor.owned)
        result["elapsed"] = clock() - t0
        passed = (result["status"] == "CANDIDATE_C6A" and not result["errors"] and not result["unreaped"]
                  and not CANCELLATION["count"] and clock() < supervisor.end("final"))
        result["status"] = "PASS_C6A_CI" if passed else "FAIL_C6A"
        save(artifacts / "RESULT.json", result)
        SEALED_CONTEXT = artifacts
        # handler保持生效到return；内外PASS收据都不是充分证据。
        if CANCELLATION["count"] or clock() >= supervisor.end("final"):
            save(artifacts / "LATE-OR-CANCELLED.json", {"signals": dict(CANCELLATION), "at": clock(), "accepted": False})
            passed = False
    return 0 if passed and not CANCELLATION["count"] and clock() < supervisor.end("final") else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--worker", nargs=2)
    args = parser.parse_args()
    if args.worker:
        worker(args.worker[0], small_json(args.worker[1]))
        return 0
    if args.run:
        return real_run()
    ready = (CONTRACT_READY and SEMANTICS_READY and all(len(value) == 40 and
             all(character in "0123456789abcdef" for character in value) for value in (REVIEWED_SOURCE, REVIEWED_TREE)))
    print(json.dumps({"status": "READY_FOR_REVIEWED_RUN" if ready else "BLOCKED_PREPARATION", "CONTRACT_READY": CONTRACT_READY,
        "source": REVIEWED_SOURCE, "tree": REVIEWED_TREE, "semantics_ready": SEMANTICS_READY,
        "endpoints": ENDPOINTS, "resources": RESOURCE_LIMITS,
        "blockers": ["independent Linux supervisor review", "independent 41-step semantic verifier review",
                     "exact source/two-file tail binding", "old review counterexamples not recovered"]}, ensure_ascii=False))
    return 0 if ready else 2


if __name__ == "__main__":
    raise SystemExit(main())
