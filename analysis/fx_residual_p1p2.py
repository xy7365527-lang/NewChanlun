#!/usr/bin/env python3
"""Part 1 + Part 2：残差-金相关性 & 残差变化率对 K4 配置转换的领先性。

残差定义（254/517 号货币层和乐）：
    r = log DX + 0.576·log EURUSD,  EURUSD = 1/usd6e
      = log DX − 0.576·log usd6e
r = DX（6 货币篮子美元强度）中**无法被单条 EUR 腿解释**的部分 = 非欧元货币篮子。

Part 1：corr(Δr, Δlog GC) vs corr(Δlog DX, Δlog GC)，分 regime。
        看残差（非欧元篮子）与金价相关性是否比 DX 与金价更高/更稳定。
Part 2：残差变化率是否**领先** K4 配置(σ_P,σ_C,σ_R) 的 regime 切换。
        Granger 因果 + 滞后相关。σ 复用 k4_config_transition_1min.json 缓存。

⚠ 数据约束（诚实标注，no-workaround）：DX/BRN 仅 2018-12-26 起。
  请求的 regime (2010-2015, 2015-2020, 2020-2025) 中，残差仅 2019+ 可算。
  故 regime 切分在 DX 可得域内重做（2019-2021 / 2021-2023 / 2023-2026），
  并显式声明 2010-2018 残差不存在（非省略）。

认识论等级：L2（真实数据，可证伪）。
"""

from __future__ import annotations

import json
import sys
from datetime import date, datetime, timedelta
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "analysis" / "data_cache"
EUR_WEIGHT = 0.576


def load_close_series(sym: str) -> pd.Series:
    """加载 columnar JSON → pandas Series（UTC datetime index, close）。NaN/非正过滤。"""
    with open(DATA / f"{sym}_1m_databento_10y.json") as f:
        d = json.load(f)
    idx = pd.to_datetime(d["dates"], utc=True)
    c = pd.Series(np.asarray(d["closes"], dtype=float), index=idx, name=sym)
    c = c[(c > 0) & np.isfinite(c)]
    return c


def epoch_day_to_date(ed: int) -> date:
    return date(1970, 1, 1) + timedelta(days=int(ed))


# ════════════════════════════════════════════════════════════
# Part 1：残差-金相关性
# ════════════════════════════════════════════════════════════

REGIMES = [
    ("2019-2021", "2019-01-01", "2021-01-01"),
    ("2021-2023", "2021-01-01", "2023-01-01"),
    ("2023-2026", "2023-01-01", "2026-07-01"),
    ("全 2019-2026", "2019-01-01", "2026-07-01"),
]


