#!/usr/bin/env python3
"""G1 仓位管理模块——缠论体系风控的工程化实现。

基于操作方法论 v1 (267号) + v2 修正 (338号) + 风控定义 (fengkong.md)。

核心设计：
- 风控不是独立模块，是买卖点系统的内在属性（第32课）
- 退出条件 = 买入程序的判断条件被否定（第13课）
- 仓位管理 = 走势阶段驱动的三阶段推进（第31课）
- 短差程序 = 次级别买卖点降成本（第14课）

认识论等级：L0（从267号/338号/fengkong.md定义直接推导）

谱系引用：
- 267号：操作方法论 v1（满仓满融降成本体系）
- 338号：操作方法论 v2（268a四项否定修正）
- fengkong.md：缠论风控体系定义
- 220号：缠论交易策略体系
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, replace
from enum import Enum
from typing import Literal


# ── 阶段枚举 ──


class Phase(str, Enum):
    """仓位管理阶段（第31课三阶段 + 空仓）。

    状态流转：IDLE → COST_POSITIVE → COST_ZERO → FREE_POSITION → IDLE
    """

    IDLE = "idle"
    COST_POSITIVE = "cost_positive"       # 阶段1：成本>0，降成本
    COST_ZERO = "cost_zero"               # 阶段2过渡：成本归零
    FREE_POSITION = "free_position"       # 阶段3：成本=0，挣股票


# ── 信号定义 ──


@dataclass(frozen=True, slots=True)
class Signal:
    """买卖点信号——仓位管理的唯一输入。

    风控 = 买卖点系统本身（第32课）。
    仓位管理只响应买卖点信号，不响应价格阈值或效率度量。

    Attributes
    ----------
    kind : 买卖点类型（type1/type2/type3）
    side : 买/卖方向
    level : 信号所在级别（0=操作级别，1=次级别，2=次次级别...）
    price : 信号价格
    timestamp : 信号时间戳
    confirmed : 是否已确认
    invalidated : 买入程序的判断条件是否被否定（退出条件，第13课）
    """

    kind: Literal["type1", "type2", "type3"]
    side: Literal["buy", "sell"]
    level: int
    price: float
    timestamp: float
    confirmed: bool = False
    invalidated: bool = False


# ── 短差记录 ──


@dataclass(frozen=True, slots=True)
class ShortDiffRecord:
    """单次短差操作记录。

    短差程序（第14课）：次级别卖点减仓，次级别买点回补。
    目的是降低成本，不是独立盈利。

    Attributes
    ----------
    sell_price : 减仓价格
    sell_ts : 减仓时间
    buy_price : 回补价格（未回补时为 None）
    buy_ts : 回补时间
    level : 短差操作级别
    profit : 单次短差利润（sell_price - buy_price，未回补时为 0）
    shares : 短差动用份额
    """

    sell_price: float
    sell_ts: float
    buy_price: float | None = None
    buy_ts: float | None = None
    level: int = 1
    profit: float = 0.0
    shares: float = 0.0


# ── 持仓状态（不可变） ──


@dataclass(frozen=True, slots=True)
class Position:
    """持仓状态——纯数据，不可变。

    每次状态变化创建新的 Position 实例。

    Attributes
    ----------
    phase : 当前阶段
    entry_kind : 建仓时的买点类型
    entry_price : 建仓价格
    entry_ts : 建仓时间
    shares : 持仓份额（含融资）
    own_capital : 本次循环的自有资金（338号修正1：动态重置）
    cost : 当前持仓成本（总成本 / 总份额）
    accumulated_recovery : 累计回收金额（短差利润累计）
    short_diffs : 短差记录列表
    pending_sell_price : 短差卖出后等待回补的价格（None=无待回补）
    pending_sell_level : 待回补短差的级别
    """

    phase: Phase = Phase.IDLE
    entry_kind: Literal["type1", "type2", "type3"] | None = None
    entry_price: float = 0.0
    entry_ts: float = 0.0
    shares: float = 0.0
    own_capital: float = 0.0
    cost: float = 0.0
    accumulated_recovery: float = 0.0
    short_diffs: tuple[ShortDiffRecord, ...] = ()
    pending_sell_price: float | None = None
    pending_sell_level: int = 0


# ── 操作指令（不可变） ──


@dataclass(frozen=True, slots=True)
class Action:
    """仓位管理引擎的输出——操作指令。

    Attributes
    ----------
    action_type : 指令类型
    shares : 操作份额
    price : 操作价格
    reason : 操作理由（缠论依据）
    """

    action_type: Literal[
        "enter",           # 建仓
        "exit_structure",  # 结构退出（买入程序否定）
        "short_sell",      # 短差减仓
        "short_buy",       # 短差回补
        "skip_buyback",    # 338号修正4：强趋势浅回调不加回
        "extract_capital", # 退出本金
        "final_exit",      # 主级别卖点清仓
        "hold",            # 持有不动
    ]
    shares: float = 0.0
    price: float = 0.0
    reason: str = ""


# ── 配置（不入语法的外部参数） ──


@dataclass(frozen=True, slots=True)
class PositionConfig:
    """不入语法的外部参数（267号/338号）。

    这些参数不入缠论语法，但必须显式声明。

    Attributes
    ----------
    own_capital : 自有资金
    leverage_ratio : 杠杆倍数（满仓满融=2.0，纯自有=1.0）
    short_diff_ratio : 每层短差动用比例（<= 上层的 n%）
    min_operable_level : 最小可操作短差级别（338号修正2）
    friction_cost : 单次操作摩擦成本比例（佣金+印花税+滑点）
    """

    own_capital: float = 100000.0
    leverage_ratio: float = 2.0
    short_diff_ratio: float = 0.3
    min_operable_level: int = 2
    friction_cost: float = 0.002


# ── 退出条件判断（fengkong.md 概念1） ──


def check_exit_condition(
    position: Position,
    signal: Signal,
) -> bool:
    """检查退出条件——买入程序的判断条件是否被否定。

    退出与盈亏无关，只与走势结构有关（第13课）。
    这不是"止损"，是买入逻辑的否定。

    第一类买点退出：上涨中再次形成同级别中枢（第13课）
    第二类买点退出：跌破前一个下跌趋势最低位（第13课）
    第三类买点退出：回试跌破 ZG（逻辑推论）

    Parameters
    ----------
    position : 当前持仓状态
    signal : 当前信号（含 invalidated 标记）

    Returns
    -------
    bool : True = 买入程序被否定，应退出
    """
    if position.phase == Phase.IDLE:
        return False
    if signal.level != 0:
        return False
    return signal.invalidated


# ── 短差可行性检查（338号修正2） ──


def short_diff_feasible(
    config: PositionConfig,
    signal_level: int,
    expected_spread: float,
) -> bool:
    """检查短差是否可行——摩擦成本截止条件。

    338号修正2：当某级别的期望短差价差不足以覆盖该层操作的总摩擦成本时，
    递归在该级别截止。

    Parameters
    ----------
    config : 配置参数
    signal_level : 短差操作级别
    expected_spread : 期望短差价差比例

    Returns
    -------
    bool : True = 短差可行
    """
    if signal_level > config.min_operable_level:
        return False
    return expected_spread > config.friction_cost * 2


# ── 核心状态转换函数 ──


def process_signal(
    position: Position,
    signal: Signal,
    config: PositionConfig,
) -> tuple[Position, Action]:
    """处理一个信号，返回新的持仓状态和操作指令。

    纯函数：不修改输入，返回新的不可变对象。

    状态机逻辑：
    1. IDLE + 操作级别买点 → 建仓 → COST_POSITIVE
    2. COST_POSITIVE + 退出条件 → 结构退出 → IDLE
    3. COST_POSITIVE + 次级别卖点 → 短差减仓
    4. COST_POSITIVE + 次级别买点 → 短差回补（或338号修正4不加回）
    5. COST_POSITIVE + 累计回收 >= 自有资金 → COST_ZERO
    6. COST_ZERO + 大级别卖点 → 退出本金 → FREE_POSITION
    7. FREE_POSITION + 主级别卖点 → 清仓 → IDLE

    Parameters
    ----------
    position : 当前持仓状态
    signal : 输入信号
    config : 配置参数

    Returns
    -------
    (new_position, action) : 新状态 + 操作指令
    """
    # ── IDLE：等待建仓信号 ──
    if position.phase == Phase.IDLE:
        return _handle_idle(position, signal, config)

    # ── 任何持仓阶段：检查结构退出 ──
    if check_exit_condition(position, signal):
        return _handle_exit(position, signal)

    # ── COST_POSITIVE：降成本阶段 ──
    if position.phase == Phase.COST_POSITIVE:
        return _handle_cost_positive(position, signal, config)

    # ── COST_ZERO：成本归零过渡 ──
    if position.phase == Phase.COST_ZERO:
        return _handle_cost_zero(position, signal)

    # ── FREE_POSITION：免费仓位管理 ──
    if position.phase == Phase.FREE_POSITION:
        return _handle_free_position(position, signal)

    return position, Action(action_type="hold", reason="未匹配任何规则")


def _handle_idle(
    position: Position,
    signal: Signal,
    config: PositionConfig,
) -> tuple[Position, Action]:
    """IDLE → 建仓。

    267号§二阶段1：买点出现，一次性满仓满融进入。不分批。
    """
    if signal.side != "buy" or signal.level != 0 or not signal.confirmed:
        return position, Action(action_type="hold", reason="无操作级别确认买点")

    total_capital = config.own_capital * config.leverage_ratio
    shares = total_capital / signal.price

    new_position = Position(
        phase=Phase.COST_POSITIVE,
        entry_kind=signal.kind,
        entry_price=signal.price,
        entry_ts=signal.timestamp,
        shares=shares,
        own_capital=config.own_capital,
        cost=signal.price,
        accumulated_recovery=0.0,
    )

    return new_position, Action(
        action_type="enter",
        shares=shares,
        price=signal.price,
        reason=f"操作级别{signal.kind}买点确认，一次性满仓满融建仓",
    )


def _handle_exit(
    position: Position,
    signal: Signal,
) -> tuple[Position, Action]:
    """结构退出——买入程序的判断条件被否定。

    满融下止损必须绝对果断——没有等待空间（267号§四）。
    """
    return Position(), Action(
        action_type="exit_structure",
        shares=position.shares,
        price=signal.price,
        reason=(
            f"{position.entry_kind}买入程序的判断条件被否定，"
            "结构退出（第13课）"
        ),
    )


def _handle_cost_positive(
    position: Position,
    signal: Signal,
    config: PositionConfig,
) -> tuple[Position, Action]:
    """COST_POSITIVE 阶段——降成本。

    第31课：成本为0前，每次短差不增加持仓总量。
    338号修正4：次级别买点价格 >= 前次卖出价格时，不执行加回。
    """
    # 检查阶段推进：累计回收 >= 自有资金 → COST_ZERO
    if position.accumulated_recovery >= position.own_capital:
        new_pos = replace(position, phase=Phase.COST_ZERO)
        return new_pos, Action(
            action_type="hold",
            reason="累计回收 >= 自有资金，进入成本归零阶段",
        )

    # 次级别卖点 → 短差减仓
    if signal.side == "sell" and signal.level >= 1 and signal.confirmed:
        return _short_diff_sell(position, signal, config)

    # 次级别买点 → 短差回补（或不加回）
    if signal.side == "buy" and signal.level >= 1 and signal.confirmed:
        return _short_diff_buy(position, signal, config)

    return position, Action(action_type="hold", reason="等待次级别买卖点信号")


def _short_diff_sell(
    position: Position,
    signal: Signal,
    config: PositionConfig,
) -> tuple[Position, Action]:
    """短差减仓——次级别卖点减仓。

    第14课：大级别买点介入的，在次级别第一类卖点出现时，可以先减仓。
    """
    if position.pending_sell_price is not None:
        return position, Action(
            action_type="hold",
            reason="已有未回补的短差卖出，等待回补或截止",
        )

    diff_shares = position.shares * config.short_diff_ratio

    new_pos = replace(
        position,
        pending_sell_price=signal.price,
        pending_sell_level=signal.level,
    )

    return new_pos, Action(
        action_type="short_sell",
        shares=diff_shares,
        price=signal.price,
        reason=f"次级别({signal.level})卖点，短差减仓{config.short_diff_ratio:.0%}",
    )


def _short_diff_buy(
    position: Position,
    signal: Signal,
    config: PositionConfig,
) -> tuple[Position, Action]:
    """短差回补——次级别买点回补。

    第14课：在次级别第一类买点出现时回补。
    338号修正4：次级别买点价格 >= 前次卖出价格时，不执行加回。
    第31课阶段一约束：每次短差，买入数量=卖出数量，不增加持仓总量。
    """
    if position.pending_sell_price is None:
        return position, Action(
            action_type="hold",
            reason="无待回补的短差卖出",
        )

    # 338号修正4：强趋势浅回调不加回
    if signal.price >= position.pending_sell_price:
        new_pos = replace(
            position,
            pending_sell_price=None,
            pending_sell_level=0,
        )
        return new_pos, Action(
            action_type="skip_buyback",
            price=signal.price,
            reason=(
                f"买点价格({signal.price:.2f}) >= "
                f"前次卖出价格({position.pending_sell_price:.2f})，"
                "不执行加回（338号修正4：强趋势浅回调）"
            ),
        )

    # 正常回补：利润 = 卖价 - 买价
    diff_shares = position.shares * config.short_diff_ratio
    profit = (position.pending_sell_price - signal.price) * diff_shares
    new_recovery = position.accumulated_recovery + profit

    record = ShortDiffRecord(
        sell_price=position.pending_sell_price,
        sell_ts=0.0,
        buy_price=signal.price,
        buy_ts=signal.timestamp,
        level=position.pending_sell_level,
        profit=profit,
        shares=diff_shares,
    )

    # 更新成本
    total_value = position.cost * position.shares
    new_cost = (total_value - profit) / position.shares

    # 检查阶段推进
    new_phase = position.phase
    if new_recovery >= position.own_capital:
        new_phase = Phase.COST_ZERO

    new_pos = replace(
        position,
        phase=new_phase,
        cost=max(new_cost, 0.0),
        accumulated_recovery=new_recovery,
        short_diffs=position.short_diffs + (record,),
        pending_sell_price=None,
        pending_sell_level=0,
    )

    return new_pos, Action(
        action_type="short_buy",
        shares=diff_shares,
        price=signal.price,
        reason=(
            f"次级别({signal.level})买点回补，"
            f"短差利润={profit:.2f}，累计回收={new_recovery:.2f}"
        ),
    )


def _handle_cost_zero(
    position: Position,
    signal: Signal,
) -> tuple[Position, Action]:
    """COST_ZERO → FREE_POSITION。

    第31课：在股票达到1倍升幅附近找一个大级别卖点出掉部分，
    把成本降为0。
    """
    if signal.side == "sell" and signal.level == 0 and signal.confirmed:
        # 出掉半仓使剩余成本=0
        exit_shares = position.shares / 2
        remaining = position.shares - exit_shares

        new_pos = replace(
            position,
            phase=Phase.FREE_POSITION,
            shares=remaining,
            cost=0.0,
        )

        return new_pos, Action(
            action_type="extract_capital",
            shares=exit_shares,
            price=signal.price,
            reason="操作级别卖点，退出本金，剩余仓位成本归零（第31课）",
        )

    return position, Action(
        action_type="hold",
        reason="等待操作级别卖点退出本金",
    )


def _handle_free_position(
    position: Position,
    signal: Signal,
) -> tuple[Position, Action]:
    """FREE_POSITION——免费仓位管理。

    267号§六：当时结构是什么就怎么处理。
    主级别卖点出现就平仓。
    第31课阶段三：成本=0后，抛出后全部回补，仓位持续增加。
    """
    if signal.side == "sell" and signal.level == 0 and signal.confirmed:
        return Position(), Action(
            action_type="final_exit",
            shares=position.shares,
            price=signal.price,
            reason="主级别卖点，免费仓位清仓（267号§六）",
        )

    # 阶段三短差：卖出后全额回补，仓位增加
    if signal.side == "sell" and signal.level >= 1 and signal.confirmed:
        new_pos = replace(
            position,
            pending_sell_price=signal.price,
            pending_sell_level=signal.level,
        )
        return new_pos, Action(
            action_type="short_sell",
            shares=position.shares * 0.3,
            price=signal.price,
            reason="免费仓位次级别卖点，短差减仓（阶段三：卖出后全额回补）",
        )

    if signal.side == "buy" and signal.level >= 1 and signal.confirmed:
        if position.pending_sell_price is not None:
            # 阶段三：全额回补 = 用卖出的钱全部买入
            sell_value = position.pending_sell_price * position.shares * 0.3
            buy_shares = sell_value / signal.price
            new_shares = position.shares + buy_shares - (position.shares * 0.3)

            new_pos = replace(
                position,
                shares=new_shares,
                pending_sell_price=None,
                pending_sell_level=0,
            )

            return new_pos, Action(
                action_type="short_buy",
                shares=buy_shares,
                price=signal.price,
                reason=(
                    f"免费仓位次级别买点全额回补，"
                    f"仓位从{position.shares:.2f}增至{new_shares:.2f}（第31课阶段三）"
                ),
            )

    return position, Action(action_type="hold", reason="免费仓位等待信号")


# ── 批量处理 ──


def run_signals(
    signals: list[Signal],
    config: PositionConfig,
) -> list[tuple[Position, Action]]:
    """批量处理信号序列，返回每步的 (状态, 操作) 对。

    纯函数：无副作用。

    Parameters
    ----------
    signals : 信号序列
    config : 配置参数

    Returns
    -------
    list of (Position, Action) : 每步的状态和操作
    """
    results: list[tuple[Position, Action]] = []
    position = Position()

    for signal in signals:
        position, action = process_signal(position, signal, config)
        results.append((position, action))

    return results


# ── 序列化 ──


def position_to_dict(position: Position) -> dict:
    """Position → dict（可 JSON 序列化）。"""
    return {
        "phase": position.phase.value,
        "entry_kind": position.entry_kind,
        "entry_price": position.entry_price,
        "entry_ts": position.entry_ts,
        "shares": position.shares,
        "own_capital": position.own_capital,
        "cost": position.cost,
        "accumulated_recovery": position.accumulated_recovery,
        "short_diff_count": len(position.short_diffs),
        "pending_sell_price": position.pending_sell_price,
    }


def action_to_dict(action: Action) -> dict:
    """Action → dict（可 JSON 序列化）。"""
    return {
        "action_type": action.action_type,
        "shares": action.shares,
        "price": action.price,
        "reason": action.reason,
    }


# ── CLI 入口 ──


def _demo_scenario() -> list[Signal]:
    """构造一个演示信号序列。

    场景：操作级别type1买点建仓 → 次级别短差降成本 → 退出本金 → 清仓。
    """
    return [
        # 1. 操作级别 type1 买点 → 建仓
        Signal(kind="type1", side="buy", level=0, price=10.0,
               timestamp=1000.0, confirmed=True),
        # 2. 次级别卖点 → 短差减仓
        Signal(kind="type1", side="sell", level=1, price=11.0,
               timestamp=2000.0, confirmed=True),
        # 3. 次级别买点 → 短差回补（价格低于卖出价）
        Signal(kind="type1", side="buy", level=1, price=10.2,
               timestamp=3000.0, confirmed=True),
        # 4. 次级别卖点 → 再次短差
        Signal(kind="type1", side="sell", level=1, price=12.0,
               timestamp=4000.0, confirmed=True),
        # 5. 次级别买点 → 价格高于卖出价（338号修正4：不加回）
        Signal(kind="type1", side="buy", level=1, price=12.5,
               timestamp=5000.0, confirmed=True),
        # 6. 等待信号（无操作）
        Signal(kind="type2", side="sell", level=2, price=13.0,
               timestamp=6000.0, confirmed=False),
        # 7. 结构退出信号（买入程序被否定）
        Signal(kind="type1", side="sell", level=0, price=9.0,
               timestamp=7000.0, confirmed=True, invalidated=True),
    ]


def main() -> None:
    parser = argparse.ArgumentParser(
        description="缠论仓位管理引擎（G1）",
    )
    parser.add_argument(
        "--signals", type=str, default=None,
        help="信号文件路径（JSON），省略则运行演示场景",
    )
    parser.add_argument(
        "--capital", type=float, default=100000.0,
        help="自有资金（默认 100000）",
    )
    parser.add_argument(
        "--leverage", type=float, default=2.0,
        help="杠杆倍数（默认 2.0，满仓满融）",
    )
    parser.add_argument(
        "--ratio", type=float, default=0.3,
        help="短差动用比例（默认 0.3）",
    )
    parser.add_argument(
        "--output", type=str, default=None,
        help="输出文件路径（JSON），省略则输出到 stdout",
    )
    args = parser.parse_args()

    config = PositionConfig(
        own_capital=args.capital,
        leverage_ratio=args.leverage,
        short_diff_ratio=args.ratio,
    )

    if args.signals:
        with open(args.signals, encoding="utf-8") as f:
            raw = json.load(f)
        signals = [
            Signal(
                kind=s["kind"],
                side=s["side"],
                level=s["level"],
                price=s["price"],
                timestamp=s.get("timestamp", 0.0),
                confirmed=s.get("confirmed", False),
                invalidated=s.get("invalidated", False),
            )
            for s in raw
        ]
    else:
        signals = _demo_scenario()

    results = run_signals(signals, config)

    output_data = []
    for i, (pos, act) in enumerate(results):
        output_data.append({
            "step": i,
            "signal": {
                "kind": signals[i].kind,
                "side": signals[i].side,
                "level": signals[i].level,
                "price": signals[i].price,
                "confirmed": signals[i].confirmed,
                "invalidated": signals[i].invalidated,
            },
            "action": action_to_dict(act),
            "position": position_to_dict(pos),
        })

    output_json = json.dumps(output_data, ensure_ascii=False, indent=2)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(output_json)
        print(f"结果已写入 {args.output}", file=sys.stderr)
    else:
        print(output_json)


if __name__ == "__main__":
    main()
