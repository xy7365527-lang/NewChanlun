"""递归级别 >= 1 的 T8 背驰拓扑化验证（v2）。

v1 发现：所有数据集最高只到 level 1。
根因：strict 模式线段太少，confirmed trends < 3。

v2 策略：
1. 尝试 wide/new 模式（产生更多笔 -> 更多线段）
2. sustain_m=1（降低中枢确认门槛）
3. 更长的合成数据（10000+ bars）
4. 如果仍无法产生 level >= 2，记录结构性原因
"""

import sys
import os
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

import numpy as np
import pandas as pd

from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_recursive_engine import build_recursive_levels
from newchan.a_divergence import divergences_from_level
from newchan.a_topology import check_divergence_topology


# =========================================================================
# 数据生成
# =========================================================================

def make_multilevel_v2(n=5000, seed=42):
    """v2 合成数据：更多小振荡确保产生大量笔和线段。"""
    rng = np.random.RandomState(seed)
    t = np.arange(n)

    # 短周期振荡（产生笔）
    short = 2.0 * np.sin(2 * np.pi * t / 15)
    # 中周期（产生线段方向变化）
    medium = 5.0 * np.sin(2 * np.pi * t / 80)
    # 长周期（产生走势类型）
    long_w = 10.0 * np.sin(2 * np.pi * t / 400)
    # 超长周期（产生高级别中枢）
    vlong = 15.0 * np.sin(2 * np.pi * t / 2000)

    noise = rng.randn(n) * 0.3
    close = 100.0 + short + medium + long_w + vlong + noise
    high = close + rng.uniform(0.05, 0.5, n)
    low = close - rng.uniform(0.05, 0.5, n)
    opn = close + rng.uniform(-0.2, 0.2, n)

    dates = pd.date_range("2010-01-01", periods=n, freq="D")
    return pd.DataFrame(
        {"open": opn, "high": high, "low": low, "close": close},
        index=dates,
    )


def make_zigzag_rich(n=8000, seed=77):
    """构造丰富 zigzag 结构的数据——确保大量线段。

    策略：用分段线性 + 噪声，每 20-40 bars 一个转折。
    """
    rng = np.random.RandomState(seed)
    prices = [100.0]
    direction = 1

    while len(prices) < n:
        seg_len = rng.randint(15, 40)
        amplitude = rng.uniform(1.0, 5.0)
        for _ in range(seg_len):
            step = direction * rng.uniform(0.02, 0.15) + rng.randn() * 0.08
            prices.append(prices[-1] + step)
        direction *= -1

    prices = np.array(prices[:n])
    high = prices + rng.uniform(0.05, 0.3, n)
    low = prices - rng.uniform(0.05, 0.3, n)
    opn = prices + rng.uniform(-0.1, 0.1, n)

    dates = pd.date_range("2010-01-01", periods=n, freq="D")
    return pd.DataFrame(
        {"open": opn, "high": high, "low": low, "close": prices},
        index=dates,
    )


def fetch_akshare_daily(symbol, name, start, end):
    try:
        import akshare as ak
        time.sleep(2)
        df = ak.stock_zh_a_daily(
            symbol=symbol, start_date=start, end_date=end, adjust="qfq",
        )
        if df is None or len(df) == 0:
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


# =========================================================================
# 管线
# =========================================================================

def run_pipeline(df_raw, mode="wide", sustain_m=1):
    df_merged, merged_to_raw = merge_inclusion(df_raw)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode=mode,
        merged_to_raw=merged_to_raw if mode == "new" else None,
    )
    segments = segments_from_strokes_v1(strokes)
    rec_levels = build_recursive_levels(segments, sustain_m=sustain_m)
    return strokes, segments, rec_levels


