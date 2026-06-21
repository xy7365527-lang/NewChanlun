#!/usr/bin/env python3
"""段内原始 trades 诊断：shares 量级、long/short 原始分布。不取整。"""
import json, numpy as np
BASE = "/Users/silencehan/Projects/NewChanlun/analysis"
with open(f"{BASE}/data_cache/t_engine_BTC_structural_trades.json") as f:
    td = json.load(f)
sch = td['trade_schema']; trades = td['trades']
IX = {n:i for i,n in enumerate(sch)}
def g(t,k): return t[IX[k]]
SEG_LO, SEG_HI = 2218375, 2760270

out = []
# 全局 shares 量级
allsh = np.array([g(t,'shares') for t in trades])
out.append(f"全局 shares: max={allsh.max():.6f} min={allsh.min():.6f} nonzero={int((allsh>1e-9).sum())}/{len(allsh)}")
# 全局按 polarity 的 shares
for pol in ['long','short']:
    sh = np.array([g(t,'shares') for t in trades if g(t,'polarity')==pol])
    out.append(f"  {pol}: n={len(sh)} sum_sh={sh.sum():.4f} max_sh={sh.max():.6f} nonzero={int((sh>1e-9).sum())}")

# 段内原始 trades（entry_bar 在段内）
seg = [t for t in trades if SEG_LO<=g(t,'entry_bar')<=SEG_HI]
out.append(f"\n段内 entry trades: {len(seg)}")
for pol in ['long','short']:
    sub=[t for t in seg if g(t,'polarity')==pol]
    sh=np.array([g(t,'shares') for t in sub]) if sub else np.array([0.0])
    out.append(f"  {pol}: n={len(sub)} sum_sh={sh.sum():.6f} max_sh={sh.max():.6f}")

# 全文件里 shares 最大的 10 笔（校准“满仓”是多少股）
top = sorted(trades, key=lambda t:-g(t,'shares'))[:10]
out.append("\nshares 最大的 10 笔:")
for t in top:
    out.append(f"  lad{g(t,'ladder')} {g(t,'polarity'):5s} eb={g(t,'entry_bar')} ep={g(t,'entry_price'):.1f} xb={g(t,'exit_bar')} xp={g(t,'exit_price'):.1f} sh={g(t,'shares'):.6f} {g(t,'exit_reason')}")

# 段内 shares 最大的 short 与 long
for pol in ['short','long']:
    sub=sorted([t for t in seg if g(t,'polarity')==pol], key=lambda t:-g(t,'shares'))[:8]
    out.append(f"\n段内 {pol} shares 最大 8 笔:")
    for t in sub:
        out.append(f"  lad{g(t,'ladder')} eb={g(t,'entry_bar')} ep={g(t,'entry_price'):.1f} xb={g(t,'exit_bar')} xp={g(t,'exit_price'):.1f} sh={g(t,'shares'):.6f} {g(t,'exit_reason')}")

# exposure / phys bars from header
out.append(f"\nheader: phys_short_bars={td['phys_short_bars']} phys_long_bars={td['phys_long_bars']} n_bars={td['n_bars']}")
out.append(f"n_entries_by_ladder={td['n_entries_by_ladder']}")
out.append(f"n_liquidations_by_ladder={td['n_liquidations_by_ladder']}")

open(f"{BASE}/_btc_dseg_raw.txt","w").write("\n".join(out))
print("done")
print("\n".join(out))
