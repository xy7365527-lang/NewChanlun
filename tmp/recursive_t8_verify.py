"""递归级别 >= 1 的 T8 背驰拓扑化验证。

验证目标：当递归引擎产生 level >= 2 时，T8（背驰 = W1 单调下降）
在高级别递归层是否仍然成立。

策略：
1. 用 akshare 获取长周期日线数据（5-10年），尽量产生多层递归
2. 用合成数据构造确定性多层递归场景
3. 对每个递归层运行 divergences_from_level + check_divergence_topology
4. 汇总结果
"""

import sys
import os
import time
import traceback

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

import numpy as np
import pandas as pd

from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_recursive_engine import build_recursive_levels
from newchan.a_divergence import divergences_from_level
from newchan.a_topology import check_divergence_topology, centers_to_barcode, _w1_norm


# =========================================================================
# 数据获取
# =========================================================================

def fetch_akshare_daily(symbol: str, name: str, start: str, end: str) -> pd.DataFrame | None:
    """用 akshare 获取 A 股日线数据。"""
    try:
        import akshare as ak
        time.sleep(2)
        df = ak.stock_zh_a_daily(
            symbol=symbol, start_date=start, end_date=end, adjust="qfq",
        )
        if df is None or len(df) == 0:
            print(f"  [WARN] {symbol} {name}: no data")
            return None
        result = pd.DataFrame({
            "open": df["open"].values.astype(float),
            "high": df["high"].values.astype(float),
            "low": df["low"].values.astype(float),
            "close": df["close"].values.astype(float),
        })
        result.index = pd.to_datetime(df["date"].values)
        print(f"  [OK] {symbol} {name}: {len(result)} bars")
        return result
    except Exception as e:
        print(f"  [ERR] {symbol} {name}: {e}")
        return None


def make_synthetic_multilevel(n=2000, seed=42, amplitude=10.0, period=50):
    """构造确定性多层递归合成数据。

    策略：叠加多个周期的正弦波 + 趋势，确保产生足够多的
    线段 -> 中枢 -> 走势类型 -> 高级别中枢。

    关键：需要足够长的数据 + 明确的多尺度振荡结构。
    """
    rng = np.random.RandomState(seed)
    t = np.arange(n)

    # 多尺度叠加：短周期（笔）+ 中周期（线段/中枢）+ 长周期（走势类型）
    short = amplitude * 0.3 * np.sin(2 * np.pi * t / period)
    medium = amplitude * 0.6 * np.sin(2 * np.pi * t / (period * 5))
    long_wave = amplitude * 1.0 * np.sin(2 * np.pi * t / (period * 25))
    trend = t * 0.002  # 微弱上升趋势
    noise = rng.randn(n) * amplitude * 0.05

    close = 100.0 + short + medium + long_wave + trend + noise
    high = close + rng.uniform(0.1, amplitude * 0.1, n)
    low = close - rng.uniform(0.1, amplitude * 0.1, n)
    opn = close + rng.uniform(-amplitude * 0.05, amplitude * 0.05, n)

    dates = pd.date_range("2016-01-01", periods=n, freq="D")
    return pd.DataFrame(
        {"open": opn, "high": high, "low": low, "close": close},
        index=dates,
    )


def make_synthetic_strong_oscillation(n=3000, seed=99):
    """更强振荡的合成数据——确保产生 level >= 2。

    用分段线性构造明确的 ABCABC... 结构。
    """
    rng = np.random.RandomState(seed)
    prices = [100.0]

    # 构造多组"上-下-上-下"大波段，每个波段内有小振荡
    for cycle in range(6):  # 6 个大周期
        base = prices[-1]
        direction = 1 if cycle % 2 == 0 else -1

        for sub in range(5):  # 每个大周期 5 个子波段
            sub_dir = direction if sub % 2 == 0 else -direction
            sub_len = rng.randint(30, 80)
            sub_amp = rng.uniform(3, 8)

            for _ in range(sub_len):
                step = sub_dir * rng.uniform(0.01, 0.15) + rng.randn() * 0.1
                prices.append(prices[-1] + step)

            # 子波段结束后反转
            for _ in range(rng.randint(10, 30)):
                step = -sub_dir * rng.uniform(0.05, 0.2) + rng.randn() * 0.1
                prices.append(prices[-1] + step)

    prices = np.array(prices[:n])
    high = prices + rng.uniform(0.1, 0.5, len(prices))
    low = prices - rng.uniform(0.1, 0.5, len(prices))
    opn = prices + rng.uniform(-0.2, 0.2, len(prices))

    dates = pd.date_range("2016-01-01", periods=len(prices), freq="D")
    return pd.DataFrame(
        {"open": opn, "high": high, "low": low, "close": prices},
        index=dates,
    )


# =========================================================================
# 管线运行
# =========================================================================

