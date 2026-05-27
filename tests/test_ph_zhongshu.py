"""a_ph_zhongshu 的 PH-中枢检测测试。

坐实模块 docstring 的核心声明（消除声明膨胀，no-patch-mentality #5）：
1. 同层 ≥3 个时间连续、价格重叠的摆动 → 一个中枢，重叠区 [zd, zg] = 缠论 ZD/ZG。
2. merge tree 链式嵌套不阻碍中枢识别（中枢按 persistence 层成组，不按树深度）。
3. 纯趋势（无重叠振荡）→ 无中枢。

认识论等级：合成数据 = L0/L1（验证算法管线，零信息增量）；中枢↔缠论对应需 700 L2。
"""

from __future__ import annotations

import pytest

from newchan.a_online_persistence import MergeBar, OnlineMergeTree
from newchan.a_ph_zhongshu import (
    Zhongshu,
    ZhongshuPolicy,
    build_containment_forest,
    detect_zhongshu,
)


def _mergebars(closes: list[float]) -> tuple[MergeBar, ...]:
    """完整因果序列 → finalize 的 settled MergeBars（含 lo/hi/span/is_global）。"""
    return OnlineMergeTree.from_prices([float(c) for c in closes]).settled_bars


# 合成中枢：价格在 10~19 band 内振荡 3 次（valley 10/11/12，saddle 19/18/17），
# 最后崩到 2。手工推导：bar1[10,19] bar2[11,18] bar3[12,17] 同量级(9/7/5)、
# 时间连续、重叠滑窗收敛 → 重叠区 ZD/ZG = [max(births),min(deaths)] = [12,17]。
ZHONGSHU_SERIES = [20, 10, 18, 11, 17, 12, 19, 5, 8, 2]


# ====================================================================
# 1. 合成中枢识别（核心）
# ====================================================================


@pytest.mark.unit
def test_detect_synthetic_zhongshu():
    """合成振荡 band → 恰好识别 1 个中枢，重叠区 = [12, 17]。"""
    bars = _mergebars(ZHONGSHU_SERIES)
    zs = detect_zhongshu(bars)
    assert len(zs) == 1
    z = zs[0]
    assert z.n_members == 3
    assert z.zd == pytest.approx(12.0)
    assert z.zg == pytest.approx(17.0)
    assert z.zd < z.zg  # 重叠区非空（缠论中枢成立的硬条件）


@pytest.mark.unit
def test_zhongshu_members_same_level_and_overlap():
    """中枢成员两两价格重叠（公共区非空）且 persistence 同量级。"""
    z = detect_zhongshu(_mergebars(ZHONGSHU_SERIES))[0]
    # 公共重叠区 = [max births, min deaths] 非空
    assert max(m.birth_price for m in z.members) < min(m.death_price for m in z.members)
    pers = [m.persistence for m in z.members]
    assert max(pers) / min(pers) < 3.0  # 默认 same_level_ratio


@pytest.mark.unit
def test_envelope_wider_than_overlap():
    """外包络区间（任务字面"父bar价格范围"）⊇ 重叠区 [zd,zg]（缠论 ZD/ZG）。

    两种读法的关系：外包络是中枢所在的整个摆动带，重叠区是次级别走势公共重叠。
    """
    z = detect_zhongshu(_mergebars(ZHONGSHU_SERIES))[0]
    assert z.envelope_low <= z.zd
    assert z.envelope_high >= z.zg


# ====================================================================
# 2. 纯趋势无中枢（边界）
# ====================================================================


@pytest.mark.unit
def test_monotone_trend_no_zhongshu():
    """单调下跌（无重叠振荡）→ 无中枢。"""
    assert detect_zhongshu(_mergebars([20, 18, 15, 11, 8, 5, 2])) == ()


@pytest.mark.unit
def test_single_oscillation_no_zhongshu():
    """单次回调（仅 1 个内部摆动，< min_members）→ 无中枢。"""
    # V 形 + 一个小反弹，不足 3 个同级重叠摆动
    assert detect_zhongshu(_mergebars([20, 10, 14, 6, 2])) == ()


