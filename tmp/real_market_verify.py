"""真实市场数据验证脚本 v3：stock_zh_a_daily + 合成数据。"""

import sys
import time
import traceback
from datetime import datetime

sys.path.insert(0, "src")

import pandas as pd
import numpy as np


# ---------------------------------------------------------------------------
# 1. 数据获取
# ---------------------------------------------------------------------------

STOCKS = [
    ("sz000001", "平安银行", "大盘股"),
    ("sh600036", "招商银行", "大盘股"),
    ("sz000858", "五粮液", "中盘股"),
    ("sh601318", "中国平安", "大盘股"),
    ("sz002475", "立讯精密", "中小盘股"),
]

START_DATE = "20180101"
END_DATE = "20260225"


def fetch_stock(symbol: str, name: str) -> pd.DataFrame | None:
    try:
        import akshare as ak
        time.sleep(3)
        df = ak.stock_zh_a_daily(
            symbol=symbol, start_date=START_DATE, end_date=END_DATE, adjust="qfq",
        )
        if df is None or len(df) == 0:
            print(f"  [WARN] {symbol} {name}: no data")
            return None
        result = pd.DataFrame({
            "open": df["open"].values.astype(float),
            "high": df["high"].values.astype(float),
            "low": df["low"].values.astype(float),
            "close": df["close"].values.astype(float),
        }, index=pd.to_datetime(df["date"]))
        print(f"  [OK] {symbol} {name}: {len(result)} bars, "
              f"{result.index[0].date()} ~ {result.index[-1].date()}")
        return result
    except Exception as e:
        print(f"  [ERR] {symbol} {name}: {e}")
        return None


def generate_deep_recursive(n: int = 1200, seed: int = 42) -> pd.DataFrame:
    rng = np.random.RandomState(seed)
    t = np.arange(n, dtype=float)
    trend = np.zeros(n)
    phase_len = n // 4
    for i in range(n):
        phase = i // phase_len
        if phase == 0:
            trend[i] = 100 + 30 * (i / phase_len)
        elif phase == 1:
            trend[i] = 130 - 20 * ((i - phase_len) / phase_len)
        elif phase == 2:
            trend[i] = 110 + 40 * ((i - 2*phase_len) / phase_len)
        else:
            trend[i] = 150 - 30 * ((i - 3*phase_len) / phase_len)
    cycle_mid = 8.0 * np.sin(2 * np.pi * t / 50)
    cycle_small = 3.0 * np.sin(2 * np.pi * t / 12)
    noise = rng.randn(n) * 0.3
    close = trend + cycle_mid + cycle_small + noise
    high = close + rng.uniform(0.2, 1.2, n)
    low = close - rng.uniform(0.2, 1.2, n)
    opn = close + rng.uniform(-0.3, 0.3, n)
    dates = pd.date_range("2024-01-01", periods=n, freq="30min")
    return pd.DataFrame({"open": opn, "high": high, "low": low, "close": close}, index=dates)


def generate_multi_center(n: int = 1500, seed: int = 77) -> pd.DataFrame:
    rng = np.random.RandomState(seed)
    t = np.arange(n, dtype=float)
    prices = np.zeros(n)
    base = 100.0
    segments = [
        (0, 200, 0.15, 6.0), (200, 400, 0.0, 8.0),
        (400, 550, 0.08, 5.0), (550, 750, -0.12, 6.0),
        (750, 950, 0.0, 7.0), (950, 1100, -0.06, 5.0),
        (1100, 1300, 0.10, 6.0), (1300, 1500, 0.0, 8.0),
    ]
    for start, end, drift, amp in segments:
        actual_end = min(end, n)
        seg_len = actual_end - start
        if seg_len <= 0:
            break
        seg_t = np.arange(seg_len, dtype=float)
        seg_prices = base + drift * seg_t + amp * np.sin(2*np.pi*seg_t/40) + 2.5*np.sin(2*np.pi*seg_t/10)
        prices[start:actual_end] = seg_prices
        base = seg_prices[-1]
    noise = rng.randn(n) * 0.2
    close = prices + noise
    high = close + rng.uniform(0.2, 1.0, n)
    low = close - rng.uniform(0.2, 1.0, n)
    opn = close + rng.uniform(-0.3, 0.3, n)
    dates = pd.date_range("2024-01-01", periods=n, freq="15min")
    return pd.DataFrame({"open": opn, "high": high, "low": low, "close": close}, index=dates)


