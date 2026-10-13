"""#1373：S 已封存 TB-02-A 事实的类型、身份和来源校验；不执行结构判断。"""
import hashlib
import json
import re

PROFILE = "ohlc_integer_tb02a_v1"
RAW_SCHEMA = "s-ohlc/1"
KINDS = ("CC-004.inclusion_step", "CC-005.inclusion_group", "CC-007.fractal_description", "CC-054.knowledge_state")
AXES = ("CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-054", "CC-056")
LEGACY_AXES = AXES
BI_KINDS = ("CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation", "CC-055.change_event", "CC-056.version")
KINDS += BI_KINDS
AXES += ("CC-008", "CC-009", "CC-010", "CC-055")
BI_AXES = AXES
SEGMENT_KINDS = ("CC-011.segment_seed", "CC-012.feature_sequence", "CC-013.segment", "CC-055.segment_relation", "CC-055.segment_change", "CC-056.segment_version", "CC-011.seed_candidate")
KINDS += SEGMENT_KINDS
AXES += ("CC-011", "CC-012", "CC-013")
FIRST_SEGMENT_AXES = AXES
AXES += ("CC-014",)
SEGMENT_SECOND_AXES = AXES
SEGMENT_SECOND_RULE = "s2-axis-quantifiers+new-bi/1+segment-second/1"
# #1419 TB-02-E：新规则修订只追加中枢族与三轴；旧规则 cut 仍按旧轴集严格读取。
CENTER_RULE = SEGMENT_SECOND_RULE + "+center-seed/1"
CENTER_KINDS = ("CC-016.center_seed_attempt", "CC-016.center", "CC-017.center_roles", "CC-022.unit_assignment", "CC-055.center_relation", "CC-055.center_change", "CC-056.center_version")
KINDS += CENTER_KINDS
AXES += ("CC-016", "CC-017", "CC-022")
CENTER_AXES = AXES

FACT_COLUMNS = ("object_id", "object_revision", "kind", "batch_id", "fact_key", "payload_json", "input_refs_json", "source_coords_json", "first_known_generation", "first_known_cut", "published_generation", "withdrawn_generation", "withdrawal_reason", "superseded_by")
FACT_FIELDS = ("object_id", "object_revision", "kind", "batch_id", "fact_key", "payload", "input_refs", "source_coords", "first_known_generation", "first_known_cut", "published_generation", "withdrawn_generation", "withdrawal_reason", "superseded_by", "lifecycle")
REF_FIELDS = ("identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord")
SCHEMA = """
CREATE TABLE raw_ohlc (
 identity_key TEXT NOT NULL, revision INTEGER NOT NULL, schema_revision TEXT NOT NULL,
 open TEXT NOT NULL, high TEXT NOT NULL, low TEXT NOT NULL, close TEXT NOT NULL,
 PRIMARY KEY(identity_key,revision));
CREATE TABLE structure_facts (
 object_id TEXT PRIMARY KEY, object_revision INTEGER NOT NULL, kind TEXT NOT NULL,
 batch_id TEXT NOT NULL, fact_key TEXT NOT NULL, payload_json TEXT NOT NULL,
 input_refs_json TEXT NOT NULL, source_coords_json TEXT NOT NULL,
 first_known_generation INTEGER NOT NULL, first_known_cut TEXT NOT NULL,
 published_generation INTEGER NOT NULL, withdrawn_generation INTEGER,
 withdrawal_reason TEXT, superseded_by TEXT);
"""


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def keys(value, expected, name):
    if type(value) is not dict or set(value) != set(expected):
        raise ValueError(name + " 字段缺失/多余")


def text(value, name):
    if type(value) is not str or not value:
        raise ValueError(name + " 必须是非空文本")
    value.encode("utf-8")
    return value


def integer(value, name, minimum=None):
    if type(value) is not str or not re.fullmatch(r"(?:0|[1-9][0-9]*|-[1-9][0-9]*)", value) or len(value) > 20:
        raise ValueError(name + " 必须是规范 i64 文本")
    number = int(value)
    if not -(2**63) <= number < 2**63 or (minimum is not None and number < minimum):
        raise ValueError(name + " 超出整数域")
    return number


def array(value, name):
    if type(value) is not list:
        raise ValueError(name + " 必须是数组")
    return value


def strings(value, name):
    for item in array(value, name):
        text(item, name)
    return value


def coords(value, name):
    values = [integer(item, name, 0) for item in array(value, name)]
    if values != sorted(set(values)):
        raise ValueError(name + " 必须按真实源坐标严格递增")
    return value


def wire_tree(value):
    if type(value) is dict:
        for key, item in value.items():
            text(key, "payload.key")
            wire_tree(item)
    elif type(value) is list:
        for item in value:
            wire_tree(item)
    elif type(value) is str:
        value.encode("utf-8")
    elif value is not None and type(value) is not bool:
        raise ValueError("TB02 wire 不允许 JSON number 或未知类型")


def bar(value, name):
    keys(value, ("source_coord", "open", "high", "low", "close"), name)
    integer(value["source_coord"], name + ".source_coord", 0)
    # acc/group 是同核合并段，open/close 沿原锚语义；不能对其重套 raw 几何门。
    for key in ("open", "high", "low", "close"):
        integer(value[key], name + "." + key)


def direction(value):
    if value is not None and value not in ("UP", "DOWN"):
        raise ValueError("direction 不属于已声明值")


def direction_evidence(value):
    if value is None:
        return
    keys(value, ("direction", "established_at", "previous_acc", "incoming", "source_coords"), "direction_evidence")
    if value["direction"] not in ("UP", "DOWN"):
        raise ValueError("方向见证方向缺失")
    integer(value["established_at"], "established_at", 0)
    bar(value["previous_acc"], "previous_acc")
    bar(value["incoming"], "direction.incoming")
    coords(value["source_coords"], "direction.source_coords")
    if value["established_at"] != value["incoming"]["source_coord"]:
        raise ValueError("方向见证建立坐标与原 incoming 不同")


def validate_payload(kind, p):
    wire_tree(p)
    _validate_payload_fields(kind, p)


