"""hold26_anc（祖先趋势豁免，26:80 下沉）八标的预注册回测——P1-P7 裁决。

上游：`analysis/slow_bull_vs_bh_research.md` §4（anc 定义）/ §6（预注册）。
实现零接触：纯消费 rust/src/trading/positional_fusion.rs 新增
`TrendScope::{Ancestor, SelfOrAncestor}`（mode="hold26_anc" / "fusion_ta"）。

anc 判据（标的无关，纯磁带内局部对象，零新参数）：
    in_trend(k) := ∃ j > k: trend_state[j] ∧ dir_state[j] == Up
hold26_anc（M1 主臂）= 纯祖先窗口替换自层；fusion_ta（M3 对照臂）= 自层 ∨ 祖先。

══════════════ 预注册判据（slow_bull §6，先于运行声明）══════════════

P1 BH 域修复（核心）：ES anc > fusion_t 在册 +369.1 且 BH gap 较 hold26
   口径收窄 ≥50%（G4 复活）；QQQ 同构（> +102.3，gap 收窄 ≥50%）。
   强分支 anc ≥ BH 不预期成立，若成立单独记录（BH 域被翻越第一例）。
P2 BTC 不损：anc(BTC) > hold26 +1247.1；与 fusion_t +4174.6 差距记录在案
   ——anc ≥ fusion_t ⇒ anc 升统一基座候选并废白名单；anc < fusion_t ⇒
   BTC 维持 fusion_t，anc 只接管 BH 域。
P3 GC/DX 零接触界：|anc − hold26| ≤ 5pp（GC +300.5 / DX +15.9 基准；
   代理预测 GC −1.3pp / DX −1.9pp）。
P4 熊市 α 保持：有熊市年标的 bear α(anc) ≥ bear α(hold26) − 0.25 nats
   且符号全正（代理泄漏上界 BTC −0.24）。
P5 MDD 界：anc MDD ≥ BH_MDD − 5pp 逐标的。
P6 声部域不翻转：OKLO/BRN anc < V2oa25_ht 在册（G3 同构）。
P7 机制对齐：豁免触发 bar 分布与调研 §1.4 occupancy 一致（anc_up_bars
   读数）；ES 的 recL2（ladder 4）豁免拦截 ≥ 28 次（fusion_t 残余 E0 桶
   n=28 为下界；n_anc_exempt_blocks_by_ladder 直读）。

升基座判决规则：P1∧P3∧P4 全过 ⇒ anc 替代"fusion_t 白名单 + hold26 残留"
二分（P2 决定 BTC 归属）；P1 过而 P3 破 ⇒ regime 函数家族新例，anc 入
白名单部署；P1 不过 ⇒ 26:80 下沉假设在配额层否证，开放轴转 V2/回复侧。

guard-a：hold26 逐位复现 positional_fugue_multi 在册（容差 0.05pp）。
guard-b：fusion_t 逐位复现 fusion_t_eight 在册（容差 0.05pp）。

═══════════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/hold26_anc_backtest.py [SYM ...]
输出：analysis/data_cache/hold26_anc_<SYM>.json + hold26_anc_summary.json
"""

from __future__ import annotations

import json
import math
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
MODES = ["hold26", "fusion_t", "hold26_anc", "fusion_ta"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
REGIME_NATS = 0.10
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}
# 在册基准（slow_bull §6 P1-P3 引用值；guard 用逐位文件，此表只供判据阈值）
FUSION_T_REF = {"ES": 369.1, "QQQ": 102.3, "BTC": 4174.6}
HOLD26_REF = {"BTC": 1247.1, "GC": 300.5, "DX": 15.9}
DOMAIN = {"BTC": "trend", "CL": "trend", "GC": "trend", "DX": "trend",
          "OKLO": "osc_voice", "BRN": "osc_voice",
          "ES": "bh", "QQQ": "bh"}


