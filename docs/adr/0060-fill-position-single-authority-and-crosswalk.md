# ADR 0060：成交/持仓唯一写者分工与 crosswalk

> 正本：本文件是成交/持仓唯一写者分工与 crosswalk 的正本。
> 最后裁定：#1539（2026-11-02，用户 P 类批准，[批复评论](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6564138279)）
> 受影响代码清单：见「受影响票与 blocking 边变化」节

**状态**：已采纳（P 类，用户批准）。用户批准的版本是 [影子评论 6551181286](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6551181286) 的问表及同版草稿、考古（Q4、Q12 两项 C 类推荐答案；Q9、Q10、Q11、Q13、Q15 五项 B 类默认值；ADR 0038、ADR 0039 文末 append-only 修订授权）。本 ADR 只定写者、证据载体、恢复读法与 crosswalk 结构；实现与运行证据归消费票，不以本 ADR 声称实现完成。
**裁定依据**：两轮独审 BLOCK 后按 `.sandcastle/docs/graph-engineering.md:73` 转 P；40R 返修经 40Y 独审 BLOCK（B1–B4），41F 返修经 41K 非作者独审 PASS（R1–R8），41P 只做 41K 列出的三项同义文书订正。原件存档于 `.chanlun/review-results/issue1539-decision-20261102/`（源 `/Volumes/AgentStorage/decisions/1539/`、`fu17-review40y/`、`fu17-review41k/`、`fu17-publish41p/`）。
**行号基线**：引文行号以冻结 main `9de18e606b2c0330162a83e2a030898b46c3690f` 为准；落地基线 main `784bc6d770` 与之相比，本文引用的 ADR、`CONTEXT.md` 与 `rust/src/economic_session/`、`rust/src/session_transport.rs`、`rust/src/bin/x_execution_session.rs` 均无改动，行号照搬。INTERFACE-CONTRACTS 指 SPEC #1340 包内 `INTERFACE-CONTRACTS.md`。

## 问题

#1539（FU-17）要裁定三件事：每个 fill/position 由谁做唯一权威写者；重复、乱序、费用、更正、恢复下累计量怎样保持稳定；crosswalk 怎样做到可审计。决策地图 `/Volumes/AgentStorage/decision-map-1323-倒推-20261018.md:69` 记为「唯一 owner、来源、跨模式映射待定」。分级评论 [6543394411](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6543394411) 定为 T 类；两轮独审 BLOCK 后按 `.sandcastle/docs/graph-engineering.md:73` 转 P（[影子评论 6544082577](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6544082577)）。获批稿是该影子稿的第二次返修（41F，经 41P 同义订正）：在 40R 返修稿上修正 40Y 独审指出的四项合同冲突（H2 射程、减少前件、原件事实类别、crosswalk 物化），考古随之修订（`/Volumes/AgentStorage/fu17-repair41f/archaeology.md`，冻结 main `9de18e606b2c0330162a83e2a030898b46c3690f`）。

考古找出的冲突，起草时逐条处理：

1. **物理持仓没有写者记载（Q4，C）。** ADR 0038:53 把「物理持仓（场所回报与内部投影对账）」列为六面之一，正本层没写谁写它、它算不算独立真账。旧臂 `rust/src/theta_v0/nautilus/account_adapter.rs:8` 把 `portfolio.net_position(instrument_id)` 直接写成 `voice_qty[0]`，即用场所净仓覆写私账，违反 #1340 C06.7（:300「账实快照不能直接覆写私账」）与 C09.9（:338「不从场所净仓重建成本」），不作先例。
2. **「crosswalk」一词多义（Q12，C）。** `CONTEXT.md` 0 命中。仓内至少四义：① SPEC 包 `IMPLEMENTATION-CROSSWALK.json`（#1406:43 与 :50 的链接）；② ADR 0037:26 的 crosswalk oracle 与 :29 的 crosswalk 锁（S 布局对拍）；③ #1340 C09.11（:340「责任 crosswalk」）、C11.5（:358「身份 crosswalk」）、ADR 0052:267（「crosswalk 记录 id」）、ADR 0028:71（「带仓身份映射」）的旧→新迁移映射；④ 决策地图 :69 的「跨模式映射」。ADR 0012:106 已把「拿词的外形……当同一性判据」判为元判据错误，须先定词再裁结构。
3. **恢复语义：两份已裁正本与已批 SPEC 方向相反（Q10，B）。** ADR 0038:60 写「launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar；杀停等同未完成 bar 的全部外效未发生，S/B/E 回退到上一 checkpoint」，未按 `evidence_class` 区分。ADR 0039 把这条写进正式 launcher（含 production）的启动序列：:27 引其前句，:147「启动和换代时核……checkpoint（ADR 0038:60）」，:289「启动时检测 checkpoint 并恢复（ADR 0038:60）」。在 experiment/production 下，后句与以下已裁/已批条文相反：#1340 C07.6（:310「提交后崩溃……保留最大责任，不自动重发」）、C09.8（:337「锁或 epoch 本身不取消已发网络请求，其责任继续保留」）、C09.9（:338「不能从旧 Open 恢复」）、AT-13（:425「回退保留所有已经发生的外效」）、ADR 0030:48（「恢复后先只读对账在途订单（外部未知按最大保留，不伪造终态）」）。同向的还有 ADR 0039:26 引 ADR 0030「不回退旧 writer」、C09.11（:340「回退保留新外效和尾责」）。代码侧 `store.rs:1692-1725` 的域 `recover` 审计现库、提升 epoch，:1710-1711 拒绝早于最后提交的恢复时刻；它不删已提交的 facts/applications/commits，schema v2 下只清未发布的 `content_staging` 暂存（`content_store.rs:1206-1215`）。只在新 ADR 里单方面收窄 ADR 0038:60 会让两份正本各说各的，所以本稿把修订落在 ADR 0038 与 ADR 0039 本身（见裁定附件）。
4. **代码落后于正本。** `rust/src/economic_session/book.rs:106-107` 只接受 TestOnly format，而 ADR 0056:46 定的是三值 format。`rust/src/bin/x_execution_session.rs:22` 的 `MODES` 含 `dry-run`，而 ADR 0056 裁定二（:23-27）的 `evidence_class` 只有三值，ADR 0056 全文 `dry-run` 0 命中，ADR 0056:120 对 #1769 只登记「时钟 kind 与 `evidence_class` 绑定」。两处只登记，不作先例。
5. **E 原件必填项与「X 不判归属」的张力（Q13，B）。** `book.rs:117-124` 要求原件带 `allocation_id`，非投影形另须 `family_id` 与 `target`；#1770:34 写明 X 观测记录「不含 `allocation_id`、`family_id`、`book_identity`，不判定归属」。须裁定受控撮合原件与场所 adapter 原件里这几个槽由谁填、填什么，同时保证归属只有 E 一个写者。场所原件不止成交：ADR 0056:125 的账户查询结果（身份、订单、持仓、回报前沿）与 ADR 0032:33 的 command/period 费用都没有单笔发令 manifest，字段义务须按事实类别分开写。
6. **累计完整性有在案先例（Q9，B）。** ADR 0033:31「bar 收盘即其终态累计 F 完整」（ADR 0052:35 转引）给出 simulation 下累计 F 何时完整。减少责任的前件已由已批合同按动作给出：#1340 C06.2（:295）列出「不可再执行/累计完整性的证据」与参与账回执，C07.8（:312）对非原子 amend 转移明写「等待旧单不可再执行且历史累计完整」。本票不改这些前件，只裁证据由谁写、以什么事实为载体。
7. **引用行号漂移（只登记）。** ADR 0030 的恢复条在 #1599 批复插入观察期段后由 :44 移到 :48（`40f11b540f`）；ADR 0039:26/:288/:326/:412/:427、ADR 0045:23/:63、ADR 0048:114/:378 仍写 :44。ADR 0039:300 的代码指针 `session_transport.rs:53/:62`、`store.rs:466-467` 现址为 :56/:65、:657-659；ADR 0056:40 的 `ADR 0039:292`、`store.rs:653` 现址为 ADR 0039:290/:300、store.rs:657。ADR 0039 的五处随本票修订节订正；其余登记给下次修订该文件的票。

