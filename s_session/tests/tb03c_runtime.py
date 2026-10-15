#!/usr/bin/env python3
"""#1406 C7a：WE-5 正式 E/B 进程、双跑与 Resolve 提交边界恢复。

只有完整六轨运行成功才输出 PASS_C7A；不构建二进制、不启动 S/浏览器。
原件/独立 oracle 固定，生产数据库仅作实际观测，绝不作为资金期望来源。
"""
import argparse
import copy
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
import hashlib
import json
from pathlib import Path
import shutil
import signal
import sqlite3
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT))
from s_socket_client import canonical, decode_frame, digest, exchange, validate_reply
import tb03c_oracle as oracle

FIXTURES = HERE / "fixtures" / "tb03c"
SEED = HERE / "fixtures" / "tb03c_runtime" / "SEED.json"
SOURCE = {"source_namespace": "testonly-tb1406", "source_epoch": "fixture-epoch-1",
          "producer_id": "tb1406-c7a", "producer_epoch": "1"}
DOMAINS = ("E", "B:chong-R1")
TABLES = ("meta", "epochs", "commits", "facts", "pending", "applications", "unmatched", "book", "messages", "receipts")
BINDINGS = {"tb03c_oracle.py", "fixtures/tb03c/MANIFEST.json", "fixtures/tb03c/HAND-ORACLE.json",
            "fixtures/tb03c/raw/chong-R1.json", "fixtures/tb03c/raw/chong-U1.json"}


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def equal(actual, expected, detail):
    if actual != expected:
        raise ValueError(detail + ": " + json.dumps({"expected": expected, "actual": actual}, ensure_ascii=False, default=repr))


def save(path, value):
    with Path(path).open("xb") as stream:
        stream.write(canonical(value) + b"\n")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def event_at(doc, pointer):
    parts = pointer.split("/")
    require(len(parts) == 3 and parts[1] == "events" and parts[2].isdigit(), "原件指针无效")
    return doc["events"][int(parts[2])]


def identity(event):
    return digest(list(oracle.fact_identity(event)))


def fact_hash(raw, pointer):
    doc = decode_frame(raw.encode())
    return digest({"raw_sha256": hashlib.sha256(raw.encode()).hexdigest(), "json_pointer": pointer,
                   "event": event_at(doc, pointer), "units": doc["units"]})


def load_inputs(seed_path=SEED):
    seed = decode_frame(Path(seed_path).read_bytes())
    equal(seed["schema_revision"], "tb03c-runtime-seed/1", "种子版本")
    equal(seed["scope"], "TestOnly WE-5 C7a", "种子范围")
    equal({item["path"] for item in seed["bindings"]}, BINDINGS, "来源集合不得漏绑")
    equal(len(seed["bindings"]), len(BINDINGS), "来源不得重复")
    for item in seed["bindings"]:
        path = (HERE / item["path"]).resolve()
        require(path.is_relative_to(HERE), "绑定路径越界")
        equal(sha(path), item["sha256"], "来源哈希不符 " + item["path"])
    hand = decode_frame((FIXTURES / "HAND-ORACLE.json").read_bytes())
    equal(seed["e_case_index"], 0, "只运行完整 WE-5")
    case = hand["e_cases"][seed["e_case_index"]]
    equal(case["chong_id"], "chong-R1", "具名重")
    equal(oracle.run_e_case(FIXTURES, case), (len(case["steps"]), []), "独立 E oracle 自检")
    for field in ("startup_timeout_ms", "exchange_timeout_ms", "marker_timeout_ms", "process_timeout_ms"):
        require(type(seed[field]) is int and 0 < seed[field] <= 60000, "期限须具名且有界")
    return seed, case


def read_domain(path):
    """一次只读快照，保留所有十表原始 SQL 行（含 JSON 文本）；不归一化经济字段。"""
    with closing(sqlite3.connect(Path(path).resolve().as_uri() + "?mode=ro", uri=True, timeout=5)) as db:
        db.execute("PRAGMA query_only=ON")
        db.execute("BEGIN")
        tables = {row[0] for row in db.execute("SELECT name FROM sqlite_schema WHERE type='table'")}
        equal(tables, set(TABLES), "C7a /1 域表集合")
        result = {}
        for table in TABLES:
            columns = [row[1] for row in db.execute('PRAGMA table_info("' + table + '")')]
            rows = [list(row) for row in db.execute('SELECT * FROM "' + table + '" ORDER BY 1')]
            result[table] = {"columns": columns, "rows": rows}
        db.rollback()
    return result


