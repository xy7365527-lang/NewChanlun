"""Fugue engine tests.

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
    ShortDiff,
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


def _reach_principal_withdrawn() -> FugueEngine:
    """Reach PRINCIPAL_WITHDRAWN via extreme profit short-diff."""
    engine = _buy_in(_make_engine(equity=10.0, margin=10.0), price=1.0)
    engine = _start_reducing(engine, price=100.0)
    engine = _close_short(engine, price=0.1)
    return engine


def _reach_stopped_out() -> FugueEngine:
    """Reach STOPPED_OUT via buy point negation."""
    engine = _buy_in(_make_engine())
    return engine.apply(FugueEvent(
        event_type=FugueEventType.BUY_POINT_NEGATED,
        price=8.0, level="30min",
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

    def test_state_is_principal_withdrawn(self) -> None:
        engine = _reach_principal_withdrawn()
        assert engine.state == FugueState.PRINCIPAL_WITHDRAWN

    def test_can_continue_short_diff_on_free_position(self) -> None:
        engine = _reach_principal_withdrawn()
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=50.0,
            level="5min",
        ))
        assert engine.state == FugueState.COST_REDUCING

    def test_main_sell_point_exits(self) -> None:
        engine = _reach_principal_withdrawn()
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


# =============================================================================
# 12. Illegal transitions per state (349 spec section 4 completeness)
# =============================================================================


class TestIllegalTransitions:
    """Each state rejects events not in its transition row."""

    def test_scanning_rejects_sub_level_buy(self) -> None:
        engine = _make_engine()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
                price=10.0, level="5min",
            ))

    def test_scanning_rejects_buy_point_negated(self) -> None:
        engine = _make_engine()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.BUY_POINT_NEGATED,
                price=10.0, level="30min",
            ))

    def test_scanning_rejects_main_sell(self) -> None:
        engine = _make_engine()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.MAIN_LEVEL_SELL_POINT,
                price=10.0, level="30min",
            ))

    def test_scanning_rejects_level_upgrade(self) -> None:
        engine = _make_engine()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.LEVEL_UPGRADE,
                price=10.0, level="daily",
            ))

    def test_scanning_rejects_reset(self) -> None:
        engine = _make_engine()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.RESET,
                price=0.0, level="",
            ))

    def test_position_open_rejects_buy_point_confirmed(self) -> None:
        engine = _buy_in(_make_engine())
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.BUY_POINT_CONFIRMED,
                price=10.0, level="30min",
            ))

    def test_position_open_rejects_sub_level_buy(self) -> None:
        engine = _buy_in(_make_engine())
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
                price=9.0, level="5min",
            ))

    def test_position_open_rejects_main_sell(self) -> None:
        engine = _buy_in(_make_engine())
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.MAIN_LEVEL_SELL_POINT,
                price=12.0, level="30min",
            ))

    def test_position_open_rejects_level_upgrade(self) -> None:
        engine = _buy_in(_make_engine())
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.LEVEL_UPGRADE,
                price=12.0, level="daily",
            ))

    def test_position_open_rejects_reset(self) -> None:
        engine = _buy_in(_make_engine())
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.RESET,
                price=0.0, level="",
            ))

    def test_cost_reducing_rejects_buy_point_confirmed(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.BUY_POINT_CONFIRMED,
                price=10.0, level="30min",
            ))

    def test_cost_reducing_rejects_reset(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()))
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.RESET,
                price=0.0, level="",
            ))

    def test_principal_withdrawn_rejects_buy_point_confirmed(self) -> None:
        engine = _reach_principal_withdrawn()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.BUY_POINT_CONFIRMED,
                price=10.0, level="30min",
            ))

    def test_principal_withdrawn_rejects_sub_level_buy(self) -> None:
        engine = _reach_principal_withdrawn()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
                price=5.0, level="5min",
            ))

    def test_principal_withdrawn_rejects_level_upgrade(self) -> None:
        engine = _reach_principal_withdrawn()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.LEVEL_UPGRADE,
                price=50.0, level="daily",
            ))

    def test_principal_withdrawn_rejects_reset(self) -> None:
        engine = _reach_principal_withdrawn()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.RESET,
                price=0.0, level="",
            ))

    def test_stopped_out_rejects_buy_point_confirmed(self) -> None:
        engine = _reach_stopped_out()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.BUY_POINT_CONFIRMED,
                price=10.0, level="30min",
            ))

    def test_stopped_out_rejects_sub_level_sell(self) -> None:
        engine = _reach_stopped_out()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
                price=10.0, level="5min",
            ))

    def test_stopped_out_rejects_sub_level_buy(self) -> None:
        engine = _reach_stopped_out()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
                price=10.0, level="5min",
            ))

    def test_stopped_out_rejects_main_sell(self) -> None:
        engine = _reach_stopped_out()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.MAIN_LEVEL_SELL_POINT,
                price=10.0, level="30min",
            ))

    def test_stopped_out_rejects_level_upgrade(self) -> None:
        engine = _reach_stopped_out()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.LEVEL_UPGRADE,
                price=10.0, level="daily",
            ))

    def test_stopped_out_rejects_buy_point_negated(self) -> None:
        engine = _reach_stopped_out()
        with pytest.raises(IllegalFugueTransition):
            engine.apply(FugueEvent(
                event_type=FugueEventType.BUY_POINT_NEGATED,
                price=10.0, level="30min",
            ))


# =============================================================================
# 13. ShortDiff data structure
# =============================================================================


class TestShortDiff:
    """ShortDiff profit calculation and immutability."""

    def test_open_short_profit_is_zero(self) -> None:
        sd = ShortDiff(level="5min", shares=100.0, sell_price=11.0)
        assert sd.is_open is True
        assert sd.profit == 0.0

    def test_closed_short_profit_calculation(self) -> None:
        sd = ShortDiff(
            level="5min", shares=100.0,
            sell_price=11.0, buy_price=9.0, is_open=False,
        )
        assert sd.profit == pytest.approx(200.0)

    def test_closed_short_negative_profit(self) -> None:
        sd = ShortDiff(
            level="5min", shares=100.0,
            sell_price=9.0, buy_price=11.0, is_open=False,
        )
        assert sd.profit == pytest.approx(-200.0)

    def test_short_diff_is_frozen(self) -> None:
        sd = ShortDiff(level="5min", shares=100.0, sell_price=11.0)
        with pytest.raises(AttributeError):
            sd.is_open = False  # type: ignore[misc]


# =============================================================================
# 14. FugueVoiceInfo data structure
# =============================================================================


class TestFugueVoiceInfo:
    """Voice info data integrity."""

    def test_voice_defaults(self) -> None:
        v = FugueVoiceInfo(level="30min", shares=1000.0, is_profit_floor=False)
        assert v.cumulative_profit == 0.0

    def test_voice_is_frozen(self) -> None:
        v = FugueVoiceInfo(level="30min", shares=1000.0, is_profit_floor=False)
        with pytest.raises(AttributeError):
            v.level = "daily"  # type: ignore[misc]


# =============================================================================
# 15. Factory defaults and reset preservation
# =============================================================================


class TestFactoryAndReset:
    """create() defaults and reset field preservation."""

    def test_create_default_margin_zero(self) -> None:
        engine = FugueEngine.create(own_capital=100_000.0)
        assert engine.margin_amount == 0.0

    def test_create_default_sub_ratio(self) -> None:
        engine = FugueEngine.create(own_capital=100_000.0)
        assert engine.sub_ratio == pytest.approx(0.3)

    def test_create_custom_sub_ratio(self) -> None:
        engine = FugueEngine.create(own_capital=100_000.0, sub_ratio=0.5)
        assert engine.sub_ratio == pytest.approx(0.5)

    def test_reset_preserves_margin_and_sub_ratio(self) -> None:
        engine = FugueEngine.create(
            own_capital=100_000.0, margin_amount=50_000.0, sub_ratio=0.4,
        )
        engine = _buy_in(engine)
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=8.0, level="30min",
        ))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.RESET,
            price=0.0, level="",
        ))
        assert engine.margin_amount == pytest.approx(50_000.0)
        assert engine.sub_ratio == pytest.approx(0.4)

    def test_reset_without_new_capital_preserves_old(self) -> None:
        engine = FugueEngine.create(own_capital=100_000.0)
        engine = _buy_in(engine)
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=8.0, level="30min",
        ))
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.RESET,
            price=0.0, level="",
        ))
        assert engine.own_capital == pytest.approx(100_000.0)


# =============================================================================
# 16. Stop-out side effects
# =============================================================================


class TestStopOutSideEffects:
    """Stop-out clears voices and active short."""

    def test_stop_out_clears_active_short(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine()), price=11.0)
        assert engine.has_active_short_diff is True
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=7.0, level="30min",
        ))
        assert engine.has_active_short_diff is False
        assert engine.active_short is None

    def test_principal_withdrawn_buy_point_negated_stops_out(self) -> None:
        engine = _reach_principal_withdrawn()
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_NEGATED,
            price=0.5, level="30min",
        ))
        assert engine.state == FugueState.STOPPED_OUT
        assert engine.voice_count == 0


# =============================================================================
# 17. Multi-cycle cost reduction
# =============================================================================


class TestMultiCycleCostReduction:
    """Multiple short-diff cycles progressively lower cost basis."""

    def test_two_profitable_cycles_lower_cost_twice(self) -> None:
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000), price=10.0)
        cost_0 = engine.cost_basis

        engine = _start_reducing(engine, price=11.0)
        engine = _close_short(engine, price=9.0)
        cost_1 = engine.cost_basis
        assert cost_1 < cost_0

        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=12.0, level="5min",
        ))
        engine = _close_short(engine, price=8.0)
        cost_2 = engine.cost_basis
        assert cost_2 < cost_1

    def test_completed_shorts_accumulate(self) -> None:
        engine = _buy_in(_make_engine(), price=10.0)

        engine = _start_reducing(engine, price=11.0)
        engine = _close_short(engine, price=9.0)
        assert len(engine.completed_shorts) == 1
        assert engine.completed_shorts[0].is_open is False

        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=12.0, level="5min",
        ))
        engine = _close_short(engine, price=8.0)
        assert len(engine.completed_shorts) == 2

    def test_cumulative_recovered_sums_all_profits(self) -> None:
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000), price=10.0)
        total = engine.total_shares  # 20_000

        engine = _start_reducing(engine, price=11.0)
        engine = _close_short(engine, price=9.0)
        short_shares = total * 0.3
        profit_1 = (11.0 - 9.0) * short_shares

        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=12.0, level="5min",
        ))
        engine = _close_short(engine, price=8.0)
        profit_2 = (12.0 - 8.0) * short_shares

        assert engine.cumulative_recovered == pytest.approx(profit_1 + profit_2)


# =============================================================================
# 18. Shallow callback continuation
# =============================================================================


class TestShallowCallbackContinuation:
    """After shallow callback skip, normal operations still work."""

    def test_can_still_close_normally_after_shallow_skip(self) -> None:
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        # Shallow callback -- skipped
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
            price=12.0, level="5min",
        ))
        assert engine.has_active_short_diff is True
        # Now a real buy-back below sell price
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
            price=9.0, level="5min",
        ))
        assert engine.has_active_short_diff is False
        assert engine.cost_basis < 10.0

    def test_shallow_callback_exact_equal_price(self) -> None:
        """buy_price == sell_price also triggers skip (>= condition)."""
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        before_cost = engine.cost_basis
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
            price=11.0, level="5min",
        ))
        assert engine.cost_basis == pytest.approx(before_cost)
        assert engine.has_active_short_diff is True


# =============================================================================
# 19. Complete fugue lifecycle
# =============================================================================


class TestFugueLifecycle:
    """Full lifecycle: scan -> buy -> reduce -> split -> withdraw -> exit."""

    def test_full_lifecycle_to_principal_withdrawn_after_split(self) -> None:
        engine = FugueEngine.create(own_capital=10.0, margin_amount=10.0)
        # Scan -> position open
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_CONFIRMED,
            price=1.0, level="30min",
        ))
        assert engine.state == FugueState.POSITION_OPEN
        # Start reducing
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=10.0, level="5min",
        ))
        assert engine.state == FugueState.COST_REDUCING
        # Level upgrade -> fugue split
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.LEVEL_UPGRADE,
            price=15.0, level="daily",
        ))
        assert engine.voice_count == 2
        # New short diff at new level
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=100.0, level="4h",
        ))
        # Close short with massive profit -> principal withdrawn
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
            price=0.1, level="4h",
        ))
        assert engine.state == FugueState.PRINCIPAL_WITHDRAWN
        # Main sell point exits
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.MAIN_LEVEL_SELL_POINT,
            price=50.0, level="daily",
        ))
        assert engine.state == FugueState.STOPPED_OUT
        # Reset
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.RESET,
            price=0.0, level="",
            new_own_capital=100.0,
        ))
        assert engine.state == FugueState.SCANNING
        assert engine.own_capital == pytest.approx(100.0)

    def test_principal_withdrawn_continues_reducing_after_new_sell(self) -> None:
        """PRINCIPAL_WITHDRAWN + SUB_LEVEL_SELL_POINT -> COST_REDUCING."""
        engine = _reach_principal_withdrawn()
        engine = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_SELL_POINT,
            price=50.0, level="5min",
        ))
        assert engine.state == FugueState.COST_REDUCING
        assert engine.has_active_short_diff is True


# =============================================================================
# 20. Close short edge cases
# =============================================================================


class TestCloseShortEdgeCases:
    """Edge cases in _close_short."""

    def test_sub_level_buy_in_cost_reducing_with_no_active_short(self) -> None:
        """SUB_LEVEL_BUY_POINT when active_short=None returns engine unchanged."""
        engine = _start_reducing(_buy_in(_make_engine(), price=10.0), price=11.0)
        engine = _close_short(engine, price=9.0)
        assert engine.has_active_short_diff is False
        cost_before = engine.cost_basis
        # Try close again -- no active short
        engine_after = engine.apply(FugueEvent(
            event_type=FugueEventType.SUB_LEVEL_BUY_POINT,
            price=8.0, level="5min",
        ))
        assert engine_after.cost_basis == pytest.approx(cost_before)

    def test_short_diff_shares_use_sub_ratio(self) -> None:
        """Short-diff shares = total_shares * sub_ratio."""
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000, sub_ratio=0.4), price=10.0)
        total = engine.total_shares
        engine = _start_reducing(engine, price=11.0)
        assert engine.active_short is not None
        assert engine.active_short.shares == pytest.approx(total * 0.4)


# =============================================================================
# 21. Event type completeness
# =============================================================================


class TestEventTypeCompleteness:
    """All seven event types from 349 spec section 3 exist."""

    def test_all_seven_event_types_exist(self) -> None:
        names = {e.name for e in FugueEventType}
        assert names == {
            "BUY_POINT_CONFIRMED",
            "SUB_LEVEL_SELL_POINT",
            "SUB_LEVEL_BUY_POINT",
            "BUY_POINT_NEGATED",
            "MAIN_LEVEL_SELL_POINT",
            "LEVEL_UPGRADE",
            "RESET",
        }

    def test_event_count_is_seven(self) -> None:
        assert len(FugueEventType) == 7


# =============================================================================
# 22. Snapshot completeness
# =============================================================================


class TestSnapshotCompleteness:
    """Snapshot captures all relevant engine state."""

    def test_snapshot_fields_after_cost_reduction(self) -> None:
        engine = _buy_in(_make_engine(equity=100_000, margin=100_000), price=10.0)
        engine = _start_reducing(engine, price=11.0)
        engine = _close_short(engine, price=9.0)
        snap = engine.snapshot()
        assert snap.state == FugueState.COST_REDUCING
        assert snap.own_capital == pytest.approx(100_000.0)
        assert snap.margin_amount == pytest.approx(100_000.0)
        assert snap.total_shares == pytest.approx(20_000.0)
        assert snap.cost_basis < 10.0
        assert snap.cumulative_recovered > 0.0
        assert snap.entry_price == pytest.approx(10.0)
        assert snap.entry_level == "30min"
        assert snap.voice_count == 1

    def test_snapshot_after_principal_withdrawn(self) -> None:
        engine = _reach_principal_withdrawn()
        snap = engine.snapshot()
        assert snap.state == FugueState.PRINCIPAL_WITHDRAWN
        assert snap.cumulative_recovered >= snap.own_capital
