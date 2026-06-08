"""完整版缠论+PH多重赋格操盘系统 — G/H（在 E/F 简化实验上补全 7 模块）。

E/F 是简化实验（`fugue_alpha_diagnosis.py`）。本脚本是正式版，一次性补全设计文档
`docs/architecture/complete_fugue_design.md` 中的全部缺失模块：

  1. 4 层递归级别架构（显式化级别→PH 映射）
       L2 趋势级（move-level PH，l2_flip/l2_direction）—— 向上段/向下段切换语境
       L1 操作级（move settle + 背驰）—— 进出场（底背驰 1 买、顶背驰 1 卖）
       L0 次操作级（segment-level PH + 背驰）—— 降成本/挣股数（次级别背驰高抛低吸）
  2. 区间套精确入场（递归展开，非独立过滤器）
       L1 确认买点（趋势下跌底背驰）→ ARM，不在当前 bar 进场；
       L0 定位精确入场价（l0_r1_down 底背驰 / type1 买点）→ 建仓。
  3. 2 买实现（趋势后第一次回调不破 1 买低点 = 2 买）
       消费引擎 type2 买点候选 + 回调低点 ≥ 1 买低点 → 标记加仓位置
       （267 号 D2 无加仓 FSM —— 加仓已移除，本版只计数标记，不实际加仓）。
  4. Persistence 过滤（38 课走势完成 vs 噪声）
       高 persistence 的 up-move settle → 触发退出评估（顶背驰 → 卖）；
       低 persistence 的 settle → 只触发降成本（不退出，穿越噪声）。
  5. 中枢判定（趋势 vs 盘整）
       1 买要求趋势下跌（down-move zs_count ≥ 2，至少经过 2 个中枢 = 趋势背驰）；
       盘整背驰（zs_count == 1）不构成 1 买。
  6. 完整三阶段降成本 FSM 接入（`cost_reduction_fsm.py`）
       POSITION_OPEN → COST_REDUCING（股数守恒，降 cost_basis）
       → PRINCIPAL_WITHDRAWN / EARNING_SHARES（cost_basis≤0，金额守恒，挣股数）。
  7. 降成本触发改为次级别买卖点（非每个 PH settle）
       次级别（segment）顶背驰 = SUB_LEVEL_SELL_POINT（trim 减仓）；
       次级别底背驰/买点 = SUB_LEVEL_BUY_POINT（回补买回）。

对照组（BH / A / E / F / G / H / I）：
  A = 38 课 FSM 纯进出场（run_trading none，复用 E/F 脚本）
  E = 背驰定位器简化版（run_swing_trading none，复用）
  F = E + 简化降成本（run_swing_trading refined，复用）
  G = 完整版（本脚本 run_complete，区间套+中枢趋势门+persistence退出+2买标记，无降成本）
  H = 完整版 + 三阶段 FSM 降成本 + 挣股数（本脚本 run_complete_h）
  I = 最佳组合（本脚本 run_complete_i）= E 的出场（L2 趋势翻转 l2_flip_short∧exit_div_ok）
      + H 的进场和持仓（区间套+中枢趋势门+三阶段 FSM 降成本+次级别背驰触发+2买标记）。
      诊断动机：G/H 把出场降为 L1 操作级 persistence 退出，在强趋势标的被 move 级回调震出
      （OKLO G+267% < E+455%）；I 把出场升回 E 的 L2 趋势级，保留 H 的完整进场持仓。

性能（compute-once 复用）：引擎/PH/MACD 在所有模式间相同，抽离为一次 compute_signals_v2
→ BarSignalV2 超集磁带；5 个交易模式重放磁带（近乎瞬时）。BarSignalV2 是 BarSignal 的
严格超集 → 复用 E/F 脚本的 run_trading/run_swing_trading（鸭子类型，A/E/F 数字逐位复现）。

认识论等级：L2（真实数据 QQQ/OKLO 1min；含否定性结果）。
521 号限定：信号属 candidate 层（PH 门控 + MACD 面积力度代理），非 confirmed 买卖点。
谱系引用：521 号（PH 纯拓扑无动量须 MACD 闸）、267 号（满仓满融降成本 + D2 无加仓）、
268a（own_capital 独立核算）、project_divergence_locator_entry_exit（E/F 背驰定位器）、
project_costreduction_moneyprinter_bug（截断 bug 已去）、
project_backtest_benchmark_falsifiability（BH 基准可证伪性）。
"""

from __future__ import annotations

import json
import os
import sys
import time
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))  # spawn 子进程可 import fugue_alpha_diagnosis

from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
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

# ── 复用 E/F 实验脚本的全部构件（不重新实现 → A/E/F 数字逐位复现）──
import fugue_alpha_diagnosis as _ef  # noqa: E402
from fugue_alpha_diagnosis import (  # noqa: E402
    INITIAL_CAPITAL,
    MODE_NONE,
    MODE_REFINED,
    PHLevelState,
    SUB_EXPIRY,
    CompletedTrade,
    MoveRecord,
    UpSegRecord,
    _median_alive_persistence,
    _move_force_simple,
    _persistence_ratio,
    _refined_gate,
    check_divergence,
    compute_metrics,
    load_1min,
    run_swing_trading,
    run_trading,
)

# BarSignal 与 BarSignalV2 共享字段（tape-equality 复现证明：V2 ≡ 原始 compute_signals）
_SHARED_FIELDS = (
    "close", "l0_r1_down", "l0_r1_up", "l1_nr1_up", "l1_nr1_down",
    "l2_flip_long", "l2_flip_short", "l2_direction", "buy_cands", "sell_cands",
    "buy_invalidates", "med_persistence", "l1_up_ratio", "refined_gate_ok",
    "entry_div_ok", "exit_div_ok", "down_move_settled", "up_move_settled",
)
# 嵌入式 tape-equality 校验前缀长度（前 N bar 跑原始 compute_signals 对比）
VALIDATE_N = 30000

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_complete_v2_results.md"

# 持仓期回调内 2 买窗口（与 SUB_EXPIRY 同尺度）
PULLBACK_EXPIRY = SUB_EXPIRY
# 滚动中位数窗口（persistence 高/低分类，近期同向 move 的 persistence 中位数）
PERSIST_MED_WINDOW = 50

# 模式键
MODE_A = "A_fsm_none"          # 38 课 FSM 纯进出场（复用 run_trading none）
MODE_E = "E_swing_none"        # 背驰定位器简化版（复用 run_swing none）
MODE_F = "F_swing_refined"     # E + 简化降成本（复用 run_swing refined）
MODE_G = "G_complete"          # 完整版（本脚本）
MODE_H = "H_complete_cost"     # 完整版 + 三阶段 FSM 降成本 + 挣股数（本脚本）
MODE_I = "I_best_combo"        # 最佳组合：E 出场（L2 趋势翻转）+ H 进场/持仓（本脚本）


# ════════════════════════════════════════════════════════════
# 信号磁带超集（BarSignalV2 = BarSignal ∪ G/H 新字段）
# ════════════════════════════════════════════════════════════

