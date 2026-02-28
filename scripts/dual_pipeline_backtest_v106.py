"""双管道策略回测对比 — 253号谱系。

v106 方向：用 252号的 1min 递归产出的真实 BSP（7 个），通过 241号双管道
（直积版 vs 纤维丛版）分别做共振筛选，比较两套筛选后的信号。

两种模式：
  - 离线模式（默认）：直接用 252号已保存的 BSP 数据做共振筛选对比
  - 在线模式（--online）：重新拉 yfinance 1min 数据做完整 bar-by-bar 回测

概念溯源：
  - 253号：双管道真实 BSP 回测
  - 252号：递归 1min 深度验证
  - 244号：合成数据双管道对比（直积 vs 纤维丛平局 0.8765 vs 0.8765，22.2% 分歧）
  - 241号：_build_resonance_signals 双模型并行共振构建
  - 243号：fiber_scan_direction 纤维丛版 scan direction
"""

from __future__ import annotations

import json
import logging
import sys
from pathlib import Path

# 确保项目在 path 上
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.resonance import (
    classify_resonance,
    resonance_check,
    resonance_strength,
)
from newchan.pipeline import (
    TradingContext,
    compute_position,
    create_context,
    locate_bsp,
    scan_configuration,
)
from newchan.pipeline_backtest import (
    DualResonanceSignals,
    _build_resonance_signals,
)
from newchan.topology.config_space import (
    Configuration,
    ConfigurationSpace,
    polarity_index,
)
from newchan.topology.fiber_pipeline_adapter import FiberSignalFilter

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("dual_pipeline_backtest_v106")


# ── 252号 BSP 数据（从 tmp/recursive-1min-v105.json 提取） ───────

_BSP_RAW = [
    {"kind": "type3", "side": "sell", "seg_idx": 6, "price": 690.85, "confirmed": True},
    {"kind": "type3", "side": "sell", "seg_idx": 8, "price": 682.61, "confirmed": True},
    {"kind": "type3", "side": "buy", "seg_idx": 13, "price": 693.89, "confirmed": True},
    {"kind": "type3", "side": "sell", "seg_idx": 26, "price": 686.28, "confirmed": True},
    {"kind": "type3", "side": "sell", "seg_idx": 38, "price": 684.06, "confirmed": True},
    {"kind": "type3", "side": "buy", "seg_idx": 47, "price": 690.17, "confirmed": False},
    {"kind": "type3", "side": "sell", "seg_idx": 52, "price": 689.81, "confirmed": False},
]

_KIND_BUY = {"type1": BSPType.B1, "type2": BSPType.B2, "type3": BSPType.B3}
_KIND_SELL = {"type1": BSPType.S1, "type2": BSPType.S2, "type3": BSPType.S3}


def _load_252_bsps() -> list[BSP]:
    """加载 252号的 7 个 BSP。"""
    bsps: list[BSP] = []
    for r in _BSP_RAW:
        mapping = _KIND_BUY if r["side"] == "buy" else _KIND_SELL
        bsps.append(BSP(
            edge_id=f"bt_{r['seg_idx']}",
            level=1,
            time=float(r["seg_idx"]),
            bsp_type=mapping[r["kind"]],
            price=r["price"],
        ))
    return bsps


# ── 时间容差 ──────────────────────────────────────────────────


def _default_time_tolerance(level_i: int, level_j: int) -> float:
    return float(abs(level_i - level_j) + 1) * 100.0


# ── 核心对比逻辑 ─────────────────────────────────────────────