# ---------------------------------------------------------------------------
# 2. Pipeline
# ---------------------------------------------------------------------------

def run_verification(df: pd.DataFrame, label: str) -> dict:
    from newchan.a_topology import (
        gauge_equivalence_report, check_trend_move_equivalence,
        check_recursive_barcode_order, check_divergence_topology,
        check_cross_level_leray, _run_pipeline,
    )
    report = gauge_equivalence_report(df, tau=0.0)

    best_t5, best_t7 = [], []
    all_t8 = []  # collect ALL T8 results across modes/levels
    all_t6 = []  # collect ALL T6 results across modes
    for mode in ("wide", "strict", "new"):
        try:
            s, seg, c, t, rl = _run_pipeline(df, mode)
            if len(rl) >= 2:
                t5c = check_trend_move_equivalence(rl)
                if len(t5c) > len(best_t5):
                    best_t5 = [{"level_low": r.level_low, "level_high": r.level_high,
                                "passed": r.passed, "confirmed_only": r.confirmed_only,
                                "identity": r.identity, "mode": mode} for r in t5c]
                t7c = check_recursive_barcode_order(rl, tau=0.0)
                if len(t7c) > len(best_t7):
                    best_t7 = [{"level_low": r.level_low, "level_high": r.level_high,
                                "passed": r.passed, "trimmed_low_count": r.trimmed_low_count,
                                "high_count": r.high_count, "mode": mode} for r in t7c]
                # T6 跨层 Leray 可计算近似
                t6c = check_cross_level_leray(rl, tau=0.0)
                for r in t6c:
                    all_t6.append({
                        "level_low": r.level_low, "level_high": r.level_high,
                        "passed": r.passed, "w1_low": r.w1_low, "w1_high": r.w1_high,
                        "w1_ratio": r.w1_ratio, "w1_ratio_bounded": r.w1_ratio_bounded,
                        "bottleneck_dist": r.bottleneck_dist, "bottleneck_bounded": r.bottleneck_bounded,
                        "kl_divergence": r.kl_divergence, "kl_bounded": r.kl_bounded,
                        "inconclusive": r.inconclusive,
                        "n_bars_low": r.n_bars_low, "n_bars_high": r.n_bars_high,
                        "mode": mode,
                    })
            from newchan.a_divergence import divergences_from_level
            for level in (rl if rl else [None]):
                try:
                    if level is not None:
                        moves = level.moves if hasattr(level, "moves") else seg
                        centers, trends, lvl = level.centers, level.trends, level.level
                    else:
                        moves, centers, trends, lvl = seg, c, t, 0
                    divs = divergences_from_level(moves, centers, trends, lvl)
                    if divs:
                        t8c = check_divergence_topology(divs, moves, centers, strokes=s, eta=0.0)
                        for r in t8c:
                            all_t8.append({"divergence_index": r.divergence_index, "kind": r.kind,
                                           "direction": r.direction, "w1_a": r.w1_a, "w1_c": r.w1_c,
                                           "w1_drop": r.w1_drop, "passed": r.passed,
                                           "inconclusive": r.inconclusive, "macd_agrees": r.macd_agrees,
                                           "mode": mode, "level": lvl,
                                           "n_centers_a": r.n_centers_a, "n_centers_c": r.n_centers_c})
                except Exception:
                    pass
        except Exception:
            pass

    report["t5_trend_move"] = best_t5
    report["t7_recursive_order_extended"] = best_t7
    report["t8_divergence_topology_extended"] = all_t8
    report["t6_cross_level_leray_extended"] = all_t6
    report["label"] = label
    report["n_bars"] = len(df)
    return report


