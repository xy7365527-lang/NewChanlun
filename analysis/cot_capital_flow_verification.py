"""COT 净持仓 → 资本流转流量假说验证（L2/L3 真实数据）。

接续 capital_flow_ontology.md §9.4/§9.6 的否定性结论：成交量流量 proxy
`sign(Δr)×min(Vol)` 不领先价格（§9.4 被否证），**修正方向 = 改用 COT 净持仓**
（真持仓量变化，而非成交量换手）。本脚本对这一修正做 L2 验证。

═══════════════════════════════════════════════════════════════════════
核心可证伪命题（§2.1 共线风险的直接检验）
═══════════════════════════════════════════════════════════════════════
COT 净持仓 ΔNet 对残差/价格方向是否有**信息增量**，取决于领先/滞后符号：

  - ΔNet 领先价格（互相关峰值在 lag>0 / Granger ΔNet→Δr 显著）
        → COT 携带价格之外的方向信息（假说支持，L2）。
  - ΔNet 滞后价格（峰值在 lag<0 / Granger Δr→ΔNet 显著）
        → COT 是价格的滞后函数（Managed Money 趋势跟随）= 同义反复，
          信息增量趋零（假说否证，L2，缩小有效域）。

这与 §9.4 否证成交量 proxy 同构：高相关本身无意义（构造性/共线性），
**符号化的领先/滞后判定**才是信息增量的判据。

═══════════════════════════════════════════════════════════════════════
零前视纪律（auto-memory project_zero_lookahead_backtest）
═══════════════════════════════════════════════════════════════════════
COT report_date 是周二 as-of 快照，周五发布。本脚本的领先/滞后检验用 as-of
日期对齐残差，衡量的是 **COT 的信息内容**（持仓在周二时点已反映什么）。
可交易性须在领先结论上再加 ≥1 周滞后（周五才拿到数据）——故只有当
ΔNet 领先残差 ≥2 周时，信号才在扣除发布延迟后仍可交易。报告显式标注。

认识论等级：L2（真实数据、单标的可证伪）；4 标的 × 2 残差跨品种 ≈ L3。

用法：PYTHONPATH=src .venv/bin/python analysis/cot_capital_flow_verification.py
"""

from __future__ import annotations

import json
import math
import warnings
from pathlib import Path

import numpy as np
import pandas as pd

from cot_data import fetch_all

warnings.filterwarnings("ignore")  # statsmodels 小样本 RuntimeWarning（已知，结果有效）

_DATA = Path(__file__).resolve().parent / "data_cache"
_EUR_WEIGHT = 0.576  # EUR 在 DXY 的权重（verify_cross_national_closure Part B 回归系数）

# 三个 regime 转折点（任务指定）+ 两个补充（user_trading_direction）。
_REGIMES: list[tuple[str, str]] = [
    ("2022-03-15", "首次加息（COT 周二）"),
    ("2022-09-27", "美元/利率见顶区（COT 周二）"),
    ("2024-09-17", "美联储首次降息（COT 周二）"),
]


# ════════════════════════════════════════════════════════════
# 数据层：小文件残差/价格 → 日度收盘 Series
# ════════════════════════════════════════════════════════════


def _load_daily_close(fname: str) -> pd.Series:
    """加载 {fname}.json（dict: dates/closes），降采样为日度收盘 Series（tz-naive）。

    1h databento（tz-aware）与 yfinance 日线统一为按自然日取最后一个收盘。
    """
    d = json.load(open(_DATA / f"{fname}.json"))
    dates = pd.to_datetime(d["dates"], utc=True, format="mixed").tz_convert(None)
    s = pd.Series(d["closes"], index=dates, dtype="float64")
    s = s[s > 0].sort_index()
    daily = s.groupby(s.index.normalize()).last()  # 每自然日末收盘
    daily.index = pd.DatetimeIndex(daily.index)
    return daily


