"""a_path_persistence 的 L0 算法锁定测试。

本模块是实验性证伪通道（见 docstring + persistence_theory.md §8），其经验结论
已被 L2 否决。这里只锁定 **L0 纯算法行为**（点云构造、退化处理、Gemini 论断3 的
代数预言），不验证经验断言。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_path_persistence import (
    max_step_distance,
    path_persistence,
    path_point_cloud,
    scale_estimate,
)


@pytest.mark.unit
def test_scale_estimate_methods():
    vals = [1.0, 2.0, 3.0, 4.0, 5.0]
    assert scale_estimate(vals, "std") == pytest.approx(math.sqrt(2.0))
    assert scale_estimate(vals, "range") == pytest.approx(4.0 / math.sqrt(12.0))
    # mad: median=3, abs devs sorted [0,1,1,2,2], median=1 → 1.4826
    assert scale_estimate(vals, "mad") == pytest.approx(1.4826)
    assert scale_estimate([7.0], "std") == 0.0  # n<2 退化


@pytest.mark.unit
def test_point_cloud_normalization_and_degenerate():
    cloud, st, sp = path_point_cloud([10.0, 12.0, 11.0, 13.0], normalization="std")
    assert len(cloud) == 4
    assert st > 0 and sp > 0
    # 归一化后每点 = (t/σ_t, p/σ_p)
    assert cloud[0] == pytest.approx([0.0 / st, 10.0 / sp])
    # 常数价格序列 → σ_p=0 → 联合度量无法定义
    with pytest.raises(ValueError):
        path_point_cloud([5.0, 5.0, 5.0, 5.0])


@pytest.mark.unit
def test_monotonic_leg_has_empty_h1():
    """Gemini 论断3 前半：单调笔路径空间 H1 恒空（无 loop）。"""
    monotonic = [float(100 - 3 * i) for i in range(15)]  # 严格单调下跌
    res = path_persistence(monotonic, normalization="std", maxdim=1)
    assert res.total_persistence(1) == 0.0
    assert len(res.by_dimension(1)) == 0


@pytest.mark.unit
def test_h0max_equals_max_step_for_monotonic():
    """Gemini 论断3 后半：单调路径上 H0 主导 persistence = 相邻点最大步距。

    严格单调序列的 Rips H0 = 单连接树，最大有限 persistence = 最长相邻边。
    """
    monotonic = [100.0, 95.0, 88.0, 84.0, 70.0, 65.0]  # 步长不均
    res = path_persistence(monotonic, normalization="std", maxdim=0)
    msd = max_step_distance(monotonic, normalization="std")
    # ripser 内部用 float32，diagram 精度 ~1e-7；容差按 float32 而非 float64
    assert res.max_persistence(0) == pytest.approx(msd, rel=1e-5)


@pytest.mark.unit
def test_result_is_immutable():
    res = path_persistence([10.0, 12.0, 9.0, 14.0, 8.0], normalization="std")
    with pytest.raises((AttributeError, TypeError)):
        res.n_points = 99  # frozen dataclass
    if res.bars:
        with pytest.raises((AttributeError, TypeError)):
            res.bars[0].persistence = 0.0


@pytest.mark.unit
def test_normalization_changes_result():
    """Gemini 论断2：不同归一化给出不同 persistence（自由参数）。"""
    prices = [100.0, 80.0, 95.0, 70.0, 90.0, 60.0, 85.0]
    std_tot = path_persistence(prices, normalization="std").total_persistence(0)
    mad_tot = path_persistence(prices, normalization="mad").total_persistence(0)
    # 不同 σ → 不同联合距离 → 不同总持续度（否则自由参数论断在此数据上不成立）
    assert std_tot != pytest.approx(mad_tot)


@pytest.mark.unit
def test_explicit_sigma_overrides_scale_estimate():
    """显式 σ 覆盖 scale_estimate：点云直接用传入的全局尺度归一化。"""
    prices = [10.0, 12.0, 11.0, 13.0]
    cloud, st, sp = path_point_cloud(prices, sigma_t=2.0, sigma_p=5.0)
    assert st == 2.0 and sp == 5.0
    assert cloud[0] == pytest.approx([0.0 / 2.0, 10.0 / 5.0])
    assert cloud[3] == pytest.approx([3.0 / 2.0, 13.0 / 5.0])
    # 结果对象暴露实际使用的全局尺度（审计自由参数）
    res = path_persistence(prices, sigma_t=2.0, sigma_p=5.0, maxdim=0)
    assert res.sigma_t == 2.0 and res.sigma_p == 5.0


@pytest.mark.unit
def test_per_segment_normalization_erases_amplitude():
    """各段自归一化抹除幅度——段间背驰对比因此失效（实验脚本的张力锚点）。

    两段同形状（线性下跌 6 点）但幅度差 10 倍。各段用自身 std 归一化时，
    σ_p ∝ 自身幅度 → 归一化后几何全等 → H0 总持续度相等 → 大跌与小跌不可分。
    """
    big = [100.0, 80.0, 60.0, 40.0, 20.0, 0.0]      # 跌 100
    small = [10.0, 8.0, 6.0, 4.0, 2.0, 0.0]          # 跌 10（同形状）
    self_big = path_persistence(big, normalization="std", maxdim=0).total_persistence(0)
    self_small = path_persistence(small, normalization="std", maxdim=0).total_persistence(0)
    assert self_big == pytest.approx(self_small, rel=1e-4)  # 幅度被抹除（背驰对比失效）


@pytest.mark.unit
def test_global_sigma_recovers_amplitude_only_when_time_axis_collapsed():
    """全局 σ 能恢复幅度，但仅当 σ_t 足够大（时间轴塌缩、价格轴主导）时。

    这是 σ_p/σ_t 比值权衡的 L0 锚点：σ_t→∞ 时退化为 1D 价格空间（≈ time-blind
    sublevel H0），此时幅度完全保留；σ_t≈Δt 时时间轴主导，幅度被时间冲淡。
    → 比值同时控制"幅度保真"与"时间信息"，无法两者兼得（Gemini 论断2 的深层形式）。
    """
    big = [100.0, 80.0, 60.0, 40.0, 20.0, 0.0]
    small = [10.0, 8.0, 6.0, 4.0, 2.0, 0.0]
    g_sigma_p = 30.0  # 公共价格尺子
    # σ_t 巨大 → 时间轴塌缩 → 价格主导 → 幅度按 10:1 保留
    big_collapsed = path_persistence(
        big, sigma_p=g_sigma_p, sigma_t=1e6, maxdim=0
    ).total_persistence(0)
    small_collapsed = path_persistence(
        small, sigma_p=g_sigma_p, sigma_t=1e6, maxdim=0
    ).total_persistence(0)
    assert big_collapsed > small_collapsed * 5  # 幅度保留
    # σ_t≈1（时间与价格同量级）→ 时间主导 → 幅度被冲淡（差距远小于 10×）
    big_mixed = path_persistence(
        big, sigma_p=g_sigma_p, sigma_t=1.0, maxdim=0
    ).total_persistence(0)
    small_mixed = path_persistence(
        small, sigma_p=g_sigma_p, sigma_t=1.0, maxdim=0
    ).total_persistence(0)
    assert big_mixed < small_mixed * 2  # 幅度被时间冲淡，几乎不可分
