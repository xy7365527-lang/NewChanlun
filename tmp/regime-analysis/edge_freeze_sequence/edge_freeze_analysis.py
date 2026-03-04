"""
边冻结进入/退出序列分析（v3——相对 vol 比率法）
330号谱系"断裂位点→边冻结模式"预测的直接检验

核心方法变更（v2 → v3）:
329号的发现是：同步压缩天内 OIL边 vol=正常天1.5x，非OIL边 vol=正常天0.5x。
关键不是绝对 z-score，而是**边间 vol 比率的分异**。
一条边 vol 上升而其他边 vol 下降 = 断裂信号。

v3 方法：
1. 计算每条边在 entry_phase 的 vol / baseline_vol 比率
2. 如果某节点的边 vol_ratio > 1（上升）而不含该节点的边 vol_ratio < 1（下降） → 该节点是断裂位点
3. 允许"全图激活"（所有边都上升）的情况——此时用 max_ratio 边来推断断裂节点
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd

warnings.filterwarnings("ignore")

# ────────────────────────────────────────────────────────────
# 1. 数据获取
# ────────────────────────────────────────────────────────────

def fetch_yfinance(ticker: str, start: str, end: str) -> pd.Series:
    import yfinance as yf
    df = yf.download(ticker, start=start, end=end, progress=False, auto_adjust=True)
    if df.empty:
        raise ValueError(f"yfinance 返回空数据: {ticker} [{start}, {end}]")
    close = df["Close"]
    if isinstance(close, pd.DataFrame):
        close = close.iloc[:, 0]
    close.index = close.index.tz_localize(None)
    return close.rename(ticker)


def fetch_fred_wti(start: str, end: str) -> pd.Series:
    url = (
        f"https://fred.stlouisfed.org/graph/fredgraph.csv"
        f"?id=DCOILWTICO&cosd={start}&coed={end}"
    )
    df = pd.read_csv(url)
    df.columns = ["date", "value"]
    df["date"] = pd.to_datetime(df["date"])
    df = df.set_index("date")
    df["value"] = pd.to_numeric(df["value"], errors="coerce")
    df = df.dropna()
    return df["value"].rename("WTI")


def get_three_node_data(start: str, end: str) -> pd.DataFrame:
    dxy = fetch_yfinance("DX-Y.NYB", start, end)
    spx = fetch_yfinance("^GSPC", start, end)
    wti = fetch_fred_wti(start, end)
    return pd.DataFrame({"USD": dxy, "Equity": spx, "Commodity": wti}).dropna()


def get_four_node_data(start: str, end: str) -> pd.DataFrame:
    au = fetch_yfinance("GC=F", start, end)
    dxy = fetch_yfinance("DX-Y.NYB", start, end)
    spx = fetch_yfinance("^GSPC", start, end)
    wti = fetch_fred_wti(start, end)
    return pd.DataFrame({"Au": au, "USD": dxy, "Equity": spx, "Commodity": wti}).dropna()


# ────────────────────────────────────────────────────────────
# 2. 边计算
# ────────────────────────────────────────────────────────────

def compute_edge_log_returns(df: pd.DataFrame, nodes: list[str]) -> pd.DataFrame:
    edges = {}
    for i, a in enumerate(nodes):
        for b in nodes[i + 1:]:
            ratio = np.log(df[a] / df[b])
            ret = ratio.diff()
            edges[f"{a}/{b}"] = ret
    return pd.DataFrame(edges).dropna()


def rolling_vol(edge_returns: pd.DataFrame, window: int = 20) -> pd.DataFrame:
    return edge_returns.rolling(window).std().dropna()


# ────────────────────────────────────────────────────────────
# 3. 分析核心
# ────────────────────────────────────────────────────────────

def analyze_regime_v3(
    data: pd.DataFrame,
    nodes: list[str],
    regime_start: str,
    regime_end: str,
    baseline_end: str | None = None,
    baseline_days: int = 60,
    entry_phase_days: int = 30,
    post_days: int = 30,
    regime_name: str = "",
) -> dict:
    regime_start_dt = pd.Timestamp(regime_start)
    regime_end_dt = pd.Timestamp(regime_end)
    baseline_end_dt = pd.Timestamp(baseline_end) if baseline_end else regime_start_dt

    edge_returns = compute_edge_log_returns(data, nodes)
    vol = rolling_vol(edge_returns, window=20)
    valid_dates = vol.index

    # ── 窗口定义 ──
    bl_candidates = valid_dates[valid_dates < baseline_end_dt]
    baseline_dates = bl_candidates[-baseline_days:] if len(bl_candidates) >= baseline_days else bl_candidates

    regime_dates = valid_dates[(valid_dates >= regime_start_dt) & (valid_dates <= regime_end_dt)]
    if len(regime_dates) == 0:
        return {"error": f"No data in [{regime_start}, {regime_end}]"}

    entry_n = min(entry_phase_days, len(regime_dates))
    entry_dates = regime_dates[:entry_n]
    sustained_dates = regime_dates[entry_n:] if len(regime_dates) > entry_n else pd.DatetimeIndex([])

    pre_candidates = valid_dates[valid_dates < regime_start_dt]
    pre_dates = pre_candidates[-30:] if len(pre_candidates) >= 30 else pre_candidates

    post_candidates = valid_dates[valid_dates > regime_end_dt]
    post_dates_ = post_candidates[:post_days] if len(post_candidates) >= post_days else post_candidates

    # ── baseline 统计 ──
    bl_means, bl_stds = {}, {}
    for e in vol.columns:
        bv = vol[e].reindex(baseline_dates).dropna()
        bl_means[e] = float(bv.mean())
        bl_stds[e] = float(bv.std())

    # ── vol ratio = phase_mean / baseline_mean ──
    def phase_vol_ratios(dates):
        ratios = {}
        z_scores = {}
        raw_means = {}
        for e in vol.columns:
            pv = vol[e].reindex(dates).dropna()
            pm = float(pv.mean()) if len(pv) > 0 else np.nan
            bm = bl_means[e]
            bs = bl_stds[e]
            ratios[e] = pm / bm if bm > 0 and not np.isnan(pm) else np.nan
            z_scores[e] = (pm - bm) / bs if bs > 0 and not np.isnan(pm) else 0.0
            raw_means[e] = pm
        return ratios, z_scores, raw_means

    entry_ratios, entry_zscores, entry_means = phase_vol_ratios(entry_dates)
    sustained_ratios, sustained_zscores, sustained_means = phase_vol_ratios(sustained_dates) if len(sustained_dates) > 0 else ({}, {}, {})
    regime_ratios, regime_zscores, regime_means = phase_vol_ratios(regime_dates)
    pre_ratios, pre_zscores, pre_means = phase_vol_ratios(pre_dates)
    post_ratios, post_zscores, post_means = phase_vol_ratios(post_dates_)

    # ── 日度 z-score 分类（用于序列发现） ──
    def daily_classification(dates):
        """返回每条边每天的 z-score + state"""
        records = []
        for date in dates:
            row = {"date": str(date.date())}
            for e in vol.columns:
                v = vol[e].get(date, np.nan)
                bm = bl_means[e]
                bs = bl_stds[e]
                z = (v - bm) / bs if bs > 0 and not np.isnan(v) else 0.0
                if z > 2.0:
                    st = "riot"
                elif z < -1.0:
                    st = "freeze"
                else:
                    st = "normal"
                row[f"{e}_z"] = round(z, 2)
                row[f"{e}_state"] = st
            records.append(row)
        return records

    entry_daily = daily_classification(entry_dates)
    pre_daily = daily_classification(pre_dates)

    # ── 第一条暴动边的时序 ──
    first_riot = {}
    # 先检查 pre
    for row in pre_daily:
        for e in vol.columns:
            if row[f"{e}_state"] == "riot" and e not in first_riot:
                first_riot[e] = row["date"]
    # 再检查 entry
    for row in entry_daily:
        for e in vol.columns:
            if row[f"{e}_state"] == "riot" and e not in first_riot:
                first_riot[e] = row["date"]

    riot_sequence = sorted(first_riot.items(), key=lambda x: x[1])

    # ── 第一条冻结边的时序（entry only） ──
    first_freeze = {}
    for row in entry_daily:
        for e in vol.columns:
            if row[f"{e}_state"] == "freeze" and e not in first_freeze:
                first_freeze[e] = row["date"]
    freeze_sequence = sorted(first_freeze.items(), key=lambda x: x[1])

    # ── 断裂节点推断——基于 vol ratio 分异 ──
    # 对每个节点 N，计算：
    #   avg_ratio_with_N = 含N边的 entry vol_ratio 均值
    #   avg_ratio_without_N = 不含N边的 entry vol_ratio 均值
    #   分异度 = avg_ratio_with_N - avg_ratio_without_N
    # 分异度最大的节点 = 断裂位点候选

    node_divergence = {}
    for n in nodes:
        edges_with = [e for e in vol.columns if n in e.split("/")]
        edges_without = [e for e in vol.columns if n not in e.split("/")]
        with_ratios = [entry_ratios[e] for e in edges_with if not np.isnan(entry_ratios.get(e, np.nan))]
        without_ratios = [entry_ratios[e] for e in edges_without if not np.isnan(entry_ratios.get(e, np.nan))]
        avg_with = np.mean(with_ratios) if with_ratios else np.nan
        avg_without = np.mean(without_ratios) if without_ratios else np.nan
        divergence = (avg_with - avg_without) if not np.isnan(avg_with) and not np.isnan(avg_without) else 0.0
        node_divergence[n] = {
            "avg_ratio_with": round(float(avg_with), 4) if not np.isnan(avg_with) else None,
            "avg_ratio_without": round(float(avg_without), 4) if not np.isnan(avg_without) else None,
            "divergence": round(float(divergence), 4),
            "edges_with": edges_with,
            "edges_without": edges_without,
        }

    # 断裂节点 = divergence 最大
    best_node = max(node_divergence, key=lambda n: node_divergence[n]["divergence"])
    best_div = node_divergence[best_node]["divergence"]

    # 330号预测检验：含断裂节点边 vol 上升 + 不含断裂节点边 vol 下降
    edges_with_best = node_divergence[best_node]["edges_with"]
    edges_without_best = node_divergence[best_node]["edges_without"]
    with_ratios_vals = [entry_ratios[e] for e in edges_with_best]
    without_ratios_vals = [entry_ratios[e] for e in edges_without_best]

    with_above_1 = all(r > 1.0 for r in with_ratios_vals if not np.isnan(r))
    without_below_1 = all(r < 1.0 for r in without_ratios_vals if not np.isnan(r))
    with_mean = np.mean(with_ratios_vals)
    without_mean = np.mean(without_ratios_vals)

    # sustained phase 的 vol ratio 用于区分：
    # - 进入暴动→冻结持续（标准冻结模式）
    # - 进入暴动→暴动持续（全局激活模式）
    if sustained_ratios:
        sustained_with = np.mean([sustained_ratios[e] for e in edges_with_best if not np.isnan(sustained_ratios.get(e, np.nan))])
        sustained_without = np.mean([sustained_ratios[e] for e in edges_without_best if not np.isnan(sustained_ratios.get(e, np.nan))])
    else:
        sustained_with = np.nan
        sustained_without = np.nan

    # 判定
    if best_div > 0.1 and with_above_1 and without_below_1:
        verdict = "STRONG_CONFIRM"
    elif best_div > 0.05 and with_mean > without_mean:
        verdict = "DIRECTIONAL_CONFIRM"
    elif best_div > 0.0:
        verdict = "WEAK_SIGNAL"
    elif best_div <= 0:
        verdict = "NO_DIVERGENCE"
    else:
        verdict = "INCONCLUSIVE"

    prediction_check = {
        "rupture_node": best_node,
        "divergence": round(float(best_div), 4),
        "edges_with_rupture": {
            "edges": edges_with_best,
            "entry_vol_ratios": {e: round(entry_ratios[e], 4) for e in edges_with_best},
            "mean_ratio": round(float(with_mean), 4),
            "all_above_1": with_above_1,
        },
        "edges_without_rupture": {
            "edges": edges_without_best,
            "entry_vol_ratios": {e: round(entry_ratios[e], 4) for e in edges_without_best},
            "mean_ratio": round(float(without_mean), 4),
            "all_below_1": without_below_1,
        },
        "sustained_with_mean_ratio": round(float(sustained_with), 4) if not np.isnan(sustained_with) else None,
        "sustained_without_mean_ratio": round(float(sustained_without), 4) if not np.isnan(sustained_without) else None,
    }

    # ── 退出恢复序列 ──
    exit_recovery = []
    for e in vol.columns:
        pv = vol[e].reindex(post_dates_).dropna()
        bl_m = bl_means[e]
        bl_s = bl_stds[e]
        cls = pd.Series("normal", index=pv.index)
        cls[pv > bl_m + 2 * bl_s] = "riot"
        cls[pv < bl_m - 1 * bl_s] = "freeze"
        normal_days = cls[cls == "normal"].index
        exit_recovery.append({
            "edge": e,
            "first_normal_date": str(normal_days[0].date()) if len(normal_days) > 0 else None,
            "post_normal_fraction": float((cls == "normal").mean()) if len(cls) > 0 else None,
        })
    exit_recovery.sort(key=lambda x: x["first_normal_date"] or "9999")

    result = {
        "regime_name": regime_name,
        "regime_start": regime_start,
        "regime_end": regime_end,
        "baseline_end": baseline_end or regime_start,
        "nodes": nodes,
        "edges": list(vol.columns),
        "regime_trading_days": len(regime_dates),
        "baseline_trading_days": len(baseline_dates),
        "entry_phase_days": len(entry_dates),
        "sustained_phase_days": len(sustained_dates),
        "baseline_means": {e: round(v, 6) for e, v in bl_means.items()},
        "baseline_stds": {e: round(v, 6) for e, v in bl_stds.items()},
        "entry_vol_ratios": {e: round(v, 4) for e, v in entry_ratios.items()},
        "entry_vol_zscores": {e: round(v, 2) for e, v in entry_zscores.items()},
        "sustained_vol_ratios": {e: round(v, 4) for e, v in sustained_ratios.items()} if sustained_ratios else None,
        "regime_full_vol_ratios": {e: round(v, 4) for e, v in regime_ratios.items()},
        "post_vol_ratios": {e: round(v, 4) for e, v in post_ratios.items()},
        "node_divergence": node_divergence,
        "sequence_analysis": {
            "riot_sequence": [{"edge": e, "date": d} for e, d in riot_sequence],
            "freeze_sequence": [{"edge": e, "date": d} for e, d in freeze_sequence],
            "first_rioting_edge": riot_sequence[0][0] if riot_sequence else None,
            "first_riot_date": riot_sequence[0][1] if riot_sequence else None,
            "first_freezing_edge": freeze_sequence[0][0] if freeze_sequence else None,
            "first_freeze_date": freeze_sequence[0][1] if freeze_sequence else None,
            "inferred_rupture_node": best_node,
            "prediction_check": prediction_check,
            "prediction_verdict": verdict,
            "exit_recovery_sequence": exit_recovery,
        },
    }
    return result


# ────────────────────────────────────────────────────────────
# 4. 日度明细
# ────────────────────────────────────────────────────────────

def daily_vol_table(
    data: pd.DataFrame,
    nodes: list[str],
    regime_start: str,
    regime_end: str,
    baseline_end: str | None = None,
    baseline_days: int = 60,
    post_days: int = 30,
) -> pd.DataFrame:
    regime_start_dt = pd.Timestamp(regime_start)
    regime_end_dt = pd.Timestamp(regime_end)
    baseline_end_dt = pd.Timestamp(baseline_end) if baseline_end else regime_start_dt

    edge_returns = compute_edge_log_returns(data, nodes)
    vol = rolling_vol(edge_returns, window=20)
    valid_dates = vol.index

    bl_candidates = valid_dates[valid_dates < baseline_end_dt]
    baseline_dates = bl_candidates[-baseline_days:] if len(bl_candidates) >= baseline_days else bl_candidates

    bl_means, bl_stds = {}, {}
    for e in vol.columns:
        bv = vol[e].reindex(baseline_dates).dropna()
        bl_means[e] = float(bv.mean())
        bl_stds[e] = float(bv.std())

    regime_dates = valid_dates[(valid_dates >= regime_start_dt) & (valid_dates <= regime_end_dt)]
    pre_dates = valid_dates[valid_dates < regime_start_dt][-30:]
    post_candidates = valid_dates[valid_dates > regime_end_dt]
    post_dates_ = post_candidates[:post_days]

    if len(baseline_dates) > 0 and len(pre_dates) > 0:
        start = min(baseline_dates[0], pre_dates[0])
    elif len(baseline_dates) > 0:
        start = baseline_dates[0]
    elif len(pre_dates) > 0:
        start = pre_dates[0]
    else:
        start = regime_dates[0]
    end_ = post_dates_[-1] if len(post_dates_) > 0 else regime_dates[-1]
    all_dates = valid_dates[(valid_dates >= start) & (valid_dates <= end_)]

    rows = []
    for d in all_dates:
        if d in regime_dates.values:
            w = "regime"
        elif len(pre_dates) > 0 and d >= pre_dates[0] and d < regime_start_dt:
            w = "pre"
        elif len(post_dates_) > 0 and d in post_dates_.values:
            w = "post"
        else:
            w = "baseline"
        row = {"date": str(d.date()), "window": w}
        for e in vol.columns:
            v = vol[e].get(d, np.nan)
            bm = bl_means[e]
            bs = bl_stds[e]
            z = (v - bm) / bs if bs > 0 and not np.isnan(v) else 0.0
            ratio = v / bm if bm > 0 and not np.isnan(v) else np.nan
            st = "riot" if z > 2.0 else ("freeze" if z < -1.0 else "normal")
            row[f"{e}_vol"] = round(float(v), 6) if not np.isnan(v) else None
            row[f"{e}_z"] = round(float(z), 2) if not np.isnan(v) else None
            row[f"{e}_ratio"] = round(float(ratio), 3) if not np.isnan(ratio) else None
            row[f"{e}_state"] = st
        rows.append(row)
    return pd.DataFrame(rows)


# ────────────────────────────────────────────────────────────
# 5. 主程序
# ────────────────────────────────────────────────────────────

def main():
    output_dir = Path(__file__).parent
    results = {}

    # ── 体制 1: 1987 ──
    print("=" * 70)
    print("体制 1: 1987 (三节点, 22天)")
    print("=" * 70)
    try:
        data_1987 = get_three_node_data("1986-06-01", "1987-04-30")
        print(f"  数据: {data_1987.index[0].date()} ~ {data_1987.index[-1].date()}")

        r = analyze_regime_v3(
            data_1987,
            nodes=["USD", "Equity", "Commodity"],
            regime_start="1987-01-06",
            regime_end="1987-02-04",
            regime_name="1987_pre_crash",
        )
        results["1987"] = r

        print(f"  entry vol ratios: {r['entry_vol_ratios']}")
        sa = r["sequence_analysis"]
        print(f"  节点分异度:")
        for n, nd in r["node_divergence"].items():
            print(f"    {n}: with={nd['avg_ratio_with']}, without={nd['avg_ratio_without']}, div={nd['divergence']}")
        print(f"  推断断裂节点: {sa['inferred_rupture_node']} (div={sa['prediction_check']['divergence']})")
        print(f"  riot序列: {sa['riot_sequence']}")
        print(f"  freeze序列: {sa['freeze_sequence']}")
        print(f"  判定: {sa['prediction_verdict']}")

        daily = daily_vol_table(data_1987, ["USD", "Equity", "Commodity"], "1987-01-06", "1987-02-04")
        daily.to_csv(output_dir / "daily_vol_1987.csv", index=False)

    except Exception as e:
        import traceback; traceback.print_exc()
        results["1987"] = {"error": str(e)}

    # ── 体制 2: 1990-91 ──
    print()
    print("=" * 70)
    print("体制 2: 1990-91 (三节点, 219天)")
    print("=" * 70)
    try:
        data_1990 = get_three_node_data("1990-04-01", "1991-12-31")
        print(f"  数据: {data_1990.index[0].date()} ~ {data_1990.index[-1].date()}")

        r = analyze_regime_v3(
            data_1990,
            nodes=["USD", "Equity", "Commodity"],
            regime_start="1990-10-23",
            regime_end="1991-09-06",
            regime_name="1990_91_gulf_war",
        )
        results["1990_91"] = r

        print(f"  entry vol ratios: {r['entry_vol_ratios']}")
        if r.get("sustained_vol_ratios"):
            print(f"  sustained vol ratios: {r['sustained_vol_ratios']}")
        sa = r["sequence_analysis"]
        print(f"  节点分异度:")
        for n, nd in r["node_divergence"].items():
            print(f"    {n}: with={nd['avg_ratio_with']}, without={nd['avg_ratio_without']}, div={nd['divergence']}")
        print(f"  推断断裂节点: {sa['inferred_rupture_node']} (div={sa['prediction_check']['divergence']})")
        print(f"  riot序列: {sa['riot_sequence']}")
        print(f"  freeze序列: {sa['freeze_sequence']}")
        print(f"  判定: {sa['prediction_verdict']}")

        daily = daily_vol_table(data_1990, ["USD", "Equity", "Commodity"], "1990-10-23", "1991-09-06")
        daily.to_csv(output_dir / "daily_vol_1990_91.csv", index=False)

    except Exception as e:
        import traceback; traceback.print_exc()
        results["1990_91"] = {"error": str(e)}

    # ── 体制 3: 2020-21 ──
    print()
    print("=" * 70)
    print("体制 3: 2020-21 (四节点, 263天)")
    print("=" * 70)
    try:
        data_2020 = get_four_node_data("2019-06-01", "2021-08-31")
        print(f"  数据: {data_2020.index[0].date()} ~ {data_2020.index[-1].date()}")

        r = analyze_regime_v3(
            data_2020,
            nodes=["Au", "USD", "Equity", "Commodity"],
            regime_start="2020-04-03",
            regime_end="2021-04-23",
            baseline_end="2020-02-15",
            regime_name="2020_21_covid_qe",
        )
        results["2020_21"] = r

        print(f"  baseline截止: 2020-02-15")
        print(f"  entry vol ratios: {r['entry_vol_ratios']}")
        if r.get("sustained_vol_ratios"):
            print(f"  sustained vol ratios: {r['sustained_vol_ratios']}")
        sa = r["sequence_analysis"]
        print(f"  节点分异度:")
        for n, nd in r["node_divergence"].items():
            print(f"    {n}: with={nd['avg_ratio_with']}, without={nd['avg_ratio_without']}, div={nd['divergence']}")
        print(f"  推断断裂节点: {sa['inferred_rupture_node']} (div={sa['prediction_check']['divergence']})")
        print(f"  riot序列: {sa['riot_sequence']}")
        print(f"  freeze序列: {sa['freeze_sequence']}")
        print(f"  判定: {sa['prediction_verdict']}")

        daily = daily_vol_table(
            data_2020, ["Au", "USD", "Equity", "Commodity"],
            "2020-04-03", "2021-04-23", baseline_end="2020-02-15"
        )
        daily.to_csv(output_dir / "daily_vol_2020_21.csv", index=False)

    except Exception as e:
        import traceback; traceback.print_exc()
        results["2020_21"] = {"error": str(e)}

    # ── 汇总 ──
    print()
    print("=" * 70)
    print("330号预测汇总检验")
    print("=" * 70)

    rupture_nodes = {}
    verdicts = {}
    for key in ["1987", "1990_91", "2020_21"]:
        r = results.get(key, {})
        if "error" in r:
            verdicts[key] = "ERROR"
            rupture_nodes[key] = None
            continue
        sa = r["sequence_analysis"]
        verdicts[key] = sa["prediction_verdict"]
        rupture_nodes[key] = sa["inferred_rupture_node"]
        pc = sa["prediction_check"]
        print(f"  {key}:")
        print(f"    断裂节点: {pc['rupture_node']} (divergence={pc['divergence']})")
        print(f"    含{pc['rupture_node']}边: ratio_mean={pc['edges_with_rupture']['mean_ratio']}, all>1={pc['edges_with_rupture']['all_above_1']}")
        print(f"    不含{pc['rupture_node']}边: ratio_mean={pc['edges_without_rupture']['mean_ratio']}, all<1={pc['edges_without_rupture']['all_below_1']}")
        print(f"    判定: {sa['prediction_verdict']}")

    valid_nodes = [n for n in rupture_nodes.values() if n is not None]
    unique = set(valid_nodes)

    strong = sum(1 for v in verdicts.values() if v in ("STRONG_CONFIRM", "DIRECTIONAL_CONFIRM"))
    all_different = len(unique) == len(valid_nodes) and len(valid_nodes) >= 2

    if strong >= 2 and all_different:
        overall = "CONFIRMED: 不同体制的断裂节点分异 + 边冻结/暴动方向符合预测"
    elif strong >= 2 and not all_different:
        overall = "PARTIAL: 方向符合但断裂节点未分异（相同节点）"
    elif strong >= 1 and all_different:
        overall = "PARTIAL: 至少一个体制方向符合 + 断裂节点分异"
    elif all(v == "NO_DIVERGENCE" for v in verdicts.values() if v != "ERROR"):
        overall = "DENIED: 没有体制显示边间分异"
    else:
        overall = "INCONCLUSIVE: 信号混合"

    results["overall_verdict"] = {
        "verdicts": verdicts,
        "rupture_nodes": rupture_nodes,
        "unique_rupture_nodes": list(unique),
        "conclusion": overall,
    }
    print(f"\n  总结论: {overall}")

    with open(output_dir / "edge_freeze_results.json", "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2, default=str)
    print(f"\n结果写入 {output_dir / 'edge_freeze_results.json'}")


if __name__ == "__main__":
    main()
