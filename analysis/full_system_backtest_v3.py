"""全市场端到端回测 v3 — 期货保证金+固定佣金 / BTC 10x杠杆 / 波动率分层降成本 / 比价递归选股。

四项改进：
  1. ES期货替代QQQ：固定佣金$2.25/手/边，$15K保证金/手，最多6手
  2. BTC 10x杠杆：0.1%手续费，5%强平线
  3. 降成本分层：高波动(ES/OKLO/BTC)=full L0+L1，低波动(HK700)=conservative L1 only
  4. 比价递归选股：K4 σ推导比价方向，标注相对强度

认识论等级：L2（真实数据，含否定性结果）。
"""

from __future__ import annotations

import bisect
import gc as garbage_collect
import json
import math
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
from newchan.types import Bar

DATA_DIR = ROOT / "analysis" / "data_cache"
CACHE_DIR = DATA_DIR / "recursive_cache"
OUTPUT_MD = ROOT / "analysis" / "full_system_backtest_v3.md"

INITIAL_CAPITAL = 100_000.0
BSP_WINDOW = 500
BSP_MARGIN = 50
LEVEL_ID = 1
PENDING_EXPIRY = 390

BOTH = frozenset({1, -1})
LONG_ONLY = frozenset({1})
NO_TRADE = frozenset()


def _log(msg: str) -> None:
    print(msg)
    sys.stdout.flush()


# ════════════════════════════════════════════════════════════
# Asset configuration
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class AssetConfig:
    symbol: str
    filename: str
    fmt: str
    has_dates: bool
    asset_type: str         # "futures", "stock", "crypto"
    fee_rate: float         # percentage per side (stock/crypto)
    fixed_comm: float       # $ per contract per side (futures)
    point_value: float      # $ per point (futures)
    max_contracts: int      # max contracts (futures)
    margin_per_contract: float
    leverage: float         # 1.0 = no leverage
    liq_move: float         # adverse price move fraction triggering liquidation (0=none)
    cr_mode: str            # "full" (L0+L1) or "conservative" (L1 only)
    cr_threshold: float     # PH ratio threshold for CR trim
    k4_affected: bool
    default_dirs: frozenset[int]
    truncate_from: str | None  # date string to truncate data from (e.g., "2024-01-01")


ASSETS = [
    AssetConfig("ES", "es_1m_databento.json", "columnar", True,
                "futures", 0.0, 2.25, 50.0, 6, 15_000.0,
                1.0, 0.0, "full", 0.05, True, LONG_ONLY, None),
    AssetConfig("OKLO", "oklo_1m_databento_full.json", "columnar", False,
                "stock", 0.001, 0.0, 1.0, 0, 0.0,
                1.0, 0.0, "full", 0.05, False, BOTH, None),
    AssetConfig("BTC", "btc_1m_3year.json", "columnar", True,
                "crypto", 0.001, 0.0, 1.0, 0, 0.0,
                10.0, 0.05, "full", 0.05, False, BOTH, None),
    AssetConfig("HK700", "hk700_1m_tws.json", "row", True,
                "stock", 0.001, 0.0, 1.0, 0, 0.0,
                1.0, 0.0, "conservative", 0.10, True, LONG_ONLY, "2024-01-01"),
]

K4_EDGES = [
    ("ES", "es_1m_databento.json"),
    ("GC", "gc_1m_databento.json"),
    ("CL", "cl_1m_databento.json"),
]


# ════════════════════════════════════════════════════════════
# Data types
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class SigmaEvent:
    bar_idx: int
    date: str
    sigma: int

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
    cumulative_cr_dollars: float
    principal_withdrawn: bool
    exit_reason: str
    n_earn_shares: int = 0          # 挣股数阶段增持次数
    earn_notional: float = 0.0      # 挣股数阶段累计额外仓位名义额($)

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
    config: AssetConfig


class St(Enum):
    SCANNING = auto()
    POSITION_OPEN = auto()
    COST_REDUCING = auto()
    PRINCIPAL_WITHDRAWN = auto()
    STOPPED_OUT = auto()


# ════════════════════════════════════════════════════════════
# PnL helpers
# ════════════════════════════════════════════════════════════

def _trade_pnl_pct(
    cfg: AssetConfig,
    entry_price: float,
    exit_price: float,
    direction: int,
    cumul_cr_dollars: float,
) -> float:
    if cfg.asset_type == "futures":
        gross = direction * (exit_price - entry_price) * cfg.max_contracts * cfg.point_value
        comm = 2.0 * cfg.max_contracts * cfg.fixed_comm
        net = gross - comm + cumul_cr_dollars
    else:
        notional = INITIAL_CAPITAL * cfg.leverage
        gross = direction * (exit_price - entry_price) / entry_price * notional
        comm = 2.0 * cfg.fee_rate * notional
        net = gross - comm + cumul_cr_dollars
    return net / INITIAL_CAPITAL * 100.0


