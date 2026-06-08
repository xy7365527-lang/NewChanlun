"""全市场端到端回测：K4选股层驱动M1操盘层（v2 — 期货1min递归）。

两阶段架构：
  阶段1（预计算）：对每个标的独立跑完整 1min 递归，缓存 Move 方向变化时间序列
  阶段2（组合回测）：消费缓存结果，K4配置追踪 + 品种池调度 + FSM操盘

K4 三条独立边用真实期货 1min 数据：
  ES (E-mini S&P 500) → E/$
  GC (Gold)           → Au/$
  CL (Crude Oil)      → Oil/$

品种池规则：
  σ(E/$)=↑ or 0 → QQQ/HK700 long_only
  σ(E/$)=↓       → QQQ/HK700 不操作
  OKLO            → 始终 both（个股可做空）
  BTC             → 始终 both（独立于 K4）

认识论等级：L2（真实期货+真实标的 1min 数据，可产生否定性结果）。
"""

from __future__ import annotations

import bisect
import gc as garbage_collect
import json
import sys
import time
from collections import Counter
from dataclasses import dataclass, replace as dc_replace
from datetime import datetime
from enum import Enum, auto
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

DUMMY_TS = datetime(2020, 1, 1)

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.a_move_v1 import moves_from_zhongshus
from newchan.a_online_persistence import MergeBar, OnlineMergeTree
from newchan.a_zhongshu_v1 import zhongshu_from_strokes
from newchan.bi_engine import BiEngine
from newchan.core.recursion.buysellpoint_state import diff_buysellpoints
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.topology.config_space import Configuration, WalkDirection
from newchan.types import Bar

DATA_DIR = ROOT / "analysis" / "data_cache"
CACHE_DIR = DATA_DIR / "recursive_cache"
OUTPUT_MD = ROOT / "analysis" / "full_system_backtest.md"

INITIAL_CAPITAL = 100_000.0
BSP_WINDOW = 500
BSP_MARGIN = 50
LEVEL_ID = 1
PENDING_EXPIRY = 390
FEE_RATE = 0.001

BOTH = frozenset({1, -1})
LONG_ONLY = frozenset({1})
NO_TRADE = frozenset()


def _log(msg: str) -> None:
    print(msg)
    sys.stdout.flush()


# ════════════════════════════════════════════════════════════
# Data types
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class SigmaEvent:
    bar_idx: int
    date: str
    sigma: int  # +1, 0, -1


@dataclass(frozen=True)
class RecursiveCacheResult:
    symbol: str
    n_bars: int
    date_first: str
    date_last: str
    max_level: int
    sigma_events: list[SigmaEvent]
    final_sigma: int


@dataclass(frozen=True)
class CompletedTrade:
    entry_bar: int
    entry_price: float
    exit_bar: int
    exit_price: float
    direction: int
    pnl_pct: float
    n_short_diffs: int
    cumulative_recovered: float
    principal_withdrawn: bool
    exit_reason: str


@dataclass(frozen=True)
class AssetResult:
    symbol: str
    n_bars: int
    first_price: float
    last_price: float
    bh_pct: float
    trades: list[CompletedTrade]
    metrics: dict
    elapsed: float
    mode: str


class St(Enum):
    SCANNING = auto()
    POSITION_OPEN = auto()
    COST_REDUCING = auto()
    PRINCIPAL_WITHDRAWN = auto()
    STOPPED_OUT = auto()


# ════════════════════════════════════════════════════════════
# Data loading
# ════════════════════════════════════════════════════════════

def load_columnar(filename: str) -> tuple[str, list[str], list[float], list[float], list[float], list[float]]:
    raw = json.loads((DATA_DIR / filename).read_text())
    sym = raw.get("symbol", filename.split("_")[0].upper())
    return (
        sym,
        raw.get("dates", []),
        [float(x) for x in raw["opens"]],
        [float(x) for x in raw["highs"]],
        [float(x) for x in raw["lows"]],
        [float(x) for x in raw["closes"]],
    )


def load_row_based(filename: str, symbol: str) -> tuple[str, list[str], list[float], list[float], list[float], list[float]]:
    raw = json.loads((DATA_DIR / filename).read_text())
    return (
        symbol,
        [b.get("date", f"bar_{i}") for i, b in enumerate(raw)],
        [float(b.get("open", b["close"])) for b in raw],
        [float(b.get("high", b["close"])) for b in raw],
        [float(b.get("low", b["close"])) for b in raw],
        [float(b["close"]) for b in raw],
    )


# ════════════════════════════════════════════════════════════
# Phase 1: Precompute recursive structure — extract sigma time series
# ════════════════════════════════════════════════════════════

