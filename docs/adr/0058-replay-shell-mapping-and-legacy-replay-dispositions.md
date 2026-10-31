# ADR 0058：#1304 回放外壳映射与旧回放件终态

> 正本：本文件是 #1550（FU-19）#1304 回放外壳与唯一 launcher `production_session` 之间的 seam、外壳输出合同映射、旧回放件终态表、#1528/#1607 处置与 AT-14 实体报告（`at14-report/1`）载体、路径和判定口径的正本。
> 最后裁定：#1550（FU-19，2026-10-31，#1323 Notes N-1 预授权 T 类判定）
> 受影响代码清单：见本文「受影响票与 blocking 边变化」节（`rust/src/bin/production_session.rs:25-26`、`rust/src/bin/x_execution_session.rs:22/:55`、`rust/src/bin/replay_compare.rs:94`、`rust/src/trading/replay_comparator.rs:151`、`rust/src/trading/replay.rs`、`rust/tests/replay_compare_test.rs`、`scripts/replay_compare.sh`、`.github/workflows/ci.yml:516-523`；实施归 #1769、#1381、#1411、#1413、#1418）

**状态**：T 类裁定，文书待 main 合入。本 ADR 只裁映射与终态，不产运行证据（同 ADR 0052:7、ADR 0033:5）。
**行号基线**：草稿行号以起草时 main 为准；本文照搬草稿行号，不重定位。落地基线 main `457902e3c3`。

## 问题

#1550（FU-19）要把 #1304「生产回放验收系统」接进 #1323 的完整生产程序。票面有三项待决：

1. **外壳调用什么**。#1304 票面 :7 写的是「复用生产 runner 路径本身（判定面已统一），新建的只是**历史 feed 适配 + 确定性回放控制 + 对拍 harness**」。但 main 上 `rust/src/bin/replay_compare.rs:94` 实际是 `let engine = Box::new(MinimalEngine);`。#1304 评论 6245981746 已核认它「产生空信号/固定3层零账；ReplayBook只有bar/snapshot计数与层名，比较器仅长度及时间戳，业务字段仍TODO」，所以不能代完整回放。ADR 0039 裁定三已把正式 launcher 定为 `production_session`，main 上 `rust/src/bin/production_session.rs`（502 行，首行自述「#1769（ADR 0039 迁移步骤 1）：唯一生产 launcher 可运行骨架」）和 `x_execution_session.rs` 都已存在。还没定的是 seam：#1304 外壳与这个唯一入口之间怎么接，外壳包括什么、不包括什么。
2. **外壳输出合同**。票面五个词「逐重状态、毛账、订单/成交/持仓、恢复、AsKnown」，各自对应哪个已裁对象。
3. **AT-14 实体报告**。由谁产出、写哪些字段、放在哪里、CI 怎么查、PASS/FAIL 按什么判。

考古标出、本次取证坐实的冲突：

- **#1528 旧正文与 ADR 0039:217 冲突**。#1528 body :19 要「将 `replay_compare` CLI 从 `MinimalEngine` 替换为 `run_theta_v0_pi` 生产链」；ADR 0039:217 已把 `theta_backtest`→`run_theta_v0_pi` 定为「冻结影子：π 批量对照臂，只出组件证据（C10.1）」，并写明「`backtest_bin` 不进 `production_session` feature」。#1528 的头注（body :3）已经自降为「具名组件/影子工具」，但旧正文没改。ADR 0052:292 把这条冲突和 #1607 比较器的去向一起交给本票裁。
- **ADR 0052:336 的验收项还没闭合**。它要求 #1550 评论里 `#1528`、`#1607` 各出现至少 1 次。实测 #1550 现有两条评论（6404224864、6543393815），两者都是 0 次。
- **代码里的 `dry-run` 与正本 evidence_class 三值冲突**。`production_session.rs:26` 的 `PROFILES` 和 `x_execution_session.rs:22` 的 `MODES` 都是四个值，多出一个 `dry-run`。ADR 0038:74、ADR 0056:23-27、CONTEXT.md:374 只认 `{simulation, experiment, production}`，而且 adapter 装配（ADR 0039:280）和时钟 kind（ADR 0056:52）都挂在这三个值上，`dry-run` 没有对应规则。**订正考古**：考古 Q7 拿 #1536 评论 6391077284「dry-run 不算已裁定」当这条冲突的证据。核原文后，这条评论说的是 decision-claimer 的 dry-run 运行态（「仅有旧 dry-run `shadow_p` 的 `/Volumes/AgentStorage/decisions/1536` 改名保留」），与 profile 值 `dry-run` 无关。本 ADR 不用它作依据，冲突只凭代码与正本的直接对照成立。
- **ADR 0052:25 和 :368 登记的图结构风险**。ADR 0052:25 写「如果再加 #1523 blocks #1381，就会出现 TB-07 等 #1523、#1523 又等 TB-07 的环」；:368 要求把「`production_session` 骨架什么时候可供 #1381 使用」作为图结构风险「在 #1550 一并登记」。现在骨架由 #1523 的子票 #1769 交付（ADR 0039:493「B10、B11 的验收票写作『#1523（经子票 #1769）』」；`rust/Cargo.toml:293` 注释「#1769：唯一生产 launcher」），#1769 没有 blocker，但它和 #1381 之间没有边。
- **chong-traceability.md:50 与 main 现状不符**。它写「该基线尚无这两个文件」，但这句话锚定的是文件头 :3 声明的基线 `99df2ed762201236b89f0e81c0276cb3aa847f0a`，相对那个基线是真的。本票不改教义，只在 :65 补指针。

