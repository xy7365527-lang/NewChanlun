"""版本 I（最终设计）— 全仓 + 中枢门 + 级别归属动态出场 + 多重赋格降成本。

代码基础（用户明确）：`fugue_alpha_diagnosis.py` 的 `run_swing_trading`（E：底背驰
进场 → 持仓穿越 → 顶背驰出场），**不在** G/H 的 fugue_complete_v2 上改。

═══════════════════════════════════════════════════════════════════════
版本 I 最终设计（用户累积修正后）
═══════════════════════════════════════════════════════════════════════

进场（全仓）：
  - **全仓买入**（不做仓位分配；降成本是满仓基础上做短差，不是减仓）。
  - **中枢趋势门**（1买定义的一部分，非过滤器）：走势级 down settle 的 zs_count ≥ 1
    才算趋势背驰（没中枢→没趋势→没背驰→不是1买）。zs_count=0 不进场。
  - **confirmed 底背驰**（走势级 = E 的 down_move_settled ∧ entry_div_ok，MACD 闸）。
  - **区间套**精确定位（走势级 ARM → bar/segment 精确入场价）。
  - **级别归属**：entry_level = ARM 窗口内同时确认买点的最高涌现层（走势级及以上）。

持仓（多重赋格降成本，全仓基础上短差）：
  - 单个 `CostReductionFSM`（满仓 own_capital=本金，sub_ratio=0.3）跟踪 cost_basis 演化。
  - 降成本触发 = 次级别背驰买卖点，**persistence 过滤**（只有高 persistence 的 settle
    才触发 trim；低 persistence=噪音，不做短差）。次级别通道：
      · segment 级（复用 v2 的 sub_sell/sub_buy：l1 persistence ratio + refined MACD 门）；
      · 归属级别以下的 move 级（move persistence ≥ 近期中位数门，entry_level≥走势之上时）。
    bar 级**不作降成本触发**（太细，persistence 门无意义）。
  - 三阶段：COST_REDUCING（股数守恒降 cost_basis）→ cost_basis≤0 → EARNING_SHARES（挣股数）。
  - persistence 过滤**只影响降成本触发频率，不影响进出场**（进出场用 confirmed 买卖点）。

出场：
  - **只在 entry_level 卖点**（归属级别顶背驰）→ **全仓清出**（单 FSM，清仓前回补未闭短差）。
  - entry_level 以下的卖点不触发出场（只触发降成本）。

2 买标记：全仓已在 1 买买入，2 买不触发额外操作；仅标记位置（type2 candidate 计数）用于分析。

═══════════════════════════════════════════════════════════════════════
地基/口径（透明，可质询）
═══════════════════════════════════════════════════════════════════════
- 引擎不产每层 confirmed BSP（递归栈只产中枢/走势 settle，BSP 仅 level=1）。每层"第一类
  买卖点 confirmed" = move settle + 背驰（521号 candidate+MACD闸 ≡ BSP type1 confirmed
  force_c/force_a≤0.9，同构，非降级）。
- 级别阶梯 ladder（低→高）：idx0=bar(PH) / idx1=segment / idx2=走势(orch move1, E 的 l2_flip
  鲁棒级, MACD背驰) / idx≥3=orch 递归 move-of-move(persistence 背驰；高层 first_seg_s0 是
  component 索引非 bar→不能算 MACD)。归属/出场候选 = idx≥2。
- 1min 上高层(orch 递归)极稀疏 → entry_level 大概率=走势级(ladder2)，I 退化为 E + segment
  降成本。归属分布由回测实测输出（揭示有效域）。
- E 经 tape-equality 逐字段复现（compute_signals_i 的 E 字段 ≡ compute_signals）。

认识论等级：L2（真实数据；含高层稀疏 + 降成本拖累的否定性结果）。
谱系：521号 / 267号 / project_divergence_locator_entry_exit /
project_costreduction_moneyprinter_bug / project_complete_fugue_v2。
"""

from __future__ import annotations

import json
import os
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)
from newchan.types import Bar  # noqa: E402

