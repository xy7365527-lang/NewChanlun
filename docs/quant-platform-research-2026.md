# 量化交易平台调研报告：Rust缠论引擎集成方案

> 调研日期：2026-06-12
> 核心约束：1s K线作为a0 / 纯LMT挂单 / 全市场覆盖 / Rust引擎直接集成

---

## 零、执行摘要

**推荐方案**：两层架构 —— NautilusTrader（Rust核心 + IBKR adapter）覆盖传统市场，Binance/OKX WebSocket直连覆盖加密合约。Databento作为统一历史数据源。

**核心理由**：

1. NautilusTrader是唯一Rust-native的生产级量化框架，与现有Rust缠论引擎天然兼容
2. IBKR单一账户覆盖美股/期货/期权/港股，NautilusTrader已有完整IBKR adapter
3. Databento提供免费的1s OHLCV历史数据（CME回溯至2010），且与NautilusTrader有一级集成
4. 加密合约通过Binance/OKX WebSocket获取原生1s K线，NautilusTrader同样有Binance adapter

**最小平台组合**：

| 层级 | 组件 | 覆盖 |
|------|------|------|
| 信号引擎 | Rust缠论引擎（现有） | 全标的通用 |
| 执行框架 | NautilusTrader | 统一回测/实盘，事件驱动 |
| 传统市场broker | IBKR（现有账户） | 美股/期货/期权/港股 |
| 加密合约 | Binance Futures + OKX | BTC及主流币永续合约 |
| 历史数据 | Databento（现有订阅） | 期货/美股 1s OHLCV |
| 实时数据 | Databento Live + 交易所WebSocket | 全标的 |
| 可视化 | TradingView MCP（现有） | 图表+缠论指标叠加 |

---

## 一、1秒K线（a0）数据能力评估

### 1.1 为什么1s是硬约束

系统已验证：1s K线作为a0可以产生+1层递归深度（lid4涌现），CL 1s数据5.96M bars/年引擎4.2秒处理完。1s不仅是更细的交易粒度，更是为相位机提供更高分辨率的结构信号。maker执行模型下1s在可行域内。

**因此：不支持1s级实时数据的平台直接排除。**

### 1.2 各数据源1s能力对照

| 数据源 | 原生1s K线 | Tick级数据 | 历史1s深度 | 实时推送频率 | 适用标的 |
|--------|-----------|-----------|-----------|------------|---------|
| **Databento** | YES（ohlcv-1s） | YES（trades/mbp-1） | CME回溯至2010 | Live API实时 | 期货/美股 |
| **IBKR reqTickByTickData** | NO（需自聚合） | YES（Last/BidAsk/AllLast） | 1s历史可查但pacing严格 | 逐tick推送 | 全标的 |
| **IBKR reqRealTimeBars** | NO（最小5s） | — | — | 5s | 全标的 |
| **Binance Futures WS** | YES（kline_1s） | YES（aggTrade） | REST可查 | 250ms更新 | 加密合约 |
| **OKX WS** | YES（candle1s） | YES（trades） | REST可查 | ~1s | 加密合约 |
| **Bybit WS** | NO（最小1m） | YES（trades） | — | 1-60s | 加密合约 |
| **Polygon.io** | YES（A.*流） | YES | 历史可查 | 逐秒推送 | 期货/美股 |
| **TradingView** | 有限（5000-20000 bars） | — | 极浅（~1.4-5.5小时） | 实时 | 展示用 |

### 1.3 IBKR Tick数据详解

**reqMktData**：250ms聚合快照，top-of-book报价+最新价。适合一般行情监控，不适合构建精确1s bar。

**reqTickByTickData**：逐tick推送，三种模式：
- `Last`：成交tick（价格、数量、交易所）
- `BidAsk`：报价tick（bid/ask价格和数量）
- `AllLast`：含盘后交易的所有成交

**关键限制**：
- 历史1s bar请求：每次最多1800秒（30分钟），每10分钟最多60次请求
- 实际吞吐：约3个交易日的1s数据/10分钟pacing窗口
- 同时订阅数受market data lines限制（默认100条）

