# NewChan 量化交易系统总纲领

## 系统愿景

构建一套以缠论为核心决策框架的全自动量化交易系统：从 1 分钟 K 线出发，通过五层递归管线自动涌现多级别走势结构，PH 拓扑校验级别真实性，MACD 背驰确认买卖点；K4 完全图拓扑与 ω 美元信用溢价联动实现品种池动态调整；最终通过 IBKR TWS API 完成全市场扫描、信号生成、风控和自动执行的闭环。系统的每一层都是事件驱动、增量计算的——核心引擎 O(1)/bar，从回测到实盘共用同一套管线，零代码切换。

---

## 四大支柱

### 支柱 I：缠论引擎（信号生成层）

**当前状态**：核心管线已完成，增量计算稳定。**8 层引擎已全量 Rust 重写并逐位等价（bit-exact）**——bi / segment / zhongshu / move / BSP / PH / MACD / RecursiveOrchestrator 八层逐字段移植，PyO3 0.23 + maturin 构建，L1/L2 真实数据上与 Python 实现逐位一致（`rust/src/*.rs` + `tests/test_rust_*_equivalence.py` 九组等价测试）。

| 组件 | 状态 | 代码位置 |
|------|------|---------|
| BiEngine（笔引擎） | ✅ 增量 O(1)/bar，257 个测试 | `bi_engine.py` |
| SegmentEngine（线段引擎） | ✅ 特征序列法，事件驱动 | `core/recursion/segment_engine.py` |
| ZhongshuEngine（中枢引擎） | ✅ 事件驱动 | `core/recursion/zhongshu_engine.py` |
| MoveEngine（走势类型引擎） | ✅ 趋势/盘整分型 | `core/recursion/move_engine.py` |
| BuySellPointEngine（买卖点引擎） | ✅ 三态分离（candidate/confirmed/negated） | `core/recursion/buysellpoint_engine.py` |
| RecursiveStack（级别递归栈） | ✅ 自动递归至终止 | `core/recursion/recursive_stack.py` |
| RecursiveOrchestrator（五层编排器） | ✅ Bi→Seg→ZS→Move→BSP 全链 | `orchestrator/recursive.py` |
| MACD 背驰 v1 | ✅ 面积/DIF 峰值/柱子高度三维度 | `a_divergence_v1.py` |
| OnlineMacdState（增量 MACD） | ✅ 增量 EMA + 穿透管线 | `a_macd.py` |
| **Rust 引擎（8 层逐位等价）** | ✅ bi/seg/zs/move/BSP/PH/MACD/Orchestrator bit-exact | `rust/src/*.rs` |

**目标**：作为不可替代的信号源，为回测和实盘提供统一的 `RecursiveOrchestratorSnapshot` 输出。

**已知开放问题**：
- 背驰 T6/T7 与 T5 的组合方式未结算（`a_divergence_v1.py` 注释标注）
- 级别涌现边界受行情反复约束，非 bar 数（memory 记录）

### 支柱 II：PH 拓扑（级别验证层）

**当前状态**：在线 merge tree 和 settle 判据已实现，与缠论级别递归的对接初步完成。

| 组件 | 状态 | 代码位置 |
|------|------|---------|
| OnlineMergeTree | ✅ 因果 settle 判据 | `a_online_persistence.py` |
| PH Barcode | ✅ persistence 条形码 | `a_persistence_barcode.py` |
| PathPersistence | ✅ 路径持久性计算 | `a_path_persistence.py` |
| PH Layer（缠论集成） | ✅ settle_trigger + level 校验 | `ph_layer.py`, `a_settle_trigger.py` |
| PH Zhongshu（PH 中枢） | ✅ 拓扑中枢定义 | `a_ph_zhongshu.py` |

**目标**：
- 作为级别递归的**停止条件**：PH settle 确认级别真实性后才允许递归推进
- 作为买卖点的**必要条件**（candidate 层）：PH settle 是形态学前提，confirmed 需 MACD 背驰（521 号谱系）
- 结构变化检测：online merge tree 的 death 事件 = 走势结构的拓扑转折

**已知边界**（memory 记录）：PH settle 是买卖点 candidate 层的必要条件，不是操作充分确认。用作退出/入场门控是越界。

### 支柱 III：K4 拓扑选股（品种选择层）

**当前状态**：K4 完全图模型和配置空间已定义，品种池生成器（帕萨卡利亚条件设定链）已实现，商空间排序经 L2 验证。

