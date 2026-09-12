"""#1372：固定 cut 分页/有界 Watch；Q 不写 S 存储，不要求 S socket 存活。"""

import hashlib
import json
import os
import sqlite3
import time
import threading
from pathlib import Path


RESOURCE_KEYS = (
    "max_frame_bytes", "max_pending_connections", "socket_read_timeout_ms", "socket_write_timeout_ms",
    "max_capture_bytes", "capture_deadline_ms", "verify_deadline_ms", "max_query_workers",
    "max_query_queue", "max_reply_bytes", "max_atomic_batch_bytes", "max_cached_images", "max_cache_bytes",
    "poll_interval_ms", "max_client_leases", "client_lease_ms", "max_client_pending_batches",
    "max_page_size", "max_watch_batches",
)
FAMILIES = ("objects", "withdrawn_objects", "witnesses", "relations", "observations", "raw_history")
ORDER_VERSION = "s-record-order/1"
TOKEN_SCHEMA = "s-snapshot-token/1"
TOKEN_FIELDS = {"token_schema", "session_id", "session_generation", "cut_generation", "scope", "history_mode",
                "structure_cut", "catalog_revision", "index_frontier", "profile_hash", "rule_revision",
                "order_version", "page_size", "offset", "cut_projection_digest", "issued_capture_digest", "token_checksum"}


def load_resources(path, audit):
    with Path(path).open("rb") as handle:
        raw = handle.read(65537)
    if len(raw) > 65536:
        raise ValueError("Q 资源配置超过 64 KiB")
    data = audit.parse_json(raw)
    if set(data) != set(RESOURCE_KEYS):
        raise ValueError("Q 资源配置字段缺失或未声明")
    for key in RESOURCE_KEYS:
        if type(data.get(key)) is not int or data[key] <= 0:
            raise ValueError("Q 资源配置必须显式给出正整数 " + key)
    if data["max_cached_images"] < data["max_query_workers"]:
        raise ValueError("Q 每 worker 最多缓存一个已核图像；max_cached_images 不得小于 worker 数")
    return data


class AuditReader:
    """一个 worker 独占一个只读连接；data_version 绝不跨连接比较。"""

    def __init__(self, db_path, resources, helpers, seam=None):
        self.path = Path(db_path).resolve()
        self.resources = resources
        self.h = helpers
        self.audit = helpers["_query_integrity"]()
        self.connection = None
        self.identity = None
        self.cached = None
        self.cached_version = None
        self.seam = seam or (lambda _name: None)
        self.stats = {"captures": 0, "cache_hits": 0}

    def close(self):
        self.cached = self.cached_version = None
        if self.connection is not None:
            self.connection.close()
            self.connection = None

    def _identity(self):
        stat = self.path.stat()
        return stat.st_dev, stat.st_ino

    def capture_verified(self, _capture_deadline=None, _retried=False):
        try:
            identity = self._identity()
        except OSError:
            self.close()
            raise
        if self.connection is None or self.identity != identity:
            self.close()
            # 执行期间仍只属此 worker；允许 server 在所有 worker join 后统一关闭。
            self.connection = self.h["open_readonly"](self.path, check_same_thread=False)
            self.identity = identity
        conn = self.connection
        deadline = (_capture_deadline if _capture_deadline is not None else
                    time.monotonic() + self.resources["capture_deadline_ms"] / 1000)
        conn.set_progress_handler(lambda: int(time.monotonic() > deadline), 1000)
        try:
            if conn.in_transaction:
                raise ValueError("Q 连接意外处于跨请求事务")
            if conn.execute("PRAGMA query_only").fetchone()[0] != 1 or conn.execute("PRAGMA read_uncommitted").fetchone()[0] != 0:
                raise ValueError("Q 只读隔离设置被改变")
            if [r[1] for r in conn.execute("PRAGMA database_list")] != ["main"]:
                raise ValueError("Q 连接不得附加其他数据库")
            if conn.execute("PRAGMA journal_mode").fetchone()[0].lower() != "wal":
                raise ValueError("Q 需要 S 已建立的 WAL 存储")
            v0 = conn.execute("PRAGMA data_version").fetchone()[0]
            self.seam("after_v0")
            conn.execute("BEGIN")
            conn.execute("SELECT key,value FROM meta ORDER BY key LIMIT 1").fetchone()
            self.seam("after_first_read")
            v1 = conn.execute("PRAGMA data_version").fetchone()[0]
            candidate = self.cached is not None and self.cached_version == v0 == v1
            image = None if candidate else self.audit.capture(conn, self.resources["max_capture_bytes"], deadline)
            conn.rollback()
            self.seam("after_transaction")
            v2 = conn.execute("PRAGMA data_version").fetchone()[0]
            if self._identity() != identity:
                raise self.audit.QueryBudgetExceeded("捕获期间 DB 文件身份改变；本次未完成")
            if candidate and v0 == v1 == v2:
                self.stats["cache_hits"] += 1
                return self.cached
            if candidate:
                # 候选版本未被夹持：从新的完整读事务重新捕获，不重标旧图像。
                self.cached = self.cached_version = None
                conn.set_progress_handler(None, 0)
                if _retried or time.monotonic() > deadline:
                    raise self.audit.QueryBudgetExceeded("并发提交阻止无变化缓存复用；本次未完成")
                return self.capture_verified(deadline, True)
            self.cached = self.cached_version = None
            self.stats["captures"] += 1
        except Exception as exc:
            if conn.in_transaction:
                conn.rollback()
            self.cached = self.cached_version = None
            if isinstance(exc, sqlite3.OperationalError) and time.monotonic() > deadline:
                raise self.audit.QueryBudgetExceeded("一致材料捕获达到时间上限") from exc
            raise
        finally:
            conn.set_progress_handler(None, 0)
        verify_deadline = time.monotonic() + self.resources["verify_deadline_ms"] / 1000
        proof = self.audit.verify(image, self.h, verify_deadline)
        # 捕获值与解码后保留图像各受 max_capture_bytes 限制；该数不是进程 RSS。
        # JSON 解码、规范化与投影有临时对象，worker/帧/捕获上限共同限定输入域。
        retained_bytes = self.audit.object_bytes(proof, self.resources["max_capture_bytes"], verify_deadline)
        self.seam("before_cache_registration")
        v3 = conn.execute("PRAGMA data_version").fetchone()[0]
        allowance = self.resources["max_cache_bytes"] // self.resources["max_query_workers"]
        if v0 == v1 == v2 == v3 and self._identity() == identity and retained_bytes <= allowance:
            self.cached, self.cached_version = proof, v3
        return proof


