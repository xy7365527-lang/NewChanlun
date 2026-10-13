#!/usr/bin/env python3
"""#1419 TB-02-E 独立 oracle 复算（S0 工位）。

只用 Python 标准库与整数算术；不导入、不读取任何项目实现（rust/src、formal/、src/newchan）。
输入只有本目录 worked-examples.json 里的端点序列与手算期待值；本脚本按定义从端点重算，逐项断言。

定义（出处与四步核见 ORACLE-DESIGN.md §2）：
  单元 u_k = 端点 P_k→P_{k+1}，区间 [min,max]，方向 UP/DOWN。
  seed(i) = (u_i,u_{i+1},u_{i+2}) 三者已完成；Bool5 = (完成, 连续, 同级, 交替, strict)；
           ZD=max(三低)、ZG=min(三高)，strict ⟺ ZD<ZG（Z-1、Z-2；020:52、020:54）。
  延伸：闭区间相交 lo<=ZG ∧ hi>=ZD，对全部单元测（zhongshu.md §4.1；G3 第 5 条 Q2、D6）。
  Zn：与 seed 第一单元方向相等的成员（020:52；ROOT-RULINGS-1419 D-A）。
  GG=max(gn)、G=min(gn)、D=max(dn)、DD=min(dn)，n 遍历全部 Zn（020:52、S-7；G3 第 5 条 Q1）。
  ZD/ZG 冻结在 seed。
  续扫：结算后从 last+2（Z-5）；seed 不成立 +1；扫描在首个未结算中枢处停（Lean refStep 语义）。
  ≥9 成员：窗口内单元 NOT_YET_ASSIGNED/UPGRADE_PENDING_TB02H，只给段层判定（D-I、D5）。
  归属：D9 + D-G（见 assign_c）。
  塔现状对照：结算后 i=j（从突破段本身续扫），dd/gg 取全部成员（PREP C1/C2 记录的现状口径）。
"""
import itertools
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PREP_JSON = "/private/tmp/goal1323-native-20261012/tb1419-prep/worked-examples.json"

N_ASSERT = 0
FAILS = []


def check(cond, msg):
    global N_ASSERT
    N_ASSERT += 1
    if not cond:
        FAILS.append(msg)


# ---------------------------------------------------------------- 基本对象
def mk_units(P):
    us = []
    for k in range(len(P) - 1):
        a, b = P[k], P[k + 1]
        us.append({"idx": k, "dir": "UP" if b > a else "DOWN", "from": a, "to": b,
                   "lo": a if a < b else b, "hi": b if a < b else a})
    return us


def hits(us, idxs, key, val):
    return [i for i in idxs if us[i][key] == val]


def robj(val, contrib):
    return {"value": val, "contributors": contrib, "same_price_witness": contrib if len(contrib) > 1 else []}


def seed_eval(us, i):
    tri = [i, i + 1, i + 2]
    lows = [us[t]["lo"] for t in tri]
    highs = [us[t]["hi"] for t in tri]
    zd, zg = max(lows), min(highs)
    alt = us[i]["dir"] != us[i + 1]["dir"] and us[i + 1]["dir"] != us[i + 2]["dir"]
    strict = zd < zg
    if zd < zg:
        order = "ZD<ZG"
    elif zd == zg:
        order = "ZD=ZG"
    else:
        order = "ZD>ZG"
    return {"start": i, "units": tri, "known_at": "K%d" % (i + 2),
            "vector": {"completed": True, "consecutive": True, "same_level": True, "alternating": alt, "strict": strict},
            "ZD": robj(zd, hits(us, tri, "lo", zd)), "ZG": robj(zg, hits(us, tri, "hi", zg)),
            "core_order": order, "result": "FORMED" if (alt and strict) else "EXCLUDED",
            "exclusion_reason": None if strict else ("TANGENT_ZD_EQ_ZG" if zd == zg else "INVERTED_ZD_GT_ZG")}


def overlaps(u, zd, zg):
    return u["lo"] <= zg and u["hi"] >= zd


