# #1419×#1393 CF+W 整合：W 移植非作者独审（/root/review_w_port，2026-10-19）

审查对象：分支 `codex/1419-1393-cfw-integrate-20261019`，提交 `f2763c5e`（CF 合入 + W 语义移植）。
以 `50cf66a27e` 为准；复核时分支已到 `0defdcc22b`，`rust/src/theta_v0/parser`、`tb02a_facts.rs`、`tb02b.rs`、`content_producer.rs`、`s_session/` 在 `f2763c5e..0defdcc22b` 间无改动，结论同样适用于 `0defdcc22b`。
W 原语义依据：`de73545cbf` 的 `inclusion.rs`/`tb02a_facts.rs`/`s_tb02_contract.py`，Lean `formal/Origin/RawInclusionInitialization.lean`。

## 结论

- **inclusion.rs W 状态机移植：PASS。**
- **W 字段逐字（v2 写者 tb02a_facts 全量/增量、tb02b `W_CONTRACT`、Q3 py/js typed/consumer）：PASS（静态；s_session 未过编译器）。**
- **W 移植整体：BLOCK。** 阻断项在 S3 路径与未编译面（B1–B4），不在 parser 核。

## 1. 状态机逐分支（对 W 源与 Lean）

`inclusion.rs` 第 1–500 行（`contains`/`strict_dir`/`merge`/`first_raw_direction`/`process_inclusion`/`IncrInclusion`/`fold_step`/`fold_all`）与 `de73545cbf` **逐字节相同**。
`IncrFactsState` 重写部分逐分支对照：

| 分支 | W 源 | 移植 | 判定 |
|---|---|---|---|
| 等待期 | 只判新增末对；未建立推 `pending_raw`，waiting 步来源 = {incoming} | 同；另记 `unmapped`、共享 StepRecord | 一致 |
| 建立 | 首个 raw 相邻非包含严格对；证据 = (raw[n-2], raw[n-1], 方向, 全部前缀坐标)；清步骤，seed raw0，raw[1..] 走同一折叠 | `establish` 同；`established_at=n`；共享节点 coords = `raw_prefix` | 一致 |
| 已建折叠 | 来源 = 末组成员 ∪ 当前证据来源 ∪ {incoming}；同价根按到达序 | `fold` 同；共享表示取 `raw_prefix`（非降级域下与上式集合相等，已用闭式 oracle 验证） | 一致 |
| 发证 | 重折这一代 0，否则 merged-1 | 同（另显式 unsettled→0） | 一致 |
| 旧「建立前遇包含即永久未定」 | 删除 | 已删，`inclusion.rs` 内无残留 | 一致 |
| 回退早于建立点 | — | 按原件重放 raw[..rank]，结果必为等待期 | 正确 |
| 回退恰在建立点 | — | 逐步弹出，`replayed_from_zero = (established_at == len)`，与新鲜重放一致 | 正确 |

边界：建立前全包含（CE4，永不建立，零正式组）、建立对恰在首两根（CE3）、回退跨建立点（随机 1500 组 × 每个 rank）均已执行验证。

## 2. 批量重折与逐根增量的共享证据一致性（反例尝试）

新增独立 oracle：`inclusion_wreview_tests.rs`（本目录；Lean `firstPair/foldFrom` 逐字转写，不复用 `first_raw_direction/fold_step`），并按 W 闭式核来源集合（步 k 来源 = raw[..=max(k,e)]，e = 建立对右端）。手构反例：

- CE1 U04（相邻包含非链式）后接严格对，从 raw0 UP 重折；
- CE2 U10 变体，中段 DOWN 建立成单组，其后同向/反向；
- CE3 建立对在首两根，其后全包含再反向；
- CE4 等宽互含全包含，永不建立；
- CE5 建立前包含 + 同价极值根集合跨重折。

另随机 1500 序列（小价域高包含率）：逐前缀核 oracle、共享=展开、变化见证（重折时 step/group/merged 从 0 脏，其余前缀不变）、发证规则；对每个 rank 回退后接异样后缀，对新鲜重放逐位相同；800 序列核「已发证前缀此后不被改写」。

结果：`nc-remote review/w-port-oracle-20261019 rust cargo test --lib theta_v0::parser::inclusion::wreview_tests` → **3 passed / 0 failed**（`remote-wreview-oracle.log`）。**未能打破。**

## 3. W 字段逐字

