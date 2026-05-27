"""A 系统 — PH-中枢检测：从 merge tree 的 H0 barcode 内在地识别缠论中枢。

存在论位置（§7 内在递归的中枢化）
----------------------------------
`persistence_theory.md` §7.1 证明 H0 sublevel barcode 是真正的区间套树（merge
tree），且价格嵌套同时是时间嵌套（0 违例）。本模块把这个嵌套树的**重叠结构**
形式化为缠论中枢——级别从 persistence 自动涌现，无需预选周期。

任务字面定义的严格化（必读——这是一次定义澄清，不是偏离）
--------------------------------------------------------
用户给定的字面判据是「一个父 bar 有 ≥3 个同级别子 bar（persistence 在同一量级的
children）= 中枢」。但 merge tree 的价格区间包含是**链式**的（elder rule 使每个
内部摆动严格窄于其包络）——实测合成中枢 [20,10,18,11,17,12,19,5,8,2] 的包含树是
bar1[10,19]⊃bar2[11,18]⊃bar3[12,17]，**每个父只有 1 个直接 child**，永远凑不齐 3
个直接 children。§7.3 已结算同一现象："树不平衡，不能往下走一层=次级别"。

字面判据在 merge tree 上的**精确等价形式**：bar1/bar2/bar3 这三个 persistence
同量级（9/7/5）、时间连续、价格区间两两重叠的摆动，正是缠论"≥3 个连续次级别走势"。
它们的重叠区 [max(births), min(deaths)] = [12,17] = 缠论 ZD/ZG。故：

  「父 bar 有 ≥3 个同级别子 bar」
  ⟺ 「同一 persistence 层内，≥3 个时间连续、价格区间两两重叠的 bars」
  其**外包络**（含这组的最小 bar）= 字面定义中的「父 bar」。

这与缠论中枢的标准识别算法（连续 3+ 次级别走势重叠）**同构**，区别仅在：层不是
预选周期，而是 persistence 量级的 log-gap 横切（§4/§7.3，a_level_detection）。

中枢区间的两种读法（任务定义 vs 缠论 ground-truth）
----------------------------------------------------
- **外包络** [envelope_low, envelope_high] = 父 bar 的 [birth, death]（任务字面
  "中枢区间=父bar价格范围"）——这是中枢所在的**整个摆动带**，比真实中枢宽。
- **重叠区** [zd, zg] = [max(member births), min(member deaths)]（缠论 ZD/ZG）——
  这是次级别走势的**公共重叠**，是缠论中枢的精确边界。

两者都产出，供 L2 验证「PH 中枢 vs 缠论/一禅中枢」的对应（外包络对应"中枢所在
段"，重叠区对应"中枢矩形框"）。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- 重叠滑窗 + log-gap 分层 + 外包络：**L0**（纯算法，从 barcode 确定性推导）。
- "PH 中枢 ↔ 缠论中枢对应"经验断言：腾讯 700 L2 验证前停留 **L1**。

设计约束（coding-style）
------------------------
- 所有产出（Zhongshu）：frozen + slots，immutable。
- 中枢判据（ZhongshuPolicy.same_level_members）是**显式可配置的值判断**——它直接
  塑造"同量级"的含义与中枢数量，是本特征行为的核心决策点（learning-mode）。

概念溯源标签
-----------
- PH-中枢 = 同层重叠摆动组 [新缠论:候选——§7 嵌套树重叠化]
- 中枢 ZD/ZG = 重叠区 [新缠论:知识库——中枢重叠区间定义]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

from newchan.a_level_detection import (
    CalendarPeriodNamer,
    PeriodNamer,
    adaptive_gap_boundaries,
)
from newchan.a_online_persistence import MergeBar

__all__ = [
    "Zhongshu",
    "ZhongshuPolicy",
    "build_containment_forest",
    "detect_zhongshu",
]


# ====================================================================
# 中枢数据类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class Zhongshu:
    """从 H0 barcode 识别出的一个中枢 = 同层 ≥3 个重叠摆动构成的振荡结构。

    Attributes
    ----------
    zd : float
        中枢下沿 = max(member births)（缠论 ZD，重叠区下界）。
    zg : float
        中枢上沿 = min(member deaths)（缠论 ZG，重叠区上界）。
    envelope_low, envelope_high : float
        外包络父 bar 的 [birth, death]（任务字面"中枢区间=父bar价格范围"）。
        若无更紧外包络（成员本身即顶层），退化为成员 births/deaths 的极值。
    level_persistence : float
        构成中枢的同级别摆动的代表 persistence（成员 persistence 中位数）。
    members : tuple[MergeBar, ...]
        构成中枢的 ≥3 个同级别、时间连续、价格重叠的摆动（按时间排序）。
    parent_bar : MergeBar | None
        外包络父 bar（含全部成员的最小 persistence bar）；无更紧者为 None。
    lo, hi : int
        中枢覆盖的时间区间（= 成员时间区间的并）。
    period_label : str
        中枢级别名（由父 bar / 成员 avg_span 映射）。
    """

    zd: float
    zg: float
    envelope_low: float
    envelope_high: float
    level_persistence: float
    members: tuple[MergeBar, ...]
    parent_bar: MergeBar | None
    lo: int
    hi: int
    period_label: str

    @property
    def n_members(self) -> int:
        return len(self.members)

    @property
    def width(self) -> float:
        """中枢重叠区宽度 = zg − zd（中枢震荡幅度）。"""
        return self.zg - self.zd

    @property
    def span(self) -> int:
        """中枢覆盖的 K 线根数 = hi − lo + 1。"""
        return self.hi - self.lo + 1


# ====================================================================
# 中枢判据（值判断 —— 直接塑造"同量级"与中枢数量）
# ====================================================================


@dataclass(frozen=True, slots=True)
class ZhongshuPolicy:
    """控制中枢识别的策略（值判断，塑造中枢数量与"同级别"含义）。

    Attributes
    ----------
    min_members : int
        构成中枢的最小同级别摆动数（缠论：至少 3 个连续次级别走势）。
    same_level_ratio : float
        判定两个摆动"persistence 同量级"的最大比值（max/min < ratio）。
        与 log-gap 分层互补：log-gap 给出全局层边界，此比值在层内再约束
        成员紧致度（防止一层内 persistence 跨度过大的伪中枢）。
    require_overlap : bool
        是否要求成员价格区间存在公共重叠（max births < min deaths）。
        True = 严格缠论中枢（重叠区非空）；False = 仅同层时间连续即可。
    """

    min_members: int = 3
    same_level_ratio: float = 3.0
    require_overlap: bool = True

    def same_level_members(
        self, time_sorted_layer: Sequence[MergeBar], start: int
    ) -> tuple[MergeBar, ...]:
        """从某 persistence 层内、按时间排序的 bars，自 start 起取极大中枢成员组。

        ★ Learning-mode 决策点 ★
        这是 PH-中枢识别的核心：它决定"同量级 + 时间连续 + 价格重叠"如何精确成组，
        从而决定数据里到底涌现几个中枢、各中枢的边界在哪。缠论中枢的本质是
        「连续次级别走势的重叠区间」——本函数把它形式化为：自 start 起，沿时间
        扩展成员，维持 (a) 公共价格重叠非空 [max births < min deaths] 且
        (b) persistence 同量级 [max/min < same_level_ratio]；任一破裂则停止。

        若你想改变中枢的成组逻辑——例如允许重叠区短暂破裂后重连（缠论中枢延伸/
        新生的边界）、或放宽/收紧同量级判据——就在这里调整。

        Parameters
        ----------
        time_sorted_layer : Sequence[MergeBar]
            同一 persistence 层（log-gap 横切出的一个量级）内、按 lo 升序的 bars。
        start : int
            从该层的第 start 个 bar 开始尝试成组。

        Returns
        -------
        tuple[MergeBar, ...]
            自 start 起满足重叠 + 同量级的极大连续成员组（可能 < min_members，
            由调用方过滤）。
        """
        members: list[MergeBar] = []
        zd = float("-inf")  # 重叠区下界 = max(births)
        zg = float("inf")   # 重叠区上界 = min(deaths)
        pmin = float("inf")
        pmax = float("-inf")
        for k in range(start, len(time_sorted_layer)):
            b = time_sorted_layer[k]
            new_zd = max(zd, b.birth_price)
            new_zg = min(zg, b.death_price)
            new_pmin = min(pmin, b.persistence)
            new_pmax = max(pmax, b.persistence)
            # (a) 重叠约束：加入 b 后公共重叠仍非空
            if self.require_overlap and new_zd >= new_zg:
                break
            # (b) 同量级约束：加入 b 后成员 persistence 仍在一个量级内
            if new_pmin > 0 and new_pmax / new_pmin >= self.same_level_ratio:
                break
            members.append(b)
            zd, zg, pmin, pmax = new_zd, new_zg, new_pmin, new_pmax
        return tuple(members)


# ====================================================================
# 包含森林（外包络父 bar 用）
# ====================================================================


def build_containment_forest(
    bars: Sequence[MergeBar],
) -> dict[int, list[int]]:
    """从 bars 的价格区间包含关系构建直接父→子映射（含 -1 作为虚根）。

    A 是 B 的祖先 ⟺ A 的价格区间 [birth, death] 严格包含 B 的（A.birth ≤ B.birth、
    A.death ≥ B.death、A.persistence > B.persistence）。直接父 = 所有祖先中
    persistence 最小者（最紧外包络）。无祖先者父为 -1（森林根）。

    返回 {parent_index: [child_index, ...]}，parent_index=-1 收集所有根。
    索引是 bars 序列中的位置。仅用于外包络归属——中枢识别本身按 persistence 层
    （不依赖此树的链式深度，见模块 docstring §7.3）。

    认识论等级：L0（确定性，从区间包含推导）。
    """
    n = len(bars)
    forest: dict[int, list[int]] = {-1: []}
    for j in range(n):
        bj = bars[j]
        best_parent = -1
        best_pers = float("inf")
        for i in range(n):
            if i == j:
                continue
            bi = bars[i]
            contains = (
                bi.birth_price <= bj.birth_price
                and bi.death_price >= bj.death_price
                and bi.persistence > bj.persistence
            )
            if contains and bi.persistence < best_pers:
                best_pers = bi.persistence
                best_parent = i
        forest.setdefault(best_parent, []).append(j)
    return forest


def _tightest_envelope(
    members: Sequence[MergeBar], bars: Sequence[MergeBar]
) -> MergeBar | None:
    """含全部成员（价格 + 时间区间）的最小 persistence bar = 外包络父 bar。

    None = 不存在比成员更高一级的外包络（成员本身即顶层结构）。
    """
    m_birth = min(m.birth_price for m in members)
    m_death = max(m.death_price for m in members)
    m_lo = min(m.lo for m in members)
    m_hi = max(m.hi for m in members)
    member_ids = {id(m) for m in members}
    best: MergeBar | None = None
    for b in bars:
        if id(b) in member_ids:
            continue
        if (
            b.birth_price <= m_birth
            and b.death_price >= m_death
            and b.lo <= m_lo
            and b.hi >= m_hi
            and b.persistence > max(m.persistence for m in members)
        ):
            if best is None or b.persistence < best.persistence:
                best = b
    return best


# ====================================================================
# 核心：分层 + 重叠滑窗中枢识别
# ====================================================================


def _layer_bars(
    bars: Sequence[MergeBar], *, n_mad: float, noise_floor: float
) -> list[list[MergeBar]]:
    """按 persistence 量级（log-gap 横切，§4/§7.3）把 bars 分层。

    复用 a_level_detection.adaptive_gap_boundaries（MAD 离群 log-gap，L0 稳健统计，
    非领域调参）。每层 = 一个 persistence 量级 = 一个缠论级别。返回各层 bars 列表
    （粗→细），层内未排序（调用方按时间排序）。全局 bar（is_global）排除——它是
    全段包络，不参与某一级别的中枢成组。
    """
    pos = [
        b
        for b in bars
        if b.persistence > max(0.0, noise_floor) and not b.is_global
    ]
    if not pos:
        return []
    boundaries = adaptive_gap_boundaries([b.persistence for b in pos], n_mad=n_mad)
    if not boundaries:
        return [pos]
    edges = [float("inf"), *sorted(boundaries, reverse=True), 0.0]
    layers: list[list[MergeBar]] = []
    for k in range(len(edges) - 1):
        hi, lo = edges[k], edges[k + 1]
        band = [
            b for b in pos if (b.persistence < hi or k == 0) and b.persistence >= lo
        ]
        if band:
            layers.append(band)
    return layers


def detect_zhongshu(
    bars: Sequence[MergeBar],
    *,
    bars_per_day: float = 1.0,
    noise_floor: float = 0.0,
    n_mad: float = 3.0,
    policy: ZhongshuPolicy | None = None,
    namer: PeriodNamer | None = None,
) -> tuple[Zhongshu, ...]:
    """从 H0 merge tree 的 bars 识别全部中枢（每个 persistence 层独立成组）。

    流程（见模块 docstring 的定义澄清）：
    1. 按 persistence log-gap 横切分层（a_level_detection）——每层 = 一个级别。
    2. 每层内 bars 按时间排序，用重叠滑窗（ZhongshuPolicy.same_level_members）找
       极大的「时间连续 + 价格重叠 + 同量级」成员组。
    3. 成员数 ≥ policy.min_members 的组 = 一个中枢；中枢区间取重叠区 [zd, zg]
       与外包络 [envelope_low, envelope_high] 两种读法。

    Parameters
    ----------
    bars : Sequence[MergeBar]
        H0 特征（OnlineMergeTree.settled_bars / finalize，或批量 sublevel_h0_bars
        转 MergeBar）。
    bars_per_day : float
        K 线密度（周期映射桥接，日线=1）。
    noise_floor : float
        persistence ≤ 此值的特征视为噪声丢弃（建议 atr_noise_threshold）。
    n_mad : float
        分层的 MAD 离群阈（L0 稳健统计，默认 3σ）。
    policy : ZhongshuPolicy | None
        中枢判据（默认 ZhongshuPolicy()）。
    namer : PeriodNamer | None
        级别命名（默认 CalendarPeriodNamer()）。

    Returns
    -------
    tuple[Zhongshu, ...]
        全部中枢，按时间起点 lo 升序。

    认识论等级：分层 + 重叠滑窗 = L0；中枢↔缠论对应 = L1（待 700 L2 验证）。
    """
    policy = policy or ZhongshuPolicy()
    namer = namer or CalendarPeriodNamer()
    bars = tuple(bars)

    layers = _layer_bars(bars, n_mad=n_mad, noise_floor=noise_floor)
    zhongshus: list[Zhongshu] = []

    for layer in layers:
        time_sorted = sorted(layer, key=lambda b: b.lo)
        i = 0
        while i < len(time_sorted):
            members = policy.same_level_members(time_sorted, i)
            if len(members) >= policy.min_members:
                zhongshus.append(_make_zhongshu(members, bars, bars_per_day, namer))
                i += len(members)  # 极大组后从组尾继续（不重叠扫描）
            else:
                i += 1

    zhongshus.sort(key=lambda z: z.lo)
    return tuple(zhongshus)


def _median(xs: list[float]) -> float:
    s = sorted(xs)
    n = len(s)
    mid = n // 2
    return s[mid] if n % 2 else (s[mid - 1] + s[mid]) / 2.0


def _make_zhongshu(
    members: tuple[MergeBar, ...],
    bars: Sequence[MergeBar],
    bars_per_day: float,
    namer: PeriodNamer,
) -> Zhongshu:
    zd = max(m.birth_price for m in members)   # 重叠区下界
    zg = min(m.death_price for m in members)   # 重叠区上界
    envelope = _tightest_envelope(members, bars)
    if envelope is not None:
        env_low, env_high = envelope.birth_price, envelope.death_price
    else:
        env_low = min(m.birth_price for m in members)
        env_high = max(m.death_price for m in members)
    lo = min(m.lo for m in members)
    hi = max(m.hi for m in members)
    level_pers = _median([m.persistence for m in members])
    # 级别命名：用外包络的 span（中枢所在级别）；无包络用成员平均 span
    if envelope is not None:
        label = namer.name(float(envelope.span), bars_per_day)
    else:
        avg_span = sum(m.span for m in members) / len(members)
        label = namer.name(avg_span, bars_per_day)
    return Zhongshu(
        zd=zd,
        zg=zg,
        envelope_low=env_low,
        envelope_high=env_high,
        level_persistence=level_pers,
        members=tuple(sorted(members, key=lambda b: b.lo)),
        parent_bar=envelope,
        lo=lo,
        hi=hi,
        period_label=label,
    )
