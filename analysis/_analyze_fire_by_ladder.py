"""各级别 nf_sell/nf_buy fire 计数 + 入场级别 + 空头僵尸验证（调研）。

关键假设验证: 深亏标的在高级别(recL2/recL3)开空, 但同级别 nf_buy 几乎不 fire
⇒ recover 死锁 ⇒ 空头僵尸。盈利标的(OKLO/CL)在低级别(move/L1)操作 ⇒ nf_buy 频繁 ⇒ 闭合健康。
"""
import json

SYMS = ['CL', 'BRN', 'DX', 'GC', 'ES', 'QQQ', 'BTC', 'OKLO']
# ladder 命名 (从 DX by_ladder 推断): 0,1=笔以下, 2=segment, 3=move(L1), 4=recL2, 5=recL3, 6=recL4
LAD_NAME = {0:'bar',1:'bi',2:'seg',3:'move/L1',4:'recL2',5:'recL3',6:'recL4',7:'recL5',8:'L6',9:'L7',10:'L8'}

for sym in SYMS:
    d = json.load(open(f'trading_system/data_cache/fugue_v3_{sym}.json'))
    fs = d['fire_sell_by_ladder']
    fb = d['fire_buy_by_ladder']
    print(f"\n===== {sym} (strat {d['strat_pct']:.0f}% / BH {d['bh_pct']:.0f}% / P1={d['P1_ge_bh']}) =====")
    print(f"  {'级别':10} {'nf_sell':>8} {'nf_buy':>8}  sell/buy比")
    for k in range(2, 8):
        if fs[k] or fb[k]:
            ratio = (fs[k]/fb[k]) if fb[k] else float('inf')
            print(f"  L{k} {LAD_NAME.get(k,'?'):8} {fs[k]:>8} {fb[k]:>8}  {ratio:>6.1f}x" if fb[k] else
                  f"  L{k} {LAD_NAME.get(k,'?'):8} {fs[k]:>8} {fb[k]:>8}  ∞ (buy=0!)")
    # by_ladder 聚合(净仓位级别分布)
    print(f"  by_ladder(净仓位汇总): {d['by_ladder']}")
