"""
FRED + Yahoo Finance K4 eff.dim cluster 分析
=============================================

谱系引用：329号（K4折叠内在性检验最终结算）、330号（K4拓扑与资本循环映射）
认识论等级：L2（真实数据，可否证）

数据源：
  - Au：Yahoo Finance GC=F（gold futures，2000-08-30 起）
         注：FRED GOLDAMGBD228NLBM（LBMA London Fix）已于 2022-01-31 从 FRED 删除
         （ICE Benchmark Administration 数据全部移除），无法获取 1986-2000 黄金日线
  - WTI：FRED DCOILWTICO（WTI spot price，USD/barrel，1986-01-02 起）
         通过 Playwright 从 FRED 下载，保存为本地 CSV
  - SPX：Yahoo Finance ^GSPC（S&P 500，1986-01-02 起）
  - DXY：Yahoo Finance DX-Y.NYB（US Dollar Index，1986-01-01 起）

公共窗口：受 GC=F 限制，四节点公共窗口从 ~2000-08-30 起
         WTI+SPX+DXY 三节点可扩展到 1986-01-02 起

计算逻辑完全复制 scripts/effdim_monitor.py：
  - 每节点3条边 = log(price_node / price_peer) 的 diff
  - Rolling 252d 协方差 -> 特征值 -> Shannon entropy -> eff.dim = exp(entropy)
  - 同步压缩 = 四节点 eff.dim 同时 < 各自全样本 5th percentile
  - Cluster = 同步压缩日连续或间隔 <= 5 天
"""

from __future__ import annotations

import json
import sys
import warnings
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd
from scipy.spatial.distance import mahalanobis

warnings.filterwarnings("ignore")

# ============================================================
# 常量
# ============================================================

EFFDIM_WINDOW = 252
PHASE_THRESHOLD_PERCENTILE = 5.0
CLUSTER_GAP_DAYS = 5  # 间隔 <= 5 天归为同一 cluster

# 四节点定义
NODES = {
    "Au": {
        "peers": ["DXY", "SPX", "WTI"],
        "edge_names": ("Au/$", "Au/Equity", "Au/Commodity"),
    },
    "$": {
        "peers": ["Au", "SPX", "WTI"],
        "edge_names": ("$/Au", "$/Equity", "$/Commodity"),
    },
    "Equity": {
        "peers": ["DXY", "Au", "WTI"],
        "edge_names": ("Equity/$", "Equity/Au", "Equity/Commodity"),
    },
    "Commodity": {
        "peers": ["DXY", "Au", "SPX"],
        "edge_names": ("Commodity/$", "Commodity/Au", "Commodity/Equity"),
    },
}


# ============================================================
# 数据类
# ============================================================

@dataclass(frozen=True)
class Cluster:
    """同步压缩 cluster"""
    cluster_id: int
    start_date: str
    end_date: str
    duration_days: int
    n_sync_days: int
    au_effdim_min: float
    dollar_effdim_min: float
    equity_effdim_min: float
    commodity_effdim_min: float
    mahalanobis_mean: float
    mahalanobis_max: float
    historical_context: str


# ============================================================
# 步骤1：下载并对齐四条序列
# ============================================================

def load_wti_from_local_csv(csv_path: Path) -> pd.Series:
    """从本地 CSV 加载 WTI FRED 数据"""
    df = pd.read_csv(csv_path, parse_dates=["observation_date"], index_col="observation_date")
    s = df["DCOILWTICO"].replace(".", np.nan).astype(float)
    s.index = pd.DatetimeIndex(s.index)
    s.name = "WTI"
    return s.dropna()


def download_yahoo_series(ticker: str, start: str) -> pd.Series:
    """从 Yahoo Finance 下载收盘价"""
    import yfinance as yf
    df = yf.download(ticker, start=start, progress=False)
    if df.empty:
        raise RuntimeError(f"Yahoo Finance 返回空数据: {ticker}")
    # yfinance 2.x 返回 MultiIndex columns
    if isinstance(df.columns, pd.MultiIndex):
        close = df["Close"]
        if isinstance(close, pd.DataFrame):
            close = close.iloc[:, 0]
    else:
        close = df["Close"]
    s = close.copy()
    s.index = pd.DatetimeIndex(s.index)
    return s


