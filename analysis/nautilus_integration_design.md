# NautilusTrader × Rust 缠论引擎集成设计

> 日期：2026-06-12
> 状态：设计文档（L0——纯架构推导 + 文档调研，零运行验证；所有"可行"声明在 L2 实测前均为待验证）
> 上游：`docs/quant-platform-research-2026.md`（平台选型）、`analysis/notional_exposure_leverage_research.md`（杠杆框架）、`analysis/maker_fill_rate_feasibility.md`（maker 执行定型）
> 调研基底：NautilusTrader 1.228.0（beta，2026-06-08），文档 latest + develop 分支源码逐字提取

---

## 0. 三个结构性判决（先于一切设计细节）

设计开始前必须钉死三个张力，否则后面所有代码结构都是错的：

### 判决一：撮合口径——Nautilus 默认是我们守卫要防的乐观口径

Nautilus L1 回测中限价单默认 `prob_fill_on_limit = 1.0`（touch 即成交，队首假设）。
本仓库 CL maker 执行判决（在册）：**touch 模式限价回测必须过 next 模式才可采纳**——BRN 的现实成交归零教训正是 wick 伪影。

**判决**：所有 Nautilus 回测 venue 配置强制：

```python
fill_model = FillModel(
    prob_fill_on_limit=0.0,   # 队列末位：只有价格穿越（trade through）限价位才成交
    prob_slippage=0.0,
)
```

`prob_fill_on_limit=0.0` 对应我们的"下界口径"（队列末位穿越，30s 窗口 fill rate 66.6%）。
乐观口径（=1.0）只允许作为对照臂出现，且两口径数字**不可混表**（与 fusion_u 口径声明同纪律）。

### 判决二：账本真相源——venue 是真账本，OrganicLedger 降级为策略意图状态机

现有交易层"信号→账本直接更新"在回测中合法（账本=世界）。实盘中**世界在交易所那边**：
订单可能被拒、部分成交、撤单失败。如果 OrganicLedger 继续假装自己是真账本，声明与实际不一致（090号声明膨胀）。

**判决**（严格形式，不是 workaround）：
- **真账本** = venue 账户状态，经 Nautilus `Portfolio` + reconciliation 对账机制镜像到本地。
- **OrganicLedger** = 策略意图状态机：它回答"按缠论结构，现在*应该*持有什么"，
  输出的是**目标敞口（target exposure）**，不是成交事实。
- 两者之间的差 = 待执行订单意图，由执行层（maker 挂单状态机）收敛。
- 这要求 Rust 交易层新增**意图输出模式**（§4），不是在现有代码上打补丁，是补全实盘必需的存在层级。

### 判决三：时间轴归属——引擎零时间戳是特性，时间映射全部住在桥接层

引擎 `process_bar(o,h,l,c)` 无时间戳、要求 bar 序列 gap-free（bar index = 隐式时间轴，
"时间不是缠论量"的工程体现）。Nautilus bar 带纳秒 `ts_event`、live 流可能有缺口/重叠
（历史预热与实时订阅之间引擎不做去重承诺）。

**判决**：时间戳↔bar_index 的双向映射、单调性守卫、gap 检测，全部集中在桥接层
`ChanlunBridge` 一处实现（§3.3）。引擎和 Strategy 都不碰时间换算。违反单调性
（`ts_event` 倒退或重复）= fail-fast 抛异常，不静默跳过。

---

## 1. 架构概览与组件映射

### 1.1 NautilusTrader 核心组件 ↔ 现有系统对应