def _compare_single(
    bsp: BSP,
    config: Configuration,
    fiber_filter: FiberSignalFilter,
) -> dict:
    """对单个 BSP × Configuration 做双管道共振对比。"""
    ctx = create_context(config)
    ctx = locate_bsp(ctx, bsp)

    dual = _build_resonance_signals(bsp, ctx, fiber_filter)

    # 直积版
    product_sigs = list(dual.product_signals)
    product_resonant = resonance_check(product_sigs, _default_time_tolerance)
    product_str = resonance_strength(product_sigs)
    product_pos, _ = compute_position(ctx, product_sigs, _default_time_tolerance)

    # 纤维丛版
    fiber_sigs = list(dual.fiber_signals)
    fiber_resonant = resonance_check(fiber_sigs, _default_time_tolerance)
    fiber_str = resonance_strength(fiber_sigs)
    fiber_pos, _ = compute_position(ctx, fiber_sigs, _default_time_tolerance)

    # 方向过滤
    _, dirs = scan_configuration(ctx)
    product_scan_dir = dirs[0] if dirs else "neutral"
    fiber_scan_dir = dual.fiber_scan_direction

    bsp_is_buy = bsp.bsp_type.is_buy
    product_dir_ok = (bsp_is_buy and product_scan_dir == "buy") or (
        not bsp_is_buy and product_scan_dir == "sell"
    )
    fiber_dir_ok = (bsp_is_buy and fiber_scan_dir == "buy") or (
        not bsp_is_buy and fiber_scan_dir == "sell"
    )

    product_enter = product_pos > 0 and product_dir_ok
    fiber_enter = fiber_pos > 0 and fiber_dir_ok

    return {
        "bsp_edge_id": bsp.edge_id,
        "bsp_type": bsp.bsp_type.value,
        "bsp_price": bsp.price,
        "bsp_direction": "buy" if bsp_is_buy else "sell",
        "config_label": config.label,
        "config_tuple": config.as_tuple,
        "polarity": polarity_index(config),
        "product_polarity": dual.product_polarity,
        "fiber_polarity": dual.fiber_polarity,
        "polarity_divergence": dual.polarity_divergence,
        "product_scan_dir": product_scan_dir,
        "fiber_scan_dir": fiber_scan_dir,
        "product_resonant": product_resonant,
        "fiber_resonant": fiber_resonant,
        "product_strength": product_str,
        "fiber_strength": fiber_str,
        "product_position": product_pos,
        "fiber_position": fiber_pos,
        "product_enter": product_enter,
        "fiber_enter": fiber_enter,
        "entry_divergence": product_enter != fiber_enter,
    }


# ── 主逻辑 ──────────────────────────────────────────────────


def run() -> dict:
    """执行离线双管道共振对比。"""
    bsps = _load_252_bsps()
    space = ConfigurationSpace()
    configs = list(space)
    fiber_filter = FiberSignalFilter()

    logger.info("252号 BSP 数: %d, K4 配置数: %d", len(bsps), len(configs))

    all_entries: list[dict] = []
    for cfg in configs:
        for bsp in bsps:
            entry = _compare_single(bsp, cfg, fiber_filter)
            all_entries.append(entry)

    # 汇总
    total = len(all_entries)
    entry_divergences = [e for e in all_entries if e["entry_divergence"]]
    polarity_divergences = [e for e in all_entries if e["polarity_divergence"]]
    product_enters = sum(1 for e in all_entries if e["product_enter"])
    fiber_enters = sum(1 for e in all_entries if e["fiber_enter"])

    # 按 BSP 汇总
    bsp_summary: dict[str, dict] = {}
    for e in all_entries:
        key = e["bsp_edge_id"]
        if key not in bsp_summary:
            bsp_summary[key] = {
                "bsp": key,
                "type": e["bsp_type"],
                "direction": e["bsp_direction"],
                "price": e["bsp_price"],
                "configs_tested": 0,
                "product_entries": 0,
                "fiber_entries": 0,
                "entry_divergences": 0,
            }
        bsp_summary[key]["configs_tested"] += 1
        if e["product_enter"]:
            bsp_summary[key]["product_entries"] += 1
        if e["fiber_enter"]:
            bsp_summary[key]["fiber_entries"] += 1
        if e["entry_divergence"]:
            bsp_summary[key]["entry_divergences"] += 1

    # 按 Configuration 汇总
    config_summary: dict[str, dict] = {}
    for e in all_entries:
        key = e["config_label"]
        if key not in config_summary:
            config_summary[key] = {
                "config": key,
                "tuple": e["config_tuple"],
                "polarity": e["polarity"],
                "bsps_tested": 0,
                "product_entries": 0,
                "fiber_entries": 0,
                "entry_divergences": 0,
                "polarity_divergences": 0,
            }
        config_summary[key]["bsps_tested"] += 1
        if e["product_enter"]:
            config_summary[key]["product_entries"] += 1
        if e["fiber_enter"]:
            config_summary[key]["fiber_entries"] += 1
        if e["entry_divergence"]:
            config_summary[key]["entry_divergences"] += 1
        if e["polarity_divergence"]:
            config_summary[key]["polarity_divergences"] += 1

    # 按极性分组汇总
    polarity_summary: dict[int, dict] = {}
    for e in all_entries:
        pol = e["polarity"]
        if pol not in polarity_summary:
            polarity_summary[pol] = {
                "polarity": pol,
                "total": 0,
                "product_entries": 0,
                "fiber_entries": 0,
                "entry_divergences": 0,
                "polarity_divergences": 0,
            }
        polarity_summary[pol]["total"] += 1
        if e["product_enter"]:
            polarity_summary[pol]["product_entries"] += 1
        if e["fiber_enter"]:
            polarity_summary[pol]["fiber_entries"] += 1
        if e["entry_divergence"]:
            polarity_summary[pol]["entry_divergences"] += 1
        if e["polarity_divergence"]:
            polarity_summary[pol]["polarity_divergences"] += 1

    div_rate = len(entry_divergences) / total if total > 0 else 0.0

    return {
        "metadata": {
            "source_genealogy": [252, 241, 233],
            "bsp_count": len(bsps),
            "config_count": len(configs),
            "total_comparisons": total,
            "mode": "offline",
        },
        "summary": {
            "total_comparisons": total,
            "product_total_entries": product_enters,
            "fiber_total_entries": fiber_enters,
            "entry_divergences": len(entry_divergences),
            "entry_divergence_rate": div_rate,
            "polarity_divergence_count": len(polarity_divergences),
            "v244_comparison": {
                "v244_synthetic_divergence_rate": 0.222,
                "v106_real_divergence_rate": div_rate,
                "note": "v244 用合成数据，v106 用真实 252号 1min 递归 BSP",
            },
        },
        "polarity_summary": sorted(polarity_summary.values(), key=lambda x: x["polarity"]),
        "bsp_summary": list(bsp_summary.values()),
        "config_summary": sorted(config_summary.values(), key=lambda x: x["polarity"]),
        "entry_divergence_details": entry_divergences,
        "all_entries": all_entries,
    }


