"""RecursiveOrchestrator — 口径 A 正式路径。

[新缠论] 从 K 线出发，经五层管线构造 level=1 走势类型，
再通过 RecursiveStack 自动递归构造更高级别中枢和走势。

引擎链：
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine
                                                  ↓ MoveSnapshot (level=1)
                                            RecursiveStack
                                                  ↓
                                        (自动递归至终止)
"""

from __future__ import annotations

from dataclasses import dataclass, field

import pandas as pd

from newchan.a_macd import OnlineMacdState
from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.core.recursion.buysellpoint_engine import BuySellPointEngine
from newchan.core.recursion.buysellpoint_state import BuySellPointSnapshot
from newchan.core.recursion.move_engine import MoveEngine
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.core.recursion.recursive_stack import RecursiveStack
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot
from newchan.a_level_fsm_newchan import LStar
from newchan.events import DomainEvent
from newchan.orchestrator.bus import EventBus
from newchan.types import Bar


@dataclass
class RecursiveOrchestratorSnapshot:
    """一次 process_bar 后的完整快照。

    包含 level=1 五层管线快照 + 递归层快照。
    """

    bar_idx: int
    bar_ts: float
    bi_snapshot: BiEngineSnapshot
    seg_snapshot: SegmentSnapshot
    zs_snapshot: ZhongshuSnapshot
    move_snapshot: MoveSnapshot
    bsp_snapshot: BuySellPointSnapshot
    recursive_snapshots: list[RecursiveLevelSnapshot] = field(default_factory=list)
    all_events: list[DomainEvent] = field(default_factory=list)
    lstar: LStar | None = None


