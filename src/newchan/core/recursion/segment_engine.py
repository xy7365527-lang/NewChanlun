"""SegmentEngine — 事件驱动线段引擎（增量优化）

核心流程（Diff-based + 两层加速）：
1. 接收 BiEngineSnapshot（含 strokes 快照 + 笔事件）
2. 层1 跳过：笔列表未变 → 直接返回缓存（~98% 的 bar）
3. 层2 恢复：笔列表变化 → 从最后确认线段断点增量计算
4. diff_segments(prev, curr) 产生线段事件
5. 为每个事件计算确定性 event_id

架构对齐：
- 与 BiEngine 同构——纯函数 + 差分产生事件
- 增量优化不改变语义：segments_from_strokes_v1 的 _resume 参数
  在数学上等价于全量计算（已确认线段不因后续笔变化而改变）
- SegmentEngine 不修改 BiEngine 的输出
"""

from __future__ import annotations

from newchan.a_segment_v0 import Segment
from newchan.a_segment_v1 import _FeatureSeqState, segments_from_strokes_v1
from newchan.a_stroke import Stroke
from newchan.bi_engine import BiEngineSnapshot
from newchan.core.recursion.segment_state import SegmentSnapshot, diff_segments
from newchan.events import DomainEvent

_SECOND_SEQ_MARGIN: int = _FeatureSeqState.MAX_SECOND_SEQ_SCAN


def _stroke_tail_key(strokes: list[Stroke]) -> tuple:
    """最后两根笔的身份指纹，用于快速变化检测。"""
    n = len(strokes)
    if n == 0:
        return ()
    if n >= 2:
        s1, s2 = strokes[-2], strokes[-1]
        return (n, s1.i0, s1.i1, s1.p0, s1.p1, s2.i0, s2.i1, s2.p0, s2.p1)
    s1 = strokes[-1]
    return (n, s1.i0, s1.i1, s1.p0, s1.p1)


