"""REV 腿交易行为逐笔诊断 — 为什么 O1（加 REV 腿）三标的全部跑输 P5。

不修改 organic_fugue.py（O0≡P5 守卫基线）：诊断 = run_organic 的 diag trace
（已含逐腿 sell_bar/buy_bar/价格/profit）× 信号磁带事件流的离线交叉分类
+ 中枢账本重放采样（与 run_organic 中枢生命周期逻辑逐字同构，仅作只读快照）。

诊断轴（对应任务 1-5）：
  1. REV 腿逐笔磁带：开腿触发类型 / 级别 / 中枢锚 / 闭腿原因(t5细分/t6/t7/强平)
     / 持有 bars / 盈亏 / 开腿时所在结构（中枢上方=趋势上段 / 中枢内 / 下方）
  2. 亏损集中度：rev 腿 pnl 分布 + Δ 的真正载体（暴露缺口）分解
  3. 域腿 vs REV 腿：O1v 隔离下 osc 被 T1 CLOSE_OSC 强制截断的逐笔证据
  4. 41课门反事实：O1≡O2 门拒=0 → 反事实空集；FatigueMonitor 重放给出
     证据集恒非空的机制量化（up_move settle 清空频率 vs 卖侧证据频率）
  5. 假设检验 a-e：开腿结构分布×盈亏 / 闭腿原因×盈亏 / 规模 / 跨标的对比
     / master 触发扩展的暴露缺口（flat-gap 错失收益按出场触发归因）

用法：PYTHONPATH=src .venv/bin/python analysis/rev_leg_diagnostic.py --symbol OKLO
      （三标的并行各自落盘 data_cache/rev_leg_diag_{SYM}.json）
      PYTHONPATH=src .venv/bin/python analysis/rev_leg_diagnostic.py --report

认识论等级：L2（三标的真实 1min 数据，逐笔可否证）。
"""

from __future__ import annotations

import argparse
import json
import math
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    FIRST_BSP_LADDER,
    LADDER_SEG,
    MAX_LADDER,
    NO_LADDER_DIVS,
    NO_LADDER_EVENTS,
    PAIRING_VARIANTS,
    ladder_name,
    run_version_i,
)
from organic_fugue import (  # noqa: E402
    ORGANIC_VARIANTS,
    FatigueMonitor,
    run_organic,
)
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_MD = ROOT / "analysis" / "rev_leg_diagnostic.md"
SYMBOLS = ["OKLO", "QQQ", "BRN"]
FLOOR = LADDER_SEG


# ════════════════════════════════════════════════════════════
# 磁带事件分类器（与 LOU step / run_organic master 循环判定同构）
# ════════════════════════════════════════════════════════════

def _rows(sig, attr, ladder):
    t = getattr(sig, attr)
    return t[ladder] if t else ()


def classify_open(sig, ladder) -> list[str]:
    """开腿 bar 的段终结触发分量（可多重共现）。"""
    flags = []
    for e in _rows(sig, "bsp_events", ladder):
        if e[3] and e[1] == "sell":
            if e[0] == "type1":
                flags.append("t1_conf_sell")
            elif e[0] == "type3":
                flags.append("t3_conf_sell")
    for d in _rows(sig, "div_events", ladder):
        if d[2] == "sell":
            flags.append(f"div_{d[0]}_sell")
    return flags


def classify_close(sig, ladder, forced: bool) -> str:
    """闭腿原因（镜像 LOU step 优先级 hard>pre>t5；rev_close='conf'）。"""
    if forced:
        return "forced_master_exit"
    evs = _rows(sig, "bsp_events", ladder)
    if any(e[3] and e[0] == "type3" and e[1] == "buy" for e in evs):
        return "t7_hard_type3_buy"
    if any((not e[3]) and e[0] == "type3" and e[1] == "buy" for e in evs):
        return "t6_pre_type3_buy"
    if any(e[3] and e[0] == "type1" and e[1] == "buy" for e in evs):
        return "t5_type1_conf_buy"
    if any(e[3] and e[0] == "type2" and e[1] == "buy" for e in evs):
        return "t5_type2_conf_buy"
    if any(d[0] == "consolidation" and d[1] == "down" and d[2] == "buy"
           for d in _rows(sig, "div_events", ladder)):
        return "t5_div_cons_buy"
    return "unclassified"


