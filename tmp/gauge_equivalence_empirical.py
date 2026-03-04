"""gauge_equivalence_report 经验验证脚本。

在多组合成数据上运行 gauge_equivalence_report，
统计强不变量候选在不同参数下的保持率。
"""

import sys
import os
import json
from collections import defaultdict
from itertools import product

import numpy as np
import pandas as pd

# Ensure the project root is on sys.path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from newchan.a_topology import gauge_equivalence_report


# ---------------------------------------------------------------------------
# Data generation (same as test helper)
# ---------------------------------------------------------------------------

def _make_ohlc(n=200, seed=42):
    rng = np.random.RandomState(seed)
    close = 100.0 + np.cumsum(rng.randn(n) * 0.5)
    high = close + rng.uniform(0.1, 1.0, n)
    low = close - rng.uniform(0.1, 1.0, n)
    opn = close + rng.uniform(-0.3, 0.3, n)
    dates = pd.date_range("2024-01-01", periods=n, freq="h")
    return pd.DataFrame(
        {"open": opn, "high": high, "low": low, "close": close},
        index=dates,
    )


# ---------------------------------------------------------------------------
# Parameters
# ---------------------------------------------------------------------------

LENGTHS = [100, 200, 300, 500]
SEEDS = [42, 99, 123, 456, 789]
TAU_VALUES = [0.0, 1.0, 3.0]
TOLERANCE_VALUES = [0.0, 1.0]
MODES = ("wide", "strict", "new")
STRONG_KEYS = [
    "n_centers", "center_zd_zg_pairs", "trend_kinds",
    "beta1_tau", "barcode_bottleneck",
]


# ---------------------------------------------------------------------------
# Run experiments
# ---------------------------------------------------------------------------

def run_all():
    """Run gauge_equivalence_report for all parameter combinations."""
    results = []
    total = len(LENGTHS) * len(SEEDS) * len(TAU_VALUES) * len(TOLERANCE_VALUES)
    count = 0

    for n, seed, tau, tol in product(LENGTHS, SEEDS, TAU_VALUES, TOLERANCE_VALUES):
        count += 1
        print(f"  [{count}/{total}] n={n}, seed={seed}, tau={tau}, tol={tol} ... ", end="", flush=True)
        try:
            df = _make_ohlc(n=n, seed=seed)
            report = gauge_equivalence_report(
                df, modes=MODES, tolerance=tol, tau=tau,
            )
            results.append({
                "n": n,
                "seed": seed,
                "tau": tau,
                "tolerance": tol,
                "report": report,
            })
            # Quick status
            summary = report["strong_invariant_summary"]
            rates = {k: f"{summary[k]['rate']:.0%}" for k in STRONG_KEYS}
            print(f"OK  rates={rates}")
        except Exception as e:
            print(f"ERROR: {e}")
            results.append({
                "n": n,
                "seed": seed,
                "tau": tau,
                "tolerance": tol,
                "report": None,
                "error": str(e),
            })

    return results


# ---------------------------------------------------------------------------
# Analysis
# ---------------------------------------------------------------------------