def _cr_profit_dollars(
    cfg: AssetConfig,
    direction: int,
    sell_price: float,
    buy_price: float,
    ratio: float,
) -> float:
    """降成本 trim 单次往返的真实美元盈亏（可正可负，区分多空方向）。

    审计 A1/A2 修复：不再对结果做 ``max(0.0, ...)`` 截断——亏损的 trim
    照实扣减。多头 (高抛低吸)：realized = sell_price - buy_price；空头 (回补
    再卖)：realized = buy_price - sell_price = 再卖价 - 回补价。两个方向不变式
    一致：**高卖低买为正，高买低卖为负**（A2 用户裁定）。截断移除后，价格逆向
    运动导致的亏损 trim 会真实减少 cumul_cr_dollars。
    """
    if cfg.asset_type == "futures":
        trim_units = cfg.max_contracts * ratio
        if direction == 1:
            profit = (sell_price - buy_price) * trim_units * cfg.point_value
        else:
            profit = (buy_price - sell_price) * trim_units * cfg.point_value
        comm = 2.0 * trim_units * cfg.fixed_comm
    else:
        notional = INITIAL_CAPITAL * cfg.leverage
        trim_val = notional * ratio
        if direction == 1:
            profit = (sell_price - buy_price) / sell_price * trim_val
        else:
            profit = (buy_price - sell_price) / sell_price * trim_val
        comm = 2.0 * cfg.fee_rate * trim_val
    return profit - comm


def _effective_leverage(cfg: AssetConfig, price: float) -> float:
    if cfg.asset_type == "futures":
        return cfg.max_contracts * cfg.point_value * price / INITIAL_CAPITAL
    return cfg.leverage


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
# Phase 1: Precompute recursive structure
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
        "symbol": result.symbol, "n_bars": result.n_bars,
        "date_first": result.date_first, "date_last": result.date_last,
        "max_level": result.max_level, "final_sigma": result.final_sigma,
        "sigma_events": [
            {"bar_idx": e.bar_idx, "date": e.date, "sigma": e.sigma}
            for e in result.sigma_events
        ],
    }
    (CACHE_DIR / f"{result.symbol.lower()}_sigma.json").write_text(json.dumps(data, indent=2))


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
# K4 config tracker + ratio score
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


def ratio_score(config: tuple[int, int, int]) -> int:
    """Derive equity relative strength from K4 config.

    Score 0-3: how many non-equity edges are NOT rising (i.e., equity is outperforming).
    Higher = equity is relatively stronger.
    """
    sigma_e, sigma_au, sigma_oil = config
    score = 0
    if sigma_au <= 0:
        score += 1
    if sigma_oil <= 0:
        score += 1
    if sigma_e > 0:
        score += 1
    return score


# ════════════════════════════════════════════════════════════
# Pool rules + date alignment
# ════════════════════════════════════════════════════════════

def _pool_rule(config: tuple[int, int, int], cfg: AssetConfig) -> frozenset[int]:
    sigma_e = config[0]
    if cfg.k4_affected:
        if sigma_e >= 0:
            return cfg.default_dirs
        return NO_TRADE
    return cfg.default_dirs


