"""T6/T7 真实数据贡献率统计脚本。

T6 = 嵌套背驰（区间套跨级别背驰搜索）
T7 = 买卖点（三类买卖点识别）

在 AAPL 1分钟真实数据上运行 RecursiveOrchestrator，
统计 T6/T7 的触发频率、信号质量和分布。

用法：
    python scripts/t6t7_contribution_analysis.py
    python scripts/t6t7_contribution_analysis.py --help
    python scripts/t6t7_contribution_analysis.py --output results.json
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import Counter
from datetime import datetime
from pathlib import Path

# 确保 src 在 sys.path 中
_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.a_nested_divergence import (
    NestedDivergence,
    nested_divergence_search,
)
from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.types import Bar

# ── 数据加载 ──

DATA_PATH = _project_root / "data" / "AAPL_20260209_20260213_1m.csv"


def _load_bars(path: Path) -> list[Bar]:
    bars: list[Bar] = []
    with open(path, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            ts = datetime.fromisoformat(row["timestamp"])
            bars.append(Bar(
                ts=ts,
                open=float(row["open"]),
                high=float(row["high"]),
                low=float(row["low"]),
                close=float(row["close"]),
                volume=float(row["volume"]) if row.get("volume") else None,
            ))
    return bars


def _filter_rth(bars: list[Bar]) -> list[Bar]:
    """保留美东常规交易时段 (9:30-16:00 ET = 14:30-21:00 UTC)。"""
    result = []
    for bar in bars:
        minutes = bar.ts.hour * 60 + bar.ts.minute
        if 14 * 60 + 30 <= minutes < 21 * 60:
            result.append(bar)
    return result


# ── T6 统计 ──


def _analyze_t6(
    snapshots: list[RecursiveOrchestratorSnapshot],
) -> dict:
    """统计嵌套背驰（T6）的触发频率和信号质量。"""
    all_nested: list[NestedDivergence] = []
    trigger_bar_indices: list[int] = []

    for snap in snapshots:
        results = nested_divergence_search(snap)
        if results:
            all_nested.extend(results)
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
            "avg_bar_range_width": 0.0,
            "max_chain_depth": 0,
            "kind_distribution": {},
        }

    chain_depths = [len(nd.chain) for nd in all_nested]
    bar_range_widths = [nd.bar_range[1] - nd.bar_range[0] for nd in all_nested]

    # 方向统计：取链中最高级别的 divergence 方向
    direction_counter: Counter[str] = Counter()
    kind_counter: Counter[str] = Counter()
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
        "trigger_rate_pct": round(trigger_bars / total_bars * 100, 2) if total_bars else 0.0,
        "chain_depth_distribution": dict(Counter(chain_depths)),
        "direction_distribution": dict(direction_counter),
        "confirmed_count": confirmed,
        "unconfirmed_count": unconfirmed,
        "avg_chain_depth": round(sum(chain_depths) / len(chain_depths), 2),
        "avg_bar_range_width": round(sum(bar_range_widths) / len(bar_range_widths), 2),
        "max_chain_depth": max(chain_depths),
        "kind_distribution": dict(kind_counter),
    }


# ── T7 统计 ──


def _analyze_t7(
    snapshots: list[RecursiveOrchestratorSnapshot],
) -> dict:
    """统计买卖点（T7）的触发频率和分布。"""
    # 使用最终快照的 BSP 列表（全量计算结果）
    final_snap = snapshots[-1] if snapshots else None
    if final_snap is None:
        return {
            "total_buysellpoints": 0,
            "kind_distribution": {},
            "side_distribution": {},
            "confirmed_count": 0,
            "settled_count": 0,
            "overlap_count": 0,
            "level_distribution": {},
        }

    bsps = final_snap.bsp_snapshot.buysellpoints

    # 逐 bar 追踪 BSP 数量变化，统计新增事件
    bsp_event_bars: list[int] = []
    prev_count = 0
    for snap in snapshots:
        curr_count = len(snap.bsp_snapshot.buysellpoints)
        if curr_count > prev_count:
            bsp_event_bars.append(snap.bar_idx)
        prev_count = curr_count

    kind_counter: Counter[str] = Counter()
    side_counter: Counter[str] = Counter()
    level_counter: Counter[int] = Counter()
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

    total_bars = len(snapshots)

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
            len(bsp_event_bars) / total_bars * 100, 2,
        ) if total_bars else 0.0,
    }


# ── 主流程 ──


def run_analysis(data_path: Path) -> dict:
    """运行完整分析流程，返回统计结果。"""
    print(f"[T6T7] 加载数据: {data_path}")
    bars = _load_bars(data_path)
    rth_bars = _filter_rth(bars)
    print(f"[T6T7] 全量 bar: {len(bars)}, RTH bar: {len(rth_bars)}")

    orch = RecursiveOrchestrator(
        stream_id="t6t7_analysis",
        max_levels=6,
        stroke_mode="new",
    )

    snapshots: list[RecursiveOrchestratorSnapshot] = []
    for i, bar in enumerate(rth_bars):
        snap = orch.process_bar(bar)
        snapshots.append(snap)
        if (i + 1) % 500 == 0:
            print(f"[T6T7] 已处理 {i + 1}/{len(rth_bars)} bars")

    print(f"[T6T7] 处理完成，共 {len(snapshots)} 个快照")

    # 管线基础统计
    final = snapshots[-1]
    pipeline_summary = {
        "total_bars_rth": len(rth_bars),
        "L1_strokes": len(final.bi_snapshot.strokes),
        "L1_segments": len(final.seg_snapshot.segments),
        "L1_zhongshus": len(final.zs_snapshot.zhongshus),
        "L1_moves": len(final.move_snapshot.moves),
        "recursive_levels": len(final.recursive_snapshots),
    }

    print("[T6T7] 分析 T6（嵌套背驰）...")
    t6_stats = _analyze_t6(snapshots)

    print("[T6T7] 分析 T7（买卖点）...")
    t7_stats = _analyze_t7(snapshots)

    return {
        "data_source": str(data_path.name),
        "pipeline_summary": pipeline_summary,
        "T6_nested_divergence": t6_stats,
        "T7_buysellpoints": t7_stats,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="T6/T7 真实数据贡献率统计",
    )
    parser.add_argument(
        "--data", type=Path, default=DATA_PATH,
        help=f"CSV 数据路径 (默认: {DATA_PATH})",
    )
    parser.add_argument(
        "--output", "-o", type=Path, default=None,
        help="输出 JSON 路径 (默认: stdout)",
    )
    args = parser.parse_args()

    if not args.data.exists():
        print(f"[T6T7] 数据文件不存在: {args.data}", file=sys.stderr)
        sys.exit(1)

    result = run_analysis(args.data)

    output_json = json.dumps(result, indent=2, ensure_ascii=False)

    if args.output:
        args.output.write_text(output_json, encoding="utf-8")
        print(f"[T6T7] 结果已写入: {args.output}")
    else:
        print("\n" + output_json)


if __name__ == "__main__":
    main()
