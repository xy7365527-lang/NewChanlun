#!/usr/bin/env python3
"""#205 研究件：方向见证分歧 469 例机制分类 + 多级链零闭合首因分解（只呈事实，不做裁定）。

090 纪律：全量测量（非抽样推断）；确证/未确证分项标注；本脚本对合并 dump 重算的全部数字
即 resolution 评论的数字；断言失败即非零退出。

输入（全部只读）：
  chanlun/review-results/typed-none-strict-chain-dump-candidates-20260723.jsonl
    = T3（#172）阶段 B 交付的三窗合并 dump（4875 行 = p3fold 1633 + wf7 1724 + wf8 1518），
    每行含 window/migration/dir_witness/chain{top,closed_down_to,first_gap,levels[...]}/union 等。

输出：stdout = 两问分类账全部数字 + 代表例坐标；零文件写入。
"""
import json, collections, sys, os

WS = os.path.dirname(os.path.abspath(__file__))
DUMP_PATH = f"{WS}/typed-none-strict-chain-dump-candidates-20260723.jsonl"
TAGS = ["p3fold", "wf7", "wf8"]

rows = [json.loads(l) for l in open(DUMP_PATH)]

OK = []
def check(name, cond, note=""):
    OK.append((name, bool(cond), note))
    return cond

key = lambda r: (r["bar"], r["level"], r["source_index"], r["dir"])
divsig = lambda r: tuple((g["level"], g["dir_witness"]) for g in r["chain"]["levels"] if g["dir_witness"] != "agree")

# ════════════════ §0 corpus 基线断言 ════════════════
print("══ §0 corpus 基线 ══")
check("总行数 4875", len(rows) == 4875, f"{len(rows)}")
wc = collections.Counter(r["window"] for r in rows)
check("分窗 1633/1724/1518", (wc["p3fold"], wc["wf7"], wc["wf8"]) == (1633, 1724, 1518), str(dict(wc)))

# ════════════════ Q1 方向见证分歧 469 例 ════════════════
print("\n══ Q1 方向见证分歧 cohort ══")
div = [r for r in rows if divsig(r)]
dc = collections.Counter(r["window"] for r in div)
check("分歧 469 = 130+229+110", len(div) == 469 and (dc["p3fold"], dc["wf7"], dc["wf8"]) == (130, 229, 110),
      f"{len(div)} {dict(dc)}")

# ── 机制四分类（级别位置 × 见证类型；与票面候选类的映射见 resolution 评论）──
def cls(r):
    kinds = {t for _, t in divsig(r)}
    if kinds == {"layer_only_opposite"}:
        pos = {("below" if lv < r["level"] else "above" if lv > r["level"] else "own")
               for lv, t in divsig(r) if t == "layer_only_opposite"}
        return "A1-纯层分叉·全在下方" if pos == {"below"} else "A2-纯层分叉·含上方"
    if kinds == {"certs_only_opposite"}:
        return "B-纯证书跨向"
    return "C-兼型"

CLS = {t: collections.Counter(cls(r) for r in div if r["window"] == t) for t in TAGS}
TOT = collections.Counter(cls(r) for r in div)
for t in TAGS:
    print(f"  {t}: {dict(CLS[t].most_common())}")
print(f"  合计: {dict(TOT.most_common())}")
check("四类分窗计数", (CLS["p3fold"]["A1-纯层分叉·全在下方"], CLS["wf7"]["A1-纯层分叉·全在下方"], CLS["wf8"]["A1-纯层分叉·全在下方"]) == (72, 98, 55)
      and (CLS["p3fold"]["A2-纯层分叉·含上方"], CLS["wf7"]["A2-纯层分叉·含上方"], CLS["wf8"]["A2-纯层分叉·含上方"]) == (4, 5, 1)
      and (CLS["p3fold"]["B-纯证书跨向"], CLS["wf7"]["B-纯证书跨向"], CLS["wf8"]["B-纯证书跨向"]) == (51, 88, 48)
      and (CLS["p3fold"]["C-兼型"], CLS["wf7"]["C-兼型"], CLS["wf8"]["C-兼型"]) == (3, 38, 6),
      str(dict(TOT)))
check("四类合计 469", sum(TOT.values()) == 469)