**结论**：IBKR适合实时tick→1s聚合，但历史回填效率极低。历史数据走Databento。

### 1.4 Databento 1s数据架构

**Schema选择**：`ohlcv-1s` —— 一级支持，非衍生产品。

**关键优势**：
- OHLCV-1s和OHLCV-1m历史数据**免费**（unlimited access）
- 纳秒精度时间戳
- CME Globex数据回溯至2010年6月
- 数据量估算：CL 5.96M bars/年 × ~64 bytes/record ≈ 380 MB/年（极易管理）
- Rust SDK（`databento` crate）基于tokio异步，历史order book回放达1900万事件/秒

**Intraday Replay（消除拼接gap）**：单次API调用中无缝衔接历史回放与实时流，24小时intraday历史窗口。

### 1.5 加密交易所1s数据

**Binance Futures**（推荐首选）：
- 订阅格式：`btcusdt@kline_1s`
- 推送频率：250ms更新一次（每根1s K线推送4次中间状态）
- 连续合约：`btcusdt_perpetual@continuousKline_1s`
- aggTrade流可用于自聚合验证

**OKX**：
- WebSocket支持`candle1s`频道
- 支持USDT-M/USDC-M/Crypto-M永续和交割合约

**Bybit**：最小K线1分钟，需从trade流自聚合1s bar。增加复杂度，不推荐作为1s数据主源。

### 1.6 数据拼接架构

```
┌─────────────────────────────────────────────────────────┐
│                    历史回测路径                           │
│  Databento .dbn.zst (ohlcv-1s) → Parquet catalog        │
│  → NautilusTrader BacktestEngine (DataFusion流式读取)    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                    实时交易路径                           │
│                                                         │
│  期货/美股:                                              │
│    Databento Live API (intraday replay消除启动gap)       │
│    → NautilusTrader DataEngine → 1s bar聚合             │
│                                                         │
│  加密合约:                                               │
│    Binance WS kline_1s / OKX WS candle1s                │
│    → Ring Buffer (lock-free) → 1s bar归一化             │
│                                                         │
│  统一输出:                                               │
│    所有1s bars → Rust缠论引擎 → 买卖点信号              │
│    → NautilusTrader Strategy → IBKR/交易所下单          │
└─────────────────────────────────────────────────────────┘
```

**Gap处理原则**：
- 非交易时段gap：预期行为，不填充
- 交易时段gap：Databento intraday replay回填（期货），标记`gap=True`传递给引擎（引擎处理笔/段中断）
- **绝不合成数据填充gap**

**时间戳对齐**：
- 统一为UTC纳秒（Databento原生格式）
- IBKR时间戳从交易所本地时间转换
- 加密交易所毫秒→纳秒扩展

---

## 二、平台全景对照表

### 2.1 执行框架对照

| 平台 | 语言 | 1s数据支持 | IBKR集成 | 加密合约 | Rust集成度 | 实盘交易 | 全市场覆盖 | 推荐度 |
|------|------|-----------|---------|---------|-----------|---------|-----------|-------|
| **NautilusTrader** | Rust+Python | 原生（BarSpec 1s） | 完整adapter | Binance/Bybit adapter | **极高**（Rust core） | 完整 | 是 | ★★★★★ |
| **rust-ibapi直连** | Rust | 自聚合 | 原生crate | 需另接 | **极高**（纯Rust） | 完整 | 仅IBKR标的 | ★★★★ |
| **QuantConnect Lean** | C#/Python | 支持（tick级） | 完整 | 支持 | 低（HTTP/TCP外部信号） | 完整 | 是 | ★★★ |
| **Alpaca** | REST API | 支持 | 不适用（自身是broker） | 美股+加密 | 高（HTTP直调） | 完整 | 无期货 | ★★ |
| **Backtrader** | Python | 自聚合 | 有 | 无 | 中 | 有 | 有限 | ★★ |
| **vn.py (VeighNa)** | Python | 自聚合 | 有（vnpy_ib） | 有（vnpy_binance） | 低 | 完整 | 是 | ★★ |
| **Zipline-reloaded** | Python | 无 | 无原生 | 无 | 低 | 无 | 无 | ★ |
| **VectorBT** | Python | 无 | 无 | 无 | 低 | 无 | 无 | ★ |
| **Freqtrade** | Python | 自聚合 | 无 | 仅加密 | 低 | 有 | 无 | ★ |

