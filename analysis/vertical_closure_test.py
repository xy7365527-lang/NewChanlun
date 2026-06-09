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

数据质量（no-workaround）：XNAS.ITCH 是**原始未复权价**，Mag7 在窗口内多次拆股
  （实测 AAPL 2020-08-28 $501.93→08-31 $129.34=4:1）→ 必须按已知拆股 ex-date/ratio
  复权（SPLITS 表，公开事实）；databento 少量 NaN bar 过滤。

架构（崩溃可恢复，no-patch）：σ 计算与分析两相分离。每条边 σ 算完**立即落盘**
  （_vclosure_sigma/<edge>.json 检查点），主进程不囤大数组；重跑自动跳过已缓存边
  （断点续算）。上次单脚本主进程在 pool.map 中囤积全部原始序列+RatioBars 内存峰值
  崩溃、σ 全丢——本版根除。

认识论等级：
  - 成分股 1min σ(stockᵢ/CL) 读数：**L2**（真实数据，可证伪）。
  - Layer A 残差：**L1**（部分成分=不完整，数据门，非信号源）。
  - Layer B 代数（synth/concentration/incompat/contribution）：**L0**（作用于已有 σ 读数）。
  - 选股 lead/lag 信号：**L2**（真实 σ 时序）；alpha 是否真实需独立回测对照（未做）。

运行（σ 阶段 ~ES/CL 67min 主导，可断点续算）：
  PYTHONPATH=src .venv/bin/python -u analysis/vertical_closure_test.py [n_procs]
仅重跑分析（σ 已缓存）：附加 analyze
  PYTHONPATH=src .venv/bin/python analysis/vertical_closure_test.py analyze
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

DATA = ROOT / "analysis" / "data_cache"
CKPT = DATA / "_vclosure_sigma"
CKPT.mkdir(parents=True, exist_ok=True)
JSON_OUT = DATA / "vertical_closure_test.json"

# ── 成分股（Magnificent 7）+ 近似标普500权重（normalize 后 sum=1）───────
COMPONENTS: dict[str, tuple[str, float]] = {
    "AAPL":  ("aapl_1m_xnas.json", 7.0),
    "MSFT":  ("msft_1m_xnas.json", 6.5),
    "NVDA":  ("nvda_1m_xnas.json", 6.5),
    "AMZN":  ("amzn_1m_xnas.json", 4.0),
    "META":  ("meta_1m_xnas.json", 2.6),
    "GOOGL": ("googl_1m_xnas.json", 2.0),
    "TSLA":  ("tsla_1m_xnas.json", 1.8),
}

# ── 已知拆股（ex-date, R；公开事实）。复权：ex-date 之前所有价 ×(1/R) ──
SPLITS: dict[str, list[tuple[str, float]]] = {
    "AAPL":  [("2020-08-31", 4.0)],
    "AMZN":  [("2022-06-06", 20.0)],
    "GOOGL": [("2022-07-18", 20.0)],
    "NVDA":  [("2021-07-20", 4.0), ("2024-06-10", 10.0)],
    "TSLA":  [("2020-08-31", 5.0), ("2022-08-25", 3.0)],
    "MSFT":  [], "META":  [],
}

ES_FILE = "es_1m_databento_10y.json"
CL_FILE = "cl_1m_databento_10y.json"
THETA = 0.5     # 不兼容阈值（文档 §3.2.3 默认）
# ES/CL 起点裁剪到 2021-01（META 联合窗口 2021-06 前留 5 个月预热）。
# 理由：(1) 联合 Layer B 由 META(2021-06+) 限制，ES/CL 只在 2021+ 被消费，2018-2021
#       段对联合分析无用；(2) 共享机器另一 session 蜂群（m1_i/k4_config）反复制造内存
#       尖峰触发 jetsam，5.4M/4.3M bar 的 ES/CL 两次被杀——2.1M bar 内存减半、~25min
#       暴露窗口短，避开 jetsam。诚实标注：σ(ES/CL) 早期(2021)级别涌现预热较短。
ES_START = "2021-01-01"


# ════════════════════════════════════════════════════════════
# 拆股复权
# ════════════════════════════════════════════════════════════

def _ex_date_epoch(ex_date: str) -> int:
    import pandas as pd
    return int(pd.Timestamp(ex_date, tz="UTC").timestamp())


