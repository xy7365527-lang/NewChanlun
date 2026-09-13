"""#1372：只读一致图像及逐代关联审计；不执行结构判定，不修改 S 的库。

全表捕获与 CPU 审计分离。累计 batch 本身可能含全部历史；这里避免每代额外
构造 before/current Snapshot，不承诺总历史字节线性增长或 O(page) 查询。
"""

import hashlib
import json
import sqlite3
import struct
import sys
import time
import re
import zlib
from collections import defaultdict


class QueryBudgetExceeded(RuntimeError):
    """未取得完整健康证明；不是持久损坏，也不能返回半份成功切面。"""


# 与正式 S v1 DDL 同一列/约束；v2 的控制表由其显式版本另行登记。
LEGACY_SCHEMA = """
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE raw_events (
 identity_key TEXT NOT NULL, revision INTEGER NOT NULL, input_revision TEXT NOT NULL,
 payload_hash TEXT NOT NULL, receipt_id TEXT NOT NULL, seq INTEGER NOT NULL UNIQUE,
 source_namespace TEXT NOT NULL, source_epoch TEXT NOT NULL, instrument TEXT NOT NULL,
 event_id TEXT NOT NULL, received_at TEXT NOT NULL, raw_text TEXT NOT NULL,
 price TEXT NOT NULL, ts TEXT NOT NULL, volume TEXT NOT NULL, source_coord TEXT NOT NULL,
 supersedes_revision INTEGER, PRIMARY KEY(identity_key,revision));
CREATE TABLE batches (batch_id TEXT PRIMARY KEY, canonical_bytes BLOB NOT NULL, byte_len INTEGER NOT NULL);
CREATE TABLE objects (
 object_id TEXT PRIMARY KEY, object_revision INTEGER NOT NULL, kind TEXT NOT NULL,
 batch_id TEXT NOT NULL, branch TEXT NOT NULL, dir_ab TEXT NOT NULL, dir_bc TEXT NOT NULL,
 window_start INTEGER NOT NULL, window_mid INTEGER NOT NULL, window_end INTEGER NOT NULL,
 comparisons_json TEXT NOT NULL, input_refs_json TEXT NOT NULL, source_coords_json TEXT NOT NULL,
 first_known_generation INTEGER NOT NULL, first_known_cut TEXT NOT NULL,
 published_generation INTEGER NOT NULL, withdrawn_generation INTEGER,
 withdrawal_reason TEXT, superseded_by TEXT);
CREATE TABLE witnesses (
 witness_id TEXT PRIMARY KEY, object_id TEXT NOT NULL, slot INTEGER NOT NULL,
 merged_source_index INTEGER NOT NULL, merged_high TEXT NOT NULL, merged_low TEXT NOT NULL,
 merged_open TEXT NOT NULL, merged_close TEXT NOT NULL, raw_json TEXT NOT NULL,
 published_generation INTEGER NOT NULL);
CREATE TABLE relations (
 relation_id TEXT PRIMARY KEY, subject TEXT NOT NULL, relation_type TEXT NOT NULL,
 object TEXT NOT NULL, published_generation INTEGER NOT NULL);
CREATE TABLE observations (
 observation_id TEXT PRIMARY KEY, batch_id TEXT NOT NULL, kind TEXT NOT NULL,
 window_start INTEGER, window_mid INTEGER, window_end INTEGER, reason TEXT NOT NULL,
 detail_json TEXT NOT NULL, published_generation INTEGER NOT NULL);
CREATE TABLE structure_deltas (
 generation INTEGER PRIMARY KEY, session_id TEXT NOT NULL, catalog_revision TEXT NOT NULL,
 base_cut TEXT NOT NULL, next_cut TEXT NOT NULL, seq_range_json TEXT NOT NULL,
 index_frontier TEXT NOT NULL, input_frontier INTEGER NOT NULL, catalog_run_status TEXT NOT NULL,
 catalog_evidence_json TEXT NOT NULL, delta_json TEXT NOT NULL);
CREATE TABLE writer_epoch_history (
 ordinal INTEGER PRIMARY KEY, from_epoch TEXT NOT NULL, to_epoch TEXT NOT NULL,
 generation_at_transition TEXT NOT NULL, advance_state_at_transition TEXT NOT NULL,
 transitioned_at TEXT NOT NULL);
CREATE TABLE catalog (
 catalog_id TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, domain TEXT NOT NULL,
 branches_json TEXT NOT NULL, impl_status TEXT NOT NULL, proof_status TEXT NOT NULL,
 run_status TEXT NOT NULL, evidence_json TEXT NOT NULL);
"""


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def _pairs_unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("持久 JSON 含重复键")
        result[key] = value
    return result


def _nonfinite(value):
    raise ValueError("持久 JSON 含非有限数")


def parse_json(raw, expected=dict):
    if type(raw) not in (str, bytes):
        raise ValueError("持久 JSON 类型损坏")
    result = json.loads(raw, object_pairs_hook=_pairs_unique, parse_constant=_nonfinite)
    if type(result) not in (expected if type(expected) is tuple else (expected,)):
        raise ValueError("持久 JSON 形状损坏")
    # 同时拒绝转义孤立代理码点；规范字节必须可表示为 UTF-8。
    canonical(result)
    return result


def _deadline(deadline):
    if deadline is not None and time.monotonic() > deadline:
        raise QueryBudgetExceeded("查询阶段达到事前时间上限")


def object_bytes(value, limit=None, deadline=None):
    """按对象身份去重计量已保留 Python 图像；不把估算倍数当缓存字节数。"""
    seen, pending, total = set(), [value], 0
    while pending:
        _deadline(deadline)
        item = pending.pop()
        identity = id(item)
        if identity in seen:
            continue
        seen.add(identity)
        total += sys.getsizeof(item)
        if limit is not None and total > limit:
            raise QueryBudgetExceeded("已解码图像超过事前字节上限")
        if type(item) is dict:
            pending.extend(item.keys())
            pending.extend(item.values())
        elif type(item) in (list, tuple, set, frozenset):
            pending.extend(item)
    return total


def _schema(conn):
    rows = tuple(conn.execute("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name"))
    tables = {}
    for kind, name, _table, _sql in rows:
        if kind == "table":
            info = tuple(conn.execute("SELECT * FROM pragma_table_info(?)", (name,)))
            indexes = []
            for _seq, idx, unique, origin, partial in conn.execute("SELECT * FROM pragma_index_list(?)", (name,)):
                columns = tuple(r[2] for r in conn.execute("SELECT * FROM pragma_index_info(?)", (idx,)))
                indexes.append((unique, origin, partial, columns))
            tables[name] = (info, tuple(sorted(indexes)))
    return rows, tables


def _expected_schema(sql):
    conn = sqlite3.connect(":memory:")
    try:
        conn.executescript(sql)
        return _schema(conn)[1]
    finally:
        conn.close()


