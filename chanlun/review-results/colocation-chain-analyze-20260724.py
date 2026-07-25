#!/usr/bin/env python3
"""T5a（#207 方向退役）阶段 B 分析脚本：新链（同点位共置/去方向）vs 旧链（T3 方向过滤）逐候选对照。

090 纪律：全量测量（非抽样推断）；确证/未确证分项；本脚本对两个合并 dump 重算报告全部数字；
断言失败即非零退出。只读两个 jsonl，零文件写入、零 git mutation、零 rust 源码接触。

输入（全部只读，与本脚本同目录）：
  typed-none-strict-chain-dump-candidates-20260723.jsonl  = 旧链基线（T3 #172 阶段 B 交付，4875 行，
    方向过滤语义；含 union/single/old_arm/xzd/base shadow 列与 dir_witness 见证列）
  colocation-chain-dump-candidates-20260724.jsonl        = 新链 dump（T5a 阶段 A 重放产出三单窗
    /tmp/t5a_chain_dump_{p3fold,wf7,wf8}.jsonl 的逐字节合并拷贝，4875 行；去方向语义，
    levels 无 dir_witness，新增 anchor 字段）
  typed-none-joint-dump-candidates-20260722.jsonl         = #164 并集时代逐候选 dump（仅 wf7 结算面
    的 typed_none 口径逐位对读用；T3 报告 §1.2 已证 wf7 逐行对齐）

阶段 A 证据（不在本脚本重算范围，报告引用）：/tmp/t5a_evidence/redline_report.md（+
redline_compare.py，本报告作者复跑退出码 0）——不变面（STATS/INDEX 13 项/tower MD5×3/
EXIT 零行/候选流逐行键对齐）、index_builds 足迹、trades 逐笔归因。
"""
import json, collections, sys, os

WS = os.path.dirname(os.path.abspath(__file__))
OLD_PATH = f"{WS}/typed-none-strict-chain-dump-candidates-20260723.jsonl"
NEW_PATH = f"{WS}/colocation-chain-dump-candidates-20260724.jsonl"
J164_PATH = f"{WS}/typed-none-joint-dump-candidates-20260722.jsonl"
TAGS = ["p3fold", "wf7", "wf8"]
ROWS_EXP = {"p3fold": 1633, "wf7": 1724, "wf8": 1518}

old_all = [json.loads(l) for l in open(OLD_PATH)]
new_all = [json.loads(l) for l in open(NEW_PATH)]
j164 = [json.loads(l) for l in open(J164_PATH)]

N_OK = 0
FAIL = []
def check(name, cond, note=""):
    global N_OK
    ok = bool(cond)
    N_OK += ok
    if not ok:
        FAIL.append(name)
    print(("  ✓ " if ok else "  ✗ ") + name + (f"（{note}）" if note else ""))
    return ok

key = lambda r: (r["bar"], r["level"], r["source_index"], r["dir"])
OLD = {t: [r for r in old_all if r["window"] == t] for t in TAGS}
NEW = {t: [r for r in new_all if r["window"] == t] for t in TAGS}
PAIRS = {t: list(zip(OLD[t], NEW[t])) for t in TAGS}

# ════════════════ §1 corpus 基线 + 对齐验证 ════════════════
print("══ §1 corpus 基线 + 对齐验证 ══")
check("旧 dump 4875 行 / 新 dump 4875 行", len(old_all) == 4875 == len(new_all), f"{len(old_all)}/{len(new_all)}")
for t in TAGS:
    check(f"{t} 分窗行数 {ROWS_EXP[t]} == {ROWS_EXP[t]}",
          len(OLD[t]) == ROWS_EXP[t] == len(NEW[t]), f"{len(OLD[t])}/{len(NEW[t])}")
DUP_EXP = {"p3fold": 64, "wf7": 59, "wf8": 66}   # T3 报告 §1.2：L0 同脚双发重复键对
for t in TAGS:
    kc = collections.Counter(key(r) for r in OLD[t])
    dups = sum(1 for v in kc.values() if v > 1)
    align = sum(1 for o, n in PAIRS[t] if key(o) == key(n))
    price_eq = sum(1 for o, n in PAIRS[t] if o["price"] == n["price"])
    check(f"{t} 键四元组非唯一（重复键 {dups} 对）⟹ 按窗内行序对齐", dups == DUP_EXP[t], f"{dups}")
    check(f"{t} 候选流逐行键对齐 {align}/{ROWS_EXP[t]}（含 price 逐行一致）",
          align == ROWS_EXP[t] and price_eq == ROWS_EXP[t])
# 新 dump 形态：dir_witness 退役、anchor 字段语义
check("新 dump levels 无 dir_witness 字段（方向见证装置退役）",
      all("dir_witness" not in g for r in new_all for g in r["chain"]["levels"]))
check("新 dump 无 union/single/old_arm/xzd/base 列（shadow 装置未随迁）",
      all(not any(k in r for k in ("union", "single", "old_arm", "xzd", "base")) for r in new_all))
check("anchor is None ⟺ chain.top is None（4875 行）",
      all((r.get("anchor") is None) == (r["chain"]["top"] is None) for r in new_all))
