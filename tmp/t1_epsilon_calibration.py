"""T1 barcode bottleneck ε 校准 + 扩展样本验证（203号-3/4）。

扩展到 15 只 A 股（覆盖大/中/小盘 + 不同行业），
从 bottleneck 分布推导 ε 阈值建议。
"""

import sys
import time
import traceback
from datetime import datetime

sys.path.insert(0, "src")

import pandas as pd
import numpy as np


# ---------------------------------------------------------------------------
# 1. 扩展股票池（15只，覆盖大/中/小盘 + 多行业）
# ---------------------------------------------------------------------------

STOCKS = [
    # 大盘股（银行/保险/白酒）
    ("sz000001", "平安银行", "大盘-银行"),
    ("sh600036", "招商银行", "大盘-银行"),
    ("sh601318", "中国平安", "大盘-保险"),
    ("sh600519", "贵州茅台", "大盘-白酒"),
    # 中盘股
    ("sz000858", "五粮液", "中盘-白酒"),
    ("sz002475", "立讯精密", "中盘-电子"),
    ("sh600276", "恒瑞医药", "中盘-医药"),
    ("sz000333", "美的集团", "中盘-家电"),
    # 科技/新能源
    ("sz300750", "宁德时代", "大盘-新能源"),
    ("sh601012", "隆基绿能", "中盘-新能源"),
    ("sz002594", "比亚迪", "大盘-汽车"),
    # 周期/资源
    ("sh601899", "紫金矿业", "中盘-矿业"),
    ("sh600028", "中国石化", "大盘-石化"),
    # 中小盘
    ("sz002230", "科大飞", "中小盘-AI"),
    ("sh688981", "中芯国际", "大盘-半导体"),
]

START_DATE = "20230601"
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


# ---------------------------------------------------------------------------
# 2. 验证 + bottleneck 收集
# ---------------------------------------------------------------------------

def run_gauge_report(df: pd.DataFrame) -> dict:
    from newchan.a_topology import gauge_equivalence_report
    return gauge_equivalence_report(df, tau=0.0)


def collect_bottlenecks(report: dict) -> list[dict]:
    """从 gauge report 提取所有 transition 的 bottleneck 数据。"""
    rows = []
    for tr in report.get("transitions", []):
        d = tr["delta"]
        rows.append({
            "source": tr["source"],
            "target": tr["target"],
            "bottleneck": d["bottleneck_distance"],
            "stroke_diff": d["stroke_count_diff"],
            "segment_diff": d["segment_count_diff"],
            "center_diff": d["center_count_diff"],
            "beta1_tau_diff": d["beta1_tau_diff"],
            "barcode_preserved": tr["strong_preserved"]["barcode_bottleneck"],
            "n_centers_preserved": tr["strong_preserved"]["n_centers"],
            "beta1_preserved": tr["strong_preserved"]["beta1_tau"],
        })
    return rows


# ---------------------------------------------------------------------------
# 3. ε 校准分析
# ---------------------------------------------------------------------------

def calibrate_epsilon(all_bottlenecks: list[dict]) -> dict:
    """从 bottleneck 分布推导 ε 阈值建议。

    策略：
    - ε = 0 时，任何非零 bottleneck 都判定为"不一致"
    - 合理的 ε 应该让"结构等价但数值微扰"的 transition 通过
    - 从数据分布推导：取非零 bottleneck 的分位数
    """
    bns = [r["bottleneck"] for r in all_bottlenecks]
    nonzero = [b for b in bns if b > 0]

    result = {
        "total_transitions": len(bns),
        "zero_bottleneck": sum(1 for b in bns if b == 0),
        "nonzero_bottleneck": len(nonzero),
        "pass_rate_eps0": sum(1 for b in bns if b == 0) / len(bns) if bns else 0,
    }

    if nonzero:
        arr = np.array(nonzero)
        result["nonzero_stats"] = {
            "min": float(np.min(arr)),
            "max": float(np.max(arr)),
            "mean": float(np.mean(arr)),
            "median": float(np.median(arr)),
            "std": float(np.std(arr)),
            "p25": float(np.percentile(arr, 25)),
            "p75": float(np.percentile(arr, 75)),
            "p90": float(np.percentile(arr, 90)),
            "p95": float(np.percentile(arr, 95)),
        }
        # ε 建议：取 p75 作为宽松阈值，p90 作为严格阈值
        result["epsilon_suggestions"] = {
            "strict": float(np.percentile(arr, 50)),
            "moderate": float(np.percentile(arr, 75)),
            "lenient": float(np.percentile(arr, 90)),
        }
        # 各 ε 下的通过率
        for label, eps in result["epsilon_suggestions"].items():
            passed = sum(1 for b in bns if b <= eps)
            result[f"pass_rate_{label}"] = passed / len(bns) if bns else 0
    else:
        result["nonzero_stats"] = None
        result["epsilon_suggestions"] = {"strict": 0.0, "moderate": 0.0, "lenient": 0.0}

    # 按 transition 类型分组分析
    by_pair = {}
    for r in all_bottlenecks:
        pair = f"{r['source']}->{r['target']}"
        by_pair.setdefault(pair, []).append(r["bottleneck"])
    result["by_transition_pair"] = {
        pair: {
            "count": len(vals),
            "zero": sum(1 for v in vals if v == 0),
            "nonzero": sum(1 for v in vals if v > 0),
            "mean": float(np.mean(vals)) if vals else 0,
            "max": float(np.max(vals)) if vals else 0,
        }
        for pair, vals in by_pair.items()
    }

    return result