### 2.2 数据源对照

| 数据源 | 期货 | 美股 | 加密 | 期权 | 港股 | 1s原生 | 历史深度 | 成本 |
|--------|------|------|------|------|------|--------|---------|------|
| **Databento** | CL/BZ/ES/GC/DX ✓ | ✓ | ✗ | CME期权 ✓ | ✗ | ohlcv-1s免费 | 2010至今 | Live CME $179/月 |
| **IBKR** | ✓ | ✓ | 有限 | ✓ | ✓ | 需自聚合 | pacing受限 | 行情订阅费 |
| **Binance** | ✗ | ✗ | ✓ | ✗ | ✗ | kline_1s | 数月 | 免费 |
| **OKX** | ✗ | ✗ | ✓ | ✗ | ✗ | candle1s | 数月 | 免费 |
| **Polygon.io** | ✓ | ✓ | ✓ | ✓ | ✗ | A.*流 | 完整 | $99-$399/月 |
| **TradingView** | 展示用 | 展示用 | 展示用 | 展示用 | 展示用 | 有限 | 极浅 | 订阅费 |

### 2.3 Broker能力对照

| Broker | 期货 | 美股 | 加密 | 期权 | 港股 | API质量 | 纸交易 |
|--------|------|------|------|------|------|--------|--------|
| **IBKR** | CL/BZ/ES/GC/DX ✓ | ✓ | Coinbase纳米BTC期货（2026.02新增） | COIL等 ✓ | ✓ | 成熟但复杂 | 完整（端口切换） |
| **Binance** | ✗ | ✗ | BTC/ETH等永续 ✓ | ✗ | ✗ | 优秀 | Testnet ✓ |
| **OKX** | ✗ | ✗ | BTC/ETH等永续 ✓ | BTC期权 ✓ | ✗ | 良好 | Demo ✓（2026.06可能关闭） |
| **Alpaca** | ✗ | ✓ | ✓ | ✓ | ✗ | 极简REST | ✓ |

---

## 三、方案A：自建方案（IBKR API直连）

### 3.1 技术栈

**Rust直连路径**：`ibapi` crate（wboayue/rust-ibapi）
- 当前稳定版：v2.2.2（2025-11-25），sync模式（crossbeam）
- 开发版：v3.0（git only），async模式（tokio + broadcast channels + protobuf）
- 安装v3.0：`cargo add ibapi --git https://github.com/wboayue/rust-ibapi`

**Python辅助路径**：`ib_async` v2.1.0（ib_insync继任者）
- 原始ib_insync创建者Ewald de Wit已故（2024），ib_async由新团队维护
- PyPI包名：`ib_async`
- 文档：https://ib-api-reloaded.github.io/ib_async/

### 3.2 连接架构

```
                    ┌─────────────────────┐
                    │   IB Gateway (4001)  │
                    │   + IBC Controller   │  ← 自动重启/登录
                    │   + Docker容器部署    │
                    └──────────┬──────────┘
                               │ TCP Socket
           ┌───────────────────┼───────────────────┐
           │                   │                   │
    [Rust引擎 cId=1]    [风控模块 cId=2]    [监控 cId=3]
    ibapi v3 crate       ib_async            ib_async
    行情→1s聚合→信号     仓位/风险检查        账户/P&L
```

- IB Gateway vs TWS：生产环境用Gateway（轻量无GUI），开发用TWS
- 端口：Live=4001/7496，Paper=4002/7497
- 每实例最多32个并发客户端连接
- IBC Controller处理每日凌晨自动重启和2FA

### 3.3 LMT订单实现

```rust
// Rust (ibapi v3 伪代码)
let order = OrderBuilder::new()
    .action(Action::Buy)
    .order_type(OrderType::Limit)
    .total_quantity(1.0)
    .limit_price(mid_price)  // 从实时行情计算
    .tif(TimeInForce::GTC)
    .build();
```

