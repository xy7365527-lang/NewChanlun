#!/usr/bin/env python3
"""#1394 当前协议静态 driver 适配层。CLI 仅检查，不提供产品执行入口。"""
from __future__ import annotations
import argparse
from contextlib import closing
from dataclasses import dataclass
import math
from typing import TypedDict, TypeAlias
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import socket
import sqlite3
import stat
import subprocess
import sys
import time
import uuid

STRUCTURE = {"binary", "binary_sha256", "db", "session_id",
             "session_generation", "writer_epoch", "reader_revision",
             "content_format", "reader_resources", "reader_session"}
SESSION = {"schema_revision", "startup_timeout_ms", "retained_bytes",
           "max_requests", "max_request_bytes", "max_reply_bytes",
           "request_timeout_ms", "max_capture_bytes", "max_closure_nodes"}
READER = {"schema_revision", "max_request_bytes", "max_reply_bytes",
          "max_stderr_bytes", "request_timeout_ms", "job_limits"}
JOB = {"max_total_wire_bytes", "max_total_node_bytes", "max_total_nodes",
       "max_total_records", "max_total_requests", "max_elapsed_ms",
       "max_staged_bytes", "max_store_bytes"}
MARKER = {"domain_id", "message_id", "phase", "pid", "test_only"}
FAULTS = (("PrepareDecision", "B:alpha"), ("Reserve", "E"),
          ("CommitDecision", "B:alpha"), ("ImportResponsibility", "E"),
          ("AbortDecision", "B:alpha"), ("CloseApplicants", "E"))


JSON: TypeAlias = dict[str, "JSON"] | list["JSON"] | str | int | float | bool | None


class Rejected(ValueError):
    """失败保留原件；调用者不得以默认值或合成结果补齐。"""


def require(condition: object, message: str) -> None:
    if not condition:
        raise Rejected(message)


def canonical(value: JSON) -> bytes:
    return json.dumps(value, sort_keys=True, ensure_ascii=False,
                      separators=(",", ":"), allow_nan=False).encode()


def sha(raw: bytes | bytearray) -> str:
    return hashlib.sha256(raw).hexdigest()


def strict(raw: bytes | bytearray | str) -> JSON:
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "重复 JSON 键")
            result[key] = value
        return result
    def constant(value):
        raise Rejected("非有限 JSON 值：" + value)
    def finite(value: str) -> float:
        number = float(value)
        require(math.isfinite(number), "非有限 JSON 浮点")
        return number
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant, parse_float=finite)


def exact(value: JSON, keys: set[str], label: str) -> None:
    require(type(value) is dict and set(value) == keys, label + " exact 字段不符")


def positive(value: JSON) -> int:
    require(type(value) is str and re.fullmatch(r"[1-9][0-9]*", value),
            "必须为规范正整数文本，不能用 null 或默认值")
    number = int(value)
    require(number <= 2**63 - 1, "数值超界")
    return number


def pinned(item: dict[str, str]) -> bytes:
    exact(item, {"path", "sha256"}, "原件指针")
    path = Path(item["path"])
    require(path.is_absolute() and path.is_file() and not path.is_symlink(),
            "原件必须是绝对普通文件")
    raw = path.read_bytes()
    require(sha(raw) == item["sha256"], "原件 SHA 不符：" + str(path))
    return raw


def save_new(path: str | Path, value: JSON, *, deadline: float | None = None,
             directory: OwnedDirectory | None = None) -> None:
    if deadline is not None:
        remaining(deadline)
    with (directory.open_new(Path(path).name) if directory else Path(path).open("xb")) as stream:
        stream.write(canonical(value) + b"\n")
        stream.flush()
        os.fsync(stream.fileno())
    if deadline is not None:
        remaining(deadline)
    if directory:
        directory.verify()


class OwnedDirectory:
    """只在本实例新建且路径身份未替换的目录写原件；fd 防止路径切换后误写。"""
    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        require(self.path.is_absolute(), "原件目录必须绝对路径")
        self.fd = None
        self.parent_fd = os.open(self.path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            os.mkdir(self.path.name, mode=0o700, dir_fd=self.parent_fd)
            created = os.stat(self.path.name, dir_fd=self.parent_fd, follow_symlinks=False)
            self.fd = os.open(self.path.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                              dir_fd=self.parent_fd)
            self.identity = (created.st_dev, created.st_ino)
            self.verify()
        except BaseException:
            self.close()
            raise

    def verify(self) -> None:
        require(self.fd is not None and self.parent_fd is not None, "原件目录句柄已关闭")
        observed = self.path.lstat()
        held = os.fstat(self.fd)
        parent = self.path.parent.stat()
        parent_held = os.fstat(self.parent_fd)
        require(stat.S_ISDIR(observed.st_mode) and
                (observed.st_dev, observed.st_ino) == self.identity == (held.st_dev, held.st_ino)
                and (parent.st_dev, parent.st_ino) == (parent_held.st_dev, parent_held.st_ino),
                "原件目录已被替换")

    def open_new(self, name: str):
        require(name not in {"", ".", ".."} and Path(name).name == name, "原件名称无效")
        self.verify()
        fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                     0o600, dir_fd=self.fd)
        return os.fdopen(fd, "wb")

    def close(self) -> None:
        for field in ("fd", "parent_fd"):
            fd = getattr(self, field, None)
            if fd is not None:
                os.close(fd)
                setattr(self, field, None)