本票只裁映射与终态，不产运行证据（先例：ADR 0052:7、ADR 0033:5）。不涉及资金参数和真实账户，也不裁 #1304 关票和 #1279 去向（这两件归 TB-11 #1369 / FU-20 #1551）。

## 权威依据

**入口与 seam（A）**
- `docs/adr/0039-production-entry-inventory-and-unique-launcher.md:276`：「全仓只有这一个 launcher。`theta_backtest`、`trading_system/live/runner.py`、`newchan` CLI、任何 `*_launcher.py` 都不是 launcher。」
- 同 ADR `:280`：「`evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合，`experiment` 和 `production` 装场所 adapter。三种运行环境只在 adapter 处分叉（ADR 0027:46）。」
- 同 ADR `:296`：「AT-01/02/03/16 以 `production_session` 拉起的独立进程为最高缝（C12.1）。直接调用 `s_structure_session` 等域 bin 只算组件证据。」
- 同 ADR `:217`：`theta_backtest` 行，终态「冻结影子：π 批量对照臂，只出组件证据（C10.1）」，失去写权的机制是「`backtest_bin` 不进 `production_session` feature」。`:220`：「42 个无 required-features 的 bin、`lav_seal_samples`」也是冻结影子，「不在生产二进制集合内」。
- 同 ADR `:493`：B10、B11 由「#1523（经子票 #1769）」验收，运行对象是 #1769 交付的两个二进制。
- `docs/adr/0027-classification-chong-operating-program-ownership.md:46`：「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。不得为任一运行环境保留特有的结构判定、重状态推进或交易决策程序。」
- `.chanlun/definitions/chong-traceability.md:11`：「唯一正式 launcher 是 `production_session`……`run_theta_v0_pi` 不是正式 launcher 的替代链。」`:65` 是本票承接行：「#1304 外壳调用正式 launcher，输出逐重状态与毛账、恢复和 AsKnown 证据，完整 AT-14 报告。」
- `docs/adr/0052-history-input-profile-and-model-order-unspecified.md:285`：「验证入口只有 `production_session`……它以 `evidence_class=simulation` 装配受控撮合 adapter……并装载 `history-input-profile/1`。」`:286`：「#1523 交付之前，这条路径处于『等前置』状态，不用影子工具顶替。」`:291` (d)：「`theta_backtest`/`replay_compare`/`theta_replay` 的产出只标为『组件证据』，不进入 FU-14 判据。」`:292`：「#1528 旧正文……和 #1607 比较器的去向，登记交给 #1550 裁决。」`:300` 是 `scenario_contract.fault_schedule[]`；`:301` 是 `scheduler = "launcher-sched/sequential/1"`。
- #1340 body `:344`（C10.1）：「提供唯一完整会话入口……单独 π batch、ThetaPiStream 或旧 Python 递归结果仅可标注对应组件证据。」`:350`（C10.7）：「#1304 的历史数据/回放外壳接入本入口」。`:363`（C12.1）：「以最高完整会话入口为主测试缝」。`:448`（范围外）：「用局部研究臂 alpha、单组件回放或 UI 图形代替完整程序及真实场所验收。」
- `rust/src/bin/production_session.rs:25` 是 `MANIFEST_REVISION: &str = "production-session-manifest/1"`；`rust/Cargo.toml:293-302` 中两个 bin 都是 `required-features = ["production_session"]`。

**输出合同（A）**
- `docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:49-54` 定六面：1 S 结构快照；2 各重 B 账（Voice、TStage latch 值与转换历史、筹码、毛账、cost_remaining）；3 E 责任；4 订单/成交原件（epoch、身份、前后状态、cause 链）；5 物理持仓（场所回报与内部投影对账）；6 外效历史。
  - `:56`：「checkpoint 取每 bar 收盘后、下一 bar 开盘前」，批内顺序为 `Unknown`。
  - `:60`：「launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar；杀停等同未完成 bar 的全部外效未发生，S/B/E 回退到上一 checkpoint。」
  - `:74`：evidence_class 三值；「simulation 产出不进 TStage 触闸计数」。
- #1340 body `:323`（C08.6）：「AsKnown 使用实际 S/E/B/X 记录和依赖向量；恢复的晚到应用不能回填为故障当时已处理。RecomputedWithRevision 独立命名并绑定输入/规则，不冒充当时可知。」`:338`（C09.9）：「B 恢复重放原归属应用，不从场所净仓重建成本」。`:347`（C10.4）：「比较完整结构关系、逐重状态/毛账、责任、命令/回报/分配和 AsKnown……禁止只比较最终净仓。」
- `docs/adr/0056-three-mode-shared-core-adapter-boundary.md:70`：「比较面与时点：ADR 0038:49-54 六面全部字段……各面导出按（域, `event_id`）排序」。`:59`：「一个会话只有一个 `evidence_class`，也只有一个外部事实源」。
- ADR 0052 `:280`：「#1418 的报告单列 `ModelOrderUnspecified` 计数。双跑比较面（ADR 0038:49-54 的第 3、4 面）包含这个终态事件。」`:350`：「#1418（TB-07-D）：报告单列 `ModelOrderUnspecified`、`DiagnosticNotProduction`、`InputSourceUnavailable` 计数，以及『校验源』标注。」
- `CONTEXT.md:451`：「`TStage` latch 只投影 campaign 内达标事件史（AsKnown 不回填）」。

**判定口径与报告（A / B）**
- ADR 0038 `:84`：「任一 H1–H4 的 diff 非空 ⟹ FAIL 并留档……四项全空 ⟹ PASS。」`:88-91` 是四个 cargo test 名：`dual_run_determinism_h1`、`recovery_after_kill_h2`、`seek_no_future_leak_h3`、`asknown_evidence_segregation_h4`。`:94`：「TB-11 整图验收包含 H1–H4 四项 oracle 的实际运行证据」。
- ADR 0052 `:288-291`：FU-14 判据 (a)–(d)，四条都是二值。
- #1340 body `:434`：「现有对拍仅信号面硬断言、PASS 可含 inconclusive、同进程第二实例恢复、忽略测试或缺依赖跳过，都不能计作本 SPEC 通过」。`:436`：「新增一个从正式 launcher 启动的完整会话测试夹具……使用两个独立新进程双跑及真正杀进程/重启……不继承旧 expected/advisory 规则掩盖差异」。`:438`：「运行报告保存输入/schema/构建/政策/能力 hash、实际负载、故障与调度序列、完整输出、未执行项以及独评绑定面」。`:426` 是 AT-14 行。
- #1418 body :11：「报告至少含输入/schema/构建/规则/模型/政策/能力hash、实际负载、故障与调度、三面及完整域差异、未跑项、证据与独评绑定，不能只输出信号计数」。:12：「验收报告器和生产改动触发的回归锁有真实代码/CI或项目既定载体及失败负对照」。:13：「报告明确只证明有界受测域」。
- `docs/agents/delivery-discipline.md:16`（第 6 项）：「关票时本票报告……已入 git，以仓内相对路径引用（如 `chanlun/review-results/`）……引 `/tmp` 或绝对路径视同未入仓」。`:15`（第 5 项）：「有在制品未合入的不得关票，先处置在制品」。`docs/agents/generation-constitution.md:52`：「review-results / handoff = **工作草稿**……**活期绑定票**——关联票 open 期间留仓，票关后月度归档」。
- `.github/workflows/ci.yml:516-523`：已有 `cargo test … --features production_session_testonly --test production_session_entry`（:517）和纯生产构建 `--features production_session`（:523）两段。
- `rust/src/bin/theta_accept.rs:27`：「`--out DIR` 报告写盘（每品种一份 <SYMBOL>.md + summary.md；默认只打 stdout）」。这是影子工具的先例。

**旧件终态（A / B）**
- #1532 评论 6286105763 表行：「`trading::{replay, replay_comparator}`、`bin/replay_compare.rs` | **冻结影子** | — | 不持域库 | D4 回放对拍器 | **不由本票定**：回放功能件去向归 #1304」。
- `docs/adr/0048-legacy-engine-module-level-dispositions.md:20`：「不替 #1096/#1304 做退役取舍」。`:208` B-Q11：「`ThetaPiStream`/`PyThetaStream`：默认值 冻结影子，本票不授权删除。退场条件 = #1385 :16 OE08 勾选 ∧ #1413 CLOSED ∧ #1550 CLOSED；删除由 #1304 路径定」。`:287`（`backtest_bin` 门内 bin，退场 `#1390:17✓ ∧ #1413 CLOSED`）、`:288`（42 个无 required-features 的 bin，同一退场条件）、`:289`（ThetaPiStream 行「#1304 外壳接入由 #1550 裁」）。
- #1323 body `:64-69` 是六项 Destination，其中 :67 ④「完整历史回放可执行：历史数据驱动的确是上述完整程序，输出逐重状态、毛账、订单投影、成交与唯一真账；同输入双跑确定」，:69 ⑥「#1304 作为回放功能工作线接入本图」。`:275`：「#1096/#1304 义务不提前删除，#1319/#1320/#1279 继续受本图约束。**#1279 不关图、不宣布试运行放行」。
- #1304 评论 5501144736：「#1304 保留回放功能工作线名分……不得把核级 theta_replay 结果冒充完整程序读数」。评论 5596451156：「完整历史入口、恢复/AsKnown、全部状态对拍由TB-07 #1366接单，其中内部TB-07-D明确交付实际验收报告器和生产改动回归锁……#1320七标的十年1min/冻结7/7重型测量额外义务单列保留」。评论 6245981746：「1381完整历史feed→全部权威域；1399 seek/修订/AsKnown；1411双独立进程/全状态及真实崩溃恢复；1418实际报告器与回归锁」，以及「p123消费列式OHLCV，replay CLI消费对象数组；真实DIV/TERM/CERT与比较器要求的bar/timestamp_ms格式不符」。
- #1607 body :21：「本票先请 #1304 owner 定：是补齐这条比较，还是随影子退场删除」。:30-33 是二选一，其中「**退场**：随冻结影子删除，并在 #1304 记录去向」。
- ADR 0056 `:92`（裁定八）：「现有 Python/NT 外壳只做模式输入、协议与展示适配……不得成为第二个 launcher……逐条去向归 #1550 与 TB-10 #1368。」#1340 body `:330`（C09.1）、`:355`（C11.2）「旧 Python 回放/session/API 外壳迁为同核 adapter」、`:357`（C11.4）「每个条目仅有收编、冻结影子、删除之一的终态，附替代入口、失去的生产写权、保留用途和退场条件……冻结影子只输出明确带 provenance 的比较读数，不能影响准入、资金或订单」。