def six(us, members, zd, zg):
    zdir = us[members[0]]["dir"]
    zn = [m for m in members if us[m]["dir"] == zdir]
    gg = max(us[m]["hi"] for m in zn)
    g = min(us[m]["hi"] for m in zn)
    d = max(us[m]["lo"] for m in zn)
    dd = min(us[m]["lo"] for m in zn)
    tri = members[:3]
    vals = {
        "ZD": robj(zd, hits(us, tri, "lo", zd)), "ZG": robj(zg, hits(us, tri, "hi", zg)),
        "DD": robj(dd, hits(us, zn, "lo", dd)), "GG": robj(gg, hits(us, zn, "hi", gg)),
        "D": robj(d, hits(us, zn, "lo", d)), "G": robj(g, hits(us, zn, "hi", g)),
    }
    frozen = {"ZD": True, "ZG": True, "DD": False, "GG": False, "D": False, "G": False}
    groups = {"CORE": ["ZD", "ZG"], "OUTER": ["DD", "GG"], "INNER_EXTREMA": ["D", "G"]}
    roles = {}
    for g_name, names in groups.items():
        roles[g_name] = {}
        for nm in names:
            o = dict(vals[nm])
            o["frozen"] = frozen[nm]
            roles[g_name][nm] = o
    return zn, roles


# ---------------------------------------------------------------- C 口径扫描（last+2、Zn 外缘）
def scan_c(us, K):
    """K = 最后一个已完成单元下标。返回 (centers, attempts, cursor)。"""
    centers, attempts = [], []
    i = 0
    cursor = None
    while True:
        if i + 2 > K:
            cursor = {"state": "SEEKING_SEED", "seed_start": i, "awaiting_unit": i + 2}
            break
        ev = seed_eval(us, i)
        attempts.append(ev)
        if ev["result"] != "FORMED":
            i += 1
            continue
        zd, zg = ev["ZD"]["value"], ev["ZG"]["value"]
        mem = [i, i + 1, i + 2]
        j = i + 3
        while j <= K and overlaps(us[j], zd, zg):
            mem.append(j)
            j += 1
        settled = j <= K
        cid = chr(ord("A") + len(centers))
        up9 = len(mem) >= 9
        zn, roles = six(us, mem, zd, zg)
        c = {"id": cid, "seed": [i, i + 1, i + 2], "zn_dir": us[i]["dir"], "first": i, "last": mem[-1],
             "count": len(mem), "settled": settled, "break_unit": j if settled else None,
             "break_up": (us[j]["lo"] > zg) if settled else None,
             "resume_anchor": mem[-1] + 2 if settled else None,
             "next_tested_unit": None if settled else mem[-1] + 1,
             "known_at": "K%d" % (i + 2), "zn_members": zn, "upgrade_pending": up9,
             "roles": None if up9 else roles,
             "c2_contrast_all_unit_dd": None if up9 else min(us[m]["lo"] for m in mem),
             "c2_contrast_all_unit_gg": None if up9 else max(us[m]["hi"] for m in mem),
             "_zn_dd_gg_raw": (min(us[m]["lo"] for m in zn), max(us[m]["hi"] for m in zn)),
             "_members": mem}
        centers.append(c)
        if not settled:
            cursor = {"state": "EXTENDING", "center": cid, "next_unit": j}
            break
        i = mem[-1] + 2
    return centers, attempts, cursor


def scan_tower(us, K):
    """塔现状口径：结算后 i=j，dd/gg 全部成员。"""
    out = []
    i = 0
    while i + 2 <= K:
        ev = seed_eval(us, i)
        if ev["result"] != "FORMED":
            i += 1
            continue
        zd, zg = ev["ZD"]["value"], ev["ZG"]["value"]
        mem = [i, i + 1, i + 2]
        j = i + 3
        while j <= K and overlaps(us[j], zd, zg):
            mem.append(j)
            j += 1
        settled = j <= K
        out.append({"seed": [i, i + 1, i + 2], "first": i, "last": mem[-1], "count": len(mem), "settled": settled,
                    "break_unit": j if settled else None, "zd": zd, "zg": zg,
                    "dd": min(us[m]["lo"] for m in mem), "gg": max(us[m]["hi"] for m in mem)})
        if not settled:
            break
        i = j
    return out