def validate_resources(session: dict[str, JSON], reader: dict[str, JSON]) -> None:
    exact(session, SESSION, "session resources/3")
    exact(reader, READER, "reader resources/1")
    exact(reader["job_limits"], JOB, "reader Job")
    require(session["schema_revision"] == "s-financial-session-resources/3",
            "会话资源版本不符")
    require(reader["schema_revision"] == "s-financial-reader-resources/1",
            "reader 资源版本不符")
    for key in SESSION - {"schema_revision"}:
        positive(session[key])
    for key in READER - {"schema_revision", "job_limits"}:
        positive(reader[key])
    for value in reader["job_limits"].values():
        positive(value)
    require(positive(session["retained_bytes"]) <= 268435456,
            "金融 retained_bytes 超过 A6 256 MiB 固定上限")
    require(positive(session["max_reply_bytes"]) >= positive(reader["max_reply_bytes"]),
            "会话 reply 帽小于冻结 reader 帽")


def validate_manifest(manifest: dict[str, JSON], session_resources: dict[str, JSON], *, check_files: bool = True) -> dict[str, str]:
    require(manifest["schema_revision"] == "economic-manifest/1", "外层 manifest 版本")
    require(manifest.get("protocol_revision") == "economic-session/2", "必须显式经济域 /2")
    require(manifest["clock"]["kind"] == "TestOnly", "仅具名 TestOnly 域")
    require(bool(manifest.get("voice_binding")), "缺 Voice 权威绑定")
    require({b["chong_id"] for b in manifest["chongs"]} == {"alpha", "beta"}
            and len(manifest["chongs"]) == 2, "须 alpha/beta 两个 owner")
    finance = manifest["finance"]
    exact(finance, {"schema_revision", "input_path", "input_sha256",
                    "coordinator_source", "structure"}, "finance")
    require(finance["schema_revision"] == "tb03b-finance/2", "必须 finance/2")
    structure = finance["structure"]
    exact(structure, STRUCTURE, "structure")
    require(structure["reader_revision"] == "s-financial-basis/3"
            and structure["content_format"] == "s-content/1", "reader 版本")
    require(structure["session_id"] == manifest["session_id"]
            and structure["session_generation"] == manifest["session_generation"],
            "结构与经济会话身份不同")
    positive(structure["writer_epoch"])
    require(type(structure["reader_session"]) is str, "SessionClosure 不接受 null 或冷读法")
    reader = Path(structure["reader_session"])
    require(reader.is_absolute() and len(os.fsencode(reader)) <= 103,
            "SessionClosure 必须是已冻绝对 Unix socket，长度 <=103")
    if check_files:
        parent = reader.parent.stat()
        require(parent.st_uid == os.geteuid() and stat.S_ISDIR(parent.st_mode)
                and stat.S_IMODE(parent.st_mode) & 0o077 == 0,
                "SessionClosure 父目录必须本用户私有")
        pinned({"path": structure["binary"], "sha256": structure["binary_sha256"]})
        raw = pinned({"path": finance["input_path"], "sha256": finance["input_sha256"]})
        require(strict(raw)["schema_revision"] == "tb03b-finance/2", "资金输入版本")
    validate_resources(session_resources, structure["reader_resources"])
    for key in ("binary", "db"):
        require(Path(structure[key]).is_absolute(), key + " 必须绝对路径")
    return {"reading": "SessionClosure", "finance": "tb03b-finance/2"}


def economic_envelope(manifest: dict[str, JSON], source: dict[str, str], message_id: str, payload: JSON, causal_refs: list[JSON]) -> dict[str, JSON]:
    require(manifest.get("protocol_revision") == "economic-session/2", "拒绝混合经济域")
    exact(source, {"source_namespace", "source_epoch", "producer_id", "producer_epoch"},
          "请求 source")
    require(type(message_id) is str and bool(message_id), "缺原业务身份")
    return {"schema_revision": "economic-session/2", "session_id": manifest["session_id"],
            "session_generation": manifest["session_generation"], **source,
            "message_id": message_id, "payload_hash": sha(canonical(payload)),
            "causal_refs": causal_refs, "payload": payload}


