"""K4 C 路径 — 六边同步分析（不压缩成极性 S）。

与 M2（``k4_integration.py``）的本质区别
----------------------------------------
M2 路径把 K4 压缩成极性指数 S∈[-3,+3] 与 ω regime，三条派生边被丢弃、六条边的
**个体身份被抹平**。C 路径相反：六条边各自独立跑缠论递归引擎，保留每条边的
买卖点与走势方向读数，再用 K4 的**圈闭合冗余**做纠错，最后做**联合读数选股**。

K4 图论结构（本实例顶点 = {ES, GC, CL, $}）
-------------------------------------------
- 主边（连接 $，构成生成树，3 自由度）：ES/$, GC/$, CL/$
- 派生边（资产三角 ES-GC-CL 的三条弦，每条造一个独立闭合圈）：ES/GC, ES/CL, GC/CL

4 顶点的生成树只需 3 条边；3 条派生边是冗余的"弦"。于是每个资产对有 **3 条独立
估计路径**（直接读、过 $、过第三资产），构成纠错码式的多数表决——这是"六边各自独立
跑"相对"3 条派生边由 3 条主边代数推导"的全部价值：冗余 → 可检错可纠错。

圈闭合恒等式（对数收益层，构造上恒成立）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    r(A/B) = r(A/$) − r(B/$)            （过现金）
    r(ES/CL) = r(ES/GC) + r(GC/CL)      （过第三资产）

价格比值层这些恒等式**构造上必然成立**。但六条边各自独立跑引擎得到的 **σ（走势方向
读数）** 是离散符号 {+1,0,−1}，符号不满足加法恒等，故独立读数之间可能矛盾——
**矛盾即可纠错信号**（噪声/级别错配在某条边产生伪方向，被另外两条估计路径否决）。

认识论标注（formalization-validity-domain 规则）
-----------------------------------------------
- 圈闭合纠错机制（``closure_consensus``）：L0——纯代数一致性运算，作用于已有读数，
  零信息增量（不依赖数据，是 K4 图结构的逻辑推论）。
- 单边的 σ / 买卖点读数：L2（真实数据，缠论引擎产出，可证伪）。
- 联合读数选股的 alpha：L2，由 ``analysis/m2_path_c_multilevel_backtest.py`` 检验，可产生否定性结果。

谱系引用
--------
- 527号：config σ 已结算为走势方向态（σ 可 −1→+1 跳变）。本模块读 σ 为走势方向态。
- project_omega_regime_falsified：ω→美股方向曾被反向证伪。C 路径**不走 ω 压缩**，
  直接读 ES-incident 边，是对该否定性结论的结构性回应（不依赖被证伪的 ω→regime 映射）。
- 526号 a0 递归本体论区分；525号 level_id 为构造标签非有效性判据。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Literal, Protocol

from newchan.topology.config_space import WalkDirection


# ═══════════════════════════════════════════════════════════════
# 顶点与边（本 K4 实例：ES/GC/CL/$）
# ═══════════════════════════════════════════════════════════════


class CVertex(Enum):
    """C 路径 K4 的四个顶点。"""

    EQUITY = "ES"   # 动产（股指期货 ES）
    GOLD = "GC"     # 商品-金（黄金期货 GC）
    OIL = "CL"      # 商品-油（原油期货 CL）
    CASH = "$"      # 现金/主权信用


@dataclass(frozen=True, slots=True)
class CEdge:
    """K4 的一条**有向**比价边（num/den）。

    方向语义：边代表比值序列 num/den。"buy"（底背驰/一买）= 比值见底 = num 相对 den
    走强 → 利好 num。``direction``=UP = num 相对 den 升值。
    """

    num: CVertex   # 分子
    den: CVertex   # 分母
    is_derived: bool

    @property
    def label(self) -> str:
        return f"{self.num.value}/{self.den.value}"


# 3 条主边（连接 CASH，生成树）
EDGE_ES_USD = CEdge(CVertex.EQUITY, CVertex.CASH, is_derived=False)
EDGE_GC_USD = CEdge(CVertex.GOLD, CVertex.CASH, is_derived=False)
EDGE_CL_USD = CEdge(CVertex.OIL, CVertex.CASH, is_derived=False)

# 3 条派生边（资产三角 ES-GC-CL 的弦）
EDGE_ES_GC = CEdge(CVertex.EQUITY, CVertex.GOLD, is_derived=True)
EDGE_ES_CL = CEdge(CVertex.EQUITY, CVertex.OIL, is_derived=True)
EDGE_GC_CL = CEdge(CVertex.GOLD, CVertex.OIL, is_derived=True)

MAIN_EDGES: tuple[CEdge, ...] = (EDGE_ES_USD, EDGE_GC_USD, EDGE_CL_USD)
DERIVED_EDGES: tuple[CEdge, ...] = (EDGE_ES_GC, EDGE_ES_CL, EDGE_GC_CL)
ALL_EDGES: tuple[CEdge, ...] = MAIN_EDGES + DERIVED_EDGES


# ═══════════════════════════════════════════════════════════════
# 单边读数协议（圈闭合纠错 + 联合读数共享的级别对齐契约）
# ═══════════════════════════════════════════════════════════════


class EdgeReadingLike(Protocol):
    """单边读数协议——圈闭合纠错与联合读数的级别对齐契约。

    **级别对齐由协议保证（任务要求"不混级别"）**：``closure_consensus`` 永远只在
    ``.sigma`` 之间做圈闭合检验，``joint_reading`` 永远只用 ``.bsp_signed_vote()``。
    "取哪个级别的 σ / 买卖点" 内化进具体 reading 类型，圈闭合逻辑零分支——
    单级别 reading 的 ``.sigma`` = level-0 走势方向；多级别 reading 的 ``.sigma``
    = 最高级别 L2 走势方向。同级别 σ 闭合，不存在跨级别混读的可能。
    """

    edge: CEdge

    @property
    def sigma(self) -> WalkDirection:
        """圈闭合检验使用的走势方向 σ（级别由实现类型固定）。"""
        ...

    def bsp_signed_vote(self) -> float:
        """买卖点 → 对**分子资产**的有向投票（买点利好分子→正；无则 0）。"""
        ...


@dataclass(frozen=True, slots=True)
class EdgeReading:
    """单条边的**单级别**缠论读数快照（六边各自独立跑引擎的产物）。

    级别语义：``direction`` / 买卖点均取 ``RecursiveOrchestrator`` 的 level-0
    （move_snapshot 走势 + BuySellPointEngine 买卖点）——这是 C 路径单级别版本。
    多级别版本见 ``LeveledEdgeReading``。

    Attributes
    ----------
    edge : CEdge
        所属边。
    direction : WalkDirection
        当前 settled 走势方向 σ（缠论走势方向，盘整→FLAT）。
    bsp_side : {"buy", "sell"} | None
        最近一个**有方向操作意义**的买卖点的方向（买/卖）；无则 None。
    bsp_kind : {"type1", "type2", "type3"} | None
        最近买卖点的类型；无则 None。
    bsp_confirmed : bool
        该买卖点是否 confirmed（右侧确认，可操作）；candidate-only 为 False。
    """

    edge: CEdge
    direction: WalkDirection
    bsp_side: Literal["buy", "sell"] | None
    bsp_kind: Literal["type1", "type2", "type3"] | None
    bsp_confirmed: bool

    @property
    def sigma(self) -> WalkDirection:
        """单级别 σ = level-0 settled 走势方向。"""
        return self.direction

    def bsp_signed_vote(self) -> float:
        """单级别买卖点有向投票（type1>type2>type3、confirmed>candidate 加权）。"""
        if self.bsp_side is None or self.bsp_kind is None:
            return 0.0
        kind_w = _BSP_KIND_WEIGHT[self.bsp_kind]
        conf_w = W_BSP_CONFIRMED if self.bsp_confirmed else W_BSP_CANDIDATE
        sign = 1.0 if self.bsp_side == "buy" else -1.0
        return sign * kind_w * conf_w


# ═══════════════════════════════════════════════════════════════
# 圈闭合纠错（L0：纯代数一致性，作用于已有读数）
# ═══════════════════════════════════════════════════════════════


def _sign(x: int) -> int:
    """符号函数：>0→+1, <0→−1, ==0→0。"""
    return (x > 0) - (x < 0)


@dataclass(frozen=True, slots=True)
class ConsensusDirection:
    """某资产对（派生边）经圈闭合多数表决后的共识方向。

    Attributes
    ----------
    edge : CEdge
        派生边（如 ES/GC）。
    direct : int
        直接读数 σ(num/den).value ∈ {-1,0,+1}。
    via_cash : int
        过现金估计 sign(σ(num/$) − σ(den/$))。
    via_third : int
        过第三资产估计（见 closure_consensus 文档）。
    consensus : int
        三估计多数表决结果 ∈ {-1,0,+1}（平局回退到 direct）。
    dissent : int
        与 consensus 不一致的估计条数（0=三估计全一致；1=一条矛盾；2=两条矛盾）。
        dissent≥1 即圈闭合检出了不一致——纠错发生处。
    """

    edge: CEdge
    direct: int
    via_cash: int
    via_third: int
    consensus: int
    dissent: int


def _majority(direct: int, via_cash: int, via_third: int) -> tuple[int, int]:
    """三估计多数表决。返回 (consensus, dissent_count)。

    consensus = sign(三者之和)；和为 0（如 +1/−1/0 或 0/0/0）回退到 direct，
    因为直接读数是该边自身引擎的产出（最高优先级，其余两条仅作纠错冗余）。
    dissent = 与 consensus 符号不同的估计条数。
    """
    s = direct + via_cash + via_third
    consensus = _sign(s)
    if consensus == 0:
        consensus = direct
    estimates = (direct, via_cash, via_third)
    dissent = sum(1 for e in estimates if e != consensus)
    return consensus, dissent


def closure_consensus(
    readings: dict[CEdge, EdgeReadingLike],
) -> dict[CEdge, ConsensusDirection]:
    """对三条派生边各自做圈闭合多数表决纠错。

    每个资产对有三条独立方向估计路径（K4 冗余）：

      ES/GC：直接 σ(ES/GC)；过现金 sign(σ(ES/$)−σ(GC/$))；
             过油 sign(σ(ES/CL)−σ(GC/CL))。
      ES/CL：直接 σ(ES/CL)；过现金 sign(σ(ES/$)−σ(CL/$))；
             过金 sign(σ(ES/GC)+σ(GC/CL))   ← ES/CL=(ES/GC)·(GC/CL)，对数收益相加。
      GC/CL：直接 σ(GC/CL)；过现金 sign(σ(GC/$)−σ(CL/$))；
             过股 sign(σ(ES/CL)−σ(ES/GC)).

    三估计多数表决 → 共识方向 + dissent 计数（检错量）。

    Parameters
    ----------
    readings : dict[CEdge, EdgeReadingLike]
        六条边的读数（必须含全部 ALL_EDGES）。圈闭合只读 ``.sigma``——单级别
        reading 给 level-0 方向，多级别 reading 给 L2 方向，**同级别闭合不混级别**。

    Returns
    -------
    dict[CEdge, ConsensusDirection]
        三条派生边的共识方向（键为 EDGE_ES_GC / EDGE_ES_CL / EDGE_GC_CL）。
    """
    d = {e: readings[e].sigma.value for e in ALL_EDGES}

    # ES/GC
    es_gc = _majority(
        direct=d[EDGE_ES_GC],
        via_cash=_sign(d[EDGE_ES_USD] - d[EDGE_GC_USD]),
        via_third=_sign(d[EDGE_ES_CL] - d[EDGE_GC_CL]),
    )
    # ES/CL（过金：(ES/GC)·(GC/CL) → 对数收益相加）
    es_cl = _majority(
        direct=d[EDGE_ES_CL],
        via_cash=_sign(d[EDGE_ES_USD] - d[EDGE_CL_USD]),
        via_third=_sign(d[EDGE_ES_GC] + d[EDGE_GC_CL]),
    )
    # GC/CL（过股：(ES/CL)/(ES/GC) → 对数收益相减）
    gc_cl = _majority(
        direct=d[EDGE_GC_CL],
        via_cash=_sign(d[EDGE_GC_USD] - d[EDGE_CL_USD]),
        via_third=_sign(d[EDGE_ES_CL] - d[EDGE_ES_GC]),
    )

    return {
        EDGE_ES_GC: ConsensusDirection(
            EDGE_ES_GC, d[EDGE_ES_GC], _sign(d[EDGE_ES_USD] - d[EDGE_GC_USD]),
            _sign(d[EDGE_ES_CL] - d[EDGE_GC_CL]), es_gc[0], es_gc[1],
        ),
        EDGE_ES_CL: ConsensusDirection(
            EDGE_ES_CL, d[EDGE_ES_CL], _sign(d[EDGE_ES_USD] - d[EDGE_CL_USD]),
            _sign(d[EDGE_ES_GC] + d[EDGE_GC_CL]), es_cl[0], es_cl[1],
        ),
        EDGE_GC_CL: ConsensusDirection(
            EDGE_GC_CL, d[EDGE_GC_CL], _sign(d[EDGE_GC_USD] - d[EDGE_CL_USD]),
            _sign(d[EDGE_ES_CL] - d[EDGE_ES_GC]), gc_cl[0], gc_cl[1],
        ),
    }


# ═══════════════════════════════════════════════════════════════
# 联合读数（投票权重为 [新缠论:选择] 可调常数）
# ═══════════════════════════════════════════════════════════════

# 走势方向 σ 的投票权重。
W_DIRECTION = 1.0

# 买卖点类型权重 [新缠论:选择]：一类（趋势背驰，反转最强）> 二类 > 三类（中枢突破，延续）。
# 缠论 17/20/24 课：第一类买卖点是最高级别的反转信号。
_BSP_KIND_WEIGHT: dict[str, float] = {
    "type1": 1.0,
    "type2": 0.7,
    "type3": 0.5,
}
# confirmed（右侧确认）相对 candidate（仅左侧形成）的权重 [新缠论:选择]。
W_BSP_CONFIRMED = 1.0
W_BSP_CANDIDATE = 0.5

# ── 多级别 L1 背驰买卖点投票权重 [新缠论:选择] ──
# L1 操作级买卖点 = 走势级背驰点（37 课底背驰买/顶背驰卖）。背驰=第一类买卖点
# （最强反转信号），故只用单一权重（不分 type1/2/3——背驰天然是 type1 语义）。
#
# **这是 C 路径多级别版本最关键的价值判断点（详见 LeveledEdgeReading）**：
#   W_L1_DIVERGENCE_BSP 相对 W_DIRECTION 的比值决定门控性格——
#     比值 > 1：买卖点主导（反转择时，敏感，可能逆 L2 趋势进场）；
#     比值 < 1：L2 方向主导（趋势跟随，鲁棒，背驰仅微调）；
#     比值 = 1：方向与买卖点等权（默认）。
# 与 E 版本进出场结构对齐：E 用 L1 底背驰进场（敏感）+ L2 顶背驰出场（鲁棒），
# 默认取等权，让联合读数同时表达"趋势在哪"（L2 σ）与"背驰拐点"（L1 BSP）。
W_L1_DIVERGENCE_BSP = 1.0


# ═══════════════════════════════════════════════════════════════
# 多级别单边读数（L2 方向 + L1 操作级背驰买卖点）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class LeveledEdgeReading:
    """单条边的**多级别**缠论读数（与 E 版本进出场逻辑同构）。

    级别语义（任务要求"L0/L1/L2 完整级别体系，方向取最高级、买卖点取操作级"）：

    - ``l2_direction``（**方向信号，取最高级别 L2**）= L2 PH 翻转累积态：
      L1 走势 settle 端点喂 PH 树，底分型 settle → 转 UP、顶分型 settle → 转 DOWN
      （E 版本 ``l2_flip_long/short`` → ``l2_direction``）。圈闭合纠错用此级别。

    - ``l1_bsp_side``（**买卖点信号，取操作级别 L1**）= L1 走势级背驰买卖点：
      buy = 底背驰（down-move settle 那一刻且相对前一 down-move 力度衰减/不创新低）；
      sell = 顶背驰（up-move settle ∧ 力度衰减/不创新高）。背驰确认（MACD 面积 ∨
      价格不创极值，37 课）**内化**——``l1_bsp_side`` 仅在背驰成立时非 None。

    与 E 版本的同构：E 的进场 = L1 底背驰、出场 = L2 趋势顶背驰。本读数把这两个级别
    的信号同时暴露给联合读数——``sigma``（L2）给趋势方向、``bsp_signed_vote()``（L1）
    给操作拐点。

    Attributes
    ----------
    edge : CEdge
        所属边。
    l2_direction : WalkDirection
        最高级别 L2 走势方向（PH 翻转累积；尚未翻转 → FLAT）。
    l1_bsp_side : {"buy", "sell"} | None
        最近的 L1 操作级背驰买卖点方向；无背驰确认买卖点则 None。
    l1_div_confirmed : bool
        该 L1 买卖点是否经背驰确认（37 课力度衰减/不创极值）。E 版本语义下
        ``l1_bsp_side`` 非 None ⟺ ``l1_div_confirmed`` True（背驰内化于买卖点）；
        保留独立字段供审计/未来放宽（candidate 背驰）。
    """

    edge: CEdge
    l2_direction: WalkDirection
    l1_bsp_side: Literal["buy", "sell"] | None
    l1_div_confirmed: bool

    @property
    def sigma(self) -> WalkDirection:
        """多级别 σ = 最高级别 L2 走势方向（圈闭合纠错在此级别对齐）。"""
        return self.l2_direction

    def bsp_signed_vote(self) -> float:
        """L1 操作级背驰买卖点有向投票（底背驰买→正利好分子、顶背驰卖→负）。

        背驰=第一类买卖点（最强反转），权重 ``W_L1_DIVERGENCE_BSP``；未确认背驰
        （``l1_div_confirmed`` False）权重折半（与单级别 candidate 对称）。
        """
        if self.l1_bsp_side is None:
            return 0.0
        conf_w = 1.0 if self.l1_div_confirmed else W_BSP_CANDIDATE
        sign = 1.0 if self.l1_bsp_side == "buy" else -1.0
        return sign * W_L1_DIVERGENCE_BSP * conf_w


@dataclass(frozen=True, slots=True)
class EdgeContribution:
    """单条边对某目标顶点联合读数的贡献明细（可审计）。"""

    edge: CEdge
    orientation: int        # +1 = 目标是分子；−1 = 目标是分母
    direction_used: int     # 实际使用的方向（派生边=共识纠错后；主边=直接读）
    direction_vote: float   # orientation * W_DIRECTION * direction_used
    bsp_vote: float         # orientation * _bsp_signed_vote(reading)
    dissent: int            # 该边（若派生）的圈闭合检错量；主边恒 0


@dataclass(frozen=True, slots=True)
class JointReading:
    """某目标顶点的联合读数——六边读数 + 圈闭合纠错的聚合产物。

    Attributes
    ----------
    target : CVertex
        目标资产顶点。
    score : float
        联合得分。>0 = 做多该资产为最强方向；<0 = 做空/回避；幅度 = 信号强度。
    favored : WalkDirection
        排序结论：score>0→UP（做多）、score<0→DOWN（做空/回避）、==0→FLAT。
    contributions : tuple[EdgeContribution, ...]
        参与的（与目标关联的）各边贡献明细。
    total_dissent : int
        参与边的圈闭合检错总量（>0 表示读数间存在被纠正的矛盾）。
    """

    target: CVertex
    score: float
    favored: WalkDirection
    contributions: tuple[EdgeContribution, ...]
    total_dissent: int


def joint_reading(
    readings: dict[CEdge, EdgeReadingLike],
    target: CVertex,
) -> JointReading:
    """联合读数：聚合与 ``target`` 关联的边，给出做多/做空排序得分。

    **不做门控（允许/拦截），做排序**——返回 ``favored`` 方向与 ``score`` 强度，
    由调用方决定如何消费（回测中投射到纯多头引擎 = 只在 favored=UP 时入场）。

    机制：
      1. ``closure_consensus`` 先对三条派生边做圈闭合纠错 → 共识方向。
      2. 取与 target 关联的边（K4 中每顶点关联 3 条边）。
      3. 每条边贡献 = 方向投票 + 买卖点投票，按 orientation（target 为分子/分母）定号。
         派生边用共识纠错方向；主边用直接读数。
      4. score = 各边贡献之和；favored = sign(score)。

    Parameters
    ----------
    readings : dict[CEdge, EdgeReadingLike]
        六条边读数（含全部 ALL_EDGES）。单级别用 EdgeReading，多级别用
        LeveledEdgeReading——方向票取 ``.sigma``、买卖点票取 ``.bsp_signed_vote()``。
    target : CVertex
        目标资产（如 EQUITY，对应 OKLO 这类动产标的）。

    Returns
    -------
    JointReading
    """
    consensus = closure_consensus(readings)
    contribs: list[EdgeContribution] = []
    score = 0.0
    total_dissent = 0

    for edge in ALL_EDGES:
        if target not in (edge.num, edge.den):
            continue  # 该顶点不关联的边（与 $ 无关的纯第三方边）不直接投票
        orientation = 1 if edge.num is target else -1

        if edge.is_derived:
            cons = consensus[edge]
            dir_used = cons.consensus
            dissent = cons.dissent
        else:
            dir_used = readings[edge].sigma.value
            dissent = 0

        dir_vote = orientation * W_DIRECTION * dir_used
        bsp_vote = orientation * readings[edge].bsp_signed_vote()
        contribs.append(EdgeContribution(
            edge=edge, orientation=orientation, direction_used=dir_used,
            direction_vote=dir_vote, bsp_vote=bsp_vote, dissent=dissent,
        ))
        score += dir_vote + bsp_vote
        total_dissent += dissent

    if score > 0:
        favored = WalkDirection.UP
    elif score < 0:
        favored = WalkDirection.DOWN
    else:
        favored = WalkDirection.FLAT

    return JointReading(
        target=target,
        score=score,
        favored=favored,
        contributions=tuple(contribs),
        total_dissent=total_dissent,
    )


def rank_assets(
    readings: dict[CEdge, EdgeReadingLike],
) -> tuple[JointReading, ...]:
    """对三个**资产**顶点（ES/GC/CL，不含 $）按联合得分降序排序。

    实现"哪个方向信号最强"——返回的首元素是当前最值得做多的资产。
    现金顶点不参与（它是计价基准，不是可做多的资产标的）。

    Returns
    -------
    tuple[JointReading, ...]
        按 score 降序的三个资产联合读数。
    """
    assets = (CVertex.EQUITY, CVertex.GOLD, CVertex.OIL)
    jrs = [joint_reading(readings, a) for a in assets]
    return tuple(sorted(jrs, key=lambda j: j.score, reverse=True))
