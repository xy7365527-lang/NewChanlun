"""TFOrchestrator — 多级别并行调度器

持有多个 ReplaySession（每 TF 一个），以 base TF 时间戳为锚点
驱动高 TF 步进。

核心规则：
- base TF 每步进 1 bar，检查高 TF 是否有 bar 的 close time ≤ 当前 base 时间
- 有 → 该 TF 步进；无 → 跳过
- 各 TF 的 BiEngine 完全独立，互不污染
"""

from __future__ import annotations

from datetime import datetime, timezone

import pandas as pd

from newchan.b_timeframe import resample_ohlc
from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.core.recursion.buysellpoint_engine import BuySellPointEngine
from newchan.core.recursion.move_engine import MoveEngine
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.orchestrator.bus import EventBus
from newchan.replay import ReplaySession
from newchan.types import Bar


def _dt_to_epoch(dt: datetime) -> float:
    """datetime → epoch 秒。naive datetime 视为 UTC。"""
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.timestamp()


def _bars_to_df(bars: list[Bar]) -> pd.DataFrame:
    """Bar 列表 → pandas DataFrame（用于 resample）。"""
    data = {
        "open": [b.open for b in bars],
        "high": [b.high for b in bars],
        "low": [b.low for b in bars],
        "close": [b.close for b in bars],
    }
    volumes = [b.volume for b in bars]
    if any(v is not None for v in volumes):
        data["volume"] = [v if v is not None else 0.0 for v in volumes]
    index = pd.DatetimeIndex([b.ts for b in bars], name="time")
    return pd.DataFrame(data, index=index)


def _df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """pandas DataFrame → Bar 列表。"""
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        dt = ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts
        if not isinstance(dt, datetime):
            dt = pd.Timestamp(dt).to_pydatetime()
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        vol = float(row["volume"]) if "volume" in row.index and pd.notna(row["volume"]) else None
        bars.append(Bar(
            ts=dt,
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=vol,
        ))
    return bars


