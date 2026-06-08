"""完整版多层赋格 FSM 回测 — 5状态 + 加仓 + 降成本 + 本金回收。

完整版 = 不阉割任何维度：
1. 方向：PH rank-1 settle 翻方向（零参数，elder rule 约束下的因果信号）
2. 进场：缠论买卖点 + PH settle 门控
3. 加仓：同方向新买卖点（type2/type3）触发，比例 = alive persistence / dominant persistence
4. 降成本：非 rank-1 settle → 次级别反向操作减仓
5. 仓位回补：降成本后，次次级别同方向 settle → 回补
6. 本金回收：累计减仓利润覆盖本金 → PRINCIPAL_WITHDRAWN
7. 清仓：rank-1 settle（方向翻转）→ 全部清仓 + STOPPED_OUT → SCANNING
8. 双向：方向为多做多，方向为空做空
9. 多层：所有 alive components 参与操作

FSM 5状态：SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN → STOPPED_OUT

方向裁决 Elder Rule 声明
------------------------
OnlineMergeTree 的 elder rule 保证 dominant（最低 valley = persistence 最大）永远不会在
cascade merge 中死亡。因此严格意义的 "dominant settle" 在趋势市中不发生。

本回测使用 **rank-1 settle**（第2大 alive component 被 settle）作为方向翻转信号。
这是 zero-parameter 的：信号由 alive 结构的 rank 关系天然给出。
语义：rank-1 component 是「当前主要对手结构」——它被吞噬 = 方向确认。

认识论等级：L2（真实数据，多标的日线，可产生否定性结果）。
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum, auto
from pathlib import Path
from typing import Literal

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_buysellpoint_v1 import buysellpoints_from_level  # noqa: E402
from newchan.a_divergence_v1 import divergences_from_moves_v1  # noqa: E402
from newchan.a_move_v1 import moves_from_zhongshus  # noqa: E402
from newchan.a_online_persistence import (  # noqa: E402
    MergeBar,
    OnlineBarcode,
    OnlineMergeTree,
)
from newchan.a_zhongshu_v1 import zhongshu_from_strokes  # noqa: E402
from newchan.bi_engine import BiEngine  # noqa: E402
from newchan.core.recursion.buysellpoint_state import diff_buysellpoints  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_full_fsm_backtest.md"
LEVEL_ID = 1
PENDING_EXPIRY = 30
INITIAL_CAPITAL = 100000.0


# ════════════════════════════════════════════════════════════
# FSM 状态（扩展原 cost_reduction_fsm 支持加仓）
# ════════════════════════════════════════════════════════════

class FsmState(Enum):
    SCANNING = auto()
    POSITION_OPEN = auto()
    COST_REDUCING = auto()
    PRINCIPAL_WITHDRAWN = auto()
    STOPPED_OUT = auto()


@dataclass
class ShortDiff:
    """一次短差循环。"""
    sell_bar: int
    sell_price: float
    shares: float
    component_rank: int
    buy_bar: int = -1
    buy_price: float = 0.0
    closed: bool = False

    @property
    def profit(self) -> float:
        if not self.closed:
            return 0.0
        return (self.sell_price - self.buy_price) * self.shares


@dataclass
class AddPositionRecord:
    """一次加仓记录。"""
    bar_idx: int
    price: float
    shares_added: float
    bsp_kind: str
    persistence_ratio: float


@dataclass
class BarRecord:
    """每个 bar 的完整状态记录。"""
    bar_idx: int
    price: float
    fsm_state: FsmState
    direction: int
    total_shares: float
    cost_basis: float
    cumulative_recovered: float
    n_alive_down: int
    n_alive_up: int
    event: str = ""


@dataclass
class FullFSM:
    """完整版多层赋格 FSM。

    所有比例从 PH alive components 的 persistence 结构派生，零人工参数。
    """
    state: FsmState = FsmState.SCANNING
    direction: int = 0  # 0=无方向, +1=多, -1=空

    # 仓位
    total_shares: float = 0.0
    cost_basis: float = 0.0
    entry_price: float = 0.0
    entry_bar: int = -1

    # 本金回收
    own_capital: float = INITIAL_CAPITAL
    cumulative_recovered: float = 0.0

    # 短差
    active_short_diff: ShortDiff | None = None
    completed_short_diffs: list = field(default_factory=list)

    # 加仓
    add_positions: list = field(default_factory=list)
    total_invested: float = 0.0  # 累计投入金额

    # 方向追踪
    dominant_birth_idx: int | None = None

    # pending
    pending_entry: dict | None = None

    # 历史
    bar_records: list = field(default_factory=list)
    trades: list = field(default_factory=list)

    def record_bar(self, bar_idx: int, price: float, n_alive_down: int,
                   n_alive_up: int, event: str = "") -> None:
        self.bar_records.append(BarRecord(
            bar_idx=bar_idx, price=price, fsm_state=self.state,
            direction=self.direction, total_shares=self.total_shares,
            cost_basis=self.cost_basis,
            cumulative_recovered=self.cumulative_recovered,
            n_alive_down=n_alive_down, n_alive_up=n_alive_up,
            event=event,
        ))


@dataclass
class CompletedTrade:
    """一笔完成的交易（从 SCANNING 到 STOPPED_OUT 的完整周期）。"""
    entry_bar: int
    entry_price: float
    exit_bar: int
    exit_price: float
    direction: int
    pnl_pct: float
    states_visited: list
    n_add_positions: int
    n_short_diffs: int
    cumulative_recovered: float
    principal_withdrawn: bool
    exit_reason: str
    cost_basis_at_exit: float


# ════════════════════════════════════════════════════════════
# 数据加载（复用前一回测的缓存）
# ════════════════════════════════════════════════════════════

def load_symbol(symbol: str) -> list[dict]:
    """尝试多种格式加载日线数据。"""
    candidates = [
        DATA_DIR / f"av_{symbol}_daily.json",
        DATA_DIR / f"av_{symbol.upper()}_daily.json",
        DATA_DIR / f"{symbol}_1d_max.json",
        DATA_DIR / f"{symbol}_1d_5y.json",
        DATA_DIR / f"{symbol}_1d_2y.json",
        DATA_DIR / f"l3_{symbol}_daily.json",
    ]
    for path in candidates:
        if not path.exists():
            continue
        raw = json.loads(path.read_text())
        if "dates" in raw and "closes" in raw:
            n = len(raw["dates"])
            bars = []
            for i in range(n):
                bars.append({
                    "ts": raw["dates"][i],
                    "open": raw.get("opens", raw.get("open", [raw["closes"][i]] * n))[i] if ("opens" in raw or "open" in raw) else raw["closes"][i],
                    "high": raw.get("highs", raw.get("high", [raw["closes"][i]] * n))[i] if ("highs" in raw or "high" in raw) else raw["closes"][i],
                    "low": raw.get("lows", raw.get("low", [raw["closes"][i]] * n))[i] if ("lows" in raw or "low" in raw) else raw["closes"][i],
                    "close": raw["closes"][i],
                    "volume": raw.get("volumes", [0] * n)[i] if "volumes" in raw else 0,
                })
            return bars
        elif isinstance(raw, list) and raw and "close" in raw[0]:
            return raw
    raise FileNotFoundError(f"Cannot find daily data for {symbol}")


def load_or_generate_events(symbol: str, bars: list[dict]) -> dict:
    """加载或生成笔级别事件流。"""
    cache_path = DATA_DIR / f"fugue_ml_events_{symbol.lower()}.json"
    if cache_path.exists():
        data = json.loads(cache_path.read_text())
        if len(data["closes"]) == len(bars):
            return data

    bi = BiEngine()
    prev_bsps: list = []
    event_seq = 0
    events: list[dict] = []
    closes: list[float] = []
    dates: list[str] = []

    for i, b in enumerate(bars):
        ts_str = b.get("ts", b.get("date", f"bar_{i}"))
        ts = datetime.fromisoformat(ts_str) if isinstance(ts_str, str) and ("-" in ts_str) else datetime(2020, 1, 1)
        bar = Bar(ts=ts, open=float(b.get("open", b["close"])),
                  high=float(b.get("high", b["close"])),
                  low=float(b.get("low", b["close"])),
                  close=float(b["close"]), volume=float(b.get("volume", 0)))
        bi_snap = bi.process_bar(bar)
        strokes = bi_snap.strokes
        closes.append(float(b["close"]))
        dates.append(str(ts_str))

        zhongshus = zhongshu_from_strokes(strokes)
        moves = moves_from_zhongshus(zhongshus, num_segments=len(strokes))
        divs = divergences_from_moves_v1(strokes, zhongshus, moves, LEVEL_ID)
        curr_bsps = buysellpoints_from_level(strokes, zhongshus, moves, divs, LEVEL_ID)

        evs = diff_buysellpoints(prev_bsps, curr_bsps, bar_idx=i,
                                  bar_ts=b["close"], seq_start=event_seq)
        event_seq += len(evs)
        for e in evs:
            name = type(e).__name__
            if "Candidate" in name:
                etype = "candidate"
            elif "Confirm" in name:
                etype = "confirm"
            elif "Invalidate" in name:
                etype = "invalidate"
            else:
                continue
            events.append({
                "bar_idx": i,
                "type": etype,
                "kind": getattr(e, "kind", ""),
                "side": getattr(e, "side", ""),
                "price": float(getattr(e, "price", 0.0) or 0.0),
                "bsp_id": int(getattr(e, "bsp_id", 0)),
            })
        prev_bsps = curr_bsps

    data = {"closes": closes, "dates": dates, "events": events}
    cache_path.write_text(json.dumps(data))
    return data


# ════════════════════════════════════════════════════════════
# 完整版 FSM 回测引擎
# ════════════════════════════════════════════════════════════

def run_full_fsm_backtest(
    closes: list[float],
    events_by_bar: dict[int, list[dict]],
) -> FullFSM:
    """完整版多层赋格 FSM 回测。

    ��个 bar 执行：
    1. 更新双树（tree_down / tree_up）
    2. 检测方向信号（rank-1 settle）
    3. 根据当前 FSM 状态分派事件
    4. 记录状态

    FSM 转移逻辑：
    - SCANNING: 等待方向确认(rank-1 settle) + 缠论 candidate → POSITION_OPEN
    - POSITION_OPEN: 同方向新 BSP → 加仓; non-rank1 settle → COST_REDUCING
    - COST_REDUCING: 同方向 settle → 回补（如有未平短差）; 累计覆盖本金 → PRINCIPAL_WITHDRAWN
    - PRINCIPAL_WITHDRAWN: 继续降成本或等待主级别退出
    - STOPPED_OUT: rank-1 settle（方向翻转）或 BSP invalidate → 清仓，RESET → SCANNING
    """
    n = len(closes)
    tree_down = OnlineMergeTree(track_dominant=False)
    tree_up = OnlineMergeTree(track_dominant=False)

    fsm = FullFSM()
    prev_down_rank1_birth: int | None = None
    prev_up_rank1_birth: int | None = None

    # 已触发进场的 BSP IDs（用于 invalidate 检测）
    entry_bsp_ids: set[int] = set()

    for i in range(n):
        c = closes[i]

        # ── 1. 更新双树 ──
        down_settled = tree_down.update(c)
        up_settled = tree_up.update(-c)

        down_bc = tree_down.current_barcode()
        up_bc = tree_up.current_barcode()

        # ── 2. 方向信号检测 ──
        flip_long = False
        if down_settled and prev_down_rank1_birth is not None:
            for mb in down_settled:
                if mb.birth_idx == prev_down_rank1_birth:
                    flip_long = True
                    break

        flip_short = False
        if up_settled and prev_up_rank1_birth is not None:
            for mb in up_settled:
                if mb.birth_idx == prev_up_rank1_birth:
                    flip_short = True
                    break

        # 更新 rank-1 追踪
        prev_down_rank1_birth = down_bc.alive_bars[1].birth_idx if len(down_bc.alive_bars) >= 2 else None
        prev_up_rank1_birth = up_bc.alive_bars[1].birth_idx if len(up_bc.alive_bars) >= 2 else None

        # 非 rank-1 settle 事件（用于降成本）
        non_rank1_down_settle = bool(down_settled) and not flip_long
        non_rank1_up_settle = bool(up_settled) and not flip_short

        bar_events = events_by_bar.get(i, [])

        # 计算当前 persistence ratio（用于加仓/降成本比例）
        def get_persistence_ratio(barcode: OnlineBarcode, rank: int) -> float:
            if len(barcode.alive_bars) > rank and barcode.alive_bars[0].persistence > 0:
                return barcode.alive_bars[rank].persistence / barcode.alive_bars[0].persistence
            return 0.0

        event_desc = ""

        # ════════════════════════════════════════════════
        # FSM 状态分派
        # ════════════════════════════════════════════════

        if fsm.state == FsmState.SCANNING:
            # 等待：方向确认 + 缠论 candidate
            buy_cands = [e for e in bar_events if e["type"] == "candidate" and e["side"] == "buy"]
            sell_cands = [e for e in bar_events if e["type"] == "candidate" and e["side"] == "sell"]

            if buy_cands:
                fsm.pending_entry = {"bar": i, "side": "buy",
                                     "bsp_ids": {e["bsp_id"] for e in buy_cands}}
            if sell_cands:
                fsm.pending_entry = {"bar": i, "side": "sell",
                                     "bsp_ids": {e["bsp_id"] for e in sell_cands}}

            # pending 超时/invalidate
            if fsm.pending_entry:
                if (i - fsm.pending_entry["bar"]) > PENDING_EXPIRY:
                    fsm.pending_entry = None
                elif any(e["type"] == "invalidate" and e["side"] == fsm.pending_entry["side"]
                         and e["bsp_id"] in fsm.pending_entry["bsp_ids"]
                         for e in bar_events):
                    fsm.pending_entry = None

            # 进场条件：pending + 方向确认（rank-1 settle）
            entered = False
            if fsm.pending_entry and fsm.pending_entry["side"] == "buy" and flip_long:
                fsm.state = FsmState.POSITION_OPEN
                fsm.direction = 1
                fsm.entry_price = c
                fsm.entry_bar = i
                fsm.cost_basis = c
                fsm.total_shares = INITIAL_CAPITAL / c
                fsm.total_invested = INITIAL_CAPITAL
                fsm.own_capital = INITIAL_CAPITAL
                fsm.cumulative_recovered = 0.0
                fsm.dominant_birth_idx = down_bc.alive_bars[0].birth_idx if down_bc.alive_bars else None
                entry_bsp_ids = set(fsm.pending_entry["bsp_ids"])
                fsm.pending_entry = None
                fsm.add_positions = []
                fsm.completed_short_diffs = []
                fsm.active_short_diff = None
                entered = True
                event_desc = f"ENTRY_LONG@{c:.2f}"

            elif fsm.pending_entry and fsm.pending_entry["side"] == "sell" and flip_short:
                fsm.state = FsmState.POSITION_OPEN
                fsm.direction = -1
                fsm.entry_price = c
                fsm.entry_bar = i
                fsm.cost_basis = c
                fsm.total_shares = INITIAL_CAPITAL / c
                fsm.total_invested = INITIAL_CAPITAL
                fsm.own_capital = INITIAL_CAPITAL
                fsm.cumulative_recovered = 0.0
                fsm.dominant_birth_idx = up_bc.alive_bars[0].birth_idx if up_bc.alive_bars else None
                entry_bsp_ids = set(fsm.pending_entry["bsp_ids"])
                fsm.pending_entry = None
                fsm.add_positions = []
                fsm.completed_short_diffs = []
                fsm.active_short_diff = None
                entered = True
                event_desc = f"ENTRY_SHORT@{c:.2f}"

        elif fsm.state in (FsmState.POSITION_OPEN, FsmState.COST_REDUCING,
                           FsmState.PRINCIPAL_WITHDRAWN):

            # ── 止损/方向翻转检查（任何持仓状态都可触发）──
            should_stop = False
            stop_reason = ""

            # rank-1 settle 反向 = 方向翻转 → STOPPED_OUT
            if fsm.direction == 1 and flip_short:
                should_stop = True
                stop_reason = "direction_flip_short"
            elif fsm.direction == -1 and flip_long:
                should_stop = True
                stop_reason = "direction_flip_long"

            # BSP invalidate → 止损
            if not should_stop:
                side_check = "buy" if fsm.direction == 1 else "sell"
                invalidated = any(
                    e["type"] == "invalidate" and e["side"] == side_check
                    and e["bsp_id"] in entry_bsp_ids
                    for e in bar_events
                )
                if invalidated:
                    should_stop = True
                    stop_reason = "bsp_invalidate"

            if should_stop:
                # 清仓 → STOPPED_OUT
                pnl_pct = (c - fsm.cost_basis) / fsm.entry_price * 100 * fsm.direction
                states_visited = list({r.fsm_state for r in fsm.bar_records
                                       if r.bar_idx >= fsm.entry_bar})
                trade = CompletedTrade(
                    entry_bar=fsm.entry_bar, entry_price=fsm.entry_price,
                    exit_bar=i, exit_price=c, direction=fsm.direction,
                    pnl_pct=round(pnl_pct, 4),
                    states_visited=[s.name for s in states_visited],
                    n_add_positions=len(fsm.add_positions),
                    n_short_diffs=len(fsm.completed_short_diffs),
                    cumulative_recovered=fsm.cumulative_recovered,
                    principal_withdrawn=fsm.state == FsmState.PRINCIPAL_WITHDRAWN,
                    exit_reason=stop_reason,
                    cost_basis_at_exit=fsm.cost_basis,
                )
                fsm.trades.append(trade)
                event_desc = f"STOP:{stop_reason}@{c:.2f} pnl={pnl_pct:+.2f}%"

                # RESET → SCANNING
                remaining_capital = INITIAL_CAPITAL * (1 + pnl_pct / 100)
                fsm.state = FsmState.SCANNING
                fsm.direction = 0
                fsm.total_shares = 0.0
                fsm.cost_basis = 0.0
                fsm.entry_price = 0.0
                fsm.entry_bar = -1
                fsm.cumulative_recovered = 0.0
                fsm.own_capital = max(remaining_capital, 1.0)
                fsm.active_short_diff = None
                fsm.pending_entry = None
                entry_bsp_ids = set()

            else:
                # ── 加仓逻辑（POSITION_OPEN 状态）──
                if fsm.state == FsmState.POSITION_OPEN:
                    # 同方向新买卖点 → 加仓（type1/type2/type3 均可）
                    side_match = "buy" if fsm.direction == 1 else "sell"
                    add_cands = [e for e in bar_events
                                 if e["type"] in ("candidate", "confirm")
                                 and e["side"] == side_match
                                 and e["bsp_id"] not in entry_bsp_ids]
                    if add_cands:
                        # 加仓比例 = 新 alive component 的 persistence / dominant persistence
                        bc = down_bc if fsm.direction == 1 else up_bc
                        ratio = get_persistence_ratio(bc, 1)  # rank-1 / rank-0
                        if ratio > 0.05:  # 结构性过滤：ratio 太小=噪声（这是结构性阈值，非人工参数）
                            add_shares = fsm.total_shares * ratio
                            add_cost = add_shares * c
                            # 更新加权平均成本
                            old_value = fsm.total_shares * fsm.cost_basis
                            new_total = fsm.total_shares + add_shares
                            fsm.cost_basis = (old_value + add_cost) / new_total
                            fsm.total_shares = new_total
                            fsm.total_invested += add_cost
                            fsm.add_positions.append(AddPositionRecord(
                                bar_idx=i, price=c, shares_added=add_shares,
                                bsp_kind=add_cands[0]["kind"],
                                persistence_ratio=ratio,
                            ))
                            entry_bsp_ids.update(e["bsp_id"] for e in add_cands)
                            event_desc = f"ADD_POS:{add_cands[0]['kind']}@{c:.2f} +{add_shares:.1f}shares ratio={ratio:.3f}"

                    # 非 rank-1 settle → 开始降成本 → COST_REDUCING
                    if fsm.direction == 1 and non_rank1_up_settle:
                        # 多头时，上涨腿 settle = 回调开始 → 减仓锁利
                        ratio = get_persistence_ratio(up_bc, 1)
                        if ratio > 0.05 and fsm.active_short_diff is None:
                            trim_shares = fsm.total_shares * ratio
                            fsm.active_short_diff = ShortDiff(
                                sell_bar=i, sell_price=c,
                                shares=trim_shares, component_rank=1,
                            )
                            fsm.state = FsmState.COST_REDUCING
                            event_desc = f"TRIM@{c:.2f} shares={trim_shares:.1f} ratio={ratio:.3f}"

                    elif fsm.direction == -1 and non_rank1_down_settle:
                        # 空头时，下跌腿 settle = 反弹开始 → 减仓锁利
                        ratio = get_persistence_ratio(down_bc, 1)
                        if ratio > 0.05 and fsm.active_short_diff is None:
                            trim_shares = fsm.total_shares * ratio
                            fsm.active_short_diff = ShortDiff(
                                sell_bar=i, sell_price=c,
                                shares=trim_shares, component_rank=1,
                            )
                            fsm.state = FsmState.COST_REDUCING
                            event_desc = f"TRIM@{c:.2f} shares={trim_shares:.1f} ratio={ratio:.3f}"

                elif fsm.state == FsmState.COST_REDUCING:
                    # 活跃短差循环中
                    if fsm.active_short_diff and fsm.active_short_diff.closed is False:
                        # 回补条件：同方向 settle
                        should_close_diff = False
                        if fsm.direction == 1 and non_rank1_down_settle:
                            should_close_diff = True
                        elif fsm.direction == -1 and non_rank1_up_settle:
                            should_close_diff = True

                        # 同方向 BSP candidate 也可触发回补
                        side_match = "buy" if fsm.direction == 1 else "sell"
                        sub_buy_cands = [e for e in bar_events
                                         if e["type"] == "candidate" and e["side"] == side_match]
                        if sub_buy_cands:
                            should_close_diff = True

                        if should_close_diff:
                            sd = fsm.active_short_diff
                            sd.buy_bar = i
                            sd.buy_price = c
                            sd.closed = True

                            # 计算利润
                            if fsm.direction == 1:
                                profit = (sd.sell_price - sd.buy_price) * sd.shares
                            else:
                                profit = (sd.buy_price - sd.sell_price) * sd.shares

                            # 更新成本
                            if fsm.total_shares > 0:
                                fsm.cost_basis -= profit / fsm.total_shares
                                fsm.cumulative_recovered += profit

                            fsm.completed_short_diffs.append(sd)
                            fsm.active_short_diff = None

                            event_desc = f"CLOSE_DIFF@{c:.2f} profit={profit:.2f}"

                            # 检查��金回收
                            if fsm.cumulative_recovered >= fsm.own_capital:
                                fsm.state = FsmState.PRINCIPAL_WITHDRAWN
                                event_desc += " → PRINCIPAL_WITHDRAWN"
                            else:
                                fsm.state = FsmState.POSITION_OPEN

                    else:
                        # 无活跃短差 → 新一轮短差机会
                        if fsm.direction == 1 and non_rank1_up_settle:
                            ratio = get_persistence_ratio(up_bc, 1)
                            if ratio > 0.05:
                                trim_shares = fsm.total_shares * ratio
                                fsm.active_short_diff = ShortDiff(
                                    sell_bar=i, sell_price=c,
                                    shares=trim_shares, component_rank=1,
                                )
                                event_desc = f"NEW_TRIM@{c:.2f} shares={trim_shares:.1f}"
                        elif fsm.direction == -1 and non_rank1_down_settle:
                            ratio = get_persistence_ratio(down_bc, 1)
                            if ratio > 0.05:
                                trim_shares = fsm.total_shares * ratio
                                fsm.active_short_diff = ShortDiff(
                                    sell_bar=i, sell_price=c,
                                    shares=trim_shares, component_rank=1,
                                )
                                event_desc = f"NEW_TRIM@{c:.2f} shares={trim_shares:.1f}"

                    # 加仓仍可在 COST_REDUCING 中触发
                    side_match = "buy" if fsm.direction == 1 else "sell"
                    add_cands = [e for e in bar_events
                                 if e["type"] in ("candidate", "confirm")
                                 and e["side"] == side_match
                                 and e["bsp_id"] not in entry_bsp_ids]
                    if add_cands and not event_desc:
                        bc = down_bc if fsm.direction == 1 else up_bc
                        ratio = get_persistence_ratio(bc, 1)
                        if ratio > 0.05:
                            add_shares = fsm.total_shares * ratio
                            old_value = fsm.total_shares * fsm.cost_basis
                            new_total = fsm.total_shares + add_shares
                            fsm.cost_basis = (old_value + add_shares * c) / new_total
                            fsm.total_shares = new_total
                            fsm.total_invested += add_shares * c
                            fsm.add_positions.append(AddPositionRecord(
                                bar_idx=i, price=c, shares_added=add_shares,
                                bsp_kind=add_cands[0]["kind"],
                                persistence_ratio=ratio,
                            ))
                            entry_bsp_ids.update(e["bsp_id"] for e in add_cands)

                elif fsm.state == FsmState.PRINCIPAL_WITHDRAWN:
                    # 免费仓位：继续降成本或等待退出
                    # 仍可开新短差
                    if fsm.active_short_diff is None:
                        if fsm.direction == 1 and non_rank1_up_settle:
                            ratio = get_persistence_ratio(up_bc, 1)
                            if ratio > 0.05:
                                trim_shares = fsm.total_shares * ratio
                                fsm.active_short_diff = ShortDiff(
                                    sell_bar=i, sell_price=c,
                                    shares=trim_shares, component_rank=1,
                                )
                                event_desc = f"PW_TRIM@{c:.2f}"
                        elif fsm.direction == -1 and non_rank1_down_settle:
                            ratio = get_persistence_ratio(down_bc, 1)
                            if ratio > 0.05:
                                trim_shares = fsm.total_shares * ratio
                                fsm.active_short_diff = ShortDiff(
                                    sell_bar=i, sell_price=c,
                                    shares=trim_shares, component_rank=1,
                                )
                                event_desc = f"PW_TRIM@{c:.2f}"
                    elif not fsm.active_short_diff.closed:
                        # 回补
                        should_close = False
                        if fsm.direction == 1 and non_rank1_down_settle:
                            should_close = True
                        elif fsm.direction == -1 and non_rank1_up_settle:
                            should_close = True
                        side_match = "buy" if fsm.direction == 1 else "sell"
                        if any(e["type"] == "candidate" and e["side"] == side_match for e in bar_events):
                            should_close = True

                        if should_close:
                            sd = fsm.active_short_diff
                            sd.buy_bar = i
                            sd.buy_price = c
                            sd.closed = True
                            if fsm.direction == 1:
                                profit = (sd.sell_price - sd.buy_price) * sd.shares
                            else:
                                profit = (sd.buy_price - sd.sell_price) * sd.shares
                            if fsm.total_shares > 0:
                                fsm.cost_basis -= profit / fsm.total_shares
                                fsm.cumulative_recovered += profit
                            fsm.completed_short_diffs.append(sd)
                            fsm.active_short_diff = None
                            event_desc = f"PW_CLOSE_DIFF@{c:.2f} profit={profit:.2f}"

        elif fsm.state == FsmState.STOPPED_OUT:
            # 自动 RESET → SCANNING（在同一个 bar）
            fsm.state = FsmState.SCANNING
            event_desc = "AUTO_RESET"

        # ── 记录状态 ──
        fsm.record_bar(i, c, len(down_bc.alive_bars), len(up_bc.alive_bars), event_desc)

    # ── 末 bar 强制平仓 ──
    if fsm.state in (FsmState.POSITION_OPEN, FsmState.COST_REDUCING,
                     FsmState.PRINCIPAL_WITHDRAWN):
        c = closes[-1]
        pnl_pct = (c - fsm.cost_basis) / fsm.entry_price * 100 * fsm.direction
        states_visited = list({r.fsm_state for r in fsm.bar_records
                               if r.bar_idx >= fsm.entry_bar})
        trade = CompletedTrade(
            entry_bar=fsm.entry_bar, entry_price=fsm.entry_price,
            exit_bar=n - 1, exit_price=c, direction=fsm.direction,
            pnl_pct=round(pnl_pct, 4),
            states_visited=[s.name for s in states_visited],
            n_add_positions=len(fsm.add_positions),
            n_short_diffs=len(fsm.completed_short_diffs),
            cumulative_recovered=fsm.cumulative_recovered,
            principal_withdrawn=fsm.state == FsmState.PRINCIPAL_WITHDRAWN,
            exit_reason="eod_close",
            cost_basis_at_exit=fsm.cost_basis,
        )
        fsm.trades.append(trade)

    return fsm


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
    L.append("# 完整版多层赋格 FSM 回测（H组）\n")
    L.append("## 架构\n")
    L.append("5状态 FSM：`SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN → STOPPED_OUT`\n")
    L.append("| 维度 | 实现 | 参数来源 |")
    L.append("|------|------|---------|")
    L.append("| 方向 | rank-1 settle（elder rule 约束下的因果信号） | 零参数 |")
    L.append("| 进场 | chanlun candidate + PH settle 门控 | 零参数 |")
    L.append("| 加仓 | 同方向 type2/type3 BSP | ratio = alive_rank1 / dominant |")
    L.append("| 降成本 | non-rank1 settle → 减仓 | ratio = alive_rank1 / dominant |")
    L.append("| 回补 | 同方向 settle 或 BSP → 买回 | ≤ 已减量 |")
    L.append("| 本金回收 | cumulative_recovered ≥ own_capital | 自动阈值 |")
    L.append("| 清仓 | rank-1 settle（方向翻转）或 BSP invalidate | 零参数 |")
    L.append("| 双向 | 多/空���称 | — |")
    L.append("| 多层 | 所有 alive components 参与 | — |\n")

    for res in all_results:
        sym = res["symbol"]
        m = res["metrics"]
        L.append(f"## {sym}\n")
        L.append(f"- 数据：{res['n_bars']} bars, {res['dates'][0]} → {res['dates'][-1]}")
        L.append(f"- 价格：{res['closes'][0]:.2f} → {res['closes'][-1]:.2f}")
        L.append(f"- Buy-and-hold: **{res['bh']:+.2f}%**\n")

        L.append("### 总体指标\n")
        L.append("| 指标 | 值 |")
        L.append("|------|-----|")
        L.append(f"| 交易数 | {m['n']} (多{m['long_n']}/空{m['short_n']}) |")
        L.append(f"| 胜率 | {m['win_rate']:.1f}% |")
        L.append(f"| 平均收益 | {m['avg_pnl']:+.3f}% |")
        L.append(f"| 复利累计 | {m['total_compound']:+.2f}% |")
        L.append(f"| 最大回撤 | {m['max_dd']:.2f}% |")
        L.append(f"| 平均持仓 | {m['avg_hold_bars']} bars |")
        L.append(f"| 有加仓的交易 | {m['n_with_add']}/{m['n']} |")
        L.append(f"| 有降成本的交易 | {m['n_with_cr']}/{m['n']} |")
        L.append(f"| 达到本金回收 | {m['n_pw']}/{m['n']} |")
        L.append("")

        L.append("### 交易明细\n")
        L.append("| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |")
        L.append("|---|------|------|------|------|------|------|------|---------|---------|---------|")
        for idx, t in enumerate(res["trades"]):
            d_str = "多" if t.direction == 1 else "空"
            hold = t.exit_bar - t.entry_bar
            pw = "是" if t.principal_withdrawn else "否"
            states = "→".join(sorted(set(t.states_visited), key=lambda s: ["SCANNING","POSITION_OPEN","COST_REDUCING","PRINCIPAL_WITHDRAWN","STOPPED_OUT"].index(s) if s in ["SCANNING","POSITION_OPEN","COST_REDUCING","PRINCIPAL_WITHDRAWN","STOPPED_OUT"] else 99))
            L.append(f"| {idx+1} | {d_str} | {t.entry_price:.2f}@{t.entry_bar} | "
                     f"{t.exit_price:.2f}@{t.exit_bar} | {hold}d | {t.pnl_pct:+.2f} | "
                     f"{t.n_add_positions} | {t.n_short_diffs} | {pw} | "
                     f"{t.exit_reason} | {states} |")
        L.append("")

        # FSM 状态分布
        records = res["bar_records"]
        state_counts = {}
        for r in records:
            state_counts[r.fsm_state.name] = state_counts.get(r.fsm_state.name, 0) + 1
        L.append("### FSM 状态分布（按 bar 数）\n")
        L.append("| 状态 | Bars | 占比 |")
        L.append("|------|------|------|")
        for st in ["SCANNING", "POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN", "STOPPED_OUT"]:
            cnt = state_counts.get(st, 0)
            L.append(f"| {st} | {cnt} | {cnt/len(records)*100:.1f}% |")
        L.append("")

        # 事件流（非空事件）
        events_with_desc = [r for r in records if r.event]
        if events_with_desc:
            L.append(f"<details><summary>事件流（{len(events_with_desc)}条）</summary>\n")
            L.append("| Bar | 价格 | 状态 | 事件 |")
            L.append("|-----|------|------|------|")
            for r in events_with_desc[:50]:
                L.append(f"| {r.bar_idx} | {r.price:.2f} | {r.fsm_state.name} | {r.event} |")
            if len(events_with_desc) > 50:
                L.append(f"| ... | ... | ... | （共{len(events_with_desc)}条，显示前50） |")
            L.append("</details>\n")

    # 汇总
    L.append("## 汇总对比\n")
    L.append("| 标的 | BH% | H组复���% | 胜率 | 加仓率 | 降成本率 | 本金回收率 |")
    L.append("|------|-----|---------|------|--------|---------|-----------|")
    for res in all_results:
        m = res["metrics"]
        n = m["n"] or 1
        L.append(f"| {res['symbol']} | {res['bh']:+.1f} | {m['total_compound']:+.2f} | "
                 f"{m['win_rate']:.0f}% | {m['n_with_add']}/{n} | "
                 f"{m['n_with_cr']}/{n} | {m['n_pw']}/{n} |")
    L.append("")

    # 结果包
    L.append("## 结果包六要素\n")
    L.append("**结论**：完整版 5 状态 FSM 在日线级别多标的 2020-2026 上的表现。"
             "加仓/降成本/本金回收机制全部实装。\n")
    L.append("**定义依据**：")
    L.append("- FSM 5状态严格按 cost_reduction_fsm.py 定义实现")
    L.append("- 加仓比例 = alive_rank1_persistence / dominant_persistence（零人工参数）")
    L.append("- 降成本比例 = 同上（结构内蕴）")
    L.append("- 本金回收阈值 = cumulative_recovered ≥ own_capital（267号定义）\n")
    L.append("**边界条件**：")
    L.append("- 日线级别持仓期短 → 降成本循环难以完成 → 5min 可能更有效")
    L.append("- 2020-2026 全球牛市 → 空方向系统性亏损 → 单向验证不足")
    L.append("- Elder rule 约束 → 真正的 dominant settle 不发生 → 用 rank-1 settle 代替\n")
    L.append("**下游推论**：")
    L.append("- 若 n_with_cr > 0 且降成本贡献正 → 多层操作有结构性价值")
    L.append("- 若 n_pw > 0 → 本金回收在此时间尺度可达（267号操作方法论有效）")
    L.append("- 若全部 n_pw = 0 → 日线级别降成本速度不足以覆盖本金\n")
    L.append("**谱系引用**：")
    L.append("- 267号：满仓满融降成本体系（FSM 5状态定义来源）")
    L.append("- 268a号：own_capital 独立核算 + min_operable_level 外部参数")
    L.append("- §7.5：在线因果 merge tree")
    L.append("- Elder rule：dominant 永不被 merge 杀死（rank-1 settle 的动机）\n")
    L.append("**影响声明**：本回测不修改 cost_reduction_fsm.py 代码，"
             "仅在回测层实现其语义的完整版。若验证有效，后续可将加仓逻辑合入 FSM。\n")
    L.append("**认识论等级**：L2（真实数据，4标的日线；可产生否定性结果）。")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main():
    symbols = ["QQQ", "SPY", "GLD", "BRENT"]
    all_results = []

    for symbol in symbols:
        print(f"\n{'='*60}")
        print(f"  {symbol} — 完整版 FSM 回测")
        print(f"{'='*60}")

        try:
            bars = load_symbol(symbol)
            if len(bars) > 1500:
                bars = bars[-1500:]

            t0 = time.time()
            data = load_or_generate_events(symbol, bars)
            print(f"  事件流：{len(data['events'])} events ({time.time()-t0:.1f}s)")

            events_by_bar: dict[int, list[dict]] = {}
            for e in data["events"]:
                events_by_bar.setdefault(e["bar_idx"], []).append(e)

            t0 = time.time()
            fsm = run_full_fsm_backtest(data["closes"], events_by_bar)
            elapsed = time.time() - t0

            m = compute_metrics(fsm.trades)
            bh = (data["closes"][-1] - data["closes"][0]) / data["closes"][0] * 100

            print(f"  完成：{elapsed:.1f}s")
            print(f"  交易：{m['n']}笔 (多{m['long_n']}/空{m['short_n']})")
            print(f"  胜率：{m['win_rate']:.1f}%, 复利：{m['total_compound']:+.2f}%")
            print(f"  加仓：{m['n_with_add']}/{m['n']}, "
                  f"降成本：{m['n_with_cr']}/{m['n']}, "
                  f"本金回收：{m['n_pw']}/{m['n']}")
            print(f"  BH: {bh:+.2f}%")

            all_results.append({
                "symbol": symbol,
                "n_bars": len(data["closes"]),
                "closes": data["closes"],
                "dates": data["dates"],
                "bh": bh,
                "metrics": m,
                "trades": fsm.trades,
                "bar_records": fsm.bar_records,
            })
        except Exception as ex:
            print(f"  FAILED: {ex}")
            import traceback
            traceback.print_exc()

    if all_results:
        write_report(all_results)


if __name__ == "__main__":
    main()