**通则**
- `AGENTS.md:48`（#799）：「同一个判断不得存在宽严两档实现，更不得让宽松的那档接管严格那档的失败」。`:55` 限定词 2：「诊断模式 vs 生产模式：诊断口径可更宽……但不得参与准入」。
- `docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md` 裁定一（:9 适用范围）、裁定二（:11 与 #799 的关系）、裁定三（:17 落法）、裁定四（:27-30 合法候选保留，真错仍排除）。
- `.agents/skills/domain-modeling/SKILL.md:68-72` 是 ADR 三条件；`.sandcastle/docs/graph-engineering.md:23` 是 T 类范畴。

## 候选

### Q1 入口（A，沿用）
唯一入口是 `production_session`（ADR 0039:276；ADR 0052:285；chong-traceability.md:11）。不列为待决。

### Q2 旧回放件终态与 #1528/#1607 的处置（B）
默认值：**全部「冻结影子·只出组件证据」；不接 `run_theta_v0_pi`，不升级为完整回放；各件沿用 ADR 0048 现有行的退场条件**（出处：#1532 评论 6286105763 表行；ADR 0039:217/:220；ADR 0052:291；ADR 0048:287/:288/:289；ADR 0027:46）。

推理链重过 `[新缠论:选择]`：
1. ADR 0039 裁定三已定全仓唯一 launcher，且 `backtest_bin` 不进 `production_session` feature（:217）。以 `run_theta_v0_pi` 为主干的「接入生产链」等于第二条决策主干，与 ADR 0027:46 正面冲突，所以 #1528 旧正文的目标不可执行。这不是「比较下去的合法取值」，而是违反已裁推论，按 ADR 0041 裁定四（:30）排除。
2. #1607 要补的是 D4↔p123 dump 的 CERT/DIV/TERM 对拍（`replay_comparator.rs:151` 现在是 TODO 加 DevSkim 抑制注释）。AT-14 的正式比较面已由 ADR 0038:49-54 六面加 ADR 0056:70 导出规则规定。补上它只会多一个与正式 oracle 平行的比较器，产出仍是组件证据，产生不了 Destination ④ 证据，还落进 #1340 :448 的「单组件回放代替完整程序」。
3. 代码实测：`MinimalEngine` 在 rust/ 里共 18 处，全部在 `replay.rs`(5)/`replay_comparator.rs`(4)/`bin/replay_compare.rs`(3)/`tests/replay_compare_test.rs`(6)。`trading::{replay, replay_comparator}` 的使用方只有 `bin/replay_compare.rs:17-20` 和它的测试，没有任何域进程引用。`cargo metadata` 显示 `replay_compare` 是 `required-features: null` 的自动发现 bin，属于 ADR 0039:220 / ADR 0048:288 那一行；`theta_replay`/`theta_accept` 在 `Cargo.toml:206-208/:213-215` 是 `backtest_bin` 门内 bin，属于 ADR 0048:287 那一行。所以终态和退场条件都已有现成的行，本票不另立新条件。
4. ADR 0048:208/:289 把 `ThetaPiStream` 的退场条件绑在「#1550 CLOSED」上，本票不裁，这条退场链就断在这里。删除本身仍走 #1304 路径（#1323:275「#1096/#1304 义务不提前删除」）。本票只提供这个前置，不授权删除任何文件。

