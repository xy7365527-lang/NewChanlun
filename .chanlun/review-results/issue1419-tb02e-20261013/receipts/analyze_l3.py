#!/usr/bin/env python3
"""#1419 收尾第 4 项：真实 BTC 尾 5 万根上的 L3 对拍与 oracle 读数（本地证据，不入仓）。

输入：l3-dump.json（组合树 = #1419 9cd64da5cd + 根裁定 L3 修复 ecc9b196d3 的 tb02e.rs/Q/JS 改动 + #1405 199b820c9d，
tmp_tb1419_realdata_l3_dump 导出）。旧读数（schema v1.2 几何端点口径）见 l3-dump-v1.2.json / l3-readout-v1.2.json。
三方：生产塔 L0 中枢（classify().levels[0].centers）、S 写者（tb02c → 输入合同 → tb02e::build）、
S0 独立 oracle（.chanlun/review-results/issue1419-tb02e-20261013/oracle_check.py，按定义从单元重算）。
解除判据（根裁定 L3 第 4 条）：9 个切点上输入合同全 PASS、S 单元 = 塔单元、S 写者对塔逐位全过、S 对 oracle 全过、
塔对 oracle 全过、Q validate_center_cut 全 ACCEPT。
"""
import importlib.util
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
COMPOSITE = Path("/private/tmp/goal1323-native-20261012/tb1419-x1405-composite")
spec = importlib.util.spec_from_file_location("oracle", COMPOSITE / ".chanlun/review-results/issue1419-tb02e-20261013/oracle_check.py")
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
sys.path.insert(0, str(COMPOSITE / "s_session"))
import s_tb02_contract as contract  # noqa: E402


def s_units(segment_records):
    """schema v1.3 §1 的 S 侧单元：完成段区间 = 成员笔方向极值（Q 合同 `_member_extremes`，根裁定 L3）。
    同时记几何端点 sort(stroke_refs[0].start_price, geometric_end.price)，只用来数判别力。"""
    rows = []
    for r in segment_records:
        if r["kind"] != "CC-013.segment":
            continue
        d = r["payload"]["data"]
        if d["result"] not in ("CASE_ONE", "CASE_TWO"):
            continue
        a, b = int(d["stroke_refs"][0]["start_price"]), int(d["geometric_end"]["price"])
        lo, hi = contract._member_extremes(d)
        rows.append({"start": int(d["construction"]["start_stroke"]), "unit_id": d["entity_id"],
                     "dir": d["construction"]["direction"], "lo": lo, "hi": hi, "geo_lo": min(a, b), "geo_hi": max(a, b),
                     "geo_start_not_extreme": a not in (lo, hi), "geo_end_not_extreme": b not in (lo, hi),
                     "n_members": len(d["construction"]["stroke_indices"]), "n_stroke_refs": len(d["stroke_refs"])})
    rows.sort(key=lambda x: x["start"])
    return rows


def oracle_scan(units):
    """S0 oracle 的 C 口径扫描（last+2、Zn 外缘、≥9 只给段层判定）。units 需带 idx/dir/lo/hi。"""
    us = [{"idx": k, "dir": u["dir"], "lo": u["lo"], "hi": u["hi"], "from": None, "to": None} for k, u in enumerate(units)]
    if len(us) < 3:
        return [], [], None, us
    centers, attempts, cursor = oracle.scan_c(us, len(us) - 1)
    return centers, attempts, cursor, us


def tower_windows(tower_centers, units):
    """塔中枢 → (首单元位, 末单元位)，以单元 start/end_index 对齐。"""
    by_start = {u["start_index"]: k for k, u in enumerate(units)}
    by_end = {u["end_index"]: k for k, u in enumerate(units)}
    return [(by_start.get(c["start_index"]), by_end.get(c["end_index"])) for c in tower_centers]


