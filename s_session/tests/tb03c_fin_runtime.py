#!/usr/bin/env python3
"""#1673（TB-03-C/S5，#1406 AC1/AC4 后端）：正式会话入口的 FIN/1 + tb03c-book/1 合链运行。

实际启动 s_structure_session（经 s_service_control）、e_economic_session、b_chong_session 与 Q
（s_readonly_server --economic-config），经 Unix socket 走完 #1406 S1 同一链
Prepare → Reserve → Commit → Capture → Resolve → Apply，再经 Q 公共 HTTP 的
/api/economic/snapshot 与 /watch 核对新对象、逐 cut 变化和证据引用。

信任边界（写进 RESULT，不得改述为已成立）：
- S 是真实进程与真实 tb01c 冻结输入；计划的 s_ref 绑定本次 financial-basis 实读的真实结构对象。
- Commit 与成交的对应仍由夹具构造（成交预列于冻结 FIN baseline、command_id 由夹具填为 plan-alpha），
  产品绑定 F1/F2 归 TB-04-F #1425，本驱动不认定其已存在。
- Voice/γ 授权是单位信任夹具（与 S1 同式），不是真实 γ 独评。
- 资金数值全部 TestOnly；θ/增长/本金政策不签、不填默认（ADR 0041 裁定二），只观测 NotEvaluable。
冻结的 tb03c_query_runtime.py 与既有 tb03c* 夹具不读不改；本驱动只新增 fixtures/tb03c_fin_runtime/。
"""
import argparse
import base64
from contextlib import closing
import copy
import hashlib
import io
import json
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.request

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT))
from s_socket_client import canonical, decode_frame, digest, exchange, read_economic_pages, validate_reply

FIX = HERE / "fixtures" / "tb03c_fin_runtime"
TB01C = HERE / "fixtures" / "tb01c"
FIN1, BOOK1 = "tb03b-finance/1", "tb03c-book/1"
FILL_POLICY_ID = "frozen-manifest-proportional-cumulative/2"  # allocation::FILL_ALLOCATION_POLICY_ID
# 冻结输入的原字节身份（相对 s_session/）；运行不得改写来源换 oracle。
INPUT_HASHES = {
    "tests/fixtures/tb03c_fin_runtime/OPENING-BASE.json": "9cd91d4f174db5293ad5f85925c7e6ac735f9a32eac7d92eb8b394927d694fda",
    "tests/fixtures/tb03c_fin_runtime/HAND-ORACLE.json": "68fc52b4ec4fc387bd7571a64acf2a0375e844cf8f0e8ebbd5ac0c07eb99ed6a",
    "tests/fixtures/tb01c/messages.jsonl": "29a202bf4a94e69ed503db54e1e30c52d052616447bbd1c4f37488169cc8ac58",
    "tests/fixtures/tb01c/clock-plan.json": "3d7d0470f9e6e358e645b3e757b85ed212d45bb0024f9a90b6919bebc4001709",
    "tests/fixtures/tb01c/profile.json": "2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc",
    "tests/fixtures/tb01c/query-resources.json": "36b188ae42be8e24e7fdbb44e7d3a7068200f7c942f50975337d07595ac62e64",
    "tests/fixtures/tb01c/runtime-plan.json": "18e9b3cd6cb5f17ba715b09405995b68b627bcd696ba457de790dfa9a51125c3",
    "catalog/signed-catalog.json": "938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069",
}
INPUT_NAMES = tuple(INPUT_HASHES)
# 与 S1 同一业务源；Q 使用独立只读源。
SOURCE = {"source_namespace": "test", "source_epoch": "one", "producer_id": "unit", "producer_epoch": "1"}
QUERY_SOURCE = {"source_namespace": "testonly-tb1673-query", "source_epoch": "fixture-epoch-1",
                "producer_id": "tb1673-query", "producer_epoch": "1"}
# Q 单帧 watch 携带每个变化的完整图像（含嵌入的结构快照与历史），预算放到与 owner 帧界同为 16MB。
QUERY_RESOURCES = {"max_frame_bytes": "16777216", "timeout_ms": "30000", "operation_timeout_ms": "30000",
                   "max_reply_bytes": "16777216", "max_cached_bytes": "67108864", "max_snapshots": "4",
                   "max_domains": "4", "max_changes": "32"}
WRITE_OPS = ("ImportResponsibility", "CaptureExternal", "ResolveEvidence", "ApplyAllocation", "BindVoice",
             "PrepareDecision", "Reserve", "CommitDecision")
QUERY_OPS = ("ReadView", "Watch", "QueryDecision", "QueryGrant", "EconomicReadOpen", "EconomicReadPage", "EconomicReadClose")
READ_OPS = ("ReadView", "Watch", "QueryFact", "QueryAllocation", "QueryDecision", "QueryGrant", "QueryVoice")
# E/B 时钟原点：严格晚于 tb01c S 时钟计划全部时刻（knowledge_floor：获知不早于实际 S 提交）。
T0 = 946687000000000000
CLOCK_MESSAGES, RECOVER_EPOCHS = 96, 16
# tb01c seed_main_schedule 的最短前缀：第 3 条输入后真实 S 首次出现结构对象（远端探针实测：
# 2 条 0 对象；3 条 1 对象、financial-basis 约 8KB；95 条约 0.5MB）。B 每个决策记录与历史项都整份嵌入
# 结构快照，0.5MB 级快照会让 owner 图像越过 16MB 帧界，故只喂到首个对象出现为止。
S_OPERATIONS = 3
DOMAINS = ("E", "B:alpha")
# 断言集合：每个场景必须全部实际执行，契约测试锁定。
CHECKS = {
    "covered": (
        "s.ingest_committed", "s.basis_identity", "s.basis_not_pending", "s.object_bound",
        "owner.init", "owner.ready", "q.ready_revisions", "q.initial_snapshot",
        "open.capture", "open.apply", "open.voice", "fin.import", "oracle.k_before",
        "fin.prepare", "oracle.lock", "oracle.worst", "fin.reserve", "oracle.total", "fin.grant",
        "fin.commit", "fin.commit_no_position", "fin.total_after_commit",
        "gap.capture", "gap.resolve_rejected", "gap.state_unchanged", "gap.apply_awaiting", "gap.b_unchanged",
        "fill.capture", "fill.capture_resend", "fill.capture_exact_resend", "fill.conflict_rejected",
        "fill.conflict_unchanged", "fill.apply_awaiting", "fill.b_unchanged",
        "resolve.covered", "resolve.resend", "resolve.other_rejected", "resolve.total_unchanged",
        "resolve.allocation_rows", "oracle.allocation", "resolve.command_fixture_passthrough", "resolve.gap_pending",
        "apply.wrong_hash", "apply.wrong_hash_unchanged", "apply.applied", "apply.resend", "apply.once",
        "oracle.units_after", "oracle.k_after", "book.fixed_q", "book.principal_not_evaluable",
        "book.growth_not_evaluable", "oracle.cost_remaining", "book.voice_of_decision", "book.fact_allocation",
        "fin.total_final", "valuation.capture", "valuation.rejected", "valuation.unchanged", "oracle.k_after_valuation",
        "q.snapshot_matches_owner", "q.watch_contiguous", "q.watch_matches_store", "q.watch_final_image",
        "q.commit_receipt", "q.commit_e_cut", "q.receipt_cuts", "q.s_ref", "q.fill_allocation", "q.oracle_book",
        "q.original_decision", "reopen.recovered", "reopen.public_equal", "q.restart_equal",
        "reopen.apply_resend", "reopen.commit_resend", "reopen.capture_resend", "reopen.no_new_commit",
    ),
    "uncovered": (
        "owner.init", "owner.ready", "open.capture", "open.apply", "open.voice", "fin.import",
        "fin.prepare", "fin.reserve", "fin.commit", "fill.capture", "resolve.uncovered_rejected",
        "resolve.uncovered_unchanged", "fin.total_after_commit",
    ),
}


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def save(path, value):
    with Path(path).open("xb") as stream:
        stream.write(canonical(value) + b"\n")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_inputs():
    """原字节锁定后才解码；任何漂移都在启动进程前失败。"""
    for name in INPUT_NAMES:
        require(sha(ROOT / name) == INPUT_HASHES[name], "冻结输入 hash 不符 " + name)
    messages = [decode_frame(line.encode()) for line in (TB01C / "messages.jsonl").read_text().splitlines()]
    clock = decode_frame((TB01C / "clock-plan.json").read_bytes())
    floor = max(int(v) for event in clock["events"].values() for v in _numbers(event))
    require(T0 > floor, "E/B 时钟原点须晚于全部 S 时刻")
    plan = decode_frame((TB01C / "runtime-plan.json").read_bytes())
    require(plan["seed_main_schedule"][0] == 0 and S_OPERATIONS - 1 <= plan["seed_main_schedule"][1], "S 输入须为 seed_main_schedule 原前缀")
    return {"opening": decode_frame((FIX / "OPENING-BASE.json").read_bytes()),
            "oracle": decode_frame((FIX / "HAND-ORACLE.json").read_bytes()),
            "messages": messages[:S_OPERATIONS],
            "clock": clock,
            "plan": plan}


