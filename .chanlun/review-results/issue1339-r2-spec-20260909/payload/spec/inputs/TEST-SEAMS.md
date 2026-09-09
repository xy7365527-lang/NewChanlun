# #1323 R2 完整 SPEC：现有行为测试缝输入

可复用决策核回放/三面对拍工具、候选生命周期、完成态日志恢复、WS 初始快照和毛账算式的局部测试。没有据这些先例确认 R2 的生产 launcher→结构事实→资金责任→成交回填→恢复→客户端观察整链已获验收。下列所有覆盖判断来自源代码阅读，本轮未运行测试或 launcher。

- 仓库：/Users/silencehan/Projects/NewChanlun
- HEAD：6df8d1921c72c84da90987f8f6c02ebc11fe6aac
- 方法：先核 MCP 当前 Rust 子目录索引；索引缺项时回落当前文件。未使用其他 worktree 图。
- 本材料是 SPEC 输入；仅写本 inputs 目录，未修改仓库、未执行测试。

## 12 个当前测试/装配先例

### T01 · rust/src/bin/theta_accept.rs

层级：CLI 装配/报告入口；不是测试。

入口/测试：[main](/Users/silencehan/Projects/NewChanlun/rust/src/bin/theta_accept.rs:313)；[run_symbol](/Users/silencehan/Projects/NewChanlun/rust/src/bin/theta_accept.rs:114)；[SymbolReport::verdict](/Users/silencehan/Projects/NewChanlun/rust/src/bin/theta_accept.rs:75)

实际覆盖：run_symbol:156-165 复用 run_replay_double，再 run_batch/run_stream/diff；可复用按样本窗口和品种组织的报告入口。

未覆盖：没有启动 live launcher、Nautilus 真实成交回填、前端订阅或跨进程恢复。:161 含不可交易 bar 即 diff_authoritative=false；:65-86 此时对拍不阻断 PASS，标签明确含 inconclusive，不能作为完整验收成功。

执行门与复用限制：CLI required feature=backtest_bin；本轮未执行。

SHA-256：dedee90b260ccdb3d22e90fbe7b49b860f7d13b5ccaa3c3841574dcb463185f2；Git blob：e31d4ca9860a8dcbbe72222f106bd902e9555d98。

### T02 · rust/src/theta_v0/backtest/replay_dump.rs

层级：决策核回放装配/确定性比较函数；不是测试。

入口/测试：[replay](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/replay_dump.rs:143)；[run_replay_double](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/replay_dump.rs:241)

实际覆盖：两份独立 ThetaPiStream 同一进程逐 bar 回放；:149-150 使用上一根 p_star 作 p_t、固定 initial_nav。:170-205 终态声部按身份排序并输出账本读数；:252-260 两次 dump 字节比较并保留首分歧。

未覆盖：不是两个 OS 进程或持久化重启。没有真实成交/部分成交时序，不是 Chong 毛账恢复。dump 的 bar 行仅 p_star/action/qty/exec_index，不能证明完整结构 snapshot/delta 或生命周期发布。

执行门与复用限制：当前可复用稳定排序、环境档案、首分歧定位；新验收不得沿用 self-state 假定冒充资金反馈。

SHA-256：336692c86110ff870856dd701ea9882ce9b2c1f0670e5a29bdbfc5ecd687ea84；Git blob：19c74bac31f0bf67b318b8c1a5c5cec8f542eec6。

### T03 · rust/src/theta_v0/backtest/theta_pi_diff.rs

层级：库内跨组件对拍 + diff 引擎单位负对照。

入口/测试：[replay_vs_batch_signal_surface_bit_exact_and_report](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/theta_pi_diff.rs:582)；[expected_diff_rules_are_machine_encoded](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/theta_pi_diff.rs:600)；[diff_engine_flags_unexpected_state_divergence](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/theta_pi_diff.rs:662)；[replay_vs_batch_structured_report](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/theta_pi_diff.rs:737)

实际覆盖：batch:278 调真实 pi_theta_fill_loop_overlay；stream:326 逐 bar 喂批量侧 p_t/equity_nav。:582 硬断言分类/tower 等信号面零差；:662 手造单 bar 篡改 sep_legs，要求恰一条预期外状态差。机械规则分桶可复用为诊断工具。

未覆盖：常规测试:582 只断言 n_signal_diffs()==0；状态/账本差异只打印。结构化测试:737 标 #[ignore]，且仍只断言信号零差。当前允许 TW/TypedTrade 仅 batch 有、风控/候选过滤/级联等预期差，不能直接带入新 R2 完整同义行为验收。浅锯齿可能不产腿，非真空资金覆盖不成立。

