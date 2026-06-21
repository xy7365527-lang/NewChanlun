"""分析 sink/recover 级别锚定与空头行为（调研诊断，一次性）。

trade11 契约: [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
               weight_at_entry, deferred_bars, partial, exit_reason, polarity]
空头盈亏: short 的 realized = shares*(entry_price - exit_price) (entry>exit 才赚)。
持有期: exit_bar - entry_bar。
"""
import json

SYMS = ['CL', 'BRN', 'DX', 'GC', 'ES', 'QQQ', 'BTC', 'OKLO']
L = {'ladder':0,'eb':1,'ep':2,'xb':3,'xp':4,'sh':5,'w':6,'def':7,'part':8,'reason':9,'pol':10}

def load(sym):
    return json.load(open(f'trading_system/data_cache/fugue_v3_{sym}.json'))

print(f"{'SYM':5} {'strat%':>8} {'bh%':>8} {'P1':>4} {'ntr':>4} {'nL':>3} {'nS':>3} "
      f"{'liq':>4} {'sink':>5} {'recv':>5} {'clr':>4} {'mobPnL':>12}")
print('-'*90)
rows = {}
for sym in SYMS:
    d = load(sym)
    rows[sym] = d
    print(f"{sym:5} {d['strat_pct']:>8.1f} {d['bh_pct']:>8.1f} {str(d['P1_ge_bh']):>4} "
          f"{d['n_trades']:>4} {d['n_long']:>3} {d['n_short']:>3} {d['liquidations']:>4} "
          f"{d['cycle_opens']:>5} {d['cycle_closes']:>5} {d['core_clears']:>4} {d['mobile_realized_pnl']:>12.0f}")

print('\n\n========== 空头 trade 行为分解（polarity=short）==========')
for sym in SYMS:
    d = rows[sym]
    trades = d['trades']
    shorts = [t for t in trades if t[L['pol']] == 'short']
    longs = [t for t in trades if t[L['pol']] == 'long']
    if not shorts:
        print(f"\n--- {sym}: 无空头 trade（n_trades={d['n_trades']}, all long）---")
        continue
    print(f"\n--- {sym} (strat {d['strat_pct']:.0f}% vs BH {d['bh_pct']:.0f}%) ---")
    # 空头盈亏 + 持有期
    spnl = [t[L['sh']]*(t[L['ep']]-t[L['xp']]) for t in shorts]  # 空头: entry-exit
    hold = [t[L['xb']]-t[L['eb']] for t in shorts]
    win = sum(1 for p in spnl if p > 0)
    lose = sum(1 for p in spnl if p < 0)
    # 按 exit_reason 分组空头
    from collections import Counter
    rc = Counter(t[L['reason']] for t in shorts)
    lc = Counter(t[L['ladder']] for t in shorts)
    print(f"  空头 {len(shorts)} 笔: 赚 {win} / 亏 {lose}; 空头总实现盈亏={sum(spnl):.0f}")
    print(f"  空头持有期(bar): min={min(hold)} median={sorted(hold)[len(hold)//2]} max={max(hold)} mean={sum(hold)/len(hold):.0f}")
    print(f"  空头 exit_reason: {dict(rc)}")
    print(f"  空头 ladder 分布: {dict(lc)}  (ladder: 3=move/L1, 4=recL2, 5=recL3 ...)")
    # 逐笔（最多 12 笔）
    print(f"  逐笔空头 [ladder|entry_bar->exit_bar (hold)|entry_px->exit_px|reason|pnl]:")
    for t in shorts[:14]:
        pnl = t[L['sh']]*(t[L['ep']]-t[L['xp']])
        pxchg = (t[L['xp']]/t[L['ep']]-1)*100
        print(f"    L{t[L['ladder']]} {t[L['eb']]:>8}->{t[L['xb']]:>8} ({t[L['xb']]-t[L['eb']]:>7}b) "
              f"{t[L['ep']]:.3f}->{t[L['xp']]:.3f} ({pxchg:+.1f}%) {t[L['reason']]:14} pnl={pnl:>10.0f}")