@dataclass(slots=True)
class BarSignalV2:
    """单 bar 的 mode-independent 信号超集。

    前 19 个字段与 `fugue_alpha_diagnosis.BarSignal` 同名同义 → run_trading /
    run_swing_trading 鸭子类型消费本磁带，A/E/F 数字逐位复现。后 7 个字段是
    G/H 完整版新增（中枢趋势门、persistence 分类、2 买、次级别背驰降成本触发）。
    """

    # ── BarSignal 兼容字段（A/E/F 复用，逐位等价）──
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
    exit_div_ok: bool
    down_move_settled: bool
    up_move_settled: bool
    # ── G/H 完整版新增字段 ──
    entry_zs_count: int        # 模块5：本 bar settle 的 down-move 中枢数（≥2=趋势下跌=1买门）
    exit_zs_count: int         # 本 bar settle 的 up-move 中枢数（信息性）
    entry_persistence_high: bool  # 模块4：down-move persistence ≥ 近期中位数（信息性）
    exit_persistence_high: bool   # 模块4：up-move persistence ≥ 近期中位数（高→退出评估）
    type2_buy: bool            # 模块3：本 bar 出现 type2 买点候选（2买）
    sub_sell_signal: bool      # 模块7：次级别（segment）顶背驰 → trim 减仓
    sub_buy_signal: bool       # 模块7：次级别（segment）底背驰/买点 → 回补买回