# ---------------------------------------------------------------------------
# 4. 报告生成
# ---------------------------------------------------------------------------

def generate_report(results: list, calibration: dict, all_bottlenecks: list):
    lines = []
    lines.append("# T1 Barcode Bottleneck ε 校准报告")
    lines.append("")
    lines.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}")
    lines.append(f"谱系引用：203号-3（ε 校准）+ 203号-4（扩展样本验证）")
    lines.append("")

    lines.append("## 数据集")
    lines.append("")
    lines.append(f"- 股票数量：{len(results)}")
    lines.append(f"- 时间范围：{START_DATE} ~ {END_DATE}")
    lines.append(f"- 总 transition 数：{calibration['total_transitions']}")
    lines.append("")

    lines.append("| 股票 | K线数 | T1(barcode) | T3(centers) | T4(beta1) | 强不变量通过率 |")
    lines.append("|------|-------|-------------|-------------|-----------|---------------|")
    for symbol, name, cap, r in results:
        ss = r["strong_invariant_summary"]
        t1 = ss.get("barcode_bottleneck", {})
        t3 = ss.get("n_centers", {})
        t4 = ss.get("beta1_tau", {})
        tp = sum(v.get("preserved", 0) for v in ss.values())
        tt = sum(v.get("total", 0) for v in ss.values())
        rate = tp / tt if tt > 0 else 0.0
        lines.append(f"| {symbol} {name} | {r['n_bars']} | {t1.get('preserved',0)}/{t1.get('total',0)} "
                     f"| {t3.get('preserved',0)}/{t3.get('total',0)} | {t4.get('preserved',0)}/{t4.get('total',0)} "
                     f"| {rate:.1%} |")
    lines.append("")

    lines.append("## Bottleneck 分布统计")
    lines.append("")
    lines.append(f"- 零 bottleneck（完全一致）：{calibration['zero_bottleneck']}/{calibration['total_transitions']} "
                 f"({calibration['pass_rate_eps0']:.1%})")
    lines.append(f"- 非零 bottleneck：{calibration['nonzero_bottleneck']}/{calibration['total_transitions']}")
    if calibration["nonzero_stats"]:
        ns = calibration["nonzero_stats"]
        lines.append(f"- 非零统计：min={ns['min']:.4f}, max={ns['max']:.4f}, "
                     f"mean={ns['mean']:.4f}, median={ns['median']:.4f}, std={ns['std']:.4f}")
        lines.append(f"- 分位数：p25={ns['p25']:.4f}, p75={ns['p75']:.4f}, p90={ns['p90']:.4f}, p95={ns['p95']:.4f}")
    lines.append("")

    lines.append("## ε 阈值建议")
    lines.append("")
    if calibration["epsilon_suggestions"]:
        es = calibration["epsilon_suggestions"]
        lines.append(f"| 策略 | ε 值 | T1 通过率 |")
        lines.append(f"|------|------|----------|")
        for label in ("strict", "moderate", "lenient"):
            rate = calibration.get(f"pass_rate_{label}", 0)
            lines.append(f"| {label} | {es[label]:.4f} | {rate:.1%} |")
    lines.append("")

    lines.append("## 按 Transition 类型分组")
    lines.append("")
    lines.append("| 模式对 | 总数 | 零bottleneck | 非零 | 平均 | 最大 |")
    lines.append("|--------|------|-------------|------|------|------|")
    for pair, stats in calibration["by_transition_pair"].items():
        lines.append(f"| {pair} | {stats['count']} | {stats['zero']} | {stats['nonzero']} "
                     f"| {stats['mean']:.4f} | {stats['max']:.4f} |")
    lines.append("")

    lines.append("## 逐股 Bottleneck 详情")
    lines.append("")
    lines.append("| 股票 | wide→strict | wide→new | strict→new |")
    lines.append("|------|-------------|----------|------------|")
    for symbol, name, cap, r in results:
        bns = {}
        for tr in r.get("transitions", []):
            pair = f"{tr['source']}→{tr['target']}"
            bns[pair] = tr["delta"]["bottleneck_distance"]
        ws = bns.get("wide→strict", "N/A")
        wn = bns.get("wide→new", "N/A")
        sn = bns.get("strict→new", "N/A")
        fmt = lambda v: f"{v:.4f}" if isinstance(v, float) else v
        lines.append(f"| {symbol} {name} | {fmt(ws)} | {fmt(wn)} | {fmt(sn)} |")
    lines.append("")

    lines.append("## 结论")
    lines.append("")
    if calibration["nonzero_stats"]:
        es = calibration["epsilon_suggestions"]
        lines.append(f"1. 非零 bottleneck 的中位数为 {calibration['nonzero_stats']['median']:.4f}，"
                     f"建议 strict ε = {es['strict']:.4f}")
        lines.append(f"2. moderate ε = {es['moderate']:.4f} 可将 T1 通过率提升至 "
                     f"{calibration.get('pass_rate_moderate', 0):.1%}")
        lines.append(f"3. lenient ε = {es['lenient']:.4f} 可将 T1 通过率提升至 "
                     f"{calibration.get('pass_rate_lenient', 0):.1%}")
    else:
        lines.append("所有 bottleneck 均为零——ε = 0 已足够。")
    lines.append("")

    path = ".chanlun/review-results/t1-epsilon-calibration-report.md"
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"  报告已写入 {path}")


