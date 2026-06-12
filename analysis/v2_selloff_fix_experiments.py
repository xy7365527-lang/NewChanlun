"""v2 卖飞修复实验 — 短差层政策反事实矩阵（OKLO + BRN，I_seg2 口径）。

═══════════════════════════════════════════════════════════════════════
问题（用户）：卖飞怎么解决？
═══════════════════════════════════════════════════════════════════════
根因诊断（v2_selloff_root_cause）：卖出信号不滞后（卖飞笔 gap_sell 0.99% < 成功笔
1.31%）；卖飞 = 次级别局部上冲（run_up 2.24%）> 买回信号可分辨粒度（买回滞后局部底
2.33%，而卖飞笔回调深度仅 0.68%）。→ 修复轴在**买回侧机制**与**卖点质量门控**，
不在卖出时机。

═══════════════════════════════════════════════════════════════════════
判定标准（可证伪，严格）
═══════════════════════════════════════════════════════════════════════
floor 只影响短差层，不影响进出场（active_levels = range(floor, entry_ladder)，
entry/exit FSM 与 floor 无关）→ 同一信号磁带上：

    短差层净贡献 = compound(有短差政策) − compound(B0 无短差)

**"解决卖飞" = 存在政策使净贡献 > 0**（OKLO B0 ≈ +290.3% = I_move3 实测）。
打败 I_seg2 baseline(+224%) 但输给 B0 = 只是少亏，严格结论仍是删除该层。

═══════════════════════════════════════════════════════════════════════
政策矩阵（修复轴）
═══════════════════════════════════════════════════════════════════════
开仓门控 open_gate：
  - any = sell_any（现行：任意类型 confirmed 卖点）
  - t1  = sell1（缠论正典：type1 顶背驰卖点才高抛，第33课"向上离开中枢出现顶背驰"）
  - none = 不开短差（B0 基线锚）

平仓机制 close_mode：
  - signal          = buy_any 确认买点平（现行 baseline）
  - bracket(δ,ε)    = 限价回补 sell×(1−δ) / 卖飞止损 sell×(1+ε)，价格触达即成交，
                      无需信号确认（绕过买回确认粒度——根因的直接回应）
  - sig_stop(ε)     = buy_any 平 + 卖飞止损 ε（**不冻结级别**，区别于 stop_mode B
                      的冻结——冻结杀死后续成功短差，是 B 失败的结构原因之一）
  - sig_profit_stop(ε) = 只接受盈利的信号平（c < sell），亏损方向由止损 ε 接管；
                      ε=inf = 永不止损、只赚不平（亏损扛到 trade 出场）

成交模型（保守约定，诚实声明）：
  - 限价回补：open ≤ limit → 按 open 成交（更优）；low ≤ limit → 按 limit。
  - 止损回补：open ≥ stop → 按 open 成交（更差）；high ≥ stop → 按 stop。
  - 同 bar 双触（high≥stop 且 low≤limit）→ **取止损**（最坏情形）。
  - 开仓沿用 baseline：信号 bar 收盘价。

═══════════════════════════════════════════════════════════════════════
等价性守卫（差分测试）
═══════════════════════════════════════════════════════════════════════
B1_baseline（any + signal）必须与 fugue_version_i.run_version_i(floor=2) 的
trades 列表逐笔相等（pnl_pct 序列 + compound），否则 abort——保证政策 runner
的 FSM/账本与生产逐字等价，变体差异只来自政策本身。

认识论 L2（真实数据 447K OKLO + 2.4M BRN，含否定性结果可能：全矩阵无人打败 B0
→ 卖飞在该粒度不可解，严格解=放弃该层短差）。

输出：analysis/v2_selloff_fix_experiments.md + data_cache/v2_selloff_fix_experiments.json
"""

from __future__ import annotations

import json
import math
import os
import sys
import time
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as _ef  # noqa: E402
from fugue_alpha_diagnosis import INITIAL_CAPITAL, CompletedTrade  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    FIRST_BSP_LADDER,
    LADDER_BAR,
    LADDER_MOVE,
    LADDER_SEG,
    MAX_LADDER,
    _SharedFugue,
    extended_metrics,
    ladder_name,
    run_version_i,
)
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "v2_selloff_fix_experiments.json"
OUT_MD = ROOT / "analysis" / "v2_selloff_fix_experiments.md"

