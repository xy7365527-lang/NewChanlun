"""a_zhongshu_force 的「中枢基线偏离积分」测试（无参数力度度量）。

坐实模块 docstring 的核心声明（no-patch-mentality #5）：
1. 偏离积分公式正确：force=Σ|close−M|，signed=Σ(close−M)，normalized=force/span。
2. **时间外延性（§5 决定性维度）**：相同幅度下，缓跌（多 bar）的 force > 急跌（少 bar），
   而 force_normalized 反之——这正是 force（MACD 面积类比）与 H0（端点振幅）的区别。
3. 背驰判据：C 段力度 < A 段 = 背驰；field 切换改变行为；noise_floor 守卫。
4. 无参数性 + 不可变性。

认识论等级：合成数据 = L0/L1（算法管线，零信息增量）；
"偏离积分≡MACD 面积背驰 / 能区分急跌缓跌"经验断言需 700 L2（见 scripts/tencent_zhongshu_force.py）。
"""

from __future__ import annotations

import pytest

from newchan.a_zhongshu_force import (
    BaselineDivergence,
    ZhongshuForce,
    baseline_deviation_force,
    compare_force,
    zhongshu_midprice,
)


# ====================================================================
# 1. 中枢中间价
# ====================================================================


@pytest.mark.unit
def test_midprice_is_arithmetic_mean():
    assert zhongshu_midprice(10.0, 20.0) == 15.0
    assert zhongshu_midprice(12.0, 17.0) == 14.5


@pytest.mark.unit
def test_midprice_degenerate_width():
    """ZD==ZG（重叠区退化为一点）→ M 即该点。"""
    assert zhongshu_midprice(15.0, 15.0) == 15.0


# ====================================================================
# 2. 偏离积分公式
# ====================================================================


@pytest.mark.unit
def test_force_basic_formula():
    """force=Σ|close−M|，signed=Σ(close−M)，normalized=force/span。"""
    # M=10；价格 [13,7,12] → 偏离 [3,-3,2] → |·|=[3,3,2]
    f = baseline_deviation_force([13.0, 7.0, 12.0], 10.0)
    assert f.force == pytest.approx(8.0)
    assert f.force_signed == pytest.approx(2.0)  # 3-3+2
    assert f.span == 3
    assert f.force_normalized == pytest.approx(8.0 / 3.0)
    assert f.zhongshu_midprice == 10.0


@pytest.mark.unit
def test_force_signed_sign_distinguishes_direction():
    """整体在 M 之下 → signed < 0（空头偏离）；整体在 M 之上 → signed > 0。"""
    below = baseline_deviation_force([8.0, 7.0, 9.0], 10.0)
    above = baseline_deviation_force([12.0, 13.0, 11.0], 10.0)
    assert below.force_signed < 0
    assert above.force_signed > 0
    # |偏离量级| 相同时 force 相同（force 不分方向）
    assert below.force == pytest.approx(above.force)


@pytest.mark.unit
def test_force_slice_range():
    """i0/i1 闭区间切片，只对段内求和。"""
    prices = [10.0, 13.0, 7.0, 12.0, 10.0]  # 整段 M=10
    f = baseline_deviation_force(prices, 10.0, i0=1, i1=3)  # [13,7,12]
    assert f.span == 3
    assert f.force == pytest.approx(8.0)


@pytest.mark.unit
def test_force_empty_segment():
    """i1 < i0 → 空段 → 全 0（但保留 midprice）。"""
    f = baseline_deviation_force([10.0, 11.0, 12.0], 10.0, i0=2, i1=1)
    assert f.span == 0
    assert f.force == 0.0
    assert f.force_normalized == 0.0
    assert f.zhongshu_midprice == 10.0


@pytest.mark.unit
def test_force_empty_prices():
    f = baseline_deviation_force([], 5.0)
    assert f.span == 0 and f.force == 0.0


# ====================================================================
# 3. 时间外延性 —— §5 决定性维度（急跌 vs 缓跌）
# ====================================================================


@pytest.mark.unit
def test_time_extensive_force_vs_intensive_normalized():
    """同基线、相同偏离深度，缓跌（多 bar）force 更大，急跌 normalized 更大。

    §5：急跌#33 与缓跌#37 幅度几乎相同，MACD 面积差 20 倍（缓跌大），H0 几乎相同。
    本测试坐实：force（时间外延，MACD 类比）随 bar 数增长 → 缓跌 > 急跌；
    force_normalized（每 bar 平均，H0/速度类比）剔除 bar 数 → 急跌 ≈ 缓跌每 bar 偏离。
    """
    M = 100.0
    # 急跌：4 根，深偏离（每根 -10）
    sharp = baseline_deviation_force([90.0, 90.0, 90.0, 90.0], M)
    # 缓跌：12 根，同样深偏离（每根 -10），仅 bar 数 3 倍
    gradual = baseline_deviation_force([90.0] * 12, M)
    # force（时间外延）：缓跌 = 3×急跌
    assert gradual.force == pytest.approx(3.0 * sharp.force)
    assert gradual.force > sharp.force
    # normalized（时间内涵）：相同（每 bar 偏离都是 10）
    assert gradual.force_normalized == pytest.approx(sharp.force_normalized)


