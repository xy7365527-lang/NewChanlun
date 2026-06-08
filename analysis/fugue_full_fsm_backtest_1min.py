"""完整版多层赋格 FSM 回测 — 1分钟数据版本（单 pass + 滑窗 BSP + fast PH）。

性能优化：
1. BiEngine 增量笔检测 — O(1)/bar
2. 滑窗 BSP pipeline (W=500 笔) — O(W)/笔变化，不随总笔数增长
3. PH merge tree settle-only _fast_alive — O(stack)/settle，跳过 settled 排序
4. 稀疏记录 — 仅存有事件的 bar
5. 单 pass — BiEngine + BSP + PH + FSM 同一循环

认识论等级：L2（真实数据，3标的 1min；可产生否定性结果）。
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass, field, replace as dc_replace
from datetime import datetime, timedelta
from enum import Enum, auto
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_buysellpoint_v1 import buysellpoints_from_level  # noqa: E402
from newchan.a_divergence_v1 import divergences_from_moves_v1  # noqa: E402
from newchan.a_move_v1 import moves_from_zhongshus  # noqa: E402
from newchan.a_online_persistence import (  # noqa: E402
    MergeBar,
    OnlineMergeTree,
)
from newchan.a_zhongshu_v1 import zhongshu_from_strokes  # noqa: E402
from newchan.bi_engine import BiEngine  # noqa: E402
from newchan.core.recursion.buysellpoint_state import diff_buysellpoints  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_full_fsm_backtest_1min.md"
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100000.0
BSP_WINDOW = 500
BSP_MARGIN = 50
LEVEL_ID = 1


# ════════════════════════════════════════════════════════════
# fast PH alive extraction — O(stack) 跳过 O(settled·log)
# ════════════════════════════════════════════════════════════

class _Alive:
    __slots__ = ("bars",)
    def __init__(self, bars: tuple[MergeBar, ...]):
        self.bars = bars


def _fast_alive(tree: OnlineMergeTree) -> _Alive:
    stack = tree._stack
    if not stack:
        return _Alive(())
    cap = tree._running_max
    n = len(tree._prices)
    last = len(stack) - 1
    bidx = tree._barrier_idx
    alive = []
    for k, c in enumerate(stack):
        lo = 0 if k == 0 else bidx[k - 1] + 1
        hi = n - 1 if k == last else bidx[k] - 1
        alive.append(MergeBar(
            birth_price=c.val, death_price=cap, persistence=cap - c.val,
            birth_idx=c.idx, death_idx=None, lo=lo, hi=hi, settled=False,
        ))
    alive.sort(key=lambda b: b.persistence, reverse=True)
    return _Alive(tuple(alive))


# ════════════════════════════════════════════════════════════
# FSM types
# ════════════════════════════════════════════════════════════

class St(Enum):
    SCANNING = auto()
    POSITION_OPEN = auto()
    COST_REDUCING = auto()
    PRINCIPAL_WITHDRAWN = auto()
    STOPPED_OUT = auto()


@dataclass
class CompletedTrade:
    entry_bar: int
    entry_price: float
    exit_bar: int
    exit_price: float
    direction: int
    pnl_pct: float
    states_visited: list[str]
    n_add_positions: int
    n_short_diffs: int
    cumulative_recovered: float
    principal_withdrawn: bool
    exit_reason: str
    cost_basis_at_exit: float


@dataclass
class EventRecord:
    bar_idx: int
    price: float
    state: str
    event: str


# ════════════════════════════════════════════════════════════
# 数据加载
# ════════════════════════════════════════════════════════════

_1MIN_FILES = {
    "QQQ": DATA_DIR / "qqq_1m_databento_full.json",
    "OKLO": DATA_DIR / "oklo_1m_databento_full.json",
    "HK700": DATA_DIR / "hk700_1m_tws.json",
}


def load_1min(symbol: str) -> tuple[list[float], list[float], list[float], list[float], list[str]]:
    path = _1MIN_FILES[symbol.upper()]
    raw = json.loads(path.read_text())
    if isinstance(raw, dict) and "opens" in raw:
        n = len(raw["closes"])
        return (
            [float(x) for x in raw["opens"]],
            [float(x) for x in raw["highs"]],
            [float(x) for x in raw["lows"]],
            [float(x) for x in raw["closes"]],
            [f"bar_{i}" for i in range(n)],
        )
    return (
        [float(b.get("open", b["close"])) for b in raw],
        [float(b.get("high", b["close"])) for b in raw],
        [float(b.get("low", b["close"])) for b in raw],
        [float(b["close"]) for b in raw],
        [b.get("date", f"bar_{i}") for i, b in enumerate(raw)],
    )


# ════════════════════════════════════════════════════════════
# 单 pass 回测引擎
# ════════════════════════════════════════════════════════════

def run_backtest(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> tuple[list[CompletedTrade], list[EventRecord], dict[str, int]]:
    n = len(closes)
    bi = BiEngine()
    tree_down = OnlineMergeTree(track_dominant=False)
    tree_up = OnlineMergeTree(track_dominant=False)
    base_ts = datetime(2020, 1, 1)

    # BSP windowed state
    prev_adj_bsps: list = []
    prev_n_strokes = 0
    bsp_seq = 0

    # PH cached state
    down_alive = _fast_alive(tree_down)
    up_alive = _fast_alive(tree_up)
    prev_down_r1: int | None = None
    prev_up_r1: int | None = None

    # FSM
    state = St.SCANNING
    direction = 0
    total_shares = 0.0
    cost_basis = 0.0
    entry_price = 0.0
    entry_bar = -1
    own_capital = INITIAL_CAPITAL
    cumul_recovered = 0.0
    trim_active = False
    trim_sell_price = 0.0
    trim_shares = 0.0
    total_invested = 0.0
    pending: dict | None = None
    entry_bsps: set[int] = set()
    states_in_trade: set[str] = set()
    add_count = 0
    diff_count = 0

    trades: list[CompletedTrade] = []
    event_recs: list[EventRecord] = []
    state_counts: dict[str, int] = {s.name: 0 for s in St}
    last_log = 0

    def _pratio(alive: _Alive, rank: int) -> float:
        ab = alive.bars
        if len(ab) > rank and ab[0].persistence > 0:
            return ab[rank].persistence / ab[0].persistence
        return 0.0

    def _close_trade(idx: int, price: float, reason: str) -> None:
        nonlocal state, direction, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumul_recovered, own_capital
        nonlocal trim_active, pending, entry_bsps, states_in_trade
        nonlocal add_count, diff_count
        pnl = (price - cost_basis) / entry_price * 100 * direction
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=idx, exit_price=price, direction=direction,
            pnl_pct=round(pnl, 4),
            states_visited=sorted(states_in_trade),
            n_add_positions=add_count, n_short_diffs=diff_count,
            cumulative_recovered=cumul_recovered,
            principal_withdrawn=(state == St.PRINCIPAL_WITHDRAWN),
            exit_reason=reason, cost_basis_at_exit=cost_basis,
        ))
        rem = INITIAL_CAPITAL * (1 + pnl / 100)
        state = St.SCANNING
        direction = 0
        total_shares = cost_basis = entry_price = 0.0
        entry_bar = -1
        cumul_recovered = 0.0
        own_capital = max(rem, 1.0)
        trim_active = False
        pending = None
        entry_bsps = set()
        states_in_trade = set()
        add_count = diff_count = 0

    for i in range(n):
        c = closes[i]
        bar = Bar(ts=base_ts + timedelta(minutes=i),
                  open=opens[i], high=highs[i], low=lows[i], close=c, volume=0.0)

        # ── 1. BiEngine (incremental strokes) ──
        bi_snap = bi.process_bar(bar)

        # ── 2. Windowed BSP (only on stroke change) ──
        buy_cands: list[int] = []
        sell_cands: list[int] = []
        buy_inv: list[int] = []
        sell_inv: list[int] = []

        if bi_snap.events:
            strokes = bi_snap.strokes
            ns = len(strokes)
            if ns != prev_n_strokes:
                prev_n_strokes = ns
                w_start = max(0, ns - BSP_WINDOW)
                window = strokes[w_start:]

                zs = zhongshu_from_strokes(window)
                mv = moves_from_zhongshus(zs, num_segments=len(window))
                dv = divergences_from_moves_v1(window, zs, mv, LEVEL_ID)
                curr = buysellpoints_from_level(window, zs, mv, dv, LEVEL_ID)
                adj = [dc_replace(bp, seg_idx=bp.seg_idx + w_start) for bp in curr]

                evs = diff_buysellpoints(prev_adj_bsps, adj, bar_idx=i,
                                          bar_ts=c, seq_start=bsp_seq)
                bsp_seq += len(evs)
                inv_thresh = w_start + BSP_MARGIN

                for e in evs:
                    nm = type(e).__name__
                    bid = e.bsp_id
                    side = e.side
                    if "Invalidate" in nm:
                        if getattr(e, "seg_idx", 0) < inv_thresh:
                            continue
                        (buy_inv if side == "buy" else sell_inv).append(bid)
                    elif "Candidate" in nm:
                        (buy_cands if side == "buy" else sell_cands).append(bid)
                    elif "Confirm" in nm:
                        (buy_cands if side == "buy" else sell_cands).append(bid)

                prev_adj_bsps = adj

        # ── 3. PH (settle-only barcode) ──
        ds = tree_down.update(c)
        us = tree_up.update(-c)

        flip_long = flip_short = non_r1_down = non_r1_up = False

        if ds:
            if prev_down_r1 is not None:
                for mb in ds:
                    if mb.birth_idx == prev_down_r1:
                        flip_long = True
                        break
            non_r1_down = bool(ds) and not flip_long
            down_alive = _fast_alive(tree_down)
            prev_down_r1 = down_alive.bars[1].birth_idx if len(down_alive.bars) >= 2 else None

        if us:
            if prev_up_r1 is not None:
                for mb in us:
                    if mb.birth_idx == prev_up_r1:
                        flip_short = True
                        break
            non_r1_up = bool(us) and not flip_short
            up_alive = _fast_alive(tree_up)
            prev_up_r1 = up_alive.bars[1].birth_idx if len(up_alive.bars) >= 2 else None

        # ── 4. FSM ──
        state_counts[state.name] += 1
        ev = ""

        if state == St.SCANNING:
            if buy_cands:
                pending = {"bar": i, "side": "buy", "ids": set(buy_cands)}
            if sell_cands:
                pending = {"bar": i, "side": "sell", "ids": set(sell_cands)}
            if pending:
                if (i - pending["bar"]) > PENDING_EXPIRY:
                    pending = None
                else:
                    inv = buy_inv if pending["side"] == "buy" else sell_inv
                    if any(b in pending["ids"] for b in inv):
                        pending = None

            entered = False
            if pending and pending["side"] == "buy" and flip_long:
                direction = 1
                entered = True
                ev = f"ENTRY_LONG@{c:.2f}"
            elif pending and pending["side"] == "sell" and flip_short:
                direction = -1
                entered = True
                ev = f"ENTRY_SHORT@{c:.2f}"

            if entered:
                state = St.POSITION_OPEN
                entry_price = cost_basis = c
                entry_bar = i
                total_shares = INITIAL_CAPITAL / c
                total_invested = INITIAL_CAPITAL
                own_capital = INITIAL_CAPITAL
                cumul_recovered = 0.0
                entry_bsps = set(pending["ids"])
                pending = None
                trim_active = False
                add_count = diff_count = 0
                states_in_trade = {St.POSITION_OPEN.name}

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            states_in_trade.add(state.name)
            should_stop = False
            stop_reason = ""

            if direction == 1 and flip_short:
                should_stop, stop_reason = True, "direction_flip_short"
            elif direction == -1 and flip_long:
                should_stop, stop_reason = True, "direction_flip_long"

            if not should_stop:
                s_inv = buy_inv if direction == 1 else sell_inv
                if any(b in entry_bsps for b in s_inv):
                    should_stop, stop_reason = True, "bsp_invalidate"

            if should_stop:
                pnl = (c - cost_basis) / entry_price * 100 * direction
                ev = f"STOP:{stop_reason}@{c:.2f} pnl={pnl:+.2f}%"
                _close_trade(i, c, stop_reason)

            else:
                # 加仓
                new_bids = [b for b in (buy_cands if direction == 1 else sell_cands) if b not in entry_bsps]
                if new_bids and state in (St.POSITION_OPEN, St.COST_REDUCING):
                    al = down_alive if direction == 1 else up_alive
                    r = _pratio(al, 1)
                    if r > 0.05:
                        add_s = total_shares * r
                        cost_basis = (total_shares * cost_basis + add_s * c) / (total_shares + add_s)
                        total_shares += add_s
                        total_invested += add_s * c
                        add_count += 1
                        entry_bsps.update(new_bids)
                        ev = f"ADD@{c:.2f} +{add_s:.1f} r={r:.3f}"

                # 降成本
                if state == St.POSITION_OPEN:
                    do_trim = False
                    if direction == 1 and non_r1_up:
                        r = _pratio(up_alive, 1)
                        do_trim = r > 0.05
                    elif direction == -1 and non_r1_down:
                        r = _pratio(down_alive, 1)
                        do_trim = r > 0.05
                    if do_trim and not trim_active:
                        trim_sell_price = c
                        trim_shares = total_shares * r
                        trim_active = True
                        state = St.COST_REDUCING
                        states_in_trade.add(state.name)
                        if not ev:
                            ev = f"TRIM@{c:.2f} s={trim_shares:.1f} r={r:.3f}"

                elif state == St.COST_REDUCING:
                    if trim_active:
                        close_d = False
                        if direction == 1 and non_r1_down:
                            close_d = True
                        elif direction == -1 and non_r1_up:
                            close_d = True
                        if buy_cands if direction == 1 else sell_cands:
                            close_d = True
                        if close_d:
                            profit = ((trim_sell_price - c) if direction == 1 else (c - trim_sell_price)) * trim_shares
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumul_recovered += profit
                            diff_count += 1
                            trim_active = False
                            ev = f"CLOSE_DIFF@{c:.2f} p={profit:.2f}"
                            if cumul_recovered >= own_capital:
                                state = St.PRINCIPAL_WITHDRAWN
                                states_in_trade.add(state.name)
                                ev += " → PW"
                            else:
                                state = St.POSITION_OPEN
                    else:
                        do_new = False
                        if direction == 1 and non_r1_up:
                            r = _pratio(up_alive, 1)
                            do_new = r > 0.05
                        elif direction == -1 and non_r1_down:
                            r = _pratio(down_alive, 1)
                            do_new = r > 0.05
                        if do_new:
                            trim_sell_price = c
                            trim_shares = total_shares * r
                            trim_active = True
                            if not ev:
                                ev = f"NEW_TRIM@{c:.2f} s={trim_shares:.1f}"

                elif state == St.PRINCIPAL_WITHDRAWN:
                    if not trim_active:
                        do_pw = False
                        if direction == 1 and non_r1_up:
                            r = _pratio(up_alive, 1)
                            do_pw = r > 0.05
                        elif direction == -1 and non_r1_down:
                            r = _pratio(down_alive, 1)
                            do_pw = r > 0.05
                        if do_pw:
                            trim_sell_price = c
                            trim_shares = total_shares * r
                            trim_active = True
                            ev = f"PW_TRIM@{c:.2f}"
                    elif trim_active:
                        close_d = False
                        if direction == 1 and non_r1_down:
                            close_d = True
                        elif direction == -1 and non_r1_up:
                            close_d = True
                        if buy_cands if direction == 1 else sell_cands:
                            close_d = True
                        if close_d:
                            profit = ((trim_sell_price - c) if direction == 1 else (c - trim_sell_price)) * trim_shares
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumul_recovered += profit
                            diff_count += 1
                            trim_active = False
                            ev = f"PW_CLOSE@{c:.2f} p={profit:.2f}"

        elif state == St.STOPPED_OUT:
            state = St.SCANNING
            ev = "AUTO_RESET"

        if ev:
            event_recs.append(EventRecord(i, c, state.name, ev))

        if i - last_log >= 100000:
            print(f"    [{i/n*100:5.1f}%] bar {i:,}/{n:,}")
            last_log = i

    if state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
        _close_trade(n - 1, closes[-1], "eod_close")

    return trades, event_recs, state_counts


# ════════════════════════════════════════════════════════════
# 统计 + 报告
# ════════════════════════════════════════════════════════════

def compute_metrics(trades: list[CompletedTrade]) -> dict:
    if not trades:
        return {"n": 0, "win_rate": 0.0, "avg_pnl": 0.0,
                "total_compound": 0.0, "max_dd": 0.0,
                "avg_hold_bars": 0, "n_with_add": 0,
                "n_with_cr": 0, "n_pw": 0, "long_n": 0, "short_n": 0}
    wins = sum(1 for t in trades if t.pnl_pct > 0)
    eq = 1.0
    peak = 1.0
    max_dd = 0.0
    for t in trades:
        eq *= (1 + t.pnl_pct / 100)
        peak = max(peak, eq)
        max_dd = min(max_dd, (eq - peak) / peak)
    return {
        "n": len(trades),
        "win_rate": wins / len(trades) * 100,
        "avg_pnl": sum(t.pnl_pct for t in trades) / len(trades),
        "total_compound": (eq - 1) * 100,
        "max_dd": max_dd * 100,
        "avg_hold_bars": round(sum(t.exit_bar - t.entry_bar for t in trades) / len(trades)),
        "n_with_add": sum(1 for t in trades if t.n_add_positions > 0),
        "n_with_cr": sum(1 for t in trades if t.n_short_diffs > 0),
        "n_pw": sum(1 for t in trades if t.principal_withdrawn),
        "long_n": sum(1 for t in trades if t.direction == 1),
        "short_n": sum(1 for t in trades if t.direction == -1),
    }


def write_report(all_results: list[dict]) -> None:
    L: list[str] = []
    L.append("# 完整版多层赋格 FSM 回测（H组）— 1分钟数据\n")
    L.append("## 架构\n")
    L.append("5状态 FSM：`SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN → STOPPED_OUT`\n")
    L.append("单 pass：BiEngine (增量笔) + 滑窗 BSP (W=500) + PH settle-only + FSM。\n")
    L.append("| 维度 | 实现 | 参数来源 |")
    L.append("|------|------|---------|")
    L.append("| 方向 | rank-1 settle | 零参数 |")
    L.append("| 进场 | chanlun candidate + PH settle 门控 | 零参数 |")
    L.append("| 加仓 | 同方向新 BSP | ratio = alive_rank1 / dominant |")
    L.append("| 降成本 | non-rank1 settle → 减仓 | ratio = alive_rank1 / dominant |")
    L.append("| 回补 | 同方向 settle 或 BSP → 买回 | ≤ 已减量 |")
    L.append("| 本金回收 | cumulative_recovered ≥ own_capital | 自动阈值 |")
    L.append("| 清仓 | rank-1 settle 或 BSP invalidate | 零参数 |")
    L.append("| 双向 | 多/空对称 | — |")
    L.append(f"| PENDING_EXPIRY | {PENDING_EXPIRY} bars | — |\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：**{res['n_bars']:,}** bars (1min), {res['dates'][0]} → {res['dates'][-1]}")
        L.append(f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}")
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**")
        L.append(f"- 回测耗时：**{res['elapsed']:.1f}s** ({res['n_bars']/res['elapsed']:.0f} bars/s)\n")

        L.append("### 总体指标\n")
        L.append("| 指标 | 值 |")
        L.append("|------|-----|")
        L.append(f"| 交易数 | {m['n']} (多{m['long_n']}/空{m['short_n']}) |")
        L.append(f"| 胜率 | {m['win_rate']:.1f}% |")
        L.append(f"| 平均收益 | {m['avg_pnl']:+.3f}% |")
        L.append(f"| 复利累计 | {m['total_compound']:+.2f}% |")
        L.append(f"| 最大回撤 | {m['max_dd']:.2f}% |")
        L.append(f"| 平均持仓 | {m['avg_hold_bars']:,} bars |")
        L.append(f"| 有加仓 | {m['n_with_add']}/{m['n']} |")
        L.append(f"| 有降成本 | {m['n_with_cr']}/{m['n']} |")
        L.append(f"| 本金回收 | {m['n_pw']}/{m['n']} |")
        L.append("")

        L.append("### 交易明细\n")
        L.append("| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | PW | 退出 | 状态 |")
        L.append("|---|------|------|------|------|------|------|------|-----|------|------|")
        for idx, t in enumerate(res["trades"]):
            d = "多" if t.direction == 1 else "空"
            pw = "Y" if t.principal_withdrawn else ""
            sts = "→".join(t.states_visited)
            L.append(f"| {idx+1} | {d} | {t.entry_price:.2f}@{t.entry_bar} | "
                     f"{t.exit_price:.2f}@{t.exit_bar} | {t.exit_bar-t.entry_bar:,} | "
                     f"{t.pnl_pct:+.2f} | {t.n_add_positions} | {t.n_short_diffs} | "
                     f"{pw} | {t.exit_reason} | {sts} |")
        L.append("")

        L.append("### FSM 状态分布\n")
        L.append("| 状态 | Bars | 占比 |")
        L.append("|------|------|------|")
        tot = sum(res["state_counts"].values())
        for st in ["SCANNING", "POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN", "STOPPED_OUT"]:
            cnt = res["state_counts"].get(st, 0)
            L.append(f"| {st} | {cnt:,} | {cnt/tot*100:.1f}% |")
        L.append("")

        evts = res["event_records"]
        if evts:
            L.append(f"<details><summary>事件流（{len(evts)}条）</summary>\n")
            L.append("| Bar | 价格 | 状态 | 事件 |")
            L.append("|-----|------|------|------|")
            for r in evts[:80]:
                L.append(f"| {r.bar_idx:,} | {r.price:.2f} | {r.state} | {r.event} |")
            if len(evts) > 80:
                L.append(f"| ... | ... | ... | （共{len(evts)}条） |")
            L.append("</details>\n")

    L.append("## 汇总\n")
    L.append("| 标的 | Bars | BH% | FSM% | 胜率 | 交易 | 加仓 | 降成本 | PW | 耗时 |")
    L.append("|------|------|-----|------|------|------|------|--------|-----|------|")
    for res in all_results:
        m = res["metrics"]
        n = m["n"] or 1
        L.append(f"| {res['symbol']} | {res['n_bars']:,} | {res['bh']:+.1f} | "
                 f"{m['total_compound']:+.2f} | {m['win_rate']:.0f}% | {m['n']} | "
                 f"{m['n_with_add']}/{n} | {m['n_with_cr']}/{n} | {m['n_pw']}/{n} | "
                 f"{res['elapsed']:.0f}s |")
    L.append("")

    L.append("## 结果包\n")
    L.append("**结论**：完整版 5 状态 FSM 在 1min 级别 3 标的。加仓/降成本/本金回收全实装。\n")
    L.append("**定义依据**：FSM 5状态(267号)，加仓/降成本 ratio 零参数(alive persistence 结构)。\n")
    L.append("**边界条件**：")
    L.append(f"- PENDING_EXPIRY={PENDING_EXPIRY} bars，BSP_WINDOW={BSP_WINDOW} 笔")
    L.append("- 无滑点/手续费，1min 噪声交易多\n")
    L.append("**下游推论**：降成本周期更多(1min 级别 settle 频繁)，但本金回收率仍低。\n")
    L.append("**谱系**：267号(FSM)，268a号(own_capital)，§7.5(merge tree)，Elder rule。\n")
    L.append("**影响**：独立脚本，不修改 FSM 主代码。\n")
    L.append("**认识论等级**：L2。")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


def main():
    symbols = ["QQQ", "OKLO", "HK700"]
    all_results = []

    for symbol in symbols:
        print(f"\n{'='*60}")
        print(f"  {symbol} — 完整版 FSM 回测 (1min)")
        print(f"{'='*60}")
        try:
            opens, highs, lows, closes, dates = load_1min(symbol)
            n = len(closes)
            print(f"  数据：{n:,} bars")

            t0 = time.time()
            trades, event_recs, state_counts = run_backtest(opens, highs, lows, closes)
            elapsed = time.time() - t0

            m = compute_metrics(trades)
            bh = (closes[-1] - closes[0]) / closes[0] * 100

            print(f"  完成：{elapsed:.1f}s ({n/elapsed:.0f} bars/s)")
            print(f"  交易：{m['n']}笔 (多{m['long_n']}/空{m['short_n']})")
            print(f"  胜率：{m['win_rate']:.1f}%, 复利：{m['total_compound']:+.2f}%")
            print(f"  加仓：{m['n_with_add']}/{m['n']}, "
                  f"降成本：{m['n_with_cr']}/{m['n']}, "
                  f"本金回收：{m['n_pw']}/{m['n']}")
            print(f"  BH: {bh:+.2f}%")

            all_results.append({
                "symbol": symbol, "n_bars": n, "closes": closes, "dates": dates,
                "bh": bh, "metrics": m, "trades": trades,
                "event_records": event_recs, "state_counts": state_counts,
                "elapsed": elapsed,
            })
        except Exception as ex:
            print(f"  FAILED: {ex}")
            import traceback
            traceback.print_exc()

    if all_results:
        write_report(all_results)


if __name__ == "__main__":
    main()