def exchange_frame(endpoint: str, original_request: bytes, timeout_ms: int, max_frame_bytes: int, absolute_deadline: float) -> dict[str, JSON]:
    """单次传输的所有步骤共用最早截止；失败只记 DeliveryUnknown 原因。"""
    request = strict(original_request)
    require(type(request) is dict, "请求原件必须对象")
    wire = original_request if original_request.endswith(b"\n") else original_request + b"\n"
    require(len(wire) <= max_frame_bytes, "E/B 请求超独立 frame 帽")
    deadline = min(absolute_deadline, time.monotonic() + timeout_ms / 1000)
    response, sent = bytearray(), 0
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as channel:
            def remaining():
                value = deadline - time.monotonic()
                if value <= 0:
                    raise TimeoutError("同一次传输绝对截止已到")
                return value
            channel.settimeout(remaining())
            channel.connect(endpoint)
            while sent < len(wire):
                channel.settimeout(remaining())
                count = channel.send(wire[sent:])
                if count == 0:
                    raise ConnectionError("发送期间断开")
                sent += count
            while not response.endswith(b"\n"):
                channel.settimeout(remaining())
                chunk = channel.recv(min(4096, max_frame_bytes + 1 - len(response)))
                if not chunk:
                    raise ConnectionError("回复未封帧便断开")
                response.extend(chunk)
                require(len(response) <= max_frame_bytes, "E/B 回复超独立 frame 帽")
        decoded = strict(response)
        require(time.monotonic() < deadline, "回复解析后固定截止已到")
        return {"transport": "Received", "request_sha256": sha(original_request),
                "sent_bytes": sent, "response_raw_hex": response.hex(), "response": decoded}
    except (OSError, Rejected, ValueError) as error:
        return {"transport": "DeliveryUnknown", "request_sha256": sha(original_request),
                "sent_bytes": sent, "response_raw_hex": response.hex(),
                "error_type": type(error).__name__, "error": str(error)}


def ab_directories(root: str | Path) -> dict[str, Path]:
    root = Path(root)
    require(root.is_absolute() and not root.exists(), "A/B 根必须是新绝对路径")
    root.mkdir(mode=0o700)
    for name in ("A", "B"):
        (root / name).mkdir(mode=0o700)
    return {name: root / name for name in ("A", "B")}


def financial_serve_argv(structure: dict[str, JSON], resources_path: str | Path, custody_path: str | Path, receipts_path: str | Path) -> list[str]:
    """只构造当前六参数入口；调用方仍需监督 ACK 和精确运行准入。"""
    require(structure["reader_session"] is not None, "禁止新域冷回落")
    return [structure["binary"], "financial-serve", "--protocol-revision",
            "s-session/3", "--db", structure["db"], "--socket",
            structure["reader_session"], "--resources", str(resources_path),
            "--custody", str(custody_path), "--receipts", str(receipts_path)]


def verify_ready(raw_line: bytes, startup_path: str | Path, structure: dict[str, JSON], resources: dict[str, JSON], owner_pid: int, expected_anchor: JSON, *, budget: ReadBudget) -> dict[str, JSON]:
    budget.check()
    require(raw_line.startswith(b"READY ") and raw_line.endswith(b"\n"),
            "未取得真实 READY 原行")
    ready = strict(raw_line[6:])
    exact(ready, {"schema", "startup_receipt_sha256", "startup_receipt_path",
                  "startup_receipt"}, "READY")
    require(ready["schema"] == "s-financial-session-ready/1", "READY 版本")
    raw = read_original(startup_path, budget)
    receipt = strict(raw)
    budget.check()
    require(ready["startup_receipt_path"] == str(startup_path)
            and ready["startup_receipt"] == receipt
            and ready["startup_receipt_sha256"] == sha(raw.rstrip(b"\n"))
            and sha(canonical(receipt)) == ready["startup_receipt_sha256"],
            "READY 与 STARTUP 原件/SHA 不同")
    require(receipt["schema"] == "s-financial-session-startup/2"
            and receipt["status"] == "READY", "启动未真实就绪")
    for key, wanted in {"binary_sha256": structure["binary_sha256"],
                        "pid": str(owner_pid), "db": structure["db"],
                        "consistency": "a", "anchor_mismatch_policy": "reject-request",
                        "resources": resources, "resources_sha256": sha(canonical(resources)),
                        "anchor": expected_anchor}.items():
        require(receipt[key] == wanted, "READY 绑定不符：" + key)
    versions = receipt["data_version"]
    require(type(versions) is list and len(versions) == 3
            and len(set(versions)) == 1, "启动 capture data_version 三点不同")
    budget.check()
    return receipt


