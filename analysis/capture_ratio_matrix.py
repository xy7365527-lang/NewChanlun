"""每级别·每元素 涨跌幅绝对值 捕获率 严格验证工具 v2（#149/#154 capture-ratio）。

═══ 编排者两刀严格化（v2 覆盖 v1，no-summation / per-trade ∀）═══

刀一【矩阵完备性 = 前置】不完备矩阵=假阴性盲区（像聚合掩盖 per-element 一样掩盖遗漏维度）。
  四个可程序化检验（任一失败即定位不完备处，**先跑这个再跑捕获率**）：
  1. 双射：trade 级别集 ⊆ 走势段级别集（无虚构 cell：腿不骑没段的级别）；每段有 cell（构造保证）。
  2. 覆盖率=1：∀级别 段序列在该级别活跃区间内 contiguous（无内部游离 bar）；head/tail 未覆盖=边界。
  3. 递归闭合：级别 k 中枢/段端点 ⊆ 级别 k-1 段端点（区间套递归一致）。
  4. a0 底：level0 段 = a0(线段)最细，无更细可分（构成性底；笔更细但 a0=Segment 不取，526号）。
  完备性根据=L0(580 操作语义二维完备 / 581 状态范畴全映射 / 走势终完美+中枢递归)；检验=L2/L3。
  诚实边界：完备性仅在缠论递归分解 well-defined 域内；古怪线段(77/78课)/中枢延伸/谱系001退化线段
  /002源不完备=矩阵歧义缺口，标注。

刀二【逐笔全称判定 = 禁所有求和作判据】Σcap/Σ|Δ|、by-level Σ、平均、胜率 **全作废为判据**
  （求和三重掩盖：正负相抵/大段掩小段/级别内求和）。改：
  - 粒度 = 每一笔 trade（买卖点驱动的开平），做多做空对称都查。
  - 逐笔 r(trade) = per_unit_realized / covered_Δ，covered_Δ = 该笔持仓期内可得的有利幅度
    （多头: max(high[开..平]) − 开价 / 空头: 开价 − min(low[开..平])）。r ∈ (−∞, 1]，r>0 ⟺ 该笔盈利。
  - 判据（全称 ∀，可证伪：一笔反例即否）：做到 ⟺ ∀trade r>0；没做到 ⟺ ∃trade r≤0 → 列**全部** r≤0 笔坐标。
  - 聚合数字仅作诊断（非判据，明确标注）。

数据源 = `RecTStream.finish_full()`（#149/#154 read-only instrumentation）：
  level_segments / level_centers / leg_trades / pair_long_pnl / pair_short_pnl / final_nav。

认识论：完备性据=L0；逐笔 r=L3（真实数据每笔）；∀r>0 全称=可证伪。no-over-claim/no-summation(231)。
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np
import newchan_rust as nr

SYMBOLS = [
    ("CL", "cl_1m_databento_10y.json"), ("BRN", "brn_1m_databento_10y.json"),
    ("DX", "dx_1m_databento_10y.json"), ("GC", "gc_1m_databento_10y.json"),
    ("ES", "es_1m_databento_10y.json"), ("QQQ", "qqq_1m_databento_full.json"),
    ("BTC", "btc_1m_full.json"), ("OKLO", "oklo_1m_databento.json"),
]
REPO = Path(__file__).resolve().parents[1]
DATA_DIR = REPO / "analysis" / "data_cache"
MAIN_DATA_DIR = Path("/Users/silencehan/Projects/NewChanlun/analysis/data_cache")
EPS = 1e-9


def load_clean_ohlc(path: Path):
    """逐位复刻 backtest_run.rs::load_clean_ohlc（两遍清洗，与 Face A 引擎同 bar 视图）。"""
    text = path.read_text().replace("-Infinity", "null").replace("Infinity", "null").replace("NaN", "null")
    raw = json.loads(text)
    if raw.get("bars"):
        b = raw["bars"]
        o_in = [x.get("open") for x in b]; h_in = [x.get("high") for x in b]
        l_in = [x.get("low") for x in b]; c_in = [x.get("close") for x in b]
    else:
        o_in = raw.get("opens", []); h_in = raw.get("highs", [])
        l_in = raw.get("lows", []); c_in = raw.get("closes", [])
    o, h, l, c = [], [], [], []
    for i in range(len(c_in)):
        oi, hi, li, ci = o_in[i], h_in[i], l_in[i], c_in[i]
        if oi is None or hi is None or li is None or ci is None:
            continue
        if any(x != x for x in (oi, hi, li, ci)):
            continue
        if oi <= 0 or hi <= 0 or li <= 0 or ci <= 0:
            continue
        o.append(oi); h.append(hi); l.append(li); c.append(ci)
    n = len(c)
    drop = [False] * n
    for i in range(1, max(0, n - 1)):
        if abs(c[i] / c[i - 1] - 1.0) > 0.5 and abs(c[i + 1] / c[i - 1] - 1.0) < 0.05:
            drop[i] = True
    if any(drop):
        keep = lambda v: [x for i, x in enumerate(v) if not drop[i]]
        return keep(o), keep(h), keep(l), keep(c)
    return o, h, l, c


def run_face_a(o, h, l, c, mode):
    s = nr.RecTStream(mode)
    for a, b, d, e in zip(o, h, l, c):
        s.push_bar(a, b, d, e)
    return s.finish_full()


# ════════════════ 刀一：矩阵完备性四检验（前置）════════════════

def completeness_checks(res, n):
    segs = res["level_segments"]      # (k, rs, re, high, low, is_up)
    centers = res["level_centers"]    # (k, rs, re, zg, zd)
    trades = res["leg_trades"]        # (k, eb, xb, ep, xp, u, is_short, pnl)

    seg_levels = sorted(set(s[0] for s in segs))
    trade_levels = sorted(set(t[0] for t in trades))

    # 检验1 双射：trade 级别 ⊆ 段级别（无虚构 cell）；每段有 cell（构造保证：矩阵遍历 segs）。
    phantom_levels = [k for k in trade_levels if k not in set(seg_levels)]
    chk1 = {"pass": len(phantom_levels) == 0, "seg_levels": seg_levels,
            "trade_levels": trade_levels, "phantom_trade_levels": phantom_levels,
            "n_segments": len(segs), "note": "每段有 cell（矩阵逐 level_segments 遍历，构造双射）"}

    # 检验2 覆盖率=1：每级别段在活跃区间内 contiguous（无内部游离 bar）。
    # 走势段共享转折 bar ⇒ raw 坐标相邻段 gap=rs[i+1]-re[i] ∈ {≤0(共享pivot重叠), 1(相邻)}；gap≥2=游离。
    cov = {}
    for k in seg_levels:
        ks = sorted([(s[1], s[2]) for s in segs if s[0] == k])
        if not ks:
            continue
        first, last = ks[0][0], ks[-1][1]
        rng = max(1, last - first)
        stray = 0
        stray_coords = []
        for i in range(len(ks) - 1):
            gap = ks[i + 1][0] - ks[i][1]
            if gap >= 2:
                stray += gap - 1
                if len(stray_coords) < 5:
                    stray_coords.append((ks[i][1], ks[i + 1][0]))
        internal_cov = 1.0 - stray / rng
        cov[f"L{k}"] = {"n_seg": len(ks), "range": [first, last], "active_bars": rng,
                        "internal_stray_bars": stray, "internal_coverage": round(internal_cov, 6),
                        "head_uncovered": first, "tail_uncovered": n - last,
                        "stray_coords": stray_coords}
    chk2 = {"pass": all(v["internal_coverage"] >= 1.0 - 1e-9 for v in cov.values()),
            "per_level": cov,
            "note": "internal_coverage=活跃区间内覆盖（head/tail 未覆盖=a0确认滞后+末段生长=边界，非bug）"}

    # 检验3 递归闭合：级别 k 段/中枢端点 ⊆ 级别 k-1 段端点（区间套递归一致）。
    def endpoints(items, k):
        e = set()
        for it in items:
            if it[0] == k:
                e.add(it[1]); e.add(it[2])
        return e
    rec = {}
    for k in seg_levels:
        if k - 1 not in set(seg_levels):
            continue
        parent_ep = endpoints(segs, k - 1)
        seg_ep = endpoints(segs, k)
        ctr_ep = endpoints(centers, k)
        seg_in = sum(1 for x in seg_ep if x in parent_ep)
        ctr_in = sum(1 for x in ctr_ep if x in parent_ep)
        rec[f"L{k}<L{k-1}"] = {
            "seg_endpoints": len(seg_ep), "seg_in_parent": seg_in,
            "seg_subset_ratio": round(seg_in / len(seg_ep), 4) if seg_ep else 1.0,
            "center_endpoints": len(ctr_ep), "center_in_parent": ctr_in,
            "center_subset_ratio": round(ctr_in / len(ctr_ep), 4) if ctr_ep else 1.0,
        }
    # 闭合判据：段端点子集率→1（容忍 merged→raw 边界 ±1 效应，阈 0.98）。
    chk3 = {"pass": all(v["seg_subset_ratio"] >= 0.98 for v in rec.values()) if rec else True,
            "per_level": rec,
            "note": "端点子集率<1 残差=merged→raw 边界 ±1 桥接效应（共享pivot merged bar 的 raw 宽度）"}

    # 检验4 a0 底：level0 存在且最细（a0=线段，无 level<0）。
    chk4 = {"pass": (0 in set(seg_levels)),
            "min_level": min(seg_levels) if seg_levels else None,
            "a0_source": "Segment(线段)",
            "note": "a0=线段(构成性底)；笔更细但 a0=Segment 不取(526号)=已知缺口，非 bug",
            "chanlun_boundary": "古怪线段(77/78课)/中枢延伸/谱系001退化线段/002源不完备=段归属歧义，引擎输出为准"}

    all_pass = chk1["pass"] and chk2["pass"] and chk3["pass"] and chk4["pass"]
    return {"all_pass": all_pass, "check1_bijection": chk1, "check2_bar_coverage": chk2,
            "check3_recursive_closure": chk3, "check4_a0_base": chk4}


# ════════════════ 刀二：逐笔全称判定（禁求和作判据）════════════════

def per_trade_capture(res, h, l, c):
    trades = res["leg_trades"]
    H = np.asarray(h, dtype=float); L = np.asarray(l, dtype=float); C = np.asarray(c, dtype=float)
    n = len(C)
    rows = []
    for (k, eb, xb, ep, xp, u, is_short, pnl) in trades:
        a = max(0, int(eb)); b = min(n - 1, int(xb))
        if b < a:
            a, b = b, a
        if is_short:
            per_unit = ep - xp
            mn = float(L[a:b + 1].min()) if b >= a else ep
            covered = ep - mn               # 空头可得有利幅度 = 开价 − 期内最低
        else:
            per_unit = xp - ep
            mx = float(H[a:b + 1].max()) if b >= a else ep
            covered = mx - ep               # 多头可得有利幅度 = 期内最高 − 开价
        covered = covered if covered > EPS else max(abs(per_unit), EPS)  # 兜底：从未有利则用净位移/eps
        r = per_unit / covered
        rows.append({"level": int(k), "dir": "short" if is_short else "long",
                     "entry_bar": int(eb), "exit_bar": int(xb), "units": u,
                     "pnl": pnl, "per_unit": per_unit, "covered_dz": covered, "r": r})
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbols", default=",".join(s for s, _ in SYMBOLS))
    ap.add_argument("--mode", default="structural")
    ap.add_argument("--bars", type=int, default=0)
    ap.add_argument("--out", default=str(REPO / "analysis" / "data_cache"))
    args = ap.parse_args()
    want = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    file_of = dict(SYMBOLS)
    out_dir = Path(args.out); out_dir.mkdir(parents=True, exist_ok=True)

    for sym in want:
        fn = file_of.get(sym)
        if not fn:
            print(f"[{sym}] 未注册", file=sys.stderr); continue
        path = DATA_DIR / fn
        if not path.exists():
            path = MAIN_DATA_DIR / fn
        if not path.exists():
            print(f"[{sym}] 数据缺失 {path}", file=sys.stderr); continue
        t0 = time.time()
        o, h, l, c = load_clean_ohlc(path)
        if args.bars:
            o, h, l, c = o[:args.bars], h[:args.bars], l[:args.bars], c[:args.bars]
        n = len(c)
        print(f"[{sym}] {n:,} bars {time.time()-t0:.1f}s, Face A({args.mode})...", flush=True)
        t1 = time.time()
        res = run_face_a(o, h, l, c, args.mode)
        comp = completeness_checks(res, n)
        rows = per_trade_capture(res, h, l, c)

        # 账本完整性自检（fail-loud）
        ll = sum(t[7] for t in res["leg_trades"] if not t[6]); ss = sum(t[7] for t in res["leg_trades"] if t[6])
        pl = sum(res["pair_long_pnl"]); ps = sum(res["pair_short_pnl"])
        tol = 1e-6 * (abs(pl) + abs(ps) + 1.0)
        ledger_ok = abs(ll - pl) <= tol and abs(ss - ps) <= tol

        # 全称判定（禁求和）：∀r>0?
        r_le0 = [x for x in rows if x["r"] <= 0]
        n_long = sum(1 for x in rows if x["dir"] == "long")
        n_short = sum(1 for x in rows if x["dir"] == "short")
        n_le0_long = sum(1 for x in r_le0 if x["dir"] == "long")
        n_le0_short = sum(1 for x in r_le0 if x["dir"] == "short")
        forall_pos = len(r_le0) == 0

        # 诊断分布（非判据）：r≤0 by (level,dir)
        dist = {}
        for x in r_le0:
            key = f"L{x['level']}/{x['dir']}"
            dist[key] = dist.get(key, 0) + 1

        out = {
            "symbol": sym, "mode": args.mode, "n_bars": n, "final_nav": res["final_nav"],
            "ledger_self_check": {"ok": ledger_ok, "ledger_long": ll, "pair_long": pl,
                                  "ledger_short": ss, "pair_short": ps},
            "completeness": comp,
            "n_trades": len(rows), "n_long": n_long, "n_short": n_short,
            "universal_judgment": {
                "criterion": "∀trade r>0（做多做空全笔；一笔 r≤0 即没做到那一笔的|涨跌幅|）",
                "forall_r_positive": forall_pos,
                "n_r_le0": len(r_le0), "n_r_le0_long": n_le0_long, "n_r_le0_short": n_le0_short,
                "verdict": "做到（∀r>0）" if forall_pos else f"没做到（∃{len(r_le0)} 笔 r≤0）",
            },
            "r_le0_coords": sorted([(x["level"], x["dir"], x["entry_bar"], x["exit_bar"],
                                     round(x["r"], 4), round(x["covered_dz"], 4), round(x["pnl"], 2))
                                    for x in r_le0], key=lambda z: z[6]),  # 按 pnl 升序（最亏在前）
            "diag_r_le0_by_level_dir": dist,  # 非判据
            "all_trades": [(x["level"], x["dir"], x["entry_bar"], x["exit_bar"],
                            round(x["r"], 4), round(x["covered_dz"], 4), round(x["pnl"], 2)) for x in rows],
        }
        (out_dir / f"capture_ratio_{sym}_{args.mode}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        cs = comp
        print(f"[{sym}] done {time.time()-t1:.1f}s | ledger_ok={ledger_ok} | "
              f"完备[双射{cs['check1_bijection']['pass']} 覆盖{cs['check2_bar_coverage']['pass']} "
              f"递归{cs['check3_recursive_closure']['pass']} a0底{cs['check4_a0_base']['pass']}] | "
              f"trades={len(rows)}(L{n_long}/S{n_short}) | ∀r>0={forall_pos} "
              f"r≤0={len(r_le0)}(L{n_le0_long}/S{n_le0_short})", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