def bh_mdd(closes) -> float:
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def analyze(res: dict, closes, years) -> dict:
    """单模式结果 → NAV 重建守卫 + MDD + 分年/regime + 分层归因 + anc 观测。"""
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, *_pol) in trades:
        assert not _pol or _pol[0] == "long", f"anc 臂不产 short 行：{_pol}"
        dshares[eb] += sh
        dshares[xb] -= sh
        dcash[eb] -= sh * ep
        dcash[xb] += sh * xp
    nav = [0.0] * n
    shares = 0.0
    pool = 100_000.0
    peak = mdd = 0.0
    yearly: dict[str, dict] = {}
    prev_expo = 0.0
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        nav[i] = pool + shares * closes[i]
        peak = max(peak, nav[i])
        mdd = min(mdd, nav[i] / peak - 1.0)
        if years is not None and i >= 1:
            y = yearly.setdefault(str(years[i]),
                                  {"bh_log": 0.0, "strat_log": 0.0,
                                   "flat_missed": 0.0, "exp_sum": 0.0,
                                   "bars": 0})
            r_mkt = math.log(closes[i] / closes[i - 1])
            y["bh_log"] += r_mkt
            y["strat_log"] += math.log(nav[i] / nav[i - 1])
            y["flat_missed"] += (1.0 - prev_expo) * r_mkt
            y["exp_sum"] += prev_expo
            y["bars"] += 1
        prev_expo = shares * closes[i] / nav[i] if nav[i] > 0 else 0.0
    assert abs(nav[-1] - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{nav[-1]} ≠ {res['final_nav']}"
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        for k in ("bh_log", "strat_log", "flat_missed"):
            y[k] = round(y[k], 4)

    if years is None:
        yearly_out = None
        regime = None
    else:
        regime_years: dict[str, list] = {"bull": [], "bear": [], "range": []}
        for yk, v in sorted(yearly.items()):
            if v["bh_log"] >= REGIME_NATS:
                regime_years["bull"].append(yk)
            elif v["bh_log"] <= -REGIME_NATS:
                regime_years["bear"].append(yk)
            else:
                regime_years["range"].append(yk)

        def agg(ys):
            b = sum(yearly[y]["bh_log"] for y in ys)
            s = sum(yearly[y]["strat_log"] for y in ys)
            return {"bh_log": round(b, 4), "strat_log": round(s, 4),
                    "alpha": round(s - b, 4), "years": ys}

        regime = {r: agg(ys) for r, ys in regime_years.items()}
        yearly_out = yearly

    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, *_pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        d = by_ladder.setdefault(lad, {"n": 0, "pnl_cash": 0.0, "wins": 0,
                                       "w_sum": 0.0, "held": 0})
        d["n"] += 1
        pnl = sh * (xp - ep)
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["w_sum"] += w
        d["held"] += xb - eb
    for lad, d in by_ladder.items():
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_weight"] = round(d["w_sum"] / d["n"], 4)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0)
        del d["w_sum"], d["held"]

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "trend_holds_by_ladder": res["n_trend_holds_by_ladder"],
        "anc_exempt_blocks_by_ladder": res["n_anc_exempt_blocks_by_ladder"],
        "anc_up_bars_by_ladder": res["anc_up_bars_by_ladder"],
        "yearly": yearly_out,
        "regime": regime,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
    }