def _validate_payload_fields(kind, p):
    """内部字段门；调用方须在同次调用先完成此原树的 wire_tree。"""
    if kind in SEGMENT_KINDS:
        validate_segment_payload(kind, p)
    elif kind in CENTER_KINDS:
        validate_center_payload(kind, p)
    elif kind in BI_KINDS:
        validate_bi_payload(kind, p)
    elif kind == KINDS[0]:
        keys(p, ("action", "incoming", "acc_before", "acc_after", "direction", "direction_evidence", "comparisons", "endpoint_order", "contains", "high_sources", "low_sources", "waiting_reasons"), kind)
        if p["action"] not in ("seed", "establish_direction", "merge", "new_group", "waiting"):
            raise ValueError("包含步骤 action 未声明")
        bar(p["incoming"], "incoming")
        for name in ("acc_before", "acc_after"):
            if p[name] is not None:
                bar(p[name], name)
        direction(p["direction"])
        direction_evidence(p["direction_evidence"])
        comparisons = array(p["comparisons"], "comparisons")
        if len(comparisons) not in (0, 2):
            raise ValueError("comparisons 必须是 high/low 两轴或 seed 空集")
        for index, item in enumerate(comparisons):
            keys(item, ("axis", "acc", "incoming", "order"), "comparison")
            if item["axis"] != ("high", "low")[index] or item["order"] not in ("LT", "EQ", "GT"):
                raise ValueError("comparison 轴/枚举不符")
            integer(item["acc"], "comparison.acc")
            integer(item["incoming"], "comparison.incoming")
        flattened = []
        for group in array(p["endpoint_order"], "endpoint_order"):
            if not strings(group, "endpoint class"):
                raise ValueError("端点等价类为空")
            flattened.extend(group)
        if flattened and (len(flattened) != 4 or set(flattened) != {"acc.low", "acc.high", "incoming.low", "incoming.high"}):
            raise ValueError("端点标签缺失或重复")
        if p["contains"] is not None and type(p["contains"]) is not bool:
            raise ValueError("contains 必须是 bool/null")
        if p["acc_before"] is None and (comparisons or flattened or p["contains"] is not None):
            raise ValueError("无 acc 的 seed 冒出比较结果")
        for key in ("high_sources", "low_sources"):
            coords(p[key], key)
        strings(p["waiting_reasons"], "waiting_reasons")
    elif kind == KINDS[1]:
        keys(p, ("group_index", "group_anchor", "members", "open", "high", "low", "close", "high_sources", "low_sources", "confirmed", "direction", "direction_evidence", "confirmation_evidence", "waiting_reasons"), kind)
        integer(p["group_index"], "group_index", 0)
        integer(p["group_anchor"], "group_anchor", 0)
        bar({"source_coord": p["group_anchor"], **{k: p[k] for k in ("open", "high", "low", "close")}}, "group")
        members = coords(p["members"], "members")
        if not members or p["group_anchor"] != members[0]:
            raise ValueError("组首必须绑定该组首个真实成员")
        for key in ("high_sources", "low_sources"):
            if not coords(p[key], key) or not set(p[key]) <= set(members):
                raise ValueError("组极值来源不属于完整成员")
        if type(p["confirmed"]) is not bool:
            raise ValueError("confirmed 必须 bool")
        direction_evidence(p["confirmation_evidence"])
        if p["confirmed"] != (p["confirmation_evidence"] is not None):
            raise ValueError("组确认位与实际关闭见证不一致")
        direction(p["direction"])
        direction_evidence(p["direction_evidence"])
        strings(p["waiting_reasons"], "waiting_reasons")
    elif kind == KINDS[2]:
        keys(p, ("shape_object_id", "branch", "window", "description", "reference_prices", "descriptive_labels", "subsequent_development"), kind)
        text(p["shape_object_id"], "shape_object_id")
        if p["branch"] not in ("TOP", "BOTTOM") or len(coords(p["window"], "window")) != 3:
            raise ValueError("CC007 只描述已经成立的顶/底三组")
        text(p["description"], "description")
        refs = array(p["reference_prices"], "reference_prices")
        names = [f"{slot}.{axis}" for slot in ("left", "mid", "right") for axis in ("high", "low")]
        if len(refs) != 6:
            raise ValueError("CC007 六参考价不完整")
        for index, ref in enumerate(refs):
            keys(ref, ("name", "source_coord", "axis", "value"), "reference_price")
            if ref["name"] != names[index] or ref["axis"] != names[index].split(".")[1] or ref["source_coord"] != p["window"][index // 2]:
                raise ValueError("CC007 参考价身份/组锚不一致")
            integer(ref["value"], "reference.value")
        labels = p["descriptive_labels"]
        keys(labels, ("status", "reason", "labels", "source", "raw_ohlc"), "descriptive_labels")
        if (labels["status"], labels["reason"], labels["labels"], labels["source"]) != ("not_determined", "no_settled_numeric_rule", [], "fenxing.md:64-74"):
            raise ValueError("CC007 形容词未定边界被改写")
        for item in array(labels["raw_ohlc"], "raw_ohlc"):
            bar(item, "raw_ohlc")
        subsequent = p["subsequent_development"]
        keys(subsequent, ("status", "reason", "bars"), "subsequent_development")
        bars = array(subsequent["bars"], "subsequent.bars")
        if (subsequent["status"], subsequent["reason"]) != (("observed", None) if bars else ("insufficient_knowledge", "no_subsequent_bar")):
            raise ValueError("后续事实状态与是否有事实不一致")
        for item in bars:
            keys(item, ("bar", "relations"), "subsequent.bar")
            bar(item["bar"], "subsequent.bar")
            relations = array(item["relations"], "subsequent.relations")
            expected = [(axis, ref) for axis in ("open", "high", "low", "close") for ref in refs]
            if len(relations) != len(expected):
                raise ValueError("后续四价×六参考价关系不完整")
            for rel, (axis, ref) in zip(relations, expected):
                keys(rel, ("axis", "value", "reference", "reference_value", "order"), "subsequent.relation")
                if (rel["axis"], rel["value"], rel["reference"], rel["reference_value"]) != (axis, item["bar"][axis], ref["name"], ref["value"]) or rel["order"] not in ("LT", "EQ", "GT"):
                    raise ValueError("后续关系来源字段不一致")
    elif kind == KINDS[3]:
        keys(p, ("scope", "input_frontier", "known_facts", "waiting_reasons", "domain", "requests"), kind)
        if p["scope"] != "TB-02-A":
            raise ValueError("knowledge scope 不符")
        integer(p["input_frontier"], "knowledge.input_frontier", -1)
        strings(p["waiting_reasons"], "knowledge.waiting_reasons")
        if p["known_facts"] != ["raw_ohlc", "inclusion_trace", "group_provenance"] or p["domain"] != "established_direction_without_extreme_identity_competition":
            raise ValueError("knowledge 已知事实/限定域声明不符")
        request_ids = set()
        for request in array(p["requests"], "knowledge.requests"):
            keys(request, ("request_id", "subject_id", "axes", "reasons", "source_coords"), "request")
            text(request["request_id"], "request_id")
            if request["request_id"] in request_ids:
                raise ValueError("逐请求语义槽重复")
            request_ids.add(request["request_id"])
            if request["subject_id"] is not None:
                text(request["subject_id"], "subject_id")
            allowed = {"input_quality": ("SUFFICIENT", "INSUFFICIENT", "INCONSISTENT"), "applicability": ("IN_DOMAIN", "OUTSIDE_DOMAIN", "DOMAIN_PROOF_MISSING"), "computation": ("COMPLETE", "INCOMPLETE"), "validity": ("CURRENT", "SUPERSEDED", "WITHDRAWN")}
            keys(request["axes"], allowed, "request.axes")
            if any(request["axes"][key] not in values for key, values in allowed.items()):
                raise ValueError("逐请求四轴值不属于已声明目录")
            strings(request["reasons"], "request.reasons")
            coords(request["source_coords"], "request.source_coords")
    else:
        raise ValueError("未知 TB02 typed kind")


def project_fact(row, parse, *, meta=None, raw=None, parse_canonical=None):
    return _project_fact(row, parse, meta=meta, raw=raw, parse_canonical=parse_canonical)


def _project_fact(row, parse, *, meta=None, raw=None, parse_canonical=None, _identity_hash=None):
    if type(row) is not dict:
        row = dict(zip(FACT_COLUMNS, row))
    result = {k: row[k] for k in FACT_COLUMNS if k not in ("payload_json", "input_refs_json", "source_coords_json")}
    for name in ("payload", "input_refs", "source_coords"):
        expected = dict if name == "payload" else list
        if parse_canonical is None:
            result[name] = parse(row[name + "_json"], expected)
            if row[name + "_json"] != canonical(result[name]).decode():
                raise ValueError("事实 JSON 列不是封存规范字节")
        else:
            # 明确注入的严格列解析器同时核原串；失败直接传播，不换解析路径。
            result[name] = parse_canonical(row[name + "_json"], expected)
    for name in ("object_revision", "first_known_generation", "published_generation", "withdrawn_generation"):
        if result[name] is not None:
            if type(result[name]) is not int:
                raise ValueError("事实持久整数类型不符")
            result[name] = str(result[name])
    result["lifecycle"] = "active" if result["withdrawn_generation"] is None else "withdrawn"
    if _identity_hash is None:
        validate_fact(result, meta, raw)
    else:
        _validate_fact(result, meta, raw, _identity_hash=_identity_hash)
    return result


def validate_fact(value, meta=None, raw=None):
    return _validate_fact(value, meta, raw)


def _validate_fact(value, meta=None, raw=None, *, _identity_hash=None):
    keys(value, FACT_FIELDS, "typed object")
    wire_tree(value)
    for key in ("object_id", "batch_id", "fact_key", "first_known_cut"):
        text(value[key], key)
    if value["kind"] not in KINDS or value["object_revision"] != "1":
        raise ValueError("typed kind/revision 未声明")
    first = integer(value["first_known_generation"], "first_known_generation", 1)
    published = integer(value["published_generation"], "published_generation", 1)
    if first != published or value["first_known_cut"] != f"cut-{first}":
        raise ValueError("事实首次获知/发布代不一致")
    wg = value["withdrawn_generation"]
    if wg is None:
        if (value["lifecycle"], value["withdrawal_reason"], value["superseded_by"]) != ("active", None, None):
            raise ValueError("活动事实携带撤回信息")
    else:
        if integer(wg, "withdrawn_generation", 1) <= first or value["lifecycle"] != "withdrawn":
            raise ValueError("事实生命周期不一致")
        if value["withdrawal_reason"] not in ("fact_removed", "superseded_by_revision") or (value["withdrawal_reason"] == "fact_removed") != (value["superseded_by"] is None):
            raise ValueError("事实撤回原因/替代目标不一致")
    # #1414：本次 wire_tree(value) 已完整覆盖同一未修改的 payload 原树。
    _validate_payload_fields(value["kind"], value["payload"])
    references = array(value["input_refs"], "input_refs")
    for ref in references:
        keys(ref, REF_FIELDS, "input_ref")
        for name in REF_FIELDS:
            text(ref[name], name)
        for name in ("input_revision", "revision", "seq", "source_coord"):
            integer(ref[name], name, 1 if name in ("input_revision", "revision") else 0)
        if ref["input_revision"] != ref["revision"] or not re.fullmatch(r"[a-f0-9]{64}", ref["payload_hash"]):
            raise ValueError("input_ref 修订/hash 不规范")
        if raw is not None:
            seq = int(ref["seq"])
            if seq >= len(raw) or any(ref[key] != raw[seq][key] for key in REF_FIELDS):
                raise ValueError("typed input_ref 与完整已接纳账不一致")
    source_coords = coords(value["source_coords"], "source_coords")
    if source_coords != [ref["source_coord"] for ref in references]:
        raise ValueError("事实源坐标与扁平输入引用不一一对应")
    p, kind = value["payload"], value["kind"]
    slot = p["slot"] if kind in BI_KINDS + SEGMENT_KINDS + CENTER_KINDS else p["incoming"]["source_coord"] if kind == KINDS[0] else p["group_anchor"] if kind == KINDS[1] else p["window"] if kind == KINDS[2] else "TB-02-A"
    if value["fact_key"] != canonical([kind, slot]).decode():
        raise ValueError("fact_key 与真实语义槽不符")
    if meta is not None:
        if kind in SEGMENT_KINDS:
            expected_schema = "s-segment/2" if meta["rule_revision"] in (SEGMENT_SECOND_RULE, CENTER_RULE) else "s-segment/1" if meta["rule_revision"] == "s2-axis-quantifiers+new-bi/1+segment-first/1" else None
            if p["schema_revision"] != expected_schema:
                raise ValueError("段合同与cut规则版本不符")
        if kind in CENTER_KINDS and meta["rule_revision"] != CENTER_RULE:
            raise _e("RULE_MIX", "s-center/1 对象出现在旧规则 cut")
        identity = {key: value[key] for key in ("kind", "fact_key", "payload", "input_refs")}
        identity.update(profile_id=meta["profile_id"], rule_revision=meta["rule_revision"])
        digest = (hashlib.sha256(canonical(identity)).hexdigest() if _identity_hash is None
                  else _identity_hash(identity))
        if value["object_id"] != "obj-" + digest:
            raise ValueError("typed object 内容身份 hash 不闭合")


def validate_sources(value, effective):
    """只核已有字段的原始来源与已封存对象引用，不计算包含、方向或分型。"""
    sources = set(value["source_coords"])
    def walk(item, key=None):
        if type(item) is dict:
            for name, child in item.items():
                walk(child, name)
        elif type(item) is list:
            if key in ("members", "high_sources", "low_sources", "source_coords", "window"):
                if not set(item) <= sources:
                    raise ValueError("typed payload 使用未绑定的原始坐标")
            else:
                for child in item:
                    walk(child)
        elif key in ("source_coord", "group_anchor", "established_at") and item not in sources:
            raise ValueError("typed payload 使用未绑定的原始坐标")
    walk(value["payload"])
    for ref in value["input_refs"]:
        if effective.get(ref["source_coord"]) != ref:
            raise ValueError("typed input_ref 不是该发布 cut 的有效源修订")
    by_source = {ref["source_coord"]: ref for ref in value["input_refs"]}
    p, kind = value["payload"], value["kind"]
    if kind == KINDS[0]:
        raw_bars = [p["incoming"]]
        if p["direction_evidence"] is not None:
            raw_bars.append(p["direction_evidence"]["incoming"])
    elif kind == KINDS[1]:
        raw_bars = [p[key]["incoming"] for key in ("direction_evidence", "confirmation_evidence") if p[key] is not None]
    elif kind == KINDS[2]:
        raw_bars = p["descriptive_labels"]["raw_ohlc"] + [item["bar"] for item in p["subsequent_development"]["bars"]]
    else:
        raw_bars = []
    return by_source, raw_bars


def validate_axes(axes, object_ids=None):
    keys(axes, CENTER_AXES if "CC-016" in axes else SEGMENT_SECOND_AXES if "CC-014" in axes else FIRST_SEGMENT_AXES if "CC-011" in axes else BI_AXES if "CC-008" in axes else LEGACY_AXES, "catalog.axes")
    for cid, value in axes.items():
        keys(value, ("impl_status", "proof_status", "run_status", "evidence"), cid)
        if value["impl_status"] not in ("implemented", "not_implemented") or value["proof_status"] != "not_proved" or value["run_status"] not in ("run", "waiting", "not_run"):
            raise ValueError("逐轴实现/证明/运行状态不符")
        evidence = value["evidence"]
        base = ("scope", "object_ids", "waiting_reasons", "raw_revisions", "withdrawals", "replaces") if cid == "CC-056" else ("scope", "object_ids", "waiting_reasons")
        center_scoped = evidence.get("scope") == "TB-02-E"
        scoped = evidence.get("scope") == "TB-02-D"
        keys(evidence, base + (("delivered_subobligations", "deferred_subobligations") if scoped or center_scoped else ()), cid + ".evidence")
        if center_scoped:
            if evidence["delivered_subobligations"] != TB02E_DELIVERED or evidence["deferred_subobligations"] != TB02E_DEFERRED:
                raise ValueError("中枢子义务范围不符")
            if value["impl_status"] != "not_implemented":
                raise ValueError("有限交付冒称全轴完成")
        if scoped:
            if evidence["delivered_subobligations"] != ["finite_complete_prefix_strict_second", "independent_stroke_and_segment_break_facts"] or evidence["deferred_subobligations"] != ["general_equal_identity", "general_scan_continuation", "global_G006_bridge"]:
                raise ValueError("第二种子义务范围不符")
            if cid in ("CC-012", "CC-013", "CC-014") and value["impl_status"] != "not_implemented":
                raise ValueError("有限交付冒称全轴完成")
        if cid == "CC-056":
            for name in ("raw_revisions", "withdrawals", "replaces"):
                array(evidence[name], "revision."+name)
            for ref in evidence["raw_revisions"]:
                keys(ref, REF_FIELDS, "revision.raw_ref")
                for key in REF_FIELDS:
                    text(ref[key], key)
                if integer(ref["revision"], "revision", 2) != integer(ref["input_revision"], "input_revision", 2):
                    raise ValueError("修订轴引用不是合法修订")
            actual_change = any(evidence[name] for name in ("raw_revisions", "withdrawals", "replaces"))
            if value["run_status"] != ("run" if actual_change else "not_run"):
                raise ValueError("修订轴状态与实际修订/撤回/替代证据不一致")
        if evidence["scope"] != ("TB-02-E" if cid in ("CC-016", "CC-017", "CC-022") else "TB-02-D" if "CC-014" in axes and cid in ("CC-011", "CC-012", "CC-013", "CC-014") else "TB-02-C" if cid in ("CC-011", "CC-012", "CC-013") else "TB-02-A"):
            raise ValueError("逐轴证据范围不符")
        ids = strings(evidence["object_ids"], "axis.object_ids")
        if len(ids) != len(set(ids)) or (object_ids is not None and not set(ids) <= object_ids):
            raise ValueError("逐轴对象证据缺失/重复")
        strings(evidence["waiting_reasons"], "axis.waiting_reasons")


def validate_segment_links(records, axes=None):
    """#1414：核同cut内容引用和FIRST/SECOND角色，不计算段终结。"""
    by_id = {r["object_id"]: r for r in records}
    for r in records:
        if r["kind"] != "CC-013.segment" or r["payload"]["schema_revision"] != "s-segment/2":
            continue
        d = r["payload"]["data"]
        if d["second_sequence_id"] is None and not d["fact_vector"]["scan_complete"]:
            raise ValueError("未完成扫描缺SECOND资源证据")
        for prefix, role in (("sequence", "FIRST_SEQUENCE"), ("second_sequence", "SECOND_SEQUENCE")):
            oid = d[prefix + "_object_id"]
            if oid is None:
                continue
            seq = by_id.get(oid)
            if seq is None or seq["kind"] != "CC-012.feature_sequence" or seq["payload"]["schema_revision"] != "s-segment/2":
                raise ValueError("段引用缺同cut序列对象")
            q = seq["payload"]["data"]
            if q["entity_id"] != d[prefix + "_id"] or q["segment_id"] != d["entity_id"] or q["role"] != role:
                raise ValueError("段序列引用身份/角色不符")
            if not set(seq["source_coords"]) <= set(r["source_coords"]):
                raise ValueError("段缺序列实际来源")
            if role == "FIRST_SEQUENCE" and d["strict"] is not None and not any(c["elements"] == d["strict"]["elements"] for c in q["facts"]["checks"]):
                raise ValueError("strict未绑定FIRST检查")
            if role == "SECOND_SEQUENCE":
                incomplete = q["facts"]["resource_truncated"] and q["facts"]["matched"] is None
                if d["fact_vector"]["scan_complete"] == incomplete:
                    raise ValueError("段事实可知性未绑定SECOND扫描证据")
            if role == "SECOND_SEQUENCE" and d["result"] == "CASE_TWO":
                if q["second_sequence_status"] != "CONFIRMED" or d["fact_vector"]["termination_witness"]["second_match"] != q["facts"]["matched"]:
                    raise ValueError("第二种终结证书未绑定同次SECOND命中")
    if axes is not None and "CC-014" in axes:
        if axes["CC-014"]["evidence"]["object_ids"] != axes["CC-013"]["evidence"]["object_ids"] or any(by_id[oid]["kind"] != "CC-013.segment" for oid in axes["CC-014"]["evidence"]["object_ids"]):
            raise ValueError("CC014未绑定CC013实际对象")


def validate_shape_refs(shape, effective=None, groups=None):
    """CC006 三组真实成员与构造/当前分组边界依据分开；只对拍 S 自己的组事实。"""
    if shape["kind"] != "CC-006.local_shape":
        raise ValueError("OHLC 旧对象表中出现未声明 kind")
    window = [shape[key] for key in ("window_start", "window_mid", "window_end")]
    references = array(shape["input_refs"], "shape.input_refs")
    if len(references) != 3:
        raise ValueError("CC006 缺完整三组来源")
    union = set()
    previous_index = None
    for anchor, group in zip(window, references):
        keys(group, ("merged_index", "raw_refs", "dependency_refs"), "OHLC input_refs.group")
        merged_index = integer(group["merged_index"], "merged_index", 0)
        if previous_index is not None and merged_index != previous_index + 1:
            raise ValueError("CC006 来源组序号不连续")
        previous_index = merged_index
        coords_by_role = {}
        for name in ("raw_refs", "dependency_refs"):
            refs = array(group[name], name)
            for ref in refs:
                keys(ref, REF_FIELDS, name)
                for key in REF_FIELDS:
                    text(ref[key], key)
                for key in ("revision", "input_revision", "seq", "source_coord"):
                    integer(ref[key], key, 1 if key in ("revision", "input_revision") else 0)
                if ref["revision"] != ref["input_revision"] or not re.fullmatch(r"[a-f0-9]{64}", ref["payload_hash"]):
                    raise ValueError("CC006 来源修订/hash 不规范")
                if effective is not None and effective.get(ref["source_coord"]) != ref:
                    raise ValueError("CC006 来源不是该 cut 的有效原始修订")
            coords_by_role[name] = coords([ref["source_coord"] for ref in refs], name)
            union.update(coords_by_role[name])
        members, dependencies = coords_by_role["raw_refs"], coords_by_role["dependency_refs"]
        if not members or set(members) & set(dependencies):
            raise ValueError("CC006 成员为空或将构造/边界依据伪装成成员")
        if members[0] != anchor:
            raise ValueError("CC006 实际成员首坐标与窗口锚不同")
        if groups is not None:
            fact = groups.get(anchor)
            if fact is None or fact["members"] != members or fact["group_index"] != group["merged_index"]:
                raise ValueError("CC006 成员与同 cut 完整组映射不同")
            boundary_sources = set()
            for name in ("direction_evidence", "confirmation_evidence"):
                if fact[name] is not None:
                    boundary_sources.update(fact[name]["source_coords"])
            if set(dependencies) != boundary_sources - set(members):
                raise ValueError("CC006 构造/分组边界依据与同 cut 原始依赖不同")
    if coords(shape["source_coords"], "shape.source_coords") != sorted(union, key=int):
        raise ValueError("CC006 source_coords 不等于成员与构造/边界依据并集")


def validate_segment_payload(kind, p):
    """只验类型、身份、适用性与引用闭合；不另算分型/包含或 seed 谓词。"""
    keys(p, ("schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"), "段合同")
    v2 = p["schema_revision"] == "s-segment/2"
    expected_version = ("s-segment/2", "segment-second/1", "seed_closed_strict_second", "main") if v2 else ("s-segment/1", "segment-seed-first/1", "seed_closed_first_no_gap", "main")
    if (p["schema_revision"], p["semantic_version"], p["policy_version"], p["view_role"]) != expected_version:
        raise ValueError("段规则/范围版本不符")
    integer(p["object_generation"], "object_generation", 1)
    d = p["data"]

    def known(k):
        keys(k, ("generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"), "段获知时点")
        integer(k["generation"], "generation", 1)
        integer(k["input_frontier"], "input_frontier", -1)
        # 空输入也发布具名等待态，没有虚造接纳回执。
        for name in ("receipt_id", "received_at"):
            if k[name] is not None:
                text(k[name], name)
        if k["semantic_commit_ns"] is not None:
            integer(k["semantic_commit_ns"], "semantic_commit_ns", 0)

    refs = {}
    for r in d.get("stroke_refs", []):
        keys(r, ("stroke_index", "entity_id", "start_anchor", "end_anchor", "start_price", "end_price", "start_roots", "end_roots"), "段笔引用")
        for name in ("stroke_index", "start_anchor", "end_anchor"):
            integer(r[name], name, 0)
        for name in ("start_price", "end_price"):
            integer(r[name], name)
        for name in ("start_roots", "end_roots"):
            if not coords(r[name], name):
                raise ValueError("段笔引用缺实际根")
        if r["entity_id"] != f'bi:{p["object_generation"]}:{r["start_anchor"]}' or r["stroke_index"] in refs:
            raise ValueError("段笔身份未绑定代际/重复")
        refs[r["stroke_index"]] = r

    def indices(xs):
        coords(xs, "stroke_indices")
        if not set(xs) <= refs.keys():
            raise ValueError("段事实引用了未绑定的笔")

    def construction(v):
        keys(v, ("start_stroke", "stroke_indices", "length", "vector", "overlap", "overlap_relation", "direction", "failed_conditions"), "段完整条件")
        integer(v["start_stroke"], "start_stroke", 0)
        indices(v["stroke_indices"])
        n = len(v["stroke_indices"])
        start = int(v["start_stroke"])
        if v["stroke_indices"] != [str(i) for i in range(start, start + n)]:
            raise ValueError("候选笔片段不连续")
        expected = f"n={n}" if n < 3 else "even_ge3" if n % 2 == 0 else "odd_ge3"
        if v["length"] != expected:
            raise ValueError("段长度分域不符")
        vector = v["vector"]
        if expected == "odd_ge3":
            if type(vector) is not list or len(vector) != 4 or any(type(x) is not bool for x in vector):
                raise ValueError("段四条件向量缺位/错型")
        elif vector is not None:
            raise ValueError("非适用长度虚填四条件")
        if n >= 3:
            if len(array(v["overlap"], "overlap")) != 2 or v["overlap_relation"] not in ("OVERLAP", "TOUCH", "DISJOINT"):
                raise ValueError("缺闭区间见证")
            for x in v["overlap"]:
                integer(x, "overlap")
        elif v["overlap"] is not None or v["overlap_relation"] is not None:
            raise ValueError("不足三笔虚填公共区间")
        if vector == [True] * 4:
            if v["direction"] not in ("UP", "DOWN") or v["failed_conditions"]:
                raise ValueError("有效候选缺方向或仍称失败")
        elif v["direction"] is not None:
            raise ValueError("失败候选冒称有效方向")
        strings(v["failed_conditions"], "failed_conditions")

    def element(e):
        keys(e, ("high", "low", "stroke_idx", "stroke_indices"), "FIRST元素")
        integer(e["high"], "high")
        integer(e["low"], "low")
        indices(e["stroke_indices"])
        if e["stroke_idx"] not in e["stroke_indices"]:
            raise ValueError("元素代表笔不属于来源")

    if kind in (SEGMENT_KINDS[0], SEGMENT_KINDS[6]):
        keys(d, ("entity_id", "construction", "stroke_refs"), kind)
        construction(d["construction"])
        if (kind == SEGMENT_KINDS[0]) != (d["construction"]["vector"] == [True] * 4):
            raise ValueError("未满足完整条件的候选冒充seed")
    elif kind == SEGMENT_KINDS[1]:
        keys(d, ("entity_id", "segment_id", "role", "direction", "member_direction", "facts", "stroke_refs", "scan", "second_sequence_status"), kind)
        second = v2 and d["role"] == "SECOND_SEQUENCE"
        statuses = ("PENDING", "CONFIRMED") if second else ("NOT_APPLICABLE", "PENDING", "CONFIRMED") if v2 else ("NOT_APPLICABLE", "WAITING_FOLLOWUP")
        if d["role"] not in (("FIRST_SEQUENCE", "SECOND_SEQUENCE") if v2 else ("FIRST_SEQUENCE",)) or d["member_direction"] not in ("UP", "DOWN") or d["second_sequence_status"] not in statuses:
            raise ValueError("FIRST角色/后续片边界不符")
        direction(d["direction"])
        keys(d["scan"], ("from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "tail_cache_window"), "扫描范围")
        for k, v in d["scan"].items():
            if k != "resource_truncated":
                integer(v, k, 0)
        if type(d["scan"]["resource_truncated"]) is not bool:
            raise ValueError("资源截断状态缺失")
        if not refs:
            raise ValueError("特征序列缺段笔引用")
        # 身份锚＝stroke_refs 中数值最小的笔号，即段起笔；wire 整数是十进制文本，须按数值取最小。
        segment_start = min(refs, key=int)
        if not second and d["scan"]["from_stroke"] != segment_start:
            raise ValueError("FIRST扫描起点不是段起笔")
        f = d["facts"]
        keys(f, ("raw_elements", "standard", "steps", "checks") + (("from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "matched") if second else ()), "序列事实")
        if second:
            for name in ("from_stroke", "input_stroke_count"):
                integer(f[name], name, 0)
            integer(f["through_stroke"], "through_stroke", -1)
            if type(f["resource_truncated"]) is not bool or d["scan"]["tail_cache_window"] != "0" or any(f[name] != d["scan"][name] for name in ("from_stroke", "through_stroke", "input_stroke_count", "resource_truncated")) or int(d["scan"]["from_stroke"]) <= int(segment_start):
                raise ValueError("SECOND扫描边界不符")
        for e in array(f["raw_elements"], "raw_elements") + array(f["standard"], "standard"):
            element(e)
        for step in array(f["steps"], "steps"):
            keys(step, ("incoming", "before", "after", "action", "merge_direction", "scope"), "包含步骤")
            element(step["incoming"])
            element(step["after"])
            if step["before"] is not None:
                element(step["before"])
            action = step["action"]
            if action not in (("append", "merge") if second else ("append", "merge", "turn_boundary")) or step["scope"] != ("SECOND_SEQUENCE" if second else "TURN_HYPOTHESIS" if action == "turn_boundary" else "SAME_SEQUENCE"):
                raise ValueError("包含跨作用域或动作未声明")
            if (action == "merge" and step["merge_direction"] not in ("UP", "DOWN")) or (action != "merge" and step["merge_direction"] is not None):
                raise ValueError("方向性包含缺见证")
        def second_check(check):
            keys(check, ("elements", "top_fractal", "bottom_fractal", "through_stroke"), "SECOND分型见证")
            if len(array(check["elements"], "elements")) != 3:
                raise ValueError("SECOND分型窗口缺元素")
            for e in check["elements"]:
                element(e)
            if any(type(check[name]) is not bool for name in ("top_fractal", "bottom_fractal")):
                raise ValueError("SECOND分型错型")
            indices([check["through_stroke"]])
        if second:
            if (f["matched"] is None) != (d["second_sequence_status"] == "PENDING"):
                raise ValueError("SECOND命中与状态不符")
            if f["matched"] is not None:
                second_check(f["matched"])
                if f["matched"] not in f["checks"] or not (f["matched"]["top_fractal"] or f["matched"]["bottom_fractal"]):
                    raise ValueError("SECOND命中未绑定检查")
        for check in array(f["checks"], "checks"):
            if second:
                second_check(check)
                continue
            keys(check, ("elements", "first_fractal", "gap", "gap_closed_by_c", "provisional", "turn_hypothesis_pair"), "分型见证")
            if len(array(check["elements"], "elements")) != 3:
                raise ValueError("分型窗口缺元素")
            for e in check["elements"]:
                element(e)
            expected_pair = [e["stroke_idx"] for e in check["elements"][:2]] if check["first_fractal"] else None
            if check["turn_hypothesis_pair"] != expected_pair:
                raise ValueError("假设转折点两侧未绑定a/b元素")
            for k in ("first_fractal", "provisional"):
                if type(check[k]) is not bool:
                    raise ValueError("分型事实错型")
            for k in ("gap", "gap_closed_by_c"):
                if check[k] is not None and type(check[k]) is not bool:
                    raise ValueError("gap见证错型")
            if (not check["first_fractal"] and check["gap"] is not None) or (check["gap"] is not True and check["gap_closed_by_c"] is not None):
                raise ValueError("不适用条件虚填布尔")
    elif kind == SEGMENT_KINDS[2]:
        keys(d, ("entity_id", "entity_revision", "seed_id", "sequence_id", "sequence_object_id", "construction", "result", "waiting_reasons", "termination_attempts", "geometric_end", "trigger_stroke", "formed_known_at", "terminated_known_at", "stroke_refs") + (("second_sequence_id", "second_sequence_object_id", "strict", "fact_vector") if v2 else ()), kind)
        integer(d["entity_revision"], "entity_revision", 1)
        construction(d["construction"])
        strings(d["waiting_reasons"], "waiting_reasons")
        known(d["formed_known_at"])
        for attempt in array(d["termination_attempts"], "termination_attempts"):
            keys(attempt, ("end_stroke", "trigger_stroke", "minimum_count_met", "construction"), "终结准入见证")
            indices([attempt["end_stroke"]])
            indices([attempt["trigger_stroke"]])
            if type(attempt["minimum_count_met"]) is not bool:
                raise ValueError("最小笔数条件错型")
            construction(attempt["construction"])
        if d["result"] not in ("NO_FIRST", "CASE_ONE", "PENDING_CASE_TWO", "FAILED_GEOMETRY") + (("CASE_TWO",) if v2 else ()):
            raise ValueError("冒称交付第二种终结")
        if d["result"] in ("CASE_ONE", "CASE_TWO"):
            known(d["terminated_known_at"])
            e = d["geometric_end"]
            keys(e, ("stroke_index", "group_anchor", "extreme_roots", "price"), "几何段端")
            indices([e["stroke_index"]])
            indices([d["trigger_stroke"]])
            coords(e["extreme_roots"], "extreme_roots")
            integer(e["group_anchor"], "group_anchor", 0)
            integer(e["price"], "price")
        elif any(d[k] is not None for k in ("geometric_end", "trigger_stroke", "terminated_known_at")):
            raise ValueError("发展态虚填终结")
        if v2:
            strict = d["strict"]
            if strict is not None:
                keys(strict, ("elements", "raw_gap", "gap_closed_by_c", "effective_case", "closure_operator", "closure_left", "closure_right"), "strict事实")
                if len(array(strict["elements"], "elements")) != 3:
                    raise ValueError("strict缺FIRST窗口")
                for e in strict["elements"]:
                    element(e)
                if type(strict["raw_gap"]) is not bool or (type(strict["gap_closed_by_c"]) is not bool if strict["raw_gap"] else strict["gap_closed_by_c"] is not None):
                    raise ValueError("strict适用性不符")
                if strict["effective_case"] not in ("CASE_ONE", "CASE_TWO") or strict["closure_operator"] not in ("<=", ">="):
                    raise ValueError("strict规则未声明")
                integer(strict["closure_left"], "closure_left")
                integer(strict["closure_right"], "closure_right")
                a, _, c = strict["elements"]
                up = strict["closure_operator"] == "<="
                expected_closure = ("<=", c["low"], a["high"]) if up else (">=", c["high"], a["low"])
                if (strict["closure_operator"], strict["closure_left"], strict["closure_right"]) != expected_closure:
                    raise ValueError("strict比较未绑定FIRST元素")
            if d["result"] in ("CASE_ONE", "CASE_TWO") and (strict is None or strict["effective_case"] != d["result"]):
                raise ValueError("终结缺适用strict对象")
            paired = (d["second_sequence_id"], d["second_sequence_object_id"])
            if any(x is None for x in paired) and paired != (None, None):
                raise ValueError("SECOND身份/对象引用缺位")
            if paired[0] is not None:
                if paired[0] != "second:" + d["entity_id"]:
                    raise ValueError("SECOND身份未绑定候选")
                text(paired[1], "second_sequence_object_id")
            if d["result"] in ("CASE_TWO", "PENDING_CASE_TWO") and paired[0] is None:
                raise ValueError("第二种缺SECOND引用")
            b = d["fact_vector"]
            keys(b, ("stroke_broken", "new_segment_formed", "segment_broken", "stroke_break_witness", "scan_from_stroke", "scan_through_stroke", "scan_complete", "termination_witness"), "独立破坏事实")
            if any(type(b[name]) is not bool for name in ("stroke_broken", "scan_complete")) or any(b[name] is not None and type(b[name]) is not bool for name in ("new_segment_formed", "segment_broken")):
                raise ValueError("破坏事实错型")
            integer(b["scan_from_stroke"], "scan_from_stroke", 0)
            integer(b["scan_through_stroke"], "scan_through_stroke", -1)
            terminated = d["result"] in ("CASE_ONE", "CASE_TWO")
            if (b["new_segment_formed"] is True) != terminated or b["segment_broken"] != b["new_segment_formed"] or (b["new_segment_formed"] is None) != (not b["scan_complete"]):
                raise ValueError("段破坏适用性不符")
            certificate = b["termination_witness"]
            if (certificate is not None) != (b["new_segment_formed"] is True):
                raise ValueError("段破坏缺同次终结证书")
            if certificate is not None:
                keys(certificate, ("kind", "strict", "second_match", "construction", "minimum_count_met", "through_stroke"), "终结证书")
                if certificate["kind"] != d["result"] or certificate["strict"] != strict or certificate["construction"] != d["construction"]:
                    raise ValueError("终结证书未绑定同次strict/几何")
                construction(certificate["construction"])
                if certificate["minimum_count_met"] is not True or certificate["construction"]["vector"] != [True]*4 or certificate["through_stroke"] != d["trigger_stroke"]:
                    raise ValueError("终结证书缺有效准入/实际前沿")
                indices([certificate["through_stroke"]])
                match = certificate["second_match"]
                if (match is not None) != (certificate["kind"] == "CASE_TWO"):
                    raise ValueError("终结证书SECOND适用性不符")
                if match is not None:
                    keys(match, ("elements", "top_fractal", "bottom_fractal", "through_stroke"), "SECOND终结证书")
                    if len(array(match["elements"], "elements")) != 3 or any(type(match[k]) is not bool for k in ("top_fractal", "bottom_fractal")) or not (match["top_fractal"] or match["bottom_fractal"]):
                        raise ValueError("终结证书缺任意分型见证")
                    for e in match["elements"]: element(e)
                    indices([match["through_stroke"]])
            w = b["stroke_break_witness"]
            if (w is not None) != b["stroke_broken"]:
                raise ValueError("笔破坏缺独立见证")
            if w is not None:
                keys(w, ("earlier_stroke", "later_stroke", "earlier_price", "later_price", "comparison"), "笔破坏见证")
                indices([w["earlier_stroke"], w["later_stroke"]])
                for name in ("earlier_price", "later_price"):
                    integer(w[name], name)
                if w["comparison"] not in ("<=", ">="):
                    raise ValueError("笔破坏比较未声明")
    elif kind == SEGMENT_KINDS[3]:
        keys(d, ("source_id", "relation_kind", "target_id", "version", "witness_object_id"), kind)
        for v in d.values():
            text(v, "段关系")
        if d["relation_kind"] not in ("member_of", "derived_from"):
            raise ValueError("未声明段关系")
    elif kind == SEGMENT_KINDS[4]:
        keys(d, ("entity_id", "change", "before", "after", "known_at", "object_id"), kind)
        if d["change"] not in ("formed", "updated", "terminated", "withdrawn"):
            raise ValueError("未声明段变化")
        known(d["known_at"])
        for name in ("before", "after"):
            if d[name] is not None:
                validate_segment_payload(SEGMENT_KINDS[2], {**p, "slot": d[name]["entity_id"], "object_generation": d[name]["entity_id"].split(":")[1], "data": d[name]})
    else:
        keys(d, ("input_frontier", "waiting_reasons", "known_at", "delivered_scope", "deferred_scope"), kind)
        integer(d["input_frontier"], "input_frontier", -1)
        known(d["known_at"])
        strings(d["waiting_reasons"], "waiting_reasons")
        if d["delivered_scope"] != ("seed_strict_first_and_finite_second" if v2 else "seed_and_no_gap_first_kind") or d["deferred_scope"] != (["general_equal_identity", "general_scan_continuation", "global_G006_bridge"] if v2 else ["second_kind", "general_equal_identity"]):
            raise ValueError("段交付范围不符")
    if kind in SEGMENT_KINDS[:3] + SEGMENT_KINDS[6:]:
        if kind == SEGMENT_KINDS[1]:
            expected_segment = f'segment:{p["object_generation"]}:{refs[segment_start]["start_anchor"]}'
            if d["segment_id"] != expected_segment or d["entity_id"] != ("second:" if d["role"] == "SECOND_SEQUENCE" else "first:") + expected_segment:
                raise ValueError("FIRST身份未绑定候选")
            if (d["direction"] == d["member_direction"]) != (d["role"] == "SECOND_SEQUENCE"):
                raise ValueError("FIRST未取反向笔")
        else:
            first = d["construction"]["start_stroke"]
            anchor = refs[first]["start_anchor"] if refs else "empty"
            expected = f'{"seed" if kind in (SEGMENT_KINDS[0], SEGMENT_KINDS[6]) else "segment"}:{p["object_generation"]}:{anchor}'
            if d["entity_id"] != expected:
                raise ValueError("段身份未绑定代际/起笔")
            if kind == SEGMENT_KINDS[2]:
                if d["seed_id"] != f'seed:{p["object_generation"]}:{anchor}' or d["sequence_id"] != 'first:' + expected:
                    raise ValueError("段缺同源seed/FIRST身份")
                text(d["sequence_object_id"], "sequence_object_id")
                if d["geometric_end"] is not None:
                    e = d["geometric_end"]
                    r = refs[e["stroke_index"]]
                    if (e["group_anchor"], e["price"], e["extreme_roots"]) != (r["end_anchor"], r["end_price"], r["end_roots"]):
                        raise ValueError("几何段端与实际笔端不符")
    slot = [d["source_id"], d["relation_kind"], d["target_id"]] if kind == SEGMENT_KINDS[3] else [d["known_at"]["generation"], d["entity_id"], d["change"]] if kind == SEGMENT_KINDS[4] else ("TB-02-D" if v2 else "TB-02-C") if kind == SEGMENT_KINDS[5] else d["entity_id"]
    if p["slot"] != slot:
        raise ValueError("段语义槽身份不符")


def validate_bi_payload(kind, p):
    """#1392：仅验证 S 发布的完整形状和来源引用，不代算成笔。"""
    keys(p, ("schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"), kind)
    if (p["schema_revision"], p["semantic_version"], p["policy_version"], p["view_role"]) != ("s-new-bi/1", "new-bi-dual-coordinate/1", "standard_raw_gap_3", "main"):
        raise ValueError("新笔合同/规则/政策版本不符")
    integer(p["object_generation"], "object_generation", 1)
    data = p["data"]

    def endpoint(e):
        keys(e, ("kind", "merged_index", "group_anchor", "price", "extreme_roots", "raw_position", "source_coords", "sealed_at", "waiting_reasons"), "endpoint")
        if e["kind"] not in ("TOP", "BOTTOM"):
            raise ValueError("端点类型未声明")
        for k in ("merged_index", "group_anchor"):
            integer(e[k], k, 0)
        integer(e["price"], "price")
        coords(e["extreme_roots"], "extreme_roots")
        coords(e["source_coords"], "source_coords")
        if not set(e["extreme_roots"]) <= set(e["source_coords"]):
            raise ValueError("实际根无原始来源")
        for k in ("raw_position", "sealed_at"):
            if e[k] is not None:
                integer(e[k], k, 0)
        strings(e["waiting_reasons"], "waiting_reasons")

    def conditions(c):
        keys(c, ("merged_gap", "raw_between_actual_extrema", "top_price", "bottom_price", "vector", "failed_conditions", "waiting_reasons"), "conditions")
        integer(c["merged_gap"], "merged_gap", 0)
        if c["raw_between_actual_extrema"] is not None:
            integer(c["raw_between_actual_extrema"], "raw_between", 0)
        for k in ("top_price", "bottom_price"):
            integer(c[k], k)
        if len(array(c["vector"], "vector")) != 3 or any(v is not None and type(v) is not bool for v in c["vector"]):
            raise ValueError("完整条件向量缺位/错型")
        strings(c["failed_conditions"], "failed_conditions")
        strings(c["waiting_reasons"], "waiting_reasons")

    def known(k):
        keys(k, ("generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"), "known_at")
        integer(k["generation"], "known_generation", 1)
        integer(k["input_frontier"], "known_frontier", 0)
        text(k["receipt_id"], "known_receipt")
        text(k["received_at"], "received_at")
        if k["semantic_commit_ns"] is not None:
            integer(k["semantic_commit_ns"], "semantic_commit_ns", 0)

    if kind == BI_KINDS[0]:
        endpoint(data)
    elif kind in BI_KINDS[1:3]:
        keys(data, ("old", "new", "endpoint_kind_pair", "conditions", "comparison", "selection", "retained_anchors", "waiting_reasons"), kind)
        endpoint(data["old"])
        endpoint(data["new"])
        if data["endpoint_kind_pair"] != data["old"]["kind"] + "/" + data["new"]["kind"]:
            raise ValueError("端点分域标签不闭合")
        if kind == BI_KINDS[1]:
            if data["old"]["kind"] == data["new"]["kind"]:
                raise ValueError("异型请求的端点实际同型")
            conditions(data["conditions"])
            if data["comparison"] is not None or data["selection"] is not None:
                raise ValueError("异型域混入同型选择")
        elif data["old"]["kind"] != data["new"]["kind"] or data["conditions"] is not None or data["comparison"] not in ("LT", "EQ", "GT") or data["selection"] not in ("KEEP", "REPLACE", "UNDETERMINED"):
            raise ValueError("同型域混入异型假位或选择缺失")
        coords(data["retained_anchors"], "retained_anchors")
        strings(data["waiting_reasons"], "waiting_reasons")
    elif kind == BI_KINDS[3]:
        keys(data, ("identity_anchor", "start", "end", "formation", "confirmation", "entity_id", "entity_revision", "state", "formed_known_at", "confirmed_known_at", "formed_evidence", "version_causes"), kind)
        integer(data["identity_anchor"], "identity_anchor", 0)
        integer(data["entity_revision"], "entity_revision", 1)
        text(data["entity_id"], "entity_id")
        if data["entity_id"] != "bi:" + p["object_generation"] + ":" + data["identity_anchor"]:
            raise ValueError("笔身份未绑定输入代际与形成起点")
        endpoint(data["start"])
        endpoint(data["end"])
        conditions(data["formation"])
        conditions(data["formed_evidence"])
        known(data["formed_known_at"])
        if data["state"] != ("FORMED_UNCONFIRMED" if data["confirmation"] is None else "CONFIRMED"):
            raise ValueError("未声明笔生命周期")
        if data["confirmation"] is not None:
            c = data["confirmation"]
            keys(c, ("successor_start", "successor_end", "successor_conditions", "right_group_sealed_at", "source_coords"), "confirmation")
            for k in ("successor_start", "successor_end", "right_group_sealed_at"):
                integer(c[k], k, 0)
            conditions(c["successor_conditions"])
            coords(c["source_coords"], "confirmation.sources")
            known(data["confirmed_known_at"])
        elif data["confirmed_known_at"] is not None:
            raise ValueError("无见证却有确认获知时间")
        strings(data["version_causes"], "version_causes")
    elif kind == BI_KINDS[4]:
        keys(data, ("source_id", "relation_kind", "target_id", "version", "witness_object_id"), kind)
        for value in data.values():
            text(value, "relation")
        if data["relation_kind"] not in ("member_of", "derived_from", "successor_of", "leaves", "retests", "returns_to", "extends", "newborn_after", "expands_with", "caused_turn_at", "confirms", "selected_from"):
            raise ValueError("未声明关系种类")
    elif kind == BI_KINDS[5]:
        keys(data, ("entity_id", "change", "before", "after", "known_at", "event_at", "causes", "object_id"), kind)
        text(data["entity_id"], "entity_id")
        if data["change"] not in ("formed", "extended", "endpoint_replaced", "confirmed", "source_or_boundary_updated", "withdrawn"):
            raise ValueError("未声明变化种类")
        known(data["known_at"])
        integer(data["event_at"], "event_at")
        strings(data["causes"], "causes")
        for k in ("before", "after"):
            if data[k] is not None:
                validate_bi_payload(BI_KINDS[3], {**p, "object_generation": data[k]["entity_id"].split(":")[1], "data": data[k]})
    else:
        keys(data, ("input_frontier", "version_causes", "identity_basis", "classification_obligation", "waiting_reasons", "known_at"), kind)
        integer(data["input_frontier"], "input_frontier", -1)
        strings(data["version_causes"], "version_causes")
        strings(data["waiting_reasons"], "waiting_reasons")
        known(data["known_at"])


# ── #1419 TB-02-E 中枢事实（schema `s-center/1`，tb1419-schema.md v1）─────────────────────────────
# 只验类型、身份、引用闭合与跨对象一致；中枢判定本身由 S 写者的共用扫描内核给出。每个拒绝以稳定原因码
# `TB02E_<CODE>:` 开头，浏览器端 tb01c-client.js 对同一语料给出同一原因码（test_tb02e_contract 逐条对拍）。
CENTER_TUPLE = ("s-center/1", "center-seed/1", "strict_seed_zn_outer_c_resume", "main")
TB02E_DELIVERED = ["l0_strict_seed_vector_and_core_order", "zn_outer_inner_roles_with_contributors", "c_resume_assignment_and_connectors"]
TB02E_DEFERRED = ["global_G009_upper_levels_1429", "ge9_upgrade_1429", "third_class_and_break_1434", "expansion_newborn_1424", "connector_upper_construction_1429"]
_ROLE_LAYOUT = (("CORE", ("ZD", "ZG")), ("OUTER", ("DD", "GG")), ("INNER_EXTREMA", ("D", "G")))
# 角色 → (单元行里取 lo=2 / hi=3, 极值方向)；ZD/ZG 在 seed 三单元上取，其余在 Zn 上取（020:52）。
_ROLE_RULE = {"ZD": (2, max), "ZG": (3, min), "DD": (2, min), "GG": (3, max), "D": (2, max), "G": (3, min)}
_WAIT_REASONS = ("NO_LEFT_CENTER_IN_WINDOW", "SEED_NOT_YET_EVALUABLE", "UPGRADE_PENDING_TB02H", "UNIT_NOT_COMPLETED")


def _e(code, message):
    return ValueError("TB02E_" + code + ": " + message)


def _need(ok, code, message):
    if not ok:
        raise _e(code, message)


def _keys_e(value, expected, name):
    _need(type(value) is dict and set(value) == set(expected), "SHAPE", name + " 字段缺失/多余")


def _int_e(value, name, minimum=None):
    try:
        return integer(value, name, minimum)
    except ValueError as err:
        raise _e("SHAPE", str(err)) from None


def _text_e(value, name):
    _need(type(value) is str and value != "", "SHAPE", name + " 必须是非空文本")
    return value


def _unit_anchor(unit_id):
    _need(type(unit_id) is str and re.fullmatch(r"segment:[1-9][0-9]*:(?:0|[1-9][0-9]*)", unit_id) is not None, "IDENTITY", "单元身份须为 segment:<代>:<锚>")
    return unit_id[len("segment:"):]


def _known_e(k):
    _keys_e(k, ("generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"), "known_at")
    _int_e(k["generation"], "generation", 1)
    _int_e(k["input_frontier"], "input_frontier", -1)
    for name in ("receipt_id", "received_at"):
        if k[name] is not None:
            _text_e(k[name], name)
    if k["semantic_commit_ns"] is not None:
        _int_e(k["semantic_commit_ns"], "semantic_commit_ns", 0)


def _completed_unit(u, name):
    _keys_e(u, ("unit_id", "unit_object_id", "direction", "lo", "hi"), name)
    _unit_anchor(u["unit_id"])
    _text_e(u["unit_object_id"], name + ".unit_object_id")
    _need(u["direction"] in ("UP", "DOWN"), "SHAPE", name + ".direction 未声明")
    _need(_int_e(u["lo"], name + ".lo") < _int_e(u["hi"], name + ".hi"), "UNIT", name + " 区间须 lo<hi")


def _id_list(xs, name):
    _need(type(xs) is list and all(type(x) is str and x for x in xs), "SHAPE", name + " 须为文本数组")
    _need(len(xs) == len(set(xs)), "SHAPE", name + " 重复")
    return xs


def _witness_ok(contributors, witness):
    return witness == (contributors if len(contributors) >= 2 else [])


def validate_center_payload(kind, p):
    """单对象门：外层元组、严格 keys、身份与对象内一致性（schema v1 §3）。"""
    _keys_e(p, ("schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"), "中枢合同")
    _need((p["schema_revision"], p["semantic_version"], p["policy_version"], p["view_role"]) == CENTER_TUPLE, "VERSION", "中枢规则/范围版本不符")
    _int_e(p["object_generation"], "object_generation", 1)
    d = p["data"]
    if kind == CENTER_KINDS[0]:
        _keys_e(d, ("entity_id", "units", "vector", "ZD", "ZG", "core_order", "result", "exclusion_reason", "center_id", "known_at"), kind)
        units = d["units"]
        _need(type(units) is list and len(units) == 3, "SHAPE", "seed 须恰三单元")
        for u in units:
            _completed_unit(u, "seed.unit")
        _need(len({u["unit_id"] for u in units}) == 3, "SHAPE", "seed 单元重复")
        anchor = _unit_anchor(units[0]["unit_id"])
        _need(d["entity_id"] == "seed_attempt:" + anchor, "IDENTITY", "seed 尝试身份未绑定首单元")
        v = d["vector"]
        _keys_e(v, ("completed", "consecutive", "same_level", "alternating", "strict"), "seed.vector")
        _need(all(type(v[x]) is bool for x in ("completed", "consecutive", "same_level", "alternating")) and (v["strict"] is None or type(v["strict"]) is bool), "SHAPE", "Bool⁵ 错型")
        _need(d["exclusion_reason"] != "NOT_ALTERNATING" and v["alternating"], "NOT_ALTERNATING", "L0 段恒交替，NOT_ALTERNATING 一律拒")
        _need(v["completed"] and v["consecutive"] and v["same_level"], "VECTOR", "已发布的 seed 尝试须三单元完成、相连、同级")
        _need((v["strict"] is None) == (not v["completed"]), "VECTOR", "strict 仅在未完成时为 null")
        lows = [int(u["lo"]) for u in units]
        highs = [int(u["hi"]) for u in units]
        for name, want, attr in (("ZD", max(lows), "lo"), ("ZG", min(highs), "hi")):
            e = d[name]
            _keys_e(e, ("value", "contributors", "same_price_witness"), "seed." + name)
            _need(_int_e(e["value"], name) == want, "CORE_VALUE", name + " 值与 seed 三单元不符")
            _id_list(e["contributors"], name + ".contributors")
            _need(e["contributors"] == [u["unit_id"] for u in units if int(u[attr]) == want], "CONTRIBUTORS", name + " 贡献成员缺失或不符")
            _need(_witness_ok(e["contributors"], e["same_price_witness"]), "WITNESS", name + " 同价见证不符")
        zd, zg = int(d["ZD"]["value"]), int(d["ZG"]["value"])
        order = "ZD<ZG" if zd < zg else "ZD=ZG" if zd == zg else "ZD>ZG"
        _need(d["core_order"] == order, "CORE_ORDER", "核心序三叶与 ZD/ZG 不符")
        _need(v["strict"] == (order == "ZD<ZG"), "VECTOR", "strict 位与核心序不符")
        _need(d["result"] in ("FORMED", "EXCLUDED"), "SHAPE", "seed 结果未声明")
        formed = d["result"] == "FORMED"
        _need(formed == (v["strict"] is True), "RESULT", "FORMED 须五位全真")
        if formed:
            _need(d["exclusion_reason"] is None, "EXCLUSION_REASON", "成立的 seed 不得带排除原因")
            _need(d["center_id"] == "center:" + anchor, "IDENTITY", "成立的 seed 须指向同锚中枢")
        else:
            _need(d["exclusion_reason"] == ("TANGENT_ZD_EQ_ZG" if order == "ZD=ZG" else "INVERTED_ZD_GT_ZG"), "EXCLUSION_REASON", "相切/倒置缺原因或原因不符")
            _need(d["center_id"] is None, "RESULT", "不成立的 seed 不得指向中枢")
        _known_e(d["known_at"])
    elif kind == CENTER_KINDS[1]:
        _keys_e(d, ("entity_id", "entity_revision", "seed_attempt_id", "seed_units", "zn_direction", "members", "zn_members", "status", "settled", "break_unit", "break_up", "break_up_defined", "resume_anchor", "next_tested_unit", "roles_id", "formed_known_at", "settled_known_at"), kind)
        members = _id_list(d["members"], "members")
        _need(len(members) >= 3, "MEMBERS", "中枢至少 seed 三单元")
        for m in members:
            _unit_anchor(m)
        anchor = _unit_anchor(members[0])
        _need(d["entity_id"] == "center:" + anchor and d["seed_attempt_id"] == "seed_attempt:" + anchor and d["roles_id"] == "roles:" + d["entity_id"], "IDENTITY", "中枢身份未绑定 seed 首单元")
        _int_e(d["entity_revision"], "entity_revision", 1)
        _need(d["seed_units"] == members[:3], "MEMBERS", "members 须以 seed 三单元开头")
        _need(d["break_unit"] is None or d["break_unit"] not in members, "CONNECTOR_AS_MEMBER", "突破段不得是成员")
        zn = _id_list(d["zn_members"], "zn_members")
        _need(zn[:1] == members[:1] and all(z in members for z in zn), "ZN", "Zn 成员须含首单元且属成员")
        _need(d["zn_direction"] in ("UP", "DOWN"), "SHAPE", "zn_direction 未声明")
        _need(d["status"] in ("EXTENDING", "SETTLED", "UPGRADE_PENDING_TB02H"), "SHAPE", "中枢状态未声明")
        _need((d["status"] == "UPGRADE_PENDING_TB02H") == (len(members) >= 9), "UPGRADE", "≥9 成员须且只须标 UPGRADE_PENDING_TB02H")
        _need(type(d["settled"]) is bool and type(d["break_up"]) is bool and type(d["break_up_defined"]) is bool, "SHAPE", "结算位错型")
        settled = d["settled"]
        if d["status"] != "UPGRADE_PENDING_TB02H":
            _need((d["status"] == "SETTLED") == settled, "SETTLEMENT", "状态与 settled 不符")
        _need((d["break_unit"] is not None) == settled and (d["resume_anchor"] is not None) == settled and (d["settled_known_at"] is not None) == settled, "SETTLEMENT", "结算位与突破段/续扫锚/结算时点不一致")
        _need(d["break_up_defined"] == settled, "BREAK_UP_UNDEFINED", "未结算中枢 break_up_defined 须为 false（R4）")
        if not settled:
            _need(d["break_up"] is False, "BREAK_UP_UNDEFINED", "未结算中枢 break_up 须为 false（R4）")
            if d["next_tested_unit"] is not None:
                _unit_anchor(d["next_tested_unit"])
                _need(d["next_tested_unit"] not in members, "NEXT_TESTED", "待测单元不得是成员")
        else:
            _unit_anchor(d["break_unit"])
            _need(d["next_tested_unit"] is None, "NEXT_TESTED", "结算后不再有待测延伸单元")
            ra = d["resume_anchor"]
            _keys_e(ra, ("after_unit_id", "offset"), "resume_anchor")
            _need(ra["after_unit_id"] == members[-1] and ra["offset"] == "2", "RESUME_ANCHOR", "续扫锚须为 {末成员, 2}（R8）")
            _known_e(d["settled_known_at"])
        _known_e(d["formed_known_at"])
    elif kind == CENTER_KINDS[2]:
        _keys_e(d, ("entity_id", "entity_revision", "center_id", "roles", "previous", "known_at"), kind)
        _need(type(d["center_id"]) is str and d["center_id"].startswith("center:") and d["entity_id"] == "roles:" + d["center_id"], "IDENTITY", "角色身份未绑定中枢")
        revision = _int_e(d["entity_revision"], "entity_revision", 1)
        _validate_roles(d["roles"])
        if d["previous"] is not None:
            _keys_e(d["previous"], ("entity_revision", "roles"), "previous")
            _need(_int_e(d["previous"]["entity_revision"], "previous.entity_revision", 1) < revision, "SHAPE", "previous 修订号须小于当前")
            _validate_roles(d["previous"]["roles"])
        _known_e(d["known_at"])
    elif kind == CENTER_KINDS[3]:
        _keys_e(d, ("entity_id", "unit", "status", "center_id", "left_center_id", "right_center_id", "right_pending", "reason", "seed_from_self", "known_at"), kind)
        u = d["unit"]
        completed = not (type(u) is dict and "completed" in u)
        if completed:
            _completed_unit(u, "assignment.unit")
        else:
            _keys_e(u, ("unit_id", "unit_object_id", "completed"), "assignment.unit")
            _unit_anchor(u["unit_id"])
            _text_e(u["unit_object_id"], "unit_object_id")
            _need(u["completed"] is False, "SHAPE", "进行中单元 completed 须为 false")
        _need(d["entity_id"] == "assign:" + u["unit_id"], "IDENTITY", "归属身份未绑定单元")
        _need(d["seed_from_self"] in ("FORMED", "FAILED", "NOT_EVALUATED"), "SHAPE", "seed_from_self 未声明")
        _need(type(d["right_pending"]) is bool, "SHAPE", "right_pending 错型")
        status = d["status"]
        if status == "CENTER_MEMBER":
            ok = d["center_id"] is not None and d["left_center_id"] is None and d["right_center_id"] is None and not d["right_pending"] and d["reason"] is None
        elif status == "CONNECTOR":
            ok = d["center_id"] is None and d["left_center_id"] is not None and (d["right_center_id"] is not None) != d["right_pending"] and d["reason"] is None
        elif status == "NOT_YET_ASSIGNED":
            ok = d["reason"] in _WAIT_REASONS and d["center_id"] is None and d["left_center_id"] is None and d["right_center_id"] is None and not d["right_pending"]
        else:
            ok = False
        _need(ok, "ASSIGNMENT", "归属三值须恰一且字段与状态一致")
        _need(completed != (d["reason"] == "UNIT_NOT_COMPLETED"), "ASSIGNMENT", "UNIT_NOT_COMPLETED 当且仅当单元进行中")
        if not completed:
            _need(d["seed_from_self"] == "NOT_EVALUATED", "ASSIGNMENT", "进行中单元未被评估过 seed")
        _known_e(d["known_at"])
    elif kind == CENTER_KINDS[4]:
        _keys_e(d, ("source_id", "relation_kind", "target_id", "version", "witness_object_id"), kind)
        for name in ("source_id", "target_id", "witness_object_id"):
            _text_e(d[name], name)
        _need(d["relation_kind"] in ("member_of", "connects_left", "connects_right", "attempted_from"), "SHAPE", "未声明中枢关系")
        _int_e(d["version"], "version", 1)
    elif kind == CENTER_KINDS[5]:
        _keys_e(d, ("entity_id", "change", "before", "after", "known_at", "object_id"), kind)
        _text_e(d["entity_id"], "entity_id")
        _need(d["change"] in ("formed", "member_added", "roles_updated", "settled", "upgrade_pending", "withdrawn"), "SHAPE", "未声明中枢变化")
        withdrawn = d["change"] == "withdrawn"
        _need((d["after"] is None) == withdrawn and (d["object_id"] is None) == withdrawn and (d["before"] is None) == (d["change"] == "formed"), "SHAPE", "变化前后件与变化种类不符")
        if d["object_id"] is not None:
            _text_e(d["object_id"], "object_id")
        _known_e(d["known_at"])
    elif kind == CENTER_KINDS[6]:
        _keys_e(d, ("input_frontier", "known_at", "scan_cursor", "waiting_reasons", "delivered_scope", "deferred_scope"), kind)
        _int_e(d["input_frontier"], "input_frontier", -1)
        _known_e(d["known_at"])
        _need(type(d["waiting_reasons"]) is list and all(r in _WAIT_REASONS for r in d["waiting_reasons"]), "SHAPE", "等待原因未声明")
        _need(d["delivered_scope"] == "l0_strict_seed_zn_roles_c_resume_assignment" and d["deferred_scope"] == TB02E_DEFERRED, "SCOPE", "中枢交付范围不符")
        c = d["scan_cursor"]
        _keys_e(c, ("state", "center_id", "seed_start_unit_id", "awaiting_unit_id", "awaiting_after_unit_id", "awaiting_offset"), "scan_cursor")
        if c["state"] == "EXTENDING":
            _need(type(c["center_id"]) is str and c["center_id"].startswith("center:") and c["seed_start_unit_id"] is None, "SCAN_CURSOR", "EXTENDING 须指中枢且无 seed 起点")
        else:
            _need(c["state"] == "SEEKING_SEED" and c["center_id"] is None, "SCAN_CURSOR", "游标状态未声明")
            if c["seed_start_unit_id"] is not None:
                _unit_anchor(c["seed_start_unit_id"])
        if c["awaiting_unit_id"] is not None:
            _unit_anchor(c["awaiting_unit_id"])
            _need(c["awaiting_after_unit_id"] is None and c["awaiting_offset"] is None, "SCAN_CURSOR", "等待单元与实体锚恰一组")
        else:
            _need(_int_e(c["awaiting_offset"], "awaiting_offset", 1) >= 1, "SCAN_CURSOR", "awaiting_offset 须 ≥1")
            if c["awaiting_after_unit_id"] is not None:
                _unit_anchor(c["awaiting_after_unit_id"])
    else:
        raise _e("CANDIDATE_KIND", "未声明的中枢 kind（不发 candidate）")
    slot = ([d["source_id"], d["relation_kind"], d["target_id"]] if kind == CENTER_KINDS[4]
            else [d["known_at"]["generation"], d["entity_id"], d["change"]] if kind == CENTER_KINDS[5]
            else "TB-02-E" if kind == CENTER_KINDS[6] else d["entity_id"])
    _need(p["slot"] == slot, "SLOT", "中枢语义槽身份不符")


def _validate_roles(roles):
    _keys_e(roles, [r for r, _ in _ROLE_LAYOUT], "roles")
    for role, names in _ROLE_LAYOUT:
        _need(type(roles[role]) is dict and set(roles[role]) == set(names), "ROLES_SHAPE", role + " 角色缺变量或混入他角色变量（D/G 不得冒充核心）")
        for name in names:
            v = roles[role][name]
            _keys_e(v, ("value", "contributors", "same_price_witness", "frozen", "null_reason"), name)
            _need(v["frozen"] is (role == "CORE"), "ROLES_SHAPE", "只有核心冻结")
            contributors = _id_list(v["contributors"], name + ".contributors")
            if v["value"] is None:
                _need(role != "CORE" and v["null_reason"] == "UPGRADE_PENDING_TB02H" and not contributors, "ROLE_NULL", "只有 ≥9 的动态变量可为 null 且须带原因")
            else:
                _int_e(v["value"], name)
                _need(v["null_reason"] is None, "ROLE_NULL", "有值变量不得带 null 原因")
                _need(len(contributors) >= 1, "CONTRIBUTORS", name + " 缺贡献成员")
            _need(_witness_ok(contributors, v["same_price_witness"]), "WITNESS", name + " 同价见证不符")
    val = {name: roles[role][name]["value"] for role, names in _ROLE_LAYOUT for name in names}
    _need(int(val["ZD"]) < int(val["ZG"]), "ROLE_ORDER", "核心须 ZD<ZG")
    if val["DD"] is not None and val["GG"] is not None:
        _need(int(val["DD"]) <= int(val["ZD"]) and int(val["ZG"]) <= int(val["GG"]), "ROLE_ORDER", "须 DD≤ZD<ZG≤GG")
    _need((val["DD"] is None) == (val["GG"] is None) == (val["D"] is None) == (val["G"] is None), "ROLE_NULL", "动态四变量须同时有值或同时置 null")


def validate_center_rule(records, rule_revision):
    """规则修订与中枢族互斥：中枢对象只在中枢规则 cut 出现；跨版本混用拒绝。"""
    center_rule = rule_revision == CENTER_RULE
    for r in records:
        if r["kind"] in CENTER_KINDS or r["kind"].startswith(("CC-016.", "CC-017.", "CC-022.")):
            _need(center_rule, "RULE_MIX", "s-center/1 对象出现在旧规则 cut")
            _need(r["payload"].get("schema_revision") == "s-center/1", "RULE_MIX", "中枢 kind 混用旧 tuple")
        elif r["kind"] in SEGMENT_KINDS:
            _need(not center_rule or r["payload"].get("schema_revision") == "s-segment/2", "RULE_MIX", "中枢规则 cut 的段对象须为 s-segment/2")


def _member_extremes(d):
    """完成段成员笔的方向极值（schema v1.3 §1，根裁定 ROOT-RULING-1419-L3）：stroke_refs 中下标属于
    construction.stroke_indices 的笔，取全部笔端价的 [min, max]；不取几何端点，也不取全部 stroke_refs（会越过段端）。"""
    refs = {r["stroke_index"]: r for r in d["stroke_refs"]}
    members = d["construction"]["stroke_indices"]
    _need(len(members) > 0 and all(m in refs for m in members), "UNIT_SOURCE", "完成段成员笔不在同对象 stroke_refs 中")
    prices = [int(refs[m][k]) for m in members for k in ("start_price", "end_price")]
    return min(prices), max(prices)


def _segment_units(records):
    """同 cut CC-013.segment → (完成单元序 [(unit_id, object_id, lo, hi, direction)], 进行中 unit_id 集)。"""
    rows = []
    for r in records:
        if r["kind"] != "CC-013.segment":
            continue
        d = r["payload"]["data"]
        done = d["result"] in ("CASE_ONE", "CASE_TWO")
        start = int(d["construction"]["start_stroke"])
        if done:
            lo, hi = _member_extremes(d)
            rows.append((start, d["entity_id"], r["object_id"], lo, hi, d["construction"]["direction"], True))
        else:
            rows.append((start, d["entity_id"], r["object_id"], None, None, None, False))
    rows.sort()
    completed = [row[1:6] for row in rows if row[6]]
    pending = {row[1]: row[2] for row in rows if not row[6]}
    return completed, pending


def _validate_scan_cursor(versions, completed, pending, pos, centers, attempts):
    """游标与中枢/尝试/单元一致（schema v1.1 §3 位置约定、§6 游标条目；根裁定 D-J2 第 1 点）。"""
    _need(len(versions) == 1, "SCAN_CURSOR", "每 cut 恰一个中枢版本对象")
    cur = versions[0]["payload"]["data"]["scan_cursor"]
    n = len(completed)
    _need(len(pending) <= 1, "SCAN_CURSOR", "至多一个进行中单元")
    ip = next(iter(pending), None)

    def unit_at(k):
        return completed[k][0] if 0 <= k < n else ip if k == n else None

    last = centers[-1] if centers else None
    if last is not None and not last["settled"]:
        _need(cur["state"] == "EXTENDING" and cur["center_id"] == last["entity_id"] and cur["seed_start_unit_id"] is None, "SCAN_CURSOR", "末中枢开放时游标须 EXTENDING 指末中枢")
        wait = pos[last["members"][-1]] + 1
    else:
        s = pos[last["members"][-1]] + 2 if last is not None else 0
        while s < n and attempts.get("seed_attempt:" + _unit_anchor(completed[s][0]), {}).get("result") == "EXCLUDED":
            s += 1
        _need(cur["state"] == "SEEKING_SEED" and cur["center_id"] is None and cur["seed_start_unit_id"] == unit_at(s), "SCAN_CURSOR", "SEEKING_SEED 起点须为续扫锚经 EXCLUDED 尝试推进后的位置")
        wait = s + 2
    if wait == n and ip is not None:
        want = (ip, None, None)
    else:
        _need(wait > n - 1, "SCAN_CURSOR", "等待位置不得落在已完成单元上")
        want = (None, completed[-1][0] if n else None, str(wait - (n - 1)))
    _need((cur["awaiting_unit_id"], cur["awaiting_after_unit_id"], cur["awaiting_offset"]) == want, "SCAN_CURSOR", "等待位置的实体锚不符")
    if last is not None and not last["settled"]:
        _need(last["next_tested_unit"] == want[0], "SCAN_CURSOR", "游标与中枢 next_tested_unit 不一致")


def validate_center_links(records, axes=None):
    """跨对象门（schema v1 §1/§3/§6）：单元来源、成员连续、续扫锚、连接段、归属恰一、角色取值与贡献、目录轴绑定。"""
    by_kind = {k: [r for r in records if r["kind"] == k] for k in CENTER_KINDS}
    if not any(by_kind.values()):
        return
    for r in records:
        if r["kind"].startswith(("CC-016.", "CC-017.", "CC-022.")) and r["kind"] not in CENTER_KINDS:
            raise _e("CANDIDATE_KIND", "未声明的中枢 kind（不发 candidate）")
    completed, pending = _segment_units(records)
    pos = {u[0]: i for i, u in enumerate(completed)}
    seg = {u[0]: u for u in completed}

    def check_unit(u):
        if "completed" in u:
            _need(pending.get(u["unit_id"]) == u["unit_object_id"], "UNIT_SOURCE", "进行中单元未绑定同 cut 段对象")
            return
        _need(u["unit_id"] in seg, "MEMBER_NOT_COMPLETED", "单元不是同 cut 已完成段")
        s = seg[u["unit_id"]]
        _need((u["unit_object_id"], int(u["lo"]), int(u["hi"]), u["direction"]) == (s[1], s[2], s[3], s[4]), "UNIT_SOURCE", "单元区间/方向/对象与同 cut CC-013 不符")

    attempts = {r["payload"]["data"]["entity_id"]: r["payload"]["data"] for r in by_kind[CENTER_KINDS[0]]}
    for a in attempts.values():
        for u in a["units"]:
            check_unit(u)
        ids = [u["unit_id"] for u in a["units"]]
        _need([pos[i] for i in ids] == list(range(pos[ids[0]], pos[ids[0]] + 3)), "SEED_LINK", "seed 三单元须在完成单元序中相连")
    roles = {r["payload"]["data"]["center_id"]: r["payload"]["data"] for r in by_kind[CENTER_KINDS[2]]}
    centers = sorted((r["payload"]["data"] for r in by_kind[CENTER_KINDS[1]]), key=lambda c: pos.get(c["members"][0], -1))
    assignments = {}
    for r in by_kind[CENTER_KINDS[3]]:
        d = r["payload"]["data"]
        check_unit(d["unit"])
        _need(d["unit"]["unit_id"] not in assignments, "ASSIGNMENT_CONSISTENCY", "单元归属重复")
        assignments[d["unit"]["unit_id"]] = d
    _need(set(assignments) == set(seg) | set(pending), "ASSIGNMENT_CONSISTENCY", "每个单元须恰一归属")
    member_of = {}
    for c in centers:
        for m in c["members"]:
            _need(m in seg, "MEMBER_NOT_COMPLETED", "成员不是同 cut 已完成段")
        first = pos[c["members"][0]]
        _need([pos[m] for m in c["members"]] == list(range(first, first + len(c["members"]))), "MEMBERS", "成员须在完成单元序中连续")
        seed = attempts.get(c["seed_attempt_id"])
        _need(seed is not None and seed["result"] == "FORMED" and [u["unit_id"] for u in seed["units"]] == c["seed_units"] and seed["center_id"] == c["entity_id"], "SEED_LINK", "中枢须由同 cut 成立的 seed 尝试支撑")
        upgrade = c["status"] == "UPGRADE_PENDING_TB02H"
        for m in c["members"]:
            a = assignments.get(m)
            want = ("NOT_YET_ASSIGNED", "UPGRADE_PENDING_TB02H") if upgrade else ("CENTER_MEMBER", None)
            if a is not None and a["status"] == "CONNECTOR":
                raise _e("CONNECTOR_AS_MEMBER", "连接段被列为中枢成员")
            _need(a is not None and (a["status"], a["reason"]) == want and (upgrade or a["center_id"] == c["entity_id"]), "ASSIGNMENT_CONSISTENCY", "成员归属与中枢成员表不一致")
            member_of[m] = c["entity_id"]
        units = [seg[m] for m in c["members"]]
        zn = [m for m, u in zip(c["members"], units) if u[4] == units[0][4]]
        _need(c["zn_members"] == zn and c["zn_direction"] == units[0][4], "ZN", "Zn 成员须是与首单元同向的全部成员")
        if c["settled"]:
            nxt = first + len(c["members"])
            _need(nxt < len(completed) and completed[nxt][0] == c["break_unit"], "BREAK_UNIT", "突破段须是末成员后的下一个完成单元")
            seed_units = [seg[m] for m in c["members"][:3]]
            zd, zg = max(u[2] for u in seed_units), min(u[3] for u in seed_units)
            b = completed[nxt]
            up = True if b[2] > zg else False if b[3] < zd else b[3] > zg
            _need(c["break_up"] is up, "BREAK_UP", "break_up 与突破段方向不符（C4）")
        rd = roles.get(c["entity_id"])
        _need(rd is not None and rd["entity_id"] == c["roles_id"], "ROLES_LINK", "中枢缺同 cut 角色对象")
        allowed = {"ZD": c["members"][:3], "ZG": c["members"][:3], "DD": zn, "GG": zn, "D": zn, "G": zn}
        for role, names in _ROLE_LAYOUT:
            for name in names:
                v = rd["roles"][role][name]
                if v["value"] is None:
                    _need(upgrade, "ROLE_NULL", "非 ≥9 中枢的动态变量不得为 null")
                    continue
                attr, pick = _ROLE_RULE[name]
                values = [seg[m][attr] for m in allowed[name]]
                want = pick(values)
                _need(int(v["value"]) == want, "ROLE_VALUE", name + " 值不是允许成员集上的极值（D/G 不得冒充核心）")
                expected = [m for m in allowed[name] if seg[m][attr] == want]
                _need(v["contributors"] == expected, "CONTRIBUTORS", name + " 贡献成员缺失或不在允许集")
    for a, b in zip(centers, centers[1:]):
        a_last, b_first = pos[a["members"][-1]], pos[b["members"][0]]
        _need(b_first != a_last + 1, "PLUS_ONE_REUSE", "下一中枢首成员是上一中枢末成员的下一个单元（+1 复用）")
        _need(b_first > a_last + 1 and a["settled"], "RESUME_ANCHOR", "下一中枢首成员早于续扫锚或上一中枢未结算")
        for k in range(a_last + 1, b_first):
            x = assignments[completed[k][0]]
            _need((x["status"], x["left_center_id"], x["right_center_id"]) == ("CONNECTOR", a["entity_id"], b["entity_id"]), "CONNECTOR", "两中枢之间的单元须全是 CONNECTOR(A→B)")
        for k in range(a_last + 2, b_first):
            t = attempts.get("seed_attempt:" + _unit_anchor(completed[k][0]))
            _need(t is not None and t["result"] == "EXCLUDED", "RESUME_GAP", "续扫锚到下一中枢之间的单元须有不成立的 seed 尝试")
    upgrade_members = {m for c in centers if c["status"] == "UPGRADE_PENDING_TB02H" for m in c["members"]}
    for uid, a in assignments.items():
        if a["status"] == "CENTER_MEMBER":
            _need(member_of.get(uid) == a["center_id"] and uid not in upgrade_members, "ASSIGNMENT_CONSISTENCY", "CENTER_MEMBER 指向的中枢不含该单元")
        if a["status"] == "CONNECTOR":
            _need(uid not in member_of, "CONNECTOR_AS_MEMBER", "连接段被列为中枢成员")
        if a["reason"] == "UPGRADE_PENDING_TB02H":
            _need(uid in upgrade_members, "ASSIGNMENT_CONSISTENCY", "UPGRADE_PENDING_TB02H 归属的单元不属任何 UPGRADE 中枢")
    by_center = {c["entity_id"]: c for c in centers}
    edges = {}
    for r in by_kind[CENTER_KINDS[4]]:
        d = r["payload"]["data"]
        if d["relation_kind"] == "member_of":
            target = by_center.get(d["target_id"])
            _need(target is not None and target["status"] != "UPGRADE_PENDING_TB02H" and d["source_id"] in target["members"], "MEMBER_EDGE", "member_of 只能从非 UPGRADE 中枢的成员指向该中枢")
            edges[(d["source_id"], d["target_id"])] = edges.get((d["source_id"], d["target_id"]), 0) + 1
    # schema v1.2 §6 E4：非 UPGRADE 中枢的每个成员恰有一条指向该中枢的 member_of 边（缺边、重边都拒）。
    for c in centers:
        if c["status"] != "UPGRADE_PENDING_TB02H":
            for m in c["members"]:
                _need(edges.get((m, c["entity_id"]), 0) == 1, "MEMBER_EDGE_COUNT", "非 UPGRADE 中枢的每个成员须恰有一条指向该中枢的 member_of 边（E4）")
    _validate_scan_cursor(by_kind[CENTER_KINDS[6]], completed, pending, pos, centers, attempts)
    if axes is not None and "CC-016" in axes:
        ids = lambda *kinds: sorted(r["object_id"] for r in records if r["kind"] in kinds)
        for axis, kinds in (("CC-016", CENTER_KINDS[:2]), ("CC-017", CENTER_KINDS[2:3]), ("CC-022", CENTER_KINDS[3:4])):
            _need(sorted(axes[axis]["evidence"]["object_ids"]) == ids(*kinds), "AXIS_BINDING", axis + " 目录证据未绑定实际中枢对象")


def validate_center_cut(records, rule_revision, axes=None):
    """语料与离线核对用的整 cut 门：candidate 类 kind → 单对象 → 规则修订 → 跨对象，顺序与 JS 端一致。"""
    for r in records:
        if r["kind"].startswith(("CC-016.", "CC-017.", "CC-022.")) and r["kind"] not in CENTER_KINDS:
            raise _e("CANDIDATE_KIND", "未声明的中枢 kind（不发 candidate）")
    for r in records:
        if r["kind"] in CENTER_KINDS:
            validate_payload(r["kind"], r["payload"])
    validate_center_rule(records, rule_revision)
    validate_center_links(records, axes)


def expand_center_corpus(corpus):
    """把语料差分展开为 (name, expected, rule_revision, records, axes)。"""
    bases = {c["name"]: c for c in corpus["cases"] if c["base"] is None}
    for case in corpus["cases"]:
        if case["base"] is None:
            yield case["name"], case["expected"], case["rule_revision"], case["records"], case["axes"]
            continue
        base = bases[case["base"]]
        records, seen = [], set()
        for r in base["records"]:
            oid = r["object_id"]
            if oid in case["remove"]:
                continue
            records.append(case["replace"][oid] if oid in case["replace"] and oid not in seen else r)
            seen.add(oid)
        records.extend(case["add"])
        yield case["name"], case["expected"], case["rule_revision"], records, case["axes"] if case["axes"] is not None else base["axes"]