# ── 分歧级次类型×底质对应（装置一致性：layer_only 只出现在 missing_existence 分支，certs_only 只出现在 missing_cert）──
ts = collections.Counter()
for r in div:
    for lv, t in divsig(r):
        g = [g for g in r["chain"]["levels"] if g["level"] == lv][0]
        ts[(t, g["status"])] += 1
print(f"  分歧级次 (类型,底质): {dict(ts.most_common())}（合计 {sum(ts.values())} 级次）")
check("layer_only⟺missing_existence 且 certs_only⟺missing_cert（551 级次全对应）",
      ts[("layer_only_opposite", "missing_existence")] == 311 and ts[("certs_only_opposite", "missing_cert")] == 240
      and sum(ts.values()) == 551 and len(ts) == 2, str(dict(ts)))

# ── 分歧级相对候选级位置 ──
pos = collections.Counter()
for r in div:
    for lv, t in divsig(r):
        pos[(t, "below" if lv < r["level"] else "own" if lv == r["level"] else "above")] += 1
print(f"  分歧级位置: {dict(pos.most_common())}")
check("layer_only 全部不在本级（own=0；锚由本级解析 ⟹ 本级存在性恒真）",
      pos[("layer_only_opposite", "own")] == 0 and pos[("layer_only_opposite", "below")] == 294
      and pos[("layer_only_opposite", "above")] == 17 and pos[("certs_only_opposite", "own")] == 158
      and pos[("certs_only_opposite", "below")] == 50 and pos[("certs_only_opposite", "above")] == 32,
      str(dict(pos)))

# ── 裁决 × 分歧 ──
vd = collections.Counter(r["chain"]["verdict"] for r in div)
print(f"  分歧内裁决: {dict(vd)}")
for t in TAGS:
    d_t = [r for r in rows if r["window"] == t and divsig(r)]
    a_t = [r for r in rows if r["window"] == t and not divsig(r)]
    p_d = sum(1 for r in d_t if r["chain"]["verdict"] == "pass")
    p_a = sum(1 for r in a_t if r["chain"]["verdict"] == "pass")
    print(f"  {t}: 分歧确认 {p_d}/{len(d_t)} = 0.00% ｜ 非分歧确认 {p_a}/{len(a_t)} = {p_a/len(a_t)*100:.2f}%")
    check(f"{t} 分歧确认率 0.00%", p_d == 0)
check("分歧 cohort 裁决 no_chain 444 + reject 25 + pass 0",
      vd["no_chain"] == 444 and vd["reject"] == 25 and vd["pass"] == 0, str(dict(vd)))
mg = collections.Counter(r["migration"] for r in div)
print(f"  分歧 cohort 迁移归因: {dict(mg.most_common())}")

# ── 候选类否证 1：L0 同脚双发型 ──
posm = collections.defaultdict(list)
for r in rows:
    posm[key(r)].append(r)
pairs = {k: v for k, v in posm.items() if len(v) > 1}
pw = collections.Counter()
for k in pairs:
    for t in TAGS:
        if any(r["window"] == t for r in posm[k]):
            pw[t] += 1
check("双发对 189 = 64+59+66 且全部恰 2 发", len(pairs) == 189 and (pw["p3fold"], pw["wf7"], pw["wf8"]) == (64, 59, 66)
      and all(len(v) == 2 for v in pairs.values()), f"{len(pairs)} {dict(pw)}")
ov = sum(1 for k, v in pairs.items() if any(divsig(r) for r in v))
indup = sum(1 for r in div if key(r) in pairs)
pass_dup = sum(1 for k, v in pairs.items() for r in v if r["chain"]["verdict"] == "pass")
print(f"  双发对 {len(pairs)}：含分歧成员的对 {ov}；分歧∩双发成员 {indup}；双发成员 pass {pass_dup}")
check("L0 同脚双发型否证：双发与分歧零交集", ov == 0 and indup == 0)

# ── 候选类不可分：前缀水位型（dump 无锚缺失字段；top=None 人口按构造无见证）──
nonetop = [r for r in rows if r["chain"]["top"] is None]
nc = collections.Counter(r["window"] for r in nonetop)
print(f"  top=None（锚不可解）{len(nonetop)} 行 {dict(nc)}；levels 全空 {all(len(r['chain']['levels']) == 0 for r in nonetop)}")
check("top=None 98 = 45+23+30 且 levels 全空（不产生见证）", len(nonetop) == 98
      and (nc["p3fold"], nc["wf7"], nc["wf8"]) == (45, 23, 30)
      and all(len(r["chain"]["levels"]) == 0 for r in nonetop))

