#!/usr/bin/env python3
"""#1406 C6a：远端真实 E/B → 正式 Q 公共 HTTP，WE5/WE7 正常路径。

本机只能运行纯数据/替身合同；main 会启动真实产品，须经精确源码独审及远端 CI。
不导入 SQLite/C7a runtime，不恢复、不注入故障、不启动 S 或浏览器。
600 秒工作、630 秒清理/封口共用 t0；Python 检查不是不可中断 I/O 的硬期限。
"""
import argparse
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import threading
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
REPO = ROOT.parent
sys.path.insert(0, str(ROOT))
from s_socket_client import canonical, decode_frame, digest, validate_reply
import tb03c_oracle as we5_oracle

BASE = "4d9b072e5b671fc1a84f7d75b3976471b93d2afb"
BASE_TREE = "e2668b87c3f01c67ed7c8cb4bacc88c62f14e71c"
WORK_SECONDS, CASE_SECONDS, FINAL_SECONDS = 600, 300, 630
STARTUP_SECONDS, EXCHANGE_SECONDS, PROCESS_SECONDS = 10, 30, 30
TERM_SECONDS, KILL_SECONDS = 3, 3
FRAME_BYTES = 1048576
SOURCE = {"source_namespace": "testonly-tb1406", "source_epoch": "fixture-epoch-1",
          "producer_id": "tb1406-c7a", "producer_epoch": "1"}
QUERY_SOURCE = {"source_namespace": "testonly-tb1406-query", "source_epoch": "fixture-epoch-1",
                "producer_id": "tb1406-c6a-query", "producer_epoch": "1"}
QUERY_RESOURCES = {"max_frame_bytes": "1048576", "timeout_ms": "30000", "operation_timeout_ms": "30000",
                   "max_reply_bytes": "1048576", "max_cached_bytes": "4194304", "max_snapshots": "4",
                   "max_domains": "4", "max_changes": "32"}
SOURCE_FILES = (
    "s_session/economic_query.py", "s_session/economic_http.py", "s_session/s_socket_client.py",
    "s_session/s_readonly_server.py", "s_session/s_query.py", "s_session/s_query_integrity.py",
    "s_session/s_tb02_contract.py", "s_session/tests/fixtures/tb01c/query-resources.json",
    "s_session/tests/test_tb03c_query.py", "s_session/tests/tb03c_query_runtime.py",
    "s_session/tests/test_tb03c_query_runtime_contract.py",
)
# 固定对象的原字节身份；运行不能通过重写外部 seed 自行换 oracle。
INPUT_HASHES = {'fixtures/tb03c_runtime/SEED.json': '5f5931dfba64cec634ca5e4bc8cdef2805aaebdb2bdbe3456ba4711cd4d2ed49', 'tb03c_oracle.py': '67e52e8f505f283cf232ff1774248bb60edffaa77fd3255497f59999092689f1', 'fixtures/tb03c/MANIFEST.json': 'bf6c8d8081acdcf9be56f97ceebc3d0081fc9bd71aff098e6991656e337d4648', 'fixtures/tb03c/HAND-ORACLE.json': '39d4af35236c229512a1cec3bfbb07e366ed02896d1aee93e75bee6a8179ba3c', 'fixtures/tb03c/raw/chong-R1.json': '2c7254dd6ec90e93bbf8bc5bcbe82adccd4428def389abf30ac428a04e5990a1', 'fixtures/tb03c/raw/chong-U1.json': '97a8d5c2b7946c64b8d4c88477400b46fde55757437967810637973e4b9b7aea', 'fixtures/tb03c_allocation/RAW.json': '5b56870f8801ffc205683972158c286cfea25bc7672efd732300ac97d89c84e9', 'fixtures/tb03c_allocation/FROZEN-MANIFEST.json': 'd724063f029eaa047fb591f4b953d6e28abaf2c2f27793b548a96ab09c6e51b0', 'fixtures/tb03c_allocation/HAND-ORACLE.json': '213b82b3d9b57b233f5aa8d4ffc4116f34687d3b2713180c21710dfaee0d619d', 'fixtures/tb03c_allocation/test_oracle.py': '4e7a8abbdad5d0089f56ca4152ff5092d2e99da87093fcfe1bd977962a49f5ef'}
INPUT_NAMES = (
    "fixtures/tb03c_runtime/SEED.json", "tb03c_oracle.py", "fixtures/tb03c/MANIFEST.json",
    "fixtures/tb03c/HAND-ORACLE.json", "fixtures/tb03c/raw/chong-R1.json", "fixtures/tb03c/raw/chong-U1.json",
    "fixtures/tb03c_allocation/RAW.json", "fixtures/tb03c_allocation/FROZEN-MANIFEST.json",
    "fixtures/tb03c_allocation/HAND-ORACLE.json", "fixtures/tb03c_allocation/test_oracle.py",
)


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def equal(actual, expected, detail):
    if actual != expected:
        raise ValueError(detail + ": " + json.dumps({"expected": expected, "actual": actual}, ensure_ascii=False, default=repr))


def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(65536), b""):
            h.update(block)
    return h.hexdigest()


def save(path, value):
    with Path(path).open("xb") as stream:
        stream.write(canonical(value) + b"\n")


class Budget:
    def __init__(self, t0=None, clock=time.monotonic):
        self.clock = clock
        self.t0 = clock() if t0 is None else t0
        self.work_end, self.final_end = self.t0 + WORK_SECONDS, self.t0 + FINAL_SECONDS
        self.case_end = self.work_end
        self.cancel = None
        self.sealed_output = None

    def receive_cancel(self, number, frame=None):
        self.cancel = number
        if self.sealed_output is not None:
            # 成功文件只是候选。封口后独立写失败原件，绝不覆盖/重写 RESULT。
            # 即使独立失败原件写入失败，也以非零退出；外层必须同时核退出码。
            try:
                save(self.sealed_output / "CANCELLED-AFTER-SEAL.json",
                     {"status": "FAIL", "cancellation": int(number),
                      "elapsed": self.clock() - self.t0, "candidate_invalid": True})
            finally:
                raise SystemExit(1)

    def until(self, end):
        self.check()
        left = min(end, self.case_end, self.work_end) - self.clock()
        if left <= 0:
            raise TimeoutError("绝对子期限已到；不得重开窗口")
        return left

    def begin_case(self):
        self.check()
        self.case_end = min(self.work_end, self.clock() + CASE_SECONDS)

    def check(self, *, final=False):
        if not final and self.cancel is not None:
            raise InterruptedError("收到取消：" + str(self.cancel))
        end = self.final_end if final else min(self.work_end, self.case_end)
        remaining = end - self.clock()
        if remaining <= 0:
            raise TimeoutError("固定总期限已到；不得重开窗口或晚到 PASS")
        return remaining

    def remaining(self, maximum, *, final=False):
        return min(maximum, self.check(final=final))


