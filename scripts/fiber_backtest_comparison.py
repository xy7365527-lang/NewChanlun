"""纤维丛回测对比脚本 — 直积 polarity vs 纤维丛修正 polarity。

240号谱系：量化纤维丛修正对策略信号的实际影响。

方法：
  1. 遍历全部 27 种配置
  2. 对每种配置计算直积 polarity 和纤维丛 polarity
  3. 统计 polarity 分歧率、方向翻转明细
  4. 模拟策略信号差异（buy/sell/neutral 方向变化）

输出：tmp/fiber-backtest-comparison.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# 确保项目根目录在 sys.path 中
project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(project_root / "src"))

from newchan.pipeline import create_context, scan_configuration
from newchan.topology.config_space import Configuration, WalkDirection, polarity_index
from newchan.topology.fiber_bundle import Connection, FiberBundleConfigSpace, default_fiber_bundle
from newchan.topology.fiber_pipeline_adapter import (
    FiberSignalFilter,
    compute_fiber_correction,
    create_fiber_context,
)


def _polarity_to_direction(s: int) -> str:
    """polarity_index -> 策略方向。"""
    if s > 0:
        return "buy"
    if s < 0:
        return "sell"
    return "neutral"


def run_comparison() -> dict:
    """运行全配置比较，返回结果字典。"""
    fb = default_fiber_bundle()
    signal_filter = FiberSignalFilter()

    configs_detail: list[dict] = []
    divergence_count = 0
    direction_flip_count = 0
    override_count = 0
    total = 0

    for e in WalkDirection:
        for c in WalkDirection:
            for r in WalkDirection:
                total += 1
                config = Configuration(e, c, r)
                ctx = create_context(config, timestamp=float(total))

                # 直积 pipeline
                product_ctx, product_directions = scan_configuration(ctx)
                product_polarity = product_ctx.polarity

                # 纤维丛修正
                fiber_ctx = create_fiber_context(ctx, fb)
                fiber_polarity = fiber_ctx.fiber_polarity

                # 方向比较
                product_dir = _polarity_to_direction(product_polarity)
                fiber_dir = _polarity_to_direction(fiber_polarity)

                divergence = fiber_ctx.polarity_divergence
                direction_flipped = product_dir != fiber_dir
                should_override = signal_filter.should_override_polarity(fiber_ctx)

                if divergence:
                    divergence_count += 1
                if direction_flipped:
                    direction_flip_count += 1
                if should_override:
                    override_count += 1

                report = signal_filter.correction_report(fiber_ctx)

                configs_detail.append({
                    "config": config.as_tuple,
                    "product_polarity": product_polarity,
                    "fiber_polarity": fiber_polarity,
                    "product_direction": product_dir,
                    "fiber_direction": fiber_dir,
                    "polarity_divergence": divergence,
                    "direction_flipped": direction_flipped,
                    "should_override": should_override,
                    "kl_divergence": report["kl_divergence"],
                    "correction_magnitude": report["correction_magnitude"],
                })

    # 汇总统计
    summary = {
        "total_configs": total,
        "polarity_divergence_count": divergence_count,
        "polarity_divergence_rate": divergence_count / total,
        "direction_flip_count": direction_flip_count,
        "direction_flip_rate": direction_flip_count / total,
        "override_count": override_count,
        "override_rate": override_count / total,
        "effective_dimension": fb.effective_dimension(),
        "global_kl_divergence": fb.kl_divergence_from_product(),
    }

    # 分歧配置明细
    divergence_details = [
        d for d in configs_detail if d["polarity_divergence"]
    ]

    return {
        "summary": summary,
        "divergence_details": divergence_details,
        "all_configs": configs_detail,
    }


def main() -> None:
    result = run_comparison()

    output_path = project_root / "tmp" / "fiber-backtest-comparison.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(result, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )

    s = result["summary"]
    print(f"总配置数: {s['total_configs']}")
    print(f"polarity 分歧: {s['polarity_divergence_count']} ({s['polarity_divergence_rate']:.1%})")
    print(f"方向翻转: {s['direction_flip_count']} ({s['direction_flip_rate']:.1%})")
    print(f"建议覆盖: {s['override_count']} ({s['override_rate']:.1%})")
    print(f"有效自由度 D_eff: {s['effective_dimension']:.4f}")
    print(f"全局 KL 散度: {s['global_kl_divergence']:.6f}")
    print(f"\n结果已写入: {output_path}")

    if result["divergence_details"]:
        print(f"\n分歧配置明细 ({len(result['divergence_details'])} 个):")
        for d in result["divergence_details"]:
            print(
                f"  {d['config']}  "
                f"直积={d['product_direction']}({d['product_polarity']:+d})  "
                f"纤维丛={d['fiber_direction']}({d['fiber_polarity']:+d})  "
                f"KL={d['kl_divergence']:.4f}"
            )


if __name__ == "__main__":
    main()
