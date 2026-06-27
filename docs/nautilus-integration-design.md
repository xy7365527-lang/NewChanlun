# Nautilus Trader ↔ canonical S_Θ 集成设计

> 工位：nautilus-integration（goal `g-l2-nautilus-production` 第 2 段）
> 日期：2026-06-27
> 认识论等级：本文档全篇 **L0/L1**（架构设计 + 真实接口接线设计，零信息增量）；
> 真实集成跑通 = **L2**（待 nautilus 依赖加入 + 真实数据回测后验，本文档不声称）。
> 调研依据：context7 `/nautechsystems/nautilus_trader`（develop 分支实际代码）+ WebSearch（2026-06）。

---

## 0. 结果包六要素（result-package 强制）

1. **结论**：canonical S_Θ（`theta_v0`，纯 Rust crate）应通过 **Rust-native Strategy 路径**
   （`StrategyCore` + `DataActor` trait + `nautilus_strategy!` 宏）接入 Nautilus Trader，回测用
   `BacktestNode`、实盘用 `LiveNode` + IB adapter，**同一 `ThetaStrategy` 适配器零代码切换**。
2. **定义依据**：S_Θ 是 `Origin.StrategyFamily.piTheta` 的 bit-exact Rust 实装（纯 Rust crate，
   依赖仅 pyo3），输入 `Bar`（OHLC 整数 tick），产 `VoiceDecision`/`Order`（见 §3 接口形状）。
   Nautilus v2 Rust crate（`nautilus-trading`/`nautilus-model`/`nautilus-backtest`）提供 Rust-native
   Strategy trait + IB adapter（context7 坐实，见 §1.4）。两者均为 Rust ⟹ 适配层是同语言 trait 实现，
   零 FFI/Python 胶水开销——满足编排者「性能最大化」定调。
3. **边界条件**（结论翻转条件）：
   - 若 Nautilus Rust-native IB adapter 在目标部署时**实际不可用/不稳定**（本设计基于 develop 分支
     `crates/adapters/interactive_brokers/examples/node_exec_tester.rs` 实例 + crate README，**非
     release 稳定性背书**）⟹ 翻转到 **PyO3 Strategy 路径**（§4.2 备选）。
   - 若 S_Θ 的 1s/1min bar 决策延迟要求容许 Python 胶水毫秒级延迟（旧调研判定可忽略）⟹ 路径选择
     退化为团队偏好问题，非性能硬约束。
   - 若编排者改变「纯 Rust S_Θ 作生产引擎」定调（如保留旧 PyO3 缠论引擎）⟹ 旧调研路径 A 重新成为
     首选。
4. **下游推论**：选 Rust-native 路径 ⟹ (a) `theta_v0` crate 须从 `#[cfg(test)]` 门控的 backtest
   harness 升级为可被外部 binary crate 链接的公开 API（当前 `backtest` 模块 test-only）；(b) 新增
   nautilus 依赖须分离到独立 binary/feature（不污染现有 cdylib Python 扩展构建，见 §5）；(c) sizing
   主线（trades≈0 诊断）的修复**不阻塞本适配层结构**，但适配层的 L2 验证依赖 sizing 产非空订单流。
5. **谱系引用**：本领域（生产引擎路径选择）的**既有谱系记录** = `docs/nautilustrader_research.md`
   （2026-06-20）+ `docs/quant-platform-research-2026.md`（2026-06-12）。**本设计与既有调研存在一处
   决定性冲突**（既有判 Rust-native「不推荐」因 IB 不支持；本设计经 2026-06-27 机器证据坐实该前提
   已过时）——见 §2 冲突标记（这是「选择」类，路径裁定权属编排者，本工位提供证据不擅自结算）。
6. **影响声明**：本产出新增 `docs/nautilus-integration-design.md` + `rust/src/theta_v0/nautilus/`
   适配层骨架（标 TODO，不编译，nautilus 依赖未加）。**不改** S_Θ 任何现有模块、不改 Cargo.toml、
   不改 lib.rs 注册（新模块报 Lead 登记）。

---

## 1. 调研发现（基于真实文档/代码，非参数知识）

### 1.1 Nautilus 架构（context7 + `docs/nautilustrader_research.md` §1）

