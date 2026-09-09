# 旧入口与实际调用去向：R1 候选附录

> 名分：#1339 独立生产架构候选的静态处置附录；未采纳、未切换、未实施。
> 绑定 R1：`f6054b52f86ba80fafd6386d1ac6ede341c6c6a17cb9efbed5d2dad42571824d`（正文）；正文保持冻结。

本候选保留行情、呈现、回放、venue 适配与验证职责；旧独立结构判定、聚合经济内核和旁路发送权在新内核验收并获准切换后退役。旧引擎可保留为明确隔离的对照。每项去向都跟随实际公共入口、分派 callee 或真实调用者，不能把 helper、配置或登记关系另算一个资金 owner。

旧实现行为依据是已封存 [SCAN-DELIVERY](/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json) 指定的 [canonical 当前入口/消费者登记](/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CURRENT-ENTRY-CONSUMER-REGISTRY.json)，登记 SHA-256 为 `fd2547944345274391c72f516bb90de0c4c8ace41dc084757af77723efc7376f`。这是冻结静态证据，未据此推断当前部署。

## 计数与使用口径

登记共 1815 条关系；其中 1309 条为 import 定位，不证明调用。本附录将余下 506 条关系逐一挂到 45 个处置组，机器 JSON 给出全部登记 ID 与原数组指针；45 组含入口、公共 callee、支持、验证和范围边界，不能报道为 45 个可执行入口。没有复制 import 表或扩大 census。

目录/通配符是代表家族，不是逐文件删除名单。源记录已标明更深生产体阅读时，本附录只引用该冻结结论，不冒领当前源码重读或测试通过。

## 共同退出条件

- **X1** 确定实际入口/constructor/feature/env/default preset 的最终清单；映射 session、数据、账户/子账户、venue、重、voice、逻辑/物理订单、执行 epoch、旧版本。冻结登记不足以替代上线前当前状态核对。
- **X2** 新 R1 必需实现通过完整分类输入桥与产出验证（62CC/10LC/32义务按各自性质结清）、资金决策的明确批准、各重账务与 FE01–FE11/相应 AC、O1–O3、RF-01；协议的 0H/0M 静态评审不等于实现验收。
- **X3** 保存全部旧事实、真实账户余额/仓位/订单、未决 Prepared/Reserved/MayHaveSent、未入账回报、费用/修正、成对差价责任和退出尾债；做身份交叉表和可重放导入。不能迁移一个净值数字便声称账已迁完。
- **X4** 新唯一发送者完成账户/venue 能力验证及 P1/P4/P7 崩溃、重复、延迟、部分成交、并发改价测试；证明旧引擎无旁路发单。paper 有独立身份和凭据隔离，同协议但不冒称外部成交。
- **X5** 得到明确的生产切换/撤旧发送权限批准后才执行：先停旧新增授权并在公共顺序中 fence 旧 epoch，保留既有可能外部效应责任及查询/回报入口，再按已验收切换方案启用新唯一发送者；任何时刻不能让两个经济内核独立支配同一账户。
- **X6** 回滚预案必须接受切换之后已发生的新事实和资金责任；只能交棒写权限，不能倒退账本或释放未终结责任。旧观察/隔离对照可长期保留；文件删除是后续逐依赖验收事项，不由本候选自动授权。

以下迁移/退役项均须满足相应 X1–X6；保留范围外项不产生新的修改或生产切换授权。

## 实际入口、callee 与支持去向

### OE01 newchan CLI、Bottle 图表服务与开发启动入口

**类型：**入口。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`pyproject.toml`；`src/newchan/cli.py`；`src/newchan/server.py`；`scripts/dev.ps1`。

**依据：**project.scripts→cli.main；chart→Bottle run_server。Bottle 8765 与 ASGI 8766 是不同服务；登记的 dev.ps1 请求 serve，而 CLI 仅注册 chart，不能视为已打通的启动链。

**R1 归属：**启动清单只装配 R1 数据/观测适配器；状态和判定从同一生产核心读取。

- 逐一核对实际 CLI 命令、参数、默认 preset、端口与生命周期；消除 serve/chart 错配后做一次启动/停止验收。
- 图表启动不得隐式启动另一套交易策略；保留纯观测启动方式。

**登记 ID：**`IO-FR001、IO-FR002、AUX-FR0009`。

### OE02 行情获取、缓存、重采样前的数据适配

**类型：**入口与数据支持。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`src/newchan/data_databento.py`；`src/newchan/cache.py`；`trading_system/data/databento_loader.py`；`trading_system/data/feed_abstraction.py`；`trading_system/config/instruments.py`；`scripts/fetch_1m_databento_10y.py`；`scripts/consolidate_massive_tick.py`；`scripts/cdp_extract_ohlcv.py`。

**依据：**抓取/缓存/NT catalog、feed 选择、SIP 合并、TV 导出属于输入职责。live feeder 到 gateway 的 on_bar 接缝尚未由该登记证明；AUX20 同时有 K4 读者，不能把该读者迁入本候选。

**R1 归属：**R1 唯一市场输入及数据身份适配；行情不携带旧结构/资金决策。

- 核 symbol、venue、时间单位、交易日、修订、重复与缺口；生产结构输入按 R1 BeginRevision 顺序进入。
- 证明 feeder 实际送达；数据退役/删除原始文件是独立操作，不能随架构迁移自动执行。