import fugue_alpha_diagnosis as _ef  # noqa: E402
from fugue_alpha_diagnosis import (  # noqa: E402
    INITIAL_CAPITAL,
    MODE_NONE,
    PHLevelState,
    CompletedTrade,
    MoveRecord,
    UpSegRecord,
    _median_alive_persistence,
    _move_force_simple,
    _persistence_ratio,
    _refined_gate,
    check_divergence,
    compute_metrics,
    run_swing_trading,
)
from fugue_complete_v2 import BarSignalV2, _median, _refined_gate_down  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_version_i_results.md"

DATA_FILES = {
    "OKLO": "oklo_1m_databento_full.json",
    "QQQ": "qqq_1m_databento_full.json",
    "ES": "es_1m_databento_10y.json",
    "GC": "gc_1m_databento_10y.json",
    "CL": "cl_1m_databento_10y.json",
    "ZN": "zn_1m_databento_10y.json",
    "6E": "usd6e_1m_databento_10y.json",
    "BRN": "brn_1m_databento_10y.json",
}

MAX_LEVELS = 8
MAX_LADDER = MAX_LEVELS + 2
PERSIST_MED_WINDOW = 50
SUB_RATIO = 0.3  # 全仓基础上单次短差动用比例（单 FSM）


_COST_OPEN_STATES = frozenset({
    CostState.POSITION_OPEN, CostState.COST_REDUCING,
    CostState.PRINCIPAL_WITHDRAWN, CostState.EARNING_SHARES,
})
_COST_ACTIVE_DIFF_STATES = frozenset({
    CostState.COST_REDUCING, CostState.EARNING_SHARES,
})


# ════════════════════════════════════════════════════════════
# 数据加载（两种 schema + 可选 dates）
# ════════════════════════════════════════════════════════════

def load_symbol(symbol: str) -> tuple[list, list, list, list, list | None]:
    path = DATA_DIR / DATA_FILES[symbol.upper()]
    raw = json.loads(path.read_text())
    opens = [float(x) for x in raw["opens"]]
    highs = [float(x) for x in raw["highs"]]
    lows = [float(x) for x in raw["lows"]]
    closes = [float(x) for x in raw["closes"]]
    years: list | None = None
    if "dates" in raw:
        years = [int(str(d)[:4]) for d in raw["dates"]]
    return opens, highs, lows, closes, years


# ════════════════════════════════════════════════════════════
# 每层背驰状态（move settle → buy / sell / sell_persist_high）
# ════════════════════════════════════════════════════════════

class LevelDivState:
    """单个递归 move 级别（ladder≥3）的背驰 + persistence 状态。

    买点 = down settle ∧ 底背驰；卖点 = up settle ∧ 顶背驰（persistence 作 force）。
    sell_persist_high = up settle 的 persistence ≥ 近期中位数（降成本触发门）。
    """

    __slots__ = ("down_hist", "up_hist", "up_persist")

    def __init__(self) -> None:
        self.down_hist: list[MoveRecord] = []
        self.up_hist: list[MoveRecord] = []
        self.up_persist: list[float] = []

    def on_settle(
        self, direction: str, high: float, low: float, persistence: float,
    ) -> tuple[bool, bool, bool]:
        rec = MoveRecord(direction=direction, high=high, low=low, macd_area=persistence)
        if direction == "down":
            self.down_hist.append(rec)
            is_buy = (
                len(self.down_hist) >= 2
                and check_divergence(self.down_hist[-1], self.down_hist[-2], "down"))
            return is_buy, False, False
        self.up_hist.append(rec)
        is_sell = (
            len(self.up_hist) >= 2
            and check_divergence(self.up_hist[-1], self.up_hist[-2], "up"))
        med = _median(self.up_persist)
        ph = persistence >= med if self.up_persist else True
        self.up_persist.append(persistence)
        if len(self.up_persist) > PERSIST_MED_WINDOW:
            self.up_persist.pop(0)
        return False, is_sell, ph


# ════════════════════════════════════════════════════════════
# 分层信号磁带（每 bar）
# ════════════════════════════════════════════════════════════

