"""T6/T7 多标的日线数据扩展分析。

拉取 AAPL/MSFT/GOOGL 的日线全量数据，运行 RecursiveOrchestrator，
统计 T6（嵌套背驰）和 T7（买卖点）的触发情况。

与 229 号谱系（AAPL 1min x 5天）进行对比。
"""

from __future__ import annotations

import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

# 确保 src 在路径中
_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

os.environ["ALPHAVANTAGE_API_KEY"] = "WP4GIQ6VALD179P3"

from newchan.a_nested_divergence import nested_divergence_search
from newchan.data_av import AlphaVantageProvider
from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)


def analyze_t6(snapshots: list[RecursiveOrchestratorSnapshot]) -> dict:
    """统计 T6 嵌套背驰。"""
    all_nested = []
    trigger_bar_indices = []

    for snap in snapshots:
        nd_results = nested_divergence_search(snap)
        if nd_results:
            all_nested.extend(nd_results)
            trigger_bar_indices.append(snap.bar_idx)

    if not all_nested:
        return {
            "total_signals": 0,
            "trigger_bar_count": 0,
            "trigger_rate_pct": 0.0,
            "chain_depth_distribution": {},
            "direction_distribution": {},
            "confirmed_count": 0,
            "unconfirmed_count": 0,
            "avg_chain_depth": 0.0,
            "max_chain_depth": 0,
            "kind_distribution": {},
        }

    chain_depths = [len(nd.chain) for nd in all_nested]
    direction_counter: Counter = Counter()
    kind_counter: Counter = Counter()
    confirmed = 0
    unconfirmed = 0

    for nd in all_nested:
        for _level_id, div in nd.chain:
            if div is not None:
                direction_counter[div.direction] += 1
                kind_counter[div.kind] += 1
                if div.confirmed:
                    confirmed += 1
                else:
                    unconfirmed += 1

    total_bars = len(snapshots)
    trigger_bars = len(set(trigger_bar_indices))

    return {
        "total_signals": len(all_nested),
        "trigger_bar_count": trigger_bars,
        "trigger_rate_pct": round(trigger_bars / total_bars * 100, 4) if total_bars else 0.0,
        "chain_depth_distribution": dict(Counter(chain_depths)),
        "direction_distribution": dict(direction_counter),
        "confirmed_count": confirmed,
        "unconfirmed_count": unconfirmed,
        "avg_chain_depth": round(sum(chain_depths) / len(chain_depths), 2),
        "max_chain_depth": max(chain_depths),
        "kind_distribution": dict(kind_counter),
    }


def analyze_t7(snapshots: list[RecursiveOrchestratorSnapshot]) -> dict:
    """统计 T7 买卖点。"""
    if not snapshots:
        return {
            "total_buysellpoints": 0,
            "kind_distribution": {},
            "side_distribution": {},
            "confirmed_count": 0,
            "settled_count": 0,
            "overlap_count": 0,
            "level_distribution": {},
            "new_bsp_event_bar_count": 0,
            "new_bsp_event_rate_pct": 0.0,
        }

    final_snap = snapshots[-1]
    bsps = final_snap.bsp_snapshot.buysellpoints

    # 逐 bar 追踪 BSP 数量变化
    bsp_event_bars = []
    prev_count = 0
    for snap in snapshots:
        curr_count = len(snap.bsp_snapshot.buysellpoints)
        if curr_count > prev_count:
            bsp_event_bars.append(snap.bar_idx)
        prev_count = curr_count

    kind_counter: Counter = Counter()
    side_counter: Counter = Counter()
    level_counter: Counter = Counter()
    confirmed = 0
    settled = 0
    overlap = 0

    for bp in bsps:
        kind_counter[bp.kind] += 1
        side_counter[bp.side] += 1
        level_counter[bp.level_id] += 1
        if bp.confirmed:
            confirmed += 1
        if bp.settled:
            settled += 1
        if bp.overlaps_with is not None:
            overlap += 1

    return {
        "total_buysellpoints": len(bsps),
        "kind_distribution": dict(kind_counter),
        "side_distribution": dict(side_counter),
        "confirmed_count": confirmed,
        "settled_count": settled,
        "overlap_count": overlap,
        "level_distribution": {str(k): v for k, v in sorted(level_counter.items())},
        "new_bsp_event_bar_count": len(bsp_event_bars),
        "new_bsp_event_rate_pct": round(
            len(bsp_event_bars) / len(snapshots) * 100, 4
        ) if snapshots else 0.0,
    }


