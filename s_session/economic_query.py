"""#1374：只读组合独立 E/B 权威图像，不持写连接、不运行经济 reducer。"""

from collections import OrderedDict
from collections.abc import Mapping
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import base64
import copy
import hashlib
import json
from pathlib import Path
import re
import threading
import time
import uuid

from s_socket_client import canonical, decode_frame, digest, exchange, validate_reply


class EconomicQueryError(ValueError):
    pass


def integer(value, field, *, positive=False):
    if type(value) is not str or not re.fullmatch(r"0|[1-9][0-9]*", value):
        raise EconomicQueryError(field + " 须为规范非负整数文本")
    result = int(value)
    if result > 2**63 - 1 or (positive and result == 0):
        raise EconomicQueryError(field + " 越出范围")
    return result


def text(value, field):
    if type(value) is not str or not value:
        raise EconomicQueryError(field + " 缺失")
    return value


def verify_cut(cut, domain):
    if type(cut) is not dict or set(cut) != {"domain_id", "commit_seq", "root_hash"}:
        raise EconomicQueryError("经济 cut 字段不完整或多余")
    if cut["domain_id"] != domain:
        raise EconomicQueryError("经济 cut 属于其他域")
    integer(cut["commit_seq"], "commit_seq")
    if type(cut["root_hash"]) is not str or not re.fullmatch(r"[0-9a-f]{64}", cut["root_hash"]):
        raise EconomicQueryError("经济 cut 缺少完整根摘要")
    return cut


def economic_revisions(manifest: Mapping[str, object]) -> tuple[str, str]:
    """按冻结 manifest 分派，与 Rust Config 域选择同义；不从响应猜版本。"""
    protocol = manifest.get("protocol_revision")
    if protocol is None:
        protocol = "economic-session/1"
    if type(protocol) is not str or protocol not in ("economic-session/1", "economic-session/2"):
        raise EconomicQueryError("未知经济会话版本")
    finance_revision = "tb03b-finance/2" if protocol == "economic-session/2" else "tb03b-finance/1"
    finance = manifest.get("finance")
    if finance is None:
        if protocol == "economic-session/2":
            raise EconomicQueryError("economic-session/2 必须冻结 tb03b-finance/2")
    elif type(finance) is not dict:
        raise EconomicQueryError("经济会话与财务版本混搭或未知")
    elif protocol == "economic-session/1" and finance.get("schema_revision") == "tb03b-finance/3":
        # #1716 I1-C：/1 域显式选择的新合同（签署 cap 原子激活）；/1、/2 不变。
        finance_revision = "tb03b-finance/3"
    elif finance.get("schema_revision") != finance_revision:
        raise EconomicQueryError("经济会话与财务版本混搭或未知")
    return protocol, finance_revision


def book_revision(manifest):
    """#1406/#1620：null/缺省仍属旧书账。Rust book::rules 认 tb03c-book/1 与 /2（C8 资金签署）；
    本读者接受两者——/2 只在公共查询字段上与 /1 等效（不处理资金合同，只读取公共图像）。"""
    revision = manifest.get("book_revision")
    if revision is not None and (type(revision) is not str or revision not in ("tb03c-book/1", "tb03c-book/2")):
        raise EconomicQueryError("未知书账版本")
    return revision


def _exact(value, fields, name):
    if type(value) is not dict or set(value) != fields:
        raise EconomicQueryError(name + " 字段不完整或多余")


def _hash(value, name):
    if type(value) is not str or re.fullmatch(r"[0-9a-f]{64}", value) is None:
        raise EconomicQueryError(name + " 不是完整摘要")


def _signed_integer(value, name):
    if type(value) is not str or re.fullmatch(r"0|-?[1-9][0-9]*", value) is None:
        raise EconomicQueryError(name + " 须为规范整数文本")
    if not -(2**63) <= int(value) < 2**63:
        raise EconomicQueryError(name + " 越出范围")


def allocation_sources(manifest, revision):
    """冻结来源身份，不实现份额或金额 reducer。无政策的清单仍可等待。
    #1620：tb03c-book/2 在外效分配字段上与 /1 等效（资金签署不影响此读者）。"""
    if "fill_manifests" not in manifest and "fill_allocation_policy" not in manifest:
        return None, {}
    if revision not in ("tb03c-book/1", "tb03c-book/2"):
        raise EconomicQueryError("外效分配来源须冻结 tb03c-book/1 或 tb03c-book/2")
    policy = manifest.get("fill_allocation_policy")
    policies = {"netted": "frozen-manifest-proportional-cumulative/2",
                "per_leg": "per-leg-source-attribution/1"}
    if "fill_allocation_policy" in manifest:
        _exact(policy, {"fill_allocation_policy_id", "policy_revision"}, "分配政策")
        if policy["fill_allocation_policy_id"] not in policies.values():
            raise EconomicQueryError("未知分配政策")
        text(policy["policy_revision"], "policy_revision")
    manifests = manifest.get("fill_manifests")
    if type(manifests) is not list:
        raise EconomicQueryError("fill_manifests 须为数组")
    result = {}
    for source in manifests:
        _exact(source, {"projection_kind", "symbol", "participants", "tie_break_order", "manifest_hash"}, "投影清单")
        kind = source["projection_kind"]
        if type(kind) is not str or kind not in policies:
            raise EconomicQueryError("未知投影类型")
        if policy is not None and policy["fill_allocation_policy_id"] != policies[kind]:
            raise EconomicQueryError("投影与分配政策混搭")
        text(source["symbol"], "symbol")
        _hash(source["manifest_hash"], "manifest_hash")
        if (source["manifest_hash"] != digest({k: v for k, v in source.items() if k != "manifest_hash"})
                or source["manifest_hash"] in result):
            raise EconomicQueryError("投影清单摘要不同或重复")
        if type(source["participants"]) is not list or not source["participants"]:
            raise EconomicQueryError("投影参与重缺失")
        ids = []
        for participant in source["participants"]:
            _exact(participant, {"chong_id", "command_id", "family_id", "revision", "requested_q"}, "参与重")
            for field in ("chong_id", "command_id", "family_id", "revision"):
                text(participant[field], field)
            integer(participant["requested_q"], "requested_q", positive=True)
            matches = [r for r in manifest.get("chongs", []) if type(r) is dict
                       and r.get("chong_id") == participant["chong_id"]]
            if len(matches) != 1 or matches[0].get("symbol") != source["symbol"]:
                raise EconomicQueryError("投影参与重与冻结路由不同")
            ids.append(participant["chong_id"])
        if len(set(ids)) != len(ids) or source["tie_break_order"] != sorted(ids):
            raise EconomicQueryError("投影参与重重复或并列顺序不同")
        result[source["manifest_hash"]] = copy.deepcopy(source)
    return copy.deepcopy(policy), result


