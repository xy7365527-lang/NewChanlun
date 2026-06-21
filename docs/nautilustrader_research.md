# NautilusTrader 研究报告

> 基于 NautilusTrader 官方文档（2026-06-20 版本）的完整研读
> 目的：评估 NautilusTrader 作为缠论量化系统的研究和生产平台

---

## 1. 架构概述

### 1.1 设计哲学

NautilusTrader 是一个 Rust 原生、事件驱动的多资产多场所交易引擎。核心设计模式：

- **领域驱动设计（DDD）**：围绕交易领域概念构建
- **事件驱动架构**：所有组件通过事件和消息交互
- **端口和适配器模式**：模块化集成外部交易所和数据源
- **崩溃恢复设计（Crash-only）**：启动和崩溃恢复共享同一代码路径
- **数据完整性优先**：遇到无效数据（NaN、溢出等）立即 panic，不允许静默腐败

### 1.2 核心组件

| 组件 | 职责 |
|------|------|
| **NautilusKernel** | 中央编排，初始化和管理所有系统组件 |
| **MessageBus** | 组件间通信骨干（Pub/Sub、Req/Rep、点对点） |
| **Cache** | 高性能内存存储（instruments、orders、positions 等） |
| **DataEngine** | 处理和路由市场数据到订阅者 |
| **ExecutionEngine** | 管理订单生命周期和执行 |
| **RiskEngine** | 交易前风控检查和验证 |

### 1.3 线程模型

**关键点**：内核在**单线程**上消费和分发消息。这包括 MessageBus、策略逻辑、风控引擎、Cache 读写。单线程核心提供确定性事件顺序，保证回测-实盘一致性。

后台服务使用独立线程或异步运行时：
- 网络 I/O（WebSocket、REST）
- 持久化（DataFusion 查询、数据库操作）
- 适配器异步操作

### 1.4 环境上下文

- **Backtest**：历史数据 + 模拟场所
- **Sandbox**：实时数据 + 模拟场所
- **Live**：实时数据 + 真实场所（模拟盘或实盘）

### 1.5 三套系统实现

| 实现 | 特点 | IB 支持 |
|------|------|---------|
| **v1 Legacy（Cython）** | 最完整，覆盖最广 | 有 |
| **v2 Rust** | 纯 Rust，无需 Python | 无 |
| **v2 PyO3** | Python 策略运行在 Rust 引擎上 | 无 |

**关键发现：IB 适配器目前只在 v1 Legacy 中可用。** v2 Rust 和 v2 PyO3 支持大多数加密货币交易所但不支持 IB。

---

## 2. 我们的 Rust 引擎怎么接入

### 2.1 集成架构选择

有三条可行路径：

#### 路径 A：PyO3 Strategy 包装（推荐）

我们的 Rust 缠论引擎通过 PyO3 暴露为 Python 模块，编写一个继承 `Strategy` 的 Python 类作为胶水层：

```python
from nautilus_trader.trading.strategy import Strategy
from chanlun_engine import ChanLunEngine  # 我们的 PyO3 模块

class ChanLunStrategy(Strategy):
    def __init__(self, config):
        super().__init__(config)
        self.engine = ChanLunEngine(config.engine_params)

    def on_bar(self, bar):
        # 把 bar 数据喂给我们的 Rust 引擎
        signal = self.engine.process_bar(
            bar.open.as_double(),
            bar.high.as_double(),
            bar.low.as_double(),
            bar.close.as_double(),
            bar.volume.as_double(),
            bar.ts_event,
        )
        if signal.has_bsp():
            self._execute_signal(signal)
```

**优点**：
- 可以利用 NautilusTrader 的完整执行层、风控、回测
- 我们的引擎保持独立，NautilusTrader 负责执行和基础设施
- IB 适配器可用（v1 Legacy）
- 回测和实盘零代码切换

**缺点**：
- 需要 Python 层作为胶水，引入少量延迟
- 依赖 v1 Legacy 路径（IB 的限制）

#### 路径 B：CustomData 通道

将我们的信号引擎作为独立进程运行，通过 NautilusTrader 的 CustomData 机制注入信号：

```python
from nautilus_trader.core import Data

@customdataclass_pyo3
class ChanLunSignal:
    instrument_id: str
    signal_type: str  # "B1", "B2", "B3", "S1", "S2", "S3"
    level: int
    price: float
    ts_event: int
    ts_init: int
```

信号通过 DataEngine 路由到 Strategy 的 `on_data()` handler，策略只负责执行。

**优点**：引擎和平台完全解耦
**缺点**：多进程通信开销，需要管理两个系统