**登记 ID：**`IO-FR003、IO-FR004、IO-FR005、IO-FR006 等 15 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE03 REST/WS replay 与 TFOrchestrator

**类型：**入口。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`src/newchan/gateway.py`；`src/newchan/replay.py`；`src/newchan/orchestrator/timeframes.py`；`src/newchan/b_timeframe.py`；`frontend/src/hooks/useReplay.ts`；`frontend/src/hooks/useEventFeed.ts`。

**依据：**REST replay 可选 mode/timeframes；WS start 使用默认 Recursive/1min。ReplaySession step/seek 重放旧 Python 核心；进程全局 session、WS 断开、stop 删除分别有不同生命周期，seek 消息契约存在接缝。

**R1 归属：**同一 R1 核心的 session/游标适配；时间框数据输入与结构级别分开。

- REST、WS、逐步、seek、重连在相同 cut 下读出同一完整对象；每个 session 明确身份、epoch 和销毁边界。
- 明确历史 seek 为只读重建或隔离回放，不能重发历史交易；不用 TF 时间框数量伪装递归级别。

**登记 ID：**`IO-FR008、IO-FR009、IO-FR010、IO-FR012、IO-FR013、IO-FR014、IO-FR132、KERNEL-FR028、KERNEL-FR029`。

### OE04 overlay、前端与 realtime_server 查看入口

**类型：**入口与读端。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`src/newchan/ab_bridge_newchan.py`；`src/newchan/a_recursive_engine.py`；`src/newchan/a_center_v0.py`；`src/newchan/a_trendtype_v0.py`；`src/newchan/a_level_fsm_newchan.py`；`src/newchan/nested_pipeline.py`；`scripts/realtime_server.py`；`analysis/chanlun_viewer.html`；`frontend/index.html`。

**依据：**overlay 内部另跑 A 递归、中枢/趋势、lstar、nested 搜索及 BSP；realtime_server 也持有 Python Recursive 核心。两个查看器不是天然同源。

**R1 归属：**保留呈现与订阅；结构/操作/账务视图仅投影 R1 Observation cut。

- 撤除查看接口内独立重判和 lstar 选择权；对象保留完整类型、来源、首次得知时间和修订链。
- 实时、回放和 UI 同 cut 对拍；前端无交易写权限；原视图若保留比较须展示不同 run/core 版本。

**登记 ID：**`IO-FR011、IO-FR040、IO-FR041、KERNEL-FR030、KERNEL-FR031、KERNEL-FR032、KERNEL-FR033、KERNEL-FR036、KERNEL-FR048、AUX-FR0002、AUX-FR0003`。

### OE05 Python RecursiveOrchestrator 及其结构流水线

**类型：**旧公共核心。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`src/newchan/orchestrator/recursive.py`；`src/newchan/bi_engine.py`；`src/newchan/core/recursion/segment_engine.py`；`src/newchan/core/recursion/zhongshu_engine.py`；`src/newchan/core/recursion/move_engine.py`；`src/newchan/core/recursion/recursive_stack.py`；`src/newchan/core/recursion/buysellpoint_engine.py`；`src/newchan/orchestrator/bus.py`。

**依据：**Python 每 bar 分别推进笔、段、中枢、走势、递归及 BSP/event bus；其构造参数和级别标记是冻结观察，不是 R1 定义依据。

**R1 归属：**R1 Classification/Operation 读取取代旧整条权威；可按独立证明复用纯计算。

- 切断 OE03/OE04/旧 reader 的默认生产连接；完成 CC001–CC062 实际数据域桥与层级同判据验证。
- 保留旧输入/输出作有版本的对照，不以旧结果给新判据失败兜底。

**登记 ID：**`KERNEL-FR001、KERNEL-FR002、KERNEL-FR003、KERNEL-FR004、KERNEL-FR005、KERNEL-FR006、KERNEL-FR007`。

### OE06 PyRecursiveOrchestrator / Rust RecursiveOrchestrator

**类型：**旧 FFI 公共核心。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/orchestrator.rs`；`rust/src/bi_zhongshu_bsp.rs`；`rust/src/zhongshu.rs`；`rust/src/moves.rs`；`rust/src/divergence.rs`；`rust/src/buysellpoint.rs`。

**依据：**PyRecursive 的 enable_bsp 同时控制递归栈；take_trend/take_bi、旧 MACD/幅度/confirm ratio 链被策略与研究 reader 消费。导出类存在不证明现役服务已采用。

**R1 归属：**FFI 兼容层若保留必须显式绑定 R1 版本；旧类只能隔离对照。

- 清点这个类的真实构造者与 preset；旧 take_* 不能继续成为任何 R1 交易信号来源。
- 新 FFI 暴露完整分类与对象身份，不能只用旧信号 tape 作为强制语义接口。

**登记 ID：**`KERNEL-FR009、KERNEL-FR037、KERNEL-FR046、KERNEL-FR047`。

### OE07 ChanlunStrategy、ChanlunBridge 与 MakerOptimizer

**类型：**策略和实际外部适配链。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`trading_system/strategy/chanlun_strategy.py`；`trading_system/strategy/signal_bridge.py`；`trading_system/strategy/bar_counter.py`；`trading_system/execution/maker_optimizer.py`；`trading_system/execution/lmt_executor.py`；`trading_system/execution/leverage_calculator.py`；`trading_system/execution/risk_controls.py`；`trading_system/persistence/trade_journal.py`；`trading_system/persistence/database.py`；`trading_system/persistence/bar_cache.py`。

**依据：**旧 Recursive events→ChanlunBridge→策略→MakerOptimizer→NT 下单；RiskGate 与恢复数据库不能从声明推定已完整挂载。现有 first-seen watermark、计时改价和 journal 不等于 R1 协议。

**R1 归属：**保留 venue/NT 编码、查询、行情适配职责；经济判断归各重账，所有实际发送归公共授权后的唯一执行器。

- P1/P4/P7 覆盖 submit、定时重定价、部分成交、撤单、断线恢复；证明逻辑数量上限跨独立可执行 allocation 成立。
- 所有外部回报先经共同 CaptureExternal；旧 journal 迁为事实来源，不能由另一个策略恢复后继续独立发单。

**登记 ID：**`ACT-FR003、ACT-FR004、ACT-FR005、ACT-FR006、ACT-FR007、ACT-FR116、ACT-FR117、ACT-FR118、ACT-FR119、ACT-FR120、ACT-FR121、ACT-FR122、ACT-FR123`。

### OE08 PyThetaStream、ThetaPiStream 与 ThetaRecStrategy

**类型：**旧 FFI/策略入口。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`rust/src/theta_v0/ffi.rs`；`rust/src/theta_v0/stream.rs`；`rust/src/theta_v0/classifier/streaming.rs`；`rust/src/theta_v0/classifier/e2eo.rs`；`rust/src/theta_v0/strategy/consume_router.rs`；`trading_system/strategy/theta_rec_strategy.py`。

**依据：**默认 cdylib 注册 PyThetaStream；ThetaRecStrategy 按 bar 推入并按净头寸行动；owned classifier 以 operating_levels=&[] 调用，stream 输出消费未构成外部成交回灌；registry 查询本身不具备恢复闭环。

**R1 归属：**公开流适配可迁移；旧 stream 的聚合资金与自动信号消费连接退役，R1 重/分支挂载必须显式。

- 真实 operating mount、完整分类输出、各重 ledger、外部 fill/bust 回灌和重启重放闭环全部接通。
- 禁止 nav fallback 充当账本真值；on_stop close-all 与 finish 必须服从退出尾债协议，不能借停止清除责任。

**登记 ID：**`ACT-FR008、ACT-FR009、ACT-FR010、ACT-FR011、KERNEL-FR008、KERNEL-FR010、KERNEL-FR012、KERNEL-FR020、KERNEL-FR022`。