# ---------------------------------------------------------------- 归属（D9 + D-G + D-I）
def assign_c(us, K, centers, attempts):
    n_vis = K + 2 if K + 1 < len(us) else K + 1
    failed = {a["start"] for a in attempts if a["result"] != "FORMED"}
    formed = {a["start"] for a in attempts if a["result"] == "FORMED"}
    rec = {}
    for idx, c in enumerate(centers):
        for m in c["_members"]:
            if c["upgrade_pending"]:
                rec[m] = {"unit": m, "status": "NOT_YET_ASSIGNED", "reason": "UPGRADE_PENDING_TB02H"}
            else:
                rec[m] = {"unit": m, "status": "CENTER_MEMBER", "center": c["id"]}
        if c["settled"]:
            nxt = centers[idx + 1] if idx + 1 < len(centers) else None
            end = nxt["first"] - 1 if nxt else K
            for m in range(c["break_unit"], end + 1):
                if nxt is not None:
                    rec[m] = {"unit": m, "status": "CONNECTOR", "left": c["id"], "right": nxt["id"]}
                elif m == c["break_unit"] or m in failed:
                    rec[m] = {"unit": m, "status": "CONNECTOR", "left": c["id"], "right": "PENDING"}
                else:
                    rec[m] = {"unit": m, "status": "NOT_YET_ASSIGNED", "reason": "SEED_NOT_YET_EVALUABLE"}
    first = centers[0]["first"] if centers else K + 1
    for m in range(0, min(first, K + 1)):
        rec[m] = {"unit": m, "status": "NOT_YET_ASSIGNED", "reason": "NO_LEFT_CENTER_IN_WINDOW"}
    if K + 1 < len(us):
        rec[K + 1] = {"unit": K + 1, "status": "NOT_YET_ASSIGNED", "reason": "UNIT_NOT_COMPLETED"}
    out = []
    for m in range(n_vis):
        r = rec[m]  # KeyError = 有单元无状态 → 违反「恰一状态」
        r["seed_from_self"] = "FORMED" if m in formed else ("FAILED" if m in failed else "NOT_EVALUATED")
        out.append(r)
    return out


def changes(prev, cur, k, new_attempts):
    if prev is None:
        return None
    ch = [{"kind": "UNIT_COMPLETED", "unit": k}]
    for a in new_attempts:
        ch.append({"kind": "SEED_ATTEMPT", "start": a["start"], "result": a["result"]})
    old = {c["id"]: c for c in prev["centers"]}
    for c in cur["centers"]:
        p = old.get(c["id"])
        if p is None:
            ch.append({"kind": "CENTER_FORMED", "center": c["id"]})
            continue
        for u in range(p["last"] + 1, c["last"] + 1):
            ch.append({"kind": "MEMBER_ADDED", "center": c["id"], "unit": u})
        if c["upgrade_pending"] and not p["upgrade_pending"]:
            ch.append({"kind": "UPGRADE_PENDING", "center": c["id"]})
        if c["roles"] and p["roles"]:
            for grp, names in (("CORE", ("ZD", "ZG")), ("OUTER", ("DD", "GG")), ("INNER_EXTREMA", ("D", "G"))):
                for nm in names:
                    x, y = p["roles"][grp][nm], c["roles"][grp][nm]
                    if (x["value"], x["contributors"]) != (y["value"], y["contributors"]):
                        ch.append({"kind": "ROLE_CHANGED", "center": c["id"], "var": nm,
                                   "from": {"value": x["value"], "contributors": x["contributors"]},
                                   "to": {"value": y["value"], "contributors": y["contributors"]}})
        if c["settled"] and not p["settled"]:
            ch.append({"kind": "SETTLED", "center": c["id"], "break_unit": c["break_unit"],
                       "resume_anchor": c["resume_anchor"]})
    old_a = {a["unit"]: a for a in prev["assignment"]}
    for a in cur["assignment"]:
        if old_a.get(a["unit"]) != a:
            ch.append({"kind": "ASSIGNMENT_CHANGED", "unit": a["unit"], "from": old_a.get(a["unit"]), "to": a})
    return ch


