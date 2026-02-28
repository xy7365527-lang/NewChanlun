"""离散化算子 D 的核（kernel）结构与三态分类 — 缠论的内在选择性原理。

233号谱系：离散化算子 D 的核结构。
235号谱系：D 的三态分类——吸收/保留/放大。

缠论的离散化不是对连续动力学的近似，而是拓扑滤波器。
D 对耦合模式的作用是三态的：

  1. 吸收（ker(D)）：纯幅度耦合 → 0
  2. 保留：底空间独立耦合 → 不变
  3. 放大（image(D) 的非平凡部分）：方向耦合通过噪声滤除被相对放大

离散化算子 D = D3 . D2 . D1：

  D1: bar -> bi (笔)
      包含处理 + 分型 -> 笔只编码方向（up/down），不编码幅度。
      公理依据："笔是走势的最小可编码单元"（bi.md）

  D2: bi -> xianduan (线段)
      特征序列分型 -> 短促振荡被合并（古怪线段吸收机制）。
      公理依据："线段被终结，当且仅当至少被线段终结"（xianduan.md）

  D3: xianduan -> zhongshu -> zoushi direction
      中枢区间 [ZD,ZG] 吸收所有振荡为"无方向"。
      公理依据："盘整哪里有什么方向，只有趋势才有方向"（qushi.md）

ker(D) 的结构定理：

  一个耦合模式 phi in C 属于 ker(D)，当且仅当 phi 是纯幅度耦合——
  即 phi 仅调制共同方向内的收益率大小，不调制方向本身。

  形式化：设两资产连续收益率为 (r_X, r_Y)。
  - 幅度耦合 = Cov(|r_X|, |r_Y|) != 0 但 P(sign(r_X) = sign(r_Y)) = independent
  - 方向耦合 = P(sign(r_X) = sign(r_Y)) != independent（不论幅度相关性）

  D1 丢弃幅度保留方向 -> 幅度耦合 in ker(D1) subset ker(D)
  D2 通过特征序列合并吸收短程方向噪声 -> 弱方向耦合部分进入 ker(D2)
  D3 通过中枢吸收区间内振荡 -> 中枢内振荡幅度耦合 in ker(D3)

放大机制（235号）：
  D 的每一层滤除幅度噪声（ker 部分），使方向信号的信噪比递增。
  C-R 避险方向耦合在 D1（笔）层面信噪比低（被幅度噪声淹没），
  经 D2（线段滤噪）和 D3（中枢吸收区间振荡）后信噪比急剧上升。
  234号数据：C-R β_d1=0.009 → β_d3=0.688，放大比≈76倍。

232号经验验证：
  - E-R: partial_corr = -0.336, beta = -0.017 -> ker(D) (幅度跷跷板被 FLAT 吸收)
  - C-R: partial_corr = 0.153, beta = 0.688 -> not ker(D) (方向避险同步性穿透)
  - E-C: partial_corr = -0.029, beta ~ 0 -> 底空间独立性（不走纤维丛联络）
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from enum import Enum
from typing import Literal


class CouplingType(Enum):
    """耦合类型分类（235号三态扩展）。"""

    AMPLITUDE = "amplitude"  # 纯幅度耦合：方向独立但幅度相关（ker(D)）
    DIRECTION = "direction"  # 方向耦合：方向本身存在相关性（not ker(D)）
    INDEPENDENT = "independent"  # 底空间独立（不走联络）
    AMPLIFIED = "amplified"  # 方向耦合被递归放大（image(D) 非平凡部分）


@dataclass(frozen=True, slots=True)
class CouplingClassification:
    """耦合分类结果。

    Attributes
    ----------
    coupling_type : CouplingType
        耦合类型。
    in_kernel : bool
        是否属于 ker(D)。
    absorption_ratio : float
        吸收率 = 1 - |beta| / |partial_corr|。接近 1 = 几乎完全吸收。
    dominant_layer : str
        主要吸收层（D1/D2/D3 或 "none"）。
    """

    coupling_type: CouplingType
    in_kernel: bool
    absorption_ratio: float
    dominant_layer: str


@dataclass(frozen=True, slots=True)
class DiscretizationLayer:
    """离散化算子的单层。

    每一层编码缠论公理对信号的选择性过滤。

    Attributes
    ----------
    name : str
        层名称（D1/D2/D3）。
    description : str
        层功能描述。
    absorbs_amplitude : bool
        是否吸收幅度信息。
    absorbs_weak_direction : bool
        是否吸收弱方向信号。
    axiom : str
        公理依据。
    """

    name: str
    description: str
    absorbs_amplitude: bool
    absorbs_weak_direction: bool
    axiom: str


# ── 三层离散化算子 ──────────────────────────────────────────

D1 = DiscretizationLayer(
    name="D1",
    description="bar -> bi: 包含合并 + 分型 -> 笔（方向编码）",
    absorbs_amplitude=True,
    absorbs_weak_direction=False,
    axiom="笔是走势的最小可编码单元（bi.md）。"
    "笔只看方向（涨/跌），不看幅度。"
    "包含处理合并相邻 bar，丢弃幅度细节。",
)

D2 = DiscretizationLayer(
    name="D2",
    description="bi -> xianduan: 特征序列分型 -> 线段（结构方向编码）",
    absorbs_amplitude=False,
    absorbs_weak_direction=True,
    axiom="线段被终结，当且仅当至少被线段终结（xianduan.md）。"
    "古怪线段=笔破坏未发展为线段破坏=短程方向噪声被吸收。"
    "线段至少三笔且前三笔必须有重叠。",
)

D3 = DiscretizationLayer(
    name="D3",
    description="xianduan -> zhongshu -> zoushi: 中枢吸收 -> 方向判定",
    absorbs_amplitude=True,
    absorbs_weak_direction=True,
    axiom="盘整哪里有什么方向，只有趋势才有方向（qushi.md）。"
    "中枢区间 [ZD,ZG] 内的所有振荡 = 无方向。"
    "走势方向 = 中枢间位置关系（后DD>前GG=上涨）。",
)

DISCRETIZATION_LAYERS: tuple[DiscretizationLayer, ...] = (D1, D2, D3)


# ── 核结构 ──────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class KernelStructure:
    """ker(D) 的结构性描述。

    Attributes
    ----------
    description : str
        核的概念描述。
    absorption_mechanism : tuple[str, ...]
        各层的吸收机制描述。
    axiom_chain : tuple[str, ...]
        公理推导链。
    """

    description: str
    absorption_mechanism: tuple[str, ...]
    axiom_chain: tuple[str, ...]


def kernel_structure() -> KernelStructure:
    """返回 ker(D) 的结构性描述。

    ker(D) = {耦合模式 phi | D(phi) -> 0}

    从缠论公理推导：ker(D) 恰好是纯幅度耦合的集合。

    推导链：
    1. D1（笔）只编码方向，丢弃幅度 -> 幅度信息在 D1 已被丢弃
    2. D2（线段）通过特征序列合并吸收短程方向噪声
    3. D3（中枢）通过 [ZD,ZG] 区间吸收所有区间内振荡

    结论：耦合仅以"同涨同跌幅度不同"形式存在 -> D1 丢弃幅度差异
           -> 离散层面耦合 -> 0

    反之：耦合以"方向同步"形式存在 -> D1 保留方向
           -> D2/D3 进一步过滤但不消除结构性方向耦合
           -> 离散层面耦合 != 0

    Returns
    -------
    KernelStructure
    """
    return KernelStructure(
        description=(
            "ker(D) = 纯幅度耦合的集合。"
            "缠论离散化系统性地吸收幅度维度的信息，"
            "保留方向维度的信息。"
            "这不是信息损失，而是拓扑滤波："
            "幅度耦合被识别为'噪声'（FLAT 态吸收），"
            "方向耦合被识别为'信号'（走势方向编码）。"
        ),
        absorption_mechanism=(
            "D1（笔）：包含处理合并 bar，分型只检测方向极值。"
            "笔编码 = (起点, 终点, 方向)，无幅度字段。"
            "-> 幅度信息在第一层即被丢弃。",
            "D2（线段）：特征序列对反向笔做分型分析。"
            "短促振荡（笔破坏未发展为线段破坏）被吸收。"
            "-> 弱方向信号被归入'线段内部噪声'。",
            "D3（中枢→走势）：中枢区间 [ZD,ZG] 定义'无方向区域'。"
            "区间内所有振荡 = 盘整 = 无方向。"
            "走势方向仅由中枢间位置关系决定。"
            "-> 幅度耦合表现为区间内振荡，被 FLAT 态吸收。",
        ),
        axiom_chain=(
            "公理1（bi.md）：笔是走势的最小可编码单元，只编码方向。",
            "公理2（xianduan.md）：线段被终结当且仅当被线段终结。",
            "公理3（zhongshu.md）：中枢 = 至少三个连续次级别走势类型重叠。",
            "公理4（qushi.md）：只有趋势才有方向，盘整无方向。",
            "推论：D = D3.D2.D1 系统性地滤除幅度信息，"
            "保留方向信息。ker(D) = 纯幅度耦合。",
        ),
    )


# ── 耦合分类器 ──────────────────────────────────────────────


def classify_coupling(
    partial_corr: float,
    beta: float,
    *,
    beta_d3: float | None = None,
    kernel_threshold: float = 0.05,
    independence_threshold: float = 0.05,
    amplification_threshold: float = 5.0,
) -> CouplingClassification:
    """判断一对资产间的耦合是否属于 ker(D)。

    基于连续偏相关和离散联络参数的对比，判断耦合类型。
    235号扩展：当同时提供 D1 级 beta 和 D3 级 beta_d3 时，
    检测放大态（AMPLIFIED）。

    判断逻辑：
    1. 如果 |partial_corr| < independence_threshold -> INDEPENDENT（底空间独立）
    2. 如果提供 beta_d3 且 |beta_d3| / max(|beta|, epsilon) > amplification_threshold
       -> AMPLIFIED（方向耦合被递归放大）
    3. 如果 |partial_corr| 显著但 |beta| < kernel_threshold -> AMPLITUDE（ker(D)）
    4. 如果 |beta| >= kernel_threshold -> DIRECTION（not ker(D)）

    Parameters
    ----------
    partial_corr : float
        连续收益率的偏相关系数。
    beta : float
        离散联络参数（D1 级，纤维丛 softmax 模型的耦合系数）。
    beta_d3 : float | None
        D3 级离散联络参数（走势方向级别），可选。
        提供时启用三态放大检测。
    kernel_threshold : float
        离散耦合的显著性阈值。|beta| 低于此值视为被吸收。
    independence_threshold : float
        底空间独立性阈值。|partial_corr| 低于此值视为独立。
    amplification_threshold : float
        放大比阈值。|beta_d3| / max(|beta|, epsilon) 超过此值时分类为 AMPLIFIED。
        默认 5.0（五倍放大）。

    Returns
    -------
    CouplingClassification
    """
    abs_pc = abs(partial_corr)
    abs_beta = abs(beta)

    # Case 1: 底空间独立
    if abs_pc < independence_threshold:
        return CouplingClassification(
            coupling_type=CouplingType.INDEPENDENT,
            in_kernel=False,
            absorption_ratio=0.0,
            dominant_layer="none",
        )

    # 计算吸收率
    if abs_pc > 0:
        absorption = 1.0 - abs_beta / abs_pc
    else:
        absorption = 0.0

    # Case 2: 放大检测（235号三态扩展）
    if beta_d3 is not None:
        abs_beta_d3 = abs(beta_d3)
        _epsilon = 1e-10
        ratio = abs_beta_d3 / max(abs_beta, _epsilon)
        if ratio > amplification_threshold and abs_beta_d3 >= kernel_threshold:
            return CouplingClassification(
                coupling_type=CouplingType.AMPLIFIED,
                in_kernel=False,
                absorption_ratio=0.0,
                dominant_layer="none",
            )

    # Case 3: 连续层面有耦合，离散层面被吸收 -> ker(D)
    if abs_beta < kernel_threshold:
        return CouplingClassification(
            coupling_type=CouplingType.AMPLITUDE,
            in_kernel=True,
            absorption_ratio=max(0.0, min(1.0, absorption)),
            dominant_layer=_identify_dominant_layer(abs_pc, abs_beta),
        )

    # Case 4: 离散层面仍有显著耦合 -> not ker(D)
    return CouplingClassification(
        coupling_type=CouplingType.DIRECTION,
        in_kernel=False,
        absorption_ratio=max(0.0, min(1.0, absorption)),
        dominant_layer="none",
    )


def _identify_dominant_layer(abs_pc: float, abs_beta: float) -> str:
    """识别主要吸收层。

    D1 是幅度吸收的主要机制（笔丢弃幅度），
    D3 是二次吸收机制（中枢吸收区间内振荡）。
    D2 主要吸收弱方向信号，对幅度吸收贡献较小。

    Parameters
    ----------
    abs_pc : float
        连续偏相关绝对值。
    abs_beta : float
        离散联络参数绝对值。

    Returns
    -------
    str
        "D1", "D3", 或 "D1+D3"。
    """
    if abs_pc > 0.2 and abs_beta < 0.02:
        # 强连续耦合被几乎完全吸收 -> D1（方向编码丢弃幅度）+ D3（FLAT 态吸收）
        return "D1+D3"
    if abs_pc > 0.1:
        return "D1"
    return "D3"


# ── 放大分析（235号三态扩展）──────────────────────────────────


@dataclass(frozen=True, slots=True)
class AmplificationAnalysis:
    """方向耦合放大分析结果（235号）。

    当方向耦合在 D1→D3 过程中被放大时，记录放大的定量和定性信息。

    放大机制：D 的每一层滤除幅度噪声（ker 部分），使方向信号的
    信噪比递增。这不是绝对放大，而是噪声滤除后的相对放大。

    Attributes
    ----------
    amplification_ratio : float
        放大比 = |beta_d3| / max(|beta_d1|, epsilon)。
    mechanism : str
        放大机制的描述。
    resonance_layers : tuple[str, ...]
        产生共振/放大的层。
    """

    amplification_ratio: float
    mechanism: str
    resonance_layers: tuple[str, ...]


def analyze_amplification(
    beta_d1: float,
    beta_d3: float,
    partial_corr: float,
) -> AmplificationAnalysis:
    """分析方向耦合在 D1→D3 过程中的放大效应。

    从缠论公理推导放大机制：
    - D1（笔）保留方向但不放大：笔只编码方向，不累积信号
    - D2（线段）通过特征序列合并滤除短程噪声，使弱方向信号信噪比上升
    - D3（中枢→走势）通过 [ZD,ZG] 区间吸收幅度振荡，方向信号相对增强

    每一层滤除幅度噪声 = 放大方向信号的相对强度。

    Parameters
    ----------
    beta_d1 : float
        D1 级（笔方向）离散联络参数。
    beta_d3 : float
        D3 级（走势方向）离散联络参数。
    partial_corr : float
        连续层偏相关系数。

    Returns
    -------
    AmplificationAnalysis
    """
    _epsilon = 1e-10
    abs_d1 = abs(beta_d1)
    abs_d3 = abs(beta_d3)
    ratio = abs_d3 / max(abs_d1, _epsilon)

    # 识别共振层
    resonance: list[str] = []
    if abs_d1 < abs_d3:
        # D2 滤除短程噪声使信噪比上升
        resonance.append("D2")
        # D3 吸收区间振荡使方向信号相对增强
        resonance.append("D3")

    mechanism_parts: list[str] = []
    if "D2" in resonance:
        mechanism_parts.append(
            "D2（线段）通过特征序列合并滤除短程方向噪声，"
            "使弱方向信号的信噪比上升"
        )
    if "D3" in resonance:
        mechanism_parts.append(
            "D3（中枢→走势）通过 [ZD,ZG] 区间吸收幅度振荡，"
            "方向信号相对增强"
        )
    if not mechanism_parts:
        mechanism_parts.append("无显著放大（D1→D3 信号强度未增加）")

    mechanism = "；".join(mechanism_parts)

    return AmplificationAnalysis(
        amplification_ratio=ratio,
        mechanism=mechanism,
        resonance_layers=tuple(resonance),
    )


# ── 离散化算子（形式化表示）──────────────────────────────────


@dataclass(frozen=True, slots=True)
class DiscretizationOperator:
    """形式化的离散化算子 D: C -> S。

    C = 连续收益率配置空间
    S = {-1, 0, +1} 离散缠论走势方向空间

    D = D3 . D2 . D1

    此类不执行实际离散化（那是 pipeline 的工作），
    而是表达 D 的结构性质——特别是 ker(D) 的推导。

    Attributes
    ----------
    layers : tuple[DiscretizationLayer, ...]
        组成 D 的各层。
    """

    layers: tuple[DiscretizationLayer, ...] = DISCRETIZATION_LAYERS

    @property
    def kernel(self) -> KernelStructure:
        """ker(D) 的结构。"""
        return kernel_structure()

    def classify(
        self,
        partial_corr: float,
        beta: float,
        **kwargs: float,
    ) -> CouplingClassification:
        """分类耦合模式是否属于 ker(D)。

        Parameters
        ----------
        partial_corr : float
            连续收益率偏相关。
        beta : float
            离散联络参数。
        **kwargs
            传递给 classify_coupling 的额外参数。

        Returns
        -------
        CouplingClassification
        """
        return classify_coupling(partial_corr, beta, **kwargs)

    def layer_analysis(self) -> tuple[dict[str, str], ...]:
        """各层的吸收分析。

        Returns
        -------
        tuple[dict[str, str], ...]
            每层的分析字典。
        """
        return tuple(
            {
                "name": layer.name,
                "description": layer.description,
                "absorbs_amplitude": str(layer.absorbs_amplitude),
                "absorbs_weak_direction": str(layer.absorbs_weak_direction),
                "axiom": layer.axiom,
            }
            for layer in self.layers
        )

    def verify_232(self) -> dict[str, CouplingClassification]:
        """用 232号经验数据验证 ker(D) 理论。

        Returns
        -------
        dict[str, CouplingClassification]
            {"E-R": ..., "C-R": ..., "E-C": ...} 分类结果。
        """
        return {
            "E-R": self.classify(partial_corr=-0.336, beta=-0.017),
            "C-R": self.classify(partial_corr=0.153, beta=0.688),
            "E-C": self.classify(partial_corr=-0.029, beta=0.0),
        }

    def tristate_classify(
        self,
        partial_corr: float,
        beta_d1: float,
        beta_d3: float,
        **kwargs: float,
    ) -> CouplingClassification:
        """三态分类（235号）：吸收/保留/放大。

        同时考虑 D1 级和 D3 级离散参数，检测放大态。

        Parameters
        ----------
        partial_corr : float
            连续收益率偏相关。
        beta_d1 : float
            D1 级（笔方向）离散联络参数。
        beta_d3 : float
            D3 级（走势方向）离散联络参数。
        **kwargs
            传递给 classify_coupling 的额外关键字参数。

        Returns
        -------
        CouplingClassification
        """
        return classify_coupling(
            partial_corr, beta_d1, beta_d3=beta_d3, **kwargs,
        )

    def verify_234(self) -> dict[str, CouplingClassification]:
        """用 234号六条边数据验证三态分类。

        234号数据（4780 交易日，SPY/GLD/TLT/UUP）。
        C-R 使用双层数据（beta_d1=0.009, beta_d3=0.688）启用放大检测。
        其他五条边仅有 D1 层数据。

        Returns
        -------
        dict[str, CouplingClassification]
            六条边的分类结果。
        """
        return {
            # 吸收态：强连续耦合被 D 几乎完全吸收
            "E-R": self.classify(partial_corr=-0.317, beta=-0.020),
            "E-$": self.classify(partial_corr=-0.183, beta=-0.006),
            "C-$": self.classify(partial_corr=-0.414, beta=-0.030),
            # 吸收态（边界）：弱连续耦合，弱吸收
            "R-$": self.classify(partial_corr=-0.065, beta=0.050),
            # 放大态：D1 近零，D3 强耦合
            "C-R": self.tristate_classify(
                partial_corr=0.166, beta_d1=0.009, beta_d3=0.688,
            ),
            # 保留态：底空间独立（沿用 232号偏相关 -0.029）
            "E-C": self.classify(partial_corr=-0.029, beta=0.023),
        }
