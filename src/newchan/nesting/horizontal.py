"""横向区间套六步状态机。

概念溯源：
  [新缠论] levels-bsp-v2 §一：横向区间套六步
  六步不是六种操作，是同一个操作 wait_divergence -> confirm_BSP -> narrow_scope 的六次应用。

§1.3 递归不变量：
  Invariant(step_i):
    入口 = 上一步产出的买卖点确认
    操作 = 在当前层的比价线集合中等背驰、确认买卖点
    产出 = 流向确认 ∧ 介入目标 ∧ 进入下一步的资格

§1.4 阻塞与跳过：
  阻塞规则：若 step_i 无买卖点出现 -> 阻塞在 step_i
  跳过规则：仅 step_3（选国家）可跳过
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.nesting.bsp import BSP


class HorizontalStep(Enum):
    """横向区间套六步。"""

    CONFIG_BSP = 1  # 第一步：配置层出现买点
    EDGE_BSP = 2  # 第二步：独立边出现买点
    COUNTRY = 3  # 第三步：选国家
    SECTOR = 4  # 第四步：角内选板块
    STOCK = 5  # 第五步：板块内选标的
    VERTICAL_ENTRY = 6  # 第六步：标的内纵向区间套

    @property
    def can_skip(self) -> bool:
        """仅第三步可跳过（§1.4）。"""
        return self is HorizontalStep.COUNTRY

    @property
    def next_step(self) -> HorizontalStep | None:
        """下一步（第六步无后续）。"""
        members = list(HorizontalStep)
        idx = members.index(self)
        if idx + 1 < len(members):
            return members[idx + 1]
        return None


@dataclass(frozen=True, slots=True)
class StepResult:
    """单步执行结果。"""

    step: HorizontalStep
    bsp: BSP | None  # 本步确认的买卖点（None = 阻塞）
    target: str  # 本步产出的目标（角/国家/板块/标的/空串）
    skipped: bool = False  # 是否跳过（仅第三步可能为 True）


@dataclass(frozen=True, slots=True)
class HorizontalNesting:
    """横向区间套六步状态机。

    不可变设计：每次 advance 返回新实例。

    Attributes
    ----------
    current_step : HorizontalStep
        当前所在步骤。
    completed_results : tuple[StepResult, ...]
        已完成的步骤结果。
    blocked : bool
        当前步是否阻塞（等待买卖点）。
    """

    current_step: HorizontalStep
    completed_results: tuple[StepResult, ...] = ()
    blocked: bool = True  # 初始状态 = 阻塞在第一步

    @property
    def is_complete(self) -> bool:
        """六步全部完成。"""
        return (
            len(self.completed_results) >= len(HorizontalStep)
            or (
                len(self.completed_results) > 0
                and self.completed_results[-1].step is HorizontalStep.VERTICAL_ENTRY
            )
        )


def create_horizontal_nesting() -> HorizontalNesting:
    """创建初始横向区间套状态机。"""
    return HorizontalNesting(current_step=HorizontalStep.CONFIG_BSP)


def entry_condition(state: HorizontalNesting, bsp: BSP | None) -> bool:
    """当前步的入口条件是否满足。

    §1.2 每步入口条件的统一形式：
    - 第一步：无前置条件（等配置层买卖点即可）
    - 其余步：上一步已完成（有 bsp 确认）
    """
    step = state.current_step
    if step is HorizontalStep.CONFIG_BSP:
        return bsp is not None and bsp.bsp_type.is_present
    # 其余步：上一步必须已完成
    if len(state.completed_results) == 0:
        return False
    prev = state.completed_results[-1]
    if prev.bsp is None and not prev.skipped:
        return False
    return bsp is not None and bsp.bsp_type.is_present


def exit_condition(state: HorizontalNesting, bsp: BSP | None) -> bool:
    """当前步的出口条件是否满足。

    出口条件 = 入口条件（买卖点确认即同时满足入口和出口——三位一体）。
    """
    return entry_condition(state, bsp)


def is_blocked(state: HorizontalNesting) -> bool:
    """当前步是否阻塞。

    阻塞 = 当前搜索空间内无买卖点。
    等待是区间套的核心纪律。
    """
    return state.blocked


def advance(
    state: HorizontalNesting,
    bsp: BSP | None,
    target: str = "",
) -> HorizontalNesting:
    """推进状态机。

    Parameters
    ----------
    state : HorizontalNesting
        当前状态。
    bsp : BSP | None
        当前步确认的买卖点（None 表示继续阻塞）。
    target : str
        本步产出的目标标识。

    Returns
    -------
    HorizontalNesting
        新状态。
    """
    step = state.current_step

    # 无买卖点 -> 继续阻塞（除非可跳过）
    if bsp is None or not bsp.bsp_type.is_present:
        if step.can_skip:
            result = StepResult(step=step, bsp=None, target=target, skipped=True)
            next_step = step.next_step
            if next_step is None:
                return HorizontalNesting(
                    current_step=step,
                    completed_results=state.completed_results + (result,),
                    blocked=False,
                )
            return HorizontalNesting(
                current_step=next_step,
                completed_results=state.completed_results + (result,),
                blocked=True,
            )
        return HorizontalNesting(
            current_step=step,
            completed_results=state.completed_results,
            blocked=True,
        )

    # 有买卖点 -> 完成当前步，推进到下一步
    result = StepResult(step=step, bsp=bsp, target=target)
    next_step = step.next_step
    if next_step is None:
        # 第六步完成
        return HorizontalNesting(
            current_step=step,
            completed_results=state.completed_results + (result,),
            blocked=False,
        )
    return HorizontalNesting(
        current_step=next_step,
        completed_results=state.completed_results + (result,),
        blocked=True,
    )