def _integer(raw, name, invalid, positive=False):
    if type(raw) is not str or not raw.isascii() or not raw.isdigit() or len(raw)>19 or (len(raw)>1 and raw[0]=='0'):
        raise invalid(name + " 必须是规范整数文本")
    value = int(raw)
    if value < int(positive) or value >= 2**63:
        raise invalid(name + " 超出声明整数域")
    return value


def _text(value, name, invalid):
    if type(value) is not str or not value:
        raise invalid(name + " 必须是非空文本")
    try:
        value.encode("utf-8")
    except UnicodeError as exc:
        raise invalid(name + " 不是 UTF-8 文本") from exc
    return value


def validate_envelope(envelope, audit, invalid):
    fields = {"schema_revision", "session_id", "session_generation", "source_namespace", "source_epoch",
              "message_id", "producer_id", "producer_epoch", "payload_hash", "causal_refs", "payload"}
    if type(envelope) is not dict or set(envelope) != fields or envelope["schema_revision"] != "s-session/2":
        raise invalid("公共头 schema/完整字段不符")
    for key in fields - {"payload", "causal_refs", "schema_revision"}:
        _text(envelope[key], key, invalid)
    _integer(envelope["session_generation"], "session_generation", invalid, True)
    _integer(envelope["producer_epoch"], "producer_epoch", invalid, True)
    if type(envelope["causal_refs"]) is not list or type(envelope["payload"]) is not dict:
        raise invalid("公共头 causal_refs/payload 形状不符")
    for reference in envelope["causal_refs"]:
        if type(reference) is not dict:
            raise invalid("causal_refs 成员必须是身份对象")
        for key in ("source_namespace", "source_epoch", "message_id", "payload_hash"):
            _text(reference.get(key), "causal_refs."+key, invalid)
    if hashlib.sha256(audit.canonical(envelope["payload"])).hexdigest() != envelope["payload_hash"]:
        raise invalid("请求规范 payload_hash 不符")
    return envelope["payload"]


