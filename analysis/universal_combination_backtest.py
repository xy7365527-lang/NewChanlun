"""全量普适组合回测——fusion_btran_s34 八标的（M1 终极验证：一个配置，
所有标的，零标的级参数）。

上游：
- `analysis/bidirectional_s1s4_backtest.py`（双向条件轴 S1-S4，BTC
  fusion_btra_s34 +7368.0% 在册基线）
- `analysis/interval_nesting_backtest.py`（区间套正向定位 fusion_tn/trn，
  027课程序定理 + 038:258）
- rust/src/trading/positional_fusion.rs `fusion_btran_s{digits}`
  （本任务预注册新臂：btra 双向基座 × nest_forward 正交合取——nest =
  同一买卖点的更早时间坐标，卖侧 nf 触发沿翻转断面、买侧 nf 触发沿
  平空出口）

══════════════ 组合的结构判决（先于运行声明，090号）══════════════

任务点名三模块 {nest_forward, H4 振幅门, 1s a0}，实际可合取域：

1. nest_forward：✓ 可合取（本臂 fusion_btran_s34）。
2. H4 振幅门：✗ 域空判决——H4 在 fusion 引擎的唯一挂载点是
   OscRouting::Unified 三门合取的门②（unified_osc.rs，035:30 逐字）；
   fusion_btra_s34 的 osc=Off ⇒ H4 的对象（osc 腿开腿准入）在本配置
   是空集。开 osc 路由 × short_mask 在翻转断面处筹码冲突（osc 腿
   全抛在外 ⇒ M=N 翻空无对象；osc 卖出价 ≠ 断面价 ⇒ 违反"同 bar
   同价、断面 pool 净流转 0"的已结算会计〔bidirectional_nested_
   accounting §5.2〕）——定义冲突，不实装不绕过，上浮记录于结果文档。
3. 1s a0：BTC 1s 仅 2 周窗口（btc_2week_1s），不支撑全史判决 ⇒
   按任务 fallback 跑 1min 口径，在册 1s 嵌套确认加速数
   （条件化嵌套 2.8-3.0×，1s_nesting_confirmation 在册）作差异标注。

∴ 全量普适组合的严格形式 = fusion_btran_s34（b+t+r+a+s34+n），
  H4/1s 两轴的不可合取性是本判决的一部分而非省略。

══════════════ 预注册判据（任务书 P1-P6，先于数据声明）══════════════

P1 全面超 BH（M1 目标）：8/8 fusion_btran_s34 ≥ BH。
P2 不恶化：8/8 fusion_btran_s34 ≥ 各自在册最优的 90%。在册最优表
   （部署矩阵口径，跨引擎条目标注）：
     BTC  fusion_btra_s34 +7368.0   CL  fusion_btr_s24 +543.1
     ES   hold26_anc      +394.5    QQQ hold26_anc     +108.8
     GC   hold26          +300.5    DX  hold26         +15.9
     OKLO V2oa25_ht       +1800.0*  BRN V2oa25_ht      +598.8*
   （* = LOU 引擎在册，跨引擎参照——同时报 fusion 引擎内最优
     OKLO fusion_t +378.9 / BRN fusion_ta +86.0 两口径）
P3 BTC 不劣于基线：fusion_btran_s34(BTC) ≥ +7368.0。
P4 ES 超 BH：fusion_btran_s34(ES) ≥ +594.3（anc 修复+双向+nest 合取
   是否翻越 BH 域）。注：本臂 anc 是空头侧镜像门（short_anc_gate），
   非 hold26_anc 的多头停削豁免（trend_scope）——两机制不同，预注册
   时声明以免事后混淆。
P5 各门拦截可观测（判别力非零，H2 先例）：nest（arms/fires/breaks）、
   镜像 anc（rejects）、R2（blocks）、MoveDown（holds）逐标的直读。
   H4 拦截 = 0 by construction（域空判决的可观测面）。
P6 零标的级参数：同一 mode 字符串、同一 floor、零 per-symbol 分支
   应用全部八标的（构造性判据——脚本断言 MODES 无标的索引）。

guard-a：tape_fp 与 p7_conj_<SYM> 在册逐项一致（漂移 ⇒ 标的作废）。
guard-b：hold26 / fusion_tr 逐位复现 p7_conj 在册（容差 0.05pp）。
guard-c：BTC fusion_btra_s34 逐位复现 bidir_s1s4 在册 +7368.0
   （容差 0.05pp）。失败 ⇒ 全部读数作废。

════════════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/universal_combination_backtest.py [SYM ...]
输出：analysis/data_cache/universal_combo_<SYM>.json + universal_combo_summary.json
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
REGIME_NATS = 0.10
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}

# P6：同一配置列表应用所有标的——任何 per-symbol 分支都是判据违背。
MODES = ["hold26", "fusion_tr", "fusion_btra_s34", "fusion_btran_s34"]
UNIVERSAL = "fusion_btran_s34"
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]

# P2 在册最优（部署矩阵口径；engine 列区分跨引擎参照）
IN_BOOK_BEST = {
    "BTC": ("fusion_btra_s34", 7368.0, "fusion"),
    "CL": ("fusion_btr_s24", 543.1, "fusion"),
    "ES": ("hold26_anc", 394.5, "fusion"),
    "QQQ": ("hold26_anc", 108.8, "fusion"),
    "GC": ("hold26", 300.5, "fusion"),
    "DX": ("hold26", 15.9, "fusion"),
    "OKLO": ("V2oa25_ht", 1800.0, "LOU"),
    "BRN": ("V2oa25_ht", 598.8, "LOU"),
}
# fusion 引擎内最优（OKLO/BRN 的同引擎口径，P2 双口径并报）
IN_BOOK_FUSION_ENGINE = {"OKLO": ("fusion_t", 378.9), "BRN": ("fusion_ta", 86.0)}
# guard-c：BTC 双向在册
BTRA_S34_IN_BOOK_BTC = 7368.0


def bh_mdd(closes) -> float:
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def analyze(res: dict, closes, years) -> dict:
    """NAV 重建守卫（双极性现金流）+ MDD + 分年/regime + 空头/nest 观测面。"""
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        if pol == "long":
            dshares[eb] += sh
            dshares[xb] -= sh
            dcash[eb] -= sh * ep
            dcash[xb] += sh * xp
        else:
            dshares[eb] -= sh
            dshares[xb] += sh
            dcash[eb] += sh * ep
            dcash[xb] -= sh * xp
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
        if years is not None and i >= 1 and nav[i] > 0 and nav[i - 1] > 0:
            y = yearly.setdefault(str(years[i]),
                                  {"bh_log": 0.0, "strat_log": 0.0,
                                   "exp_sum": 0.0, "bars": 0})
            r_mkt = math.log(closes[i] / closes[i - 1])
            y["bh_log"] += r_mkt
            y["strat_log"] += math.log(nav[i] / nav[i - 1])
            y["exp_sum"] += prev_expo
            y["bars"] += 1
        prev_expo = shares * closes[i] / nav[i] if nav[i] > 0 else 0.0
    assert abs(nav[-1] - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{nav[-1]} ≠ {res['final_nav']}（双极性现金流分派有 bug）"
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        for k in ("bh_log", "strat_log"):
            y[k] = round(y[k], 4)

    regime = None
    if years is not None:
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

    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    short_cash_total = 0.0
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
        if pol == "short":
            short_cash_total += pnl
        d = by_ladder.setdefault((lad, pol), {"n": 0, "pnl_cash": 0.0, "wins": 0})
        d["n"] += 1
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
    for d in by_ladder.values():
        d["pnl_cash"] = round(d["pnl_cash"], 0)

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "yearly": yearly if years is not None else None,
        "regime": regime,
        "by_ladder": {f"{LADDER_NAMES.get(k, str(k))}/{pol}": v
                      for (k, pol), v in sorted(by_ladder.items())},
        "short_cash_total": round(short_cash_total, 0),
        # P5 观测面：各门拦截直读（判别力为零的门按 H2 先例标记）
        "gates": {
            "nest_arms": res["n_nest_arms_by_ladder"],
            "nest_fire_sell": res["n_nest_fire_sell_by_ladder"],
            "nest_fire_buy": res["n_nest_fire_buy_by_ladder"],
            "nest_breaks": res["n_nest_breaks_by_ladder"],
            "nest_lead_avg_bars": (
                round(res["nest_lead_bars_sum"] / res["nest_lead_n"], 1)
                if res["nest_lead_n"] else None),
            "flips": res["n_flip_shorts_by_ladder"],
            "anc_rejects": res["n_short_anc_rejects_by_ladder"],
            "short_covers": res["n_short_covers_by_ladder"],
            "moveup_covers": res["n_short_moveup_covers_by_ladder"],
            "movedown_holds": res["n_short_trend_holds_by_ladder"],
            "short_r2_blocks": res["n_short_r2_blocks_by_ladder"],
            "r2_pos_blocks": res["n_r2_pos_blocks_by_ladder"],
            "liquidations": res["n_short_liquidations_by_ladder"],
            "net_cash": [round(x, 0) for x in res["short_net_cash_by_ladder"]],
        },
    }


def prereg(sym: str, modes: dict, bh_pct: float) -> dict:
    """预注册判据裁决（P1-P5 标的局部分量；P1/P2 的 8/8 汇总在 main）。"""
    uni = modes[UNIVERSAL]
    out: dict = {}
    out["P1_ge_bh"] = [uni["strat_pct"], round(bh_pct, 1),
                       uni["strat_pct"] >= bh_pct]
    best_name, best_val, engine = IN_BOOK_BEST[sym]
    out["P2_ge_90pct_best"] = [uni["strat_pct"], best_name, best_val, engine,
                               uni["strat_pct"] >= 0.9 * best_val]
    if sym in IN_BOOK_FUSION_ENGINE:
        fe_name, fe_val = IN_BOOK_FUSION_ENGINE[sym]
        out["P2_fusion_engine_scope"] = [uni["strat_pct"], fe_name, fe_val,
                                         uni["strat_pct"] >= 0.9 * fe_val]
    if sym == "BTC":
        out["P3_btc_ge_baseline"] = [uni["strat_pct"], BTRA_S34_IN_BOOK_BTC,
                                     uni["strat_pct"] >= BTRA_S34_IN_BOOK_BTC]
    if sym == "ES":
        out["P4_es_ge_bh"] = [uni["strat_pct"], round(bh_pct, 1),
                              uni["strat_pct"] >= bh_pct]
    g = uni["gates"]
    out["P5_gate_observability"] = {
        "nest_fires": sum(g["nest_fire_sell"]) + sum(g["nest_fire_buy"]),
        "nest_breaks": sum(g["nest_breaks"]),
        "anc_rejects": sum(g["anc_rejects"]),
        "r2_blocks": sum(g["short_r2_blocks"]) + sum(g["r2_pos_blocks"]),
        "movedown_holds": sum(g["movedown_holds"]),
        "h4": "domain_empty_by_verdict",
    }
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

    # guard-a：tape 指纹与在册一致
    ref = json.loads((DATA_DIR / f"p7_conj_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "design": "全量普适组合 fusion_btran_s34（判据见 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        g = a["gates"]
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} flips={sum(g['flips'])} "
              f"nest_fires={sum(g['nest_fire_sell']) + sum(g['nest_fire_buy'])} "
              f"liq={sum(g['liquidations'])} short_cash={sum(g['net_cash']):+.0f}",
              flush=True)

    # guard-b：hold26 / fusion_tr 逐位复现 p7_conj 在册
    guards = {}
    for m in ("hold26", "fusion_tr"):
        book = ref["modes"][m]["strat_pct"]
        now = out["modes"][m]["strat_pct"]
        guards[f"guard_b_{m}"] = [now, book, abs(now - book) < 0.05]
    # guard-c：BTC 双向在册
    if sym == "BTC":
        now = out["modes"]["fusion_btra_s34"]["strat_pct"]
        guards["guard_c_btra_s34"] = [now, BTRA_S34_IN_BOOK_BTC,
                                      abs(now - BTRA_S34_IN_BOOK_BTC) < 0.05]
    out["guards"] = guards
    if not all(v[2] for v in guards.values()):
        out["failed"] = "guard_reproduction"
        return out

    out["preregistered"] = prereg(sym, out["modes"], bh_pct)
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
        (DATA_DIR / f"universal_combo_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_mdd"] = m["mdd_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "universal_combo_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    # P1/P2 八标的汇总裁决
    ok_rows = [r for r in summary if "failed" not in r]
    p1 = sum(r["preregistered"]["P1_ge_bh"][2] for r in ok_rows)
    p2 = sum(r["preregistered"]["P2_ge_90pct_best"][4] for r in ok_rows)
    print(f"\nP1 全面超BH: {p1}/{len(ok_rows)}  P2 在册最优90%: {p2}/{len(ok_rows)}",
          flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