**Bid/Mid/Ask定价策略**：
- 订阅`reqTickByTickData(BidAsk)`获取实时bid/ask
- 买单价 = mid_price 或 bid（保守）
- 卖单价 = mid_price 或 ask（保守）
- IB原生支持Peg to Midpoint（`PEG MID`），自动跟随中间价
- Adaptive Orders：IB算法在bid/ask间寻找最优成交价

**速率限制**：
- 50 messages/second（全局）
- Order Efficiency Ratio (OER) < 20：`(提交+修改+撤单) / (成交+1)`
- 每合约每方向最多20个活跃订单

### 3.4 多标的覆盖

| 标的 | IBKR合约规格 | 交易所 |
|------|-------------|--------|
| CL（WTI原油） | FUT, NYMEX | CME Group |
| BZ（Brent原油） | FUT, NYMEX (ICE路由) | ICE via IB |
| ES（E-mini S&P） | FUT, CME | CME Group |
| GC（黄金） | FUT, COMEX | CME Group |
| DX（美元指数） | FUT, NYBOT | ICE |
| OKLO/PINS/QQQ | STK, SMART | US交易所 |
| COIL期权 | OPT | ICE |
| 港股 | STK, SEHK | 香港交易所 |
| BTC纳米期货 | FUT, Coinbase Derivatives | Coinbase（2026.02新增） |

### 3.5 纸交易→实盘切换

切换仅需改端口号（Paper: 4002 → Live: 4001）。推荐用环境变量控制：

```bash
# .env
IBKR_MODE=paper  # or "live"
IBKR_PORT=4002   # 4001 for live
```

**Paper环境限制**：不支持VWAP/Auction/RFQ/Pegged to Market订单类型（LMT完全支持）。

### 3.6 方案A评估

**优点**：
- 零中间件，最低延迟
- 纯Rust端到端（ibapi v3 + 缠论引擎）
- 单一broker覆盖期货+美股+期权+港股+BTC纳米期货
- IBC+Docker可实现无人值守运行

**缺点**：
- 风控/组合管理/回测引擎全部自建
- ibapi v3尚未正式发布crates.io
- IBKR加密能力有限（仅Coinbase纳米合约）
- 历史1s数据回填受pacing限制（需Databento补充）

---

## 四、方案B：NautilusTrader（推荐）

### 4.1 为什么NautilusTrader是最佳选择

1. **Rust-native核心**：22个Rust core crate + 17个adapter crate，PyO3桥接到Python策略层
2. **IBKR完整支持**：InteractiveBrokersClient/DataClient/ExecutionClient，覆盖所有IBKR资产类别
3. **Databento一级集成**：`nautilus-databento` crate，支持直接加载.dbn文件和Parquet catalog
4. **确定性架构**：同一策略代码零修改从回测切换到实盘（research-to-live semantic parity）
5. **1s bar原生支持**：`BarSpecification(step=1, step_type=SECOND)` + DataEngine自动从tick聚合
6. **加密交易所adapter**：Binance、Bybit等

**当前版本**：v1.227.0 Beta（2026-05-18），GitHub 22,915 stars，LGPL v3.0开源

### 4.2 与Rust缠论引擎的集成方式

**方式一：Rust crate直接依赖**

`nautilus-trading` 发布在crates.io，缠论引擎可作为Rust依赖直接调用NautilusTrader核心做下单。

**方式二：C FFI**

cbindgen生成C接口，缠论引擎通过FFI调用NautilusTrader。

**方式三：MessageBus信号注入（推荐）**

```
Rust缠论引擎 → NATS/Redis → NautilusTrader MessageBus → Strategy消费信号 → 下单
```

NautilusTrader MessageBus原生支持Redis backing，所有可序列化消息通过MPSC channel传输到独立Rust线程。

**方式四：PyO3混合**

缠论引擎通过PyO3暴露为Python模块（`#[pyclass]`），在NautilusTrader Python策略中直接调用：

