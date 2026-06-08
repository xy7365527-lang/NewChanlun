"""两层分离回测 — 操盘层（走势类型区分策略）+ 选股层（最高级别方向代理）。

操盘层：根据交易级别（L1）的当前 Move 类型决定操作方向：
  - trend + up → 只做多（卖点仅减仓）
  - trend + down → 只做空（买点仅减仓）
  - consolidation → 双向短差，仓位减半

选股层：最高有效递归级别的当前 Move 方向作为品种方向代理：
  - 最高级别 Move 方向 up → 品种标记 long
  - 最高级别 Move 方向 down → 品种标记 short
  - 未决/盘整 → neutral

两层组合：入场需两层同意，矛盾时不操作。

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
from typing import Literal

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
OUTPUT_MD = ROOT / "analysis" / "fugue_twolayer_backtest_1min.md"
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100000.0


# ════════════════════════════════════════════════════════════
# PH alive 提取（复用上版）
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
# 两层状态定义
# ════════════════════════════════════════════════════════════

class TradingLayerMode(Enum):
    """操盘层模式 — 由当前交易级别 Move 类型决定。"""
    UNKNOWN = auto()
    LONG_ONLY = auto()      # trend + up
    SHORT_ONLY = auto()     # trend + down
    BOTH_HALF = auto()      # consolidation

class StockLayerBias(Enum):
    """选股层偏向 — 由最高有效递归级别 Move 方向决定。"""
    NEUTRAL = auto()
    LONG = auto()
    SHORT = auto()


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
    trading_mode: str
    stock_bias: str


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
# 两层状态提取
# ════════════════════════════════════════════════════════════

def _extract_trading_layer(moves: list) -> tuple[TradingLayerMode, str]:
    """从 L1 Move 列表提取操盘层模式。

    取最后一个 Move（当前所处走势）的 kind + direction。
    """
    if not moves:
        return TradingLayerMode.UNKNOWN, ""
    last_move = moves[-1]
    if last_move.kind == "trend":
        if last_move.direction == "up":
            return TradingLayerMode.LONG_ONLY, "trend_up"
        return TradingLayerMode.SHORT_ONLY, "trend_down"
    return TradingLayerMode.BOTH_HALF, f"consolidation_{last_move.direction}"


def _extract_stock_layer(recursive_snapshots: list, l1_moves: list) -> tuple[StockLayerBias, int, str]:
    """从最高有效递归级别提取选股层偏向。

    遍历 recursive_snapshots，找有 moves 的最高 level_id。
    若无递归层有效，退回 L1 move_snapshot。
    """
    best_level = 0
    best_moves: list = []

    for rs in recursive_snapshots:
        if rs.moves and rs.level_id > best_level:
            best_level = rs.level_id
            best_moves = rs.moves

    if not best_moves:
        best_level = 1
        best_moves = l1_moves

    if not best_moves:
        return StockLayerBias.NEUTRAL, 0, "no_moves"

    last = best_moves[-1]
    if last.kind == "trend":
        if last.direction == "up":
            return StockLayerBias.LONG, best_level, f"L{best_level}_trend_up"
        return StockLayerBias.SHORT, best_level, f"L{best_level}_trend_down"

    if last.direction == "up":
        return StockLayerBias.LONG, best_level, f"L{best_level}_consol_up"
    if last.direction == "down":
        return StockLayerBias.SHORT, best_level, f"L{best_level}_consol_down"
    return StockLayerBias.NEUTRAL, best_level, f"L{best_level}_neutral"


def _layers_allow_long(trading_mode: TradingLayerMode, stock_bias: StockLayerBias) -> bool:
    """两层组合：是否允许做多。"""
    if stock_bias == StockLayerBias.SHORT:
        return False
    if trading_mode == TradingLayerMode.SHORT_ONLY:
        return False
    if trading_mode == TradingLayerMode.UNKNOWN:
        return False
    return True


def _layers_allow_short(trading_mode: TradingLayerMode, stock_bias: StockLayerBias) -> bool:
    """两层组合：是否允许做空。"""
    if stock_bias == StockLayerBias.LONG:
        return False
    if trading_mode == TradingLayerMode.LONG_ONLY:
        return False
    if trading_mode == TradingLayerMode.UNKNOWN:
        return False
    return True


def _position_scale(trading_mode: TradingLayerMode) -> float:
    """操盘层决定仓位比例：盘整减半。"""
    if trading_mode == TradingLayerMode.BOTH_HALF:
        return 0.5
    return 1.0


# ════════════════════════════════════════════════════════════
# 单 pass 多级别回测引擎（两层分离版）
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
        "trading_mode_changes": 0,
        "stock_bias_changes": 0,
        "layer_conflicts": 0,
    }

    # ── L2 方向状态（PH 方向裁决，保留用于止损） ──
    l2_direction = 0

    # ── 两层状态（新增） ──
    trading_mode = TradingLayerMode.UNKNOWN
    stock_bias = StockLayerBias.NEUTRAL
    stock_bias_level = 0
    prev_trading_mode = TradingLayerMode.UNKNOWN
    prev_stock_bias = StockLayerBias.NEUTRAL

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
    entry_trading_mode = TradingLayerMode.UNKNOWN
    entry_stock_bias = StockLayerBias.NEUTRAL

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
        nonlocal entry_trading_mode, entry_stock_bias

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
            trading_mode=entry_trading_mode.name,
            stock_bias=entry_stock_bias.name,
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
        entry_trading_mode = TradingLayerMode.UNKNOWN
        entry_stock_bias = StockLayerBias.NEUTRAL

    for i in range(n):
        c = closes[i]
        bar = Bar(ts=base_ts + timedelta(minutes=i),
                  open=opens[i], high=highs[i], low=lows[i], close=c, volume=0.0)

        # ══════ 1. Incremental BSP pipeline ══════
        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events

        # ══════ 2. 两层状态更新 ══════
        prev_trading_mode = trading_mode
        prev_stock_bias = stock_bias

        trading_mode, _tm_desc = _extract_trading_layer(snap.move_snapshot.moves)
        stock_bias, stock_bias_level, _sb_desc = _extract_stock_layer(
            snap.recursive_snapshots, snap.move_snapshot.moves
        )

        if trading_mode != prev_trading_mode and prev_trading_mode != TradingLayerMode.UNKNOWN:
            ph_counts["trading_mode_changes"] += 1
        if stock_bias != prev_stock_bias and prev_stock_bias != StockLayerBias.NEUTRAL:
            ph_counts["stock_bias_changes"] += 1

        allow_long = _layers_allow_long(trading_mode, stock_bias)
        allow_short = _layers_allow_short(trading_mode, stock_bias)
        if not allow_long and not allow_short and trading_mode != TradingLayerMode.UNKNOWN:
            ph_counts["layer_conflicts"] += 1

        scale = _position_scale(trading_mode)

        # ══════ 3. L0 PH（每 bar 更新） ══════
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

        # ══════ 4. L1 PH（仅在 L1 线段 settle 时更新） ══════
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

        # ══════ 5. L2 PH（仅在 L1 走势 settle 时更新） ══════
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

        if l2_flip_long:
            if l2_direction != 1:
                ph_counts["l2_direction_flips"] += 1
            l2_direction = 1
        if l2_flip_short:
            if l2_direction != -1:
                ph_counts["l2_direction_flips"] += 1
            l2_direction = -1

        # ══════ 6. BSP 事件提取 ══════
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

        # ══════ 7. FSM 状态机（两层分离版） ══════
        if state == St.SCANNING:
            # BSP candidates 需两层同意
            if buy_cands and allow_long:
                pending_entry = {"bar": i, "side": "buy", "bsp_ids": set(buy_cands)}
            if sell_cands and allow_short:
                pending_entry = {"bar": i, "side": "sell", "bsp_ids": set(sell_cands)}

            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                else:
                    ps = pending_entry["side"]
                    inv_list = buy_invalidates if ps == "buy" else sell_invalidates
                    if any(bid in pending_entry["bsp_ids"] for bid in inv_list):
                        pending_entry = None

                    # 两层变化可能使 pending 失效
                    if pending_entry:
                        if ps == "buy" and not allow_long:
                            pending_entry = None
                        elif ps == "sell" and not allow_short:
                            pending_entry = None

            # 进场：pending BSP + L0 rank-1 settle 确认
            if pending_entry and pending_entry["side"] == "buy" and l0_r1_down:
                capital_used = INITIAL_CAPITAL * scale
                state = St.POSITION_OPEN
                direction = 1
                entry_price = c
                entry_bar = i
                cost_basis = c
                total_shares = capital_used / c
                total_invested = capital_used
                own_capital = INITIAL_CAPITAL
                cumulative_recovered = 0.0
                entry_bsp_ids = set(pending_entry["bsp_ids"])
                pending_entry = None
                has_active_trim = False
                add_count = 0
                diff_count = 0
                states_in_trade = {St.POSITION_OPEN.name}
                entry_trading_mode = trading_mode
                entry_stock_bias = stock_bias
                ev = f"ENTRY_LONG@{c:.2f} mode={trading_mode.name} bias={stock_bias.name} scale={scale:.1f}"

            elif pending_entry and pending_entry["side"] == "sell" and l0_r1_up:
                capital_used = INITIAL_CAPITAL * scale
                state = St.POSITION_OPEN
                direction = -1
                entry_price = c
                entry_bar = i
                cost_basis = c
                total_shares = capital_used / c
                total_invested = capital_used
                own_capital = INITIAL_CAPITAL
                cumulative_recovered = 0.0
                entry_bsp_ids = set(pending_entry["bsp_ids"])
                pending_entry = None
                has_active_trim = False
                add_count = 0
                diff_count = 0
                states_in_trade = {St.POSITION_OPEN.name}
                entry_trading_mode = trading_mode
                entry_stock_bias = stock_bias
                ev = f"ENTRY_SHORT@{c:.2f} mode={trading_mode.name} bias={stock_bias.name} scale={scale:.1f}"

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            states_in_trade.add(state.name)

            should_stop = False
            stop_reason = ""

            # 止损条件1：L2 PH 方向翻转（保留）
            if direction == 1 and l2_flip_short:
                should_stop = True
                stop_reason = "l2_direction_flip_short"
            elif direction == -1 and l2_flip_long:
                should_stop = True
                stop_reason = "l2_direction_flip_long"

            # 止损条件2：选股层翻转为反方向
            if not should_stop:
                if direction == 1 and stock_bias == StockLayerBias.SHORT:
                    should_stop = True
                    stop_reason = "stock_bias_flip_short"
                elif direction == -1 and stock_bias == StockLayerBias.LONG:
                    should_stop = True
                    stop_reason = "stock_bias_flip_long"

            # 止损条件3：操盘层翻转为严格反向
            if not should_stop:
                if direction == 1 and trading_mode == TradingLayerMode.SHORT_ONLY:
                    should_stop = True
                    stop_reason = "trading_mode_flip_short"
                elif direction == -1 and trading_mode == TradingLayerMode.LONG_ONLY:
                    should_stop = True
                    stop_reason = "trading_mode_flip_long"

            # BSP invalidate
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
                # ── 加仓逻辑 ──
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

                # ── 降成本逻辑 ──
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
            print(f"    [{i/n*100:5.1f}%] bar {i:,}/{n:,}  mode={trading_mode.name} bias={stock_bias.name}")
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


def _mode_distribution(trades: list[CompletedTrade]) -> dict[str, list[CompletedTrade]]:
    """按入场时操盘层模式分组。"""
    groups: dict[str, list[CompletedTrade]] = {}
    for t in trades:
        groups.setdefault(t.trading_mode, []).append(t)
    return groups


def write_report(all_results: list[dict]) -> None:
    L: list[str] = []
    L.append("# 两层分离回测 — 1分钟数据\n")
    L.append("## 架构\n")
    L.append("两层分离 + 5状态 FSM + 三级别 PH 分层：\n")
    L.append("| 层 | 信号来源 | 决定什么 |")
    L.append("|---|---------|---------|")
    L.append("| 选股层 | 最高有效递归级别 Move 方向 | 品种方向（long/short/neutral） |")
    L.append("| 操盘层 | L1 Move 类型+方向 | 操作策略（只多/只空/双向半仓） |")
    L.append("| PH L2 | L1 走势端点 PH rank-1 settle | 止损触发 |")
    L.append("| PH L1 | L1 线段端点 PH | 降成本/加仓比例 |")
    L.append("| PH L0 | 1min close PH rank-1 settle | 进场门控 |\n")
    L.append("**两层组合规则**：入场需两层都同意，矛盾时不操作。\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        ph = res["ph_counts"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：**{res['n_bars']:,}** bars (1min)")
        L.append(f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}")
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**")
        L.append(f"- 回测耗时：{res['elapsed']:.1f}s\n")

        L.append("### 两层状态统计\n")
        L.append("| 指标 | 值 |")
        L.append("|------|-----|")
        L.append(f"| 操盘层模式切换 | {ph['trading_mode_changes']} |")
        L.append(f"| 选股层偏向切换 | {ph['stock_bias_changes']} |")
        L.append(f"| 两层矛盾（不操作） | {ph['layer_conflicts']:,} bars |")
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

        # 按操盘层模式分组统计
        mode_groups = _mode_distribution(res["trades"])
        if mode_groups:
            L.append("### 按操盘层模式分组\n")
            L.append("| 模式 | 交易数 | 胜率 | 平均PnL | 复利% |")
            L.append("|------|--------|------|---------|------|")
            for mode_name, mode_trades in sorted(mode_groups.items()):
                mm = compute_metrics(mode_trades)
                L.append(f"| {mode_name} | {mm['n']} | {mm['win_rate']:.0f}% | "
                         f"{mm['avg_pnl']:+.3f}% | {mm['total_compound']:+.2f}% |")
            L.append("")

        L.append("### 交易明细\n")
        L.append("| # | 方向 | 入场 | 出场 | bars | PnL% | 模式 | 偏向 | 加仓 | 短差 | PW | 退出原因 |")
        L.append("|---|------|------|------|------|------|------|------|------|------|----|---------| ")
        for idx, t in enumerate(res["trades"]):
            d_str = "多" if t.direction == 1 else "空"
            hold = t.exit_bar - t.entry_bar
            pw = "是" if t.principal_withdrawn else "否"
            L.append(f"| {idx+1} | {d_str} | {t.entry_price:.2f}@{t.entry_bar} | "
                     f"{t.exit_price:.2f}@{t.exit_bar} | {hold:,} | {t.pnl_pct:+.2f} | "
                     f"{t.trading_mode} | {t.stock_bias} | "
                     f"{t.n_add_positions} | {t.n_short_diffs} | {pw} | "
                     f"{t.exit_reason} |")
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

    # ── 三版对比 ──
    L.append("## 三版对比\n")
    L.append("| 标的 | 版本 | 复利% | 胜率 | 交易数 | 最大回撤 |")
    L.append("|------|------|------|------|--------|---------|")

    old_ph = {"QQQ": -100, "OKLO": -100, "HK700": -100}
    ml_ph = {"QQQ": 58, "OKLO": 72, "HK700": 223}

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        L.append(f"| {sym} | 单层PH（旧H组） | 全亏 | — | — | — |")
        L.append(f"| {sym} | 多级别PH（上版） | +{ml_ph.get(sym, '?')}% | — | — | — |")
        L.append(f"| {sym} | **两层分离（本版）** | **{m['total_compound']:+.2f}%** | "
                 f"{m['win_rate']:.0f}% | {m['n']} | {m['max_dd']:.1f}% |")
    L.append("")

    # ── 汇总 ──
    L.append("## 汇总\n")
    L.append("| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | 模式切换 | 偏向切换 | 矛盾bars | 耗时 |")
    L.append("|------|------|-----|------|------|--------|---------|---------|---------|------|")
    for res in all_results:
        m = res["metrics"]
        ph = res["ph_counts"]
        L.append(f"| {res['symbol']} | {res['n_bars']:,} | {res['bh']:+.1f} | "
                 f"{m['total_compound']:+.2f} | {m['win_rate']:.0f}% | {m['n']} | "
                 f"{ph['trading_mode_changes']} | {ph['stock_bias_changes']} | "
                 f"{ph['layer_conflicts']:,} | {res['elapsed']:.0f}s |")
    L.append("")

    L.append("## 结果包六要素\n")
    L.append("**结论**：两层分离回测 — 操盘层用 L1 走势类型区分策略，选股层用最高递归级别方向做代理。\n")
    L.append("**定义依据**：")
    L.append("- 走势类型 Move.kind: trend（2+中枢递升/递降）/ consolidation（1中枢）— `a_move_v1.py`")
    L.append("- Move.direction: up/down — 趋势=中枢递升方向，盘整=break_direction")
    L.append("- 操盘层规则：trend_up→只多, trend_down→只空, consolidation→双向半仓")
    L.append("- 选股层规则：最高有效递归级别最后一个 Move 的方向")
    L.append("- 两层组合：入场需两层同意（选股层不反对 + 操盘层允许该方向）\n")
    L.append("**边界条件**：")
    L.append(f"- PENDING_EXPIRY = {PENDING_EXPIRY} bars")
    L.append("- 递归深度 max_levels=2 → 最高有效级别取决于数据量是否足以涌现")
    L.append("- 选股层在递归未涌现时退回 L1 → 两层可能实质退化为单层")
    L.append("- 盘整半仓是固定 0.5 比例 — 可考虑动态调整\n")
    L.append("**下游推论**：")
    L.append("- 若两层分离优于单纯多级别PH → 走势类型区分是有效的过滤器")
    L.append("- 若矛盾bars占比高 → 两层频繁矛盾说明级别错配")
    L.append("- 若盘整交易亏损多 → consolidation 策略需要更严格的进出条件\n")
    L.append("**谱系引用**：")
    L.append("- 走势方向代理陷阱（memory: project_trend_direction_proxy）")
    L.append("- 526号：递归存在论区分")
    L.append("- 267号：满仓满融降成本体系\n")
    L.append("**影响声明**：新建独立回测脚本 `fugue_twolayer_backtest_1min.py`，不修改引擎代码或旧版回测。\n")
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
        print(f"  {symbol} — 两层分离回测 (1min)")
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
            print(f"  两层统计：模式切换={ph_counts['trading_mode_changes']}, "
                  f"偏向切换={ph_counts['stock_bias_changes']}, "
                  f"矛盾={ph_counts['layer_conflicts']:,} bars")
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