def recompute_stream(P, K_last, with_tower, raw_final):
    us = mk_units(P)
    cuts, prev, seen = [], None, 0
    all_att = []
    for k in range(1, K_last + 1):
        cs, att, cur = scan_c(us, k)
        new_att = att[seen:] if att[:seen] == all_att[:seen] else None
        assert new_att is not None, "扫描路径不单调"
        all_att = att
        seen = len(att)
        cut = {"cut": "K%d" % k, "k": k, "completed_through": k,
               "in_progress": k + 1 if k + 1 < len(us) else None,
               "centers": cs, "scan_cursor": cur, "assignment": assign_c(us, k, cs, att)}
        if with_tower:
            tw = scan_tower(us, k)
            c_seeds = {c["first"] for c in scan_c(us, K_last)[0]}
            for t_i, t in enumerate(tw):
                t_id = chr(ord("A") + t_i)
                t["id"] = t_id if t["first"] in c_seeds else t_id + "prime"
            cut["tower_status_quo"] = {"centers": [{kk: t[kk] for kk in ("id", "seed", "first", "last", "count", "settled", "break_unit", "zd", "zg", "dd", "gg")} for t in tw]}
        else:
            cut["tower_status_quo"] = None
        cut["changes_from_previous"] = changes(prev, cut, k, new_att)
        cuts.append(cut)
        prev = cut
    last = cuts[-1]
    lean = []
    for c in last["centers"]:
        zdd, zgg = c["_zn_dd_gg_raw"]
        z = seed_eval(us, c["first"])
        lean.append({"start": c["first"], "end": c["last"], "count": c["count"], "settled": c["settled"],
                     "breakIndex": c["break_unit"] if c["settled"] else -1,
                     "breakUp": bool(c["break_up"]) if c["settled"] else False,
                     "breakUp_defined": c["settled"],
                     "zd": z["ZD"]["value"], "zg": z["ZG"]["value"], "gg": zgg, "dd": zdd})
    out = {"endpoints": P, "units": us, "completed_through": K_last, "in_progress": last["in_progress"],
           "seed_attempts": all_att, "cuts": cuts, "final_scan_lean_ref": lean,
           "final_scan_lean_ref_scope": "RAW_EXTENSION_NO_UPGRADE" if raw_final else "ZN_OUTER_C_RESUME"}
    if with_tower:
        out["final_scan_tower_status_quo"] = last["tower_status_quo"]["centers"]
    strip(out)
    return out, us


def strip(o):
    if isinstance(o, dict):
        for k in [k for k in o if k.startswith("_")]:
            del o[k]
        for v in o.values():
            strip(v)
    elif isinstance(o, list):
        for v in o:
            strip(v)


def compare(path, exp, got):
    """exp=复算，got=JSON 手算账。逐叶断言；JSON 中的 notes/note 字段不比。"""
    if isinstance(exp, dict):
        check(isinstance(got, dict), "%s: 类型不符" % path)
        if not isinstance(got, dict):
            return
        keys = set(exp) | {k for k in got if k not in ("notes", "note")}
        for k in sorted(keys):
            check(k in exp and k in got, "%s.%s: 键缺失（复算=%s，JSON=%s）" % (path, k, k in exp, k in got))
            if k in exp and k in got:
                compare(path + "." + k, exp[k], got[k])
    elif isinstance(exp, list):
        check(isinstance(got, list) and len(got) == len(exp), "%s: 列表长度不符 %r vs %r" % (path, exp, got))
        if isinstance(got, list):
            for i, (a, b) in enumerate(zip(exp, got)):
                compare("%s[%d]" % (path, i), a, b)
    else:
        check(type(exp) == type(got) and exp == got, "%s: 复算=%r JSON=%r" % (path, exp, got))


