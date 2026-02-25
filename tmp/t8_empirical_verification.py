"""T8 背驰拓扑后验验证——经验验证脚本。

在多组合成数据上运行 T8 验证，统计：
- 背驰检出率（有多少数据集产生了背驰）
- inconclusive 比率（任一侧无中枢）
- passed 比率（W₁(C) ≤ W₁(A) - η）
- W₁ drop 分布
"""

import sys
import os
import json
from collections import defaultdict

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from newchan.a_topology import (
    _run_pipeline,
    check_divergence_topology,
)
from newchan.a_divergence import divergences_from_level


# ---------------------------------------------------------------------------
# Data generation
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
ETA_VALUES = [0.0, 0.01, 0.05]
MODES = ["wide", "strict", "new"]


# ---------------------------------------------------------------------------
# Run experiments
# ---------------------------------------------------------------------------

def run_all():
    results = []
    total = len(LENGTHS) * len(SEEDS) * len(MODES) * len(ETA_VALUES)
    done = 0

    for n in LENGTHS:
        for seed in SEEDS:
            df = _make_ohlc(n=n, seed=seed)
            for mode in MODES:
                try:
                    strokes, segments, centers, trends, rec_levels = _run_pipeline(df, mode)
                except Exception:
                    for eta in ETA_VALUES:
                        done += 1
                        results.append({
                            "n": n, "seed": seed, "mode": mode, "eta": eta,
                            "error": True, "n_divergences": 0,
                        })
                    continue

                # Get divergences
                if rec_levels:
                    level0 = rec_levels[0]
                    moves = level0.moves
                    used_centers = level0.centers
                    used_trends = level0.trends
                    level_id = level0.level
                else:
                    moves = segments
                    used_centers = centers
                    used_trends = trends
                    level_id = 0

                try:
                    divs = divergences_from_level(moves, used_centers, used_trends, level_id)
                except Exception:
                    divs = []

                for eta in ETA_VALUES:
                    done += 1
                    if not divs:
                        results.append({
                            "n": n, "seed": seed, "mode": mode, "eta": eta,
                            "error": False, "n_divergences": 0,
                            "n_passed": 0, "n_inconclusive": 0, "n_failed": 0,
                            "w1_drops": [],
                        })
                        continue

                    t8_checks = check_divergence_topology(
                        divs, moves, used_centers, eta=eta, normalize=True,
                    )

                    n_passed = sum(1 for r in t8_checks if r.passed)
                    n_inconclusive = sum(1 for r in t8_checks if r.inconclusive)
                    n_failed = sum(1 for r in t8_checks if not r.passed and not r.inconclusive)
                    w1_drops = [r.w1_drop for r in t8_checks if not r.inconclusive]

                    results.append({
                        "n": n, "seed": seed, "mode": mode, "eta": eta,
                        "error": False, "n_divergences": len(divs),
                        "n_passed": n_passed, "n_inconclusive": n_inconclusive,
                        "n_failed": n_failed,
                        "w1_drops": w1_drops,
                    })

                    if done % 20 == 0:
                        print(f"  [{done}/{total}] n={n} seed={seed} mode={mode} eta={eta} "
                              f"divs={len(divs)} passed={n_passed} inc={n_inconclusive}")

    return results


# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