def _build_residual_series() -> dict[str, pd.Series]:
    """构造日度残差/价格序列字典。

    - dx_resid = log(DX) − 0.576·log(EURUSD)  跨结算尺美元残差（§1.1，33.8% DX方差）
    - omega    = log(GC) − log(CL)            金油比/空转剥削率（482号）
    - eurusd, gc, cl, es: 各标的 log 价（单标的共线性检验用）
    """
    dx = np.log(_load_daily_close("dx_1d_yf_2000"))
    eur = np.log(_load_daily_close("eurusd_1h_databento"))
    gc = np.log(_load_daily_close("gc_1h_databento"))
    cl = np.log(_load_daily_close("cl_1h_databento"))
    es = np.log(_load_daily_close("es_1h_databento"))

    # 残差需 DX 与 EURUSD 共同日期
    common = dx.index.intersection(eur.index)
    dx_resid = (dx.reindex(common) - _EUR_WEIGHT * eur.reindex(common)).dropna()

    common2 = gc.index.intersection(cl.index)
    omega = (gc.reindex(common2) - cl.reindex(common2)).dropna()

    return {
        "dx_resid": dx_resid,
        "omega": omega,
        "eurusd": eur,
        "gc": gc,
        "cl": cl,
        "es": es,
    }


def _align_to_cot(cot: list[tuple[str, float]], series: pd.Series) -> pd.DataFrame:
    """把日度残差/价格对齐到 COT 周二 as-of 日期（merge_asof backward，≤7d 容差）。

    返回 DataFrame[date, net, level]，level = 周二（或之前最近交易日）的残差/价格收盘。
    """
    cdf = pd.DataFrame(cot, columns=["date", "net"])
    cdf["date"] = pd.to_datetime(cdf["date"])
    cdf = cdf.sort_values("date").reset_index(drop=True)

    sdf = pd.DataFrame({"date": series.index, "level": series.to_numpy()})
    sdf = sdf.sort_values("date").reset_index(drop=True)

    merged = pd.merge_asof(
        cdf, sdf, on="date", direction="backward",
        tolerance=pd.Timedelta(days=7),
    ).dropna(subset=["level"]).reset_index(drop=True)
    return merged


# ════════════════════════════════════════════════════════════
# 验证 A：滞后互相关（ΔNet vs Δlevel，符号化领先/滞后判定）
# ════════════════════════════════════════════════════════════


def _zscore(x: np.ndarray) -> np.ndarray:
    s = x.std()
    return (x - x.mean()) / s if s > 0 else x - x.mean()


def lagged_xcorr(d_net: np.ndarray, d_lvl: np.ndarray, max_lag: int = 6) -> dict:
    """corr(ΔNet[t], Δlevel[t+lag])。lag>0 = ΔNet 领先 level（COT 有信息增量）。

    返回 {lag: corr} + 峰值 lag/corr + lag0 corr。
    """
    zn, zl = _zscore(d_net), _zscore(d_lvl)
    out: dict[int, float] = {}
    for lag in range(-max_lag, max_lag + 1):
        if lag == 0:
            c = float(np.corrcoef(zn, zl)[0, 1])
        elif lag > 0:
            c = float(np.corrcoef(zn[:-lag], zl[lag:])[0, 1])
        else:
            c = float(np.corrcoef(zn[-lag:], zl[:lag])[0, 1])
        out[lag] = c
    peak_lag = max(out, key=lambda k: abs(out[k]))
    return {"profile": out, "peak_lag": peak_lag, "peak_corr": out[peak_lag],
            "lag0": out[0]}


# ════════════════════════════════════════════════════════════
# 验证 B：Granger 因果双向
# ════════════════════════════════════════════════════════════


def granger_bidirectional(d_net: np.ndarray, d_lvl: np.ndarray,
                          max_lag: int = 4) -> dict:
    """双向 Granger。返回每方向各 lag 的最小 p 值。

    ΔNet→Δlvl：检验 net 变化是否 Granger-导致 level 变化（COT 领先）。
    Δlvl→ΔNet：检验 level 变化是否 Granger-导致 net 变化（COT 滞后/趋势跟随）。
    statsmodels grangercausalitytests(data[[y,x]]) 检验 x 是否导致 y。
    """
    from statsmodels.tsa.stattools import grangercausalitytests

    def _min_p(y: np.ndarray, x: np.ndarray) -> tuple[float, int]:
        data = np.column_stack([y, x])
        res = grangercausalitytests(data, maxlag=max_lag, verbose=False)
        best_p, best_l = 1.0, 0
        for lag in range(1, max_lag + 1):
            p = res[lag][0]["ssr_ftest"][1]
            if p < best_p:
                best_p, best_l = p, lag
        return best_p, best_l

    p_nl, l_nl = _min_p(d_lvl, d_net)   # x=ΔNet 导致 y=Δlvl
    p_ln, l_ln = _min_p(d_net, d_lvl)   # x=Δlvl 导致 y=ΔNet
    return {"net_leads": (p_nl, l_nl), "lvl_leads": (p_ln, l_ln)}