#### 路径 C：Rust 原生策略（不推荐）

直接用 Rust 写 NautilusTrader 策略，调用我们的引擎 crate：

```rust
impl DataActor for ChanLunStrategy { ... }
impl Strategy for ChanLunStrategy { ... }
```

**致命缺点**：v2 Rust 不支持 IB 适配器，无法满足我们的期货执行需求。

### 2.2 推荐集成方式

**选择路径 A（PyO3 Strategy 包装）**，理由：
1. IB 适配器只在 v1 Legacy 中可用，路径 A 是唯一兼容路径
2. 我们的引擎已经有 PyO3 绑定，胶水层编写成本低
3. 回测-实盘一致性由框架保证
4. 风控引擎、执行算法等基础设施免费获得

### 2.3 数据流设计

```
市场数据 → NautilusTrader DataEngine → on_bar() → 我们的Rust引擎
                                                       ↓
                                                  BSP信号产出
                                                       ↓
                                              Strategy.submit_order()
                                                       ↓
                                              RiskEngine → ExecutionEngine → IB
```

### 2.4 需要实现的接口

1. **Strategy 子类**：继承 `Strategy`，实现 `on_start()`, `on_bar()`, `on_order_filled()`, `on_position_changed()` 等
2. **StrategyConfig 子类**：定义配置参数（引擎参数、交易参数）
3. **数据转换层**：NautilusTrader `Bar` → 我们的引擎输入格式
4. **信号到订单映射**：BSP 信号 → NautilusTrader `Order` 对象

---

## 3. 回测能力

### 3.1 两级 API

| 级别 | 类 | 特点 | 适用场景 |
|------|-----|------|---------|
| **低级 API** | `BacktestEngine` | 手动加载数据，直接控制 | 数据量可放入内存 |
| **高级 API** | `BacktestNode` | 从 Parquet Catalog 流式加载 | 大数据集，参数扫描 |

### 3.2 事件驱动回测

NautilusTrader 的回测是严格的事件驱动：
- 每个数据点按时间顺序处理
- 模拟交易所先处理订单（使用最新价格撮合），再将数据分发给策略
- 保证策略不会看到"未来"的价格
- 支持多标的、多场所并行模拟

### 3.3 数据格式

支持的数据类型：
- `Bar`（OHLCV K 线）—— **我们的主要数据类型**
- `QuoteTick`（买卖报价）
- `TradeTick`（逐笔成交）
- `OrderBookDeltas`（订单簿增量）

**数据加载方式**：
- 低级 API：直接传入 Python 列表，`engine.add_data(bars)`
- 高级 API：Parquet 文件 catalog，支持流式加载和分块

**Parquet Catalog** 是推荐的数据存储方式，使用 Apache Arrow 格式，支持：
- 高效列式存储
- 按时间范围查询
- 流式加载（不需要全部放入内存）

### 3.4 跟我们现有 `t_engine_run.rs` 回测的对比

| 维度 | 我们的 `t_engine_run.rs` | NautilusTrader BacktestEngine |
|------|------------------------|-------------------------------|
| 执行速度 | 纯 Rust，极快 | Rust 核心 + Python 胶水，略慢 |
| 订单模拟 | 简单（可能无滑点/佣金） | 完整模拟（滑点、佣金、延迟、拒绝） |
| 订单类型 | 有限 | 完整（Market/Limit/Stop/StopLimit/TrailingStop 等） |
| 风控 | 无 | 完整（仓位限制、名义值限制、频率限制） |
| 多标的 | 支持 | 支持，且有完整的 Portfolio 管理 |
| 实盘切换 | 需要重写 | 零代码切换 |
| 绩效分析 | 需要自建 | 内置 PortfolioAnalyzer |

**建议**：保留 `t_engine_run.rs` 用于纯信号验证（速度优势），使用 NautilusTrader 做完整回测（订单模拟、风控、绩效分析）。

### 3.5 大数据处理

我们有约 460 万 bar/标的，6 个标的 = 约 2760 万 bar。

NautilusTrader 的处理策略：
1. **低级 API**：一次性加载所有数据到内存。对于 bar 数据，每个 bar 大约 100 bytes，2760 万 bar ≈ 2.7 GB。在现代机器上可以放入内存
2. **高级 API**：使用 `BacktestNode` 从 Parquet catalog 流式加载，分块处理，内存占用可控
3. **优化技巧**：按标的顺序加载数据并设 `sort=False`，最后统一排序一次

### 3.6 连续期货回测