@dataclass(slots=True)
class BarSignalI:
    close: float
    buy: tuple          # ladder idx → 买点（confirmed）
    sell: tuple         # ladder idx → 卖点（confirmed）
    sell_ph: tuple      # ladder idx → 该层卖点 persistence-high（降成本门，move 级有效）
    max_emerged: int
    entry_zs_count: int  # 走势级本 bar down settle 的中枢数（中枢门；无则 0）
    sub_sell: bool       # segment 级降成本卖点（v2 口径：l1 persistence ratio + refined MACD 门）
    sub_buy: bool        # segment 级降成本买点
    type2_buy: bool      # 2 买标记（candidate，仅计数）


# ════════════════════════════════════════════════════════════
# 信号层（compute-once）：单次 orch pass → E 磁带 + I 磁带
# ════════════════════════════════════════════════════════════

def compute_signals_i(
    opens: list[float], highs: list[float], lows: list[float], closes: list[float],
) -> tuple[list[BarSignalV2], list[BarSignalI]]:
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id="bt_i", max_levels=MAX_LEVELS)
    base_ts = datetime(2020, 1, 1)

    l1_down = PHLevelState.make(); l1_up = PHLevelState.make()
    l2_down = PHLevelState.make(); l2_up = PHLevelState.make()
    l0_down = PHLevelState.make(); l0_up = PHLevelState.make()

    macd = OnlineMacdState()
    pos_cum = 0.0; pos_cum_hist: list[float] = []
    neg_cum = 0.0; neg_cum_hist: list[float] = []

    l1_up_segs: list[tuple[float, float]] = []
    last_l1up_i = 0
    l1_down_segs: list[tuple[float, float]] = []
    last_l1down_i = 0
    down_move_hist: list[MoveRecord] = []
    up_move_hist: list[MoveRecord] = []
    l2_direction = 0
    up_persists_trend: list[float] = []  # 走势级 up move persistence（sell_ph[2] 用）

    level_div: dict[int, LevelDivState] = {}
    max_emerged = 2

    e_signals: list[BarSignalV2] = []
    i_signals: list[BarSignalI] = []
    last_progress = 0

    def _macd_pos_area(a: int, b: int) -> float:
        if b < 0:
            return 0.0
        return pos_cum_hist[b] if a <= 0 else pos_cum_hist[b] - pos_cum_hist[a - 1]

    def _macd_neg_area(a: int, b: int) -> float:
        if b < 0:
            return 0.0
        return neg_cum_hist[b] if a <= 0 else neg_cum_hist[b] - neg_cum_hist[a - 1]

    for i in range(n):
        c = closes[i]; h = highs[i]; lo = lows[i]
        ts = base_ts + timedelta(minutes=i)
        bar = Bar(ts=ts, open=opens[i], high=h, low=lo, close=c, volume=0.0)

        _, _, hist = macd.update(c, ts)
        pos_cum += hist if hist > 0 else 0.0
        pos_cum_hist.append(pos_cum)
        neg_cum += -hist if hist < 0 else 0.0
        neg_cum_hist.append(neg_cum)

        snap = orch.process_bar(bar)
        moves = snap.move_snapshot.moves

        l0_ds = l0_down.tree.update(c); l0_us = l0_up.tree.update(-c)
        l0_r1_down, _ = l0_down.detect_settle(l0_ds)
        l0_r1_up, _ = l0_up.detect_settle(l0_us)

        l1_nr1_up = False; l1_nr1_down = False
        seg_buy = False; seg_sell = False
        for e in snap.seg_snapshot.events:
            if isinstance(e, SegmentSettleV1):
                ep = e.ep1_price
                if ep <= 0:
                    continue
                ds1 = l1_down.tree.update(ep); us1 = l1_up.tree.update(-ep)
                _, nr1d = l1_down.detect_settle(ds1)
                _, nr1u = l1_up.detect_settle(us1)
                if nr1u:
                    l1_nr1_up = True
                if nr1d:
                    l1_nr1_down = True
                if e.ep1_price > e.ep0_price:
                    seg_sell = True
                    seg_lo = last_l1up_i + 1 if last_l1up_i + 1 <= i else i
                    seg_high = max(highs[seg_lo:i + 1]) if seg_lo <= i else h
                    l1_up_segs.append((seg_high, _macd_pos_area(last_l1up_i, i)))
                    last_l1up_i = i
                elif e.ep1_price < e.ep0_price:
                    seg_buy = True
                    seg_lo2 = last_l1down_i + 1 if last_l1down_i + 1 <= i else i
                    seg_low = min(lows[seg_lo2:i + 1]) if seg_lo2 <= i else lo
                    l1_down_segs.append((seg_low, _macd_neg_area(last_l1down_i, i)))
                    last_l1down_i = i

        # 走势级（ladder2 = orch move1）：E 字段 + 中枢门 zs_count + sell_ph
        l2_flip_long = False; l2_flip_short = False
        down_move_settled = False; up_move_settled = False
        entry_zs_count = 0
        sell_ph2 = False
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1):
                mv = None
                for m in moves:
                    if (m.seg_start == e.seg_start and m.direction == e.direction
                            and m.settled):
                        mv = m
                        break
                if mv is None:
                    continue
                if mv.direction == "down":
                    down_move_hist.append(MoveRecord(
                        direction="down", high=mv.high, low=mv.low,
                        macd_area=_macd_neg_area(mv.first_seg_s0, mv.last_seg_s1)))
                    down_move_settled = True
                    entry_zs_count = mv.zs_count  # 中枢门
                else:
                    up_move_hist.append(MoveRecord(
                        direction="up", high=mv.high, low=mv.low,
                        macd_area=_macd_pos_area(mv.first_seg_s0, mv.last_seg_s1)))
                    up_move_settled = True
                    med_u = _median(up_persists_trend)
                    sell_ph2 = mv.persistence >= med_u if up_persists_trend else True
                    up_persists_trend.append(mv.persistence)
                    if len(up_persists_trend) > PERSIST_MED_WINDOW:
                        up_persists_trend.pop(0)
                ep = mv.high if mv.direction == "up" else mv.low
                if ep <= 0:
                    continue
                ds2 = l2_down.tree.update(ep); us2 = l2_up.tree.update(-ep)
                r1d, _ = l2_down.detect_settle(ds2)
                r1u, _ = l2_up.detect_settle(us2)
                if r1d:
                    l2_flip_long = True
                if r1u:
                    l2_flip_short = True
        if l2_flip_long:
            l2_direction = 1
        if l2_flip_short:
            l2_direction = -1

        # E 字段：BSP candidate / type2 / new_up / 标量（逐字同 v2）
        bsp_events = snap.bsp_snapshot.events
        buy_cands: list[int] = []; sell_cands: list[int] = []; buy_invalidates: list[int] = []
        type2_buy = False
        for e in bsp_events:
            nm = type(e).__name__; bid = e.bsp_id; side = e.side
            if "Candidate" in nm:
                (buy_cands if side == "buy" else sell_cands).append(bid)
                if side == "buy" and getattr(e, "kind", "") == "type2":
                    type2_buy = True
            elif "Invalidate" in nm and side == "buy":
                buy_invalidates.append(bid)
        new_up: list[UpSegRecord] = []
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1) and e.direction == "up":
                for m in moves:
                    if (m.seg_start == e.seg_start and m.direction == "up" and m.settled):
                        new_up.append(UpSegRecord(
                            seg_start=m.seg_start, high=m.high, low=m.low,
                            bar_start=m.first_seg_s0, bar_end=m.last_seg_s1,
                            force=_move_force_simple(m),
                            macd_area=_macd_pos_area(m.first_seg_s0, m.last_seg_s1),
                            persistence=m.persistence))
                        break
        med_persistence = _median_alive_persistence(l1_down.alive) if new_up else 0.0
        if l1_nr1_up:
            l1_up_ratio = _persistence_ratio(l1_up.alive, 1)
            refined_gate_ok = _refined_gate(l1_up_segs)
        else:
            l1_up_ratio = 0.0; refined_gate_ok = False
        entry_div_ok = (
            check_divergence(down_move_hist[-1], down_move_hist[-2], "down")
            if len(down_move_hist) >= 2 else False)
        exit_div_ok = (
            check_divergence(up_move_hist[-1], up_move_hist[-2], "up")
            if len(up_move_hist) >= 2 else False)

        # segment 级降成本信号（v2 口径：persistence ratio + refined MACD 门）
        sub_sell = bool(l1_nr1_up and l1_up_ratio > 0.05 and refined_gate_ok)
        if l1_nr1_down:
            l1_down_ratio = _persistence_ratio(l1_down.alive, 1)
            sub_buy_div = l1_down_ratio > 0.05 and _refined_gate_down(l1_down_segs)
        else:
            sub_buy_div = False
        sub_buy = bool(sub_buy_div or bool(buy_cands))

        e_signals.append(BarSignalV2(
            close=c, l0_r1_down=l0_r1_down, l0_r1_up=l0_r1_up,
            l1_nr1_up=l1_nr1_up, l1_nr1_down=l1_nr1_down,
            l2_flip_long=l2_flip_long, l2_flip_short=l2_flip_short,
            l2_direction=l2_direction,
            buy_cands=tuple(buy_cands), sell_cands=tuple(sell_cands),
            buy_invalidates=tuple(buy_invalidates), new_up_moves=tuple(new_up),
            med_persistence=med_persistence, l1_up_ratio=l1_up_ratio,
            refined_gate_ok=refined_gate_ok, entry_div_ok=entry_div_ok,
            exit_div_ok=exit_div_ok, down_move_settled=down_move_settled,
            up_move_settled=up_move_settled,
            entry_zs_count=0, exit_zs_count=0, entry_persistence_high=False,
            exit_persistence_high=False, type2_buy=False,
            sub_sell_signal=False, sub_buy_signal=False))

        # ── I 分层买卖点磁带 ──
        buy = [False] * MAX_LADDER; sell = [False] * MAX_LADDER
        sell_ph = [True] * MAX_LADDER
        buy[0] = l0_r1_down; sell[0] = l0_r1_up           # bar（仅区间套定位，不作降成本）
        buy[1] = seg_buy; sell[1] = seg_sell              # segment（降成本走 sub_sell/sub_buy）
        buy[2] = down_move_settled and entry_div_ok       # 走势级买点（E 进场）
        sell[2] = l2_flip_short and exit_div_ok           # 走势级卖点（E 出场）
        sell_ph[2] = sell_ph2

        for rs in snap.recursive_snapshots:
            ladder = rs.level_id + 1
            if ladder >= MAX_LADDER:
                continue
            if ladder > max_emerged:
                max_emerged = ladder
            st = level_div.get(ladder)
            if st is None:
                st = LevelDivState(); level_div[ladder] = st
            for e in rs.move_events:
                if isinstance(e, MoveSettleV1):
                    m = None
                    for cand in rs.moves:
                        if (cand.seg_start == e.seg_start
                                and cand.direction == e.direction and cand.settled):
                            m = cand
                            break
                    if m is None:
                        continue
                    is_buy, is_sell, ph = st.on_settle(
                        m.direction, m.high, m.low, m.persistence)
                    if is_buy:
                        buy[ladder] = True
                    if is_sell:
                        sell[ladder] = True
                        sell_ph[ladder] = ph

        i_signals.append(BarSignalI(
            close=c, buy=tuple(buy), sell=tuple(sell), sell_ph=tuple(sell_ph),
            max_emerged=max_emerged, entry_zs_count=entry_zs_count,
            sub_sell=sub_sell, sub_buy=sub_buy, type2_buy=type2_buy))

        if i - last_progress >= 500_000:
            print(f"    [{i / n * 100:5.1f}%] signal bar {i:,}/{n:,}")
            last_progress = i

    return e_signals, i_signals