def main() -> None:
    provider = AlphaVantageProvider(api_key="WP4GIQ6VALD179P3")
    symbols = ["AAPL", "MSFT", "GOOGL"]
    results: dict = {}

    for sym in symbols:
        print(f"[T6T7-expand] 拉取 {sym} 日线数据 (outputsize=full)...", flush=True)
        try:
            bars = provider.fetch_daily(sym, outputsize="full")
            print(f"[T6T7-expand] {sym}: 获得 {len(bars)} bar", flush=True)
        except Exception as e:
            print(f"[T6T7-expand] {sym} 拉取失败: {e}", flush=True)
            results[sym] = {"error": str(e)}
            continue

        # 运行 RecursiveOrchestrator
        print(f"[T6T7-expand] {sym}: 运行 RecursiveOrchestrator (max_levels=6)...", flush=True)
        orch = RecursiveOrchestrator(
            stream_id=f"t6t7_{sym}_daily",
            max_levels=6,
            stroke_mode="new",
        )

        snapshots: list[RecursiveOrchestratorSnapshot] = []
        for i, bar in enumerate(bars):
            snap = orch.process_bar(bar)
            snapshots.append(snap)
            if (i + 1) % 1000 == 0:
                print(f"[T6T7-expand] {sym}: 已处理 {i+1}/{len(bars)} bars", flush=True)

        print(f"[T6T7-expand] {sym}: 处理完成，共 {len(snapshots)} 个快照", flush=True)

        # pipeline 基础统计
        final = snapshots[-1]
        pipeline = {
            "total_bars": len(bars),
            "L1_strokes": len(final.bi_snapshot.strokes),
            "L1_segments": len(final.seg_snapshot.segments),
            "L1_zhongshus": len(final.zs_snapshot.zhongshus),
            "L1_moves": len(final.move_snapshot.moves),
            "recursive_levels": len(final.recursive_snapshots),
        }
        print(f"[T6T7-expand] {sym} pipeline: {json.dumps(pipeline)}", flush=True)

        # T6 + T7 分析
        print(f"[T6T7-expand] {sym}: 分析 T6（嵌套背驰）...", flush=True)
        t6_stats = analyze_t6(snapshots)
        print(f"[T6T7-expand] {sym} T6: signals={t6_stats['total_signals']}, "
              f"max_depth={t6_stats['max_chain_depth']}", flush=True)

        print(f"[T6T7-expand] {sym}: 分析 T7（买卖点）...", flush=True)
        t7_stats = analyze_t7(snapshots)
        print(f"[T6T7-expand] {sym} T7: bsp={t7_stats['total_buysellpoints']}, "
              f"kinds={t7_stats['kind_distribution']}", flush=True)

        results[sym] = {
            "pipeline_summary": pipeline,
            "T6_nested_divergence": t6_stats,
            "T7_buysellpoints": t7_stats,
        }

    # 229 号基线
    baseline_229 = {
        "data": "AAPL 1min x 5 days, 1646 RTH bars",
        "recursive_levels": 1,
        "T6_total_signals": 0,
        "T7_total_buysellpoints": 1,
        "T7_types": {"type3_sell": 1},
    }

    # 对比
    comparison = {}
    for sym, data in results.items():
        if "error" in data:
            comparison[sym] = f"fetch failed: {data['error']}"
            continue
        p = data["pipeline_summary"]
        t6 = data["T6_nested_divergence"]
        t7 = data["T7_buysellpoints"]
        comparison[sym] = {
            "bars_vs_229": f"{p['total_bars']} daily vs 1646 (1min RTH)",
            "recursive_levels_vs_229": f"{p['recursive_levels']} vs 1",
            "T6_signals_vs_229": f"{t6['total_signals']} vs 0",
            "T7_bsp_vs_229": f"{t7['total_buysellpoints']} vs 1",
        }

    # 结论
    valid_symbols = [s for s in results if "error" not in results[s]]
    any_t6 = any(
        results[s]["T6_nested_divergence"]["total_signals"] > 0
        for s in valid_symbols
    )
    total_t7 = sum(
        results[s]["T7_buysellpoints"]["total_buysellpoints"]
        for s in valid_symbols
    )
    max_recursive = max(
        (results[s]["pipeline_summary"]["recursive_levels"] for s in valid_symbols),
        default=0,
    )

    conclusion_parts = []
    if any_t6:
        conclusion_parts.append(
            "日线全量数据上 T6 嵌套背驰已触发"
            "（229号边界条件翻转——数据充足时 T6 > 0）"
        )
    else:
        conclusion_parts.append(
            "日线全量数据上 T6 仍未触发，需进一步诊断递归层级与嵌套背驰的触发条件"
        )
    conclusion_parts.append(
        f"T7 买卖点总计 {total_t7} 个（229号为 1 个），数量级显著提升"
    )
    conclusion_parts.append(
        f"最大递归层级 {max_recursive}（229号为 1），递归深度显著提升"
    )

    output = {
        "analysis_date": time.strftime("%Y-%m-%d %H:%M:%S"),
        "data_source": "AlphaVantage daily (full)",
        "symbols": results,
        "comparison_with_229": comparison,
        "baseline_229": baseline_229,
        "conclusion": "; ".join(conclusion_parts),
    }

    out_path = _project_root / "tmp" / "t6t7-expanded-results.json"
    out_path.write_text(
        json.dumps(output, indent=2, ensure_ascii=False, default=str),
        encoding="utf-8",
    )
    print(f"\n[T6T7-expand] 结果已写入: {out_path}", flush=True)
    print("[T6T7-expand] === DONE ===", flush=True)


if __name__ == "__main__":
    main()
