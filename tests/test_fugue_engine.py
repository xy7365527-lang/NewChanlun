"""Fugue engine tests -- TDD RED phase.

Epistemology: L0 (derived from 349 spec + 267 section 7 definitions).
Genealogy: 349 (fugue state machine formal definition).
"""

from __future__ import annotations

import pytest

from newchan.fugue_engine import (
    FugueEngine,
    FugueEvent,
    FugueEventType,
    FugueSnapshot,
    FugueState,
    FugueVoiceInfo,
    IllegalFugueTransition,
)


# -- factories ---------------------------------------------------------------


def _make_engine(
    *,
    equity: float = 100_000.0,
    margin: float = 100_000.0,
    sub_ratio: float = 0.3,
) -> FugueEngine:
    """Create engine in SCANNING state."""
    return FugueEngine.create(
        own_capital=equity,
        margin_amount=margin,
        sub_ratio=sub_ratio,
    )


def _buy_in(engine: FugueEngine, price: float = 10.0) -> FugueEngine:
    """SCANNING -> POSITION_OPEN."""
    return engine.apply(FugueEvent(
        event_type=FugueEventType.BUY_POINT_CONFIRMED,
        price=price,
        level="30min",
    ))


def _start_reducing(engine: FugueEngine, price: float = 11.0) -> FugueEngine:
    """POSITION_OPEN -> COST_REDUCING."""
    return engine.apply(FugueEvent(
        event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
        price=price,
        level="5min",
    ))


def _close_short(engine: FugueEngine, price: float = 9.0) -> FugueEngine:
    """Close active short diff cycle by buying back."""
    return engine.apply(FugueEvent(
        event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
        price=price,
        level="5min",
    ))


# =============================================================================
# 1. State definitions and initial state
# =============================================================================


class TestStateDefinitions:
    """Five core states from 349 spec section 2."""

    def test_initial_state_is_scanning(self) -> None:
        engine = _make_engine()
        assert engine.state == FugueState.SCANNING

    def test_initial_voice_count_is_zero(self) -> None:
        engine = _make_engine()
        assert engine.voice_count == 0

    def test_all_five_states_exist(self) -> None:
        names = {s.name for s in FugueState}
        assert names == {
            "SCANNING",
            "POSITION_OPEN",
            "COST_REDUCING",
            "PRINCIPAL_WITHDRAWN",
            "STOPPED_OUT",
        }


# =============================================================================
# 2. Core transition: SCANNING -> POSITION_OPEN
# =============================================================================


class TestScanningToOpen:
    """BUY_POINT_CONFIRMED triggers full position entry."""

    def test_state_changes(self) -> None:
        engine = _buy_in(_make_engine())
        assert engine.state == FugueState.POSITION_OPEN

    def test_shares_calculated(self) -> None:
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000), price=10.0)
        assert engine.total_shares == pytest.approx(20_000.0)

    def test_cost_basis_equals_entry_price(self) -> None:
        engine = _buy_in(_make_engine(), price=10.0)
        assert engine.cost_basis == pytest.approx(10.0)

    def test_voice_count_becomes_one(self) -> None:
        engine = _buy_in(_make_engine())
        assert engine.voice_count == 1

    def test_scanning_rejects_non_buy(self) -> None:
        engine = _make_engine()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
                price=11.0,
                level="5min",
            ))


# =============================================================================
# 3. Core transition: POSITION_OPEN -> COST_REDUCING
# =============================================================================


class TestOpenToReducing:
    """SUB_LEVEL_SELL_POINT starts first short-diff cycle."""

    def test_state_changes(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        assert engine.state == FugueState.COST_REDUCING

    def test_has_active_short_diff(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        assert engine.has_active_short_diff is True

    def test_position_open_stop_loss(self) -> None:
        engine = _buy_in(_make_engine())
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        ))
        assert engine.state == FugueState.STOPPED_OUT


# =============================================================================
# 4. Cost reduction cycle
# =============================================================================


