"""EngineStateStore —— 引擎重放锚点 + 结构快照（审计/对账）。

## 严格声明（不做声明膨胀）

引擎（RecursiveOrchestrator）是路径依赖状态机且**无状态序列化能力**
（在册风险 R3）。因此本模块提供的"恢复"严格地是：

    恢复 = BarCache 本地全史重放 + 重放后与崩溃前快照对账

不是结构反序列化。结构快照（strokes/segments/zhongshus/moves 计数）的用途：
    1. 审计：每日收盘/停机时的结构状态留痕
    2. **重放正确性守卫**：恢复重放到崩溃前 watermark 后，结构计数必须与
       最后快照一致——不一致 = bar 缓存缺损或引擎版本漂移，fail-fast 停机，
       不带着错误结构进入交易。

成本（在册）：1m 床位全史重放分钟级，可接受；1s 床位不可接受——
真正的 checkpoint 序列化是独立 Rust 工程项（结构快照落盘已是其审计前置）。
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from trading_system.persistence.bar_cache import BarCache
from trading_system.persistence.database import TradingDatabase

if TYPE_CHECKING:  # 运行时不导入 strategy 包——避免 persistence↔strategy 循环
    from trading_system.strategy.signal_bridge import ChanlunBridge


class RecoveryMismatch(RuntimeError):
    """重放后结构与崩溃前快照不一致——数据缺损或引擎漂移，禁止继续交易。"""


class EngineStateStore:
    def __init__(self, db: TradingDatabase) -> None:
        self._db = db

    # ── 快照写入 ────────────────────────────────────────────────

    def save_snapshot(self, instrument_id: str, bridge: ChanlunBridge) -> None:
        """落盘当前结构计数 + watermark（每日收盘/停机/定时调用）。"""
        snap = bridge.structure_snapshot()
        self._db.conn.execute(
            "INSERT INTO engine_snapshots "
            "(ts_event_ns, instrument_id, bar_count, watermark_ns, strokes, segments, zhongshus, moves) "
            "VALUES (?,?,?,?,?,?,?,?)",
            (bridge.watermark_ns or 0, instrument_id, bridge.bar_count,
             bridge.watermark_ns or 0, snap["strokes"], snap["segments"],
             snap["zhongshus"], snap["moves"]),
        )
        self._db.commit()

    def last_snapshot(self, instrument_id: str) -> dict | None:
        row = self._db.conn.execute(
            "SELECT ts_event_ns, bar_count, watermark_ns, strokes, segments, zhongshus, moves "
            "FROM engine_snapshots WHERE instrument_id=? ORDER BY id DESC LIMIT 1",
            (instrument_id,),
        ).fetchone()
        if row is None:
            return None
        keys = ("ts_event_ns", "bar_count", "watermark_ns", "strokes",
                "segments", "zhongshus", "moves")
        return dict(zip(keys, row))

    # ── 恢复（重放 + 对账）──────────────────────────────────────

    def recover(
        self,
        bridge: ChanlunBridge,
        bar_cache: BarCache,
        instrument_id: str,
        verify_against_snapshot: bool = True,
    ) -> int:
        """崩溃恢复：缓存全史重放进（新建的）bridge，对账崩溃前快照。

        返回重放 bar 数。bridge 必须是新建实例（已喂过数据的 bridge 重放
        会触发 watermark 倒退 fail-fast——这是守卫不是缺陷）。

        重放完成后调用方继续：request_bars(start=bridge.watermark_ns) 补
        崩溃窗口的缺口 → 影子账本对账 → 才放开下单（见 persistence/__init__）。
        """
        n = 0
        for ts_ns, o, h, l, c, _v in bar_cache.iter_bars(instrument_id):
            bridge.feed_ohlc(ts_ns, o, h, l, c)
            n += 1

        if verify_against_snapshot:
            snap = self.last_snapshot(instrument_id)
            if snap is not None:
                self._verify(bridge, snap, instrument_id)
        return n

    def _verify(self, bridge: ChanlunBridge, snap: dict, instrument_id: str) -> None:
        """重放正确性守卫：到崩溃前 watermark 为止的结构必须逐项一致。

        注意：缓存可能含崩溃前快照**之后**的 bar（快照非每 bar 落盘），
        此时结构计数允许 ≥ 快照值但 bar_count/watermark 必须 ≥；
        严格相等校验只在 watermark 精确对齐时执行。
        """
        if bridge.watermark_ns == snap["watermark_ns"]:
            current = bridge.structure_snapshot()
            expected = {k: snap[k] for k in ("strokes", "segments", "zhongshus", "moves")}
            if current != expected or bridge.bar_count != snap["bar_count"]:
                raise RecoveryMismatch(
                    f"{instrument_id} 重放结构与快照不一致: "
                    f"replayed={current} bars={bridge.bar_count} "
                    f"snapshot={expected} bars={snap['bar_count']}"
                    "——bar 缓存缺损或引擎版本漂移，禁止继续交易"
                )
        elif (bridge.watermark_ns or 0) < snap["watermark_ns"]:
            raise RecoveryMismatch(
                f"{instrument_id} 缓存水位线 {bridge.watermark_ns} 落后于快照 "
                f"{snap['watermark_ns']}——bar 缓存缺损，禁止继续交易"
            )
        # watermark 超过快照（快照后又收过 bar）：无法逐项对账，仅水位线检查通过
