"""Fugue engine -- passacaglia layer-2 melody fugue state machine.

Embeds within the cost-reduction FSM (267 section 2) and extends it with
a voice dimension for multi-level nested short-diff operations (267 section 7).

Epistemology: L0 (state set, transition function, input alphabet derived
from 349 spec + 267 section 7 + 338 amendments).

Genealogy:
  349 -- fugue state machine formal definition
  267 -- operating methodology v1 (section 7: fugue definition)
  265 -- passacaglia model (layer 2 melody fugue)
  338 -- operating methodology v2 (four negation amendments)
  268a -- settlement amendments (own_capital dynamic reset)
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import Sequence


# =============================================================================
# Enums
# =============================================================================


class FugueState(Enum):
    """Five states from 349 spec section 2."""

    SCANNING = auto()
    POSITION_OPEN = auto()
    COST_REDUCING = auto()
    PRINCIPAL_WITHDRAWN = auto()
    STOPPED_OUT = auto()


class FugueEventType(Enum):
    """Input alphabet from 349 spec section 3.

    No SWAP / SWITCH_POSITION / CHANGE_STOCK (267 section 5).
    """

    BUY_POINT_CONFIRMED = auto()
    SUB_LEVEL_SELL_POINT = auto()
    SUB_LEVEL_BUY_POINT = auto()
    BUY_POINT_NEGATED = auto()
    MAIN_LEVEL_SELL_POINT = auto()
    LEVEL_UPGRADE = auto()
    RESET = auto()


# =============================================================================
# Data structures (all frozen)
# =============================================================================


@dataclass(frozen=True, slots=True)
class FugueEvent:
    """Input event for the fugue engine."""

    event_type: FugueEventType
    price: float
    level: str
    new_own_capital: float | None = None


@dataclass(frozen=True, slots=True)
class ShortDiff:
    """A single short-diff cycle record."""

    level: str
    shares: float
    sell_price: float
    buy_price: float = 0.0
    is_open: bool = True

    @property
    def profit(self) -> float:
        if self.is_open:
            return 0.0
        return (self.sell_price - self.buy_price) * self.shares


@dataclass(frozen=True, slots=True)
class FugueVoiceInfo:
    """A fugue voice -- one level's cost-reduction cycle.

    On LEVEL_UPGRADE the old voice freezes as profit floor,
    a new voice starts at the upgraded level.
    """

    level: str
    shares: float
    is_profit_floor: bool
    cumulative_profit: float = 0.0


@dataclass(frozen=True, slots=True)
class FugueSnapshot:
    """Serialisable snapshot for audit."""

    state: FugueState
    own_capital: float
    margin_amount: float
    total_shares: float
    cost_basis: float
    cumulative_recovered: float
    entry_price: float
    entry_level: str
    voice_count: int


# =============================================================================
# Error
# =============================================================================


class IllegalFugueTransition(Exception):
    """Raised when an event is not valid in the current state."""


# =============================================================================
# Engine (frozen dataclass -- every apply returns a new instance)
# =============================================================================


@dataclass(frozen=True, slots=True)
class FugueEngine:
    """Fugue state machine.

    Immutable. ``apply(event)`` returns a new engine instance.
    """

    state: FugueState
    own_capital: float
    margin_amount: float
    total_shares: float
    cost_basis: float
    cumulative_recovered: float
    entry_price: float
    entry_level: str
    sub_ratio: float
    active_short: ShortDiff | None
    completed_shorts: tuple[ShortDiff, ...]
    _voices: tuple[FugueVoiceInfo, ...]

    # -- factories -----------------------------------------------------------

    @staticmethod
    def create(
        *,
        own_capital: float,
        margin_amount: float = 0.0,
        sub_ratio: float = 0.3,
    ) -> FugueEngine:
        return FugueEngine(
            state=FugueState.SCANNING,
            own_capital=own_capital,
            margin_amount=margin_amount,
            total_shares=0.0,
            cost_basis=0.0,
            cumulative_recovered=0.0,
            entry_price=0.0,
            entry_level="",
            sub_ratio=sub_ratio,
            active_short=None,
            completed_shorts=(),
            _voices=(),
        )

    # -- public queries ------------------------------------------------------

    @property
    def voice_count(self) -> int:
        return len(self._voices)

    @property
    def voices(self) -> tuple[FugueVoiceInfo, ...]:
        return self._voices

    @property
    def has_active_short_diff(self) -> bool:
        return self.active_short is not None and self.active_short.is_open

    def snapshot(self) -> FugueSnapshot:
        return FugueSnapshot(
            state=self.state,
            own_capital=self.own_capital,
            margin_amount=self.margin_amount,
            total_shares=self.total_shares,
            cost_basis=self.cost_basis,
            cumulative_recovered=self.cumulative_recovered,
            entry_price=self.entry_price,
            entry_level=self.entry_level,
            voice_count=self.voice_count,
        )

    # -- transition entry point ----------------------------------------------

    def apply(self, event: FugueEvent) -> FugueEngine:
        """Return a new engine after applying *event*."""
        _HANDLERS = {
            FugueState.SCANNING: _on_scanning,
            FugueState.POSITION_OPEN: _on_position_open,
            FugueState.COST_REDUCING: _on_cost_reducing,
            FugueState.PRINCIPAL_WITHDRAWN: _on_principal_withdrawn,
            FugueState.STOPPED_OUT: _on_stopped_out,
        }
        handler = _HANDLERS.get(self.state)
        if handler is None:
            raise IllegalFugueTransition(f"Unknown state: {self.state}")
        return handler(self, event)


# =============================================================================
# Internal helpers
# =============================================================================


def _replace(eng: FugueEngine, **kw: object) -> FugueEngine:
    """Return a new FugueEngine with selected fields replaced."""
    return FugueEngine(
        state=kw.get("state", eng.state),  # type: ignore[arg-type]
        own_capital=kw.get("own_capital", eng.own_capital),  # type: ignore[arg-type]
        margin_amount=kw.get("margin_amount", eng.margin_amount),  # type: ignore[arg-type]
        total_shares=kw.get("total_shares", eng.total_shares),  # type: ignore[arg-type]
        cost_basis=kw.get("cost_basis", eng.cost_basis),  # type: ignore[arg-type]
        cumulative_recovered=kw.get("cumulative_recovered", eng.cumulative_recovered),  # type: ignore[arg-type]
        entry_price=kw.get("entry_price", eng.entry_price),  # type: ignore[arg-type]
        entry_level=kw.get("entry_level", eng.entry_level),  # type: ignore[arg-type]
        sub_ratio=kw.get("sub_ratio", eng.sub_ratio),  # type: ignore[arg-type]
        active_short=kw.get("active_short", eng.active_short),  # type: ignore[arg-type]
        completed_shorts=kw.get("completed_shorts", eng.completed_shorts),  # type: ignore[arg-type]
        _voices=kw.get("_voices", eng._voices),  # type: ignore[arg-type]
    )


def _stop_out(eng: FugueEngine) -> FugueEngine:
    return _replace(
        eng,
        state=FugueState.STOPPED_OUT,
        active_short=None,
        _voices=(),
    )


# =============================================================================
# State handlers (349 spec section 4)
# =============================================================================


def _on_scanning(eng: FugueEngine, ev: FugueEvent) -> FugueEngine:
    if ev.event_type != FugueEventType.BUY_POINT_CONFIRMED:
        raise IllegalFugueTransition(
            f"SCANNING only accepts BUY_POINT_CONFIRMED, got {ev.event_type.name}"
        )
    total_capital = eng.own_capital + eng.margin_amount
    shares = total_capital / ev.price
    initial_voice = FugueVoiceInfo(
        level=ev.level,
        shares=shares,
        is_profit_floor=False,
    )
    return _replace(
        eng,
        state=FugueState.POSITION_OPEN,
        total_shares=shares,
        cost_basis=ev.price,
        entry_price=ev.price,
        entry_level=ev.level,
        cumulative_recovered=0.0,
        active_short=None,
        completed_shorts=(),
        _voices=(initial_voice,),
    )


def _on_position_open(eng: FugueEngine, ev: FugueEvent) -> FugueEngine:
    if ev.event_type == FugueEventType.BUY_POINT_NEGATED:
        return _stop_out(eng)
    if ev.event_type == FugueEventType.SUB_LEVEL_SELL_POINT:
        return _open_short(eng, ev.price, ev.level)
    raise IllegalFugueTransition(
        f"POSITION_OPEN does not accept {ev.event_type.name}"
    )


def _on_cost_reducing(eng: FugueEngine, ev: FugueEvent) -> FugueEngine:
    if ev.event_type == FugueEventType.BUY_POINT_NEGATED:
        return _stop_out(eng)
    if ev.event_type == FugueEventType.MAIN_LEVEL_SELL_POINT:
        return _stop_out(eng)
    if ev.event_type == FugueEventType.SUB_LEVEL_BUY_POINT:
        return _close_short(eng, ev.price)
    if ev.event_type == FugueEventType.SUB_LEVEL_SELL_POINT:
        return _open_short(eng, ev.price, ev.level)
    if ev.event_type == FugueEventType.LEVEL_UPGRADE:
        return _level_upgrade(eng, ev.level)
    raise IllegalFugueTransition(
        f"COST_REDUCING does not accept {ev.event_type.name}"
    )


def _on_principal_withdrawn(eng: FugueEngine, ev: FugueEvent) -> FugueEngine:
    if ev.event_type == FugueEventType.MAIN_LEVEL_SELL_POINT:
        return _stop_out(eng)
    if ev.event_type == FugueEventType.BUY_POINT_NEGATED:
        return _stop_out(eng)
    if ev.event_type == FugueEventType.SUB_LEVEL_SELL_POINT:
        reducing = _replace(eng, state=FugueState.COST_REDUCING)
        return _open_short(reducing, ev.price, ev.level)
    raise IllegalFugueTransition(
        f"PRINCIPAL_WITHDRAWN does not accept {ev.event_type.name}"
    )


def _on_stopped_out(eng: FugueEngine, ev: FugueEvent) -> FugueEngine:
    if ev.event_type != FugueEventType.RESET:
        raise IllegalFugueTransition(
            f"STOPPED_OUT only accepts RESET, got {ev.event_type.name}"
        )
    capital = ev.new_own_capital if ev.new_own_capital is not None else eng.own_capital
    return FugueEngine.create(
        own_capital=capital,
        margin_amount=eng.margin_amount,
        sub_ratio=eng.sub_ratio,
    )


# =============================================================================
# Short-diff operations
# =============================================================================


def _open_short(eng: FugueEngine, price: float, level: str) -> FugueEngine:
    if eng.active_short is not None and eng.active_short.is_open:
        raise IllegalFugueTransition(
            "Cannot open new short-diff while one is active"
        )
    short_shares = eng.total_shares * eng.sub_ratio
    cycle = ShortDiff(level=level, shares=short_shares, sell_price=price)
    return _replace(eng, state=FugueState.COST_REDUCING, active_short=cycle)


def _close_short(eng: FugueEngine, buy_price: float) -> FugueEngine:
    cycle = eng.active_short
    if cycle is None or not cycle.is_open:
        return eng

    # 338 amendment 4: shallow callback -- skip buy-back
    if buy_price >= cycle.sell_price:
        return eng

    closed = ShortDiff(
        level=cycle.level,
        shares=cycle.shares,
        sell_price=cycle.sell_price,
        buy_price=buy_price,
        is_open=False,
    )
    profit = closed.profit
    new_cost = eng.cost_basis - profit / eng.total_shares
    new_recovered = eng.cumulative_recovered + max(profit, 0.0)
    new_completed = eng.completed_shorts + (closed,)

    if new_recovered >= eng.own_capital:
        return _replace(
            eng,
            state=FugueState.PRINCIPAL_WITHDRAWN,
            cost_basis=new_cost,
            cumulative_recovered=new_recovered,
            active_short=None,
            completed_shorts=new_completed,
        )
    return _replace(
        eng,
        cost_basis=new_cost,
        cumulative_recovered=new_recovered,
        active_short=None,
        completed_shorts=new_completed,
    )


# =============================================================================
# Fugue split (349 spec section 4.2)
# =============================================================================


def _level_upgrade(eng: FugueEngine, new_level: str) -> FugueEngine:
    """Small-to-large: freeze current active voice, start new voice.

    The last non-profit-floor voice is the current active voice.
    It gets replaced by a frozen copy + a new active voice at the
    upgraded level.
    """
    frozen = FugueVoiceInfo(
        level=eng.entry_level,
        shares=eng.total_shares,
        is_profit_floor=True,
        cumulative_profit=eng.cumulative_recovered,
    )
    new_active = FugueVoiceInfo(
        level=new_level,
        shares=eng.total_shares,
        is_profit_floor=False,
        cumulative_profit=0.0,
    )
    # Replace the last voice (current active) with frozen + new active
    prev = eng._voices[:-1] if eng._voices else ()
    new_voices = prev + (frozen, new_active)
    return _replace(
        eng,
        entry_level=new_level,
        active_short=None,
        _voices=new_voices,
    )