def run_pipeline(df_raw, mode="strict", sustain_m=2):
    """运行完整管线，返回 (strokes, segments, rec_levels)。"""
    df_merged, merged_to_raw = merge_inclusion(df_raw)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode=mode,
        merged_to_raw=merged_to_raw if mode == "new" else None,
    )
    segments = segments_from_strokes_v1(strokes)
    rec_levels = build_recursive_levels(segments, sustain_m=sustain_m)
    return strokes, segments, rec_levels


# =========================================================================
# T8 递归层验证
# =========================================================================

def verify_t8_at_level(rec_level, strokes=None, level_label=""):
    """对单个递归层运行 T8 验证。

    Returns dict with verification results.
    """
    moves = rec_level.moves
    centers = rec_level.centers
    trends = rec_level.trends
    level_id = rec_level.level

    # 检测背驰
    divs = divergences_from_level(moves, centers, trends, level_id)

    if not divs:
        return {
            "level": level_id,
            "label": level_label,
            "n_divergences": 0,
            "t8_results": [],
            "summary": "no divergences detected",
        }

    # T8 拓扑后验
    t8_checks = check_divergence_topology(
        divs, moves, centers, strokes=strokes, eta=0.0, tau=0.0,
    )

    details = []
    for r in t8_checks:
        details.append({
            "div_idx": r.divergence_index,
            "kind": r.kind,
            "direction": r.direction,
            "w1_a": round(r.w1_a, 6),
            "w1_c": round(r.w1_c, 6),
            "w1_drop": round(r.w1_drop, 6),
            "passed": r.passed,
            "inconclusive": r.inconclusive,
            "n_centers_a": r.n_centers_a,
            "n_centers_c": r.n_centers_c,
            "macd_agrees": r.macd_agrees,
        })

    n_total = len(t8_checks)
    n_inconclusive = sum(1 for r in t8_checks if r.inconclusive)
    n_conclusive = n_total - n_inconclusive
    conclusive = [r for r in t8_checks if not r.inconclusive]
    n_passed = sum(1 for r in conclusive if r.passed)
    n_failed = sum(1 for r in conclusive if not r.passed)

    return {
        "level": level_id,
        "label": level_label,
        "n_divergences": len(divs),
        "n_total": n_total,
        "n_inconclusive": n_inconclusive,
        "n_conclusive": n_conclusive,
        "n_passed": n_passed,
        "n_failed": n_failed,
        "pass_rate": n_passed / n_conclusive if n_conclusive > 0 else None,
        "t8_results": details,
    }


def verify_all_levels(df_raw, label, mode="strict"):
    """对一组数据运行管线并验证所有递归层的 T8。"""
    print(f"\n{'='*60}")
    print(f"  {label} (mode={mode}, {len(df_raw)} bars)")
    print(f"{'='*60}")

    strokes, segments, rec_levels = run_pipeline(df_raw, mode=mode)
    print(f"  Strokes: {len(strokes)}, Segments: {len(segments)}, "
          f"Recursive levels: {len(rec_levels)}")

    if not rec_levels:
        print("  [SKIP] No recursive levels produced")
        return {"label": label, "mode": mode, "n_levels": 0, "levels": []}

    for i, rl in enumerate(rec_levels):
        n_confirmed = sum(1 for t in rl.trends if t.confirmed)
        print(f"  Level {rl.level}: {len(rl.centers)} centers, "
              f"{len(rl.trends)} trends ({n_confirmed} confirmed), "
              f"{len(rl.moves)} moves")

    results = []
    for rl in rec_levels:
        # strokes 只在 level 1 有意义（用于笔振荡 fallback）
        # 高级别的 moves 是 TrendTypeInstance，没有对应的 strokes
        s = strokes if rl.level == 1 else None
        r = verify_t8_at_level(rl, strokes=s, level_label=label)
        results.append(r)

        status = "PASS" if r.get("pass_rate") is not None and r["pass_rate"] >= 0.5 else "---"
        if r["n_divergences"] == 0:
            print(f"    Level {rl.level}: no divergences")
        elif r.get("n_conclusive", 0) == 0:
            print(f"    Level {rl.level}: {r['n_divergences']} divs, all inconclusive")
        else:
            print(f"    Level {rl.level}: {r['n_divergences']} divs, "
                  f"{r['n_passed']}/{r['n_conclusive']} passed "
                  f"({r.get('pass_rate', 0):.0%}), "
                  f"{r['n_inconclusive']} inconclusive  [{status}]")

    max_level = max(rl.level for rl in rec_levels)
    return {
        "label": label,
        "mode": mode,
        "n_bars": len(df_raw),
        "n_strokes": len(strokes),
        "n_segments": len(segments),
        "n_levels": len(rec_levels),
        "max_level": max_level,
        "levels": results,
    }


# =========================================================================
# 主流程
# =========================================================================

