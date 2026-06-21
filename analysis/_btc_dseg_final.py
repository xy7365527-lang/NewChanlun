#!/usr/bin/env python3
"""决定性诊断：PnL来源 + 仓位shares几何衰减时间线 + 段内exposure + 段内空头reduce赢/亏。"""
import json, numpy as np
BASE = "/Users/silencehan/Projects/NewChanlun/analysis"
with open(f"{BASE}/data_cache/t_engine_BTC_structural_trades.json") as f:
    td = json.load(f)
sch = td['trade_schema']; trades = td['trades']
IX = {n:i for i,n in enumerate(sch)}
def g(t,k): return t[IX[k]]
SEG_LO, SEG_HI = 2218375, 2760270
def pnl(t):
    ep,xp,sh=g(t,'entry_price'),g(t,'exit_price'),g(t,'shares')
    return (xp-ep)*sh if g(t,'polarity')=='long' else (ep-xp)*sh

out=[]
# A) 全局 PnL 重对账：分时段累计 pnl
total_pnl = sum(pnl(t) for t in trades)
out.append(f"A) 全局 sum(per-trade pnl) = {total_pnl:.2f}  (final_nav={td['final_nav']:.2f}, 起始约100000)")
# 按 exit_bar 分桶累计 pnl（看收益发生在哪个时间段）
buckets = [(0,500000,'2017-18 $3k-8k'),(500000,1000000,'2018-20 $6k-12k'),
           (1000000,1806004,'2020-21升'),(1806004,SEG_LO,'2021升至顶前'),
           (SEG_LO,SEG_HI,'★下跌段69k->15k'),(SEG_HI,5000000,'2022底之后')]
out.append("   按 exit_bar 时段的已实现 pnl:")
for lo,hi,name in buckets:
    p=sum(pnl(t) for t in trades if lo<=g(t,'exit_bar')<hi)
    pl=sum(pnl(t) for t in trades if lo<=g(t,'exit_bar')<hi and g(t,'polarity')=='long')
    ps=sum(pnl(t) for t in trades if lo<=g(t,'exit_bar')<hi and g(t,'polarity')=='short')
    out.append(f"     {name:22s}: total={p:+12.2f}  long={pl:+12.2f}  short={ps:+12.2f}")

# B) 仓位 shares 几何衰减：每个时段“在场最大 shares”
out.append("\nB) 各时段 trade shares 量级（在场仓位规模代理）:")
for lo,hi,name in buckets:
    sh=[g(t,'shares') for t in trades if lo<=g(t,'entry_bar')<hi]
    if sh:
        sh=np.array(sh)
        out.append(f"     {name:22s}: n={len(sh):4d} max_sh={sh.max():.6e} median={np.median(sh):.6e}")
    else:
        out.append(f"     {name:22s}: n=0")

# C) exposure_series / equity 在下跌段（推断采样率）
eq = td.get('equity'); ex = td.get('exposure_series')
out.append(f"\nC) equity len={len(eq) if eq else 0} exposure_series len={len(ex) if ex else 0}")
if eq and ex:
    # 推断采样：n_bars / len
    step = td['n_bars']/len(ex)
    i_lo = int(SEG_LO/step); i_hi=min(len(ex)-1,int(SEG_HI/step))
    out.append(f"   采样步长~{step:.0f} bars/点; 段 index [{i_lo},{i_hi}]")
    exseg = ex[i_lo:i_hi+1]
    # exposure 可能是 [long,short] 对或标量
    out.append(f"   exposure_series 段内首元素样本: {ex[i_lo]}")
    # 若是标量
    try:
        exarr=np.array(exseg, dtype=float)
        out.append(f"   exposure 段内: min={exarr.min():.4f} max={exarr.max():.4f} mean={exarr.mean():.4f}")
    except Exception as e:
        out.append(f"   exposure 非标量: {type(exseg[0])}  sample5={exseg[:5]}")
    out.append(f"   equity: 顶时={eq[i_lo]}  底时={eq[i_hi]}  起始={eq[0]}  末尾={eq[-1]}")

# D) 段内空头 reduce/recover：赢(exit<entry)还是亏(exit>entry)
seg_short=[t for t in trades if SEG_LO<=g(t,'entry_bar')<=SEG_HI and g(t,'polarity')=='short']
out.append(f"\nD) 段内空头 trades={len(seg_short)}")
from collections import Counter
rc=Counter(g(t,'exit_reason') for t in seg_short)
out.append(f"   exit_reason: {dict(rc)}")
win=[t for t in seg_short if g(t,'exit_price')<g(t,'entry_price')]
lose=[t for t in seg_short if g(t,'exit_price')>g(t,'entry_price')]
out.append(f"   平仓时赢(exit<entry,跌着平=空头获利): {len(win)}笔  亏(exit>entry,涨着平): {len(lose)}笔")
# 赢/亏的平均持仓bar数
def held_bars(ts): return np.mean([g(t,'exit_bar')-g(t,'entry_bar') for t in ts]) if ts else 0
out.append(f"   赢笔平均持有={held_bars(win):.0f} bar, 亏笔平均持有={held_bars(lose):.0f} bar")
# 段内空头持有时长分布
hb=np.array([g(t,'exit_bar')-g(t,'entry_bar') for t in seg_short])
out.append(f"   段内空头持有时长: median={np.median(hb):.0f} bar max={hb.max()} (段长={SEG_HI-SEG_LO})")
# 段内最长持有的空头
longest=sorted(seg_short,key=lambda t:-(g(t,'exit_bar')-g(t,'entry_bar')))[:5]
out.append("   段内持有最久的5笔空头:")
for t in longest:
    out.append(f"     lad{g(t,'ladder')} eb={g(t,'entry_bar')}(p{g(t,'entry_price'):.0f}) xb={g(t,'exit_bar')}(p{g(t,'exit_price'):.0f}) held={g(t,'exit_bar')-g(t,'entry_bar')} {g(t,'exit_reason')} {'赢' if g(t,'exit_price')<g(t,'entry_price') else '亏'}")

# E) 段内多头同样
seg_long=[t for t in trades if SEG_LO<=g(t,'entry_bar')<=SEG_HI and g(t,'polarity')=='long']
lwin=[t for t in seg_long if g(t,'exit_price')>g(t,'entry_price')]
out.append(f"\nE) 段内多头 trades={len(seg_long)}  赢(exit>entry,涨着平)={len(lwin)} 亏={len(seg_long)-len(lwin)}")
out.append(f"   段内多头 sum_pnl={sum(pnl(t) for t in seg_long):.4f}  段内空头 sum_pnl={sum(pnl(t) for t in seg_short):.4f}")

open(f"{BASE}/_btc_dseg_final.txt","w").write("\n".join(out))
print("\n".join(out))
