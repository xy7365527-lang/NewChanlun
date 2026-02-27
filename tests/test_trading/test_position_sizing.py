"""仓位确定性函数测试。"""

from __future__ import annotations

import math

import pytest
from dataclasses import replace

from newchan.trading.position_sizing import (
    Layer,
    PositionSizer,
    Signal,
    STANDARD_SIGNALS,
    W_CONFIG,
    W_EDGE,
    W_SINGLE,
)


# ── 辅助 ──

def _confirm(signals: tuple[Signal, ...], ids: set[str]) -> list[Signal]:
    """将指定 id 的信号设为 confirmed。"""
    return [
        replace(s, confirmed=(s.id in ids))
        for s in signals
    ]


def _confirm_all(signals: tuple[Signal, ...]) -> list[Signal]:
    return [replace(s, confirmed=True) for s in signals]


# ── Signal 基本测试 ──


class TestSignal:
    def test_create(self) -> None:
        s = Signal(id="s1", layer=Layer.L0, weight=0.15)
        assert s.id == "s1"
        assert s.layer == Layer.L0
        assert s.weight == 0.15
        assert s.confirmed is False

    def test_weight_must_be_positive(self) -> None:
        with pytest.raises(ValueError, match="weight must be > 0"):
            Signal(id="bad", layer=Layer.L0, weight=0.0)
        with pytest.raises(ValueError, match="weight must be > 0"):
            Signal(id="bad", layer=Layer.L0, weight=-0.1)

    def test_immutable(self) -> None:
        s = Signal(id="s1", layer=Layer.L0, weight=0.15)
        with pytest.raises(AttributeError):
            s.confirmed = True  # type: ignore[misc]


# ── STANDARD_SIGNALS 结构测试 ──


class TestStandardSignals:
    def test_count(self) -> None:
        assert len(STANDARD_SIGNALS) == 8

    def test_ids_s1_to_s8(self) -> None:
        ids = [s.id for s in STANDARD_SIGNALS]
        assert ids == [f"s{i}" for i in range(1, 9)]

    def test_weight_ordering(self) -> None:
        """w1 > w2 > w3 >= w4 >= w5 > w6 > w7 > w8（§七.3）。"""
        w = [s.weight for s in STANDARD_SIGNALS]
        assert w[0] > w[1] > w[2]  # w1 > w2 > w3
        assert w[2] >= w[3]        # w3 >= w4
        assert w[3] >= w[4]        # w4 >= w5
        assert w[4] > w[5]         # w5 > w6
        assert w[5] > w[6]         # w6 > w7
        assert w[6] > w[7]         # w7 > w8


# ── PositionSizer 测试 ──