class Supervisor:
    """唯一主线程收割直接 child；无 killpg，无后台收割者，无名称杀进程。"""
    def __init__(self, output, budget):
        self.output, self.budget = Path(output), budget
        self.children = {}
        self.sequence = 0
        self.thread_id = threading.get_ident()
        self.events = []
        self.log = (self.output / "processes.jsonl").open("xb")

    def event(self, entry):
        row = {"monotonic": self.budget.clock(), **entry}
        self.events.append(row)
        self.log.write(canonical(row) + b"\n")
        self.log.flush()

    def identity(self, proc):
        require(threading.get_ident() == self.thread_id, "只允许登记的主线程收割 child")
        require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "SIGCHLD 必须为默认，禁止外部收割")
        require(id(proc) in self.children and self.children[id(proc)]["proc"] is proc, "不是仍登记的直接 child")

    def spawn(self, command, label, *, cwd=REPO):
        self.budget.check()
        require(threading.get_ident() == self.thread_id, "只能主线程创建 child")
        require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "SIGCHLD 不是默认")
        self.sequence += 1
        name = f"{self.sequence:04d}-{label}"
        stdout, stderr = self.output / (name + ".stdout"), self.output / (name + ".stderr")
        # TERM/INT handler 只标记取消，不在 Popen 返回与登记之间抛异常。
        with stdout.open("xb") as out, stderr.open("xb") as err:
            proc = subprocess.Popen(command, cwd=cwd, stdout=out, stderr=err, stdin=subprocess.DEVNULL,
                                    start_new_session=False, close_fds=True)
            self.children[id(proc)] = {"proc": proc, "command": list(command), "stdout": stdout,
                                       "stderr": stderr, "label": label}
        self.event({"event": "spawn", "pid": proc.pid, "command": list(command),
                    "stdout": stdout.name, "stderr": stderr.name, "direct_child": True, "inherits_driver_group": True})
        self.budget.check()
        return proc

    def _reaped(self, proc):
        self.identity(proc)
        require(proc.returncode is not None, "尚未回收，不能注销")
        child = self.children[id(proc)]
        self.event({"event": "reaped", "pid": proc.pid, "returncode": proc.returncode,
                    "label": child["label"], "stdout": child["stdout"].name, "stderr": child["stderr"].name})
        del self.children[id(proc)]
        return child

    def run(self, command, label):
        proc = self.spawn(command, label)
        self.identity(proc)
        proc.wait(timeout=self.budget.remaining(PROCESS_SECONDS))
        child = self._reaped(proc)
        require(proc.returncode == 0, "CLI 非零退出；原 stdout/stderr 已保留：" + label)
        self.budget.check()
        return child["stdout"].read_bytes()

    def stop(self, proc):
        self.identity(proc)
        if proc.returncode is not None:
            return self._reaped(proc)
        child = self.children[id(proc)]
        require(not child.get("cleanup_attempted"), "本句柄已尝试完整回收；禁止超时后重开信号窗口")
        child["cleanup_attempted"] = True
        for sig, grace in ((signal.SIGTERM, TERM_SECONDS), (signal.SIGKILL, KILL_SECONDS)):
            self.budget.check(final=True)
            self.identity(proc)
            if proc.returncode is not None:
                return self._reaped(proc)
            # 此处没有 poll/wait：未收割的直接 child（含 zombie）仍占有原 PID。
            self.event({"event": "signal", "pid": proc.pid, "signal": int(sig), "returncode_before": proc.returncode})
            try:
                os.kill(proc.pid, sig)
            except ProcessLookupError:
                # 仍须唯一 wait 者确认；不改按名称/PGID 搜寻。
                self.event({"event": "signal-esrch", "pid": proc.pid, "signal": int(sig)})
            self.event({"event": "wait", "pid": proc.pid, "grace_seconds": grace})
            try:
                proc.wait(timeout=self.budget.remaining(grace, final=True))
            except subprocess.TimeoutExpired:
                self.event({"event": "wait-timeout", "pid": proc.pid, "signal": int(sig)})
                continue
            return self._reaped(proc)
        raise TimeoutError("直接 child 在最后 KILL/wait 后仍未回收；保留登记和原件")

    def close(self):
        failures = []
        for child in list(self.children.values()):
            try:
                self.stop(child["proc"])
            except BaseException as error:
                failures.append({"pid": child["proc"].pid, "error": repr(error)})
        if failures:
            raise RuntimeError("清理未完成：" + json.dumps(failures))
        self.log.flush()