def file_stat(path: str | Path) -> dict[str, str]:
    info = Path(path).stat()
    require(stat.S_ISREG(info.st_mode), "DB 原件不是普通文件")
    return {key: str(getattr(info, key)) for key in
            ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")}


def custody(db_path: str | Path, *, budget: ReadBudget) -> dict[str, JSON]:
    """按 custody/2 观察主库与 WAL；只有 writer 已 wait/reap 后调用。"""
    budget.check()
    db = Path(db_path)
    require(db.is_absolute(), "DB 须绝对路径")
    result = {"schema_revision": "s-financial-session-custody/2", "db": str(db)}
    for label, suffix in (("main", ""), ("wal", "-wal"), ("shm", "-shm")):
        path = Path(str(db) + suffix)
        if not path.exists():
            require(label != "main", "缺真实主库")
            result[label] = None
            continue
        require(not path.is_symlink(), "不接受 DB/WAL/shm symlink")
        before = file_stat(path)
        if label == "shm":
            result[label] = before
        else:
            digest = hash_original(path, budget)
            require(file_stat(path) == before, "DB/WAL 在托管期间变化")
            result[label] = {**before, "sha256": digest}
    budget.check()
    return result


def read_domain(db_path: str | Path, *, budget: ReadBudget | None = None) -> dict[str, JSON]:
    """显式只读/关闭；每行跨库累计计费，共用固定 Job 截止。"""
    path = Path(db_path)
    require(path.is_file() and not path.is_symlink(), "缺真实 DB")
    require(type(budget) is ReadBudget, "真实 DB 必须绑定共享累计读取预算")
    timeout = budget.check()
    with closing(sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True,
                                timeout=timeout)) as db:
        try:
            db.set_progress_handler(lambda: int(time.monotonic() >= budget.deadline), 100)
            db.execute("PRAGMA busy_timeout=" + str(max(1, int(budget.check() * 1000))))
            db.execute("PRAGMA query_only=ON")
            db.execute("BEGIN")
            def rows(sql: str):
                budget.check()
                # SQLite busy wait uses at most remaining Job time for this query.
                db.execute("PRAGMA busy_timeout=" + str(max(1, int(budget.check() * 1000))))
                db.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, max(1, budget.max_bytes - budget.bytes_read))
                cursor = db.execute(sql)
                try:
                    while True:
                        budget.check()
                        db.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, max(1, budget.max_bytes - budget.bytes_read))
                        row = cursor.fetchone()
                        if row is None:
                            break
                        budget.charge(row)
                        yield row
                finally:
                    cursor.close()
                budget.check()
            meta = dict(rows("SELECT k,v FROM meta"))
            commits = [{"seq": seq, "root": root, "commit_ns": ns, "image": strict(image)}
                       for seq, root, ns, image in rows(
                           "SELECT seq,root,commit_ns,image FROM commits ORDER BY seq")]
            receipts = {key: strict(raw) for key, raw in rows(
                "SELECT k,json FROM receipts ORDER BY k")}
            requests = {key: strict(raw) for key, raw in rows(
                "SELECT k,request_json FROM messages ORDER BY k")}
            budget.check()
        finally:
            db.set_progress_handler(None, 0)
            db.rollback()
    budget.check()
    require(bool(commits), "真实 DB 没有提交")
    return {"meta": meta, "commits": commits, "receipts": receipts, "requests": requests}


def recovery_comparison(before: dict[str, JSON], after: dict[str, JSON], old_epoch: str, new_epoch: str) -> dict[str, JSON]:
    require(positive(new_epoch) == positive(old_epoch) + 1, "恢复 epoch 须递增一次")
    require(after["meta"]["writer_epoch"] == new_epoch, "持久 epoch 不符")
    for key in ("commits", "receipts", "requests"):
        require(before[key] == after[key], "恢复改写业务原件：" + key)
    return {"new_epoch": new_epoch, "persistent_history_unchanged": True}


def compare_ab(actual_a: dict[str, JSON], actual_b: dict[str, JSON]) -> dict[str, JSON]:
    """两个 actual 必须来自各自只读原库；从不把 oracle 赋给 actual。"""
    for domain in ("E", "B:alpha", "B:beta"):
        for key in ("commits", "receipts", "requests"):
            require(actual_a[domain][key] == actual_b[domain][key],
                    "A/B 原件不确定：" + domain + "/" + key)
    return {"domain_hashes": {d: sha(canonical(actual_a[d])) for d in actual_a}}


def remaining(deadline: float) -> float:
    """检查同一个绝对单调时钟截止；不得重新起算。"""
    value = deadline - time.monotonic()
    require(math.isfinite(value) and value > 0, "同一固定截止已到")
    return value


@dataclass
class ReadBudget:
    deadline: float
    max_records: int
    max_bytes: int
    records: int = 0
    bytes_read: int = 0

    def check(self) -> float:
        require(type(self.max_records) is int and self.max_records > 0
                and type(self.max_bytes) is int and self.max_bytes > 0,
                "累计原件读取帽必须显式为正")
        require(self.records <= self.max_records and self.bytes_read <= self.max_bytes,
                "累计原件读取帽已超")
        return remaining(self.deadline)

    def charge(self, row: tuple[object, ...]) -> None:
        self.check()
        size = sum(len(value if isinstance(value, bytes) else str(value).encode())
                   for value in row if value is not None)
        # 计费后即使解析失败也不退还，跨库复用同一对象。
        self.records += 1
        self.bytes_read += size
        self.check()


class ProcessIdentity(TypedDict):
    pid: int
    start_identity: str
    process_group: int


class CleanupReceipt(TypedDict):
    owner: dict[str, JSON]
    returncode: int | None
    reaped: bool
    errors: list[str]
    reason: str
    binary_sha256_before: str
    binary_sha256_after: str | None