def _numbers(value):
    if type(value) is dict:
        for item in value.values():
            yield from _numbers(item)
    elif type(value) is list:
        for item in value:
            yield from _numbers(item)
    elif type(value) is str and value.isdigit():
        yield value


def hand_oracle(inputs):
    """独立手算（与 S1-oracle.py 同式）；不读任何产品输出。"""
    i = inputs
    lock = i["quantity"] * i["limit_price"] + i["fee_tail"]
    require(lock <= i["right_cash"], "锁定超过私有现金权利")
    total = i["baseline"] + lock
    require(total <= i["cap"], "共同界装不下")
    worst = i["opening_units"] + i["quantity"]
    require(worst <= i["fixed_q"], "最坏数量越过 fixed_q")
    units = i["fill_to"] - i["fill_from"]
    require(units == i["quantity"] and i["fill_price"] <= i["limit_price"] and i["fill_commission"] <= i["fee_tail"],
            "成交须在计划锁定之内")
    cash = units * i["fill_price"]
    k_after = i["opening_cash"] + cash + i["fill_commission"]
    return {"lock": lock, "responsibility_total": total, "worst_quantity": worst,
            "allocation": {"units": units, "acquisition_cash": cash, "commission": i["fill_commission"]},
            "K_before": i["opening_cash"], "K_after": k_after, "units_after": i["opening_units"] + units,
            "K_after_valuation": k_after, "cost_remaining": k_after}


def read_tables(path):
    """一次只读快照，保留全部表原始行；BLOB 以十六进制保存。"""
    with closing(sqlite3.connect(Path(path).resolve().as_uri() + "?mode=ro", uri=True, timeout=5)) as db:
        db.execute("PRAGMA query_only=ON")
        db.execute("BEGIN")
        result = {}
        for (table,) in db.execute("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name").fetchall():
            columns = [row[1] for row in db.execute('PRAGMA table_info("' + table + '")')]
            rows = [[v.hex() if type(v) is bytes else v for v in row]
                    for row in db.execute('SELECT * FROM "' + table + '" ORDER BY 1')]
            result[table] = {"columns": columns, "rows": rows}
        db.rollback()
    return result


def commits_of(tables):
    columns = tables["commits"]["columns"]
    return [{k: row[columns.index(k)] for k in columns} for row in tables["commits"]["rows"]]


# ---- 原件构造：与 tb03c_chain_tests.rs 逐字同式 ----
def source(fact, cursor):
    return {"source_namespace": "test", "source_epoch": "one", "external_fact_id": fact, "source_cursor": cursor,
            "received_time": "source-" + cursor, "claimed_origin": "TestOnly S1", "source_event_time": None}


def units_doc(base, events, fragments):
    return {"format": base["format"], "units": base["units"], "events": events, "evidence_index": {"fragments": fragments}}


def fill_doc(base, fact, cursor, seq, start, end, price, manifest_hash):
    return units_doc(base, [{"event": fact, "allocation_id": fact, "source": source(fact, cursor),
        "stable_point": {"profile": "TestOnly", "seq": seq, "source_ref": "sp"},
        "netted_fill": {"manifest_hash": manifest_hash, "symbol": "alpha", "report_kind": "incremental",
                        "reported_quantity": str(int(end) - int(start)), "from": start, "to": end, "unit_price": price,
                        "commission": "2", "cash_kind": "acquisition_cash",
                        "income_changes": {"Pi": "0", "A": "0", "W": "0"}, "source_ref": "fill"}}],
        {"sp": [{"json_pointer": "/events/0/stable_point"}], "fill": [{"json_pointer": "/events/0/netted_fill"}]})


def fact_key(event):
    s = event["source"]
    return digest([s["source_namespace"], s["source_epoch"], s["external_fact_id"]])


def fact_hash(doc):
    return digest({"raw_sha256": hashlib.sha256(canonical(doc)).hexdigest(), "json_pointer": "/events/0",
                   "event": doc["events"][0], "units": doc["units"]})


def covered_of(doc, token):
    return {"fact_key": fact_key(doc["events"][0]), "fact_hash": fact_hash(doc), "token": token}


def documents(base):
    opening = copy.deepcopy(base)
    event = opening["events"][0]
    event["book_identity"] = {"chong_id": "alpha", "symbol": "alpha", "op_level": "2", "fixed_q": "12", "side": "long"}
    event["observed_existing_book"]["units"] = "4"
    event["source"]["external_fact_id"] = "opening-alpha"
    event["allocation_id"] = "allocation-alpha"
    event["stable_point"] = {"profile": "TestOnly", "seq": "1", "source_ref": "s1-sp"}
    opening["evidence_index"]["fragments"]["s1-sp"] = [{"json_pointer": "/events/0/stable_point"}]
    family = event["family_id"]
    manifest = {"projection_kind": "netted", "symbol": "alpha", "participants": [{"chong_id": "alpha",
                "command_id": "plan-alpha", "family_id": family, "revision": "1", "requested_q": "2"}],
                "tie_break_order": ["alpha"]}
    manifest["manifest_hash"] = digest(manifest)
    mh = manifest["manifest_hash"]
    valuation = units_doc(base, [{"event": "s1-val", "allocation_id": "s1-val", "chong_id": "alpha", "family_id": family,
        "component_witnesses": [{"fields": ["Pi", "A", "W"], "kind": "no_other_income_book_changes_in_this_allocation",
                                 "source_ref": "w0"}],
        "deltas": {"A": "0", "Pi": "0", "W": "0", "units": "0"}, "known_cost_components": [],
        "source": source("s1-val", "5"), "stable_point": {"profile": "TestOnly", "seq": "4", "source_ref": "sp"},
        "valuation_observation": {"reference_price_before": "14", "reference_price_after": "9", "source_ref": "val"}}],
        {"sp": [{"json_pointer": "/events/0/stable_point"}], "w0": [{"json_pointer": "/events/0/component_witnesses/0"}],
         "val": [{"json_pointer": "/events/0/valuation_observation"}]})
    return {"opening": opening, "fill_manifest": manifest,
            "fill": fill_doc(base, "s1-fill", "3", "2", "0", "2", "13", mh),
            "gap": fill_doc(base, "s1-gap", "4", "3", "2", "3", "13", mh),
            "conflict": fill_doc(base, "s1-fill", "3", "2", "0", "2", "12", mh),
            "valuation": valuation}


def clock_plan():
    messages = {}
    for i in range(CLOCK_MESSAGES):
        t = 1000 + 10 * i
        times = lambda offset: [str(T0 + r * 1000000 + t + offset) for r in range(RECOVER_EPOCHS)]
        messages["m-" + str(i)] = {"received_ns": times(0), "first_known_ns": times(1), "commit_ns": times(2)}
    return {"init_ns": str(T0 + 1), "recover_ns": [str(T0 + r * 1000000) for r in range(1, RECOVER_EPOCHS + 1)],
            "messages": messages}


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as held:
        held.bind(("127.0.0.1", 0))  # DevSkim: ignore DS162092 有意使用本机回环（本地服务/测试），非调试残留
        return str(held.getsockname()[1])


