"""#1394 工作草稿：固定历史协议的受准入读侧捕获器。

只产生未认证原件。根 grant、统一源兼容审查和独立运行监督缺任一项即拒绝。
MemoryTransport 仅供有限内存协议锁；不能注入 capture_product 的生产入口。
"""
from __future__ import annotations

import copy
from dataclasses import dataclass, field
import hashlib
import ipaddress
import math
import os
from pathlib import Path
import re
import socket
import stat
import subprocess
import sys
import time
import uuid

import captured_cut as x
import closure as c
import driver as d


SOURCE_FREEZE = "7b984bfcd74e7f22f7d229f12a317cb7746cedb3e4371a17a5282ef2f4c1d8ed"
ENVELOPE = set("schema_revision session_id session_generation source_namespace source_epoch message_id producer_id producer_epoch payload_hash causal_refs payload".split())
USAGE = dict(wire="max_total_wire_bytes", node_bytes="max_total_node_bytes",
             nodes="max_total_nodes", records="max_total_records", requests="max_total_requests",
             elapsed_ms="max_elapsed_ms", staged_bytes="max_staged_bytes", store_bytes="max_store_bytes")
MISSING = ["根外部冻结的 export_captured 合同 expected_sha 和独立原库节点读取器",
           "同域 terminal 全标签、完整 K/source、独立 γ、各 owner Voice 与独评",
           "完整产品 OS/传输、A/B、故障、历史、资金、隔离和恢复验收"]


def _text(value, label):
    d.require(type(value) is str and bool(value) and "\x00" not in value, label + " 缺失")
    return value


def _time(value):
    d.require(type(value) in (int, float) and math.isfinite(value) and value > 0,
              "缺固定 monotonic 时间")
    return value


def _q_uint(value):
    # q_snapshot 的 page_size/offset/count 是原 u64 文本域。
    d.require(type(value) is str and re.fullmatch(r"0|[1-9][0-9]*", value), "Q 非规范无符号整数")
    n = int(value)
    d.require(n < 2**64, "Q u64 超界")
    return n


def _ref(value):
    d.exact(value, {"format", "root", "kind", "extent"}, "ContentRef")
    d.require(value["format"] == "s-content/1" and value["kind"] in c.KINDS.values(), "ContentRef 版本/类型")
    c.root_bytes(value["root"])
    if value["extent"] is not None:
        x.decimal(value["extent"])


def financial_argv(binary, route, db, session):
    """八参数会话客户端；永不回落六参数冷读。"""
    d.require(route in {"financial-basis", "financial-closure"}, "未知 financial route")
    for value in (binary, db, session):
        d.require(Path(_text(value, "路径")).is_absolute(), "客户端路径须绝对")
    command = "financial-basis-ref" if route == "financial-basis" else "financial-closure"
    return [binary, command, "--protocol-revision", "s-session/3", "--db", db, "--session", session]


def _endpoint(value):
    d.exact(value, {"host", "port", "path"}, "Q endpoint")
    host = ipaddress.ip_address(value["host"])
    d.require(host.version == 4 and str(host) == value["host"], "本纵片仅实现明确 IPv4，无 DNS/代理")
    d.require(type(value["port"]) is int and 0 < value["port"] < 65536
              and value["path"] == "/api/v3/snapshot", "Q endpoint 路由/端口不符")


def http_request(endpoint, body):
    _endpoint(endpoint)
    d.require(type(body) is bytes, "HTTP body 必须原 bytes")
    return (f"POST /api/v3/snapshot HTTP/1.1\r\nHost: {endpoint['host']}:{endpoint['port']}\r\n"
            f"Content-Type: application/json\r\nContent-Length: {len(body)}\r\nConnection: close\r\n\r\n").encode("ascii") + body


def http_reply(raw):
    """仅解码已冻结 q_runtime 的 HTTP/1.0 外壳，保留完整原件于 Exchange。"""
    head, sep, body = raw.partition(b"\r\n\r\n")
    d.require(bool(sep), "HTTP 回复缺完整头")
    lines = head.split(b"\r\n")
    reasons = {200: "OK", 400: "Bad Request", 404: "Not Found", 414: "Request-URI Too Long",
               431: "Request Header Fields Too Large", 501: "Not Implemented", 503: "Service Unavailable"}
    d.require(len(lines) == 5, "Q HTTP 头字段不符")
    parts = lines[0].split(b" ", 2)
    d.require(len(parts) == 3 and parts[0] == b"HTTP/1.0" and parts[1].isdigit(), "Q HTTP 状态行不符")
    status = int(parts[1])
    d.require(status in reasons and parts[2] == reasons[status].encode(), "Q 未知 HTTP 结果")
    d.require(lines[1] == b"Content-Type: application/json; charset=utf-8"
              and lines[3:] == [b"Cache-Control: no-store", b"Connection: close"], "Q HTTP 固定头漂移")
    d.require(lines[2].startswith(b"Content-Length: "), "HTTP 缺 Content-Length")
    n = x.decimal(lines[2][16:].decode("ascii"))
    d.require(len(body) == n, "HTTP 截断/尾随原字节")
    return status, body


