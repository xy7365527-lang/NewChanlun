# #1534 考古（返修副本）

> 返修副本：基于 `/Volumes/AgentStorage/decisions/1534/archaeology.md`，按独审 R8 与 main `bea0a169f99293e90429ec834b69cb6a34a1958f` 复跑结果更正 Q6/Q14 命中数、Q14 依赖边与 Q5/Q8/Q9/Q10/Q13 行号；更正明细见文末「返修更正」。原件不动。

[grilling] FU-11 逐 bar 事件、身份、生命周期合同（#1323，新增）

归类：A 已裁 1 · B 有先例未正式裁 13 · C 确未设计 0。只有 C 类可列为真正待决；B 类以「默认值，确认或否决」呈现；A 类直接沿用。

## Q1 [B] 逐 bar 事件的必含字段集（cut、身份、epoch、policy/model 版本、前后状态）用什么形状落正本——是否已有统一 schema，还是各域各一套？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 事件 schema | `gh search issues "事件 schema" --repo xy7365527-lang/NewChanlun --limit 200` | 7 |
| tracker | 前后状态 | `gh search issues "前后状态" --repo xy7365527-lang/NewChanlun --limit 200` | 200 |
| repo | before_state\|after_state\|前后状态\|prev_state | `rg -n -i "before_state\|after_state\|前后状态\|prev_state\|pre_state\|state_before" docs/adr/ .chanlun/definitions/ CONTEXT.md` | 4 |
| repo | fact_key / stamp 字段 | `rg -n "fact_key\|commit_seq\|stable_point" rust/src/economic_session/book.rs rust/src/economic_session/tb03c_book.rs` | 444 |

命中：
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:99：#1534（FU-11，事件 schema 须含 epoch、policy/model hash、前后状态以支持恢复）
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:52：4. 订单/成交原件（epoch、身份、前后状态、cause 链）；
- .chanlun/definitions/chong-traceability.md:13：身份、epoch、policy/model 版本、前后状态、撤回和续接的具体事件合同由 #1534 承接。本票不提前发明第二套事件 schema。
- #1340 SPEC C02.1：标准输入包含 source namespace/epoch、instrument、原始事件身份和修订、原始坐标、received_at、逻辑输入顺序、替代关系及来源证据。
- rust/src/economic_session/tb03c_book.rs:585-596：e["stable_point"] = sp["seq"]; e["commit_seq"] = …; e["fact_key"] = …; json!({"kind":"growth_eligible","from":prev_status,"to":status})
- rust/src/economic_session/store.rs:46：CREATE TABLE commits(seq INTEGER PRIMARY KEY,prev_root TEXT NOT NULL,root TEXT NOT NULL UNIQUE,commit_ns TEXT NOT NULL,image TEXT NOT NULL) STRICT;

- 出处：ADR 0038:52/99（正本，点名本票须含 epoch/policy-model hash/前后状态）；#1340 C02.1/C08.2（已批准 SPEC）；chong-traceability.md:13（正本，明令本票承接且不得发明第二套 schema）；代码先例 tb03c_book.rs:585-596、store.rs:46、s_structure_session.rs:219-307。
- 推理链：[新缠论:选择] 事件 schema 无缠论原文可锚，属 graph-engineering.md:23 列明的 T 类「状态机与事件 schema」。推理链：① ADR 0038 已把事件字段要求（epoch、身份、前后状态、cause 链、policy/model 版本）写进六面全状态并交由本票定形；② B/E 域代码已实际持久 commit 根链（prev_root/root/image）、阶段事件 stamp（stable_point、commit_seq、fact_key、from/to）；S 域已持久 raw_events 来源身份与修订、structure_deltas 的 base_cut/next_cut/policy_records；③ 两域字段名各自成形但从未在正本层合成一张合同，缺的是「统一外壳」而非字段本身。
- 默认值：正本层定义一个「域事件外壳」而不改各域内部载荷：外壳字段 = {domain_id, writer_epoch, commit_seq, prev_root, root, structure_cut（S cut 字符串，B/E 事件消费的结构切面）, identity{chong_id=(S,ℓ), voice=(carrier,γ,σ,n)|null, fact_key|command_id}, source{source_namespace, source_epoch, external_fact_id, source_cursor, received_time}, policy_ref{policy_id, policy_revision, content_hash}, model_ref{model_id, revision, content_hash}, transition{field, from, to}, cause[]（前件 fact_key/commit 引用）, kind, payload}。既有 tb03c_book 的 stamp 与 S structure_deltas 直接映射进外壳，不另造第二套。