### OE09 Rust run_theta_backtest / Nautilus ThetaStrategy

**类型：**旧回测/策略入口。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`rust/src/theta_v0/nautilus/backtest_engine.rs`；`rust/src/theta_v0/nautilus/strategy.rs`；`rust/src/theta_v0/nautilus/account_adapter.rs`；`rust/src/theta_v0/nautilus/bar_adapter.rs`；`rust/src/theta_v0/nautilus/order_adapter.rs`。

**依据：**feature nautilus/backtest_bin 的装配运行 ThetaStrategy/ThetaCore；recognize_nested/plan_orders 是独立旧决策链。登记范围 BTC/CASH/NETTING，position closed 清理不证明部分成交恢复正确。

**R1 归属：**回测 driver/NT 数据与订单编码适配到同一 R1 内核；旧 ThetaCore 决策/持仓状态退役。

- 回测、paper 与真实适配共享逻辑命令及账务协议；模拟终态/成交不得用更宽准入。
- 补部分成交、撤单未终态、回报延迟、账户身份和初始净值一致性；没有通过前只作旧对照。

**登记 ID：**`ACT-FR016、ACT-FR017、ACT-FR094、ACT-FR095、ACT-FR096、ACT-FR097、THC-FR003`。

### OE10 fugue_v3 / PyFugueV3Stream 与批量入口

**类型：**旧引擎家族。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/fugue_v3/ffi.rs`；`rust/src/fugue_v3/engine.rs`；`rust/src/fugue_v3/operate.rs`；`rust/src/fugue_v3/accounting.rs`；`rust/src/fugue_v3/axis.rs`；`rust/src/fugue_v3/cycle.rs`；`rust/src/fugue_v3/layer.rs`；`rust/src/fugue_v3/morphology.rs`；`rust/src/fugue_v3/observe.rs`；`rust/src/fugue_v3/prove.rs`。

**依据：**Signal/Morphology/Observe→Operate/accounting/cycle；FUGUE_LONG_ONLY 依环境变量存在与否生效。Morphology 也被 Spiral/recursive_t 消费，因此不能只退 recursive_t 而遗留同权威分支。

**R1 归属：**旧整个资金/操作组织冻结；R1 每重状态和持久责任替代；形态函数仅在同语义域证明后复用。

- 对所有 FFI/批量装配写明旧 core id 和无外部发送权限；逐一替换真实调用者。
- 旧 Long-only、floor、成本门与共享状态均不能默认继承为新资金政策。

**登记 ID：**`ACT-FR018、ACT-FR033、ACT-FR038、ACT-FR039、ACT-FR040、ACT-FR041、ACT-FR042、ACT-FR043、ACT-FR044、ACT-FR045、ACT-FR046、ACT-FR047`。

### OE11 recursive_t、RecTStream / TFugueStream / PyRecStream

**类型：**旧引擎家族。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/recursive_t/ffi.rs`；`rust/src/recursive_t/rec_stream.rs`；`rust/src/recursive_t/rec_driver.rs`；`rust/src/recursive_t/rec_engine.rs`；`rust/src/recursive_t/stream.rs`；`rust/src/recursive_t/t_engine.rs`；`rust/src/recursive_t/backtest.rs`；`rust/src/recursive_t/backtest_run.rs`；`rust/src/recursive_t/t_engine_run.rs`。

**依据：**RecTStream push/target_net/snapshot/finish 与 TFugue trade11 被 Python/NT 壳消费；Python new_production 与 Rust env constructor 默认路线不同。8×3 runner 是研究输出，不能据此声称外部执行。

**R1 归属：**冻结整个旧生产组织及默认 FFI；R1 新结构读取、各重账户和授权执行承接生产职责。

- enumerate 实际 constructor/feature/env/NT 壳，不能只删模块名而保留 target_net 发送旁门。
- 旧账户历史、未完成 T/voice 与可能外部尾债先做身份映射和可回放记录，再撤销旧发送 epoch。

**登记 ID：**`ACT-FR020、ACT-FR021、ACT-FR048、ACT-FR049 等 18 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE12 SpiralEngine 家族与 FFI

**类型：**旧引擎家族。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/spiral/ffi.rs`；`rust/src/spiral/engine.rs`；`rust/src/spiral/accounting.rs`；`rust/src/spiral/operation.rs`；`rust/src/spiral/closure.rs`；`rust/src/spiral/signal.rs`；`rust/src/spiral/state.rs`；`rust/src/spiral/voice.rs`。

**依据：**SpiralEngineCore→close_voice/try_spawn_cost_gated；signal/形态被其他旧引擎复用。研究结果或纯 for-loop 不等于交易 venue 回报闭环。

**R1 归属：**R1 各重账/完整生命周期承接，旧独立经济模型保留隔离比较。

- 确认所有脚本/FFI 核心版本；不得把 cost-gated spawn 自动翻译成 R1 初始/追加资金政策。

**登记 ID：**`ACT-FR019、ACT-FR062、ACT-FR063、ACT-FR064、ACT-FR065、ACT-FR066、ACT-FR067、ACT-FR068、ACT-FR069、ACT-FR070、ACT-FR071`。

### OE13 NT backtest/live/paper 装配面

**类型：**装配入口登记。**候选去向：**保留入口或适配职责，迁移到 R1；旧判定/经济/独立发送连接在验收后退役。

**冻结路径：**`trading_system/`；`trading_system/strategy/chanlun_strategy.py`；`trading_system/strategy/theta_rec_strategy.py`。

**依据：**登记 ACT-ENT009/010/011/012 指向两个实际回测装配、一个 data-only node 与一个纯 stub；BZ 默认 enable-orders 关闭。BTC 回测 capital=1e5 与 Theta 默认 initial_nav=1e6 未统一。

**R1 归属：**driver/环境只选择 R1 同一个内核和隔离账户；paper 独立 account/run id、模拟外部事实适配。

- 逐个 origin_entry_id 补出准确 launcher 文件/命令、配置、启动清单；不得将纯 stub 或 data-only 填报为已实现 paper。
- paper 模拟延迟/成交/撤单/改价依同一 P1/P4/P7；禁用真实凭据；真实迁移另经明确验收。

**登记 ID：**`ACT-FR025`。

**边界：**本附录只读 canonical registry；该聚合记录未给四个 main 的逐文件名称，不能冒称逐入口命令清单已齐。

### OE14 人工 HL 验证脚本与 IB warrant 工具

**类型：**已知人工外部发送入口。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`trading_system/live/hl_verify_*.py`；`analysis/buy_28316_warrant.py`。

