"""持久化层（SQLite）——实盘崩溃恢复与审计。

与 Nautilus 自有持久化的分工（评估结论）：
    - Nautilus ParquetDataCatalog：**批量历史数据**（DBN→catalog，路A）。
      append 不友好（面向批写），不适合逐 bar 实时缓存。
    - Nautilus Cache + reconciliation：**订单/成交/仓位真相**——venue 是真账本
      （设计判决二），订单层恢复由 Nautilus 启动时拉 venue 报告内建完成，
      本层**不重复造**订单状态机。
    - 本层（SQLite）：Nautilus 不管的部分——
        bar_cache      已接收 K 线增量缓存（崩溃重放数据源，避免重复拉取）
        trade_journal  意图全生命周期（信号/订单/成交/撤单）+ voice 状态 + 杠杆历史
        engine_state   引擎重放锚点 + 结构快照（审计 + 重放正确性对账）

崩溃恢复协议（关键场景：BTC 持仓中崩溃→重启→不丢仓位不重复下单）：
    1. Nautilus reconciliation 拉 venue 订单/成交/仓位 → Cache 对齐（内建）
    2. engine_state.recover(bridge, bar_cache)：本地缓存全史重放 → 引擎结构重建
       （在册 R3：引擎**无状态序列化**，恢复=重放；1m 床位分钟级可接受。
        重放后与崩溃前最后快照对账 strokes/segments 计数——不一致=数据缺损，停）
    3. request_bars(start=watermark) 补崩溃窗口缺口 → 推进水位线
    4. trade_journal.voice_state ↔ Portfolio 对账（偏差超容差=人工介入，不自动纠偏）
    5. 全部通过 → 恢复意图产生
"""

from trading_system.persistence.bar_cache import BarCache
from trading_system.persistence.database import TradingDatabase
from trading_system.persistence.engine_state import EngineStateStore
from trading_system.persistence.trade_journal import TradeJournal

__all__ = ["TradingDatabase", "BarCache", "TradeJournal", "EngineStateStore"]