## Q2 [B] 事件里的「cut」指哪一个：S 的 structure_cut（cut-{gen}）还是本域 commit cut（domain_id, commit_seq, root_hash）？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | cut 身份 | `gh search issues "cut 身份" --repo xy7365527-lang/NewChanlun --limit 200` | 1 |
| tracker | known_frontier_cut | `gh search issues "known_frontier_cut" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| repo | \bcut\b 正本层 | `rg -n "\bcut\b" .chanlun/definitions/ docs/adr/ CONTEXT.md` | 14 |
| repo | base_cut\|next_cut | `rg -c "base_cut\|next_cut" rust/src/` | 129 |

命中：
- .chanlun/definitions/baohan.md:191：一个 cut 可以包含多根 K 线：cut 按代编号（整合分支 `rust/src/bin/s_structure_session.rs:3349` `structure_cut = cut-{gen}`）
- rust/src/economic_session/service.rs:188-190：exact(wanted, &["domain_id", "commit_seq", "root_hash"])?; if wanted["domain_id"] != config.domain { return Err("IdentityConflict：跨域cut".into()); }
- docs/adr/0033-history-simulation-bar-batch-model.md:48：`model_id/revision/content_hash`；`input_domain`；`clock`（bar 索引、`known_frontier_cut`）
- #1340 SPEC C08.4：跨域总览携各权威域前沿、consumed_structure、Pending、缺项及可信旧 cut。
- #1340 SPEC C07.3：共同市场证书带来源 namespace/epoch、事件与修订、集合或规范前缀、替代关系和实际依赖，不能比较不相干整数游标。

- 出处：baohan.md:191（正本，S cut 形态）；ADR 0033:48（known_frontier_cut 进时钟字段）；#1340 C08.4/C07.3；代码 service.rs:188-190（B/E cut 三元组，拒绝跨域 cut）。
- 推理链：[新缠论:选择] 同一个词「cut」在仓内已有两个具体实体：S 侧是按代编号的字符串切面，B/E 侧是 (domain_id, commit_seq, root_hash) 三元组且代码明令跨域 cut 为 IdentityConflict。票面「事件含 cut」若不指明，会把两者混成一个字段；C07.3 又禁止比较不相干整数游标，所以事件必须同时带「自己的 cut」和「消费的结构 cut」两格，不能只留一格。
- 默认值：外壳里分两个字段：`cut` = 本域 (domain_id, commit_seq, root_hash)；`structure_cut` = 该事件判定所依据的 S cut（cut-{gen}，对应 C08.4 的 consumed_structure）。S 域自身事件 structure_cut = next_cut。禁止把 commit_seq 与 S generation 互相比较。
- 冲突：「cut」同名异物：S 正本 baohan.md:191 为 cut-{gen} 字符串；B/E 代码 service.rs:188-190 为 {domain_id, commit_seq, root_hash}。正本层未登记此差别，CONTEXT.md 无「cut」词条。

## Q3 [B] 事件里的「epoch」是哪一个：持久域写者 writer_epoch 还是输入来源 source_epoch？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | writer_epoch | `gh search issues "writer_epoch" --repo xy7365527-lang/NewChanlun --limit 200` | 9 |
| tracker | source_epoch | `gh search issues "source_epoch" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| tracker | epoch | `gh search issues "epoch" --repo xy7365527-lang/NewChanlun --limit 200` | 44 |
| repo | epoch 正本层 | `rg -n -i "epoch" docs/adr/ .chanlun/definitions/ CONTEXT.md` | 19 |
| repo | writer_epoch 代码 | `rg -c "writer_epoch" rust/src/` | 650 |
| repo | source_namespace\|source_epoch 代码 | `rg -c "source_namespace\|source_epoch" rust/src/` | 407 |

命中：
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:170：(a) S/E/B/X 任一持久域的写者身份，即 `writer_epoch` 加 `.writer.lock`；
- docs/adr/0039-production-entry-inventory-and-unique-launcher.md:282：换代：先确认旧 X 和它的发送子进程已退出，再提升 `writer_epoch`、取得排他进程锁（C09.8）。旧 epoch 的写入被 `store.rs:466-467` 拒绝。
- rust/src/economic_session/book.rs:92-99：fact_key … hash(&json!([required(s, "source_namespace")?, required(s, "source_epoch")?, required(s, "external_fact_id")?]))
- rust/src/economic_session/store.rs:654：return Err("StaleWriter：持久epoch不等于获准写者".into());
- #1340 SPEC C07.5：相同 ID 不因换 epoch/计划/证书重建，重复、并发与恢复仅查询旧状态。

- 出处：ADR 0039:170/282（正本，writer_epoch）；#1340 C02.1/C07.5/C09.7；代码 book.rs:92-99（source_epoch 入 fact_key）、store.rs:654（StaleWriter）。
- 推理链：[新缠论:选择] 仓内 epoch 已是两个物：ADR 0039 的 writer_epoch 是「谁有权写这个域」的代数；C02.1 与 book.rs 的 source_epoch 是「输入来源命名空间的纪元」，并参与 fact 身份哈希。ADR 0038:52 要求订单/成交原件带「epoch」时语境是恢复与换代，指 writer_epoch；而 fact 去重依赖的是 source_epoch。两者混名即混掉「身份不随 epoch 重建」（C07.5）这条不变式。
- 默认值：两个字段都进外壳且不共用名字：`writer_epoch`（提交该事件的域写者代数，恢复/换代审计用）与 `source.source_epoch`（来源纪元，入 fact_key）。事件身份与 fact_key 均不含 writer_epoch；换代只改 writer_epoch，不得重铸任何 ID。CONTEXT.md 补「写者纪元 / 来源纪元」两条词。
- 冲突：「epoch」同名异物：writer_epoch（ADR 0039，域写权）vs source_epoch（C02.1 + book.rs fact_key，来源身份）。正本层未分词。

