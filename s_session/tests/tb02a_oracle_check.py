#!/usr/bin/env python3
"""#1373：独立冻结手算核验及完整双跑比对；不实现包含或分型算法。

只读取已结束运行的原件。手算未给出的语义字段明确单列；全字段双跑相等、
六族与持久表相等均不能冒充一般域语义证明。任何缺失、截断或失败返回非零。
"""

import argparse
import base64
from collections import Counter
from contextlib import closing
import datetime
import hashlib
import json
from pathlib import Path
import sqlite3


FAMILIES = ("objects", "withdrawn_objects", "witnesses", "relations", "observations", "raw_history")
TABLES = tuple(sorted(("batches", "catalog", "meta", "objects", "observations", "raw_events", "relations",
                       "s_clock_events", "s_delivery_policy", "s_delivery_refs", "s_input_messages",
                       "s_protocol_meta", "structure_deltas", "witnesses", "writer_epoch_history",
                       "raw_ohlc", "structure_facts")))
KINDS = ("CC-004.inclusion_step", "CC-005.inclusion_group", "CC-006.local_shape",
         "CC-007.fractal_description", "CC-054.knowledge_state")
PRICE_FIELDS = ("open", "high", "low", "close")
MAX_JSON_BYTES = 64 * 1024 * 1024
MAX_RECORD_BYTES = 16 * 1024 * 1024
CAPTURED = "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER"


class CheckFailure(ValueError):
    """具名证据与冻结义务不相符。"""


def unique_pairs(pairs: list) -> dict:
    """拒绝重复键，避免静默覆盖证据。"""
    result = {}
    for key, value in pairs:
        if key in result:
            raise CheckFailure("重复 JSON 键：" + key)
        result[key] = value
    return result


def reject_number(value: str) -> None:
    """证据中的小数和非有限 JSON 数不得隐式失精。"""
    raise CheckFailure("非精确 JSON 数：" + value)


def parse(raw: bytes | str):
    """同一严格解码用于原件、数据库 JSON 和行记录。"""
    return json.loads(raw, object_pairs_hook=unique_pairs, parse_float=reject_number,
                      parse_constant=reject_number)


def canonical(value) -> bytes:
    """仅用于摘要和数据库无序行；不用于双跑记录改写。"""
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def load(path: Path):
    """有界读取，不把截断 JSON 当作成功原件。"""
    with path.open("rb") as stream:
        raw = stream.read(MAX_JSON_BYTES + 1)
    if len(raw) > MAX_JSON_BYTES:
        raise CheckFailure("JSON 原件超过读取界限：" + str(path))
    return parse(raw)


def read_core(path: Path) -> tuple[bytes, list]:
    """保留原字节；每条记录必须完整换行、非空且无重复键。"""
    chunks, records = [], []
    with path.open("rb") as stream:
        while raw := stream.readline(MAX_RECORD_BYTES + 1):
            if len(raw) > MAX_RECORD_BYTES or not raw.endswith(b"\n") or raw == b"\n":
                raise CheckFailure("语义记录超界、截断或为空：" + str(path))
            record = parse(raw)
            if not isinstance(record, dict) or not record:
                raise CheckFailure("语义记录不是非空对象：" + str(path))
            chunks.append(raw)
            records.append(record)
    return b"".join(chunks), records


class Audit:
    """计数所有实际完成的断言；首个失败带字段路径返回。"""

    def __init__(self):
        self.checks = 0
        self.leaf_values = 0

    def require(self, condition: bool, path: str) -> None:
        self.checks += 1
        if not condition:
            raise CheckFailure(path)

    def equal(self, actual, expected, path: str) -> None:
        self.require(type(actual) is type(expected), path + ": 类型不同")
        if isinstance(expected, dict):
            self.require(set(actual) == set(expected), path + ": 字段集合不同")
            for key in expected:
                self.equal(actual[key], expected[key], path + "/" + str(key))
        elif isinstance(expected, list):
            self.require(len(actual) == len(expected), path + ": 数组长度不同")
            for index, (left, right) in enumerate(zip(actual, expected)):
                self.equal(left, right, path + "/" + str(index))
        else:
            self.leaf_values += 1
            self.require(actual == expected, path + ": 实际 " + repr(actual)[:180]
                         + "；期望 " + repr(expected)[:180])


def index_unique(rows: list, keys: tuple, audit: Audit, label: str) -> dict:
    indexed = {tuple(row[key] for key in keys): row for row in rows}
    audit.require(len(indexed) == len(rows), label + ": 身份重复")
    return indexed


def decimal_tree(value):
    """持久引用中整数到 wire 文本的机械编码；不重排或筛字段。"""
    if isinstance(value, dict):
        return {key: decimal_tree(item) for key, item in value.items()}
    if isinstance(value, list):
        return [decimal_tree(item) for item in value]
    return str(value) if type(value) is int else value


def scalar(value):
    if isinstance(value, bytes):
        return {"sqlite_type": "blob", "base64": base64.b64encode(value).decode("ascii"),
                "bytes": str(len(value)), "sha256": digest(value)}
    return value


