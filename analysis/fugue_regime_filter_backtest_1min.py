"""多级别 PH 分层赋格 FSM 回测 + Regime Filter — 1分钟数据版本。

在 fugue_multilevel_ph_backtest_1min.py 基础上增加 regime filter：
最高有效级别的走势方向（中枢序列 ZG/ZD 单调性）决定允许的操作方向。

- regime=上升（ZG 依次递增）→ 只做多，卖点仅减仓
- regime=下降（ZD 依次递降）→ 只做空，买点仅减仓
- regime=盘整 → 双向响应

已持仓状态不受 regime 影响（已有仓位按原逻辑管理）。

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
from typing import Sequence

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import (  # noqa: E402
    MergeBar,
    OnlineMergeTree,
)
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_regime_filter_backtest_1min.md"
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100000.0


# ════════════════════════════════════════════════════════════
# Regime 判断
# ════════════════════════════════════════════════════════════

class Regime(Enum):
    UP = auto()
    DOWN = auto()
    NEUTRAL = auto()


def determine_regime(snap: RecursiveOrchestratorSnapshot) -> tuple[Regime, int]:
    """从快照中提取最高有效级别的中枢序列，判断 regime。

    Returns: (regime, level_used)
        level_used: 实际使用的级别编号（0=无有效级别）
    """
    # 从最高递归层往下扫，找有 ≥2 中枢的层级
    for level_idx in range(len(snap.recursive_snapshots) - 1, -1, -1):
        rs = snap.recursive_snapshots[level_idx]
        if len(rs.zhongshus) >= 2:
            return _regime_from_level_zhongshus(rs.zhongshus), level_idx + 2

    # level 1 中枢
    if len(snap.zs_snapshot.zhongshus) >= 2:
        return _regime_from_zhongshus(snap.zs_snapshot.zhongshus), 1

    return Regime.NEUTRAL, 0


def _regime_from_zhongshus(zs_list: Sequence[Zhongshu]) -> Regime:
    """level 1 中枢序列的 regime 判断。"""
    if len(zs_list) < 2:
        return Regime.NEUTRAL
    zgs = [z.zg for z in zs_list]
    zds = [z.zd for z in zs_list]
    if _is_monotone_increasing(zgs):
        return Regime.UP
    if _is_monotone_decreasing(zds):
        return Regime.DOWN
    return Regime.NEUTRAL


def _regime_from_level_zhongshus(zs_list: Sequence[LevelZhongshu]) -> Regime:
    """level 2+ 中枢序列的 regime 判断。"""
    if len(zs_list) < 2:
        return Regime.NEUTRAL
    zgs = [z.zg for z in zs_list]
    zds = [z.zd for z in zs_list]
    if _is_monotone_increasing(zgs):
        return Regime.UP
    if _is_monotone_decreasing(zds):
        return Regime.DOWN
    return Regime.NEUTRAL


def _is_monotone_increasing(vals: list[float]) -> bool:
    for k in range(1, len(vals)):
        if vals[k] <= vals[k - 1]:
            return False
    return True


def _is_monotone_decreasing(vals: list[float]) -> bool:
    for k in range(1, len(vals)):
        if vals[k] >= vals[k - 1]:
            return False
    return True


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
    regime_at_entry: str = ""


@dataclass
class EventRecord:
    bar_idx: int
    price: float
    state: str
    event: str
    regime: str = ""


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
# 单 pass 多级别回测引擎（带 regime filter）
# ════════════════════════════════════════════════════════════

def run_backtest(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> tuple[list[CompletedTrade], list[EventRecord], dict[str, int], dict[str, int], dict[str, int]]:
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

    # ── Regime 统计 ──
    regime_counts: dict[str, int] = {"UP": 0, "DOWN": 0, "NEUTRAL": 0, "filtered_buy": 0, "filtered_sell": 0}

    # ── L2 方向状态（持久） ──
    l2_direction = 0

    # ── Regime 状态 ──
    current_regime = Regime.NEUTRAL
    regime_level = 0

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
    entry_regime = ""

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
        event_records.append(EventRecord(bar_idx, price, state.name, ev, current_regime.name))

    def _close_trade(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, direction, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumulative_recovered, own_capital
        nonlocal has_active_trim, pending_entry, entry_bsp_ids
        nonlocal add_count, diff_count, states_in_trade, entry_regime

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
            regime_at_entry=entry_regime,
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
        entry_regime = ""

    for i in range(n):
        c = closes[i]
        bar = Bar(ts=base_ts + timedelta(minutes=i),
                  open=opens[i], high=highs[i], low=lows[i], close=c, volume=0.0)

        # ══════ 1. Incremental BSP pipeline ══════
        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events

        # ══════ 1.5 Regime 判断 ══════
        current_regime, regime_level = determine_regime(snap)
        regime_counts[current_regime.name] += 1

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

        # ══════ 3. L1 PH ══════
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

        # ══════ 4. L2 PH ══════
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

        # ══════ 6. FSM 状态机（带 regime filter）══════
        if state == St.SCANNING:
            # ── Regime filter：SCANNING 阶段只响应匹配 regime 的候选 ──
            filtered_buy_cands = buy_cands
            filtered_sell_cands = sell_cands

            if current_regime == Regime.UP:
                if sell_cands:
                    regime_counts["filtered_sell"] += len(sell_cands)
                filtered_sell_cands = []
            elif current_regime == Regime.DOWN:
                if buy_cands:
                    regime_counts["filtered_buy"] += len(buy_cands)
                filtered_buy_cands = []
            # NEUTRAL → 双向都保留

            # BSP candidates 要求同时匹配 L2 方向
            if filtered_buy_cands and l2_direction == 1:
                pending_entry = {"bar": i, "side": "buy", "bsp_ids": set(filtered_buy_cands)}
            if filtered_sell_cands and l2_direction == -1:
                pending_entry = {"bar": i, "side": "sell", "bsp_ids": set(filtered_sell_cands)}

            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                else:
                    ps = pending_entry["side"]
                    inv_list = buy_invalidates if ps == "buy" else sell_invalidates
                    if any(bid in pending_entry["bsp_ids"] for bid in inv_list):
                        pending_entry = None

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
                entry_regime = current_regime.name
                ev = f"ENTRY_LONG@{c:.2f} L2dir={l2_direction} regime={current_regime.name}"

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
                entry_regime = current_regime.name
                ev = f"ENTRY_SHORT@{c:.2f} L2dir={l2_direction} regime={current_regime.name}"

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            states_in_trade.add(state.name)

            should_stop = False
            stop_reason = ""

            if direction == 1 and l2_flip_short:
                should_stop = True
                stop_reason = "l2_direction_flip_short"
            elif direction == -1 and l2_flip_long:
                should_stop = True
                stop_reason = "l2_direction_flip_long"

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

    return trades, event_records, state_counts, ph_counts, regime_counts


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
    L.append("# 多级别 PH 分层赋格 FSM 回测 + Regime Filter — 1分钟数据\n")
    L.append("## 架构变更\n")
    L.append("在 `fugue_multilevel_ph_backtest_1min.py` 基础上增加 **Regime Filter**：\n")
    L.append("| Regime | 判断方法 | 允许操作 |")
    L.append("|--------|---------|---------|")
    L.append("| UP | 最高有效级别 ≥2 中枢且 ZG 依次递增 | 仅做多 |")
    L.append("| DOWN | 最高有效级别 ≥2 中枢且 ZD 依次递降 | 仅做空 |")
    L.append("| NEUTRAL | 其他 | 双向 |\n")
    L.append("Regime 仅影响 SCANNING 阶段的候选过滤。已持仓按原逻辑管理。\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        ph = res["ph_counts"]
        rc = res["regime_counts"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：**{res['n_bars']:,}** bars (1min)")
        L.append(f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}")
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**")
        L.append(f"- 回测耗时：{res['elapsed']:.1f}s\n")

        L.append("### Regime 分布\n")
        total_bars = res["n_bars"]
        L.append("| Regime | Bars | 占比 | 过滤的候选数 |")
        L.append("|--------|------|------|-------------|")
        for rg in ["UP", "DOWN", "NEUTRAL"]:
            cnt = rc.get(rg, 0)
            L.append(f"| {rg} | {cnt:,} | {cnt/total_bars*100:.1f}% | — |")
        L.append(f"| 被过滤的 buy | — | — | {rc.get('filtered_buy', 0)} |")
        L.append(f"| 被过滤的 sell | — | — | {rc.get('filtered_sell', 0)} |")
        L.append("")

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
        L.append("| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | PW | 退出原因 | Regime | 状态路径 |")
        L.append("|---|------|------|------|---------|------|------|------|-----|---------|--------|---------|")
        for idx, t in enumerate(res["trades"]):
            d_str = "多" if t.direction == 1 else "空"
            hold = t.exit_bar - t.entry_bar
            pw = "是" if t.principal_withdrawn else "否"
            states = "→".join(t.states_visited)
            L.append(f"| {idx+1} | {d_str} | {t.entry_price:.2f}@{t.entry_bar} | "
                     f"{t.exit_price:.2f}@{t.exit_bar} | {hold:,} | {t.pnl_pct:+.2f} | "
                     f"{t.n_add_positions} | {t.n_short_diffs} | {pw} | "
                     f"{t.exit_reason} | {t.regime_at_entry} | {states} |")
        L.append("")

        L.append("### FSM 状态分布（按 bar 数）\n")
        L.append("| 状态 | Bars | 占比 |")
        L.append("|------|------|------|")
        total_st = sum(res["state_counts"].values())
        for st in ["SCANNING", "POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN", "STOPPED_OUT"]:
            cnt = res["state_counts"].get(st, 0)
            L.append(f"| {st} | {cnt:,} | {cnt/total_st*100:.1f}% |")
        L.append("")

        evts = res["event_records"]
        if evts:
            L.append(f"<details><summary>事件流（{len(evts)}条）</summary>\n")
            L.append("| Bar | 价格 | 状态 | 事件 | Regime |")
            L.append("|-----|------|------|------|--------|")
            for r in evts[:100]:
                L.append(f"| {r.bar_idx:,} | {r.price:.2f} | {r.state} | {r.event} | {r.regime} |")
            if len(evts) > 100:
                L.append(f"| ... | ... | ... | （共{len(evts)}条，显示前100） | ... |")
            L.append("</details>\n")

    # ── 汇总对比表（本版本内部） ──
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

    # ── vs 无 regime filter 版本对比 ──
    L.append("## Regime Filter 效果对比\n")
    L.append("与 `fugue_multilevel_ph_backtest_1min.py`（无 regime filter）的对比：\n")
    L.append("| 标的 | 指标 | 无Regime | 有Regime | 差值 |")
    L.append("|------|------|---------|---------|------|")
    L.append("| — | — | 需先运行无 regime 版本对比 | — | — |")
    L.append("")
    L.append("*注：手动填入或使用对比脚本。*\n")

    L.append("## 结果包六要素\n")
    L.append("**结论**：在多级别 PH 分层 FSM 基础上增加 regime filter，"
             "利用最高有效级别中枢序列的 ZG/ZD 单调性约束操作方向。\n")
    L.append("**定义依据**：")
    L.append("- Regime 判断 = 最高有效级别（≥2 中枢的最高递归层）的中枢序列 ZG/ZD 单调性")
    L.append("- ZG 依次递增 = 上升趋势（中枢抬高）→ 只做多")
    L.append("- ZD 依次递降 = 下降趋势（中枢降低）→ 只做空")
    L.append("- 缠论原文：大级别走势类型方向约束操作方向\n")
    L.append("**边界条件**：")
    L.append("- 若最高级别始终 <2 中枢 → regime 始终 NEUTRAL → 退化为无 filter 版本")
    L.append("- 单调性检查严格（任一对不满足即 NEUTRAL）→ 震荡市大部分时间 NEUTRAL")
    L.append("- regime 判断滞后：中枢形成需要时间，趋势初期可能仍为 NEUTRAL\n")
    L.append("**下游推论**：")
    L.append("- 若 regime filter 减少亏损交易数 → 有效过滤了逆势交易")
    L.append("- 若胜率提升但交易数大幅减少 → filter 过于保守，需放宽条件")
    L.append("- 若无改善 → 中枢单调性不是有效的趋势判据，需其他方法\n")
    L.append("**谱系引用**：")
    L.append("- 267号：满仓满融降成本体系")
    L.append("- §7.5：在线因果 merge tree")
    L.append("- 526号：递归存在论区分\n")
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
        print(f"  {symbol} — 多级别 PH 分层 FSM + Regime Filter 回测 (1min)")
        print(f"{'='*60}")

        try:
            opens, highs, lows, closes, dates = load_1min(symbol)
            n = len(closes)
            print(f"  数据：{n:,} bars")

            t0 = time.time()
            trades, event_records, state_counts, ph_counts, regime_counts = run_backtest(
                opens, highs, lows, closes
            )
            elapsed = time.time() - t0

            m = compute_metrics(trades)
            bh = (closes[-1] - closes[0]) / closes[0] * 100

            print(f"  完成：{elapsed:.1f}s ({n/elapsed:.0f} bars/s)")
            print(f"  Regime 分布：UP={regime_counts['UP']:,} DOWN={regime_counts['DOWN']:,} "
                  f"NEUTRAL={regime_counts['NEUTRAL']:,}")
            print(f"  被过滤：buy={regime_counts['filtered_buy']} sell={regime_counts['filtered_sell']}")
            print(f"  PH 统计：L0={ph_counts['l0_r1_settles']} r1, "
                  f"L1={ph_counts['l1_updates']} updates/{ph_counts['l1_r1_settles']} r1, "
                  f"L2={ph_counts['l2_updates']} updates/{ph_counts['l2_r1_settles']} r1, "
                  f"方向翻转={ph_counts['l2_direction_flips']}")
            print(f"  交易：{m['n']}笔 (多{m['long_n']}/空{m['short_n']})")
            print(f"  胜率：{m['win_rate']:.1f}%, 复利：{m['total_compound']:+.2f}%")
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
                "regime_counts": regime_counts,
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