# ════════════════════════════════════════════════════════════
# 版本 I 交易：全仓 + 中枢门 + 级别归属动态出场 + 多重赋格降成本
# ════════════════════════════════════════════════════════════

_FLAT, _ARMED, _LONG = 0, 1, 2


def run_version_i(
    i_signals: list[BarSignalI],
) -> tuple[list[CompletedTrade], dict]:
    n = len(i_signals)
    state = _FLAT
    entry_bar = -1; entry_price = 0.0; entry_ladder = -1; arm_bar = -1
    arm_high = 2
    fsm: CostReductionFSM | None = None
    SUB_EXPIRY = _ef.SUB_EXPIRY
    n_addon = 0  # 2 买标记计数（不操作）

    trades: list[CompletedTrade] = []
    ladder_attribution: dict[int, int] = {}
    ladder_held_bars: dict[int, int] = {}

    def _open(bar_idx: int, price: float, el: int) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, fsm
        entry_bar = bar_idx; entry_price = price; entry_ladder = el
        f0 = CostReductionFSM.create(
            own_capital=INITIAL_CAPITAL, margin_amount=0.0, sub_ratio=SUB_RATIO)
        fsm = transition(
            f0, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, price=price, level=f"L{el}"))
        state = _LONG

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, fsm
        if fsm is None or entry_price <= 0:
            state = _FLAT; fsm = None; return
        f = fsm
        if (f.active_short_diff is not None and f.active_short_diff.is_open
                and f.state in _COST_ACTIVE_DIFF_STATES):
            f = transition(
                f, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, price=price, level="sub"))
        snap = f.snapshot()
        pnl_pct = (
            snap.cumulative_recovered + snap.total_shares * price - INITIAL_CAPITAL
        ) / INITIAL_CAPITAL * 100
        held = bar_idx - entry_bar
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price, exit_bar=bar_idx,
            exit_price=price, pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=len(f.completed_short_diffs), cost_basis_at_exit=snap.cost_basis))
        ladder_attribution[entry_ladder] = ladder_attribution.get(entry_ladder, 0) + 1
        ladder_held_bars[entry_ladder] = ladder_held_bars.get(entry_ladder, 0) + held
        state = _FLAT; entry_price = 0.0; entry_ladder = -1; fsm = None

    for i in range(n):
        sig = i_signals[i]
        c = sig.close

        if state == _FLAT:
            # 走势级 confirmed 底背驰 ∧ 中枢门(zs_count≥1) → ARM
            if sig.buy[2] and sig.entry_zs_count >= 1:
                state = _ARMED; arm_bar = i
                arm_high = 2
                for k in range(3, min(sig.max_emerged + 1, MAX_LADDER)):
                    if sig.buy[k]:
                        arm_high = k

        elif state == _ARMED:
            for k in range(3, min(sig.max_emerged + 1, MAX_LADDER)):
                if sig.buy[k] and k > arm_high:
                    arm_high = k
            do_enter = sig.buy[0] or sig.buy[1]
            if not do_enter and (i - arm_bar) > SUB_EXPIRY:
                do_enter = True
            if do_enter:
                _open(i, c, arm_high)
            elif sig.sell[2]:
                state = _FLAT

        elif state == _LONG:
            assert fsm is not None
            if sig.sell[entry_ladder]:
                _close(i, c, f"exit_L{entry_ladder}_sell")  # 全仓清出
            else:
                # 多重赋格降成本（次级别背驰 + persistence 过滤，单 FSM 满仓短差）
                f = fsm
                has_open = (
                    f.active_short_diff is not None and f.active_short_diff.is_open)
                # 降成本卖点（开 trim）：segment 通道 ∨ 归属级别以下 move 级（persistence 门）
                sell_trig = sig.sub_sell
                buy_trig = sig.sub_buy
                for d in range(2, entry_ladder):
                    if sig.sell[d] and sig.sell_ph[d]:
                        sell_trig = True
                    if sig.buy[d]:
                        buy_trig = True
                if sell_trig and not has_open and f.state in _COST_OPEN_STATES:
                    fsm = transition(
                        f, FsmEvent(FsmEventType.SUB_LEVEL_SELL_POINT, price=c, level="sub"))
                elif buy_trig and has_open and f.state in _COST_ACTIVE_DIFF_STATES:
                    fsm = transition(
                        f, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, price=c, level="sub"))
                # 2 买标记（不操作）
                if sig.type2_buy:
                    n_addon += 1

    if state == _LONG and fsm is not None:
        _close(n - 1, i_signals[-1].close, "eod_close")

    avg_held = {
        k: round(ladder_held_bars[k] / ladder_attribution[k], 1)
        for k in ladder_attribution if ladder_attribution[k]
    }
    return trades, {
        "ladder_attribution": ladder_attribution,
        "ladder_avg_held_bars": avg_held,
        "addon_2buy_marks": n_addon,
    }