| 组件 | 状态 | 代码位置 |
|------|------|---------|
| K4 完全图（E/Au/R/$） | ✅ 六边比价模型 | `topology/graph.py` |
| Configuration 三元组 | ✅ (E/$, Au/$, R/$) 压缩视图 | `topology/config_space.py` |
| K4 Scanner | ✅ RecursiveOrchestratorSnapshot → Configuration | `topology/k4_scanner.py` |
| 极性指数 | ✅ 配置空间 → 宏观方向 | `topology/config_space.py` |
| 折叠等价类 + 商空间排序 | ✅ L2 验证确认（366 号） | `trading/fold_equivalence.py`, `scanner.py` |
| 比价引擎（ratio_engine） | ✅ 多对比价并行分析 | `ratio_engine.py` |
| 标的池生成器（scanner_pool） | ✅ 帕萨卡利亚三阶段 | `trading/scanner_pool.py` |
| 四层递归状态机 | ✅ L0-L3 边界条件枚举 | `trading/state_machine.py` |
| EquivalencePair 比价构造 | ✅ 验证 + ratio K 线生成 | `equivalence.py` |

**与 ω 研究的联动**：
- ω（金油比）告诉你宏观 regime（美元信用溢价方向）
- K4 极性指数告诉你品种间微观结构（哪类资产受益）
- 两者叠加 = regime 检测 + 品种池动态调整

**目标**：
- 从 K4 配置空间的 regime 断点自动触发品种池重构
- 与 ω 美元信用溢价研究（`capital_flow.py`, `flow_relation.py`）打通数据通道
- 全市场扫描的入口：scanner_pool 输出 → 买卖点信号过滤 → 交易执行

### 支柱 IV：IBKR 执行（交易执行层）

**当前状态**：TWS API 数据适配器已实现，LiveEngine/LivePool 实时引擎就绪，架构设计完成。

| 组件 | 状态 | 代码位置 |
|------|------|---------|
| IBKR 数据适配器 | ✅ TWS API + ib_insync | `data_ibkr.py` |
| LiveEngine（实时引擎） | ✅ 逐 bar 驱动 RecursiveOrchestrator | `live_engine.py` |
| LivePool（多标的并发池） | ✅ asyncio task pool | `live_pool.py` |
| WebSocket 推送 | ✅ lightweight-charts 可视化 | `server.py` |
| 成本模型（佣金 + 滑点） | ✅ IBKR 费率表 | `cost/commission.py`, `cost/slippage.py` |
| 架构设计文档 | ✅ 三套方案评估 | `docs/architecture/ibkr_quant_system.md` |
| 风控模块 | ❌ 未实现 | — |
| 下单执行器 | ❌ 未实现 | — |
| 仓位管理 | 🔸 部分（position_sizing, cost_reduction_fsm） | `trading/position_sizing.py`, `trading/cost_reduction_fsm.py` |

**目标**：TWS API 实时数据接入 → RecursiveOrchestrator 信号生成 → 风控校验 → LMT 下单 → 仓位管理闭环。

---

## 里程碑时间线

里程碑按依赖关系排列，不绑定日期。每个里程碑的前置条件在"阻塞于"列中标明。

### M1：回测验证（缠论策略可证伪性）

**目标**：在真实历史数据上验证缠论买卖点信号的统计特性，建立策略可证伪的基线。

**阻塞于**：无（当前代码已具备执行条件）

**当前进展**（L2 真实数据，1min 期货/股票）：
- **E 版本（背驰定位器主轴）有正 alpha**：次级别底背驰进场 + L2 趋势顶背驰出场 + 持仓穿越降成本，OKLO 完整时段 +932%（>buy-hold +307%），两标的 E>A>D（背驰门控进出场 D 被证伪——见 memory）
- **Version I（动态级别归属）完成**：买点归属最高涌现层定出场级别 + 多重赋格降成本，降成本风险改善但仍负贡献（OKLO I+416% < E+455%，regime-dependent 非全标的最优）
- **否定性结果已沉淀**：降成本提款机 bug（trim 被 max(0,·) 建模为只赚不赔）已定位；ω regime→美股方向被真实数据反向证伪（p=0.069 方向相反）；挣股数阶段在 1min 上空有效域
- **数据基底**：31.8M bars 1min 纯期货（ES/GC/CL/ZN/6E/BRN/DX）+ Databento 十年历史，消除连续合约拼接前视