_SOURCE_ID = ("source_namespace", "source_epoch", "external_fact_id")


def _source_key(source):
    if type(source) is not dict:
        raise EconomicQueryError("事实来源身份缺失")
    return digest([text(source.get(k), k) for k in _SOURCE_ID])


def _original_event(record):
    """book::fact_key + service::record_from_payload 的只读身份核验。"""
    raw = text(record.get("raw_utf8"), "raw_utf8")
    raw_hash = record.get("raw_sha256")
    _hash(raw_hash, "raw_sha256")
    pointer = text(record.get("json_pointer"), "json_pointer")
    if not pointer.startswith("/events/"):
        raise EconomicQueryError("事实指针须定位 events 原件")
    try:
        raw_bytes = raw.encode("utf-8")
        if hashlib.sha256(raw_bytes).hexdigest() != raw_hash:
            raise EconomicQueryError("原件字节与 raw_sha256 不同")
        document = decode_frame(raw_bytes)
        # serde_json 的整数域；经济原件不接受浮点或任意精度 JSON 数字。
        pending = [document]
        while pending:
            value = pending.pop()
            if type(value) is dict:
                pending.extend(value.values())
            elif type(value) is list:
                pending.extend(value)
            elif type(value) is int and not -(2**63) <= value < 2**64:
                raise EconomicQueryError("原件 JSON 整数越出生产域")
        if (document.get("format") != "neutral TestOnly economic raw originals; not production transport wire"
                or type(document.get("units")) is not dict):
            raise EconomicQueryError("原件 profile/单位缺失")
        selected = document
        for token in pointer[1:].split("/"):
            token = token.replace("~1", "/").replace("~0", "~")
            if type(selected) is dict and token in selected:
                selected = selected[token]
            elif (type(selected) is list and re.fullmatch(r"0|[1-9][0-9]*", token)
                  and int(token) < len(selected)):
                selected = selected[int(token)]
            else:
                raise EconomicQueryError("事实指针不可解析")
        event = record.get("event")
        if type(selected) is not dict or canonical(selected) != canonical(event):
            raise EconomicQueryError("事实事件与原件指针不同")
        text(event.get("allocation_id"), "allocation_id")
        if record.get("fact_key") != _source_key(event.get("source")):
            raise EconomicQueryError("事实身份与原来源三元组不同")
        content_hash = digest({"raw_sha256": raw_hash, "json_pointer": pointer,
                               "event": event, "units": document["units"]})
        if record.get("fact_hash") != content_hash:
            raise EconomicQueryError("事实内容摘要与原件/指针/事件/单位不同")
        return event
    except (ValueError, UnicodeError, TypeError, OverflowError) as exc:
        raise EconomicQueryError("事实原件绑定无效：" + str(exc)) from exc


def _correction_predecessor(row, event, rows, verified_event):
    """只对应具名前件和政策引用；不重放累计前沿或计算逆份额。"""
    report = event["netted_fill"]
    reference, dependency = report.get("fill_ref"), event.get("depends_on")
    _exact(reference, set(_SOURCE_ID) | {"allocation_id", "manifest_hash",
           "fill_allocation_policy_id", "policy_revision"}, "更正 fill_ref")
    _exact(dependency, set(_SOURCE_ID) | {"source_ref"}, "更正 depends_on")
    for key in reference:
        text(reference[key], key)
    text(dependency["source_ref"], "depends_on.source_ref")
    key = _source_key(dependency)
    if (key != _source_key(reference) or key != row["original_fact_key"]
            or key == row["fact_key"]):
        raise EconomicQueryError("更正具名前件与原件依赖不同")
    original = verified_event(key)
    fill = original.get("netted_fill")
    if (type(fill) is not dict or fill.get("report_kind") not in ("incremental", "cumulative")
            or integer(fill.get("from"), "原 fill.from") >= integer(fill.get("to"), "原 fill.to")):
        raise EconomicQueryError("更正前件不是普通非零净额 fill")
    if (any(reference[k] != original["source"][k] for k in _SOURCE_ID)
            or reference["allocation_id"] != original.get("allocation_id")
            or reference["manifest_hash"] != fill.get("manifest_hash")
            or report["symbol"] != fill.get("symbol")):
        raise EconomicQueryError("更正原 allocation/清单/标的不同")
    original_rows = [r for r in rows if type(r) is dict and r.get("fact_key") == key]
    if len(original_rows) != 1:
        raise EconomicQueryError("更正缺唯一原 fill 分配")
    original_row = original_rows[0]
    for field in ("manifest_hash", "fill_allocation_policy_id", "policy_revision"):
        if reference[field] != original_row.get(field) or reference[field] != row[field]:
            raise EconomicQueryError("更正政策引用与具名原分配不同")