# ════════════════════════════════════════════════════════════
# 扩展指标（compute_metrics + 盈亏比 + 按年）
# ════════════════════════════════════════════════════════════

def extended_metrics(trades: list[CompletedTrade], years: list | None = None) -> dict:
    m = dict(compute_metrics(trades))
    pnls = [t.pnl_pct for t in trades]
    gross_win = sum(p for p in pnls if p > 0)
    gross_loss = -sum(p for p in pnls if p < 0)
    m["profit_factor"] = (gross_win / gross_loss) if gross_loss > 0 else float("inf")
    by_year: dict[int, dict] = {}
    if years is not None:
        for t in trades:
            if 0 <= t.entry_bar < len(years):
                y = years[t.entry_bar]
                yb = by_year.setdefault(y, {"n": 0, "compound": 1.0, "wins": 0})
                yb["n"] += 1
                yb["compound"] *= 1 + t.pnl_pct / 100
                if t.pnl_pct > 0:
                    yb["wins"] += 1
        for y in by_year:
            yb = by_year[y]
            yb["return_pct"] = round((yb["compound"] - 1) * 100, 2)
            yb["win_rate"] = round(yb["wins"] / yb["n"] * 100, 1) if yb["n"] else 0.0
            del yb["compound"]
    m["by_year"] = by_year
    return m