**依据：**HL 脚本可走 NT submit 或 SDK order/cancel；IB 脚本 main 可经 TWS7496 place/cancel，人工 y 触发且 DAY 单可能存续。这是静态能力，不是当前在挂单。

**R1 归属：**保留明确批准的维护工具身份；涉及同账户的每个实际外部动作纳入 R1 风险事实和维护期栅栏。

- 迁移验收清点这些 launcher 的真实账户、凭据和可执行权限；自动策略必须能获知维护订单/成交。
- 若保留绕过自动命令流的人工下单，则先关相应账户新单 gate 并对账恢复；不能并发创建隐藏风险。
- 不删除工具，不运行脚本，不取消现有订单；实际操作须后续明确授权。

**登记 ID：**`ACT-FR031、AUX-FR0001`。

### OE15 OrganicTape / StreamingSignalReader / UnnStream 输入及比较壳

**类型：**旧信号公共接口。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`analysis/organic_signals.py`；`analysis/organic_fugue_rust_check.py`；`analysis/_dump_tape_rust.py`；`analysis/organic_fugue.py`；`analysis/fugue_version_i.py`；`analysis/fugue_v2_full_backtest.py`；`rust/src/lib.rs`；`rust/src/trading/trade_behavior.rs`；`trading_system/backtest_unn_stream.py`。

**依据：**readers→pack_tape/push_signal→OrganicTape/UnnStream；settled 默认与 NT 显式值不同，字段 capability 缺省不同。旧 MACD true 的独立 div 列空、BSP 仍另算，不能把空列叫完整分类。

**R1 归属：**保留旧 tape 格式和比较脚本；新生产读取 R1 完整分类对象，bar 数据适配可迁移。

- 新的 FFI/tape 必须有版本、字段完整性及来源身份，不以旧任意 true 字段替代 domain witness。
- 研究脚本指明 core/run id 且不能持有生产发送权限；新输出与历史格式差异显式展示。

**登记 ID：**`ACT-FR032、ACT-FR126、AUX-FR0015、AUX-FR0016、AUX-FR0017、AUX-FR0018、AUX-FR0019、AUX-FR0035、AUX-FR0036、AUX-FR0037`。

### OE16 run_organic / OrganicLedger / VoiceUnit

**类型：**旧经济入口。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/runner.rs`；`rust/src/trading/ledger.rs`；`rust/src/trading/level_operating_unit.rs`；`rust/src/lib.rs`。

**依据：**run_organic→OrganicLedger/VoiceUnit；ACT29 是限定七个实际 callee 的分派登记，不是七套新 owner。Stock/Fsm/Full/Signal、PerpShort 等参数是旧行为。

**R1 归属：**R1 每重 ledger/操作读取承接；旧 runner 只比较。

- 不能继承旧共享池、分区、摩擦率和仓位默认作为未决政策的答案；每个外部采用方迁到明确 R1 配置。

**登记 ID：**`ACT-FR028、ACT-FR029、POS-FR007`。

### OE17 run_positional 本体

**类型：**实际公开分派 callee。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/trading/positional.rs`；`rust/src/trading/positional.rs`。

**依据：**run_positional；本体通过 run_positional_rust 暴露；其他模式分派另列，不能以读过本体代替全部 callee。

**R1 归属：**R1 替代旧经济入口；旧 public name 可保留在显式 legacy comparison 命名空间。

- 真实模式别名、参数和默认启动调用者需与冻结记录对应；通过新资金/账务政策及 FE/RF 验收后才切换生产调用。
- 保留旧虚拟/实际账户来源和未完责任，不能直接把旧聚合净值当各重期初账。

**登记 ID：**`POS-FR001`。

### OE18 positional_fusion

**类型：**实际公开分派 callee。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/trading/positional.rs`；`rust/src/trading/positional_fusion.rs`。

**依据：**run_fusion；限定七中的独立 callee；不把它和 unified_voice 的 fusion_v 混为同一模式。

**R1 归属：**R1 替代旧经济入口；旧 public name 可保留在显式 legacy comparison 命名空间。

- 真实模式别名、参数和默认启动调用者需与冻结记录对应；通过新资金/账务政策及 FE/RF 验收后才切换生产调用。
- 保留旧虚拟/实际账户来源和未完责任，不能直接把旧聚合净值当各重期初账。

**登记 ID：**`POS-FR002`。

### OE19 unified_necessity / UnnStreamCore

**类型：**实际公开分派 callee。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/trading/positional.rs`；`rust/src/trading/unified_necessity.rs`。

**依据：**run_unified_necessity / UnnStreamCore；同时有 stream 公共消费面；完整流恢复需要独立验收。

**R1 归属：**R1 替代旧经济入口；旧 public name 可保留在显式 legacy comparison 命名空间。

- 真实模式别名、参数和默认启动调用者需与冻结记录对应；通过新资金/账务政策及 FE/RF 验收后才切换生产调用。
- 保留旧虚拟/实际账户来源和未完责任，不能直接把旧聚合净值当各重期初账。

**登记 ID：**`POS-FR003`。

### OE20 fusion_v / unified_voice

**类型：**实际公开分派 callee。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/trading/positional.rs`；`rust/src/trading/unified_voice.rs`。

**依据：**run_unified_voice；fusion_v 独立于 dual_voice 的 fusion_vd/vn/vdn；两者必须各有退场记录。

**R1 归属：**R1 替代旧经济入口；旧 public name 可保留在显式 legacy comparison 命名空间。

- 真实模式别名、参数和默认启动调用者需与冻结记录对应；通过新资金/账务政策及 FE/RF 验收后才切换生产调用。
- 保留旧虚拟/实际账户来源和未完责任，不能直接把旧聚合净值当各重期初账。

**登记 ID：**`POS-FR004`。

### OE21 nested_fugue

**类型：**实际公开分派 callee。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/trading/positional.rs`；`rust/src/trading/nested_fugue.rs`。

**依据：**run_nested_fugue；限定七的独立虚拟账户实现。

**R1 归属：**R1 替代旧经济入口；旧 public name 可保留在显式 legacy comparison 命名空间。

- 真实模式别名、参数和默认启动调用者需与冻结记录对应；通过新资金/账务政策及 FE/RF 验收后才切换生产调用。
- 保留旧虚拟/实际账户来源和未完责任，不能直接把旧聚合净值当各重期初账。

**登记 ID：**`POS-FR005`。

### OE22 isolated_fugue