res = [r for r in new_all if r.get("anchor") is not None]
check(f"锚已解行 anchor == source_index（{len(res)}/{len(res)}）",
      all(r["anchor"] == r["source_index"] for r in res))
topnone = collections.Counter(t for t in TAGS for r in NEW[t] if r["chain"]["top"] is None)
check("新链 top=None 36/17/30", (topnone["p3fold"], topnone["wf7"], topnone["wf8"]) == (36, 17, 30),
      str(dict(topnone)))

# ════════════════ §2 链裁决迁移矩阵 + admit/channel 平账 ════════════════
print("\n══ §2 新旧链裁决迁移矩阵 + admit/channel 平账 ══")
VT_EXP = {
    "p3fold": {("no_chain","no_chain"):1491, ("pass","pass"):24, ("reject","reject"):76,
               ("no_chain","reject"):34, ("pass","reject"):7, ("reject","pass"):1},
    "wf7":    {("no_chain","no_chain"):1591, ("pass","reject"):7, ("no_chain","reject"):41,
               ("pass","pass"):32, ("reject","reject"):52, ("reject","pass"):1},
    "wf8":    {("no_chain","no_chain"):1450, ("pass","pass"):33, ("pass","reject"):3,
               ("no_chain","reject"):7, ("no_chain","pass"):1, ("reject","reject"):24},
}
PASS_EXP_OLD = {"p3fold": 31, "wf7": 39, "wf8": 36}
PASS_EXP_NEW = {"p3fold": 25, "wf7": 33, "wf8": 34}
FLIP_EXP = {"p3fold": 42, "wf7": 49, "wf8": 11}
ADMIT_D_EXP = {"p3fold": -23, "wf7": -33, "wf8": -7}   # == 阶段 A after STATS.admitted Δ（redline 报告）
for t in TAGS:
    vt = collections.Counter((o["chain"]["verdict"], n["chain"]["verdict"]) for o, n in PAIRS[t])
    check(f"{t} 裁决迁移矩阵 == 阶段 A", dict(vt) == VT_EXP[t], f"{dict(vt)}")
    check(f"{t} 无 reject→no_chain / pass→no_chain（逆向迁移零发生）",
          vt.get(("reject","no_chain"), 0) == 0 and vt.get(("pass","no_chain"), 0) == 0)
    op = sum(1 for o, n in PAIRS[t] if o["chain"]["verdict"] == "pass")
    np_ = sum(1 for o, n in PAIRS[t] if n["chain"]["verdict"] == "pass")
    check(f"{t} chain_pass 旧 {PASS_EXP_OLD[t]} → 新 {PASS_EXP_NEW[t]}（净 {np_-op:+d}）",
          op == PASS_EXP_OLD[t] and np_ == PASS_EXP_NEW[t])
    flips = [(o, n) for o, n in PAIRS[t] if o["admit"] != n["admit"] or o["channel"] != n["channel"]]
    check(f"{t} admit/channel 翻转 {FLIP_EXP[t]} 例（阶段 A 口径）", len(flips) == FLIP_EXP[t])
    check(f"{t} 翻转全部伴随链裁决迁移（同裁决翻转 0）",
          all(o["chain"]["verdict"] != n["chain"]["verdict"] for o, n in flips))
    d = sum((1 if n["admit"] else 0) - (1 if o["admit"] else 0) for o, n in flips)
    check(f"{t} admit 翻转净差 {d:+d} == 阶段 A after STATS.admitted Δ（平账）", d == ADMIT_D_EXP[t])
FLIP_CLS_EXP = {
    "p3fold": {("no_chain→reject","cert_none→nest_n_delta_false"):2, ("no_chain→reject","xzd_gate_fail→nest_n_delta_false"):15,
               ("no_chain→reject","xzd_pass→nest_n_delta_false"):17, ("pass→reject","nest_pass→nest_n_delta_false"):7,
               ("reject→pass","nest_n_delta_false→nest_pass"):1},
    "wf7":    {("no_chain→reject","xzd_gate_fail→nest_n_delta_false"):14, ("no_chain→reject","xzd_pass→nest_n_delta_false"):27,
               ("pass→reject","nest_pass→nest_n_delta_false"):7, ("reject→pass","nest_n_delta_false→nest_pass"):1},
    "wf8":    {("no_chain→pass","xzd_pass→nest_pass"):1, ("no_chain→reject","cert_none→nest_n_delta_false"):1,
               ("no_chain→reject","xzd_gate_fail→nest_n_delta_false"):2, ("no_chain→reject","xzd_pass→nest_n_delta_false"):4,
               ("pass→reject","nest_pass→nest_n_delta_false"):3},
}
for t in TAGS:
    fc = collections.Counter((f'{o["chain"]["verdict"]}→{n["chain"]["verdict"]}',
                              f'{o["channel"]}→{n["channel"]}')
                             for o, n in PAIRS[t] if o["admit"] != n["admit"] or o["channel"] != n["channel"])
    check(f"{t} 翻转（裁决迁移 × 通道迁移）分类 == 阶段 A", dict(fc) == FLIP_CLS_EXP[t])