# ════════════════════════════════════════════════════════════
# 单标的管线
# ════════════════════════════════════════════════════════════

def process_symbol(symbol: str) -> tuple[str, dict]:
    print(f"\n{'=' * 60}\n  {symbol} — 版本 I 回测（E vs I，compute-once）\n{'=' * 60}")
    opens, highs, lows, closes, years = load_symbol(symbol)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    t0 = time.time()
    e_signals, i_signals = compute_signals_i(opens, highs, lows, closes)
    sig_elapsed = time.time() - t0
    print(f"  [{symbol}] {n:,} bars  BH={bh:+.2f}%  信号层 {sig_elapsed:.1f}s")

    e_trades, _ = run_swing_trading(e_signals, MODE_NONE)
    i_trades, i_extra = run_version_i(i_signals)

    em = extended_metrics(e_trades, years)
    im = extended_metrics(i_trades, years)
    print(f"  [{symbol}/E] 交易={em['n']:3d} 胜率={em['win_rate']:4.0f}% "
          f"复利={em['total_compound']:+10.2f}% 超额={em['total_compound']-bh:+10.2f}% "
          f"MDD={em['max_dd']:+.1f}% 夏普={em['sharpe']:+.3f} 盈亏比={em['profit_factor']:.2f}")
    print(f"  [{symbol}/I] 交易={im['n']:3d} 胜率={im['win_rate']:4.0f}% "
          f"复利={im['total_compound']:+10.2f}% 超额={im['total_compound']-bh:+10.2f}% "
          f"MDD={im['max_dd']:+.1f}% 夏普={im['sharpe']:+.3f} 盈亏比={im['profit_factor']:.2f} "
          f"降成本笔={im['n_with_cr']}")
    print(f"  [{symbol}/I] 归属(ladder→笔): {i_extra['ladder_attribution']}  "
          f"平均持仓bar: {i_extra['ladder_avg_held_bars']}  2买标记: {i_extra['addon_2buy_marks']}")

    return symbol, {
        "n_bars": n, "bh": bh, "first": closes[0], "last": closes[-1],
        "E": em, "I": im,
        "ladder_attribution": i_extra["ladder_attribution"],
        "ladder_avg_held_bars": i_extra["ladder_avg_held_bars"],
        "addon_2buy_marks": i_extra["addon_2buy_marks"],
        "sig_elapsed": round(sig_elapsed, 1),
    }


