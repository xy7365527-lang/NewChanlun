#!/usr/bin/env python3
"""#1373：显式消息经正式 S/Q 入口，保全完整公共状态和持久原件。

本驱动不构造预期结构、不裁定包含。结构 oracle 另由冻结手算账簿核对；
这里的成功仅表示实际进程轨迹完整采集，仍须双跑、oracle 与真实界面验收。
"""

import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import selectors
import sqlite3
import subprocess
import threading
import time

from tb01c_runtime import (
    Run, canonical, decode_frame, exchange, read_config, require_kill_receipt,
    request_for, transport_semantics, validate_reply, wait_for_marker,
)


def write_json(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def helper_exchange(process, command, timeout=35, max_bytes=1048576):
    """同一绝对期限覆盖请求写入和完整 JSONL 帧，不能以首字节当作完成。"""
    deadline = time.monotonic() + timeout
    payload = canonical(command) + b"\n"
    if len(payload) > max_bytes:
        raise ValueError("HTTP 采集命令超过声明大小")
    incoming = bytearray()
    with selectors.DefaultSelector() as selector:
        for pipe in (process.stdin, process.stdout):
            os.set_blocking(pipe.fileno(), False)
        selector.register(process.stdin, selectors.EVENT_WRITE)
        while payload:
            if time.monotonic() >= deadline:
                raise TimeoutError("HTTP 采集命令未在期限内写完")
            if not selector.select(max(0, deadline - time.monotonic())):
                raise TimeoutError("HTTP 采集命令未在期限内写完")
            try:
                sent = os.write(process.stdin.fileno(), payload)
            except BlockingIOError:
                continue
            if not sent:
                raise EOFError("HTTP 采集命令写入提前结束")
            payload = payload[sent:]
        selector.unregister(process.stdin)
        selector.register(process.stdout, selectors.EVENT_READ)
        while True:
            if time.monotonic() >= deadline:
                raise TimeoutError("公共 HTTP 采集未在期限内返回完整帧")
            if not selector.select(max(0, deadline - time.monotonic())):
                raise TimeoutError("公共 HTTP 采集未在期限内返回完整帧")
            try:
                chunk = os.read(process.stdout.fileno(), min(65536, max_bytes + 1 - len(incoming)))
            except BlockingIOError:
                continue
            if not chunk:
                raise EOFError("HTTP 采集响应在完整帧之前结束")
            incoming.extend(chunk)
            if len(incoming) > max_bytes:
                raise ValueError("HTTP 采集响应超过声明大小")
            if b"\n" in incoming:
                raw, extra = incoming.split(b"\n", 1)
                if extra:
                    raise ValueError("HTTP 采集返回未请求的额外帧")
                return decode_frame(raw)


class Tb02aRun(Run):
    def __init__(self, config, messages, output, node, fault_index, fault_stage):
        self.config_path = config.resolve()
        self.config = read_config(self.config_path)
        self.output = output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.state_dir = self.output / "control"
        self.root = Path(__file__).resolve().parents[1]
        self.messages_path = messages.resolve()
        self.messages = [decode_frame(row) for row in self.messages_path.read_bytes().splitlines()]
        if not self.messages or Path(self.config["db"]).exists():
            raise ValueError("必须使用非空冻结消息和未存在的新数据库")
        if fault_index is not None and not 0 <= fault_index < len(self.messages):
            raise ValueError("故障消息索引不在冻结输入内")
        self.fault_index, self.fault_stage = fault_index, fault_stage
        self.node = str(node.resolve())
        self.plan = {"input_reply_timeout_ms": 10000}
        self.event_lock = threading.Lock()
        self.events = (self.output / "events.jsonl").open("x")
        self.results, self.lifecycle = {}, {}
        self.public = []
        self.helper = None
        self.helper_stderr = None
        self.source_hashes = {}
        before = self.output / "source-before-run"
        before.mkdir()
        paths = [self.config_path, self.messages_path, Path(__file__).resolve()]
        paths += [Path(self.config[k]) for k in
                  ("binary", "catalog", "profile", "clock_plan", "query_resource_config")]
        # 保全消费者和输入；准确 Rust 源另由构建提交与构建收据绑定。
        paths += [p for p in self.root.rglob("*") if p.is_file()
                  and p.suffix in {".py", ".js", ".cjs", ".html", ".sh", ".json", ".jsonl"}
                  and "__pycache__" not in p.parts]
        entries = []
        for path in dict.fromkeys(paths):
            raw = path.read_bytes()
            digest = hashlib.sha256(raw).hexdigest()
            destination = before / (str(len(entries)).zfill(4) + "-" + path.name)
            destination.write_bytes(raw)
            self.source_hashes[str(path)] = digest
            entries.append({"source": str(path), "snapshot": destination.name,
                            "bytes": len(raw), "sha256": digest})
        write_json(before / "MANIFEST.json", entries)

    def query(self, command):
        if self.helper is None:
            self.helper_stderr = (self.output / "http-helper.stderr").open("x")
            self.helper = subprocess.Popen(
                [self.node, str(self.root / "tests/tb01c_http_capture.cjs"),
                 "--base-url", "http://127.0.0.1:" + self.config["port"],
                 "--output-dir", str(self.output / "http")],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.helper_stderr,
                bufsize=0,
            )
        response = helper_exchange(self.helper, command)
        if response.get("id") != command["id"] or response.get("result") != "captured_validated_commit":
            raise ValueError("公共查询未完整提交：" + json.dumps(response, ensure_ascii=False))
        receipt = json.loads(Path(response["path"]).read_text())
        candidate = json.loads(Path(receipt["candidate"]["path"]).read_text())
        self.public.append({"command": command, "candidate": candidate})
        return candidate

    def inject_fault(self, index):
        self.control("stop-s")
        marker = self.output / (self.fault_stage + ".marker")
        environment = dict(os.environ)
        environment.update(S_SESSION_PAUSE=self.fault_stage, S_SESSION_PAUSE_MARKER=str(marker),
                           S_SESSION_PAUSE_DB=self.config["db"])
        environment.pop("S_SESSION_PAUSE_RELEASE", None)
        self.control("start-s", "--writer-epoch", "1", environment=environment)
        peer, writer = self.identity("q"), self.identity("s")
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.send, index, time.monotonic_ns())
            wait_for_marker(marker, {"pid": str(writer["pid"]), "stage": self.fault_stage,
                                    "db": self.config["db"], "exe": self.config["binary"]},
                            time.monotonic() + 10)
            before = self.observed_frontier()
            if before != {"generation": str(index), "accepted": str(index + 1), "committed": str(index)}:
                raise ValueError("故障点不是具名的已接纳、未提交消息")
            stop = self.control("stop-s", "--signal", "KILL")
            require_kill_receipt(stop, "s", writer)
            if self.identity("q") != peer:
                raise ValueError("S故障改变Q进程身份")
            if pending.result(timeout=2)["transport"] != "DeliveryUnknown":
                raise ValueError("真实故障未留下原消息DeliveryUnknown")
        self.control("recover", "--new-epoch", "2", "--clock-event-id", "recover-001")
        self.control("start-s", "--writer-epoch", "2")
        after = self.observed_frontier()
        if after != {"generation": str(index + 1), "accepted": str(index + 1), "committed": str(index + 1)}:
            raise ValueError("原始消息未在Ready前完成持久恢复")
        if self.identity("q") != peer:
            raise ValueError("恢复改变Q进程身份")
        self.lifecycle = {"stage": self.fault_stage, "signal": "SIGKILL", "before": before,
                          "after": after, "writer_epoch": "2", "q_survived": True}
        self.emit("fault", old_s=writer, new_s=self.identity("s"), q=peer, stop_receipt=stop)

    def collect_core(self):
        records = [{"kind": "actual_ingest_result", "operation": str(i),
                    "result": transport_semantics(self.results[i], original)}
                   for i, original in enumerate(self.messages)]
        for index, original in enumerate(self.messages):
            payload = {"op": "receipt", "original_source_namespace": original["source_namespace"],
                       "original_source_epoch": original["source_epoch"], "original_message_id": original["message_id"],
                       "original_payload_hash": original["payload_hash"]}
            request = request_for(self.config, "final-receipt-" + str(index), payload)
            response = exchange(self.config["socket"], request, timeout_ms=self.plan["input_reply_timeout_ms"],
                                max_frame_bytes=int(self.config["s_resources"]["max_frame_bytes"]))
            if response["transport"] != "Received":
                raise ValueError("最终原身份收据未完整收到")
            receipt = validate_reply(request, response["response"], producer="S",
                                     producer_epoch="2" if self.fault_index is not None else "1")
            if receipt["kind"] != "Committed" or receipt["accepted_seq"] != str(index):
                raise ValueError("最终原身份收据不完整")
            records.append({"kind": "final_receipt", "operation": str(index), "request": request, "response": response})
        records.extend({"kind": "public_candidate", **item} for item in self.public)
        with closing(sqlite3.connect(Path(self.config["db"]).as_uri() + "?mode=ro", uri=True)) as conn, conn:
            conn.execute("BEGIN")
            tables = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
            for table in tables:
                if not table.replace("_", "").isalnum():
                    raise ValueError("未支持的权威表标识")
                columns = [r[1] for r in conn.execute('PRAGMA table_info("' + table + '")')]
                def scalar(value):
                    if type(value) is bytes:
                        return {"sqlite_type": "blob", "base64": base64.b64encode(value).decode("ascii"),
                                "bytes": str(len(value)), "sha256": hashlib.sha256(value).hexdigest()}
                    return value
                rows = [[scalar(v) for v in row] for row in conn.execute('SELECT * FROM "' + table + '"')]
                rows.sort(key=canonical)
                records.append({"kind": "authoritative_table", "table": table, "columns": columns, "rows": rows})
            records.append({"kind": "authoritative_schema", "rows": list(conn.execute(
                "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name"))})
            with closing(sqlite3.connect(self.output / "database-backup.sqlite")) as backup:
                conn.backup(backup)
        records.append({"kind": "lifecycle_semantics", "value": self.lifecycle})
        with (self.output / "semantic-core.jsonl").open("x") as stream:
            for item in records:
                stream.write(json.dumps(item, ensure_ascii=True, separators=(",", ":")) + "\n")
        return len(records)

    def cleanup(self):
        """各资源独立清理；任何一步失败都保留，且不能跳过其他服务。"""
        outcomes, errors = [], []

        def attempt(action, operation):
            try:
                value = operation()
                outcomes.append({"action": action, "result": value})
                return value
            except Exception as exc:
                errors.append({"action": action, "error": type(exc).__name__ + ": " + str(exc)})
                return None

        if self.helper is not None:
            attempt("helper-close-input", self.helper.stdin.close)
            code = attempt("helper-wait", lambda: self.helper.wait(timeout=35))
            if code is None:
                attempt("helper-kill", self.helper.kill)
                code = attempt("helper-reap", lambda: self.helper.wait(timeout=5))
            if code != 0:
                errors.append({"action": "helper-exit", "returncode": code})
            attempt("helper-close-output", self.helper.stdout.close)
        if self.helper_stderr is not None:
            attempt("helper-close-error-log", self.helper_stderr.close)
        for service in ("q", "s"):
            receipt = attempt("stop-" + service, lambda service=service: self.control("stop-" + service))
            if receipt is not None and (receipt.get("service") != service or receipt.get("state") not in ("stopped", "not_started")):
                errors.append({"action": "stop-" + service, "invalid_receipt": receipt})
        attempt("close-event-log", self.events.close)
        return {"outcomes": outcomes, "errors": errors}

    def execute(self):
        result = {"status": "NOT_VERIFIED", "semantic_oracle": "not_evaluated",
                  "browser": "not_evaluated", "input_count": len(self.messages)}
        try:
            self.control("start")
            originals = []
            for index in range(len(self.messages)):
                if index == self.fault_index:
                    self.inject_fault(index)
                elif self.send(index, time.monotonic_ns())["transport"] != "Received":
                    raise ValueError("非故障消息未完整接收")
                op = "load" if index == 0 else "watch"
                self.query({"id": "live-" + str(index), "client": "live", "op": op})
                originals.append(self.query({"id": "known-" + str(index), "client": "history",
                                             "op": "load", "mode": "AsKnown", "generation": str(index + 1)}))
            for index, expected in enumerate(originals):
                actual = self.query({"id": "revisit-" + str(index), "client": "history", "op": "load",
                                     "mode": "AsKnown", "generation": str(index + 1)})
                if actual != expected:
                    raise ValueError("后续输入/修订改变了旧cut的完整公共候选：" + str(index + 1))
            result["core_records"] = self.collect_core()
            if {p: hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in self.source_hashes} != self.source_hashes:
                raise ValueError("实际运行期间受测源码/构建/输入发生变化")
            result["status"] = "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER"
        except Exception as exc:
            result.update(status="FAIL", error=type(exc).__name__ + ": " + str(exc))
        finally:
            result["cleanup"] = self.cleanup()
            if result["cleanup"]["errors"]:
                result["status"] = "FAIL"
            write_json(self.output / "RESULT.json", result)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("config", "messages", "output", "node"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--fault-index", type=int)
    parser.add_argument("--fault-stage", choices=("after_begin", "after_batch"), default="after_batch")
    args = parser.parse_args()
    result = Tb02aRun(**vars(args)).execute()
    print(json.dumps(result, ensure_ascii=False))
    return 0 if result["status"] == "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER" else 1


if __name__ == "__main__":
    raise SystemExit(main())