def _extract_sigma(snap) -> int:
    for rs in reversed(snap.recursive_snapshots):
        if rs.moves:
            m = rs.moves[-1]
            if m.kind == "consolidation":
                return 0
            return 1 if m.direction == "up" else -1
    if snap.move_snapshot.moves:
        m = snap.move_snapshot.moves[-1]
        if m.kind == "consolidation":
            return 0
        return 1 if m.direction == "up" else -1
    return 0


def _extract_max_level(snap) -> int:
    lvl = 1
    for rs in snap.recursive_snapshots:
        if rs.moves:
            lvl = max(lvl, rs.level_id)
    return lvl


def precompute_recursive(
    symbol: str,
    dates: list[str],
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> RecursiveCacheResult:
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id=symbol, max_levels=6, stroke_mode="wide")

    sigma_events: list[SigmaEvent] = []
    prev_sigma = 0
    max_level = 1

    for i in range(n):
        bar = Bar(ts=DUMMY_TS, open=opens[i], high=highs[i],
                  low=lows[i], close=closes[i], volume=0.0)
        snap = orch.process_bar(bar)

        sigma = _extract_sigma(snap)
        ml = _extract_max_level(snap)
        if ml > max_level:
            max_level = ml

        if sigma != prev_sigma:
            date_str = dates[i][:19] if dates else f"bar_{i}"
            sigma_events.append(SigmaEvent(bar_idx=i, date=date_str, sigma=sigma))
            prev_sigma = sigma

        if (i + 1) % 100_000 == 0:
            _log(f"      [{i/n*100:5.1f}%] {i+1:,}/{n:,}")

    date_first = dates[0][:19] if dates else "bar_0"
    date_last = dates[-1][:19] if dates else f"bar_{n-1}"

    return RecursiveCacheResult(
        symbol=symbol, n_bars=n,
        date_first=date_first, date_last=date_last,
        max_level=max_level, sigma_events=sigma_events,
        final_sigma=prev_sigma,
    )


def save_cache(result: RecursiveCacheResult) -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    data = {
        "symbol": result.symbol,
        "n_bars": result.n_bars,
        "date_first": result.date_first,
        "date_last": result.date_last,
        "max_level": result.max_level,
        "final_sigma": result.final_sigma,
        "sigma_events": [
            {"bar_idx": e.bar_idx, "date": e.date, "sigma": e.sigma}
            for e in result.sigma_events
        ],
    }
    path = CACHE_DIR / f"{result.symbol.lower()}_sigma.json"
    path.write_text(json.dumps(data, indent=2))


def load_cache(symbol: str) -> RecursiveCacheResult | None:
    path = CACHE_DIR / f"{symbol.lower()}_sigma.json"
    if not path.exists():
        return None
    data = json.loads(path.read_text())
    return RecursiveCacheResult(
        symbol=data["symbol"], n_bars=data["n_bars"],
        date_first=data["date_first"], date_last=data["date_last"],
        max_level=data["max_level"], final_sigma=data["final_sigma"],
        sigma_events=[
            SigmaEvent(bar_idx=e["bar_idx"], date=e["date"], sigma=e["sigma"])
            for e in data["sigma_events"]
        ],
    )


# ════════════════════════════════════════════════════════════
# Phase 2: K4 config tracker consuming precomputed sigma
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class K4Transition:
    date: str
    old_config: tuple[int, int, int]
    new_config: tuple[int, int, int]


def build_k4_config_timeline(
    es_result: RecursiveCacheResult,
    gc_result: RecursiveCacheResult,
    cl_result: RecursiveCacheResult,
    es_dates: list[str],
    gc_dates: list[str],
    cl_dates: list[str],
) -> tuple[list[tuple[str, tuple[int, int, int]]], list[K4Transition]]:
    """Build date → (sigma_e, sigma_au, sigma_oil) timeline.

    Merge sigma events from all three edges into a unified timeline sorted by date.
    """
    all_events: list[tuple[str, str, int]] = []
    for e in es_result.sigma_events:
        d = e.date if e.date.startswith("20") else es_dates[e.bar_idx][:19]
        all_events.append((d, "E", e.sigma))
    for e in gc_result.sigma_events:
        d = e.date if e.date.startswith("20") else gc_dates[e.bar_idx][:19]
        all_events.append((d, "Au", e.sigma))
    for e in cl_result.sigma_events:
        d = e.date if e.date.startswith("20") else cl_dates[e.bar_idx][:19]
        all_events.append((d, "Oil", e.sigma))

    all_events.sort(key=lambda x: x[0])

    current = {"E": 0, "Au": 0, "Oil": 0}
    timeline: list[tuple[str, tuple[int, int, int]]] = []
    transitions: list[K4Transition] = []

    for date_str, edge, sigma in all_events:
        old = (current["E"], current["Au"], current["Oil"])
        current[edge] = sigma
        new = (current["E"], current["Au"], current["Oil"])
        if new != old:
            transitions.append(K4Transition(date=date_str, old_config=old, new_config=new))
        timeline.append((date_str, new))

    return timeline, transitions


