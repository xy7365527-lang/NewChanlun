"""区间套正向定位八标的 A/B —— nest_forward（fusion_tn/fusion_trn）vs confirmed 基线。

任务（2026-06-12 编排者）：确认滞后是系统终极瓶颈。当前 confirmed 机制 =
"从下往上等完成"（掩码仅由 confirmed 事件置位，organic_signals.py:89）；
区间套正向用法 = "从上往下定位"（高级别 candidate 进入背驰段 → 次级别第一个
同侧证据 = 精确操作点，不等本级别 confirmed）。

原文依据（逐字，调研报告 §1）：
- chan99/0027:5  "区间套：就是根据背驰段从高级别向低级别逐级寻找背驰点的方法"
- chan99/0027:19 "本级别进入背驰段后，到次级别去寻找背驰点"
- blog/038:258   "不是等真跌了才问卖不卖，而是涨的时候一旦进入背驰的区间套里，
                  就要陆续走"（明确反对等完成）
- blog/027:42    精确大转折点寻找程序定理
- 否定词汇：blog/027:25 "只要没有打破背驰段，就要密切注意"（逆否：打破即作废）

实装（rust/src/trading/positional_fusion.rs nest_forward 轴）：
  武装 = 本级别 candidate Type1/Type3 事件；触发 = 次级别(k−1)第一个同侧
  BSP/背驰事件（bi 层用方向翻转沿，SC 先例）；否定 = 价格越过 candidate 极值。
  触发与 confirmed 掩码是 ∨ 关系（同一买卖点的更早时间坐标，非新类别）。

══════════════ 预注册判据（先于盲测数据声明）══════════════

盲测域：八标的 fusion_tn/fusion_trn 从未运行。
N1 t 基座增益：fusion_tn ≥ fusion_t 逐标的计数（8 标的）。
N2 tr 基座增益：fusion_trn ≥ fusion_tr 逐标的计数（在册最优基座的最小差分）。
N3 BH 域收敛（核心假设检验）：ES/QQQ 上 gap(BH−strat) 相对基线收窄 ≥ 25%
   ——若确认滞后是 ES/QQQ 失血主因（"幻影下跌"：等确认时回调已走完），
   提前触发应显著收窄差距；不收窄 ⇒ 滞后假设对 BH 域被否证。
N4 机制非零：n_nest_fire_{sell,buy} > 0 至少一侧逐标的——零触发 = 词汇空集，
   判决无效（非否证）。
N5 滞后测量（独立于 A/B，任务第 3 项）：candidate→confirmed 配对的
   bar 滞后与错过行情 %（按 ladder 分桶）——纯观测，无判据。

裁决规则（预注册）：N1∧N2 ≥ 6/8 且 N3 成立 ⇒ nest 升 fusion 基座候选；
N1/N2 分裂 ⇒ regime 函数（在册第十例候选），白名单 = 胜出标的集；
全负 ⇒ 正向定位在本词汇（k−1 单层截断）下被否证，开放轴 = 递归到最低级别。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/interval_nesting_backtest.py [SYM ...]
输出：analysis/data_cache/interval_nesting_<SYM>.json + interval_nesting_summary.json
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
from fugue_version_i import LADDER_SEG, MAX_LADDER  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
MODES = ["fusion_t", "fusion_tn", "fusion_tr", "fusion_trn"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}


def bh_mdd(closes) -> float:
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def lag_stats(tape, closes) -> dict:
    """N5：candidate→confirmed 滞后测量（任务第 3 项）。

    配对键 = (ladder, kind, side, cs)——与 Rust lead 配对同口径（type1 的
    seg_idx 随 C 段延伸漂移，cs 锚稳定）。仅 type1/type3（type2 的 confirmed
    是同 bar 价格比较，无时间等待）。
    missed_pct = confirmed bar 收盘相对 candidate bar 收盘的变动 %（卖点为负
    = 等确认期间价格已跌掉的"幻影行情"）。
    """
    first_cand: dict = {}   # key -> (bar, close)
    per_ladder: dict = {}
    orphans = 0             # candidate 无 confirmed（含被否定/截尾）
    instant = 0             # confirmed 首现无前置 candidate（滞后 0 不入桶）
    for i, s in enumerate(tape):
        if not s.bsp_events:
            continue
        for lad in range(MAX_LADDER):
            for (kind, side, _seg, confirmed, cs, _zd, _zg, _price) in \
                    s.bsp_events[lad]:
                if kind == "type2" or cs is None:
                    continue
                key = (lad, kind, side, cs)
                if not confirmed:
                    if key not in first_cand:
                        first_cand[key] = (i, closes[i])
                else:
                    if key in first_cand:
                        cb, cc = first_cand.pop(key)
                        d = per_ladder.setdefault(lad, {
                            "n": 0, "lag_sum": 0, "lags": [],
                            "missed_sum": 0.0})
                        lag = i - cb
                        d["n"] += 1
                        d["lag_sum"] += lag
                        d["lags"].append(lag)
                        sgn = 1.0 if side == "buy" else 1.0
                        move = (closes[i] - cc) / cc * 100.0 * sgn
                        # 卖点：负值 = 等确认期间已下跌的行情；买点：正值 =
                        # 等确认期间已反弹的行情（两侧都是入场/出场恶化）。
                        d["missed_sum"] += move if side == "sell" else -move
                    else:
                        instant += 1
    orphans = len(first_cand)
    out = {}
    for lad, d in sorted(per_ladder.items()):
        lags = sorted(d["lags"])
        out[LADDER_NAMES.get(lad, str(lad))] = {
            "n_pairs": d["n"],
            "mean_lag_bars": round(d["lag_sum"] / d["n"], 1),
            "median_lag_bars": lags[len(lags) // 2],
            "p90_lag_bars": lags[int(len(lags) * 0.9)],
            # 卖点=已跌幅/买点=已涨幅 的均值（负 = 行情损耗方向）
            "mean_missed_pct": round(d["missed_sum"] / d["n"], 3),
        }
    return {"by_ladder": out, "orphan_candidates": orphans,
            "instant_confirms": instant}


def analyze(res: dict, closes, years) -> dict:
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for t in trades:
        lad, eb, ep, xb, xp, sh = t[0], t[1], t[2], t[3], t[4], t[5]
        polarity = t[10] if len(t) > 10 else "long"
        assert polarity == "long", "本 harness 仅预注册多头模式"
        dshares[eb] += sh
        dshares[xb] -= sh
        dcash[eb] -= sh * ep
        dcash[xb] += sh * xp
    nav = [0.0] * n
    shares = 0.0
    pool = 100_000.0
    peak = mdd = 0.0
    yearly: dict = {}
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
                                   "exp_sum": 0.0, "bars": 0})
            y["bh_log"] += math.log(closes[i] / closes[i - 1])
            y["strat_log"] += math.log(nav[i] / nav[i - 1])
            y["exp_sum"] += prev_expo
            y["bars"] += 1
        prev_expo = shares * closes[i] / nav[i] if nav[i] > 0 else 0.0
    assert abs(nav[-1] - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{nav[-1]} ≠ {res['final_nav']}"
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        for k in ("bh_log", "strat_log"):
            y[k] = round(y[k], 4)

    reason_counts: dict = {}
    for t in trades:
        reason_counts[t[9]] = reason_counts.get(t[9], 0) + 1

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "yearly": yearly if years is not None else None,
        "nest": {
            "arms": res["n_nest_arms_by_ladder"],
            "fire_sell": res["n_nest_fire_sell_by_ladder"],
            "fire_buy": res["n_nest_fire_buy_by_ladder"],
            "breaks": res["n_nest_breaks_by_ladder"],
            "lead_bars_sum": res["nest_lead_bars_sum"],
            "lead_n": res["nest_lead_n"],
            "mean_lead_bars": round(
                res["nest_lead_bars_sum"] / res["nest_lead_n"], 1)
            if res["nest_lead_n"] else None,
        },
    }


def prereg(modes: dict, bh_pct: float) -> dict:
    ft, ftn = modes["fusion_t"]["strat_pct"], modes["fusion_tn"]["strat_pct"]
    fr, frn = modes["fusion_tr"]["strat_pct"], modes["fusion_trn"]["strat_pct"]
    gap_base = bh_pct - max(ft, fr)
    gap_nest = bh_pct - max(ftn, frn)
    fires = sum(modes["fusion_tn"]["nest"]["fire_sell"]) + \
        sum(modes["fusion_tn"]["nest"]["fire_buy"])
    return {
        "N1_tn_ge_t": ftn >= ft,
        "N1_delta_pp": round(ftn - ft, 1),
        "N2_trn_ge_tr": frn >= fr,
        "N2_delta_pp": round(frn - fr, 1),
        "N3_gap_shrink": round(1.0 - gap_nest / gap_base, 3)
        if gap_base > 0 else None,
        "N4_mechanism_nonzero": fires > 0,
        "n_nest_fires_total": fires,
    }


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
                     for l in range(MAX_LADDER)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(MAX_LADDER)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)

    lag = lag_stats(tape, closes)
    print(f"[{sym}] N5 滞后测量 {json.dumps(lag, ensure_ascii=False)}",
          flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)
    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "lag_stats": lag,
           "design": "区间套正向定位 A/B（判据见 docstring）",
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
    out["preregistered"] = prereg(out["modes"], bh_pct)
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
        (DATA_DIR / f"interval_nesting_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"]}
            for mode in MODES:
                row[mode] = out["modes"][mode]["strat_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "interval_nesting_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
