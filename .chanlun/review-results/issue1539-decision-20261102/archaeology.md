# #1539 考古（41F 修订＋41P 同义订正，以 fu17-repair40r 为底）

[grilling] FU-17 成交/持仓唯一真账与 crosswalk（#1323，新增）

归类：A 已裁 8 · B 有先例未正式裁 5 · C 确未设计 2。只有 C 类列为真正待决；B 类以「默认值，确认或否决」呈现；A 类直接沿用。

**取证基线**：仓内查询与命中在 main `9de18e606b2c0330162a83e2a030898b46c3690f` 的冻结树（`/Volumes/AgentStorage/fu17-repair40r/frozen`）上执行。GitHub 正文与评论为 `/Volumes/AgentStorage/fu17-repair40r/gh/` 快照；#1411 正文与 #1366/#1400/#1411/#1425/#1382/#1367 的依赖边为本轮 live 只读快照（`/Volumes/AgentStorage/fu17-repair41f/gh/`）。tracker 计数沿用 40R 读数（2026-10-31T23:50:54Z–2026-10-31T23:51:46Z）；本轮新增仓内查询复跑于 2026-11-01T00:12:03Z–2026-11-01T00:12:04Z。「首版」列为 `/Volumes/AgentStorage/decisions/1539/archaeology.json`。

**本版改动**：只改 40Y 独审 B1–B4 涉及的 Q4、Q9、Q10、Q12、Q13；新增查询与命中标「41F 新增」，各点「首版订正」末尾追加「【40Y 订正】」条目。其余各点与 40R 逐字相同。生成脚本 `work/build_archaeology_41f.py`，变更数据 `work/changes_41f.json`。

## Q1 [A] 「每个 fill/position 一个权威写者」的实现机制是什么——靠什么让第二个写者物理上写不进去？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 权威写者 | `gh search issues "权威写者" --repo xy7365527-lang/NewChanlun --limit 100` | 98 | 100 |
| repo | writer_epoch|writer.lock | `rg -n -i 'writer_epoch\|writer\.lock' rust/src` | 712 | 713 |
| repo | 权威写者|单写者|唯一写者 | `rg -n '权威写者\|单写者\|唯一写者\|single.writer' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 2 | 2 |

命中（原文逐条核验）：
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:300：1. **持久域层**：S/E/B/X 每个域一个库、一个 `.writer.lock`（`session_transport.rs:53`）。第二个写者进程取锁失败时，以 `StaleWriter：<域>独占进程锁不可得` 退出（`:62`）。持久 `writer_…
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:282：- 以独立 OS 进程启动 S、E、X、各 Bᵢ（C09.1）；每个域有自己的 `--db`、`.writer.lock`、`writer_epoch`。
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:290：6. **换代**：先确认旧 X 和它的发送子进程已退出，再提升 `writer_epoch`、取得排他进程锁（C09.8）。旧 epoch 的写入被 `store.rs:466-467` 拒绝。
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:40：| 受控持久化 | 各域单写者 | 同一 SQLite/WAL 实现 | 同一实现 | 只换 DB 路径；启动前件不满足即拒写（C09.2 :331）；锁与 epoch 见 ADR 0039:292、`stor…
- #1340 正文 :331：2. 每个域一份独立 SQLite 数据库和 WAL，由该域单写者进行有限事务；不使用 ATTACH 跨域原子提交，不以共享连接或统一事务管理器串起 S/E/B。所有权威提交采用 WAL + synchronous=FULL，macOS 开启 fullfsy…
- #1340 正文 :336：7. S/E/B 的写事务核数据库持久 writer_epoch；事务写入与 epoch 更替同序。X 是唯一持有场所写凭据并执行网络请求的进程；其他域和旧脚本在获准部署中无该凭据。X 的适配器在 X 内部受控调用，不能委托另一个仍有直发权的异步后台；SDK 对写请求的隐式重试、后台提交及恢复重投必须逐层核实并纳入一次调用…
- rust/src/session_transport.rs:56：let p = PathBuf::from(format!("{}.writer.lock", canonical.display()));
- rust/src/session_transport.rs:65：.map_err(|e| format!("StaleWriter：{domain_name}独占进程锁不可得：{e}"))?;
- rust/src/economic_session/store.rs:657-659：return Err("StaleWriter：持久epoch不等于获准写者".into());

- 出处：ADR 0039 裁定三·3（:282）、裁定三·6（:290）与裁定四·1（:300）；ADR 0056 裁定三「受控持久化｜各域单写者」（:40）；#1340 C09.2（:331）/C09.7（:336）；代码 session_transport.rs:56/:65、store.rs:657-659。机制已裁、已落码，本票只引用，不重裁。
- 冲突：正本内代码指针漂移（只登记，不改）：ADR 0039:300 写 `session_transport.rs:53`/`:62`、`store.rs:466-467`，ADR 0056:40 写 `ADR 0039:292`、`store.rs:653`；冻结 head 上实际为 session_transport.rs:56/:65、store.rs:657-659、ADR 0039:290/:300。
- 首版订正：
  - 首版命中写 ADR 0039:298-299，:298 是标题、:299 是空行，正文在 :300（两轮审查 R8 均指出）。

## Q2 [A] fill 原件（成交事实）的权威写者是哪个域？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 场所回报 E 原件 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "场所回报 E 原件 in:comments"` | 55 | 58 |
| repo | fact_key | `rg -n 'fact_key' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 29 | 29 |
| repo | fact_key | `rg -n 'fact_key' rust/src` | 492 | 492 |

命中（原文逐条核验）：
- .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:35：…持久可读 blob、可信连接身份/范围、token、source epoch/cursor、received 时间 | 同事务写 Recorded 和保守 Pending；随后才可解释风险/用于经营 | 未规范接纳的缓存不算 Recorded；字节不可读保持 Pending，不能只留无实体 hash |
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:46：E.CaptureExternal（INTERFACE-CONTRACTS:35）对三模式只认一份原件 schema：ADR 0051:244 的 `source`，加 `book.rs:117-124` 的必填项（`fact_key` 在 `:124`，定义在 `:93`）。`format`…
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:310：1. **真相**（append-only）：`facts / applications / commits` 根链，以及 S 的 `raw_events / structure_deltas`。
- rust/src/economic_session/book.rs:93-99：required(s, "external_fact_id")?
- #1340 正文 :295：2. 责任增加先在 E 持久保留，再允许对应本地和外部生效；减少须具备该责任不可再执行/累计完整性的证据及其参与账的持久应用回执。回执集合仅含实际关联账，不扩大为全批次或所有重。
- #1770 正文 :34：…。这两个关联字段是 X 自己的调用身份（INTERFACE-CONTRACTS :60），不是 E 或 B 的归属。记录不含 `allocation_id`、`family_id`、`book_identity`，不判定归属。E 目前只接受 TestOnly 经济原件 profile（`book.rs:106-107`），原件还须带归属字段（`:117-123`），所以 C-API 的观测记录不是 E…

- 出处：fill 原件唯一写者＝E（E.CaptureExternal 同事务 Recorded+Pending，INTERFACE-CONTRACTS:35；`facts` append-only 为真相，ADR 0051:310 裁定五）；去重键 `fact_key=hash(source_namespace,source_epoch,external_fact_id)`（book.rs:93-99；ADR 0051 裁定二·1）。X 的调用/回报观测记录是第 6 面 journal，不是 E 原件（#1770:34）。
- 首版订正：
  - 首版命中写 ADR 0051:309（「持久边界」节下空行），「真相」条在 :310。

## Q3 [A] 逐重 position（持仓、毛账、成本）的权威写者是哪个域？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 持仓真账 | `gh search issues "持仓真账" --repo xy7365527-lang/NewChanlun --limit 100` | 100 | 100 |
| repo | 毛账 逐重归属 | `rg -n '毛账' .chanlun/definitions/chong-traceability.md CONTEXT.md docs/adr/0024-total-ledger-convention.md` | 12 | 14 |
| repo | ApplyAllocation|applications | `rg -n 'applications\(\|ApplyAllocation' rust/src/economic_session/store.rs .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md` | 3 | 4 |

命中（原文逐条核验）：
- .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:37：…fact/allocation_id、归属重/Voice/family、数量/现金/费用/更正关系及已批分配政策 | 本地唯一应用并返回持久收据；真实经济事件驱动毛账/成本/阶段 | 重复同内容无变化；异内容冲突；缺前件/配对保留未匹配，不归新 Voice |
- #1340 正文 :338：…corded/Pending、证据及源前沿、责任集合、Released 和对应 S/X 状态，不能从旧 Open 恢复。B 恢复重放原归属应用，不从场所净仓重建成本。S 恢复未完成修订时保持相应条件门关闭，自己的恢复不等待 E。
- .chanlun/definitions/chong-traceability.md:35：逐重毛账按腿记录，不以净额代替。毛暴露是两腿绝对值之和，子对冲额度受父级短差额度约束。零筹码期间仍保留金额账和阶段事件史。依据为 `CONTEXT.md:426、430`、ADR 0027:26 与三阶段正本…
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:311：2. **可重算投影**：book image、`growth_eligible`、毛账合计。可以物化，但审计时必须等于重放结果。
- rust/src/economic_session/store.rs:75：CREATE TABLE applications(k TEXT PRIMARY KEY,fact_key TEXT NOT NULL UNIQUE,json TEXT NOT NULL) STRICT;
- docs/adr/0024-total-ledger-convention.md:19-20：- **验收用净额账**——执行层按什么成交就按什么验收，账实一致；

- 出处：每重 position 唯一写者＝该重 Bᵢ（B.ApplyAllocation 唯一应用，INTERFACE-CONTRACTS:37；`applications.fact_key UNIQUE`，store.rs:75；ADR 0039:282 各 Bᵢ 独立进程/库）；book image 是可重算投影（ADR 0051:311），不是第二本账；C09.9（#1340:338）禁从场所净仓重建；ADR 0024:19-20 净额账验收、毛账归因两套并记，写者仍是 B。
- 首版订正：
  - 首版命中写 chong-traceability.md:33，该行是 Voice 标识句；「逐重毛账按腿记录」在 :35。
  - 首版命中写 ADR 0051:310 为「可重算投影」，实际 :310 是「真相」，「可重算投影」在 :311。
  - 首版出处写 store.rs:49 `applications.fact_key UNIQUE`，实际在 :75（:49 不是该表）。
  - 首版「毛账」查询记 12 条；冻结 head 复跑为 14 条（相关三个文件在 457902e3c3 与 9de18e60 间无差异，首版计数本身有误）。

## Q4 [C] 「物理持仓」（ADR 0038 第 5 面：场所回报与内部投影对账）由谁写、是否构成独立真账？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 物理持仓 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "物理持仓 in:comments"` | 117 | 119 |
| repo | 物理持仓|场所净仓|账实 | `rg -n '物理持仓\|场所净仓\|账实' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 6 | 6 |
| repo | net_position|physical_position | `rg -n -i 'physical_position\|venue_position\|net_position\|reconcile_position' rust/src trading_system scripts` | 57 | 57 |
| repo | 换说法：对账|reconcil（正本层） | `rg -n '对账\|reconcil' docs/adr CONTEXT.md .chanlun/definitions` | —（本版新增） | 24 |

命中（原文逐条核验）：
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:53：5. 物理持仓（场所回报与内部投影对账）；
- #1340 正文 :300：…累计与逐笔报使用同一规范化合同。缺来源身份或无法区别同形事实时保留冲突，不能按内容去重误吞真实成交。更正按前件因果应用，账实快照不能直接覆写私账。
- #1340 正文 :338：…ng、证据及源前沿、责任集合、Released 和对应 S/X 状态，不能从旧 Open 恢复。B 恢复重放原归属应用，不从场所净仓重建成本。S 恢复未完成修订时保持相应条件门关闭，自己的恢复不等待 E。
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:125：…er/venue 行（查询走 X 内场所 adapter，X 是唯一执行网络请求的进程，C09.7 :336）、裁定四（查询结果以「场所 adapter 原件」进 E）、裁定六（单一外部事实源）；能力证据形状沿用 C10.5（:348）。本票不改其 AC；它已经 #1382、#1366 被本票传递阻塞，不加直接边。建议测试：`venue_rea…
- #1400 评论 6452358366：26-10-28 批复 #1523 裁定稿 28G（Q11）。账户级只读查询，即在挂订单、持仓与回报前沿的回读，其 HL adapter 扩展由本票在 #1770（C-API）之上承接（落点本票 :7、:11-12）。#1770 只交按原订单目标的单订单状态查询，请求账
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:59：一个会话只有一个 `evidence_class`，也只有一个外部事实源（C10.2 :345；ADR 0052:250）。模拟 fill 与真实 fill 不得同会话并存。混源只能出现在另一个会话里（见裁定七 (c)）。simulation 会话的产…
- .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:40：…conomicView / History | session、指定域前沿或 AsKnown 依赖向量、scope | 返回每个域真实 cut、consumed_structure、完整性、缺项和权威状态 | 不阻 S；不把恢复应用回填旧时刻；重算版本另模式 |
- rust/src/theta_v0/nautilus/account_adapter.rs:8：//! - `voice_qty[0]` ← `portfolio.net_position(instrument_id)` 的手数（v0 单声部 depth=0）。

