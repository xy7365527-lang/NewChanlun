# 收据 workers/fix-0044/REPORT.md

- 原件：`workers/fix-0044/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`9981981da0520c6425341def70aa2a2a0aded7e3d6728806549cb3655043b16e`
- 字节数：8812；行数：99

## 关键行（行号: 原文）

```
1: # #1488 返修 0044（按复审 0043）—— REPORT
3: - 工位 budget-handoff-fix-v2-checkout，起点 `35ceec8cf9`，终点 **`fff166540e`**（tree 与实施提交 `cc5e324736` 相同，负控均已 revert，`git diff cc5e324736 fff166540e` 为空）。只追加提交，没有 push 主分支，没有合 main。
5: - **结论：五项都已完成。快通道全锁 80/80 PASS；CI run 39364508567 两个 job 都是 success（testonly 组 80 PASS）；宽对拍与基线相比，只多出本段两条新锁和 0042 的改名，16 条既有失败逐条不变。**
7: ## 1. 写集（净差 `35ceec8cf9..fff166540e`，6 文件 +127/−9）
20: ## 2. 逐项
22: ### P2-1：handoff_peak 上限改读生产实际生效值
29: - **口径**：生产只对 `handoff_peak` 去掉 entry 后的本请求域峰设门，不对整个 `handoff_peak` 设门。所以新断言是 `handoff_peak − entry ≤ complete_limit_for_test()`（`driver.rs:2165-2167`），其中 `limit` 直接读候选实际字段，`entry` 读生产写入的 `charge.entry_reserved_bytes`，测试里不再复述 `total − entry`。这样消除了复审指出的 entry 双计。控制文“断言 handoff_peak ≤ 该值”我按“与生产同一口径比较”
39: ### P3-3：保留读数改读生产值
43: ### P3-1：资格记录改指他账或旧域后经合法重绑，记录不前进
59: ### P3-2：receipt 路径正控
63: ### P3-4：§2.1 结论范围限定
70: “本锁原 ==0 不可达”的结论不受影响，它由释放阈值（M < 728,328B）与扫描共同支撑。合并锁的文档注释已同步改正（`driver.rs` 合并锁 doc）。
72: ## 3. 提交表
76: | cc5e324736 | 实施（P2-1/P3-3 断言改写、P3-1/P3-2 新锁、helper/trace、锁清单） |
82: ## 4. 验证
84: - 新锁与改写锁 @cc5e324736：3/3 ok（`/tmp/nc-remote/20261020-111000-cc5e324736.log`）。
87:   - 失败集只少了 0042 删掉的原 write_budget 锁，其余 16 条逐条相同（都是既有已知失败，不在本段范围）；
91:   - `uo (s_session,s_capture_native_testonly)`：**80 PASS**；
92:   - `uo (s_session)`：1 PASS / 79 NOT_LISTED（testonly 锁照例不在这一组）。
95: ## 5. 未闭合项
```
