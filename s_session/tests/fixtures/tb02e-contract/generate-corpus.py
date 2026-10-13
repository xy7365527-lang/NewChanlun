#!/usr/bin/env python3
"""#1419 TB-02-E：Q/JS 合同共享语料（schema v1.2）。只用标准库整数算术，不导入项目代码。

合法 cut 由段层端点按定义直算（seed=三单元交替且 max(低)<min(高)；延伸=闭区间相交；Zn=与 seed 首单元同向；
结算后自 last+2 续扫；归属按根裁定 D9/D-G/D-I 与 r2 R2/R3/R6），再按 schema v1 投影成 wire 对象。反例是对合法
cut 的补丁，每条标 expected 原因码。Python（test_tb02e_contract.py）与 JS（test_tb02e_contract.cjs）各自判同一
语料，并逐条比对两侧结论与原因码。重生成：python3 -B generate-corpus.py > corpus.json
"""
import copy
import json
import sys

CENTER_RULE = "s2-axis-quantifiers+new-bi/1+segment-second/1+center-seed/1"
SEGMENT_SECOND_RULE = "s2-axis-quantifiers+new-bi/1+segment-second/1"
TUPLE = {"schema_revision": "s-center/1", "semantic_version": "center-seed/1",
         "policy_version": "strict_seed_zn_outer_c_resume", "view_role": "main"}
KNOWN = {"generation": "13", "input_frontier": "12", "receipt_id": "receipt-12",
         "received_at": "2000-01-01T00:00:00Z", "semantic_commit_ns": None}
DEFERRED = ["global_G009_upper_levels_1429", "ge9_upgrade_1429", "third_class_and_break_1434",
            "expansion_newborn_1424", "connector_upper_construction_1429"]
STREAMS = {
    "UP": [10, 40, 20, 35, 30, 38, 21, 25, 5, 15, 2, 12, 4, 9, 6],
    "E4": [10, 40, 20, 35, 5, 15, 2, 18, 15, 17, 1, 8, 6],
    "LEAD": [10, 40, 5, 9, 3, 20, 12, 15],
    "W9": [10, 40, 20, 35, 25, 38, 22, 36, 24, 37, 26, 28],
}
STREAMS["DOWN"] = [45 - p for p in STREAMS["UP"]]
STREAMS["E4_DOWN"] = [35, 5, 25, 10, 40, 30, 43, 27, 30, 28, 44, 37, 39]


def uid(k):
    return f"segment:1:{10 + 4 * k}"


def units_of(endpoints):
    return [{"k": k, "dir": "UP" if b > a else "DOWN", "lo": min(a, b), "hi": max(a, b)}
            for k, (a, b) in enumerate(zip(endpoints, endpoints[1:]))]


def scan(units):
    attempts, windows, i, n = [], [], 0, len(units)
    while i + 2 < n:
        s = units[i:i + 3]
        zd, zg = max(u["lo"] for u in s), min(u["hi"] for u in s)
        alt = s[0]["dir"] != s[1]["dir"] and s[1]["dir"] != s[2]["dir"]
        formed = alt and zd < zg
        attempts.append((i, formed))
        if not formed:
            i += 1
            continue
        j = i + 3
        while j < n and units[j]["lo"] <= zg and units[j]["hi"] >= zd:
            j += 1
        windows.append((i, j))
        i = j + 1 if j < n else j
    return attempts, windows, i


def extremum(members, attr, pick):
    value = pick(u[attr] for u in members)
    contributors = [uid(u["k"]) for u in members if u[attr] == value]
    return value, contributors


def witness(c):
    return c if len(c) >= 2 else []


def rec(kind, oid, slot, data):
    return {"kind": kind, "object_id": oid, "payload": dict(TUPLE, object_generation="1", slot=slot, data=data)}