**交付物**：
1. **回测管线完整跑通**：`backtest/engine.py` + `backtest/full_pipeline.py` 在多标的、多时段上产出完整交易记录
2. **统计报告**：胜率、盈亏比、最大回撤、Sharpe 比率、按买卖点类型分层统计
3. **随机门控对照**：相同品种上的随机入场基线，证伪策略 alpha 的来源（非 buy-hold 同义反复）
4. **前视检查审计**：自动化检测入场价/退出价是否使用了未来数据
5. **认识论标注**：所有回测结论标注 L2（真实数据验证），附否定性结果

**关键约束**（memory 记录）：
- 全历史压缩/超长上行数据上的 buy-hold 比较不可证伪，不作为评判标准
- 走势方向用中枢定义（缠论正典），不用 swing 笔端点代理

**代码映射**：
- `backtest/engine.py` — 核心回测引擎
- `backtest/full_pipeline.py` — 端到端管线
- `backtest/analysis.py` — 统计分析
- `backtest/state_machine.py` — 交易状态机
- `backtest/orchestrator.py` — 多标的编排

### M2：选股正则化（K4 拓扑 + ω regime）

**目标**：从全市场缩窄到高概率品种池，用 K4 拓扑结构和 ω 宏观 regime 联合驱动。

**阻塞于**：M1（需要回测结论验证单标的信号质量后，才能有意义地评估选股效果）

**当前进展**：
- **K4 折叠通道重构完成（528/529 号）**：顶点模型 E/Au/R/$ → 折叠通道模型 M/P/C/R，金/油降为折叠通道观测量（Au=C↔M、Oil=C→P），σ_e→σ_p，配置转换图 54 边 → 81 边（527 号 σ 已结算为走势方向态，禁 +↔− 是实现错误）；`data_mapping` 成为单一真相源（R=VNQ 非 TLT）
- **C 路径 96% alpha 留存**：K4 折叠重构后主轴信号路径的 alpha 保真度验证
- **递归分解树设计完成**：`topology/decomposition_tree.py` + `docs/architecture/recursive_decomposition_tree.md`，买点动态级别归属的结构化分解
- **σ 配置态本体修正（527 号）**：transition.py 应为 81 边图（σ 可 −1→+1 跳变），原 54 边图的图论性质建立在错误连续性假设上
- **否定性结论保留**：K4 门控在 OKLO 2 年毁 alpha（E+932% > E+M2+128%）——选股门控当前为净负贡献，作为可证伪基线沉淀

**交付物**：
1. **K4→品种池端到端管线**：K4 极性指数 + 折叠等价类 → 商空间排序 → 品种池输出
2. **ω 数据接入**：金油比（XAUUSD/CL）实时计算 + GDP 协整检验 + regime 断点检测
3. **品种池动态调整**：regime 断点触发品种池重构（不是定时重算）
4. **选股效果回测**：对比全市场扫描 vs K4 过滤后的信号质量差异
5. **四层递归状态机集成**：L0 配置层 → L1 跨角层 → L2 角内轮动 → L3 标的内操作

**代码映射**：
- `topology/` — K4 完全图、配置空间、极性指数
- `trading/scanner_pool.py` — 品种池生成器
- `trading/fold_equivalence.py` — 折叠等价类
- `trading/state_machine.py` — 四层递归状态机
- `ratio_engine.py` — 比价分析
- `capital_flow.py`, `flow_relation.py` — ω 相关计算
- `scanner.py` — 统一扫描器入口

### M3：全市场实时信号

**目标**：品种池内所有标的实时接收数据、运行缠论管线、产出买卖点信号。

**阻塞于**：M1（信号生成管线经过回测验证），M2（品种池确定）

**交付物**：
1. **IBKR TWS API 实时数据流**：LivePool 管理品种池内所有标的的并发订阅
2. **信号聚合仪表盘**：每个标的的当前走势结构 + 买卖点状态 + PH settle 状态
3. **WebSocket 实时推送**：lightweight-charts 前端显示多标的走势
4. **信号过滤器**：从 RecursiveOrchestratorSnapshot 到可操作信号的转换（confirmed BSP + PH settle + MACD 背驰三重确认）
5. **区间套共振检测**：多条比价线同时出现方向一致买卖点 → 共振信号