# 按候选级分层（新 pass 首次突破 L0 候选层）
NEWPASS_LVL_EXP = {"p3fold": {0: 24, 1: 1}, "wf7": {0: 32, 1: 1}, "wf8": {0: 34}}
print("  ── 按候选级分层（n / 旧pass / 新pass / 恢复 / 延伸 / 解放 / unchanged）──")
LVL_TAB = {}
for t in TAGS:
    for L in sorted({n["level"] for o, n in PAIRS[t]}):
        rows = [(o, n) for o, n in PAIRS[t] if n["level"] == L]
        rec = [len(rows),
               sum(1 for o, n in rows if o["chain"]["verdict"] == "pass"),
               sum(1 for o, n in rows if n["chain"]["verdict"] == "pass"),
               sum(1 for o, n in rows if o["chain"]["verdict"] != "pass" and n["chain"]["verdict"] == "pass"),
               sum(1 for o, n in rows if o["chain"]["verdict"] == "pass" and n["chain"]["verdict"] == "reject"),
               sum(1 for o, n in rows if o["chain"]["verdict"] == "no_chain" and n["chain"]["verdict"] == "reject"),
               sum(1 for o, n in rows if o["chain"]["verdict"] == n["chain"]["verdict"])]
        LVL_TAB[(t, L)] = rec
        print(f"    {t} L{L}: n={rec[0]} 旧pass={rec[1]} 新pass={rec[2]} 恢复={rec[3]} 延伸={rec[4]} 解放={rec[5]} unchanged={rec[6]}")
for t in TAGS:
    nl = collections.Counter(n["level"] for o, n in PAIRS[t] if n["chain"]["verdict"] == "pass")
    check(f"{t} 新 pass 候选级分布 {dict(sorted(nl.items()))}", dict(nl) == NEWPASS_LVL_EXP[t])
check("新链 pass 突破 L0 候选层恰 2 例（p3fold/wf7 各 1，L1 候选）",
      sum(v for (t, L), v in ((k, r[2]) for k, r in LVL_TAB.items()) if L >= 1 for v in [v]) == 2)

# ════════════════ §3 条件①：全量 4875 行四类归因，零未解释 ════════════════
print("\n══ §3 条件① 迁移归因分类账（全量四类 + 机制断言）══")
def cls(o, n):
    ov, nv = o["chain"]["verdict"], n["chain"]["verdict"]
    if nv == "pass" and ov != "pass": return "方向自由化恢复"
    if ov == "pass" and nv == "reject": return "链顶延伸致拒"
    if ov == "no_chain" and nv == "reject": return "存在性解放但闭合仍缺"
    if ov == nv: return "unchanged"
    return "UNCLASSIFIED"
CLS_EXP = {  # (恢复, 延伸, 解放, unchanged)
    "p3fold": (1, 7, 34, 1591), "wf7": (1, 7, 41, 1675), "wf8": (1, 3, 7, 1507)}
BY_CLS = {c: [(o, n) for o, n in (p for t in TAGS for p in PAIRS[t]) if cls(o, n) == c]
          for c in ("方向自由化恢复", "链顶延伸致拒", "存在性解放但闭合仍缺", "unchanged", "UNCLASSIFIED")}
for t in TAGS:
    cc = collections.Counter(cls(o, n) for o, n in PAIRS[t])
    exp = CLS_EXP[t]
    check(f"{t} 四类计数 恢复{exp[0]}/延伸{exp[1]}/解放{exp[2]}/unchanged{exp[3]}，未分类 0",
          (cc["方向自由化恢复"], cc["链顶延伸致拒"], cc["存在性解放但闭合仍缺"], cc["unchanged"]) == exp
          and cc["UNCLASSIFIED"] == 0, f"{dict(cc)}")
check("全量 4875 = 3 + 17 + 82 + 4773，UNCLASSIFIED 全局 0",
      len(BY_CLS["方向自由化恢复"]) == 3 and len(BY_CLS["链顶延伸致拒"]) == 17
      and len(BY_CLS["存在性解放但闭合仍缺"]) == 82 and len(BY_CLS["unchanged"]) == 4773
      and len(BY_CLS["UNCLASSIFIED"]) == 0)

# ── 类 1 方向自由化恢复（3 例）：机制断言 + 全清单 ──
print("  ── 类 1 方向自由化恢复（A 类回归，3 例全清单）──")
for o, n in BY_CLS["方向自由化恢复"]:
    wit = {g["dir_witness"] for g in o["chain"]["levels"]}
    print(f"    [{n['window']}] bar={n['bar']} lvl={n['level']} si={n['source_index']} dir={n['dir']} "
          f"price={n['price']} anchor={n['anchor']}")
    print(f"      旧: verdict={o['chain']['verdict']} top={o['chain']['top']} cdt={o['chain']['closed_down_to']} "
          f"gap={o['chain']['first_gap']} witness={sorted(wit)}")
    for g in o["chain"]["levels"]:
        print(f"        old L{g['level']}(ev{g['event_level']}): {g['status']} gap={g['gap']} "
              f"certs={g['certs']} clean={g['clean']} rungs={g['rungs']} wit={g['dir_witness']}")
    print(f"      新: verdict={n['chain']['verdict']} top={n['chain']['top']} cdt={n['chain']['closed_down_to']} "
          f"gap={n['chain']['first_gap']}")
    for g in n["chain"]["levels"]:
        print(f"        new L{g['level']}(ev{g['event_level']}): {g['status']} gap={g['gap']} "
              f"certs={g['certs']} clean={g['clean']} rungs={g['rungs']}")
    print(f"      通道 {o['channel']}→{n['channel']} admit {o['admit']}→{n['admit']}；"
          f"旧 shadow single={o.get('single')} union_merged={o['union']['merged_pass']} hit={o['union']['hit_levels']}")