def download_and_align(out_dir: Path) -> tuple[pd.DataFrame, dict[str, Any]]:
    """下载四条序列并对齐到公共窗口"""
    print("步骤1：下载并对齐数据...")

    # WTI: 从本地 FRED CSV
    wti_csv = out_dir / "wti_fred.csv"
    if not wti_csv.exists():
        raise RuntimeError(
            f"WTI FRED CSV 不存在: {wti_csv}\n"
            "请先通过 Playwright 从 FRED 下载 DCOILWTICO 数据。"
        )
    print("  加载 WTI (FRED DCOILWTICO, 本地 CSV)...")
    wti = load_wti_from_local_csv(wti_csv)
    print(f"    WTI: {wti.index[0].date()} to {wti.index[-1].date()}, {len(wti)} obs")

    # Au: Yahoo Finance GC=F (gold futures)
    print("  下载 Au (Yahoo GC=F, gold futures)...")
    au = download_yahoo_series("GC=F", "2000-01-01")
    print(f"    Au: {au.index[0].date()} to {au.index[-1].date()}, {len(au)} obs")

    # SPX: Yahoo Finance ^GSPC
    print("  下载 SPX (Yahoo ^GSPC)...")
    spx = download_yahoo_series("^GSPC", "1986-01-01")
    print(f"    SPX: {spx.index[0].date()} to {spx.index[-1].date()}, {len(spx)} obs")

    # DXY: Yahoo Finance DX-Y.NYB
    print("  下载 DXY (Yahoo DX-Y.NYB)...")
    dxy = download_yahoo_series("DX-Y.NYB", "1986-01-01")
    print(f"    DXY: {dxy.index[0].date()} to {dxy.index[-1].date()}, {len(dxy)} obs")

    # 对齐到公共窗口（四节点）
    df = pd.DataFrame({"Au": au, "WTI": wti, "SPX": spx, "DXY": dxy})
    df = df.dropna()
    df.index = pd.DatetimeIndex(df.index)

    meta = {
        "raw_counts": {
            "Au_GCF": int(len(au)),
            "WTI_FRED": int(len(wti)),
            "SPX": int(len(spx)),
            "DXY": int(len(dxy)),
        },
        "aligned_start": str(df.index[0].date()),
        "aligned_end": str(df.index[-1].date()),
        "aligned_count": int(len(df)),
        "data_sources": {
            "Au": "Yahoo Finance GC=F (gold futures, NOT LBMA spot)",
            "WTI": "FRED DCOILWTICO (WTI spot price, via Playwright download)",
            "SPX": "Yahoo Finance ^GSPC (S&P 500)",
            "DXY": "Yahoo Finance DX-Y.NYB (US Dollar Index)",
        },
        "note": (
            "FRED GOLDAMGBD228NLBM (LBMA London Fix gold) 于 2022-01-31 被删除 "
            "(ICE Benchmark Administration 数据全部移除)。"
            "黄金用 GC=F 期货替代，公共窗口因此从 ~2000-09 开始而非 1986。"
        ),
    }

    print(f"\n  对齐后公共窗口: {meta['aligned_start']} to {meta['aligned_end']}")
    print(f"  观测数量: {meta['aligned_count']}")
    print(f"  注意: LBMA gold 数据已从 FRED 删除，Au 用 GC=F 替代（窗口从 2000 起）")

    return df, meta


# ============================================================
# 步骤2：计算四节点 eff.dim
# ============================================================

def compute_log_ratio_returns(prices: pd.DataFrame, node_col: str,
                              peer_cols: list[str]) -> pd.DataFrame:
    """计算节点与三个 peer 的 log(price ratio) return"""
    node_price = prices[node_col]
    returns = {}
    for i, peer in enumerate(peer_cols):
        ratio = node_price / prices[peer]
        log_ret = np.log(ratio).diff()
        returns[f"edge_{i}"] = log_ret
    return pd.DataFrame(returns, index=prices.index).dropna()