@pytest.mark.unit
def test_two_oscillations_below_min_members():
    """仅 2 次重叠振荡 < min_members(3) → 无中枢。"""
    bars = _mergebars([20, 10, 18, 11, 19, 2])  # 2 个内部摆动
    assert detect_zhongshu(bars) == ()


# ====================================================================
# 3. policy 行为（min_members / require_overlap）
# ====================================================================


@pytest.mark.unit
def test_min_members_relaxed_to_two():
    """放宽 min_members=2 → 2 次重叠振荡也算中枢（policy 塑造中枢数）。"""
    bars = _mergebars([20, 10, 18, 11, 19, 2])
    zs = detect_zhongshu(bars, policy=ZhongshuPolicy(min_members=2))
    assert len(zs) >= 1
    assert zs[0].n_members >= 2


@pytest.mark.unit
def test_require_overlap_false_allows_nonoverlapping():
    """require_overlap=False → 同层时间连续即可成组（不强制公共重叠）。"""
    bars = _mergebars(ZHONGSHU_SERIES)
    strict = detect_zhongshu(bars, policy=ZhongshuPolicy(require_overlap=True))
    loose = detect_zhongshu(bars, policy=ZhongshuPolicy(require_overlap=False))
    # 放宽重叠约束后，成员组不会更少（约束更松）
    assert sum(z.n_members for z in loose) >= sum(z.n_members for z in strict)


# ====================================================================
# 4. 包含森林（外包络）
# ====================================================================


@pytest.mark.unit
def test_literal_three_children_criterion_never_fires():
    """字面判据「父 bar 有 ≥3 直接 children」在 merge tree 上检测到 0 个中枢。

    实测森林 {-1:[0], 0:[1,4], 1:[2], 2:[3]}：振荡 band 内部 bar[10,19]⊃[11,18]⊃
    [12,17] 是链式（各 1 child），全局 bar 仅分叉到 band 入口 + 尾部 dip（2 child）。
    **无任何节点有 ≥3 直接 children** → 字面判据失效（§7.3 merge tree 不平衡），
    这正是必须改用「同层重叠成组」重构的根因（模块 docstring 的定义澄清）。
    """
    bars = _mergebars(ZHONGSHU_SERIES)
    forest = build_containment_forest(bars)
    max_direct_children = max(len(children) for children in forest.values())
    assert max_direct_children < 3  # 字面"≥3 children"永不成立
    # 但同层重叠成组能正确识别该中枢（对照）
    assert len(detect_zhongshu(bars)) == 1


# ====================================================================
# 5. 中枢级别（persistence 量级映射）
# ====================================================================


@pytest.mark.unit
def test_zhongshu_has_period_label():
    """中枢携带级别名（从 persistence 量级 / 外包络 span 映射）。"""
    z = detect_zhongshu(_mergebars(ZHONGSHU_SERIES))[0]
    assert isinstance(z.period_label, str) and z.period_label
    assert z.level_persistence > 0


@pytest.mark.unit
def test_noise_floor_filters_small_oscillations():
    """高 noise_floor 滤掉小摆动 → 中枢消失（级别谱横切，§4）。"""
    bars = _mergebars(ZHONGSHU_SERIES)
    # band 振荡 persistence 5~9，noise_floor=10 应滤光
    assert detect_zhongshu(bars, noise_floor=10.0) == ()


# ====================================================================
# 6. 不可变性
# ====================================================================


@pytest.mark.unit
def test_zhongshu_immutable():
    z = detect_zhongshu(_mergebars(ZHONGSHU_SERIES))[0]
    with pytest.raises((AttributeError, TypeError)):
        z.zd = 0.0  # type: ignore[misc]


@pytest.mark.unit
def test_zhongshu_span_and_width():
    z = detect_zhongshu(_mergebars(ZHONGSHU_SERIES))[0]
    assert z.span == z.hi - z.lo + 1
    assert z.width == pytest.approx(z.zg - z.zd)
