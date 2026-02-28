"""降成本状态机 — 267号操作方法论 v1 的代码翻译。

认识论标注：
  - L0：状态定义、转移规则、成本递推公式（从267号定义直接推导）
  - L2（待验证）：sub_ratio 默认值、融资利率等参数

谱系引用：267号操作方法论 v1——满仓满融降成本体系。
268a号结算修正：
  - 攻击1（接受）："初始自有资金"每次新循环独立核算，RESET 事件携带新 own_capital
  - 攻击2（部分接受）：min_operable_level 作为不入语法的外部参数标注

状态：SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN → STOPPED_OUT
      任何持仓状态 → STOPPED_OUT（买点失效 = 止损）
      STOPPED_OUT → SCANNING（重置，own_capital 按实际资金重新核算）
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Sequence


# ═══════════════════════════════════════════════════════════════
# 枚举
# ═══════════════════════════════════════════════════════════════


class CostState(Enum):
    """降成本状态机的五种状态。"""

    SCANNING = auto()            # 选股扫描
    POSITION_OPEN = auto()       # 满仓满融建仓完成
    COST_REDUCING = auto()       # 次级别短差循环运行中
    PRINCIPAL_WITHDRAWN = auto() # 本金已退出，免费仓位
    STOPPED_OUT = auto()         # 止损退出


class FsmEventType(Enum):
    """状态机事件类型。

    不存在 SWAP / SWITCH_POSITION / CHANGE_STOCK（不换仓原则，267号§五）。
    """

    BUY_POINT_CONFIRMED = auto()   # 买点确认 → 建仓
    SUB_LEVEL_SELL_POINT = auto()  # 次级别卖点 → 短差卖出
    SUB_LEVEL_BUY_POINT = auto()   # 次级别买点 → 短差买回
    BUY_POINT_NEGATED = auto()     # 买点失效 → 止损
    MAIN_LEVEL_SELL_POINT = auto() # 主级别卖点 → 退出
    LEVEL_UPGRADE = auto()         # 小转大 → 赋格分裂
    RESET = auto()                 # 止损后重置 → 回到扫描


# ═══════════════════════════════════════════════════════════════
# 数据结构（全部 frozen）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class FsmEvent:
    """状态机输入事件。"""

    event_type: FsmEventType
    price: float
    level: str
    new_own_capital: float | None = None  # 268a攻击1：RESET时携带止损后的实际自有资金


@dataclass(frozen=True, slots=True)
class ShortDiffCycle:
    """短差循环记录。

    每次次级别卖点开启短差（卖出部分），买点买回。
    profit = (sell_price - buy_price) * shares。
    """

    level: str
    shares: float
    sell_price: float
    buy_price: float = 0.0
    is_open: bool = True  # True=已卖出等待买回, False=已完成

    @property
    def profit(self) -> float:
        if self.is_open:
            return 0.0
        return (self.sell_price - self.buy_price) * self.shares


@dataclass(frozen=True, slots=True)
class FugueVoice:
    """赋格声部。

    小转大后产生：旧循环利润底仓（is_profit_floor=True）+
    新级别新降成本循环（is_profit_floor=False）。
    """

    level: str
    shares: float
    is_profit_floor: bool
    cumulative_profit: float = 0.0


@dataclass(frozen=True, slots=True)
class FsmSnapshot:
    """状态快照（用于序列化/审计）。"""

    state: CostState
    own_capital: float
    margin_amount: float
    total_shares: float
    cost_basis: float
    cumulative_recovered: float
    entry_price: float
    entry_level: str
    fugue_voice_count: int


@dataclass(frozen=True, slots=True)
class CostReductionFSM:
    """降成本状态机（不可变）。

    每次 transition 返回新实例。

    Attributes
    ----------
    state : CostState
        当前状态。
    own_capital : float
        初始自有资金（268a攻击1修正：每次新循环独立核算，等于当时实际自有资金）。
    margin_amount : float
        融资额度。
    total_shares : float
        总持仓份额。
    cost_basis : float
        当前每股成本。
    cumulative_recovered : float
        累计回收金额。
    entry_price : float
        建仓价格。
    entry_level : str
        操作主级别。
    sub_ratio : float
        每层短差动用比例上限（外部参数，267号§不入语法）。
    min_operable_level : str
        最小可操作短差级别（外部参数，268a攻击2：市场交易制度约束，如A股T+1）。
    active_short_diff : ShortDiffCycle | None
        当前活跃的短差循环。
    completed_short_diffs : tuple[ShortDiffCycle, ...]
        已完成的短差循环历史。
    fugue_voices : tuple[FugueVoice, ...]
        赋格声部（小转大后 >= 2）。
    """

    state: CostState
    own_capital: float
    margin_amount: float
    total_shares: float
    cost_basis: float
    cumulative_recovered: float
    entry_price: float
    entry_level: str
    sub_ratio: float
    min_operable_level: str
    active_short_diff: ShortDiffCycle | None
    completed_short_diffs: tuple[ShortDiffCycle, ...]
    fugue_voices: tuple[FugueVoice, ...]

    @staticmethod
    def create(
        *,
        own_capital: float,
        margin_amount: float = 0.0,
        sub_ratio: float = 0.3,
        min_operable_level: str = "",
    ) -> CostReductionFSM:
        """创建初始状态机（SCANNING）。"""
        return CostReductionFSM(
            state=CostState.SCANNING,
            own_capital=own_capital,
            margin_amount=margin_amount,
            total_shares=0.0,
            cost_basis=0.0,
            cumulative_recovered=0.0,
            entry_price=0.0,
            entry_level="",
            sub_ratio=sub_ratio,
            min_operable_level=min_operable_level,
            active_short_diff=None,
            completed_short_diffs=(),
            fugue_voices=(),
        )

    def snapshot(self) -> FsmSnapshot:
        """生成快照。"""
        return FsmSnapshot(
            state=self.state,
            own_capital=self.own_capital,
            margin_amount=self.margin_amount,
            total_shares=self.total_shares,
            cost_basis=self.cost_basis,
            cumulative_recovered=self.cumulative_recovered,
            entry_price=self.entry_price,
            entry_level=self.entry_level,
            fugue_voice_count=len(self.fugue_voices),
        )


# ═══════════════════════════════════════════════════════════════
# 转移函数（纯函数）
# ═══════════════════════════════════════════════════════════════


def _replace(fsm: CostReductionFSM, **kwargs: object) -> CostReductionFSM:
    """返回新 FSM 实例，仅替换指定字段。"""
    return CostReductionFSM(
        state=kwargs.get("state", fsm.state),  # type: ignore[arg-type]
        own_capital=kwargs.get("own_capital", fsm.own_capital),  # type: ignore[arg-type]
        margin_amount=kwargs.get("margin_amount", fsm.margin_amount),  # type: ignore[arg-type]
        total_shares=kwargs.get("total_shares", fsm.total_shares),  # type: ignore[arg-type]
        cost_basis=kwargs.get("cost_basis", fsm.cost_basis),  # type: ignore[arg-type]
        cumulative_recovered=kwargs.get("cumulative_recovered", fsm.cumulative_recovered),  # type: ignore[arg-type]
        entry_price=kwargs.get("entry_price", fsm.entry_price),  # type: ignore[arg-type]
        entry_level=kwargs.get("entry_level", fsm.entry_level),  # type: ignore[arg-type]
        sub_ratio=kwargs.get("sub_ratio", fsm.sub_ratio),  # type: ignore[arg-type]
        min_operable_level=kwargs.get("min_operable_level", fsm.min_operable_level),  # type: ignore[arg-type]
        active_short_diff=kwargs.get("active_short_diff", fsm.active_short_diff),  # type: ignore[arg-type]
        completed_short_diffs=kwargs.get("completed_short_diffs", fsm.completed_short_diffs),  # type: ignore[arg-type]
        fugue_voices=kwargs.get("fugue_voices", fsm.fugue_voices),  # type: ignore[arg-type]
    )


class IllegalTransitionError(Exception):
    """非法状态转移。"""


def transition(fsm: CostReductionFSM, event: FsmEvent) -> CostReductionFSM:
    """状态转移主入口。输入旧状态+事件→输出新状态。"""
    handlers = {
        CostState.SCANNING: _handle_scanning,
        CostState.POSITION_OPEN: _handle_position_open,
        CostState.COST_REDUCING: _handle_cost_reducing,
        CostState.PRINCIPAL_WITHDRAWN: _handle_principal_withdrawn,
        CostState.STOPPED_OUT: _handle_stopped_out,
    }
    handler = handlers.get(fsm.state)
    if handler is None:
        raise IllegalTransitionError(f"未知状态: {fsm.state}")
    return handler(fsm, event)


# ── SCANNING ──────────────────────────────────────────────────


def _handle_scanning(
    fsm: CostReductionFSM, event: FsmEvent,
) -> CostReductionFSM:
    if event.event_type != FsmEventType.BUY_POINT_CONFIRMED:
        raise IllegalTransitionError(
            f"SCANNING 状态仅接受 BUY_POINT_CONFIRMED，收到 {event.event_type.name}"
        )
    total_capital = fsm.own_capital + fsm.margin_amount
    shares = total_capital / event.price
    return _replace(
        fsm,
        state=CostState.POSITION_OPEN,
        total_shares=shares,
        cost_basis=event.price,
        entry_price=event.price,
        entry_level=event.level,
        cumulative_recovered=0.0,
        active_short_diff=None,
        completed_short_diffs=(),
        fugue_voices=(),
    )


# ── POSITION_OPEN ────────────────────────────────────────────


def _handle_position_open(
    fsm: CostReductionFSM, event: FsmEvent,
) -> CostReductionFSM:
    if event.event_type == FsmEventType.BUY_POINT_NEGATED:
        return _stop_out(fsm, event.price)

    if event.event_type == FsmEventType.SUB_LEVEL_SELL_POINT:
        short_shares = fsm.total_shares * fsm.sub_ratio
        cycle = ShortDiffCycle(
            level=event.level,
            shares=short_shares,
            sell_price=event.price,
        )
        return _replace(
            fsm,
            state=CostState.COST_REDUCING,
            active_short_diff=cycle,
        )

    raise IllegalTransitionError(
        f"POSITION_OPEN 不接受 {event.event_type.name}"
    )


# ── COST_REDUCING ────────────────────────────────────────────


def _handle_cost_reducing(
    fsm: CostReductionFSM, event: FsmEvent,
) -> CostReductionFSM:
    if event.event_type == FsmEventType.BUY_POINT_NEGATED:
        return _stop_out(fsm, event.price)

    if event.event_type == FsmEventType.SUB_LEVEL_BUY_POINT:
        return _close_short_diff(fsm, event.price, event.level)

    if event.event_type == FsmEventType.SUB_LEVEL_SELL_POINT:
        return _open_short_diff(fsm, event.price, event.level)

    if event.event_type == FsmEventType.LEVEL_UPGRADE:
        return _level_upgrade(fsm, event.price, event.level)

    if event.event_type == FsmEventType.MAIN_LEVEL_SELL_POINT:
        return _stop_out(fsm, event.price)

    raise IllegalTransitionError(
        f"COST_REDUCING 不接受 {event.event_type.name}"
    )


def _close_short_diff(
    fsm: CostReductionFSM, buy_price: float, level: str,
) -> CostReductionFSM:
    """次级别买点：买回，完成短差循环，更新成本。"""
    cycle = fsm.active_short_diff
    if cycle is None or not cycle.is_open:
        # 无活跃短差时，忽略买点（已满仓）
        return fsm

    closed = ShortDiffCycle(
        level=cycle.level,
        shares=cycle.shares,
        sell_price=cycle.sell_price,
        buy_price=buy_price,
        is_open=False,
    )
    profit = closed.profit

    # cost(t) = cost(t-1) - profit / total_shares
    new_cost = fsm.cost_basis - profit / fsm.total_shares
    new_recovered = fsm.cumulative_recovered + max(profit, 0.0)
    new_completed = fsm.completed_short_diffs + (closed,)

    # 检查是否达到本金退出条件
    if new_recovered >= fsm.own_capital:
        return _replace(
            fsm,
            state=CostState.PRINCIPAL_WITHDRAWN,
            cost_basis=new_cost,
            cumulative_recovered=new_recovered,
            active_short_diff=None,
            completed_short_diffs=new_completed,
        )

    return _replace(
        fsm,
        cost_basis=new_cost,
        cumulative_recovered=new_recovered,
        active_short_diff=None,
        completed_short_diffs=new_completed,
    )


def _open_short_diff(
    fsm: CostReductionFSM, sell_price: float, level: str,
) -> CostReductionFSM:
    """次级别卖点：开启新短差循环。"""
    if fsm.active_short_diff is not None and fsm.active_short_diff.is_open:
        raise IllegalTransitionError(
            "已有未完成的短差循环，不能开启新的"
        )
    short_shares = fsm.total_shares * fsm.sub_ratio
    cycle = ShortDiffCycle(
        level=level,
        shares=short_shares,
        sell_price=sell_price,
    )
    return _replace(fsm, active_short_diff=cycle)


def _level_upgrade(
    fsm: CostReductionFSM, price: float, new_level: str,
) -> CostReductionFSM:
    """小转大：旧循环利润底仓 + 新级别新降成本循环。

    267号§三：旧级别短差停止，在新主级别定义下的次级别出现后，
    在新级别上继续短差降成本。
    """
    # 旧声部：已有的利润底仓
    old_voice = FugueVoice(
        level=fsm.entry_level,
        shares=fsm.total_shares,
        is_profit_floor=True,
        cumulative_profit=fsm.cumulative_recovered,
    )
    # 新声部：新级别上的新循环
    new_voice = FugueVoice(
        level=new_level,
        shares=fsm.total_shares,
        is_profit_floor=False,
        cumulative_profit=0.0,
    )
    new_voices = fsm.fugue_voices + (old_voice, new_voice)

    return _replace(
        fsm,
        entry_level=new_level,
        active_short_diff=None,  # 旧短差停止
        fugue_voices=new_voices,
    )


# ── PRINCIPAL_WITHDRAWN ──────────────────────────────────────


def _handle_principal_withdrawn(
    fsm: CostReductionFSM, event: FsmEvent,
) -> CostReductionFSM:
    """免费仓位：二值分类，主级别卖点或买点失效 → 退出。"""
    if event.event_type == FsmEventType.MAIN_LEVEL_SELL_POINT:
        return _stop_out(fsm, event.price)

    if event.event_type == FsmEventType.BUY_POINT_NEGATED:
        return _stop_out(fsm, event.price)

    if event.event_type == FsmEventType.SUB_LEVEL_SELL_POINT:
        return _open_short_diff(
            _replace(fsm, state=CostState.COST_REDUCING),
            event.price, event.level,
        )

    raise IllegalTransitionError(
        f"PRINCIPAL_WITHDRAWN 不接受 {event.event_type.name}"
    )


# ── STOPPED_OUT ──────────────────────────────────────────────


def _handle_stopped_out(
    fsm: CostReductionFSM, event: FsmEvent,
) -> CostReductionFSM:
    if event.event_type != FsmEventType.RESET:
        raise IllegalTransitionError(
            f"STOPPED_OUT 仅接受 RESET，收到 {event.event_type.name}"
        )
    # 268a攻击1修正：每次新循环独立核算，own_capital 按实际资金重新设定
    new_capital = event.new_own_capital if event.new_own_capital is not None else fsm.own_capital
    return CostReductionFSM.create(
        own_capital=new_capital,
        margin_amount=fsm.margin_amount,
        sub_ratio=fsm.sub_ratio,
        min_operable_level=fsm.min_operable_level,
    )


# ── 止损（共用）─────────────────────────────────────────────


def _stop_out(fsm: CostReductionFSM, price: float) -> CostReductionFSM:
    """止损退出。买点失效 = 止损（267号§四）。"""
    return _replace(
        fsm,
        state=CostState.STOPPED_OUT,
        active_short_diff=None,
    )
