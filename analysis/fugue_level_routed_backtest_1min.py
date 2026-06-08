"""级别路由版缠论+PH赋格回测 — 阶段一（纯做多+降成本+MACD背驰）。

基于 fugue_complete_backtest_1min.py（完整版阶段一），两个核心改动：

1. 走势 settle 事件的**级别路由**：
   - persistence >= P75 → **操作级别路径**：38课段间比较（S7a/S7b），可能退出
   - persistence < P75  → **短差级别路径**：降成本（减仓/回补循环），不触发退出
   - 两条路径不相交

2. **MACD背驰替代价格振幅**：
   - S7b-i 盘整背驰用三维度 OR 判定（MACD面积 + DIF峰值 + 柱子高度）
   - 价格振幅仅作 fallback（MACD数据不足时）
   - OnlineMacdState 增量计算，直接读 _hist_hist/_macd_hist 列表避免 DataFrame 开销

不变点：
- S7a 不创新高判断仍为价格比较（定义本身是价格级别的）
- L2 PH 方向翻转仍为最高优先级退出
- L0 PH 仍用于区间套精确入场
- 引擎 BSP 行为不变（MACD仅用于回测自身的段间比较）

认识论等级：L2（真实数据，3标的 1min；可产生否定性结果）。
"""

from __future__ import annotations

import bisect
import json
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum, auto
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.a_online_persistence import (  # noqa: E402
    MergeBar,
    OnlineMergeTree,
)
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_level_routed_backtest_1min.md"
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100_000.0


# ════════════════════════════════════════════════════════════
# Persistence 级别路由器
# ════════════════════════════════════════════════════════════

@dataclass
class PersistenceRouter:
    """用 P75 阈值将走势 settle 事件路由到操作级别或短差级别。

    操作级别：persistence >= P75 → 38课段间比较
    短差级别：persistence < P75  → 降成本循环
    阈值从历史走势 settle 的 persistence 分布涌现。
    """

    _sorted: list[float] = field(default_factory=list)
    n_operation: int = 0
    n_short_diff: int = 0

    def add(self, persistence: float) -> None:
        bisect.insort(self._sorted, persistence)

    def threshold(self) -> float:
        n = len(self._sorted)
        if n == 0:
            return 0.0
        idx = int(n * 0.75)
        return self._sorted[min(idx, n - 1)]

    def route(self, persistence: float) -> str:
        t = self.threshold()
        if persistence >= t:
            self.n_operation += 1
            return "operation"
        self.n_short_diff += 1
        return "short_diff"


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
    alive: list[MergeBar] = []
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
    tree: OnlineMergeTree = field(
        default_factory=lambda: OnlineMergeTree(track_dominant=False)
    )
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
# FSM 状态（完整 38 课操作程式）
# ════════════════════════════════════════════════════════════

class St(Enum):
    WAIT_ENTRY = auto()
    ENTRY = auto()
    HOLDING = auto()
    EVAL = auto()
    WAIT_DIP = auto()
    OBSERVE = auto()
    EXIT = auto()


class CostSt(Enum):
    FULL_POS = auto()
    REDUCED = auto()
    PRINCIPAL_RECOVERED = auto()


@dataclass(frozen=True)
class UpSegRecord:
    """记录一个向上段走势的关键数据，用于段间比较（38课S7-S9）。"""
    seg_start: int
    high: float
    low: float
    bar_start: int
    bar_end: int
    force: float
    macd_area: float = 0.0
    dif_peak: float = 0.0
    hist_peak: float = 0.0


@dataclass
class CompletedTrade:
    entry_bar: int
    entry_price: float
    exit_bar: int
    exit_price: float
    pnl_pct: float
    states_visited: list[str]
    n_add_positions: int
    n_short_diffs: int
    cumulative_recovered: float
    principal_withdrawn: bool
    exit_reason: str
    cost_basis_at_exit: float
    n_up_segs: int = 0
    n_op_routes: int = 0
    n_diff_routes: int = 0


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


def load_1min(
    symbol: str,
) -> tuple[list[float], list[float], list[float], list[float], list[str]]:
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
# 走势段比较工具
# ════════════════════════════════════════════════════════════

def _move_force_simple(move) -> float:
    return abs(move.high - move.low)


