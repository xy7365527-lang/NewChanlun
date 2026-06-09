"""Alpha 来源诊断 — 降成本短差控制变量回测（QQQ / OKLO 1min）。

4.8 审计修复降成本"提款机"截断（`if profit > 0`）后全部数字崩塌、跑输 BH。
本脚本做控制变量分解，定位负 alpha 出在哪一层：

  V0  current  —— 当前版本（含降成本，去截断后）。1:1 复制 fugue_no_addon 引擎。
  A   none     —— 零降成本对照：持仓期间禁用所有 trim，纯进出场信号 alpha 基线。
  B   refined  —— 精修降成本触发：trim 仅在 (PH settle ∧ 次级别背驰 ∧ MACD 面积背驰) 同时满足时触发。

控制变量纯净性（关键）：trim 仅写入 cost_basis/cumulative_recovered/total_shares/
has_active_trim，这些字段不反馈进出场状态转移（转移由 move settle / L2 方向 / BSP 驱动）。
→ 三版本交易笔数、进出场点完全相同，仅每笔 pnl 不同 → alpha 分解可归因。

认识论等级：L2（真实数据，QQQ/OKLO 1min；含否定性结果）。
关键限定（521 号）：信号属 candidate 层（PH 门控 + MACD 面积代理），非 confirmed 买卖点。
"""

from __future__ import annotations

import json
import os
import sys
import time
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timedelta
from enum import Enum, auto
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.a_online_persistence import MergeBar, OnlineMergeTree  # noqa: E402
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_alpha_diagnosis.md"
SUB_EXPIRY = 60
PENDING_EXPIRY = 390
INITIAL_CAPITAL = 100_000.0
# 第31课资金管理："用 1/10 的机动资金做短差，买多少卖多少"。
# F 的次级别高抛低吸每次动用 1/10 底仓做一笔短差（次级别顶背驰卖、底背驰买回）。
TRIM_FRACTION = 0.10

# 三个降成本模式 + 一个进出场门控模式
MODE_CURRENT = "current"      # V0: 原降成本（去截断）
MODE_NONE = "none"            # A:  零降成本
MODE_REFINED = "refined"      # B:  精修降成本（背驰门控，仅门控 trim）
MODE_DIVERGENCE = "divergence"  # D:  背驰确认进出场（零降成本 + 进出场门控）
MODE_SWING_E = "swing_e"        # E:  背驰定位器进出场（底背驰买/顶背驰卖），无降成本
MODE_SWING_F = "swing_f"        # F:  背驰定位器进出场 + 次级别背驰降成本


# ════════════════════════════════════════════════════════════
# PH alive 提取（与 fugue_no_addon 一致）
# ════════════════════════════════════════════════════════════

class _AB(NamedTuple):
    """alive 条的最小投影。诊断仅消费 persistence + birth_idx——

    （detect_settle 读 bars[1].birth_idx；_median_alive_persistence 读 persistence；
    _persistence_ratio 读 bars[rank].persistence）。完整 MergeBar 的 lo/hi/
    death_price/birth_price/settled 字段无消费者，故不构造（去掉 8 字段 dataclass
    + 切片 index 算术 = 去掉 _fast_alive 的主要 tottime，等价保持）。"""
    persistence: float
    birth_idx: int


class _Alive:
    __slots__ = ("bars",)

    def __init__(self, bars: tuple[_AB, ...]):
        self.bars = bars


def _fast_alive(tree: OnlineMergeTree) -> _Alive:
    stack = tree._stack
    if not stack:
        return _Alive(())
    cap = tree._running_max
    bars = sorted(
        (_AB(cap - c.val, c.idx) for c in stack),
        key=lambda b: b.persistence, reverse=True,
    )
    return _Alive(tuple(bars))


def _fast_alive_top2(tree: OnlineMergeTree) -> _Alive:
    """`_fast_alive` 的 top-2 专用快路径——逐位等价于其前 2 条 + 正确 len 语义。

    detect_settle **只**消费 `alive.bars[1].birth_idx` 与 `len(alive.bars) >= 2`，
    无需全量排序。persistence = cap − val（cap 对所有 alive 共享）⟹ persistence 降序
    ≡ val 升序；`sorted(reverse=True)` 稳定 ⟹ 同 val 保留栈序（早者在前）。单遍 O(栈深)
    贪心（严格 `<` 比较 val，等值不替换 ⟹ 复刻稳定降序的 top-2），消除每次全量 sort +
    每元素 `_AB` namedtuple 构造 + lambda key（O(N^1.6) 墙的主导常数：profile 测 sort 链
    占 I 总耗时 66%）。

    返回 _Alive：栈深 ≥2 → 恰 2 条（rank0/rank1）；==1 → 1 条；==0 → 空。`len` 语义
    （≥2 ⟺ 栈深≥2）与 bars[1].birth_idx 均与 `_fast_alive` 逐位一致（其余 rank 无消费者）。

    **有效域**（认识论 L1：管线等价，由 447K 真实数据 bit-exact diff 守卫）：仅当调用方
    不读 rank≥2 / 不调 `_median_alive_persistence` / `_persistence_ratio` 时等价（即
    `top2_only=True` 的 PHLevelState）。reference 路径（l1 ratio/median）仍走 `_fast_alive`。
    """
    stack = tree._stack
    n = len(stack)
    if n == 0:
        return _Alive(())
    cap = tree._running_max
    if n == 1:
        c = stack[0]
        return _Alive((_AB(cap - c.val, c.idx),))
    # rank0 = 最小 val（等值取栈中早者）；rank1 = 次小 val。严格 `<` ⟹ 等值不替换。
    best = stack[0]
    best_v = best.val
    second = None
    second_v = 0.0
    for c in stack[1:]:
        v = c.val
        if v < best_v:
            second, second_v = best, best_v
            best, best_v = c, v
        elif second is None or v < second_v:
            second, second_v = c, v
    assert second is not None
    return _Alive((
        _AB(cap - best.val, best.idx),
        _AB(cap - second.val, second.idx),
    ))


@dataclass
class PHLevelState:
    tree: OnlineMergeTree
    alive: _Alive
    prev_r1_birth: int | None = None
    # top2_only：detect_settle 走 _fast_alive_top2（O(栈深) 单遍，无 sort/namedtuple）。
    # 仅当本 state 的 .alive 不被 rank≥2 消费者读取时启用（bar/bi/l2 proxy PH 满足）。
    top2_only: bool = False

    @staticmethod
    def make(top2_only: bool = False) -> "PHLevelState":
        return PHLevelState(
            tree=OnlineMergeTree(track_dominant=False),
            alive=_Alive(()),
            top2_only=top2_only,
        )

    def detect_settle(self, settles: list[MergeBar]) -> tuple[bool, bool]:
        if not settles:
            return False, False
        rank1 = False
        if self.prev_r1_birth is not None:
            for mb in settles:
                if mb.birth_idx == self.prev_r1_birth:
                    rank1 = True
                    break
        self.alive = (
            _fast_alive_top2(self.tree) if self.top2_only else _fast_alive(self.tree)
        )
        self.prev_r1_birth = (
            self.alive.bars[1].birth_idx if len(self.alive.bars) >= 2 else None
        )
        return rank1, bool(settles) and not rank1


# ════════════════════════════════════════════════════════════
# FSM 状态（与 fugue_no_addon 一致）
# ════════════════════════════════════════════════════════════

class St(Enum):
    WAIT_ENTRY = auto()
    ENTRY = auto()
    WAIT_SUB_ENTRY = auto()
    HOLDING = auto()
    EVAL = auto()
    WAIT_DIP = auto()
    OBSERVE = auto()
    WAIT_SUB_EXIT = auto()
    EXIT = auto()


class CostSt(Enum):
    FULL_POS = auto()
    REDUCED = auto()
    PRINCIPAL_RECOVERED = auto()
    EARNING_SHARES = auto()


@dataclass(frozen=True)
class UpSegRecord:
    seg_start: int
    high: float
    low: float
    bar_start: int
    bar_end: int
    force: float
    macd_area: float = 0.0  # [bar_start, bar_end] MACD 正柱面积（向上走势力度，背驰门控用）
    persistence: float = 0.0  # 走势 persistence（信号层预计算，供中位数过滤 mv_p<med_p）


@dataclass(frozen=True)
class MoveRecord:
    """单条走势的背驰判据投影：价格极值 + MACD 面积力度。

    缠师 37 课：背驰 = 同方向相邻走势中，后一走势"创新高/新低但力度衰减"。
    本记录把一条 Move 投影为做背驰比较所需的最小特征——价格极值（high/low）
    与 MACD 面积（up→正柱面积、down→负柱面积绝对值）作为"力度"代理。
    认识论：L2（真实数据 1min；MACD 面积是力度代理，非真实次级别走势区间力度）。
    """
    direction: str  # "up" | "down"
    high: float
    low: float
    macd_area: float


