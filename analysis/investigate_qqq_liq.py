"""调查 QQQ bar 581842 附近的强平：被强平空头 voice 的完整生命周期。

编排者问："有一笔很大的空头仓位在价格上涨的时候没有平吗？具体到底是什么情况？"
回答 5 问：①哪个 voice（根/子）什么级别 ②何时何价开仓 ③开仓到强平间有无买点未平
④capital 多少、价涨多少耗尽 ⑤降成本子 voice 有无运行。

用法：PYTHONPATH=src .venv/bin/python analysis/investigate_qqq_liq.py
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR, LADDER_NAMES  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["QQQ"])
dir_flips: list = []
tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips, require_settled=True)
rtape = pack_tape(tape, dir_flips=dir_flips)
res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")

trades = res["trades"]
print(f"\n{'='*70}\nQQQ unn 全部 trades（共 {len(trades)}）——按 exit_bar 排序\n{'='*70}")
# trade = (ladder, entry_bar, entry_price, exit_bar, exit_price, shares, w, dfr, partial, reason, polarity)
for t in sorted(trades, key=lambda x: x[3]):
    lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol = t
    pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
    move = (xp / ep - 1) * 100 if ep else 0
    print(f"  [{reason:12s}] {LADDER_NAMES.get(lad, lad):8s} {pol:5s} "
          f"entry bar={eb:>7} @{ep:.2f} → exit bar={xb:>7} @{xp:.2f} "
          f"shares={sh:.1f} 价动={move:+.1f}% pnl={pnl:,.0f}")

# ── 被强平的空头 voice 完整生命周期 ──
liqs = [t for t in trades if t[9] == "liq"]
print(f"\n{'='*70}\n强平 trades（exit_reason='liq'）：{len(liqs)} 笔\n{'='*70}")
for t in liqs:
    lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol = t
    capital = sh * ep  # 1x 逐仓空头：capital = shares × entry_price
    move = (xp / ep - 1) * 100
    is_root_guess = "根(最大 shares?)" if sh == max(x[5] for x in liqs) else "子?"
    print(f"\n  ① 级别={lad}({LADDER_NAMES.get(lad, lad)}) 方向={pol} （{is_root_guess}）")
    print(f"  ② 开仓 bar={eb} @ {ep:.2f}（年={years[min(eb, len(years)-1)] if years is not None else '?'}）")
    print(f"     强平 bar={xb} @ {xp:.2f}（年={years[min(xb, len(years)-1)] if years is not None else '?'}）")
    print(f"  ④ shares={sh:.1f} capital={capital:,.0f}（=shares×entry，1x逐仓）")
    print(f"     价格 {ep:.2f}→{xp:.2f} = {move:+.1f}%（强平条件 c≥2×basis ⟺ 涨100%；"
          f"实际涨{move:.1f}%，持有 {xb-eb:,} bars）")
    # ③ 开仓→强平间，该级别 + 所有级别有无买点？
    print(f"  ③ 开仓→强平间 [{eb}, {xb}] 的买点扫描（why 没平/没回补）：")
    buy_hits = {}
    for i in range(eb, min(xb + 1, len(tape))):
        s = tape[i]
        for k in range(len(s.buy_any)):
            if s.buy_any[k]:
                buy_hits.setdefault(k, []).append(i)
    if not buy_hits:
        print(f"     无任何级别买点（区间内 buy_any 全空）⇒ 没有触发回补/翻多的买卖点")
    else:
        for k in sorted(buy_hits):
            bars = buy_hits[k]
            b1 = [i for i in bars if tape[i].buy1[k]]
            print(f"     级别{k}({LADDER_NAMES.get(k, k)}): {len(bars)} 个 buy_any bar"
                  f"（type1: {len(b1)} 个）首/末={bars[0]}/{bars[-1]}"
                  f"{'  ← 含 type1 底背驰' if b1 else ''}")
        print(f"     注：root 翻多需 type1@source≥root_ladder({lad})；子回补需 buy_any@子层。")

# ── ⑤ 降成本子 voice 运行情况 ──
spawns = res["n_nrf_spawns_by_ladder"]
recovers = sum(1 for t in trades if t[9] == "recover")
flips = res["n_nrf_root_flips_by_ladder"]
print(f"\n{'='*70}\n⑤ 降成本子 voice + 翻转情况\n{'='*70}")
print(f"  spawns（降成本子 voice 诞生）by ladder: "
      f"{[(k, LADDER_NAMES.get(k, k), spawns[k]) for k in range(len(spawns)) if spawns[k] > 0]}")
print(f"  recover（子回补）笔数: {recovers}")
print(f"  root_flips（type1 翻转）by ladder: "
      f"{[(k, flips[k]) for k in range(len(flips)) if flips[k] > 0]}")
liq_by_lad = res["n_short_liquidations_by_ladder"]
print(f"  root_entries: {sum(res['n_nrf_root_entries_by_ladder'])}  "
      f"liquidations by ladder: "
      f"{[(k, liq_by_lad[k]) for k in range(len(liq_by_lad)) if liq_by_lad[k] > 0]}")
print(f"  final_nav={res['final_nav']:,.0f}（初始 100,000）")