- 推荐值：物理持仓不是独立真账，没有自己的写者。它是两列的派生对账读数：「E 已 Recorded 的场所持仓/成交原件」（经 X 内 adapter 回读，ADR 0056:125；账户级回读由 #1400 承接，评论 6452358366）与「Σ各重 B applications 投影」，只经 `Q.ReadEconomicView`（INTERFACE-CONTRACTS:40）输出。差异只产 Pending/封闸（C06.4，#1340:297），永不覆写 B 私账（C06.7，#1340:300），不从场所净仓重建成本（C09.9，#1340:338）。推导：ADR 0051:310 只认根链为真相；ADR 0038:53 把第 5 面定性为「对账」；ADR 0056:59 一个会话一个外部事实源 ⟹ 再立一本真账即两个权威。 账户查询结果是场所 adapter 原件、进 E 成 Recorded 事实，只作对账列与证据载体，不推进成交 F、不产生分配、不写 B（Q13 类别三）。
- 已搜关键词：物理持仓、场所净仓、账实快照、net_position、position reconciliation、对账、reconcil
- 缺口去向：交用户决定（decision）
- 冲突：正本层无物理持仓 owner 记载（换说法「对账|reconcil」复搜正本层 24 行，无一处给物理持仓指定写者）；旧臂 `theta_v0/nautilus/account_adapter.rs:8` 把 `portfolio.net_position` 直接写成 `voice_qty[0]`（场所净仓→私账），与 C06.7/C09.9 相反，不得作先例。
- 首版订正：
  - 首版把「查询结果以场所 adapter 原件进 E」归为「#1400 评论」原文；该句是 ADR 0056:125 对 #1400 的登记，评论 6452358366 的原文是「账户级只读查询，即在挂订单、持仓与回报前沿的回读……由本票在 #1770 之上承接」。两处分列。
  - 【40Y 订正】40R 的 Q9 默认值把「账户回读」列进 F 归一函数，与本点「回读只作对账列」相撞（40Y B3）。本版 Q9/Q13 已把账户查询移出 F 归一，本点推荐值补末句，其余不变。

## Q5 [A] 重复回报（同事实重投）的去重协议是什么？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | fact_key 场所 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "fact_key 场所 in:comments"` | 5 | 6 |
| repo | IdentityConflict | `rg -n 'IdentityConflict' rust/src docs/adr` | 423 | 425 |
| repo | 去重|幂等 | `rg -n '去重\|幂等' docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md` | 14 | 21 |

命中（原文逐条核验）：
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:266-267：1. 去重键是身份，不是内容：外部事实用 `fact_key`，S 输入用 `(identity_key, revision)`，命令用 `command_id`（C06.7）。
- #1340 正文 :300：…ID 与重 ID 同内容只应用一次，异内容冲突；累计与逐笔报使用同一规范化合同。缺来源身份或无法区别同形事实时保留冲突，不能按内容去重误吞真实成交。更正按前件因果应用，账实快照不能直接覆写私账。
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:320：X 侧 `OwnAttempt / MayHaveCalled` 和 fill→Voice 归属的实体规则归 #1330、#1539；本合同只要求它们套 `domain-event/1` 外壳并带 `command_id`。#1539 票面已声明依赖 FU-11，裁定时要引用本合同，因此接 `#1534 blo…
- rust/src/economic_session/store.rs:75：CREATE TABLE applications(k TEXT PRIMARY KEY,fact_key TEXT NOT NULL UNIQUE,json TEXT NOT NULL) STRICT;

- 出处：ADR 0051 裁定二·1-2（:266-267，用户 2026-10-27 批复）；#1340 C06.7（:300）；store.rs:75 `applications.fact_key UNIQUE`。X 侧命令去重键＝`command_id`（C07.5，#1340:309）；本票按 ADR 0051:320 只声明 X 观测记录套同一外壳、沿 no-op / IdentityConflict 两值判，不另立规则。
- 首版订正：
  - 首版出处写 store.rs:49，实际在 :75。
  - 首版「去重|幂等」查询记 14 条；ADR 0051 在 457902e3c3 与 9de18e60 间无差异，两处复跑均为 21 条，首版计数有误。

## Q6 [A] 乱序到达（fee 先于 fill、更正先于原报、跨重先后）的应用协议是什么？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | fee 更正 乱序 | `gh search issues "fee 更正 乱序" --repo xy7365527-lang/NewChanlun` | 0 | 0 |
| tracker | 乱序 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "乱序 in:comments"` | 45 | 13 |
| repo | 乱序|received_time|source_cursor | `rg -n '乱序\|received_time\|source_cursor' rust/src/economic_session` | 23 | 23 |
| repo | 乱序 | `rg -n '乱序\|out.of.order' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 6 | 6 |

命中（原文逐条核验）：
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:268-269：3. 乱序到达：先在同一事务里持久写成 Recorded/Pending（C06.4），用 `received_time` 和 `source_cursor` 记下原始顺序。应用时按 `cause[]` 前件因果执行；缺前件就留在 Pending，不丢…
- docs/adr/0032-fill-allocation-policy.md:33：…rincipal-batches-phase-predicate-corporate-actions.md) 裁定二）。fee-before-fill：E 侧 AwaitingDependency，前件到达后经同一 `book::apply` 应用一次。跨 Voice 迟到归原 family/Voice 尾账。
- rust/src/economic_session/tb03c_e.rs:3-4：//!   `AwaitingDependency`；目标重不在 manifest ⟹ 照记、Pending 范围封为 `account:*`，不路由任何 B。