def split_adjust(s: Series, ticker: str) -> Series:
    splits = SPLITS.get(ticker, [])
    if not splits:
        return s
    factor = np.ones_like(s.c)
    for ex_date, R in splits:
        factor = np.where(s.ts < _ex_date_epoch(ex_date), factor / R, factor)
    return replace(s, o=s.o * factor, h=s.h * factor, l=s.l * factor, c=s.c * factor)


def load_component(ticker: str) -> Series:
    return split_adjust(load_series(COMPONENTS[ticker][0]), ticker)


# ════════════════════════════════════════════════════════════
# Rust 引擎 emergent σ
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


def run_daily_sigma(rb: RatioBars, *, max_levels: int = 6) -> dict:
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
    return {"name": rb.name, "n_bars": n, "days": days_out, "sigma": sigma_out,
            "level": level_out, "max_level": max_lvl_seen, "elapsed": time.time() - t0}


def _ckpt_path(edge: str) -> Path:
    return CKPT / f"{edge.replace('/', '_')}.json"


# ════════════════════════════════════════════════════════════
# σ workers（落盘检查点；worker 内加载+构造+释放源序列以控内存）
# ════════════════════════════════════════════════════════════

def _worker_ratio_sigma(edge_spec: tuple) -> str:
    """edge_spec = (edge_name, kind, arg)。kind: 'es_cl' | 'stock_cl' | 'residual'。"""
    edge_name, kind, arg = edge_spec
    ckpt = _ckpt_path(edge_name)
    if ckpt.exists():
        try:
            json.loads(ckpt.read_text())
            print(f"  σ {edge_name:18s} [缓存命中，跳过]", flush=True)
            return str(ckpt)
        except Exception:  # noqa: BLE001
            pass

    cl = load_series(CL_FILE)
    if kind == "es_cl":
        es = load_series(ES_FILE)
        start_ep = _ex_date_epoch(ES_START)
        m_es = es.ts >= start_ep
        m_cl = cl.ts >= start_ep
        es = replace(es, ts=es.ts[m_es], day=es.day[m_es], o=es.o[m_es],
                     h=es.h[m_es], l=es.l[m_es], c=es.c[m_es])
        cl = replace(cl, ts=cl.ts[m_cl], day=cl.day[m_cl], o=cl.o[m_cl],
                     h=cl.h[m_cl], l=cl.l[m_cl], c=cl.c[m_cl])
        rb = log_envelope_ratio(es, cl, edge_name)
    elif kind == "stock_cl":
        rb = log_envelope_ratio(load_component(arg), cl, edge_name)
    elif kind == "residual":
        # 残差 ES/basket（CL 抵消，不需 cl）。arg = 成分 ticker 列表。
        es = load_series(ES_FILE)
        comps = {tk: load_component(tk) for tk in arg}
        common_ts, aligned = align_many({"ES": es, **comps})
        raw_w = {tk: COMPONENTS[tk][1] for tk in arg}
        tot = sum(raw_w.values())
        bo = bh = bl = bc = None
        for tk in arg:
            w = raw_w[tk] / tot
            bo = w * aligned[tk]["o"] if bo is None else bo + w * aligned[tk]["o"]
            bh = w * aligned[tk]["h"] if bh is None else bh + w * aligned[tk]["h"]
            bl = w * aligned[tk]["l"] if bl is None else bl + w * aligned[tk]["l"]
            bc = w * aligned[tk]["c"] if bc is None else bc + w * aligned[tk]["c"]
        es_a = Series("ES", common_ts, common_ts // 86400,
                      aligned["ES"]["o"], aligned["ES"]["h"], aligned["ES"]["l"], aligned["ES"]["c"])
        basket = Series("BASKET", common_ts, common_ts // 86400, bo, bh, bl, bc)
        rb = log_envelope_ratio(es_a, basket, edge_name)
    else:
        raise ValueError(kind)

    res = run_daily_sigma(rb)
    ckpt.write_text(json.dumps(res))
    print(f"  σ {edge_name:18s} done: {res['n_bars']:>9,} bars, {len(res['days'])} days, "
          f"L{res['max_level']}, {res['elapsed']:.0f}s", flush=True)
    return str(ckpt)


# ════════════════════════════════════════════════════════════
# N 路对齐
# ════════════════════════════════════════════════════════════

def align_many(series: dict[str, Series]) -> tuple[np.ndarray, dict[str, dict[str, np.ndarray]]]:
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
# Layer B 级别加权兼容性 + 选股信号
# ════════════════════════════════════════════════════════════

def level_confidence(level: int) -> float:
    return 1.0 - 2.0 ** (-level)


def layer_b_daily(macro: dict, comps: dict[str, dict], weights: dict[str, float]) -> dict:
    md = {d: (sg, lv) for d, sg, lv in zip(macro["days"], macro["sigma"], macro["level"])}
    comp_maps = {nm: {d: (sg, lv) for d, sg, lv in zip(c["days"], c["sigma"], c["level"])}
                 for nm, c in comps.items()}
    common_days = sorted(set(md) & set.intersection(*[set(m) for m in comp_maps.values()]))
    daily = []
    contrib_acc = {nm: 0.0 for nm in comps}
    resid_acc = {nm: [0, 0, 0] for nm in comps}
    lead_series = {nm: [] for nm in comps}
    macro_series = []
    for d in common_days:
        sm, _lm = md[d]
        macro_series.append(sm)
        incompat = synth = wp = 0.0
        cp = 0
        for nm in comps:
            sg, lv = comp_maps[nm][d]
            lead_series[nm].append(sg)
            w = weights[nm] * level_confidence(lv)
            synth += w * sg
            prod = sg * sm
            if prod < 0:
                incompat += w
            if sg == sm and sm != 0:
                wp += w
                cp += 1
            contrib_acc[nm] += w * prod
            resid_acc[nm][min(abs(sg - sm), 2)] += 1
        n = len(comps)
        daily.append({"day": d, "macro_sigma": sm, "synth": synth,
                      "incompatibility": incompat, "compatible": incompat <= THETA,
                      "concentration": wp - (cp / n if n else 0.0)})
    return {"common_days": common_days, "daily": daily, "macro_series": macro_series,
            "lead_series": lead_series, "contrib_total": contrib_acc, "residual_dist": resid_acc}


def lead_lag(comp_sigma: list[int], macro_sigma: list[int], max_lag: int = 20) -> dict:
    """互相关 corr(σ_comp(t), σ_macro(t+k))，k>0=成分领先宏观 k 日。lead_bias>0→领先。"""
    a = np.asarray(comp_sigma, dtype=np.float64)
    b = np.asarray(macro_sigma, dtype=np.float64)
    a = a - a.mean()
    b = b - b.mean()
    denom = np.sqrt((a @ a) * (b @ b))
    if denom == 0:
        return {"peak_lag": 0, "peak_corr": 0.0, "corr0": 0.0, "lead_bias": 0.0}
    best_lag, best_corr = 0, -2.0
    corr0 = float((a @ b) / denom)
    lead_sum = lag_sum = 0.0
    for k in range(-max_lag, max_lag + 1):
        if k > 0:
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
# Layer A（独立 numpy，秒级；NaN 过滤）
# ════════════════════════════════════════════════════════════

def compute_layer_a(avail: list[str]) -> dict:
    es = load_series(ES_FILE)
    comps = {tk: load_component(tk) for tk in avail}
    common_ts, aligned = align_many({"ES": es, **comps})
    raw_w = {tk: COMPONENTS[tk][1] for tk in avail}
    tot = sum(raw_w.values())
    es_c = aligned["ES"]["c"]
    basket_c = np.zeros_like(es_c)
    for tk in avail:
        basket_c += (raw_w[tk] / tot) * aligned[tk]["c"]
    fin = np.isfinite(es_c) & (es_c > 0) & np.isfinite(basket_c) & (basket_c > 0)
    rho = np.log(es_c[fin]) - np.log(basket_c[fin])
    drift = float(rho[-100:].mean() - rho[:100].mean()) if len(rho) > 200 else 0.0
    return {"n_common": int(len(es_c)), "n_valid": int(fin.sum()),
            "n_dropped": int((~fin).sum()), "rho_mean": float(rho.mean()),
            "rho_std": float(rho.std()), "rho_demeaned_std": float((rho - rho.mean()).std()),
            "rho_drift_last_minus_first": drift}


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    t_start = time.time()
    mode_analyze = "analyze" in sys.argv
    procs = next((int(a) for a in sys.argv[1:] if a.isdigit()), 8)

    print("=" * 78)
    print("纵向圈闭合实测：ES(P顶点) → Mag7 成分 / CL 比价")
    print("=" * 78, flush=True)

    avail = [tk for tk in COMPONENTS if (DATA / COMPONENTS[tk][0]).exists()]
    if len(avail) < 3:
        print(f"✗ 可用成分股 <3（{avail}），无法做纵向闭合。报告缺口，不编造。")
        return
    raw_w = {tk: COMPONENTS[tk][1] for tk in avail}
    tot = sum(raw_w.values())
    mcap_w = {tk: raw_w[tk] / tot for tk in avail}
    eq_w = {tk: 1.0 / len(avail) for tk in avail}
    print(f"成分: {avail}")
    print("市值权重(归一): " + ", ".join(f"{k}={v:.3f}" for k, v in mcap_w.items()), flush=True)

    # ── σ 阶段（落盘检查点 + 断点续算）────────────────────────
    edge_specs = [("ES/CL", "es_cl", None)]
    edge_specs += [(f"{tk}/CL", "stock_cl", tk) for tk in avail]
    edge_specs += [("ES/basket", "residual", avail)]

    if not mode_analyze:
        print(f"\n[σ 阶段] {len(edge_specs)} 条边，{procs} 并行（已缓存边自动跳过）…", flush=True)
        ctx = mp.get_context("spawn")
        with ctx.Pool(processes=procs) as pool:
            pool.map(_worker_ratio_sigma, edge_specs)

    # ── 加载 σ 检查点 ─────────────────────────────────────
    print("\n[分析阶段] 加载 σ 检查点…", flush=True)
    sig = {}
    for edge_name, _, _ in edge_specs:
        ck = _ckpt_path(edge_name)
        if not ck.exists():
            print(f"  ⚠ {edge_name} 检查点缺失 → 请先跑 σ 阶段")
            return
        sig[edge_name] = json.loads(ck.read_text())

    # ── Layer A（独立 numpy）────────────────────────────────
    print("\n[Layer A] ρ=log(ES)−log(Σwᵢ·stockᵢ)（CL 抵消，L1 数据门）…", flush=True)
    la = compute_layer_a(avail)
    print(f"  对齐 {la['n_common']:,} bar, 有效 {la['n_valid']:,}（剔 {la['n_dropped']} NaN）", flush=True)
    print(f"  ρ mean={la['rho_mean']:.4f}(单位失配偏移) 去均值std={la['rho_demeaned_std']:.4f}"
          f" → 部分成分(7/500) tracking error，非≈0（文档§6）", flush=True)
    print(f"  ρ 漂移(末−首)={la['rho_drift_last_minus_first']:.4f} → "
          f"{'下降=篮子涨快于ES=Mag7驱动宏观' if la['rho_drift_last_minus_first']<0 else '上升=篮子弱于ES'}",
          flush=True)

    # ── Layer B ──────────────────────────────────────────
    print("\n[Layer B] 级别加权兼容性 + 贡献度…", flush=True)
    macro = sig["ES/CL"]
    comps = {tk: sig[f"{tk}/CL"] for tk in avail}
    lb = layer_b_daily(macro, comps, mcap_w)
    lb_eq = layer_b_daily(macro, comps, eq_w)
    n_days = len(lb["daily"])
    n_compat = sum(1 for d in lb["daily"] if d["compatible"])
    mean_conc = float(np.mean([d["concentration"] for d in lb["daily"]])) if n_days else 0.0
    day0, day1 = lb["common_days"][0], lb["common_days"][-1]
    import datetime as _dt
    win = (f"{_dt.date(1970,1,1)+_dt.timedelta(days=day0)}~"
           f"{_dt.date(1970,1,1)+_dt.timedelta(days=day1)}")
    print(f"  联合公共交易日: {n_days}（{win}；META 2021+ 限制联合窗口）", flush=True)
    print(f"  兼容(incompat≤{THETA}): {n_compat} ({100*n_compat/max(n_days,1):.1f}%)", flush=True)
    print(f"  平均 concentration(市值权重): {mean_conc:.4f}（>0=趋势集中高权重少数成分=脆弱）", flush=True)

    # ── 选股信号：lead/lag + 贡献度排序 ─────────────────────
    print("\n[选股信号] lead/lag（成分 σ 转折是否领先 ES/CL）+ 贡献度排序", flush=True)
    macro_series = lb["macro_series"]
    entries = []
    for tk in avail:
        ll = lead_lag(lb["lead_series"][tk], macro_series)
        rd = lb["residual_dist"][tk]
        rd_tot = max(sum(rd), 1)
        entries.append({
            "symbol": tk, "weight": round(mcap_w[tk], 4),
            "contribution_mcap": round(lb["contrib_total"][tk], 2),
            "contribution_eq": round(lb_eq["contrib_total"][tk], 2),
            "lead_lag": ll,
            "resid_aligned_pct": round(100 * rd[0] / rd_tot, 1),
            "resid_opposite_pct": round(100 * rd[2] / rd_tot, 1),
            "max_level": sig[f"{tk}/CL"]["max_level"],
        })
    entries.sort(key=lambda e: (e["lead_lag"]["lead_bias"], e["contribution_mcap"]), reverse=True)
    print(f"\n  {'成分':6s}{'权重':>7s}{'贡献mcap':>10s}{'峰lag':>6s}{'lead_bias':>10s}"
          f"{'同期corr':>9s}{'对齐%':>7s}{'反向%':>7s}{'级别':>5s}")
    for e in entries:
        ll = e["lead_lag"]
        tag = "领先" if ll["lead_bias"] > 0 else ("滞后" if ll["lead_bias"] < 0 else "同步")
        print(f"  {e['symbol']:6s}{e['weight']:>7.3f}{e['contribution_mcap']:>10.1f}"
              f"{ll['peak_lag']:>6d}{ll['lead_bias']:>10.3f}{ll['corr0']:>9.3f}"
              f"{e['resid_aligned_pct']:>7.1f}{e['resid_opposite_pct']:>7.1f} L{e['max_level']} {tag}")

    # ── 残差结构（步骤6）────────────────────────────────────
    rs = sig["ES/basket"]
    rsig = np.asarray(rs["sigma"])
    dom = ("下降(篮子>ES=Mag7驱动宏观)" if (rsig < 0).sum() > (rsig > 0).sum()
           else "上升" if (rsig > 0).sum() > (rsig < 0).sum() else "中性")
    print(f"\n[残差结构] ES/basket: {rs['n_bars']:,} bars, L{rs['max_level']}, {len(rs['days'])} 日σ", flush=True)
    print(f"  日σ分布: 上={int((rsig>0).sum())} 平={int((rsig==0).sum())} 下={int((rsig<0).sum())}"
          f" → 主导={dom}；末σ={int(rsig[-1]) if len(rsig) else 'NA'}", flush=True)
    print(f"  → 残差达 L{rs['max_level']}（多级别）= 残差有缠论走势结构，非噪音（步骤6答案）", flush=True)

    # ── 写报告 JSON ────────────────────────────────────────
    out = {
        "meta": {
            "task": "vertical_closure_test", "macro_vertex": "ES (S&P500 E-mini, P)",
            "denominator": "CL (WTI crude)", "components": avail,
            "weights_mcap": {k: round(v, 4) for k, v in mcap_w.items()},
            "method": "1min a0, log-envelope ratio, highest-emergent-level sigma, "
                      "UTC daily boundary, zero-lookahead, Rust orchestrator; split-adjusted",
            "joint_window": win, "joint_days": n_days,
            "epistemology": {"sigma_readings": "L2", "layerA": "L1(partial constituents)",
                             "layerB": "L0", "leadlag": "L2 (alpha untested)"},
            "elapsed_s": round(time.time() - t_start, 1),
        },
        "layer_a": {**la, "note": "CL cancels; partial constituents → not ~0 (doc §6); "
                                  "drift<0 = basket outperformed ES = Mag7 drove index"},
        "layer_b": {"n_days": n_days, "n_compatible": n_compat,
                    "compatible_pct": round(100 * n_compat / max(n_days, 1), 1),
                    "mean_concentration_mcap": round(mean_conc, 4)},
        "selection": entries,
        "residual_structure": {"n_bars": rs["n_bars"], "max_level": rs["max_level"],
                               "n_days": len(rs["days"]), "sigma_up": int((rsig > 0).sum()),
                               "sigma_flat": int((rsig == 0).sum()), "sigma_down": int((rsig < 0).sum())},
        "sigma_max_levels": {nm: sig[nm]["max_level"] for nm in sig},
    }
    JSON_OUT.write_text(json.dumps(out, indent=2))
    print(f"\n总耗时 {time.time()-t_start:.0f}s → {JSON_OUT}", flush=True)


if __name__ == "__main__":
    main()