def cut(stream, k, windows=None, attempts=None):
    """K_k：u0..u_k 完成、u_{k+1}（若端点在）进行中。可覆写窗口/尝试以造跨对象反例。"""
    endpoints = STREAMS[stream]
    units = units_of(endpoints[:k + 2])
    n = len(units)
    has_pending = k + 2 < len(endpoints)
    a0, w0, consumed = scan(units)
    windows = w0 if windows is None else windows
    attempts = a0 if attempts is None else attempts
    records = []
    for u in units:
        a, b = endpoints[u["k"]], endpoints[u["k"] + 1]
        records.append({"kind": "CC-013.segment", "object_id": f"obj-seg-{u['k']}", "payload": {"schema_revision": "s-segment/2", "data": {
            "entity_id": uid(u["k"]), "result": "CASE_ONE", "construction": {"start_stroke": str(3 * u["k"]), "direction": u["dir"]},
            "stroke_refs": [{"start_price": str(a)}], "geometric_end": {"price": str(b)}}}})
    if has_pending:
        records.append({"kind": "CC-013.segment", "object_id": f"obj-seg-{n}", "payload": {"schema_revision": "s-segment/2", "data": {
            "entity_id": uid(n), "result": "NO_FIRST", "construction": {"start_stroke": str(3 * n), "direction": None},
            "stroke_refs": [{"start_price": str(endpoints[n])}], "geometric_end": None}}})
    wire_unit = lambda u: {"unit_id": uid(u["k"]), "unit_object_id": f"obj-seg-{u['k']}", "direction": u["dir"], "lo": str(u["lo"]), "hi": str(u["hi"])}
    center_of = {i: f"center:1:{10 + 4 * i}" for i, _ in windows}
    for i, formed in attempts:
        s = units[i:i + 3]
        (zd, zdc), (zg, zgc) = extremum(s, "lo", max), extremum(s, "hi", min)
        order = "ZD<ZG" if zd < zg else "ZD=ZG" if zd == zg else "ZD>ZG"
        records.append(rec("CC-016.center_seed_attempt", f"obj-attempt-{i}", f"seed_attempt:1:{10 + 4 * i}", {
            "entity_id": f"seed_attempt:1:{10 + 4 * i}", "units": [wire_unit(u) for u in s],
            "vector": {"completed": True, "consecutive": True, "same_level": True, "alternating": True, "strict": zd < zg},
            "ZD": {"value": str(zd), "contributors": zdc, "same_price_witness": witness(zdc)},
            "ZG": {"value": str(zg), "contributors": zgc, "same_price_witness": witness(zgc)},
            "core_order": order, "result": "FORMED" if formed else "EXCLUDED",
            "exclusion_reason": None if formed else ("TANGENT_ZD_EQ_ZG" if order == "ZD=ZG" else "INVERTED_ZD_GT_ZG"),
            "center_id": center_of.get(i) if formed else None, "known_at": KNOWN}))
    member_of, upgraded = {}, set()
    for w, (i, j) in enumerate(windows):
        members = units[i:j]
        settled = j < n
        zn = [u for u in members if u["dir"] == members[0]["dir"]]
        up9 = len(members) >= 9
        cid = center_of[i]
        roles = {}
        for role, name, pool, attr, pick in (("CORE", "ZD", members[:3], "lo", max), ("CORE", "ZG", members[:3], "hi", min),
                                             ("OUTER", "DD", zn, "lo", min), ("OUTER", "GG", zn, "hi", max),
                                             ("INNER_EXTREMA", "D", zn, "lo", max), ("INNER_EXTREMA", "G", zn, "hi", min)):
            value, contributors = extremum(pool, attr, pick)
            hidden = up9 and role != "CORE"
            roles.setdefault(role, {})[name] = {"value": None if hidden else str(value), "contributors": [] if hidden else contributors,
                                                "same_price_witness": [] if hidden else witness(contributors), "frozen": role == "CORE",
                                                "null_reason": "UPGRADE_PENDING_TB02H" if hidden else None}
        zg = min(u["hi"] for u in members[:3])
        records.append(rec("CC-016.center", f"obj-center-{i}", cid, {
            "entity_id": cid, "entity_revision": "1", "seed_attempt_id": f"seed_attempt:1:{10 + 4 * i}",
            "seed_units": [uid(u["k"]) for u in members[:3]], "zn_direction": members[0]["dir"],
            "members": [uid(u["k"]) for u in members], "zn_members": [uid(u["k"]) for u in zn],
            "status": "UPGRADE_PENDING_TB02H" if up9 else "SETTLED" if settled else "EXTENDING", "settled": settled,
            "break_unit": uid(j) if settled else None, "break_up": settled and units[j]["lo"] > zg, "break_up_defined": settled,
            "resume_anchor": {"after_unit_id": uid(j - 1), "offset": "2"} if settled else None,
            "next_tested_unit": uid(n) if not settled and has_pending else None, "roles_id": "roles:" + cid,
            "formed_known_at": KNOWN, "settled_known_at": KNOWN if settled else None}))
        records.append(rec("CC-017.center_roles", f"obj-roles-{i}", "roles:" + cid, {
            "entity_id": "roles:" + cid, "entity_revision": "1", "center_id": cid, "roles": roles, "previous": None, "known_at": KNOWN}))
        for u in members:
            member_of[u["k"]] = w
            if up9:
                upgraded.add(u["k"])
    tried = dict(attempts)
    reasons = []
    for u in units + ([{"k": n}] if has_pending else []):
        k = u["k"]
        status, center, left, right, pending, reason = "NOT_YET_ASSIGNED", None, None, None, False, None
        if k >= n:
            reason = "UNIT_NOT_COMPLETED"
        elif k in upgraded:
            reason = "UPGRADE_PENDING_TB02H"
        elif k in member_of:
            status, center = "CENTER_MEMBER", center_of[windows[member_of[k]][0]]
        else:
            lefts = [w for w, (i, j) in enumerate(windows) if j <= k]
            if not lefts:
                reason = "NO_LEFT_CENTER_IN_WINDOW"
            else:
                l = lefts[-1]
                if l + 1 < len(windows):
                    status, left, right = "CONNECTOR", center_of[windows[l][0]], center_of[windows[l + 1][0]]
                elif windows[l][1] == k or tried.get(k) is False:
                    status, left, pending = "CONNECTOR", center_of[windows[l][0]], True
                else:
                    reason = "SEED_NOT_YET_EVALUABLE"
        if reason:
            reasons.append(reason)
        unit = wire_unit(u) if k < n else {"unit_id": uid(k), "unit_object_id": f"obj-seg-{k}", "completed": False}
        seed_from_self = "NOT_EVALUATED" if k not in tried else "FORMED" if tried[k] else "FAILED"
        records.append(rec("CC-022.unit_assignment", f"obj-assign-{k}", "assign:" + uid(k), {
            "entity_id": "assign:" + uid(k), "unit": unit, "status": status, "center_id": center, "left_center_id": left,
            "right_center_id": right, "right_pending": pending, "reason": reason, "seed_from_self": seed_from_self, "known_at": KNOWN}))
    id_at = lambda k: uid(k) if k < n or (k == n and has_pending) else None
    if windows and windows[-1][1] == n:
        i, j = windows[-1]
        nxt = id_at(n)
        cursor = {"state": "EXTENDING", "center_id": center_of[i], "seed_start_unit_id": None, "awaiting_unit_id": nxt,
                  "awaiting_after_unit_id": None if nxt else uid(n - 1), "awaiting_offset": None if nxt else "1"}
    else:
        want = consumed + 2
        nxt = id_at(want)
        cursor = {"state": "SEEKING_SEED", "center_id": None, "seed_start_unit_id": id_at(consumed), "awaiting_unit_id": nxt,
                  "awaiting_after_unit_id": None if nxt or n == 0 else uid(n - 1), "awaiting_offset": None if nxt else str(want - (n - 1))}
    records.append(rec("CC-056.center_version", "obj-version", "TB-02-E", {
        "input_frontier": "12", "known_at": KNOWN, "scan_cursor": cursor, "waiting_reasons": sorted(set(reasons)),
        "delivered_scope": "l0_strict_seed_zn_roles_c_resume_assignment", "deferred_scope": DEFERRED}))
    for i, j in windows:
        if j - i >= 9:
            continue  # UPGRADE_PENDING_TB02H 修订不发 member_of（schema v1.1 §4，D-J2 第 2 点）
        for k in range(i, j):
            records.append(rec("CC-055.center_relation", f"obj-rel-{k}", [uid(k), "member_of", center_of[i]],
                               {"source_id": uid(k), "relation_kind": "member_of", "target_id": center_of[i], "version": "1", "witness_object_id": f"obj-center-{i}"}))
    if windows:
        i = windows[0][0]
        records.append(rec("CC-055.center_change", "obj-change-0", ["13", center_of[i], "formed"],
                           {"entity_id": center_of[i], "change": "formed", "before": None, "after": {"entity_id": center_of[i]}, "known_at": KNOWN, "object_id": f"obj-center-{i}"}))
    axes = {"CC-016": {"evidence": {"object_ids": sorted(r["object_id"] for r in records if r["kind"] in ("CC-016.center_seed_attempt", "CC-016.center"))}},
            "CC-017": {"evidence": {"object_ids": sorted(r["object_id"] for r in records if r["kind"] == "CC-017.center_roles")}},
            "CC-022": {"evidence": {"object_ids": sorted(r["object_id"] for r in records if r["kind"] == "CC-022.unit_assignment")}}}
    return {"rule_revision": CENTER_RULE, "records": records, "axes": axes}