def _expected_sql(sql):
    conn = sqlite3.connect(":memory:")
    try:
        conn.executescript(sql)
        return {name: re.sub(r"\s+", "", statement).rstrip(";")
                for kind, name, _table, statement in _schema(conn)[0] if kind == "table"}
    finally:
        conn.close()


EXPECTED_LEGACY = _expected_schema(LEGACY_SCHEMA)

CONTROL_SCHEMA = """
CREATE TABLE s_protocol_meta (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), protocol_revision TEXT NOT NULL,
 session_generation TEXT NOT NULL, clock_plan_hash TEXT NOT NULL, clock_plan_json TEXT NOT NULL,
 next_attempt_ordinal INTEGER NOT NULL CHECK(next_attempt_ordinal>=1), logical_phase_frontier TEXT NOT NULL);
CREATE TABLE s_input_messages (
 source_namespace TEXT NOT NULL, source_epoch TEXT NOT NULL, message_id TEXT NOT NULL,
 payload_hash TEXT NOT NULL, canonical_envelope TEXT NOT NULL, receipt_id TEXT NOT NULL,
 accepted_seq INTEGER NOT NULL CHECK(accepted_seq>=0),
 status TEXT NOT NULL CHECK(status IN ('AcceptedPending','Committed')),
 first_published_generation INTEGER, clock_event_id TEXT NOT NULL UNIQUE,
 attempt_count INTEGER NOT NULL CHECK(attempt_count>=0),
 PRIMARY KEY(source_namespace,source_epoch,message_id));
CREATE TABLE s_clock_events (
 clock_event_id TEXT NOT NULL, phase TEXT NOT NULL CHECK(phase IN ('accept','begin','commit','recover')),
 attempt_index INTEGER NOT NULL, semantic_ns TEXT NOT NULL, message_key TEXT NOT NULL,
 attempt_ordinal INTEGER, generation INTEGER NOT NULL CHECK(generation>=0),
 PRIMARY KEY(clock_event_id,phase,attempt_index));
CREATE TABLE s_delivery_policy (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), session_id TEXT NOT NULL,
 session_generation TEXT NOT NULL, policy_revision TEXT NOT NULL,
 retain_generations INTEGER NOT NULL CHECK(retain_generations>=1),
 first_available_generation INTEGER NOT NULL CHECK(first_available_generation>=1),
 head_generation INTEGER NOT NULL CHECK(head_generation>=0));
CREATE TABLE s_delivery_refs (
 generation INTEGER PRIMARY KEY CHECK(generation>=1),
 FOREIGN KEY(generation) REFERENCES structure_deltas(generation));
"""
EXPECTED_V2 = _expected_schema(LEGACY_SCHEMA + CONTROL_SCHEMA)
EXPECTED_SQL = _expected_sql(LEGACY_SCHEMA + CONTROL_SCHEMA)


def _validate_schema(rows, tables):
    if tables not in (EXPECTED_LEGACY, EXPECTED_V2):
        raise ValueError("S 权威表/列/PK/UNIQUE/NOT NULL 与已知协议模式不符")
    if any(kind not in ("table", "index") for kind, *_rest in rows):
        raise ValueError("S 权威模式包含未声明视图/触发器")
    if any(kind == "index" and sql is not None for kind, _name, _table, sql in rows):
        raise ValueError("S 权威模式包含未声明索引")
    for kind, name, _table, sql in rows:
        if kind == "table" and re.sub(r"\s+", "", sql).rstrip(";") != EXPECTED_SQL[name]:
            raise ValueError("S 权威表 SQL/约束定义与协议不符")


def capture(conn, max_bytes=None, deadline=None):
    """调用者独占连接并持有读事务；这里只读/拷贝，返回后才做语义验证。

max_bytes 计实际 Python 捕获对象大小的保守增量（含每行/每值容器开销）。
单个 SQLite 值读出前先核 SQL length，避免已超上限的 BLOB 先进入 Python。
"""
    _deadline(deadline)
    if max_bytes is not None:
        count, schema_bytes = conn.execute(
            "SELECT COUNT(*),COALESCE(SUM(length(CAST(sql AS BLOB))),0) FROM sqlite_schema").fetchone()
        if count > 2*len(EXPECTED_V2) or schema_bytes > min(max_bytes, 128*1024):
            raise QueryBudgetExceeded("当前模式元数据超过捕获界限")
    schema_rows, schema = _schema(conn)
    _validate_schema(schema_rows, schema)
    image = {"schema_rows": schema_rows, "schema": schema, "tables": {}}
    used = sys.getsizeof(image) + len(canonical(schema_rows)) + len(canonical(schema))
    for table, (columns, _indexes) in schema.items():
        _deadline(deadline)
        names = [column[1] for column in columns]
        # 标识符只来自上方经固定 DDL 核对的字段集合。
        lengths = "+".join(f"COALESCE(length(CAST({name} AS BLOB)),0)" for name in names)
        if max_bytes is not None:
            oversized = conn.execute(f"SELECT EXISTS(SELECT 1 FROM {table} WHERE {lengths}>?)",
                                     (max_bytes,)).fetchone()[0]
            if oversized:
                raise QueryBudgetExceeded("单条持久记录超过捕获字节上限")
        pk = [c[1] for c in sorted(columns, key=lambda c: c[5]) if c[5]]
        order = "seq" if table == "raw_events" else ",".join(pk)
        records = []
        for row in conn.execute(f"SELECT {','.join(names)} FROM {table} ORDER BY {order}"):
            _deadline(deadline)
            used += sys.getsizeof(row) + sum(sys.getsizeof(v) for v in row) + 16
            if max_bytes is not None and used > max_bytes:
                raise QueryBudgetExceeded("全域捕获超过事前字节上限")
            records.append(row)
        image["tables"][table] = tuple(records)
    image["captured_bytes"] = used
    return image


def capture_digest(image):
    """包含当前全部 typed 行及模式；连接版本、墙钟、路径和物理页号不入业务摘要。"""
    digest = hashlib.sha256()
    digest.update(canonical({"schema": image["schema"], "schema_rows": image["schema_rows"]}))
    for table in sorted(image["tables"]):
        digest.update(canonical(table))
        for row in image["tables"][table]:
            digest.update(b"[")
            for value in row:
                if type(value) is bytes:
                    data, tag = value, b"b"
                else:
                    data, tag = canonical(value), type(value).__name__.encode("ascii")
                digest.update(tag + struct.pack(">Q", len(data)) + data)
            digest.update(b"]")
    return digest.hexdigest()


def _typed_rows(image, deadline=None):
    result = {}
    for table, rows in image["tables"].items():
        columns = image["schema"][table][0]
        converted = []
        identities = set()
        for row in rows:
            _deadline(deadline)
            for column, value in zip(columns, row):
                _cid, name, declaration, not_null, _default, pk = column
                if value is None and not not_null and not pk:
                    continue
                expected = {"TEXT": str, "INTEGER": int, "BLOB": bytes}[declaration]
                if type(value) is not expected:
                    raise ValueError(f"{table}.{name} 持久列类型损坏")
                if expected is int and not -(2**63) <= value < 2**63:
                    raise ValueError(f"{table}.{name} 超出 i64")
            identity = tuple(row[c[0]] for c in sorted(columns, key=lambda c: c[5]) if c[5])
            if identity in identities:
                raise ValueError(f"{table} 主身份重复")
            identities.add(identity)
            converted.append(dict(zip((c[1] for c in columns), row)))
        result[table] = converted
    return result


