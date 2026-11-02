# 收据 workers/fix-review-0090/REVIEW.md

- 原件：`workers/fix-review-0090/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`6c446cf3b4da7b74976220f6f9fef53c49971b2fbfb5549935d7a213965ccfe2`
- 字节数：14482；行数：239

## 关键行（行号: 原文）

```
1: # fix-review-0090：0089 改动非作者独审
5: - 依据：任务书 control/1488-fix-review-0090.md；作者报告 workers/fix-0089/REPORT.md；缺陷出处 workers/gap-review-0087/REVIEW.md §1 ①-F-1、§5 ⑤-P2-1。
7: ## 结论：**CONDITIONAL**（0 个 P1，1 个 P2，3 个 P3）
11: 唯一的条件是 P2-1：作者用来证明「生产路径不可达」的前提「生产 cache limit=0」与代码不符，需要更正这条论证。
17: ## 1. ①-F-1 修复（d2b6e25a5d，`rust/src/bin/s_session_v2/cache.rs` 的 `remove_cert_admitted`，:396-445）
19: ### 1.1 各失败路径：无漏账、无重复记账
41: ### 1.2 成功路径与修复前语义等价
51: ### 1.3 红测试能区分缺陷
65: ### 1.4 生产 nobid
75: ### 1.5 「生产路径不可达（limit=0）」论证**不属实**，见 P2-1
79: ## 2. ⑤-P2-1 负控（eb668db7f0 / a7481aacb5）
81: ### 2.1 变异位置
90: ### 2.2 红点
97: 结论：红点落在可达闭包逐位比较处，属于跨臂对拍断言。
99: ### 2.3 revert
105: ## 3. S4 CI 形态（ffb89f7fa8）
107: ### 3.1 测试本身
114: ### 3.2 锁表
124: ### 3.3 issue-1488-uo-iter.yml 的 release 步
132: - **判定**：只有 rc==0 且出现 `test result: ok. 1 passed` 才记 PASS_RELEASE；否则记 FAIL_RELEASE 并置 fail=1，最后 `exit $fail` 使 job 失败。`set +e` 在这里是有意为之。
137: ### 3.4 main ci.yml 不会以 debug 跑到 S4
144: ### 3.5 uo-ci run 39588126034
149: - uo (s_session)：success。release 两步为 skipped。locks.tsv 中 4 条 PASS、93 条 NOT_LISTED。
150: - uo (s_session,s_capture_native_testonly)：success。build release-only locks 与 release-only locks 两步都是 success。locks.tsv 中 97 条 PASS、1 条 **PASS_RELEASE**。
155: ### 3.6 uo-remote.sh
161: ## 4. 327aa94c39 只动空白
164: - 两版文件去掉全部空白（空格、制表、换行）后，sha256 都是 `144d65a7…21ac0`，完全相同。
165: - 结论：只动空白，属实。
169: ## Findings
171: ### P2-1：「生产路径不可达」的前提「生产 cache limit=0」与代码不符
189: - 受影响的是验收记录：「生产行为不变」这一结论目前依托在一条错误的论证上。
200: ### P3-1：注释里的 S4 时长与实测不符，可能误导日后的超时取值
207: ### P3-2：S4 release 锁只挂在 CI 分支的工作流上
211: - 负责人的裁定允许二选一，作者也在 REPORT §6 第 2 点主动披露了这一点。
215: ### P3-3：作者验证日志 185314 开头有 shell 引号错误
223: ## 区间外线索（不计入 verdict，未核可达性）
231: ## 本审远端收据
237: | 20261021-200122-eb668db7f0.log | refs/uo1488/r90-eb668db7f0 | `--release --exact` S4 | FAILED，prepared.rs:1853 |
238: | 20261021-200251-b4eac1860f.log | refs/uo1488/r90-b4eac1860f | `--exact` 红测试（native） | FAILED，cache.rs:2232，272/4432 |
```
