"""V2r 逐笔交易行为分解 — 统计聚合（2026-06-11 任务）。

输入（cargo test trade_behavior 纯 Rust 落盘）：
  data_cache/_trade_behavior_{SYM}.json — open_log/close_log/rev_legs/trades
  data_cache/_tape_v2r_{SYM}_ts.i64    — 清洗后逐 bar epoch 秒（naive 原生时区）

本脚本只做 JSON 统计聚合（任务允许：信号层/交易层均不走 Python）。
用法：python3 analysis/_trade_behavior_analysis.py OKLO BRN
"""

from __future__ import annotations

import json
import struct
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

DATA = Path(__file__).resolve().parent / "data_cache"
REASON = {5: "t5_paired_buy1", 6: "t6_pre_buy3", 7: "t7_buy3", 8: "zd_touch",
          0: "forced(master/eod)"}
TRIGGER = {1: "type1_sell", 2: "consol_div_sell", 3: "both", 4: "sell3_escape"}


def load(sym: str):
    d = json.loads((DATA / f"_trade_behavior_{sym}.json").read_text())
    raw = (DATA / f"_tape_v2r_{sym}_ts.i64").read_bytes()
    ts = struct.unpack(f"<{len(raw) // 8}q", raw)
    assert len(ts) == d["n_bars"], "ts 与磁带 bar 数不一致"
    # join：rev_legs (ladder, sell_bar, sell_p, buy_bar, buy_p, shares, diff, profit)
    open_by = {(o[0], o[1]): o for o in d["open_log"]}
    close_by = {(c[0], c[1]): c for c in d["close_log"]}
    legs = []
    for lad, sb, sp, bb, bp, sh, df, pf in d["rev_legs"]:
        o = open_by.get((lad, sb))
        c = close_by.get((lad, sb))
        assert o is not None, f"rev leg (lad={lad}, sell_bar={sb}) 无 open_log 对应"
        legs.append({
            "ladder": lad, "open_bar": sb, "close_bar": bb,
            "open_price": sp, "close_price": bp, "profit": pf,
            "diff": df, "shares": sh,
            "trigger": o[3], "anchor_cs": o[4], "zd": o[5], "zg": o[6],
            "reason": c[3] if c is not None else 0,
            "hold_bars": bb - sb,
            "open_dt": datetime.fromtimestamp(ts[sb], tz=timezone.utc),
            "close_dt": datetime.fromtimestamp(ts[bb], tz=timezone.utc),
        })
    legs.sort(key=lambda x: x["open_bar"])
    return d, ts, legs


def fmt_dist(counter: Counter, total: int) -> dict:
    return {str(k): f"{v} ({v / total * 100:.1f}%)"
            for k, v in sorted(counter.items())}


def runs_of(signs: list[int]) -> tuple[Counter, Counter, int, int]:
    win_runs, loss_runs = Counter(), Counter()
    i = 0
    while i < len(signs):
        j = i
        while j < len(signs) and signs[j] == signs[i]:
            j += 1
        (win_runs if signs[i] > 0 else loss_runs)[j - i] += 1
        i = j
    max_w = max(win_runs) if win_runs else 0
    max_l = max(loss_runs) if loss_runs else 0
    return win_runs, loss_runs, max_w, max_l