def _signed_i64_text(value, name):
    """与 Rust parse_canonical_i64 同域：唯一十进制表示，含合法负数。"""
    if type(value) is not str or not value or not value.isascii():
        raise ValueError(name + " 必须是规范有符号整数文本")
    if len(value) > 20:
        raise ValueError(name + " 超出 i64")
    negative = value.startswith("-")
    digits = value[1:] if negative else value
    if not digits.isdigit() or (digits.startswith("0") and (negative or len(digits)>1)):
        raise ValueError(name + " 必须是规范有符号整数文本")
    number = int(value)
    if not -(2**63) <= number < 2**63:
        raise ValueError(name + " 超出 i64")
    return number


def _int_text(value, name, minimum=0):
    number = _signed_i64_text(value, name)
    if number < minimum:
        raise ValueError(name + " 超出声明整数域")
    return number


def _raw_wire(row):
    result = dict(row)
    for key in ("revision", "seq", "supersedes_revision"):
        value = result[key]
        if value is None and key == "supersedes_revision":
            continue
        if type(value) is not int or not 0 <= value < 2**63:
            raise ValueError("raw." + key + " 不是非负 i64")
        result[key] = str(value)
    return result


def _verify_raw(rows, deadline=None):
    owners, positions, revisions = {}, {}, {}
    for seq, row in enumerate(rows):
        _deadline(deadline)
        # 已接纳但未发布的行也必须先核数值域；自洽 hash/receipt 不能证明域合法。
        for key in ("price", "ts", "volume"):
            _signed_i64_text(row[key], "raw."+key)
        if row["seq"] != seq:
            raise ValueError("raw.seq 接纳全序断裂/重复")
        identity = "|".join(row[k] for k in ("source_namespace", "source_epoch", "instrument", "event_id"))
        if identity != row["identity_key"]:
            raise ValueError("raw.identity_key 与业务来源四元组不一致")
        coordinate = _int_text(row["source_coord"], "raw.source_coord")
        if coordinate > sys.maxsize * 2 + 1:
            raise ValueError("raw.source_coord 超出 usize")
        if owners.setdefault(coordinate, identity) != identity or positions.setdefault(identity, coordinate) != coordinate:
            raise ValueError("原始源坐标与业务身份双向唯一归属损坏")
        revision = _int_text(row["input_revision"], "raw.input_revision", 1)
        if revision != row["revision"] or revision <= revisions.get(identity, 0):
            raise ValueError("raw.revision 与源修订/接纳顺序不一致")
        if row["supersedes_revision"] != revisions.get(identity):
            raise ValueError("raw.supersedes_revision 不指向该身份上一接纳修订")
        revisions[identity] = revision
        content = {k: row[k] for k in ("event_id", "price", "raw_text", "received_at", "volume")}
        content.update(revision=row["input_revision"], seq=row["source_coord"], timestamp=row["ts"])
        payload_hash = hashlib.sha256(canonical(content)).hexdigest()
        receipt = "rcpt-" + hashlib.sha256((identity + "|" + payload_hash).encode()).hexdigest()[:16]
        if payload_hash != row["payload_hash"] or receipt != row["receipt_id"]:
            raise ValueError("raw 规范事件字节/hash/receipt 不闭合")


def _record_set(records, h, name):
    result = set()
    for record in records:
        if type(record) is not dict:
            raise ValueError(name + " 成员不是对象")
        key = canonical(h["_legacy_payload_projection"](record))
        if key in result:
            raise ValueError(name + " 含重复成员")
        result.add(key)
    return result


def _same(actual, expected, h, name):
    if type(actual) is not list:
        raise ValueError(name + " 必须是数组")
    if _record_set(actual, h, name) != _record_set(expected, h, name):
        raise ValueError(name + " 与独立持久索引/封存内容不一致")


def _intern_set(records, h, name, pool):
    return frozenset(pool.setdefault(key, key) for key in _record_set(records, h, name))


def _same_keys(actual, expected, name):
    if actual != expected:
        raise ValueError(name + " 与独立持久索引/封存内容不一致")