执行门与复用限制：replay_vs_batch_structured_report 默认忽略；本轮均未执行。

SHA-256：2b32cfe9d1c869913b4eeb913d6b82b5a34e580f3304d1289e8fc316198a27d1；Git blob：b14d096e34e764835b57000da823fc1426cecf5d。

### T04 · rust/src/theta_v0/classifier/six_state.rs

层级：纯分类函数有限枚举/工作例单位测试。

入口/测试：[six_state_bot_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:122)；[six_state_inside_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:131)；[six_state_above_no3b_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:140)；[six_state_above_b3_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:149)；[six_state_below_no3s_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:158)；[six_state_below_s3_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:167)；[six_state_exhaustive_and_bot_dimension](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:177)；[signal_2b3b_coexist_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:248)；[signal_1b2b_exclusive_parity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/six_state.rs:264)

实际覆盖：固定中枢[100,200]，None/Some×7个价格×b3×s3，共56组上下文；包含端点，检查无中枢恒 Bot 和六个符号均可达。另有 BSP 位集合兼容/互斥工作例。

未覆盖：有限枚举不是所有完整分类轴验收；parity 名称不表示运行了 Lean。测试直接喂 last_center/price/b3/s3，不证明真实 bar 装配会调用六态分类并原样发布，亦不证明跨级同源、snapshot/delta 字段完备。

执行门与复用限制：可复用字面值反例，但完整需求每一轴须另列可达见证及输出事实。

SHA-256：8f172502ffc2bb743b23d893e07c5998b8ab260915eaf58fd6034b6f55b905a4；Git blob：213af9727d5535dd3ab5771c686265238f186cbd。

### T05 · rust/src/theta_v0/classifier/cand_event/book.rs

层级：候选事件簿状态机测试；局部扫描到事件簿的跨组件先例。

入口/测试：[append_only_revisions_keep_identity_and_clocks_and_same_as_of_is_idempotent](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:362)；[confirmed_clock_uses_first_observation_as_of_not_geometry_clock](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:382)；[interval_shrink_appends_invalidation](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:408)；[disappearance_appends_invalidation_and_terminal_never_revives](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:420)；[same_key_transitions_unresolved_to_provisional_and_pins_first_provable_clock](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:432)；[first_provable_clock_survives_fallback_to_unresolved](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:485)；[rule_version_bump_mints_new_key_and_never_revives_terminal_one](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/cand_event/book.rs:500)

实际覆盖：稳定 key、revision/supersedes、观察钟/首证钟/确认钟、同 as_of 幂等；回缩与消失追加 Invalidated，终态 key 不复活；版本变化新 key。:432 从真实局部扫描 fixture 产生 Unresolved→Provisional，并断言非真空探针。

未覆盖：这是候选对象族，不是全部结构对象的统一观察协议。未覆盖 public snapshot/delta 游标重连、进程间投递、落盘重启、浏览器撤回渲染；Invalidated 也不能无映射地泛化为所有领域的撤回。

执行门与复用限制：可复用局部生命周期工作例与非真空命中断言。

SHA-256：156b01c5ca72f99e6edc72cdd4665edd9d3d4f5584cdfbbbc89d5a01c0b0f2f3；Git blob：7b3eb2069b38a80f10ed6e1e18a07dd34eaefb99。

### T06 · rust/src/theta_v0/classifier/level_view_store.rs

层级：文件存储 + 完成态 reducer 的进程内恢复测试。

入口/测试：[append_only_prefix_and_completed_freeze_reject_write](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/level_view_store.rs:632)；[reopen_and_replay_twice_are_idempotent](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/level_view_store.rs:676)；[legacy_v0_migrates_on_reopen](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/classifier/level_view_store.rs:695)

实际覆盖：写第二个完成事件只追加字节；修改冻结走势产生 CompletedFreezeViolation 且零写入；reopen 与原冻结表相同；同事件 replay 两次等于一次；v0 载荷读取迁移。

未覆盖：fixture 直接构造 LevelAsOfView，保护范围只有 completed freeze。没有进程被杀、截断/损坏文件、fsync 时序、候选撤回或全观察协议恢复；更不覆盖资金账本或成交幂等。

执行门与复用限制：可复用持久化往返、冲突零写、两次恢复不重复的测试形态。

SHA-256：89a33979ebd73735057d40ad4881e511ab49bf6a258b682625c2530e9e7dfb14；Git blob：90e117fe4c164b831e6f3797c62d7634caef7689。

### T07 · tests/test_gateway_live.py