**类型：**实际公开分派 callee。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/lib.rs`；`rust/src/trading/positional.rs`；`rust/src/trading/isolated_fugue.rs`。

**依据：**run_isolated_fugue；文件名 isolated 不能证明其账户、事实记录和恢复已经满足 R1 隔离。

**R1 归属：**R1 替代旧经济入口；旧 public name 可保留在显式 legacy comparison 命名空间。

- 真实模式别名、参数和默认启动调用者需与冻结记录对应；通过新资金/账务政策及 FE/RF 验收后才切换生产调用。
- 保留旧虚拟/实际账户来源和未完责任，不能直接把旧聚合净值当各重期初账。

**登记 ID：**`POS-FR006`。

### OE23 run_positional 延续模式分派表

**类型：**已有入口的分派证据。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/trading/positional.rs`。

**依据：**POS008/ACT144 指向六组真实延续 callee；不新增入口数量或资金 owner。

**R1 归属：**挂靠 OE17；每个实际模式落 OE24–OE29。

- 记录每个分支在构建/启动时的选择值；下列模式全部有独立处置后才能说本入口完成迁移。

**登记 ID：**`POS-FR008、ACT-FR144`。

### OE24 fusion_va

**类型：**实际模式/preset。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/positional.rs`；`rust/src/trading/axiom_voice.rs`。

**依据：**run_axiom_voice；Long/Gated 实际可达而真 Short 不可达；PairedOut 全卖后名义 Long 保留、钱进同批共享池，恢复并无独立 escrow。

**R1 归属：**各模式仅作隔离对照；R1 的选择必须从已签需求和未决政策独立落定。

- 不能把 PairedOut/restore_due 翻译成已资助 Short；R1 对应动作必须有本重资金、冻结责任与成对尾债。
- 保留模式/config/core id 的历史记录；撤销旧生产接线需共同退出条件 X1–X6。

**登记 ID：**`POS-FR009`。

### OE25 urs

**类型：**实际模式/preset。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/positional.rs`；`rust/src/trading/unified_recursive.rs`。

**依据：**run_unified_recursive；C_CLEAR_NOT_FLIP=false，root E* 触发整仓翻 Short、子级零 Long 壳；内部池半分。

**R1 归属：**各模式仅作隔离对照；R1 的选择必须从已签需求和未决政策独立落定。

- 明确清算/翻仓与子树生命周期；旧 half split 和 root 规则不是新资金政策。
- 保留模式/config/core id 的历史记录；撤销旧生产接线需共同退出条件 X1–X6。

**登记 ID：**`POS-FR010`。

### OE26 pcf

**类型：**实际模式/preset。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/positional.rs`；`rust/src/trading/positioning_chain_fugue.rs`。

**依据：**run_positioning_chain_fugue；op_ladder 固定 3，来源前缀 stamp 不等于真实链证书；theta 权重共享池。

**R1 归属：**各模式仅作隔离对照；R1 的选择必须从已签需求和未决政策独立落定。

- 用 R1 真实 parent/anchor/chain 证据替代前缀猜测；不能把固定 3 固化为所有级别规则。
- 保留模式/config/core id 的历史记录；撤销旧生产接线需共同退出条件 X1–X6。

**登记 ID：**`POS-FR011`。

### OE27 nifN

**类型：**实际模式/preset。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/positional.rs`；`rust/src/trading/nested_interval_fugue.rs`。

**依据：**run_nested_interval_fugue；N 是 numeric trade floor，已见 wrapper 3/4；root<=min 清、以上翻，不是 urs 的同名别称。

**R1 归属：**各模式仅作隔离对照；R1 的选择必须从已签需求和未决政策独立落定。

- 列出各已采用 N 的 preset；不外推 nif2=urs；逐个核级别/清翻/资金边界。
- 保留模式/config/core id 的历史记录；撤销旧生产接线需共同退出条件 X1–X6。

**登记 ID：**`POS-FR012`。

### OE28 rnf

**类型：**实际模式/preset。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/positional.rs`；`rust/src/trading/recursive_nested_fugue.rs`。

**依据：**run_recursive_nested_fugue；root 级别重标、无 C-clear，内部池半分。

**R1 归属：**各模式仅作隔离对照；R1 的选择必须从已签需求和未决政策独立落定。

- 新 root 变动依 R1 修订和持久责任协议；不能重标后丢掉旧持仓与 tail debt。
- 保留模式/config/core id 的历史记录；撤销旧生产接线需共同退出条件 X1–X6。

**登记 ID：**`POS-FR013`。

### OE29 fusion_vd / fusion_vn / fusion_vdn

**类型：**实际模式/preset。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/trading/positional.rs`；`rust/src/trading/dual_voice.rs`。

**依据：**run_dual_voice：vd=(dual_book=true,nest_deep=false)，vn=(false,true)，vdn=(true,true)；直接调用 false/false 不推出存在第四命名 preset。

**R1 归属：**各模式仅作隔离对照；R1 的选择必须从已签需求和未决政策独立落定。

- 三命名组合分别对照双账/深窗真实消费；不能把共享池、恢复配额和非对称门翻译成 R1 资金权利。
- 保留模式/config/core id 的历史记录；撤销旧生产接线需共同退出条件 X1–X6。

**登记 ID：**`POS-FR014`。

### OE30 POS015 mixed family 与配置/共享 helper

**类型：**附属调用证据。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/trading/config.rs`；`rust/src/trading/allocator.rs`；`rust/src/trading/center_book.rs`；`rust/src/trading/ledger.rs`；`rust/src/trading/master.rs`；`rust/src/trading/third_point_book.rs`。

**依据：**mixed35 覆盖 run_organic_rust/config::variant、run_positional 实际 callee、独立 run_recursive_rust、支持 API 和 cfg(test) 子域。该分类记录不是额外发送者。

**R1 归属：**每个 helper 依真实调用边归 OE16–OE29，未挂载 API 保持未挂载；纯函数可逐项证明后复用。

- 实施清单按 raw id/实际 callee 展开；不因 mixed 标签整目录删除，也不把 config 模块提升成公共资金权威。
- 分别核 production body、测试断言、未挂载 API；旧总池状态不可换个名字接进 R1。

**登记 ID：**`POS-FR015、KERNEL-FR-OPEN-F64-TRADING`。

### OE31 POS016 RetraceLedger、book/portal/log/audit

**类型：**结构记录支持。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/theta_v0/classifier/retrace_ledger/adapter.rs`；`rust/src/theta_v0/classifier/retrace_ledger/audit.rs`；`rust/src/theta_v0/classifier/retrace_ledger/book.rs`；`rust/src/theta_v0/classifier/retrace_ledger/established.rs`；`rust/src/theta_v0/classifier/retrace_ledger/log.rs`；`rust/src/theta_v0/classifier/retrace_ledger/mod.rs`；`rust/src/theta_v0/classifier/retrace_ledger/portal.rs`；`rust/src/trading/center_book.rs`；`rust/src/trading/third_point_book.rs`。