def main() -> None:
    result = run()
    out_path = PROJECT_ROOT / "tmp" / "dual-pipeline-backtest-v106.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    # 打印摘要
    s = result["summary"]
    print(f"\n=== 双管道回测对比结果 ===")
    print(f"总比较数: {s['total_comparisons']}")
    print(f"直积版入场: {s['product_total_entries']}")
    print(f"纤维丛版入场: {s['fiber_total_entries']}")
    print(f"入场分歧数: {s['entry_divergences']} ({s['entry_divergence_rate']:.1%})")
    print(f"极性分歧数: {s['polarity_divergence_count']}")
    print(f"\nv244 合成数据分歧率: {s['v244_comparison']['v244_synthetic_divergence_rate']:.1%}")
    print(f"v106 真实数据分歧率: {s['v244_comparison']['v106_real_divergence_rate']:.1%}")

    print(f"\n--- 按极性分组 ---")
    for p in result["polarity_summary"]:
        print(f"  S={p['polarity']:+d}: total={p['total']}, "
              f"product={p['product_entries']}, fiber={p['fiber_entries']}, "
              f"div={p['entry_divergences']}, pol_div={p['polarity_divergences']}")

    print(f"\n--- 按 BSP ---")
    for b in result["bsp_summary"]:
        print(f"  {b['bsp']} ({b['type']}, {b['direction']}, ${b['price']:.2f}): "
              f"product={b['product_entries']}/{b['configs_tested']}, "
              f"fiber={b['fiber_entries']}/{b['configs_tested']}, "
              f"div={b['entry_divergences']}")

    if result["entry_divergence_details"]:
        print(f"\n--- 分歧点详情（前10） ---")
        for d in result["entry_divergence_details"][:10]:
            print(f"  {d['config_label']} × {d['bsp_edge_id']} ({d['bsp_type']})")
            print(f"    product: enter={d['product_enter']}, scan={d['product_scan_dir']}, "
                  f"pol={d['product_polarity']}, pos={d['product_position']:.4f}")
            print(f"    fiber:   enter={d['fiber_enter']}, scan={d['fiber_scan_dir']}, "
                  f"pol={d['fiber_polarity']}, pos={d['fiber_position']:.4f}")

    print(f"\n结果已写入: {out_path}")


if __name__ == "__main__":
    main()
