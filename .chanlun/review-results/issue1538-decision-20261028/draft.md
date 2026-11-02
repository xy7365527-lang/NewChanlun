## 给用户的决策摘要

本票原是 T 类（技术判定），起草与独审两轮各被 BLOCK 一次后按 graph-engineering 规则转为 P 类，等你批。两轮 BLOCK 都落在引用缺行号、受影响票清单漏票、测试锁判据写法（R1/R3/R6）；裁定方向本身在两轮独审中 R2、R4、R5、R7、R8 均为 PASS。本稿已按第二轮审查补齐这三处。要你点头的是下面四个选择：

1. **时钟随证据类绑定，不当自由度**：历史回放（`simulation`）用逻辑时钟 `logical-bar/1`，paper（`experiment`）与 live（`production`）用系统时钟；`TestOnly` 时钟只留给组件测试，正式 launcher 拒收。这会让 main 代码（`store.rs` 目前只认 `System`/`TestOnly`）向 ADR 0052 对齐，由 #1769/#1381 改码。
2. **只有 X 对场所那一段随模式换**：域间传输、每域 SQLite/WAL 持久化三模式同一实现，回放也不许用内存库或关 fsync；撮合/场所 adapter 只在 X 进程内。
3. **E 只有一份原件 schema**：受控撮合与各场所 adapter（HL、IBKR paper、Binance 只读）产出同一形状，`format` 只是来源标签，E 校验它与会话证据类一致，不一致即拒收。
4. **跨模式「同输入同结果」的测试锁分三层**：(a) 同模式沿用 ADR 0038 双跑；(b) 场所之前的面（S 结构面、X 调用边命令）直接逐字段比；(c) 场所之后的面用「回报录放」——把 paper/live 会话录下的场所回报在另一个回放会话里重放，六面逐字段比，只许八项预先列明、各有必然成因的字段改名（见裁定七 (c) 表），其余任何差异判 FAIL。录放落为模型取值 `recorded-venue-replay/1`，只在回放对拍会话合法。

**推荐：四条全批。** 理由：1–3 是 ADR 0027:46/48、ADR 0039:272、ADR 0052:296、#1340 C09/C10 已裁内容的直接推论，换别的写法要么违反已裁正本，要么在同一判断上造出两档实现（AGENTS.md:48）；第 4 条是 #1382:12「各核心语义输出逐字段相等」唯一能二值落地的办法——只比 (b) 覆盖不到 B/E 面，要求跨模式六面无改名全等则因场所回报与模拟撮合不可能相同而恒 FAIL。

**不批的后果**：#1538 不关，它直接阻塞的 7 张票（#1362、#1366、#1367、#1525、#1539、#1540、#1550）和经 #1366/#1382 传递阻塞的 #1382、#1400、#1523、#1524 继续等；main 时钟代码与 ADR 0052 的冲突无人裁，#1769 只能自选口径；#1382 的跨模式验收没有二值判据。若只否其中一条，请指明哪条，其余可照批，第 4 条 (c) 的重命名闭集用到第 1 条的时钟绑定与第 3 条的 `format` 标签，否掉 1 或 3 时 (c) 的闭集须随之重写；1、2、3 之间互不依赖。

## 问题

#1538（FU-16）要裁两件事：

1. 历史、paper、live 三模式共用一个核心。随模式替换的只有 feed、clock、matcher/venue、传输、受控持久化几类 adapter。每类 adapter 的 owner 落点、跨模式换什么、哪些部分不换，要逐类定清。
2. 「同输入、语义结果相同」用什么二值测试锁来验证。

ADR 0027 裁定三已经定了两个核心 owner 共用，环境差异只能落在 adapter。该 ADR 同时写明「adapter 的责任范围与具体接口不在本 ADR 内」，留给后续裁定。ADR 0052:353 也登记本票消费其裁定一和裁定三 (d)。

考古查出两处冲突，本票必须对齐：

- **冲突一（时钟，考古 Q4）**：ADR 0052 裁定四（:296）定 simulation 用 `clock.kind = "logical-bar/1"`，而且「不设可选值」。main 代码 `rust/src/economic_session/store.rs:253-258` 只认 `System` 和 `TestOnly`，其他值一律 `InvalidDomain：必须显式指定时钟adapter`。另外 `store.rs:263-264` 把故障注入绑在 `TestOnly` 时钟上（`故障暂停仅TestOnly`），但 ADR 0052 裁定五（:300）把 `fault_schedule` 放进 simulation profile 的 `scenario_contract`。
- **冲突二（E 接纳原件，考古 Q10）**：`rust/src/economic_session/book.rs:106-107` 只接受 `format` 为「neutral TestOnly economic raw originals; not production transport wire」的原件，其他一律 `SchemaUnsupported`。按 #1340 C10.2（:345），三模式要共用「身份」，E 对外部事实就只能有一个接纳合同。现在代码实际上只开了 TestOnly 一档。

另记一处行号漂移（不影响裁定）：ADR 0039:292 写「持久 `writer_epoch` 不等时被拒（`store.rs:466-467`）」。main 上 `store.rs:466-467` 现在是 `statfs` 代码，`writer_epoch` 的比对在 `store.rs:653`。本稿引用写权 epoch 时以 `store.rs:653` 为准。

本票考古里没有 C 类点。B 类点按默认值呈现。本票分级为 T 类，但两轮独审 BLOCK 后已按 graph-engineering.md:73 转 P 类，B 类默认值由用户批复后生效，不再依 N-1 预授权自行采纳。

## 权威依据

