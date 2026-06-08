"""Rust 引擎驱动的 E 版本信号层（替代 Python compute_signals 的 O(N²) 缠论递归）。

## 为何需要

`fugue_alpha_diagnosis.compute_signals` 用 Python `RecursiveOrchestrator` 逐 bar 驱动，
线段层全量重算 → O(N²)（实测 400k bar 211s，5M 外推 ~9 小时/标的，不可行）。
`newchan_rust.RecursiveOrchestrator` 逐位等价但快 ~23×（400k 9s）——仍 O(N²)（同款线段
全量重算），但 5M 外推 ~40-50min/标的，可行。

## 严格等价（零语义偏差）

E 版本 `run_swing_trading(MODE_NONE)` 只读 BarSignal 的 5 个字段：
  close / down_move_settled / up_move_settled(F才用) / entry_div_ok / l2_flip_short / exit_div_ok。
这些只依赖 **L1 走势 settle 事件 + L2 PH + MACD 面积 + 相邻走势背驰**——
不依赖 BSP / L0·L1 PH / segment settle / new_up_moves（故 compute_signals 里那些 O(N²)
路径——L0 PH 每 bar sorted、l1_up_segs 切片 max——全部省去）。

本模块用 Rust orchestrator 跑 process_bar，每 bar 取 `current_moves()` 构造**真实**
`a_move_v1.Move` 对象，喂 Python orchestrator 内部所用的**同一个** `diff_moves` 函数
重建 `MoveSettleV1` 事件，再用 `compute_signals` 内部所用的**同一套** L2 `PHLevelState`
/ `OnlineMacdState` / `MoveRecord` / `check_divergence` 算 4 字段。

→ 唯一被替换的是引擎（Python orchestrator → Rust orchestrator，记忆 bit-exact）；
事件 diff / PH / MACD / 背驰判据全部逐字复用 `fugue_alpha_diagnosis` 与 `move_state`，
故产出 BarSignal 的 E 相关字段逐位等价（`verify_equivalence` 在 100k bar 上证明）。

认识论等级：移植正确性 L0/L1（管线等价，由 bit-exact 验证保证）；
回测结论 L2（真实数据，由 run_swing_trading 在真实 OHLC 上产出）。
"""

from __future__ import annotations

import sys
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust  # noqa: E402

import fugue_alpha_diagnosis as F  # noqa: E402
from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.a_move_v1 import Move  # noqa: E402
from newchan.core.recursion.move_state import diff_moves  # noqa: E402
from newchan.events import MoveSettleV1  # noqa: E402

# compute_signals 用 max_levels=2；其余引擎参数取 Python 默认（= Rust 默认）：
#   stroke_mode="wide", min_strict_sep=5, reset_dir_on_fractal=False,
#   new_raw_gap_min=3, enable_macd_divergence=False。
_MAX_LEVELS = 2

_BASE_TS = datetime(2020, 1, 1)


def _move_from_tuple(t) -> Move:
    """Rust MoveTuple → 真实 a_move_v1.Move（字段逐一对应，零损失）。

    MoveTuple = ((kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count,
                  settled), (high, low, first_seg_s0, last_seg_s1, zg_max, zd_min,
                  persistence))。
    """
    head, tail = t
    return Move(
        kind=head[0],
        direction=head[1],
        seg_start=head[2],
        seg_end=head[3],
        zs_start=head[4],
        zs_end=head[5],
        zs_count=head[6],
        settled=head[7],
        high=tail[0],
        low=tail[1],
        first_seg_s0=tail[2],
        last_seg_s1=tail[3],
        zg_max=tail[4],
        zd_min=tail[5],
        persistence=tail[6],
    )


def _e_signal(
    close: float,
    down_move_settled: bool,
    up_move_settled: bool,
    down_move_hist: list[F.MoveRecord],
    up_move_hist: list[F.MoveRecord],
    l2_flip_short: bool = False,
    l2_flip_long: bool = False,
) -> F.BarSignal:
    """构造 BarSignal——E 相关字段精确，E 不消费的字段填默认。

    entry_div_ok/exit_div_ok 每 bar 照常算（基于当前 down/up_move_hist 末两条），
    与 compute_signals 逐字一致（即使本 bar 无新 settle，值也照常输出）。
    """
    entry_div_ok = (
        F.check_divergence(down_move_hist[-1], down_move_hist[-2], "down")
        if len(down_move_hist) >= 2 else False
    )
    exit_div_ok = (
        F.check_divergence(up_move_hist[-1], up_move_hist[-2], "up")
        if len(up_move_hist) >= 2 else False
    )
    return F.BarSignal(
        close=close,
        l0_r1_down=False, l0_r1_up=False,
        l1_nr1_up=False, l1_nr1_down=False,
        l2_flip_long=l2_flip_long, l2_flip_short=l2_flip_short,
        l2_direction=0,
        buy_cands=(), sell_cands=(), buy_invalidates=(),
        new_up_moves=(),
        med_persistence=0.0,
        l1_up_ratio=0.0, refined_gate_ok=False,
        entry_div_ok=entry_div_ok,
        exit_div_ok=exit_div_ok,
        down_move_settled=down_move_settled,
        up_move_settled=up_move_settled,
    )