## Q4 [B] 「policy/model hash」取什么形态：单个不透明哈希，还是 (id, revision, content_hash) 引用三元组？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | policy hash | `gh search issues "policy hash" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| tracker | policy_version | `gh search issues "policy_version" --repo xy7365527-lang/NewChanlun --limit 200` | 6 |
| tracker | manifest_hash | `gh search issues "manifest_hash" --repo xy7365527-lang/NewChanlun --limit 200` | 1 |
| repo | policy_version\|profile_hash\|rule_revision\|model_id\|manifest | `rg -n -i "policy_version\|profile_hash\|rule_revision\|model_id\|policy_records\|manifest" docs/adr/ CONTEXT.md .chanlun/definitions/` | 50 |
| repo | policy_records\|profile_hash\|rule_revision 代码 | `rg -c "policy_records\|profile_hash\|rule_revision" rust/src/` | 682 |

命中：
- CONTEXT.md:454：`(fill_allocation_policy_id, manifest_hash, policy_revision)` 三元组随每条 allocation 持久；更正与重放引用原版本的逆规则，换版本只能是显式迁移事件。
- docs/adr/0033-history-simulation-bar-batch-model.md:48：`model_id/revision/content_hash`；`input_domain`；`clock`（bar 索引、`known_frontier_cut`）；`price_rules`（带 ADR 0018/0019 条款 id）
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:47：固定五元组（input, policy_version, model_id/revision, seed, scheduler）下，两次独立运行在每个 checkpoint 的全状态必须逐位等价。
- CONTEXT.md:202：随 `stroke::RULE`（`new-bi-dual-coordinate/2`）进 S 发布的 `policy_version`；改动即开新版本，旧代际 AsKnown 不回填。
- rust/src/bin/s_structure_session.rs:307：policy_records TEXT NOT NULL DEFAULT 'legacy/unknown'

- 出处：CONTEXT.md:454（正本，三元组随每条 allocation 持久）；ADR 0033:48（model_id/revision/content_hash）；ADR 0038:47（policy_version + model_id/revision）；CONTEXT.md:202（policy_version 换版不回填）；代码 structure_deltas.policy_records、s_session profile_hash。
- 推理链：[新缠论:选择] 正本已有三处同形先例：外效分配政策用 (id, manifest_hash, revision) 三元组；历史模拟模型用 model_id/revision/content_hash；S 发布用 policy_version/rule_revision。三处都不是「单个 hash」，而是「可读 id + 版本 + 绑定内容哈希」。票面写「hash」是简称；按先例应落三元组，hash 负责绑定、id/revision 负责可读与迁移。
- 默认值：外壳字段 `policy_ref = {policy_id, policy_revision, content_hash}`、`model_ref = {model_id, revision, content_hash}`，直接沿用 CONTEXT.md:454 形状；content_hash 为 JCS 规范化字节的 SHA-256（ADR 0025 先例）。版本变化只能是显式迁移事件，不在同一 writer_epoch 内原地切换；历史事件 latch 与当时 policy_ref 绑定（ADR 0038 三.4）。

## Q5 [B] 事件的「身份」由哪几层键构成（重、Voice、事实、命令、事件自身）？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | fact_key | `gh search issues "fact_key" --repo xy7365527-lang/NewChanlun --limit 200` | 3 |
| tracker | fact_key in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "fact_key in:comments"` | 1 |
| tracker | 身份 Voice carrier | `gh search issues "身份 Voice carrier" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| repo | event_id\|事件身份 | `rg -n -i "event_id\|事件 id\|事件身份\|事件 ID" docs/adr/ .chanlun/definitions/ CONTEXT.md` | 1 |
| repo | fact_key 代码 | `rg -c "fact_key" rust/src/` | 444 |

命中：
- CONTEXT.md:88-89（声部词条）：一次建仓的完整生命周期实体，v = (carrier, γ, σ, n)。一经建立不可变，出场即终结。
- .chanlun/definitions/chong-traceability.md:33：追溯必须区分重身份与 Voice 生命周期，不能拿 Voice 的退出清空重账。
- rust/src/economic_session/book.rs:85-99：target(event) → book_identity.chong_id；fact_key(event) = hash([source_namespace, source_epoch, external_fact_id])
- #1340 SPEC C09.10：不可复活命令身份、内容摘要、判定和一次调用占有记录为长期审计事实……回收普通流游标不回收命令去重依据。
- #1516（closed）标题：消除 make_token() 时间戳/PID 依赖（回放 BLOCKER）

- 出处：ADR 0010（重 = ⟨S, ℓ, Q⟩）与 CONTEXT.md:88（Voice 四元组）为正本；chong-traceability.md:33；fact 身份为代码先例 book.rs:92-99；命令身份 #1340 C07.5/C09.10；#1516 已关（身份不得含时间戳/PID）。
- 推理链：[新缠论:推论] 重与 Voice 的身份已裁（ADR 0010、CONTEXT.md:88）；事实身份 fact_key 与命令身份在代码与 SPEC 中已定但未入正本。推理链：事件要可追溯到「哪一重、哪一个 Voice、哪条外部事实、哪次提交」，四层键各自独立且都已有实体；事件自身 ID 应取 (domain_id, commit_seq) 而非内容哈希或时间戳，否则重放（#1516）与去重（C06.7）都破。
- 默认值：身份四层：`identity.chong_id=(S,ℓ)`；`identity.voice=(carrier,γ,σ,n)|null`（零筹码期为 null，重账仍在）；`identity.fact_key=hash(source_namespace, source_epoch, external_fact_id)` 或 `command_id`；事件自身 `event_id=(domain_id, commit_seq)`。任何一层都不含 wall-clock/PID；Voice 退出不改 chong_id。