def verify_t8_at_level(rec_level, strokes=None):
    moves = rec_level.moves
    centers = rec_level.centers
    trends = rec_level.trends
    level_id = rec_level.level

    divs = divergences_from_level(moves, centers, trends, level_id)
    if not divs:
        return {
            "level": level_id,
            "n_divergences": 0,
            "n_conclusive": 0,
            "n_passed": 0,
            "n_failed": 0,
            "n_inconclusive": 0,
            "pass_rate": None,
            "details": [],
        }

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
    n_inc = sum(1 for r in t8_checks if r.inconclusive)
    conclusive = [r for r in t8_checks if not r.inconclusive]
    n_pass = sum(1 for r in conclusive if r.passed)
    n_fail = sum(1 for r in conclusive if not r.passed)

    return {
        "level": level_id,
        "n_divergences": len(divs),
        "n_conclusive": len(conclusive),
        "n_passed": n_pass,
        "n_failed": n_fail,
        "n_inconclusive": n_inc,
        "pass_rate": n_pass / len(conclusive) if conclusive else None,
        "details": details,
    }


# =========================================================================
# 主流程
# =========================================================================

def run_one(df_raw, label, mode, sustain_m):
    """运行单个配置并返回结果。"""
    strokes, segments, rec_levels = run_pipeline(df_raw, mode=mode, sustain_m=sustain_m)
    max_level = max((rl.level for rl in rec_levels), default=0)
    n_confirmed_l1 = 0
    if rec_levels:
        n_confirmed_l1 = sum(1 for t in rec_levels[0].trends if t.confirmed)

    tag = f"{label} mode={mode} m={sustain_m}"
    print(f"  {tag}: {len(strokes)} strokes, {len(segments)} segs, "
          f"{len(rec_levels)} levels (max={max_level}), "
          f"L1 confirmed={n_confirmed_l1}")

    level_results = []
    for rl in rec_levels:
        s = strokes if rl.level == 1 else None
        r = verify_t8_at_level(rl, strokes=s)
        level_results.append(r)

        if r["n_divergences"] > 0:
            if r["n_conclusive"] > 0:
                print(f"    L{rl.level}: {r['n_passed']}/{r['n_conclusive']} passed, "
                      f"{r['n_inconclusive']} inconc")
            else:
                print(f"    L{rl.level}: {r['n_divergences']} divs, all inconclusive")

    return {
        "label": tag,
        "n_bars": len(df_raw),
        "n_strokes": len(strokes),
        "n_segments": len(segments),
        "n_levels": len(rec_levels),
        "max_level": max_level,
        "n_confirmed_l1": n_confirmed_l1,
        "levels": level_results,
    }