class Checks:
    def __init__(self, scenario):
        self.scenario, self.passed = scenario, []

    def ok(self, name, condition, detail=None):
        require(name in CHECKS[self.scenario], "未登记断言 " + name)
        if not condition:
            raise AssertionError(self.scenario + "/" + name + ("" if detail is None else "：" + json.dumps(
                detail, ensure_ascii=False, default=repr)[:4000]))
        if name not in self.passed:
            self.passed.append(name)

    def complete(self):
        missing = [n for n in CHECKS[self.scenario] if n not in self.passed]
        require(not missing, self.scenario + " 断言未全部执行 " + repr(missing))


class Stack:
    """一次运行的全部进程、私有短路径工作区（socket < 100 字节）与证据目录。"""

    def __init__(self, inputs, binaries, output):
        self.inputs, self.binaries, self.output = inputs, binaries, output
        self.work = Path(tempfile.mkdtemp(prefix="f1673-", dir="/tmp"))
        output.mkdir(parents=True)
        self.log = (output / "commands.jsonl").open("x")
        self.number, self.streams, self.processes = 0, [], {}
        self.s_started = self.q_started = False
        self.q_revision = 0

    def record(self, entry):
        self.log.write(json.dumps(entry, ensure_ascii=False) + "\n")
        self.log.flush()

    def label(self, name):
        self.number += 1
        return str(self.number).zfill(4) + "-" + name

    def command(self, args, name, *, timeout=120):
        label = self.label(name)
        out_path, err_path = self.output / (label + ".stdout"), self.output / (label + ".stderr")
        with out_path.open("xb") as out, err_path.open("xb") as err:
            done = subprocess.run(args, stdout=out, stderr=err, timeout=timeout)
        self.record({"event": "command", "command": args, "exit_code": done.returncode,
                     "stdout": {"path": out_path.name, "sha256": sha(out_path)},
                     "stderr": {"path": err_path.name, "sha256": sha(err_path)}})
        require(done.returncode == 0, "正式命令失败 " + label)
        return decode_frame(out_path.read_bytes())

    # ---- S/Q：经正式 s_service_control 控制 ----
    def s_control(self, action):
        return self.command([sys.executable, str(ROOT / "s_service_control.py"), action, "--config",
                             str(self.s_config), "--state-dir", str(self.work / "control-s")], "s-" + action)

    def start_s(self, checks):
        plan, first = self.inputs["plan"], self.inputs["messages"][0]
        self.s = {"schema_revision": "s-launcher/2",
                  **{k: first[k] for k in ("session_id", "session_generation", "source_namespace", "source_epoch",
                                           "producer_id", "producer_epoch")},
                  "writer_epoch": "1", "query_epoch": "1", "delivery_retain_generations": str(plan["delivery_retain_generations"]),
                  "port": free_port(), "startup_timeout_ms": str(plan["startup_timeout_ms"]),
                  "db": str(self.work / "s.db"), "socket": str(self.work / "s.sock"),
                  "binary": str(self.binaries / "s_structure_session"), "python": sys.executable,
                  "catalog": str(ROOT / "catalog/signed-catalog.json"), "profile": str(TB01C / "profile.json"),
                  "clock_plan": str(TB01C / "clock-plan.json"), "browser": str(ROOT / "browser/index.html"),
                  "query_resource_config": str(TB01C / "query-resources.json"), "s_resources": plan["s_resources"]}
        self.s_config = self.work / "s-launcher.json"
        save(self.s_config, self.s)
        save(self.output / "s-launcher.json", self.s)
        self.s_started = True
        save(self.output / "s-start.json", self.s_control("start-s"))
        rows = []
        for request in self.inputs["messages"]:
            result = exchange(self.s["socket"], request, timeout_ms=int(plan["input_reply_timeout_ms"]),
                              max_frame_bytes=int(self.s["s_resources"]["max_frame_bytes"]))
            require(result["transport"] == "Received", "S 正式输入 DeliveryUnknown " + request["message_id"])
            reply = validate_reply(request, result["response"], producer="S", producer_epoch=self.s["writer_epoch"])
            rows.append({"message_id": request["message_id"], "request_payload_hash": request["payload_hash"],
                         "kind": reply.get("kind"), "reply_sha256": digest(reply)})
            checks.ok("s.ingest_committed", reply.get("kind") == "Committed", rows[-1])
        save(self.output / "s-inputs.json", rows)
        basis = self.command([self.s["binary"], "financial-basis", "--db", self.s["db"]], "s-financial-basis")
        save(self.output / "s-basis.json", basis)
        checks.ok("s.basis_identity", (basis["kind"], basis["session_id"], basis["session_generation"],
                  basis["writer_epoch"]) == ("FinancialBasis", self.s["session_id"], self.s["session_generation"],
                                             self.s["writer_epoch"]), {k: basis.get(k) for k in ("kind", "session_id")})
        checks.ok("s.basis_not_pending", basis["pending_revision"] is False and int(basis["commit_ns"]) < T0)
        objects = basis["snapshot"]["objects"]
        require(type(objects) is list and objects, "真实 S 尚无结构对象，不能绑定计划")
        obj = objects[0]
        self.s_ref = {"object_id": obj["object_id"], "object_hash": digest(obj), "profile_id": basis["snapshot"]["profile_id"]}
        checks.ok("s.object_bound", type(obj["object_id"]) is str and obj["object_id"], self.s_ref)
        self.basis, self.s_object = basis, obj
        return basis

    def start_q(self, economy, expected_epochs):
        self.q_revision += 1
        config = {"schema_revision": "economic-query/1", "manifest": str(economy.manifest_path),
                  "manifest_hash": digest(economy.manifest), "query_source": QUERY_SOURCE, "producer_epoch": "1",
                  "expected_epochs": dict(expected_epochs), "resources": QUERY_RESOURCES}
        path = self.work / ("economic-query-r" + str(self.q_revision) + ".json")
        save(path, config)
        save(self.output / path.name, config)
        self.s["economic_query_config"] = str(path)
        self.s_config = self.work / ("s-launcher-r" + str(self.q_revision) + ".json")
        save(self.s_config, self.s)
        self.q_started = True
        save(self.output / ("q-start-r" + str(self.q_revision) + ".json"), self.s_control("start-q"))
        self.q_manifest_hash = config["manifest_hash"]

    def stop_q(self):
        self.s_control("stop-q")
        self.q_started = False

    def http(self, economy, path, payload=None):
        url = "http://127.0.0.1:" + self.s["port"] + path  # DevSkim: ignore DS162092 有意使用本机回环（本地服务/测试），非调试残留
        label = self.label("q" + path.replace("/", "-"))
        if payload is None:
            with urllib.request.urlopen(url, timeout=30) as response:
                body = decode_frame(response.read(int(QUERY_RESOURCES["max_reply_bytes"]) + 1))
            save(self.output / (label + ".response.json"), body)
            return body
        request = {"schema_revision": "economic-session/1", "session_id": economy.manifest["session_id"],
                   "session_generation": economy.manifest["session_generation"], **QUERY_SOURCE,
                   "message_id": label, "payload_hash": digest(payload), "causal_refs": [], "payload": payload}
        http_request = urllib.request.Request(url, data=canonical(request), headers={"Content-Type": "application/json"})
        save(self.output / (label + ".request.json"), request)
        try:
            with urllib.request.urlopen(http_request, timeout=60) as response:
                raw = response.read(int(QUERY_RESOURCES["max_reply_bytes"]) + 1)
        except urllib.error.HTTPError as error:
            body = error.read(65536)
            (self.output / (label + ".http-error")).write_bytes(body)
            raise RuntimeError("Q HTTP " + str(error.code) + " " + path + "：" + body.decode("utf-8", "replace")[:2000]) from None
        reply = decode_frame(raw)
        save(self.output / (label + ".response.json"), reply)
        return validate_reply(request, reply, producer="Q", producer_epoch="1")

    def paged(self, economy, query):
        """Q 公共 HTTP 单帧硬帽 1MB（economic_http.py:116）；完整 snapshot/watch 走正式分页 /api/economic/read。"""
        deadline = time.monotonic() + 30
        value = read_economic_pages(query, lambda payload, _remaining: self.http(economy, "/api/economic/read", payload),
            session_id=economy.manifest["session_id"], session_generation=economy.manifest["session_generation"],
            producer_id="Q:" + economy.manifest["session_id"], producer_epoch="1",
            manifest_hash=digest(economy.manifest), owner_epochs=dict(economy.epochs), deadline=deadline,
            max_frame_bytes=1048576, max_document_bytes=4194304)
        save(self.output / (self.label("q-paged-" + query["op"]) + ".document.json"), value)
        return value

    def close(self):
        errors = []
        for name in ("q", "s"):
            if getattr(self, name + "_started"):
                try:
                    self.s_control("stop-" + name)
                    setattr(self, name + "_started", False)
                except (OSError, ValueError, subprocess.SubprocessError) as error:
                    errors.append(name + ": " + str(error))
        for stream in self.streams:
            stream.close()
        self.log.close()
        return errors


