"""自适应多级别 PH 分层赋格 FSM 回测 — 1分钟数据版本。

核心改进（vs fugue_multilevel_ph_backtest_1min.py）：
- 方向裁决级别不再写死 L2，而是从递归结构涌现
- 最高有效 PH 级别做方向裁决，次高做进场门控，再低一级做降成本
- 每个标的的有效级别可能不同（QQQ→L4, OKLO→L3 等）

PH 级别与结构级别的映射：
  PH L0 ← 1min close（每 bar）
  PH L1 ← L1 线段端点 ep1_price（segment settle 时）
  PH L(N+1) ← 结构层级 N 走势端点（move settle 时，up→high, down→low）

信号路由（设有效级别=E）：
  方向裁决 = PH E 的 rank-1 settle
  进场门控 = PH max(E-1, 0) 的 rank-1 settle + BSP
  降成本   = PH max(E-2, 0) 的 non-rank-1 settle
  加仓比例 = PH max(E-2, 0) 的 persistence ratio

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
OUTPUT_MD = ROOT / "analysis" / "fugue_adaptive_ph_backtest_1min.md"
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100000.0
MIN_PH_UPDATES = 10
MAX_PH_LEVELS = 8


# ════════════════════════════════════════════════════════════
# PH alive 提取（O(stack)）
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
# PH rank-1 settle 检测
# ════════════════════════════════════════════════════════════

@dataclass
class PHLevelState:
    tree: OnlineMergeTree = field(default_factory=lambda: OnlineMergeTree(track_dominant=False))
    alive: _Alive = field(default_factory=lambda: _Alive(()))
    prev_r1_birth: int | None = None

    def detect_settle(self, settles: list[MergeBar]) -> tuple[bool, bool]:
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


class BarSignals:
    __slots__ = ("r1_down", "r1_up", "nr1_down", "nr1_up")
    def __init__(self):
        self.r1_down = False
        self.r1_up = False
        self.nr1_down = False
        self.nr1_up = False


# ════════════════════════════════════════════════════════════
# 自适应 PH 栈
# ════════════════════════════════════════════════════════════

class AdaptivePHStack:
    """动态管理多级别 PH 树，每层信号独立追踪。"""

    def __init__(self):
        self.ph_down: dict[int, PHLevelState] = {}
        self.ph_up: dict[int, PHLevelState] = {}
        self.update_counts: dict[int, int] = {}
        self.settle_counts: dict[int, int] = {}
        self.r1_counts: dict[int, int] = {}
        self.level_directions: dict[int, int] = {}
        self.bar_sigs: dict[int, BarSignals] = {}
        self._eff_transitions: list[tuple[int, int]] = []
        self._prev_eff: int = 0

    def _ensure(self, level: int) -> None:
        if level not in self.ph_down and level < MAX_PH_LEVELS:
            self.ph_down[level] = PHLevelState()
            self.ph_up[level] = PHLevelState()
            self.update_counts[level] = 0
            self.settle_counts[level] = 0
            self.r1_counts[level] = 0
            self.level_directions[level] = 0

    def reset_bar(self) -> None:
        self.bar_sigs = {lv: BarSignals() for lv in self.ph_down}

    def feed(self, level: int, price: float) -> None:
        self._ensure(level)
        if level not in self.bar_sigs:
            self.bar_sigs[level] = BarSignals()
        sig = self.bar_sigs[level]

        ds = self.ph_down[level].tree.update(price)
        us = self.ph_up[level].tree.update(-price)
        self.update_counts[level] += 1

        r1d, nr1d = self.ph_down[level].detect_settle(ds)
        r1u, nr1u = self.ph_up[level].detect_settle(us)

        if ds:
            self.settle_counts[level] += len(ds)
        if us:
            self.settle_counts[level] += len(us)
        if r1d:
            self.r1_counts[level] += 1
            sig.r1_down = True
            self.level_directions[level] = 1
        if r1u:
            self.r1_counts[level] += 1
            sig.r1_up = True
            self.level_directions[level] = -1
        if nr1d:
            sig.nr1_down = True
        if nr1u:
            sig.nr1_up = True

    def effective_level(self) -> int:
        best = 0
        for lv in self.ph_down:
            if (self.update_counts.get(lv, 0) >= MIN_PH_UPDATES
                    and self.r1_counts.get(lv, 0) >= 1):
                best = max(best, lv)
        return best

    def track_eff_transition(self, bar_idx: int) -> None:
        eff = self.effective_level()
        if eff != self._prev_eff:
            self._eff_transitions.append((bar_idx, eff))
            self._prev_eff = eff

    def signals_at(self, level: int) -> BarSignals:
        return self.bar_sigs.get(level, BarSignals())

    def alive_at(self, level: int) -> tuple[_Alive, _Alive]:
        if level in self.ph_down:
            return self.ph_down[level].alive, self.ph_up[level].alive
        return _Alive(()), _Alive(())

    def max_level(self) -> int:
        return max(self.ph_down.keys()) if self.ph_down else 0


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
    effective_level: int


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


def _find_move_ep(moves: list, event: MoveSettleV1) -> float | None:
    for m in moves:
        if m.seg_start == event.seg_start and m.direction == event.direction and m.settled:
            return m.high if m.direction == "up" else m.low
    return None


# ════════════════════════════════════════════════════════════
# 单 pass 自适应回测引擎
# ════════════════════════════════════════════════════════════

def run_backtest(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> tuple[list[CompletedTrade], list[EventRecord], dict[str, int], AdaptivePHStack]:
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id="bt", max_levels=6)
    base_ts = datetime(2020, 1, 1)

    ph = AdaptivePHStack()
    ph._ensure(0)
    ph._ensure(1)

    # FSM 方向（持久，由最高有效级别的 rank-1 settle 更新）
    fsm_direction = 0

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
    entry_eff_level = 0

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
        nonlocal add_count, diff_count, states_in_trade, entry_eff_level

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
            effective_level=entry_eff_level,
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
        entry_eff_level = 0

    for i in range(n):
        c = closes[i]
        bar = Bar(ts=base_ts + timedelta(minutes=i),
                  open=opens[i], high=highs[i], low=lows[i], close=c, volume=0.0)

        # ══════ 1. Incremental pipeline ══════
        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events

        # ══════ 2. PH 分层喂数据 ══════
        ph.reset_bar()

        # L0: 每 bar
        ph.feed(0, c)

        # L1: L1 线段 settle
        for e in snap.seg_snapshot.events:
            if isinstance(e, SegmentSettleV1) and e.ep1_price > 0:
                ph.feed(1, e.ep1_price)

        # L2: L1 走势 settle → PH level 2
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1):
                ep = _find_move_ep(snap.move_snapshot.moves, e)
                if ep and ep > 0:
                    ph.feed(2, ep)

        # L3+: 递归层级 K 走势 settle → PH level K+1
        for rs in snap.recursive_snapshots:
            target = rs.level_id + 1
            for e in rs.move_events:
                if isinstance(e, MoveSettleV1):
                    ep = _find_move_ep(rs.moves, e)
                    if ep and ep > 0:
                        ph.feed(target, ep)

        # ══════ 3. 有效级别 + 信号路由 ══════
        ph.track_eff_transition(i)
        eff = ph.effective_level()
        dir_lv = eff
        gate_lv = max(eff - 1, 0)
        cost_lv = max(eff - 2, 1)  # L0 太噪，降成本最低用 L1

        dir_sig = ph.signals_at(dir_lv)
        gate_sig = ph.signals_at(gate_lv)
        cost_sig = ph.signals_at(cost_lv)

        # 方向更新（仅由 direction level 的 rank-1 settle 驱动）
        dir_flip_long = False
        dir_flip_short = False
        if dir_sig.r1_down:
            if fsm_direction != 1:
                dir_flip_long = True
            fsm_direction = 1
        if dir_sig.r1_up:
            if fsm_direction != -1:
                dir_flip_short = True
            fsm_direction = -1

        # 有效级别跳升时继承新级别的已有方向
        if ph._eff_transitions and ph._eff_transitions[-1][0] == i:
            new_dir = ph.level_directions.get(eff, 0)
            if new_dir != 0 and new_dir != fsm_direction:
                if new_dir == 1 and fsm_direction != 1:
                    dir_flip_long = True
                elif new_dir == -1 and fsm_direction != -1:
                    dir_flip_short = True
                fsm_direction = new_dir

        # 进场门控信号
        gate_r1_down = gate_sig.r1_down
        gate_r1_up = gate_sig.r1_up

        # 降成本信号
        cost_nr1_down = cost_sig.nr1_down
        cost_nr1_up = cost_sig.nr1_up

        # 降成本 alive（用于 persistence ratio）
        cost_down_alive, cost_up_alive = ph.alive_at(cost_lv)

        # ══════ 4. BSP 事件提取 ══════
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

        # ══════ 5. FSM 状态机 ══════
        if state == St.SCANNING:
            if buy_cands and fsm_direction == 1:
                pending_entry = {"bar": i, "side": "buy", "bsp_ids": set(buy_cands)}
            if sell_cands and fsm_direction == -1:
                pending_entry = {"bar": i, "side": "sell", "bsp_ids": set(sell_cands)}

            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                else:
                    ps = pending_entry["side"]
                    inv_list = buy_invalidates if ps == "buy" else sell_invalidates
                    if any(bid in pending_entry["bsp_ids"] for bid in inv_list):
                        pending_entry = None

            if pending_entry and pending_entry["side"] == "buy" and gate_r1_down:
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
                entry_eff_level = eff
                ev = f"ENTRY_LONG@{c:.2f} eff=L{eff} gate=L{gate_lv}"

            elif pending_entry and pending_entry["side"] == "sell" and gate_r1_up:
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
                entry_eff_level = eff
                ev = f"ENTRY_SHORT@{c:.2f} eff=L{eff} gate=L{gate_lv}"

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            states_in_trade.add(state.name)

            should_stop = False
            stop_reason = ""

            if direction == 1 and dir_flip_short:
                should_stop = True
                stop_reason = f"dir_flip_short_L{dir_lv}"
            elif direction == -1 and dir_flip_long:
                should_stop = True
                stop_reason = f"dir_flip_long_L{dir_lv}"

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
                # ── 加仓（BSP + cost-level persistence ratio）──
                side_match_cands = buy_cands + buy_confirms if direction == 1 else sell_cands + sell_confirms
                new_bsp_ids = [bid for bid in side_match_cands if bid not in entry_bsp_ids]

                if new_bsp_ids and state in (St.POSITION_OPEN, St.COST_REDUCING):
                    alive = cost_down_alive if direction == 1 else cost_up_alive
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
                        ev = f"ADD_POS@{c:.2f} +{add_shares:.1f} ratio={ratio:.3f}"

                # ── 降成本（cost-level non-rank-1 settle）──
                if state == St.POSITION_OPEN:
                    should_trim = False
                    ratio = 0.0
                    if direction == 1 and cost_nr1_up:
                        ratio = _persistence_ratio(cost_up_alive, 1)
                        should_trim = ratio > 0.05
                    elif direction == -1 and cost_nr1_down:
                        ratio = _persistence_ratio(cost_down_alive, 1)
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
                            ev = f"TRIM@{c:.2f} shares={trim_shares:.1f} ratio={ratio:.3f}"

                elif state == St.COST_REDUCING:
                    if has_active_trim:
                        should_close_diff = False
                        if direction == 1 and cost_nr1_down:
                            should_close_diff = True
                        elif direction == -1 and cost_nr1_up:
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
                        ratio = 0.0
                        if direction == 1 and cost_nr1_up:
                            ratio = _persistence_ratio(cost_up_alive, 1)
                            should_new_trim = ratio > 0.05
                        elif direction == -1 and cost_nr1_down:
                            ratio = _persistence_ratio(cost_down_alive, 1)
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
                        ratio = 0.0
                        if direction == 1 and cost_nr1_up:
                            ratio = _persistence_ratio(cost_up_alive, 1)
                            should_pw_trim = ratio > 0.05
                        elif direction == -1 and cost_nr1_down:
                            ratio = _persistence_ratio(cost_down_alive, 1)
                            should_pw_trim = ratio > 0.05
                        if should_pw_trim:
                            active_trim_sell_bar = i
                            active_trim_sell_price = c
                            active_trim_shares = total_shares * ratio
                            has_active_trim = True
                            ev = f"PW_TRIM@{c:.2f}"
                    elif has_active_trim:
                        should_close = False
                        if direction == 1 and cost_nr1_down:
                            should_close = True
                        elif direction == -1 and cost_nr1_up:
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
            eff_now = ph.effective_level()
            print(f"    [{i/n*100:5.1f}%] bar {i:,}/{n:,}  eff=L{eff_now}")
            last_progress = i

    if state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
        _close_trade(n - 1, closes[-1], "eod_close")

    return trades, event_records, state_counts, ph


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
        eq *= max(1 + t.pnl_pct / 100, 0.01)
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
    L.append("# 自适应多级别 PH 分层赋格 FSM 回测 — 1分钟数据\n")
    L.append("## 架构\n")
    L.append("5状态 FSM + 自适应 PH 分层（方向级别从数据涌现）：\n")
    L.append("| PH级别 | 输入 | 信号用途 |")
    L.append("|--------|------|---------|")
    L.append("| L0 | 1min close | 仅作为 PH 基础数据，不用于降成本 |")
    L.append("| L1 | L1线段端点 | 门控/降成本（取决于 E） |")
    L.append("| L(N+1) | 结构层级N走势端点 | 方向/门控/降成本（取决于 E） |")
    L.append("")
    L.append("**信号路由规则**：设有效级别 E = 最高有充足数据的 PH 级别")
    L.append("- 方向裁决 = PH L(E) rank-1 settle")
    L.append("- 进场门控 = PH L(E-1) rank-1 settle + BSP")
    L.append("- 降成本 = PH L(max(E-2, 1)) non-rank-1 settle（下界 L1，L0 太噪）")
    L.append(f"- 有效阈值：≥{MIN_PH_UPDATES} 次更新 + ≥1 次 rank-1 settle\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        ph: AdaptivePHStack = res["ph"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：**{res['n_bars']:,}** bars (1min)")
        L.append(f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}")
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**")
        L.append(f"- 回测耗时：{res['elapsed']:.1f}s")
        L.append(f"- **最终有效级别：L{ph.effective_level()}**\n")

        L.append("### PH 分层统计\n")
        L.append("| PH级别 | 更新次数 | settle总数 | rank-1 settle | 有效？ |")
        L.append("|--------|---------|-----------|---------------|--------|")
        for lv in sorted(ph.ph_down.keys()):
            upd = ph.update_counts.get(lv, 0)
            stl = ph.settle_counts.get(lv, 0)
            r1 = ph.r1_counts.get(lv, 0)
            is_eff = upd >= MIN_PH_UPDATES and r1 >= 1
            eff_str = "**是**" if is_eff else "否"
            L.append(f"| L{lv} | {upd:,} | {stl:,} | {r1} | {eff_str} |")
        L.append("")

        if ph._eff_transitions:
            L.append("### 有效级别涌现历史\n")
            L.append("| Bar | 有效级别 |")
            L.append("|-----|---------|")
            for bar_idx, eff_lv in ph._eff_transitions:
                L.append(f"| {bar_idx:,} | L{eff_lv} |")
            L.append("")

        final_eff = ph.effective_level()
        L.append(f"### 信号路由（最终：方向=L{final_eff}, "
                 f"门控=L{max(final_eff-1,0)}, "
                 f"降成本=L{max(final_eff-2,0)}）\n")

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
        L.append("| # | 方向 | EffLv | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | PW | 退出原因 |")
        L.append("|---|------|-------|------|------|---------|------|------|------|-----|---------|")
        for idx, t in enumerate(res["trades"]):
            d_str = "多" if t.direction == 1 else "空"
            hold = t.exit_bar - t.entry_bar
            pw = "是" if t.principal_withdrawn else ""
            L.append(f"| {idx+1} | {d_str} | L{t.effective_level} | "
                     f"{t.entry_price:.2f}@{t.entry_bar:,} | "
                     f"{t.exit_price:.2f}@{t.exit_bar:,} | {hold:,} | {t.pnl_pct:+.2f} | "
                     f"{t.n_add_positions} | {t.n_short_diffs} | {pw} | {t.exit_reason} |")
        L.append("")

        L.append("### FSM 状态分布\n")
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
            for r in evts[:120]:
                L.append(f"| {r.bar_idx:,} | {r.price:.2f} | {r.state} | {r.event} |")
            if len(evts) > 120:
                L.append(f"| ... | ... | ... | （共{len(evts)}条，显示前120） |")
            L.append("</details>\n")

    L.append("## 汇总对比\n")
    L.append("| 标的 | Bars | EffLv | BH% | 复利% | 胜率 | 交易数 | 加仓率 | 降成本率 | PW率 | 耗时 |")
    L.append("|------|------|-------|-----|------|------|--------|--------|---------|------|------|")
    for res in all_results:
        m = res["metrics"]
        ph_s: AdaptivePHStack = res["ph"]
        n = m["n"] or 1
        L.append(f"| {res['symbol']} | {res['n_bars']:,} | L{ph_s.effective_level()} | "
                 f"{res['bh']:+.1f} | "
                 f"{m['total_compound']:+.2f} | {m['win_rate']:.0f}% | {m['n']} | "
                 f"{m['n_with_add']}/{n} | {m['n_with_cr']}/{n} | {m['n_pw']}/{n} | "
                 f"{res['elapsed']:.0f}s |")
    L.append("")

    L.append("## 结果包六要素\n")
    L.append("**结论**：自适应 PH 分层 FSM — 方向裁决级别从数据涌现。\n")
    L.append("**定义依据**：")
    L.append("- PH L0 = 1min close 的因果 merge tree")
    L.append("- PH L1 = L1 线段端点序列的 merge tree")
    L.append("- PH L(N+1) = 结构层级 N 走势端点序列的 merge tree")
    L.append(f"- 有效级别 E = 最高有 ≥{MIN_PH_UPDATES} 更新 + ≥1 rank-1 settle 的 PH 级别")
    L.append("- 信号路由：方向=E, 门控=E-1, 降成本=E-2（下界 0）\n")
    L.append("**边界条件**：")
    L.append(f"- MIN_PH_UPDATES = {MIN_PH_UPDATES}，更小 → 更早激活高级别但可能不稳定")
    L.append(f"- PENDING_EXPIRY = {PENDING_EXPIRY} bars")
    L.append("- 有效级别只会上升不会下降（结构一旦涌现不会消失）")
    L.append("- 数据前期只有低级别有效 → 行为与旧版类似\n")
    L.append("**下游推论**：")
    L.append("- 若不同标的涌现不同有效级别 → 证明自适应有意义")
    L.append("- 若有效级别越高、胜率越高 → 级别匹配假说进一步确认")
    L.append("- 若高级别标的（如 QQQ L4）比低级别标的（如 OKLO L3）表现更好 → 级别深度有价值\n")
    L.append("**谱系引用**：267号, §7.5, 526号\n")
    L.append("**影响声明**：新建独立回测脚本。RecursiveOrchestrator 使用 max_levels=6 充分递归。\n")
    L.append("**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


def main():
    symbols = ["QQQ", "OKLO", "HK700"]
    all_results = []

    for symbol in symbols:
        print(f"\n{'='*60}")
        print(f"  {symbol} — 自适应 PH 分层 FSM 回测 (1min)")
        print(f"{'='*60}")

        try:
            opens, highs, lows, closes, dates = load_1min(symbol)
            n = len(closes)
            print(f"  数据：{n:,} bars")

            t0 = time.time()
            trades, event_records, state_counts, ph = run_backtest(opens, highs, lows, closes)
            elapsed = time.time() - t0

            m = compute_metrics(trades)
            bh = (closes[-1] - closes[0]) / closes[0] * 100
            eff = ph.effective_level()

            print(f"  完成：{elapsed:.1f}s ({n/elapsed:.0f} bars/s)")
            print(f"  有效级别：L{eff}")
            print(f"  PH 级别涌现：", end="")
            for lv in sorted(ph.ph_down.keys()):
                upd = ph.update_counts.get(lv, 0)
                r1 = ph.r1_counts.get(lv, 0)
                is_eff = upd >= MIN_PH_UPDATES and r1 >= 1
                mark = "*" if is_eff else ""
                print(f"L{lv}({upd}/{r1}){mark} ", end="")
            print()
            if ph._eff_transitions:
                print(f"  涌现历史：" + " → ".join(
                    f"L{lv}@bar{b:,}" for b, lv in ph._eff_transitions))
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
                "ph": ph,
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
