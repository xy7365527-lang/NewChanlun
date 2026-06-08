"""完整版四操作分离 + 多级别 PH 多重赋格回测 — 1分钟数据。

核心改进（vs fugue_multilevel_ph_backtest_1min.py）：
- 6状态 FSM：SCANNING → LONG_OPEN/LONG_COST_REDUCING → OBSERVING ←→ SHORT_OPEN/SHORT_COST_REDUCING
- 四操作独立判断：开多/平多/开空/平空 不是镜像对称
- 平仓后进入 OBSERVING，等待新 L2 PH 方向 settle 才能重新进场
- 多空各有独立的降成本/本金回收路径

信号消费映射（不变）：
| 信号         | PH级别 | 触发条件              |
|-------------|--------|----------------------|
| 方向裁决     | L2     | rank-1 settle        |
| 进场门控     | L0     | rank-1 settle + BSP  |
| 降成本触发   | L1     | non-rank-1 settle    |
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
OUTPUT_MD = ROOT / "analysis" / "fugue_full_fouroperation_backtest_1min.md"
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


# ════════════════════════════════════════════════════════════
# FSM 6状态
# ════════════════════════════════════════════════════════════

class St(Enum):
    SCANNING = auto()
    LONG_OPEN = auto()
    LONG_COST_REDUCING = auto()
    SHORT_OPEN = auto()
    SHORT_COST_REDUCING = auto()
    OBSERVING = auto()


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
# 单 pass 四操作分离回测引擎
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

    ph_counts: dict[str, int] = {
        "l0_settles": 0, "l0_r1_settles": 0,
        "l1_updates": 0, "l1_settles": 0, "l1_r1_settles": 0,
        "l2_updates": 0, "l2_settles": 0, "l2_r1_settles": 0,
        "l2_direction_flips": 0,
    }

    # ── L2 方向状态 ──
    l2_direction = 0  # 0=未决, 1=多, -1=空
    l2_direction_bar = -1  # L2 方向确认的 bar

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

    # OBSERVING 状态追踪
    observing_since_bar = -1
    observing_l2_dir_at_entry = 0  # 进入 OBSERVING 时的 L2 方向

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
        nonlocal observing_since_bar, observing_l2_dir_at_entry

        pnl_pct = (price - cost_basis) / entry_price * 100 * direction
        was_pw = St.LONG_COST_REDUCING.name in states_in_trade or St.SHORT_COST_REDUCING.name in states_in_trade
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price, direction=direction,
            pnl_pct=round(pnl_pct, 4),
            states_visited=sorted(states_in_trade),
            n_add_positions=add_count,
            n_short_diffs=diff_count,
            cumulative_recovered=cumulative_recovered,
            principal_withdrawn=(cumulative_recovered >= own_capital),
            exit_reason=reason, cost_basis_at_exit=cost_basis,
        ))
        remaining = INITIAL_CAPITAL * (1 + pnl_pct / 100)

        # 进入 OBSERVING 而非 SCANNING
        observing_since_bar = bar_idx
        observing_l2_dir_at_entry = l2_direction
        state = St.OBSERVING

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
        prev_l2 = l2_direction
        if l2_flip_long:
            if l2_direction != 1:
                ph_counts["l2_direction_flips"] += 1
            l2_direction = 1
            l2_direction_bar = i
        if l2_flip_short:
            if l2_direction != -1:
                ph_counts["l2_direction_flips"] += 1
            l2_direction = -1
            l2_direction_bar = i

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

        # ══════ 6. FSM 四操作分离状态机 ══════

        if state == St.SCANNING:
            # SCANNING：首次进场，L2方向必须已确认
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

            if pending_entry and pending_entry["side"] == "buy" and l0_r1_down:
                state = St.LONG_OPEN
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
                states_in_trade = {St.LONG_OPEN.name}
                ev = f"ENTRY_LONG@{c:.2f} L2dir={l2_direction}"

            elif pending_entry and pending_entry["side"] == "sell" and l0_r1_up:
                state = St.SHORT_OPEN
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
                states_in_trade = {St.SHORT_OPEN.name}
                ev = f"ENTRY_SHORT@{c:.2f} L2dir={l2_direction}"

        elif state == St.OBSERVING:
            # OBSERVING：等待新的 L2 方向 settle 信号
            # 只有在 L2 方向发生了新的 settle（与进入 OBSERVING 时不同）才转为 SCANNING
            new_l2_signal = False
            if l2_flip_long and l2_direction != observing_l2_dir_at_entry:
                new_l2_signal = True
            if l2_flip_short and l2_direction != observing_l2_dir_at_entry:
                new_l2_signal = True

            if new_l2_signal:
                state = St.SCANNING
                pending_entry = None
                ev = f"OBSERVING→SCANNING L2dir={l2_direction} (was {observing_l2_dir_at_entry})"
            else:
                # 即使没有新方向翻转，也收集 BSP candidates 为下次准备
                if buy_cands and l2_direction == 1 and l2_direction != observing_l2_dir_at_entry:
                    pending_entry = {"bar": i, "side": "buy", "bsp_ids": set(buy_cands)}
                if sell_cands and l2_direction == -1 and l2_direction != observing_l2_dir_at_entry:
                    pending_entry = {"bar": i, "side": "sell", "bsp_ids": set(sell_cands)}

        # ── 多头持仓状态 ──
        elif state in (St.LONG_OPEN, St.LONG_COST_REDUCING):
            states_in_trade.add(state.name)

            should_close = False
            close_reason = ""

            # 平多条件1：卖点 candidate 出现
            if sell_cands:
                should_close = True
                close_reason = "sell_candidate"

            # 平多条件2：L2 PH方向翻转为空
            if not should_close and l2_flip_short:
                should_close = True
                close_reason = "l2_direction_flip_short"

            # 平多条件3：BSP invalidate
            if not should_close:
                if any(bid in entry_bsp_ids for bid in buy_invalidates):
                    should_close = True
                    close_reason = "bsp_invalidate"

            if should_close:
                pnl = (c - cost_basis) / entry_price * 100
                ev = f"CLOSE_LONG:{close_reason}@{c:.2f} pnl={pnl:+.2f}%"
                _close_trade(i, c, close_reason)
            else:
                # ── 加仓逻辑 ──
                new_buy_ids = [bid for bid in (buy_cands + buy_confirms) if bid not in entry_bsp_ids]
                if new_buy_ids and state == St.LONG_OPEN:
                    alive = l1_down.alive
                    ratio = _persistence_ratio(alive, 1)
                    if ratio > 0.05:
                        add_shares = total_shares * ratio
                        old_val = total_shares * cost_basis
                        new_total = total_shares + add_shares
                        cost_basis = (old_val + add_shares * c) / new_total
                        total_shares = new_total
                        total_invested += add_shares * c
                        add_count += 1
                        entry_bsp_ids.update(new_buy_ids)
                        ev = f"LONG_ADD@{c:.2f} +{add_shares:.1f} L1ratio={ratio:.3f}"

                # ── 降成本逻辑（L1 反向 non-rank-1 settle → 减仓） ──
                if state == St.LONG_OPEN:
                    if l1_nr1_up:
                        ratio = _persistence_ratio(l1_up.alive, 1)
                        if ratio > 0.05 and not has_active_trim:
                            trim_shares = total_shares * ratio
                            active_trim_sell_bar = i
                            active_trim_sell_price = c
                            active_trim_shares = trim_shares
                            has_active_trim = True
                            state = St.LONG_COST_REDUCING
                            states_in_trade.add(state.name)
                            if not ev:
                                ev = f"LONG_TRIM@{c:.2f} shares={trim_shares:.1f} L1ratio={ratio:.3f}"

                elif state == St.LONG_COST_REDUCING:
                    if has_active_trim:
                        should_close_diff = l1_nr1_down or bool(buy_cands)
                        if should_close_diff:
                            profit = (active_trim_sell_price - c) * active_trim_shares
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumulative_recovered += profit
                            diff_count += 1
                            has_active_trim = False
                            ev = f"LONG_CLOSE_DIFF@{c:.2f} profit={profit:.2f}"
                            if cumulative_recovered >= own_capital:
                                ev += " → PW"
                            state = St.LONG_OPEN
                    else:
                        if l1_nr1_up:
                            ratio = _persistence_ratio(l1_up.alive, 1)
                            if ratio > 0.05:
                                active_trim_sell_bar = i
                                active_trim_sell_price = c
                                active_trim_shares = total_shares * ratio
                                has_active_trim = True
                                if not ev:
                                    ev = f"LONG_NEW_TRIM@{c:.2f} shares={active_trim_shares:.1f}"

        # ── 空头持仓状态 ──
        elif state in (St.SHORT_OPEN, St.SHORT_COST_REDUCING):
            states_in_trade.add(state.name)

            should_close = False
            close_reason = ""

            # 平空条件1：买点 candidate 出现
            if buy_cands:
                should_close = True
                close_reason = "buy_candidate"

            # 平空条件2：L2 PH方向翻转为多
            if not should_close and l2_flip_long:
                should_close = True
                close_reason = "l2_direction_flip_long"

            # 平空条件3：BSP invalidate
            if not should_close:
                if any(bid in entry_bsp_ids for bid in sell_invalidates):
                    should_close = True
                    close_reason = "bsp_invalidate"

            if should_close:
                pnl = (cost_basis - c) / entry_price * 100
                ev = f"CLOSE_SHORT:{close_reason}@{c:.2f} pnl={pnl:+.2f}%"
                _close_trade(i, c, close_reason)
            else:
                # ── 加仓逻辑（空头加仓用卖点） ──
                new_sell_ids = [bid for bid in (sell_cands + sell_confirms) if bid not in entry_bsp_ids]
                if new_sell_ids and state == St.SHORT_OPEN:
                    alive = l1_up.alive
                    ratio = _persistence_ratio(alive, 1)
                    if ratio > 0.05:
                        add_shares = total_shares * ratio
                        old_val = total_shares * cost_basis
                        new_total = total_shares + add_shares
                        cost_basis = (old_val + add_shares * c) / new_total
                        total_shares = new_total
                        total_invested += add_shares * c
                        add_count += 1
                        entry_bsp_ids.update(new_sell_ids)
                        ev = f"SHORT_ADD@{c:.2f} +{add_shares:.1f} L1ratio={ratio:.3f}"

                # ── 降成本逻辑（空头：L1 反向=down non-rank-1 settle → 减仓） ──
                if state == St.SHORT_OPEN:
                    if l1_nr1_down:
                        ratio = _persistence_ratio(l1_down.alive, 1)
                        if ratio > 0.05 and not has_active_trim:
                            trim_shares = total_shares * ratio
                            active_trim_sell_bar = i
                            active_trim_sell_price = c
                            active_trim_shares = trim_shares
                            has_active_trim = True
                            state = St.SHORT_COST_REDUCING
                            states_in_trade.add(state.name)
                            if not ev:
                                ev = f"SHORT_TRIM@{c:.2f} shares={trim_shares:.1f} L1ratio={ratio:.3f}"

                elif state == St.SHORT_COST_REDUCING:
                    if has_active_trim:
                        should_close_diff = l1_nr1_up or bool(sell_cands)
                        if should_close_diff:
                            profit = (c - active_trim_sell_price) * active_trim_shares
                            if total_shares > 0:
                                cost_basis += profit / total_shares
                                cumulative_recovered += profit
                            diff_count += 1
                            has_active_trim = False
                            ev = f"SHORT_CLOSE_DIFF@{c:.2f} profit={profit:.2f}"
                            if cumulative_recovered >= own_capital:
                                ev += " → PW"
                            state = St.SHORT_OPEN
                    else:
                        if l1_nr1_down:
                            ratio = _persistence_ratio(l1_down.alive, 1)
                            if ratio > 0.05:
                                active_trim_sell_bar = i
                                active_trim_sell_price = c
                                active_trim_shares = total_shares * ratio
                                has_active_trim = True
                                if not ev:
                                    ev = f"SHORT_NEW_TRIM@{c:.2f} shares={active_trim_shares:.1f}"

        if ev:
            _record(i, c, ev)

        if i - last_progress >= 100000:
            print(f"    [{i/n*100:5.1f}%] bar {i:,}/{n:,}")
            last_progress = i

    # EOD 平仓
    if state in (St.LONG_OPEN, St.LONG_COST_REDUCING):
        _close_trade(n - 1, closes[-1], "eod_close")
    elif state in (St.SHORT_OPEN, St.SHORT_COST_REDUCING):
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
    L.append("# 完整版四操作分离 + 多级别 PH 多重赋格回测 — 1分钟数据\n")
    L.append("## 架构\n")
    L.append("6状态 FSM + 三级别 PH 分层 + 四操作独立判断：\n")
    L.append("```")
    L.append("SCANNING ──BSP+L0settle──→ LONG_OPEN ←→ LONG_COST_REDUCING")
    L.append("    │                         │")
    L.append("    │                    平多(卖点/L2翻转/invalidate)")
    L.append("    │                         │")
    L.append("    │                         ↓")
    L.append("    │                     OBSERVING ←── 平空")
    L.append("    │                         │            │")
    L.append("    │                    新L2 settle       │")
    L.append("    │                         │            │")
    L.append("    │                         ↓            │")
    L.append("    └──BSP+L0settle──→ SHORT_OPEN ←→ SHORT_COST_REDUCING")
    L.append("```\n")
    L.append("### 四操作独立判断\n")
    L.append("| 操作 | 条件 |")
    L.append("|------|------|")
    L.append("| 开多 | L2方向=多 + 买点candidate + L0 rank-1 settle |")
    L.append("| 平多 | 卖点candidate / L2翻空 / BSP invalidate → OBSERVING |")
    L.append("| 开空 | L2方向=空 + 卖点candidate + L0 rank-1 settle |")
    L.append("| 平空 | 买点candidate / L2翻多 / BSP invalidate → OBSERVING |\n")
    L.append("**关键区别**：平仓后进入 OBSERVING，等待新 L2 PH settle 确认后才可重新进场。不立刻反手。\n")

    L.append("| PH级别 | 输入 | 更新频率 | 信号用途 |")
    L.append("|--------|------|---------|---------|")
    L.append("| L0 | 1min close | 每bar | 进场门控（rank-1 settle） |")
    L.append("| L1 | L1线段端点价格 | ~数千次 | 降成本触发/加仓比例（non-rank-1 settle） |")
    L.append("| L2 | L1走势端点价格 | ~数百次 | 方向裁决/止损（rank-1 settle） |\n")

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
        L.append(f"| 复利累计 | **{m['total_compound']:+.2f}%** |")
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
        for st in ["SCANNING", "LONG_OPEN", "LONG_COST_REDUCING", "SHORT_OPEN", "SHORT_COST_REDUCING", "OBSERVING"]:
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
    L.append("| 标的 | Bars | BH% | 复利% | 胜率 | 交易数(多/空) | L2翻转 | 加仓率 | 降成本率 | PW率 | 耗时 |")
    L.append("|------|------|-----|------|------|-------------|--------|--------|---------|------|------|")
    for res in all_results:
        m = res["metrics"]
        ph = res["ph_counts"]
        n = m["n"] or 1
        L.append(f"| {res['symbol']} | {res['n_bars']:,} | {res['bh']:+.1f} | "
                 f"**{m['total_compound']:+.2f}** | {m['win_rate']:.0f}% | "
                 f"{m['n']}({m['long_n']}/{m['short_n']}) | "
                 f"{ph['l2_direction_flips']} | "
                 f"{m['n_with_add']}/{n} | {m['n_with_cr']}/{n} | {m['n_pw']}/{n} | "
                 f"{res['elapsed']:.0f}s |")
    L.append("")

    L.append("## 六版横向对比\n")
    L.append("| 版本 | QQQ | OKLO | HK700 |")
    L.append("|------|-----|------|-------|")
    L.append("| 1. 单层PH（简单镜像） | -19.8% | -99% | -84% |")
    L.append("| 2. 多级别PH双向（简单镜像） | +58.3% | +72.3% | +222.8% |")
    L.append("| 3. 纯做多 | +71.7% | +228% | +292.9% |")
    for res in all_results:
        m = res["metrics"]
        if res["symbol"] == "QQQ":
            qqq = f"{m['total_compound']:+.1f}%"
        elif res["symbol"] == "OKLO":
            oklo = f"{m['total_compound']:+.1f}%"
        elif res["symbol"] == "HK700":
            hk700 = f"{m['total_compound']:+.1f}%"
    L.append(f"| **4. 完整版四操作分离（本版）** | **{qqq}** | **{oklo}** | **{hk700}** |")
    L.append("")

    L.append("## 结果包六要素\n")
    L.append("**结论**：完整版四操作分离 FSM 在 1 分钟级别 3 标的上的表现。"
             "核心改进：平仓后进入 OBSERVING 而非立刻反手，四操作独立判断。\n")
    L.append("**定义依据**：")
    L.append("- L0 PH：1min close 序列的在线因果 merge tree（§7.5）")
    L.append("- L1 PH：L1 线段端点 ep1_price 序列的 merge tree")
    L.append("- L2 PH：L1 走势端点（up→high, down→low）序列的 merge tree")
    L.append("- 方向裁决 = L2 rank-1 settle（结构级别匹配交易级别）")
    L.append("- 降成本 = L1 non-rank-1 settle（中间级别的结构信号）")
    L.append("- 进场门控 = L0 rank-1 settle + BSP candidate（微观确认）")
    L.append("- **四操作分离**：开多/平多/开空/平空各有独立判断条件，平仓→OBSERVING→新L2 settle→重新进场\n")
    L.append("**边界条件**：")
    L.append(f"- PENDING_EXPIRY = {PENDING_EXPIRY} bars — BSP 候选需在 L0 rank-1 settle 前出现")
    L.append("- OBSERVING 退出条件：L2 方向发生了与进入时不同的 settle → 回到 SCANNING")
    L.append("- 若 L2 方向长期不翻转 → OBSERVING 时间长 → 错过同方向机会（设计选择，不是 bug）")
    L.append("- 无滑点/手续费建模\n")
    L.append("**下游推论**：")
    L.append("- 若本版 > 简单镜像版 → OBSERVING 过滤有效，机械反手损耗可观")
    L.append("- 若本版 < 简单镜像版 → OBSERVING 过于保守，错过的同方向机会 > 避免的反手损耗")
    L.append("- 若本版 ≈ 纯做多版 → 空头操作在当前标的上无显著 alpha\n")
    L.append("**谱系引用**：")
    L.append("- 267号：满仓满融降成本体系")
    L.append("- §7.5：在线因果 merge tree")
    L.append("- 526号：递归存在论区分（a0 级别分层的谱系依据）\n")
    L.append("**影响声明**：新建独立回测脚本 `fugue_full_fouroperation_backtest_1min.py`，不修改引擎代码或旧版回测。\n")
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
        print(f"  {symbol} — 完整版四操作分离回测 (1min)")
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
            print(f"  FSM: " + " | ".join(f"{k}={v:,}" for k, v in state_counts.items() if v > 0))

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
