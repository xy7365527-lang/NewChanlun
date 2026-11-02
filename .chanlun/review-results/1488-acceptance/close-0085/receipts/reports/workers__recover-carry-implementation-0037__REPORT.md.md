# 收据 workers/recover-carry-implementation-0037/REPORT.md

- 原件：`workers/recover-carry-implementation-0037/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`6ee2c5a47390b0a65b36a6400ea037b5bc40ff06d3bb66c5ccf8f4ef488b5ce0`
- 字节数：10187；行数：82

## 关键行（行号: 原文）

```
1: # #1488 Recover 父账绑定返修（0037，按独审 0036 P2-1）
3: - 工位 `budget-handoff-fix-v2-checkout`，起点 HEAD `cc9fe7e1d6`（0035 journal 段终点，未改动其任何提交）；终点 **`280437d999f58522d9cebef9215182f51993b898`**（tree 与实施提交 `27ff299759` 相同）。
5: - 验收依据：主控裁定 issuecomment-6278995984 第一条（绑定父账 id 与冻结预算摘要/epoch，不一致则拒绝；仍过原总容量门，不重绑、不换账）。
7: ## 1. P2-1 修法
18: ## 2. 新锁（已追加到 `.github/1488-uo-locks.txt`）
27: ## 3. 负控（每条都是去掉守卫后变红，并已随即 revert）
37: ## 4. 测试证据
39: - **快通道全锁** `280437d999`：`/tmp/nc-remote/20261020-090457-280437d999.log`，75 ok / 1 FAILED。失败的是 `audit_owner_write_budget_releases_candidate_after_successful_normal_append`（`driver.rs:2004`，left 3602194 right 0），属已知未闭合项，见 0035 REPORT。基线 `cc9fe7e1d6` 为 73 ok / 1 FAILED（left 3602154）。+40B 来自资格新增的 `Budg
41: - **CI run 39352182056**（uo-ci-fixed.sh，head_sha=`280437d999f58522d9cebef9215182f51993b898`，conclusion=failure）：`uo (s_session)` success，1 PASS / 75 NOT_LISTED；`uo (s_session,s_capture_native_testonly)` failure，**75 PASS / 1 FAIL(101)=write_budget**。基线 run 39349246070 为 73 PASS / 1 FAIL。两组 build.log
43: ## 5. 实际写集与禁区核对
51: ## 6. 已知取舍与未闭合项
53: - **冻结域按精确相等比对**：这是对裁定“不一致拒绝”的字面实现。如果某个请求重绑了候选冻结域（bind_complete_budget→handoff），但没有重新提交资格（该事务没有构造），之后的 Recover 会因“所记冻结域不符”被拒，并撤销资格走冷审，属 fail-closed。现有全锁和宽对拍中没有出现这条路径。若主控认为应放宽为“同 Ledger 且记录域不晚于当前域”，只需改 `construction_state.rs:550` 一处；N1b 已证明只核 Ledger 时域段锁会变红。
54: - `write_budget` 锁仍为 FAIL，属于 0035 段的已知未闭合项（锁的期望与资源保留规则冲突），本段没有处理。
57: ## 7. P3-1..5 现状登记（不改代码，整票验收前由负责人逐条给处置）
67: ## 8. 提交表
74: | 0a59f45a6f / 280437d999 | N2 / revert（终点） |
76: ## 勘误（0077 P12，P3-REGISTER R-12 / 0040 P3-4，2026-10-21 追加）
79: - 0037 tree `280437d999`：远端快通道与宽对拍为 3602194，CI run 39352182056 为 3602192，同一 tree 运行间相差 2B。
82: - 引用时以注明来源的单次日志为准。该锁在 0042 起已改名并改写断言（见 P18），这里的数值只作历史记录。
```
