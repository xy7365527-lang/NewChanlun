#!/usr/bin/env python3
"""
BTC: 为什么 T 引擎只 +30% 而不接近理论上限？
只算三件事：1) 理论上限  2) 仓位利用率  3) 盈亏四象限拆解
用法: .venv/bin/python analysis/btc_why_30pct.py
"""
import json, math, os
import numpy as np

TRADES_F = "analysis/data_cache/t_engine_BTC_structural_trades.json"
PRICE_F  = "analysis/data_cache/btc_1m_full.json"
NPY_F    = "analysis/data_cache/_btc_closes.npy"

def banner(t): print("\n" + "=" * 64 + f"\n{t}\n" + "=" * 64)

# ---------- 加载 ----------
print("加载 trades + 价格...")
T = json.load(open(TRADES_F))
if os.path.exists(NPY_F):
    closes = np.load(NPY_F)
else:
    print("  首次: 解析330MB json并缓存为npy...")
    P = json.load(open(PRICE_F))
    closes = np.asarray(P["closes"], dtype=np.float64)
    np.save(NPY_F, closes)
N = len(closes)
trades   = T["trades"]
equity   = T["equity"]            # [bar, nav]  每1440采样
exposure = T["exposure_series"]   # [bar, short_shares, long_shares]
assert N == T["n_bars"], (N, T["n_bars"])
assert abs(float(closes[trades[0][1]]) - trades[0][2]) < 1e-6, "bar未对齐"

bh_mult = float(closes[-1] / closes[0])
strat_pct = T["final_nav"] / 100000.0 * 100 - 100
print(f"  bars={N:,}  closes {closes[0]:.0f}→{closes[-1]:.0f}")
print(f"  Buy&Hold = {(bh_mult-1)*100:+.1f}%   策略 = {strat_pct:+.1f}%")

LAD, EB, EP, XB, XP, SH, W, DEF, PART, XR, POL, ORI = range(12)

# ================================================================
# 1. 理论上限（完美后见之明）
# ================================================================
banner("1. 理论上限（完美后见之明）")

# 1a. 逐分钟完美翻转 = 完美抓每个单调段（数学恒等：望远镜相消）
r = closes[1:] / closes[:-1] - 1.0
absr = np.abs(r)
nz = absr > 0
sum_abs_r  = float(absr.sum())
log10_mult = float(np.log10(1.0 + absr[nz]).sum())
sgn = np.sign(r[nz])
seg_count = int(1 + np.count_nonzero(np.diff(sgn) != 0))
print(f"[1分钟级] 单调段数 = {seg_count:,}")
print(f"[1分钟级] Σ|涨跌幅| = {sum_abs_r*100:,.0f}%  (每分钟波动绝对值相加)")
print(f"[1分钟级] 完美每分钟翻转上限 = 10^{log10_mult:,.0f} 倍  (天文数字, 不可操作)")

# 1b. 5% ZigZag 可操作上限
def zigzag_pivots(c, thr=0.05):
    pivots = [float(c[0])]
    last_pivot = float(c[0]); extreme = float(c[0]); direction = 0
    for x in c[1:]:
        x = float(x)
        if direction == 0:
            if x >= last_pivot * (1 + thr): direction = 1; extreme = x
            elif x <= last_pivot * (1 - thr): direction = -1; extreme = x
        elif direction == 1:
            if x > extreme: extreme = x
            elif x <= extreme * (1 - thr):
                pivots.append(extreme); last_pivot = extreme; direction = -1; extreme = x
        else:
            if x < extreme: extreme = x
            elif x >= extreme * (1 + thr):
                pivots.append(extreme); last_pivot = extreme; direction = 1; extreme = x
    pivots.append(extreme)
    return np.asarray(pivots)

piv = zigzag_pivots(closes, 0.05)
ratios = piv[1:] / piv[:-1]
zz_log10 = float(np.log10(np.abs(ratios)).sum())
zz_sum_pct = float(np.abs(ratios - 1).sum()) * 100
print(f"\n[5% ZigZag] 波段数 = {len(piv)-1:,}")
print(f"[5% ZigZag] Σ|波段幅度| = {zz_sum_pct:,.0f}%")
print(f"[5% ZigZag] 完美双向操作上限 = 10^{zz_log10:,.1f} 倍  (可操作上限, 仍极大)")
print(f"\n参照: Buy&Hold = {(bh_mult-1)*100:+.1f}%  |  策略实际 = {strat_pct:+.1f}%")

# ================================================================
# 2. 仓位利用率（净敞口 vs 市场方向）
# ================================================================
banner("2. 仓位利用率（净敞口 vs 市场方向）")
n_pt = min(len(equity), len(exposure))
WIN = 1440  # 用前一日收盘判断当日涨跌
up_net, dn_net = [], []
up_long, up_short, dn_long, dn_short = [], [], [], []
abs_net_all, gross_all = [], []
correct_dir = total_dir = 0
for i in range(n_pt):
    bar = exposure[i][0]; short_sh = exposure[i][1]; long_sh = exposure[i][2]
    nav = equity[i][1]
    if equity[i][0] != bar or nav <= 0: continue
    px = float(closes[bar]) if bar < N else float(closes[-1])
    long_exp  = long_sh  * px / nav * 100
    short_exp = short_sh * px / nav * 100
    net_exp = long_exp - short_exp; gross = long_exp + short_exp
    abs_net_all.append(abs(net_exp)); gross_all.append(gross)
    if bar < WIN: continue
    mkt = float(closes[bar] - closes[bar-WIN])
    if mkt == 0: continue
    total_dir += 1
    if (mkt > 0 and net_exp > 0) or (mkt < 0 and net_exp < 0): correct_dir += 1
    if mkt > 0: up_net.append(net_exp); up_long.append(long_exp); up_short.append(short_exp)
    else:       dn_net.append(net_exp); dn_long.append(long_exp); dn_short.append(short_exp)

