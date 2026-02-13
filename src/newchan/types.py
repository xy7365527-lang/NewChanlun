"""核心数据类型"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime


@dataclass(slots=True)
class Bar:
    """单根 K 线"""

    ts: datetime
    open: float
    high: float
    low: float
    close: float
    volume: float | None = None
