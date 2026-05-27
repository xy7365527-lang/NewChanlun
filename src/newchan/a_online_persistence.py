"""A 系统 — 在线（因果）H0 merge tree：让级别递归从数据内在涌现。

存在论位置（§7.5 升级方向的实现）
-----------------------------------
`persistence_theory.md` §7.5 把 PH 的内在递归定位为"因果性受限的、按 prominence
组织的**事后**区间套"——整棵树用**含未来的全序列**算出，每个特征的 death = 未来的
鞍点，因此是 hindsight decomposition，不能进缠论的因果决策层。§7.5 给出唯一的升级
方向：

  > 只用截至当前的序列做**在线 merge tree**，但那样 death 会随未来不断改写，
  > 稳定性优势（bottleneck 定理）随之削弱。这是一个真实的张力。

本模块实现这个在线 merge tree，并把"death 随未来改写"的张力转化为一个**精确的因果
判据**：哪些特征已经 settled（death 因果确定，未来不可改写），哪些仍 alive（death
待定）。

因果 settle 判据（核心定理）
----------------------------
H0 sublevel merge tree 的一个合并事件发生在鞍点 peak `P`（两个相邻分量在阈值升到 P
时连通，elder rule 下年轻分量死亡）。**这个合并的配对结果（谁死、persistence 多少）
在因果上确定，当且仅当 P 右侧已经出现 ≥ P 的价格点。**

证明（充分性 + 必要性）：合并在 P 处把 P 左、右两个区域连通。死亡的是两区域 elder
（最低 valley）中较年轻者。P 左侧 elder 在 P 之前已确定。P 右侧 elder 在"P 右侧 P-region
被一个 ≥ P 的屏障封闭"之前可能继续降低（未来创新低翻转 elder rule）。一旦右侧出现
≥ P 的点 Q，P 右侧低于 P 的区域被 Q 封闭，右侧 elder 定格 → 配对定格。反之未出现 Q
时，未来一个更低的 valley 会改写右侧 elder，从而翻转死亡方与 persistence（见
test_online_persistence 的翻转反例）。∎

**与缠论的同构**：这个判据正是缠论"顶/底分型需要后续 K 线确认才成立"的拓扑形式化。
分型的因果确认 = persistence 合并事件的因果 settle。

算法（单调栈，O(n) 流式）
-------------------------
逐根 K 线 `update(price)`。维护一个 alive 分量栈 `_stack`（时间从左到右）与分量之间
的屏障 peak 栈 `_barriers`（不变量：屏障值从栈底到栈顶**严格递减**）。

- 价格**上升**越过栈顶屏障 → cascade 合并（settle 年轻分量），死亡值 = 屏障值。
  这正是"右侧出现 ≥ P 的点"的因果时刻。
- 价格在一个已 bottomed 的分量上**回落** → 确认一个 peak，该 peak 成为新屏障，
  新分量在回落处诞生。
- 仍在栈中的分量 = alive（其向左合并尚未被右侧 ≥ 屏障的价格触发）。栈底 = 全局
  最低 valley，永远 alive 直到 finalize 封顶。

`update()` 内的合并顺序（按价格升序触及屏障）与批量 `sublevel_h0_bars()` 的价格升序
扫描**数学等价**：在线版 settled ∪ alive 的有限 persistence 多重集 ≡ 批量版的有限
bar 多重集（test_online_persistence 用随机 + 结构化序列做 property-based 验证——
**因果性不牺牲精度**）。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- 在线 merge tree 算法 + 与批量等价性：**L0**（纯算法，确定性，零信息增量；等价性是
  数学命题，由 property test 在 L1 管线层确认无 bug）。
- 因果 settle 判据：**L0**（merge tree 的数学属性，上文已证）。
- "settled bar 稳定 → 可进决策层"等经验断言：真实数据 L2/L3 前不声称（§8）。

设计约束（coding-style）
------------------------
- 对外暴露的快照（MergeBar / OnlineBarcode）：frozen + slots，immutable。
- OnlineMergeTree 是**显式 stateful 的流式累积器**（流式算法的本质，诚实声明，
  非补丁）——`update()` 原地推进内部 union-find 栈，不 mutate 调用方传入的数据，
  且 `current_barcode()` 返回 immutable 快照。
- 私有 `_Comp` 是 alive union-find 状态，用 __slots__，非 frozen（它就是活的状态）。

概念溯源标签
-----------
- 在线因果 merge tree [新缠论:候选——§7.5 升级方向]
- 因果 settle = 分型确认 [新缠论:候选——拓扑↔缠论同构]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol, Sequence, runtime_checkable

__all__ = [
    "MergeBar",
    "OnlineBarcode",
    "OnlineMergeTree",
    "PriceStream",
    "EntryState",
    "TrendHealth",
    "StopSignal",
    "LevelSwitchEvent",
]


# ====================================================================
# 对外快照类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class MergeBar:
    """在线 merge tree 中的一个 H0 持续同调特征。

    比批量版 `a_persistence_barcode.Bar` 多 **时间索引 + span**——因为在线版的核心
    产出之一是"特征覆盖多少根 K 线"（span → 缠论级别，见 a_level_detection）。

    Attributes
    ----------
    birth_price : float
        诞生价（局部极小 valley）。
    death_price : float
        死亡价（合并处的鞍点 peak）；全局分量为 finite_cap。
    persistence : float
        death_price − birth_price = 该摆动的 prominence = 该维度力度。
    birth_idx : int
        诞生 valley 的时间索引。
    death_idx : int | None
        死亡鞍点的时间索引；全局分量（封顶死亡）为 None。
    lo, hi : int
        死亡时分量覆盖的连续时间区间 [lo, hi]（闭区间）。
    settled : bool
        True = death 因果确定（未来不可改写）；False = alive（death 为当前估计）。
    """

    birth_price: float
    death_price: float
    persistence: float
    birth_idx: int
    death_idx: int | None
    lo: int
    hi: int
    settled: bool

    @property
    def span(self) -> int:
        """特征覆盖的 K 线根数 = hi − lo + 1。"""
        return self.hi - self.lo + 1

    @property
    def is_global(self) -> bool:
        """是否为全局分量（最低 valley，死亡值封顶）。"""
        return self.death_idx is None


@dataclass(frozen=True, slots=True)
class OnlineBarcode:
    """某一时刻的 barcode 快照 = settled bars + alive bars（immutable）。

    Attributes
    ----------
    settled_bars : tuple[MergeBar, ...]
        death 已因果确定的特征（persistence 锁定），persistence 降序。
    alive_bars : tuple[MergeBar, ...]
        仍在演化的特征，death_price 是**当前估计**（运行最高价封顶），persistence 降序。
    n_points : int
        已观测的 K 线数。
    """

    settled_bars: tuple[MergeBar, ...]
    alive_bars: tuple[MergeBar, ...]
    n_points: int

    @property
    def all_bars(self) -> tuple[MergeBar, ...]:
        """settled + alive 合并，persistence 降序。"""
        merged = [*self.settled_bars, *self.alive_bars]
        merged.sort(key=lambda b: b.persistence, reverse=True)
        return tuple(merged)


@runtime_checkable
class PriceStream(Protocol):
    """逐根产出 close 价的因果数据源的最小接口。

    任何按时间顺序逐根产出价格的源（实时行情 / 回放 / 序列迭代器）都可喂给
    OnlineMergeTree。本协议不要求随机访问——这是因果性的接口层体现。
    """

    def __iter__(self): ...


# ====================================================================
# 止损 / 走势健康度状态对象（frozen + slots，immutable）
# ====================================================================
#
# 认识论 + 缠论边界（必读，formalization-validity-domain + §5）：
# 本模块用 alive 分量 persistence 度量"大级别下跌是否还在创新低（幅度是否扩大）"。
# 缠论原文（第27/37/61课，原文考据 2026-05-27）：买点确认是**单调否定过程**——背驰段
# 是"被假设的"，由后续走势**创新低**否定（"对象否定对象"）；真背驰 = 创新低 + **力度
# 衰竭**，中继(假底) = 创新低 + **力度未减**。
#
# **关键有效域约束**：persistence ≈ 幅度 ∈ ker(D)（§5/239号），**只能算"创新低/幅度"
# 这个必要条件**，不能算"力度衰竭"（MACD 动量，§5 不可约维度）。故本模块产出的是
# **否定性（必要条件）信号**——能否定假买点（大级别还在创新低 → 次级别背驰不可靠），
# **不能肯定真买点**（需 MACD 力度，§5）。这与第44课"只有必要条件，没有充分条件"一致。
# 不声称比 MACD 止损"更准"——只声称提供一个**独立的、因果的、幅度维度**的否定过滤器。


@dataclass(frozen=True, slots=True)
class EntryState:
    """入场快照：在某根 K 线上、依据某个 alive 分量入场的不可变记录。

    Attributes
    ----------
    birth_idx : int
        入场依据的 alive 分量的 valley 索引（分量身份标识——它若 death 即被否定）。
    entry_persistence : float
        入场时该分量的 persistence（作为反向分量超越的阈值）。
    direction : int
        +1 = 做多（在下跌 alive 分量的底部抄底，赌反转向上）；-1 = 做空。
    n_at_entry : int
        入场时已观测 K 线数（区分"入场后新生"的分量）。
    """

    birth_idx: int
    entry_persistence: float
    direction: int
    n_at_entry: int


@dataclass(frozen=True, slots=True)
class TrendHealth:
    """主导 alive 分量的健康度快照（趋势是否还在延续/创新极值）。

    persistence_growth_rate > 0 ⟺ 主导 alive 分量 persistence 仍在增长 ⟺ 该方向走势
    仍在创新极值（幅度扩大）⟺ 趋势健康（未衰竭）。缠论映射：下跌中此值 > 0 → 次级别
    背驰买点更可能是**假底/中继**（大级别还在创新低）；停滞/转负 → 接近衰竭，买点
    可靠性上升（但真假最终判别需 MACD 力度，§5）。
    """

    dominant_persistence: float
    dominant_span: int
    dominant_birth_idx: int
    persistence_growth_rate: float   # 近 window 的 persistence 变化率（每根 K）
    suppression_ratio: float          # 主导 / 次大 alive persistence（段内压制）
    suppression_ratio_change: float   # 压制比相对 window 前的变化
    healthy: bool                     # growth_rate > 0（趋势仍在创新极值）


@dataclass(frozen=True, slots=True)
class StopSignal:
    """止损信号（否定性必要条件，非肯定性确认——见模块顶部边界说明）。

    triggered=True 表示入场依据被走势否定（缠论"对象否定对象"的拓扑形态）：
    - "entry_death"：入场分量已 death（它依托的支撑/背驰段被合并掉，结构破坏）。
    - "reverse_exceeds"：入场后新生的反向分量 persistence 超过入场阈值（更大的反向
      结构形成 = 创新极值否定了入场假设）。
    """

    triggered: bool
    reason: str          # 'none' | 'entry_death' | 'reverse_exceeds' | 'both'
    reverse_persistence: float
    entry_alive: bool    # 入场分量当前是否仍 alive（未 death）


@dataclass(frozen=True, slots=True)
class LevelSwitchEvent:
    """级别切换事件：主导 alive 分量 death + 新分量取代。

    switched=True ⟺ 当前主导 alive 分量的身份（birth_idx）与上一参照不同，
    且旧主导已进入 settled（death 确认）——主导级别发生因果切换。
    """

    switched: bool
    old_dominant_birth_idx: int | None
    new_dominant_birth_idx: int | None
    old_dominant_died: bool


# ====================================================================
# 私有：alive 分量（活的 union-find 状态，__slots__，非 frozen）
# ====================================================================


class _Comp:
    """一个 alive 连通分量（局部极小 valley）。

    这是流式 union-find 的活状态，被 update() 原地推进——它不是对外数据，
    不受 immutability 约束（它就是"正在变化的那个东西"）。

    覆盖区间 [lo, hi] **不在此追踪**——它由 death 值在 settle 时反推（见
    OnlineMergeTree._component_extent）：分量在 death⁻ 处 = 包含 valley 的、所有
    价格 < death 的极大连续段。这消除了鞍点/高端点的归属歧义，与批量定义同构。
    """

    __slots__ = ("val", "idx", "bottomed")

    def __init__(self, val: float, idx: int) -> None:
        self.val = val          # 诞生价（valley，可在初始下降中继续降低）
        self.idx = idx          # 诞生 valley 索引
        self.bottomed = False   # 是否已触底（开始上升 → valley 定格）


# ====================================================================
# 在线 merge tree（显式 stateful 流式累积器）
# ====================================================================


class OnlineMergeTree:
    """1D 价格序列的**在线/因果** sublevel-set H0 merge tree。

    用法（流式）::

        tree = OnlineMergeTree()
        for price in stream:                 # 每来一根 K 线
            newly_settled = tree.update(price)
            snap = tree.current_barcode()    # 任意时刻取快照
        final = tree.finalize()              # 序列结束，封顶全局分量

    与批量 `sublevel_h0_bars()` 的等价性见模块 docstring（property-based 验证）。
    """

    __slots__ = (
        "_prices", "_stack", "_barrier_val", "_barrier_idx", "_settled",
        "_prev", "_running_max", "_dom_hist",
    )

    def __init__(self) -> None:
        self._prices: list[float] = []
        self._stack: list[_Comp] = []
        self._barrier_val: list[float] = []   # 屏障 peak 值，严格递减（栈底→栈顶）
        self._barrier_idx: list[int] = []      # 屏障 peak 索引，与 _barrier_val 平行
        self._settled: list[MergeBar] = []
        self._prev: float | None = None
        self._running_max: float = float("-inf")
        # 主导 alive 分量历史 (n_points, dom_persistence, dom_span, dom_birth_idx)
        # —— 供 trend_health 计算 persistence 增长率（趋势健康度）。
        self._dom_hist: list[tuple[int, float, int, int]] = []

    # ---------------------------------------------------------------
    # 流式入口
    # ---------------------------------------------------------------

    def update(self, price: float) -> tuple[MergeBar, ...]:
        """喂入一根新 K 线（close 价），增量推进 merge tree。

        Returns
        -------
        tuple[MergeBar, ...]
            **本次调用新 settle** 的特征（可能为空）。死亡值因果确定，未来不变。
        """
        p = float(price)
        t = len(self._prices)
        self._prices.append(p)
        self._running_max = max(self._running_max, p)

        if not self._stack:
            self._stack.append(_Comp(p, t))
            self._prev = p
            self._record_dom()
            return ()

        prev = self._prev
        assert prev is not None
        settle_start = len(self._settled)

        # (1) 上升确认栈顶分量触底（valley 定格）
        if p > prev:
            self._stack[-1].bottomed = True

        # (2) cascade settle：上升越过栈顶屏障 → 合并（这正是因果 settle 时刻）
        #     不变量：_barrier_val 严格递减 → 栈顶屏障最小 → 自小到大依次触及。
        while len(self._stack) >= 2 and p >= self._barrier_val[-1]:
            self._merge_top()

        # (3) 放置当前点
        top = self._stack[-1]
        if not top.bottomed:
            # 仍在初始下降（或持平） → 可能降低 valley
            if p < top.val:
                top.val = p
                top.idx = t
        elif p >= prev:
            # 已触底后上升/持平 → 无操作（区间在 settle 时反推）
            pass
        else:
            # 已触底后回落 → t-1 是 peak（鞍点），新分量在此诞生。
            self._barrier_val.append(prev)
            self._barrier_idx.append(t - 1)
            self._stack.append(_Comp(p, t))

        self._prev = p
        self._record_dom()
        return tuple(self._settled[settle_start:])

    def _record_dom(self) -> None:
        """记录当前主导 alive 分量 (n, persistence, span, birth_idx) 到历史。

        主导 alive = persistence 最大的 alive 分量。cap（运行最高价）对所有 alive 共享，
        故 persistence = cap − val 最大者 = valley 最低者。
        """
        if not self._stack:
            return
        cap = self._running_max
        dom = min(self._stack, key=lambda c: c.val)  # valley 最低 = persistence 最大
        lo, hi = self._component_extent(dom.idx, cap) if cap > dom.val else (dom.idx, dom.idx)
        self._dom_hist.append((len(self._prices), cap - dom.val, hi - lo + 1, dom.idx))

    def _component_extent(self, birth_idx: int, death: float) -> tuple[int, int]:
        """分量在 death⁻ 处的时间区间 = 含 valley 的、价格 < death 的极大连续段。

        这是 sublevel 连通分量的精确定义（与批量 sublevel_h0_bars 同构），用已观测
        的 prices 前缀确定性反推——完全因果（只读过去）。
        """
        prices = self._prices
        lo = birth_idx
        while lo - 1 >= 0 and prices[lo - 1] < death:
            lo -= 1
        hi = birth_idx
        n = len(prices)
        while hi + 1 < n and prices[hi + 1] < death:
            hi += 1
        return lo, hi

    def _merge_top(self) -> None:
        """在栈顶屏障处合并：年轻分量死亡（settle），elder 吸收。"""
        bv = self._barrier_val.pop()
        bi = self._barrier_idx.pop()
        right = self._stack.pop()
        left = self._stack[-1]
        # elder rule：valley 更低者存活；相等时取 right 为年轻（一致即可）
        if right.val >= left.val:
            younger, elder = right, left
        else:
            younger, elder = left, right
        lo, hi = self._component_extent(younger.idx, bv)
        self._settled.append(
            MergeBar(
                birth_price=younger.val,
                death_price=bv,
                persistence=bv - younger.val,
                birth_idx=younger.idx,
                death_idx=bi,
                lo=lo,
                hi=hi,
                settled=True,
            )
        )
        self._stack[-1] = elder

    # ---------------------------------------------------------------
    # 快照 / 收尾
    # ---------------------------------------------------------------

    def current_barcode(self) -> OnlineBarcode:
        """当前时刻的 barcode 快照 = 已 settled 特征 + alive 特征估计。

        alive 特征的 death 尚未因果确定（在未来）。本快照用**当前运行最高价**作为
        alive 死亡值的估计（persistence 上界）——它是"假设序列在此刻被一个全局最高
        屏障封顶"的结果。alive bar 的 persistence 会随未来 K 线改写（这正是 §7.5
        指出的稳定性代价），settled bar 不会。
        """
        settled = sorted(self._settled, key=lambda b: b.persistence, reverse=True)
        cap = self._running_max if self._stack else 0.0
        n = len(self._prices)
        last = len(self._stack) - 1
        alive: list[MergeBar] = []
        # alive 分量的 death 在未来（§7.5 不稳定性）：用当前运行最高价 cap 作估计上界。
        # 区间用已确认的左右屏障平铺 [0, n-1]——左端=左屏障后一格（栈底=0），
        # 右端=右屏障前一格（栈顶开放=n-1）。
        for k, c in enumerate(self._stack):
            lo = 0 if k == 0 else self._barrier_idx[k - 1] + 1
            hi = n - 1 if k == last else self._barrier_idx[k] - 1
            alive.append(
                MergeBar(
                    birth_price=c.val,
                    death_price=cap,
                    persistence=cap - c.val,
                    birth_idx=c.idx,
                    death_idx=None,
                    lo=lo,
                    hi=hi,
                    settled=False,
                )
            )
        alive.sort(key=lambda b: b.persistence, reverse=True)
        return OnlineBarcode(
            settled_bars=tuple(settled),
            alive_bars=tuple(alive),
            n_points=len(self._prices),
        )

    def finalize(self, *, finite_cap: float | None = None) -> OnlineBarcode:
        """序列结束：合并所有剩余 alive 分量，封顶全局分量。

        路径图最终全连通 → 恰好剩一个全局分量（最低 valley），死亡值封顶
        `finite_cap`（默认 max(prices)，使全局 bar persistence = 价格全幅）。
        其余剩余分量在各自屏障处合并（自小屏障到大屏障 = elder rule 顺序）。

        finalize 后所有特征都是 settled（序列终止 = 一切因果确定）。返回的
        OnlineBarcode 的 settled_bars 应与批量 `sublevel_h0_bars()` 的有限 + 全局
        bar 多重集一致。
        """
        if not self._stack:
            return OnlineBarcode(settled_bars=(), alive_bars=(), n_points=0)

        # 屏障自小到大（栈顶到栈底）依次合并
        while len(self._stack) >= 2:
            self._merge_top()

        g = self._stack[0]
        cap = max(self._prices) if finite_cap is None else float(finite_cap)
        # 全局分量覆盖整个序列（路径图全连通）
        global_bar = MergeBar(
            birth_price=g.val,
            death_price=cap,
            persistence=cap - g.val,
            birth_idx=g.idx,
            death_idx=None,
            lo=0,
            hi=len(self._prices) - 1,
            settled=True,
        )
        all_settled = sorted(
            [*self._settled, global_bar], key=lambda b: b.persistence, reverse=True
        )
        return OnlineBarcode(
            settled_bars=tuple(all_settled),
            alive_bars=(),
            n_points=len(self._prices),
        )

    # ---------------------------------------------------------------
    # 止损 / 走势健康度（流式调用——在 finalize 之前的活树上）
    # ---------------------------------------------------------------

    def trend_health(self, *, window: int = 5) -> TrendHealth | None:
        """主导 alive 分量的健康度：persistence 增长率 + suppression ratio 变化。

        persistence_growth_rate = (当前主导 persistence − window 根前) / 实际跨度。
        > 0 ⟺ 主导方向仍在创新极值（趋势健康/未衰竭）。suppression_ratio = 主导 /
        次大 alive persistence（段内压制强度）。数据不足（<2 个历史点）返回 None。

        缠论边界（见模块顶部）：本量度的是**幅度维度**（创新低），是真假买点的**必要
        条件过滤器**，不是 MACD 力度衰竭判据（§5）。
        """
        if len(self._dom_hist) < 2 or not self._stack:
            return None
        n_now, dom_p, dom_span, dom_idx = self._dom_hist[-1]
        ref_i = max(0, len(self._dom_hist) - 1 - window)
        n_ref, dom_p_ref, _, _ = self._dom_hist[ref_i]
        dn = n_now - n_ref
        growth = (dom_p - dom_p_ref) / dn if dn > 0 else 0.0

        cap = self._running_max
        alive_pers = sorted((cap - c.val for c in self._stack), reverse=True)
        sr = alive_pers[0] / alive_pers[1] if len(alive_pers) >= 2 and alive_pers[1] > 0 else float("inf")
        # 压制比变化：用 window 前的快照难以无状态重建，这里用当前 vs 主导历史的代理——
        # 主导 persistence 增长而次级别相对停滞 → sr 上升。简化为 growth 的符号一致量。
        sr_change = growth  # 代理：主导增长即压制增强（同号），诚实标注为代理量
        return TrendHealth(
            dominant_persistence=dom_p,
            dominant_span=dom_span,
            dominant_birth_idx=dom_idx,
            persistence_growth_rate=growth,
            suppression_ratio=sr,
            suppression_ratio_change=sr_change,
            healthy=growth > 0.0,
        )

    def stop_signal(self, entry: EntryState) -> StopSignal:
        """止损信号：入场分量 death 或反向新分量 persistence 超过入场阈值。

        否定性必要条件信号（"对象否定对象"的拓扑形态，见模块顶部）：
        - entry_death：入场分量（birth_idx==entry.birth_idx）已进入 settled（death 确认）
          → 入场依托的结构被合并掉。
        - reverse_exceeds：入场后新生（birth_idx > entry.birth_idx）的 alive 分量
          persistence > entry.entry_persistence → 更大的反向结构形成，否定入场假设。
        """
        entry_died = any(b.birth_idx == entry.birth_idx for b in self._settled)
        entry_alive = any(c.idx == entry.birth_idx for c in self._stack)
        cap = self._running_max
        reverse_p = 0.0
        for c in self._stack:
            if c.idx > entry.birth_idx:
                reverse_p = max(reverse_p, cap - c.val)
        reverse_exceeds = reverse_p > entry.entry_persistence
        if entry_died and reverse_exceeds:
            reason = "both"
        elif entry_died:
            reason = "entry_death"
        elif reverse_exceeds:
            reason = "reverse_exceeds"
        else:
            reason = "none"
        return StopSignal(
            triggered=entry_died or reverse_exceeds,
            reason=reason,
            reverse_persistence=reverse_p,
            entry_alive=entry_alive,
        )

    def level_switch_event(self, prev_dominant_birth_idx: int | None) -> LevelSwitchEvent:
        """级别切换：主导 alive 分量 death + 新分量取代。

        与上一参照（prev_dominant_birth_idx，通常来自上一次 trend_health 的
        dominant_birth_idx）比较：当前主导身份变化 + 旧主导已 death ⟺ 主导级别切换。
        """
        if not self._stack:
            return LevelSwitchEvent(False, prev_dominant_birth_idx, None, False)
        dom = min(self._stack, key=lambda c: c.val)
        new_idx = dom.idx
        old_died = prev_dominant_birth_idx is not None and any(
            b.birth_idx == prev_dominant_birth_idx for b in self._settled
        )
        switched = (
            prev_dominant_birth_idx is not None
            and new_idx != prev_dominant_birth_idx
            and old_died
        )
        return LevelSwitchEvent(
            switched=switched,
            old_dominant_birth_idx=prev_dominant_birth_idx,
            new_dominant_birth_idx=new_idx,
            old_dominant_died=old_died,
        )

    # ---------------------------------------------------------------
    # 便捷构造
    # ---------------------------------------------------------------

    @classmethod
    def from_prices(
        cls, prices: Sequence[float], *, finite_cap: float | None = None
    ) -> OnlineBarcode:
        """对完整序列逐根 update 后 finalize——便于与批量版对比。

        注意：这仍是**因果**处理（逐根喂入），只是一次跑完整段。等价性测试用它。
        """
        tree = cls()
        for p in prices:
            tree.update(p)
        return tree.finalize(finite_cap=finite_cap)