def compute_effdim_series(returns: pd.DataFrame,
                          window: int = EFFDIM_WINDOW) -> pd.Series:
    """rolling 窗口计算 eff.dim 时间序列"""
    values = []
    dates = []
    arr = returns.values

    for i in range(window, len(arr)):
        w = arr[i - window: i]
        cov = np.cov(w.T)
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 1e-15]
        if len(eigvals) == 0:
            values.append(1.0)
        else:
            p = eigvals / eigvals.sum()
            entropy = -np.sum(p * np.log(p))
            values.append(np.exp(entropy))
        dates.append(returns.index[i])

    return pd.Series(values, index=pd.DatetimeIndex(dates))


def compute_all_effdim(prices: pd.DataFrame) -> dict[str, pd.Series]:
    """计算四节点的 eff.dim 时间序列"""
    print("\n步骤2：计算四节点 eff.dim...")
    result = {}

    node_col_map = {"Au": "Au", "$": "DXY", "Equity": "SPX", "Commodity": "WTI"}

    for node_name, cfg in NODES.items():
        peer_cols = []
        for p in cfg["peers"]:
            if p == "DXY":
                peer_cols.append("DXY")
            elif p == "Au":
                peer_cols.append("Au")
            elif p == "SPX":
                peer_cols.append("SPX")
            elif p == "WTI":
                peer_cols.append("WTI")

        node_col = node_col_map[node_name]
        returns = compute_log_ratio_returns(prices, node_col, peer_cols)
        effdim = compute_effdim_series(returns)
        result[node_name] = effdim
        print(f"  {node_name}: {len(effdim)} eff.dim values, "
              f"range [{effdim.min():.4f}, {effdim.max():.4f}], "
              f"median {effdim.median():.4f}")

    return result


# ============================================================
# 步骤3：识别同步压缩 cluster
# ============================================================

def compute_independent_edge_returns(prices: pd.DataFrame) -> pd.DataFrame:
    """计算3条线性独立边的 return（用于 Mahalanobis 距离）

    K4 有 C(4,2)=6 条边，但只有 N-1=3 条线性独立。
    6 条边的 6x6 协方差矩阵秩 = 3（rank deficient），直接 inv 会失败。
    选择一个节点（Au）的3条边作为独立基：Au/$, Au/Equity, Au/Commodity。

    注意：negative WTI (2020-04-20) 导致 log(Au/WTI) = NaN，
    需要 dropna 去掉该天。
    """
    pairs = [
        ("Au", "DXY", "Au/$"),
        ("Au", "SPX", "Au/Equity"),
        ("Au", "WTI", "Au/Commodity"),
    ]
    result = {}
    for col_a, col_b, name in pairs:
        ratio = prices[col_a] / prices[col_b]
        result[name] = np.log(ratio).diff()
    df = pd.DataFrame(result, index=prices.index).dropna()
    # Remove any remaining inf values (from negative WTI etc.)
    df = df.replace([np.inf, -np.inf], np.nan).dropna()
    return df


def find_sync_compressed_dates(
    effdim_dict: dict[str, pd.Series],
) -> tuple[pd.DatetimeIndex, dict[str, float]]:
    """找到四节点同时 < 各自 5th percentile 的日期"""
    df = pd.DataFrame(effdim_dict)
    df = df.dropna()

    thresholds = {}
    for col in df.columns:
        thresholds[col] = float(np.percentile(df[col].values, PHASE_THRESHOLD_PERCENTILE))

    print(f"\n  全样本 5th percentile 阈值:")
    for node, thresh in thresholds.items():
        print(f"    {node}: {thresh:.6f}")

    mask = pd.Series(True, index=df.index)
    for col in df.columns:
        mask = mask & (df[col] < thresholds[col])

    sync_dates = df.index[mask]
    print(f"\n  同步压缩日数: {len(sync_dates)}")

    return sync_dates, thresholds