保留可选用途：`theta_replay`/`theta_accept`/`replay_compare` 继续作诊断臂，输出必须带 provenance、标明「组件证据」（#1340 :357）。这属于 AGENTS.md:55 限定词 2 的正当分化，不是宽严两档。

### Q3 输出合同映射（A，沿用）
票面五个词映射到 ADR 0038 六面、ADR 0038:56/:60/:74 和 #1340 C08.6，详见裁定二。

### Q4 AT-14 报告的载体与路径（B）
默认值：**由 #1418 报告器在 `production_session`（`evidence_class=simulation`）完整会话夹具结束时产出。载体是具名版本化记录 `at14-report/1`（机器可读 JSON）加一份人读 `summary.md`。入仓路径是 `.chanlun/review-results/issue1304-at14-<YYYYMMDD>-<short-sha>/`。CI 载体是 ADR 0038:88-91 的四个具名 cargo test，进 `ci.yml` 的 `production_session` feature 段**（出处：#1340 :438；#1418 :11-13；ADR 0052:280/:350；ADR 0038:88-94；delivery-discipline.md:16；generation-constitution.md:52；ci.yml:516-523；theta_accept.rs:27 只借形式）。

推理链重过 `[新缠论:选择]`：
1. 字段清单已由 #1340 :438、#1418 :11 和 ADR 0052:280/:350 写死，缺的只是载体与路径。
2. 仓内「实体报告」唯一的硬口径是 delivery-discipline.md:16（入 git、用仓内相对路径）；generation-constitution.md:52 又规定 `review-results/` 活期绑定票，所以目录必须绑一张 open 票。考古默认绑 #1366，本裁定改绑 **#1304**，理由：
   - 报告的消费者是 #1304 和 #1279，两张票都在 TB-11 才得到合法去向（#1340 :378）。
   - 绑 #1366 的话，报告可能在 TB-11 总验之前就被月度归档移出仓。
   - 仓内已有 `.chanlun/review-results/issue1304-phase0-specs-20261017.md` 和 `issue1304-replay-system-frontier-20261017.md`，同前缀有先例。
   - 绑 #1304 不改 #1304 的任何义务。
   `issue1366-…` 作为另一种合法取值留在字段说明里，代价是 #1366 一关就被归档（ADR 0041 裁定四 :29）。