NautilusTrader 内置 **Continuous Futures** 支持：
- 支持四种调整模式：向前/向后 × 价差/比率
- 调用者提供 roll transition 表（时间、前合约、后合约、前价格、后价格）
- 引擎自动处理价格调整和合约切换
- 这对我们的 CL/BRN/ES 期货回测非常有用

---

## 4. IBKR 实盘集成

### 4.1 IB 适配器状态

**状态：Stable（稳定）**。这是官方支持的集成。

核心组件：
- `InteractiveBrokersClient`：TWS API 的中心客户端
- `InteractiveBrokersDataClient`：市场数据流
- `InteractiveBrokersExecutionClient`：订单管理和执行
- `InteractiveBrokersInstrumentProvider`：合约/工具定义
- `HistoricInteractiveBrokersClient`：历史数据获取

### 4.2 连接方式

两种连接方式：
1. **直连 TWS/IB Gateway**：指定 host/port
2. **Docker 化 IB Gateway**（推荐生产部署）：自动管理连接

```python
data_config = InteractiveBrokersDataClientConfig(
    ibg_host="127.0.0.1",
    ibg_port=4001,  # 实盘端口
    ibg_client_id=1,
)
exec_config = InteractiveBrokersExecClientConfig(
    ibg_host="127.0.0.1",
    ibg_port=4001,
    ibg_client_id=1,
    account_id="U1234567",
)
```

### 4.3 支持的资产类别

- 股票（Equities）
- 期货（Futures）—— **我们的核心需求**
- 期权（Options）—— **COIL 期权需要**
- 外汇（Forex）
- 加密货币（Crypto）—— **BTC**
- 商品（Commodities）
- 指数（Indices）
- 债券（Bonds）
- CFDs

### 4.4 期货合约支持

NautilusTrader 有完整的 `FuturesContract` 类型，字段包括：
- `instrument_id`：如 `ESZ25.XCME`
- `underlying`：如 `ES`
- `currency`：如 `USD`
- `multiplier`：合约乘数
- `activation_ns` / `expiration_ns`：生效/到期时间
- `price_precision` / `price_increment`：价格精度

期货链加载示例：
```python
IBContract(secType='CONTFUT', exchange='NYMEX', symbol='CL', build_futures_chain=True)
IBContract(secType='FUT', exchange='CME', symbol='ES', lastTradeDateOrContractMonth='20250315')
```

**符号约定**（IB_SIMPLIFIED 模式）：
- 连续期货：`ES.CME`, `CL.NYMEX`
- 个别合约：`ESM4.CME`, `CLZ7.NYMEX`
- MIC 转换：`CME → XCME`, `NYMEX → XNYM`

### 4.5 支持的订单类型

NautilusTrader 支持完整的订单类型谱：

| 订单类型 | 说明 | 我们的用途 |
|---------|------|----------|
| Market | 市价单 | 紧急出场 |
| Limit | 限价单 | **主要订单类型（LMT only 策略）** |
| Stop Market | 止损市价 | 风控止损 |
| Stop Limit | 止损限价 | 精确止损 |
| Market-if-Touched | 触碰市价 | 入场触发 |
| Limit-if-Touched | 触碰限价 | 入场触发 |
| Trailing Stop Market | 追踪止损 | 利润保护 |

**订单特性**：
- 支持 GTC、GTD、IOC、FOK 等时效
- 支持 reduce-only 标记
- 支持订单列表（bracket orders）
- 策略可管理 GTD 过期（即使交易所不支持）

### 4.6 执行和调和

实盘执行有完善的调和（Reconciliation）机制：
- 启动时自动调和：对比本地状态和交易所状态，补齐缺失事件
- 运行时持续检查：in-flight 订单超时检测、定期轮询 open orders
- 重复成交检测：基于 trade_id 的去重
- 位置不匹配修复：自动生成调和订单

### 4.7 OMS 类型

- **NETTING**：每个标的一个净仓位（期货的标准模式）
- **HEDGING**：每个标的可有多个仓位（多头+空头）

对我们的缠论系统，**NETTING** 模式更合适——每个标的维护一个净仓位，买卖点信号直接转换为仓位调整。

---

## 5. Pro 版 vs 开源版

### 5.1 Pro 版提供的额外功能

| 功能 | 说明 | 交付方式 |
|------|------|---------|
| **Pro Dashboard** | 实时监控面板：Portfolio 状态、执行追踪、策略管理 | Docker |
| **Execution Engine** | 执行算法：VWAP、Iceberg、POV（参与率）、Sniper | Docker |
| **Risk Engine** | 高级风控：交易前检查、仓位限制、保证金控制、频率限制 | Docker |
| **Nautilus Nexus** | 分布式消息总线：零拷贝 IPC、亚微秒延迟 | Docker |

