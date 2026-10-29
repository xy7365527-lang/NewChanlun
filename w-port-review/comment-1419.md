## W 移植非作者独审（/root/review_w_port）：parser 核 PASS，W 移植整体 BLOCK

审查对象 `f2763c5e`（以 `50cf66a27e` 为准；`0defdcc22b` 对所审文件无改动，结论同样适用）。完整报告：`/Volumes/AgentStorage/issue1419-cfw-integrate-20261019/w-port-review/REVIEW.md`

**PASS：`inclusion.rs` W 状态机。** 1–500 行与 W `de73545cbf` 逐字节相同；`IncrFactsState` 的等待、建立（raw0 重折走同一 `fold`）、发证、回退（早于/恰在建立点）逐分支与 W 源和 Lean `RawInclusionInitialization` 一致，旧「建立前遇包含即永久未定」已删、parser 内无第二档。新增独立 Lean 转写 oracle（不复用 `first_raw_direction/fold_step`）：5 个手构反例 + 1500 随机序列（逐前缀、每个 rank 回退跨建立点、变化见证、已发证前缀不改写）**3/3 通过，未能打破**；`cargo test --lib theta_v0::parser` 远端 **179/0**（9 ignored 为既有）。

**BLOCK：**
1. **S3 知识域自相矛盾**：`content_producer.rs:1416` 写 `CURRENT_CONTRACT.knowledge_domain`，S3 只写 W 后继串，`source_audit.rs:1820` 要求 `raw_adjacent_W_initialization` ⟹ S3 生成物会被自身审计拒绝。
2. **S3 未发布 W 对象且发布 waiting 步**（与 v2 W 写者不同；即待裁定 3）。
3. **旧判据残留在 S 测试期望**：`content_producer.rs` `CASE_WAITING`（`:3439/:4092/:6074–6083`）断言「seed 后全部 waiting、永久未定」，但 W 下 (1)→(2) 为严格向上对、第三根即建立 ⟹ 编译后必败，须按 W 改写。
4. `--features s_session` 未编译：tb02a_facts 增量缓存、tb02b、W S 测试全未执行；字段/缓存只做了静态核对（逐字一致，缓存失效条件无遗漏）。

建议把 oracle 测试 `inclusion_wreview_tests.rs`（同目录）纳入分支：现有 delta 测试的「全量旧核」已是同一状态机，独立性不足。
