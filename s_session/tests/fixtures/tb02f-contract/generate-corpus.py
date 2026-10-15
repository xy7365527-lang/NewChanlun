#!/usr/bin/env python3
"""#1424 TB-02-F：Q/JS 中枢对合同共享语料（tb1424-schema.md）。只用标准库整数算术，不导入项目代码。

合法 cut：中枢族（s-center/1）沿用 #1419 语料生成器 `../tb02e-contract/generate-corpus.py` 的 `cut()`（同一份按定义直算
的 TB-02-E 投影，不另写一套）；对族（s-center-pair/1）在本文件按定义直算——延伸测试单元位置五叶、相邻中枢五叶逐叶
独立求值、16 格端点矩阵、相切/触及/包含证据、一端 ≥9 窗口记待定（#1424 根裁定第 2 问）。反例是对合法 cut 的补丁，
每条标 expected 原因码。Python（test_tb02f_contract.py）与 JS（test_tb02f_contract.cjs）各自判同一语料，并逐条比对
两侧结论与原因码。重生成：python3 -B generate-corpus.py > corpus.json
"""
import copy
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("tb02e_corpus", HERE.parent / "tb02e-contract" / "generate-corpus.py")
E = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(E)

PAIR_RULE = E.CENTER_RULE + "+center-pair/1"
TUPLE = {"schema_revision": "s-center-pair/1", "semantic_version": "center-pair/1",
         "policy_version": "zn_outer_five_leaf_pending_ge9", "view_role": "main"}
DEFERRED = ["upper_level_pairs_1429", "ge9_pairs_pending_1429", "upper_center_realization_1429", "trend_realization_1426",
            "first_retest_destruction_1434", "two_step_settlement_r5_1496"]
REALIZATION = {"upper_center": "NOT_CONSTRUCTED_DEFERRED_1429", "trend": "NOT_ASSERTED_DEFERRED_1426"}
DESTRUCTION = "NOT_EVALUATED_DEFERRED_1434"
PENDING = "PENDING_UPGRADE_TB02H"
ENDPOINTS = ("DD", "ZD", "ZG", "GG")
BASE = [10, 40, 20, 35, 30, 38, 21, 25, 5]
# 工作例（编号同 rust/tests/fixtures/center_pair_1424_worked_examples.json）；末尾多一个端点作进行中单元。
E.STREAMS.update({
    "P_W3": BASE + [9, 2, 8, 3, 5],
    "P_W5": BASE + [10, 2, 8, 3, 5],
    "P_W6": [10, 40, 20, 35, 30, 50, 40, 60, 30, 40, 35, 38, 36],
    "P_W7": [10, 40, 20, 35, 30, 50, 40, 55, 42, 48, 5, 60, 45, 50],
    "P_W9": [10, 40, 20, 35, 30, 38, 22, 36, 25, 33, 21, 26, 5, 9, 2, 8, 3, 5],
    "P_W10": [10, 40, 20, 45, 40, 50, 12, 20, 8, 15, 3, 12, 5, 9],
    # W3 的镜像（45−p，同 #1419 DOWN 流的造法）：向上新生 UP_NEWBIRTH，补齐合法 cut 的五叶覆盖
    "P_W3_MIRROR": [45 - p for p in BASE + [9, 2, 8, 3, 5]],
    # W6 的镜像：核心相切落在另一侧（B.ZG = A.ZD），两侧相切各有一个合法 cut
    "P_W6_MIRROR": [45 - p for p in [10, 40, 20, 35, 30, 50, 40, 60, 30, 40, 35, 38, 36]],
})


def anchor(x, prefix):
    return x[len(prefix) + 1:]


def rec(kind, oid, slot, data):
    return {"kind": kind, "object_id": oid, "payload": dict(TUPLE, object_generation="1", slot=slot, data=data)}


def position(lo, hi, zd, zg):
    return "STRICT_BELOW" if hi < zd else "LOW_TOUCH" if hi == zd else "STRICT_ABOVE" if lo > zg else "HIGH_TOUCH" if lo == zg else "INTERIOR_CONTACT"


def leaves(a, b):
    sep = b["ZG"] < a["ZD"] or b["ZD"] > a["ZG"]
    preds = (("CORE_NOT_STRICTLY_SEPARATED", not sep), ("UP_NEWBIRTH", b["DD"] > a["GG"]), ("DOWN_NEWBIRTH", b["GG"] < a["DD"]),
             ("UP_EXPANSION", b["ZD"] > a["ZG"] and b["DD"] <= a["GG"]), ("DOWN_EXPANSION", b["ZG"] < a["ZD"] and b["GG"] >= a["DD"]))
    return [leaf for leaf, holds in preds if holds]


