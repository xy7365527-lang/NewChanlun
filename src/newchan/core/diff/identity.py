"""Identity / state key helpers — 五层同构 diff 的身份分离工具。

每个 domain entity（笔/线段/中枢/走势类型/买卖点）都有 *身份*（identity）和 *状态*（state）。
身份标识"它是谁"，状态标识"它现在怎样"。

Diff 规则：
- identity 相同 + state 变化 → 禁止 invalidate，改为 emit 升级/更新事件
- identity 消失 → 允许 emit invalidate
"""

from __future__ import annotations

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.a_move_v1 import Move
from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu


# ── Segment ──────────────────────────────────────────────


def segment_identity_key(seg: Segment) -> tuple[int, str]:
    """段身份 = (起点 stroke 索引, 方向)。"""
    return (seg.s0, seg.direction)


def same_segment_identity(a: Segment, b: Segment) -> bool:
    """两段是否具有同一身份（s0 + direction 相同）。"""
    return a.s0 == b.s0 and a.direction == b.direction


# ── Zhongshu ─────────────────────────────────────────────


def zhongshu_identity_key(zs: Zhongshu) -> tuple[float, float, int]:
    """中枢身份 = (ZD, ZG, 首段索引)。"""
    return (zs.zd, zs.zg, zs.seg_start)


def same_zhongshu_identity(a: Zhongshu, b: Zhongshu) -> bool:
    """两个中枢是否具有同一身份（zd + zg + seg_start 相同）。"""
    return a.zd == b.zd and a.zg == b.zg and a.seg_start == b.seg_start


# ── Move ────────────────────────────────────────────────


def move_identity_key(m: Move) -> tuple[int]:
    """走势类型身份 = (首段索引,)。稳定，不随趋势升级而变。"""
    return (m.seg_start,)


def same_move_identity(a: Move, b: Move) -> bool:
    """两个 Move 是否具有同一身份（seg_start 相同）。"""
    return a.seg_start == b.seg_start


# ── BuySellPoint ──────────────────────────────────────────


def bsp_identity_key(bp: BuySellPoint) -> tuple[int, str, str, int]:
    """买卖点身份 = (seg_idx, kind, side, level_id)。四元组唯一键。"""
    return (bp.seg_idx, bp.kind, bp.side, bp.level_id)


def same_bsp_identity(a: BuySellPoint, b: BuySellPoint) -> bool:
    """两个买卖点是否具有同一身份。"""
    return (
        a.seg_idx == b.seg_idx
        and a.kind == b.kind
        and a.side == b.side
        and a.level_id == b.level_id
    )
