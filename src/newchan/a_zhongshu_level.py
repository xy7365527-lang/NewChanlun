"""A 系统 — 泛化中枢构造 (级别递归)

P3: 将 zhongshu_from_segments() 和 moves_from_zhongshus() 泛化为
    接受 MoveProtocol 组件序列的版本，实现级别递归：

    Segment → zhongshu_from_components() → LevelZhongshu
           → moves_from_level_zhongshus() → Move
           → (adapt_moves) → zhongshu_from_components() → ...

核心算法与 a_zhongshu_v1 / a_move_v1 完全相同，只是：
- 输入类型从 Segment/Zhongshu 泛化为 MoveProtocol/LevelZhongshu
- 过滤条件从 confirmed 改为 completed
- 位置标识从 seg index 改为 component_idx
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Literal, Sequence

from newchan.a_level_protocol import MoveProtocol
from newchan.a_move_v1 import Move

__all__ = [
    "LevelZhongshu",
    "zhongshu_from_components",
    "moves_from_level_zhongshus",
]


# ====================================================================
# 数据类型
# ====================================================================


@dataclass(frozen=True, slots=True)
class LevelZhongshu:
    """泛化中枢：可由任意级别的 MoveProtocol 组件构造。

    Attributes
    ----------
    zd : float
        中枢下沿 = max(初始3组件的 low)。
    zg : float
        中枢上沿 = min(初始3组件的 high)。
    comp_start : int
        第一个组件的 component_idx。
    comp_end : int
        最后包含组件的 component_idx。
    comp_count : int
        构成组件数 (>= 3)。
    settled : bool
        True = 已被突破组件闭合。
    break_comp : int
        突破组件的 component_idx（-1 = 未闭合）。
    break_direction : str
        突破方向："up" / "down" / ""（未闭合时为空）。
    gg : float
        波动区间上界 GG = max(所有构成组件的 high)。
    dd : float
        波动区间下界 DD = min(所有构成组件的 low)。
    level_id : int
        中枢所属级别。
    """

    zd: float
    zg: float
    comp_start: int
    comp_end: int
    comp_count: int
    settled: bool
    break_comp: int = -1
    break_direction: str = ""
    gg: float = 0.0
    dd: float = 0.0
    level_id: int = 0


# ====================================================================
# 泛化中枢构造函数
# ====================================================================


def zhongshu_from_components(
    components: Sequence[MoveProtocol],
) -> list[LevelZhongshu]:
    """从 MoveProtocol 组件序列计算中枢。

    算法与 zhongshu_from_segments() 完全相同：
    1. 过滤 completed=True 的组件
    2. 滑窗三组件：ZD=max(lows), ZG=min(highs)
    3. ZG > ZD → 中枢成立
    4. 延伸：后续组件与 [ZD, ZG] 有交集，同时更新 GG/DD
    5. 突破：不重叠组件终结中枢
    6. 续进：从 break 组件在 completed 列表中的位置 - 2 开始
    """
    completed = [c for c in components if c.completed]
    n = len(completed)
    if n < 3:
        return []

    # 推断 level_id：取第一个 completed 组件的 level_id + 1
    level_id = completed[0].level_id + 1

    result: list[LevelZhongshu] = []
    i = 0

    while i + 2 < n:
        c1, c2, c3 = completed[i], completed[i + 1], completed[i + 2]

        # 三组件重叠检测
        zd = max(c1.low, c2.low, c3.low)
        zg = min(c1.high, c2.high, c3.high)

        if zg <= zd:
            # 无重叠，前进一步
            i += 1
            continue

        # 中枢成立：[ZD, ZG] 固定；波动区间初始化
        gg = max(c1.high, c2.high, c3.high)
        dd = min(c1.low, c2.low, c3.low)
        end_offset = i + 2

        # 尝试延伸
        j = i + 3
        while j < n:
            cj = completed[j]
            # 延伸判定：与 [ZD, ZG] 有交集（弱不等式）
            if cj.high >= zd and cj.low <= zg:
                end_offset = j
                gg = max(gg, cj.high)
                dd = min(dd, cj.low)
                j += 1
            else:
                break

        # 判断是否已被突破
        settled = j < n
        break_comp_idx = completed[j].component_idx if settled else -1

        # 突破方向
        break_dir = ""
        if settled:
            breaker = completed[j]
            if breaker.low > zg:
                break_dir = "up"
            elif breaker.high < zd:
                break_dir = "down"
            else:
                # 防御性分支（理论上弱不等式后突破必为严格不等）
                break_dir = "up" if breaker.high > zg else "down"

        result.append(LevelZhongshu(
            zd=zd,
            zg=zg,
            comp_start=completed[i].component_idx,
            comp_end=completed[end_offset].component_idx,
            comp_count=end_offset - i + 1,
            settled=settled,
            break_comp=break_comp_idx,
            break_direction=break_dir,
            gg=gg,
            dd=dd,
            level_id=level_id,
        ))

        # 续进策略：从 break 在 completed 中的位置 - 2 开始
        if settled:
            i = max(j - 2, end_offset)
        else:
            break  # 最后一个中枢未闭合，结束

    return result


# ====================================================================
# 泛化走势构造函数
# ====================================================================


def _is_ascending(c1: LevelZhongshu, c2: LevelZhongshu) -> bool:
    """后枢 DD 严格高于 前枢 GG → 上涨延续。"""
    return c2.dd > c1.gg


def _is_descending(c1: LevelZhongshu, c2: LevelZhongshu) -> bool:
    """后枢 GG 严格低于 前枢 DD → 下跌延续。"""
    return c2.gg < c1.dd


def moves_from_level_zhongshus(zhongshus: list[LevelZhongshu]) -> list[Move]:
    """从 LevelZhongshu 列表构造 Move。

    算法与 moves_from_zhongshus() 完全相同：
    1. 过滤 settled=True 的中枢
    2. 贪心分组：同向中枢归入同一 group
    3. >=2 同向中枢 → trend；1 中枢 → consolidation
    4. 最后一个 move → settled=False

    注意：返回的 Move 中 seg_start/seg_end 指向 comp_start/comp_end，
    zs_start/zs_end 指向中枢在 zhongshus 列表中的索引。
    """
    # 过滤 settled 中枢并记录原始索引
    settled_indices: list[int] = []
    settled_zs: list[LevelZhongshu] = []
    for idx, zs in enumerate(zhongshus):
        if zs.settled:
            settled_indices.append(idx)
            settled_zs.append(zs)

    if not settled_zs:
        return []

    # 贪心分组
    groups: list[tuple[list[int], str]] = []
    current_offsets: list[int] = [0]
    current_dir: str = ""

    for k in range(1, len(settled_zs)):
        prev_zs = settled_zs[k - 1]
        curr_zs = settled_zs[k]

        if _is_ascending(prev_zs, curr_zs) and current_dir in ("", "up"):
            current_offsets.append(k)
            current_dir = "up"
        elif _is_descending(prev_zs, curr_zs) and current_dir in ("", "down"):
            current_offsets.append(k)
            current_dir = "down"
        else:
            groups.append((current_offsets, current_dir))
            current_offsets = [k]
            current_dir = ""

    groups.append((current_offsets, current_dir))

    # 转换 groups → Moves
    result: list[Move] = []
    for offsets, direction in groups:
        first_zs = settled_zs[offsets[0]]
        last_zs = settled_zs[offsets[-1]]
        zs_count = len(offsets)

        zs_start = settled_indices[offsets[0]]
        zs_end = settled_indices[offsets[-1]]

        if zs_count >= 2:
            kind: Literal["consolidation", "trend"] = "trend"
            move_dir: Literal["up", "down"] = direction  # type: ignore[assignment]
        else:
            kind = "consolidation"
            move_dir = first_zs.break_direction  # type: ignore[assignment]

        group_centers = [settled_zs[o] for o in offsets]
        result.append(Move(
            kind=kind,
            direction=move_dir,
            seg_start=first_zs.comp_start,
            seg_end=last_zs.comp_end,
            zs_start=zs_start,
            zs_end=zs_end,
            zs_count=zs_count,
            settled=True,
            high=max(zs.zg for zs in group_centers),
            low=min(zs.zd for zs in group_centers),
            first_seg_s0=first_zs.comp_start,
            last_seg_s1=last_zs.comp_end,
        ))

    # 最后一个 move → unsettled
    if result:
        result[-1] = replace(result[-1], settled=False)

    return result
