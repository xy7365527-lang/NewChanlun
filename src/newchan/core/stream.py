"""StreamId — 流标识

唯一标识一条事件流。
stream_id = InstrumentId + ScaleSpec + source 的确定性组合。
使用格式化字符串（可读、可调试）而非纯哈希。
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass

from newchan.core.instrument import InstrumentId
from newchan.core.scale import ScaleSpec


@dataclass(frozen=True, slots=True)
class StreamId:
    """流标识。

    Attributes
    ----------
    instrument : InstrumentId
        标的身份。
    scale : ScaleSpec
        粒度规格。
    source : str
        数据来源："replay" / "live" / "backtest"。
    """

    instrument: InstrumentId
    scale: ScaleSpec
    source: str = "replay"

    def __post_init__(self) -> None:
        if not self.source:
            raise ValueError("source 不能为空")

    @property
    def value(self) -> str:
        """可读的规范字符串。

        格式: ``{exchange}:{symbol}/{base_interval}@{display_tf}:L{level_id}/{source}``
        示例: ``CME:BZ/1min@5m:L0/replay``
        """
        return f"{self.instrument.canonical}/{self.scale.canonical}/{self.source}"

    @property
    def short_hash(self) -> str:
        """12 hex 短哈希，用于日志/指标标签。"""
        return hashlib.sha256(self.value.encode("utf-8")).hexdigest()[:12]

    def __str__(self) -> str:
        return self.value

    def __eq__(self, other: object) -> bool:
        if isinstance(other, StreamId):
            return self.value == other.value
        return NotImplemented

    def __hash__(self) -> int:
        return hash(self.value)