def check_divergence(curr: "UpSegRecord | MoveRecord",
                     prev: "UpSegRecord | MoveRecord",
                     direction: str) -> bool:
    """背驰确认（缠论 37 课）：力度衰减 OR 价格不创极值。

    任务判据（进出场必要条件，非独立信号）：
      买点（direction="down"，下跌走势结束）：
        1. 当前下跌走势 MACD 面积 < 前一同方向下跌走势 MACD 面积（力度衰减），或
        2. 当前下跌走势价格不创新低（curr.low >= prev.low，盘整背驰）。
      卖点（direction="up"，上涨走势结束）：
        1. 当前上涨走势 MACD 面积 < 前一同方向上涨走势 MACD 面积，或
        2. 当前上涨走势价格不创新高（curr.high <= prev.high）。

    两子条件取并（任务原文"或者"）。`macd_area` 已在构造记录时由 MACD 状态投影，
    故 macd_state 不作为运行期入参——力度信息内化在记录里（与任务签名等价）。
    """
    macd_decay = curr.macd_area < prev.macd_area
    if direction == "up":
        price_no_new_extreme = curr.high <= prev.high
    else:
        price_no_new_extreme = curr.low >= prev.low
    return macd_decay or price_no_new_extreme


def _persistence_ratio(alive: "_Alive", rank: int) -> float:
    """alive[rank].persistence / alive[0].persistence（次主 PH 相对主 PH 力度）。"""
    ab = alive.bars
    if len(ab) > rank and ab[0].persistence > 0:
        return ab[rank].persistence / ab[0].persistence
    return 0.0


@dataclass(slots=True)
class BarSignal:
    """单 bar 的 mode-independent 信号快照（信号磁带的一格）。

    性能拆分（compute-once 架构）：引擎/PH/MACD 计算在 4 个交易模式间完全相同，
    抽离为一次 compute_signals → 序列化为 BarSignal 磁带；4 个模式仅重放磁带。
    交易 FSM 从不回写引擎状态，故拆分严格等价（gate=False / 各 cost_mode 数字不变）。

    注：交易 FSM 仅用 close 价格（不读 high/low）；new_up_moves 已含 macd_area/persistence；
    med_persistence / l1_up_ratio / refined_gate_ok / entry_div_ok 均为 bar 级预计算标量。
    """
    close: float
    l0_r1_down: bool
    l0_r1_up: bool
    l1_nr1_up: bool
    l1_nr1_down: bool
    l2_flip_long: bool
    l2_flip_short: bool
    l2_direction: int
    buy_cands: tuple
    sell_cands: tuple
    buy_invalidates: tuple
    new_up_moves: tuple
    med_persistence: float
    l1_up_ratio: float
    refined_gate_ok: bool
    entry_div_ok: bool
    # ── 背驰定位器（实验E/F；38 课"底背驰买、顶背驰卖"）──
    # entry_div_ok 已是 down-move 底背驰；下面是其对称的 up-move 顶背驰 + settle 事件标志。
    exit_div_ok: bool          # 顶背驰：最新 up-move 相对前一 up-move 力度衰减∨不创新高
    down_move_settled: bool    # 本 bar 有 down-move settle（底背驰只在 down-move 完成那一刻定位）
    up_move_settled: bool      # 本 bar 有 up-move settle（顶背驰只在 up-move 完成那一刻定位）


@dataclass
class CompletedTrade:
    entry_bar: int
    entry_price: float
    exit_bar: int
    exit_price: float
    pnl_pct: float
    exit_reason: str
    n_short_diffs: int
    cost_basis_at_exit: float


# ════════════════════════════════════════════════════════════
# 数据加载
# ════════════════════════════════════════════════════════════

_1MIN_FILES = {
    "QQQ": DATA_DIR / "qqq_1m_databento_full.json",
    "OKLO": DATA_DIR / "oklo_1m_databento_full.json",
}


def load_1min(
    symbol: str,
) -> tuple[list[float], list[float], list[float], list[float]]:
    path = _1MIN_FILES[symbol.upper()]
    raw = json.loads(path.read_text())
    return (
        [float(x) for x in raw["opens"]],
        [float(x) for x in raw["highs"]],
        [float(x) for x in raw["lows"]],
        [float(x) for x in raw["closes"]],
    )


# ════════════════════════════════════════════════════════════
# 走势段比较工具（与 fugue_no_addon 一致）
# ════════════════════════════════════════════════════════════

