"""扫描器→赋格状态机接口 -- scanner 输出 rep([S]) 接入 fugue SCANNING→POSITION_OPEN。

352号 §8 下游推论2：
  "扫描器输出的 rep([S]) 是349号 SCANNING → POSITION_OPEN 转移的输入。
   扫描器给出'操作哪个标的'，赋格状态机给出'怎么操作'。"

本模块实现两者之间的接口：
  1. scanner_to_fugue_event: 将 rep([S]) 转换为 BUY_POINT_CONFIRMED 事件
  2. try_scanner_to_fugue_transition: 安全地尝试驱动赋格状态机转移

认识论标注：L0（从349号和352号定义直接推导）。
谱系引用：349号赋格状态机、352号多标的扫描器。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.fugue_engine import (
    FugueEngine,
    FugueEvent,
    FugueEventType,
    FugueState,
)
from newchan.trading.fold_equivalence import TargetAttributes
from newchan.trading.scanner_pool import ScannerPoolResult


# ═══════════════════════════════════════════════════════════════
# 转换：rep([S]) → FugueEvent
# ═══════════════════════════════════════════════════════════════


def scanner_to_fugue_event(
    representative: TargetAttributes,
    price: float,
) -> FugueEvent:
    """将扫描器代表元转换为赋格状态机的 BUY_POINT_CONFIRMED 事件。

    映射规则：
    - event_type: BUY_POINT_CONFIRMED（区间套收敛到一买）
    - price: 当前价格（由调用方提供）
    - level: 从代表元的 level_magnitude 映射为 "L{n}" 字符串

    Parameters
    ----------
    representative : TargetAttributes
        扫描器选出的等价类代表元 rep([S])。
    price : float
        买入价格。

    Returns
    -------
    FugueEvent
        可直接用于 FugueEngine.apply() 的事件。
    """
    level_str = f"L{int(representative.level_magnitude)}"
    return FugueEvent(
        event_type=FugueEventType.BUY_POINT_CONFIRMED,
        price=price,
        level=level_str,
    )


# ═══════════════════════════════════════════════════════════════
# 转移结果
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class ScannerFugueTransition:
    """扫描器到赋格状态机的转移结果。

    Attributes
    ----------
    selected_symbol : str
        被选中的标的代码。
    event : FugueEvent
        用于驱动转移的事件。
    new_engine : FugueEngine
        转移后的赋格状态机实例。
    """

    selected_symbol: str
    event: FugueEvent
    new_engine: FugueEngine


# ═══════════════════════════════════════════════════════════════
# 安全转移
# ═══════════════════════════════════════════════════════════════


def try_scanner_to_fugue_transition(
    pool_result: ScannerPoolResult,
    engine: FugueEngine,
    price: float,
) -> ScannerFugueTransition | None:
    """尝试将扫描器输出接入赋格状态机。

    前置条件：
    1. pool_result 有代表元（top_representative is not None）
    2. engine 在 SCANNING 状态

    如果前置条件不满足，返回 None。

    Parameters
    ----------
    pool_result : ScannerPoolResult
        标的池生成结果。
    engine : FugueEngine
        当前赋格状态机实例。
    price : float
        买入价格。

    Returns
    -------
    ScannerFugueTransition | None
        转移成功返回结果，条件不满足返回 None。
    """
    if pool_result.top_representative is None:
        return None
    if engine.state is not FugueState.SCANNING:
        return None

    rep = pool_result.top_representative
    event = scanner_to_fugue_event(rep, price)
    new_engine = engine.apply(event)

    return ScannerFugueTransition(
        selected_symbol=rep.symbol,
        event=event,
        new_engine=new_engine,
    )