## Q6 [B] 「前后状态」怎么表达：事件内联完整 before/after 图像，还是 from/to 字段 + 提交根链？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | prev_root | `gh search issues "prev_root" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| tracker | 前后状态 | `gh search issues "前后状态" --repo xy7365527-lang/NewChanlun --limit 200` | 200 |
| repo | prev_root 代码 | `rg -c "prev_root" rust/src/` | 12 |
| repo | "from":prev_status | `rg -c '"from":prev_status' rust/src/` | 1 |
| repo | 快照仅派生缓存 | `rg -n "快照仅派生缓存\|日志折叠\|唯一真相" docs/adr/` | 3 |

命中：
- docs/adr/0008-first-retrace-ledger-semantics.md:12：append-only 修订日志 = 唯一真相（CompletedFreezeReducer 先例：幂等重放、冲突拒绝）；状态 = 日志折叠（active/consumed/restart_allowed 是投影不单独持久化）；快照仅派生缓存（带溯源、可校验、非真相）
- rust/src/economic_session/store.rs:46：commits(seq INTEGER PRIMARY KEY,prev_root TEXT NOT NULL,root TEXT NOT NULL UNIQUE,commit_ns TEXT NOT NULL,image TEXT NOT NULL)
- rust/src/economic_session/store.rs:1095-1097：if image != super::tb03c_e::replay(c, config, &previous_image, event, seq, &ns, budget)? … return Err("StorageUnavailable：Resolve 重放不等于完整提交图像".into());
- rust/src/economic_session/tb03c_book.rs:595：json!({"kind":"growth_eligible","from":prev_status,"to":status})
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:52：4. 订单/成交原件（epoch、身份、前后状态、cause 链）；

- 出处：ADR 0008（first-retrace）:12（正本，日志真相/快照派生）；ADR 0038:52；代码 store.rs:46（prev_root/root/image 根链）、store.rs:1095-1164（每步重放等于提交图像）、tb03c_book.rs:595（from/to）。
- 推理链：[新缠论:选择] 两条先例互补：ADR 0008 说真相是日志、快照是派生缓存；B 域代码每次 commit 既存 image 又在审计时用 replay 核等。因此「前后状态」不需要在每条事件内联完整图像——prev_root/root 已把前后全状态钉死且可校验；对状态机字段（latch、growth_eligible、Voice 状态）再加 from/to 使单条事件可读。两者都存在于代码，只缺正本定形。
- 默认值：事件外壳 `prev_root`/`root`（域图像哈希链）+ `transition{field, from, to}`（仅对状态机字段）+ `cause[]`。不内联完整 before/after 图像；完整图像由 commits.image 提供且必须等于重放结果。

## Q7 [B] 重复与乱序投递如何处理：按身份去重还是按内容去重？乱序事实的接纳与应用顺序？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 乱序 | `gh search issues "乱序" --repo xy7365527-lang/NewChanlun --limit 200` | 30 |
| tracker | 重复投递 | `gh search issues "重复投递" --repo xy7365527-lang/NewChanlun --limit 200` | 70 |
| repo | 乱序 正本层 | `rg -n "乱序" docs/adr/ .chanlun/definitions/ CONTEXT.md` | 0 |
| repo | 乱序 全仓 | `rg -c -i "乱序" rust/src docs analysis .chanlun` | 247 |
| repo | applications fact_key UNIQUE | `rg -n "fact_key TEXT NOT NULL UNIQUE\|seq INTEGER NOT NULL UNIQUE\|supersedes_revision" rust/src/economic_session/store.rs rust/src/bin/s_structure_session.rs` | 3 |

命中：
- #1340 SPEC C06.7：同事实/分配 ID 与重 ID 同内容只应用一次，异内容冲突；累计与逐笔报使用同一规范化合同。缺来源身份或无法区别同形事实时保留冲突，不能按内容去重误吞真实成交。更正按前件因果应用，账实快照不能直接覆写私账。
- #1340 SPEC AT-10：100/40/20 改价、未知发送、cancel/ack/fill 乱序、重复、fee/bust 和后续更正维持全部累计量与潜量
- rust/src/economic_session/store.rs:49：CREATE TABLE applications(k TEXT PRIMARY KEY,fact_key TEXT NOT NULL UNIQUE,json TEXT NOT NULL) STRICT;
- rust/src/bin/s_structure_session.rs:219-238：raw_events(identity_key, revision, … seq INTEGER NOT NULL UNIQUE, source_namespace, source_epoch, … event_id, received_at, … supersedes_revision INTEGER, PRIMARY KEY (identity_key, revision))
- docs/adr/0008-first-retrace-ledger-semantics.md:11：门卫钟（last_as_of）倒退拒收 + 警报；每条修订带知情时（as_of）
- docs/adr/0033-history-simulation-bar-batch-model.md:12：批内各重订单各自独立对 OHLC 判定……互不排序、互不占用对方资金，`intra_batch_order: Unknown`