# ── 压制反事实（只呈现）：缺口全集⊆分歧级 / 缺口全为 certs_only ──
only_div = [r for r in div if all(g["dir_witness"] != "agree" for g in r["chain"]["levels"] if g["status"] != "closed")]
cf = [r for r in div if all(g["dir_witness"] == "certs_only_opposite" for g in r["chain"]["levels"] if g["status"] != "closed")]
cfvd = collections.Counter(r["chain"]["verdict"] for r in cf)
print(f"  缺口级全集⊆分歧级: {len(only_div)}/469；缺口全为 certs_only（异向有证）: {len(cf)}/469"
      f"（裁决 {dict(cfvd)}，其中 top≥1 {sum(1 for r in cf if r['chain']['top'] and r['chain']['top'] >= 1)}）")
check("反事实计数", len(only_div) == 44 and len(cf) == 40 and cfvd["no_chain"] == 34 and cfvd["reject"] == 6)

# ── 分歧∩reject 25 例的 first_gap 与分歧级关系 ──
rej = [r for r in div if r["chain"]["verdict"] == "reject"]
fgd = collections.Counter()
for r in rej:
    gl = r["chain"]["first_gap"]["level"]
    dv = {lv for lv, _ in divsig(r)}
    fgd[("gap级分歧" if gl in dv else "gap级非分歧", r["chain"]["first_gap"]["kind"])] += 1
print(f"  分歧∩reject 25: {dict(fgd.most_common())}")
t0 = [r for r in div if r["chain"]["top"] == 0]
check("top=0 分歧 34 例全部纯 certs_only", len(t0) == 34
      and all({t for _, t in divsig(r)} == {"certs_only_opposite"} for r in t0))

# ── Q1 代表例坐标 ──
print("\n  Q1 代表例（坐标 = window/bar/候选级/source_index/dir）：")
def show(r, tag):
    lvls = " ".join(f"L{g['level']}:{g['status']}/{g['dir_witness']}/c{g['certs']}" for g in r["chain"]["levels"])
    print(f"    [{tag}] {r['window']} bar={r['bar']} L{r['level']} src={r['source_index']} {r['dir']}"
          f" top={r['chain']['top']} {r['chain']['verdict']} gap={r['chain']['first_gap']} mig={r['migration']}\n         {lvls}")
find = lambda w, b, l, s: [r for r in rows if r["window"] == w and r["bar"] == b and r["level"] == l and r["source_index"] == s][0]
show(find("p3fold", 10422, 1, 10362), "A1 层分叉·下方")
show(find("p3fold", 190306, 1, 190252), "A1 层分叉·断环拒")
show(find("p3fold", 72135, 0, 72040), "A2 层分叉·上方")
show(find("p3fold", 37366, 1, 37309), "B 证书跨向·本级")
show(find("p3fold", 58365, 0, 58311), "B 证书跨向·L0单级")
show(find("p3fold", 44705, 2, 44650), "C 兼型")

# ════════════════ Q2 多级链零闭合 ════════════════
print("\n══ Q2 多级链（top≥1）零闭合首因 ══")
ml = [r for r in rows if r["chain"]["top"] is not None and r["chain"]["top"] >= 1]
mc_ = collections.Counter(r["window"] for r in ml)
check("多级候选 1852 = 614+725+513", len(ml) == 1852 and (mc_["p3fold"], mc_["wf7"], mc_["wf8"]) == (614, 725, 513),
      f"{len(ml)} {dict(mc_)}")
mvd = collections.Counter(r["chain"]["verdict"] for r in ml)
check("多级候选 0 pass（154 reject + 1698 no_chain）", mvd["pass"] == 0 and mvd["reject"] == 154 and mvd["no_chain"] == 1698)
p0 = collections.Counter(r["chain"]["verdict"] for r in rows if r["chain"]["top"] == 0)
check("106 个 pass 全部 top=0（31/39/36）", p0["pass"] == 106 and
      all(sum(1 for r in rows if r["window"] == t and r["chain"]["verdict"] == "pass") == n
          for t, n in (("p3fold", 31), ("wf7", 39), ("wf8", 36))))