## 裁定


正本落点：docs/adr/0060-fill-position-single-authority-and-crosswalk.md；docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md（追加修订节，附件 A）；docs/adr/0039-production-entry-inventory-and-unique-launcher.md（追加修订节，附件 B）；CONTEXT.md「运行边界」节（第九条）。ADR 0030 不改。

新 ADR 依次写以下九条：

1. **写者分工表 `[新缠论:推论]`**：
   | 对象 | 唯一写者 | 载体 |
   |---|---|---|
   | 外部事实原件：成交报（逐笔/累计）、fill/command/period 费用、更正、bust、账户查询结果 | E（`E.CaptureExternal`） | E 域 `facts`，append-only；只有成交报推进 F |
   | 逐重分配 | E（Resolve） | `fill_allocations` 行 |
   | 逐重 position（毛账、成本、阶段） | 该重 Bᵢ（`B.ApplyAllocation`） | Bᵢ 域 `applications` |
   | 调用与回报观测 | X | X 域 journal |

   第二个写者在物理上被三样东西挡住：每域独立进程、`.writer.lock`、持久 `writer_epoch`（ADR 0039:282/:300；`session_transport.rs:56/:65`；`store.rs:657-659`）。book image 和毛账合计都是投影，不是第二本账（ADR 0051:311）。

2. **物理持仓是派生对账读数**（采纳 Q4 推荐答案），没有写者，只经 `Q.ReadEconomicView` 输出。账实差异只产生 Pending 或封闸，永不覆写 B，不从场所净仓重建成本。

3. **F 与累计完整性**（采纳 Q9 默认值）：F 由 E 按 `fact_key` 去重后经 ADR 0032:21 同一函数归一，只有成交报推进 F。减少责任的前件照 #1340 C06.2、C07.8 与 INTERFACE-CONTRACTS:38，本 ADR 不改：C07.8 非原子转移须「旧单不可再执行」与「历史累计完整」同时成立；一般减少须覆盖该责任的证据与实际参与账持久回执。证据载体：simulation 依 ADR 0033:31；experiment/production 依已 Recorded、且种类由该 adapter 能力证据（C10.5）声明的场所 adapter 原件，所需各项分别证明。缺任一所需证据保原潜量；能力声明不能取消前件。

4. **重复、乱序、费用、更正**：沿用 ADR 0051 裁定二、ADR 0032 裁定五/六、#1340 C06.7，不另立规则。X 观测记录套用 `domain-event/1` 外壳，带 `command_id`，判定只有 no-op 和 `IdentityConflict` 两种结果（ADR 0051:320）。

5. **恢复**（采纳 Q10 默认值）：恢复语义的正本是 ADR 0038「修订（#1539）」节（附件 A），本 ADR 不另写恢复规则；H1/H2 射程照 ADR 0056:63 对三种模式不变。这里只记它对累计量的后果——恢复后 F 只由重放的 E facts 决定，不因恢复倒退、不补发；experiment/production 下 checkpoint 之后已 Recorded 的事实与已持久的 MayHaveCalled 计入 F 与最大潜量（C07.8）。

6. **X→E 原件字段义务**（采纳 Q13 默认值）：
   - 共享外壳：来源身份 `source`；原件级 `allocation_id` 等于 `external_fact_id`；`cause[]` 含 X 观测记录 `event_id`；不带 `family_id`、`target`；逐重归属只在 E 产生（Q11）。
   - 命令相关成交报只取投影形，带所发命令的 `manifest_hash` 与 `command_id`；其 fill 类费用与更正按原 fill 绑定具名前件。
   - command/period 费用按 ADR 0032:33 既裁因果路径由 E 归属，原件只带来源身份、费用类别与因果锚，不伪造成交量或 manifest 投影。
   - 账户查询结果（ADR 0056:125）不推进 F、不产生分配、不写 B，只作对账列与证据载体。
   ADR 0056:46 的单 schema 不改；`book.rs:106-124` 的 TestOnly 解码器不改，场所 adapter 原件解码由 #1425、#1400 按上述类别实施，不得靠让 X 填归属字段满足必填项。

7. **crosswalk**（采纳 Q12 推荐答案）：五类身份记录之间的四跳关系；每跳由所在域的唯一写者持久化；不设第二权威写入口，物化汇总只能是可重算、不得反写的只读投影（是否交付属实施范围）；审计以 `Q.ReadEconomicView` 双向逐跳拼接为准，缺跳报缺项。跨模式时逐字节相等（ADR 0056:85）。

8. **边界**：迁移 crosswalk 归 #1535；场所账户、权限、端点归 #1540（ADR 0056:42）；X 侧 `OwnAttempt/MayHaveCalled` 的实体规则归 #1330（ADR 0051:320）；恢复时钟取值归 #1769 FU-16（本票不定 `recover_ns`）；`book.rs:106-107` 只收 TestOnly、`x_execution_session.rs:22` 含 `dry-run` 两处登记为实装滞后，由 #1425/#1769 消费，不作先例。