@pytest.mark.unit
def test_force_distinguishes_what_h0_cannot():
    """相同价格幅度、不同耗时 → force 不同（H0 端点度量会判为相同）。

    两段都从 M 下方 10 单位的深度，但一段 2 bar、一段 8 bar。
    端点振幅（≈H0）相同（都是深度 10），但 force 积分 4 倍——捕获了 H0 丢的时间维度。
    """
    M = 100.0
    short_leg = baseline_deviation_force([90.0, 90.0], M)
    long_leg = baseline_deviation_force([90.0] * 8, M)
    # H0 端点度量会判相同（同深度）；force 区分（积分随时长增长）
    assert long_leg.force == pytest.approx(4.0 * short_leg.force)


# ====================================================================
# 4. 背驰判定
# ====================================================================


@pytest.mark.unit
def test_divergence_default_field_is_normalized():
    """默认背驰判据 = force_normalized（523号编排者裁定：第24课面积=动量强度）。

    A 段每 bar 偏离 10（深）> C 段每 bar 偏离 3（浅）→ C 弱 → 背驰。
    """
    M = 100.0
    fa = baseline_deviation_force([90.0] * 10, M)  # norm = 10
    fc = baseline_deviation_force([97.0] * 6, M)   # norm = 3
    div = compare_force(fa, fc)
    assert div.field == "force_normalized"  # 523号：默认字段
    assert div.is_divergent is True
    assert div.ratio < 1.0


@pytest.mark.unit
def test_no_divergence_when_c_stronger():
    M = 100.0
    fa = baseline_deviation_force([97.0] * 6, M)   # A 弱
    fc = baseline_deviation_force([90.0] * 10, M)  # C 强
    div = compare_force(fa, fc)
    assert div.is_divergent is False
    assert div.ratio > 1.0


@pytest.mark.unit
def test_divergence_field_switch_changes_verdict():
    """field 切换可翻转判定：急跌 A vs 缓跌 C。

    A 急跌（深而短）、C 缓跌（浅但长到积分更大）：
      - field='force'（时间外延）：C 积分 > A → 不背驰（C 看似更强）。
      - field='force_normalized'（速度）：A 每 bar 偏离更大 → C 弱 → 背驰。
    坐实"判据字段是缠论建模决策"——同一对段，不同字段给不同背驰结论。
    """
    M = 100.0
    a_sharp = baseline_deviation_force([85.0] * 4, M)    # 深 15 × 4 = force 60，norm 15
    c_gradual = baseline_deviation_force([95.0] * 20, M)  # 浅 5 × 20 = force 100，norm 5
    div_force = compare_force(a_sharp, c_gradual, field="force")
    div_norm = compare_force(a_sharp, c_gradual, field="force_normalized")
    assert div_force.is_divergent is False  # C force 更大
    assert div_norm.is_divergent is True    # A 速度更大 → C 弱


@pytest.mark.unit
def test_signed_field_compares_magnitude():
    """field='force_signed'：空头偏离 value 为负，按量级比较。"""
    M = 100.0
    fa = baseline_deviation_force([88.0] * 8, M)  # signed = -96（强空头）
    fc = baseline_deviation_force([96.0] * 5, M)  # signed = -20（弱空头）
    div = compare_force(fa, fc, field="force_signed")
    assert div.value_a < 0 and div.value_c < 0
    assert div.is_divergent is True  # |C| < |A|


@pytest.mark.unit
def test_noise_floor_blocks_divergence():
    """A 段力度低于 noise_floor → 不判背驰。"""
    M = 100.0
    fa = baseline_deviation_force([90.0] * 10, M)
    fc = baseline_deviation_force([97.0] * 6, M)
    div = compare_force(fa, fc, noise_floor=1e9)
    assert div.is_divergent is False


@pytest.mark.unit
def test_zero_a_force_ratio_inf():
    """A 段力度为 0（价格恰在 M 上）→ ratio=inf，不背驰。"""
    M = 100.0
    fa = baseline_deviation_force([100.0] * 5, M)  # force=0
    fc = baseline_deviation_force([90.0] * 5, M)
    div = compare_force(fa, fc)
    assert div.ratio == float("inf")
    assert div.is_divergent is False


# ====================================================================
# 5. 无参数性 + 不可变性
# ====================================================================


@pytest.mark.unit
def test_force_immutable():
    f = baseline_deviation_force([90.0, 95.0], 100.0)
    with pytest.raises((AttributeError, TypeError)):
        f.force = 0.0  # type: ignore[misc]


@pytest.mark.unit
def test_divergence_immutable():
    M = 100.0
    div = compare_force(
        baseline_deviation_force([90.0] * 5, M),
        baseline_deviation_force([95.0] * 5, M),
    )
    with pytest.raises((AttributeError, TypeError)):
        div.is_divergent = True  # type: ignore[misc]


@pytest.mark.unit
def test_no_parameters_in_signature():
    """无参数性守卫：force 计算签名不含任何 MACD 式可调参数（fast/slow/signal）。

    度量对 MACD 参数 100% 鲁棒（L0，同义反复）——因为它根本不接收这些参数。
    """
    import inspect

    sig = inspect.signature(baseline_deviation_force)
    param_names = set(sig.parameters)
    forbidden = {"fast", "slow", "signal", "window", "alpha", "span_ema"}
    assert param_names & forbidden == set()