def main() -> None:
    order = ["OKLO", "QQQ", "BRN", "ZN", "GC", "CL", "ES", "6E"]
    only = os.environ.get("BT_SYMBOLS")
    if only:
        order = [s.strip().upper() for s in only.split(",")]

    results: dict = {}
    t_all = time.time()
    for sym in order:
        _, out = process_symbol(sym)
        results[sym] = out
        _write_report(results)
        (DATA_DIR / "fugue_version_i_results.json").write_text(
            json.dumps(results, indent=2, ensure_ascii=False, default=str))
    print(f"\n总耗时 {time.time() - t_all:.1f}s（{len(order)} 标的）")
    print("报告：fugue_version_i_results.md / .json")


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 版本 I — 全仓 + 中枢门 + 级别归属动态出场 + 多重赋格降成本\n")
    L.append(
        "> 基线 E（`run_swing_trading`）。I 设计：全仓进场 + 中枢门(zs≥1) + confirmed 底背驰 + "
        "区间套 + 级别归属；持仓多重赋格降成本(persistence 过滤次级别背驰短差→挣股数)；"
        "出场=归属级别卖点全仓清出。脚本 `analysis/fugue_version_i.py`。认识论 **L2**。\n")
    L.append(
        "> 521号口径：每层第一类买卖点 = move settle + 背驰（走势级 MACD，高层 persistence）。"
        "高层力度为 PH 纯拓扑代理（521号边界）。降成本在强趋势为负 alpha（拖累 I<E，实测）。\n")
    L.append("## E vs I 对照\n")
    L.append("| 标的 | bars | BH% | E复利% | I复利% | E超额 | I超额 | E夏普 | I夏普 | E MDD | I MDD | E盈亏比 | I盈亏比 | E笔 | I笔 | I降成本笔 |")
    L.append("|------|------|-----|--------|--------|-------|-------|-------|-------|-------|-------|---------|---------|-----|-----|----------|")
    for s in results:
        r = results[s]; e = r["E"]; ii = r["I"]; bh = r["bh"]
        L.append(
            f"| {s} | {r['n_bars']:,} | {bh:+.1f} | {e['total_compound']:+.1f} | "
            f"{ii['total_compound']:+.1f} | {e['total_compound']-bh:+.1f} | "
            f"{ii['total_compound']-bh:+.1f} | {e['sharpe']:+.2f} | {ii['sharpe']:+.2f} | "
            f"{e['max_dd']:+.1f} | {ii['max_dd']:+.1f} | {e['profit_factor']:.2f} | "
            f"{ii['profit_factor']:.2f} | {e['n']} | {ii['n']} | {ii['n_with_cr']} |")
    L.append("")
    L.append("## 级别归属分布（实测，揭示高层稀疏有效域）\n")
    L.append("ladder: idx2=走势(用户L2,MACD) idx3=用户L3 idx4+=更高(persistence,稀疏)\n")
    L.append("| 标的 | 归属(ladder→笔数) | 平均持仓bar | 2买标记 |")
    L.append("|------|------------------|------------|--------|")
    for s in results:
        r = results[s]
        L.append(f"| {s} | {r['ladder_attribution']} | {r['ladder_avg_held_bars']} | "
                 f"{r['addon_2buy_marks']} |")
    L.append("")
    L.append("## 按年收益（_10y 标的，有 dates）\n")
    for s in results:
        ii = results[s]["I"]; e = results[s]["E"]
        if ii.get("by_year"):
            L.append(f"### {s}\n")
            L.append("| 年 | E笔 | E收益% | E胜率 | I笔 | I收益% | I胜率 |")
            L.append("|----|-----|--------|-------|-----|--------|-------|")
            yrs = sorted(set(ii["by_year"]) | set(e.get("by_year", {})))
            for y in yrs:
                ey = e.get("by_year", {}).get(y, {}); iy = ii["by_year"].get(y, {})
                L.append(
                    f"| {y} | {ey.get('n',0)} | {ey.get('return_pct',0):+.1f} | "
                    f"{ey.get('win_rate',0):.0f}% | {iy.get('n',0)} | "
                    f"{iy.get('return_pct',0):+.1f} | {iy.get('win_rate',0):.0f}% |")
            L.append("")
    OUTPUT_MD.write_text("\n".join(L))


if __name__ == "__main__":
    main()