**依据：**typed retrace observation/truth→RetraceLedger adapter/book/portal/log/audit→CenterBook/ThirdPointBook/ShortRetracePortal；结构 truth ledger 与资金 ledger 同名异物。

**R1 归属：**结构证据层或只读审计支持；资金和发送权限仍只在 R1 对应模块。

- 按 POS016 的 R00805–R00825 逐项判用途；canonical approved_source_registry 已逐 raw 映射：R00805–R00811 是 adapter/audit/book/established/log/mod/portal 七份生产文件，R00812–R00825 是十四份 tests 文件。路径与源哈希见 JSON，另与 KERNEL-FR-OPEN-DEEP-SUBSTRATE.packet_paths 相符；这是冻结元数据核对，不是测试源码重读/运行或当前文件验收。
- 只有满足修订/身份/完整分类域的结构记录可迁移；不得因叫 ledger 就删除或当账户账本。

**登记 ID：**`POS-FR016`。

### OE32 theta_v0 π fill、coverage、TW、campaign 旧组织

**类型：**旧经济核心及挂载状态。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`rust/src/theta_v0/backtest/fill.rs`；`rust/src/theta_v0/backtest/ledger.rs`；`rust/src/theta_v0/backtest/admission.rs`；`rust/src/theta_v0/backtest/econ_positive.rs`；`rust/src/theta_v0/backtest/treasury.rs`；`rust/src/theta_v0/strategy/coverage/`；`rust/src/theta_v0/strategy/account.rs`；`rust/src/theta_v0/strategy/campaign_book.rs`；`rust/src/theta_v0/strategy/short_diff_bucket.rs`；`rust/src/theta_v0/closed_loop/`。

**依据：**π fill 的预算、pending、typed fill、TW、coverage/held、campaign、mu/selector 与额外 closed_loop pass 分别有实效范围；额外结构 pass 不等于真实 fill/TW 来源，声明同状态也不足。

**R1 归属：**R1 分解为结构/操作读取、各重账、公共约束、外部事实与观察；旧聚合调度退出生产。

- 把当前有资金权/状态变化的实际调用逐一对到 R1 单一权威；不得把旧门、mirror、optional seed 当完整恢复证据。
- 62CC/16ES 的输入桥和账务政策未全落地前不判某个现役 helper 已获复用许可；保留比较/否决读数。

**登记 ID：**`ACT-FR012、ACT-FR013、ACT-FR014、ACT-FR015 等 30 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE33 theta_v0 parser/classifier 与因果账支持

**类型：**候选算法/结构支持。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/theta_v0/parser/`；`rust/src/theta_v0/classifier/`；`rust/src/theta_v0/lineage_book.rs`。

**依据：**解析、递归构造、投影、判据、nest/candidate、修订 lineage 和结构 ledger 已有实际消费关系；选定函数可成为 R1 候选实现，现有代码不能裁定其语义。

**R1 归属：**挂靠 R1 Classification/Operation；先按版本化 StaticProofPackage/EvaluationReceipt 判逐函数适配。

- 对 62 分类轴、10 合法组合、32 待清义务逐项证明实现域与实际产出；诊断 relaxed candidate 不进入准入。
- 有状态的 parser/lineage/book 必须保留回滚/修订/身份，不因本行称支持就当纯函数移植。

**登记 ID：**`KERNEL-FR011、KERNEL-FR013、KERNEL-FR014、KERNEL-FR015 等 24 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE34 Gamma、opsem、replay dump 与对拍输出

**类型：**读端和比较入口。**候选去向：**保留为只读观测或诊断旁路；不得参与生产准入、资金分配或外部交易指令。

**冻结路径：**`rust/src/theta_v0/backtest/incremental.rs`；`rust/src/theta_v0/backtest/opsem_dump.rs`；`rust/src/theta_v0/backtest/replay_dump.rs`；`rust/src/theta_v0/backtest/theta_pi_diff.rs`；`rust/src/bin/theta_accept.rs`；`rust/src/bin/theta_backtest.rs`；`rust/src/bin/theta_overlay.rs`；`rust/src/bin/theta_replay.rs`；`rust/src/bin/theta_m1_dual.rs`。

**依据：**Gamma/Opsem/replay/capture 各自输出；登记存在按 run 截断文件与信号面 diff 的边界。CLI 是报告/回放/回测调用面，不构成另一个分类 owner。

**R1 归属：**迁为 R1 O1–O3 同 cut 观测与有版本比较；旧信号差异报告保留受限名分。

- 保留完整分类、账务尾债、首次得知/生效时钟、来源和修订，不让 dump 反向决定 p_star。
- 恢复 run/session 时不能截断历史事实；输出格式改版明确，bit-exact 仅限声明字段域。

**登记 ID：**`ACT-FR078、ACT-FR084、ACT-FR088、ACT-FR089、ACT-FR093、ACT-FR128、KERNEL-FR-CLI-R00630、KERNEL-FR-CLI-R00631、KERNEL-FR-CLI-R00632、KERNEL-FR-CLI-R00633、KERNEL-FR-CLI-R02019`。

### OE35 研究 runner、结构探针与离线 CLI

**类型：**诊断入口。**候选去向：**保留为只读观测或诊断旁路；不得参与生产准入、资金分配或外部交易指令。

**冻结路径：**`rust/src/bin/`；`rust/src/theta_v0/backtest/issue837_probe.rs`；`rust/src/theta_v0/backtest/issue841_probe.rs`；`rust/src/theta_v0/backtest/l3_pi_probe.rs`。

**依据：**CLI 包含因果账、终端前缀、symbol 统计、gauge、旧 dump 重建、浮点/计时/工件封存等；名称 production_orders 也不能证明在发外部单。ACT141/142 是受限 cfg(test) 断言。

**R1 归属：**保留诊断/否决/比较能力；每次产出声明 core/run/proof/配置域。

- 真实 report 入口只读生产快照或独立回放；不能以探针通过替代 FE/RF 与外部闭环测试。
- KERNEL-OPEN-BIN-OTHERS 聚合定位项只挂既有诊断族；其中未逐入口读取的参数/命令不声称已验收。

**登记 ID：**`ACT-FR133、ACT-FR134、ACT-FR135、ACT-FR136 等 70 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE36 Python FugueEngine、scanner_fugue、FSM、路径经济实现

**类型：**旧经济/扫描消费家族。**候选去向：**冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件。