def _sealed_generation(row, entry, gen, frontier, meta, binding, active, observations, raw, h, pool):
    """验证当前代封存原件的内部关系；返回可逐字绑定源的规范集合，非当前库健康结论。"""
    seq_range, evidence, delta = (parse_json(row[k]) for k in ("seq_range_json", "catalog_evidence_json", "delta_json"))
    header = {k: row[k] for k in ("session_id", "catalog_revision", "base_cut", "next_cut", "index_frontier", "catalog_run_status")}
    header.update(generation=str(gen), input_frontier=str(frontier), seq_range=seq_range, catalog_evidence=evidence)
    if any(key not in delta or canonical(delta[key]) != canonical(value) for key, value in header.items()):
        raise ValueError("Delta 内层字段与外层全部列不一致")
    h["_validate_delta_shape"](delta)
    batch_id = row["index_frontier"]
    batch = parse_json(entry["canonical_bytes"])
    for key, value in dict(generation=gen, structure_cut=row["next_cut"], input_frontier=frontier,
                           catalog_revision=meta["catalog_revision"], rule_revision=meta["rule_revision"]).items():
        if key not in batch or canonical(batch[key]) != canonical(value):
            raise ValueError("batch 封存坐标/规则与根不一致")
    h["_verify_batch_profile"](batch, binding)
    h["_validate_scope"](batch.get("scope"))
    h["_validate_scope"](evidence.get("scope"))
    for key in ("profile_id", "profile_hash", "rule_revision"):
        if key not in evidence or canonical(evidence[key]) != canonical(batch[key]):
            raise ValueError("目录证据与 batch 绑定不一致")
    if evidence.get("batch_id") != batch_id or evidence.get("structure_cut") != row["next_cut"]:
        raise ValueError("目录证据 batch/cut 不一致")
    for key in ("classified_objects", "windows_total", "merged_bars", "effective_source_positions",
                "withdrawals", "replaces", "insufficient_knowledge", "domain_not_satisfied"):
        value = evidence.get(key)
        number = _int_text(value, key) if type(value) is str else value
        if type(number) is not int or not 0 <= number < 2**63:
            raise ValueError("目录计数不属于非负 i64")
    if row["catalog_run_status"] != ("run" if batch["objects"] else "not_run"):
        raise ValueError("目录运行状态与封存结果不符")
    sets = {name: _intern_set(delta[name], h, name, pool) for name in
            ("upserts", "withdrawals", "replaces", "witnesses", "relations", "observations", "raw_history_added")}
    upserts = []
    for obj in batch["objects"]:
        values = (obj["object_id"], obj["object_revision"], obj["kind"], batch_id,
                  obj["branch"], obj["dir_ab"], obj["dir_bc"], obj["window_start"], obj["window_mid"], obj["window_end"],
                  obj["comparisons_json"], obj["input_refs_json"], obj["source_coords_json"], obj["first_known_generation"],
                  obj["first_known_cut"], obj["published_generation"], None, None, None)
        value = h["_project_object_row"](values)
        previous = active.get(value["object_id"])
        if previous is not None:
            for key in ("batch_id", "first_known_generation", "first_known_cut", "published_generation"):
                value[key] = previous[key]
        upserts.append(value)
    _same_keys(sets["upserts"], _intern_set(upserts, h, "upserts/batch", pool), "upserts/batch")
    witnesses = [dict(w, slot=h["_num_to_str"](w["slot"]),
                      merged_source_index=h["_num_to_str"](w["merged_source_index"]),
                      raw_bars=h["_project_raw_bars"](w["raw_bars"])) for w in batch["witnesses"]]
    _same_keys(sets["witnesses"], _intern_set(witnesses, h, "witnesses/batch", pool), "witnesses/batch")
    relations = list(batch["relations"])
    relations.extend({"subject": r["new_object_id"], "relation_type": "replaces", "object": r["old_object_id"]}
                     for r in delta["replaces"])
    for item in raw[:frontier+1]:
        if item["supersedes_revision"] is not None:
            relations.append({"subject": item["identity_key"]+"@"+item["revision"], "relation_type": "supersedes",
                              "object": item["identity_key"]+"@"+item["supersedes_revision"]})
    _same_keys(sets["relations"], _intern_set(relations, h, "relations/batch", pool), "relations/batch")
    firsts, records = {}, []
    for ob in batch["observations"]:
        oid = h["_observation_id"](ob)
        if oid not in observations:
            original = dict(ob, observation_id=oid, batch_id=batch_id)
            for key in ("window_start", "window_mid", "window_end"):
                original[key] = h["_num_or_none_to_str"](original[key])
            observations[oid] = firsts[oid] = original
        records.append(observations[oid])
    _same_keys(sets["observations"], _intern_set(records, h, "observations/首封存版本", pool), "observations/首封存版本")
    return {"header": header, "sets": sets, "first_observations": firsts,
            "delta_public": zlib.compress(row["delta_json"].encode("utf-8"), 1),
            "raw_keys": _intern_set([_raw_wire(item) for item in batch["raw_events"]], h, "raw_history/batch", pool),
            "batch": {key: batch[key] for key in ("profile_id", "profile_hash", "protocol_revision",
                      "session_generation", "clock_plan_hash", "semantic_commit_ns") if key in batch}}


def project_delta(proof, index):
    """全历史原JSON以普通bytes无损保留；显式解压/解码，字段和数组原顺序不裁减。"""
    return dict(proof["deltas"][index], delta=parse_json(zlib.decompress(proof["delta_json"][index])))