def _order(family, record):
    numeric = lambda key: int(record[key])
    text = lambda key: record[key].encode("utf-8")
    if family == "objects":
        return numeric("window_start"), numeric("window_mid"), numeric("window_end"), text("object_id"), numeric("object_revision")
    if family == "withdrawn_objects":
        return numeric("withdrawn_generation"), text("object_id"), numeric("object_revision")
    if family == "witnesses":
        return text("object_id"), numeric("slot"), text("witness_id")
    if family == "relations":
        return text("subject"), text("relation_type"), text("object")
    if family == "observations":
        return text("kind"), (-1 if record["window_start"] is None else numeric("window_start")), text("observation_id")
    return numeric("seq"), text("identity_key"), numeric("revision")


def _key(family, record):
    if family in ("objects", "withdrawn_objects"):
        return [record["object_id"]]
    if family == "relations":
        return [record[k] for k in ("subject", "relation_type", "object")]
    if family == "raw_history":
        return [record["identity_key"], record["revision"]]
    return [record["witness_id" if family == "witnesses" else "observation_id"]]


def project_fixed_cut(proof, generation, mode, h):
    audit = h["_query_integrity"]()
    state = audit.project_state(proof, generation, h)
    snapshot = state["snapshot"]
    snapshot["history_mode"] = mode
    snapshot["as_of_generation"] = str(generation) if mode == "AsKnown" else None
    rows, counts = [], {}
    object_ids = set()
    for family in FAMILIES:
        records = sorted(snapshot[family], key=lambda value: _order(family, value))
        identities = set()
        for record in records:
            key = tuple(_key(family, record))
            if key in identities:
                raise ValueError(family + " 含重复公开身份")
            identities.add(key)
            if family in ("objects", "withdrawn_objects"):
                if key in object_ids:
                    raise ValueError("active/withdrawn 对象身份冲突")
                object_ids.add(key)
            rows.append({"family": family, "key": list(key), "record": record})
        snapshot[family] = records
        counts[family] = str(len(records))
    counts["total"] = str(len(rows))
    protocol = proof.get("protocol")
    if protocol is None:
        raise ValueError("v2 查询需要完整协议存储")
    if generation == 0:
        snapshot["profile_id"], snapshot["profile_hash"] = proof["meta"]["profile_id"], proof["meta"]["profile_hash"]
    fixed = {key: snapshot[key] for key in ("session_id", "structure_cut", "catalog_revision", "index_frontier",
             "profile_id", "profile_hash", "input_frontier", "seq_range", "catalog_run_status", "catalog_evidence", "scope")}
    fixed.update(session_generation=protocol["session_generation"], cut_generation=str(generation),
                 rule_revision=proof["meta"]["rule_revision"], history_mode=mode,
                 as_of_generation=snapshot["as_of_generation"])
    projection = dict(fixed_cut=fixed, catalog=state["catalog"], rows=rows, counts=counts,
                      scope=snapshot["scope"], omissions=[], order_version=ORDER_VERSION)
    return projection, hashlib.sha256(audit.canonical(projection)).hexdigest()


def _token(projection, digest, capture_digest, page_size, offset, audit):
    fixed = projection["fixed_cut"]
    result = {key: fixed[key] for key in ("session_id", "session_generation", "cut_generation", "scope", "history_mode",
              "structure_cut", "catalog_revision", "index_frontier", "profile_hash", "rule_revision")}
    result.update(token_schema=TOKEN_SCHEMA, order_version=ORDER_VERSION, page_size=str(page_size), offset=str(offset),
                  cut_projection_digest=digest, issued_capture_digest=capture_digest)
    result["token_checksum"] = hashlib.sha256(audit.canonical(result)).hexdigest()
    return result


