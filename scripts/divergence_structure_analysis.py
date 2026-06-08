"""22.2% 分歧事件结构特征分析。

245号谱系：验证 236号假设——分歧集中在 C-R 方向不一致的配置。

编排者问题：22.2% 分歧日的特征是什么？
236号假设：分歧集中在 risk-on/off 分歧点，即 C 和 R 方向相反的日子。
验证方法：取 120 个分歧事件，看它们是否系统性地对应 C-R 方向不一致。

定义：
  C-R 方向不一致 := sign(product_polarity) != sign(fiber_polarity)
  即两条管道给出了不同的策略方向（buy/sell/neutral）。

  decision divergence := product_resonates != fiber_resonates
  即两条管道给出了不同的入场/阻断决策。

输出：tmp/divergence-structure-analysis.json
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(project_root / "src"))

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.resonance import resonance_check
from newchan.pipeline import create_context, scan_configuration
from newchan.pipeline_backtest import (
    DualResonanceSignals,
    _build_resonance_signals,
)
from newchan.topology.config_space import (
    Configuration,
    WalkDirection,
    polarity_index,
)
from newchan.topology.fiber_pipeline_adapter import FiberSignalFilter


def _default_time_tolerance(level_i: int, level_j: int) -> float:
    """默认时间容差（与 pipeline 一致）。"""
    return float(abs(level_i - level_j) + 1) * 100.0


def _sign(x: int) -> int:
    """符号函数：正→+1，负→-1，零→0。"""
    if x > 0:
        return 1
    if x < 0:
        return -1
    return 0


# ── BSP 序列（与 dual_resonance_backtest 完全一致）──


def _generate_bsp_sequence(n: int = 20) -> list[BSP]:
    """生成交替的 buy/sell BSP 序列。与 dual_resonance_backtest 完全一致。"""
    bsps: list[BSP] = []
    for i in range(n):
        if i % 2 == 0:
            price = 95.0 + (i % 5) * 2.0
            bsps.append(BSP(
                edge_id=f"edge_{i}",
                level=1,
                time=float(i * 10),
                bsp_type=BSPType.B1,
                price=price,
            ))
        else:
            price = 115.0 + (i % 5) * 2.0
            bsps.append(BSP(
                edge_id=f"edge_{i}",
                level=1,
                time=float(i * 10),
                bsp_type=BSPType.S1,
                price=price,
            ))
    return bsps


# ── 单事件分析记录 ──


@dataclass(frozen=True, slots=True)
class DivergenceEvent:
    """单个决策分歧事件的完整结构特征。"""

    config_tuple: tuple[int, int, int]
    config_label: str
    bsp_idx: int
    bsp_type: str

    # 两条管道的决策
    product_resonates: bool
    fiber_resonates: bool

    # 两条管道的 polarity
    product_polarity: int
    fiber_polarity: int

    # 方向签名
    product_sign: int  # sign(product_polarity)
    fiber_sign: int    # sign(fiber_polarity)

    # C-R 方向一致性
    sign_divergent: bool  # product_sign != fiber_sign

    # 配置分量
    sigma_p: int
    sigma_c: int
    sigma_r: int

    # 纤维丛版 scan direction
    fiber_scan_direction: str


# ── 核心分析 ──


def run_divergence_analysis() -> dict:
    """执行分歧事件结构特征分析。

    Returns
    -------
    dict
        结构化分析结果，包含分歧/非分歧事件的 C-R 一致性统计。
    """
    bsps = _generate_bsp_sequence(20)
    fiber_filter = FiberSignalFilter(kl_threshold=0.0)

    divergence_events: list[DivergenceEvent] = []
    non_divergence_events: list[DivergenceEvent] = []

    # 遍历全部 27 配置 × 20 BSP
    for e in WalkDirection:
        for c in WalkDirection:
            for r in WalkDirection:
                config = Configuration(e, c, r)
                ctx = create_context(config, timestamp=0.0)
                product_pol = polarity_index(config)

                for idx, bsp in enumerate(bsps):
                    dual = _build_resonance_signals(bsp, ctx, fiber_filter)

                    # 共振检查
                    product_resonates = resonance_check(
                        list(dual.product_signals), _default_time_tolerance,
                    )
                    fiber_resonates = resonance_check(
                        list(dual.fiber_signals), _default_time_tolerance,
                    )

                    p_sign = _sign(dual.product_polarity)
                    f_sign = _sign(dual.fiber_polarity)
                    sign_div = p_sign != f_sign

                    event = DivergenceEvent(
                        config_tuple=config.as_tuple,
                        config_label=config.label,
                        bsp_idx=idx,
                        bsp_type=bsp.bsp_type.value,
                        product_resonates=product_resonates,
                        fiber_resonates=fiber_resonates,
                        product_polarity=dual.product_polarity,
                        fiber_polarity=dual.fiber_polarity,
                        product_sign=p_sign,
                        fiber_sign=f_sign,
                        sign_divergent=sign_div,
                        sigma_p=e.value,
                        sigma_c=c.value,
                        sigma_r=r.value,
                        fiber_scan_direction=dual.fiber_scan_direction,
                    )

                    if product_resonates != fiber_resonates:
                        divergence_events.append(event)
                    else:
                        non_divergence_events.append(event)

    # ── 统计 ──

    total_events = len(divergence_events) + len(non_divergence_events)
    n_div = len(divergence_events)
    n_nondiv = len(non_divergence_events)

    # 分歧事件中 C-R 方向不一致（sign_divergent=True）的占比
    div_sign_divergent = sum(1 for ev in divergence_events if ev.sign_divergent)
    # 非分歧事件中 C-R 方向不一致的占比
    nondiv_sign_divergent = sum(1 for ev in non_divergence_events if ev.sign_divergent)

    # 分歧事件的配置分布
    div_config_counts: dict[str, int] = {}
    for ev in divergence_events:
        div_config_counts[ev.config_label] = div_config_counts.get(ev.config_label, 0) + 1

    # 分歧事件按 sign 组合分类
    sign_combo_div: dict[str, int] = {}
    sign_combo_nondiv: dict[str, int] = {}
    for ev in divergence_events:
        key = f"P={ev.product_sign:+d},F={ev.fiber_sign:+d}"
        sign_combo_div[key] = sign_combo_div.get(key, 0) + 1
    for ev in non_divergence_events:
        key = f"P={ev.product_sign:+d},F={ev.fiber_sign:+d}"
        sign_combo_nondiv[key] = sign_combo_nondiv.get(key, 0) + 1

    # 分歧事件的详细列表（按配置分组）
    div_details_by_config: dict[str, list[dict]] = {}
    for ev in divergence_events:
        if ev.config_label not in div_details_by_config:
            div_details_by_config[ev.config_label] = []
        div_details_by_config[ev.config_label].append({
            "bsp_idx": ev.bsp_idx,
            "bsp_type": ev.bsp_type,
            "product_resonates": ev.product_resonates,
            "fiber_resonates": ev.fiber_resonates,
            "product_polarity": ev.product_polarity,
            "fiber_polarity": ev.fiber_polarity,
            "product_sign": ev.product_sign,
            "fiber_sign": ev.fiber_sign,
            "sign_divergent": ev.sign_divergent,
        })

    # 236号假设验证：分歧事件是否 100% 对应 sign_divergent
    hypothesis_236_verified = (
        n_div > 0
        and div_sign_divergent == n_div
    )

    # β_cr=0.688 含义验证：纤维丛修正只在 C-R 分歧时起作用
    # 即：非分歧事件中不应有 sign_divergent
    fiber_correction_only_on_cr_divergence = (
        nondiv_sign_divergent == 0
    )

    # 配置级别分析：哪些配置产生了分歧事件
    divergent_config_set = set(ev.config_label for ev in divergence_events)
    all_configs_with_sign_divergence = set()
    for ev in divergence_events:
        if ev.sign_divergent:
            all_configs_with_sign_divergence.add(ev.config_label)
    for ev in non_divergence_events:
        if ev.sign_divergent:
            all_configs_with_sign_divergence.add(ev.config_label)

    return {
        "summary": {
            "total_events": total_events,
            "divergence_events": n_div,
            "non_divergence_events": n_nondiv,
            "divergence_rate": n_div / total_events if total_events > 0 else 0.0,
        },
        "sign_divergence_in_divergence_events": {
            "count": div_sign_divergent,
            "total": n_div,
            "rate": div_sign_divergent / n_div if n_div > 0 else 0.0,
            "description": "分歧事件中 sign(product_polarity) != sign(fiber_polarity) 的占比",
        },
        "sign_divergence_in_non_divergence_events": {
            "count": nondiv_sign_divergent,
            "total": n_nondiv,
            "rate": nondiv_sign_divergent / n_nondiv if n_nondiv > 0 else 0.0,
            "description": "非分歧事件中 sign(product_polarity) != sign(fiber_polarity) 的占比",
        },
        "hypothesis_236": {
            "verified": hypothesis_236_verified,
            "description": (
                "236号假设：分歧事件 100% 对应 C-R 方向不一致 "
                "(sign(product_polarity) != sign(fiber_polarity))"
            ),
            "detail": (
                f"{div_sign_divergent}/{n_div} 分歧事件的 product/fiber polarity 符号不同"
            ),
        },
        "fiber_correction_scope": {
            "only_on_sign_divergence": fiber_correction_only_on_cr_divergence,
            "description": (
                "纤维丛修正是否只在 C-R 方向不一致时产生决策差异"
            ),
            "non_div_with_sign_divergence": nondiv_sign_divergent,
        },
        "sign_combo_distribution": {
            "divergence_events": sign_combo_div,
            "non_divergence_events": sign_combo_nondiv,
        },
        "config_distribution": {
            "divergent_configs": sorted(divergent_config_set),
            "configs_with_decision_diffs": div_config_counts,
            "configs_with_sign_divergence": sorted(all_configs_with_sign_divergence),
        },
        "divergence_event_details": div_details_by_config,
    }


def main() -> None:
    result = run_divergence_analysis()

    output_path = project_root / "tmp" / "divergence-structure-analysis.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(result, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )

    s = result["summary"]
    h = result["hypothesis_236"]
    sd = result["sign_divergence_in_divergence_events"]
    sn = result["sign_divergence_in_non_divergence_events"]
    fc = result["fiber_correction_scope"]

    print("=" * 60)
    print("分歧事件结构特征分析（245号）")
    print("=" * 60)
    print(f"总事件数: {s['total_events']}")
    print(f"分歧事件: {s['divergence_events']} ({s['divergence_rate']:.1%})")
    print(f"非分歧事件: {s['non_divergence_events']}")
    print()
    print("--- 236号假设验证 ---")
    print(f"  分歧事件中 sign 不一致: {sd['count']}/{sd['total']} ({sd['rate']:.1%})")
    print(f"  非分歧事件中 sign 不一致: {sn['count']}/{sn['total']} ({sn['rate']:.1%})")
    print(f"  236号假设验证: {'通过' if h['verified'] else '未通过'}")
    print(f"  {h['detail']}")
    print()
    print("--- 纤维丛修正作用域 ---")
    print(f"  修正只在 sign 不一致时起作用: {'是' if fc['only_on_sign_divergence'] else '否'}")
    print()
    print("--- 分歧事件配置分布 ---")
    for label, count in sorted(
        result["config_distribution"]["configs_with_decision_diffs"].items(),
        key=lambda x: -x[1],
    ):
        print(f"  {label}: {count} 个分歧事件")
    print()
    print("--- sign 组合分布（分歧事件）---")
    for combo, count in sorted(result["sign_combo_distribution"]["divergence_events"].items()):
        print(f"  {combo}: {count}")
    print()
    print(f"结果已写入: {output_path}")


if __name__ == "__main__":
    main()