### 5.2 对我们的评估

| Pro 功能 | 对我们有用吗 | 理由 |
|---------|-------------|------|
| Dashboard | **中等** | 有用但非必须，可以自建 |
| VWAP/Iceberg | **低** | 我们的订单量小（CL 每天几手），不需要大单拆分 |
| POV | **低** | 同上 |
| Risk Engine | **低** | 开源版的 RiskEngine 已经够用 |
| Nexus | **低** | 我们是单节点运行，不需要分布式 |

### 5.3 结论

**不需要 Pro 版。** 开源版完全满足我们的需求。Pro 版主要面向：
- 高频交易（需要 VWAP/Iceberg 拆单）
- 多节点分布式部署（需要 Nexus 消息总线）
- 机构级运营（需要 Dashboard 监控）

我们的场景是低频期货交易（每天几笔到几十笔），单节点部署，开源版的功能绑绑有余。

### 5.4 执行算法说明

虽然 Pro 版的 VWAP/Iceberg 对我们不必要，但开源版内置了 **TWAP（时间加权平均价）** 执行算法，且支持自定义执行算法。如果未来需要：

```python
order = self.order_factory.market(
    instrument_id=self.instrument_id,
    order_side=OrderSide.BUY,
    quantity=self.instrument.make_qty(10),
    exec_algorithm_id=ExecAlgorithmId("TWAP"),
    exec_algorithm_params={"horizon_secs": 60, "interval_secs": 5},
)
```

---

## 6. 对我们项目的具体建议

### 6.1 迁移路径

#### 第一阶段：数据准备（1-2 周）

1. **将现有 bar 数据转换为 NautilusTrader Parquet catalog 格式**
   - 数据类型：1 分钟 Bar（OHLCV）
   - 每个标的一个 Parquet 文件或分片
   - 定义 `FuturesContract` instrument 对象

2. **数据转换脚本**
   ```python
   from nautilus_trader.persistence.catalog import ParquetDataCatalog
   from nautilus_trader.model import Bar, BarType
   # 将我们的数据格式转换为 NautilusTrader Bar 格式
   ```

#### 第二阶段：Strategy 包装（1-2 周）

1. **编写 `ChanLunStrategy`** 继承 `Strategy`
2. **编写 `ChanLunStrategyConfig`** 继承 `StrategyConfig`
3. **实现数据转换层**：`Bar` → 引擎输入
4. **实现信号执行层**：BSP 信号 → 订单提交

#### 第三阶段：回测验证（2-3 周）

1. 使用 `BacktestEngine` 运行历史回测
2. 对比结果与 `t_engine_run.rs` 的输出
3. 验证订单执行、仓位计算、PnL 计算
4. 添加连续期货 roll 处理

#### 第四阶段：IB 实盘接入（1-2 周）

1. 配置 `InteractiveBrokersDataClientConfig` 和 `InteractiveBrokersExecClientConfig`
2. Paper trading 验证
3. 订单类型映射（BSP → Limit Order）
4. 风控参数配置

### 6.2 需要改什么

| 项目 | 是否需要改 | 改动内容 |
|------|----------|---------|
| 缠论 Rust 引擎 | **不需要** | 引擎保持独立，通过 PyO3 调用 |
| 信号计算逻辑 | **不需要** | 引擎内部逻辑不变 |
| 数据格式 | **需要** | 添加 NautilusTrader Bar 格式的输入适配 |
| 执行层 | **需要** | 从我们自己的 IBKR 接口迁移到 NautilusTrader 执行层 |
| 回测驱动 | **需要** | 从 `t_engine_run.rs` 迁移到 NautilusTrader 回测框架 |
| 风控 | **需要** | 利用 NautilusTrader 的 RiskEngine 替代我们的手动风控 |
| 绩效分析 | **可选** | 可以利用 NautilusTrader 的 PortfolioAnalyzer |

### 6.3 不需要改什么

- **缠论引擎核心**（笔、线段、中枢、买卖点识别）—— 完全保留
- **多级别递归分析**（赋格结构）—— 完全保留
- **PyO3 绑定层** —— 已有，直接复用
- **TradingView MCP 集成** —— 独立于执行平台

### 6.4 风险和注意事项

#### 高风险

1. **IB 适配器只在 v1 Legacy 中可用**
   - v2 Rust/PyO3 路径不支持 IB，这意味着我们被锁定在 v1 Legacy 路径
   - 如果 Nautilus 团队未来废弃 v1 Legacy（目前没有迹象），我们需要迁移
   - 缓解：关注 IB 适配器是否被移植到 v2

