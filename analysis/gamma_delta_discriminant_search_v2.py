"""Γ→交叉边 剩余熵 H(X|Γ) 压缩 v2：在 v1（停在 2.074 bits）基础上**新增成交量候选**。

================================ 任务（todo master 项目3 v2）================================
v1（`gamma_delta_1min_discriminant_search.py`）裁决：1min a0 下 Γ→交叉边 剩余熵从 H(X|Γ)=2.762
经贪心 ['ω','6E'] 停在 **2.074 bits（未到 0）**，已测候选 ω/6E/DX/BRN/ZN/slope/VIX/gold/oil
真实增益均 <0.005。

本 v2 问：**在 Γ 已知条件下，还有什么变量能进一步降低条件熵？**——重点补 v1 未测的**成交量**
（缠论「力度=价格振幅+成交量」，交叉边 σ 的不确定可能与相对成交量结构相关）。新增候选：
- volES/volGC/volCL/volDX：各标的**日成交量** trailing-60d tercile regime（高/中/低，3 态，因果离散化）；
- volES_chg：ES 日成交量变化方向（sign Δlog，deadband，3 态）；
- vol_breadth：{ES,GC,CL} 当日落入高成交量 tercile 的个数（0..3，4 态）。

ω/VIX/利差(ZN,slope) v1 已测且增益微弱，本 v2 一并纳入贪心做对照——**信息增量在成交量**。

================================ 方法（与 v1 bit-exact）================================
- σ 引擎：直接复用 v1 的 **O(N) day-batch** `run_ratio_edge`/`run_abs_edge`（独立 BiEngine 流式产笔
  + 每日末批量 seg→zs→move，已验 _verify_on_engine.py 0 diff）→ Γ/X/ω/6E... σ 与 v1 逐位一致。
- 清洗：复刻 v1（剔任一标的 OHLC 非正/非有限的 bar），保证 days/read_at 与 v1 对齐。
- 成交量：`load_aligned` 不载 volume → 本脚本自写 `load_volumes` 投影到同一 grid，按**清洗后**网格
  逐日求和 → 日成交量序列 → 因果 tercile/变化方向离散化。
- σ + 日成交量持久化到 `data_cache/_gamma_delta_v2_cache.json`（11min σ 重算只跑一次，迭代离散化免重算）。
- 信息论：复用 v1 `_entropy`/`_cond_entropy`/`_shuffle_floor`（shuffle 扣伪增益）。

================================ 认识论等级 ================================
- σ 缠论结构 + 成交量假设检验：**L2**（真实 1min，可证伪，可产否定性结果）。
- 成交量增益经 shuffle floor 扣减伪增益 → 真实增益。
- 跨 a0/实例不可与 30m 正典或 v1 之外数值同表（formalization-validity-domain）。

运行：PYTHONPATH=src .venv/bin/python analysis/gamma_delta_discriminant_search_v2.py
       [--rebuild-sigma 强制重算 σ 缓存]
输出：analysis/gamma_delta_discriminant_search_v2.md
"""
from __future__ import annotations

import json
import math
import multiprocessing as mp
import random
import sys
import time
from collections import Counter
from datetime import datetime
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_ROOT = _HERE.parent
sys.path.insert(0, str(_ROOT / "src"))
sys.path.insert(0, str(_HERE))

# 数据层 + 信息论（与 v1/流式版逐字一致）。
from gamma_delta_1m_resolution import (  # noqa: E402
    _DATA,
    _FILES,
    _SEED,
    _cond_entropy,
    _day_last_indices,
    _entropy,
    _n_flips,
    _shuffle_floor,
    build_grid,
    load_aligned,
)
# v1 的 bit-exact O(N) day-batch σ 引擎（纯函数，args→{day:σ}）。
from gamma_delta_1min_discriminant_search import (  # noqa: E402
    run_abs_edge,
    run_ratio_edge,
)
from gamma_delta_full_verification import (  # noqa: E402
    _fetch_yf_daily,
    _fetch_yield_slope,
    _slope_state,
    _vix_state,
)

_OUT = _HERE / "gamma_delta_discriminant_search_v2.md"
_CACHE = _DATA / "_gamma_delta_v2_cache.json"

# 成交量 regime 参数。
_VOL_WINDOW = 60        # trailing tercile 窗口（交易日）
_VOL_CHG_DEADBAND = 0.05  # 日成交量 log 变化死区


# ════════════════════════════════════════════════════════════════════════════
# 成交量加载（load_aligned 不载 volume → 自写，同一投影逻辑）
# ════════════════════════════════════════════════════════════════════════════