- 出处：ADR 0051 裁定二·3-4（:268-269）；ADR 0032 裁定五 fee-before-fill（:33）；#1340 C06.4/C06.7；代码 tb03c_e.rs:3-4（AwaitingDependency）与 tb03c_corrections.rs:100-132 `predecessor`。

## Q7 [A] 费用事件（fill 类/command 类/period 类、迟到费用）入账到哪一重、何时应用？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 净额外效 部分成交费用 | `gh search issues "净额外效 部分成交费用" --repo xy7365527-lang/NewChanlun` | 1 | 0 |
| repo | fee_only|bust|reverse_component | `rg -n 'fee_only\|bust\|reverse_component' rust/src/economic_session` | 81 | 81 |
| repo | 费用按因果 | `rg -n '费用按因果\|cause ∈' docs/adr/` | 2 | 2 |

命中（原文逐条核验）：
- docs/adr/0032-fill-allocation-policy.md:31-33：### 五、费用按因果原件归属（第四问 A）
- rust/src/economic_session/tb03c_e.rs:33：/// #1406 H1：费用必须绑定原 fill 的来源身份和原 allocation，不能让任意 Applied 原件充当前件。
- rust/src/economic_session/tb03c_e.rs:59-63：|| reference["allocation_id"] != predecessor["allocation_id"]
- #1340 正文 :287：4. 成本只由真实现金流、成交、费用及更正改变；价格估值变化不能改成本。亏损往返和迟到费用如实入账，手续费、funding、borrow 的来源、计量坐标、归属和未分配项分别可追踪。

- 出处：ADR 0032 裁定五（:31-33，#1350 FU-03）；ADR 0031 裁定二；ADR 0024 裁定六；#1340 C05.4（:287）；代码 tb03c_e.rs:33-35 `check_fill_predecessor`、:59-63 原 allocation 绑定。票面「费用」指入账协议，已裁。
- 首版订正：
  - 首版命中写 tb03c_e.rs:30（该行是 `]))))`），「#1406 H1」注释在 :33。
  - 首版写 C05.4 在 #1340:286，实际在 :287（两轮审查均指出）。

## Q8 [A] 更正/bust/反冲如何应用且不删原报、不让累计量倒退？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | bust 更正 逆规则 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "bust 逆规则 in:comments"` | 6 | 4 |
| repo | 更正|correction | `rg -n '更正\|correction' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 22 | 22 |
| repo | A(F) − A(F−b) | `rg -n 'A_i\(F\) − A_i\(F−b\)\|A\(F\) − A\(F−b\)' docs/adr rust/src` | 4 | 6 |

命中（原文逐条核验）：
- docs/adr/0032-fill-allocation-policy.md:37：…⟹ 反冲该分量；无指向的总量 b ⟹ 在同一 manifest、同一版本下计算 `A_i(F) − A_i(F−b)`。原报与旧 Applied 不删。迁移到新版本只能是带前后政策对照的显式事件（C09.11）。
- rust/src/economic_session/tb03c_corrections.rs:1-3：//! 跨多笔原 fill 的总量 bust 按 ADR 0032 裁定六（#1683）：逐重单位数 = A(F) − A(F−b)，
- #1340 正文 :314：…及退出各有独立权限与前件。旧框终止新经营不终止尾账；业务退出和责任退休分开。未来更正超出原覆盖时如实记录并封受影响新增，不裁掉原报凑不变量。

- 出处：ADR 0032 裁定六（:37）+ 修订节（配额法保单调）；#1340 C06.7/C07.10（:314）；ADR 0051 裁定三·1（撤回＝追加事件，旧行不动）；代码 tb03c_corrections.rs:1-3。

## Q9 [B] 累计成交量 F 的权威来源是什么——本地 Σ去重 fills 还是场所累计回报？「累计完整性证据」由谁出？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 累计完整性 | `gh search issues "累计完整性" --repo xy7365527-lang/NewChanlun --limit 100` | 57 | 58 |
| tracker | 累计完整性 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "累计完整性 in:comments"` | 45 | 46 |
| repo | 累计完整|cumulative | `rg -n '累计完整\|cumulative' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 2 | 2 |
| repo | cumulative|累计量 | `rg -n -i 'cumulative\|累计完整\|累计量' rust/src/economic_session` | 18 | 18 |
| repo | 换说法：终态累计|不可再执行|累计完整（正本层） | `rg -n '终态累计\|不可再执行\|累计完整' docs/adr CONTEXT.md .chanlun/definitions` | —（本版新增） | 2 |
| repo | 换说法：report_kind（代码归一） | `rg -n 'report_kind' rust/src/economic_session` | —（本版新增） | 14 |
| repo | 换说法：减少前件与合取（40Y 复搜） | `rg -n '不可再执行.*累计\|累计.*完整\|回执集合\|所有实际参与账' docs/adr/ .chanlun/definitions/ .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md` | 41F 新增 | 3 |

命中（原文逐条核验）：
- #1340 正文 :295：2. 责任增加先在 E 持久保留，再允许对应本地和外部生效；减少须具备该责任不可再执行/累计完整性的证据及其参与账的持久应用回执。回执集合仅含实际关联账，不扩大为全批次或所有重。
- #1340 正文 :312：8. 逻辑订单首次量 L 冻结，改价不重测 L。始终保持已确定累计执行 F 加各独立可成交分配的最大未计潜量不超过 L。原子 amend 仅在真实累计上限/版本互斥合同成立时共用一份额度；否则等待旧单不可再执行且历史累计完整，再以族/版本 CAS 唯一转移剩余量。
- docs/adr/0032-fill-allocation-policy.md:21：…i(F) ≤ q_i`、`A_i(Q) = q_i`。逐笔分配 ＝ `A_i(F_new) − A_i(F_old)`；逐笔回报与累计回报由同一函数归一。比例权重取冻结请求量（根确认），`[新缠论:选择]`。
- #1425 正文 :12：- [ ] 累计与逐笔规范化同源，更正按前件因果；内容相同的独立真实成交不因内容去重被吞。
- #1340 正文 :348：5. 每个 paper/live adapter 提供能力证据：身份与代际、取消/改单、累计完整性、幂等/查询范围、费用/更正、会话权限、挂单限制、时间语义及唯一外部事实来源。缺项准确 Unsupported；历史通过不表示场所已通过。
- docs/adr/0033-history-simulation-bar-batch-model.md:31：旧单在当前 bar 继续担责并可能成交；bar 收盘即其终态累计 F 完整；改价新版本从 t+1 起，额度 L−F（C07.8/9）。禁止用收盘信息取消 bar 内可能已发生的成交（ADR 0018 裁定四）。
- docs/adr/0052-history-input-profile-and-model-order-unspecified.md:35：- :31「旧单在当前 bar 继续担责并可能成交；bar 收盘即其终态累计 F 完整；改价新版本从 t+1 起」；
- .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:38：…ty | 特定责任或已分配份额的不可再执行/累计完整证据与所有实际参与账收据 | 同一责任界在 E 原子下降/关闭 | 缺任何前件保原潜量；不要求不相关 A 的 Applied；无记录和超时无效 |
- rust/src/economic_session/tb03c_allocation.rs:202-205："cumulative" if reported == to => (),
- #1425 正文 :13：- [ ] 只等实际关联账收据，不把无关坏A纳入本责任下降门；缺前件、累计完整或不可再执行证据保原潜量。（41F 新增）