def validate_plan(plan):
    """纯数据预检；不连接、不读文件、不创建目录，也不批准产品执行。"""
    d.exact(plan, {"schema_revision", "lifecycle_id", "capture_id", "source", "fixed_cut", "anchor",
                   "page_size", "cut_projection_digest", "resources", "stage", "reader_binary",
                   "session", "q", "slots", "initial_usage", "initial_session_requests", "verification"}, "读侧计划")
    d.require(plan["schema_revision"] == "issue1394-product-capture/1", "读侧计划版本")
    for name in ("lifecycle_id", "capture_id", "stage"):
        _text(plan[name], name)
    fixed, anchor, resources = plan["fixed_cut"], plan["anchor"], plan["resources"]
    d.exact(fixed, x.FIXED, "固定 cut")
    d.exact(anchor, x.ANCHOR, "anchor")
    for key in ("session_id", "session_generation", "structure_cut", "index_frontier"):
        d.require(anchor[key] == fixed[key], "financial/Q 不同 cut")
    _ref(anchor["basis_ref"])
    d.require(fixed["history_mode"] == "AsKnown" and fixed["as_of_generation"] == fixed["cut_generation"], "未固定历史切面")
    source = plan["source"]
    d.exact(source, {"commit", "tree", "binary_sha256", "input_sha256", "artifact_manifest_sha256",
                     "profile_hash", "rule_revision", "catalog_revision"}, "来源身份")
    for key, length in (("commit", 40), ("tree", 40), ("binary_sha256", 64), ("input_sha256", 64),
                        ("artifact_manifest_sha256", 64), ("profile_hash", 64)):
        d.require(type(source[key]) is str and re.fullmatch("[0-9a-f]{" + str(length) + "}", source[key]), "来源摘要不符")
    d.require(all(source[k] == fixed[k] for k in ("profile_hash", "rule_revision", "catalog_revision")), "来源版本不符")
    # 复用原逐对象合同校验；这里的占位 Budget 仅携带已冻结截止，不发新帽。
    ends = [resources["deadlines"]["work"], resources["deadlines"]["stages"][plan["stage"]],
            *(j["deadline"] for j in resources["jobs"].values())]
    limit = min(ends)
    x.resources(resources, d.ReadBudget(limit, 1, 1), c.Budget(1, 1, 1, limit), plan["stage"])
    verify = plan["verification"]
    d.exact(verify, {"read_max_records", "read_max_bytes", "closure_max_nodes", "closure_max_bytes", "closure_max_visits"}, "根冻结工具读取预算")
    for value in verify.values():
        d.positive(value)
    d.require(0 < _q_uint(plan["page_size"]) <= d.positive(resources["q"]["limits"]["max_page_size"]), "Q 原 page_size 帽")
    d.require(re.fullmatch(r"qcut3-[0-9a-f]{64}", plan["cut_projection_digest"]), "Q 投影根格式")
    d.exact(plan["session"], {"owner_key", "identity", "binary_sha256", "db", "socket"}, "session owner")
    d.exact(plan["q"], {"owner_key", "identity", "binary_sha256", "endpoint", "producer_epoch", "request_header"}, "Q owner")
    _endpoint(plan["q"]["endpoint"])
    header = plan["q"]["request_header"]
    d.exact(header, {"source_namespace", "source_epoch", "producer_id", "producer_epoch"}, "Q 请求头身份")
    for val in header.values():
        _text(val, "Q 身份")
    d.positive(header["producer_epoch"])
    d.positive(plan["q"]["producer_epoch"])
    for role in ("session", "q"):
        item = plan[role]
        d.require(type(item["owner_key"]) is list and len(item["owner_key"]) == 3, "owner key 缺失")
        d.exact(item["identity"], {"pid", "start_identity", "process_group", "domain_id", "writer_epoch"}, "进程身份")
        for key in ("pid", "process_group"):
            d.require(type(item["identity"][key]) is int and item["identity"][key] > 0, "owner PID/group 非法")
        _text(item["identity"]["start_identity"], "start identity")
        d.require(item["identity"]["domain_id"] == ("S" if role == "session" else "Q")
                  and item["owner_key"][1:] == [item["identity"]["domain_id"], item["identity"]["writer_epoch"]], "owner 域/epoch 不符")
        d.require(re.fullmatch("[0-9a-f]{64}", item["binary_sha256"]), "owner binary pin")
    d.require(plan["session"]["binary_sha256"] == source["binary_sha256"], "reader/session binary 不同源")
    d.require(plan["session"]["identity"]["writer_epoch"] == anchor["writer_epoch"]
              and plan["q"]["identity"]["writer_epoch"] == plan["q"]["producer_epoch"], "owner epoch 不同")
    slots = plan["slots"]
    d.require(type(slots) is list and slots, "缺逐次原请求日程")
    seen, ranks = set(), []
    for slot in slots:
        d.exact(slot, {"id", "route", "stage", "deadline", "binding", "argv", "closure_max_reply_bytes"}, "请求槽")
        d.require(_text(slot["id"], "请求 ID") not in seen, "重复请求 ID")
        d.require(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", slot["id"]), "请求 ID 不得成为任意路径")
        seen.add(slot["id"])
        d.require(slot["stage"] in resources["deadlines"]["stages"], "请求当前 stage 未冻结")
        _time(slot["deadline"])
        name = x.capture_job(x.Capture(slot["route"], b"", b"", slot["binding"]), resources)
        d.require(slot["deadline"] <= min(resources["deadlines"]["work"],
                  resources["deadlines"]["stages"][slot["stage"]], resources["jobs"][name]["deadline"]), "请求截止晚于原阶段/whole/所属 Job")
        ranks.append({"snapshot": 0, "financial-basis": 1, "financial-closure": 2}[slot["route"]])
        expected = [] if slot["route"] == "snapshot" else financial_argv(
            plan["reader_binary"], slot["route"], plan["session"]["db"], plan["session"]["socket"])
        d.require(slot["argv"] == expected, "argv 偏离真实会话 CLI")
        if slot["route"] == "financial-closure":
            d.positive(slot["closure_max_reply_bytes"])
        else:
            d.require(slot["closure_max_reply_bytes"] is None, "非闭包请求不得借用闭包帽")
    d.require(ranks == sorted(ranks) and ranks.count(1) == 1 and ranks[0] == 0 and ranks[-1] == 2,
              "日程不是 Q 全页→单 basis→闭包全页")
    d.require(set(plan["initial_usage"]) == set(resources["jobs"]), "缺原生命周期累计值")
    for usage in plan["initial_usage"].values():
        d.exact(usage, set(USAGE), "Job 累计维度")
        d.require(all(type(v) is int and v >= 0 for v in usage.values()), "累计值非法")
    d.require(type(plan["initial_session_requests"]) is int and plan["initial_session_requests"] >= 0, "缺原 session 请求累计")


class Ledger:
    """根冻结生命周期单一账本；失败消耗保留，不得每次 capture 清零。"""
    def __init__(self, plan):
        validate_plan(plan)
        self.lifecycle = plan["lifecycle_id"]
        self.plan_fingerprint = x.fingerprint(plan)
        self.resources = copy.deepcopy(plan["resources"])
        self.used = copy.deepcopy(plan["initial_usage"])
        self.session_requests = plan["initial_session_requests"]
        self.consumed = set()
        self.dispatched = set()
        self.capture_ids = set()
        self.halted = False
        verify = plan["verification"]
        self.read_budget = d.ReadBudget(min(plan["resources"]["deadlines"]["work"],
            plan["resources"]["deadlines"]["stages"][plan["stage"]],
            *(slot["deadline"] for slot in plan["slots"])),
            d.positive(verify["read_max_records"]), d.positive(verify["read_max_bytes"]))
        self._check()

    def _check(self):
        for name, counts in self.used.items():
            for key, maximum in USAGE.items():
                d.require(counts[key] <= d.positive(self.resources["jobs"][name]["limits"][maximum]), "所属 Job 累计超帽：" + name + "/" + key)
        d.require(self.session_requests <= d.positive(self.resources["session"]["max_requests"]), "session 累计请求超帽")

    def add(self, name, **counts):
        for key, value in counts.items():
            d.require(key in USAGE and type(value) is int and value >= 0, "非法累计计费")
            self.used[name][key] += value
        self._check()

    def begin(self, plan, slot, request, transport_request, now):
        d.require(not self.halted and plan["lifecycle_id"] == self.lifecycle
                  and plan["resources"] == self.resources and x.fingerprint(plan) == self.plan_fingerprint,
                  "生命周期失效/计划或资源重置")
        key = slot["id"]
        d.require(key not in self.consumed, "单次请求已消费；失败不得重试")
        name = x.capture_job(x.Capture(slot["route"], request, b"", slot["binding"]), self.resources)
        roles = ("q",) if slot["route"] == "snapshot" else ("session", "reader")
        for role in roles:
            caps = self.resources["q"]["limits"] if role == "q" else self.resources[role]
            size = len(transport_request) if role == "q" else len(request)
            d.require(size <= d.positive(caps["max_frame_bytes" if role == "q" else "max_request_bytes"]), "原请求超过 " + role + " 帽")
        self.consumed.add(key)
        if slot["route"] != "snapshot":
            self.session_requests += 1
        self.add(name, requests=1, wire=len(transport_request))
        self.elapsed(name, now)
        return name

    def elapsed(self, name, now):
        job = self.resources["jobs"][name]
        d.require(now >= job["started_at"] and now < job["deadline"], "所属 Job 尚未开始/原截止已到")
        self.used[name]["elapsed_ms"] = max(self.used[name]["elapsed_ms"], math.ceil((now - job["started_at"]) * 1000))
        self._check()


@dataclass
class Exchange:
    """请求/回复保持 bytes；不把 JSON 重编码当原件。"""
    reply: bytes = b""
    stderr: bytes = b""
    transport_reply: bytes = b""
    returncode: int | None = None
    started: float = 0
    ended: float = 0
    complete: bool = False
    identity: dict = field(default_factory=dict)
    error: str | None = None


class MemoryTransport:
    """有限内存测试边界。没有 socket/Popen/文件读取或任意 callable。"""
    def __init__(self, replies):
        d.require(type(replies) is list and all(type(r) is Exchange for r in replies), "仅接受有限原件列表")
        self.replies = copy.deepcopy(replies)
        self.requests = []

    def exchange(self, slot, request, wire, deadline):
        self.requests.append((copy.deepcopy(slot), request, wire, deadline))
        d.require(bool(self.replies), "内存 fixture 缺回复")
        result = self.replies.pop(0)
        if result.started == 0:
            result.started = time.monotonic()
        if result.ended == 0:
            result.ended = time.monotonic()
        return result


class _ReadWork:
    """共用工具账；一次结构检查点计一条，原 bytes 只在进入解析前计入。"""
    def __init__(self, budget):
        self.budget = budget

    def check(self):
        self.budget.charge(())


class _ClosureWork(c.Budget):
    def check(self):
        self.read_budget.charge(())
        super().check()


class CaptureMachine:
    """有序协议状态机；传输异常及验证异常均停止，已收到的错误原件不丢。"""
    def __init__(self, plan, ledger, transport, *, clock=time.monotonic):
        validate_plan(plan)
        d.require(type(ledger) is Ledger and type(transport) in (MemoryTransport, ProductTransport), "未实现传输边界")
        self.plan, self.ledger, self.transport, self.clock = copy.deepcopy(plan), ledger, transport, clock
        if type(transport) is ProductTransport:
            d.require(plan == transport.gate.plan and ledger is transport.gate.ledger and clock is time.monotonic,
                      "生产状态机偏离根计划/原账本/原时钟")
            d.require(transport.machine is None, "生产传输不能绑定第二状态机")
            transport.machine = self
        self.rows, self.nodes, self.captures, self.originals = [], [], [], []
        self.q_done = self.closure_done = False
        self.metadata = self.token = self.issued = self.basis = self.total_nodes = None
        self.after = ""
        self.read_budget = ledger.read_budget
        self.read_work = _ReadWork(self.read_budget)

    def _deadline(self, slot, started):
        r = self.plan["resources"]
        name = slot["binding"]["job"]
        if slot["route"] == "snapshot":
            caps = r["q"]["limits"]
            timeout = min(d.positive(caps["socket_read_timeout_ms"]), d.positive(caps["socket_write_timeout_ms"]))
        else:
            timeout = min(d.positive(r[role]["request_timeout_ms"]) for role in ("session", "reader"))
        return min(slot["deadline"], r["deadlines"]["work"], r["deadlines"]["stages"][self.plan["stage"]],
                   r["deadlines"]["stages"][slot["stage"]], r["jobs"][name]["deadline"], started + timeout / 1000)

    def request(self, slot):
        fixed, anchor = self.plan["fixed_cut"], self.plan["anchor"]
        if slot["route"] == "snapshot":
            d.require(not self.q_done and self.basis is None, "Q 完成后多页/反序")
            payload = {"op": "snapshot", "snapshot_token": self.token} if self.token is not None else {
                "op": "snapshot", "session_id": fixed["session_id"], "session_generation": fixed["session_generation"],
                "scope": fixed["scope"], "history_mode": "AsKnown", "as_of_generation": fixed["cut_generation"],
                "page_size": self.plan["page_size"], "order_version": "s-record-order/2"}
            return d.canonical({"schema_revision": "s-session/3", "session_id": fixed["session_id"],
                "session_generation": fixed["session_generation"], **self.plan["q"]["request_header"],
                "message_id": slot["id"], "payload_hash": x.fingerprint(payload), "causal_refs": [], "payload": payload})
        d.require(self.q_done, "缺 Q 末页，不能发 financial")
        if slot["route"] == "financial-basis":
            d.require(self.basis is None, "重复 basis")
            return d.canonical({"schema_revision": "s-financial-query/1",
                **{k: anchor[k] for k in ("session_id", "session_generation", "writer_epoch")},
                "object_ids": [row["key"][0] for row in self.rows if row["family"] == "objects"]}) + b"\n"
        d.require(self.basis is not None and not self.closure_done, "缺 basis 或闭包已完成")
        return d.canonical({"schema_revision": "s-financial-closure/1", "anchor": anchor, "path": [],
                            "after": self.after, "max_reply_bytes": slot["closure_max_reply_bytes"]}) + b"\n"

    def _snapshot(self, capture, deadline):
        self.read_work.check()
        request, response = d.strict(capture.request), d.strict(capture.reply)
        d.exact(request, ENVELOPE, "Q 原请求信封")
        d.exact(response, ENVELOPE, "Q 原回复信封")
        _, page = x.envelope(capture, self.plan["fixed_cut"])
        d.require(response["source_namespace"] == "s-observe/replies"
                  and response["producer_id"] == "Q:" + self.plan["fixed_cut"]["session_id"]
                  and response["source_epoch"] == response["producer_epoch"] == self.plan["q"]["producer_epoch"], "Q 回复 owner 身份漂移")
        d.exact(page, {"fixed_cut", "catalog", "rows", "counts", "scope", "omissions", "order_version",
                       "offset", "done", "next_token", "cut_projection_digest", "validated_capture_digest", "ok"}, "Q snapshot 页")
        d.require(page["ok"] is True and page["fixed_cut"] == self.plan["fixed_cut"]
                  and page["scope"] == self.plan["fixed_cut"]["scope"] and page["omissions"] == []
                  and page["order_version"] == "s-record-order/2"
                  and page["cut_projection_digest"] == self.plan["cut_projection_digest"], "混 cut/错误/不完整 Q 页")
        _ref(page["catalog"])
        metadata = {k: page[k] for k in ("fixed_cut", "catalog", "counts", "scope", "omissions", "order_version", "cut_projection_digest")}
        if self.metadata is None:
            self.metadata = metadata
            self.issued = page["validated_capture_digest"]
        d.require(metadata == self.metadata and re.fullmatch("[0-9a-f]{64}", page["validated_capture_digest"]), "Q 元数据/捕获身份漂移")
        # 固定 cut token 的 issued 不变；协议允许下一次审计的 validated digest 改变。
        d.exact(page["counts"], set(x.FAMILIES) | {"total"}, "Q 六族计数")
        total = _q_uint(page["counts"]["total"])
        d.require(sum(_q_uint(page["counts"][f]) for f in x.FAMILIES) == total, "Q 总数不符")
        d.require(_q_uint(page["offset"]) == len(self.rows) and type(page["rows"]) is list
                  and len(page["rows"]) == min(_q_uint(self.plan["page_size"]), total - len(self.rows)), "Q 乱序/漏页/页长错误")
        seen = {(row["family"], d.canonical(row["key"])) for row in self.rows}
        object_ids = {row["key"][0] for row in self.rows if row["family"] in ("objects", "withdrawn_objects")}
        for row in page["rows"]:
            self.read_work.check()
            d.exact(row, {"family", "key", "record"}, "Q 公开记录")
            d.require(row["family"] in x.FAMILIES and type(row["key"]) is list, "Q 未知记录族")
            width = 3 if row["family"] == "relations" else 2 if row["family"] == "raw_history" else 1
            d.require(len(row["key"]) == width and all(type(v) is str and v for v in row["key"]), "Q 公开键格式")
            key = (row["family"], d.canonical(row["key"]))
            d.require(key not in seen, "Q 重复记录")
            seen.add(key)
            if row["family"] in ("objects", "withdrawn_objects"):
                d.require(row["key"][0] not in object_ids, "Q active/withdrawn 重复对象")
                object_ids.add(row["key"][0])
            d.require(not self.rows or x.FAMILIES.index(row["family"]) >= x.FAMILIES.index(self.rows[-1]["family"]), "Q 记录族反序")
            _ref(row["record"])
            self.rows.append(row)
        done = len(self.rows) == total
        d.require(page["done"] is done, "Q 提前/滞后 done")
        expected = None
        if not done:
            expected = {k: self.plan["fixed_cut"][k] for k in x.TOKEN_FIXED}
            expected.update(token_schema="s-snapshot-token/2", order_version="s-record-order/2",
                page_size=self.plan["page_size"], offset=str(len(self.rows)),
                cut_projection_digest=page["cut_projection_digest"], issued_capture_digest=self.issued)
            expected["token_checksum"] = x.fingerprint(expected)
        d.require(page["next_token"] == expected, "Q 17 字段 token/issued/checksum 不同")
        self.token, self.q_done = page["next_token"], done
        if done:
            for family in x.FAMILIES:
                d.require(sum(r["family"] == family for r in self.rows) == _q_uint(page["counts"][family]), "Q 族计数不符")
            self._check_projection(deadline)
        # 历史 Q 每页重投影完整六族；不能用该页仅返回的行数缩小原 Job 计费。
        # 当前全历史审计另有访问，实际额外消耗仍必须由 READ_ACK 提供。
        return total

    def _check_projection(self, deadline):
        # 全页原序先与冻结根相符，才可发 basis；不等 financial 后才发现 Q 行序变动。
        budget = self.read_work
        fixed = self.plan["fixed_cut"]
        projection = {**self.metadata, "fixed_cut": dict(fixed, catalog_evidence=x.Edge(fixed["catalog_evidence"])),
                      "catalog": x.Edge(self.metadata["catalog"]),
                      "rows": [dict(row, record=x.Edge(row["record"])) for row in self.rows]}
        projection.pop("cut_projection_digest")
        root = x.content_ref(projection, budget)
        digest = "qcut3-" + c.root_bytes(c.node_root(0x22, c.root_bytes(root["root"]))).hex()
        d.require(digest == self.plan["cut_projection_digest"], "Q 全页原序/完整投影根不同")

    def _basis(self, capture):
        self.read_work.check()
        request, reply, anchor = d.strict(capture.request), d.strict(capture.reply), self.plan["anchor"]
        d.exact(reply, {"schema_revision", "content_format", "kind", "session_id", "session_generation", "writer_epoch",
                        "pending_revision", "commit_ns", "basis_ref", "snapshot_ref", "selected_objects", "proof_nodes"}, "basis 回复")
        d.require(reply["schema_revision"] == "s-financial-basis/3" and reply["kind"] == "FinancialBasisRef"
                  and reply["content_format"] == "s-content/1" and reply["pending_revision"] is False
                  and reply["basis_ref"] == anchor["basis_ref"]
                  and all(reply[k] == anchor[k] for k in ("session_id", "session_generation", "writer_epoch", "commit_ns")), "basis 错绑/未知结果")
        _ref(reply["snapshot_ref"])
        wanted = request["object_ids"]
        selected = reply["selected_objects"]
        d.require(type(selected) is list and len(selected) == len(wanted), "basis 对象漏项")
        references = {r["key"][0]: r["record"] for r in self.rows if r["family"] == "objects"}
        indices = set()
        for oid, item in zip(wanted, selected):
            self.read_work.check()
            d.exact(item, {"object_id", "object_ref", "object_hash", "snapshot_index"}, "selected object")
            index = x.decimal(item["snapshot_index"])
            d.require(index < len(wanted) and index not in indices, "basis index 缺漏/重复")
            indices.add(index)
            ref = references[oid]
            d.require(item["object_id"] == oid and item["object_ref"] == ref
                      and item["object_hash"] == c.root_bytes(c.node_root(0x23, c.root_bytes(ref["root"]))).hex(), "basis 非全对象原序/ref/hash")
        d.require(type(reply["proof_nodes"]) is list, "basis proof 非数组")
        prior = ""
        for raw in reply["proof_nodes"]:
            self.read_work.check()
            node = self._node(raw)
            d.require(node.root > prior, "basis proof 乱序/重复")
            prior = node.root
        self.basis = reply

    @staticmethod
    def _node(raw):
        d.exact(raw, {"root", "tag", "body_hex"}, "闭包节点")
        d.require(type(raw["tag"]) is str and re.fullmatch("[0-9a-f]{2}", raw["tag"])
                  and type(raw["body_hex"]) is str and re.fullmatch("(?:[0-9a-f]{2})*", raw["body_hex"]), "节点编码不是原规范 hex")
        node = c.Node(raw["root"], int(raw["tag"], 16), bytes.fromhex(raw["body_hex"]))
        d.require(c.node_root(node.tag, node.body) == node.root, "节点摘要不符")
        return node

    def _closure(self, capture):
        self.read_work.check()
        request, reply, anchor = d.strict(capture.request), d.strict(capture.reply), self.plan["anchor"]
        d.exact(reply, {"anchor", "path", "after", "target_ref", "total", "nodes", "next", "done", "ok"}, "闭包回复")
        d.require(reply["ok"] is True and reply["anchor"] == anchor and reply["path"] == []
                  and reply["after"] == self.after and reply["target_ref"] == anchor["basis_ref"], "闭包错 anchor/path/after")
        d.require(len(capture.reply) <= d.positive(request["max_reply_bytes"]), "闭包超过本页原回复帽")
        total = x.decimal(reply["total"])
        if self.total_nodes is None:
            self.total_nodes = total
        d.require(total == self.total_nodes and type(reply["nodes"]) is list and reply["nodes"], "闭包 total 漂移/空页")
        count = size = 0
        for raw in reply["nodes"]:
            self.read_work.check()
            node = self._node(raw)
            d.require(node.root > (self.nodes[-1].root if self.nodes else ""), "闭包乱序/重复")
            self.nodes.append(node)
            count += 1
            size += len(c.DOMAIN) + 9 + len(node.body)
        d.require(len(self.nodes) <= total and len(self.nodes) <= d.positive(self.plan["resources"]["session"]["max_closure_nodes"]), "闭包 total/原 session 节点帽")
        self.after = self.nodes[-1].root
        self.closure_done = len(self.nodes) == total
        d.require(reply["next"] == self.after and reply["done"] is self.closure_done, "闭包真实 next/done 错误")
        return count, size

    def _finish_graph(self, deadline):
        """只核收到流内部的可达全集，绝不冒充独立持久 source 或调用 export。"""
        reader_job = next(n for n, j in self.plan["resources"]["jobs"].items()
                          if j["owner"] == self.plan["resources"]["object_ids"]["reader"])
        caps = self.plan["resources"]["jobs"][reader_job]["limits"]
        # 沿根可达边对照收到的节点全集；typed verifier 与独立库读取留给 export。
        by_root = {node.root: node for node in self.nodes}
        for raw in self.basis["proof_nodes"]:
            self.read_work.check()
            n = self._node(raw)
            d.require(by_root.get(n.root) == n, "basis proof 不是同一闭包原件")
        d.require(len(self.nodes) <= d.positive(caps["max_total_nodes"]), "reader 节点帽")
        verify = self.plan["verification"]
        budget = self.read_work
        cb = _ClosureWork(d.positive(verify["closure_max_nodes"]), d.positive(verify["closure_max_bytes"]),
                      d.positive(verify["closure_max_visits"]), deadline)
        cb.read_budget = self.read_budget
        # 此调用只做收到流的内部类型/可达校验。source 是同一流，故不是独立原库验真。
        # 不把 c.verify 的返回值作为导出认证，不调用 export_captured。
        c.verify(self.plan["anchor"]["basis_ref"], self.nodes, by_root.__getitem__, cb)
        basis = x.decode_verified(self.plan["anchor"]["basis_ref"], by_root, budget)
        anchor, fixed = self.plan["anchor"], self.plan["fixed_cut"]
        d.require(basis["kind"] == "FinancialBasis" and basis["pending_revision"] is False
                  and all(basis[k] == anchor[k] for k in ("session_id", "session_generation", "writer_epoch", "commit_ns")), "闭包 basis 身份漂移")
        snapshot = basis["snapshot"]
        d.exact(snapshot, x.SNAPSHOT, "完整 snapshot")
        for key, skey in (("session_id", "session_id"), ("cut_generation", "generation"), ("structure_cut", "structure_cut"),
                          ("index_frontier", "index_frontier"), ("profile_hash", "profile_hash"), ("profile_id", "profile_id"),
                          ("catalog_revision", "catalog_revision"), ("input_frontier", "input_frontier"), ("scope", "scope"),
                          ("seq_range", "seq_range"), ("catalog_run_status", "catalog_run_status"),
                          ("history_mode", "history_mode"), ("as_of_generation", "as_of_generation")):
            d.require(snapshot[skey] == fixed[key], "原闭包 snapshot 不同 cut：" + key)
        d.require(x.content_ref(snapshot["catalog_evidence"], budget) == fixed["catalog_evidence"], "catalog evidence 根不同")
        d.require(x.content_ref(snapshot, budget) == self.basis["snapshot_ref"], "basis snapshot_ref 不同")
        expected = []
        for family in x.FAMILIES:
            for record in sorted(snapshot[family], key=lambda row: x.row_identity(family, row)[1]):
                budget.check()
                expected.append({"family": family, "key": x.row_identity(family, record)[0], "record": x.content_ref(record, budget)})
        d.require(expected == self.rows, "Q 原序/六族全集不是实际闭包 snapshot")
        for item in self.basis["selected_objects"]:
            record = snapshot["objects"][x.decimal(item["snapshot_index"])]
            d.require(record["object_id"] == item["object_id"] and x.content_ref(record, budget) == item["object_ref"], "basis snapshot_index 错位")
        self.ledger.add(reader_job, records=len(expected))
        d.require(self.clock() < deadline, "最终校验超过原截止")

    def run(self):
        d.require(not self.ledger.halted and self.plan["capture_id"] not in self.ledger.capture_ids,
                  "本生命周期已失败或 capture identity 重复")
        self.ledger.capture_ids.add(self.plan["capture_id"])
        failure = None
        try:
            for slot in self.plan["slots"]:
                started = self.clock()
                deadline = self._deadline(slot, started)
                d.require(started < deadline, "调用前原截止已到")
                request = self.request(slot)
                wire = http_request(self.plan["q"]["endpoint"], request) if slot["route"] == "snapshot" else request
                name = self.ledger.begin(self.plan, slot, request, wire, started)
                exchange = Exchange(started=started)
                original = {"slot": copy.deepcopy(slot), "deadline": deadline, "request": request,
                            "transport_request": wire, "exchange": exchange}
                self.originals.append(original)
                # 一份原请求帧计一条；完整 HTTP 头也计入。不能以 canonical 大小替代。
                self.read_budget.deadline = min(self.read_budget.deadline, deadline)
                self.read_budget.charge((wire,))
                try:
                    received = self.transport.exchange(slot, request, wire, deadline)
                    d.require(type(received) is Exchange, "传输未返回原件")
                    exchange = received
                except (d.Rejected, OSError, ValueError, TypeError, KeyError) as error:
                    exchange.error = "DeliveryUnknown: " + type(error).__name__ + ": " + str(error)
                    exchange.ended = self.clock()
                original["exchange"] = exchange
                # 先保留全部已收到原件，再处理超时、错误码、解析或帽失败。
                capture = x.Capture(slot["route"], request, exchange.reply, copy.deepcopy(slot["binding"]))
                self.captures.append(capture)
                # 先封存实收原件，再在任何解析前计费。CLI reply 与 transport_reply 是同一原件。
                try:
                    self.ledger.add(name, wire=len(exchange.transport_reply) + len(exchange.stderr))
                finally:
                    # 一份回复原件连同 stderr 计一条；任一账拒绝都不能抹掉另一账已观测的用量。
                    self.read_budget.charge((exchange.transport_reply, exchange.stderr))
                d.require(exchange.complete and exchange.error is None, "传输未知/截断：" + str(exchange.error))
                d.require(started <= exchange.started <= exchange.ended <= self.clock() < deadline,
                          "调用后超原截止/时间身份非法")
                self.ledger.elapsed(name, self.clock())
                if slot["route"] == "snapshot":
                    status, body = http_reply(exchange.transport_reply)
                    d.require(body == exchange.reply and status == 200 and exchange.returncode is None
                              and exchange.stderr == b"", "HTTP 错误/原信封丢失")
                    d.require(exchange.identity == {"owner": self.plan["q"]["identity"],
                        "endpoint": self.plan["q"]["endpoint"], "http_status": status}, "HTTP owner/端点身份不同")
                    d.require(len(body) <= d.positive(self.plan["resources"]["q"]["limits"]["max_reply_bytes"]), "Q 原回复帽")
                    records = self._snapshot(capture, deadline)
                    self.ledger.add(name, records=records)
                else:
                    d.require(exchange.transport_reply == exchange.reply and type(exchange.returncode) is int
                              and exchange.returncode == 0, "CLI 非零退出/未知结果")
                    ident = exchange.identity
                    d.require(ident.get("argv") == slot["argv"] and ident.get("stage") == slot["stage"]
                              and ident.get("binary_sha256_before") == ident.get("binary_sha256_after") == self.plan["source"]["binary_sha256"]
                              and ident.get("reaped") is True and ident["owner"]["domain_id"] == "S"
                              and ident["owner"]["writer_epoch"] == self.plan["anchor"]["writer_epoch"], "CLI 命令/binary/退出身份不同")
                    for role in ("session", "reader"):
                        d.require(len(exchange.reply) <= d.positive(self.plan["resources"][role]["max_reply_bytes"]), "原 stdout 超 " + role + " 帽")
                    d.require(len(exchange.stderr) <= d.positive(self.plan["resources"]["reader"]["max_stderr_bytes"]), "原 stderr 超 reader 帽")
                    if slot["route"] == "financial-basis":
                        self._basis(capture)
                    else:
                        count, size = self._closure(capture)
                        self.ledger.add(name, nodes=count, node_bytes=size)
                d.require(self.clock() < deadline, "解析/计费后原截止已到")
                if type(self.transport) is ProductTransport:
                    self.transport.gate.guard(slot, request, deadline, "after")
            d.require(self.q_done and self.basis is not None and self.closure_done, "缺完整末页/basis/closure")
            self._finish_graph(deadline)
            if type(self.transport) is ProductTransport:
                self.transport.gate.guard(slot, request, deadline, "finalize")
        except (d.Rejected, ValueError, KeyError, TypeError, OSError, OverflowError) as error:
            self.ledger.halted = True
            failure = type(error).__name__ + ": " + str(error)
        return {"status": "MISSING_DEPENDENCY", "capture_status": "INCOMPLETE" if failure else "UNAUTHENTICATED_RAW_CAPTURE",
                "execution_authorized": False, "product_status": "NOT_RUN" if type(self.transport) is MemoryTransport else "NOT_ACCEPTED",
                "capture_id": self.plan["capture_id"], "lifecycle_id": self.ledger.lifecycle,
                "captures": [item.identity() for item in self.captures], "observed_job_usage": copy.deepcopy(self.ledger.used),
                "session_requests": self.ledger.session_requests, "failure": failure, "missing": list(MISSING)}


class ReadAdmission:
    """复用真实 ExecutionAdmission；计划和代码 pin 必须在根合同里。

    read_guard/1 是尚待独立 OS 工位实现的控制扩展。旧 ACK 不会被解释成 READ_ACK；
    没有根冻结的实现与兼容审查原件，构造即失败；服务不支持扩展则产品 I/O 前失败。
    """
    def __init__(self, admission, supervisor):
        d.require(type(admission) is d.ExecutionAdmission and type(supervisor) is d.Supervisor,
                  "产品未准入；布尔值/模式/fake 不能授权")
        d.require(supervisor.contract == admission.contract_sha
                  and supervisor.endpoint == admission.contract["supervisor_socket"], "监督未绑定根 grant")
        ext = admission.contract.get("product_capture")
        d.exact(ext, {"schema_revision", "adapter_sha256", "plan", "compatibility", "supervision", "supervisor_limits", "output"}, "根读侧准入扩展")
        d.require(Path(_text(ext["output"], "原件输出目录")).is_absolute(), "输出目录须由根冻结绝对路径")
        d.exact(ext["supervisor_limits"], {"max_frame_bytes", "deadline"}, "原控制输送限制")
        d.require(supervisor.frame == d.positive(ext["supervisor_limits"]["max_frame_bytes"])
                  and supervisor.deadline == _time(ext["supervisor_limits"]["deadline"])
                  and supervisor.deadline <= admission.business_deadline, "监督控制帧/原截止偏离根冻结")
        d.require(ext["schema_revision"] == "issue1394-read-admission/1"
                  and ext["adapter_sha256"] == d.sha(Path(__file__).read_bytes()), "adapter 未被根精确 pin")
        for key in ("plan", "compatibility", "supervision"):
            d.require(ext[key] in admission.contract["artifacts"], "读侧原件不在根 artifacts 中")
        raw = d.pinned(ext["plan"])
        plan = d.strict(raw)
        validate_plan(plan)
        compatibility = d.strict(d.pinned(ext["compatibility"]))
        d.exact(compatibility, {"schema_revision", "source", "historical_freeze_sha256", "adapter_sha256", "verdict"}, "统一源兼容核认")
        d.require(compatibility == {"schema_revision": "issue1394-read-compatibility/1", "source": plan["source"],
                    "historical_freeze_sha256": SOURCE_FREEZE, "adapter_sha256": ext["adapter_sha256"],
                    "verdict": "COMPATIBLE"}, "缺统一新 source 语义兼容核认")
        supervision = d.strict(d.pinned(ext["supervision"]))
        d.exact(supervision, {"schema_revision", "implementation", "capabilities", "verdict"}, "独立 OS 监督实现核认")
        d.require(supervision["schema_revision"] == "issue1394-read-supervision/1"
                  and supervision["verdict"] == "APPROVE_READ_GUARD"
                  and supervision["capabilities"] == ["process_tree", "hard_deadlines", "per_object_resources",
                                                        "endpoint_owner", "read_guard/1"]
                  and supervision["implementation"] in admission.contract["artifacts"], "MISSING_DEPENDENCY：未实现/未核认读侧 OS 监督")
        d.pinned(supervision["implementation"])
        self.admission, self.supervisor, self.plan, self.plan_raw = admission, supervisor, plan, raw
        self.contract_fingerprint = x.fingerprint(admission.contract)
        self.extension = copy.deepcopy(ext)
        self.supervision = supervision
        d.require(plan["resources"]["deadlines"]["work"] <= admission.business_deadline
                  and plan["resources"]["deadlines"]["cleanup"] <= admission.cleanup_deadline
                  and plan["resources"]["deadlines"]["seal"] <= admission.seal_deadline, "读侧延长根截止")
        # 账本放在唯一已消费的 grant 生命周期，重复建 wrapper 不会重置。
        state = getattr(admission, "_product_capture_state", None)
        if state is None:
            state = (d.sha(raw), Ledger(plan))
            admission._product_capture_state = state
        d.require(state[0] == d.sha(raw), "同 grant 不准更换生命周期/初始计数")
        self.ledger = state[1]
        self.guards = []
        self.guarded = None
        self.preflight()

    def preflight(self):
        """先做全部纯身份/命令/端点/原截止检查，错误时零连接/Popen。"""
        a = self.admission
        d.require(type(a) is d.ExecutionAdmission and x.fingerprint(a.contract) == self.contract_fingerprint,
                  "根合同运行中被修改")
        d.require(a.contract["product_capture"] == self.extension and type(self.supervisor) is d.Supervisor
                  and self.supervisor.contract == a.contract_sha and self.supervisor.endpoint == a.contract["supervisor_socket"]
                  and self.supervisor.frame == d.positive(self.extension["supervisor_limits"]["max_frame_bytes"])
                  and self.supervisor.deadline == self.extension["supervisor_limits"]["deadline"], "准入扩展/监督连接运行中漂移")
        d.require(d.canonical(self.plan) == d.canonical(d.strict(self.plan_raw)), "读侧计划运行中漂移")
        d.require(not self.ledger.halted, "原生命周期已拒绝")
        for role in ("session", "q"):
            pin = self.plan[role]
            owner = a.active_owners.get(tuple(pin["owner_key"]))
            d.require(type(owner) is d.OwnedProcess and owner.admission is a and owner.released
                      and owner.identity == pin["identity"] and owner.binary_before == pin["binary_sha256"]
                      and owner.supervisor is self.supervisor, "缺同 grant 已登记受监督 owner：" + role)
            command = next((cmd for cmd in a.contract["commands"] if cmd["stage"] == pin["owner_key"][0]), None)
            endpoint = pin["endpoint"] if role == "q" else {"db": pin["db"], "socket": pin["socket"]}
            d.require(command is not None and command.get("read_endpoint") == endpoint
                      and command["argv"] == owner.argv and command["binary_sha256"] == pin["binary_sha256"], "owner/端点未在原命令合同绑定")
        seen = set()
        for slot in self.plan["slots"]:
            d.require(slot["stage"] not in seen, "逐次命令 stage 必须唯一")
            seen.add(slot["stage"])
            d.require(slot["deadline"] <= a.business_deadline and
                      (slot["id"] in self.ledger.consumed or time.monotonic() < slot["deadline"]), "原请求截止已过/越界")
            if slot["route"] == "snapshot":
                continue
            commands = [cmd for cmd in a.contract["commands"] if cmd["stage"] == slot["stage"]]
            d.require(len(commands) == 1, "缺单次精确 reader 命令")
            cmd = commands[0]
            d.require(cmd["argv"] == slot["argv"] and cmd["binary_sha256"] == self.plan["source"]["binary_sha256"]
                      and cmd["domain_id"] == "S" and cmd["writer_epoch"] == self.plan["anchor"]["writer_epoch"]
                      and cmd["resource_caps"] == self.plan["resources"], "reader argv/binary/domain/epoch/资源不符")

    def guard(self, slot, request, deadline, phase):
        self.preflight()
        a, p = self.admission, self.plan
        d.require(time.monotonic() < deadline, "guard 前原截止已到")
        role = "q" if slot["route"] == "snapshot" else "session"
        owner = a.active_owners[tuple(p[role]["owner_key"])]
        owner.verify_identity(deadline=deadline)
        d.require(d.bounded_hash(owner.argv[0], deadline) == p[role]["binary_sha256"], "owner binary 漂移")
        if slot["argv"]:
            d.require(d.bounded_hash(slot["argv"][0], deadline) == p["source"]["binary_sha256"], "reader binary 漂移")
        endpoint = p["q"]["endpoint"] if role == "q" else {"db": p["session"]["db"], "socket": p["session"]["socket"]}
        # 既有精确 Supervisor ACK 不等于 endpoint/Native 资源证明。
        self.supervisor.ack(slot["stage"], owner.identity, p["resources"], slot["deadline"])
        d.remaining(deadline)
        before = time.monotonic()
        binding = {"contract_sha256": a.contract_sha, "plan_sha256": d.sha(self.plan_raw),
                   "nonce": uuid.uuid4().hex, "phase": phase, "owner": owner.identity, "endpoint": endpoint,
                   "slot": slot, "request_sha256": d.sha(request), "deadline": deadline,
                   "lifecycle_id": self.ledger.lifecycle, "usage_floor": copy.deepcopy(self.ledger.used),
                   "session_requests_floor": self.ledger.session_requests}
        request_raw = d.canonical({"kind": "READ_GUARD", "schema_revision": "issue1394-read-guard/1", "binding": binding}) + b"\n"
        # 复用已有有界原字节控制输送；超时不续期，不使用旧 ACK 兜底。
        result = d.exchange_frame(self.supervisor.endpoint, request_raw,
                                  max(1, math.ceil((deadline - before) * 1000)), self.supervisor.frame, deadline)
        self.guards.append({"request": request_raw, "result": result})
        d.require(result["transport"] == "Received", "MISSING_DEPENDENCY：read_guard/1 未收到完整监督回复")
        reply = result["response"]
        d.exact(reply, {"kind", "binding", "checked_at", "job_usage", "session_requests"}, "独立 READ_ACK")
        d.require(reply["kind"] == "READ_ACK" and reply["binding"] == binding
                  and before <= _time(reply["checked_at"]) <= time.monotonic() < deadline,
                  "独立读侧监督身份/原截止不符")
        d.require(set(reply["job_usage"]) == set(self.ledger.used), "监督 Job 漏项")
        for name, counts in reply["job_usage"].items():
            d.exact(counts, set(USAGE), "监督 Job 八维")
            d.require(all(type(v) is int and v >= self.ledger.used[name][k] for k, v in counts.items()), "监督累计回退/不可测")
        d.require(type(reply["session_requests"]) is int and reply["session_requests"] >= self.ledger.session_requests, "监督 session 累计回退")
        self.ledger.used = copy.deepcopy(reply["job_usage"])
        self.ledger.session_requests = reply["session_requests"]
        self.ledger._check()
        self.guarded = (slot["id"], d.sha(request), deadline, phase)

    def verify_current(self):
        """目录占用前重核实际 owner/binary 和原最早截止；不发监督或产品请求。"""
        self.preflight()
        r, a = self.plan["resources"], self.admission
        deadline = min(a.business_deadline, self.supervisor.deadline, r["deadlines"]["work"],
            r["deadlines"]["stages"][self.plan["stage"]],
            *(j["deadline"] for j in r["jobs"].values()),
            *(min(s["deadline"], r["deadlines"]["stages"][s["stage"]]) for s in self.plan["slots"]))
        d.remaining(deadline)
        for role in ("session", "q"):
            pin = self.plan[role]
            owner = a.active_owners[tuple(pin["owner_key"])]
            owner.verify_identity(deadline=deadline)
            d.require(d.bounded_hash(owner.argv[0], deadline) == pin["binary_sha256"], "owner binary 漂移")
        d.require(d.bounded_hash(self.plan["reader_binary"], deadline) == self.plan["source"]["binary_sha256"],
                  "reader binary 漂移")
        d.remaining(deadline)


class _StdinProcess(d.OwnedProcess):
    """原 OwnedProcess 的严格 stdin gate 适配；复用登记、身份、cleanup/reap。

    gate 用 os.read(fd,1) 读取控制行，绝不预读产品 stdin。ACK 后才 exec。
    不调用 subprocess.run，不改变 driver 原来的 writer 路径。
    """
    def __init__(self, gate, slot, request, output, deadline):
        d.require(type(gate) is ReadAdmission, "stdin 进程缺真实准入")
        gate.preflight()
        d.require(gate.guarded == (slot["id"], d.sha(request), deadline, "before")
                  and slot["id"] in gate.ledger.dispatched, "stdin gate 缺本次独立读侧监督")
        a, supervisor = gate.admission, gate.supervisor
        commands = [cmd for cmd in a.contract["commands"] if cmd["stage"] == slot["stage"]]
        d.require(len(commands) == 1, "缺单次 reader 命令")
        caps = commands[0]["resource_caps"]
        # 唯一消费点；预检绝不提前调用 check_command。
        command = a.check_command(slot["argv"], slot["stage"], caps, deadline)
        self.domain, self.epoch, self.stage = command["domain_id"], command["writer_epoch"], slot["stage"]
        self.fault, self.admission, self.supervisor = None, a, supervisor
        self.cleanup_deadline, self.seal_deadline = a.cleanup_deadline, a.seal_deadline
        self.term_grace_seconds, self.kill_wait_seconds = a.term_grace_seconds, a.kill_wait_seconds
        self.output, self.argv, self.caps, self.deadline = Path(output), slot["argv"], caps, deadline
        self.binary_before = command["binary_sha256"]
        self.stdout = self.stderr = self.process = self.directory = None
        self.output_readers, self.output_originals, self.output_inventory = {}, {}, {}
        self.output_errors = []
        self.identity, self.cleanup_result, self.released = {}, None, False
        self.sent = 0
        try:
            d.remaining(deadline)
            self.directory = d.OwnedDirectory(output)
            for name in ("stdout", "stderr"):
                stream = self.directory.open_new(name)
                setattr(self, name, stream)
                # 独立读句柄只从刚取得的目录打开，并与本次新建的写句柄逐项核身份。
                fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=self.directory.fd)
                try:
                    held, created = os.fstat(fd), os.fstat(stream.fileno())
                    d.require(stat.S_ISREG(held.st_mode) and (held.st_dev, held.st_ino) ==
                              (created.st_dev, created.st_ino), "原输出文件身份被替换")
                    self.directory.verify()
                    self.output_readers[name] = (fd, (held.st_dev, held.st_ino))
                except BaseException:
                    os.close(fd)
                    raise
            control = ("import json,os,time\nb=bytearray()\nwhile True:\n c=os.read(0,1)\n"
                       " if not c: raise SystemExit(125)\n b.extend(c)\n if c==b'\\n': break\n"
                       "p=json.loads(b)\nif time.monotonic()>=p['deadline']: raise SystemExit(124)\n"
                       "os.execv(p['argv'][0],p['argv'])\n")
            self.process = subprocess.Popen([sys.executable, "-c", control], stdin=subprocess.PIPE,
                stdout=self.stdout, stderr=self.stderr, start_new_session=True, close_fds=True,
                cwd=a.contract["cwd"], env=a.contract["environment"])
            self.identity = {"pid": self.process.pid, "domain_id": self.domain, "writer_epoch": self.epoch}
            # 原 register_owner 禁止所有同域并活；会话客户端必须与已登记 S session 共存。
            # 只允许根计划点名的那一个 session，其他同域存活进程仍拒绝。
            key = (self.stage, self.domain, self.epoch)
            session_key = tuple(gate.plan["session"]["owner_key"])
            d.require(key in a.consumed and key not in a.active_owners, "reader 未精确消费/重复登记")
            for prior_key, prior in a.active_owners.items():
                d.require(prior.domain != self.domain or prior_key == session_key or prior.process.poll() is not None,
                          "同 S 域存在未许可的并行客户端")
            a.active_owners[key] = self
            self.identity.update(d.process_identity(self.process, deadline))
            ack = supervisor.ack(self.stage, self.identity, caps, slot["deadline"])
            d.remaining(deadline)
            self._save("ACK.json", ack, deadline)
            self._save("OWNER.json", {**self.identity, "argv": self.argv, "stdin_sha256": d.sha(request)}, deadline)
            d.require(d.bounded_hash(self.argv[0], deadline) == self.binary_before, "exec 前 binary 漂移")
            self.directory.verify()
            wire = d.canonical({"argv": self.argv, "deadline": deadline}) + b"\n"
            d.require(len(wire) <= min(supervisor.frame, 1048576), "stdin gate 控制帧超原监督/driver 帽")
            os.set_blocking(self.process.stdin.fileno(), False)
            self.released = True
            for block in (wire, request):
                sent = 0
                while sent < len(block):
                    d.remaining(deadline)
                    try:
                        n = os.write(self.process.stdin.fileno(), block[sent:])
                        d.require(n > 0, "stdin 写入停滞")
                        sent += n
                    except BlockingIOError:
                        time.sleep(min(0.001, d.remaining(deadline)))
                if block is request:
                    self.sent = sent
            d.remaining(deadline)
            self.process.stdin.close()
        except BaseException as error:
            error.read_owner = self
            error.cleanup_receipt = self.cleanup(str(error))
            raise

    def _save(self, name, value, deadline):
        super()._save(name, value, deadline)
        observed = os.stat(name, dir_fd=self.directory.fd, follow_symlinks=False)
        raw = d.canonical(value) + b"\n"
        self.output_inventory[name] = {"bytes": len(raw), "sha256": d.sha(raw),
            "identity": (observed.st_dev, observed.st_ino),
            "times": (observed.st_mtime_ns, observed.st_ctime_ns)}

    def cleanup(self, reason="cleanup"):
        # TERM/KILL/wait/reap、关闭写句柄与回执仍完整走原 OwnedProcess。
        receipt = super().cleanup(reason)
        for name, (fd, identity) in list(self.output_readers.items()):
            try:
                d.remaining(self.seal_deadline)
                before = os.fstat(fd)
                d.require(stat.S_ISREG(before.st_mode) and (before.st_dev, before.st_ino) == identity,
                          "持有的原输出句柄身份变化")
                cap = d.positive(self.caps["reader"]["max_reply_bytes" if name == "stdout" else "max_stderr_bytes"])
                digest, count, prefix = hashlib.sha256(), 0, bytearray()
                # 文件仍在本实例取得的句柄上；目录被换也绝不读替换路径。
                while True:
                    d.remaining(self.seal_deadline)
                    block = os.read(fd, 65536)
                    if not block:
                        break
                    digest.update(block)
                    count += len(block)
                    prefix.extend(block[:max(0, cap + 1 - len(prefix))])
                    self.output_originals[name] = bytes(prefix)
                self.output_originals[name] = bytes(prefix)
                after = os.fstat(fd)
                d.require((before.st_size, before.st_mtime_ns, before.st_ctime_ns) ==
                          (after.st_size, after.st_mtime_ns, after.st_ctime_ns) and count == after.st_size,
                          "原输出封口中变化")
                self.output_inventory[name] = {"bytes": count, "sha256": digest.hexdigest(), "identity": identity,
                    "times": (after.st_mtime_ns, after.st_ctime_ns)}
                if count > cap:
                    self.output_errors.append(name + " 原输出超帽；Exchange 仅保留原前缀，完整原文件未改")
            except (d.Rejected, OSError, ValueError) as error:
                self.output_errors.append(name + ": " + type(error).__name__ + ": " + str(error))
            finally:
                os.close(fd)
                del self.output_readers[name]
        return receipt