# ---------------------------------------------------------------------------
# 3. Report
# ---------------------------------------------------------------------------

def generate_report(results: list):
    lines = []
    lines.append("# 真实市场数据验证报告")
    lines.append("")
    lines.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}")
    lines.append(f"谱系引用：195号-5（拓扑不变量真实数据验证）+ 202号-3（gauge equivalence 经验验证）")
    lines.append("")
    lines.append("## 数据来源")
    lines.append("")
    lines.append("- 真实数据：akshare `stock_zh_a_daily`（前复权）")
    lines.append(f"- 时间范围：{START_DATE} ~ {END_DATE}")
    lines.append("- 合成数据：深递归结构（1200根30min）+ 多中枢背驰结构（1500根15min）")
    lines.append("- 数据集：")
    for symbol, name, cap, r in results:
        lines.append(f"  - {symbol} {name}（{cap}）— {r['n_bars']} 根K线")
    lines.append("")

    lines.append("## 结果汇总")
    lines.append("")
    lines.append("| 数据集 | K线数 | T1(barcode) | T3(centers) | T4(beta1) | T5 | T6 | T7 | T8 | 强不变量通过率 |")
    lines.append("|--------|-------|-------------|-------------|-----------|-----|-----|-----|-----|---------------|")

    for symbol, name, cap, r in results:
        ss = r["strong_invariant_summary"]
        t1 = ss.get("barcode_bottleneck", {})
        t3 = ss.get("n_centers", {})
        t4 = ss.get("beta1_tau", {})
        t5l = r.get("t5_trend_move", [])
        t6l = r.get("t6_cross_level_leray_extended", [])
        t7l = r.get("t7_recursive_order_extended", []) or r.get("t7_recursive_order", [])
        t8l = r.get("t8_divergence_topology_extended", []) or r.get("t8_divergence_topology", [])
        t8ni = [x for x in t8l if not x.get("inconclusive", True)]
        t8inc = len(t8l) - len(t8ni)
        t6ni = [x for x in t6l if not x.get("inconclusive", False)]
        t6inc = len(t6l) - len(t6ni)
        tp = sum(v.get("preserved", 0) for v in ss.values())
        tt = sum(v.get("total", 0) for v in ss.values())
        rate = tp / tt if tt > 0 else 0.0

        def fmt(lst, key="passed"): return f"{sum(1 for x in lst if x[key])}/{len(lst)}" if lst else "N/A"
        t8s = f"{sum(1 for x in t8ni if x['passed'])}/{len(t8ni)}" if t8ni else (f"0/0(inc:{t8inc})" if t8inc else "N/A")
        t6s = f"{sum(1 for x in t6ni if x['passed'])}/{len(t6ni)}" if t6ni else (f"0/0(inc:{t6inc})" if t6inc else "N/A")

        lines.append(f"| {symbol} {name} | {r['n_bars']} | {t1.get('preserved',0)}/{t1.get('total',0)} "
                     f"| {t3.get('preserved',0)}/{t3.get('total',0)} | {t4.get('preserved',0)}/{t4.get('total',0)} "
                     f"| {fmt(t5l)} | {t6s} | {fmt(t7l)} | {t8s} | {rate:.1%} |")
    lines.append("")

    # T6 跨层 Leray 可计算近似详细结果
    lines.append("## T6 跨层 Leray 可计算近似详细结果")
    lines.append("")
    any_t6 = False
    all_t6_ratios = []
    all_t6_bn = []
    all_t6_kl = []
    for symbol, name, cap, r in results:
        t6l = r.get("t6_cross_level_leray_extended", [])
        if not t6l: continue
        any_t6 = True
        lines.append(f"### {symbol} {name}")
        lines.append("")
        lines.append("| level | passed | W₁_low | W₁_high | W₁_ratio | ratio_bounded | bn_dist | bn_bounded | KL | kl_bounded | inconclusive | bars_low | bars_high | mode |")
        lines.append("|-------|--------|--------|---------|----------|---------------|---------|------------|-----|------------|--------------|----------|-----------|------|")
        for t6 in t6l:
            w1r = t6['w1_ratio']
            if w1r != float('inf'):
                all_t6_ratios.append(w1r)
            all_t6_bn.append(t6['bottleneck_dist'])
            if not t6.get('inconclusive', False):
                all_t6_kl.append(t6['kl_divergence'])
            lines.append(f"| {t6['level_low']}→{t6['level_high']} | {t6['passed']} "
                         f"| {t6['w1_low']:.2f} | {t6['w1_high']:.2f} | {w1r:.4f} "
                         f"| {t6['w1_ratio_bounded']} | {t6['bottleneck_dist']:.4f} "
                         f"| {t6['bottleneck_bounded']} | {t6['kl_divergence']:.4f} "
                         f"| {t6['kl_bounded']} | {t6['inconclusive']} "
                         f"| {t6['n_bars_low']} | {t6['n_bars_high']} | {t6.get('mode','?')} |")
        ni = [x for x in t6l if not x.get("inconclusive", False)]
        lines.append("")
        lines.append(f"非inconclusive：{len(ni)}项" + (f"，通过{sum(1 for x in ni if x['passed'])}项" if ni else ""))
        lines.append("")
    if not any_t6:
        lines.append("所有数据集均无 T6 结果（递归级别 < 2）。\n")

    # T6 参数校准
    lines.append("## T6 参数校准（207号-3）")
    lines.append("")
    if all_t6_ratios:
        import numpy as np
        ratios = np.array(all_t6_ratios)
        bns = np.array(all_t6_bn)
        lines.append(f"### W₁ 比率（λ 校准）")
        lines.append(f"- 样本数：{len(ratios)}")
        lines.append(f"- 中位数：{np.median(ratios):.4f}")
        lines.append(f"- 均值：{np.mean(ratios):.4f}")
        lines.append(f"- 最大值：{np.max(ratios):.4f}")
        lines.append(f"- P75：{np.percentile(ratios, 75):.4f}")
        lines.append(f"- P90：{np.percentile(ratios, 90):.4f}")
        lines.append(f"- P95：{np.percentile(ratios, 95):.4f}")
        lines.append(f"- 建议 λ_strict={np.percentile(ratios, 75):.2f}, λ_moderate={np.percentile(ratios, 90):.2f}, λ_lenient={np.percentile(ratios, 95):.2f}")
        lines.append("")
        lines.append(f"### Bottleneck 距离（δ 校准）")
        lines.append(f"- 样本数：{len(bns)}")
        lines.append(f"- 中位数：{np.median(bns):.4f}")
        lines.append(f"- 均值：{np.mean(bns):.4f}")
        lines.append(f"- 最大值：{np.max(bns):.4f}")
        lines.append(f"- P75：{np.percentile(bns, 75):.4f}")
        lines.append(f"- P90：{np.percentile(bns, 90):.4f}")
        lines.append(f"- P95：{np.percentile(bns, 95):.4f}")
        lines.append(f"- 建议 δ_strict={np.percentile(bns, 75):.2f}, δ_moderate={np.percentile(bns, 90):.2f}, δ_lenient={np.percentile(bns, 95):.2f}")
        lines.append("")
        if all_t6_kl:
            kls = np.array(all_t6_kl)
            lines.append(f"### KL 散度（κ 校准）")
            lines.append(f"- 样本数（非inconclusive）：{len(kls)}")
            lines.append(f"- 中位数：{np.median(kls):.4f}")
            lines.append(f"- 均值：{np.mean(kls):.4f}")
            lines.append(f"- 最大值：{np.max(kls):.4f}")
            lines.append(f"- P75：{np.percentile(kls, 75):.4f}")
            lines.append(f"- P90：{np.percentile(kls, 90):.4f}")
            lines.append(f"- P95：{np.percentile(kls, 95):.4f}")
            lines.append(f"- 建议 κ_strict={np.percentile(kls, 75):.4f}, κ_moderate={np.percentile(kls, 90):.4f}, κ_lenient={np.percentile(kls, 95):.4f}")
            lines.append("")
        else:
            lines.append("所有 T6 结果均为 inconclusive（小样本），无 KL 校准数据。\n")
    else:
        lines.append("无 T6 数据可供校准。\n")

    # T8
    lines.append("## T8 背驰拓扑详细结果")
    lines.append("")
    any_t8 = False
    for symbol, name, cap, r in results:
        t8l = r.get("t8_divergence_topology_extended", []) or r.get("t8_divergence_topology", [])
        if not t8l: continue
        any_t8 = True
        lines.append(f"### {symbol} {name}")
        lines.append("")
        lines.append("| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |")
        lines.append("|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|")
        for t8 in t8l:
            lines.append(f"| {t8['divergence_index']} | {t8['kind']} | {t8['direction']} "
                         f"| {t8['w1_a']:.4f} | {t8['w1_c']:.4f} | {t8['w1_drop']:.4f} "
                         f"| {t8['passed']} | {t8['inconclusive']} "
                         f"| {t8.get('n_centers_a','?')} | {t8.get('n_centers_c','?')} "
                         f"| {t8.get('mode','?')} | {t8.get('level','?')} |")
        ni = [x for x in t8l if not x.get("inconclusive", True)]
        lines.append("")
        lines.append(f"非inconclusive：{len(ni)}项" + (f"，通过{sum(1 for x in ni if x['passed'])}项" if ni else ""))
        lines.append("")
    if not any_t8:
        lines.append("所有数据集均无 T8 结果。\n")

    # T7
    lines.append("## T7 递归条形码偏序详细结果")
    lines.append("")
    any_t7 = False
    for symbol, name, cap, r in results:
        t7l = r.get("t7_recursive_order_extended", []) or r.get("t7_recursive_order", [])
        if not t7l: continue
        any_t7 = True
        lines.append(f"### {symbol} {name}\n")
        lines.append("| level_low | level_high | passed | trimmed_low | high_count | mode |")
        lines.append("|-----------|------------|--------|-------------|------------|------|")
        for t7 in t7l:
            lines.append(f"| {t7['level_low']} | {t7['level_high']} | {t7['passed']} "
                         f"| {t7['trimmed_low_count']} | {t7['high_count']} | {t7.get('mode','?')} |")
        lines.append("")
    if not any_t7:
        lines.append("所有数据集均无 T7 结果（递归级别 < 2）。\n")

    # T5
    lines.append("## T5 走势等价详细结果")
    lines.append("")
    any_t5 = False
    for symbol, name, cap, r in results:
        t5l = r.get("t5_trend_move", [])
        if not t5l: continue
        any_t5 = True
        lines.append(f"### {symbol} {name}\n")
        lines.append("| level_low | level_high | passed | confirmed_only | identity | mode |")
        lines.append("|-----------|------------|--------|----------------|----------|------|")
        for t5 in t5l:
            lines.append(f"| {t5['level_low']} | {t5['level_high']} | {t5['passed']} "
                         f"| {t5['confirmed_only']} | {t5['identity']} | {t5.get('mode','?')} |")
        lines.append("")
    if not any_t5:
        lines.append("所有数据集均无 T5 结果（递归级别 < 2）。\n")

    # Transitions
    lines.append("## 模式对 Transition 详情")
    lines.append("")
    for symbol, name, cap, r in results:
        lines.append(f"### {symbol} {name}\n")
        for tr in r.get("transitions", []):
            d = tr["delta"]
            lines.append(f"**{tr['source']} -> {tr['target']}**")
            lines.append(f"- stroke_diff={d['stroke_count_diff']}, segment_diff={d['segment_count_diff']}, "
                         f"center_diff={d['center_count_diff']}, level_diff={d['level_diff']}")
            lines.append(f"- bottleneck={d['bottleneck_distance']:.4f}, beta1_tau_diff={d['beta1_tau_diff']}")
            lines.append(f"- trend_mutations={d['trend_mutations']}")
            lines.append(f"- strong: {tr['strong_preserved']}\n")

    # 结论
    lines.append("## 结论\n")
    for label, subset in [("真实数据", [(s,n,c,r) for s,n,c,r in results if not s.startswith("SYN")]),
                          ("合成数据", [(s,n,c,r) for s,n,c,r in results if s.startswith("SYN")]),
                          ("总体", results)]:
        sp = sum(v.get("preserved",0) for _,_,_,r in subset for v in r["strong_invariant_summary"].values())
        st = sum(v.get("total",0) for _,_,_,r in subset for v in r["strong_invariant_summary"].values())
        rate = sp/st if st>0 else 0.0
        at7 = [x for _,_,_,r in subset for x in (r.get("t7_recursive_order_extended",[]) or r.get("t7_recursive_order",[]))]
        at8 = [x for _,_,_,r in subset for x in (r.get("t8_divergence_topology_extended",[]) or r.get("t8_divergence_topology",[]))]
        at5 = [x for _,_,_,r in subset for x in r.get("t5_trend_move",[])]
        t8ni = [x for x in at8 if not x.get("inconclusive",True)]
        lines.append(f"### {label}")
        lines.append(f"- 强不变量通过率：{sp}/{st} = {rate:.1%}")
        lines.append(f"- T5：{sum(1 for x in at5 if x['passed'])}/{len(at5)}")
        lines.append(f"- T7：{sum(1 for x in at7 if x['passed'])}/{len(at7)}")
        lines.append(f"- T8 非inconclusive：{sum(1 for x in t8ni if x['passed'])}/{len(t8ni)}，inconclusive：{len(at8)-len(t8ni)}\n")

    with open(".chanlun/review-results/real-market-data-verification-report.md", "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"  报告已写入")


def main():
    print("=" * 60)
    print("真实市场数据验证 v3")
    print("=" * 60)

    datasets = []

    print("\n[Phase 1] 获取 A 股日线数据（stock_zh_a_daily）...")
    for symbol, name, cap in STOCKS:
        df = fetch_stock(symbol, name)
        if df is not None and len(df) >= 100:
            datasets.append((symbol, name, cap, df))

    print(f"\n[Phase 1b] 生成合成数据...")
    df_deep = generate_deep_recursive(1200, 42)
    datasets.append(("SYN-A", "深递归合成", "合成", df_deep))
    print(f"  [OK] SYN-A: {len(df_deep)} bars")
    df_multi = generate_multi_center(1500, 77)
    datasets.append(("SYN-B", "多中枢背驰合成", "合成", df_multi))
    print(f"  [OK] SYN-B: {len(df_multi)} bars")

    print(f"\n[Phase 2] 运行验证（共 {len(datasets)} 组）...")
    results = []
    for symbol, name, cap, df in datasets:
        label = f"{symbol} {name} ({cap})"
        print(f"\n  >>> {label} ({len(df)} bars)...")
        try:
            r = run_verification(df, label)
            results.append((symbol, name, cap, r))
            ss = r["strong_invariant_summary"]
            parts = [f"{k}={v['preserved']}/{v['total']}" for k, v in ss.items()]
            print(f"      强不变量: {' '.join(parts)}")
            t5 = r.get("t5_trend_move", [])
            t7 = r.get("t7_recursive_order_extended", [])
            t8 = r.get("t8_divergence_topology_extended", [])
            t8ni = [x for x in t8 if not x.get("inconclusive", True)]
            print(f"      T5={len(t5)} T7={len(t7)} T8={len(t8)}(non-inc:{len(t8ni)})")
            for x in t8ni:
                print(f"        T8: idx={x['divergence_index']} {x['kind']} {x['direction']} "
                      f"w1_a={x['w1_a']:.4f} w1_c={x['w1_c']:.4f} passed={x['passed']} lvl={x['level']}")
        except Exception as e:
            print(f"      [ERR] {e}")
            traceback.print_exc()

    print(f"\n[Phase 3] 生成报告...")
    generate_report(results)
    print("\n[DONE]")


if __name__ == "__main__":
    main()