def _verify_control(tables, meta, raw, deltas, batches, deadline=None):
    names = set(tables) - set(EXPECTED_LEGACY)
    if not names:
        if "protocol_revision" in meta:
            raise ValueError("协议标记存在但控制表缺失")
        return None
    if meta.get("protocol_revision") != "s-session/2":
        raise ValueError("控制表与协议版本标记不一致")
    # v2 新目录从 epoch 1 初始化且只递增；其 S producer_epoch 是正数。
    # 旧 v1 writer_epoch 的非负域在共享 typed 核中继续保留。
    _int_text(meta.get("writer_epoch"), "v2 meta.writer_epoch", 1)
    if any(not meta.get(key) for key in ("profile_id", "profile_hash", "profile_definition", "input_profile")):
        raise ValueError("v2 初始化缺完整持久 profile 绑定")
    if any(row["advance_state_at_transition"] != "idle" for row in tables["writer_epoch_history"]):
        raise ValueError("v2 换代历史必须记录 recover 后的 idle 状态")
    catalog_source = parse_json(meta.get("catalog_definition"))
    if hashlib.sha256(canonical(catalog_source)).hexdigest() != meta.get("catalog_hash") or catalog_source.get("catalog_revision") != meta["catalog_revision"]:
        raise ValueError("v2 catalog 定义/hash/目录版本不符")
    declared_items = catalog_source.get("items")
    if type(declared_items) is not list:
        raise ValueError("v2 catalog 定义缺完整 items")
    expected_catalog = {}
    for item in declared_items:
        _deadline(deadline)
        if type(item) is not dict or type(item.get("id")) is not str or item["id"] in expected_catalog:
            raise ValueError("v2 catalog 定义身份重复/损坏")
        expected_catalog[item["id"]] = {key: item.get(key, "") for key in ("kind", "title", "domain")}
        expected_catalog[item["id"]]["branches"] = item.get("branches", [])
    actual_catalog = {item["catalog_id"]: {"kind": item["kind"], "title": item["title"], "domain": item["domain"],
                      "branches": parse_json(item["branches_json"], (dict, list))} for item in tables["catalog"]}
    if canonical(actual_catalog) != canonical(expected_catalog):
        raise ValueError("完整 catalog 静态列与持久正本定义不一致")
    if len(tables["s_protocol_meta"]) != 1 or len(tables["s_delivery_policy"]) != 1:
        raise ValueError("协议/投递策略必须是唯一单例")
    protocol, policy = tables["s_protocol_meta"][0], tables["s_delivery_policy"][0]
    if protocol["singleton"] != 1 or protocol["protocol_revision"] != "s-session/2" or policy["singleton"] != 1:
        raise ValueError("协议单例/版本损坏")
    _int_text(protocol["session_generation"], "session_generation", 1)
    phase_frontier = (-1 if protocol["logical_phase_frontier"] == "-1" else
                      _int_text(protocol["logical_phase_frontier"], "logical_phase_frontier"))
    clock = parse_json(protocol["clock_plan_json"])
    if canonical(clock).decode() != protocol["clock_plan_json"] or hashlib.sha256(canonical(clock)).hexdigest() != protocol["clock_plan_hash"]:
        raise ValueError("clock 规范内容/hash 不闭合")
    if clock.get("schema_revision") != "s-clock-plan/1" or clock.get("unit") != "ns" or type(clock.get("events")) is not dict:
        raise ValueError("clock schema/事件形状不符")
    for key in ("clock_plan_id", "origin_utc"):
        if type(clock.get(key)) is not str or not clock[key]:
            raise ValueError("clock 必填来源字段缺失")
    for identity, event in clock["events"].items():
        _deadline(deadline)
        if not identity or type(event) is not dict:
            raise ValueError("clock 事件身份/对象形状损坏")
        if "recover_ns" in event:
            if set(event) != {"recover_ns"}:
                raise ValueError("recover clock 字段不符")
            _int_text(event["recover_ns"], "recover_ns")
        else:
            if set(event) != {"accept_ns", "attempts"} or type(event["attempts"]) is not list or not event["attempts"]:
                raise ValueError("输入 clock 缺 accept/非空 attempts")
            previous = _int_text(event["accept_ns"], "accept_ns")
            for attempt in event["attempts"]:
                if type(attempt) is not dict or set(attempt) != {"begin_ns", "commit_ns"}:
                    raise ValueError("clock attempt 字段不符")
                begin, commit = (_int_text(attempt[key], key) for key in ("begin_ns", "commit_ns"))
                if not previous <= begin <= commit:
                    raise ValueError("clock attempt 时间倒退")
                previous = commit
    gen = len(deltas)
    retain = policy["retain_generations"]
    first = max(1, gen-retain+1)
    if retain < 1 or policy["session_id"] != meta["session_id"] or policy["session_generation"] != protocol["session_generation"]:
        raise ValueError("投递策略 session/化身/保留数不符")
    if policy["policy_revision"] != "generation-window/1" or policy["head_generation"] != gen or policy["first_available_generation"] != first:
        raise ValueError("投递 floor/head 与永久代链不闭合")
    if [r["generation"] for r in tables["s_delivery_refs"]] != list(range(first, gen+1)):
        raise ValueError("普通投递引用缺失/未来/超保留范围")
    published_at = {}
    for index, delta in enumerate(deltas, 1):
        for seq in range(int(delta["seq_range"]["from"]), int(delta["input_frontier"])+1):
            published_at[seq] = index
    messages, by_key = {}, {}
    expected_header = {"schema_revision", "session_id", "session_generation", "source_namespace", "source_epoch",
                       "message_id", "producer_id", "producer_epoch", "payload_hash", "causal_refs", "payload"}
    for row in tables["s_input_messages"]:
        _deadline(deadline)
        envelope = parse_json(row["canonical_envelope"])
        if set(envelope) != expected_header or envelope.get("schema_revision") != "s-session/2" or canonical(envelope).decode() != row["canonical_envelope"]:
            raise ValueError("消息完整公共头/规范字节不符")
        for key in expected_header - {"causal_refs", "payload"}:
            if type(envelope[key]) is not str or not envelope[key]:
                raise ValueError("消息公共头必需文本损坏")
        if type(envelope["causal_refs"]) is not list or type(envelope["payload"]) is not dict:
            raise ValueError("消息 causal_refs/payload 形状损坏")
        for reference in envelope["causal_refs"]:
            if type(reference) is not dict or any(type(reference.get(key)) is not str or not reference[key]
                for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")):
                raise ValueError("消息 causal_refs 成员身份损坏")
        payload = envelope["payload"]
        if set(payload) != {"op", "target_session_id", "target_session_generation", "writer_epoch", "clock_event_id", "raw_input"}:
            raise ValueError("ingest 字段缺失/多余")
        if any(envelope[key] != row[key] for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")):
            raise ValueError("消息持久身份/hash 列与封存头不一致")
        if hashlib.sha256(canonical(payload)).hexdigest() != row["payload_hash"]:
            raise ValueError("消息 payload_hash 与当前规范字节不一致")
        if envelope["session_id"] != meta["session_id"] or envelope["session_generation"] != protocol["session_generation"]:
            raise ValueError("消息不属于当前 session/化身")
        if payload.get("op") != "ingest" or payload.get("clock_event_id") != row["clock_event_id"]:
            raise ValueError("消息 op/clock_event_id 与持久列不一致")
        if payload.get("target_session_id") != meta["session_id"] or payload.get("target_session_generation") != protocol["session_generation"]:
            raise ValueError("消息目标 session/化身不符")
        _int_text(payload.get("writer_epoch"), "消息 writer_epoch", 1)
        _int_text(envelope["producer_epoch"], "消息 producer_epoch", 1)
        seq = row["accepted_seq"]
        if not 0 <= seq < len(raw) or row["receipt_id"] != raw[seq]["receipt_id"]:
            raise ValueError("消息 receipt/accepted_seq 与原始接纳事实不闭合")
        source = payload.get("raw_input")
        if type(source) is not dict or type(source.get("events")) is not list or len(source["events"]) != 1:
            raise ValueError("消息原始输入不是单事件")
        if set(source) != {"schema_revision", "session_id", "source_namespace", "source_epoch", "instrument", "profile", "events"}:
            raise ValueError("消息原始输入字段缺失/多余")
        if any(source[key] != envelope[key] for key in ("source_namespace", "source_epoch")):
            raise ValueError("原始输入来源与公共头身份不同")
        if source.get("schema_revision") != "1" or source.get("session_id") != meta["session_id"]:
            raise ValueError("消息原始输入 schema/session 不符")
        if any(source.get(key) != raw[seq][key] for key in ("source_namespace", "source_epoch", "instrument")):
            raise ValueError("消息原始来源与接纳行不符")
        expected_event = {key: raw[seq][key] for key in ("event_id", "received_at", "raw_text", "price", "volume")}
        expected_event.update(revision=raw[seq]["input_revision"], seq=raw[seq]["source_coord"], timestamp=raw[seq]["ts"])
        if canonical(source["events"][0]) != canonical(expected_event) or source.get("profile") != meta.get("profile_id"):
            raise ValueError("消息原始内容/profile 与接纳行不符")
        published = published_at.get(seq)
        if row["first_published_generation"] != published or row["status"] != ("Committed" if published is not None else "AcceptedPending"):
            raise ValueError("消息首次发布结果与永久 Delta 前沿不符")
        if row["attempt_count"] < 0:
            raise ValueError("消息 attempt_count 负数")
        message_key = canonical([row[key] for key in ("source_namespace", "source_epoch", "message_id")]).decode()
        if row["clock_event_id"] in messages:
            raise ValueError("消息 clock 身份重复")
        messages[row["clock_event_id"]] = row
        by_key[message_key] = row
    phases, ordinals, begins_by_message, commits_by_generation = {}, {}, defaultdict(set), {}
    for row in tables["s_clock_events"]:
        _deadline(deadline)
        identity, phase, index = row["clock_event_id"], row["phase"], row["attempt_index"]
        event = clock["events"].get(identity)
        if type(event) is not dict:
            raise ValueError("phase 引用未知 clock 事件")
        number = _int_text(row["semantic_ns"], "phase.semantic_ns")
        if phase in ("accept", "recover"):
            expected_ns = event.get(phase+"_ns")
            if index != -1 or row["attempt_ordinal"] is not None:
                raise ValueError("accept/recover 不得携带 Begin 序号")
        elif phase in ("begin", "commit"):
            attempts = event.get("attempts", [])
            if not 0 <= index < len(attempts) or type(row["attempt_ordinal"]) is not int or row["attempt_ordinal"] < 1:
                raise ValueError("phase attempt 索引/序号不合法")
            expected_ns = attempts[index].get(phase+"_ns")
        else:
            raise ValueError("未知 phase")
        if row["semantic_ns"] != expected_ns or number > phase_frontier:
            raise ValueError("phase 与 clock 原值/逻辑前沿不符")
        if phase == "recover":
            if row["message_key"] != "" or not 0 <= row["generation"] <= gen:
                raise ValueError("recover phase 归属不符")
        else:
            message = messages.get(identity)
            if message is None or by_key.get(row["message_key"]) is not message:
                raise ValueError("phase 消息归属不符")
            if not 0 <= row["generation"] <= gen + int(phase == "begin"):
                raise ValueError("phase generation 越界")
            published = message["first_published_generation"]
            if phase == "begin" and row["generation"] != (gen+1 if published is None else published):
                raise ValueError("Begin 代际与消息首次发布/待推进目标不同")
            if phase == "commit" and published != row["generation"]:
                raise ValueError("Commit 代际与消息实际首次发布不同")
        phases[(identity, phase, index)] = row
        if phase == "begin":
            if row["attempt_ordinal"] in ordinals:
                raise ValueError("Begin 全局序号重复")
            ordinals[row["attempt_ordinal"]] = row
            begins_by_message[identity].add(index)
        if phase == "commit":
            if not 1 <= row["generation"] <= gen:
                raise ValueError("Commit phase 缺发布代")
            batch = batches[row["generation"]]
            if row["generation"] in commits_by_generation:
                raise ValueError("一个已发布代对应重复 Commit phase")
            commits_by_generation[row["generation"]] = row
            if any(batch.get(key) != value for key, value in (
                ("protocol_revision", "s-session/2"), ("session_generation", protocol["session_generation"]),
                ("clock_plan_hash", protocol["clock_plan_hash"]), ("semantic_commit_ns", row["semantic_ns"]))):
                raise ValueError("Commit phase 与封存批次语义时刻不闭合")
    if protocol["next_attempt_ordinal"] != len(ordinals)+1 or set(ordinals) != set(range(1, len(ordinals)+1)):
        raise ValueError("Begin 序号与 next_attempt_ordinal 不连续")
    if phase_frontier != max((_int_text(r["semantic_ns"], "semantic_ns") for r in phases.values()), default=-1):
        raise ValueError("logical_phase_frontier 与实际 phase 账本不一致")
    for identity, message in messages.items():
        _deadline(deadline)
        if (identity, "accept", -1) not in phases:
            raise ValueError("已接纳消息缺实际 accept phase")
        indexes = begins_by_message[identity]
        if len(indexes) != message["attempt_count"] or any(index != expected for expected, index in enumerate(sorted(indexes))):
            raise ValueError("消息 attempt_count 与 Begin 账本不符")
        if message["attempt_count"] and message["first_published_generation"] is not None and (identity, "commit", message["attempt_count"]-1) not in phases:
            raise ValueError("已发布消息缺最后一次 attempt 的 Commit")
    if set(commits_by_generation) != set(batches):
        raise ValueError("v2 已发布批次与实际 Commit phase 不完整闭合")
    if {m["accepted_seq"] for m in messages.values()} != set(range(len(raw))):
        raise ValueError("v2 原始接纳记录缺少持久消息承接")
    for (identity, phase, index), row in phases.items():
        if phase == "commit":
            begin = phases.get((identity, "begin", index))
            if begin is None or begin["attempt_ordinal"] != row["attempt_ordinal"] or begin["generation"] != row["generation"]:
                raise ValueError("Commit 缺对应 Begin")
    recover_facts = sorted((r["semantic_ns"], str(r["generation"])) for r in phases.values() if r["phase"] == "recover")
    epoch_facts = sorted((r["transitioned_at"], r["generation_at_transition"]) for r in tables["writer_epoch_history"])
    if recover_facts != epoch_facts:
        raise ValueError("v2 recover phase 与 writer 换代历史不闭合")
    protocol = dict(protocol, delivery_policy=policy)
    return protocol


def verify(image, h, deadline=None, memo=None):
    """一次处理全 typed 索引；各代只做该 batch/Delta 和该代变化的关联。"""
    _deadline(deadline)
    tables = _typed_rows(image, deadline)
    meta = dict((r["key"], r["value"]) for r in tables["meta"])
    generation = h["_validate_required_meta"](meta)
    epoch = _int_text(meta.get("writer_epoch"), "meta.writer_epoch")
    previous_epoch, previous_generation = None, 0
    for ordinal, transition in enumerate(tables["writer_epoch_history"], 1):
        start = _int_text(transition["from_epoch"], "epoch.from")
        end = _int_text(transition["to_epoch"], "epoch.to")
        known = _int_text(transition["generation_at_transition"], "epoch.generation")
        _int_text(transition["transitioned_at"], "epoch.transitioned_at")
        if transition["ordinal"] != ordinal or (previous_epoch is not None and start != previous_epoch) or end <= start or not previous_generation <= known <= generation:
            raise ValueError("writer epoch 换代历史不连续/越界")
        previous_epoch, previous_generation = end, known
    if previous_epoch is not None and previous_epoch != epoch:
        raise ValueError("meta.writer_epoch 与持久换代历史不闭合")
    if type(meta.get("rule_revision")) is not str or not meta["rule_revision"]:
        raise ValueError("meta.rule_revision 缺失")
    _verify_raw(tables["raw_events"], deadline)
    raw = [_raw_wire(row) for row in tables["raw_events"]]
    batches = {r["batch_id"]: r for r in tables["batches"]}
    deltas = tables["structure_deltas"]
    if len(deltas) != generation:
        raise ValueError("已发布代链断裂或存在未来 Delta")
    binding = (meta.get("profile_id"), meta.get("profile_hash"))
    if not raw and binding == (None, None) and meta.get("input_profile") is None and meta.get("profile_definition") is None:
        binding = ("", "")
    else:
        pid, phash = binding
        if type(pid) is not str or not pid or type(phash) is not str or len(phash) != 64 or any(c not in "0123456789abcdef" for c in phash):
            raise ValueError("固定 profile_id/hash 不规范")
        if meta.get("input_profile") != pid:
            raise ValueError("input_profile 与固定 profile 不一致")
        definition = meta.get("profile_definition")
        if definition is not None:
            profile = parse_json(definition)
            if profile.get("profile_id") != pid or hashlib.sha256(canonical(profile)).hexdigest() != phash:
                raise ValueError("profile 定义与固定 ID/hash 不一致")
        elif not deltas:
            profile = parse_json((h["Path"](h["__file__"]).resolve().parent /
                                  "profiles/testonly_tick_1_1_ohlc.json").read_bytes())
            if profile.get("profile_id") != pid or hashlib.sha256(canonical(profile)).hexdigest() != phash:
                raise ValueError("旧未发布 profile 没有可核来源")

    # memo是进程内已核源的派生集合；每次捕获完整typed材料后才允许比对原字节。
    context = canonical((image["schema_rows"], image["schema"], binding,
                         [(key, meta.get(key)) for key in ("session_id", "catalog_revision", "rule_revision",
                          "scope", "profile_definition", "input_profile")]))
    previous_entries = memo.get("entries", []) if memo is not None and memo.get("context") == context else []
    reusable = bool(previous_entries)
    next_entries, pool, memo_hits = [], {}, 0
    for certificate in previous_entries:
        for values in (*certificate["sets"].values(), certificate["raw_keys"]):
            for key in values:
                pool.setdefault(key, key)
    raw_keys = []
    for item in raw:
        key = canonical(h["_legacy_payload_projection"](item))
        raw_keys.append(pool.setdefault(key, key))
    if len(set(raw_keys)) != len(raw_keys):
        raise ValueError("raw_history 含重复成员")
    births, deaths = defaultdict(list), defaultdict(list)
    active, wires = {}, {"objects": [], "witnesses": [], "relations": [], "observations": []}
    for table in ("objects", "witnesses", "relations", "observations"):
        for row in tables[table]:
            _deadline(deadline)
            for key in (("first_known_generation", "published_generation", "withdrawn_generation")
                        if table == "objects" else ("published_generation",)):
                number = row[key]
                if number is None and key == "withdrawn_generation":
                    continue
                if type(number) is not int or not 1 <= number <= generation:
                    raise ValueError(f"{table}.{key} 不属于 1..G 发布代")
            if table == "objects":
                first, withdrawn = row["first_known_generation"], row["withdrawn_generation"]
                if withdrawn is not None and withdrawn <= first:
                    raise ValueError("对象生命周期非法")
                value = h["_project_object_row"](tuple(row.values()))
                births[first].append(value)
                if withdrawn is not None:
                    deaths[withdrawn].append(value)
            elif table == "witnesses":
                value = {k: row[k] for k in ("witness_id", "object_id", "merged_high", "merged_low", "merged_open", "merged_close")}
                value.update(slot=str(row["slot"]), merged_source_index=str(row["merged_source_index"]),
                             raw_bars=h["_project_raw_bars"](parse_json(row["raw_json"], list)))
            elif table == "relations":
                value = {k: row[k] for k in ("subject", "relation_type", "object")}
            else:
                value = {k: row[k] for k in ("observation_id", "batch_id", "kind", "reason")}
                value.update({k: None if row[k] is None else str(row[k]) for k in ("window_start", "window_mid", "window_end")})
                value["detail"] = parse_json(row["detail_json"])
            wires[table].append((row["published_generation"], value))

    stored, born, seen = {}, {}, {}
    for table in ("witnesses", "relations", "observations"):
        stored[table], born[table], seen[table] = {}, defaultdict(set), set()
        for published, value in wires[table]:
            key = canonical(h["_legacy_payload_projection"](value))
            if key in stored[table]:
                raise ValueError(table + " 独立索引包含重复语义记录")
            stored[table][key] = published
            born[table][published].add(key)
    original_observations = {}
    verified_deltas, decoded_batches, delta_json = [], {}, []
    active_keys = {}
    prev_frontier = -1
    for gen, row in enumerate(deltas, 1):
        _deadline(deadline)
        if row["generation"] != gen or row["session_id"] != meta["session_id"] or row["catalog_revision"] != meta["catalog_revision"]:
            raise ValueError("Delta generation/session/catalog 链不一致")
        if row["base_cut"] != f"cut-{gen-1}" or row["next_cut"] != f"cut-{gen}":
            raise ValueError("Delta cut 链不一致")
        frontier = h["_frontier_i64"](row["input_frontier"])
        if frontier < prev_frontier or frontier >= len(raw):
            raise ValueError("Delta 输入前沿不属于已接纳 raw")
        batch_id = row["index_frontier"]
        entry = batches.get(batch_id)
        if entry is None or batch_id != "batch-" + hashlib.sha256(entry["canonical_bytes"]).hexdigest() or entry["byte_len"] != len(entry["canonical_bytes"]):
            raise ValueError("可达 batch 内容/长度/哈希不闭合")
        # 全typed列逐值一致，Delta原文另按UTF-8字节比较；避免每次重新转义整个大JSON文本。
        # _typed_rows已核确切SQL类型，tuple保留列名、次序、值及null，无字符串化降格。
        row_columns = tuple((key, value) for key, value in row.items() if key != "delta_json")
        certificate = previous_entries[gen-1] if reusable and gen <= len(previous_entries) else None
        if certificate is not None:
            if (certificate["delta_columns"] != row_columns
                    or zlib.decompress(certificate["delta_public"]) != row["delta_json"].encode("utf-8")
                    or zlib.decompress(certificate["batch_source"]) != entry["canonical_bytes"]):
                certificate = None
        if certificate is None:
            # 一处源变化后，后续first-known/首版observation依赖全部重新建立。
            reusable = False
            certificate = _sealed_generation(row, entry, gen, frontier, meta, binding, active,
                                             original_observations, raw, h, pool)
            if memo is not None:
                certificate["delta_columns"] = row_columns
                certificate["batch_source"] = zlib.compress(entry["canonical_bytes"], 1)
        else:
            memo_hits += 1
            for oid, original in certificate["first_observations"].items():
                if oid in original_observations:
                    raise ValueError("memo首版observation顺序不闭合")
                original_observations[oid] = original
        if memo is not None:
            next_entries.append(certificate)
        header, sets = certificate["header"], certificate["sets"]
        if header["seq_range"] != {"from": str(prev_frontier+1), "to": str(frontier)}:
            raise ValueError("Delta seq_range 与前沿链不一致")
        # 无论命中memo与否，独立索引生命周期和全raw前缀均来自本次完整typed捕获。
        for obj in births[gen]:
            value = dict(obj, withdrawn_generation=None, withdrawal_reason=None,
                         superseded_by=None, lifecycle="active")
            active[obj["object_id"]] = value
            key = canonical(h["_legacy_payload_projection"](value))
            active_keys[obj["object_id"]] = pool.setdefault(key, key)
        withdrawals, replaces = [], []
        for obj in deaths[gen]:
            if active.pop(obj["object_id"], None) is None:
                raise ValueError("对象撤回无活动前件")
            active_keys.pop(obj["object_id"])
            withdrawals.append({"object_id": obj["object_id"], "window_start": obj["window_start"],
                                "window_mid": obj["window_mid"], "window_end": obj["window_end"],
                                "reason": obj["withdrawal_reason"], "superseded_by": obj["superseded_by"]})
            if obj["superseded_by"] is not None:
                replaces.append({"old_object_id": obj["object_id"], "new_object_id": obj["superseded_by"]})
        current_objects = frozenset(active_keys.values())
        if len(current_objects) != len(active):
            raise ValueError("objects/独立索引 含重复成员")
        _same_keys(sets["upserts"], current_objects, "objects/独立索引")
        _same_keys(sets["withdrawals"], _intern_set(withdrawals, h, "withdrawals", pool), "withdrawals")
        _same_keys(sets["replaces"], _intern_set(replaces, h, "replaces", pool), "replaces")
        _same_keys(certificate["raw_keys"], frozenset(raw_keys[:frontier+1]), "raw_history/batch")
        _same_keys(sets["raw_history_added"], frozenset(raw_keys[prev_frontier+1:frontier+1]), "raw_history_added")
        for table in ("witnesses", "relations", "observations"):
            added = sets[table]
            for key in added:
                published = stored[table].get(key)
                if published is None or published > gen:
                    raise ValueError(table + " 内容/发布代与独立索引不一致")
                if key not in seen[table] and published != gen:
                    raise ValueError(table + " 首次出现代与持久发布代不一致")
            if not born[table][gen] <= added:
                raise ValueError(table + " 含该代没有发布的幽灵端点")
            seen[table].update(added)
        verified_deltas.append(header)
        decoded_batches[gen] = certificate["batch"]
        delta_json.append(certificate["delta_public"])
        prev_frontier = frontier
    for table in seen:
        if seen[table] != stored[table].keys():
            raise ValueError(table + " 存在未被任何已发布代证明的索引")
    catalog = tables["catalog"]
    if meta.get("profile_definition") is None and binding != ("", "") and not any(b["profile_id"] for b in decoded_batches.values()):
        profile = parse_json((h["Path"](h["__file__"]).resolve().parent / "profiles/testonly_tick_1_1_ohlc.json").read_bytes())
        if (profile.get("profile_id"), hashlib.sha256(canonical(profile)).hexdigest()) != binding:
            raise ValueError("旧 profile 缺可核封存/声明来源")
    if generation:
        cc = [r for r in catalog if r["catalog_id"] == "CC-006"]
        last = verified_deltas[-1]
        if len(cc) != 1 or cc[0]["run_status"] != last["catalog_run_status"] or canonical(parse_json(cc[0]["evidence_json"])) != canonical(last["catalog_evidence"]):
            raise ValueError("当前目录与末代 Delta 证据不一致")
        if any(meta[key] != last[dkey] for key, dkey in (("index_frontier", "index_frontier"),
               ("structure_cut", "next_cut"), ("last_advance_frontier", "input_frontier"))):
            raise ValueError("meta 与末代 Delta 根元组不一致")
    protocol = _verify_control(tables, meta, raw, verified_deltas, decoded_batches, deadline)
    digest = capture_digest(image)
    _deadline(deadline)
    # 完整新鲜审计结束后才缩减保留量；上方所有当前typed行、batch原字节和关联门仍必跑。
    # 投影依赖完整清单：meta/generation身份与根、raw公开历史、wires四类索引、
    # catalog全列、完整逐代Delta、每代profile绑定、protocol投递策略及本次capture_digest。
    # 原image、其余typed表及已解码batch的重复历史仅属验证材料，不是后续查询依赖。
    # 下一次data_version变化仍从当前全库重新capture/verify，不能拿此精简值替代审计输入。
    profiles = {gen: {key: batch[key] for key in ("profile_id", "profile_hash")}
                for gen, batch in decoded_batches.items()}
    proof = {"meta": meta, "generation": generation, "raw": raw, "wires": wires,
             "tables": {"catalog": catalog}, "deltas": verified_deltas, "delta_json": tuple(delta_json),
             "batches": profiles, "capture_digest": digest, "protocol": protocol}
    if memo is not None:
        proof["_audit_memo"] = {"context": context, "entries": next_entries,
                                "hits": memo_hits, "misses": generation-memo_hits}
    return proof


def project_state(proof, as_of, h):
    """从已核不可变图像投影一个 cut；不查询 SQLite，也不重新做结构判定。"""
    current = proof["generation"]
    gen = current if as_of is None else as_of
    if type(gen) is not int or gen < 0:
        raise h["InvalidQuery"]("as_of 必须是非负整数")
    if gen > current:
        raise ValueError("Unavailable：请求 cut 尚未发布")
    meta = proof["meta"]
    delta = proof["deltas"][gen-1] if gen else None
    publication = ({key: delta[key] for key in ("input_frontier", "seq_range", "catalog_run_status", "catalog_evidence")}
                   if gen else dict(input_frontier="-1", seq_range={"from": "0", "to": "-1"},
                                    catalog_run_status="not_run", catalog_evidence={}))
    header = dict(session_id=meta["session_id"], generation=str(gen), structure_cut=f"cut-{gen}",
                  catalog_revision=meta["catalog_revision"], index_frontier=delta["index_frontier"] if gen else "")
    batch = proof["batches"].get(gen)
    snapshot = dict(publication, **header, scope=h["_scope"](meta),
                    profile_id=batch["profile_id"] if batch else "", profile_hash=batch["profile_hash"] if batch else "",
                    history_mode="RecomputedWithRevision" if as_of is None else "AsKnown",
                    as_of_generation=None if as_of is None else str(gen))
    active, withdrawn = [], []
    for _published, obj in proof["wires"]["objects"]:
        if int(obj["first_known_generation"]) > gen:
            continue
        wg = obj["withdrawn_generation"]
        if wg is None or int(wg) > gen:
            active.append(dict(obj, withdrawn_generation=None, withdrawal_reason=None,
                               superseded_by=None, lifecycle="active"))
        else:
            withdrawn.append(obj)
    snapshot["objects"] = sorted(active, key=lambda o: int(o["window_start"]))
    snapshot["withdrawn_objects"] = sorted(withdrawn, key=lambda o: int(o["first_known_generation"]))
    snapshot["witnesses"] = sorted([v for pg, v in proof["wires"]["witnesses"] if pg <= gen],
                                    key=lambda v: (v["object_id"], int(v["slot"])))
    snapshot["relations"] = sorted([v for pg, v in proof["wires"]["relations"] if pg <= gen],
                                    key=lambda v: (v["subject"], v["relation_type"], v["object"]))
    snapshot["observations"] = sorted([v for pg, v in proof["wires"]["observations"] if pg <= gen],
        key=lambda v: (v["kind"], -1 if v["window_start"] is None else int(v["window_start"])))
    snapshot["raw_history"] = proof["raw"][:int(publication["input_frontier"])+1]
    items = []
    for row in proof["tables"]["catalog"]:
        item = {"id": row["catalog_id"], "kind": row["kind"], "title": row["title"], "domain": row["domain"],
                "branches": parse_json(row["branches_json"], (list, dict)),
                "implementation_status": row["impl_status"], "proof_status": row["proof_status"],
                "run_status": row["run_status"], "evidence": parse_json(row["evidence_json"])}
        if as_of is not None:
            item["implementation_status"] = "implemented" if item["id"] == "CC-006" and gen else "not_implemented"
            item["proof_status"] = "not_proved"
            if gen == 0 or item["id"] == "CC-006":
                item["run_status"], item["evidence"] = publication["catalog_run_status"], publication["catalog_evidence"]
        items.append(item)
    catalog = dict(publication, **header, scope=h["_scope"](meta), items=items,
                   counts={"implemented": sum(i["implementation_status"] == "implemented" for i in items),
                           "not_implemented": sum(i["implementation_status"] == "not_implemented" for i in items),
                           "run": sum(i["run_status"] == "run" for i in items),
                           "not_run": sum(i["run_status"] == "not_run" for i in items)})
    return h["_project_wire_integers"]({"cut": header, "catalog": catalog, "snapshot": snapshot})