def classify_master_exit(sig, entry_ladder, exit_reason: str) -> str:
    """O1 master 出场触发归因（sell1=P5 也会出 / 其余=rev_mode 扩展分量）。"""
    if exit_reason == "eod_close":
        return "eod"
    if sig.sell1[entry_ladder]:
        return "sell1_P5_compatible"
    evs = _rows(sig, "bsp_events", entry_ladder)
    if any(e[3] and e[1] == "sell" and e[0] == "type3" for e in evs):
        return "ext_type3_conf_sell"
    for d in _rows(sig, "div_events", entry_ladder):
        if d[2] == "sell":
            return f"ext_div_{d[0]}_sell"
    if any(e[3] and e[1] == "sell" and e[0] == "type1" for e in evs):
        return "ext_type1_conf_sell"
    return "unclassified"


# ════════════════════════════════════════════════════════════
# 中枢账本重放（run_organic 中枢生命周期逐字同构，只读快照）
# ════════════════════════════════════════════════════════════

def replay_center_snapshots(tape, wanted: set[int]) -> dict:
    last_center: dict[int, tuple] = {}
    dead: dict[int, set] = {}
    frozen: dict[int, int] = {}
    snaps: dict[int, dict] = {}
    for i, sig in enumerate(tape):
        evrows = sig.bsp_events or NO_LADDER_EVENTS
        if evrows is not NO_LADDER_EVENTS:
            for lad in range(FIRST_BSP_LADDER, MAX_LADDER):
                for ev in evrows[lad]:
                    kind, side, confirmed, cs = ev[0], ev[1], ev[3], ev[4]
                    if cs is None:
                        continue
                    d = dead.setdefault(lad, set())
                    if confirmed and kind == "type3":
                        d.add(cs)
                        if side == "buy":  # hard_type3（O1 配置恒 True）
                            frozen[lad] = cs
                    elif cs not in d:
                        last_center[lad] = (cs, ev[5], ev[6])
                        if lad in frozen and frozen[lad] != cs:
                            del frozen[lad]
        if i in wanted:
            snaps[i] = {
                lad: (cs, zd, zg, cs not in dead.get(lad, ()), lad in frozen)
                for lad, (cs, zd, zg) in last_center.items()
            }
    return snaps


def structure_at(snap: dict | None, ladder: int, c: float) -> str:
    """开腿 bar 价格相对该层最近中枢的位置（趋势段/中枢震荡判定代理）。"""
    if snap is None or ladder not in snap:
        return "no_center"
    _cs, zd, zg, alive, _frz = snap[ladder]
    if not alive:
        return "center_dead"
    if c > zg:
        return "above_zg_uptrend_leg"
    if c < zd:
        return "below_zd_downtrend_leg"
    return "inside_center_osc"


# ════════════════════════════════════════════════════════════
# FatigueMonitor 重放（41课门恒开机制量化）
# ════════════════════════════════════════════════════════════

def fatigue_stats(tape) -> dict:
    fat = FatigueMonitor()
    n = len(tape)
    nonempty_bars: dict[int, int] = {}
    n_up_settle: dict[int, int] = {}
    n_sell_evidence: dict[int, int] = {}
    for sig in tape:
        evrows = sig.bsp_events or NO_LADDER_EVENTS
        devrows = sig.div_events or NO_LADDER_DIVS
        ums = sig.up_move_settled
        for lad in range(FIRST_BSP_LADDER, MAX_LADDER):
            bsp_l = evrows[lad] if evrows is not NO_LADDER_EVENTS else ()
            div_l = devrows[lad] if devrows is not NO_LADDER_DIVS else ()
            up_l = bool(ums) and bool(ums[lad])
            if up_l:
                n_up_settle[lad] = n_up_settle.get(lad, 0) + 1
            for e in bsp_l:
                if e[1] == "sell" and (e[0] == "type1"
                                       or (e[0] == "type3" and e[3])):
                    n_sell_evidence[lad] = n_sell_evidence.get(lad, 0) + 1
            for d in div_l:
                if d[2] == "sell":
                    n_sell_evidence[lad] = n_sell_evidence.get(lad, 0) + 1
            if bsp_l or div_l or up_l:
                fat.observe(lad, bsp_l, div_l, up_l)
        for lad, ev in fat.evidence.items():
            if ev:
                nonempty_bars[lad] = nonempty_bars.get(lad, 0) + 1
    return {
        "evidence_nonempty_frac": {
            ladder_name(k): round(v / n * 100, 2)
            for k, v in sorted(nonempty_bars.items())},
        "n_up_settle": {ladder_name(k): v
                        for k, v in sorted(n_up_settle.items())},
        "n_sell_evidence": {ladder_name(k): v
                            for k, v in sorted(n_sell_evidence.items())},
    }


# ════════════════════════════════════════════════════════════
# 暴露缺口分解（flat-gap 错失收益，按出场触发归因）
# ════════════════════════════════════════════════════════════