def snapshot_page(proof, request, resources, h):
    audit, invalid = h["_query_integrity"](), h["InvalidQuery"]
    if "snapshot_token" in request:
        if set(request) != {"op", "snapshot_token"}:
            raise invalid("续页只能提供原 snapshot_token")
        token = request["snapshot_token"]
        if type(token) is not dict or set(token) != TOKEN_FIELDS:
            raise invalid("snapshot_token 必须含完整且唯一的声明字段")
        issued = token["issued_capture_digest"]
        if type(issued) is not str or len(issued) != 64 or any(c not in "0123456789abcdef" for c in issued):
            raise invalid("snapshot_token 签发来源摘要不规范")
        unsigned = {k: v for k, v in token.items() if k != "token_checksum"}
        if token.get("token_checksum") != hashlib.sha256(audit.canonical(unsigned)).hexdigest():
            raise invalid("snapshot_token checksum 不符")
        generation = _integer(token.get("cut_generation"), "cut_generation", invalid)
        page_size = _integer(token.get("page_size"), "page_size", invalid, True)
        offset = _integer(token.get("offset"), "offset", invalid)
        mode = token.get("history_mode")
    else:
        required = {"op", "session_id", "session_generation", "scope", "history_mode", "as_of_generation", "page_size", "order_version"}
        if set(request) != required:
            raise invalid("首个 snapshot 请求完整字段不符")
        generation = proof["generation"] if request["as_of_generation"] is None else _integer(request["as_of_generation"], "as_of_generation", invalid)
        page_size = _integer(request["page_size"], "page_size", invalid, True)
        offset, mode = 0, request["history_mode"]
        token = None
        if request["session_id"] != proof["meta"]["session_id"] or request["session_generation"] != proof.get("protocol", {}).get("session_generation"):
            raise invalid("请求 session/化身不符；必须显式重新建立快照")
        if request["scope"] != h["_scope"](proof["meta"]) or request["order_version"] != ORDER_VERSION:
            raise invalid("请求 scope/order_version 不符")
    if mode not in ("AsKnown", "RecomputedWithRevision"):
        raise invalid("history_mode 不符")
    if page_size > resources["max_page_size"]:
        raise audit.QueryBudgetExceeded("page_size 超出事前上限")
    projection, digest = project_fixed_cut(proof, generation, mode, h)
    issued = token["issued_capture_digest"] if token else proof["capture_digest"]
    expected_token = _token(projection, digest, issued, page_size, offset, audit)
    if token is not None and token != expected_token:
        raise invalid("snapshot_token 固定切面/身份/schema 不符；需要重建")
    if offset > len(projection["rows"]):
        raise invalid("分页 offset 超出完整切面")
    end = min(offset + page_size, len(projection["rows"]))
    done = end == len(projection["rows"])
    next_token = None if done else _token(projection, digest, issued, page_size, end, audit)
    return dict(projection, rows=projection["rows"][offset:end], offset=str(offset), done=done,
                next_token=next_token, cut_projection_digest=digest,
                validated_capture_digest=proof["capture_digest"])


def cursor_at(proof, generation, h):
    publication = proof["deltas"][generation-1] if generation else None
    return {"session_id": proof["meta"]["session_id"],
            "session_generation": proof["protocol"]["session_generation"],
            "catalog_revision": proof["meta"]["catalog_revision"], "scope": h["_scope"](proof["meta"]),
            "after_generation": str(generation), "base_cut": f"cut-{generation}",
            "index_frontier": publication["index_frontier"] if publication else "",
            "last_seq": publication["input_frontier"] if publication else "-1", "order_version": ORDER_VERSION}


class ClientLeases:
    """只存客户租约和有界积压计数，不创建第二份永久 Delta 或写 delivery floor。"""

    def __init__(self, resources):
        self.resources = resources
        self.lock = threading.Lock()
        self.clients = {}

    def observe(self, client, cursor, proof, audit):
        now = time.monotonic()
        identity = proof["meta"]["session_id"], proof["protocol"]["session_generation"]
        head = proof["generation"]
        with self.lock:
            expired = [key for key, lease in self.clients.items() if lease["expires"] < now]
            for key in expired:
                del self.clients[key]
            for lease in self.clients.values():
                if lease["identity"] == identity:
                    lease["observed_head"] = max(lease["observed_head"], head)
                    pending = lease["observed_head"] - max(lease["delivered"], lease["initial_head"])
                    if pending > self.resources["max_client_pending_batches"]:
                        lease["overflow"] = True
            lease = self.clients.get(client)
            if lease is None or lease["identity"] != identity:
                if lease is None and len(self.clients) >= self.resources["max_client_leases"]:
                    raise audit.QueryBudgetExceeded("客户租约达到事前上限")
                lease = {"identity": identity, "delivered": int(cursor["after_generation"]),
                         "initial_head": head, "observed_head": head, "overflow": False}
                self.clients[client] = lease
            elif int(cursor["after_generation"]) >= lease["observed_head"]:
                # 完整快照安装后的新 cursor 已通过永久 cut 绑定校验。
                lease.update(delivered=int(cursor["after_generation"]), initial_head=head, overflow=False)
            lease["expires"] = now + self.resources["client_lease_ms"] / 1000
            return lease["overflow"]

    def delivered(self, client, cursor):
        with self.lock:
            lease = self.clients.get(client)
            if lease is not None and lease["identity"] == (cursor["session_id"], cursor["session_generation"]):
                lease["delivered"] = max(lease["delivered"], int(cursor["after_generation"]))


