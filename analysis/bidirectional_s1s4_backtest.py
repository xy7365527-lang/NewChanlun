"""双向条件轴 S1-S4 回测——fusion_btr/fusion_btra vs fusion_tr 纯多头对照。

上游：`analysis/bidirectional_nested_accounting.md`（双向会计 L0 形式化 +
§8 实装序）+ `analysis/slow_bull_vs_bh_research.md` §7（探针2 六标的 24M bar
L2：空头增量 = (标的×层级) 二维 regime 符号函数，有效域 {BTC,CL,DX}×中间层，
条件轴 S1-S4 预注册）。

[镜像推导] 声明（S4 判据，090号）：全部空头侧机制无原文锚——84课做空程序 =
意向串行切换、头寸恒非负；84:316 是语料中对期货净空头的唯一直接判决且为负
（条件域 = 低流通逼空市况）。净空头身份 = 镜像推导 + 编排者决断。

══════════════ 预注册判据（先于运行声明）══════════════

条件轴（探针2 §7.3 空头增量列，零摩擦磁带测量参照）：
  S1: BTC move(L1)  +1.583 nats   S3: BTC recL2 +0.514 nats
  S2: CL  segment   +1.071 nats   S4: CL  recL2 +1.010 nats
⇒ 实装臂：BTC = fusion_btr_s34（move+recL2 翻空白名单）；
          CL  = fusion_btr_s24（segment+recL2）。
  条件化变体：fusion_btra_s{..}（镜像 anc 门：开空 iff ∃j>k Trend∧Down，
  探针 §7.3 镜像 anc 列 BTC move +1.465 / CL seg +1.140 为参照）。

B1 守卫（零接触）：fusion_tr 复现在册（BTC +4298.4 / CL +331.7，容差
   0.05pp）∧ tape_fp 与 p7_conj 在册逐项一致。失败 ⇒ 全部读数作废。
B2 核心判据（做空段正 alpha）：空头腿已实现净现金 > 0（按层分解），
   且 btr 总收益 > fusion_tr 在册。两条均过 = S 轴在引擎实装下成立；
   空头腿净现金 > 0 但总收益 ≤ fusion_tr ⇒ 空头段正但翻多回复侧受损
   （配额重入效应），机制归因单列。
B3 引擎≠磁带口径声明：探针2 是行级方向跟随（dir 窗口持空）；引擎是
   BSP 驱动（卖点开空/买点平空，确认滞后内嵌）。两口径在册同号互证
   （§1.2 配对 vs §7.3 行级），但量级不可直接比——nats 增量仅作方向
   参照，不构成预测（GC 符号翻转先例，§5.3 代理边界）。
B4 尾部风险观测：n_short_liquidations 计数；>0 ⇒ 逐笔解剖强平窗口。
B5 镜像 anc 门方向：btra 若 > btr（同标的）⇒ 窗口条件化有效（S2 形式
   被支持）；btra < btr ⇒ 无条件翻空更优，anc 镜像门关闭。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/bidirectional_s1s4_backtest.py [SYM ...]
输出：analysis/data_cache/bidir_s1s4_<SYM>.json + bidir_s1s4_summary.json
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

# S1-S4 白名单（探针2 §7.6 预注册；S3 界 ⇒ 全部 ≤ recL2）
MODES_BY_SYM = {
    "BTC": ["fusion_tr", "fusion_btr_s34", "fusion_btra_s34"],
    "CL": ["fusion_tr", "fusion_btr_s24", "fusion_btra_s24"],
}
# B1 守卫在册参照（p7_conj_<SYM>.json）
FUSION_TR_IN_BOOK = {"BTC": 4298.4, "CL": 331.7}
SYMBOLS = sys.argv[1:] or ["BTC", "CL"]


def bh_mdd(closes) -> float:
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def analyze(res: dict, closes, years) -> dict:
    """NAV 重建守卫（双极性现金流）+ MDD + 分年/regime + 空头腿分解。

    trade 行 11 元组（lib.rs 导出）：(..., exit_reason, polarity)。
    空头行现金流镜像：开空收 proceeds（+sh×ep）持仓 −sh；平空付买回款
    （−sh×xp）。proceeds 口径 NAV = pool + Σ shares×c（shares 带符号）
    与引擎内 margin 口径恒等（会计文档 §4.3 两口径恒等定理）。
    """
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

    # 空头腿分解（核心读数：做空段是否贡献正 alpha）
    short_by_ladder: dict = {}
    short_yearly: dict[str, float] = {}
    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
        d = by_ladder.setdefault((lad, pol), {"n": 0, "pnl_cash": 0.0,
                                              "wins": 0, "held": 0})
        d["n"] += 1
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["held"] += xb - eb
        if pol == "short":
            s = short_by_ladder.setdefault(lad, {"n": 0, "pnl_cash": 0.0,
                                                 "wins": 0, "held": 0})
            s["n"] += 1
            s["pnl_cash"] += pnl
            s["wins"] += pnl > 0
            s["held"] += xb - eb
            if years is not None:
                yk = str(years[min(xb, n - 1)])
                short_yearly[yk] = short_yearly.get(yk, 0.0) + pnl
    for d in list(by_ladder.values()) + list(short_by_ladder.values()):
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0)
        del d["held"]

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "yearly": yearly if years is not None else None,
        "regime": regime,
        "by_ladder": {f"{LADDER_NAMES.get(k, str(k))}/{pol}": v
                      for (k, pol), v in sorted(by_ladder.items())},
        "short_by_ladder": {LADDER_NAMES.get(k, str(k)): v
                            for k, v in sorted(short_by_ladder.items())},
        "short_yearly": {k: round(v, 0)
                         for k, v in sorted(short_yearly.items())},
        "short_counters": {
            "flips": res["n_flip_shorts_by_ladder"],
            "covers": res["n_short_covers_by_ladder"],
            "moveup_covers": res["n_short_moveup_covers_by_ladder"],
            "movedown_holds": res["n_short_trend_holds_by_ladder"],
            "r2_blocks": res["n_short_r2_blocks_by_ladder"],
            "anc_rejects": res["n_short_anc_rejects_by_ladder"],
            "liquidations": res["n_short_liquidations_by_ladder"],
            "held_bars": res["short_held_bars_by_ladder"],
            "net_cash": [round(x, 0) for x in res["short_net_cash_by_ladder"]],
        },
    }


def prereg(sym: str, modes: dict) -> dict:
    """预注册判据裁决（B1-B5，先于运行声明在模块 docstring）。"""
    tr = modes["fusion_tr"]
    btr = modes[MODES_BY_SYM[sym][1]]
    btra = modes[MODES_BY_SYM[sym][2]]
    out: dict = {}
    book = FUSION_TR_IN_BOOK[sym]
    out["B1_tr_in_book"] = [tr["strat_pct"], book,
                            abs(tr["strat_pct"] - book) < 0.05]
    short_cash = sum(btr["short_counters"]["net_cash"])
    out["B2_short_legs_positive"] = [round(short_cash, 0), short_cash > 0]
    out["B2_btr_beats_tr"] = [btr["strat_pct"], tr["strat_pct"],
                              btr["strat_pct"] > tr["strat_pct"]]
    out["B4_liquidations"] = [sum(btr["short_counters"]["liquidations"]),
                              sum(btra["short_counters"]["liquidations"])]
    out["B5_anc_gate_direction"] = [btra["strat_pct"], btr["strat_pct"],
                                    btra["strat_pct"] > btr["strat_pct"]]
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

    ref = json.loads((DATA_DIR / f"p7_conj_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "design": "双向条件轴 S1-S4（判据见探针 docstring，[镜像推导]）",
           "modes": {}}
    for mode in MODES_BY_SYM[sym]:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        sc = a["short_counters"]
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} flips={sum(sc['flips'])} "
              f"liq={sum(sc['liquidations'])} "
              f"short_cash={sum(sc['net_cash']):+.0f}",
              flush=True)
    out["preregistered"] = prereg(sym, out["modes"])
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
        (DATA_DIR / f"bidir_s1s4_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"]}
            for mode in MODES_BY_SYM[sym]:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_mdd"] = m["mdd_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "bidir_s1s4_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