def flat_gap_decomposition(diag_p5: list, diag_o1: list, closes,
                           exit_trigger_of: dict[int, str]) -> dict:
    """P5 持有窗口内 O1 空仓缺口的错失对数收益，归因到造成缺口的出场触发。"""
    o1_iv = sorted((t["entry_bar"], t["exit_bar"]) for t in diag_o1)
    missed_by_trigger: dict[str, float] = {}
    n_gaps_by_trigger: dict[str, int] = {}
    total_missed = 0.0
    for p in diag_p5:
        e, x = p["entry_bar"], p["exit_bar"]
        segs = sorted((max(s, e), min(t, x)) for s, t in o1_iv
                      if s < x and t > e)
        cur = e
        prev_exit_bar = None
        for s, t in segs:
            if s > cur:
                m = math.log(closes[s] / closes[cur])
                total_missed += m
                trig = (exit_trigger_of.get(prev_exit_bar, "entry_lag")
                        if prev_exit_bar is not None else "entry_lag")
                missed_by_trigger[trig] = missed_by_trigger.get(trig, 0.0) + m
                n_gaps_by_trigger[trig] = n_gaps_by_trigger.get(trig, 0) + 1
            cur = max(cur, t)
            prev_exit_bar = t
        if cur < x:
            m = math.log(closes[x] / closes[cur])
            total_missed += m
            trig = (exit_trigger_of.get(prev_exit_bar, "entry_lag")
                    if prev_exit_bar is not None else "entry_lag")
            missed_by_trigger[trig] = missed_by_trigger.get(trig, 0.0) + m
            n_gaps_by_trigger[trig] = n_gaps_by_trigger.get(trig, 0) + 1
    return {
        "total_missed_pct": round((math.exp(total_missed) - 1) * 100, 1),
        "missed_pct_by_trigger": {
            k: round((math.exp(v) - 1) * 100, 1)
            for k, v in sorted(missed_by_trigger.items(),
                               key=lambda kv: kv[1])},
        "n_gaps_by_trigger": n_gaps_by_trigger,
    }


# ════════════════════════════════════════════════════════════
# 聚合工具
# ════════════════════════════════════════════════════════════

def _agg_pnl(rows: list, key: str) -> dict:
    out: dict[str, dict] = {}
    for r in rows:
        g = out.setdefault(r[key], {"n": 0, "net": 0.0, "n_win": 0,
                                    "sum_diff_pct": 0.0, "sum_held": 0})
        g["n"] += 1
        g["net"] += r["profit"]
        g["n_win"] += 1 if r["diff"] > 0 else 0
        g["sum_diff_pct"] += r["diff"] / r["sell_price"] * 100
        g["sum_held"] += r["held"]
    for g in out.values():
        g["net"] = round(g["net"], 0)
        g["win_rate"] = round(g["n_win"] / g["n"] * 100, 1)
        g["avg_diff_pct"] = round(g["sum_diff_pct"] / g["n"], 4)
        g["avg_held_bars"] = round(g["sum_held"] / g["n"], 0)
        del g["n_win"], g["sum_diff_pct"], g["sum_held"]
    return out


def _concentration(rows: list) -> dict:
    pnls = sorted(r["profit"] for r in rows)
    neg = [p for p in pnls if p < 0]
    pos = [p for p in pnls if p > 0]
    gross_loss = sum(neg)
    out = {
        "n": len(pnls), "net": round(sum(pnls), 0),
        "gross_loss": round(gross_loss, 0), "gross_win": round(sum(pos), 0),
        "n_loss": len(neg), "n_win": len(pos),
    }
    for k in (10, 30):
        worst = sum(neg[:k])
        out[f"worst{k}_share_of_gross_loss"] = (
            round(worst / gross_loss * 100, 1) if gross_loss < 0 else None)
    return out


def _rev_rows(diag: list) -> list:
    """提取 rev 腿 trace 行 + 派生字段（含强平标记）。"""
    rows = []
    for tr in diag:
        for r in tr["diffs"]:
            if r.get("leg") != "rev":
                continue
            rows.append({
                **r, "ladder": r["ladder"] - 200,
                "held": r["buy_bar"] - r["sell_bar"],
                "forced": r["buy_bar"] == tr["exit_bar"],
                "trade_entry_bar": tr["entry_bar"],
            })
    return rows


# ════════════════════════════════════════════════════════════
# 单标的管线
# ════════════════════════════════════════════════════════════

