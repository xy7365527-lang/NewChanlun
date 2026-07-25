#!/usr/bin/env python3
"""#210 Q3 配套：#205 首缺 L1 集中（879/1852）的底质分解。

人口与 #205 resolution 同口径（top>=1 多级候选 1852 = 614+725+513），
输入 = T3 交付合并 dump（typed-none-strict-chain-dump-candidates-20260723.jsonl，只读）。
断言锚定 #205 已发表交叉表（L1/missing 879 等），再往下分解：
  (a) first_gap=L1/missing 879 例的 L1 级底质（missing_cert/missing_existence/missing_causal）× 见证（agree/div）；
  (b) 同人口内 L2/L3/L4 missing 首缺的同构分解（对照）；
  (c) 全 1852 候选的 L1 环（非仅首缺）底质分布——L1 环缺席 vs 无证的全景；
  (d) first_gap=L1/missing 中候选级分布（L1 首缺是否主要压在 L0 候选头顶）。
"""
import json, collections, sys

PATH = "chanlun/review-results/typed-none-strict-chain-dump-candidates-20260723.jsonl"
TAGS = ("p3fold", "wf7", "wf8")
rows = [json.loads(l) for l in open(PATH)]
assert len(rows) == 4875, len(rows)
ml = [r for r in rows if r["chain"]["top"] is not None and r["chain"]["top"] >= 1]
mc_ = collections.Counter(r["window"] for r in ml)
assert len(ml) == 1852 and (mc_["p3fold"], mc_["wf7"], mc_["wf8"]) == (614, 725, 513), (len(ml), mc_)

def lvl_entry(r, lv):
    for g in r["chain"]["levels"]:
        if g["level"] == lv:
            return g
    return None

# 锚定 #205 交叉表（first_gap level×kind）
cross = collections.Counter((r["chain"]["first_gap"]["level"], r["chain"]["first_gap"]["kind"]) for r in ml)
assert cross[(1, "missing")] == 879 and cross[(2, "missing")] == 512 and cross[(3, "missing")] == 249 \
    and cross[(4, "missing")] == 128 and cross[(0, "broken")] == 50 and cross[(1, "broken")] == 34 \
    and sum(cross.values()) == 1852, cross

print("== 锚定 #205 交叉表通过（L1/missing 879；合计 1852）==\n")

# (a) first_gap=L1/missing 879：首缺级的底质×见证
fg1 = [r for r in ml if r["chain"]["first_gap"]["level"] == 1 and r["chain"]["first_gap"]["kind"] == "missing"]
st_wit = collections.Counter((lvl_entry(r, 1)["status"], lvl_entry(r, 1)["dir_witness"]) for r in fg1)
print("(a) first_gap=L1/missing 879 例：L1 级底质×见证")
for k, n in sorted(st_wit.items(), key=lambda x: -x[1]):
    print(f"    {k[0]:18s} {k[1]:28s} {n:4d}  ({n/879*100:.1f}%)")
perw = collections.Counter(r["window"] for r in fg1)
print(f"    分窗：{dict(perw)}")

# (b) 对照：L2/L3/L4 missing 首缺的底质×见证
for lv, exp in ((2, 512), (3, 249), (4, 128)):
    fg = [r for r in ml if r["chain"]["first_gap"]["level"] == lv and r["chain"]["first_gap"]["kind"] == "missing"]
    assert len(fg) == exp
    sw = collections.Counter((lvl_entry(r, lv)["status"], lvl_entry(r, lv)["dir_witness"]) for r in fg)
    print(f"(b) first_gap=L{lv}/missing {exp} 例：{dict(sw)}")

# (c) 全 1852 候选的 L1 环底质（L1 在区间内时的状态分布；top>=1 ⟹ L1 必在区间）
l1all = collections.Counter((lvl_entry(r, 1)["status"], lvl_entry(r, 1)["dir_witness"]) for r in ml)
print("\n(c) 全 1852 多级候选的 L1 环底质×见证（不论是否首缺）")
for k, n in sorted(l1all.items(), key=lambda x: -x[1]):
    print(f"    {k[0]:18s} {k[1]:28s} {n:4d}  ({n/1852*100:.1f}%)")
l1closed = sum(n for (s, w), n in l1all.items() if s == "closed")
print(f"    L1 环已闭合 {l1closed}/1852 = {l1closed/1852*100:.1f}%")

# (d) first_gap=L1/missing 的候选级分布 + 首缺即链顶占比
cl = collections.Counter(r["level"] for r in fg1)
attop = sum(1 for r in fg1 if r["chain"]["first_gap"]["level"] == r["chain"]["top"])
print(f"\n(d) first_gap=L1/missing 879 例的候选级分布：{dict(sorted(cl.items()))}")
print(f"    首缺恰为链顶（top==1）：{attop}/879 = {attop/879*100:.1f}%")
# L1 环有证（certs>0）但没过守卫/装配的比例（missing_causal + certs>0 的 missing_cert）
cpos = sum(1 for r in fg1 if lvl_entry(r, 1)["certs"] > 0)
print(f"    首缺 L1 级键域内有证（certs>0）的例数：{cpos}/879（底质分布见 (a)）")

print("\n全部断言通过，退出码 0")
sys.exit(0)