def main():
    print("=" * 60)
    print("递归 T8 验证 v2：多模式 + 多参数扫描")
    print("=" * 60)

    all_results = []
    MODES = ["wide", "strict", "new"]
    SUSTAIN = [1, 2]

    # --- Phase 1: 合成数据参数扫描 ---
    print("\n### Phase 1: 合成数据参数扫描 ###\n")

    datasets = [
        ("synth-multilevel-5k", make_multilevel_v2(5000, 42)),
        ("synth-multilevel-10k", make_multilevel_v2(10000, 42)),
        ("synth-zigzag-8k", make_zigzag_rich(8000, 77)),
        ("synth-zigzag-8k-s2", make_zigzag_rich(8000, 200)),
    ]

    for name, df in datasets:
        print(f"\n--- {name} ({len(df)} bars) ---")
        for mode in MODES:
            for m in SUSTAIN:
                r = run_one(df, name, mode, m)
                all_results.append(r)

    # --- Phase 2: 真实数据（长周期）---
    print("\n\n### Phase 2: 真实数据（2010-2026）###\n")

    STOCKS = [
        ("sz000001", "平安银行"),
        ("sh600519", "贵州茅台"),
        ("sz300750", "宁德时代"),
        ("sh601899", "紫金矿业"),
    ]

    for symbol, name in STOCKS:
        df = fetch_akshare_daily(symbol, name, "20100101", "20260225")
        if df is not None and len(df) > 500:
            print(f"\n--- {symbol} {name} ({len(df)} bars) ---")
            for mode in MODES:
                for m in SUSTAIN:
                    r = run_one(df, f"{symbol}-{name}", mode, m)
                    all_results.append(r)

    # --- 汇总 ---
    print("\n\n" + "=" * 60)
    print("汇总")
    print("=" * 60)

    # 找到产生 level >= 2 的配置
    multi = [r for r in all_results if r["max_level"] >= 2]
    print(f"\n产生 level >= 2 的配置: {len(multi)}/{len(all_results)}")

    if multi:
        print("\n详细：")
        for r in multi:
            print(f"  {r['label']}: max_level={r['max_level']}, "
                  f"{r['n_segments']} segs, {r['n_confirmed_l1']} confirmed_L1")

    # 按级别汇总
    level_stats = {}
    for r in all_results:
        for lv in r["levels"]:
            lvl = lv["level"]
            if lvl not in level_stats:
                level_stats[lvl] = {"datasets": 0, "divs": 0, "conclusive": 0,
                                     "passed": 0, "inconclusive": 0}
            level_stats[lvl]["datasets"] += 1
            level_stats[lvl]["divs"] += lv["n_divergences"]
            level_stats[lvl]["conclusive"] += lv["n_conclusive"]
            level_stats[lvl]["passed"] += lv["n_passed"]
            level_stats[lvl]["inconclusive"] += lv["n_inconclusive"]

    print(f"\n{'Level':<8} {'Sets':<6} {'Divs':<6} {'Conc':<6} "
          f"{'Pass':<6} {'Rate':<8} {'Inconc':<8}")
    print("-" * 50)
    for lvl in sorted(level_stats.keys()):
        s = level_stats[lvl]
        rate = s["passed"] / s["conclusive"] if s["conclusive"] > 0 else float("nan")
        rate_str = f"{rate:.1%}" if s["conclusive"] > 0 else "N/A"
        print(f"{lvl:<8} {s['datasets']:<6} {s['divs']:<6} {s['conclusive']:<6} "
              f"{s['passed']:<6} {rate_str:<8} {s['inconclusive']:<8}")

    # Level >= 2 详细
    print("\n--- Level >= 2 T8 详细 ---")
    found_high = False
    for r in all_results:
        for lv in r["levels"]:
            if lv["level"] >= 2 and lv["n_divergences"] > 0:
                found_high = True
                print(f"\n  [{r['label']}] Level {lv['level']}:")
                for d in lv["details"]:
                    st = "PASS" if d["passed"] else ("INCONC" if d["inconclusive"] else "FAIL")
                    print(f"    #{d['div_idx']} {d['kind']}/{d['direction']}: "
                          f"W1_A={d['w1_a']:.4f} W1_C={d['w1_c']:.4f} "
                          f"drop={d['w1_drop']:.4f} "
                          f"cA={d['n_centers_a']} cC={d['n_centers_c']} [{st}]")

    if not found_high:
        print("  (无 level >= 2 背驰)")

    # 结论
    print("\n\n" + "=" * 60)
    print("结论")
    print("=" * 60)

    max_lvl = max(level_stats.keys()) if level_stats else 0
    if max_lvl < 2:
        print(f"最高递归层级: {max_lvl}")
        print("所有配置（3 modes x 2 sustain_m x 多数据集）均未产生 level >= 2。")
        print("结构性原因：递归引擎要求 confirmed_trends >= 3 才触发下一级，")
        print("而当前数据/参数组合下 level 1 的 confirmed trends 不足。")
        print("\n这是递归引擎的正常行为——级别是市场'长'出来的，")
        print("不是所有数据都能产生高级别递归。")
        print("\nT8 在 level 1 的验证结果：")
        if 1 in level_stats:
            s = level_stats[1]
            if s["conclusive"] > 0:
                rate = s["passed"] / s["conclusive"]
                print(f"  通过率: {rate:.1%} ({s['passed']}/{s['conclusive']})")
            print(f"  inconclusive: {s['inconclusive']}")
    else:
        print(f"最高递归层级: {max_lvl}")
        for lvl in sorted(level_stats.keys()):
            if lvl >= 2:
                s = level_stats[lvl]
                if s["conclusive"] > 0:
                    rate = s["passed"] / s["conclusive"]
                    print(f"  Level {lvl}: T8 通过率 {rate:.1%} "
                          f"({s['passed']}/{s['conclusive']})")
                else:
                    print(f"  Level {lvl}: {s['divs']} divs, all inconclusive")

    return all_results


if __name__ == "__main__":
    main()