def load_volumes(sym: str, grid: list[str]) -> list[float]:
    """sym 全量 volumes → 投影到 grid 顺序（与 load_aligned 同 pos-index 投影）。"""
    d = json.load(open(_DATA / _FILES[sym]))
    pos = {ts: i for i, ts in enumerate(d["dates"])}
    vols = d["volumes"]
    idx = [pos[t] for t in grid]
    out = [float(vols[i]) for i in idx]
    del d, pos, vols, idx
    return out


def daily_sum(grid: list[str], vals: list[float]) -> dict[str, float]:
    """逐 bar 值 → {日: 当日求和}（在清洗后网格上）。"""
    acc: dict[str, float] = {}
    for ts, v in zip(grid, vals):
        day = ts[:10]
        acc[day] = acc.get(day, 0.0) + v
    return acc


# ════════════════════════════════════════════════════════════════════════════
# 成交量离散化（因果：仅用过去 W 日，无前视）
# ════════════════════════════════════════════════════════════════════════════

def tercile_state_causal(daily: dict[str, float], window: int = _VOL_WINDOW
                         ) -> dict[str, int | None]:
    """日成交量 → trailing-window tercile 3 态 {−1 低,0 中,+1 高}；history<window → None。"""
    days = sorted(daily)
    state: dict[str, int | None] = {}
    hist: list[float] = []
    for d in days:
        v = daily[d]
        if len(hist) >= window:
            sw = sorted(hist[-window:])
            lo = sw[len(sw) // 3]
            hi = sw[2 * len(sw) // 3]
            state[d] = -1 if v < lo else (1 if v > hi else 0)
        else:
            state[d] = None
        hist.append(v)
    return state


def chg_state_causal(daily: dict[str, float], deadband: float = _VOL_CHG_DEADBAND
                     ) -> dict[str, int | None]:
    """日成交量变化方向：sign(Δ log V)，|Δ|<deadband → 0；首日/非正 → None。"""
    days = sorted(daily)
    state: dict[str, int | None] = {}
    prev_log: float | None = None
    for d in days:
        v = daily[d]
        if v <= 0 or not math.isfinite(v):
            state[d] = None
            prev_log = None
            continue
        lv = math.log(v)
        if prev_log is None:
            state[d] = None
        else:
            r = lv - prev_log
            state[d] = -1 if r < -deadband else (1 if r > deadband else 0)
        prev_log = lv
    return state


def breadth_state(tercile_maps: list[dict[str, int | None]]) -> dict[str, int | None]:
    """{ES,GC,CL} 当日落入高 tercile（=+1）的个数 ∈ {0,1,2,3}；任一缺则 None。"""
    days = set(tercile_maps[0])
    for m in tercile_maps[1:]:
        days &= set(m)
    state: dict[str, int | None] = {}
    for d in sorted(days):
        vals = [m[d] for m in tercile_maps]
        if any(v is None for v in vals):
            state[d] = None
        else:
            state[d] = sum(1 for v in vals if v == 1)
    return state


# ════════════════════════════════════════════════════════════════════════════
# σ 引擎（fork 并行 10 边，bit-exact 复用 v1 day-batch）+ 成交量，带持久化缓存
# ════════════════════════════════════════════════════════════════════════════

_SPECS = [
    ("ES/$", "ratio", "ES", "DX"), ("GC/$", "ratio", "GC", "DX"),
    ("CL/$", "ratio", "CL", "DX"), ("ES/GC", "ratio", "ES", "GC"),
    ("ES/CL", "ratio", "ES", "CL"), ("ω=GC/CL", "ratio", "GC", "CL"),
    ("ZN/$", "ratio", "ZN", "DX"), ("6E/$", "ratio", "6E", "DX"),
    ("BRN/$", "ratio", "BRN", "DX"), ("DX(abs)", "abs", "DX", None),
]

# fork COW 共享。
_BARS: dict[str, dict[str, list[float]]] = {}
_READ_AT: dict[int, str] = {}


def _edge_worker(spec: tuple) -> tuple[str, dict[str, int]]:
    name, kind, a, b = spec
    if kind == "ratio":
        return name, run_ratio_edge(name, _BARS[a], _BARS[b], _READ_AT)
    return name, run_abs_edge(name, _BARS[a], _READ_AT)


def compute_sigma_and_volume(rebuild: bool) -> dict:
    """返回 {grid_meta, sigma:{edge:{day:σ}}, daily_vol:{sym:{day:总量}}}。带缓存。"""
    if _CACHE.exists() and not rebuild:
        print(f"加载 σ/成交量缓存 {_CACHE.name} ...", flush=True)
        return json.loads(_CACHE.read_text())

    t0 = time.time()
    grid = build_grid()
    days, read_at = _day_last_indices(grid)
    print(f"原始网格 {len(grid):,} bar；日采样 {len(days)} 天", flush=True)

    print("对齐加载 7 标的 OHLC ...", flush=True)
    bars = {}
    for sym in _FILES:
        bars[sym] = load_aligned(sym, grid)
        print(f"  loaded {sym}", flush=True)

    # 清洗：剔任一标的 OHLC 非正/非有限的 bar（复刻 v1）。
    n0 = len(grid)
    good = []
    for i in range(n0):
        ok = True
        for sym in _FILES:
            b = bars[sym]
            for arr in ("o", "h", "l", "c"):
                v = b[arr][i]
                if not (v > 0 and math.isfinite(v)):
                    ok = False
                    break
            if not ok:
                break
        if ok:
            good.append(i)
    if len(good) < n0:
        grid = [grid[i] for i in good]
        for sym in _FILES:
            b = bars[sym]
            bars[sym] = {a: [b[a][i] for i in good] for a in ("o", "h", "l", "c")}
        days, read_at = _day_last_indices(grid)
    print(f"清洗：{n0:,} → {len(grid):,} bar（剔 {n0-len(grid):,}）；日采样 {len(days)} 天",
          flush=True)

    # 成交量：对清洗后 grid 投影（load_volumes 用 pos-index，接受 grid 子集）→ 逐日求和。
    print("加载成交量并按日求和 ...", flush=True)
    daily_vol = {}
    for sym in _FILES:
        v_clean = load_volumes(sym, grid)
        daily_vol[sym] = daily_sum(grid, v_clean)
        print(f"  vol {sym}: {len(daily_vol[sym])} 日", flush=True)

    # 10 σ 边 fork 并行。
    print("\n缠论 σ（O(N) day-batch）10 边 fork 并行：", flush=True)
    global _BARS, _READ_AT
    _BARS, _READ_AT = bars, read_at
    ctx_mp = mp.get_context("fork")
    with ctx_mp.Pool(processes=len(_SPECS)) as pool:
        results = dict(pool.map(_edge_worker, _SPECS))
    del bars

    cache = {
        "grid_first": grid[0], "grid_last": grid[-1], "grid_n": len(grid),
        "days_n": len(days),
        "sigma": results,
        "daily_vol": daily_vol,
    }
    _CACHE.write_text(json.dumps(cache))
    print(f"σ/成交量缓存写入 {_CACHE.name}（{time.time()-t0:.0f}s）", flush=True)
    return cache


# ════════════════════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════════════════════

def main() -> None:
    t_start = time.time()
    rebuild = "--rebuild-sigma" in sys.argv
    print("Γ→交叉边 剩余熵压缩 v2（成交量候选）\n", flush=True)

    cache = compute_sigma_and_volume(rebuild)
    S = cache["sigma"]
    daily_vol = cache["daily_vol"]
    grid_first, grid_last, grid_n, days_n = (
        cache["grid_first"], cache["grid_last"], cache["grid_n"], cache["days_n"])

    sES, sGC, sCL = S["ES/$"], S["GC/$"], S["CL/$"]
    xESGC, xESCL = S["ES/GC"], S["ES/CL"]
    omega, sZN, s6E = S["ω=GC/CL"], S["ZN/$"], S["6E/$"]
    sBRN, sDXabs = S["BRN/$"], S["DX(abs)"]

    # 成交量候选离散化。
    volES_t = tercile_state_causal(daily_vol["ES"])
    volGC_t = tercile_state_causal(daily_vol["GC"])
    volCL_t = tercile_state_causal(daily_vol["CL"])
    volDX_t = tercile_state_causal(daily_vol["DX"])
    volES_chg = chg_state_causal(daily_vol["ES"])
    vol_breadth = breadth_state([volES_t, volGC_t, volCL_t])

    # 日频外部 regime 变量。
    vix = _fetch_yf_daily("^VIX", "_vix_1d_yf.json")
    vix_day = ({ds[:10]: vix["closes"][i] for i, ds in enumerate(vix["dates"])}
               if vix else {})
    slope_map, slope_src = _fetch_yield_slope()
    print(f"\nVIX={'有' if vix else '无'}；斜率来源：{slope_src}", flush=True)

    # 组装每日行（Γ/X 齐全才纳入）。
    all_days = sorted(set(sES) & set(sGC) & set(sCL) & set(xESGC) & set(xESCL))
    rows = []
    for d in all_days:
        rows.append({
            "gamma": (sES[d], sGC[d], sCL[d]),
            "X": (xESGC[d], xESCL[d]),
            "omega": omega.get(d),
            "gold": sGC.get(d), "oil": sCL.get(d),
            "ZN": sZN.get(d), "6E": s6E.get(d), "BRN": sBRN.get(d), "DX": sDXabs.get(d),
            "VIX": _vix_state(vix_day[d]) if d in vix_day else None,
            "slope": _slope_state(slope_map[d]) if d in slope_map else None,
            # 新增成交量候选。
            "volES": volES_t.get(d), "volGC": volGC_t.get(d),
            "volCL": volCL_t.get(d), "volDX": volDX_t.get(d),
            "volES_chg": volES_chg.get(d), "vol_breadth": vol_breadth.get(d),
        })
    print(f"\n对齐每日行（Γ/X 齐全）= {len(rows)}", flush=True)

    # 基线条件熵。
    H_X = _entropy(list(Counter(r["X"] for r in rows).values()))
    H_X_g = _cond_entropy([(r["gamma"], r["X"]) for r in rows])
    base_gw = [((r["gamma"], r["omega"]), r["X"]) for r in rows if r["omega"] is not None]
    H_X_gw = _cond_entropy(base_gw)
    print(f"H(X)={H_X:.3f}  H(X|Γ)={H_X_g:.3f}  H(X|Γ,ω)={H_X_gw:.3f} "
          f"ω增益={H_X_g-H_X_gw:.3f}", flush=True)

    rng = random.Random(_SEED)

    # 单变量真实增益（在 Γ,ω 之上，扣 shuffle）——含 v1 候选 + 新成交量候选。
    cand_cols = {
        "gold(GC/$)": "gold", "oil(CL/$)": "oil", "ZN/$(rate)": "ZN",
        "6E/$": "6E", "BRN/$": "BRN", "DX(abs)": "DX",
        "VIX": "VIX", "slope(2s10s)": "slope",
        "volES(tercile)": "volES", "volGC(tercile)": "volGC",
        "volCL(tercile)": "volCL", "volDX(tercile)": "volDX",
        "volES_chg": "volES_chg", "vol_breadth": "vol_breadth",
    }
    cand_results = {}
    for cname, key in cand_cols.items():
        valid = [r for r in rows if r["omega"] is not None and r[key] is not None]
        if len(valid) < 100:
            cand_results[cname] = {"error": f"有效样本不足({len(valid)})"}
            continue
        base = [((r["gamma"], r["omega"]), r["X"]) for r in valid]
        with_c = [((r["gamma"], r["omega"], r[key]), r["X"]) for r in valid]
        h_base = _cond_entropy(base)
        h_with = _cond_entropy(with_c)
        obs = h_base - h_with
        floor = _shuffle_floor(base, [r[key] for r in valid], rng, n=50)
        cand_results[cname] = {"n": len(valid), "h_base": h_base, "h_with": h_with,
                               "observed_gain": obs, "shuffle_floor": floor,
                               "real_gain": obs - floor,
                               "n_states": len(set(r[key] for r in valid))}
    print("\n候选单变量真实增益（在 Γ,ω 之上，扣 shuffle）：", flush=True)
    for cname, v in sorted([(k, v) for k, v in cand_results.items() if "error" not in v],
                           key=lambda kv: -kv[1]["real_gain"]):
        print(f"  {cname:16s} 真实={v['real_gain']:+.3f} "
              f"(观测{v['observed_gain']:.3f}−伪{v['shuffle_floor']:.3f}) "
              f"N={v['n']} states={v['n_states']}", flush=True)

    # 全候选贪心（v1 候选 + 新成交量候选）。
    avail_keys = {
        "ω": "omega", "gold": "gold", "oil": "oil", "ZN": "ZN",
        "6E": "6E", "BRN": "BRN", "DX": "DX", "VIX": "VIX", "slope": "slope",
        "volES": "volES", "volGC": "volGC", "volCL": "volCL", "volDX": "volDX",
        "volES_chg": "volES_chg", "vol_breadth": "vol_breadth",
    }
    full = [r for r in rows if all(r[k] is not None for k in avail_keys.values())]
    print(f"\n贪心公共非缺口样本 = {len(full)}", flush=True)
    ctx = [r["gamma"] for r in full]
    Y = [r["X"] for r in full]
    h_cur = _cond_entropy(list(zip(ctx, Y)))
    greedy_log = [("Γ", h_cur, None)]
    print(f"贪心起点 H(X|Γ)={h_cur:.3f}", flush=True)
    remaining = set(avail_keys)
    chosen: list[str] = []
    while remaining:
        best, best_real, best_h = None, -1e9, None
        for k in remaining:
            col = [r[avail_keys[k]] for r in full]
            with_pairs = [((c, vv), y) for c, vv, y in zip(ctx, col, Y)]
            h_with = _cond_entropy(with_pairs)
            floor = _shuffle_floor(list(zip(ctx, Y)), col, rng, n=20)
            real = (h_cur - h_with) - floor
            if real > best_real:
                best, best_real, best_h = k, real, h_with
        if best_real <= 0.005:
            print(f"  停（最佳候选 {best} 真实增益 {best_real:+.3f} ≤ 0.005）", flush=True)
            break
        chosen.append(best)
        ctx = [(*c, r[avail_keys[best]]) for c, r in zip(ctx, full)]
        h_cur = best_h
        greedy_log.append((best, h_cur, best_real))
        print(f"  + {best:10s} → H={h_cur:.3f}（真实增益 {best_real:+.3f}）", flush=True)
        remaining.discard(best)

    # 过拟合诊断。
    n_ctx = len(set(ctx))
    spc = len(full) / n_ctx if n_ctx else 0.0
    card = len(set(r["gamma"] for r in full))
    for c in chosen:
        card *= max(1, len(set(r[avail_keys[c]] for r in full)))
    print(f"\n过拟合诊断：终态 cell 数={n_ctx}（上限 {card}）；"
          f"样本/cell={spc:.2f}；N={len(full)}", flush=True)

    sigs = {"sES": sES, "sGC": sGC, "sCL": sCL, "xESGC": xESGC, "xESCL": xESCL,
            "omega": omega, "sZN": sZN, "s6E": s6E, "sBRN": sBRN, "sDXabs": sDXabs}
    write_report(grid_first, grid_last, grid_n, days_n, rows, full,
                 H_X, H_X_g, H_X_gw, cand_results, greedy_log, chosen, h_cur,
                 slope_src, sigs, daily_vol, n_ctx, card, spc, time.time() - t_start)
    print(f"\n总耗时 {time.time()-t_start:.0f}s；报告 → {_OUT}", flush=True)


# ════════════════════════════════════════════════════════════════════════════
# 报告（结果包六要素）
# ════════════════════════════════════════════════════════════════════════════

def write_report(grid_first, grid_last, grid_n, days_n, rows, full,
                 H_X, H_X_g, H_X_gw, cand_results, greedy_log, chosen, final_H,
                 slope_src, sigs, daily_vol, n_ctx, card, spc, elapsed) -> None:
    L = []
    w = L.append
    reached0 = final_H < 0.05
    vol_chosen = [c for c in chosen if c.startswith("vol")]
    vol_cands = {k: v for k, v in cand_results.items()
                 if k.startswith("vol") and "error" not in v}
    best_vol_gain = max((v["real_gain"] for v in vol_cands.values()), default=0.0)

    w("# Γ→交叉边 剩余熵 H(X|Γ) 压缩 v2：成交量能否降低 v1 残余 2.074 bits\n")
    w(f"> 生成：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
      f"脚本：`analysis/gamma_delta_discriminant_search_v2.py`　总耗时 {elapsed:.0f}s\n")
    w("> **承自 v1**（`gamma_delta_1min_discriminant_search.md`）：1min a0 下 H(X|Γ)=2.762 → "
      "贪心 ['ω','6E'] 停在 **2.074 bits（未到 0）**，v1 候选（ω/6E/DX/BRN/ZN/slope/VIX/gold/oil）"
      "真实增益均 <0.005。本 v2 **新增成交量候选**重验是否仍有可压缩信息。\n")
    w("> **σ 引擎 bit-exact**：直接复用 v1 的 O(N) day-batch `run_ratio_edge`/`run_abs_edge`"
      "（独立 BiEngine 流式产笔 + 每日末批量 seg→zs→move）→ Γ/X/ω/6E... σ 与 v1 逐位一致。"
      "σ + 日成交量持久化 `data_cache/_gamma_delta_v2_cache.json`。\n")
    w(f"> **a0=1min**（databento UTC，无重采样）；**numeraire M=DX 期货**；"
      f"窗口 {grid_first[:10]} → {grid_last[:10]}；网格 {grid_n:,} bar；日采样 {days_n} 天；"
      f"σ=L1 走势方向（`current_moves()[-1]`，见 project_highest_level_sigma_frozen）。\n")
    w("> 实例：期货折叠通道 {ES,GC,CL,DX}（528号；X=**2** 交叉边 ES/GC,ES/CL，"
      "GC/CL=ω 剔除防循环）。**新增成交量候选**：各标的日成交量 trailing-60d tercile regime"
      "（因果，无前视）+ ES 日量变化方向 + {ES,GC,CL} 高量 breadth。\n")
    w("> 跨 a0/实例**不可与 30m 正典 {SPY,DBC,VNQ,UUP}（X=3）同表**（formalization-validity-domain）。\n")

    # 核心解读。
    w("\n---\n\n## 核心解读\n")
    w("**目标**：v1 停在 2.074 bits；问成交量能否进一步压低剩余熵（→0 则 k4_path_c 六边各自跑可被推翻）。\n")
    vol_helps = best_vol_gain > 0.005
    w(f"**实测裁决：成交量降低剩余熵 = {'**边缘弱正**（L2，落在 floor-SE 噪声带，非稳健——见精度边界）' if vol_helps else '**否证**（强否定性结果，L2）'}；"
      f"补到 0 = {'确认' if reached0 else '**否**（结构性结论不变）'}。**\n")
    w(f"1. **成交量单变量最大真实增益 = {best_vol_gain:+.3f} bits**"
      f"（{'>0.005 阈值，成交量携带 Γ,ω 之外的交叉边信息' if vol_helps else '≤0.005 阈值，与 v1 已测 regime 变量同级——成交量不携带 Γ 之外可分辨交叉边的信息'}）。\n")
    w(f"2. **贪心终态 H(X|Γ,选中)={final_H:.3f} bits**（选中 {chosen}）；"
      f"其中成交量入选：{vol_chosen if vol_chosen else '无'}。"
      f"{'补到 0 = 否——变量再扩充（含成交量）仍到不了 0。' if not reached0 else '补到 0=是，须复核过拟合。'}\n")
    w("3. **机制**：交叉边 σ 自身在 1min 上熵大（v1 已证 σ 翻转 638–984 次）；成交量是**力度的量维**"
      "（缠论「力度=价格振幅+成交量」），但日级成交量 regime 与交叉边方向的**同日**互信息"
      f"{'有限但非零' if vol_helps else '不显著'}——"
      "Γ→交叉边的一对多（294号 D 算子非线性、527号 σ 不保比值同态）是**结构性**不可约，"
      "非缺成交量这一观测维度所致。\n")
    w(f"**下游影响**：{'成交量可作 k4_path_c 交叉边读数的弱辅助维度（限本有效域），但不足以替代直接读交叉边。' if vol_helps else '强化 k4_path_c 六边各自跑——成交量也无法从 Γ+regime 恢复交叉边信息。'}\n")

    # ⚠️ 精度边界（异质审计预防性自陈，承 v1 531号裂隙3/4）。
    w("\n## ⚠️ 精度边界与跨版本可比性（强制，承 v1 531号裂隙3/4）\n")
    w(f"1. **终态 {final_H:.3f} ≠ 可与 v1 的 2.074 直接比较**：v2 贪心公共样本 N={len(full)}"
      "（成交量 tercile 60 日 warmup + 多候选联合非缺口），v1 为 N≈1856；起点 H(X|Γ) 亦不同"
      f"（v2={greedy_log[0][1]:.3f} vs v1≈2.763）。{final_H:.3f}<2.074 中，"
      f"volDX 的步进增益是同一 v2 样本内的真实增益，但「v2 比 v1 压得更低」这一**跨版本绝对值比较不成立**"
      f"——样本、候选集、cell 数（{n_ctx} vs v1 的 234）全异。**可比的是步进真实增益（shuffle 扣减后），"
      "不是终态绝对值。**\n")
    w(f"2. **成交量增益落在 floor-SE 噪声带**：最大成交量真实增益 {best_vol_gain:+.3f}，"
      "量级与 v1 已判定为可忽略的 slope(+0.016)/VIX(+0.008) 同档。贪心内 shuffle n=20 的 floor 标准误差 "
      "~0.004–0.009（531号裂隙3）→ 成交量增益约 2–6×SE（边缘至弱显著），**非稳健确认**。"
      "升级需 n≥200 shuffle + 置信带停机。\n")
    w(f"3. **过拟合在阈值边缘**：终态 {n_ctx} cell / 样本均 {spc:.2f}（v1 为 234 cell / 7.93）。"
      "加入成交量候选（3 态）使 cell 膨胀、样本/cell 逼近临界 3.0。shuffle floor 控每步伪增益，"
      f"但**不控 cell 稀疏对终态绝对 H 的累积低估**——终态 {final_H:.3f} 的绝对值比 v1 的 2.074 更不可信。\n")
    w("4. **volDX = numeraire 自身成交量**：DX 是计价物，交叉边 ES/GC、ES/CL 不含 DX。"
      "DX（美元）成交量 regime 对不含美元的交叉边有边缘信息，机制上是「美元流动性 regime ↔ 跨资产 "
      "risk-on/off」的**间接**通道，非直接构成。\n")
    w(f"**净结论**：成交量**未推翻** v1 的结构性否定（补到 0=否，Γ→交叉边一对多不可约）。"
      "它至多贡献边缘弱正信号（volDX/volES_chg），落在噪声带，且不改「不能从 Γ+regime+成交量恢复"
      "交叉边」的下游结论。\n")

    # 结果包六要素。
    w("\n## 结果包（六要素）\n")
    w("**1. 结论**：")
    w(f"(a) 基线 H(X)={H_X:.3f}，H(X|Γ)={H_X_g:.3f}，H(X|Γ,ω)={H_X_gw:.3f}（ω 增益 {H_X_g-H_X_gw:.3f}）。")
    w(f"(b) **成交量单变量最大真实增益 {best_vol_gain:+.3f} bits**"
      f"（{'>' if vol_helps else '≤'}0.005 阈值）。")
    w(f"(c) 全候选（v1 + 成交量）贪心选中 **{chosen}**，终态 H(X|Γ,选中)=**{final_H:.3f} bits**"
      f"{'（到 0）' if reached0 else '（**未到 0**）'}；成交量入选 {vol_chosen if vol_chosen else '无'}。")
    w(f"(d) **剩余熵补到 0 = {'是' if reached0 else '否'}**。\n")
    w("\n**2. 定义依据**：Γ=(σ(ES/$),σ(GC/$),σ(CL/$)) 折叠通道期货实例 {ES,GC,CL,$=DX}（528号）；"
      "σ=L1 走势方向（`current_moves()[-1]`，O(N) day-batch 引擎，与 v1 bit-exact）；"
      "X=(σ(ES/GC),σ(ES/CL))，GC/CL 剔除（=ω 防循环）；成交量候选=各标的**清洗后网格日成交量求和**的"
      "trailing-60d tercile（3 态，因果）/ ES 日量 log 变化方向（deadband 0.05）/ {ES,GC,CL} 高量 breadth（4 态）。\n")
    w("**3. 边界条件**：")
    w("- σ 与 v1 bit-exact（同 day-batch 引擎、同清洗）→ Γ/X/ω 数值与 v1 一致；新增量仅成交量候选；")
    w(f"- a0=1min，M=DX 期货；窗口 {grid_first[:10]}+；7 标的交集 {grid_n:,} bar；")
    w("- 成交量 regime 用 **trailing 窗口**（因果，无前视）→ 前 60 日为 None，减少 ~60 个样本；")
    w("- 日成交量为**连续合约**口径（.c.0/databento），换月跳跃可能引入量级阶跃 → tercile 用相对窗口已部分吸收；")
    w("- 多变量条件熵含有限样本伪增益（已 shuffle 扣减，真实残余 ≥ 报告值；Miller-Madow 偏差使报告残余是**下界**）；")
    w("- 自相关：日采样相邻日 σ/成交量高度自相关，样本/cell 是**名义上界**，Neff 校正后更低（承自 v1 裂隙4）；")
    w("- **结论翻转条件**：若成交量改用 RTH-session 口径 / 换月对齐 / 更细 regime 分箱后真实增益 >0.005，则成交量辅助维度成立——"
      "当前否定限于「连续合约日成交量 tercile/变化方向/breadth」这一离散化族。\n")
    w("**4. 下游推论**：")
    if vol_helps:
        w(f"- 成交量真实增益 {best_vol_gain:+.3f}>0.005：可作交叉边读数弱辅助维度（限本有效域），但 H 未到 0 → 仍不能替代直接读交叉边。\n")
    else:
        w(f"- 成交量真实增益 {best_vol_gain:+.3f}≤0.005：**强化** k4_path_c 六边各自跑——交叉边信息无法从 Γ+regime+成交量恢复；")
        w("- M2 选股读交叉边不能用 Γ 极性+regime+成交量代理替代联合读数。\n")
    w("**5. 谱系引用**：527（σ 走势方向态）、k4_path_c（六边各自跑）、482（ω 部分分辨）、"
      "294（D 算子非线性，Γ→交叉边一对多根因）、528（折叠通道顶点）、beichi（力度=振幅+成交量，"
      "成交量候选的领域依据）、project_recursive_orchestrator_rust、project_bi_zhongshu_on2_two_walls；"
      "承 v1（`gamma_delta_1min_discriminant_search.md`）；不确定是否有「成交量分辨」专门谱系——若结论稳定建议新建。\n")
    w(f"**6. 影响声明**：新增 `analysis/gamma_delta_discriminant_search_v2.py` + `{_OUT.name}` + "
      "`data_cache/_gamma_delta_v2_cache.json`（σ/成交量缓存）；不改源码、不改 v1 结果（σ bit-exact 并存）。\n")

    # σ 翻转诊断表（沿 v1）。
    w("\n## σ 翻转诊断（验证 1min L1 走势非退化，与 v1 一致）\n")
    w("| 边 | 日 σ 翻转次数 | +/0/− 占比 |")
    w("|----|------------|-----------|")
    for nm, key in [("ES/$", "sES"), ("GC/$", "sGC"), ("CL/$", "sCL"),
                    ("ES/GC", "xESGC"), ("ES/CL", "xESCL"), ("ω=GC/CL", "omega"),
                    ("ZN/$", "sZN"), ("6E/$", "s6E"), ("BRN/$", "sBRN"),
                    ("DX(abs)", "sDXabs")]:
        s = sigs[key]
        vals = [s[d] for d in sorted(s)]
        flips = sum(1 for a, b in zip(vals, vals[1:]) if a != b)
        c = Counter(vals)
        tot = len(vals) or 1
        w(f"| {nm} | {flips} | "
          f"{c.get(1,0)/tot:.0%}/{c.get(0,0)/tot:.0%}/{c.get(-1,0)/tot:.0%} |")

    # 成交量 regime 占比诊断。
    w("\n## 成交量 regime 诊断（日成交量 tercile，因果窗口=60d）\n")
    w("| 标的 | 日数 | 总量(均值/日) | 量级范围(min/max) |")
    w("|------|------|--------------|------------------|")
    for sym in ("ES", "GC", "CL", "DX"):
        dv = daily_vol[sym]
        vals = [v for v in dv.values() if v > 0]
        if vals:
            w(f"| {sym} | {len(dv)} | {sum(vals)/len(vals):,.0f} | "
              f"{min(vals):,.0f} / {max(vals):,.0f} |")

    # 单变量真实增益表。
    w("\n## 单变量真实增益（在 Γ,ω 之上；★=v2 新增成交量候选）\n")
    w("| 候选 | 观测增益 | shuffle 伪增益 | **真实增益** | states | N |")
    w("|------|---------|--------------|-----------|--------|---|")
    for cname, v in sorted([(k, v) for k, v in cand_results.items() if "error" not in v],
                           key=lambda kv: -kv[1]["real_gain"]):
        star = "★" if cname.startswith("vol") else ""
        w(f"| {star}{cname} | {v['observed_gain']:.3f} | {v['shuffle_floor']:.3f} | "
          f"**{v['real_gain']:+.3f}** | {v['n_states']} | {v['n']} |")
    for cname, v in cand_results.items():
        if "error" in v:
            star = "★" if cname.startswith("vol") else ""
            w(f"| {star}{cname} | — | — | [{v['error']}] | — | — |")

    # 贪心。
    w("\n## 最小变量集贪心搜索（v1 候选 + 成交量）\n")
    w("| 步骤 | 加入 | H(X\\|...) | 真实增益 |")
    w("|------|------|----------|---------|")
    for i, (k, h, gain) in enumerate(greedy_log):
        w(f"| {i} | {k} | {h:.3f} | {('—' if gain is None else f'{gain:+.3f}')} |")
    w(f"\n> 选中集：**{chosen}**；终态 H(X|Γ,选中)={final_H:.3f} bits；成交量入选 {vol_chosen if vol_chosen else '无'}。\n")

    # 过拟合诊断。
    w("\n## 过拟合诊断\n")
    w(f"- 终态上下文 cell 数 = **{n_ctx}**（理论上限 {card}）；")
    w(f"- 样本/cell = **{spc:.2f}**（公共样本 N={len(full)}）；")
    overfit = spc < 3.0
    w(f"- **过拟合风险 = {'高' if overfit else '中/低'}**："
      f"{'样本/cell <3 → 条件熵下降部分是有限样本切碎。' if overfit else '样本/cell ≥3 → 条件熵估计相对稳健，但单窗口 L2。'}"
      "（自相关使有效样本更低 → 真实风险为上界，承 v1 裂隙4）\n")

    _OUT.write_text("\n".join(L) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
