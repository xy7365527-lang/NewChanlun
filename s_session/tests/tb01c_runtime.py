#!/usr/bin/env python3
"""#1372：正式 launcher/常驻 S/Q 的冻结输入与真实故障驱动；不生成结构结果。

计时观察、完整请求/回执和逐代权威记录分别留存。任何失败保留原件，
本脚本成功只证明它实际覆盖的进程轨迹；GUI/容量/完整验收仍须独立审阅。
"""

import argparse
import base64
import concurrent.futures
from contextlib import closing
import hashlib
import json
import os
import signal
import sqlite3
import subprocess
import sys
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from s_service_control import canonical, process_identity, read_config
from s_socket_client import decode_frame, exchange, validate_reply


TABLES = ("batches", "catalog", "meta", "objects", "observations", "raw_events", "relations",
          "s_clock_events", "s_delivery_policy", "s_delivery_refs", "s_input_messages",
          "s_protocol_meta", "structure_deltas", "witnesses", "writer_epoch_history")
# 固定160个实际输入结果+160个最终收据+160个历史cut+13个其余表+schema+故障语义。
EXPECTED_CORE_RECORDS = 495


class NotVerified(RuntimeError):
    """没有观测到要求的真实故障，不得把已停止或推测当成已注入。"""


def require_kill_receipt(result, service, record):
    if (result.get("service") != service or result.get("state") != "stopped"
            or result.get("pid") != record["pid"] or result.get("signal") != "SIGKILL"):
        raise NotVerified("没有目标进程实际 SIGKILL 的完整控制回执：" + service)


def wait_for_marker(marker, expected, deadline):
    """路径可先于完整内容可见；只在原期限内等待，完整身份仍须严格相同。"""
    while time.monotonic() < deadline:
        try:
            with marker.open("rb") as stream:
                raw = stream.read(65537)
        except FileNotFoundError:
            raw = b""
        if len(raw) > 65536:
            raise ValueError("暂停标记超过有界大小")
        if raw.endswith(b"\n") and raw.count(b"\n") >= len(expected):
            lines = raw.decode("utf-8").splitlines()
            pairs = [line.split("=", 1) for line in lines]
            if (len(pairs) != len(expected) or any(len(pair) != 2 for pair in pairs)
                    or len({pair[0] for pair in pairs}) != len(expected)
                    or dict(pairs) != expected):
                raise ValueError("暂停标记与正式S进程不符")
            return
        time.sleep(min(0.02, max(0, deadline - time.monotonic())))
    raise TimeoutError("原期限内未观测到完整after_begin marker")


def transport_semantics(result, original):
    """仅系统诊断 detail 单列；正式响应、未知状态和原消息身份仍完整比较。"""
    if result.get("transport") == "Received":
        if set(result) != {"transport", "response"}:
            raise ValueError("Received 输送字段集合变化")
        return result
    if (result.get("transport") not in ("DeliveryUnknown", "TransportUnavailable")
            or set(result) != {"transport", "message_id", "detail"}
            or result["message_id"] != original["message_id"] or type(result["detail"]) is not str):
        raise ValueError("未完整接收输送的原身份/字段不符")
    return {"transport": result["transport"], "message_id": result["message_id"]}


def request_for(config, message_id, payload):
    return {"schema_revision": "s-session/2", "session_id": config["session_id"],
            "session_generation": config["session_generation"],
            "source_namespace": config["source_namespace"], "source_epoch": config["source_epoch"],
            "message_id": message_id, "producer_id": "tb01c-input-driver", "producer_epoch": "1",
            "payload_hash": hashlib.sha256(canonical(payload)).hexdigest(), "causal_refs": [], "payload": payload}


