# #1538 考古

[grilling] FU-16 paper/live 与回放同核边界（#1323，新增）

归类：A 已裁 7 · B 有先例未正式裁 7 · C 确未设计 0。只有 C 类可列为真正待决；B 类以「默认值，确认或否决」呈现；A 类直接沿用。

## Q1 [A] 「三模式共享核心」具体包含哪些东西（reducer/身份/责任/执行生命周期/观察协议），正本已否列全？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 同核 | `gh search issues "同核" --repo xy7365527-lang/NewChanlun --json number,title,state --limit 50` | 50 |
| tracker | paper 同核 | `gh search issues "paper 同核" --repo xy7365527-lang/NewChanlun --limit 100` | 10 |
| repo | 同核 | `rg -n '同核' docs/adr .chanlun/definitions CONTEXT.md` | 12 |
| repo | 环境差异只允许 | `rg -n '环境差异只允许' docs/adr docs/unified-architecture-canon.md` | 2 |

命中：
- docs/adr/0027-classification-chong-operating-program-ownership.md:46-48：历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。……环境差异只允许落在后续裁定的行情、撮合和场所 adapter。adapter 的责任范围与具体接口不在本 ADR 内。
- #1340 body C10.2（:345）：历史、paper、live 共用上述核心 reducer、身份、责任、执行生命周期和观察协议；只替换真实变化的 feed、clock、matcher/venue、传输与受控持久化 adapter。
- .chanlun/definitions/chong-traceability.md:64：#1538 | 三模式保持同一结构与经营核心，只替换已裁 adapter；同输入核对本票的逐重语义结果。
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:49-54：全状态六面：S 结构快照…各重 B 账…E 责任…订单/成交原件…物理持仓…外效历史

- 出处：ADR 0027 裁定三（:46-48）定两个 owner 共用；#1340 C10.2 列全五项共享面（reducer、身份、责任、执行生命周期、观察协议）；身份=ADR 0051 domain-event/1，执行生命周期=#1340 C07（OwnAttempt/MayHaveCalled），观察协议=#1340 C08；ADR 0038:49-54 六面全状态即共享核心的可比较面。本票只需引用，不重裁。

## Q2 [A] 运行环境在哪一点分叉：由什么字段决定装哪套 adapter，装配权归谁？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | evidence_class | `gh search issues "evidence_class" --repo xy7365527-lang/NewChanlun --limit 100` | 12 |
| repo | evidence_class | `rg -n 'evidence_class' docs/adr CONTEXT.md .chanlun/definitions` | 19 |
| repo | evidence_class（代码） | `rg -n 'evidence_class' rust/src trading_system scripts tests` | 1 |

