"""买卖点类型定义（区间套语境）。

概念溯源：
  [新缠论] levels-bsp-v2 §四：买卖点=流向确认=介入信号
  [新缠论] levels-bsp-v2 §4.5：买卖点否定（对象自我否定）
  [旧缠论] 第17课、第20课、第21课

此模块定义区间套语境下的买卖点类型和否定判定。
与 a_buysellpoint_v1.py 的区别：
  a_buysellpoint_v1 是单条比价线内的买卖点识别（纵向）。
  本模块是区间套路径上的买卖点状态描述（横向+纵向统一视角）。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class BSPType(Enum):
    """买卖点类型。

    B1/B2/B3 = 第一/二/三类买点
    S1/S2/S3 = 第一/二/三类卖点
    NONE = 无买卖点
    """

    B1 = "1B"
    B2 = "2B"
    B3 = "3B"
    S1 = "1S"
    S2 = "2S"
    S3 = "3S"
    NONE = "none"

    @property
    def is_buy(self) -> bool:
        return self in (BSPType.B1, BSPType.B2, BSPType.B3)

    @property
    def is_sell(self) -> bool:
        return self in (BSPType.S1, BSPType.S2, BSPType.S3)

    @property
    def is_present(self) -> bool:
        return self is not BSPType.NONE


class DivergenceType(Enum):
    """背驰类型。"""

    TOP_DIV = "top_div"
    BOT_DIV = "bot_div"
    NONE = "none"


@dataclass(frozen=True, slots=True)
class BSP:
    """区间套语境下的买卖点实例。

    比 a_buysellpoint_v1.BuySellPoint 更轻量：
    只携带区间套路径推进所需的信息。
    """

    edge_id: str  # 比价线标识
    level: int  # 递归层级
    time: float  # 时刻（epoch seconds 或 bar_idx）
    bsp_type: BSPType
    price: float  # 买卖点价格
    divergence: DivergenceType = DivergenceType.NONE
    negated: bool = False  # 是否已被否定

    @property
    def direction_is_buy(self) -> bool:
        return self.bsp_type.is_buy


def _is_buy_negated(bsp: BSP, current_price: float) -> bool:
    """买点否定判定。

    §4.5:
    - 一买否定：价格跌破一买前低点（此处近似为 bsp.price）
    - 二买否定：价格跌破一买低点（以一买价格为基准，此处近似为 bsp.price）
    - 三买否定：价格跌回中枢内部（此处近似为跌破 bsp.price）

    共同简化：current_price < bsp.price 即视为否定。
    严格实现需要前低点/中枢区间信息，由上层传入。
    """
    return current_price < bsp.price


def _is_sell_negated(bsp: BSP, current_price: float) -> bool:
    """卖点否定判定（与买点对称）。

    - 一卖否定：价格突破一卖前高点
    - 二卖否定：价格突破一卖高点
    - 三卖否定：价格涨回中枢内部
    """
    return current_price > bsp.price


def is_negated(bsp: BSP, current_price: float) -> bool:
    """买卖点否定判定。

    §4.5: 否定不是外加的止损规则，是走势结构的自我否定。
    BSP 否定 <=> 流向确认撤销 <=> 介入信号失效。

    Parameters
    ----------
    bsp : BSP
        待检查的买卖点。
    current_price : float
        当前价格。

    Returns
    -------
    bool
        True 表示买卖点已被否定。
    """
    if not bsp.bsp_type.is_present:
        return False
    if bsp.bsp_type.is_buy:
        return _is_buy_negated(bsp, current_price)
    return _is_sell_negated(bsp, current_price)