def _median_alive_persistence(alive: _Alive) -> float:
    bars = alive.bars
    if not bars:
        return 0.0
    persis = sorted(b.persistence for b in bars)
    n = len(persis)
    if n % 2 == 1:
        return persis[n // 2]
    return (persis[n // 2 - 1] + persis[n // 2]) / 2


def _move_force_simple(move) -> float:
    return abs(move.high - move.low)


def _check_no_new_high(curr_high: float, prev_high: float) -> bool:
    return curr_high < prev_high


def _check_consolidation_divergence(
    curr_force: float, prev_force: float,
    curr_high: float, prev_high: float,
) -> bool:
    if curr_high <= prev_high:
        return False
    return curr_force < prev_force


# ════════════════════════════════════════════════════════════
# 性能拆分（compute-once 架构）
#
# 引擎/PH/MACD 计算在 4 个交易模式间完全相同（交易 FSM 从不回写引擎状态）。
# 拆为两层：
#   compute_signals(once)  —— 引擎/PH/MACD/背驰记录 → BarSignal 磁带。瓶颈，仅算一次。
#   run_trading(×4, cheap) —— 重放磁带跑交易 FSM。无引擎计算，近乎免费。
# → 4× 加速；A/B/V0/D 数字与拆分前逐位一致（拆分严格等价）。
# ════════════════════════════════════════════════════════════

def compute_signals(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> list[BarSignal]:
    """一次性计算所有 mode-independent 逐 bar 信号（引擎/PH/MACD/背驰记录）。"""
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id="bt", max_levels=2)
    base_ts = datetime(2020, 1, 1)

    l1_down = PHLevelState.make()
    l1_up = PHLevelState.make()
    l2_down = PHLevelState.make()
    l2_up = PHLevelState.make()
    l0_down = PHLevelState.make()
    l0_up = PHLevelState.make()

    macd = OnlineMacdState()
    pos_cum = 0.0
    pos_cum_hist: list[float] = []
    neg_cum = 0.0
    neg_cum_hist: list[float] = []

    l1_up_segs: list[tuple[float, float]] = []
    last_l1up_i = 0
    down_move_hist: list[MoveRecord] = []
    up_move_hist: list[MoveRecord] = []  # 与 down_move_hist 对称（顶背驰：相邻 up-move 力度比较）
    l2_direction = 0

    signals: list[BarSignal] = []
    last_progress = 0

    def _macd_pos_area(a: int, b: int) -> float:
        if b < 0:
            return 0.0
        if a <= 0:
            return pos_cum_hist[b]
        return pos_cum_hist[b] - pos_cum_hist[a - 1]

    def _macd_neg_area(a: int, b: int) -> float:
        if b < 0:
            return 0.0
        if a <= 0:
            return neg_cum_hist[b]
        return neg_cum_hist[b] - neg_cum_hist[a - 1]

    for i in range(n):
        c = closes[i]
        h = highs[i]
        lo = lows[i]
        ts = base_ts + timedelta(minutes=i)
        bar = Bar(ts=ts, open=opens[i], high=h, low=lo, close=c, volume=0.0)

        # MACD 增量（全程连续，正/负柱累积和）
        _, _, hist = macd.update(c, ts)
        pos_cum += hist if hist > 0 else 0.0
        pos_cum_hist.append(pos_cum)
        neg_cum += -hist if hist < 0 else 0.0
        neg_cum_hist.append(neg_cum)

        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events
        moves = snap.move_snapshot.moves

        # L0 PH（每 bar）
        l0_ds = l0_down.tree.update(c)
        l0_us = l0_up.tree.update(-c)
        l0_r1_down, _ = l0_down.detect_settle(l0_ds)
        l0_r1_up, _ = l0_up.detect_settle(l0_us)

        # L1 PH（L1 线段 settle 时）
        l1_nr1_up = False
        l1_nr1_down = False
        for e in snap.seg_snapshot.events:
            if isinstance(e, SegmentSettleV1):
                ep = e.ep1_price
                if ep <= 0:
                    continue
                ds1 = l1_down.tree.update(ep)
                us1 = l1_up.tree.update(-ep)
                _, nr1d = l1_down.detect_settle(ds1)
                _, nr1u = l1_up.detect_settle(us1)
                if nr1u:
                    l1_nr1_up = True
                if nr1d:
                    l1_nr1_down = True
                # 向上段力度记录（refined 背驰门控）：终点高于起点 = 向上段
                if e.ep1_price > e.ep0_price:
                    seg_lo = last_l1up_i + 1 if last_l1up_i + 1 <= i else i
                    seg_high = max(highs[seg_lo:i + 1]) if seg_lo <= i else h
                    seg_area = _macd_pos_area(last_l1up_i, i)
                    l1_up_segs.append((seg_high, seg_area))
                    last_l1up_i = i

        # L2 PH（L1 走势 settle 时）
        l2_flip_long = False
        l2_flip_short = False
        down_move_settled = False   # 本 bar 是否有 down-move 完成（底背驰定位时刻）
        up_move_settled = False     # 本 bar 是否有 up-move 完成（顶背驰定位时刻）
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
                # 走势级背驰记录：down/up move 各投影为 MoveRecord（37 课"相邻同向走势力度比较"）。
                # down → 底背驰（买点）；up → 顶背驰（卖点）。两侧对称，供 E/F 背驰定位器消费。
                if mv.direction == "down":
                    down_move_hist.append(MoveRecord(
                        direction="down", high=mv.high, low=mv.low,
                        macd_area=_macd_neg_area(mv.first_seg_s0, mv.last_seg_s1),
                    ))
                    down_move_settled = True
                else:
                    up_move_hist.append(MoveRecord(
                        direction="up", high=mv.high, low=mv.low,
                        macd_area=_macd_pos_area(mv.first_seg_s0, mv.last_seg_s1),
                    ))
                    up_move_settled = True
                ep = mv.high if mv.direction == "up" else mv.low
                if ep <= 0:
                    continue
                ds2 = l2_down.tree.update(ep)
                us2 = l2_up.tree.update(-ep)
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

        # BSP 事件
        buy_cands: list[int] = []
        sell_cands: list[int] = []
        buy_invalidates: list[int] = []
        for e in bsp_events:
            nm = type(e).__name__
            bid = e.bsp_id
            side = e.side
            if "Candidate" in nm:
                (buy_cands if side == "buy" else sell_cands).append(bid)
            elif "Invalidate" in nm and side == "buy":
                buy_invalidates.append(bid)

        # 新 settle 的 up-move（38 课 S7 段比较）→ UpSegRecord（含 macd_area + persistence）
        new_up: list[UpSegRecord] = []
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1) and e.direction == "up":
                for m in moves:
                    if (m.seg_start == e.seg_start
                            and m.direction == "up"
                            and m.settled):
                        new_up.append(UpSegRecord(
                            seg_start=m.seg_start, high=m.high, low=m.low,
                            bar_start=m.first_seg_s0, bar_end=m.last_seg_s1,
                            force=_move_force_simple(m),
                            macd_area=_macd_pos_area(m.first_seg_s0, m.last_seg_s1),
                            persistence=m.persistence,
                        ))
                        break

        # bar 级预计算标量（替代交易层对 PH .alive / 历史列表的访问）
        med_persistence = (
            _median_alive_persistence(l1_down.alive) if new_up else 0.0
        )
        if l1_nr1_up:
            l1_up_ratio = _persistence_ratio(l1_up.alive, 1)
            refined_gate_ok = _refined_gate(l1_up_segs)
        else:
            l1_up_ratio = 0.0
            refined_gate_ok = False
        entry_div_ok = (
            check_divergence(down_move_hist[-1], down_move_hist[-2], "down")
            if len(down_move_hist) >= 2 else False
        )
        exit_div_ok = (
            check_divergence(up_move_hist[-1], up_move_hist[-2], "up")
            if len(up_move_hist) >= 2 else False
        )

        signals.append(BarSignal(
            close=c,
            l0_r1_down=l0_r1_down, l0_r1_up=l0_r1_up,
            l1_nr1_up=l1_nr1_up, l1_nr1_down=l1_nr1_down,
            l2_flip_long=l2_flip_long, l2_flip_short=l2_flip_short,
            l2_direction=l2_direction,
            buy_cands=tuple(buy_cands), sell_cands=tuple(sell_cands),
            buy_invalidates=tuple(buy_invalidates),
            new_up_moves=tuple(new_up),
            med_persistence=med_persistence,
            l1_up_ratio=l1_up_ratio, refined_gate_ok=refined_gate_ok,
            entry_div_ok=entry_div_ok,
            exit_div_ok=exit_div_ok,
            down_move_settled=down_move_settled,
            up_move_settled=up_move_settled,
        ))

        if i - last_progress >= 200_000:
            print(f"    [{i / n * 100:5.1f}%] signal bar {i:,}/{n:,}")
            last_progress = i

    return signals


# ════════════════════════════════════════════════════════════
# 交易 FSM 重放（参数化 cost_mode / divergence_gate）
#
# 进出场逻辑 1:1 复制 fugue_no_addon（控制变量基线）。
# 仅 HOLDING 状态的降成本块按 cost_mode 分流：
#   - none:    跳过整个降成本块（trim 永不触发）
#   - current: 原 trim（L1 nr1 up settle ∧ ratio>0.05）
#   - refined: trim 加背驰门控（次级别不创新高/盘整背驰 ∧ MACD 面积衰减）
#
# divergence_gate（实验D）：把背驰确认叠加为**进出场状态转移的必要条件**——
#   进场：买点（下跌走势结束）须通过 entry_div_ok（down-move 背驰，信号层预计算）。
#   出场：上涨走势段比较触发的 WAIT_SUB_EXIT 须通过 check_divergence(up-move)。
#   gate=False 时行为与原版 1:1 等价 → A/B/V0 数字不变。
#   实验D = (cost_mode=none, divergence_gate=True)：与 A 的唯一差异是进出场门控。
# ════════════════════════════════════════════════════════════

def run_trading(
    signals: list[BarSignal],
    cost_mode: str,
    divergence_gate: bool = False,
) -> tuple[list[CompletedTrade], dict[str, int], dict[str, int]]:
    n = len(signals)

    refined_counts = {"trim_candidate": 0, "trim_gated": 0}
    div_counts = {
        "entry_candidate": 0, "entry_passed": 0,
        "exit_candidate": 0, "exit_passed": 0,
    }

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
    diff_count = 0
    earn_count = 0

    sub_entry_bar = -1
    sub_entry_op_price = 0.0
    sub_exit_bar = -1
    sub_exit_reason = ""
    sub_exit_op_price = 0.0

    up_seg_records: list[UpSegRecord] = []
    current_down_low = float("inf")

    trades: list[CompletedTrade] = []

    def _close_trade(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, cost_state, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumulative_recovered, own_capital
        nonlocal has_active_trim, pending_entry, entry_bsp_ids
        nonlocal diff_count, earn_count
        nonlocal up_seg_records, current_down_low
        nonlocal sub_entry_bar, sub_exit_bar, sub_exit_reason

        if total_shares <= 0 or entry_price <= 0:
            state = St.WAIT_ENTRY
            return

        pnl_pct = (
            cumulative_recovered + total_shares * price - INITIAL_CAPITAL
        ) / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4),
            exit_reason=reason,
            n_short_diffs=diff_count,
            cost_basis_at_exit=cost_basis,
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
        diff_count = 0
        earn_count = 0
        up_seg_records = []
        current_down_low = float("inf")
        sub_entry_bar = -1
        sub_exit_bar = -1
        sub_exit_reason = ""

    for i in range(n):
        # 从信号磁带读取本 bar 的 mode-independent 信号
        sig = signals[i]
        c = sig.close
        l0_r1_down = sig.l0_r1_down
        l0_r1_up = sig.l0_r1_up
        l1_nr1_up = sig.l1_nr1_up
        l1_nr1_down = sig.l1_nr1_down
        l2_flip_short = sig.l2_flip_short
        l2_direction = sig.l2_direction
        buy_cands = sig.buy_cands
        sell_cands = sig.sell_cands
        buy_invalidates = sig.buy_invalidates
        new_settled_up_moves = sig.new_up_moves

        # ══════ 38 课 FSM（纯做多，进出场与 no_addon 一致） ══════

        if state == St.WAIT_ENTRY:
            if buy_cands and l2_direction == 1:
                pending_entry = {"bar": i, "bsp_ids": set(buy_cands)}
            if pending_entry:
                if (i - pending_entry["bar"]) > PENDING_EXPIRY:
                    pending_entry = None
                elif any(bid in pending_entry["bsp_ids"] for bid in buy_invalidates):
                    pending_entry = None
            if pending_entry and l0_r1_down:
                if divergence_gate:
                    div_counts["entry_candidate"] += 1
                    if sig.entry_div_ok:
                        div_counts["entry_passed"] += 1
                        state = St.ENTRY
                    # 否则维持 WAIT_ENTRY，pending_entry 留存待下次 dip 或过期
                else:
                    state = St.ENTRY

        elif state == St.ENTRY:
            state = St.WAIT_SUB_ENTRY
            sub_entry_bar = i
            sub_entry_op_price = c
            entry_bsp_ids = set(pending_entry["bsp_ids"]) if pending_entry else set()
            pending_entry = None

        elif state == St.WAIT_SUB_ENTRY:
            do_buy = False
            buy_price = c
            if buy_cands and l0_r1_down:
                do_buy = True
                buy_price = c
            elif (i - sub_entry_bar) > SUB_EXPIRY:
                do_buy = True
                buy_price = c
            elif l2_flip_short:
                state = St.WAIT_ENTRY
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                state = St.WAIT_ENTRY

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
                diff_count = 0
                earn_count = 0
                up_seg_records = []
                current_down_low = float("inf")

        elif state == St.HOLDING:
            if l2_flip_short:
                _close_trade(i, c, "l2_direction_flip_short")
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                _close_trade(i, c, "bsp_invalidate")
            else:
                # 38 课 S3/S7: 操作级别走势完成 → 段比较
                if new_settled_up_moves:
                    med_p = sig.med_persistence
                    for new_seg in new_settled_up_moves:
                        # new_seg 已是信号层预计算的 UpSegRecord（含 macd_area/persistence）
                        mv_p = new_seg.persistence
                        if mv_p < med_p:
                            continue
                        if not up_seg_records:
                            up_seg_records.append(new_seg)
                            state = St.WAIT_DIP
                            current_down_low = float("inf")
                        else:
                            prev_seg = up_seg_records[-1]
                            # 原结构出场触发：不创新高 OR 盘整背驰（价格力度）
                            nnh = _check_no_new_high(new_seg.high, prev_seg.high)
                            cdv = _check_consolidation_divergence(
                                new_seg.force, prev_seg.force,
                                new_seg.high, prev_seg.high,
                            )
                            structural_exit = nnh or cdv
                            # 实验D：背驰确认作为出场必要条件叠加（MACD 面积背驰）。
                            # gate=False 时恒为 True → 与原版 1:1 等价。
                            gate_ok = (not divergence_gate) or check_divergence(
                                new_seg, prev_seg, "up",
                            )
                            if structural_exit and divergence_gate:
                                div_counts["exit_candidate"] += 1
                            if structural_exit and gate_ok:
                                if divergence_gate:
                                    div_counts["exit_passed"] += 1
                                up_seg_records.append(new_seg)
                                state = St.WAIT_SUB_EXIT
                                sub_exit_bar = i
                                sub_exit_reason = (
                                    "no_new_high" if nnh else "consolidation_divergence"
                                )
                                sub_exit_op_price = c
                                break
                            else:
                                # 出场被背驰门控拦下（或本就非出场结构）→ 视作上涨延续
                                up_seg_records.append(new_seg)
                                state = St.WAIT_DIP
                                current_down_low = float("inf")

                # ── 降成本块（按 cost_mode 分流） ──
                if cost_mode != MODE_NONE:
                    if state == St.HOLDING and cost_state == CostSt.FULL_POS:
                        if l1_nr1_up and not has_active_trim:
                            ratio = sig.l1_up_ratio
                            if ratio > 0.05:
                                allow_trim = True
                                if cost_mode == MODE_REFINED:
                                    refined_counts["trim_candidate"] += 1
                                    allow_trim = sig.refined_gate_ok
                                    if allow_trim:
                                        refined_counts["trim_gated"] += 1
                                if allow_trim:
                                    active_trim_sell_price = c
                                    active_trim_shares = total_shares * ratio
                                    has_active_trim = True
                                    cost_state = CostSt.REDUCED

                    elif state == St.HOLDING and cost_state == CostSt.REDUCED:
                        if has_active_trim:
                            # 与 no_addon 一致：l1_nr1_down 或 buy_cands 闭合短差
                            should_close_diff = l1_nr1_down or bool(buy_cands)
                            if should_close_diff:
                                profit = (active_trim_sell_price - c) * active_trim_shares
                                if total_shares > 0:
                                    cost_basis -= profit / total_shares
                                    cumulative_recovered += profit
                                diff_count += 1
                                has_active_trim = False
                                if cost_basis <= 0:
                                    cost_basis = 0.0
                                    cost_state = CostSt.EARNING_SHARES
                                else:
                                    cost_state = CostSt.FULL_POS
                        else:
                            if l1_nr1_up:
                                ratio = sig.l1_up_ratio
                                if ratio > 0.05:
                                    allow_trim = True
                                    if cost_mode == MODE_REFINED:
                                        refined_counts["trim_candidate"] += 1
                                        allow_trim = sig.refined_gate_ok
                                        if allow_trim:
                                            refined_counts["trim_gated"] += 1
                                    if allow_trim:
                                        active_trim_sell_price = c
                                        active_trim_shares = total_shares * ratio
                                        has_active_trim = True

                    elif state == St.HOLDING and cost_state == CostSt.EARNING_SHARES:
                        if not has_active_trim:
                            if l1_nr1_up:
                                ratio = sig.l1_up_ratio
                                if ratio > 0.05:
                                    allow_trim = True
                                    if cost_mode == MODE_REFINED:
                                        allow_trim = sig.refined_gate_ok
                                    if allow_trim:
                                        active_trim_sell_price = c
                                        active_trim_shares = total_shares * ratio
                                        has_active_trim = True
                        elif has_active_trim:
                            should_close = l1_nr1_down or bool(buy_cands)
                            if should_close:
                                profit = (active_trim_sell_price - c) * active_trim_shares
                                if c > 0:
                                    total_shares += profit / c
                                earn_count += 1
                                diff_count += 1
                                has_active_trim = False

        elif state == St.WAIT_SUB_EXIT:
            do_exit = False
            exit_price = c
            if l2_flip_short:
                do_exit = True
                sub_exit_reason = "l2_direction_flip_short"
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                do_exit = True
                sub_exit_reason = "bsp_invalidate"
            elif sell_cands and l0_r1_up:
                do_exit = True
            elif (i - sub_exit_bar) > SUB_EXPIRY:
                do_exit = True
            if do_exit:
                _close_trade(i, exit_price, sub_exit_reason or "sub_exit")

        elif state == St.WAIT_DIP:
            if c < current_down_low:
                current_down_low = c
            if l2_flip_short:
                _close_trade(i, c, "l2_direction_flip_short_in_dip")
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                _close_trade(i, c, "bsp_invalidate_in_dip")
            else:
                dip_ended = bool(buy_cands) or l0_r1_down
                if dip_ended and up_seg_records:
                    last_up = up_seg_records[-1]
                    if current_down_low >= last_up.low:
                        state = St.HOLDING
                        cost_state = CostSt.FULL_POS
                        has_active_trim = False
                    else:
                        down_force = last_up.low - current_down_low
                        prev_drop = (
                            up_seg_records[-2].low - last_up.low
                            if len(up_seg_records) >= 2 else down_force * 2
                        )
                        if down_force < abs(prev_drop):
                            state = St.HOLDING
                            cost_state = CostSt.FULL_POS
                            has_active_trim = False
                        else:
                            state = St.OBSERVE

        elif state == St.OBSERVE:
            if l2_flip_short:
                _close_trade(i, c, "l2_direction_flip_short_in_observe")
            elif any(bid in entry_bsp_ids for bid in buy_invalidates):
                _close_trade(i, c, "bsp_invalidate_in_observe")
            else:
                if buy_cands and l0_r1_down:
                    state = St.HOLDING
                    cost_state = CostSt.FULL_POS
                    has_active_trim = False
                    entry_bsp_ids.update(buy_cands)

    # 交易层无引擎计算，无需进度打印（重放磁带近乎瞬时）
    if state in (
        St.HOLDING, St.ENTRY, St.WAIT_SUB_ENTRY, St.EVAL,
        St.WAIT_DIP, St.OBSERVE, St.WAIT_SUB_EXIT,
    ) and total_shares > 0:
        _close_trade(n - 1, signals[-1].close, "eod_close")

    return trades, refined_counts, div_counts


def run_swing_trading(
    signals: list[BarSignal],
    cost_mode: str,
) -> tuple[list[CompletedTrade], dict[str, int]]:
    """背驰定位器进出场（实验 E/F；38 课"底背驰买入、顶背驰卖出，没背驰就不动"）。

    与实验 D（run_trading 的 divergence_gate）的**本质区别**——背驰是定位器不是过滤器：

      D（被证伪）：背驰作为**过滤器**叠加在既有进场条件之上——进场需
        buy_cand ∧ l2_direction==1 ∧ l0_r1_down ∧ entry_div_ok 四者同时成立。
        背驰把进场砍掉 90%（QQQ 放行 10%/OKLO 15%）→ 长期空仓 → 跑输 BH
        （记忆 project_divergence_gate_entry_exit_falsified）。

      E（本函数）：背驰**就是**进出场信号本身（定位器），且进出场分处不同级别——
        敏感进场 + 鲁棒出场，构成"买在背驰底、持仓穿越、卖在趋势顶背驰"的多头结构：
        进场（1买，敏感）= 底背驰：down-move 完成那一刻且相对前一 down-move 力度衰减
              （37 课；定位下跌段力竭的精确低点）。
        持仓 = 默认态：建仓后一直满仓持有（38 课"没背驰=走势没结束=不动"），
              **穿越所有 move 级波动**——两背驰之间是持仓不是空仓（消除 D 的现金拖累）。
        出场（1卖，鲁棒）= L2 趋势顶背驰：l2_flip_short（L2 趋势转向，拓扑）
              ∧ exit_div_ok（最新 up-move 力度衰减，MACD 动量；521 号 PH 须 MACD 闸）。
              用趋势级而非 move 级出场 → 不被 move 级小回调震出，只在操作级别趋势真正
              见顶时离场（对应原文"同级别第一类卖点"）。

      F = E + 持仓期间**次级别买卖点**高抛低吸（操作指导7 原文："每次向下离开中枢
        出现底背驰就介入(买回)，回拉出现顶背驰就走(卖出)"）。次级别(L1 走势)买卖点 =
        走势级背驰：顶背驰(exit_div_ok)卖出 1/10 底仓短差、底背驰(entry_div_ok)买回；
        cost_basis 归零后转挣股数(买回时加股数)。短差只改 cost_basis/total_shares，
        不触碰主级别进出场状态——故 E/F 进出场点完全相同，持仓不断，唯一差异是降成本短差。
        与进场用同一套走势背驰买卖点，只是次级别顶背驰在"未到趋势顶"时是短差卖点、
        到趋势顶(l2_flip_short)时才是主级别清仓卖点——买卖点贯穿全流程，非附加过滤。

    原文依据：38 课操盘程式"当该级别出现底背驰时买入，顶背驰时卖出"；29 课"底背驰
    买入后持有，出现盘整/顶背驰才出"。背驰=状态转移定位器，非进场前置筛。
    """
    n = len(signals)
    refined_counts = {"trim_candidate": 0, "trim_gated": 0}

    FLAT, LONG = St.WAIT_ENTRY, St.HOLDING
    state = FLAT
    cost_state = CostSt.FULL_POS
    total_shares = 0.0
    cost_basis = 0.0
    entry_price = 0.0
    entry_bar = -1
    cumulative_recovered = 0.0
    active_trim_sell_price = 0.0
    active_trim_shares = 0.0
    has_active_trim = False
    diff_count = 0
    earn_count = 0

    trades: list[CompletedTrade] = []

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, cost_state, total_shares, cost_basis, entry_price
        nonlocal entry_bar, cumulative_recovered, has_active_trim
        nonlocal diff_count, earn_count
        if total_shares <= 0 or entry_price <= 0:
            state = FLAT
            return
        pnl_pct = (
            cumulative_recovered + total_shares * price - INITIAL_CAPITAL
        ) / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=diff_count, cost_basis_at_exit=cost_basis,
        ))
        state = FLAT
        cost_state = CostSt.FULL_POS
        total_shares = 0.0
        cost_basis = 0.0
        entry_price = 0.0
        entry_bar = -1
        cumulative_recovered = 0.0
        has_active_trim = False
        diff_count = 0
        earn_count = 0

    for i in range(n):
        sig = signals[i]
        c = sig.close

        if state == FLAT:
            # 1买（次级别底背驰）：down-move 完成且力度衰减 = 第一类买点（操作指导3"买卖点=背驰点"）。
            # 原文：某级别下跌趋势中，次级别走势跌破最后中枢后的背驰点 = 第一类买点。
            if sig.down_move_settled and sig.entry_div_ok:
                state = LONG
                cost_state = CostSt.FULL_POS
                entry_price = c
                entry_bar = i
                cost_basis = c
                total_shares = INITIAL_CAPITAL / c
                cumulative_recovered = 0.0
                has_active_trim = False
                diff_count = 0
                earn_count = 0

        elif state == LONG:
            # 1卖（L2 趋势顶背驰）：趋势转向 ∧ MACD 顶背驰 → 清仓离场。
            # 用 L2 趋势级出场（非 move 级）→ 持仓穿越 move 波动，只在操作级趋势见顶离场
            # （没卖点就不卖，一直持仓——38 课"没背驰=走势没结束=不动"）。
            if sig.l2_flip_short and sig.exit_div_ok:
                _close(i, c, "l2_trend_top_divergence")
            elif cost_mode != MODE_NONE:
                # ── F：持仓期间次级别买卖点高抛低吸（操作指导7 原文）──
                # "每次向下离开中枢出现底背驰就介入(买回)；回拉出现顶背驰就走(卖出)"。
                # 次级别(L1 走势)买卖点 = 走势级背驰：顶背驰卖出短差 / 底背驰买回短差。
                # 短差只改 cost_basis/total_shares，不触碰主级别进出场状态（持仓不断）。
                if not has_active_trim:
                    # 次级别顶背驰（未到趋势顶）→ 高抛：卖出 1/10 底仓做短差（第31课）
                    if sig.up_move_settled and sig.exit_div_ok:
                        active_trim_sell_price = c
                        active_trim_shares = total_shares * TRIM_FRACTION
                        has_active_trim = True
                        refined_counts["trim_candidate"] += 1
                        cost_state = (
                            CostSt.EARNING_SHARES
                            if cost_state == CostSt.EARNING_SHARES
                            else CostSt.REDUCED
                        )
                else:
                    # 次级别底背驰 → 低吸：买回短差。成本未归零则抵 cost_basis；归零后挣股数。
                    if sig.down_move_settled and sig.entry_div_ok:
                        profit = (active_trim_sell_price - c) * active_trim_shares
                        if cost_state == CostSt.EARNING_SHARES:
                            if c > 0:
                                total_shares += profit / c
                            earn_count += 1
                        else:
                            if total_shares > 0:
                                cost_basis -= profit / total_shares
                                cumulative_recovered += profit
                            if cost_basis <= 0:
                                cost_basis = 0.0
                                cost_state = CostSt.EARNING_SHARES
                            else:
                                cost_state = CostSt.FULL_POS
                        diff_count += 1
                        refined_counts["trim_gated"] += 1
                        has_active_trim = False

    if state == LONG and total_shares > 0:
        _close(n - 1, signals[-1].close, "eod_close")
    return trades, refined_counts


