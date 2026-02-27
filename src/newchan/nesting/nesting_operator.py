"""统一区间套算子。

概念溯源：
  [新缠论] levels-bsp-v2 §二：统一区间套算子 N
  N : (SearchSpace, Level) -> (Target, BSP_confirmation)

纵向和横向区间套都是 N 的实例。
交织路径 = N 的有限序列，受三个结构性约束。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.nesting.bsp import BSP


class NestingType(Enum):
    """区间套类型。"""

    HORIZONTAL = "horizontal"  # 横向：搜索空间收缩（配置→角→板块→标的）
    VERTICAL = "vertical"  # 纵向：级别下钻（同一标的，级别 k → k-1）


@dataclass(frozen=True, slots=True)
class NestingStep:
    """区间套路径中的一步。

    N : (search_space, level) -> (target, bsp_confirmation)
    """

    nesting_type: NestingType
    search_space: frozenset[str]  # 当前搜索空间的标识集合
    level: int  # 当前层级
    target: str  # 买卖点指向的下一层目标
    bsp_confirmation: BSP | None  # 买卖点确认记录


@dataclass(frozen=True, slots=True)
class NestingPath:
    """区间套路径：NestingStep 的有限序列。

    §2.2 约束：
    - 约束1（级别单调性）：沿路径级别序列 k₁ >= k₂ >= ... >= kₘ
    - 约束2（搜索空间收缩性）：|S₁| >= |S₂| >= ... >= |Sₘ|
    - 约束3（终止性）：Path 必终止于具体标的的具体级别上的买卖点
    """

    steps: tuple[NestingStep, ...]

    @property
    def is_empty(self) -> bool:
        return len(self.steps) == 0

    @property
    def current_level(self) -> int | None:
        if self.is_empty:
            return None
        return self.steps[-1].level

    @property
    def current_target(self) -> str | None:
        if self.is_empty:
            return None
        return self.steps[-1].target


def check_level_monotonicity(path: NestingPath) -> bool:
    """约束1：级别单调性。

    沿路径，级别序列 k₁ >= k₂ >= ... >= kₘ。
    区间套只能向低级别下钻，不能向上跳。
    """
    steps = path.steps
    for i in range(1, len(steps)):
        if steps[i].level > steps[i - 1].level:
            return False
    return True


def check_search_space_contraction(path: NestingPath) -> bool:
    """约束2：搜索空间收缩性。

    |S₁| >= |S₂| >= ... >= |Sₘ|。
    横向的每一步收缩搜索空间（全局→角→板块→标的）。
    纵向的每一步收缩时间范围（大级别背驰区域→小级别精确点）。
    """
    steps = path.steps
    for i in range(1, len(steps)):
        if len(steps[i].search_space) > len(steps[i - 1].search_space):
            return False
    return True


def check_termination(path: NestingPath) -> bool:
    """约束3：终止性。

    Path 必终止于某个具体标的的某个具体级别上的买卖点。
    终止条件：最后一步有 bsp_confirmation 且 search_space 为单元素。
    """
    if path.is_empty:
        return False
    last = path.steps[-1]
    return (
        last.bsp_confirmation is not None
        and last.bsp_confirmation.bsp_type.is_present
        and len(last.search_space) == 1
    )


def validate_path(path: NestingPath) -> tuple[bool, list[str]]:
    """验证区间套路径的所有约束。

    Returns
    -------
    (is_valid, violations) : tuple[bool, list[str]]
        is_valid 为 True 时 violations 为空列表。
    """
    violations: list[str] = []
    if not check_level_monotonicity(path):
        violations.append("level_monotonicity_violated")
    if not check_search_space_contraction(path):
        violations.append("search_space_contraction_violated")
    if not check_termination(path):
        violations.append("termination_not_reached")
    return (len(violations) == 0, violations)


def append_step(path: NestingPath, step: NestingStep) -> NestingPath:
    """向路径追加一步（不可变——返回新路径）。"""
    return NestingPath(steps=path.steps + (step,))
