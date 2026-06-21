#!/usr/bin/env python3
"""核心仓 units 衰减链 + 段内 short exposure 时间线（6位精度）。"""
import json, numpy as np
BASE = "/Users/silencehan/Projects/NewChanlun/analysis"
with open(f"{BASE}/data_cache/t_engine_BTC_structural_trades.json") as f:
    td = json.load(f)
sch = td['trade_schema']; trades = td['trades']
IX = {n:i for i,n in enumerate(sch)}
def g(t,k): return t[IX[k]]
SEG_LO, SEG_HI = 2218375, 2760270
out=[]

# 1) 4 笔真实建仓在哪
out.append("1) 全运行真实建仓事件 (n_entries_by_ladder=[0,0,0,2,2,0,...]):")
# 找不属于 reduce/recover/drain 的“建仓”——用 flip(2) + 推断；实际看每个 ladder 最早出现
# 直接看 exit_reason==flip 的（这些是方向翻转开新仓）
for t in trades:
    if g(t,'exit_reason')=='flip':
        out.append(f"   flip: lad{g(t,'ladder')} {g(t,'polarity')} eb={g(t,'entry_bar')} ep={g(t,'entry_price'):.1f} xb={g(t,'exit_bar')} sh={g(t,'shares'):.6f}")

# 2) units 衰减链：把全程按 exit_bar 排序，每 ~30万 bar 取一个“当时最大在场 shares”
out.append("\n2) 仓位 units(shares) 几何衰减链（按建仓bar分窗，窗内最大shares=该期满仓规模）:")
edges=[0,200000,400000,700000,1000000,1400000,1806004,2000000,SEG_LO,2350000,2500000,SEG_HI,3000000,4625119]
closes=np.load(f"{BASE}/data_cache/_btc_closes.npy")
for i in range(len(edges)-1):
    lo,hi=edges[i],edges[i+1]
    sh=[g(t,'shares') for t in trades if lo<=g(t,'entry_bar')<hi]
    if sh:
        mx=max(sh)
        # 该窗中点价格
        mid=(lo+hi)//2
        px=float(closes[mid]) if mid<len(closes) else 0
        out.append(f"   bar[{lo:>8}-{hi:<8}] ~price${px:>8.0f}: max_units={mx:.6e}  ({'满仓notional$%.0f'%(mx*px)})")

# 3) 一个具体的高ladder空头批次：reduce/recover 逐 leg 的 units 变化（6位）
# 取段内持有最久的空头 lad6 eb=2357602
from collections import defaultdict
batches=defaultdict(list)
for t in trades:
    batches[(g(t,'ladder'),g(t,'entry_bar'),g(t,'polarity'))].append(t)
key=(6,2357602,'short')
if key in batches:
    ts=sorted(batches[key],key=lambda t:g(t,'exit_bar'))
    out.append(f"\n3) 段内持有最久空头批次 lad6 entry_bar=2357602 的 units 链:")
    tot=sum(g(t,'shares') for t in ts)
    out.append(f"   该批次总 units={tot:.6e}, leg数={len(ts)}")
    for t in ts:
        out.append(f"     xb={g(t,'exit_bar')} ep={g(t,'entry_price'):.1f} xp={g(t,'exit_price'):.1f} Δunits={g(t,'shares'):.6e} {g(t,'exit_reason')} {'赢' if g(t,'exit_price')<g(t,'entry_price') else '亏'}")

# 4) 段内 short exposure 时间线（exposure_series = [bar, long, short]）
ex=td['exposure_series']
out.append(f"\n4) 段内 short exposure 时间线 (exposure_series=[bar,long,short], 每~1440bar):")
out.append(f"   {'bar':>9} {'price':>8} {'long_exp':>10} {'short_exp':>10}")
step=td['n_bars']/len(ex)
for row in ex:
    b=row[0]
    if SEG_LO<=b<=SEG_HI and (int(b/step)%40==0):  # 抽样约每40点
        px=float(closes[b]) if b<len(closes) else 0
        out.append(f"   {b:>9} {px:>8.0f} {row[1]:>10.6f} {row[2]:>10.6f}")
# 段内 exposure 极值
seg_rows=[r for r in ex if SEG_LO<=r[0]<=SEG_HI]
le=np.array([r[1] for r in seg_rows]); se=np.array([r[2] for r in seg_rows])
out.append(f"   段内 long_exp:  min={le.min():.6f} max={le.max():.6f} mean={le.mean():.6f}")
out.append(f"   段内 short_exp: min={se.min():.6f} max={se.max():.6f} mean={se.mean():.6f}")
out.append(f"   (exposure=1.0 表示满仓; 段内最大 short_exp={se.max():.6f} = 满仓的 {se.max()*100:.4f}%)")

open(f"{BASE}/_btc_units_chain.txt","w").write("\n".join(out))
print("\n".join(out))