class SegmentEngine:
    """事件驱动线段引擎 — 消费 BiEngineSnapshot，产生 segment 事件。

    两层增量优化：
    - 层1：笔列表未变（~98% 的 bar 不产生新笔）→ O(1) 返回
    - 层2：笔列表变化时从最后确认线段断点恢复 → O(tail) 而非 O(n)

    用法::

        seg_engine = SegmentEngine()
        for bar in bars:
            bi_snap = bi_engine.process_bar(bar)
            seg_snap = seg_engine.process_snapshot(bi_snap)
            for event in seg_snap.events:
                handle(event)

    Parameters
    ----------
    stream_id : str
        所属流标识（透传到事件中，仅用于日志）。
    """

    def __init__(self, stream_id: str = "") -> None:
        self._prev_segments: list[Segment] = []
        self._event_seq: int = 0
        self._stream_id = stream_id
        # 层1：变化检测
        self._last_tail_key: tuple = ()
        # 层2：增量恢复检查点
        self._cp_segments: list[Segment] = []
        self._cp_seg_start: int = 0
        self._cp_seg_dir: str = "up"
        self._cp_stroke_key: tuple = ()
        self._cp_valid: bool = False
        self._diff_hint: int = 0
        # 稳定前缀续扫状态（修复 _update_checkpoint 的 O(N²)）
        self._cp_stable_count: int = 0
        self._cp_stable_key: tuple = ()

    @property
    def current_segments(self) -> list[Segment]:
        """当前线段列表（浅拷贝）。"""
        return list(self._prev_segments)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._prev_segments = []
        self._event_seq = 0
        self._last_tail_key = ()
        self._cp_segments = []
        self._cp_seg_start = 0
        self._cp_seg_dir = "up"
        self._cp_stroke_key = ()
        self._cp_valid = False
        self._diff_hint = 0
        self._cp_stable_count = 0
        self._cp_stable_key = ()

    def _try_resume(self, strokes: list[Stroke]) -> tuple[list[Segment], int, str] | None:
        """检查增量恢复检查点是否有效。"""
        if not self._cp_valid or not self._cp_segments:
            return None
        if self._cp_seg_start >= len(strokes):
            return None
        sk = strokes[self._cp_seg_start]
        key = (sk.i0, sk.i1, round(sk.p0, 8), round(sk.p1, 8))
        if key != self._cp_stroke_key:
            return None
        return (self._cp_segments, self._cp_seg_start, self._cp_seg_dir)

    def _update_checkpoint(self, segments: list[Segment], strokes: list[Stroke]) -> None:
        """从当前线段列表更新增量恢复检查点。

        只缓存稳定的已确认线段：任何已确认段（不分 gap_type）都需要
        trigger_k + 1 + MARGIN <= len(strokes)，保证段内所有候选缺口分型的
        第二特征序列扫描窗口已完全展开，后续新增笔不影响该段的断点。

        增量续扫（修复 O(N²)）：稳定前缀单调增长——confirmed 段不变，且
        稳定条件 trigger_k+1+MARGIN <= n_strokes 随 n_strokes
        只增不减，故一旦稳定永远稳定。从上次 stable_count 续扫，并以边界段
        身份键做 O(1) 校验（不匹配则回退全扫，语义等价）。stable_count 未
        增长时检查点字段全部不变，直接早退避免 O(N) 的 segments[:k] 切片。
        认识论等级：L0（单调不变量推导，逐位等价）。
        """
        n_strokes = len(strokes)
        n_seg = len(segments)

        # O(1) 校验缓存边界仍有效，确定续扫起点
        start = self._cp_stable_count
        if start > n_seg:
            start = 0
        elif start > 0:
            bnd = segments[start - 1]
            if (bnd.s0, bnd.s1, bnd.i0, bnd.i1) != self._cp_stable_key:
                start = 0

        stable_count = start
        while stable_count < n_seg:
            seg = segments[stable_count]
            if not seg.confirmed:
                break
            if seg.break_evidence is None:
                break
            # 稳定性判据对 gap_type 无关（修复 resume≠full 发散）：
            # 一个最终 gap_type="none" 的段，段内更早处可能存在被跳过的缺口
            # 分型——当时第二特征序列窗口未成形而 continue，扫描继续到更远处
            # 才以 none 断段。后续笔到达后该缺口分型的第二序列成形，full 会在
            # 更早处以 second 断段，使该段缩短。scan_trigger 唯一的前向依赖是
            # _second_seq_has_fractal（窗口 <= MARGIN），且段内所有候选分型的
            # b_stroke <= trigger_k，故 trigger_k+1+MARGIN<=n_strokes 是与
            # gap_type 无关的充分稳定条件。trigger_k 固定、n_strokes 单调增，
            # 故一旦稳定永远稳定（续扫单调性保持）。
            trigger_k = seg.break_evidence.trigger_stroke_k
            if trigger_k + 1 + _SECOND_SEQ_MARGIN > n_strokes:
                break
            stable_count += 1

        if stable_count == 0:
            self._cp_valid = False
            self._cp_stable_count = 0
            return

        # stable_count 未增长且检查点仍有效 → 所有字段不变，跳过重建
        if stable_count == start and self._cp_valid:
            return

        self._cp_stable_count = stable_count
        bnd = segments[stable_count - 1]
        self._cp_stable_key = (bnd.s0, bnd.s1, bnd.i0, bnd.i1)

        last = segments[stable_count - 1]
        seg_start = last.break_evidence.trigger_stroke_k
        if seg_start < n_strokes:
            seg_dir = "down" if last.direction == "up" else "up"
            sk = strokes[seg_start]
            self._cp_segments = segments[:stable_count]
            self._cp_seg_start = seg_start
            self._cp_seg_dir = seg_dir
            self._cp_stroke_key = (sk.i0, sk.i1, round(sk.p0, 8), round(sk.p1, 8))
            self._cp_valid = True
        else:
            self._cp_valid = False

    def process_snapshot(self, snap: BiEngineSnapshot) -> SegmentSnapshot:
        """处理一个 BiEngine 快照，产生 segment 事件。

        层1：笔列表未变 → 直接返回缓存的线段列表（无事件）。
        层2：笔列表变化 → 从检查点增量恢复计算。
        回退：检查点无效 → 全量计算。

        Parameters
        ----------
        snap : BiEngineSnapshot
            包含当前笔列表和笔事件的快照。

        Returns
        -------
        SegmentSnapshot
            包含当前线段列表和本轮产生的线段事件。
        """
        # 层1：笔列表未变 → 直接返回缓存
        tail_key = _stroke_tail_key(snap.strokes)
        if tail_key == self._last_tail_key:
            # 返回引用而非拷贝：引擎从不原地 mutate _prev_segments（只整体替换），
            # 消费者不 mutate 快照列表（已核查），与 fresh 路径 + BiEngine 一致。
            # 消除 99% 安静 bar 上 O(N_segs)/bar 的拷贝税。
            return SegmentSnapshot(
                bar_idx=snap.bar_idx,
                bar_ts=snap.bar_ts,
                segments=self._prev_segments,
                events=[],
            )
        self._last_tail_key = tail_key

        resume = self._try_resume(snap.strokes)
        if resume is not None:
            curr_segments = segments_from_strokes_v1(snap.strokes, _resume=resume)
        else:
            curr_segments = segments_from_strokes_v1(snap.strokes)

        self._update_checkpoint(curr_segments, snap.strokes)

        events = diff_segments(
            self._prev_segments,
            curr_segments,
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            seq_start=self._event_seq,
            known_common_prefix=self._diff_hint,
        )
        self._event_seq += len(events)

        self._diff_hint = max(0, len(self._cp_segments) - 1) if self._cp_valid else 0
        self._prev_segments = curr_segments

        return SegmentSnapshot(
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            segments=curr_segments,
            events=events,
        )