def part1() -> dict:
    print("\n" + "=" * 70)
    print("Part 1：残差-金相关性  r = log DX − 0.576·log usd6e")
    print("=" * 70, flush=True)
    dx = load_close_series("dx")
    u6 = load_close_series("usd6e")
    gc = load_close_series("gc")

    # 分钟对齐（三序列交集）
    df = pd.concat([dx.rename("dx"), u6.rename("u6"), gc.rename("gc")],
                   axis=1, join="inner").dropna()
    print(f"  分钟三序列对齐 = {len(df):,} bars  "
          f"{df.index[0].date()} → {df.index[-1].date()}", flush=True)

    ldx = np.log(df["dx"])
    lu6 = np.log(df["u6"])
    lgc = np.log(df["gc"])
    r = ldx - EUR_WEIGHT * lu6   # = log DX + 0.576·log EURUSD

    # ── 日度重采样（每 UTC 日末值），换算对数收益 ──
    daily = pd.DataFrame({"ldx": ldx, "r": r, "lgc": lgc})
    daily_last = daily.resample("1D").last().dropna()
    d_dx = daily_last["ldx"].diff()
    d_r = daily_last["r"].diff()
    d_gc = daily_last["lgc"].diff()
    dd = pd.DataFrame({"d_dx": d_dx, "d_r": d_r, "d_gc": d_gc}).dropna()

    # ── 分钟收益（稳健性对照，受微观结构/非同步噪声主导）──
    m = pd.DataFrame({
        "d_dx": ldx.diff(), "d_r": r.diff(), "d_gc": lgc.diff(),
    }).dropna()

    results = {"daily": [], "minute": [], "n_daily": len(dd), "n_minute": len(m),
               "date_range": [str(df.index[0].date()), str(df.index[-1].date())]}

    def regime_corr(frame: pd.DataFrame, label: str, lo: str, hi: str) -> dict:
        sub = frame[(frame.index >= lo) & (frame.index < hi)]
        if len(sub) < 30:
            return {"regime": label, "n": len(sub), "corr_r_gc": None,
                    "corr_dx_gc": None}
        cr = float(np.corrcoef(sub["d_r"], sub["d_gc"])[0, 1])
        cdx = float(np.corrcoef(sub["d_dx"], sub["d_gc"])[0, 1])
        return {"regime": label, "n": int(len(sub)),
                "corr_r_gc": cr, "corr_dx_gc": cdx, "delta": cr - cdx}

    print("\n  ── 日度对数收益相关性（主指标）──")
    print(f"  {'regime':14s} {'n':>6s} {'corr(Δr,ΔlnGC)':>16s} "
          f"{'corr(ΔDX,ΔlnGC)':>16s} {'残差更负?':>10s}")
    for label, lo, hi in REGIMES:
        res = regime_corr(dd, label, lo, hi)
        results["daily"].append(res)
        if res["corr_r_gc"] is not None:
            better = "是" if res["corr_r_gc"] < res["corr_dx_gc"] else "否"
            print(f"  {label:14s} {res['n']:>6d} {res['corr_r_gc']:>16.4f} "
                  f"{res['corr_dx_gc']:>16.4f} {better:>10s}")

    print("\n  ── 分钟对数收益相关性（噪声对照）──")
    for label, lo, hi in REGIMES:
        res = regime_corr(m, label, lo, hi)
        results["minute"].append(res)
        if res["corr_r_gc"] is not None:
            print(f"  {label:14s} {res['n']:>7d} {res['corr_r_gc']:>16.4f} "
                  f"{res['corr_dx_gc']:>16.4f}")

    # 稳定性：各 regime corr 的标准差（越小越稳定）
    daily_valid = [x for x in results["daily"][:-1] if x["corr_r_gc"] is not None]
    if len(daily_valid) >= 2:
        std_r = float(np.std([x["corr_r_gc"] for x in daily_valid]))
        std_dx = float(np.std([x["corr_dx_gc"] for x in daily_valid]))
        results["stability"] = {"std_corr_r_gc": std_r, "std_corr_dx_gc": std_dx,
                                "residual_more_stable": std_r < std_dx}
        print(f"\n  ── regime 间稳定性（跨 3 个 regime 的 corr 标准差，越小越稳定）──")
        print(f"  残差-金 corr std = {std_r:.4f}  vs  DX-金 corr std = {std_dx:.4f}  "
              f"→ 残差{'更稳定' if std_r < std_dx else '不更稳定'}")

    return results


# ════════════════════════════════════════════════════════════
# Part 2：残差变化率领先 K4 配置切换
# ════════════════════════════════════════════════════════════

def _daily_residual() -> pd.Series:
    """日度残差 r（每 UTC 日末），用于与逐日 σ 对齐。"""
    dx = load_close_series("dx")
    u6 = load_close_series("usd6e")
    df = pd.concat([dx.rename("dx"), u6.rename("u6")], axis=1, join="inner").dropna()
    r = np.log(df["dx"]) - EUR_WEIGHT * np.log(df["u6"])
    r_daily = r.resample("1D").last().dropna()
    # index → epoch//86400 日序号（与缓存 σ 的 day 对齐）。分辨率无关：
    # 归一到秒再 //86400（pandas 3.x 可能是 datetime64[us]，不可写死 ns 除数）。
    sec = r_daily.index.tz_convert(None).to_numpy().astype("datetime64[s]").astype("int64")
    days = sec // 86400
    r_daily = pd.Series(r_daily.to_numpy(), index=days, name="r")
    r_daily = r_daily[~r_daily.index.duplicated(keep="last")]
    return r_daily