def analyse(results):
    """Aggregate results into tables."""

    # --- 1. Overall preservation rate per strong invariant ---
    overall = {k: {"preserved": 0, "total": 0} for k in STRONG_KEYS}
    for r in results:
        if r["report"] is None:
            continue
        summary = r["report"]["strong_invariant_summary"]
        for k in STRONG_KEYS:
            overall[k]["preserved"] += summary[k]["preserved"]
            overall[k]["total"] += summary[k]["total"]

    # --- 2. By tau ---
    by_tau = {tau: {k: {"preserved": 0, "total": 0} for k in STRONG_KEYS} for tau in TAU_VALUES}
    for r in results:
        if r["report"] is None:
            continue
        tau = r["tau"]
        summary = r["report"]["strong_invariant_summary"]
        for k in STRONG_KEYS:
            by_tau[tau][k]["preserved"] += summary[k]["preserved"]
            by_tau[tau][k]["total"] += summary[k]["total"]

    # --- 3. By tolerance ---
    by_tol = {tol: {k: {"preserved": 0, "total": 0} for k in STRONG_KEYS} for tol in TOLERANCE_VALUES}
    for r in results:
        if r["report"] is None:
            continue
        tol = r["tolerance"]
        summary = r["report"]["strong_invariant_summary"]
        for k in STRONG_KEYS:
            by_tol[tol][k]["preserved"] += summary[k]["preserved"]
            by_tol[tol][k]["total"] += summary[k]["total"]

    # --- 4. By n (data length) ---
    by_n = {n: {k: {"preserved": 0, "total": 0} for k in STRONG_KEYS} for n in LENGTHS}
    for r in results:
        if r["report"] is None:
            continue
        n = r["n"]
        summary = r["report"]["strong_invariant_summary"]
        for k in STRONG_KEYS:
            by_n[n][k]["preserved"] += summary[k]["preserved"]
            by_n[n][k]["total"] += summary[k]["total"]

    # --- 5. Per-pair delta statistics ---
    pair_deltas = defaultdict(lambda: {
        "bottleneck_distances": [],
        "center_count_diffs": [],
        "stroke_count_diffs": [],
        "segment_count_diffs": [],
        "beta1_tau_diffs": [],
    })
    for r in results:
        if r["report"] is None:
            continue
        for t in r["report"]["transitions"]:
            pair = f"{t['source']} -> {t['target']}"
            pair_deltas[pair]["bottleneck_distances"].append(t["delta"]["bottleneck_distance"])
            pair_deltas[pair]["center_count_diffs"].append(t["delta"]["center_count_diff"])
            pair_deltas[pair]["stroke_count_diffs"].append(t["delta"]["stroke_count_diff"])
            pair_deltas[pair]["segment_count_diffs"].append(t["delta"]["segment_count_diff"])
            pair_deltas[pair]["beta1_tau_diffs"].append(t["delta"]["beta1_tau_diff"])

    # --- 6. Zero-center stats (how many runs produce 0 centers) ---
    zero_center_runs = 0
    total_runs = 0
    for r in results:
        if r["report"] is None:
            continue
        total_runs += 1
        for t in r["report"]["transitions"]:
            # Check source or target with 0 centers
            if t["delta"]["center_count_diff"] == 0:
                # Need to check actual fingerprints via n_centers
                pass  # We don't have raw fingerprints in report; use transition info

    return {
        "overall": overall,
        "by_tau": by_tau,
        "by_tol": by_tol,
        "by_n": by_n,
        "pair_deltas": pair_deltas,
        "n_successful": sum(1 for r in results if r["report"] is not None),
        "n_failed": sum(1 for r in results if r["report"] is None),
        "total": len(results),
    }


def _rate_str(preserved, total):
    if total == 0:
        return "N/A"
    return f"{preserved}/{total} ({preserved/total:.1%})"