def cluster_dates(dates: pd.DatetimeIndex, gap_days: int = CLUSTER_GAP_DAYS) -> list[list]:
    """将日期聚合为 cluster（间隔 <= gap_days 天归为同一 cluster）"""
    if len(dates) == 0:
        return []

    sorted_dates = sorted(dates)
    clusters = [[sorted_dates[0]]]

    for d in sorted_dates[1:]:
        if (d - clusters[-1][-1]).days <= gap_days:
            clusters[-1].append(d)
        else:
            clusters.append([d])

    return clusters


# 历史事件查找表
HISTORICAL_EVENTS = {
    (1986, 1990): "1987 Black Monday; 1989 S&L Crisis; 1990 Gulf War buildup",
    (1990, 1991): "1990-91 Gulf War; US recession",
    (1991, 1994): "Post-Gulf War recovery; 1994 Bond massacre",
    (1994, 1995): "1994 Bond massacre; Mexico Tequila crisis",
    (1995, 1998): "Asian Financial Crisis (1997); LTCM (1998)",
    (1998, 1999): "LTCM collapse; Russian debt crisis; Fed rescue",
    (1999, 2001): "Dot-com bubble peak and burst; 9/11",
    (2001, 2003): "Post-9/11 recession; Iraq War buildup",
    (2003, 2007): "Credit boom; housing bubble inflation",
    (2007, 2009): "2007-09 Global Financial Crisis; Lehman collapse",
    (2008, 2010): "GFC depths; QE1 launch (Nov 2008); recovery start",
    (2010, 2012): "European debt crisis; QE2; US debt ceiling",
    (2012, 2014): "QE3/QE infinity; Taper tantrum (2013)",
    (2014, 2016): "Oil crash 2014-16; China slowdown; Yuan deval (2015)",
    (2016, 2018): "Trump election; tax reform; Fed tightening",
    (2018, 2020): "Trade war; 2018 Q4 selloff; COVID-19 crash (2020)",
    (2020, 2021): "COVID crash + massive QE/fiscal; V-shaped recovery",
    (2021, 2023): "Post-COVID inflation; Fed rate hikes 2022-23",
    (2023, 2026): "Rate plateau; AI boom; geopolitical tensions",
}


def get_historical_context(date: pd.Timestamp) -> str:
    """查找日期对应的历史事件"""
    year = date.year
    for (start_y, end_y), event in HISTORICAL_EVENTS.items():
        if start_y <= year < end_y:
            return event
    return f"Year {year}"


