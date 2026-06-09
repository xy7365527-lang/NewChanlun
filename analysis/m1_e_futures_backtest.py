"""期货全标的 E 版本回测 + P3 随机门控（一体，增量持久化）。

## 为何合并 E 回测与 P3

`compute_e_signals_rust`（O(N²) 缠论递归，5.5M bar ≈ 28min/标的）是唯一计算瓶颈。
E 版本 metrics 与 P3 随机门控**共享同一份 signals/trades**——P3 的随机门控只需
`closes` + 交易笔数 N + 平均持有 hold_bars（5000 次均匀采样是秒级）。故每标的
只跑一次 Rust 引擎，E metrics 与 P3 秩统计一气呵成；分两脚本会让引擎白跑两遍。

## 增量持久化（防 jetsam / 中断丢全量）

每标的完成立即写 `data_cache/_m1e_fut_{sym}.json`（含 E metrics + P3 秩统计）。
重跑时已存在的标的直接跳过（resume）——单进程被杀不丢已完成标的的结果。

## 严格等价

信号层 `compute_e_signals_rust` 与 `fugue_alpha_diagnosis.compute_signals` 的 E 字段
逐位等价（m1_e_rust_engine 文档证明，ES/BRN 150k bit-exact）；交易层
`run_swing_trading(MODE_NONE)`、P3 `large_bootstrap_percentile` 均逐字复用，不改任何
信号/引擎/交易逻辑——本脚本仅是 driver。

## 认识论等级

- 移植正确性 L0/L1（管线等价，bit-exact 保证）。
- 回测结论 L2（真实数据，7 期货标的 1min 10-16y；含否定性结果）。
- P3 多标的联合秩 → L3 候选（多标的交叉，若各标的判决一致则否证鲁棒性提升）。

521 号限定：信号属 candidate 层（PH 门控 + MACD 面积代理），非 confirmed 买卖点。
"""

from __future__ import annotations

import json
import math
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402
import p3_random_gate_control as P3  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"

# 标的 → 10-16y 1min databento 数据文件（6E 实际文件名为 usd6e）。
SYMBOL_FILES = {
    "ES": DATA_DIR / "es_1m_databento_10y.json",
    "GC": DATA_DIR / "gc_1m_databento_10y.json",
    "CL": DATA_DIR / "cl_1m_databento_10y.json",
    "ZN": DATA_DIR / "zn_1m_databento_10y.json",
    "BRN": DATA_DIR / "brn_1m_databento_10y.json",
    "6E": DATA_DIR / "usd6e_1m_databento_10y.json",
    "DX": DATA_DIR / "dx_1m_databento_10y.json",
}

# P3 随机门控触发阈值：N < MIN_N_FOR_P3 时统计功效过低（OKLO 6 笔即此问题），
# 不跑随机门控（仍记录 E metrics）。任务指定 N≥10 才判决。
MIN_N_FOR_P3 = 10


def _out_path(sym: str) -> Path:
    return DATA_DIR / f"_m1e_fut_{sym}.json"


def load_ohlc(
    path: Path,
) -> tuple[list[float], list[float], list[float], list[float]]:
    """加载 databento 数组格式（opens/highs/lows/closes）+ nan 清洗。

    删除任一 OHLC 为 nan 的整根 bar（缺失 bar=非交易，标准预处理）。nan 污染
    MACD/PH/背驰下游（BRN 0.68% nan 致 down_move_settled 分叉）。逐字复用
    m1_e_rust_backtest.load_ohlc 的清洗逻辑（清洗后 bit-exact 已验证）。
    """
    raw = json.loads(path.read_text())
    o_in, h_in = raw["opens"], raw["highs"]
    l_in, c_in = raw["lows"], raw["closes"]
    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []
    for o, h, l, c in zip(o_in, h_in, l_in, c_in):
        o, h, l, c = float(o), float(h), float(l), float(c)
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
    return opens, highs, lows, closes