命中：
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:270-272：一份 manifest：已批准规则和政策版本、能力 manifest、原始输入来源、各域 DB 路径、时钟、adapter 选择（C10.1）……`evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合，`experiment` 和 `production` 装场所 adapter。三种运行环境只在 adapter 处分叉（ADR 0027:46）。
- CONTEXT.md:371-372：证据类（Evidence Class）…`simulation`（确定性回放/受控模拟）、`experiment`（独立实验部署）、`production`（目标生产部署）。低类证据不得被读为高类有效
- docs/adr/0028-first-capability-verification-profile.md:28：任一字段变更即新版本；`environment` 与 `execution_form` 不得在版本内切换。
- rust/src（代码）：`evidence_class` 在 rust/src 仅 1 处命中且为 lean 镜像测试名；launcher/manifest 装配代码在 main 上不存在（#1769 在飞）

- 出处：ADR 0039 裁定三·2（:270-272）：分叉字段=profile.evidence_class，装配权=唯一 launcher `production_session` 的 manifest「adapter 选择」段；ADR 0028 裁定一版本内不得切换；ADR 0052 已裁沿用 Q14（:250）历史=P0/simulation。

## Q3 [A] feed adapter 的落点：结构源 feed（S 入口）与执行行情 feed（X 侧）是否已分离，各自跨模式换什么？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | feed clock matcher | `gh search issues "feed clock matcher" --repo xy7365527-lang/NewChanlun --limit 100` | 4 |
| tracker | 结构源 执行源（#1523 评论） | `gh issue view 1523 --repo xy7365527-lang/NewChanlun --json comments` | 7 |
| repo | feed/clock | `rg -n 'feed.*clock.*matcher\|feed/clock' docs .chanlun analysis` | 10 |
| repo | 结构源与执行源 | `sed -n '150,156p' docs/adr/0020-*.md; sed -n '28,36p' docs/adr/0023-*.md` | 2 |

命中：
- docs/adr/0020（重写的裁定二 :152-154）：结构/决策层：消费美股 tick（回测 = Databento 历史全史；实盘 = Databento 实时流）；执行层：成交在股票永续（映射标的），价差/滑点 = 执行成本项
- /Volumes/AgentStorage/decisions/1523/draft-28G.md:170（用户 2026-10-28 批准，#1523 评论 6452353777）：Q7 结构源与执行源分离，S 独立推进。……S 的结构源在全部 profile 下都是个股线美股 tick 或显式 TestOnly 结构输入。paper 与 live 的实时结构源沿 ADR 0023:30-34，已由 #1044 落地……HL 永续行情与回报只属执行层
- #1769 body :12：manifest 的「原始输入来源」只声明 S 的结构源……场所执行行情（HL 永续成交与 bar）只能出现在 X 的 adapter 选择段。……判据是来源身份，与 adapter 名称无关。
- docs/adr/0052:§六：`input_source` 是可调字段…默认 `massive-stock-tick/1`…可选值 `databento-equs-mini-tick/1`

- 出处：ADR 0020 重写裁定二 + ADR 0023 实时流四条 + #1523 28G Q7（已批准）+ #1769:12：结构 feed 只进 S.AcceptInput（C02.1 原件形状，历史=ADR 0052 §六 input_source，paper/live=Massive 实时 WS #1044），执行行情 feed 只进 X adapter 选择段；来源以 source_namespace 判别。本票引用即可，不重裁。

## Q4 [B] clock adapter：simulation 已裁 logical-bar/1，paper/live 用哪种时钟 kind？代码 E/B manifest 现只认 System/TestOnly，如何对齐？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 时钟 adapter | `gh search issues "时钟 adapter" --repo xy7365527-lang/NewChanlun --limit 100` | 2 |
| tracker | logical-bar | `gh search issues "logical-bar" --repo xy7365527-lang/NewChanlun --limit 100` | 1 |
| repo | logical-bar/1\|逻辑时钟 | `rg -n 'logical-bar/1\|逻辑时钟' docs/adr .chanlun` | 6 |
| repo | manifest clock kind（代码） | `rg -n '"clock"\|clock\.kind\|"System"' rust/src/economic_session/store.rs rust/src/economic_session/service.rs` | 7 |

命中：
- docs/adr/0052-history-input-profile-and-model-order-unspecified.md:294-296：`clock.kind = "logical-bar/1"`。在 `simulation` 下，S/B/E/X 不读系统时钟，`received_at` 取原件值。checkpoint、seek 和 `known_frontier_cut` 只认 `bar_index`
- rust/src/economic_session/store.rs:253-258：match required(&manifest["clock"], "kind")? { "System" => (), "TestOnly" => {…} _ => return Err("InvalidDomain：必须显式指定时钟adapter".into()), }
- rust/src/economic_session/store.rs:263：if manifest["clock"]["kind"] != "TestOnly" { return Err("InvalidDomain：故障暂停仅TestOnly".into()); }
- rust/src/economic_session/service.rs:686-688：config.clock(Some(msg), "received_ns"…) / "first_known_ns" / "commit_ns"
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:172：任何一层都不含 wall-clock 或 PID

- 出处：ADR 0052 裁定四（simulation=logical-bar/1）；ADR 0038:56（checkpoint 取 bar 收盘）；ADR 0051:172（身份不含 wall-clock）；代码 store.rs:253-263 已有显式「时钟adapter」缝，kind ∈ {System, TestOnly}。
- 推理链：[新缠论:推论] ①ADR 0027:48 把时钟差异划归 adapter，ADR 0039:270 把「时钟」列进 manifest ⟹ 时钟是 manifest 选择的 adapter，不是各域自读。②ADR 0052 §四 只为 simulation 定了 logical-bar/1；experiment/production 的 received_ns/first_known_ns/commit_ns 必须是真实到达时间（C02.1 保留 received_at 原始值、INTERFACE-CONTRACTS:9「原始 received 时间、真实 first_known、持久提交序号分别保留」）⟹ paper/live 的时钟 kind 只能是 System（已在代码）。③ADR 0051:172 身份层不含 wall-clock ⟹ System 时钟只进时间戳字段，不进 identity，双跑按 ADR 0038:47 比较时时间戳属「非语义字段」。④代码 TestOnly 是 plan 驱动假时钟且绑故障暂停（store.rs:263），与 ADR 0052 §五 fault_schedule（simulation 下）同构但名字不同。
- 默认值：时钟 adapter 三 kind 封闭枚举，由 evidence_class 一一绑定：simulation→`logical-bar/1`（ADR 0052 §四，received_at 取原件、checkpoint 只认 bar_index）；experiment/production→`System`（store.rs:317 现有实现，只写时间戳字段，不进 identity）；`TestOnly` 只许测试夹具与 fault 计划，禁止出现在任何 production_session 装配的 profile。实施义务：store.rs:253 枚举补 `logical-bar/1` 并把 fault_schedule 的「故障暂停」从 TestOnly 解绑到 simulation（归 #1769/#1381），本票不改代码。
- 已搜关键词：时钟 adapter、logical-bar、clock kind、逻辑时钟
- 冲突：ADR 0052 §四 定 simulation 时钟 kind=`logical-bar/1`，但 main 代码 store.rs:253-258 只接受 System/TestOnly、其余值 InvalidDomain；且 store.rs:263 把故障注入绑死在 TestOnly 时钟上，而 ADR 0052 §五 把 fault_schedule 放在 simulation profile 下。两边未对齐，不能归 A。

## Q5 [A] matcher/venue adapter 的 owner 落点：受控撮合与场所 adapter 是否都在 X 进程内，E/B 是否不持撮合？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 受控撮合 | `gh search issues "受控撮合" --repo xy7365527-lang/NewChanlun --limit 100` | 12 |
| tracker | simulation adapter | `gh search issues "simulation adapter" --repo xy7365527-lang/NewChanlun --limit 100` | 1 |
| repo | 模拟 matcher | `rg -n '模拟 matcher\|受控撮合' docs/adr` | 4 |
| repo | trait Matcher/Venue/Adapter（代码） | `rg -n -i 'trait .*Adapter\|trait .*Venue\|trait .*Feed\|trait .*Matcher' rust/src trading_system` | 0 |

命中：
- docs/adr/0052:279：X 的模拟 matcher 只做一个不变式断言：收到的命令在 `input_domain` 内。……它不判成交、不回落，也不算第二个判定点。
- #1340 body C09.7（ADR 0039:285 转引）：X 的适配器在 X 内部受控调用，不能委托另一个仍有直发权的异步后台
- docs/adr/0039:284-285：#1523 的「正式 paper launcher」交付物按本 ADR 读作：`production_session` 与 `x_execution_session` 骨架，加 X 内的 HL adapter。……只能是被 X 进程内部调用的 adapter 模块
- #1718 body：fill_rule={cross(k≥1),touch}，默认cross…touch只在诊断域合法；生产准入与正式验收入口拒绝touch配置/结果

- 出处：ADR 0039:272/:284-285 + #1340 C09.7：场所 adapter 与受控撮合都在 X 内部；ADR 0052 §二·5 模拟 matcher 在 X；撮合模型本体=ADR 0033（bar 批、严格穿越）与 #1718 cross 规则组件；#1770（C-API）交 X 内 HL adapter 离线合同。代码现无任何 Matcher/Venue trait（0 命中），seam 由 #1769/#1770 新立。

## Q6 [B] 「传输 adapter」跨模式换什么：域间传输（S/E/B/X 之间）是否随模式变，还是只有 X↔场所的网络传输变？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | session_transport | `gh search issues "session_transport" --repo xy7365527-lang/NewChanlun --limit 100` | 5 |
| tracker | 持久化 adapter | `gh search issues "持久化 adapter" --repo xy7365527-lang/NewChanlun --limit 100` | 3 |
| repo | session_transport | `rg -l 'session_transport' rust/src --glob '*.rs'` | 11 |
| repo | pub trait（transport） | `rg -n 'pub trait' rust/src/session_transport.rs rust/src/economic_session/finance_transport.rs` | 2 |

命中：
- rust/src/session_transport.rs:1-3：#1374：每个 S/E/B 独立实例复用的有界 Unix 输送与进程排他锁。输送线程只解帧和排队；权威 handler 在调用线程串行执行。
- #1340 body C09.1：进程间走带版本和身份的本地消息协议，消息到达不等于持久提交。
- #1340 body C09.7：X 是唯一持有场所写凭据并执行网络请求的进程……SDK 对写请求的隐式重试、后台提交及恢复重投必须逐层核实并纳入一次调用合同
- /Volumes/AgentStorage/decisions/1523/draft-28G.md:273：本地夹具端点收到的请求体与 rust/tests/fixtures/hl_adapter/requests/ 下按 HL API 文档写成的金样逐字节相等

- 出处：#1340 C09.1（同宿主本地消息协议，不分模式）、C09.7（唯一网络请求在 X）、C10.2「只替换真实变化的…传输」；代码 session_transport.rs 为三域共用 Unix 输送，无模式分支。
- 推理链：[新缠论:推论] ①C09.1 对三模式一律同宿主独立进程+本地协议，所以域间传输没有「真实变化」，按 C10.2 不替换。②唯一随环境变化的传输是 X 对外：simulation 下撮合在 X 进程内（无网络）、experiment/production 下是场所 HTTP/WS（C09.7）；#1523 28G 的本地夹具端点正是这条传输的测试替身。③把域间传输也列为可换 adapter 会制造第二套 S/E/B 接线，违反 AGENTS.md:48 不得两档。
- 默认值：「传输 adapter」= 仅 X↔场所一段：simulation=进程内受控撮合（零网络），experiment/production=场所 HTTP/WS 客户端（SDK 重试/幂等按 C09.7 纳入一次调用合同）。域间 session_transport 三模式同一实现，只有 TransportBounds 数值随 manifest 走，不算 adapter 替换。
- 已搜关键词：session_transport、传输 adapter、本地消息协议

## Q7 [B] 「受控持久化 adapter」跨模式换什么：三模式是否都用 C09.2 的每域 SQLite/WAL，simulation 可否用别的存储？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 持久化 adapter | `gh issue list --repo xy7365527-lang/NewChanlun --state all --search "受控持久化 in:comments" --limit 20` | 3 |
| repo | 受控持久化\|持久化 adapter | `rg -n '受控持久化\|持久化 adapter' docs .chanlun` | 3 |
| repo | writer.lock/writer_epoch | `rg -n 'writer_epoch\|writer\.lock' rust/src/session_transport.rs rust/src/economic_session/store.rs \| head` | 6 |

命中：
- #1340 body C09.2：每个域一份独立 SQLite 数据库和 WAL，由该域单写者进行有限事务；不使用 ATTACH 跨域原子提交……所有权威提交采用 WAL + synchronous=FULL
- docs/adr/0039:292：持久域层：S/E/B/X 每个域一个库、一个 `.writer.lock`（`session_transport.rs:53`）。……持久 `writer_epoch` 不等时被拒（`store.rs:466-467`）
- docs/adr/0038:60：恢复：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar

- 出处：#1340 C09.2 对三模式无区分；ADR 0039 裁定四·1 的写权锁第一义依赖每域库+锁+epoch；ADR 0038 H2 杀停恢复要求 simulation 也走持久 checkpoint。
- 推理链：[新缠论:推论] ①ADR 0038 H2（SIGKILL 恢复）是 TB-07 必跑 oracle，simulation 若换成内存存储则 H2 无法成立 ⟹ simulation 必须用同一持久实现。②ADR 0039 裁定四·1 的「失去写权第一义」靠 .writer.lock+writer_epoch，换存储等于换锁语义。③因此「受控持久化 adapter」真实变化的只有 manifest 里各域 DB 路径、fsync 前件（fullfsync 等启动校验）与数据目录，不是存储实现。
- 默认值：持久化 adapter 不按模式替换：三模式同一 SQLite/WAL 单写者实现（C09.2）。可变项只允许 manifest 的各域 `--db` 路径与启动前件校验；simulation 不得降级为内存或关 fsync（否则 H2 失效）。「受控」一词读作「启动校验不满足拒写」（C09.2），不是「可换实现」。
- 已搜关键词：受控持久化、持久化 adapter、writer_epoch

## Q8 [B] 「同输入语义结果相同」的验证机制：跨模式（simulation vs paper/live）比较哪些面、怎么构造同输入、哪些字段允许不同？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 同输入 逐字段 | `gh search issues "同输入 逐字段" --repo xy7365527-lang/NewChanlun --limit 100` | 0 |
| tracker | 跨模式 | `gh search issues "跨模式" --repo xy7365527-lang/NewChanlun --limit 100` | 100 |
| tracker | #1382 AC（TB-08-A） | `gh issue view 1382 --repo xy7365527-lang/NewChanlun --json body --jq .body` | 1 |
| repo | 逐字段相等\|逐字段 | `rg -n '逐字段相等\|逐字段' docs/adr .chanlun/definitions` | 10 |
| repo | 跨模式（代码） | `rg -n '跨模式\|cross.mode' rust/src tests` | 2 |
| repo | simulation_and_hl_adapters_share_the_x_call_edge | `rg -n 'simulation_and_hl_adapters_share_the_x_call_edge' docs .chanlun rust` | 0 |

命中：
- #1382 body :12：相同受控事件经过两个模式的适配层，各核心语义输出逐字段相等；模拟fill与真实fill不能在同会话同时为权威。
- #1340 body C10.4（:347）：双跑固定原始输入及修订、政策/模型版本、时钟、随机种子、数值语义、故障与调度轨迹，比较完整结构关系、逐重状态/毛账、责任、命令/回报/分配和 AsKnown。允许不同运行 ID 等非语义字段经过预先声明的一一重命名；禁止只比较最终净仓。
- docs/adr/0038:47：固定五元组（input, policy_version, model_id/revision, seed, scheduler）下，两次独立运行在每个 checkpoint 的全状态必须逐位等价。
- /Volumes/AgentStorage/decisions/1523/draft-28G.md:220（已批准）：测试 `simulation_and_hl_adapters_share_the_x_call_edge` 用同一 ExactCommand 夹具，经 X 唯一调用边分别走 simulation adapter 与 HL adapter（本地夹具端点）。X 侧调用记录除 adapter 身份与场所回执外逐字段相等，ADR 0039 A4 写权闭包 diff 为空。
- .chanlun/review-results/issue1340-publication-20260909/view/inputs/RD-17-MODEL-PLAN.md:60：TB-08：仅共享核心/可替换 adapter 的跨模式验收中实际消费该历史模拟模型的对照成功

- 出处：ADR 0038 裁定一/四（同模式 H1 六面逐位）；#1340 C10.4（比较面与允许重命名）；#1382 AC :12（跨模式逐字段相等）；#1523 28G :220（X 调用边对拍，由 #1525 在本票裁定后实现）。
- 推理链：[新缠论:选择] ①同模式双跑已由 ADR 0038 H1 闭合，本票只补「跨模式」。②跨模式天然不可能六面全等：第 4-6 面（成交原件、物理持仓、外效历史）由场所决定（C10.2 模拟 fill 与真实 fill 不同权威）。③能要求相等的是 adapter 之前的面：S 面（1）、命令投影（ExactCommand 直到 X 调用边，28G :220 已定「除 adapter 身份与场所回执外逐字段相等」）。④B/E 面（2、3）依赖回报，要让它们可比只能把场所回报当原件再喂给 simulation：录放（record/replay）——把 paper/live 会话的外效 journal（第 6 面）作为受控撮合的「回报脚本」重放，此时 1-6 面应逐位相等；这正是 C10.2「模拟 fill 不能与真实 fill 同会话同为权威」允许的形态（不同会话）。⑤C10.4 允许预先声明的一一重命名，用于 run id / received_ns（Q4 System 时钟）等非语义字段。
- 默认值：三层 oracle：(a) 同模式：沿用 ADR 0038 H1，不重裁；(b) 跨模式前段：同一结构原件集与五元组下，simulation 与 experiment/production 会话的 S 面与 ExactCommand 到 X 调用边逐字段相等，差异只允许 adapter 身份、场所回执与预先声明重命名的非语义字段（28G :220 的测试即此面，归 #1525）；(c) 跨模式后段「回报录放」：取 paper/live 会话的外效 journal 作受控撮合的回报脚本，在另一个 simulation 会话重放，六面与原 paper/live 会话逐位相等——这是 TB-08-A（#1382 :12）「各核心语义输出逐字段相等」的二值落法。禁止只比净仓（C10.4）。
- 已搜关键词：跨模式、逐字段相等、同输入、record replay、回报录放

## Q9 [A] 模拟 fill 与真实 fill 的权威排他：同一会话内是否可同时存在两种成交源？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 模拟 fill（评论） | `gh issue list --repo xy7365527-lang/NewChanlun --state all --search "模拟 fill in:comments" --limit 10` | 10 |
| repo | 模拟 fill | `rg -n '模拟 fill\|simulated fill\|模拟fill' docs .chanlun analysis` | 62 |

命中：
- #1340 body C10.2（:345）：模拟 fill 不能与同会话真实 fill 同时充当权威。
- docs/adr/0038:74：实验证据不得读为生产有效，simulation 产出不进 TStage 触闸计数
- docs/adr/0052:250（已裁沿用 Q14）：唯一外部事实源由 adapter 装配锁定（#1340 C10.2 :345；ADR 0028:28；ADR 0038:74；ADR 0039:272）

- 出处：#1340 C10.2 + ADR 0038 裁定三·2 + ADR 0052 已裁沿用 Q14：一个会话一个 evidence_class，一个外部事实源，由 launcher 装配锁定；混源只能是另一会话（见 Q8(c) 录放）。

## Q10 [B] E 接纳场所回报的 profile 形状：paper/live adapter 产出的原件要对齐 E.CaptureExternal 哪份合同？代码 E 现只收 TestOnly 经济原件。

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | CaptureExternal | `gh search issues "CaptureExternal" --repo xy7365527-lang/NewChanlun --limit 100` | 12 |
| repo | TestOnly 经济原件 profile（代码） | `rg -n 'TestOnly' rust/src/economic_session/book.rs \| head -5` | 5 |
| repo | source 六字段 | `rg -n 'source_namespace' rust/src/economic_session/book.rs \| head` | 4 |

命中：
- rust/src/economic_session/book.rs:106-107：if doc["format"] != "neutral TestOnly economic raw originals; not production transport wire" { return Err("SchemaUnsupported：仅已声明 TestOnly 经济原件 profile".into()); }
- /Volumes/AgentStorage/decisions/1523/draft-28G.md:446：C-API 的……输出对齐 main 上 E 原件的 `source` 字段，不发明 #1539 的归属语义，也不预设 #1534 的事件合同。
- docs/adr/0051:244：`source = {source_namespace, source_epoch, external_fact_id, source_cursor, received_time}`
- .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:35：E.CaptureExternal | 原字节或已持久可读 blob、可信连接身份/范围、token、source epoch/cursor、received 时间 | 同事务写 Recorded 和保守 Pending

- 出处：INTERFACE-CONTRACTS E.CaptureExternal 行；ADR 0051:244 source 形状；#1523 28G F2（C-API 输出对齐 E 原件 source 字段）；代码 book.rs:106-107 为现状。
- 推理链：[新缠论:推论] ①同核要求 E 对三模式只有一个接纳合同（C10.2 共享「身份」）；②ADR 0051:244 已给唯一 source 形状，28G 已令 HL adapter 输出对齐它；③book.rs:106 的 format 字符串是 TestOnly 闸，不是第二套合同——按模式再立一套 profile 即两档（AGENTS.md:48）。
- 默认值：E 原件 profile 只有一份 schema（ADR 0051:244 source + book.rs:117-123 的 allocation_id/family_id/重身份），`format` 改为按 evidence_class 具名的三值（simulation/experiment/production 原件），E 校验 format 与会话 profile 一致否则 SchemaUnsupported；受控撮合与 HL adapter 都产这份原件，不产第二形状。实施归 #1770/#1425，归属语义仍归 #1539。
- 已搜关键词：CaptureExternal、TestOnly 经济原件、source_namespace

## Q11 [B] S 域内部单调时钟与传输 deadline（job.rs Clock、TransportBounds Duration）在 simulation 下是否破坏确定性，归语义还是归故障？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | deadline 确定性 | `gh search issues "deadline 确定性" --repo xy7365527-lang/NewChanlun --limit 100` | 4 |
| repo | trait Clock / Instant::now | `rg -n 'trait Clock\|Instant::now' rust/src/s_content/job.rs rust/src/session_transport.rs` | 8 |
| repo | SystemTime::now（域代码） | `rg -n 'SystemTime::now' rust/src/bin/s_structure_session.rs rust/src/economic_session/store.rs rust/src/economic_session/content_store.rs` | 5 |

命中：
- rust/src/s_content/job.rs:286-288：/// 单调时钟缝：生产写 `Box::new(Instant::now)`，测试注入假时钟（本模块不 sleep）。 pub trait Clock { fn now(&self) -> Instant; }
- rust/src/session_transport.rs:30-39：pub struct TransportBounds { pub frame: usize, pub read: Duration, pub write: Duration, pub response: Duration, …}
- rust/src/bin/s_structure_session.rs:126-132：fn make_token() … SystemTime::now()… format!("{}:{nanos}", std::process::id())  // Begin 唯一推进 token
- docs/adr/0052:298-300（裁定五）：`kind ∈ {crash_before_commit, crash_after_commit, reorder_reports, drop, duplicate, restart, writer_epoch_bump, store_unreadable, store_write_failure}`…故障只来自声明，不来自随机

- 出处：ADR 0038 裁定一（五元组内逐位等价；非语义字段可重命名）；ADR 0052 裁定五（故障只来自声明）；代码 job.rs:286 单调时钟缝与 session_transport.rs TransportBounds。
- 推理链：[新缠论:推论] ①job.rs Clock 与 TransportBounds 是资源/期限闸，不进任何事件内容；超期结果是 StorageUnavailable 类失败，按 ADR 0027:32「失败只能是失败」不分叉语义。②若 simulation 双跑一次超期一次未超期，六面会 diff ⟹ 按 ADR 0038 H1 判 FAIL，这是正确的（暴露资源故障），不需要把它写进五元组。③make_token 的 PID+ns 是 Begin token，属 C10.4「运行 ID 等非语义字段」。
- 默认值：把单调时钟/期限归入「资源故障」而非 adapter：simulation 下不替换 job.rs Clock 与 TransportBounds 实现，但 profile 的 resources 段必须取足够宽的上界并写入 content_hash；期限触发即故障、双跑 diff 即 FAIL；Begin token/PID 列入 C10.4 预先声明的重命名字段。不新增 ADR 0052 §五 的 kind。
- 已搜关键词：Instant::now、TransportBounds、单调时钟、deadline

## Q12 [A] 旧 Python 回放/session/API 外壳「迁为同核 adapter」是否在本票裁？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 同核 adapter 外壳 | `gh search issues "外壳 同核 adapter" --repo xy7365527-lang/NewChanlun --limit 100` | 6 |
| repo | 迁为同核 adapter | `rg -n '迁为同核 adapter' docs/adr .chanlun` | 3 |

命中：
- .chanlun/definitions/chong-traceability.md:52：现有 Python 运行与发送路径只用于旧入口终态核查，不能成为第二个正式 launcher 或决策主干。终态实施归 TB-10 #1368，三模式同核与外壳接入归 #1538、#1550。
- docs/adr/0039:241-243：`newchan` CLI…收编：只读查询壳 adapter（C09.1、C11.2），改读 S 发布 | 永不持 manifest 和凭据
- docs/adr/0052:§三·4：#1528 旧正文（要接 `run_theta_v0_pi` 生产链）和 #1607 比较器的去向，登记交给 #1550 裁决。

- 出处：ADR 0039 裁定二表（每入口终态）、ADR 0048 模块级终态、chong-traceability.md:52（外壳接入归 #1538/#1550，终态实施归 TB-10）：本票只定「同核 adapter」的边界定义，旧外壳逐条去向已裁且归 #1550/TB-10，不重开。

## Q13 [B] 正本落点：新 ADR 还是追加到 ADR 0039/0052，CONTEXT.md 需补哪些词？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 正本 ADR 0038 0052 落点 | `gh search issues "正本落点 ADR" --repo xy7365527-lang/NewChanlun --limit 100` | 30 |
| repo | adapter 词条 | `rg -n -i 'adapter' CONTEXT.md` | 2 |
| repo | ADR 编号末位 | `ls docs/adr \| tail -1` | 1 |

命中：
- docs/adr/0038:1-3：> 正本：本文件是双跑确定性、恢复语义、seek 不偷未来、AsKnown 分证协议的教义正本。> 最后裁定：#1537
- docs/adr/0052:1-5：ADR 0052…T 类裁定，文书待 main 合入。本 ADR 只定 profile 字段、终态与判据；实现与运行证据归 TB-07 叶票
- CONTEXT.md:365-372：「生产」拆三义…场所 profile…证据类（Evidence Class）
- .agents/skills/domain-modeling/SKILL.md：Only offer to create an ADR when all three are true: Hard to reverse / Surprising without context / The result of a real trade-off

- 出处：ADR 0038/0052 同系先例（FU-15/FU-14 各落一份新 ADR）；ADR 0027:48「adapter 的责任范围与具体接口不在本 ADR 内」留给后续；CONTEXT.md 现无 adapter 五类词条（仅「生产拆三义」提及 broker adapter）。
- 推理链：[新缠论:选择] ①本裁定是跨 ADR 0027/0038/0039/0052 的接缝合同，难逆、需上下文、有取舍（Q4/Q6/Q7/Q8 都排除过候选）⟹ 满足 domain-modeling 三条；②追加到 0039 会把 launcher 清单与 adapter 边界混在一起，0052 只管历史 profile。
- 默认值：新开 docs/adr/0053-three-mode-shared-core-and-adapter-boundary.md（头三行沿 0038/0052 格式），裁定分：共享核心面、五类 adapter 的 owner 落点与跨模式替换表（Q3-Q7）、时钟 kind 枚举（Q4）、跨模式三层 oracle（Q8）、受影响票与边；CONTEXT.md「运行边界」节补「同核 adapter（Shared-Core Adapter）」一条词条，只列五类名与 owner，不写接口。
- 已搜关键词：正本落点、adapter 词条、ADR 0053

## Q14 [A] 旧票 #1329（C3 同核与 Adapter seam，open/ready-for-human）与本票重叠，如何登记而不重裁？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | Adapter seam | `gh search issues "Adapter seam" --repo xy7365527-lang/NewChanlun --limit 100` | 2 |
| tracker | #1329 评论 | `gh issue view 1329 --repo xy7365527-lang/NewChanlun --json comments` | 1 |
| repo | Adapter seam | `rg -n 'Adapter seam\|adapter seam' docs analysis .chanlun` | 7 |

命中：
- #1329 评论 5596449009：旧 C1–C6 设计排程继续暂停；本票不再按旧候选组织恢复两方案选型。……本票原义务的真实接单处：TB-07 #1366、TB-08 #1367、TB-09 #1365、TB-11 #1369。
- #1340 body C12.3：旧 #1327–#1333 仍是暂停的历史设计 DAG，不直接重新打上拾取标签。
- gh api …/issues/1538/dependencies/blocking：[1539,1540,1550,1362,1366,1367,1525]

- 出处：#1329 评论 5596449009 + #1340 C12.3：#1329 暂停、义务已移交 TB-07/08/09/11；本票 resolution 在 #1329 登记一条指针即可，不解除其暂停、不关它。

## 机械校验

通过。
