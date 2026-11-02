# 收据 workers/fix-0069/REPORT.md

- 原件：`workers/fix-0069/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`4056c5399c1e2f5aca9e7b9752a37d86a743aa30c4205ba8ddd892fe399bea69`
- 字节数：10106；行数：108

## 关键行（行号: 原文）

```
1: # #1488 fix-0069 报告：按独审 0068 收紧降级判据
3: - 工位：`budget-handoff-fix-v2-checkout`，分支 `codex/1488-budget-full-chain-fix-v2-20261018`；起点 HEAD `44d312d7cf`（含 0067 合并）。
8: - 改动范围：只改 testonly 门内代码。`audit_owner.rs` 731 行前只有第 688 行文档注释原位改写（行数不变），其余改动都在文件尾 gated impl；`effects.rs` 整体处于 `s_capture_native_testonly` 门内；`driver.rs` 只改文件末尾 tests mod。生产代码 `CodecWork::run`（scratch.rs:420）未改。资源帽、codec/业务语义、冻结不变量、原测试断言、Cargo/features 均未改。
10: ## 1. F1：降级判据收紧
16: - 新增判据 `publication_degrade_verdict(error, operation_error)`：
30: 1. `v3::driver::tests::publication_real_error_after_budget_failure_rejects_without_degrade`：F1 负控。这是独审探针 e（`refs/r68-red/e` = `50af198264`）场景在新 HEAD 上的移植：尾检先 `reject(Allocation)`，再返回 `stale("publication尾检身份不符（0069负控注入）")`。修后断言请求被拒且错误原样，不降级。
32: 3. `v3::audit_owner::publication_degrade_verdict_tests::publication_degrade_verdict_only_for_pure_budget_shortfall`：判据真值表。桥文本取自真实 `WorkScope::reject` 的输出，不是手写常量。
36: ## 2. 红证（负控，悬空提交，父均为 `2d58d00ecb`，ref `refs/f69-red/*`）
48: ## 3. I1：第二次 `runtime.discard` 红证 d
50: 独审红证 d 原为 `049d284e0c`（基于旧 HEAD）。在新 HEAD 上重建为 `ba722f7c9e`，结果见上表：已有降级锁 `publication_candidate_shortfall_*` 与新锁 2 都会变红。由此该变异已被锁表覆盖，并在此记为负控。
52: ## 4. 生产二进制不变（复核）
56: | 提交 | 原始 sha256 | 去 build-id+strip sha256 |
59: | 5aff2d1b41（HEAD） | `bc4757ae…`（12,753,968 B） | `c81b4c5f26213e15832aa6616b510f8d5efebdfd48508510f0b5714e375c1ea6` |
61: **逐字节一致**。未处理二进制的差异只来自 build-id 段与符号/调试信息，去除后完全相同。两种 feature 组合 `cargo check` 均 rc=0，warning 数 359/358，与起点相同。日志：`runs/20261021-033354-5aff2d1b41.log`（HEAD）。
63: ## 5. 全锁 + 宽对拍
65: - 全锁表（HEAD `5aff2d1b41`）：LOCKS=90，`test result: ok. 90 passed; 0 failed`（`runs/20261021-033548-5aff2d1b41.log`）。
66: - 宽对拍 `v3::`（HEAD `5aff2d1b41`，testonly features，`--test-threads 4`）：`657 passed; 23 failed; 14 ignored`（`runs/20261021-033858-5aff2d1b41.log`）。23 件失败**与期望完全一致**：
71: ## 6. F2、F3：0065 报告订正
73: 以下订正 `workers/degrade-0065/REPORT.md` §5、§6、§3。VERDICT（PASS，INVARIANT 31/31）不变。
75: **F2-1 S86 CONDITIONAL 差异归因（订正 §5“native 在放弃候选前的复用减少了中间节点写入”）**
103: ## 7. 局限与说明
105: - 桥识别是文本白名单。`CodecError::scratch` 的显示文本（s_content.rs:126 单点生成）若改动，真实预算不足会被拒绝而不是降级。这是 fail-closed 方向，真值表与锁 2 会立即变红提示。更强的做法是让 effects 链路携带类型化的 `ScratchFailure`，但那会动到 testonly 门外的错误类型，不在本单范围。
```