- 出处：C06.2（#1340:295）/C06.7（:300）/C07.8（:312）/C10.5（:348）；ADR 0032:21；ADR 0033:31（ADR 0052:35 转引）；INTERFACE-CONTRACTS:38；#1425:12、:13；代码 tb03c_allocation.rs:202-205 逐笔/累计两种 `report_kind` 进同一区间核验。
- 推理链：[新缠论:推论] 1) ADR 0051:310 真相只有根链；场所累计回报若绕开 E 即第二权威，违 ADR 0056:59。2) C06.7 要求累计与逐笔同一规范化合同 ⟹ 场所逐笔成交报与累计成交报进 E 各是一条 Recorded 成交事实，经 ADR 0032:21 同一函数归一（tb03c_allocation.rs:202-205 已如此）；账户查询结果不是成交报，不进该函数（Q13 类别三）。3) 减少责任要哪些前件，已批合同按动作给出，本票只裁证据由谁写、以什么事实为载体，不改前件：C06.2（:295）与 INTERFACE-CONTRACTS:38 要覆盖该责任的不可再执行/累计完整证据及所有实际参与账收据，「缺任何前件保原潜量」；C07.8（:312）对非原子 amend 转移明写「等待旧单不可再执行且历史累计完整，再以族/版本 CAS 唯一转移剩余量」；#1425:13 把「缺前件、累计完整或不可再执行证据」都列为保原潜量。C06.2 的「/」是证据种类的并列，不授权用一种证据顶替另一种。反例：旧单已撤、迟到成交尚未收齐时，撤单终态只证明不可再执行，不证明历史累计完整，C07.8 转移必须继续等。4) simulation 下 ADR 0033:31「bar 收盘即其终态累计 F 完整」同时给出两项事实；experiment/production 下各项分别由 E 已 Recorded 的场所 adapter 原件证明，原件种类由该 adapter 的 C10.5 能力证据声明。能力声明说明「怎样证明」，不能选择取消哪一项前件；能力缺项（Unsupported）即保原潜量。
- 默认值：F 的唯一写者＝E：场所逐笔成交报与累计成交报都作为原件进 E，各成一条 Recorded 事实，经 ADR 0032:21 同一函数归一，只有 F 前进时应用 `A_i(F_new) − A_i(F_old)`；账户查询结果另作 Recorded 原件，不推进 F（Q13）。减少责任的前件照已批合同，不在本票重裁：C06.2/INTERFACE-CONTRACTS:38 的一般减少要覆盖该责任的证据及实际参与账持久回执；C07.8 非原子转移要「旧单不可再执行」与「历史累计完整」两项同时成立，再以族/版本 CAS 转移剩余量。本票裁证据载体：simulation 以受控撮合在 bar 收盘产出的终态为据（ADR 0033:31，同时给出两项事实）；experiment/production 以 E 已 Recorded、种类由该 adapter C10.5 能力证据声明的场所 adapter 原件为据，所需各项分别证明（例如撤单终态回报只证不可再执行，累计完整须由该 adapter 声明的累计完整性能力对应的原件另证）。缺任一所需证据或能力缺项，保原潜量。不新增「Σ逐笔＝场所累计」一类合同外判据。回报通道（逐笔/累计/两者）是场所载体事实，记入 adapter 能力证据，不作可调字段（ADR 0041:13）。
- 冲突：40R 版把 C06.2 的「/」读成「不可再执行或累计完整性之一」，与 C07.8（#1340:312）明写的合取及 INTERFACE-CONTRACTS:38「缺任何前件保原潜量」相反，本版撤回。首版「Σ去重 fills ＝ 场所累计 ∧ 订单终态」作为一切减少的统一判据仍撤回，C06.2 原文没有它。
- 首版订正：
  - 首版漏列 ADR 0033:31「bar 收盘即其终态累计 F 完整」与 ADR 0052:35 转引（两轮审查均指出），本版补入并以换说法复搜核实正本层只此两处。
  - 首版默认值「累计完整性证据＝Σ去重 fills ＝ 场所累计且订单已终态」撤回：该合取在 C06.2 原文无据，且 simulation 无场所回读（审查 attempt1 R2-2）。本版默认值按上述载体事实重写。
  - 【40Y 订正】40R 默认值与推理链第 3、4 步写「「不可再执行」或「累计完整性」之一，照 C06.2 原文的「/」」，并把账户回读列为 F 归一函数的输入。40Y B2/B3 指出 C07.8（#1340:312）对非原子转移明写两项合取，斜线推不出可互换，账户查询也不是成交报。本版按动作列已批前件（不改合同），只裁证据载体；账户查询移出 F 归一（见 Q13）。新增查询 1 条、命中 1 条（#1425:13）。

## Q10 [B] 崩溃/重启/换代后累计量与持仓如何保持稳定——回退到 checkpoint 还是保留 MayHaveCalled 最大责任？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | MayHaveCalled | `gh search issues "MayHaveCalled" --repo xy7365527-lang/NewChanlun --limit 100` | 10 | 11 |
| repo | OwnAttempt|MayHaveCalled | `rg -n 'OwnAttempt\|MayHaveCalled' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 6 | 6 |
| repo | 恢复 checkpoint 回退 | `rg -n '回退到上一 checkpoint\|重放日志\|StorageUnavailable' docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md` | 4 | 7 |
| repo | 换说法：回退|checkpoint（正本层） | `rg -n '回退\|checkpoint' docs/adr CONTEXT.md .chanlun/definitions` | —（本版新增） | 92 |
| repo | 换说法：ADR 0038:60 的全部引用方 | `rg -n '0038:60' docs/adr CONTEXT.md .chanlun/definitions` | —（本版新增） | 9 |
| repo | 换说法：ADR 0030 恢复条的引用方 | `rg -n '0030:44\|0030:48' docs/adr` | —（本版新增） | 9 |
| repo | 换说法：域 recover 代码是否回退 | `rg -n '恢复时刻早于已提交事实' rust/src` | —（本版新增） | 1 |
| tracker | 换说法：外效未发生 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "外效未发生 in:comments"` | —（本版新增） | 184 |
| repo | 换说法：同模式 H1/H2 射程（40Y 复搜） | `rg -n '同模式\|H1/H2\|杀停\|H2' docs/adr/0056-three-mode-shared-core-adapter-boundary.md` | 41F 新增 | 2 |