| NautilusTrader | 职责 | 现有系统对应物 | 集成关系 |
|---|---|---|---|
| `DataEngine` + adapters | 行情接收、按订阅分发 | analysis/*.py 的数据加载代码 | **替换**：Databento/IBKR/Binance adapter 接管 |
| `Strategy`（⊂ `Actor`） | 订阅数据、产生订单 | 无直接对应（新写） | **新增**：`ChanlunFugueStrategy`，薄壳 |
| `RecursiveOrchestrator`（我们的） | K线→笔→段→中枢→走势→BSP | rust/src/lib.rs:852 | **保留**：Strategy 内通过桥接层调用 |
| 交易层 voice 状态机（我们的） | fusion/hold26/positional 决策 | rust/src/trading/runner.rs, positional_fusion.rs | **改造**：新增流式意图模式（§4） |
| `OrderFactory` + `submit_order` | 订单创建与提交 | 无（回测直接改账本） | **新增**：意图→LimitOrder 翻译 |
| `ExecutionEngine` + exec adapters | 订单路由、状态追踪、成交回报 | 无 | **采用** |
| `Portfolio` + `Cache` | 仓位/账户真相镜像 | OrganicLedger（降级，见判决二） | **采用**为真账本 |
| `RiskEngine` | 交易前风控 | L_max 杠杆钳制（外层） | **采用** + 自有 L_max 计算器叠加 |
| `BacktestNode` / `TradingNode` | 回测/实盘容器 | analysis/ 下各回测脚本 | **采用**：同一 Strategy 双模式 |
| `FillModel` / `LatencyModel` | 回测撮合假设 | 回测脚本内 touch/next 双模式 | **采用**，按判决一钉死保守口径 |

关键事实（调研逐字确认）：
- Strategy 继承 Actor，同一 Strategy 类回测/实盘**零修改**（官方逐字："The same actors,
  strategies, and execution algorithms run against both the backtest engine and a live trading node."）
- kernel 单线程确定性消费消息——与引擎的逐 bar 确定性语义兼容。
- 同进程禁止多个 TradingNode（全局单例）——IBKR + Binance 必须并联在**单 node** 内。
- Price/Quantity 是 i128 定点数（精度16）——无浮点漂移，利好 bit-exact 验证文化。

### 1.2 三层架构

```
┌─────────────────────────────────────────────────────────────┐
│  NautilusTrader 层（Python）                                  │
│  TradingNode/BacktestNode · DataEngine · ExecEngine ·        │
│  Portfolio · RiskEngine                                      │
│       │ on_bar(Bar)              ▲ submit_order(LimitOrder)  │
├───────┼──────────────────────────┼───────────────────────────┤
│  桥接层（Python，新写，唯一的胶水）                              │
│  ChanlunFugueStrategy(Strategy)  ── 生命周期、订阅、订单事件     │
│  ChanlunBridge                   ── 时间映射、gap守卫、          │
│                                     Bar→OHLC f64、意图→订单     │
│  MakerExecutor                   ── 挂单→等候→撤单状态机         │
│  LeverageGovernor                ── L_max = 1/(D_struct+mm)   │
│       │ process_bar(o,h,l,c)     ▲ intents / signals          │
├───────┼──────────────────────────┼───────────────────────────┤
│  Rust 引擎层（newchan_rust，PyO3）                             │
│  RecursiveOrchestrator           ── 结构计算（不动）            │
│  PositionalStream（新增）         ── 交易层流式意图模式（§4）     │
│  OrganicLedger                   ── 影子账本/意图状态机          │
└─────────────────────────────────────────────────────────────┘
```

设计原则：**Nautilus 不知道缠论，引擎不知道 Nautilus**。两者只通过桥接层的两种原语耦合：
下行 `(open, high, low, close)`，上行 `TargetExposure` 意图。

---

## 2. 数据接入设计

### 2.1 标的与数据源矩阵

| 标的域 | 历史 | 实时 | Nautilus instrument_id（待 L2 确认） | bar 来源 |
|---|---|---|---|---|
| CME 期货（ES/GC/CL/ZN/6E/DX） | Databento DBN（ohlcv-1s/1m，CME 回溯 2010） | Databento Live（$179/月）或 IBKR | `ES.v.0.GLBX`（连续，成交量滚动）/ 单月 `ESM6.XCME` | EXTERNAL（Databento 已聚合） |
| ICE（BRN） | Databento | IBKR | 待确认 venue MIC | EXTERNAL |
| 美股（OKLO/QQQ/PINS） | Databento XNAS.ITCH | IBKR | `OKLO.XNAS` | EXTERNAL |
| 加密（BTC） | Binance 归档（已有 4.6M bar 管线） | Binance Futures WS | `BTCUSDT-PERP.BINANCE` | 1m EXTERNAL；**1s 必须 INTERNAL**（见下） |

**Binance futures 1s 判决**：交易所侧 1s kline 仅 Spot 提供，USD-M Futures kline 从 1m 起。
futures 秒级路径 = 订阅 aggTrade（TradeTick）→ Nautilus 内部聚合：
`BTCUSDT-PERP.BINANCE-1-SECOND-LAST-INTERNAL`。此结论需 testnet 实测收口（L0→L2 缺口）。

**Databento 时间戳口径**（逐字确认）：DBN bar 原始 `ts_event` 在 open 时刻，Nautilus
decoder 归一化到 **close 时刻**——零前视，与零前视回测纪律一致，无需额外处理。

### 2.2 历史数据：现有数据 → ParquetDataCatalog

**迁移成本点（调研确认）**：本仓库现有 databento 10y 数据是 opens 数组格式（自有管线落盘），
**不是 DBN**，不能直接喂 `DatabentoDataLoader`。两条路：

- **路 A（首选）**：重下 DBN 原始文件（Databento 历史额度内），`DatabentoDataLoader.from_dbn_file()`
  → `catalog.write_data()`。顺带补拉 `schema=definition`（catalog 必须先有 instrument
  definitions，否则 `catalog.instruments()` 为空）。
- **路 B（fallback，仅当额度不够）**：自写 `Bar` 构造器把现有数组转 Nautilus Bar 对象写 catalog。
  风险：精度/字段口径自己负责，与路 A 产物**不可混表**。

BTC Binance 归档：无 DBN 可言，恒走路 B（数组→Bar 构造器），已有 fetch_btc_binance_full.py
管线下游加一个 catalog writer 即可。

### 2.3 历史+实时拼接（warm-up）

官方三步模式（`on_start` 内，顺序强制）：

```python
def on_start(self):
    # 1) 注册（我们没有 Nautilus 指标，跳过 register_indicator——引擎预热走自己的路径）
    # 2) 请求历史 → on_historical_data() 回调
    self.request_bars(self.bar_type, start=self.clock.utc_now() - WARMUP_SPAN)
    # 3) 订阅实时 → on_bar() 回调
    self.subscribe_bars(self.bar_type)
```

**缠论引擎的特殊性**：路径依赖状态机，预热不是"填指标窗口"而是**重放全史**（笔/段/中枢
结构取决于完整序列）。官方模式只保证指标无缝，**不保证历史/实时之间无重叠无缺口**。

预热协议（桥接层实现，严格形式）：

1. **冷启动重放**：`on_start` 先从本地 catalog 读全史 bars（不走 `request_bars`，量太大），
   逐 bar 喂引擎，记录最后 `ts_event` 为水位线 `watermark`。
2. **缺口补齐**：`request_bars(start=watermark)` 拉 catalog 末尾到现在的增量，
   `on_historical_data` 中喂引擎，推进水位线。
3. **实时衔接**：`on_bar` 中，`ts_event ≤ watermark` 的 bar 丢弃（重叠），
   `ts_event` 跳变超过一个 bar 间隔的记录 gap 计数（不填充——与现有"记录消除不填充"约定一致），
   合法 bar 喂引擎并推进水位线。
4. **重放期间禁止下单**：预热完成前 `self._warmed_up = False`，所有意图只更新影子账本不出订单；
   预热完成后先做一次**意图↔Portfolio 对账**（§4.4），再放开下单。

5M bar 级重放耗时：引擎 Rust 侧已知 ~30min/标的量级（O(N²) 残余），1m bar 全史可接受；
1s 全史重放不可接受 → 1s 床位用"checkpoint + 增量重放"（引擎状态序列化是开放工程项，
短期内 1s 床位限制重放窗口或接受冷启动成本，见 §8 风险清单）。

---

## 3. 桥接层设计（核心代码结构）

### 3.1 文件结构

```
nautilus_chanlun/                    # 新 Python 包（仓库内）
├── __init__.py
├── strategy.py        # ChanlunFugueStrategy(Strategy) —— Nautilus 生命周期壳
├── bridge.py          # ChanlunBridge —— 时间映射/gap守卫/Bar→f64/引擎驱动
├── intents.py         # TargetExposure / OrderIntent 数据类型（不可变）
├── executor.py        # MakerExecutor —— LMT挂单→等候→撤单状态机
├── leverage.py        # LeverageGovernor —— L_max = 1/(D_struct + mm)
├── configs.py         # ChanlunStrategyConfig(StrategyConfig)
└── catalog_tools/     # DBN→catalog / 数组→Bar 构造器（一次性迁移工具）
```

### 3.2 多标的拓扑：每标的一个 Strategy 实例

两种官方模式中选**多实例**（每实例一标的，`order_id_tag` 区分）：

- 各标的的 voice 配置本来就按白名单分裂（fusion_t 白名单 / hold26 正域 / BH 域——
  regime 函数九例的在册结论），每实例独立配置变体是自然表达；
- 引擎实例本就按标的隔离（一个 RecursiveOrchestrator 只吃一个序列）；
- 标的间无跨标的耦合（当前所有在册策略都是单标的语法）。

Portfolio 天然跨实例聚合（`net_exposures()` 按币种分组），账户级风控在 RiskEngine +
LeverageGovernor 层做，不需要标的间通信。

### 3.3 ChanlunBridge：唯一的时间/格式翻译点

职责（全部集中此处，引擎与 Strategy 零时间逻辑）：

```python
class ChanlunBridge:
    """引擎驱动 + 时间轴守卫。一个实例绑定一个标的一个床位（bar 周期）。"""

    def __init__(self, engine_config: dict):
        self._orch = newchan_rust.RecursiveOrchestrator(**engine_config)
        self._watermark_ns: int | None = None   # 最后接受的 ts_event
        self._bar_index: int = -1               # 引擎隐式时间轴
        self._ts_to_index: list[int] = []       # bar_index → ts_event_ns（订单意图回溯锚定用）
        self._gap_count: int = 0

    def feed(self, bar) -> FeedResult:
        ts = bar.ts_event
        if self._watermark_ns is not None:
            if ts <= self._watermark_ns:
                return FeedResult.DUPLICATE      # 历史/实时重叠，丢弃
            # gap 只记录不填充（与现有约定一致）
            if self._expected_next_ns and ts > self._expected_next_ns:
                self._gap_count += 1
        self._orch.process_bar(
            bar.open.as_double(), bar.high.as_double(),
            bar.low.as_double(),  bar.close.as_double(),
        )
        self._bar_index += 1
        self._ts_to_index.append(ts)
        self._watermark_ns = ts
        return FeedResult.ACCEPTED
```

要点：
- `Price.as_double()` 一次性转 f64——引擎吃 f64，定点→浮点只在边界发生一次；
- 时间戳单调性违规 fail-fast（重复=丢弃可计数；倒退=抛异常，说明上游数据流坏了）；
- `_ts_to_index` 保留 bar_index↔ts 映射，意图/诊断回溯时能锚定真实时间。

### 3.4 信号消费：复用现有 delta 接口

引擎已暴露完整 O(Δ) 增量接口（B-scaling 已消除，直接复用）：

| 接口 | 用途 | 复杂度 |
|---|---|---|
| `trend_new_signals(level_id)` | (buy1, sell1, sell_any, buy_any, type2_buy) 布尔门控 | O(1)（epoch 未变直接 false） |
| `take_trend_bsp_events()` | BSP delta 事件流（kind/side/confirmed/zd/zg/price） | O(Δevent) |
| `take_trend_div_events()` | 背驰 delta 事件 | O(Δevent) |
| `take_move_settle_events()` | move 结算事件 | O(Δevent) |
| `move_epoch() / bsp_epoch()` | 内容变化纪元门控 | O(1) |

每 bar 调用模式 = 现有 m1_e_rust_engine.py 的 epoch 门控模式原样平移，无新接口需求。

---

## 4. 交易层改造：流式意图模式（关键工程项）

### 4.1 现状与缺口

现状（internal survey 确认）：
- `run_positional_rust(tape, floor_ladder, mode)` 吃**整条 OrganicTape 离线批跑**；
- voice 状态机决策直接调用 `ledger.open_diff()/close_diff()` 更新账本，
  **无可拦截的订单意图层**；
- `OrganicTape.from_columns()` 一次性列式构造，专为消除逐 bar FFI 开销设计。

实盘需要的是：逐 bar 推进 voice 状态机 → 产出意图 → 外部执行 → 回报确认后才更新账本。
每 bar 重建整条 tape 重跑是 O(N²) 且语义错误（重跑会重放历史成交），**不是选项**。

### 4.2 严格形式：`PositionalStream`

在 `rust/src/trading/` 新增流式容器（不改 runner.rs 的批语义——批模式继续服务回测对照，
两条路径用同一 voice 状态机代码，分叉只在驱动方式与账本写入时机）：

```rust
// rust/src/trading/stream.rs（新增）
#[pyclass(name = "PositionalStream")]
pub struct PositionalStream {
    voices: Vec<VoiceUnit>,        // 与 runner.rs 同一状态机类型
    ledger: OrganicLedger,         // 影子账本（意图状态机，见判决二）
    cfg: OrganicConfig,
    pending: Vec<OrderIntent>,     // 已发出未确认的意图
}

#[pymethods]
impl PositionalStream {
    /// 喂入一行磁带（= from_columns 的单行），返回本 bar 产生的意图列表。
    /// 不更新影子账本——账本只在 confirm_fill 时更新。
    fn on_bar_row(&mut self, row: TapeRow) -> Vec<OrderIntentTuple>;

    /// 执行层回报：成交确认 → 影子账本按真实成交价/量更新。
    fn confirm_fill(&mut self, intent_id: u64, fill_price: f64, fill_qty: f64, bar: usize);

    /// 执行层回报：撤单/拒单 → 意图作废，voice 状态机回滚到挂单前。
    fn reject_intent(&mut self, intent_id: u64);

    /// 当前目标敞口（影子账本视角），供对账。
    fn target_exposure(&self) -> f64;
}
```

`OrderIntent`（PyO3 元组）：

```
(intent_id: u64, side: "buy"|"sell", qty_frac: f64,    # 名义配额比例
 anchor_price: f64,                                     # 结构锚定价（限价基准）
 voice_key: (ladder, slot),                             # 归属 voice（名义敞口独立跟踪）
 reason: &'static str)                                  # "buy1_open"/"trim"/"recover"/...
```

设计要点：
- **意图≠成交**：voice 状态机发出意图后进入 `PENDING` 态，confirm 前不再对同 slot 发新意图
  （防止重复开腿）；reject 回滚（与 maker"撤单不追价"对接——撤单 = reject = 该意图永久作废，
  voice 等待下一个结构事件，不追）。
- **回测等价守卫**：批模式（runner.rs）与流式模式（stream.rs）在"理想成交"假设下
  （confirm_fill 用意图价立即回报）必须 **bit-exact**——这是新增代码的预注册验收判据，
  也是两条路径共享 voice 状态机代码的强制理由。
- **per-voice 名义敞口**：`legs: Vec<(SlotKey, OpenLeg)>` 已按 voice 隔离，意图带 voice_key
  即满足"嵌套递归赋格下各 voice 名义敞口独立跟踪"，无需新结构。

### 4.3 磁带行实时构造

`TapeRow` = `from_columns` 列的单 bar 切片。Python 侧每 bar 从引擎 delta 接口读出
（bsp 事件、div 事件、dir_flips、trend_flips、max_ladder…）组装成行——
这正是现有磁带打包代码的逐 bar 版本，逻辑平移，无新概念。

### 4.4 对账（reconciliation）

两级对账，缺一不可：

1. **Nautilus ↔ venue**：TradingNode 启动时内建（`generate_order_status_reports` /
   `generate_fill_reports` / `generate_position_status_reports`），采用即可。
2. **影子账本 ↔ Portfolio**：桥接层周期性（每 bar 或定时器）比对
   `stream.target_exposure()` 与 `portfolio.net_position(instrument_id)`。
   偏差超容差（一个最小交易单位）→ 告警 + 停止新意图（不自动"纠偏下单"——
   偏差意味着状态机与现实脱钩，自动纠偏是把诊断问题变成新的盲目交易）。

---

## 5. 执行层设计

### 5.1 LMT-only 硬约束

- 桥接层 `MakerExecutor` 是**唯一**调用 `order_factory` 的地方，且只调用 `order_factory.limit()`。
- 防御深度：RiskEngine 之外加一层本地断言——任何 `OrderType != LIMIT` 的订单在
  submit 前抛异常（不是过滤，是 fail-fast：出现 MKT 单意味着代码有 bug）。
- Binance 侧可叠加 `post_only=True`（taker 即拒单，交易所级保证 maker）；
  IBKR 无 post-only 等价物，靠限价价位选择（挂 passive 侧）保证。

### 5.2 maker 状态机（在册判决的直接实装）

在册定型（maker_fill_rate_feasibility）：**限价挂单 ≤60s 未成交则撤单，不追价**；
撤单优于追价 27pp；fill rate 下界 66.6%（30s）/74.4%（60s）已过相对 alpha 前沿。

```
IDLE ──intent──▶ PLACED(LMT @ anchor_price, 启动60s定时器)
PLACED ──on_order_filled──▶ confirm_fill → IDLE
PLACED ──定时器到期──▶ cancel_order → CANCELING
CANCELING ──on_order_canceled──▶ reject_intent（不追价，意图作废）→ IDLE
PLACED ──on_order_rejected──▶ reject_intent → IDLE（计数告警）
部分成交：on_order_filled(partial) → confirm_fill(部分量)；到期撤余量 → 余量 reject
```

定时器用 Nautilus `clock.set_time_alert()`（回测/实盘同一时钟抽象，回测中定时器
按模拟时间触发——撤单逻辑在回测中同样生效，这是用 Nautilus 撮合层的核心收益：
**60s 撤单窗口第一次可以在回测里被忠实模拟**，现有自研回测做不到这一点）。

### 5.3 LeverageGovernor：L_max = 1/(D_struct + mm)

在册框架（notional_exposure_leverage_research，L0）：

- `D_struct(t) = (c − P_neg)/c + θ_q(j−1)`；P_neg 由相位定义（MOVE↑→ZG_new；OSC→ZD）；
  mm = venue 维持保证金率（instrument 元数据，Nautilus `Instrument.margin_maint` 可读）。
- 每 bar 引擎侧已能输出 D_struct 所需结构量（中枢 ZG/ZD、相位）——P6 相位机已实装
  （PhaseView 三值接口），桥接层组装即可。
- 仓位钳制：`qty = min(intent.qty_frac × equity / price, L_max × equity / price − current_exposure)`。
- 恒仓极性（hold26）标的：L_max ≈ 1.0–1.3x ⇒ BTC 主仓**现货或 1x 永续逐仓**
  （adapter 级表达：`futures_leverages={"BTCUSDT": 1}` + `futures_margin_types={"BTCUSDT": "isolated"}`）。

LeverageGovernor 在 MakerExecutor 之前钳制每个意图的量，是纯函数层（无状态，
每 bar 输入结构量输出上限），独立可测。

---

## 6. 分步实施计划

每阶段有预注册验收判据，过门才进下一阶段。阶段内完整交付，不留半成品。

### 阶段 0：环境与数据基底（~1周）

- [ ] `pip install -U "nautilus_trader[docker,ib]"`（Python 3.12+ venv，注意 `.local/bin/cc`
      劫持问题——构建 Rust wheel 时已知陷阱，前置 /usr/bin）
- [ ] 重下 1-2 个标的的 DBN（CL ohlcv-1m + definition），建 ParquetDataCatalog
- [ ] BTC 数组→Bar 构造器，写 catalog
- **验收**：`catalog.instruments()` 非空；catalog 读出的 bar 与现有数组逐 bar 数值一致（抽样对账）

### 阶段 1：信号层贯通——只读 Strategy（~1-2周）

- [ ] `ChanlunBridge` + `ChanlunFugueStrategy`（只订阅、喂引擎、log 信号，**不下单**）
- [ ] BacktestNode 跑 CL 1m 全史
- **验收（核心：双引擎对账）**：Nautilus 驱动下引擎产出的 BSP/div 事件序列与现有
  自研管线（同数据）**bit-exact**——这验证时间映射、gap 守卫、f64 转换全链路无损。
  不一致 = 桥接层有 bug，过门前必须归零。

### 阶段 2：流式交易层（~2-3周，最大工程项）

- [ ] Rust `PositionalStream`（§4.2）+ PyO3 暴露
- [ ] 批↔流 bit-exact 守卫测试（理想成交假设下与 `run_positional_rust` 全量对账，
      至少 fusion_t + hold26 两个 mode × 2 标的）
- **验收**：守卫测试绿；流式模式 8 标的在册数字复现（同口径）

### 阶段 3：Nautilus 回测闭环（~2周）

- [ ] MakerExecutor + LeverageGovernor + intents 接 BacktestNode
- [ ] FillModel 双口径跑：保守（prob=0.0）+ 对照（prob=1.0），与在册 touch/next 双模式
      数字对照（量级一致性检查，不要求逐笔相等——撮合模型不同）
- **验收**：保守口径下策略存活（在册判决：现实成交归零的标的如 BRN wick 伪影
      应在此口径下显形）；60s 撤单逻辑在回测中可观测（撤单计数 > 0）

### 阶段 4：纸交易（~2-4周）

- [ ] TradingNode + IBKR paper（port 7497/4002）单标的 CL
- [ ] Binance testnet BTC（1x 逐仓）
- [ ] 预热协议（§2.3）实测：冷启动重放 + 水位线衔接
- [ ] 影子账本↔Portfolio 对账告警实测（人为制造偏差验证告警触发）
- **验收**：连续运行 ≥5 个交易日无对账偏差、无未处理异常；fill rate 实测值落入
      [66.6%, 92.1%] 预测区间（落外 = maker 可行性判决需要重审）

### 阶段 5：单标的实盘 → 多标的（门控推进）

- [ ] CL 单标的最小名义额实盘
- [ ] 多实例扩展（白名单标的逐个加入，每标的独立 `order_id_tag`）
- [ ] 部署迁移：本地 Mac → VPS（§7）

---

## 7. 部署架构

### 7.1 本地 Mac（阶段 0-4）vs VPS（阶段 5+）

| 维度 | 本地 Mac | VPS（推荐 NY4/芝加哥区域 Linux） |
|---|---|---|
| 适用阶段 | 开发/回测/纸交易 | 实盘 |
| IBKR 接入 | TWS 桌面（7497/7496） | dockerized IB Gateway（`gnzsnz/ib-gateway-docker`，Nautilus 官方 `DockerizedIBGateway` 封装） |
| 风险 | 睡眠/重启/jetsam 杀进程（已有前科）、住宅网络 | 需自管安全（密钥、防火墙） |
| 延迟 | 不敏感（maker 判决：co-location 非必需，30-60s 等候窗口） | 同左，VPS 价值在**可用性**不在延迟 |

maker 判决已裁定 co-location 非必需，VPS 选择标准是稳定性与运维便利，不是延迟竞赛。

### 7.2 进程管理与监控

- 单进程单 TradingNode（全局单例约束），`systemd` 单元管理（`Restart=on-failure` +
  `RestartSec` 退避）；Mac 阶段用 `launchd` 或显式前台运行（不依赖 nohup——
  serena/后台任务的已知陷阱）。
- 监控三层：
  1. Nautilus 自带结构化日志（JSON，按 trader_id 分文件）；
  2. 对账偏差/拒单计数/gap 计数 → 自有告警通道（起步：本地日志 + 退出码；
     加固：Telegram bot 或邮件）；
  3. 心跳：定时器写 watermark 文件，外部 cron 检查时效性。

### 7.3 断线重连与状态恢复

- 行情/执行连接重连：adapter 内建（Databento `reconnect_timeout_mins=10`；
  IBKR/Binance adapter 自动重连），采用。
- **引擎状态恢复**（关键缺口）：引擎是路径依赖状态机，进程死 = 结构状态丢失。
  恢复路径 = 冷启动重放（§2.3）。1m 床位重放分钟级可接受；崩溃重启期间
  **不持有未管理仓位**——重启序列：先对账 venue 仓位 → 重放至水位线 →
  影子账本对齐检查 → 通过后才恢复意图产生。对不齐 = 人工介入，不自动强平。
- 订单层恢复：Nautilus reconciliation 内建（启动时拉 venue 订单/成交/仓位报告对齐 Cache）。

### 7.4 日志与审计

- 每个意图全生命周期落盘：intent → order → fill/cancel 链路（intent_id ↔
  client_order_id 关联），Parquet 追加写。
- 引擎结构快照：每日收盘后落盘 strokes/zhongshus/moves 序列化（审计 + 重放加速 checkpoint 的前置）。
- 回测/实盘同一日志 schema——审计工具一套两用。

---

## 8. 第一步代码骨架

阶段 1 的最小可运行骨架（只读信号，不下单）：

```python
# nautilus_chanlun/configs.py
from nautilus_trader.config import StrategyConfig

class ChanlunStrategyConfig(StrategyConfig, frozen=True):
    instrument_id: str          # 如 "CL.v.0.GLBX"
    bar_type: str               # 如 "CL.v.0.GLBX-1-MINUTE-LAST-EXTERNAL"
    engine_max_levels: int = 6
    engine_stroke_mode: str = "wide"
    trading_mode: str = "fusion_t"     # config.rs 变体名
    floor_ladder: int = 2
    warmup_catalog_path: str | None = None   # 冷启动重放数据源
```

```python
# nautilus_chanlun/strategy.py
from nautilus_trader.trading.strategy import Strategy
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.identifiers import InstrumentId

from .bridge import ChanlunBridge, FeedResult
from .configs import ChanlunStrategyConfig


class ChanlunFugueStrategy(Strategy):
    """缠论赋格策略壳：生命周期 + 订阅 + 意图→订单翻译。

    阶段1形态：只喂引擎、记录信号事件。下单路径（MakerExecutor）阶段3接入。
    """

    def __init__(self, config: ChanlunStrategyConfig) -> None:
        super().__init__(config)
        self.instrument_id = InstrumentId.from_str(config.instrument_id)
        self.bar_type = BarType.from_str(config.bar_type)
        self.bridge = ChanlunBridge(
            engine_config=dict(
                max_levels=config.engine_max_levels,
                stroke_mode=config.engine_stroke_mode,
            ),
        )
        self._warmed_up = False

    def on_start(self) -> None:
        # 1) 冷启动重放（catalog 全史 → 引擎），见设计 §2.3
        if self.config.warmup_catalog_path:
            n = self.bridge.replay_catalog(
                self.config.warmup_catalog_path, self.bar_type,
            )
            self.log.info(f"replayed {n} bars, watermark={self.bridge.watermark}")
        # 2) 缺口补齐：catalog 末尾 → 现在
        self.request_bars(self.bar_type, start=self.bridge.watermark_dt())
        # 3) 实时订阅
        self.subscribe_bars(self.bar_type)

    def on_historical_data(self, data) -> None:
        if isinstance(data, Bar):
            self.bridge.feed(data)

    def on_bar(self, bar: Bar) -> None:
        result = self.bridge.feed(bar)
        if result is not FeedResult.ACCEPTED:
            return                       # 重叠丢弃 / gap 已计数
        if not self._warmed_up:
            if self.bridge.caught_up(self.clock.utc_now()):
                self._warmed_up = True
                self.log.info("warm-up complete, signal phase live")
            return
        # —— 信号消费（O(Δ) delta 接口，epoch 门控）——
        for ev in self.bridge.drain_bsp_events():
            self.log.info(f"BSP {ev}", color=2)
        for ev in self.bridge.drain_div_events():
            self.log.info(f"DIV {ev}")
        # 阶段2+：row = self.bridge.tape_row(); intents = self.stream.on_bar_row(row)
        # 阶段3+：for it in intents: self.maker.execute(it)

    def on_stop(self) -> None:
        self.cancel_all_orders(self.instrument_id)   # 阶段3+生效；阶段1为空操作
        self.log.info(f"gaps={self.bridge.gap_count}")
```

```python
# 阶段1回测入口（BacktestNode 高层 API）
from nautilus_trader.backtest.node import BacktestNode
from nautilus_trader.config import (
    BacktestRunConfig, BacktestEngineConfig, BacktestDataConfig,
    BacktestVenueConfig, ImportableStrategyConfig,
)

run_config = BacktestRunConfig(
    engine=BacktestEngineConfig(strategies=[
        ImportableStrategyConfig(
            strategy_path="nautilus_chanlun.strategy:ChanlunFugueStrategy",
            config_path="nautilus_chanlun.configs:ChanlunStrategyConfig",
            config=dict(
                instrument_id="CL.v.0.GLBX",
                bar_type="CL.v.0.GLBX-1-MINUTE-LAST-EXTERNAL",
                trading_mode="fusion_t",
            ),
        ),
    ]),
    data=[BacktestDataConfig(
        catalog_path="./catalog", data_cls="nautilus_trader.model.data:Bar",
        instrument_id="CL.v.0.GLBX",
    )],
    venues=[BacktestVenueConfig(
        name="GLBX", oms_type="NETTING", account_type="MARGIN",
        base_currency="USD", starting_balances=["1_000_000 USD"],
        # 阶段3接入订单后：fill_model 强制保守口径（设计 §0 判决一）
    )],
)
results = BacktestNode(configs=[run_config]).run()
```

---

## 9. 风险与待解决问题清单

按"翻转设计的严重度"排序：

| # | 风险/缺口 | 等级 | 触发后果 | 处置 |
|---|---|---|---|---|
| R1 | **NautilusTrader 1.x 是 beta**，官方明示 releases 间有 breaking changes（弃用流程要等 Rust 移植完成 2.x 定型） | 高 | 升级即破坏；桥接层 API 漂移 | 锁定版本（pin 1.228.0），升级走显式迁移评审；桥接层是唯一接触面，破坏半径受控 |
| R2 | **批↔流 bit-exact 是阶段 2 过门判据**，voice 状态机若有隐式全 tape 依赖（如向后看的归因统计）流式化会暴露 | 高 | 阶段 2 工期膨胀或发现状态机语义需重述 | 预注册判据已定；若发现真实语义依赖未来数据 = 定义冲突，走矛盾上浮不绕过 |
| R3 | **引擎无状态序列化**：崩溃恢复 = 全史重放；1s 床位重放成本不可接受 | 中（1m 床位）/ 高（1s 床位） | 1s 床位实盘恢复窗口过长 | 1m 床位先行；checkpoint 序列化作为独立工程项立项（结构快照已有审计需求，复用） |
| R4 | **回测 fill 语义 ≠ 在册自研回测**：FillModel 概率撮合 vs 自研 touch/next 确定性双模式，数字不可逐笔对账 | 中 | 阶段 3 验收只能做量级一致性，不能 bit-exact | 接受；在册数字与 Nautilus 数字永久双口径标注，禁止混表 |
| R5 | Binance futures 1s 必须 INTERNAL 聚合（aggTrade→1s bar），聚合口径与交易所 1m kline 的对账未验证 | 中 | BTC 1s 床位数据口径漂移 | testnet 实测：INTERNAL 1s 聚合出的 1m 与 EXTERNAL 1m kline 对账 |
| R6 | IBKR 历史/实时限制：pacing violation、行情线 ~100 条、秒级 bar 实务下限 ~5s | 中 | 多标的扩展受行情线约束；1s 床位不能靠 IBKR 数据 | 期货行情走 Databento Live；IBKR 只做执行+对账 |
| R7 | `ES.v.0.GLBX` 等连续合约 instrument_id 形式未实测；**连续合约不可交易**，执行须映射到当前主力单月合约 | 中 | 信号标的≠下单标的，需滚动映射层 | 桥接层加 signal_instrument→exec_instrument 映射；滚动日处理规则待阶段 4 定 |
| R8 | Python 每 bar 回调开销官方无数字（推断微秒级） | 低（1m）/ 中（1s 多标的） | 1s × 多标的吞吐不足 | 阶段 1 顺带 benchmark；超限则信号层下沉 Rust（Nautilus v2 Rust API 路径预留） |
| R9 | 多币种账户收益分析静默降级（Nautilus 文档逐字："falls back to position returns silently"） | 低 | 绩效报表口径错 | 报表按 venue 分账户出，不用跨币种聚合序列 |
| R10 | DBN 重下载额度 / 现有数组数据不能直接喂 DatabentoDataLoader | 低 | 阶段 0 成本 | 路 A 优先，路 B fallback（§2.2），两路产物不混表 |

**待编排者裁决项**：无——本设计在已结算判决（平台选型、maker 定型、杠杆框架、撮合口径守卫）
的约束下推导，未遇到定义冲突。R2 是潜在冲突点，触发时上浮。

---

## 结果包

1. **结论**：三层架构（Nautilus 壳 / 桥接层 / Rust 引擎），三个先行判决（保守撮合口径钉死、
   venue 为真账本+OrganicLedger 降级为意图状态机、时间映射集中桥接层），一个关键工程项
   （`PositionalStream` 流式意图模式，批↔流 bit-exact 预注册守卫），五阶段实施计划
   （数据基底→信号贯通→流式交易层→回测闭环→纸交易→实盘），阶段 1 代码骨架。
2. **定义依据**：NautilusTrader 1.228.0 官方文档/源码逐字提取（Strategy⊂Actor、
   FillModel.prob_fill_on_limit、单进程单 node、BarType 约定）；本仓库 lib.rs PyO3 接口面
   （process_bar 无时间戳、delta 接口、from_columns 磁带）；在册判决（maker ≤60s 撤单不追、
   L_max=1/(D_struct+mm)、touch 必须过 next 守卫）。
3. **边界条件**：(a) Nautilus beta 破坏性变更可使桥接层 API 失效（R1）；(b) 若 voice 状态机
   存在真实的全 tape 依赖，流式化判决翻转为定义冲突上浮（R2）；(c) 若阶段 4 实测 fill rate
   落出 [66.6%, 92.1%]，maker 可行性判决重审，执行层设计连带重审；(d) 若 Python 回调开销
   实测超 1s 多标的预算，信号层下沉 Rust，桥接层形态改变（R8）。
4. **下游推论**：M4（IBKR 执行支柱）的执行层落点确定为 Nautilus adapter 而非自研 TWS 桥；
   在册回测数字与 Nautilus 回测数字永久双口径（R4）；引擎 checkpoint 序列化从"优化项"
   升格为 1s 床位实盘的前置依赖（R3）。
5. **谱系引用**：CL maker 执行判决（touch/next 双模式守卫）、BC 配额架构原文判决
   （恒仓 1x 极性）、533 号（结构滞后三层谱型——对账容差设计的背景）；
   流式化改造是否触及交易层概念定义不确定——若 R2 触发即有新谱系条目。
6. **影响声明**：纯设计文档，零代码改动；新增 `analysis/nautilus_integration_design.md`；
   未修改任何在册定义、引擎代码、回测数字。提议的未来改动面：新增 `nautilus_chanlun/`
   Python 包、新增 `rust/src/trading/stream.rs`、`lib.rs` 增加 PositionalStream 暴露
   （均待实施阶段）。