- Rust 原生、事件驱动、单线程核心（MessageBus + Cache + DataEngine + ExecutionEngine + RiskEngine）。
- 单线程核心保证**回测-实盘事件顺序确定性一致**（回测=实盘共用同一策略代码路径，crash-only 设计）。
- 三套实现：**v1 legacy**（Cython，最全）/ **v2 Rust**（纯 Rust，crates/）/ **v2 PyO3**（Python 组件跑在
  Rust 核心上）。

### 1.2 三种策略接入路径的真实接口

| 路径 | 策略语言 | 调用 S_Θ 方式 | 引擎核心 | 来源 |
|------|---------|--------------|---------|------|
| **Rust-native** | Rust | 直接函数调用（同 crate 链接） | Rust v2 | `docs/how_to/write_rust_strategy.md` |
| **PyO3 Strategy** | Python | PyO3 暴露 S_Θ → Python `on_bar` 调 | Rust v2 PyO3 | `docs/concepts/rust.md` |
| **CustomData 通道** | Python | S_Θ 独立进程 → CustomData 注入信号 | 任意 | 旧调研 §2.1 路径B |

### 1.3 Rust-native Strategy 真实接口（context7 `write_rust_strategy.md` 实证）

```rust
use nautilus_common::actor::DataActor;
use nautilus_model::{data::Bar, enums::OrderSide, identifiers::InstrumentId, types::Quantity};
use nautilus_trading::{nautilus_strategy, strategy::{Strategy, StrategyConfig, StrategyCore}};

pub struct MyStrategy {
    core: StrategyCore,            // ★提供 order_factory + portfolio 集成（区别于 DataActorCore）
    instrument_id: InstrumentId,
    trade_size: Quantity,
}

nautilus_strategy!(MyStrategy);    // ★宏生成 Deref + Strategy trait 实现

impl DataActor for MyStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.instrument_id, None, None); // bar 用 subscribe_bars
        Ok(())
    }
    fn on_quote(&mut self, quote: &QuoteTick) -> anyhow::Result<()> {  // bar 用 on_bar(&mut self, bar: &Bar)
        let order = self.core.order_factory().market(
            self.instrument_id, OrderSide::Buy, self.trade_size,
            None, None, None, None, None, None, None,
        );
        self.submit_order(order, None, None)?;
        Ok(())
    }
}
```

订单工厂（context7 `orders/limit.md` / `stop_limit.md` 实证）：
- `order_factory().market(instrument_id, side, qty, ...)`
- `order_factory().limit(instrument_id, side, qty, price, Some(TimeInForce::Gtc), None, post_only, reduce_only, ...)`
- `order_factory().stop_limit(instrument_id, side, qty, trigger_price, limit_price, Some(TriggerType::...), ...)`

### 1.4 ★决定性发现：Rust-native IB adapter 已可用（2026-06-27，推翻旧调研前提）

context7 从 develop 分支拿到的**实际代码**（非文档叙述）：

1. `crates/adapters/interactive_brokers/examples/node_exec_tester.rs`：
   ```rust
   let mut node = LiveNode::builder(trader_id, Environment::Live)?
       .with_reconciliation(true)
       .add_data_client(None, Box::new(InteractiveBrokersDataClientFactory::new()), Box::new(data_config))?
       .add_exec_client(None, Box::new(InteractiveBrokersExecutionClientFactory::new(trader_id, account_id)), Box::new(exec_config))?
       .build()?;
   node.add_strategy(ExecTester::new(tester_config))?;   // ★Rust 原生策略
   node.run().await?;
   ```
   context7 注释：**"proving that IBKR live trading is supported from Rust"**。
2. `crates/adapters/interactive_brokers/README.md`：「`nautilus-interactive-brokers` crate provides
   **Rust adapters** ... wraps the `ibapi` client ... **Optional PyO3 bindings** for Python integration.」
3. `RELEASES.md`：「Initial integration adapters added for Coinbase (Rust) and **Interactive Brokers
   (Rust with PyO3 compatibility)**.」