层级：FastAPI TestClient 的进程内 HTTP/WS 边界测试。

入口/测试：[TestWsLiveConnect.test_connect_with_cache](/Users/silencehan/Projects/NewChanlun/tests/test_gateway_live.py:80)；[TestWsLiveDisconnect.test_disconnect_removes_client](/Users/silencehan/Projects/NewChanlun/tests/test_gateway_live.py:107)；[TestWsLivePing.test_ping_pong](/Users/silencehan/Projects/NewChanlun/tests/test_gateway_live.py:121)；[TestOnLiveBar.test_engine_processes_bar](/Users/silencehan/Projects/NewChanlun/tests/test_gateway_live.py:140)；[TestLiveEngineReuse.test_engine_reused](/Users/silencehan/Projects/NewChanlun/tests/test_gateway_live.py:171)；[TestLiveStatusEndpoint.test_status_reflects_connected_client](/Users/silencehan/Projects/NewChanlun/tests/test_gateway_live.py:212)

实际覆盖：真实路由 /ws/live/{symbol} 初始 snapshot、bar_idx、断连清理、ping/pong、缓存预热后引擎复用、HTTP live 状态。_load_bars 被替换，内部使用 RecursiveOrchestrator。

未覆盖：不是 R2/Theta 同源观察发布；测试未断言全量分类事实、delta 版本/游标/撤回/证据链。没有浏览器消费者或网络进程；_on_live_bar 的两个测试仅断言计数/缓存，并未读取实时 WS delta。

执行门与复用限制：可复用对外 WS/HTTP 驱动方式；必须替换到采纳架构指定的正式观察入口。

SHA-256：c1e4b1620c5b361b6f35cfb42b3f8b76b60437bab242209a8c15a969474369b8；Git blob：85c2b0d334dc44ece57696c49a4e1a7a1017ba27。

### T08 · rust/src/theta_v0/strategy/chong.rs

层级：毛账算式与重身份单位测试。

入口/测试：[posted_margin_symmetric_legs_full_polarity](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/chong.rs:237)；[net_savings_not_deployable](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/chong.rs:259)；[cost_state_not_in_key](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/chong.rs:279)；[register_independent_quota_fail_loud](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/chong.rs:300)；[single_chong_degenerates_to_account_net](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/chong.rs:317)

实际覆盖：字面工作例：两个重各+50/-50，price=100、Lmax=10，posted_margin=1000、posted_notional=10000；净额抵消不得释放名义资金；平一腿只释放该腿计提；成本演进不换重键，重复/非法开户拒绝。

未覆盖：直接 set_position_lots；没有订单责任分配、venue fill/fee、成交去重、in-flight、全局预算并发、失败重隔离或重启恢复。单重退化测试不能代替多重生产接线。

执行门与复用限制：上述数值仅是既存工作例，不是建议的资金默认值或新资金公式。

SHA-256：10f275046901df9143548c1b400a73dc6386824ed12e86f83173e5cb2a376e4d；Git blob：39f0d236e60ddeb7dc8ab9a01ccbabd833f6390a。

### T09 · tests/test_trading_system_skeleton.py

层级：旧桥接持久化组件测试与日志单次生命周期测试。

入口/测试：[TestPersistence.test_bar_cache_roundtrip_and_dedup](/Users/silencehan/Projects/NewChanlun/tests/test_trading_system_skeleton.py:145)；[TestPersistence.test_crash_recovery_replay_matches_snapshot](/Users/silencehan/Projects/NewChanlun/tests/test_trading_system_skeleton.py:159)；[TestPersistence.test_recovery_mismatch_fails_fast](/Users/silencehan/Projects/NewChanlun/tests/test_trading_system_skeleton.py:191)；[TestPersistence.test_trade_journal_signal_and_order_lifecycle](/Users/silencehan/Projects/NewChanlun/tests/test_trading_system_skeleton.py:212)

实际覆盖：bar 缓存按时间去重保留首值；500根合成bar的 ChanlunBridge 第二实例从同一数据库缓存重放，结构快照/水位相同；缓存只存5根而快照到10根时抛 RecoveryMismatch；一次 log_signal→order→fill 后断言 FILLED 与 signals=1。

未覆盖：crash 测试没有启动/杀死进程，是同进程构造第二个桥。未证明 Theta/R2 一致，也未验证 venue 对账、Chong 毛账恢复；日志只填一次，没有重复成交/部分成交/手续费/改撤单交错。

执行门与复用限制：模块开头:12 pytest.importorskip('nautilus_trader.model')，缺依赖会整模块跳过；本轮不确认实际执行数。

