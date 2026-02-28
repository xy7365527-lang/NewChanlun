"""方向性力度指标——不依赖幅度的走势强度度量。

239号谱系：方向性力度 ∉ ker(D)，替代振幅力度 ∈ ker(D)。
237号诊断：高级别背驰判断使用 _amplitude_force（价格振幅），∈ ker(D)。
235号证明：缠论离散化系统性地吸收幅度信息。

三种方向性力度方法：
  A. directional_persistence — 同向线段持续性（纯方向信息，∉ ker(D)）
  B. stroke_density_force — 笔密度（D1 输出统计性质，∉ ker(D)）
  C. zhongshu_drift_force — 中枢偏移（D3 保留的位置关系，∉ ker(D)）

概念溯源标签
-----------
- 方向性力度 [新缠论:239号]
- ker(D) 与力度 [新缠论:235号/237号]
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Literal, Protocol, Sequence

logger = logging.getLogger(__name__)


# ── 协议：避免直接导入具体类型 ──────────────────────────────────


class SegmentLike(Protocol):
    """线段的最小接口。"""

    @property
    def direction(self) -> str: ...

    @property
    def high(self) -> float: ...

    @property
    def low(self) -> float: ...

    @property
    def i0(self) -> int: ...

    @property
    def i1(self) -> int: ...


class StrokeLike(Protocol):
    """笔的最小接口。"""

    @property
    def i0(self) -> int: ...

    @property
    def i1(self) -> int: ...


class ZhongshuLike(Protocol):
    """中枢的最小接口。"""

    @property
    def zd(self) -> float: ...

    @property
    def zg(self) -> float: ...

    @property
    def dd(self) -> float: ...

    @property
    def gg(self) -> float: ...

    @property
    def seg_start(self) -> int: ...

    @property
    def seg_end(self) -> int: ...

    @property
    def settled(self) -> bool: ...


# ── 结果类型 ──────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class DirectionalDivergence:
    """方向性力度背驰判定结果。

    Attributes
    ----------
    force_a : float
        A 段方向性力度。
    force_c : float
        C 段方向性力度。
    is_divergent : bool
        是否背驰（力度衰减）。
    method : str
        使用的力度方法名。
    comparison_with_amplitude : dict[str, float]
        与振幅力度的对比数据。
    """

    force_a: float
    force_c: float
    is_divergent: bool
    method: str
    comparison_with_amplitude: dict[str, float]


@dataclass(frozen=True, slots=True)
class ForceComparison:
    """方向性力度 vs 振幅力度的单次对比结果。

    Attributes
    ----------
    amplitude_force_a : float
        A 段振幅力度。
    amplitude_force_c : float
        C 段振幅力度。
    amplitude_divergent : bool
        振幅力度是否判定背驰。
    directional_force_a : float
        A 段方向性力度。
    directional_force_c : float
        C 段方向性力度。
    directional_divergent : bool
        方向性力度是否判定背驰。
    method : str
        方向性力度方法名。
    agreement : bool
        两种方法是否一致。
    """

    amplitude_force_a: float
    amplitude_force_c: float
    amplitude_divergent: bool
    directional_force_a: float
    directional_force_c: float
    directional_divergent: bool
    method: str
    agreement: bool


# ── 方法 A：方向持续性力度 ──────────────────────────────────────


def directional_persistence(
    segments: Sequence[SegmentLike],
    seg_start: int,
    seg_end: int,
    direction: str,
) -> float:
    """计算走势区间内同向线段的持续性。

    方向持续性 = 同向线段数 / 总线段数。
    纯方向信息（∉ ker(D)）——只看方向，不看幅度。

    D 的每一层保留方向编码：
    - D1（笔）：编码方向（up/down），丢弃幅度
    - D2（线段）：聚合方向，滤除短程噪声
    - D3（中枢→走势）：方向持续性穿透中枢吸收

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    seg_start, seg_end : int
        线段索引范围 [seg_start, seg_end]（闭区间）。
    direction : str
        走势方向（"up" / "down"）。

    Returns
    -------
    float
        方向持续性 ∈ [0, 1]。1 = 所有线段同向，0 = 无同向线段。

    概念溯源: [新缠论:239号] — 方向性力度方法A
    """
    if seg_start > seg_end or seg_start < 0 or seg_end >= len(segments):
        return 0.0

    total = seg_end - seg_start + 1
    if total <= 0:
        return 0.0

    same_dir_count = sum(
        1 for i in range(seg_start, seg_end + 1)
        if segments[i].direction == direction
    )

    return same_dir_count / total


def _max_consecutive_same_direction(
    segments: Sequence[SegmentLike],
    seg_start: int,
    seg_end: int,
    direction: str,
) -> int:
    """计算最长连续同向线段数。"""
    if seg_start > seg_end or seg_start < 0 or seg_end >= len(segments):
        return 0

    max_run = 0
    current_run = 0
    for i in range(seg_start, seg_end + 1):
        if segments[i].direction == direction:
            current_run += 1
            if current_run > max_run:
                max_run = current_run
        else:
            current_run = 0
    return max_run


def directional_persistence_weighted(
    segments: Sequence[SegmentLike],
    seg_start: int,
    seg_end: int,
    direction: str,
) -> float:
    """加权方向持续性 = 比例 * 最长连续同向 / 总段数。

    结合比例信息和连续性信息，更敏感地捕捉方向衰减。

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    seg_start, seg_end : int
        线段索引范围。
    direction : str
        走势方向。

    Returns
    -------
    float
        加权方向持续性 ∈ [0, 1]。

    概念溯源: [新缠论:239号] — 方向性力度方法A变体
    """
    ratio = directional_persistence(segments, seg_start, seg_end, direction)
    total = seg_end - seg_start + 1
    if total <= 0:
        return 0.0

    max_run = _max_consecutive_same_direction(
        segments, seg_start, seg_end, direction,
    )

    return ratio * (max_run / total)


# ── 方法 B：笔密度力度 ──────────────────────────────────────────


def stroke_density_force(
    strokes: Sequence[StrokeLike],
    bar_start: int,
    bar_end: int,
) -> float:
    """计算走势区间内的笔密度。

    笔密度 = 区间内笔数量 / 区间 bar 跨度。
    笔越多 → 方向反转越频繁 → 走势力度越弱。
    不依赖幅度，依赖 D1 输出的结构密度。

    D1 将 bar 编码为笔（方向编码）。笔数量是 D1 输出的统计性质：
    - 趋势性走势：笔少（方向一致，D1 产出少量长笔）
    - 震荡走势：笔多（方向频繁翻转，D1 产出大量短笔）

    **力度 = 1 / density** — 密度越高力度越低。

    Parameters
    ----------
    strokes : Sequence[StrokeLike]
        笔序列。
    bar_start, bar_end : int
        merged bar 索引范围 [bar_start, bar_end]（闭区间）。

    Returns
    -------
    float
        力度值（≥ 0）。高值 = 笔密度低 = 方向持续性强。
        bar 跨度为 0 时返回 0.0。

    概念溯源: [新缠论:239号] — 方向性力度方法B
    """
    if bar_start > bar_end:
        return 0.0

    bar_span = bar_end - bar_start + 1
    if bar_span <= 0:
        return 0.0

    # 统计区间内的笔数量
    stroke_count = 0
    for stroke in strokes:
        # 笔完全或部分落入区间
        if stroke.i1 >= bar_start and stroke.i0 <= bar_end:
            stroke_count += 1

    if stroke_count == 0:
        return float(bar_span)  # 无笔 = 最高力度（极端情况）

    density = stroke_count / bar_span
    return 1.0 / density


def stroke_density_from_segments(
    segments: Sequence[SegmentLike],
    seg_start: int,
    seg_end: int,
) -> float:
    """从线段序列估算笔密度力度（当笔数据不可用时）。

    线段至少包含 3 笔。线段数量是笔密度的上界估计。
    笔密度 ≈ 3 * segment_count / bar_span（下界估计）。

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    seg_start, seg_end : int
        线段索引范围。

    Returns
    -------
    float
        力度值。
    """
    if seg_start > seg_end or seg_start < 0 or seg_end >= len(segments):
        return 0.0

    i0 = segments[seg_start].i0
    i1 = segments[seg_end].i1
    bar_span = i1 - i0 + 1
    if bar_span <= 0:
        return 0.0

    seg_count = seg_end - seg_start + 1
    estimated_stroke_count = seg_count * 3  # 每段至少 3 笔
    density = estimated_stroke_count / bar_span
    if density <= 0:
        return float(bar_span)
    return 1.0 / density


# ── 方法 C：中枢偏移力度 ──────────────────────────────────────


def zhongshu_drift_force(
    zhongshus: Sequence[ZhongshuLike],
    zs_indices: Sequence[int],
    direction: str,
) -> float:
    """计算连续中枢的方向偏移强度。

    偏移 = 连续中枢的 [ZD, ZG] 区间沿走势方向的位移。
    使用中枢位置关系（D3 保留的信息），不使用绝对价格幅度。

    D3 保留的信息：中枢间的相对位置关系（后DD > 前GG = 上涨方向）。
    ker(D) 中的信息：中枢内振荡的绝对幅度。

    度量方式：
    - 上涨方向：midpoint 递增 → 正偏移 → 力度强
    - 下跌方向：midpoint 递减 → 正偏移 → 力度强

    力度 = sum of normalized shifts。

    Parameters
    ----------
    zhongshus : Sequence[ZhongshuLike]
        中枢序列。
    zs_indices : Sequence[int]
        参与比较的中枢索引列表（至少 2 个）。
    direction : str
        走势方向（"up" / "down"）。

    Returns
    -------
    float
        中枢偏移力度（≥ 0）。高值 = 中枢沿方向偏移强。
        中枢不足 2 个时返回 0.0。

    概念溯源: [新缠论:239号] — 方向性力度方法C
    """
    if len(zs_indices) < 2:
        return 0.0

    valid_indices = [
        idx for idx in zs_indices
        if 0 <= idx < len(zhongshus)
    ]
    if len(valid_indices) < 2:
        return 0.0

    total_drift = 0.0
    drift_count = 0

    for k in range(1, len(valid_indices)):
        prev_zs = zhongshus[valid_indices[k - 1]]
        curr_zs = zhongshus[valid_indices[k]]

        prev_mid = (prev_zs.zd + prev_zs.zg) / 2
        curr_mid = (curr_zs.zd + curr_zs.zg) / 2

        # 归一化偏移：除以前中枢宽度避免幅度依赖
        prev_width = prev_zs.zg - prev_zs.zd
        if prev_width <= 0:
            continue

        shift = (curr_mid - prev_mid) / prev_width

        # 方向一致性判定
        if direction == "up" and shift > 0:
            total_drift += shift
        elif direction == "down" and shift < 0:
            total_drift += abs(shift)

        drift_count += 1

    if drift_count == 0:
        return 0.0

    return total_drift / drift_count


# ── 复合力度 ──────────────────────────────────────────────────


def composite_directional_force(
    segments: Sequence[SegmentLike],
    seg_start: int,
    seg_end: int,
    direction: str,
    zhongshus: Sequence[ZhongshuLike] | None = None,
    zs_indices: Sequence[int] | None = None,
) -> float:
    """复合方向性力度——结合多种方向性信号。

    当中枢数据可用时，组合三种信号：
    - 方向持续性（权重 0.4）
    - 笔密度估计（权重 0.3）
    - 中枢偏移（权重 0.3）

    无中枢数据时，组合两种信号：
    - 方向持续性（权重 0.5）
    - 笔密度估计（权重 0.5）

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    seg_start, seg_end : int
        线段索引范围。
    direction : str
        走势方向。
    zhongshus : Sequence[ZhongshuLike] | None
        中枢序列（可选）。
    zs_indices : Sequence[int] | None
        中枢索引列表（可选）。

    Returns
    -------
    float
        复合方向性力度。
    """
    persistence = directional_persistence_weighted(
        segments, seg_start, seg_end, direction,
    )
    density = stroke_density_from_segments(segments, seg_start, seg_end)

    if (
        zhongshus is not None
        and zs_indices is not None
        and len(zs_indices) >= 2
    ):
        drift = zhongshu_drift_force(zhongshus, zs_indices, direction)
        return 0.4 * persistence + 0.3 * density + 0.3 * drift

    return 0.5 * persistence + 0.5 * density


# ── 振幅力度（参照基线，仅用于对比）────────────────────────────


def _amplitude_force(
    segments: Sequence[SegmentLike],
    seg_start: int,
    seg_end: int,
) -> float:
    """振幅力度——price amplitude（∈ ker(D)，仅用于对比）。

    这是 a_divergence_v1.py 中 MACD 不可用时的 fallback 力度。
    复制于此仅作为方向性力度的对比基线。

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    seg_start, seg_end : int
        线段索引范围。

    Returns
    -------
    float
        振幅力度 = (high - low) * duration。
    """
    if seg_start > seg_end or seg_start < 0 or seg_end >= len(segments):
        return 0.0

    high = max(segments[k].high for k in range(seg_start, seg_end + 1))
    low = min(segments[k].low for k in range(seg_start, seg_end + 1))
    i0 = segments[seg_start].i0
    i1 = segments[seg_end].i1
    duration = max(1, i1 - i0)
    return (high - low) * duration


# ── 力度对比 ──────────────────────────────────────────────────


def compare_force_methods(
    segments: Sequence[SegmentLike],
    a_start: int,
    a_end: int,
    c_start: int,
    c_end: int,
    direction: str,
    method: Literal["persistence", "density", "composite"] = "composite",
    zhongshus: Sequence[ZhongshuLike] | None = None,
    zs_indices_a: Sequence[int] | None = None,
    zs_indices_c: Sequence[int] | None = None,
) -> ForceComparison:
    """对比方向性力度 vs 振幅力度在 A/C 段的判定结果。

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    a_start, a_end : int
        A 段索引范围。
    c_start, c_end : int
        C 段索引范围。
    direction : str
        走势方向。
    method : str
        方向性力度方法。
    zhongshus : Sequence[ZhongshuLike] | None
        中枢序列（method="composite" 时可选）。
    zs_indices_a, zs_indices_c : Sequence[int] | None
        A/C 段对应的中枢索引。

    Returns
    -------
    ForceComparison
    """
    amp_a = _amplitude_force(segments, a_start, a_end)
    amp_c = _amplitude_force(segments, c_start, c_end)
    amp_div = amp_a > 0 and amp_c < amp_a

    if method == "persistence":
        dir_a = directional_persistence_weighted(
            segments, a_start, a_end, direction,
        )
        dir_c = directional_persistence_weighted(
            segments, c_start, c_end, direction,
        )
    elif method == "density":
        dir_a = stroke_density_from_segments(segments, a_start, a_end)
        dir_c = stroke_density_from_segments(segments, c_start, c_end)
    else:
        dir_a = composite_directional_force(
            segments, a_start, a_end, direction,
            zhongshus, zs_indices_a,
        )
        dir_c = composite_directional_force(
            segments, c_start, c_end, direction,
            zhongshus, zs_indices_c,
        )

    dir_div = dir_a > 0 and dir_c < dir_a

    return ForceComparison(
        amplitude_force_a=amp_a,
        amplitude_force_c=amp_c,
        amplitude_divergent=amp_div,
        directional_force_a=dir_a,
        directional_force_c=dir_c,
        directional_divergent=dir_div,
        method=method,
        agreement=amp_div == dir_div,
    )


def detect_directional_divergence(
    segments: Sequence[SegmentLike],
    a_start: int,
    a_end: int,
    c_start: int,
    c_end: int,
    direction: str,
    method: Literal["persistence", "density", "composite"] = "composite",
    zhongshus: Sequence[ZhongshuLike] | None = None,
    zs_indices_a: Sequence[int] | None = None,
    zs_indices_c: Sequence[int] | None = None,
) -> DirectionalDivergence:
    """使用方向性力度检测背驰。

    Parameters
    ----------
    segments : Sequence[SegmentLike]
        线段序列。
    a_start, a_end : int
        A 段索引范围。
    c_start, c_end : int
        C 段索引范围。
    direction : str
        走势方向。
    method : str
        方向性力度方法。
    zhongshus : Sequence[ZhongshuLike] | None
        中枢序列。
    zs_indices_a, zs_indices_c : Sequence[int] | None
        A/C 段的中枢索引。

    Returns
    -------
    DirectionalDivergence
    """
    comparison = compare_force_methods(
        segments, a_start, a_end, c_start, c_end, direction,
        method, zhongshus, zs_indices_a, zs_indices_c,
    )

    return DirectionalDivergence(
        force_a=comparison.directional_force_a,
        force_c=comparison.directional_force_c,
        is_divergent=comparison.directional_divergent,
        method=method,
        comparison_with_amplitude={
            "amplitude_force_a": comparison.amplitude_force_a,
            "amplitude_force_c": comparison.amplitude_force_c,
            "amplitude_divergent": float(comparison.amplitude_divergent),
            "agreement": float(comparison.agreement),
        },
    )


# ── 高级别力度（Move-as-component 场景）──────────────────────────
#
# 237号核心问题：高级别（level >= 2）的 _amplitude_force 使用 Move 的
# high/low 计算振幅，∈ ker(D)。以下函数提供方向性力度替代方案。
#
# Move 没有 i0/i1（merged bar 索引），而是有 seg_start/seg_end（线段索引）。
# 因此需要单独的高级别 API。


class MoveLike(Protocol):
    """Move 的最小接口（用于高级别力度计算）。"""

    @property
    def direction(self) -> str: ...

    @property
    def high(self) -> float: ...

    @property
    def low(self) -> float: ...

    @property
    def seg_start(self) -> int: ...

    @property
    def seg_end(self) -> int: ...


class LevelZhongshuLike(Protocol):
    """泛化中枢的最小接口。"""

    @property
    def zd(self) -> float: ...

    @property
    def zg(self) -> float: ...

    @property
    def comp_start(self) -> int: ...

    @property
    def comp_end(self) -> int: ...

    @property
    def settled(self) -> bool: ...


def level_directional_persistence(
    components: Sequence[MoveLike],
    start: int,
    end: int,
    direction: str,
) -> float:
    """高级别方向持续性：components[start:end+1] 中同向组件占比。

    组件 = settled(parent level moves)。
    纯方向信息——D 保留走势方向，不保留走势幅度。

    Parameters
    ----------
    components : Sequence[MoveLike]
        组件序列（settled parent level moves）。
    start, end : int
        组件索引范围 [start, end]（闭区间）。
    direction : str
        走势方向。

    Returns
    -------
    float
        方向持续性 ∈ [0, 1]。
    """
    if start > end or start < 0 or end >= len(components):
        return 0.0

    total = end - start + 1
    if total <= 0:
        return 0.0

    same_dir = sum(
        1 for i in range(start, end + 1)
        if components[i].direction == direction
    )
    return same_dir / total


def _level_max_consecutive(
    components: Sequence[MoveLike],
    start: int,
    end: int,
    direction: str,
) -> int:
    """高级别最长连续同向组件数。"""
    if start > end or start < 0 or end >= len(components):
        return 0
    max_run = 0
    current_run = 0
    for i in range(start, end + 1):
        if components[i].direction == direction:
            current_run += 1
            if current_run > max_run:
                max_run = current_run
        else:
            current_run = 0
    return max_run


def level_directional_persistence_weighted(
    components: Sequence[MoveLike],
    start: int,
    end: int,
    direction: str,
) -> float:
    """高级别加权方向持续性。

    = ratio * (max_consecutive_run / total)
    """
    ratio = level_directional_persistence(components, start, end, direction)
    total = end - start + 1
    if total <= 0:
        return 0.0
    max_run = _level_max_consecutive(components, start, end, direction)
    return ratio * (max_run / total)


def level_component_density_force(
    components: Sequence[MoveLike],
    start: int,
    end: int,
) -> float:
    """高级别组件密度力度。

    在高级别，组件（= settled parent moves）的密度反映结构复杂度。
    seg_span = 总线段跨度 ≈ 复杂度的代理。
    组件数 / seg_span = 密度。力度 = 1 / density。

    Parameters
    ----------
    components : Sequence[MoveLike]
        组件序列。
    start, end : int
        组件索引范围。

    Returns
    -------
    float
        力度值。
    """
    if start > end or start < 0 or end >= len(components):
        return 0.0

    seg_span = components[end].seg_end - components[start].seg_start + 1
    if seg_span <= 0:
        return 0.0

    comp_count = end - start + 1
    density = comp_count / seg_span
    if density <= 0:
        return float(seg_span)
    return 1.0 / density


def level_zhongshu_drift_force(
    zhongshus: Sequence[LevelZhongshuLike],
    zs_indices: Sequence[int],
    direction: str,
) -> float:
    """高级别中枢偏移力度（泛化中枢版本）。

    与 zhongshu_drift_force 逻辑相同，但接口适配 LevelZhongshu。

    Parameters
    ----------
    zhongshus : Sequence[LevelZhongshuLike]
        泛化中枢序列。
    zs_indices : Sequence[int]
        参与比较的中枢索引。
    direction : str
        走势方向。

    Returns
    -------
    float
        中枢偏移力度。
    """
    if len(zs_indices) < 2:
        return 0.0

    valid = [i for i in zs_indices if 0 <= i < len(zhongshus)]
    if len(valid) < 2:
        return 0.0

    total_drift = 0.0
    count = 0
    for k in range(1, len(valid)):
        prev = zhongshus[valid[k - 1]]
        curr = zhongshus[valid[k]]
        prev_mid = (prev.zd + prev.zg) / 2
        curr_mid = (curr.zd + curr.zg) / 2
        prev_width = prev.zg - prev.zd
        if prev_width <= 0:
            continue
        shift = (curr_mid - prev_mid) / prev_width
        if direction == "up" and shift > 0:
            total_drift += shift
        elif direction == "down" and shift < 0:
            total_drift += abs(shift)
        count += 1

    if count == 0:
        return 0.0
    return total_drift / count


def level_directional_force(
    components: Sequence[MoveLike],
    start: int,
    end: int,
    direction: str,
    zhongshus: Sequence[LevelZhongshuLike] | None = None,
    zs_indices: Sequence[int] | None = None,
) -> float:
    """高级别复合方向性力度——替代 _amplitude_force。

    这是 a_nested_divergence.py 中 _amplitude_force 的方向性替代。
    237号诊断的直接修复：力度 ∉ ker(D)。

    Parameters
    ----------
    components : Sequence[MoveLike]
        组件序列（settled parent level moves）。
    start, end : int
        组件索引范围。
    direction : str
        走势方向。
    zhongshus : Sequence[LevelZhongshuLike] | None
        泛化中枢序列（可选）。
    zs_indices : Sequence[int] | None
        中枢索引列表（可选）。

    Returns
    -------
    float
        方向性力度。
    """
    persistence = level_directional_persistence_weighted(
        components, start, end, direction,
    )
    density = level_component_density_force(components, start, end)

    if (
        zhongshus is not None
        and zs_indices is not None
        and len(zs_indices) >= 2
    ):
        drift = level_zhongshu_drift_force(zhongshus, zs_indices, direction)
        return 0.4 * persistence + 0.3 * density + 0.3 * drift

    return 0.5 * persistence + 0.5 * density


def level_amplitude_force(
    components: Sequence[MoveLike],
    start: int,
    end: int,
) -> float:
    """高级别振幅力度（∈ ker(D)，仅用于对比基线）。

    复制自 a_nested_divergence.py 的 _amplitude_force。
    """
    if start > end or start < 0 or end >= len(components):
        return 0.0
    high = max(components[k].high for k in range(start, end + 1))
    low = min(components[k].low for k in range(start, end + 1))
    duration = max(1, end - start + 1)
    return (high - low) * duration


@dataclass(frozen=True, slots=True)
class LevelForceComparison:
    """高级别力度对比结果。"""

    amplitude_force_a: float
    amplitude_force_c: float
    amplitude_divergent: bool
    directional_force_a: float
    directional_force_c: float
    directional_divergent: bool
    agreement: bool


def compare_level_force(
    components: Sequence[MoveLike],
    a_start: int,
    a_end: int,
    c_start: int,
    c_end: int,
    direction: str,
    zhongshus: Sequence[LevelZhongshuLike] | None = None,
    zs_indices_a: Sequence[int] | None = None,
    zs_indices_c: Sequence[int] | None = None,
) -> LevelForceComparison:
    """高级别 A/C 段力度对比。

    同时计算振幅力度和方向性力度，对比两者的背驰判定。
    这是 T6 可达性验证的核心工具。

    Parameters
    ----------
    components : Sequence[MoveLike]
        组件序列。
    a_start, a_end : int
        A 段组件索引范围。
    c_start, c_end : int
        C 段组件索引范围。
    direction : str
        走势方向。
    zhongshus, zs_indices_a, zs_indices_c
        泛化中枢信息（可选）。

    Returns
    -------
    LevelForceComparison
    """
    amp_a = level_amplitude_force(components, a_start, a_end)
    amp_c = level_amplitude_force(components, c_start, c_end)
    amp_div = amp_a > 0 and amp_c < amp_a

    dir_a = level_directional_force(
        components, a_start, a_end, direction,
        zhongshus, zs_indices_a,
    )
    dir_c = level_directional_force(
        components, c_start, c_end, direction,
        zhongshus, zs_indices_c,
    )
    dir_div = dir_a > 0 and dir_c < dir_a

    return LevelForceComparison(
        amplitude_force_a=amp_a,
        amplitude_force_c=amp_c,
        amplitude_divergent=amp_div,
        directional_force_a=dir_a,
        directional_force_c=dir_c,
        directional_divergent=dir_div,
        agreement=amp_div == dir_div,
    )