def watch_page(proof, request, resources, h, leases):
    audit, invalid = h["_query_integrity"](), h["InvalidQuery"]
    if set(request) != {"op", "cursor", "max_batches", "client_id"}:
        raise invalid("Watch 请求完整字段不符")
    cursor = request["cursor"]
    client = _text(request["client_id"], "client_id", invalid)
    maximum = _integer(request["max_batches"], "max_batches", invalid, True)
    if maximum > resources["max_watch_batches"]:
        raise audit.QueryBudgetExceeded("Watch 批数超出事前上限")
    if proof.get("protocol") is None:
        raise ValueError("v2 Watch 需要完整协议存储")
    shape = set(cursor_at(proof, 0, h))
    if type(cursor) is not dict or set(cursor) != shape:
        raise invalid("Watch cursor 完整字段不符")
    after = _integer(cursor["after_generation"], "after_generation", invalid)
    _text(cursor["session_id"], "cursor.session_id", invalid)
    _integer(cursor["session_generation"], "cursor.session_generation", invalid, True)
    head = proof["generation"]
    policy = proof["protocol"]["delivery_policy"]
    current, digest = project_fixed_cut(proof, head, "RecomputedWithRevision", h)
    output = {"deltas": [], "next_cursor": cursor, "observed_head": current["fixed_cut"],
              "delivery_frontier": {key: str(policy[key]) if type(policy[key]) is int else policy[key]
                                    for key in ("policy_revision", "retain_generations", "first_available_generation", "head_generation")},
              "has_more": False, "gap": None, "validated_capture_digest": proof["capture_digest"]}
    same_identity = (cursor["session_id"], cursor["session_generation"]) == (
                     proof["meta"]["session_id"], proof["protocol"]["session_generation"])
    reason, missing_range = None, None
    if not same_identity:
        reason = "session_identity_changed"
    elif after > head:
        raise invalid("同一 session/化身的 cursor 超出永久发布历史")
    elif cursor != cursor_at(proof, after, h):
        raise invalid("cursor base/index/seq/catalog/scope/order 与固定 cut 不符")
    elif after < policy["first_available_generation"]-1:
        reason = "delivery_retention_gap"
        missing_range = {"from_generation": str(after+1), "to_generation": str(policy["first_available_generation"]-1)}
    elif leases.observe(client, cursor, proof, audit):
        reason = "client_backlog_overflow"
        missing_range = {"from_generation": str(after+1), "to_generation": str(head)}
    if reason is not None:
        output["gap"] = {"requested_identity": cursor, "reason": reason, "missing_range": missing_range,
                         "rebuild": current["fixed_cut"],
                         "snapshot_token": _token(current, digest, proof["capture_digest"], resources["max_page_size"], 0, audit)}
        return output
    for index in range(after, min(head, after+maximum)):
        delta = h["_project_wire_integers"](proof["deltas"][index])
        if len(audit.canonical(delta)) > resources["max_atomic_batch_bytes"]:
            raise audit.QueryBudgetExceeded("单个完整 Delta 超出原子批字节上限")
        output["deltas"].append(delta)
        output["next_cursor"] = cursor_at(proof, index+1, h)
        output["has_more"] = index+1 < head
        if len(audit.canonical(output)) > resources["max_reply_bytes"]:
            output["deltas"].pop()
            if not output["deltas"]:
                raise audit.QueryBudgetExceeded("单个完整 Watch 响应超出回复上限")
            output["next_cursor"] = cursor_at(proof, index, h)
            output["has_more"] = True
            break
    return output


def response_envelope(request, payload, proof, producer_epoch, audit):
    digest = hashlib.sha256(audit.canonical(payload)).hexdigest()
    identity = [request[key] for key in ("source_namespace", "source_epoch", "message_id")]
    reference = {key: request[key] for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")}
    session = proof["meta"]["session_id"] if proof is not None else request["session_id"]
    incarnation = proof["protocol"]["session_generation"] if proof is not None else request["session_generation"]
    return {"schema_revision": "s-session/2", "session_id": session,
            "session_generation": incarnation,
            "source_namespace": "s-observe/replies", "source_epoch": str(producer_epoch),
            "message_id": "reply-"+hashlib.sha256(audit.canonical(identity+[digest])).hexdigest(),
            "producer_id": "Q:"+session, "producer_epoch": str(producer_epoch),
            "payload_hash": digest, "causal_refs": [reference], "payload": payload}