# ---------------------------------------------------------------- 中性域
def neutral_domain(name, us, s, decl):
    last = s["cuts"][-1]
    for u in us:
        check(u["lo"] < u["hi"], "%s N4 零幅段 u%d" % (name, u["idx"]))
    for k in range(1, len(us)):
        check(us[k]["dir"] != us[k - 1]["dir"] and us[k]["from"] == us[k - 1]["to"], "%s 端点相连交替 u%d" % (name, k))
    cs, att, _ = scan_c(us, s["completed_through"])
    for c in cs:
        if not decl.get("allow_ge9"):
            check(c["count"] <= 8, "%s N3 成员>8 %s" % (name, c["id"]))
        z = seed_eval(us, c["first"])
        zd, zg = z["ZD"]["value"], z["ZG"]["value"]
        if c["settled"]:
            j = c["break_unit"]
            check(us[j]["dir"] == c["zn_dir"], "%s N2 离开方向未与 Zn 相反（突破段非 Zn）%s" % (name, c["id"]))
        tested = c["_members"][3:] + ([c["break_unit"]] if c["settled"] else [])
        for t in tested:
            check(us[t]["hi"] != zd and us[t]["lo"] != zg, "%s N5 延伸边界相等 u%d vs %s" % (name, t, c["id"]))
    for a in att:
        if a["start"] in decl.get("designed_tangent", []):
            check(a["core_order"] == "ZD=ZG", "%s 设计相切未出现 seed(%d)" % (name, a["start"]))
        else:
            check(a["core_order"] != "ZD=ZG", "%s N5 非设计相切 seed(%d)" % (name, a["start"]))


# ---------------------------------------------------------------- 穷举性质
def zigzags(vals, length):
    """端点相连、方向严格交替的全部端点序列。"""
    for first_up in (True, False):
        def rec(seq, up):
            if len(seq) == length:
                yield list(seq)
                return
            for v in vals:
                if (up and v > seq[-1]) or ((not up) and v < seq[-1]):
                    seq.append(v)
                    yield from rec(seq, not up)
                    seq.pop()
        for v0 in vals:
            yield from rec([v0], first_up)


def properties(report):
    n_seq = n_centers = n_seed = 0
    bad = {"lemma": 0, "zn_seed_outer": 0, "two_zn_core": 0, "nonzn_internal": 0, "only_last_nonzn": 0,
           "fail_twice": 0, "fail_char": 0, "connector_len": 0, "lead_len": 0}
    hist_conn = {}
    hist_lead = {}
    # 两个域：端点值 0..6、端点数 4..8（覆盖 PREP 的 0..6/5..8）；端点值 0..5、端点数 4..9（让两中枢 + 两单元连接段可达）
    domains = [(range(0, 7), range(4, 9)), (range(0, 6), range(4, 10))]
    for vals, lengths in domains:
        for L in lengths:
            for P in zigzags(vals, L):
                n_seq += 1
                us = mk_units(P)
                K = len(us) - 1
                # seed 级性质：全部三元组
                for i in range(0, K - 1):
                    n_seed += 1
                    ev = seed_eval(us, i)
                    tri = [us[i], us[i + 1], us[i + 2]]
                    zn_lo = min(tri[0]["lo"], tri[2]["lo"])
                    zn_hi = max(tri[0]["hi"], tri[2]["hi"])
                    if (zn_lo, zn_hi) != (min(t["lo"] for t in tri), max(t["hi"] for t in tri)):
                        bad["zn_seed_outer"] += 1
                    # 020:54 两 Zn 式：ZD=max(d1,d2)、ZG=min(g1,g2)，Z1、Z2 = 第 1、3 单元
                    if (max(tri[0]["lo"], tri[2]["lo"]), min(tri[0]["hi"], tri[2]["hi"])) != (ev["ZD"]["value"], ev["ZG"]["value"]):
                        bad["two_zn_core"] += 1
                    fail = ev["result"] != "FORMED"
                    up = us[i]["dir"] == "UP"
                    char = (P[i + 3] <= P[i]) if up else (P[i + 3] >= P[i])
                    if fail != char:
                        bad["fail_char"] += 1
                    if i + 1 <= K - 2 and fail and seed_eval(us, i + 1)["result"] != "FORMED":
                        bad["fail_twice"] += 1
                # 扫描级性质
                cs, att, _ = scan_c(us, K)
                if cs:
                    hist_lead[cs[0]["first"]] = hist_lead.get(cs[0]["first"], 0) + 1
                if cs and cs[0]["first"] > 1:
                    bad["lead_len"] += 1
                for idx, c in enumerate(cs):
                    n_centers += 1
                    z = seed_eval(us, c["first"])
                    zd, zg = z["ZD"]["value"], z["ZG"]["value"]
                    mem = c["_members"]
                    if c["settled"]:
                        j = c["break_unit"]
                        if us[j]["lo"] > zg and us[j]["dir"] != "DOWN":
                            bad["lemma"] += 1
                        if us[j]["hi"] < zd and us[j]["dir"] != "UP":
                            bad["lemma"] += 1
                        if j + 1 <= K and overlaps(us[j + 1], zd, zg):
                            bad["lemma"] += 1
                        if idx + 1 < len(cs):
                            gap = cs[idx + 1]["first"] - c["last"] - 1
                            hist_conn[gap] = hist_conn.get(gap, 0) + 1
                            if not (1 <= gap <= 2):
                                bad["connector_len"] += 1
                    zdir = us[mem[0]]["dir"]
                    for t in range(3, len(mem) + 1):
                        pre = mem[:t]
                        zn = [m for m in pre if us[m]["dir"] == zdir]
                        zx = (min(us[m]["lo"] for m in zn), max(us[m]["hi"] for m in zn))
                        inner = zn + [m for m in pre[:-1] if us[m]["dir"] != zdir]
                        if (min(us[m]["lo"] for m in inner), max(us[m]["hi"] for m in inner)) != zx:
                            bad["nonzn_internal"] += 1
                        allx = (min(us[m]["lo"] for m in pre), max(us[m]["hi"] for m in pre))
                        if allx != zx and us[pre[-1]]["dir"] == zdir:
                            bad["only_last_nonzn"] += 1
    for k, v in bad.items():
        check(v == 0, "穷举性质 %s 反例数 %d" % (k, v))
    report.append("穷举域：{端点值 0..6、端点数 4..8} ∪ {端点值 0..5、端点数 4..9}，端点相连且方向严格交替的全部序列 %d 条（两域重叠部分计两次）；三元组 %d 个；C 口径中枢 %d 个" % (n_seq, n_seed, n_centers))
    for k, v in bad.items():
        report.append("  性质 %-16s 反例 %d" % (k, v))
    report.append("  连接段单元数分布 %s；首中枢前导单元数分布 %s" % (dict(sorted(hist_conn.items())), dict(sorted(hist_lead.items()))))
    return {"sequences": n_seq, "triples": n_seed, "centers": n_centers, "counterexamples": bad,
            "connector_len_hist": {str(k): v for k, v in sorted(hist_conn.items())},
            "lead_len_hist": {str(k): v for k, v in sorted(hist_lead.items())}}


