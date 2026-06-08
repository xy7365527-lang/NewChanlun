#!/usr/bin/env python3
"""递归分解树·纵向圈闭合实测（ES P 顶点 → Mag7 成分股 / CL 比价）。

设计文档：docs/architecture/recursive_decomposition_tree.md §3-4。
任务：以 ES（标普500 E-mini，P 顶点）为宏观，Magnificent 7 为其（部分）成分分解，
  每个成分构造 stockᵢ/CL 比价，做纵向圈闭合：
    Layer A（a0 精确闭合，L1 数据质量门）：ρ = log(ES) − log(Σ wᵢ·stockᵢ)。
    Layer B（走势状态软闭合，L0 兼容性）：σ(stockᵢ/CL) vs σ(ES/CL) 级别加权一致性。
    残差结构（步骤6）：ES/basket 残差比价跑缠论递归，看是否有走势结构。
    选股信号（步骤7）：哪些成分的 σ 转折领先 ES/CL（lead/lag）。

═══════════════════════════════════════════════════════════════════════
关键结构事实（必须显式声明，§11 命题1 + CL 抵消）：
  1. CL 在 Layer A 残差中**完全抵消**：log(ES/CL)−log(basket/CL)=log(ES/basket)。
     故 Layer A 残差 = ES vs 成分篮子的 tracking error，与 CL 无关。
  2. Layer B 的 σ(stockᵢ/CL) **不抵消**：σ 对比价非线性（§11 命题1，σ 不保比值同态），
     σ(stockᵢ/CL) 不是 σ(stockᵢ) 与 σ(CL) 的函数 → CL 实质参与各成分走势读数。
  3. 部分成分（7/500）→ Layer A **不可能 ≈0**（文档 §6 缺口）。残差的趋势恰是信号：
     Mag7 跑赢指数 → basket 涨快于 ES → ES/basket 下降趋势 = Mag7 驱动宏观（步骤7 宏观结论）。
═══════════════════════════════════════════════════════════════════════

数据质量（no-workaround，必须复权）：XNAS.ITCH 是**原始未复权价**，Mag7 在窗口内
  多次拆股（实测 AAPL 2020-08-28 $501.93→08-31 $129.34=4:1）。拆股日的价格跳变会被
  缠论读成虚假趋势 → 必须用已知拆股 ex-date/ratio 复权（SPLITS 表，公开事实）。

认识论等级：
  - 成分股 1min σ(stockᵢ/CL) 读数：**L2**（真实数据，可证伪）。
  - Layer A 残差：**L1**（部分成分=不完整，数据门，非信号源；诚实标注）。
  - Layer B 代数（synth/concentration/incompat/contribution）：**L0**（作用于已有 σ 读数）。
  - 选股 lead/lag 信号：**L2**（真实 σ 时序，可证伪）；alpha 是否真实需独立回测对照（未做）。

运行：PYTHONPATH=src python analysis/vertical_closure_test.py [n_procs]
"""

from __future__ import annotations

import json
import multiprocessing as mp
import sys
import time
from dataclasses import dataclass, replace
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for p in (str(ROOT), str(ROOT / "src")):
    if p not in sys.path:
        sys.path.insert(0, p)

from analysis.k4_1min_lib import RatioBars, Series, load_series, log_envelope_ratio

JSON_OUT = ROOT / "analysis" / "data_cache" / "vertical_closure_test.json"

# ── 成分股（Magnificent 7）+ 近似标普500权重（normalize 后 sum=1）───────
#   权重来源：约 2026 初标普500 公开权重（AAPL~7.0 MSFT~6.5 NVDA~6.5 AMZN~4.0
#   META~2.6 GOOGL~2.0 TSLA~1.8，合计~30.4%），在 7 只内归一。L1-近似（静态权重
#   对 2018 是 anachronistic——NVDA 2018 远小；同时报 equal-weight 做鲁棒性）。
COMPONENTS: dict[str, tuple[str, float]] = {
    # ticker: (file, raw_index_weight_pct)
    "AAPL":  ("aapl_1m_xnas.json", 7.0),
    "MSFT":  ("msft_1m_xnas.json", 6.5),
    "NVDA":  ("nvda_1m_xnas.json", 6.5),
    "AMZN":  ("amzn_1m_xnas.json", 4.0),
    "META":  ("meta_1m_xnas.json", 2.6),
    "GOOGL": ("googl_1m_xnas.json", 2.0),
    "TSLA":  ("tsla_1m_xnas.json", 1.8),
}