# ════════════════════════════════════════════════════════════
# Pool rules + date alignment
# ════════════════════════════════════════════════════════════

def _pool_rule(config: tuple[int, int, int], asset: str) -> frozenset[int]:
    sigma_e = config[0]
    if asset in ("QQQ", "HK700"):
        if sigma_e >= 0:
            return LONG_ONLY
        return NO_TRADE
    return BOTH


def build_bar_allowed_dirs(
    n_bars: int,
    asset: str,
    k4_timeline: list[tuple[str, tuple[int, int, int]]],
    has_dates: bool,
    asset_dates: list[str] | None,
    cal_trading_dates: list[str],
    cal_median_bpd: int,
    with_k4: bool,
) -> list[frozenset[int]]:
    if not with_k4:
        if asset == "QQQ":
            return [LONG_ONLY] * n_bars
        return [BOTH] * n_bars

    if not k4_timeline:
        return [BOTH] * n_bars

    k4_dates = [d for d, _ in k4_timeline]
    k4_configs = [c for _, c in k4_timeline]

    def _lookup(date_str: str) -> tuple[int, int, int]:
        idx = bisect.bisect_right(k4_dates, date_str) - 1
        if idx < 0:
            return (0, 0, 0)
        return k4_configs[idx]

    result: list[frozenset[int]] = []

    if has_dates and asset_dates:
        for i in range(n_bars):
            day = asset_dates[i][:19]
            cfg = _lookup(day)
            result.append(_pool_rule(cfg, asset))
    else:
        n_asset_days = n_bars // cal_median_bpd + 1
        total_cal = len(cal_trading_dates)
        start_offset = max(0, total_cal - n_asset_days)
        for i in range(n_bars):
            day_idx = start_offset + i // cal_median_bpd
            if day_idx >= total_cal:
                day_idx = total_cal - 1
            day = cal_trading_dates[day_idx]
            cfg = _lookup(day)
            result.append(_pool_rule(cfg, asset))

    return result


# ════════════════════════════════════════════════════════════
# Fast PH alive extraction
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
# FSM — D2 (no add-positions) + PH settle gate + cost reduction
# ════════════════════════════════════════════════════════════