9. **词条**：`CONTEXT.md`「运行边界」节补两条。
   - 「唯一真账（Single Authority Ledger）」：fill 原件与分配归 E，逐重 position 归 Bᵢ，物理持仓为派生对账读数。`_Avoid_`：把场所净仓或 book image 叫真账。
   - 「crosswalk」：取第七条的定义。`_Avoid_`：与 `IMPLEMENTATION-CROSSWALK.json`、ADR 0037 crosswalk oracle、迁移 crosswalk 混称。


## 权威依据

**写者机制（A，Q1）**
- ADR 0039:282：「以独立 OS 进程启动 S、E、X、各 Bᵢ（C09.1）；每个域有自己的 `--db`、`.writer.lock`、`writer_epoch`。」ADR 0039:300：「S/E/B/X 每个域一个库、一个 `.writer.lock`……影子臂不持有任何域库的 epoch 和锁，物理上无法写入任何域库」。ADR 0039:290：换代先确认旧 X 已退出，再提升 `writer_epoch`。
- 代码现址：`session_transport.rs:56` 生成 `.writer.lock`，`:65` 报「StaleWriter：{domain_name}独占进程锁不可得」；`store.rs:657-659` 的 `check_epoch` 报「StaleWriter：持久epoch不等于获准写者」。
- ADR 0056:40：「受控持久化｜各域单写者｜同一 SQLite/WAL 实现」。#1340 C09.2（:331）：每域一份独立 SQLite 和 WAL，由该域单写者进行有限事务。C09.7（:336）：X 是唯一持有场所写凭据并执行网络请求的进程。
- ADR 0027:44-48 裁定三：历史、paper、live 共用两个核心 owner，环境差异只落在 adapter。

**fill 原件与 position 的写者（A，Q2/Q3）**
- INTERFACE-CONTRACTS:35 `E.CaptureExternal`：「同事务写 Recorded 和保守 Pending」。INTERFACE-CONTRACTS:37 `B.ApplyAllocation`：「本地唯一应用并返回持久收据……重复同内容无变化；异内容冲突」。
- ADR 0051:310：真相只有 `facts / applications / commits` 根链。ADR 0051:311：book image、毛账合计是可重算投影，「审计时必须等于重放结果」。
- `store.rs:75`：`applications(... fact_key TEXT NOT NULL UNIQUE ...)`。`book.rs:93-99`：`fact_key = hash(source_namespace, source_epoch, external_fact_id)`。
- `.chanlun/definitions/chong-traceability.md:35`：「逐重毛账按腿记录，不以净额代替」。ADR 0024:17-20 裁定二：验收用净额账，归因用毛账，两套并记。
- #1340 C09.9（:338）：「B 恢复重放原归属应用，不从场所净仓重建成本」。

**重复、乱序、费用、更正（A，Q5-Q8）**
- ADR 0051:266-269 裁定二：去重键是身份不是内容；同 ID 同内容 no-op，异内容持久写 `IdentityConflict`；乱序先 Recorded/Pending，按 `cause[]` 前件因果应用；跨重和批内先后为 `Unknown`。
- #1340 C06.7（:300）：「累计与逐笔报使用同一规范化合同……不能按内容去重误吞真实成交。更正按前件因果应用」。C06.4（:297）：原报与 Pending 同事务。
- ADR 0032:21：「逐笔回报与累计回报由同一函数归一」。ADR 0032:31-33 裁定五：`cause ∈ {fill, command, period}`，fee-before-fill 时 E 侧 AwaitingDependency。ADR 0032:37 裁定六：总量更正按 `A_i(F) − A_i(F−b)` 计，「原报与旧 Applied 不删」。
- #1340 C05.4（:287）：「手续费、funding、borrow 的来源、计量坐标、归属和未分配项分别可追踪」。C07.10（:314）：未来更正超出原覆盖时如实记录，「不裁掉原报凑不变量」。
- 代码：`tb03c_e.rs:3-4`（AwaitingDependency；目标重不在 manifest 时封 `account:*`）；`tb03c_e.rs:33-37`（`check_fill_predecessor`，投影形经 :36-37 分派到 `tb03c_corrections::predecessor`）；`tb03c_corrections.rs:122-129`（投影形费用/更正按原件 `source` 三元、`allocation_id`、`manifest_hash` 绑定具名前件）；`tb03c_corrections.rs:1-3`（迟到费用、具名冲销、`A(F) − A(F−b)`）。

**归属与分配（Q11、Q13）**
- ADR 0032:12：命令投影时冻结 `manifest = {projection_kind ∈ {per_leg, netted}, participants:[(chong_id, command_id, family_id, revision, requested_q_i)], …}`。ADR 0032:14-15：`per_leg` 缺归属时记 Unmatched；`netted` 只走冻结比例。
- `tb03c_allocation.rs:9-11`：投影形原件以 `netted_fill`/`per_leg_report` 识别。`:136-151` `check_capture`：原件类型或标的与冻结投影不符报 `IdentityConflict`。`:340-349`：逐重 `allocation_id` 由 E 以 `hash(fact_key, 原件 allocation_id, manifest_hash, participant)` 派生，同时写入 `chong_id/command_id/family_id/units`；`tb03c_per_leg.rs:102-110` 为 per_leg 同款。`:365`：`fill_allocations` 行带 `fact_key/manifest_hash/policy_id/from/to/evidence_id`。`:387-405`：B 应用投影形事实时从 E 的 `fill_allocations` 取本重分配，不读原件归属。
- `book.rs:117-124`：`allocation_id` 必填；只有非投影形原件才必填 `family_id` 与 `target`。投影形核验只覆盖成交：`tb03c_per_leg.rs:28-31` 要 `manifest_hash`、`reported_quantity`、`unit_price`；`tb03c_allocation.rs:127` 按 `manifest_hash` 取冻结 manifest。
- 非成交类原件：ADR 0056:125 把账户身份、订单、持仓、回报前沿查询结果定为「场所 adapter 原件」进 E（#1400:7）；ADR 0032:33 费用 `cause ∈ {fill, command, period}`，command 类全额归发出命令的重，period 类按 ADR 0024 裁定六（:43）坐标，`du = 0` 合法；`tb03c_book.rs:129` 已按三类收费用分量。
- #1770:34：X 观测记录带与 E `source` 同名的出处字段，「不含 `allocation_id`、`family_id`、`book_identity`，不判定归属」。#1770:22：场所单号「只能来自 C1 自己的回报」。
- ADR 0051:320：「X 侧 `OwnAttempt / MayHaveCalled` 和 fill→Voice 归属的实体规则归 #1330、#1539」。