def load_inputs():
    equal(set(INPUT_HASHES), set(INPUT_NAMES), "固定来源集合")
    for name, expected in INPUT_HASHES.items():
        equal(sha(HERE / name), expected, "原件/oracle hash " + name)
    seed = decode_frame((HERE / INPUT_NAMES[0]).read_bytes())
    hand = decode_frame((HERE / "fixtures/tb03c/HAND-ORACLE.json").read_bytes())
    case = hand["e_cases"][0]
    equal((case["chong_id"], seed["e_case_index"]), ("chong-R1", 0), "WE5 身份")
    equal(we5_oracle.run_e_case(HERE / "fixtures/tb03c", case), (len(case["steps"]), []), "WE5 独立 RAW/HAND oracle")
    spec = importlib.util.spec_from_file_location("c6a_we7_oracle", HERE / "fixtures/tb03c_allocation/test_oracle.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    ledger = module.IndependentLedger()
    for index, expected in enumerate(module.HAND["reports"], 3):
        key = ledger.capture(index)
        ledger.resolve(key)
        for owner in range(3):
            ledger.apply(key, owner)
        equal(ledger.q, expected["book_quantity"], "WE7 独立数量/HAND")
        equal(ledger.k, expected["K"], "WE7 独立账目/HAND")
    return seed, case, module


def verify_build(build, checkout, tree, source_hashes, binary_hashes):
    required = {"source_sha", "source_tree", "command", "exit_code", "source_files", "binaries"}
    equal(set(build), required, "构建证据字段")
    require(type(build["exit_code"]) is int and build["exit_code"] == 0, "构建未成功")
    require(type(build["command"]) is list and bool(build["command"])
            and all(type(x) is str and x for x in build["command"]), "缺完整构建命令")
    for value in (checkout, tree):
        require(type(value) is str and len(value) == 40 and all(c in "0123456789abcdef" for c in value), "源码对象身份不完整")
    equal(build["source_sha"], checkout, "构建与精确 HEAD 不同")
    equal(build["source_tree"], tree, "构建与精确 tree 不同")
    equal(set(source_hashes), set(SOURCE_FILES), "源码绑定集合")
    equal(build["source_files"], source_hashes, "构建源码字节不同")
    equal(set(binary_hashes), {"e_economic_session", "b_chong_session"}, "二进制集合")
    equal(build["binaries"], binary_hashes, "实际二进制 hash 不同")


def event_at(raw, pointer):
    parts = pointer.split("/")
    require(len(parts) == 3 and parts[1] == "events" and parts[2].isdigit(), "原件指针不合法")
    return decode_frame(raw.encode())["events"][int(parts[2])]


def fact_key(event):
    return digest(list(we5_oracle.fact_identity(event)))


def fact_hash(raw, pointer):
    doc = decode_frame(raw.encode())
    return digest({"raw_sha256": hashlib.sha256(raw.encode()).hexdigest(), "json_pointer": pointer,
                   "event": event_at(raw, pointer), "units": doc["units"]})


def check_snapshot(snapshot, observations):
    equal(snapshot["kind"], "EconomicSnapshot", "公共快照类型")
    equal(snapshot["completeness"], "CompleteVector", "公共域完整性")
    equal(set(snapshot["domains"]), set(observations), "公共域集合")
    for domain, owner in observations.items():
        got = snapshot["domains"][domain]
        equal(got["status"], "Available", "公共可读 " + domain)
        for field in ("cut", "commit_ns", "image_hash"):
            equal(got[field], owner[field], "公共切面/摘要 " + domain + field)
        equal(canonical(got["image"]), canonical(owner["image"]), "公共原图像字节 " + domain)


def check_watch(watch, before, after):
    equal(watch["kind"], "EconomicWatch", "公共变化类型")
    equal(set(watch["domains"]), set(before), "变化域集合")
    for domain, old in before.items():
        row, current = watch["domains"][domain], after[domain]
        equal(row["status"], "Available", "公共变化可读 " + domain)
        equal(row["base_cut"], old["cut"], "变化起点")
        equal(row["next_cut"], current["cut"], "变化终点")
        delta = int(current["cut"]["commit_seq"]) - int(old["cut"]["commit_seq"])
        require(delta in (0, 1), "本驱动每步最多一个 owner 原子提交")
        expected = [] if delta == 0 else [{k: current[k] for k in ("cut", "commit_ns", "image", "image_hash")}]
        equal(canonical(row["changes"]), canonical(expected), "逐 cut 原子图像/无缺重批")


def project_book(book):
    """仅投影观测字段；经济计算来自独立 oracle。"""
    num = we5_oracle.num
    return {"q": num(book["actual_quantity"]), "K": num(book["cost_basis"]["K"]),
            "P": num(book["principal"]["P"]) if book["principal"]["status"] == "Known" else None,
            "P_recovered": num(book["principal"]["P_recovered"]),
            **{k: num(book["income_book"][k]) for k in ("Pi", "A", "W", "R")},
            "fixed_q": num(book["fixed_q"]), "fees": num(book["realized"]["fees"]),
            "realized_gross": num(book["realized"]["gross"]), "cost_remaining": num(book["cost_remaining"]["value"]),
            "growth_eligible": book["phase"]["growth_eligible"]["status"], "latch": book["phase"]["latch"],
            "gate": None if book["phase"]["gate"] is None else book["phase"]["gate"]["kind"],
            "growth_enabled": book["phase"]["growth_enabled"]["status"],
            "unit_cost": {"K": num(book["unit_cost"]["K"]), "q": num(book["unit_cost"]["q"])},
            "stable_point": book["stable_point"]["seq"]}


class We5Expected:
    def __init__(self):
        self.book, self.facts, self.pending, self.applied, self.resolved = None, {}, {}, {}, []

    def advance(self, step):
        if step["outcome"] not in ("Recorded", "Resolved", "Applied"):
            return
        if step["op"] == "resolve":
            if step["evidence_id"] not in self.resolved:
                self.resolved.append(step["evidence_id"])
                raw = (HERE / "fixtures/tb03c/raw/chong-R1.json").read_text()
                for item in step["covered"]:
                    self.pending.pop(fact_key(event_at(raw, item["pointer"])))
            return
        raw = (HERE / "fixtures/tb03c" / step["raw"]).read_text()
        event = event_at(raw, step["pointer"])
        key = fact_key(event)
        if step["op"] == "capture":
            self.facts[key] = (raw, step["pointer"], event)
            self.pending[key] = {"token": "t-" + event["source"]["external_fact_id"], "scope": step["scope"],
                                 "reason": step["pending_reason"]}
        elif key not in self.applied:
            we5_oracle.verify_fill_predecessors(event, {we5_oracle.fact_identity(e): e for _, _, e in self.applied.values()})
            self.book, outcome = we5_oracle.apply(self.book, decode_frame(raw.encode()), step["pointer"])
            equal(outcome, step["outcome"], "独立书账 outcome")
            self.applied[key] = (raw, step["pointer"], event)

    def check(self, images):
        e, b = images["E"], images["B:chong-R1"]
        for records, expected in ((e["recorded"], self.facts), (b["recorded"], self.applied)):
            equal(len(records), len(expected), "逐事实不重复")
            equal({r["fact_key"] for r in records}, set(expected), "逐事实集合")
            for r in records:
                raw, pointer, event = expected[r["fact_key"]]
                for key, value in {"raw_utf8": raw, "json_pointer": pointer, "event": event,
                                   "fact_hash": fact_hash(raw, pointer), "raw_sha256": hashlib.sha256(raw.encode()).hexdigest()}.items():
                    equal(r[key], value, "原件来源 " + key)
        equal({p["fact_key"]: {"token": p["token"], "scope": p["scope"], "reason": p["reason"]["kind"]}
               for p in e["pending"]}, self.pending, "Pending 原因/范围/原 token")
        equal(len(e["pending"]), len(self.pending), "Pending 不重复")
        equal([h["evidence_id"] for h in e["history"] if h["kind"] == "ResolveEvidence"], self.resolved, "Resolve 原史")
        equal({a["fact_key"]: (a["allocation_id"], a["status"]) for a in b["applied"]},
              {k: (event["allocation_id"], "Applied") for k, (_, _, event) in self.applied.items()}, "B 唯一 Applied")
        equal(len(b["applied"]), len(self.applied), "B 不重复应用")
        equal((e["applied"], b["unmatched"], b["pending"]), ([], [], []), "owner 职责与非降级")
        if self.book is None:
            equal(b["book"], None, "未开局不补钱")
            expected = None
        else:
            expected = self.book.view()
            equal(project_book(b["book"]), expected, "WE5 独立 oracle 全部 view 字段")
            equal(b["book"]["phase"]["events"], self.book.events, "阶段原史")
            equal(b["book"]["principal"]["batches"], [], "无本金批不补零")
            equal(b["book"]["phase"]["growth_enabled"],
                  {"status": "NotEvaluable", "reason": "principal batches absent"}, "增长等待原因")
        return {"book": expected, "pending": self.pending, "resolved": self.resolved,
                "recorded": sorted(self.facts), "applied": sorted(self.applied)}


def parse_http(raw):
    header, separator, body = raw.partition(b"\r\n\r\n")
    require(separator, "HTTP 响应无完整头")
    lines = header.split(b"\r\n")
    require(lines[0].split()[1:2] == [b"200"], "HTTP 非 200 原件已保存")
    fields = [line.split(b":", 1) for line in lines[1:]]
    require(all(len(f) == 2 for f in fields), "HTTP 头格式错误")
    lengths = [v.strip() for k, v in fields if k.lower() == b"content-length"]
    require(len(lengths) == 1 and lengths[0].isdigit() and int(lengths[0]) == len(body), "HTTP 长度缺失/截断")
    require(not any(k.lower() == b"transfer-encoding" for k, _ in fields), "本合同不接受分块或混合传输")
    return decode_frame(body)


class Case:
    def __init__(self, name, output, supervisor, binaries, manifest):
        self.name, self.output, self.supervisor, self.binaries = name, Path(output), supervisor, Path(binaries)
        self.budget = supervisor.budget
        self.output.mkdir()
        self.work = self.output / "state"
        self.work.mkdir()
        self.manifest = copy.deepcopy(manifest)
        self.manifest["session_id"] = "tb1406-c6a-" + name
        self.manifest["e"] = {}
        self.nodes = {"E": self.manifest["e"], **{"B:" + n["chong_id"]: n for n in self.manifest["chongs"]}}
        for index, node in enumerate(self.nodes.values()):
            node.update(db=str(self.work / f"{index}.db"), socket=str(self.work / f"{index}.sock"))
            require(len(node["socket"].encode()) < 100, "socket 路径须少于 100 字节")
        self.manifest["authorized_sources"].append({**QUERY_SOURCE, "operations": ["ReadView", "Watch"]})
        self.epochs, self.processes, self.requests = {}, [], {}
        self.sequence, self.query_sequence, self.port = 0, 0, None
        save(self.work / "manifest.json", self.manifest)

    def args(self, domain, op):
        args = [str(self.binaries / ("e_economic_session" if domain == "E" else "b_chong_session")), op,
                "--manifest", str(self.work / "manifest.json"), "--db", self.nodes[domain]["db"]]
        if domain != "E":
            args += ["--chong", domain[2:]]
        if op == "serve":
            args += ["--epoch", self.epochs[domain]]
            if domain != "E":
                args += ["--e-epoch", self.epochs["E"]]
        return args

    def envelope(self, payload, message, *, query=False):
        if payload["op"] in ("CaptureExternal", "ResolveEvidence", "ApplyAllocation"):
            require(message in self.manifest["clock"]["plan"]["messages"], "写消息须有冻结 TestOnly 时钟")
        return {"schema_revision": "economic-session/1", "session_id": self.manifest["session_id"],
                "session_generation": self.manifest["session_generation"], **(QUERY_SOURCE if query else SOURCE),
                "message_id": message, "payload_hash": digest(payload), "payload": payload, "causal_refs": []}

    def wire(self, address, request, *, http=False, limit=FRAME_BYTES, deadline=None):
        end = min(self.budget.clock() + EXCHANGE_SECONDS, self.budget.case_end, self.budget.work_end,
                  deadline if deadline is not None else self.budget.work_end)
        self.budget.until(end)
        self.sequence += 1
        label = f"{self.sequence:04d}"
        request_path = self.output / (label + (".http-request.bin" if http else ".owner-request.bin"))
        response_path = self.output / (label + (".http-response.bin" if http else ".owner-response.bin"))
        require(len(request) <= FRAME_BYTES, "请求超帧")
        request_path.write_bytes(request)
        response = bytearray()
        def remaining():
            return self.budget.until(end)
        try:
            with response_path.open("xb") as evidence, socket.socket(socket.AF_INET if http else socket.AF_UNIX, socket.SOCK_STREAM) as connection:
                connection.settimeout(remaining())
                connection.connect(address)
                connection.settimeout(remaining())
                connection.sendall(request)
                if not http:
                    connection.shutdown(socket.SHUT_WR)
                while True:
                    connection.settimeout(remaining())
                    part = connection.recv(min(65536, limit + 1 - len(response)))
                    if not part:
                        break
                    evidence.write(part)
                    evidence.flush()
                    response.extend(part)
                    require(len(response) <= limit, "响应超过具名字节界限")
            remaining()
            return bytes(response)
        finally:
            # 即使连接/解码/比较失败也保留完整已收到的原始字节。
            save(self.output / (label + "-transport.json"), {"request": request_path.name, "response": response_path.name,
                 "request_sha256": sha(request_path), "response_sha256": sha(response_path) if response_path.exists() else None,
                 "response_bytes": len(response), "http": http, "address": address})

    def owner(self, domain, payload, message, *, deadline=None):
        request = self.envelope(payload, message, query=payload["op"] in ("ReadView", "Watch"))
        if message in self.requests:
            equal(self.requests[message], request, "精确消息重送")
        self.requests[message] = copy.deepcopy(request)
        raw = self.wire(self.nodes[domain]["socket"], canonical(request) + b"\n", deadline=deadline)
        frame, separator, tail = raw.partition(b"\n")
        require(separator and not tail, "owner 单行原帧不完整或多帧")
        result = validate_reply(request, decode_frame(frame), producer=domain, producer_epoch=self.epochs[domain])
        equal(result.get("domain_id"), domain, "owner 域身份")
        if deadline is not None:
            self.budget.until(deadline)
        return result

    def http(self, path, payload=None, *, deadline=None):
        if payload is None:
            body, request = b"", None
            method = "GET"
        else:
            self.query_sequence += 1
            request = self.envelope(payload, "public-" + str(self.query_sequence), query=True)
            body, method = canonical(request), "POST"
        wire = (f"{method} {path} HTTP/1.0\r\nHost: 127.0.0.1:{self.port}\r\nContent-Type: application/json\r\n"
                f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n").encode() + body
        response = parse_http(self.wire(("127.0.0.1", self.port), wire, http=True,
                                        limit=FRAME_BYTES + 16384, deadline=deadline))
        result = response if request is None else validate_reply(request, response, producer="Q", producer_epoch="1")
        if deadline is not None:
            self.budget.until(deadline)
        return result

    def start(self):
        for domain in self.nodes:
            result = decode_frame(self.supervisor.run(self.args(domain, "init"), self.name + "-init"))
            equal((result["kind"], result["domain_id"]), ("Initialized", domain), "init 原回执")
            self.epochs[domain] = result["writer_epoch"]
            end = min(self.budget.clock() + STARTUP_SECONDS, self.budget.case_end, self.budget.work_end)
            proc = self.supervisor.spawn(self.args(domain, "serve"), self.name + "-serve")
            self.processes.append(proc)
            self.budget.until(end)
            while not Path(self.nodes[domain]["socket"]).is_socket():
                time.sleep(min(.02, self.budget.until(end)))
            equal(self.owner(domain, {"op": "ReadView", "cut": None, "as_known_ns": None},
                             "ready-" + domain, deadline=end)["kind"], "EconomicView", "owner 完整就绪")
            self.budget.until(end)
        config = {"schema_revision": "economic-query/1", "manifest": "manifest.json", "manifest_hash": digest(self.manifest),
                  "query_source": QUERY_SOURCE, "producer_epoch": "1", "expected_epochs": self.epochs,
                  "resources": QUERY_RESOURCES}
        save(self.work / "query.json", config)
        # 正式 CLI 无继承监听 FD；释放至子绑定存在竞态。冲突即失败，不探测/终止占用者。
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as held:
            held.bind(("127.0.0.1", 0))
            self.port = held.getsockname()[1]
        save(self.output / "port.json", {"host": "127.0.0.1", "port": self.port, "released_before_child_bind": True,
                                        "collision_policy": "FAIL without touching occupant"})
        self.structural_db = self.work / "UNUSED-STRUCTURE.db"
        command = [sys.executable, "-B", str(ROOT / "s_readonly_server.py"), "--db", str(self.structural_db),
                   "--browser", str(self.work / "UNUSED-BROWSER.html"), "--host", "127.0.0.1", "--port", str(self.port),
                   "--resource-config", str(HERE / "fixtures/tb01c/query-resources.json"), "--producer-epoch", "1",
                   "--economic-config", str(self.work / "query.json")]
        end = min(self.budget.clock() + STARTUP_SECONDS, self.budget.case_end, self.budget.work_end)
        self.processes.append(self.supervisor.spawn(command, self.name + "-q"))
        while True:
            self.budget.until(end)
            try:
                ready = self.http("/api/economic/ready", deadline=end)
                self.budget.until(end)
            except ConnectionRefusedError:
                time.sleep(min(.02, max(0, end - self.budget.clock())))
                continue
            equal(ready["manifest_hash"], digest(self.manifest), "公共 Q manifest 身份")
            equal(ready["book_revision"], "tb03c-book/1", "公共 Q 合同版本")
            equal(set(ready["domains"]), set(self.nodes), "公共 Q owner 集合")
            equal((ready["session_id"], ready["session_generation"], ready["producer_epoch"]),
                  (self.manifest["session_id"], self.manifest["session_generation"], "1"), "公共 Q 会话/epoch")
            require(not self.structural_db.exists(), "经济路由不得初始化结构数据库")
            self.budget.until(end)
            return

    def observe(self):
        result = {}
        for domain in self.nodes:
            self.query_sequence += 1
            owner = self.owner(domain, {"op": "ReadView", "cut": None, "as_known_ns": None}, "observe-" + str(self.query_sequence))
            equal(owner["kind"], "EconomicView", "完整 owner view")
            equal(owner["image_hash"], digest(owner["image"]), "owner image hash")
            result[domain] = owner
        return result

    def snapshot(self, *, cuts=None, as_known=None, token=None):
        return self.http("/api/economic/snapshot", {"op": "snapshot", "cuts": cuts, "as_known_ns": as_known, "snapshot_token": token})

    def check_public(self, before, after, label):
        current = self.snapshot()
        check_snapshot(current, after)
        fixed = self.snapshot(cuts={d: o["cut"] for d, o in after.items()})
        check_snapshot(fixed, after)
        watch = self.http("/api/economic/watch", {"op": "watch", "cuts": {d: o["cut"] for d, o in before.items()}})
        check_watch(watch, before, after)
        # 每步全域观测在无并发写入下冻结；前一最大 commit_ns 正好覆盖之前向量。
        as_known = self.snapshot(as_known=max((o["commit_ns"] for o in before.values()), key=int))
        check_snapshot(as_known, before)
        old_fixed = self.snapshot(cuts={d: o["cut"] for d, o in before.items()})
        check_snapshot(old_fixed, before)
        equal(self.snapshot(token=old_fixed["snapshot_token"]), old_fixed, "固定 token 原字节")
        save(self.output / (label + "-public.json"), {"current": current, "fixed": fixed, "watch": watch,
                                                    "old_fixed": old_fixed, "as_known": as_known})
        return current

    def finish(self):
        for proc in reversed(self.processes):
            if id(proc) in self.supervisor.children:
                self.supervisor.stop(proc)
                require(proc.returncode in (0, -signal.SIGTERM), "正常轨终结出现非预期退出或 KILL 升级")
        require(not self.structural_db.exists(), "经济路由不得产生结构数据库")


def outcome(reply):
    return reply["error"].split("：", 1)[0] if reply["kind"] == "Error" else reply["kind"]


def we5_payload(step, recorded):
    if step["op"] == "resolve":
        raw = (HERE / "fixtures/tb03c/raw/chong-R1.json").read_text()
        covered = []
        for item in step["covered"]:
            event = event_at(raw, item["pointer"])
            key = fact_key(event)
            covered.append({"fact_key": key, "fact_hash": recorded[key]["fact_hash"] if item["fact_hash"] == "recorded" else "0" * 64,
                            "token": item["token"] or "t-" + event["source"]["external_fact_id"]})
        return "E", {"op": "ResolveEvidence", "evidence_id": step["evidence_id"], "covered": covered,
                     "frontier": {"source_namespace": SOURCE["source_namespace"], "source_epoch": SOURCE["source_epoch"],
                                  "cursor": str(step["frontier_cursor"])}, "dependency_version": "testonly-deps/1"}
    raw = (HERE / "fixtures/tb03c" / step["raw"]).read_text()
    event = event_at(raw, step["pointer"])
    if step["op"] == "capture":
        return "E", {"op": "CaptureExternal", "raw_utf8": raw, "json_pointer": step["pointer"],
                     "token": "t-" + event["source"]["external_fact_id"], "source_event_time": event["source"]["source_event_time"]}
    return "B:chong-R1", {"op": "ApplyAllocation", "fact_key": fact_key(event), "allocation_id": event["allocation_id"],
                         "expected_fact_hash": recorded[fact_key(event)]["fact_hash"]}


def execute_we5(case, plan):
    case.start()
    expected, recorded = We5Expected(), {}
    before = case.observe()
    check_snapshot(case.snapshot(), before)
    for index, step in enumerate(plan["steps"]):
        label = f"step-{index:03d}"
        domain, payload = we5_payload(step, recorded)
        reply = case.owner(domain, payload, label)
        equal(outcome(reply), step["outcome"], "WE5 HAND outcome " + label)
        if step["op"] == "capture" and step["outcome"] == "Recorded":
            event = event_at(payload["raw_utf8"], step["pointer"])
            equal(reply["fact_key"], fact_key(event), "Capture 原事实身份")
            equal(reply["fact_hash"], fact_hash(payload["raw_utf8"], step["pointer"]), "Capture 原件内容")
            recorded[reply["fact_key"]] = reply
        expected.advance(step)
        after = case.observe()
        public = case.check_public(before, after, label)
        oracle = expected.check({d: v["image"] for d, v in public["domains"].items()})
        save(case.output / (label + "-oracle.json"), oracle)
        if step["outcome"] in ("Recorded", "Resolved", "Applied"):
            equal(case.owner(domain, payload, label), reply, "同消息精确重送原回执")
            duplicate = case.observe()
            equal(duplicate, after, "重送不增加 cut/Applied/金额/历史")
            check_snapshot(case.snapshot(), duplicate)
        else:
            equal(after, before, "等待/拒绝不得变化")
        before = after
    equal({k: expected.book.view()[k] for k in ("q", "K", "Pi")}, {"q": 3, "K": 305, "Pi": 19}, "WE5 HAND 终态")
    case.finish()
    return {"case": "WE5", "steps": len(plan["steps"]), "oracle": expected.book.view()}


def we7_manifest(seed, module):
    manifest = copy.deepcopy(seed["manifest"])
    frozen = module.IndependentLedger().manifest
    manifest["chongs"] = [{"chong_id": p["chong_id"], "symbol": frozen["symbol"], "operation_level": str(i + 1),
                           "fixed_q": p["requested_q"]} for i, p in enumerate(frozen["participants"])]
    manifest["fill_manifests"] = [frozen]
    manifest["fill_allocation_policy"] = {"fill_allocation_policy_id": module.POLICY, "policy_revision": "we7-frozen-1"}
    # 仅正常一化身的显式测试时钟；不增加恢复场景、不改经济原件。
    manifest["clock"] = {"kind": "TestOnly", "plan": {"init_ns": "1", "recover_ns": [], "messages": {
        f"we7-{i:03d}": {"received_ns": str(1000 + i * 100), "first_known_ns": str(1001 + i * 100),
                         "commit_ns": str(1002 + i * 100)} for i in range(21)}}}
    return manifest


def we7_row(module, ledger, raw, index):
    """固定 RAW/HAND + 独立 Fraction oracle；不读取被测分配作为预期。"""
    event, report = ledger.raw["events"][index], module.HAND["reports"][index - 3]
    key, manifest = fact_key(event), ledger.manifest
    allocations = []
    for i, participant in enumerate(manifest["participants"]):
        _, cash_and_fee = ledger.allocations[event["source"]["external_fact_id"]][i]
        allocations.append({**{k: participant[k] for k in ("chong_id", "command_id", "family_id", "revision")},
            "allocation_id": "netted:" + digest([key, event["allocation_id"], manifest["manifest_hash"], participant]),
            "units": str(report["quantity"][i]), "commission": str(report["fees"][i]),
            "acquisition_cash": str(cash_and_fee - report["fees"][i])})
    return {"fact_key": key, "fact_hash": fact_hash(raw, f"/events/{index}"),
            "manifest_hash": manifest["manifest_hash"], "symbol": manifest["symbol"],
            "fill_allocation_policy_id": module.POLICY, "policy_revision": "we7-frozen-1",
            "from": str(report["from"]), "to": str(report["to"]),
            "evidence_id": "resolve-" + key, "allocations": allocations}


class We7Expected:
    """只服务固定 WE7 正常轨；完整书账预期不借被测 Q 或 owner 的金额。"""
    def __init__(self, module, ledger, raw, manifest):
        self.module, self.ledger, self.raw = module, ledger, raw
        self.raw_sha = hashlib.sha256(raw.encode()).hexdigest()
        # service::authoritative_fact_inner 从同一 manifest 的 Config 克隆 E；
        # 会话/代次取本案启动输入，不能从正在验的 B 副本反向抄得。
        self.e_source_identity = {"e_domain_id": "E", "e_session_id": manifest["session_id"],
                                  "e_session_generation": manifest["session_generation"]}
        require(all(type(value) is str and value for value in self.e_source_identity.values()),
                "WE7 E 来源身份须为 manifest 的非空字符串")
        self.captures, self.resolutions = {}, {}
        self.applies = {p["chong_id"]: {} for p in ledger.manifest["participants"]}

    def remember(self, domain, payload, reply, before, after):
        op = payload["op"]
        if op == "CaptureExternal":
            index = int(payload["json_pointer"].split("/")[-1])
            self.captures[index] = {"before": copy.deepcopy(before["E"]),
                                    "after": copy.deepcopy(after["E"]), "receipt": copy.deepcopy(reply)}
            equal(reply["cut"], after["E"]["cut"], "WE7 Capture cut 与真实提交对应")
        elif op == "ResolveEvidence":
            index = next(i for i, e in enumerate(self.ledger.raw["events"])
                         if fact_key(e) == payload["covered"][0]["fact_key"])
            self.resolutions[index] = {"after": copy.deepcopy(after["E"]), "receipt": copy.deepcopy(reply)}
            equal(reply["cut"], after["E"]["cut"], "WE7 Resolve cut 与真实提交对应")
        else:
            index = next(i for i, e in enumerate(self.ledger.raw["events"]) if fact_key(e) == payload["fact_key"])
            self.applies[domain[2:]][index] = {"cut": copy.deepcopy(after[domain]["cut"]),
                                             "commit_ns": after[domain]["commit_ns"]}

    def records(self, records, indices, domain):
        expected = {fact_key(self.ledger.raw["events"][i]): i for i in indices}
        equal(len(records), len(expected), "WE7 " + domain + " 原事实精确数量")
        equal({r["fact_key"] for r in records}, set(expected), "WE7 " + domain + " 原事实唯一完整集合")
        for record in records:
            index = expected[record["fact_key"]]
            event = self.ledger.raw["events"][index]
            target = ("B:" + event["book_identity"]["chong_id"] if index < 3
                      else "E:netted:" + self.ledger.manifest["manifest_hash"])
            for field, value in {"raw_utf8": self.raw, "raw_sha256": self.raw_sha,
                                 "json_pointer": f"/events/{index}", "event": event,
                                 "fact_hash": fact_hash(self.raw, f"/events/{index}"), "target_domain": target}.items():
                equal(record[field], value, "WE7 " + domain + " 原来源 " + field)
            if domain != "E":
                for field, value in self.e_source_identity.items():
                    require(field in record, "WE7 " + domain + " 原 E 来源身份缺 " + field)
                    equal(record[field], value, "WE7 " + domain + " 原 E 来源身份 " + field)
                capture = self.captures[index]
                equal(record["cut"], capture["after"]["cut"], "WE7 B 原 capture cut")
                equal(record["e_receipt"], capture["receipt"], "WE7 B 原 E 收据")
                equal(record["e_commit_proof"], {"previous_root": capture["before"]["cut"]["root_hash"],
                      "commit_ns": capture["after"]["commit_ns"], "image": capture["after"]["image"]}, "WE7 B 原 Capture 图像证明")
                if index >= 3:
                    i = next(i for i, p in enumerate(self.ledger.manifest["participants"]) if "B:" + p["chong_id"] == domain)
                    row = we7_row(self.module, self.ledger, self.raw, index)
                    equal(record["resolved_allocation"], row["allocations"][i], "WE7 B 精确逐重分配身份/金额")
                    proof, resolution = record["e_resolution_proof"], self.resolutions[index]
                    equal(proof["image"], resolution["after"]["image"], "WE7 B 冻结 Resolve 来源/政策集合")
                    equal(proof["receipt"], resolution["receipt"], "WE7 B 原 Resolve 收据")

    def book(self, i):
        participant = self.ledger.manifest["participants"][i]
        applied = self.applies[participant["chong_id"]]
        if not applied:
            return None
        opening = self.ledger.raw["events"][i]
        identity, observed = opening["book_identity"], opening["observed_existing_book"]
        opening_key = fact_key(opening)
        fees = sum(self.module.HAND["reports"][n - 3]["fees"][i] for n in applied if n >= 3)
        quantity, cost = self.ledger.q[i], self.ledger.k[i]
        components = [{**c, "fact_key": opening_key, "raw_sha256": self.raw_sha}
                      for c in opening["known_cost_components"]]
        for index in applied:
            if index < 3:
                continue
            row = we7_row(self.module, self.ledger, self.raw, index)
            allocation = row["allocations"][i]
            if int(allocation["units"]) > 0:
                for kind in ("acquisition_cash", "commission"):
                    components.append({"kind": kind, "canonical_amount": allocation[kind],
                        "fact_key": row["fact_key"], "raw_sha256": self.raw_sha,
                        "derivation": {"kind": "E_resolved_netted_allocation",
                            "allocation_id": allocation["allocation_id"], "manifest_hash": row["manifest_hash"],
                            "resolve_cut": self.resolutions[index]["after"]["cut"]}})
        last = self.ledger.raw["events"][next(reversed(applied))]
        return {"book_revision": "tb03c-book/1", "chong_id": participant["chong_id"],
            "symbol": identity["symbol"], "operation_level": identity["op_level"], "fixed_q": identity["fixed_q"],
            "side": identity["side"], "units": self.ledger.raw["units"], "family_id": opening["family_id"],
            "opening_fact_ref": {"fact_key": opening_key, "fact_hash": fact_hash(self.raw, f"/events/{i}"),
                "e_cut": self.captures[i]["after"]["cut"], "json_pointer": f"/events/{i}", "raw_sha256": self.raw_sha},
            "opening_quantity": observed["units"], "actual_quantity": str(quantity),
            "income_book": {"Pi": observed["Pi"], "A": observed["A"], "W": observed["W"],
                            "R": str(int(observed["Pi"]) - int(observed["A"]) - int(observed["W"]))},
            "known_cost_components": components,
            "cost_totals": {"acquisition_cash": str(cost - fees), "commission": str(fees),
                            "disposal_cash": "0", "fee_only": "0", "dividend_cash": "0", "withholding_tax": "0"},
            "realized": {"gross": "0", "fees": str(fees), "after_fees": str(-fees)},
            "principal": {"batches": [], "P_recovered": "0", "status": "NotEvaluable", "reason": "no authorized principal batch"},
            "phase": {"latch": "CostReduction", "events": [], "gate": None,
                      "growth_eligible": {"status": "NotEvaluable"},
                      "growth_enabled": {"status": "NotEvaluable", "reason": "principal batches absent"}},
            "unknown": {"complete_historical_fees": {"status": "Unknown", "reason": "only named known components available"},
                        "target": {"status": "Unknown", "reason": "not supplied"}},
            "voice": {"status": "MissingDependency", "reason": "confirmed gamma binding absent", "identity": None},
            "admission": "NotEvaluated", "cost_remaining": {"status": "Known", "value": str(cost)},
            "cost_basis": {"status": "Known", "K": str(cost)},
            "unit_cost": {"status": "Known", "K": str(cost), "q": str(quantity)},
            "stable_point": {"profile": last["stable_point"]["profile"], "seq": last["stable_point"]["seq"]}}

    def check(self, images):
        e = images["E"]
        self.records(e["recorded"], self.captures, "E")
        rows = [we7_row(self.module, self.ledger, self.raw, i) for i in self.resolutions]
        # 整行精确对拍先于任何遍历：空、重复、缺参与重、错 allocation_id 都不能漏过。
        equal(e.get("fill_allocations", []), rows, "WE7 完整分配/参与重/政策集合")
        pending = []
        for i in self.captures:
            if i in self.resolutions:
                continue
            event = self.ledger.raw["events"][i]
            target = "B:" + event["book_identity"]["chong_id"] if i < 3 else "E:netted:" + self.ledger.manifest["manifest_hash"]
            pending.append({"fact_key": fact_key(event), "token": "t-" + event["source"]["external_fact_id"],
                            "target_domain": target, "scope": target, "status": "Pending", "reason": {"kind": "Unresolved"}})
        equal([{k: p[k] for k in ("fact_key", "token", "target_domain", "scope", "status", "reason")}
               for p in e["pending"]], pending, "WE7 Pending 精确集合")
        equal([(h["evidence_id"], h["covered"]) for h in e["history"] if h["kind"] == "ResolveEvidence"],
              [("resolve-" + r["fact_key"], [r["fact_key"]]) for r in rows], "WE7 Resolve 原史集合")
        equal((e["book"], e["applied"], e["unmatched"]), (None, [], []), "WE7 E 只记录/分配")
        books, expected_applied = {}, {}
        for i, participant in enumerate(self.ledger.manifest["participants"]):
            chong, domain = participant["chong_id"], "B:" + participant["chong_id"]
            image, indices = images[domain], self.applies[chong]
            self.records(image["recorded"], indices, domain)
            applications = []
            for index, observed in indices.items():
                event = self.ledger.raw["events"][index]
                allocation_id = event["allocation_id"] if index < 3 else we7_row(self.module, self.ledger, self.raw, index)["allocations"][i]["allocation_id"]
                applications.append({"fact_key": fact_key(event), "fact_hash": fact_hash(self.raw, f"/events/{index}"),
                    "allocation_id": allocation_id, "target_domain": domain, "status": "Applied",
                    "commit_seq": observed["cut"]["commit_seq"], "commit_ns": observed["commit_ns"]})
            # 普通 netted Applied 当前没有直接政策三元组；不在测试内补造它。
            equal(image["applied"], applications, "WE7 B 完整 Applied 身份/状态/唯一顺序")
            equal((image["pending"], image["unmatched"]), ([], []), "WE7 B 不等待或降级")
            books[chong] = self.book(i)
            equal(image["book"], books[chong], "WE7 RAW/HAND 独立完整账目与来源")
            expected_applied[chong] = applications
        return {"e_source_identity": dict(self.e_source_identity),
                "books": books, "fill_allocations": rows, "recorded_keys": [fact_key(self.ledger.raw["events"][i]) for i in self.captures],
                "pending": pending, "applied": expected_applied, "q": self.ledger.q, "K": self.ledger.k}


def execute_we7(case, module):
    case.start()
    ledger = module.IndependentLedger()
    raw = (HERE / "fixtures/tb03c_allocation/RAW.json").read_text()
    expected = We7Expected(module, ledger, raw, case.manifest)
    message = 0
    before = case.observe()
    check_snapshot(case.snapshot(), before)

    def step(domain, payload, expected_kind, advance):
        nonlocal message, before
        label = f"we7-{message:03d}"
        message += 1
        reply = case.owner(domain, payload, label)
        equal(outcome(reply), expected_kind, "WE7 outcome")
        advance()
        after = case.observe()
        public = case.check_public(before, after, label)
        images = {d: v["image"] for d, v in public["domains"].items()}
        expected.remember(domain, payload, reply, before, after)
        oracle_result = expected.check(images)
        save(case.output / (label + "-oracle.json"), oracle_result)
        equal(case.owner(domain, payload, label), reply, "WE7 精确重送原回执")
        equal(case.observe(), after, "WE7 重送无新 cut/金额/Applied")
        before = after
        return reply

    for index, event in enumerate(ledger.raw["events"]):
        key = fact_key(event)
        pointer = f"/events/{index}"
        def capture(index=index):
            if index >= 3:
                ledger.capture(index)
        reply = step("E", {"op": "CaptureExternal", "raw_utf8": raw, "json_pointer": pointer,
                           "token": "t-" + event["source"]["external_fact_id"], "source_event_time": event["source"]["source_event_time"]}, "Recorded", capture)
        equal((reply["fact_key"], reply["fact_hash"]), (key, fact_hash(raw, pointer)), "WE7 Capture 身份")
        if index < 3:
            chong = event["book_identity"]["chong_id"]
            step("B:" + chong, {"op": "ApplyAllocation", "fact_key": key, "allocation_id": event["allocation_id"],
                                "expected_fact_hash": fact_hash(raw, pointer)}, "Applied", lambda: None)
            continue
        external = event["source"]["external_fact_id"]
        def resolve(external=external):
            ledger.resolve(external)
        step("E", {"op": "ResolveEvidence", "evidence_id": "resolve-" + key,
                   "covered": [{"fact_key": key, "fact_hash": fact_hash(raw, pointer), "token": "t-" + external}],
                   "frontier": {"source_namespace": SOURCE["source_namespace"], "source_epoch": SOURCE["source_epoch"],
                                "cursor": event["source"]["source_cursor"]}, "dependency_version": "testonly-deps/1"}, "Resolved", resolve)
        for i, participant in enumerate(ledger.manifest["participants"]):
            allocation_id = "netted:" + digest([key, event["allocation_id"], ledger.manifest["manifest_hash"], participant])
            chong = participant["chong_id"]
            def apply(i=i, external=external):
                ledger.apply(external, i)
            step("B:" + chong, {"op": "ApplyAllocation", "fact_key": key, "allocation_id": allocation_id,
                                "expected_fact_hash": fact_hash(raw, pointer)}, "Applied", apply)
    equal(message, 21, "WE7 显式时钟覆盖")
    equal(ledger.q, module.HAND["reports"][-1]["book_quantity"], "WE7 HAND 终态数量")
    equal(ledger.k, module.HAND["reports"][-1]["K"], "WE7 HAND 终态账目")
    case.finish()
    return {"case": "WE7", "steps": message, "q": ledger.q, "K": ledger.k}


def verify_unchanged(budget, source_hashes):
    for base, expected, detail in ((REPO, source_hashes, "运行期间源码改变"),
                                   (HERE, INPUT_HASHES, "运行期间原件改变")):
        for name, wanted in expected.items():
            budget.check()
            actual = sha(base / name)
            budget.check()  # 源 hash 中收到取消或超过 t0+600 不能落入候选封口。
            equal(actual, wanted, detail + ": " + name)


def finalize(output, budget, supervisor, results, primary, cleanup):
    budget.check(final=True)
    files = []
    for path in sorted(Path(output).rglob("*")):
        budget.check(final=True)
        if path.is_file():
            files.append({"path": str(path.relative_to(output)), "bytes": path.stat().st_size, "sha256": sha(path)})
    save(Path(output) / "ARTIFACTS.json", {"files": files})
    manifest_hash = sha(Path(output) / "ARTIFACTS.json")
    budget.check(final=True)
    ok = (primary is None and cleanup is None and not supervisor.children
          and len(results) == 2 and budget.cancel is None)
    receipt = {"status": "CANDIDATE_C6A_NORMAL_ONLY" if ok else "FAIL", "scope": "TestOnly WE5/WE7 public Q normal",
               "results": results, "primary_error": primary, "cleanup_error": cleanup,
               "cancellation": budget.cancel, "candidate_only": True,
               "outer_acceptance_requires": ["exit_zero", "no_cancellation", "no_failure_markers",
                                              "complete_verified_originals", "group_reaped_with_owned_identity"],
               "remaining_children": [c["proc"].pid for c in supervisor.children.values()],
               "work_seconds": WORK_SECONDS, "final_seconds": FINAL_SECONDS, "elapsed": budget.clock() - budget.t0,
               "artifact_manifest_sha256": manifest_hash,
               "not_proven": ["complete C6", "FIN/2", "real S/gamma", "per_leg success", "C7 recovery", "frontend", "1394 readiness"]}
    save(Path(output) / "RESULT.json", receipt)
    budget.sealed_output = Path(output)
    # 写收据期间的取消不能被候选状态遮住；封口后 handler 继续有效直到进程退出。
    if budget.cancel is not None:
        save(Path(output) / "FINALIZATION-REJECTED.json",
             {"status": "FAIL", "reason": "cancelled", "cancellation": int(budget.cancel),
              "candidate_invalid": True})
        return 1
    if budget.clock() >= budget.final_end:
        save(Path(output) / "FINALIZATION-REJECTED.json",
             {"status": "FAIL", "reason": "final receipt exceeded fixed deadline", "candidate_invalid": True})
        raise TimeoutError("晚到候选拒绝")
    return 0 if ok else 1


def main():
    t0 = time.monotonic()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binaries", required=True, type=Path)
    parser.add_argument("--build-evidence", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=False, exist_ok=False)
    budget = Budget(t0)
    supervisor = Supervisor(output, budget)
    handlers = {}
    results, primary, cleanup = [], None, None
    try:
        require(threading.current_thread() is threading.main_thread(), "main 必须主线程")
        require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "SIGCHLD 必须默认")
        require(os.getpid() == os.getpgrp() == os.getsid(0), "须由外层创建独立 driver session/group")
        for sig in (signal.SIGTERM, signal.SIGINT):
            handlers[sig] = signal.signal(sig, budget.receive_cancel)
        seed, plan, oracle = load_inputs()
        budget.check()
        checkout = supervisor.run(["git", "rev-parse", "HEAD"], "source-head").decode().strip()
        tree = supervisor.run(["git", "rev-parse", "HEAD^{tree}"], "source-tree").decode().strip()
        equal(supervisor.run(["git", "status", "--porcelain", "--untracked-files=all"], "source-clean"), b"", "精确远端源码须 clean")
        hashes = {name: sha(REPO / name) for name in SOURCE_FILES}
        binaries = {name: sha(args.binaries / name) for name in ("e_economic_session", "b_chong_session")}
        build = decode_frame(args.build_evidence.read_bytes())
        verify_build(build, checkout, tree, hashes, binaries)
        save(output / "SOURCE.json", {"base": BASE, "base_tree": BASE_TREE, "build": build, "inputs": INPUT_HASHES,
             "q_resource_sha256": sha(HERE / "fixtures/tb01c/query-resources.json"), "limits": {
                 "work": WORK_SECONDS, "per_case": CASE_SECONDS, "final": FINAL_SECONDS,
                 "startup": STARTUP_SECONDS, "exchange": EXCHANGE_SECONDS, "cli": PROCESS_SECONDS,
                 "term_grace": TERM_SECONDS, "kill_grace": KILL_SECONDS}, "hard_io_deadline": False})
        for name in ("we5", "we7"):
            budget.begin_case()
            manifest = seed["manifest"] if name == "we5" else we7_manifest(seed, oracle)
            case = Case(name, output / name, supervisor, args.binaries.resolve(), manifest)
            results.append(execute_we5(case, plan) if name == "we5" else execute_we7(case, oracle))
            budget.check()
        # Q/resources/fixtures 不得在运行中换源；失败原件不删。
        verify_unchanged(budget, hashes)
    except BaseException as error:
        primary = {"type": type(error).__name__, "detail": str(error)}
    finally:
        try:
            supervisor.close()
        except BaseException as error:
            cleanup = {"type": type(error).__name__, "detail": str(error)}
        # 不在封口前恢复 handler；封口后的取消须独立落失败原件并非零退出。
        supervisor.log.close()
    try:
        return finalize(output, budget, supervisor, results, primary, cleanup)
    except BaseException as error:
        # 超期限不再重试归档；只留下小型明确失败标记，绝不补写 PASS。
        path = output / "FINALIZATION-FAILED.json"
        if not path.exists():
            save(path, {"status": "FAIL", "error": repr(error), "primary_error": primary, "cleanup_error": cleanup})
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