SHA-256：66600bdd2c6086ecbcc429c9953e4048f2cb8eb0414a76bc8e5d3b220926abf5；Git blob：d73a03afc788abc0e11ee958702e02194a181c77。

### T10 · trading_system/persistence/trade_journal.py

层级：成交日志装配/缺口证据；无本文件测试。

入口/测试：[TradeJournal.log_fill](/Users/silencehan/Projects/NewChanlun/trading_system/persistence/trade_journal.py:47)；[TradeJournal.upsert_voice_state](/Users/silencehan/Projects/NewChanlun/trading_system/persistence/trade_journal.py:67)；[TradeJournal.total_position](/Users/silencehan/Projects/NewChanlun/trading_system/persistence/trade_journal.py:89)

实际覆盖：log_fill 仅以 client_order_id、ts、price、qty 插入 fills 并把订单置 FILLED；voice_state 有覆盖写与净持仓求和出口。

未覆盖：接口没有 venue execution id/Chong 责任/fee，也未在本函数中去重；每次 log_fill 都写 FILLED，单靠它不能表示部分成交生命周期。:71-72 confirm_fill 接线仍为 TODO。没有证据可将此日志认定为 R2 成交幂等或权威毛账。

执行门与复用限制：只作为新验收必须穿过真实 fill 边界的动机，不将旧注释当新架构裁定。

SHA-256：1ce73e3db6b9f79f0a9c9149a6cbbd1c1ffaff95f5c7a811d998673d8a8a7df1；Git blob：aaa172052932ad6ab1445038eb432fd384057c66。

### T11 · trading_system/strategy/theta_rec_strategy.py

层级：现有 Nautilus 策略装配；无本文件测试。

入口/测试：[ThetaRecStrategy.on_bar](/Users/silencehan/Projects/NewChanlun/trading_system/strategy/theta_rec_strategy.py:73)；[ThetaRecStrategy._nav](/Users/silencehan/Projects/NewChanlun/trading_system/strategy/theta_rec_strategy.py:100)；[ThetaRecStrategy._rebalance](/Users/silencehan/Projects/NewChanlun/trading_system/strategy/theta_rec_strategy.py:107)；[ThetaRecStrategy.on_stop](/Users/silencehan/Projects/NewChanlun/trading_system/strategy/theta_rec_strategy.py:126)

实际覆盖：bar 水位去重后读 Portfolio 净仓，传 ThetaStream.push_bar；_rebalance 对目标与净仓差额 market 提单；on_stop close_all_positions。模块:14 明示成交回执不回填 Rust。

未覆盖：没有按 Chong 责任写回的成交入口、partial fill 幂等或毛账恢复；_nav:101-107 有 initial_nav 兜底，不能把它当采纳 R2 的资金可靠性规则。策略类存在不表示被 production launcher 装配和验收。

执行门与复用限制：可在隔离模拟 venue 中用作旧行为负对照定位；新 SPEC 依据已采纳架构决定退役/限制使用的范围。

SHA-256：27cd12bb42a16dd9c01a3b632c595279d8be2973fc929dfcf51db289218bd3c0；Git blob：9cb50196a3dbbd372fce98d5f9a9949416edf6f4。

### T12 · trading_system/live/runner.py

层级：live CLI 骨架/退役隔离检查对象；无本文件测试。

入口/测试：[main](/Users/silencehan/Projects/NewChanlun/trading_system/live/runner.py:31)；[__main__](/Users/silencehan/Projects/NewChanlun/trading_system/live/runner.py:72)

实际覆盖：main 的 TradingNode/build/run 只在 docstring 伪代码中；:61 输出尚未实装，:69 返回1。

未覆盖：不能证明生产运行，更没有 fail-before-submit 的退役隔离行为测试。骨架文字与 return1 不能证明其他配置/包装/导入路径不能激活旧策略。

执行门与复用限制：新生产 launcher 验收应从正式外部入口启动，再用退役目标配置做负对照；只读本文件不能推导全仓旧路径已经不可达。

SHA-256：b83fba8cdd7f426a1d5fb38e5b871f34f265d78505fcee55afe5da34cff3dc62；Git blob：418b8893ec18aff9509d2c0085a804552c0bcd4a。

## 建议纳入完整 SPEC 的最高行为缝

### A01 · 最高缝：正式生产 launcher 的受控整链运行

使用 R2 已采纳装配对应的正式外部入口。固定小型输入事件档案、规则/配置版本、账户与 venue 夹具，分别以两个新进程运行，采集对外结构事实、订单责任、模拟 venue 接收记录、成交后每重毛账及观察输出。禁止跳过正式 launcher 直接调用内部策略来取得整链 PASS。