def _verdict(xc: dict, gr: dict) -> str:
    """综合互相关峰值符号 + Granger 双向 p 值的领先/滞后裁决。"""
    peak_lag, peak_corr = xc["peak_lag"], xc["peak_corr"]
    p_nl = gr["net_leads"][0]
    p_ln = gr["lvl_leads"][0]
    net_lead = peak_lag > 0 and p_nl < 0.05
    lvl_lead = peak_lag < 0 and p_ln < 0.05
    if net_lead and not lvl_lead:
        return "✅ COT 领先（信息增量，假说支持）"
    if lvl_lead and not net_lead:
        return "❌ COT 滞后价格（趋势跟随，同义反复，假说否证）"
    if net_lead and lvl_lead:
        return "⚠ 双向反馈（部分领先，需更长样本分离）"
    if abs(peak_corr) < 0.1:
        return "○ 无显著关系（COT 与该残差近独立）"
    return f"○ 同步/弱（峰值lag={peak_lag}，Granger 双向均不显著）"


# ════════════════════════════════════════════════════════════
# 验证 C：regime 转折点 COT 行为放大镜
# ════════════════════════════════════════════════════════════


def regime_lens(name: str, merged: pd.DataFrame, win_weeks: int = 8) -> None:
    print(f"\n  [{name}] regime 窗口 COT 净持仓轨迹（±{win_weeks}周）")
    net = merged["net"].to_numpy()
    lvl = merged["level"].to_numpy()
    dates = merged["date"]
    for ds, label in _REGIMES:
        rd = pd.to_datetime(ds)
        # 最近的 COT 周二索引
        idx = int((dates - rd).abs().idxmin())
        days = abs((dates.iloc[idx] - rd).days)
        if days > 10:
            print(f"    {ds} {label}: COT 无邻近周（差{days}d，跳过——样本未覆盖）")
            continue
        lo, hi = max(0, idx - win_weeks), min(len(net) - 1, idx + win_weeks)
        seg_net = net[lo:hi + 1]
        seg_lvl = lvl[lo:hi + 1]
        # net 在窗口内的拐点（极值）相对 regime 中心的周偏移
        net_peak_off = int(np.argmax(np.abs(seg_net - seg_net.mean()))) - (idx - lo)
        lvl_peak_off = int(np.argmax(np.abs(seg_lvl - seg_lvl.mean()))) - (idx - lo)
        d_net_at = net[idx] - net[max(0, idx - 1)]
        print(f"    {ds} {label} (COT {dates.iloc[idx].date()}, ±{days}d):")
        print(f"        net@regime={net[idx]:>10,.0f}  ΔNet={d_net_at:>+9,.0f}  "
              f"net极值偏移={net_peak_off:+d}周  level极值偏移={lvl_peak_off:+d}周")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════


