"""a_geometric_momentum 的「几何动量 over 拓扑特征」测试（521号定位）。

坐实模块 docstring 的核心声明（no-patch-mentality #5）：
1. **全局 EMA 硬约束**（§5）：barcode 用全局 MACD 的 hist 切片，**不**段内重置
   EMA——保留跨笔记忆/0 轴基准。验证：全局切片 hist ≠ 段内重置 hist。这是
   "动量来自 MACD 几何"（521号）的可执行守卫。
2. total_persistence（over-hist 拓扑泛函）≈ MACD 面积（over-hist 积分泛函）的类比；
   背驰 = C 段力度 < A 段。两者都 over 同一 MACD 几何动量（非纯拓扑动量，521号）。
3. agrees_with_area：拓扑泛函背驰与积分泛函背驰的一致性（L2 核心观测量）。

认识论等级：合成数据 = L0/L1（算法管线，零信息增量）；
"拓扑泛函背驰≡积分泛函背驰"经验断言需 700 L2。
"""

from __future__ import annotations

import numpy as np
import pytest

from newchan.a_geometric_momentum import (
    MomentumBarcode,
    MomentumDivergence,
    macd_hist,
    momentum_barcode,
    momentum_divergence,
)


def _divergence_series() -> tuple[list[float], tuple[int, int], tuple[int, int]]:
    """合成背驰场景：A 段急跌（强动量）→ 反弹 → C 段缓跌创新低（弱动量）。

    返回 (prices, A_range, C_range)。A 急 C 缓 + C 创新低 → 应触发底背驰
    （C 段 MACD 绿柱面积 < A 段，缠论第24课）。
    """
    prices: list[float] = []
    # 0-30: 预热 + 上涨 100→130（MACD slow=26 预热）
    for i in range(31):
        prices.append(100 + i)
    # 31-38: 急跌 130→90（强动量，8 根 -40）
    for p in [124, 116, 108, 100, 96, 93, 91, 90]:
        prices.append(p)
    # 39-51: 反弹 90→112（B 段中枢/回拉 0 轴）
    for p in [94, 98, 102, 105, 108, 110, 111, 112, 111, 110, 109, 110, 111]:
        prices.append(p)
    # 52-78: 缓跌 110→85（弱动量，创新低 85<90，27 根 -25）
    c_start = len(prices)
    for p in [109, 108, 107, 105, 104, 103, 102, 100, 99, 98, 97, 96, 95,
              94, 93, 92, 91, 90, 89, 88, 87, 87, 86, 86, 85, 85, 85]:
        prices.append(p)
    c_end = len(prices) - 1
    return prices, (31, 38), (c_start, c_end)


# ====================================================================
# 1. 全局 EMA 硬约束（§5）：全局切片 ≠ 段内重置
# ====================================================================


@pytest.mark.unit
def test_global_macd_slice_differs_from_reset_ema():
    """动量 barcode 用全局 MACD hist 切片 → 保留跨笔 EMA（§5），≠ 段内重置 EMA。

    若两者相等 = 丢掉了 0 轴跨段基准 = 退回 520/§8 否决的退化版本。
    """
    prices, _, c_range = _divergence_series()
    i0, i1 = c_range
    # 全局：在完整序列上算 MACD 再切片
    bc_global = momentum_barcode(prices, i0=i0, i1=i1)
    global_slice = list(bc_global.hist_slice)
    # 段内重置：只在 C 段价格上算 MACD（EMA 从段首重置）
    reset_slice = list(macd_hist(prices[i0 : i1 + 1]))
    assert len(global_slice) == len(reset_slice)
    # 两者必须不同（保留了跨段记忆）——至少段首因预热差异显著
    assert global_slice != pytest.approx(reset_slice)


# ====================================================================
# 2. 力度度量与面积类比
# ====================================================================


@pytest.mark.unit
def test_total_persistence_tracks_area_magnitude():
    """急跌段（强动量）的 total_persistence 与 MACD 面积同向偏大。"""
    prices, a_range, c_range = _divergence_series()
    bc_a = momentum_barcode(prices, i0=a_range[0], i1=a_range[1])
    bc_c = momentum_barcode(prices, i0=c_range[0], i1=c_range[1])
    # A 急跌动量强 → persistence 与面积都更大
    assert bc_a.total_persistence > bc_c.total_persistence
    assert bc_a.macd_area > bc_c.macd_area


@pytest.mark.unit
def test_macd_params_recorded():
    prices, a_range, _ = _divergence_series()
    bc = momentum_barcode(prices, i0=a_range[0], i1=a_range[1], fast=8, slow=21, signal=5)
    assert bc.macd_params == (8, 21, 5)


# ====================================================================
# 3. 动量背驰 + 与面积判据一致性
# ====================================================================


@pytest.mark.unit
def test_momentum_divergence_detects_beichi():
    """A 急 C 缓创新低 → 动量拓扑背驰（C 段力度 < A 段）。"""
    prices, a_range, c_range = _divergence_series()
    div = momentum_divergence(prices, a_range, c_range)
    assert div.force_c < div.force_a
    assert div.ratio < 1.0
    assert div.is_divergent is True


@pytest.mark.unit
def test_momentum_divergence_agrees_with_area_in_clean_case():
    """干净背驰场景：动量拓扑背驰与传统 MACD 面积背驰一致。"""
    prices, a_range, c_range = _divergence_series()
    div = momentum_divergence(prices, a_range, c_range)
    assert div.area_is_divergent is True
    assert div.agrees_with_area is True


@pytest.mark.unit
def test_no_divergence_when_c_stronger():
    """C 段动量强于 A 段（A↔C 互换）→ 不背驰。"""
    prices, a_range, c_range = _divergence_series()
    # 互换：把"缓跌"当 A，"急跌"当 C → C 更强 → 不背驰
    div = momentum_divergence(prices, c_range, a_range)
    assert div.is_divergent is False
    assert div.ratio > 1.0


@pytest.mark.unit
def test_noise_floor_blocks_divergence_on_weak_a():
    """A 段力度低于 noise_floor → 不判背驰（避免对噪声的算术比较，§3 干净前提）。"""
    prices, a_range, c_range = _divergence_series()
    div = momentum_divergence(prices, a_range, c_range, noise_floor=1e9)
    assert div.is_divergent is False


# ====================================================================
# 4. diagram 输出（persim/bottleneck 兼容，模块3 用）
# ====================================================================


@pytest.mark.unit
def test_diagram_shape():
    prices, a_range, _ = _divergence_series()
    bc = momentum_barcode(prices, i0=a_range[0], i1=a_range[1])
    dgm = bc.diagram()
    assert dgm.ndim == 2 and dgm.shape[1] == 2
    assert dgm.shape[0] == len(bc.bars)


@pytest.mark.unit
def test_empty_segment_diagram():
    """空/退化段 → 空 diagram（persim.bottleneck 接受 (0,2)）。"""
    bc = momentum_barcode([100.0] * 30, i0=5, i1=4)  # i1<i0 → 空
    assert bc.n_points == 0
    assert bc.diagram().shape == (0, 2)
    assert bc.total_persistence == 0.0


# ====================================================================
# 5. 不可变性
# ====================================================================


@pytest.mark.unit
def test_momentum_barcode_immutable():
    prices, a_range, _ = _divergence_series()
    bc = momentum_barcode(prices, i0=a_range[0], i1=a_range[1])
    with pytest.raises((AttributeError, TypeError)):
        bc.n_points = 0  # type: ignore[misc]