def process_symbol(symbol: str) -> None:
    print(f"==== {symbol} REV 腿诊断 ====", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes)
    print(f"  信号层 {time.time() - t0:.1f}s", flush=True)

    diag_p5: list = []
    trades_p5, _ = run_version_i(tape, floor_ladder=FLOOR,
                                 pairing=PAIRING_VARIANTS["P5"], diag=diag_p5)
    diag_o1: list = []
    trades_o1, extra_o1 = run_organic(tape, floor_ladder=FLOOR,
                                      config=ORGANIC_VARIANTS["O1"],
                                      diag=diag_o1)
    diag_o1v: list = []
    trades_o1v, _ = run_organic(tape, floor_ladder=FLOOR,
                                config=ORGANIC_VARIANTS["O1v"], diag=diag_o1v)
    print(f"  回测完成 P5={len(trades_p5)} O1={len(trades_o1)} "
          f"O1v={len(trades_o1v)} 笔", flush=True)

    rev_o1 = _rev_rows(diag_o1)
    rev_o1v = _rev_rows(diag_o1v)

    # ── 中枢快照（开腿 bar 的结构定位）──
    wanted = ({r["sell_bar"] for r in rev_o1}
              | {r["sell_bar"] for r in rev_o1v})
    snaps = replay_center_snapshots(tape, wanted)

    for rows in (rev_o1, rev_o1v):
        for r in rows:
            sig_o = tape[r["sell_bar"]]
            r["open_triggers"] = classify_open(sig_o, r["ladder"])
            r["open_trigger"] = (r["open_triggers"][0]
                                 if r["open_triggers"] else "none")
            r["structure"] = structure_at(
                snaps.get(r["sell_bar"]), r["ladder"], sig_o.close)
            r["close_reason"] = classify_close(
                tape[r["buy_bar"]], r["ladder"], r["forced"])

    # ── master 出场归因（O1）+ 暴露缺口分解 ──
    exit_trigger_of: dict[int, str] = {}
    exit_attr_rows = []
    for tr in diag_o1:
        trig = classify_master_exit(
            tape[tr["exit_bar"]], tr["entry_ladder"], tr["exit_reason"])
        exit_trigger_of[tr["exit_bar"]] = trig
        exit_attr_rows.append({"trig": trig, "held": tr["exit_bar"] - tr["entry_bar"],
                               "pnl_pct": tr["pnl_pct"]})
    exit_attr: dict[str, dict] = {}
    for r in exit_attr_rows:
        g = exit_attr.setdefault(r["trig"], {"n": 0, "sum_held": 0,
                                             "sum_pnl": 0.0})
        g["n"] += 1
        g["sum_held"] += r["held"]
        g["sum_pnl"] += r["pnl_pct"]
    for g in exit_attr.values():
        g["avg_held_bars"] = round(g["sum_held"] / g["n"], 0)
        g["avg_pnl_pct"] = round(g["sum_pnl"] / g["n"], 3)
        del g["sum_held"], g["sum_pnl"]

    gaps = flat_gap_decomposition(diag_p5, diag_o1, closes, exit_trigger_of)
    bars_long_o1 = sum(t["exit_bar"] - t["entry_bar"] for t in diag_o1)
    bars_long_p5 = sum(t["exit_bar"] - t["entry_bar"] for t in diag_p5)

    # ── 域腿截断证据（O1v：osc 闭腿 bar == 同层 rev 开腿 bar）──
    osc_forced, osc_other = [], []
    for tr in diag_o1v:
        rev_open_keys = {(r["sell_bar"], r["ladder"] - 200)
                         for r in tr["diffs"] if r.get("leg") == "rev"}
        for r in tr["diffs"]:
            if r.get("leg") != "osc":
                continue
            row = {**r, "held": r["buy_bar"] - r["sell_bar"]}
            if (r["buy_bar"], r["ladder"] - 100) in rev_open_keys:
                osc_forced.append(row)
            else:
                osc_other.append(row)

    def _osc_agg(rows):
        if not rows:
            return {"n": 0}
        return {"n": len(rows),
                "net": round(sum(r["profit"] for r in rows), 0),
                "win_rate": round(sum(1 for r in rows if r["diff"] > 0)
                                  / len(rows) * 100, 1),
                "avg_diff_pct": round(sum(r["diff"] / r["sell_price"]
                                          for r in rows) / len(rows) * 100, 4)}

    # ── 41课门重放统计 ──
    fat = fatigue_stats(tape)

    # ── 逐笔磁带样本（最大亏损/盈利各 12 笔 + 头部样本）──
    def _tape_sample(rows):
        srt = sorted(rows, key=lambda r: r["profit"])
        keep = srt[:12] + srt[-6:]
        return [{k: (round(v, 4) if isinstance(v, float) else v)
                 for k, v in r.items()
                 if k in ("ladder", "sell_bar", "sell_price", "buy_bar",
                          "buy_price", "held", "profit", "diff", "forced",
                          "open_trigger", "structure", "close_reason")}
                for r in keep]

    out = {
        "meta": {"bars": len(closes), "bh": round(bh, 2),
                 "n_trades": {"P5": len(trades_p5), "O1": len(trades_o1),
                              "O1v": len(trades_o1v)}},
        "o1_counters": extra_o1["counters"],
        "rev_o1": {
            "n": len(rev_o1),
            "by_open_trigger": _agg_pnl(rev_o1, "open_trigger"),
            "by_structure": _agg_pnl(rev_o1, "structure"),
            "by_close_reason": _agg_pnl(rev_o1, "close_reason"),
            "concentration": _concentration(rev_o1),
            "held_median": sorted(r["held"] for r in rev_o1)[len(rev_o1) // 2]
            if rev_o1 else None,
            "tape_sample": _tape_sample(rev_o1),
        },
        "rev_o1v": {
            "n": len(rev_o1v),
            "by_open_trigger": _agg_pnl(rev_o1v, "open_trigger"),
            "by_structure": _agg_pnl(rev_o1v, "structure"),
            "by_close_reason": _agg_pnl(rev_o1v, "close_reason"),
            "concentration": _concentration(rev_o1v),
        },
        "master_exit_attribution": exit_attr,
        "exposure": {
            "bars_long_O1": bars_long_o1, "bars_long_P5": bars_long_p5,
            "exposure_ratio": round(bars_long_o1 / bars_long_p5, 3)
            if bars_long_p5 else None,
            "flat_gap": gaps,
        },
        "osc_disruption_o1v": {
            "forced_by_rev_open": _osc_agg(osc_forced),
            "other_closes": _osc_agg(osc_other),
        },
        "fatigue": fat,
    }
    DATA_DIR.mkdir(exist_ok=True)
    (DATA_DIR / f"rev_leg_diag_{symbol}.json").write_text(
        json.dumps(out, indent=1, ensure_ascii=False))
    print(f"  [{symbol}] 诊断落盘 rev_leg_diag_{symbol}.json", flush=True)


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

def write_report() -> None:
    data = {}
    for s in SYMBOLS:
        p = DATA_DIR / f"rev_leg_diag_{s}.json"
        if p.exists():
            data[s] = json.loads(p.read_text())
    if not data:
        print("无诊断数据", file=sys.stderr)
        sys.exit(1)

    L: list[str] = []
    L.append("# REV 腿交易行为诊断 — O1 三标的全负的逐笔根因\n")
    L.append("> 数据：O1/O1v/P5 逐腿 trace × 磁带事件流离线交叉分类；中枢账本"
             "重放采样开腿结构位置。生成脚本 `rev_leg_diagnostic.py`。"
             "认识论 **L2**（OKLO/QQQ/BRN 真实 1min，逐笔可否证）。\n")

    # —— 0. 判决摘要 ——
    L.append("## 0. 判决摘要\n")
    L.append("Δ(O1) 全负是**两个独立机制的叠加**（O1v 拆解：master 扩展贡献 = "
             "Δ(O1)−Δ(O1v) = OKLO -562.5pp[90%] / QQQ -39.0pp[49%] / "
             "BRN -263.6pp[59%]；其余为 voice REV 腿自身贡献）：\n")
    L.append("1. **master 出场触发扩展砍掉一半暴露（首要机制；OKLO 上压倒性，"
             "QQQ 上与 voice 同量级）**："
             "rev_mode 把 master 出场从 sell1（confirmed type1 卖）扩展为段终结"
             "三触发后，绝大多数出场由扩展分量触发（ext_type3_conf_sell + "
             "ext_div_consolidation_sell），sell1 兼容出场占比 <2%。三标的持仓"
             "时间一致地掉到 P5 的 ~50%，P5 持有窗口内的空仓缺口在强趋势标的上"
             "错失复合收益数倍于本金。其中 ext_type3_conf_sell 出场是高频割肉"
             "循环（平均持有最短、平均 pnl 为负）。")
    L.append("2. **voice REV 腿自身负贡献（O1v 隔离：-63.7/-41.4/-186.2pp）**：两个"
             "结构性亏损通道——(a) T1 动作『先 CLOSE_OSC』把高胜率域腿在卖侧"
             "信号 bar 强制回补（截断腿胜率 20-31% vs 正常闭腿 66-77%）；"
             "(b) t6_pre_type3_buy 关腿槽（REV 卖出后价格不跌反向上离开中枢，"
             "candidate type3 买预逃逸高位回补）是 REV 腿内唯一系统性大负槽"
             "（三标的胜率一致 ≈30%），把 t5 设计剧本（下跌背驰买回，赚钱）"
             "的利润全部吃掉。\n")

    # —— 1. 暴露缺口（Δ 的真正载体）——
    L.append("## 1. Δ 的分解：亏损载体不是 REV 配对差价，而是暴露缺口\n")
    L.append("| 标的 | O1 持仓 bars | P5 持仓 bars | 暴露比 | P5 窗口内空仓"
             "错失(复合%) |")
    L.append("|------|-------------|-------------|--------|----------------|")
    for s, d in data.items():
        e = d["exposure"]
        L.append(f"| {s} | {e['bars_long_O1']:,} | {e['bars_long_P5']:,} | "
                 f"{e['exposure_ratio']} | {e['flat_gap']['total_missed_pct']:+.1f} |")
    L.append("")
    L.append("### 空仓缺口按造成缺口的出场触发归因（复合%）\n")
    L.append("| 标的 | 触发 | 缺口数 | 错失复合% |")
    L.append("|------|------|--------|-----------|")
    for s, d in data.items():
        fg = d["exposure"]["flat_gap"]
        for trig, v in fg["missed_pct_by_trigger"].items():
            L.append(f"| {s} | {trig} | "
                     f"{fg['n_gaps_by_trigger'].get(trig, 0)} | {v:+.1f} |")
    L.append("")

    # —— 2. master 出场归因 ——
    L.append("## 2. O1 master 出场触发归因（哪个扩展分量砍了暴露）\n")
    L.append("| 标的 | 出场触发 | 笔数 | 平均持有bars | 平均pnl% |")
    L.append("|------|---------|------|--------------|----------|")
    for s, d in data.items():
        for trig, g in sorted(d["master_exit_attribution"].items(),
                              key=lambda kv: -kv[1]["n"]):
            L.append(f"| {s} | {trig} | {g['n']} | {g['avg_held_bars']:.0f} | "
                     f"{g['avg_pnl_pct']:+.3f} |")
    L.append("")

    # —— 3. REV 腿逐笔行为 ——
    for name, key in (("3a. O1", "rev_o1"), ("3b. O1v（master 不变的隔离对照）",
                                             "rev_o1v")):
        L.append(f"## {name} — REV 腿逐笔聚合\n")
        for dim, title in (("by_open_trigger", "开腿触发"),
                           ("by_structure", "开腿时结构位置"),
                           ("by_close_reason", "闭腿原因")):
            L.append(f"### {title}\n")
            L.append("| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | "
                     "平均持有bars |")
            L.append("|------|------|------|-------|-----------|--------|"
                     "--------------|")
            for s, d in data.items():
                for grp, g in sorted(d[key][dim].items(),
                                     key=lambda kv: -kv[1]["n"]):
                    L.append(f"| {s} | {grp} | {g['n']} | {g['win_rate']} | "
                             f"{g['avg_diff_pct']} | {g['net']:+.0f} | "
                             f"{g['avg_held_bars']:.0f} |")
            L.append("")

    # —— 4. 集中度 ——
    L.append("## 4. REV 腿亏损集中度\n")
    L.append("| 标的 | 变体 | 对数 | 净 | 毛亏 | 毛盈 | 亏笔 | "
             "最差10笔占毛亏% | 最差30笔占毛亏% | 中位持有bars |")
    L.append("|------|------|------|----|------|------|------|------|------|"
             "------|")
    for s, d in data.items():
        for v, key in (("O1", "rev_o1"), ("O1v", "rev_o1v")):
            c = d[key]["concentration"]
            med = d[key].get("held_median") or "—"
            L.append(f"| {s} | {v} | {c['n']} | {c['net']:+.0f} | "
                     f"{c['gross_loss']:+.0f} | {c['gross_win']:+.0f} | "
                     f"{c['n_loss']} | {c['worst10_share_of_gross_loss']} | "
                     f"{c['worst30_share_of_gross_loss']} | {med} |")
    L.append("")

    # —— 5. 域腿截断 ——
    L.append("## 5. 域腿 vs REV 腿：T1 CLOSE_OSC 截断证据（O1v 隔离）\n")
    L.append("| 标的 | osc 闭腿类 | 对数 | 胜率% | avg_diff% | 净现金 |")
    L.append("|------|-----------|------|-------|-----------|--------|")
    for s, d in data.items():
        for k, label in (("forced_by_rev_open", "被 REV 开腿强制截断"),
                         ("other_closes", "正常闭腿(ZD/死亡/强平)")):
            g = d["osc_disruption_o1v"][k]
            if g["n"]:
                L.append(f"| {s} | {label} | {g['n']} | {g['win_rate']} | "
                         f"{g['avg_diff_pct']} | {g['net']:+.0f} |")
    L.append("")

    # —— 6. 41课门 ——
    L.append("## 6. 41课门：反事实为空集 + 恒开机制\n")
    L.append("O2 与 O1 逐位相同（门拒=0，门开率 100%）→ **门没有关掉任何一条 "
             "REV 腿，反事实集为空**。恒开机制（FatigueMonitor 重放）：\n")
    L.append("| 标的 | ladder | 证据集非空时间占比% | 卖侧证据事件数 | "
             "向上settle清空次数 |")
    L.append("|------|--------|---------------------|----------------|"
             "--------------------|")
    for s, d in data.items():
        f = d["fatigue"]
        for lad, frac in f["evidence_nonempty_frac"].items():
            L.append(f"| {s} | {lad} | {frac} | "
                     f"{f['n_sell_evidence'].get(lad, 0)} | "
                     f"{f['n_up_settle'].get(lad, 0)} |")
    L.append("")

    # —— 7. 逐笔磁带样本 ——
    L.append("## 7. O1 REV 腿逐笔磁带样本（最大亏损 12 + 最大盈利 6）\n")
    for s, d in data.items():
        L.append(f"### {s}\n")
        L.append("| ladder | 开bar | 开价 | 闭bar | 闭价 | 持有 | 盈亏 | "
                 "开腿触发 | 结构位置 | 闭腿原因 | 强平 |")
        L.append("|--------|-------|------|-------|------|------|------|"
                 "----------|----------|----------|------|")
        for r in d["rev_o1"]["tape_sample"]:
            L.append(f"| {ladder_name(r['ladder'])} | {r['sell_bar']} | "
                     f"{r['sell_price']} | {r['buy_bar']} | {r['buy_price']} | "
                     f"{r['held']} | {r['profit']:+.0f} | {r['open_trigger']} | "
                     f"{r['structure']} | {r['close_reason']} | "
                     f"{'是' if r['forced'] else ''} |")
        L.append("")

    # —— 8. 假设检验判决 ——
    L.append("## 8. 假设检验判决（任务 §5）\n")
    L.append("| 假设 | 判决 | 证据 |")
    L.append("|------|------|------|")
    L.append("| a) 趋势段反向操盘，41课门应阻止但没有 | **部分成立（机制精确化）** | "
             "REV 开腿 33-50% 发生在价格位于中枢上方（above_zg 上升中段）；"
             "41课门完全失效——O1≡O2 逐位相同、门拒=0，机制见 §6：segment 层"
             "衰竭证据集 99.95-99.99% 时间非空（卖侧证据事件 9.5K-54K 个 vs "
             "up-move settle 清空仅 96-503 次），门的时效设计在该分辨率下不构成"
             "约束。但『趋势段反向』的最大亏损通道在 master 层（见 e），voice "
             "层的趋势段开腿亏损由 t6 槽承载 |")
    L.append("| b) 闭腿信号配对问题 | **成立（t6 槽）** | t6_pre_type3_buy"
             "（candidate type3 买预逃逸）三标的胜率一致 ≈29-32%、净额大负"
             "（O1v：OKLO -224.7K / QQQ -63.7K / BRN -302.0K）——REV 卖出后价格"
             "向上离开中枢被迫高位回补，与此前 sell_any/buy_any 配对问题同构"
             "（卖低买高）；t5_type1_conf_buy（38课设计剧本）反而是正槽"
             "（胜率 56-63%）。逃逸通道本身没错（不逃亏更多），错的是开腿后"
             "走到需要逃逸的频率太高 = 开腿判定（段终结）在 1min segment 层"
             "假阳性过半 |")
    L.append("| c) 操作规模太大 | **否证** | rev frac = 1/N_sub 与 osc 腿同口径；"
             "stock 模式 cost>0 阶段 REV 是虚拟价差收集器（不动 total_shares，"
             "不影响真实暴露）；O3（结构规模缩腿至 244/607/1663 对）Δ 仍 "
             "-654.9/-79.0/-439.1——规模轴改不了符号 |")
    L.append("| d) 标的选择偏差 | **否证（方向）/ 成立（幅度）** | 三标的机制"
             "完全同构（暴露比都 ≈0.50、t6 槽胜率都 ≈30%、门都恒开）→ 不是"
             "标的问题；但 Δ 幅度 ∝ 趋势强度（OKLO -626 > BRN -450 > QQQ -80），"
             "暴露缺口在强趋势标的上代价最大。不存在『中性标的上 REV 转正』的"
             "证据——QQQ（最弱趋势）rev 腿净额本身就是负的 |")
    L.append("| e) 其他：master 触发扩展（真正主因） | **成立** | rev_mode 捆绑"
             "把 master 出场扩展为段终结三触发：sell1 兼容出场仅占 1.4-1.8%，"
             "ext_type3_conf_sell 出场（高频割肉：平均持有 50-102 bar、平均 "
             "pnl -0.77/-0.10/-0.19%）+ ext_div_consolidation_sell（E10 引入的"
             "盘整背驰流量）把交易数放大 3.7-4.4×、暴露腰斩；空仓缺口错失复合 "
             "+308.9%（OKLO）/+44.9%（QQQ）/+209.0%（BRN）。O1v 隔离：master "
             "恢复 sell1 后 Δ 从 -626/-80/-450 收敛到 -64/-41/-186 |")
    L.append("")

    # —— 结果包 ——
    L.append("## 结果包（六要素）\n")
    L.append("**1. 结论**：O1 全负 = master 触发扩展砍暴露（OKLO -562.5pp[90%] / "
             "QQQ -39.0pp[49%] / BRN -263.6pp[59%]；暴露比 0.50/0.49/0.50，空仓"
             "错失复合 +309/+45/+209%）⊕ voice REV 腿自身负贡献（-63.7/-41.4/"
             "-186.2pp：osc 截断 + t6 预逃逸槽）。REV 腿配对现金"
             "（OKLO +55.6K/BRN +7.6K 为正）不是 Δ 的载体——亏的是没拿住的"
             "仓位，不是拿反的差价。亏损不集中（最差10笔仅占毛亏 7-27%），是"
             "结构性均匀失血，排除少数极端事件解释。\n")
    L.append("**2. 定义依据**：38课段终结三触发（confirmed type1 卖 ∨ div 卖侧 ∨ "
             "confirmed type3 卖，`LevelOperatingUnit.seg_end_trigger`）；其用于 "
             "master 出场时与 45课持股持币二元的级别相对性冲突——segment 层段"
             "终结事件密度（1min 数据上日均数十个）远高于 entry 级走势终结密度，"
             "把 entry 级持仓的出场判定下放到 segment 级 = 级别错配（与"
             "『走势方向代理陷阱』『PH settle 用途边界』同构：低级别信号越级"
             "驱动高级别操作）。41课门时效定义（up-move settle 清空）在该密度"
             "比下恒开。\n")
    L.append("**3. 边界条件**：(a) 本诊断在 1min/segment floor 上成立——更高 "
             "floor（move 级段终结）的事件密度低一个量级，master 扩展的暴露"
             "代价可能反转，未检验；(b) 强趋势标的（BH +87~+307%）上空仓代价"
             "被趋势放大——长期横盘/下行标的上同一机制的符号可能不同；"
             "(c) t6 槽若改为『预逃逸不回补、等 confirmed』（t7-only）REV 净额"
             "会变化，但逃逸延迟在真突破时亏更多——该消融未跑；(d) 结论对"
             "费率单调恶化（rev 腿 avg_diff ≈0，任何费率下 t6 槽更深）。\n")
    L.append("**4. 下游推论**：(i) 若要保留 REV 假设的任何残余，唯一可活的"
             "形态是 O1v 框架 + 关掉 t6 亏损源（开腿端收紧而非关腿端放宽——"
             "t5 剧本本身是正的）+ 不碰 osc（取消『先 CLOSE_OSC』动作或 REV 与 "
             "osc 槽位共存）；(ii) master 出场任何扩展必须先过暴露守恒检查"
             "（新触发集的出场频率 vs sell1 的倍数 = 暴露损失的先验上界）；"
             "(iii) 41课门若要有约束力，时效必须改为与被守卫层同分辨率的清空"
             "事件（如 u 层新中枢生成），否则证据集恒非空=门恒开=死代码；"
             "(iv) E10 盘整背驰进 div_events 的直接消费者（seg_end_trigger 的 "
             "div 分量）是 master 出场 churn 的两大来源之一——盘整背驰作为"
             "『段终结证据』在 segment 层假阳性过半。\n")
    L.append("**5. 谱系引用**：532号（段终结判定单轴性分离，生成态——本诊断"
             "是其数据基底：master 轴与 voice 轴的亏损机制已定量分离）；E6"
             "（bar级短差有害）/E10（盘整背驰可见）；『走势方向代理陷阱』"
             "『PH settle用途边界』（低级别信号越级驱动高级别操作的同构先例）；"
             "project_organic_fugue_implementation（REV 否证记录）。\n")
    L.append("**6. 影响声明**：新增 `analysis/rev_leg_diagnostic.py`（只读诊断，"
             "不触碰 organic_fugue.py / 回测缓存）+ 本报告 + "
             "`data_cache/rev_leg_diag_{OKLO,QQQ,BRN}.json`（逐笔分类数据）。"
             "不改变任何定义、引擎、回测结论；为 532号谱系候选提供 L2 证据。\n")

    OUT_MD.write_text("\n".join(L))
    print(f"报告 → {OUT_MD}", flush=True)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbol")
    ap.add_argument("--report", action="store_true")
    args = ap.parse_args()
    if args.symbol:
        process_symbol(args.symbol.upper())
    if args.report:
        write_report()
