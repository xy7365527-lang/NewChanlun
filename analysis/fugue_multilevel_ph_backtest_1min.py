"""多级别 PH 分层赋格 FSM 回测 — 1分钟数据版本。

核心改进（vs fugue_full_fsm_backtest_1min.py）：
- L0 PH树：输入=1min close，settle事件做进场门控
- L1 PH树：输入=L1线段端点价格，settle事件做降成本触发
- L2 PH树：输入=L1走势端点价格，rank-1 settle做方向裁决

信号消费映射：
| 信号         | PH级别 | 触发条件              |
|-------------|--------|----------------------|
| 方向裁决     | L2     | rank-1 settle        |
| 进场门控     | L0     | rank-1 settle + BSP  |
| 降成本触发   | L1     | non-rank-1 settle    |
| 降成本平仓   | L1     | non-rank-1 反向settle |
| 加仓比例     | L1     | persistence ratio    |
| 止损         | L2     | rank-1 反向settle     |

认识论等级：L2（真实数据，3标的 1min；可产生否定性结果）。
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum, auto
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import (  # noqa: E402
    MergeBar,
    OnlineMergeTree,
)
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_multilevel_ph_backtest_1min.md"
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100000.0


# ════════════════════════════════════════════════════════════
# PH alive 提取（O(stack) — 跳过 current_barcode 的 O(settled·log)）
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
# PH rank-1 settle 检测（三级别通用）
# ════════════════════════════════════════════════════════════

@dataclass
class PHLevelState:
    """一个方向（down/up）的 PH 树 + rank-1 追踪状态。"""
    tree: OnlineMergeTree = field(default_factory=lambda: OnlineMergeTree(track_dominant=False))
    alive: _Alive = field(default_factory=lambda: _Alive(()))
    prev_r1_birth: int | None = None

    def detect_settle(self, settles: list[MergeBar]) -> tuple[bool, bool]:
        """检测 rank-1 settle 和 non-rank-1 settle。

        Returns: (is_rank1, is_non_rank1)
        """
        if not settles:
            return False, False
        rank1 = False
        if self.prev_r1_birth is not None:
            for mb in settles:
                if mb.birth_idx == self.prev_r1_birth:
                    rank1 = True
                    break
        self.alive = _fast_alive(self.tree)
        self.prev_r1_birth = (
            self.alive.bars[1].birth_idx if len(self.alive.bars) >= 2 else None
        )
        return rank1, bool(settles) and not rank1


# ════════════════════════════════════════════════════════════
# FSM 状态
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
# 单 pass 多级别回测引擎
# ════════════════════════════════════════════════════════════

def run_backtest(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> tuple[list[CompletedTrade], list[EventRecord], dict[str, int], dict[str, int]]:
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id="bt", max_levels=2)
    base_ts = datetime(2020, 1, 1)

    # ── 三级别 PH 树 ──
    l0_down = PHLevelState()
    l0_up = PHLevelState()
    l1_down = PHLevelState()
    l1_up = PHLevelState()
    l2_down = PHLevelState()
    l2_up = PHLevelState()

    # ── 更新计数器 ──
    ph_counts: dict[str, int] = {
        "l0_settles": 0, "l0_r1_settles": 0,
        "l1_updates": 0, "l1_settles": 0, "l1_r1_settles": 0,
        "l2_updates": 0, "l2_settles": 0, "l2_r1_settles": 0,
        "l2_direction_flips": 0,
    }

    # ── L2 方向状态（持久） ──
    l2_direction = 0  # 0=未决, 1=多, -1=空

    # ── FSM 状态 ──
    state = St.SCANNING
    direction = 0
    total_shares = 0.0
    cost_basis = 0.0
    entry_price = 0.0
    entry_bar = -1
    own_capital = INITIAL_CAPITAL
    cumulative_recovered = 0.0
    active_trim_sell_bar = -1
    active_trim_sell_price = 0.0
    active_trim_shares = 0.0
    has_active_trim = False
    total_invested = 0.0
    pending_entry: dict | None = None
    entry_bsp_ids: set[int] = set()
    states_in_trade: set[str] = set()

    add_count = 0
    diff_count = 0

    trades: list[CompletedTrade] = []
    event_records: list[EventRecord] = []
    state_counts: dict[str, int] = {s.name: 0 for s in St}
    last_progress = 0

    def _persistence_ratio(alive: _Alive, rank: int) -> float:
        ab = alive.bars
        if len(ab) > rank and ab[0].persistence > 0:
            return ab[rank].persistence / ab[0].persistence
        return 0.0

    def _record(bar_idx: int, price: float, ev: str) -> None:
        event_records.append(EventRecord(bar_idx, price, state.name, ev))

    def _close_trade(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, direction, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumulative_recovered, own_capital
        nonlocal has_active_trim, pending_entry, entry_bsp_ids
        nonlocal add_count, diff_count, states_in_trade

        pnl_pct = (price - cost_basis) / entry_price * 100 * direction
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price, direction=direction,
            pnl_pct=round(pnl_pct, 4),
            states_visited=sorted(states_in_trade),
            n_add_positions=add_count,
            n_short_diffs=diff_count,
            cumulative_recovered=cumulative_recovered,
            principal_withdrawn=(state == St.PRINCIPAL_WITHDRAWN),
            exit_reason=reason, cost_basis_at_exit=cost_basis,
        ))
        remaining = INITIAL_CAPITAL * (1 + pnl_pct / 100)
        state = St.SCANNING
        direction = 0
        total_shares = 0.0
        cost_basis = 0.0
        entry_price = 0.0
        entry_bar = -1
        cumulative_recovered = 0.0
        own_capital = max(remaining, 1.0)
        has_active_trim = False
        pending_entry = None
        entry_bsp_ids = set()
        add_count = 0
        diff_count = 0
        states_in_trade = set()

    for i in range(n):
        c = closes[i]
        bar = Bar(ts=base_ts + timedelta(minutes=i),
                  open=opens[i], high=highs[i], low=lows[i], close=c, volume=0.0)

        # ══════ 1. Incremental BSP pipeline ══════
        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events

        # ══════ 2. L0 PH（每 bar 更新） ══════
        l0_ds = l0_down.tree.update(c)
        l0_us = l0_up.tree.update(-c)

        l0_r1_down, l0_nr1_down = l0_down.detect_settle(l0_ds)
        l0_r1_up, l0_nr1_up = l0_up.detect_settle(l0_us)

        if l0_ds:
            ph_counts["l0_settles"] += len(l0_ds)
        if l0_us:
            ph_counts["l0_settles"] += len(l0_us)
        if l0_r1_down:
            ph_counts["l0_r1_settles"] += 1
        if l0_r1_up:
            ph_counts["l0_r1_settles"] += 1

        # ══════ 3. L1 PH（仅在 L1 线段 settle 时更新） ══════
        l1_r1_down = False
        l1_r1_up = False
        l1_nr1_down = False
        l1_nr1_up = False

        for e in snap.seg_snapshot.events:
            if isinstance(e, SegmentSettleV1):
                ph_counts["l1_updates"] += 1
                ep = e.ep1_price
                if ep <= 0:
                    continue
                ds1 = l1_down.tree.update(ep)
                us1 = l1_up.tree.update(-ep)
                r1d, nr1d = l1_down.detect_settle(ds1)
                r1u, nr1u = l1_up.detect_settle(us1)
                if ds1:
                    ph_counts["l1_settles"] += len(ds1)
                if us1:
                    ph_counts["l1_settles"] += len(us1)
                if r1d:
                    l1_r1_down = True
                    ph_counts["l1_r1_settles"] += 1
                if r1u:
                    l1_r1_up = True
                    ph_counts["l1_r1_settles"] += 1
                if nr1d:
                    l1_nr1_down = True
                if nr1u:
                    l1_nr1_up = True

        # ══════ 4. L2 PH（仅在 L1 走势 settle 时更新） ══════
        l2_flip_long = False
        l2_flip_short = False

        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1):
                mv = None
                for m in snap.move_snapshot.moves:
                    if m.seg_start == e.seg_start and m.direction == e.direction and m.settled:
                        mv = m
                        break
                if mv is None:
                    continue
                ep = mv.high if mv.direction == "up" else mv.low
                if ep <= 0:
                    continue
                ph_counts["l2_updates"] += 1
                ds2 = l2_down.tree.update(ep)
                us2 = l2_up.tree.update(-ep)
                r1d, _ = l2_down.detect_settle(ds2)
                r1u, _ = l2_up.detect_settle(us2)
                if ds2:
                    ph_counts["l2_settles"] += len(ds2)
                if us2:
                    ph_counts["l2_settles"] += len(us2)
                if r1d:
                    l2_flip_long = True
                    ph_counts["l2_r1_settles"] += 1
                if r1u:
                    l2_flip_short = True
                    ph_counts["l2_r1_settles"] += 1

        # ── 更新 L2 方向状态 ──
        if l2_flip_long:
            if l2_direction != 1:
                ph_counts["l2_direction_flips"] += 1
            l2_direction = 1
        if l2_flip_short:
            if l2_direction != -1:
                ph_counts["l2_direction_flips"] += 1
            l2_direction = -1

        # ══════ 5. BSP 事件提取 ══════
        buy_cands: list[int] = []
        sell_cands: list[int] = []
        buy_confirms: list[int] = []
        sell_confirms: list[int] = []
        buy_invalidates: list[int] = []
        sell_invalidates: list[int] = []

        for e in bsp_events:
            nm = type(e).__name__
            bid = e.bsp_id
            side = e.side
            if "Candidate" in nm:
                (buy_cands if side == "buy" else sell_cands).append(bid)
            elif "Confirm" in nm:
                (buy_confirms if side == "buy" else sell_confirms).append(bid)
            elif "Invalidate" in nm:
                (buy_invalidates if side == "buy" else sell_invalidates).append(bid)

        state_counts[state.name] += 1
        ev = ""

        # ══════ 6. FSM 状态机 ══════
        if state == St.SCANNING:
            # BSP candidates 只接受匹配 L2 方向的
            if buy_cands and l2_direction == 1:
                pending_entry = {"bar": i, "side": "buy", "bsp_ids": set(buy_cands)}
            if sell_cands and l2_direction == -1:
                pending_entry = {"bar": i, "side": "sell", "bsp_ids": set(sell_cands)}

            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                else:
                    ps = pending_entry["side"]
                    inv_list = buy_invalidates if ps == "buy" else sell_invalidates
                    if any(bid in pending_entry["bsp_ids"] for bid in inv_list):
                        pending_entry = None

            # 进场：pending BSP + L0 rank-1 settle 确认
            if pending_entry and pending_entry["side"] == "buy" and l0_r1_down:
                state = St.POSITION_OPEN
                direction = 1
                entry_price = c
                entry_bar = i
                cost_basis = c
                total_shares = INITIAL_CAPITAL / c
                total_invested = INITIAL_CAPITAL
                own_capital = INITIAL_CAPITAL
                cumulative_recovered = 0.0
                entry_bsp_ids = set(pending_entry["bsp_ids"])
                pending_entry = None
                has_active_trim = False
                add_count = 0
                diff_count = 0
                states_in_trade = {St.POSITION_OPEN.name}
                ev = f"ENTRY_LONG@{c:.2f} L2dir={l2_direction}"

            elif pending_entry and pending_entry["side"] == "sell" and l0_r1_up:
                state = St.POSITION_OPEN
                direction = -1
                entry_price = c
                entry_bar = i
                cost_basis = c
                total_shares = INITIAL_CAPITAL / c
                total_invested = INITIAL_CAPITAL
                own_capital = INITIAL_CAPITAL
                cumulative_recovered = 0.0
                entry_bsp_ids = set(pending_entry["bsp_ids"])
                pending_entry = None
                has_active_trim = False
                add_count = 0
                diff_count = 0
                states_in_trade = {St.POSITION_OPEN.name}
                ev = f"ENTRY_SHORT@{c:.2f} L2dir={l2_direction}"

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            states_in_trade.add(state.name)

            should_stop = False
            stop_reason = ""

            # 止损：L2 方向翻转
            if direction == 1 and l2_flip_short:
                should_stop = True
                stop_reason = "l2_direction_flip_short"
            elif direction == -1 and l2_flip_long:
                should_stop = True
                stop_reason = "l2_direction_flip_long"

            # BSP invalidate 仍然检查
            if not should_stop:
                side_inv = buy_invalidates if direction == 1 else sell_invalidates
                if any(bid in entry_bsp_ids for bid in side_inv):
                    should_stop = True
                    stop_reason = "bsp_invalidate"

            if should_stop:
                pnl = (c - cost_basis) / entry_price * 100 * direction
                ev = f"STOP:{stop_reason}@{c:.2f} pnl={pnl:+.2f}%"
                _close_trade(i, c, stop_reason)

            else:
                # ── 加仓逻辑（BSP + L1 persistence ratio）──
                side_match_cands = buy_cands + buy_confirms if direction == 1 else sell_cands + sell_confirms
                new_bsp_ids = [bid for bid in side_match_cands if bid not in entry_bsp_ids]

                if new_bsp_ids and state in (St.POSITION_OPEN, St.COST_REDUCING):
                    alive = l1_down.alive if direction == 1 else l1_up.alive
                    ratio = _persistence_ratio(alive, 1)
                    if ratio > 0.05:
                        add_shares = total_shares * ratio
                        old_val = total_shares * cost_basis
                        new_total = total_shares + add_shares
                        cost_basis = (old_val + add_shares * c) / new_total
                        total_shares = new_total
                        total_invested += add_shares * c
                        add_count += 1
                        entry_bsp_ids.update(new_bsp_ids)
                        ev = f"ADD_POS@{c:.2f} +{add_shares:.1f} L1ratio={ratio:.3f}"

                # ── 降成本逻辑（L1 PH non-rank-1 settle）──
                if state == St.POSITION_OPEN:
                    should_trim = False
                    if direction == 1 and l1_nr1_up:
                        ratio = _persistence_ratio(l1_up.alive, 1)
                        should_trim = ratio > 0.05
                    elif direction == -1 and l1_nr1_down:
                        ratio = _persistence_ratio(l1_down.alive, 1)
                        should_trim = ratio > 0.05

                    if should_trim and not has_active_trim:
                        trim_shares = total_shares * ratio
                        active_trim_sell_bar = i
                        active_trim_sell_price = c
                        active_trim_shares = trim_shares
                        has_active_trim = True
                        state = St.COST_REDUCING
                        states_in_trade.add(state.name)
                        if not ev:
                            ev = f"TRIM@{c:.2f} shares={trim_shares:.1f} L1ratio={ratio:.3f}"

                elif state == St.COST_REDUCING:
                    if has_active_trim:
                        # 平仓触发：L1 反向 non-rank-1 settle 或 同向 BSP
                        should_close_diff = False
                        if direction == 1 and l1_nr1_down:
                            should_close_diff = True
                        elif direction == -1 and l1_nr1_up:
                            should_close_diff = True
                        side_match_buy = buy_cands if direction == 1 else sell_cands
                        if side_match_buy:
                            should_close_diff = True

                        if should_close_diff:
                            if direction == 1:
                                profit = (active_trim_sell_price - c) * active_trim_shares
                            else:
                                profit = (c - active_trim_sell_price) * active_trim_shares
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumulative_recovered += profit
                            diff_count += 1
                            has_active_trim = False
                            ev = f"CLOSE_DIFF@{c:.2f} profit={profit:.2f}"

                            if cumulative_recovered >= own_capital:
                                state = St.PRINCIPAL_WITHDRAWN
                                states_in_trade.add(state.name)
                                ev += " → PW"
                            else:
                                state = St.POSITION_OPEN
                    else:
                        should_new_trim = False
                        if direction == 1 and l1_nr1_up:
                            ratio = _persistence_ratio(l1_up.alive, 1)
                            should_new_trim = ratio > 0.05
                        elif direction == -1 and l1_nr1_down:
                            ratio = _persistence_ratio(l1_down.alive, 1)
                            should_new_trim = ratio > 0.05

                        if should_new_trim:
                            trim_shares = total_shares * ratio
                            active_trim_sell_bar = i
                            active_trim_sell_price = c
                            active_trim_shares = trim_shares
                            has_active_trim = True
                            if not ev:
                                ev = f"NEW_TRIM@{c:.2f} shares={trim_shares:.1f}"

                elif state == St.PRINCIPAL_WITHDRAWN:
                    if not has_active_trim:
                        should_pw_trim = False
                        if direction == 1 and l1_nr1_up:
                            ratio = _persistence_ratio(l1_up.alive, 1)
                            should_pw_trim = ratio > 0.05
                        elif direction == -1 and l1_nr1_down:
                            ratio = _persistence_ratio(l1_down.alive, 1)
                            should_pw_trim = ratio > 0.05
                        if should_pw_trim:
                            active_trim_sell_bar = i
                            active_trim_sell_price = c
                            active_trim_shares = total_shares * ratio
                            has_active_trim = True
                            ev = f"PW_TRIM@{c:.2f}"
                    elif has_active_trim:
                        should_close = False
                        if direction == 1 and l1_nr1_down:
                            should_close = True
                        elif direction == -1 and l1_nr1_up:
                            should_close = True
                        side_match_buy = buy_cands if direction == 1 else sell_cands
                        if side_match_buy:
                            should_close = True
                        if should_close:
                            if direction == 1:
                                profit = (active_trim_sell_price - c) * active_trim_shares
                            else:
                                profit = (c - active_trim_sell_price) * active_trim_shares
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumulative_recovered += profit
                            diff_count += 1
                            has_active_trim = False
                            ev = f"PW_CLOSE@{c:.2f} profit={profit:.2f}"

        elif state == St.STOPPED_OUT:
            state = St.SCANNING
            ev = "AUTO_RESET"

        if ev:
            _record(i, c, ev)

        if i - last_progress >= 100000:
            print(f"    [{i/n*100:5.1f}%] bar {i:,}/{n:,}")
            last_progress = i

    if state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
        _close_trade(n - 1, closes[-1], "eod_close")

    return trades, event_records, state_counts, ph_counts


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
    L.append("# 多级别 PH 分层赋格 FSM 回测 — 1分钟数据\n")
    L.append("## 架构\n")
    L.append("5状态 FSM + 三级别 PH 分层：\n")
    L.append("| PH级别 | 输入 | 更新频率 | 信号用途 |")
    L.append("|--------|------|---------|---------|")
    L.append("| L0 | 1min close | 每bar | 进场门控（rank-1 settle） |")
    L.append("| L1 | L1线段端点价格 | ~数千次 | 降成本触发/加仓比例（non-rank-1 settle） |")
    L.append("| L2 | L1走势端点价格 | ~数百次 | 方向裁决/止损（rank-1 settle） |\n")
    L.append("**vs 旧版 H 组**：旧版只有一棵 L0 PH 树做方向裁决，方向翻转~780次；")
    L.append("新版用 L2 PH 做方向裁决，翻转频率应降至~47次量级。\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        ph = res["ph_counts"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：**{res['n_bars']:,}** bars (1min)")
        L.append(f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}")
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**")
        L.append(f"- 回测耗时：{res['elapsed']:.1f}s\n")

        L.append("### PH 分层统计\n")
        L.append("| 级别 | 更新次数 | settle总数 | rank-1 settle |")
        L.append("|------|---------|-----------|---------------|")
        L.append(f"| L0 | {res['n_bars']:,} (每bar) | {ph['l0_settles']:,} | {ph['l0_r1_settles']} |")
        L.append(f"| L1 | {ph['l1_updates']:,} | {ph['l1_settles']:,} | {ph['l1_r1_settles']} |")
        L.append(f"| L2 | {ph['l2_updates']:,} | {ph['l2_settles']:,} | {ph['l2_r1_settles']} |")
        L.append(f"| **L2 方向翻转** | — | — | **{ph['l2_direction_flips']}** |")
        L.append("")

        L.append("### 总体指标\n")
        L.append("| 指标 | 值 |")
        L.append("|------|-----|")
        L.append(f"| 交易数 | {m['n']} (多{m['long_n']}/空{m['short_n']}) |")
        L.append(f"| 胜率 | {m['win_rate']:.1f}% |")
        L.append(f"| 平均收益 | {m['avg_pnl']:+.3f}% |")
        L.append(f"| 复利累计 | {m['total_compound']:+.2f}% |")
        L.append(f"| 最大回撤 | {m['max_dd']:.2f}% |")
        L.append(f"| 平均持仓 | {m['avg_hold_bars']:,} bars |")
        L.append(f"| 有加仓的交易 | {m['n_with_add']}/{m['n']} |")
        L.append(f"| 有降成本的交易 | {m['n_with_cr']}/{m['n']} |")
        L.append(f"| 达到本金回收 | {m['n_pw']}/{m['n']} |")
        L.append("")

        L.append("### 交易明细\n")
        L.append("| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |")
        L.append("|---|------|------|------|---------|------|------|------|---------|---------|---------|")
        for idx, t in enumerate(res["trades"]):
            d_str = "多" if t.direction == 1 else "空"
            hold = t.exit_bar - t.entry_bar
            pw = "是" if t.principal_withdrawn else "否"
            states = "→".join(t.states_visited)
            L.append(f"| {idx+1} | {d_str} | {t.entry_price:.2f}@{t.entry_bar} | "
                     f"{t.exit_price:.2f}@{t.exit_bar} | {hold:,} | {t.pnl_pct:+.2f} | "
                     f"{t.n_add_positions} | {t.n_short_diffs} | {pw} | "
                     f"{t.exit_reason} | {states} |")
        L.append("")

        L.append("### FSM 状态分布（按 bar 数）\n")
        L.append("| 状态 | Bars | 占比 |")
        L.append("|------|------|------|")
        total_bars = sum(res["state_counts"].values())
        for st in ["SCANNING", "POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN", "STOPPED_OUT"]:
            cnt = res["state_counts"].get(st, 0)
            L.append(f"| {st} | {cnt:,} | {cnt/total_bars*100:.1f}% |")
        L.append("")

        evts = res["event_records"]
        if evts:
            L.append(f"<details><summary>事件流（{len(evts)}条）</summary>\n")
            L.append("| Bar | 价格 | 状态 | 事件 |")
            L.append("|-----|------|------|------|")
            for r in evts[:100]:
                L.append(f"| {r.bar_idx:,} | {r.price:.2f} | {r.state} | {r.event} |")
            if len(evts) > 100:
                L.append(f"| ... | ... | ... | （共{len(evts)}条，显示前100） |")
            L.append("</details>\n")

    L.append("## 汇总对比\n")
    L.append("| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | L2翻转 | 加仓率 | 降成本率 | PW率 | 耗时 |")
    L.append("|------|------|-----|------|------|--------|--------|--------|---------|------|------|")
    for res in all_results:
        m = res["metrics"]
        ph = res["ph_counts"]
        n = m["n"] or 1
        L.append(f"| {res['symbol']} | {res['n_bars']:,} | {res['bh']:+.1f} | "
                 f"{m['total_compound']:+.2f} | {m['win_rate']:.0f}% | {m['n']} | "
                 f"{ph['l2_direction_flips']} | "
                 f"{m['n_with_add']}/{n} | {m['n_with_cr']}/{n} | {m['n_pw']}/{n} | "
                 f"{res['elapsed']:.0f}s |")
    L.append("")

    L.append("## 结果包六要素\n")
    L.append("**结论**：多级别 PH 分层 FSM 在 1 分钟级别 3 标的上的表现。"
             "方向裁决从 L0 提升到 L2，降成本从 L0 提升到 L1。\n")
    L.append("**定义依据**：")
    L.append("- L0 PH：1min close 序列的在线因果 merge tree（§7.5）")
    L.append("- L1 PH：L1 线段端点 ep1_price 序列的 merge tree")
    L.append("- L2 PH：L1 走势端点（up→high, down→low）序列的 merge tree")
    L.append("- 方向裁决 = L2 rank-1 settle（结构级别匹配交易级别）")
    L.append("- 降成本 = L1 non-rank-1 settle（中间级别的结构信号）")
    L.append("- 进场门控 = L0 rank-1 settle + BSP candidate（微观确认）\n")
    L.append("**边界条件**：")
    L.append(f"- PENDING_EXPIRY = {PENDING_EXPIRY} bars — BSP 候选需在 L0 rank-1 settle 前出现")
    L.append("- L2 PH 更新频率取决于 L1 走势 settle 速度 — 早期数据不足时 L2 方向可能长时间未决")
    L.append("- 若 L2 方向长期未翻转 → 交易数少 → 统计不显著")
    L.append("- 无滑点/手续费建模\n")
    L.append("**下游推论**：")
    L.append("- 若 L2 翻转~47次、胜率≥57% → 分层有效，方向级别匹配假说成立")
    L.append("- 若翻转频率仍高 → L2 PH 输入不够稀疏，需更高级别输入")
    L.append("- 若交易数为0 → L2 方向长期未决 或 BSP+L0 settle 未在窗口内共现 → 调整 PENDING_EXPIRY\n")
    L.append("**谱系引用**：")
    L.append("- 267号：满仓满融降成本体系")
    L.append("- §7.5：在线因果 merge tree")
    L.append("- 526号：递归存在论区分（a0 级别分层的谱系依据）\n")
    L.append("**影响声明**：新建独立回测脚本，不修改引擎代码或旧版回测。\n")
    L.append("**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main():
    symbols = ["QQQ", "OKLO", "HK700"]
    all_results = []

    for symbol in symbols:
        print(f"\n{'='*60}")
        print(f"  {symbol} — 多级别 PH 分层 FSM 回测 (1min)")
        print(f"{'='*60}")

        try:
            opens, highs, lows, closes, dates = load_1min(symbol)
            n = len(closes)
            print(f"  数据：{n:,} bars")

            t0 = time.time()
            trades, event_records, state_counts, ph_counts = run_backtest(opens, highs, lows, closes)
            elapsed = time.time() - t0

            m = compute_metrics(trades)
            bh = (closes[-1] - closes[0]) / closes[0] * 100

            print(f"  完成：{elapsed:.1f}s ({n/elapsed:.0f} bars/s)")
            print(f"  PH 统计：L0={ph_counts['l0_r1_settles']} r1, "
                  f"L1={ph_counts['l1_updates']} updates/{ph_counts['l1_r1_settles']} r1, "
                  f"L2={ph_counts['l2_updates']} updates/{ph_counts['l2_r1_settles']} r1, "
                  f"方向翻转={ph_counts['l2_direction_flips']}")
            print(f"  交易：{m['n']}笔 (多{m['long_n']}/空{m['short_n']})")
            print(f"  胜率：{m['win_rate']:.1f}%, 复利：{m['total_compound']:+.2f}%")
            print(f"  加仓：{m['n_with_add']}/{m['n']}, "
                  f"降成本：{m['n_with_cr']}/{m['n']}, "
                  f"本金回收：{m['n_pw']}/{m['n']}")
            print(f"  BH: {bh:+.2f}%")

            all_results.append({
                "symbol": symbol,
                "n_bars": n,
                "closes": closes,
                "dates": dates,
                "bh": bh,
                "metrics": m,
                "trades": trades,
                "event_records": event_records,
                "state_counts": state_counts,
                "ph_counts": ph_counts,
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