def build_bar_allowed_dirs(
    n_bars: int,
    cfg: AssetConfig,
    k4_timeline: list[tuple[str, tuple[int, int, int]]],
    asset_dates: list[str] | None,
    cal_trading_dates: list[str],
    cal_median_bpd: int,
    with_k4: bool,
) -> list[frozenset[int]]:
    if not with_k4:
        return [cfg.default_dirs] * n_bars
    if not k4_timeline:
        return [cfg.default_dirs] * n_bars

    k4_dates = [d for d, _ in k4_timeline]
    k4_configs = [c for _, c in k4_timeline]

    def _lookup(date_str: str) -> tuple[int, int, int]:
        idx = bisect.bisect_right(k4_dates, date_str) - 1
        return k4_configs[idx] if idx >= 0 else (0, 0, 0)

    result: list[frozenset[int]] = []
    if cfg.has_dates and asset_dates:
        for i in range(n_bars):
            day = asset_dates[i][:19]
            result.append(_pool_rule(_lookup(day), cfg))
    else:
        n_asset_days = n_bars // cal_median_bpd + 1
        total_cal = len(cal_trading_dates)
        start_offset = max(0, total_cal - n_asset_days)
        for i in range(n_bars):
            day_idx = min(start_offset + i // cal_median_bpd, total_cal - 1)
            result.append(_pool_rule(_lookup(cal_trading_dates[day_idx]), cfg))
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
# FSM — D2 + PH settle gate + asset-specific commission/leverage
# ════════════════════════════════════════════════════════════

def run_fsm(
    cfg: AssetConfig,
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
    bar_allowed_dirs: list[frozenset[int]],
    stop_pct_of_margin: float = 0.0,
) -> list[CompletedTrade]:
    # stop_pct_of_margin: 单笔亏损达到保证金的此比例时强制平仓 (0=禁用)。
    # 保证金 = INITIAL_CAPITAL，名义 = INITIAL_CAPITAL*leverage，
    # 故 保证金损失比例 = adverse_move * leverage → 触发反向移动 = stop_pct/leverage。
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
    entry_price = 0.0
    entry_bar = -1
    cumul_cr_dollars = 0.0
    trim_active = False
    trim_sell_price = 0.0
    trim_ratio = 0.0
    pending: dict | None = None
    entry_bsps: set[int] = set()
    diff_count = 0
    earn_legs: list[tuple[float, float]] = []   # 挣股数阶段额外仓位 (notional$, 买入价)
    earn_count = 0

    trades: list[CompletedTrade] = []
    last_log = 0
    use_l0 = cfg.cr_mode == "full"

    def _pratio(alive: _Alive, rank: int) -> float:
        ab = alive.bars
        if len(ab) > rank and ab[0].persistence > 0:
            return ab[rank].persistence / ab[0].persistence
        return 0.0

    def _earn_pnl_dollars(exit_price: float) -> float:
        """挣股数阶段额外仓位在退出时的美元盈亏（百分比收益近似，区分多空）。"""
        total = 0.0
        for notional, buy_px in earn_legs:
            if buy_px > 0:
                total += direction * (exit_price - buy_px) / buy_px * notional
        return total

    def _close_trade(idx: int, price: float, reason: str) -> None:
        nonlocal state, direction, entry_price, entry_bar
        nonlocal cumul_cr_dollars, trim_active, pending, entry_bsps, diff_count
        nonlocal earn_legs, earn_count
        # 全额：基础仓位 pnl + 已回收降成本现金 + 挣股数额外仓位 pnl
        pnl = _trade_pnl_pct(cfg, entry_price, price, direction, cumul_cr_dollars)
        pnl += _earn_pnl_dollars(price) / INITIAL_CAPITAL * 100.0
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=idx, exit_price=price, direction=direction,
            pnl_pct=round(pnl, 4),
            n_short_diffs=diff_count,
            cumulative_cr_dollars=round(cumul_cr_dollars, 2),
            principal_withdrawn=(state == St.PRINCIPAL_WITHDRAWN),
            exit_reason=reason,
            n_earn_shares=earn_count,
            earn_notional=round(sum(p for p, _ in earn_legs), 2),
        ))
        state = St.SCANNING
        direction = 0
        entry_price = 0.0
        entry_bar = -1
        cumul_cr_dollars = 0.0
        trim_active = False
        pending = None
        entry_bsps = set()
        diff_count = 0
        earn_legs = []
        earn_count = 0

    def _check_liquidation(idx: int, c: float, h: float, l: float) -> bool:
        if cfg.liq_move <= 0:
            return False
        worst = l if direction == 1 else h
        adverse_move = -direction * (worst - entry_price) / entry_price
        if adverse_move >= cfg.liq_move:
            liq_price = entry_price * (1.0 - direction * cfg.liq_move)
            _close_trade(idx, liq_price, "liquidation")
            return True
        return False

    # 止损反向移动阈值：保证金损失比例 stop_pct → adverse_move = stop_pct/leverage。
    stop_move = (stop_pct_of_margin / cfg.leverage) if stop_pct_of_margin > 0 else 0.0

    def _check_stop(idx: int, h: float, l: float) -> bool:
        if stop_move <= 0:
            return False
        worst = l if direction == 1 else h
        adverse_move = -direction * (worst - entry_price) / entry_price
        if adverse_move >= stop_move:
            stop_price = entry_price * (1.0 - direction * stop_move)
            _close_trade(idx, stop_price, "stop_loss")
            return True
        return False

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

        # L1 PH (stroke-level) updates
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

        # L0 PH (bar-level) updates
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

        # CR trigger: full mode uses L0+L1, conservative uses L1 only
        if use_l0:
            cr_trigger_up = non_r1_up or l1_non_r1_up
            cr_trigger_down = non_r1_down or l1_non_r1_down
        else:
            cr_trigger_up = l1_non_r1_up
            cr_trigger_down = l1_non_r1_down

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
                entry_bar = i
                cumul_cr_dollars = 0.0
                entry_bsps = set(pending["ids"])
                pending = None
                trim_active = False
                diff_count = 0

        elif state in (St.POSITION_OPEN, St.COST_REDUCING, St.PRINCIPAL_WITHDRAWN):
            # 止损先于强平检查：止损阈值 (<50%保证金) 永远比强平 (50%保证金) 更紧。
            if _check_stop(i, highs[i], lows[i]):
                continue
            if _check_liquidation(i, c, highs[i], lows[i]):
                continue

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
                def _best_ratio_up() -> float:
                    r0 = _pratio(up_alive, 1) if (use_l0 and non_r1_up) else 0.0
                    r1 = _pratio(l1_up_alive, 1) if l1_non_r1_up else 0.0
                    return max(r0, r1)

                def _best_ratio_down() -> float:
                    r0 = _pratio(down_alive, 1) if (use_l0 and non_r1_down) else 0.0
                    r1 = _pratio(l1_down_alive, 1) if l1_non_r1_down else 0.0
                    return max(r0, r1)

                def _try_open_trim() -> bool:
                    nonlocal trim_active, trim_sell_price, trim_ratio, state
                    r = 0.0
                    do_trim = False
                    if direction == 1 and cr_trigger_up:
                        r = _best_ratio_up()
                        do_trim = r > cfg.cr_threshold
                    elif direction == -1 and cr_trigger_down:
                        r = _best_ratio_down()
                        do_trim = r > cfg.cr_threshold
                    if do_trim and not trim_active:
                        trim_sell_price = c
                        trim_ratio = r
                        trim_active = True
                        if state == St.POSITION_OPEN:
                            state = St.COST_REDUCING
                        return True
                    return False

                def _try_close_trim() -> None:
                    nonlocal trim_active, cumul_cr_dollars, diff_count, state
                    nonlocal earn_legs, earn_count
                    close_d = False
                    if direction == 1 and cr_trigger_down:
                        close_d = True
                    elif direction == -1 and cr_trigger_up:
                        close_d = True
                    if (buy_cands if direction == 1 else sell_cands):
                        close_d = True
                    if close_d:
                        profit = _cr_profit_dollars(
                            cfg, direction, trim_sell_price, c, trim_ratio)
                        diff_count += 1
                        trim_active = False
                        if state == St.PRINCIPAL_WITHDRAWN:
                            # 挣股数阶段（53/81课）：成本已归零，短差利润不再回收为现金，
                            # 而是按当前价 c 买入额外仓位（notional=profit），退出时计入。
                            earn_legs.append((profit, c))
                            earn_count += 1
                        else:
                            cumul_cr_dollars += profit
                            if cumul_cr_dollars >= INITIAL_CAPITAL:
                                state = St.PRINCIPAL_WITHDRAWN  # 成本归零→挣股数
                            elif state == St.COST_REDUCING:
                                state = St.POSITION_OPEN

                if state == St.POSITION_OPEN:
                    _try_open_trim()
                elif state == St.COST_REDUCING:
                    if trim_active:
                        _try_close_trim()
                    else:
                        _try_open_trim()
                elif state == St.PRINCIPAL_WITHDRAWN:
                    if trim_active:
                        _try_close_trim()
                    else:
                        _try_open_trim()

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
                "long_n": 0, "short_n": 0, "n_liq": 0, "n_stop": 0,
                "n_earn": 0}
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
        "n_liq": sum(1 for t in trades if t.exit_reason == "liquidation"),
        "n_stop": sum(1 for t in trades if t.exit_reason == "stop_loss"),
        "n_earn": sum(1 for t in trades if t.n_earn_shares > 0),
    }