def identify_clusters(effdim_dict: dict[str, pd.Series],
                      prices: pd.DataFrame) -> tuple[list[Cluster], dict]:
    """识别同步压缩 cluster 并计算 Mahalanobis 距离"""
    print("\n步骤3：识别同步压缩 cluster...")

    sync_dates, thresholds = find_sync_compressed_dates(effdim_dict)

    if len(sync_dates) == 0:
        print("  未发现同步压缩日期")
        return [], {
            "thresholds": {k: round(v, 6) for k, v in thresholds.items()},
            "n_sync_days": 0,
            "n_clusters": 0,
            "effdim_window": EFFDIM_WINDOW,
            "phase_threshold_percentile": PHASE_THRESHOLD_PERCENTILE,
            "cluster_gap_days": CLUSTER_GAP_DAYS,
        }

    raw_clusters = cluster_dates(sync_dates)
    print(f"  聚合为 {len(raw_clusters)} 个 cluster")

    effdim_df = pd.DataFrame(effdim_dict).dropna()

    # 步骤4：Mahalanobis 距离交叉验证
    print("\n步骤4：Mahalanobis 距离交叉验证...")
    indep_edge_returns = compute_independent_edge_returns(prices)

    common_dates = effdim_df.index.intersection(indep_edge_returns.index)
    edge_ret_aligned = indep_edge_returns.loc[common_dates]

    global_mean = edge_ret_aligned.mean().values
    global_cov = edge_ret_aligned.cov().values
    cov_rank = np.linalg.matrix_rank(global_cov)
    print(f"  3条独立边协方差矩阵 rank: {cov_rank} (should be 3)")
    try:
        global_cov_inv = np.linalg.inv(global_cov)
    except np.linalg.LinAlgError:
        global_cov_inv = np.linalg.pinv(global_cov)

    # 计算全样本 Mahalanobis 统计量（用于判断 cluster 是否极端）
    all_maha = []
    for idx in common_dates:
        x = edge_ret_aligned.loc[idx].values
        try:
            md = mahalanobis(x, global_mean, global_cov_inv)
            all_maha.append(md)
        except Exception:
            pass
    all_maha = np.array(all_maha)
    maha_p95 = float(np.percentile(all_maha, 95))
    maha_p99 = float(np.percentile(all_maha, 99))
    print(f"  全样本 Mahalanobis: mean={np.mean(all_maha):.2f}, "
          f"p95={maha_p95:.2f}, p99={maha_p99:.2f}")

    clusters = []
    for idx, cl_dates in enumerate(raw_clusters):
        start_d = min(cl_dates)
        end_d = max(cl_dates)

        cl_effdim = effdim_df.loc[effdim_df.index.isin(cl_dates)]
        au_min = float(cl_effdim["Au"].min()) if "Au" in cl_effdim else float("nan")
        dollar_min = float(cl_effdim["$"].min()) if "$" in cl_effdim else float("nan")
        equity_min = float(cl_effdim["Equity"].min()) if "Equity" in cl_effdim else float("nan")
        commodity_min = float(cl_effdim["Commodity"].min()) if "Commodity" in cl_effdim else float("nan")

        maha_values = []
        for d in cl_dates:
            if d in edge_ret_aligned.index:
                x = edge_ret_aligned.loc[d].values
                try:
                    md = mahalanobis(x, global_mean, global_cov_inv)
                    maha_values.append(md)
                except Exception:
                    pass

        maha_mean = float(np.mean(maha_values)) if maha_values else float("nan")
        maha_max = float(np.max(maha_values)) if maha_values else float("nan")

        context = get_historical_context(start_d)

        cluster = Cluster(
            cluster_id=idx + 1,
            start_date=str(start_d.date()),
            end_date=str(end_d.date()),
            duration_days=(end_d - start_d).days + 1,
            n_sync_days=len(cl_dates),
            au_effdim_min=round(au_min, 6),
            dollar_effdim_min=round(dollar_min, 6),
            equity_effdim_min=round(equity_min, 6),
            commodity_effdim_min=round(commodity_min, 6),
            mahalanobis_mean=round(maha_mean, 4),
            mahalanobis_max=round(maha_max, 4),
            historical_context=context,
        )
        clusters.append(cluster)
        print(f"  Cluster #{idx+1}: {cluster.start_date} ~ {cluster.end_date} "
              f"({cluster.duration_days}d, {cluster.n_sync_days} sync days) "
              f"Maha mean={cluster.mahalanobis_mean:.2f}, max={cluster.mahalanobis_max:.2f}")

    stats = {
        "thresholds": {k: round(v, 6) for k, v in thresholds.items()},
        "n_sync_days": int(len(sync_dates)),
        "n_clusters": len(clusters),
        "effdim_window": EFFDIM_WINDOW,
        "phase_threshold_percentile": PHASE_THRESHOLD_PERCENTILE,
        "cluster_gap_days": CLUSTER_GAP_DAYS,
        "mahalanobis_global": {
            "mean": round(float(np.mean(all_maha)), 4),
            "p95": round(maha_p95, 4),
            "p99": round(maha_p99, 4),
        },
    }

    return clusters, stats


# ============================================================
# 输出
# ============================================================

