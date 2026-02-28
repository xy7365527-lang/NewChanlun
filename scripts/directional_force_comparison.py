#!/usr/bin/env python3
"""方向性力度 vs 振幅力度对比脚本。

239号谱系：方向性力度 ∉ ker(D)，替代振幅力度 ∈ ker(D)。
237号诊断：高级别背驰判断使用振幅力度 ∈ ker(D)。

获取 SPY 日线数据，用 BiEngine + 线段 + 中枢 + 走势类型管线
计算所有背驰，同时对比振幅力度和方向性力度的判定差异。

输出 JSON 到 tmp/directional-force-comparison.json。
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_PROJECT_ROOT / "src"))

import logging
logging.getLogger("newchan").setLevel(logging.ERROR)

import yfinance as yf

from newchan.bi_engine import BiEngine
from newchan.types import Bar
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.core.recursion.move_engine import MoveEngine
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.topology.directional_force import (
    compare_force_methods,
    detect_directional_divergence,
    directional_persistence,
    directional_persistence_weighted,
    stroke_density_from_segments,
    zhongshu_drift_force,
)


TICKER = "SPY"
START_DATE = "2007-03-01"
END_DATE = "2026-02-27"


def fetch_daily_data(ticker: str) -> list[Bar]:
    """获取日线数据并转换为 Bar 序列。"""
    print(f"  获取 {ticker} 日线数据 [{START_DATE} ~ {END_DATE}]...")
    df = yf.download(ticker, start=START_DATE, end=END_DATE, auto_adjust=True)
    if df.empty:
        raise RuntimeError(f"无法获取 {ticker} 数据")
    if hasattr(df.columns, "levels") and len(df.columns.levels) > 1:
        df.columns = df.columns.get_level_values(0)
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime().replace(tzinfo=timezone.utc),
            open=float(row.iloc[row.index.get_loc("Open")]) if "Open" in row.index else float(row.iloc[0]),
            high=float(row.iloc[row.index.get_loc("High")]) if "High" in row.index else float(row.iloc[1]),
            low=float(row.iloc[row.index.get_loc("Low")]) if "Low" in row.index else float(row.iloc[2]),
            close=float(row.iloc[row.index.get_loc("Close")]) if "Close" in row.index else float(row.iloc[3]),
        ))
    print(f"    → {len(bars)} bars")
    return bars


def run_pipeline(bars: list[Bar]) -> dict:
    """运行完整管线，返回 segments, zhongshus, moves, divergences。"""
    bi_engine = BiEngine(stroke_mode="new")
    seg_engine = SegmentEngine()
    zs_engine = ZhongshuEngine()
    move_engine = MoveEngine()

    for bar in bars:
        bi_snap = bi_engine.process_bar(bar)
        seg_snap = seg_engine.process_snapshot(bi_snap)
        zs_snap = zs_engine.process_segment_snapshot(seg_snap)
        move_engine.process_zhongshu_snapshot(zs_snap)

    segments = seg_engine.current_segments
    zhongshus = zs_engine.current_zhongshus
    moves = move_engine.current_moves

    print(f"  管线输出: {len(segments)} segments, {len(zhongshus)} zhongshus, {len(moves)} moves")

    divergences = divergences_from_moves_v1(
        segments, zhongshus, moves, level_id=1,
    )
    print(f"  背驰数量: {len(divergences)}")

    return {
        "segments": segments,
        "zhongshus": zhongshus,
        "moves": moves,
        "divergences": divergences,
        "strokes": bi_engine.current_strokes,
    }


def compare_divergences(pipeline: dict) -> list[dict]:
    """对每个已检测的背驰，对比振幅力度和三种方向性力度。"""
    segments = pipeline["segments"]
    zhongshus = pipeline["zhongshus"]
    divergences = pipeline["divergences"]
    results = []

    for div in divergences:
        direction = "up" if div.direction == "top" else "down"

        # 三种方向性力度方法
        methods_results = {}
        for method in ("persistence", "density", "composite"):
            zs_a = None
            zs_c = None
            if method == "composite" and zhongshus:
                zs_a = list(range(
                    max(0, div.center_idx - 1), div.center_idx + 1,
                ))
                zs_c = [div.center_idx]

            comparison = compare_force_methods(
                segments,
                div.seg_a_start, div.seg_a_end,
                div.seg_c_start, div.seg_c_end,
                direction,
                method=method,
                zhongshus=zhongshus if method == "composite" else None,
                zs_indices_a=zs_a,
                zs_indices_c=zs_c,
            )
            methods_results[method] = {
                "amplitude_force_a": comparison.amplitude_force_a,
                "amplitude_force_c": comparison.amplitude_force_c,
                "amplitude_divergent": comparison.amplitude_divergent,
                "directional_force_a": comparison.directional_force_a,
                "directional_force_c": comparison.directional_force_c,
                "directional_divergent": comparison.directional_divergent,
                "agreement": comparison.agreement,
            }

        results.append({
            "kind": div.kind,
            "direction": div.direction,
            "seg_a": [div.seg_a_start, div.seg_a_end],
            "seg_c": [div.seg_c_start, div.seg_c_end],
            "center_idx": div.center_idx,
            "original_force_a": div.force_a,
            "original_force_c": div.force_c,
            "methods": methods_results,
        })

    return results


def compute_summary(comparisons: list[dict]) -> dict:
    """计算总结统计。"""
    total = len(comparisons)
    if total == 0:
        return {"total": 0, "note": "no divergences detected"}

    summary: dict = {"total": total}

    for method in ("persistence", "density", "composite"):
        agreements = sum(
            1 for c in comparisons if c["methods"][method]["agreement"]
        )
        disagreements = total - agreements

        # 分类不一致的情况
        only_amplitude = sum(
            1 for c in comparisons
            if c["methods"][method]["amplitude_divergent"]
            and not c["methods"][method]["directional_divergent"]
        )
        only_directional = sum(
            1 for c in comparisons
            if not c["methods"][method]["amplitude_divergent"]
            and c["methods"][method]["directional_divergent"]
        )

        summary[method] = {
            "agreement_count": agreements,
            "agreement_rate": round(agreements / total, 4),
            "disagreement_count": disagreements,
            "only_amplitude_divergent": only_amplitude,
            "only_directional_divergent": only_directional,
        }

    return summary


def main() -> None:
    print("=== 方向性力度 vs 振幅力度对比 ===")
    print()

    try:
        bars = fetch_daily_data(TICKER)
    except Exception as e:
        print(f"数据获取失败: {e}")
        print("使用合成数据进行对比...")
        bars = _generate_synthetic_bars()

    print()
    print("运行管线...")
    pipeline = run_pipeline(bars)

    print()
    print("对比力度方法...")
    comparisons = compare_divergences(pipeline)
    summary = compute_summary(comparisons)

    print()
    print(f"总背驰数: {summary['total']}")
    for method in ("persistence", "density", "composite"):
        if method in summary:
            m = summary[method]
            print(
                f"  {method}: 一致率={m['agreement_rate']:.1%}, "
                f"不一致={m['disagreement_count']}, "
                f"仅振幅背驰={m['only_amplitude_divergent']}, "
                f"仅方向性背驰={m['only_directional_divergent']}"
            )

    output = {
        "ticker": TICKER,
        "date_range": [START_DATE, END_DATE],
        "pipeline_stats": {
            "n_bars": len(bars),
            "n_segments": len(pipeline["segments"]),
            "n_zhongshus": len(pipeline["zhongshus"]),
            "n_moves": len(pipeline["moves"]),
            "n_divergences": len(pipeline["divergences"]),
        },
        "summary": summary,
        "comparisons": comparisons,
        "genealogy": "239号——方向性力度 ∉ ker(D)",
    }

    out_path = _PROJECT_ROOT / "tmp" / "directional-force-comparison.json"
    out_path.parent.mkdir(exist_ok=True)
    out_path.write_text(json.dumps(output, indent=2, ensure_ascii=False))
    print(f"\n输出已写入: {out_path}")


def _generate_synthetic_bars() -> list[Bar]:
    """生成合成 bar 数据（无法获取 yfinance 时 fallback）。"""
    import random
    random.seed(42)
    bars = []
    price = 100.0
    for i in range(2000):
        change = random.gauss(0, 1.5)
        o = price
        c = price + change
        h = max(o, c) + abs(random.gauss(0, 0.5))
        l = min(o, c) - abs(random.gauss(0, 0.5))
        bars.append(Bar(
            ts=datetime(2020, 1, 1, tzinfo=timezone.utc),
            open=o,
            high=h,
            low=l,
            close=c,
        ))
        price = c
    print(f"    → {len(bars)} synthetic bars")
    return bars


if __name__ == "__main__":
    main()
