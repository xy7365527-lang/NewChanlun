# ADR 0051：逐 bar 域事件外壳、身份与生命周期合同

> 正本：本文件是逐 bar 域事件外壳、身份、epoch 分词、去重与因果、撤回与恢复幂等、snapshot/delta 续接与持久边界的教义正本。
> 最后裁定：#1534（FU-11，2026-10-28 落地；用户 2026-10-27 批复）
> 受影响代码清单：见本文「受影响票与 blocking 边变化」节

**状态**：用户已批准，文书待 main 合入。本 ADR 定逐 bar 域事件的外壳、身份与生命周期规则；B/E 的外壳映射与 delta/Gap 读口由「要开的票」所列实施票落地，本文不宣称已实现。
**批准**：[用户 2026-10-27 原话「批准」](https://github.com/xy7365527-lang/NewChanlun/issues/1534#issuecomment-6446727344)。对象是返修候选稿及其异家族独审 PASS；两轮 BLOCK 后按 `.sandcastle/docs/graph-engineering.md` 第 4 步升为 P，本条依用户批复落地，不以 #1323 Notes N-1 代替。
**历史原件**：获批候选稿、考古与独审原件封存在 `.chanlun/review-results/issue1534-policy-20261027/`，保留当时待批措辞，不作为新增教义来源。
**行号基线**：仓内引用固定到 main `da03a10ee0`；候选稿基线为 `bea0a169f9`，两者间仅 `finance_tests.rs` 注释行移位（:754 → :791，逐字不变），本文已订正。

## 问题

#1534（FU-11）要裁三件事：
1. 逐 bar 域事件必含哪些字段（cut、身份、epoch、policy/model hash、前后状态），各字段取什么形状；
2. 撤回与恢复靠什么机制做到幂等；
3. snapshot/delta 的续接协议。

三件都在 `.sandcastle/docs/graph-engineering.md:23` 列出的 T 类范围内，即「状态机与事件 schema」「恢复语义」「接口与 seam」。缠师原文里找不到锚点，所以全部按 `[新缠论:选择]` 或 `[新缠论:推论]` 处理。

考古共 14 问：A 类已裁 1 问（Q13），B 类有先例未正式裁 13 问，C 类确未设计 0 问。没有交给用户的 decision，也没有 research 或 probe 缺口。

考古和独审标出的冲突有五处，本票必须分词或划界：

1. **「cut」一词两物**：S 正本 `.chanlun/definitions/baohan.md:191` 里的 cut 是按代编号的字符串 `cut-{gen}`。B/E 代码 `rust/src/economic_session/service.rs:190-192` 里的 cut 是三元组 `["domain_id","commit_seq","root_hash"]`，跨域时报 `IdentityConflict：跨域cut`。正本层没有登记这个差别，`CONTEXT.md` 也没有「cut」词条。
2. **「epoch」一词两物**：ADR 0039:170 的 `writer_epoch` 是域写者身份；#1340 C02.1 和 `rust/src/economic_session/book.rs:92-99` 的 `source_epoch` 是来源纪元，并参与 `fact_key` 哈希。`CONTEXT.md` 没有任何 epoch 词条。
3. **「代际/generation」和「事件」各自一词两物**：`CONTEXT.md:150-151` 的代际指声部嵌套代数 n；`CONTEXT.md:166` 和 `baohan.md:191` 指 S 的发布代。`CONTEXT.md:291-292` 的「背驰段候选事件」已被 ADR 0007 用作塔内一等事件对象。
4. **政策引用可能出现第二套形状**（独审 R8 指出）：ADR 0049:19-21（#1610 用户批复）规定 `q_ref`「定位 profile 的授权身份，另绑定 version」，且「固定版本不能在运行中换解析策略」；ADR 0029:35 的记录形状是 `⟨policy_id, version, cap, currency, valid_through, authorizer, authorized_at, profile_id⟩`；`CONTEXT.md:454` 的外效分配用三元组 `(fill_allocation_policy_id, manifest_hash, policy_revision)`。本票的 `policy_ref` 必须写明自己和这几种形状的对应关系，不能另起一套。
5. **范围重叠与漏接的依赖边**：#1330（open）和 #1539（open）的题面都包含幂等、乱序和 journal。正本层没有 FU-11 与 FU-17 的分工记录：`OwnAttempt|MayHaveCalled` 在 main `bea0a169f9` 的 `docs/adr/`、`.chanlun/definitions/`、`CONTEXT.md` 中命中 0 条，只出现在 SPEC #1340 正文 6 行（`:200`、`:309-311`、`:337`、`:421`）。仓外决策地图 `/Volumes/AgentStorage/decision-map-1323-倒推-20261018.md:50-51` 写着 FU-11「阻塞 TB-02/03/05/07；依赖 FU-06/10」，`:69` 写着 FU-17「依赖 FU-03/04/11/16」。#1539 票面「依赖的 FU」节也写着「FU-03、FU-04、FU-11（#1534）、FU-16（#1538）」，但 tracker 里 #1539 的 blocked-by 只有 #1538，`#1534 blocks #1539` 一直没接。本票补接这条边，划界见裁定七。

另登记一处漂移：`baohan.md:191` 指向的 `s_structure_session.rs:3349`，当前该行是 `);`；`cut-{…}` 实际在 `:1099` 和 `:3676`。本票不改 baohan.md，只在这里登记，留给下游修订。

## 权威依据

**正本层**

- `docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md`
  - `:47`：「固定五元组（input, policy_version, model_id/revision, seed, scheduler）下，两次独立运行在每个 checkpoint 的全状态必须逐位等价。」
  - `:52`：「4. 订单/成交原件（epoch、身份、前后状态、cause 链）；」
  - `:56`：「批内顺序声明为 `Unknown`（ADR 0033 裁定一），不作比较维度」
  - `:60`：「**恢复**：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar」
  - `:62`「**seek(t) 语义**」到 `:69`「**不偷未来**」
  - `:73`：latch 存储「首次达标 bar 索引 + 证据指纹」，后续 bar 不回填
  - `:76`：「**政策版本不回退覆盖**」
  - `:99`：「#1534（FU-11，事件 schema 须含 epoch、policy/model hash、前后状态以支持恢复）」
  - `:108`：「保持：#1534（FU-11）blocks #1537（依赖既存）」
- `docs/adr/0039-production-entry-inventory-and-unique-launcher.md:170`：「S/E/B/X 任一持久域的写者身份，即 `writer_epoch` 加 `.writer.lock`」。`:282`：「先确认旧 X 和它的发送子进程已退出，再提升 `writer_epoch`……旧 epoch 的写入被 `store.rs:466-467` 拒绝。」
- `docs/adr/0033-history-simulation-bar-batch-model.md:12`：「`intra_batch_order: Unknown`」。`:48`：「`model_id/revision/content_hash`；`input_domain`；`clock`（bar 索引、`known_frontier_cut`）」
- `docs/adr/0008-first-retrace-ledger-semantics.md`
  - `:11`：「门卫钟（last_as_of）倒退拒收 + 警报；每条修订带知情时（as_of）」
  - `:12`：「**append-only 修订日志 = 唯一真相**（CompletedFreezeReducer 先例：幂等重放、冲突拒绝）；状态 = 日志折叠……快照仅派生缓存（带溯源、可校验、非真相）」
  - `:24`：「**状态快照当真相持久化**：被否——双真相必漂移」
- `docs/adr/0007-n1-divergence-candidate-event.md:9`：「修订只追加、旧 revision 保留（禁删除模拟失效）」
- `docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md:22`：「`phase_events` append-only，AsKnown 不回填；更正类按 C08.6 以 RecomputedWithRevision 另列。」
- `docs/adr/0043-chong-reversal-account-continuity.md:27`：「### 三、latch 不回退；空头蕴含订正；开多求值点补发 `enterEarning`」
- `docs/adr/0010-chong-persistent-operating-unit.md:23`：「**重 = ⟨标的 S，操作级别 ℓ，专属筹码 Q⟩**，三要件缺一不可。」
- `docs/adr/0025-lav-shadow-ranking-evidence-contract.md:30-31`：「manifest 是按 RFC 8785（JCS）规范化的 UTF-8 JSON 字节串；证据对象是以 SHA-256 内容寻址的不可变 blobs」「文件路径、目录名、生成时间或封存时间均不得充当身份。」
- `docs/adr/0029-common-cap-contention-policy.md:35`：「记录形状 ⟨policy_id, version, cap, currency, valid_through, authorizer, authorized_at, profile_id⟩。注入/取出/改额都是新版本」
- `docs/adr/0049-funding-quantity-and-policy-reference-dimensions.md`
  - `:20`：「q_ref 沿用 funding-record 形状……q_ref 定位 profile 的授权身份，另绑定 version」
  - `:21`：「沿用现形状以避免无收益迁移，但固定版本不能在运行中换解析策略。」
  - `docs/adr/0040-funding-signature-and-chong-derivation.md:150` 给出实例：`q_ref: #1557/funding-authorization/v1@P0`，所指 funding-record 在 `:110` 起，含 `policy_id`、`version` 字段。
- `docs/adr/0012-inference-layer-document-naming.md:52-58` 裁定二：正本层分三处——`.chanlun/definitions/` 放缠论教义，`docs/adr/` 放架构决策，`CONTEXT.md` 放词。`:88` 起是裁定五（同名异物）。`:108`：「把词的对应写进 `CONTEXT.md`……（**正本层，允许改**），推论层原文一个字不动。」
- `docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md`
  - `:9` 裁定一：适用范围；
  - `:11` 裁定二：与 #799 的关系；
  - `:17` 裁定三：落法，即版本化记录字段、默认值、出处、改签即新版本；
  - `:27` 裁定四：合法候选保留，真错照旧排除。
- `.chanlun/definitions/chong-traceability.md`
  - `:1-3`：三行头，最后裁定 #1533；
  - `:13`：「身份、epoch、policy/model 版本、前后状态、撤回和续接的具体事件合同由 #1534 承接。本票不提前发明第二套事件 schema。」
  - `:27`：「空头事件不把 latch 推进到 EarningShares……以上直接沿用 ADR 0043 裁定三，不新设阶段。」
  - `:29`：「换向属于同一重的 Voice 换代。P、K、R、费用、phase_events、G 与 withdrawn 连续」
  - `:33`：「追溯必须区分重身份与 Voice 生命周期，不能拿 Voice 的退出清空重账。」
- `.chanlun/definitions/baohan.md:191`：「cut 按代编号（整合分支 `rust/src/bin/s_structure_session.rs:3349` `structure_cut = cut-{gen}`）」
- `CONTEXT.md`
  - `:88-89` 声部：「`v = (carrier, γ, σ, n)`。一经建立不可变，出场即终结。」
  - `:150-151` 代际：「声部在赋格树中的嵌套代数」
  - `:166`：「S 快照里跨代累计的六族数组按记录进入该族的代际升序」
  - `:202`：「随 `stroke::RULE`……进 S 发布的 `policy_version`；改动即开新版本，旧代际 AsKnown 不回填」
  - `:291-292`：背驰段候选事件，「塔内原生一等事件对象……ADR-0007」
  - `:445`：「`TStage` latch 只投影 campaign 内达标事件史（AsKnown 不回填）」
  - `:454`：「`(fill_allocation_policy_id, manifest_hash, policy_revision)` 三元组随每条 allocation 持久；更正与重放引用原版本的逆规则，换版本只能是显式迁移事件。」
  - `:36` 量档 policy：「q_ref 绑定资金授权身份及版本，不能作算术量（ADR 0049）」
- `AGENTS.md:46-62` 收敛通则（#799）与 `:64-107` 总缝规则（#804）：本票所有规则都是「同一个外壳、同一个判据」，各域之间的差异只体现在字段值上。

**已批准 SPEC #1340**（行号取自 `gh issue view 1340 --json body` 的输出；`:460` 有用户「批准」记录）

- `:259` C02.1：「标准输入包含 source namespace/epoch、instrument、原始事件身份和修订、原始坐标、received_at、逻辑输入顺序、替代关系及来源证据。」
- `:297` C06.4：「规范外部接纳先将原报和保守受影响范围的 Pending 同事务持久写入，再解释风险语义。」
- `:300` C06.7：「同事实/分配 ID 与重 ID 同内容只应用一次，异内容冲突……不能按内容去重误吞真实成交。更正按前件因果应用」
- `:307` C07.3：「共同市场证书带来源 namespace/epoch、事件与修订……」
- `:308` C07.4：「已有 ExecuteCommitted 只返回原收据，不铸第二份资格。」
- `:309` C07.5：「相同 ID 不因换 epoch/计划/证书重建，重复、并发与恢复仅查询旧状态。」
- `:319` C08.2：「正式结构快照固定 session/generation、structure_cut、catalog_revision、scope 和 index_frontier。分页不跨切面，结构 delta 带 base/next、持久序号和原子批次；恢复续接与同 cut 快照逐字段相等，缺口显式 Gap」
- `:321` C08.4：「跨域总览携各权威域前沿、consumed_structure、Pending、缺项及可信旧 cut。」
- `:323` C08.6：「恢复的晚到应用不能回填为故障当时已处理。RecomputedWithRevision 独立命名并绑定输入/规则」
- `:333` C09.4：「持久 delta 按独立保留策略存储，落后消费者收到 Gap 并从指定 cut 重建。」
- `:336` C09.7：「S/E/B 的写事务核数据库持久 writer_epoch」
- `:338` C09.9：「B 恢复重放原归属应用，不从场所净仓重建成本。」
- `:339` C09.10：「可迁往只读分段归档但查询必须覆盖全部有效代际……回收普通流游标不回收命令去重」
- `:340` C09.11：「发布新 writer_epoch 后旧写者不得再写。」
- `:416` AT-04、`:422` AT-10。

**代码先例**（只用于否决权和字段实证，不作口径依据）

- `rust/src/economic_session/store.rs`
  - `:46` `commits(seq, prev_root, root UNIQUE, commit_ns, image)`；
  - `:49` `applications(k, fact_key UNIQUE, json)`；
  - `:653-654` `StaleWriter：持久epoch不等于获准写者`；
  - `:1095-1097` `StorageUnavailable：Resolve 重放不等于完整提交图像`。
- `rust/src/economic_session/book.rs:92-99`：`fact_key = hash([source_namespace, source_epoch, external_fact_id])`。
- `rust/src/economic_session/service.rs:185-192`：cut 与 `as_known_ns` 互斥；cut 三元组；跨域报 `IdentityConflict`。
- `rust/src/economic_session/tb03c_book.rs`
  - `:10-11`：多头 ⇒ EarningShares，空头止于 CapitalRecovered；
  - `:585-596`：stamp 写入 `stable_point`、`commit_seq`、`fact_key`，并有 `{"kind":"growth_eligible","from":prev_status,"to":status}`，一次提交可以 push 多条 events；
  - `:632`：`LATCH[prev_rank.max(rank)]`。
- `rust/src/bin/s_structure_session.rs`
  - `:219-238`：`raw_events`，主键 `(identity_key, revision)`，含 `source_namespace/source_epoch/event_id/received_at/supersedes_revision`；
  - `:258-263`：`first_known_generation … withdrawn_generation, withdrawal_reason, superseded_by`；
  - `:295-307`：`structure_deltas`，含 `generation PRIMARY KEY, base_cut, next_cut, seq_range_json, index_frontier, policy_records`；
  - `:1099`、`:3676`：`cut-{…}`；
  - `:5506`：「重复（generation ≤ cursor）不回放；缺口显式 Gap 并给重建 cut」；
  - `:9257`、`:11556`：`DELETE FROM structure_deltas` 只出现在 `#[cfg(test)]` 模块内。
- `rust/src/economic_session/finance_tests.rs:791`：「原 Committed 幂等重送仍返回原永久收据，不被后来 Pending 倒写。」

**票面与评论**

- [#1534 评论 6404223521](https://github.com/xy7365527-lang/NewChanlun/issues/1534#issuecomment-6404223521)（xy7365527-lang，2026-10-25）：#1533 返修承接登记，「本票原有行为验收范围保持」。
- [#1534 评论 6428699494](https://github.com/xy7365527-lang/NewChanlun/issues/1534#issuecomment-6428699494)（2026-10-26）：「分级：T」。
- [#1533 评论 6428629112](https://github.com/xy7365527-lang/NewChanlun/issues/1533#issuecomment-6428629112)（2026-10-26）：FU-10 resolution，正本为 `chong-traceability.md`。
- [#1542 评论 6262301982](https://github.com/xy7365527-lang/NewChanlun/issues/1542#issuecomment-6262301982)（2026-10-19）：裁定票作废，「它对 FU 票的 blocking 边随关闭失效」。
- [#1323 评论 6261281111](https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111)（2026-10-19）：Notes N-1 原文。
- #1516 CLOSED（标题「消除 make_token() 时间戳/PID 依赖」）；#1371、#1372 CLOSED（S 侧原子撤回、固定 cut 分页与断线续接）；#1610 CLOSED（ADR 0049 驱动票）。

## 候选

13 个 B 类点按考古写成默认值，推理链已按考古重过一遍；用户 2026-10-27 批复「批准」，全部按默认值确认。A 类只有 Q13，直接沿用，不在这里列。

**Q1 外壳形状**
- 默认值：正本层定义统一外壳 `domain-event/1`，各域的既有载荷映射进来，不另造第二套 schema。
- 出处：ADR 0038:52/:99；chong-traceability.md:13；store.rs:46；s_structure_session.rs:219-307。
- 已确认（用户 2026-10-27 批复）。
- 推理链：ADR 0038 已把字段要求写进六面全状态，并指定由本票定形。B/E 和 S 两边的字段各自长成了，但从来没有合成一张合同。缺的是统一外壳，字段本身都有。

**Q2 「cut」指哪个**
- 默认值：外壳分两格。`cut` 是本域提交切面；`structure_cut` 是判定所依据的 S cut `cut-{gen}`，即 C08.4 的 consumed_structure。禁止拿 B/E 的 `commit_seq` 和 S generation 互相比较。
- 出处：baohan.md:191；service.rs:185-192；#1340 C07.3/C08.4。
- 已确认（用户 2026-10-27 批复）。
- 推理链：这两个实体在仓内都已存在，代码明令跨域 cut 报 IdentityConflict；C07.3 又禁止比较不相干的整数游标，所以只能分两格。

**Q3 「epoch」指哪个**
- 默认值：`writer_epoch` 和 `source.source_epoch` 两个字段并存，名字不共用。事件身份和 `fact_key` 都不含 `writer_epoch`；换代只改 `writer_epoch`，不重铸任何 ID。
- 出处：ADR 0039:170/:282；book.rs:92-99；#1340 C07.5/C09.7/C09.11。
- 已确认（用户 2026-10-27 批复）。
- 推理链：ADR 0038:52 里的「epoch」出现在恢复和换代语境，指写者纪元；而 fact 去重依赖的是来源纪元。两者混成一个名字，就会破坏 C07.5 的「相同 ID 不因换 epoch 重建」。

**Q4 policy/model hash 的形态**
- 默认值：`policy_refs[]` 里每项是 `policy_ref = {policy_id, version, profile_id|null, content_hash}`；`model_ref = {model_id, revision, content_hash}`；`content_hash` 默认取 JCS 规范化字节的 SHA-256。
- 出处：ADR 0029:35；ADR 0049:20-21；CONTEXT.md:454/:202；ADR 0033:48；ADR 0038:47/:76；ADR 0025:30-31。
- 已确认（用户 2026-10-27 批复）。
- 推理链：在案的政策记录全部采用「可读 id + 版本 (+ profile)」形状，见 ADR 0029:35、ADR 0034:14、ADR 0035:14。票面写的「hash」是简称。
- **与 q_ref 的关系**：两者是同一个引用。`q_ref` 是 `policy_ref` 在量档记录里的字符串写法。例如 `#1557/funding-authorization/v1@P0` 就是 `policy_id@profile_id`，`version` 按 ADR 0049:20 另外绑定。`policy_ref` 把这三项按 funding-record 自己的字段名（`policy_id`、`version`、`profile_id`）原样展开，只追加 `content_hash` 做内容绑定，不改 `q_ref` 的形状和语义。q_ref 不参与算术（CONTEXT.md:36）。
- 外效分配三元组也落到同一形状：`fill_allocation_policy_id → policy_id`，`policy_revision → version`，`manifest_hash → content_hash`。这样只有一套政策引用形状，各域原有写法都是它的投影。

**Q5 身份**
- 默认值分四层：
  - `identity.chong_id = (S, ℓ)`；
  - `identity.voice = (carrier, γ, σ, n)`，零筹码期为 `null`；
  - `identity.fact_key` 或 `identity.command_id`；
  - 事件自身 `event_id = (domain_id, commit_seq, ordinal)`。
  - 任何一层都不含 wall-clock 或 PID；Voice 退出不改 chong_id。
- 出处：ADR 0010:23；CONTEXT.md:89；chong-traceability.md:33；book.rs:92-99；#1340 C07.5/C09.10；#1516。
- 已确认（用户 2026-10-27 批复）。
- `ordinal` 是在考古默认值上补的：`tb03c_book.rs:585-596` 一次提交可以 push 多条事件，只用 `(domain_id, commit_seq)` 区分不了它们。

**Q6 前后状态怎么表达**
- 默认值：用 `prev_root`/`root` 加 `transition{field, from, to}`，再加 `cause[]`；不在事件里内联完整图像。
- 出处：ADR 0008:12/:24；store.rs:46/:1095-1097；tb03c_book.rs:595。
- 已确认（用户 2026-10-27 批复）。按 ADR 0041 落为可调字段 `state_carrier`，见裁定四。

**Q7 重复与乱序**
- 默认值：按身份去重，不按内容去重。
  - 同 ID 同内容：no-op，返回原 application；
  - 同 ID 异内容：持久写 `IdentityConflict`；
  - 乱序：到达即进入 Recorded/Pending，之后按 `cause[]` 前件因果应用；
  - 批内或跨重的先后：`Unknown`。
- 出处：#1340 C06.4/C06.7/AT-10；ADR 0008:11；ADR 0033:12；store.rs:49；s_structure_session.rs:219-238。
- 已确认（用户 2026-10-27 批复）。

**Q8 撤回与恢复幂等**
- 默认值：撤回就是追加一条 `withdrawn` 事件。恢复的判据有两条：重放返回原 application，且图像根逐位相等；否则报 `StorageUnavailable`。
- 出处：ADR 0008:12；ADR 0007:9；ADR 0038:60；s_structure_session.rs:258-263；store.rs:1095-1097；finance_tests.rs:791；#1371/#1516。
- 已确认（用户 2026-10-27 批复）。

**Q9 snapshot/delta 续接**
- 默认值：S、B、E 三域统一沿用 C08.2 的形状。delta 带 base/next/seq_range/原子批次。判据是续接结果与同 cut 快照逐字段相等。消费者落后时收到 Gap，旧代际和旧 `writer_epoch` 一律拒收。S 侧沿用 #1372 的实现。
- 出处：#1340 C08.2/C09.4/AT-04；ADR 0038:60-69；s_structure_session.rs:295-307/:5506；service.rs:185-187；#1372。
- 已确认（用户 2026-10-27 批复）。保留策略落为可调字段 `delta_retention`，见裁定四。

**Q10 持久边界**
- 默认值分三类：真相、可重算投影、AsKnown 事实。
- 出处：ADR 0008:12；ADR 0031:22；ADR 0038:73；CONTEXT.md:445；#1340 C08.6；tb03c_book.rs:632。
- 已确认（用户 2026-10-27 批复）。

**Q11 分词**
- 默认值：`CONTEXT.md` 新增「域事件」「写者纪元」「来源纪元」「结构代」四个词条；在「代际」和「背驰段候选事件」两个词条下各加一句 Avoid 译注；不改 ADR 0007 和 S 的既有字段名。
- 出处：CONTEXT.md:150/:166/:291；baohan.md:191；ADR 0012:108。
- 已确认（用户 2026-10-27 批复）。

**Q12 正本落点**
- 默认值：新建 ADR 0051，`CONTEXT.md` 只加 Q11 的词条，`chong-traceability.md` 不改。
- 出处：ADR 0012:52-58；同图先例 ADR 0038:1；chong-traceability.md:13。
- 已确认（用户 2026-10-27 批复）。

**Q14 范围**
- 默认值：本票只裁 S/B/E 的逐 bar 外壳及其规则。X 侧 `OwnAttempt/MayHaveCalled` 和 fill→Voice 归属只约定「套同一外壳并带 `command_id`」，实体规则归 #1330 和 #1539。#1539 消费本合同，补接 `#1534 blocks #1539`；#1330 票面没有对本票的依赖声明，不接边。
- 出处：决策地图 :50-51/:69；#1539 票面「依赖的 FU」节；#1340 C07.5；ADR 0038:52。
- 已确认（用户 2026-10-27 批复）。
- 考古更正：考古 Q14 的 repo 查询 `rg -n "OwnAttempt|MayHaveCalled" docs/adr/ .chanlun/definitions/` 记 6 条，在 main `bea0a169f9` 复跑为 0 条，连同 `CONTEXT.md` 也是 0 条。6 恰是 #1340 正文的命中行数，是把 tracker 命中记到了 repo 路下。正本层 0 条正好说明这里没有分工记录，划界结论不变。考古默认值里「blocking 边不变」与 #1539 票面的依赖声明相抵，改为补接这一条边。

## 裁定

本条依用户 2026-10-27 批复判定（P 类，见「授权引用」）。

正本落点：本文件 `docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md`，三行头见文首。

另外修改 `CONTEXT.md`，只加词条和 Avoid 译注。`.chanlun/definitions/chong-traceability.md` 不改。

### 一、域事件外壳 `domain-event/1` `[新缠论:选择]`

S、E、各 Bᵢ 的每条持久事件都套同一个外壳。X 侧的订单/成交原件作为一种 `kind`，也套这个外壳。各域的内部载荷放进 `payload`。

| 字段 | 定义 | 先例 |
|---|---|---|
| `contract` | 固定写 `domain-event/1`；改形状即新版本 | ADR 0041:17 |
| `domain_id` | 写入域：S / E / B⟨S,ℓ⟩ / X | service.rs:191 |
| `event_id` | `(domain_id, commit_seq, ordinal)`。S 的 `commit_seq` 取 `structure_deltas.generation`；`ordinal` 是同一次提交内的事件序号。不含 wall-clock 或 PID | #1516；ADR 0025:31；tb03c_book.rs:585-596 |
| `writer_epoch` | 写者纪元，即提交该事件时的域写者代数 | ADR 0039:170；C09.7 |
| `cut` | 本域提交切面。B/E/X 为 `{domain_id, commit_seq, root_hash}`；S 为 `{domain_id: S, structure_cut}` | service.rs:190；baohan.md:191 |
| `structure_cut` / `structure_generation` | 判定所依据的 S cut 及其整数代（即 consumed_structure）。S 自身事件取 `next_cut`。结构代只和 S generation 比较 | C08.2；C08.4；C07.3 |
| `prev_root` / `root` | 域图像哈希链 | store.rs:46 |
| `identity` | `{chong_id: (S,ℓ), voice: (carrier,γ,σ,n)\|null, fact_key\|command_id}`；Voice 退出不改 `chong_id` | ADR 0010:23；CONTEXT.md:89；chong-traceability.md:33；book.rs:92-99；C07.5 |
| `source` | `{source_namespace, source_epoch, external_fact_id, source_cursor, received_time}`，其中 `source_epoch` 是来源纪元 | C02.1；s_structure_session.rs:219-238 |
| `policy_refs[]` | 每项为 `{policy_id, version, profile_id\|null, content_hash}` | ADR 0029:35；ADR 0049:20；CONTEXT.md:454 |
| `model_ref` | `{model_id, revision, content_hash}`；没有模型参与的域写 `null` | ADR 0033:48；ADR 0038:47 |
| `transition` | `{field, from, to}` 或 `null`；只有状态机字段填写（latch、growth_eligible、Voice 状态、withdrawn） | tb03c_book.rs:595 |
| `cause[]` | 前件 `event_id` 或 `fact_key` | ADR 0038:52；C06.7 |
| `kind` / `payload` | 域内事件种类与载荷 | 各域既有 |

**`policy_ref` 和既有引用的对应关系**（不另起第二套政策引用形状）：

- 资金和量档记录：`policy_ref` 的 `policy_id`、`version`、`profile_id` 原样取 `q_ref` 所指 funding-record 里的同名字段（ADR 0029:35；ADR 0040:110 起）。`q_ref` 字符串 `policy_id@profile_id`（ADR 0040:150）就是它的投影，version 另绑（ADR 0049:20）。
- 外效分配：`fill_allocation_policy_id → policy_id`，`policy_revision → version`，`manifest_hash → content_hash`（CONTEXT.md:454）。
- S 的 `structure_deltas.policy_records`：每条记录映射成一个 `policy_ref`，S 的字段名不改。

一个事件绑定的版本在运行中不换，换版本只能走显式迁移事件（ADR 0049:21；CONTEXT.md:454；ADR 0038:76）。

**分词**（落 `CONTEXT.md`）：
- 「写者纪元」`writer_epoch` 和「来源纪元」`source_epoch` 不共用名字。事件身份和 `fact_key` 都不含 `writer_epoch`；换代只改它，不重铸任何 ID（C07.5、C09.11）。
- 「结构代」`structure_generation` 和「代际」`voice.n` 不共用名字。
- 「域事件」和 ADR 0007 的「背驰段候选事件」不共用名字。后者是 S `payload` 里的内容之一，不是外壳。

### 二、重复、乱序与因果 `[新缠论:推论]`

1. 去重键是身份，不是内容：外部事实用 `fact_key`，S 输入用 `(identity_key, revision)`，命令用 `command_id`（C06.7）。
2. 同 ID 同内容重投：no-op，返回原 application 或原收据（C07.4；finance_tests.rs:791）。同 ID 异内容：持久写 `IdentityConflict`，不吞、不覆盖、不自动择一。
3. 乱序到达：先在同一事务里持久写成 Recorded/Pending（C06.4），用 `received_time` 和 `source_cursor` 记下原始顺序。应用时按 `cause[]` 前件因果执行；缺前件就留在 Pending，不丢。门卫钟倒退拒收（ADR 0008:11）。
4. 跨重和批内的先后一律声明为 `Unknown`，不作比较维度（ADR 0033:12；ADR 0038:56）。

### 三、撤回与恢复幂等 `[新缠论:推论]`

1. 撤回就是追加一条 `kind: withdrawn` 域事件：`payload` 带 `withdrawn_generation / withdrawal_reason / superseded_by`，`transition = {withdrawn, null, <generation>}`。旧行、旧 cut、旧 AsKnown 都不动（ADR 0007:9；ADR 0008:12；s_structure_session.rs:258-263）。
2. 恢复就是从最后一个 checkpoint 加载快照，再重放日志（ADR 0038:60）。幂等要求两条同时成立：
   - (a) 重放已应用过的 `fact_key`/`command_id` 时返回原 application，`applications.fact_key UNIQUE` 不新增行；
   - (b) 重放得到的 `root` 与原 commit 的 `root` 逐位相等。

   任一条不成立就报 `StorageUnavailable` 并停止写入（store.rs:1095-1097），不走任何备用路径。
3. 旧 `writer_epoch` 写入时报 `StaleWriter` 并拒绝（store.rs:653-654；ADR 0039:282）。
4. 恢复后晚到的应用不回填成故障当时已处理。重算结果以 `RecomputedWithRevision` 另列，并绑定当时的 `policy_refs`/`model_ref`（C08.6；ADR 0031:22）。

### 四、snapshot/delta 续接（三域统一） `[新缠论:选择]`

1. snapshot 钉住 `{domain_id, writer_epoch, cut, structure_cut, rule/catalog_revision, scope, frontier, policy_refs, model_ref}`。
2. delta 带 `base_cut`、`next_cut`、`seq_range` 和原子批次 id。S 现行的 `structure_deltas` 就是这个形状。
3. 续接判据：从 snapshot(base) 应用 delta 后的状态，必须与 snapshot(next) 逐字段相等（C08.2）；不等就拒绝续接并报 `Gap`。
4. 消费者游标落后于保留窗时，显式返回 `Gap` 并给出重建 cut（C09.4；s_structure_session.rs:5506）。旧代际和旧 `writer_epoch` 拒收。`cut` 与 AsKnown 不能在同一查询里同时指定（service.rs:185-187）。
5. S 侧沿用 #1372 的实现；B/E 侧按本协议补 delta 读口和 Gap。

**可调字段**（ADR 0041：落在 `domain-event/1` 合同记录里，事前单选，改值即新版本，运行中只走一个值）

| 字段 | 默认值 | 出处 | 可选值与代价 |
|---|---|---|---|
| `state_carrier` | `root_chain+transition` | ADR 0008:12/:24；store.rs:46/:1095-1097 | `inline_image`：每条事件内联完整的前后图像。好处是单条事件自足可读；代价是存储随事件数线性膨胀，与 `commits.image` 重复，还要另加「内联图像 = 重放结果」的校验。内联图像始终是派生物，不当真相 |
| `content_hash_algo` | `jcs-sha256` | ADR 0025:30-31 | 其他内容寻址摘要（如 blake3）。代价是与 ADR 0025 的证据 bundle 不同构，跨正本比对要维护两套摘要 |
| `delta_retention` | `retain_all` | 生产路径没有回收（`DELETE FROM structure_deltas` 只出现在测试模块）；C09.4 允许独立的保留策略 | `segment_archive`：按 C09.10 迁入只读分段归档。代价是查询必须覆盖全部有效代际；归档不可读时，不得把历史 ID 当成新 ID |

这三个字段都不是同一判断的宽严两档：它们不接在同一个 if-else 上，也没有「主值失败就换备用值」的路径。

**排除（真错）**：
- 按内容去重：违反 C06.7 原文。
- 撤回 = 删行：违反 ADR 0007:9 和 ADR 0008:12。
- `event_id` 或幂等键含时间戳/PID：违反 #1516 的已关裁定。
- 一个 `epoch` 字段同时指写者和来源：违反 C07.5。
- 一个 `cut` 字段同时指 S 代和域提交：违反 C07.3，且 service.rs:192 会报 IdentityConflict。
- 另立一套与 `q_ref` 不对应的政策引用形状：违反 ADR 0049:20-21。

### 五、持久边界 `[新缠论:推论]`

1. **真相**（append-only）：`facts / applications / commits` 根链，以及 S 的 `raw_events / structure_deltas`。
2. **可重算投影**：book image、`growth_eligible`、毛账合计。可以物化，但审计时必须等于重放结果。
3. **AsKnown 事实**（作为事件持久，不当投影）：`phase_events`/latch（首次达标 bar + 证据指纹，ADR 0038:73）、`first_known_cut`、`withdrawn_generation`。重算结果只能以 `RecomputedWithRevision` 另列（C08.6）。

### 六、沿用 A 类（Q13）

空头事件不把 latch 推进到 EarningShares。换向属于同一重的 Voice 换代，P、K、R、费用、`phase_events`、G 与 withdrawn 都连续（chong-traceability.md:27/:29；ADR 0043:27 裁定三；tb03c_book.rs:10-11/:632）。本合同不新设阶段事件，`transition` 只记录既有的 LegalTransition。

### 七、范围

X 侧 `OwnAttempt / MayHaveCalled` 和 fill→Voice 归属的实体规则归 #1330、#1539；本合同只要求它们套 `domain-event/1` 外壳并带 `command_id`。#1539 票面已声明依赖 FU-11，裁定时要引用本合同，因此接 `#1534 blocks #1539`；#1330 票面没有这一依赖，不接边。`policy_refs` 只定引用的形状，不定任何签署值。资金、量档、cap、θ 都不在本票范围内。

## 验收清单

- [ ] `ls docs/adr/*-per-bar-domain-event-identity-lifecycle-contract.md | wc -l` 输出 `1`。
- [ ] `grep -c '^> 正本：本文件是逐 bar 域事件外壳' docs/adr/*-per-bar-domain-event-identity-lifecycle-contract.md` 输出 `1`。
- [ ] `grep -c 'domain-event/1' docs/adr/*-per-bar-domain-event-identity-lifecycle-contract.md` 输出 ≥1；`grep -c 'q_ref' docs/adr/*-per-bar-domain-event-identity-lifecycle-contract.md` 输出 ≥1（policy_ref 与 q_ref 的对应已写入正本）。
- [ ] `grep -c '^\*\*域事件（Domain Event）\*\*\|^\*\*写者纪元（Writer Epoch）\*\*\|^\*\*来源纪元（Source Epoch）\*\*\|^\*\*结构代（Structure Generation）\*\*' CONTEXT.md` 输出 `4`。
- [ ] `sed -n '/^\*\*代际（Generation, n）\*\*/,/^$/p' CONTEXT.md | grep -c 'structure_generation'` 输出 ≥1；`sed -n '/^\*\*背驰段候选事件/,/^$/p' CONTEXT.md | grep -c '域事件'` 输出 ≥1。
- [ ] `git diff --name-only origin/main...decision/1534 -- .chanlun/definitions/chong-traceability.md` 输出为空。
- [ ] 依赖边已接：`gh api repos/xy7365527-lang/NewChanlun/issues/1539/dependencies/blocked_by --jq '[.[]|select(.number==1534)]|length'` 输出 `1`。
- [ ] 票状态：`gh issue view 1534 --repo xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。
- [ ] `gh api repos/xy7365527-lang/NewChanlun/issues/1534/comments --jq '[.[]|select(.body|startswith("用户 2026-10-27 批复"))]|length'` 输出 ≥1（P 类批复依据，取代候选稿的 N-1 判据）。
- [ ] 实施票已开：`gh issue list --repo xy7365527-lang/NewChanlun --state all --search '"domain-event/1" in:body "[impl]" in:title' --json number --jq length` 输出 ≥1。

## 受影响票与 blocking 边变化

**受影响票**

- #1360（TB-02 汇总）、#1361（TB-03 汇总）、#1363（TB-05 汇总）、#1366（TB-07 汇总）：目前被 #1534 阻塞；本票关闭后这四条边自动解除。
- #1536（FU-14，OPEN）：被 #1534 阻塞，关票后解除。FU-14 的模型 profile 须使用本合同的 `model_ref` 形状。
- #1537（FU-15，CLOSED）：ADR 0038:108「保持：#1534 blocks #1537」。#1537 已关，这条边随之失效，不改。
- #1533、#1542（CLOSED）：对本票的 blocked-by 边已随关闭失效，不改。
- #1381、#1399、#1411、#1418（TB-07-A/B/C/D）：恢复、seek、双跑须按本合同的外壳和幂等判据实现。不新增边。
- #1406、#1415（TB-03-C/D）：去重键和 `IdentityConflict` 按裁定二执行。不新增边。
- #1396、#1409、#1423（TB-05-B/C/E）：旧写者拒收和非空恢复按裁定三执行。不新增边。
- #1539（FU-17，OPEN）：票面「依赖的 FU」节列有「FU-11（#1534）」，决策地图 :69 同列；裁定时须引用 ADR 0051 的外壳并带 `command_id`。tracker 里 #1539 的 blocked-by 只有 #1538，**新增 `#1534 blocks #1539`**。这条边不成环：#1534 的 blocked-by 只有已关的 #1533、#1542。它反映的是 #1539 票面早已声明的依赖，不取决于本裁定的内容；主控已于本票待批期间接上（[#1539 评论 6429488851](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6429488851)），落地不重复接。
- #1330（OPEN）：划界见裁定七，裁定时须引用 ADR 0051 的外壳并带 `command_id`。票面没有对本票的依赖声明，不新增边。
- #1610（CLOSED，ADR 0049）：`q_ref` 形状保持不变，本合同的 `policy_ref` 是它的字段展开。不改。
- `.chanlun/definitions/baohan.md:191` 的代码指针漂移（`:3349` 应为 `:1099`/`:3676`）：登记在 ADR 0051 的「越界登记」里，留给下次修订 baohan.md 的票顺手订正。

**要开的票**

- 1 张 `[impl]`「B/E 域事件外壳 `domain-event/1` 映射与 delta/Gap 读口」，作为 #1366 的子票。正文写入以下 7 个测试名作为该票验收，判据是 `cargo test --features s_session <名>` 通过：
  1. `domain_event_envelope_rejects_missing_required_fields`
  2. `domain_event_same_fact_key_same_content_noop_returns_original_application`
  3. `domain_event_same_fact_key_different_content_persists_identity_conflict`
  4. `domain_event_withdrawn_is_appended_and_old_rows_unchanged`
  5. `recovery_replay_root_equals_original_commit_root_or_storage_unavailable`
  6. `stale_writer_epoch_rejected_and_fact_key_unchanged_across_epoch_bump`
  7. `be_snapshot_delta_resume_equals_same_cut_snapshot_fieldwise_and_gap_on_lag`

  另加 1 条身份确定性测试 `event_id_fact_key_command_id_bitwise_equal_across_process_and_time`：固定输入，在两个独立进程、不同时刻分别计算 `event_id`/`fact_key`/`command_id`，结果须逐位相等。这条替代上一稿里对 store.rs 的 grep 判据。
- 无 research 票，无 probe 票（考古 C 类为 0）。

**blocking 边增删汇总**

- 增 3 条：#1534 blocks #1539（已接）；新 impl 票 blocks #1361；新 impl 票 blocks #1363（后两条随 impl 票开票时接）。另将新 impl 票挂为 #1366 的 sub-issue。
- 删 0 条。#1534 → #1360/#1361/#1363/#1366/#1536/#1539 这 6 条边由关票自动解除。

**受影响代码**（行号固定到 main `da03a10ee0`；真改码进上列实施票，本 ADR 零代码改动）

- B/E 外壳映射与读口：`rust/src/economic_session/store.rs:46`（commits 根链）、`:49`（`applications.fact_key UNIQUE`）、`:653-654`（StaleWriter）、`:1095-1097`（重放不等即 StorageUnavailable）；`rust/src/economic_session/book.rs:92-99`（`fact_key` 构成，保持不变）；`rust/src/economic_session/service.rs:185-192`（cut 查询，按裁定四补 delta 读口与 Gap）；`rust/src/economic_session/tb03c_book.rs:585-596`（一次提交多条事件，映射时补 `ordinal`）、`:632`（latch 单调）。
- S 侧：`rust/src/bin/s_structure_session.rs:219-238`（raw_events）、`:258-263`（撤回字段）、`:295-307`（structure_deltas，含 `policy_records`）、`:5506`（重复不回放、缺口显式 Gap）。字段名不改，按裁定一映射进外壳；续接沿用 #1372 的实现。
- Python 读者（`s_session/` 下非测试模块，按读取字段列出函数区间；均为兼容核对点，不是本 ADR 的修改点）：
  - `s_query.py:212-230` `validate_envelope`（`source_epoch`/`producer_epoch` 信封）、`:237-252` `_order`（`fact_key` 排序与撤回代）、`:417-476` `watch_page`（`deltas`、`next_cursor`、`gap`）、`:479-490` `response_envelope`。
  - `economic_query.py:39-47` `verify_cut`（`domain_id`/`commit_seq`/`root_hash`）、`:164-212` `_original_event`（`fact_key` 来源身份核验）、`:215-245` `_correction_predecessor`（`original_fact_key`）、`:248-365` `verify_fill_allocations`（按 `fact_key` 去重与对账）。
  - `s_readonly_server.py:168-174` `_verify_batch_profile`（`raw_events`）、`:177-187` `_profile_at_cut`、`:292-327` 按 generation 读 `structure_deltas` 的三个函数、`:373-395` `_read_objects_view`（`withdrawn_generation`/撤回）、`:403-534` `_read_snapshot_in_tx`（`raw_events` 的 `source_epoch`/`event_id`）。
  - `s_socket_client.py:33-46` `reply_envelope`、`:75-110` `validate_reply`（生产者 epoch）、`:277-282` `_page_cut`、`:290-308` `_page_changes`（`commit_seq` 连续）、`:311-372` `_verify_page_document`（`writer_epoch` 与固定 cut）。
  - `s_query_integrity.py:37-81` `LEGACY_SCHEMA`（`raw_events`、`structure_deltas`、`writer_epoch_history` 表结构）、`:472-553` `evidence_sources`、`:565-594` `materialize_graph_originals`（`delta_json`）、`:731-756` `capture_digest`。
  - `economic_http.py:10-11` `HEADER`、`:14-33` `validate_request`（信封 epoch 与因果引用）。
  - `economic_service_control.py:19-58` `read_config`（各域 epoch 与源身份）、`:83-124` `start_owner`（回复 `writer_epoch`）。
  - `s_service_control.py:61-93` `read_config`（`writer_epoch`）、`:552-567` `ready_request`、`:569-621` `wait_ready`（`cut` 与 `producer_epoch`）、`:763-792` `recover`（新 epoch）。
  - `s_tb02_contract.py:61-62` `FACT_COLUMNS`/`FACT_FIELDS`、`:325-382` `_validate_fact`（`fact_key`、撤回生命周期）。

  本裁定不改既有字段名；实施票改 B/E 读口时须逐一核这些读者的兼容，本 ADR 不声称零影响。
- Lean：`rg -l 'structure_deltas|fact_key|writer_epoch' -g '*.lean'` 在 `da03a10ee0` 上 0 命中。

## 越界登记

- `.chanlun/definitions/baohan.md:191` 的代码指针 `rust/src/bin/s_structure_session.rs:3349` 已漂移：main `da03a10ee0` 上该行是 `);`，`cut-{…}` 实际在 `:1099`、`:3676`。本 ADR 不改 baohan.md，留给下次修订 baohan.md 的票顺手订正。

## 授权引用

本条依用户 2026-10-27 批复判定。主控把返修候选稿与异家族独审 PASS 交用户后，用户原话「批准」（[#1534 评论 6446727344](https://github.com/xy7365527-lang/NewChanlun/issues/1534#issuecomment-6446727344)）。批准范围是 `domain-event/1` 统一事件合同与 13 项有先例默认值，三个自由度保持版本化可选；不包含 main 合入、真实资金启用或其他待批门。

流程留痕：分级评论 6428699494 判 T，依据是票面三项全属 graph-engineering.md T 类所列「状态机与事件 schema」「恢复语义」「接口与 seam」。decision-claimer 两轮独审均 BLOCK（第 1 轮 R1/R3/R8，第 2 轮 R6/R8），按 graph-engineering.md 第 4 步「再 BLOCK 转 P 类」转 P 并挂 `needs-approval`；返修稿补依赖与引用后，异家族独审 R1–R8 全 PASS。题目仍是技术取舍，转 P 来自流程升级，所以本条以用户批复为授权，#1323 Notes N-1（用户 2026-10-18 原话「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」，#1323 评论 6261281111）不作为本条依据。

本合同没有夹带资金、量档、cap、场所或 Destination 的改写；`policy_refs`/`model_ref` 只定引用形状，不定任何签署值。

## 引用自证

| 引用 | 核实命令 | 结果摘要 |
|---|---|---|
| AGENTS.md:46-62 / :64-107 / :129-130 | Read AGENTS.md | 依次是 #799 收敛通则节、#804 总缝规则节、引原文四步 |
| graph-engineering.md:23 | Read .sandcastle/docs/graph-engineering.md | 「T 类（技术取舍）：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义」 |
| ADR 0038:1/47/52/56/60/62/69/73/76/99/108 | `for n in 1 47 52 56 60 62 69 73 76 99 108; do sed -n ${n}p docs/adr/0038-…md; done` | 每行原文与正文所引逐字一致 |
| ADR 0039:170/:282 | `sed -n 170p; sed -n 282p docs/adr/0039-*.md` | 「`writer_epoch` 加 `.writer.lock`」；「再提升 `writer_epoch`……旧 epoch 的写入被 `store.rs:466-467` 拒绝」 |
| ADR 0033:12/:48 | `sed -n 12p; sed -n 48p docs/adr/0033-*.md` | 含 `intra_batch_order: Unknown`；「`model_id/revision/content_hash`；…`known_frontier_cut`」 |
| ADR 0008:11/:12/:24 | `sed -n 11,12p docs/adr/0008-first-retrace-ledger-semantics.md`；`rg -n "快照仅派生缓存\|日志折叠\|唯一真相" docs/adr/` | :11 门卫钟倒退拒收；:12 「append-only 修订日志 = 唯一真相」；:24 「状态快照当真相持久化：被否——双真相必漂移」（该 rg 共 3 条命中） |
| ADR 0007:9 | `sed -n 9p docs/adr/0007-*.md` | 「修订只追加、旧 revision 保留（禁删除模拟失效）」 |
| ADR 0031:22 | `sed -n 22p docs/adr/0031-*.md` | 「`phase_events` append-only，AsKnown 不回填；更正类按 C08.6 以 RecomputedWithRevision 另列」 |
| ADR 0043:27 | `sed -n 27p docs/adr/0043-*.md` | 「### 三、latch 不回退；空头蕴含订正；开多求值点补发 `enterEarning`」 |
| ADR 0010:23 | `sed -n 23p docs/adr/0010-*.md` | 「**重 = ⟨标的 S，操作级别 ℓ，专属筹码 Q⟩**，三要件缺一不可」 |
| ADR 0025:30-31 | `sed -n 30,31p docs/adr/0025-*.md` | RFC 8785（JCS）+ SHA-256；「文件路径、目录名、生成时间或封存时间均不得充当身份」 |
| ADR 0029:35 | `grep -n "policy_id, version, cap, currency" docs/adr/0029-*.md` | :35「记录形状 ⟨policy_id, version, cap, currency, valid_through, authorizer, authorized_at, profile_id⟩」 |
| ADR 0049:19-21 | `sed -n 19,21p docs/adr/0049-*.md` | :19「### Q2：引用与逐重数值」；:20「q_ref 定位 profile 的授权身份，另绑定 version」；:21「固定版本不能在运行中换解析策略」 |
| ADR 0040:110/:150 | `sed -n 104,130p; sed -n 145,152p docs/adr/0040-*.md` | :110 `policy_id: #1557/funding-authorization/v1`，下接 `version: v1`；:150 `q_ref: #1557/funding-authorization/v1@P0` |
| ADR 0034:14、ADR 0035:14 | `sed -n 14p docs/adr/0034-*.md docs/adr/0035-*.md` | 两处记录形状都以 ⟨policy_id, version, profile_id, …⟩ 开头 |
| ADR 0012:52-58/:88/:108 | `grep -n "^### " docs/adr/0012-*.md`；`sed -n 52,60p; sed -n 108p` | 裁定二三段式；:88 「### 五、…同名异物」；:108 「把词的对应写进 `CONTEXT.md`……（正本层，允许改）」 |
| ADR 0041:9/:11/:17/:27 | `grep -n "一、适用范围\|二、与 \|三、落法\|四、合法候选保留" docs/adr/0041-*.md` | 四条裁定分别在 :9、:11、:17、:27 |
| chong-traceability.md:1-3/:13/:27/:29/:33 | `sed -n 1,40p .chanlun/definitions/chong-traceability.md`；`grep -n "" … \| sed -n '13p;25p;27p;29p;33p'` | 三行头；:13 「由 #1534 承接。本票不提前发明第二套事件 schema」；:27 「空头事件不把 latch 推进到 EarningShares……沿用 ADR 0043 裁定三」；:29 换向连续；:33 区分重身份与 Voice 生命周期（:25 为 TStage latch 句，已避开） |
| baohan.md:191 | `sed -n 191p .chanlun/definitions/baohan.md` | 「cut 按代编号（整合分支 `rust/src/bin/s_structure_session.rs:3349` `structure_cut = cut-{gen}`）」 |
| CONTEXT.md:36/88-89/150-151/166/202/291-292/445/454 | `sed -n …p CONTEXT.md` 逐行查看 | 每行原文与正文所引一致 |
| CONTEXT.md 无 epoch/域事件词条 | `grep -n -i "epoch\|域事件\|写者纪元\|来源纪元\|结构代" CONTEXT.md` | 零命中 |
| #1340 :259/:297/:300/:307/:308/:309/:319/:321/:323/:333/:336/:338/:339/:340/:416/:422/:460 | `gh issue view 1340 --json body --jq .body > /tmp/b1340.md; sed -n …p` | 每条逐字可见；:333 的完整行含「持久 delta 按独立保留策略存储，落后消费者收到 Gap」；:460 为用户「批准」记录；#1340 OPEN |
| #1534 评论 6404223521 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/6404223521` | xy7365527-lang，2026-10-25T22:24Z：「…本票原有行为验收范围保持…」 |
| #1534 评论 6428699494 | `gh api …/issues/comments/6428699494` | xy7365527-lang，2026-10-26T22:20Z：「分级：T — 票面三项裁决…」 |
| #1533 评论 6428629112 | `gh api …/issues/comments/6428629112` | xy7365527-lang，2026-10-26T22:16Z：「本条依 #1323 Notes N-1 技术类预授权判定…正本：`.chanlun/definitions/chong-traceability.md`」 |
| #1542 评论 6262301982 | `gh api …/issues/comments/6262301982` | xy7365527-lang，2026-10-19T07:46Z：「作废关闭；它对 FU 票的 blocking 边随关闭失效」 |
| #1323 评论 6261281111 | `gh api …/issues/comments/6261281111` | xy7365527-lang，2026-10-19T06:38Z：「用户 2026-10-18 原话：『其实很多技术的东西 完全不需要我来做 我完全信任你的判断』」 |
| #1534 blocked-by/blocking | `gh issue view 1534 --repo xy7365527-lang/NewChanlun` | blocked-by #1542、#1533；blocking #1366、#1363、#1361、#1360、#1537、#1536；OPEN（返修稿时点；其后主控补接 #1539） |
| 各票状态 | `for n in 1323 1330 1360 … 1610; do gh issue view $n --json number,state,title; done` | #1371/#1372/#1516/#1533/#1537/#1542/#1610 CLOSED；#1323/#1330/#1360/#1361/#1363/#1366/#1381/#1396/#1399/#1406/#1409/#1411/#1415/#1418/#1423/#1536/#1539 OPEN；标题与正文所述一致 |
| 决策地图 :50-51/:69 | `sed -n '50,51p;69p' /Volumes/AgentStorage/decision-map-1323-倒推-20261018.md` | :50 「### FU-11…」；:51 「…阻塞 TB-02/03/05/07；依赖 FU-06/10」；:69 FU-17「…阻塞 TB-04/07/08/09；依赖 FU-03/04/11/16」 |
| #1539 票面依赖与边 | `gh issue view 1539 --repo xy7365527-lang/NewChanlun --json body`；`gh api repos/xy7365527-lang/NewChanlun/issues/1539/dependencies/blocked_by` | 「依赖的 FU」节：「FU-03、FU-04、FU-11（#1534）、FU-16（#1538）」；blocked_by 只有 #1538（OPEN）；`#1534 blocks #1539` 未接（返修稿时点；2026-10-26T23:09:40Z 已补接） |
| #1330 无依赖声明 | `gh issue view 1330 --repo xy7365527-lang/NewChanlun --json body --jq .body \| grep -c -E "1534\|FU-11\|依赖"` | `0` |
| 正本层无 X 侧分工记录（Q14） | `git grep -n -E "OwnAttempt\|MayHaveCalled" bea0a169f9 -- docs/adr .chanlun/definitions CONTEXT.md`；`gh issue view 1340 --json body --jq .body \| grep -n -E "OwnAttempt\|MayHaveCalled"` | 正本层 0 条；#1340 正文 6 行（:200/:309/:310/:311/:337/:421） |
| store.rs:46/:49/:653-654/:1095-1097 | `sed -n 46p; 49p; 653,654p; 1095,1097p rust/src/economic_session/store.rs` | commits 表；applications 表（fact_key UNIQUE）；StaleWriter；StorageUnavailable 重放不等 |
| book.rs:92-99 | `sed -n 92,99p rust/src/economic_session/book.rs` | `fact_key` = hash([source_namespace, source_epoch, external_fact_id]) |
| service.rs:185-192 | `sed -n 185,192p rust/src/economic_session/service.rs` | cut 与 AsKnown 互斥；`exact(wanted, &["domain_id","commit_seq","root_hash"])`；`IdentityConflict：跨域cut` |
| tb03c_book.rs:10-11/:585-596/:632 | `sed -n 10,11p; 585,596p; 632p` | 单向不变式注释；stamp 闭包与 `events.push(stamp(json!({"kind":"growth_eligible","from":prev_status,"to":status})))`；`LATCH[prev_rank.max(rank)]` |
| s_structure_session.rs:219-238/:258-263/:295-307 | `sed -n 219,238p; 258,263p; 295,307p` | raw_events（PK identity_key+revision）；withdrawn_generation/withdrawal_reason/superseded_by；structure_deltas |
| s_structure_session.rs:1099/:3349/:3676/:5506 | `sed -n 1099p; 3349p; 3676p; 5506p` | `format!("cut-{generation}")`；`);`；`&format!("cut-{gen}")`；「重复…不回放；缺口显式 Gap 并给重建 cut」 |
| s_structure_session.rs:9257/:11556 只在测试内 | `grep -n "DELETE FROM structure_deltas"`；`grep -n "^mod \|^#\[cfg(test)\]$"` | 只有这两处；分别位于 :5799 `#[cfg(test)] mod tests` 与 :11068 `#[cfg(test)] mod staged_graph_audit_regressions` 内 |
| finance_tests.rs:791 | `sed -n 791p rust/src/economic_session/finance_tests.rs`（候选稿基线 bea0a169f9 为 :754） | 「// 原 Committed 幂等重送仍返回原永久收据，不被后来 Pending 倒写。」 |
| feature `s_session` | `grep -n "s_session" rust/Cargo.toml`；`grep -n "pub mod economic_session" -B1 rust/src/lib.rs` | Cargo.toml:83 定义 feature；lib.rs:2915-2916 `#[cfg(feature = "s_session")] pub mod economic_session;` |
| grilling/domain-modeling SKILL | `ls .agents/skills/grilling/SKILL.md .agents/skills/domain-modeling/SKILL.md` | 两个文件都存在 |

## 考古对照

| 点 | 归类 | 出处 | 草稿处置 |
|---|---|---|---|
| Q1 外壳形状 | B | ADR 0038:52/:99；chong-traceability.md:13；store.rs:46；s_structure_session.rs:219-307 | 裁定一：`domain-event/1`，默认值待确认 |
| Q2 cut 指哪个 | B（冲突） | baohan.md:191；service.rs:185-192；C07.3/C08.4 | 裁定一：`cut` 与 `structure_cut` 分两格；冲突写进「问题」1 |
| Q3 epoch 指哪个 | B（冲突） | ADR 0039:170/:282；book.rs:92-99；C07.5/C09.7 | 裁定一：写者纪元与来源纪元分名；冲突写进「问题」2 |
| Q4 policy/model hash 形态 | B | CONTEXT.md:454/:202；ADR 0033:48；ADR 0038:47/:76；ADR 0025:30-31；**补 ADR 0049:19-21、ADR 0029:35、ADR 0040:110/:150**（考古漏列，独审 R8 指出） | 裁定一：`policy_refs[]` 每项 = `{policy_id, version, profile_id, content_hash}`；写明 q_ref 和分配三元组都是它的投影，不形成第二套；`content_hash_algo` 为可调字段 |
| Q5 身份 | B | ADR 0010:23；CONTEXT.md:89；chong-traceability.md **:33**（考古误记 :37）；book.rs:92-99；#1516 | 裁定一：`identity` 三层 + `event_id` 加 `ordinal` |
| Q6 前后状态 | B | ADR 0008:12/:24；store.rs:46/:1095-1097；tb03c_book.rs:595（考古 rg 称 1 条，复跑得 3 条） | 裁定一 + 四：`state_carrier` 可调，`inline_image` 保留为可选值 |
| Q7 重复与乱序 | B（范围冲突） | C06.4/C06.7/AT-10；ADR 0008:11；ADR 0033:12；store.rs:49 | 裁定二；按内容去重排除；X 侧归 #1330/#1539 |
| Q8 撤回与恢复幂等 | B | ADR 0008:12；ADR 0007:9；ADR 0038 **:60**（考古误记 :66）；s_structure_session.rs:258-263；store.rs:1095-1097；#1371/#1516 | 裁定三；删行式撤回排除 |
| Q9 snapshot/delta 续接 | B | C08.2/C09.4/AT-04；ADR 0038 **:60-69**（考古误记 :68-73）；structure_deltas；#1372 | 裁定四：三域统一；`delta_retention` 可调 |
| Q10 持久边界 | B | ADR 0008:12；ADR 0031:22；ADR 0038 **:73**（考古误记 :78）；CONTEXT.md:445；C08.6 | 裁定五：真相 / 投影 / AsKnown 三类 |
| Q11 分词 | B（冲突） | CONTEXT.md:150/:166/:291；baohan.md:191；ADR 0012:108 | 裁定一末段：四个词条 + 两处 Avoid 译注；冲突写进「问题」3 |
| Q12 正本落点 | B | ADR 0012:52-58；ADR 0038:1；chong-traceability.md:13 | 正本落点 ADR 0051；CONTEXT.md 加词；chong-traceability.md 不改 |
| Q13 空头/换向承接 | A | chong-traceability.md **:27/:29**（考古误记 :25/:27）；ADR 0043:27；tb03c_book.rs:10-11/:632 | 裁定六直接沿用，不列为待决 |
| Q14 范围边界 | B（冲突） | 决策地图 :50-51/:69；#1539 票面「依赖的 FU」节；#1330/#1539 OPEN；ADR 0038:52；C07.5；repo `rg OwnAttempt\|MayHaveCalled` 正本层 **0 条**（考古记 6，main bea0a169f9 复跑 0；6 是 #1340 正文命中行数） | 裁定七；补接 `#1534 blocks #1539`，#1330 不接边；冲突写进「问题」5 |