硬判据：逐事件身份、顺序与规范字段确定性；非真空出现结构形成/修订/撤回和至少两重真实责任/成交；最终毛账与 venue 夹具独立账簿相符。仅实际墙钟耗时、性能测量及预先声明的非语义运行身份一一重命名可排除；语义发生、获知、确认、接纳、应用时刻/前沿及其固定clock关系必须硬比较，其余语义字段差异均硬判。

可复用先例：T01, T02, T03。
当前缺口：现有 self-state 回放只覆盖决策核；需追加正式生产装配到执行反馈与观察两支的整链验收。

### A02 · 完整分类与同源观察缝

从同一正式入口送入各需求轴的已审定正负/端点见证，由公共 snapshot 与 cursor delta 读取事实和证据引用；观察期待值来自需求/权威边界工作例，不从被测判定函数再次计算。

硬判据：所有完整分类行都有非空/未决/已证/撤回等适用态的明确期待；适用的互斥分类恰一项、允许共存的信号不被强行互斥。把 delta 作用于基线 snapshot 后必须等于同版本/同水位新 snapshot；所有对象/修订引用能回查同一来源证据。

可复用先例：T04, T05, T06, T07。
当前缺口：目前局部测试不能证明全部分类轴已经接到生产快照、delta 与显示。

### A03 · 重连、重复投递与生命周期恢复缝

用 WS/查询客户端记录 snapshot 与游标，跨确认/撤回断连；重连重复投递、旧版本游标、窗口变更与进程重启均经真实协议驱动。

硬判据：合法游标无丢事件、重复不会重复生效、版本不匹配按采纳协议明确拒绝或重建；已撤回身份不会因重放复活；冻结完成态不得被未来数据回写。浏览器视图另验撤回消失及证据下钻，不用 TestClient 冒称界面验收。

可复用先例：T05, T06, T07。
当前缺口：全观察协议持久化与浏览器行为本轮未发现现成完整先例。

### A04 · Chong 成交责任、幂等与受损恢复缝

在模拟 venue 的外部回执边界注入分批 fill、重复 execution、乱序回执、改撤单与在途交错、费用及 crash/restart；输入参数采用已采纳协议，不在测试输入生成器里补裁默认资金语义。

硬判据：同一成交身份只影响一次对应 Chong 责任与费用；部分成交仅释放/落实相应责任；不同重反向持有仍各自计提且净额节省不能重新使用。逐个持久化边界重启与不中断基线相等；单重故障但最大占用仍可证时，健全重必须出现新开操作；共享资本约束不可证时拒绝依赖它的新操作，不能伪造安全余额。

可复用先例：T08, T09, T10, T11。
当前缺口：现有毛账是直接赋仓算式测试，成交日志是单次 happy path；二者拼接不能证明成交权威闭环。

### A05 · 生产退役隔离缝

以正式 launcher 的实际配置/插件/路由选择入口驱动：合法 R2 模式完整运行；每个禁止的旧执行目标配置都在建立市场会话/提交订单之前可观察地失败。检查外部模拟 venue 的 connect/submit 记录，不只检查 import 文本。

硬判据：被禁止配置不能产订单或资金写入，也不能默默转为宽松旧策略。允许保留的历史/研究入口按采纳范围明确隔离，其存在不自动等同生产可达。

可复用先例：T11, T12。
当前缺口：当前只见旧策略与骨架入口，未核得完整生产退役行为锁。

## 验收报告口径

- 本轮所有测试均未执行；无测试通过数、CI 绿色或运行验收结论。
- 只存在测试代码、测试名含 parity/e2e/crash、CLI 打 PASS，均不提升证据层级。
- 现有三面对拍的 expected/advisory 规则只作旧行为诊断；新 R2 的硬验收条件须由已采纳契约重列。
- 同输入对拍可锁回归，不能单独证明两侧同源错误不存在；必须同时保留独立工作例、负对照和非真空命中。
- 若后续执行时 importorskip、#[ignore]、不可交易窗口 advisory 或空结构触发，应单独报告 skipped/inconclusive，不计作完整覆盖。


## S1 独评后的根修订

范围独评指出 A01 把“时间”整体排除会掩盖 AsKnown 错误。已限定只有非语义耗时/性能和预先声明的运行身份重命名可以排除；语义时刻与各域前沿全部硬判。原初稿在 snapshots/s1-initial-79b6aba5 中保留，12个当前代码先例的阅读和未运行声明不变。
