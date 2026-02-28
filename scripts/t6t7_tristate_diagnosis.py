"""T6/T7 三态诊断脚本 — 237号谱系。

用 235号三态分类工具重新审视 229号 T6/T7 低贡献率问题。

核心问题：T6/T7 需要的跨级别耦合在三态分类中属于哪一态？
纤维丛修正（236号）对 T6/T7 可达性有何影响？

依赖：229号（T6/T7 贡献率分析）、235号（三态分类）、236号（纤维丛集成）。
"""

from __future__ import annotations

import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Literal

# 确保 src 在 path 中
_src = str(Path(__file__).resolve().parent.parent / "src")
if _src not in sys.path:
    sys.path.insert(0, _src)

from newchan.topology.discretization_kernel import (
    DISCRETIZATION_LAYERS,
    AmplificationAnalysis,
    CouplingClassification,
    CouplingType,
    DiscretizationOperator,
    analyze_amplification,
    classify_coupling,
)


# ── 信息类型分类 ──────────────────────────────────────────────


class InfoType:
    """判断步骤的信息类型。"""

    AMPLITUDE = "amplitude"  # 纯幅度信息
    DIRECTION = "direction"  # 方向信息
    MIXED = "mixed"  # 幅度+方向混合


@dataclass(frozen=True)
class JudgmentStep:
    """T6/T7 中单个判断步骤的信息类型标注。

    Attributes
    ----------
    name : str
        步骤名称。
    description : str
        步骤描述。
    info_type : str
        信息类型：amplitude / direction / mixed。
    d_layer_interaction : str
        与离散化层 D1/D2/D3 的交互方式。
    tristate_position : str
        在三态分类中的位置：absorbed / preserved / amplified / conditional。
    rationale : str
        推理依据。
    """

    name: str
    description: str
    info_type: str
    d_layer_interaction: str
    tristate_position: str
    rationale: str


# ── T6 嵌套背驰判断步骤分解 ──────────────────────────────────