def cmp(x, y):
    return "<" if x < y else "=" if x == y else ">"


def containment(alo, ahi, blo, bhi):
    a_has_b, b_has_a = alo <= blo and bhi <= ahi, blo <= alo and ahi <= bhi
    return "EQUAL" if a_has_b and b_has_a else "A_CONTAINS_B" if a_has_b else "B_CONTAINS_A" if b_has_a else "NONE"


def cut(stream, k):
    """在 #1419 的 K_k 合法 cut 上追加对族对象（全部修订号 1、获知时点同中枢族）。"""
    case = E.cut(stream, k)
    case["rule_revision"] = PAIR_RULE
    records = case["records"]
    seg = {}
    for r in records:
        if r["kind"] == "CC-013.segment" and r["payload"]["data"]["result"] == "CASE_ONE":
            d = r["payload"]["data"]
            a, b = int(d["stroke_refs"][0]["start_price"]), int(d["stroke_refs"][0]["end_price"])
            seg[d["entity_id"]] = {"unit_id": d["entity_id"], "unit_object_id": r["object_id"], "direction": d["construction"]["direction"],
                                   "lo": str(min(a, b)), "hi": str(max(a, b))}
    order = sorted(seg, key=lambda u: int(u.split(":")[2]))
    pos = {u: i for i, u in enumerate(order)}
    centers = sorted((r["payload"]["data"] for r in records if r["kind"] == "CC-016.center"), key=lambda c: pos[c["members"][0]])
    roles = {r["payload"]["data"]["center_id"]: r["payload"]["data"]["roles"] for r in records if r["kind"] == "CC-017.center_roles"}

    def side(cid):
        ro = roles[cid]
        return {"ZD": ro["CORE"]["ZD"]["value"], "ZG": ro["CORE"]["ZG"]["value"], "DD": ro["OUTER"]["DD"]["value"], "GG": ro["OUTER"]["GG"]["value"]}

    added, edges = [], []
    for c in centers:
        cid = c["entity_id"]
        zd, zg = int(roles[cid]["CORE"]["ZD"]["value"]), int(roles[cid]["CORE"]["ZG"]["value"])
        up9 = c["status"] == "UPGRADE_PENDING_TB02H"
        tested = [(m, "UPGRADE_PENDING_MEMBER" if up9 else "EXTENSION_MEMBER") for m in c["members"][3:]]
        if c["settled"]:
            tested.append((c["break_unit"], "BREAK_UNIT"))
        for uid, role in tested:
            u = seg[uid]
            p = position(int(u["lo"]), int(u["hi"]), zd, zg)
            closed = p in ("LOW_TOUCH", "INTERIOR_CONTACT", "HIGH_TOUCH")
            eid = "ucp:" + anchor(cid, "center") + ":" + anchor(uid, "segment")
            oid = f"obj-ucp-{anchor(cid, 'center')}-{anchor(uid, 'segment')}"
            added.append(rec("CC-018.unit_core_position", oid, eid, {
                "entity_id": eid, "center_id": cid, "center_revision": "1", "unit": u, "core": {"ZD": str(zd), "ZG": str(zg)},
                "position": p, "closed_overlap": closed, "strict_leave": not closed,
                "leave_side": "DOWN" if p == "STRICT_BELOW" else "UP" if p == "STRICT_ABOVE" else None,
                "construction_role": role, "destruction": DESTRUCTION, "known_at": E.KNOWN}))
            edges += [(eid, "position_of", uid, "1", oid), (eid, "position_against", cid, "1", oid)]
    pairs = []
    for i, (a, b) in enumerate(zip(centers, centers[1:])):
        eid = "pair:" + anchor(a["entity_id"], "center") + ":" + anchor(b["entity_id"], "center")
        sa, sb = side(a["entity_id"]), side(b["entity_id"])
        between = order[pos[a["members"][-1]] + 1:pos[b["members"][0]]]
        data = {"entity_id": eid, "entity_revision": "1", "a_center_id": a["entity_id"], "a_revision": a["entity_revision"],
                "b_center_id": b["entity_id"], "b_revision": b["entity_revision"], "connector_units": between,
                "provisional": not b["settled"], "inputs": {"A": sa, "B": sb}, "realization": REALIZATION, "previous": None,
                "formed_known_at": E.KNOWN, "known_at": E.KNOWN}
        if sa["DD"] is None or sb["DD"] is None:
            data.update(status=PENDING, leaf=None, direction=None, matrix=None, witnesses=None, null_reason=PENDING)
        else:
            A = {k: int(v) for k, v in sa.items()}
            B = {k: int(v) for k, v in sb.items()}
            (leaf,) = leaves(A, B)
            data.update(status="EVALUATED", leaf=leaf, direction="UP" if leaf.startswith("UP_") else "DOWN" if leaf.startswith("DOWN_") else None,
                        matrix={f"B.{x}?A.{y}": cmp(B[x], A[y]) for x in ENDPOINTS for y in ENDPOINTS},
                        witnesses={"core_tangent": B["ZD"] == A["ZG"] or B["ZG"] == A["ZD"], "outer_touch": B["DD"] == A["GG"] or B["GG"] == A["DD"],
                                   "outer_containment": containment(A["DD"], A["GG"], B["DD"], B["GG"]),
                                   "core_containment": containment(A["ZD"], A["ZG"], B["ZD"], B["ZG"])},
                        null_reason=None)
        oid = f"obj-pair-{i}"
        added.append(rec("CC-020.center_pair", oid, eid, data))
        edges += [(eid, "pair_from", a["entity_id"], "1", oid), (eid, "pair_to", b["entity_id"], "1", oid)]
        pairs.append(data)
    for n, (src, kind, tgt, ver, oid) in enumerate(edges):
        added.append(rec("CC-055.pair_relation", f"obj-prel-{n}", [src, kind, tgt],
                         {"source_id": src, "relation_kind": kind, "target_id": tgt, "version": ver, "witness_object_id": oid}))
    if pairs:
        added.append(rec("CC-055.pair_change", "obj-pchange-0", ["13", pairs[0]["entity_id"], "formed"],
                         {"entity_id": pairs[0]["entity_id"], "change": "formed", "before": None, "after": pairs[0], "known_at": E.KNOWN, "object_id": "obj-pair-0"}))
    pending = sum(p["status"] == PENDING for p in pairs)
    added.append(rec("CC-056.pair_version", "obj-pversion", "TB-02-F", {
        "input_frontier": "12", "known_at": E.KNOWN, "pair_count": str(len(pairs)), "pending_pairs": str(pending),
        "waiting_reasons": [PENDING] if pending else [], "delivered_scope": "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position",
        "deferred_scope": DEFERRED}))
    records.extend(added)
    case["axes"]["CC-018"] = {"evidence": {"object_ids": sorted(r["object_id"] for r in added if r["kind"] == "CC-018.unit_core_position")}}
    case["axes"]["CC-020"] = {"evidence": {"object_ids": sorted(r["object_id"] for r in added if r["kind"] == "CC-020.center_pair")}}
    return case