avg = lambda a: sum(a)/len(a) if a else 0.0
print(f"采样点(天) = {n_pt:,}   有方向的天 = {total_dir:,}")
print(f"\n涨日({len(up_net):,}天)  理想净敞口=+100%")
print(f"   实际 净敞口均值 = {avg(up_net):+6.1f}%   (多头{avg(up_long):.1f}% − 空头{avg(up_short):.1f}%)")
print(f"跌日({len(dn_net):,}天)  理想净敞口=−100%")
print(f"   实际 净敞口均值 = {avg(dn_net):+6.1f}%   (多头{avg(dn_long):.1f}% − 空头{avg(dn_short):.1f}%)")
print(f"\n方向正确率(净敞口与当日涨跌同号) = {correct_dir/total_dir*100:.1f}%  (随机=50%)")
print(f"平均总暴露 gross = {avg(gross_all):.1f}%   平均|净敞口| = {avg(abs_net_all):.1f}%")
if avg(gross_all) > 0:
    print(f"对冲比率 = 1 − |净|/gross = {(1-avg(abs_net_all)/avg(gross_all))*100:.1f}%  "
          f"(越高=多空互相抵消越多)")
print(f"\n持仓时间: 多头 {T['phys_long_bars']/N*100:.1f}% bar, "
      f"空头 {T['phys_short_bars']/N*100:.1f}% bar  → 几乎全程双向持仓")

# ================================================================
# 3. 盈亏来源四象限拆解
# ================================================================
banner("3. 盈亏来源四象限拆解")
quad = {"多头顺势(涨段持多)": [0.0, 0], "空头顺势(跌段持空)": [0.0, 0],
        "多头逆势(跌段持多)": [0.0, 0], "空头逆势(涨段持空)": [0.0, 0]}
pnl_by_lad = [0.0]*11; total_pnl = 0.0
for tr in trades:
    pol = tr[POL]; sh = tr[SH]; dpx = tr[XP] - tr[EP]
    sign = 1.0 if pol == "long" else -1.0
    pnl = sh * dpx * sign
    total_pnl += pnl; pnl_by_lad[tr[LAD]] += pnl
    if pol == "long": k = "多头顺势(涨段持多)" if dpx >= 0 else "多头逆势(跌段持多)"
    else:             k = "空头顺势(跌段持空)" if dpx <= 0 else "空头逆势(涨段持空)"
    quad[k][0] += pnl; quad[k][1] += 1

print(f"{'象限':<22}{'PnL($)':>16}{'笔数':>8}")
print("-"*48)
gain = loss = 0.0
for k, (v, n) in quad.items():
    print(f"{k:<22}{v:>16,.0f}{n:>8}")
    if v >= 0: gain += v
    else: loss += v
print("-"*48)
print(f"{'顺势毛利合计':<22}{gain:>16,.0f}")
print(f"{'逆势毛损合计':<22}{loss:>16,.0f}")
print(f"{'净 PnL':<22}{total_pnl:>16,.0f}   (NAV增量={T['final_nav']-100000:,.0f})")
print(f"\n毛利/毛损 = {gain/abs(loss):.2f}   逆势磨损吃掉顺势利润的 {abs(loss)/gain*100:.0f}%")

print("\n[校验] trade级 ladder PnL vs 引擎 pnl_by_ladder:")
eng_lad = json.load(open("analysis/data_cache/t_engine_BTC_structural.json"))["pnl_by_ladder"]
for l in range(11):
    if abs(pnl_by_lad[l]) > 1 or abs(eng_lad[l]) > 1:
        print(f"   ladder{l}: 本脚本{pnl_by_lad[l]:>14,.0f}  引擎{eng_lad[l]:>14,.0f}")

banner("结论速览")
print(f"理论上限(5%ZZ): 10^{zz_log10:.1f}倍   B&H: {(bh_mult-1)*100:+.0f}%   策略: {strat_pct:+.0f}%")
print(f"方向正确率: {correct_dir/total_dir*100:.0f}%   净敞口|涨日{avg(up_net):+.0f}% 跌日{avg(dn_net):+.0f}%|")
print(f"对冲比率: {(1-avg(abs_net_all)/avg(gross_all))*100:.0f}%   逆势磨损/顺势利润: {abs(loss)/gain*100:.0f}%")

out = {
  "bh_pct": (bh_mult-1)*100, "strat_pct": strat_pct,
  "upper_1min_log10": log10_mult, "upper_1min_sum_abs_pct": sum_abs_r*100,
  "upper_zigzag5_log10": zz_log10, "zigzag5_segments": int(len(piv)-1),
  "dir_correct_rate": correct_dir/total_dir,
  "up_net_exp": avg(up_net), "dn_net_exp": avg(dn_net),
  "avg_gross": avg(gross_all), "avg_abs_net": avg(abs_net_all),
  "hedge_ratio": 1-avg(abs_net_all)/avg(gross_all),
  "quadrant": {k: v for k, v in quad.items()},
  "gross_gain": gain, "gross_loss": loss,
}
json.dump(out, open("analysis/data_cache/btc_why_30pct_result.json", "w"),
          ensure_ascii=False, indent=2)
print("\n结果已存: analysis/data_cache/btc_why_30pct_result.json")