def t6_judgment_steps() -> list[JudgmentStep]:
    """T6 嵌套背驰的每个判断步骤及其信息类型。

    T6 = nested_divergence_search，依赖：
    1. RecursiveStack 产出 recursive_levels >= 2
    2. 高级别背驰检测（_detect_level_trend_divergence / _detect_level_consolidation_divergence）
    3. 向下收缩 bar 范围
    4. level=1 MACD 背驰检测
    """
    return [
        JudgmentStep(
            name="recursive_depth_check",
            description="检查 snap.recursive_snapshots 非空（即 recursive_levels >= 2）",
            info_type=InfoType.DIRECTION,
            d_layer_interaction=(
                "D = D3.D2.D1 的递归应用。第一次 D 将 bar->bi->segment->zhongshu->move，"
                "第二次 D 将 settled(L1 moves) 作为新的 'bar' 再次 D。"
                "递归终止条件 = len(moves) < 3。"
            ),
            tristate_position="conditional",
            rationale=(
                "递归深度取决于 D 的压缩率。229号发现：500+ bi -> 2-9 segments -> 0-2 zhongshus -> "
                "recursive_levels=1。压缩率由线段定义的固有特性决定——线段至少三笔且需要特征序列分型"
                "确认终结。这是 D2 层对弱方向信号的吸收（古怪线段吸收机制）在递归叠加时的"
                "累积效应：D2 每吸收一轮短程方向噪声，下一层输入的数据量骤降。"
            ),
        ),
        JudgmentStep(
            name="level_k_trend_divergence",
            description="高级别趋势背驰：force_a vs force_c（价格振幅力度）",
            info_type=InfoType.MIXED,
            d_layer_interaction=(
                "force = (max_high - min_low) * duration。"
                "高级别无 MACD 数据，用价格振幅作为力度代理。"
                "这是幅度信息（振幅）+ 方向信息（趋势走势的方向）的混合。"
            ),
            tristate_position="absorbed",
            rationale=(
                "价格振幅力度的核心判据是 force_c < force_a（C段力度小于A段）。"
                "这依赖幅度比较——而幅度信息在 D1 层即被丢弃。"
                "高级别背驰使用振幅力度作为 MACD 的代理，但振幅恰好是 ker(D) 的典型成员。"
                "这意味着高级别背驰判断本质上在使用一个被 D 系统性吸收的信号。"
            ),
        ),
        JudgmentStep(
            name="level_k_consolidation_divergence",
            description="高级别盘整背驰：离开中枢的力度比较",
            info_type=InfoType.MIXED,
            d_layer_interaction=(
                "同样使用 _amplitude_force，即价格振幅力度。"
                "盘整中枢 [ZD,ZG] 由 D3 构造，离开中枢的方向是 D3 保留的信息，"
                "但力度比较是幅度信息。"
            ),
            tristate_position="absorbed",
            rationale=(
                "盘整背驰力度 = 离开中枢的振幅。"
                "中枢方向判定 ∉ ker(D)（D3 保留走势方向），"
                "但力度衰减判定 ∈ ker(D)（幅度信息被 D1 吸收）。"
                "混合判据中的关键比较（力度衰减）属于 ker(D)。"
            ),
        ),
        JudgmentStep(
            name="bar_range_narrowing",
            description="区间套向下收缩：从高级别 C 段映射到 bar 范围",
            info_type=InfoType.DIRECTION,
            d_layer_interaction=(
                "bar 范围映射只使用 seg_start/seg_end 索引（结构信息），"
                "不使用幅度。这是 D 保留的信息——位置关系。"
            ),
            tristate_position="preserved",
            rationale=(
                "区间套收缩纯粹基于位置索引映射，不涉及幅度。"
                "这是 D 的 image 部分——结构方向信息完整保留。"
            ),
        ),
        JudgmentStep(
            name="level1_macd_divergence",
            description="L1 MACD 背驰检测（在收缩后的 bar 范围内）",
            info_type=InfoType.MIXED,
            d_layer_interaction=(
                "MACD = EMA(close, 12) - EMA(close, 26)。"
                "MACD 力度 = 面积（MACD 值之和的绝对值）。"
                "close 是精确价格（幅度信息），但 MACD 经过 EMA 平滑后"
                "保留了方向趋势（低频方向信号通过 D2 线段滤噪后信噪比上升）。"
            ),
            tristate_position="amplified",
            rationale=(
                "L1 MACD 背驰在线段级别（D2 后）操作。"
                "234号数据表明：方向耦合在 D1→D3 过程中被放大（C-R beta_d1=0.009 -> beta_d3=0.688）。"
                "MACD 面积虽然包含幅度信息，但经过 D2 滤噪后方向信号被相对放大。"
                "然而，背驰的核心判据仍是 force_c < force_a（力度衰减），这是幅度比较。"
                "放大的是方向信号的信噪比，不是幅度比较的有效性。"
            ),
        ),
    ]


# ── T7 买卖点判断步骤分解 ──────────────────────────────────


