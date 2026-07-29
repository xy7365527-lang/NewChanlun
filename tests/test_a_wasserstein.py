"""Wasserstein-1 匹配桥接测试（a_wasserstein，issue #324）。

核心约束：恒等持续图的 W1 **精确**为 0.0 且平台无关（cdist 逐坐标差分，
不走 sklearn 的 ‖x‖²+‖y‖²−2x·y 平方展开——后者在零距离处留 ~1e-7 残差且
随平台 BLAS 变化，是 #324 里 ubuntu x86 红 / macOS arm64 绿的根因）。
"""

import math

import numpy as np
import pytest

from newchan.a_wasserstein import wasserstein_1

EMPTY = np.empty((0, 2), dtype=float)


class TestZeroDistanceExact:
    @pytest.mark.unit
    def test_identical_diagram_exactly_zero(self):
        dgm = np.array([[1.0, 3.0], [2.0, 5.0], [0.5, 0.75]])
        assert wasserstein_1(dgm, dgm) == 0.0

    @pytest.mark.unit
    def test_identical_random_diagrams_exactly_zero(self):
        rng = np.random.RandomState(324)
        for n in (1, 3, 7, 20):
            birth = rng.uniform(0.0, 1e4, n)
            death = birth + rng.uniform(0.0, 1e3, n)
            dgm = np.stack([birth, death], axis=1)
            assert wasserstein_1(dgm, dgm) == 0.0

    @pytest.mark.unit
    def test_both_empty_is_zero(self):
        assert wasserstein_1(EMPTY, EMPTY) == 0.0


class TestKnownValues:
    @pytest.mark.unit
    def test_single_point_pairing_beats_diagonal(self):
        """(0,2) ↔ (0,4)：点对点代价 2.0 < 双双入对角线 (2+4)/√2。"""
        a = np.array([[0.0, 2.0]])
        b = np.array([[0.0, 4.0]])
        assert wasserstein_1(a, b) == pytest.approx(2.0, rel=1e-12)

    @pytest.mark.unit
    def test_empty_counterpart_is_diagonal_distance(self):
        """空图：所有点搬到对角线，代价 = Σ (death−birth)/√2。"""
        a = np.array([[0.0, 2.0], [1.0, 4.0]])
        expected = (2.0 + 3.0) / math.sqrt(2.0)
        assert wasserstein_1(a, EMPTY) == pytest.approx(expected, rel=1e-12)

    @pytest.mark.unit
    def test_symmetric(self):
        a = np.array([[0.0, 2.0], [1.0, 9.0]])
        b = np.array([[0.3, 2.2]])
        assert wasserstein_1(a, b) == pytest.approx(
            wasserstein_1(b, a), rel=1e-12
        )

    @pytest.mark.unit
    def test_nonnegative_and_finite(self):
        rng = np.random.RandomState(7)
        for _ in range(20):
            a_birth = rng.uniform(0, 10, 5)
            b_birth = rng.uniform(0, 10, 3)
            a = np.stack([a_birth, a_birth + rng.uniform(0, 5, 5)], axis=1)
            b = np.stack([b_birth, b_birth + rng.uniform(0, 5, 3)], axis=1)
            dist = wasserstein_1(a, b)
            assert dist >= 0.0
            assert math.isfinite(dist)


class TestInputValidation:
    @pytest.mark.unit
    def test_wrong_column_count_raises(self):
        bad = np.array([[1.0, 2.0, 3.0]])
        with pytest.raises(ValueError, match="shape"):
            wasserstein_1(bad, EMPTY)

    @pytest.mark.unit
    def test_one_dimensional_raises(self):
        with pytest.raises(ValueError, match="shape"):
            wasserstein_1(np.array([1.0, 2.0]), EMPTY)

    @pytest.mark.unit
    def test_infinite_death_raises(self):
        """W1 在含无穷 bar 的图上无定义——报错，不静默丢点。"""
        bad = np.array([[1.0, np.inf]])
        with pytest.raises(ValueError, match="非有限"):
            wasserstein_1(bad, np.array([[1.0, 2.0]]))

    @pytest.mark.unit
    def test_nan_raises(self):
        bad = np.array([[np.nan, 2.0]])
        with pytest.raises(ValueError, match="非有限"):
            wasserstein_1(bad, EMPTY)