SYMBOLS = [s.strip().upper() for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
FLOOR = LADDER_SEG  # I_seg2 口径
INF = math.inf

_FLAT, _ARMED, _LONG = 0, 1, 2


SLIP = 0.0005  # 穿透/滑点 5 bps（fill_mode="pen"）


@dataclass(frozen=True)
class DiffPolicy:
    name: str
    open_gate: str       # "any" | "t1" | "none"
    close_mode: str      # "signal" | "bracket" | "sig_stop" | "sig_profit_stop"
    limit_pct: float = 0.0   # δ：限价回补深度 %
    stop_pct: float = 0.0    # ε：卖飞止损幅度 %（INF=无止损）
    fill_mode: str = "touch"
    # touch = 触碰即按限价/止损价成交（标准回测约定，乐观侧）
    # pen   = 穿透成交：限价需 low ≤ limit×(1−5bps) 才算成交（队列清空证据）；
    #         止损市价按触发价×(1+5bps) 滑点成交（悲观侧）
    # next  = 触发只作信号，市价单在**次 bar 开盘**成交（最保守，
    #         消灭全部 intrabar 微观结构假设）


def build_policies() -> list[DiffPolicy]:
    ps = [
        DiffPolicy("B0_nodiff", "none", "signal"),
        DiffPolicy("B1_baseline", "any", "signal"),
        DiffPolicy("t1_signal", "t1", "signal"),
    ]
    for g in ("any", "t1"):
        for d in (0.5, 1.0, 2.0):
            for e in (0.5, 1.0, 2.0, INF):
                ps.append(DiffPolicy(f"{g}_brk_d{d}_e{'inf' if e == INF else e}",
                                     g, "bracket", d, e))
        for e in (0.5, 1.0, 2.0):
            ps.append(DiffPolicy(f"{g}_sigstop_e{e}", g, "sig_stop", 0.0, e))
        for e in (0.5, 1.0, 2.0, INF):
            ps.append(DiffPolicy(f"{g}_sigprofit_e{'inf' if e == INF else e}",
                                 g, "sig_profit_stop", 0.0, e))
    return ps


def _close_fill(policy: DiffPolicy, sell_price: float, o: float, hi: float,
                lo: float, buy_sig: bool, c: float) -> float | None:
    """本 bar 是否平掉该短差；返回成交价或 None（fill_mode touch/pen）。

    成交约定（保守）：止损 gap 开盘按 open（更差）；限价 gap 开盘按 open（更优）；
    同 bar 双触取止损（最坏情形）。pen 模式：限价需穿透 5bps 才成交，止损加 5bps 滑点。
    """
    pen = policy.fill_mode == "pen"
    stop = (sell_price * (1.0 + policy.stop_pct / 100)
            if policy.stop_pct != INF and policy.stop_pct > 0 else None)

    def stop_fill(trigger_price: float) -> float:
        return trigger_price * (1.0 + SLIP) if pen else trigger_price

    if policy.close_mode == "signal":
        return c if buy_sig else None
    if policy.close_mode == "bracket":
        limit = sell_price * (1.0 - policy.limit_pct / 100)
        limit_trig = limit * (1.0 - SLIP) if pen else limit  # pen：须穿透 5bps
        if stop is not None and o >= stop:
            return stop_fill(o)
        if o <= limit_trig:
            return o          # gap 开盘低于限价 → 按 open 成交（限价单不会更差）
        hit_stop = stop is not None and hi >= stop
        hit_limit = lo <= limit_trig
        if hit_stop:           # 双触也取止损（最坏情形）
            return stop_fill(stop)
        if hit_limit:
            return limit
        return None
    # sig_stop / sig_profit_stop：止损优先（同 bar 信号+止损 → 取止损，最坏情形）
    if stop is not None:
        if o >= stop:
            return stop_fill(o)
        if hi >= stop:
            return stop_fill(stop)
    if policy.close_mode == "sig_stop":
        return c if buy_sig else None
    # sig_profit_stop：只接受盈利的信号平
    return c if (buy_sig and c < sell_price) else None


def _touched(policy: DiffPolicy, sell_price: float, o: float, hi: float,
             lo: float) -> bool:
    """fill_mode="next" 的触发检测：限价/止损任一价位本 bar 被触及（不成交，
    只发市价单信号——次 bar 开盘成交）。"""
    stop = (sell_price * (1.0 + policy.stop_pct / 100)
            if policy.stop_pct != INF and policy.stop_pct > 0 else None)
    limit = sell_price * (1.0 - policy.limit_pct / 100)
    if stop is not None and (o >= stop or hi >= stop):
        return True
    return o <= limit or lo <= limit


def run_policy(i_signals, opens, highs, lows, policy: DiffPolicy,
               floor_ladder: int = FLOOR):
    """政策化 Version I runner——FSM/账本与 run_version_i 逐字等价，仅短差
    open/close 触发由 policy 决定。stop_mode='none' 语义（无主仓止损）。"""
    n = len(i_signals)
    state = _FLAT
    entry_bar = -1; entry_price = 0.0; entry_ladder = -1; arm_bar = -1
    arm_ladder = LADDER_MOVE
    pos: _SharedFugue | None = None
    active_levels: list[int] = []
    SUB_EXPIRY = _ef.SUB_EXPIRY
    trades: list[CompletedTrade] = []
    all_cycles: list = []   # (ladder, ShortDiffCycle) 全部已闭合短差（跨交易）
    pending: set[int] = set()  # fill_mode="next"：上 bar 触发、待本 bar 开盘成交的 ladder
    is_next = policy.fill_mode == "next" and policy.close_mode == "bracket"

    def _open(bar_idx: int, price: float, el: int) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, pos, active_levels
        entry_bar = bar_idx; entry_price = price; entry_ladder = el
        active_levels = [] if policy.open_gate == "none" \
            else list(range(floor_ladder, el))
        n_sub = len(active_levels)
        level_frac = (1.0 / n_sub) if n_sub > 0 else 0.0
        pos = _SharedFugue(entry_price=price, total_shares=INITIAL_CAPITAL / price,
                           cost_basis=price, level_frac=level_frac)
        state = _LONG

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, pos, active_levels
        if pos is None or entry_price <= 0 or pos.total_shares <= 0:
            state = _FLAT; pos = None; active_levels = []; return
        for ladder in list(pos.active.keys()):
            pos.close_diff(ladder, price, bar_idx)
        total_value = pos.total_shares * price + pos.cumulative_recovered
        pnl_pct = (total_value - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price, exit_bar=bar_idx,
            exit_price=price, pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=len(pos.completed),
            cost_basis_at_exit=round(pos.cost_basis, 6)))
        all_cycles.extend(pos.completed)
        state = _FLAT; entry_price = 0.0; entry_ladder = -1; pos = None
        active_levels = []
        pending.clear()

    for i in range(n):
        sig = i_signals[i]
        c = sig.close

        if state == _FLAT:
            hi_l = -1
            for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
                if sig.buy1[k]:
                    hi_l = k
            if hi_l >= FIRST_BSP_LADDER:
                state = _ARMED; arm_bar = i; arm_ladder = hi_l

        elif state == _ARMED:
            for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
                if sig.buy1[k] and k > arm_ladder:
                    arm_ladder = k
            do_enter = any(sig.buy_any[k] for k in range(LADDER_BAR, arm_ladder))
            if not do_enter and (i - arm_bar) > SUB_EXPIRY:
                do_enter = True
            if do_enter:
                _open(i, c, arm_ladder)
            elif sig.sell1[arm_ladder]:
                state = _FLAT

        elif state == _LONG:
            if sig.sell1[entry_ladder]:
                _close(i, c, f"exit_{ladder_name(entry_ladder)}_type1sell")
            else:
                # next 模式：上 bar 触发的市价回补单在本 bar 开盘成交
                closed_now: set[int] = set()
                if is_next and pending:
                    for ladder in list(pending):
                        if pos.active.get(ladder) is not None:
                            pos.close_diff(ladder, opens[i], i)
                            closed_now.add(ladder)
                    pending.clear()
                for ladder in active_levels:
                    rec = pos.active.get(ladder)
                    if rec is not None:
                        if is_next:
                            if _touched(policy, rec[0].sell_price, opens[i],
                                        highs[i], lows[i]):
                                pending.add(ladder)
                        else:
                            fill = _close_fill(policy, rec[0].sell_price, opens[i],
                                               highs[i], lows[i],
                                               sig.buy_any[ladder], c)
                            if fill is not None:
                                pos.close_diff(ladder, fill, i)
                        # baseline elif 语义：平仓 bar 不再开仓
                    elif ladder not in closed_now:
                        open_sig = (sig.sell1[ladder] if policy.open_gate == "t1"
                                    else sig.sell_any[ladder])
                        if open_sig:
                            pos.open_diff(ladder, pos.level_frac, c, i)

    if state == _LONG and pos is not None and pos.total_shares > 0:
        _close(n - 1, i_signals[-1].close, "eod_close")

    n_diffs = len(all_cycles)
    n_fly = sum(1 for _, cyc in all_cycles if cyc.profit < 0)
    diff_net = sum(cyc.profit for _, cyc in all_cycles)
    # 盈亏平衡费率（bps/腿）：短差净现金 ÷ (短差数 × 2 腿 × ~满仓名义)。
    # 实际单腿费率高于此值 → 短差层净贡献转负。slice 名义以 INITIAL_CAPITAL 近似
    # （level_frac=1.0 时精确到 sell_price/entry_price 漂移内）。
    breakeven = (diff_net / (n_diffs * 2 * INITIAL_CAPITAL) * 1e4) if n_diffs else None
    return trades, {
        "n_diffs": n_diffs, "n_fly": n_fly,
        "fly_rate": round(n_fly / n_diffs * 100, 1) if n_diffs else None,
        "diff_net_cash": round(diff_net, 0),
        "breakeven_bps": round(breakeven, 1) if breakeven is not None else None,
    }