def read_original(path: str | Path, budget: ReadBudget) -> bytes:
    budget.check()
    result = bytearray()
    with Path(path).open("rb") as stream:
        while True:
            budget.check()
            available = budget.max_bytes - budget.bytes_read
            part = stream.read(min(65536, available + 1))
            budget.bytes_read += len(part)
            budget.check()
            if not part:
                return bytes(result)
            result.extend(part)


def hash_original(path: str | Path, budget: ReadBudget) -> str:
    budget.check()
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        while True:
            budget.check()
            part = stream.read(min(65536, budget.max_bytes - budget.bytes_read + 1))
            budget.bytes_read += len(part)
            budget.check()
            if not part:
                return digest.hexdigest()
            digest.update(part)


def bounded_hash(path: str | Path, deadline: float) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        while True:
            remaining(deadline)
            part = stream.read(65536)
            remaining(deadline)
            if not part:
                return digest.hexdigest()
            digest.update(part)


def process_identity(process: subprocess.Popen[bytes], deadline: float) -> ProcessIdentity:
    remaining(deadline)
    require(process.poll() is None, "owner 已退出")
    result = subprocess.run(["/bin/ps", "-p", str(process.pid), "-o", "lstart="],
                            capture_output=True, text=True, check=True,
                            timeout=remaining(deadline))
    remaining(deadline)
    start = result.stdout.strip()
    require(bool(start), "缺进程 start identity")
    group = os.getpgid(process.pid)
    remaining(deadline)
    require(process.poll() is None, "owner 已退出")
    return {"pid": process.pid, "start_identity": start, "process_group": group}


class Supervisor:
    """ACK 只是一段控制协议，不代替外部监督资源证据。"""
    def __init__(self, endpoint: str, contract_sha256: str, root_deadline: float,
                 max_frame_bytes: int) -> None:
        self.endpoint, self.contract = endpoint, contract_sha256
        self.deadline, self.frame = root_deadline, max_frame_bytes

    def ack(self, stage: str, owner: dict[str, JSON], caps: dict[str, JSON],
            deadline: float) -> dict[str, JSON]:
        limit = min(self.deadline, deadline)
        worker_start = subprocess.run(["/bin/ps", "-p", str(os.getpid()), "-o", "lstart="],
                                      capture_output=True, text=True, check=True,
                                      timeout=remaining(limit)).stdout.strip()
        remaining(limit)
        require(bool(worker_start), "worker start identity 缺失")
        binding = {"contract_sha256": self.contract, "nonce": uuid.uuid4().hex,
                   "worker_pid": os.getpid(), "worker_start_identity": worker_start,
                   "worker_process_group": os.getpgrp(), "stage": stage, "owner": owner,
                   "absolute_deadline_monotonic": limit, "resource_caps": caps}
        payload = canonical({"kind": "ACK_REQUEST", "binding": binding}) + b"\n"
        require(len(payload) <= self.frame, "监督请求超帧")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as channel:
            channel.settimeout(remaining(limit))
            channel.connect(self.endpoint)
            channel.settimeout(remaining(limit))
            channel.sendall(payload)
            remaining(limit)
            raw = bytearray()
            while not raw.endswith(b"\n"):
                channel.settimeout(remaining(limit))
                chunk = channel.recv(min(4096, self.frame + 1 - len(raw)))
                remaining(limit)
                require(bool(chunk), "监督 ACK 失联")
                raw.extend(chunk)
                require(len(raw) <= self.frame, "监督回复超帧")
        reply = strict(raw)
        require(reply == {"kind": "ACK", "binding": binding}, "监督 ACK 身份/帽/截止不符")
        remaining(limit)
        return reply