def run_symbol(symbol: str) -> tuple[str, dict]:
    """单标的：load → Rust E 信号 → 交易 → metrics →（N≥10）P3 秩统计 → 增量落盘。"""
    out_path = _out_path(symbol)
    if out_path.exists():
        try:
            cached = json.loads(out_path.read_text())
            print(f"  [{symbol:4s}] 已完成（resume，跳过）", flush=True)
            return symbol, cached
        except (json.JSONDecodeError, OSError):
            pass  # 损坏 → 重跑

    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    # ── E 版本信号 + 交易（唯一的 O(N²) 计算） ──
    t0 = time.time()
    print(f"  [{symbol:4s}] 启动：{n:,} bars，Rust E 信号计算中…", flush=True)
    signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t0

    trades, _ = F.run_swing_trading(signals, F.MODE_NONE)
    m = F.compute_metrics(trades)
    n_trades = len(trades)
    avg_hold = (
        sum(t.exit_bar - t.entry_bar for t in trades) / n_trades
        if n_trades else 0.0
    )

    out: dict = {
        "symbol": symbol,
        "n_bars": n,
        "bh": bh,
        "compound": m["total_compound"],
        "excess": m["total_compound"] - bh,
        "n_trades": n_trades,
        "win_rate": m["win_rate"],
        "sharpe": m["sharpe"],
        "max_dd": m["max_dd"],
        "avg_hold": avg_hold,
        "sig_s": sig_s,
    }

    # ── P3 随机门控秩统计（N≥10 才判决，复用 closes，秒级） ──
    if n_trades >= MIN_N_FOR_P3:
        hold_bars = int(round(avg_hold))
        t_p3 = time.time()
        pct = P3.large_bootstrap_percentile(
            closes, n_trades, hold_bars, m["total_compound"]
        )
        out["p3"] = {
            "n_runs": pct.n_runs,
            "p_random_ge_e": pct.p_random_ge_e,
            "e_percentile": pct.e_percentile,
            "rnd_median": pct.rnd_median,
            "rnd_mean": pct.rnd_mean,
            "rnd_std": pct.rnd_std,
            "hold_bars": hold_bars,
            "p3_s": time.time() - t_p3,
        }
    else:
        out["p3"] = None  # N 太小，功效不足，不判决

    out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False))

    p3 = out.get("p3")
    p3_str = (
        f"P(rnd≥E)={p3['p_random_ge_e']:.1%} pct={p3['e_percentile']:.0f}"
        if p3 else f"P3跳过(N<{MIN_N_FOR_P3})"
    )
    print(
        f"  [{symbol:4s}] {n:>9,} bars | E={m['total_compound']:+10.2f}% "
        f"BH={bh:+9.2f}% 超额={out['excess']:+10.2f}% | 交易={n_trades:4d} "
        f"胜率={m['win_rate']:5.1f}% MDD={m['max_dd']:+7.2f}% | {p3_str} "
        f"| 信号 {sig_s/60:.1f}min",
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
    print(f"期货 E 回测 + P3：{len(symbols)} 标的，{workers} 进程并行")
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
    print(f"\n总耗时 {elapsed/60:.1f}min\n")

    # 汇总表
    hdr = (
        f"{'标的':<6}{'Bars':>11}{'E复利%':>13}{'BH%':>12}{'超额%':>13}"
        f"{'交易':>6}{'胜率%':>8}{'MDD%':>9}{'P(rnd≥E)':>10}{'E百分位':>9}"
    )
    print(hdr)
    print("-" * len(hdr))
    for sym in symbols:
        if sym not in results:
            continue
        r = results[sym]
        p3 = r.get("p3")
        p3c = f"{p3['p_random_ge_e']:>9.1%}" if p3 else f"{'—':>10}"
        p3p = f"{p3['e_percentile']:>9.0f}" if p3 else f"{'—':>9}"
        print(
            f"{sym:<6}{r['n_bars']:>11,}{r['compound']:>13.2f}{r['bh']:>12.2f}"
            f"{r['excess']:>13.2f}{r['n_trades']:>6}{r['win_rate']:>8.1f}"
            f"{r['max_dd']:>9.2f}{p3c}{p3p}"
        )

    agg_path = DATA_DIR / "m1_e_futures_results.json"
    agg_path.write_text(json.dumps(results, indent=2, ensure_ascii=False))
    print(f"\n汇总 JSON：{agg_path}")


if __name__ == "__main__":
    main()
