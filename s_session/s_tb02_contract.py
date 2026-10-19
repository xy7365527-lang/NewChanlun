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
# #1405：TB-02-A 知识域两代——旧值只在旧规则会话里 AsKnown 可读；#1343 后极值身份以根集合计。
KNOWLEDGE_DOMAIN_V1 = "established_direction_without_extreme_identity_competition"
KNOWLEDGE_DOMAIN_V2 = "established_direction_with_extreme_root_sets"
# #1405：会话级 rule_revision 绑定（新笔合同, 段合同, TB-02-A 知识域）代际；旧规则会话只读（AsKnown）。
RULE_CONTRACTS = {
    "s2-axis-quantifiers": (None, None, KNOWLEDGE_DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/1": ("s-new-bi/1", None, KNOWLEDGE_DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/1+segment-first/1": ("s-new-bi/1", "s-segment/1", KNOWLEDGE_DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/1+segment-second/1": ("s-new-bi/1", "s-segment/2", KNOWLEDGE_DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/2+segment-second/1": ("s-new-bi/2", "s-segment/2", KNOWLEDGE_DOMAIN_V2),
}
# #1393：旧 cut 继续按旧 schema 读取；W 字段必须与该 cut 的规则版本一致。
INITIALIZATION_POLICY = "raw-initialization-w/1"
INITIALIZATION_KINDS = ("CC-004.initialization", "CC-004.undetermined_raw")
KINDS += INITIALIZATION_KINDS
# #1393 W 单域知识域；W 规则 cut 的 TB-02-A 知识状态逐字取 W 冻结源。
W_KNOWLEDGE_DOMAIN = "raw_adjacent_W_initialization"
# #1393×#1405 整合：完整规则串精确白名单——无后缀者即自身基规则；带 W 后缀者显式映射，不剥后缀推断。
RULE_BASES = {rule: rule for rule in RULE_CONTRACTS}
RULE_BASES["s2-axis-quantifiers+new-bi/1+segment-second/1+" + INITIALIZATION_POLICY] = (
    "s2-axis-quantifiers+new-bi/1+segment-second/1"
)


def base_rule(rule_revision):
    if type(rule_revision) is not str or rule_revision not in RULE_BASES:
        raise ValueError("未获准的完整规则组合")
    return RULE_BASES[rule_revision]


def initialization_policy(payload):
    if payload["initialization_policy_revision"] != INITIALIZATION_POLICY:
        raise ValueError("初始化政策版本未声明")


def direction_state(payload):
    if payload["direction_state"] not in ("Undetermined", "Established"):
        raise ValueError("初始化方向状态未声明")


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
    elif kind in BI_KINDS:
        validate_bi_payload(kind, p)
    elif kind == "CC-004.initialization":
        keys(p, ("initialization_policy_revision", "direction_state", "first_raw_pair"), kind)
        initialization_policy(p)
        direction_state(p)
        direction_evidence(p["first_raw_pair"])
        if (p["direction_state"] == "Undetermined") != (p["first_raw_pair"] is None):
            raise ValueError("初始化状态与首对证据不一致")
    elif kind == "CC-004.undetermined_raw":
        keys(p, ("raw", "direction_state", "initialization_policy_revision", "pending_since_cut"), kind)
        initialization_policy(p)
        bar(p["raw"], "undetermined.raw")
        if p["direction_state"] != "Undetermined":
            raise ValueError("未定原件携伪方向")
        if not re.fullmatch(r"cut-[1-9][0-9]*", text(p["pending_since_cut"], "pending_since_cut")):
            raise ValueError("未定记录发生 cut 非法")
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
        extra = ("initialization_policy_revision", "initial_direction_evidence") if "initialization_policy_revision" in p else ()
        keys(p, ("group_index", "group_anchor", "members", "open", "high", "low", "close", "high_sources", "low_sources", "confirmed", "direction", "direction_evidence", "confirmation_evidence", "waiting_reasons") + extra, kind)
        if extra:
            initialization_policy(p)
            direction_evidence(p["initial_direction_evidence"])
            if p["initial_direction_evidence"] is None:
                raise ValueError("W 正式组缺初始化证据")
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
        extra = ("initialization_policy_revision", "direction_state") if "initialization_policy_revision" in p else ()
        keys(p, ("scope", "input_frontier", "known_facts", "waiting_reasons", "domain", "requests") + extra, kind)
        if extra:
            initialization_policy(p)
            direction_state(p)
        if p["scope"] != "TB-02-A":
            raise ValueError("knowledge scope 不符")
        integer(p["input_frontier"], "knowledge.input_frontier", -1)
        strings(p["waiting_reasons"], "knowledge.waiting_reasons")
        if p["known_facts"] != ["raw_ohlc", "inclusion_trace", "group_provenance"] or p["domain"] not in ((W_KNOWLEDGE_DOMAIN,) if extra else (KNOWLEDGE_DOMAIN_V1, KNOWLEDGE_DOMAIN_V2)):
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
    rule = base_rule(meta["rule_revision"]) if meta is not None else None
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
    slot = p["slot"] if kind in BI_KINDS + SEGMENT_KINDS else p["incoming"]["source_coord"] if kind == KINDS[0] else p["group_anchor"] if kind == KINDS[1] else p["window"] if kind == KINDS[2] else "TB-02-A"
    if kind == "CC-004.initialization":
        slot = "W"
    elif kind == "CC-004.undetermined_raw":
        slot = p["raw"]["source_coord"]
        if p["pending_since_cut"] != value["first_known_cut"]:
            raise ValueError("未定发生 cut 与首次获知不同")
    if value["fact_key"] != canonical([kind, slot]).decode():
        raise ValueError("fact_key 与真实语义槽不符")
    if meta is not None:
        w_policy = meta["rule_revision"] != rule
        if kind in INITIALIZATION_KINDS and not w_policy:
            raise ValueError("旧规则 cut 混入 W 初始化对象")
        if kind in (KINDS[1], KINDS[3]) and w_policy != ("initialization_policy_revision" in p):
            raise ValueError("初始化字段与 cut 规则版本不符")
        bi_schema, segment_schema, domain = RULE_CONTRACTS[rule]
        if w_policy:
            domain = W_KNOWLEDGE_DOMAIN
        if kind in SEGMENT_KINDS and p["schema_revision"] != segment_schema:
            raise ValueError("段合同与cut规则版本不符")
        if kind in BI_KINDS and p["schema_revision"] != bi_schema:
            raise ValueError("新笔合同与cut规则版本不符")
        if kind == KINDS[3] and domain is not None and p["domain"] != domain:
            raise ValueError("TB-02-A 知识域与cut规则版本不符")
        identity = {key: value[key] for key in ("kind", "fact_key", "payload", "input_refs")}
        identity.update(profile_id=meta["profile_id"], rule_revision=meta["rule_revision"])
        digest = (hashlib.sha256(canonical(identity)).hexdigest() if _identity_hash is None
                  else _identity_hash(identity))
        if value["object_id"] != "obj-" + digest:
            raise ValueError("typed object 内容身份 hash 不闭合")


def validate_sources(value, effective, *, raw=None):
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
    if kind == "CC-004.undetermined_raw":
        raw_bars = [p["raw"]]
    elif kind == "CC-004.initialization":
        evidence = p["first_raw_pair"]
        raw_bars = [] if evidence is None else [evidence["previous_acc"], evidence["incoming"]]
    elif kind == KINDS[0]:
        raw_bars = [p["incoming"]]
        if p["direction_evidence"] is not None:
            raw_bars.append(p["direction_evidence"]["incoming"])
    elif kind == KINDS[1]:
        raw_bars = [p[key]["incoming"] for key in ("direction_evidence", "confirmation_evidence") if p[key] is not None]
        if "initial_direction_evidence" in p:
            evidence = p["initial_direction_evidence"]
            raw_bars.extend((evidence["previous_acc"], evidence["incoming"]))
    elif kind == KINDS[2]:
        raw_bars = p["descriptive_labels"]["raw_ohlc"] + [item["bar"] for item in p["subsequent_development"]["bars"]]
    else:
        raw_bars = []
    if raw is not None:
        for item in raw_bars:
            ref = by_source[item["source_coord"]]
            seq = int(ref["seq"])
            if seq >= len(raw) or any(ref[key] != raw[seq][key] for key in REF_FIELDS):
                raise ValueError("typed input_ref 与完整已接纳账不一致")
            if any(item[key] != raw[seq][key] for key in ("open", "high", "low", "close")):
                raise ValueError("事实中原始 OHLC 值与真实源记录不符")
    return by_source, raw_bars


def validate_initialization_links(records, rule_revision):
    """#1393：核同 cut 的初始化原件与引用一致性，不重算 W 几何。"""
    if base_rule(rule_revision) == rule_revision:
        return
    initial = [r for r in records if r["kind"] == "CC-004.initialization"]
    if len(initial) != 1:
        raise ValueError("同 cut 缺少唯一初始化对象")
    initial = initial[0]
    p = initial["payload"]
    initial_refs = {ref["source_coord"]: ref for ref in initial["input_refs"]}
    for record in records:
        kind, payload = record["kind"], record["payload"] if "payload" in record else None
        if kind == KINDS[1]:
            if (p["direction_state"] != "Established"
                    or payload.get("initial_direction_evidence") != p["first_raw_pair"]):
                raise ValueError("组初始化证据与同 cut 初始化对象不一致")
            refs = {ref["source_coord"]: ref for ref in record["input_refs"]}
            if any(refs.get(coord) != ref for coord, ref in initial_refs.items()):
                raise ValueError("组初始化引用与同 cut 原始修订不同")
        elif kind == "CC-004.undetermined_raw":
            if p["direction_state"] != "Undetermined":
                raise ValueError("已定初始化混入未定原件")
            if any(initial_refs.get(ref["source_coord"]) != ref for ref in record["input_refs"]):
                raise ValueError("未定原件引用与同 cut 初始化不同")
        elif kind == KINDS[3] and payload["direction_state"] != p["direction_state"]:
            raise ValueError("知识方向状态与同 cut 初始化不同")


def validate_axes(axes, object_ids=None):
    keys(axes, AXES if "CC-014" in axes else FIRST_SEGMENT_AXES if "CC-011" in axes else BI_AXES if "CC-008" in axes else LEGACY_AXES, "catalog.axes")
    for cid, value in axes.items():
        keys(value, ("impl_status", "proof_status", "run_status", "evidence"), cid)
        if value["impl_status"] not in ("implemented", "not_implemented") or value["proof_status"] != "not_proved" or value["run_status"] not in ("run", "waiting", "not_run"):
            raise ValueError("逐轴实现/证明/运行状态不符")
        evidence = value["evidence"]
        base = ("scope", "object_ids", "waiting_reasons", "raw_revisions", "withdrawals", "replaces") if cid == "CC-056" else ("scope", "object_ids", "waiting_reasons")
        scoped = evidence.get("scope") == "TB-02-D"
        keys(evidence, base + (("delivered_subobligations", "deferred_subobligations") if scoped else ()), cid + ".evidence")
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
        if evidence["scope"] != ("TB-02-D" if "CC-014" in axes and cid in ("CC-011", "CC-012", "CC-013", "CC-014") else "TB-02-C" if cid in ("CC-011", "CC-012", "CC-013") else "TB-02-A"):
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


# #1405（#1343）：新笔合同两代并存——`s-new-bi/1` 旧对象按 AsKnown 保留、不回填；
# `s-new-bi/2` 端点带极值根集首末 raw 序位，同型只有 KEEP/REPLACE 并带同价证据，
# `policy_version` 并列记录 raw 间隔门与同价策略。
SAME_PRICE_POLICY = "same-price/keep-earlier+extreme-region/1"
SAME_PRICE_KEEP_EVIDENCE = "same_kind_equal_price_keep_earlier"
BI_VERSIONS = {
    "s-new-bi/1": ("new-bi-dual-coordinate/1", "standard_raw_gap_3"),
    "s-new-bi/2": ("new-bi-dual-coordinate/2", "standard_raw_gap_3;" + SAME_PRICE_POLICY),
}
# #1343 Q4：同价域已无任何未定态；v2 对象再出现这些等待原因即合同破坏。
RETIRED_SAME_PRICE_WAITING = frozenset(("raw_extreme_identity_tie", "actual_extreme_mapping_missing",
                                        "same_kind_endpoint_identity_tie", "earlier_endpoint_identity_unsettled"))


def _bi_version(schema_revision):
    semantic, policy = BI_VERSIONS[schema_revision]
    return {"schema_revision": schema_revision, "semantic_version": semantic, "policy_version": policy}


def validate_bi_payload(kind, p):
    """#1392/#1405：仅验证 S 发布的完整形状和来源引用，不代算成笔。"""
    keys(p, ("schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"), kind)
    if p["schema_revision"] not in BI_VERSIONS or (p["semantic_version"], p["policy_version"], p["view_role"]) != (*BI_VERSIONS[p["schema_revision"]], "main"):
        raise ValueError("新笔合同/规则/政策版本不符")
    v2 = p["schema_revision"] == "s-new-bi/2"
    integer(p["object_generation"], "object_generation", 1)
    data = p["data"]

    def waiting(value, name):
        strings(value, name)
        if v2 and RETIRED_SAME_PRICE_WAITING.intersection(value):
            raise ValueError("同价域退役的等待态在 v2 对象复现")

    def endpoint(e):
        positions = ("raw_first", "raw_last") if v2 else ("raw_position",)
        keys(e, ("kind", "merged_index", "group_anchor", "price", "extreme_roots", *positions, "source_coords", "sealed_at", "waiting_reasons"), "endpoint")
        if e["kind"] not in ("TOP", "BOTTOM"):
            raise ValueError("端点类型未声明")
        for k in ("merged_index", "group_anchor"):
            integer(e[k], k, 0)
        integer(e["price"], "price")
        coords(e["extreme_roots"], "extreme_roots")
        coords(e["source_coords"], "source_coords")
        if not set(e["extreme_roots"]) <= set(e["source_coords"]):
            raise ValueError("实际根无原始来源")
        if v2:
            # 极值区：根集非空，首末序位覆盖全部根（单根时首末相同）。
            first = integer(e["raw_first"], "raw_first", 0)
            last = integer(e["raw_last"], "raw_last", first)
            if not e["extreme_roots"] or last - first < len(e["extreme_roots"]) - 1:
                raise ValueError("极值根集与首末 raw 序位不闭合")
        elif e["raw_position"] is not None:
            integer(e["raw_position"], "raw_position", 0)
        if e["sealed_at"] is not None:
            integer(e["sealed_at"], "sealed_at", 0)
        waiting(e["waiting_reasons"], "waiting_reasons")

    def conditions(c):
        keys(c, ("merged_gap", "raw_between_actual_extrema", "top_price", "bottom_price", "vector", "failed_conditions", "waiting_reasons"), "conditions")
        integer(c["merged_gap"], "merged_gap", 0)
        if v2 or c["raw_between_actual_extrema"] is not None:
            integer(c["raw_between_actual_extrema"], "raw_between", 0)
        for k in ("top_price", "bottom_price"):
            integer(c[k], k)
        allowed = (bool,) if v2 else (bool, type(None))
        if len(array(c["vector"], "vector")) != 3 or any(type(v) not in allowed for v in c["vector"]):
            raise ValueError("完整条件向量缺位/错型")
        strings(c["failed_conditions"], "failed_conditions")
        waiting(c["waiting_reasons"], "waiting_reasons")

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
        keys(data, ("old", "new", "endpoint_kind_pair", "conditions", "comparison", "selection", "retained_anchors", *(("evidence",) if v2 else ()), "waiting_reasons"), kind)
        endpoint(data["old"])
        endpoint(data["new"])
        if data["endpoint_kind_pair"] != data["old"]["kind"] + "/" + data["new"]["kind"]:
            raise ValueError("端点分域标签不闭合")
        if kind == BI_KINDS[1]:
            if data["old"]["kind"] == data["new"]["kind"]:
                raise ValueError("异型请求的端点实际同型")
            conditions(data["conditions"])
            if data["comparison"] is not None or data["selection"] is not None or (v2 and data["evidence"] != []):
                raise ValueError("异型域混入同型选择")
        elif data["old"]["kind"] != data["new"]["kind"] or data["conditions"] is not None or data["comparison"] not in ("LT", "EQ", "GT") or data["selection"] not in (("KEEP", "REPLACE") if v2 else ("KEEP", "REPLACE", "UNDETERMINED")):
            raise ValueError("同型域混入异型假位或选择缺失")
        elif v2:
            # 一个比较器三输出（#1343 Q1）：严格更强 ⟹ REPLACE；更弱或同价 ⟹ KEEP 在先者。
            stronger = data["comparison"] == ("GT" if data["old"]["kind"] == "TOP" else "LT")
            if data["selection"] != ("REPLACE" if stronger else "KEEP"):
                raise ValueError("同型取舍与价格比较不闭合")
            if data["retained_anchors"] != [data["new" if stronger else "old"]["group_anchor"]]:
                raise ValueError("同型保留锚不符取舍")
            if data["evidence"] != ([SAME_PRICE_KEEP_EVIDENCE] if data["comparison"] == "EQ" else []):
                raise ValueError("同价保留在先者的证据缺失或混入")
        coords(data["retained_anchors"], "retained_anchors")
        if v2:
            strings(data["evidence"], "evidence")
        waiting(data["waiting_reasons"], "waiting_reasons")
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
                # after 与本事件同版本；before 是先前已发布的对象，按其自身代际形状校验（AsKnown）。
                version = p["schema_revision"] if k == "after" else ("s-new-bi/2" if "raw_first" in data[k]["start"] else "s-new-bi/1")
                validate_bi_payload(BI_KINDS[3], {**p, **_bi_version(version), "object_generation": data[k]["entity_id"].split(":")[1], "data": data[k]})
    else:
        keys(data, ("input_frontier", "version_causes", "identity_basis", "classification_obligation", "waiting_reasons", "known_at"), kind)
        integer(data["input_frontier"], "input_frontier", -1)
        strings(data["version_causes"], "version_causes")
        waiting(data["waiting_reasons"], "waiting_reasons")
        known(data["known_at"])