def generate_results(clusters: list[Cluster], stats: dict,
                     data_meta: dict, effdim_dict: dict[str, pd.Series]) -> dict:
    """生成结构化结果"""
    effdim_stats = {}
    for node, series in effdim_dict.items():
        vals = series.values
        effdim_stats[node] = {
            "count": int(len(series)),
            "mean": round(float(series.mean()), 6),
            "std": round(float(series.std()), 6),
            "min": round(float(series.min()), 6),
            "p5": round(float(np.percentile(vals, 5)), 6),
            "p25": round(float(np.percentile(vals, 25)), 6),
            "median": round(float(series.median()), 6),
            "p75": round(float(np.percentile(vals, 75)), 6),
            "p95": round(float(np.percentile(vals, 95)), 6),
            "max": round(float(series.max()), 6),
        }

    return {
        "metadata": {
            "analysis": "FRED WTI spot + Yahoo Finance K4 eff.dim cluster analysis",
            "run_time": datetime.now().isoformat(),
            "data_sources": data_meta.get("data_sources", {}),
            "data_window": {
                "aligned_start": data_meta["aligned_start"],
                "aligned_end": data_meta["aligned_end"],
                "aligned_count": data_meta["aligned_count"],
            },
            "note": data_meta.get("note", ""),
            "epistemological_level": "L2 (real data, falsifiable)",
        },
        "parameters": {
            "effdim_window": EFFDIM_WINDOW,
            "phase_threshold_percentile": PHASE_THRESHOLD_PERCENTILE,
            "cluster_gap_days": CLUSTER_GAP_DAYS,
        },
        "effdim_statistics": effdim_stats,
        "sync_compression": stats,
        "clusters": [asdict(c) for c in clusters],
    }


