"""等待状态形式化。

概念溯源：
  [新缠论] levels-bsp-v2 §八：等待状态的形式化

等待是区间套的核心纪律。
等待不是无信息的空白——WaitState 有内部结构和演化。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.nesting.bsp import BSP
from newchan.nesting.horizontal import HorizontalStep


@dataclass(frozen=True, slots=True)
class WaitState:
    """等待状态。

    §8.1:
    step: 当前阻塞在横向区间套的第几步
    scope: 当前搜索空间的标识集合
    level: 等待的递归层级
    time: 进入等待的时刻
    """

    step: HorizontalStep
    scope: frozenset[str]
    level: int
    time: float


# ── 仓位上限映射 ──

# §8.4: 每步的仓位上限（0.0~1.0）
_POSITION_LIMITS: dict[HorizontalStep, float] = {
    HorizontalStep.CONFIG_BSP: 0.1,  # 等配置层买点：轻仓
    HorizontalStep.EDGE_BSP: 0.3,  # 配置层已确认：中等仓位
    HorizontalStep.COUNTRY: 0.4,  # 独立边已确认
    HorizontalStep.SECTOR: 0.5,  # 国家已确认
    HorizontalStep.STOCK: 0.7,  # 板块已确认
    HorizontalStep.VERTICAL_ENTRY: 0.9,  # 标的已确认，等纵向收敛
}


@dataclass(frozen=True, slots=True)
class LocalOpBoundary:
    """等待状态的局部操作边界。

    §8.4:
    wait_step=1: 允许第一层局部操作，仓位上限=轻仓
    wait_step=2: 配置层方向已确认，仓位上限=中等
    wait_step=3-5: 每确认一层增加仓位上限
    wait_step=6: 标的已确认但精确时机未定，仓位上限=高但不满仓
    """

    wait_step: HorizontalStep
    max_position: float  # 仓位上限（0.0~1.0）
    allow_local_ops: bool  # 是否允许局部操作


def _build_boundary(step: HorizontalStep) -> LocalOpBoundary:
    """根据步骤构建局部操作边界。"""
    return LocalOpBoundary(
        wait_step=step,
        max_position=_POSITION_LIMITS[step],
        allow_local_ops=True,  # 第一层分析永远合法（§8.4）
    )


def can_operate(wait_state: WaitState, signal: BSP | None) -> bool:
    """是否允许操作。

    §8.3 退出条件：搜索空间内某条边出现买卖点。
    §8.3 不允许退出的原因：时间、proximity、市场在动、其他层信号。

    Parameters
    ----------
    wait_state : WaitState
        当前等待状态。
    signal : BSP | None
        当前搜索空间内的买卖点信号。

    Returns
    -------
    bool
        True 表示等待状态退出，可以操作。
    """
    if signal is None:
        return False
    if not signal.bsp_type.is_present:
        return False
    # 信号必须来自当前搜索空间
    if signal.edge_id not in wait_state.scope:
        return False
    return True


def max_position(wait_state: WaitState) -> float:
    """当前等待状态下的仓位上限。

    §8.4: LocalOpBoundary 按步骤递增。

    Parameters
    ----------
    wait_state : WaitState
        当前等待状态。

    Returns
    -------
    float
        仓位上限（0.0~1.0）。
    """
    boundary = _build_boundary(wait_state.step)
    return boundary.max_position