def _move_macd_metrics(
    move,
    segments: list,
    merged_to_raw: list,
    macd_dif: list[float],
    macd_hist: list[float],
    direction: str,
) -> tuple[float, float, float]:
    """计算走势的 MACD 三维度指标：(面积, DIF峰值, 柱子高度峰值)。

    直接读列表切片，避免 DataFrame 开销。O(move_bar_count)。
    """
    if (move.seg_start >= len(segments)
            or move.seg_end >= len(segments)):
        return (0.0, 0.0, 0.0)

    i0 = segments[move.seg_start].i0
    i1 = segments[move.seg_end].i1

    if i0 >= len(merged_to_raw) or i1 >= len(merged_to_raw):
        return (0.0, 0.0, 0.0)

    raw_i0 = merged_to_raw[i0][0]
    raw_i1 = merged_to_raw[i1][1]

    if raw_i0 < 0:
        raw_i0 = 0
    if raw_i1 >= len(macd_hist):
        raw_i1 = len(macd_hist) - 1
    if raw_i0 > raw_i1:
        return (0.0, 0.0, 0.0)

    area = 0.0
    dif_peak = 0.0
    hist_peak = 0.0

    if direction == "up":
        for k in range(raw_i0, raw_i1 + 1):
            h = macd_hist[k]
            if h > 0:
                area += h
            if h > hist_peak:
                hist_peak = h
            d = macd_dif[k]
            if d > dif_peak:
                dif_peak = d
    else:
        for k in range(raw_i0, raw_i1 + 1):
            h = macd_hist[k]
            if h < 0:
                area -= h
            ah = -h
            if ah > hist_peak:
                hist_peak = ah
            d = -macd_dif[k]
            if d > dif_peak:
                dif_peak = d

    return (area, dif_peak, hist_peak)


def _check_no_new_high(curr_high: float, prev_high: float) -> bool:
    return curr_high < prev_high


def _check_consolidation_divergence_macd(
    curr_area: float,
    prev_area: float,
    curr_dif: float,
    prev_dif: float,
    curr_hist: float,
    prev_hist: float,
    curr_high: float,
    prev_high: float,
) -> bool:
    """38课S7b-i: 盘整背驰判断（MACD三维度OR）。

    创新高但 MACD 力度减弱 = 盘整背驰。
    三维度 OR（beichi.md #2 已结算）：任一满足即背驰。
    """
    if curr_high <= prev_high:
        return False
    t2 = prev_area > 0 and curr_area < prev_area
    t6 = prev_dif > 0 and curr_dif < prev_dif
    t7 = prev_hist > 0 and curr_hist < prev_hist
    return t2 or t6 or t7