class TFOrchestrator:
    """多级别并行调度器。

    Parameters
    ----------
    session_id : str
        顶层会话标识。
    base_bars : list[Bar]
        base TF 的完整 bar 序列。
    timeframes : list[str]
        TF 列表，第一个为 base TF，例如 ``["5m", "30m"]``。
    stroke_mode : str
        笔模式（传给每个 BiEngine）。
    min_strict_sep : int
        严笔最小间距。

    Usage::

        orch = TFOrchestrator("sid", bars, ["5m", "30m"])
        result = orch.step(1)
        # result["5m"] = [BiEngineSnapshot, ...]
        # result["30m"] = [BiEngineSnapshot, ...]  # 可能为空
    """

    def __init__(
        self,
        session_id: str,
        base_bars: list[Bar],
        timeframes: list[str],
        stroke_mode: str = "wide",
        min_strict_sep: int = 5,
        symbol: str = "",
    ) -> None:
        if not timeframes:
            raise ValueError("timeframes 不能为空")

        self.session_id = session_id
        self.base_tf = timeframes[0]
        self.timeframes = list(timeframes)
        self.symbol = symbol
        self.bus = EventBus()

        # MVP-B0: 为每个 TF 计算 stream_id
        self._stream_ids: dict[str, str] = {}
        if symbol:
            from newchan.core.adapters import tf_to_stream_id
            for tf in self.timeframes:
                sid = tf_to_stream_id(symbol=symbol, tf=tf)
                self._stream_ids[tf] = sid.value

        # MVP-B1: 为每个 TF 创建 SegmentEngine
        self._segment_engines: dict[str, SegmentEngine] = {}
        for tf in self.timeframes:
            sid = self._stream_ids.get(tf, "")
            self._segment_engines[tf] = SegmentEngine(stream_id=sid)

        # MVP-C0: 为每个 TF 创建 ZhongshuEngine
        self._zhongshu_engines: dict[str, ZhongshuEngine] = {}
        for tf in self.timeframes:
            sid = self._stream_ids.get(tf, "")
            self._zhongshu_engines[tf] = ZhongshuEngine(stream_id=sid)

        # MVP-D0: 为每个 TF 创建 MoveEngine
        self._move_engines: dict[str, MoveEngine] = {}
        for tf in self.timeframes:
            sid = self._stream_ids.get(tf, "")
            self._move_engines[tf] = MoveEngine(stream_id=sid)

        # MVP-E0: 为每个 TF 创建 BuySellPointEngine
        self._bsp_engines: dict[str, BuySellPointEngine] = {}
        for tf_idx, tf in enumerate(self.timeframes):
            sid = self._stream_ids.get(tf, "")
            self._bsp_engines[tf] = BuySellPointEngine(
                level_id=tf_idx + 1, stream_id=sid,
            )

        # 为每个 TF 创建独立 session
        self.sessions: dict[str, ReplaySession] = {}
        self._higher_tf_bars: dict[str, list[Bar]] = {}

        # base TF session
        engine_base = BiEngine(stroke_mode=stroke_mode, min_strict_sep=min_strict_sep)
        self.sessions[self.base_tf] = ReplaySession(
            session_id=f"{session_id}_{self.base_tf}",
            bars=base_bars,
            engine=engine_base,
        )

        # 高 TF sessions：预重采样
        if len(timeframes) > 1:
            df_base = _bars_to_df(base_bars)
            for tf in timeframes[1:]:
                df_resampled = resample_ohlc(df_base, tf)
                tf_bars = _df_to_bars(df_resampled)
                engine = BiEngine(stroke_mode=stroke_mode, min_strict_sep=min_strict_sep)
                self.sessions[tf] = ReplaySession(
                    session_id=f"{session_id}_{tf}",
                    bars=tf_bars,
                    engine=engine,
                )
                self._higher_tf_bars[tf] = tf_bars

    @property
    def base_session(self) -> ReplaySession:
        """base TF 的 ReplaySession。"""
        return self.sessions[self.base_tf]

    @property
    def current_idx(self) -> int:
        """base TF 的当前 bar 索引。"""
        return self.base_session.current_idx

    @property
    def total_bars(self) -> int:
        """base TF 的 bar 总数。"""
        return self.base_session.total_bars

    @property
    def mode(self) -> str:
        """base TF 的模式。"""
        return self.base_session.mode

    @mode.setter
    def mode(self, value: str) -> None:
        self.base_session.mode = value

    @property
    def speed(self) -> float:
        return self.base_session.speed

    @speed.setter
    def speed(self, value: float) -> None:
        self.base_session.speed = value

    @property
    def bars(self) -> list[Bar]:
        """base TF 的 bar 列表。"""
        return self.base_session.bars

    def step(self, count: int = 1) -> dict[str, list[BiEngineSnapshot]]:
        """步进 base TF count 根 bar。

        高 TF 根据时间戳对齐自动步进。
        返回各 TF 的快照列表（可能为空表示该 TF 本轮无步进）。
        所有事件同时进入 EventBus（带 tf 标签）。
        """
        result: dict[str, list[BiEngineSnapshot]] = {tf: [] for tf in self.timeframes}

        for _ in range(count):
            if self.base_session.current_idx >= self.base_session.total_bars:
                break

            # 获取当前 base bar 的时间戳（步进前）
            base_bar_idx = self.base_session.current_idx
            base_bar = self.base_session.bars[base_bar_idx]
            base_ts = _dt_to_epoch(base_bar.ts)

            # 步进 base TF
            base_snaps = self.base_session.step(1)
            result[self.base_tf].extend(base_snaps)
            base_sid = self._stream_ids.get(self.base_tf, "")
            for snap in base_snaps:
                # SegmentEngine: 追加 segment 事件到 snap.events
                seg_snap = self._segment_engines[self.base_tf].process_snapshot(snap)
                if seg_snap.events:
                    snap.events = list(snap.events) + seg_snap.events
                # ZhongshuEngine: 追加 zhongshu 事件到 snap.events
                zs_snap = self._zhongshu_engines[self.base_tf].process_segment_snapshot(seg_snap)
                if zs_snap.events:
                    snap.events = list(snap.events) + zs_snap.events
                # MoveEngine: 追加 move 事件到 snap.events
                move_snap = self._move_engines[self.base_tf].process_zhongshu_snapshot(zs_snap)
                if move_snap.events:
                    snap.events = list(snap.events) + move_snap.events
                # BuySellPointEngine: 追加 bsp 事件到 snap.events
                bsp_snap = self._bsp_engines[self.base_tf].process_snapshots(
                    move_snap, zs_snap, seg_snap,
                )
                if bsp_snap.events:
                    snap.events = list(snap.events) + bsp_snap.events
                self.bus.push(
                    self.base_tf, snap.events,
                    stream_id=base_sid,
                )

            # 检查高 TF 是否需要步进
            for tf in self.timeframes[1:]:
                sess = self.sessions[tf]
                tf_sid = self._stream_ids.get(tf, "")
                # 步进所有 close time ≤ base_ts 的高 TF bar
                while sess.current_idx < sess.total_bars:
                    next_bar = sess.bars[sess.current_idx]
                    next_ts = _dt_to_epoch(next_bar.ts)
                    if next_ts <= base_ts:
                        tf_snaps = sess.step(1)
                        result[tf].extend(tf_snaps)
                        for snap in tf_snaps:
                            seg_snap = self._segment_engines[tf].process_snapshot(snap)
                            if seg_snap.events:
                                snap.events = list(snap.events) + seg_snap.events
                            zs_snap = self._zhongshu_engines[tf].process_segment_snapshot(seg_snap)
                            if zs_snap.events:
                                snap.events = list(snap.events) + zs_snap.events
                            move_snap = self._move_engines[tf].process_zhongshu_snapshot(zs_snap)
                            if move_snap.events:
                                snap.events = list(snap.events) + move_snap.events
                            bsp_snap = self._bsp_engines[tf].process_snapshots(
                                move_snap, zs_snap, seg_snap,
                            )
                            if bsp_snap.events:
                                snap.events = list(snap.events) + bsp_snap.events
                            self.bus.push(
                                tf, snap.events,
                                stream_id=tf_sid,
                            )
                    else:
                        break

        return result

    def seek(self, target_idx: int) -> dict[str, BiEngineSnapshot | None]:
        """Seek base TF 到 target_idx。

        高 TF 按时间戳对齐 seek 到对应位置。
        返回各 TF 的最终快照。
        """
        result: dict[str, BiEngineSnapshot | None] = {}

        # 重置所有 SegmentEngine / ZhongshuEngine / MoveEngine（seek 会重置 BiEngine，需同步）
        for eng in self._segment_engines.values():
            eng.reset()
        for eng in self._zhongshu_engines.values():
            eng.reset()
        for eng in self._move_engines.values():
            eng.reset()

        # base TF seek
        base_snap = self.base_session.seek(target_idx)
        result[self.base_tf] = base_snap

        if target_idx <= 0:
            for tf in self.timeframes[1:]:
                self.sessions[tf].seek(0)
                result[tf] = None
            return result

        # 计算 base TF 到达 target_idx 时的时间戳
        base_bar = self.base_session.bars[min(target_idx, self.total_bars - 1)]
        base_ts = _dt_to_epoch(base_bar.ts)

        # 高 TF：找到最后一根 close time ≤ base_ts 的 bar 索引
        for tf in self.timeframes[1:]:
            sess = self.sessions[tf]
            tf_target = -1
            for i, bar in enumerate(sess.bars):
                if _dt_to_epoch(bar.ts) <= base_ts:
                    tf_target = i
                else:
                    break

            if tf_target >= 0:
                result[tf] = sess.seek(tf_target)
            else:
                sess.seek(0)
                result[tf] = None

        return result

    def get_status(self) -> dict[str, dict]:
        """返回各 TF 的状态。"""
        return {tf: sess.get_status() for tf, sess in self.sessions.items()}