class ProductTransport:
    """只接受根 grant 包装；真实 CLI/HTTP，无重试、代理或裸产品 subprocess.run。"""
    def __init__(self, gate, output, *, directory=None):
        d.require(type(gate) is ReadAdmission, "未准入零产品 IO")
        self.gate, self.output, self.directory = gate, Path(output), directory
        self._verify_directory()
        gate.verify_current()
        self.machine = None

    def _verify_directory(self):
        d.require(str(self.output) == self.gate.extension["output"], "输出目录未绑定根准入")
        d.require(type(self.directory) is d.OwnedDirectory and self.directory.path == self.output,
                  "产品传输缺本次独占目录句柄")
        self.directory.verify()

    def exchange(self, slot, request, wire, deadline):
        d.require(type(self.gate) is ReadAdmission, "产品传输缺准入")
        d.require(type(self.machine) is CaptureMachine and slot in self.gate.plan["slots"]
                  and request == self.machine.request(slot), "传输不接受任意 argv/payload/slot")
        expected_wire = http_request(self.gate.plan["q"]["endpoint"], request) if slot["route"] == "snapshot" else request
        d.require(wire == expected_wire and slot["id"] in self.gate.ledger.consumed
                  and slot["id"] not in self.gate.ledger.dispatched, "传输原字节不符/未消费/重试")
        self.gate.ledger.dispatched.add(slot["id"])
        result = Exchange(started=time.monotonic())
        try:
            self._verify_directory()
            self.gate.guard(slot, request, deadline, "before")
            self._verify_directory()
            if slot["route"] == "snapshot":
                result = self._http(slot, wire, deadline, result)
            else:
                result = self._cli(slot, request, deadline, result)
            d.remaining(deadline)
        except (d.Rejected, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
            result.error = type(error).__name__ + ": " + str(error)
            result.complete = False
        result.ended = time.monotonic()
        return result

    def _http(self, slot, wire, deadline, result):
        p = self.gate.plan
        endpoint = p["q"]["endpoint"]
        _endpoint(endpoint)
        raw = bytearray()
        cap = d.positive(p["resources"]["q"]["limits"]["max_reply_bytes"])
        job = slot["binding"]["job"]
        wire_left = d.positive(p["resources"]["jobs"][job]["limits"]["max_total_wire_bytes"]) - self.gate.ledger.used[job]["wire"]
        # q_runtime 固定五行头的语法上界，不是新增运行资源帽。
        header_limit = len(b"HTTP/1.0 503 Service Unavailable\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: \r\nCache-Control: no-store\r\nConnection: close\r\n\r\n") + max(31 - len("Service Unavailable"), 0) + len(str(cap))
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as channel:
                channel.settimeout(d.remaining(deadline))
                channel.connect((endpoint["host"], endpoint["port"]))
                channel.settimeout(d.remaining(deadline))
                channel.sendall(wire)
                while b"\r\n\r\n" not in raw:
                    channel.settimeout(d.remaining(deadline))
                    chunk = channel.recv(1)
                    d.require(bool(chunk), "HTTP 头未完成便断开")
                    raw.extend(chunk)
                    d.remaining(deadline)
                    d.require(len(raw) <= wire_left, "Q 原回复超过所属 Job 剩余 wire")
                    d.require(len(raw) <= header_limit, "HTTP 头偏离固定协议")
                head = bytes(raw)
                lines = head[:-4].split(b"\r\n")
                lengths = [line[16:] for line in lines if line.startswith(b"Content-Length: ")]
                d.require(len(lengths) == 1, "HTTP 长度头不唯一")
                count = x.decimal(lengths[0].decode("ascii"))
                d.require(count <= cap, "Q 声明回复超过原对象帽")
                while len(raw) < len(head) + count:
                    channel.settimeout(d.remaining(deadline))
                    chunk = channel.recv(min(65536, len(head) + count - len(raw), wire_left + 1 - len(raw)))
                    d.require(bool(chunk), "HTTP body 截断")
                    raw.extend(chunk)
                    d.remaining(deadline)
                    d.require(len(raw) <= wire_left, "Q 原回复超过所属 Job 剩余 wire")
                channel.settimeout(d.remaining(deadline))
                extra = channel.recv(1)
                raw.extend(extra)
                d.require(not extra, "HTTP 尾随字节/连接未关闭")
                status, result.reply = http_reply(bytes(raw))
                result.identity = {"owner": copy.deepcopy(p["q"]["identity"]), "endpoint": endpoint, "http_status": status}
                result.complete = True
        finally:
            result.transport_reply = bytes(raw)
            if not result.reply and b"\r\n\r\n" in raw:
                result.reply = bytes(raw).split(b"\r\n\r\n", 1)[1]
        return result

    def _cli(self, slot, request, deadline, result):
        owner = None
        try:
            owner = _StdinProcess(self.gate, slot, request, self.output / slot["id"], deadline)
            caps = self.gate.plan["resources"]["reader"]
            job = slot["binding"]["job"]
            wire_left = d.positive(self.gate.plan["resources"]["jobs"][job]["limits"]["max_total_wire_bytes"]) - self.gate.ledger.used[job]["wire"]
            while owner.process.poll() is None:
                d.remaining(deadline)
                d.require(os.fstat(owner.stdout.fileno()).st_size <= d.positive(caps["max_reply_bytes"])
                          and os.fstat(owner.stderr.fileno()).st_size <= d.positive(caps["max_stderr_bytes"]), "reader 原输出超帽")
                d.require(os.fstat(owner.stdout.fileno()).st_size + os.fstat(owner.stderr.fileno()).st_size <= wire_left, "reader 原输出超过所属 Job 剩余 wire")
                time.sleep(min(0.001, d.remaining(deadline)))
            result.returncode = owner.process.wait(timeout=d.remaining(deadline))
            result.identity = {"owner": copy.deepcopy(owner.identity), "argv": owner.argv,
                               "stage": owner.stage,
                               "binary_sha256_before": owner.binary_before,
                               "binary_sha256_after": d.bounded_hash(owner.argv[0], deadline), "reaped": True}
            d.require(result.identity["binary_sha256_after"] == owner.binary_before, "reader 退出 binary 漂移")
            result.complete = True
        except BaseException as error:
            owner = owner or getattr(error, "read_owner", None)
            raise
        finally:
            if owner is not None:
                result.identity.setdefault("owner", copy.deepcopy(owner.identity))
                receipt = owner.cleanup("read-capture-finished")
                result.identity["cleanup"] = receipt
                result.returncode = receipt["returncode"]
                for key, filename in (("reply", "stdout"), ("stderr", "stderr")):
                    setattr(result, key, owner.output_originals.get(filename, b""))
                result.identity["output_errors"] = list(owner.output_errors)
                result.transport_reply = result.reply
                if not receipt["reaped"] or receipt["errors"] or owner.output_errors:
                    result.complete = False
        return result


def capture_product(gate, output):
    """唯一生产入口。没有有效 ReadAdmission 时，先于目录/Popen/HTTP 拒绝。"""
    d.require(type(gate) is ReadAdmission, "execution_authorized=false；缺根精确 ReadAdmission")
    gate.preflight()
    d.require(gate.plan["capture_id"] not in gate.ledger.capture_ids, "同 grant/capture 不得再次运行")
    d.require(str(Path(output)) == gate.extension["output"], "输出目录未绑定根准入")
    gate.verify_current()
    directory = d.OwnedDirectory(output)
    try:
        transport = ProductTransport(gate, output, directory=directory)
        machine = CaptureMachine(gate.plan, gate.ledger, transport)
        result = machine.run()
        result["originals"] = persist_originals(machine, result, directory, gate.admission.seal_deadline)
        return result, machine
    finally:
        directory.close()


def persist_originals(machine, result, directory, deadline):
    """逐原件清单仅记观测摘要，不生成 root expected_sha/批准或自产 export 合同。"""
    d.require(type(directory) is d.OwnedDirectory, "原件保存缺独占新目录")
    inventory = []

    def save(name, raw):
        d.remaining(deadline)
        directory.verify()
        with directory.open_new(name) as stream:
            stream.write(raw)
            stream.flush()
        d.remaining(deadline)
        inventory.append({"path": name, "bytes": len(raw), "sha256": d.sha(raw)})

    for i, original in enumerate(machine.originals):
        exchange = original["exchange"]
        prefix = f"capture-{i:06d}"
        for suffix, raw in (("request", original["request"]), ("transport-request", original["transport_request"]),
                            ("reply", exchange.reply), ("transport-reply", exchange.transport_reply), ("stderr", exchange.stderr)):
            save(prefix + "." + suffix, raw)
        save(prefix + ".json", d.canonical({"slot": original["slot"], "deadline": original["deadline"],
             "started": exchange.started, "ended": exchange.ended, "complete": exchange.complete,
             "identity": exchange.identity, "returncode": exchange.returncode, "error": exchange.error}) + b"\n")
    if type(machine.transport) is ProductTransport:
        for i, guard in enumerate(machine.transport.gate.guards):
            save(f"guard-{i:06d}.request", guard["request"])
            save(f"guard-{i:06d}.result.json", d.canonical(guard["result"]) + b"\n")
        gate = machine.transport.gate
        for slot in machine.plan["slots"]:
            owner = gate.admission.active_owners.get((slot["stage"], "S", machine.plan["anchor"]["writer_epoch"]))
            if type(owner) is not _StdinProcess:
                continue
            # 摘要来自持有的原句柄。这里只核路径仍指向原 inode，不读取路径中的任何内容。
            fd = None
            try:
                directory.verify()
                d.remaining(deadline)
                d.require(owner.directory is not None and owner.output == directory.path / slot["id"],
                          "原进程目录不属于本次输出")
                fd = os.open(slot["id"], os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory.fd)
                observed = os.fstat(fd)
                d.require((observed.st_dev, observed.st_ino) == owner.directory.identity, "原进程目录被替换")
                for filename, item in owner.output_inventory.items():
                    observed = os.stat(filename, dir_fd=fd, follow_symlinks=False)
                    d.require(stat.S_ISREG(observed.st_mode) and (observed.st_dev, observed.st_ino) == item["identity"]
                              and observed.st_size == item["bytes"]
                              and (observed.st_mtime_ns, observed.st_ctime_ns) == item["times"], "原进程文件被替换")
                    inventory.append({"path": slot["id"] + "/" + filename,
                                      "bytes": item["bytes"], "sha256": item["sha256"]})
            except (d.Rejected, OSError) as error:
                result.setdefault("original_path_errors", []).append(str(error))
            finally:
                if fd is not None:
                    os.close(fd)
    save("RESULT.json", d.canonical(result) + b"\n")
    # 不在清单内列清单自身；不包含未来 commit。
    with directory.open_new("ORIGINALS.json") as stream:
        stream.write(d.canonical({"status": "UNAUTHENTICATED_ORIGINALS", "files": inventory}) + b"\n")
    d.remaining(deadline)
    return inventory