**累计完整性（Q9）**
- #1340 C06.2（:295）：「减少须具备该责任不可再执行/累计完整性的证据及其参与账的持久应用回执。回执集合仅含实际关联账」。INTERFACE-CONTRACTS:38 `E.ReduceResponsibility`：前件为「不可再执行/累计完整证据与所有实际参与账收据」，「缺任何前件保原潜量」。#1425:13：「缺前件、累计完整或不可再执行证据保原潜量」。
- ADR 0033:31：「bar 收盘即其终态累计 F 完整」（ADR 0052:35 转引）。#1340 C07.8（:312）：已确定累计执行 F 加最大未计潜量不超过 L；「原子 amend 仅在真实累计上限/版本互斥合同成立时共用一份额度；否则等待旧单不可再执行且历史累计完整，再以族/版本 CAS 唯一转移剩余量」。C10.5（:348）：每个 paper/live adapter 提供能力证据，含「累计完整性」与「唯一外部事实来源」，缺项准确报 Unsupported。
- 代码 `tb03c_allocation.rs:201-205`：`netted_fill` 的 `report_kind` 取 `incremental`（报量＝`to−from`）或 `cumulative`（报量＝`to`），两种都落进同一累计区间核验，不符报 `IdentityConflict`。

**恢复（Q10）**
- ADR 0038:60（原文见「问题」3）；ADR 0038:56：checkpoint 取每 bar 收盘后；ADR 0038:81/:89：H2 以「六面快照与未杀停的连续运行相同」为判据；ADR 0038:101 列 #1381/1399/1411/1418 实装 H1–H4。
- ADR 0056:63 裁定七 (a)：「同模式：沿用 ADR 0038 H1/H2（:80/:81），本 ADR 不重裁」，对三种模式都适用；:66 的跨模式夹具用「HL adapter（本地夹具端点）」；:128：#1411「裁定七 (a) 沿用 ADR 0038 H1/H2，不加新义务」。#1340 C10.4（:347）：双跑固定原始输入及修订、政策/模型、时钟、种子、数值语义、故障与调度轨迹；AT-14（:426）；:436：正式 launcher、两个独立新进程、真正杀进程/重启、各面硬判。#1411:11/:13：固定轨迹下各面硬判，同时「恢复不重复调用、不复活旧身份、不丢最大责任」。
- ADR 0051:274-278 裁定三：「恢复就是从最后一个 checkpoint 加载快照，再重放日志（ADR 0038:60）」，要求 (a) 重放已应用 `fact_key`/`command_id` 返回原 application、`applications.fact_key UNIQUE` 不新增行，(b) root 逐位相等；否则报 `StorageUnavailable` 停写。
- #1340 C07.6（:310）、C09.8（:337）、C09.9（:338）、C09.11（:340）、AT-13（:425），原文见「问题」3。
- ADR 0030:48 裁定五：「授权在首次外部动作前持久化；恢复后先只读对账在途订单（外部未知按最大保留，不伪造终态），再于同一授权 id 下续做……无法连接场所是失败态（保留最大责任、重试、不切判据、不回退旧 writer）」。ADR 0039:26 以「ADR 0030:44」引其后半句。
- ADR 0039:27/:147/:289（正式 launcher 引 ADR 0038:60）；ADR 0042:40（S 写者「从已持久化状态重建运行态，或按 ADR 0038:60 从 checkpoint 加载六面快照后逐 bar 增量重放」）；ADR 0052:296（simulation 下 checkpoint 只认 `bar_index`）。
- 代码 `store.rs:1692-1725` `recover`：`audit` 现库、取最新提交、:1710-1711 若 `recover_ns < last_commit_ns` 报「InvalidDomain：恢复时刻早于已提交事实」，随后提升 `writer_epoch`；不删已提交行，schema v2 下只清未发布的 `content_staging`（`content_store.rs:1206-1215`）。
- 跨正本修订先例：ADR 0039:2 三行头第二行加「修订：#1523」，:485-487 文末 append-only「修订」节（「裁定一至七原文不改，本节为现行修订」）。

**跨模式与权威排他（A，Q14）**
- ADR 0056:59：「一个会话只有一个 `evidence_class`，也只有一个外部事实源……模拟 fill 与真实 fill 不得同会话并存」。#1340 C10.2（:345）同义。
- ADR 0056:85：重命名闭集之外的字段一律逐字节相等，含 `source` 四元、`fact_key`、`allocation_id`、`family_id`、`target`、cause 链、`exec_id`、物理持仓、各重 B 账。ADR 0056:88：`recorded-venue-replay/1`「不作正式成交或持仓来源」。#1340 C10.4（:347）：「禁止只比较最终净仓」。
- ADR 0056:46：E 对三模式只认一份原件 schema，`format` 取三值并与 `evidence_class` 校验。ADR 0056:52：时钟 kind 由 `evidence_class` 一一推出。ADR 0056:42：「成交/持仓唯一真账与 crosswalk 归 #1539」。ADR 0039:280：`evidence_class` 只决定 adapter 装配。
- #1538 影子稿评论 [6466830839](https://github.com/xy7365527-lang/NewChanlun/issues/1538#issuecomment-6466830839) 原文：「hands venue accounts, permissions and endpoints to #1540 and the single ledger / crosswalk to #1539」。用户批复 [6472079617](https://github.com/xy7365527-lang/NewChanlun/issues/1538#issuecomment-6472079617)：「1538 四条全批」，其中第③条「E 只认一份原件 schema，format 为来源标签并与 evidence_class 校验」。

**物理持仓回读与读口（Q4）**
- ADR 0056:125：#1400 的账户查询走 X 内场所 adapter，「查询结果以『场所 adapter 原件』进 E」。评论 [6452358366](https://github.com/xy7365527-lang/NewChanlun/issues/1400#issuecomment-6452358366)：「账户级只读查询，即在挂订单、持仓与回报前沿的回读……由本票在 #1770（C-API）之上承接」。
- INTERFACE-CONTRACTS:40 `Q.ReadEconomicView`：「返回每个域真实 cut、consumed_structure、完整性、缺项和权威状态」。ADR 0030:48「只读对账在途订单」是对账只读的同向先例。

