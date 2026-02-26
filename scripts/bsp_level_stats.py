#!/usr/bin/env python3
"""买卖点级别分布分析 — 统计各递归级别上买卖点的触发频率和有效性。

分析维度：
- 各级别触发频率分布
- 各级别确认率 + 前向收益
- 级别间实战价值对比

用法：
    python scripts/bsp_level_stats.py

输出：tmp/bsp-level-stats-report.json
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import pandas as pd

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

CACHE_DIR = Path(__file__).resolve().parent.parent / ".cache"
OUTPUT_PATH = Path(__file__).resolve().parent.parent / "tmp" / "bsp-level-stats-report.json"
DEFAULT_HORIZON = 20


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


# ── 信号收集 ──


def collect_signals(bars: list[Bar], symbol: str) -> list[dict]:
    """运行 RecursiveOrchestrator，收集所有买卖点信号。"""
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


# ── 级别频率统计（纯函数，可测试） ──


def compute_level_frequency(signals: list[dict]) -> dict[int, dict[str, Any]]:
    """各级别触发频率。

    Returns
    -------
    dict[int, dict]
        {level_id: {"count": N, "ratio": N/total}}
    """
    if not signals:
        return {}

    counts: dict[int, int] = defaultdict(int)
    for sig in signals:
        counts[sig["level_id"]] += 1

    total = len(signals)
    return {
        lvl: {"count": cnt, "ratio": cnt / total}
        for lvl, cnt in sorted(counts.items())
    }


# ── 级别有效性统计（纯函数，可测试） ──


def compute_level_effectiveness(
    signals: list[dict],
    bars: list[Bar],
    horizon: int = DEFAULT_HORIZON,
) -> dict[int, dict[str, Any]]:
    """各级别确认率 + 前向收益。

    Returns
    -------
    dict[int, dict]
        {level_id: {"confirmed_rate": float, "mean_fwd_return": float, "count": int}}
    """
    if not signals:
        return {}

    groups: dict[int, list[dict]] = defaultdict(list)
    for sig in signals:
        groups[sig["level_id"]].append(sig)

    n_bars = len(bars)
    result: dict[int, dict[str, Any]] = {}

    for lvl in sorted(groups.keys()):
        sigs = groups[lvl]
        confirmed_count = sum(1 for s in sigs if s["confirmed"])
        confirmed_rate = confirmed_count / len(sigs)

        fwd_returns: list[float] = []
        for s in sigs:
            bi = s["bar_idx"]
            price = s["price"]
            target = bi + horizon
            if target >= n_bars or price <= 0:
                continue
            ret = (bars[target].close - price) / price
            if s["side"] == "sell":
                ret = -ret
            fwd_returns.append(ret)

        mean_fwd = sum(fwd_returns) / len(fwd_returns) if fwd_returns else 0.0

        result[lvl] = {
            "count": len(sigs),
            "confirmed_rate": confirmed_rate,
            "mean_fwd_return": mean_fwd,
        }

    return result


# ── 级别对比（纯函数，可测试） ──


def compare_levels(
    level_stats: dict[int, dict[str, Any]],
    sort_by: str = "mean_fwd_return",
) -> list[dict[str, Any]]:
    """按指定指标排序级别，返回排名列表。"""
    if not level_stats:
        return []

    items = [
        {"level_id": lvl, **stats}
        for lvl, stats in level_stats.items()
    ]
    items.sort(key=lambda x: x.get(sort_by, 0), reverse=True)
    return items


# ── 主流程 ──


def main() -> None:
    symbols = discover_symbols()
    if not symbols:
        print("未找到 1min 缓存数据。")
        return

    print(f"发现 {len(symbols)} 个标的: {', '.join(symbols)}")

    all_signals: list[dict] = []
    per_symbol: dict[str, Any] = {}

    for sym in symbols:
        print(f"\n处理 {sym} ...")
        bars = load_bars(sym)
        signals = collect_signals(bars, sym)
        print(f"  {len(bars)} bars, {len(signals)} 个信号")

        all_signals.extend(signals)

        freq = compute_level_frequency(signals)
        eff = compute_level_effectiveness(signals, bars)
        ranked = compare_levels(eff)

        per_symbol[sym] = {
            "frequency": {str(k): v for k, v in freq.items()},
            "effectiveness": {str(k): v for k, v in eff.items()},
            "ranking": ranked,
        }

        for r in ranked:
            print(f"  Level {r['level_id']}: count={r['count']}, "
                  f"confirmed={r['confirmed_rate']:.1%}, "
                  f"fwd={r['mean_fwd_return']:+.4%}")

    # 全局聚合
    global_freq = compute_level_frequency(all_signals)
    # 全局有效性需要所有 bars — 此处按标的分别计算后合并
    global_eff: dict[int, dict[str, Any]] = {}
    for sym_data in per_symbol.values():
        for lvl_str, eff_data in sym_data["effectiveness"].items():
            lvl = int(lvl_str)
            if lvl not in global_eff:
                global_eff[lvl] = {"count": 0, "confirmed_sum": 0, "fwd_sum": 0.0, "fwd_n": 0}
            global_eff[lvl]["count"] += eff_data["count"]
            global_eff[lvl]["confirmed_sum"] += round(eff_data["confirmed_rate"] * eff_data["count"])
            if eff_data["mean_fwd_return"] != 0.0:
                global_eff[lvl]["fwd_sum"] += eff_data["mean_fwd_return"] * eff_data["count"]
                global_eff[lvl]["fwd_n"] += eff_data["count"]

    global_eff_clean: dict[int, dict[str, Any]] = {}
    for lvl, data in sorted(global_eff.items()):
        cnt = data["count"]
        global_eff_clean[lvl] = {
            "count": cnt,
            "confirmed_rate": data["confirmed_sum"] / cnt if cnt > 0 else 0.0,
            "mean_fwd_return": data["fwd_sum"] / data["fwd_n"] if data["fwd_n"] > 0 else 0.0,
        }

    global_ranked = compare_levels(global_eff_clean)

    report = {
        "generated_at": datetime.now().isoformat(),
        "symbols": symbols,
        "horizon": DEFAULT_HORIZON,
        "total_signals": len(all_signals),
        "global_frequency": {str(k): v for k, v in global_freq.items()},
        "global_effectiveness": {str(k): v for k, v in global_eff_clean.items()},
        "global_ranking": global_ranked,
        "per_symbol": per_symbol,
    }

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n报告已写入: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