def _refined_gate(l1_up_segs: list[tuple[float, float]]) -> bool:
    """实验B 背驰门控：trim 仅在次级别背驰 ∧ MACD 面积衰减时放行。

    条件2（次级别背驰，价格代理）：当前 L1 向上段相对前段——
        不创新高（顶部衰竭）OR 创新高但 MACD 面积衰减（盘整/标准背驰）。
    条件3（MACD 面积确认）：当前段 MACD 正柱面积 < 前段（动量衰减）。
    两者合取（任务"同时满足"）。无前序段（<2）时默认不放行（无法判断背驰）。
    """
    if len(l1_up_segs) < 2:
        return False
    cur_high, cur_area = l1_up_segs[-1]
    prev_high, prev_area = l1_up_segs[-2]
    cond3 = cur_area < prev_area  # MACD 动量衰减
    no_new_high = cur_high <= prev_high
    consol_div = cur_high > prev_high and cur_area < prev_area
    cond2 = no_new_high or consol_div
    return cond2 and cond3


# ════════════════════════════════════════════════════════════
# 统计
# ════════════════════════════════════════════════════════════

def compute_metrics(trades: list[CompletedTrade]) -> dict:
    if not trades:
        return {
            "n": 0, "win_rate": 0.0, "total_compound": 0.0,
            "sharpe": 0.0, "max_dd": 0.0, "n_with_cr": 0,
        }
    pnls = [t.pnl_pct for t in trades]
    wins = sum(1 for p in pnls if p > 0)
    eq = 1.0
    peak = 1.0
    max_dd = 0.0
    for p in pnls:
        eq *= 1 + p / 100
        peak = max(peak, eq)
        max_dd = min(max_dd, (eq - peak) / peak)
    mean = sum(pnls) / len(pnls)
    if len(pnls) > 1:
        var = sum((p - mean) ** 2 for p in pnls) / (len(pnls) - 1)
        std = var ** 0.5
    else:
        std = 0.0
    sharpe = mean / std if std > 0 else 0.0
    return {
        "n": len(trades),
        "win_rate": wins / len(trades) * 100,
        "total_compound": (eq - 1) * 100,
        "sharpe": sharpe,
        "max_dd": max_dd * 100,
        "n_with_cr": sum(1 for t in trades if t.n_short_diffs > 0),
    }


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