def portfolio_compound(results: list[AssetResult]) -> float:
    if not results:
        return 0.0
    return sum(r.metrics["total_compound"] for r in results) / len(results)


# ════════════════════════════════════════════════════════════
# OKLO anomaly analysis
# ════════════════════════════════════════════════════════════

def analyze_oklo_trades(trades: list[CompletedTrade]) -> list[str]:
    if not trades:
        return ["无 OKLO 交易数据。"]
    L: list[str] = []
    L.append("### OKLO 交易异常值分析\n")

    pnls = [t.pnl_pct for t in trades]
    abs_pnls = [abs(p) for p in pnls]
    mean_abs = sum(abs_pnls) / len(abs_pnls)
    std_abs = (sum((x - mean_abs) ** 2 for x in abs_pnls) / len(abs_pnls)) ** 0.5
    threshold = mean_abs + 3 * std_abs

    outliers = [(i, t) for i, t in enumerate(trades) if abs(t.pnl_pct) > threshold]
    big_cr = [(i, t) for i, t in enumerate(trades) if t.n_short_diffs > 50]

    L.append(f"- 总交易数：{len(trades)}")
    L.append(f"- PnL 均值：{sum(pnls)/len(pnls):+.2f}%，|PnL| 均值：{mean_abs:.2f}%，标准差：{std_abs:.2f}%")
    L.append(f"- 异常阈值（μ+3σ）：{threshold:.2f}%")
    L.append(f"- 异常交易数（|PnL|>{threshold:.1f}%）：{len(outliers)}\n")

    if outliers:
        L.append("| # | Dir | Entry→Exit | Hold | PnL% | CR次数 | 判定 |")
        L.append("|---|-----|-----------|------|------|--------|------|")
        for idx, t in outliers:
            d = "L" if t.direction == 1 else "S"
            hold = t.exit_bar - t.entry_bar
            if hold > 1000 and t.n_short_diffs > 50:
                verdict = "CR驱动，高波动真实"
            elif abs(t.pnl_pct) > 50:
                verdict = "需人工检查"
            else:
                verdict = "高波动真实"
            L.append(f"| {idx+1} | {d} | {t.entry_price:.2f}→{t.exit_price:.2f} "
                     f"| {hold:,} | {t.pnl_pct:+.2f} | {t.n_short_diffs} | {verdict} |")
    L.append("")

    if big_cr:
        L.append(f"**高 CR 次数交易**（>50次）：{len(big_cr)} 笔\n")
        for idx, t in big_cr[:5]:
            d = "L" if t.direction == 1 else "S"
            L.append(f"- #{idx+1}: {d} {t.entry_price:.2f}→{t.exit_price:.2f}, "
                     f"hold {t.exit_bar-t.entry_bar:,} bars, "
                     f"CR×{t.n_short_diffs}, CR$={t.cumulative_cr_dollars:,.0f}, "
                     f"PnL {t.pnl_pct:+.2f}%")
        L.append("")

    top5 = sorted(trades, key=lambda t: t.pnl_pct, reverse=True)[:5]
    bot5 = sorted(trades, key=lambda t: t.pnl_pct)[:5]
    L.append("**Top 5 盈利**：" + ", ".join(f"{t.pnl_pct:+.2f}%" for t in top5))
    L.append("**Top 5 亏损**：" + ", ".join(f"{t.pnl_pct:+.2f}%" for t in bot5))
    L.append("")

    winners = [t for t in trades if t.pnl_pct > 0]
    losers = [t for t in trades if t.pnl_pct <= 0]
    if winners:
        L.append(f"- 胜者均值：{sum(t.pnl_pct for t in winners)/len(winners):+.2f}%，"
                 f"中位数：{sorted(t.pnl_pct for t in winners)[len(winners)//2]:+.2f}%")
    if losers:
        L.append(f"- 败者均值：{sum(t.pnl_pct for t in losers)/len(losers):+.2f}%，"
                 f"中位数：{sorted(t.pnl_pct for t in losers)[len(losers)//2]:+.2f}%")

    compound_no_outliers = 1.0
    for t in trades:
        if abs(t.pnl_pct) <= threshold:
            compound_no_outliers *= (1 + t.pnl_pct / 100)
    L.append(f"\n- 剔除异常值后复合收益：{(compound_no_outliers-1)*100:+.2f}%")
    L.append(f"- 含异常值复合收益：{compute_metrics(trades)['total_compound']:+.2f}%")
    L.append("")
    return L


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
    L.append("# 全市场端到端回测 v3 — 期货保证金 + BTC杠杆 + 分层降成本\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}")
    L.append(f"总耗时：{total_elapsed:.1f}s\n")

    L.append("## 架构\n")
    L.append("### v2→v3 改进\n")
    L.append("| 项目 | v2 | v3 |")
    L.append("|------|-----|-----|")
    L.append("| 股指 | QQQ 现货 (0.1%费率) | ES E-mini 期货 ($2.25/手, 6手, ~15x杠杆) |")
    L.append("| BTC | 1x 无杠杆 (0.1%费率) | 10x 杠杆 (0.1%费率, 5%强平线) |")
    L.append("| 降成本 | 全标的统一 L0+L1 | 高波动=full(L0+L1), 低波动=conservative(L1 only) |")
    L.append("| HK700 CR | threshold=0.05 | threshold=0.10 (conservative) |")
    L.append("| 选股 | K4 σ(E/$) 门控 | K4 门控 + 比价递归相对强度标注 |\n")

    L.append("### 标的参数\n")
    L.append("| 标的 | 类型 | 手续费 | 杠杆 | 强平线 | CR模式 | K4门控 |")
    L.append("|------|------|--------|------|--------|--------|--------|")
    for c in ASSETS:
        if c.asset_type == "futures":
            fee_desc = f"${c.fixed_comm}/手/边"
            lev_desc = f"~{c.max_contracts}×${c.point_value:.0f}/pt"
        else:
            fee_desc = f"{c.fee_rate*100:.1f}%/边"
            lev_desc = f"{c.leverage:.0f}x"
        liq = f"{c.liq_move*100:.0f}%" if c.liq_move > 0 else "无"
        k4 = "σ(E/$)≥0→多" if c.k4_affected else "独立"
        L.append(f"| {c.symbol} | {c.asset_type} | {fee_desc} | {lev_desc} | {liq} | {c.cr_mode} | {k4} |")
    L.append("")

    # K4 recursive summary
    L.append("## K4 三边递归摘要\n")
    L.append("| 边 | 标的 | Bars | 最高级别 | σ切换 | 最终σ |")
    L.append("|-----|------|------|---------|-------|-------|")
    for edge, cache in [("E/$", es_cache), ("Au/$", gc_cache), ("Oil/$", cl_cache)]:
        L.append(f"| {edge} | {cache.symbol} | {cache.n_bars:,} "
                 f"| L{cache.max_level} | {len(cache.sigma_events)} "
                 f"| {_s(cache.final_sigma)} |")
    L.append("")

    # K4 transitions
    L.append("## K4 配置变化\n")
    L.append(f"总切换次数：{len(k4_transitions)}\n")
    if k4_transitions:
        L.append("| 时间 | 旧配置 | 新配置 | σ(E/$) | σ(Au/$) | σ(Oil/$) | 比价得分 |")
        L.append("|------|--------|--------|--------|---------|----------|----------|")
        shown = k4_transitions[-30:] if len(k4_transitions) > 30 else k4_transitions
        if len(k4_transitions) > 30:
            L.append(f"| ... | ... | ... | ... | ... | ... | (前{len(k4_transitions)-30}条省略) |")
        for t in shown:
            old = f"({_s(t.old_config[0])},{_s(t.old_config[1])},{_s(t.old_config[2])})"
            new = f"({_s(t.new_config[0])},{_s(t.new_config[1])},{_s(t.new_config[2])})"
            rs = ratio_score(t.new_config)
            L.append(f"| {t.date} | {old} | {new} "
                     f"| {_s(t.new_config[0])} | {_s(t.new_config[1])} | {_s(t.new_config[2])} | {rs}/3 |")
    L.append("")

    # Ratio recursive selection context
    L.append("## 比价递归选股\n")
    L.append("K4 σ 推导的比价方向（非独立计算）：\n")
    if k4_transitions:
        last_cfg = k4_transitions[-1].new_config
        se, sau, soil = last_cfg
        L.append(f"- 当前 K4 配置：σ(E/$)={_s(se)}, σ(Au/$)={_s(sau)}, σ(Oil/$)={_s(soil)}")
        e_au = "↑" if (se > sau) else "↓" if (se < sau) else "─"
        e_oil = "↑" if (se > soil) else "↓" if (se < soil) else "─"
        au_oil = "↑" if (sau > soil) else "↓" if (sau < soil) else "─"
        L.append(f"- 推导 E/Au≈{e_au}, E/Oil≈{e_oil}, Au/Oil≈{au_oil}")
        rs = ratio_score(last_cfg)
        L.append(f"- 股票相对强度得分：{rs}/3 ({'强' if rs>=2 else '弱' if rs<=1 else '中'})")
        L.append(f"\n**选股含义**：得分≥2时股票类资产（ES/OKLO/HK700）处于相对强势环境；"
                 "得分≤1时金/油表现优于股票，股票配置应谨慎。")
    L.append("")

    # Results tables
    for mode_label, results in [("WITH K4", results_k4), ("WITHOUT K4", results_base)]:
        L.append(f"## {mode_label}\n")
        L.append("| 标的 | 类型 | Bars | 杠杆 | BH% | FSM% | 胜率 | 交易 | 多/空 | CR | PW | 强平 | MDD | 耗时 |")
        L.append("|------|------|------|------|-----|------|------|------|-------|-----|-----|------|-----|------|")
        for r in results:
            m = r.metrics
            n_t = m["n"] or 1
            eff_lev = f"{_effective_leverage(r.config, r.first_price):.1f}x"
            L.append(
                f"| {r.symbol} | {r.config.asset_type} | {r.n_bars:,} | {eff_lev} | {r.bh_pct:+.1f} "
                f"| {m['total_compound']:+.2f} | {m['win_rate']:.0f}% "
                f"| {m['n']} | {m['long_n']}/{m['short_n']} "
                f"| {m['n_with_cr']}/{n_t} | {m['n_pw']}/{n_t} "
                f"| {m['n_liq']} | {m['max_dd']:.1f}% | {r.elapsed:.0f}s |"
            )
        port = portfolio_compound(results)
        bh_avg = sum(r.bh_pct for r in results) / len(results) if results else 0
        L.append(f"| **组合** | — | — | — | {bh_avg:+.1f} | {port:+.2f} | — | — | — | — | — | — | — | — |")
        L.append("")

        for r in results:
            if r.metrics["n"] == 0:
                continue
            L.append(f"### {r.symbol} ({mode_label})\n")
            L.append(f"- Bars: **{r.n_bars:,}**, Price: {r.first_price:.2f} → {r.last_price:.2f}")
            eff_lev = _effective_leverage(r.config, r.first_price)
            L.append(f"- BH: **{r.bh_pct:+.2f}%**, FSM: **{r.metrics['total_compound']:+.2f}%**, "
                     f"有效杠杆: {eff_lev:.1f}x")
            if r.config.asset_type == "futures":
                L.append(f"- 合约: {r.config.max_contracts}手 × ${r.config.point_value}/pt, "
                         f"保证金: ${r.config.margin_per_contract:,.0f}/手, "
                         f"佣金: ${r.config.fixed_comm}/手/边")
            elif r.config.leverage > 1:
                L.append(f"- 杠杆: {r.config.leverage:.0f}x, "
                         f"手续费: {r.config.fee_rate*100:.1f}%/边, "
                         f"强平线: {r.config.liq_move*100:.0f}%反向")
            L.append("")
            L.append("| # | Dir | Entry | Exit | Hold | PnL% | CR | CR$ | PW | Reason |")
            L.append("|---|-----|-------|------|------|------|-----|------|-----|--------|")
            for idx, t in enumerate(r.trades[:20]):
                d = "L" if t.direction == 1 else "S"
                pw = "Y" if t.principal_withdrawn else ""
                L.append(f"| {idx+1} | {d} | {t.entry_price:.2f}@{t.entry_bar:,} "
                         f"| {t.exit_price:.2f}@{t.exit_bar:,} "
                         f"| {t.exit_bar-t.entry_bar:,} | {t.pnl_pct:+.2f} "
                         f"| {t.n_short_diffs} | {t.cumulative_cr_dollars:,.0f} | {pw} | {t.exit_reason} |")
            if len(r.trades) > 20:
                L.append(f"| ... | ... | ... | ... | ... | ... | ... | ... | ... | (共{len(r.trades)}笔) |")
            L.append("")

    # OKLO anomaly analysis
    oklo_results = [r for r in results_k4 if r.symbol == "OKLO"]
    if oklo_results:
        L.append("## OKLO 异常值分析\n")
        L.extend(analyze_oklo_trades(oklo_results[0].trades))

    # K4 comparison
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

    # Result package
    L.append("## 结果包\n")
    L.append("**结论**：v3 全市场端到端回测——ES期货(替代QQQ) + OKLO + BTC(10x) + HK700。\n")
    L.append("**定义依据**：")
    L.append("- K4 σ：RecursiveOrchestrator 1min 递归最高级别 Move direction")
    L.append("- FSM：267号 D2（无加仓）+ PH settle gate + 分层降成本")
    L.append("- 期货模型：E-mini ES, $50/pt, 6手, $2.25/手/边")
    L.append("- BTC模型：10x杠杆，0.1%/边，5%强平线")
    L.append("- 降成本分层：full(ES/OKLO/BTC)=L0+L1@0.05, conservative(HK700)=L1@0.10\n")
    L.append("**边界条件**：")
    L.append("- ES 既是 K4 σ(E/$) 数据源又是交易标的（自参照但无前视：递归增量计算）")
    L.append("- BTC 强平价基于 bar 内最差价（long=low, short=high），可能略悲观")
    L.append("- 期货允许 fractional contracts 用于 CR（建模近似，实际需整手）")
    L.append("- OKLO 无原始日期，通过 ES 交易日历推断")
    L.append("- 比价递归得分从 K4 三边 σ 推导，非独立 ratio 递归计算\n")
    L.append("**谱系引用**：267号(FSM), config_space(K4), 526号(递归存在论区分)。\n")
    L.append("**影响声明**：新建回测脚本和报告，不修改引擎代码。\n")
    L.append("**认识论等级**：L2（真实期货+真实标的 1min 数据，含杠杆模型）。")

    OUTPUT_MD.write_text("\n".join(L))
    _log(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# Main
# ════════════════════════════════════════════════════════════

def main() -> None:
    t0 = time.time()

    _log("=" * 60)
    _log("  全市场端到端回测 v3：期货保证金 + BTC杠杆 + 分层降成本")
    _log("=" * 60)

    # Phase 1: K4 precompute
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
             f"{len(result.sigma_events)} σ events, final σ={_s(result.final_sigma)}")
        del dates, opens, highs, lows, closes
        garbage_collect.collect()

    es_cache = k4_caches["ES"]
    gc_cache = k4_caches["GC"]
    cl_cache = k4_caches["CL"]

    # Phase 1b: K4 timeline
    _log("\n[Phase 1b] 构建 K4 配置时间线 ...")
    k4_timeline, k4_transitions = build_k4_config_timeline(
        es_cache, gc_cache, cl_cache,
        k4_dates["ES"], k4_dates["GC"], k4_dates["CL"],
    )
    _log(f"  配置切换次数：{len(k4_transitions)}")
    if k4_transitions:
        last = k4_transitions[-1]
        _log(f"  最新配置：({_s(last.new_config[0])},{_s(last.new_config[1])},{_s(last.new_config[2])})")
        _log(f"  比价得分：{ratio_score(last.new_config)}/3")

    es_day_strs = [d[:10] for d in k4_dates["ES"]]
    es_bpd = Counter(es_day_strs)
    cal_trading_dates = sorted(es_bpd.keys())
    cal_vals = sorted(es_bpd.values())
    cal_median_bpd = cal_vals[len(cal_vals) // 2]
    _log(f"  交易日历：{len(cal_trading_dates)} days, 中位 {cal_median_bpd} bars/day")

    # Phase 2: Per-asset backtests
    _log("\n[Phase 2] 逐标的 FSM 回测 ...")

    results_k4: list[AssetResult] = []
    results_base: list[AssetResult] = []

    for cfg in ASSETS:
        _log(f"\n  加载 {cfg.symbol} ...")
        if cfg.fmt == "columnar":
            _, dates, opens, highs, lows, closes = load_columnar(cfg.filename)
        else:
            _, dates, opens, highs, lows, closes = load_row_based(cfg.filename, cfg.symbol)

        if cfg.truncate_from:
            for i, d in enumerate(dates):
                if d[:10] >= cfg.truncate_from:
                    old_n = len(closes)
                    dates, opens, highs, lows, closes = (
                        dates[i:], opens[i:], highs[i:], lows[i:], closes[i:],
                    )
                    _log(f"  {cfg.symbol} truncated: {old_n:,} → {len(closes):,} (from {cfg.truncate_from})")
                    break

        n = len(closes)
        bh = (closes[-1] - closes[0]) / closes[0] * 100
        eff_lev = _effective_leverage(cfg, closes[0])
        _log(f"  {cfg.symbol}: {n:,} bars, {closes[0]:.2f} → {closes[-1]:.2f}, "
             f"有效杠杆 {eff_lev:.1f}x")

        k4_affects = cfg.k4_affected
        modes = ["k4", "baseline"] if k4_affects else ["both"]

        for mode in modes:
            with_k4 = mode == "k4"
            label = {"k4": "WITH K4", "baseline": "WITHOUT K4", "both": "BOTH=BASELINE"}.get(mode, mode)
            _log(f"\n  ── {cfg.symbol} ({label}) ──")

            bar_dirs = build_bar_allowed_dirs(
                n_bars=n, cfg=cfg, k4_timeline=k4_timeline,
                asset_dates=dates if cfg.has_dates else None,
                cal_trading_dates=cal_trading_dates, cal_median_bpd=cal_median_bpd,
                with_k4=with_k4,
            )

            t1 = time.time()
            trades = run_fsm(cfg, opens, highs, lows, closes, bar_dirs)
            elapsed = time.time() - t1

            metrics = compute_metrics(trades)
            result = AssetResult(
                symbol=cfg.symbol, n_bars=n,
                first_price=closes[0], last_price=closes[-1],
                bh_pct=bh, trades=trades, metrics=metrics,
                elapsed=elapsed, mode=mode, config=cfg,
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
            if m["n_liq"] > 0:
                _log(f"    强平：{m['n_liq']}笔")

        del dates, opens, highs, lows, closes
        garbage_collect.collect()

    total_elapsed = time.time() - t0

    write_report(k4_transitions, es_cache, gc_cache, cl_cache,
                 results_k4, results_base, total_elapsed)

    def _dump(results: list[AssetResult]) -> dict:
        return {
            r.symbol: {
                "n_bars": r.n_bars,
                "bh": r.bh_pct,
                "fsm_compound": r.metrics["total_compound"],
                "win_rate": r.metrics["win_rate"],
                "n_trades": r.metrics["n"],
                "long_n": r.metrics["long_n"],
                "short_n": r.metrics["short_n"],
                "max_dd": r.metrics["max_dd"],
                "n_earn": r.metrics.get("n_earn", 0),
                "pnls": [t.pnl_pct for t in r.trades],
            }
            for r in results
        }

    (CACHE_DIR.parent / "full_system_v3_audit_results.json").write_text(
        json.dumps({"with_k4": _dump(results_k4), "without_k4": _dump(results_base)}, indent=2)
    )
    _log("审计 JSON 已写入：full_system_v3_audit_results.json")

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