class TestPositionSizer:
    def setup_method(self) -> None:
        self.sizer = PositionSizer()

    # -- w_base --

    def test_w_base_no_confirmed(self) -> None:
        signals = list(STANDARD_SIGNALS)  # 全未确认
        assert self.sizer.w_base(signals) == 0.0

    def test_w_base_with_s1(self) -> None:
        signals = _confirm(STANDARD_SIGNALS, {"s1", "s2", "s8"})
        assert self.sizer.w_base(signals) == W_CONFIG

    def test_w_base_with_s2_no_s1(self) -> None:
        signals = _confirm(STANDARD_SIGNALS, {"s2", "s6", "s7", "s8"})
        assert self.sizer.w_base(signals) == W_EDGE

    def test_w_base_only_s8(self) -> None:
        signals = _confirm(STANDARD_SIGNALS, {"s8"})
        assert self.sizer.w_base(signals) == W_SINGLE

    # -- position: 单调性 --

    def test_monotonicity_adding_signals_increases_position(self) -> None:
        """每增加一个确认信号，仓位严格递增（§七.5）。"""
        ids_sequence = ["s8", "s7", "s6", "s5", "s4", "s3", "s2", "s1"]
        confirmed_ids: set[str] = set()
        prev_pos = 0.0

        for sid in ids_sequence:
            confirmed_ids.add(sid)
            signals = _confirm(STANDARD_SIGNALS, confirmed_ids)
            pos = self.sizer.position(signals)
            assert pos > prev_pos, (
                f"adding {sid}: position {pos} not > {prev_pos}"
            )
            prev_pos = pos

    # -- position: 上界 --

    def test_upper_bound(self) -> None:
        """W <= 1（§七.5）。"""
        signals = _confirm_all(STANDARD_SIGNALS)
        pos = self.sizer.position(signals)
        assert pos <= 1.0

    # -- position: 下界 --

    def test_lower_bound(self) -> None:
        """W >= W_single 只要有任何买点（§七.5）。"""
        signals = _confirm(STANDARD_SIGNALS, {"s8"})
        pos = self.sizer.position(signals)
        assert pos >= W_SINGLE

    # -- position: 满共振 ≈ 0.97 --

    def test_full_resonance(self) -> None:
        """满共振场景：W = 0.5 * prod(1+w_i for all 8 signals)。

        文档 §七.4 声称 ≈0.97，但实际算术结果 ≈0.83：
        0.5 * 1.15 * 1.12 * 1.06 * 1.06 * 1.05 * 1.04 * 1.03 * 1.02 ≈ 0.830
        文档的 0.97 是算术错误。实现忠于公式本身。
        """
        signals = _confirm_all(STANDARD_SIGNALS)
        pos = self.sizer.position(signals)
        # 精确计算: 0.5 * prod([1.15, 1.12, 1.06, 1.06, 1.05, 1.04, 1.03, 1.02])
        expected = 0.5 * math.prod(
            [1.15, 1.12, 1.06, 1.06, 1.05, 1.04, 1.03, 1.02]
        )
        assert abs(pos - expected) < 1e-9, f"expected {expected}, got {pos}"
        assert pos < 1.0  # 上界
        assert pos > 0.5  # 大于 W_base

    # -- position: 文档中的具体场景 --

    def test_config_plus_edge(self) -> None:
        """配置+边：s1, s2, s8 → ~0.66（§七.4）。"""
        signals = _confirm(STANDARD_SIGNALS, {"s1", "s2", "s8"})
        pos = self.sizer.position(signals)
        assert 0.60 <= pos <= 0.72, f"config+edge position = {pos}"

    def test_edge_only(self) -> None:
        """仅边层：s2, s6, s7, s8 → ~0.31（§七.4）。"""
        signals = _confirm(STANDARD_SIGNALS, {"s2", "s6", "s7", "s8"})
        pos = self.sizer.position(signals)
        assert 0.28 <= pos <= 0.35, f"edge only position = {pos}"

    def test_single_target(self) -> None:
        """仅标的：s8 → ~0.08（§七.4）。"""
        signals = _confirm(STANDARD_SIGNALS, {"s8"})
        pos = self.sizer.position(signals)
        assert 0.07 <= pos <= 0.10, f"single target position = {pos}"

    # -- position: 无信号返回0 --

    def test_no_confirmed_returns_zero(self) -> None:
        signals = list(STANDARD_SIGNALS)
        assert self.sizer.position(signals) == 0.0

    # -- validate_weights --

    def test_validate_weights_standard(self) -> None:
        """标准信号集的权重应满足约束。"""
        assert self.sizer.validate_weights(STANDARD_SIGNALS) is True

    def test_validate_weights_empty(self) -> None:
        assert self.sizer.validate_weights([]) is True

    def test_validate_weights_excessive(self) -> None:
        """过大的权重违反约束。"""
        big = [Signal(id="s1", layer=Layer.L0, weight=5.0)]
        assert self.sizer.validate_weights(big) is False

    # -- 层级敏感性 --

    def test_layer_sensitivity(self) -> None:
        """去掉 s1 对仓位的影响远大于去掉 s8（§七.5）。"""
        all_signals = _confirm_all(STANDARD_SIGNALS)
        w_all = self.sizer.position(all_signals)

        no_s1 = _confirm(STANDARD_SIGNALS, {f"s{i}" for i in range(2, 9)})
        w_no_s1 = self.sizer.position(no_s1)

        no_s8 = _confirm(STANDARD_SIGNALS, {f"s{i}" for i in range(1, 8)})
        w_no_s8 = self.sizer.position(no_s8)

        impact_s1 = w_all - w_no_s1
        impact_s8 = w_all - w_no_s8
        assert impact_s1 > impact_s8

    # -- 构造参数校验 --

    def test_invalid_w_config(self) -> None:
        with pytest.raises(ValueError):
            PositionSizer(w_config=0.0)
        with pytest.raises(ValueError):
            PositionSizer(w_config=1.5)