def compute_e_signals_rust(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> list[F.BarSignal]:
    """Rust 引擎驱动，产出 BarSignal 磁带（E 相关字段精确，其余填默认）。

    与 `compute_signals` 的差异**仅**是引擎来源（Rust 替 Python orchestrator）+
    省去 E 不消费的字段（BSP / L0·L1 PH / new_up_moves → 默认值）。L1 走势 settle
    事件 / L2 PH / MACD 面积 / 背驰记录全部逐字复用，故 E 字段逐位等价。
    """
    n = len(closes)
    orch = newchan_rust.RecursiveOrchestrator(max_levels=_MAX_LEVELS)

    # L2 PH（L1 走势 settle 时喂端点）——复用 compute_signals 同款 PHLevelState。
    l2_down = F.PHLevelState.make()
    l2_up = F.PHLevelState.make()

    macd = OnlineMacdState()
    pos_cum = 0.0
    neg_cum = 0.0
    pos_cum_hist: list[float] = []
    neg_cum_hist: list[float] = []

    down_move_hist: list[F.MoveRecord] = []
    up_move_hist: list[F.MoveRecord] = []

    prev_moves: list[Move] = []
    # tuple 短路：L1 走势极稀疏（100k bar 仅 ~38 moves），99.9% 的 bar moves 不变。
    # 仅在 current_moves() 元组列表变化时才构造 Move 对象 + 跑 diff_moves（moves
    # 不变 ⟹ diff 公共前缀=全部 ⟹ 无事件，短路语义等价）。去掉每 bar 全量 Move
    # 构造（O(N×n_moves) frozen dataclass）这一主开销。
    prev_tuples: tuple | None = None

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

    signals: list[F.BarSignal] = []
    last_progress = 0

    for i in range(n):
        c = closes[i]
        ts = _BASE_TS + timedelta(minutes=i)

        # MACD 增量（全程连续，正/负柱累积和）——与 compute_signals 逐字一致。
        _, _, hist = macd.update(c, ts)
        pos_cum += hist if hist > 0 else 0.0
        pos_cum_hist.append(pos_cum)
        neg_cum += -hist if hist < 0 else 0.0
        neg_cum_hist.append(neg_cum)

        # Rust 引擎逐 bar（O(N²) 但快 23×，唯一被替换的部件）。
        orch.process_bar(opens[i], highs[i], lows[i], c)
        curr_tuples = tuple(orch.current_moves())

        # tuple 短路：moves 未变 ⟹ 无 settle 事件，跳过 Move 构造 + diff。
        if curr_tuples == prev_tuples:
            signals.append(_e_signal(
                c, False, False,
                down_move_hist, up_move_hist,
            ))
            if i - last_progress >= 500_000:
                print(f"    [{i / n * 100:5.1f}%] rust signal bar {i:,}/{n:,}",
                      flush=True)
                last_progress = i
            continue
        prev_tuples = curr_tuples

        curr_moves = [_move_from_tuple(t) for t in curr_tuples]
        # 用 Python orchestrator 内部所用的同一 diff_moves 重建 MoveSettleV1 事件。
        events = diff_moves(prev_moves, curr_moves, bar_idx=i, bar_ts=0.0)
        prev_moves = curr_moves

        down_move_settled = False
        up_move_settled = False
        l2_flip_short = False
        l2_flip_long = False

        for e in events:
            if not isinstance(e, MoveSettleV1):
                continue
            mv = None
            for m in curr_moves:
                if (m.seg_start == e.seg_start
                        and m.direction == e.direction
                        and m.settled):
                    mv = m
                    break
            if mv is None:
                continue
            # 走势级背驰记录（与 compute_signals 逐字一致）。
            if mv.direction == "down":
                down_move_hist.append(F.MoveRecord(
                    direction="down", high=mv.high, low=mv.low,
                    macd_area=_macd_neg_area(mv.first_seg_s0, mv.last_seg_s1),
                ))
                down_move_settled = True
            else:
                up_move_hist.append(F.MoveRecord(
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

        signals.append(_e_signal(
            c, down_move_settled, up_move_settled,
            down_move_hist, up_move_hist,
            l2_flip_short=l2_flip_short, l2_flip_long=l2_flip_long,
        ))

        if i - last_progress >= 500_000:
            print(f"    [{i / n * 100:5.1f}%] rust signal bar {i:,}/{n:,}", flush=True)
            last_progress = i

    return signals