```python
from chanlun_engine import ChanEngine  # PyO3编译的Rust模块

class ChanStrategy(Strategy):
    def on_bar(self, bar: Bar):
        signal = self.engine.process_bar(bar)  # Rust计算
        if signal.is_buy():
            self.submit_order(...)  # NautilusTrader下单
```

### 4.3 架构图

```
                    ┌──────────────────────────────┐
                    │        数据层                  │
                    │                              │
                    │  Databento Live API ──────────┤──→ 期货/美股 1s
                    │  Binance WS kline_1s ────────┤──→ BTC永续 1s
                    │  OKX WS candle1s ────────────┤──→ 备选加密 1s
                    │  IBKR reqTickByTickData ──────┤──→ 港股/期权 tick
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │   NautilusTrader DataEngine    │
                    │   (Rust core, 1s bar聚合)     │
                    └──────────────┬───────────────┘
                                   │ 1s bars
                    ┌──────────────▼───────────────┐
                    │   Rust 缠论引擎               │
                    │   (PyO3 #[pyclass] 或         │
                    │    NATS MessageBus)            │
                    │   K线→笔→线段→中枢→走势→买卖点 │
                    └──────────────┬───────────────┘
                                   │ 信号
                    ┌──────────────▼───────────────┐
                    │   NautilusTrader Strategy      │
                    │   (Python策略层)               │
                    │   信号消费→风控→下单            │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │        执行层                  │
                    │                              │
                    │  IB Adapter ─────────────────┤──→ IBKR (期货/美股/期权/港股)
                    │  Binance Adapter ────────────┤──→ Binance (BTC永续)
                    └──────────────────────────────┘
```

### 4.4 回测路径

```python
# 使用Databento Parquet catalog（推荐，比直接DBN解码快10x+）
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.adapters.databento import DatabentoDataLoader

# 加载历史1s数据
loader = DatabentoDataLoader()
bars = loader.from_dbn_file("CL_2024_ohlcv_1s.dbn.zst")

# 写入catalog
catalog = ParquetDataCatalog("./catalog")
catalog.write_data(bars)

# 回测
engine = BacktestEngine(config)
engine.add_venue(IBKR_VENUE)
engine.add_data(catalog.bars(instrument_id, bar_type="1-SECOND-LAST"))
engine.add_strategy(ChanStrategy(config))
engine.run()
```

### 4.5 方案B评估

**优点**：
- Rust-native，与缠论引擎语言统一
- 完整的事件驱动框架（数据→信号→风控→执行→报告）
- 回测/实盘代码统一，确定性回放
- Databento一级集成（免费1s历史数据）
- IBKR+Binance adapter覆盖全市场
- 不需要自建风控/组合管理/回测引擎

**缺点**：
- 仍为Beta（v1.x），API可能变化
- 学习曲线较陡（概念多：Venue/Instrument/DataEngine/ExecEngine/Actor/Strategy）
- IBKR adapter不是最活跃的adapter（Databento和Binance文档更完善）
- Python策略层引入GIL约束（但核心计算在Rust中，影响有限）

---

## 五、方案C：Rust直连（无框架）

### 5.1 架构

```
Rust缠论引擎 ──→ ibapi v3 (tokio) ──→ IBKR TWS/Gateway
              ├→ crypto-botters   ──→ Binance/Bybit
              └→ rs_bybit         ──→ Bybit
```

### 5.2 关键Rust Crate

| Crate | 用途 | 版本/状态 |
|-------|------|----------|
| `ibapi` (wboayue/rust-ibapi) | IBKR连接 | v2.2.2稳定 / v3.0开发中 |
| `crypto-botters` | Binance/Bybit统一接口 | 活跃 |
| `binance-rs` | Binance专用 | ccxt org维护 |
| `rs_bybit` | Bybit V5 API | REST+下单 |
| `databento` | 历史/实时数据 | 官方Rust SDK |
| `tokio-tungstenite` | WebSocket基础 | 成熟 |
| `fefix` (FerrumFIX) | FIX协议 | 纯Rust |

### 5.3 方案C评估

**优点**：端到端Rust，性能最优，无FFI/IPC开销，单一语言栈

