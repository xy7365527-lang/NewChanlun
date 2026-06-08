"""版本 I — OKLO 447K 完整数据（oklo_1m_databento.json，含盘前盘后）重跑。

动机：之前 fugue_version_i.py 用的是缺损的 333K 数据（oklo_1m_databento_full.json），
得 E=455 / I=422。完整数据 447K 上 E 版本曾跑出 +932%（见 project_oklo_data_basis_mismatch）。
本脚本在 447K 完整数据上跑 Version I，看 I 在完整数据上的表现。

口径：复用 fugue_version_i 的信号层 / 交易层 / 指标层逐字不变，只换数据加载
（447K 文件是 {"bars": [{open,high,low,close,...}]} schema，与 load_symbol 的
opens/highs/lows/closes schema 不同）。

认识论等级：L2（真实数据，OKLO 单标的）。
"""

from __future__ import annotations

import json
import time
from pathlib import Path

import fugue_version_i as vi
from fugue_alpha_diagnosis import MODE_NONE, run_swing_trading

DATA_PATH = vi.DATA_DIR / "oklo_1m_databento.json"


def load_bars_schema(path: Path):
    """{"bars": [{ts, open, high, low, close, volume}]} → opens/highs/lows/closes/years。"""
    raw = json.loads(path.read_text())
    bars = raw["bars"]
    opens = [float(b["open"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    closes = [float(b["close"]) for b in bars]
    years = [int(str(b["ts"])[:4]) for b in bars]
    return opens, highs, lows, closes, years


def main() -> None:
    print(f"加载 {DATA_PATH.name} …")
    opens, highs, lows, closes, years = load_bars_schema(DATA_PATH)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"OKLO 447K: {n:,} bars  {years[0]}–{years[-1]}  "
          f"首={closes[0]:.2f} 末={closes[-1]:.2f}  BH={bh:+.2f}%")

    t0 = time.time()
    e_signals, i_signals = vi.compute_signals_i(opens, highs, lows, closes)
    print(f"信号层 compute-once: {time.time() - t0:.1f}s")

    e_trades, _ = run_swing_trading(e_signals, MODE_NONE)
    i_trades, i_extra = vi.run_version_i(i_signals)

    em = vi.extended_metrics(e_trades, years)
    im = vi.extended_metrics(i_trades, years)

    print("\n" + "=" * 72)
    print("  OKLO 447K 完整数据 — E vs I")
    print("=" * 72)
    hdr = f"{'':>4} {'交易':>5} {'胜率':>6} {'复利%':>12} {'超额%':>12} {'夏普':>8} {'MDD%':>9} {'盈亏比':>7}"
    print(hdr)
    for tag, m in (("E", em), ("I", im)):
        print(f"{tag:>4} {m['n']:>5d} {m['win_rate']:>5.0f}% "
              f"{m['total_compound']:>+12.2f} {m['total_compound']-bh:>+12.2f} "
              f"{m['sharpe']:>+8.3f} {m['max_dd']:>+9.2f} {m['profit_factor']:>7.2f}")
    print(f"{'BH':>4} {'':>5} {'':>6} {bh:>+12.2f}")
    print(f"\nI 降成本笔: {im['n_with_cr']}")
    print(f"I 级别归属(ladder→笔): {i_extra['ladder_attribution']}")
    print(f"I 平均持仓bar: {i_extra['ladder_avg_held_bars']}")
    print(f"I 2买标记: {i_extra['addon_2buy_marks']}")

    out = {
        "data_file": DATA_PATH.name,
        "n_bars": n, "year_range": [years[0], years[-1]],
        "first": closes[0], "last": closes[-1], "bh": bh,
        "E": em, "I": im,
        "ladder_attribution": i_extra["ladder_attribution"],
        "ladder_avg_held_bars": i_extra["ladder_avg_held_bars"],
        "addon_2buy_marks": i_extra["addon_2buy_marks"],
    }
    out_path = vi.DATA_DIR / "fugue_version_i_oklo447k.json"
    out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False, default=str))
    print(f"\n结果 JSON: {out_path}")


if __name__ == "__main__":
    main()