def run_symbol(symbol: str) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 卖飞修复政策矩阵（I_seg2 口径）\n{'=' * 64}",
          flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  {n:,} bars  BH={bh:+.2f}%  信号层（Rust O(N)）…", flush=True)
    t0 = time.time()
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)
    print(f"  信号层 {time.time() - t0:.0f}s。差分守卫…", flush=True)

    # ── 等价性守卫：B1_baseline ≡ run_version_i(floor=2) ──
    ref_trades, _ = run_version_i(i_signals, floor_ladder=FLOOR)
    b1 = DiffPolicy("B1_baseline", "any", "signal")
    chk_trades, _ = run_policy(i_signals, opens, highs, lows, b1)
    ref_pnls = [t.pnl_pct for t in ref_trades]
    chk_pnls = [t.pnl_pct for t in chk_trades]
    if ref_pnls != chk_pnls:
        raise AssertionError(
            f"差分守卫失败：政策 runner 与 run_version_i 不等价 "
            f"(n_ref={len(ref_pnls)} n_chk={len(chk_pnls)})——abort，禁止产出")
    print(f"  ✓ 差分守卫通过：{len(ref_pnls)} 笔逐笔相等。政策矩阵…", flush=True)

    rows: list[dict] = []
    b0_compound = None
    for policy in build_policies():
        trades, dstats = run_policy(i_signals, opens, highs, lows, policy)
        m = extended_metrics(trades, years)
        comp = round(m["total_compound"], 2)
        if policy.name == "B0_nodiff":
            b0_compound = comp
        rows.append({
            "name": policy.name, "open_gate": policy.open_gate,
            "close_mode": policy.close_mode,
            "limit_pct": None if policy.limit_pct == 0 else policy.limit_pct,
            "stop_pct": ("inf" if policy.stop_pct == INF
                         else (None if policy.stop_pct == 0 else policy.stop_pct)),
            "compound": comp, "n_trades": m["n"],
            "win_rate": round(m["win_rate"], 1), "max_dd": round(m["max_dd"], 1),
            **dstats,
        })
    for r in rows:
        r["vs_B0_pp"] = round(r["compound"] - b0_compound, 2)
    rows_sorted = sorted(rows, key=lambda r: -r["compound"])
    print(f"  B0(无短差)={b0_compound:+.1f}%  BH={bh:+.1f}%", flush=True)
    for r in rows_sorted[:8]:
        print(f"    {r['name']:24s} 复利={r['compound']:+9.2f}% vs_B0={r['vs_B0_pp']:+8.2f}pp"
              f" 短差={r['n_diffs']:4d} 卖飞率={r['fly_rate']}%", flush=True)
    n_beat = sum(1 for r in rows if r["vs_B0_pp"] > 0 and r["name"] != "B0_nodiff")
    print(f"  打败 B0 的政策数：{n_beat}/{len(rows) - 1}", flush=True)
    return {"n_bars": n, "bh": round(bh, 2), "b0_compound": b0_compound,
            "rows": rows_sorted}