def t7_judgment_steps() -> list[JudgmentStep]:
    """T7 买卖点的每个判断步骤及其信息类型。

    T7 = buysellpoints_from_level，依赖：
    1. segments, zhongshus, moves, divergences（全部由 D 构造）
    2. Type1: 趋势背驰点
    3. Type2: Type1 之后的回调/反弹
    4. Type3: 中枢突破后回试
    """
    return [
        JudgmentStep(
            name="type1_trend_divergence_trigger",
            description="Type1 买卖点触发条件：趋势背驰（div.kind == 'trend'）",
            info_type=InfoType.MIXED,
            d_layer_interaction=(
                "趋势背驰 = MACD 力度衰减。"
                "趋势方向 ∉ ker(D)（D3 保留），力度比较的幅度成分 ∈ ker(D)。"
            ),
            tristate_position="absorbed",
            rationale=(
                "Type1 的必要前提是趋势背驰，而趋势走势需要 ≥2 中枢。"
                "229号数据：L1 中枢 0-2 个，走势 0-1 个，趋势走势极稀缺。"
                "即使在有趋势走势的情况下，背驰力度判断的幅度成分 ∈ ker(D)。"
                "但 Type1 不依赖跨级别——它可以在 L1 单层触发。"
                "低产出原因不在三态位置，而在中枢/走势稀缺。"
            ),
        ),
        JudgmentStep(
            name="type1_direction_assignment",
            description="Type1 方向判定：div.direction == 'bottom' -> buy, 'top' -> sell",
            info_type=InfoType.DIRECTION,
            d_layer_interaction="方向由背驰方向决定，纯方向信息。",
            tristate_position="preserved",
            rationale="方向信息 ∉ ker(D)，在离散化后完全保留。",
        ),
        JudgmentStep(
            name="type2_rebound_search",
            description="Type2 回调/反弹搜索：找 Type1 之后的第一个反向段",
            info_type=InfoType.DIRECTION,
            d_layer_interaction=(
                "使用 segment.direction 搜索反向段。"
                "方向信息由 D1（笔方向）和 D2（线段方向）保留。"
            ),
            tristate_position="preserved",
            rationale=(
                "Type2 搜索纯粹基于线段方向，不使用幅度信息。"
                "方向 ∉ ker(D)。Type2 的低产出完全由 Type1 稀缺导致（Type2 依赖 Type1）。"
            ),
        ),
        JudgmentStep(
            name="type3_break_direction",
            description="Type3 中枢突破方向：zs.break_direction",
            info_type=InfoType.DIRECTION,
            d_layer_interaction=(
                "中枢突破方向由 D3 构造保留。"
                "break_direction = 离开中枢的方向（up/down）。"
            ),
            tristate_position="preserved",
            rationale="方向信息 ∉ ker(D)。",
        ),
        JudgmentStep(
            name="type3_pullback_range_check",
            description="Type3 回试范围检查：pullback_seg.low > zs.zg 或 pullback_seg.high < zs.zd",
            info_type=InfoType.AMPLITUDE,
            d_layer_interaction=(
                "zg/zd 是中枢区间边界（D3 构造的结构信息，但数值来源是线段端点价格）。"
                "pullback_seg.low/high 是线段端点价格。"
                "比较 low > zg 或 high < zd 是幅度比较。"
            ),
            tristate_position="preserved",
            rationale=(
                "这里的幅度比较有特殊性：它不是判断力度衰减（那属于 ker(D)），"
                "而是判断位置关系（回试后仍在中枢之外）。"
                "位置关系 = 结构信息，由 D3 保留。"
                "虽然使用价格数值，但判据实质是拓扑判断（在/不在中枢区间内），"
                "不是幅度大小比较。∉ ker(D)。"
            ),
        ),
    ]


# ── 纤维丛视角分析 ──────────────────────────────────────────


@dataclass(frozen=True)
class FiberBundleImpact:
    """纤维丛修正对 T6/T7 的影响分析。

    Attributes
    ----------
    t6_uses_polarity : bool
        T6 代码路径是否使用 polarity_index 或 Configuration。
    t7_uses_polarity : bool
        T7 代码路径是否使用 polarity_index 或 Configuration。
    fiber_correction_channel : str
        纤维丛修正可能影响 T6/T7 的通道描述。
    impact_assessment : str
        影响评估。
    """

    t6_uses_polarity: bool
    t7_uses_polarity: bool
    fiber_correction_channel: str
    impact_assessment: str


def analyze_fiber_bundle_impact() -> FiberBundleImpact:
    """分析纤维丛修正对 T6/T7 的影响。

    代码审查结论：
    - a_nested_divergence.py: 无 import Configuration/polarity_index
    - a_buysellpoint_v1.py: 无 import Configuration/polarity_index
    - 两者都只操作 bi/segment/zhongshu/move/divergence 层面的对象

    纤维丛修正的作用域：
    - pipeline.py 中 polarity_index(config) 用于 scan_configuration
    - FiberPipelineAdapter 修正 Configuration -> FiberCorrection
    - 修正输出到 TradingContext.polarity 和策略层

    T6/T7 操作在 pipeline 的上游（构造层），而纤维丛修正在下游（策略层）。
    """
    return FiberBundleImpact(
        t6_uses_polarity=False,
        t7_uses_polarity=False,
        fiber_correction_channel=(
            "纤维丛修正 → TradingContext.polarity → scan_configuration 方向 → "
            "区间套定位/共振仓位/状态机。"
            "T6/T7 不在此通道上——它们在构造层（bi/segment/zhongshu/move）操作，"
            "不依赖 Configuration 或 polarity_index。"
        ),
        impact_assessment=(
            "纤维丛修正对 T6/T7 **无直接影响**。"
            "236号发现的 45% polarity 差异影响的是策略层（scan_configuration → 仓位），"
            "不影响构造层（bi → segment → zhongshu → move → divergence → buysellpoint）。"
            "T6/T7 的低贡献率根因在递归深度不足，这是构造层（D 算子的递归应用）的问题，"
            "与纤维丛修正所在的策略层无关。"
            "\n\n"
            "唯一的间接通道：如果纤维丛修正改变了交易决策"
            "（通过 polarity 影响入退场），可能影响对 T6/T7 信号的 *使用*，"
            "但不影响 T6/T7 信号的 *产生*。产生问题是 229号的核心问题。"
        ),
    )


