"""TradeJournal —— 意图全生命周期日志 + voice 状态 + 杠杆历史。

审计原则（设计 §7.4）：每个意图 intent → order → fill/cancel 链路落盘可回溯；
回测/实盘同一 schema——审计工具一套两用。

voice_state 的存在论位置（设计判决二）：它是**影子账本（意图状态机）的持久化**，
回答"按缠论结构应该持有什么"；venue 仓位真相在 Nautilus Cache/Portfolio。
恢复时两者对账（偏差超容差=人工介入），不是谁覆盖谁。
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from trading_system.persistence.database import TradingDatabase

if TYPE_CHECKING:  # 运行时不导入 strategy 包——避免 persistence↔strategy 循环
    from trading_system.strategy.signal_bridge import ChanlunSignal


class TradeJournal:
    def __init__(self, db: TradingDatabase) -> None:
        self._db = db

    # ── 信号 ────────────────────────────────────────────────────

    def log_signal(self, instrument_id: str, sig: ChanlunSignal) -> None:
        self._db.conn.execute(
            "INSERT INTO signals "
            "(ts_event_ns, instrument_id, action, level, kind, price, notional_frac, bar_index, reason) "
            "VALUES (?,?,?,?,?,?,?,?,?)",
            (sig.ts_event_ns, instrument_id, sig.action, sig.level, sig.kind,
             sig.price, sig.notional_frac, sig.bar_index, sig.reason),
        )

    # ── 订单生命周期 ────────────────────────────────────────────

    def log_order_placed(
        self, client_order_id: str, ts_ns: int, instrument_id: str,
        side: str, quantity: float, limit_price: float, reason: str = "",
    ) -> None:
        self._db.conn.execute(
            "INSERT OR REPLACE INTO orders VALUES (?,?,?,?,?,?,'PLACED',?)",
            (client_order_id, ts_ns, instrument_id, side, quantity, limit_price, reason),
        )

    def log_fill(self, client_order_id: str, ts_ns: int, fill_price: float, fill_qty: float) -> None:
        self._db.conn.execute(
            "INSERT INTO fills (client_order_id, ts_ns, fill_price, fill_qty) VALUES (?,?,?,?)",
            (client_order_id, ts_ns, fill_price, fill_qty),
        )
        self._set_order_status(client_order_id, "FILLED")

    def log_cancel(self, client_order_id: str) -> None:
        self._set_order_status(client_order_id, "CANCELED")

    def log_reject(self, client_order_id: str) -> None:
        self._set_order_status(client_order_id, "REJECTED")

    def _set_order_status(self, client_order_id: str, status: str) -> None:
        self._db.conn.execute(
            "UPDATE orders SET status=? WHERE client_order_id=?", (status, client_order_id),
        )

    # ── voice 状态（影子账本持久化）─────────────────────────────

    def upsert_voice_state(
        self, instrument_id: str, voice_key: str, position_qty: float,
        cost_basis: float, notional_frac: float, operating_level: int, ts_ns: int,
    ) -> None:
        """TODO(阶段2): PositionalStream 接入后由 confirm_fill 回调驱动——
        影子账本只在成交确认时更新，本方法是其持久化出口。"""
        self._db.conn.execute(
            "INSERT OR REPLACE INTO voice_state VALUES (?,?,?,?,?,?,?)",
            (instrument_id, voice_key, position_qty, cost_basis,
             notional_frac, operating_level, ts_ns),
        )

    def load_voice_states(self, instrument_id: str) -> list[dict]:
        rows = self._db.conn.execute(
            "SELECT voice_key, position_qty, cost_basis, notional_frac, operating_level, updated_ts_ns "
            "FROM voice_state WHERE instrument_id=?",
            (instrument_id,),
        ).fetchall()
        keys = ("voice_key", "position_qty", "cost_basis", "notional_frac",
                "operating_level", "updated_ts_ns")
        return [dict(zip(keys, r)) for r in rows]

    def total_position(self, instrument_id: str) -> float:
        """影子账本净持仓——恢复时与 Portfolio.net_position 对账的左边。"""
        row = self._db.conn.execute(
            "SELECT COALESCE(SUM(position_qty), 0) FROM voice_state WHERE instrument_id=?",
            (instrument_id,),
        ).fetchone()
        return float(row[0])

    # ── 杠杆历史 ────────────────────────────────────────────────

    def log_leverage(
        self, ts_event_ns: int, instrument_id: str, price: float,
        d_struct: float, l_max: float,
    ) -> None:
        self._db.conn.execute(
            "INSERT INTO leverage_log (ts_event_ns, instrument_id, price, d_struct, l_max) "
            "VALUES (?,?,?,?,?)",
            (ts_event_ns, instrument_id, price, d_struct, l_max),
        )

    def commit(self) -> None:
        self._db.commit()
