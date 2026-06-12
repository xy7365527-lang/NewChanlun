"""配置层：标的定义 + broker 连接配置。"""

from trading_system.config.instruments import (
    INSTRUMENTS,
    InstrumentSpec,
    make_instrument,
)

__all__ = ["INSTRUMENTS", "InstrumentSpec", "make_instrument"]