# ── 递归深度与 D 迭代的关系 ──────────────────────────────────


@dataclass(frozen=True)
class RecursiveDepthAnalysis:
    """递归深度在三态框架下的分析。

    Attributes
    ----------
    d_iteration_count : int
        D 需要成功迭代的次数才能达到 recursive_levels >= 2。
    compression_per_iteration : str
        每次 D 迭代的压缩率描述。
    amplification_after_two_iterations : str
        两次 D 迭代后放大效应的状态。
    structural_barrier : str
        结构性障碍描述。
    """

    d_iteration_count: int
    compression_per_iteration: str
    amplification_after_two_iterations: str
    structural_barrier: str


def analyze_recursive_depth() -> RecursiveDepthAnalysis:
    """分析递归深度在三态框架下的状态。

    229号数据：
    - AAPL 日线 6621 bar → 574 bi → 7 segments → 1 zhongshu → 1 move → recursive_levels=1
    - GOOGL 日线 5416 bar → 476 bi → 9 segments → 2 zhongshus → 1 move → recursive_levels=1

    D 的压缩链：
    - D1: bar → bi（保留方向极值，压缩率 ~10:1）
    - D2: bi → segment（特征序列合并，压缩率 ~50-100:1）
    - D3: segment → zhongshu → move（中枢吸收，压缩率 ~3-10:1）
    - 总压缩率 D = D3.D2.D1: ~6000:1 → 1 move

    要达到 recursive_levels=2，需要 D(moves) 产出 ≥ 3 个 L2 moves。
    这意味着 L1 需要 settled moves ≥ 3 → L1 zhongshus ≥ 2（趋势）或 moves ≥ 3（多个走势类型）。
    """
    return RecursiveDepthAnalysis(
        d_iteration_count=2,
        compression_per_iteration=(
            "D1: ~10:1（bar→bi）。"
            "D2: ~50-100:1（bi→segment，线段引擎压缩率极高——229号实证）。"
            "D3: ~3-10:1（segment→zhongshu→move）。"
            "单次 D 迭代总压缩率 ~1500-10000:1。"
        ),
        amplification_after_two_iterations=(
            "235号放大效应是针对跨资产方向耦合（beta_d1→beta_d3）。"
            "递归深度问题不是跨资产耦合问题，而是单资产内部的数据量问题。"
            "D 的放大效应（滤噪后方向信号信噪比上升）无法解决数据量不足——"
            "放大的是信号质量，不是信号数量。"
            "recursive_levels=1 意味着 D 的第一次迭代已经把数据压缩到"
            "只剩 0-1 个 move，第二次迭代无数据可处理。"
        ),
        structural_barrier=(
            "D2（线段引擎）的压缩率是结构性瓶颈。"
            "缠论线段定义的固有特性：线段至少三笔、需要特征序列分型确认终结、"
            "古怪线段被吸收（D2 absorbs_weak_direction=True）。"
            "这些公理性约束使得长期数据中的笔被大规模合并为极少线段。"
            "例如 GOOGL 388 笔 ≈ 14年数据被合并为 1 条下跌线段。"
            "这不是实现缺陷——这是 D2 层按公理执行弱方向信号吸收的结果。"
            "在三态框架下，D2 的弱方向吸收导致递归输入数据量骤降，"
            "这是 ker(D) 在递归维度上的表现——不是幅度被吸收，"
            "而是弱方向信号在递归叠加中被累积吸收。"
        ),
    )


