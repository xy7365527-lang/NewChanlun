"""持续同调 barcode 引擎测试 — a_persistence_barcode + a_divergence_topo。

覆盖：
- sublevel H0 正确性（单调趋势 vs 震荡的特征数与全幅 bar）
- H1 loop 检测（相空间振荡产生环）
- immutability（frozen dataclass 拒绝赋值）
- 级别判断（active_bars / dominant_bar = argmax persistence / ATR 阈值）
- 挂载 overlay（attach_barcodes 不侵入原对象）
- topo_force / topo_divergence（力度衰减判定 + Wasserstein 距离）
"""

import dataclasses
import math

import numpy as np
import pytest

from newchan.a_persistence_barcode import (
    Bar,
    PersistenceBarcode,
    active_bars,
    atr,
    atr_noise_threshold,
    attach_barcodes,
    barcode_from_prices,
    dominant_bar,
    rips_h1_bars,
    sublevel_h0_bars,
)
from newchan.a_divergence_topo import (
    TopoDivergence,
    topo_divergence,
    topo_divergence_from_prices,
    topo_force,
)


# ──────────────────────────────────────────────────────────────
# sublevel set H0
# ──────────────────────────────────────────────────────────────


class TestSublevelH0:
    def test_empty(self):
        assert sublevel_h0_bars([]) == ()

    def test_single_point(self):
        bars = sublevel_h0_bars([5.0])
        assert len(bars) == 1
        assert bars[0].persistence == 0.0
        assert bars[0].dimension == 0

    def test_monotone_trend_single_full_range_bar(self):
        """单调上升：H0 退化为单一 bar，persistence = 全幅。"""
        prices = [1.0, 2.0, 3.0, 4.0, 5.0]
        bars = sublevel_h0_bars(prices)
        # 单调序列只有一个存活分量（全局），封顶死亡 = max
        finite = [b for b in bars if b.persistence > 0]
        assert len(finite) == 1
        assert finite[0].birth == pytest.approx(1.0)
        assert finite[0].death == pytest.approx(5.0)
        assert finite[0].persistence == pytest.approx(4.0)

    def test_oscillation_multiple_features(self):
        """W 形振荡：产生不止一个 H0 特征（多个谷）。"""
        prices = [0.0, 5.0, 1.0, 6.0, 0.5, 7.0]
        bars = sublevel_h0_bars(prices)
        non_trivial = [b for b in bars if b.persistence > 0]
        assert len(non_trivial) >= 2

    def test_bars_sorted_desc(self):
        prices = [0.0, 5.0, 1.0, 6.0, 0.5, 7.0]
        bars = sublevel_h0_bars(prices)
        pers = [b.persistence for b in bars]
        assert pers == sorted(pers, reverse=True)

    def test_finite_cap_overrides_global_death(self):
        prices = [1.0, 2.0, 3.0]
        bars = sublevel_h0_bars(prices, finite_cap=100.0)
        glob = max(bars, key=lambda b: b.persistence)
        assert glob.death == pytest.approx(100.0)

    def test_all_deaths_finite(self):
        prices = [0.0, 3.0, 1.0, 4.0, 2.0]
        bars = sublevel_h0_bars(prices)
        assert all(math.isfinite(b.death) for b in bars)


# ──────────────────────────────────────────────────────────────
# H1 via time-delay embedding + Rips
# ──────────────────────────────────────────────────────────────


class TestRipsH1:
    def test_too_short_returns_empty(self):
        assert rips_h1_bars([1.0, 2.0, 3.0], embedding_dim=3, embedding_delay=1) == ()

    def test_oscillation_produces_loop(self):
        """正弦振荡在相空间形成环 → 至少一个显著 H1 特征。"""
        t = np.linspace(0, 8 * np.pi, 300)
        prices = (10.0 + np.sin(t)).tolist()
        bars = rips_h1_bars(prices, embedding_dim=3, embedding_delay=5)
        assert len(bars) >= 1
        assert all(b.dimension == 1 for b in bars)
        assert max(b.persistence for b in bars) > 0.1

    def test_h1_deaths_finite(self):
        t = np.linspace(0, 6 * np.pi, 200)
        prices = (np.sin(t)).tolist()
        bars = rips_h1_bars(prices, embedding_dim=3, embedding_delay=4)
        assert all(math.isfinite(b.death) for b in bars)


# ──────────────────────────────────────────────────────────────
# barcode_from_prices
# ──────────────────────────────────────────────────────────────


