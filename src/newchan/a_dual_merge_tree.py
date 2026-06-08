"""A 系统 — 双树法（Dual Merge Tree）K 线 PH 引擎：把 PH 从 close 曲线升级到 K 线 HL。

存在论位置（§7.5 在线 merge tree 的 K 线化）
---------------------------------------------
`a_online_persistence` 的 OnlineMergeTree 只吃**一维 close 序列**——它把 K 线压成一个
点，丢掉了 high/low 的振幅信息。缠论的分型/笔建立在 K 线的**顶（high 局部极大）与底
（low 局部极小）**上，而 sublevel merge tree 只能"看见"valley（局部极小）。本模块用
**两棵同一引擎的 OnlineMergeTree**，分别编码 K 线的两端：

  T_low  = OnlineMergeTree(low)      —— low 的 sublevel：valley = 真实**底**，
                                        settle = 底被反弹**涨过**屏障确认（与单树 close 同向）。
  T_high = OnlineMergeTree(-high)    —— -high 的 sublevel：valley = high 的 peak = 真实**顶**，
                                        settle = -high 涨过屏障 = high **跌破**屏障 → 顶被杀死。

取负是把"顶"翻译进 sublevel 语言的**唯一**手段（merge tree 只合并 valley）。取负在
**输入时**施加（喂 -high），在**输出时**还原（用户看到真实 high）——本模块所有对外
价格都是真实价。

settle 语义的方向对称（核心，必读）
-----------------------------------
| 树     | 喂入   | valley 含义 | settle 触发        | 缠论同构          |
|--------|--------|-------------|--------------------|-------------------|
| T_low  | low    | 真实底      | 价格**涨过** trigger | 底分型后续 K 线确认 |
| T_high | -high  | 真实顶      | high **跌破** trigger | 顶分型后续 K 线确认 |

persistence 在取负下**不变**：T_high 分量 persistence = (-high 屏障) − (-high peak)
= high_peak − high_屏障 = **真实下跌幅度**（顶到杀死价的落差）。T_low 分量 persistence
= 真实反弹幅度。两者都 ≥ 0，含义对称。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- 双树构建 + 取负还原 + settle 方向翻转：**L0**（纯算法，确定性；OnlineMergeTree 的
  L0 等价性继承，本模块只做坐标变换与组合）。
- strokes()/containment_pairs() 作为缠论"笔/包含"的拓扑形式：**L0 算法 + 候选同构**
  （是否真等于缠论指标的笔，需 B 部分真实数据对比 → L2/L3，本模块不声称）。

设计约束（coding-style / no-patch）
-----------------------------------
- 对外快照（DualComponent / DualSettleTrigger / StrokeCandidate / ContainmentPair）：
  frozen + slots，immutable。
- OnlineMergeTree 作为**黑盒**调用（只用其公开 API：update / current_barcode /
  alive_settle_thresholds / settled_bars / alive_bars），不触碰内部状态，不修改其源码。
- 取负是**确定性坐标变换**，不是概率范式（llm-role-boundary 不适用——这是数值计算）。

概念溯源标签
-----------
- 双树法 K 线 PH [新缠论:候选——§7.5 在线树的 HL 升级]
- 顶 settle = high 跌破确认 / 底 settle = low 涨过确认 [新缠论:候选——分型确认的拓扑对称]
- 双树交替 death = 笔候选 [新缠论:候选——待 B 部分 L2 对比]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Mapping, Sequence

from newchan.a_online_persistence import MergeBar, OnlineMergeTree
from newchan.a_settle_trigger import settle_triggers

__all__ = [
    "DualComponent",
    "DualSettleTrigger",
    "StrokeCandidate",
    "ContainmentPair",
    "DualMergeTree",
]


# ====================================================================
# 对外快照类型（frozen + slots，immutable，全部真实价格坐标）
# ====================================================================


@dataclass(frozen=True, slots=True)
class DualComponent:
    """一棵树的一个分量在**真实价格**坐标下的视图（顶或底）。

    Attributes
    ----------
    kind : str
        'top'（来自 T_high）或 'bottom'（来自 T_low）。
    extreme_price : float
        极值真实价：顶 = high peak，底 = low valley。
    confirm_price : float
        死亡屏障真实价：顶 = 跌破确认价（< extreme），底 = 涨过确认价（> extreme）。
        alive 分量为**当前估计**（顶=至今最低 high，底=至今最高估计），settled 为定值。
    persistence : float
        摆动幅度 = |extreme_price − confirm_price|（顶=下跌幅度，底=反弹幅度），取负不变。
    birth_idx : int
        极值点时间索引（分量身份）。
    death_idx : int | None
        死亡屏障时间索引；全局分量为 None。
    lo, hi : int
        分量覆盖的连续时间区间 [lo, hi]（闭区间）。
    settled : bool
        True = death 因果确定（顶被杀/底被确认，不可改写）；False = alive。
    """

    kind: str
    extreme_price: float
    confirm_price: float
    persistence: float
    birth_idx: int
    death_idx: int | None
    lo: int
    hi: int
    settled: bool

    @property
    def span(self) -> int:
        """覆盖 K 线根数。"""
        return self.hi - self.lo + 1

    @property
    def is_global(self) -> bool:
        return self.death_idx is None


@dataclass(frozen=True, slots=True)
class DualSettleTrigger:
    """一个 alive 分量的 settle 触发快照（真实价格，方向已按 kind 还原）。

    Attributes
    ----------
    kind : str
        'top' | 'bottom'。
    birth_idx : int
        分量身份（极值时间索引）。
    extreme_price : float
        极值真实价（顶 peak / 底 valley）。
    trigger_price : float | None
        顶 = high 跌破此价确认顶死；底 = low 涨过此价确认底。全局分量为 None。
    trigger_persistence : float | None
        |trigger_price − extreme_price| = 触发所需幅度。全局为 None。
    last_price : float
        顶 = 最近一根 high；底 = 最近一根 low。
    gap : float | None
        还需移动多少触发：顶 = 还需下跌幅度，底 = 还需上涨幅度。
        ≤ 0 表示当前价已穿越屏障（本轮即将/已 settle）。全局为 None。
    is_dominant : bool
        是否为该树 persistence 最大的 alive 分量（最深的腿）。
    can_settle : bool
        False ⟺ 全局极值分量（顶不可被跌破 settle / 底不可被反弹 settle，只能反向否定）。
    reversal_amplitude : float | None
        仅全局分量非 None：要否定该结构，反向腿 persistence 需超过的幅度。
    """

    kind: str
    birth_idx: int
    extreme_price: float
    trigger_price: float | None
    trigger_persistence: float | None
    last_price: float
    gap: float | None
    is_dominant: bool
    can_settle: bool
    reversal_amplitude: float | None


@dataclass(frozen=True, slots=True)
class StrokeCandidate:
    """从两棵树的极值交替序列提取的一条"笔"候选。

    端点 = 两棵树全部分量（settled + alive）的极值点。相邻同类极值按 alternation
    折叠（保留更极端者 = 缠论分型交替的拓扑形式），相邻异类极值成一笔候选。

    **候选 ≠ 已确认笔**：settled 是端点的**确认属性**（start_confirmed/end_confirmed），
    不是笔存在性的过滤器——趋势中一侧极值持续 alive 是真实结构，排除它会丢掉整个
    单向笔列（no-patch：不在提取期预判确认）。下游要"已确认笔"则过滤
    start_confirmed and end_confirmed。

    Attributes
    ----------
    direction : str
        'up'（底→顶）或 'down'（顶→底）。
    start_idx, end_idx : int
        两端极值的时间索引（start_idx < end_idx）。
    start_price, end_price : float
        两端真实极值价。
    amplitude : float
        |end_price − start_price|。
    start_kind, end_kind : str
        端点类型（'top'/'bottom'），与 direction 自洽。
    start_confirmed, end_confirmed : bool
        端点对应分量是否已 settled（顶被跌破确认 / 底被涨过确认）。
    """

    direction: str
    start_idx: int
    end_idx: int
    start_price: float
    end_price: float
    amplitude: float
    start_kind: str
    end_kind: str
    start_confirmed: bool
    end_confirmed: bool


@dataclass(frozen=True, slots=True)
class ContainmentPair:
    """T_high 分量与 T_low 分量的时间区间嵌套关系（同一时间窗内）。

    relation ∈ {'high_contains_low', 'low_contains_high', 'equal'}：
    描述顶结构与底结构在时间轴上的包含——缠论"包含关系/中枢覆盖"的拓扑前形态。
    """

    high: DualComponent
    low: DualComponent
    relation: str


# ====================================================================
# 双树引擎（两棵黑盒 OnlineMergeTree 的组合）
# ====================================================================


def _restore_top(bar: MergeBar) -> DualComponent:
    """把 -high 空间的 MergeBar 还原为真实顶视图。

    -high valley = high peak → extreme = -birth_price；
    -high 屏障   = high 跌破价 → confirm = -death_price；persistence 不变。
    """
    return DualComponent(
        kind="top",
        extreme_price=-bar.birth_price,
        confirm_price=-bar.death_price,
        persistence=bar.persistence,
        birth_idx=bar.birth_idx,
        death_idx=bar.death_idx,
        lo=bar.lo,
        hi=bar.hi,
        settled=bar.settled,
    )


def _restore_bottom(bar: MergeBar) -> DualComponent:
    """T_low 的 MergeBar 直接是真实底视图（low 不取负）。"""
    return DualComponent(
        kind="bottom",
        extreme_price=bar.birth_price,
        confirm_price=bar.death_price,
        persistence=bar.persistence,
        birth_idx=bar.birth_idx,
        death_idx=bar.death_idx,
        lo=bar.lo,
        hi=bar.hi,
        settled=bar.settled,
    )


def _extract_hl(bar: Any) -> tuple[float, float]:
    """从一个 OHLC bar（dict / 对象 / (h,l) 兼容）提取 (high, low)。"""
    if isinstance(bar, Mapping):
        return float(bar["high"]), float(bar["low"])
    if hasattr(bar, "high") and hasattr(bar, "low"):
        return float(bar.high), float(bar.low)
    raise TypeError(
        "update_bar 需要含 high/low 的 Mapping 或对象；逐根两值请用 update(high, low)。"
    )


class DualMergeTree:
    """K 线双树 PH 引擎：T_high(-high) 追顶的死亡史，T_low(low) 追底的确认史。

    用法（流式）::

        dt = DualMergeTree()
        for h, l in zip(highs, lows):
            dt.update(h, l)
        tops = dt.alive_components_high()      # 真实价
        top_ladder = dt.settle_triggers_high() # 顶 settle 阶梯（跌破确认）
        bot_ladder = dt.settle_triggers_low()  # 底 settle 阶梯（涨过确认）
        bi = dt.strokes()                      # 笔候选

    OnlineMergeTree 黑盒：只喂 -high / low，只读其公开快照。
    """

    __slots__ = ("_t_high", "_t_low")

    def __init__(self) -> None:
        self._t_high = OnlineMergeTree()  # 喂 -high
        self._t_low = OnlineMergeTree()   # 喂 low

    # ---------------------------------------------------------------
    # 流式入口
    # ---------------------------------------------------------------

    def update(self, high: float, low: float) -> None:
        """喂入一根 K 线的 (high, low)。high 取负进 T_high，low 直接进 T_low。"""
        self._t_high.update(-float(high))
        self._t_low.update(float(low))

    def update_bar(self, bar: Any) -> None:
        """喂入一个 OHLC bar 对象（dict / 含 .high/.low 的对象）。"""
        h, l = _extract_hl(bar)
        self.update(h, l)

    @classmethod
    def from_ohlc(
        cls, highs: Sequence[float], lows: Sequence[float]
    ) -> "DualMergeTree":
        """从平行的 highs / lows 序列批量构建（仍是逐根因果喂入）。"""
        if len(highs) != len(lows):
            raise ValueError(f"highs({len(highs)}) 与 lows({len(lows)}) 长度不一致")
        dt = cls()
        for h, l in zip(highs, lows):
            dt.update(h, l)
        return dt

    @classmethod
    def from_dataframe(cls, df: Any) -> "DualMergeTree":
        """从 OHLC DataFrame 构建（列名 high/low 或 High/Low）。"""
        cols = {c.lower(): c for c in df.columns}
        if "high" not in cols or "low" not in cols:
            raise KeyError("DataFrame 需含 high/low 列（大小写不敏感）")
        highs = [float(x) for x in df[cols["high"]].tolist()]
        lows = [float(x) for x in df[cols["low"]].tolist()]
        return cls.from_ohlc(highs, lows)

    # ---------------------------------------------------------------
    # 黑盒树访问（供分析脚本对照单树法）
    # ---------------------------------------------------------------

    @property
    def t_high(self) -> OnlineMergeTree:
        """T_high 黑盒（-high 坐标，价需取负还原）。"""
        return self._t_high

    @property
    def t_low(self) -> OnlineMergeTree:
        """T_low 黑盒（low 坐标，真实价）。"""
        return self._t_low

    # ---------------------------------------------------------------
    # 分量读出（真实价）
    # ---------------------------------------------------------------

    def alive_components_high(self) -> tuple[DualComponent, ...]:
        """T_high 的 alive 顶分量（真实价，persistence 降序）。"""
        return tuple(
            _restore_top(b) for b in self._t_high.current_barcode().alive_bars
        )

    def alive_components_low(self) -> tuple[DualComponent, ...]:
        """T_low 的 alive 底分量（真实价，persistence 降序）。"""
        return tuple(
            _restore_bottom(b) for b in self._t_low.current_barcode().alive_bars
        )

    def settled_components_high(self) -> tuple[DualComponent, ...]:
        """T_high 已死亡（顶被跌破确认）的顶分量（真实价）。"""
        return tuple(
            _restore_top(b) for b in self._t_high.current_barcode().settled_bars
        )

    def settled_components_low(self) -> tuple[DualComponent, ...]:
        """T_low 已确认（底被反弹涨过）的底分量（真实价）。"""
        return tuple(
            _restore_bottom(b) for b in self._t_low.current_barcode().settled_bars
        )

    # ---------------------------------------------------------------
    # settle 阶梯（方向已按 kind 还原）
    # ---------------------------------------------------------------

    def settle_triggers_high(self) -> tuple[DualSettleTrigger, ...]:
        """顶的 settle 阶梯：每个 alive 顶要被确认死亡，high 需**跌破**多少。

        从 settle_triggers(T_high)（-high 坐标）还原：
        - trigger_price_real = −settle_price_(-high)（高价坐标，< extreme = 跌破）。
        - last_price_real    = −last_(-high)（最近 high）。
        - gap（还需下跌）     = st.gap_to_settle（-high 需上涨量 = high 需下跌量，等值）。
        """
        out: list[DualSettleTrigger] = []
        for st in settle_triggers(self._t_high):
            extreme = -st.birth_price
            if st.can_settle_by_rebound and st.settle_price is not None:
                trigger_price: float | None = -st.settle_price
                trig_pers: float | None = abs(trigger_price - extreme)
                gap: float | None = st.gap_to_settle  # high 还需下跌量
            else:
                trigger_price = None
                trig_pers = None
                gap = None
            out.append(
                DualSettleTrigger(
                    kind="top",
                    birth_idx=st.birth_idx,
                    extreme_price=extreme,
                    trigger_price=trigger_price,
                    trigger_persistence=trig_pers,
                    last_price=-st.last_price,
                    gap=gap,
                    is_dominant=st.is_dominant,
                    can_settle=st.can_settle_by_rebound,
                    reversal_amplitude=st.reversal_amplitude,
                )
            )
        return tuple(out)

    def settle_triggers_low(self) -> tuple[DualSettleTrigger, ...]:
        """底的 settle 阶梯：每个 alive 底要被确认，low 需**涨过**多少。

        T_low 与单树 close 引擎同坐标，直接映射 SettleTrigger（涨过方向）。
        """
        out: list[DualSettleTrigger] = []
        for st in settle_triggers(self._t_low):
            extreme = st.birth_price
            if st.can_settle_by_rebound and st.settle_price is not None:
                trigger_price: float | None = st.settle_price
                trig_pers: float | None = abs(trigger_price - extreme)
                gap: float | None = st.gap_to_settle  # low 还需上涨量
            else:
                trigger_price = None
                trig_pers = None
                gap = None
            out.append(
                DualSettleTrigger(
                    kind="bottom",
                    birth_idx=st.birth_idx,
                    extreme_price=extreme,
                    trigger_price=trigger_price,
                    trigger_persistence=trig_pers,
                    last_price=st.last_price,
                    gap=gap,
                    is_dominant=st.is_dominant,
                    can_settle=st.can_settle_by_rebound,
                    reversal_amplitude=st.reversal_amplitude,
                )
            )
        return tuple(out)

    # ---------------------------------------------------------------
    # containment：高/低分量时间区间嵌套
    # ---------------------------------------------------------------

    def containment_pairs(self) -> tuple[ContainmentPair, ...]:
        """当前快照下，T_high 与 T_low 分量的时间区间嵌套对（settled + alive）。

        对每个 (顶分量, 底分量)，若其 [lo,hi] 互相包含则记一对。同区间记 'equal'。
        相离/相交但不包含 → 不记（缠论"包含关系"要求一方区间含于另一方）。

        认识论：L0 区间运算。"嵌套 = 缠论包含/中枢覆盖"是候选同构（待 L2 验证）。
        """
        highs = [
            _restore_top(b)
            for b in (
                self._t_high.current_barcode().settled_bars
                + self._t_high.current_barcode().alive_bars
            )
        ]
        lows = [
            _restore_bottom(b)
            for b in (
                self._t_low.current_barcode().settled_bars
                + self._t_low.current_barcode().alive_bars
            )
        ]
        pairs: list[ContainmentPair] = []
        for h in highs:
            for l in lows:
                rel = _containment(h.lo, h.hi, l.lo, l.hi)
                if rel is not None:
                    pairs.append(ContainmentPair(high=h, low=l, relation=rel))
        return tuple(pairs)

    # ---------------------------------------------------------------
    # strokes：交替 death 事件 → 笔候选
    # ---------------------------------------------------------------

    def strokes(self, *, min_amplitude: float = 0.0) -> tuple[StrokeCandidate, ...]:
        """从两棵树的全部极值（settled + alive）提取笔候选序列。

        步骤：
        1. 收集全部顶（T_high settled+alive，birth_idx=peak，price=-birth_price）与
           全部底（T_low settled+alive，birth_idx=valley，price=birth_price），各带
           confirmed=settled 标记。
        2. 按时间排序，强制 alternation：相邻同类保留更极端者（更高顶 / 更低底）——
           缠论分型交替的拓扑形式（折叠时端点 confirmed 随保留者更新）。
        3. 相邻异类极值成一笔候选（底→顶=up，顶→底=down）。

        为什么用 settled+alive 而非 settled-only：趋势中一侧极值持续 alive（如单边
        上行时所有顶都未被跌破确认），settled-only 会丢掉整个单向笔列。settled 是端点
        **确认属性**（StrokeCandidate.start_confirmed/end_confirmed），不是存在性过滤。

        min_amplitude：过滤幅度过小的候选（默认 0，保留全部——候选非终笔，级别/跨度
        过滤交下游，no-patch：不在此处预判级别）。

        认识论：L0 算法。"= 缠论笔" 是候选同构，需 B 部分与缠论指标 L2 对比。
        """
        snap_h = self._t_high.current_barcode()
        snap_l = self._t_low.current_barcode()
        # (birth_idx, kind, price, confirmed)
        extremes: list[tuple[int, str, float, bool]] = []
        for b in (*snap_h.settled_bars, *snap_h.alive_bars):
            extremes.append((b.birth_idx, "top", -b.birth_price, b.settled))
        for b in (*snap_l.settled_bars, *snap_l.alive_bars):
            extremes.append((b.birth_idx, "bottom", b.birth_price, b.settled))
        # 时间排序（同 idx 时 top 在前，确定性 tie-break）
        extremes.sort(key=lambda e: (e[0], 0 if e[1] == "top" else 1))

        kept: list[tuple[int, str, float, bool]] = []
        for item in extremes:
            idx, kind, price, conf = item
            if kept and kept[-1][1] == kind:
                _, _, p_price, _ = kept[-1]
                more_extreme = price > p_price if kind == "top" else price < p_price
                if more_extreme:
                    kept[-1] = item
            else:
                kept.append(item)

        strokes: list[StrokeCandidate] = []
        for a, b in zip(kept, kept[1:]):
            a_idx, a_kind, a_price, a_conf = a
            b_idx, b_kind, b_price, b_conf = b
            amp = abs(b_price - a_price)
            if amp < min_amplitude:
                continue
            strokes.append(
                StrokeCandidate(
                    direction="up" if a_kind == "bottom" else "down",
                    start_idx=a_idx,
                    end_idx=b_idx,
                    start_price=a_price,
                    end_price=b_price,
                    amplitude=amp,
                    start_kind=a_kind,
                    end_kind=b_kind,
                    start_confirmed=a_conf,
                    end_confirmed=b_conf,
                )
            )
        return tuple(strokes)


def _containment(
    a_lo: int, a_hi: int, b_lo: int, b_hi: int
) -> str | None:
    """高区间 [a_lo,a_hi] 与低区间 [b_lo,b_hi] 的包含关系。

    返回 'high_contains_low' / 'low_contains_high' / 'equal' / None（不包含）。
    """
    if a_lo == b_lo and a_hi == b_hi:
        return "equal"
    if a_lo <= b_lo and b_hi <= a_hi:
        return "high_contains_low"
    if b_lo <= a_lo and a_hi <= b_hi:
        return "low_contains_high"
    return None