def write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# v2 卖飞修复实验 — 短差层政策反事实矩阵（I_seg2 口径）\n")
    L.append(
        "> **判定标准**：floor 只影响短差层不影响进出场 → 短差层净贡献 = "
        "compound(政策) − compound(B0 无短差)。**『解决卖飞』= vs_B0 > 0**；"
        "打败 B1_baseline 但输 B0 = 只是少亏，严格结论仍是删除该层。"
        "差分守卫：B1_baseline 与 `run_version_i(floor=2)` 逐笔相等后矩阵才运行。"
        "成交模型保守（同 bar 双触取止损、止损 gap 按 open 更差价）。认识论 **L2**。\n")
    L.append(
        "> 政策轴：开仓门控（any=任意卖点[现行] / t1=type1 顶背驰[缠论第33课正典]）×"
        "平仓机制（signal=买点确认[现行] / bracket=限价回补δ+卖飞止损ε[绕过买回确认"
        "粒度] / sig_stop=信号+止损不冻结 / sig_profit_stop=只盈利信号平+止损）。\n")
    for sym, r in results.items():
        L.append(f"## {sym}（{r['n_bars']:,} bars，BH={r['bh']:+.1f}%，"
                 f"B0 无短差={r['b0_compound']:+.1f}%）\n")
        L.append("| 政策 | 门控 | 平仓 | δ% | ε% | 复利% | **vs_B0(pp)** | 笔 | 短差 | "
                 "卖飞率% | 短差净现金 | MDD |")
        L.append("|------|------|------|----|----|-------|--------------|----|------|"
                 "--------|-----------|-----|")
        for row in r["rows"]:
            L.append(
                f"| {row['name']} | {row['open_gate']} | {row['close_mode']} | "
                f"{row['limit_pct'] or '—'} | {row['stop_pct'] or '—'} | "
                f"{row['compound']:+.1f} | **{row['vs_B0_pp']:+.1f}** | "
                f"{row['n_trades']} | {row['n_diffs']} | "
                f"{row['fly_rate'] if row['fly_rate'] is not None else '—'} | "
                f"{row['diff_net_cash']:+.0f} | {row['max_dd']:+.1f} |")
        L.append("")
        beat = [row for row in r["rows"]
                if row["vs_B0_pp"] > 0 and row["name"] != "B0_nodiff"]
        if beat:
            L.append(f"> **{sym} 判决：{len(beat)} 个政策打败 B0**（短差层净贡献转正）。"
                     f"最优 = `{beat[0]['name']}`（vs_B0 {beat[0]['vs_B0_pp']:+.1f}pp）。\n")
        else:
            L.append(f"> **{sym} 判决：全矩阵无人打败 B0（否定性结果）**——在 segment "
                     "粒度上，卖出门控与买回机制的任何组合都无法使短差层净贡献转正。\n")
    OUT_MD.write_text("\n".join(L))