check("类 1 全部 3 例旧链带方向压制见证（layer_only_opposite / certs_only_opposite）",
      all(any(g["dir_witness"] in ("layer_only_opposite", "certs_only_opposite") for g in o["chain"]["levels"])
          for o, n in BY_CLS["方向自由化恢复"]))
check("类 1 全部 3 例新链全级闭合（pass 且 first_gap 无、cdt=0）",
      all(n["chain"]["first_gap"] is None and n["chain"]["closed_down_to"] == 0
          and all(g["status"] == "closed" for g in n["chain"]["levels"])
          for o, n in BY_CLS["方向自由化恢复"]))
check("类 1 旧裁决构成 == 阶段 A（reject→pass 1/1/0 + no_chain→pass 0/0/1）",
      collections.Counter((n["window"], o["chain"]["verdict"]) for o, n in BY_CLS["方向自由化恢复"])
      == collections.Counter({("p3fold","reject"):1, ("wf7","reject"):1, ("wf8","no_chain"):1}))

# ── 类 2 链顶延伸致拒（17 例）：机制断言 + 全清单（§5.5 展开）──
EXT = BY_CLS["链顶延伸致拒"]
check("类 2 全部 17 例旧 top=0（旧单级链确认）",
      all(o["chain"]["top"] == 0 for o, n in EXT))
check("类 2 全部 17 例新 top≥1 且首 gap 恰在新顶、首因全 missing、cdt None",
      all(n["chain"]["top"] >= 1 and n["chain"]["first_gap"]["level"] == n["chain"]["top"]
          and n["chain"]["first_gap"]["kind"] == "missing" and n["chain"]["closed_down_to"] is None
          for o, n in EXT))
check("类 2 全部 17 例新链 L0 仍 closed（旧证据保持，拒因纯在延伸段）",
      all(n["chain"]["levels"][0]["status"] == "closed" for o, n in EXT))
nt = collections.Counter(n["chain"]["top"] for o, n in EXT)
check("类 2 新顶分布 {1:9, 2:3, 3:3, 4:2}", dict(nt) == {1: 9, 2: 3, 3: 3, 4: 2}, str(dict(nt)))
gs = collections.Counter(n["chain"]["levels"][n["chain"]["first_gap"]["level"]]["status"] for o, n in EXT)
check("类 2 首缺级底质 missing_cert 16 + missing_causal 1", dict(gs) == {"missing_cert": 16, "missing_causal": 1})

# ── 类 3 存在性解放但闭合仍缺（82 例）：机制断言 ──
LIB = BY_CLS["存在性解放但闭合仍缺"]
check("类 3 全部 82 例旧链零闭合（no_chain 本义）",
      all(all(g["status"] != "closed" for g in o["chain"]["levels"]) for o, n in LIB))
check("类 3 全部 82 例新链 ≥1 级闭合但未全闭（reject 本义）",
      all(any(g["status"] == "closed" for g in n["chain"]["levels"]) and n["chain"]["first_gap"] is not None
          for o, n in LIB))
fg3 = collections.Counter(n["chain"]["first_gap"]["kind"] for o, n in LIB)
check("类 3 新首因 missing 68 / broken 14", dict(fg3) == {"missing": 68, "broken": 14}, str(dict(fg3)))
wit3 = collections.Counter()
for o, n in LIB:
    gl = n["chain"]["first_gap"]["level"]
    wit3[o["chain"]["levels"][gl]["dir_witness"] if gl < len(o["chain"]["levels"]) else "<旧级别不可见>"] += 1
check("类 3 残余缺口的方向相关性：agree 62 / 旧级别不可见 19 / certs_only_opposite 1",
      dict(wit3) == {"agree": 62, "<旧级别不可见>": 19, "certs_only_opposite": 1}, str(dict(wit3)))

# ── unchanged（4773）：探针同/异分层 + 探针不变 ⟹ admit/channel 一致 ──
PROBE_SAME_EXP = {"p3fold": 1431, "wf7": 1382, "wf8": 1345}
PROBE_DIFF_SAMEV_EXP = {"p3fold": 160, "wf7": 293, "wf8": 162}
def probe_same(o, n):
    oc, nc = o["chain"], n["chain"]
    return (oc["verdict"] == nc["verdict"] and oc.get("top") == nc.get("top")
            and oc.get("closed_down_to") == nc.get("closed_down_to") and oc.get("first_gap") == nc.get("first_gap")
            and [{k: v for k, v in g.items() if k != "dir_witness"} for g in oc["levels"]] == nc["levels"]
            and o.get("price") == n.get("price"))