3. CI 可查性来自四个具名测试进 `production_session` feature 段（ci.yml:516-523 已有该段），不来自报告文件。报告是测试夹具的产物，它的 `ci_run_id` 字段指向那次 run，不构成第二个判据。四个测试名在 rust/ 当前 0 命中，这是 #1411/#1418 的实施缺口，不是裁决缺口。
4. `theta_accept.rs:27` 的「`<SYMBOL>.md + summary.md`」只借「人读 md + 机器数据」的双形式，不借它的比较面。

### Q5 判定口径（A，沿用）
沿用 ADR 0038:84、ADR 0052:288-291 (a)–(d)、#1340 :434/:436。详见裁定五第 5 条。

### Q6 实施归属（A，沿用）
#1381 负责输入与 profile 装载，#1399 负责 seek/AsKnown，#1411 负责双进程与恢复，#1418 负责报告器与回归锁（#1304 评论 5596451156 / 6245981746）。本票只裁映射。

### Q7 `dry-run`（B）
默认值：**#1304 外壳只以 `evidence_class=simulation` 调用 `production_session`。回放/AT-14 路径不接受 `dry-run`，#1418 报告器拒收 profile 值不属于 `{simulation, experiment, production}` 的会话产物。`dry-run` 在 launcher 里的存废作为冲突登记转给 #1769（骨架 owner），本票不代裁**（出处：ADR 0038:74；ADR 0056:23-27/:52/:53；ADR 0052:271/:285；CONTEXT.md:374；`production_session.rs:26`；`x_execution_session.rs:22/:55`；ADR 0039:322；#1769 body :17）。

推理链重过 `[新缠论:选择]`：
1. 正本三处一致只认三个值，并把 adapter 装配（ADR 0039:280）、时钟 kind（ADR 0056:52「由 `evidence_class` 一一推出……不设可选值」）、TStage 触闸（ADR 0038:74）都挂在 evidence_class 上；`dry-run` 没有对应规则。evidence_class 的取值集合是已裁推论，不是 ADR 0041 意义上的自由度（裁定一 :9），不能给它补一个可调值。
2. ADR 0039:322 的「这一步可以无凭据 dry-run」是对离线 Step 2 的口语描述，不是 profile 值。#1769 body :17「simulation 与 dry-run 的运行命令文档」说明骨架作者把它当成了一种运行模式。
3. 回放外壳按 ADR 0052:271/:285 只走 `simulation`，与 `dry-run` 没有交集，所以本票不裁它的存废也能完成映射。

### Q8 #1304 / #1279 / #1320 的去向（A，沿用）
归 TB-11 #1369 与 FU-20 #1551。#1279 不关，#1320 单列（#1323:275；#1340 :226/:378；#1304 评论 5596451156；ADR 0052:310）。本票不动。

### Q9 旧回放数据协议（B）
默认值：**不修。#1304 外壳的输入一律是 `history-input-profile/1` 加 `production-session-manifest/1`（由 #1381 交付）。p123 dump、replay CLI 对象数组和 `scripts/replay_compare.sh` 保持现状，终态表标「协议已断、不修复、仅诊断」**（出处：ADR 0052:3/:285/:306；`production_session.rs:25`；#1304 评论 6245981746；#1340 :448）。

推理链重过：正式输入合同已由 ADR 0052（:3 正本声明）定为 `history-input-profile/1`，和 p123 dump、对象数组都不是同一个东西。旧协议两端按 Q2 都是冻结影子，影子之间的协议断裂不影响任何正式证据。要修就得写转换脚本和 manifest，产出仍只是组件证据，又落进 #1340 :448。数据源缺失按 ADR 0052:306 的 `input_source` 可调字段处置，本票不派探针。

### Q10 Python/NT 外壳（A，沿用）
按 ADR 0056:92 裁定八和 chong-traceability.md:52：只做 adapter，不得成为第二个 launcher。终态实施归 TB-10 #1368，条目见 ADR 0048:289-290。本票只给指针。

### Q11 正本落点（B）
默认值：**新立 ADR 0058（英文短名 `replay-shell-mapping-and-legacy-replay-dispositions`）；chong-traceability.md:65 行末只补 ADR 指针，不改教义和三行头**（出处：同系 FU-14/15/16 都落在 ADR 0052/0038/0056；domain-modeling SKILL.md:68-72 三条件；chong-traceability.md:3 基线声明）。

推理链重过：本票要裁四件事——旧回放件终态、#1528/#1607 处置、`dry-run` 冲突登记、报告 schema。四件都难逆转、没有上下文看不懂、而且有候选被比较下去，满足 ADR 三条件。同系 FU 票都以 ADR 为正本，TB-11 逐项核验时形式一致。chong-traceability.md 是教义正本，代码清单锚定 :3 的基线，不适合承载入口终态表；:50 相对该基线为真，不改。

### C 类
无。考古 C 类为 0 项，本次取证也没有新增事实缺口。

## 裁定

本条依 #1323 Notes N-1 预授权判定（T 类）。

### 一、#1304 外壳的 seam：包括什么、不包括什么 `[新缠论:推论]`（ADR 0039 裁定三与 ADR 0052 裁定三的组合）

1. **#1304 外壳由三件非主干物组成**：
   - (a) 一份 `production-session-manifest/1`（`production_session.rs:25`）和一份 `history-input-profile/1`（ADR 0052:271，`evidence_class` 固定为 `simulation`）；
   - (b) #1340 :436 要求的「从正式 launcher 启动的完整会话测试夹具」（两个独立新进程，真杀进程、真重启）；
   - (c) #1418 的报告器。
   三件都只调用 `production_session`，不包含任何 `main`、runner、引擎、分类或账本代码。