# ════════════════════════════════════════════════════════════
# 单 pass 级别路由赋格回测引擎（阶段一：纯做多+降成本）
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

    # ── MACD 增量状态（独立于引擎，仅用于回测段间比较） ──
    macd_state = OnlineMacdState()

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
        "route_operation": 0, "route_short_diff": 0,
    }

    # ── L2 方向状态 ──
    l2_direction = 0

    # ── 级别路由器 ──
    persistence_router = PersistenceRouter()

    # ── 38课程式 FSM ──
    state = St.WAIT_ENTRY
    cost_state = CostSt.FULL_POS
    total_shares = 0.0
    cost_basis = 0.0
    entry_price = 0.0
    entry_bar = -1
    own_capital = INITIAL_CAPITAL
    cumulative_recovered = 0.0
    active_trim_sell_price = 0.0
    active_trim_shares = 0.0
    has_active_trim = False
    pending_entry: dict | None = None
    entry_bsp_ids: set[int] = set()
    states_in_trade: set[str] = set()
    add_count = 0
    diff_count = 0
    trade_op_routes = 0
    trade_diff_routes = 0

    # ── 向上段走势记录（用于段间比较） ──
    up_seg_records: list[UpSegRecord] = []

    # ── 向下段记录 ──
    current_down_low = float("inf")

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
        nonlocal state, cost_state, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumulative_recovered, own_capital
        nonlocal has_active_trim, pending_entry, entry_bsp_ids
        nonlocal add_count, diff_count, states_in_trade
        nonlocal up_seg_records, current_down_low
        nonlocal trade_op_routes, trade_diff_routes

        if total_shares <= 0 or entry_price <= 0:
            state = St.WAIT_ENTRY
            return

        pnl_pct = (price - cost_basis) / entry_price * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4),
            states_visited=sorted(states_in_trade),
            n_add_positions=add_count,
            n_short_diffs=diff_count,
            cumulative_recovered=cumulative_recovered,
            principal_withdrawn=(cost_state == CostSt.PRINCIPAL_RECOVERED),
            exit_reason=reason, cost_basis_at_exit=cost_basis,
            n_up_segs=len(up_seg_records),
            n_op_routes=trade_op_routes,
            n_diff_routes=trade_diff_routes,
        ))
        remaining = INITIAL_CAPITAL * (1 + pnl_pct / 100)
        state = St.WAIT_ENTRY
        cost_state = CostSt.FULL_POS
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
        up_seg_records = []
        current_down_low = float("inf")
        trade_op_routes = 0
        trade_diff_routes = 0

    for i in range(n):
        c = closes[i]
        h = highs[i]
        lo = lows[i]
        bar = Bar(
            ts=base_ts + timedelta(minutes=i),
            open=opens[i], high=h, low=lo, close=c, volume=0.0,
        )

        # ══════ 1. Incremental BSP pipeline + MACD ══════
        snap = orch.process_bar(bar)
        macd_state.update(c, bar.ts)
        bsp_events = snap.bsp_snapshot.events
        moves = snap.move_snapshot.moves
        segments = snap.seg_snapshot.segments
        merged_to_raw = snap.bi_snapshot.merged_to_raw

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

        # ══════ 4. L2 PH + 级别路由（在 L1 走势 settle 时） ══════
        l2_flip_long = False
        l2_flip_short = False
        routed_op_up: list = []
        routed_diff_up: list = []
        routed_diff_down: list = []

        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1):
                mv = None
                for m in moves:
                    if (m.seg_start == e.seg_start
                            and m.direction == e.direction
                            and m.settled):
                        mv = m
                        break
                if mv is None:
                    continue

                ep = mv.high if mv.direction == "up" else mv.low
                if ep <= 0:
                    continue

                # L2 PH 更新（不变）
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

                # ── 级别路由：persistence vs P75 ──
                route = persistence_router.route(mv.persistence)
                persistence_router.add(mv.persistence)

                if route == "operation":
                    ph_counts["route_operation"] += 1
                    if mv.direction == "up":
                        routed_op_up.append(mv)
                else:
                    ph_counts["route_short_diff"] += 1
                    if mv.direction == "up":
                        routed_diff_up.append(mv)
                    else:
                        routed_diff_down.append(mv)

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
        buy_invalidates: list[int] = []

        for e in bsp_events:
            nm = type(e).__name__
            bid = e.bsp_id
            side = e.side
            if "Candidate" in nm:
                if side == "buy":
                    buy_cands.append(bid)
                else:
                    sell_cands.append(bid)
            elif "Confirm" in nm:
                if side == "buy":
                    buy_confirms.append(bid)
            elif "Invalidate" in nm:
                if side == "buy":
                    buy_invalidates.append(bid)

        state_counts[state.name] += 1
        ev = ""

        # ══════ 6. 级别路由版 FSM（纯做多） ══════

        if state == St.WAIT_ENTRY:
            if buy_cands and l2_direction == 1:
                pending_entry = {"bar": i, "bsp_ids": set(buy_cands)}

            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                elif any(
                    bid in pending_entry["bsp_ids"] for bid in buy_invalidates
                ):
                    pending_entry = None

            if pending_entry and l0_r1_down:
                state = St.ENTRY
                ev = f"WAIT→ENTRY BSP={pending_entry['bsp_ids']}"

        elif state == St.ENTRY:
            state = St.HOLDING
            cost_state = CostSt.FULL_POS
            entry_price = c
            entry_bar = i
            cost_basis = c
            total_shares = INITIAL_CAPITAL / c
            own_capital = INITIAL_CAPITAL
            cumulative_recovered = 0.0
            entry_bsp_ids = (
                set(pending_entry["bsp_ids"]) if pending_entry else set()
            )
            pending_entry = None
            has_active_trim = False
            add_count = 0
            diff_count = 0
            states_in_trade = {St.ENTRY.name, St.HOLDING.name}
            up_seg_records = []
            current_down_low = float("inf")
            trade_op_routes = 0
            trade_diff_routes = 0
            ev = f"ENTRY_LONG@{c:.2f} L2dir={l2_direction}"

        elif state == St.HOLDING:
            states_in_trade.add(St.HOLDING.name)

            # ── 退出条件1: L2 方向翻空（最高优先级） ──
            if l2_flip_short:
                ev = f"L2_FLIP_SHORT@{c:.2f}"
                _close_trade(i, c, "l2_direction_flip_short")

            # ── 退出条件2: BSP invalidate ──
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                ev = f"BSP_INVALIDATE@{c:.2f}"
                _close_trade(i, c, "bsp_invalidate")

            else:
                # ═══════════════════════════════════════════
                # 操作级别路径：高 persistence up-move → 段间比较
                # ═══════════════════════════════════════════
                if routed_op_up:
                    for settled_mv in routed_op_up:
                        trade_op_routes += 1
                        mv_area, mv_dif, mv_hist = _move_macd_metrics(
                            settled_mv, segments, merged_to_raw,
                            macd_state._macd_hist,
                            macd_state._hist_hist, "up",
                        )
                        new_seg = UpSegRecord(
                            seg_start=settled_mv.seg_start,
                            high=settled_mv.high,
                            low=settled_mv.low,
                            bar_start=settled_mv.first_seg_s0,
                            bar_end=settled_mv.last_seg_s1,
                            force=_move_force_simple(settled_mv),
                            macd_area=mv_area,
                            dif_peak=mv_dif,
                            hist_peak=mv_hist,
                        )
                        threshold = persistence_router.threshold()

                        states_in_trade.add(St.EVAL.name)
                        ev = (
                            f"→EVAL(op_route) "
                            f"p={settled_mv.persistence:.2f}"
                            f"≥P75={threshold:.2f} "
                            f"high={settled_mv.high:.2f} "
                            f"area={mv_area:.2f}"
                        )

                        if not up_seg_records:
                            up_seg_records.append(new_seg)
                            state = St.WAIT_DIP
                            states_in_trade.add(St.WAIT_DIP.name)
                            current_down_low = float("inf")
                            ev += (
                                f" → WAIT_DIP"
                                f"(U1完成 high={new_seg.high:.2f})"
                            )
                        else:
                            prev_seg = up_seg_records[-1]

                            if _check_no_new_high(
                                new_seg.high, prev_seg.high,
                            ):
                                up_seg_records.append(new_seg)
                                ev += (
                                    f" → EXIT(不创新高"
                                    f" {new_seg.high:.2f}"
                                    f"<{prev_seg.high:.2f})"
                                )
                                _close_trade(i, c, "no_new_high")
                                break

                            elif _check_consolidation_divergence_macd(
                                new_seg.macd_area,
                                prev_seg.macd_area,
                                new_seg.dif_peak,
                                prev_seg.dif_peak,
                                new_seg.hist_peak,
                                prev_seg.hist_peak,
                                new_seg.high,
                                prev_seg.high,
                            ):
                                up_seg_records.append(new_seg)
                                ev += (
                                    f" → EXIT(盘整背驰MACD"
                                    f" area {new_seg.macd_area:.2f}"
                                    f"<{prev_seg.macd_area:.2f})"
                                )
                                _close_trade(
                                    i, c, "consolidation_divergence_macd",
                                )
                                break

                            else:
                                up_seg_records.append(new_seg)
                                state = St.WAIT_DIP
                                states_in_trade.add(St.WAIT_DIP.name)
                                current_down_low = float("inf")
                                ev += (
                                    f" → WAIT_DIP(创新高无背驰"
                                    f" high={new_seg.high:.2f})"
                                )

                # ═══════════════════════════════════════════
                # 短差级别路径：低 persistence move → 降成本
                # （仅在操作级别路径未触发退出时执行）
                # ═══════════════════════════════════════════
                if state == St.HOLDING:
                    # ── 短差 up-move settle → TRIM（高点减仓） ──
                    if routed_diff_up and not has_active_trim:
                        trade_diff_routes += 1
                        diff_mv = routed_diff_up[0]
                        threshold = persistence_router.threshold()
                        ratio = (
                            diff_mv.persistence / threshold
                            if threshold > 0 else 0.1
                        )
                        ratio = max(0.05, min(ratio, 0.5))

                        if cost_state in (
                            CostSt.FULL_POS,
                            CostSt.PRINCIPAL_RECOVERED,
                        ):
                            active_trim_sell_price = c
                            active_trim_shares = total_shares * ratio
                            has_active_trim = True
                            if cost_state == CostSt.FULL_POS:
                                cost_state = CostSt.REDUCED
                            if not ev:
                                ev = (
                                    f"TRIM(diff_route) "
                                    f"p={diff_mv.persistence:.2f}"
                                    f"<P75={threshold:.2f} "
                                    f"ratio={ratio:.3f} "
                                    f"shares={active_trim_shares:.1f}"
                                )

                    # ── 短差 down-move settle / buy BSP → CLOSE_DIFF ──
                    if has_active_trim:
                        should_close = (
                            bool(routed_diff_down) or bool(buy_cands)
                        )
                        if should_close:
                            trade_diff_routes += 1
                            profit = (
                                (active_trim_sell_price - c)
                                * active_trim_shares
                            )
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumulative_recovered += profit
                            diff_count += 1
                            has_active_trim = False
                            if not ev:
                                ev = (
                                    f"CLOSE_DIFF(diff_route)@{c:.2f}"
                                    f" profit={profit:.2f}"
                                )

                            if cumulative_recovered >= own_capital:
                                cost_state = CostSt.PRINCIPAL_RECOVERED
                                states_in_trade.add("PRINCIPAL_RECOVERED")
                                ev += " → PW"
                            elif cost_state == CostSt.REDUCED:
                                cost_state = CostSt.FULL_POS

                # ── 加仓逻辑（buy BSP + L1 ratio，独立于路由） ──
                if state == St.HOLDING and (buy_cands or buy_confirms):
                    side_match = buy_cands + buy_confirms
                    new_bsp_ids = [
                        bid for bid in side_match if bid not in entry_bsp_ids
                    ]
                    if new_bsp_ids:
                        ratio = _persistence_ratio(l1_down.alive, 1)
                        if ratio > 0.05:
                            add_shares = total_shares * ratio
                            old_val = total_shares * cost_basis
                            new_total = total_shares + add_shares
                            cost_basis = (
                                (old_val + add_shares * c) / new_total
                            )
                            total_shares = new_total
                            add_count += 1
                            entry_bsp_ids.update(new_bsp_ids)
                            if not ev:
                                ev = (
                                    f"ADD_POS@{c:.2f} +{add_shares:.1f}"
                                    f" L1ratio={ratio:.3f}"
                                )

        elif state == St.WAIT_DIP:
            states_in_trade.add(St.WAIT_DIP.name)

            if c < current_down_low:
                current_down_low = c

            if l2_flip_short:
                ev = f"WAIT_DIP:L2_FLIP@{c:.2f}"
                _close_trade(i, c, "l2_direction_flip_short_in_dip")

            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                ev = f"WAIT_DIP:BSP_INV@{c:.2f}"
                _close_trade(i, c, "bsp_invalidate_in_dip")

            else:
                dip_ended = False
                if buy_cands:
                    dip_ended = True
                elif l0_r1_down:
                    dip_ended = True

                if dip_ended and up_seg_records:
                    last_up = up_seg_records[-1]

                    if current_down_low >= last_up.low:
                        state = St.HOLDING
                        cost_state = CostSt.FULL_POS
                        has_active_trim = False

                        ratio = _persistence_ratio(l1_down.alive, 1)
                        if ratio > 0.05 and buy_cands:
                            add_shares = total_shares * ratio
                            old_val = total_shares * cost_basis
                            new_total = total_shares + add_shares
                            cost_basis = (
                                (old_val + add_shares * c) / new_total
                            )
                            total_shares = new_total
                            add_count += 1
                            entry_bsp_ids.update(buy_cands)

                        ev = (
                            f"DIP_REBUY@{c:.2f} 不跌破前低"
                            f"(down_low={current_down_low:.2f}"
                            f" >= prev_low={last_up.low:.2f})"
                        )

                    elif current_down_low < last_up.low:
                        down_force = last_up.low - current_down_low
                        prev_drop = (
                            up_seg_records[-2].low - last_up.low
                            if len(up_seg_records) >= 2
                            else down_force * 2
                        )

                        if down_force < abs(prev_drop):
                            state = St.HOLDING
                            cost_state = CostSt.FULL_POS
                            has_active_trim = False
                            ev = (
                                f"DIP_REBUY_DIVERGE@{c:.2f}"
                                f" 跌破+盘整背驰"
                            )
                        else:
                            state = St.OBSERVE
                            states_in_trade.add(St.OBSERVE.name)
                            ev = (
                                f"→OBSERVE@{c:.2f}"
                                f" 跌破无背驰"
                            )

        elif state == St.OBSERVE:
            states_in_trade.add(St.OBSERVE.name)

            if l2_flip_short:
                ev = f"OBSERVE:L2_FLIP@{c:.2f}"
                _close_trade(i, c, "l2_direction_flip_short_in_observe")

            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                ev = f"OBSERVE:BSP_INV@{c:.2f}"
                _close_trade(i, c, "bsp_invalidate_in_observe")

            else:
                if buy_cands and l0_r1_down:
                    state = St.HOLDING
                    cost_state = CostSt.FULL_POS
                    has_active_trim = False
                    entry_bsp_ids.update(buy_cands)
                    ev = f"OBSERVE_REENTRY@{c:.2f} 新下跌背驰"

        if ev:
            _record(i, c, ev)

        if i - last_progress >= 100_000:
            print(f"    [{i / n * 100:5.1f}%] bar {i:,}/{n:,}")
            last_progress = i

    # 期末清仓
    if state in (
        St.HOLDING, St.ENTRY, St.EVAL, St.WAIT_DIP, St.OBSERVE,
    ) and total_shares > 0:
        _close_trade(n - 1, closes[-1], "eod_close")

    return trades, event_records, state_counts, ph_counts