**词与落点（Q12/Q15）**
- ADR 0012:106：同名异物是「拿词的外形……当同一性判据」。ADR 0012:108：把词的对应写进 `CONTEXT.md`（「正本层，允许改」）。
- `.agents/skills/domain-modeling/SKILL.md:70-72`：开 ADR 的三条件（难以逆转、缺上下文会意外、确有取舍）。`CONTEXT.md:364` 是「### 运行边界」。
- ADR 0041:11、:13：可调字段与 #799 的判别句；ADR 0041:17-20：落法。AGENTS.md:46（#799 收敛通则）、:64（#804 总缝规则）。

## 候选与推理链

**A 类（Q1、Q2、Q3、Q5、Q6、Q7、Q8、Q14）** 直接沿用「权威依据」所列出处，不列为待决。

**B 类：默认值，确认或否决**

- **Q9：F 的来源与累计完整性证据。** 默认值（出处 #1340 C06.2 :295、C06.7 :300、C07.8 :312、C10.5 :348、ADR 0032:21、ADR 0033:31、INTERFACE-CONTRACTS:38、#1425:13、`tb03c_allocation.rs:201-205`）——确认或否决：
  - F 的唯一写者是 E。场所的逐笔成交报与累计成交报都作为原件进 E，各成一条 Recorded 事实，经 ADR 0032:21 同一函数归一；只有 F 前进时才应用 `A_i(F_new) − A_i(F_old)`。账户查询结果另作 Recorded 原件，不进这个函数、不推进 F（Q13 类别三）。
  - 减少责任的前件照已批合同，本票不改：一般减少按 C06.2 与 INTERFACE-CONTRACTS:38，要覆盖该责任的证据及实际参与账的持久回执；C07.8 非原子 amend 转移要「旧单不可再执行」与「历史累计完整」两项同时成立，再以族/版本 CAS 转移剩余量。C06.2 的「/」是证据种类的并列，不授权以一种证据顶替另一种。反例：旧单已撤、迟到成交尚未收齐时，撤单终态只证明不可再执行，不证明历史累计完整，转移必须继续等。
  - 本票裁的是证据载体，按载体事实分两类，不是同一判断的松紧两档：simulation 以受控撮合在 bar 收盘产出的终态为据（ADR 0033:31，同时给出两项事实）；experiment/production 以 E 已 Recorded 的场所 adapter 原件为据，所需各项分别证明，原件种类由该 adapter 的 C10.5 能力证据声明。能力声明说明「怎样证明」，不能选择取消哪一项前件。
  - 缺任一所需证据、证据未 Recorded 或能力缺项（Unsupported）时保原潜量（INTERFACE-CONTRACTS:38；#1425:13）。不新增「Σ逐笔＝场所累计」这类合同外判据。
  - 回报通道（逐笔/累计/两者）是场所载体事实，写进 adapter 能力证据记录（C10.5），不列为可调字段（ADR 0041:13：它不是事前偏好）。
  - 推理链 `[新缠论:推论]`：① ADR 0051:310 真相只有根链，场所累计报若绕开 E 就成第二权威，违 ADR 0056:59；② C06.7 要求累计报与逐笔报同一规范化 ⟹ 累计报也只是一条 E 事实，代码 `tb03c_allocation.rs:201-205` 已按此落；③ 前件由 C06.2/C07.8/INTERFACE-CONTRACTS:38 按动作给定，ADR 0033:31 给出 simulation 的完整性时点 ⟹ 本票能裁的只剩证据的写者与载体，experiment/production 的载体只能取自 adapter 声明的能力（C10.5），不另造判据，也不删前件。

- **Q10：恢复语义。** 默认值（出处 ADR 0038:60、ADR 0051:274-278、#1340 C07.6 :310、C09.8 :337、C09.9 :338、C09.11 :340、AT-13 :425、ADR 0030:48、ADR 0039:26/:27/:147/:289、`store.rs:1692-1725`）——确认或否决：
  - **修订落点。** 恢复语义的正本是 ADR 0038（三行头第一行即「恢复语义……的教义正本」），所以修订写进 ADR 0038 本身：文末追加 append-only「修订（#1539）」节，三行头第二行加修订指针，裁定一至四原文不动。ADR 0039 同样文末追加「修订（#1539）」节，指明 :27/:147/:289 按 ADR 0038 修订读，并订正 :26/:288/:326/:412/:427 的 ADR 0030 行号。ADR 0030 不改：它的裁定五正是本修订援引的同向先例，正文与本修订无冲突。新 ADR 裁定五只指向 ADR 0038 修订，不另写第二份恢复规则。修订原文见「裁定」附件 A、B。
  - **通用（三种证据类）。** 恢复＝从最后一个 checkpoint 加载六面快照，再按持久日志重放（ADR 0038:60 前句；ADR 0051:274）；幂等判据为 ADR 0051:275-276 的 (a)(b)；任何证据类下恢复都不删除 append-only 真相行（ADR 0051:310）。
  - **simulation。** ADR 0038:60 后句照旧成立：未完成 bar 的外效视为未发生，S/B/E 工作态回到 checkpoint 快照，由受控撮合按同一五元组重跑该 bar；重跑产出与已持久行同 `fact_key` 同内容，按 (a) 返回原 application，不一致即报 `StorageUnavailable`。
  - **experiment/production。** 不适用 ADR 0038:60 后句。重放持久日志直至最后提交，checkpoint 之后已 Recorded 的 E 事实与 Pending、已 Applied 的 Bᵢ 应用、X 已持久的 OwnAttempt/MayHaveCalled 与观测记录全部保留（C09.9）；在途订单先只读对账，外部未知按最大责任保留、不伪造终态（ADR 0030:48）；MayHaveCalled 不自动重发（C07.6）；X 重启/换代先确认旧发送者退出（C09.8）。
  - **验收。** H1/H2（ADR 0038:80/:81、:89）的射程不变：按 ADR 0056:63 裁定七 (a)，三种模式的同模式会话都在固定完整输入（输入及修订、政策/模型、时钟、种子、数值语义、故障与调度轨迹，C10.4 :347；experiment/production 的场所侧用受控场所夹具，如 ADR 0056:66 的本地夹具端点）下验 H2：杀停恢复后六面与未杀停的连续运行相同。各证据类用各自的恢复方法满足同一 H2。真实外效不回退、不重发（AT-13 :425、C07.6）是附加的恢复约束，不替代六面 H2。恢复幂等另按 ADR 0051 裁定三 (a)(b)。本修订不增删验收项，ADR 0056 无需同步修订。
  - 推理链 `[新缠论:推论]`：ADR 0038:60 后句的前提「外效未发生」只在受控撮合下成立；真实外效已发生时回退 E/B 会丢 Recorded 回报并重铸责任，同时违 ADR 0051:275、C09.9、AT-13 与 ADR 0030:48。ADR 0039 把该条写进 production launcher，单在新 ADR 里收窄会让三份正本对同一条恢复语义各说各的，故修订落在 ADR 0038/0039。两条分支由会话开始时固定的 `evidence_class` 选定（ADR 0056:59 一个会话一个 `evidence_class`），源于载体事实不同，不接在任何失败后改走另一支的 if-else 上（AGENTS.md:46 #799；ADR 0041:13）。分的是恢复方法，不是验收射程：ADR 0056:63/:128 已把同模式 H1/H2 交给三种模式与 #1411，#1411:11/:13 同时要求「与相同轨迹的未中断运行一致」和「恢复不重复调用、不丢最大责任」，可见固定轨迹下不重发与六面等价是并存约束。

