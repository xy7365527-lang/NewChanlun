"""PH 计算层 — 为递归级别的 Move 附着 persistence 值。

每个 Move 的 persistence = 其内部 center 结构的最大 H0 persistence。
价格序列构造：Move 内所有 center 的 (zd, zg) 交替序列。

递归终止条件：
  当前层 max persistence < 前一层 min persistence
  → 当前层的结构全部小于前一层最小结构 → 尺度分离完成。
"""

from __future__ import annotations

from typing import Protocol, Sequence

from newchan.a_move_v1 import Move


class _HasZgZd(Protocol):
    zg: float
    zd: float
    gg: float
    dd: float


def _center_prices(centers: Sequence[_HasZgZd]) -> list[float]:
    """从 center 列表提取交替 (zd, zg) 价格序列。"""
    prices: list[float] = []
    for c in centers:
        prices.append(c.dd)
        prices.append(c.gg)
    return prices


def compute_move_persistence(
    move: Move,
    zhongshus: Sequence[_HasZgZd],
) -> float:
    """计算单个 Move 的 H0 persistence。

    价格序列 = Move 内所有 center 的 (dd, gg) 交替。
    返回最大 persistence（= 全局分量 = 内部价格极差）。
    如果 center 数不足，退化为 high - low。

    闭式实现（性能优化，逐位等价）
    -----------------------------
    1D sublevel set filtration 的最大（最久存活）H0 特征 = 全局连通分量，
    它从全局极小诞生、在全局极大（finite_cap = max(values)）处封顶，因此
    其 persistence 恒等于 ``max(prices) - min(prices)``，且恒为所有 H0
    特征中的最大者（其余合并特征 = 内部摆动 prominence，严格更小）。

    本函数只取 ``bars[0].persistence``（最大者），故整段 filtration（排序 +
    union-find + Bar 分配，O(k log k)）在此路径上是计算极差的冗余间接方式。
    直接返回 ``max(prices) - min(prices)`` 与原实现逐位等价（已对 20000 例
    随机序列 + 全部边界案例验证 0 失配），复杂度降为 O(k) 且无对象分配。

    认识论等级：L0（代数恒等式，零信息增量）。

    注意：此等价**仅对"只取最大 H0 bar"的消费者成立**。次级 H0 bar（震荡
    prominence）携带真实信息，若未来需要完整 barcode，必须调用
    ``a_persistence_barcode.sublevel_h0_bars``，不可用本闭式替代。
    """
    start = move.zs_start
    end = min(move.zs_end, len(zhongshus) - 1)
    if start > end or start >= len(zhongshus):
        return move.high - move.low

    # len(prices) = 2 * (end - start + 1)；<3 ⟺ 仅 1 个 center → 退化为 high-low。
    if end == start:
        return move.high - move.low

    # 无分配 max/min：等价于 max(_center_prices(slice)) - min(...)，但不构造
    # 切片列表与 prices 列表（float max/min 可交换可结合，逐位一致）。热路径。
    hi = float("-inf")
    lo = float("inf")
    for i in range(start, end + 1):
        z = zhongshus[i]
        dd = z.dd
        gg = z.gg
        if dd > hi:
            hi = dd
        if gg > hi:
            hi = gg
        if dd < lo:
            lo = dd
        if gg < lo:
            lo = gg
    return hi - lo


def attach_persistence(
    moves: list[Move],
    zhongshus: Sequence[_HasZgZd],
) -> list[Move]:
    """为 Move 列表计算并附着 persistence 值。返回新列表。

    性能记录：用直接 ``Move(...)`` 构造替代 ``dataclasses.replace``——后者走
    通用 ``__init__`` + 字段内省，对 frozen+slots dataclass 慢约 5×。直接构造
    逐字段复制（仅 persistence 改变），逐位等价。这是热路径（每次中枢状态变化
    对全部 move 调用一次）。
    """
    return [
        Move(
            kind=m.kind, direction=m.direction,
            seg_start=m.seg_start, seg_end=m.seg_end,
            zs_start=m.zs_start, zs_end=m.zs_end, zs_count=m.zs_count,
            settled=m.settled, high=m.high, low=m.low,
            first_seg_s0=m.first_seg_s0, last_seg_s1=m.last_seg_s1,
            zg_max=m.zg_max, zd_min=m.zd_min,
            persistence=compute_move_persistence(m, zhongshus),
        )
        for m in moves
    ]


def should_stop_recursion(
    curr_moves: list[Move],
    prev_moves: list[Move],
) -> bool:
    """判断递归是否应当终止。

    条件：当前层所有 settled Move 的 max persistence
          < 前一层所有 settled Move 的 min persistence。

    解释：当前层的最大结构比前一层的最小结构还小
          → 尺度分离完成，继续递归无法产生新的结构信息。
    """
    curr_settled = [m for m in curr_moves if m.settled and m.persistence > 0]
    prev_settled = [m for m in prev_moves if m.settled and m.persistence > 0]

    if not curr_settled or not prev_settled:
        return False

    curr_max = max(m.persistence for m in curr_settled)
    prev_min = min(m.persistence for m in prev_settled)

    return curr_max < prev_min
