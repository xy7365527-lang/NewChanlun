"""M1 E 版本回测 — 7 标的 1min 10 年期货全量（背驰定位器进出场，无降成本）。

E 版本（MODE_SWING_E）：`run_swing_trading(signals, MODE_NONE)`——
  进场（1买）= 次级别底背驰（down-move 完成 ∧ 力度衰减）；
  持仓 = 默认满仓穿越；
  出场（1卖）= L2 趋势顶背驰（l2_flip_short ∧ exit_div_ok）。
无降成本（cost_mode=none）→ 纯进出场信号 alpha 基线。

信号层 `compute_signals` 1:1 复用 `fugue_alpha_diagnosis`（引擎/PH/MACD/背驰记录），
不修改任何引擎/信号逻辑——本脚本仅是 driver（参数化标的 + 输出指标）。

认识论等级：L2（真实数据，7 期货标的 1min 10y；含否定性结果）。
521 号限定：信号属 candidate 层（PH 门控 + MACD 面积代理），非 confirmed 买卖点。
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"

# 标的 → 10y 1min 数据文件（注意 6E 实际文件名为 usd6e）
SYMBOL_FILES = {
    "ES": DATA_DIR / "es_1m_databento_10y.json",
    "GC": DATA_DIR / "gc_1m_databento_10y.json",
    "CL": DATA_DIR / "cl_1m_databento_10y.json",
    "ZN": DATA_DIR / "zn_1m_databento_10y.json",
    "BRN": DATA_DIR / "brn_1m_databento_10y.json",
    "6E": DATA_DIR / "usd6e_1m_databento_10y.json",
    "DX": DATA_DIR / "dx_1m_databento_10y.json",
}


def load_ohlc(
    path: Path,
) -> tuple[list[float], list[float], list[float], list[float]]:
    raw = json.loads(path.read_text())
    return (
        [float(x) for x in raw["opens"]],
        [float(x) for x in raw["highs"]],
        [float(x) for x in raw["lows"]],
        [float(x) for x in raw["closes"]],
    )


def run_symbol(symbol: str) -> tuple[str, dict]:
    """单标的 E 版本：load → compute_signals → run_swing_trading(none) → metrics。"""
    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    t0 = time.time()
    signals = F.compute_signals(opens, highs, lows, closes)
    sig_s = time.time() - t0

    t1 = time.time()
    trades, _ = F.run_swing_trading(signals, F.MODE_NONE)
    trade_s = time.time() - t1

    m = F.compute_metrics(trades)
    out = {
        "n_bars": n,
        "bh": bh,
        "compound": m["total_compound"],
        "excess": m["total_compound"] - bh,
        "n_trades": m["n"],
        "win_rate": m["win_rate"],
        "sharpe": m["sharpe"],
        "max_dd": m["max_dd"],
        "sig_s": sig_s,
        "trade_s": trade_s,
    }
    print(
        f"  [{symbol:4s}] {n:>9,} bars | E 复利={m['total_compound']:+10.2f}% "
        f"BH={bh:+9.2f}% 超额={out['excess']:+10.2f}% | 交易={m['n']:4d} "
        f"胜率={m['win_rate']:5.1f}% 夏普={m['sharpe']:+.3f} MDD={m['max_dd']:+7.2f}% "
        f"| 信号 {sig_s:6.1f}s",
        flush=True,
    )
    return symbol, out


def main() -> None:
    symbols = [s for s in SYMBOL_FILES if SYMBOL_FILES[s].exists()]
    only = os.environ.get("BT_SYMBOLS")
    if only:
        symbols = [s.strip().upper() for s in only.split(",") if s.strip()]

    workers = int(os.environ.get("BT_WORKERS", "0")) or min(
        len(symbols), os.cpu_count() or 2
    )
    print(f"M1 E 版本回测：{len(symbols)} 标的，{workers} 进程并行")
    print(f"标的：{', '.join(symbols)}\n")

    results: dict[str, dict] = {}
    t_all = time.time()
    if workers > 1 and len(symbols) > 1:
        from concurrent.futures import ProcessPoolExecutor

        with ProcessPoolExecutor(max_workers=workers) as ex:
            for sym, out in ex.map(run_symbol, symbols):
                results[sym] = out
    else:
        for sym in symbols:
            _, out = run_symbol(sym)
            results[sym] = out

    elapsed = time.time() - t_all
    print(f"\n总耗时 {elapsed:.1f}s\n")

    # 输出表格
    hdr = (
        f"{'标的':<6}{'Bars':>11}{'复利%':>13}{'BH%':>12}"
        f"{'超额%':>13}{'交易':>6}{'胜率%':>8}{'夏普':>8}{'MDD%':>9}"
    )
    print(hdr)
    print("-" * len(hdr))
    rows = []
    for sym in symbols:
        if sym not in results:
            continue
        r = results[sym]
        line = (
            f"{sym:<6}{r['n_bars']:>11,}{r['compound']:>13.2f}{r['bh']:>12.2f}"
            f"{r['excess']:>13.2f}{r['n_trades']:>6}{r['win_rate']:>8.1f}"
            f"{r['sharpe']:>8.3f}{r['max_dd']:>9.2f}"
        )
        print(line)
        rows.append(line)

    out_path = DATA_DIR / "m1_e_rust_backtest_results.json"
    out_path.write_text(json.dumps(results, indent=2, ensure_ascii=False))
    print(f"\n结果 JSON 已写入：{out_path}")


if __name__ == "__main__":
    main()