# ── 已知拆股（ex-date, 拆股比 R；公开事实）。复权：ex-date 之前所有价 ×(1/R)──
#   使序列连续到 post-split（当前股本）基准。窗口 2018-05 ~ 2026-06。
SPLITS: dict[str, list[tuple[str, float]]] = {
    "AAPL":  [("2020-08-31", 4.0)],
    "AMZN":  [("2022-06-06", 20.0)],
    "GOOGL": [("2022-07-18", 20.0)],
    "NVDA":  [("2021-07-20", 4.0), ("2024-06-10", 10.0)],
    "TSLA":  [("2020-08-31", 5.0), ("2022-08-25", 3.0)],
    "MSFT":  [],
    "META":  [],
}

ES_FILE = "es_1m_databento_10y.json"
CL_FILE = "cl_1m_databento_10y.json"

W_BSP = 0.0     # λ：无 per-edge 操作背驰买卖点输入 → λ=0（纯方向，诚实；文档 §3.2.6）
THETA = 0.5     # 不兼容阈值（文档 §3.2.3 默认）


# ════════════════════════════════════════════════════════════
# 拆股复权
# ════════════════════════════════════════════════════════════

def _ex_date_epoch(ex_date: str) -> int:
    import pandas as pd
    return int(pd.Timestamp(ex_date, tz="UTC").timestamp())


def split_adjust(s: Series, ticker: str) -> Series:
    """对 ex-date 之前的所有 bar 价格 ×(1/R)，复权到 post-split 股本基准。"""
    splits = SPLITS.get(ticker, [])
    if not splits:
        return s
    factor = np.ones_like(s.c)
    for ex_date, R in splits:
        ex_ep = _ex_date_epoch(ex_date)
        factor = np.where(s.ts < ex_ep, factor / R, factor)
    return replace(s, o=s.o * factor, h=s.h * factor, l=s.l * factor, c=s.c * factor)


# ════════════════════════════════════════════════════════════
# Rust 引擎 emergent σ（复用 gold_k4_sigma 语义，自包含避免导入脆弱）
# ════════════════════════════════════════════════════════════

def _sigma_from_head(head) -> int:
    kind, direction = head[0], head[1]
    if kind == "consolidation":
        return 0
    return 1 if direction == "up" else (-1 if direction == "down" else 0)


def _emergent_sigma_level(orch) -> tuple[int, int]:
    chosen_head = None
    max_lvl = -1
    for level_id, _zs, moves in orch.current_recursive():
        if moves and level_id > max_lvl:
            max_lvl = level_id
            chosen_head = moves[-1][0]
    if chosen_head is None:
        l1 = orch.current_moves()
        if l1:
            chosen_head = l1[-1][0]
            max_lvl = 1
    if chosen_head is None:
        return 0, 1
    return _sigma_from_head(chosen_head), max(1, max_lvl)


@dataclass
class EdgeDailySigma:
    name: str
    n_bars: int
    days: list[int]
    sigma: list[int]
    level: list[int]
    max_level: int
    elapsed: float


def run_edge_daily_sigma_rust(rb: RatioBars, *, max_levels: int = 6) -> EdgeDailySigma:
    """Rust 引擎流式跑比价 bar，每 UTC 日边界记录 as-of 最高涌现级别 σ（zero-lookahead）。"""
    import newchan_rust

    t0 = time.time()
    n = len(rb.c)
    day = rb.day
    o, h, l, c = rb.o, rb.h, rb.l, rb.c
    orch = newchan_rust.RecursiveOrchestrator(max_levels=max_levels, stroke_mode="wide")
    days_out: list[int] = []
    sigma_out: list[int] = []
    level_out: list[int] = []
    max_lvl_seen = 1
    for i in range(n):
        orch.process_bar(float(o[i]), float(h[i]), float(l[i]), float(c[i]))
        if i == n - 1 or day[i + 1] != day[i]:
            sg, lv = _emergent_sigma_level(orch)
            days_out.append(int(day[i]))
            sigma_out.append(sg)
            level_out.append(lv)
            max_lvl_seen = max(max_lvl_seen, lv)
    return EdgeDailySigma(
        name=rb.name, n_bars=n, days=days_out, sigma=sigma_out,
        level=level_out, max_level=max_lvl_seen, elapsed=time.time() - t0,
    )


