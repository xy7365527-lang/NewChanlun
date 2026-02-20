"""区间套跨级别背驰搜索（nested divergence search）。

从 RecursiveStack 产出的最高级别开始，逐级向下检测背驰，
每级的检测范围被上一级的背驰 C 段约束。

级别 = 递归层级（level_id），由自底向上的递归构造决定，
不是时间周期。

D_N ⊃ D_{N-1} ⊃ ... ⊃ D_1

规范引用: beichi.md #5 区间套
原文依据: 第27课 精确大转折点寻找程序定理
概念溯源: [旧缠论]
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING, Literal

import pandas as pd

from newchan.a_divergence import Divergence
from newchan.a_divergence_v1 import (
    _seg_merged_range,
    divergences_in_bar_range,
)
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_level import LevelZhongshu

if TYPE_CHECKING:
    from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
    from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot

logger = logging.getLogger(__name__)


# ── 结果类型 ──────────────────────────────────


@dataclass(frozen=True)
class NestedDivergence:
    """区间套搜索结果 — 一条从高级别到低级别的背驰嵌套链。

    Attributes
    ----------
    chain : list[tuple[int, Divergence | None]]
        (level_id, Divergence) 从高到低。Divergence 为 None 表示该级别
        未检测到背驰（链在此截断）。
    bar_range : tuple[int, int]
        最终收缩到的 merged bar 索引范围（闭区间）。
    """

    chain: list[tuple[int, Divergence | None]]
    bar_range: tuple[int, int]


# ── 辅助函数 ──────────────────────────────────


def _get_moves_at_level(
    level: int,
    snap: RecursiveOrchestratorSnapshot,
) -> list[Move]:
    """从 RecursiveOrchestratorSnapshot 中提取指定级别的 Move 列表。

    level=1 → snap.move_snapshot.moves
    level=k (k>=2) → snap.recursive_snapshots[k-2].moves
    """
    if level == 1:
        return snap.move_snapshot.moves
    idx = level - 2
    if idx < len(snap.recursive_snapshots):
        return snap.recursive_snapshots[idx].moves
    return []


def _level_move_to_bar_range(
    move: Move,
    level: int,
    snap: RecursiveOrchestratorSnapshot,
) -> tuple[int, int]:
    """将任意级别的 Move 映射到 level 1 merged bar 索引范围。

    递归下降映射链：
    level k Move.seg_start/seg_end
      → settled(level k-1 moves)[idx].seg_start/seg_end
      → ... → level 1 segments[idx].i0/i1
    """
    if level == 1:
        segments = snap.seg_snapshot.segments
        if move.seg_start >= len(segments) or move.seg_end >= len(segments):
            return (0, 0)
        return _seg_merged_range(segments, move.seg_start, move.seg_end)

    # level k: seg_start/seg_end 索引 settled(level k-1 moves)
    parent_moves = _get_moves_at_level(level - 1, snap)
    settled_parent = [m for m in parent_moves if m.settled]

    if (
        move.seg_start >= len(settled_parent)
        or move.seg_end >= len(settled_parent)
    ):
        return (0, 0)

    first_parent = settled_parent[move.seg_start]
    last_parent = settled_parent[move.seg_end]

    start, _ = _level_move_to_bar_range(first_parent, level - 1, snap)
    _, end = _level_move_to_bar_range(last_parent, level - 1, snap)
    return (start, end)


# ── level 2+ 背驰检测（价格振幅） ─────────────


def _amplitude_force(
    components: list[Move],
    start: int,
    end: int,
) -> float:
    """计算组件范围 [start, end] 的价格振幅力度。

    力度 = (max_high - min_low) * 组件数
    level 2+ 无 MACD 数据，只用价格振幅。
    """
    if start > end or start < 0 or end >= len(components):
        return 0.0
    high = max(components[k].high for k in range(start, end + 1))
    low = min(components[k].low for k in range(start, end + 1))
    duration = max(1, end - start + 1)
    return (high - low) * duration


def _level_trend_ac_segments(
    zhongshus: list[LevelZhongshu],
    move: Move,
    components: list[Move],
) -> tuple[int, int, int, int] | None:
    """计算递归层级趋势背驰的 A/C 段范围。"""
    move_zs_indices: list[int] = [
        i for i in range(move.zs_start, min(move.zs_end + 1, len(zhongshus)))
        if zhongshus[i].settled
    ]
    if len(move_zs_indices) < 2:
        return None

    zs_prev = zhongshus[move_zs_indices[-2]]
    zs_last = zhongshus[move_zs_indices[-1]]

    a_start = zs_prev.comp_end + 1
    a_end = zs_last.comp_start - 1
    if a_start > a_end:
        a_start = a_end = zs_prev.comp_end

    c_start = zs_last.comp_end + 1
    c_end = move.seg_end
    if c_start > c_end:
        return None
    if a_start >= len(components) or c_end >= len(components):
        return None

    return a_start, a_end, c_start, c_end


def _detect_level_trend_divergence(
    zhongshus: list[LevelZhongshu],
    move: Move,
    components: list[Move],
    level_id: int,
) -> Divergence | None:
    """检测递归层级的趋势背驰（价格振幅力度）。"""
    if move.kind != "trend" or move.zs_count < 2:
        return None

    ac = _level_trend_ac_segments(zhongshus, move, components)
    if ac is None:
        return None
    a_start, a_end, c_start, c_end = ac

    force_a = _amplitude_force(components, a_start, a_end)
    force_c = _amplitude_force(components, c_start, c_end)

    if force_a <= 0:
        return None

    if force_c < force_a:
        idx_last = [
            i for i in range(move.zs_start, min(move.zs_end + 1, len(zhongshus)))
            if zhongshus[i].settled
        ][-1]
        div_dir: Literal["top", "bottom"] = (
            "top" if move.direction == "up" else "bottom"
        )
        return Divergence(
            kind="trend",
            direction=div_dir,
            level_id=level_id,
            seg_a_start=a_start,
            seg_a_end=a_end,
            seg_c_start=c_start,
            seg_c_end=c_end,
            center_idx=idx_last,
            force_a=force_a,
            force_c=force_c,
            confirmed=move.settled,
        )

    return None


def _collect_level_exit_comps(
    zs: LevelZhongshu,
    move: Move,
    components: list[Move],
) -> dict[str, list[int]]:
    """收集递归层级中枢的离开组件索引，按方向分组。"""
    exit_comps_by_dir: dict[str, list[int]] = {"up": [], "down": []}

    for k in range(zs.comp_start, min(zs.comp_end + 1, len(components))):
        comp = components[k]
        if comp.high > zs.zg or comp.low < zs.zd:
            d = comp.direction
            if d in exit_comps_by_dir:
                exit_comps_by_dir[d].append(k)

    for k in range(zs.comp_end + 1, min(move.seg_end + 1, len(components))):
        comp = components[k]
        d = comp.direction
        if d in exit_comps_by_dir:
            exit_comps_by_dir[d].append(k)

    return exit_comps_by_dir


def _detect_level_consolidation_divergence(
    zhongshus: list[LevelZhongshu],
    move: Move,
    components: list[Move],
    level_id: int,
) -> Divergence | None:
    """检测递归层级的盘整背驰（价格振幅力度）。"""
    if move.kind != "consolidation" or move.zs_count < 1:
        return None
    if move.zs_start >= len(zhongshus):
        return None

    zs = zhongshus[move.zs_start]
    exit_comps_by_dir = _collect_level_exit_comps(zs, move, components)

    for direction, exits in exit_comps_by_dir.items():
        if len(exits) < 2:
            continue

        a_idx, c_idx = exits[-2], exits[-1]
        force_a = _amplitude_force(components, a_idx, a_idx)
        force_c = _amplitude_force(components, c_idx, c_idx)

        if force_a <= 0:
            continue

        if force_c < force_a:
            div_dir: Literal["top", "bottom"] = (
                "top" if direction == "up" else "bottom"
            )
            return Divergence(
                kind="consolidation",
                direction=div_dir,
                level_id=level_id,
                seg_a_start=a_idx,
                seg_a_end=a_idx,
                seg_c_start=c_idx,
                seg_c_end=c_idx,
                center_idx=move.zs_start,
                force_a=force_a,
                force_c=force_c,
                confirmed=move.settled,
            )

    return None


def divergences_from_level_snapshot(
    level_snap: RecursiveLevelSnapshot,
    components: list[Move],
) -> list[Divergence]:
    """检测递归级别快照中的所有背驰。

    Parameters
    ----------
    level_snap : RecursiveLevelSnapshot
        递归级别快照（level_id >= 2）。
    components : list[Move]
        该级别的"线段" = settled(level k-1 moves)。

    Returns
    -------
    list[Divergence]
        检测到的背驰列表（价格振幅力度）。
    """
    result: list[Divergence] = []

    for move in level_snap.moves:
        div = _detect_level_trend_divergence(
            level_snap.zhongshus, move, components, level_snap.level_id,
        )
        if div is not None:
            result.append(div)
            continue

        div = _detect_level_consolidation_divergence(
            level_snap.zhongshus, move, components, level_snap.level_id,
        )
        if div is not None:
            result.append(div)

    return result


# ── 主搜索辅助 ───────────────────────────────


def _top_level_divs_with_components(
    top_level: int,
    snap: RecursiveOrchestratorSnapshot,
) -> tuple[list[Divergence], list[Move]]:
    """获取指定级别的背驰列表和组件。无背驰时返回空列表。"""
    top_idx = top_level - 2
    if top_idx >= len(snap.recursive_snapshots):
        return [], []
    level_snap = snap.recursive_snapshots[top_idx]
    parent_moves = _get_moves_at_level(top_level - 1, snap)
    components = [m for m in parent_moves if m.settled]
    top_divs = divergences_from_level_snapshot(level_snap, components)
    return top_divs, components


def _initial_bar_range(
    top_div: Divergence,
    components: list[Move],
    top_level: int,
    snap: RecursiveOrchestratorSnapshot,
) -> tuple[int, int] | None:
    """将高级别背驰的 C 段映射到 bar 范围。无效时返回 None。"""
    c_move_idx = top_div.seg_c_end
    if c_move_idx >= len(components):
        return None
    c_move = components[c_move_idx]
    bar_range = _level_move_to_bar_range(c_move, top_level - 1, snap)
    if bar_range[0] >= bar_range[1]:
        return None
    return bar_range


def _drill_down_mid_levels(
    top_level: int,
    snap: RecursiveOrchestratorSnapshot,
    current_range: tuple[int, int],
) -> tuple[list[tuple[int, Divergence | None]], tuple[int, int]]:
    """逐级向下搜索中间级别，返回 (chain_entries, narrowed_range)。"""
    chain: list[tuple[int, Divergence | None]] = []
    for mid_level in range(top_level - 1, 1, -1):
        mid_idx = mid_level - 2
        if mid_idx >= len(snap.recursive_snapshots):
            break
        mid_snap = snap.recursive_snapshots[mid_idx]
        mid_parent = _get_moves_at_level(mid_level - 1, snap)
        mid_components = [m for m in mid_parent if m.settled]
        mid_divs = divergences_from_level_snapshot(mid_snap, mid_components)
        matched = _filter_divs_in_range(
            mid_divs, mid_components, mid_level, snap, current_range,
        )
        if not matched:
            break
        mid_div = matched[-1]  # 取最后一个（最新的）
        chain.append((mid_level, mid_div))
        if mid_div.seg_c_end < len(mid_components):
            c_comp = mid_components[mid_div.seg_c_end]
            current_range = _level_move_to_bar_range(c_comp, mid_level - 1, snap)
            if current_range[0] >= current_range[1]:
                break
    return chain, current_range


def _finalize_with_level1(
    snap: RecursiveOrchestratorSnapshot,
    current_range: tuple[int, int],
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> tuple[tuple[int, Divergence] | None, tuple[int, int]]:
    """level=1 最终检测（完整 MACD 支持），返回 (chain_entry, final_range)。"""
    l1_divs = divergences_in_bar_range(
        snap.seg_snapshot.segments,
        snap.zs_snapshot.zhongshus,
        snap.move_snapshot.moves,
        level_id=1,
        bar_range=current_range,
        df_macd=df_macd,
        merged_to_raw=merged_to_raw,
    )
    if l1_divs:
        final_range = _div_to_bar_range(l1_divs[-1], 1, snap)
        return (1, l1_divs[-1]), final_range
    return None, current_range


def _build_nested_chain(
    top_level: int,
    top_div: Divergence,
    components: list[Move],
    snap: RecursiveOrchestratorSnapshot,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> NestedDivergence | None:
    """从单个高级别背驰出发，向下构建完整嵌套链。"""
    bar_range = _initial_bar_range(top_div, components, top_level, snap)
    if bar_range is None:
        return None

    chain: list[tuple[int, Divergence | None]] = [(top_level, top_div)]
    mid_chain, current_range = _drill_down_mid_levels(top_level, snap, bar_range)
    chain.extend(mid_chain)

    l1_entry, final_range = _finalize_with_level1(
        snap, current_range, df_macd, merged_to_raw,
    )
    if l1_entry is not None:
        chain.append(l1_entry)
        current_range = final_range

    return NestedDivergence(chain=chain, bar_range=current_range)


# ── 主搜索 ────────────────────────────────────


def nested_divergence_search(
    snap: RecursiveOrchestratorSnapshot,
    *,
    df_macd: pd.DataFrame | None = None,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list[NestedDivergence]:
    """区间套跨级别背驰搜索。

    从最高递归级别开始，逐级向下检测背驰并收缩 bar 范围，
    直到 level=1。级别由递归构造决定，不接受时间周期参数。

    概念溯源: [旧缠论] 第27课 区间套（精确大转折点寻找程序定理）
    """
    if not snap.recursive_snapshots:
        return []

    max_level = len(snap.recursive_snapshots) + 1
    results: list[NestedDivergence] = []

    for top_level in range(max_level, 1, -1):
        top_divs, components = _top_level_divs_with_components(top_level, snap)
        for top_div in top_divs:
            nested = _build_nested_chain(
                top_level, top_div, components, snap, df_macd, merged_to_raw,
            )
            if nested is not None:
                results.append(nested)

    return results


def _filter_divs_in_range(
    divs: list[Divergence],
    components: list[Move],
    level: int,
    snap: RecursiveOrchestratorSnapshot,
    bar_range: tuple[int, int],
) -> list[Divergence]:
    """过滤出 C 段落入 bar_range 内的背驰。"""
    result: list[Divergence] = []
    for div in divs:
        if div.seg_c_end >= len(components):
            continue
        c_comp = components[div.seg_c_end]
        c_range = _level_move_to_bar_range(c_comp, level - 1, snap)
        if c_range[0] >= bar_range[0] and c_range[1] <= bar_range[1]:
            result.append(div)
    return result


def _div_to_bar_range(
    div: Divergence,
    level: int,
    snap: RecursiveOrchestratorSnapshot,
) -> tuple[int, int]:
    """将背驰的 C 段映射到 bar 范围。"""
    if level == 1:
        segments = snap.seg_snapshot.segments
        if div.seg_c_start >= len(segments) or div.seg_c_end >= len(segments):
            return (0, 0)
        return _seg_merged_range(segments, div.seg_c_start, div.seg_c_end)

    parent_moves = _get_moves_at_level(level - 1, snap)
    settled = [m for m in parent_moves if m.settled]
    if div.seg_c_end >= len(settled):
        return (0, 0)
    c_comp = settled[div.seg_c_end]
    return _level_move_to_bar_range(c_comp, level - 1, snap)