# ════════════════════════════════════════════════════════════
# 稳健性验证：跨标的赢家 × 成交模式（touch / pen / next）
# ════════════════════════════════════════════════════════════
#
# touch 模式的限价 wick 触碰成交是 1-min 回测最常见的假 alpha 来源。
# 判定：若 pen（穿透+滑点）与 next（次 bar 开盘市价，零 intrabar 假设）下
# vs_B0 仍 > 0 → 修复是结构性的；若只在 touch 下成立 → 微观结构伪影，否定。

ROBUST_OUT_MD = ROOT / "analysis" / "v2_selloff_fix_robustness.md"
ROBUST_OUT_JSON = DATA_DIR / "v2_selloff_fix_robustness.json"

# 跨标的赢家（touch 模式下 OKLO+BRN 双双 vs_B0>0）+ 两个 OKLO 单边最优（对照）
ROBUST_SET: list[tuple[str, float, float]] = [
    ("t1", 0.5, 0.5), ("t1", 0.5, 1.0), ("t1", 0.5, 2.0), ("t1", 0.5, INF),
    ("t1", 1.0, 0.5), ("any", 0.5, 2.0),
    ("t1", 1.0, 2.0), ("t1", 1.0, INF),   # OKLO 单边赢家（BRN 为负，对照）
]