class Economy:
    """一个 FIN/1 + tb03c-book/1 经济域（E + B:alpha），同 S1 夹具；s_ref 绑定真实 S 对象。"""

    def __init__(self, stack, name, cover):
        self.stack, self.name, self.cover = stack, name, cover
        self.checks = Checks(name)
        self.work, self.output = stack.work / name, stack.output / name
        self.work.mkdir()
        self.output.mkdir()
        self.processes, self.epochs, self.next = {}, {}, 0
        self.docs = documents(stack.inputs["opening"])
        d = self.docs
        opening_cov = covered_of(d["opening"], "token-opening")
        opening_event = d["opening"]["events"][0]
        # ---- Voice 授权单位信任夹具（非真实 γ 独评，与 S1 同式） ----
        fk, fh = fact_key(opening_event), opening_cov["fact_hash"]
        raw_hash = hashlib.sha256(canonical(d["opening"])).hexdigest()
        reference = lambda oid: {"authority_id": "alpha", "object_id": oid, "revision": "0", "content_hash": digest(oid)}
        self.binding = {"op": "BindVoice", "binding_id": "voice-alpha", "evidence_id": "alpha",
                        "carrier_ref": reference("unit-carrier"), "gamma_ref": reference("unit-gamma"),
                        "parent_voice_id": None, "sigma": "Long", "n": "0",
                        "opening_basis": {"fact_key": fk, "expected_fact_hash": fh, "allocation_id": "allocation-alpha"}}
        trusted = {"evidence_id": "alpha", "instrument": "alpha", "chong_id": "alpha", "operation_level": "2",
                   "carrier_ref": self.binding["carrier_ref"], "gamma_ref": self.binding["gamma_ref"], "sigma": "Long",
                   "n": "0", "parent_voice_id": None,
                   "structure_view": {"profile_id": "unit", "execution_level": "2", "confirmation_level": "3", "nest_depth": "1"},
                   "opening_source": {"fact_key": fk, "fact_hash": fh, "allocation_id": "allocation-alpha",
                                      "raw_sha256": raw_hash, "json_pointer": "/events/0", "quantity": "4",
                                      "units": d["opening"]["units"]}}
        artifacts = []
        for artifact in ("input", "controlled_gammas", "carrier_objects", "instrument_declaration", "opening_raw"):
            value = d["opening"] if artifact == "opening_raw" else {"scope": "unit trust root only, not formal gamma proof", "name": artifact}
            path = self.work / ("alpha-" + artifact + ".json")
            path.write_bytes(canonical(value))
            artifacts.append({"name": artifact, "path": str(path), "sha256": sha(path), "bytes": str(len(canonical(value)))})
        bundle = {"schema": "voice-binding-authority/1", "authority_id": "alpha",
                  "source": {"instrument": "alpha", "input_sha256": artifacts[0]["sha256"], "profile_id": "unit",
                             "first_confirming_observation_source": "1",
                             "clock_mapping": {"kind": "TestOnlyAffineOrdinalToNs", "source_origin": "0",
                                               "nanosecond_origin": "0", "nanoseconds_per_ordinal": "1"}},
                  "artifacts": artifacts, "bindings": [trusted]}
        bundle_path = self.work / "alpha-authority.json"
        bundle_path.write_bytes(canonical(bundle))
        review_path = self.work / "alpha-unit-review.json"
        review_path.write_bytes(canonical({"schema": "voice-binding-independent-verification/1",
            "verdict": "APPROVE_CONTROLLED_VOICE_BINDING", "authority_id": "alpha", "bundle_sha256": sha(bundle_path),
            "artifacts": artifacts, "verified_evidence_ids": ["alpha"], "scope": "unit trust fixture only; no independent acceptance"}))
        # ---- FIN/1 冻结输入：s_ref 为本次真实 S 对象；其余与 S1 同值（policy 截止平移到 T0 时间轴） ----
        s_ref = stack.s_ref
        self.plan = {"decision_id": "alpha", "attempt_id": "attempt-alpha", "plan_id": "plan-alpha", "owner": "B:alpha",
                     "voice_binding_id": "voice-alpha", "right_id": "alpha", "expected_book_version": "2",
                     "expected_rights_version": "1", "applicant_epoch": "1", "applicant_generation": "1", "slot_id": "slot",
                     "quantity": "2", "limit_price": "14", "fee_tail": "2", "quantity_source": "TestOnly-quantity/alpha",
                     "price_source": "TestOnly-price/alpha", "fee_source": "TestOnly-tail/alpha", "s_ref": s_ref,
                     "o_ref": {"operation_id": "TestOnly-operation-alpha", "kind": "TestOnlyCashBuy",
                               "source_ref": "TestOnly-operation-sheet/alpha", "s_object_id": s_ref["object_id"], "quantity": "2"}}
        applicants = ["alpha"]
        covered = [opening_cov, covered_of(d["fill"], "token-s1-fill"), covered_of(d["gap"], "token-s1-gap")] if cover else [opening_cov]
        self.input = {"schema_revision": FIN1, "kind": "TestOnly", "source_id": "s1-fixture", "source_epoch": "1",
            "policy": {"policy_id": "TestOnly-cash-cap-100", "allocation_policy_id": "#1349/atomic-slot-wait/v1", "cap": "100",
                       "currency": "TEST-CASH", "money_unit": "TEST-CASH", "quantity_unit": "TEST-UNIT", "quantum": "1",
                       "capability_id": "TestOnly-no-leverage-no-venue", "model": "TestOnlyUnleveragedCashBuy",
                       "valid_through_ns": str(T0 + 1000000000)},
            "baseline": {"responsibility_id": "existing20", "amount": "20",
                         "source_ref": "TestOnly-existing-full-liability-original", "covered_facts": covered},
            "rights": [{"right_id": "alpha", "owner": "B:alpha", "version": "1", "amount": "40", "currency": "TEST-CASH",
                        "source_ref": "TestOnly-privately-owned-cash-original/alpha"}],
            "plans": [self.plan],
            "slots": [{"slot_id": "slot", "applicants": applicants, "input_end": {"kind": "ControlledInputEnd",
                       "source_id": "s1-fixture", "source_epoch": "1", "watermark": "2", "complete_request_hash": digest(applicants)}}]}
        input_path = self.work / "fin-input.json"
        input_path.write_bytes(canonical(self.input))
        session = stack.s
        self.manifest = {"schema_revision": "economic-manifest/1", "session_id": session["session_id"],
            "session_generation": session["session_generation"], "book_revision": BOOK1,
            "e": {"db": str(self.work / "e.db"), "socket": str(self.work / "e.sock")},
            "chongs": [{"chong_id": "alpha", "symbol": "alpha", "operation_level": "2", "fixed_q": "12",
                        "db": str(self.work / "alpha.db"), "socket": str(self.work / "alpha.sock")}],
            "fill_manifests": [d["fill_manifest"]],
            "fill_allocation_policy": {"fill_allocation_policy_id": FILL_POLICY_ID, "policy_revision": "s1-fin-1"},
            "authorized_sources": [{**SOURCE, "operations": list(WRITE_OPS + READ_OPS)},
                                   {**QUERY_SOURCE, "operations": list(QUERY_OPS)}],
            # 传输预算同 tb03c_runtime SEED（30s）：Prepare 内同步调用真实 S financial-basis（debug 构建、约 0.5MB 输出），
            # S1 单元测试的 3s 桩子预算对真实进程不足。只是等待上限，不是经济默认值。
            "resources": {"max_frame_bytes": "16777216", "read_timeout_ms": "30000", "write_timeout_ms": "30000",
                          "response_timeout_ms": "30000", "queue_capacity": "4", "max_connections": "4"},
            "clock": {"kind": "TestOnly", "plan": clock_plan()},
            "voice_binding": {"schema_revision": "voice-binding/1", "authorities": [{"authority_id": "alpha",
                "bundle_path": str(bundle_path), "bundle_sha256": sha(bundle_path), "verification_path": str(review_path),
                "verification_sha256": sha(review_path), "max_bytes": "1048576"}]},
            "finance": {"schema_revision": FIN1, "input_path": str(input_path), "input_sha256": sha(input_path),
                        "coordinator_source": SOURCE,
                        "structure": {"binary": session["binary"], "binary_sha256": sha(session["binary"]),
                                      "db": session["db"], "session_id": session["session_id"],
                                      "session_generation": session["session_generation"],
                                      "writer_epoch": session["writer_epoch"]}}}
        self.manifest_path = self.work / "manifest.json"
        save(self.manifest_path, self.manifest)
        for path in (self.manifest_path, input_path, bundle_path, review_path):
            shutil.copy(path, self.output / path.name)
        self.log = (self.output / "messages.jsonl").open("x")
        stack.streams.append(self.log)

    # ---- 正式 E/B 进程 ----
    def node(self, domain):
        return self.manifest["e"] if domain == "E" else self.manifest["chongs"][0]

    def args(self, domain, op):
        binary = "e_economic_session" if domain == "E" else "b_chong_session"
        args = [str(self.stack.binaries / binary), op, "--manifest", str(self.manifest_path), "--db", self.node(domain)["db"]]
        if domain != "E":
            args += ["--chong", "alpha"]
        if op == "serve":
            args += ["--epoch", self.epochs[domain]]
            if domain != "E":
                args += ["--e-epoch", self.epochs["E"]]
        return args

    def cli(self, domain, op):
        result = self.stack.command(self.args(domain, op), self.name + "-" + domain.replace(":", "-") + "-" + op)
        self.checks.ok("owner.init" if op == "init" else "reopen.recovered",
                       (result["kind"], result["domain_id"]) == ({"init": "Initialized", "recover": "Recovered"}[op], domain), result)
        self.epochs[domain] = result["writer_epoch"]
        return result

    def serve(self, domain):
        sock = Path(self.node(domain)["socket"])
        if sock.exists():
            require(sock.parent == self.work and sock.is_socket(), "残留路径不是本次 socket")
            sock.unlink()
        label = self.stack.label(self.name + "-" + domain.replace(":", "-") + "-serve")
        out = (self.stack.output / (label + ".stdout")).open("xb")
        err = (self.stack.output / (label + ".stderr")).open("xb")
        self.stack.streams += [out, err]
        args = self.args(domain, "serve")
        proc = subprocess.Popen(args, stdout=out, stderr=err)
        self.processes[domain] = proc
        self.stack.record({"event": "spawn", "command": args, "pid": proc.pid})
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            require(proc.poll() is None, "owner 在就绪前退出 " + domain)
            if sock.is_socket():
                view = self.read(domain, {"op": "ReadView", "cut": None, "as_known_ns": None})
                self.checks.ok("owner.ready", view["kind"] == "EconomicView", view)
                return
            time.sleep(0.02)
        raise TimeoutError("owner socket 启动超时 " + domain)

    def stop(self, domain):
        proc = self.processes.pop(domain)
        require(proc.poll() is None, "owner 已意外退出")
        proc.terminate()
        code = proc.wait(timeout=5)
        self.stack.record({"event": "reaped", "domain": domain, "pid": proc.pid, "exit_code": code})

    def close(self):
        errors = []
        for domain in list(self.processes):
            proc = self.processes.pop(domain)
            try:
                if proc.poll() is None:
                    proc.terminate()
                    try:
                        proc.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait(timeout=3)
                self.stack.record({"event": "cleanup-reaped", "domain": domain, "pid": proc.pid, "exit_code": proc.returncode})
            except (OSError, subprocess.SubprocessError) as error:
                errors.append(domain + ": " + str(error))
        return errors

    # ---- 消息 ----
    def envelope(self, payload, message):
        return {"schema_revision": "economic-session/1", "session_id": self.manifest["session_id"],
                "session_generation": self.manifest["session_generation"], **SOURCE, "message_id": message,
                "payload_hash": digest(payload), "causal_refs": [], "payload": payload}

    def deliver(self, domain, request):
        result = exchange(self.node(domain)["socket"], request, timeout_ms=60000,
                          max_frame_bytes=int(self.manifest["resources"]["max_frame_bytes"]))
        self.log.write(json.dumps({"domain": domain, "request": request, "delivery": result}, ensure_ascii=False) + "\n")
        self.log.flush()
        require(result["transport"] == "Received", "必须取得完整业务响应 " + request["message_id"])
        return validate_reply(request, result["response"], producer=domain, producer_epoch=self.epochs[domain])

    def write(self, domain, payload):
        """写入口逐条取登记时钟槽 m-i（与 S1 一致：每次新消息身份）。"""
        require(payload["op"] in WRITE_OPS and self.next < CLOCK_MESSAGES, "写消息须有登记时钟")
        request = self.envelope(payload, "m-" + str(self.next))
        self.next += 1
        return request, self.deliver(domain, request)

    def send(self, domain, payload):
        return self.write(domain, payload)[1]

    def read(self, domain, payload):
        self.next_read = getattr(self, "next_read", 0) + 1
        return self.deliver(domain, self.envelope(payload, "r-" + str(self.next_read)))

    def image(self, domain):
        return self.read(domain, {"op": "ReadView", "cut": None, "as_known_ns": None})["image"]

    def tables(self, domain):
        return read_tables(self.node(domain)["db"])

    def capture(self, doc, token):
        return self.write("E", {"op": "CaptureExternal", "raw_utf8": canonical(doc).decode(), "json_pointer": "/events/0",
                                "token": token, "source_event_time": None})

    def resolve(self, receipt, evidence, cursor):
        return self.send("E", {"op": "ResolveEvidence", "evidence_id": evidence,
            "covered": [{k: receipt[k] for k in ("fact_key", "fact_hash", "token")}],
            "frontier": {"source_namespace": "test", "source_epoch": "one", "cursor": cursor}, "dependency_version": "s1-deps/1"})

    def fin(self, op, domain):
        payload = {"op": op, "writer_epoch": self.epochs[domain]}
        if op == "ImportResponsibility":
            payload["responsibility_id"] = "existing20"
        elif op == "Reserve":
            payload.update(slot_id="slot", request_set_hash=digest(self.input["slots"][0]["applicants"]))
        else:
            payload.update(decision_id="alpha", plan_hash=digest(self.plan))
        return payload

    def start(self):
        for domain in DOMAINS:
            self.cli(domain, "init")
        for domain in DOMAINS:
            self.serve(domain)

    # ---- 共同前缀：开局 → Voice → 既有责任 → Prepare/Reserve/Commit ----
    def prefix(self, oracle):
        c, d = self.checks, self.docs
        _, opened = self.capture(d["opening"], "token-opening")
        c.ok("open.capture", opened["kind"] == "Recorded", opened)
        self.open_receipt = opened
        applied = self.send("B:alpha", {"op": "ApplyAllocation", "fact_key": opened["fact_key"],
                                        "allocation_id": "allocation-alpha", "expected_fact_hash": opened["fact_hash"]})
        c.ok("open.apply", applied["kind"] == "Applied", applied)
        self.open_applied = applied
        bound = self.send("B:alpha", self.binding)
        c.ok("open.voice", bound["kind"] == "VoiceBound", bound)
        imported = self.send("E", self.fin("ImportResponsibility", "E"))
        c.ok("fin.import", imported.get("record", {}).get("state") == "ResponsibilityImported", imported)
        if self.cover:
            c.ok("oracle.k_before", int(self.image("B:alpha")["book"]["cost_basis"]["K"]) == oracle["K_before"])
        prepared = self.send("B:alpha", self.fin("PrepareDecision", "B:alpha"))
        c.ok("fin.prepare", prepared.get("record", {}).get("state") == "Prepared", prepared)
        if self.cover:
            c.ok("oracle.lock", prepared["record"]["prepared"]["locked_amount"] == str(oracle["lock"]), prepared)
            c.ok("oracle.worst", prepared["record"]["prepared"]["worst_quantity"] == str(oracle["worst_quantity"]), prepared)
        self.prepared = prepared
        reserved = self.send("E", self.fin("Reserve", "E"))
        c.ok("fin.reserve", reserved.get("record", {}).get("state") == "Reserved", reserved)
        self.reserved = reserved
        grants = reserved["record"]["grants"]
        if self.cover:
            c.ok("oracle.total", reserved["record"]["retained_total"] == str(oracle["responsibility_total"]), reserved)
            c.ok("fin.grant", len(grants) == 1 and (grants[0]["decision_id"], grants[0]["owner"], grants[0]["amount"])
                 == ("alpha", "B:alpha", str(oracle["lock"])), grants)
        self.grant_id = grants[0]["grant_id"]
        commit = dict(self.fin("CommitDecision", "B:alpha"), grant_id=self.grant_id)
        self.commit_request, committed = self.write("B:alpha", commit)
        c.ok("fin.commit", committed["kind"] == "LocalCommitReceipt" and committed["record"]["state"] == "Committed"
             and committed["record"]["local_commit_receipt"]["grant_id"] == self.grant_id, committed)
        self.committed = committed
        if self.cover:
            c.ok("fin.commit_no_position", self.image("B:alpha")["book"]["actual_quantity"] == "4")
        c.ok("fin.total_after_commit", responsibility_total(self.image("E")) == oracle["responsibility_total"])