2. **外壳不是**：不是新 bin；不是 `*_launcher.py` 或 shell 包装里的判定逻辑；也不是 `theta_replay`、`theta_accept`、`replay_compare`、`MinimalEngine`、`ThetaPiStream`、`run_theta_v0_pi` 中的任何一个（ADR 0039:276；chong-traceability.md:11）。允许存在的 shell/CI 包装只做 exec 转发，不读结果，也不判定。
3. **调用形式**：`production_session` 载入第 1 条 (a) 的 manifest 与 profile，`evidence_class = simulation`。由此装配受控撮合 adapter（ADR 0039:280）、`logical-bar/1` 时钟（ADR 0056:52）、`fault_schedule`（ADR 0052:300）和 `launcher-sched/sequential/1`（ADR 0052:301）；时钟与 evidence_class 不符时，在拉起任何域之前以 `InvalidDomain` 退出（ADR 0056:53）。`dry-run` 不是 evidence_class，回放/AT-14 路径不得出现它；报告器遇到即拒收。它在 launcher 里的存废不在本票裁，作冲突登记转 #1769。
4. **输入协议**：只认第 1 条 (a) 的两份文件。p123 列式 dump、replay CLI 对象数组、`scripts/replay_compare.sh` 的协议断裂（#1304 评论 6245981746）不修复，标「协议已断、仅诊断」。

### 二、输出合同：票面五个词对应的已裁对象（沿用 A，不新立对象）

| 票面词 | 对应的正本对象 | 出处 |
|---|---|---|
| 逐重状态 | ADR 0038 面 2：Voice、TStage latch 值与转换历史、筹码、cost_remaining（逐重） | ADR 0038:50；CONTEXT.md:451 |
| 毛账 | ADR 0038 面 2「毛账」 | ADR 0038:50 |
| 订单/成交 | ADR 0038 面 4：订单/成交原件（epoch、身份、前后状态、cause 链），含 `ModelOrderUnspecified` 终态事件 | ADR 0038:52；ADR 0052:280 |
| 持仓 | ADR 0038 面 5：物理持仓（场所回报与内部投影对账） | ADR 0038:53 |
| 恢复 | ADR 0038 checkpoint 语义（每 bar 收盘后；杀停等于未完成 bar 的外效未发生）加 C09.9 各域恢复次序；用 H2 oracle 验 | ADR 0038:56/:60；#1340 :338 |
| AsKnown | ADR 0038:74 分证（simulation 产出不进 TStage 触闸计数）、latch 不回填，加 C08.6（`RecomputedWithRevision` 独立命名、另列）；用 H3/H4 oracle 验 | ADR 0038:74；CONTEXT.md:451；#1340 :323 |

补充约束：面 1、3、6 也不得缺（ADR 0056:70「六面全部字段」）；各面导出按（域, `event_id`）排序，批内顺序不比（ADR 0038:56）；禁止只比最终净仓（#1340 :347）。序列化载体格式归 #1411/#1418 实施，本票不裁。

### 三、旧回放件终态表（按 #1340 :357 的列）

| 条目 | 终态 | 替代入口 | 失去的生产写权 | 保留用途 | 退场条件 |
|---|---|---|---|---|---|
| `bin/replay_compare.rs`（无 required-features）及只被它使用的 `trading::{replay, replay_comparator}`、`MinimalEngine`、`tests/replay_compare_test.rs`、`scripts/replay_compare.sh` | 冻结影子（#1532 表行） | `production_session` simulation | 本来就不持域库；不在生产二进制集合内（ADR 0039:220） | D4 回放对拍诊断；协议已断、不修复 | 沿用 ADR 0048:288 行：`#1390:17✓ ∧ #1413 CLOSED`；删除走 #1304 路径 |
| `theta_replay`、`theta_accept`（`backtest_bin` 门内） | 冻结影子 | 同上 | `backtest_bin` 不进 `production_session` feature（ADR 0039:217） | 组件证据，输出必须标 provenance | 沿用 ADR 0048:287 行 |
| `ThetaPiStream`/`PyThetaStream` | 冻结影子（ADR 0048:289 不变） | 同上 | 生产进程不加载 cdylib | 同上 | 沿用 ADR 0048:289 原条件；本票关票即满足其中「#1550 CLOSED」一项 |
| 旧 Python 回放/session/API 外壳（`trading_system/`） | 按 ADR 0056:92 只做 adapter；逐条终态归 ADR 0048:289-290 与 TB-10 #1368 | — | — | — | 本票只给指针 |

### 四、#1528 与 #1607 的处置

1. **#1528**：旧正文目标「替换为 `run_theta_v0_pi` 生产链」与 ADR 0039:217、ADR 0027:46 冲突，**不实施**。改题关闭为「不实施：与 ADR 0039 冻结影子终态冲突」。它要的「逐 bar 分类/信号/订单/持仓/账目」对拍由父票 #1411 的六面 oracle（ADR 0038:49-54）覆盖，不另开票。在制分支 `codex/issue1528-replay-compare-20261018`（origin head `72b1dbcc73`，见 #1528 评论 6243030915 / 6278136284）按 delivery-discipline.md:15 先处置，再关票：不合入，在 #1528 留下 head SHA 后删除远端分支。#1411 要复用这批组件，就按该 SHA 自取。
2. **#1607**：选票面的「退场」臂，**不补齐**。文件不在本票删除（终态表的退场条件还没成立）。`replay_comparator.rs:151` 的 TODO 和 DevSkim 抑制注释改成带票号、指向 ADR 0058 的一行指路注释，这项订正归 #1413（TB-10-I 入口排除边界）。按 #1607 body :33「在 #1304 记录去向」，在 #1304 发去向登记，然后关闭 #1607。
3. 两项处置都在 #1550 发冲突登记评论，补齐 ADR 0052:336 的验收项。