def compare_tower_oracle(tower_centers, units, o_centers):
    """塔 vs oracle（同一单元列表）：<9 窗口一一对应逐字段；≥9 窗口塔走旧子窗（§9.5 R22 过渡态），只记数。"""
    win = tower_windows(tower_centers, units)
    rows, diffs, ti = [], [], 0
    for c in o_centers:
        mem = c["_members"] if "_members" in c else list(range(c["first"], c["last"] + 1))
        if c["upgrade_pending"]:
            emitted = [k for k, (f, l) in enumerate(win) if f is not None and l is not None and c["first"] <= f and l <= c["last"]]
            rows.append({"first": c["first"], "last": c["last"], "count": c["count"], "upgrade_pending": True, "tower_subcenters": len(emitted)})
            ti = (max(emitted) + 1) if emitted else ti
            continue
        if ti >= len(tower_centers):
            diffs.append({"oracle_center": [c["first"], c["last"]], "tower": None})
            continue
        t = tower_centers[ti]
        r = c["roles"]
        want = {"first": c["first"], "last": c["last"], "zd": r["CORE"]["ZD"]["value"], "zg": r["CORE"]["ZG"]["value"],
                "dd": r["OUTER"]["DD"]["value"], "gg": r["OUTER"]["GG"]["value"]}
        got = {"first": win[ti][0], "last": win[ti][1], "zd": t["zd"], "zg": t["zg"], "dd": t["dd"], "gg": t["gg"]}
        if want != got:
            diffs.append({"oracle": want, "tower": got})
        rows.append(dict(want, count=c["count"], settled=c["settled"], upgrade_pending=False))
        ti += 1
    extra = len(tower_centers) - ti
    return rows, diffs, extra


def s_centers(records):
    by = {}
    for r in records:
        by.setdefault(r["kind"], []).append(r["payload"]["data"])
    return by


def compare_s_oracle(records, units, o_centers, o_attempts, o_cursor):
    by = s_centers(records)
    pos = {u["unit_id"]: k for k, u in enumerate(units)}
    roles = {d["center_id"]: d["roles"] for d in by.get("CC-017.center_roles", [])}
    sc = sorted(by.get("CC-016.center", []), key=lambda c: pos[c["members"][0]])
    diffs = []
    if len(sc) != len(o_centers):
        diffs.append({"count": [len(sc), len(o_centers)]})
    for c, o in zip(sc, o_centers):
        got = {"first": pos[c["members"][0]], "last": pos[c["members"][-1]], "settled": c["settled"],
               "status_upgrade": c["status"] == "UPGRADE_PENDING_TB02H",
               "break_unit": pos.get(c["break_unit"]) if c["break_unit"] else None, "break_up": c["break_up"] if c["settled"] else None,
               "resume_after": pos[c["resume_anchor"]["after_unit_id"]] if c["resume_anchor"] else None}
        want = {"first": o["first"], "last": o["last"], "settled": o["settled"], "status_upgrade": o["upgrade_pending"],
                "break_unit": o["break_unit"], "break_up": o["break_up"], "resume_after": o["last"] if o["settled"] else None}
        if got != want:
            diffs.append({"s": got, "oracle": want})
        rs = roles[c["entity_id"]]
        for grp, names in (("CORE", ("ZD", "ZG")), ("OUTER", ("DD", "GG")), ("INNER_EXTREMA", ("D", "G"))):
            for nm in names:
                sv = rs[grp][nm]
                s_val = None if sv["value"] is None else int(sv["value"])
                s_con = [pos[x] for x in sv["contributors"]]
                if o["roles"] is None:
                    ok = (grp == "CORE") or s_val is None
                    if grp == "CORE":
                        z = oracle.seed_eval(oracle_us_cache["us"], o["first"])[nm]
                        ok = (s_val, s_con) == (z["value"], z["contributors"])
                else:
                    ov = o["roles"][grp][nm]
                    ok = (s_val, s_con) == (ov["value"], ov["contributors"])
                if not ok:
                    diffs.append({"center": c["entity_id"], "var": nm, "s": [s_val, s_con], "oracle": None if o["roles"] is None else o["roles"][grp][nm]})
    att = sorted(by.get("CC-016.center_seed_attempt", []), key=lambda a: pos[a["units"][0]["unit_id"]])
    s_att = [(pos[a["units"][0]["unit_id"]], a["result"], a["exclusion_reason"]) for a in att]
    o_att = [(a["start"], a["result"], a["exclusion_reason"]) for a in o_attempts]
    if s_att != o_att:
        diffs.append({"attempts": {"s": s_att[:20], "oracle": o_att[:20], "len": [len(s_att), len(o_att)]}})
    return {"s_centers": len(sc), "s_attempts": len(s_att), "diffs": diffs}


