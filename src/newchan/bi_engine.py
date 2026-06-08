"""笔事件引擎 — 逐 bar 驱动的增量引擎

核心流程（每 bar 均摊 O(1)，总计 O(N)）：
1. 增量包含处理：仅处理新 bar，维护 merge_buf 状态
2. 增量分型检测：仅检查 merged 序列尾部 3 根 bar
3. 增量笔构造：two-pointer 检查点 + 尾部重算
4. 差分前后 Stroke 快照 → 产生域事件

约束：
- 每次计算输入严格为 bars[:bar_idx+1]，保证无未来函数
- 增量结果与全量纯函数重跑等价（由 257 个测试保证）
- 对外暴露事件流，对内保留 Stroke.confirmed 语义
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

import numpy as np

from newchan.a_fractal import Fractal, _classify_fractal
from newchan.a_inclusion import _is_fractal_pattern
from newchan.a_stroke import (
    Stroke,
    _build_stroke,
    _check_gap,
    _extend_prev_stroke,
    _is_more_extreme,
    _mark_last_unconfirmed,
    _validate_direction,
)
from newchan.audit.checker import InvariantChecker
from newchan.bi_differ import diff_strokes
from newchan.core.bar import BarV1
from newchan.events import DomainEvent
from newchan.types import Bar


@dataclass
class BiEngineSnapshot:
    """一次 process_bar 后的完整快照。"""

    bar_idx: int
    bar_ts: float  # epoch 秒
    strokes: list[Stroke]
    events: list[DomainEvent]
    n_merged: int
    n_fractals: int
    merged_to_raw: list[tuple[int, int]] = field(default_factory=list)


_INITIAL_CAPACITY = 2000


class BiEngine:
    """笔事件引擎 — 逐 bar 驱动，增量计算，差分产生域事件。

    用法::

        engine = BiEngine()
        for bar in bars:
            snap = engine.process_bar(bar)
            for event in snap.events:
                handle(event)

    Parameters
    ----------
    stroke_mode : str
        笔模式，``"new"``（新笔，raw K线计数）、``"wide"``（旧笔宽，gap>=4 merged）
        或 ``"strict"``（旧笔严）。
    min_strict_sep : int
        严笔模式下两分型最小间距。
    """

    def __init__(
        self,
        stroke_mode: str = "new",
        min_strict_sep: int = 5,
        reset_dir_on_fractal: bool = False,
        new_raw_gap_min: int = 3,
    ) -> None:
        self._stroke_mode = stroke_mode
        self._min_strict_sep = min_strict_sep
        self._reset_dir_on_fractal = reset_dir_on_fractal
        self._new_raw_gap_min = new_raw_gap_min
        self._use_new_bi = stroke_mode == "new"
        self._min_gap = 4 if stroke_mode in ("wide", "new") else min_strict_sep

        self._bar_idx: int = -1
        self._event_seq: int = 0

        # ── 增量 merge 状态 ──
        # merge_buf[i] = [open, high, low, close, raw_start, raw_end]
        self._merge_buf: list[list[float | int]] = []
        self._merge_dir_state: str | None = None
        # 预分配 numpy 数组（merged highs/lows 用于 stroke 构造的 slice.max/min）
        self._m_cap: int = _INITIAL_CAPACITY
        self._m_highs: np.ndarray = np.empty(_INITIAL_CAPACITY, dtype=np.float64)
        self._m_lows: np.ndarray = np.empty(_INITIAL_CAPACITY, dtype=np.float64)
        self._m_count: int = 0
        # merged → raw 映射（增量维护）
        self._merged_to_raw: list[tuple[int, int]] = []

        # ── 增量 fractal 状态 ──
        self._confirmed_fractals: list[Fractal] = []
        self._pending_fractal: Fractal | None = None
        self._last_confirmed_m: int = 0

        # ── 增量 stroke 状态（two-pointer 检查点）──
        # _fxs: 从 confirmed fractals 增量 dedup+alternate 得到的序列
        self._fxs: list[Fractal] = []
        self._cp_i: int = 0       # 检查点 two-pointer i
        self._cp_j: int = 1       # 检查点 two-pointer j（下一个待处理位置）
        self._cp_strokes: list[Stroke] = []

        # ── diff 状态 ──
        self._prev_strokes: list[Stroke] = []
        self._diff_hint: int = 0
        self._checker = InvariantChecker()

    @property
    def bar_count(self) -> int:
        """已处理的 bar 总数。"""
        return self._bar_idx + 1

    @property
    def current_strokes(self) -> list[Stroke]:
        """当前快照的笔列表（浅拷贝）。"""
        return list(self._prev_strokes)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._bar_idx = -1
        self._event_seq = 0
        self._merge_buf.clear()
        self._merge_dir_state = None
        self._m_cap = _INITIAL_CAPACITY
        self._m_highs = np.empty(_INITIAL_CAPACITY, dtype=np.float64)
        self._m_lows = np.empty(_INITIAL_CAPACITY, dtype=np.float64)
        self._m_count = 0
        self._merged_to_raw.clear()
        self._confirmed_fractals.clear()
        self._pending_fractal = None
        self._last_confirmed_m = 0
        self._fxs.clear()
        self._cp_i = 0
        self._cp_j = 1
        self._cp_strokes.clear()
        self._prev_strokes.clear()
        self._diff_hint = 0
        self._checker.reset()

    # ================================================================
    # 增量包含处理 — O(1) per bar
    # ================================================================

    def _ensure_merged_capacity(self) -> None:
        if self._m_count >= self._m_cap:
            self._m_cap *= 2
            new_h = np.empty(self._m_cap, dtype=np.float64)
            new_l = np.empty(self._m_cap, dtype=np.float64)
            new_h[: self._m_count] = self._m_highs[: self._m_count]
            new_l[: self._m_count] = self._m_lows[: self._m_count]
            self._m_highs = new_h
            self._m_lows = new_l

    def _incremental_merge(self, bar: Bar | BarV1) -> bool:
        """处理一根新 bar 的包含关系。返回 True 表示新增了 merged bar。"""
        i = self._bar_idx
        curr_h: float = bar.high
        curr_l: float = bar.low
        curr_o: float = bar.open
        curr_c: float = bar.close

        if not self._merge_buf:
            self._merge_buf.append([curr_o, curr_h, curr_l, curr_c, i, i])
            self._ensure_merged_capacity()
            self._m_highs[0] = curr_h
            self._m_lows[0] = curr_l
            self._m_count = 1
            self._merged_to_raw.append((i, i))
            return True

        last = self._merge_buf[-1]
        last_h: float = last[1]
        last_l: float = last[2]

        has_inclusion = (last_h >= curr_h and last_l <= curr_l) or (
            curr_h >= last_h and curr_l <= last_l
        )

        if has_inclusion:
            if self._merge_dir_state is not None:
                effective_up = self._merge_dir_state == "UP"
            else:
                effective_up = last[3] >= last[0]
            if effective_up:
                last[1] = max(last_h, curr_h)
                last[2] = max(last_l, curr_l)
            else:
                last[1] = min(last_h, curr_h)
                last[2] = min(last_l, curr_l)
            last[3] = curr_c
            last[5] = i
            mc = self._m_count
            self._m_highs[mc - 1] = last[1]
            self._m_lows[mc - 1] = last[2]
            old_start = self._merged_to_raw[-1][0]
            self._merged_to_raw[-1] = (old_start, i)
            return False

        prev_dir = self._merge_dir_state
        if curr_h > last_h and curr_l > last_l:
            self._merge_dir_state = "UP"
        elif curr_h < last_h and curr_l < last_l:
            self._merge_dir_state = "DOWN"
        self._merge_buf.append([curr_o, curr_h, curr_l, curr_c, i, i])

        if self._reset_dir_on_fractal:
            if prev_dir is not None and self._merge_dir_state != prev_dir:
                self._merge_dir_state = None
            elif _is_fractal_pattern(self._merge_buf, len(self._merge_buf)):
                self._merge_dir_state = None

        self._ensure_merged_capacity()
        mc = self._m_count
        self._m_highs[mc] = curr_h
        self._m_lows[mc] = curr_l
        self._m_count = mc + 1
        self._merged_to_raw.append((i, i))
        return True

    # ================================================================
    # 增量分型检测 — O(1) per bar
    # ================================================================

    def _update_fractals(self) -> bool:
        """更新 confirmed/pending fractals。返回 True 表示 fractals 有变化。"""
        m = self._m_count
        if m < 3:
            return False

        changed = False

        if m > self._last_confirmed_m:
            if self._pending_fractal is not None:
                self._confirmed_fractals.append(self._pending_fractal)
                self._add_confirmed_to_fxs(self._pending_fractal)
                changed = True
            self._last_confirmed_m = m

        new_pending = _classify_fractal(
            self._m_highs[m - 3],
            self._m_highs[m - 2],
            self._m_highs[m - 1],
            self._m_lows[m - 3],
            self._m_lows[m - 2],
            self._m_lows[m - 1],
            idx=m - 2,
        )
        if new_pending != self._pending_fractal:
            changed = True
        self._pending_fractal = new_pending
        return changed

    # ================================================================
    # 增量笔构造 — checkpoint two-pointer
    # ================================================================

    def _add_confirmed_to_fxs(self, fx: Fractal) -> None:
        """将确认的分型加入 deduped fxs 并推进检查点。"""
        if not self._fxs:
            self._fxs.append(fx)
            return

        last = self._fxs[-1]
        if fx.kind == last.kind:
            if _is_more_extreme(fx, last):
                self._fxs[-1] = fx
            return

        self._advance_checkpoint_step()
        self._fxs.append(fx)

    def _advance_checkpoint_step(self) -> None:
        """推进 two-pointer 一步（处理 _fxs[cp_j]）。"""
        j = self._cp_j
        fxs = self._fxs
        if j >= len(fxs) or j < 1:
            if j < len(fxs):
                self._cp_j = j + 1
            return

        i = self._cp_i
        strokes = self._cp_strokes
        highs = self._m_highs[: self._m_count]
        lows = self._m_lows[: self._m_count]

        start, cand = fxs[i], fxs[j]

        if cand.kind == start.kind:
            if _is_more_extreme(cand, start):
                if strokes:
                    _extend_prev_stroke(strokes, cand, highs, lows)
                self._cp_i = j
            self._cp_j = j + 1
            return

        if not _check_gap(
            start,
            cand,
            self._use_new_bi,
            self._min_gap,
            self._merged_to_raw,
            self._new_raw_gap_min,
        ):
            self._cp_j = j + 1
            return

        result = _validate_direction(start, cand)
        if result is None:
            self._cp_j = j + 1
            return
        direction, valid = result
        if not valid:
            self._cp_j = j + 1
            return

        strokes.append(_build_stroke(start, cand, direction, highs, lows))
        self._cp_i = j
        self._cp_j = j + 1

    def _compute_strokes_from_checkpoint(self) -> list[Stroke]:
        """从检查点恢复，处理 fxs 尾部 + pending，返回完整笔序列。"""
        pending = self._pending_fractal
        fxs = self._fxs

        # 构建 ext_fxs：base fxs + 可能的 pending（dedup 合入）
        if pending is None or not fxs:
            tail_fxs = [pending] if (pending is not None and not fxs) else fxs
        elif pending.kind == fxs[-1].kind and _is_more_extreme(pending, fxs[-1]):
            tail_fxs = fxs[:-1]  # 不拷贝全量，下面直接 append
            tail_fxs = list(fxs)
            tail_fxs[-1] = pending
        elif pending.kind != fxs[-1].kind:
            tail_fxs = fxs  # 延后 append
        else:
            tail_fxs = fxs

        # 处理 pending 追加（不同类型）
        append_pending = (
            pending is not None
            and fxs
            and pending.kind != fxs[-1].kind
        )
        effective_len = len(tail_fxs) + (1 if append_pending else 0)

        if effective_len < 2:
            return []

        i = self._cp_i
        j = self._cp_j
        cp = self._cp_strokes
        cp_len = len(cp)
        highs = self._m_highs[: self._m_count]
        lows = self._m_lows[: self._m_count]

        # 尾部缓冲：仅含可能被修改的最后一笔（_extend_prev_stroke 只改 strokes[-1]）
        # + 后续新增笔。确认前缀 cp[:cp_len-1] 不被本循环触及，故无需每 bar 拷贝
        # （实测前缀均长 ~6000、尾部迭代≈1，原 list(cp) 拷贝是主导 O(N²) 项）。
        if cp_len:
            tail = [cp[-1]]
            base_len = cp_len - 1
        else:
            tail = []
            base_len = 0

        total_len = len(tail_fxs) + (1 if append_pending else 0)

        while j < total_len:
            if j < 1:
                j = 1
                continue

            cand_fx = (
                tail_fxs[j]
                if j < len(tail_fxs)
                else pending  # type: ignore[assignment]
            )
            start_fx = (
                tail_fxs[i]
                if i < len(tail_fxs)
                else pending  # type: ignore[assignment]
            )

            if cand_fx.kind == start_fx.kind:
                if _is_more_extreme(cand_fx, start_fx):
                    if tail:
                        _extend_prev_stroke(tail, cand_fx, highs, lows)
                    i = j
                j += 1
                continue

            if not _check_gap(
                start_fx,
                cand_fx,
                self._use_new_bi,
                self._min_gap,
                self._merged_to_raw,
                self._new_raw_gap_min,
            ):
                j += 1
                continue

            result = _validate_direction(start_fx, cand_fx)
            if result is None:
                j += 1
                continue
            direction, valid = result
            if not valid:
                j += 1
                continue

            tail.append(
                _build_stroke(start_fx, cand_fx, direction, highs, lows)
            )
            i = j
            j += 1

        # 最后一笔置为未确认（与原全量路径一致）。
        if tail:
            last = tail[-1]
            if last.confirmed:
                tail[-1] = Stroke(
                    i0=last.i0, i1=last.i1, direction=last.direction,
                    high=last.high, low=last.low,
                    p0=last.p0, p1=last.p1, confirmed=False,
                )

        if not tail:
            return []

        # 结果不变 → 复用 prev 引用，零拷贝（实测 72.5% 的调用）。判据与
        # process_bar 的 unchanged 检查同构（len + 末笔身份）；cp 单调增长 ⟹
        # 前缀 cp[:base_len] 与 prev 前缀逐元素一致，故 len+末笔相等 ⟹ 全列表相等。
        prev = self._prev_strokes
        new_len = base_len + len(tail)
        t_last = tail[-1]
        if (
            len(prev) == new_len
            and prev
            and prev[-1].i0 == t_last.i0
            and prev[-1].i1 == t_last.i1
            and abs(prev[-1].p1 - t_last.p1) < 1e-9
        ):
            return prev

        # 结果变化 → 构建完整列表（O(N)，仅 27.5% 的调用）。
        result_strokes = list(cp[:base_len])
        result_strokes.extend(tail)
        return result_strokes

    # ================================================================
    # diff 与不变量检查
    # ================================================================

    def _diff_and_check(
        self,
        strokes: list[Stroke],
        bar_idx: int,
        bar_ts: float,
    ) -> list[DomainEvent]:
        """差分前后 Stroke 快照并执行不变量检查，返回事件列表。"""
        events = diff_strokes(
            self._prev_strokes,
            strokes,
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            seq_start=self._event_seq,
            known_common_prefix=self._diff_hint,
        )
        self._event_seq += len(events)

        violations = self._checker.check(events, bar_idx, bar_ts)
        if violations:
            events = list(events) + violations
            self._event_seq += len(violations)

        return events

    # ================================================================
    # 主入口
    # ================================================================

    def process_bar(self, bar: Bar | BarV1) -> BiEngineSnapshot:
        """处理一根新 K 线，返回快照（含本 bar 产生的事件）。

        保证：
        1. 仅使用 bars[:bar_idx+1] 的数据（无未来函数）
        2. 增量计算等价于全量纯函数重跑
        3. diff 产生事件
        """
        self._bar_idx += 1
        bar_ts = _dt_to_epoch(bar.ts)

        new_merged = self._incremental_merge(bar)
        fractals_changed = self._update_fractals()

        if fractals_changed:
            strokes = self._compute_strokes_from_checkpoint()
            prev = self._prev_strokes
            if (
                len(strokes) == len(prev)
                and (
                    not strokes
                    or (
                        strokes[-1].i1 == prev[-1].i1
                        and strokes[-1].i0 == prev[-1].i0
                        and abs(strokes[-1].p1 - prev[-1].p1) < 1e-9
                    )
                )
            ):
                events = []
            else:
                events = self._diff_and_check(strokes, self._bar_idx, bar_ts)
                self._prev_strokes = strokes
            self._diff_hint = max(0, len(self._cp_strokes) - 1)
        else:
            strokes = self._prev_strokes
            events = []

        return BiEngineSnapshot(
            bar_idx=self._bar_idx,
            bar_ts=bar_ts,
            strokes=strokes,
            events=events,
            n_merged=self._m_count,
            n_fractals=len(self._confirmed_fractals)
            + (1 if self._pending_fractal else 0),
            merged_to_raw=self._merged_to_raw,
        )


def _dt_to_epoch(dt: datetime) -> float:
    """datetime → epoch 秒。naive datetime 视为 UTC。"""
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.timestamp()
