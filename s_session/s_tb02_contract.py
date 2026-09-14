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
    if kind in BI_KINDS:
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


def project_fact(row, parse):
    if type(row) is not dict:
        row = dict(zip(FACT_COLUMNS, row))
    result = {k: row[k] for k in FACT_COLUMNS if k not in ("payload_json", "input_refs_json", "source_coords_json")}
    for name in ("payload", "input_refs", "source_coords"):
        result[name] = parse(row[name + "_json"], dict if name == "payload" else list)
        if row[name + "_json"] != canonical(result[name]).decode():
            raise ValueError("事实 JSON 列不是封存规范字节")
    for name in ("object_revision", "first_known_generation", "published_generation", "withdrawn_generation"):
        if result[name] is not None:
            if type(result[name]) is not int:
                raise ValueError("事实持久整数类型不符")
            result[name] = str(result[name])
    result["lifecycle"] = "active" if result["withdrawn_generation"] is None else "withdrawn"
    validate_fact(result)
    return result


def validate_fact(value, meta=None, raw=None):
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
    validate_payload(value["kind"], value["payload"])
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
    slot = p["slot"] if kind in BI_KINDS else p["incoming"]["source_coord"] if kind == KINDS[0] else p["group_anchor"] if kind == KINDS[1] else p["window"] if kind == KINDS[2] else "TB-02-A"
    if value["fact_key"] != canonical([kind, slot]).decode():
        raise ValueError("fact_key 与真实语义槽不符")
    if meta is not None:
        identity = {key: value[key] for key in ("kind", "fact_key", "payload", "input_refs")}
        identity.update(profile_id=meta["profile_id"], rule_revision=meta["rule_revision"])
        if value["object_id"] != "obj-" + hashlib.sha256(canonical(identity)).hexdigest():
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
    keys(axes, AXES if "CC-008" in axes else LEGACY_AXES, "catalog.axes")
    for cid, value in axes.items():
        keys(value, ("impl_status", "proof_status", "run_status", "evidence"), cid)
        if value["impl_status"] not in ("implemented", "not_implemented") or value["proof_status"] != "not_proved" or value["run_status"] not in ("run", "waiting", "not_run"):
            raise ValueError("逐轴实现/证明/运行状态不符")
        evidence = value["evidence"]
        keys(evidence, ("scope", "object_ids", "waiting_reasons", "raw_revisions", "withdrawals", "replaces") if cid == "CC-056" else ("scope", "object_ids", "waiting_reasons"), cid + ".evidence")
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
        if evidence["scope"] != "TB-02-A":
            raise ValueError("逐轴证据范围不符")
        ids = strings(evidence["object_ids"], "axis.object_ids")
        if len(ids) != len(set(ids)) or (object_ids is not None and not set(ids) <= object_ids):
            raise ValueError("逐轴对象证据缺失/重复")
        strings(evidence["waiting_reasons"], "axis.waiting_reasons")


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
