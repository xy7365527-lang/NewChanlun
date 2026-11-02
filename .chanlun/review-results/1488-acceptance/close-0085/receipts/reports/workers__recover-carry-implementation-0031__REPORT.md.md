# 收据 workers/recover-carry-implementation-0031/REPORT.md

- 原件：`workers/recover-carry-implementation-0031/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`2672f9cce5ef747758234dfb3fd31377f6685c2467e8358a416145d9e5dfe2e0`
- 字节数：12163；行数：106

## 关键行（行号: 原文）

```
1: # #1488 Recover/Carry 段实施报告（0031）
3: - 工位：`budget-handoff-fix-v2-checkout`；起点 `62bad42a1b`；终点 **`ea363d5070`**（tree `319709cf9721a1eee19d7d33ce8d65f804f1eb5a`）。
5: - 结论：两条目标锁（accept_tail、incremental）已转 PASS；新增 7 条锁全部 PASS，每条都有负控，负控提交时变红并随即 revert；宽对拍没有新增失败。
7: ## 1. 合同条款 → 实现 → 锁
9: | 合同条款 | 实现（终点行号） | 锁 / 负控 |
11: | 保留冻结六测中同 owner Recover 成功 | `audit_owner.rs:2808-2835` 新增受控迁移分支：Recover 事务内若尚无构造、owner 持有已提交资格，就把原资格迁入事务（strict），不再走 `invalidate_construction`。 | 原锁 `audit_owner_complete_accept_tail_…`、`audit_owner_incremental_…` 由 FAIL（Carry租约已撤销）转 PASS，断言未改。 |
25: ## 2. 生命周期状态机（Recover 段）
43: ## 3. 提交表
68: 所有负控 revert 后，`git diff 7c51c1fff3 HEAD` 只剩 ad190e5a7f 的测试打印。
70: ## 4. 实际写集
77: - 负控临时改动过 `audit_owner/effects.rs`，均已 revert，终点不在写集中。
80: ## 5. 原断言核对
82: `git diff 62bad42a1b HEAD -- driver.rs` 的删除行数为 0。全仓删除行只有 9 行，都在 audit_owner.rs 生产代码中：`if next<=parse…` 改为先取 from 再比较（文案不变）；`FixedOperation::Recover` 改为带参；close_effects 调用改写为同时调用 confirm_recover。测试 01 与 incremental 测试逐字未动。
84: ## 6. 结果
90: ## 7. 未闭合项与反例
97: 6. 本段之外：`normal_append`、`write_budget` 仍 FAIL（journal 帽）。
100: ## 8. CI（uo-ci-fixed.sh，run 39332712343，sha ea363d50705b0cc8a99a170a0d6cc48e26e4c6e7）
102: - 总结论为 failure。按 job 拆开如下：
103:   - `uo (s_session)`（非 feature 构建）：**success**。build.log 中没有 error，只有既有 warning。锁表为 1 PASS / 60 NOT_LISTED（feature 门控，符合预期）。
104:   - `uo (s_session,s_capture_native_testonly)`：failure，结果为 **59 PASS / 2 FAIL**。两条 FAIL 是已知的 journal 帽段：`normal_append…`（“stage缺材料cut、重入或顺序倒退”）和 `write_budget…`（“publication暂存资源已失效”），消息与起点逐字相同。
105: - 两条目标锁 accept_tail、incremental 均 PASS；7 条 recover_lock_* 均 PASS。
```