**诚实标注（有效域）**：以上是 develop/nightly 分支证据。WebSearch（文档叙述层）仍概括「IB remains
Python-based」——这是文档滞后于代码的差异。**Rust-native IB 的 release 稳定性未由本工位独立验证**（L0
读码级证据，非 L2 跑通）。这是 §0 边界条件第 1 条的来源。

### 1.5 回测能力（旧调研 §3 + context7）

- 低级 API `BacktestEngine`：手动 `add_venue(SimulatedVenueConfig)` + `add_instrument` + `add_data(bars)`。
- 高级 API `BacktestNode`：Parquet catalog 流式加载（大数据集/参数扫描）。
- Rust 回测（context7 `run_rust_backtest.md`）：
  ```rust
  let mut node = BacktestNode::new(vec![run_config])?;
  node.build()?;
  let engine = node.get_engine_mut("run-id")?;
  engine.add_strategy(MyStrategy::new(...))?;   // ★同一 Strategy 类型，回测=实盘
  node.run()?;
  ```
- venue 配置（context7）：`SimulatedVenueConfig::builder().venue("SIM").oms_type(OmsType::Netting)
  .account_type(AccountType::Margin).starting_balances(vec![Money::from("1_000_000 USD")]).build()`。

### 1.6 Portfolio / 仓位 / PnL（context7 `concepts/strategies.md`）

策略可访问 portfolio：`net_position(instrument_id) -> Decimal`、`unrealized_pnl`/`realized_pnl`、
`is_net_long`/`is_net_short`/`is_flat`。事件 handler：`on_order_filled`、`on_position_opened`/
`on_position_closed`。OMS：**NETTING**（每标的单净仓，期货标准，对齐缠论买卖点→净仓调整）vs HEDGING。

---

## 2. ★与既有调研的冲突标记（谱系：选择类，路径裁定权属编排者）

| 维度 | `docs/nautilustrader_research.md`（2026-06-20） | 本设计（2026-06-27） |
|------|------------------------------------------------|---------------------|
| 推荐路径 | **路径 A（PyO3 Python Strategy）** | **Rust-native Strategy** |
| Rust-native 判定 | **「不推荐」** | **推荐** |
| 决定性理由 | 「v2 Rust 不支持 IB adapter」 | 该前提**已过时**：Rust IB adapter 已在 develop（§1.4 实证） |
| 生产引擎 | 旧 PyO3 缠论引擎（已有 Python 绑定） | canonical S_Θ（`theta_v0`，纯 Rust，编排者新定调） |

**为什么不是矛盾上浮（no-workaround）而是路径选择**：两份调研的差异不是「定义冲突」，而是
**(a) 客观事实更新**（IB adapter 状态 2026-06-20 → 2026-06-27 变化，机器证据可坐实）+
**(b) 上游定调变化**（编排者改用纯 Rust S_Θ 作生产引擎，旧调研针对的是 PyO3 旧引擎）。
两个前提都变 ⟹ 旧调研的结论自然失效，**新前提下 Rust-native 是定理推论**（纯 Rust 引擎 + Rust IB
可用 + 性能最大化定调 ⟹ Rust-native）。

**仍标为「选择」交编排者裁定的原因**：§0 边界条件第 1 条——Rust-native IB 的 **release 稳定性**是
本工位无法用 L0 读码证据结算的风险点（需 L2 实盘 paper trading 验证）。若编排者要求生产**当下**就要
IB 稳定可用，PyO3 路径（v1 legacy IB，旧调研已验稳定）是更保守选择。这是真实的价值判断（性能最大化
vs 稳定性当下可用），属四分法「选择」类，**本工位提供证据不擅自结算**。

---

## 3. S_Θ 侧接口形状（只读了解，本工位不改）

S_Θ 端到端入口（`rust/src/theta_v0/backtest/runner.rs`）：
```
run_theta_v0(dataset, config, years, initial_nav)
  = parse_layer → classify → recognize → plan_and_fill_mtm + run_closed_loop
```
策略族入口（`rust/src/theta_v0/strategy/mod.rs`）：
```
StrategyFamily::pi(param: &ThetaConfig, classification, bars, account) -> Vec<Order>
  = recognize(classification, bars, config)            // H→D：声部决策
  → plan_orders(decisions, bars, account, config)      // D→Order：唯一订单流
```