**缺点**：风控/回测/监控/报告全部自建，ibapi v3未正式发布，策略迭代速度慢（Rust vs Python），加密交易所Rust生态不如Python成熟

**适用**：延迟极度敏感（毫秒内）、策略逻辑固化不常变、长期目标。当前不推荐作为MVP方案。

---

## 六、方案D：消息队列解耦

### 6.1 架构

```
Rust缠论引擎 ──→ NATS JetStream ──→ 执行服务A (IBKR) [Python/Rust]
                                  ├→ 执行服务B (Binance) [Python/Rust]
                                  ├→ 监控服务 (Grafana)
                                  └→ 审计日志 (持久化)
```

### 6.2 方案D评估

**优点**：完全解耦（引擎和执行层独立演进），天然多消费者，NATS JetStream持久化（审计追踪），NautilusTrader MessageBus原生支持Redis backing，可靠性最高

**缺点**：额外基础设施（NATS/Redis），端到端延迟增加（毫秒级），运维复杂度最高

**适用**：生产加固阶段（M5），不适合MVP。

---

## 七、其他平台简评

### 7.1 不推荐用于本系统的平台

| 平台 | 排除原因 |
|------|---------|
| **Zipline-reloaded** | 无原生实盘，纯回测引擎，无1s支持 |
| **VectorBT** | 回测/研究工具，非执行引擎 |
| **Freqtrade** | 仅加密货币，不覆盖传统市场 |
| **Hummingbot** | 加密做市专用，不覆盖传统市场 |
| **Jesse** | 仅加密货币 |
| **聚宽/米筐/掘金/天勤/QMT** | 纯中国A股市场基础设施，不支持国际市场/IBKR |

### 7.2 条件性可用的平台

| 平台 | 场景 | 限制 |
|------|------|------|
| **QuantConnect (Lean)** | 如果偏好C#/Python全托管云方案 | 架构重，C#引擎增加复杂度，Rust集成需HTTP/TCP桥接 |
| **Backtrader** | 快速原型，已有Backtrader经验 | 框架老化，维护不积极，无1s原生支持 |
| **vn.py (VeighNa)** | 统一Python框架+IBKR+加密 | 文档主要中文，与现有Rust架构冗余 |
| **Alpaca** | 仅做美股+加密，免佣金 | 无期货，不覆盖全市场 |

### 7.3 TradingView的角色

TradingView不应作为执行层或数据源，而应作为：
- **可视化层**：缠论指标叠加、图表分析（通过现有TradingView MCP）
- **信号监控**：截图+标注查看当前走势状态
- **不适合**：1s历史数据（深度仅5000-20000 bars），程序化数据获取

---

## 八、加密合约平台专项评估

### 8.1 交易所对照

| 维度 | Binance Futures | OKX | Bybit | dYdX v4 |
|------|----------------|-----|-------|---------|
| **1s K线** | 原生kline_1s | 原生candle1s | 无（最小1m） | 无（最小1m） |
| **BTC永续费率** | 0.02%/0.05% (maker/taker) | 0.02%/0.05% | 0.01%/0.06% | 0.01%/0.05% |
| **最大杠杆** | 100x | 100x | 100x | 20x |
| **API速率限制** | 1200 weight/min | 20 req/2s (最严) | 120 req/5s | 链上tx |
| **WS推送频率** | 250ms | ~1s | 1-60s | ~1s块 |
| **Testnet** | 完整 | 2026.06可能关闭 | 完整 | 不适用（去中心化） |
| **Rust SDK** | binance-rs (社区) | 无成熟 | rs_bybit | dydx-v4-client |
| **Python SDK** | 官方 | 官方 | 官方 | 官方 |
| **订单延迟** | ~10-50ms | ~10-50ms | <10ms（声称） | 1-2s（块确认） |
| **API文档质量** | 最佳 | 良好 | 良好 | 范式不同 |

### 8.2 推荐组合

**主选**：Binance Futures —— 原生1s K线、最低延迟、最完善文档、最大流动性

**备选**：OKX —— 原生1s K线、Portfolio Margin模式、BTC期权（如需）

**不推荐**：
- Bybit：无原生1s K线，需自聚合
- dYdX v4：最大20x杠杆，1-2s块确认延迟，1s bar不原生