def _median(values: list[float]) -> float:
    if not values:
        return 0.0
    s = sorted(values)
    n = len(s)
    if n % 2 == 1:
        return s[n // 2]
    return (s[n // 2 - 1] + s[n // 2]) / 2


def _refined_gate_down(l1_down_segs: list[tuple[float, float]]) -> bool:
    """次级别底背驰门控（_refined_gate 的下跌对称版）。

    条件2（次级别底背驰，价格代理）：当前 L1 向下段相对前段——
        不创新低（cur_low >= prev_low，底部衰竭）OR
        创新低但 MACD 负柱面积衰减（盘整/标准背驰）。
    条件3（MACD 面积确认）：当前段 MACD 负柱面积 < 前段（下跌动量衰减）。
    两者合取。无前序段（<2）默认不放行。
    """
    if len(l1_down_segs) < 2:
        return False
    cur_low, cur_area = l1_down_segs[-1]
    prev_low, prev_area = l1_down_segs[-2]
    cond3 = cur_area < prev_area  # 下跌 MACD 动量衰减
    no_new_low = cur_low >= prev_low
    consol_div = cur_low < prev_low and cur_area < prev_area
    cond2 = no_new_low or consol_div
    return cond2 and cond3


# ════════════════════════════════════════════════════════════
# 信号层（compute-once）— BarSignal 兼容计算逐字复制 + G/H 新字段扩展
#
# 引擎/PH/MACD 计算与 fugue_alpha_diagnosis.compute_signals 完全一致（保证 A/E/F
# 逐位复现）；仅在 settle 事件处额外捕获 zs_count/persistence、type2 买点、次级别
# 上/下段背驰，组装为 BarSignalV2 超集。
# ════════════════════════════════════════════════════════════

def compute_signals_v2(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> list[BarSignalV2]:
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id="bt_v2", max_levels=2)
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
    l1_down_segs: list[tuple[float, float]] = []  # G/H：次级别向下段（low, neg_area）→ 底背驰
    last_l1down_i = 0
    down_move_hist: list[MoveRecord] = []
    up_move_hist: list[MoveRecord] = []
    l2_direction = 0

    # G/H：滚动 persistence 中位数（高/低分类）
    down_persists: list[float] = []
    up_persists: list[float] = []

    signals: list[BarSignalV2] = []
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

        _, _, hist = macd.update(c, ts)
        pos_cum += hist if hist > 0 else 0.0
        pos_cum_hist.append(pos_cum)
        neg_cum += -hist if hist < 0 else 0.0
        neg_cum_hist.append(neg_cum)

        snap = orch.process_bar(bar)
        bsp_events = snap.bsp_snapshot.events
        moves = snap.move_snapshot.moves

        # L0 PH（每 bar close）
        l0_ds = l0_down.tree.update(c)
        l0_us = l0_up.tree.update(-c)
        l0_r1_down, _ = l0_down.detect_settle(l0_ds)
        l0_r1_up, _ = l0_up.detect_settle(l0_us)

        # L1 PH（segment settle）+ 次级别上/下段力度记录
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
                if e.ep1_price > e.ep0_price:
                    # 向上段
                    seg_lo = last_l1up_i + 1 if last_l1up_i + 1 <= i else i
                    seg_high = max(highs[seg_lo:i + 1]) if seg_lo <= i else h
                    seg_area = _macd_pos_area(last_l1up_i, i)
                    l1_up_segs.append((seg_high, seg_area))
                    last_l1up_i = i
                elif e.ep1_price < e.ep0_price:
                    # G/H：向下段（次级别底背驰用）
                    seg_lo2 = last_l1down_i + 1 if last_l1down_i + 1 <= i else i
                    seg_low = min(lows[seg_lo2:i + 1]) if seg_lo2 <= i else lo
                    seg_narea = _macd_neg_area(last_l1down_i, i)
                    l1_down_segs.append((seg_low, seg_narea))
                    last_l1down_i = i

        # L2 PH（move settle）+ G/H 中枢数/persistence 捕获
        l2_flip_long = False
        l2_flip_short = False
        down_move_settled = False
        up_move_settled = False
        entry_zs_count = 0
        exit_zs_count = 0
        entry_persistence_high = False
        exit_persistence_high = False
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
                if mv.direction == "down":
                    down_move_hist.append(MoveRecord(
                        direction="down", high=mv.high, low=mv.low,
                        macd_area=_macd_neg_area(mv.first_seg_s0, mv.last_seg_s1),
                    ))
                    down_move_settled = True
                    # 模块5：中枢数（≥2=趋势下跌=1买门）
                    entry_zs_count = mv.zs_count
                    # 模块4：persistence 高/低（相对近期 down-move 中位数）
                    med_d = _median(down_persists)
                    entry_persistence_high = (
                        mv.persistence >= med_d if down_persists else True
                    )
                    down_persists.append(mv.persistence)
                    if len(down_persists) > PERSIST_MED_WINDOW:
                        down_persists.pop(0)
                else:
                    up_move_hist.append(MoveRecord(
                        direction="up", high=mv.high, low=mv.low,
                        macd_area=_macd_pos_area(mv.first_seg_s0, mv.last_seg_s1),
                    ))
                    up_move_settled = True
                    exit_zs_count = mv.zs_count
                    med_u = _median(up_persists)
                    exit_persistence_high = (
                        mv.persistence >= med_u if up_persists else True
                    )
                    up_persists.append(mv.persistence)
                    if len(up_persists) > PERSIST_MED_WINDOW:
                        up_persists.pop(0)
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

        # BSP 事件（含 type2 买点 = 2买）
        buy_cands: list[int] = []
        sell_cands: list[int] = []
        buy_invalidates: list[int] = []
        type2_buy = False
        for e in bsp_events:
            nm = type(e).__name__
            bid = e.bsp_id
            side = e.side
            if "Candidate" in nm:
                (buy_cands if side == "buy" else sell_cands).append(bid)
                if side == "buy" and getattr(e, "kind", "") == "type2":
                    type2_buy = True
            elif "Invalidate" in nm and side == "buy":
                buy_invalidates.append(bid)

        # 新 settle 的 up-move（38 课 S7 段比较，A 复用）
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

        # bar 级预计算标量（A/E/F 复用，逐位等价）
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

        # 模块7：次级别买卖点触发降成本（非每个 PH settle）
        #   次级别顶背驰（segment up 背驰 + MACD 面积衰减）→ trim 减仓
        sub_sell_signal = bool(l1_nr1_up and l1_up_ratio > 0.05 and refined_gate_ok)
        #   次级别底背驰（segment down 背驰）或 type1 买点 → 回补
        if l1_nr1_down:
            l1_down_ratio = _persistence_ratio(l1_down.alive, 1)
            sub_buy_div = l1_down_ratio > 0.05 and _refined_gate_down(l1_down_segs)
        else:
            sub_buy_div = False
        sub_buy_signal = bool(sub_buy_div or bool(buy_cands))

        signals.append(BarSignalV2(
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
            entry_div_ok=entry_div_ok, exit_div_ok=exit_div_ok,
            down_move_settled=down_move_settled, up_move_settled=up_move_settled,
            entry_zs_count=entry_zs_count, exit_zs_count=exit_zs_count,
            entry_persistence_high=entry_persistence_high,
            exit_persistence_high=exit_persistence_high,
            type2_buy=type2_buy,
            sub_sell_signal=sub_sell_signal, sub_buy_signal=sub_buy_signal,
        ))

        if i - last_progress >= 200_000:
            print(f"    [{i / n * 100:5.1f}%] signal bar {i:,}/{n:,}")
            last_progress = i

    return signals


# ════════════════════════════════════════════════════════════
# G：完整版（区间套入场 + 中枢趋势门 + persistence 退出 + 2买标记，无降成本）
# ════════════════════════════════════════════════════════════

# 完整版状态
_FLAT, _ARMED, _LONG = 0, 1, 2


def run_complete(
    signals: list["BarSignalV2"],
) -> tuple[list[CompletedTrade], dict[str, int]]:
    """完整版 G —— 4 层级别 + 区间套精确入场 + 中枢趋势门 + persistence 退出 + 2买标记。

    入场（区间套两阶段，模块1/2/5）：
      阶段1（L1 操作级确认，ARM）：down-move settle ∧ 底背驰（entry_div_ok）∧
        zs_count ≥ 2（趋势下跌，至少 2 中枢 = 趋势背驰 1 买；盘整背驰 zs_count==1 不入）。
        → 不在当前 bar 进场，转 ARMED，等 L0 定位（区间套递归展开，非独立过滤）。
      阶段2（L0 次操作级定位，建仓）：ARMED 中等 l0_r1_down（L0 底背驰）∨ type1 买点
        → 精确入场价建仓；超时（PULLBACK_EXPIRY）退化为操作级入场。

    持仓 = 默认态（38 课"没背驰=走势没结束=不动"），穿越 move 级波动。

    出场（L1 操作级 1 卖，模块4）：up-move settle ∧ exit_persistence_high（高 persistence
      → 触发退出评估）∧ exit_div_ok（顶背驰）→ 清仓。低 persistence 的 up settle 不退出
      （穿越噪声；G 无降成本，低 persistence settle 在 G 中无操作，在 H 中触发降成本）。

    2 买（模块3）：持仓期回调（down-move settle 开始回调跟踪）中出现 type2 买点候选
      且回调低点 ≥ 1 买低点 → 标记加仓机会（267 号 D2 无加仓 FSM，加仓已移除，只计数）。

    L2 角色（模块1）：L2 是向上段/向下段切换语境，非 1 买前置门——1 买是反转进场，
      买在向下段（L2 still down）的趋势背驰处。若用 l2_direction==1 门控入场则永远抓不到
      反转（实验 D 陷阱）。故 long-only 完整版不以 l2_direction 门控入场。
    """
    n = len(signals)
    state = _FLAT
    total_shares = 0.0
    entry_price = 0.0
    entry_bar = -1
    entry_1buy_low = float("inf")
    arm_bar = -1
    arm_dip_low = float("inf")
    in_pullback = False
    pullback_low = float("inf")
    n_addon_signals = 0  # 2买加仓机会（加仓已移除，标记计数）

    trades: list[CompletedTrade] = []

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, total_shares, entry_price, entry_bar, entry_1buy_low
        nonlocal in_pullback, pullback_low
        if total_shares <= 0 or entry_price <= 0:
            state = _FLAT
            return
        pnl_pct = (total_shares * price - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=0, cost_basis_at_exit=entry_price,
        ))
        state = _FLAT
        total_shares = 0.0
        entry_price = 0.0
        entry_bar = -1
        entry_1buy_low = float("inf")
        in_pullback = False
        pullback_low = float("inf")

    for i in range(n):
        sig = signals[i]
        c = sig.close

        if state == _FLAT:
            # 模块2/5：L1 趋势下跌底背驰 → ARM（区间套阶段1）
            if (sig.down_move_settled and sig.entry_div_ok
                    and sig.entry_zs_count >= 2):
                state = _ARMED
                arm_bar = i
                arm_dip_low = c

        elif state == _ARMED:
            arm_dip_low = min(arm_dip_low, c)
            # 区间套阶段2：L0 底背驰/type1 买点 → 精确入场
            if sig.l0_r1_down or bool(sig.buy_cands):
                state = _LONG
                entry_price = c
                entry_bar = i
                total_shares = INITIAL_CAPITAL / c
                entry_1buy_low = arm_dip_low
                in_pullback = False
                pullback_low = float("inf")
            elif (i - arm_bar) > PULLBACK_EXPIRY:
                # 超时 fallback：L0 未给精确点，区间套退化为单级操作入场
                state = _LONG
                entry_price = c
                entry_bar = i
                total_shares = INITIAL_CAPITAL / c
                entry_1buy_low = arm_dip_low
                in_pullback = False
                pullback_low = float("inf")
            elif sig.l2_flip_short:
                # 向下段未结束、趋势再创新低 → 放弃本次 ARM
                state = _FLAT

        elif state == _LONG:
            # 模块4：高 persistence 顶背驰 → L1 操作级 1 卖
            if (sig.up_move_settled and sig.exit_persistence_high
                    and sig.exit_div_ok):
                _close(i, c, "l1_op_top_divergence")
            else:
                # 模块3：2 买追踪（趋势后回调不破 1 买低点 = 2买）
                #   引擎不发 type2 买点（只 type1/type3），故用结构化判定：回调段
                #   （down-move settle 起）的低点 ≥ 1 买低点 ∧ 出现买点候选（回调底确认）
                #   = 2 买。这是原文忠实的 2 买（第二类买点结构），非依赖引擎 kind 标签。
                if sig.down_move_settled:
                    in_pullback = True
                    pullback_low = c
                if in_pullback:
                    pullback_low = min(pullback_low, c)
                    if pullback_low < entry_1buy_low:
                        # 回调破 1 买低点 → 本次回调非 2 买，重置等待下次
                        in_pullback = False
                    elif sig.type2_buy or bool(sig.buy_cands) or sig.l0_r1_down:
                        n_addon_signals += 1
                        # ★ 加仓位置（2买）：原应在此加仓，当前加仓被移除
                        #    （267 号 D2 无加仓 FSM）→ 只标记不操作
                        in_pullback = False

    if state == _LONG and total_shares > 0:
        _close(n - 1, signals[-1].close, "eod_close")

    return trades, {"addon_signals": n_addon_signals}


# ════════════════════════════════════════════════════════════
# H：完整版 + 三阶段降成本 FSM（cost_reduction_fsm.py）+ 挣股数
# ════════════════════════════════════════════════════════════

_COST_OPEN_STATES = frozenset({
    CostState.POSITION_OPEN,
    CostState.COST_REDUCING,
    CostState.PRINCIPAL_WITHDRAWN,
    CostState.EARNING_SHARES,
})
_COST_ACTIVE_DIFF_STATES = frozenset({
    CostState.COST_REDUCING,
    CostState.EARNING_SHARES,
})


def run_complete_h(
    signals: list["BarSignalV2"],
) -> tuple[list[CompletedTrade], dict[str, int]]:
    """完整版 H —— G 的进出场 + 接入 `cost_reduction_fsm.py` 三阶段降成本 + 挣股数。

    进出场点与 G 完全相同（降成本不反馈进出场状态转移）。持仓期间用真实
    CostReductionFSM（满仓满融，own_capital=本金，margin=0）跟踪成本演化（模块6）：

      建仓 → FsmEvent(BUY_POINT_CONFIRMED) → POSITION_OPEN。
      持仓期（模块7，次级别买卖点驱动，非每个 PH settle）：
        sub_sell_signal（次级别顶背驰）→ SUB_LEVEL_SELL_POINT（trim）
          → COST_REDUCING（股数守恒，价差降 cost_basis）。
        sub_buy_signal（次级别底背驰/买点）→ SUB_LEVEL_BUY_POINT（回补）
          → cost_basis ≤ 0 时进 EARNING_SHARES（金额守恒，total_shares 增长）。
      出场（L1 顶背驰）→ 先在出场价回补未闭合短差 → 读 snapshot → pnl。

    事件按 fsm.state 守卫派发（每 bar 至多一个），不触发 IllegalTransitionError——
    no-workaround：不用 try/except 吞概念异常，用状态守卫保证转移合法。

    pnl 与 E/F/F 同式：(cumulative_recovered + total_shares·price − 本金)/本金。
    cumulative_recovered = 已实现短差现金（COST_REDUCING 价差），total_shares 含挣股数增量。
    """
    n = len(signals)
    state = _FLAT
    entry_bar = -1
    entry_price = 0.0
    entry_1buy_low = float("inf")
    arm_bar = -1
    arm_dip_low = float("inf")
    in_pullback = False
    pullback_low = float("inf")
    fsm: CostReductionFSM | None = None
    n_addon_signals = 0
    n_earn_trades = 0  # 进入挣股数阶段的交易数

    trades: list[CompletedTrade] = []

    def _open_fsm(price: float) -> CostReductionFSM:
        f0 = CostReductionFSM.create(
            own_capital=INITIAL_CAPITAL, margin_amount=0.0, sub_ratio=0.3,
        )
        return transition(
            f0, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, price=price, level="L1"),
        )

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, fsm, entry_bar, entry_price, entry_1buy_low
        nonlocal in_pullback, pullback_low, n_earn_trades
        if fsm is None or entry_price <= 0:
            state = _FLAT
            fsm = None
            return
        f = fsm
        # 出场前回补未闭合短差（在出场价），避免 EARNING_SHARES 卖出未回补的现金漏计
        if (f.active_short_diff is not None and f.active_short_diff.is_open
                and f.state in _COST_ACTIVE_DIFF_STATES):
            f = transition(
                f, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, price=price, level="L0"),
            )
        snap = f.snapshot()
        if snap.state == CostState.EARNING_SHARES or f.state == CostState.EARNING_SHARES:
            n_earn_trades += 1
        pnl_pct = (
            snap.cumulative_recovered + snap.total_shares * price - INITIAL_CAPITAL
        ) / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=len(f.completed_short_diffs),
            cost_basis_at_exit=snap.cost_basis,
        ))
        state = _FLAT
        fsm = None
        entry_bar = -1
        entry_price = 0.0
        entry_1buy_low = float("inf")
        in_pullback = False
        pullback_low = float("inf")

    for i in range(n):
        sig = signals[i]
        c = sig.close

        if state == _FLAT:
            if (sig.down_move_settled and sig.entry_div_ok
                    and sig.entry_zs_count >= 2):
                state = _ARMED
                arm_bar = i
                arm_dip_low = c

        elif state == _ARMED:
            arm_dip_low = min(arm_dip_low, c)
            do_enter = sig.l0_r1_down or bool(sig.buy_cands)
            if not do_enter and (i - arm_bar) > PULLBACK_EXPIRY:
                do_enter = True
            if do_enter:
                state = _LONG
                entry_price = c
                entry_bar = i
                entry_1buy_low = arm_dip_low
                fsm = _open_fsm(c)
                in_pullback = False
                pullback_low = float("inf")
            elif sig.l2_flip_short:
                state = _FLAT

        elif state == _LONG:
            assert fsm is not None
            # 出场判定（与 G 同）
            if (sig.up_move_settled and sig.exit_persistence_high
                    and sig.exit_div_ok):
                _close(i, c, "l1_op_top_divergence")
            else:
                # 模块6/7：次级别买卖点驱动三阶段降成本（每 bar 至多一个事件，状态守卫）
                f = fsm
                has_open_diff = (
                    f.active_short_diff is not None and f.active_short_diff.is_open
                )
                if (sig.sub_sell_signal and not has_open_diff
                        and f.state in _COST_OPEN_STATES):
                    fsm = transition(
                        f, FsmEvent(
                            FsmEventType.SUB_LEVEL_SELL_POINT, price=c, level="L0"),
                    )
                elif (sig.sub_buy_signal and has_open_diff
                        and f.state in _COST_ACTIVE_DIFF_STATES):
                    fsm = transition(
                        f, FsmEvent(
                            FsmEventType.SUB_LEVEL_BUY_POINT, price=c, level="L0"),
                    )

                # 2 买追踪（与 G 同，结构化判定，引擎不发 type2）
                if sig.down_move_settled:
                    in_pullback = True
                    pullback_low = c
                if in_pullback:
                    pullback_low = min(pullback_low, c)
                    if pullback_low < entry_1buy_low:
                        in_pullback = False
                    elif sig.type2_buy or bool(sig.buy_cands) or sig.l0_r1_down:
                        n_addon_signals += 1
                        in_pullback = False

    if state == _LONG and fsm is not None:
        _close(n - 1, signals[-1].close, "eod_close")

    return trades, {"addon_signals": n_addon_signals, "earn_trades": n_earn_trades}