class TestBarcodeFromPrices:
    def test_maxdim_0_only_h0(self):
        t = np.linspace(0, 6 * np.pi, 200)
        prices = (10 + np.sin(t)).tolist()
        bc = barcode_from_prices(prices, maxdim=0)
        assert all(b.dimension == 0 for b in bc.bars)
        assert bc.n_points == 200

    def test_maxdim_1_has_both(self):
        t = np.linspace(0, 8 * np.pi, 300)
        prices = (10 + np.sin(t)).tolist()
        bc = barcode_from_prices(prices, maxdim=1, embedding_delay=5)
        dims = {b.dimension for b in bc.bars}
        assert 0 in dims and 1 in dims

    def test_by_dimension_and_totals(self):
        prices = [0.0, 3.0, 1.0, 4.0, 2.0, 5.0]
        bc = barcode_from_prices(prices, maxdim=0)
        assert bc.total_persistence(0) == pytest.approx(
            sum(b.persistence for b in bc.by_dimension(0))
        )
        assert bc.max_persistence(0) == max(b.persistence for b in bc.by_dimension(0))

    def test_empty_dimension_totals_zero(self):
        bc = barcode_from_prices([1.0, 2.0, 3.0, 4.0], maxdim=0)
        assert bc.total_persistence(1) == 0.0
        assert bc.max_persistence(1) == 0.0


# ──────────────────────────────────────────────────────────────
# immutability
# ──────────────────────────────────────────────────────────────


class TestImmutability:
    def test_bar_frozen(self):
        b = Bar(0.0, 1.0, 1.0, 0)
        with pytest.raises(dataclasses.FrozenInstanceError):
            b.birth = 2.0  # type: ignore[misc]

    def test_barcode_frozen(self):
        bc = PersistenceBarcode(bars=(), n_points=0)
        with pytest.raises(dataclasses.FrozenInstanceError):
            bc.n_points = 5  # type: ignore[misc]

    def test_slots_no_dict(self):
        """slots 生效：实例无 __dict__，无法附加任意属性。"""
        b = Bar(0.0, 1.0, 1.0, 0)
        assert not hasattr(b, "__dict__")
        bc = PersistenceBarcode(bars=(), n_points=0)
        assert not hasattr(bc, "__dict__")


# ──────────────────────────────────────────────────────────────
# 级别判断
# ──────────────────────────────────────────────────────────────


class TestLevelJudgment:
    def _barcode(self):
        bars = (
            Bar(0.0, 10.0, 10.0, 0),
            Bar(0.0, 3.0, 3.0, 0),
            Bar(0.0, 0.5, 0.5, 0),
            Bar(0.0, 2.0, 2.0, 1),
        )
        return PersistenceBarcode(bars=bars, n_points=50)

    def test_active_bars_filters_noise(self):
        bc = self._barcode()
        actives = active_bars(bc, tau=1.0)
        assert all(b.persistence > 1.0 for b in actives)
        assert len(actives) == 3  # 0.5 被滤掉

    def test_active_bars_sorted_desc(self):
        bc = self._barcode()
        actives = active_bars(bc, tau=0.0)
        pers = [b.persistence for b in actives]
        assert pers == sorted(pers, reverse=True)

    def test_dominant_is_argmax(self):
        bc = self._barcode()
        dom = dominant_bar(bc, tau=0.0)
        assert dom is not None
        assert dom.persistence == 10.0

    def test_dominant_by_dimension(self):
        bc = self._barcode()
        dom1 = dominant_bar(bc, dimension=1, tau=0.0)
        assert dom1 is not None
        assert dom1.dimension == 1
        assert dom1.persistence == 2.0

    def test_dominant_none_when_all_below_tau(self):
        bc = self._barcode()
        assert dominant_bar(bc, tau=100.0) is None


class TestATR:
    def test_atr_constant_range(self):
        highs = [11.0] * 10
        lows = [9.0] * 10
        closes = [10.0] * 10
        # TR = max(2, |11-10|, |9-10|) = 2
        assert atr(highs, lows, closes, period=14) == pytest.approx(2.0)

    def test_atr_too_short(self):
        assert atr([1.0], [1.0], [1.0]) == 0.0

    def test_noise_threshold_multiple(self):
        highs = [11.0] * 10
        lows = [9.0] * 10
        closes = [10.0] * 10
        tau = atr_noise_threshold(highs, lows, closes, multiple=2.0)
        assert tau == pytest.approx(4.0)


# ──────────────────────────────────────────────────────────────
# 挂载 overlay（不侵入原对象）
# ──────────────────────────────────────────────────────────────


class _FakeSegment:
    """模拟暴露 i0/i1 的线段。"""

    def __init__(self, i0, i1):
        self.i0 = i0
        self.i1 = i1


class TestAttachBarcodes:
    def test_attaches_per_component(self):
        prices = list(np.linspace(0, 10, 40))
        comps = [_FakeSegment(0, 19), _FakeSegment(20, 39)]
        anns = attach_barcodes(comps, prices, maxdim=0)
        assert len(anns) == 2
        assert anns[0].raw_i0 == 0 and anns[0].raw_i1 == 19
        assert anns[1].component_idx == 1
        assert all(isinstance(a.barcode, PersistenceBarcode) for a in anns)

    def test_skips_too_short(self):
        prices = list(np.linspace(0, 10, 40))
        comps = [_FakeSegment(0, 1)]  # 2 点 < min_points
        anns = attach_barcodes(comps, prices, maxdim=0, min_points=4)
        assert anns == ()

    def test_skips_unresolvable(self):
        prices = list(np.linspace(0, 10, 40))
        anns = attach_barcodes([object()], prices, maxdim=0)
        assert anns == ()

    def test_does_not_mutate_component(self):
        prices = list(np.linspace(0, 10, 40))
        seg = _FakeSegment(0, 39)
        attach_barcodes([seg], prices, maxdim=0)
        assert not hasattr(seg, "barcode")  # overlay 不侵入