- **Q11：分配记录的写者。** 默认值（出处 ADR 0032:12-15、`tb03c_allocation.rs:340-349/:365/:387-405`、#1770:34、ADR 0051:320）——确认或否决：逐重分配（`allocation_id`、`chong_id`、`command_id`、`family_id`、`units`、费用分量）的唯一写者是 E，在 Resolve 时按冻结 manifest 派生；B 是唯一应用者；X 不参与归属。推理链 `[新缠论:推论]`：X 不判归属（#1770:34），B 只应用（INTERFACE-CONTRACTS:37），能写归属的只剩 E；现有代码 `tb03c_allocation.rs:340-349` 由 E 派生，`:387-405` B 侧只读 E 的分配行。

- **Q13：X→E 原件映射。** 默认值（出处 ADR 0056:46/:125、ADR 0032:33、#1770:22/:34、#1400:7、`book.rs:93-99/:117-124`、`tb03c_allocation.rs:9-11/:127/:136-151/:340-347`、`tb03c_per_leg.rs:28-31/:102-110`、`tb03c_corrections.rs:122-129`、`tb03c_book.rs:129`）——确认或否决：
  - 映射在 X 进程内由场所 adapter 生成，产物是 ADR 0056:46 的「场所 adapter 原件」；受控撮合原件形状相同。
  - **共享的是来源、身份与因果外壳。** 每条原件进 E 成一条 Recorded 事实，带 `source` 来源身份，`cause[]` 含 X 观测记录的 `event_id`。原件级 `allocation_id` 槽取值固定为该原件的 `external_fact_id`：它在同一 `source_namespace`/`source_epoch` 内唯一，不含任何重、Voice、family 信息。任何类别的场所原件都不带 X 写的归属字段（`family_id`、`target`），逐重归属只在 E 产生（Q11）。
  - **payload 按既有事实类别分：**
    1. 命令相关成交报（逐笔/累计）只取投影形：`per_leg_report` 或 `netted_fill`，带所发命令的 `manifest_hash`，`command_id` 由 X 依本单回执关联（#1770:22），E 用 `check_capture` 与冻结 manifest 核对。原件级 `allocation_id` 在这条路径上有两种用途：作 E 派生逐重 id 的种子（`tb03c_allocation.rs:344`、`tb03c_per_leg.rs:106`），以及让 fill 类费用与更正/bust 按原 fill 的来源三元、`allocation_id`、`manifest_hash` 绑定具名前件（`tb03c_corrections.rs:122-129`）；B 侧应用不读原件归属（`tb03c_allocation.rs:387-405`）。
    2. command 类与 period 类费用保留 ADR 0032:33 既裁因果路径：command 类全额归发出命令的重，period 类按 ADR 0024 裁定六坐标，由 E 归属。原件只带来源身份、费用类别与因果锚（command 类为 `command_id`），不伪造成交量或 manifest 投影；`du = 0` 合法、只改 K。
    3. 账户查询结果（身份、订单、持仓、回报前沿；ADR 0056:125，#1400:7）不推进成交 F、不产生分配、不写 B。持仓作 Q4 对账列；订单状态与回报前沿可作 Q9 所需证据的载体，前提是该 adapter 的 C10.5 能力证据声明它能证明相应事实。
  - 非投影成交形（带 `family_id`、`target`）只留给 TestOnly 夹具原件。
  - 与 #1770:34 不冲突：该条约束 X 观测记录；原件级 `allocation_id`＝`external_fact_id` 不是归属。上上稿让 X 从 manifest 复制 `allocation_id/family_id/target`，审查 attempt1 R2-1 判为两个写者，已撤回。
  - 推理链 `[新缠论:推论]`：投影形核验要所发命令的 `manifest_hash`、成交量与单价（`tb03c_per_leg.rs:28-31`；`tb03c_allocation.rs:127`），只覆盖命令相关成交；ADR 0056:125 的账户查询结果与 ADR 0032:33 的 command/period 费用不都以单笔成交 manifest 为前件，不能一律套成交量/单价投影，硬套就得伪造所发命令、成交量或归属；command 费用按 `command_id` 归属，period 费用按既裁坐标（ADR 0024:43 裁定六）。故只共享外壳，payload 按类别各走既裁路径，归属写者仍只有 E。

- **Q15：正本落点。** 默认值（出处 SKILL.md:70-72、ADR 0056:42、ADR 0051:320、ADR 0012:108、ADR 0039:2/:485-487）——确认或否决：新开 ADR，编号落地时取当时 main 的下一空号（0058 已由 #1550 的 PR #1821 使用，本稿不预占），文件名 `docs/adr/0060-fill-position-single-authority-and-crosswalk.md`；同批在 ADR 0038、ADR 0039 追加修订节（Q10）；`CONTEXT.md`「运行边界」节（:364）补「唯一真账」「crosswalk」两个词条。推理链 `[新缠论:选择]`：三条件都满足——持久行形状难以逆转；「物理持仓为何没有写者」「F 为何不取场所累计」「production 为何不回退」缺上下文会让人意外；Q4、Q9、Q10、Q12 确有取舍。追加进 ADR 0051 会把事件外壳和账目写者混写，追加进 ADR 0056 会把 adapter 边界和真账混写。

**C 类 decision：真正待决，各附推荐答案**