关键类型（`rust/src/theta_v0/types.rs` + `strategy/mod.rs`）：

| S_Θ 类型 | 字段 | 角色 |
|---------|------|------|
| `Bar` | source_index, timestamp, open/high/low/close (Tick=i64), volume, untradable | 输入 H |
| `VoiceDecision` | depth, root_side(VoiceSide), exit, enter_ok, bsp(BspBits), signal_index, stop_in(StopInput), entry(Tick), cost_per_unit, level | recog 输出 D |
| `Order` | action(StrictAction), qty(i64 lot), exec_index(usize) | π_Θ 输出 |
| `StrictAction` | Buy/Sell/Add/Reduce/Hold/Close/Wait | 动作类 |
| `AccountState` | nav(f64), voice_qty(Vec<u32>) | 账户输入 Z |
| `BspBits` | buy1/2/3, sell1/2/3 (非互斥 bit-vector) | 买卖点标签集 |

**口径关键**（runner.rs:96-98）：价格在整数 tick 域（`px = close as f64 * tick_size` 还原美元）；
NAV 须与品种价格量级匹配（NAV 太小 ⟹ sizing qty=0 不产单）。

---

## 4. 适配架构设计

### 4.1 推荐路径：Rust-native `ThetaStrategy`（适配层结构）

```
┌────────────────────────────────────────────────────────────────────┐
│  Nautilus 引擎（Rust v2 core，回测=实盘单线程确定性）                  │
│                                                                      │
│   DataEngine ──(Nautilus Bar)──► ThetaStrategy::on_bar               │
│                                       │                              │
│              ┌────────────────────────┴────────────────────────┐    │
│              │  适配层 rust/src/theta_v0/nautilus/              │    │
│              │  ① bar_adapter:  Nautilus Bar → S_Θ types::Bar  │    │
│              │     （f64 OHLC → quantize(tick_size) → i64 Tick）│    │
│              │  ② 累积 bar 窗口 → ParseLayer → classify         │    │
│              │     → StrategyFamily::pi(config, cls, bars, acct)│    │
│              │     ⟹ Vec<theta_v0 Order>                        │    │
│              │  ③ order_adapter: S_Θ Order(StrictAction,qty,    │    │
│              │     exec_index) → Nautilus order_factory().limit │    │
│              │     /market/stop（OrderSide + Quantity + Price）  │    │
│              │  ④ account_adapter: portfolio.net_position +     │    │
│              │     NAV → S_Θ AccountState{nav, voice_qty}       │    │
│              └────────────────────────┬────────────────────────┘    │
│                                       ▼                              │
│              core.order_factory() → submit_order                    │
│                  │                                                   │
│   RiskEngine ◄───┘──► ExecutionEngine ──► SimulatedVenue（回测）      │
│                                       └──► IB adapter（实盘 LiveNode）│
│   on_order_filled / on_position_opened/closed ──► account_adapter   │
└────────────────────────────────────────────────────────────────────┘
```

**回测/实盘统一**：同一 `ThetaStrategy` 类型，回测时 `BacktestNode.add_strategy(ThetaStrategy::new(cfg))`，
实盘时 `LiveNode.builder(...).add_exec_client(IB factory).build().add_strategy(ThetaStrategy::new(cfg))`。
零代码切换由 Nautilus 单线程核心保证。

### 4.2 数据流（每 bar）

```
Nautilus Bar(f64 OHLC, ts_event)
  → [①bar_adapter] quantize → theta_v0::types::Bar(i64 Tick, untradable 判定)
  → [②] 追加到 S_Θ bar 窗口 Vec<Bar>
  → parse_layer(bars, cfg) → classify(l0, cfg) → recognize(cls, bars, cfg) → Vec<VoiceDecision>
  → plan_orders(decisions, bars, [④account from portfolio], cfg) → Vec<theta_v0::Order>
  → [③order_adapter] 每个 Order:
        StrictAction::Buy/Add  → OrderSide::Buy,  qty lot → Quantity
        StrictAction::Sell     → OrderSide::Sell (建空/翻空)
        StrictAction::Close/Reduce → reduce_only=true, side=持仓反向
        Hold/Wait → 不下单
     价格：S_Θ entry(Tick) × tick_size → Price（LMT only 策略用 limit；紧急用 market）
  → submit_order
```