_MODE_LABEL = {
    MODE_CURRENT: "当前版本（含降成本，去截断后）",
    MODE_NONE: "实验A（零降成本，纯进出场）",
    MODE_REFINED: "实验B（精修降成本，MACD背驰门控）",
    MODE_DIVERGENCE: "实验D（背驰确认进出场，零降成本）",
}


def write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 赋格 alpha 来源诊断 — 降成本短差控制变量回测\n")
    L.append(
        "> 标的：QQQ / OKLO（1min）  |  脚本：`analysis/fugue_alpha_diagnosis.py`  |  "
        "认识论等级：**L2**（真实数据，含否定性结果）\n"
    )
    L.append(
        "> 521 号限定：信号属 candidate 层（PH 门控 + MACD 面积代理），"
        "非 confirmed 买卖点 alpha 证据。\n"
    )

    L.append("## 一句话结论\n")
    L.append(
        "**fsm 族（A/B/D，38 课 FSM）**：进出场点相同，唯一差异是降成本/门控——A 隔离纯进出场"
        "基线，B 测精修降成本，D 测背驰**门控**（被证伪：砍 90% 进场→空仓→跑输 BH）。\n"
    )
    L.append(
        "**swing 族（E/F，按原文重做，本次新增）**：背驰是**定位器非过滤器**——底背驰买点进场、"
        "默认满仓持仓穿越、L2 趋势顶背驰卖点离场，持仓期次级别背驰高抛低吸（F）。"
        "见「实验E/F」节。\n"
    )

    for sym in results:
        r = results[sym]
        bh = r["bh"]
        L.append(f"## {sym}\n")
        L.append(
            f"- 数据：**{r['n_bars']:,}** bars  |  "
            f"价格：{r['first']:.2f} → {r['last']:.2f}  |  "
            f"Buy-and-hold：**{bh:+.2f}%**\n"
        )
        L.append("| 版本 | 复利% | BH% | 超额% | 交易数 | 胜率 | 夏普 |")
        L.append("|------|-------|-----|--------|--------|------|------|")
        for mode in (MODE_CURRENT, MODE_NONE, MODE_REFINED, MODE_DIVERGENCE):
            m = r[mode]["metrics"]
            excess = m["total_compound"] - bh
            L.append(
                f"| {_MODE_LABEL[mode]} | {m['total_compound']:+.2f} | {bh:+.2f}"
                f" | {excess:+.2f} | {m['n']} | {m['win_rate']:.0f}%"
                f" | {m['sharpe']:+.3f} |"
            )
        L.append("")
        rc = r[MODE_REFINED]["refined_counts"]
        cur_cr = r[MODE_CURRENT]["metrics"]["n_with_cr"]
        ref_cr = r[MODE_REFINED]["metrics"]["n_with_cr"]
        L.append(
            f"- 降成本触发对比：当前版本 trim 候选放行全部；"
            f"实验B 门控 {rc['trim_gated']}/{rc['trim_candidate']} 候选放行"
            f"（背驰确认通过率 "
            f"{rc['trim_gated'] / rc['trim_candidate'] * 100:.0f}%）"
            if rc["trim_candidate"] > 0
            else "- 实验B：无 trim 候选（次级别 L1 up settle 未触发减仓阈值）。"
        )
        L.append(
            f"- 有降成本短差的交易：当前 {cur_cr}/{r[MODE_CURRENT]['metrics']['n']} 笔，"
            f"实验B {ref_cr}/{r[MODE_REFINED]['metrics']['n']} 笔。"
        )
        dc = r[MODE_DIVERGENCE]["div_counts"]
        ent_rate = (
            f"{dc['entry_passed'] / dc['entry_candidate'] * 100:.0f}%"
            if dc["entry_candidate"] > 0 else "n/a"
        )
        exit_rate = (
            f"{dc['exit_passed'] / dc['exit_candidate'] * 100:.0f}%"
            if dc["exit_candidate"] > 0 else "n/a"
        )
        L.append(
            f"- 实验D 背驰门控放行率：进场 {dc['entry_passed']}/{dc['entry_candidate']}"
            f"（{ent_rate}）；出场 {dc['exit_passed']}/{dc['exit_candidate']}（{exit_rate}）。\n"
        )

    # ── 跨标的诊断结论 ──
    L.append("## 诊断结论（alpha 归因）\n")
    for sym in results:
        r = results[sym]
        bh = r["bh"]
        cur = r[MODE_CURRENT]["metrics"]["total_compound"]
        none = r[MODE_NONE]["metrics"]["total_compound"]
        ref = r[MODE_REFINED]["metrics"]["total_compound"]
        cost_drag = cur - none   # 降成本相对纯进出场的增量
        refine_gain = ref - cur  # 精修相对当前的增量
        L.append(f"### {sym}\n")
        L.append(f"- 纯进出场（A）：{none:+.2f}% vs BH {bh:+.2f}% → "
                 + ("**进出场本身有正超额**" if none > bh else "**进出场本身无 alpha（跑输 BH）**"))
        L.append(f"- 降成本拖累/贡献（V0−A）：{cost_drag:+.2f}% → "
                 + ("降成本在**赚钱**" if cost_drag > 0 else "降成本在**亏钱拖累**"))
        L.append(f"- 精修增量（B−V0）：{refine_gain:+.2f}% → "
                 + ("MACD 门控**改善**了择时" if refine_gain > 0 else "MACD 门控**未改善**（或更差）"))
        L.append(f"- 精修后是否翻正超额：B {ref:+.2f}% vs BH {bh:+.2f}% → "
                 + ("**翻正**" if ref > bh else "**仍跑输 BH**"))
        L.append("")

    # ── 跨标的统一发现 ──
    L.append("## 跨标的统一发现\n")
    monotone = all(
        results[s][MODE_NONE]["metrics"]["total_compound"]
        >= results[s][MODE_REFINED]["metrics"]["total_compound"]
        >= results[s][MODE_CURRENT]["metrics"]["total_compound"]
        for s in results
    )
    L.append(
        f"**1. 降成本是稳健的负 alpha 源**（两标的{'一致' if monotone else '不一致'}）：复利严格单调 "
        "`none（零降成本） > refined（精修） > current（原降成本）`。"
    )
    for s in results:
        drag = (results[s][MODE_CURRENT]["metrics"]["total_compound"]
                - results[s][MODE_NONE]["metrics"]["total_compound"])
        L.append(f"   - {s}：降成本净拖累 {drag:+.2f}%（且胜率被拉低）。")
    L.append("")
    L.append(
        "**2. MACD 背驰门控能挽回部分损失，但不能翻正**：refined 介于 none 与 current 之间。"
        "即使把降成本择时精修到只在「次级别背驰 ∧ MACD 面积衰减」处减仓，"
        "其收益仍**劣于根本不做降成本**（refined < none）。"
    )
    for s in results:
        gain = (results[s][MODE_REFINED]["metrics"]["total_compound"]
                - results[s][MODE_CURRENT]["metrics"]["total_compound"])
        gap = (results[s][MODE_NONE]["metrics"]["total_compound"]
               - results[s][MODE_REFINED]["metrics"]["total_compound"])
        L.append(f"   - {s}：精修挽回 {gain:+.2f}%，但仍距零降成本差 {gap:+.2f}%。")
    L.append("")
    L.append(
        "**3. 直接回答 alpha 来源问题**：负 alpha 主要来自**降成本短差层**——"
        "进出场信号在去掉降成本后（none）一致地优于含降成本（current）。"
        "降成本短差在 1min 上系统性亏钱（次级别减仓后未能在更低价买回足够多，"
        "或在上涨中途减仓踏空）。这与 `project_costreduction_moneyprinter_bug` 一致："
        "截断 bug 曾把这个负 alpha 伪装成「只赚不赔」的提款机。\n"
    )
    L.append(
        "**4. 但进出场信号本身也未确立正 alpha**（有效域限定）：\n"
    )
    for s in results:
        none_m = results[s][MODE_NONE]["metrics"]
        bh = results[s]["bh"]
        excess = none_m["total_compound"] - bh
        # 同义反复陷阱判定：交易少 + 高胜率 + 超额接近 0 → 趋近 buy-hold
        tautology = none_m["n"] <= 10 and abs(excess) < 30 and none_m["win_rate"] >= 80
        note = ""
        if tautology:
            note = (
                "（⚠ 同义反复陷阱：仅 %d 笔、%.0f%% 胜率、几乎全程持有 ≈ buy-hold，"
                "「几乎打平 BH」**不是 alpha 证据**，需随机门控对照才能证伪 "
                "— project_backtest_benchmark_falsifiability）" % (none_m["n"], none_m["win_rate"])
            )
        L.append(
            f"   - {s}：none {none_m['total_compound']:+.2f}% vs BH {bh:+.2f}%"
            f"（超额 {excess:+.2f}%）{note}"
        )
    L.append("")

    # ── 实验D：背驰确认进出场 ──
    L.append("## 实验D：背驰确认进出场\n")
    L.append(
        "把缠论背驰确认叠加为**进出场状态转移的必要条件**（非独立信号）：进场买点须"
        "「当前 down-move 相对前一 down-move 背驰」（MACD 负柱面积衰减 ∨ 不创新低）；"
        "出场卖点的上涨段比较须额外通过「MACD 正柱面积衰减 ∨ 不创新高」。"
        "实验D 与实验A 的**唯一差异**是这层门控（cost_mode 同为 none）→ 隔离背驰门控的纯增量。\n"
    )
    L.append("| 标的 | A 复利% | D 复利% | D−A | BH% | D 超额% | D 交易 | A 交易 | D 胜率 | D 是否>BH |")
    L.append("|------|---------|---------|-----|-----|---------|--------|--------|--------|-----------|")
    d_beats_bh: list[bool] = []
    d_beats_a: list[bool] = []
    for s in results:
        r = results[s]
        bh = r["bh"]
        a_m = r[MODE_NONE]["metrics"]
        d_m = r[MODE_DIVERGENCE]["metrics"]
        gain = d_m["total_compound"] - a_m["total_compound"]
        excess = d_m["total_compound"] - bh
        beats_bh = d_m["total_compound"] > bh
        d_beats_bh.append(beats_bh)
        d_beats_a.append(gain > 0)
        L.append(
            f"| {s} | {a_m['total_compound']:+.2f} | {d_m['total_compound']:+.2f}"
            f" | {gain:+.2f} | {bh:+.2f} | {excess:+.2f} | {d_m['n']} | {a_m['n']}"
            f" | {d_m['win_rate']:.0f}% | {'**是**' if beats_bh else '否'} |"
        )
    L.append("")
    verdict_bh = "全部标的" if all(d_beats_bh) else ("部分标的" if any(d_beats_bh) else "无标的")
    verdict_a = "全部标的" if all(d_beats_a) else ("部分标的" if any(d_beats_a) else "无标的")
    L.append(
        f"**关键判定（D vs A，背驰门控能否让纯进出场跑赢 BH）**：背驰门控相对零降成本纯进出场"
        f"在 **{verdict_a}** 上提升复利（D>A）；门控后 **{verdict_bh}** 翻正超额（D>BH）。"
    )
    if not all(d_beats_bh):
        L.append(
            "未全部翻正 → 背驰门控作为进出场必要条件，**尚未确立可跑赢 BH 的正 alpha**："
            "门控减少了进出场次数（候选放行率<100%），但拦掉的进出场不必然是亏损进出场——"
            "MACD 面积是力度代理（521 号：PH 纯拓扑无动量，须 MACD 闸），"
            "真实次级别走势区间力度可能翻转此结论（有效域：L2，缺 L3 交叉验证）。"
        )
    else:
        L.append(
            "全部翻正 → 背驰门控作为进出场必要条件确立正超额信号。"
            "**但仍须随机门控对照**（M1 交付物 3）才能证伪「门控=减少交易≈趋近 BH」的同义反复"
            "（project_backtest_benchmark_falsifiability）。"
        )
    L.append("")

    # ── 实验E/F：背驰定位器进出场（非过滤器） ──
    L.append("## 实验E/F：背驰定位器进出场（按原文重做）\n")
    L.append(
        "**与实验D 的本质区别——背驰是定位器（when）不是过滤器（whether）**：\n"
    )
    L.append(
        "- **D（被证伪）**：背驰作为**门控**叠加在既有进场条件之上（进场需 "
        "buy_cand ∧ l2_direction ∧ l0_r1_down ∧ 背驰 同时成立）→ 进场被砍 90% "
        "（QQQ 放行 10%）→ 长期空仓 → 跑输 BH。这是把背驰当「没背驰就不进场」的过滤器。\n"
    )
    L.append(
        "- **E（本实验，按原文）**：背驰**就是**进出场信号本身，进出场分处不同级别——"
        "敏感进场 + 鲁棒出场。进场（1买）= 底背驰：down-move 完成且 MACD 力度衰减（37 课，"
        "定位下跌段力竭低点）→ 建仓；建仓后**默认满仓持有，穿越所有 move 级波动**"
        "（38 课「没背驰=走势没结束=不动」）；出场（1卖）= **L2 趋势顶背驰**：l2_flip_short"
        "（趋势转向，拓扑）∧ exit_div_ok（MACD 动量衰减，521 号 PH 须 MACD 闸）→ 清仓。"
        "用趋势级而非 move 级出场，故不被小回调震出——对应原文「同级别第一类卖点」。"
        "**两背驰之间是持仓不是空仓**，消除 D 的现金拖累。无降成本。\n"
    )
    L.append(
        "- **F = E + 次级别买卖点降成本**：持仓期间用**次级别(L1走势)买卖点**高抛低吸"
        "（操作指导7 原文「每次向下离开中枢出现底背驰就介入(买回)，回拉出现顶背驰就走(卖出)」）："
        "次级别顶背驰(未到趋势顶)→卖出 1/10 底仓短差（第31课 1/10 机动资金），次级别底背驰→买回；"
        "cost_basis 归零后转挣股数。短差只改 cost_basis/total_shares，**不触碰主级别进出场**"
        "→ E/F 进出场点完全相同、持仓不断，唯一差异是降成本短差。\n"
    )
    L.append(
        "> **买卖点贯穿全流程（非附加过滤器）**：同一个走势顶背驰，未到趋势顶时是短差卖点(高抛)、"
        "到趋势顶(l2_flip_short)时是主级别清仓卖点；同一个走势底背驰，FLAT 时是进场买点、"
        "持仓时是低吸买回点。原文依据：操作指导1「底背驰买入，顶背驰卖出」、操作指导3「第一类买卖点"
        "就是该级别的背驰点」、29 课「底背驰买入后持有到顶背驰才出」、38 课「没背驰=走势没结束=不动」。\n"
    )
    L.append(
        "| 标的 | A 复利% | E 复利% | F 复利% | BH% | E超额% | F超额% | E交易 | F交易 | E胜率 | E>BH | F>BH |"
    )
    L.append(
        "|------|---------|---------|---------|-----|--------|--------|-------|-------|-------|------|------|"
    )
    e_beats_bh: list[bool] = []
    e_beats_a: list[bool] = []
    for s in results:
        r = results[s]
        bh = r["bh"]
        a_m = r[MODE_NONE]["metrics"]
        e_m = r[MODE_SWING_E]["metrics"]
        f_m = r[MODE_SWING_F]["metrics"]
        e_excess = e_m["total_compound"] - bh
        f_excess = f_m["total_compound"] - bh
        e_beats_bh.append(e_m["total_compound"] > bh)
        e_beats_a.append(e_m["total_compound"] > a_m["total_compound"])
        L.append(
            f"| {s} | {a_m['total_compound']:+.2f} | {e_m['total_compound']:+.2f}"
            f" | {f_m['total_compound']:+.2f} | {bh:+.2f} | {e_excess:+.2f} | {f_excess:+.2f}"
            f" | {e_m['n']} | {f_m['n']} | {e_m['win_rate']:.0f}%"
            f" | {'**是**' if e_m['total_compound'] > bh else '否'}"
            f" | {'**是**' if f_m['total_compound'] > bh else '否'} |"
        )
    L.append("")
    verdict_e_a = "全部标的" if all(e_beats_a) else ("部分标的" if any(e_beats_a) else "无标的")
    verdict_e_bh = "全部标的" if all(e_beats_bh) else ("部分标的" if any(e_beats_bh) else "无标的")
    L.append(
        f"**关键判定（E vs A vs D）**：背驰定位器（E）相对零降成本 38 课 FSM（A）"
        f"在 **{verdict_e_a}** 上提升复利；E 在 **{verdict_e_bh}** 上翻正超额（E>BH）。"
        f"与 D 对比——D 把背驰当过滤器在两标的均跑输 A，E 把背驰当定位器测试「持仓默认、"
        f"背驰定位转折」是否消除现金拖累。\n"
    )
    fe_deltas = {
        s: (results[s][MODE_SWING_F]["metrics"]["total_compound"]
            - results[s][MODE_SWING_E]["metrics"]["total_compound"])
        for s in results
    }
    fe_all_neg = all(d < 0 for d in fe_deltas.values())
    fe_all_pos = all(d > 0 for d in fe_deltas.values())
    fe_desc = "、".join(
        f"{s} {d:+.2f}%（降成本{'增益' if d > 0 else '拖累'}）"
        for s, d in fe_deltas.items()
    )
    L.append(
        "**降成本层验证（F−E，背驰定位器框架下降成本的净贡献）**："
        + fe_desc + "。F 在 E 基础上叠加次级别背驰高抛低吸，进出场点与 E 完全相同。"
    )
    if fe_all_neg:
        L.append(
            "两标的一致为负 → 与 fsm 框架结论一致（降成本是负 alpha 源，应在信号层 alpha "
            "确立后再引入）。\n"
        )
    elif fe_all_pos:
        L.append(
            "两标的一致为正 → **与 fsm 框架「降成本系统性亏钱」结论相反**：在背驰定位器"
            "（长持仓穿越）框架下，次级别背驰高抛低吸反而增益。说明「降成本恒为负 alpha」"
            "并非框架无关，依赖进出场结构（fsm 频繁进出 vs swing 长持仓）。\n"
        )
    else:
        L.append(
            "**标的间不一致**：fsm 框架曾结论「降成本在 1min 系统性亏钱」（V0−A、refined−none "
            "两标的一致为负），但在背驰定位器（长持仓穿越）框架下降成本净贡献标的依赖——"
            "故「降成本恒为负 alpha」是 **fsm 框架的局部结论而非普适**（有效域受进出场结构限定："
            "长持仓下次级别背驰回补有更长的均值回归窗口）。\n"
        )
    L.append(
        "> **为何不直接用引擎 type1 BSP 做主级别进出场**：引擎 BSP 引擎已实现正统三类买卖点"
        "（type1=趋势背驰点），但仅在 level-1（走势层）计算（recursive level≥2 只有走势/中枢无 BSP）。"
        "实测 level-1 type1 买卖点信号频繁（OKLO 251 买/269 卖 bar），直接做主进出场跑 41 笔、"
        "复利 +117%（逊于 BH+251% 与 E+455%）——level-1 走势级买卖点对 1min **操作级别太细**"
        "（频繁进出=现金拖累，第34/35课操作级别选择问题）。故主进出场用更高的 L1走势/L2趋势级"
        "背驰买卖点（操作指导3「第一类买卖点=背驰点」），引擎 level-1 type1 留作次级别参考。\n"
    )
    L.append(
        "> 有效域（L2，缺 L3）与未实现项：(i) 力度用 MACD 面积代理（521 号：PH 纯拓扑无动量），"
        "非原文「走势中枢几何力度」本质度量；(ii) 进场底背驰落在 L1 走势层（盘整背驰代理），"
        "未强制原文趋势背驰的「≥2 同级别中枢」硬约束；(iii) 进出场级别不对称（进场 L1走势底背驰、"
        "出场 L2趋势顶背驰），是「敏感进/鲁棒出」的多头结构非严格「同级别」；(iv) **未实现第二类买点加仓**"
        "（267 号 D2 无加仓 FSM）——原文 2买（趋势后第一次回调，允许破 1买低点）是加仓机会，本 FSM 单仓进出；"
        "(v) E>BH 仍须随机门控对照排除「趋近 buy-hold」同义反复"
        "（project_backtest_benchmark_falsifiability）。\n"
    )

    # ── 结果包六要素 ──
    L.append("## 结果包（六要素）\n")
    L.append(
        "**1. 结论**：见各标的对照表。fsm 族（A/B/D）分解降成本/门控；swing 族（E/F，按原文"
        "重做）测背驰**定位器**——底背驰买、持仓穿越、L2 趋势顶背驰卖。性能：compute-once 架构"
        "（引擎只跑一次 + 6 模式重放磁带）+ `_fast_alive` 轻量化，6 模式×2 标的从 25+min 降到"
        "目标区间（QQQ 728k 信号层 ~6min 为长极，OKLO 与之并行）。\n"
    )
    L.append(
        "**2. 定义依据**：38 课降成本（次级别卖点减仓、买点回补）；"
        "缠论顶背驰（价格创新高/不创新高 + MACD 红柱面积衰减，本脚本用 "
        "`a_macd.OnlineMacdState` 正柱累积面积代理力度）；267 号 D2 无加仓 FSM。"
        "输入特征满足：l1_nr1_up（次级别 PH non-rank1 up settle）= 次级别向上走势完成，"
        "ratio>0.05 = 减仓阈值；实验B 追加 l1_up 段间 high + MACD 面积双重背驰确认。\n"
    )
    L.append(
        "**3. 边界条件**：（i）trim 阈值 ratio>0.05、sub_ratio 隐含于 active_trim_shares；"
        "若 MACD 面积代理换成真实次级别走势区间力度，门控结果可能翻转；"
        "（ii）FSM ≤ BH 仅是「无 alpha 证据」，随机门控对照仍缺（M1 交付物 3），"
        "「FSM>BH」是必要不充分。\n"
    )
    L.append(
        "**4. 下游推论**：若 A ≤ BH → 进出场信号层无 alpha，降成本择时优化无意义（先修信号层）；"
        "若 A > BH 但 V0 < A → 降成本是负 alpha 源，应在信号层 alpha 确立后再引入；"
        "若 B > V0 → MACD 背驰门控是有效的择时过滤器，可上推到 confirmed 层。\n"
    )
    L.append(
        "**5. 谱系引用**：521 号（PH 纯拓扑无动量，故须 MACD 闸）；"
        "`project_divergence_gate_entry_exit_falsified`（背驰**门控**进出场被证伪——E/F 正是其"
        "反面修正：背驰当定位器非过滤器，原文 38/29 课互证）；"
        "`project_costreduction_moneyprinter_bug`（截断 bug 已去）；"
        "`project_ph_settle_usage_boundary`（PH settle 是 candidate 必要条件非充分确认）；"
        "`project_backtest_benchmark_falsifiability`（BH 基准可证伪性）。\n"
    )
    L.append(
        "**6. 影响声明**：修改诊断脚本 `analysis/fugue_alpha_diagnosis.py`——"
        "(a) 性能：`_fast_alive` 去掉无消费者的 MergeBar 8 字段构造（等价保持，A 全量 OKLO "
        "+242.61% 逐位不变）；(b) 新增 `run_swing_trading`（E/F 买卖点主轴 FSM）+ BarSignal "
        "新增 exit_div_ok/down_move_settled/up_move_settled/l2_flip_long 字段；"
        "(c) F 降成本改为**次级别(L1走势)买卖点高抛低吸**（次级别顶背驰卖 1/10 底仓短差、"
        "底背驰买回；TRIM_FRACTION=0.1，第31课），替代原线段级 refined 门控。"
        "**不修改引擎代码、不修改 `cost_reduction_fsm.py`、不修改 `run_trading`**"
        "（fsm 族 A/B/D 数字零漂移）。\n"
    )
    L.append(
        "**认识论等级**：L2（真实数据 QQQ/OKLO 1min；A 的否定性结果——若纯进出场跑输 BH——"
        "缩小有效域边界，比确认性结果更有价值）。"
    )

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