# ── 综合诊断 ──────────────────────────────────────────────────


@dataclass(frozen=True)
class TristatePosition:
    """三态分类汇总。"""

    category: str  # absorbed / preserved / amplified / conditional
    description: str


@dataclass(frozen=True)
class DiagnosisResult:
    """T6/T7 三态诊断综合结果。

    Attributes
    ----------
    t6_steps : list[JudgmentStep]
        T6 各步骤分析。
    t7_steps : list[JudgmentStep]
        T7 各步骤分析。
    t6_tristate_summary : TristatePosition
        T6 在三态中的总位置。
    t7_tristate_summary : TristatePosition
        T7 在三态中的总位置。
    fiber_impact : FiberBundleImpact
        纤维丛影响分析。
    recursive_depth : RecursiveDepthAnalysis
        递归深度分析。
    root_cause : str
        根因分类。
    actionable_paths : list[str]
        可操作路径。
    """

    t6_steps: list[JudgmentStep]
    t7_steps: list[JudgmentStep]
    t6_tristate_summary: TristatePosition
    t7_tristate_summary: TristatePosition
    fiber_impact: FiberBundleImpact
    recursive_depth: RecursiveDepthAnalysis
    root_cause: str
    actionable_paths: list[str]


def run_diagnosis() -> DiagnosisResult:
    """执行完整的 T6/T7 三态诊断。"""

    t6_steps = t6_judgment_steps()
    t7_steps = t7_judgment_steps()
    fiber_impact = analyze_fiber_bundle_impact()
    recursive_depth = analyze_recursive_depth()

    t6_summary = TristatePosition(
        category="conditional-on-absorbed",
        description=(
            "T6 的瓶颈是递归深度不足（recursive_levels=1）。"
            "递归深度不足的根因是 D2 层对弱方向信号的累积吸收——"
            "每次递归迭代，D2 都大规模压缩输入数据量。"
            "在三态框架中，这属于 ker(D) 在递归维度的表现："
            "D2 的弱方向吸收使得递归叠加后数据量指数衰减。"
            "T6 的两个背驰力度判断步骤（level_k_trend/consolidation）"
            "使用价格振幅力度（∈ ker(D)），进一步削弱了信号质量。"
            "结论：T6 低贡献率是结构性的——在单 TF 递归架构下不可改变。"
        ),
    )

    t7_summary = TristatePosition(
        category="preserved-but-starved",
        description=(
            "T7 的核心判断步骤大多基于方向信息（∉ ker(D)）。"
            "Type3 的位置判断（low > zg / high < zd）虽使用价格数值，"
            "但实质是拓扑判断（中枢内外），∉ ker(D)。"
            "T7 低产出不是因为信号被 D 吸收，"
            "而是因为上游结构稀缺——中枢/走势/背驰太少，导致买卖点触发条件难以满足。"
            "T7 的三态位置是 preserved（方向信息保留），但上游 'starved'（数据不足）。"
            "Type1 依赖趋势背驰（趋势需要 ≥2 中枢，当前数据 0-2 中枢）。"
            "Type2 依赖 Type1（Type1 稀缺 → Type2 更稀缺）。"
            "Type3 依赖中枢突破（中枢稀缺 → Type3 稀缺）。"
        ),
    )

    root_cause = (
        "T6/T7 低贡献率的根因在三态框架下分属两类：\n"
        "1. T6: ker(D) 在递归维度的累积效应——D2 弱方向吸收导致递归输入"
        "指数衰减，recursive_levels 无法突破 1。这是结构性不可达。"
        "高级别背驰力度判断使用的幅度信息也 ∈ ker(D)，双重削弱。\n"
        "2. T7: ∉ ker(D) 但上游 starved——买卖点判断步骤大多基于方向信息"
        "（preserved），但上游构造层产出（中枢/走势/背驰）不足。"
        "上游不足又回到 D2 的压缩率问题。\n"
        "3. 纤维丛修正（236号）对两者无直接影响——T6/T7 在构造层操作，"
        "纤维丛修正在策略层操作，两者在 pipeline 中没有数据通路。"
    )

    actionable_paths = [
        (
            "多 TF 输入（multi-timeframe）：229号已识别此路径。"
            "用不同 TF 的 bar 数据分别构建各级别，"
            "绕过单 TF 递归中 D2 的压缩率瓶颈。"
            "三态分析支持此结论：问题出在 D2 的累积吸收，"
            "多 TF 直接绕过 D2 递归叠加。"
        ),
        (
            "高级别背驰力度替代指标：当前 _amplitude_force 使用价格振幅"
            "（∈ ker(D)）。如果有方向性力度指标（∉ ker(D)），"
            "高级别背驰判断可能更有效。但这不解决递归深度问题，"
            "只优化假设递归深度足够时的信号质量。"
        ),
        (
            "不可操作（结构性约束）：D2 的弱方向吸收是缠论线段定义的"
            "固有特性。修改 D2 = 修改线段定义 = 修改缠论公理。"
            "这不是工程选择，是公理约束。"
        ),
    ]

    return DiagnosisResult(
        t6_steps=t6_steps,
        t7_steps=t7_steps,
        t6_tristate_summary=t6_summary,
        t7_tristate_summary=t7_summary,
        fiber_impact=fiber_impact,
        recursive_depth=recursive_depth,
        root_cause=root_cause,
        actionable_paths=actionable_paths,
    )