### 4.3 接口映射表（S_Θ ↔ Nautilus）

| S_Θ 概念 | Nautilus 对应 | 映射注记 |
|---------|--------------|---------|
| `types::Bar`（i64 Tick OHLC） | `nautilus_model::data::Bar`（Price/Quantity） | quantize/dequantize via tick_size；`untradable` 由 Bar 完整性判（halt/limit 另需 venue 标记） |
| `VoiceDecision.signal_index/exec_index` | bar 序号（策略内累积窗口的索引） | exec 延迟（spec:50 下一根成交）映射为 Nautilus 下一 bar 提交 |
| `Order.action=Buy/Sell/Add` | `OrderSide::Buy/Sell` + `order_factory().limit/market` | 开仓侧 |
| `Order.action=Close/Reduce` | `OrderSide`(反向) + `reduce_only=true` | 平仓侧 |
| `Order.qty`（整数 lot） | `Quantity::from(qty)` × instrument multiplier | lot→合约张数（期货 multiplier 由 instrument 定） |
| `AccountState.nav` | `portfolio.account(venue).balance` 或 net_exposure 推算 | sizing 基数；实盘取真实账户净值 |
| `AccountState.voice_qty[depth]` | `portfolio.net_position(instrument_id)`（depth=0 单声部） | v0 单声部 ⟹ voice_qty[0] = 净仓手数 |
| `StopInput`（结构止损价） | `order_factory().stop_market/stop_limit` trigger_price | 止损腿（S_Θ structural_stop → Nautilus stop 订单，或策略内监控触发 Close） |
| ledger `R=Π−A−W` / TW 守恒 | `portfolio.realized_pnl`/`unrealized_pnl` | S_Θ 双账本是结构不变量（L0）；Nautilus PnL 是经验账本——二者口径对齐由 on_order_filled 回写 |
| OMS | `OmsType::Netting` | 缠论买卖点→净仓调整，对齐单声部 v0 |

### 4.4 状态管理映射

- **持仓**：S_Θ `plan_and_fill_mtm` 内部维护 `cash/units/held[depth]` 台账（回测自洽）。接 Nautilus 后，
  **持仓真相源移交 Nautilus portfolio**（`net_position` + `on_position_opened/closed`），适配层每 bar 从
  portfolio 读真实持仓回填 `AccountState.voice_qty` + 退出生成器的 `held` 台账。这消除 S_Θ 回测台账与
  实盘账本的双源——**Nautilus 是实盘持仓单一真相源**（避免漂移）。
- **退出决策生成器**（runner.rs `exit_decision_for`，§9 closePred）：止损/反向 BSP/RiskClose 触发逻辑
  **保留在适配层**（缠论语义），但触发后产 Close 订单走 Nautilus submit（不再用 S_Θ 内部 fill 模拟）。
- **延迟成交**（spec:50）：S_Θ `fill_bar_index` 的「下一根成交」语义，在 Nautilus 中由 venue 撮合自然
  实现（市价/限价的实际成交由 ExecutionEngine 模拟/真实执行）——适配层**不再自己模拟 fill 价**，
  S_Θ 的 `plan_and_fill_mtm` 的 fill/equity 部分在生产路径**不使用**（只用 recognize+plan_orders 产订单）。

---

## 5. 依赖方案（★大决策，待编排者选路径，本工位不擅自加 Cargo 依赖）

### 5.1 当前 Cargo 现状

`rust/Cargo.toml`：`crate-type = ["cdylib", "rlib"]`，依赖仅 `pyo3 0.23`；`serde/serde_json` 在
dev-dependencies（backtest harness test-only）。`theta_v0::backtest` 模块 `#[cfg(test)]` 门控。

### 5.2 方案 A（推荐，对齐 Rust-native 路径）：独立 binary crate / workspace member