def _sigma_worker(rb: RatioBars) -> dict:
    r = run_edge_daily_sigma_rust(rb)
    print(f"  σ {r.name:14s} done: {r.n_bars:>9,} bars, {len(r.days)} days, "
          f"L{r.max_level}, {r.elapsed:.0f}s", flush=True)
    return {"name": r.name, "n_bars": r.n_bars, "days": r.days,
            "sigma": r.sigma, "level": r.level, "max_level": r.max_level}


# ════════════════════════════════════════════════════════════
# N 路时间对齐 + 加权篮子
# ════════════════════════════════════════════════════════════

def align_many(series: dict[str, Series]) -> tuple[np.ndarray, dict[str, dict[str, np.ndarray]]]:
    """多序列按 epoch 时间戳取公共交集（inner join）。返回 (common_ts, {name:{o,h,l,c}})。"""
    names = list(series)
    common = series[names[0]].ts
    for nm in names[1:]:
        common = np.intersect1d(common, series[nm].ts)
    out: dict[str, dict[str, np.ndarray]] = {}
    for nm in names:
        s = series[nm]
        idx = np.searchsorted(s.ts, common)
        out[nm] = {"o": s.o[idx], "h": s.h[idx], "l": s.l[idx], "c": s.c[idx]}
    return common, out


# ════════════════════════════════════════════════════════════
# Layer B 级别加权兼容性（文档 §3.2，逐日时间序列）
# ════════════════════════════════════════════════════════════

def level_confidence(level: int) -> float:
    """conf = 1 − 2^(−level)（级别清晰度，与 σ 正交；文档 §3.2.2）。"""
    return 1.0 - 2.0 ** (-level)


def layer_b_daily(macro: dict, comps: dict[str, dict], weights: dict[str, float]) -> dict:
    """逐日 Layer B 兼容性 + 贡献度。所有 σ 时序按公共 day 对齐。

    macro/comps[name] = {"days":[...], "sigma":[...], "level":[...]}。
    """
    md = {d: (sg, lv) for d, sg, lv in zip(macro["days"], macro["sigma"], macro["level"])}
    comp_maps = {
        nm: {d: (sg, lv) for d, sg, lv in zip(c["days"], c["sigma"], c["level"])}
        for nm, c in comps.items()
    }
    common_days = sorted(set(md) & set.intersection(*[set(m) for m in comp_maps.values()]))

    daily = []
    contrib_acc = {nm: 0.0 for nm in comps}
    resid_acc = {nm: [0, 0, 0] for nm in comps}  # 计数 {0,1,2}
    lead_series = {nm: [] for nm in comps}       # 每个成分的逐日 σ（用于 lead/lag）
    macro_series = []
    for d in common_days:
        sm, _lm = md[d]
        macro_series.append(sm)
        eff = {}
        incompat = 0.0
        synth = 0.0
        wp = 0.0   # weighted participation（同向有效权重）
        cp = 0     # count participation（同向个数）
        for nm in comps:
            sg, lv = comp_maps[nm][d]
            lead_series[nm].append(sg)
            w = weights[nm] * level_confidence(lv)
            eff[nm] = w
            synth += w * sg
            prod = sg * sm
            if prod < 0:
                incompat += w
            if sg == sm and sm != 0:
                wp += w
                cp += 1
            contrib_acc[nm] += w * prod
            resid = abs(sg - sm)
            resid_acc[nm][min(resid, 2)] += 1
        n = len(comps)
        concentration = wp - (cp / n if n else 0.0)
        daily.append({
            "day": d, "macro_sigma": sm, "synth": synth,
            "incompatibility": incompat, "compatible": incompat <= THETA,
            "concentration": concentration,
        })
    return {
        "common_days": common_days, "daily": daily,
        "macro_series": macro_series, "lead_series": lead_series,
        "contrib_total": contrib_acc, "residual_dist": resid_acc,
    }


# ════════════════════════════════════════════════════════════
# 选股信号：lead/lag（成分 σ 转折是否领先宏观 σ）
# ════════════════════════════════════════════════════════════

