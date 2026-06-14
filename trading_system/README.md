# trading_system —— NautilusTrader × 缠论引擎骨架

> 状态：阶段1骨架（信号贯通 L2 验证；下单链路冒烟；持久化+崩溃恢复 L1 验证；流式交易层/实盘为 TODO 骨架）
> 设计文档：`analysis/nautilus_integration_design.md`
> 架构：**IBKR（期货/美股/期权）+ Hyperliquid（加密永续）+ Databento（历史+期货实时）**
> NautilusTrader：pin **1.228.0**（PyPI，已装入 `.venv`；克隆的 v1.229.0 源码仅作 API 参考）

## 运行

```bash
# 阶段1：信号贯通（只读，不下单）—— 默认 BZ 1min 2024 数据前 3 万根
.venv/bin/python trading_system/backtest/runner.py

# 阶段3形态冒烟：LMT-only + maker 60s 撤单 + L_max 杠杆钳制 + 保守撮合 + SQLite 持久化
.venv/bin/python trading_system/backtest/runner.py --enable-orders --db /tmp/chanlun_bt.db

# 实盘入口（阶段4骨架，打印配置诊断）
.venv/bin/python trading_system/live/runner.py

# 守卫测试（17 项）
.venv/bin/python -m pytest tests/test_trading_system_skeleton.py -q
```

## 已验证（2026-06-12，BZ 1min 2024）

| 验证项 | 结果 |
|---|---|
| 回测跑通 / Strategy 接收 bar / PyO3 引擎产信号 | ✅ 30000 bar → 2096笔/212段/44中枢/14走势，68 confirmed BSP 信号 |
| LMT-only + maker 状态机 | ✅ fills=2 cancels=60 rejects=0（60s 撤单在回测中可观测） |
| 保守撮合口径 | ✅ FillModel(prob_fill_on_limit=0.0) 钉死（设计判决一） |
| SQLite 持久化 | ✅ bars=30000 / signals=68 / orders=62（60撤+2成，与 maker 计数器逐项一致）/ 快照 1 |
| **崩溃恢复** | ✅ 新进程从 BarCache 重放 30000 根 → 结构与崩溃前快照逐项一致，守卫未抛 |
| Hyperliquid adapter | ✅ 1.228.0 官方内置，testnet 配置类构造通过 |
| **Databento 历史→catalog（路A）** | ✅ ES 1min 一个月（30,120 bar）+ definition 去重 → ParquetDataCatalog（`.cache/nautilus_catalog`，instrument `ESM6.GLBX` 精度2） |
| **BacktestNode 消费 catalog** | ✅ `catalog_smoke.py`：30,120/30,120 bar 送达 `BarCounter.on_bar()` |
| **Databento Live 接入** | ✅ gateway 认证 + ESM6/CLN6 definition 加载 + 双路 1min 订阅接受；on_bar 收数因 CME 周末闭市（周五17:00 ET后运行）待开盘时段复验：`live/databento_live_runner.py --duration 180` |

## 结构

```
trading_system/
├── strategy/
│   ├── chanlun_strategy.py    # ChanlunStrategy(Strategy)：生命周期壳 + 持久化挂钩
│   └── signal_bridge.py       # ChanlunBridge：唯一时间映射点（feed/feed_ohlc 同一守卫路径）
├── execution/
│   ├── lmt_executor.py        # LMT-only（非LIMIT订单 submit 前抛 MarketOrderForbidden）
│   ├── maker_optimizer.py     # 挂单→≤60s等候→撤单不追价（在册 maker 判决实装）
│   └── leverage_calculator.py # L_max = 1/(D_struct+mm)，纯函数逐bar，含合约乘数
├── data/
│   ├── DATA_SOURCES.md        # 实时数据源选型评估（HL/Databento/IBKR/AV 四源矩阵）
│   ├── feed_abstraction.py    # 统一数据抽象：域→源路由 + 能力声明（防声明膨胀）
│   ├── databento_loader.py    # 路A: DBN→catalog（委托 databento_catalog）/ 路B: parquet数组→Bar
│   ├── databento_catalog.py   # 路A CLI: API拉取(费用守卫)→DBN缓存→definition去重→catalog→验证
│   └── bar_aggregator.py      # INTERNAL 1s 聚合 BarType（HL 无交易所 1s K线判决）
├── persistence/               # SQLite：崩溃恢复 + 审计（与 Nautilus 自有持久化分工见 __init__）
│   ├── database.py            # 连接管理 + schema（WAL）
│   ├── bar_cache.py           # K线增量缓存 = 崩溃重放数据源（主键去重）
│   ├── trade_journal.py       # 信号/订单/成交/撤单 + voice 状态 + 杠杆历史
│   └── engine_state.py        # 重放锚点 + 结构快照对账守卫（RecoveryMismatch fail-fast）
├── config/
│   ├── instruments.py         # BTC-USD-PERP.HYPERLIQUID / CL / BZ 注册表
│   └── broker_config.py       # IBKR paper / Hyperliquid testnet / Databento（环境变量）
├── strategy/bar_counter.py    # 数据管线冒烟空策略（回测/实盘共用，只计数不下单）
├── backtest/runner.py         # BacktestEngine 入口（--db 开持久化）
├── backtest/catalog_smoke.py  # BacktestNode × DBN catalog 消费端验证
├── live/runner.py             # TradingNode 骨架（阶段4）
└── live/databento_live_runner.py  # Databento Live 数据流冒烟（数据-only node，定时停机）
```