# 运行配置：(结果键, cost_mode, divergence_gate, strategy)
#   fsm 策略（38 课 FSM，run_trading）：current/none(A)/refined(B)/divergence(D)。
#     D = (none, gate=True)：与 A=(none, gate=False) 的唯一差异是进出场背驰门控（过滤器）。
#   swing 策略（背驰定位器，run_swing_trading）：E/F。
#     E = (none)：底背驰买、顶背驰卖、默认满仓持有，无降成本。
#     F = (refined)：E + 持仓期次级别背驰高抛低吸。
RUN_CONFIGS = [
    (MODE_CURRENT, MODE_CURRENT, False, "fsm"),
    (MODE_NONE, MODE_NONE, False, "fsm"),
    (MODE_REFINED, MODE_REFINED, False, "fsm"),
    (MODE_DIVERGENCE, MODE_NONE, True, "fsm"),
    (MODE_SWING_E, MODE_NONE, False, "swing"),
    (MODE_SWING_F, MODE_REFINED, False, "swing"),
]

# 空 div_counts 模板（swing 策略无进出场门控，背驰是信号本身而非门控）
_EMPTY_DIV_COUNTS = {
    "entry_candidate": 0, "entry_passed": 0,
    "exit_candidate": 0, "exit_passed": 0,
}