2. **单进程单 TradingNode 限制**
   - 同一进程不能运行多个 `TradingNode`（全局单例状态）
   - 多策略必须添加到同一个 `TradingNode`
   - 如果需要隔离，必须运行多个进程

#### 中风险

3. **数据格式转换**
   - 我们的 460 万 bar/标的的数据需要转换为 Parquet catalog 格式
   - 一次性工作，但需要确保时间戳精度（纳秒）和价格精度对齐

4. **连续期货 roll 处理**
   - NautilusTrader 要求调用者提供 roll transition 表
   - 我们需要自己构建和维护这个表（时间、前后合约、前后价格）
   - 引擎不会自动发现 roll 时点

5. **版本迭代速度**
   - NautilusTrader 仍在活跃开发中，API 可能在版本间有 breaking changes
   - 双周发布节奏
   - 需要固定版本号并定期评估升级

#### 低风险

6. **Python 层性能**
   - 虽然策略逻辑通过 Python 胶水层调用我们的 Rust 引擎，但 bar 级别的延迟（毫秒级）对 1 分钟 bar 策略完全可以忽略
   - 真正的性能瓶颈不在这里

7. **安装复杂度**
   - 需要 Rust toolchain + Python 3.12-3.14
   - 但 `uv pip install nautilus_trader[ib]` 就够了（预编译 wheel）

### 6.5 安装命令

```bash
# 基础安装
uv pip install nautilus_trader

# 带 IB 支持
uv pip install "nautilus_trader[ib,docker]"

# 带可视化
uv pip install "nautilus_trader[ib,docker,visualization]"
```

支持平台：Python 3.12-3.14，Linux x86_64/ARM64、macOS ARM64、Windows x86_64。

### 6.6 最小可行示例

```python
from decimal import Decimal
from nautilus_trader.config import StrategyConfig
from nautilus_trader.model import Bar, BarType, InstrumentId
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.trading.strategy import Strategy

class ChanLunConfig(StrategyConfig):
    instrument_id: InstrumentId
    bar_type: BarType
    trade_size: Decimal

class ChanLunStrategy(Strategy):
    def __init__(self, config: ChanLunConfig):
        super().__init__(config)
        # from chanlun_engine import Engine
        # self.engine = Engine(...)

    def on_start(self):
        self.instrument = self.cache.instrument(self.config.instrument_id)
        if not self.instrument:
            self.log.error(f"Instrument not found: {self.config.instrument_id}")
            self.stop()
            return
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar):
        # signal = self.engine.process_bar(...)
        # if signal.is_buy():
        #     order = self.order_factory.limit(
        #         instrument_id=self.config.instrument_id,
        #         order_side=OrderSide.BUY,
        #         quantity=self.instrument.make_qty(self.config.trade_size),
        #         price=self.instrument.make_price(signal.entry_price),
        #     )
        #     self.submit_order(order)
        pass

    def on_stop(self):
        self.cancel_all_orders(self.config.instrument_id)
        self.close_all_positions(self.config.instrument_id)
```

---

## 附录：关键 API 参考

| 文档页 | URL |
|--------|-----|
| Architecture | https://nautilustrader.io/docs/latest/concepts/architecture/ |
| Strategies | https://nautilustrader.io/docs/latest/concepts/strategies/ |
| Data | https://nautilustrader.io/docs/latest/concepts/data/ |
| Backtesting | https://nautilustrader.io/docs/latest/concepts/backtesting/ |
| Live Trading | https://nautilustrader.io/docs/latest/concepts/live/ |
| Execution | https://nautilustrader.io/docs/latest/concepts/execution/ |
| Orders | https://nautilustrader.io/docs/latest/concepts/orders/ |
| Futures Contract | https://nautilustrader.io/docs/latest/concepts/instruments/futures_contract/ |
| Continuous Futures | https://nautilustrader.io/docs/latest/concepts/continuous_futures/ |
| Custom Data | https://nautilustrader.io/docs/latest/concepts/custom_data/ |
| IB Integration | https://nautilustrader.io/docs/latest/integrations/ib/ |
| Rust | https://nautilustrader.io/docs/latest/concepts/rust/ |
| Installation | https://nautilustrader.io/docs/latest/getting_started/installation/ |
| Pro | https://nautilustrader.io/pro/ |
| Python API | https://nautilustrader.io/docs/python-api-latest/ |
| Rust API | https://nautilustrader.io/docs/rust-api-latest/ |