def part2() -> dict:
    from statsmodels.tsa.stattools import grangercausalitytests

    print("\n" + "=" * 70)
    print("Part 2：残差变化率领先 K4 配置 (σ_P,σ_C,σ_R) 切换")
    print("=" * 70, flush=True)

    cache = json.load(open(DATA / "k4_config_transition_1min.json"))
    edges = cache["edges"]
    # σ_P=P/M, σ_C=C/M, σ_R=R/M（逐日，day=epoch//86400）
    sig = {}
    for key, tag in [("P/M", "sP"), ("C/M", "sC"), ("R/M", "sR")]:
        e = edges[key]
        s = pd.Series(e["sigma"], index=e["days"], name=tag)
        sig[tag] = s[~s.index.duplicated(keep="last")]

    r_daily = _daily_residual()
    dr = r_daily.diff().dropna()           # 残差日度变化
    dr.name = "dr"
    adr = dr.abs(); adr.name = "abs_dr"    # 残差变化幅度（regime 动荡）

    # 配置切换指示：σ^X_t ≠ σ^X_{t-1}
    df = pd.concat([sig["sP"], sig["sC"], sig["sR"]], axis=1, join="inner").dropna()
    switch = pd.DataFrame({
        "swP": (df["sP"].diff() != 0).astype(int),
        "swC": (df["sC"].diff() != 0).astype(int),
        "swR": (df["sR"].diff() != 0).astype(int),
    }, index=df.index)
    switch["sw_any"] = ((switch["swP"] + switch["swC"] + switch["swR"]) > 0).astype(int)

    # 与残差变化对齐（共同日序号）
    panel = pd.concat([dr, adr, switch], axis=1, join="inner").dropna()
    print(f"  残差变化 ∩ σ 配置 共同交易日 = {len(panel):,}  "
          f"{epoch_day_to_date(panel.index[0])} → {epoch_day_to_date(panel.index[-1])}",
          flush=True)

    results = {"n": int(len(panel)),
               "date_range": [str(epoch_day_to_date(panel.index[0])),
                              str(epoch_day_to_date(panel.index[-1]))],
               "lag_corr": {}, "granger": {}}

    # ── 滞后相关：|Δr|_{t-k} 与 sw_any_t（k>0 = 残差领先切换）──
    print("\n  ── 滞后相关 corr(|Δr|_{t−k}, σ切换_t)，k>0=残差领先 ──")
    maxlag = 10
    for target in ["sw_any", "swP", "swC", "swR"]:
        row = []
        for k in range(-3, maxlag + 1):
            a = panel["abs_dr"].shift(k)   # k>0：用过去的 |Δr| 预测当前切换
            b = panel[target]
            v = pd.concat([a, b], axis=1).dropna()
            c = float(np.corrcoef(v.iloc[:, 0], v.iloc[:, 1])[0, 1]) if len(v) > 30 else float("nan")
            row.append((k, c))
        results["lag_corr"][target] = row
        best = max((x for x in row if not np.isnan(x[1])), key=lambda z: z[1])
        print(f"  {target:7s} 最强滞后 k={best[0]:+d} corr={best[1]:+.4f}  "
              f"(k>0=残差领先)  [k=0: {dict(row)[0]:+.4f}]")

    # ── Granger 因果：Δr → σ切换（H0: Δr 不 Granger-导致切换）──
    print("\n  ── Granger 因果检验 Δr → σ切换（p<0.05 拒绝H0=残差领先）──")
    for target in ["sw_any", "swP", "swC", "swR"]:
        g = pd.concat([panel[target].rename("y"), panel["dr"].rename("x")],
                      axis=1).dropna()
        arr = g[["y", "x"]].values
        try:
            gres = grangercausalitytests(arr, maxlag=5, verbose=False)
            pvals = {int(lag): float(gres[lag][0]["ssr_ftest"][1]) for lag in gres}
            minlag = min(pvals, key=pvals.get)
            results["granger"][target] = {"pvals": {str(k): v for k, v in pvals.items()},
                                          "min_p_lag": int(minlag),
                                          "min_p": pvals[minlag]}
            sig_mark = "✓领先" if pvals[minlag] < 0.05 else "✗无"
            print(f"  Δr → {target:7s}: min p={pvals[minlag]:.4f} @lag{minlag}  {sig_mark}")
        except Exception as ex:
            results["granger"][target] = {"error": str(ex)}
            print(f"  Δr → {target:7s}: ERROR {ex}")

    # ── 反向对照：σ切换 → Δr（排除反向因果）──
    print("\n  ── 反向对照 Granger σ切换 → Δr（应弱于正向才支持残差领先）──")
    for target in ["sw_any"]:
        g = pd.concat([panel["dr"].rename("y"), panel[target].rename("x")],
                      axis=1).dropna()
        try:
            gres = grangercausalitytests(g[["y", "x"]].values, maxlag=5, verbose=False)
            pvals = {lag: float(gres[lag][0]["ssr_ftest"][1]) for lag in gres}
            minlag = min(pvals, key=pvals.get)
            results["granger"][f"reverse_{target}"] = {"min_p_lag": int(minlag),
                                                       "min_p": pvals[minlag]}
            print(f"  {target} → Δr: min p={pvals[minlag]:.4f} @lag{minlag}")
        except Exception as ex:
            print(f"  reverse ERROR {ex}")

    return results


if __name__ == "__main__":
    out = {"part1": part1(), "part2": part2()}
    outpath = DATA / "fx_residual_p1p2.json"
    outpath.write_text(json.dumps(out, indent=2, default=str))
    print(f"\n结果 → {outpath}")