**代码映射**：
- `live_engine.py` — 单标的实时引擎
- `live_pool.py` — 多标的并发池
- `data_ibkr.py` — TWS 数据适配器
- `server.py` — HTTP API + WebSocket
- `nesting/resonance.py` — 共振检测
- `nesting/bsp.py` — 区间套买卖点

### M4：风控与执行

**目标**：从信号到下单的完整闭环，包含风控校验。

**阻塞于**：M3（实时信号流就绪）

**交付物**：
1. **风控模块**：
   - 单标的风控：最大持仓比例、止损阈值、单笔最大亏损
   - 组合风控：行业集中度、相关性约束、总敞口上限
   - 品种特定参数：期货保证金、合约乘数、最小变动单位
2. **下单执行器**：
   - LMT 限价单（默认），不用市价单
   - TWS API 下单 → 回报 → 状态更新
   - 订单生命周期管理（pending → filled / cancelled / rejected）
3. **仓位管理**：
   - 成本归零阶段转换（cost_reduction → cost_zero → earn_shares）
   - 短差降成本状态机（`cost_reduction_fsm.py` 已有框架）
   - 仓位大小计算（`position_sizing.py` 已有框架）
4. **执行日志**：每笔交易完整记录，可追溯到信号来源的 BSP 事件

**代码映射**：
- `trading/position_sizing.py` — 仓位计算
- `trading/cost_reduction_fsm.py` — 成本归零状态机
- `cost/` — 佣金和滑点模型
- 新建：`execution/` — 下单执行器 + 风控模块

### M5：生产加固

**目标**：系统在生产环境中持续稳定运行。

**阻塞于**：M4（执行闭环完成）

**交付物**：
1. **监控告警**：引擎延迟、数据断流、异常订单、持仓偏离
2. **故障恢复**：TWS 断连重连、引擎状态持久化与恢复
3. **合规记录**：交易日志满足监管要求
4. **性能优化**：多标的并发下的 CPU/内存 profiling，热路径优化

---

## 技术依赖图

```
M1 回测验证
├── 缠论引擎 (已就绪)
├── PH 拓扑 (已就绪)
├── 回测引擎 (已就绪)
└── 数据源: AlphaVantage/Databento 历史数据 (已就绪)

M2 选股正则化
├── M1 (信号质量基线)
├── K4 拓扑 (已就绪)
├── ω 数据接入 (部分就绪: capital_flow.py)
└── 四层递归状态机 (已就绪)

M3 全市场实时信号
├── M1 (管线验证)
├── M2 (品种池)
├── LiveEngine/LivePool (已就绪)
├── IBKR 数据适配器 (已就绪)
└── WebSocket 推送 (已就绪)

M4 风控与执行
├── M3 (实时信号流)
├── 风控模块 (待建)
├── 下单执行器 (待建)
└── 仓位管理 (部分就绪)

M5 生产加固
├── M4 (执行闭环)
├── 监控告警 (待建)
└── 故障恢复 (待建)
```

**并行可能性**：M1 和 M2 的基础设施工作可部分并行——K4 拓扑和品种池生成器已就绪，品种池的**回测验证**依赖 M1，但品种池的**构建和 ω 接入**不依赖 M1。

---

## 代码映射总览

