"""递归分解树 + 纵向圈闭合 — M2 选股层的纵向维度。

概念溯源：529号/528号折叠通道重构（M/P/C/R 四顶点）、527号 σ 走势方向态、
          292号折叠区间套（外层约束内层）、254号跨国三层、缠论第73课（行业→龙头→比价）。
设计文档：docs/architecture/recursive_decomposition_tree.md

## 三层圈闭合中的纵向层

M2 已有两层圈闭合：
  - 横向（strategy/k4_path_c.py）：同层 K4 六边 σ 的三估计多数表决纠错。
  - 跨国（topology/cross_national_pipeline.py）：各经济体 K4 经汇率边 Σ 对齐。

本模块补齐**纵向圈闭合**：宏观顶点（如 P=股权）与其成分（行业 ETF→成分股）之间的
一致性检验。三者同构——都在 σ 符号层做一致性表决（L0，零信息增量）——区别只在表决
对象与权重：横向是三估计等权，纵向是成分市值加权。

## 为什么"不闭合"是信号而非 bug

σ 是离散走势方向符号 {+1,0,−1}（527号），加法不封闭。故宏观方向（直接读 σ(P/ref)）
与成分加权合成方向（Σ wᵢ σ(cᵢ/ref)）可能矛盾。矛盾即结构信息：

  - 不闭合 + 高集中度 = 趋势由少数大票驱动 = 结构脆弱。
  - 成分残差小（与宏观同向）= 趋势主驱动力 = 选股排序靠前（第73课"龙头"）。
  - 成分残差大（偏离宏观）= 比价偏离 = 落后/低估候选（第73课"比价关系发现低估"）。

## 递归分解树四层

  Layer 3 CROSS_NATIONAL       跨国 K4（cross_national_pipeline.py）
  Layer 2 NATIONAL_K4          单国 M/P/C/R 六边（graph.py + k4_path_c.py）
  Layer 1 VERTEX_DECOMPOSITION 顶点→行业→成分股  ← 纵向圈闭合在此层
  Layer 0 INSTRUMENT           叶标的，E 版本背驰定位器操盘

上层闭合是下层分解的前提（292号区间套：外层约束内层）——σ_macro=FLAT 时纵向分解
无意义，SelectionRanker 返回空排序。

## 认识论标注（formalization-validity-domain 规则）

  - 纵向圈闭合代数（synth/concentration/residual/contribution）：L0（纯代数，
    作用于已有 σ 读数，零信息增量，与横向 closure_consensus 同级）。
  - 树结构验证（权重归一/层级约束）：L0（定义检查）。
  - 单成分 σ(cᵢ/ref) 读数：L2（真实数据缠论引擎产出，可证伪）。
  - 纵向选股 alpha：L2（待验证）——按 contribution 排序是否跑赢需独立回测对照，
    本模块**不声称已验证**。

谱系引用：529/528（折叠重构）、527（σ 本体论）、292（区间套）、254（跨国）、
project_m2_k4_selection_gate（K4 门控毁 alpha → 本模块用排序替代门控）、
project_ph_settle_usage_boundary（形态学是 candidate、操作需 MACD 背驰 → role 非操作确认）。
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass, field
from enum import Enum

from newchan.topology.config_space import WalkDirection
from newchan.topology.graph import Vertex

# 权重归一容差（市值权重和判定）。
_WEIGHT_EPS = 1e-6

# contribution 视为"中性"（≈0）的容差（role 分类用）。
_CONTRIB_EPS = 1e-9


def _sign(x: float) -> int:
    """符号函数：>0→+1, <0→−1, ==0→0。"""
    return (x > 0) - (x < 0)


# ═══════════════════════════════════════════════════════════════
# 分解树结构（Layer 3 → Layer 0）
# ═══════════════════════════════════════════════════════════════


class TreeLayer(Enum):
    """递归分解树的四层（由宏观到微观）。"""

    CROSS_NATIONAL = 3        # 跨国 K4 层（US/CN/EU/JP，汇率边连接）
    NATIONAL_K4 = 2           # 单国 K4 层（M/P/C/R 六边）
    VERTEX_DECOMPOSITION = 1  # 顶点分解层（顶点→行业→成分股），纵向圈闭合发生层
    INSTRUMENT = 0            # 个股操盘层（叶标的，E 版本操盘）


@dataclass(frozen=True, slots=True)
class DecompositionNode:
    """递归分解树的一个节点。

    Attributes
    ----------
    label : str
        人类可读标识（如 "US"、"P"、"SOXX"、"NVDA"）。
    layer : TreeLayer
        所在层。
    weight : float
        在**父节点**中的市值权重；兄弟节点权重和应 ≈ 1（根节点为 1.0）。
    vertex : Vertex | None
        对应的 K4 顶点；仅 VERTEX_DECOMPOSITION 层的顶点节点非空（标明分解哪个顶点）。
    symbol : str | None
        数据源标的代理；叶节点（可操作）与可观测节点非空。
    children : tuple[DecompositionNode, ...]
        子节点；叶节点为空元组。
    """

    label: str
    layer: TreeLayer
    weight: float = 1.0
    vertex: Vertex | None = None
    symbol: str | None = None
    children: tuple["DecompositionNode", ...] = field(default_factory=tuple)

    @property
    def is_leaf(self) -> bool:
        """是否为叶节点（无子节点）。"""
        return len(self.children) == 0

    @property
    def children_weight_sum(self) -> float:
        """子节点权重和（非叶节点应 ≈ 1）。"""
        return sum(c.weight for c in self.children)


class DecompositionTree:
    """递归分解树：管理四层结构、遍历、层级取节点、权重校验。

    不可变（构建后只读）。校验在构建时执行（系统边界输入验证，fail-fast）。
    """

    __slots__ = ("_root",)

    def __init__(self, root: DecompositionNode) -> None:
        """构建并校验分解树。

        Parameters
        ----------
        root : DecompositionNode
            根节点（通常 layer=CROSS_NATIONAL 或 NATIONAL_K4）。

        Raises
        ------
        ValueError
            如果违反结构约束（权重未归一、叶节点无 symbol、顶点分解节点无 vertex）。
        """
        self._validate(root)
        self._root = root

    @staticmethod
    def _validate(node: DecompositionNode) -> None:
        """递归校验结构约束（fail-fast）。"""
        if node.is_leaf:
            if node.symbol is None:
                raise ValueError(
                    f"叶节点 {node.label!r}（layer={node.layer.name}）必须有 symbol（可操作）"
                )
        else:
            ws = node.children_weight_sum
            if abs(ws - 1.0) > _WEIGHT_EPS:
                raise ValueError(
                    f"节点 {node.label!r} 的子节点权重和={ws:.6f}，须归一到 1.0±{_WEIGHT_EPS}"
                )
            for c in node.children:
                DecompositionTree._validate(c)

        if node.layer is TreeLayer.VERTEX_DECOMPOSITION and node.vertex is None and not node.is_leaf:
            raise ValueError(
                f"顶点分解节点 {node.label!r} 必须标明 vertex（分解的是哪个 K4 顶点）"
            )

    @property
    def root(self) -> DecompositionNode:
        """根节点。"""
        return self._root

    def nodes_at_layer(self, layer: TreeLayer) -> tuple[DecompositionNode, ...]:
        """返回指定层的全部节点（前序遍历顺序）。

        Parameters
        ----------
        layer : TreeLayer
            目标层。

        Returns
        -------
        tuple[DecompositionNode, ...]
        """
        result: list[DecompositionNode] = []

        def _walk(n: DecompositionNode) -> None:
            if n.layer is layer:
                result.append(n)
            for c in n.children:
                _walk(c)

        _walk(self._root)
        return tuple(result)

    def leaves(self) -> tuple[DecompositionNode, ...]:
        """返回全部叶节点（可操作标的，喂给 Layer 0 E 版本）。"""
        result: list[DecompositionNode] = []

        def _walk(n: DecompositionNode) -> None:
            if n.is_leaf:
                result.append(n)
            for c in n.children:
                _walk(c)

        _walk(self._root)
        return tuple(result)

    def children_of(self, label: str) -> tuple[DecompositionNode, ...]:
        """返回指定 label 节点的直接子节点（用于取某顶点的成分集）。

        Parameters
        ----------
        label : str
            目标节点 label（须唯一）。

        Returns
        -------
        tuple[DecompositionNode, ...]

        Raises
        ------
        KeyError
            如果找不到该 label。
        """
        found: DecompositionNode | None = None

        def _walk(n: DecompositionNode) -> None:
            nonlocal found
            if n.label == label:
                found = n
            for c in n.children:
                _walk(c)

        _walk(self._root)
        if found is None:
            raise KeyError(f"分解树中找不到节点 label={label!r}")
        return found.children


# ═══════════════════════════════════════════════════════════════
# 纵向圈闭合（L0：σ 符号层加权方向一致性，作用于已有读数）
# ═══════════════════════════════════════════════════════════════


# 纵向操作级买卖点投票权重 λ [新缠论:选择]——与 k4_path_c.W_L1_DIVERGENCE_BSP 同性质。
# λ>1 买卖点主导（反转择时，敏感）；λ=1 等权（默认）；λ<1 方向主导（趋势跟随）；λ=0 纯方向。
W_VERTICAL_BSP = 1.0

# 不兼容阈值 θ [新缠论:选择]：相反高级别边的有效权重和超过 θ 即判不兼容（设计文档 §3.2.3）。
THETA_INCOMPAT = 0.5

# a0 精确闭合的残差容差（数据质量门，设计文档 §3.1）。指数复制 log 残差超此值即数据不洁。
_A0_TOL = 1e-3


def level_confidence(level: int) -> float:
    """级别 → 置信度（级别清晰度，与方向 σ 正交）。conf = 1 − 2^(−level)。

    设计文档 §3.2.2 [新缠论:选择]：先涌现高级别结构的边信号最清晰，置信度最高。
    L1→0.5、L2→0.75、L3→0.875…（"每升一级不确定性减半"）；level≤0 → 0（无结构）。

    **只依赖 level 不依赖 σ**：高级别盘整（L3 中枢，σ=FLAT）是清晰结构（高 conf）；
    低级别盘整（L1 未涌现方向）才是不确定（低 conf）。这是"低级别盘整边权重降低"的精确含义。
    可改为线性/分段（属 L2 待标定），不改变接口。
    """
    if level <= 0:
        return 0.0
    return 1.0 - 2.0 ** (-level)


# ═══════════════════════════════════════════════════════════════
# Layer A：a0 精确闭合（数据质量门，L1——等式，残差应 ≈ 0）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class A0ClosureResult:
    """a0 精确闭合（指数复制残差）检验产物——数据质量门，非信号源。

    ρ(t) = log(P(t)) − log(Σ wᵢ Sᵢ(t))，价格/权重自洽时应 ≈ 0（设计文档 §3.1）。
    横向对应：ρ(t)=log(P/C)−[log(P/M)+log(M/C)]（比值 log 恒等，构造上必为 0）。

    Attributes
    ----------
    n_bars : int
        参与检验的 bar 数。
    mean_abs_residual : float
        |ρ(t)| 均值。
    max_abs_residual : float
        |ρ(t)| 最大值。
    is_clean : bool
        max_abs_residual ≤ tol：数据/权重自洽，下游 Layer B 结论可信。
        False = 数据质量门未通过（权重错/成分不全/价格错位/分红未调整），先修数据。
    """

    n_bars: int
    mean_abs_residual: float
    max_abs_residual: float
    is_clean: bool


def a0_precision_residual(
    macro_prices: Sequence[float],
    weighted_components: Sequence[tuple[float, Sequence[float]]],
    tol: float = _A0_TOL,
) -> A0ClosureResult:
    """Layer A：逐 bar 检查指数复制恒等 ρ(t)=log(P)−log(Σ wᵢ Sᵢ)（数据质量门，L1）。

    这是 Layer B（走势状态软闭合）的**前置门**：ρ 大 → 成分加权无法复制宏观 →
    数据不洁 → 不在脏数据上跑选股（no-workaround）。L1：验证数据，零信息增量。

    Parameters
    ----------
    macro_prices : Sequence[float]
        宏观顶点（指数）价格序列，全正。
    weighted_components : Sequence[tuple[float, Sequence[float]]]
        成分 (市值权重, 价格序列)；权重和应 ≈ 1，各价格序列与 macro_prices 等长对齐、全正。
    tol : float
        is_clean 的残差容差（默认 _A0_TOL）。

    Returns
    -------
    A0ClosureResult

    Raises
    ------
    ValueError
        如果序列空/不等长/权重未归一/价格非正。
    """
    if not weighted_components:
        raise ValueError("weighted_components 不能为空")
    n = len(macro_prices)
    if n == 0:
        raise ValueError("macro_prices 不能为空")
    wsum = sum(w for w, _ in weighted_components)
    if abs(wsum - 1.0) > _WEIGHT_EPS:
        raise ValueError(f"成分权重和={wsum:.6f}，须归一到 1.0±{_WEIGHT_EPS}")
    for w, prices in weighted_components:
        if len(prices) != n:
            raise ValueError(
                f"成分价格序列长度={len(prices)} 与 macro_prices 长度={n} 不对齐"
            )

    total = 0.0
    max_abs = 0.0
    for t in range(n):
        synth = sum(w * prices[t] for w, prices in weighted_components)
        if macro_prices[t] <= 0 or synth <= 0:
            raise ValueError(f"价格须为正（t={t}）：macro={macro_prices[t]}, synth={synth}")
        rho = abs(math.log(macro_prices[t]) - math.log(synth))
        total += rho
        max_abs = max(max_abs, rho)

    return A0ClosureResult(
        n_bars=n,
        mean_abs_residual=total / n,
        max_abs_residual=max_abs,
        is_clean=(max_abs <= tol),
    )


# ═══════════════════════════════════════════════════════════════
# Layer B：走势状态软闭合（兼容性，不是等式；级别加权）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class ComponentReading:
    """一个成分相对参照系顶点的缠论读数（纵向圈闭合 Layer B 的输入单元）。

    Attributes
    ----------
    symbol : str
        成分标的（行业 ETF 或成分股）。
    weight : float
        在宏观顶点中的**市值权重**（同一宏观下成分权重和应 ≈ 1）。
    sigma : WalkDirection
        σ(成分 / 参照系) 的走势方向——该成分**最高涌现级别**的走势方向（527号走势方向态）。
    level : int
        该成分边从 a0 各自递归涌现的**最高级别**（≥1）。级别越高信号越清晰、置信度越高
        （526号：三个 L1 不是同一个 L1，级别由各边独立涌现，不跨边对齐）。
    bsp_vote : float
        操作级背驰买卖点有向投票（底背驰买→正利好该成分、顶背驰卖→负）；
        无操作级买卖点为 0.0。语义与 k4_path_c.EdgeReadingLike.bsp_signed_vote() 同构。
    """

    symbol: str
    weight: float
    sigma: WalkDirection
    level: int = 1
    bsp_vote: float = 0.0


class ComponentRole(Enum):
    """成分相对宏观趋势的角色分类（形态学 candidate 层，非操作确认）。"""

    DRIVER = "driver"          # 主驱动力：同向高贡献（第73课"龙头"）
    LAGGARD = "laggard"        # 落后：盘整偏离宏观（第73课"比价低估候选"）
    CONTRARIAN = "contrarian"  # 反向：逆宏观（做空/回避候选）
    MARGINAL = "marginal"      # 边缘：其他


@dataclass(frozen=True, slots=True)
class ComponentContribution:
    """单成分对纵向圈闭合 Layer B 的贡献明细（可审计）。"""

    symbol: str
    weight: float            # 市值权重
    sigma: WalkDirection
    level: int               # 成分边涌现的最高级别
    level_confidence: float  # conf(level) = 1 − 2^(−level)
    eff_weight: float        # 有效权重 = weight × level_confidence（信号强度维度）
    residual: int            # |σ_i − σ_macro| ∈ {0,1,2}：0 对齐、1 正交、2 反向
    direction_vote: float    # eff_weight * (σ_i.value * σ_macro.value)
    bsp_vote: float          # W_VERTICAL_BSP * 成分 bsp_vote
    contribution: float      # direction_vote + bsp_vote（选股排序主键）


@dataclass(frozen=True, slots=True)
class VerticalClosureResult:
    """纵向圈闭合 Layer B 检验产物（走势状态软闭合，兼容性而非等式）。

    Attributes
    ----------
    macro_sigma : WalkDirection
        宏观边 σ(P/ref) 的直接读数（最高涌现级别走势方向）。
    macro_level : int
        宏观边涌现的最高级别。
    macro_directional : bool
        宏观方向是否明确（σ_macro ≠ FLAT）；False 时纵向分解无意义（§2.1）。
    synth : float
        成分**有效权重**方向合成 Σ eff_weightᵢ σᵢ.value（高级别清晰边主导）。
    compatible : bool
        走势状态是否兼容（incompatibility ≤ θ）——**替代等式 closed**。
        兼容 ≠ 相等：成分盘整/低级别反向不破坏兼容，仅高级别相反才不兼容（§3.2.3）。
    incompatibility : float
        与宏观高级别相反成分的有效权重和（Σ_{σᵢ·σ_macro<0} eff_weightᵢ）。
    weighted_participation : float
        与宏观同向成分的**市值权重和**（市值集中度维度，与级别正交）。
    count_participation : float
        与宏观同向成分的**个数比例**。
    concentration : float
        weighted_participation − count_participation：
        >0 趋势集中在少数大票（结构脆弱）；≈0 广泛参与（稳健）；<0 龙头分歧（罕见）。
    contributions : tuple[ComponentContribution, ...]
        各成分贡献明细（输入顺序）。
    """

    macro_sigma: WalkDirection
    macro_level: int
    macro_directional: bool
    synth: float
    compatible: bool
    incompatibility: float
    weighted_participation: float
    count_participation: float
    concentration: float
    contributions: tuple[ComponentContribution, ...]


class VerticalClosure:
    """纵向圈闭合 Layer B 检验器（L0：纯代数，级别加权兼容性，作用于已有 σ 读数）。"""

    @staticmethod
    def check(
        macro_sigma: WalkDirection,
        components: tuple[ComponentReading, ...],
        macro_level: int = 1,
        w_bsp: float = W_VERTICAL_BSP,
        theta: float = THETA_INCOMPAT,
    ) -> VerticalClosureResult:
        """对宏观顶点与其成分做走势状态软闭合检验（兼容性，不强制级别对齐）。

        与等式检查的本质区别（设计文档 §3.2）：各边从 a0 各自递归涌现不同级别
        （三个 L1 不是同一个 L1），故**不**做 "Σσᵢ == σ_macro" 等式，而做兼容性——
        成分盘整/低级别反向不破坏兼容，仅高级别相反才不兼容。级别经 conf(level)
        加权进 synth/incompatibility/contribution；concentration 用市值权重（结构维度）。

        Parameters
        ----------
        macro_sigma : WalkDirection
            宏观边 σ(P/ref) 的最高涌现级别走势方向。
        components : tuple[ComponentReading, ...]
            成分读数集（携带各自 level；同一宏观下市值权重和应 ≈ 1）。
        macro_level : int
            宏观边涌现的最高级别（默认 1）。
        w_bsp : float
            操作级买卖点投票权重 λ（默认 W_VERTICAL_BSP）。
        theta : float
            不兼容阈值 θ（默认 THETA_INCOMPAT）。

        Returns
        -------
        VerticalClosureResult

        Raises
        ------
        ValueError
            如果 components 为空，或市值权重和未归一。
        """
        if not components:
            raise ValueError("components 不能为空")
        wsum = sum(c.weight for c in components)
        if abs(wsum - 1.0) > _WEIGHT_EPS:
            raise ValueError(
                f"成分市值权重和={wsum:.6f}，须归一到 1.0±{_WEIGHT_EPS}"
            )

        m = macro_sigma.value
        synth = 0.0
        incompatibility = 0.0
        weighted_participation = 0.0   # 市值维度（concentration 用）
        same_count = 0
        contribs: list[ComponentContribution] = []

        for c in components:
            conf = level_confidence(c.level)
            eff = c.weight * conf
            residual = abs(c.sigma.value - m)
            direction_vote = eff * (c.sigma.value * m)
            bsp_vote = w_bsp * c.bsp_vote

            synth += eff * c.sigma.value
            if c.sigma.value * m < 0:          # 高级别相反才计入（eff 含 conf）
                incompatibility += eff
            if c.sigma is macro_sigma:          # 市值参与度（结构维度，不乘 conf）
                weighted_participation += c.weight
                same_count += 1

            contribs.append(
                ComponentContribution(
                    symbol=c.symbol,
                    weight=c.weight,
                    sigma=c.sigma,
                    level=c.level,
                    level_confidence=conf,
                    eff_weight=eff,
                    residual=residual,
                    direction_vote=direction_vote,
                    bsp_vote=bsp_vote,
                    contribution=direction_vote + bsp_vote,
                )
            )

        count_participation = same_count / len(components)
        return VerticalClosureResult(
            macro_sigma=macro_sigma,
            macro_level=macro_level,
            macro_directional=(macro_sigma is not WalkDirection.FLAT),
            synth=synth,
            compatible=(incompatibility <= theta),
            incompatibility=incompatibility,
            weighted_participation=weighted_participation,
            count_participation=count_participation,
            concentration=weighted_participation - count_participation,
            contributions=tuple(contribs),
        )


# ═══════════════════════════════════════════════════════════════
# 纵向选股排序（不做门控，做排序——与 joint_reading 哲学一致）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class SelectionEntry:
    """单成分的选股排序条目。"""

    symbol: str
    contribution: float
    weight: float
    residual: int
    role: ComponentRole


@dataclass(frozen=True, slots=True)
class SelectionRanking:
    """纵向选股排序结果。

    Attributes
    ----------
    macro_sigma : WalkDirection
        宏观方向（排序的背景）。
    entries : tuple[SelectionEntry, ...]
        按 contribution 降序的成分；macro 非方向（FLAT）时为空（§2.1）。
    concentration : float
        市场结构脆弱性指标（市值集中度，透传自 closure）。
    compatible : bool
        走势状态是否兼容（透传自 closure；不兼容=高级别成分逆宏观，选股结论需谨慎）。
    """

    macro_sigma: WalkDirection
    entries: tuple[SelectionEntry, ...]
    concentration: float
    compatible: bool

    @property
    def drivers(self) -> tuple[SelectionEntry, ...]:
        """主驱动力成分（第73课"龙头"候选）。"""
        return tuple(e for e in self.entries if e.role is ComponentRole.DRIVER)


def _classify_role(contribution: float, residual: int) -> ComponentRole:
    """成分角色分类——把缠论第73课的成分定位译成代数（设计文档 §4.2）。

    【关键价值判断点】这是本模块唯一携带领域价值判断的逻辑。它把 (contribution, residual)
    映射为第73课的成分角色：

      - DRIVER（龙头）   ：与宏观同向且正贡献（趋势主驱动力）。
      - LAGGARD（落后）  ：贡献中性但方向偏离（盘整 → 比价低估补涨候选）。
      - CONTRARIAN（反向）：逆宏观负贡献（做空/回避候选）。
      - MARGINAL（边缘） ：其他。

    分类阈值（contribution 的正/负/≈0 边界、residual 的对齐/偏离判定）是 [新缠论:选择]
    类决策——多种合理方案并存，最优阈值属 L2 需回测标定。当前实现是设计文档 §4.2 的默认方案。

    Parameters
    ----------
    contribution : float
        成分综合贡献（方向 + 买卖点）。
    residual : int
        成分对宏观的偏离 ∈ {0,1,2}。

    Returns
    -------
    ComponentRole
    """
    if contribution > _CONTRIB_EPS and residual == 0:
        return ComponentRole.DRIVER
    if abs(contribution) <= _CONTRIB_EPS and residual >= 1:
        return ComponentRole.LAGGARD
    if contribution < -_CONTRIB_EPS and residual == 2:
        return ComponentRole.CONTRARIAN
    return ComponentRole.MARGINAL


class SelectionRanker:
    """从纵向圈闭合残差产出选股排序（L0 排序，alpha 属 L2 待验证）。"""

    @staticmethod
    def rank(closure: VerticalClosureResult) -> SelectionRanking:
        """按成分贡献度降序排序，分类角色。

        宏观方向不明（σ_macro=FLAT）时返回空排序——分解一个无方向的宏观顶点无意义
        （292号区间套：上层闭合是下层分解的前提，设计文档 §2.1）。

        Parameters
        ----------
        closure : VerticalClosureResult
            纵向圈闭合检验产物。

        Returns
        -------
        SelectionRanking
        """
        if not closure.macro_directional:
            return SelectionRanking(
                macro_sigma=closure.macro_sigma,
                entries=(),
                concentration=closure.concentration,
                compatible=closure.compatible,
            )

        entries = [
            SelectionEntry(
                symbol=c.symbol,
                contribution=c.contribution,
                weight=c.weight,
                residual=c.residual,
                role=_classify_role(c.contribution, c.residual),
            )
            for c in closure.contributions
        ]
        entries.sort(key=lambda e: e.contribution, reverse=True)
        return SelectionRanking(
            macro_sigma=closure.macro_sigma,
            entries=tuple(entries),
            concentration=closure.concentration,
            compatible=closure.compatible,
        )


# ═══════════════════════════════════════════════════════════════
# 资本病理学：纵向沉没检测（设计文档 §8.3——沉没=走势缺失）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class SedimentationEntry:
    """单成分的沉没明细（卢麒元"沉没"=资本退出循环锁死）。"""

    symbol: str
    weight: float            # 市值权重（资本量）
    level: int               # 涌现级别（越低越死水）
    score: float             # market_weight × (1 − conf(level))，仅 σ=FLAT 非零


@dataclass(frozen=True, slots=True)
class SedimentationResult:
    """纵向圈闭合检出的资本沉没信号（设计文档 §8.3）。

    Attributes
    ----------
    total : float
        市场总沉没度 Σ scoreᵢ（资本死水占比的代理）。
    entries : tuple[SedimentationEntry, ...]
        沉没成分（score>0，按 score 降序）。
    """

    total: float
    entries: tuple[SedimentationEntry, ...]


def sedimentation_signal(closure: VerticalClosureResult) -> SedimentationResult:
    """从纵向圈闭合检测资本沉没（设计文档 §8.3：走势缺失=资本死水）。

    沉没 = **高市值 + 低 level + σ=FLAT**（资本沉淀但不涌现缠论走势结构）：

        scoreᵢ = market_weightᵢ × (1 − conf(levelᵢ))   仅 σᵢ=FLAT 成分非零

    - 低级别 FLAT（level=1, conf=0.5）→ 高沉没度 = **死水**（资本锁死，482号病态沉没）。
    - 高级别 FLAT（level=3, conf=0.875）→ 低沉没度 = 清晰中枢（健康盘整，资本待突破）。

    与 ComponentRole.LAGGARD 的区分（§8.3）：LAGGARD 是高 level 盘整（可补涨，资本会
    流转）；沉没是低 level FLAT（死水，资本不流转）。级别维度（conf）是判据。

    认识论：**L0**（纯代数，作用于已有 (σ,level) 读数）。"沉没→回避该成分"对操作的
    指导属 **L2 待验证**（需回测：沉没成分是否真的跑输）。有效域边界见 §8.7（金融折叠
    投影，非沉没的实体机制）。

    Parameters
    ----------
    closure : VerticalClosureResult
        纵向圈闭合 Layer B 检验产物。

    Returns
    -------
    SedimentationResult
    """
    entries: list[SedimentationEntry] = []
    for c in closure.contributions:
        if c.sigma is WalkDirection.FLAT:
            score = c.weight * (1.0 - c.level_confidence)
            if score > 0:
                entries.append(
                    SedimentationEntry(
                        symbol=c.symbol, weight=c.weight, level=c.level, score=score
                    )
                )
    entries.sort(key=lambda e: e.score, reverse=True)
    return SedimentationResult(
        total=sum(e.score for e in entries),
        entries=tuple(entries),
    )