def image_of(snapshot):
    columns = snapshot["commits"]["columns"]
    return decode_frame(snapshot["commits"]["rows"][-1][columns.index("image")].encode())


def receipts_of(snapshot):
    return {row[0]: decode_frame(row[1].encode()) for row in snapshot["receipts"]["rows"]}


def assert_recovery(before, after, recovered):
    """仅允许 epoch 与 meta.writer_epoch 按 recovery 回执改变，其他原始表逐字不变。"""
    old_meta, new_meta = dict(before["meta"]["rows"]), dict(after["meta"]["rows"])
    old_epoch = int(old_meta["writer_epoch"])
    equal(recovered["writer_epoch"], str(old_epoch + 1), "recover 实际 epoch")
    old_meta["writer_epoch"] = recovered["writer_epoch"]
    equal(new_meta, old_meta, "recover 改了非 epoch meta")
    equal(after["epochs"]["rows"], before["epochs"]["rows"] +
          [[old_epoch + 1, "recover", recovered["recover_ns"]]], "恢复事件史")
    for table in TABLES:
        if table not in ("meta", "epochs"):
            equal(after[table], before[table], "恢复改变已持久表 " + table)


class Expected:
    """只接收冻结原件/预期动作；不接收生产输出来计算金额。"""
    def __init__(self):
        self.book = None
        self.facts = {}
        self.pending = {}
        self.applied = {}
        self.resolved = {}
        self.components = []
        self.totals = {k: 0 for k in ("acquisition_cash", "disposal_cash", "commission", "fee_only", "dividend_cash", "withholding_tax")}

    def advance(self, step):
        if step["outcome"] not in ("Recorded", "Resolved", "Applied", "Unmatched"):
            return
        if step["op"] == "resolve":
            if step["evidence_id"] not in self.resolved:
                self.resolved[step["evidence_id"]] = copy.deepcopy(step)
                doc = decode_frame((FIXTURES / "raw/chong-R1.json").read_bytes())
                for covered in step["covered"]:
                    self.pending.pop(identity(event_at(doc, covered["pointer"])))
            return
        raw = (FIXTURES / step["raw"]).read_text()
        doc = decode_frame(raw.encode())
        event = event_at(doc, step["pointer"])
        key = identity(event)
        if step["op"] == "capture":
            require(key not in self.facts, "本轨基础 capture 不应重送")
            self.facts[key] = (raw, step["pointer"], event)
            reason = {"kind": step["pending_reason"]}
            if reason["kind"] == "AwaitingDependency":
                reason["on"] = digest(list(oracle.dependency_identity(event)))
            self.pending[key] = {"token": "t-" + event["source"]["external_fact_id"], "scope": step["scope"], "reason": reason}
        elif key not in self.applied:
            oracle.verify_fill_predecessors(event, {oracle.fact_identity(v[2]): v[2] for v in self.applied.values()})
            self.book, outcome = oracle.apply(self.book, doc, step["pointer"])
            equal(outcome, step["outcome"], "独立金额 oracle outcome")
            require(outcome == "Applied", "WE-5 本片不应意外含 Unmatched 成功域")
            self.applied[key] = (raw, step["pointer"], event)
            for component in event["known_cost_components"]:
                c = dict(component, fact_key=key, raw_sha256=hashlib.sha256(raw.encode()).hexdigest())
                self.components.append(c)
                if c["kind"] in self.totals:
                    self.totals[c["kind"]] += oracle.num(c["canonical_amount"])

    def check_records(self, records, expected):
        equal({r["fact_key"] for r in records}, set(expected), "原件集合")
        equal(len(records), len(expected), "原件不重复")
        for record in records:
            raw, pointer, event = expected[record["fact_key"]]
            for key, value in {"event": event, "raw_utf8": raw, "json_pointer": pointer,
                               "raw_sha256": hashlib.sha256(raw.encode()).hexdigest(),
                               "fact_hash": fact_hash(raw, pointer), "token": "t-" + event["source"]["external_fact_id"]}.items():
                equal(record[key], value, "原件身份/内容 " + key)

    def check_book(self, actual):
        if self.book is None:
            equal(actual, None, "未开局书账")
            return None
        view = self.book.view()
        equal(set(actual), {"book_revision", "chong_id", "symbol", "operation_level", "fixed_q", "side", "units",
                            "family_id", "opening_fact_ref", "opening_quantity", "actual_quantity", "income_book",
                            "known_cost_components", "cost_totals", "realized", "principal", "phase", "unknown",
                            "voice", "admission", "cost_remaining", "cost_basis", "unit_cost", "stable_point"}, "WE-5 书账完整键集")
        # 独立 oracle 所有 view 字段均有对应，不按 HAND 的小字段子集取样。
        got = {"q": oracle.num(actual["actual_quantity"]), "K": oracle.num(actual["cost_basis"]["K"]),
               "P": oracle.num(actual["principal"]["P"]) if actual["principal"]["status"] == "Known" else None,
               "P_recovered": oracle.num(actual["principal"]["P_recovered"]),
               **{k: oracle.num(actual["income_book"][k]) for k in ("Pi", "A", "W", "R")},
               "fixed_q": oracle.num(actual["fixed_q"]), "fees": oracle.num(actual["realized"]["fees"]),
               "realized_gross": oracle.num(actual["realized"]["gross"]),
               "cost_remaining": oracle.num(actual["cost_remaining"]["value"]),
               "growth_eligible": actual["phase"]["growth_eligible"]["status"], "latch": actual["phase"]["latch"],
               "gate": None if actual["phase"]["gate"] is None else actual["phase"]["gate"]["kind"],
               "growth_enabled": actual["phase"]["growth_enabled"]["status"],
               "unit_cost": {"K": oracle.num(actual["unit_cost"]["K"]), "q": oracle.num(actual["unit_cost"]["q"])},
               "stable_point": actual["stable_point"]["seq"]}
        equal(got, view, "全部独立金额/阶段字段")
        equal(actual["phase"]["events"], self.book.events, "WE-5 阶段事件史")
        equal(actual["principal"]["batches"], [], "无本金批不得当零本金")
        equal(actual["principal"]["status"], "NotEvaluable", "本金状态")
        equal(actual["principal"], {"batches": [], "P_recovered": "0", "status": "NotEvaluable", "reason": "no authorized principal batch"}, "本金完整结构")
        equal(actual["phase"]["growth_enabled"], {"status": "NotEvaluable", "reason": "principal batches absent"}, "扩量许可等待原因")
        equal(actual["stable_point"], {"profile": "TestOnly", "seq": self.book.sp}, "稳定点完整身份")
        equal(actual["cost_totals"], {k: str(v) for k, v in self.totals.items()}, "全部成本分量总账")
        equal(actual["known_cost_components"], self.components, "原费用分量及来源")
        equal(actual["realized"]["after_fees"], str(view["realized_gross"] - view["fees"]), "费后实现")
        equal(actual["book_revision"], "tb03c-book/1", "书账政策")
        equal(actual["family_id"], "fam-R1", "家族")
        equal(actual["chong_id"], "chong-R1", "重")
        equal(actual["side"], "long", "方向")
        raw, _, event = next(v for v in self.applied.values() if "observed_existing_book" in v[2])
        equal(actual["units"], decode_frame(raw.encode())["units"], "量纲")
        equal(actual["opening_quantity"], event["observed_existing_book"]["units"], "开局量")
        equal(actual["symbol"], event["book_identity"]["symbol"], "证券")
        equal(actual["operation_level"], event["book_identity"]["op_level"], "操作级别")
        equal(actual["cost_basis"], {"status": "Known", "K": str(view["K"])}, "成本完整结构")
        equal(actual["cost_remaining"], {"status": "Known", "value": str(view["cost_remaining"])}, "剩余成本完整结构")
        equal(actual["unit_cost"], {"status": "Known", "K": str(view["K"]), "q": str(view["q"])}, "单位量完整结构")
        equal(actual["unknown"], {"complete_historical_fees": {"status": "Unknown", "reason": "only named known components available"},
                                 "target": {"status": "Unknown", "reason": "not supplied"}}, "未知字段不得造默认")
        equal(actual["admission"], "NotEvaluated", "不得伪造准入")
        equal(actual["voice"], {"status": "MissingDependency", "reason": "confirmed gamma binding absent", "identity": None}, "不得伪造 gamma")
        return got

    def check(self, e, b):
        self.check_records(e["recorded"], self.facts)
        self.check_records(b["recorded"], self.applied)
        pending = {p["fact_key"]: {k: p[k] for k in ("token", "scope", "reason")} for p in e["pending"]}
        equal(pending, self.pending, "Pending 逐 token/范围/原因")
        equal(len(e["pending"]), len(pending), "Pending 不重复")
        actual_resolved = [h["evidence_id"] for h in e["history"] if h["kind"] == "ResolveEvidence"]
        equal(actual_resolved, list(self.resolved), "Resolved 事件序列不重复")
        equal({a["fact_key"]: (a["allocation_id"], a["status"]) for a in b["applied"]},
              {key: (v[2]["allocation_id"], "Applied") for key, v in self.applied.items()}, "逐件唯一 Applied")
        equal(len(b["applied"]), len(self.applied), "不重复入账")
        equal(b["unmatched"], [], "WE-5 不可降级 Unmatched")
        equal(e["applied"], [], "E 不代 B 应用")
        equal(b["pending"], [], "B 无 E Pending")
        return {"book": self.check_book(b["book"]), "pending": pending,
                "resolved": actual_resolved, "applied": sorted(self.applied),
                "recorded": sorted(self.facts), "components": self.components}

    def check_receipts(self, e, b):
        er, br = receipts_of(e), receipts_of(b)
        equal(set(er), set(self.facts) | {"resolve:" + digest(k) for k in self.resolved}, "E 永久收据全集")
        equal(set(br), {v[2]["allocation_id"] for v in self.applied.values()}, "B 永久收据全集（等待无收据）")
        for key, (_, _, event) in self.applied.items():
            receipt = br[event["allocation_id"]]
            equal(receipt["fact_key"], key, "Applied 回执原事实")
            equal(receipt["kind"], "Applied", "Applied 回执状态")
            equal(receipt["fact_hash"], er[key]["fact_hash"], "Applied 回执原件内容")
            if "observed_existing_book" in event:
                raw, pointer, _ = self.applied[key]
                equal(image_of(b)["book"]["opening_fact_ref"],
                      {"fact_key": key, "fact_hash": fact_hash(raw, pointer), "e_cut": er[key]["cut"],
                       "json_pointer": pointer, "raw_sha256": hashlib.sha256(raw.encode()).hexdigest()}, "开局绑定完整 E 原收据")


