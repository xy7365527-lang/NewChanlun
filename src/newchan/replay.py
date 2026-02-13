"""回放会话管理 — 逐 bar 重放引擎状态

通过 BiEngine 逐 bar 驱动，支持步进、跳转、自动播放。
每个 ReplaySession 绑定一组固定的 bar 数据和一个独立的引擎实例。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.types import Bar


@dataclass
class ReplaySession:
    """管理单个回放会话的状态。

    Attributes
    ----------
    session_id : str
        唯一会话标识（UUID）。
    bars : list[Bar]
        完整的 bar 序列（只读，回放数据源）。
    engine : BiEngine
        笔事件引擎实例（每个会话独立）。
    current_idx : int
        下一根待处理 bar 的索引（已处理 bar 数量）。
    total_bars : int
        bar 总数。
    mode : str
        当前模式：idle / playing / paused / done。
    speed : float
        自动播放倍速（1.0 = 基准速度）。
    event_log : list[BiEngineSnapshot]
        历史快照记录（可选，用于调试）。
    """

    session_id: str
    bars: list[Bar]
    engine: BiEngine
    current_idx: int = 0
    total_bars: int = 0
    mode: Literal["idle", "playing", "paused", "done"] = "idle"
    speed: float = 1.0
    event_log: list[BiEngineSnapshot] = field(default_factory=list)

    def __post_init__(self) -> None:
        self.total_bars = len(self.bars)

    def step(self, count: int = 1) -> list[BiEngineSnapshot]:
        """步进 count 根 bar，返回每一步的快照。

        如果剩余 bar 不足 count 根，则处理到末尾。
        到达末尾后 mode 变为 "done"。
        """
        snapshots: list[BiEngineSnapshot] = []
        for _ in range(count):
            if self.current_idx >= self.total_bars:
                self.mode = "done"
                break
            snap = self.engine.process_bar(self.bars[self.current_idx])
            self.current_idx += 1
            snapshots.append(snap)
            self.event_log.append(snap)

        # 到达末尾
        if self.current_idx >= self.total_bars:
            self.mode = "done"

        return snapshots

    def seek(self, target_idx: int) -> BiEngineSnapshot | None:
        """跳转到指定位置。重置引擎，从头重跑到 target_idx。

        target_idx 是目标 bar 索引（0-based，含该 bar）。
        返回跳转后的最终快照；如果 target_idx <= 0 则只重置，返回 None。
        """
        # 限制范围
        target_idx = max(0, min(target_idx, self.total_bars - 1))

        # 重置引擎和状态
        self.engine.reset()
        self.current_idx = 0
        self.event_log.clear()

        # 从头重跑到 target_idx（含）
        snap: BiEngineSnapshot | None = None
        for i in range(target_idx + 1):
            snap = self.engine.process_bar(self.bars[i])
            self.current_idx = i + 1
            self.event_log.append(snap)

        # 更新模式
        if self.current_idx >= self.total_bars:
            self.mode = "done"
        elif self.mode == "done":
            self.mode = "paused"

        return snap

    def get_status(self) -> dict:
        """返回当前回放状态摘要。"""
        return {
            "session_id": self.session_id,
            "mode": self.mode,
            "current_idx": self.current_idx,
            "total_bars": self.total_bars,
            "speed": self.speed,
        }