**冻结路径：**`src/newchan/fugue_engine.py`；`src/newchan/trading/scanner_fugue.py`；`src/newchan/trading/cost_reduction_fsm.py`；`src/newchan/trading/paths.py`；`src/newchan/trading/ph_fugue.py`；`src/newchan/backtest/state_machine.py`；`scripts/position_manager.py`；`prototypes/no-gate-two-books/sim.py`。

**依据：**scanner event/路径/四层机/FugueVoice 分别改变旧本地成本与账户；position_manager 有硬编码比例，某些 helper 未挂主链；原型即使形似双账也不是生产架构验收。

**R1 归属：**旧经济/策略组织仅隔离对照；可迁数据/展示职责；K4 producer 仍范围外。

- 不能把旧 confidence、scanner gate 或比例默认为 R1 资金政策；先用签署需求与完整分类映射实际动作。
- 原型/模拟无真实发送权，不因未来替换生产就删除历史研究。

**登记 ID：**`ACT-FR001、ACT-FR022、ACT-FR023、ACT-FR024 等 20 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE37 FeeQuoter / FeeSchedule

**类型：**费用接口支持。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/theta_v0/backtest/treasury.rs`；`rust/src/theta_v0/config.rs`。

**依据：**fee schedule/quoter→VoiceExecBook；Some/None、Taker 与直接角色 API、per_notional/per_share、datum IO 的生效范围不同。

**R1 归属：**可迁为 R1 venue 费用证据/报价适配；共享安全域核版本/有效期，各重记实际费用。

- 核精度/币种/角色/费率单位/舍入与失效；外部费规变化先经 P4 闭门，不能先解释后通知。
- 费用上界与资金政策分别落定；不能把缺 schedule 当零费用。

**登记 ID：**`ACT-FR147`。

### OE38 SeparateLedger / DualLedger 未挂载或测试 API