class Run:
    def __init__(self, seed, case, work, output, binaries, phase):
        self.seed, self.case, self.work, self.output, self.phase = seed, case, work, output, phase
        self.binaries = binaries
        self.processes, self.streams, self.epochs = {}, [], {}
        self.number = 0
        self.expected = Expected()
        self.recorded = {}
        self.samples = []
        self.requests = {}
        require(len(str(work / "e.sock").encode()) < 100 and len(str(work / "b.sock").encode()) < 100,
                "Unix socket 路径过长；远端使用短的全新输出路径")
        work.mkdir()
        output.mkdir()
        self.manifest = copy.deepcopy(seed["manifest"])
        self.manifest["e"] = {"db": str(work / "e.db"), "socket": str(work / "e.sock")}
        self.manifest["chongs"][0].update(db=str(work / "b.db"), socket=str(work / "b.sock"))
        self.marker = work / "resolve-marker.json"
        if phase:
            self.manifest["test_only_fault"] = {"message_id": "step-014", "phase": phase, "marker_path": str(self.marker)}
        save(work / "manifest.json", self.manifest)
        save(output / "manifest.json", self.manifest)
        self.log = (output / "commands.jsonl").open("x")

    def record(self, entry):
        self.log.write(json.dumps(entry, ensure_ascii=False) + "\n")
        self.log.flush()

    def node(self, domain):
        return self.manifest["e"] if domain == "E" else self.manifest["chongs"][0]

    def command(self, domain, op):
        binary = "e_economic_session" if domain == "E" else "b_chong_session"
        args = [str(self.binaries / binary), op, "--manifest", str(self.work / "manifest.json"), "--db", self.node(domain)["db"]]
        if domain != "E":
            args += ["--chong", "chong-R1"]
        if op == "serve":
            args += ["--epoch", self.epochs[domain]]
            if domain != "E":
                args += ["--e-epoch", self.epochs["E"]]
        return args

    def cli(self, domain, op):
        args = self.command(domain, op)
        done = subprocess.run(args, capture_output=True, timeout=self.seed["process_timeout_ms"] / 1000)
        self.record({"command": args, "exit_code": done.returncode, "stdout": done.stdout.decode(errors="replace"), "stderr": done.stderr.decode(errors="replace")})
        require(done.returncode == 0, "正式 CLI 失败 " + domain + " " + op)
        result = decode_frame(done.stdout)
        equal(result["domain_id"], domain, "CLI domain")
        equal(result["kind"], {"init": "Initialized", "recover": "Recovered"}[op], "CLI outcome")
        self.epochs[domain] = result["writer_epoch"]
        return result

    def start(self, domain):
        require(domain not in self.processes, "owner 已在本驱动运行")
        sock = Path(self.node(domain)["socket"])
        # SIGKILL 留下的路径只在确认旧句柄已退出后移除，且只移除本次私有目录内 socket。
        if sock.exists():
            require(sock.parent == self.work and sock.is_socket(), "残留路径不是本次 socket")
            sock.unlink()
        self.number += 1
        out = (self.output / (str(self.number) + "-serve.stdout")).open("xb")
        err = (self.output / (str(self.number) + "-serve.stderr")).open("xb")
        self.streams += [out, err]
        args = self.command(domain, "serve")
        proc = subprocess.Popen(args, stdout=out, stderr=err)
        self.processes[domain] = proc
        self.record({"command": args, "pid": proc.pid, "event": "spawn"})
        deadline = time.monotonic() + self.seed["startup_timeout_ms"] / 1000
        while time.monotonic() < deadline:
            require(proc.poll() is None, "owner 在 socket 就绪前退出 " + domain)
            if sock.is_socket():
                # 完整身份校验的 ReadView 才作为正式就绪证据。
                reply = self.send(domain, {"op": "ReadView", "cut": None, "as_known_ns": None}, "ready-" + str(self.number))
                equal(reply["kind"], "EconomicView", "owner ready")
                return
            time.sleep(0.02)
        raise TimeoutError("owner socket 启动超时 " + domain)

    def stop(self, domain, *, kill=False, marker=None):
        proc = self.processes[domain]
        require(proc.poll() is None, "owner 已意外退出，不能当成正常停止")
        if marker is not None:
            equal(marker, {"domain_id": domain, "message_id": "step-014", "phase": self.phase,
                           "pid": proc.pid, "test_only": True}, "marker 完整身份与当前进程句柄")
        if kill:
            proc.kill()
        else:
            proc.terminate()
        code = proc.wait(timeout=self.seed["process_timeout_ms"] / 1000)
        if kill:
            equal(code, -signal.SIGKILL, "真实 SIGKILL 退出")
        else:
            require(code in (0, -signal.SIGTERM), "owner 非预期停止码")
        self.record({"event": "reaped", "domain": domain, "pid": proc.pid, "exit_code": code})
        del self.processes[domain]
        return {"domain": domain, "pid": proc.pid, "exit_code": code}

    def envelope(self, payload, message):
        return {"schema_revision": "economic-session/1", "session_id": self.manifest["session_id"],
                "session_generation": self.manifest["session_generation"], **SOURCE, "message_id": message,
                "payload_hash": digest(payload), "payload": payload, "causal_refs": []}

    def deliver(self, domain, request):
        return exchange(self.node(domain)["socket"], request, timeout_ms=self.seed["exchange_timeout_ms"],
                        max_frame_bytes=int(self.manifest["resources"]["max_frame_bytes"]))

    def accept(self, domain, request, delivered):
        self.record({"domain": domain, "request": request, "delivery": delivered})
        equal(delivered["transport"], "Received", "必须取得完整业务响应")
        return validate_reply(request, delivered["response"], producer=domain, producer_epoch=self.epochs[domain])

    def send(self, domain, payload, message):
        request = self.envelope(payload, message)
        if message in self.requests:
            equal(request, self.requests[message], "同消息身份内容不可改")
        self.requests[message] = request
        return self.accept(domain, request, self.deliver(domain, request))

    def view(self, domain, label):
        response = self.send(domain, {"op": "ReadView", "cut": None, "as_known_ns": None}, "view-" + label + "-" + domain)
        equal(response["kind"], "EconomicView", "正式可读视图")
        equal(response["image_hash"], digest(response["image"]), "视图内容哈希")
        equal(response["image"], image_of(read_domain(self.node(domain)["db"])), "公共读取与持久图像一致")
        return response["image"]

    def recover(self, domain):
        require(domain not in self.processes, "恢复前旧 owner 必须已回收")
        before = read_domain(self.node(domain)["db"])
        recovered = self.cli(domain, "recover")
        after = read_domain(self.node(domain)["db"])
        assert_recovery(before, after, recovered)
        self.number += 1
        save(self.output / (str(self.number) + "-recovery.json"), {"domain": domain, "before": before, "after": after, "receipt": recovered})
        return recovered

    def fault_resolve(self, payload):
        request = self.envelope(payload, "step-014")
        self.requests["step-014"] = request
        require(not self.marker.exists(), "本次故障 marker 必须不存在")
        before = read_domain(self.node("E")["db"])
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(self.deliver, "E", request)
            deadline = time.monotonic() + self.seed["marker_timeout_ms"] / 1000
            marker = None
            while marker is None:
                require(self.processes["E"].poll() is None, "E 在 marker 前退出")
                if self.marker.exists():
                    try:
                        marker = decode_frame(self.marker.read_bytes())
                    except (ValueError, UnicodeError):
                        pass  # create_new 先于完整写入；仍受同一个有界 deadline 约束。
                    if marker is not None:
                        break
                if future.done():
                    self.record({"unexpected_fault_delivery": future.result()})
                    raise ValueError("未到达具名 pause，不能算故障通过")
                if time.monotonic() >= deadline:
                    raise TimeoutError("具名 marker 等待超时")
                time.sleep(0.02)
            stopped = self.stop("E", kill=True, marker=marker)
            delivered = future.result(timeout=self.seed["exchange_timeout_ms"] / 1000 + 1)
            self.record({"domain": "E", "request": request, "delivery": delivered})
            equal(delivered["transport"], "DeliveryUnknown", "提交边界应丢失回执输送")
        killed = read_domain(self.node("E")["db"])
        key = "resolve:" + digest("r6")
        equal(key in receipts_of(killed), self.phase == "after_commit", "提交边界原收据存在性")
        if self.phase == "before_commit":
            equal(killed, before, "提交前 kill 不得残留半事务")
        else:
            equal(len(killed["commits"]["rows"]), len(before["commits"]["rows"]) + 1, "提交后只增加一次 Resolve")
            equal(image_of(killed)["pending"], [], "提交后 r6 清覆盖集合")
        save(self.output / "fault.json", {"marker": marker, "kill": stopped, "before": before, "after_kill": killed})
        # B 的 --e-epoch 是 serve 时绑定；两 owner 都实际 recover，后续时钟同处恢复后的区间。
        self.stop("B:chong-R1")
        self.recover("E")
        self.recover("B:chong-R1")
        self.start("E")
        self.start("B:chong-R1")
        reply = self.send("E", payload, "step-014")
        equal(reply["kind"], "Resolved", "原身份重送 Resolve")
        if self.phase == "after_commit":
            equal(reply, receipts_of(killed)[key], "提交后重送回原回执")
            equal(read_domain(self.node("E")["db"])["commits"], killed["commits"], "提交后重送不加提交")
        return reply

    def payload(self, step):
        if step["op"] == "resolve":
            doc = decode_frame((FIXTURES / "raw/chong-R1.json").read_bytes())
            covered = []
            for item in step["covered"]:
                ev = event_at(doc, item["pointer"])
                key = identity(ev)
                covered.append({"fact_key": key,
                                "fact_hash": self.recorded[key]["fact_hash"] if item["fact_hash"] == "recorded" else "0" * 64,
                                "token": item["token"] or "t-" + ev["source"]["external_fact_id"]})
            return "E", {"op": "ResolveEvidence", "evidence_id": step["evidence_id"], "covered": covered,
                         "frontier": {"source_namespace": SOURCE["source_namespace"], "source_epoch": SOURCE["source_epoch"],
                                      "cursor": str(step["frontier_cursor"])}, "dependency_version": "testonly-deps/1"}
        raw = (FIXTURES / step["raw"]).read_text()
        ev = event_at(decode_frame(raw.encode()), step["pointer"])
        if step["op"] == "capture":
            return "E", {"op": "CaptureExternal", "raw_utf8": raw, "json_pointer": step["pointer"],
                         "token": "t-" + ev["source"]["external_fact_id"], "source_event_time": ev["source"]["source_event_time"]}
        key = identity(ev)
        return "B:chong-R1", {"op": "ApplyAllocation", "fact_key": key, "allocation_id": ev["allocation_id"],
                              "expected_fact_hash": self.recorded[key]["fact_hash"]}

    @staticmethod
    def outcome(reply):
        return reply["error"].split("：", 1)[0] if reply["kind"] == "Error" else reply["kind"]

    def execute(self):
        for domain in DOMAINS:
            self.cli(domain, "init")
            self.start(domain)
        for index, step in enumerate(self.case["steps"]):
            label = "step-" + str(index).zfill(3)
            domain, payload = self.payload(step)
            before = read_domain(self.node(domain)["db"])
            reply = self.fault_resolve(payload) if self.phase and index == 14 else self.send(domain, payload, label)
            equal(self.outcome(reply), step["outcome"], "具名步骤 " + label)
            if step["outcome"] not in ("Applied", "Recorded", "Resolved", "Unmatched"):
                equal(read_domain(self.node(domain)["db"]), before, "拒绝/等待不得持久收据或部分状态 " + label)
            if step["op"] == "capture" and step["outcome"] == "Recorded":
                ev = event_at(decode_frame(payload["raw_utf8"].encode()), step["pointer"])
                key = identity(ev)
                equal(reply["fact_key"], key, "Capture 返回事实身份")
                equal(reply["fact_hash"], fact_hash(payload["raw_utf8"], step["pointer"]), "Capture 返回内容身份")
                self.recorded[key] = reply
            self.expected.advance(step)
            observed = self.expected.check(self.view("E", label), self.view("B:chong-R1", label))
            self.expected.check_receipts(read_domain(self.node("E")["db"]), read_domain(self.node("B:chong-R1")["db"]))
            self.samples.append(observed)
            save(self.output / (label + "-oracle.json"), observed)
            if step["outcome"] in ("Applied", "Recorded", "Resolved"):
                # 原消息精确重送：十表全部不变，不仅最终金额不变。
                state = read_domain(self.node(domain)["db"])
                equal(self.send(domain, payload, label), reply, "重复回原回执 " + label)
                equal(read_domain(self.node(domain)["db"]), state, "重复无新持久行 " + label)
            if index == 14:
                # E 已 resolve，B 仍未应用具名 fill：不可提前应用 fee，也不可写 Unmatched。
                waiting = self.case["steps"][12]
                bd, bp = self.payload(waiting)
                state = read_domain(self.node(bd)["db"])
                equal(self.outcome(self.send(bd, bp, "extra-local-fill-wait")), "AwaitingDependency", "E Resolved 不等于本 B Applied")
                equal(read_domain(self.node(bd)["db"]), state, "本地前件等待无任何持久效果")
        final = self.samples[-1]
        equal({k: final["book"][k] for k in ("q", "K", "Pi")}, {"q": 3, "K": 305, "Pi": 19}, "冻结手算终态")
        # 真实 B 重启后重送原 Applied 请求；不重复入账。
        self.stop("B:chong-R1")
        self.recover("B:chong-R1")
        self.start("B:chong-R1")
        fee_domain, fee_payload = self.payload(self.case["steps"][16])
        state = read_domain(self.node(fee_domain)["db"])
        receipt = receipts_of(state)["alloc-R-fee"]
        equal(self.send(fee_domain, fee_payload, "step-016"), receipt, "B 新进程重送原回执")
        equal(read_domain(self.node(fee_domain)["db"]), state, "B 恢复重送无新行")
        # E 整个目录移离原路径、socket owner 已回收；B 本地恢复不读 E。
        self.stop("E")
        self.stop("B:chong-R1")
        hidden = self.work / "e-offline"
        hidden.mkdir()
        for path in list(self.work.glob("e.db*")) + [self.work / "e.sock"]:
            if path.exists():
                shutil.move(str(path), str(hidden / path.name))
        require(not Path(self.node("E")["db"]).exists(), "E 数据库必须实际不可访问")
        self.recover("B:chong-R1")
        self.start("B:chong-R1")
        local = self.view("B:chong-R1", "e-offline")
        equal(self.expected.check_book(local["book"]), final["book"], "E 离线 B 本地重放")
        self.stop("B:chong-R1")
        snapshots = {"E": read_domain(hidden / "e.db"), "B:chong-R1": read_domain(self.node("B:chong-R1")["db"])}
        save(self.output / "final-tables.json", snapshots)
        save(self.output / "result.json", {"status": "PASS_C7A_TRAJECTORY", "phase": self.phase, "economic": final,
                                          "scope": "TestOnly WE-5; no FIN/2, S/gamma, browser or full #1406 acceptance"})
        return snapshots, self.samples

    def close(self):
        errors = []
        for domain in list(self.processes):
            proc = self.processes[domain]
            try:
                if proc.poll() is None:
                    proc.terminate()
                    try:
                        proc.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait(timeout=3)
                self.record({"event": "cleanup-reaped", "domain": domain, "pid": proc.pid, "exit_code": proc.returncode})
            except (OSError, subprocess.SubprocessError) as error:
                errors.append(str(error))
        for stream in self.streams:
            stream.close()
        self.log.close()
        require(not errors, "本次进程清理失败 " + repr(errors))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binaries", type=Path, required=True)
    parser.add_argument("--build-evidence", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    seed, case = load_inputs()
    binaries, output = args.binaries.resolve(), args.output.resolve()
    require(not output.exists(), "输出目录必须新建，不能覆盖证据")
    build = decode_frame(args.build_evidence.read_bytes())
    require(type(build.get("source_sha")) is str and len(build["source_sha"]) == 40
            and all(c in "0123456789abcdef" for c in build["source_sha"])
            and build.get("command") and type(build.get("exit_code")) is int and build["exit_code"] == 0,
            "须提供精确源码 SHA 和真实构建成功证据")
    checkout = subprocess.run(["git", "-C", str(ROOT.parent), "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10, check=True).stdout.strip()
    equal(checkout, build["source_sha"], "构建证据须属于当前精确源码")
    for name in ("e_economic_session", "b_chong_session"):
        equal(sha(binaries / name), build["binaries"][name], "构建二进制 SHA " + name)
    output.mkdir(parents=True)
    save(output / "provenance.json", {"seed": {"path": str(SEED), "sha256": sha(SEED)}, "bindings": seed["bindings"],
                                      "build": build, "build_evidence_sha256": sha(args.build_evidence), "driver_sha256": sha(__file__)})
    work = output / "runtime"
    previous_samples = None
    try:
        for phase in (None, "before_commit", "after_commit"):
            previous_tables = None
            for repeat in (1, 2):
                name = (phase or "normal") + "-" + str(repeat)
                run = Run(seed, case, work, output / name, binaries, phase)
                try:
                    tables, samples = run.execute()
                finally:
                    run.close()
                if previous_tables is not None:
                    equal(tables, previous_tables, "同输入新进程双跑十表逐字一致 " + str(phase))
                if previous_samples is not None:
                    equal(samples, previous_samples, "故障与无故障全部步骤经济/因果状态一致")
                previous_tables, previous_samples = tables, samples
                shutil.move(str(work), str(output / name / "runtime-archive"))
        save(output / "RESULT.json", {"status": "PASS_C7A", "trajectories": 6,
                                     "full_table_double_run": True, "all_step_oracle": True,
                                     "actual_resolve_kills": 4, "scope": "TestOnly WE-5 only"})
        return 0
    except Exception as error:
        save(output / "RESULT.json", {"status": "FAIL", "error": str(error), "scope": "C7a"})
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