def main():
    print("=" * 60)
    print("递归级别 >= 1 的 T8 背驰拓扑化验证")
    print("=" * 60)

    all_results = []

    # --- Phase 1: 合成数据（确保产生多层递归）---
    print("\n\n### Phase 1: 合成数据 ###")

    for n, seed in [(2000, 42), (3000, 99), (2000, 123), (5000, 456)]:
        df = make_synthetic_multilevel(n=n, seed=seed)
        r = verify_all_levels(df, f"synthetic-multilevel-n{n}-s{seed}")
        all_results.append(r)

    for seed in [99, 200, 301]:
        df = make_synthetic_strong_oscillation(n=3000, seed=seed)
        r = verify_all_levels(df, f"synthetic-strong-osc-s{seed}")
        all_results.append(r)

    # --- Phase 2: 真实市场数据（akshare）---
    print("\n\n### Phase 2: 真实市场数据 ###")

    STOCKS = [
        ("sz000001", "平安银行"),
        ("sh600036", "招商银行"),
        ("sh600519", "贵州茅台"),
        ("sz300750", "宁德时代"),
        ("sz002594", "比亚迪"),
        ("sh601899", "紫金矿业"),
    ]

    for symbol, name in STOCKS:
        df = fetch_akshare_daily(symbol, name, "20160101", "20260225")
        if df is not None and len(df) > 200:
            r = verify_all_levels(df, f"{symbol}-{name}")
            all_results.append(r)

    # --- 汇总 ---
    print("\n\n" + "=" * 60)
    print("汇总")
    print("=" * 60)

    # 统计有 level >= 2 的数据集
    multi_level = [r for r in all_results if r["n_levels"] >= 2]
    print(f"\n产生 level >= 2 的数据集: {len(multi_level)}/{len(all_results)}")

    # 按级别汇总 T8 结果
    level_stats = {}
    for r in all_results:
        for lv in r["levels"]:
            lvl = lv["level"]
            if lvl not in level_stats:
                level_stats[lvl] = {
                    "n_datasets": 0,
                    "total_divs": 0,
                    "total_conclusive": 0,
                    "total_passed": 0,
                    "total_inconclusive": 0,
                }
            level_stats[lvl]["n_datasets"] += 1
            level_stats[lvl]["total_divs"] += lv["n_divergences"]
            level_stats[lvl]["total_conclusive"] += lv.get("n_conclusive", 0)
            level_stats[lvl]["total_passed"] += lv.get("n_passed", 0)
            level_stats[lvl]["total_inconclusive"] += lv.get("n_inconclusive", 0)

    print(f"\n{'Level':<8} {'Datasets':<10} {'Divs':<8} {'Conclusive':<12} "
          f"{'Passed':<8} {'Rate':<8} {'Inconclusive':<14}")
    print("-" * 78)
    for lvl in sorted(level_stats.keys()):
        s = level_stats[lvl]
        rate = s["total_passed"] / s["total_conclusive"] if s["total_conclusive"] > 0 else float("nan")
        rate_str = f"{rate:.1%}" if s["total_conclusive"] > 0 else "N/A"
        print(f"{lvl:<8} {s['n_datasets']:<10} {s['total_divs']:<8} "
              f"{s['total_conclusive']:<12} {s['total_passed']:<8} "
              f"{rate_str:<8} {s['total_inconclusive']:<14}")

    # 重点：level >= 2 的 T8 结果
    print("\n--- Level >= 2 的 T8 详细结果 ---")
    has_high_level_t8 = False
    for r in all_results:
        for lv in r["levels"]:
            if lv["level"] >= 2 and lv["n_divergences"] > 0:
                has_high_level_t8 = True
                print(f"\n  [{r['label']}] Level {lv['level']}:")
                for d in lv["t8_results"]:
                    status = "PASS" if d["passed"] else ("INCONC" if d["inconclusive"] else "FAIL")
                    print(f"    div#{d['div_idx']} {d['kind']}/{d['direction']}: "
                          f"W1_A={d['w1_a']:.4f} W1_C={d['w1_c']:.4f} "
                          f"drop={d['w1_drop']:.4f} "
                          f"centers_A={d['n_centers_a']} centers_C={d['n_centers_c']} "
                          f"[{status}]")

    if not has_high_level_t8:
        print("  (无 level >= 2 的背驰检测结果)")

    # 结论
    print("\n\n" + "=" * 60)
    print("结论")
    print("=" * 60)

    if not level_stats:
        print("无递归层产生，验证无法进行。")
        return all_results

    max_lvl = max(level_stats.keys())
    if max_lvl < 2:
        print(f"最高递归层级: {max_lvl}。未产生 level >= 2，")
        print("T8 在递归层的验证仅限于 level 1。")
        print("根因分析：数据长度/结构不足以产生高级别递归。")
    else:
        for lvl in sorted(level_stats.keys()):
            if lvl >= 2:
                s = level_stats[lvl]
                if s["total_conclusive"] > 0:
                    rate = s["total_passed"] / s["total_conclusive"]
                    print(f"Level {lvl}: T8 通过率 {rate:.1%} "
                          f"({s['total_passed']}/{s['total_conclusive']})")
                else:
                    print(f"Level {lvl}: {s['total_divs']} 个背驰全部 inconclusive "
                          f"(A/C 段无中枢)")

    return all_results


if __name__ == "__main__":
    results = main()