# ---------------------------------------------------------------- E1
def recompute_e1(P, completed):
    us = mk_units(P)
    if len(us) < 3:
        return {"endpoints": P, "units": us, "completed": list(completed), "result": "INSUFFICIENT_INPUT",
                "vector": None, "candidate_center": False}
    if not all(completed):
        alt = us[0]["dir"] != us[1]["dir"] and us[1]["dir"] != us[2]["dir"]
        return {"endpoints": P, "units": us, "completed": list(completed), "result": "INSUFFICIENT_INPUT",
                "vector": {"completed": False, "consecutive": True, "same_level": True, "alternating": alt, "strict": None},
                "ZD": None, "ZG": None, "core_order": None, "candidate_center": False}
    ev = seed_eval(us, 0)
    z1, z2 = us[0], us[2]
    return {"endpoints": P, "units": us, "completed": list(completed), "vector": ev["vector"], "ZD": ev["ZD"],
            "ZG": ev["ZG"], "core_order": ev["core_order"],
            "two_zn_formula_020_54": {"ZD": max(z1["lo"], z2["lo"]), "ZG": min(z1["hi"], z2["hi"])},
            "result": ev["result"], "exclusion_reason": ev["exclusion_reason"], "candidate_center": False}


# ---------------------------------------------------------------- PREP 交叉核
def cross_prep(doc, report):
    if not os.path.exists(PREP_JSON):
        report.append("PREP 交叉核：未找到 %s，跳过" % PREP_JSON)
        return
    pr = json.load(open(PREP_JSON))
    diffs, n = [], 0
    def same(tag, a, b):
        nonlocal n
        n += 1
        if a != b:
            diffs.append("%s: 本稿=%r PREP=%r" % (tag, a, b))
    se = doc["seed_examples"]
    for mine, theirs in (("E1a", "a"), ("E1b", "b"), ("E1b_prime", "b_plus"), ("E1c", "c")):
        m, t = se[mine], pr["E1"][theirs]
        v = m["vector"]
        same(mine + ".vector", [v["completed"], v["consecutive"], v["same_level"], v["alternating"], v["strict"]], t["vector"])
        same(mine + ".ZD", m["ZD"]["value"], t["ZD"])
        same(mine + ".ZG", m["ZG"]["value"], t["ZG"])
        same(mine + ".core_order", m["core_order"], t["core_order"])
        same(mine + ".two_zn", m["two_zn_formula_020_54"], t["two_Zn_formula"])
    up = {c["cut"]: c for c in doc["streams"]["UP"]["cuts"]}
    for kname, t in pr["UP_named_cuts"].items():
        m = up[kname]
        mc = [{"seed": c["seed"], "members": list(range(c["first"], c["last"] + 1)), "settled": c["settled"],
               "resume": c["resume_anchor"],
               "six": {v: c["roles"][g][v]["value"] for g, vs in (("CORE", ("ZD", "ZG")), ("OUTER", ("DD", "GG")), ("INNER_EXTREMA", ("D", "G"))) for v in vs},
               "tower_dd_gg": [c["c2_contrast_all_unit_dd"], c["c2_contrast_all_unit_gg"]]} for c in m["centers"]]
        tc = [{kk: c[kk] for kk in ("seed", "members", "settled", "resume", "six", "tower_dd_gg")} for c in t["centers_C"]]
        same(kname + ".centers_C", mc, tc)
        mb = [{"seed": c["seed"], "members": list(range(c["first"], c["last"] + 1))} for c in m["tower_status_quo"]["centers"]]
        tb = [{"seed": c["seed"], "members": c["members"]} for c in t["centers_B_tower_style"]]
        same(kname + ".centers_B", mb, tb)
    fin = {a["unit"]: a for a in doc["streams"]["UP"]["cuts"][-1]["assignment"]}
    ren = {"A": "C0", "B": "C1", "PENDING": "pending"}
    for u, s in pr["UP_stream"]["assignment_final"].items():
        a = fin[int(u)]
        if a["status"] == "CENTER_MEMBER":
            mine = "CENTER_MEMBER(%s)" % ren[a["center"]]
        elif a["status"] == "CONNECTOR":
            mine = "CONNECTOR(%s->%s)" % (ren[a["left"]], ren[a["right"]])
        else:
            mine = "NOT_YET_ASSIGNED(incomplete)" if a["reason"] == "UNIT_NOT_COMPLETED" else "NOT_YET_ASSIGNED(%s)" % a["reason"]
        same("UP.K12.u%s" % u, mine, s)
    dn = doc["streams"]["DOWN"]["cuts"][-1]["centers"]
    for m, t in zip(dn, pr["DOWN_stream"]["centers_C"]):
        h = t["history"][-1]
        same("DOWN.%s.six" % m["id"], {v: m["roles"][g][v]["value"] for g, vs in (("CORE", ("ZD", "ZG")), ("OUTER", ("DD", "GG")), ("INNER_EXTREMA", ("D", "G"))) for v in vs},
             {v: h[v] for v in ("ZD", "ZG", "DD", "GG", "D", "G")})
        same("DOWN.%s.contrib" % m["id"], {v: m["roles"][g][v]["contributors"] for g, vs in (("CORE", ("ZD", "ZG")), ("OUTER", ("DD", "GG")), ("INNER_EXTREMA", ("D", "G"))) for v in vs},
             {v: h["contrib"][v] for v in ("ZD", "ZG", "DD", "GG", "D", "G")})
    report.append("PREP 交叉核（只读 %s）：比对 %d 项，不一致 %d 项" % (PREP_JSON, n, len(diffs)))
    for d in diffs:
        report.append("  差异 " + d)
    doc_expected = doc.get("cross_check_prep", {}).get("expected_differences", [])
    check(sorted(diffs) == sorted(doc_expected), "PREP 交叉核差异与 JSON 登记不符：%r" % diffs)