class TestCostReduction:
    """Short-diff buy-back lowers cost basis."""

    def test_cost_decreases_on_profitable_cycle(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        engine = _close_short(engine, price=9.0)
        assert engine.cost_basis < 10.0

    def test_cost_formula_exact(self) -> None:
        """cost(t) = cost(t-1) - profit(t) / total_shares."""
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000), price=10.0)
        total = engine.total_shares  # 20_000
        engine = _start_reducing(engine, price=11.0)
        short_shares = total * 0.3  # 6_000
        engine = _close_short(engine, price=9.0)
        profit = (11.0 - 9.0) * short_shares  # 12_000
        expected_cost = 10.0 - profit / total  # 10.0 - 0.6 = 9.4
        assert engine.cost_basis == pytest.approx(expected_cost)

    def test_cumulative_recovered_tracks_profit(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        engine = _close_short(engine, price=9.0)
        assert engine.cumulative_recovered > 0.0

    def test_no_active_short_after_close(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        engine = _close_short(engine, price=9.0)
        assert engine.has_active_short_diff is False

    def test_open_new_cycle_after_close(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        engine = _close_short(engine, price=9.0)
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=12.0,
            level="5min",
        ))
        assert engine.has_active_short_diff is True

    def test_cannot_open_when_active_exists(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()), price=11.0)
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
                price=12.0,
                level="5min",
            ))


# =============================================================================
# 5. Principal withdrawal
# =============================================================================


class TestPrincipalWithdrawn:
    """Cumulative recovered >= own_capital triggers state change."""

    def _reach_withdrawn(self) -> FugueEngine:
        engine = _buy_in(_make_engine(equity=10.0, margin=10.0), price=1.0)
        engine = _start_reducing(engine, price=100.0)
        engine = _close_short(engine, price=0.1)
        return engine

    def test_state_is_principal_withdrawn(self) -> None:
        engine = self._reach_withdrawn()
        assert engine.state == FugueState.PRINCIPAL_WITHDRAWN

    def test_can_continue_short_diff_on_free_position(self) -> None:
        engine = self._reach_withdrawn()
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=50.0,
            level="5min",
        ))
        assert engine.state == FugueState.COST_REDUCING

    def test_main_sell_point_exits(self) -> None:
        engine = self._reach_withdrawn()
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.MAIN_LEVEL_SELL_POINT,
            price=50.0,
            level="30min",
        ))
        assert engine.state == FugueState.STOPPED_OUT


# =============================================================================
# 6. Stop loss and reset
# =============================================================================


class TestStopLossAndReset:
    """All holding states -> STOPPED_OUT -> SCANNING."""

    def test_cost_reducing_stop_loss(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=7.0,
            level="30min",
        ))
        assert engine.state == FugueState.STOPPED_OUT
        assert engine.voice_count == 0

    def test_main_level_sell_point_exits(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.MAIN_LEVEL_SELL_POINT,
            price=15.0,
            level="30min",
        ))
        assert engine.state == FugueState.STOPPED_OUT

    def test_reset_back_to_scanning(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=7.0,
            level="30min",
        ))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.RESET,
            price=0.0,
            level="",
        ))
        assert engine.state == FugueState.SCANNING

    def test_reset_with_new_capital(self) -> None:
        engine = _buy_in(_make_engine(equity=100_000))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        ))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.RESET,
            price=0.0,
            level="",
            new_own_capital=80_000.0,
        ))
        assert engine.own_capital == pytest.approx(80_000.0)


# =============================================================================
# 7. Fugue split (LEVEL_UPGRADE) -- core of 349 spec
# =============================================================================