def generate_report(results):
    lines = []
    lines.append("# T8 背驰拓扑后验验证——经验验证报告\n")

    # 总体统计
    total_runs = len(results)
    errors = sum(1 for r in results if r.get("error"))
    has_divs = sum(1 for r in results if r.get("n_divergences", 0) > 0)
    total_divs = sum(r.get("n_divergences", 0) for r in results)
    total_passed = sum(r.get("n_passed", 0) for r in results)
    total_inconclusive = sum(r.get("n_inconclusive", 0) for r in results)
    total_failed = sum(r.get("n_failed", 0) for r in results)

    lines.append("## 实验参数\n")
    lines.append(f"- **数据长度**: {LENGTHS}")
    lines.append(f"- **随机种子**: {SEEDS}")
    lines.append(f"- **eta 值**: {ETA_VALUES}")
    lines.append(f"- **模式**: {MODES}")
    lines.append(f"- **总运行数/错误数**: {total_runs}/{errors}")
    lines.append(f"- **产生背驰的运行数**: {has_divs}/{total_runs}")
    lines.append("")

    lines.append("## 1. 总体 T8 验证统计\n")
    lines.append("| 指标 | 数值 |")
    lines.append("|---|---|")
    lines.append(f"| 总背驰数 | {total_divs} |")
    lines.append(f"| T8 passed | {total_passed} |")
    lines.append(f"| T8 inconclusive | {total_inconclusive} |")
    lines.append(f"| T8 failed | {total_failed} |")
    if total_divs > 0:
        lines.append(f"| passed 率 | {total_passed/total_divs*100:.1f}% |")
        lines.append(f"| inconclusive 率 | {total_inconclusive/total_divs*100:.1f}% |")
        lines.append(f"| failed 率 | {total_failed/total_divs*100:.1f}% |")
    lines.append("")

    # 按 eta 分组
    lines.append("## 2. 按 eta 分组\n")
    lines.append("| eta | 背驰数 | passed | inconclusive | failed | passed率 |")
    lines.append("|---|---|---|---|---|---|")
    for eta in ETA_VALUES:
        subset = [r for r in results if r.get("eta") == eta]
        divs = sum(r.get("n_divergences", 0) for r in subset)
        passed = sum(r.get("n_passed", 0) for r in subset)
        inc = sum(r.get("n_inconclusive", 0) for r in subset)
        failed = sum(r.get("n_failed", 0) for r in subset)
        rate = f"{passed/divs*100:.1f}%" if divs > 0 else "N/A"
        lines.append(f"| {eta} | {divs} | {passed} | {inc} | {failed} | {rate} |")
    lines.append("")

    # 按模式分组
    lines.append("## 3. 按模式分组\n")
    lines.append("| 模式 | 背驰数 | passed | inconclusive | failed | passed率 |")
    lines.append("|---|---|---|---|---|---|")
    for mode in MODES:
        subset = [r for r in results if r.get("mode") == mode]
        divs = sum(r.get("n_divergences", 0) for r in subset)
        passed = sum(r.get("n_passed", 0) for r in subset)
        inc = sum(r.get("n_inconclusive", 0) for r in subset)
        failed = sum(r.get("n_failed", 0) for r in subset)
        rate = f"{passed/divs*100:.1f}%" if divs > 0 else "N/A"
        lines.append(f"| {mode} | {divs} | {passed} | {inc} | {failed} | {rate} |")
    lines.append("")

    # 按数据长度分组
    lines.append("## 4. 按数据长度分组\n")
    lines.append("| 长度 | 背驰数 | passed | inconclusive | failed | passed率 |")
    lines.append("|---|---|---|---|---|---|")
    for n in LENGTHS:
        subset = [r for r in results if r.get("n") == n]
        divs = sum(r.get("n_divergences", 0) for r in subset)
        passed = sum(r.get("n_passed", 0) for r in subset)
        inc = sum(r.get("n_inconclusive", 0) for r in subset)
        failed = sum(r.get("n_failed", 0) for r in subset)
        rate = f"{passed/divs*100:.1f}%" if divs > 0 else "N/A"
        lines.append(f"| {n} | {divs} | {passed} | {inc} | {failed} | {rate} |")
    lines.append("")

    # W₁ drop 分布
    all_drops = []
    for r in results:
        all_drops.extend(r.get("w1_drops", []))
    if all_drops:
        arr = np.array(all_drops)
        lines.append("## 5. W₁ drop 分布（非 inconclusive）\n")
        lines.append(f"- **样本数**: {len(arr)}")
        lines.append(f"- **mean**: {arr.mean():.4f}")
        lines.append(f"- **std**: {arr.std():.4f}")
        lines.append(f"- **min**: {arr.min():.4f}")
        lines.append(f"- **max**: {arr.max():.4f}")
        lines.append(f"- **median**: {np.median(arr):.4f}")
        lines.append(f"- **>0 比率**: {(arr > 0).sum()}/{len(arr)} ({(arr > 0).mean()*100:.1f}%)")
        lines.append(f"- **<0 比率**: {(arr < 0).sum()}/{len(arr)} ({(arr < 0).mean()*100:.1f}%)")
        lines.append("")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    print("Running T8 empirical verification...")
    results = run_all()
    report = generate_report(results)

    out_path = os.path.join(
        os.path.dirname(__file__), "..", ".chanlun", "review-results",
        "t8-empirical-verification-report.md",
    )
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report)

    print(f"\nReport written to: {out_path}")
    print(report)