def find(case, kind, entity):
    for r in case["records"]:
        if r["kind"] == kind and r["payload"]["data"].get("entity_id") == entity:
            return r["payload"]["data"]
    raise KeyError((kind, entity))


def history_cuts(stream="P_W12", cuts=(11, 12, 13), prefix="history_w12_k"):
    """W12 的真实输入前缀：B 加成员（叶不变）再改外缘（叶变），附稳定及撤回切面。

    按 schema 的修订规则组装历史，不导入生产校验器或 Rust 输出。
    """
    E.STREAMS["P_W12"] = BASE + [9, 2, 8, 3, 12, 4]
    E.STREAMS["P_PENDING"] = BASE + [9, 2, 8, 3, 7, 4, 7, 4, 7, 4, 7]
    result, previous = {}, None
    for revision, k in enumerate(cuts, 1):
        current = cut(stream, k)
        pair_r = next(r for r in current["records"] if r["kind"] == "CC-020.center_pair")
        d = pair_r["payload"]["data"]
        known = dict(E.KNOWN, generation=str(k + 1), input_frontier=str(k))
        old = None if previous is None else next(r["payload"]["data"] for r in previous["records"] if r["kind"] == "CC-020.center_pair")
        for r in current["records"]:
            if r["kind"] == "CC-016.center" and r["payload"]["data"]["entity_id"] == d["b_center_id"]:
                r["payload"]["data"]["entity_revision"] = str(revision)
        d.update(entity_revision=str(revision), b_revision=str(revision), known_at=known,
                 formed_known_at=known if old is None else old["formed_known_at"],
                 previous=None if old is None else {key: old[key] for key in ("entity_revision", "status", "leaf")})
        pair_r["object_id"] = "obj-history-pair-" + str(revision)
        pair_r["payload"]["object_generation"] = str(k + 1)
        change = ("formed" if old is None else "pending_upgrade" if old["status"] != d["status"] and d["status"] == PENDING
                  else "leaf_revised" if old["leaf"] != d["leaf"] else "inputs_revised")
        for r in current["records"]:
            value = r["payload"]["data"]
            if r["kind"] == "CC-055.pair_relation" and value["source_id"] == d["entity_id"]:
                value.update(version=str(revision), witness_object_id=pair_r["object_id"])
            if r["kind"] == "CC-055.pair_change":
                value.update(change=change, before=copy.deepcopy(old), after=copy.deepcopy(d), known_at=known, object_id=pair_r["object_id"])
                r["payload"]["slot"] = [known["generation"], d["entity_id"], change]
                r["object_id"] = "obj-history-change-" + str(revision)
        current["axes"]["CC-020"]["evidence"]["object_ids"] = [pair_r["object_id"]]
        result[prefix + str(k)] = current
        previous = current
    if stream != "P_W12":
        return result
    assert [next(r["payload"]["data"]["change"] for r in c["records"] if r["kind"] == "CC-055.pair_change") for c in result.values()] == ["formed", "inputs_revised", "leaf_revised"]
    stable = copy.deepcopy(previous)
    stable["records"] = [r for r in stable["records"] if r["kind"] != "CC-055.pair_change"]
    result["history_stable"] = stable
    withdrawn = cut("UP", 1)  # 输入修订撤去中枢后的合法空对切面。
    old = next(r["payload"]["data"] for r in stable["records"] if r["kind"] == "CC-020.center_pair")
    known = dict(E.KNOWN, generation="16")
    withdrawn["records"].append(rec("CC-055.pair_change", "obj-history-withdrawal", ["16", old["entity_id"], "withdrawn"],
        dict(entity_id=old["entity_id"], change="withdrawn", before=copy.deepcopy(old), after=None, known_at=known, object_id=None)))
    result["history_withdrawn"] = withdrawn
    return result


