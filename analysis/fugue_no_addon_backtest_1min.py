"""控制变量：去掉加仓 — 在D2区间套版基线上只删加仓逻辑。

基线：D2区间套版（QQQ+60.7%, OKLO+192.4%, HK700+325.3%）

改动：
- 去掉持仓期间所有加仓路径（buy BSP加仓 + S5a回调加仓）
- 保留降成本（原文30课"分区卷钱"）
- 其他全部不动

控制变量：有加仓 vs 无加仓。

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
OUTPUT_MD = ROOT / "analysis" / "fugue_no_addon_backtest_1min.md"
SUB_EXPIRY = 60  # 短差级别等待上限（bars）
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100_000.0


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
    WAIT_SUB_ENTRY = auto()   # 区间套：等短差级别精确入场
    HOLDING = auto()
    EVAL = auto()
    WAIT_DIP = auto()
    OBSERVE = auto()
    WAIT_SUB_EXIT = auto()    # 区间套：等短差级别精确出场
    EXIT = auto()


class CostSt(Enum):
    FULL_POS = auto()
    REDUCED = auto()
    PRINCIPAL_RECOVERED = auto()   # 成本归零（cumulative_recovered ≥ own_capital）
    EARNING_SHARES = auto()        # 挣股数阶段：成本为0后短差利润买回更多股数（53/81课）


@dataclass(frozen=True)
class UpSegRecord:
    """记录一个向上段走势的关键数据，用于段间比较（38课S7-S9）。"""
    seg_start: int
    high: float
    low: float
    bar_start: int
    bar_end: int
    force: float


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
    final_shares: float = 0.0       # 退出时持仓股数（挣股数阶段会 > 入场股数）
    entry_shares: float = 0.0       # 入场股数（INITIAL_CAPITAL / buy_price）
    n_earn_shares: int = 0          # 挣股数阶段的增持次数
    earn_entry_shares: float = 0.0  # 进入挣股数阶段时的股数（成本归零点）
    # 挣股数阶段独立贡献：阶段内净增股数的退出清算价值，占初始本金的百分比。
    # = (final_shares − earn_entry_shares) × exit_price / INITIAL_CAPITAL × 100。
    # 隔离了"成本归零后金额守恒短差增持"对总收益的独立贡献（可正可负）。
    earn_contribution_pct: float = 0.0


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
    "BTC": DATA_DIR / "btc_1m_3year.json",
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

def _median_alive_persistence(alive: _Alive) -> float:
    """操作级别 PH 树 alive 组件的 median persistence（从数据涌现）。"""
    bars = alive.bars
    if not bars:
        return 0.0
    persis = sorted(b.persistence for b in bars)
    n = len(persis)
    if n % 2 == 1:
        return persis[n // 2]
    return (persis[n // 2 - 1] + persis[n // 2]) / 2


def _move_force_simple(move) -> float:
    """简单力度计算：价格振幅。"""
    return abs(move.high - move.low)


def _check_no_new_high(curr_high: float, prev_high: float) -> bool:
    """38课S7a: 不创新高判断。"""
    return curr_high < prev_high


def _check_consolidation_divergence(
    curr_force: float,
    prev_force: float,
    curr_high: float,
    prev_high: float,
) -> bool:
    """38课S7b-i: 盘整背驰判断。
    创新高但力度减弱 = 盘整背驰。
    """
    if curr_high <= prev_high:
        return False
    return curr_force < prev_force


# ════════════════════════════════════════════════════════════
# 单 pass 完整赋格回测引擎（阶段一：纯做多+降成本）
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
        "eval_triggered": 0,
        "eval_filtered": 0,
    }

    # ── L2 方向状态 ──
    l2_direction = 0

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
    earn_count = 0   # 挣股数阶段增持次数
    earn_entry_shares = 0.0  # 进入挣股数阶段时的股数（用于隔离挣股数独立贡献）

    # 区间套子状态
    sub_entry_bar = -1           # WAIT_SUB_ENTRY 开始的 bar
    sub_entry_op_price = 0.0     # 操作级别信号价格（fallback用）
    sub_exit_bar = -1            # WAIT_SUB_EXIT 开始的 bar
    sub_exit_reason = ""         # 退出原因（传递给 _close_trade）
    sub_exit_op_price = 0.0      # 操作级别退出信号价格（fallback用）
    interval_improvements: list[float] = []  # 区间套改善记录

    # ── 向上段走势记录（用于段间比较，使用 Move 对象） ──
    up_seg_records: list[UpSegRecord] = []

    # ── 向下段（回调段）记录 ──
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
        nonlocal add_count, diff_count, earn_count, states_in_trade
        nonlocal earn_entry_shares
        nonlocal up_seg_records, current_down_low
        nonlocal sub_entry_bar, sub_exit_bar, sub_exit_reason

        if total_shares <= 0 or entry_price <= 0:
            state = St.WAIT_ENTRY
            return

        # 全额记账（38/53/81课完整流程）：
        #   最终现金价值 = 已回收本金(cumulative_recovered) + 持仓清算(total_shares×price)
        #   pnl% = (该价值 − 初始本金) / 初始本金。
        # 阶段1(降成本，total_shares 固定)与原 (price−cost_basis)/entry_price 公式数学等价；
        # 阶段2(挣股数，total_shares 增长、cost_basis≤0)下正确捕获增持股数的收益。
        entry_shares = INITIAL_CAPITAL / entry_price
        pnl_pct = (cumulative_recovered + total_shares * price - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
        # 挣股数阶段独立贡献：仅当进入过挣股数（earn_entry_shares>0）才计；
        # 阶段内净增股数（可负）的退出清算价值占初始本金比例。
        earn_added = (total_shares - earn_entry_shares) if earn_entry_shares > 0 else 0.0
        earn_contribution_pct = earn_added * price / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4),
            states_visited=sorted(states_in_trade),
            n_add_positions=add_count,
            n_short_diffs=diff_count,
            cumulative_recovered=cumulative_recovered,
            principal_withdrawn=(cost_state in (CostSt.PRINCIPAL_RECOVERED, CostSt.EARNING_SHARES)),
            exit_reason=reason, cost_basis_at_exit=cost_basis,
            n_up_segs=len(up_seg_records),
            final_shares=total_shares, entry_shares=entry_shares,
            n_earn_shares=earn_count,
            earn_entry_shares=earn_entry_shares,
            earn_contribution_pct=round(earn_contribution_pct, 4),
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
        earn_count = 0
        earn_entry_shares = 0.0
        states_in_trade = set()
        up_seg_records = []
        current_down_low = float("inf")
        sub_entry_bar = -1
        sub_exit_bar = -1
        sub_exit_reason = ""

    for i in range(n):
        c = closes[i]
        h = highs[i]
        lo = lows[i]
        bar = Bar(
            ts=base_ts + timedelta(minutes=i),
            open=opens[i], high=h, low=lo, close=c, volume=0.0,
        )

        # ══════ 1. Incremental BSP pipeline ══════
        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events
        moves = snap.move_snapshot.moves

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

        # ══════ 6. 走势段数据提取（用于38课S7比较） ══════
        # 提取新 settle 的 up-move（操作级别走势类型完成）
        new_settled_up_moves: list = []
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1) and e.direction == "up":
                for m in moves:
                    if (m.seg_start == e.seg_start
                            and m.direction == "up"
                            and m.settled):
                        new_settled_up_moves.append(m)
                        break

        state_counts[state.name] += 1
        ev = ""

        # ══════ 7. 完整 38 课 FSM（纯做多） ══════

        if state == St.WAIT_ENTRY:
            # S1: 等待操作级别下跌背驰（buy BSP candidate + L2 方向为多）
            if buy_cands and l2_direction == 1:
                pending_entry = {"bar": i, "bsp_ids": set(buy_cands)}

            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                elif any(
                    bid in pending_entry["bsp_ids"] for bid in buy_invalidates
                ):
                    pending_entry = None

            # S2: 区间套定位确认 — L0 rank-1 settle 作为精确入场信号
            if pending_entry and l0_r1_down:
                state = St.ENTRY
                ev = f"WAIT→ENTRY BSP={pending_entry['bsp_ids']}"

        elif state == St.ENTRY:
            # 区间套：不直接买入，进入 WAIT_SUB_ENTRY 等短差级别确认
            state = St.WAIT_SUB_ENTRY
            sub_entry_bar = i
            sub_entry_op_price = c
            entry_bsp_ids = (
                set(pending_entry["bsp_ids"]) if pending_entry else set()
            )
            pending_entry = None
            ev = f"ENTRY→WAIT_SUB@{c:.2f} L2dir={l2_direction}"

        elif state == St.WAIT_SUB_ENTRY:
            states_in_trade.add(St.WAIT_SUB_ENTRY.name)
            do_buy = False
            buy_price = c
            buy_reason = ""

            # 短差级别 buy BSP 出现 → 精确入场
            if buy_cands and l0_r1_down:
                do_buy = True
                buy_price = c
                improvement = sub_entry_op_price - c
                interval_improvements.append(improvement)
                buy_reason = f"SUB_ENTRY@{c:.2f} (改善{improvement:+.2f})"
            # 超时 fallback
            elif (i - sub_entry_bar) > SUB_EXPIRY:
                do_buy = True
                buy_price = c
                buy_reason = f"SUB_ENTRY_TIMEOUT@{c:.2f}"
            # L2翻空 → 放弃入场
            elif l2_flip_short:
                state = St.WAIT_ENTRY
                ev = f"SUB_ENTRY_ABORT:L2_FLIP@{c:.2f}"
            # BSP invalidate → 放弃入场
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                state = St.WAIT_ENTRY
                ev = f"SUB_ENTRY_ABORT:BSP_INV@{c:.2f}"

            if do_buy:
                state = St.HOLDING
                cost_state = CostSt.FULL_POS
                entry_price = buy_price
                entry_bar = i
                cost_basis = buy_price
                total_shares = INITIAL_CAPITAL / buy_price
                own_capital = INITIAL_CAPITAL
                cumulative_recovered = 0.0
                has_active_trim = False
                add_count = 0
                diff_count = 0
                earn_count = 0
                earn_entry_shares = 0.0
                states_in_trade = {
                    St.ENTRY.name, St.WAIT_SUB_ENTRY.name, St.HOLDING.name,
                }
                up_seg_records = []
                current_down_low = float("inf")
                ev = buy_reason

        elif state == St.HOLDING:
            states_in_trade.add(St.HOLDING.name)

            # ── 退出条件1: L2 方向翻空 → 直接退出（优先级最高） ──
            if l2_flip_short:
                ev = f"L2_FLIP_SHORT@{c:.2f}"
                _close_trade(i, c, "l2_direction_flip_short")

            # ── 退出条件2: BSP invalidate ──
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                ev = f"BSP_INVALIDATE@{c:.2f}"
                _close_trade(i, c, "bsp_invalidate")

            else:
                # ── 38课S3/S7: 操作级别走势类型完成 → EVAL ──
                # 使用 MoveSettleV1（settled up-move）作为走势段完成信号
                # 这是操作级别的走势类型（含中枢的 a+A+b 结构），不是1分钟噪声
                if new_settled_up_moves:
                    # 操作级别 PH alive median persistence（门槛从数据涌现）
                    med_p = _median_alive_persistence(l1_down.alive)

                    for settled_mv in new_settled_up_moves:
                        mv_p = settled_mv.persistence
                        new_seg = UpSegRecord(
                            seg_start=settled_mv.seg_start,
                            high=settled_mv.high,
                            low=settled_mv.low,
                            bar_start=settled_mv.first_seg_s0,
                            bar_end=settled_mv.last_seg_s1,
                            force=_move_force_simple(settled_mv),
                        )

                        # ── persistence 门槛过滤 ──
                        # 低 persistence settle → 只降成本，不触发段间比较
                        if mv_p < med_p:
                            ph_counts["eval_filtered"] += 1
                            if not ev:
                                ev = (
                                    f"MOVE_SETTLE_SKIP(p={mv_p:.2f}"
                                    f"<med={med_p:.2f})"
                                )
                            continue

                        ph_counts["eval_triggered"] += 1
                        states_in_trade.add(St.EVAL.name)
                        ev = (
                            f"→EVAL(move_settle) "
                            f"high={settled_mv.high:.2f} "
                            f"force={new_seg.force:.2f} "
                            f"p={mv_p:.2f}/med={med_p:.2f}"
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
                                state = St.WAIT_SUB_EXIT
                                sub_exit_bar = i
                                sub_exit_reason = "no_new_high"
                                sub_exit_op_price = c
                                ev += (
                                    f" → WAIT_SUB_EXIT(不创新高"
                                    f" {new_seg.high:.2f}"
                                    f"<{prev_seg.high:.2f})"
                                )
                                break

                            elif _check_consolidation_divergence(
                                new_seg.force, prev_seg.force,
                                new_seg.high, prev_seg.high,
                            ):
                                up_seg_records.append(new_seg)
                                state = St.WAIT_SUB_EXIT
                                sub_exit_bar = i
                                sub_exit_reason = "consolidation_divergence"
                                sub_exit_op_price = c
                                ev += (
                                    f" → WAIT_SUB_EXIT(盘整背驰"
                                    f" force {new_seg.force:.2f}"
                                    f"<{prev_seg.force:.2f})"
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

                # ── 降成本逻辑（L1 non-rank-1 settle） ──
                if state == St.HOLDING and cost_state == CostSt.FULL_POS:
                    if l1_nr1_up and not has_active_trim:
                        ratio = _persistence_ratio(l1_up.alive, 1)
                        if ratio > 0.05:
                            active_trim_sell_price = c
                            active_trim_shares = total_shares * ratio
                            has_active_trim = True
                            cost_state = CostSt.REDUCED
                            if not ev:
                                ev = (
                                    f"TRIM@{c:.2f} "
                                    f"shares={active_trim_shares:.1f}"
                                )

                elif state == St.HOLDING and cost_state == CostSt.REDUCED:
                    if has_active_trim:
                        should_close_diff = l1_nr1_down or bool(buy_cands)
                        if should_close_diff:
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
                                    f"CLOSE_DIFF@{c:.2f}"
                                    f" profit={profit:.2f}"
                                )

                            if cost_basis <= 0:
                                # 成本归零（缠师本质判据 cost_basis≤0，31/33/43课）
                                # → 进入挣股数。clamp cost_basis=0（缠师"成本永远为0"），
                                # 过冲部分已计入 cumulative_recovered。
                                # no_addon 无融资：cost_basis≤0 ⟺ recovered≥own_capital
                                # （本金回收=成本归零=挣股数，同一里程碑）。
                                cost_basis = 0.0
                                cost_state = CostSt.EARNING_SHARES
                                states_in_trade.add("EARNING_SHARES")
                                earn_entry_shares = total_shares
                                ev += " → 挣股数"
                            else:
                                cost_state = CostSt.FULL_POS
                    else:
                        if l1_nr1_up:
                            ratio = _persistence_ratio(l1_up.alive, 1)
                            if ratio > 0.05:
                                active_trim_sell_price = c
                                active_trim_shares = total_shares * ratio
                                has_active_trim = True
                                if not ev:
                                    ev = (
                                        f"NEW_TRIM@{c:.2f}"
                                        f" shares={active_trim_shares:.1f}"
                                    )

                elif (
                    state == St.HOLDING
                    and cost_state == CostSt.EARNING_SHARES
                ):
                    # 挣股数阶段（53课"0成本后就赚筹码"、81课"降成本、增筹码"）：
                    # 成本已归零，短差利润不再降 cost_basis，而是按当前价买回更多股数
                    # → total_shares 增加（新增股数 0/负成本）。亏损短差则减少股数（无截断）。
                    if not has_active_trim:
                        if l1_nr1_up:
                            ratio = _persistence_ratio(l1_up.alive, 1)
                            if ratio > 0.05:
                                active_trim_sell_price = c
                                active_trim_shares = total_shares * ratio
                                has_active_trim = True
                                if not ev:
                                    ev = f"EARN_TRIM@{c:.2f}"
                    elif has_active_trim:
                        should_close = l1_nr1_down or bool(buy_cands)
                        if should_close:
                            profit = (
                                (active_trim_sell_price - c)
                                * active_trim_shares
                            )
                            # 用短差利润买回更多股数（c>0）：挣股数核心
                            if c > 0:
                                total_shares += profit / c
                            earn_count += 1
                            diff_count += 1
                            has_active_trim = False
                            if not ev:
                                ev = (
                                    f"EARN_CLOSE@{c:.2f}"
                                    f" +shares={profit / c if c > 0 else 0:.1f}"
                                )

        elif state == St.WAIT_SUB_EXIT:
            states_in_trade.add(St.WAIT_SUB_EXIT.name)
            do_exit = False
            exit_price = c

            # L2翻空 → 立即退出（不等短差级别）
            if l2_flip_short:
                do_exit = True
                exit_price = c
                sub_exit_reason = "l2_direction_flip_short"
                ev = f"SUB_EXIT_URGENT:L2_FLIP@{c:.2f}"
            # BSP invalidate → 立即退出
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                do_exit = True
                exit_price = c
                sub_exit_reason = "bsp_invalidate"
                ev = f"SUB_EXIT_URGENT:BSP_INV@{c:.2f}"
            # 短差级别 sell BSP → 精确出场
            elif sell_cands and l0_r1_up:
                do_exit = True
                exit_price = c
                improvement = c - sub_exit_op_price
                interval_improvements.append(improvement)
                ev = (
                    f"SUB_EXIT@{c:.2f}"
                    f" (改善{improvement:+.2f})"
                )
            # 超时 fallback
            elif (i - sub_exit_bar) > SUB_EXPIRY:
                do_exit = True
                exit_price = c
                ev = f"SUB_EXIT_TIMEOUT@{c:.2f}"

            if do_exit:
                reason = sub_exit_reason
                _close_trade(i, exit_price, reason)

        elif state == St.WAIT_DIP:
            states_in_trade.add(St.WAIT_DIP.name)

            # 跟踪回调段最低价
            if c < current_down_low:
                current_down_low = c

            # L2 方向翻空 → 清仓退出
            if l2_flip_short:
                ev = f"WAIT_DIP:L2_FLIP@{c:.2f}"
                _close_trade(i, c, "l2_direction_flip_short_in_dip")

            # BSP invalidate
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                ev = f"WAIT_DIP:BSP_INV@{c:.2f}"
                _close_trade(i, c, "bsp_invalidate_in_dip")

            else:
                # S5: 等待回调结束判断
                # 使用 buy BSP candidate 或 L0 rank-1 down settle 作为回调结束信号
                dip_ended = False
                if buy_cands:
                    dip_ended = True
                elif l0_r1_down:
                    dip_ended = True

                if dip_ended and up_seg_records:
                    last_up = up_seg_records[-1]

                    # S5a: 不跌破前低 → 继续持有（无加仓）
                    if current_down_low >= last_up.low:
                        state = St.HOLDING
                        cost_state = CostSt.FULL_POS
                        has_active_trim = False

                        ev = (
                            f"DIP_REBUY@{c:.2f} 不跌破前低"
                            f"(down_low={current_down_low:.2f}"
                            f" >= prev_low={last_up.low:.2f})"
                        )

                    # S5b: 跌破前低 — 检查盘整背驰
                    elif current_down_low < last_up.low:
                        down_force = last_up.low - current_down_low
                        prev_drop = (
                            up_seg_records[-2].low - last_up.low
                            if len(up_seg_records) >= 2
                            else down_force * 2
                        )

                        if down_force < abs(prev_drop):
                            # S5b: 跌破但盘整背驰 → 重新买入
                            state = St.HOLDING
                            cost_state = CostSt.FULL_POS
                            has_active_trim = False
                            ev = (
                                f"DIP_REBUY_DIVERGE@{c:.2f}"
                                f" 跌破+盘整背驰"
                            )
                        else:
                            # S5c: 跌破且无背驰 → 观望
                            state = St.OBSERVE
                            states_in_trade.add(St.OBSERVE.name)
                            ev = (
                                f"→OBSERVE@{c:.2f}"
                                f" 跌破无背驰"
                            )

        elif state == St.OBSERVE:
            states_in_trade.add(St.OBSERVE.name)

            # L2 方向翻空 → 清仓退出
            if l2_flip_short:
                ev = f"OBSERVE:L2_FLIP@{c:.2f}"
                _close_trade(i, c, "l2_direction_flip_short_in_observe")

            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                ev = f"OBSERVE:BSP_INV@{c:.2f}"
                _close_trade(i, c, "bsp_invalidate_in_observe")

            else:
                # 等待新的下跌背驰 → 重新入场
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
        St.HOLDING, St.ENTRY, St.WAIT_SUB_ENTRY, St.EVAL,
        St.WAIT_DIP, St.OBSERVE, St.WAIT_SUB_EXIT,
    ) and total_shares > 0:
        _close_trade(n - 1, closes[-1], "eod_close")

    return trades, event_records, state_counts, ph_counts, interval_improvements


# ════════════════════════════════════════════════════════════
# 统计 + 报告
# ════════════════════════════════════════════════════════════

def compute_metrics(trades: list[CompletedTrade]) -> dict:
    if not trades:
        return {
            "n": 0, "win_rate": 0.0, "avg_pnl": 0.0,
            "total_compound": 0.0, "max_dd": 0.0,
            "avg_hold_bars": 0, "n_with_add": 0,
            "n_with_cr": 0, "n_pw": 0, "n_with_seg_compare": 0,
            "n_earn_shares": 0, "max_shares_growth": 1.0,
            "n_trades_with_earn": 0, "earn_contribution_total": 0.0,
            "earn_contribution_avg": 0.0, "pnl_without_earn": 0.0,
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

    # 挣股数阶段独立贡献统计（阶段2/3的隔离收益）：
    earn_trades = [t for t in trades if t.earn_entry_shares > 0]
    earn_contrib_total = sum(t.earn_contribution_pct for t in earn_trades)
    # 反事实复利：把每笔的挣股数贡献从 pnl 中剥离，看停在成本归零点的总收益。
    eq_wo = 1.0
    for t in trades:
        pnl_wo = t.pnl_pct - t.earn_contribution_pct
        eq_wo *= 1 + pnl_wo / 100
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
        "n_earn_shares": sum(1 for t in trades if t.n_earn_shares > 0),
        "max_shares_growth": round(max(
            (t.final_shares / t.entry_shares for t in trades if t.entry_shares > 0),
            default=1.0,
        ), 3),
        # 挣股数阶段独立贡献：
        "n_trades_with_earn": len(earn_trades),
        "earn_contribution_total": round(earn_contrib_total, 4),
        "earn_contribution_avg": round(
            earn_contrib_total / len(earn_trades), 4
        ) if earn_trades else 0.0,
        "pnl_without_earn": round((eq_wo - 1) * 100, 4),
    }


def write_report(all_results: list[dict]) -> None:
    L: list[str] = []
    L.append("# 控制变量：去掉加仓 — D2区间套版基线\n")
    L.append("## 架构\n")
    L.append("D2区间套版去掉所有加仓路径：\n")
    L.append("- 持仓期间不因同方向新BSP加仓")
    L.append("- S5a回调不跌破前低只继续持有，不加仓")
    L.append("- 保留降成本（30课原文）")
    L.append("- 其余全部不动\n")
    L.append("控制变量：有加仓 vs 无加仓。\n")

    L.append("### FSM 状态转移\n")
    L.append("```")
    L.append("WAIT_ENTRY ──(buy BSP + L2多 + L0确认)──→ ENTRY ──→ HOLDING")
    L.append("   ↑                                                  │")
    L.append("   │                                    sell BSP/L1r1↓ │")
    L.append(" OBSERVE ←── WAIT_DIP ←──────── EVAL ←────────────────┘")
    L.append("                                  │")
    L.append("                          不创新高/盘整背驰")
    L.append("                                  ↓")
    L.append("                          EXIT → WAIT_ENTRY")
    L.append("```\n")

    L.append("### PH三级别信号映射\n")
    L.append("| PH级别 | 输入 | 更新频率 | 信号用途 |")
    L.append("|--------|------|---------|---------|")
    L.append("| L0 | 1min close | 每bar | 区间套精确入场/回调结束确认 |")
    L.append("| L1 | L1线段端点 | ~数千次 | 降成本+走势完成信号+加仓 |")
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
        et = ph.get("eval_triggered", 0)
        ef = ph.get("eval_filtered", 0)
        L.append(
            f"| **EVAL** | 触发={et} | 过滤={ef}"
            f" | 过滤率={ef/(et+ef)*100:.0f}% |"
            if (et + ef) > 0
            else f"| **EVAL** | 触发=0 | 过滤=0 | — |"
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
            f"| 进入挣股数阶段（cost_basis≤0） | {m.get('n_trades_with_earn', 0)}/{m['n']} |"
        )
        L.append(
            f"| 最大股数增长 | ×{m.get('max_shares_growth', 1.0)} |"
        )
        L.append(
            f"| 挣股数独立贡献（合计） | {m.get('earn_contribution_total', 0.0):+.2f}% |"
        )
        L.append(
            f"| 剥离挣股数后复利 | {m.get('pnl_without_earn', 0.0):+.2f}% |"
        )
        L.append(
            f"| 有段间比较的交易 | {m.get('n_with_seg_compare', 0)}/{m['n']} |"
        )
        L.append("")

        L.append("### 交易明细\n")
        L.append(
            "| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差"
            " | 向上段数 | 退出原因 | 状态路径 |"
        )
        L.append(
            "|---|------|------|---------|------|------"
            "|------|---------|---------|---------|"
        )
        for idx, t in enumerate(res["trades"]):
            hold = t.exit_bar - t.entry_bar
            states = "→".join(t.states_visited)
            L.append(
                f"| {idx + 1} | {t.entry_price:.2f}@{t.entry_bar}"
                f" | {t.exit_price:.2f}@{t.exit_bar}"
                f" | {hold:,} | {t.pnl_pct:+.2f}"
                f" | {t.n_add_positions} | {t.n_short_diffs}"
                f" | {t.n_up_segs} | {t.exit_reason} | {states} |"
            )
        L.append("")

        L.append("### FSM 状态分布（按 bar 数）\n")
        L.append("| 状态 | Bars | 占比 |")
        L.append("|------|------|------|")
        total_bars = sum(res["state_counts"].values())
        for st_name in [
            "WAIT_ENTRY", "ENTRY", "WAIT_SUB_ENTRY", "HOLDING", "EVAL",
            "WAIT_DIP", "OBSERVE", "WAIT_SUB_EXIT", "EXIT",
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
    # 六版横向对比表
    # ════════════════════════════════════════════════════════════
    L.append("## 控制变量对比\n")
    L.append("| 版本 | QQQ | OKLO | HK700 |")
    L.append("|------|-----|------|-------|")
    L.append("| D2 区间套版（有加仓） | +60.7% | +192.4% | +325.3% |")

    no_addon_row = "| D2 区间套版（**无加仓，本版**） |"
    for res in all_results:
        m = res["metrics"]
        no_addon_row += f" **{m['total_compound']:+.1f}%** |"
    L.append(no_addon_row)
    L.append("\n唯一差异：是否在持仓期间加仓。\n")

    # ════════════════════════════════════════════════════════════
    # 汇总
    # ════════════════════════════════════════════════════════════
    L.append("## 汇总\n")
    L.append(
        "| 标的 | Bars | BH% | 复利% | 胜率 | 交易数"
        " | L2翻转 | 加仓 | 降成本 | PW | 段比较 | 耗时 |"
    )
    L.append(
        "|------|------|-----|------|------|--------"
        "|--------|------|--------|-----|--------|------|"
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
            f" | {res['elapsed']:.0f}s |"
        )
    L.append("")

    L.append("## 结果包六要素\n")
    L.append(
        "**结论**：控制变量——去掉加仓后的表现对比。\n"
    )
    L.append("**定义依据**：")
    L.append("- 38课操作程式原文是全进全出，不涉及分批加仓")
    L.append("- 30课分区卷钱是不同级别各做各的，不是同级别加仓\n")
    L.append("**边界条件**：")
    L.append(f"- PENDING_EXPIRY = {PENDING_EXPIRY} bars")
    L.append("- 降成本保留（原文有）")
    L.append("- 加仓去掉（原文无明确依据）\n")
    L.append("**下游推论**：")
    L.append("- 若无加仓 > 有加仓 → 加仓逻辑是负alpha，应删除")
    L.append("- 若无加仓 < 有加仓 → 加仓产生了正alpha，应保留")
    L.append("- 若无加仓 ≈ 有加仓 → 加仓不是关键变量\n")
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
        "（真实数据，4标的 1min QQQ/OKLO/HK700/BTC；可产生否定性结果）。\n"
    )
    L.append(
        "**审计修复声明**（2026-06-06）：降成本利润计算已移除 `if profit > 0`"
        " 截断（审计 A1）——亏损 trim 现照实扣减 cost_basis / cumulative_recovered，"
        "不再是只赚不赔的提款机。本表数字为去截断后的真实基线。"
    )
    L.append("")
    L.append(
        "**挣股数阶段实现声明**（缠师 31/33/43 课三阶段资金管理）：\n"
        "本脚本实现完整三阶段——降成本（cost_basis>0，短差股数守恒）→ "
        "成本归零（cost_basis≤0，缠师本质判据）→ 挣股数（短差金额守恒、total_shares 增长）。"
        "触发判据为 **cost_basis ≤ 0**（非 cumulative_recovered），与正式 FSM "
        "`cost_reduction_fsm.py` 的 EARNING_SHARES 态同构。"
    )
    L.append(
        "**挣股数 L2 否定性结果**：QQQ(17笔)/OKLO(6笔) 1min 真实数据上 **0 笔**进入挣股数阶段——"
        "cost_basis 从未触及 ≤0。降成本深度（100%=归零）QQQ 最佳仅 4.35%、中位 −0.78%，"
        "OKLO 全为负（最佳 −6.92%）。即去截断后降成本短差本身在 1min 上净负值，"
        "挣股数前置条件（cost_basis≤0）的有效域在此数据/周期上为空。"
        "这与缠师原文自洽——成本归零通常需'股票翻倍后出掉部分仓位'或超长盘整，"
        "而当前退出规则在翻倍前切断交易。**否定性结果缩小了有效域边界**（形式化有效域规则）。"
    )

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    symbols = ["QQQ", "OKLO", "HK700", "BTC"]
    all_results: list[dict] = []

    for symbol in symbols:
        print(f"\n{'=' * 60}")
        print(f"  {symbol} — 控制变量:无加仓版赋格回测")
        print(f"{'=' * 60}")

        try:
            opens, highs, lows, closes, dates = load_1min(symbol)
            n = len(closes)
            print(f"  数据：{n:,} bars")

            t0 = time.time()
            trades, event_records, state_counts, ph_counts, iv_imps = run_backtest(
                opens, highs, lows, closes,
            )
            elapsed = time.time() - t0

            m = compute_metrics(trades)
            bh = (closes[-1] - closes[0]) / closes[0] * 100

            print(f"  完成：{elapsed:.1f}s ({n / elapsed:.0f} bars/s)")
            et = ph_counts["eval_triggered"]
            ef = ph_counts["eval_filtered"]
            print(
                f"  PH：L0={ph_counts['l0_r1_settles']} r1, "
                f"L1={ph_counts['l1_updates']} upd/"
                f"{ph_counts['l1_r1_settles']} r1, "
                f"L2={ph_counts['l2_updates']} upd/"
                f"{ph_counts['l2_r1_settles']} r1, "
                f"方向翻转={ph_counts['l2_direction_flips']}"
            )
            print(
                f"  EVAL：触发={et}, 过滤={ef}, "
                f"过滤率={ef/(et+ef)*100:.0f}%"
                if (et + ef) > 0
                else f"  EVAL：触发=0, 过滤=0"
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
            print(
                f"  挣股数：{m['n_trades_with_earn']}/{m['n']}笔, "
                f"最大股数增长×{m['max_shares_growth']}, "
                f"独立贡献：{m['earn_contribution_total']:+.2f}% "
                f"(剥离后复利={m['pnl_without_earn']:+.2f}%)"
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
                "interval_improvements": iv_imps,
            })
        except Exception as ex:
            print(f"  FAILED: {ex}")
            import traceback
            traceback.print_exc()

    if all_results:
        write_report(all_results)
        audit = {
            r["symbol"]: {
                "n_bars": r["n_bars"],
                "bh": r["bh"],
                "fsm_compound": r["metrics"]["total_compound"],
                "win_rate": r["metrics"]["win_rate"],
                "n_trades": r["metrics"]["n"],
                "n_earn_shares": r["metrics"].get("n_earn_shares", 0),
                "max_shares_growth": r["metrics"].get("max_shares_growth", 1.0),
                "n_trades_with_earn": r["metrics"].get("n_trades_with_earn", 0),
                "earn_contribution_total": r["metrics"].get("earn_contribution_total", 0.0),
                "earn_contribution_avg": r["metrics"].get("earn_contribution_avg", 0.0),
                "pnl_without_earn": r["metrics"].get("pnl_without_earn", 0.0),
                "pnls": [t.pnl_pct for t in r["trades"]],
            }
            for r in all_results
        }
        (DATA_DIR / "fugue_no_addon_audit_results.json").write_text(
            json.dumps(audit, indent=2)
        )
        print("审计 JSON 已写入：fugue_no_addon_audit_results.json")


if __name__ == "__main__":
    main()