for t in TAGS:
    ps = sum(1 for o, n in PAIRS[t] if probe_same(o, n))
    pd_sv = sum(1 for o, n in PAIRS[t] if not probe_same(o, n) and o["chain"]["verdict"] == n["chain"]["verdict"])
    check(f"{t} 探针逐字节不变 {PROBE_SAME_EXP[t]}；探针变但裁决同 {PROBE_DIFF_SAMEV_EXP[t]}",
          ps == PROBE_SAME_EXP[t] and pd_sv == PROBE_DIFF_SAMEV_EXP[t])
    check(f"{t} 探针不变 ⟹ admit/channel 双一致（阶段 A(a) 复算）",
          all(o["admit"] == n["admit"] and o["channel"] == n["channel"]
              for o, n in PAIRS[t] if probe_same(o, n)))

# ── 单调性（方向退役只增不减）──
RANK = {"missing_existence": 0, "missing_causal": 1, "missing_cert": 1, "closed": 2}
viol = [(n["window"], n["bar"], g0["level"]) for o, n in (p for t in TAGS for p in PAIRS[t])
        for g0, g1 in zip(o["chain"]["levels"], n["chain"]["levels"])
        if g1["certs"] < g0["certs"] or g1["clean"] < g0["clean"] or RANK[g1["status"]] < RANK[g0["status"]]]
check("反单调案例 = 0（certs/clean 逐级不减、底质格序不降；阶段 A(b) 复算）", len(viol) == 0, str(viol[:3]))

# ── 阶段 A 探针机制计数复算（同 redline_compare.py 口径）──
MECH_EXP = {
    "p3fold": {"L0:existence_liberated":66, "L2:cert_pool_merged":13, "L2:status:missing_cert→missing_causal":13,
               "L0:cert_pool_merged":30, "L0:status:missing_cert→missing_causal":12, "L1:cert_pool_merged":22,
               "L1:status:missing_cert→missing_causal":13, "L1:existence_liberated":16,
               "L0:status:missing_cert→closed":6, "L2:existence_liberated":2},
    "wf7":    {"L0:cert_pool_merged":38, "L0:status:missing_cert→missing_causal":11, "L0:existence_liberated":85,
               "L1:existence_liberated":44, "L1:cert_pool_merged":27, "L0:status:missing_cert→closed":12,
               "L1:status:missing_cert→missing_causal":17, "L3:cert_pool_merged":64,
               "L3:status:missing_cert→missing_causal":64, "L3:existence_liberated":6,
               "L1:status:missing_cert→closed":1, "L2:cert_pool_merged":17,
               "L2:status:missing_cert→missing_causal":17, "L2:existence_liberated":16},
    "wf8":    {"L0:existence_liberated":53, "L0:cert_pool_merged":20, "L0:status:missing_cert→closed":1,
               "L0:status:missing_cert→missing_causal":16, "L1:cert_pool_merged":6,
               "L1:status:missing_cert→missing_causal":6, "L2:existence_liberated":4, "L1:existence_liberated":9,
               "L2:cert_pool_merged":31, "L2:status:missing_cert→closed":2,
               "L2:status:missing_cert→missing_causal":29},
}
for t in TAGS:
    mech = collections.Counter()
    for o, n in PAIRS[t]:
        if probe_same(o, n): continue
        oc, nc = o["chain"], n["chain"]
        reasons = []
        if o.get("price") != n.get("price"): reasons.append("price")
        if oc.get("top") is None and nc.get("top") is not None: reasons.append("newly_eval")
        if oc.get("top") != nc.get("top"): reasons.append("top")
        if len(oc["levels"]) != len(nc["levels"]): reasons.append("interval")
        if not all(g1["certs"] >= g0["certs"] and g1["clean"] >= g0["clean"]
                   for g0, g1 in zip(oc["levels"], nc["levels"])): reasons.append("shrunk")
        if reasons: continue
        sub = []
        for g0, g1 in zip(oc["levels"], nc["levels"]):
            if g0["status"] == "missing_existence" and g1["status"] != "missing_existence":
                sub.append(f"L{g0['level']}:existence_liberated")
            if g1["certs"] > g0["certs"] or g1["clean"] > g0["clean"]:
                sub.append(f"L{g0['level']}:cert_pool_merged")
            if g0["status"] != g1["status"] and g0["status"] != "missing_existence":
                sub.append(f"L{g0['level']}:status:{g0['status']}→{g1['status']}")
        mech.update(sub if sub else ["probe_field_only"])
    check(f"{t} 阶段 A 探针机制计数复算一致（存在性解放 L0/L1 等 10-13 项）",
          dict(mech) == MECH_EXP[t], f"{len(mech)} 项")