def verify_fill_allocations(image, policy, sources):
    """只核原图像的字段、类型与具名来源；返回时不改写、补齐或重算。"""
    if policy is None:
        raise EconomicQueryError("分配图像缺冻结政策")
    rows = image["fill_allocations"]
    if type(rows) is not list:
        raise EconomicQueryError("fill_allocations 须为数组")
    verified = {}

    def verified_event(key):
        if key not in verified:
            records = [r for r in image["recorded"]
                       if type(r) is dict and r.get("fact_key") == key]
            if len(records) != 1:
                raise EconomicQueryError("分配缺唯一具名事实")
            verified[key] = (_original_event(records[0]), records[0]["fact_hash"])
        return verified[key][0]

    seen = set()
    for row in rows:
        if type(row) is not dict:
            raise EconomicQueryError("分配记录须为对象")
        common = {"fact_key", "fact_hash", "manifest_hash", "symbol", "fill_allocation_policy_id",
                  "policy_revision", "evidence_id", "allocations"}
        per_leg = row.get("projection_kind") == "per_leg"
        correction = "report_kind" in row
        extra = {"projection_kind"} if per_leg else {"from", "to"}
        if correction:
            extra |= {"report_kind", "component", "original_fact_key"}
        _exact(row, common | extra, "分配记录")
        for field in ("fact_key", "fact_hash", "manifest_hash"):
            _hash(row[field], field)
        text(row["evidence_id"], "evidence_id")
        source = sources.get(row["manifest_hash"])
        if (source is None or source["symbol"] != row["symbol"]
                or source["projection_kind"] != ("per_leg" if per_leg else "netted")
                or any(row[k] != policy[k] for k in policy)):
            raise EconomicQueryError("分配记录与冻结清单/政策不同")
        if row["fact_key"] in seen:
            raise EconomicQueryError("分配事实重复")
        seen.add(row["fact_key"])
        event = verified_event(row["fact_key"])
        if verified[row["fact_key"]][1] != row["fact_hash"]:
            raise EconomicQueryError("分配事实内容身份不同")
        report = event.get("per_leg_report" if per_leg else "netted_fill") if type(event) is dict else None
        if (type(report) is not dict or report.get("manifest_hash") != row["manifest_hash"]
                or report.get("symbol") != row["symbol"]):
            raise EconomicQueryError("分配记录与原事实投影来源不同")
        evidence = [h for h in image["history"] if type(h) is dict and h.get("kind") == "ResolveEvidence"
                    and h.get("evidence_id") == row["evidence_id"]]
        if (len(evidence) != 1 or type(evidence[0].get("covered")) is not list
                or row["fact_key"] not in evidence[0]["covered"]):
            raise EconomicQueryError("分配缺具名 Resolve 原史")
        if not per_leg:
            integer(row["from"], "from")
            integer(row["to"], "to")
        if correction:
            expected = {"fee_only": "fee_only", "bust": "acquisition_cash", "reverse_component": "commission"}
            if (per_leg or type(row["report_kind"]) is not str
                    or row["report_kind"] not in expected
                    or expected[row["report_kind"]] != row["component"]
                    or report.get("report_kind") != row["report_kind"]
                    or (row["report_kind"] in ("bust", "reverse_component")
                        and report.get("component") != row["component"])):
                raise EconomicQueryError("更正类型与原事实不同")
            _hash(row["original_fact_key"], "original_fact_key")
            _correction_predecessor(row, event, rows, verified_event)
        elif not per_leg and report.get("report_kind") not in ("incremental", "cumulative"):
            raise EconomicQueryError("普通分配原事实类型不同")
        if not per_leg:
            kind = report["report_kind"]
            if kind in ("incremental", "cumulative", "bust"):
                if row["from"] != report.get("from") or row["to"] != report.get("to"):
                    raise EconomicQueryError("分配区间与原报告不同")
                forward = int(row["to"]) > int(row["from"])
                if (kind == "bust" and int(row["to"]) >= int(row["from"])) or (kind != "bust" and not forward):
                    raise EconomicQueryError("分配区间方向与变体不同")
            elif row["from"] != row["to"]:
                raise EconomicQueryError("零数量更正改变了区间")
        allocations = row["allocations"]
        if type(allocations) is not list or len(allocations) != len(source["participants"]):
            raise EconomicQueryError("分配参与重不完整")
        for allocation, participant in zip(allocations, source["participants"]):
            identity = {"allocation_id", "chong_id", "command_id", "family_id", "revision"}
            amounts = {"units", "acquisition_cash", "commission"}
            fields = identity | amounts
            if per_leg:
                fields = identity | {"unmatched_reason", "manifest_hash", "fill_allocation_policy_id", "policy_revision"}
            elif correction:
                fields |= {"component", "reversal", "original_fact_key"}
                if row["component"] == "fee_only":
                    fields = (fields - {"commission"}) | {"fee_only"}
            _exact(allocation, fields, "逐重分配")
            if any(allocation[k] != participant[k] for k in identity - {"allocation_id"}):
                raise EconomicQueryError("逐重分配身份不同")
            prefix = "per-leg:" if per_leg else "netted:"
            expected_id = prefix + digest([row["fact_key"], event.get("allocation_id"), row["manifest_hash"], participant])
            if allocation["allocation_id"] != expected_id:
                raise EconomicQueryError("逐重分配原身份摘要不同")
            if per_leg:
                if (allocation["unmatched_reason"] != "per_leg_attribution_missing"
                        or allocation["manifest_hash"] != row["manifest_hash"]
                        or any(allocation[k] != policy[k] for k in policy)):
                    raise EconomicQueryError("逐腿未匹配来源不同")
            else:
                _signed_integer(allocation["units"], "units")
                units = int(allocation["units"])
                if ((kind in ("incremental", "cumulative") and units < 0)
                        or (kind == "bust" and units > 0)
                        or (kind in ("fee_only", "reverse_component") and units != 0)):
                    raise EconomicQueryError("分配 units 符号域与变体不同")
                for field in fields & {"acquisition_cash", "commission", "fee_only"}:
                    integer(allocation[field], field)
                if correction and (allocation["component"] != row["component"]
                                   or allocation["original_fact_key"] != row["original_fact_key"]
                                   or type(allocation["reversal"]) is not bool
                                   or allocation["reversal"] != (row["report_kind"] != "fee_only")):
                    raise EconomicQueryError("更正分配来源不同")