**类型：**支持库。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/theta_v0/ledger/separate.rs`；`rust/src/theta_v0/backtest/dual_ledger.rs`。

**依据：**SeparateLedger 真实位置轨迹到 nu 的构造仍缺挂载；DualLedger direct API/本地 tests 不等于 π fill 生产消费者。

**R1 归属：**保持已知挂载名分；若复用仅是受 R1 各重账约束的实现部件。

- 不得凭 API 名称宣布双账生产已完成；实际输入账和事件因果桥验收前不挂新权威。

**登记 ID：**`ACT-FR075、ACT-FR124、ACT-FR130`。

### OE39 统计、显著性与指标旁路

**类型：**只读支持。**候选去向：**保留为只读观测或诊断旁路；不得参与生产准入、资金分配或外部交易指令。

**冻结路径：**`rust/src/theta_v0/backtest/metrics.rs`；`rust/src/theta_v0/backtest/decontam.rs`；`rust/src/theta_v0/backtest/pooling_icc.rs`；`rust/src/theta_v0/backtest/prereg_windows.rs`；`trading_system/analyze_fugue_v3_losses.py`。

**依据：**RunResult/ResidualTrade/窗口→metrics/perm_test/decontam/ICC 等读数。诊断输出不能反向成为交易批准。

**R1 归属：**R1 完整事实和同 cut 只读结果。FG-025 已裁的闸门前费后成本相对降幅/占用时间、单边 95% 下界、按月分块 bootstrap、事件≥30、月块≥12、≥3 非重叠窗且各过线、合计≥50% 全史直接承接，不重新等待批准。仅 FU-05 具名的取样语义、同时间多事件、跨闸、短仓基价正负/零时长/C_before≤0 边界及闸门后评价参数保持待澄清；全部盈亏/费用和排除原因先完整保留，不只采正事件。

- 保留分母、窗口、fee/gauge 与实际成交来源；新旧结果不可混池冒充一致样本。

**登记 ID：**`ACT-FR073、ACT-FR074、ACT-FR082、ACT-FR086、ACT-FR087、ACT-FR112、ACT-FR125`。

### OE40 CI、Lean mirror、fixture 与局部测试锁

**类型：**验证设施。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`.github/workflows/ci.yml`；`scripts/check_fixture_drift.py`；`rust/src/bin/strict_nest_check.rs`；`rust/src/theta_v0/classifier/lean_mirror.rs`；`formal/Origin/ScanAssemblyBridge.lean`。

**依据：**冻结登记区分抽象定理、导出 fixture、逐事件 mirror、checkpoint 与负例、普通测试和未执行诊断。锁的声明不等于运行通过。

**R1 归属：**保留验证设施并按 R1 新域补锁；不拥有生产判据。

- 版本化输入域、输出字段、证明边界和实际执行记录；更改 formal 依项目要求跑 fixture drift。
- 本附录没有跑任何这些工具，亦未完整重读每个测试源码/断言。

**登记 ID：**`FRO-LOCK-CI、FRO-LOCK-FIXTURE、FRO-LOCK-ABSTRACT、FRO-LOCK-MIRROR-SOURCE 等 27 条（本组完整集合及 JSON pointer 见同名 JSON）`。

**路径补充出处：**同一 SCAN canonical approved_source_registry：`.github/workflows/ci.yml`＝`R00023`；`scripts/check_fixture_drift.py`＝`R01015`。只核冻结路径/哈希元数据，未重读源码或运行。

### OE41 K4/PH/跨国 topology 及混合统计输入

**类型：**范围边界。**候选去向：**本次交易架构范围外，保留原物；本候选不授予其 R1 生产权限。

**冻结路径：**`src/newchan/topology/`；`src/newchan/backtest/analysis.py`；`src/newchan/backtest/types.py`；`rust/src/theta_v0/backtest/m1_dual.rs`；`rust/src/theta_v0/backtest/tick.rs`；`analysis/k4_1min_lib.py`；`analysis/k4_1min_rust_lib.py`。

**依据：**部分记录同时包含市场 TickFeed/统计与 K4State/scanner/拓扑消费。它们不是一个可整体删除或迁入的生产家族。

**R1 归属：**K4 不进入本 R1 候选；纯行情部分可由 OE02 独立承接，统计按 OE39 只读保留。

- 若后续要接 K4，另行明确需求与授权；当前保持原调用名分，不跨仓追读或导入。
- 不能因保留 K4 研究就保留旧核心的生产交易权限；独立比较身份及数据接口必须明确。

**登记 ID：**`ACT-FR002、ACT-FR081、ACT-FR090、ACT-FR098 等 19 条（本组完整集合及 JSON pointer 见同名 JSON）`。

### OE42 审阅/派发/知识图与部署工具

**类型：**范围外工具。**候选去向：**本次交易架构范围外，保留原物；本候选不授予其 R1 生产权限。

**冻结路径：**`scripts/start_with_gateway.sh`；`scripts/stance_parser.py`；`scripts/consensus_trigger.py`；`scripts/block_topology.py`；`scripts/reflexive_layer.py`；`scripts/chanlun/dag_executor.py`；`scripts/ipfs_deploy.py`。

**依据：**这些是 daemon、审阅协议、块拓扑、历史审计、派发或部署工具；有写文件/发布能力不等于交易发送者。

**R1 归属：**本架构不改动；任何独立部署/写入仍遵其原授权。

- 不从 import/脚本位置推导新交易 owner；不能拿不相干工具数量扩张旧引擎退役分母。

**登记 ID：**`AUX-FR0006、AUX-FR0007、AUX-FR0008、AUX-FR0010、AUX-FR0011、AUX-FR0012、AUX-FR0027、AUX-FR0028、AUX-FR0029、AUX-FR0030、AUX-FR0031、AUX-FR0032、AUX-FR0038、AUX-FR0040`。

### OE43 独立 ledger-adopt / gap-connector 原型

**类型：**隔离原型。**候选去向：**保留为只读观测或诊断旁路；不得参与生产准入、资金分配或外部交易指令。

**冻结路径：**`prototypes/task68-ledger-adopt/src/main.rs`；`prototypes/task68-ledger-adopt/src/lib.rs`；`prototypes/task72-gap-connector/src/main.rs`；`prototypes/task72-gap-connector/src/lib.rs`。

**依据：**独立 Cargo 原型，对 fixture/as_of/candidate 做集成演示；其选择首个、Other 等局部行为不因此进入新完整分类。

**R1 归属：**保留为实验材料；结果不得当 R1 生产定义或已完成证明。

- 若抽取实现，重新证明输入域与语义映射；不自动迁移原型选择规则。

**登记 ID：**`AUX-FR0024、AUX-FR0025`。

### OE44 封存/深层支持库与交易外数据读者

**类型：**支持证据。**候选去向：**挂靠实际调用者的支持模块；逐函数判定复用，不能成为新账本或发送者。

**冻结路径：**`rust/src/lav_seal/`；`rust/src/theta_v0/classifier/`；`analysis/`。

**依据：**seal sample/bundle 与深层65raw/独立 Python reader 的记录是支持/消费者证据，不是额外生产入口。各自具体用途与测试界限仍由冻结登记承担。

**R1 归属：**工件封存、读端/比较或实际旧调用者的支持；不改变 R1 账务/发送所有权。

- 逐函数复用先查所属真实调用者；未逐正文读取的聚合 raw 明确保留未审语义边界。
- 本附录仅作去向挂靠，不宣布这些库已符合 R1，也不广播删除。

**登记 ID：**`KERNEL-FR060、KERNEL-FR-OPEN-DEEP-SUBSTRATE、KERNEL-FR-PYREADER-R00351、KERNEL-FR-PYREADER-R00352、KERNEL-FR-PYREADER-R00353、KERNEL-FR-PYREADER-R00354、KERNEL-FR-PYREADER-R00355、KERNEL-FR-PYREADER-R00356、KERNEL-FR-PYREADER-R00357、KERNEL-FR-PYREADER-R00358、KERNEL-FR-PYREADER-R00359`。

### OE45 topological-computation daemon/前端及代理服务

**类型：**范围外运行入口。**候选去向：**本次交易架构范围外，保留原物；本候选不授予其 R1 生产权限。

**冻结路径：**`topological-computation/`；`src/newchan/codex/`；`src/newchan/gemini/`。

**依据：**剩余 IO 登记涉及 topological-computation daemon/前端、代理审阅和 VCP 等执行设施；与交易 CLI/Bottle/replay 已分开。

**R1 归属：**保留原运行设施，不纳入本交易架构新发送者或旧引擎删除面。

- 此组只声明范围，不据冻结关系声明当前在线；其独立退役/部署须另定范围。

**登记 ID：**`IO-FR015、IO-FR016、IO-FR017、IO-FR018 等 115 条（本组完整集合及 JSON pointer 见同名 JSON）`。

## 未读、未验证与候选终态

- 本轮旧实现行为来源仅为 SCAN-DELIVERY 指定的 canonical CURRENT-ENTRY-CONSUMER-REGISTRY；完整冻结路径/源哈希仅补查同一 SCAN 的 approved_source_registry 元数据，FG-025/FU-05 的名分沿用已签 FUGUE 需求。未读取旧架构候选/归档方案，也未重新扫当前代码或运行项目。
- 1815 是 current_frontiers 关系记录，不是 1815 个入口。1309 条 import-locator 明确不是调用证明，本文件不复制、不计入入口退役。506 个非 import 记录被挂靠 45 组；组有真实入口、分派/支持/验证和范围边界等不同种类，45 也不是可执行入口数。
- 各组代表路径来自冻结登记和该登记已给的入口定位；目录/通配符只表达族。没有据此验证当前文件哈希、当前默认启动、运行进程、broker 账户或挂单。
- 登记原作者 production body 已读的结论只是上游来源陈述；本作者读取了本任务相关登记字段，未冒领逐生产源码或全部 cfg(test) 的阅读/运行覆盖。
- 锁/THC mixed/deep substrate 等聚合项仅做用途挂靠；未展开的逐测试断言、公共 API 全调用域、命令/feature/env 与完整默认 launcher 清单仍未验收。OE13 明列四个 origin_entry_id 的逐 launcher 名称缺口。
- 每一去向都是独立 R1 候选的设计终态。当前未修改代码、构建配置、凭据、账户、任务、票或生产进程；没有删除任何旧事实，也没有执行撤权/发单/撤单。
- 选择保留 adapter 不代表接受其原实现正确；选择隔离旧引擎也不代表全目录可删。纯函数、账务 helper、结构 ledger、配置和诊断须按实际调用者逐项验收。

- 进入 SPEC 前，将本附录的冻结源/登记哈希与拟实施基线核对；只查变动文件、实际 launcher/default/feature/env 及受影响调用边，更新具体实施清单。该核对未执行，不能用本附录声称当前全仓已验证；不为此重启全仓 census。

## 附录校验

已核冻结输入哈希、登记 ID 存在、506 条非 import 关系各归一组、1309 条 import 定位被排除；所要求的 PyRecursive/PyTheta/ThetaRecStrategy、fugue_v3、recursive_t、fusion_v、六组延续 preset、POS015/016、CLI/Bottle/REST/WS/Gamma/paper 均有明确落点。仅作工件校验，未运行代码或项目测试。