`raw-initialization-w/1`、`CC-004.initialization`（槽 `W`）、`CC-004.undetermined_raw`、`first_raw_pair`、`direction_state`（`Undetermined`/`Established`）、`pending_since_cut`、`initialization_policy_revision`、组 `initial_direction_evidence`、CC-054 `domain = raw_adjacent_W_initialization`、请求 `["CC-005.inclusion_group","pending"]`：v2 写者 `add_initialization`（全量/增量共用）、`tb02b::W_CONTRACT`、`s_tb02_contract.py`、`q3-typed.js`、`q3-consumer.js`、`tb01c-client.js` 与 W 源逐字一致。组/步请求原因去掉 `equal_extreme_identity` 属 #1405 主线语义，非 W 偏离。

增量缓存：初始化指纹（首对坐标+方向）变化即整体失效；步 dep_max 取来源最大坐标（覆盖重折步依赖整建立前缀）；组 dep_max 并入初始化证据来源。静态复核无漏失效路径。

## 4. #799 宽严两档

`inclusion.rs` 无第二判据：全量入口与热路径同一 `IncrFactsState`；降级域（坐标非递增）回落 `process_inclusion_with_facts` 仍是同一状态机，且显式 `SharedEvidenceUnavailable`。**旧判据只残留在 S 测试期望里（见 B3），不在生产判定路径。**

## 5. 复跑

`~/.local/bin/nc-remote codex/1419-1393-cfw-integrate-20261019 rust cargo test --lib theta_v0::parser`（sha `0defdcc22b`，parser 与 `50cf66a27e` 相同）→ **179 passed / 0 failed / 9 ignored**（ignored 为既有 profile/diag/tmp_export），含 W tb02u 6 项（`remote-parser-baseline.log`）。注：`nc-remote f2763c5e` 直接给短 sha 会报 empty bundle，需传 ref。

## 阻断项

- **B1（S3 知识域自相矛盾）**：`content_producer.rs:1416` S3 的 CC-054 `domain` 写 `tb02b::CURRENT_CONTRACT.knowledge_domain`（`established_direction_with_extreme_root_sets`），而 S3 写者只接受 W 后继规则串（`s_session_v3/mod.rs:126`），`source_audit.rs:1820` 按 `contract_for_rule(W)` 要求 `raw_adjacent_W_initialization` ⟹ S3 生成物会被自己的来源审计拒绝。应改用 `W_CONTRACT`（或按 rule 取 contract）。
- **B2（S3 未发布 W 对象 / 发布 waiting 步）**：S3 构造按步一对象（`content_producer.rs` 测试 `objects.len() == facts.steps.len()`），等待期会把 waiting 步发布成 `CC-004.inclusion_step`，且无 `CC-004.initialization`/`undetermined_raw`、CC-054 无 `initialization_policy_revision`/`direction_state`；与 v2 W 写者语义不同（同 owner 待裁定 3）。
- **B3（旧判据残留于测试期望）**：`content_producer.rs:3439` `CASE_WAITING=[(0,10,5),(1,9,6),(2,12,7)]` 注释「初始方向永久未定：首步 seed 之后全部 waiting」，`:4092` 期望无方向证据，`:6074–6083` 断言 unsettled、`steps[0]=="seed"`、waiting 来源 `[0,1]`/`[0,1,2]`。W 下 (1)→(2) 为严格向上非包含对，第三根即建立并从 raw0 重折 ⟹ 这些断言编码的是已删判据，编译后必败；须按 W 改写（另补真正永不建立的样例）。
- **B4（未编译面）**：`--features s_session` 不可编译（CF 自身 24 错），tb02a_facts 增量缓存、tb02b、v2/v3 W 改动、S 侧 W 测试全部未执行；字段与缓存结论只是静态。

## 非阻断建议

- N1：现有 parser delta/rewind 测试的「全量旧核」`process_inclusion_with_facts` 已是同一状态机，独立性弱；建议并入本 oracle 测试（`inclusion_wreview_tests.rs`）。
- N2：`q3-typed.js` 未复刻 py 的 `pending_since_cut` 形如 `cut-[1-9][0-9]*` 检查（以等于 `first_known_cut` 代替），可补。
- N3：回退早于建立点为 O(rank) 重放，等待期通常短，可接受。

本地审查分支 `review/w-port-oracle-20261019`（`97f07d4ee9`，未推送）位于 `scratch-wt/`。