### 五、AT-14 实体报告 `[新缠论:选择]`

1. **产出者**：#1418 报告器，在第一条第 1 项 (b) 的夹具会话结束时写出。`TestOnly` 会话、诊断夹具和 `backtest_bin` 影子的产出不得写入报告目录。
2. **载体**：具名版本化记录 `at14-report/1`（JSON，JCS 规范化后取 `content_hash`），字段如下：
   - `profile`/`manifest` 回显与 hash（含 `evidence_class`、`clock.kind`、`scheduler`）；
   - 构建指纹（commit、lockfile hash、features）；
   - 规则/政策/模型版本集；
   - 能力 manifest hash；
   - 实际负载（`input_range`、bar 数、checkpoint 数）；
   - `fault_schedule` 与调度轨迹的实际值；
   - `h1`..`h4`，各为 `{result: PASS|FAIL, diff_summary}`；
   - 六面逐面行数与 diff 摘要；
   - `model_order_unspecified_count`、`diagnostic_not_production_count`、`input_source_unavailable_count` 和「校验源」标注（ADR 0052:280/:350）；
   - `unexecuted[]`；
   - `bounded_domain_statement`（「仅证有界受测域」，#1418 :13）；
   - `ci_run_id`；
   - `review_binding`（独审 commit）。

   人读的 `summary.md` 从同一份数据渲染，不含独立判定。改字段即升为 `at14-report/2`。
3. **路径**：`.chanlun/review-results/issue1304-at14-<YYYYMMDD>-<short-sha>/{at14-report.json, summary.md}`，用仓内相对路径引用。#1304、#1279、#1369 的评论只引这个相对路径和 commit。另一种合法取值 `issue1366-…` 留在字段说明里，代价是 #1366 先于 TB-11 关闭就会被归档。
4. **CI 载体**：ADR 0038:88-91 的四个具名测试进 `ci.yml` 的 `production_session` feature 段（ci.yml:516-523 已有该段）。回归锁就是生产改动 PR 触发同一段。报告不是第二个判据，只是测试的产物。
5. **判定口径**（沿用 A）：AT-14 PASS 当且仅当两件同时成立：该会话满足 ADR 0052:288-291 (a)–(d)，并且 ADR 0038:84 的 H1–H4 四项 diff 全空。任一不成立就判 FAIL 并留档。旧的 `expected/advisory`、`inconclusive`、同进程第二实例恢复、忽略或跳过的测试，一律不计 PASS（#1340 :434/:436）。(a)–(d) 判「跑的是不是正式对象」，H1–H4 判「行为对不对」，两者是不同判断，不是宽严两档。

### 六、不裁的事项与指针

- #1304 关票、#1279 去向、#1320 重型义务：归 TB-11 #1369 / FU-20 #1551（#1323:275；#1340 :378；ADR 0052:310）。
- `dry-run` 存废：转 #1769。
- 图结构风险（ADR 0052:25/:368）：加边 #1769 → #1381，让 #1381 显式等待骨架。#1769 没有 blocker，不会成环，也不碰 ADR 0052:368 拒加的那条 #1523 → #1381。
- chong-traceability.md:50 相对 :3 的基线为真，不改；只在 `:65` 行末补 ADR 0058 指针。

正本落点：docs/adr/0058-replay-shell-mapping-and-legacy-replay-dispositions.md

## 验收清单

