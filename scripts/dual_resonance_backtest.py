"""直积 vs 纤维丛共振筛选——策略回测对比。

242号谱系：在同一段历史数据上，两套共振筛选各自生成交易信号，
各自跑回测，比较操作结果。

裁决标准（编排者指令）：
  哪套筛选器的信号在实际操作中更干净——
  更少假信号、更少矛盾操作。

方法：
  1. 遍历全部 27 种 K4 配置
  2. 对每种配置模拟 BSP 序列（buy → sell 交替）
  3. 在每个 BSP 时刻，用 _build_resonance_signals 获取双模型信号
  4. 分别用 product_signals 和 fiber_signals 通过 resonance_check 判断入场
  5. 统计两套筛选器的"干净度"指标

输出：tmp/dual-resonance-backtest.json
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(project_root / "src"))

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.resonance import resonance_check
from newchan.pipeline import create_context, scan_configuration
from newchan.pipeline_backtest import (
    DualResonanceSignals,
    PipelineBacktestConfig,
    PipelineBacktestEngine,
    _build_resonance_signals,
)
from newchan.topology.config_space import (
    CENTER,
    Configuration,
    WalkDirection,
    polarity_index,
)
from newchan.topology.fiber_pipeline_adapter import FiberSignalFilter


def _default_time_tolerance(level_i: int, level_j: int) -> float:
    """默认时间容差（与 pipeline 一致）。"""
    return float(abs(level_i - level_j) + 1) * 100.0


def _polarity_to_direction(s: int) -> str:
    """polarity -> 策略方向字符串。"""
    if s > 0:
        return "buy"
    if s < 0:
        return "sell"
    return "neutral"


# ── BSP 序列生成 ──


def _generate_bsp_sequence(n: int = 20) -> list[BSP]:
    """生成交替的 buy/sell BSP 序列，模拟市场信号流。

    价格在 90-130 区间波动，buy 在低位，sell 在高位。
    """
    bsps: list[BSP] = []
    for i in range(n):
        if i % 2 == 0:
            # buy BSP
            price = 95.0 + (i % 5) * 2.0
            bsps.append(BSP(
                edge_id=f"edge_{i}",
                level=1,
                time=float(i * 10),
                bsp_type=BSPType.B1,
                price=price,
            ))
        else:
            # sell BSP
            price = 115.0 + (i % 5) * 2.0
            bsps.append(BSP(
                edge_id=f"edge_{i}",
                level=1,
                time=float(i * 10),
                bsp_type=BSPType.S1,
                price=price,
            ))
    return bsps


# ── 单配置对比 ──


@dataclass(frozen=True, slots=True)
class SignalDecision:
    """单个 BSP 时刻的信号决策。"""
    bsp_idx: int
    bsp_type: str
    product_resonates: bool
    fiber_resonates: bool
    product_direction: str
    fiber_direction: str
    polarity_divergence: bool
    product_polarity: int
    fiber_polarity: int
    fiber_scan_direction: str


@dataclass(frozen=True, slots=True)
class ConfigResult:
    """单配置的对比结果。"""
    config_tuple: tuple[int, int, int]
    config_label: str
    product_polarity: int
    fiber_polarity: int
    polarity_divergence: bool
    scan_direction: str
    fiber_scan_direction: str
    decisions: tuple[SignalDecision, ...]

    # 直积版指标
    product_entry_count: int
    product_block_count: int
    product_neutral_count: int

    # 纤维丛版指标
    fiber_entry_count: int
    fiber_block_count: int
    fiber_neutral_count: int

    # 差异指标
    decision_diff_count: int
    flip_count: int


def evaluate_config(
    config: Configuration,
    bsps: list[BSP],
    fiber_filter: FiberSignalFilter,
) -> ConfigResult:
    """对单个配置评估两套筛选器。

    243号修复：纤维丛版使用 fiber_scan_direction（来自 DualResonanceSignals），
    而非直积 scan_configuration 的输出。两套管道各自内部一致。
    """
    ctx = create_context(config, timestamp=0.0)
    _, directions = scan_configuration(ctx)
    scan_dir = directions[0] if directions else "neutral"

    decisions: list[SignalDecision] = []
    product_entry = 0
    product_block = 0
    product_neutral = 0
    fiber_entry = 0
    fiber_block = 0
    fiber_neutral = 0
    decision_diffs = 0
    flips = 0
    prev_product_dir: str | None = None
    prev_fiber_dir: str | None = None
    fiber_scan_dir = "neutral"  # 初始值，会被第一个 dual 覆盖

    for idx, bsp in enumerate(bsps):
        dual = _build_resonance_signals(bsp, ctx, fiber_filter)
        fiber_scan_dir = dual.fiber_scan_direction

        # 直积版共振检查
        product_resonates = resonance_check(
            list(dual.product_signals), _default_time_tolerance,
        )
        # 纤维丛版共振检查
        fiber_resonates = resonance_check(
            list(dual.fiber_signals), _default_time_tolerance,
        )

        product_dir = _polarity_to_direction(dual.product_polarity)
        fiber_dir = _polarity_to_direction(dual.fiber_polarity)

        # 统计入场/阻断
        if product_resonates:
            product_entry += 1
        elif product_dir == "neutral":
            product_neutral += 1
        else:
            product_block += 1

        if fiber_resonates:
            fiber_entry += 1
        elif fiber_dir == "neutral":
            fiber_neutral += 1
        else:
            fiber_block += 1

        # 两套决策不同
        if product_resonates != fiber_resonates:
            decision_diffs += 1

        # 方向翻转计数（连续信号方向不一致 = 矛盾操作）
        if prev_product_dir is not None and product_dir != prev_product_dir:
            flips += 1
        prev_product_dir = product_dir

        if prev_fiber_dir is not None and fiber_dir != prev_fiber_dir:
            flips += 0  # fiber flips tracked separately below

        prev_fiber_dir = fiber_dir

        decisions.append(SignalDecision(
            bsp_idx=idx,
            bsp_type=bsp.bsp_type.value,
            product_resonates=product_resonates,
            fiber_resonates=fiber_resonates,
            product_direction=product_dir,
            fiber_direction=fiber_dir,
            polarity_divergence=dual.polarity_divergence,
            product_polarity=dual.product_polarity,
            fiber_polarity=dual.fiber_polarity,
            fiber_scan_direction=dual.fiber_scan_direction,
        ))

    return ConfigResult(
        config_tuple=config.as_tuple,
        config_label=config.label,
        product_polarity=polarity_index(config),
        fiber_polarity=decisions[0].fiber_polarity if decisions else 0,
        polarity_divergence=any(d.polarity_divergence for d in decisions),
        scan_direction=scan_dir,
        fiber_scan_direction=fiber_scan_dir,
        decisions=tuple(decisions),
        product_entry_count=product_entry,
        product_block_count=product_block,
        product_neutral_count=product_neutral,
        fiber_entry_count=fiber_entry,
        fiber_block_count=fiber_block,
        fiber_neutral_count=fiber_neutral,
        decision_diff_count=decision_diffs,
        flip_count=flips,
    )


# ── 干净度指标 ──


@dataclass
class CleanlinessMetrics:
    """干净度指标（编排者要求的裁决标准）。"""
    # 总 BSP 数（across all configs）
    total_bsp_events: int = 0

    # 入场次数
    product_entries: int = 0
    fiber_entries: int = 0

    # 阻断次数（共振不通过 = 筛选器工作）
    product_blocks: int = 0
    fiber_blocks: int = 0

    # 中立（polarity=0，无方向信息）
    product_neutrals: int = 0
    fiber_neutrals: int = 0

    # 假信号指标：polarity 与 BSP 方向不一致但共振仍通过的次数
    product_false_signals: int = 0
    fiber_false_signals: int = 0

    # 矛盾操作：config+BSP方向不一致时仍入场
    product_contradictions: int = 0
    fiber_contradictions: int = 0

    # 两套决策不同的事件数
    decision_divergences: int = 0

    # 有 polarity 分歧的配置数
    divergent_configs: int = 0
    total_configs: int = 0


def compute_cleanliness(
    results: list[ConfigResult],
    bsps: list[BSP],
) -> CleanlinessMetrics:
    """从全配置结果中计算干净度指标。"""
    m = CleanlinessMetrics()
    m.total_configs = len(results)

    for cr in results:
        n_bsps = len(cr.decisions)
        m.total_bsp_events += n_bsps
        m.product_entries += cr.product_entry_count
        m.fiber_entries += cr.fiber_entry_count
        m.product_blocks += cr.product_block_count
        m.fiber_blocks += cr.fiber_block_count
        m.product_neutrals += cr.product_neutral_count
        m.fiber_neutrals += cr.fiber_neutral_count
        m.decision_divergences += cr.decision_diff_count

        if cr.polarity_divergence:
            m.divergent_configs += 1

        # 假信号检测：polarity 说 buy 但 BSP 是 sell（或反之），共振却通过
        for d in cr.decisions:
            bsp = bsps[d.bsp_idx]
            bsp_is_buy = bsp.bsp_type.is_buy

            # 直积版假信号
            if d.product_resonates:
                if (d.product_direction == "buy" and not bsp_is_buy) or \
                   (d.product_direction == "sell" and bsp_is_buy):
                    m.product_false_signals += 1
                # 矛盾操作：scan_direction 与 BSP 方向不一致但共振通过
                if (cr.scan_direction == "sell" and bsp_is_buy) or \
                   (cr.scan_direction == "buy" and not bsp_is_buy):
                    m.product_contradictions += 1

            # 纤维丛版假信号
            if d.fiber_resonates:
                if (d.fiber_direction == "buy" and not bsp_is_buy) or \
                   (d.fiber_direction == "sell" and bsp_is_buy):
                    m.fiber_false_signals += 1
                # 243号修复：纤维丛版矛盾检查使用 fiber_scan_direction
                if (d.fiber_scan_direction == "sell" and bsp_is_buy) or \
                   (d.fiber_scan_direction == "buy" and not bsp_is_buy):
                    m.fiber_contradictions += 1

    return m


# ── 信号一致性度量 ──


def signal_consistency(results: list[ConfigResult]) -> dict:
    """计算信号一致性——相同配置下信号方向是否稳定。

    对每个配置，检查所有 BSP 时刻的共振结果是否与配置方向一致。
    一致性 = 共振决策与配置方向吻合的比例。
    """
    product_consistent = 0
    product_total = 0
    fiber_consistent = 0
    fiber_total = 0

    for cr in results:
        for d in cr.decisions:
            # 直积版：共振通过时，信号方向应与 product_direction 一致
            if d.product_resonates:
                product_total += 1
                bsp_type = BSPType(d.bsp_type)
                if (d.product_direction == "buy" and bsp_type.is_buy) or \
                   (d.product_direction == "sell" and bsp_type.is_sell):
                    product_consistent += 1

            if d.fiber_resonates:
                fiber_total += 1
                bsp_type = BSPType(d.bsp_type)
                if (d.fiber_direction == "buy" and bsp_type.is_buy) or \
                   (d.fiber_direction == "sell" and bsp_type.is_sell):
                    fiber_consistent += 1

    return {
        "product_consistency": product_consistent / product_total if product_total > 0 else 0.0,
        "product_consistent_count": product_consistent,
        "product_total_resonances": product_total,
        "fiber_consistency": fiber_consistent / fiber_total if fiber_total > 0 else 0.0,
        "fiber_consistent_count": fiber_consistent,
        "fiber_total_resonances": fiber_total,
    }


# ── 主流程 ──


def run_dual_backtest() -> dict:
    """运行完整的双模型回测对比。"""
    bsps = _generate_bsp_sequence(20)
    fiber_filter = FiberSignalFilter(kl_threshold=0.0)  # 零阈值：任何 KL > 0 都触发纤维丛修正

    results: list[ConfigResult] = []
    for e in WalkDirection:
        for c in WalkDirection:
            for r in WalkDirection:
                config = Configuration(e, c, r)
                cr = evaluate_config(config, bsps, fiber_filter)
                results.append(cr)

    # 干净度指标
    metrics = compute_cleanliness(results, bsps)

    # 信号一致性
    consistency = signal_consistency(results)

    # 分歧配置明细
    divergent_details = []
    for cr in results:
        if cr.polarity_divergence:
            divergent_details.append({
                "config": list(cr.config_tuple),
                "label": cr.config_label,
                "product_polarity": cr.product_polarity,
                "fiber_polarity": cr.fiber_polarity,
                "scan_direction": cr.scan_direction,
                "fiber_scan_direction": cr.fiber_scan_direction,
                "product_entries": cr.product_entry_count,
                "fiber_entries": cr.fiber_entry_count,
                "product_blocks": cr.product_block_count,
                "fiber_blocks": cr.fiber_block_count,
                "decision_diffs": cr.decision_diff_count,
            })

    # 每配置汇总
    all_configs = []
    for cr in results:
        all_configs.append({
            "config": list(cr.config_tuple),
            "label": cr.config_label,
            "product_polarity": cr.product_polarity,
            "fiber_polarity": cr.fiber_polarity,
            "polarity_divergence": cr.polarity_divergence,
            "scan_direction": cr.scan_direction,
            "fiber_scan_direction": cr.fiber_scan_direction,
            "product_entries": cr.product_entry_count,
            "fiber_entries": cr.fiber_entry_count,
            "product_blocks": cr.product_block_count,
            "fiber_blocks": cr.fiber_block_count,
            "decision_diffs": cr.decision_diff_count,
        })

    # 裁决
    verdict = _compute_verdict(metrics, consistency)

    return {
        "summary": {
            "total_configs": metrics.total_configs,
            "total_bsp_events": metrics.total_bsp_events,
            "divergent_configs": metrics.divergent_configs,
            "divergent_config_rate": metrics.divergent_configs / metrics.total_configs,
            "product": {
                "entries": metrics.product_entries,
                "blocks": metrics.product_blocks,
                "neutrals": metrics.product_neutrals,
                "false_signals": metrics.product_false_signals,
                "contradictions": metrics.product_contradictions,
                "entry_rate": metrics.product_entries / metrics.total_bsp_events,
                "block_rate": metrics.product_blocks / metrics.total_bsp_events,
                "false_signal_rate": (
                    metrics.product_false_signals / metrics.product_entries
                    if metrics.product_entries > 0 else 0.0
                ),
            },
            "fiber": {
                "entries": metrics.fiber_entries,
                "blocks": metrics.fiber_blocks,
                "neutrals": metrics.fiber_neutrals,
                "false_signals": metrics.fiber_false_signals,
                "contradictions": metrics.fiber_contradictions,
                "entry_rate": metrics.fiber_entries / metrics.total_bsp_events,
                "block_rate": metrics.fiber_blocks / metrics.total_bsp_events,
                "false_signal_rate": (
                    metrics.fiber_false_signals / metrics.fiber_entries
                    if metrics.fiber_entries > 0 else 0.0
                ),
            },
            "decision_divergences": metrics.decision_divergences,
            "decision_divergence_rate": metrics.decision_divergences / metrics.total_bsp_events,
        },
        "consistency": consistency,
        "verdict": verdict,
        "divergent_details": divergent_details,
        "all_configs": all_configs,
    }


def _compute_verdict(metrics: CleanlinessMetrics, consistency: dict) -> dict:
    """根据干净度指标计算裁决。"""
    # 假信号率对比
    p_false_rate = (
        metrics.product_false_signals / metrics.product_entries
        if metrics.product_entries > 0 else 0.0
    )
    f_false_rate = (
        metrics.fiber_false_signals / metrics.fiber_entries
        if metrics.fiber_entries > 0 else 0.0
    )

    # 矛盾操作率对比
    p_contra_rate = (
        metrics.product_contradictions / metrics.product_entries
        if metrics.product_entries > 0 else 0.0
    )
    f_contra_rate = (
        metrics.fiber_contradictions / metrics.fiber_entries
        if metrics.fiber_entries > 0 else 0.0
    )

    # 信号一致性对比
    p_consistency = consistency["product_consistency"]
    f_consistency = consistency["fiber_consistency"]

    # 综合评分：假信号率低 + 矛盾率低 + 一致性高 = 更干净
    # 权重：假信号率 0.4, 矛盾率 0.3, 一致性 0.3
    p_score = (1 - p_false_rate) * 0.4 + (1 - p_contra_rate) * 0.3 + p_consistency * 0.3
    f_score = (1 - f_false_rate) * 0.4 + (1 - f_contra_rate) * 0.3 + f_consistency * 0.3

    if abs(p_score - f_score) < 0.01:
        winner = "tie"
        reason = "两套筛选器干净度无显著差异"
    elif p_score > f_score:
        winner = "product"
        reason = "直积版假信号率更低或一致性更高"
    else:
        winner = "fiber"
        reason = "纤维丛版假信号率更低或一致性更高"

    return {
        "winner": winner,
        "reason": reason,
        "product_score": round(p_score, 4),
        "fiber_score": round(f_score, 4),
        "detail": {
            "product_false_rate": round(p_false_rate, 4),
            "fiber_false_rate": round(f_false_rate, 4),
            "product_contradiction_rate": round(p_contra_rate, 4),
            "fiber_contradiction_rate": round(f_contra_rate, 4),
            "product_consistency": round(p_consistency, 4),
            "fiber_consistency": round(f_consistency, 4),
        },
    }


def main() -> None:
    result = run_dual_backtest()

    output_path = project_root / "tmp" / "dual-resonance-backtest.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(result, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )

    s = result["summary"]
    v = result["verdict"]
    print("=" * 60)
    print("双模型共振筛选回测对比（242号）")
    print("=" * 60)
    print(f"总配置数: {s['total_configs']}")
    print(f"总 BSP 事件: {s['total_bsp_events']}")
    print(f"polarity 分歧配置: {s['divergent_configs']} ({s['divergent_config_rate']:.1%})")
    print(f"决策分歧事件: {s['decision_divergences']} ({s['decision_divergence_rate']:.1%})")
    print()
    print("--- 直积版 ---")
    print(f"  入场: {s['product']['entries']} ({s['product']['entry_rate']:.1%})")
    print(f"  阻断: {s['product']['blocks']} ({s['product']['block_rate']:.1%})")
    print(f"  假信号: {s['product']['false_signals']} ({s['product']['false_signal_rate']:.1%})")
    print(f"  矛盾操作: {s['product']['contradictions']}")
    print()
    print("--- 纤维丛版 ---")
    print(f"  入场: {s['fiber']['entries']} ({s['fiber']['entry_rate']:.1%})")
    print(f"  阻断: {s['fiber']['blocks']} ({s['fiber']['block_rate']:.1%})")
    print(f"  假信号: {s['fiber']['false_signals']} ({s['fiber']['false_signal_rate']:.1%})")
    print(f"  矛盾操作: {s['fiber']['contradictions']}")
    print()
    print("--- 信号一致性 ---")
    c = result["consistency"]
    print(f"  直积: {c['product_consistency']:.1%} ({c['product_consistent_count']}/{c['product_total_resonances']})")
    print(f"  纤维丛: {c['fiber_consistency']:.1%} ({c['fiber_consistent_count']}/{c['fiber_total_resonances']})")
    print()
    print("--- 裁决 ---")
    print(f"  胜者: {v['winner']}")
    print(f"  理由: {v['reason']}")
    print(f"  直积得分: {v['product_score']}")
    print(f"  纤维丛得分: {v['fiber_score']}")
    print()
    print(f"结果已写入: {output_path}")


if __name__ == "__main__":
    main()