- 出处：#1340 C06.7/C06.4/AT-10（已批准 SPEC）；ADR 0008（first-retrace）:11（门卫钟倒退拒收）；ADR 0033:12（批内顺序 Unknown）；代码 store.rs:49（fact_key UNIQUE）、s_structure_session.rs:219-238（identity_key+revision+supersedes_revision）。
- 推理链：[新缠论:选择] SPEC 已定「同 ID 同内容只应用一次、异内容冲突、不按内容去重」，代码以 fact_key UNIQUE 与 (identity_key, revision) 主键实装；乱序则由 C06.4「先 Recorded/Pending 再解释」+ 更正按前件因果（C06.7）+ 批内顺序 Unknown（ADR 0033）共同覆盖。正本层缺一句把这些合成「事件合同」的话。
- 默认值：去重键 = 身份（fact_key / (identity_key, revision)），不按内容；同 ID 同内容重投 → no-op 并返回原 application；同 ID 异内容 → 持久 IdentityConflict，不吞不覆盖；乱序 → 到达即 Recorded/Pending（按 received_time + source_cursor 记原序），应用按 cause[] 前件因果，缺前件则 Pending 不丢；跨重/批内顺序声明 Unknown，不作比较面。
- 冲突：范围边界：订单/fill 层的重复与乱序（cancel/ack/fill）归 #1330（open，C4 custody）与 #1539（FU-17）；本票只裁 S/B 逐 bar 事件外壳的去重与因果规则，不裁 X 侧调用占有。

## Q8 [B] 撤回与恢复的幂等机制：撤回是删除还是追加事件？恢复重放已应用事实如何保证 no-op？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 幂等 | `gh search issues "幂等" --repo xy7365527-lang/NewChanlun --limit 200` | 55 |
| tracker | 撤回 替代 | `gh search issues "撤回 替代" --repo xy7365527-lang/NewChanlun --limit 200` | 9 |
| tracker | writer_epoch in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "writer_epoch in:comments"` | 4 |
| repo | 幂等 正本层 | `rg -n "幂等" docs/adr/ .chanlun/definitions/` | 8 |
| repo | withdrawn_generation\|superseded_by | `rg -c "withdrawn_generation\|superseded_by" rust/src/` | 287 |

命中：
- docs/adr/0008-first-retrace-ledger-semantics.md:12：append-only 修订日志 = 唯一真相（CompletedFreezeReducer 先例：幂等重放、冲突拒绝）
- rust/src/bin/s_structure_session.rs:258-262：first_known_generation, first_known_cut, published_generation, withdrawn_generation INTEGER, withdrawal_reason TEXT, superseded_by TEXT
- #1371（closed）resolution：实际更正/撤回/替代、原始获知史、同 cut 原子变化与真进程恢复的证据见 …ROOT-ACCEPTANCE.md
- #1340 SPEC C07.4：已有 ExecuteCommitted 只返回原收据，不铸第二份资格。
- rust/src/economic_session/finance_tests.rs:754：// 原 Committed 幂等重送仍返回原永久收据，不被后来 Pending 倒写。
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:60：恢复：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar；杀停等同未完成 bar 的全部外效未发生

- 出处：ADR 0008（first-retrace）:12（正本，幂等重放/冲突拒绝）；ADR 0038 裁定二（恢复语义，正本）；#1371 已关（S 侧原子撤回已交付）；#1340 C07.4/C09.9；代码 s_structure_session.rs:258-262（withdrawn_generation/superseded_by）、finance_tests.rs:754。
- 推理链：[新缠论:推论] 由 ADR 0008「日志唯一真相」推出：撤回只能是追加一条带 withdrawn_generation/withdrawal_reason/superseded_by 的事件，旧对象与旧 AsKnown 不动（baohan.md:172/186 已明文）；恢复 = 从 checkpoint 重放日志（ADR 0038 裁定二），重放到已应用 fact_key 时由 UNIQUE 约束与 replay 等值检查（store.rs:1095-1164）保证 no-op。S 侧已由 #1371 实证，B/E 侧有代码先例但正本未写。
- 默认值：撤回 = append-only 事件 {kind: withdrawn, withdrawn_generation, withdrawal_reason, superseded_by}，不删行、不回填旧 cut/AsKnown；恢复幂等的判据：重放同一 fact_key/command_id 返回原 application/收据，图像根与原 commit 的 root 逐位相等，否则 StorageUnavailable 拒绝继续写；幂等键永不含时间戳/PID（#1516）。

## Q9 [B] snapshot/delta 续接协议：快照钉住哪些字段，delta 带什么，续接等价判据与 Gap 处理是否对 S/B/E 三域统一？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | snapshot delta | `gh search issues "snapshot delta" --repo xy7365527-lang/NewChanlun --limit 200` | 5 |
| tracker | 快照 续接 | `gh search issues "快照 续接" --repo xy7365527-lang/NewChanlun --limit 200` | 1 |
| tracker | checkpoint | `gh search issues "checkpoint" --repo xy7365527-lang/NewChanlun --limit 200` | 39 |
| repo | 续接\|resume\|checkpoint 正本层 | `rg -n -i "续接\|resume\|恢复点\|checkpoint" docs/adr/ .chanlun/definitions/` | 25 |
| repo | Gap S 域 | `rg -c "Gap" rust/src/bin/s_session_v3/` | 41 |

命中：
- #1340 SPEC C08.2：正式结构快照固定 session/generation、structure_cut、catalog_revision、scope 和 index_frontier。分页不跨切面，结构 delta 带 base/next、持久序号和原子批次；恢复续接与同 cut 快照逐字段相等，缺口显式 Gap，旧代际明确拒收。
- #1372（closed）resolution：160输入/500ms、固定cut分页与持续推进、有效95→99含撤回替代正控、双S95/Q109故障独立轨迹、495完整核心记录、AsKnown/修订与过滤恢复均按原报告的声明域
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:60-69：seek(t) 语义：1. 取 S/B/E 最后不晚于 t 的快照；2. 从该快照向前重放至 t 的收盘；3. `as_of=t` 输出只含索引 ≤t 的事实
- rust/src/bin/s_structure_session.rs:295-307：structure_deltas(generation INTEGER PRIMARY KEY, session_id, catalog_revision, base_cut, next_cut, seq_range_json, index_frontier, input_frontier, … policy_records)
- rust/src/economic_session/service.rs:186：return Err("InvalidDomain：cut与AsKnown不能同时指定".into());

- 出处：ADR 0038 裁定二（正本，seek/checkpoint）；#1340 C08.2/C09.4/AT-04；#1372 已关（S 侧固定 cut 分页与断线续接已交付）；代码 structure_deltas（S）、commits + cut/as_known_ns 读口（B/E）。
- 推理链：[新缠论:选择] S 侧协议已由 C08.2 定形并经 #1372 实证；B/E 侧只有 commits 根链与 cut/AsKnown 互斥读口，没有 delta base/next 与 Gap 语义。ADR 0038 把三域快照放进同一恢复协议，所以 B/E 续接应复用 C08.2 形状，而非另立。
- 默认值：三域统一：snapshot 钉 (domain_id, generation|commit_seq, cut, rule/catalog_revision, scope, frontier)；delta 带 base_cut、next_cut、seq_range、原子批次 id；续接判据 = 续接结果与同 cut 快照逐字段相等；落后消费者收 Gap 并从指定 cut 重建；旧代际/旧 writer_epoch 拒收；cut 与 AsKnown 互斥查询。S 侧沿用 #1372 实装不改。

## Q10 [B] 持久边界：哪些是真相日志、哪些是可重算投影、哪些投影因 AsKnown 必须持久？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | RecomputedWithRevision | `gh search issues "RecomputedWithRevision" --repo xy7365527-lang/NewChanlun --limit 200` | 7 |
| tracker | as_known_ns | `gh search issues "as_known_ns" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| repo | AsKnown 正本层 | `rg -n "AsKnown" docs/adr/ .chanlun/definitions/ CONTEXT.md` | 38 |
| repo | revision\|RecomputedWithRevision 正本层 | `rg -n -i "revision\|RecomputedWithRevision" docs/adr/ .chanlun/definitions/` | 33 |