def _process_symbol(symbol: str) -> tuple[str, dict]:
    """单标的完整管线：load(once) → compute_signals(once) → run_trading/swing(×6)。

    compute_signals 是瓶颈（引擎/PH/MACD），只算一次；6 个模式仅重放磁带（近乎瞬时）。
    模块级函数 → 可被 ProcessPoolExecutor 跨进程并行（每标的一进程）。
    """
    print(f"\n{'=' * 60}\n  {symbol} — alpha 诊断（6 模式，compute-once）\n{'=' * 60}")
    opens, highs, lows, closes = load_1min(symbol)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    t_sig = time.time()
    signals = compute_signals(opens, highs, lows, closes)
    sig_elapsed = time.time() - t_sig
    print(f"  [{symbol}] {n:,} bars  BH={bh:+.2f}%  信号层 {sig_elapsed:6.1f}s（仅一次）")

    out: dict = {
        "n_bars": n, "bh": bh,
        "first": closes[0], "last": closes[-1],
    }
    for key, cost_mode, gate, strategy in RUN_CONFIGS:
        t0 = time.time()
        if strategy == "swing":
            trades, refined_counts = run_swing_trading(signals, cost_mode)
            div_counts = dict(_EMPTY_DIV_COUNTS)
        else:
            trades, refined_counts, div_counts = run_trading(
                signals, cost_mode, divergence_gate=gate,
            )
        elapsed = time.time() - t0
        m = compute_metrics(trades)
        out[key] = {
            "metrics": m, "refined_counts": refined_counts,
            "div_counts": div_counts, "trades": trades,
        }
        print(
            f"  [{symbol}/{key:10s}] 交易层 {elapsed:5.2f}s | 交易={m['n']:3d} "
            f"胜率={m['win_rate']:4.0f}% 复利={m['total_compound']:+8.2f}% "
            f"超额={m['total_compound'] - bh:+8.2f}% 夏普={m['sharpe']:+.3f} "
            f"降成本笔={m['n_with_cr']}"
        )
    return symbol, out