# ── 序列化输出 ──────────────────────────────────────────────


def _step_to_dict(step: JudgmentStep) -> dict:
    return {
        "name": step.name,
        "description": step.description,
        "info_type": step.info_type,
        "d_layer_interaction": step.d_layer_interaction,
        "tristate_position": step.tristate_position,
        "rationale": step.rationale,
    }


def result_to_dict(result: DiagnosisResult) -> dict:
    """将诊断结果转换为可 JSON 序列化的字典。"""
    return {
        "metadata": {
            "genealogy_id": 237,
            "title": "T6/T7 三态诊断——229号递归深度问题的三态根因分类",
            "depends_on": [229, 235, 236],
            "date": "2026-02-28",
        },
        "t6_analysis": {
            "steps": [_step_to_dict(s) for s in result.t6_steps],
            "tristate_summary": {
                "category": result.t6_tristate_summary.category,
                "description": result.t6_tristate_summary.description,
            },
        },
        "t7_analysis": {
            "steps": [_step_to_dict(s) for s in result.t7_steps],
            "tristate_summary": {
                "category": result.t7_tristate_summary.category,
                "description": result.t7_tristate_summary.description,
            },
        },
        "fiber_bundle_impact": {
            "t6_uses_polarity": result.fiber_impact.t6_uses_polarity,
            "t7_uses_polarity": result.fiber_impact.t7_uses_polarity,
            "fiber_correction_channel": result.fiber_impact.fiber_correction_channel,
            "impact_assessment": result.fiber_impact.impact_assessment,
        },
        "recursive_depth_analysis": {
            "d_iteration_count": result.recursive_depth.d_iteration_count,
            "compression_per_iteration": result.recursive_depth.compression_per_iteration,
            "amplification_after_two_iterations": result.recursive_depth.amplification_after_two_iterations,
            "structural_barrier": result.recursive_depth.structural_barrier,
        },
        "root_cause": result.root_cause,
        "actionable_paths": result.actionable_paths,
    }


def main() -> None:
    """执行诊断并输出 JSON。"""
    result = run_diagnosis()
    output = result_to_dict(result)

    out_path = Path(__file__).resolve().parent.parent / "tmp" / "t6t7-tristate-diagnosis.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"诊断结果已写入: {out_path}")

    # 打印摘要
    print("\n=== T6 三态位置 ===")
    print(f"分类: {result.t6_tristate_summary.category}")
    print(f"描述: {result.t6_tristate_summary.description[:200]}...")

    print("\n=== T7 三态位置 ===")
    print(f"分类: {result.t7_tristate_summary.category}")
    print(f"描述: {result.t7_tristate_summary.description[:200]}...")

    print("\n=== 纤维丛影响 ===")
    print(f"T6 使用 polarity: {result.fiber_impact.t6_uses_polarity}")
    print(f"T7 使用 polarity: {result.fiber_impact.t7_uses_polarity}")

    print("\n=== 根因 ===")
    print(result.root_cause)


if __name__ == "__main__":
    main()
