"""gauge 真实数据扩展验证——5只A股日线。

第三轮验证：扩展品种覆盖，分析低保持率不变量的失败模式。
"""

import sys
import os
import json
import traceback

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from newchan.a_topology import gauge_equivalence_report


# ---------------------------------------------------------------------------
# 数据获取
# ---------------------------------------------------------------------------

STOCKS = [
    ("sh600519", "贵州茅台", "大盘股-白酒"),
    ("sz000333", "美的集团", "大盘股-家电"),
    ("sh601012", "隆基绿能", "中盘股-新能源"),
    ("sz002594", "比亚迪", "大盘股-新能源车"),
    ("sh688981", "中芯国际", "大盘股-半导体"),
]


def fetch_stock_data(symbol: str) -> pd.DataFrame:
    """用 akshare 获取 A 股日线数据（前复权），至少 3 年。"""
    import akshare as ak

    # akshare stock_zh_a_hist 需要纯数字代码
    code = symbol[2:]  # 去掉 sh/sz 前缀
    df = ak.stock_zh_a_hist(
        symbol=code,
        period="daily",
        start_date="20210101",
        end_date="20260226",
        adjust="qfq",
    )
    # 标准化列名
    df = df.rename(columns={
        "日期": "date",
        "开盘": "open",
        "最高": "high",
        "最低": "low",
        "收盘": "close",
        "成交量": "volume",
    })
    df["date"] = pd.to_datetime(df["date"])
    df = df.set_index("date")
    df = df[["open", "high", "low", "close"]].astype(float)
    return df


def fetch_stock_data_yfinance(symbol: str) -> pd.DataFrame:
    """Fallback: 用 yfinance 获取美股数据。"""
    import yfinance as yf

    mapping = {
        "sh600519": "600519.SS",
        "sz000333": "000333.SZ",
        "sh601012": "601012.SS",
        "sz002594": "002594.SZ",
        "sh688981": "688981.SS",
    }
    ticker = mapping.get(symbol, symbol)
    df = yf.download(ticker, start="2021-01-01", end="2026-02-26", auto_adjust=True)
    if isinstance(df.columns, pd.MultiIndex):
        df.columns = df.columns.get_level_values(0)
    df = df.rename(columns={
        "Open": "open", "High": "high", "Low": "low", "Close": "close",
    })
    df = df[["open", "high", "low", "close"]].astype(float)
    return df


# ---------------------------------------------------------------------------
# 分析低保持率不变量的失败模式
# ---------------------------------------------------------------------------

