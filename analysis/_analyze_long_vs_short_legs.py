"""多头腿 vs 空头腿盈亏分解 + 僵尸空头单笔识别（调研最终验证）。

假设: 深亏标的的多头腿赚钱, 全部亏损集中在 recover 死锁的僵尸空头单笔(持有>10万bar)。
"""
import json

SYMS = ['CL', 'BRN', 'DX', 'GC', 'ES', 'QQQ', 'BTC', 'OKLO']
L = {'ladder':0,'eb':1,'ep':2,'xb':3,'xp':4,'sh':5,'reason':9,'pol':10}

print(f"{'SYM':5} {'strat%':>7} | {'多头腿$':>11} {'空头腿$':>11} | {'僵尸空头$':>11} {'僵尸笔':>5} {'僵尸均持(bar)':>13}")
print('-'*95)
for sym in SYMS:
    d = json.load(open(f'trading_system/data_cache/fugue_v3_{sym}.json'))
    trades = d['trades']
    long_pnl = sum(t[L['sh']]*(t[L['xp']]-t[L['ep']]) for t in trades if t[L['pol']]=='long')
    short_pnl = sum(t[L['sh']]*(t[L['ep']]-t[L['xp']]) for t in trades if t[L['pol']]=='short')
    # 僵尸空头: 持有 > 100000 bar 的空头
    zombies = [t for t in trades if t[L['pol']]=='short' and (t[L['xb']]-t[L['eb']]) > 100000]
    zpnl = sum(t[L['sh']]*(t[L['ep']]-t[L['xp']]) for t in zombies)
    zhold = sum(t[L['xb']]-t[L['eb']] for t in zombies)/len(zombies) if zombies else 0
    print(f"{sym:5} {d['strat_pct']:>7.0f} | {long_pnl:>11.0f} {short_pnl:>11.0f} | "
          f"{zpnl:>11.0f} {len(zombies):>5} {zhold:>13.0f}")

print("\n\n===== 僵尸空头 vs 强平线验证 (深亏标的 top 亏损单) =====")
print("僵尸空头靠 c>=2*basis 强平线根本够不到 ⇒ 无止损 ⇒ MtM 浮亏到 eod\n")
for sym in ['ES','QQQ','GC']:
    d = json.load(open(f'trading_system/data_cache/fugue_v3_{sym}.json'))
    trades = d['trades']
    shorts = [t for t in trades if t[L['pol']]=='short']
    worst = sorted(shorts, key=lambda t: t[L['sh']]*(t[L['ep']]-t[L['xp']]))[:3]
    print(f"--- {sym} 最差3笔空头 ---")
    for t in worst:
        pnl = t[L['sh']]*(t[L['ep']]-t[L['xp']])
        liq_line = 2*t[L['ep']]  # 强平触发价 = 2*basis
        pxchg = (t[L['xp']]/t[L['ep']]-1)*100
        print(f"  L{t[L['ladder']]}({['bar','bi','seg','move/L1','recL2','recL3','recL4'][t[L['ladder']]]}) "
              f"持有{t[L['xb']]-t[L['eb']]:>9}bar entry={t[L['ep']]:.1f} exit={t[L['xp']]:.1f}({pxchg:+.0f}%) "
              f"强平线={liq_line:.1f}(未触及) reason={t[L['reason']]} pnl={pnl:>10.0f}")
