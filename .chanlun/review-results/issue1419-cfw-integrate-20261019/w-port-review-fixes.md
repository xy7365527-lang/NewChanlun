# #1419 W 移植独审阻断项处理（B1/B3/N1）

依据：独审 #1419 issuecomment-6265478755，报告 `w-port-review/REVIEW.md`。B2（S3 发 W 对象）与 #1565 无关项之外不在本轮。

## 合入独审 oracle（N1）
cherry-pick `review/w-port-oracle-20261019`：`a55f3855e0`（`inclusion_wreview_tests.rs` + `inclusion.rs` 挂 mod）、`425101092a`（借用修复）。

## B1：S3 知识域与来源审计一致
- `content_producer.rs::build_knowledge_state`：`domain` 由 `CURRENT_CONTRACT` 改 `tb02b::W_CONTRACT.knowledge_domain`（S3 写者只写 W 后继串，`source_audit.rs` 按 `contract_for_rule(W)` 核 `raw_adjacent_W_initialization`）；载荷补 `initialization_policy_revision = raw-initialization-w/1` 与 `direction_state`（`Undetermined`/`Established`），与 v2 种子写者 `tb02a_facts` CC-054 同字段同值。新增参数 `initial_direction_unsettled`，`generation.rs` 传 `state.unsettled()`，测试传 `facts.initial_direction_unsettled`。
- `source_pending.rs::equal_is_evidence`：W 合同（`..CURRENT_CONTRACT`，只换知识域）与 /2 同按「同价极值是证据」口径，否则 W 串下步骤审计会把 `equal_extreme_identity` 当等待原因。
- 未改：S3 来源审计未对 CC-054 新两字段做值校验（审计目前只核 `domain`/`scope`/来源）——是否补「`direction_state` 与同 cut 步骤一致」的审计，随 #1565 一并定。

## B3：旧判据残留
- `CASE_WAITING` 拆为 `CASE_ESTABLISH_AT_THIRD`（原数据；W 下第三根建立、从 raw0 重折）与 `CASE_NEVER_ESTABLISHED = [(0,10,5),(1,9,6),(2,11,4)]`（相邻对全包含）。
- `cc004_record_matches_full_oracle_bytes` / `cc005_group_matches_full_oracle_bytes`：对拍样例换成建立样例（cc005 期望 any_direction=true）；永不建立样例的 S3 发布形态阻塞于 #1565，注明未纳入。
- 原「seed 后全部 waiting」块改为：建立样例无 waiting 步、无 `initial_direction_unsettled`、组0 成员 [0,1]、各步来源 = W 闭式 raw[..=max(k,2)]；永不建立样例全 waiting、各步只绑定自身 raw、零合并零组。
- **同族真缺陷（顺带修）**：`raw_prefix_coords` 原取「末步 coords」当前缀；W 等待期 waiting 步只绑 `single(incoming)`，S3 在等待期给 CC-054 的来源会只剩末根，被自身审计（「CC054来源不是同cut全部生效原始账」）拒绝。改为取状态机维护的 `raw_prefix`：`inclusion.rs` 新增 `IncrFactsState::shared_raw_prefix()`（退化态 `None`）。测试 `raw_prefix_chain_is_last_step_coords_for_every_prefix` 更名 `..._is_full_raw_prefix_...`，加入两样例，末步 coords 期望按 W 分已建立/等待期。

## 验证
- `nc-remote … cargo test --locked --lib theta_v0::parser`（`c42098154c`）：182 passed / 0 failed / 9 ignored（含独审 oracle 3 项），日志 `/tmp/nc-remote/20261019-185731-c42098154c.log`。
- `content_producer.rs`/`source_pending.rs`/`generation.rs` 属 `s_session`，因 #1488 缺件未编译、未执行。