class ExecutionAdmission:
    """候选接线合同，尚无已准入实例。只能消费根另行冻结的精确合同/准入原件。"""
    def __init__(self, contract_pointer: dict[str, str], grant_pointer: dict[str, str]) -> None:
        self.t0 = time.monotonic()
        self.active_owners: dict[tuple[str, str, str], OwnedProcess] = {}
        self.consumed: set[tuple[str, str, str]] = set()
        raw = pinned(contract_pointer)
        contract, grant = strict(raw), strict(pinned(grant_pointer))
        require(contract.get("schema_revision") == "issue1394-driver-execution/1",
                "缺独立审过的 driver 运行合同")
        require(contract.get("execution_authorized") is True
                and grant.get("execution_authorized") is True, "运行未获精确准入")
        require(grant["contract_sha256"] == sha(raw)
                and grant["driver_sha256"] == sha(Path(__file__).read_bytes()),
                "准入未绑定本 driver/合同字节")
        require(bool(grant["nonce"]), "准入 nonce 缺失")
        self.contract, self.contract_sha = contract, sha(raw)
        require(Path(contract["cwd"]).is_absolute() and Path(contract["cwd"]).is_dir(),
                "运行 cwd 未冻结")
        require(type(contract["environment"]) is dict, "运行环境须显式冻结")
        require(bool(contract["source_roles"]) and bool(contract["input_originals"])
                and bool(contract["artifacts"]), "来源、输入或产物未冻结")
        total = positive(contract["total_timeout_ms"]) / 1000
        cleanup = positive(contract["cleanup_reserve_ms"]) / 1000
        seal = positive(contract["seal_reserve_ms"]) / 1000
        self.term_grace_seconds = positive(contract["term_grace_ms"]) / 1000
        self.kill_wait_seconds = positive(contract["kill_wait_ms"]) / 1000
        require(cleanup >= self.term_grace_seconds + self.kill_wait_seconds
                and total > cleanup + seal, "缺固定回收/封口预留窗")
        self.business_deadline = self.t0 + total - cleanup - seal
        self.cleanup_deadline = self.t0 + total - seal
        self.seal_deadline = self.t0 + total
        remaining(self.business_deadline)
        for item in [contract["source_roles"], *contract["input_originals"], *contract["artifacts"]]:
            exact(item, {"path", "sha256"}, "原件指针")
            path = Path(item["path"])
            require(path.is_absolute() and path.is_file() and not path.is_symlink(), "原件须绝对普通文件")
            require(bounded_hash(item["path"], self.business_deadline) == item["sha256"], "原件 SHA 不符")
        # x 创建构成单次消费凭证；不得删凭证后复用同一准入。
        save_new(contract["single_use_receipt"],
                 {"nonce": grant["nonce"], "contract_sha256": self.contract_sha,
                  "worker_pid": os.getpid(), "t0_monotonic": self.t0},
                 deadline=self.business_deadline)

    def check_command(self, argv: list[str], stage: str, caps: dict[str, JSON],
                      deadline: float) -> dict[str, JSON]:
        require(deadline <= self.business_deadline and deadline > time.monotonic(),
                "业务动作越过原总截止或预留回收窗")
        matches = [c for c in self.contract["commands"] if c["stage"] == stage]
        require(len(matches) == 1, "阶段命令必须唯一具名")
        command = matches[0]
        require(command["argv"] == argv and command["resource_caps"] == caps,
                "命令或资源帽偏离冻结合同")
        key = (stage, command["domain_id"], command["writer_epoch"])
        require(key not in self.consumed, "本 stage/domain/epoch 已消费，失败不得重试")
        self.consumed.add(key)
        require(bounded_hash(argv[0], deadline) == command["binary_sha256"],
                "启动前 binary 摘要不符")
        require({"path": argv[0], "sha256": command["binary_sha256"]}
                in self.contract["artifacts"], "执行入口不在冻结产物中")
        positive(command["writer_epoch"])
        require(command["domain_id"] in {"S", "E", "B:alpha", "B:beta", "Q"},
                "owner 域未冻结")
        remaining(deadline)
        return command

    def register_owner(self, owner: OwnedProcess) -> None:
        key = (owner.stage, owner.domain, owner.epoch)
        require(key in self.consumed and key not in self.active_owners,
                "owner 未消费阶段或重复注册")
        for prior in self.active_owners.values():
            require(prior.domain != owner.domain or prior.process.poll() is not None,
                    "同一owner域已有存活进程")
        self.active_owners[key] = owner

    def cleanup_all(self, reason: str) -> list[CleanupReceipt]:
        results = []
        for owner in list(self.active_owners.values()):
            # 单 owner 的失败不会阻断后续 owner 或封口。
            try:
                results.append(owner.cleanup(reason))
            except BaseException as error:
                results.append({"owner": getattr(owner, "identity", {}),
                                "returncode": None, "reaped": False,
                                "errors": [type(error).__name__ + ": " + str(error)], "reason": reason,
                                "binary_sha256_before": owner.binary_before,
                                "binary_sha256_after": None})
        return results