- **Q4：物理持仓由谁写，算不算独立真账？**
  - 推荐：**不算独立真账，也没有自己的写者。** 物理持仓是派生对账读数，由两列组成：「E 已 Recorded 的场所持仓/成交原件」（经 X 内 adapter 回读，ADR 0056:125；账户级回读由 #1400 承接）和「Σ各重 B applications 投影」，经 `Q.ReadEconomicView` 只读口给出（INTERFACE-CONTRACTS:40）。两列有差异时只产生 Pending 或封闸（C06.4 :297），永不覆写 B 私账（C06.7 :300），也不从场所净仓重建成本（C09.9 :338）。账户查询原件不推进成交 F、不产生分配（Q13 类别三）。
  - 推导：ADR 0051:310 只认根链为真相，ADR 0038:53 把第 5 面定性为「对账」，ADR 0056:59 规定单一外部事实源 ⟹ 再设一本真账就有两个权威。
  - 排除「物理持仓由 X 或 E 另建持久账」（违 ADR 0056:59）与 `account_adapter.rs:8` 的写法（违 C06.7/C09.9）。两条都违反已裁推论，不作可选值。

- **Q12：本票的 crosswalk 指什么，与迁移 crosswalk 怎样划界？**
  - 推荐：crosswalk ＝「同一外效在五类身份记录之间持久、可逆的四跳逐跳映射」：
    - ① 场所身份（客户端单号、`oid`、成交标识）↔ X 观测记录（`event_id`、`command_id`），写在 X 域库（#1770:22/:34）；
    - ② X 观测记录 ↔ E 原件（同名 `source` 字段，`cause[]` 含 X `event_id`，`fact_key` 按 `book.rs:93-99` 算），写在 E 的 facts；
    - ③ E 原件 ↔ 逐重分配（`fill_allocations` 行，`tb03c_allocation.rs:349/:365`），写在 E；
    - ④ 逐重分配 ↔ Bᵢ application（`applications.fact_key UNIQUE`，`store.rs:75`），写在 Bᵢ。
  - 每一跳只由该跳所在域的唯一写者写。禁止第二权威写入口：任何汇总表或索引都不得独立写入映射，也不得反写任一跳。日后若物化汇总，它只能是可由四跳逐跳记录重算的只读投影，审计时必须等于逐跳拼接结果（ADR 0051:311 对 book image 的同款规定）。本票不交付物化层，这是实施范围，不是禁令。审计时由 `Q.ReadEconomicView` 双向逐跳拼接，任何一跳缺失报缺项，不补猜。跨模式时四跳字段按 ADR 0056:85 逐字节相等。
  - 旧→新迁移的身份/责任/尾账 crosswalk（C09.11 :340、C11.5 :358、ADR 0052:267、ADR 0028:71）归 #1535，消费本定义。
  - `CONTEXT.md` 词条用 `_Avoid_` 标出与 `IMPLEMENTATION-CROSSWALK.json`（故事↔验收映射）、ADR 0037:26 crosswalk oracle（S 布局对拍锁）混称。
  - 推导：ADR 0012:106/:108 要求先定词并写进 CONTEXT.md；逐跳持久化由 ADR 0039:300 的每域单写者直接推出；ADR 0051:310-311 区分权威根链与可物化的可重算投影，故禁的是第二权威，不是只读投影。

**自由度（ADR 0041）：** 本票没有一项裁为可调字段。写者分工、恢复判据、crosswalk 结构都是已裁推论或载体事实的直接推论；回报通道属于场所能力事实（C10.5）。crosswalk 是否物化只读汇总是实施范围，本票不交付也不禁止，不构成可调字段。没有资金、量档、cap、θ 类签署门字段。

## 验收清单

- [ ] 新 ADR 存在：`ls docs/adr/0060-fill-position-single-authority-and-crosswalk.md | wc -l` 输出 `1`。
- [ ] 新 ADR 三行头：`grep -c '^> 正本：本文件是成交/持仓唯一写者分工与 crosswalk 的正本' docs/adr/0060-fill-position-single-authority-and-crosswalk.md` 输出 `1`。
- [ ] `grep -c 'ADR 0033:31' docs/adr/0060-fill-position-single-authority-and-crosswalk.md` 输出非 0。
- [ ] `grep -c '修订（#1539）' docs/adr/0060-fill-position-single-authority-and-crosswalk.md` 输出非 0（裁定五指向 ADR 0038 修订节）。
- [ ] ADR 0038 修订节：`grep -c '^## 修订（#1539' docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md` 输出 `1`；`sed -n 2p docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md | grep -c '修订：#1539'` 输出 `1`。
- [ ] ADR 0038 原文不动：`git diff -U0 origin/main...HEAD -- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md | grep -c '^-[^-]'` 输出 `1`（只换第 2 行）。
- [ ] ADR 0038 修订节引已批先例：`sed -n '/^## 修订（#1539/,$p' docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md | grep -c -E 'ADR 0030:48|AT-13|C07\.6'` 输出不小于 `3`。
- [ ] ADR 0038 修订节保留三模式 H2：`sed -n '/^## 修订（#1539/,$p' docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md | grep -c 'ADR 0056:63'` 输出不小于 `1`。
- [ ] 新 ADR 保留 C07.8 两项前件：`grep -c '历史累计完整' docs/adr/0060-fill-position-single-authority-and-crosswalk.md` 输出非 0。
- [ ] 新 ADR 写明账户查询类别：`grep -c 'ADR 0056:125' docs/adr/0060-fill-position-single-authority-and-crosswalk.md` 输出非 0。
- [ ] ADR 0039 修订节：`grep -c '^## 修订（#1539' docs/adr/0039-production-entry-inventory-and-unique-launcher.md` 输出 `1`；`sed -n 2p docs/adr/0039-production-entry-inventory-and-unique-launcher.md | grep -c '#1539'` 输出 `1`；`git diff -U0 origin/main...HEAD -- docs/adr/0039-production-entry-inventory-and-unique-launcher.md | grep -c '^-[^-]'` 输出 `1`。
- [ ] ADR 0030 不改：`git diff --quiet origin/main...HEAD -- docs/adr/0030-exit-authorization-and-liability-retirement.md` 退出码为 0。
- [ ] `grep -c '唯一真账' CONTEXT.md` 输出非 0；`grep -c -i 'crosswalk' CONTEXT.md` 输出非 0。
- [ ] `grep -c 'fact_key TEXT NOT NULL UNIQUE' rust/src/economic_session/store.rs` 输出 `1`。
- [ ] 在 nc-remote 或 CI 上 `cd rust && cargo test --lib <测试名>` 退出码为 0，测试名逐个为：`stale_writer_epoch_rejected_and_fact_key_unchanged_across_epoch_bump`、`domain_event_same_fact_key_same_content_noop_returns_original_application`、`domain_event_same_fact_key_different_content_persists_identity_conflict`、`recovery_replay_root_equals_original_commit_root_or_storage_unavailable`、`c4b_fee_before_fill_waits_for_named_record_resolution_and_each_b_application`、`c4b_late_fee_bust_and_explicit_commission_reverse_match_hand_reopen_and_replay`、`c4b_total_bust_across_three_fills_reverses_per_unit_from_range_stack_and_recovers`。
- [ ] 边存在：`gh api repos/xy7365527-lang/NewChanlun/issues/1539/dependencies/blocking --jq '.[].number' | grep -x 1425` 退出码为 0。
- [ ] 票状态：#1539 为 CLOSED，且 resolution 评论含新 ADR 与 ADR 0038、ADR 0039 三个 `docs/adr/` 路径。

