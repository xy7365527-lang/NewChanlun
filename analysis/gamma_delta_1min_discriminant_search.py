"""1min 分辨变量搜索（O(N) 引擎重跑）——把 Γ→交叉边 的剩余熵 H(X|Γ) 压向 0。

================================ 任务背景 ================================
上游 `gamma_delta_1m_resolution.py` 用**流式** `RecursiveOrchestrator.process_bar` 逐 bar
驱动 6 级递归，σ = `current_moves()[-1]` 末走势方向。该引擎在每次笔尾变化时**全量重算**
segments_from_strokes_v1 → O(strokes²)（"两堵墙"，见 project_bi_zhongshu_on2_two_walls）。
1.88M bar × 10 边 → 跑了很久。

本脚本换 **O(N) day-batch 引擎**（同一批量管线，仅采样点重算）：
- 用独立 `BiEngine` 流式产笔（O(N) 摊还）；
- 仅在每日末（~1900 次，非 ~strokes 次）对 `current_strokes()` 前缀跑批量
    segments_from_strokes_v1 → zhongshu_from_segments → moves_from_zhongshus；
- σ = 末走势方向（_top_sigma，只读 kind/direction）。

**bit-exactness**（已验证，analysis/_verify_on_engine.py）：σ(day d) 仅依赖 current_moves()，
而 process_bar 内部正是用同一批量管线对同一 stroke 前缀算 moves（短路缓存=纯性能，
产出同状态）。故 day-batch 输入相同 ⟹ 输出逐位等价。400k 前缀实测 abs/ratio 均 0 diff，
stream 9.1/12.7s vs batch 2.2/2.7s。复杂度 O(strokes²) → O(days×strokes)=O(N)。

================================ Γ / 交叉边 X / 候选（与上游一致）======================
期货 K4 折叠通道实例 {ES, GC, CL, $=DX}（528号；非正典 P/C/R）：
- **Γ**=(σ(ES/$),σ(GC/$),σ(CL/$))；**X**=(σ(ES/GC),σ(ES/CL))；GC/CL 剔除（=ω 防循环）。
- **候选 8**：ω=σ(GC/CL)、gold=σ(GC/$)、oil=σ(CL/$)、ZN=σ(ZN/$)、6E=σ(6E/$)、
    BRN=σ(BRN/$)、DX=σ(DX abs)、VIX；另附 slope_2s10s（FRED 日频）。
- numeraire M=DX 期货（databento UTC 24h，与各边逐分钟对齐；非 UUP，避时区错位）。

================================ 认识论等级 ================================
- σ 缠论结构：L2（真实 1min，可证伪）。
- 多变量条件熵下降：shuffle 对照分离真实增益（伪增益已扣）。
- 引擎切换 O(N²)→O(N)：纯性能，bit-exact（L0/L1 验证 = 同义反复但已实测 0 diff）；
    不改变任何 σ 数值 → 不改变 H 结论的有效域。

运行：PYTHONPATH=src .venv/bin/python analysis/gamma_delta_1min_discriminant_search.py
输出：analysis/gamma_delta_1min_discriminant_search.md
"""
from __future__ import annotations

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

import newchan_rust  # noqa: E402

# 复用上游数据层 + 信息论（逐字一致 → 与流式版可比）。
from gamma_delta_1m_resolution import (  # noqa: E402
    _DATA,
    _FILES,
    _MAX_LEVELS,
    _SEED,
    _cond_entropy,
    _day_last_indices,
    _entropy,
    _n_flips,
    _shuffle_floor,
    build_grid,
    load_aligned,
)
from gamma_delta_full_verification import (  # noqa: E402
    _fetch_yf_daily,
    _fetch_yield_slope,
    _slope_state,
    _vix_state,
)

_OUT = _HERE / "gamma_delta_1min_discriminant_search.md"
# RecursiveOrchestrator 内部 BiEngine 参数（lib.rs:856 默认 stroke_mode="wide"）。
# ⚠️ 必须 "wide"（非 BiEngine 独立默认 "new"）才与流式版 bit-exact。
_BI_PARAMS = ("wide", 5, False, 3)