# ── top 迁移（含 None）：延伸规模与锚新解 ──
TT_EXP = {
    "p3fold": {(0,0):917,(0,1):26,(0,2):25,(0,3):6,(1,1):270,(1,2):11,(1,3):6,(2,2):255,(2,3):1,(3,3):71,
               (None,0):1,(None,1):6,(None,2):2,(None,None):36},
    "wf7":    {(0,0):914,(0,1):24,(0,2):19,(0,3):4,(0,4):15,(1,1):276,(1,2):11,(1,3):5,(1,4):18,(2,2):201,
               (2,3):22,(3,3):59,(3,4):5,(4,4):128,(None,1):6,(None,None):17},
    "wf8":    {(0,0):924,(0,1):32,(0,2):10,(0,3):9,(1,1):315,(1,3):11,(2,2):72,(2,3):1,(3,3):114,(None,None):30},
}
for t in TAGS:
    tt = collections.Counter((o["chain"]["top"], n["chain"]["top"]) for o, n in PAIRS[t])
    check(f"{t} top 迁移矩阵（含 None）", dict(tt) == TT_EXP[t])
    n2s = sum(v for (a, b), v in tt.items() if a is None and b is not None)
    s2n = sum(v for (a, b), v in tt.items() if a is not None and b is None)
    ext_up = sum(v for (a, b), v in tt.items() if a is not None and b is not None and b > a)
    print(f"    {t} top None→Some {n2s}（锚/存在性新解）；Some→None {s2n}（锚丢失须 0）；同有延伸（b>a）{ext_up}")
    check(f"{t} 无 Some→None（无锚丢失/存在性回撤）", s2n == 0)

# ════════════════ §4 链谱系分布新旧对照（链顶延伸实证）══
print("\n══ §4 链谱系分布新旧对照 ══")
TOPDIST_EXP = {
    "p3fold": ({0:974, 1:287, 2:256, 3:71, None:45}, {0:918, 1:302, 2:293, 3:84, None:36}),
    "wf7":    ({0:976, 1:310, 2:223, 3:64, 4:128, None:23}, {0:914, 1:306, 2:231, 3:90, 4:166, None:17}),
    "wf8":    ({0:975, 1:326, 2:73, 3:114, None:30}, {0:924, 1:347, 2:82, 3:135, None:30}),
}
for t in TAGS:
    od = collections.Counter(o["chain"]["top"] for o, n in PAIRS[t])
    nd = collections.Counter(n["chain"]["top"] for o, n in PAIRS[t])
    check(f"{t} top_dist 旧 == redline T3 行（baseline）", dict(od) == TOPDIST_EXP[t][0], f"{dict(sorted(od.items(), key=str))}")
    check(f"{t} top_dist 新 == redline T3 行（after）", dict(nd) == TOPDIST_EXP[t][1], f"{dict(sorted(nd.items(), key=str))}")
REJ_EXP = {"p3fold": ({"missing":29, "broken":48}, {"missing":64, "broken":53}),
           "wf7":    ({"missing":31, "broken":22}, {"missing":75, "broken":25}),
           "wf8":    ({"missing":10, "broken":14}, {"missing":16, "broken":18})}
for t in TAGS:
    orj = collections.Counter(o["chain"]["first_gap"]["kind"] for o, n in PAIRS[t] if o["chain"]["verdict"] == "reject")
    nrj = collections.Counter(n["chain"]["first_gap"]["kind"] for o, n in PAIRS[t] if n["chain"]["verdict"] == "reject")
    check(f"{t} reject 首因 旧 {REJ_EXP[t][0]} / 新 {REJ_EXP[t][1]}（== redline T3 行）",
          dict(orj) == REJ_EXP[t][0] and dict(nrj) == REJ_EXP[t][1])
# broken/missing 按来源分解
for t in TAGS:
    src = collections.Counter((o["chain"]["verdict"], n["chain"]["first_gap"]["kind"])
                              for o, n in PAIRS[t] if n["chain"]["verdict"] == "reject")
    print(f"    {t} 新 reject 首因×来源: {dict(sorted(src.items()))}")
brk_src = collections.Counter((n["window"], o["chain"]["verdict"]) for o, n in (p for t in TAGS for p in PAIRS[t])
                              if n["chain"]["verdict"] == "reject" and n["chain"]["first_gap"]["kind"] == "broken")
check("新 broken 拒 96 = 旧 reject 延存 82 + 解放 cohort 14（延伸 cohort 贡献 0）",
      brk_src == collections.Counter({("p3fold","reject"):47, ("p3fold","no_chain"):6,
                                      ("wf7","reject"):21, ("wf7","no_chain"):4,
                                      ("wf8","reject"):14, ("wf8","no_chain"):4}), str(dict(brk_src)))
rr = [(o, n) for o, n in (p for t in TAGS for p in PAIRS[t])
      if o["chain"]["verdict"] == "reject" and n["chain"]["verdict"] == "reject"]
check("reject→reject 152 例首因零漂移（broken→broken 82 + missing→missing 70）",
      collections.Counter((o["chain"]["first_gap"]["kind"], n["chain"]["first_gap"]["kind"]) for o, n in rr)
      == collections.Counter({("broken","broken"):82, ("missing","missing"):70}))

# pass 谱系：多级全链闭合首次非零
print("  ── pass 谱系：多级全链闭合（T3 时代 106 例全 top=0）──")
MLC = [(o, n) for o, n in (p for t in TAGS for p in PAIRS[t])
       if n["chain"]["verdict"] == "pass" and n["chain"]["top"] != 0]
for o, n in MLC:
    print(f"    [{n['window']}] bar={n['bar']} lvl={n['level']} si={n['source_index']} dir={n['dir']} "
          f"price={n['price']} 旧={o['chain']['verdict']}(top={o['chain']['top']}) 通道 {o['channel']}→{n['channel']}")
