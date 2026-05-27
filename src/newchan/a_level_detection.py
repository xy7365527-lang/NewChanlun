"""A 系统 — 从 persistence 分布自动检测缠论级别（级别从数据内在涌现）。

存在论位置
----------
`persistence_theory.md` §4 + §7.3 确立：**级别 = persistence 阈值的水平横切**（不是
merge tree 的深度）。本模块把"横切"自动化——不再人工给定阈值 τ，而是从 persistence
分布的**对数间隙（log gap）**自动找级别边界。

核心直觉（为什么取对数 + 找 gap）
---------------------------------
缠论级别是**乘性**的（次级别 ~ 主级别的固定比例，如日线→30分 ≈ 8 倍 K 线密度）。
乘性结构在**对数尺度**上变成加性——同一级别的特征 persistence 聚成一簇，相邻级别
之间在对数轴上留下**间隙**。最大对数间隙 = 两个相邻级别最自然的边界。

递归：在每个级别簇内部继续找次大间隙 → 子级别。检测到几级就是几级，**不预设级别数**
（与缠论"级别无限递归"一致）。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- log-gap 检测算法：**L0**（纯算法，从 persistence 分布确定性推导）。
- "log-gap 边界 = 缠论级别边界"经验断言：真实数据 L2/L3（跟一禅指标对比，见 §8）前
  停留 **L1**。
- span→周期映射阈值（日历日）：**L2**（经验映射，编排者/操盘域给定，可否证）。

设计约束（coding-style）
------------------------
- 所有产出（Level / LevelStructure）：frozen + slots，immutable。
- 周期命名是 Protocol（PeriodNamer）——可替换不同市场的周期口径。
- 级别分裂的**终止策略**（SplitPolicy）是显式可配置的值判断——它直接塑造"涌现几个
  级别"，是本特征的行为核心。

概念溯源标签
-----------
- persistence log-gap 自动定级 [新缠论:候选——§4/§7.3 横切自动化]
- 级别乘性递归 [新缠论:知识库——级别无限递归]
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Protocol, Sequence, runtime_checkable

from newchan.a_online_persistence import MergeBar

__all__ = [
    "Level",
    "LevelStructure",
    "SplitPolicy",
    "PeriodNamer",
    "CalendarPeriodNamer",
    "detect_levels",
    "detect_levels_adaptive",
    "adaptive_gap_boundaries",
    "SpectrumPoint",
    "PersistenceSpectrum",
    "persistence_spectrum",
]


# ====================================================================
# 周期命名（Protocol + 默认日历口径实现）
# ====================================================================


@runtime_checkable
class PeriodNamer(Protocol):
    """把"平均 span（K 线根数）"映射到缠论级别名称的策略接口。

    不同市场 / 周期的 K 线密度不同（bars_per_day 不同），命名口径可替换。
    """

    def name(self, avg_span_bars: float, bars_per_day: float) -> str: ...


@dataclass(frozen=True, slots=True)
class CalendarPeriodNamer:
    """日历日口径的周期命名（编排者/操盘域给定的经验阈值，L2）。

    把 span（K 线根数）换算为日历日（span_days = avg_span_bars / bars_per_day），
    再按日历日阈值归入缠论级别。换算桥接了不同周期数据（日线 bars_per_day=1，
    30 分钟港股 ≈ 11、A 股 ≈ 8）——所以阈值只需用日历日表达一次，对任意周期通用。

    阈值（日历日，编排者给定）::

        > 100 日 → 月线级
        40 ~ 100 → 周线级
        10 ~ 40  → 日线级
        2 ~ 10   → 60分钟级
        < 2 日   → 30分钟级或更小
    """

    # (上界日历日, 级别名)，升序；最后一项 inf 兜底
    thresholds: tuple[tuple[float, str], ...] = (
        (2.0, "30分钟级或更小"),
        (10.0, "60分钟级"),
        (40.0, "日线级"),
        (100.0, "周线级"),
        (math.inf, "月线级"),
    )

    def name(self, avg_span_bars: float, bars_per_day: float) -> str:
        span_days = avg_span_bars / bars_per_day if bars_per_day > 0 else avg_span_bars
        for upper, label in self.thresholds:
            if span_days < upper:
                return label
        return self.thresholds[-1][1]


# ====================================================================
# 级别分裂终止策略（值判断 —— 直接塑造涌现几个级别）
# ====================================================================


@dataclass(frozen=True, slots=True)
class SplitPolicy:
    """控制 log-gap 递归分裂何时停止的策略（值判断，塑造级别数）。

    三个旋钮（任一不满足即停止在该簇分裂，该簇成为一个叶级别）：

    Attributes
    ----------
    min_gap_ratio : float
        触发分裂所需的最小 persistence **比值**（簇间间隙 = e^(log_gap)）。
        例：1.8 表示相邻级别的 persistence 至少差 1.8 倍才算"自然边界"。
        越大越保守（级别越少、越粗），越小越敏感（级别越多、越细）。
    min_bars_per_level : int
        一个簇低于此特征数就不再细分（避免把噪声切成"级别"）。
    max_levels : int
        **软性分裂闸**（非严格总数上界）。在分裂决策点检查 current_level_count；
        但深度优先递归中一次分裂原子地承诺 2 个子级别，已承诺子树仍会完成，故
        实际级别数**可溢出** max_levels（max_levels=4 实测产出 7）。
        ⚠ **spec-execution-gap（待裁决）**：严格"总数上界"在二叉递归上需破坏分裂
        原子性（产生残缺子树）或事后剪枝；"深度上界"则干净（匹配递归结构）但与
        参数名 max_levels（总数）冲突。语义裁决（总数 vs 深度）留待编排者，
        当前保持软闸不猜测性 patch（见 test_level_detection.py 的 xfail）。
    """

    min_gap_ratio: float = 1.8
    min_bars_per_level: int = 2
    max_levels: int = 8

    def should_split(
        self,
        log_gap: float,
        group_size: int,
        current_level_count: int,
    ) -> bool:
        """是否在某簇的最大对数间隙处继续分裂出子级别。

        ★ Learning-mode 决策点 ★
        这是本特征行为的核心：它决定"数据里到底涌现几个级别"。逻辑是三个旋钮的
        合取（全满足才分裂）。如果你想改变级别涌现的灵敏度——例如要求子级别之间
        必须有更大的力度落差、或允许更深的递归——就在这里调整判据。

        Parameters
        ----------
        log_gap : float
            该簇内最大对数间隙（= ln(高簇persistence / 低簇persistence) 在边界处）。
        group_size : int
            该簇当前的特征数。
        current_level_count : int
            已经形成的级别数（用于 max_levels 保护）。

        Returns
        -------
        bool
            True = 在此间隙处分裂出子级别；False = 停止，该簇为叶级别。
        """
        # 默认合取逻辑（learning-mode 决策点，编排者 2026-05-27 裁定：先保持默认）。
        if group_size < self.min_bars_per_level * 2:
            return False  # 分裂后两侧至少各 min_bars_per_level 个，否则不分
        if current_level_count >= self.max_levels:
            return False  # 软性分裂闸：在分裂点检查，已承诺的子树仍会完成（可溢出）
        return log_gap >= math.log(self.min_gap_ratio)


# ====================================================================
# 级别数据类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class Level:
    """一个自动检测出的缠论级别（persistence 分布的一个簇）。

    Attributes
    ----------
    level_id : int
        级别编号（0 = 根/最粗；编号随检测顺序递增）。
    persistence_range : tuple[float, float]
        该级别特征的 persistence 区间 [min, max]。
    bars : tuple[MergeBar, ...]
        归入该级别的特征（persistence 降序）。
    avg_span : float
        该级别特征的平均 span（K 线根数）——映射周期的依据。
    period_label : str
        映射到的缠论级别名称（如"日线级"）。
    parent_id : int | None
        次级别归属的上级级别编号（None = 根级别）。
    child_ids : tuple[int, ...]
        该级别向下分裂出的子级别编号。
    depth : int
        递归深度（0 = 根）。
    """

    level_id: int
    persistence_range: tuple[float, float]
    bars: tuple[MergeBar, ...]
    avg_span: float
    period_label: str
    parent_id: int | None
    child_ids: tuple[int, ...]
    depth: int

    @property
    def n_bars(self) -> int:
        return len(self.bars)

    @property
    def max_persistence(self) -> float:
        return self.persistence_range[1]


@dataclass(frozen=True, slots=True)
class LevelStructure:
    """一段序列上自动检测出的完整级别结构（级别树）。

    Attributes
    ----------
    levels : tuple[Level, ...]
        全部级别（按 level_id 升序）。
    bars_per_day : float
        计算所用的 K 线密度（周期映射桥接参数）。
    """

    levels: tuple[Level, ...]
    bars_per_day: float

    def by_id(self, level_id: int) -> Level:
        return self.levels[level_id]

    @property
    def leaves(self) -> tuple[Level, ...]:
        """叶级别（不再细分的最细级别）——通常对应可直接交易的级别。"""
        return tuple(lv for lv in self.levels if not lv.child_ids)

    @property
    def roots(self) -> tuple[Level, ...]:
        """根级别（无上级）。"""
        return tuple(lv for lv in self.levels if lv.parent_id is None)


# ====================================================================
# 核心：log-gap 递归级别检测
# ====================================================================


def _largest_log_gap(persistences: list[float]) -> tuple[float, float] | None:
    """在 persistence 多重集中找最大对数间隙。

    Returns
    -------
    (log_gap, split_value) 或 None
        log_gap = 最大相邻对数间隙；split_value = 分裂阈值（间隙两端的几何中点）。
        persistence < split_value → 细簇；>= split_value → 粗簇。
        正特征 < 2 个时返回 None（无法分裂）。
    """
    logs = sorted(math.log(p) for p in persistences if p > 0.0)
    if len(logs) < 2:
        return None
    best_gap = -1.0
    best_mid = 0.0
    for i in range(1, len(logs)):
        gap = logs[i] - logs[i - 1]
        if gap > best_gap:
            best_gap = gap
            best_mid = (logs[i] + logs[i - 1]) / 2.0
    return best_gap, math.exp(best_mid)


def detect_levels(
    bars: Sequence[MergeBar],
    *,
    bars_per_day: float = 1.0,
    noise_floor: float = 0.0,
    policy: SplitPolicy | None = None,
    namer: PeriodNamer | None = None,
) -> LevelStructure:
    """从一组 H0 特征自动检测级别结构（log-gap 递归横切）。

    Parameters
    ----------
    bars : Sequence[MergeBar]
        H0 特征（来自 OnlineMergeTree 的 settled/all bars，或批量 barcode）。
    bars_per_day : float
        K 线密度（日线=1，30 分钟港股≈11）——周期映射桥接。
    noise_floor : float
        噪声阈值 τ（§4）：persistence <= τ 的特征被丢弃，不参与级别检测。
        **这一步是算法前置**——用户 spec 的"取所有 active bar"即指超过 τ 的 bar。
        不滤噪时近零噪声会被 log-gap 反复剥离成伪级别（建议用 atr_noise_threshold）。
        默认 0（仅滤掉 persistence<=0 的奇点）。
    policy : SplitPolicy | None
        分裂终止策略（默认 SplitPolicy()）。
    namer : PeriodNamer | None
        周期命名策略（默认 CalendarPeriodNamer()）。

    Returns
    -------
    LevelStructure
        级别树（levels 按 level_id 升序，含 parent/child 关系）。

    认识论等级：log-gap 检测 L0；级别↔周期映射 L2。
    """
    policy = policy or SplitPolicy()
    namer = namer or CalendarPeriodNamer()

    pos_bars = tuple(b for b in bars if b.persistence > max(0.0, noise_floor))
    levels: list[Level] = []

    def _make_level(
        group: tuple[MergeBar, ...], parent_id: int | None, depth: int
    ) -> int:
        """登记一个级别（暂不填 child_ids，分裂后回填），返回 level_id。"""
        ordered = tuple(sorted(group, key=lambda b: b.persistence, reverse=True))
        pers = [b.persistence for b in ordered]
        avg_span = sum(b.span for b in ordered) / len(ordered)
        level_id = len(levels)
        levels.append(
            Level(
                level_id=level_id,
                persistence_range=(min(pers), max(pers)),
                bars=ordered,
                avg_span=avg_span,
                period_label=namer.name(avg_span, bars_per_day),
                parent_id=parent_id,
                child_ids=(),
                depth=depth,
            )
        )
        return level_id

    def _subdivide(group: tuple[MergeBar, ...], parent_id: int | None, depth: int) -> int:
        my_id = _make_level(group, parent_id, depth)
        gap = _largest_log_gap([b.persistence for b in group])
        if gap is None:
            return my_id
        log_gap, split_value = gap
        if not policy.should_split(log_gap, len(group), len(levels)):
            return my_id
        fine = tuple(b for b in group if b.persistence < split_value)
        coarse = tuple(b for b in group if b.persistence >= split_value)
        if not fine or not coarse:
            return my_id  # 退化分裂（全在一侧），停止
        # 粗簇在前（高级别），细簇在后（次级别）
        child_coarse = _subdivide(coarse, my_id, depth + 1)
        child_fine = _subdivide(fine, my_id, depth + 1)
        # 回填 child_ids（dataclass frozen → 重建）
        parent = levels[my_id]
        levels[my_id] = Level(
            level_id=parent.level_id,
            persistence_range=parent.persistence_range,
            bars=parent.bars,
            avg_span=parent.avg_span,
            period_label=parent.period_label,
            parent_id=parent.parent_id,
            child_ids=(child_coarse, child_fine),
            depth=parent.depth,
        )
        return my_id

    if pos_bars:
        _subdivide(pos_bars, None, 0)

    return LevelStructure(levels=tuple(levels), bars_per_day=bars_per_day)


# ====================================================================
# 自适应 log-gap 定级（无固定 min_gap_ratio——编排者 2026-05-27 第二次裁定）
# ====================================================================


def _median(xs: list[float]) -> float:
    s = sorted(xs)
    n = len(s)
    if n == 0:
        return 0.0
    mid = n // 2
    return s[mid] if n % 2 else (s[mid - 1] + s[mid]) / 2.0


def adaptive_gap_boundaries(persistences: list[float], *, n_mad: float = 3.0) -> list[float]:
    """数据驱动地找级别边界（不设固定比值阈值）——MAD 离群 log-gap。

    对排序后的 log-persistence 相邻间隙集，用稳健统计（中位数 + n_mad·MAD）判定
    **离群间隙** = 级别边界。MAD（中位数绝对偏差）×1.4826 ≈ 正态标准差，故 n_mad=3
    是通用的"3σ 离群"判据，**不是领域调参**——阈值随每段数据的间隙分布自适应。

    Returns
    -------
    list[float]
        分裂阈值（split_value）列表，升序。persistence 落在相邻阈值之间 = 同一级别。
        无离群间隙（间隙均匀）→ 返回空（单一级别）。
    """
    logs = sorted(math.log(p) for p in persistences if p > 0.0)
    if len(logs) < 3:
        return []
    gaps = [logs[i] - logs[i - 1] for i in range(1, len(logs))]
    med = _median(gaps)
    mad = _median([abs(g - med) for g in gaps])
    if mad <= 0.0:
        # 间隙全等 → 用极差兜底：仅当最大间隙显著大于均值才算边界
        mean_g = sum(gaps) / len(gaps)
        thr = mean_g * 2.0
    else:
        thr = med + n_mad * 1.4826 * mad
    boundaries = [
        math.exp((logs[i] + logs[i - 1]) / 2.0)
        for i in range(1, len(logs))
        if logs[i] - logs[i - 1] > thr
    ]
    return sorted(boundaries)


def detect_levels_adaptive(
    bars: Sequence[MergeBar],
    *,
    bars_per_day: float = 1.0,
    noise_floor: float = 0.0,
    n_mad: float = 3.0,
    namer: PeriodNamer | None = None,
) -> LevelStructure:
    """自适应级别检测：MAD 离群 log-gap 平铺分级（无固定 min_gap_ratio）。

    与递归 `detect_levels`（固定比值阈值）的区别：本函数**一次性**用稳健离群判据
    切出所有级别边界，级别数完全由数据的间隙分布决定（编排者裁定的"自适应最大
    log-gap"方案）。产出**线性级别链**（粗→细，parent = 上一更粗级别），每个级别是
    一个 persistence 带。

    认识论等级：MAD 离群 = L0（稳健统计，确定性）；级别↔周期 = L2。
    """
    namer = namer or CalendarPeriodNamer()
    pos = sorted(
        (b for b in bars if b.persistence > max(0.0, noise_floor)),
        key=lambda b: b.persistence,
        reverse=True,
    )
    if not pos:
        return LevelStructure(levels=(), bars_per_day=bars_per_day)

    boundaries = adaptive_gap_boundaries([b.persistence for b in pos], n_mad=n_mad)
    # 用边界把 bars 切成带（persistence 降序 → 粗级别在前）
    edges = [math.inf, *sorted(boundaries, reverse=True), 0.0]
    levels: list[Level] = []
    for k in range(len(edges) - 1):
        hi, lo = edges[k], edges[k + 1]
        band = tuple(b for b in pos if (b.persistence < hi or k == 0) and b.persistence >= lo)
        if not band:
            continue
        prs = [b.persistence for b in band]
        avg_span = sum(b.span for b in band) / len(band)
        lid = len(levels)
        levels.append(
            Level(
                level_id=lid,
                persistence_range=(min(prs), max(prs)),
                bars=band,
                avg_span=avg_span,
                period_label=namer.name(avg_span, bars_per_day),
                parent_id=lid - 1 if lid > 0 else None,  # 线性链：上一更粗级别为父
                child_ids=(),
                depth=lid,
            )
        )
    # 回填 child_ids（线性链：每级的子 = 下一更细级别）
    for i in range(len(levels) - 1):
        p = levels[i]
        levels[i] = Level(
            level_id=p.level_id, persistence_range=p.persistence_range, bars=p.bars,
            avg_span=p.avg_span, period_label=p.period_label, parent_id=p.parent_id,
            child_ids=(i + 1,), depth=p.depth,
        )
    return LevelStructure(levels=tuple(levels), bars_per_day=bars_per_day)


# ====================================================================
# 连续谱版本（不离散化——persistence diagram 本身就是连续尺度谱，§4）
# ====================================================================


@dataclass(frozen=True, slots=True)
class SpectrumPoint:
    """连续级别谱的一个点 = 一个特征在 (persistence, span) 尺度轴上的位置。"""

    persistence: float
    span: int
    period_label: str
    birth_idx: int
    death_idx: int | None


@dataclass(frozen=True, slots=True)
class PersistenceSpectrum:
    """连续级别谱：不切级别，直接暴露 persistence→span→周期 的连续映射（§4）。

    级别离散化（detect_levels / detect_levels_adaptive）是这个连续谱上的横切。
    连续谱保留全部分辨率，供"区间套连续读级别"使用。
    """

    points: tuple[SpectrumPoint, ...]  # persistence 降序
    bars_per_day: float

    def active_above(self, tau: float) -> tuple[SpectrumPoint, ...]:
        """τ 横切：persistence > τ 的特征（= §4 active_bars 的连续谱形式）。"""
        return tuple(p for p in self.points if p.persistence > tau)

    def active_count(self, tau: float) -> int:
        """τ 横切下的"在运作级别数"——τ 的阶梯函数（连续谱的可观测投影）。"""
        return sum(1 for p in self.points if p.persistence > tau)

    def level_count_curve(self, taus: Sequence[float]) -> tuple[tuple[float, int], ...]:
        """给定 τ 序列，返回 (τ, active_count) 曲线——级别数随尺度的连续变化。"""
        return tuple((float(t), self.active_count(t)) for t in taus)


def persistence_spectrum(
    bars: Sequence[MergeBar],
    *,
    bars_per_day: float = 1.0,
    noise_floor: float = 0.0,
    namer: PeriodNamer | None = None,
) -> PersistenceSpectrum:
    """从特征构造连续级别谱（不离散化）。

    每个特征按 persistence 降序排成谱点，span 映射周期名。这是"persistence diagram
    本身就是连续尺度谱"（§4）的直接对象化——级别检测是它的横切，连续谱是底层。

    认识论等级：L0（确定性映射）；周期名 L2。
    """
    namer = namer or CalendarPeriodNamer()
    pos = sorted(
        (b for b in bars if b.persistence > max(0.0, noise_floor)),
        key=lambda b: b.persistence,
        reverse=True,
    )
    points = tuple(
        SpectrumPoint(
            persistence=b.persistence,
            span=b.span,
            period_label=namer.name(float(b.span), bars_per_day),
            birth_idx=b.birth_idx,
            death_idx=b.death_idx,
        )
        for b in pos
    )
    return PersistenceSpectrum(points=points, bars_per_day=bars_per_day)