def format_report(analysis):
    """Format analysis into a markdown report."""
    lines = []
    lines.append("# gauge_equivalence_report 经验验证报告")
    lines.append("")
    lines.append("## 实验参数")
    lines.append("")
    lines.append(f"- **数据长度**: {LENGTHS}")
    lines.append(f"- **随机种子**: {SEEDS}")
    lines.append(f"- **tau 值**: {TAU_VALUES}")
    lines.append(f"- **tolerance 值**: {TOLERANCE_VALUES}")
    lines.append(f"- **模式**: {list(MODES)}")
    lines.append(f"- **成功/失败/总计**: {analysis['n_successful']}/{analysis['n_failed']}/{analysis['total']}")
    lines.append("")

    # --- Overall ---
    lines.append("## 1. 总体强不变量保持率")
    lines.append("")
    lines.append("| 强不变量候选 | 保持次数/总比较次数 | 保持率 |")
    lines.append("|---|---|---|")
    for k in STRONG_KEYS:
        d = analysis["overall"][k]
        lines.append(f"| {k} | {_rate_str(d['preserved'], d['total'])} | {d['preserved']/d['total']:.1%} |" if d["total"] > 0 else f"| {k} | N/A | N/A |")
    lines.append("")

    # --- By tau ---
    lines.append("## 2. 按 tau 分组的保持率")
    lines.append("")
    header = "| 强不变量候选 |"
    sep = "|---|"
    for tau in TAU_VALUES:
        header += f" tau={tau} |"
        sep += "---|"
    lines.append(header)
    lines.append(sep)
    for k in STRONG_KEYS:
        row = f"| {k} |"
        for tau in TAU_VALUES:
            d = analysis["by_tau"][tau][k]
            if d["total"] > 0:
                row += f" {_rate_str(d['preserved'], d['total'])} |"
            else:
                row += " N/A |"
        lines.append(row)
    lines.append("")

    # --- By tolerance ---
    lines.append("## 3. 按 tolerance 分组的保持率")
    lines.append("")
    header = "| 强不变量候选 |"
    sep = "|---|"
    for tol in TOLERANCE_VALUES:
        header += f" tol={tol} |"
        sep += "---|"
    lines.append(header)
    lines.append(sep)
    for k in STRONG_KEYS:
        row = f"| {k} |"
        for tol in TOLERANCE_VALUES:
            d = analysis["by_tol"][tol][k]
            if d["total"] > 0:
                row += f" {_rate_str(d['preserved'], d['total'])} |"
            else:
                row += " N/A |"
        lines.append(row)
    lines.append("")

    # --- By data length ---
    lines.append("## 4. 按数据长度分组的保持率")
    lines.append("")
    header = "| 强不变量候选 |"
    sep = "|---|"
    for n in LENGTHS:
        header += f" n={n} |"
        sep += "---|"
    lines.append(header)
    lines.append(sep)
    for k in STRONG_KEYS:
        row = f"| {k} |"
        for n in LENGTHS:
            d = analysis["by_n"][n][k]
            if d["total"] > 0:
                row += f" {d['preserved']/d['total']:.1%} |"
            else:
                row += " N/A |"
        lines.append(row)
    lines.append("")

    # --- Per-pair delta statistics ---
    lines.append("## 5. 模式对之间的结构差异统计")
    lines.append("")
    for pair, deltas in sorted(analysis["pair_deltas"].items()):
        lines.append(f"### {pair}")
        lines.append("")
        lines.append(f"- 样本数: {len(deltas['bottleneck_distances'])}")

        for metric_name, metric_key in [
            ("bottleneck_distance", "bottleneck_distances"),
            ("center_count_diff", "center_count_diffs"),
            ("stroke_count_diff", "stroke_count_diffs"),
            ("segment_count_diff", "segment_count_diffs"),
            ("beta1_tau_diff", "beta1_tau_diffs"),
        ]:
            vals = deltas[metric_key]
            if vals:
                arr = np.array(vals)
                lines.append(
                    f"- **{metric_name}**: "
                    f"mean={arr.mean():.3f}, "
                    f"std={arr.std():.3f}, "
                    f"min={arr.min():.3f}, "
                    f"max={arr.max():.3f}, "
                    f"median={np.median(arr):.3f}"
                )
        lines.append("")

    # --- Classification ---
    lines.append("## 6. 经验分类")
    lines.append("")
    lines.append("基于保持率的经验分类（阈值：保持率 >= 90% 为强候选，50%-90% 为中等，< 50% 为弱候选）：")
    lines.append("")
    lines.append("| 不变量候选 | 总体保持率 | 经验分类 |")
    lines.append("|---|---|---|")
    for k in STRONG_KEYS:
        d = analysis["overall"][k]
        if d["total"] > 0:
            rate = d["preserved"] / d["total"]
            if rate >= 0.9:
                classification = "**强不变量**"
            elif rate >= 0.5:
                classification = "中等候选"
            else:
                classification = "弱候选（不保持）"
            lines.append(f"| {k} | {rate:.1%} | {classification} |")
        else:
            lines.append(f"| {k} | N/A | 数据不足 |")
    lines.append("")

    # --- tau sensitivity ---
    lines.append("## 7. tau 敏感性分析")
    lines.append("")
    lines.append("beta1_tau 和 barcode_bottleneck 在不同 tau 下的保持率变化：")
    lines.append("")
    lines.append("| tau | beta1_tau 保持率 | barcode_bottleneck 保持率 |")
    lines.append("|---|---|---|")
    for tau in TAU_VALUES:
        b1 = analysis["by_tau"][tau]["beta1_tau"]
        bb = analysis["by_tau"][tau]["barcode_bottleneck"]
        b1_rate = f"{b1['preserved']/b1['total']:.1%}" if b1["total"] > 0 else "N/A"
        bb_rate = f"{bb['preserved']/bb['total']:.1%}" if bb["total"] > 0 else "N/A"
        lines.append(f"| {tau} | {b1_rate} | {bb_rate} |")
    lines.append("")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    print("=" * 60)
    print("gauge_equivalence_report 经验验证")
    print("=" * 60)
    print()

    results = run_all()

    print()
    print("Analysing results ...")
    analysis = analyse(results)

    report_md = format_report(analysis)

    output_path = os.path.join(
        os.path.dirname(__file__), "..",
        ".chanlun", "review-results", "gauge-equivalence-empirical-report.md",
    )
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(report_md)

    print(f"Report written to: {output_path}")
    print()
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print()
    for k in STRONG_KEYS:
        d = analysis["overall"][k]
        if d["total"] > 0:
            print(f"  {k:30s} {d['preserved']:3d}/{d['total']:3d} = {d['preserved']/d['total']:.1%}")
        else:
            print(f"  {k:30s} N/A")
    print()
    print(f"  Successful runs: {analysis['n_successful']}/{analysis['total']}")
    if analysis['n_failed'] > 0:
        print(f"  Failed runs: {analysis['n_failed']}")
