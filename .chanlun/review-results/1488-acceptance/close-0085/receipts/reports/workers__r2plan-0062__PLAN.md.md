# 收据 workers/r2plan-0062/PLAN.md

- 原件：`workers/r2plan-0062/PLAN.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`f68fcb234e3968bef900a0c44b9c5ef63c69b09664b58dea9d620fa82c10d769`
- 字节数：18136；行数：133

## 关键行（行号: 原文）

```
1: # r2plan-0062 PLAN：R2 六阶段 O(N) 来源定位与 Δ_R 化方案（只写方案）
3: > 约束：#1612 裁定前不升格、不拆票、不改生产路径。本文件只是方案。行号以 `accept-s4-checkout` HEAD `ba83f5fc60` 为准（与合并面 c2bdf9b662 只差 cfg(test)）。
7: ## 0. 先讲清楚的前提
9: 1. **生产构建里只有完整路径。** `--features s_session` 下每代都走 `generate_impl(delta=None)`（`prepared.rs:1067-1068` → `generation.rs:563/577`）。增量入口 `generate_bound`（`generation.rs:591`）、Native 捕获、keeper、候选保留都在 `s_capture_native_testonly` 之后（S4 REPORT §5）。所以 R2 的六阶段 Δ_R 化**只对 testonly 增量臂有意义**，直到 #1612 裁定是否升格。
17: ## 1. 逐阶段 O(N) 来源
21: ### 1.1 generate（`generation.rs:658` `generate_impl`）
33: ### 1.2 object_plan（`object_index.rs:1187` `plan`）
39: ### 1.3 prepare_source_audit（`source_audit.rs:1489` `audit_generated` → `:2143` `audit_generated_in_store`）
45: ### 1.4 assemble_manifest（`prepared.rs:1120` `finish_node_products`）
60: ### 1.5 stage_indexes（`prepared.rs:1000-1034`）
66: ### 1.6 publish_borrowed（`commit.rs:63` → `:144` `publish_prepared_inner`）
74: ## 2. 逐阶段 Δ_R 化方案
80: | prepare_source_audit | ① **先修 S4 §3.1 的根因**：对象差分用的身份摘要必须排除每代都变的字段（推测为 generation/frontier 一类，需先核实），让 `apply_order` 的删除/新增集合等于真实 Δ。② ordinary 分支的原始账改用持久的逐事件哈希台账（只追加），每代只哈希新事件。③ 组、pending、形态、见证、观察的核验只覆盖脏对象，加上它们引用的 raw 坐标（划分不变式改为逐坐标增量维护）。 | `source_audit.rs`（audit_generated_in_store、apply_order、objec
81: | assemble_manifest | ① aux 计划不再克隆整份旧索引：按 Index 变更行喂 `sync_splice`（MAP 原建议）。② `raw.prefix` 两次全量 `sha_json` 改为复用 generate 已算的 raw 节点摘要（`all_nodes`）。③ `public_sets` 改为四条持久序列的后缀 splice。④ `assemble_manifest_body` 的全闭包 `verify_ref` 改为「只核本事务新落节点 + 根边界」，**需裁定**，因为注释明确禁止用路径算子替代全闭包。 | `aux_index.rs`（plan、pub
85: ## 3. 可并行切分（#1612 裁定后才能拆票；这里只给建议的切分面）
87: - **P1（独立，先做）**：prepare_source_audit ①，修对象身份摘要，让 `apply_order` 的差分正确。写集只在 `source_audit.rs`。它直接决定增量臂的来源审计能不能降到 O(Δ)，也是 S4 交 D6/#1491 的第一项。
88: - **P2**：stage_indexes ①② 计数器与批量尾检。写集 `audit_owner.rs` 的 construction 段加 `prepared.rs` testonly 臂，与 P1 不相交。
89: - **P3**：publish_borrowed ①②③。写集 `content_publication.rs`、`commit.rs`、`stored_clock`，与 P1、P2 不相交。
92: - **P6（裁定门）**：assemble_manifest ④（全闭包核验放宽）和 prepare_source_audit ②③（核验面缩小）。两项都是 spec 级，与 #1481 方案三合并裁定，不单独开工。
95: ## 4. 与段 B #1491（bounded-layout/1，ADR 0037）的边界
99: - 所以 assemble_manifest ①③、generate ③ 的后缀 splice 在 layout/1 下仍然会随 g 增长。这部分残差登记给 #1491，不在 R2 内追求有界。这条按 D6 的建议口径处理，裁定仍待主控。
102: ## 5. 与 CC-007 的边界
107: ## 6. 候选保留预算随代增长的有界化（不改资源帽）
126: ## 7. 建议顺序
128: P1（身份摘要修复）和 P2（计数器）先行。两者都很小，而且会改变后续阶段的读量基线。之后 P3、P4 并行，P5 跟在 P4 后，P6 与 #1481 方案三一起裁定。§6 的 1–3 只在 #1612 决定升格增量路径时才开工。
130: ## 8. 与 #1612 的关系（只陈述，不裁定）
```
