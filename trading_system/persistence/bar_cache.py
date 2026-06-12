"""BarCache —— 已接收 K 线增量缓存（崩溃重放数据源）。

与 ParquetDataCatalog 的分工：catalog 面向批量历史（路A 落盘），
本缓存面向**实时增量逐 bar 追加 + 主键去重**——崩溃恢复时引擎全史重放
从这里读，不重复拉外部数据源。

批量提交：逐 bar INSERT 但每 commit_interval 根才 commit 一次
（WAL 下单条 commit ~ms 级，1s 床位逐根 commit 会吃掉预算）。
"""

from __future__ import annotations

from collections.abc import Iterator

from trading_system.persistence.database import TradingDatabase


class BarCache:
    def __init__(self, db: TradingDatabase, commit_interval: int = 500) -> None:
        self._db = db
        self._commit_interval = commit_interval
        self._pending = 0

    def append(
        self,
        instrument_id: str,
        ts_event_ns: int,
        open_: float,
        high: float,
        low: float,
        close: float,
        volume: float = 0.0,
    ) -> None:
        """追加一根 bar。重复 ts（主键冲突）静默忽略——与桥接层 DUPLICATE 语义一致，
        重放/实时重叠窗口天然去重。"""
        self._db.conn.execute(
            "INSERT OR IGNORE INTO bars VALUES (?,?,?,?,?,?,?)",
            (instrument_id, ts_event_ns, open_, high, low, close, volume),
        )
        self._pending += 1
        if self._pending >= self._commit_interval:
            self._db.commit()
            self._pending = 0

    def flush(self) -> None:
        self._db.commit()
        self._pending = 0

    def iter_bars(
        self, instrument_id: str, start_ns: int | None = None, end_ns: int | None = None,
    ) -> Iterator[tuple[int, float, float, float, float, float]]:
        """按 ts 升序迭代 (ts_event_ns, o, h, l, c, v)——重放消费接口。"""
        sql = "SELECT ts_event_ns, open, high, low, close, volume FROM bars WHERE instrument_id=?"
        params: list = [instrument_id]
        if start_ns is not None:
            sql += " AND ts_event_ns >= ?"
            params.append(start_ns)
        if end_ns is not None:
            sql += " AND ts_event_ns <= ?"
            params.append(end_ns)
        sql += " ORDER BY ts_event_ns ASC"
        yield from self._db.conn.execute(sql, params)

    def last_ts(self, instrument_id: str) -> int | None:
        """缓存水位线（崩溃后 request_bars(start=...) 补缺口的起点）。"""
        row = self._db.conn.execute(
            "SELECT MAX(ts_event_ns) FROM bars WHERE instrument_id=?", (instrument_id,),
        ).fetchone()
        return row[0]

    def count(self, instrument_id: str) -> int:
        row = self._db.conn.execute(
            "SELECT COUNT(*) FROM bars WHERE instrument_id=?", (instrument_id,),
        ).fetchone()
        return int(row[0])