1. **两个核心 owner 三模式共用，环境差异只落在 adapter**：docs/adr/0027-classification-chong-operating-program-ownership.md:46「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。」:48「环境差异只允许落在后续裁定的行情、撮合和场所 adapter。adapter 的责任范围与具体接口不在本 ADR 内。」
2. **失败就是失败，不换判据重判**：docs/adr/0027-classification-chong-operating-program-ownership.md:32「失败必须作为失败处理，不能切到另一套判据、旧入口或宽松路径重新判定。」
3. **共享面五项与可换 adapter 名单**：[#1340](https://github.com/xy7365527-lang/NewChanlun/issues/1340) 正文 C10.2（:345）「历史、paper、live 共用上述核心 reducer、身份、责任、执行生命周期和观察协议；只替换真实变化的 feed、clock、matcher/venue、传输与受控持久化 adapter。模拟 fill 不能与同会话真实 fill 同时充当权威。」
4. **唯一完整会话入口**：#1340 C10.1（:344）要求载入「存储/时钟和执行 adapter」，并按 S→操作→B→E→X→事实→B→观察推进。
5. **双跑比较面与允许重命名**：#1340 C10.4（:347）「比较完整结构关系、逐重状态/毛账、责任、命令/回报/分配和 AsKnown。允许不同运行 ID 等非语义字段经过预先声明的一一重命名；禁止只比较最终净仓。」
6. **adapter 能力证据**：#1340 C10.5（:348）「每个 paper/live adapter 提供能力证据……唯一外部事实来源。缺项准确 Unsupported；历史通过不表示场所已通过。」
7. **TestOnly 不证明完整经营**：#1340 C10.3（:346）「TestOnly 顺序可用于协议试验，但不能证明完整经营验收。」
8. **独立进程与本地协议**：#1340 C09.1（:330）「进程间走带版本和身份的本地消息协议，消息到达不等于持久提交。」
9. **每域 SQLite/WAL**：#1340 C09.2（:331）「每个域一份独立 SQLite 数据库和 WAL，由该域单写者进行有限事务……锁/持久化/文件系统前件启动时验证，不满足则拒绝相应写入。」
10. **X 是唯一网络发送者**：#1340 C09.7（:336）「X 是唯一持有场所写凭据并执行网络请求的进程……X 的适配器在 X 内部受控调用……SDK 对写请求的隐式重试、后台提交及恢复重投必须逐层核实并纳入一次调用合同」。
11. **结构标准输入**：#1340 C02.1（:259）「标准输入包含 source namespace/epoch、instrument、原始事件身份和修订、原始坐标、received_at……执行市场价格不得偷偷改变结构级别语义。」
12. **最高测试缝**：#1340 C12.1（:363）「以最高完整会话入口为主测试缝」。#1340 C12.3（:382）「旧 #1327–#1333 仍是暂停的历史设计 DAG，不直接重新打上拾取标签。」
13. **唯一 launcher 与装配分叉点**：docs/adr/0039-production-entry-inventory-and-unique-launcher.md:266（bin `production_session`）；:270「一份 manifest：已批准规则和政策版本、能力 manifest、原始输入来源、各域 DB 路径、时钟、adapter 选择（C10.1）」；:272「`evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合，`experiment` 和 `production` 装场所 adapter。三种运行环境只在 adapter 处分叉」；:274（独立 OS 进程，各域自有 `--db`、`.writer.lock`、`writer_epoch`）；:284-285（X 内 HL adapter，不得有独立 `__main__` 发送路径）；:288（以 `production_session` 拉起的独立进程为最高缝，直调域 bin 只算组件证据）；:292（持久域层：每域一库、一 `.writer.lock`）。
14. **双跑六面与 oracle**：docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:47「固定五元组……两次独立运行在每个 checkpoint 的全状态必须逐位等价」，六面见 :49-54；:56（checkpoint 取每 bar 收盘后）；:60（恢复从 checkpoint 加载六面并重放）；:74「simulation 产出不进 TStage 触闸计数」；:75「分证不交叉引用」；:80 H1；:81 H2；:84 反例判据。
15. **历史 profile、逻辑时钟、故障、模拟 matcher**：docs/adr/0052-history-input-profile-and-model-order-unspecified.md:250（已裁沿用 Q14：历史 profile 为 P0/`evidence_class=simulation`，唯一外部事实源由 adapter 装配锁定）；:264-269（六域引用清单）；:279「X 的模拟 matcher 只做一个不变式断言……它不判成交、不回落，也不算第二个判定点。」:285（验证入口只有 `production_session`，以 `evidence_class=simulation` 装配受控撮合 adapter）；:296 逻辑时钟；:300 `fault_schedule`；:306-308 `input_source` 可调字段；:353 本票消费项。
16. **身份不含 wall-clock 与 source 形状**：docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:172「任何一层都不含 wall-clock 或 PID」；:244 `source = {source_namespace, source_epoch, external_fact_id, source_cursor, received_time}`。
17. **profile 版本内不切换**：docs/adr/0028-first-capability-verification-profile.md:28「任一字段变更即新版本；`environment` 与 `execution_form` 不得在版本内切换。」:25 `P1-hl-testnet` 的 `evidence_class = experiment`。
18. **结构源与执行源分层**：docs/adr/0020-backtest-cash-live-perp-filtered.md:153-154（结构/决策层消费美股 tick；执行层在股票永续，价差/滑点是执行成本项，不进结构）；docs/adr/0023-data-layer-spec-two-lines-two-sources.md:11（个股线源 = Massive）；:30-34（实时流接入四条）。
19. **#1523 已批准的 28G 稿**（用户 2026-10-28 批复，[评论 6452353777](https://github.com/xy7365527-lang/NewChanlun/issues/1523#issuecomment-6452353777)）：/Volumes/AgentStorage/decisions/1523/draft-28G.md:170（Q7 结构源与执行源分离，HL 永续行情与回报只属执行层）；:220（`simulation_and_hl_adapters_share_the_x_call_edge`，由 #1525 在本票裁定后实现）；:446（C-API 输出对齐 main 上 E 原件的 `source` 字段）。该批复评论另写明「本票保留全部原义务：正式 paper 成功、paper/live 同核……」。
20. **manifest 来源判据**：[#1769](https://github.com/xy7365527-lang/NewChanlun/issues/1769) 正文 :12「manifest 的『原始输入来源』只声明 S 的结构源……场所执行行情（HL 永续成交与 bar）只能出现在 X 的 adapter 选择段……判据是来源身份，与 adapter 名称无关。」:13（S 结构源声明在 init 时持久化，运行中不切换）。
21. **跨模式验收原文**：[#1382](https://github.com/xy7365527-lang/NewChanlun/issues/1382) 正文 :11「仅替换实际变化的feed/clock/matcher/venue/transport及受控持久adapter，核心reducer、身份、权责和观察协议相同。」:12「相同受控事件经过两个模式的适配层，各核心语义输出逐字段相等；模拟fill与真实fill不能在同会话同时为权威。」
22. **撮合规则组件**：[#1718](https://github.com/xy7365527-lang/NewChanlun/issues/1718) 正文 :17-18（`fill_rule={cross(k≥1),touch}`，默认 cross；touch 只在诊断域合法，生产准入与正式验收入口拒收）。
23. **#1524 交付面**：[#1524](https://github.com/xy7365527-lang/NewChanlun/issues/1524) 正文 :2（HL trade tick → 1m 唯一 feed 接口）；:7「与 #1304/#1305 回放 driver 共核」。
24. **#1525 的同核验收项**：[#1525](https://github.com/xy7365527-lang/NewChanlun/issues/1525) 正文 :26「paper/live 共用同核证明（无旁路执行路径）」。
25. **追溯合同的本票行**：.chanlun/definitions/chong-traceability.md:64「#1538 | 三模式保持同一结构与经营核心，只替换已裁 adapter；同输入核对本票的逐重语义结果。」:52（「三模式同核与外壳接入归 #1538、#1550」，终态实施归 TB-10 #1368）；:50（`production_session.rs` 由 #1523 交付，不编造行号）；:58（「不删除边」先例）。
26. **接口合同**（SPEC 附件）：.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:9（received 时间、first_known、提交序号分别保留）；:21（S.AcceptInput 是 S 唯一输入接纳器）；:35（E.CaptureExternal：同事务写 Recorded 和保守 Pending）。
27. **词汇**：CONTEXT.md:364（「运行边界」节）；:374（证据类三值；低类证据不得读为高类有效）。行号按 main `0b3097aae8`；ADR 0054 入 main 时 CONTEXT.md 在 :176 前插入两行，旧稿 :362/:372 随之漂到 :364/:374。
28. **收敛通则与总缝**：AGENTS.md:48（#799「同一个判断不得存在宽严两档实现」）；AGENTS.md:70（#804 判别式「两边输出必须逐位相同」）。
29. **自由度通则**：docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md:13（判别句）；:15「可调字段不得有『主值失败时的备用值』」。
30. **代码现状**（只读事实）：
    - `rust/src/economic_session/store.rs:251`：manifest `resources` 各项必须为正；
    - `store.rs:253-258`、`:263-264`：时钟 kind 与故障绑定；
    - `store.rs:317-318`：`System` 时钟读 `SystemTime::now()`；
    - `store.rs:653`：`writer_epoch` 比对；
    - `rust/src/economic_session/book.rs:93`：`fact_key` 定义；
    - `book.rs:106-107`：TestOnly format 闸；
    - `book.rs:117-124`：`event`、`allocation_id`、`family_id`、`target` 必填，`:124` 调 `fact_key`；
    - `rust/src/economic_session/service.rs:686-688`：`received_ns`、`first_known_ns`、`commit_ns` 三个时钟读数；
    - `rust/src/session_transport.rs:1-3`（三域复用的有界 Unix 输送）、`:33-40`（`TransportBounds`）、`:56`（`.writer.lock`）；
    - `rust/src/s_content/job.rs:286-289`：单调时钟缝 `trait Clock`；
    - `rust/src/bin/s_structure_session.rs:126-133`：`make_token` 含 PID 与纳秒。
31. **#1329 暂停登记**：[评论 5596449009](https://github.com/xy7365527-lang/NewChanlun/issues/1329#issuecomment-5596449009)。旧 C1–C6 设计排程继续暂停；该票原义务的真实接单处是 TB-07 #1366、TB-08 #1367、TB-09 #1365、TB-11 #1369；保留义务含「paper/live同核适配边界」。
32. **本票既有承接登记**：[评论 6404224455](https://github.com/xy7365527-lang/NewChanlun/issues/1538#issuecomment-6404224455)（消费 #1533 追溯合同本票行，正式路径为 `production_session` 拉起 S/E/各 Bᵢ/X）；[评论 6465991201](https://github.com/xy7365527-lang/NewChanlun/issues/1538#issuecomment-6465991201)（分级：T）。

## 候选

**A 类，已裁沿用，不列为待决**

- Q1 共享核心面：#1340 C10.2（:345）列全五项（reducer、身份、责任、执行生命周期、观察协议）。可比较的面就是 ADR 0038:49-54 的六面。
- Q2 分叉字段与装配权：分叉字段是 `profile.evidence_class`，装配权在唯一 launcher `production_session` 的 manifest「adapter 选择」段（ADR 0039:270/:272）。版本内不切换（ADR 0028:28）。
- Q3 feed 分离：结构 feed 只进 S.AcceptInput（C02.1 :259；INTERFACE-CONTRACTS:21）。执行行情只进 X 的 adapter 选择段（#1769:12；28G:170）。按来源身份判别。
- Q5 matcher/venue 在 X 内：ADR 0039:284-285；C09.7（:336）；ADR 0052:279/:285。撮合规则组件见 #1718:17-18。
- Q9 权威排他：C10.2（:345）；ADR 0052:250；ADR 0038:74。
- Q12 旧外壳去向：chong-traceability.md:52、ADR 0039:241/:243 已裁。本票只定同核 adapter 边界，逐条去向归 #1550/TB-10 #1368。
- Q14 #1329：评论 5596449009 与 C12.3（:382）已定暂停与义务移交。本票只登记指针。

**B 类，默认值，确认或否决（本票已转 P 类，待用户批复，推理链见下）**

- **Q4 时钟 adapter**：默认值是三值封闭枚举，由 `evidence_class` 一一绑定：`simulation→logical-bar/1`，`experiment/production→System`；`TestOnly` 只用于直调域 bin 的组件测试夹具，`production_session` 拒收（出处：ADR 0052:296、`store.rs:253-258/:317-318`、ADR 0051:172、ADR 0039:288）——确认或否决。
  - 推理链 `[新缠论:推论]`：
    1. ADR 0027:48 把环境差异划给 adapter，ADR 0039:270 把「时钟」列进 manifest，所以时钟是 manifest 选的 adapter，不由各域自读。
    2. ADR 0052:296 只为 simulation 定了 `logical-bar/1`，并且「不设可选值」。experiment/production 的 `received_ns/first_known_ns/commit_ns`（`service.rs:686-688`）必须是真实到达时间和真实提交时间（INTERFACE-CONTRACTS:9「分别保留，不能互相替代」），只能用 `System`（现有实现 `store.rs:317-318`）。
    3. ADR 0051:172 规定身份层不含 wall-clock，所以 `System` 读数只进时间戳字段，不进 identity。
    4. `TestOnly` 只能证明协议（C10.3 :346）；直调域 bin 只算组件证据（ADR 0039:288）。
    5. 时钟 kind 由 `evidence_class` 推出，不是自由度，不落成可调字段（ADR 0041:13）。
  - 冲突一的对齐义务（不在本票改码）：`store.rs:253` 的枚举要补 `logical-bar/1`；`store.rs:263` 的「故障暂停仅TestOnly」要改成 `fault_schedule` 只在 `evidence_class=simulation` 的会话里合法，`TestOnly` 时钟下的现有夹具行为不变。实施归 #1769（manifest 校验）和 #1381（`fault_schedule` 触发）。
- **Q6 传输 adapter**：默认值是只有 X↔场所这一段随模式替换：simulation 是 X 进程内受控撮合（零网络），experiment/production 是 X 内场所 HTTP/WS 客户端（SDK 重试与幂等按 C09.7 纳入一次调用合同）；S/E/B/X 之间的 `session_transport` 三模式同一个实现，`TransportBounds` 的数值走 manifest，不算 adapter 替换（出处：C09.1 :330、C09.7 :336、`session_transport.rs:1-3/:33-40`、28G:273 本地夹具端点）——确认或否决。
  - 推理链 `[新缠论:推论]`：
    1. C09.1 对三模式一律是同宿主独立进程加本地协议，域间传输没有「真实变化」，按 C10.2 不替换。
    2. 唯一随环境变化的传输是 X 对外，而且只在 X 内（C09.7）。
    3. 如果把域间传输也列为可换项，S/E/B 就有第二套接线，构成同一判断的两档（AGENTS.md:48）。
- **Q7 受控持久化 adapter**：默认值是不按模式换实现，三模式都用 C09.2 每域 SQLite/WAL 单写者、`synchronous=FULL`、`.writer.lock` 加持久 `writer_epoch`；可变的只有 manifest 的各域 DB 路径和数据目录；「受控」读作「启动前件不满足即拒写」（C09.2 :331）；simulation 不得改用内存库，也不得关 fsync（出处：C09.2 :331、ADR 0039:274/:292、`session_transport.rs:56`、`store.rs:653`、ADR 0038:60/:81）——确认或否决。
  - 推理链 `[新缠论:推论]`：
    1. ADR 0038 H2（:81）的 SIGKILL 恢复 oracle 要求 simulation 也走持久 checkpoint，用内存库 H2 无法成立。
    2. ADR 0039:292 的「失去写权第一义」靠每域库加锁加 epoch，换存储就换了锁语义。
    3. 所以真实变化的只有路径。
- **Q8 同输入等价验证**：默认值是三层二值 oracle（出处：ADR 0038:47/:80/:84、C10.4 :347、#1382:12、28G:220、C10.2 :345）——确认或否决：
  - (a) 同模式：沿用 ADR 0038 H1，不重裁。
  - (b) 跨模式前段：同一份 S 结构原件集，同一组 policy/model/seed/scheduler，simulation 会话和 experiment 会话的 S 面（第 1 面）逐字段相等。同一 ExactCommand 夹具经 X 唯一调用边，分别走 simulation adapter 和 HL adapter，X 侧调用记录除 adapter 身份与场所回执外逐字段相等。
  - (c) 跨模式后段，回报录放：把 experiment/production 会话的外效 journal（第 6 面）里的场所回报作为受控撮合的回报脚本，在另一个 simulation 会话里重放；结构原件取原会话 S 已持久的输入记录。六面与原会话逐字段相等，差异只允许落在预先声明、封闭列举的一一重命名字段（裁定七 (c) 表列八项，含 E 原件 `format` 标签的改标）。
  - 推理链 `[新缠论:选择]`：
    1. 跨模式不可能六面全等：第 4-6 面由场所决定，C10.2 禁止同会话两个成交权威。
    2. adapter 之前的面（S 面、X 调用边上的命令投影）不依赖场所，可以直接比。S 不依赖执行侧，见 28G:170「执行侧任何故障……都不得停止 S」。
    3. B/E 面依赖回报，要让它们可比，唯一办法是把场所回报当原件在另一个会话里重放。不同会话、单一权威，符合 C10.2。
    4. C10.4 允许预先声明的一一重命名，用来处理运行 ID、Begin token、`System` 时间戳；装配差异（证据类、时钟、adapter 身份、E `format` 标签）也只以一一改名出现。
    5. 录放件保留录制的 `source` 四元而不重新生成事实身份：否则 `fact_key` 与第 4/6 面全部 cause 引用都要改名，闭集无法封闭，也违背 C10.2 共享「身份」。
  - 自由度：回报录放作为 `model_ref.model_id` 的一个具名取值 `recorded-venue-replay/1` 落进 `history-input-profile/1` 的 model 段（ADR 0052:266）。它只在 `evidence_class=simulation` 的对拍会话里合法，运行前单选，一次运行只读一个值，没有备用值（ADR 0041:13/:15）。它的输入（录制的回报）和 ADR 0033 系撮合模型（版本合同字段见 ADR 0033:48）不同，不是同一判断的松紧两档。命令与录制不符时按 ADR 0052:279 同款不变式判硬错误、停进程，不回落到其他撮合。
- **Q10 E 接纳原件的形状**：默认值是 E 只有一份原件 schema，即 ADR 0051:244 的 `source` 形状加 `book.rs:117-124` 的必填项（`event`、`allocation_id`、非 netted 时的 `family_id` 与 `target`，以及 `:124` 的 `fact_key`，定义在 `book.rs:93`）；`format` 按来源具名：受控撮合、场所 adapter、TestOnly 夹具各一个值，E 校验 `format` 与会话 profile 的 `evidence_class` 一致，不一致就 `SchemaUnsupported`，不落行；受控撮合和 HL adapter 都只产这一种形状（出处：INTERFACE-CONTRACTS:35、ADR 0051:244、28G:446、`book.rs:93/:106-107/:117-124`）——确认或否决。
  - 推理链 `[新缠论:推论]`：
    1. C10.2 共享「身份」，所以 E 对三模式只有一个接纳合同。
    2. ADR 0051:244 已给出唯一的 source 形状，28G:446 已令 HL adapter 输出对齐它。
    3. `book.rs:106` 的 format 字符串是来源闸，不是第二套合同。按模式另立一套 schema 就是两档（AGENTS.md:48）。
  - 归属语义仍归 #1539，本票不定。
- **Q11 单调时钟与传输期限**：默认值是 `job.rs:286-289` 的 `Clock` 缝与 `TransportBounds` 归资源闸，不归 adapter，三模式实现不换；数值取 manifest `resources` 段（`store.rs:251` 要求为正），进 profile `content_hash`；期限触发即失败（ADR 0027:32），双跑 diff 非空即 FAIL（ADR 0038:84）；`make_token`（`s_structure_session.rs:126-133`）的 PID:ns 列入 C10.4 预先声明的重命名字段；不新增 ADR 0052:300 的故障 kind（出处同上）——确认或否决。
  - 推理链 `[新缠论:推论]`：期限不进任何事件内容，超期结果只是失败，不分叉语义；暴露资源故障本来就是 H1 该判的 FAIL。
- **Q13 正本落点**：默认值是新开 ADR NNNN，CONTEXT.md「运行边界」节补一条「同核 adapter（Shared-Core Adapter）」词条（出处：ADR 0027:48 留口；ADR 0038/0052 同系先例；`.agents/skills/domain-modeling/SKILL.md:68-70` 三条件）——确认或否决。
  - 推理链 `[新缠论:选择]`：本裁定是跨 ADR 0027/0038/0039/0052 的接缝合同，难以逆转，缺上下文会意外，确有取舍。追加到 ADR 0039 会把 launcher 清单和 adapter 边界混在一起；ADR 0052 只管历史 profile。

**C 类**：无。

## 裁定

用户 <日期> 批复：「<用户原话>」（落地时由 `/graph-approve` 按用户最终口径填写；以下按「四条全批」写成，用户改哪条则按改后口径落）。上述 B 类默认值全部采纳，A 类按出处沿用，分九条落为一份新 ADR，并在 CONTEXT.md 补一条词条。

正本落点：docs/adr/NNNN-three-mode-shared-core-adapter-boundary.md

### 一、共享核心（沿用 A）

三模式共用 #1340 C10.2（:345）五项：核心 reducer、身份（ADR 0051 `domain-event/1`）、责任、执行生命周期、观察协议。两个核心 owner 的实现只有一个（ADR 0027:46）。可比较的全状态就是 ADR 0038:49-54 六面。任何模式特有的结构判定、重状态推进或交易决策程序都违反 ADR 0027:46。

### 二、模式与装配

| 模式 | `evidence_class` | 依据 |
|---|---|---|
| 历史回放 | `simulation`（P0） | ADR 0052:250；ADR 0028:28 |
| paper | `experiment` | ADR 0028:25 的 `P1-hl-testnet` 取 `experiment`；ADR 0039:284 正式 paper 是 X 内 HL adapter，而 :272 只在 experiment/production 装场所 adapter `[新缠论:推论]` |
| live | `production` | CONTEXT.md:374；ADR 0039:272 |

装配只由 `production_session` 按 profile 的 `evidence_class` 和 manifest 的「adapter 选择」段决定（ADR 0039:270/:272），版本内不切换（ADR 0028:28）。各域进程不自选 adapter。

### 三、adapter 替换表（owner 落点）

| adapter | owner 进程 | simulation | experiment / production | 三模式不变的部分 |
|---|---|---|---|---|
| feed·结构源 | S（经 S.AcceptInput） | `history-input-profile/1` 的 `structure.source_ref`（ADR 0052:264；`input_source` 按 ADR 0052:306-308） | 个股线实时流（ADR 0023:11/:30-34） | C02.1 原件形状（:259）；源声明在 init 持久化，运行中不切换（#1769:13） |
| feed·执行行情 | X（adapter 选择段） | 受控撮合消费的行情 | 场所行情（#1524 交付） | 永远不进 S；按来源身份判别（#1769:12；28G:170） |
| clock | manifest `clock` 段，各域按其读 | `logical-bar/1`（ADR 0052:296） | `System`（`store.rs:317-318`） | 身份不含 wall-clock（ADR 0051:172）；`TestOnly` 不进 `production_session` |
| matcher/venue | X 进程内（ADR 0039:284-285；C09.7） | 受控撮合，规则 `cross`（#1718:17-18），附 ADR 0052:279 不变式 | X 内场所 adapter；能力证据按 C10.5（:348） | X 唯一调用边；同一 ExactCommand 投影 |
| 传输 | X 对外一段 | 进程内调用，零网络 | 场所 HTTP/WS，按一次调用合同（C09.7） | 域间 `session_transport` 同一实现（C09.1；`session_transport.rs:1-3`） |
| 受控持久化 | 各域单写者 | 同一 SQLite/WAL 实现 | 同一实现 | 只换 DB 路径；启动前件不满足即拒写（C09.2 :331）；锁与 epoch 见 ADR 0039:292、`store.rs:653` |

场所账户、权限与具体端点归 #1540（FU-18）。成交/持仓唯一真账与 crosswalk 归 #1539（FU-17）。本表不定这两项。

### 四、E 接纳原件只有一份 schema

E.CaptureExternal（INTERFACE-CONTRACTS:35）对三模式只认一份原件 schema：ADR 0051:244 的 `source`，加 `book.rs:117-124` 的必填项（`fact_key` 在 `:124`，定义在 `:93`）。`format` 是来源标签，取三值之一：受控撮合原件、场所 adapter 原件、TestOnly 夹具原件。E 校验 `format` 与会话 `evidence_class` 一致（TestOnly 夹具原件只配 `TestOnly` 时钟的组件会话），不一致返回 `SchemaUnsupported`，不落行，不改走其他校验。受控撮合和场所 adapter 产出同一形状。本条对所有具名场所 adapter 一视同仁（HL、#1382 的 IBKR paper、#1400 的 Binance 只读查询都产「场所 adapter 原件」），场所、账户与权限的绑定仍归 #1540/#1351。

回报录放会话（裁定七 (c)）是 `simulation` 会话，时钟为 `logical-bar/1`，E 只接受「受控撮合原件」：录放 matcher 把录制件重发为受控撮合原件，`format` 改标为「受控撮合原件」，`source` 四元（`source_namespace`、`source_epoch`、`external_fact_id`、`source_cursor`）原样保留，`received_time` 由逻辑时钟重写。「TestOnly 夹具原件只配 `TestOnly` 时钟」与录放会话无交集：录放会话由 `production_session` 拉起，按裁定五第 2 条不可能配 `TestOnly` 时钟，出现 TestOnly 夹具原件即 `SchemaUnsupported`、不落行，并因行数不等使裁定七 (c) 判 FAIL。

### 五、时钟与期限

1. 时钟 kind 是封闭枚举，由 `evidence_class` 一一推出：`simulation→logical-bar/1`，`experiment/production→System`。不设可选值。
2. `production_session` 遇到 `TestOnly` 时钟，或时钟 kind 与 `evidence_class` 不符，在拉起任何域之前以 `InvalidDomain` 退出。
3. `fault_schedule`（ADR 0052:300）只在 `simulation` 会话合法。`store.rs:263` 现在的 `TestOnly` 绑定只保留给直调域 bin 的组件夹具。
4. `job.rs` 的 `Clock` 和 `TransportBounds` 是资源闸，三模式实现不换，数值走 manifest `resources`，进 `content_hash`。期限触发即失败，不重试换判据（ADR 0027:32）。

### 六、权威排他（沿用 A）

一个会话只有一个 `evidence_class`，也只有一个外部事实源（C10.2 :345；ADR 0052:250）。模拟 fill 与真实 fill 不得同会话并存。混源只能出现在另一个会话里（见裁定七 (c)）。simulation 会话的产出不进 TStage 触闸计数（ADR 0038:74），分证不交叉引用（ADR 0038:75）。

### 七、同输入等价的三层 oracle（测试锁）

- **(a) 同模式**：沿用 ADR 0038 H1/H2（:80/:81），本 ADR 不重裁。
- **(b) 跨模式前段**：
  - 同一份 S 结构原件集，同一组 policy/model/seed/scheduler，simulation 会话与 experiment 会话的第 1 面（S 结构快照）逐字段相等。
  - 同一 ExactCommand 夹具经 X 唯一调用边，分别走 simulation adapter 与 HL adapter（本地夹具端点）。X 侧调用记录除 adapter 身份与场所回执外逐字段相等，ADR 0039 A4 写权闭包 diff 为空（28G:220）。
- **(c) 跨模式后段，回报录放**：
  - 输入：取一次 experiment/production 原会话里 S 已持久的结构输入记录，以及 X 外效 journal 里的场所回报、更正、bust、持仓回报。
  - 重放：在新的 `simulation` 会话中以 `model_ref.model_id = recorded-venue-replay/1` 运行。录放 matcher 收到 X 调用边上的 ExactCommand 投影后与录制命令比对，相等则按录制件重发回报（`format` 改标、`source` 四元原样、`received_time` 由逻辑时钟重写，见裁定四末段）；不等即硬错误停进程，不回落到 `cross` 撮合（同 ADR 0052:279 不变式）。
  - 比较面与时点：ADR 0038:49-54 六面全部字段；时点为每个 bar 收盘后 checkpoint（ADR 0038:56）。各面导出按（域, `event_id`）排序，因此同一 bar 内的行序不参与比较（ADR 0038:56 批内顺序 `Unknown`）；提交序号的值另列为下表第 8 项。时间戳与提交序号分属第 3、8 项，各自独立配对，不互相替代（INTERFACE-CONTRACTS:9）。
  - 判定程序：对两次运行在每个 checkpoint 导出六面，把下表「重命名闭集」作为一一映射施加到原会话导出上，再与重放会话导出逐字节比较。
  - **重命名闭集**（共八项；每项都是两种会话装配不同而必然产生的差异，原会话一个值只对应重放会话一个值，反之亦然）：

    | # | 字段 | 出现面 | 原会话取值 | 重放会话取值 | 必然不同的成因 |
    |---|---|---|---|---|---|
    | 1 | 会话 ID、运行 ID | 六面记录头 | 原会话 ID | 重放会话 ID | 两个会话；C10.4（:347）点名的非语义字段 |
    | 2 | `make_token` 的 PID:ns | Begin token | 原进程 PID 与纳秒 | 重放进程 PID 与纳秒 | `s_structure_session.rs:126-133` 取进程号与系统时间 |
    | 3 | 时钟写出的 `received_ns`、`first_known_ns`、`commit_ns`（`service.rs:686-688`）及 E 原件 `source.received_time` | 第 1–6 面中带这些字段的行 | `System` 读数 | `logical-bar/1` 读数 | 裁定五第 1 条按证据类绑定时钟；映射按（域, `event_id`, 字段名）逐条配对 |
    | 4 | 证据类与时钟、模型回显：`evidence_class`、`clock.kind`、`model_ref` | profile/manifest 回显 | `experiment` 或 `production`；`System`；原会话撮合模型 | `simulation`；`logical-bar/1`；`recorded-venue-replay/1` | 裁定二、五：录放只在 `simulation` 合法 |
    | 5 | X adapter 身份（adapter 名称与端点标识） | 第 6 面 journal 条目 | 场所 adapter（如 HL adapter） | 受控撮合（录放 matcher） | 裁定三 matcher/venue 行 |
    | 6 | E 原件 `format` 标签 | 第 4 面原件 | 场所 adapter 原件 | 受控撮合原件 | 裁定四：`simulation` 会话 E 只收受控撮合原件 |
    | 7 | adapter 线格式请求体（实现若在 journal 里记录） | 第 6 面 journal 条目 | 场所 HTTP/WS 请求体 | 无（录放 matcher 不出网） | 线格式在 X 调用边之后、属 adapter 自身输出；它另由 (b) 的本地夹具金样锁（28G:273）逐字节判定 |
| 8 | 持久提交序号 | 各域写入行 | 原会话序号 | 重放会话序号 | 同一 bar 内的到达与提交次序在 experiment/production 由场所与网络决定，ADR 0038:56 已裁批内顺序为 `Unknown`、不作比较维度；映射按（域, `event_id`）配对，且须保持 bar 归属与跨 bar 次序：原会话第 k 个 bar 内的行只能映到重放会话第 k 个 bar 内的行 |

  - 闭集之外的字段一律原值逐字节相等，包括：`source` 四元、`fact_key`、`allocation_id`、`family_id`、`target`、cause 链、订单/成交 epoch 与前后状态、数量/价格/费用、场所回执内容（如 `exec_id`）、物理持仓、各重 B 账与 E 责任、S 结构对象与修订链。
  - **PASS** 当且仅当以下五条同时成立：施加映射后六面逐字节无差异；每项映射都是一一的（无两个原值映到同一重放值，也无反向）；被改名的值只出现在表中所列字段位置；每个 checkpoint 六面行数相等；重放全程未触发命令不符硬错误。任一不成立即 **FAIL**。
  - 这是 #1382:12「各核心语义输出逐字段相等」的二值落法。禁止只比净仓（C10.4 :347）。
- `recorded-venue-replay/1` 只在 simulation 对拍会话中合法，产出是 simulation 证据（ADR 0038:74），不作正式成交或持仓来源。

### 八、旧外壳与旧设计票（沿用 A）

- 现有 Python/NT 外壳只做模式输入、协议与展示适配（C09.1 :330），不得成为第二个 launcher（chong-traceability.md:52；ADR 0039:285）。逐条去向归 #1550 与 TB-10 #1368。
- #1329 只登记本 ADR 的指针，不解除暂停，不关票（评论 5596449009；C12.3 :382）。

### 九、词汇

CONTEXT.md「运行边界」节（:364）补一条「同核 adapter（Shared-Core Adapter）」：只列六类名称（结构 feed、执行行情 feed、clock、matcher/venue、传输、受控持久化）及各自 owner 进程，不写接口。

## 验收清单

（`<ADR>` 代表 docs/adr/NNNN-three-mode-shared-core-adapter-boundary.md，NNNN 落地时按 main 顺序分配。本票只验文书、票状态与边；运行类测试是消费票的验收，见下一段，不作本票关票条件。）

- [ ] 文件存在：`test -f <ADR>` 退出码 0。
- [ ] `rg -c '^正本落点：docs/adr/' <ADR>` 输出 = 1。
- [ ] `for t in logical-bar/1 System TestOnly evidence_class recorded-venue-replay/1 重命名闭集 simulation_and_hl_adapters_share_the_x_call_edge E.CaptureExternal session_transport SchemaUnsupported InvalidDomain; do rg -F -q "$t" <ADR> || echo "MISSING $t"; done` 输出为空。
- [ ] `rg -F -c '同核 adapter（Shared-Core Adapter）' CONTEXT.md` 输出 ≥ 1（当前为 0）。
- [ ] 票状态：`gh issue view 1538 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1538/comments --paginate --jq '[.[]|select(.body|test("^用户 [0-9]{4}-[0-9]{2}-[0-9]{2} 批复"))]|length'` 输出 ≥ 1（P 类 resolution 首行格式，见 `.agents/skills/graph-approve/SKILL.md:16`）。
- [ ] 承接登记：`for n in 1329 1381 1382 1400 1523 1524 1525 1769 1770; do c=$(gh issue view $n -R xy7365527-lang/NewChanlun --comments | grep -c 'FU-16 承接登记'); [ "$c" -ge 1 ] || echo "MISSING #$n"; done` 输出为空（当前九票都是 0）。
- [ ] #1329 不被关：`gh issue view 1329 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `OPEN`。
- [ ] 边不删：`gh api repos/xy7365527-lang/NewChanlun/issues/1538/dependencies/blocking --jq 'map(.number)|sort|map(tostring)|join(",")'` 输出 `1362,1366,1367,1525,1539,1540,1550`。
- [ ] blocker 已解除：`gh api repos/xy7365527-lang/NewChanlun/issues/1538/dependencies/blocked_by --jq '[.[]|select(.state=="open")]|length'` 输出 `0`。

## 受影响票与 blocking 边变化

**受影响票**

下列测试名是建议名，由各票 owner 实施时固定。判据统一为 `cargo test <完整路径> -- --exact` 的输出含 `1 passed`。

- **#1769**（OPEN，`production_session` 与 `x_execution_session` 骨架）：消费裁定二、三、五。时钟枚举补 `logical-bar/1`；时钟 kind 与 `evidence_class` 绑定，不符时在拉起域之前 `InvalidDomain`；`production_session` 拒收 `TestOnly` 时钟。建议测试：`clock_kind_bound_to_evidence_class`、`production_session_rejects_testonly_clock`、`simulation_store_keeps_wal_full_sync`。
- **#1381**（OPEN，TB-07-A）：消费裁定五第 3 条。`fault_schedule` 在 simulation 会话下触发，不依赖 `TestOnly` 时钟。建议测试：`simulation_fault_schedule_without_testonly_clock`。
- **#1770**（OPEN，X 内 HL adapter 离线合同）与 **#1425**（OPEN，TB-04-F 回报应用）：消费裁定四。场所 adapter 和受控撮合产出同一 E 原件 schema；E 校验 `format` 与 `evidence_class` 一致。建议测试：`matcher_and_hl_adapter_emit_same_e_raw_schema`、`e_rejects_raw_format_mismatching_session_evidence_class`。
- **#1525**（OPEN，TB-04 纵片 C）：:26「paper/live 共用同核证明」按裁定七 (b) 落成 `simulation_and_hl_adapters_share_the_x_call_edge`（28G:220）。
- **#1382**（OPEN，TB-08-A）：AC :11/:12 按裁定三、七 (b)(c) 落值。其 IBKR paper 准备登记（[评论 6405903436](https://github.com/xy7365527-lang/NewChanlun/issues/1382#issuecomment-6405903436)「新adapter边界仍受#1538约束」）按裁定二（paper 装配为 `experiment`）、三、四消费，不另立场所专属规则。建议测试：`cross_mode_structure_face_equal`、`recorded_venue_replay_six_faces_equal`、`recorded_venue_replay_command_mismatch_is_hard_error`、`recorded_venue_replay_unlisted_field_diff_fails`（在闭集外字段注入一处差异，须判 FAIL）。
- **#1400**（OPEN，TB-08-B 具名场所只读能力与事实完整性接入会话）：其准备登记（[评论 6405905587](https://github.com/xy7365527-lang/NewChanlun/issues/1400#issuecomment-6405905587)）写明「完整adapter边界受#1538约束」。交付（正文 :7 身份、订单、持仓与回报前沿查询；:12 逐项能力证据与「事实唯一来源」）消费裁定三 matcher/venue 行（查询走 X 内场所 adapter，X 是唯一执行网络请求的进程，C09.7 :336）、裁定四（查询结果以「场所 adapter 原件」进 E）、裁定六（单一外部事实源）；能力证据形状沿用 C10.5（:348）。本票不改其 AC；它已经 #1382、#1366 被本票传递阻塞，不加直接边。建议测试：`venue_readonly_query_enters_e_as_venue_adapter_raw`。
- **#1523**（OPEN，纵片 A：HL 适配器与正式 paper launcher）：批复评论 6452353777 保留的「paper/live 同核」义务，按本 ADR 裁定一至七消费：paper 装配为 `experiment`，X 内 HL adapter 遵裁定三表，同核证明按裁定七 (b)(c)。本票不改其 AC，也不改其边。
- **#1524**（OPEN，纵片 B：HL trade tick/1m feed 与断线恢复）：交付受裁定三表「feed·执行行情」一行约束。HL feed 只进 X 的 adapter 选择段，永远不进 S；按来源身份判别（#1769:12）。:7「与 #1304/#1305 回放 driver 共核」按裁定一、八读作：共用同一核心，不另立 driver 主干。
- **#1411**（OPEN，TB-07-C）：裁定七 (a) 沿用 ADR 0038 H1/H2，不加新义务；裁定五第 4 条的期限数值进 `content_hash`。
- **#1539**（FU-17）、**#1540**（FU-18）、**#1550**（FU-19）：被本票阻塞的裁决票，分别消费裁定六（单一事实源）、裁定三（场所 adapter 位置，账户/权限仍归 #1540）、裁定八（外壳只做 adapter）。
- **#1362**（TB-04）、**#1366**（TB-07）、**#1367**（TB-08）：被本票阻塞的汇总票，关票后边自然满足。
- **#1368**（TB-10）：Python 外壳终态实施沿用 chong-traceability.md:52，不变。
- **#1329**（OPEN，暂停）：登记本 ADR 指针，不解除暂停，不关。
- **#1340**（SPEC）：C10.2/C10.4 由本 ADR 落成接缝合同，SPEC 本身不改。

**受影响代码清单**（下游实施，不在本票改）：

- `rust/src/economic_session/store.rs:253-258`、`:263-264`（时钟枚举与故障绑定，归 #1769/#1381）；
- `rust/src/economic_session/book.rs:106-107`（format 闸改为来源标签校验，归 #1770/#1425）；
- `rust/src/bin/production_session.rs`、`rust/src/bin/x_execution_session.rs`（main 上尚不存在，不编行号，归 #1769，同 chong-traceability.md:50）。

**blocking 边变化**

- 不新增，不删除。依据 chong-traceability.md:58、ADR 0052:364 同款先例。
- 保留 7 条：#1538 blocks #1362、#1366、#1367、#1525、#1539、#1540、#1550。
- 已解除，不改：#1533、#1536 blocks #1538（两票都已 CLOSED）。
- 不加 #1538 → #1382、#1400、#1523、#1524 的直接边。四票都已经被 #1366 阻塞（实测 #1382 blocked_by 含 1366，#1400 为 1382、1362、1363、1366、1365、1351，#1523 含 1366，#1524 为 1366、1769），#1366 又被本票阻塞，传递约束已经成立，加直接边是冗余。
- 无 research/probe 缺口，不开新票。

## 授权引用

本票已转 P 类（graph-engineering.md:73「再 BLOCK 转 P 类」，:78 挂 `needs-approval`），裁定的授权来自用户对本稿的批复，resolution 首行写「用户 <日期> 批复」并引原话（`.agents/skills/graph-approve/SKILL.md:16`）。N-1 预授权（graph-engineering.md:90）只覆盖独审通过的 T 类，本票不援引它裁定；下列原话只作分级背景保留。用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」（[#1323 评论 6261281111](https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111)，`notes-N-1-20261019`；同文见 .sandcastle/docs/graph-engineering.md:90）。

## 引用自证

| 引用 | 核实命令 | 结果摘要 |
|---|---|---|
| ADR 0027:32 | `grep -n '' docs/adr/0027-*.md \| sed -n '32p;46p;48p'` | 「失败必须作为失败处理，不能切到另一套判据、旧入口或宽松路径重新判定。」 |
| ADR 0027:46 | 同上 | 「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。」 |
| ADR 0027:48 | 同上 | 「环境差异只允许落在后续裁定的行情、撮合和场所 adapter。adapter 的责任范围与具体接口不在本 ADR 内。」 |
| ADR 0038:47 | `grep -n '' docs/adr/0038-*.md \| sed -n '1,3p;45,62p;72,86p'` | 「固定五元组……全状态必须逐位等价。全状态六面：」 |
| ADR 0038:49-54 | 同上 | 六面：S 结构快照、各重 B 账、E 责任、订单/成交原件、物理持仓、外效历史 |
| ADR 0038:56 | 同上 | 「checkpoint 取每 bar 收盘后、下一 bar 开盘前」 |
| ADR 0038:60 | 同上 | 「launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar」 |
| ADR 0038:74 | 同上 | 「simulation 产出不进 TStage 触闸计数」 |
| ADR 0038:75 | 同上 | 「分证不交叉引用……各自独立累计」 |
| ADR 0038:80 / :81 / :84 | 同上 | H1 确定性对拍；H2 SIGKILL 恢复；「任一 H1–H4 的 diff 非空 ⟹ FAIL」 |
| ADR 0038:1-3 | `git show origin/main:docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md \| grep -n '' \| sed -n '1,3p'` | 「> 正本：本文件是双跑确定性、恢复语义、seek 不偷未来、AsKnown 分证协议的教义正本。」「> 最后裁定：#1537（2026-10-19）」「> 受影响代码清单：见「受影响票与 blocking 边变化」节」 |
| ADR 0052:3-5 | `git show origin/main:docs/adr/0052-history-input-profile-and-model-order-unspecified.md \| grep -n '' \| sed -n '3,5p'` | 「> 正本：本文件是完整历史输入 profile……的正本。」「> 最后裁定：#1536（FU-14，2026-10-28，#1323 Notes N-1 预授权 T 类判定）」「> 受影响代码清单：见本文「受影响票与 blocking 边变化」节」 |
| ADR 0033:48 | `git show origin/main:docs/adr/0033-history-simulation-bar-batch-model.md \| grep -n '' \| sed -n '46,48p'` | :46「## 版本合同字段（v1）」；:48「`model_id/revision/content_hash`；`input_domain`；`clock`（bar 索引、`known_frontier_cut`）；`price_rules`……`seed`……调度轨迹。」 |
| ADR 0039:241 / :243 | `grep -n '' docs/adr/0039-*.md \| sed -n '1,4p;239,245p;264,296p'` | `newchan` CLI 与 `ws_live` 都是「收编：只读查询壳 adapter」 |
| ADR 0039:266 | 同上 | 「bin `production_session`，源文件 `rust/src/bin/production_session.rs`」 |
| ADR 0039:270 | 同上 | 「一份 manifest：……原始输入来源、各域 DB 路径、时钟、adapter 选择（C10.1）」 |
| ADR 0039:272 | 同上 | 「`evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合，`experiment` 和 `production` 装场所 adapter。」 |
| ADR 0039:274 | 同上 | 「以独立 OS 进程启动 S、E、X、各 Bᵢ（C09.1）；每个域有自己的 `--db`、`.writer.lock`、`writer_epoch`。」 |
| ADR 0039:284-285 | 同上 | 「`production_session` 与 `x_execution_session` 骨架，加 X 内的 HL adapter」；「只能是被 X 进程内部调用的 adapter 模块」 |
| ADR 0039:288 | 同上 | 「直接调用 `s_structure_session` 等域 bin 只算组件证据。」 |
| ADR 0039:292 | 同上 | 「S/E/B/X 每个域一个库、一个 `.writer.lock`……持久 `writer_epoch` 不等时被拒（`store.rs:466-467`）」 |
| ADR 0039 A4 | `grep -n 'A4' docs/adr/0039-*.md` | :257「必须恰好等于上表点名的 10 个文件（验收 A4）」 |
| ADR 0052:250 | `grep -n '' docs/adr/0052-*.md \| sed -n '245,252p'` | 「Q14：历史 profile 是 P0 / `evidence_class=simulation`，唯一外部事实源由 adapter 装配锁定」 |
| ADR 0052:264-269 | `… \| sed -n '252,270p'` | 六域表：结构 C02.1 原件……时钟 `{kind: "logical-bar/1", …}` |
| ADR 0052:266 | 同上 | 订单/成交 `model` 加 `model_ref{model_id, revision, content_hash}` |
| ADR 0052:279 | `… \| sed -n '270,310p'` | 「X 的模拟 matcher 只做一个不变式断言……不算第二个判定点。」 |
| ADR 0052:285 | 同上 | 「验证入口只有 `production_session`……以 `evidence_class=simulation` 装配受控撮合 adapter」 |
| ADR 0052:296 | 同上 | 「`clock.kind = "logical-bar/1"`。在 `simulation` 下，S/B/E/X 不读系统时钟……本条是已裁推论的组合，不设可选值。」 |
| ADR 0052:300 | 同上 | 「`scenario_contract.fault_schedule[]` 每项为……故障只来自声明，不来自随机」 |
| ADR 0052:306-308 | 同上 | 「`input_source` 是可调字段……默认 `massive-stock-tick/1`……可选值 `databento-equs-mini-tick/1`」 |
| ADR 0052:353 / :364 | `… \| sed -n '345,370p'` | 「#1538（FU-16）：消费裁定一和裁定三 (d)。」；「不删边，关票后自然满足」 |
| ADR 0051:172 | `grep -n '' docs/adr/0051-*.md \| sed -n '170,173p;242,247p'` | 「任何一层都不含 wall-clock 或 PID」 |
| ADR 0051:244 | 同上 | `source`：`{source_namespace, source_epoch, external_fact_id, source_cursor, received_time}` |
| ADR 0028:25 | `grep -n 'evidence_class\|experiment' docs/adr/0028-*.md` | 「\| evidence_class \| experiment \|」 |
| ADR 0028:28 | `grep -n '' docs/adr/0028-*.md \| sed -n '26,30p'` | 「任一字段变更即新版本；`environment` 与 `execution_form` 不得在版本内切换。」 |
| ADR 0020:153-154 | `grep -n '' docs/adr/0020-*.md \| sed -n '150,155p'` | 结构/决策层消费美股 tick；执行层股票永续，价差/滑点是执行成本项，不进结构 |
| ADR 0023:11 | `grep -n '' docs/adr/0023-*.md \| sed -n '1,20p'` | 个股线源 = Massive Stocks，粒度 tick |
| ADR 0023:30-34 | `grep -n '' docs/adr/0023-*.md \| sed -n '28,35p'` | 「实时流接入（四条）」：Massive schema、精度、实时落盘、断线回补 |
| ADR 0041:13 / :15 | `grep -n '' docs/adr/0041-*.md \| sed -n '13,22p'` | 判别句；「可调字段不得有『主值失败时的备用值』」 |
| #1340 :259（C02.1） | `gh issue view 1340 --json body --jq .body > /tmp/b1340.md; grep -n '' /tmp/b1340.md \| sed -n '259p;…'` | 「标准输入包含 source namespace/epoch……执行市场价格不得偷偷改变结构级别语义。」 |
| #1340 :330（C09.1） | 同上；`sed -n '326,329p'` 确认 :326 为「### C09」 | 「进程间走带版本和身份的本地消息协议，消息到达不等于持久提交。」 |
| #1340 :331（C09.2） | 同上 | 「每个域一份独立 SQLite 数据库和 WAL……不满足则拒绝相应写入。」 |
| #1340 :336（C09.7） | 同上 | 「X 是唯一持有场所写凭据并执行网络请求的进程……纳入一次调用合同」 |
| #1340 :344-348（C10.1-C10.5） | 同上；`sed -n '340,342p'` 确认 :342 为「### C10 完整历史、paper/live 与 Adapter」 | :344 唯一完整会话入口；:345 C10.2 原文；:346「TestOnly 顺序……不能证明完整经营验收」；:347 C10.4 原文；:348「每个 paper/live adapter 提供能力证据」 |
| #1340 :363 / :382（C12.1 / C12.3） | 同上；`sed -n '360,362p'` 确认 :362 为「### C12」 | 「以最高完整会话入口为主测试缝」；「旧 #1327–#1333 仍是暂停的历史设计 DAG」 |
| #1382 :11-12 | `gh issue view 1382 --json body --jq .body \| grep -n '' \| sed -n '9,13p'` | AC 原文：「仅替换实际变化的feed/clock/matcher/venue/transport……」「相同受控事件经过两个模式的适配层，各核心语义输出逐字段相等……」 |
| #1769 :12-13 | `gh issue view 1769 --json body --jq .body \| grep -n '' \| sed -n '11,14p'` | 「场所执行行情……只能出现在 X 的 adapter 选择段……判据是来源身份」；S 结构源声明 init 持久化 |
| #1718 :17-18 | `gh issue view 1718 --json body --jq .body \| grep -n '' \| sed -n '16,19p'` | `fill_rule={cross(k≥1),touch}`，默认 cross；touch 只在诊断域合法 |
| #1524 :2 / :7 | `gh issue view 1524 --json body --jq .body \| grep -n '' \| sed -n '1,12p'` | 「实现 Hyperliquid trade tick → 1m……唯一 feed 接口」；「与 #1304/#1305 回放 driver 共核」 |
| #1525 :26 | `gh issue view 1525 --json body --jq .body \| grep -n '' \| sed -n '24,27p'` | 「paper/live 共用同核证明（无旁路执行路径）」 |
| #1400 正文 :7 / :12 | `gh issue view 1400 --json body --jq .body \| grep -n '' \| sed -n '7p;11,14p'` | :7「正式会话使用获准的具名场所/账户profile查询身份、订单、持仓与回报前沿……」；:12「真实查询/回报证据逐项覆盖身份代际、累计完整性、幂等/查询范围、费用/更正、时间语义和事实唯一来源；缺项为具体Unsupported……」 |
| 评论 6405905587 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/6405905587` | 作者 xy7365527-lang，2026-10-26T01:02:25Z，#1400；E/Binance 只读未激活准备登记，「完整adapter边界受#1538约束；整票依赖不变」 |
| 评论 6405903436 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/6405903436` | 作者 xy7365527-lang，#1382；D/IBKR paper 未激活准备登记，「新adapter边界仍受#1538约束，整票blocker保持」 |
| #1400 边 | `gh api …/issues/1400/dependencies/blocked_by --jq 'map(.number)'`；`…/blocking` | blocked_by `[1382,1362,1363,1366,1365,1351]`；blocking `[1412]` |
| 受影响票两路复搜 | `gh issue list --search 1538 --state all --limit 100`；`git grep -n -E '#?1538\|FU-16' origin/main -- analysis docs .chanlun CONTEXT.md AGENTS.md rust trading_system scripts .sandcastle` | tracker 命中 1323/1382/1400/1523/1525/1533/1534/1536/1538/1539/1540/1550/1599/1769/1770；新增需列的只有 #1400（OPEN）。#1534、#1599 已 CLOSED，前者只记 #1539 依赖 FU-16（已在 #1539 项），后者只复述 #1382/#1400 等 #1538 边界。仓内命中只有 chong-traceability.md:50/:52/:64、ADR 0051:346/:434、ADR 0052:55/:337/:353/:364 与 review-results 旧件，均已引用或与本票无新义务；其余为数值巧合 |
| 28G :170 | `grep -n '' /Volumes/AgentStorage/decisions/1523/draft-28G.md \| sed -n '168,172p'` | 「Q7 结构源与执行源分离……HL 永续行情与回报只属执行层」；「执行侧任何故障……都不得停止 S」 |
| 28G :220 | `… \| sed -n '218,222p'` | `simulation_and_hl_adapters_share_the_x_call_edge`……「除 adapter 身份与场所回执外逐字段相等」，由 #1525 在 #1538 裁定后实现 |
| 28G :273 | `… \| sed -n '271,275p'` | 本地夹具端点收到的请求体与金样逐字节相等 |
| 28G :446 | `… \| sed -n '444,448p'` | 「输出对齐 main 上 E 原件的 `source` 字段，不发明 #1539 的归属语义」 |
| 评论 6452353777 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/6452353777` | 作者 xy7365527-lang，2026-10-27T20:07:10Z，#1523；「用户 2026-10-28 批复：『批准推荐稿』」；「本票保留全部原义务：正式 paper 成功、paper/live 同核……」 |
| 评论 5596449009 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/5596449009` | 作者 xy7365527-lang，#1329；「旧 C1–C6 设计排程继续暂停」；义务接单处 #1366/#1367/#1365/#1369 |
| 评论 6404224455 | `gh api repos/xy7365527-lang/NewChanlun/issues/1538/comments` | 作者 xy7365527-lang，2026-10-25；#1533/PR #1704 返修承接登记 |
| 评论 6465991201 | 同上 | 作者 xy7365527-lang，2026-10-28；「分级：T — ……」 |
| 评论 6261281111 | `gh api repos/xy7365527-lang/NewChanlun/issues/1323/comments --paginate --jq '.[]\|select(.body\|contains("notes-N-1"))'` | 作者 xy7365527-lang，2026-10-19；「用户 2026-10-18 原话：『其实很多技术的东西 完全不需要我来做 我完全信任你的判断』」 |
| graph-engineering.md:73 / :78 | `git show origin/main:.sandcastle/docs/graph-engineering.md \| grep -n '' \| sed -n '73p;78p'` | :73「BLOCK 时起草者升档为 Opus 5.5 带审查意见重做一次；再 BLOCK 转 P 类。」；:78「P 类：票上发草稿与审查结论，挂 `needs-approval`，不关票……用户批量批复后按 T 类落地。」 |
| graph-approve SKILL.md:16 | `git show origin/main:.agents/skills/graph-approve/SKILL.md \| grep -n '' \| sed -n '16p'` | 「票上发 resolution 评论，首行『用户 <日期> 批复』并引用户原话；摘 `needs-approval`。」 |
| graph-engineering.md:90 | `grep -n '其实很多技术' .sandcastle/docs/graph-engineering.md` | :90 N-1 原话同上 |
| chong-traceability.md:50 / :52 / :58 / :64 | `grep -n '' .chanlun/definitions/chong-traceability.md \| sed -n '1,3p;50p;52p;58p;64p'` | :50 不编 `production_session.rs` 行号；:52「三模式同核与外壳接入归 #1538、#1550」；:58「不删除边」；:64 本票行原文 |
| INTERFACE-CONTRACTS.md:9 / :21 / :35 | `grep -n '' …/INTERFACE-CONTRACTS.md \| sed -n '9p;21p;35p'` | :9「原始 received 时间、真实 first_known、持久提交序号……分别保留」；:21「S.AcceptInput……S 唯一输入接纳器」；:35「E.CaptureExternal……同事务写 Recorded 和保守 Pending」 |
| CONTEXT.md:364 / :374 | `git show origin/main:CONTEXT.md \| grep -n '运行边界\|证据类'`；`… \| sed -n '374p'` | :364「### 运行边界」；:373 词条名「证据类（Evidence Class）」，:374 三值与「低类证据不得被读为高类有效」（main `0b3097aae8`） |
| AGENTS.md:48 / :70 | `grep -n '同一个判断不得存在宽严两档' AGENTS.md`；`grep -n '两边输出必须逐位相同' AGENTS.md` | :48 #799 通则原句；:70 #804 判别式 |
| domain-modeling SKILL.md:68-70 | `grep -n 'Hard to reverse\|three are true' .agents/skills/domain-modeling/SKILL.md` | :68「Only offer to create an ADR when all three are true:」；:70「Hard to reverse」 |
| store.rs:251 | `grep -n '' rust/src/economic_session/store.rs \| sed -n '250,265p'` | `positive(required(&manifest["resources"], k)?, k)?;` |
| store.rs:253-258 | 同上 | `"System" => ()`、`"TestOnly" => {…}`、`_ => return Err("InvalidDomain：必须显式指定时钟adapter"…)` |
| store.rs:263-264 | 同上 | `if manifest["clock"]["kind"] != "TestOnly" { return Err("InvalidDomain：故障暂停仅TestOnly"…` |
| store.rs:317-318 | `… \| sed -n '315,320p'` | `if self.manifest["clock"]["kind"] == "System" { let n = std::time::SystemTime::now()` |
| store.rs:466-467（漂移） | `… \| sed -n '464,468p'` | 实为 `CString::new(…)` 与 `statfs`，不是 epoch 比对 |
| store.rs:653 | `grep -n 'writer_epoch' rust/src/economic_session/store.rs` | `if positive(&meta(c, "writer_epoch")?, "writer_epoch")? != expected {` |
| book.rs:93 | `grep -n '' rust/src/economic_session/book.rs \| sed -n '93,96p;104,125p'` | `pub(crate) fn fact_key(event: &Value)`，:96 `required(s, "source_namespace")?` |
| book.rs:106-107 | 同上 | `doc["format"] != "neutral TestOnly economic raw originals; not production transport wire"` → `SchemaUnsupported：仅已声明 TestOnly 经济原件 profile` |
| book.rs:117-124 | 同上 | :117 `event`、:118 `allocation_id`、:121 `family_id`、:122 `target(&event)?`、:124 `fact_key(&event)?` |
| service.rs:686-688 | `grep -n '' rust/src/economic_session/service.rs \| sed -n '684,690p'` | `config.clock(Some(msg), "received_ns"…)`、`"first_known_ns"`、`"commit_ns"` |
| session_transport.rs:1-3 | `grep -n '' rust/src/session_transport.rs \| sed -n '1,4p;28,40p;50,64p'` | 「#1374：每个 S/E/B 独立实例复用的有界 Unix 输送与进程排他锁。」 |
| session_transport.rs:33-40 | 同上 | `pub struct TransportBounds { frame, read: Duration, write, response, queue, connections }` |
| session_transport.rs:56 | 同上 | `PathBuf::from(format!("{}.writer.lock", …))` |
| job.rs:286-289 | `grep -n '' rust/src/s_content/job.rs \| sed -n '285,290p'` | 「单调时钟缝：生产写 `Box::new(Instant::now)`，测试注入假时钟」；`pub trait Clock` |
| s_structure_session.rs:126-133 | `grep -n '' rust/src/bin/s_structure_session.rs \| sed -n '125,134p'` | `fn make_token()` 用 `SystemTime::now()`，`format!("{}:{nanos}", std::process::id())` |
| production_session.rs 不存在 | `ls rust/src/bin/ \| grep -E 'production_session\|x_execution\|…'` | 只列出 b_chong_session、e_economic_session、s_structure_session |
| #1538 边 | `gh api …/issues/1538/dependencies/blocking --jq 'map(.number)\|sort\|map(tostring)\|join(",")'`；`…/blocked_by` | `1362,1366,1367,1525,1539,1540,1550`；blocked_by `1533,1536`，两票都已 closed |
| #1382 / #1523 / #1524 / #1525 / #1769 / #1770 边 | `gh api …/issues/<n>/dependencies/blocked_by` 与 `/blocking` 逐票 | #1382 by 1362,1363,1366,1365,1353；#1523 by 1362,1363,1366,1365；#1524 by 1366,1769；#1525 by 1530,1538,1539,1770,1422,1425；#1769 blocks 1770,1383,1524；#1770 blocks 1525 |
| 票号与状态（#1329 #1340 #1362 #1366 #1367 #1368 #1381 #1382 #1411 #1425 #1523 #1524 #1525 #1533 #1536 #1539 #1540 #1550 #1718 #1769 #1770） | `gh issue view <n> --json state,title` 逐票 | 除 #1533、#1536 为 CLOSED 外都是 OPEN；标题与正文所述一致 |
| 登记标记现值 | `gh issue view <n> --comments \| grep -c 'FU-16 承接登记'`（1329 1381 1382 1400 1523 1524 1525 1769 1770） | 都是 0 |
| CONTEXT 词条现值 | `grep -c '同核 adapter' CONTEXT.md` | 0 |
| ADR 编号现状 | `git ls-tree --name-only origin/main docs/adr/ \| grep -E '005[0-9]'` | main `0b3097aae8` 有 0051、0052、0054、0055（无 0050、0053），新 ADR 写 NNNN，落地时按 main 分配 |
| 引文文件自 `58039c60` 起未变 | `git diff --stat 58039c60 origin/main -- rust/src/economic_session rust/src/session_transport.rs rust/src/s_content/job.rs rust/src/bin docs/adr CONTEXT.md AGENTS.md .chanlun/definitions/chong-traceability.md .sandcastle/docs/graph-engineering.md .agents/skills/domain-modeling/SKILL.md` | 只有 graph-engineering.md（:104，不影响 :90）、CONTEXT.md（+2 行，致 :364/:374 漂移，已改）、新增 ADR 0054/0055 |
| ADR 0054 / 0055 与本票无关 | `git show origin/main:docs/adr/0054-*.md`、`0055-*.md` 各 `grep -n '1538\|FU-16\|同核'` | 均 0 命中 |

## 考古对照

| 点 | 归类 | 出处 | 草稿处置 |
|---|---|---|---|
| Q1 | A | ADR 0027:46；#1340 C10.2 :345；ADR 0038:49-54 | 沿用，写入权威依据 1/3/14 与裁定一 |
| Q2 | A | ADR 0039:270/:272；ADR 0028:28；ADR 0052:250 | 沿用，裁定二；补 paper=experiment 推论（ADR 0028:25） |
| Q3 | A | ADR 0020:153-154；ADR 0023:11/:30-34；28G:170；#1769:12 | 沿用，裁定三表前两行 |
| Q4 | B（含冲突） | ADR 0052:296；store.rs:253-258/:263-264/:317-318；ADR 0051:172 | 冲突写入问题段；默认值采纳，裁定五；实施交 #1769/#1381 |
| Q5 | A | ADR 0039:284-285；C09.7 :336；ADR 0052:279/:285；#1718:17-18 | 沿用，裁定三表 matcher/venue 行 |
| Q6 | B | C09.1 :330；C09.7 :336；session_transport.rs:1-3/:33-40；28G:273 | 默认值采纳，裁定三表传输行 |
| Q7 | B | C09.2 :331；ADR 0039:274/:292；ADR 0038:60/:81；store.rs:653 | 默认值采纳，裁定三表持久化行；记 ADR 0039:292 行号漂移 |
| Q8 | B | ADR 0038:47/:80/:84；C10.4 :347；#1382:12；28G:220 | 默认值采纳，裁定七三层 oracle；录放落为 ADR 0041 可调字段取值 `recorded-venue-replay/1`；(c) 判据按第二轮审查改写为八项重命名闭集加五条 PASS 条件，`format` 改标列为第 6 项 |
| Q9 | A | C10.2 :345；ADR 0052:250；ADR 0038:74/:75 | 沿用，裁定六 |
| Q10 | B（含冲突） | INTERFACE-CONTRACTS:35；ADR 0051:244；28G:446；book.rs:93/:106-107/:117-124 | 冲突写入问题段；默认值采纳，裁定四；行号按审查订正为 :117-124，`fact_key` 在 :124、定义在 :93 |
| Q11 | B | job.rs:286-289；session_transport.rs:33-40；store.rs:251；s_structure_session.rs:126-133；ADR 0052:300 | 默认值采纳，裁定五第 4 条与裁定七 (c) 重命名集合 |
| Q12 | A | chong-traceability.md:52；ADR 0039:241/:243/:285；C09.1 :330 | 沿用，裁定八；去向归 #1550/#1368 |
| Q13 | B | ADR 0027:48；ADR 0038:1-3 与 ADR 0052:3-5 同系先例；domain-modeling SKILL.md:68-70；CONTEXT.md:364 | 默认值采纳，正本落点 ADR NNNN 加 CONTEXT 词条（裁定九） |
| Q14 | A | #1329 评论 5596449009；C12.3 :382 | 沿用，裁定八；#1329 只登记指针，验收里确认它仍为 OPEN |