class Run:
    def __init__(self, config_path, state_dir, output):
        self.config_path = config_path.resolve()
        self.config = read_config(self.config_path)
        self.state_dir = state_dir.resolve()
        self.output = output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.fixture = Path(__file__).parent / "fixtures" / "tb01c"
        self.plan = decode_frame((self.fixture / "runtime-plan.json").read_bytes())
        manifest = decode_frame((self.fixture / "RUNTIME-MANIFEST.json").read_bytes())
        for name, value in manifest["files"].items():
            raw = (self.fixture / name).read_bytes()
            if len(raw) != value["bytes"] or hashlib.sha256(raw).hexdigest() != value["sha256"]:
                raise ValueError("冻结输入原件已变化：" + name)
        self.messages = [decode_frame(line) for line in (self.fixture / "messages.jsonl").read_bytes().splitlines()]
        if len(self.messages) != 160 or self.plan["input_count"] != 160:
            raise ValueError("本驱动只执行显式160项冻结轨迹")
        for field, path in (("clock_plan", "clock-plan.json"), ("profile", "profile.json"),
                            ("query_resource_config", "query-resources.json")):
            if Path(self.config[field]).read_bytes() != (self.fixture / path).read_bytes():
                raise ValueError("launcher与冻结输入来源不同：" + field)
        if self.config["s_resources"] != self.plan["s_resources"]:
            raise ValueError("S资源必须与冻结运行计划相同")
        if Path(self.config["db"]).exists():
            raise ValueError("完整轨迹必须从未存在的新数据库开始")
        self.event_lock = threading.Lock()
        self.events = (self.output / "events.jsonl").open("x")
        self.results = {}
        self.lifecycle = {}
        root = Path(__file__).resolve().parents[1]
        files = [root / name for name in ("s_readonly_server.py", "s_query.py", "s_query_integrity.py",
                 "browser/index.html", "browser/tb01c-client.js", "launch_s.sh", "s_socket_client.py",
                 "s_service_control.py", "tests/tb01c_runtime.py", "tests/tb01c_load_report.py",
                 "tests/tb01c_http_capture.cjs", "tests/test_tb01c_browser.cjs")]
        files += [Path(self.config[key]) for key in ("binary", "catalog", "profile", "clock_plan", "query_resource_config")]
        files += [self.config_path, self.fixture / "RUNTIME-MANIFEST.json"]
        files += [self.fixture / name for name in manifest["files"]]
        # 启动前保全实际代码/配置/输入原件；后续修复不得只留下无法还原的hash。
        frozen = self.output / "source-before-run"
        frozen.mkdir(exist_ok=False)
        self.source_hashes = {}
        preserved = []
        for path in dict.fromkeys(files):
            raw = path.read_bytes()
            digest = hashlib.sha256(raw).hexdigest()
            name = str(len(preserved)).zfill(2) + "-" + path.name
            with (frozen / name).open("xb") as snapshot:
                snapshot.write(raw)
            self.source_hashes[str(path)] = digest
            preserved.append({"source": str(path), "snapshot": name, "bytes": len(raw), "sha256": digest})
        (frozen / "MANIFEST.json").write_text(json.dumps(preserved, ensure_ascii=True, indent=2) + "\n")
        self.emit("input_binding", manifest=manifest, expected_core_records=EXPECTED_CORE_RECORDS,
                  config=self.config, source_hashes=self.source_hashes, source_snapshots=preserved)

    def emit(self, kind, **fields):
        with self.event_lock:
            self.events.write(json.dumps({"kind": kind, "observed_monotonic_ns": str(time.monotonic_ns()), **fields},
                                         ensure_ascii=True, separators=(",", ":")) + "\n")
            self.events.flush()

    def control(self, action, *extra, environment=None):
        command = [str(Path(__file__).resolve().parents[1] / "launch_s.sh"), "service", action,
                   "--config", str(self.config_path), "--state-dir", str(self.state_dir), *extra]
        start = time.monotonic_ns()
        try:
            result = subprocess.run(command, env=environment, capture_output=True,
                                    timeout=int(self.config["startup_timeout_ms"]) / 1000 * 2 + 2)
        except subprocess.TimeoutExpired as exc:
            self.emit("control_timeout", action=action, command=command,
                      stdout=(exc.stdout or b"").decode("utf-8", errors="replace"),
                      stderr=(exc.stderr or b"").decode("utf-8", errors="replace"))
            raise
        self.emit("control", action=action, command=command, exit_code=result.returncode,
                  stdout=result.stdout.decode("utf-8", errors="replace"),
                  stderr=result.stderr.decode("utf-8", errors="replace"), duration_ns=str(time.monotonic_ns()-start))
        if result.returncode:
            raise ValueError("正式launcher动作失败：" + action)
        payload = decode_frame(result.stdout)["result"]
        if action in ("stop-s", "stop-q"):
            # 正式CLI的停止结果是列表；本驱动每次只停止一个具名服务。
            if type(payload) is not list or len(payload) != 1 or type(payload[0]) is not dict:
                raise NotVerified("单服务停止回执必须为唯一结果列表")
            return payload[0]
        return payload

    def identity(self, service):
        record = decode_frame((self.state_dir / (service + ".process.json")).read_bytes())
        if process_identity(record["pid"], record["command"]) != record["process_identity"]:
            raise ValueError("真实进程不再具有已登记身份：" + service)
        return record

    def observed_frontier(self):
        with closing(sqlite3.connect(Path(self.config["db"]).as_uri() + "?mode=ro", uri=True, timeout=0.2)) as conn, conn:
            conn.execute("BEGIN")
            generation = conn.execute("SELECT value FROM meta WHERE key='generation'").fetchone()[0]
            accepted = conn.execute("SELECT COUNT(*) FROM s_input_messages").fetchone()[0]
            committed = conn.execute("SELECT COUNT(*) FROM s_input_messages WHERE status='Committed'").fetchone()[0]
        return {"generation": generation, "accepted": str(accepted), "committed": str(committed)}

    def send(self, index, planned_ns):
        request = self.messages[index]
        sent = time.monotonic_ns()
        result = exchange(self.config["socket"], request,
                          timeout_ms=self.plan["input_reply_timeout_ms"],
                          max_frame_bytes=int(self.config["s_resources"]["max_frame_bytes"]))
        received = time.monotonic_ns()
        self.emit("ingest", operation=index, planned_ns=str(planned_ns), sent_ns=str(sent),
                  received_ns=str(received), request=request, result=result)
        self.results[index] = result
        if result["transport"] == "Received":
            payload = validate_reply(request, result["response"], producer="S",
                                     producer_epoch=request["payload"]["writer_epoch"])
            if payload.get("kind") != "Committed" or payload.get("accepted_seq") != str(index):
                raise ValueError("S正式接纳/发布顺序或结果不符：" + str(index))
        return result

    def phase(self, start, stop, *, restart_query=False):
        anchor = time.monotonic_ns()
        self.emit("phase_begin", first_operation=start, stop_exclusive=stop, phase_anchor_ns=str(anchor))
        done = threading.Event()
        monitor_errors = []
        restart_worker = None

        def restart_at(frontier, trigger):
            try:
                peer_before = self.identity("s")
                old_query = self.identity("q")
                stopped = self.control("stop-q", "--signal", "KILL")
                require_kill_receipt(stopped, "q", old_query)
                peer_during = self.identity("s")
                self.control("start-q", "--query-epoch", "2")
                peer_after = self.identity("s")
                new_query = self.identity("q")
                if peer_before != peer_during or peer_before != peer_after or old_query["pid"] == new_query["pid"]:
                    raise ValueError("Q真实重启改变了S身份，或未产生新Q")
                self.lifecycle["q_restart"] = {"trigger_after_operation": str(trigger-1),
                                               "min_generation": str(trigger),
                                               "signal": "SIGKILL", "producer_epoch": "2", "s_survived": True}
                self.emit("q_restart_identity", observed_generation_at_trigger=frontier["generation"],
                          s=peer_before, old_q=old_query, new_q=new_query, stop_receipt=stopped)
            except Exception as exc:
                monitor_errors.append(exc)
                self.emit("q_restart_failure", detail=str(exc))

        def monitor():
            nonlocal restart_worker
            try:
                while not done.is_set():
                    frontier = self.observed_frontier()
                    self.emit("frontier", phase_start=start, phase_offset_ns=str(time.monotonic_ns()-anchor), **frontier)
                    trigger = self.plan["q_restart"]["after_committed_operation"] + 1
                    if restart_query and restart_worker is None and int(frontier["generation"]) >= trigger:
                        # 生命周期等待不能停止100ms前沿观测；输入调度也不依赖两者。
                        restart_worker = threading.Thread(target=restart_at, args=(frontier, trigger))
                        restart_worker.start()
                    done.wait(0.1)
                if restart_query and restart_worker is None:
                    raise ValueError("未到达预声明Q重启触发")
            except Exception as exc:
                monitor_errors.append(exc)
                self.emit("monitor_failure", detail=str(exc))

        observer = threading.Thread(target=monitor)
        observer.start()
        futures = []
        try:
            # 调度器按单独墙钟发送；不等待S回执或任何Q/浏览器确认。
            with concurrent.futures.ThreadPoolExecutor(max_workers=32) as pool:
                for index in range(start, stop):
                    planned = anchor + (index-start) * self.plan["send_interval_ms"] * 1000000
                    delay = (planned-time.monotonic_ns()) / 1000000000
                    if delay > 0:
                        time.sleep(delay)
                    futures.append(pool.submit(self.send, index, planned))
                drain_deadline = time.monotonic() + self.plan["post_last_offer_drain_ms"] / 1000
                for future in futures:
                    result = future.result(timeout=max(0.001, drain_deadline-time.monotonic()))
                    if result["transport"] != "Received":
                        raise ValueError("健康输入出现未决输送，必须诊断，不能继续当作已提交")
        finally:
            done.set()
            observer.join(timeout=3)
            if restart_worker is not None:
                restart_worker.join(timeout=4 * int(self.config["startup_timeout_ms"]) / 1000 + 5)
        if observer.is_alive():
            raise TimeoutError("生命周期观测线程未在界限内结束")
        if restart_worker is not None and restart_worker.is_alive():
            raise TimeoutError("Q生命周期动作未在界限内结束")
        if monitor_errors:
            raise monitor_errors[0]
        self.emit("phase_end", first_operation=start, stop_exclusive=stop,
                  phase_offset_ns=str(time.monotonic_ns()-anchor), **self.observed_frontier())

    def fault(self):
        self.control("stop-s")
        marker = self.output / "after-begin.marker"
        environment = dict(os.environ)
        environment.update(S_SESSION_PAUSE="after_begin", S_SESSION_PAUSE_MARKER=str(marker),
                           S_SESSION_PAUSE_DB=self.config["db"])
        environment.pop("S_SESSION_PAUSE_RELEASE", None)
        self.control("start-s", "--writer-epoch", "1", environment=environment)
        peer_before = self.identity("q")
        writer_before = self.identity("s")
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(self.send, 95, time.monotonic_ns())
            deadline = time.monotonic() + self.plan["input_reply_timeout_ms"] / 1000
            wait_for_marker(marker, {"pid": str(writer_before["pid"]), "stage": "after_begin",
                                    "db": self.config["db"], "exe": self.config["binary"]}, deadline)
            frontier = self.observed_frontier()
            if frontier != {"generation": "95", "accepted": "96", "committed": "95"}:
                raise ValueError("故障触发不在预声明的持久接纳/Begin阶段")
            stopped = self.control("stop-s", "--signal", "KILL")
            require_kill_receipt(stopped, "s", writer_before)
            if self.identity("q") != peer_before:
                raise ValueError("停止S改变了Q身份")
            result = future.result(timeout=2)
            if result["transport"] != "DeliveryUnknown":
                raise ValueError("Begin后真实SIGKILL没有表现为原消息DeliveryUnknown")
        self.control("recover", "--new-epoch", "2", "--clock-event-id", "recover-001")
        self.control("start-s", "--writer-epoch", "2")
        if self.identity("q") != peer_before:
            raise ValueError("S恢复改变了Q身份")
        after = self.observed_frontier()
        if after != {"generation": "96", "accepted": "96", "committed": "96"}:
            raise ValueError("S未在Ready前按原身份恢复已接纳消息")
        self.lifecycle["s_restart"] = {"before": frontier, "after": after, "signal": "SIGKILL",
                                       "writer_epoch": "2", "q_survived": True}
        self.emit("s_restart_identity", old_s=writer_before, new_s=self.identity("s"), q=peer_before,
                  stop_receipt=stopped)

    def core_trace(self):
        with (self.output / "semantic-core.jsonl").open("x") as output:
            count = 0
            def write(record):
                nonlocal count
                output.write(json.dumps(record, ensure_ascii=True, separators=(",", ":")) + "\n")
                count += 1
            for index in range(160):
                write({"kind": "actual_ingest_result", "operation": str(index),
                       "result": transport_semantics(self.results[index], self.messages[index])})
            for index, original in enumerate(self.messages):
                payload = {"op": "receipt", "original_source_namespace": original["source_namespace"],
                           "original_source_epoch": original["source_epoch"], "original_message_id": original["message_id"],
                           "original_payload_hash": original["payload_hash"]}
                request = request_for(self.config, "final-receipt-" + str(index), payload)
                response = exchange(self.config["socket"], request, timeout_ms=self.plan["input_reply_timeout_ms"],
                                    max_frame_bytes=int(self.config["s_resources"]["max_frame_bytes"]))
                if response["transport"] != "Received":
                    raise ValueError("最终原身份收据未完整收到")
                receipt = validate_reply(request, response["response"], producer="S", producer_epoch="2")
                if receipt["kind"] != "Committed" or receipt["accepted_seq"] != str(index):
                    raise ValueError("最终原身份收据不完整")
                write({"kind": "final_receipt", "operation": str(index), "request": request, "response": response})
            with closing(sqlite3.connect(Path(self.config["db"]).as_uri() + "?mode=ro", uri=True)) as conn, conn:
                conn.execute("BEGIN")
                names = tuple(row[0] for row in conn.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"))
                if names != TABLES:
                    raise ValueError("权威表清单变化，禁止漏表后继续硬比较")
                def rows(table, where="", args=()):
                    columns = [row[1] for row in conn.execute('PRAGMA table_info("' + table + '")')]
                    query = 'SELECT * FROM "' + table + '"'
                    if where:
                        query += ' WHERE ' + where
                    def scalar(value):
                        if type(value) is bytes:
                            # SQLite BLOB 原字节完整保留；不是仅留hash或把TEXT/BLOB混成一种值。
                            return {"sqlite_type": "blob", "base64": base64.b64encode(value).decode("ascii"),
                                    "bytes": str(len(value)), "sha256": hashlib.sha256(value).hexdigest()}
                        return value
                    values = [[scalar(value) for value in row] for row in conn.execute(query, args)]
                    values.sort(key=lambda row: canonical(row))
                    return {"columns": columns, "rows": values}
                all_batch_ids = {row[0] for row in conn.execute("SELECT batch_id FROM batches")}
                referenced_batch_ids = set()
                if conn.execute("SELECT COUNT(*) FROM structure_deltas").fetchone()[0] != 160:
                    raise ValueError("Delta全表含额外或缺失行，不能只输出可见160代")
                for generation in range(1, 161):
                    delta = rows("structure_deltas", "generation=?", (generation,))
                    if len(delta["rows"]) != 1:
                        raise ValueError("逐代Delta缺失或重复")
                    batch_id = delta["rows"][0][delta["columns"].index("index_frontier")]
                    referenced_batch_ids.add(batch_id)
                    value = {"structure_deltas": delta, "batches": rows("batches", "batch_id=?", (batch_id,))}
                    if any(len(item["rows"]) != 1 for item in value.values()):
                        raise ValueError("逐代批次/Delta缺失或重复")
                    write({"kind": "authoritative_cut", "generation": str(generation), "tables": value})
                if referenced_batch_ids != all_batch_ids or len(all_batch_ids) != 160:
                    raise ValueError("批次全表与本after_begin轨迹的160代引用不闭合，数据库原件保留待查")
                for table in TABLES:
                    if table not in ("batches", "structure_deltas"):
                        write({"kind": "authoritative_table", "table": table, **rows(table)})
                write({"kind": "authoritative_schema", "rows": list(conn.execute(
                    "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name"))})
            write({"kind": "lifecycle_semantics", "value": self.lifecycle})
        if count != EXPECTED_CORE_RECORDS:
            raise ValueError("核心轨迹记录数不符合预声明")
        return count

    def execute(self):
        result = {"status": "NOT_VERIFIED", "expected_core_records": EXPECTED_CORE_RECORDS,
                  "not_covered_by_this_driver": ["浏览器", "v2多页/Watch/断连/积压", "Q完整性扰动", "负载窗口独立分析"]}
        try:
            self.control("start")
            self.phase(0, 95)
            self.fault()
            self.phase(96, 160, restart_query=True)
            result["actual_core_records"] = self.core_trace()
            after = {path: hashlib.sha256(Path(path).read_bytes()).hexdigest() for path in self.source_hashes}
            if after != self.source_hashes:
                raise ValueError("运行期间候选源或构建产物变化，不能绑定为同一候选证据")
            result["status"] = "PROCESS_TRAJECTORY_CAPTURED_REQUIRES_COMPARISON_AND_REVIEW"
        except Exception as exc:
            result["status"] = "NOT_VERIFIED" if isinstance(exc, (NotVerified, TimeoutError, subprocess.TimeoutExpired)) else "FAIL"
            result["error"] = type(exc).__name__ + ": " + str(exc)
            self.emit("run_failure", **result)
            raise
        finally:
            (self.output / "RESULT.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
            self.events.close()
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = Run(args.config, args.state_dir, args.output).execute()
        print(json.dumps(result, ensure_ascii=False))
        return 0
    except Exception as exc:
        print(json.dumps({"status": "NOT_VERIFIED", "error": str(exc)}, ensure_ascii=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