# ════════════════════════════════════════════════════════════
# I：最佳组合版（E 的出场 + G/H 的进场和持仓）
# ════════════════════════════════════════════════════════════

def run_complete_i(
    signals: list["BarSignalV2"],
) -> tuple[list[CompletedTrade], dict[str, int]]:
    """最佳组合版 I —— H 的进场与持仓（区间套 + 中枢趋势门 + 三阶段 FSM 降成本 +
    次级别背驰触发 + 2 买标记）+ **E 的出场**（L2 趋势翻转顶背驰）。

    动机（E/G 诊断）：G/H 把出场从 E 的 L2 趋势翻转（`l2_flip_short ∧ exit_div_ok`，
    操作级"同级别第一类卖点"）降级为 L1 操作级 persistence 退出（`up_move_settled ∧
    exit_persistence_high ∧ exit_div_ok`，move 级）。L1+persistence 出场在强趋势标的
    （OKLO）上被 move 级回调震出 → G 跑输 E（OKLO +267% vs E +455%）。I 把出场升回
    L2 趋势级（持仓穿越 move 波动，只在操作级趋势真正见顶离场），同时保留 H 的完整
    进场与持仓机制。

    与 H 的**唯一差异**是出场判定（其余进场 ARM 两阶段、FSM 降成本、2 买追踪逐行相同）：
      H 出场：up-move settle ∧ exit_persistence_high ∧ exit_div_ok（L1 操作级 + persistence）。
      I 出场：l2_flip_short ∧ exit_div_ok（L2 趋势翻转 + 顶背驰，= E 的鲁棒出场）。

    persistence 角色（诚实标注）：H 中 `exit_persistence_high` 唯一消费点是出场判定。I 用
    E 的 L2 出场后该字段不再被出场消费——非"丢弃功能"，而是 L2 趋势翻转在更高级别上取代了
    persistence 的"鲁棒出场（不被噪声震出）"作用：两者是同一鲁棒性需求的不同层级实现，
    在 L2 出场上叠加 persistence 是冗余（L2 翻转已过滤 move 级噪声），且会改变 E 出场语义。
    故 I 严格用 E 的两条件出场，不叠加 persistence。

    进场（与 H 同，区间套两阶段，模块1/2/5）：down-move settle ∧ 底背驰 ∧ zs_count≥2
      → ARM；ARMED 中 l0_r1_down ∨ type1 买点 → 精确入场，超时退化操作级入场。
    持仓降成本（与 H 同，模块6/7）：真实 CostReductionFSM 三阶段（满仓满融，own_capital=本金，
      margin=0），次级别顶背驰 → SUB_LEVEL_SELL_POINT（trim）、次级别底背驰/买点 →
      SUB_LEVEL_BUY_POINT（回补），cost_basis≤0 后进 EARNING_SHARES（挣股数）。
    """
    n = len(signals)
    state = _FLAT
    entry_bar = -1
    entry_price = 0.0
    entry_1buy_low = float("inf")
    arm_bar = -1
    arm_dip_low = float("inf")
    in_pullback = False
    pullback_low = float("inf")
    fsm: CostReductionFSM | None = None
    n_addon_signals = 0
    n_earn_trades = 0

    trades: list[CompletedTrade] = []

    def _open_fsm(price: float) -> CostReductionFSM:
        f0 = CostReductionFSM.create(
            own_capital=INITIAL_CAPITAL, margin_amount=0.0, sub_ratio=0.3,
        )
        return transition(
            f0, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, price=price, level="L1"),
        )

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, fsm, entry_bar, entry_price, entry_1buy_low
        nonlocal in_pullback, pullback_low, n_earn_trades
        if fsm is None or entry_price <= 0:
            state = _FLAT
            fsm = None
            return
        f = fsm
        # 出场前回补未闭合短差（在出场价），避免 EARNING_SHARES 卖出未回补的现金漏计
        if (f.active_short_diff is not None and f.active_short_diff.is_open
                and f.state in _COST_ACTIVE_DIFF_STATES):
            f = transition(
                f, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, price=price, level="L0"),
            )
        snap = f.snapshot()
        if snap.state == CostState.EARNING_SHARES or f.state == CostState.EARNING_SHARES:
            n_earn_trades += 1
        pnl_pct = (
            snap.cumulative_recovered + snap.total_shares * price - INITIAL_CAPITAL
        ) / INITIAL_CAPITAL * 100
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price,
            exit_bar=bar_idx, exit_price=price,
            pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=len(f.completed_short_diffs),
            cost_basis_at_exit=snap.cost_basis,
        ))
        state = _FLAT
        fsm = None
        entry_bar = -1
        entry_price = 0.0
        entry_1buy_low = float("inf")
        in_pullback = False
        pullback_low = float("inf")

    for i in range(n):
        sig = signals[i]
        c = sig.close

        if state == _FLAT:
            if (sig.down_move_settled and sig.entry_div_ok
                    and sig.entry_zs_count >= 2):
                state = _ARMED
                arm_bar = i
                arm_dip_low = c

        elif state == _ARMED:
            arm_dip_low = min(arm_dip_low, c)
            do_enter = sig.l0_r1_down or bool(sig.buy_cands)
            if not do_enter and (i - arm_bar) > PULLBACK_EXPIRY:
                do_enter = True
            if do_enter:
                state = _LONG
                entry_price = c
                entry_bar = i
                entry_1buy_low = arm_dip_low
                fsm = _open_fsm(c)
                in_pullback = False
                pullback_low = float("inf")
            elif sig.l2_flip_short:
                state = _FLAT

        elif state == _LONG:
            assert fsm is not None
            # 出场判定（E 的 L2 趋势翻转顶背驰 — I 与 H 的唯一差异）
            if sig.l2_flip_short and sig.exit_div_ok:
                _close(i, c, "l2_trend_top_divergence")
            else:
                # 模块6/7：次级别买卖点驱动三阶段降成本（每 bar 至多一个事件，状态守卫）
                f = fsm
                has_open_diff = (
                    f.active_short_diff is not None and f.active_short_diff.is_open
                )
                if (sig.sub_sell_signal and not has_open_diff
                        and f.state in _COST_OPEN_STATES):
                    fsm = transition(
                        f, FsmEvent(
                            FsmEventType.SUB_LEVEL_SELL_POINT, price=c, level="L0"),
                    )
                elif (sig.sub_buy_signal and has_open_diff
                        and f.state in _COST_ACTIVE_DIFF_STATES):
                    fsm = transition(
                        f, FsmEvent(
                            FsmEventType.SUB_LEVEL_BUY_POINT, price=c, level="L0"),
                    )

                # 2 买追踪（与 G/H 同，结构化判定，引擎不发 type2）
                if sig.down_move_settled:
                    in_pullback = True
                    pullback_low = c
                if in_pullback:
                    pullback_low = min(pullback_low, c)
                    if pullback_low < entry_1buy_low:
                        in_pullback = False
                    elif sig.type2_buy or bool(sig.buy_cands) or sig.l0_r1_down:
                        n_addon_signals += 1
                        in_pullback = False

    if state == _LONG and fsm is not None:
        _close(n - 1, signals[-1].close, "eod_close")

    return trades, {"addon_signals": n_addon_signals, "earn_trades": n_earn_trades}


