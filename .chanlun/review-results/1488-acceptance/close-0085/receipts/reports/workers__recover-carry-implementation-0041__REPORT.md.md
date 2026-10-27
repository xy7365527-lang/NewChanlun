# 收据 workers/recover-carry-implementation-0041/REPORT.md

- 原件：`workers/recover-carry-implementation-0041/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`201e0dc5207aa33f00c48ff05c453e67a1718d31c81f75d839dda7275750b56a`
- 字节数：9423；行数：85

## 关键行（行号: 原文）

```
1: # #1488 Recover 绑定前进（0041，按独审 0040 P2-1 与主控裁定 (a)）
3: - 工位 `budget-handoff-fix-v2-checkout`，起点 HEAD `f23344909e`（0039 终点，其提交未改动）；终点 **`38dd2ce49337474c6e8eeb088a30e8852561bd22`**（tree `34950126c3…`，与实施提交 `a6fead42c6` 相同）。
7: ## 1. 裁定 (a) 的实现
17: ## 2. 锁
21: | **新** `recover_lock_promotes_after_new_request_readonly_rebind`（`driver.rs:1675`） | 0040 反例原序列：recover_lock_driver（Begin21，请求 B）→ `native.handle(ready)`（同 owner 新请求 C）→ `conn.recover("2")`。断言：C 后候选父账已换域（旧摘要不再 matches）、资格记录 matches 新候选、Recover 成功、epoch=2、晋升资格 has_parent、持久 writer_epoch=2 | PASS |
22: | **改** `recover_lock_rejects_qualification_recorded_on_other_parent_or_domain` 第二段（`driver.rs:1636-1660`） | 拆成两段：(2a) 经合法重绑点（新请求 ready）换域 → Recover 成功，epoch=2；(2b) 绕过重绑点直接 handoff 改域（P3-1：正额度，沿用原 allowance）→ 逐字拒绝并撤销（原期望与 `assert_recover_revoked` 原样保留）。他账段、正控段不动；doc 注释同步 | PASS |
23: | **新** `recover_lock_rejects_qualification_without_parent_record`（`driver.rs:1703`） | P3-3：抹掉资格记录 → 报“Recover迁移资格缺父账记录”，撤销，epoch 不动 | PASS |
24: | 保留 `recover_lock_rejects_foreign_ledger_budget_handoff`（伪造交接摘要：他账/同账 skew 域） | 未改 | PASS |
26: 锁名说明：control 第 3 条写的是 `recover_lock_rejects_foreign_ledger_budget_handoff` 第二段，但该段是伪造摘要 skew（属第 3 条末句“伪造域拒绝负控保留”）；与“经合法重绑点换域→成功”描述及 0040 P2-1 所指相符的是 0037 新锁 `recover_lock_rejects_qualification_recorded_on_other_parent_or_domain` 第二段，按后者改。
30: ## 3. 负控（随即 revert）
39: ## 4. 测试证据
41: - **快通道全锁** `38dd2ce493`：`/tmp/nc-remote/20261020-102422-38dd2ce493.log`，78 ok / 1 FAILED=`audit_owner_write_budget_releases_candidate_after_successful_normal_append`（left 3602193，已知未闭合项）。基线 `f23344909e`（`094620`）76/1（left 3602192）。
44: - **CI**：run **39359439955**（uo-ci-fixed.sh，head_sha=`38dd2ce49337474c6e8eeb088a30e8852561bd22`，conclusion=failure）：`uo (s_session)` success，1 PASS / 78 NOT_LISTED；`uo (s_session,s_capture_native_testonly)` failure，**78 PASS / 1 FAIL(101)=write_budget**（left 3602192）。4 条 0041 相关锁（2 新 + 0037 两锁）及全部 r
46: ## 5. 实际写集与禁区
54: ## 6. P3 处置
65: ## 7. 未闭合
67: - `write_budget` 锁仍 FAIL（0035/0039 段已知项），本段未处理。
71: ## 8. 提交表
77: | c6177cf2f2 / 38dd2ce493 | NB / revert（终点） |
79: ## 勘误（0077 P12，P3-REGISTER R-12 / 0040 P3-4，2026-10-21 追加）
82: - 0037 tree `280437d999`：远端快通道与宽对拍为 3602194，CI run 39352182056 为 3602192，同一 tree 运行间相差 2B。
85: - 引用时以注明来源的单次日志为准。该锁在 0042 起已改名并改写断言（见 P18），这里的数值只作历史记录。
```