def analyze_failure_modes(report: dict, label: str) -> dict:
    """分析 center_zd_zg_pairs 和 barcode_bottleneck 的失败模式。"""
    analysis = {"label": label, "transitions": []}

    for t in report["transitions"]:
        pair = f"{t['source']} -> {t['target']}"
        strong = t["strong_preserved"]
        weak = t["weak_preserved"]

        # center_zd_zg_pairs 失败分析
        czp_preserved = weak.get("center_zd_zg_pairs", False)
        bb_preserved = weak.get("barcode_bottleneck", False)

        delta = t["delta"]
        analysis["transitions"].append({
            "pair": pair,
            "center_zd_zg_pairs_preserved": czp_preserved,
            "barcode_bottleneck_preserved": bb_preserved,
            "center_count_diff": delta["center_count_diff"],
            "bottleneck_distance": delta["bottleneck_distance"],
            "stroke_diff": delta["stroke_count_diff"],
            "segment_diff": delta["segment_count_diff"],
            "beta1_tau_diff": delta["beta1_tau_diff"],
            "strong": strong,
        })

    return analysis


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    print("=" * 60)
    print("gauge 真实数据扩展验证——第三轮（5只A股）")
    print("=" * 60)
    print()

    results = []
    failure_analyses = []

    for symbol, name, category in STOCKS:
        print(f"--- {symbol} {name} ({category}) ---")

        # 获取数据
        df = None
        source = "akshare"
        try:
            df = fetch_stock_data(symbol)
            print(f"  akshare: {len(df)} 根K线, {df.index[0].date()} ~ {df.index[-1].date()}")
        except Exception as e:
            print(f"  akshare 失败: {e}")
            try:
                df = fetch_stock_data_yfinance(symbol)
                source = "yfinance"
                print(f"  yfinance fallback: {len(df)} 根K线")
            except Exception as e2:
                print(f"  yfinance 也失败: {e2}")
                results.append({
                    "symbol": symbol, "name": name, "category": category,
                    "error": str(e), "n_bars": 0,
                })
                continue

        if df is None or len(df) < 100:
            print(f"  数据不足，跳过")
            results.append({
                "symbol": symbol, "name": name, "category": category,
                "error": "数据不足", "n_bars": len(df) if df is not None else 0,
            })
            continue

        # 运行 gauge_equivalence_report
        try:
            report = gauge_equivalence_report(df, modes=("wide", "strict", "new"))
            print(f"  gauge report 完成")

            # 提取关键指标
            strong_summary = report["strong_invariant_summary"]
            tier_weighted = report["tier_weighted_rate"]

            # T8 结果
            t8 = report.get("t8_divergence_topology", {})
            t8_summary = t8.get("summary", {}) if isinstance(t8, dict) else {}
            t8_passed = t8_summary.get("w1_passed", 0)
            t8_total = t8_summary.get("conclusive", 0)
            t8_inconclusive = t8_summary.get("inconclusive", 0)
            t8_consensus = t8_summary.get("macd_w1_consensus", 0)

            # 弱不变量保持率
            weak_preserved_count = 0
            weak_total = 0
            for t in report["transitions"]:
                for k, v in t["weak_preserved"].items():
                    weak_total += 1
                    if v:
                        weak_preserved_count += 1

            gauge_rate = weak_preserved_count / weak_total if weak_total > 0 else 0.0

            result = {
                "symbol": symbol,
                "name": name,
                "category": category,
                "source": source,
                "n_bars": len(df),
                "date_range": f"{df.index[0].date()} ~ {df.index[-1].date()}",
                "strong_summary": {
                    k: {"rate": v["rate"], "preserved": v["preserved"], "total": v["total"]}
                    for k, v in strong_summary.items()
                },
                "tier_weighted": tier_weighted,
                "t8_passed": t8_passed,
                "t8_total": t8_total,
                "t8_inconclusive": t8_inconclusive,
                "t8_consensus": t8_consensus,
                "gauge_rate": round(gauge_rate, 2),
                "transitions": report["transitions"],
            }
            results.append(result)

            # 失败模式分析
            fa = analyze_failure_modes(report, f"{symbol} {name}")
            failure_analyses.append(fa)

            # 打印摘要
            print(f"  Tier加权通过率: {tier_weighted['overall_weighted']:.1%}")
            for k, v in strong_summary.items():
                print(f"    {k}: {v['preserved']}/{v['total']} = {v['rate']:.1%}")
            print(f"  T8: {t8_passed}/{t8_total} passed, {t8_inconclusive} inconclusive, {t8_consensus} MACD-W1 consensus")
            print(f"  弱不变量保持率(gauge): {gauge_rate:.2f}")
            print()

        except Exception as e:
            print(f"  gauge report 失败: {e}")
            traceback.print_exc()
            results.append({
                "symbol": symbol, "name": name, "category": category,
                "error": str(e), "n_bars": len(df),
            })
            continue

    # ---------------------------------------------------------------------------
    # 失败模式汇总
    # ---------------------------------------------------------------------------
    print("=" * 60)
    print("低保持率不变量失败模式分析")
    print("=" * 60)
    print()

    czp_fail_patterns = []
    bb_fail_patterns = []

    for fa in failure_analyses:
        for t in fa["transitions"]:
            if not t["center_zd_zg_pairs_preserved"]:
                czp_fail_patterns.append({
                    "label": fa["label"],
                    "pair": t["pair"],
                    "center_count_diff": t["center_count_diff"],
                    "bottleneck_distance": t["bottleneck_distance"],
                    "stroke_diff": t["stroke_diff"],
                    "segment_diff": t["segment_diff"],
                })
            if not t["barcode_bottleneck_preserved"]:
                bb_fail_patterns.append({
                    "label": fa["label"],
                    "pair": t["pair"],
                    "center_count_diff": t["center_count_diff"],
                    "bottleneck_distance": t["bottleneck_distance"],
                    "stroke_diff": t["stroke_diff"],
                    "segment_diff": t["segment_diff"],
                })

    print(f"center_zd_zg_pairs 失败: {len(czp_fail_patterns)} 例")
    if czp_fail_patterns:
        # 分类：center_count_diff == 0 但 pairs 不同 vs center_count_diff != 0
        same_count = [p for p in czp_fail_patterns if p["center_count_diff"] == 0]
        diff_count = [p for p in czp_fail_patterns if p["center_count_diff"] != 0]
        print(f"  中枢数相同但区间不同: {len(same_count)} 例")
        print(f"  中枢数不同: {len(diff_count)} 例")
        if same_count:
            bns = [p["bottleneck_distance"] for p in same_count]
            print(f"  同数中枢的 bottleneck: mean={np.mean(bns):.4f}, max={np.max(bns):.4f}")
        if diff_count:
            diffs = [abs(p["center_count_diff"]) for p in diff_count]
            print(f"  中枢数差异: mean={np.mean(diffs):.1f}, max={np.max(diffs)}")

    print()
    print(f"barcode_bottleneck 失败: {len(bb_fail_patterns)} 例")
    if bb_fail_patterns:
        bns = [p["bottleneck_distance"] for p in bb_fail_patterns]
        print(f"  bottleneck 距离: mean={np.mean(bns):.4f}, max={np.max(bns):.4f}, min={np.min(bns):.4f}")
        # 按 mode pair 分组
        by_pair = {}
        for p in bb_fail_patterns:
            pair = p["pair"]
            if pair not in by_pair:
                by_pair[pair] = []
            by_pair[pair].append(p["bottleneck_distance"])
        for pair, vals in sorted(by_pair.items()):
            print(f"  {pair}: mean={np.mean(vals):.4f}, n={len(vals)}")

    # ---------------------------------------------------------------------------
    # 输出 JSON 供报告更新使用
    # ---------------------------------------------------------------------------
    output_path = os.path.join(
        os.path.dirname(__file__), "gauge_expand_results.json",
    )
    serializable_results = []
    for r in results:
        sr = {k: v for k, v in r.items() if k != "transitions"}
        serializable_results.append(sr)

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(serializable_results, f, ensure_ascii=False, indent=2, default=str)
    print(f"\n结果已保存: {output_path}")

    # 打印汇总表
    print()
    print("=" * 60)
    print("汇总")
    print("=" * 60)
    print()
    print(f"{'品种':<20} {'K线数':>6} {'T8':>8} {'Tier加权':>8} {'gauge':>6}")
    print("-" * 60)
    for r in results:
        if "error" in r and "strong_summary" not in r:
            print(f"{r['symbol']} {r['name']:<12} {'ERROR':>6}")
            continue
        t8_str = f"{r['t8_passed']}/{r['t8_total']}"
        tw = r["tier_weighted"]["overall_weighted"] if "tier_weighted" in r else 0
        print(f"{r['symbol']} {r['name']:<12} {r['n_bars']:>6} {t8_str:>8} {tw:>7.1%} {r['gauge_rate']:>6.2f}")


if __name__ == "__main__":
    main()
