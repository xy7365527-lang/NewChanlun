"""选股扫描器测试。

TDD RED phase — 先写测试，再写实现。
"""

from __future__ import annotations

import time
from unittest.mock import MagicMock

import pytest

from newchan.topology.config_space import Configuration, WalkDirection


# ── Fixtures ──────────────────────────────────────────────────


def _cfg(e: int, c: int, r: int) -> Configuration:
    """快速创建 Configuration。"""
    return Configuration(
        sigma_e=WalkDirection(e),
        sigma_c=WalkDirection(c),
        sigma_r=WalkDirection(r),
    )


def _mock_recursive_snapshot(
    *,
    levels: int = 3,
    buy_levels: tuple[int, ...] = (),
    level_ids: tuple[int, ...] | None = None,
) -> MagicMock:
    """创建 mock RecursiveOrchestratorSnapshot。

    Parameters
    ----------
    levels : int
        总递归层数（包含 level=1 的 bsp_snapshot）。
    buy_levels : tuple[int, ...]
        哪些 level_id 有活跃买点。
    level_ids : tuple[int, ...] | None
        显式指定 recursive_snapshots 的 level_id 列表。
        如果为 None，则自动生成 (2, 3, ..., levels)。
    """
    snap = MagicMock()

    # Level 1 bsp_snapshot
    bsp_snap = MagicMock()
    if 1 in buy_levels:
        bp = MagicMock()
        bp.kind = "type1"
        bp.side = "buy"
        bp.confirmed = True
        bp.settled = False
        bsp_snap.buysellpoints = [bp]
    else:
        bsp_snap.buysellpoints = []
    snap.bsp_snapshot = bsp_snap

    # Recursive level snapshots (level >= 2)
    if level_ids is None:
        level_ids = tuple(range(2, levels + 1))

    recursive_snaps = []
    for lid in level_ids:
        rs = MagicMock()
        rs.level_id = lid
        rs.moves = [MagicMock(), MagicMock(), MagicMock()]  # 3 moves
        rs.zhongshus = [MagicMock()]

        # Each level's zhongshu has a zd and zg for alignment check
        rs.zhongshus[0].zd = 100.0
        rs.zhongshus[0].zg = 110.0

        if lid in buy_levels:
            bp = MagicMock()
            bp.kind = "type1"
            bp.side = "buy"
            bp.confirmed = True
            bp.settled = False
            rs.buysellpoints = [bp]
        else:
            rs.buysellpoints = []

        recursive_snaps.append(rs)

    snap.recursive_snapshots = recursive_snaps
    return snap


# ── target_universe 测试 ─────────────────────────────────────


class TestTargetUniverse:
    """target_universe 函数测试。"""

    def test_risk_on_returns_equity(self) -> None:
        """polarity > 0 → EQUITY_UNIVERSE（sigma_e/sigma_c 非双UP时无GLD）。"""
        from newchan.trading.stock_scanner import EQUITY_UNIVERSE, target_universe

        # (+,0,0) → polarity = 1, sigma_c != UP → 纯 EQUITY
        config = _cfg(1, 0, 0)
        result = target_universe(config)
        assert result == EQUITY_UNIVERSE

    def test_risk_on_full_includes_gold(self) -> None:
        """(+,+,+) → EQUITY + GOLD（sigma_e=UP 且 sigma_c=UP）。"""
        from newchan.trading.stock_scanner import (
            EQUITY_UNIVERSE,
            GOLD_UNIVERSE,
            target_universe,
        )

        config = _cfg(1, 1, 1)
        result = target_universe(config)
        assert result == EQUITY_UNIVERSE + GOLD_UNIVERSE

    def test_risk_off_returns_rate(self) -> None:
        """polarity < 0 → RATE_UNIVERSE。"""
        from newchan.trading.stock_scanner import RATE_UNIVERSE, target_universe

        # (-,-,-) → polarity = -3
        config = _cfg(-1, -1, -1)
        result = target_universe(config)
        assert result == RATE_UNIVERSE

    def test_neutral_returns_all(self) -> None:
        """polarity == 0（中性）→ EQUITY + RATE（K4 不筛选，层2独立生效）。"""
        from newchan.trading.stock_scanner import (
            EQUITY_UNIVERSE,
            RATE_UNIVERSE,
            target_universe,
        )

        # (+,0,-) → polarity = 0
        config = _cfg(1, 0, -1)
        result = target_universe(config)
        assert result == EQUITY_UNIVERSE + RATE_UNIVERSE

    def test_mild_risk_on(self) -> None:
        """polarity = 1 → EQUITY_UNIVERSE。"""
        from newchan.trading.stock_scanner import EQUITY_UNIVERSE, target_universe

        # (+,0,0) → polarity = 1
        config = _cfg(1, 0, 0)
        result = target_universe(config)
        assert result == EQUITY_UNIVERSE

    def test_mild_risk_off(self) -> None:
        """polarity = -1 → RATE_UNIVERSE。"""
        from newchan.trading.stock_scanner import RATE_UNIVERSE, target_universe

        # (-,0,0) → polarity = -1
        config = _cfg(-1, 0, 0)
        result = target_universe(config)
        assert result == RATE_UNIVERSE

    def test_gold_augmentation(self) -> None:
        """sigma_e=UP 且 sigma_c=UP → 包含 GLD。"""
        from newchan.trading.stock_scanner import GOLD_UNIVERSE, target_universe

        # (+,+,0) → polarity = 2, sigma_e=UP, sigma_c=UP
        config = _cfg(1, 1, 0)
        result = target_universe(config)
        for symbol in GOLD_UNIVERSE:
            assert symbol in result