def main():
    bases = {"w1_up_k12": cut("UP", 12), "w2_down_k12": cut("DOWN", 12), "w3_k11": cut("P_W3", 11), "w5_k11": cut("P_W5", 11),
             "w6_k10": cut("P_W6", 10), "w7_k11": cut("P_W7", 11), "w9_k15": cut("P_W9", 15), "w10_k11": cut("P_W10", 11),
             "w3_mirror_k11": cut("P_W3_MIRROR", 11),
             "w6_mirror_k10": cut("P_W6_MIRROR", 10),
             "up_k8": cut("UP", 8), "up_k1": cut("UP", 1)}
    bases.update(history_cuts())
    bases.update(history_cuts("P_PENDING", (16, 17), "history_pending_k"))
    expect_leaf = {"w1_up_k12": "DOWN_EXPANSION", "w2_down_k12": "UP_EXPANSION", "w3_k11": "DOWN_NEWBIRTH", "w5_k11": "DOWN_EXPANSION",
                   "w6_k10": "CORE_NOT_STRICTLY_SEPARATED", "w7_k11": "UP_EXPANSION", "w9_k15": None, "w10_k11": "DOWN_EXPANSION",
                   "w3_mirror_k11": "UP_NEWBIRTH", "w6_mirror_k10": "CORE_NOT_STRICTLY_SEPARATED"}
    for name, leaf in expect_leaf.items():
        got = [r["payload"]["data"]["leaf"] for r in bases[name]["records"] if r["kind"] == "CC-020.center_pair"]
        assert got == [leaf], (name, got)  # 与 oracle 工作例同叶（自检，不是期待来源）
    cases = [{"name": name, "expected": "ACCEPT", "base": None, **case} for name, case in bases.items()]
    E.BASE_NAMES.update({id(case): name for name, case in bases.items()})
    W1, W3, W9, W6, W10 = bases["w1_up_k12"], bases["w3_k11"], bases["w9_k15"], bases["w6_k10"], bases["w10_k11"]
    P1, P3, P9 = "pair:1:10:1:46", "pair:1:10:1:46", "pair:1:10:1:62"

    def add(name, expected, base, fn):
        case = copy.deepcopy(base)
        fn(case)
        delta = E.diff(base, case)
        # Python 的 0 == False 不能抹掉 JSON 类型负控。
        originals = {r["object_id"]: r for r in base["records"]}
        for r in case["records"]:
            if r["object_id"] in originals and json.dumps(r, sort_keys=True) != json.dumps(originals[r["object_id"]], sort_keys=True):
                delta["replace"][r["object_id"]] = r
        cases.append({"name": name, "expected": expected, **delta})

    def set_pair(entity, **kv):
        def fn(case):
            find(case, "CC-020.center_pair", entity).update(copy.deepcopy(kv))
        return fn

    def drop(oid):
        def fn(case):
            case["records"] = [r for r in case["records"] if r["object_id"] != oid]
        return fn

    def ucp(case, unit):
        return find(case, "CC-018.unit_core_position", "ucp:1:10:" + anchor(unit, "segment"))

    # 叶与输入（R26：外缘换回全单元——输入与叶一致改写，单对象门通过，跨对象门按 CC-017 Zn 外缘拒）
    add("leaf_wrong_full_unit_label", "LEAF", W3, set_pair(P3, leaf="DOWN_EXPANSION"))
    def full_unit_inputs(case):
        d = find(case, "CC-020.center_pair", P3)
        d["inputs"]["A"]["DD"] = "5"
        d.update(leaf="DOWN_EXPANSION")
        d["matrix"] = {f"B.{x}?A.{y}": cmp(int(d["inputs"]["B"][x]), int(d["inputs"]["A"][y])) for x in ENDPOINTS for y in ENDPOINTS}
        d["witnesses"]["outer_containment"] = containment(5, 40, 2, 9)
    add("inputs_full_unit_outer_r26", "PAIR_INPUTS", W3, full_unit_inputs)
    add("direction_wrong", "DIRECTION", W1, set_pair(P1, direction="UP"))
    def flip_cell(case):
        find(case, "CC-020.center_pair", P1)["matrix"]["B.GG?A.DD"] = "<"
    add("matrix_cell_wrong", "MATRIX", W1, flip_cell)
    def witness_wrong(case):
        find(case, "CC-020.center_pair", "pair:1:10:1:38")["witnesses"]["core_tangent"] = False
    add("witness_core_tangent_dropped", "WITNESS", W6, witness_wrong)
    add("realization_claims_constructed", "REALIZATION", W1, set_pair(P1, realization={"upper_center": "CONSTRUCTED", "trend": "NOT_ASSERTED_DEFERRED_1426"}))
    def level_lift(case):
        find(case, "CC-020.center_pair", P1)["level_lift"] = True
    add("level_lift_key_rejected", "SHAPE", W1, level_lift)
    add("provisional_wrong", "PROVISIONAL", W1, set_pair(P1, provisional=False))
    add("revision_mismatch", "REVISION", W1, set_pair(P1, b_revision="2"))
    add("previous_on_first_revision", "REVISION", W1, set_pair(P1, previous={"entity_revision": "0", "status": "EVALUATED", "leaf": "DOWN_EXPANSION"}))
    add("connectors_wrong", "CONNECTORS", W1, set_pair(P1, connector_units=["segment:1:46"]))
    # ≥9：塔子中枢关系不得照抄（W9 塔末子中枢 → B 判 DOWN_EXPANSION）
    # 塔末子中枢 u9..u11 实测 GG=33 < ZG=35（越域，#1424 S1 发现）：原样照抄先撞载荷域门；把 GG 改成合域值仍须被
    # “输入须等于同 cut 中枢角色”拦下。两条分开登记，免得一条判据替另一条兜底。
    def copy_tower(gg):
        def fn(case):
            d = find(case, "CC-020.center_pair", P9)
            d["inputs"]["A"].update(DD="5", GG=gg)
            d.update(status="EVALUATED", leaf="DOWN_EXPANSION", direction="DOWN", null_reason=None,
                     matrix={f"B.{x}?A.{y}": cmp(int(d["inputs"]["B"][x]), int(d["inputs"]["A"][y])) for x in ENDPOINTS for y in ENDPOINTS},
                     witnesses={"core_tangent": False, "outer_touch": False, "outer_containment": "NONE", "core_containment": "NONE"})
        return fn
    add("ge9_tower_subcenter_out_of_domain", "DOMAIN", W9, copy_tower("33"))
    add("ge9_copies_tower_subcenter_relation", "PAIR_INPUTS", W9, copy_tower("40"))
    # #1424 C1：保留 12 成员待升级身份，把角色和对一起伪装为可判，值仍是 Zn 的真实极值。
    # 必须由整 cut 的中枢 upgrade/null 双向门拒绝，不能只测 pair 与角色不一致。
    def ge9_roles_and_pair_evaluated(case):
        centers = {r["payload"]["data"]["entity_id"]: r["payload"]["data"] for r in case["records"] if r["kind"] == "CC-016.center"}
        roles = {r["payload"]["data"]["center_id"]: r["payload"]["data"]["roles"] for r in case["records"] if r["kind"] == "CC-017.center_roles"}
        assert centers["center:1:10"]["status"] == "UPGRADE_PENDING_TB02H"
        assert len(centers["center:1:10"]["members"]) == 12
        seg = {}
        for r in case["records"]:
            if r["kind"] == "CC-013.segment" and r["payload"]["data"]["result"] == "CASE_ONE":
                d = r["payload"]["data"]
                ends = [int(d["stroke_refs"][0][k]) for k in ("start_price", "end_price")]
                seg[d["entity_id"]] = (min(ends), max(ends))
        for cid, center in centers.items():
            if center["status"] != "UPGRADE_PENDING_TB02H":
                continue
            for role, name, attr, pick in (("OUTER", "DD", 0, min), ("OUTER", "GG", 1, max),
                                           ("INNER_EXTREMA", "D", 0, max), ("INNER_EXTREMA", "G", 1, min)):
                value = pick(seg[m][attr] for m in center["zn_members"])
                contributors = [m for m in center["zn_members"] if seg[m][attr] == value]
                roles[cid][role][name].update(value=str(value), contributors=contributors, null_reason=None,
                                             same_price_witness=contributors if len(contributors) >= 2 else [])
        d = find(case, "CC-020.center_pair", P9)
        for side, cid in (("A", d["a_center_id"]), ("B", d["b_center_id"])):
            d["inputs"][side] = {name: roles[cid][role][name]["value"]
                                 for role, name in (("CORE", "ZD"), ("CORE", "ZG"), ("OUTER", "DD"), ("OUTER", "GG"))}
        a, b = [{key: int(value) for key, value in d["inputs"][side].items()} for side in ("A", "B")]
        assert leaves(a, b) == ["DOWN_NEWBIRTH"]
        d.update(status="EVALUATED", leaf="DOWN_NEWBIRTH", direction="DOWN", null_reason=None,
                 matrix={f"B.{x}?A.{y}": cmp(b[x], a[y]) for x in ENDPOINTS for y in ENDPOINTS},
                 witnesses={"core_tangent": b["ZD"] == a["ZG"] or b["ZG"] == a["ZD"],
                            "outer_touch": b["DD"] == a["GG"] or b["GG"] == a["DD"],
                            "outer_containment": containment(a["DD"], a["GG"], b["DD"], b["GG"]),
                            "core_containment": containment(a["ZD"], a["ZG"], b["ZD"], b["ZG"])})
        for r in case["records"]:
            data = r["payload"]["data"]
            if r["kind"] == "CC-055.pair_change" and data["after"] is not None:
                data["after"] = copy.deepcopy(d)
            if r["kind"] == "CC-056.pair_version":
                data.update(pending_pairs="0", waiting_reasons=[])
        for axis in ("CC-018", "CC-020"):
            case["axes"][axis]["evidence"]["waiting_reasons"] = []
    add("ge9_roles_and_pair_forged_evaluated", "TB02E_ROLE_NULL", W9, ge9_roles_and_pair_evaluated)
    add("pending_with_leaf", "PENDING", W9, set_pair(P9, leaf="DOWN_NEWBIRTH"))
    def pending_on_evaluable(case):
        find(case, "CC-020.center_pair", P1).update(status=PENDING, leaf=None, direction=None, matrix=None, witnesses=None, null_reason=PENDING)
    add("pending_on_evaluable_pair", "PENDING", W1, pending_on_evaluable)
    # 计数、相邻性、边、版本、变化
    add("pair_missing", "PAIR_COUNT", W1, drop("obj-pair-0"))
    def swap_ends(case):
        d = find(case, "CC-020.center_pair", P1)
        d.update(a_center_id="center:1:46", b_center_id="center:1:10", entity_id="pair:1:46:1:10")
        for r in case["records"]:
            if r["object_id"] == "obj-pair-0":
                r["payload"]["slot"] = "pair:1:46:1:10"
            if r["kind"] == "CC-055.pair_change":
                r["payload"]["data"]["entity_id"] = "pair:1:46:1:10"
                r["payload"]["slot"][1] = "pair:1:46:1:10"
    add("pair_not_adjacent_order", "ADJACENCY", W1, swap_ends)
    add("edge_missing", "EDGE", W1, drop("obj-prel-0"))
    def version_count(case):
        find_v = [r for r in case["records"] if r["object_id"] == "obj-pversion"][0]["payload"]["data"]
        find_v["pair_count"] = "2"
    add("version_pair_count_wrong", "VERSION_COUNT", W1, version_count)
    add("version_missing_under_pair_rule", "VERSION_COUNT", W1, drop("obj-pversion"))
    def scope_wrong(case):
        [r for r in case["records"] if r["object_id"] == "obj-pversion"][0]["payload"]["data"]["deferred_scope"] = DEFERRED[:-1]
    add("version_scope_wrong", "SCOPE", W1, scope_wrong)
    def change_after_stale(case):
        r = [r for r in case["records"] if r["object_id"] == "obj-pchange-0"][0]
        r["payload"]["data"]["after"] = copy.deepcopy(r["payload"]["data"]["after"])
        r["payload"]["data"]["after"]["provisional"] = False
    add("change_after_not_current", "CHANGE", W1, change_after_stale)
    # CC-018
    def pos_wrong(case):
        ucp(case, "segment:1:22")["position"] = "LOW_TOUCH"
    add("position_wrong", "POSITION", W10, pos_wrong)
    def touch_as_leave(case):
        d = ucp(case, "segment:1:34")
        d.update(position="STRICT_BELOW", closed_overlap=False, strict_leave=True, leave_side="DOWN", construction_role="BREAK_UNIT")
    add("touch_reported_as_leave", "POSITION", W10, touch_as_leave)
    def role_wrong(case):
        d = ucp(case, "segment:1:42")
        d["construction_role"] = "EXTENSION_MEMBER"
    add("break_unit_role_wrong", "ROLE", W1, role_wrong)
    def ge9_role(case):
        ucp(case, "segment:1:22")["construction_role"] = "EXTENSION_MEMBER"
    add("ge9_member_role_wrong", "ROLE", W9, ge9_role)
    def destruction(case):
        ucp(case, "segment:1:42")["destruction"] = "DESTROYED"
    add("leave_claims_destruction", "DESTRUCTION", W1, destruction)
    add("ucp_missing", "UCP_COUNT", W1, drop("obj-ucp-1:10-1:42"))
    def core_wrong(case):
        d = ucp(case, "segment:1:22")
        d["core"]["ZG"] = "36"
    add("ucp_core_not_roles", "CORE", W1, core_wrong)
    def leave_side_by_direction(case):
        ucp(case, "segment:1:42")["leave_side"] = "UP"
    add("leave_side_from_direction", "POSITION", W1, leave_side_by_direction)
    # 规则与候选
    def old_rule(case):
        case["rule_revision"] = E.CENTER_RULE
    add("pair_objects_in_center_rule_cut", "RULE_MIX", W1, old_rule)
    def candidate(case):
        case["records"].append(rec("CC-020.center_pair_candidate", "obj-cand", "x", {"entity_id": "x"}))
    add("candidate_kind", "CANDIDATE_KIND", W1, candidate)
    def axis_unbound(case):
        case["axes"]["CC-020"]["evidence"]["object_ids"] = []
    add("axis_binding_missing", "AXIS_BINDING", W1, axis_unbound)
    # #1424 H1：不存在、存在但错误类型、存在但属于另一个发边对象，均不能充当见证。
    def edge_witness(value, relation="position_of"):
        def fn(case):
            edge = next(r["payload"]["data"] for r in case["records"] if r["kind"] == "CC-055.pair_relation" and r["payload"]["data"]["relation_kind"] == relation)
            edge["witness_object_id"] = value(case)
        return fn
    for relation in ("position_of", "pair_from"):
        add("witness_missing_" + relation, "EDGE", W1, edge_witness(lambda case: "obj-does-not-exist", relation))
        add("witness_segment_" + relation, "EDGE", W1, edge_witness(lambda case: next(r["object_id"] for r in case["records"] if r["kind"] == "CC-013.segment"), relation))
    add("witness_wrong_pair_source", "EDGE", W1, edge_witness(lambda case: "obj-pair-0"))
    for field in ("core_tangent", "outer_touch"):
        def bad_bool(case, field=field):
            d = find(case, "CC-020.center_pair", P3)
            d["witnesses"][field] = int(d["witnesses"][field])
        add("witness_numeric_" + field, "WITNESS", W3, bad_bool)
    def bad_true_bool(case):
        d = next(r["payload"]["data"] for r in case["records"] if r["kind"] == "CC-020.center_pair")
        assert d["witnesses"]["core_tangent"] is True
        d["witnesses"]["core_tangent"] = 1
    add("witness_numeric_true_core_tangent", "WITNESS", W6, bad_true_bool)
    for value, name in (({}, "object"), ([], "array")):
        add("connector_" + name, "IDENTITY", W3, set_pair(P3, connector_units=[value]))
    def event(case):
        return next(r for r in case["records"] if r["kind"] == "CC-055.pair_change")
    def fake_revision(case):
        r = event(case);d = r["payload"]["data"]
        d.update(change="leaf_revised", before=copy.deepcopy(d["after"]))
        r["payload"]["slot"][-1] = "leaf_revised"
    add("fake_first_leaf_revision", "CHANGE", W1, fake_revision)
    def bad_event_oid(case):
        event(case)["payload"]["data"]["object_id"] = "obj-does-not-exist"
    add("event_wrong_object_id", "CHANGE", W1, bad_event_oid)
    H11, H12, H13 = (bases["history_w12_k" + str(k)] for k in (11, 12, 13))
    def wrong_kind(case):
        r=event(case);r["payload"]["data"]["change"]="leaf_revised";r["payload"]["slot"][-1]="leaf_revised"
    add("inputs_change_claimed_leaf", "CHANGE", H12, wrong_kind)
    add("pending_change_claimed_leaf", "CHANGE", bases["history_pending_k17"], wrong_kind)
    def unchanged_revision(case):
        r=event(case);d=r["payload"]["data"]
        # 修订号/时点合法，但 before 的事实与 after 相同。
        d["before"].update({key: copy.deepcopy(value) for key,value in d["after"].items() if key not in ("entity_revision", "previous", "formed_known_at", "known_at")})
    add("unchanged_fact_claimed_revision", "CHANGE", H12, unchanged_revision)
    def wrong_previous(case):
        d=event(case)["payload"]["data"]
        d["before"]["entity_revision"]="3"
        d["before"]["previous"]={"entity_revision":"2", "status":"EVALUATED", "leaf":d["before"]["leaf"]}
    add("event_wrong_before_revision", "CHANGE", H12, wrong_previous)
    def before_malformed(case):
        event(case)["payload"]["data"]["before"]={"entity_id":P1}
    add("event_before_malformed", "SHAPE", H12, before_malformed)
    def wrong_formed_time(case):
        event(case)["payload"]["data"]["before"]["formed_known_at"]["generation"]="1"
    add("event_changes_formed_time", "CHANGE", H12, wrong_formed_time)
    # 单 cut 内在合法但必须用真实前 cut 才能识别的坏历史。
    add("history_missing_birth", "ACCEPT", H11, drop("obj-history-change-1"))
    add("history_missing_revision", "ACCEPT", H12, drop("obj-history-change-2"))
    add("history_missing_withdrawal", "ACCEPT", bases["history_withdrawn"], drop("obj-history-withdrawal"))
    def wrong_actual_before(case):
        event(case)["payload"]["data"]["before"]["a_revision"]="2"
    add("history_wrong_actual_before", "ACCEPT", H12, wrong_actual_before)
    transitions = [
        {"name":"birth", "before":None, "after":"history_w12_k11", "expected":"ACCEPT"},
        {"name":"inputs_revision", "before":"history_w12_k11", "after":"history_w12_k12", "expected":"ACCEPT"},
        {"name":"leaf_revision", "before":"history_w12_k12", "after":"history_w12_k13", "expected":"ACCEPT"},
        {"name":"pending_upgrade", "before":"history_pending_k16", "after":"history_pending_k17", "expected":"ACCEPT"},
        {"name":"stable", "before":"history_w12_k13", "after":"history_stable", "expected":"ACCEPT"},
        {"name":"withdrawal", "before":"history_stable", "after":"history_withdrawn", "expected":"ACCEPT"},
        {"name":"missing_birth", "before":None, "after":"history_missing_birth", "expected":"CHANGE"},
        {"name":"missing_revision", "before":"history_w12_k11", "after":"history_missing_revision", "expected":"CHANGE"},
        {"name":"missing_withdrawal", "before":"history_stable", "after":"history_missing_withdrawal", "expected":"CHANGE"},
        {"name":"wrong_actual_before", "before":"history_w12_k11", "after":"history_wrong_actual_before", "expected":"CHANGE"},
        {"name":"repeated_birth", "before":"history_w12_k11", "after":"history_w12_k11", "expected":"CHANGE"},
    ]
    json.dump({"schema": "tb02f-contract-corpus/1", "pair_rule": PAIR_RULE, "cases": cases, "transitions": transitions}, sys.stdout, ensure_ascii=False, indent=0)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