NP_TOP = {t: collections.Counter(n["chain"]["top"] for o, n in PAIRS[t] if n["chain"]["verdict"] == "pass") for t in TAGS}
check("新 pass 92 = top0 88 + top1 4（多级闭合 4 例：p3fold 2 / wf7 2 / wf8 0）",
      dict(NP_TOP["p3fold"]) == {0:23, 1:2} and dict(NP_TOP["wf7"]) == {0:31, 1:2} and dict(NP_TOP["wf8"]) == {0:34})
check("多级闭合 4 例 = 2 个同点跨型候选对（同价 2682801000000，Short L0 候选 + Long L1 候选，p3fold/wf7 各一对）",
      sorted((n["window"], n["source_index"], n["dir"], n["level"]) for o, n in MLC)
      == [("p3fold",190252,"Long",1), ("p3fold",190252,"Short",0), ("wf7",122572,"Long",1), ("wf7",122572,"Short",0)]
      and all(n["price"] == 2682801000000 for o, n in MLC))
check("旧 pass 106 例全部 top=0（T3 §4.2 复核）",
      all(o["chain"]["top"] == 0 for o, n in (p for t in TAGS for p in PAIRS[t]) if o["chain"]["verdict"] == "pass"))

# 新链恒等式
for t in TAGS:
    news = NEW[t]
    check(f"{t} 恒等式：len(levels)==top+1（None ⟹ 空）",
          all((n["chain"]["top"] is not None and len(n["chain"]["levels"]) == n["chain"]["top"] + 1)
              or (n["chain"]["top"] is None and len(n["chain"]["levels"]) == 0) for n in news))
    check(f"{t} 恒等式：pass ⟹ cdt=0 且 first_gap 无",
          all(n["chain"]["closed_down_to"] == 0 and n["chain"]["first_gap"] is None
              for n in news if n["chain"]["verdict"] == "pass"))
    check(f"{t} 恒等式：reject ⟹（cdt None ⟺ 首因 missing）",
          all((n["chain"]["closed_down_to"] is None) == (n["chain"]["first_gap"]["kind"] == "missing")
              for n in news if n["chain"]["verdict"] == "reject"))
    check(f"{t} 恒等式：no_chain ⟹ cdt None；top None ⟹ first_gap None",
          all(n["chain"]["closed_down_to"] is None for n in news if n["chain"]["verdict"] == "no_chain")
          and all((n["chain"]["first_gap"] is None) == (n["chain"]["top"] is None)
                  for n in news if n["chain"]["verdict"] == "no_chain"))
SUB_EXP = {
    "p3fold": (2600, {"missing_cert":1814, "missing_existence":662, "closed":117, "missing_causal":7},
               2737, {"missing_cert":1905, "missing_existence":622, "closed":155, "missing_causal":55}),
    "wf7":    (3161, {"missing_cert":2014, "missing_existence":1046, "closed":95, "missing_causal":6},
               3409, {"missing_cert":2137, "missing_existence":984, "closed":138, "missing_causal":150}),
    "wf8":    (2302, {"missing_cert":1655, "missing_existence":577, "closed":60, "missing_causal":10},
               2404, {"missing_cert":1722, "missing_existence":548, "closed":68, "missing_causal":66}),
}
for t in TAGS:
    ost = collections.Counter(g["status"] for o, n in PAIRS[t] for g in o["chain"]["levels"])
    nst = collections.Counter(g["status"] for o, n in PAIRS[t] for g in n["chain"]["levels"])
    e = SUB_EXP[t]
    check(f"{t} 谱系底质 旧条目 {e[0]}（== T3 §4.2）/ 新条目 {e[2]}",
          sum(ost.values()) == e[0] and dict(ost) == e[1] and sum(nst.values()) == e[2] and dict(nst) == e[3],
          f"closed {e[1]['closed']}→{e[3]['closed']}；missing_causal {e[1]['missing_causal']}→{e[3]['missing_causal']}")

# ════════════════ §5 与 #164 三层对照（wf7 正式结算面）══
print("\n══ §5 与 #164 三层对照（wf7）══")
check("wf7 ↔ #164 候选键逐行对齐 1724/1724（沿 T3 §1.2 结论复算）",
      len(j164) == 1724 and sum(1 for a, b in zip(j164, OLD["wf7"]) if key(a) == key(b)) == 1724)
none164 = [not c["single_found"] for c in j164]
check("#164 typed_none n=1651（逐位，同 T3 口径）", sum(none164) == 1651, f"{sum(none164)}")
w7n = NEW["wf7"]
pass_idx = [i for i, r in enumerate(w7n) if r["chain"]["verdict"] == "pass"]
check("新链 wf7 确认 33/1724 = 1.91%", len(pass_idx) == 33, f"{len(pass_idx)/1724*100:.2f}%")
check("新链确认落在 typed_none 内 = 0（与 T3 同型：新增恢复 0）",
      sum(1 for i in pass_idx if none164[i]) == 0)
print(f"  三层：代理上界 423/1651=25.62%（#164）｜并集实恢 17/1651=1.03%（#164）｜"
      f"T3 链 39/1724=2.26% ｜ 新链 33/1724=1.91%（typed_none 内新增 0）")
