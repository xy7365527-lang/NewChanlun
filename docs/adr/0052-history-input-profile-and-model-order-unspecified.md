# ADR 0052：完整历史输入 profile 与 ModelOrderUnspecified

> 正本：本文件是完整历史输入 profile（`history-input-profile/1` 六域清单）、`ModelOrderUnspecified` 终态与正式 launcher 验证判据的正本。
> 最后裁定：#1536（FU-14，2026-10-28，#1323 Notes N-1 预授权 T 类判定）
> 受影响代码清单：见本文「受影响票与 blocking 边变化」节

**状态**：T 类裁定，文书待 main 合入。本 ADR 只定 profile 字段、终态与判据；实现与运行证据归 TB-07 叶票（#1381 等），不以本 ADR 关票声称实现完成（同 ADR 0033:5）。

## 问题

#1536（FU-14）要裁三件事。按 `.sandcastle/docs/graph-engineering.md:23`，三件都属于 T 类里的「数据结构、状态机与事件 schema、测试锁」：

1. **六域输入 schema**：完整历史输入要覆盖结构、资金、订单、成交、持仓、故障六个域。这是票面验收一，`.chanlun/definitions/chong-traceability.md:63` 也点名本票：「保留结构、资金、订单、成交、持仓和故障，不用影子 CLI 代正式路径」。考古 Q1 查实，每个域的形状都已经有正本：结构用 #1340 C02.1 的原件，资金用 ADR 0029/0049 的引用，订单与成交用 ADR 0033 的版本合同，持仓用 ADR 0038 六面全状态，故障用 ADR 0033 裁定五的 scenario 合同。缺的是一份把六域合在一起的「历史输入 profile」清单。代码里 `input_domain`/`unsupported_domains`/`model_ref` 一处都没有，所以没有已写的实现会和新清单冲突。
2. **缺模型时返回 `ModelOrderUnspecified` 的机制**：语义已由 ADR 0033:12 裁定，输入域外的请求结果是 `ModelOrderUnspecified`/`Unsupported`，不回落旧遍历。#1340 C10.3（:346）也要求「无裁定时显式 ModelOrderUnspecified」。还没定的是：由哪一层产出，落成什么终态，怎么写进 `domain-event/1` 事件和报告，和 SPEC 的 `Unsupported`、`Denied/Aborted` 是什么关系。
3. **正式 launcher 的验证路径**：ADR 0039 裁定三已定唯一 launcher 是 `production_session`。当前基线没有这个文件，由 #1523 交付。还没定的是：FU-14「正式 launcher 可执行」用什么二值判据，运行证据由哪张票产出，影子工具算什么。

附带的待定项有：时钟（Q5）、数据源与范围（Q8）、缺口 bar（Q9）、故障域与调度轨迹的形状（Q12）。随机种子（Q6）、资金和持仓初值（Q13）、同一会话只有一个外部事实源（Q14）已经裁过，直接沿用。

**考古标出的冲突（三处，都不能归 A 类）**：

- **Q4**：#1528 旧正文（body :19）要「将 `replay_compare` CLI 从 `MinimalEngine` 替换为 `run_theta_v0_pi` 生产链」，而 ADR 0039:209 把 `theta_backtest`→`run_theta_v0_pi` 定为「冻结影子：π 批量对照臂，只出组件证据」。#1528 的头注（body :3）已经把它降为「具名组件/影子工具」，但旧正文没改。#1607（body :21）还在等 #1304 owner 决定。
- **Q8**：ADR 0020:145/:149 和 ADR 0023:11 定的决策源是美股 tick 全史（Massive 十个标的，2003-09-10 起）。另一边，#1304 评论 5596451156 写明「#1320 七标的十年 1min…重型测量额外义务单列保留」，而代码 `rust/src/theta_v0/backtest/data.rs:55-67` 只能加载后者的 1m json。正典数据的前置票 #1027 还开着，`.gitignore:82` 又排除了 `analysis/data_cache/*`，所以检出里没有任何 bar 文件。
- **Q9**：`data.rs:267` 写「坏 bar 用前一根 close 填充价格」，与 #1340 C02.1（:259）「时间聚合、重采样或执行市场价格不得偷偷改变结构级别语义」和 C02.4（:262）「数据缺口…各有准确结果」冲突。另外，ADR 0019:238 的旧臂在落点 `untradable` 时把 `fill_bar_index` 顺延，而 ADR 0033:31 规定撤改在下一边界生效。

**附：图结构风险（只登记，不改边）**：ADR 0039:284 把 `production_session` 骨架交给 #1523，`.chanlun/definitions/chong-traceability.md:50` 写的也是「由 #1523 交付」。但原生边是 #1366（TB-07 汇总）blocks #1523，而 #1381（TB-07-A）要在 `production_session` 上跑。如果再加 #1523 blocks #1381，就会出现 TB-07 等 #1523、#1523 又等 TB-07 的环。

## 权威依据