## 受影响票与 blocking 边变化

- **新增边：#1539 blocks #1425**（TB-04-F）。#1425 交付「非空实际回报→Recorded/Pending→唯一明确原责任→B Applied全链」（:11）和「累计与逐笔规范化同源」（:12），消费裁定三、六、七的字段义务。查环：#1539 只被已关闭的 #1534、#1538 阻塞；#1425 的现有前置是 #1375、#1422、#1406。不成环。
- **保留现有边：** #1539 blocks #1362、#1365、#1366、#1367、#1525、#1535、#1540、#1551；#1534、#1538 blocks #1539（均已关闭）。
- **#1366**（TB-07 汇总，已被本票阻塞）：「断点恢复」按 ADR 0038 修订节验收——三种模式的同模式会话都走固定完整输入下的 H2；experiment/production 另须满足 AT-13 与 C07.6（不回退、不重发）；恢复幂等按 ADR 0051 裁定三。不新增边。
- **ADR 0056:63/:128 与 #1411**（OPEN，TB-07-C）：H1/H2 射程不变，ADR 0056 无需修订，#1411 仍「不加新义务」。#1411:11/:13 已同时要求固定轨迹下与未中断运行一致、恢复不重复调用、不丢最大责任，与 ADR 0038 修订第 3、4 条一致；修订只删掉 ADR 0038:60 字面在真实模式下与这两条的矛盾。不新增边：#1411 的实现照其已批 AC 即与修订一致，验收汇总 #1366 已被本票阻塞（live：#1366 blocked_by 含 1539）；#1411:24 写明父汇总票不作本片 blocker，加边会把文书同步变成实施前置。ADR 0038:101 列的 #1381/#1399/#1418 同理不变。
- **#1769**（OPEN，launcher 与 X 骨架，含 FU-16 时钟义务）：消费 ADR 0039 修订节的恢复读法；experiment/production 恢复不回退，与 `store.rs:1710-1711` 的「恢复时刻不早于最后提交」一致；simulation 重跑能否按 (a)(b) 幂等命中取决于时钟值逐位复现，归 FU-16 裁定，本票不定 `recover_ns`。`x_execution_session.rs:22` 的 `dry-run` 不在 `evidence_class` 三值内，交该票按 ADR 0056 裁定二处置。不新增边。
- **#1535**（FU-13，OPEN）：消费裁定七的 crosswalk 定义；其「切换/回退不丢权利」与 C09.11、ADR 0038 修订第 3 条同向。已有边，不新增。
- **#1540**（FU-18，OPEN）：场所账户与权限仍归该票。已有边，不新增。
- **#1770**（OPEN）：X 观测记录义务不变；原件字段义务（裁定六）由 #1425、#1400 落地。不新增边。
- **#1400**（OPEN，TB-08-B）：消费裁定六类别三与裁定二——账户身份、订单、持仓、回报前沿查询结果以场所 adapter 原件进 E，不推进 F、不产生分配、不写 B；持仓只作对账列，订单/回报前沿按其 C10.5 能力证据（#1400:12）作 Q9 证据载体。不新增边：live 已有传递链 #1539 → #1540 → #1382 → #1400。
- **#1425** 的消费范围（边已在上）：裁定三（C07.8 两项前件与 #1425:13「缺前件、累计完整或不可再执行证据保原潜量」一致）、裁定六类别一（成交投影及其 fill 费用/更正）与类别二（command/period 费用按 ADR 0032:33 归属，不伪造成交投影）。
- 读法不变的正本：ADR 0032:33（费用三类因果）、ADR 0024 裁定六（period 坐标）、ADR 0051:311（可重算投影可物化）。
- **#1330**（OPEN）：X 侧 `OwnAttempt/MayHaveCalled` 实体规则仍归该票（ADR 0051:320）。不新增边。
- **#1550**（ADR 0058，PR #1821 已合入 main）：其 :45/:163 在回放外壳（simulation）里引 ADR 0038:60 与 H2，与修订第 2、4 条一致，无需改动。本 ADR 取 0060（落地时 main 已占用至 0059）。
- 读法不变、无需修订的引用方：ADR 0051:274、ADR 0042:40、ADR 0052:175/:296。行号漂移登记（不在本票修）：ADR 0045:23/:63、ADR 0048:114/:378 的「ADR 0030:44」；ADR 0039:300 与 ADR 0056:40 的代码指针。
- 建议下游增补的测试名（不属本票验收）：#1425 `venue_original_with_family_or_target_rejected`、`venue_original_allocation_slot_equals_external_fact_id`、`physical_position_view_never_writes_b`；#1425 `nonatomic_transfer_waits_for_non_executable_and_cumulative_complete`、`command_and_period_fee_raw_keep_adr0032_cause_path`；#1400 `account_query_raw_does_not_advance_f_or_write_b`；#1366/#1769 `experiment_recovery_keeps_recorded_after_checkpoint`、`simulation_recovery_rerun_is_idempotent_noop`；#1411 `experiment_kill_recovery_h2_six_faces_equal_with_controlled_venue_fixture`。
- 考古没有 research/probe 缺口，不开 research 票或探针票。

## 授权引用

本票分级评论 [6543394411](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6543394411) 判 T（依 `.sandcastle/docs/graph-engineering.md:23`）；两轮独审 BLOCK 后按同文件 :73「再 BLOCK 转 P 类」转 P，影子评论 [6544082577](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6544082577) 已挂 `needs-approval`。本稿不援引 #1323 Notes N-1 自动落地；按 :78「用户批量批复后按 T 类落地」，经用户 `/graph-approve` 批复后才写正本，resolution 首行写用户批复。修改 ADR 0038/0039 两份已裁正本也由该批复覆盖，先例为 ADR 0039:485-487（#1523 P 类批复后追加修订节）。

用户 2026-11-02 批复：「批准推荐稿及正本追加修订」（[issuecomment-6564138279](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6564138279)）。完整引用自证表与考古对照见存档 `.chanlun/review-results/issue1539-decision-20261102/draft.md`「引用自证」「考古对照」两节。