def mutation_selftest(doc, report):
    """自检：故意改坏 JSON 的若干值，compare 必须报错（防止比较器空转）。"""
    import copy
    global N_ASSERT, FAILS
    saved = (N_ASSERT, list(FAILS))
    caught = 0
    muts = [("UP", 6, lambda c: c["centers"][0]["roles"]["INNER_EXTREMA"]["G"].__setitem__("value", 35)),
            ("UP", 10, lambda c: c["assignment"][8].__setitem__("right", "B")),
            ("E4", 7, lambda c: c["assignment"][5].__setitem__("status", "NOT_YET_ASSIGNED")),
            ("UP", 12, lambda c: c["centers"][1]["roles"]["OUTER"]["DD"].__setitem__("contributors", [9, 10])),
            ("UP", 11, lambda c: c["tower_status_quo"]["centers"][1].__setitem__("zd", 4))]
    for name, k, f in muts:
        st = copy.deepcopy(doc["streams"][name])
        f(st["cuts"][k - 1])
        exp, _ = recompute_stream(st["endpoints"], st["completed_through"], True, False)
        before = len(FAILS)
        compare("mut", exp, st)
        caught += 1 if len(FAILS) > before else 0
    N_ASSERT, FAILS[:] = saved[0], saved[1]
    check(caught == len(muts), "变异自检：%d/%d 个故意改坏被捕获" % (caught, len(muts)))
    report.append("变异自检：%d/%d 个故意改坏被比较器捕获" % (caught, len(muts)))