def responsibility_total(image):
    return sum(int(r["amount"]) for r in image["finance"]["responsibilities"])


def is_error(reply, prefix, contains=""):
    return (reply.get("kind") == "Error" and reply["error"].startswith(prefix) and contains in reply["error"])


def run_uncovered(stack, oracle):
    """F1 反证：baseline 未预列该成交时，Commit 的 grant 责任本身不足以让 Resolve 通过。"""
    e = Economy(stack, "uncovered", cover=False)
    try:
        e.start()
        e.prefix(oracle["expected"])
        _, fill = e.capture(e.docs["fill"], "token-s1-fill")
        e.checks.ok("fill.capture", fill["kind"] == "Recorded", fill)
        before = e.tables("E")
        reply = e.resolve(fill, "s1-resolve", "3")
        e.checks.ok("resolve.uncovered_rejected", is_error(reply, *oracle["error_prefixes"]["resolve_uncovered"]), reply)
        e.checks.ok("resolve.uncovered_unchanged", e.tables("E") == before)
        e.checks.complete()
        result = {"scenario": "uncovered", "status": "PASS", "checks": e.checks.passed, "resolve_reply": reply}
        save(e.output / "result.json", result)
        return result
    finally:
        errors = e.close()
        require(not errors, "uncovered 进程清理失败 " + repr(errors))