### 8.3 CCXT统一接口

CCXT（100+交易所）提供统一Python API，但**无官方Rust版**。社区Rust替代：
- `ccxt-rust`（by Praying）：最成熟，tokio异步，WebSocket自动重连
- `binance-rs`（ccxt org）：Binance专用

**评估**：如果通过NautilusTrader连接Binance，不需要CCXT。CCXT适合快速原型或同时连接多个交易所的场景。

---

## 九、Rust-Python互操作方案

### 9.1 PyO3（推荐首选）

- v0.28，事实标准（Polars/Pydantic V2/HuggingFace均基于PyO3）
- FFI调用开销 < 1ms，数据验证5-50x加速
- `#[pyclass]` + `#[pymethods]` 暴露Rust struct为Python class
- 构建工具：Maturin v1.8（`maturin develop` 本地开发，`maturin build --release` 生产wheel）

### 9.2 IPC方案对比

| 方案 | 延迟 | 适用场景 |
|------|------|---------|
| PyO3（in-process） | ~微秒 | 计算密集热路径（推荐阶段一） |
| 共享内存 | ~纳秒 | 跨进程极低延迟 |
| Unix Socket | ~微秒 | 跨进程中等延迟 |
| gRPC (tonic) | ~毫秒 | 跨机器/跨语言 |
| NATS/Redis | ~毫秒 | 解耦多消费者（推荐阶段三） |

### 9.3 rust-cpython

**已废弃**，不支持Python 3.12+。不应在新项目中使用。

---

## 十、从回测到实盘的Roadmap

### 阶段一：回测验证（2-4周）

**目标**：用Databento 1s历史数据在NautilusTrader中复现现有回测结果

1. 安装NautilusTrader + nautilus-databento
2. 从Databento下载CL/BZ/ES ohlcv-1s历史数据（免费）
3. 将缠论引擎通过PyO3暴露为Python模块
4. 编写NautilusTrader Strategy，调用缠论引擎处理1s bars
5. 回测验证：结果应与现有Rust回测引擎一致
6. 验证BTC数据：从Binance API获取历史1s K线

**交付物**：NautilusTrader回测报告，与现有引擎结果对比

### 阶段二：纸交易（2-4周）

**目标**：IBKR Paper Trading + Binance Testnet 联调

1. 配置NautilusTrader IBKR adapter连接Paper账户（端口4002）
2. 配置Binance adapter连接Testnet
3. Databento Live API接入实时1s数据（期货）
4. Binance WebSocket接入实时1s K线（BTC）
5. LMT订单逻辑：bid/mid/ask动态定价
6. 风控层：最大仓位、单笔金额限制、P&L止损
7. 运行72小时+无人值守测试

**交付物**：纸交易日志、订单执行报告、延迟统计

### 阶段三：单标的实盘（2-4周）

**目标**：CL期货单标的小仓位实盘

1. IBKR端口切换：4002 → 4001
2. IBC Controller + Docker部署
3. 初始仓位：1手CL（最小可交易单位）
4. 监控：实时P&L、订单状态、连接健康
5. 逐步加入BZ、ES

**交付物**：实盘交易记录、Sharpe ratio、最大回撤

### 阶段四：多标的扩展（4-8周）

**目标**：全市场覆盖

1. 加入美股（OKLO/PINS/QQQ）
2. 加入BTC永续合约（Binance Futures实盘）
3. 加入期权（COIL等通过IBKR）
4. 加入港股（如有标的需求）
5. 多标的并行信号处理 + 风控联动

**交付物**：全市场运行报告、相关性分析

### 阶段五：生产加固（持续）

**目标**：从方案B（NautilusTrader MVP）演进到方案D（消息队列解耦）

1. 引入NATS JetStream做信号分发
2. 审计日志持久化
3. Grafana监控面板
4. 灾备：多VPS部署、自动故障转移
5. 回测→实盘全自动CI/CD pipeline

---

## 十一、成本估算

### 月度固定成本