# ════════════════════════════════════════════════════════════
# 单标的管线（compute-once：信号层一次 → 6 模式重放磁带）
# ════════════════════════════════════════════════════════════

def _process_symbol(symbol: str) -> tuple[str, dict]:
    print(f"\n{'=' * 60}\n  {symbol} — 完整版回测（A/E/F/G/H，compute-once）\n{'=' * 60}")
    opens, highs, lows, closes = load_1min(symbol)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    t_sig = time.time()
    signals = compute_signals_v2(opens, highs, lows, closes)
    sig_elapsed = time.time() - t_sig
    print(f"  [{symbol}] {n:,} bars  BH={bh:+.2f}%  信号层 {sig_elapsed:6.1f}s（仅一次）")

    # ── tape-equality 复现证明（V2 磁带 ≡ 原始 compute_signals，逐字段）──
    # 严格复现锚点：不依赖可能陈旧的已发布数字，直接证明 V2 共享字段 == 原始引擎输出。
    vn = min(VALIDATE_N, n)
    orig_sig = _ef.compute_signals(opens[:vn], highs[:vn], lows[:vn], closes[:vn])
    v2_sig = signals[:vn]
    field_mismatch = 0
    for a, b in zip(orig_sig, v2_sig):
        for fld in _SHARED_FIELDS:
            if getattr(a, fld) != getattr(b, fld):
                field_mismatch += 1
                break
    tape_faithful = field_mismatch == 0
    print(
        f"  [{symbol}] tape-equality 校验前 {vn:,} bars × {len(_SHARED_FIELDS)} 字段："
        f"{'✅ 逐字段一致' if tape_faithful else f'⚠ {field_mismatch} bar 不一致'}"
    )

    out: dict = {
        "n_bars": n, "bh": bh, "first": closes[0], "last": closes[-1],
        "tape_faithful": tape_faithful, "tape_validate_n": vn,
        "tape_mismatch": field_mismatch,
    }

    # A/E/F：复用 E/F 脚本（鸭子类型消费 BarSignalV2 超集磁带）
    t0 = time.time()
    a_trades, _, _ = run_trading(signals, MODE_NONE, divergence_gate=False)
    e_trades, _ = run_swing_trading(signals, MODE_NONE)
    f_trades, _ = run_swing_trading(signals, MODE_REFINED)
    # G/H/I：本脚本完整版
    g_trades, g_extra = run_complete(signals)
    h_trades, h_extra = run_complete_h(signals)
    i_trades, i_extra = run_complete_i(signals)
    replay_elapsed = time.time() - t0

    out[MODE_A] = {"metrics": compute_metrics(a_trades), "extra": {}}
    out[MODE_E] = {"metrics": compute_metrics(e_trades), "extra": {}}
    out[MODE_F] = {"metrics": compute_metrics(f_trades), "extra": {}}
    out[MODE_G] = {"metrics": compute_metrics(g_trades), "extra": g_extra}
    out[MODE_H] = {"metrics": compute_metrics(h_trades), "extra": h_extra}
    out[MODE_I] = {"metrics": compute_metrics(i_trades), "extra": i_extra}

    print(f"  [{symbol}] 6 模式重放 {replay_elapsed:.2f}s")
    for key in (MODE_A, MODE_E, MODE_F, MODE_G, MODE_H, MODE_I):
        m = out[key]["metrics"]
        print(
            f"  [{symbol}/{key:16s}] 交易={m['n']:3d} 胜率={m['win_rate']:4.0f}% "
            f"复利={m['total_compound']:+9.2f}% 超额={m['total_compound'] - bh:+9.2f}% "
            f"夏普={m['sharpe']:+.3f} 降成本笔={m['n_with_cr']} {out[key]['extra']}"
        )
    return symbol, out


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

