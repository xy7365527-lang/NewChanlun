"""小转大编排集成 — 跨级别联动信号提取与 FSM 联动。

orchestrator 层模块：从走势结构数据中提取跨级别小转大信号，
过滤无效信号，转换为 CostReductionFSM 事件。

不修改引擎层代码（BiEngine/SegmentEngine/ZhongshuEngine/MoveEngine/BuySellPointEngine）。

原文依据：
- 第43课：小级别背驰引发大级别转折
- 第53课：小转大时买卖点选择
- 267号§三：旧级别短差停止，新级别继续
- 338号修正4：强趋势浅回调 → 小转大前兆

概念溯源: [旧缠论] 第43课 + [267号操作方法论] + [338号修正]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_xiaozhuan_da import XiaozhuanDa, detect_xiaozhuan_da
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.trading.cost_reduction_fsm import FsmEvent, FsmEventType


@dataclass(frozen=True, slots=True)
class CrossLevelSignal:
    """跨级别小转大信号。

    Attributes
    ----------
    xiaozhuan_da : XiaozhuanDa
        底层小转大检测结果。
    source_level : int
        本级别（被"转大"的级别）。
    trigger_level : int
        触发级别（次级别）= source_level - 1。
    """

    xiaozhuan_da: XiaozhuanDa
    source_level: int
    trigger_level: int


def extract_cross_level_signals(
    *,
    segments: list,
    zhongshus: list[Zhongshu],
    moves: list[Move],
    level_divergences: list[Divergence],
    sub_divergences: list[Divergence],
    level_id: int,
) -> list[CrossLevelSignal]:
    """从走势结构数据提取跨级别小转大信号。

    封装 detect_xiaozhuan_da，将其输出包装为 CrossLevelSignal。

    Parameters
    ----------
    segments : list
        线段列表。
    zhongshus : list[Zhongshu]
        中枢列表。
    moves : list[Move]
        走势类型列表。
    level_divergences : list[Divergence]
        本级别背驰列表。
    sub_divergences : list[Divergence]
        次级别背驰列表。
    level_id : int
        本级别 ID。

    Returns
    -------
    list[CrossLevelSignal]
        提取到的跨级别信号。
    """
    xzd_results = detect_xiaozhuan_da(
        segments=segments,
        zhongshus=zhongshus,
        moves=moves,
        level_divergences=level_divergences,
        sub_divergences=sub_divergences,
        level_id=level_id,
    )

    return [
        CrossLevelSignal(
            xiaozhuan_da=xzd,
            source_level=level_id,
            trigger_level=level_id - 1,
        )
        for xzd in xzd_results
    ]


def filter_actionable_signals(
    signals: list[CrossLevelSignal],
) -> list[CrossLevelSignal]:
    """过滤可操作的小转大信号。

    过滤条件：
    - 次级别背驰必须 confirmed（走势类型已完成）

    Parameters
    ----------
    signals : list[CrossLevelSignal]
        待过滤的信号列表。

    Returns
    -------
    list[CrossLevelSignal]
        可操作的信号。
    """
    return [
        sig for sig in signals
        if sig.xiaozhuan_da.sub_divergence.confirmed
    ]


def signal_to_fsm_events(
    signal: CrossLevelSignal,
    current_price: float,
) -> list[FsmEvent]:
    """将小转大信号转换为 CostReductionFSM 事件。

    267号§三：小转大 → LEVEL_UPGRADE → 赋格分裂。
    新级别标识 = "L{source_level + 1}"。

    Parameters
    ----------
    signal : CrossLevelSignal
        小转大信号。
    current_price : float
        当前价格。

    Returns
    -------
    list[FsmEvent]
        FSM 事件列表（当前固定为单个 LEVEL_UPGRADE）。
    """
    new_level = f"L{signal.source_level + 1}"

    return [
        FsmEvent(
            event_type=FsmEventType.LEVEL_UPGRADE,
            price=current_price,
            level=new_level,
        ),
    ]


def detect_shallow_pullback_precursor(
    *,
    sell_price: float,
    buy_price: float,
) -> bool:
    """检测浅回调前兆（338号修正4）。

    "次级别卖出后，若次级别买点价格 >= 前次卖出价格，不执行加回。"

    Parameters
    ----------
    sell_price : float
        前次卖出价格。
    buy_price : float
        当前买回价格。

    Returns
    -------
    bool
        True = 浅回调前兆（不应加回），False = 正常回调。
    """
    return buy_price >= sell_price
