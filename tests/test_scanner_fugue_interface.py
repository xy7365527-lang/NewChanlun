"""扫描器→赋格状态机接口工程测试。

工位：scanner-fugue-interface (integration)
来源：352号下游推论2

验证内容：
1. rep([S]) → FugueEvent 转换（BUY_POINT_CONFIRMED 事件产生）
2. FugueEngine SCANNING → POSITION_OPEN 转移
3. 完整管线：build_scanner_pool → try_scanner_to_fugue_transition 往返
4. 前置条件检查：无代表元 / 非 SCANNING 状态
5. 事件字段正确性：price, level 传递

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
    build_quotient_space,
)
from newchan.trading.scanner_fugue import (
    ScannerFugueTransition,
    scanner_to_fugue_event,
    try_scanner_to_fugue_transition,
)
from newchan.trading.scanner_pool import ScannerPoolResult, build_scanner_pool


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
        sigma_p=WalkDirection(e),
        sigma_c=WalkDirection(c),
        sigma_r=WalkDirection(r),
    )


def _pool_with_rep(rep: TargetAttributes | None) -> ScannerPoolResult:
    """构造携带指定代表元的 ScannerPoolResult。"""
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
# 转换：rep([S]) → FugueEvent
# ═══════════════════════════════════════════════════════════════


class TestRepToFugueEvent:
    """验证代表元到赋格事件的映射正确性。"""

    def test_event_type_is_buy_point_confirmed(self) -> None:
        """转换产生 BUY_POINT_CONFIRMED 事件类型。"""
        rep = _target("GLD", FoldChannel.AU)
        event = scanner_to_fugue_event(rep, price=100.0)
        assert event.event_type is FugueEventType.BUY_POINT_CONFIRMED

    def test_level_from_level_magnitude(self) -> None:
        """level 字段 = "L{level_magnitude}"。"""
        rep = _target("GLD", FoldChannel.AU, level_magnitude=5.0)
        event = scanner_to_fugue_event(rep, price=50.0)
        assert event.level == "L5"

    def test_level_truncates_to_int(self) -> None:
        """level_magnitude 为浮点数时截断为整数。"""
        rep = _target("GLD", FoldChannel.AU, level_magnitude=3.7)
        event = scanner_to_fugue_event(rep, price=50.0)
        assert event.level == "L3"

    def test_price_passthrough(self) -> None:
        """价格参数直接传递。"""
        rep = _target("GLD", FoldChannel.AU)
        event = scanner_to_fugue_event(rep, price=42.5)
        assert event.price == 42.5


# ═══════════════════════════════════════════════════════════════
# 安全转移：try_scanner_to_fugue_transition
# ═══════════════════════════════════════════════════════════════


class TestScannerToFugueTransition:
    """验证完整的 SCANNING → POSITION_OPEN 转移。"""

    def test_successful_transition(self) -> None:
        """有代表元 + SCANNING 状态 → 成功转移到 POSITION_OPEN。"""
        rep = _target("GLD", FoldChannel.AU, tightness=3.0)
        pool = _pool_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is not None
        assert isinstance(result, ScannerFugueTransition)
        assert result.new_engine.state is FugueState.POSITION_OPEN
        assert result.selected_symbol == "GLD"

    def test_no_representative_returns_none(self) -> None:
        """无代表元（无收敛标的）→ 返回 None，不触发转移。"""
        pool = _pool_with_rep(None)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is None

    def test_non_scanning_state_returns_none(self) -> None:
        """赋格状态机不在 SCANNING 状态 → 返回 None。"""
        rep = _target("GLD", FoldChannel.AU)
        pool = _pool_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        # 先转到 POSITION_OPEN
        opened = engine.apply(FugueEvent(
            event_type=FugueEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="L1",
        ))
        result = try_scanner_to_fugue_transition(pool, opened, price=100.0)
        assert result is None

    def test_transition_carries_event(self) -> None:
        """转移结果包含正确的 FugueEvent。"""
        rep = _target("GLD", FoldChannel.AU, level_magnitude=4.0)
        pool = _pool_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=55.0)
        assert result is not None
        assert result.event.event_type is FugueEventType.BUY_POINT_CONFIRMED
        assert result.event.price == 55.0
        assert result.event.level == "L4"

    def test_transition_immutability(self) -> None:
        """原 engine 不被修改（不可变语义）。"""
        rep = _target("GLD", FoldChannel.AU)
        pool = _pool_with_rep(rep)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is not None
        # 原 engine 仍在 SCANNING
        assert engine.state is FugueState.SCANNING
        # 新 engine 在 POSITION_OPEN
        assert result.new_engine.state is FugueState.POSITION_OPEN


# ═══════════════════════════════════════════════════════════════
# 端到端集成：build_scanner_pool → transition
# ═══════════════════════════════════════════════════════════════


class TestEndToEndIntegration:
    """从标的池生成到赋格转移的完整链路。"""

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
        # GLD rank = 3.0 * 4 = 12 > USO rank = 2.0 * 3 = 6
        assert result.selected_symbol == "GLD"
        assert result.new_engine.state is FugueState.POSITION_OPEN
        # 满仓满融：(100k + 100k) / 100 = 2000 shares
        assert result.new_engine.total_shares == pytest.approx(200_000.0 / 100.0)

    def test_pool_empty_no_transition(self) -> None:
        """空标的池 → 无转移。"""
        pool = build_scanner_pool(_config(), ())
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is None

    def test_multiple_channels_highest_rank_wins(self) -> None:
        """多折叠通道场景：rank 最高的等价类代表元被选中。"""
        cfg = _config(1, 1, 0)
        targets = (
            _target("GLD", FoldChannel.AU, tightness=1.0, level_magnitude=2.0),     # rank = 4
            _target("USO", FoldChannel.OIL, tightness=2.0, level_magnitude=3.0),    # rank = 6
            _target("TLT", FoldChannel.BOND, tightness=4.0, level_magnitude=4.0),   # rank = 8
        )
        pool = build_scanner_pool(cfg, targets)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=50.0)
        assert result is not None
        assert result.selected_symbol == "TLT"  # rank 8 最高
        assert result.event.level == "L4"

    def test_fold_merge_selects_class_rep(self) -> None:
        """同通道多标的折叠合并后，选中类内代表元（T最大者）。"""
        cfg = _config(1, 1, 0)
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0, level_magnitude=5.0,
                    liquidity=500.0),
            _target("GC=F", FoldChannel.AU, tightness=2.5, level_magnitude=4.0,
                    liquidity=300.0),
        )
        pool = build_scanner_pool(cfg, targets)
        engine = FugueEngine.create(own_capital=100_000.0)
        result = try_scanner_to_fugue_transition(pool, engine, price=100.0)
        assert result is not None
        assert result.selected_symbol == "GLD"  # T=3.0 > 2.5
        assert result.event.level == "L5"