_MODE_ORDER = [MODE_A, MODE_E, MODE_F, MODE_G, MODE_H, MODE_I]
_MODE_LABEL = {
    MODE_A: "A（38课FSM纯进出场）",
    MODE_E: "E（背驰定位器简化版）",
    MODE_F: "F（E+简化降成本）",
    MODE_G: "G（完整版：区间套+中枢趋势门+persistence退出+2买标记）",
    MODE_H: "H（完整版+三阶段FSM降成本+挣股数）",
    MODE_I: "I（最佳组合：E的L2趋势翻转出场+H进场/持仓）",
}

# 已发布 A/E 数字（fugue_alpha_diagnosis.md）→ 外部锚点（进出场磁带忠实性）。
# 注：.md 的 F（QQQ 50.28 / OKLO 495.63）已陈旧——其后 fugue_alpha_diagnosis.py 被修改，
# 当前脚本的 F 与本脚本一致（tape-equality 已证 V2≡原始引擎）。故 F 不作 .md 锚点，
# 改由 tape-equality 逐字段证明覆盖。
_PUBLISHED = {
    "QQQ": {MODE_A: 40.59, MODE_E: 71.16},
    "OKLO": {MODE_A: 242.61, MODE_E: 455.81},
}


def write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 完整版缠论+PH多重赋格操盘系统 — G/H 回测\n")
    L.append(
        "> 标的：QQQ / OKLO（1min）  |  脚本：`analysis/fugue_complete_v2.py`  |  "
        "认识论等级：**L2**（真实数据，含否定性结果）\n"
    )
    L.append(
        "> 521 号限定：信号属 candidate 层（PH 门控 + MACD 面积力度代理），"
        "非 confirmed 买卖点 alpha 证据。E/F 简化实验见 `fugue_alpha_diagnosis.md`。\n"
    )

    # ── 复现校验（两层：tape-equality 逐字段 + 已发布外部锚点）──
    L.append("## 复现校验（磁带超集忠实性）\n")
    tape_ok = all(results[s].get("tape_faithful") for s in results)
    L.append(
        "**第一层 tape-equality（严格证明）**：BarSignalV2 是 `BarSignal` 的严格超集。"
        "本脚本在每标的前 30,000 bars 上**逐字段**对比 `compute_signals_v2` 与原始 "
        "`fugue_alpha_diagnosis.compute_signals` 的 18 个共享字段——不依赖任何已发布数字，"
        "直接证明 V2 磁带 ≡ 原始引擎输出。\n"
    )
    L.append("| 标的 | 校验 bars | 共享字段 | 不一致 bar | 结论 |")
    L.append("|------|----------|---------|-----------|------|")
    for sym in results:
        r = results[sym]
        L.append(
            f"| {sym} | {r.get('tape_validate_n', 0):,} | {len(_SHARED_FIELDS)} | "
            f"{r.get('tape_mismatch', -1)} | "
            f"{'✅ 逐字段一致' if r.get('tape_faithful') else '⚠ 不一致'} |"
        )
    L.append("")
    L.append(
        f"**tape-equality 结论**：{'✅ V2 磁带逐字段等于原始引擎输出' if tape_ok else '⚠ 存在不一致'}"
        f"——A/E/F 由复用的 `run_trading`/`run_swing_trading` 在此磁带上重放，故"
        f"{'与原始脚本数字同源，G/H 与 A/E/F 直接可比。' if tape_ok else '对比受影响。'}\n"
    )
    L.append(
        "**第二层 已发布外部锚点（A/E）**：A/E 与 `fugue_alpha_diagnosis.md` 已发布数字对比。\n"
    )
    L.append("| 标的 | 模式 | 本脚本% | 已发布% | 偏差 |")
    L.append("|------|------|--------|---------|------|")
    anchor_ok = True
    for sym in results:
        for key in (MODE_A, MODE_E):
            got = results[sym][key]["metrics"]["total_compound"]
            pub = _PUBLISHED.get(sym, {}).get(key)
            if pub is None:
                continue
            dev = got - pub
            if abs(dev) > 0.01:
                anchor_ok = False
            L.append(f"| {sym} | {key} | {got:+.2f} | {pub:+.2f} | {dev:+.4f} |")
    L.append("")
    L.append(
        f"**外部锚点结论**：{'✅ A/E 与已发布逐位一致（进出场磁带忠实）' if anchor_ok else '⚠ A/E 偏差'}。"
        "注：`.md` 的 F（QQQ 50.28 / OKLO 495.63）**已陈旧**——其后 `fugue_alpha_diagnosis.py` "
        "被修改，当前脚本 F 与本脚本一致（已由 tape-equality 逐字段证明覆盖），"
        "故 F 不以陈旧 `.md` 为锚（formalization-validity-domain：陈旧锚点会产生假阳性漂移）。\n"
    )

    # ── 主对照表 ──
    for sym in results:
        r = results[sym]
        bh = r["bh"]
        L.append(f"## {sym}\n")
        L.append(
            f"- 数据：**{r['n_bars']:,}** bars  |  价格：{r['first']:.2f} → {r['last']:.2f}"
            f"  |  Buy-and-hold：**{bh:+.2f}%**\n"
        )
        L.append("| 版本 | 复利% | BH% | 超额% | 交易数 | 胜率 | 夏普 | 降成本笔 |")
        L.append("|------|-------|-----|--------|--------|------|------|---------|")
        for key in _MODE_ORDER:
            m = r[key]["metrics"]
            excess = m["total_compound"] - bh
            L.append(
                f"| {_MODE_LABEL[key]} | {m['total_compound']:+.2f} | {bh:+.2f} | "
                f"{excess:+.2f} | {m['n']} | {m['win_rate']:.0f}% | {m['sharpe']:+.3f}"
                f" | {m['n_with_cr']} |"
            )
        L.append("")
        g_extra = r[MODE_G]["extra"]
        h_extra = r[MODE_H]["extra"]
        L.append(
            f"- 2买加仓机会（G/H 标记，加仓已移除）：G {g_extra.get('addon_signals', 0)} 次"
            f"，H {h_extra.get('addon_signals', 0)} 次。"
        )
        L.append(
            f"- H 进入挣股数阶段（cost_basis≤0）的交易：{h_extra.get('earn_trades', 0)} 笔。\n"
        )

    # ── 跨标的判定 ──
    L.append("## 完整版判定（I/G/H vs E/F vs BH）\n")
    for sym in results:
        r = results[sym]
        bh = r["bh"]
        e = r[MODE_E]["metrics"]["total_compound"]
        g = r[MODE_G]["metrics"]["total_compound"]
        f = r[MODE_F]["metrics"]["total_compound"]
        hh = r[MODE_H]["metrics"]["total_compound"]
        ii = r[MODE_I]["metrics"]["total_compound"]
        L.append(f"### {sym}\n")
        L.append(
            f"- G vs E（完整化增量）：{g:+.2f}% vs {e:+.2f}% → "
            + ("**完整化提升**" if g > e else "**完整化未提升**")
            + f"（差 {g - e:+.2f}%）"
        )
        L.append(
            f"- G vs BH：{g:+.2f}% vs {bh:+.2f}% → "
            + ("**翻正超额**" if g > bh else "**跑输 BH**")
        )
        L.append(
            f"- H vs G（三阶段降成本+挣股数净贡献）：{hh:+.2f}% vs {g:+.2f}% → "
            + ("**降成本增益**" if hh > g else "**降成本拖累**")
            + f"（差 {hh - g:+.2f}%）"
        )
        L.append(
            f"- H vs F（完整 FSM 降成本 vs 简化降成本）：{hh:+.2f}% vs {f:+.2f}%"
            f"（差 {hh - f:+.2f}%）"
        )
        # ── I 三版本对比（E 出场 + G/H 进场持仓 = 最佳组合假设）──
        L.append(
            f"- **I vs G（出场升级 E 的 L2 趋势翻转）**：{ii:+.2f}% vs {g:+.2f}% → "
            + ("**出场升级提升**" if ii > g else "**出场升级未提升**")
            + f"（差 {ii - g:+.2f}%）"
        )
        L.append(
            f"- **I vs E（加 G/H 完整进场持仓）**：{ii:+.2f}% vs {e:+.2f}% → "
            + ("**完整化提升**" if ii > e else "**完整化未提升**")
            + f"（差 {ii - e:+.2f}%）"
        )
        L.append(
            f"- **I vs H（同进场持仓，仅出场 E vs L1+persistence）**：{ii:+.2f}% vs {hh:+.2f}%"
            f"（差 {ii - hh:+.2f}%）"
        )
        best = max(
            (("E", e), ("G", g), ("H", hh), ("I", ii)), key=lambda kv: kv[1]
        )
        L.append(
            f"- **{sym} 四版本最优**：{best[0]}（{best[1]:+.2f}%）"
            + ("，I 是最优组合 ✅" if best[0] == "I" else f"，I={ii:+.2f}% 非最优")
            + (f"；I vs BH：{'翻正超额 ✅' if ii > bh else '跑输 BH'}" )
            + "\n"
        )

    g_beats_bh = [results[s][MODE_G]["metrics"]["total_compound"] > results[s]["bh"]
                  for s in results]
    g_beats_e = [results[s][MODE_G]["metrics"]["total_compound"]
                 > results[s][MODE_E]["metrics"]["total_compound"] for s in results]
    hg = [results[s][MODE_H]["metrics"]["total_compound"]
          - results[s][MODE_G]["metrics"]["total_compound"] for s in results]
    v_bh = "全部标的" if all(g_beats_bh) else ("部分标的" if any(g_beats_bh) else "无标的")
    v_e = "全部标的" if all(g_beats_e) else ("部分标的" if any(g_beats_e) else "无标的")
    L.append(
        f"**跨标的**：G 在 **{v_e}** 上相对 E 提升（完整化有增量）；"
        f"G 在 **{v_bh}** 上翻正超额（G>BH）。"
        f"H−G 净贡献：" + "、".join(f"{s} {d:+.2f}%" for s, d in zip(results, hg)) + "。\n"
    )

    # ── 七模块完成度 ──
    L.append("## 补全的 7 模块（设计文档 → 代码映射）\n")
    L.append("| 模块 | 设计要求 | 实现 |")
    L.append("|------|---------|------|")
    L.append("| 1. 4层级别架构 | L2趋势/L1操作/L0次操作 | L2=move-level PH(l2_flip)；"
             "L1=move settle+背驰(进出场)；L0=segment PH+背驰(降成本)。L2 为段切换语境非1买门 |")
    L.append("| 2. 区间套精确入场 | L1确认→L0定位 | ARMED 两阶段：L1趋势底背驰 ARM → "
             "L0(l0_r1_down/type1) 定位精确价；递归展开非独立过滤 |")
    L.append("| 3. 2买 | 回调不破1买低点 | **结构化判定**(引擎不发type2,只type1/type3)："
             "回调段低点≥1买低点 ∧ 买点候选(回调底确认) → 标记(加仓已移除,267号D2无加仓FSM,只计数) |")
    L.append("| 4. Persistence过滤 | 高persistence触发退出 | up-move settle persistence≥近期"
             "中位数 → 退出评估；低persistence settle 不退出(穿越噪声,H中触发降成本) |")
    L.append("| 5. 中枢判定 | 1买要求趋势下跌(≥2中枢) | down-move zs_count≥2 门控入场；"
             "**经验观察(L2)**：本数据全部 settle 的 down-move 均为趋势(zs_count≥2)，"
             "盘整(zs_count==1)不作方向性settle出现 → 门语义正确但此数据上非限制性(诚实标注,非声明膨胀) |")
    L.append("| 6. 三阶段降成本FSM | 接入cost_reduction_fsm | H 用真实 CostReductionFSM："
             "POSITION_OPEN→COST_REDUCING→PRINCIPAL_WITHDRAWN/EARNING_SHARES |")
    L.append("| 7. 次级别买卖点触发降成本 | 顶背驰卖/底背驰买 | sub_sell_signal(segment顶背驰)"
             "→SUB_LEVEL_SELL_POINT；sub_buy_signal(segment底背驰/买点)→SUB_LEVEL_BUY_POINT |")
    L.append("")

    # ── 结果包六要素 ──
    L.append("## 结果包（六要素）\n")
    L.append(
        "**1. 结论**：见主对照表（BH/A/E/F/G/H）。G=完整版进出场（区间套+中枢趋势门+"
        "persistence退出+2买标记），H=G+三阶段FSM降成本+挣股数。\n"
    )
    L.append(
        "**2. 定义依据**：38课操盘程式（底背驰买/顶背驰卖/没背驰不动）；37课背驰（相邻同向"
        "走势力度衰减）；29课区间套（大级别往小级别看，敏感进场+鲁棒出场）；中枢趋势定义"
        "（`a_move_v1.py`：趋势=2+同向中枢，盘整=1中枢）；267号满仓满融降成本（`cost_reduction_fsm.py`）；"
        "缠师31/43课挣股数（cost_basis≤0后金额守恒先卖后买）。\n"
    )
    L.append(
        "**3. 边界条件**：(i) 入场要求 down-move zs_count≥2 — 趋势背驰买点在1min上稀少，"
        "若引擎中枢判定阈值变化则入场频率翻转；(ii) persistence 高/低以近期中位数(窗口50)分类，"
        "窗口变化影响退出频率；(iii) sub_ratio=0.3、trim阈值0.05 隐含；(iv) 力度用MACD面积代理"
        "(521号PH纯拓扑无动量)，换真实走势区间力度可能翻转；(v) G/H>BH 仍须随机门控对照排除"
        "趋近buy-hold同义反复(project_backtest_benchmark_falsifiability)。\n"
    )
    L.append(
        "**4. 下游推论**：若 G≤BH → 完整化的进出场信号层仍无确立 alpha，须先修信号层再谈降成本；"
        "若 G>E → 区间套+中枢趋势门+persistence退出确有完整化增量；若 H>G → 三阶段FSM降成本"
        "在完整框架下增益(可上推confirmed层)，若 H<G → 降成本仍是负alpha源(project_ph_settle_usage_boundary)。\n"
    )
    L.append(
        "**5. 谱系引用**：521号(PH须MACD闸)；267号(满仓满融降成本+D2无加仓)；268a(own_capital独立核算)；"
        "project_divergence_locator_entry_exit(E/F背驰定位器,G/H的前身)；"
        "project_divergence_gate_entry_exit_falsified(D门控被证伪,故L2不作1买门)；"
        "project_costreduction_moneyprinter_bug(截断bug已去)；"
        "project_ph_settle_usage_boundary(PH settle是candidate必要非充分)；"
        "project_backtest_benchmark_falsifiability(BH基准可证伪性)。\n"
    )
    L.append(
        "**6. 影响声明**：脚本 `analysis/fugue_complete_v2.py`(本文件)——compute_signals_v2"
        "(BarSignalV2超集磁带)+run_complete(G)+run_complete_h(H,接入cost_reduction_fsm.py)"
        "+run_complete_i(I,E出场+H进场持仓)。I 与 H 唯一差异是出场判定行(L2趋势翻转替换"
        "L1+persistence)，进场/FSM降成本/2买逐行复用 H 逻辑。"
        "**不修改引擎代码、不修改 cost_reduction_fsm.py、不修改 fugue_alpha_diagnosis.py**"
        "(A/E/F 经复现校验逐位复用)。新增报告 `fugue_complete_v2_results.md`。\n"
    )
    L.append(
        "**认识论等级**：L2（真实数据 QQQ/OKLO 1min；G 若跑输 BH 的否定性结果缩小有效域边界，"
        "比确认性结果更有价值——缺 L3 多标的交叉验证与随机门控对照）。"
    )

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