def generate_report(clusters: list[Cluster], stats: dict,
                    data_meta: dict, effdim_dict: dict[str, pd.Series]) -> str:
    """生成六要素报告"""
    lines = []

    lines.append("# FRED WTI + Yahoo K4 eff.dim 同步压缩 Cluster 分析报告")
    lines.append("")
    lines.append(f"运行时间: {datetime.now().isoformat()}")
    lines.append("认识论等级: **L2**（真实数据，可否证）")
    lines.append("")

    # Data source note
    lines.append("## 数据源说明")
    lines.append("")
    lines.append("FRED GOLDAMGBD228NLBM（LBMA London Fix gold AM price）于 2022-01-31 被 FRED 删除，")
    lines.append("原因是 ICE Benchmark Administration (IBA) 全部数据从 FRED 移除。")
    lines.append("因此本分析无法使用 1986-2000 的黄金现货日线数据。")
    lines.append("")
    lines.append("替代方案：")
    lines.append("- Au: Yahoo Finance GC=F（gold futures），数据从 2000-08-30 起")
    lines.append("- WTI: FRED DCOILWTICO（WTI spot），数据从 1986-01-02 起，通过 Playwright 下载")
    lines.append("- SPX: Yahoo Finance ^GSPC，数据从 1986+ 起")
    lines.append("- DXY: Yahoo Finance DX-Y.NYB，数据从 1986+ 起")
    lines.append("")
    lines.append("公共窗口受 GC=F 限制，从 ~2000-09 起。1986-2000 窗口无法进行四节点分析。")
    lines.append("")

    # 1. 结论
    lines.append("## 1. 结论")
    lines.append("")
    lines.append(f"- 数据窗口: {data_meta['aligned_start']} to {data_meta['aligned_end']}")
    lines.append(f"- 公共观测数: {data_meta['aligned_count']} 交易日")
    lines.append(f"- eff.dim 可计算观测数: ~{data_meta['aligned_count'] - EFFDIM_WINDOW}")
    lines.append(f"- 同步压缩日总数: {stats['n_sync_days']}")
    lines.append(f"- 同步压缩 cluster 总数: **{stats['n_clusters']}**")
    lines.append("")

    if "mahalanobis_global" in stats:
        mg = stats["mahalanobis_global"]
        lines.append(f"- Mahalanobis 全样本统计: mean={mg['mean']:.2f}, p95={mg['p95']:.2f}, p99={mg['p99']:.2f}")
        lines.append("")

    if clusters:
        lines.append("### Cluster 列表")
        lines.append("")
        lines.append("| # | 起始 | 结束 | 日历天 | 同步天 | Au min | $ min | Equity min | Commodity min | Maha mean | Maha max | 历史背景 |")
        lines.append("|---|------|------|--------|--------|--------|-------|------------|---------------|-----------|----------|----------|")
        for c in clusters:
            lines.append(
                f"| {c.cluster_id} | {c.start_date} | {c.end_date} | {c.duration_days} | "
                f"{c.n_sync_days} | {c.au_effdim_min:.4f} | {c.dollar_effdim_min:.4f} | "
                f"{c.equity_effdim_min:.4f} | {c.commodity_effdim_min:.4f} | "
                f"{c.mahalanobis_mean:.2f} | {c.mahalanobis_max:.2f} | {c.historical_context} |"
            )
        lines.append("")
    else:
        lines.append("未发现同步压缩 cluster。")
        lines.append("")

    # eff.dim 阈值
    lines.append("### 全样本 5th Percentile 阈值")
    lines.append("")
    for node, thresh in stats["thresholds"].items():
        lines.append(f"- {node}: {thresh:.6f}")
    lines.append("")

    # eff.dim 统计
    lines.append("### eff.dim 全样本统计")
    lines.append("")
    lines.append("| 节点 | count | mean | std | min | p5 | median | p95 | max |")
    lines.append("|------|-------|------|-----|-----|----|--------|-----|-----|")
    for node, series in effdim_dict.items():
        vals = series.values
        lines.append(
            f"| {node} | {len(series)} | {series.mean():.4f} | {series.std():.4f} | "
            f"{series.min():.4f} | {np.percentile(vals, 5):.4f} | {series.median():.4f} | "
            f"{np.percentile(vals, 95):.4f} | {series.max():.4f} |"
        )
    lines.append("")

    # 2. 定义依据
    lines.append("## 2. 定义依据")
    lines.append("")
    lines.append("- **eff.dim 计算**: 完全复制 `scripts/effdim_monitor.py` 逻辑。每节点3条边 = log(price_node / price_peer) 的 diff。Rolling 252d 协方差矩阵 -> 特征值分解 -> Shannon entropy -> eff.dim = exp(entropy)。")
    lines.append("- **同步压缩**: 329号结算定义——四节点 eff.dim 同时 < 各自全样本 5th percentile。")
    lines.append("- **Cluster 聚合**: 同步压缩日间隔 <= 5 个日历天归为同一 cluster。")
    lines.append("- **Mahalanobis 距离**: 3条线性独立边（Au/$, Au/Equity, Au/Commodity）的联合 return 相对全样本均值和协方差的 Mahalanobis 距离。K4 的 6 条边仅有 N-1=3 条独立，6x6 协方差矩阵 rank=3，直接 inv 会失败。选择一个节点的 3 条边作为独立基。")
    lines.append("- **数据源差异 vs effdim_monitor.py**: WTI 用 FRED 现货替代 CL=F 期货（无展期价差），Au 仍用 GC=F 期货（LBMA 数据不可用）。")
    lines.append("")

    # 3. 边界条件
    lines.append("## 3. 边界条件")
    lines.append("")
    lines.append("以下条件改变会导致 cluster 识别结果变化：")
    lines.append("")
    lines.append("1. **阈值选择**: 5th percentile 是全样本统计量。改为滚动 percentile 或不同百分位（如 10th）会改变 cluster 数量和位置。")
    lines.append("2. **窗口长度**: 252d rolling window 假设年度时间尺度。缩短（126d）增加噪声，延长（504d）平滑短期事件。")
    lines.append("3. **数据源**: GC=F 期货价格包含展期价差和期限结构效应，与现货价格微观结构不同。如果 LBMA 数据可通过其他渠道获取（如 Bloomberg），结果可能不同。")
    lines.append("4. **Gap 天数**: 间隔 <= 5 天是任意选择。改为 0 或 10 会改变 cluster 边界。")
    lines.append("5. **窗口限制**: 因 LBMA 数据删除，1986-2000 窗口无法覆盖。如果该窗口存在同步压缩 cluster，本分析将遗漏。")
    lines.append("")

    # 4. 下游推论
    lines.append("## 4. 下游推论")
    lines.append("")
    lines.append("- 本分析的公共窗口（~2000-2025）与 effdim_monitor.py 相同，因为后者也使用 GC=F 和 CL=F。")
    lines.append("- WTI 从 CL=F 改为 FRED 现货是唯一的数据源差异。如果 cluster 结果与 effdim_monitor.py 一致，说明 WTI 现货 vs 期货的差异不影响同步压缩识别。")
    lines.append("- 如果 cluster 结果显著不同，说明期货展期价差对 eff.dim 有实质影响（需进一步调查）。")
    lines.append("- 1986-2000 窗口的分析需要替代黄金数据源（如 Bloomberg LBMA、Refinitiv）。")
    lines.append("- Mahalanobis 交叉验证：如果 cluster 的 Maha > p95，说明同步压缩与联合 return 极端性一致。")
    lines.append("")

    # 5. 谱系引用
    lines.append("## 5. 谱系引用")
    lines.append("")
    lines.append("- 329号：K4 折叠内在性检验最终结算（同步压缩定义 + 阈值因果性 + QE 制度依赖）")
    lines.append("- 330号：K4 拓扑与资本循环映射（可监控信号定义）")
    lines.append("- 231号：形式化有效域规则（L2 标注要求）")
    lines.append("- FRED 公告：https://news.research.stlouisfed.org/2022/01/ice-benchmark-administration-ltd-iba-data-to-be-removed-from-fred/")
    lines.append("")

    # 6. 影响声明
    lines.append("## 6. 影响声明")
    lines.append("")
    lines.append("- 本分析不修改任何代码或定义。")
    lines.append("- 产出三个新文件写入 `tmp/fred-k4-cluster/`（分析脚本 + JSON 结果 + 本报告）。")
    lines.append("- FRED LBMA gold 数据不可用是一个外部约束（数据源删除），不是分析缺陷。")
    lines.append("- 1986-2000 四节点分析需要替代黄金数据源，本分析无法覆盖。")
    lines.append("- WTI 现货 vs 期货是本分析对 effdim_monitor.py 的唯一数据源改进。")
    lines.append("")

    return "\n".join(lines)


