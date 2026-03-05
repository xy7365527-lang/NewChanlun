"""扫描器→赋格状态机接口测试。

验证：
1. scanner 输出 rep([S]) 能正确转换为 FugueEvent（BUY_POINT_CONFIRMED）
2. 转换后的事件能驱动 FugueEngine 从 SCANNING → POSITION_OPEN
3. 多等价类选择场景
4. 边界条件：无候选、无代表元

认识论等级：L0（从349号和352号定义直接推导）。
谱系引用：349号赋格状态机、352号多标的扫描器。
"""

from __future__ import annotations

import pytest

from newchan.fugue_engine import (
    FugueEngine,
    FugueEvent,
    FugueEventType,
    FugueState,
)
from newchan.topology.config_space import Configuration, WalkDirection
from newchan.trading.fold_equivalence import (
    DTriState,
    FoldChannel,
    TargetAttributes,
)
from newchan.trading.scanner_pool import ScannerPoolResult, build_scanner_pool
from newchan.trading.scanner_fugue import (
    ScannerFugueTransition,
    scanner_to_fugue_event,
    try_scanner_to_fugue_transition,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


def _target(
    symbol: str,
    fold: FoldChannel,
    *,
    tightness: float = 2.0,
    level_magnitude: float = 3.0,
    liquidity: float = 500.0,
) -> TargetAttributes:
    return TargetAttributes(
        symbol=symbol,
        fold_channel=fold,
        sector="",
        d_tri_state=DTriState.RETAIN,
        tightness=tightness,
        level_magnitude=level_magnitude,
        liquidity=liquidity,
    )


def _config(e: int = 1, c: int = 1, r: int = 0) -> Configuration:
    return Configuration(
        sigma_e=WalkDirection(e),
        sigma_c=WalkDirection(c),
        sigma_r=WalkDirection(r),
    )


def _pool_result_with_rep(
    rep: TargetAttributes | None,
) -> ScannerPoolResult:
    """构造一个携带指定代表元的 ScannerPoolResult。"""
    from newchan.trading.fold_equivalence import build_quotient_space

    cfg = _config()
    if rep is None:
        return ScannerPoolResult(
            config=cfg,
            polarity=2,
            target_matrix=frozenset({"equity", "au"}),
            equivalence_classes=(),
            top_representative=None,
        )
    eqs = build_quotient_space((rep,))
    return ScannerPoolResult(
        config=cfg,
        polarity=2,
        target_matrix=frozenset({"equity", "au"}),
        equivalence_classes=eqs,
        top_representative=rep,
    )


# ═══════════════════════════════════════════════════════════════
# scanner_to_fugue_event 转换
# ═══════════════════════════════════════════════════════════════


class TestScannerToFugueEvent:
    """验证 rep([S]) → FugueEvent 的转换。"""

    def test_produces_buy_point_confirmed(self) -> None:
        """转换产生 BUY_POINT_CONFIRMED 事件。"""
        rep = _target("GLD", FoldChannel.AU, tightness=3.0, level_magnitude=5.0)
        event = scanner_to_fugue_event(rep, price=100.0)
        assert event.event_type is FugueEventType.BUY_POINT_CONFIRMED
        assert event.price == 100.0

    def test_level_from_level_magnitude(self) -> None:
        """事件的 level 字段来自代表元的 level_magnitude。"""
        rep = _target("GLD", FoldChannel.AU, level_magnitude=3.0)
        event = scanner_to_fugue_event(rep, price=50.0)
        assert event.level == "L3"

    def test_price_passed_through(self) -> None:
        """价格参数直接传递。"""
        rep = _target("GLD", FoldChannel.AU)
        event = scanner_to_fugue_event(rep, price=42.5)
        assert event.price == 42.5


# ═══════════════════════════════════════════════════════════════
# try_scanner_to_fugue_transition 完整流程
# ═══════════════════════════════════════════════════════════════


class TestTryScannerToFugueTransition:
    """验证 scanner 输出到赋格状态机转移的完整流程。"""

    def test_successful_transition(self) -> None:
        """有代表元时成功转移。"""
        rep = _target("GLD", FoldChannel.AU, tightness=3.0)
        pool = _pool_result_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is not None
        assert result.new_engine.state is FugueState.POSITION_OPEN
        assert result.selected_symbol == "GLD"

    def test_no_representative_returns_none(self) -> None:
        """无代表元时返回 None。"""
        pool = _pool_result_with_rep(None)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is None

    def test_engine_must_be_scanning(self) -> None:
        """赋格状态机必须在 SCANNING 状态。"""
        rep = _target("GLD", FoldChannel.AU)
        pool = _pool_result_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        # 先转到 POSITION_OPEN
        opened = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="30min",
        ))
        result = try_scanner_to_fugue_transition(pool, opened, price=100.0)
        assert result is None

    def test_result_carries_event(self) -> None:
        """结果包含用于转移的 FugueEvent。"""
        rep = _target("GLD", FoldChannel.AU, level_magnitude=4.0)
        pool = _pool_result_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=55.0)
        assert result is not None
        assert result.event.event_type is FugueEventType.BUY_POINT_CONFIRMED
        assert result.event.price == 55.0
        assert result.event.level == "L4"


# ═══════════════════════════════════════════════════════════════
# 集成：scanner_pool + fugue
# ═══════════════════════════════════════════════════════════════


class TestIntegration:
    """端到端集成测试。"""

    def test_pool_to_fugue_roundtrip(self) -> None:
        """build_scanner_pool → try_scanner_to_fugue_transition 往返。"""
        cfg = _config(1, 1, 0)
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0, level_magnitude=5.0),
            _target("USO", FoldChannel.OIL, tightness=2.0, level_magnitude=3.0),
        )
        pool = build_scanner_pool(cfg, targets)
        engine = FugueEngine.create(own_capital=100_000.0, margin_amount=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is not None
        # GLD has rank = 3.0 * 4 = 12, USO has rank = 2.0 * 3 = 6
        assert result.selected_symbol == "GLD"
        assert result.new_engine.state is FugueState.POSITION_OPEN
        assert result.new_engine.total_shares == pytest.approx(200_000.0 / 100.0)