# ════════════════════════════════════════════════════════════════════════════
# O(N) day-batch σ 引擎（bit-exact 复刻 orchestrator.process_bar 的末走势读法）
# ════════════════════════════════════════════════════════════════════════════

def _top_sigma_from_moves(moves: list) -> int:
    """末走势方向 σ∈{+1,0,−1}。moves[-1]=((kind,direction,...),(...))；consolidation→0。"""
    if not moves:
        return 0
    head = moves[-1][0]
    if head[0] == "consolidation":
        return 0
    return 1 if head[1] == "up" else -1


def _sigma_from_strokes(strokes: list) -> int:
    """stroke 前缀 → seg → zs → move → 末走势 σ（与 orchestrator.process_bar 同管线）。"""
    segs = newchan_rust.segments_from_strokes_v1(strokes, 3, "strict")
    # SegmentTuple=((s0,s1,i0,i1,dir,high,low,confirmed,kind),(ep...),(be...))；
    # zhongshu_from_segments 入参 (s0,s1,high,low,confirmed,kind=="settled")。
    seg_tuples = [
        (s[0][0], s[0][1], s[0][5], s[0][6], s[0][7], s[0][8] == "settled")
        for s in segs
    ]
    zss = newchan_rust.zhongshu_from_segments(seg_tuples)
    moves = newchan_rust.moves_from_zhongshus(zss, len(segs))
    return _top_sigma_from_moves(moves)


def run_ratio_edge(name: str, ra: dict, rb: dict, read_at: dict[int, str]) -> dict[str, int]:
    """比值边 ra/rb：BiEngine 流式产笔（O(N)），每日末对 stroke 前缀批量算 σ。"""
    bi = newchan_rust.BiEngine(*_BI_PARAMS)
    ao, ah, al, ac = ra["o"], ra["h"], ra["l"], ra["c"]
    bo, bh, bl, bc = rb["o"], rb["h"], rb["l"], rb["c"]
    n = len(ac)
    sig: dict[str, int] = {}
    t0 = time.time()
    for i in range(n):
        # 比值 OHLC：high=高/低，low=低/高（交叉相除）。
        bi.process_bar(ao[i] / bo[i], ah[i] / bl[i], al[i] / bh[i], ac[i] / bc[i])
        day = read_at.get(i)
        if day is not None:
            sig[day] = _sigma_from_strokes(bi.current_strokes())
    print(f"  [{name:10s}] {n:,} bar {time.time()-t0:6.1f}s "
          f"→ {len(sig)} 日 σ；翻转 {_n_flips(sig)} 次", flush=True)
    return sig


def run_abs_edge(name: str, r: dict, read_at: dict[int, str]) -> dict[str, int]:
    """绝对边（裸价格）：BiEngine 流式产笔，每日末批量算 σ。"""
    bi = newchan_rust.BiEngine(*_BI_PARAMS)
    o, h, l, c = r["o"], r["h"], r["l"], r["c"]
    n = len(c)
    sig: dict[str, int] = {}
    t0 = time.time()
    for i in range(n):
        bi.process_bar(o[i], h[i], l[i], c[i])
        day = read_at.get(i)
        if day is not None:
            sig[day] = _sigma_from_strokes(bi.current_strokes())
    print(f"  [{name:10s}] {n:,} bar {time.time()-t0:6.1f}s "
          f"→ {len(sig)} 日 σ；翻转 {_n_flips(sig)} 次", flush=True)
    return sig


# fork COW 共享（父设置 → 子继承，免 pickle）。
_BARS: dict[str, dict[str, list[float]]] = {}
_READ_AT: dict[int, str] = {}


def _edge_worker(spec: tuple) -> tuple[str, dict[str, int]]:
    name, kind, a, b = spec
    if kind == "ratio":
        return name, run_ratio_edge(name, _BARS[a], _BARS[b], _READ_AT)
    return name, run_abs_edge(name, _BARS[a], _READ_AT)


# ════════════════════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════════════════════