def find(case, kind, entity):
    for r in case["records"]:
        if r["kind"] == kind and r["payload"]["data"].get("entity_id") == entity:
            return r["payload"]["data"]
    raise KeyError((kind, entity))


def find_kind(case, kind):
    return next(r["payload"]["data"] for r in case["records"] if r["kind"] == kind)


def mutate(base, fn):
    case = copy.deepcopy(base)
    fn(case)
    return case


def diff(base, case):
    """把反例存成对合法 cut 的差分：按 object_id 替换（首个出现）、删除、追加；轴与规则另存。"""
    by_id = {}
    for r in base["records"]:
        by_id.setdefault(r["object_id"], r)
    seen, replace, add = set(), {}, []
    for r in case["records"]:
        oid = r["object_id"]
        if oid in by_id and oid not in seen:
            seen.add(oid)
            if r != by_id[oid]:
                replace[oid] = r
        else:
            add.append(r)
    remove = [oid for oid in by_id if oid not in seen]
    return {"base": BASE_NAMES[id(base)], "rule_revision": case["rule_revision"], "replace": replace, "add": add,
            "remove": remove, "axes": case["axes"] if case["axes"] != base["axes"] else None}


BASE_NAMES = {}


def main():
    # 七个「seed 起点为进行中单元」切面（UP/DOWN K8、E4/E4_DOWN K4 与 K10、LEAD K5）与 W9 K8/K9 按 v1.1 唯一编码。
    bases = {"up_k12": cut("UP", 12), "up_k10": cut("UP", 10), "up_k8": cut("UP", 8), "up_k1": cut("UP", 1),
             "down_k8": cut("DOWN", 8), "down_k12": cut("DOWN", 12), "e4_k4": cut("E4", 4), "e4_k7": cut("E4", 7),
             "e4_k8": cut("E4", 8), "e4_k10": cut("E4", 10), "e4down_k4": cut("E4_DOWN", 4), "e4down_k10": cut("E4_DOWN", 10),
             "lead_k5": cut("LEAD", 5), "w9_k8": cut("W9", 8), "w9_k9": cut("W9", 9)}
    cases = [{"name": name, "expected": "ACCEPT", "base": None, **case} for name, case in bases.items()]
    BASE_NAMES.update({id(case): name for name, case in bases.items()})
    A, B, UP = "center:1:10", "center:1:46", bases["up_k12"]

    def add(name, expected, base, fn):
        cases.append({"name": name, "expected": expected, **diff(base, mutate(base, fn))})

    def set_in(kind, entity, path, value):
        def fn(case):
            d = find(case, kind, entity)
            for p in path[:-1]:
                d = d[p]
            d[path[-1]] = value
        return fn

    def roles_of(case, cid):
        return find(case, "CC-017.center_roles", "roles:" + cid)["roles"]

    add("connector_as_member_break_unit", "CONNECTOR_AS_MEMBER", UP, lambda c: find(c, "CC-016.center", A)["members"].append(uid(8)))
    add("connector_assigned_as_member", "CONNECTOR", UP,
        lambda c: find(c, "CC-022.unit_assignment", "assign:" + uid(8)).update(status="CENTER_MEMBER", center_id=A, left_center_id=None, right_center_id=None))
    plus_one = cut("UP", 12, windows=[(0, 8), (8, 13)], attempts=[(0, True), (8, True)])
    cases.append({"name": "plus_one_reuse", "expected": "PLUS_ONE_REUSE", "base": None, **plus_one})
    add("resume_anchor_offset_one", "RESUME_ANCHOR", UP, set_in("CC-016.center", A, ["resume_anchor", "offset"], "1"))
    add("resume_anchor_after_not_last", "RESUME_ANCHOR", UP, set_in("CC-016.center", A, ["resume_anchor", "after_unit_id"], uid(6)))
    add("d_value_as_core", "ROLE_VALUE", UP, lambda c: roles_of(c, A)["CORE"].update(ZD={"value": "30", "contributors": [uid(4)], "same_price_witness": [], "frozen": True, "null_reason": None}))
    add("d_key_in_core", "ROLES_SHAPE", UP, lambda c: roles_of(c, A).update(CORE={"ZD": roles_of(c, A)["CORE"]["ZD"], "D": roles_of(c, A)["INNER_EXTREMA"]["D"]}))
    add("role_missing_g", "ROLES_SHAPE", UP, lambda c: roles_of(c, A)["INNER_EXTREMA"].pop("G"))
    add("contributors_missing", "CONTRIBUTORS", UP, lambda c: roles_of(c, A)["OUTER"]["DD"].update(contributors=[], same_price_witness=[]))
    add("contributors_outside_zn", "CONTRIBUTORS", UP, lambda c: roles_of(c, A)["OUTER"]["GG"].update(contributors=[uid(0), uid(1)], same_price_witness=[uid(0), uid(1)]))
    add("candidate_kind", "CANDIDATE_KIND", UP, lambda c: c["records"].append(dict(copy.deepcopy(c["records"][-1]), kind="CC-016.center_candidate")))
    add("member_not_completed", "MEMBER_NOT_COMPLETED", UP, lambda c: find(c, "CC-016.center", B).update(members=find(c, "CC-016.center", B)["members"] + [uid(13)], next_tested_unit=None))
    add("unit_source_mismatch", "UNIT_SOURCE", UP, lambda c: find(c, "CC-016.center_seed_attempt", "seed_attempt:1:10")["units"][1].update(hi="41"))
    add("unsettled_break_up_defined", "BREAK_UP_UNDEFINED", UP, set_in("CC-016.center", B, ["break_up_defined"], True))
    add("assignment_two_states", "ASSIGNMENT", UP, set_in("CC-022.unit_assignment", "assign:" + uid(13), ["center_id"], B))
    add("assignment_duplicate", "ASSIGNMENT_CONSISTENCY", UP, lambda c: c["records"].append(copy.deepcopy(next(r for r in c["records"] if r["kind"] == "CC-022.unit_assignment"))))
    add("assignment_missing", "ASSIGNMENT_CONSISTENCY", UP, lambda c: c["records"].remove(next(r for r in c["records"] if r["kind"] == "CC-022.unit_assignment" and r["payload"]["data"]["entity_id"] == "assign:" + uid(12))))
    add("rule_mix_old_rule", "RULE_MIX", UP, lambda c: c.update(rule_revision=SEGMENT_SECOND_RULE))
    add("rule_mix_old_tuple", "VERSION", UP, lambda c: next(r for r in c["records"] if r["kind"] == "CC-016.center")["payload"].update(semantic_version="segment-second/1"))
    E4K7 = bases["e4_k7"]
    tangent = "seed_attempt:1:30"
    add("tangent_missing_reason", "EXCLUSION_REASON", E4K7, set_in("CC-016.center_seed_attempt", tangent, ["exclusion_reason"], None))
    add("tangent_wrong_reason", "EXCLUSION_REASON", E4K7, set_in("CC-016.center_seed_attempt", tangent, ["exclusion_reason"], "INVERTED_ZD_GT_ZG"))
    add("not_alternating", "NOT_ALTERNATING", E4K7, lambda c: find(c, "CC-016.center_seed_attempt", tangent).update(exclusion_reason="NOT_ALTERNATING"))
    add("resume_gap_missing_failed_attempt", "RESUME_GAP", bases["e4_k8"], lambda c: c["records"].remove(next(r for r in c["records"] if r["kind"] == "CC-016.center_seed_attempt" and r["payload"]["data"]["entity_id"] == tangent)))
    add("upgrade_dynamic_role_valued", "ROLE_NULL", bases["w9_k8"], lambda c: roles_of(c, A)["OUTER"]["DD"].update(value="10", contributors=[uid(0)], null_reason=None))
    add("axis_binding_missing_object", "AXIS_BINDING", UP, lambda c: c["axes"]["CC-017"]["evidence"]["object_ids"].pop())
    version = lambda c: find_kind(c, "CC-056.center_version")["scan_cursor"]
    K8 = bases["up_k8"]
    add("cursor_seed_start_null_with_in_progress", "SCAN_CURSOR", K8, lambda c: version(c).update(seed_start_unit_id=None))
    add("cursor_offset_wrong", "SCAN_CURSOR", K8, lambda c: version(c).update(awaiting_offset="2"))
    add("cursor_after_not_last_completed", "SCAN_CURSOR", K8, lambda c: version(c).update(awaiting_after_unit_id=uid(7)))
    add("cursor_extending_on_settled_center", "SCAN_CURSOR", bases["up_k10"], lambda c: version(c).update(state="EXTENDING", center_id=A, seed_start_unit_id=None))
    add("cursor_next_tested_mismatch", "SCAN_CURSOR", UP, set_in("CC-016.center", B, ["next_tested_unit"], None))
    W9K8 = bases["w9_k8"]
    add("upgrade_member_edge", "MEMBER_EDGE", W9K8, lambda c: c["records"].append(rec("CC-055.center_relation", "obj-rel-x", [uid(0), "member_of", A],
        {"source_id": uid(0), "relation_kind": "member_of", "target_id": A, "version": "1", "witness_object_id": "obj-center-0"})))
    add("upgrade_member_assigned_to_center", "ASSIGNMENT_CONSISTENCY", W9K8, lambda c: find(c, "CC-022.unit_assignment", "assign:" + uid(3)).update(status="CENTER_MEMBER", center_id=A, reason=None))
    add("upgrade_reason_outside_upgrade_center", "ASSIGNMENT_CONSISTENCY", bases["lead_k5"], set_in("CC-022.unit_assignment", "assign:" + uid(0), ["reason"], "UPGRADE_PENDING_TB02H"))
    add("member_edge_from_connector", "MEMBER_EDGE", UP, lambda c: c["records"].append(rec("CC-055.center_relation", "obj-rel-y", [uid(8), "member_of", A],
        {"source_id": uid(8), "relation_kind": "member_of", "target_id": A, "version": "1", "witness_object_id": "obj-center-0"})))
    add("break_up_wrong", "BREAK_UP", UP, set_in("CC-016.center", A, ["break_up"], True))
    # schema v1.2 §6 E4：非 UPGRADE 中枢的每个成员恰有一条指向该中枢的 member_of 边。正例＝上面各合法 cut（两中枢的
    # up_k12/down_k12 每成员一条，W9 的 UPGRADE 中枢零条）；反例＝已结算 A 缺一条、开放 B 缺新成员那条、同一边重发。
    drop_edge = lambda k: lambda c: c["records"].remove(next(r for r in c["records"] if r["object_id"] == f"obj-rel-{k}"))
    add("member_edge_missing_settled", "MEMBER_EDGE_COUNT", UP, drop_edge(3))
    add("member_edge_missing_open_newest", "MEMBER_EDGE_COUNT", UP, drop_edge(12))
    add("member_edge_duplicate", "MEMBER_EDGE_COUNT", UP, lambda c: c["records"].append(dict(copy.deepcopy(next(r for r in c["records"] if r["object_id"] == "obj-rel-0")), object_id="obj-rel-0-dup")))
    json.dump({"schema": "tb02e-contract-corpus/1", "ticket": 1419, "schema_doc": "tb1419-schema.md v1.2（冻结，E1–E4）",
               "provenance": "generate-corpus.py 按定义直算段层合法 cut，反例为其补丁；不读生产输出", "cases": cases},
              sys.stdout, ensure_ascii=False, separators=(",", ":"))
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
