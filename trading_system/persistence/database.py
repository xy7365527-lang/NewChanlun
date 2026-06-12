"""TradingDatabase —— SQLite 连接管理与 schema。

选型依据：轻量、零配置、单文件、Python 内置；WAL 模式支持
读写并发（实盘主循环写 + 审计工具读不互斥）。
"""

from __future__ import annotations

import sqlite3
from pathlib import Path

_SCHEMA = """
CREATE TABLE IF NOT EXISTS bars (
    instrument_id TEXT NOT NULL,
    ts_event_ns   INTEGER NOT NULL,
    open REAL NOT NULL, high REAL NOT NULL, low REAL NOT NULL, close REAL NOT NULL,
    volume REAL NOT NULL DEFAULT 0,
    PRIMARY KEY (instrument_id, ts_event_ns)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS signals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_event_ns INTEGER NOT NULL,
    instrument_id TEXT NOT NULL,
    action TEXT NOT NULL,            -- BUY/SELL/HOLD
    level INTEGER NOT NULL,
    kind TEXT NOT NULL,              -- type1/type2/type3
    price REAL NOT NULL,
    notional_frac REAL NOT NULL,
    bar_index INTEGER NOT NULL,
    reason TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_signals_ts ON signals (instrument_id, ts_event_ns);

CREATE TABLE IF NOT EXISTS orders (
    client_order_id TEXT PRIMARY KEY,
    ts_ns INTEGER NOT NULL,
    instrument_id TEXT NOT NULL,
    side TEXT NOT NULL,
    quantity REAL NOT NULL,
    limit_price REAL NOT NULL,
    status TEXT NOT NULL,            -- PLACED/FILLED/CANCELED/REJECTED/PARTIAL
    reason TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS fills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_order_id TEXT NOT NULL,
    ts_ns INTEGER NOT NULL,
    fill_price REAL NOT NULL,
    fill_qty REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS voice_state (
    instrument_id TEXT NOT NULL,
    voice_key TEXT NOT NULL,         -- "(ladder,slot)" 或 mode 内部键
    position_qty REAL NOT NULL,
    cost_basis REAL NOT NULL,
    notional_frac REAL NOT NULL,
    operating_level INTEGER NOT NULL,
    updated_ts_ns INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, voice_key)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS leverage_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_event_ns INTEGER NOT NULL,
    instrument_id TEXT NOT NULL,
    price REAL NOT NULL,
    d_struct REAL NOT NULL,
    l_max REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_leverage_ts ON leverage_log (instrument_id, ts_event_ns);

CREATE TABLE IF NOT EXISTS engine_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_event_ns INTEGER NOT NULL,
    instrument_id TEXT NOT NULL,
    bar_count INTEGER NOT NULL,
    watermark_ns INTEGER NOT NULL,
    strokes INTEGER NOT NULL,
    segments INTEGER NOT NULL,
    zhongshus INTEGER NOT NULL,
    moves INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
"""


class TradingDatabase:
    """单文件 SQLite 库。一个实例 = 一个 trader 部署的全部持久状态。"""

    def __init__(self, path: str | Path) -> None:
        self._path = Path(path)
        self._path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(str(self._path))
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA synchronous=NORMAL")  # WAL 下崩溃安全且快
        self._conn.executescript(_SCHEMA)
        self._conn.commit()

    @property
    def conn(self) -> sqlite3.Connection:
        return self._conn

    @property
    def path(self) -> Path:
        return self._path

    def commit(self) -> None:
        self._conn.commit()

    def close(self) -> None:
        self._conn.commit()
        self._conn.close()

    def __enter__(self) -> TradingDatabase:
        return self

    def __exit__(self, *exc) -> None:
        self.close()