命中：
- docs/adr/0008-first-retrace-ledger-semantics.md:12：状态 = 日志折叠（active/consumed/restart_allowed 是投影不单独持久化）；快照仅派生缓存（带溯源、可校验、非真相）；拒收/警报另记 audit 流不进恢复路径
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:73：latch 存储「首次达标 bar 索引 + 证据指纹（成本/频率/速率下界的原始数值）」，后续 bar 不回填，恢复/seek 时按原 campaign 还原该值
- CONTEXT.md:445：`TStage` latch 只投影 campaign 内达标事件史（AsKnown 不回填）；扩量资格由唯一谓词按当前真实成本 K 与已退本金计算，latch 不得作其输入。
- #1340 SPEC C08.6：AsKnown 使用实际 S/E/B/X 记录和依赖向量；恢复的晚到应用不能回填为故障当时已处理。RecomputedWithRevision 独立命名并绑定输入/规则，不冒充当时可知。
- docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md:22：`phase_events` append-only，AsKnown 不回填；更正类按 C08.6 以 RecomputedWithRevision 另列。

- 出处：ADR 0008（first-retrace）:12、ADR 0031:22、ADR 0038 三.1、CONTEXT.md:445（均正本）；#1340 C08.6；代码 store.rs:46-51（commits/facts/applications/book 分表）、tb03c_book.rs:632（latch = max(prev_rank, rank)）。
- 推理链：[新缠论:推论] 由「日志唯一真相」与「AsKnown 不回填」两条正本推出：纯函数投影（growth_eligible、毛账合计、book image）可重算且须等于重放；而 latch/phase_events 是「当时首次达标」这一历史事实，重算会冒充当时可知，故必须作为事件持久而非投影。代码已按此分表（facts/applications 为真相，book 为物化并 replay 核等，phase events 入 book 随 commit 持久）。
- 默认值：持久边界三类：① 真相 = facts/applications/commits 根链与 S raw_events/structure_deltas（append-only）；② 可重算投影 = book image、growth_eligible、毛账合计，允许物化但必须等于重放；③ AsKnown 事实 = phase_events/latch（首次达标 bar + 证据指纹）、first_known_cut、withdrawn_generation，作为事件持久，重算结果只能以 RecomputedWithRevision 另列、绑定输入与 policy_ref。

## Q11 [B] 词条冲突：FU-11 的「事件」与 CONTEXT.md 已有「背驰段候选事件」、S 的「generation」与 CONTEXT.md「代际（Generation, n）」同名异物，正本怎么分词？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | generation 代际 | `gh search issues "generation 代际" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| tracker | 事件 身份 cut | `gh search issues "事件 身份 cut" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| repo | 代际 CONTEXT.md | `rg -n "代际" CONTEXT.md` | 8 |
| repo | 候选事件\|塔内原生一等事件对象 | `rg -c -i "塔内原生一等事件对象\|候选事件" rust/src docs analysis .chanlun CONTEXT.md` | 130 |