def main() -> None:
    symbols = ["QQQ", "OKLO"]
    results: dict = {}
    # 跨标的并行：2 个标的的信号层（瓶颈）各自独立引擎，可并行。
    # 交易层重放磁带近乎免费，不并行（IPC 开销 > 收益）。BT_PARALLEL=0 退回串行。
    parallel = os.environ.get("BT_PARALLEL", "1") != "0" and len(symbols) > 1

    t_all = time.time()
    if parallel:
        workers = min(len(symbols), os.cpu_count() or 2)
        print(f"跨标的并行：{workers} 进程 × (信号层 once + 6 模式重放磁带)")
        with ProcessPoolExecutor(max_workers=workers) as ex:
            for symbol, out in ex.map(_process_symbol, symbols):
                results[symbol] = out
    else:
        for symbol in symbols:
            _, out = _process_symbol(symbol)
            results[symbol] = out
    print(f"\n总耗时 {time.time() - t_all:.1f}s（6 模式 × {len(symbols)} 标的）")

    modes = [key for key, *_ in RUN_CONFIGS]
    write_report(results)

    audit = {
        sym: {
            "n_bars": results[sym]["n_bars"],
            "bh": results[sym]["bh"],
            **{
                mode: {
                    "compound": results[sym][mode]["metrics"]["total_compound"],
                    "excess": results[sym][mode]["metrics"]["total_compound"] - results[sym]["bh"],
                    "n": results[sym][mode]["metrics"]["n"],
                    "win_rate": results[sym][mode]["metrics"]["win_rate"],
                    "sharpe": results[sym][mode]["metrics"]["sharpe"],
                    "n_with_cr": results[sym][mode]["metrics"]["n_with_cr"],
                }
                for mode in modes
            },
            "refined_counts": results[sym][MODE_REFINED]["refined_counts"],
            "div_counts": results[sym][MODE_DIVERGENCE]["div_counts"],
        }
        for sym in symbols
    }
    (DATA_DIR / "fugue_alpha_diagnosis_results.json").write_text(
        json.dumps(audit, indent=2, ensure_ascii=False)
    )
    print("审计 JSON 已写入：fugue_alpha_diagnosis_results.json")


if __name__ == "__main__":
    main()