- [ ] 文件存在：`test -f docs/adr/0058-replay-shell-mapping-and-legacy-replay-dispositions.md` 退出码 0。
- [ ] 命令：`rg -F 'at14-report/1' docs/adr/0058-replay-shell-mapping-and-legacy-replay-dispositions.md | wc -l` 输出 ≥ 1。
- [ ] 命令：`rg -F 'dry-run' docs/adr/0058-replay-shell-mapping-and-legacy-replay-dispositions.md | wc -l` 输出 ≥ 1（冲突登记在正本内）。
- [ ] 命令：`rg -F '0058-replay-shell-mapping' .chanlun/definitions/chong-traceability.md | wc -l` 输出 ≥ 1。
- [ ] 命令（教义未改，diff 只限 #1550 行）：`git diff -U0 origin/main...decision/1550 -- .chanlun/definitions/chong-traceability.md | rg '^[-+][^-+]' | rg -v '^[-+]\| #1550 \|' | wc -l` 输出 = 0，且 `git diff -U0 origin/main...decision/1550 -- .chanlun/definitions/chong-traceability.md | rg '^[-+]\| #1550 \|' | wc -l` 输出 = 2。
- [ ] 命令：`rg -n 'MinimalEngine|run_theta_v0_pi|ThetaPiStream|replay_comparator' rust/src/bin/production_session.rs rust/src/bin/x_execution_session.rs | wc -l` 输出 = 0。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1550/comments --paginate --jq '.[].body' | rg -F '#1528' | wc -l` 输出 ≥ 1，且把 `'#1607'` 代入同一命令也输出 ≥ 1（ADR 0052:336 项）。
- [ ] 票状态：`gh issue view 1528 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`，且 `gh api repos/xy7365527-lang/NewChanlun/issues/1528/dependencies/blocked_by --jq '[.[]|select(.number==1381)]|length'` 输出 `0`。
- [ ] 命令：`git ls-remote --heads origin codex/issue1528-replay-compare-20261018 | wc -l` 输出 = 0，且 `gh api repos/xy7365527-lang/NewChanlun/issues/1528/comments --paginate --jq '.[].body' | rg -F '0058-replay-shell-mapping' | wc -l` 输出 ≥ 1。
- [ ] 票状态：`gh issue view 1607 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1381/dependencies/blocked_by --jq '[.[]|select(.number==1769)]|length'` 输出 `1`。
- [ ] 票状态：对 N ∈ {1304, 1411, 1413, 1418, 1769}，分别运行 `gh api repos/xy7365527-lang/NewChanlun/issues/N/comments --paginate --jq '.[].body' | rg -F '0058-replay-shell-mapping' | wc -l`，五次输出都 ≥ 1（当前基线都是 0）。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1418/comments --paginate --jq '.[].body' | rg -F 'at14-report/1' | wc -l` 输出 ≥ 1。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1413/comments --paginate --jq '.[].body' | rg -F 'replay_comparator.rs:151' | wc -l` 输出 ≥ 1。
- [ ] 票状态：`gh issue view 1304 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `OPEN`，且 `gh issue view 1279 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `OPEN`。
- [ ] 票状态：`gh issue view 1550 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`（PR 合入后）。
- 下游运行验收（不在本票）：测试 `dual_run_determinism_h1`、`recovery_after_kill_h2`、`seek_no_future_leak_h3`、`asknown_evidence_segregation_h4` 归 #1411/#1418，CI 段归 #1418；文件 `.chanlun/review-results/issue1304-at14-*/at14-report.json` 存在，也归 #1418。

## 受影响票与 blocking 边变化

| 票 | 状态 | 处置 | 边变化 |
|---|---|---|---|
| #1550 | OPEN→CLOSED | 本票；发 resolution 和 #1528/#1607 冲突登记评论 | 现有 blocked-by 1533/1536/1537/1538（都已 closed）、blocking 1366/1369/1551 不改；关票即解锁 |
| #1528 | OPEN→CLOSED | 改题「不实施：与 ADR 0039 冻结影子终态冲突」；需求由 #1411 六面 oracle 承接；在制分支按 delivery-discipline.md:15 先处置 | **删** #1381 → #1528 |
| #1607 | OPEN→CLOSED | 选「退场、不补齐」；注释订正转 #1413；在 #1304 记录去向 | 无（当前无边） |
| #1769 | OPEN | 承接登记：`dry-run` 与 evidence_class 三值冲突，回放路径只接 `simulation` | **加** #1769 → #1381（#1769 无 blocker，不成环） |
| #1381 | OPEN | 承接：外壳输入 = `production-session-manifest/1` + `history-input-profile/1`（`evidence_class=simulation`） | 新增被 #1769 阻塞；原 blocked-by 1364/1353/1481/1443 不动 |
| #1411 | OPEN | 承接：四个具名测试（ADR 0038:88-91）与六面导出；并入 #1528 的需求 | 无 |
| #1418 | OPEN | 承接：`at14-report/1` schema、`issue1304-at14-*` 路径、ci.yml `production_session` 段回归锁 | 无（blocked-by #1411 不动） |
| #1413 | OPEN | 承接：`replay_comparator.rs:151` 的 TODO/DevSkim 抑制改为带票号的指路注释 | 无 |
| #1304 | OPEN 保持 | 发承接登记（外壳 seam、终态表、报告路径、#1607 去向）；不关票 | 无（blocked-by #1333 不动） |
| #1366 / #1369 / #1551 | OPEN | 被本票阻塞，本票关闭后解锁；TB-11 消费裁定五的判定口径 | 无新增 |
| #1399 | OPEN | 不动（seek/AsKnown 实施方） | 无 |
| #1368 | OPEN | 接收 Python 外壳逐条去向的指针（ADR 0056:92） | 无 |
| #1523 | OPEN | 不动；不加 #1523 → #1381（ADR 0052:368） | 无 |
| #1279 / #1320 | OPEN | 不动 | 无 |
| #1323 | OPEN | Decisions so far 加一行（T 类 N-1；正本 ADR 0058） | 无 |

不需要开 research/probe 票。

**受影响代码清单**（实施归各承接票，本 ADR 不改码）：
- `rust/src/bin/production_session.rs:26`（`PROFILES` 含 `dry-run`）、`rust/src/bin/x_execution_session.rs:22/:55`（`MODES` 含 `dry-run`）：冲突登记转 #1769。
- `rust/src/trading/replay_comparator.rs:151`（TODO 与 DevSkim 抑制注释）：改为指向本 ADR 的指路注释，归 #1413。
- `rust/src/bin/replay_compare.rs`、`rust/src/trading/replay.rs`、`rust/src/trading/replay_comparator.rs`、`rust/tests/replay_compare_test.rs`、`scripts/replay_compare.sh`：冻结影子，不改不删，退场按裁定三。
- `.github/workflows/ci.yml:516-523`（`production_session` feature 段）：接入 ADR 0038:88-91 四个具名测试，归 #1411/#1418。

## 授权引用

本条依 #1323 Notes N-1 预授权判定。用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」。原话记录在 #1323 评论 6261281111（`<!-- notes-N-1-20261019 -->`）；另见 #1323 body :73-74 的 Notes N-1 和 `.sandcastle/docs/graph-engineering.md:90`「授权记录」节。分级评论 6543393815 判为 T，本 ADR 不改判。