命中：
- CONTEXT.md:150-151：**代际（Generation, n）**: 声部在赋格树中的嵌套代数：根声部为 0，子声部 = 父代际 + 1。
- CONTEXT.md:166：S 快照里跨代累计的六族数组按记录进入该族的代际升序、同代内按原键排列
- .chanlun/definitions/baohan.md:191：cut 按代编号（…`structure_cut = cut-{gen}`）
- CONTEXT.md:292：塔内原生一等事件对象（2026-07-28 #540 裁定，ADR-0007）：承载 E2E-D1-D3 结构载荷 + 背驰段区间
- docs/adr/0012-inference-layer-document-naming.md 裁定五：拿词的外形（「声部」两个字）当同一性判据

- 出处：CONTEXT.md:150（代际 = 声部嵌套代数，正本）vs CONTEXT.md:166 / baohan.md:191（代际/generation = S cut 代，正本）；CONTEXT.md:292（事件 = ADR 0007 塔内候选事件，正本）；ADR 0012 裁定五（同名异物先例与处置方式：译注进 CONTEXT.md）。
- 推理链：[新缠论:选择] domain-modeling「Challenge against the glossary」当场命中：Voice 四元组的 n 是「代际」，S 发布的 generation 也译作「代际」，两者在同一条事件里会同时出现；「事件」已被 ADR 0007 占用为结构层一等对象。ADR 0012 裁定五已示范处置：不改原词，在 CONTEXT.md 加译注分词。
- 默认值：事件外壳字段名分开：`voice.n`（声部代际）与 `structure_generation`（S 发布代）；FU-11 的事件统称「域事件（domain event / 账事件）」，CONTEXT.md 新增词条并在「代际」「背驰段候选事件」词条下各加一句 Avoid 译注；不改 ADR 0007 与 S 既有字段名。
- 冲突：「代际/generation」：CONTEXT.md:150（Voice n）vs CONTEXT.md:166、baohan.md:191（S cut 代）；「事件」：CONTEXT.md:292（ADR 0007 结构候选事件）vs 本票逐 bar 域事件。正本层两处均未分词。

## Q12 [B] 本票正本落点：docs/adr/ 还是 .chanlun/definitions/？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | FU-11 | `gh search issues "FU-11" --repo xy7365527-lang/NewChanlun --limit 200` | 79 |
| tracker | seek 不偷未来 | `gh search issues "seek 不偷未来" --repo xy7365527-lang/NewChanlun --limit 200` | 2 |
| repo | 正本：本文件是 教义正本 | `rg -n "^> 正本：本文件是" docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md .chanlun/definitions/chong-traceability.md` | 2 |

命中：
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:1：> 正本：本文件是双跑确定性、恢复语义、seek 不偷未来、AsKnown 分证协议的教义正本。
- .chanlun/definitions/chong-traceability.md:1：> 正本：本文件是 Chong 逐 bar 追溯的教义正本。
- ADR 0012 裁定二：正本层管「裁完的结果是什么」—— `.chanlun/definitions/`（缠论教义）／ `docs/adr/`（架构决策）／ `CONTEXT.md`（词）
- .sandcastle/docs/graph-engineering.md:23：T 类（技术取舍）：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决。

- 出处：ADR 0012 裁定二（definitions = 缠论教义，adr = 架构决策）；同图先例 ADR 0038（FU-15 恢复语义落 docs/adr）；chong-traceability.md（FU-10 落 definitions）。
- 推理链：[新缠论:选择] 事件 schema/epoch/幂等/续接全是架构合同，无缠论原文锚；ADR 0012 把架构决策归 docs/adr；最近同图先例 FU-15 同类内容落 ADR 0038。FU-10 落 definitions 是因其主体是 Classification→重的教义追溯链，本票不是。
- 默认值：落 `docs/adr/0050-per-bar-domain-event-identity-lifecycle-contract.md`（三行头 + 引用自证表，沿 ADR 0038 版式）；CONTEXT.md 只加 Q11 的分词词条；chong-traceability.md 不改。

## Q13 [A] 评论承接项：空头事件不触发第三相、换向保留同一重账目和事件史——事件合同是否需另裁？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 换向 阶段史 | `gh search issues "换向 阶段史" --repo xy7365527-lang/NewChanlun --limit 200` | 3 |
| repo | EarningShares 空头 换向 | `rg -n "空头事件不把 latch 推进到 EarningShares\|换向属于同一重的 Voice 换代" .chanlun/definitions/chong-traceability.md docs/adr/0043-chong-reversal-account-continuity.md` | 2 |

命中：
- .chanlun/definitions/chong-traceability.md:27：多头事件可推进到 EarningShares。空头只有降成本与退本金两相，空头事件不把 latch 推进到 EarningShares。……以上直接沿用 [ADR 0043 裁定三]，不新设阶段。
- .chanlun/definitions/chong-traceability.md:29：换向属于同一重的 Voice 换代。P、K、R、费用、phase_events、G 与 withdrawn 连续，不重置、不按方向拆账，也不因换向新增本金批次。
- rust/src/economic_session/tb03c_book.rs:10-11：多头资格为真 ⇒ latch = EarningShares（根裁定 OQ-2）；空头无赚股第三相（SPEC C05.5），latch 止于 CapitalRecovered。

- 出处：ADR 0043 裁定三（正本）；chong-traceability.md:27-29（正本，#1533 已关 resolution）；代码 tb03c_book.rs:10-11、:632 latch = max(prev_rank, rank) 与之一致。