# ---------------------------------------------------------------- main
def main():
    doc = json.load(open(os.path.join(HERE, "worked-examples.json")))
    report = []
    # E1
    e1_inputs = {"E1a": ([10, 40, 20, 35], (True, True, True)), "E1b": ([10, 40, 5, 10], (True, True, True)),
                 "E1b_prime": ([10, 40, 5, 11], (True, True, True)), "E1c": ([10, 40, 5, 9], (True, True, True)),
                 "E1d_two_units": ([10, 40, 20], (True, True)),
                 "E1e_third_in_progress": ([10, 40, 20, 35], (True, True, False))}
    before = N_ASSERT
    for name, (P, comp) in e1_inputs.items():
        check(doc["seed_examples"][name]["endpoints"] == P, "E1 %s 端点" % name)
        compare("seed_examples." + name, recompute_e1(P, comp), doc["seed_examples"][name])
    s = doc["seed_examples"]["E1b_stream_plus1"]
    us = mk_units(s["endpoints"])
    _, att, _ = scan_c(us, len(us) - 1)
    compare("seed_examples.E1b_stream_plus1.seed_attempts", att, s["seed_attempts"])
    report.append("E1 seed 例：%d 项断言" % (N_ASSERT - before))
    # 流
    decl = {"UP": {}, "DOWN": {}, "E4": {"designed_tangent": [5]}, "E4_DOWN": {"designed_tangent": [5]},
            "LEAD": {}, "W9": {"allow_ge9": True}}
    for name, st in doc["streams"].items():
        before = N_ASSERT
        P = st["endpoints"]
        if "mirror_of" in st:
            check(P == [45 - p for p in doc["streams"][st["mirror_of"]]["endpoints"]], "%s 镜像端点" % name)
        exp, us = recompute_stream(P, st["completed_through"], st["cuts"][0]["tower_status_quo"] is not None,
                                   st["final_scan_lean_ref_scope"] == "RAW_EXTENSION_NO_UPGRADE")
        if "mirror_of" in st:
            exp["mirror_of"] = st["mirror_of"]
            exp["mirror_map"] = "p'=45-p"
        compare("streams." + name, exp, st)
        neutral_domain(name, us, st, decl[name])
        report.append("流 %-8s 端点 %s：切面 K1..K%d，%d 项断言" % (name, P, st["completed_through"], N_ASSERT - before))
    mutation_selftest(doc, report)
    before = N_ASSERT
    pr = properties(report)
    check(doc.get("properties", {}).get("exhaustive") == pr, "JSON properties.exhaustive 与复算不符")
    cross_prep(doc, report)
    print("#1419 TB-02-E S0 oracle 复算（python3 -B oracle_check.py）")
    for line in report:
        print(line)
    print("断言总数 %d，失败 %d" % (N_ASSERT, len(FAILS)))
    for f in FAILS[:200]:
        print("FAIL " + f)
    print("RESULT " + ("PASS" if not FAILS else "FAIL"))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