# ── compute_nesting_tightness 测试 ────────────────────────────


class TestComputeNestingTightness:
    """compute_nesting_tightness 函数测试。"""

    def test_no_buy_points_zero_tightness(self) -> None:
        """无买点的 snapshot → tightness = 0。"""
        from newchan.trading.stock_scanner import compute_nesting_tightness

        snap = _mock_recursive_snapshot(levels=3, buy_levels=())
        tightness, level = compute_nesting_tightness(snap)
        assert tightness == 0.0
        assert level == ""

    def test_single_level_buy_point(self) -> None:
        """单层有买点 → tightness > 0。"""
        from newchan.trading.stock_scanner import compute_nesting_tightness

        snap = _mock_recursive_snapshot(levels=3, buy_levels=(1,))
        tightness, level = compute_nesting_tightness(snap)
        assert tightness > 0.0
        assert level != ""

    def test_multi_level_alignment(self) -> None:
        """多层买点对齐 → 更高 tightness。"""
        from newchan.trading.stock_scanner import compute_nesting_tightness

        snap_single = _mock_recursive_snapshot(levels=3, buy_levels=(1,))
        snap_multi = _mock_recursive_snapshot(levels=3, buy_levels=(1, 2, 3))

        t_single, _ = compute_nesting_tightness(snap_single)
        t_multi, _ = compute_nesting_tightness(snap_multi)

        assert t_multi > t_single

    def test_operation_level_is_highest(self) -> None:
        """operation_level = 有买点的最高级别。"""
        from newchan.trading.stock_scanner import compute_nesting_tightness

        snap = _mock_recursive_snapshot(levels=4, buy_levels=(1, 3))
        _, level = compute_nesting_tightness(snap)
        assert level == "L3"


# ── scan_candidates 测试 ──────────────────────────────────────


class TestScanCandidates:
    """scan_candidates 完整流程测试。"""

    def test_full_scan_with_candidates(self) -> None:
        """有合格候选 → selected 非 None，按 tightness 降序。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 1, 1)  # polarity = 3, risk-on
        snapshots = {
            "XLK": _mock_recursive_snapshot(levels=3, buy_levels=(1, 2, 3)),
            "XLF": _mock_recursive_snapshot(levels=3, buy_levels=(1,)),
            "XLE": _mock_recursive_snapshot(levels=3, buy_levels=()),
        }
        result = scan_candidates(config, snapshots)

        assert result.polarity == 3
        assert result.selected is not None
        assert result.selected.symbol == "XLK"
        # candidates 按 tightness 降序
        assert len(result.candidates) >= 1
        for i in range(len(result.candidates) - 1):
            assert (
                result.candidates[i].nesting_tightness
                >= result.candidates[i + 1].nesting_tightness
            )

    def test_neutral_polarity_scans_all(self) -> None:
        """polarity == 0（中性）→ 扫描 EQUITY + RATE，有买点则选中。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 0, -1)  # polarity = 0
        snapshots = {
            "XLK": _mock_recursive_snapshot(levels=3, buy_levels=(1, 2, 3)),
            "TLT": _mock_recursive_snapshot(levels=3, buy_levels=(1,)),
        }
        result = scan_candidates(config, snapshots)

        assert result.polarity == 0
        assert result.selected is not None
        assert len(result.candidates) >= 1

    def test_empty_candidate_snapshots(self) -> None:
        """空 candidate_snapshots → selected = None。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 1, 1)  # polarity = 3
        result = scan_candidates(config, {})

        assert result.selected is None
        assert result.candidates == ()

    def test_all_zero_tightness(self) -> None:
        """所有候选 tightness = 0 → selected = None。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 1, 1)
        snapshots = {
            "XLK": _mock_recursive_snapshot(levels=3, buy_levels=()),
            "XLF": _mock_recursive_snapshot(levels=3, buy_levels=()),
        }
        result = scan_candidates(config, snapshots)

        assert result.selected is None

    def test_single_candidate(self) -> None:
        """单一候选且合格 → selected = 该候选。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 1, 1)
        snapshots = {
            "XLK": _mock_recursive_snapshot(levels=3, buy_levels=(1, 2)),
        }
        result = scan_candidates(config, snapshots)

        assert result.selected is not None
        assert result.selected.symbol == "XLK"
        assert len(result.candidates) == 1

    def test_missing_symbol_in_snapshots(self) -> None:
        """universe 中的 symbol 不在 snapshots 中 → 跳过。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 1, 1)  # universe = EQUITY (XLK, XLF, XLE, XLV, XLY)
        # 只提供 XLK
        snapshots = {
            "XLK": _mock_recursive_snapshot(levels=3, buy_levels=(1, 2)),
        }
        result = scan_candidates(config, snapshots)

        # 应该只处理 XLK，不因缺少其他 symbol 而报错
        assert result.selected is not None
        assert result.selected.symbol == "XLK"

    def test_scan_result_immutability(self) -> None:
        """ScanResult 是 frozen dataclass。"""
        from newchan.trading.stock_scanner import scan_candidates

        config = _cfg(1, 1, 1)
        snapshots = {
            "XLK": _mock_recursive_snapshot(levels=3, buy_levels=(1,)),
        }
        result = scan_candidates(config, snapshots)

        with pytest.raises(AttributeError):
            result.polarity = 0  # type: ignore[misc]

    def test_scan_result_has_timestamp(self) -> None:
        """ScanResult 包含 scan_ts。"""
        from newchan.trading.stock_scanner import scan_candidates

        before = time.time()
        config = _cfg(1, 1, 1)
        snapshots = {}
        result = scan_candidates(config, snapshots)
        after = time.time()

        assert before <= result.scan_ts <= after
