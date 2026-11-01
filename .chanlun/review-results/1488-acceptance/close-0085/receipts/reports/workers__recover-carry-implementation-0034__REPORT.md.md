# 收据 workers/recover-carry-implementation-0034/REPORT.md

- 原件：`workers/recover-carry-implementation-0034/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`0fb1ec641e1e130d5ea85eca1d3288c192c35e416b867af6007e5753c03e1780`
- 字节数：12376；行数：83

## 关键行（行号: 原文）

```
1: # #1488 Recover 段返修报告（0034，按独审 0032 + 主控裁定 issuecomment-6278995984）
3: - 起点：`ea363d5070`（59 PASS / 2 已知 FAIL）
4: - 实施终点（树）：`bb7f310baa`；负控全部 revert 后的 HEAD：`69e2568ea4`（与 `bb7f310baa` 的 `git diff` 为空）
8: ## 1. 写集
20: ## 2. 各 finding 状态
22: ### P2-1 补锁：已修复（4 组锁，每组都有负控）
31: ### P3-1 预算隐式携带：已修复（主控裁定，原定“不改代码”作废）
39: ### P3-2 旧 epoch 请求撤销当前合法资格：不改代码（按 0034）
43: ### P3-3 RecoverHandoff 门不可达：已修复（删去冗余门，合并为唯一门）
49: ### P3-4 L3 未断言原因：已修复
53: ### P3-5 Recover 可用性变化：不改代码（按 0034）
55: 现状：owner 持有资格时，Recover 走 strict，需要 effects recording 额度。迁移失败会让这次 Recover 失败并撤销资格，重试走非 strict 路径。理由：这是“交接失败即撤销、禁止兜底”的直接结果；放宽为“失败自动降级成功”就是被禁止的兜底。与 journal limit/8 帽段的交互（normal_append/write_budget 已知 FAIL）留给该独立小段处理，调整上限或粒度时须纳入 Recover strict 额度。
57: ### P3-6 无锁守卫：已修复（两条守卫各有锁与负控）
62: ### P3-7 非 feature dead_code 警告：已修复
66: ## 3. 验证
68: - 快通道全锁（HEAD `69e2568ea4`，`20261020-075109-69e2568ea4.log`）：**66 PASS / 2 FAIL**。2 条 FAIL 是已知的 journal 帽段 `normal_append…`（“stage缺材料cut、重入或顺序倒退”）和 `write_budget…`（“publication暂存资源已失效”），消息与 0031 逐字相同。
70: - CI（uo-ci-fixed.sh）：run 39344926664，feature job 66 PASS / 2 已知 FAIL，见 §4。
72: ## 4. CI
74: CI run **39344926664**（`issue-1488-uo-iter.yml`，head_sha `69e2568ea4756f115541d624a9df457c8509d8c3`），conclusion 为 failure，与起点 run 39332712343 形态相同：
75: - `uo (s_session)`：success，build.log 无 error；locks.tsv 为 1 PASS / 67 NOT_LISTED。
76: - `uo (s_session,s_capture_native_testonly)`：failure，build.log 无 error；locks.tsv 为 **66 PASS / 2 FAIL(101)**。仅 `normal_append…`、`write_budget…`（journal 帽段）两条已知项失败；14 条 recover_lock 全部 PASS。
80: ## 5. 提交表（ea363d5070..69e2568ea4）
```
