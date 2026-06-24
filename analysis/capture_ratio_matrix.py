"""每级别·每元素 涨跌幅绝对值 捕获率 严格验证工具（#149 capture-ratio）。

═══════════════ 存在论位置（编排者框定 FINAL GOAL 验证方法论工具）═══════════════

FINAL GOAL = 递归嵌套多重赋格赚到**每个级别每个元素的涨跌幅绝对值** = Σ所有级别|涨跌幅|。
聚合指标（short_pnl / strat_pct / vs BH）结构性掩盖 per-element——本工具把 Face A(#113) 的
realized pnl 归属到每个 (级别 k, 走势段 e, 方向 d)，与理论上限 Σ|Δ| 对照，定位 r≤0 漏洞坐标。

数据源 = Rust 引擎 read-only instrumentation（`RecTStream.finish_full()`，#149）：
  - level_segments: 全历史 tree 各级别走势段 (level, raw_start, raw_end, high, low, is_up) —— **分母源**。
  - leg_trades:    Face A LegPair 逐笔账本 (level, entry_bar, exit_bar, entry_px, exit_px, units, is_short, pnl) —— **分子源**。
  - pair_long_pnl / pair_short_pnl: per-level realized（账本完整性自检对账基准）。

═══════════════ 定义（L0 definitional，结果包六要素见报告）═══════════════

记 level-k 走势段 e，raw 区间 [s,t]，振幅 |Δ_e| = high_e − low_e ≥ 0，方向 d_e（Up/Down）。

1. **理论上限（真基准，≫BH）**：Σ|Δ| = Σ_k Σ_e |Δ_e|。
   BH 只吃最大级别一个方向（close[-1]−close[0]）；Σ|Δ| 吃所有级别所有方向 ⇒ 完美多重赋格上限。

2. **匹配腿**：与 e 同级别 k、极性匹配 d 的 LegPair 腿（Up→多头 / Down→做空）。

3. **捕获额（分子，美元）**：cap$(e) = Σ_{匹配腿 trade T 与 [s,t] 重叠[a,b]} u_T·sign_T·(close[b]−close[a])，
   sign=+1(多)/−1(空)，a=max(s,entry_bar_T)，b=min(t,exit_bar_T)。Face A 成交=确认时点 close ⇒
   close[entry_bar]==entry_px（口径一致，无 look-ahead 注入）。

4. **理想额（分母，美元）**：ideal$(e) = |Δ_e|·u_peak(e)，u_peak=匹配腿在 [s,t] 内峰值持仓股数。
   单笔满跨段时 ideal$=|Δ|·u，r=cap$/ideal$=sign·(close[b]−close[a])/|Δ| ∈ (−∞,1]，完美持有 r→1
   （因收盘成交 + 确认滞后，r<1 是真实税，非工具误差）。

5. **覆盖（coverage，价格点）**：有匹配腿在场的走势段占比 = Σ|Δ|_covered / Σ|Δ|_all。
   揭示「没吃到」= 该级别该方向根本没腿（多重赋格未在该级别部署）。

6. **判据（可证伪+可定位）**：
   - 完全做到 ⟺ ∀(k,e) cap$>0 且 总捕获率(efficiency)=Σcap$/Σideal$ → 1 且 coverage → 1。
   - 没做到 ⟺ 列出 {(k,e,d): cap$≤0}（LOST=有腿但反亏/晚建）+ MISSED 统计（无腿）= 实现错坐标。

认识论等级：定义=L0；单标的=L2；8标的(#117)交叉=L3（formalization-validity-domain）。
no-hardcode / no-over-claim（231）；否定性结果优先（定位漏洞 > 确认）。

用法：
    .venv/bin/python analysis/capture_ratio_matrix.py                 # 8 标的 structural（production Face A）
    .venv/bin/python analysis/capture_ratio_matrix.py --symbols OKLO  # 指定标的
    .venv/bin/python analysis/capture_ratio_matrix.py --mode and      # 指定 mode
"""
from __future__ import annotations

import argparse
import json
import math
import sys
import time
from pathlib import Path

import newchan_rust as nr