## Q14 [B] 范围边界：本票事件合同是否覆盖 X 侧命令/fill/journal 生命周期，还是止于 S/B/E 逐 bar 事件外壳？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | journal 幂等 乱序（#1330/#1539） | `gh search issues "乱序" --repo xy7365527-lang/NewChanlun --limit 200 --json number,title,state` | 30 |
| tracker | event schema | `gh search issues "event schema" --repo xy7365527-lang/NewChanlun --limit 200` | 0 |
| repo | OwnAttempt\|MayHaveCalled 正本层 | `rg -n "OwnAttempt\|MayHaveCalled" docs/adr/ .chanlun/definitions/` | 0 |
| tracker | OwnAttempt\|MayHaveCalled（SPEC 正文） | `gh issue view 1340 --repo xy7365527-lang/NewChanlun --json body --jq .body \| grep -n -E "OwnAttempt\|MayHaveCalled"` | 6 |
| tracker | #1539 依赖声明与边 | `gh issue view 1539 --json body`；`gh api repos/xy7365527-lang/NewChanlun/issues/1539/dependencies/blocked_by` | 1（票面列 FU-11（#1534）；blocked_by 仅 #1538） |

命中：
- #1330（open，grilling）票面：order lifecycle、partial/full fill、cancel、kill、幂等与错误恢复；…append-only journal、startup replay/reconcile 与 fail-closed
- #1539（open）标题：[grilling] FU-17 成交/持仓唯一真账与 crosswalk（#1323，新增）
- #1340 SPEC C07.5：X 持久 OwnAttempt 以命令唯一身份和内容绑定，从 Ready 原子变为 MayHaveCalled；只有本次首次成功者可尝试一次场所调用。
- /Volumes/AgentStorage/decision-map-1323-倒推-20261018.md:51：FU-11…阻塞 TB-02/03/05/07；依赖 FU-06/10。
- /Volumes/AgentStorage/decision-map-1323-倒推-20261018.md:69：FU-17…阻塞 TB-04/07/08/09；依赖 FU-03/04/11/16。
- #1539 票面「依赖的 FU」：FU-03、FU-04、FU-11（#1534）、FU-16（#1538）。
- docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:52：4. 订单/成交原件（epoch、身份、前后状态、cause 链）；

- 出处：决策地图 FU-11/FU-17 分工（仓外原件）；#1330 与 #1539 均 open 且题面覆盖 X/fill 层；ADR 0038:52 把订单/成交原件列为全状态第四面但其身份合同归 C07。
- 推理链：[新缠论:选择] 三张票若都裁「幂等/乱序」会重裁同题。FU-11 阻塞的是 TB-02/03/05/07（结构、重账、故障隔离、双跑），不含 TB-04/08（执行）；FU-17 与 #1330 阻塞 TB-04/08。按 DAG，本票定「外壳」且让订单/成交原件作为一种 kind 套同一外壳，具体 X 侧占有与 fill 归属留给 #1330/#1539。
- 默认值：本票裁 S/B/E 逐 bar 域事件外壳、身份四层、epoch 分词、去重/因果、撤回/恢复幂等、snapshot/delta 续接；X 侧 OwnAttempt/MayHaveCalled 与 fill→Voice 归属只约定「必须套同一外壳并带 command_id」，实体规则归 #1330 与 #1539。#1539 票面已声明依赖 FU-11，补接 `#1534 blocks #1539`；#1330 无此声明，不接边。
- 冲突：#1330（open）与 #1539（open）题面与本票在「幂等/乱序/journal」上重叠，正本层无分工记录（`OwnAttempt|MayHaveCalled` 正本层 0 条）；仅决策地图（仓外）有 FU-11/FU-17 分工。#1539 票面声明的 FU-11 依赖在 tracker 中未接边。

## 机械校验

通过。

## 返修更正

冻结基线：main `bea0a169f99293e90429ec834b69cb6a34a1958f`。原查询结果存于 `/Volumes/AgentStorage/issue1534-draft-repair-20261027/evidence/`。

| 点 | 原记 | 更正 | 依据 |
|---|---|---|---|
| Q14 repo `OwnAttempt\|MayHaveCalled` 正本层 | 6 | 0（加 `CONTEXT.md` 仍 0）；另立 tracker 行：#1340 正文 6 行 | `evidence/01-q14-ownattempt.txt` |
| Q14 默认值 | 「blocking 边不变」 | 补接 `#1534 blocks #1539`；#1330 不接边 | `evidence/03-issue1539-body.txt`、`04-dependencies.txt`、`05-issue1330-no-dependency.txt`、`06-decision-map.txt` |
| Q6 repo `快照仅派生缓存\|日志折叠\|唯一真相` | 1 | 3（ADR 0008 :12/:13/:24） | `evidence/02-q6-snapshot.txt` |
| Q5 chong-traceability | :37 | :33（:37 为节标题） | `evidence/07-line-corrections.txt` |
| Q8 ADR 0038 恢复 | :66 | :60 | 同上 |
| Q9 ADR 0038 seek | :68-73 | :60-69 | 同上 |
| Q10 ADR 0038 latch | :78 | :73 | 同上 |
| Q13 chong-traceability | :25/:27 | :27/:29 | 同上 |

归类（A 1 · B 13 · C 0）与各点结论不变。