命中（原文逐条核验）：
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:274-278：2. 恢复就是从最后一个 checkpoint 加载快照，再重放日志（ADR 0038:60）。幂等要求两条同时成立：
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:60：**恢复**：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar；杀停等同未完成 bar 的全部外效未发生，S/B/E 回退到上一 checkpoint。
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:81：2. **H2 杀停恢复**：t=1000 时 SIGKILL，重启从最后 checkpoint（如 t=950）恢复并续至 t=1050，六面快照与未杀停的连续运行在 t=1050 相同；
- #1340 正文 :310：6. MayHaveCalled 在真实调用前持久提交。提交后崩溃，即使可能尚未真正调用，也保留最大责任，不自动重发。同 ID 重投仅在另有获准且已实证的场所强幂等合同下开放；本 SPEC 不提供通用网络恰一次承诺。
- #1340 正文 :337：8. 首版 X 不自动跨主机接管。重启/换代先由部署监督器确认旧进程及发送子进程已退出，并取得持久发送身份和排他进程锁，再核原 MayHaveCalled/在途记录；无法证实旧发送者隔离时不开新发送。锁或 epoch 本身不取消已发网络请求，其责任继续保留。旧凭据副…
- #1340 正文 :338：9. E 重启先恢复 Recorded/Pending、证据及源前沿、责任集合、Released 和对应 S/X 状态，不能从旧 Open 恢复。B 恢复重放原归属应用，不从场所净仓重建成本。S 恢复未完成修订时保持相应条件门关闭，自己的恢复不等待 E。
- #1340 正文 :340：…本升级先离线验证 schema/原始事实/责任 crosswalk，发布新 writer_epoch 后旧写者不得再写。回退保留新外效和尾责；旧程序不能理解新责任格式时，只允当前受控收尾路径接管，不删除日志后恢复旧交易。
- #1340 正文 :425：| AT-13 | 持仓、未结单、尾账和转移中断恢复不丢权利、不复制资金；回退保留所有已经发生的外效 | C05/09/11；FE-08 |
- docs/adr/0030-exit-authorization-and-liability-retirement.md:48：授权在首次外部动作前持久化；恢复后先只读对账在途订单（外部未知按最大保留，不伪造终态），再于同一授权 id 下续做；三类来源一律如此。无法连接场所是失败态（保留最大责任、重试、不切判据、不回退旧 writer），不是另一种政策。
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:26：7. **ADR 0030:44**：「无法连接场所是失败态（保留最大责任、重试、不切判据、不回退旧 writer），不是另一种政策。」
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:27：8. **ADR 0038:60**【裁定二】：「**恢复**：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar」。**ADR 0038:74**【裁定三】：「`evidence_class ∈ {simulation, experiment, production}` 作为 profile…
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:147：…立进程启动 S、E、各 Bᵢ、X，只把场所写凭据交给 X。启动和换代时核 TB-09 指纹（ADR 0028:46）和 checkpoint（ADR 0038:60）。现有三个会话 bin 保留为域进程，不是 launcher。X 新立为 `x_execution_session`。π、organic、Python `trading_syste…
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:289：- 启动时检测 checkpoint 并恢复（ADR 0038:60）。
- docs/adr/0042-s-writer-incremental-production-path.md:40：冷启动是增量路径从空状态起步，g=1 是增量的第一步。恢复由增量路径自己完成：从已持久化状态重建运行态，或按 ADR 0038:60 从 checkpoint 加载六面快照后逐 bar 增量重放。从 stale 停写回到写入只有一条链：监督器 recover → 增量路径持久态重建 → 继续增量。
- docs/adr/0052-history-input-profile-and-model-order-unspecified.md:296：`clock.kind = "logical-bar/1"`。在 `simulation` 下，S/B/E/X 不读系统时钟，`received_at` 取原件值。checkpoint、seek 和 `known_frontier_cut` 只认 `bar_index`（A…
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:310：1. **真相**（append-only）：`facts / applications / commits` 根链，以及 S 的 `raw_events / structure_deltas`。
- rust/src/economic_session/store.rs:1709-1711：return Err("InvalidDomain：恢复时刻早于已提交事实".into());
- rust/src/economic_session/content_store.rs:1206-1215：tx.execute("DELETE FROM content_staging", []).map_err(io)?;
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:63：- **(a) 同模式**：沿用 ADR 0038 H1/H2（:80/:81），本 ADR 不重裁。（41F 新增）
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:128：- **#1411**（OPEN，TB-07-C）：裁定七 (a) 沿用 ADR 0038 H1/H2，不加新义务；裁定五第 4 条的期限数值进 `content_hash`。（41F 新增）
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:66：- 同一 ExactCommand 夹具经 X 唯一调用边，分别走 simulation adapter 与 HL adapter（本地夹具端点）。X 侧调用记录除 adapter 身份与场所回执外逐字段相等，ADR 0039 A4 写权闭包 diff 为空（28G:220）。（41F 新增）
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:101：- #1381/1399/1411/1418（TB-07，历史双跑实现须实装 H1–H4 四项 oracle）（41F 新增）
- #1340 正文 :347：4. 双跑固定原始输入及修订、政策/模型版本、时钟、随机种子、数值语义、故障与调度轨迹，比较完整结构关系、逐重状态/毛账、责任、命令/回报/分配和 AsKnown。允许不同运行 ID 等非语义字段经过预先声明…（41F 新增）
- #1340 正文 :436：新增一个从正式 launcher 启动的完整会话测试夹具承载上述外部行为，使用两个独立新进程双跑及真正杀进程/重启。结构、状态、账本/责任和外效各面差异均硬判，不继承旧 expected/advisory 规则掩盖差异；样例必须实际产生其待测对象、动作和回报，不能以空域通过。具体样例数量服务于…（41F 新增）
- #1411 正文 :11（41F live 快照）：- [ ] 固定输入修订、政策/模型、时钟、随机种子、数值、故障与调度轨迹，结构关系/逐重毛账/责任/命令回报分配/AsKnown各面差异均硬判。（41F 新增）
- #1411 正文 :13（41F live 快照）：- [ ] 真实中断S/E/B/X各自持久提交前后及回报/应用边界；恢复不重复调用、不复活旧身份、不丢最大责任、不抹晚到时间。（41F 新增）

- 出处：ADR 0051 裁定三（:274-278，已批复）；#1340 C07.6（:310）/C09.8（:337）/C09.9（:338）/C09.11（:340）/AT-13（:425）；ADR 0030 裁定五（:48）；ADR 0038 裁定二（:60）与 H2（:81）；ADR 0039 :26/:27/:147/:289；代码 store.rs:1692-1725 `recover`（审计、提升 epoch，:1710-1711 拒绝早于最后提交的恢复时刻；不删已提交行，schema v2 下只清未发布的 `content_staging`，content_store.rs:1206-1215）。
- 推理链：[新缠论:推论] 1) ADR 0038:60 含两句：前句「检测 checkpoint、加载六面快照、重放至当前 bar」与 ADR 0051:274「快照＋重放日志」一致，三种证据类都成立；后句「未完成 bar 的全部外效未发生，S/B/E 回退」的前提只在 simulation 成立——受控撮合没有真实外效，重跑该 bar 得同一 fact_key，已持久的行按 ADR 0051:275-276 幂等命中。2) experiment/production 下 checkpoint 后已 Recorded 的场所原件、X 已持久的 MayHaveCalled 是真实外效；回退 E/B 会丢 Recorded 回报并重铸责任，违 C07.6、C09.9、ADR 0030:48 与 AT-13。3) 「回退」在任何证据类下都不得删除 append-only 真相（ADR 0051:310）；代码 store.rs `recover` 本就只审计、提升 epoch，拒绝早于最后提交的恢复时刻（:1710-1711），不删已提交行。4) ADR 0039:27/:147/:289 把 ADR 0038:60 写进正式 launcher（含 production）启动序列，没有限定证据类；两份已裁正本因此须同时修订，而不是只在新 ADR 里单方面收窄。两条分支源于载体事实，不是同一判断的宽严两档（AGENTS.md:46 #799；ADR 0041:13）。 5) 恢复方法按证据类分，不等于验收射程按证据类分：ADR 0056:63 裁定七 (a) 已把 H1/H2 定为三模式同模式 oracle，:128 交 #1411 且「不加新义务」；#1340 C10.4（:347）、AT-14（:426）、:436 要求固定完整输入、两个独立新进程、真正杀进程/重启、各面硬判。#1411:11/:13 同时要求「与相同轨迹的未中断运行一致」和「恢复不重复调用、不丢最大责任」，可见固定轨迹下不重发与六面等价是并存约束。故修订只收窄 :60 后句的射程，不收窄 H2 的射程。
- 默认值：在 ADR 0038 与 ADR 0039 文末各追加 append-only「修订（#1539）」节并改三行头第二行；ADR 0030 不改。修订内容：恢复＝从最后一个 checkpoint 加载六面快照再重放持久日志，三种证据类通用；:60 后句「外效未发生、S/B/E 回退」只适用于 simulation，且指工作态回到快照后由受控撮合重跑、已持久行按 ADR 0051:275-276 幂等命中，不删行；experiment/production 不适用后句，checkpoint 之后已持久的 X/E/Bᵢ 事实全部保留，在途订单先只读对账，外部未知按最大责任保留、不伪造终态、不自动重发。验收射程不变：H1/H2（:80/:81）按 ADR 0056:63 裁定七 (a) 对三种模式的同模式会话都适用，即在固定完整输入（输入及修订、政策/模型、时钟、种子、数值语义、故障与调度轨迹，C10.4 #1340:347；experiment/production 的场所侧用受控场所夹具，如 ADR 0056:66 的本地夹具端点）下，杀停恢复后的六面与未杀停的连续运行相同；各证据类用各自的恢复方法满足同一 H2。AT-13（:425）是真实外效的附加恢复约束（不回退、不重发），不替代六面 H2。
- 冲突：在案冲突先例：ADR 0038:60 的「回退到上一 checkpoint」未按 evidence_class 限定，与 C07.6（#1340:310）、C09.9（:338）、AT-13（:425）、ADR 0030:48 在 experiment/production 下方向相反；ADR 0039:27/:147/:289 把 ADR 0038:60 写进正式 launcher（含 production）启动序列。同向在案先例：ADR 0030:48、ADR 0039:26「不回退旧 writer」、C09.11（:340）「回退保留新外效和尾责」、store.rs:1710-1711。读法一致、无需修订的引用方：ADR 0051:274（只取快照＋重放）、ADR 0042:40（S 从持久态或快照＋增量重放）、ADR 0052:175/:296（simulation）；未入 main 的 ADR 0058（#1550，PR #1821）:45/:163 引 ADR 0038:60 于回放外壳（simulation），同样一致。指针漂移：ADR 0030 恢复条在 4bc8723e86 为 :44，40f11b540f（#1599 批复插入两行）后为 :48；ADR 0039:26/:288/:326/:412/:427、ADR 0045:23/:63、ADR 0048:114/:378 仍写 :44。 40R 版另把 H2 写成 simulation 专属 oracle、experiment/production 只按 ADR 0051 裁定三与 AT-13 验收，与 ADR 0056:63（同模式沿用 H1/H2）、:128（#1411）相冲，本版撤回。
- 首版订正：
  - 首版漏列同向先例 ADR 0030:48（及 ADR 0039:26 对它的引用）与冲突先例 ADR 0039:27/:147/:289（审查 attempt2 R8）；冲突栏未提 ADR 0039。本版补入，并列全 `0038:60` 字样的 9 行引用（ADR 0051:193/:198/:274、ADR 0039:27/:147/:289/:428、ADR 0042:40/:175），另 ADR 0052:175/:296 以「0038:56/:60」形式引用。
  - 首版第三条查询记 4 条；ADR 0038/0051 在 457902e3c3 与 9de18e60 间无差异，复跑为 7 条，首版计数有误。
  - 【40Y 订正】40R 默认值、附件 A 第 4 条与问表把 H2 缩为 simulation 的 oracle，又称「本修订不增删验收项」。40Y B1 指出 ADR 0056:63/:128 已把同模式 H1/H2 交给三模式与 #1411。本版保留三模式固定完整输入下的 H2，AT-13 写成附加约束；ADR 0056 因射程未变无需修订。新增查询 1 条、命中 8 条。

## Q11 [B] fill 归属到哪一重/Voice 由谁决定、分配记录（allocation）的写者是谁？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 原经济归属 唯一应用 | `gh search issues "原经济归属" --repo xy7365527-lang/NewChanlun` | 1 | 30 |
| repo | projection_kind|per_leg|netted | `rg -n 'projection_kind\|per_leg\|netted' rust/src/economic_session` | 128 | 128 |
| repo | 物理投影|projection_kind | `rg -n '物理投影\|projection_kind' docs/adr/ CONTEXT.md .chanlun/definitions/` | 16 | 16 |
| repo | 换说法：fill_allocations（分配记录载体） | `rg -n 'fill_allocations' rust/src/economic_session` | —（本版新增） | 32 |

命中（原文逐条核验）：
- docs/adr/0032-fill-allocation-policy.md:12-16：命令投影时冻结 `manifest = {projection_kind ∈ {per_leg, netted}, participants:[(chong_id, command_id, family_id, revision, requested_q_i)], tie_break_ord…
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:320：X 侧 `OwnAttempt / MayHaveCalled` 和 fill→Voice 归属的实体规则归 #1330、#1539；本合同只要求它们套 `domain-event/1` 外壳并带 `command_id`。#1539 票面已声明依赖 FU-11，裁定时要引用本合同，因此接 `#1534 blo…
- rust/src/economic_session/tb03c_e.rs:4：//!   `AwaitingDependency`；目标重不在 manifest ⟹ 照记、Pending 范围封为 `account:*`，不路由任何 B。
- #1770 正文 :34：…E 或 B 的归属。记录不含 `allocation_id`、`family_id`、`book_identity`，不判定归属。E 目前只接受 TestOnly 经济原件 profile（`book.rs:106-107`），原件还须带归属字段（`:117-123`），所以 C-API 的观测记录不是 E…
- rust/src/economic_session/tb03c_allocation.rs:340-349：event["allocation_id"],
- rust/src/economic_session/tb03c_allocation.rs:365：next["fill_allocations"].as_array_mut().unwrap().push(json!({"fact_key":fact["fact_key"],"fact_hash":fact["fact_h…
- .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:37：| B.ApplyAllocation | fact/allocation_id、归属重/Voice/family、数量/现金/费用/更正关系及已批分配政策 | 本地唯一应用并返回持久收据；真实经济事件驱动毛账/成本/…

- 出处：ADR 0032 裁定一/二（:12-16，#1350）；ADR 0051:320 把「fill→Voice 归属的实体规则」点名归本票；#1770:34 X 不判归属；代码 tb03c_allocation.rs:340-349/:365 由 E 派生逐重 allocation 并写 `fill_allocations` 行，tb03c_e.rs:4 缺 manifest 封 `account:*`。
- 推理链：[新缠论:推论] 1) 归属规则本体 ADR 0032 已裁，本票只补「谁写 allocation」。2) X 不判归属（#1770:34），B 只应用（INTERFACE-CONTRACTS:37），余下只有 E。3) 现有代码 tb03c_allocation.rs:340-349 正是 E 在 Resolve 时按冻结 manifest 派生逐重 id 与差量。
- 默认值：分配记录（逐重 `allocation_id`、`chong_id`、`command_id`、`family_id`、`units`、费用分量）的唯一写者＝E，在 Resolve 时按 ADR 0032 冻结 manifest 派生；B 是唯一应用者；X 不参与归属；缺 manifest/归属不明按 tb03c_e.rs:4 封 `account:*` Pending。不改分配算法。

## Q12 [C] 本票的「crosswalk」指什么——词在仓内多义且 CONTEXT.md 无词条，与 FU-13 的身份 crosswalk 如何划界？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | crosswalk | `gh search issues "crosswalk" --repo xy7365527-lang/NewChanlun --limit 100` | 100 | 100 |
| tracker | 初始持仓 crosswalk in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "初始持仓 crosswalk in:comments"` | 2 | 2 |
| repo | crosswalk | `rg -n -i 'crosswalk' docs/adr/ .chanlun/definitions/ CONTEXT.md` | 4 | 4 |
| repo | crosswalk | `rg -n -i 'crosswalk' analysis/ docs/ .chanlun/ --glob '!docs/adr/**'` | 512 | 512 |
| repo | crosswalk | `rg -n -i 'crosswalk' rust/ src/ tests/ trading_system/ scripts/` | 9 | 9 |
| repo | 换说法：跨模式映射|身份映射 | `rg -n '跨模式映射\|身份映射\|identity.map' docs/adr CONTEXT.md .chanlun/definitions analysis docs` | —（本版新增） | 4 |
| repo | 换说法：物化|可重算投影（40Y 复搜） | `rg -n '物化\|可重算投影' docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md` | 41F 新增 | 2 |

命中（原文逐条核验）：
- #1340 正文 :358：5. 切换先禁止旧程序新发送，查明旧订单/持仓/费用/更正/未决命令及最大责任，再导入新核并证明身份 crosswalk。新旧不双写；有仓、有尾账的迁移必须测试，不以空仓样例代替。
- #1340 正文 :340：11. 版本升级先离线验证 schema/原始事实/责任 crosswalk，发布新 writer_epoch 后旧写者不得再写。回退保留新外效和尾责；旧程序不能理解新责任格式时，只允当前受控收尾路径接管，不删除日志后恢复旧交易。
- #1535 正文 :3：原则已定，但非空持仓、命令、尾账 crosswalk 未验证。需裁决：
- docs/adr/0052-history-input-profile-and-model-order-unspecified.md:267：| 持仓 | `initial_positions[]` 默认空集；非空只允许 TB-10 迁移用例引用 crosswalk 记录 id，不接受裸数量 | ADR 0038:53；#1340 C11.5 :358 |
- docs/adr/0037-s-content-bounded-layout.md:26：#1488 段 A（layout/1）以公开根与 r6 逐位相同为锁；段 B（本布局）以 crosswalk oracle（§5.8 形式，含单元与比较总数相等）+ 双跑逐位一致 + 上界门（S86 g=1..86 逐代核，超界代数 0，r6 代入区分力倍数 1.10–1.68）验收；#1481 阶段…
- docs/adr/0037-s-content-bounded-layout.md:29：…ta 应用按出生序追加、Python 重建口径、tb02a oracle 随新修订重算；单元总数与比较总数与 r6 相等作 crosswalk 锁；S86 r6/r7 不重跑，继续作 #1394 Long 域证据与段 B 的 crosswalk oracle。
- #1406 正文 :43：下列ID只定位上文行为对应的子义务，完整定义、量词与跨域闭合仍由原SPEC/crosswalk持有。任何已裁/待证标签不自动生成排程边；正确拒绝、空实例、schema存在不能替代必需成功实例。
- docs/adr/0028-first-capability-verification-profile.md:71：…）；裁定一前置读数失败时退回 `P0`。只有目标生产部署才能证明的义务——生产旧凭据/launcher/OS 身份隔离、带仓身份映射、生产首次新发送前 TB-09 重核、默认启用批准——保持为「生产首次新发送前」的具名门，未触发；P1 证据不得读成已满足（`CONTEXT.md` 证据类词条）。
- docs/adr/0012-inference-layer-document-naming.md:106：**X3 从「冲突」降级为「一次读解误判」**，且该误判本身又是元判据的实例：**拿词的外形（「声部」两个字）当同一性判据**。
- docs/adr/0012-inference-layer-document-naming.md:108：**译注去向（不破裁定三）**：把词的对应写进 `CONTEXT.md` 声部词条（**正本层，允许改**），推论层原文一个字不动。
- /Volumes/AgentStorage/decision-map-1323-倒推-20261018.md:69：状态：逐重毛账/物理投影原则已定；唯一 owner、来源、跨模式映射待定。验收：每个 fill/position 一个权威写者；重复/乱序/费用/更正/恢复累计量稳定；crosswalk 可审计。阻塞 TB-04/07/08/09；依赖 FU-03/0…
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:311：2. **可重算投影**：book image、`growth_eligible`、毛账合计。可以物化，但审计时必须等于重放结果。（41F 新增）

- 推荐值：本票 crosswalk ＝「同一外效在五类身份记录之间持久、可逆的四跳逐跳映射」：① 场所身份（客户端单号、`oid`、成交标识）↔ X 观测记录（`event_id`、`command_id`），写在 X 域库（#1770:22/:34）；② X 观测记录 ↔ E 原件（同名 `source` 字段，`cause[]` 含 X `event_id`，`fact_key` 按 book.rs:93-99），写在 E facts；③ E 原件 ↔ 逐重分配（`fill_allocations` 行，tb03c_allocation.rs:349/:365），写在 E；④ 逐重分配 ↔ Bᵢ application（`applications.fact_key UNIQUE`，store.rs:75），写在 Bᵢ。每跳只由所在域唯一写者写。禁止第二权威写入口：任何汇总表或索引都不得独立写入映射，也不得反写任一跳；日后若物化汇总，只能是可由四跳逐跳记录重算的只读投影，审计时必须等于逐跳拼接结果（ADR 0051:311 同款）。本票不交付物化层，这是实施范围，不是禁令。审计由 `Q.ReadEconomicView` 双向逐跳拼接，缺跳报缺项不补猜；跨模式时四跳字段按 ADR 0056:85 逐字节相等。旧→新迁移的身份/责任/尾账 crosswalk（C09.11、C11.5、ADR 0052:267、ADR 0028:71「带仓身份映射」）归 #1535，消费本定义。CONTEXT.md 词条以 `_Avoid_` 标出与 `IMPLEMENTATION-CROSSWALK.json`、ADR 0037 crosswalk oracle 混称。推导：ADR 0012:106/:108 要求先定词并写进 CONTEXT.md；逐跳持久化由 ADR 0039:300 每域单写者直接推出。
- 已搜关键词：crosswalk、身份 crosswalk、跨模式映射、责任 crosswalk、IMPLEMENTATION-CROSSWALK、身份映射
- 缺口去向：交用户决定（decision）
- 冲突：同一词四义：① SPEC 包 `IMPLEMENTATION-CROSSWALK.json`（#1406:43/:50）；② ADR 0037:26/:29 crosswalk oracle/锁（S 布局对拍）；③ C09.11（#1340:340）/C11.5（:358）/#1535/ADR 0052:267/ADR 0028:71 的旧→新迁移身份/责任/尾账映射；④ 决策地图 :69 的「跨模式映射」。CONTEXT.md 0 命中，正本层 4 命中全是用法不是定义。
- 首版订正：
  - 首版漏列 ADR 0028:71「带仓身份映射」（换说法复搜补出，与义③同类）。
  - 【40Y 订正】40R 推荐值写「不建汇总表（汇总表即同一映射的第二写者）」，把只读可重算投影一并排除；40Y B4 指出 ADR 0051:311 允许投影物化、审计等于重放，作者 40R REPORT 也承认这一点。本版把禁令收窄为禁止第二权威写入口，物化与否属实施范围。「四个身份空间」改为「五类身份记录之间的四跳」，映射合同不变。新增查询 1 条、命中 1 条。

## Q13 [B] X 观测记录（场所回报）到 E 原件的映射由谁生成、如何让 fact_key 回溯到 command_id 与场所单号可审计？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | external_fact_id in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "external_fact_id in:comments"` | 3 | 4 |
| tracker | exec_id | `gh search issues "exec_id" --repo xy7365527-lang/NewChanlun` | 0 | 1 |
| repo | external_fact_id | `rg -n 'external_fact_id' rust/src docs/ .chanlun/` | 119 | 120 |
| repo | exec_id|cloid|bound_venue_oid | `rg -n -i 'exec_id\|cloid\|bound_venue_oid' rust/ docs/ .chanlun/ analysis/` | 4 | 4 |
| repo | evidence_class in X bin | `rg -n 'evidence_class' rust/src/bin/x_execution_session.rs rust/src/bin/production_session.rs` | 0 | 0 |
| repo | 换说法：原件级 allocation_id 的消费点 | `rg -n 'reference\["allocation_id"\]\|event\["allocation_id"\]' rust/src/economic_session` | —（本版新增） | 28 |
| repo | 换说法：费用 cause 与账户查询（40Y 复搜） | `rg -n 'period\|command\|账户查询\|账户回读\|查询结果' docs/adr/0032-fill-allocation-policy.md docs/adr/0056-three-mode-shared-core-adapter-boundary.md` | 41F 新增 | 4 |

命中（原文逐条核验）：
- #1770 正文 :34：…HTTP 响应与用户事件的 X 观测记录。记录保存原字节，并带与 main E 原件 `source` 必需字段同名的出处字段 `source_namespace`、`source_epoch`、`source_cursor`、`external_fact_id`、`received_time`、`claimed_origin`（`book.rs:93-99`、`:125-127`）。调用响应的记录另按变体带 X 侧调用关联。下单记录带 `…
- #1770 正文 :22：…观测记录绑定 `oid` 时，`target(C1)` 带可选字段 `bound_venue_oid`，没有回执时不带。场所单号只能来自 C1 自己的回报，不能从列表查询或同标的其他订单推得。C1 下单失去响应时没有回执，场所上仍可能有这张单（:15、C07.7）。按客户端单号定位，这种情况也能撤、能查。
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:46：…117-124` 的必填项（`fact_key` 在 `:124`，定义在 `:93`）。`format` 是来源标签，取三值之一：受控撮合原件、场所 adapter 原件、TestOnly 夹具原件。E 校验 `format` 与会话 `evidence_class` 一致（TestOnly 夹具原件只配 `TestOnly` 时钟的组件会话），不一致返回 `SchemaUn…
- rust/src/economic_session/book.rs:106-107：if doc["format"] != "neutral TestOnly economic raw originals; not production transport wire" {
- rust/src/economic_session/book.rs:117-124：required(&event, "allocation_id")?;
- rust/src/bin/x_execution_session.rs:22：const MODES: [&str; 4] = ["simulation", "dry-run", "experiment", "production"];
- rust/src/economic_session/tb03c_allocation.rs:9-11：event.get("netted_fill").is_some() || event.get("per_leg_report").is_some()
- rust/src/economic_session/tb03c_allocation.rs:136-151：return Err("IdentityConflict：原件类型与冻结投影不符，不得回落".into());
- rust/src/economic_session/tb03c_allocation.rs:340-347：event["allocation_id"],
- rust/src/economic_session/tb03c_per_leg.rs:106：event["allocation_id"],
- rust/src/economic_session/tb03c_corrections.rs:125：|| reference["allocation_id"] != original["allocation_id"]
- rust/src/economic_session/tb03c_allocation.rs:387-405：if !is_projected(&record["event"]) {
- rust/src/economic_session/tb03c_transfer.rs:17：…ch"],"external_fact_id":event["source"]["external_fact_id"],"allocation_id":event["allocation_id"]})
- rust/src/economic_session/tb03c_e.rs:62：|| reference["allocation_id"] != predecessor["allocation_id"]
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:125：…er/venue 行（查询走 X 内场所 adapter，X 是唯一执行网络请求的进程，C09.7 :336）、裁定四（查询结果以「场所 adapter 原件」进 E）、裁定六（单一外部事实源）；能力证据形状沿用 C10.5（:348）。本票不改其 AC；它已经 #1382、#1366 被本票传递阻塞，不加直接边。建议测试：`venue_rea…（41F 新增）
- docs/adr/0032-fill-allocation-policy.md:33：`cause ∈ {fill, command, period}`：fill 类沿该 fill 的原分配比例；command 类全额归发出命令的重；period 类按 ADR 0024 裁定六的坐标，真实场所净扣费 Σc = C 的分配矩阵与虚拟…（41F 新增）
- docs/adr/0024-total-ledger-convention.md:43：### 六、funding / borrow 毛暴露 = 每腿敞口绝对值之和（41F 新增）
- rust/src/economic_session/tb03c_book.rs:129：if !["fill", "command", "period"].contains(&required(c, "cause")?) {（41F 新增）
- rust/src/economic_session/tb03c_per_leg.rs:28："manifest_hash",（41F 新增）
- rust/src/economic_session/tb03c_per_leg.rs:30："reported_quantity",（41F 新增）
- rust/src/economic_session/tb03c_allocation.rs:127：let key = required(report(event), "manifest_hash")?;（41F 新增）
- #1400 正文 :7：正式会话使用获准的具名场所/账户profile查询身份、订单、持仓与回报前沿，并将真实能力和证据在只读界面逐项展示。（41F 新增）

- 出处：ADR 0056 裁定四（:46，E 单 schema、format 三值）；ADR 0051 裁定一 `source` 与 `fact_key`；#1770:22/:34（X 观测记录带出处字段、不判归属）；book.rs:117-124（原件必填 `allocation_id`，非投影形另须 `family_id`/`target`）；原件级 `allocation_id` 的生产消费点（换说法复搜，排除测试文件）：投影形路径只有 tb03c_allocation.rs:344（netted 派生逐重 id 的种子）、tb03c_per_leg.rs:106（per_leg 同款种子）、tb03c_corrections.rs:125（netted 费用/更正按原件 `allocation_id` 绑定具名前件），B 侧投影形应用取 E 的 `fill_allocations` 行而不读原件归属（tb03c_allocation.rs:387-405）；非投影形路径（TestOnly/Legacy）在 tb03c_e.rs:62、book.rs:443/:522、tb03c_book.rs:793/:859 把它当身份照记；tb03c_transfer.rs:17 是权利转移事件的身份键，不是场所原件。 事实类别（40Y 复搜）：ADR 0056:125（账户身份、订单、持仓、回报前沿查询结果以场所 adapter 原件进 E）；#1400:7；ADR 0032:33（费用 `cause ∈ {fill, command, period}`，`du = 0` 合法）；ADR 0024:43 裁定六（period 坐标）；tb03c_book.rs:129 已按三类收费用分量；投影形核验只覆盖成交：tb03c_per_leg.rs:28-31（`manifest_hash`、`reported_quantity`、`unit_price`）、tb03c_allocation.rs:127（按 `manifest_hash` 取冻结 manifest）。
- 推理链：[新缠论:推论] 1) C09.7 X 是唯一出网进程，场所回报与查询结果的原字节首先只落 X 观测记录。2) ADR 0056:46 要求 E 只认一份 schema，#1770:34 已令 X 记录带与 E `source` 同名字段 ⟹ X 内 adapter 把观测记录投影成「场所 adapter 原件」；共享的是来源、身份与因果外壳。3) book.rs:118 无条件要求原件带 `allocation_id`；投影形路径上该槽只作派生种子与具名前件绑定，B 侧应用取 E 的 `fill_allocations` 而不读原件归属（换说法复搜列全消费点），因此取回报级标识 `external_fact_id`：它在同一 `source_namespace`/`source_epoch` 内唯一，不含任何重/Voice/family 信息。4) 非投影成交形必填 `family_id`/`target`（book.rs:120-123）会把归属写进原件 ⟹ 命令相关成交只取投影形，非投影成交形只留给 TestOnly 夹具。5) 投影形核验要所发命令的 `manifest_hash`、成交量与单价（tb03c_per_leg.rs:28-31；tb03c_allocation.rs:127），只覆盖命令相关成交。ADR 0056:125 把账户查询结果定为场所 adapter 原件进 E，ADR 0032:33 把费用 cause 定为 fill/command/period 三类且 `du = 0` 合法；这些原件不都以单笔成交 manifest 为前件，不能一律套成交量/单价投影，硬套就得伪造所发命令、成交量或归属；command 费用按 `command_id` 归属，period 费用按既裁坐标（ADR 0024:43 裁定六）。故 payload 按既有事实类别分：成交走投影，command/period 费用走 ADR 0032:33 既裁因果路径，账户查询不推进 F。6) book.rs:106-107 只收 TestOnly、x_execution_session.rs:22 含 `dry-run` 是实装滞后，不改 ADR。
- 默认值：映射由 X 内场所 adapter 生成（不出 X 进程），产物是 ADR 0056:46 的「场所 adapter 原件」；受控撮合原件形状相同。共享外壳：每条原件进 E 成一条 Recorded 事实，带 `source` 来源身份，原件级 `allocation_id`＝该原件的 `external_fact_id`，`cause[]` 含 X 观测记录 `event_id`；都不带 X 写的归属字段（`family_id`、`target`），逐重归属只在 E 产生（Q11）。payload 按事实类别：(1) 命令相关成交报（逐笔/累计）只取投影形（`per_leg_report` 或 `netted_fill`，带所发命令的 `manifest_hash`，`command_id` 由 X 依本单回执关联，#1770:22），E 以 `check_capture` 与冻结 manifest 核对；它的 fill 类费用与更正/bust 按原 fill 的来源三元、`allocation_id`、`manifest_hash` 绑定具名前件（tb03c_corrections.rs:122-129）。(2) command 类与 period 类费用保留 ADR 0032:33 既裁因果路径（command 类全额归发出命令的重，period 类按 ADR 0024 裁定六坐标），由 E 归属；原件只带来源身份、费用类别与因果锚（command 类为 `command_id`），不伪造成交量或 manifest 投影，`du = 0` 合法。(3) 账户查询结果（身份、订单、持仓、回报前沿；ADR 0056:125，#1400:7）不推进成交 F、不产生分配、不写 B：持仓作 Q4 对账列；订单状态与回报前沿可作 Q9 所需证据的载体，以该 adapter C10.5 能力证据声明能证明相应事实为前提。ADR 0056:46 单 schema 不改；book.rs:106-124 的 TestOnly 解码器不改，场所 adapter 原件解码由 #1425（成交及其费用/更正、command/period 费用）与 #1400（账户查询）按上述类别实施，不得靠让 X 填归属字段满足必填项。
- 冲突：代码 vs 正本：book.rs:106-107 只接受 TestOnly format（ADR 0056:46 要求三值）；x_execution_session.rs:22 `MODES` 含 `dry-run`，而 ADR 0056 裁定二 `evidence_class` 只有三值、X bin 内 `evidence_class` 0 命中。两处为实装未跟进（#1769、#1770 在交付），登记不作先例。原件 `allocation_id` 必填与 #1770:34「X 记录不含 allocation_id」的张力由上述默认值消解：#1770:34 约束 X 观测记录，原件级槽取 `external_fact_id` 不是归属。 40R 版令一切场所原件只取成交投影形，Q9 又把账户回读列入 F 归一函数，排除了 ADR 0056:125 的账户查询结果与 ADR 0032:33 的 command/period 费用；本版按事实类别分 payload。
- 首版订正：
  - 首版默认值「E 原件不含 B 归属」未说明 book.rs:118 必填 `allocation_id` 槽由谁填、填什么；本版补定取 `external_fact_id`、只许投影形，并以换说法复搜列全该槽的生产消费点作依据（审查 attempt1 R2-1）。首版草稿第二轮把前件绑定写成 tb03c_e.rs:60-63，那是非投影形分支；投影形的绑定在 tb03c_corrections.rs:125，经 tb03c_e.rs:36-37 分派。
  - 【40Y 订正】40R 默认值「受控撮合原件与场所 adapter 原件只取投影形」把账户查询与 command/period 费用也压成成交投影（40Y B3）。本版只共享来源/身份/因果外壳，payload 按成交、command/period 费用、账户查询三类分开；不新增 X 归属写者，原件级 `allocation_id`＝`external_fact_id` 不变。新增查询 1 条、命中 8 条。

## Q14 [A] 跨模式（simulation ↔ experiment/production）成交/持仓映射的可审计判据是什么、模拟 fill 与真实 fill 能否并存？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 回报录放 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "回报录放 in:comments"` | 174 | 176 |
| repo | recorded-venue-replay|重命名闭集 | `rg -n 'recorded-venue-replay\|重命名闭集' docs/adr/ CONTEXT.md` | 5 | 6 |
| repo | 模拟 fill 真实 fill 权威 | `rg -n '模拟 fill' docs/adr/ CONTEXT.md` | 3 | 3 |

命中（原文逐条核验）：
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:59：…会话只有一个 `evidence_class`，也只有一个外部事实源（C10.2 :345；ADR 0052:250）。模拟 fill 与真实 fill 不得同会话并存。混源只能出现在另一个会话里（见裁定七 (c)）。simulation 会话的产出不进 TStage 触闸计数（ADR 0038:74），分证不交叉引用（ADR 0038:75）。
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:85：- 闭集之外的字段一律原值逐字节相等，包括：`source` 四元、`fact_key`、`allocation_id`、`family_id`、`target`、cause 链、订单/成交 epoch 与前后状态、…
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:88：…lay/1` 只在 simulation 对拍会话中合法，产出是 simulation 证据（ADR 0038:74），不作正式成交或持仓来源。
- #1340 正文 :345：…命周期和观察协议；只替换真实变化的 feed、clock、matcher/venue、传输与受控持久化 adapter。模拟 fill 不能与同会话真实 fill 同时充当权威。
- #1340 正文 :347：…系、逐重状态/毛账、责任、命令/回报/分配和 AsKnown。允许不同运行 ID 等非语义字段经过预先声明的一一重命名；禁止只比较最终净仓。

- 出处：ADR 0056 裁定六（:59 权威排他）与裁定七（:85 闭集外逐字节相等含 fact_key/allocation_id/exec_id/物理持仓；:88 录放不作正式来源）；#1340 C10.2（:345）/C10.4（:347）。本票引用即可，不重裁。
- 首版订正：
  - 首版命中写 ADR 0056:58（节标题后空行），正文在 :59。

## Q15 [B] 本票裁定落到哪份正本——新开 ADR 还是追加到 ADR 0051/0056，CONTEXT.md 补哪些词条？

| 路 | 查询词 | 命令 | 首版命中 | 本版命中 |
|---|---|---|---|---|
| tracker | 正本落点 新开 ADR in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "正本落点 in:comments"` | 20 | 200 |
| repo | ADR 三条件 | `rg -n 'Hard to reverse\|Surprising without context\|real trade-off' .agents/skills/domain-modeling/SKILL.md` | 3 | 3 |
| repo | 真账 词条 | `rg -n '唯一真账\|单一真账\|crosswalk' CONTEXT.md` | 0 | 0 |
| repo | 换说法：append-only 修订节先例 | `rg -n '^## 修订' docs/adr` | —（本版新增） | 5 |

命中（原文逐条核验）：
- .agents/skills/domain-modeling/SKILL.md:70-72：1. **Hard to reverse** — the cost of changing your mind later is meaningful
- #1538 评论 6466830839：pter 五类词条（仅「生产拆三义」提及 broker adapter）。 | 新开 docs/adr/0053-three-mode-shared-core-and-adapter-boundary.md（头三行沿 0038/0052 格式），裁定分：共享核心面、五类 adapter 的 owner 落点与跨模式替换表（Q3-Q7）、时钟 kind 枚举（Q4）
- docs/adr/0056-three-mode-shared-core-adapter-boundary.md:42：场所账户、权限与具体端点归 #1540（FU-18）。成交/持仓唯一真账与 crosswalk 归 #1539（FU-17）。本表不定这两项。
- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:320：…侧 `OwnAttempt / MayHaveCalled` 和 fill→Voice 归属的实体规则归 #1330、#1539；本合同只要求它们套 `domain-event/1` 外壳并带 `command_id`。#1539 票面已声明依赖 FU-11，裁定时要引用本合同，因此接 `#1534 blo…
- docs/adr/0012-inference-layer-document-naming.md:108：**译注去向（不破裁定三）**：把词的对应写进 `CONTEXT.md` 声部词条（**正本层，允许改**），推论层原文一个字不动。
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:2：> 最后裁定：#1530（2026-10-20）；修订：#1523（2026-10-28，见文末「修订」节）
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:487：（append-only：裁定一至七原文不改，本节为现行修订。依据 #1523 P 类裁定稿 28G，用户 2026-10-28 批复，异家族独审 28E（B3 外各项 PASS）与 28H（B3 修复 PASS）；草稿、考…

- 出处：同系先例 #1538→ADR 0056、#1534→ADR 0051 均新开 ADR 并补 CONTEXT.md 词条；ADR 0056:42 与 ADR 0051:320 各留口指向本票；跨 ADR 修订先例 ADR 0039:2（三行头加「修订：#1523」）与 :485-487（文末 append-only 修订节）。
- 推理链：[新缠论:选择] 本票裁跨 ADR 0032/0038/0051/0056 的写者分工与 crosswalk 结构：难以逆转（持久行形状）、缺上下文会意外（物理持仓为何无写者、F 为何不取场所累计、production 为何不回退）、确有取舍（Q4/Q9/Q10/Q12）——三条件齐（SKILL.md:70-72）。追加进 ADR 0051 会把事件外壳与账目写者混写，追加进 ADR 0056 会把 adapter 边界与真账混写。ADR 0038/0039 的恢复语义改动必须落在这两份正本本身（Q10），按 ADR 0039 的 append-only 修订节先例。
- 默认值：新开 ADR，编号落地时取当时 main 下一空号（0058 已由 #1550 PR #1821 使用，不预占），文件 `docs/adr/NNNN-fill-position-single-authority-and-crosswalk.md`；同批在 ADR 0038、ADR 0039 文末各追加「修订（#1539）」节并改三行头第二行；`CONTEXT.md`「运行边界」节（:364）补「唯一真账」「crosswalk」两词条。
- 冲突：首版默认值与已发布 P 影子问表写死 `docs/adr/0058-…`；0058 已被 #1550 的 `decision/1550-publish`（PR #1821）占用。
- 首版订正：
  - 首版命中写 SKILL.md:68-70，实际三条件在 :70-72（:68 是引导句）。
  - 首版出处写「#1538 resolution Q13」；#1538 无该 resolution，所引内容在 P 影子稿评论 6466830839 的 Q13 行（当时拟名 ADR 0053，最终落为 ADR 0056）。

## 机械校验

命中 132 条（40R 114 条沿用其核验；41F 新增 18 条本轮按「路径:行」逐条核验通过）；查询 67 条（41F 新增 4 条仓内查询本轮复跑）；无 research/probe 缺口。
