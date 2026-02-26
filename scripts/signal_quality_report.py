#!/usr/bin/env python3
"""信号质量统计报告 — 按买卖点类型统计触发次数、确认率、后续价格变动。

遍历 .cache 中可用的 1min 数据，对每个标的运行 RecursiveOrchestrator，
收集所有买卖点信号并计算质量指标。

防未来函数：信号在 bar_idx 时刻产生，后续价格变动仅使用 bar_idx+1..bar_idx+N 的数据。

用法：
    python scripts/signal_quality_report.py
"""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import pandas as pd

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

CACHE_DIR = Path(__file__).resolve().parent.parent / ".cache"
OUTPUT_PATH = Path(__file__).resolve().parent.parent / "tmp" / "signal-quality-report.json"

# 后续 N bar 的观测窗口
HORIZONS = [5, 10, 20, 50]


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


# ── 信号收集 ──


def collect_signals(bars: list[Bar], symbol: str) -> list[dict]:
    """运行 RecursiveOrchestrator，收集所有买卖点信号。

    返回信号记录列表，每条包含：
    - symbol, kind, side, level_id, seg_idx, bar_idx, price, confirmed
    """
    orch = RecursiveOrchestrator(stream_id=symbol)
    signals: list[dict] = []
    seen_keys: set[tuple] = set()

    for bar in bars:
        snap = orch.process_bar(bar)
        for bsp in snap.bsp_snapshot.buysellpoints:
            key = (bsp.seg_idx, bsp.kind, bsp.side, bsp.level_id)
            if key in seen_keys:
                continue
            seen_keys.add(key)
            signals.append({
                "symbol": symbol,
                "kind": bsp.kind,
                "side": bsp.side,
                "level_id": bsp.level_id,
                "seg_idx": bsp.seg_idx,
                "bar_idx": bsp.bar_idx,
                "price": bsp.price,
                "confirmed": bsp.confirmed,
            })

    return signals


# ── 后续价格变动计算 ──


def compute_forward_returns(
    bars: list[Bar],
    signals: list[dict],
    horizons: list[int],
) -> list[dict]:
    """为每个信号计算后续 N bar 的价格变动百分比。

    防未来函数：仅使用 bar_idx 之后的 bar。
    buy 信号：正收益 = 价格上涨；sell 信号：正收益 = 价格下跌。
    """
    n_bars = len(bars)
    enriched: list[dict] = []

    for sig in signals:
        entry = dict(sig)
        bi = sig["bar_idx"]
        price = sig["price"]

        if price <= 0:
            for h in horizons:
                entry[f"fwd_{h}"] = None
            enriched.append(entry)
            continue

        for h in horizons:
            target_idx = bi + h
            if target_idx >= n_bars:
                entry[f"fwd_{h}"] = None
            else:
                future_close = bars[target_idx].close
                raw_return = (future_close - price) / price
                # sell 信号取反：价格下跌 = 正收益
                if sig["side"] == "sell":
                    raw_return = -raw_return
                entry[f"fwd_{h}"] = raw_return

        enriched.append(entry)

    return enriched


# ── 统计聚合 ──


def compute_signal_stats(
    enriched_signals: list[dict],
    horizons: list[int],
) -> dict:
    """按 kind × side 分组聚合统计。

    返回结构：
    {
        "type1_buy": {
            "count": N,
            "confirmed_count": M,
            "confirm_rate": M/N,
            "fwd_5": {"mean": ..., "median": ..., "std": ..., "min": ..., "max": ..., "n": ...},
            ...
        },
        ...
    }
    """
    from collections import defaultdict

    groups: dict[str, list[dict]] = defaultdict(list)
    for sig in enriched_signals:
        key = f"{sig['kind']}_{sig['side']}"
        groups[key].append(sig)

    result: dict = {}
    for group_key in sorted(groups.keys()):
        sigs = groups[group_key]
        total = len(sigs)
        confirmed = sum(1 for s in sigs if s["confirmed"])

        group_stats: dict = {
            "count": total,
            "confirmed_count": confirmed,
            "confirm_rate": confirmed / total if total > 0 else 0.0,
        }

        for h in horizons:
            fwd_key = f"fwd_{h}"
            values = [s[fwd_key] for s in sigs if s[fwd_key] is not None]
            if values:
                values_sorted = sorted(values)
                n = len(values)
                mid = n // 2
                median = values_sorted[mid] if n % 2 == 1 else (values_sorted[mid - 1] + values_sorted[mid]) / 2
                mean = sum(values) / n
                variance = sum((v - mean) ** 2 for v in values) / n if n > 1 else 0.0
                group_stats[fwd_key] = {
                    "mean": round(mean, 6),
                    "median": round(median, 6),
                    "std": round(variance ** 0.5, 6),
                    "min": round(min(values), 6),
                    "max": round(max(values), 6),
                    "n": n,
                }
            else:
                group_stats[fwd_key] = {"mean": None, "median": None, "std": None, "min": None, "max": None, "n": 0}

        result[group_key] = group_stats

    return result


# ── 主流程 ──


def discover_symbols(interval: str = "1min") -> list[str]:
    """扫描 .cache 目录，发现可用标的。"""
    suffix = f"_{interval}_raw.parquet"
    symbols = []
    for p in sorted(CACHE_DIR.glob(f"*{suffix}")):
        sym = p.name.removesuffix(suffix)
        symbols.append(sym)
    return symbols


def main() -> None:
    symbols = discover_symbols("1min")
    if not symbols:
        print("未找到 1min 缓存数据。")
        return

    print(f"发现 {len(symbols)} 个标的: {', '.join(symbols)}")

    all_signals: list[dict] = []
    per_symbol_stats: dict = {}

    for sym in symbols:
        print(f"\n处理 {sym} ...")
        bars = load_bars(sym, "1min")
        print(f"  {len(bars)} bars")

        signals = collect_signals(bars, sym)
        print(f"  {len(signals)} 个信号")

        enriched = compute_forward_returns(bars, signals, HORIZONS)
        all_signals.extend(enriched)

        sym_stats = compute_signal_stats(enriched, HORIZONS)
        per_symbol_stats[sym] = sym_stats

        # 打印摘要
        for gk, gs in sym_stats.items():
            fwd20 = gs.get("fwd_20", {})
            fwd20_mean = fwd20.get("mean")
            fwd20_str = f"{fwd20_mean:+.4%}" if fwd20_mean is not None else "N/A"
            print(f"  {gk}: count={gs['count']}, confirm_rate={gs['confirm_rate']:.1%}, fwd_20_mean={fwd20_str}")

    # 全局聚合
    global_stats = compute_signal_stats(all_signals, HORIZONS)

    report = {
        "generated_at": datetime.now().isoformat(),
        "symbols": symbols,
        "horizons": HORIZONS,
        "total_signals": len(all_signals),
        "global_stats": global_stats,
        "per_symbol": per_symbol_stats,
    }

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n报告已写入: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
