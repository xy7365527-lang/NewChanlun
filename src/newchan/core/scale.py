"""ScaleSpec — 粒度规格

替代裸 tf 字符串的丰富语义载体。
遵循 CLAUDE.md 关键约束：级别 = 递归层级（level_id），
禁止用时间周期替代级别定义。
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class ScaleSpec:
    """粒度规格。

    Attributes
    ----------
    base_interval : str
        数据源原始采集周期（如 "1min"）。
    display_tf : str
        重采样后的显示周期（如 "5m", "30m"）。
    level_id : int
        递归级别，0 = 原始粒度（笔级），1 = 线段级别，……
    """

    base_interval: str
    display_tf: str
    level_id: int = 0

    def __post_init__(self) -> None:
        if not self.base_interval:
            raise ValueError("base_interval 不能为空")
        if not self.display_tf:
            raise ValueError("display_tf 不能为空")
        if self.level_id < 0:
            raise ValueError(f"level_id 不能为负: {self.level_id}")

    @property
    def canonical(self) -> str:
        """规范字符串：``{base_interval}@{display_tf}:L{level_id}``。"""
        return f"{self.base_interval}@{self.display_tf}:L{self.level_id}"