## Hyperliquid 调研结论（adapter 源码逐字核对）

- **官方 adapter 已内置 1.228.0**（`adapters/hyperliquid/`，pyo3 Rust 实现）——**不需要自建**。
- instrument_id：`BTC-USD-PERP.HYPERLIQUID`；testnet 经 `HyperliquidEnvironment.TESTNET`；
  私钥走 `HYPERLIQUID_PK` / `HYPERLIQUID_TESTNET_PK` 环境变量（adapter 自读，代码零接触）。
- **交易所侧 K 线最小 1m，无 1s**（Rust `bar_type_to_interval` 白名单核对）
  ⟹ 1s 床位 = `subscribe_trade_ticks` → Nautilus INTERNAL 聚合（与原 Binance 判决同型，R5 对账待 L2）。
- post_only（HL ALO 单）支持，"would match" 拒单被 adapter 显式处理
  ⟹ maker 纪律可在交易所级强制。
- HL 杠杆 per-position 下单时指定（非账户级预设）⟹ 恒仓 1x 约束在 LeverageGovernor 层钳制。

## 数据源决议（详见 data/DATA_SOURCES.md）

| 域 | 实时 | 历史 | 备选 |
|---|---|---|---|
| 加密 | Hyperliquid WS（零额外成本） | HL candleSnapshot / 本仓库归档 | — |
| 期货 | Databento Live（费率过门后）| Databento（已有订阅） | IBKR（免费，1m 床位够用） |
| 美股 | IBKR | Databento XNAS.ITCH | Alpha Vantage（仅历史，无推送不进实时链路） |

## 持久化与崩溃恢复（关键场景已演练）

**严格声明**：引擎无状态序列化（在册 R3）——"恢复引擎状态"的严格形式是
**BarCache 本地全史重放 + 重放后与崩溃前快照对账**（不一致 → `RecoveryMismatch`
fail-fast，禁止带错误结构交易），不是结构反序列化（那是独立 Rust checkpoint 工程项）。

与 Nautilus 自有持久化的分工：ParquetDataCatalog 管批量历史（路A）；订单/仓位真相
由 Nautilus reconciliation 从 venue 拉取（判决二：venue 是真账本）；SQLite 管
Nautilus 不管的部分（引擎重放源、voice 影子账本、信号/杠杆审计日志）。

恢复序列（live/runner.py 文档化）：reconciliation → BarCache 重放 → request_bars
补缺口 → voice_state↔Portfolio 对账 → 全部通过才恢复意图产生。

## 三个钉死的判决（设计 §0，代码中已体现）

1. **保守撮合**：`prob_fill_on_limit=0.0`；乐观口径只作对照臂，不可混表。
2. **venue 是真账本**：OrganicLedger 降级为意图状态机（`confirm_fill`/`reject_intent`
   接入点 TODO 在 chanlun_strategy.py；voice_state 持久化出口在 trade_journal.py）。
3. **时间映射全住桥接层**：`feed`/`feed_ohlc` 同一守卫路径——重放与实时无第二套时间逻辑。

## 下一步（按设计文档阶段门）

- **阶段2（最大工程项）**：Rust `PositionalStream` 流式意图模式，批↔流 bit-exact 预注册守卫。
- **阶段0 残项**：DBN 重下载 → ParquetDataCatalog。
- **阶段4**：TradingNode + IBKR paper + **Hyperliquid testnet**（水龙头领测试金，无 KYC）
  + 预热协议 + 两级对账；HL INTERNAL 1s ↔ EXTERNAL 1m 对账（R5）。
- **待 L2 收口**：Databento Live 费率、HL WS 实测延迟/重连、IBKR 快照口径影响。

## 已知边界

- 骨架信号映射（confirmed BSP→BUY/SELL）只验证管线，在册 alpha 在 fusion_tr（阶段2）。
- 回测 instrument 本地装配（HL 费率/精度为 L0 文档值），与实盘 adapter definitions 不可混表。
- 1s 床位崩溃恢复重放成本不可接受（R3）——checkpoint 序列化立项前 1s 床位不上实盘。
- **安全**：编排者消息中出现过明文 Databento API key，应视为已暴露，建议轮换；
  代码层全部走环境变量，仓库零落盘。