def check_store_changes(c, store_commits, base_seq, row):
    """Q 的每个变化逐值对上 owner 持久 commits 行（seq/root/commit_ns/image）。"""
    expected = [r for r in store_commits if int(r["seq"]) > base_seq]
    c.ok("q.watch_contiguous", [int(ch["cut"]["commit_seq"]) for ch in row["changes"]]
         == [int(r["seq"]) for r in expected] and len(expected) > 0, row["changes"][:1])
    for change, stored in zip(row["changes"], expected):
        c.ok("q.watch_matches_store", change["cut"]["root_hash"] == stored["root"]
             and change["commit_ns"] == stored["commit_ns"] and canonical(change["image"]) == canonical(decode_frame(stored["image"].encode()))
             and change["image_hash"] == digest(change["image"]), {"cut": change["cut"], "stored_seq": stored["seq"]})


def public(e, fill, gap, allocation_id):
    """公共查询面（ReadView 去掉随 recover 合法变化的 writer_epoch 与进程 runtime.pid），与 S1 同集合。"""
    out = []
    for domain in DOMAINS:
        view = e.read(domain, {"op": "ReadView", "cut": None, "as_known_ns": None})
        view.pop("writer_epoch", None)
        view.pop("runtime", None)
        out += [view, e.read(domain, {"op": "QueryFact", "fact_key": fill["fact_key"]})]
    out.append(e.read("E", {"op": "QueryFact", "fact_key": gap["fact_key"]}))
    out.append(e.read("E", {"op": "QueryGrant", "grant_id": e.grant_id, "cut": None, "as_known_ns": None}))
    out.append(e.read("B:alpha", {"op": "QueryDecision", "decision_id": "alpha", "cut": None, "as_known_ns": None}))
    out.append(e.read("B:alpha", {"op": "QueryAllocation", "allocation_id": allocation_id}))
    out.append(e.read("B:alpha", {"op": "QueryVoice", "binding_id": "voice-alpha"}))
    require(all(v.get("kind") != "Error" for v in out), "公共查询失败 " + json.dumps(out, ensure_ascii=False)[:2000])
    return out


