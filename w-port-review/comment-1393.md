## #1393 负责人视角：W 字段逐字核对（CF+W 整合 `f2763c5e`，/root/review_w_port）

对 W 冻结源 `de73545cbf` 逐字核对：`raw-initialization-w/1`、`CC-004.initialization`（槽 `W`）、`CC-004.undetermined_raw`、`first_raw_pair`、`direction_state`（`Undetermined`/`Established`）、`pending_since_cut`、`initialization_policy_revision`、组 `initial_direction_evidence`、CC-054 `domain=raw_adjacent_W_initialization`、请求 `["CC-005.inclusion_group","pending"]`。v2 写者（全量/增量共用 `add_initialization`）、`tb02b::W_CONTRACT`、`s_tb02_contract.py`、`q3-typed.js`/`q3-consumer.js`/`tb01c-client.js` **全部逐字一致**；W 单域历史串仅 `…segment-second/1+raw-initialization-w/1` 显式映射。parser 语义与 Lean 一致，独立 oracle 未能打破（详见 #1419 同轮评论）。

**不一致处（W 字段在 S3 路径未落）**：S3 CC-054 `domain` 写的是 `established_direction_with_extreme_root_sets`（`content_producer.rs:1416`），与 W 后继串及 `source_audit` 要求的 `raw_adjacent_W_initialization` 冲突；S3 也不发 `CC-004.initialization`/`undetermined_raw`、CC-054 缺 `initialization_policy_revision`/`direction_state`。另 `content_producer.rs` `CASE_WAITING` 测试仍断言旧「永久未定」。s_session 未编译，以上为静态结论。

报告：`/Volumes/AgentStorage/issue1419-cfw-integrate-20261019/w-port-review/REVIEW.md`