class RecursiveOrchestrator:
    """口径 A 递归调度器 — 从 K 线出发，构造全部递归级别。

    Parameters
    ----------
    stream_id : str
        流标识（透传到各引擎）。
    max_levels : int
        最大递归深度（透传到 RecursiveStack）。默认 6。
    stroke_mode : str
        笔模式（透传到 BiEngine）。默认 "wide"。
    min_strict_sep : int
        严格分型最小间隔（透传到 BiEngine）。默认 5。
    """

    def __init__(
        self,
        stream_id: str = "",
        max_levels: int = 6,
        stroke_mode: str = "wide",
        min_strict_sep: int = 5,
        reset_dir_on_fractal: bool = False,
        new_raw_gap_min: int = 3,
        enable_macd_divergence: bool = False,
    ) -> None:
        self._stream_id = stream_id
        self._max_levels = max_levels
        self._enable_macd_divergence = enable_macd_divergence

        # MACD 背驰开启时用增量 EMA 状态机，每 bar O(1) 更新（问题3修复）。
        # to_dataframe() 返回历史 DataFrame 供 macd_area_for_range 位置索引查询。
        self._macd_state: OnlineMacdState | None = (
            OnlineMacdState() if enable_macd_divergence else None
        )

        # Level=1 五层管线
        self._bi_engine = BiEngine(
            stroke_mode=stroke_mode,
            min_strict_sep=min_strict_sep,
            reset_dir_on_fractal=reset_dir_on_fractal,
            new_raw_gap_min=new_raw_gap_min,
        )
        self._seg_engine = SegmentEngine(stream_id=stream_id)
        self._zs_engine = ZhongshuEngine(stream_id=stream_id)
        self._move_engine = MoveEngine(stream_id=stream_id)
        self._bsp_engine = BuySellPointEngine(
            level_id=1, stream_id=stream_id,
        )

        # 递归栈（level ≥ 2）
        self._recursive_stack = RecursiveStack(
            max_levels=max_levels, stream_id=stream_id,
        )

        # 事件总线
        self.bus = EventBus()

        # LStar 变更检测
        self._last_lstar_key: tuple = ()
        self._cached_lstar: LStar | None = None

    @property
    def max_levels(self) -> int:
        """最大递归深度。"""
        return self._max_levels

    def reset(self) -> None:
        """重置所有引擎到初始状态。"""
        if self._macd_state is not None:
            self._macd_state.reset()
        self._bi_engine.reset()
        self._seg_engine.reset()
        self._zs_engine.reset()
        self._move_engine.reset()
        self._bsp_engine.reset()
        self._recursive_stack.reset()
        self._last_lstar_key = ()
        self._cached_lstar = None

    def _collect_events(
        self,
        bi_snap: BiEngineSnapshot,
        seg_snap: SegmentSnapshot,
        zs_snap: ZhongshuSnapshot,
        move_snap: MoveSnapshot,
        bsp_snap: BuySellPointSnapshot,
        recursive_snaps: list[RecursiveLevelSnapshot],
    ) -> list[DomainEvent]:
        """收集所有层级的域事件并推入事件总线。"""
        if (
            not bi_snap.events
            and not seg_snap.events
            and not zs_snap.events
            and not move_snap.events
            and not bsp_snap.events
            and all(not rs.zhongshu_events and not rs.move_events for rs in recursive_snaps)
        ):
            return []

        all_events: list[DomainEvent] = list(bi_snap.events)
        if seg_snap.events:
            all_events.extend(seg_snap.events)
        if zs_snap.events:
            all_events.extend(zs_snap.events)
        if move_snap.events:
            all_events.extend(move_snap.events)
        if bsp_snap.events:
            all_events.extend(bsp_snap.events)
        for rs in recursive_snaps:
            if rs.zhongshu_events:
                all_events.extend(rs.zhongshu_events)
            if rs.move_events:
                all_events.extend(rs.move_events)

        self.bus.push("L1", all_events, stream_id=self._stream_id)
        for rs in recursive_snaps:
            level_events = list(rs.zhongshu_events) + list(rs.move_events)
            if level_events:
                self.bus.push_level(
                    rs.level_id, level_events, stream_id=self._stream_id,
                )
        return all_events

    def _compute_macd(self, bar: Bar) -> pd.DataFrame:
        """增量更新 MACD 状态，返回历史 DataFrame（O(1) EMA 更新，问题3修复）。

        因果性：只用截至当前 bar 的 close，无未来函数。
        认识论等级：L0（EMA 递推 = 数学恒等式）。
        """
        assert self._macd_state is not None
        self._macd_state.update(bar.close, bar.ts)
        return self._macd_state.to_dataframe()

    def process_bar(self, bar: Bar) -> RecursiveOrchestratorSnapshot:
        """逐 bar 驱动全链，返回完整快照。

        链路：bar → BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine
                                                                      ↓
                                                                RecursiveStack
        """
        # Level=1 五层管线
        bi_snap = self._bi_engine.process_bar(bar)
        seg_snap = self._seg_engine.process_snapshot(bi_snap)
        zs_snap = self._zs_engine.process_segment_snapshot(seg_snap)
        move_snap = self._move_engine.process_zhongshu_snapshot(
            zs_snap, num_segments=len(seg_snap.segments),
        )

        # MACD 背驰（问题3）：累积 raw close 全量重算，索引按 raw bar 顺序对齐
        # bi_snap.merged_to_raw。关闭时传 None → 价格振幅 fallback（原行为）。
        df_macd = self._compute_macd(bar) if self._enable_macd_divergence else None
        bsp_snap = self._bsp_engine.process_snapshots(
            move_snap, zs_snap, seg_snap,
            df_macd=df_macd,
            merged_to_raw=bi_snap.merged_to_raw if df_macd is not None else None,
        )

        # 递归层（level ≥ 2）
        recursive_snaps = self._recursive_stack.process_level1_move_snapshot(
            move_snap,
        )

        all_events = self._collect_events(
            bi_snap, seg_snap, zs_snap, move_snap, bsp_snap, recursive_snaps,
        )

        snap = RecursiveOrchestratorSnapshot(
            bar_idx=bi_snap.bar_idx,
            bar_ts=bi_snap.bar_ts,
            bi_snapshot=bi_snap,
            seg_snapshot=seg_snap,
            zs_snapshot=zs_snap,
            move_snapshot=move_snap,
            bsp_snapshot=bsp_snap,
            recursive_snapshots=recursive_snaps,
            all_events=all_events,
            lstar=None,
        )

        n_segs = len(seg_snap.segments)
        n_zs = len(zs_snap.zhongshus)
        n_moves = len(move_snap.moves)
        n_rec = len(recursive_snaps)
        lstar_key = (n_segs, n_zs, n_moves, n_rec)
        if lstar_key == self._last_lstar_key:
            snap.lstar = self._cached_lstar
        else:
            from newchan.a_level_fsm_adapter import select_lstar_from_recursive_snapshot  # noqa: E402
            snap.lstar = select_lstar_from_recursive_snapshot(snap, bar.close)
            self._last_lstar_key = lstar_key
            self._cached_lstar = snap.lstar

        return snap