def run_covered(stack, oracle, s_passed):
    x, errors = oracle["expected"], oracle["exact_errors"]
    prefixes = oracle["error_prefixes"]
    e = Economy(stack, "covered", cover=True)
    c, d = e.checks, e.docs
    c.passed += s_passed  # S 正式会话入口断言（main 内 start_s 已执行）计入同一场景
    try:
        e.start()
        stack.start_q(e, e.epochs)
        ready = stack.http(e, "/api/economic/ready")
        c.ok("q.ready_revisions", (ready.get("book_revision"), ready.get("finance_revision"), ready.get("manifest_hash"),
             sorted(ready.get("domains", []))) == (BOOK1, FIN1, digest(e.manifest), sorted(DOMAINS)), ready)
        initial = stack.paged(e, {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None})
        c.ok("q.initial_snapshot", initial["kind"] == "EconomicSnapshot" and set(initial["domains"]) == set(DOMAINS), initial)
        base_cuts = {k: v["cut"] for k, v in initial["domains"].items()}
        segments = []

        def observe():
            """分段 watch：每段从上段 next_cut 起（单段完整文档受 4MB 聚合帽），合起来须无缝覆盖全部持久提交。"""
            cuts = segments[-1]["domains"] if segments else None
            cuts = base_cuts if cuts is None else {dm: row["next_cut"] for dm, row in cuts.items()}
            segments.append(stack.paged(e, {"op": "watch", "cuts": cuts}))
        e.prefix(x)
        observe()
        # ---- 负控：累计前沿缺口成交保留未匹配 ----
        _, gap = e.capture(d["gap"], "token-s1-gap")
        c.ok("gap.capture", gap["kind"] == "Recorded", gap)
        before = e.tables("E")
        reply = e.resolve(gap, "s1-resolve-gap", "4")
        c.ok("gap.resolve_rejected", is_error(reply, *prefixes["resolve_gap"]), reply)
        c.ok("gap.state_unchanged", e.tables("E") == before)
        b_before = e.tables("B:alpha")
        reply = e.send("B:alpha", {"op": "ApplyAllocation", "fact_key": gap["fact_key"], "allocation_id": "not-resolved",
                                   "expected_fact_hash": gap["fact_hash"]})
        c.ok("gap.apply_awaiting", reply.get("kind") == "Error" and reply["error"] == errors["apply_before_resolve"], reply)
        c.ok("gap.b_unchanged", e.tables("B:alpha") == b_before)
        # ---- Capture 成交：Commit↔成交对应由夹具构造（F1/F2 归 TB-04-F #1425） ----
        capture_request, fill = e.capture(d["fill"], "token-s1-fill")
        c.ok("fill.capture", fill["kind"] == "Recorded", fill)
        c.ok("fill.capture_resend", e.capture(d["fill"], "token-s1-fill")[1] == fill)
        c.ok("fill.capture_exact_resend", e.deliver("E", capture_request) == fill)
        before = e.tables("E")
        reply = e.capture(d["conflict"], "token-s1-fill")[1]
        c.ok("fill.conflict_rejected", is_error(reply, *prefixes["capture_conflict"]), reply)
        c.ok("fill.conflict_unchanged", e.tables("E") == before)
        b_before = e.tables("B:alpha")
        reply = e.send("B:alpha", {"op": "ApplyAllocation", "fact_key": fill["fact_key"], "allocation_id": "not-resolved",
                                   "expected_fact_hash": fill["fact_hash"]})
        c.ok("fill.apply_awaiting", reply.get("kind") == "Error" and reply["error"] == errors["apply_before_resolve"], reply)
        c.ok("fill.b_unchanged", e.tables("B:alpha") == b_before)
        observe()
        # ---- Resolve：经已保留责任 covered_facts 核 ----
        resolved = e.resolve(fill, "s1-resolve", "3")
        c.ok("resolve.covered", resolved["kind"] == "Resolved", resolved)
        c.ok("resolve.resend", e.resolve(fill, "s1-resolve", "3") == resolved)
        c.ok("resolve.other_rejected", is_error(e.resolve(fill, "s1-resolve-other", "3"), *prefixes["resolve_other_evidence"]))
        e_image = e.image("E")
        c.ok("resolve.total_unchanged", responsibility_total(e_image) == x["responsibility_total"])
        rows = e_image["fill_allocations"]
        c.ok("resolve.allocation_rows", len(rows) == 1 and rows[0]["fact_key"] == fill["fact_key"]
             and len(rows[0]["allocations"]) == 1, rows)
        allocation = rows[0]["allocations"][0]
        c.ok("oracle.allocation", (allocation["chong_id"], allocation["units"], allocation["acquisition_cash"], allocation["commission"])
             == ("alpha", str(x["allocation"]["units"]), str(x["allocation"]["acquisition_cash"]), str(x["allocation"]["commission"])), allocation)
        # 夹具自洽：command_id 只是夹具值经 Resolve 原样传递，非产品绑定（F2）。
        c.ok("resolve.command_fixture_passthrough", allocation["command_id"] == e.committed["record"]["prepared"]["plan"]["plan_id"], allocation)
        c.ok("resolve.gap_pending", any(p["fact_key"] == gap["fact_key"] for p in e_image["pending"]))
        observe()
        # ---- Apply ----
        apply = {"op": "ApplyAllocation", "fact_key": fill["fact_key"], "allocation_id": allocation["allocation_id"],
                 "expected_fact_hash": fill["fact_hash"]}
        b_before = e.tables("B:alpha")
        reply = e.send("B:alpha", dict(apply, expected_fact_hash="0" * 64))
        c.ok("apply.wrong_hash", reply.get("kind") == "Error" and reply["error"] == errors["apply_wrong_hash"], reply)
        c.ok("apply.wrong_hash_unchanged", e.tables("B:alpha") == b_before)
        applied_before = len(e.image("B:alpha")["applied"])
        apply_request, fill_applied = e.write("B:alpha", apply)
        c.ok("apply.applied", fill_applied["kind"] == "Applied", fill_applied)
        c.ok("apply.resend", e.send("B:alpha", apply) == fill_applied and e.deliver("B:alpha", apply_request) == fill_applied)
        b = e.image("B:alpha")
        c.ok("apply.once", len(b["applied"]) == applied_before + 1)
        check_book(c, b, x, prefix="")
        fact = e.read("B:alpha", {"op": "QueryFact", "fact_key": fill["fact_key"]})
        c.ok("book.fact_allocation", fact["receipt"] == fill_applied and fact["fact"]["resolved_allocation"]["chong_id"] == "alpha"
             and fact["fact"]["resolved_allocation"]["command_id"] == "plan-alpha", fact)
        c.ok("fin.total_final", responsibility_total(e.image("E")) == x["responsibility_total"])
        observe()
        # ---- 估值：E 记、B 拒，不改成本 ----
        _, val = e.capture(d["valuation"], "token-s1-val")
        c.ok("valuation.capture", val["kind"] == "Recorded", val)
        b_before = e.tables("B:alpha")
        reply = e.send("B:alpha", {"op": "ApplyAllocation", "fact_key": val["fact_key"], "allocation_id": "s1-val",
                                   "expected_fact_hash": val["fact_hash"]})
        c.ok("valuation.rejected", is_error(reply, *prefixes["apply_valuation"]), reply)
        c.ok("valuation.unchanged", e.tables("B:alpha") == b_before)
        c.ok("oracle.k_after_valuation", int(e.image("B:alpha")["book"]["cost_basis"]["K"]) == x["K_after_valuation"])

        # ---- Q 公共 HTTP：snapshot/watch 对 owner 持久层与证据引用 ----
        views = {dm: e.read(dm, {"op": "ReadView", "cut": None, "as_known_ns": None}) for dm in DOMAINS}
        observe()
        for dm in DOMAINS:
            c.ok("q.watch_contiguous", all(segments[i]["domains"][dm]["base_cut"] == segments[i - 1]["domains"][dm]["next_cut"]
                 for i in range(1, len(segments))) and segments[0]["domains"][dm]["base_cut"] == base_cuts[dm])
        watch = {"domains": {dm: {"base_cut": base_cuts[dm], "next_cut": segments[-1]["domains"][dm]["next_cut"],
                                  "changes": [ch for seg in segments for ch in seg["domains"][dm]["changes"]]} for dm in DOMAINS}}
        final_cuts = {dm: row["next_cut"] for dm, row in watch["domains"].items()}
        snapshot = stack.paged(e, {"op": "snapshot", "cuts": final_cuts, "as_known_ns": None, "snapshot_token": None})
        for dm in DOMAINS:
            got = snapshot["domains"][dm]
            c.ok("q.snapshot_matches_owner", got["status"] == "Available" and got["cut"] == views[dm]["cut"]
                 and canonical(got["image"]) == canonical(views[dm]["image"]) and got["image_hash"] == views[dm]["image_hash"],
                 {"domain": dm, "q_cut": got.get("cut"), "owner_cut": views[dm]["cut"]})
            row = watch["domains"][dm]
            check_store_changes(c, commits_of(e.tables(dm)), int(base_cuts[dm]["commit_seq"]), row)
            c.ok("q.watch_final_image", row["base_cut"] == base_cuts[dm] and row["next_cut"] == got["cut"]
                 and canonical(row["changes"][-1]["image"]) == canonical(got["image"]))
        e_cuts = [ch["cut"] for ch in watch["domains"]["E"]["changes"]]
        b_cuts = [ch["cut"] for ch in watch["domains"]["B:alpha"]["changes"]]
        q_b, q_e = snapshot["domains"]["B:alpha"]["image"], snapshot["domains"]["E"]["image"]
        decision = q_b["finance"]["decisions"][0]
        c.ok("q.commit_receipt", decision["state"] == "Committed"
             and decision["local_commit_receipt"] == e.committed["record"]["local_commit_receipt"], decision)
        c.ok("q.commit_e_cut", decision["local_commit_receipt"]["e_cut"] == e.reserved["cut"] and e.reserved["cut"] in e_cuts,
             decision["local_commit_receipt"])
        c.ok("q.receipt_cuts", all(r["cut"] in e_cuts for r in (e.open_receipt, gap, fill, resolved, val))
             and all(r["cut"] in b_cuts for r in (e.open_applied, e.committed, fill_applied)))
        c.ok("q.s_ref", decision["prepared"]["plan"]["s_ref"] == stack.s_ref
             and digest(stack.s_object) == stack.s_ref["object_hash"], decision["prepared"]["plan"]["s_ref"])
        c.ok("q.fill_allocation", q_e["fill_allocations"] == e_image["fill_allocations"]
             and any(a.get("fact_key") == fill["fact_key"] for a in q_b["applied"]), q_b["applied"])
        check_book(c, q_b, x, prefix="q.")
        original = stack.http(e, "/api/economic/original", {"op": "original", "kind": "decision", "domain_id": "B:alpha",
                                                            "object_id": "alpha", "cuts": final_cuts})
        c.ok("q.original_decision", original.get("kind") == "EconomicOriginal"
             and original["original"]["decision"] == decision, original)

        # ---- 关停重开：正式 recover，公共查询逐值相同；重送只回原收据 ----
        before = public(e, fill, gap, allocation["allocation_id"])
        commits_before = {dm: commits_of(e.tables(dm)) for dm in DOMAINS}
        stack.stop_q()
        for dm in reversed(DOMAINS):
            e.stop(dm)
        for dm in DOMAINS:
            e.cli(dm, "recover")
        for dm in DOMAINS:
            e.serve(dm)
        c.ok("reopen.public_equal", public(e, fill, gap, allocation["allocation_id"]) == before)
        stack.start_q(e, e.epochs)
        again = stack.paged(e, {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None})
        c.ok("q.restart_equal", all(again["domains"][dm]["cut"] == snapshot["domains"][dm]["cut"]
             and canonical(again["domains"][dm]["image"]) == canonical(snapshot["domains"][dm]["image"]) for dm in DOMAINS))
        c.ok("reopen.apply_resend", e.send("B:alpha", apply) == fill_applied)
        c.ok("reopen.commit_resend", e.send("B:alpha", e.commit_request["payload"]) == e.committed)
        c.ok("reopen.capture_resend", e.capture(d["fill"], "token-s1-fill")[1] == fill)
        c.ok("reopen.no_new_commit", {dm: commits_of(e.tables(dm)) for dm in DOMAINS} == commits_before
             and len(e.image("B:alpha")["applied"]) == applied_before + 1)
        c.complete()
        observed = {"K_before": x["K_before"], "lock": e.prepared["record"]["prepared"]["locked_amount"],
                    "worst_quantity": e.prepared["record"]["prepared"]["worst_quantity"],
                    "retained_total": e.reserved["record"]["retained_total"], "allocation": allocation,
                    "q_book": {k: q_b["book"][k] for k in ("actual_quantity", "cost_basis", "cost_remaining", "principal", "fixed_q")},
                    "q_growth_enabled": q_b["book"]["phase"]["growth_enabled"],
                    "commit_receipt": e.committed, "capture_receipt": fill, "resolve_receipt": resolved,
                    "apply_receipt": fill_applied, "s_ref": stack.s_ref,
                    "q_watch_changes": {dm: len(watch["domains"][dm]["changes"]) for dm in DOMAINS},
                    "q_watch_segments": len(segments),
                    "q_final_cuts": final_cuts}
        result = {"scenario": "covered", "status": "PASS", "checks": c.passed, "observed": observed}
        save(e.output / "result.json", result)
        return result
    finally:
        errs = e.close()
        require(not errs, "covered 进程清理失败 " + repr(errs))