# ──────────────────────────────────────────────────────────────
# 拓扑背驰
# ──────────────────────────────────────────────────────────────


class TestTopoForce:
    def test_force_equals_total_persistence(self):
        bc = barcode_from_prices([0.0, 5.0, 1.0, 6.0], maxdim=0)
        assert topo_force(bc, dimension=0) == pytest.approx(bc.total_persistence(0))

    def test_force_nonnegative(self):
        bc = barcode_from_prices([0.0, 3.0, 1.0, 4.0], maxdim=0)
        assert topo_force(bc, dimension=0) >= 0


class TestTopoDivergence:
    def test_force_decay_is_divergent(self):
        """A 段强振荡，C 段弱振荡 → 力度衰减 → 背驰。"""
        t = np.linspace(0, 6 * np.pi, 200)
        prices_a = (10 + 2.0 * np.sin(t)).tolist()  # 大振幅
        prices_c = (10 + 0.3 * np.sin(t)).tolist()  # 小振幅
        div = topo_divergence_from_prices(
            prices_a, prices_c, dimension=0, embedding_delay=5
        )
        assert div.force_a > div.force_c
        assert div.is_divergent
        assert 0.0 <= div.ratio < 1.0

    def test_force_growth_not_divergent(self):
        t = np.linspace(0, 6 * np.pi, 200)
        prices_a = (10 + 0.3 * np.sin(t)).tolist()
        prices_c = (10 + 2.0 * np.sin(t)).tolist()
        div = topo_divergence_from_prices(prices_a, prices_c, dimension=0)
        assert not div.is_divergent

    def test_noise_floor_suppresses(self):
        """A 段力度低于 noise_floor → 不报背驰。"""
        prices_a = [10.0, 10.1, 10.0, 10.1]
        prices_c = [10.0, 10.0, 10.0, 10.0]
        div = topo_divergence_from_prices(
            prices_a, prices_c, dimension=0, noise_floor=1000.0
        )
        assert not div.is_divergent

    def test_wasserstein_distance_present(self):
        t = np.linspace(0, 6 * np.pi, 200)
        prices_a = (10 + np.sin(t)).tolist()
        prices_c = (10 + 0.5 * np.sin(t)).tolist()
        div = topo_divergence_from_prices(prices_a, prices_c, dimension=0)
        assert div.wasserstein_ac >= 0
        assert math.isfinite(div.wasserstein_ac)

    def test_identical_segments_zero_wasserstein(self):
        t = np.linspace(0, 6 * np.pi, 200)
        prices = (10 + np.sin(t)).tolist()
        div = topo_divergence_from_prices(prices, prices, dimension=0)
        # cdist 代价矩阵：恒等 diagram 的 W1 精确为 0.0、平台无关（#324）
        assert div.wasserstein_ac == 0.0
        assert div.ratio == pytest.approx(1.0)
        assert not div.is_divergent  # force_c == force_a 不算衰减

    def test_identical_diagrams_bypass_solver_residual(self, monkeypatch):
        """同一 persistence diagram 的距离按定义精确为零，不消费求解器残差。"""
        barcode = PersistenceBarcode(
            bars=(
                Bar(birth=1.0, death=3.0, persistence=2.0, dimension=0),
                Bar(birth=2.0, death=5.0, persistence=3.0, dimension=0),
            ),
            n_points=4,
        )

        import persim

        monkeypatch.setattr(persim, "wasserstein", lambda *_args, **_kwargs: 1e-8)
        div = topo_divergence(barcode, barcode, dimension=0)

        assert div.wasserstein_ac == 0.0

    def test_result_is_frozen(self):
        div = TopoDivergence(1.0, 0.5, 0.5, True, 1.0, 0.3, 1)
        with pytest.raises(dataclasses.FrozenInstanceError):
            div.force_a = 2.0  # type: ignore[misc]

    def test_dominant_level_from_c(self):
        t = np.linspace(0, 6 * np.pi, 200)
        prices_a = (10 + 2 * np.sin(t)).tolist()
        prices_c = (10 + np.sin(t)).tolist()
        div = topo_divergence_from_prices(prices_a, prices_c, dimension=0)
        bc_c = barcode_from_prices(prices_c, maxdim=0)
        assert div.dominant_level_persistence == pytest.approx(
            bc_c.max_persistence(0)
        )
