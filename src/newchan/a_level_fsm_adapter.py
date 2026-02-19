"""A 系统 — L* 裁决适配器（新递归链 → FSM 桥接）

将 RecursiveOrchestratorSnapshot 转换为 LevelView 列表，
供 select_lstar_newchan() 使用。

类型映射：
  Zhongshu (a_zhongshu_v1) → Center (a_center_v0)
  LevelZhongshu (a_zhongshu_level) → Center (a_center_v0)
  Move (a_move_v1) → 鸭子类型兼容 Segment（.low/.high/.direction）

索引安全性依据：
  未确认/未结算对象只出现在列表末尾（引擎不变量），
  因此过滤列表索引 = 全列表索引（对已确认部分）。
"""

from __future__ import annotations

from newchan.a_center_v0 import Center
from newchan.a_level_fsm_newchan import LevelView, LStar, select_lstar_newchan
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot

__all__ = [
    "zhongshu_to_center",
    "level_zhongshu_to_center",
    "level_views_from_recursive_snapshot",
    "select_lstar_from_recursive_snapshot",
]


# ====================================================================
# 类型转换
# ====================================================================


def zhongshu_to_center(zs: Zhongshu) -> Center:
    """Level-1 Zhongshu → Center（供 FSM 使用）。

    seg_start/seg_end 在 Zhongshu 中是已确认段列表索引，
    因为未确认段只在末尾，所以 = 全段列表索引。
    """
    return Center(
        seg0=zs.seg_start,
        seg1=zs.seg_end,
        low=zs.zd,
        high=zs.zg,
        kind="settled" if zs.settled else "candidate",
        confirmed=True,
        sustain=max(0, zs.seg_count - 3),
        direction=zs.break_direction,
        gg=zs.gg,
        dd=zs.dd,
    )


def level_zhongshu_to_center(lzs: LevelZhongshu) -> Center:
    """递归级别 LevelZhongshu → Center（供 FSM 使用）。

    comp_start/comp_end 在 LevelZhongshu 中是已结算 moves 列表索引，
    因为未结算 move 只在末尾，所以 = 全 moves 列表索引。
    """
    return Center(
        seg0=lzs.comp_start,
        seg1=lzs.comp_end,
        low=lzs.zd,
        high=lzs.zg,
        kind="settled" if lzs.settled else "candidate",
        confirmed=True,
        sustain=max(0, lzs.comp_count - 3),
        direction=lzs.break_direction,
        gg=lzs.gg,
        dd=lzs.dd,
    )


# ====================================================================
# 主适配函数
# ====================================================================


def level_views_from_recursive_snapshot(
    snap: RecursiveOrchestratorSnapshot,
) -> list[LevelView]:
    """从 RecursiveOrchestratorSnapshot 构建 LevelView 列表。

    每个 LevelView 配对：
    - segments = 该级别中枢构造的输入（段/走势列表）
    - centers = 该级别中枢（转换为 Center 对象）

    Level 1: segments = seg_snapshot.segments, centers 来自 zs_snapshot
    Level N (N≥2): segments = 前一级别的 moves, centers 来自 recursive_snapshots
    """
    views: list[LevelView] = []

    # ── Level 1 ──
    l1_centers = [
        zhongshu_to_center(zs)
        for zs in snap.zs_snapshot.zhongshus
    ]
    if l1_centers:
        views.append(LevelView(
            level=1,
            segments=list(snap.seg_snapshot.segments),
            centers=l1_centers,
        ))

    # ── Level 2+ ──
    # 前一级别的全部 moves（含末尾 unsettled）作为本级别的 "segments"
    prev_moves: list = list(snap.move_snapshot.moves)

    for rs in snap.recursive_snapshots:
        l_centers = [
            level_zhongshu_to_center(lzs)
            for lzs in rs.zhongshus
        ]
        if l_centers:
            views.append(LevelView(
                level=rs.level_id,
                segments=prev_moves,
                centers=l_centers,
            ))
        # 下一级别的输入 = 本级别的全部 moves
        prev_moves = list(rs.moves)

    return views


def select_lstar_from_recursive_snapshot(
    snap: RecursiveOrchestratorSnapshot,
    last_price: float,
) -> LStar | None:
    """从 RecursiveOrchestratorSnapshot 直接选择 L*。

    等价于 level_views_from_recursive_snapshot + select_lstar_newchan。
    """
    views = level_views_from_recursive_snapshot(snap)
    if not views:
        return None
    return select_lstar_newchan(views, last_price)