def check_book(c, b, x, prefix):
    book = b["book"]
    names = (("oracle.units_after", book["actual_quantity"] == str(x["units_after"])),
             ("oracle.k_after", int(book["cost_basis"]["K"]) == x["K_after"]),
             ("book.fixed_q", book["fixed_q"] == "12"),
             # 未签本金/增长政策不填默认：NotEvaluable，不当 0（ADR 0031/0036/0041）。
             ("book.principal_not_evaluable", book["principal"]["status"] == "NotEvaluable"),
             ("book.growth_not_evaluable", book["phase"]["growth_enabled"]["status"] == "NotEvaluable"),
             ("oracle.cost_remaining", book["cost_remaining"] == {"status": "Known", "value": str(x["cost_remaining"])}))
    if prefix:
        c.ok("q.oracle_book", all(ok for _, ok in names), book)
        return
    for name, ok in names:
        c.ok(name, ok, book)
    decision = b["finance"]["decisions"][0]
    c.ok("book.voice_of_decision", decision["state"] == "Committed" and decision["prepared"]["plan"]["voice_binding_id"] == "voice-alpha"
         and decision["prepared"]["voice"]["binding_id"] == "voice-alpha", decision)


LOOPBACK = b"127.0.0.1"  # DevSkim: ignore DS162092 只用于证据脱敏扫描


def archive(output):
    """证据打包：回环字面量替换为占位符后输出 base64 tar.gz（远端 worktree 会被删除）。"""
    buffer = io.BytesIO()
    redacted = 0
    with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
        for path in sorted(p for p in output.rglob("*") if p.is_file()):
            raw = path.read_bytes()
            if LOOPBACK in raw:
                redacted += 1
                raw = raw.replace(LOOPBACK, b"<loopback-v4>")
            info = tarfile.TarInfo(str(path.relative_to(output.parent)))
            info.size, info.mtime = len(raw), 0
            tar.addfile(info, io.BytesIO(raw))
    return base64.b64encode(buffer.getvalue()).decode(), redacted


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binaries", type=Path, default=ROOT.parent / "rust/target/debug")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--emit-archive", action="store_true")
    args = parser.parse_args()
    binaries = args.binaries.resolve()
    output = (args.output or Path(tempfile.mkdtemp(prefix="tb1673-out-")) / "run").resolve()
    require(not output.exists(), "输出目录必须新建")
    for name in ("s_structure_session", "e_economic_session", "b_chong_session"):
        require((binaries / name).is_file(), "缺正式二进制 " + str(binaries / name))
    inputs = load_inputs()
    oracle = inputs["oracle"]
    require(hand_oracle(oracle["inputs"]) == oracle["expected"], "冻结手算 oracle 与公式不符")
    stack = Stack(inputs, binaries, output)
    checkout = subprocess.run(["git", "-C", str(ROOT.parent), "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10)
    save(output / "provenance.json", {"issue": 1673, "source_sha": checkout.stdout.strip() or None,
        "driver_sha256": sha(__file__), "inputs": {n: INPUT_HASHES[n] for n in INPUT_NAMES},
        "binaries": {n: sha(binaries / n) for n in ("s_structure_session", "e_economic_session", "b_chong_session")},
        "python": sys.version.split()[0]})
    results, failure = [], None
    try:
        s_checks = Checks("covered")
        stack.start_s(s_checks)
        covered = run_covered(stack, oracle, s_checks.passed)
        missing = [n for n in CHECKS["covered"] if n not in covered["checks"]]
        require(not missing, "covered 断言未全部执行 " + repr(missing))
        results += [covered, run_uncovered(stack, oracle)]
    except Exception as error:  # 任何失败都只写 FAIL，绝不留下 PASS 收据。
        failure = {"type": type(error).__name__, "error": str(error)}
    finally:
        cleanup = stack.close()
        shutil.rmtree(stack.work, ignore_errors=True)
    status = "PASS" if failure is None and not cleanup and len(results) == 2 else "FAIL"
    summary = {"issue": 1673, "status": status, "failure": failure, "cleanup_errors": cleanup,
               "scenarios": [{k: r[k] for k in ("scenario", "status")} | {"checks": len(r["checks"])} for r in results],
               "trust_boundary": {
                   "s": "真实 s_structure_session 进程 + tb01c 冻结输入前缀（op-0000..op-0002，首个结构对象出现处）；计划 s_ref 绑定本次 financial-basis 实读对象",
                   "commit_fill_binding": "Commit↔成交对应仍由夹具构造（成交预列于冻结 FIN baseline、command_id 夹具填 plan-alpha）；产品绑定 F1/F2 归 TB-04-F #1425，未认定存在",
                   "voice": "单位信任夹具，非真实 γ 独评",
                   "policy": "θ/增长/本金政策不签不填；观测 principal/growth_enabled = NotEvaluable（ADR 0041 裁定二）",
                   "browser": "不在本票（#1460）"},
               "results": results}
    save(output / "RESULT.json", summary)
    print(json.dumps({k: summary[k] for k in ("status", "failure", "cleanup_errors", "scenarios")}, ensure_ascii=False))
    if args.emit_archive:
        data, redacted = archive(output)
        print("REDACTED_FILES=" + str(redacted))
        print("----BEGIN TB1673 ARCHIVE----")
        for i in range(0, len(data), 76):
            print(data[i:i + 76])
        print("----END TB1673 ARCHIVE----")
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