```
src/newchan/
├── 支柱 I: 缠论引擎
│   ├── bi_engine.py              # 笔引擎 (增量 O(1)/bar)
│   ├── a_fractal.py              # 分型检测
│   ├── a_inclusion.py            # 包含处理
│   ├── a_stroke.py               # 笔构造
│   ├── a_feature_sequence.py     # 特征序列
│   ├── a_segment_v1.py           # 线段 (特征序列法)
│   ├── a_zhongshu_v1.py          # 中枢
│   ├── a_move_v1.py              # 走势类型
│   ├── a_buysellpoint_v1.py      # 买卖点 (三态分离)
│   ├── a_divergence_v1.py        # 背驰 (MACD 三维度)
│   ├── a_macd.py                 # 增量 MACD
│   ├── core/recursion/           # 五层递归引擎
│   │   ├── segment_engine.py
│   │   ├── zhongshu_engine.py
│   │   ├── move_engine.py
│   │   ├── buysellpoint_engine.py
│   │   ├── recursive_stack.py    # 自动递归至终止
│   │   └── recursive_level_engine.py
│   ├── orchestrator/recursive.py # 五层编排器 (总入口)
│   └── bi_differ.py              # 笔差分 (事件产生)
│
├── 支柱 II: PH 拓扑
│   ├── a_online_persistence.py   # 在线 merge tree (因果 settle)
│   ├── a_persistence_barcode.py  # PH barcode
│   ├── a_path_persistence.py     # 路径持久性
│   ├── a_settle_trigger.py       # settle 触发器
│   ├── a_ph_zhongshu.py          # PH 中枢
│   └── ph_layer.py               # 缠论集成层
│
├── 支柱 III: K4 拓扑选股
│   ├── topology/
│   │   ├── graph.py              # K4 完全图
│   │   ├── config_space.py       # 配置空间 + 极性指数
│   │   ├── k4_scanner.py         # Snapshot → Configuration
│   │   ├── fiber_bundle.py       # 纤维丛
│   │   ├── transition.py         # 状态转移
│   │   └── multi_tf_pipeline.py  # 多周期管线
│   ├── trading/
│   │   ├── scanner_pool.py       # 品种池 (帕萨卡利亚)
│   │   ├── fold_equivalence.py   # 折叠等价类 + 商空间排序
│   │   ├── state_machine.py      # 四层递归状态机
│   │   ├── position_sizing.py    # 仓位计算
│   │   ├── cost_reduction_fsm.py # 成本归零 FSM
│   │   └── stock_scanner.py      # 个股扫描
│   ├── equivalence.py            # 比价对构造
│   ├── ratio_engine.py           # 比价分析调度
│   ├── scanner.py                # 统一扫描入口 (商空间排序)
│   ├── capital_flow.py           # 资本流动 (ω 相关)
│   └── flow_relation.py          # 流动关系
│
├── 支柱 IV: IBKR 执行
│   ├── data_ibkr.py              # TWS API 数据适配器
│   ├── live_engine.py            # 单标的实时引擎
│   ├── live_pool.py              # 多标的并发池
│   ├── server.py                 # HTTP API + WebSocket
│   ├── cost/
│   │   ├── commission.py         # IBKR 佣金模型
│   │   ├── slippage.py           # 滑点模型
│   │   └── config.py             # 成本配置
│   └── (待建) execution/         # 下单 + 风控
│
├── 跨支柱基础设施
│   ├── types.py                  # 核心类型定义
│   ├── events.py                 # 域事件系统
│   ├── config.py                 # 全局配置
│   ├── core/bar.py               # K 线类型
│   ├── core/stream.py            # 数据流抽象
│   ├── backtest/                 # 回测框架
│   ├── nesting/                  # 区间套 (多比价线联动)
│   │   ├── bsp.py                # 区间套买卖点
│   │   ├── resonance.py          # 共振检测
│   │   └── wait_state.py         # 等待状态
│   ├── obs/                      # 可观测性
│   └── audit/                    # 不变量审计
│
└── 数据源适配器
    ├── data_av.py                # AlphaVantage
    ├── data_databento.py         # Databento (历史)
    ├── data_databento_live.py    # Databento (实时)
    ├── data_astock.py            # A 股
    └── data_crypto.py            # 加密货币
```

---

## 关键设计原则

1. **管线统一**：回测和实盘共用同一个 `RecursiveOrchestrator`，差异仅在数据源（历史文件 vs TWS 实时流）
2. **事件驱动**：所有引擎通过 `DomainEvent` 通信，下游引擎订阅上游事件而非轮询状态
3. **增量计算**：每个引擎维护内部状态，每 bar 均摊 O(1)，总计 O(N)
4. **不可变快照**：`RecursiveOrchestratorSnapshot` 是 frozen dataclass，引擎间传递不可变数据
5. **严格因果**：入场价 = BSP 确认 bar 的 close，所有决策基于截至当前的数据，零前视
6. **缠论语言封闭性**：退出条件基于走势结构（次级别走势类型完成），不引入外部金融工程概念（止损线、ATR 等）

---

## 相关文档

| 文档 | 内容 |
|------|------|
| `docs/architecture/ibkr_quant_system.md` | IBKR 接入方案详细设计（TWS API/Web API/Lightspeed Connect 三方案评估） |
| `docs/persistence_theory.md` | PH 拓扑理论基础（§7.5 在线 merge tree 升级方向） |
| `docs/formal_axioms.md` | 缠论形式化公理 |
| `docs/chan_spec.md` | 缠论编码规范 |
| `docs/spec/` | 各模块规范文档 |
| `docs/research/` | ω 研究、K4 拓扑研究输出 |