def compare_s_tower(records, units, tower_centers):
    """S 写者对塔逐位：<9 成员（非 UPGRADE）窗口比首末与 ZD/ZG/DD/GG；>=9 成员 S 标 UPGRADE_PENDING_TB02H，
    塔给子中枢（§9.5 R22 过渡态），单列。塔中枢既不对上 S 窗口、也不落在 UPGRADE 窗口内的，记差异。"""
    by = s_centers(records)
    pos = {u["unit_id"]: k for k, u in enumerate(units)}
    roles = {d["center_id"]: d["roles"] for d in by.get("CC-017.center_roles", [])}
    sc = sorted(by.get("CC-016.center", []), key=lambda c: pos[c["members"][0]])
    win = tower_windows(tower_centers, units)
    used, diffs, rows, upgrade = set(), [], [], []
    for c in sc:
        first, last = pos[c["members"][0]], pos[c["members"][-1]]
        if c["status"] == "UPGRADE_PENDING_TB02H":
            subs = [k for k, (f, l) in enumerate(win) if f is not None and l is not None and first <= f and l <= last]
            used.update(subs)
            upgrade.append({"first": first, "last": last, "members": len(c["members"]),
                            "tower_subcenters": [[win[k][0], win[k][1]] for k in subs]})
            continue
        rs = roles[c["entity_id"]]
        val = lambda grp, nm: None if rs[grp][nm]["value"] is None else int(rs[grp][nm]["value"])
        got = {"first": first, "last": last, "zd": val("CORE", "ZD"), "zg": val("CORE", "ZG"),
               "dd": val("OUTER", "DD"), "gg": val("OUTER", "GG")}
        hit = [k for k, w in enumerate(win) if w == (first, last) and k not in used]
        if len(hit) != 1:
            diffs.append({"s": got, "tower": None, "tower_hits": len(hit)})
            continue
        t = tower_centers[hit[0]]
        want = {"first": first, "last": last, "zd": t["zd"], "zg": t["zg"], "dd": t["dd"], "gg": t["gg"]}
        used.add(hit[0])
        if got != want:
            diffs.append({"s": got, "tower": want})
        rows.append(dict(got, members=len(c["members"]), settled=c["settled"]))
    orphan = [[win[k][0], win[k][1]] for k in range(len(tower_centers)) if k not in used]
    if orphan:
        diffs.append({"tower_centers_without_s": orphan})
    return {"s_centers": len(sc), "compared": len(rows), "upgrade_pending": upgrade, "diffs": diffs, "centers_detail": rows}


oracle_us_cache = {}