def run_pair(sym: str, cot: list[tuple[str, float]], series: pd.Series,
             series_name: str) -> dict:
    """单个 (COT标的, 残差/价格) 配对的完整验证。"""
    merged = _align_to_cot(cot, series)
    n = len(merged)
    d_net = np.diff(merged["net"].to_numpy())
    d_lvl = np.diff(merged["level"].to_numpy())

    print("\n" + "─" * 72)
    print(f"  COT[{sym}]  ×  {series_name}   (对齐后 {n} 周, Δ序列 {len(d_net)})")
    print("─" * 72)

    xc = lagged_xcorr(d_net, d_lvl)
    print("  滞后互相关 corr(ΔNet[t], Δlevel[t+lag])  [lag>0 = COT 领先]:")
    prof = xc["profile"]
    row = "  ".join(f"{lag:+d}:{prof[lag]:+.3f}" for lag in sorted(prof))
    print(f"    {row}")
    print(f"    峰值 lag={xc['peak_lag']:+d} corr={xc['peak_corr']:+.3f} | "
          f"lag0={xc['lag0']:+.3f}")

    gr = granger_bidirectional(d_net, d_lvl)
    print(f"  Granger: ΔNet→Δlvl(COT领先) p={gr['net_leads'][0]:.4f}(lag{gr['net_leads'][1]})"
          f" | Δlvl→ΔNet(COT滞后) p={gr['lvl_leads'][0]:.4f}(lag{gr['lvl_leads'][1]})")

    verdict = _verdict(xc, gr)
    print(f"  ▶ 裁决：{verdict}")

    regime_lens(f"{sym}×{series_name}", merged, win_weeks=8)
    return {"sym": sym, "series": series_name, "n": n, "xc": xc, "gr": gr,
            "verdict": verdict}


def main() -> None:
    print("#" * 72)
    print("# COT 净持仓 → 资本流转流量假说验证（接续 §9.6 修正）")
    print("#" * 72)

    cot = fetch_all()  # {6e,gc,cl,es: [(date,net)]}
    ser = _build_residual_series()

    for s in ("6e", "gc", "cl", "es"):
        arr = cot[s]
        print(f"\n  COT[{s}]: {len(arr)} 周  {arr[0][0]} → {arr[-1][0]}")

    print("\n" + "═" * 72)
    print("【验证 1】COT vs 自身价格 — 共线性/趋势跟随根本检验（§2.1）")
    print("═" * 72)
    results = []
    results.append(run_pair("6e", cot["6e"], ser["eurusd"], "EURUSD(log)"))
    results.append(run_pair("gc", cot["gc"], ser["gc"], "GC(log)"))
    results.append(run_pair("cl", cot["cl"], ser["cl"], "CL(log)"))
    results.append(run_pair("es", cot["es"], ser["es"], "ES(log)"))

    print("\n" + "═" * 72)
    print("【验证 2】COT vs 跨结算尺残差 — 资本流转信号（任务核心）")
    print("═" * 72)
    # 6E COT（欧元投机方向）→ DX 跨结算尺残差
    results.append(run_pair("6e", cot["6e"], ser["dx_resid"], "DX残差"))
    # 金油 COT 相对配置 → ω=GC/CL（空转剥削率）
    # 构造 GC−CL 净持仓配置差（标准化后），但 net 量级不同需各自标准化。
    gc_cdf = pd.DataFrame(cot["gc"], columns=["date", "gc_net"])
    cl_cdf = pd.DataFrame(cot["cl"], columns=["date", "cl_net"])
    rot = gc_cdf.merge(cl_cdf, on="date")
    rot["date"] = pd.to_datetime(rot["date"])
    # 金油配置差 = z(gc_net) − z(cl_net)（投机者相对偏向黄金 vs 原油）
    rot["rot"] = _zscore(rot["gc_net"].to_numpy()) - _zscore(rot["cl_net"].to_numpy())
    rot_series = [(d.strftime("%Y-%m-%d"), v) for d, v in zip(rot["date"], rot["rot"])]
    results.append(run_pair("gc-cl轮动", rot_series, ser["omega"], "ω=GC/CL残差"))
    # ES COT → ES 价格（空转/P 顶点方向）
    results.append(run_pair("es", cot["es"], ser["es"], "ES(log) [空转复核]"))

    print("\n" + "═" * 72)
    print("【汇总裁决表】")
    print("═" * 72)
    print(f"  {'COT标的':<12}{'对象':<18}{'n周':>5}{'峰值lag':>8}{'峰值corr':>9}  裁决")
    for r in results:
        print(f"  {r['sym']:<12}{r['series']:<18}{r['n']:>5}"
              f"{r['xc']['peak_lag']:>+8d}{r['xc']['peak_corr']:>+9.3f}  {r['verdict']}")

    print("\n" + "═" * 72)
    print("验证完成。零前视提醒：领先结论须 −1 周（周五发布）方为可交易。")
    print("═" * 72)


if __name__ == "__main__":
    main()