# SYMBOLS：逐字复刻 rust/src/recursive_t/backtest_run.rs::SYMBOLS（同源 a₀/同源 Face A 引擎）。
SYMBOLS = [
    ("CL", "cl_1m_databento_10y.json"),
    ("BRN", "brn_1m_databento_10y.json"),
    ("DX", "dx_1m_databento_10y.json"),
    ("GC", "gc_1m_databento_10y.json"),
    ("ES", "es_1m_databento_10y.json"),
    ("QQQ", "qqq_1m_databento_full.json"),
    ("BTC", "btc_1m_full.json"),
    ("OKLO", "oklo_1m_databento.json"),
]

REPO = Path(__file__).resolve().parents[1]
DATA_DIR = REPO / "analysis" / "data_cache"
# worktree 大 JSON 被 gitignore 不在 worktree 内 ⇒ 回落主仓库 data_cache（绝对路径，记忆 rec_stream §跑法）。
MAIN_DATA_DIR = Path("/Users/silencehan/Projects/NewChanlun/analysis/data_cache")


def load_clean_ohlc(path: Path) -> tuple[list[float], list[float], list[float], list[float]]:
    """逐位复刻 backtest_run.rs::load_clean_ohlc（两遍清洗，保证与 Face A 引擎同 bar 视图）。"""
    text = path.read_text()
    text = text.replace("-Infinity", "null").replace("Infinity", "null").replace("NaN", "null")
    raw = json.loads(text)
    if raw.get("bars"):
        bars = raw["bars"]
        o_in = [b.get("open") for b in bars]
        h_in = [b.get("high") for b in bars]
        l_in = [b.get("low") for b in bars]
        c_in = [b.get("close") for b in bars]
    else:
        o_in = raw.get("opens", [])
        h_in = raw.get("highs", [])
        l_in = raw.get("lows", [])
        c_in = raw.get("closes", [])
    # 第一遍：None/nan/≤0 清洗。
    o, h, l, c = [], [], [], []
    for i in range(len(c_in)):
        oi, hi, li, ci = o_in[i], h_in[i], l_in[i], c_in[i]
        if oi is None or hi is None or li is None or ci is None:
            continue
        if any(x != x for x in (oi, hi, li, ci)):  # nan
            continue
        if oi <= 0 or hi <= 0 or li <= 0 or ci <= 0:
            continue
        o.append(oi); h.append(hi); l.append(li); c.append(ci)
    # 第二遍：spike-and-revert 孤立坏 tick。
    n = len(c)
    drop = [False] * n
    for i in range(1, max(0, n - 1)):
        if abs(c[i] / c[i - 1] - 1.0) > 0.5 and abs(c[i + 1] / c[i - 1] - 1.0) < 0.05:
            drop[i] = True
    if any(drop):
        keep = lambda v: [x for i, x in enumerate(v) if not drop[i]]
        return keep(o), keep(h), keep(l), keep(c)
    return o, h, l, c


def run_face_a(closes_o, closes_h, closes_l, closes_c, mode: str) -> dict:
    """逐 bar 推 Face A（RecTStream=new_production 默认 Face A），finish_full 取 per-element 数据。"""
    s = nr.RecTStream(mode)
    for o, h, l, c in zip(closes_o, closes_h, closes_l, closes_c):
        s.push_bar(o, h, l, c)
    return s.finish_full()