class OwnedProcess:
    """本 driver 直接 child；完整后代监督由独立工位实现。"""
    def __init__(self, argv: list[str], output: str | Path, supervisor: Supervisor,
                 stage: str, caps: dict[str, JSON], deadline: float, *,
                 admission: ExecutionAdmission | None = None) -> None:
        require(type(admission) is ExecutionAdmission, "产品 execution_authorized=false")
        command = admission.check_command(argv, stage, caps, deadline)
        self.domain, self.epoch, self.stage = command["domain_id"], command["writer_epoch"], stage
        self.fault = command.get("fault")
        require(supervisor.contract == admission.contract_sha
                and supervisor.endpoint == admission.contract["supervisor_socket"],
                "监督端未绑定相同准入合同")
        self.admission = admission
        self.cleanup_deadline, self.seal_deadline = admission.cleanup_deadline, admission.seal_deadline
        self.term_grace_seconds, self.kill_wait_seconds = admission.term_grace_seconds, admission.kill_wait_seconds
        self.output, self.argv, self.supervisor = Path(output), argv, supervisor
        self.caps, self.deadline = caps, deadline
        # 冻结合同是唯一预期；任何观测不得升级为新的执行授权。
        self.binary_before = command["binary_sha256"]
        require(bounded_hash(argv[0], deadline) == self.binary_before,
                "构造前 binary 偏离冻结摘要")
        require(Path(argv[0]).is_absolute(), "产品 executable 须绝对路径")
        self.stdout = self.stderr = self.process = None
        self.identity: dict[str, JSON] = {}
        self.cleanup_result: CleanupReceipt | None = None
        self.released = False
        self.directory = None
        try:
            remaining(deadline)
            self.directory = OwnedDirectory(self.output)
            self.stdout = self.directory.open_new("stdout")
            self.stderr = self.directory.open_new("stderr")
            remaining(deadline)
            gate = ("import json,os,sys,time; p=json.loads(sys.stdin.buffer.readline()); "
                    "a=p[\"argv\"]; "
                    "sys.exit(124) if time.monotonic() >= p[\"deadline\"] else os.execv(a[0],a)")
            self.process = subprocess.Popen([sys.executable, "-c", gate], stdin=subprocess.PIPE,
                                            stdout=self.stdout, stderr=self.stderr,
                                            start_new_session=True, close_fds=True,
                                            cwd=admission.contract["cwd"],
                                            env=admission.contract["environment"])
            # 在任何 ps/ACK 失败前登记 Popen child；gate 未释放不可能产生产品后代。
            self.identity = {"pid": self.process.pid, "domain_id": self.domain,
                             "writer_epoch": self.epoch}
            admission.register_owner(self)
            self.identity.update(process_identity(self.process, deadline))
            ack = supervisor.ack(stage, self.identity, caps, deadline)
            self._save("ACK.json", ack, deadline)
            self._save("OWNER.json", {**self.identity, "argv": argv}, deadline)
            wire = canonical({"argv": argv, "deadline": deadline}) + b"\n"
            require(len(wire) <= 1048576, "gate 命令超过控制帧")
            require(bounded_hash(argv[0], deadline) == self.binary_before, "放行前 binary 摘要变化")
            self.directory.verify()
            os.set_blocking(self.process.stdin.fileno(), False)
            self.released = True  # 写出首字节前即按可能已执行对待
            sent = 0
            while sent < len(wire):
                remaining(deadline)
                try:
                    sent += os.write(self.process.stdin.fileno(), wire[sent:])
                except BlockingIOError:
                    time.sleep(min(0.001, remaining(deadline)))
            remaining(deadline)
            self.released = True
            self.process.stdin.close()
        except BaseException as error:
            # 没有目录所有权时回执随异常交还调用者，不写已有/被替换的路径。
            error.cleanup_receipt = self.cleanup(type(error).__name__ + ": " + str(error))
            raise

    def _save(self, name: str, value: JSON, deadline: float) -> None:
        require(self.directory is not None, "未拥有原件目录")
        save_new(self.output / name, value, deadline=deadline, directory=self.directory)

    def verify_identity(self, *, deadline: float | None = None,
                        allow_exited: bool = False) -> bool:
        limit = self.deadline if deadline is None else deadline
        if self.process.poll() is not None:
            require(allow_exited, "owner 已退出")
            return False
        try:
            observed = {**process_identity(self.process, limit), "domain_id": self.domain,
                        "writer_epoch": self.epoch}
        except (OSError, Rejected, subprocess.SubprocessError):
            if self.process.poll() is not None and allow_exited:
                return False
            raise
        require(observed == self.identity, "owner 身份漂移")
        return True

    def wait_marker(self, marker_path: str | Path, domain: str, message: str,
                    phase: str) -> dict[str, JSON]:
        require(self.fault == {"marker_path": str(marker_path), "domain_id": domain,
                               "message_id": message, "phase": phase}
                and domain == self.domain, "marker 请求未由本阶段合同冻结")
        try:
            while True:
                self.verify_identity()
                self.supervisor.ack("wait_marker", self.identity, self.caps, self.deadline)
                remaining(self.deadline)
                if Path(marker_path).exists():
                    require(not Path(marker_path).is_symlink(), "marker 不接受symlink")
                    with Path(marker_path).open("rb") as stream:
                        raw_marker = stream.read(self.supervisor.frame + 1)
                    require(len(raw_marker) <= self.supervisor.frame, "marker 超控制帧")
                    marker = strict(raw_marker)
                    remaining(self.deadline)
                    exact(marker, MARKER, "真实 owner marker")
                    require(marker == {"domain_id": domain, "message_id": message,
                                       "phase": phase, "pid": self.process.pid,
                                       "test_only": True}, "marker 不是本次 owner/请求/缝")
                    self._save("MARKER.json", marker, self.deadline)
                    self.verified_marker = marker
                    return marker
                time.sleep(min(0.02, remaining(self.deadline)))
        except BaseException as error:
            self.cleanup(str(error))
            raise

    def kill_at_marker(self, marker: dict[str, JSON]) -> dict[str, JSON]:
        try:
            require(marker == getattr(self, "verified_marker", None), "缺本实例已核真实 marker")
            require(marker["pid"] == self.identity["pid"], "kill 的 PID 与 marker 不同")
            self.verify_identity()
            ack = self.supervisor.ack("SIGKILL", self.identity, self.caps, self.deadline)
            remaining(self.deadline)
            os.kill(self.process.pid, signal.SIGKILL)
            code = self.process.wait(timeout=remaining(self.deadline))
            remaining(self.deadline)
            require(code == -signal.SIGKILL, "未实际 SIGKILL/reap")
            after = bounded_hash(self.argv[0], self.deadline)
            receipt = {"owner": self.identity, "marker": marker, "ACK": ack,
                       "returncode": code, "reaped": True, "time_ns": str(time.time_ns()),
                       "binary_sha256_before": self.binary_before, "binary_sha256_after": after}
            require(after == self.binary_before, "SIGKILL 后 binary 摘要变化")
            self._save("KILL.json", receipt, self.deadline)
            return receipt
        except BaseException as error:
            self.cleanup(str(error))
            raise
        finally:
            self._close_streams()

    def finish(self) -> dict[str, JSON]:
        try:
            self.supervisor.ack("wait_reap", self.identity, self.caps, self.deadline)
            code = self.process.wait(timeout=remaining(self.deadline))
            remaining(self.deadline)
            after = bounded_hash(self.argv[0], self.deadline)
            receipt = {"owner": self.identity, "returncode": code, "reaped": True,
                       "binary_sha256_before": self.binary_before, "binary_sha256_after": after}
            self._save("FINISH.json", receipt, self.deadline)
            require(code == 0 and after == self.binary_before, "writer 非正常完成或 binary 变化")
            return receipt
        except BaseException as error:
            self.cleanup(str(error))
            raise
        finally:
            self._close_streams()

    def _close_streams(self) -> None:
        for stream in (getattr(self.process, "stdin", None), self.stdout, self.stderr):
            if stream is not None:
                try:
                    stream.close()
                except OSError:
                    pass

    def cleanup(self, reason: str = "cleanup") -> CleanupReceipt:
        """无 ACK 前置；异常全部记入回执，始终关闭句柄，并可重复查询结果。"""
        if self.cleanup_result is not None:
            return self.cleanup_result
        errors: list[str] = []
        code, after = None, None
        try:
            if self.process is not None:
                if self.process.poll() is None:
                    # 未释放 gate 是本 Popen child；已释放者必须重核身份。
                    live = (not self.released or self.verify_identity(
                        deadline=self.cleanup_deadline, allow_exited=True))
                    if live:
                        remaining(self.cleanup_deadline)
                        try:
                            os.kill(self.process.pid, signal.SIGTERM)
                        except ProcessLookupError:
                            require(self.process.poll() is not None, "TERM 期间身份无法确认")
                        try:
                            limit = min(self.cleanup_deadline - self.kill_wait_seconds,
                                        time.monotonic() + self.term_grace_seconds)
                            self.process.wait(timeout=remaining(limit))
                        except (subprocess.TimeoutExpired, Rejected):
                            if self.process.poll() is None:
                                if not self.released or self.verify_identity(
                                        deadline=self.cleanup_deadline, allow_exited=True):
                                    remaining(self.cleanup_deadline)
                                    try:
                                        os.kill(self.process.pid, signal.SIGKILL)
                                    except ProcessLookupError:
                                        require(self.process.poll() is not None,
                                                "KILL 期间身份无法确认")
                    # poll 已退出也显式 wait/reap，避免正常退出竞态阻断收尾。
                code = self.process.wait(timeout=remaining(self.cleanup_deadline))
        except BaseException as error:
            errors.append(type(error).__name__ + ": " + str(error))
            if self.process is not None:
                code = self.process.poll()
        finally:
            self._close_streams()
        try:
            after = bounded_hash(self.argv[0], self.cleanup_deadline)
            require(after == self.binary_before, "回收后 binary 摘要变化")
        except BaseException as error:
            errors.append(type(error).__name__ + ": " + str(error))
        result: CleanupReceipt = {"owner": self.identity, "returncode": code,
                                  "reaped": code is not None, "errors": errors, "reason": reason,
                                  "binary_sha256_before": self.binary_before,
                                  "binary_sha256_after": after}
        try:
            if self.directory is not None:
                self._save("CLEANUP.json", result, self.seal_deadline)
        except BaseException as error:
            errors.append("seal: " + type(error).__name__ + ": " + str(error))
        finally:
            if self.directory is not None:
                self.directory.close()
        self.cleanup_result = result
        return result


def inspect(contract_path: str | Path) -> dict[str, JSON]:
    contract = strict(Path(contract_path).read_bytes())
    return {"schema_revision": "issue1394-driver-inspection/1",
            "execution_authorized": False, "product_executed": False,
            "contract_sha256": sha(Path(contract_path).read_bytes()),
            "missing_inputs": [k for k, v in contract.get("inputs", {}).items() if v is None],
            "note": "静态适配层；不包含根准入、外部监督者或完整 Long 业务计划"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inspect-contract", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(inspect(args.inspect_contract), ensure_ascii=False, indent=2))