# ════════════════════════════════════════════════════════════
# 统计 + 报告
# ════════════════════════════════════════════════════════════

def compute_metrics(trades: list[CompletedTrade]) -> dict:
    if not trades:
        return {
            "n": 0, "win_rate": 0.0, "avg_pnl": 0.0,
            "total_compound": 0.0, "max_dd": 0.0,
            "avg_hold_bars": 0, "n_with_add": 0,
            "n_with_cr": 0, "n_pw": 0,
        }
    wins = sum(1 for t in trades if t.pnl_pct > 0)
    eq = 1.0
    peak = 1.0
    max_dd = 0.0
    for t in trades:
        eq *= 1 + t.pnl_pct / 100
        peak = max(peak, eq)
        max_dd = min(max_dd, (eq - peak) / peak)

    n_with_segs = sum(1 for t in trades if t.n_up_segs > 1)
    total_op = sum(t.n_op_routes for t in trades)
    total_diff = sum(t.n_diff_routes for t in trades)
    return {
        "n": len(trades),
        "win_rate": wins / len(trades) * 100,
        "avg_pnl": sum(t.pnl_pct for t in trades) / len(trades),
        "total_compound": (eq - 1) * 100,
        "max_dd": max_dd * 100,
        "avg_hold_bars": round(
            sum(t.exit_bar - t.entry_bar for t in trades) / len(trades)
        ),
        "n_with_add": sum(1 for t in trades if t.n_add_positions > 0),
        "n_with_cr": sum(1 for t in trades if t.n_short_diffs > 0),
        "n_pw": sum(1 for t in trades if t.principal_withdrawn),
        "n_with_seg_compare": n_with_segs,
        "total_op_routes": total_op,
        "total_diff_routes": total_diff,
    }