# ---------------------------------------------------------------------------
# 5. Main
# ---------------------------------------------------------------------------

def main():
    print("=" * 60)
    print("T1 Barcode Bottleneck ε 校准（203号-3/4）")
    print("=" * 60)

    datasets = []
    print(f"\n[Phase 1] 获取 {len(STOCKS)} 只 A 股日线数据...")
    for symbol, name, cap in STOCKS:
        df = fetch_stock(symbol, name)
        if df is not None and len(df) >= 100:
            datasets.append((symbol, name, cap, df))

    print(f"\n[Phase 2] 运行 gauge_equivalence_report（共 {len(datasets)} 组）...")
    results = []
    all_bottlenecks = []
    for symbol, name, cap, df in datasets:
        print(f"\n  >>> {symbol} {name} ({len(df)} bars)...")
        try:
            r = run_gauge_report(df)
            r["n_bars"] = len(df)
            results.append((symbol, name, cap, r))
            bns = collect_bottlenecks(r)
            for b in bns:
                b["symbol"] = symbol
                b["name"] = name
            all_bottlenecks.extend(bns)
            ss = r["strong_invariant_summary"]
            parts = [f"{k}={v['preserved']}/{v['total']}" for k, v in ss.items()]
            print(f"      强不变量: {' '.join(parts)}")
        except Exception as e:
            print(f"      [ERR] {e}")
            traceback.print_exc()

    print(f"\n[Phase 3] ε 校准分析...")
    calibration = calibrate_epsilon(all_bottlenecks)
    print(f"  总 transitions: {calibration['total_transitions']}")
    print(f"  零 bottleneck: {calibration['zero_bottleneck']} ({calibration['pass_rate_eps0']:.1%})")
    if calibration["nonzero_stats"]:
        ns = calibration["nonzero_stats"]
        print(f"  非零 bottleneck: min={ns['min']:.4f}, max={ns['max']:.4f}, median={ns['median']:.4f}")
        es = calibration["epsilon_suggestions"]
        print(f"  ε 建议: strict={es['strict']:.4f}, moderate={es['moderate']:.4f}, lenient={es['lenient']:.4f}")

    print(f"\n[Phase 4] 生成报告...")
    generate_report(results, calibration, all_bottlenecks)
    print("\n[DONE]")


if __name__ == "__main__":
    main()