def table_evidence(evidence: Path, records: list, audit: Audit) -> dict:
    tables = {row["table"]: row for row in records if row["kind"] == "authoritative_table"}
    audit.equal(sorted(tables), list(TABLES), "完整权威表清单")
    output = {}
    uri = (evidence / "database-backup.sqlite").resolve().as_uri() + "?mode=ro"
    with closing(sqlite3.connect(uri, uri=True, timeout=1)) as connection:
        names = [row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
        audit.equal(names, list(TABLES), "备份权威表清单")
        for name in TABLES:
            columns = [row[1] for row in connection.execute('PRAGMA table_info("' + name + '")')]
            rows = [[scalar(item) for item in row] for row in connection.execute('SELECT * FROM "' + name + '"')]
            rows.sort(key=canonical)
            audit.equal(tables[name], {"kind": "authoritative_table", "table": name,
                                      "columns": columns, "rows": rows}, "备份/语义表/" + name)
            output[name] = [dict(zip(columns, row)) for row in rows]
        schema = list(connection.execute("SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name"))
    actual = [row for row in records if row["kind"] == "authoritative_schema"]
    audit.equal(actual, [{"kind": "authoritative_schema", "rows": [list(row) for row in schema]}], "完整schema")
    return output


def object_wire(row: dict, generation: int) -> dict:
    value = {}
    for key, item in row.items():
        if key.endswith("_json"):
            value[key[:-5]] = decimal_tree(parse(item))
        else:
            value[key] = str(item) if type(item) is int else item
    withdrawn = row["withdrawn_generation"]
    if withdrawn is None or withdrawn > generation:
        value.update(withdrawn_generation=None, withdrawal_reason=None, superseded_by=None, lifecycle="active")
    else:
        value["lifecycle"] = "withdrawn"
    return value


def public_raw_bar(row: dict) -> dict:
    """OHLC wire 不暴露 close 的内部兼容索引；原始列仍在全表/双跑保全。"""
    value = decimal_tree(row)
    if value.get("schema_revision") == "s-ohlc/1":
        if value.get("price") != value.get("close"):
            raise CheckFailure("witness 原件的内部 price 与 close 矛盾")
        del value["price"]
    return value


def other_wire(family: str, row: dict, ohlc: dict) -> dict:
    if family == "relations":
        return {key: row[key] for key in ("subject", "relation_type", "object")}
    if family == "observations":
        value = {key: item for key, item in row.items() if key not in ("published_generation", "detail_json")}
        for key in ("window_start", "window_mid", "window_end"):
            value[key] = str(value[key]) if value[key] is not None else None
        value["detail"] = parse(row["detail_json"])
        return value
    if family == "witnesses":
        value = {key: item for key, item in row.items() if key not in ("published_generation", "raw_json")}
        for key in ("slot", "merged_source_index"):
            value[key] = str(value[key])
        value["raw_bars"] = [public_raw_bar(bar) for bar in parse(row["raw_json"])]
        return value
    value = decimal_tree(row)
    prices = ohlc[(row["identity_key"], row["revision"])]
    value.pop("price")
    value.update({key: prices[key] for key in ("schema_revision", *PRICE_FIELDS)})
    return value


def expected_families(tables: dict, generation: int) -> dict:
    """已封存表到 wire 的字段投影，无市场判断或补算。"""
    result = {family: [] for family in FAMILIES}
    for table in ("objects", "structure_facts"):
        for row in tables[table]:
            if row["first_known_generation"] <= generation:
                value = object_wire(row, generation)
                family = "objects" if value["lifecycle"] == "active" else "withdrawn_objects"
                result[family].append(value)
    ohlc = {(row["identity_key"], row["revision"]): row for row in tables["raw_ohlc"]}
    for family in FAMILIES[2:]:
        name = "raw_events" if family == "raw_history" else family
        for row in tables[name]:
            if (row["seq"] < generation if family == "raw_history" else row["published_generation"] <= generation):
                result[family].append(other_wire(family, row, ohlc))
    return result


def candidate_evidence(candidate: dict, tables: dict, generation: int, mode: str, audit: Audit) -> dict:
    snapshot, projection = candidate["state"]["snapshot"], candidate["projection"]
    fixed = projection["fixed_cut"]
    audit.equal(fixed["cut_generation"], str(generation), "公共cut")
    audit.equal(fixed["history_mode"], mode, "公共历史模式")
    audit.equal(projection["omissions"], [], "公开无省略")
    audit.equal(projection["order_version"], "s-record-order/2", "OHLC记录全序版本")
    audit.equal(candidate["cursor"]["after_generation"], str(generation), "完整cursor")
    expected = expected_families(tables, generation)
    count = 0
    audit.require(all(row["family"] in FAMILIES for row in projection["rows"]), "不得增加未知记录族")
    for family in FAMILIES:
        wire = [row["record"] for row in projection["rows"] if row["family"] == family]
        audit.equal(snapshot[family], wire, "snapshot/完整分页/" + family)
        audit.equal(projection["counts"][family], str(len(wire)), "分页计数/" + family)
        audit.equal(sorted(wire, key=canonical), sorted(expected[family], key=canonical), "持久全字段/" + family)
        count += len(wire)
    audit.equal(projection["counts"]["total"], str(count), "分页总记录数")
    audit.equal(set(obj["kind"] for obj in snapshot["objects"]) - set(KINDS), set(), "活动kind范围")
    return snapshot


def selected(snapshot: dict, kind: str, key: str, audit: Audit) -> dict:
    rows = [obj for obj in snapshot["objects"] if obj["kind"] == kind]
    indexed = {str(obj["payload"][key]): obj for obj in rows}
    audit.require(len(rows) == len(indexed), kind + " 槽重复")
    return indexed


def bar_expected(bar: dict) -> dict:
    return {"source_coord": str(bar["raw_index"]), **{key: str(bar[key]) for key in PRICE_FIELDS}}


def group_expectation(group: dict) -> dict:
    return {"group_index": str(group["merged_index"]), "group_anchor": str(group["group_anchor_raw_index"]),
            "members": [str(value) for value in group["raw_member_indices"]],
            **{key: str(group[key]) for key in PRICE_FIELDS},
            "high_sources": [str(value) for value in group["high_raw_indices"]],
            "low_sources": [str(value) for value in group["low_raw_indices"]]}


def check_groups(snapshot: dict, expected: dict, audit: Audit, bars: list) -> dict:
    groups = selected(snapshot, KINDS[1], "group_index", audit)
    audit.equal(sorted(groups, key=int), [str(row["merged_index"]) for row in expected["groups"]], "手算组全集")
    mapping = {}
    for group in expected["groups"]:
        actual = groups[str(group["merged_index"])]["payload"]
        fields = group_expectation(group)
        audit.equal({key: actual[key] for key in fields}, fields, "手算组/" + str(group["merged_index"]))
        initial = group["merged_index"] == 0
        audit.equal(actual["waiting_reasons"], ["direction_not_yet_established"] if initial else [], "组自身方向缺口须独立保留")
        later = group["merged_index"] + 1 < len(expected["groups"])
        audit.equal(actual["confirmed"], later, "具名完整前缀中的组确认边界")
        confirmation = actual["confirmation_evidence"]
        if later:
            next_anchor = expected["groups"][group["merged_index"] + 1]["group_anchor_raw_index"]
            audit.equal(confirmation["previous_acc"], {"source_coord": str(group["group_anchor_raw_index"]),
                **{key: str(group[key]) for key in PRICE_FIELDS}}, "组关闭时的真实累加器")
            audit.equal(confirmation["incoming"], bar_expected(bars[next_anchor]), "确认的真实后继来源")
        else:
            audit.equal(confirmation, None, "未见后继不得确认尾组")
        audit.equal([int(actual["members"][0]), int(actual["members"][-1])], group["raw_range_inclusive"], "闭区间成员端点")
        for member in actual["members"]:
            audit.require(int(member) not in mapping, "原始成员不能属于两个组")
            mapping[int(member)] = int(actual["group_index"])
    audit.equal([mapping[index] for index in range(len(expected["raw_to_merged"]))], expected["raw_to_merged"], "双向成员映射")
    return groups


def check_steps(snapshot: dict, expected: dict, bars: list, prefixes: dict | None, audit: Audit) -> None:
    actual_steps = [obj for obj in snapshot["objects"] if obj["kind"] == KINDS[0]]
    steps = {obj["payload"]["incoming"]["source_coord"]: obj["payload"] for obj in actual_steps}
    audit.equal(len(steps), len(actual_steps), "步骤同槽不得重复")
    audit.equal(sorted(steps, key=int), [str(row["raw_index"]) for row in expected["fold_steps"]], "手算步骤全集")
    for item in expected["fold_steps"]:
        index = item["raw_index"]
        actual = steps[str(index)]
        action = "seed" if item["branch"] == "SINGLE" else "merge" if item["branch"].startswith("MERGE_") else "establish_direction" if index == 1 else "new_group"
        audit.equal(actual["action"], action, "手算步骤/action/" + str(index))
        audit.equal(actual["incoming"], bar_expected(bars[index]), "手算步骤/incoming/" + str(index))
        audit.equal(actual["direction"], item["direction"], "手算步骤/direction/" + str(index))
        audit.equal(actual["waiting_reasons"], [], "成功步骤无等待")
        audit.equal([actual["acc_after"]["low"], actual["acc_after"]["high"]], [str(item["tail_low"]), str(item["tail_high"])], "步骤尾价")
        audit.equal(actual["high_sources"], list(map(str, item["tail_high_raw_indices"])), "步骤高根")
        audit.equal(actual["low_sources"], list(map(str, item["tail_low_raw_indices"])), "步骤低根")
        evidence = actual["direction_evidence"]
        pair = None if evidence is None else [int(evidence["previous_acc"]["source_coord"]), int(evidence["incoming"]["source_coord"])]
        audit.equal(pair, item["source_pair_anchor_raw_indices"], "建立方向的原始组锚")
        if prefixes is not None:
            current = selected(prefixes[index + 1], KINDS[1], "group_index", audit)
            tail = current[str(len(current) - 1)]["payload"]
            audit.equal(tail["members"], list(map(str, item["tail_member_indices"])), "真实前缀的尾组完整成员")


def check_windows(snapshot: dict, expected: dict, audit: Audit) -> list:
    shapes = [obj for obj in snapshot["objects"] if obj["kind"] == KINDS[2]]
    indexed = index_unique(shapes, ("window_start", "window_mid", "window_end"), audit, "三K窗口")
    expected_keys = [tuple(map(str, window["group_anchor_raw_indices"])) for window in expected["windows"]]
    audit.equal(sorted(indexed), sorted(expected_keys), "手算窗口全集")
    for window, key in zip(expected["windows"], expected_keys):
        obj = indexed[key]
        audit.equal(obj["branch"], window["shape"], "手算四叶/" + str(key))
        groups = [expected["groups"][index] for index in window["merged_indices"]]
        comparisons = index_unique(obj["comparisons"], ("pair", "axis"), audit, "四个真实严格比较")
        audit.equal(set(comparisons), {("ab", "high"), ("ab", "low"), ("bc", "high"), ("bc", "low")}, "四比较全集")
        for i, (pair, axis) in enumerate((("ab", "high"), ("ab", "low"), ("bc", "high"), ("bc", "low"))):
            before, after = (groups[0], groups[1]) if pair == "ab" else (groups[1], groups[2])
            order = window["comparisons"][i]
            strict_up = order == ("GT" if pair == "ab" else "LT")
            audit.equal(comparisons[(pair, axis)], {"pair": pair, "axis": axis, "prev": str(before[axis]),
                                                   "cur": str(after[axis]), "strict_up": strict_up, "held": True}, "手算四比较")
    for edge in expected["edge_requests"]:
        anchor = str(expected["groups"][edge["merged_index"]]["group_anchor_raw_index"])
        audit.require(not any(row["window_mid"] == anchor for row in shapes), "边界不足不能成为第五分型")
    return shapes


def check_descriptions(snapshot: dict, expected: dict, shapes: list, audit: Audit) -> None:
    descriptions = [obj["payload"] for obj in snapshot["objects"] if obj["kind"] == KINDS[3]]
    shape_ids = {obj["object_id"] for obj in shapes if obj["branch"] in ("TOP", "BOTTOM")}
    audit.equal({row["shape_object_id"] for row in descriptions}, shape_ids, "仅真实顶底产生描述")
    audit.equal(len(descriptions), len(shape_ids), "描述一一对应")
    for row in descriptions:
        labels = row["descriptive_labels"]
        audit.equal(labels["status"], "not_determined", "无数值规则不补强弱")
        audit.equal(labels["reason"], "no_settled_numeric_rule", "未裁描述规则")
        audit.equal(labels["labels"], [], "不制造强弱标签")
    audit.equal(expected["descriptions"], {"numerical_strength_threshold": None,
                                          "strength_label": None, "later_development_events": []}, "冻结描述边界")


def check_waiting(snapshot: dict, expected: dict, audit: Audit) -> None:
    index = expected["waiting_raw_index"]
    steps = [obj["payload"] for obj in snapshot["objects"] if obj["kind"] == KINDS[0]
             and obj["payload"]["incoming"]["source_coord"] == str(index)]
    audit.equal(len(steps), 1, "等待点完整步骤")
    step = steps[0]
    reason = "initial_direction_unsettled" if expected["direction"] is None else "equal_extreme_identity"
    # 有方向的 tie 可保留数值合并与全部候选根；未获裁的是唯一根选择。
    audit.equal(step["action"], "waiting" if expected["direction"] is None else "merge", "等待阶段与已执行数值折叠分开")
    audit.equal(step["direction"], expected["direction"], "等待方向")
    audit.require(reason in step["waiting_reasons"], "具名等待原因缺失")
    audit.equal([step["acc_before"]["low"], step["acc_before"]["high"]], list(map(str, expected["accumulator"])), "等待实际累加器")
    audit.equal([step["incoming"]["low"], step["incoming"]["high"]], list(map(str, expected["incoming"])), "等待真实incoming")
    audit.equal(step["contains"], True, "冻结等待例包含事实")
    if "candidate_value_only" in expected:
        audit.equal([step["acc_after"]["low"], step["acc_after"]["high"]], list(map(str, expected["candidate_value_only"])), "等待仅保留手算数值候选")
        audit.equal(step["high_sources"], list(map(str, expected["candidate_high_raw_indices"])), "未选唯一高根")
        audit.equal(step["low_sources"], list(map(str, expected["candidate_low_raw_indices"])), "未选唯一低根")
        inverse = {"GT": "LT", "LT": "GT", "EQ": "EQ"}
        audit.equal([item["order"] for item in step["comparisons"]], [inverse[item] for item in expected["incoming_vs_accumulator"]], "等待两轴比较")
    else:
        audit.equal(step["acc_after"], None, "未建方向不得折叠")
        audit.equal(step["high_sources"], [], "未执行不得冒出高根")
        audit.equal(step["low_sources"], [], "未执行不得冒出低根")
    audit.equal([obj for obj in snapshot["objects"] if obj["kind"] == KINDS[2]], [], "等待例不制造分型")
    knowledge = [obj["payload"] for obj in snapshot["objects"] if obj["kind"] == KINDS[4]]
    audit.equal(len(knowledge), 1, "等待知识状态唯一")
    audit.require(reason in knowledge[0]["waiting_reasons"], "知识状态必须保留缺口")
    requests = [row for row in knowledge[0]["requests"] if reason in row["reasons"]]
    audit.require(bool(requests), "等待必须到具体请求")
    for row in requests:
        audit.equal(row["axes"]["applicability"], "DOMAIN_PROOF_MISSING", "等待适用域轴")
        audit.equal(row["axes"]["computation"], "INCOMPLETE", "等待计算轴")
    audit.require(any(row["reason"] == reason for row in snapshot["observations"]), "等待须有同源观察")


def check_revision(public: dict, count: int, audit: Audit) -> None:
    before = public["known-" + str(count - 2)]
    after = public["known-" + str(count - 1)]
    old_group = [obj for obj in before["objects"] if obj["kind"] == KINDS[1] and obj["payload"]["group_anchor"] == "1"]
    new_group = [obj for obj in after["objects"] if obj["kind"] == KINDS[1] and obj["payload"]["group_anchor"] == "1"]
    audit.equal(len(old_group), 1, "修订前具名组")
    audit.equal(len(new_group), 1, "修订后具名组")
    old_id, new_id = old_group[0]["object_id"], new_group[0]["object_id"]
    audit.require(old_id != new_id, "修订内容变更必须有新对象身份")
    withdrawn = {obj["object_id"]: obj for obj in after["withdrawn_objects"]}
    audit.require(old_id in withdrawn, "修订必须保留旧组撤回")
    audit.equal(withdrawn[old_id]["superseded_by"], new_id, "同槽组新旧真实身份绑定")
    audit.equal(withdrawn[old_id]["withdrawn_generation"], str(count), "修订撤回代际")
    relations = {(row["subject"], row["relation_type"], row["object"]) for row in after["relations"]}
    audit.require((new_id, "replaces", old_id) in relations, "修订真实替代边")
    old_shapes = {obj["object_id"] for obj in before["objects"] if obj["kind"] == KINDS[2]}
    audit.require(old_shapes <= set(withdrawn), "旧窗口槽消失须完整撤回")


def source_evidence(evidence: Path, plan: dict, run: dict, audit: Audit) -> dict:
    manifest = load(evidence / "source-before-run/MANIFEST.json")
    indexed = index_unique(manifest, ("source",), audit, "原件manifest")
    hashes = {}
    for entry in manifest:
        path = evidence / "source-before-run" / entry["snapshot"]
        audit.require(path.resolve().parent == (evidence / "source-before-run").resolve(), "快照路径越界")
        raw = path.read_bytes()
        audit.equal(len(raw), entry["bytes"], "原件长度/" + entry["snapshot"])
        audit.equal(digest(raw), entry["sha256"], "原件摘要/" + entry["snapshot"])
        hashes[entry["source"]] = entry["sha256"]
    fixtures = Path(__file__).resolve().parent / "fixtures/tb02a"
    for filename, field in (("raw-ledger.json", "source_ledger_sha256"), ("hand-oracle.json", "hand_oracle_sha256")):
        audit.equal(hashes[str(fixtures / filename)], plan[field], "运行冻结原件/" + filename)
    messages_path = str(Path(run["directory"]) / "messages.jsonl")
    entry = indexed[(messages_path,)]
    raw = (evidence / "source-before-run" / entry["snapshot"]).read_bytes()
    audit.equal(raw, Path(messages_path).read_bytes(), "实际冻结消息与运行计划一致")
    return hashes


def input_evidence(run: dict, ledger: dict, records: list, tables: dict, audit: Audit) -> None:
    """逐条绑定冻结原始 OHLC；包装坐标是明示测试值，不生成结构预期。"""
    cases = {row["case_id"]: row["raw_bars"] for row in ledger["cases"]}
    name = run["case_id"]
    bars = cases[name] if name != "revision_recovery" else cases["high_only_before_revision"] + [cases["high_only_after_revision"][2]]
    audit.equal(run["input_count"], len(bars), "冻结原始账簿输入数量")
    _, messages = read_core(Path(run["directory"]) / "messages.jsonl")
    audit.equal(len(messages), len(bars), "实际包装消息数量")
    ingest = index_unique([row for row in records if row["kind"] == "actual_ingest_result"], ("operation",), audit, "实际输送")
    receipts = index_unique([row for row in records if row["kind"] == "final_receipt"], ("operation",), audit, "最终原身份收据")
    rows = sorted(tables["raw_events"], key=lambda row: row["seq"])
    audit.equal(len(rows), len(bars), "持久原始账逐条数量")
    ohlc = index_unique(tables["raw_ohlc"], ("identity_key", "revision"), audit, "持久OHLC")
    audit.equal(len(ohlc), len(bars), "持久OHLC一一对应")
    for index, (message, original, stored) in enumerate(zip(messages, bars, rows)):
        event = message["payload"]["raw_input"]["events"]
        audit.equal(len(event), 1, "一消息一原始事实")
        value = event[0]
        audit.equal(set(value), {"event_id", "revision", "seq", "received_at", "raw_text",
                                 "open", "high", "low", "close", "timestamp", "volume"}, "完整OHLC wire十一字段")
        audit.equal(parse(value["raw_text"]), original, "原始账簿全部字段")
        audit.equal({key: value[key] for key in PRICE_FIELDS}, {key: str(original[key]) for key in PRICE_FIELDS}, "原始OHLC四价")
        audit.equal(value["event_id"], original["raw_id"], "原始业务身份")
        audit.equal(value["seq"], str(original["raw_index"]), "源坐标不冒充接纳序")
        audit.equal(value["timestamp"], str(original["raw_index"] * 60), "明示测试时间坐标")
        audit.equal(value["volume"], "1", "明示测试volume")
        received = datetime.datetime(2000, 1, 1, tzinfo=datetime.timezone.utc) + datetime.timedelta(seconds=index * 10)
        audit.equal(value["received_at"], received.isoformat().replace("+00:00", "Z"), "明示测试接收时钟")
        audit.equal(value["revision"], "2" if name == "revision_recovery" and index == len(bars) - 1 else "1", "具名修订")
        audit.equal(message["message_id"], "tb02a-op-" + str(index).zfill(4), "唯一输入消息ID")
        audit.equal(message["payload_hash"], digest(canonical(message["payload"])), "完整消息内容摘要")
        audit.equal(stored["seq"], index, "持久接纳序")
        audit.equal(stored["source_coord"], value["seq"], "持久原始源坐标")
        audit.equal(stored["event_id"], value["event_id"], "持久原始身份")
        audit.equal(stored["payload_hash"], digest(canonical(value)), "持久完整原始事件摘要")
        prices = ohlc[(stored["identity_key"], stored["revision"])]
        audit.equal({key: prices[key] for key in PRICE_FIELDS}, {key: value[key] for key in PRICE_FIELDS}, "持久四价")
        audit.equal(stored["price"], value["close"], "兼容close索引不改OHLC")
        fault = index == run["fault_index"]
        audit.equal(ingest[(str(index),)]["result"]["transport"], "DeliveryUnknown" if fault else "Received", "真实故障输送状态")
        final = receipts[(str(index),)]
        payload = final["request"]["payload"]
        audit.equal(payload, {"op": "receipt", **{"original_" + key: message[key] for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")}}, "收据原消息身份")
        response = final["response"]
        audit.equal(response["transport"], "Received", "最终收据完整接收")
        body = response["response"]["payload"]
        audit.equal(body["kind"], "Committed", "原身份最终已提交")
        audit.equal(body["accepted_seq"], str(index), "原身份最终接纳序")
        audit.equal(response["response"]["payload_hash"], digest(canonical(body)), "最终收据正文摘要")


def check_reference_bindings(snapshot: dict, audit: Audit) -> None:
    """所有 typed 引用绑定当前 cut 的真实 raw 版本，不只比较价格投影。"""
    raw = index_unique(snapshot["raw_history"], ("identity_key", "revision"), audit, "公开原始版本")
    latest = {}
    for row in snapshot["raw_history"]:
        coord = row["source_coord"]
        if coord not in latest or int(row["revision"]) > int(latest[coord]["revision"]):
            latest[coord] = row
    fields = ("identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord")
    for obj in snapshot["objects"] + snapshot["withdrawn_objects"]:
        if "fact_key" not in obj:
            continue
        coords = []
        for reference in obj["input_refs"]:
            source = raw[(reference["identity_key"], reference["revision"])]
            audit.equal(reference, {key: source[key] for key in fields}, "typed完整原始引用")
            if obj["lifecycle"] == "active":
                audit.equal(reference, {key: latest[reference["source_coord"]][key] for key in fields}, "活动事实绑定当cut有效源版本")
            coords.append(reference["source_coord"])
        audit.equal(obj["source_coords"], coords, "typed引用/源坐标完整对应")
        content = {key: obj[key] for key in ("kind", "fact_key", "payload", "input_refs")}
        content.update(profile_id="ohlc_integer_tb02a_v1", rule_revision=snapshot["catalog_evidence"]["rule_revision"])
        audit.equal(obj["object_id"], "obj-" + digest(canonical(content)), "typed内容寻址完整身份")
        if obj["kind"] == KINDS[1]:
            payload = obj["payload"]
            confirmation = payload["confirmation_evidence"]
            audit.equal(payload["confirmed"], confirmation is not None, "确认位/后继见证")
            audit.equal(coords, sorted(group_computation_sources(payload), key=int), "组事实完整绑定成员/方向/边界确认依赖")
    check_shape_dependencies(snapshot, audit)


def checked_refs(refs: list, raw: dict, audit: Audit, label: str, latest: dict | None = None) -> list:
    """成员和组外依赖各自有序、唯一且逐字段绑定当 cut 的已封存版本。"""
    fields = ("identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord")
    coords = []
    for reference in refs:
        row = raw[(reference["identity_key"], reference["revision"])]
        audit.equal(reference, {key: row[key] for key in fields}, label + "/真实原始版本")
        if latest is not None:
            current = latest[reference["source_coord"]]
            audit.equal(reference, {key: current[key] for key in fields}, label + "/活动事实绑定当cut有效源版本")
        coords.append(reference["source_coord"])
    audit.equal(coords, [str(value) for value in sorted({int(coord) for coord in coords})], label + "/坐标规范有序唯一")
    return coords


def group_computation_sources(group: dict) -> set:
    """§9：复核已声明的构造/边界来源，不重新判包含、方向或确认。"""
    sources = set(group["members"])
    for key in ("direction_evidence", "confirmation_evidence"):
        if group[key] is not None:
            sources.update(group[key]["source_coords"])
    return sources


def check_shape_dependencies(snapshot: dict, audit: Audit) -> None:
    """§9：三K真实成员与组外方向/边界确认依赖分别绑定当前原始版本。"""
    raw = index_unique(snapshot["raw_history"], ("identity_key", "revision"), audit, "形态原始版本")
    latest = {}
    for row in raw.values():
        coord = row["source_coord"]
        if coord not in latest or int(row["revision"]) > int(latest[coord]["revision"]):
            latest[coord] = row
    groups = {obj["payload"]["group_anchor"]: obj["payload"] for obj in snapshot["objects"] if obj["kind"] == KINDS[1]}
    witnesses = index_unique(snapshot["witnesses"], ("object_id", "slot"), audit, "形态见证槽")
    shapes = {}
    for shape in snapshot["objects"] + snapshot["withdrawn_objects"]:
        if shape["kind"] != KINDS[2]:
            continue
        audit.equal(len(shape["input_refs"]), 3, "三K依赖保持三个真实组")
        whole = set()
        anchors = [shape[key] for key in ("window_start", "window_mid", "window_end")]
        for slot, (anchor, bundle) in enumerate(zip(anchors, shape["input_refs"])):
            audit.equal(set(bundle), {"merged_index", "raw_refs", "dependency_refs"}, "OHLC形态成员/依赖两类引用")
            current = latest if shape["lifecycle"] == "active" else None
            members = checked_refs(bundle["raw_refs"], raw, audit, "形态实际成员", current)
            dependencies = checked_refs(bundle["dependency_refs"], raw, audit, "形态组外计算依赖", current)
            audit.require(not set(members) & set(dependencies), "依赖不得伪装成组成员")
            witness = witnesses[(shape["object_id"], str(slot))]
            audit.equal(witness["raw_bars"], [raw[(ref["identity_key"], ref["revision"])] for ref in bundle["raw_refs"]], "形态见证仅保真实成员完整原始行")
            audit.equal(witness["merged_source_index"], anchor, "形态见证保持真实组锚")
            if shape["lifecycle"] == "active":
                group = groups[anchor]
                audit.equal(bundle["merged_index"], group["group_index"], "形态引用的真实组序号")
                audit.equal(members, group["members"], "形态raw_refs仅含真实组成员")
                needed = group_computation_sources(group) - set(members)
                audit.equal(dependencies, sorted(needed, key=int), "形态组外依赖精确保全方向与边界确认来源")
            whole.update(members)
            whole.update(dependencies)
        audit.equal(shape["source_coords"], sorted(whole, key=int), "三K完整计算来源并集")
        if shape["lifecycle"] == "active":
            shapes[shape["object_id"]] = shape
    check_downstream_dependencies(snapshot, shapes, groups, audit)


def check_downstream_dependencies(snapshot: dict, shapes: dict, groups: dict, audit: Audit) -> None:
    """只检查来源字段间的传递，不把 S 自报的市场结论作为独立 oracle。"""
    descriptions = {obj["object_id"]: obj for obj in snapshot["objects"] if obj["kind"] == KINDS[3]}
    for description in descriptions.values():
        payload = description["payload"]
        shape = shapes[payload["shape_object_id"]]
        following = {row["bar"]["source_coord"] for row in payload["subsequent_development"]["bars"]}
        audit.equal(description["source_coords"], sorted(set(shape["source_coords"]) | following, key=int), "CC007形态计算依赖与实际后续来源完整传递")
    for obj in snapshot["objects"]:
        if obj["kind"] != KINDS[4]:
            continue
        for request in obj["payload"]["requests"]:
            slot = parse(request["request_id"])
            if slot[0] == KINDS[2] and isinstance(slot[1], list):
                if request["subject_id"] is not None:
                    audit.equal(request["source_coords"], shapes[request["subject_id"]]["source_coords"], "CC054三K请求完整传递形态来源")
                else:
                    expected = set()
                    for anchor in slot[1]:
                        expected.update(group_computation_sources(groups[anchor]))
                    audit.equal(request["source_coords"], sorted(expected, key=int), "CC054三K等待请求完整传递构造与边界来源")
            elif isinstance(slot[0], list) and slot[0][0] == KINDS[3]:
                description = descriptions[request["subject_id"]]
                shape = shapes[description["payload"]["shape_object_id"]]
                if slot[1] == "subsequent_development":
                    expected = description["source_coords"]
                elif slot[1] == "descriptive_labels":
                    expected = sorted({row["source_coord"] for row in description["payload"]["descriptive_labels"]["raw_ohlc"]}, key=int)
                else:
                    audit.equal(slot[1], "shape_description", "CC007请求具名轴")
                    expected = shape["source_coords"]
                audit.equal(request["source_coords"], expected, "CC054描述请求按本请求职责保全计算来源")


def check_relation_witnesses(snapshot: dict, oracle: dict, name: str, audit: Audit) -> None:
    """核对 S 实际累加器比较和手算相触端点；不代算未输出的 raw-pair 分类。"""
    steps = {obj["payload"]["incoming"]["source_coord"]: obj["payload"] for obj in snapshot["objects"] if obj["kind"] == KINDS[0]}
    inverse = {"GT": "LT", "LT": "GT", "EQ": "EQ"}
    for row in oracle["relation_witnesses"]:
        if row["case_id"] != name:
            continue
        if row["scope"] == "raw_adjacent_members_inside_one_merged_group":
            actual = steps[str(row["b_raw"])]
            audit.equal([value["order"] for value in actual["comparisons"]], [inverse[value] for value in row["actual_fold_at_b"]], "实际折叠比较不冒充原始成员比较")
        else:
            # 两个具名相触工作例均在 raw4 产生；端点等价类直接取手算已列等式。
            actual = steps["4"]
            classes = [["acc.low"], ["acc.high", "incoming.low"], ["incoming.high"]] if row["id"] == "UP_CROSS_GROUP_TOUCH" else [["incoming.low"], ["acc.low", "incoming.high"], ["acc.high"]]
            audit.equal(actual["endpoint_order"], classes, "S真实相触端点等价类/" + row["id"])
            audit.equal(actual["contains"], False, "具名相触不能误报包含")


def verify_run(run: dict, plan: dict, oracle: dict, ledger: dict, audit: Audit) -> tuple[bytes, dict]:
    checks_before, leaves_before = audit.checks, audit.leaf_values
    directory = Path(run["directory"])
    evidence = directory / "evidence"
    result = load(evidence / "RESULT.json")
    audit.equal(result["status"], CAPTURED, "实际采集必须成功")
    audit.equal(result["cleanup"]["errors"], [], "实际清理不得失败")
    cleanup = index_unique(result["cleanup"]["outcomes"], ("action",), audit, "清理动作")
    for service in ("q", "s"):
        receipt = cleanup[("stop-" + service,)]["result"]
        audit.equal(receipt["service"], service, "停止具名服务")
        audit.equal(receipt["state"], "stopped", "实际服务已停止")
    audit.equal(cleanup[("helper-wait",)]["result"], 0, "采集helper正常退出")
    audit.require(("close-event-log",) in cleanup, "事件日志完成关闭")
    count = run["input_count"]
    audit.equal(result["input_count"], count, "计划输入数")
    hashes = source_evidence(evidence, plan, run, audit)
    raw, records = read_core(evidence / "semantic-core.jsonl")
    expected_count = 5 * count + len(TABLES) + 2
    audit.equal(len(records), expected_count, "事前推导的完整记录数")
    audit.equal(result["core_records"], expected_count, "采集记录数回执")
    kinds = Counter(row["kind"] for row in records)
    audit.equal(dict(kinds), {"actual_ingest_result": count, "final_receipt": count,
                             "public_candidate": 3 * count, "authoritative_table": len(TABLES),
                             "authoritative_schema": 1, "lifecycle_semantics": 1}, "轨迹完整种类计数")
    tables = table_evidence(evidence, records, audit)
    input_evidence(run, ledger, records, tables, audit)
    public = {}
    for record in records:
        if record["kind"] != "public_candidate":
            continue
        command = record["command"]
        audit.require(command["id"] not in public, "公开采集命令重复")
        prefix, number = command["id"].rsplit("-", 1)
        generation = int(number) + 1
        audit.require(prefix in ("live", "known", "revisit") and 1 <= generation <= count, "公开采集命令不属于计划")
        mode = "RecomputedWithRevision" if prefix == "live" else "AsKnown"
        public[command["id"]] = candidate_evidence(record["candidate"], tables, generation, mode, audit)
        check_reference_bindings(public[command["id"]], audit)
    audit.equal(set(public), {prefix + "-" + str(index) for prefix in ("live", "known", "revisit") for index in range(count)}, "逐代三路采集全覆盖")
    for index in range(count):
        audit.equal(public["known-" + str(index)], public["revisit-" + str(index)], "旧cut重读全字段")
    lifecycle = next(row["value"] for row in records if row["kind"] == "lifecycle_semantics")
    if run["fault_index"] is None:
        audit.equal(lifecycle, {}, "无故障轨迹不能虚报恢复")
    else:
        index = run["fault_index"]
        audit.equal(lifecycle, {"stage": "after_batch", "signal": "SIGKILL",
            "before": {"generation": str(index), "accepted": str(index + 1), "committed": str(index)},
            "after": {"generation": str(index + 1), "accepted": str(index + 1), "committed": str(index + 1)},
            "writer_epoch": "2", "q_survived": True}, "具名恢复持久边界")
    check_case(run, oracle, ledger, public, audit)
    return raw, {"case_id": run["case_id"], "arm": run["arm"], "input_count": count,
                 "core_records": len(records), "core_bytes": len(raw), "core_sha256": digest(raw),
                 "checks_executed": audit.checks - checks_before,
                 "compared_leaf_values": audit.leaf_values - leaves_before,
                 "source_hashes": hashes, "status": "PASS_BOUNDED_ORACLE_AND_FULL_CAPTURE"}


def check_case(run: dict, oracle: dict, ledger: dict, public: dict, audit: Audit) -> None:
    count = run["input_count"]
    snapshot = public["known-" + str(count - 1)]
    success = {row["case_id"]: row for row in oracle["success_cases"]}
    waiting = {row["case_id"]: row for row in oracle["waiting_cases"]}
    bars = {row["case_id"]: row["raw_bars"] for row in ledger["cases"]}
    name = run["expected_case"]
    if name in waiting:
        check_waiting(snapshot, waiting[name], audit)
        return
    expected = success[name]
    check_groups(snapshot, expected, audit, bars[name])
    prefixes = None if run["case_id"] == "revision_recovery" else {index + 1: public["known-" + str(index)] for index in range(count)}
    check_steps(snapshot, expected, bars[name], prefixes, audit)
    shapes = check_windows(snapshot, expected, audit)
    check_descriptions(snapshot, expected, shapes, audit)
    check_relation_witnesses(snapshot, oracle, name, audit)
    if run["case_id"] == "revision_recovery":
        before = public["known-" + str(count - 2)]
        check_groups(before, success["high_only_before_revision"], audit, bars["high_only_before_revision"])
        check_windows(before, success["high_only_before_revision"], audit)
        check_revision(public, count, audit)


def verify_plan(path: Path, audit: Audit) -> dict:
    plan = load(path)
    fixtures = Path(__file__).resolve().parent / "fixtures/tb02a"
    ledger_raw, oracle_raw = (fixtures / "raw-ledger.json").read_bytes(), (fixtures / "hand-oracle.json").read_bytes()
    ledger, oracle = parse(ledger_raw), parse(oracle_raw)
    audit.equal(plan["source_ledger_sha256"], digest(ledger_raw), "计划冻结账簿摘要")
    audit.equal(plan["hand_oracle_sha256"], digest(oracle_raw), "计划冻结手算摘要")
    project = Path(__file__).resolve().parents[2]
    for source in oracle["sources"]:
        audit.equal(digest((project / source["path"]).read_bytes()), source["sha256"], "冻结手算定义正本/" + source["path"])
    expected_cases = {row["case_id"] for row in ledger["cases"]} | {"revision_recovery"}
    runs = index_unique(plan["runs"], ("case_id", "arm"), audit, "20次运行计划")
    audit.equal(set(runs), {(name, arm) for name in expected_cases for arm in ("a", "b")}, "20次运行全集")
    audit.equal(len(runs), 20, "冻结工作例运行数量")
    directories, sockets, ports = set(), set(), set()
    for (name, _arm), run in runs.items():
        audit.equal(run["expected_case"], "high_only_after_revision" if name == "revision_recovery" else name, "运行不能改换较易预期")
        config = load(Path(run["directory"]) / "config.json")
        audit.equal(config["session_id"], "tb02a-" + name, "配置/具名case绑定")
        audit.equal(run["fault_index"], 5 if name == "revision_recovery" else None, "冻结故障点")
        audit.require(run["directory"] not in directories and config["socket"] not in sockets and config["port"] not in ports, "20次运行物理隔离")
        directories.add(run["directory"])
        sockets.add(config["socket"])
        ports.add(config["port"])
    for name in expected_cases:
        configs = [load(Path(runs[(name, arm)]["directory"]) / "config.json") for arm in ("a", "b")]
        audit.equal(set(configs[0]), set(configs[1]), "双跑配置字段全集")
        for key in configs[0]:
            if key == "clock_plan":
                audit.equal(Path(configs[0][key]).read_bytes(), Path(configs[1][key]).read_bytes(), "双跑完整语义时钟原件")
            elif key not in ("db", "socket", "port"):
                audit.equal(configs[0][key], configs[1][key], "双跑同一配置/" + key)
    report = {"schema": "tb02a-independent-oracle-check/1", "status": "NOT_VERIFIED", "runs": [], "pairs": [],
              "excluded_fields": [], "identity_renaming": [], "semantic_oracle": "frozen_hand_oracle",
              "browser": "not_evaluated", "general_domain": "not_proved", "source_ledger_sha256": digest(ledger_raw),
              "hand_oracle_sha256": digest(oracle_raw),
              "not_independently_oracled": ["手算未列的建立方向见证快照全价；组确认后继已按账簿核对",
                  "CC007后续每条价格关系与全部描述文本的独立手算expected",
                  "两向包含位未单独暴露的原始关系见证；不以合取/析取位替代两位",
                  "成员等号与跨组相触的原始价已完整保全；未独立重写包含判断"]}
    for name in sorted(expected_cases):
        cores, summaries = [], []
        for arm in ("a", "b"):
            raw, summary = verify_run(runs[(name, arm)], plan, oracle, ledger, audit)
            cores.append(raw)
            summaries.append(summary)
            report["runs"].append(summary)
        audit.equal(cores[0], cores[1], "双跑完整语义原字节/" + name)
        # 源路径包括两次独立运行配置；实际消费者/构建文件的共同路径必须同字节。
        a, b = summaries[0]["source_hashes"], summaries[1]["source_hashes"]
        for source in a.keys() & b.keys():
            audit.equal(a[source], b[source], "双跑同源源码/" + source)
        report["pairs"].append({"case_id": name, "status": "PASS_EXACT_BYTES", "sha256": digest(cores[0]),
                                "bytes": len(cores[0]), "excluded_fields": []})
    report.update(status="PASS_BOUNDED_ORACLE_AND_EXACT_DOUBLE_RUN", checks_executed=audit.checks,
                  compared_leaf_values=audit.leaf_values, checked_runs=len(report["runs"]), checked_pairs=len(report["pairs"]))
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-plan", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    audit = Audit()
    try:
        result = verify_plan(args.run_plan.resolve(), audit)
    except (OSError, ValueError, KeyError, TypeError, IndexError, sqlite3.Error, RecursionError) as exc:
        result = {"schema": "tb02a-independent-oracle-check/1", "status": "FAIL_OR_NOT_VERIFIED",
                  "error": type(exc).__name__ + ": " + str(exc), "checks_executed": audit.checks,
                  "compared_leaf_values": audit.leaf_values, "browser": "not_evaluated", "general_domain": "not_proved"}
    result["checker_sha256"] = digest(Path(__file__).read_bytes())
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2, allow_nan=False)
        stream.write("\n")
    print(json.dumps({key: result[key] for key in ("status", "checks_executed", "compared_leaf_values")}, ensure_ascii=False))
    return 0 if result["status"] == "PASS_BOUNDED_ORACLE_AND_EXACT_DOUBLE_RUN" else 1


if __name__ == "__main__":
    raise SystemExit(main())
