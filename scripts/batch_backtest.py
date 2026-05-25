#!/usr/bin/env python3
"""批量回测 — 多标的并行运行 BacktestEngine，输出横向对比 CSV。

用法：
    python scripts/batch_backtest.py [symbol1 symbol2 ...]
    python scripts/batch_backtest.py          # 自动发现 .cache 中所有标的

输出：tmp/batch_backtest_report.csv
"""

from __future__ import annotations

import csv
import sys
from concurrent.futures import ProcessPoolExecutor, as_completed
from datetime import datetime
from pathlib import Path
from typing import Any

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.backtest import BacktestConfig, BacktestEngine, BacktestResult
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

CACHE_DIR = Path(__file__).resolve().parent.parent / ".cache"
OUTPUT_PATH = Path(__file__).resolve().parent.parent / "tmp" / "batch_backtest_report.csv"


# ── 数据加载 ──


def load_bars(symbol: str, interval: str = "1min") -> list[Bar]:
    """从 parquet 缓存加载 Bar 列表。"""
    path = CACHE_DIR / f"{symbol}_{interval}_raw.parquet"
    if not path.exists():
        raise FileNotFoundError(f"缓存不存在: {path}")
    df = pd.read_parquet(path)
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts,
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=float(row["volume"]) if "volume" in row and pd.notna(row.get("volume")) else None,
        ))
    return bars


def discover_symbols(interval: str = "1min") -> list[str]:
    """扫描 .cache 目录，发现可用标的。"""
    suffix = f"_{interval}_raw.parquet"
    return sorted(
        p.name.removesuffix(suffix)
        for p in CACHE_DIR.glob(f"*{suffix}")
    )


# ── 单标的回测 ──


def run_single_backtest(symbol: str, config: BacktestConfig | None = None) -> BacktestResult:
    """对单个标的运行完整回测。"""
    bars = load_bars(symbol)
    orch = RecursiveOrchestrator(stream_id=symbol)
    engine = BacktestEngine(config=config)
    for bar in bars:
        snap = orch.process_bar(bar)
        engine.process_snapshot(snap, bar)
    return engine.result()


# ── 结果转换（纯函数，可测试） ──


def backtest_result_to_row(symbol: str, result: BacktestResult) -> dict[str, Any]:
    """BacktestResult → 扁平 dict（一行 CSV）。

    pending-004 结算（2026-05-24）：移除 win_count/loss_count/win_rate/
    profit_loss_ratio/max_drawdown_pct/cost_to_gross_profit_ratio——这些是跨标的
    策略表现排名（"测策略好不好"），违反蓝图第一原则/220/295/SKILL§2.2。
    batch 报告只保留 L1 管线事实：交易计数 + 执行成本。
    """
    return {
        "symbol": symbol,
        "total_bars": result.total_bars,
        "trade_count": result.trade_count,
        "total_cost": result.cost_summary.total_cost,
    }


def results_to_dataframe(results: dict[str, BacktestResult]) -> pd.DataFrame:
    """多标的结果 → DataFrame，按 symbol 排序。"""
    rows = [
        backtest_result_to_row(sym, res)
        for sym, res in sorted(results.items())
    ]
    return pd.DataFrame(rows)


def write_csv(results: dict[str, BacktestResult], path: Path) -> None:
    """将多标的回测结果写入 CSV。"""
    df = results_to_dataframe(results)
    path.parent.mkdir(parents=True, exist_ok=True)
    df.to_csv(path, index=False)


# ── 主流程 ──


def main() -> None:
    symbols = sys.argv[1:] if len(sys.argv) > 1 else discover_symbols()
    if not symbols:
        print("未找到可用标的。")
        return

    print(f"回测 {len(symbols)} 个标的: {', '.join(symbols)}")
    config = BacktestConfig()
    results: dict[str, BacktestResult] = {}

    with ProcessPoolExecutor() as executor:
        futures = {
            executor.submit(run_single_backtest, sym, config): sym
            for sym in symbols
        }
        for future in as_completed(futures):
            sym = futures[future]
            try:
                results[sym] = future.result()
                r = results[sym]
                # pending-004：不再打印 win_rate/PL_ratio/max_dd（策略评价指标）
                print(f"  {sym}: {r.trade_count} trades, total_cost={r.cost_summary.total_cost:.2f}")
            except Exception as e:
                print(f"  {sym}: 失败 — {e}")

    if results:
        write_csv(results, OUTPUT_PATH)
        print(f"\n报告已写入: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
