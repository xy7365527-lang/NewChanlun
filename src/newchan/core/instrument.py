"""InstrumentId — 标的身份标识符

复用 data_databento.SYMBOL_CATALOG 的品种元数据，
将非正式的 (symbol, type, exchange) 三元组固化为 frozen 值对象。
"""

from __future__ import annotations

from dataclasses import dataclass

# 合法的标的类型
_VALID_INST_TYPES = frozenset({"FUT", "STK", "ETF", "SPREAD"})


@dataclass(frozen=True, slots=True)
class InstrumentId:
    """标的身份。

    Attributes
    ----------
    symbol : str
        品种代码，全大写（如 "BZ", "AAPL"）。
    inst_type : str
        标的类型：FUT / STK / ETF / SPREAD。
    exchange : str
        交易所代码（如 "CME", "NYMEX", "NASDAQ"）。
    """

    symbol: str
    inst_type: str
    exchange: str

    def __post_init__(self) -> None:
        if not self.symbol:
            raise ValueError("symbol 不能为空")
        if self.symbol != self.symbol.upper():
            raise ValueError(f"symbol 必须全大写: {self.symbol!r}")
        if self.inst_type not in _VALID_INST_TYPES:
            raise ValueError(
                f"无效 inst_type: {self.inst_type!r}，"
                f"允许值: {sorted(_VALID_INST_TYPES)}"
            )
        if not self.exchange:
            raise ValueError("exchange 不能为空")

    @property
    def canonical(self) -> str:
        """规范字符串：``{exchange}:{symbol}``，用于 StreamId 构造。"""
        return f"{self.exchange}:{self.symbol}"