| 项目 | 费用 | 说明 |
|------|------|------|
| Databento CME Live | $179/月 | 期货实时数据 |
| Databento OHLCV-1s历史 | 免费 | 回测数据 |
| IBKR行情订阅 | $10-30/月 | US Securities Bundle等 |
| IBKR最低活动费 | $0-10/月 | 佣金抵扣 |
| Binance Futures | 免费 | API免费 |
| VPS（可选） | $20-50/月 | 低延迟部署 |
| **合计** | **~$210-270/月** | |

### 交易成本

| 标的 | 佣金/手 | 说明 |
|------|---------|------|
| CL期货 | $1.25-2.50 | IBKR佣金 |
| ES期货 | $0.85-2.50 | IBKR佣金 |
| 美股 | $0.005/股 | IBKR固定费率 |
| BTC永续 | 0.02% maker | Binance |

---

## 十二、技术决策总结

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 执行框架 | NautilusTrader | Rust-native，IBKR+Binance adapter，1s bar原生支持 |
| 传统市场broker | IBKR | 现有账户，全市场覆盖 |
| 加密合约 | Binance Futures | 原生1s K线，最大流动性，最佳API |
| 历史数据 | Databento | 免费ohlcv-1s，CME回溯至2010，Rust SDK |
| 实时数据（期货） | Databento Live | intraday replay消除gap |
| 实时数据（加密） | Binance WebSocket | kline_1s，250ms推送 |
| Rust→Python桥接 | PyO3 + Maturin | 行业标准，<1ms开销 |
| 信号分发（MVP） | PyO3 in-process | 最简单 |
| 信号分发（生产） | NATS JetStream | 解耦，持久化，多消费者 |
| 可视化 | TradingView MCP | 现有基础设施 |
| 连接管理 | IBC + Docker | 无人值守 |
| 订单类型 | 纯LMT | 绝对禁止MKT单 |

---

## 附录A：关键链接

| 资源 | URL |
|------|-----|
| NautilusTrader | https://nautilustrader.io/ |
| NautilusTrader GitHub | https://github.com/nautechsystems/nautilus_trader |
| NautilusTrader IBKR Adapter | https://nautilustrader.io/docs/nightly/integrations/ib/ |
| nautilus-trading crate | https://crates.io/crates/nautilus-trading |
| rust-ibapi (ibapi crate) | https://github.com/wboayue/rust-ibapi |
| ib_async | https://github.com/ib-api-reloaded/ib_async |
| Databento Rust SDK | https://github.com/databento/databento-rs |
| Databento OHLCV-1s Schema | https://databento.com/docs/schemas-and-data-formats/ohlcv |
| Binance Futures WS kline_1s | https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Kline-Candlestick-Streams |
| OKX V5 API | https://www.okx.com/docs-v5/en/ |
| PyO3 | https://pyo3.rs/ |
| Maturin | https://www.maturin.rs/ |
| IBC Controller | https://github.com/IbcAlpha/IBC |
| crypto-botters | https://lib.rs/crates/crypto-botters |
| CCXT | https://docs.ccxt.com/ |
| ccxt-rust | https://github.com/Praying/ccxt-rust |

## 附录B：IBKR 1s数据Pacing规则速查

| 规则 | 限制 |
|------|------|
| 相同请求间隔 | 15秒内不得重复 |
| 同合约/交易所/Tick类型 | 2秒内不超过6次 |
| 全局请求频率 | 10分钟内不超过60次 |
| 1s历史bar每次请求最大范围 | 1800秒（30分钟） |
| 实时bar最小周期 | 5秒（reqRealTimeBars） |
| tick-by-tick同instrument间隔 | 15秒 |

## 附录C：Databento数据集映射

| 标的 | Dataset | Schema | 说明 |
|------|---------|--------|------|
| CL/ES/GC/BZ期货 | GLBX.MDP3 | ohlcv-1s | CME Globex |
| DX期货 | IFEU.IMPACT | ohlcv-1s | ICE Futures Europe |
| US股票 | XNAS.ITCH / DBEQ.BASIC | ohlcv-1s | NASDAQ/综合 |
| CME期权 | GLBX.MDP3 | ohlcv-1s | CME Group |