1. **分级与裁决权**：`.sandcastle/docs/graph-engineering.md:23`「T 类（技术取舍）：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决」；#1536 分级评论 [6452285266](https://github.com/xy7365527-lang/NewChanlun/issues/1536#issuecomment-6452285266)（xy7365527-lang，2026-10-27）判 T，并写明「资金等政策数值按 ADR 0041 只能落为 AwaitingPolicy 字段不得代填」。
2. **模型语义与版本合同**（`docs/adr/0033-history-simulation-bar-batch-model.md`）：
   - :5「本 ADR 只定模型组织、适用域与版本合同字段；实现、取证与 H1–H8 oracle 归 TB-07 叶票（#1381 等），不以本 ADR 关票声称实现完成」；
   - :12「输入域外的请求 ⟹ `ModelOrderUnspecified`/`Unsupported`，不回落旧遍历。v1 确定性，`seed: none`」；
   - :16 是 2026-10-26 用户批准 #1598 的批注；:18 给出 `model_id` 可选值，以及「`seed` 字段：v1 确定性模型取 `none`，随机模型版本取整数（非负 i64）」；
   - :22「`capacity_source ∈ {scenario_contract, bar_volume_derived}`，默认 `scenario_contract`」；
   - :31「旧单在当前 bar 继续担责并可能成交；bar 收盘即其终态累计 F 完整；改价新版本从 t+1 起」；
   - :37「完整历史 v1 默认 `projection_kind = per_leg`」；
   - :44「fill、fee、bust 各为独立原件…模拟 bust 只在 `scenario` 合同声明时产生。费率与角色来源由 `fee_profile` 引用，不填数」；
   - :48 版本合同字段：「`model_id/revision/content_hash`；`input_domain`；`clock`（bar 索引、`known_frontier_cut`）；`price_rules`…；`capacity_source`；`projection_kind`；`fee_profile`；`seed`；`unsupported_domains`；RD-11/RD-12 政策 id；调度轨迹」；
   - :56 旧臂清单（`fill.rs:4767`、`fill.rs:5044`、`strategy/exec.rs:145` 等）；
   - :64 下游票列表。
3. **双跑五元组与六面全状态**（`docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md`）：
   - :47「固定五元组（input, policy_version, model_id/revision, seed, scheduler）下，两次独立运行在每个 checkpoint 的全状态必须逐位等价」；
   - :49-54 六面：S 结构快照、各重 B 账、E 责任、订单/成交原件、物理持仓、外效历史；
   - :56「checkpoint 取每 bar 收盘后、下一 bar 开盘前，不取 bar 内中间态。v1 `seed: none` 确定性；引入随机化时另开版本」；
   - :60「launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar」；
   - :74「`evidence_class ∈ {simulation, experiment, production}` 作为 profile 版本字段…simulation 产出不进 TStage 触闸计数」；
   - :100「#1536（FU-14，完整历史 profile 须含 model_id/revision、seed、scheduler）」；
   - :109「保持：#1536（FU-14）blocks #1537（依赖既存）」。