# ── first_gap 级别×类型交叉表 ──
cross = collections.Counter((r["chain"]["first_gap"]["level"], r["chain"]["first_gap"]["kind"]) for r in ml)
print("  first_gap{level,kind} 交叉表（合计 / p3fold / wf7 / wf8）：")
perw = {t: collections.Counter((r["chain"]["first_gap"]["level"], r["chain"]["first_gap"]["kind"]) for r in ml if r["window"] == t) for t in TAGS}
EXP = {("p3fold", 0, "broken"): 20, ("p3fold", 1, "missing"): 273, ("p3fold", 1, "broken"): 28,
       ("p3fold", 2, "missing"): 222, ("p3fold", 3, "missing"): 71,
       ("wf7", 0, "broken"): 17, ("wf7", 1, "missing"): 293, ("wf7", 1, "broken"): 5,
       ("wf7", 2, "missing"): 218, ("wf7", 3, "missing"): 64, ("wf7", 4, "missing"): 128,
       ("wf8", 0, "broken"): 13, ("wf8", 1, "missing"): 313, ("wf8", 1, "broken"): 1,
       ("wf8", 2, "missing"): 72, ("wf8", 3, "missing"): 114}
for lv in sorted({l for l, _ in cross}):
    for kd in ("missing", "broken"):
        if cross[(lv, kd)]:
            print(f"    L{lv}/{kd}: 合计 {cross[(lv, kd)]}（" + " / ".join(f"{t} {perw[t][(lv, kd)]}" for t in TAGS) + "）")
check("交叉表合计 1852 且分窗逐项 == 探查值", sum(cross.values()) == 1852
      and all(perw[t][(lv, kd)] == n for (t, lv, kd), n in EXP.items()) and len(EXP) == len([1 for t in TAGS for lv in range(5) for kd in ("missing", "broken") if perw[t][(lv, kd)]]))
# 与 T3 §3 对账：reject 首因 missing/broken == NEST_GATE_T3 行（missing 29/31/10、broken 48/22/14）
rgap = {t: collections.Counter(r["chain"]["first_gap"]["kind"] for r in ml if r["window"] == t and r["chain"]["verdict"] == "reject") for t in TAGS}
check("reject 首因 broken 48/22/14 且 missing 29/31/10（== T3 断环拒/缺环拒+自回退出）",
      all(rgap[t]["broken"] == n for t, n in (("p3fold", 48), ("wf7", 22), ("wf8", 14)))
      and all(rgap[t]["missing"] == n for t, n in (("p3fold", 29), ("wf7", 31), ("wf8", 10))),
      str({t: dict(rgap[t]) for t in TAGS}))
attop = sum(1 for r in ml if r["chain"]["first_gap"]["level"] == r["chain"]["top"])
mid = [r for r in ml if r["chain"]["first_gap"]["level"] < r["chain"]["top"]]
print(f"  first_gap==top（链顶自身为首缺）{attop}/1852 = {attop/1852*100:.1f}%；顶已闭合断在中间 {len(mid)}（kind 全 broken）")
check("断在中间 84 全 broken（== 断环拒 84）", attop == 1768 and len(mid) == 84
      and all(r["chain"]["first_gap"]["kind"] == "broken" for r in mid))

# ── 主因分解（first_gap 级处 底质×见证）──
stw = collections.Counter()
for r in ml:
    gl = r["chain"]["first_gap"]["level"]
    g = [g for g in r["chain"]["levels"] if g["level"] == gl][0]
    stw[(g["status"], "agree" if g["dir_witness"] == "agree" else "div")] += 1
print(f"  first_gap 级处 (底质,见证): {dict(stw.most_common())}")
agree_gap = stw[("missing_cert", "agree")] + stw[("missing_existence", "agree")] + stw[("missing_causal", "agree")]
div_gap = sum(v for (s, w), v in stw.items() if w == "div")
print(f"  first_gap 级：双向皆缺 agree {agree_gap}/1852 = {agree_gap/1852*100:.1f}%；分歧见证 {div_gap} = {div_gap/1852*100:.1f}%；missing_causal {stw[('missing_causal','agree')]}")
check("first_gap 级分解 1744 agree-missing + 108 div + 0 causal", agree_gap == 1744 and div_gap == 108
      and stw[("missing_causal", "agree")] == 0 and stw[("missing_cert", "agree")] == 1689
      and stw[("missing_cert", "div")] == 104 and stw[("missing_existence", "agree")] == 55 and stw[("missing_existence", "div")] == 4,
      str(dict(stw)))