def analyze(sym: str) -> dict:
    d, ts, legs = load(sym)
    out: dict = {"symbol": sym, "n_legs": len(legs),
                 "n_open": len(d["open_log"]), "n_forced": len(d["open_log"]) - len(d["close_log"])}
    wins = [l for l in legs if l["profit"] > 0]
    losses = [l for l in legs if l["profit"] <= 0]
    out["n_win"], out["n_loss"] = len(wins), len(losses)
    out["net_cash"] = round(sum(l["profit"] for l in legs), 1)

    # 1. 亏损腿时间分布（开腿时刻；UTC=数据原生口径）
    out["loss_by_hour"] = fmt_dist(Counter(l["open_dt"].hour for l in losses), len(losses))
    out["loss_by_weekday"] = fmt_dist(
        Counter(l["open_dt"].strftime("%a") for l in losses), len(losses))
    out["win_by_hour"] = fmt_dist(Counter(l["open_dt"].hour for l in wins), len(wins))
    out["all_by_hour"] = fmt_dist(Counter(l["open_dt"].hour for l in legs), len(legs))

    # 2. 连胜/连败序列（开腿时序）
    signs = [1 if l["profit"] > 0 else -1 for l in legs]
    wr, lr, mw, ml = runs_of(signs)
    out["win_runs"] = dict(sorted(wr.items()))
    out["loss_runs"] = dict(sorted(lr.items()))
    out["max_win_streak"], out["max_loss_streak"] = mw, ml

    # 3. REV 腿现金曲线最大回撤 + 落点结构
    cum, peak, mdd, mdd_lo, mdd_hi, peak_i = 0.0, 0.0, 0.0, 0, 0, 0
    legs_by_close = sorted(legs, key=lambda x: x["close_bar"])
    for i, l in enumerate(legs_by_close):
        cum += l["profit"]
        if cum > peak:
            peak, peak_i = cum, i
        if cum - peak < mdd:
            mdd, mdd_lo, mdd_hi = cum - peak, peak_i, i
    dd_legs = legs_by_close[mdd_lo + 1: mdd_hi + 1]
    out["rev_cash_mdd"] = round(mdd, 1)
    out["rev_cash_peak"] = round(peak, 1)
    if dd_legs:
        out["mdd_window"] = {
            "from": str(dd_legs[0]["open_dt"]), "to": str(dd_legs[-1]["close_dt"]),
            "n_legs": len(dd_legs),
            "n_loss": sum(1 for l in dd_legs if l["profit"] <= 0),
            "reason_dist": dict(Counter(REASON[l["reason"]] for l in dd_legs)),
            "trigger_dist": dict(Counter(TRIGGER[l["trigger"]] for l in dd_legs)),
            "n_anchors": len({l["anchor_cs"] for l in dd_legs}),
            "avg_depth_pct": round(sum((l["zg"] - l["zd"]) / l["open_price"]
                                       for l in dd_legs) / len(dd_legs) * 100, 2),
        }

    # 4. 盈亏比按开腿触发
    out["by_trigger"] = {}
    for trig, name in TRIGGER.items():
        sub = [l for l in legs if l["trigger"] == trig]
        if not sub:
            continue
        w = [l["profit"] for l in sub if l["profit"] > 0]
        lo = [-l["profit"] for l in sub if l["profit"] <= 0]
        out["by_trigger"][name] = {
            "n": len(sub),
            "win_rate": round(len(w) / len(sub) * 100, 1),
            "net_cash": round(sum(l["profit"] for l in sub), 1),
            "avg_win": round(sum(w) / len(w), 1) if w else None,
            "avg_loss": round(sum(lo) / len(lo), 1) if lo else None,
            "payoff": round((sum(w) / len(w)) / (sum(lo) / len(lo)), 2)
                      if w and lo and sum(lo) > 0 else None,
        }

    # 5. 持仓时长（盈 vs 亏，单位 bar=1min）
    def hold_stats(sub):
        hs = sorted(l["hold_bars"] for l in sub)
        if not hs:
            return None
        return {"n": len(hs), "median": hs[len(hs) // 2],
                "mean": round(sum(hs) / len(hs), 1),
                "p90": hs[int(len(hs) * 0.9)], "max": hs[-1]}

    out["hold_win"] = hold_stats(wins)
    out["hold_loss"] = hold_stats(losses)

    # 6. 闭腿类型分布 × 盈亏
    out["by_reason"] = {}
    for r, name in REASON.items():
        sub = [l for l in legs if l["reason"] == r]
        if not sub:
            continue
        w = sum(1 for l in sub if l["profit"] > 0)
        out["by_reason"][name] = {
            "n": len(sub), "win_rate": round(w / len(sub) * 100, 1),
            "net_cash": round(sum(l["profit"] for l in sub), 1),
            "median_hold": sorted(l["hold_bars"] for l in sub)[len(sub) // 2],
        }

    # 7. 同锚重复开腿（中枢阶段代理：同一锚中枢的第 k 次开腿）
    seq_of_anchor: dict = {}
    by_seq: dict = {}
    for l in legs:
        k = seq_of_anchor.get(l["anchor_cs"], 0) + 1
        seq_of_anchor[l["anchor_cs"]] = k
        by_seq.setdefault(min(k, 4), []).append(l["profit"])
    out["by_anchor_seq"] = {
        (f"{k}" if k < 4 else "4+"): {
            "n": len(v), "win_rate": round(sum(1 for p in v if p > 0) / len(v) * 100, 1),
            "net_cash": round(sum(v), 1)}
        for k, v in sorted(by_seq.items())}
    out["n_distinct_anchors"] = len(seq_of_anchor)
    return out


if __name__ == "__main__":
    for sym in sys.argv[1:]:
        print(json.dumps(analyze(sym.upper()), indent=1, ensure_ascii=False))