def compute_capture(res: dict, closes: list[float]) -> dict:
    """per-element 捕获率矩阵 + r≤0 坐标定位 + 总捕获率(efficiency) + coverage + 多头/做空差异。"""
    segs = res["level_segments"]          # (level, raw_start, raw_end, high, low, is_up)
    trades = res["leg_trades"]            # (level, entry_bar, exit_bar, entry_px, exit_px, units, is_short, pnl)
    n = len(closes)

    # ── 账本完整性自检（fail-loud，no-workaround）：Σ账本多头 pnl == Σ pair_long_pnl，空头同 ──
    ledger_long = sum(t[7] for t in trades if not t[6])
    ledger_short = sum(t[7] for t in trades if t[6])
    pl = sum(res["pair_long_pnl"]); ps = sum(res["pair_short_pnl"])
    tol = 1e-6 * (abs(pl) + abs(ps) + 1.0)
    ledger_ok = abs(ledger_long - pl) <= tol and abs(ledger_short - ps) <= tol

    # ── 匹配腿索引：按 (level, is_short) 分桶 trade，便于逐段重叠归因 ──
    by_lvl_dir: dict = {}
    for (lvl, eb, xb, ep, xp, u, is_short, pnl) in trades:
        by_lvl_dir.setdefault((lvl, is_short), []).append((eb, xb, u))

    def clampc(i: int) -> float:
        if i < 0:
            i = 0
        if i >= n:
            i = n - 1
        return closes[i]

    # ── 逐走势段归因 ──
    # 矩阵聚合：key=(level, d) → {n_seg, n_cov, n_pos, n_le0, sum_abs_dz, sum_abs_dz_cov, cap$, ideal$}
    agg: dict = {}
    lost: list = []      # (level, raw_start, raw_end, dir, |Δ|, cap$)  —— 有腿但 cap$≤0（实现错精确坐标）
    missed_top: dict = {}  # (level,dir) → list of (|Δ|, s, t) 取最大若干（exemplar，非全量）
    sum_abs_dz_all = 0.0

    for (lvl, s, t, high, low, is_up) in segs:
        dz = high - low
        if dz < 0:
            dz = 0.0
        sum_abs_dz_all += dz
        d = "Up" if is_up else "Down"
        key = (lvl, d)
        a = agg.setdefault(key, {"n_seg": 0, "n_cov": 0, "n_pos": 0, "n_le0": 0,
                                 "abs_dz": 0.0, "abs_dz_cov": 0.0, "cap": 0.0, "ideal": 0.0})
        a["n_seg"] += 1
        a["abs_dz"] += dz
        # 匹配腿：Up 段→多头(is_short=False)；Down 段→做空(is_short=True)。同级别 k。
        want_short = not is_up
        legs = by_lvl_dir.get((lvl, want_short), [])
        cap = 0.0
        u_peak = 0.0
        covered = False
        sign = 1.0 if is_up else -1.0
        if t < s:
            s, t = t, s  # 防御：raw_start>raw_end（理论不发生）
        for (eb, xb, u) in legs:
            if eb < 0:
                eb = 0
            lo = s if s > eb else eb
            hi = t if t < xb else xb
            if lo > hi:
                continue  # 无重叠
            covered = True
            cap += u * sign * (clampc(hi) - clampc(lo))
            if u > u_peak:
                u_peak = u
        if covered:
            ideal = dz * u_peak
            a["n_cov"] += 1
            a["abs_dz_cov"] += dz
            a["cap"] += cap
            a["ideal"] += ideal
            if cap > 0:
                a["n_pos"] += 1
            else:
                a["n_le0"] += 1
                lost.append((lvl, s, t, d, dz, cap))
        else:
            # MISSED：无匹配腿在场（coverage gap）。记 top-N exemplar（按 |Δ| 最大）。
            mk = (lvl, d)
            lst = missed_top.setdefault(mk, [])
            lst.append((dz, s, t))

    # MISSED top-N（exemplar，no silent cap：报告记录 MISSED 总数 + 仅展示最大 N 个坐标）。
    TOPN = 8
    missed_examples = {}
    missed_count = {}
    missed_abs_dz = {}
    for mk, lst in missed_top.items():
        lst.sort(reverse=True)
        missed_examples[mk] = lst[:TOPN]
        missed_count[mk] = len(lst)
        missed_abs_dz[mk] = sum(x[0] for x in lst)

    # 总量
    sum_cap = sum(a["cap"] for a in agg.values())
    sum_ideal = sum(a["ideal"] for a in agg.values())
    sum_abs_dz_cov = sum(a["abs_dz_cov"] for a in agg.values())
    efficiency = (sum_cap / sum_ideal) if sum_ideal > 0 else float("nan")
    coverage = (sum_abs_dz_cov / sum_abs_dz_all) if sum_abs_dz_all > 0 else float("nan")

    # 多头赋格 vs 做空赋格（Up 段=多头侧 / Down 段=做空侧）实现差异
    def side(dir_):
        cap = sum(a["cap"] for (k, d), a in agg.items() if d == dir_)
        ideal = sum(a["ideal"] for (k, d), a in agg.items() if d == dir_)
        dz_all = sum(a["abs_dz"] for (k, d), a in agg.items() if d == dir_)
        dz_cov = sum(a["abs_dz_cov"] for (k, d), a in agg.items() if d == dir_)
        n_seg = sum(a["n_seg"] for (k, d), a in agg.items() if d == dir_)
        n_cov = sum(a["n_cov"] for (k, d), a in agg.items() if d == dir_)
        n_le0 = sum(a["n_le0"] for (k, d), a in agg.items() if d == dir_)
        return {
            "cap": cap, "ideal": ideal, "abs_dz_all": dz_all, "abs_dz_cov": dz_cov,
            "n_seg": n_seg, "n_cov": n_cov, "n_le0": n_le0,
            "efficiency": (cap / ideal) if ideal > 0 else float("nan"),
            "coverage": (dz_cov / dz_all) if dz_all > 0 else float("nan"),
        }

    bh = closes[-1] - closes[0]
    return {
        "n_bars": n,
        "ledger_self_check": {"ok": ledger_ok, "ledger_long": ledger_long, "pair_long": pl,
                              "ledger_short": ledger_short, "pair_short": ps},
        "sum_abs_dz_all": sum_abs_dz_all,          # 理论上限 Σ|Δ|（价格点）
        "bh_abs_move": abs(bh),                    # |close[-1]−close[0]|（BH 单级单向）
        "dz_vs_bh_ratio": (sum_abs_dz_all / abs(bh)) if bh != 0 else float("nan"),
        "sum_cap_dollars": sum_cap,
        "sum_ideal_dollars": sum_ideal,
        "total_efficiency": efficiency,            # Σcap$/Σideal$ → 1 = 部署段全吃满
        "total_coverage": coverage,                # Σ|Δ|_cov/Σ|Δ|_all → 1 = 每段都有匹配腿
        "final_nav": res["final_nav"],
        "n_trades": len(trades),
        "n_segments": len(segs),
        "long_side": side("Up"),
        "short_side": side("Down"),
        "matrix": {f"L{k}/{d}": v for (k, d), v in sorted(agg.items())},
        "lost_coords": sorted(lost, key=lambda x: x[4], reverse=True),  # 全量 r≤0（有腿反亏），按 |Δ| 降序
        "missed_count": {f"L{k}/{d}": missed_count[(k, d)] for (k, d) in missed_count},
        "missed_abs_dz": {f"L{k}/{d}": missed_abs_dz[(k, d)] for (k, d) in missed_abs_dz},
        "missed_examples": {f"L{k}/{d}": missed_examples[(k, d)] for (k, d) in missed_examples},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbols", default=",".join(s for s, _ in SYMBOLS))
    ap.add_argument("--mode", default="structural")
    ap.add_argument("--bars", type=int, default=0, help="0=全量")
    ap.add_argument("--out", default=str(REPO / "analysis" / "data_cache"))
    args = ap.parse_args()

    want = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    file_of = dict(SYMBOLS)
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    results = {}
    for sym in want:
        fn = file_of.get(sym)
        if fn is None:
            print(f"[{sym}] 未注册，跳过", file=sys.stderr); continue
        path = DATA_DIR / fn
        if not path.exists():
            path = MAIN_DATA_DIR / fn
        if not path.exists():
            print(f"[{sym}] 数据缺失 {path}", file=sys.stderr); continue
        t0 = time.time()
        o, h, l, c = load_clean_ohlc(path)
        if args.bars:
            o, h, l, c = o[:args.bars], h[:args.bars], l[:args.bars], c[:args.bars]
        print(f"[{sym}] {len(c):,} bars loaded {time.time()-t0:.1f}s, running Face A({args.mode})...", flush=True)
        t1 = time.time()
        res = run_face_a(o, h, l, c, args.mode)
        cap = compute_capture(res, c)
        cap["symbol"] = sym; cap["mode"] = args.mode
        dt = time.time() - t1
        results[sym] = cap
        sc = cap["ledger_self_check"]
        print(f"[{sym}] done {dt:.1f}s | ledger_ok={sc['ok']} | Σ|Δ|={cap['sum_abs_dz_all']:.0f} "
              f"(={cap['dz_vs_bh_ratio']:.1f}×BH) | eff={cap['total_efficiency']:.3f} "
              f"cov={cap['total_coverage']:.3f} | nav={cap['final_nav']:.0f} "
              f"L_eff={cap['long_side']['efficiency']:.3f}/cov={cap['long_side']['coverage']:.3f} "
              f"S_eff={cap['short_side']['efficiency']:.3f}/cov={cap['short_side']['coverage']:.3f} "
              f"| lost(r≤0)={len(cap['lost_coords'])}", flush=True)
        (out_dir / f"capture_ratio_{sym}_{args.mode}.json").write_text(
            json.dumps(cap, ensure_ascii=False, indent=1))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