def run_fsm(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
    bar_allowed_dirs: list[frozenset[int]],
) -> list[CompletedTrade]:
    n = len(closes)
    bi = BiEngine()
    tree_down = OnlineMergeTree(track_dominant=False)
    tree_up = OnlineMergeTree(track_dominant=False)

    l1_tree_down = OnlineMergeTree(track_dominant=False)
    l1_tree_up = OnlineMergeTree(track_dominant=False)

    prev_adj_bsps: list = []
    prev_n_strokes = 0
    bsp_seq = 0
    l1_prev_n_strokes = 0

    down_alive = _fast_alive(tree_down)
    up_alive = _fast_alive(tree_up)
    prev_down_r1: int | None = None
    prev_up_r1: int | None = None

    l1_down_alive = _fast_alive(l1_tree_down)
    l1_up_alive = _fast_alive(l1_tree_up)
    l1_prev_down_r1: int | None = None
    l1_prev_up_r1: int | None = None

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
    pending: dict | None = None
    entry_bsps: set[int] = set()
    diff_count = 0

    trades: list[CompletedTrade] = []
    last_log = 0

    def _pratio(alive: _Alive, rank: int) -> float:
        ab = alive.bars
        if len(ab) > rank and ab[0].persistence > 0:
            return ab[rank].persistence / ab[0].persistence
        return 0.0

    def _close_trade(idx: int, price: float, reason: str) -> None:
        nonlocal state, direction, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumul_recovered, own_capital
        nonlocal trim_active, pending, entry_bsps, diff_count
        eff_exit = price * (1 - FEE_RATE) if direction == 1 else price * (1 + FEE_RATE)
        pnl = (eff_exit - cost_basis) / entry_price * 100 * direction
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=idx, exit_price=price, direction=direction,
            pnl_pct=round(pnl, 4),
            n_short_diffs=diff_count,
            cumulative_recovered=cumul_recovered,
            principal_withdrawn=(state == St.PRINCIPAL_WITHDRAWN),
            exit_reason=reason,
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
        diff_count = 0

    for i in range(n):
        c = closes[i]
        bar = Bar(ts=DUMMY_TS, open=opens[i], high=highs[i], low=lows[i], close=c, volume=0.0)

        bi_snap = bi.process_bar(bar)

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
                    elif "Candidate" in nm or "Confirm" in nm:
                        (buy_cands if side == "buy" else sell_cands).append(bid)
                prev_adj_bsps = adj

        l1_non_r1_down = l1_non_r1_up = False
        strokes_now = bi_snap.strokes
        ns_now = len(strokes_now)
        if ns_now > l1_prev_n_strokes:
            for si in range(l1_prev_n_strokes, ns_now):
                ep = strokes_now[si].p1
                if ep <= 0:
                    continue
                l1_ds = l1_tree_down.update(ep)
                l1_us = l1_tree_up.update(-ep)
                if l1_ds:
                    r1_hit = False
                    if l1_prev_down_r1 is not None:
                        for mb in l1_ds:
                            if mb.birth_idx == l1_prev_down_r1:
                                r1_hit = True
                                break
                    if not r1_hit:
                        l1_non_r1_down = True
                    l1_down_alive = _fast_alive(l1_tree_down)
                    l1_prev_down_r1 = l1_down_alive.bars[1].birth_idx if len(l1_down_alive.bars) >= 2 else None
                if l1_us:
                    r1_hit = False
                    if l1_prev_up_r1 is not None:
                        for mb in l1_us:
                            if mb.birth_idx == l1_prev_up_r1:
                                r1_hit = True
                                break
                    if not r1_hit:
                        l1_non_r1_up = True
                    l1_up_alive = _fast_alive(l1_tree_up)
                    l1_prev_up_r1 = l1_up_alive.bars[1].birth_idx if len(l1_up_alive.bars) >= 2 else None
            l1_prev_n_strokes = ns_now

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

        allowed = bar_allowed_dirs[i]

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
            if pending and pending["side"] == "buy" and flip_long and 1 in allowed:
                direction = 1
                entered = True
            elif pending and pending["side"] == "sell" and flip_short and -1 in allowed:
                direction = -1
                entered = True

            if entered:
                state = St.POSITION_OPEN
                entry_price = c
                cost_basis = c * (1 + FEE_RATE) if direction == 1 else c * (1 - FEE_RATE)
                entry_bar = i
                total_shares = INITIAL_CAPITAL / c
                own_capital = INITIAL_CAPITAL
                cumul_recovered = 0.0
                entry_bsps = set(pending["ids"])
                pending = None
                trim_active = False
                diff_count = 0

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            should_stop = False
            stop_reason = ""
            if direction == 1 and flip_short:
                should_stop, stop_reason = True, "direction_flip"
            elif direction == -1 and flip_long:
                should_stop, stop_reason = True, "direction_flip"
            if not should_stop:
                s_inv = buy_inv if direction == 1 else sell_inv
                if any(b in entry_bsps for b in s_inv):
                    should_stop, stop_reason = True, "bsp_invalidate"

            if should_stop:
                _close_trade(i, c, stop_reason)
            else:
                cr_trigger_up = non_r1_up or l1_non_r1_up
                cr_trigger_down = non_r1_down or l1_non_r1_down

                def _best_ratio_up() -> float:
                    r0 = _pratio(up_alive, 1) if non_r1_up else 0.0
                    r1 = _pratio(l1_up_alive, 1) if l1_non_r1_up else 0.0
                    return max(r0, r1)

                def _best_ratio_down() -> float:
                    r0 = _pratio(down_alive, 1) if non_r1_down else 0.0
                    r1 = _pratio(l1_down_alive, 1) if l1_non_r1_down else 0.0
                    return max(r0, r1)

                if state == St.POSITION_OPEN:
                    do_trim = False
                    r = 0.0
                    if direction == 1 and cr_trigger_up:
                        r = _best_ratio_up()
                        do_trim = r > 0.05
                    elif direction == -1 and cr_trigger_down:
                        r = _best_ratio_down()
                        do_trim = r > 0.05
                    if do_trim and not trim_active:
                        trim_sell_price = c
                        trim_shares = total_shares * r
                        trim_active = True
                        state = St.COST_REDUCING
                elif state == St.COST_REDUCING:
                    if trim_active:
                        close_d = False
                        if direction == 1 and cr_trigger_down:
                            close_d = True
                        elif direction == -1 and cr_trigger_up:
                            close_d = True
                        if (buy_cands if direction == 1 else sell_cands):
                            close_d = True
                        if close_d:
                            if direction == 1:
                                profit = (trim_sell_price * (1 - FEE_RATE)
                                          - c * (1 + FEE_RATE)) * trim_shares
                            else:
                                profit = (c * (1 - FEE_RATE)
                                          - trim_sell_price * (1 + FEE_RATE)) * trim_shares
                            if total_shares > 0:
                                if direction == 1:
                                    cost_basis -= profit / total_shares
                                else:
                                    cost_basis += profit / total_shares
                                cumul_recovered += profit
                            diff_count += 1
                            trim_active = False
                            if cumul_recovered >= own_capital:
                                state = St.PRINCIPAL_WITHDRAWN
                            else:
                                state = St.POSITION_OPEN
                    else:
                        r = 0.0
                        do_new = False
                        if direction == 1 and cr_trigger_up:
                            r = _best_ratio_up()
                            do_new = r > 0.05
                        elif direction == -1 and cr_trigger_down:
                            r = _best_ratio_down()
                            do_new = r > 0.05
                        if do_new:
                            trim_sell_price = c
                            trim_shares = total_shares * r
                            trim_active = True
                elif state == St.PRINCIPAL_WITHDRAWN:
                    if not trim_active:
                        r = 0.0
                        do_pw = False
                        if direction == 1 and cr_trigger_up:
                            r = _best_ratio_up()
                            do_pw = r > 0.05
                        elif direction == -1 and cr_trigger_down:
                            r = _best_ratio_down()
                            do_pw = r > 0.05
                        if do_pw:
                            trim_sell_price = c
                            trim_shares = total_shares * r
                            trim_active = True
                    elif trim_active:
                        close_d = False
                        if direction == 1 and cr_trigger_down:
                            close_d = True
                        elif direction == -1 and cr_trigger_up:
                            close_d = True
                        if (buy_cands if direction == 1 else sell_cands):
                            close_d = True
                        if close_d:
                            if direction == 1:
                                profit = (trim_sell_price * (1 - FEE_RATE)
                                          - c * (1 + FEE_RATE)) * trim_shares
                            else:
                                profit = (c * (1 - FEE_RATE)
                                          - trim_sell_price * (1 + FEE_RATE)) * trim_shares
                            if total_shares > 0:
                                if direction == 1:
                                    cost_basis -= profit / total_shares
                                else:
                                    cost_basis += profit / total_shares
                                cumul_recovered += profit
                            diff_count += 1
                            trim_active = False

        elif state == St.STOPPED_OUT:
            state = St.SCANNING

        if i - last_log >= 100_000:
            _log(f"      [{i/n*100:5.1f}%] {i:,}/{n:,}")
            last_log = i

    if state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
        _close_trade(n - 1, closes[-1], "eod_close")

    return trades