# ── 全链视角：分歧压制面 ──
anydiv = [r for r in ml if any(g["dir_witness"] != "agree" for g in r["chain"]["levels"])]
solediv = [r for r in anydiv if all(g["dir_witness"] != "agree" for g in r["chain"]["levels"] if g["status"] != "closed")]
avd = collections.Counter(r["chain"]["verdict"] for r in anydiv)
print(f"  全链含分歧 {len(anydiv)}/1852（裁决 {dict(avd)}）；缺口全在分歧级仅 {len(solediv)} 例")
check("全链分歧 435（410 nochain+25 rej），纯分歧缺口 10", len(anydiv) == 435 and len(solediv) == 10
      and avd["no_chain"] == 410 and avd["reject"] == 25)

# ── 链底 L0：多级候选的 L0 级 底质×见证 ──
l0 = collections.Counter((r["chain"]["levels"][0]["status"], r["chain"]["levels"][0]["dir_witness"]) for r in ml)
l0closed = [r for r in ml if r["chain"]["levels"][0]["status"] == "closed"]
l0agree_missing = l0[("missing_cert", "agree")] + l0[("missing_existence", "agree")]
print(f"  多级候选 L0 级: {dict(l0.most_common())}")
print(f"  L0 未闭合 {len(ml)-len(l0closed)}/1852 = {(len(ml)-len(l0closed))/1852*100:.1f}%（其中双向皆缺 agree {l0agree_missing} = {l0agree_missing/1852*100:.1f}%）；"
      f"L0 已闭合 {len(l0closed)}（全部 reject）")
check("L0 级分解", l0[("missing_existence", "agree")] == 940 and l0[("missing_cert", "agree")] == 611
      and l0[("missing_existence", "layer_only_opposite")] == 209 and l0[("missing_cert", "certs_only_opposite")] == 32
      and l0[("missing_causal", "agree")] == 10 and l0[("closed", "agree")] == 50
      and all(r["chain"]["verdict"] == "reject" for r in l0closed), str(dict(l0)))

# ── 组锚跨级匹配：构造性论证 + 实证抽查 ──
res = [r for r in rows if r["chain"]["top"] is not None]
xh = sum(1 for r in res if any(g["status"] != "missing_existence" and g["level"] != r["level"] for g in r["chain"]["levels"]))
up = sum(1 for r in res if r["chain"]["top"] > r["level"])
dn = sum(1 for r in res if r["level"] >= 1 and r["chain"]["levels"] and r["chain"]["levels"][0]["status"] != "missing_existence")
sand = [(r, g) for r in ml for g in r["chain"]["levels"]
        if g["status"] == "missing_existence"
        and any(h["status"] != "missing_existence" and h["level"] < g["level"] for h in r["chain"]["levels"])
        and any(h["status"] != "missing_existence" and h["level"] > g["level"] for h in r["chain"]["levels"])]
sdiv = sum(1 for _, g in sand if g["dir_witness"] != "agree")
print(f"  跨级锚命中实证：他级存在 {xh}/{len(res)} 锚可解候选；锚向上命中(top>候选级) {up}；锚向下命中(候选≥L1 且 L0 存在) {dn}")
print(f"  夹心 missing_existence（上下级皆存在而中级缺失）{len(sand)} 级次，其中分歧见证 {sdiv}、双向皆缺 {len(sand)-sdiv}")
check("跨级锚实证计数", xh == 894 and len(res) == 4777 and up == 479 and dn == 354 and len(sand) == 385 and sdiv == 37)

# ── Q2 代表例坐标 ──
print("\n  Q2 代表例：")
show(find("p3fold", 1878, 0, 1832), "塔产不足·链顶首缺")
show(find("p3fold", 36812, 0, 36746), "断在中间·断环拒")
show(find("p3fold", 30181, 0, 30117), "L0已闭·上方首缺")
show(find("p3fold", 190306, 1, 190252), "纯分歧缺口·唯一批 10 例之一")

# ════════════════ 总检 ════════════════
print("\n══ 检查项总检 ══")
fails = [x for x in OK if not x[1]]
for name, ok, note in OK:
    print(f"  {'✓' if ok else '✗✗✗'} {name}  {note if not ok else ''}")
print(f"\n合计 {len(OK)} 项检查，失败 {len(fails)} 项")
sys.exit(1 if fails else 0)