check("③ 方向守卫：新链 33 > 并集基线 17", len(pass_idx) > 17, f"{len(pass_idx)/17:.2f}×")
# 人口构成
sg = [(o, n) for o, n in PAIRS["wf7"] if o.get("single") is True]
mo = [(o, n) for o, n in PAIRS["wf7"] if o["union"]["merged_pass"] is True and o.get("single") is not True]
vd_sg = collections.Counter(n["chain"]["verdict"] for o, n in sg)
vd_mo = collections.Counter(n["chain"]["verdict"] for o, n in mo)
check("wf7 人口：single 73 → 新链 pass 33 / reject 40；multi-only 17 → 0 再确认（11 no_chain + 6 reject）",
      len(sg) == 73 and dict(vd_sg) == {"reject": 40, "pass": 33}
      and len(mo) == 17 and dict(vd_mo) == {"no_chain": 11, "reject": 6}, f"{dict(vd_sg)} / {dict(vd_mo)}")
check("wf7 新 pass 33 全部旧 shadow union merged_pass=True（确认集 ⊆ 并集确认集）",
      all(o["union"]["merged_pass"] is True for o, n in PAIRS["wf7"] if n["chain"]["verdict"] == "pass"))
check("wf7 净降 -6 机制平账：+1 恢复（reject→pass）- 7 延伸（pass→reject）",
      sum(1 for o, n in PAIRS["wf7"] if o["chain"]["verdict"] == "reject" and n["chain"]["verdict"] == "pass") == 1
      and sum(1 for o, n in PAIRS["wf7"] if o["chain"]["verdict"] == "pass" and n["chain"]["verdict"] == "reject") == 7)
# 并集包含性（三窗）：wf8 1 例例外（090 显著标注）
print("  ── 新链确认集 ⊆ 并集确认集？（三窗）──")
for t in TAGS:
    np_ = [(o, n) for o, n in PAIRS[t] if n["chain"]["verdict"] == "pass"]
    inu = sum(1 for o, n in np_ if o["union"]["merged_pass"] is True)
    out = [(n["bar"], n["source_index"], n["dir"], o["channel"], n["channel"]) for o, n in np_
           if o["union"]["merged_pass"] is not True]
    print(f"    {t}: 新 pass {len(np_)}，⊆并集 {inu}，例外 {len(np_)-inu} {out}")
check("并集包含性：p3fold 25/25、wf7 33/33、wf8 33/34（例外 1：wf8 bar=23249 Short，xzd_pass→nest_pass）",
      all(o["union"]["merged_pass"] is True for o, n in PAIRS["p3fold"] if n["chain"]["verdict"] == "pass")
      and sum(1 for o, n in PAIRS["wf8"] if n["chain"]["verdict"] == "pass"
              and o["union"]["merged_pass"] is not True) == 1)

# ════════════════ §5.5 链顶延伸 cohort（17 例全清单）+ 方向分歧 cohort 去向 ════════════════
print("\n══ §5.5 链顶延伸 cohort（17 例全清单）══")
for o, n in sorted(EXT, key=lambda p: (p[1]["window"], p[1]["bar"])):
    fg = n["chain"]["first_gap"]
    g = n["chain"]["levels"][fg["level"]]
    print(f"    [{n['window']}] bar={n['bar']} si={n['source_index']} dir={n['dir']} lvl={n['level']} "
          f"top 0→{n['chain']['top']} 首缺 L{fg['level']}={g['status']} L0={n['chain']['levels'][0]['status']} "
          f"通道 {o['channel']}→{n['channel']}")
print("\n══ 旧方向分歧 cohort（469 例，T3 §5.5）新链去向 ══")
DIV_EXP = {"p3fold": (130, {("no_chain","no_chain"):92, ("no_chain","reject"):30, ("reject","reject"):7, ("reject","pass"):1}),
           "wf7":    (229, {("no_chain","no_chain"):182, ("no_chain","reject"):38, ("reject","reject"):8, ("reject","pass"):1}),
           "wf8":    (110, {("no_chain","no_chain"):96, ("no_chain","reject"):5, ("reject","reject"):8, ("no_chain","pass"):1})}
for t in TAGS:
    coh = [(o, n) for o, n in PAIRS[t] if any(g["dir_witness"] != "agree" for g in o["chain"]["levels"])]
    tr = collections.Counter((o["chain"]["verdict"], n["chain"]["verdict"]) for o, n in coh)
    npp = sum(1 for o, n in coh if n["chain"]["verdict"] == "pass")
    check(f"{t} 分歧 cohort {DIV_EXP[t][0]} 例（== T3 NEST_GATE_T3 行），新链转 pass {npp}（=该窗 A 类回归数）",
          len(coh) == DIV_EXP[t][0] and dict(tr) == DIV_EXP[t][1] and npp == 1, f"{dict(tr)}")
check("分歧 469 例新链确认率 3/469 = 0.64%（方向压制解除后供给约束仍在）",
      sum(1 for o, n in (p for t in TAGS for p in PAIRS[t])
          if any(g["dir_witness"] != "agree" for g in o["chain"]["levels"]) and n["chain"]["verdict"] == "pass") == 3)

# ════════════════ 汇总 ════════════════
print(f"\n══ 汇总：断言 {N_OK} 过 / {len(FAIL)} 失败 ══")
for f in FAIL: print("  FAIL:", f)
sys.exit(1 if FAIL else 0)