def main_robustness() -> None:
    results: dict = {}
    for sym in SYMBOLS:
        print(f"\n{'=' * 64}\n  {sym} — 成交现实性稳健验证\n{'=' * 64}", flush=True)
        opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[sym])
        bh = (closes[-1] - closes[0]) / closes[0] * 100
        i_signals = compute_i_signals_rust(opens, highs, lows, closes)
        b0_trades, _ = run_policy(i_signals, opens, highs, lows,
                                  DiffPolicy("B0", "none", "signal"))
        b0 = round(extended_metrics(b0_trades, years)["total_compound"], 2)
        rows = []
        for g, d, e in ROBUST_SET:
            tag = f"{g}_brk_d{d}_e{'inf' if e == INF else e}"
            row = {"policy": tag}
            for fm in ("touch", "pen", "next"):
                p = DiffPolicy(f"{tag}_{fm}", g, "bracket", d, e, fill_mode=fm)
                trades, dstats = run_policy(i_signals, opens, highs, lows, p)
                m = extended_metrics(trades, years)
                row[fm] = {
                    "compound": round(m["total_compound"], 2),
                    "vs_B0": round(m["total_compound"] - b0, 2),
                    "n_diffs": dstats["n_diffs"],
                    "fly_rate": dstats["fly_rate"],
                    "breakeven_bps": dstats["breakeven_bps"],
                }
            rows.append(row)
            print(f"  {tag:22s} touch={row['touch']['vs_B0']:+8.1f}pp "
                  f"pen={row['pen']['vs_B0']:+8.1f}pp next={row['next']['vs_B0']:+8.1f}pp "
                  f"| be={row['next']['breakeven_bps']}bps", flush=True)
        results[sym] = {"bh": round(bh, 2), "b0": b0, "rows": rows}
        ROBUST_OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False,
                                              default=str))
        _write_robustness_report(results)
    print(f"\n报告：{ROBUST_OUT_MD}", flush=True)


def _write_robustness_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 卖飞修复 — 成交现实性稳健验证（赢家政策 × 三成交模式）\n")
    L.append(
        "> **目的**：touch 模式（wick 触碰按限价成交）是 1-min 回测最常见假 alpha 来源。"
        "pen = 限价须穿透 5bps + 止损滑点 5bps；next = 触发只作信号、市价单**次 bar 开盘**"
        "成交（零 intrabar 微观结构假设，最保守）。**修复成立 ⇔ next 模式下 vs_B0 仍 > 0**。"
        "`be_bps` = 盈亏平衡费率（bps/腿）：实际单腿交易成本高于此值则净贡献转负。\n")
    for sym, r in results.items():
        L.append(f"## {sym}（B0 无短差 = {r['b0']:+.1f}%，BH = {r['bh']:+.1f}%）\n")
        L.append("| 政策 | touch vs_B0 | pen vs_B0 | **next vs_B0** | next短差数 | "
                 "next卖飞率% | next be(bps/腿) |")
        L.append("|------|------------|-----------|---------------|-----------|"
                 "------------|----------------|")
        for row in r["rows"]:
            L.append(
                f"| {row['policy']} | {row['touch']['vs_B0']:+.1f} | "
                f"{row['pen']['vs_B0']:+.1f} | **{row['next']['vs_B0']:+.1f}** | "
                f"{row['next']['n_diffs']} | {row['next']['fly_rate']} | "
                f"{row['next']['breakeven_bps']} |")
        L.append("")
        survivors = [row["policy"] for row in r["rows"] if row["next"]["vs_B0"] > 0]
        L.append(f"> **{sym}：next 模式存活 {len(survivors)}/{len(r['rows'])}** — "
                 f"{', '.join(survivors) if survivors else '无（touch 伪影，否定）'}\n")
    ROBUST_OUT_MD.write_text("\n".join(L))


def main() -> None:
    if "--robustness" in sys.argv:
        main_robustness()
        return
    results: dict = {}
    for sym in SYMBOLS:
        results[sym] = run_symbol(sym)
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False,
                                       default=str))
        write_report(results)
    print(f"\n报告：{OUT_MD}\n聚合：{OUT_JSON}", flush=True)


if __name__ == "__main__":
    main()