def main() -> None:
    symbols = ["QQQ", "OKLO"]
    results: dict = {}
    parallel = os.environ.get("BT_PARALLEL", "1") != "0" and len(symbols) > 1

    t_all = time.time()
    if parallel:
        workers = min(len(symbols), os.cpu_count() or 2)
        print(f"跨标的并行：{workers} 进程 ×（信号层 once + 5 模式重放磁带）")
        with ProcessPoolExecutor(max_workers=workers) as ex:
            for symbol, out in ex.map(_process_symbol, symbols):
                results[symbol] = out
    else:
        for symbol in symbols:
            _, out = _process_symbol(symbol)
            results[symbol] = out
    print(f"\n总耗时 {time.time() - t_all:.1f}s（5 模式 × {len(symbols)} 标的）")

    write_report(results)

    audit = {
        sym: {
            "n_bars": results[sym]["n_bars"],
            "bh": results[sym]["bh"],
            **{
                key: {
                    "compound": results[sym][key]["metrics"]["total_compound"],
                    "excess": results[sym][key]["metrics"]["total_compound"] - results[sym]["bh"],
                    "n": results[sym][key]["metrics"]["n"],
                    "win_rate": results[sym][key]["metrics"]["win_rate"],
                    "sharpe": results[sym][key]["metrics"]["sharpe"],
                    "n_with_cr": results[sym][key]["metrics"]["n_with_cr"],
                    "extra": results[sym][key]["extra"],
                }
                for key in _MODE_ORDER
            },
        }
        for sym in symbols
    }
    (DATA_DIR / "fugue_complete_v2_results.json").write_text(
        json.dumps(audit, indent=2, ensure_ascii=False)
    )
    print("审计 JSON 已写入：fugue_complete_v2_results.json")


if __name__ == "__main__":
    main()