class TestFugueSplit:
    """Small-to-large upgrade creates new voice, freezes old one."""

    def _upgrade(self) -> FugueEngine:
        engine = _start_reducing(_buy_in(_make_engine()), price=11.0)
        return engine.apply(FugueEvent(
            event_type=FugueEventType.LEVEL_UPGRADE,
            price=15.0,
            level="daily",
        ))

    def test_state_remains_cost_reducing(self) -> None:
        engine = self._upgrade()
        assert engine.state == FugueState.COST_REDUCING

    def test_voice_count_increases(self) -> None:
        engine = self._upgrade()
        assert engine.voice_count == 2

    def test_old_voice_is_profit_floor(self) -> None:
        engine = self._upgrade()
        voices = engine.voices
        old = voices[0]
        assert old.is_profit_floor is True
        assert old.level == "30min"

    def test_new_voice_is_active(self) -> None:
        engine = self._upgrade()
        voices = engine.voices
        new = voices[-1]
        assert new.is_profit_floor is False
        assert new.level == "daily"

    def test_entry_level_updated(self) -> None:
        engine = self._upgrade()
        assert engine.entry_level == "daily"

    def test_active_short_diff_cleared(self) -> None:
        engine = self._upgrade()
        assert engine.has_active_short_diff is False

    def test_recursive_split(self) -> None:
        """Second level upgrade produces 3 voices."""
        engine = self._upgrade()
        # Open new short diff at daily level
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=20.0,
            level="4h",
        ))
        # Second upgrade: daily -> weekly
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.LEVEL_UPGRADE,
            price=25.0,
            level="weekly",
        ))
        assert engine.voice_count == 3
        assert engine.entry_level == "weekly"
        # First two voices are profit floors
        assert engine.voices[0].is_profit_floor is True
        assert engine.voices[1].is_profit_floor is True
        assert engine.voices[2].is_profit_floor is False

    def test_split_preserves_cumulative_profit(self) -> None:
        """Old voice captures cumulative_recovered at split time."""
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000), price=10.0)
        engine = _start_reducing(engine, price=11.0)
        engine = _close_short(engine, price=9.0)
        recovered_before = engine.cumulative_recovered
        assert recovered_before > 0.0
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=12.0,
            level="5min",
        ))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.LEVEL_UPGRADE,
            price=15.0,
            level="daily",
        ))
        old_voice = engine.voices[-2]  # voice before the newest
        assert old_voice.cumulative_profit == pytest.approx(recovered_before)


# =============================================================================
# 8. Strong trend shallow callback (338 amendment 4)
# =============================================================================


class TestShallowCallback:
    """Buy price >= previous sell price -> skip buy-back."""

    def test_shallow_callback_ignored(self) -> None:
        """When buy_price >= last sell_price, buy-back is skipped."""
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        # Buy back at higher price than sell -> shallow callback
        before_cost = engine.cost_basis
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
            price=12.0,  # >= 11.0 sell price
            level="5min",
        ))
        # Cost should not change (buy-back skipped)
        assert engine.cost_basis == pytest.approx(before_cost)
        # Short diff still active (not closed)
        assert engine.has_active_short_diff is True


# =============================================================================
# 9. No swap events
# =============================================================================


class TestNoSwap:
    """267 section 5: no position swapping."""

    def test_no_swap_event_type(self) -> None:
        names = {e.name for e in FugueEventType}
        assert "SWAP" not in names
        assert "SWITCH_POSITION" not in names
        assert "CHANGE_STOCK" not in names


# =============================================================================
# 10. Immutability
# =============================================================================


class TestImmutability:
    """All state is frozen dataclass."""

    def test_engine_is_frozen(self) -> None:
        engine = _make_engine()
        with pytest.raises(AttributeError):
            engine.state = FugueState.POSITION_OPEN  # type: ignore[misc]

    def test_apply_returns_new_instance(self) -> None:
        engine = _make_engine()
        new = _buy_in(engine)
        assert new is not engine
        assert engine.state == FugueState.SCANNING
        assert new.state == FugueState.POSITION_OPEN


# =============================================================================
# 11. Snapshot
# =============================================================================


class TestSnapshot:
    """FugueSnapshot captures engine state."""

    def test_snapshot_roundtrip(self) -> None:
        engine = _buy_in(_make_engine())
        snap = engine.snapshot()
        assert isinstance(snap, FugueSnapshot)
        assert snap.state == FugueState.POSITION_OPEN
        assert snap.voice_count == 1

    def test_snapshot_after_split(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()), price=11.0)
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.LEVEL_UPGRADE,
            price=15.0,
            level="daily",
        ))
        snap = engine.snapshot()
        assert snap.voice_count == 2
        assert snap.entry_level == "daily"