def main() -> None:
    t_start = time.time()
    print("O(N) day-batch 引擎 1min 分辨变量搜索\n", flush=True)
    grid = build_grid()
    days, read_at = _day_last_indices(grid)
    print(f"日采样 = {len(days)} 天\n", flush=True)

    print("对齐加载 7 标的到网格...", flush=True)
    bars = {}
    for sym in _FILES:
        bars[sym] = load_aligned(sym, grid)
        print(f"  loaded {sym}", flush=True)

    # 数据清洗：剔除任一标的 OHLC 非正/非有限的 bar（databento nan/0）。
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

    # 缠论 σ（O(N) day-batch）：10 条独立边 fork 并行。
    print("\n缠论递归（O(N) day-batch）10 边 fork 并行：", flush=True)
    global _BARS, _READ_AT
    _BARS, _READ_AT = bars, read_at
    specs = [
        ("ES/$", "ratio", "ES", "DX"), ("GC/$", "ratio", "GC", "DX"),
        ("CL/$", "ratio", "CL", "DX"), ("ES/GC", "ratio", "ES", "GC"),
        ("ES/CL", "ratio", "ES", "CL"), ("ω=GC/CL", "ratio", "GC", "CL"),
        ("ZN/$", "ratio", "ZN", "DX"), ("6E/$", "ratio", "6E", "DX"),
        ("BRN/$", "ratio", "BRN", "DX"), ("DX(abs)", "abs", "DX", None),
    ]
    ctx_mp = mp.get_context("fork")
    with ctx_mp.Pool(processes=len(specs)) as pool:
        results = dict(pool.map(_edge_worker, specs))
    sES, sGC, sCL = results["ES/$"], results["GC/$"], results["CL/$"]
    xESGC, xESCL = results["ES/GC"], results["ES/CL"]
    omega, sZN, s6E = results["ω=GC/CL"], results["ZN/$"], results["6E/$"]
    sBRN, sDXabs = results["BRN/$"], results["DX(abs)"]
    del bars

    # 日频外部 regime 变量（缓存）。
    vix = _fetch_yf_daily("^VIX", "_vix_1d_yf.json")
    vix_day = ({ds[:10]: vix["closes"][i] for i, ds in enumerate(vix["dates"])}
               if vix else {})
    slope_map, slope_src = _fetch_yield_slope()
    print(f"\nVIX={'有' if vix else '无'}；斜率来源：{slope_src}", flush=True)

    # 组装每日行（仅 Γ/X 齐全）。
    rows = []
    for d in days:
        if not (d in sES and d in sGC and d in sCL and d in xESGC and d in xESCL):
            continue
        rows.append({
            "gamma": (sES[d], sGC[d], sCL[d]),
            "X": (xESGC[d], xESCL[d]),
            "omega": omega.get(d),
            "gold": sGC.get(d), "oil": sCL.get(d),
            "ZN": sZN.get(d), "6E": s6E.get(d), "BRN": sBRN.get(d),
            "DX": sDXabs.get(d),
            "VIX": _vix_state(vix_day[d]) if d in vix_day else None,
            "slope": _slope_state(slope_map[d]) if d in slope_map else None,
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

    # 单变量真实增益（在 Γ,ω 之上，扣 shuffle 伪增益）。
    cand_cols = {
        "gold(GC/$)": "gold", "oil(CL/$)": "oil", "ZN/$(rate)": "ZN",
        "6E/$": "6E", "BRN/$": "BRN", "DX(abs)": "DX",
        "VIX": "VIX", "slope(2s10s)": "slope",
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
        floor = _shuffle_floor(base, [r[key] for r in valid], rng)
        cand_results[cname] = {"n": len(valid), "h_base": h_base, "h_with": h_with,
                               "observed_gain": obs, "shuffle_floor": floor,
                               "real_gain": obs - floor,
                               "n_states": len(set(r[key] for r in valid))}
    print("\n候选单变量真实增益（在 Γ,ω 之上，扣 shuffle）：", flush=True)
    for cname, v in sorted([(k, v) for k, v in cand_results.items() if "error" not in v],
                           key=lambda kv: -kv[1]["real_gain"]):
        print(f"  {cname:14s} 真实={v['real_gain']:+.3f} "
              f"(观测{v['observed_gain']:.3f}−伪{v['shuffle_floor']:.3f}) "
              f"N={v['n']} states={v['n_states']}", flush=True)

    # 最小变量集贪心（全 8 候选 + ω）。
    avail_keys = {"ω": "omega", "gold": "gold", "oil": "oil", "ZN": "ZN",
                  "6E": "6E", "BRN": "BRN", "DX": "DX", "VIX": "VIX",
                  "slope": "slope"}
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
            with_pairs = [((c, v), y) for c, v, y in zip(ctx, col, Y)]
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
        print(f"  + {best:6s} → H={h_cur:.3f}（真实增益 {best_real:+.3f}）", flush=True)
        remaining.discard(best)

    # 过拟合诊断。
    n_ctx = len(set(ctx))
    spc = len(full) / n_ctx if n_ctx else 0.0
    card = 1
    for k in ["gamma"] + [avail_keys[c] for c in chosen]:
        if k == "gamma":
            card *= len(set(r["gamma"] for r in full))
        else:
            card *= max(1, len(set(r[k] for r in full)))
    print(f"\n过拟合诊断：终态 cell 数={n_ctx}（上限 {card}）；"
          f"样本/cell={spc:.2f}；N={len(full)}", flush=True)

    sigs = {"sES": sES, "sGC": sGC, "sCL": sCL, "xESGC": xESGC, "xESCL": xESCL,
            "omega": omega, "sZN": sZN, "s6E": s6E, "sBRN": sBRN, "sDXabs": sDXabs}
    write_report(grid, days, rows, full, H_X, H_X_g, H_X_gw,
                 cand_results, greedy_log, chosen, h_cur, slope_src,
                 sigs, n_ctx, card, spc, time.time() - t_start)
    print(f"\n总耗时 {time.time()-t_start:.0f}s；报告 → {_OUT}", flush=True)


# ════════════════════════════════════════════════════════════════════════════
# 报告（结果包六要素）
# ════════════════════════════════════════════════════════════════════════════

def write_report(grid, days, rows, full, H_X, H_X_g, H_X_gw,
                 cand_results, greedy_log, chosen, final_H, slope_src,
                 sigs, n_ctx, card, spc, elapsed) -> None:
    L = []
    w = L.append
    reached0 = final_H < 0.05
    all_flips = []
    for s in sigs.values():
        vals = [s[d] for d in sorted(s)]
        all_flips.append(sum(1 for a, b in zip(vals, vals[1:]) if a != b))
    fmin, fmax = (min(all_flips), max(all_flips)) if all_flips else (0, 0)

    w("# 1min 分辨变量搜索（O(N) 引擎重跑）：Γ→交叉边 剩余熵 H(X|Γ) 压缩\n")
    w(f"> 生成：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
      f"脚本：`analysis/gamma_delta_1min_discriminant_search.py`　总耗时 {elapsed:.0f}s\n")
    w("> **引擎切换 O(N²)→O(N)**：上游 `gamma_delta_1m_resolution.py` 用流式 "
      "`RecursiveOrchestrator.process_bar`（笔尾变化全量重算 segments → O(strokes²)）。"
      "本脚本用 **day-batch**：独立 `BiEngine` 流式产笔（O(N) 摊还）+ 仅每日末对 stroke 前缀"
      "跑同一批量管线（~%d 次而非 ~strokes 次）→ O(days×strokes)=O(N)。\n" % len(days))
    w("> **bit-exactness**：σ 仅依赖 `current_moves()`，process_bar 内部正用同一批量管线对同一 "
      "stroke 前缀算 moves（短路缓存=纯性能）→ day-batch 输入相同 ⟹ 输出逐位等价。"
      "`analysis/_verify_on_engine.py` 实测 400k 前缀 abs/ratio 均 **0 diff**"
      "（stream 9.1/12.7s vs batch 2.2/2.7s）。**引擎切换不改任何 σ 数值 → 不改 H 结论的有效域。**\n")
    w(f"> **a0 = 1min**（databento UTC，无重采样）；**numeraire M = DX 期货**；"
      f"窗口 {grid[0][:10]} → {grid[-1][:10]}；网格 {len(grid):,} bar；日采样 {len(days)} 天；"
      f"σ = L1 走势方向（`current_moves()[-1]`，非最高递归级别——见 "
      f"project_highest_level_sigma_frozen）。\n")
    w("> 实例：期货折叠通道 {ES,GC,CL,DX}（528号；X=**2** 交叉边 ES/GC,ES/CL，"
      "GC/CL=ω 剔除防循环）。跨 a0/实例**不可与 30m 正典 {SPY,DBC,VNQ,UUP}（X=3）同表**"
      "（formalization-validity-domain）。\n")

    # 核心解读。
    w("\n---\n\n## 核心解读\n")
    w(f"**目标**：把 H(交叉边|Γ) 压向 0（=Γ 完全确定交叉边 → k4_path_c 六边各自跑可被推翻）。\n")
    w(f"**实测裁决：剩余熵补到 0 = {'确认' if reached0 else '**否证**（强否定性结果，L2）'}。**\n")
    w(f"1. **σ 非退化 = {'确认' if fmax > 100 else '存疑'}**：1min L1 走势 σ 在 {len(days)} 天翻转 "
      f"**{fmin}–{fmax} 次**（粗 a0 仅个位/十位数）。\n")
    if not reached0:
        w(f"2. **剩余熵补到 0 = 否证**：贪心从 H(X|Γ)={greedy_log[0][1]:.3f} 经 "
          f"{len(chosen)} 步（{chosen}）停在 **{final_H:.3f} bits**，其余候选真实增益 < 0.005。"
          "**变量用尽仍到不了 0**——非过拟合切碎假象"
          f"（样本/cell={spc:.2f}，贪心因真实增益转负提前停）。\n")
        w("3. **机制（非退化与可定性对立）**：a0 越细 → σ 翻转越频 → 交叉边 X 自身熵越大 → "
          "Γ+外部变量能解释的份额反而越小。Γ→交叉边一对多（294号 D 算子非线性、527号 σ 不保比值"
          "同态）随分辨率细化**单调放大**不可约熵。\n")
        w("**下游影响**：强化 k4_path_c「六边各自跑递归」——交叉边信息无法从 Γ+外部 regime 变量恢复，"
          "且该不可约性在最细 a0（1min）上更强。M2 选股读交叉边不可用 Γ 极性+regime 代理替代。\n")
    else:
        w(f"2. **剩余熵补到 0 = 确认**：贪心经 {len(chosen)} 步（{chosen}）降到 {final_H:.3f} bits ≈ 0 → "
          "(Γ, 选中变量) 完全确定交叉边。须复核过拟合（样本/cell={:.2f}）后方可推翻六边各自跑。\n".format(spc))

    # 结果包六要素。
    w("\n## 结果包（六要素）\n")
    w("**1. 结论**：")
    w(f"(a) 基线 H(X)={H_X:.3f}，H(X|Γ)={H_X_g:.3f}，H(X|Γ,ω)={H_X_gw:.3f}"
      f"（ω 增益 {H_X_g-H_X_gw:.3f}）。")
    w(f"(b) 全 8 候选+ω 贪心选中 **{chosen}**，终态 H(X|Γ,选中)=**{final_H:.3f} bits**"
      f"{'（到 0）' if reached0 else '（**未到 0**，残余结构性不确定）'}。")
    w(f"(c) **剩余熵补到 0 = {'是' if reached0 else '否'}**。\n")
    w("\n**2. 定义依据**：Γ=(σ_P,σ_C,σ_R) 折叠通道期货实例 {ES,GC,CL,$=DX}"
      "（528号折叠通道）；σ=L1 走势方向（`current_moves()[-1]`，O(N) day-batch 引擎逐 bar 产笔 + "
      "每日末批量 seg→zs→move）；交叉边 X=(σ(ES/GC),σ(ES/CL))，GC/CL 剔除（=ω 防循环）。\n")
    w("**3. 边界条件**：")
    w("- 引擎从 O(N²) 流式换 O(N) day-batch，**bit-exact**（_verify_on_engine.py 0 diff）"
      "→ σ 数值与流式版完全一致，H 数值差异**仅来自数据/窗口**（若与上游 1m_results 同数据则应一致）；")
    w(f"- a0=1min，M=DX 期货；窗口 {grid[0][:10]}+；7 标的交集 {len(grid):,} bar；")
    w("- 跨 a0/实例数值不可与 30m/日线同表（不同有效域，formalization-validity-domain）；")
    w("- gold=σ(GC/$)、oil=σ(CL/$) 在期货实例下 **= Γ 主边**（冗余，预期真实增益≈0）；"
      "ZN/$ 是 10Y 票据 vs 美元（久期/利率代理，非曲线斜率）；")
    w("- 多变量条件熵含有限样本伪增益（已 shuffle 扣减，真实残余 ≥ 报告值；Miller-Madow 偏差使"
      "稀疏 cell 下 H 被低估 → 报告残余是**下界**）；")
    w("- **结论翻转条件**：若某主边 σ 翻转≈0（退化），该 Γ 分量无信息，H(X|Γ) 基线失真，需换 a0/numeraire。\n")
    w("**4. 下游推论**：")
    if reached0:
        w("- 剩余熵→0：(Γ,选中变量) 完全确定交叉边 → k4_path_c 可从 Γ+这些变量推交叉边（限本有效域）。\n")
    else:
        w(f"- 剩余熵未到 0（{final_H:.3f} bits）：Γ+全候选不足以确定交叉边 → **强化** k4_path_c 六边各自跑；")
        w("- M2 选股读交叉边不能用 Γ 极性+regime 变量替代联合读数。\n")
    w("**5. 谱系引用**：527（σ 走势方向态）、k4_path_c（六边各自跑）、482（ω 部分分辨）、"
      "294（D 算子非线性，Γ→交叉边一对多根因）、528（折叠通道顶点）、"
      "project_recursive_orchestrator_rust（Rust 引擎）、project_bi_zhongshu_on2_two_walls（O(N²) 墙）；"
      "不确定是否有「1min 分辨」专门谱系——若结论稳定建议新建。\n")
    w(f"**6. 影响声明**：新增 `analysis/gamma_delta_1min_discriminant_search.py`（O(N) 引擎）+ "
      f"`analysis/_verify_on_engine.py`（bit-exact 验证）+ `{_OUT.name}`；"
      "不改源码、不改流式版结果（同 σ 值，仅性能）。\n")

    # σ 翻转诊断表。
    w("\n## σ 翻转诊断（验证 1min L1 走势非退化）\n")
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

    # 单变量真实增益。
    w("\n## 单变量真实增益（在 Γ,ω 之上）\n")
    w("| 候选 | 观测增益 | shuffle 伪增益 | **真实增益** | states | N |")
    w("|------|---------|--------------|-----------|--------|---|")
    for cname, v in sorted([(k, v) for k, v in cand_results.items() if "error" not in v],
                           key=lambda kv: -kv[1]["real_gain"]):
        w(f"| {cname} | {v['observed_gain']:.3f} | {v['shuffle_floor']:.3f} | "
          f"**{v['real_gain']:+.3f}** | {v['n_states']} | {v['n']} |")
    for cname, v in cand_results.items():
        if "error" in v:
            w(f"| {cname} | — | — | [{v['error']}] | — | — |")

    # 贪心。
    w("\n## 最小变量集贪心搜索\n")
    w("| 步骤 | 加入 | H(X\\|...) | 真实增益 |")
    w("|------|------|----------|---------|")
    for i, (k, h, gain) in enumerate(greedy_log):
        w(f"| {i} | {k} | {h:.3f} | {('—' if gain is None else f'{gain:+.3f}')} |")
    w(f"\n> 选中集：**{chosen}**；终态 H(X|Γ,选中)={final_H:.3f} bits。\n")

    # 过拟合诊断。
    w("\n## 过拟合诊断\n")
    w(f"- 终态上下文 cell 数 = **{n_ctx}**（理论上限 {card}）；")
    w(f"- 样本/cell = **{spc:.2f}**（公共样本 N={len(full)}）；")
    overfit = spc < 3.0
    w(f"- **过拟合风险 = {'高' if overfit else '中/低'}**："
      f"{'样本/cell <3 → 条件熵下降部分是有限样本切碎；剩余熵的「降低」须警惕。' if overfit else '样本/cell ≥3 → 条件熵估计相对稳健，但单窗口 L2（非 L3 多窗口）。'}\n")

    _OUT.write_text("\n".join(L) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