def write_report(all_results: list[dict]) -> None:
    L: list[str] = []
    L.append("# 级别路由版缠论+PH赋格回测 — 阶段一（纯做多+降成本）\n")
    L.append("## 架构\n")
    L.append("基于完整版（v6），两个核心改动：\n")
    L.append("### 改动1：级别路由\n")
    L.append("每个 MoveSettleV1 事件按 persistence 与 P75 阈值比较，路由到不相交路径：\n")
    L.append("1. **操作级别路径**（persistence ≥ P75）：38课段间比较，可能退出")
    L.append("2. **短差级别路径**（persistence < P75）：降成本循环，不会退出")
    L.append("3. **两条路径不相交**：一个 settle 事件只走一条路径\n")
    L.append("### 改动2：MACD三维度背驰\n")
    L.append("S7b-i 盘整背驰判断从价格振幅改为 MACD 三维度 OR：")
    L.append("- T2: MACD面积(U3) < MACD面积(U1)")
    L.append("- T6: DIF峰值(U3) < DIF峰值(U1)")
    L.append("- T7: 柱子高度(U3) < 柱子高度(U1)")
    L.append("- 任一满足 → 盘整背驰（beichi.md #2 已结算）\n")

    L.append("### 与 v6/v7 的区别\n")
    L.append("| 版本 | 段间比较触发 | 背驰判据 | 降成本触发 | 两路关系 |")
    L.append("|------|-------------|---------|-----------|---------|")
    L.append("| v6 完整版 | 所有 up-move settle | 价格振幅 | L1 PH nr1 | 独立并行 |")
    L.append("| v7 过滤版 | median 以上 up-move | 价格振幅 | L1 PH nr1 | 段间比较被过滤 |")
    L.append("| v8 路由版 | P75 以上 up-move | **MACD三维度** | P75以下move | **完全不相交** |\n")

    L.append("### FSM 状态转移\n")
    L.append("```")
    L.append("WAIT_ENTRY ──(buy BSP + L2多 + L0确认)──→ ENTRY ──→ HOLDING")
    L.append("   ↑                                                  │")
    L.append("   │                          move settle (persistence) │")
    L.append(" OBSERVE ←── WAIT_DIP ←──── EVAL ←─── [≥P75 up] ─────┘")
    L.append("                              │           │")
    L.append("                        不创新高/背驰      └── [<P75] → 降成本")
    L.append("                              ↓")
    L.append("                        EXIT → WAIT_ENTRY")
    L.append("```\n")

    L.append("### PH三级别信号映射\n")
    L.append("| PH级别 | 输入 | 更新频率 | 信号用途 |")
    L.append("|--------|------|---------|---------|")
    L.append("| L0 | 1min close | 每bar | 区间套精确入场/回调结束确认 |")
    L.append("| L1 | L1线段端点 | ~数千次 | 加仓 persistence ratio |")
    L.append("| L2 | L1走势端点 | ~数百次 | 方向裁决/紧急清仓 |\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        ph = res["ph_counts"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：**{res['n_bars']:,}** bars (1min)")
        L.append(
            f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}"
        )
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**")
        L.append(f"- 回测耗时：{res['elapsed']:.1f}s\n")

        L.append("### 级别路由统计\n")
        L.append("| 路径 | 事件数 | 占比 |")
        L.append("|------|--------|------|")
        total_routed = ph["route_operation"] + ph["route_short_diff"]
        if total_routed > 0:
            op_pct = ph["route_operation"] / total_routed * 100
            sd_pct = ph["route_short_diff"] / total_routed * 100
        else:
            op_pct = sd_pct = 0.0
        L.append(
            f"| 操作级别（≥P75） | {ph['route_operation']}"
            f" | {op_pct:.1f}% |"
        )
        L.append(
            f"| 短差级别（<P75） | {ph['route_short_diff']}"
            f" | {sd_pct:.1f}% |"
        )
        L.append(f"| **合计** | **{total_routed}** | 100% |")
        L.append("")

        L.append("### PH 分层统计\n")
        L.append("| 级别 | 更新次数 | settle总数 | rank-1 settle |")
        L.append("|------|---------|-----------|---------------|")
        L.append(
            f"| L0 | {res['n_bars']:,} (每bar)"
            f" | {ph['l0_settles']:,} | {ph['l0_r1_settles']} |"
        )
        L.append(
            f"| L1 | {ph['l1_updates']:,}"
            f" | {ph['l1_settles']:,} | {ph['l1_r1_settles']} |"
        )
        L.append(
            f"| L2 | {ph['l2_updates']:,}"
            f" | {ph['l2_settles']:,} | {ph['l2_r1_settles']} |"
        )
        L.append(
            f"| **L2 方向翻转** | — | —"
            f" | **{ph['l2_direction_flips']}** |"
        )
        L.append("")

        L.append("### 总体指标\n")
        L.append("| 指标 | 值 |")
        L.append("|------|-----|")
        L.append(f"| 交易数 | {m['n']}（纯多头） |")
        L.append(f"| 胜率 | {m['win_rate']:.1f}% |")
        L.append(f"| 平均收益 | {m['avg_pnl']:+.3f}% |")
        L.append(f"| 复利累计 | **{m['total_compound']:+.2f}%** |")
        L.append(f"| 最大回撤 | {m['max_dd']:.2f}% |")
        L.append(f"| 平均持仓 | {m['avg_hold_bars']:,} bars |")
        L.append(f"| 有加仓的交易 | {m['n_with_add']}/{m['n']} |")
        L.append(f"| 有降成本的交易 | {m['n_with_cr']}/{m['n']} |")
        L.append(f"| 达到本金回收 | {m['n_pw']}/{m['n']} |")
        L.append(
            f"| 有段间比较的交易 | {m.get('n_with_seg_compare', 0)}/{m['n']} |"
        )
        L.append(
            f"| 操作级别路由总数 | {m.get('total_op_routes', 0)} |"
        )
        L.append(
            f"| 短差级别路由总数 | {m.get('total_diff_routes', 0)} |"
        )
        L.append("")

        L.append("### 交易明细\n")
        L.append(
            "| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差"
            " | 向上段 | 操作/短差路由 | 退出原因 | 状态路径 |"
        )
        L.append(
            "|---|------|------|---------|------|------"
            "|------|--------|--------------|---------|---------|"
        )
        for idx, t in enumerate(res["trades"]):
            hold = t.exit_bar - t.entry_bar
            states = "→".join(t.states_visited)
            L.append(
                f"| {idx + 1} | {t.entry_price:.2f}@{t.entry_bar}"
                f" | {t.exit_price:.2f}@{t.exit_bar}"
                f" | {hold:,} | {t.pnl_pct:+.2f}"
                f" | {t.n_add_positions} | {t.n_short_diffs}"
                f" | {t.n_up_segs}"
                f" | {t.n_op_routes}/{t.n_diff_routes}"
                f" | {t.exit_reason} | {states} |"
            )
        L.append("")

        L.append("### FSM 状态分布（按 bar 数）\n")
        L.append("| 状态 | Bars | 占比 |")
        L.append("|------|------|------|")
        total_bars = sum(res["state_counts"].values())
        for st_name in [
            "WAIT_ENTRY", "ENTRY", "HOLDING", "EVAL",
            "WAIT_DIP", "OBSERVE", "EXIT",
        ]:
            cnt = res["state_counts"].get(st_name, 0)
            pct = cnt / total_bars * 100 if total_bars else 0
            L.append(f"| {st_name} | {cnt:,} | {pct:.1f}% |")
        L.append("")

        evts = res["event_records"]
        if evts:
            L.append(f"<details><summary>事件流（{len(evts)}条）</summary>\n")
            L.append("| Bar | 价格 | 状态 | 事件 |")
            L.append("|-----|------|------|------|")
            for r in evts[:150]:
                L.append(
                    f"| {r.bar_idx:,} | {r.price:.2f}"
                    f" | {r.state} | {r.event} |"
                )
            if len(evts) > 150:
                L.append(
                    f"| ... | ... | ..."
                    f" | （共{len(evts)}条，显示前150） |"
                )
            L.append("</details>\n")

    # ════════════════════════════════════════════════════════════
    # 八版横向对比表
    # ════════════════════════════════════════════════════════════
    L.append("## 八版横向对比\n")
    L.append("| 版本 | QQQ | OKLO | HK700 |")
    L.append("|------|-----|------|-------|")
    L.append("| 1. 单层PH H组 | -19.8% | -99% | -84% |")
    L.append("| 2. 多级别PH（双向） | +58.3% | +72.3% | +222.8% |")
    L.append("| 3. Regime过滤器 | +58.3% | +23.7% | +222.8% |")
    L.append("| 4. 走势类型过滤 | +18% | -86% | +6% |")
    L.append("| 5. 纯做多 | +71.7% | +228% | +292.9% |")
    L.append("| 6. 完整版（无过滤） | +23.1% | +133.2% | +224.1% |")
    L.append("| 7. persistence过滤版 | — | — | — |")

    routed_row = "| 8. **级别路由版（本版）** |"
    for res in all_results:
        m = res["metrics"]
        routed_row += f" **{m['total_compound']:+.1f}%** |"
    L.append(routed_row)
    L.append("")

    # ════════════════════════════════════════════════════════════
    # 汇总
    # ════════════════════════════════════════════════════════════
    L.append("## 汇总\n")
    L.append(
        "| 标的 | Bars | BH% | 复利% | 胜率 | 交易数"
        " | L2翻转 | 加仓 | 降成本 | PW | 段比较"
        " | 操作路由 | 短差路由 | 耗时 |"
    )
    L.append(
        "|------|------|-----|------|------|--------"
        "|--------|------|--------|-----|--------"
        "|---------|---------|------|"
    )
    for res in all_results:
        m = res["metrics"]
        ph = res["ph_counts"]
        nn = m["n"] or 1
        L.append(
            f"| {res['symbol']} | {res['n_bars']:,}"
            f" | {res['bh']:+.1f}"
            f" | {m['total_compound']:+.2f}"
            f" | {m['win_rate']:.0f}%"
            f" | {m['n']}"
            f" | {ph['l2_direction_flips']}"
            f" | {m['n_with_add']}/{nn}"
            f" | {m['n_with_cr']}/{nn}"
            f" | {m['n_pw']}/{nn}"
            f" | {m.get('n_with_seg_compare', 0)}/{nn}"
            f" | {ph['route_operation']}"
            f" | {ph['route_short_diff']}"
            f" | {res['elapsed']:.0f}s |"
        )
    L.append("")

    L.append("## 结果包六要素\n")
    L.append(
        "**结论**：级别路由版+MACD背驰在1分钟级别3标的上的表现。"
        "两个核心改动：(1) persistence P75 级别路由，(2) MACD三维度背驰替代价格振幅。\n"
    )
    L.append("**定义依据**：")
    L.append("- 38课第36行：同级别分解操作程式——段间比较是操作级别事件")
    L.append("- 38课第30行：分区卷钱降成本——降成本是短差级别事件")
    L.append("- 第24/25课：MACD辅助力度判断（面积+DIF峰值+柱子高度）")
    L.append("- beichi.md #2（已结算）：三维度 OR 判定")
    L.append("- Move.persistence（ph_layer.py）：走势的结构性影响度\n")
    L.append("**边界条件**：")
    L.append(f"- PENDING_EXPIRY = {PENDING_EXPIRY} bars")
    L.append("- P75 阈值在前 3 个 settle 事件期间为 0（全部路由到操作级别）")
    L.append("- 盘整背驰：MACD三维度OR（面积/DIF/柱子高度任一衰减即触发）")
    L.append("- 不创新高（S7a）仍为价格比较（定义本身是价格级别的）")
    L.append("- 降成本 trim ratio = persistence / P75，裁剪到 [0.05, 0.5]")
    L.append("- 阶段一：纯做多，向下段空仓等待")
    L.append("- 无滑点/手续费建模\n")
    L.append("**下游推论**：")
    L.append("- MACD背驰 vs 价格振幅背驰的退出时机差异")
    L.append("- 若路由版 > 完整版 → 分离路径+MACD共同改善了退出")
    L.append("- 若路由版 < 完整版 → 需区分路由效应和MACD效应\n")
    L.append("**谱系引用**：")
    L.append("- 525号：组件局部完成性")
    L.append("- 526号：a0递归存在论区分")
    L.append("- 521号：PH纯拓扑无动量\n")
    L.append(
        "**影响声明**：新建独立回测脚本，"
        "不修改引擎代码或旧版回测。\n"
    )
    L.append(
        "**认识论等级**：L2"
        "（真实数据，3标的 1min；可产生否定性结果）。"
    )

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    symbols = ["QQQ", "OKLO", "HK700"]
    all_results: list[dict] = []

    for symbol in symbols:
        print(f"\n{'=' * 60}")
        print(f"  {symbol} — 级别路由版赋格回测（阶段一：纯做多+降成本）")
        print(f"{'=' * 60}")

        try:
            opens, highs, lows, closes, dates = load_1min(symbol)
            n = len(closes)
            print(f"  数据：{n:,} bars")

            t0 = time.time()
            trades, event_records, state_counts, ph_counts = run_backtest(
                opens, highs, lows, closes,
            )
            elapsed = time.time() - t0

            m = compute_metrics(trades)
            bh = (closes[-1] - closes[0]) / closes[0] * 100

            print(f"  完成：{elapsed:.1f}s ({n / elapsed:.0f} bars/s)")
            print(
                f"  路由：操作={ph_counts['route_operation']}"
                f" 短差={ph_counts['route_short_diff']}"
            )
            print(
                f"  PH：L0={ph_counts['l0_r1_settles']} r1, "
                f"L1={ph_counts['l1_updates']} upd/"
                f"{ph_counts['l1_r1_settles']} r1, "
                f"L2={ph_counts['l2_updates']} upd/"
                f"{ph_counts['l2_r1_settles']} r1, "
                f"方向翻转={ph_counts['l2_direction_flips']}"
            )
            print(f"  交易：{m['n']}笔（纯多头）")
            print(
                f"  胜率：{m['win_rate']:.1f}%, "
                f"复利：{m['total_compound']:+.2f}%"
            )
            print(
                f"  加仓：{m['n_with_add']}/{m['n']}, "
                f"降成本：{m['n_with_cr']}/{m['n']}, "
                f"本金回收：{m['n_pw']}/{m['n']}"
            )
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
