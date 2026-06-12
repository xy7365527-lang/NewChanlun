"""fusion_t（趋势停削单轴）八标的预注册复验——探索性消融读数升级 L3。

上游：hold26_counterseg_fusion_results.md §5.1（fusion_t 三标的全胜为消融臂
探索性读数，升基座需八标的预注册复验）+ hold26_multi_asset_results.md
（hold26 八标的 L3：正域 {BTC,CL,GC,DX} / 震荡声部域 {OKLO,BRN} / BH 域
{ES,QQQ}；牛市 α 6/6 全负 = flat_missed 结构缺陷）。
实现零接触：纯消费 rust/src/trading/positional_fusion.rs 在册
`PolarityMode::Fusion{trend_hold:true, counter_sub:false}`（mode="fusion_t"）。

══════════════ 预注册判据（先于盲测数据声明）══════════════

盲测域：BRN ES GC QQQ DX（五标的 fusion_t 从未运行）。
非盲域：BTC +4174.6% / OKLO +378.9% / CL +351.0%（hold26_cs_fusion 在册，
本次复跑为逐位复现守卫，不计入盲测证据）。

G1 全胜判据（升基座核心）：fusion_t ≥ hold26 逐标的，8/8。
    8/8 ⇒ fusion_t 替代 hold26 为 positional 新基座；
    分裂 ⇒ regime 函数（第九例），fusion_t 白名单 = 胜出标的集。
G2 正域超 BH：hold26 正域 {BTC,CL,GC,DX} 上 fusion_t > BH，4/4
    （BTC/CL 已知 ✓ 非盲；盲测 GC/DX）。
G3 震荡声部域不翻转：OKLO/BRN 上 fusion_t > hold26 但仍 < V2oa25_ht 在册
    （声部 alpha 粒度细于仓位削减的判决不被 T 轴翻转；OKLO 已知
    379 < 1800 非盲；盲测 BRN < 599）。
G4 BH 域收敛：ES/QQQ 上 fusion_t > hold26 且 BH 差距收窄 ≥ 50%
    （gap = BH − strat；停削在单边慢牛中趋向满仓不动）。
    强分支 fusion_t ≥ BH 先验不预期成立（停削只堵卖点漏出，买点回复
    延迟的暴露缺口仍在）——若成立则 BH 域被 T 轴翻正，单独记录。
G5 牛市赤字方向：有牛市年的标的 bull α(fusion_t) > bull α(hold26)
    全部一致（QQQ 无时间戳除外）。
G6 熊市 α 保持：bear α(fusion_t) 符号全正（有熊市年标的）；
    容差判据 bear α ≥ hold26 bear α − 0.10 nats 逐标的记录
    （BTC 已知破容差：1.83→1.30——bear rally 停削漏出是 49课二相
    结构成本，先验预期衰减 < bull 增益一个量级）。
G7 MDD 界：fusion_t MDD 不深于 BH MDD − 5pp（停削极限 = 满仓 ⇒
    回撤趋向 BH 回撤；超界 = 停削之外另有放大器，须解剖）。
GDX 机制盲预测（尖锐可证伪）：DX 九年全 range（无 bull 年，hold26 L3
    在册）⇒ 趋势相 (kind==Trend ∧ dir==Up) 触发面小 ⇒
    |fusion_t − hold26| ≤ 3pp（T 轴零接触预测）。
guard-a：hold26 八标的逐位复现 positional_fugue_multi 在册（容差 0.05pp）。
guard-b：fusion_t 三标的逐位复现 hold26_cs_fusion 在册（容差 0.05pp）。

升基座判决规则（预注册）：G1 8/8 ∧ G6 符号全正 ⇒ 升基座；
G1 分裂 ⇒ 白名单部署，检查胜出集与 bull 赤字量级的机制对齐
（预测：bull 赤字越大 T 轴增益越大；无 bull 年标的增益 ≈ 0）。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/fusion_t_eight_asset_backtest.py [SYM ...]
输出：analysis/data_cache/fusion_t_eight_<SYM>.json + fusion_t_eight_summary.json
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
MODES = ["hold26", "fusion_t"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
REGIME_NATS = 0.10
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}

# 在册守卫参照（guard-a / guard-b；缺文件 = 守卫不可执行 ⇒ 该标的 failed）
FUSION_T_IN_BOOK = {"BTC": 4174.6, "OKLO": 378.9, "CL": 351.0}
# hold26 L3 域分类（hold26_multi_asset_results.md §0，判据 G2/G3/G4 的域绑定）
DOMAIN = {"BTC": "trend", "CL": "trend", "GC": "trend", "DX": "trend",
          "OKLO": "osc_voice", "BRN": "osc_voice",
          "ES": "bh", "QQQ": "bh"}


def bh_mdd(closes) -> float:
    """满仓不动的最大回撤（G7 参照）。"""
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def analyze(res: dict, closes, years) -> dict:
    """单模式结果 → NAV 重建守卫 + MDD + 分年/regime + 分层归因。

    years=None（QQQ parallel-array 无时间戳）⇒ yearly/regime = None，
    不伪造时间轴。
    """
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason) in trades:
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
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason) in trades:
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
        "yearly": yearly_out,
        "regime": regime,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
    }


def prereg(sym: str, modes: dict, bh_pct: float, bh_mdd_pct: float,
           ht: float, hold26_book: float) -> dict:
    """预注册判据逐标的裁决（判据全文见模块 docstring，先于盲测声明）。"""
    ft, h = modes["fusion_t"], modes["hold26"]
    out: dict = {"domain": DOMAIN[sym], "blind": sym not in FUSION_T_IN_BOOK}
    out["G1_ft_ge_hold26"] = [ft["strat_pct"], h["strat_pct"],
                              ft["strat_pct"] >= h["strat_pct"]]
    if DOMAIN[sym] == "trend":
        out["G2_ft_gt_bh"] = [ft["strat_pct"], round(bh_pct, 1),
                              ft["strat_pct"] > bh_pct]
    if DOMAIN[sym] == "osc_voice":
        out["G3_ft_below_ht"] = [ft["strat_pct"], round(ht, 1),
                                 ft["strat_pct"] < ht]
    if DOMAIN[sym] == "bh":
        gap_h = bh_pct - h["strat_pct"]
        gap_f = bh_pct - ft["strat_pct"]
        narrowed = (gap_h - gap_f) / gap_h if gap_h > 0 else None
        out["G4_gap_narrow_ge_half"] = [
            round(gap_f, 1), round(gap_h, 1),
            None if narrowed is None else round(narrowed, 3),
            narrowed is not None and narrowed >= 0.5]
        out["G4_strong_ft_ge_bh"] = [ft["strat_pct"], round(bh_pct, 1),
                                     ft["strat_pct"] >= bh_pct]
    if ft["regime"] is not None:
        fb, hb = ft["regime"]["bull"], h["regime"]["bull"]
        if fb["years"]:
            out["G5_bull_alpha_improves"] = [fb["alpha"], hb["alpha"],
                                             fb["alpha"] > hb["alpha"]]
        fr, hr = ft["regime"]["bear"], h["regime"]["bear"]
        if fr["years"]:
            out["G6_bear_alpha_positive"] = [fr["alpha"], fr["alpha"] > 0]
            out["G6_bear_alpha_tolerance"] = [
                fr["alpha"], hr["alpha"], fr["alpha"] >= hr["alpha"] - 0.10]
    out["G7_mdd_within_bh"] = [ft["mdd_pct"], round(bh_mdd_pct, 1),
                               ft["mdd_pct"] >= bh_mdd_pct - 5.0]
    if sym == "DX":
        delta = abs(ft["strat_pct"] - h["strat_pct"])
        out["GDX_zero_touch"] = [round(delta, 1), delta <= 3.0]
    out["guard_a_hold26_in_book"] = [
        h["strat_pct"], hold26_book,
        abs(h["strat_pct"] - hold26_book) < 0.05]
    if sym in FUSION_T_IN_BOOK:
        out["guard_b_ft_in_book"] = [
            ft["strat_pct"], FUSION_T_IN_BOOK[sym],
            abs(ft["strat_pct"] - FUSION_T_IN_BOOK[sym]) < 0.05]
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
    scco = ref["cells"]["V2oa25_ht_scco"]["metrics"]["total_compound"]
    multi = json.loads(
        (DATA_DIR / f"positional_fugue_multi_{sym}.json").read_text())
    hold26_book = multi["modes"]["hold26"]["strat_pct"]
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "ht_in_book": round(ht, 1), "scco_in_book": round(scco, 1),
           "hold26_in_book": hold26_book,
           "design": "fusion_t 八标的预注册复验（判据见探针 docstring）",
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
                                  ht, hold26_book)
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
        (DATA_DIR / f"fusion_t_eight_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "bh_mdd_pct": out["bh_mdd_pct"],
                   "ht_in_book": out["ht_in_book"],
                   "scco_in_book": out["scco_in_book"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_mdd"] = m["mdd_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "fusion_t_eight_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