新增 workspace member `rust/nautilus_bridge/`（独立 Cargo package），依赖：
```toml
[dependencies]
newchan_rust = { path = ".." }            # 链接 theta_v0（须把 theta_v0 公开 API 去 test 门控）
nautilus-trading = "..."                  # Rust-native Strategy trait
nautilus-model = "..."
nautilus-backtest = "..."                 # BacktestNode
nautilus-interactive-brokers = "..."      # 实盘 IB（feature gated）
```
**优点**：(a) nautilus 重依赖隔离在独立 crate，**不污染** `newchan_rust` 的 cdylib（Python 扩展）构建；
(b) S_Θ crate 保持轻量（pyo3 only）；(c) Rust-native 零 FFI 开销。
**前置改动**（下游推论 §0.4a）：`theta_v0::backtest`（或新公开适配入口）须从 `#[cfg(test)]` 解门控为
公开 API，让 bridge crate 链接 `recognize`/`plan_orders`/`StrategyFamily::pi`。**此改动属 theta_v0
owner（非本工位），报 Lead 协调。**

### 5.3 方案 B（备选，对齐 PyO3 路径）：现有 cdylib + Python nautilus

S_Θ 经现有 pyo3 暴露 `recognize`/`plan_orders` 到 Python，写 Python `ThetaStrategy(Strategy)` 胶水。
依赖：`pip install nautilus_trader[ib]`（Python 侧，**不动 Cargo.toml**）。
**优点**：零 Cargo 改动；IB v1 legacy 稳定。**缺点**：Python 胶水延迟（1s/1min bar 可忽略）；
S_Θ 须补 pyo3 导出（当前 theta_v0 未接 pyo3）。

### 5.4 版本钉选

Nautilus bi-weekly 发布，API 有 breaking change 风险（旧调研 §6.4 风险5）。**须钉具体版本**
（PyPI 当前 1.228.0；Rust crate 版本待编排者选路径后定）。develop 分支的 Rust IB adapter 须确认进入
某个 tagged release（本工位未独立验证，§0 边界条件第 1 条）。

---

## 6. 骨架文件清单（`rust/src/theta_v0/nautilus/`，标 TODO，不编译）

| 文件 | 职责 | 等级 |
|------|------|------|
| `mod.rs` | 模块入口 + 路径选择文档 + 接口 trait 定义 | L0 设计 |
| `bar_adapter.rs` | Nautilus Bar ↔ S_Θ `types::Bar`（quantize/dequantize） | L0 设计 |
| `order_adapter.rs` | S_Θ `Order`(StrictAction) ↔ Nautilus OrderSide/order_factory 映射 | L0 设计 |
| `account_adapter.rs` | Nautilus portfolio ↔ S_Θ `AccountState`（NAV + voice_qty 回填） | L0 设计 |
| `strategy.rs` | `ThetaStrategy` 适配器骨架（on_bar 串 ①②③④ + 退出生成器接入点） | L0 设计 |

骨架**忠实于 §1.3 真实 Nautilus 接口签名**（不编造），但**不加 `use nautilus_*`**（依赖未加 ⟹ 不编译）——
真实 trait 名/方法名以注释 + TODO 锚定，待依赖加入后填充。这是 no-patch 的「诚实声明能力边界」：骨架
声明结构与职责，**不声明已跑通**。

---

## 7. 认识论诚实标注（formalization-validity-domain 231号）

| 产出 | 等级 | 理由 |
|------|------|------|
| 适配架构设计（§4） | L0 | 从 S_Θ 接口 + Nautilus 接口推导的结构映射，零数据 |
| 接口映射表（§4.3） | L0/L1 | S_Θ↔Nautilus 类型对应（L0）；quantize 往返一致性需 L1 验证（待依赖） |
| Rust-native IB 可用（§1.4） | L0 | develop 分支读码证据，**非 release 稳定性 L2 背书** |
| 骨架（§6） | L0 | 结构 schema，不编译不跑 |
| 真实集成跑通 + 回测指标 | **L2（待验）** | 依赖加入 + 真实数据 + sizing 产非空订单流后才可否证 |

**关键诚实**：本文档**不声称** S_Θ 接 Nautilus 后回测有效/盈利（那是 L2/L3，编排者纲领明确
「不证明 Θ 是好 Θ」）。本文档只声称**适配层结构忠实于双侧真实接口**（L0 设计正确性）。
