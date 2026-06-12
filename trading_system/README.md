# trading_system —— NautilusTrader × 缠论引擎骨架

> 状态：阶段1骨架（信号贯通已验证 L2；下单链路冒烟通过；流式交易层/实盘为 TODO 骨架）
> 设计文档：`analysis/nautilus_integration_design.md`
> NautilusTrader：pin **1.228.0**（PyPI，已装入 `.venv`；克隆的 v1.229.0 源码仅作 API 参考）

## 运行

```bash
# 阶段1：信号贯通（只读，不下单）—— 默认 BZ 1min 2024 数据前 3 万根
.venv/bin/python trading_system/backtest/runner.py

# 全量 51 万根
.venv/bin/python trading_system/backtest/runner.py --bars 0

# 阶段3形态冒烟：LMT-only + maker 60s 撤单 + L_max 杠杆钳制 + 保守撮合
.venv/bin/python trading_system/backtest/runner.py --enable-orders

# 实盘入口（阶段4骨架，当前只打印配置）
.venv/bin/python trading_system/live/runner.py
```

## 已验证（2026-06-12，BZ 1min 2024）

| 验证项 | 结果 |
|---|---|
| 回测跑通 | ✅ 30000 bar 无异常 |
| ChanlunStrategy 接收 bar | ✅ bars 喂入引擎 = 30000 |
| Rust 引擎（PyO3）产出信号 | ✅ 2096笔/212段/44中枢/14走势，68 个 confirmed BSP 信号 |
| LMT-only + maker 状态机 | ✅ fills=2 cancels=60（60s 撤单在回测中可观测）rejects=0 |
| 保守撮合口径 | ✅ FillModel(prob_fill_on_limit=0.0) 钉死（设计判决一） |

## 结构

```
trading_system/
├── strategy/
│   ├── chanlun_strategy.py    # ChanlunStrategy(Strategy)：生命周期壳，on_bar→引擎→信号→订单
│   └── signal_bridge.py       # ChanlunBridge：唯一时间映射点（watermark/gap守卫/f64边界）
├── execution/
│   ├── lmt_executor.py        # LMT-only（非LIMIT订单 submit 前抛 MarketOrderForbidden）
│   ├── maker_optimizer.py     # 挂单→≤60s等候→撤单不追价（在册 maker 判决实装）
│   └── leverage_calculator.py # L_max = 1/(D_struct+mm)，纯函数逐bar，含合约乘数
├── data/
│   ├── databento_loader.py    # 路A: DBN→catalog(TODO) / 路B: parquet数组→Bar(已实现)
│   └── bar_aggregator.py      # INTERNAL 1s 聚合 BarType（Binance futures 秒级判决）
├── config/
│   ├── instruments.py         # BTC perp / CL / BZ 注册表 + Nautilus instrument 构造
│   └── broker_config.py       # IBKR paper / Binance testnet（环境变量，不硬编码）
├── backtest/runner.py         # BacktestEngine 入口（本骨架的验证标准）
└── live/runner.py             # TradingNode 骨架（阶段4）
```

## 三个钉死的判决（来自设计文档 §0，代码中已体现）

1. **保守撮合**：`prob_fill_on_limit=0.0`（队列末位穿越才成交）；乐观口径只作对照臂，不可混表。
2. **venue 是真账本**：OrganicLedger 降级为意图状态机（阶段2 `PositionalStream` 接入点已在
   `chanlun_strategy.py` 的 TODO 标注——`confirm_fill`/`reject_intent`）。
3. **时间映射全住桥接层**：`ChanlunBridge.feed()` 是唯一的 ts↔bar_index 翻译点；
   ts 倒退 fail-fast，gap 只记录不填充。

## 下一步（按设计文档阶段门）

- **阶段2（最大工程项）**：Rust `PositionalStream` 流式意图模式（`rust/src/trading/stream.rs`），
  批↔流 bit-exact 预注册守卫；接入后 `signal_bridge.drain_signals()` 的 BSP 直读映射退役，
  由 fusion_tr voice 状态机产出意图。
- **阶段0 残项**：DBN 重下载 → ParquetDataCatalog（`databento_loader.load_dbn_to_catalog`）。
- **阶段4**：TradingNode + IBKR paper + Binance testnet + 预热协议（§2.3）+ 两级对账。

## 已知边界

- 骨架的信号映射（confirmed BSP → BUY/SELL，notional_frac=1.0 占位）**不是**在册策略——
  它只验证管线，在册 alpha 在 fusion_tr voice 状态机里（阶段2 接入）。
- 回测 instrument 是本地装配（`make_instrument`），与实盘 adapter 拉取的 definitions
  不可混表；连续合约↔主力月映射（R7）阶段4 处理。
- `Strategy.on_reset` 未覆写的 WARN 是已知噪声（engine.reset() 时出现，骨架无状态可重置）。