4. **唯一 launcher 与最高测试缝**（`docs/adr/0039-production-entry-inventory-and-unique-launcher.md`）：
   - :209 `theta_backtest`→`run_theta_v0_pi`「冻结影子：π 批量对照臂，只出组件证据（C10.1）」；:212 另有 42 个无 required-features 的 bin，也是冻结影子；
   - :266-268「bin `production_session`，源文件 `rust/src/bin/production_session.rs`…全仓只有这一个 launcher。`theta_backtest`、`trading_system/live/runner.py`、`newchan` CLI、任何 `*_launcher.py` 都不是 launcher」；
   - :270「一份 manifest：已批准规则和政策版本、能力 manifest、原始输入来源、各域 DB 路径、时钟、adapter 选择（C10.1）」；:272「`evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合」；
   - :284 #1523 交付 `production_session` 与 `x_execution_session` 骨架；
   - :288「AT-01/02/03/16 以 `production_session` 拉起的独立进程为最高缝（C12.1）。直接调用 `s_structure_session` 等域 bin 只算组件证据」。
   - 定义文件 `.chanlun/definitions/chong-traceability.md`：:11「唯一正式 launcher 是 `production_session`…`run_theta_v0_pi` 不是正式 launcher 的替代链」；:50「该基线尚无这两个文件，也无 production_session Cargo 项，不能编造行号。ADR 0039:266–288 已指定由 #1523 交付，#1536、#1538、#1550 消费正式 launcher」；:58「八条既有原生 blocking 边全部保留…不删除边」；:63 是本票的承接行；:65 是 #1550 的承接行。
   - #1533 resolution [6428629112](https://github.com/xy7365527-lang/NewChanlun/issues/1533#issuecomment-6428629112)：「正式 launcher 为 `production_session`」。
5. **事件外壳与引用形状**（`docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md`）：
   - :172「任何一层都不含 wall-clock 或 PID」；
   - :244 `source = {source_namespace, source_epoch, external_fact_id, source_cursor, received_time}`；
   - :245 `policy_refs[]` 每项「`{policy_id, version, profile_id|null, content_hash}`」；
   - :246 `model_ref`「`{model_id, revision, content_hash}`；没有模型参与的域写 `null`」；
   - :248 `cause[]`「前件 `event_id` 或 `fact_key`」；
   - :284 snapshot 钉住 `{…policy_refs, model_ref}`；
   - :340「FU-14 的模型 profile 须使用本合同的 `model_ref` 形状」。
6. **SPEC #1340（已批准）**（行号按 `gh issue view 1340 --json body --jq .body` 的输出计）：
   - US-ENG-09（:244）「具名缺政策、缺证据、不可核和未支持的原因」；
   - C02.1（:259）「标准输入包含 source namespace/epoch、instrument、原始事件身份和修订、原始坐标、received_at、逻辑输入顺序、替代关系及来源证据。结构源遵守已裁美股 tick 边界；时间聚合、重采样或执行市场价格不得偷偷改变结构级别语义」；
   - C02.4（:262）「空输入、数据缺口、进行中、N/A、已撤回、未支持各有准确结果」；
   - C07.2（:306）「每个条件记录 ID、权威、依赖及失效源、检查阶段、证据覆盖、核验能力和失败结果；漏一项则 Unsupported，不能 Release 或执行」；
   - C07.4（:308）「S 对已交付固定命令在本地稳定点完成一次条件核验…撤回、替代、缺依赖、过期或不可核则持久最终 Denied/Aborted」；
   - C10.1（:344）唯一完整会话入口，「推进完整 S→操作→B→E→X→事实→B→观察」；
   - C10.2（:345）「只替换真实变化的 feed、clock、matcher/venue、传输与受控持久化 adapter。模拟 fill 不能与同会话真实 fill 同时充当权威」；
   - C10.3（:346）「新多重并发、混合订单和分配模型无裁定时显式 ModelOrderUnspecified，不能照旧单净单实现自行外推」；
   - C10.4（:347）「双跑固定原始输入及修订、政策/模型版本、时钟、随机种子、数值语义、故障与调度轨迹」；
   - C11.5（:358）「有仓、有尾账的迁移必须测试，不以空仓样例代替」；
   - C12.1（:363）「以最高完整会话入口为主测试缝」；
   - TB 表（:373-375）：TB-06/07/08 依赖「对应 FU 决定和模型」「完整历史模型」；
   - 测试决定（:436）「新增一个从正式 launcher 启动的完整会话测试夹具…样例必须实际产生其待测对象、动作和回报，不能以空域通过。具体样例数量服务于行为覆盖，不成为产品最低重数」；
   - 测试决定（:438）「故障注入覆盖持久提交前后、回报先后、丢包、重复、重启、writer 换代、账不可读和局部存储失败。运行报告保存…故障与调度序列」。
7. **词条**：
   - CONTEXT.md:376 已知前沿：「历史模拟中第 t 根 bar 适用的订单集合 ＝ t−1 收盘时已输出并经准入的集合；收盘才知道的信息只作用于下一边界」；
   - CONTEXT.md:372 证据类：「低类证据不得被读为高类有效」；
   - CONTEXT.md:451「完整历史模拟与它共用 `projection_kind` 的定义（ADR 0033）」；
   - ADR 0028:28「任一字段变更即新版本…`P0`（确定性回放与受控模拟，TB-04/TB-07 已承担）与 `P2`…是并列槽位」。
8. **资金与政策数值不代填**：
   - ADR 0041:15「可调字段不得有『主值失败时的备用值』」；
   - ADR 0041:21「签署门字段的默认值 ＝ `AwaitingPolicy`…未签署则该字段所阻的动作不跑，不回退任何备用值」；
   - ADR 0041:29-30 写合法候选保留、真错排除；
   - CONTEXT.md:66「数值由用户随激活签署，不设默认」；
   - CONTEXT.md:36「q_ref 绑定资金授权身份及版本，不能作算术量（ADR 0049）」；
   - ADR 0029:35 funding-record 形状；
   - ADR 0049:20「q_ref 定位 profile 的授权身份，另绑定 version」；
   - ADR 0040:34「每个 profile 只签总投入 `Q_total`：`P0` 100,000…」；
   - ADR 0040:57「量档记录 `q_ref` 指 funding-record 授权身份并绑定 version」。
9. **数据源正典**：
   - ADR 0020:145「回测与实盘同吃美股数据（历史全史 + 实时流）」；
   - ADR 0020:148「美股个股 tick 订阅立项（#1027）」；
   - ADR 0020:149「粒度 = tick 全史…标的集 = AAPL/AMZN/COST/GOOGL/LLY/META/MSFT/NFLX/NVDA/TSLA」；
   - ADR 0023:11 个股线（缠论信号）用 Massive Stocks，2003-09-10 起，tick；ADR 0023:12 的 K4 线（M/P/C/R 六边）用 Databento 期货 tick；
   - ADR 0023:15「EQUS.MINI 17.55GB（2023-03 起）已拉取完毕：定位 = 对拍/校验源，不作生产供给」；
   - ADR 0023:20「tick 直算为生产正典：逐笔成交 → `Bar(O=H=L=C=成交价，亚秒时间戳)`」；
   - AGENTS.md:165「不全量重放…全量仅留重型验证窗口并照实标注」。
10. **收敛通则与总缝规则**：AGENTS.md:48「同一个判断不得存在宽严两档实现，更不得让宽松的那档接管严格那档的失败」；AGENTS.md:66「同一个判定，每一级必须跑同一个判定」。
11. **影子工具的地位（#1304 系）**：
    - #1304 评论 [6245981746](https://github.com/xy7365527-lang/NewChanlun/issues/1304#issuecomment-6245981746)「replay_compare.rs:92 实际构造 MinimalEngine，产生空信号/固定3层零账；ReplayBook只有bar/snapshot计数与层名…因此这些是有限组件」；
    - #1304 评论 [5596451156](https://github.com/xy7365527-lang/NewChanlun/issues/1304#issuecomment-5596451156)「#1320七标的十年1min/冻结7/7重型测量额外义务单列保留，不能用新有界测试销项」；
    - ADR 0048:208 B-Q11「删除由 #1304 路径定」；
    - 代码 `rust/src/bin/replay_compare.rs:37-38`「最小化分类引擎（占位符，待接入真实引擎）/ struct MinimalEngine;」；`rust/src/trading/replay.rs:94-100` 中 ReplayBook 只有三个字段；
    - #1550 票面 :3-4「MinimalEngine/ReplayBook 不足以代完整回放…#1304 外壳如何调用完整生产程序（正式 launcher）」。
12. **诊断模式拒收义务**：#1381 评论 [6409073310](https://github.com/xy7365527-lang/NewChanlun/issues/1381#issuecomment-6409073310)「由正式完整历史回放入口真实拒收 diagnostic-fill-report/1 的 mode=diagnostic 结果及诊断配置…不能用冻结theta_backtest影子门或诊断CLI自拒收代替」。
13. **本票既有登记**：#1536 评论 [6404223988](https://github.com/xy7365527-lang/NewChanlun/issues/1536#issuecomment-6404223988)「正式路径为 production_session 拉起 S/E/各 Bᵢ/X，theta_backtest 只提供组件证据」；评论 [6391077284](https://github.com/xy7365527-lang/NewChanlun/issues/1536#issuecomment-6391077284)「dry-run 不算已裁定」。#1381 票面 :7「通过正式完整会话入口载入原始输入及修订、已批准规则/政策和具名模拟模型」。#1353 resolution [6149639079](https://github.com/xy7365527-lang/NewChanlun/issues/1353#issuecomment-6149639079)「输入域、价格规则、容量来源、投影方式、费用 profile、未支持域全部进版本合同，v1 确定性、无种子」。
14. **代码先例**：
    - `rust/src/bin/s_structure_session.rs:219-238` 的 `raw_events` 表，含 identity_key/revision/seq/source_namespace/source_epoch/instrument/event_id/received_at/source_coord/supersedes_revision；
    - `rust/src/economic_session/store.rs:44-53` 的域库表；
    - `rust/src/economic_session/tb03c_allocation.rs:73` 已消费 manifest 的 `projection_kind`；
    - `rust/src/bin/s_session_v3/atomic_init.rs:225` 的 `pub(super) mod fault {`，以及 `atomic_init_tests.rs:386`「#1666 L2/L3：发布阶段故障注入（`atomic_init::fault`，仅本线程生效）」；
    - 旧加载器 `rust/src/theta_v0/backtest/data.rs` 的两条判据：:266「缺 OHLC 或越界或 vol=0 ⇒ untradable」（协议 §1.2），以及 :282/:567「#922 坏 tick 闸：四价对前收（净化后）偏离 >80% ⟹ 前收填充 + untradable」；:267 是前收填充。:326 的 `#919：f64 直存` 只管 volume 怎么存，不是缺口判据。

## 候选

### Q1 六域输入 schema 的载体 `[新缠论:选择]`（B）

默认值是**新开 ADR 0052，定义 `history-input-profile/1`：一份 manifest 段，六个域各自引用既有形状，不另造字段**（出处：ADR 0033:48；ADR 0051:244-246；ADR 0038:49-54；#1340 C02.1 :259；ADR 0033:44；`s_structure_session.rs:219-238`）。请确认或否决。

推理链：
1. 六域各自都有正本形状，但没有合成清单。正本层只有 chong-traceability.md:63 一处提到六域。
2. 代码里零命中，没有已写的实现会冲突。
3. ADR 0039:270 要求 launcher 读入「一份 manifest」，其中有「原始输入来源」一项。六域清单就是这一项的展开。

比较过的候选：
- 在 ADR 0033 里追加一节：这是合法做法，作为替代保留。不选的原因是会把模型合同和输入合同混在同一份文件里。
- 落到 `.chanlun/definitions/`：排除。AGENTS.md:21 规定该目录是「缠论教义的唯一正本入口」，而六域 schema 没有缠论语义。

### Q2 `ModelOrderUnspecified` 的产出层与终态 `[新缠论:选择]`（B）

默认值有五点（出处：ADR 0033:12；#1340 C07.2 :306、C07.4 :308、C10.3 :346、US-ENG-09 :244；ADR 0051:246/:248）。请确认或否决。

- `ModelOrderUnspecified` 是 SPEC `Unsupported` 结果的一个具名 reason，不是第二种状态。
- 唯一判定点在 E-Release 之前的条件核，条件名是 `model_coverage`，权威是 profile 的 `input_domain`/`unsupported_domains`。
- 判为域外时，命令持久终态是 `Unsupported{reason: ModelOrderUnspecified, model_ref, domain_id, command_id}`。命令不 Release、不执行。
- 这个终态**不进** C07.4 的 Denied/Aborted 族。
- 它以 `domain-event/1` 外壳写入事件，`cause[]` 指向命令身份。

推理链：
1. ADR 0033:12 把结果写成 `ModelOrderUnspecified`/`Unsupported`。C07.2 规定条件缺项的结果是 `Unsupported`，并且「不能 Release 或执行」。US-ENG-09 要求把「未支持」和其他原因分开具名。所以合起来最小的形状就是「`Unsupported` + 具名 reason」。
2. C07.4 说的是「S 对已交付固定命令」的核验终态，作用在命令交付之后。缺模型的命令在 Release 之前就被拦下，不在 C07.4 管辖内。把它包进 `Denied`，等于把两种 SPEC 结果混成一种。
3. 判定只在一处，满足 AGENTS.md:66 的总缝规则。
4. AGENTS.md:48（#799）禁止兜底，所以这个结果不得触发 ADR 0033:56 列出的任何旧臂。

比较过的候选：在 X 的模拟 matcher 里判，然后返回 `Unsupported` 回报。排除，因为命令能到 X 就说明已经 Release，违反 C07.2「不能 Release 或执行」。

### Q4 正式 launcher 验证路径 `[新缠论:选择]`（B，测试锁）

默认值（出处：ADR 0039:288；#1340 C12.1 :363、测试决定 :436；#1381 票面 :7；ADR 0039:209/:212；#1304 评论 6245981746；#1550 票面 :3-4）——请确认或否决：

- 验证路径是 `production_session` 以 `evidence_class=simulation`（P0 槽位）装载 `history-input-profile/1`。
- 运行证据由 #1381（TB-07-A）按测试决定 :436 的夹具产出。
- 本票只定四条二值判据：
  - (a) 入口进程是 `production_session`；
  - (b) 六域输入都非空，并且各自产生对应对象；
  - (c) 域外请求产出裁定二的终态，且旧臂调用次数为 0；
  - (d) 影子 bin 只出组件证据。
- #1528/#1607 的去向不在本票裁，登记交给 #1550。

推理链：
1. 最高测试缝已经裁过，是 `production_session` 拉起的独立进程。
2. #1381 的第一条验收就是「通过正式完整会话入口载入原始输入」，运行证据应归它。
3. ADR 0033:5 规定「实现、取证…归 TB-07 叶票，不以本 ADR 关票声称实现完成」，所以本票不承担运行。
4. #1528 旧正文和 ADR 0039:209 冲突，这是 #1304 接入映射的问题，属于 #1550。

比较过的候选：本票自己开一个运行夹具。排除，依据是 ADR 0033:5 的先例，以及 chong-traceability.md:50「不能编造行号」。

### Q5 时钟 `[新缠论:推论]`（B）

默认值是 **`clock = {kind: "logical-bar/1", bar_index, bar_close_ts, known_frontier_cut}`**。`bar_close_ts` 取原件时间戳。在 simulation 证据类下，S/B/E/X 一律不读系统时钟，`received_at` 直接取原件值；checkpoint 和 seek 只认 `bar_index`；tick 源按 ADR 0023:20 退化成 `Bar(O=H=L=C)` 后用同样的索引（出处：ADR 0033:48；ADR 0038:56/:60；ADR 0051:172；CONTEXT.md:376；ADR 0023:20）。请确认或否决。

推理链：这条由 ADR 0033 裁定一（已知前沿，CONTEXT.md:376）和 ADR 0038:56（checkpoint 在 bar 收盘）推出：只要有 wall-clock 读数进入判定，五元组就无法逐位复现。ADR 0023:20 已把 tick 定义成退化的 bar，所以「第 t 根 bar」在 tick 源上同样成立。这是已裁推论的组合，不设可选值。

### Q8 数据源与范围 `[新缠论:选择]`（B，ADR 0041 自由度）

默认值有三点（出处：ADR 0020:145/:148/:149；ADR 0023:11/:12/:15/:20；#1340 C02.1 :259、测试决定 :436；AGENTS.md:165；#1304 评论 5596451156）。请确认或否决。

- 可调字段 `input_source`：
  - 默认 `massive-stock-tick/1`，这是正典；
  - 可选值 `databento-equs-mini-tick/1`。代价是它在 ADR 0023:15 里的定位是对拍/校验源，报告必须标「校验源」。
- `input_range{symbol[], from, to}` 由各验收票按行为覆盖来填，FU-14 不钉数字。
- 全量只在重型验证窗口跑。

推理链：
1. 正典已经裁过。前置 #1027 还开着，所以默认值现在拿不到。选中时入口返回 `Unsupported{reason: InputSourceUnavailable}`，不伪造，也不换到另一个值。要换值只能在运行前改签新版本（ADR 0041:15）。
2. EQUS.MINI 是美股 tick，满足 C02.1 的边界，而且已经拉取完毕，可以作为合法可选值保留。
3. `databento-futures-1m/10y`（期货 1m）和 K4 线的 Databento 期货 tick（ADR 0023:12 定为 M/P/C/R 用途）用作结构源，都违反 C02.1「结构源遵守已裁美股 tick 边界」。按 ADR 0041:30「真正错误…照旧排除」，两者不进可选值集合。
4. #1320 的重型义务按 #1304 评论 5596451156 留在原路径单列，不借本票销项。

### Q9 缺口与坏 bar `[新缠论:选择]`（B）

默认值是**固定合同 `gap_handling = preserve-gap/1`**，内容如下（出处：#1340 C02.1 :259、C02.4 :262；`data.rs:266`/:267/:282/:567；ADR 0019:238；ADR 0033:31）。请确认或否决。

- 原件层逐条保留缺口事件，`kind` 取 `gap` 或 `untradable`。
- 每条带判据 id：untradable 判据对应 `data.rs:266`「缺 OHLC 或越界或 vol=0」，坏 tick 判据对应 `data.rs:282/:567` 的 #922 坏 tick 闸。
- 不生成填充价。
- S 对缺口输出 C02.4 的「数据缺口」结果，并保持前沿不变。
- 执行域在这类 bar 上不判成交，也不顺延；订单按 ADR 0033:31 进入下一个有效边界。

推理链：
1. C02.4 要求缺口要有准确结果。
2. 旧加载器「保序列、标 untradable、不在其上成交」这一半和 C02.4 同向，保留。
3. 前收填充（`data.rs:267`，以及 :567 的「前收填充 + untradable」）会造出不存在的价格送进结构域，正面违反 C02.1。按 ADR 0041:30 排除，不作为可选值；它只留在冻结影子臂（ADR 0039:209）。
4. ADR 0019:238 的顺延是旧单净单臂的行为，新模型按 ADR 0033:31 执行，不另设顺延档。

### Q12 故障域与调度轨迹 `[新缠论:选择]`（B）

默认值（出处：#1340 测试决定 :438、C10.4 :347、C10.1 :344；ADR 0033:44/:48；ADR 0038:47；#1381 评论 6409073310；`atomic_init.rs:225`、`atomic_init_tests.rs:386`）——请确认或否决：

- scenario 合同增加 `fault_schedule[]`，每项为 `{at: {bar_index | cut}, domain ∈ {S, E, B_i, X, transport, store}, kind}`；
- `kind ∈ {crash_before_commit, crash_after_commit, reorder_reports, drop, duplicate, restart, writer_epoch_bump, store_unreadable, store_write_failure}`；
- `scheduler = "launcher-sched/sequential/1"`；
- 运行时写「调度轨迹」原件，它和 `fault_schedule` 一起进入 `content_hash` 与五元组；
- `mode=diagnostic` 的输入由入口以 `Unsupported{reason: DiagnosticNotProduction}` 拒收，不计入 `ModelOrderUnspecified`。

推理链：
1. SPEC 把故障和调度序列列为报告必存项，ADR 0038:47 把 scheduler 列进五元组，但正本没有给出字段形状。
2. ADR 0033:44 已经立下「模拟 bust 只在 scenario 合同声明时产生」，推广到所有故障种类，就是故障只来自声明、不来自随机。
3. S 域代码已有线程局部的故障注入钩子，说明机制可以落地。
4. 诊断模式的拒收原因和缺模型不同，两者不能混。

**SPEC :438 八项与 `kind` 的确切对应**：

| SPEC :438 项 | `kind` |
|---|---|
| 持久提交前后 | `crash_before_commit`、`crash_after_commit` |
| 回报先后 | `reorder_reports` |
| 丢包 | `drop` |
| 重复 | `duplicate` |
| 重启 | `restart` |
| writer 换代 | `writer_epoch_bump` |
| 账不可读 | `store_unreadable` |
| 局部存储失败 | `store_write_failure` |

八项全覆盖，其中「持久提交前后」拆成两个值，所以共 9 个值。

### 已裁沿用（A 类，不列为待决）

- **Q3**：正式 launcher 是 `production_session`（ADR 0039:266-268；chong-traceability.md:11/:50；#1533 resolution 6428629112）。
- **Q6**：随机种子。v1 为 `seed: none`；随机模型版本取非负 i64；五元组固定 seed（ADR 0033:18；ADR 0038:56；#1353 resolution 6149639079）。
- **Q7**：订单模型 profile。`model_id` 可选值、`capacity_source` 默认 `scenario_contract`、`projection_kind` 默认 `per_leg`、`fee_profile` 只引用不填数、撤改从 t+1 生效（ADR 0033:18/:22/:31/:37/:44；CONTEXT.md:451；`tb03c_allocation.rs:73`）。
- **Q10**：MinimalEngine/ReplayBook 是有限组件，去向归 #1550（#1304 评论 6245981746；ADR 0048:208；chong-traceability.md:63/:65；#1550 票面 :3-4）。
- **Q11**：`model_ref` 与 `policy_refs[]` 的形状（ADR 0051:245-246/:284/:340；ADR 0038:100）。
- **Q13**：资金只用 `policy_refs` 引用 funding-record。P0 已签 `Q_total`（ADR 0040:34）；未签字段为 `AwaitingPolicy`（ADR 0041:21）；带仓迁移归 TB-10（C11.5 :358）。本票不代填任何数值。
- **Q14**：历史 profile 是 P0 / `evidence_class=simulation`，唯一外部事实源由 adapter 装配锁定（#1340 C10.2 :345；ADR 0028:28；ADR 0038:74；ADR 0039:272；CONTEXT.md:372）。

## 裁定

本条依 #1323 Notes N-1 预授权判定。上述默认值全部采纳，分七条落为一份新 ADR。

正本落点：docs/adr/0052-history-input-profile-and-model-order-unspecified.md

### 一、`history-input-profile/1`：六域引用清单 `[新缠论:选择]`

ADR 0052 定义 `history-input-profile/1`。它是 ADR 0039:270 manifest 中「原始输入来源」一段的展开，也是 ADR 0033:48 版本合同的合成载体。各字段只引用既有形状：

| 域 | 字段 | 形状来源 |
|---|---|---|
| 结构 | `structure.source_ref` 指向原件集合，每条原件 = C02.1 字段：`source_namespace, source_epoch, instrument, event_id, revision, source_coord, received_at, seq, supersedes_revision`，外加来源证据 | #1340 C02.1 :259；`s_structure_session.rs:219-238`；ADR 0051:244 |
| 资金 | `funding.policy_refs[]`，每项 `{policy_id, version, profile_id\|null, content_hash}`，指向 funding-record 和量档记录。**profile 里不出现任何金额、cap 或量档数值**；未签字段写 `AwaitingPolicy{<字段>}` | ADR 0051:245；ADR 0029:35；ADR 0040:34/:57；ADR 0049:20；ADR 0041:21 |
| 订单/成交 | `model` = ADR 0033:48 的全部版本合同字段，加 `model_ref{model_id, revision, content_hash}`；`input_domain` 与 `unsupported_domains` 必填且互斥 | ADR 0033:48；ADR 0051:246 |
| 持仓 | `initial_positions[]` 默认空集；非空只允许 TB-10 迁移用例引用 crosswalk 记录 id，不接受裸数量 | ADR 0038:53；#1340 C11.5 :358 |
| 故障 | `scenario_contract = {capacity(ref), bust[], fault_schedule[], scheduler}`，见裁定五 | ADR 0033:44；#1340 :438 |
| 时钟 | `clock = {kind: "logical-bar/1", bar_index, bar_close_ts, known_frontier_cut}`，见裁定四 | ADR 0033:48；ADR 0038:56 |

profile 另含 `evidence_class`（本 profile 固定为 `simulation`，P0 槽位）、`input_source`、`input_range`、`gap_handling`（见裁定六）。整份 profile 经 JCS 规范化后取 `content_hash`，它和原件序列哈希一起构成 ADR 0038:47 五元组里的 input 项。任何字段变更都是新版本（ADR 0028:28 同款）。

### 二、`ModelOrderUnspecified` 的状态机与事件 `[新缠论:选择]`

1. `ModelOrderUnspecified` 是 SPEC `Unsupported` 结果（C07.2 :306）的具名 reason，不是独立状态。
2. 唯一判定点在 E-Release 之前的条件核，条件名为 `model_coverage`。核法：拿命令的订单种类、并发形态和分配模型去对照 profile 的 `input_domain`；落在 `unsupported_domains` 里，或者两边都不包含，就判为域外。域外时命令持久终态为 `Unsupported{reason: ModelOrderUnspecified, model_ref, domain_id, command_id}`，命令不 Release、不执行（C07.2）。
3. 这个终态**不进** C07.4（:308）的 Denied/Aborted 族。C07.4 只管 S 对已交付固定命令的核验结果；两类原因按 US-ENG-09（:244）分开具名。
4. 终态以 `domain-event/1` 外壳写入 E 域事件：`cause[]` 指向命令身份，`model_ref` 填被对照的模型。不产生任何 fill、部分 fill、fee 或 bust 原件，也不调用 ADR 0033:56 列出的任何旧臂。
5. X 的模拟 matcher 只做一个不变式断言：收到的命令在 `input_domain` 内。断言失败就是硬错误，进程停止。它不判成交、不回落，也不算第二个判定点。
6. #1418 的报告单列 `ModelOrderUnspecified` 计数。双跑比较面（ADR 0038:49-54 的第 3、4 面）包含这个终态事件。
7. `mode=diagnostic` 的输入由入口以 `Unsupported{reason: DiagnosticNotProduction}` 拒收（#1381 评论 6409073310）。这是另一个 reason，两者不互相计入。

### 三、正式 launcher 验证路径（测试锁） `[新缠论:选择]`

1. 验证入口只有 `production_session`（ADR 0039:266-268）。它以 `evidence_class=simulation` 装配受控撮合 adapter（ADR 0039:272），并装载 `history-input-profile/1`。
2. 运行证据由 #1381 按测试决定 :436 的完整会话夹具产出。本票不编造 `production_session.rs` 的行号（chong-traceability.md:50）。#1523 交付之前，这条路径处于「等前置」状态，不用影子工具顶替。
3. FU-14 的判据有四条，都是二值：
   - (a) 进程树根是 `production_session`，S/E/各 Bᵢ/X 是它的子进程；
   - (b) 六域输入都非空，且各自产生对应对象：结构对象、资金引用解析结果、命令、模拟 fill 原件、持仓投影、按 `fault_schedule` 触发的故障记录；
   - (c) 域外请求产出裁定二的终态，且旧臂调用次数为 0；
   - (d) `theta_backtest`/`replay_compare`/`theta_replay` 的产出只标为「组件证据」，不进入 FU-14 判据。
4. #1528 旧正文（要接 `run_theta_v0_pi` 生产链）和 #1607 比较器的去向，登记交给 #1550 裁决。

### 四、逻辑时钟 `[新缠论:推论]`

`clock.kind = "logical-bar/1"`。在 `simulation` 下，S/B/E/X 不读系统时钟，`received_at` 取原件值。checkpoint、seek 和 `known_frontier_cut` 只认 `bar_index`（ADR 0038:56/:60；CONTEXT.md:376）。tick 源按 ADR 0023:20 退化成 `Bar(O=H=L=C)` 后，用同样的方式索引。本条是已裁推论的组合，不设可选值。

### 五、故障域与调度轨迹 `[新缠论:选择]`

1. `scenario_contract.fault_schedule[]` 每项为 `{at: {bar_index | cut}, domain ∈ {S, E, B_i, X, transport, store}, kind}`，其中 `kind ∈ {crash_before_commit, crash_after_commit, reorder_reports, drop, duplicate, restart, writer_epoch_bump, store_unreadable, store_write_failure}`。它和 SPEC :438 八项的确切对应见「候选」Q12 的映射表。故障只来自声明，不来自随机（由 ADR 0033:44 推广）。
2. `scheduler = "launcher-sched/sequential/1"`：每根 bar 按固定次序 S→操作→B→E→X→事实→B→观察 执行（C10.1 :344）。换次序就是新的 scheduler 版本，运行中不切换。
3. 运行时写「调度轨迹」原件，内容是实际执行次序和实际触发的 `fault_schedule` 项。它和 `fault_schedule`、`scheduler` 一起进入 `content_hash` 与五元组（ADR 0038:47）。

### 六、数据源、范围与缺口 `[新缠论:选择]`

1. `input_source` 是可调字段（ADR 0041）：
   - 默认 `massive-stock-tick/1`（ADR 0020:145/:149、ADR 0023:11）。前置 #1027 未交付时，入口返回 `Unsupported{reason: InputSourceUnavailable}`，不伪造，也不在运行中换值（ADR 0041:15）。
   - 可选值 `databento-equs-mini-tick/1`（ADR 0023:15）。代价是它属于校验源，报告必须标「校验源」。改用它要在运行前改签版本。
   - 期货 1m 和 K4 线的期货 tick 用作结构源时违反 C02.1，排除。
   - #1320 的重型义务按 #1304 评论 5596451156 单列保留，不进本 profile。
2. `input_range{symbol[], from, to}` 由各验收票按行为覆盖来填（:436），FU-14 不钉数字。全量只按 AGENTS.md:165 在重型验证窗口跑，并照实标注。
3. `gap_handling` 固定为 `preserve-gap/1`，具体做法：
   - 缺口和坏 bar 逐条保留为 `kind: gap|untradable` 原件，带判据 id（untradable 判据见 `data.rs:266`，#922 坏 tick 闸见 `data.rs:282/:567`）；
   - 不生成填充价；
   - S 输出 C02.4 的「数据缺口」结果；
   - 执行域在这类 bar 上不判成交、不顺延，订单按 ADR 0033:31 进入下一个有效边界；
   - 前收填充依据 C02.1 排除。

### 七、沿用 A 类

裁定一至六之外，`model_id`、`seed`、`capacity_source`、`projection_kind`、`fee_profile`、撤改边界、`model_ref`、`policy_refs`、P0 证据类、唯一外部事实源、资金不代填，全部按「候选」段「已裁沿用」所列出处执行。本 ADR 不重写这些条款，也不改值。

## 验收清单

（`<ADR>` 代表 `docs/adr/0052-history-input-profile-and-model-order-unspecified.md`，NNNN 换成落地时分配的编号。本票只验文书和票状态；运行类测试是 #1381/#1411/#1418 的验收，见下一段。）

- [ ] 文件存在：`test -f <ADR>` 退出码 0。
- [ ] `rg -F -c 'history-input-profile/1' <ADR>` 输出 ≥ 1。
- [ ] `rg -F -c 'Unsupported{reason: ModelOrderUnspecified' <ADR>` 输出 ≥ 1。
- [ ] `rg -c 'Denied\{reason: Unsupported' <ADR>` 无输出，且退出码为 1（断言缺模型终态没有被包进 Denied）。
- [ ] `for t in fault_schedule store_write_failure restart launcher-sched/sequential/1 logical-bar/1 preserve-gap/1 input_source InputSourceUnavailable DiagnosticNotProduction; do rg -F -q "$t" <ADR> || echo "MISSING $t"; done` 输出为空。
- [ ] `rg -c '^正本落点：docs/adr/' <ADR>` 输出 = 1。
- [ ] 票状态：`gh issue view 1536 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1536/comments --paginate --jq '[.[]|select(.body|startswith("本条依 #1323 Notes N-1 预授权判定"))]|length'` 输出 ≥ 1。
- [ ] 票状态：`gh issue view 1381 -R xy7365527-lang/NewChanlun --comments | rg -F -c 'history-input-profile'` 输出 ≥ 1。落地时在 #1381 发承接登记评论；当前值为 0。
- [ ] 票状态：`gh issue view 1550 -R xy7365527-lang/NewChanlun --comments | rg -F -c '#1528'` 输出 ≥ 1，且 `… | rg -F -c '#1607'` 输出 ≥ 1。落地时在 #1550 发冲突登记评论；当前两项都是 0。
- [ ] 边不删：`gh api repos/xy7365527-lang/NewChanlun/issues/1536/dependencies/blocking --jq 'map(.number)|sort|join(",")'` 输出 `1364,1366,1367,1537,1538,1550`。

## 受影响票与 blocking 边变化

**受影响票**（列出的 cargo 测试是该票自己的验收；判据统一为 `cargo test <完整路径> -- --exact` 的输出含 `1 passed`，测试名在实施时固定。这些测试不是 #1536 的关票条件）

- **#1381**（TB-07-A，OPEN）：实装 profile 装载、`model_coverage` 条件、`Unsupported{reason: ModelOrderUnspecified}` 终态、`DiagnosticNotProduction` 拒收、`preserve-gap/1`、`fault_schedule` 触发，并产出运行证据。建议的测试名：
  - `history_profile_six_domains_nonempty`
  - `model_order_unspecified_unsupported_no_fallback`
  - `diagnostic_mode_rejected_at_launcher`
  - `gap_bar_preserved_no_fill_no_defer`
- **#1399**（TB-07-B）：seek 只认 `clock.bar_index`（裁定四）。
- **#1411**（TB-07-C）：五元组的 input 项 = profile `content_hash` + 原件序列哈希；调度轨迹进入比较面（裁定五）。建议的测试名：`fault_schedule_trace_in_content_hash`。
- **#1418**（TB-07-D）：报告单列 `ModelOrderUnspecified`、`DiagnosticNotProduction`、`InputSourceUnavailable` 计数，以及「校验源」标注。
- **#1397/#1398**（TB-06-H/I）：scenario 合同里 `capacity`/`bust` 的字段形状按裁定一的表格。
- **#1382**（TB-08-A）：历史模式和 paper 模式共用 `history-input-profile/1` 的六域清单，只换 adapter（C10.2）。
- **#1538**（FU-16）：消费裁定一和裁定三 (d)。
- **#1550**（FU-19）：接收裁定三第 4 条的登记，裁决 #1528 旧正文与 ADR 0039:209 的冲突，以及 #1607 的去向。
- **#1523**：交付的 `production_session` 必须接受 ADR 0039:270 manifest 中的 `history-input-profile/1` 段。
- **#1027**（OPEN）：它是裁定六默认值的前置事实，本票不改它的边。
- **#1320、#1304**：七期货十年 1min 的重型义务单列保留，不借本票销项。
- **#1528、#1607**：登记到 #1550，本票不改它们的边。
- **#1366、#1364、#1367**：被本票阻塞的汇总票，关票后这些边自然满足。
- **#1340**（SPEC）：由本 ADR 把相应条款落成字段，SPEC 本身不改。

**blocking 边变化**

- 不删边，关票后自然满足。依据是 chong-traceability.md:58 的同款先例「不删除边」。涉及的边：#1536 blocks #1364（TB-06）、#1366（TB-07）、#1367（TB-08）、#1538（FU-16）、#1550（FU-19）。
- 已失效，不改：#1536 blocks #1537（FU-15 已 CLOSED；对应 ADR 0038:109）。
- 已解除，不改：#1533、#1534 blocks #1536（两票都已 CLOSED）。
- **不新增边**：
  - 不加 #1523 → #1381。原生边已有 #1366 blocks #1523，而 #1381 是 TB-07 的子片，再加这条会构成排程环。`production_session` 骨架什么时候可供 #1381 使用，作为图结构风险上报编排者，在 #1550 一并登记。
  - 不加 #1027 → #1381。可选值 `databento-equs-mini-tick/1` 已经给了合法通路，不把一张手工订阅票变成硬门。
  - #1528/#1607 不接到本票。
- 不需要开 research 或 probe 票：Q8 的数据可得性是既有任务 #1027 的交付事实，不是新的缺口。

## 授权引用

本条依 #1323 Notes N-1 预授权判定。用户 2026-10-18 原话见 #1323 评论 [6261281111](https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111)（标记 `notes-N-1-20261019`）：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」。#1323 body :54 登记了 Notes N-1 条目；流程正本 `.sandcastle/docs/graph-engineering.md:90` 有同一授权记录。分级依据是 #1536 评论 6452285266（T 类）。