def main():
    dump = json.loads((HERE / "l3-dump.json").read_text())
    report = {"schema": "tb1419-realdata-l3-readout/2", "unit_rule": "member-stroke-extremes (schema v1.3, ROOT-RULING-1419-L3)", "cuts": []}
    all_pass = True
    for cut in dump["cuts"]:
        units_t = cut["tower_units"]
        units_s = s_units(cut["segment_records"])
        assert len(units_s) == len(units_t), (cut["bars"], len(units_s), len(units_t))
        mism = [{"k": k, "s": [s["lo"], s["hi"], s["dir"]], "tower": [t["lo"], t["hi"], t["dir"]]}
                for k, (s, t) in enumerate(zip(units_s, units_t)) if (s["lo"], s["hi"], s["dir"]) != (t["lo"], t["hi"], t["dir"])]
        geo_diff = sum((s["geo_lo"], s["geo_hi"]) != (s["lo"], s["hi"]) for s in units_s)
        geo_start = sum(s["geo_start_not_extreme"] for s in units_s)
        geo_end = sum(s["geo_end_not_extreme"] for s in units_s)
        oc_t, oa_t, ocur_t, us_t = oracle_scan(units_t)
        rows_t, diffs_t, extra_t = compare_tower_oracle(cut["tower_l0_centers"], units_t, oc_t)
        entry = {"bars": cut["bars"], "strokes": cut["strokes"], "segments": cut["segments"], "completed_units": len(units_t),
                 "tower_l0_centers": len(cut["tower_l0_centers"]), "tower_levels": cut["tower_levels"],
                 "input_contract": cut["input_contract"], "s_unit_vs_tower_unit_mismatches": mism,
                 "geometric_interval_differs": geo_diff, "geometric_start_not_extreme": geo_start, "geometric_end_not_extreme": geo_end,
                 "oracle_on_tower_units": {"centers": len(oc_t), "settled": sum(c["settled"] for c in oc_t),
                                           "upgrade_pending": sum(c["upgrade_pending"] for c in oc_t), "attempts": len(oa_t),
                                           "excluded": sum(a["result"] != "FORMED" for a in oa_t), "cursor": ocur_t,
                                           "tower_vs_oracle_diffs": diffs_t, "tower_extra_centers": extra_t, "centers_detail": rows_t}}
        ok = cut["input_contract"] == "PASS" and not mism and not diffs_t and extra_t == 0
        if cut["input_contract"] == "PASS":
            ou = [dict(u, unit_id=s["unit_id"]) for u, s in zip(units_t, units_s)]
            oracle_us_cache["us"] = us_t
            entry["s_vs_tower"] = compare_s_tower(cut["s_records"], ou, cut["tower_l0_centers"])
            entry["s_vs_oracle"] = compare_s_oracle(cut["s_records"], ou, oc_t, oa_t, ocur_t)
            try:
                contract.validate_center_cut(cut["s_records"], contract.CENTER_RULE, cut["axes"])
                entry["q_contract"] = "ACCEPT"
            except ValueError as e:
                entry["q_contract"] = str(e)
            ok = ok and not entry["s_vs_tower"]["diffs"] and not entry["s_vs_oracle"]["diffs"] and entry["q_contract"] == "ACCEPT"
        else:
            entry["input_contract_receipt"] = cut["input_contract_receipt"]
        entry["pass"] = ok
        all_pass = all_pass and ok
        report["cuts"].append(entry)
        line = (f"bars={cut['bars']:>6} strokes={cut['strokes']:>5} segs={cut['segments']:>3} units={len(units_t):>3} "
                f"tower_L0={len(cut['tower_l0_centers']):>2} oracle={len(oc_t):>2} T~O_diffs={len(diffs_t)} extra={extra_t} "
                f"unit_mism={len(mism)} geo_differs={geo_diff:>2} S_input={cut['input_contract']}")
        if cut["input_contract"] == "PASS":
            st = entry["s_vs_tower"]
            line += (f" S_centers={st['s_centers']} S~T_compared={st['compared']} upgrade={len(st['upgrade_pending'])}"
                     f" S~T_diffs={len(st['diffs'])} S~O_diffs={len(entry['s_vs_oracle']['diffs'])} Q={entry['q_contract'][:40]}")
        print(line + f" PASS={ok}")
    report["all_cuts_pass"] = all_pass
    print("ALL_CUTS_PASS", all_pass)
    (HERE / "l3-readout.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