def lead_lag(comp_sigma: list[int], macro_sigma: list[int], max_lag: int = 20) -> dict:
    """互相关：corr(σ_comp(t), σ_macro(t+k))，k>0 = 成分领先宏观 k 日。

    返回峰值 lag、峰值相关、同期相关。lead_bias>0 → 成分领先。
    """
    a = np.asarray(comp_sigma, dtype=np.float64)
    b = np.asarray(macro_sigma, dtype=np.float64)
    a = a - a.mean()
    b = b - b.mean()
    denom = np.sqrt((a @ a) * (b @ b))
    if denom == 0:
        return {"peak_lag": 0, "peak_corr": 0.0, "corr0": 0.0, "lead_bias": 0.0}
    best_lag, best_corr = 0, -2.0
    corr0 = float((a @ b) / denom)
    lead_sum, lag_sum = 0.0, 0.0
    for k in range(-max_lag, max_lag + 1):
        if k > 0:       # comp(t) vs macro(t+k): 比较 a[:-k] 与 b[k:]
            x, y = a[:-k], b[k:]
        elif k < 0:
            x, y = a[-k:], b[:k]
        else:
            x, y = a, b
        if len(x) < 10:
            continue
        c = float((x @ y) / denom)
        if c > best_corr:
            best_corr, best_lag = c, k
        if k > 0:
            lead_sum += max(c, 0.0)
        elif k < 0:
            lag_sum += max(c, 0.0)
    return {"peak_lag": best_lag, "peak_corr": round(best_corr, 4),
            "corr0": round(corr0, 4), "lead_bias": round(lead_sum - lag_sum, 4)}


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    t_start = time.time()
    procs = int(sys.argv[1]) if len(sys.argv) > 1 else 8
    print("=" * 78)
    print("纵向圈闭合实测：ES(P顶点) → Mag7 成分 / CL 比价")
    print("=" * 78, flush=True)

    # ── 加载 + 复权 ───────────────────────────────────────
    print("\n[1] 加载 ES/CL + Mag7（复权）…", flush=True)
    es = load_series(ES_FILE)
    cl = load_series(CL_FILE)
    print(f"  ES {len(es.c):,} bars  CL {len(cl.c):,} bars", flush=True)

    avail: dict[str, Series] = {}
    for tk, (fn, _w) in COMPONENTS.items():
        fp = ROOT / "analysis" / "data_cache" / fn
        if not fp.exists():
            print(f"  ⚠ {tk}: {fn} 缺失，跳过（诚实标注缺口）", flush=True)
            continue
        s = split_adjust(load_series(fn), tk)
        avail[tk] = s
        nsp = len(SPLITS.get(tk, []))
        print(f"  {tk:6s} {len(s.c):>9,} bars  复权{nsp}次", flush=True)

    if len(avail) < 3:
        print(f"\n✗ 可用成分股 <3（{list(avail)}），无法做纵向闭合。报告缺口，不编造。")
        return

    # 权重（归一 + equal-weight 鲁棒性）
    raw_w = {tk: COMPONENTS[tk][1] for tk in avail}
    tot = sum(raw_w.values())
    mcap_w = {tk: raw_w[tk] / tot for tk in avail}
    eq_w = {tk: 1.0 / len(avail) for tk in avail}
    print(f"  市值权重(归一): " + ", ".join(f"{k}={v:.3f}" for k, v in mcap_w.items()), flush=True)

    # ── Layer A：a0 精确闭合残差（ES vs 加权篮子；CL 抵消）──────────
    print("\n[2] Layer A：ρ = log(ES) − log(Σ wᵢ·stockᵢ)（CL 抵消，L1 数据门）", flush=True)
    align_in = {"ES": es, **avail}
    common_ts, aligned = align_many(align_in)
    print(f"  ES+{len(avail)}成分 公共 bar 数: {len(common_ts):,}", flush=True)
    es_c = aligned["ES"]["c"]
    basket_c = np.zeros_like(es_c)
    basket_o = np.zeros_like(es_c)
    basket_h = np.zeros_like(es_c)
    basket_l = np.zeros_like(es_c)
    for tk in avail:
        w = mcap_w[tk]
        basket_c += w * aligned[tk]["c"]
        basket_o += w * aligned[tk]["o"]
        basket_h += w * aligned[tk]["h"]
        basket_l += w * aligned[tk]["l"]
    # 过滤 NaN/非正（databento 1min 已知少量 NaN bar；my-mem 数据nan必删）
    fin = np.isfinite(es_c) & (es_c > 0) & np.isfinite(basket_c) & (basket_c > 0)
    n_drop = int((~fin).sum())
    rho = np.log(es_c[fin]) - np.log(basket_c[fin])
    rho_dm = rho - rho.mean()   # 去均值（ES 与篮子单位不同，只看变化结构）
    rho_drift = float(rho[-100:].mean() - rho[:100].mean()) if len(rho) > 200 else 0.0
    print(f"  剔除 NaN/非正 {n_drop} bar；有效 {len(rho):,}", flush=True)
    print(f"  原始 ρ: mean={rho.mean():.4f} (单位失配偏移，无意义) "
          f"std={rho.std():.4f}", flush=True)
    print(f"  ρ 漂移(末100−首100)={rho_drift:.4f} "
          f"→ {'下降=篮子涨快于ES=Mag7驱动宏观' if rho_drift < 0 else '上升=篮子弱于ES'}",
          flush=True)
    print(f"  去均值 ρ: std={rho_dm.std():.4f} max|ρ̃|={np.abs(rho_dm).max():.4f} "
          f"→ 部分成分(7/500) tracking error，非≈0（文档§6缺口，符合预期）", flush=True)

    # 残差结构（步骤6）：ES/basket 残差比价 → 缠论递归
    es_series_aligned = Series(symbol="ES", ts=common_ts, day=common_ts // 86400,
                               o=aligned["ES"]["o"], h=aligned["ES"]["h"],
                               l=aligned["ES"]["l"], c=es_c)
    basket_series = Series(symbol="BASKET", ts=common_ts, day=common_ts // 86400,
                           o=basket_o, h=basket_h, l=basket_l, c=basket_c)
    resid_rb = log_envelope_ratio(es_series_aligned, basket_series, "ES/basket(残差)")

    # ── Layer B：每边 σ(stockᵢ/CL) + σ(ES/CL)（Rust 并行）+ 残差结构 ──
    print(f"\n[3] 构造比价 + Rust 递归 σ（{procs} 并行）…", flush=True)
    rbs: list[RatioBars] = [log_envelope_ratio(es, cl, "ES/CL")]
    for tk in avail:
        rbs.append(log_envelope_ratio(avail[tk], cl, f"{tk}/CL"))
    rbs.append(resid_rb)  # 残差也跑递归（步骤6）
    for rb in rbs:
        print(f"    {rb.name:18s} {len(rb.c):>9,} bars", flush=True)

    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=procs) as pool:
        sigma_results = pool.map(_sigma_worker, rbs)
    sig = {r["name"]: r for r in sigma_results}

    # ── Layer B 闭合（市值权重 + equal-weight）────────────────────
    print("\n[4] Layer B 级别加权兼容性 + 贡献度…", flush=True)
    macro = sig["ES/CL"]
    comps = {tk: sig[f"{tk}/CL"] for tk in avail}
    lb_mcap = layer_b_daily(macro, comps, mcap_w)
    lb_eq = layer_b_daily(macro, comps, eq_w)

    n_days = len(lb_mcap["daily"])
    n_compat = sum(1 for d in lb_mcap["daily"] if d["compatible"])
    mean_conc = np.mean([d["concentration"] for d in lb_mcap["daily"]]) if n_days else 0.0
    print(f"  公共交易日: {n_days}  兼容(incompat≤{THETA}): {n_compat} "
          f"({100*n_compat/max(n_days,1):.1f}%)", flush=True)
    print(f"  平均 concentration(市值权重): {mean_conc:.4f} "
          f"(>0=趋势集中高权重少数成分=脆弱)", flush=True)

    # ── 选股信号：lead/lag + 贡献度排序（步骤7）──────────────────
    print("\n[5] 选股信号：lead/lag（成分 σ 转折是否领先 ES/CL）+ 贡献度排序", flush=True)
    macro_series = lb_mcap["macro_series"]
    entries = []
    for tk in avail:
        ll = lead_lag(lb_mcap["lead_series"][tk], macro_series)
        rd = lb_mcap["residual_dist"][tk]
        rd_tot = max(sum(rd), 1)
        entries.append({
            "symbol": tk, "weight": round(mcap_w[tk], 4),
            "contribution_mcap": round(lb_mcap["contrib_total"][tk], 2),
            "contribution_eq": round(lb_eq["contrib_total"][tk], 2),
            "lead_lag": ll,
            "resid_aligned_pct": round(100 * rd[0] / rd_tot, 1),
            "resid_opposite_pct": round(100 * rd[2] / rd_tot, 1),
            "max_level": sig[f"{tk}/CL"]["max_level"],
        })
    # 排序：先按 lead_bias 降（领先性=选股核心），再按贡献度
    entries.sort(key=lambda e: (e["lead_lag"]["lead_bias"], e["contribution_mcap"]), reverse=True)
    print(f"\n  {'成分':6s} {'权重':>6s} {'贡献(mcap)':>10s} {'峰值lag':>7s} "
          f"{'lead_bias':>9s} {'同期corr':>8s} {'对齐%':>6s} {'反向%':>6s} {'级别':>4s}")
    for e in entries:
        ll = e["lead_lag"]
        lead_tag = "领先" if ll["lead_bias"] > 0 else ("滞后" if ll["lead_bias"] < 0 else "同步")
        print(f"  {e['symbol']:6s} {e['weight']:>6.3f} {e['contribution_mcap']:>10.1f} "
              f"{ll['peak_lag']:>7d} {ll['lead_bias']:>9.3f} {ll['corr0']:>8.3f} "
              f"{e['resid_aligned_pct']:>6.1f} {e['resid_opposite_pct']:>6.1f} "
              f"L{e['max_level']:<3d} {lead_tag}")

    # ── 残差结构（步骤6）──────────────────────────────────
    rs = sig["ES/basket(残差)"]
    print(f"\n[6] 残差 ES/basket 缠论结构: {rs['n_bars']:,} bars, L{rs['max_level']}, "
          f"{len(rs['days'])} 日 σ；末 σ={rs['sigma'][-1] if rs['sigma'] else 'NA'}", flush=True)
    # 残差 σ 分布
    rsig = np.asarray(rs["sigma"])
    print(f"  残差日 σ 分布: 上={int((rsig>0).sum())} 平={int((rsig==0).sum())} "
          f"下={int((rsig<0).sum())} → 主导方向="
          f"{'下降(篮子>ES=Mag7驱动宏观)' if (rsig<0).sum()>(rsig>0).sum() else '上升' if (rsig>0).sum()>(rsig<0).sum() else '中性'}",
          flush=True)

    # ── 写 JSON ───────────────────────────────────────────
    out = {
        "meta": {
            "task": "vertical_closure_test",
            "macro_vertex": "ES (S&P500 E-mini, P)",
            "denominator": "CL (WTI crude)",
            "components": list(avail),
            "weights_mcap": {k: round(v, 4) for k, v in mcap_w.items()},
            "n_common_bars_layerA": int(len(common_ts)),
            "method": "1min a0, log-envelope ratio, highest-emergent-level sigma, "
                      "UTC daily boundary, zero-lookahead, Rust orchestrator; split-adjusted",
            "epistemology": {"sigma_readings": "L2", "layerA_residual": "L1(partial constituents)",
                             "layerB_algebra": "L0", "leadlag_signal": "L2 (alpha untested)"},
            "elapsed_s": round(time.time() - t_start, 1),
        },
        "layer_a": {
            "n_valid": int(len(rho)), "n_dropped_nan": n_drop,
            "rho_mean": float(rho.mean()), "rho_std": float(rho.std()),
            "rho_demeaned_std": float(rho_dm.std()), "rho_demeaned_maxabs": float(np.abs(rho_dm).max()),
            "rho_drift_last_minus_first": rho_drift,
            "note": "CL cancels; partial constituents → not ~0 by construction (doc §6); "
                    "rho_drift<0 = basket outperformed ES = Mag7 drove index",
        },
        "layer_b": {
            "n_days": n_days, "n_compatible": n_compat,
            "compatible_pct": round(100 * n_compat / max(n_days, 1), 1),
            "mean_concentration_mcap": round(float(mean_conc), 4),
        },
        "selection": entries,
        "residual_structure": {
            "n_bars": rs["n_bars"], "max_level": rs["max_level"],
            "n_days": len(rs["days"]),
            "sigma_up": int((rsig > 0).sum()), "sigma_flat": int((rsig == 0).sum()),
            "sigma_down": int((rsig < 0).sum()),
        },
        "sigma_max_levels": {r["name"]: r["max_level"] for r in sigma_results},
    }
    JSON_OUT.write_text(json.dumps(out, indent=2))
    print(f"\n总耗时 {time.time()-t_start:.0f}s → {JSON_OUT}", flush=True)


if __name__ == "__main__":
    main()