# ════════════════════════════════════════════════════════════
# Metrics
# ════════════════════════════════════════════════════════════

def compute_metrics(trades: list[CompletedTrade]) -> dict:
    if not trades:
        return {"n": 0, "win_rate": 0.0, "avg_pnl": 0.0,
                "total_compound": 0.0, "max_dd": 0.0,
                "avg_hold_bars": 0, "n_with_cr": 0, "n_pw": 0,
                "long_n": 0, "short_n": 0}
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
        "n_with_cr": sum(1 for t in trades if t.n_short_diffs > 0),
        "n_pw": sum(1 for t in trades if t.principal_withdrawn),
        "long_n": sum(1 for t in trades if t.direction == 1),
        "short_n": sum(1 for t in trades if t.direction == -1),
    }


def portfolio_compound(results: list[AssetResult]) -> float:
    if not results:
        return 0.0
    return sum(r.metrics["total_compound"] for r in results) / len(results)


# ════════════════════════════════════════════════════════════
# Report
# ════════════════════════════════════════════════════════════

def _s(v: int) -> str:
    return {1: "↑", 0: "─", -1: "↓"}.get(v, "?")


def write_report(
    k4_transitions: list[K4Transition],
    es_cache: RecursiveCacheResult,
    gc_cache: RecursiveCacheResult,
    cl_cache: RecursiveCacheResult,
    results_k4: list[AssetResult],
    results_base: list[AssetResult],
    total_elapsed: float,
) -> None:
    L: list[str] = []
    L.append("# 全市场端到端回测 v2 — K4期货1min递归 + M1操盘\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}")
    L.append(f"总耗时：{total_elapsed:.1f}s\n")

    L.append("## 架构\n")
    L.append("- **K4 配置**：ES/GC/CL 期货 1min → RecursiveOrchestrator → 最高级别 Move direction → σ")
    L.append(f"- **操盘**：各标的 1min → D2 FSM（无加仓 + PH settle gate + L0/L1多级别降成本 + {FEE_RATE*100:.1f}%手续费）")
    L.append("- **两阶段**：阶段1预计算递归σ序列 → 阶段2消费结果做组合回测\n")

    L.append("## K4 三边递归摘要\n")
    L.append("| 边 | 标的 | Bars | 最高级别 | σ 切换次数 | 最终 σ |")
    L.append("|-----|------|------|---------|-----------|--------|")
    for edge, cache in [("E/$", es_cache), ("Au/$", gc_cache), ("Oil/$", cl_cache)]:
        L.append(f"| {edge} | {cache.symbol} | {cache.n_bars:,} "
                 f"| L{cache.max_level} | {len(cache.sigma_events)} "
                 f"| {_s(cache.final_sigma)} |")
    L.append("")

    L.append("## K4 配置变化\n")
    L.append(f"总切换次数：{len(k4_transitions)}\n")
    if k4_transitions:
        L.append("| 时间 | 旧配置 | 新配置 | σ(E/$) | σ(Au/$) | σ(Oil/$) |")
        L.append("|------|--------|--------|--------|---------|----------|")
        shown = k4_transitions[-40:] if len(k4_transitions) > 40 else k4_transitions
        if len(k4_transitions) > 40:
            L.append(f"| ... | ... | ... | ... | ... | (前{len(k4_transitions)-40}条省略) |")
        for t in shown:
            old = f"({_s(t.old_config[0])},{_s(t.old_config[1])},{_s(t.old_config[2])})"
            new = f"({_s(t.new_config[0])},{_s(t.new_config[1])},{_s(t.new_config[2])})"
            L.append(f"| {t.date} | {old} | {new} "
                     f"| {_s(t.new_config[0])} | {_s(t.new_config[1])} | {_s(t.new_config[2])} |")
    L.append("")

    for mode_label, results in [("WITH K4", results_k4), ("WITHOUT K4", results_base)]:
        L.append(f"## {mode_label}\n")
        L.append("| 标的 | Bars | BH% | FSM% | 胜率 | 交易 | 多/空 | 降成本 | PW | MDD | 耗时 |")
        L.append("|------|------|-----|------|------|------|-------|--------|-----|-----|------|")
        for r in results:
            m = r.metrics
            n_t = m["n"] or 1
            L.append(
                f"| {r.symbol} | {r.n_bars:,} | {r.bh_pct:+.1f} "
                f"| {m['total_compound']:+.2f} | {m['win_rate']:.0f}% "
                f"| {m['n']} | {m['long_n']}/{m['short_n']} "
                f"| {m['n_with_cr']}/{n_t} | {m['n_pw']}/{n_t} "
                f"| {m['max_dd']:.1f}% | {r.elapsed:.0f}s |"
            )
        port = portfolio_compound(results)
        bh_avg = sum(r.bh_pct for r in results) / len(results) if results else 0
        L.append(f"| **组合** | — | {bh_avg:+.1f} | {port:+.2f} | — | — | — | — | — | — | — |")
        L.append("")

        for r in results:
            if r.metrics["n"] == 0:
                continue
            L.append(f"### {r.symbol} ({mode_label})\n")
            L.append(f"- Bars: **{r.n_bars:,}**, Price: {r.first_price:.2f} → {r.last_price:.2f}")
            L.append(f"- BH: **{r.bh_pct:+.2f}%**, FSM: **{r.metrics['total_compound']:+.2f}%**\n")
            L.append("| # | Dir | Entry | Exit | Hold | PnL% | CR | PW | Reason |")
            L.append("|---|-----|-------|------|------|------|-----|-----|--------|")
            for idx, t in enumerate(r.trades[:20]):
                d = "L" if t.direction == 1 else "S"
                pw = "Y" if t.principal_withdrawn else ""
                L.append(f"| {idx+1} | {d} | {t.entry_price:.2f}@{t.entry_bar:,} "
                         f"| {t.exit_price:.2f}@{t.exit_bar:,} "
                         f"| {t.exit_bar-t.entry_bar:,} | {t.pnl_pct:+.2f} "
                         f"| {t.n_short_diffs} | {pw} | {t.exit_reason} |")
            if len(r.trades) > 20:
                L.append(f"| ... | ... | ... | ... | ... | ... | ... | ... | (共{len(r.trades)}笔) |")
            L.append("")

    L.append("## K4 选股效果对比\n")
    L.append("| 标的 | WITHOUT K4 | WITH K4 | 差异 | 效果 |")
    L.append("|------|-----------|---------|------|------|")
    for rb, rk in zip(results_base, results_k4):
        mb, mk = rb.metrics, rk.metrics
        diff = mk["total_compound"] - mb["total_compound"]
        eff = "改善" if diff > 0 else "恶化" if diff < 0 else "无变化"
        L.append(f"| {rb.symbol} | {mb['total_compound']:+.2f}% ({mb['n']}笔) "
                 f"| {mk['total_compound']:+.2f}% ({mk['n']}笔) "
                 f"| {diff:+.2f}% | {eff} |")
    port_base = portfolio_compound(results_base)
    port_k4 = portfolio_compound(results_k4)
    diff = port_k4 - port_base
    eff = "改善" if diff > 0 else "恶化" if diff < 0 else "无变化"
    L.append(f"| **组合** | {port_base:+.2f}% | {port_k4:+.2f}% | {diff:+.2f}% | {eff} |")
    L.append("")

    L.append("## 结果包\n")
    L.append("**结论**：K4 期货 1min 递归 + D2 FSM 全市场端到端回测。\n")
    L.append("**定义依据**：")
    L.append("- K4 σ：RecursiveOrchestrator 1min 递归最高级别 Move direction")
    L.append("- FSM：267号 D2（无加仓）+ PH settle gate + 降成本")
    L.append("- 数据：ES/GC/CL 期货（Databento）+ QQQ/OKLO/HK700/BTC 1min\n")
    L.append("**边界条件**：")
    L.append("- QQQ/OKLO 无原始日期，通过 ES 交易日历推断")
    L.append("- GC 用手动 roll（前月合约拼接），roll 边界可能有价格跳跃")
    L.append("- 无滑点/手续费，等权分配\n")
    L.append("**谱系引用**：267号(FSM), config_space(K4), 526号(递归存在论区分)。\n")
    L.append("**影响声明**：新建回测脚本和报告，不修改引擎代码。\n")
    L.append("**认识论等级**：L2。")

    OUTPUT_MD.write_text("\n".join(L))
    _log(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# Main
# ════════════════════════════════════════════════════════════

K4_EDGES = [
    ("ES", "es_1m_databento.json"),
    ("GC", "gc_1m_databento.json"),
    ("CL", "cl_1m_databento.json"),
]

TRADE_ASSETS = [
    ("QQQ", "qqq_1m_databento_full.json", "columnar", False),
    ("OKLO", "oklo_1m_databento_full.json", "columnar", False),
    ("HK700", "hk700_1m_tws.json", "row", True),
    ("BTC", "btc_1m_3year.json", "columnar", True),
]

K4_AFFECTED = {"QQQ", "HK700"}


def main() -> None:
    t0 = time.time()

    _log("=" * 60)
    _log("  全市场端到端回测 v2：K4 期货 1min 递归 + M1 操盘")
    _log("=" * 60)

    # ═══════════════════════════════════════════════════════
    # Phase 1: Precompute recursive sigma for K4 edges
    # ═══════════════════════════════════════════════════════
    _log("\n[Phase 1] K4 三边 1min 递归预计算 ...")

    k4_caches: dict[str, RecursiveCacheResult] = {}
    k4_dates: dict[str, list[str]] = {}

    for sym, fn in K4_EDGES:
        cached = load_cache(sym)
        if cached:
            _log(f"  {sym}: 使用缓存 (L{cached.max_level}, {len(cached.sigma_events)} σ events)")
            k4_caches[sym] = cached
            raw = json.loads((DATA_DIR / fn).read_text())
            k4_dates[sym] = raw.get("dates", [])
            continue

        _log(f"\n  ── {sym} 递归 ──")
        _, dates, opens, highs, lows, closes = load_columnar(fn)
        _log(f"    {len(closes):,} bars, {dates[0][:10]} → {dates[-1][:10]}")
        k4_dates[sym] = dates

        t1 = time.time()
        result = precompute_recursive(sym, dates, opens, highs, lows, closes)
        elapsed = time.time() - t1

        save_cache(result)
        k4_caches[sym] = result
        _log(f"    完成：{elapsed:.1f}s, L{result.max_level}, "
             f"{len(result.sigma_events)} σ events, "
             f"final σ={_s(result.final_sigma)}")

        del dates, opens, highs, lows, closes
        garbage_collect.collect()

    es_cache = k4_caches["ES"]
    gc_cache = k4_caches["GC"]
    cl_cache = k4_caches["CL"]

    # ═══════════════════════════════════════════════════════
    # Build K4 config timeline
    # ═══════════════════════════════════════════════════════
    _log("\n[Phase 1b] 构建 K4 配置时间线 ...")

    k4_timeline, k4_transitions = build_k4_config_timeline(
        es_cache, gc_cache, cl_cache,
        k4_dates["ES"], k4_dates["GC"], k4_dates["CL"],
    )
    _log(f"  配置切换次数：{len(k4_transitions)}")
    if k4_transitions:
        last = k4_transitions[-1]
        _log(f"  最新配置：({_s(last.new_config[0])},{_s(last.new_config[1])},{_s(last.new_config[2])})")

    # Build calendar from ES dates for no-date assets
    es_day_strs = [d[:10] for d in k4_dates["ES"]]
    es_bpd = Counter(es_day_strs)
    cal_trading_dates = sorted(es_bpd.keys())
    cal_vals = sorted(es_bpd.values())
    cal_median_bpd = cal_vals[len(cal_vals) // 2]
    _log(f"  交易日历：{len(cal_trading_dates)} days, 中位 {cal_median_bpd} bars/day")

    # ═══════════════════════════════════════════════════════
    # Phase 2: Per-asset FSM backtests
    # ═══════════════════════════════════════════════════════
    _log("\n[Phase 2] 逐标的 FSM 回测 ...")

    results_k4: list[AssetResult] = []
    results_base: list[AssetResult] = []

    for sym, fn, fmt, has_dates in TRADE_ASSETS:
        _log(f"\n  加载 {sym} ...")
        if fmt == "columnar":
            _, dates, opens, highs, lows, closes = load_columnar(fn)
        else:
            _, dates, opens, highs, lows, closes = load_row_based(fn, sym)

        if sym == "HK700":
            idx = 0
            for i, d in enumerate(dates):
                if d[:10] >= "2024-01-01":
                    idx = i
                    break
            if idx > 0:
                old_n = len(closes)
                dates, opens, highs, lows, closes = (
                    dates[idx:], opens[idx:], highs[idx:], lows[idx:], closes[idx:],
                )
                _log(f"  HK700 truncated: {old_n:,} → {len(closes):,} (from 2024)")

        n = len(closes)
        bh = (closes[-1] - closes[0]) / closes[0] * 100
        _log(f"  {sym}: {n:,} bars, {closes[0]:.2f} → {closes[-1]:.2f}")

        k4_affects = sym in K4_AFFECTED
        modes = ["k4", "baseline"] if k4_affects else ["both"]

        for mode in modes:
            with_k4 = mode == "k4"
            label = {"k4": "WITH K4", "baseline": "WITHOUT K4", "both": "BOTH=BASELINE"}.get(mode, mode)
            _log(f"\n  ── {sym} ({label}) ──")

            bar_dirs = build_bar_allowed_dirs(
                n_bars=n, asset=sym, k4_timeline=k4_timeline,
                has_dates=has_dates and bool(dates), asset_dates=dates if has_dates else None,
                cal_trading_dates=cal_trading_dates, cal_median_bpd=cal_median_bpd,
                with_k4=with_k4,
            )

            t1 = time.time()
            trades = run_fsm(opens, highs, lows, closes, bar_dirs)
            elapsed = time.time() - t1

            metrics = compute_metrics(trades)
            result = AssetResult(
                symbol=sym, n_bars=n,
                first_price=closes[0], last_price=closes[-1],
                bh_pct=bh, trades=trades, metrics=metrics,
                elapsed=elapsed, mode=mode,
            )

            if mode == "both":
                results_k4.append(result)
                results_base.append(result)
            elif with_k4:
                results_k4.append(result)
            else:
                results_base.append(result)

            m = metrics
            _log(f"    {elapsed:.1f}s ({n/max(elapsed,0.01):.0f} bars/s)")
            _log(f"    交易：{m['n']}笔 (多{m['long_n']}/空{m['short_n']})")
            _log(f"    FSM: {m['total_compound']:+.2f}%, BH: {bh:+.2f}%")
            _log(f"    胜率：{m['win_rate']:.1f}%, MDD: {m['max_dd']:.1f}%")

        del dates, opens, highs, lows, closes
        garbage_collect.collect()

    total_elapsed = time.time() - t0

    write_report(k4_transitions, es_cache, gc_cache, cl_cache,
                 results_k4, results_base, total_elapsed)

    _log("\n" + "=" * 60)
    _log("  组合汇总")
    _log("=" * 60)
    port_k4 = portfolio_compound(results_k4)
    port_base = portfolio_compound(results_base)
    bh_avg = sum(r.bh_pct for r in results_k4) / len(results_k4) if results_k4 else 0
    _log(f"  等权 BH:    {bh_avg:+.2f}%")
    _log(f"  WITHOUT K4: {port_base:+.2f}%")
    _log(f"  WITH K4:    {port_k4:+.2f}%")
    _log(f"  K4 增量:    {port_k4 - port_base:+.2f}%")
    _log(f"  总耗时:     {total_elapsed:.1f}s")


if __name__ == "__main__":
    main()
