# 收据 workers/merge-0047/REPORT.md

- 原件：`workers/merge-0047/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`fdd0b8faef0ecedcce000d2e60d0fc357278feb360cbfee426d12a622af87d33`
- 字节数：13140；行数：89

## 关键行（行号: 原文）

```
1: # #1488 冻结源 × 当前 main 合并报告（0047）
3: ## 1. 基本信息
16: ## 2. 冲突逐块表
20: ### 2.1 文本冲突（git 标记块）
27: | G3 | generation.rs `build_shape` 的 `Ok(None)` 分支 | 冻结源旧分支（视为 pending） | `271b8e8cc6` 改为错误 | `(_, Ok(None)) => return Err(format!("GenError：形态窗 {start} 缺同次三组证据"))` | main 的新不变式（三组证据同次产出），冻结源无相反裁定 |
33: | P1 | `s_session_v3/source_pending.rs` `audit` 头部 | `a36c252589` `#[cfg(test)] source_call_note` | `dccb69793c` `let equal_is_evidence = equal_is_evidence(rule)?;` | 两行都保留 | 互不冲突 |
34: | P2 | source_pending.rs 每步记录 | `9a3b27e208` 抽出 `step_record` + `StepView` trait + `StepContribution`（原生贡献，feature `s_capture_native_testonly`） | `dccb69793c` 内联 `waiting_count` 版本化逻辑 | `step_record(s, object, raw, equal_is_evidence)` 移植 main 的 `waiting_count` 逻辑；`StepView` 增 `fn waiting_count(&self
35: | P3 | source_pending.rs 请求记录 | `request_record_with_slots` 抽取 | `request_reasons(…, waiting_count, missing_direction, equal_is_evidence)` | `request_record`/`request_record_with_slots` 增 `equal_is_evidence` 参数，调用 `request_reasons(s, step.reasons(), &reasons, step.waiting_count(), missing_direction,
38: ### 2.2 无文本冲突但语义必须调和的点（merge 中的手工改动）
44: | S3 | `s_session_v3/source_contributions.rs` | `pending_phase`/`request_phase` 调 P2/P3 新签名 | 各自 `let equal_is_evidence = source_pending::equal_is_evidence(self.rule.as_str())?;` 并传入 `StepContribution::bind`/`request_record_with_slots`；`equal_is_evidence` 提为 `pub(super)` |
49: ## 3. 测试结果
60: | CI `uo-ci-fixed.sh` run 39370285846 | 组 `uo (s_session)`：**success**（编译过；锁 1 PASS + 79 NOT_LISTED，feature 门控，与基线同形）；组 `uo (s_session,s_capture_native_testonly)`：**failure = 79 PASS + r14 FAIL(101)**，与快通道一致 | 冻结源 run 39364508567：组0 1 PASS + 79 NOT_LISTED；组1 80/80 | `workers/uo-ci-runs/39370285846/`
64: ### 3.1 选用的冲突相关测试（过滤器）
79: ## 4. 未闭合项
81: 1. **锁 r14 字面值反例（需主控/非作者裁定，未改锁）**
85:    - 为什么停在这里：锁断言要求逐字保留，main 的合同字符串又是 #1405 已裁定的新行为，两边不能同时按字面成立。可选处置（任选其一，需裁定）：(a) 重锚 r14 的字面值（111,334→111,361，527,048 对应 +27），由非作者签字；(b) 把 r14 改为从合同字符串长度推导期望值。本次两种都没有做。
89: 5. 合并解法须由**非作者审阅**（主控裁定），审阅前不得合入 main。
```