class EconomicQuery:
    """一个 Q 配置固定可信 owner；固定向量缓存只留完整已核图像。"""

    def __init__(self, config_path, *, wire_evidence=None):
        self.wire_evidence = wire_evidence
        path = Path(config_path).resolve()
        config = decode_frame(path.read_bytes())
        if config.get("schema_revision") != "economic-query/1":
            raise EconomicQueryError("需要 economic-query/1")
        manifest_path = (path.parent / text(config.get("manifest"), "manifest")).resolve()
        raw_manifest = manifest_path.read_bytes()
        manifest = decode_frame(raw_manifest)
        if manifest.get("schema_revision") != "economic-manifest/1":
            raise EconomicQueryError("需要绑定 economic-manifest/1")
        if digest(manifest) != config.get("manifest_hash"):
            raise EconomicQueryError("manifest 与已配置摘要不同")
        self.protocol_revision, self.finance_revision = economic_revisions(manifest)
        self.book_revision = book_revision(manifest)
        self.fill_policy, self.fill_sources = allocation_sources(manifest, self.book_revision)
        self.manifest = manifest
        self.manifest_hash = config["manifest_hash"]
        self.session = text(manifest.get("session_id"), "session_id")
        self.generation = manifest.get("session_generation")
        integer(self.generation, "session_generation", positive=True)
        self.source = config.get("query_source")
        source_fields = {"source_namespace", "source_epoch", "producer_id", "producer_epoch"}
        if type(self.source) is not dict or set(self.source) != source_fields:
            raise EconomicQueryError("Q 连接身份不在获准源内")
        matching = [entry for entry in manifest.get("authorized_sources", [])
                    if type(entry) is dict and {k: entry.get(k) for k in source_fields} == self.source]
        read_ops = {"ReadView", "Watch", "QueryFact", "QueryAllocation", "QueryVoice", "QueryDecision", "QueryGrant",
                    "EconomicReadOpen", "EconomicReadPage", "EconomicReadClose"}
        if (len(matching) != 1 or type(matching[0].get("operations")) is not list
                or any(type(op) is not str for op in matching[0]["operations"])
                or not {"ReadView", "Watch"} <= set(matching[0]["operations"]) <= read_ops):
            raise EconomicQueryError("Q 身份须明确只有只读操作权限")
        self.authorized_read_ops = frozenset(matching[0]["operations"])
        for key, value in self.source.items():
            text(value, key)
        integer(self.source["producer_epoch"], "producer_epoch", positive=True)
        self.query_epoch = config.get("producer_epoch")
        integer(self.query_epoch, "Q producer_epoch", positive=True)
        resources = config.get("resources")
        fields = {"max_frame_bytes", "timeout_ms", "operation_timeout_ms", "max_reply_bytes",
                  "max_cached_bytes", "max_snapshots", "max_domains", "max_changes"}
        if type(resources) is not dict or set(resources) != fields:
            raise EconomicQueryError("Q 资源必须逐项声明")
        self.resources = {k: integer(resources[k], k, positive=True) for k in fields}
        nodes = [("E", manifest.get("e"))]
        chongs = manifest.get("chongs")
        if type(chongs) is not list:
            raise EconomicQueryError("manifest.chongs 必须是数组")
        for node in chongs:
            if type(node) is not dict:
                raise EconomicQueryError("重路由必须是对象")
            nodes.append(("B:" + text(node.get("chong_id"), "chong_id"), node))
        if len(nodes) > self.resources["max_domains"] or len({d for d, _ in nodes}) != len(nodes):
            raise EconomicQueryError("域数量越界或身份重复")
        self.nodes = {}
        for domain, node in nodes:
            if type(node) is not dict:
                raise EconomicQueryError("域路由缺失")
            self.nodes[domain] = str((manifest_path.parent / text(node.get("socket"), "socket")).resolve())
        epochs = config.get("expected_epochs")
        if type(epochs) is not dict or set(epochs) != set(self.nodes):
            raise EconomicQueryError("须分别声明每个 owner 的获准 epoch")
        for domain, epoch in epochs.items():
            integer(epoch, domain + " epoch", positive=True)
        self.epochs = copy.deepcopy(epochs)
        self.lock = threading.Lock()
        self.cache = OrderedDict()
        self.cached_bytes = 0
        self.transfers = {}
        self.open_records = {}
        self.metadata_bytes = 0
        self.budget = threading.local()

    def _start(self, deadline=None):
        own_deadline = time.monotonic() + self.resources["operation_timeout_ms"] / 1000
        self.budget.deadline = own_deadline if deadline is None else min(own_deadline, deadline)
        self._remaining_ms()

    def _drop_transfer_locked(self, transfer_id):
        transfer = self.transfers.pop(transfer_id)
        key = transfer["cache_key"]
        record_key = transfer.get("record_key")
        if record_key is not None:
            record = self.open_records[record_key]
            terminal = {k: record[k] for k in ("identity", "message_id", "payload_hash", "transfer_id")}
            terminal["status"] = "closed"
            self._set_record_locked(record_key, terminal)
        if transfer["temporary"] and not any(t["cache_key"] == key for t in self.transfers.values()):
            encoded = self.cache.pop(key, None)
            if encoded is not None:
                self.cached_bytes -= len(encoded)

    def _expire_transfers_locked(self):
        now = time.monotonic()
        for transfer_id in list(self.transfers):
            if now >= self.transfers[transfer_id]["deadline"]:
                self._drop_transfer_locked(transfer_id)

    def _cache_put_locked(self, key, encoded):
        """Snapshot 与活跃 watch 共用唯一正文账，活跃传输不得被淘汰。"""
        self._expire_transfers_locked()
        if len(encoded) > self.resources["max_cached_bytes"]:
            raise EconomicQueryError("QueryBudgetExceeded：完整经济文档超过聚合容量")
        if key in self.cache:
            if self.cache[key] != encoded:
                raise EconomicQueryError("缓存身份对应不同正文")
            self.cache.move_to_end(key)
            return
        self._admit_locked(len(encoded), 1)
        self.cache[key] = encoded
        self.cached_bytes += len(encoded)

    def _admit_locked(self, extra_bytes, extra_items=0, protected=()):
        """先算全部驱逐，再修改；正文与所有 Open 终态元数据同账。"""
        pinned = {t["cache_key"] for t in self.transfers.values()} | set(protected)
        byte_limit = min(4194304, self.resources["max_cached_bytes"])
        item_limit = min(4, self.resources["max_snapshots"])
        size = self.cached_bytes + self.metadata_bytes + extra_bytes
        count = len(self.cache) + extra_items
        victims = []
        for key, encoded in self.cache.items():
            if size <= byte_limit and count <= item_limit:
                break
            if key not in pinned:
                victims.append(key)
                size -= len(encoded)
                count -= 1
        if size > byte_limit or count > item_limit:
            raise EconomicQueryError("QueryBudgetExceeded：正文或元数据达到共享缓存上限")
        for key in victims:
            self.cached_bytes -= len(self.cache.pop(key))

    def _set_record_locked(self, key, record):
        old = self.open_records.get(key)
        delta = len(canonical(record)) - (len(canonical(old)) if old is not None else 0)
        self._admit_locked(delta)
        self.open_records[key] = record
        self.metadata_bytes += delta

    @staticmethod
    def _read_error(code, detail, transfer_id=None):
        return {"schema_revision": "economic-read-pages/1", "kind": "EconomicReadError",
                "code": code, "detail": detail, "transfer_id": transfer_id}

    def read(self, payload, request, *, deadline=None):
        """#1406：已验证公共头后的只读页入口；错误仍保留合法页信封。"""
        transfer_id = payload.get("transfer_id") if type(payload) is dict else None
        if type(transfer_id) is not str or re.fullmatch(r"[0-9a-f]{32}", transfer_id) is None:
            transfer_id = None
        try:
            return self._read(payload, request, deadline=deadline)
        except (ValueError, KeyError, TypeError, UnicodeError, RecursionError, OSError) as exc:
            detail = str(exc)
            code = "QueryBudgetExceeded" if "QueryBudgetExceeded" in detail else "InvalidQuery"
            return self._read_error(code, detail, transfer_id)

    def _read(self, payload, request, *, deadline=None):
        revision = "economic-read-pages/1"
        if type(payload) is not dict or payload.get("schema_revision") != revision:
            return self._read_error("SchemaUnsupported", "需要 economic-read-pages/1")
        op = payload.get("op")
        fields = {
            "EconomicReadOpen": {"schema_revision", "op", "query", "requested_remaining_ms"},
            "EconomicReadPage": {"schema_revision", "op", "transfer_id", "descriptor_sha256", "offset", "requested_remaining_ms"},
            "EconomicReadClose": {"schema_revision", "op", "transfer_id", "descriptor_sha256"},
        }
        if op not in fields:
            return self._read_error("SchemaUnsupported", "未知只读页操作")
        _exact(payload, fields[op], "分页请求")
        identity = {k: request[k] for k in ("session_id", "session_generation", "source_namespace",
                                           "source_epoch", "producer_id", "producer_epoch")}
        if op == "EconomicReadOpen":
            remaining = integer(payload["requested_remaining_ms"], "requested_remaining_ms", positive=True)
            if remaining > 30000:
                raise EconomicQueryError("requested_remaining_ms 超过30000")
            local_deadline = time.monotonic() + remaining / 1000
            self._start(local_deadline if deadline is None else min(local_deadline, deadline))
            logical_query = payload["query"]
            if type(logical_query) is not dict or logical_query.get("op") not in ("snapshot", "watch"):
                return self._read_error("SchemaUnsupported", "分页只支持原 snapshot/watch 选择")
            # 请求键保存原文身份，终态不删除；不可让相同 Open 在关闭后复活。
            record_key = tuple(identity.values()) + (request["message_id"],)
            with self.lock:
                self._expire_transfers_locked()
                existing = self.open_records.get(record_key)
                if existing is not None:
                    if existing["payload_hash"] != request["payload_hash"]:
                        return self._read_error("IdentityConflict", "同 Open 身份内容不同")
                    if existing["status"] == "active":
                        return existing["opened"]
                    return self._read_error("SourceUnavailable" if existing["status"] == "opening" else "TransferExpired",
                                            "原 Open 未完成或已经终止", existing["transfer_id"])
                transfer_id = uuid.uuid4().hex
                record = {"identity": identity, "message_id": request["message_id"],
                          "payload_hash": request["payload_hash"], "transfer_id": transfer_id, "status": "opening"}
                self._set_record_locked(record_key, record)
            committed = False
            try:
                operation_deadline = self.budget.deadline
                if logical_query["op"] == "snapshot":
                    body = self.snapshot(logical_query, deadline=operation_deadline, paged=True, cache=False)
                else:
                    body = self.watch(logical_query, deadline=operation_deadline, paged=True)
                self._remaining_ms()
                if body.get("kind") == "Gap":
                    return self._read_error("SnapshotExpired", "原 snapshot token 已失效", transfer_id)
                expected_kind = "EconomicSnapshot" if logical_query["op"] == "snapshot" else "EconomicWatch"
                if body.get("kind") != expected_kind or set(body.get("domains", {})) != set(self.nodes):
                    return self._read_error("IntegrityError", "完整逻辑文档类型或域集合错误", transfer_id)
                encoded = canonical(body)
                if len(encoded) > min(4194304, self.resources["max_cached_bytes"]):
                    raise EconomicQueryError("QueryBudgetExceeded：完整逻辑正文超过聚合帽")
                fixed = {}
                for domain, view in body["domains"].items():
                    if view.get("status") == "Available":
                        fixed[domain] = verify_cut(view.get("cut" if expected_kind == "EconomicSnapshot" else "next_cut"), domain)
                    elif view.get("status") == "Unavailable":
                        fixed[domain] = None
                    else:
                        return self._read_error("IntegrityError", "域可用状态非法", transfer_id)
                descriptor = {
                    "schema_revision": revision, "transfer_id": transfer_id,
                    "session_id": self.session, "session_generation": self.generation,
                    "manifest_hash": self.manifest_hash, "producer_id": "Q:" + self.session,
                    "producer_epoch": self.query_epoch, "query_sha256": digest(logical_query),
                    "document_kind": expected_kind, "fixed_cuts": fixed,
                    "owner_epochs": dict(self.epochs), "document_bytes": str(len(encoded)),
                    "document_sha256": hashlib.sha256(encoded).hexdigest(), "chunk_bytes": "262144",
                    "page_count": str((len(encoded) + 262143) // 262144),
                }
                opened = {"schema_revision": revision, "kind": "EconomicReadOpened", "descriptor": descriptor,
                          "descriptor_sha256": digest(descriptor)}
                temporary = expected_kind == "EconomicWatch"
                cache_key = "economic-watch-" + transfer_id if temporary else body["snapshot_token"]
                active = {**record, "status": "active", "opened": opened, "cache_key": cache_key,
                          "temporary": temporary, "deadline_ns": str(int(operation_deadline * 1000000000))}
                with self.lock:
                    self._expire_transfers_locked()
                    self._remaining_ms()
                    old_bytes = self.cache.get(cache_key)
                    if old_bytes is not None and old_bytes != encoded:
                        return self._read_error("IntegrityError", "正文缓存身份冲突", transfer_id)
                    delta_meta = len(canonical(active)) - len(canonical(self.open_records[record_key]))
                    self._admit_locked(delta_meta + (len(encoded) if old_bytes is None else 0),
                                       int(old_bytes is None), protected=(cache_key,))
                    if old_bytes is None:
                        self.cache[cache_key] = encoded
                        self.cached_bytes += len(encoded)
                    self.cache.move_to_end(cache_key)
                    self.open_records[record_key] = active
                    self.metadata_bytes += delta_meta
                    self.transfers[transfer_id] = {"record_key": record_key, "cache_key": cache_key,
                                                   "temporary": temporary, "deadline": operation_deadline}
                    committed = True
                self._remaining_ms()
                return opened
            finally:
                if not committed:
                    with self.lock:
                        failed = {**record, "status": "closed"}
                        self._set_record_locked(record_key, failed)
        transfer_id, descriptor_hash = payload["transfer_id"], payload["descriptor_sha256"]
        if type(transfer_id) is not str or re.fullmatch(r"[0-9a-f]{32}", transfer_id) is None:
            raise EconomicQueryError("transfer_id 格式错误")
        if type(descriptor_hash) is not str or re.fullmatch(r"[0-9a-f]{64}", descriptor_hash) is None:
            raise EconomicQueryError("descriptor_sha256 格式错误")
        with self.lock:
            self._expire_transfers_locked()
            transfer = self.transfers.get(transfer_id)
            closed = {"schema_revision": revision, "kind": "EconomicReadClosed", "transfer_id": transfer_id,
                      "descriptor_sha256": descriptor_hash}
            if transfer is None:
                return closed if op == "EconomicReadClose" else self._read_error("TransferExpired", "读取传输已失效", transfer_id)
            record = self.open_records[transfer["record_key"]]
            if record["identity"] != identity or record["opened"]["descriptor_sha256"] != descriptor_hash:
                return self._read_error("IdentityConflict", "读取传输来源或描述符不同", transfer_id)
            if op == "EconomicReadClose":
                self._drop_transfer_locked(transfer_id)
                return closed
            remaining = integer(payload["requested_remaining_ms"], "requested_remaining_ms", positive=True)
            if remaining > 30000:
                raise EconomicQueryError("requested_remaining_ms 超过30000")
            page_deadline = min(transfer["deadline"], time.monotonic() + remaining / 1000)
            self._start(page_deadline if deadline is None else min(page_deadline, deadline))
            offset = integer(payload["offset"], "offset")
            encoded = self.cache[transfer["cache_key"]]
            if offset >= len(encoded) or offset % 262144:
                raise EconomicQueryError("页 offset 必须是正文内固定步长边界")
            chunk = encoded[offset:offset + 262144]
            end = offset + len(chunk)
            result = {"schema_revision": revision, "kind": "EconomicReadPage", "transfer_id": transfer_id,
                      "descriptor_sha256": descriptor_hash, "offset": str(offset), "raw_byte_count": str(len(chunk)),
                      "chunk_sha256": hashlib.sha256(chunk).hexdigest(), "data_base64": base64.b64encode(chunk).decode("ascii"),
                      "next_offset": None if end == len(encoded) else str(end), "done": end == len(encoded)}
            self._remaining_ms()
            return result

    def _remaining_ms(self):
        deadline = getattr(self.budget, "deadline", None)
        if deadline is None:
            self._start()
            deadline = self.budget.deadline
        left = int((deadline - time.monotonic()) * 1000)
        if left <= 0:
            raise EconomicQueryError("QueryBudgetExceeded：完整经济查询达到总期限")
        return min(left, self.resources["timeout_ms"])

    @contextmanager
    def capture(self, request, deadline):
        scope = None if self.wire_evidence is None else self.wire_evidence.scope(request, deadline)
        self.budget.wire_scope = scope
        error = None
        try:
            yield
        except BaseException as exc:
            error = exc
            raise
        finally:
            self.budget.wire_scope = None
            if scope is not None:
                try:
                    scope.close(None if error is None else type(error).__name__)
                    if time.monotonic() >= deadline:
                        raise EconomicQueryError("原wire封存达到公共请求原期限")
                except (OSError, ValueError) as evidence_error:
                    if error is None:
                        raise
                    error.add_note("原wire封存也失败：" + str(evidence_error))

    def _parallel_domains(self, operation):
        deadline = self.budget.deadline
        paged = getattr(self.budget, "paged", False)
        wire_scope = getattr(self.budget, "wire_scope", None)

        def run(domain):
            self.budget.deadline = deadline
            self.budget.paged = paged
            self.budget.wire_scope = wire_scope
            try:
                return operation(domain)
            except (ValueError, KeyError, TypeError, OSError, RecursionError) as exc:
                return {"status": "Unavailable", "error": str(exc)}

        # 每域同时开始且各自有界；慢域不会消耗健康域尚未开始的读取机会。
        with ThreadPoolExecutor(max_workers=len(self.nodes)) as pool:
            return dict(zip(self.nodes, pool.map(run, self.nodes)))

    def request(self, payload):
        return {
            "schema_revision": self.protocol_revision, "session_id": self.session,
            "session_generation": self.generation, **self.source,
            "message_id": "query-" + uuid.uuid4().hex, "payload_hash": digest(payload),
            "causal_refs": [], "payload": payload,
        }

    def owner(self, domain, payload):
        if getattr(self.budget, "paged", False) and payload.get("op") in ("ReadView", "Watch"):
            from s_socket_client import read_economic_pages
            required = {"EconomicReadOpen", "EconomicReadPage", "EconomicReadClose"}
            if not required <= self.authorized_read_ops:
                raise EconomicQueryError("Q 来源未明确获准分页只读操作")
            def fetch(page_payload, remaining_ms):
                request = self.request(page_payload)
                wire = exchange(self.nodes[domain], request,
                                timeout_ms=min(remaining_ms, self.resources["timeout_ms"]),
                                max_frame_bytes=min(1048576, self.resources["max_frame_bytes"]),
                                evidence=getattr(self.budget, "wire_scope", None),
                                logical_deadline=self.budget.deadline)
                if wire["transport"] != "Received":
                    raise EconomicQueryError(wire["transport"] + ": " + wire.get("detail", ""))
                return validate_reply(request, wire["response"], producer=domain,
                                      producer_epoch=self.epochs[domain])
            result = read_economic_pages(
                payload, fetch, session_id=self.session, session_generation=self.generation,
                producer_id=domain + ":" + self.session, producer_epoch=self.epochs[domain],
                manifest_hash=self.manifest_hash, owner_epochs={domain: self.epochs[domain]},
                deadline=self.budget.deadline,
                max_frame_bytes=min(1048576, self.resources["max_frame_bytes"]),
                max_document_bytes=min(4194304, self.resources["max_cached_bytes"]))
            if result.get("domain_id") != domain:
                raise EconomicQueryError("权威分页内容与获准连接不同")
            self._remaining_ms()
            return result
        request = self.request(payload)
        result = exchange(self.nodes[domain], request, timeout_ms=self._remaining_ms(),
                          max_frame_bytes=self.resources["max_frame_bytes"],
                          evidence=getattr(self.budget, "wire_scope", None),
                                logical_deadline=self.budget.deadline)
        if result["transport"] != "Received":
            raise EconomicQueryError(result["transport"] + ": " + result.get("detail", ""))
        response = validate_reply(request, result["response"], producer=domain,
                                  producer_epoch=self.epochs[domain])
        if response.get("ok") is False or "error" in response:
            raise EconomicQueryError(str(response.get("error", "权威域不可用")))
        if response.get("domain_id") != domain:
            raise EconomicQueryError("权威域内容与获准连接不同")
        return response

    def view(self, domain, cut=None, as_known_ns=None):
        if cut is not None:
            verify_cut(cut, domain)
        if as_known_ns is not None:
            integer(as_known_ns, "as_known_ns")
        response = self.owner(domain, {"op": "ReadView", "cut": cut, "as_known_ns": as_known_ns})
        if response.get("kind") != "EconomicView":
            raise EconomicQueryError("owner 未返回完整经济图像")
        verify_cut(response.get("cut"), domain)
        commit_ns = response.get("commit_ns")
        integer(commit_ns, "commit_ns")
        if as_known_ns is not None and int(commit_ns) > int(as_known_ns):
            raise EconomicQueryError("owner 返回了获知截止时刻之后的图像")
        if cut is not None and response["cut"] != cut:
            raise EconomicQueryError("owner 回看切面与请求不同")
        image = response.get("image")
        fields = {"book", "recorded", "pending", "applied", "unmatched", "history"}
        fields |= {"finance"} if self.manifest.get("finance") else set()
        allowed = [fields]
        if self.book_revision in ("tb03c-book/1", "tb03c-book/2") and domain == "E":
            allowed.append(fields | {"fill_allocations"})
        if (type(image) is not dict
                or set(image) not in allowed
                or any(type(image[k]) is not list for k in set(image) - {"book", "finance"})
                or (image["book"] is not None and type(image["book"]) is not dict)):
            raise EconomicQueryError("经济图像字段不完整或类型错误")
        if image["book"] is not None and image["book"].get("book_revision") != self.book_revision:
            raise EconomicQueryError("图像书账版本与冻结域不同")
        if "fill_allocations" in image:
            verify_fill_allocations(image, self.fill_policy, self.fill_sources)
        if "finance" in image:
            finance = image["finance"]
            fields = {"schema_revision", "rights", "decisions", "slots", "responsibilities"}
            if (type(finance) is not dict or set(finance) not in (fields, fields | {"closed_applicants"})
                    or finance.get("schema_revision") != self.finance_revision
                    or any(type(finance[k]) is not list for k in set(finance) - {"schema_revision"})):
                raise EconomicQueryError("财务图像字段、版本或类型错误")
        if response.get("image_hash") != digest(image):
            raise EconomicQueryError("经济图像与完整摘要不同")
        # 当前读取 PID/运行环境/epoch 单列诊断，不改变历史图像或语义 token。
        return {"status": "Available", "cut": response["cut"], "commit_ns": commit_ns, "image": image,
                "image_hash": response["image_hash"]}

    def _store(self, body, *, paged=False, cache=True):
        token = "economic-snapshot-" + digest(body)
        result = {**body, "snapshot_token": token}
        encoded = canonical(result)
        if not paged and len(encoded) > self.resources["max_reply_bytes"]:
            raise EconomicQueryError("QueryBudgetExceeded：旧版单帧经济切面超过容量")
        self._remaining_ms()
        if cache:
            with self.lock:
                self._cache_put_locked(token, encoded)
        return result

    def snapshot(self, payload, *, deadline=None, paged=False, cache=True):
        self._start(deadline)
        self.budget.paged = paged
        if type(payload) is not dict or set(payload) != {"op", "cuts", "as_known_ns", "snapshot_token"}:
            raise EconomicQueryError("snapshot 参数不完整或多余")
        if payload["op"] != "snapshot":
            raise EconomicQueryError("请求操作不符")
        token, cuts, as_known = payload["snapshot_token"], payload["cuts"], payload["as_known_ns"]
        if token is not None:
            if type(token) is not str or cuts is not None or as_known is not None:
                raise EconomicQueryError("固定 token 不能混入新 cut 或获知时刻")
            with self.lock:
                self._expire_transfers_locked()
                encoded = self.cache.get(token)
                if encoded is not None:
                    self.cache.move_to_end(token)
            if encoded is None:
                self._remaining_ms()
                return {"kind": "Gap", "reason": "SnapshotExpired", "requested_token": token,
                        "session_id": self.session, "session_generation": self.generation}
            result = decode_frame(encoded)
            if not paged and len(encoded) > self.resources["max_reply_bytes"]:
                raise EconomicQueryError("QueryBudgetExceeded：旧版单帧经济切面超过容量")
            self._remaining_ms()
            return result
        if cuts is not None and (type(cuts) is not dict or set(cuts) != set(self.nodes)):
            raise EconomicQueryError("回看须给完整域向量")
        if cuts is not None and as_known is not None:
            raise EconomicQueryError("固定向量与获知时刻不能同时指定")
        if as_known is not None:
            integer(as_known, "as_known_ns")
        if cuts is not None:
            for domain in self.nodes:
                verify_cut(cuts[domain], domain)
        views = self._parallel_domains(lambda domain: self.view(
            domain, None if cuts is None else cuts[domain], as_known))
        return self._store({
            "kind": "EconomicSnapshot", "session_id": self.session, "session_generation": self.generation,
            "manifest_hash": self.manifest_hash,
            "mode": "AsKnown" if cuts is not None or as_known is not None else "Current",
            "as_known_ns": as_known, "domains": views,
            "completeness": "CompleteVector" if all(v["status"] == "Available" for v in views.values()) else "PartialVector",
        }, paged=paged, cache=cache)

    def original(self, payload):
        """#1394：沿指定完整经济 cut 向量查原件，绝不用后来回执填旧 cut。"""
        self._start()
        self.budget.paged = False
        if (type(payload) is not dict
                or set(payload) != {"op", "kind", "domain_id", "object_id", "cuts"}
                or payload["op"] != "original" or payload["kind"] not in ("decision", "grant")):
            raise EconomicQueryError("原件查询参数不完整或多余")
        domain, cuts = payload["domain_id"], payload["cuts"]
        if domain not in self.nodes or type(cuts) is not dict or set(cuts) != set(self.nodes):
            raise EconomicQueryError("原件查询须固定完整域向量和目标 owner")
        for name, cut in cuts.items():
            verify_cut(cut, name)
        kind = payload["kind"]
        if (kind == "grant") != (domain == "E"):
            raise EconomicQueryError("原件类型不属于该权威域")
        object_id = text(payload["object_id"], "原身份")
        view = self.view(domain, cuts[domain])
        response = self.owner(domain, {"op": "QueryDecision" if kind == "decision" else "QueryGrant",
                                      "decision_id" if kind == "decision" else "grant_id": object_id,
                                      "cut": cuts[domain], "as_known_ns": None})
        finance = view["image"].get("finance", {})
        candidates = finance.get("decisions", []) if kind == "decision" else [
            grant for slot in finance.get("slots", []) for grant in slot["grants"]]
        selected = next((x for x in candidates if x[kind + "_id"] == object_id), None)
        if selected is None or response.get(kind) != selected:
            raise EconomicQueryError("原件与指定 cut 的已核图像不同")
        receipts = response.get("receipts") if kind == "decision" else [response.get("receipt")]
        if type(receipts) is not list or not receipts:
            raise EconomicQueryError("原件缺实际持久回执")
        for receipt in receipts:
            verify_cut(receipt.get("cut"), domain)
            if int(receipt["cut"]["commit_seq"]) > int(cuts[domain]["commit_seq"]):
                raise EconomicQueryError("旧 cut 不能查到未来回执")
        return {"kind": "EconomicOriginal", "session_id": self.session,
                "session_generation": self.generation, "manifest_hash": self.manifest_hash,
                "cuts": cuts, "domain_id": domain, "original": response}

    def watch(self, payload, *, deadline=None, paged=False):
        self._start(deadline)
        self.budget.paged = paged
        if type(payload) is not dict or set(payload) != {"op", "cuts"} or payload["op"] != "watch":
            raise EconomicQueryError("watch 参数不完整或多余")
        cuts = payload["cuts"]
        if type(cuts) is not dict or set(cuts) != set(self.nodes):
            raise EconomicQueryError("watch 须给完整域向量")
        for domain, cut in cuts.items():
            verify_cut(cut, domain)
        def read_changes(domain):
            base = self.view(domain, cuts[domain])
            response = self.owner(domain, {"op": "Watch", "after_cut": cuts[domain]})
            if response.get("kind") != "EconomicChanges" or type(response.get("changes")) is not list:
                raise EconomicQueryError("变化响应不完整")
            if len(response["changes"]) > self.resources["max_changes"]:
                raise EconomicQueryError("QueryBudgetExceeded：变化数量超过事前界限")
            verify_cut(response.get("cut"), domain)
            previous, previous_ns = cuts[domain], integer(base["commit_ns"], "base commit_ns")
            for change in response["changes"]:
                if type(change) is not dict or set(change) != {"cut", "image", "image_hash", "commit_ns"}:
                    raise EconomicQueryError("变化项不是完整原子批次")
                verify_cut(change["cut"], domain)
                ns = integer(change["commit_ns"], "change commit_ns")
                if ns < previous_ns or change["image_hash"] != digest(change["image"]):
                    raise EconomicQueryError("变化时间倒退或完整图像摘要不同")
                if integer(change["cut"]["commit_seq"], "commit_seq") != integer(previous["commit_seq"], "commit_seq") + 1:
                    raise EconomicQueryError("变化序号存在缺口或重复")
                fixed = self.view(domain, change["cut"])
                if (fixed["image"] != change["image"] or fixed["image_hash"] != change["image_hash"]
                        or fixed["commit_ns"] != change["commit_ns"]):
                    raise EconomicQueryError("变化与同切面权威图像/时间不同")
                previous, previous_ns = change["cut"], ns
            if previous != response["cut"]:
                raise EconomicQueryError("变化缺少到实际前沿的完整批次")
            return {"status": "Available", "base_cut": cuts[domain],
                    "next_cut": response["cut"], "changes": response["changes"]}

        domains = self._parallel_domains(read_changes)
        result = {"kind": "EconomicWatch", "session_id": self.session,
                  "session_generation": self.generation, "manifest_hash": self.manifest_hash, "domains": domains}
        limit = self.resources["max_cached_bytes" if paged else "max_reply_bytes"]
        if len(canonical(result)) > limit:
            raise EconomicQueryError("QueryBudgetExceeded：完整经济变化超过事前容量")
        self._remaining_ms()
        return result