# ============================================================
# Main
# ============================================================

def main() -> int:
    out_dir = Path(__file__).parent

    # 步骤1
    try:
        prices, data_meta = download_and_align(out_dir)
    except Exception as e:
        print(f"\n数据下载/加载失败: {e}")
        return 1

    if len(prices) < EFFDIM_WINDOW + 10:
        print(f"数据不足: 需要 {EFFDIM_WINDOW + 10}+, 实际 {len(prices)}")
        return 1

    # 步骤2
    effdim_dict = compute_all_effdim(prices)

    # 步骤3 + 4
    clusters, stats = identify_clusters(effdim_dict, prices)

    # 生成输出
    print("\n生成输出...")
    results = generate_results(clusters, stats, data_meta, effdim_dict)

    results_path = out_dir / "fred_k4_cluster_results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    print(f"  结果 JSON: {results_path}")

    report = generate_report(clusters, stats, data_meta, effdim_dict)
    report_path = out_dir / "fred_k4_cluster_report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告: {report_path}")

    # 摘要
    print("\n" + "=" * 64)
    print(f"  K4 Cluster 分析完成")
    print(f"  数据窗口: {data_meta['aligned_start']} ~ {data_meta['aligned_end']}")
    print(f"  观测数: {data_meta['aligned_count']}")
    print(f"  同步压缩 cluster: {stats['n_clusters']}")
    if "mahalanobis_global" in stats:
        mg = stats["mahalanobis_global"]
        print(f"  Mahalanobis 全样本: mean={mg['mean']:.2f}, p95={mg['p95']:.2f}, p99={mg['p99']:.2f}")
    print("=" * 64)

    if clusters:
        print("\n  Cluster 摘要:")
        for c in clusters:
            print(f"    #{c.cluster_id}: {c.start_date} ~ {c.end_date} "
                  f"({c.duration_days}d, {c.n_sync_days} sync) "
                  f"| Maha mean={c.mahalanobis_mean:.2f}, max={c.mahalanobis_max:.2f}")
            print(f"      eff.dim min: Au={c.au_effdim_min:.4f} $={c.dollar_effdim_min:.4f} "
                  f"Eq={c.equity_effdim_min:.4f} Cm={c.commodity_effdim_min:.4f}")
            print(f"      {c.historical_context}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