def prereg(sym: str, modes: dict, bh_pct: float, bh_mdd_pct: float,
           ht: float, hold26_book: float, ft_book: float | None,
           n_bars: int) -> dict:
    """P1-P7 逐标的裁决（判据全文见模块 docstring，先于运行声明）。"""
    anc, ta = modes["hold26_anc"], modes["fusion_ta"]
    ft, h = modes["fusion_t"], modes["hold26"]
    out: dict = {"domain": DOMAIN[sym]}
    if DOMAIN[sym] == "bh":
        ref = FUSION_T_REF[sym]
        gap_h = bh_pct - h["strat_pct"]
        gap_a = bh_pct - anc["strat_pct"]
        narrowed = (gap_h - gap_a) / gap_h if gap_h > 0 else None
        out["P1_anc_gt_ft_book"] = [anc["strat_pct"], ref,
                                    anc["strat_pct"] > ref]
        out["P1_gap_narrow_ge_half"] = [
            round(gap_a, 1), round(gap_h, 1),
            None if narrowed is None else round(narrowed, 3),
            narrowed is not None and narrowed >= 0.5]
        out["P1_strong_anc_ge_bh"] = [anc["strat_pct"], round(bh_pct, 1),
                                      anc["strat_pct"] >= bh_pct]
    if sym == "BTC":
        out["P2_anc_gt_hold26"] = [anc["strat_pct"], HOLD26_REF["BTC"],
                                   anc["strat_pct"] > HOLD26_REF["BTC"]]
        out["P2_vs_ft_record"] = [anc["strat_pct"], FUSION_T_REF["BTC"],
                                  anc["strat_pct"] >= FUSION_T_REF["BTC"]]
    if sym in ("GC", "DX"):
        delta = abs(anc["strat_pct"] - h["strat_pct"])
        out["P3_zero_touch"] = [round(delta, 1), delta <= 5.0]
    if anc["regime"] is not None:
        ab, hb = anc["regime"]["bear"], h["regime"]["bear"]
        if ab["years"]:
            out["P4_bear_alpha_sign"] = [ab["alpha"], ab["alpha"] > 0]
            out["P4_bear_alpha_tolerance"] = [
                ab["alpha"], hb["alpha"], ab["alpha"] >= hb["alpha"] - 0.25]
    out["P5_mdd_within_bh"] = [anc["mdd_pct"], round(bh_mdd_pct, 1),
                               anc["mdd_pct"] >= bh_mdd_pct - 5.0]
    if DOMAIN[sym] == "osc_voice":
        out["P6_anc_below_ht"] = [anc["strat_pct"], round(ht, 1),
                                  anc["strat_pct"] < ht]
    # P7 机制对齐：豁免窗口 occupancy + ES recL2 拦截下界
    occ = {LADDER_NAMES.get(k, str(k)): round(v / n_bars, 4)
           for k, v in enumerate(anc["anc_up_bars_by_ladder"]) if v > 0}
    out["P7_anc_occupancy"] = occ
    out["P7_exempt_blocks"] = {
        LADDER_NAMES.get(k, str(k)): v
        for k, v in enumerate(anc["anc_exempt_blocks_by_ladder"]) if v > 0}
    if sym == "ES":
        n_rec2 = anc["anc_exempt_blocks_by_ladder"][4]
        out["P7_es_recl2_ge_28"] = [n_rec2, n_rec2 >= 28]
    # 对照臂记录（fusion_ta vs hold26_anc——自层窗口的边际贡献）
    out["M3_ta_vs_anc"] = [ta["strat_pct"], anc["strat_pct"],
                           round(ta["strat_pct"] - anc["strat_pct"], 1)]
    out["guard_a_hold26_in_book"] = [
        h["strat_pct"], hold26_book,
        abs(h["strat_pct"] - hold26_book) < 0.05]
    if ft_book is not None:
        out["guard_b_ft_in_book"] = [
            ft["strat_pct"], ft_book,
            abs(ft["strat_pct"] - ft_book) < 0.05]
    return out


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%",
          flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)

    ref = json.loads((DATA_DIR / f"osc_scco_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    ht = ref["cells"]["V2oa25_ht"]["metrics"]["total_compound"]
    multi = json.loads(
        (DATA_DIR / f"positional_fugue_multi_{sym}.json").read_text())
    hold26_book = multi["modes"]["hold26"]["strat_pct"]
    ft_book = None
    ft_file = DATA_DIR / f"fusion_t_eight_{sym}.json"
    if ft_file.exists():
        ft_doc = json.loads(ft_file.read_text())
        if "modes" in ft_doc:
            ft_book = ft_doc["modes"]["fusion_t"]["strat_pct"]
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "ht_in_book": round(ht, 1), "hold26_in_book": hold26_book,
           "ft_in_book": ft_book,
           "design": "hold26_anc 祖先趋势豁免 P1-P7（slow_bull §6 预注册）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} reasons={a['exit_reasons']}",
              flush=True)
    out["preregistered"] = prereg(sym, out["modes"], bh_pct, bh_dd,
                                  ht, hold26_book, ft_book, n)
    print(f"[{sym}] prereg={json.dumps(out['preregistered'])}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"hold26_anc_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "bh_mdd_pct": out["bh_mdd_pct"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_mdd"] = m["mdd_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "hold26_anc_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
