# 收据 workers/wb-rewrite-0042/REPORT.md

- 原件：`workers/wb-rewrite-0042/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`a2058e4dbbfea0b0b0f5bef38879684af4dcc2741d9c0319b8f987dee9bc876b`
- 字节数：4653；行数：52

## 关键行（行号: 原文）

```
1: # #1488 0042 write_budget 锁改写 —— REPORT
3: - 工位 budget-handoff-fix-v2-checkout，起点 `38dd2ce493`，终点 `35ceec8cf9`（tree `90f56fa409`）。只追加提交，没有 push 主分支，没有合 main。
4: - 依据：主控裁定 (i) https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6281614540 ；control/1488-wb-rewrite-0042.md。
5: - **结论：已按要求把原锁与正控锁合并为 `audit_owner_write_budget_retains_candidate_within_retention_cap_and_parent_limit`，这是本票第四处裁改断言。快通道全锁 78/78 PASS；CI run 39361551191 两个 job 都是 success，testonly 组 78/78 PASS。负控变红，命中目标断言。**
7: ## 1. 改动
22:   - 本票第四处裁改断言，附裁定链接；
27: `.github/1488-uo-locks.txt`：删了两条旧名，加了新名，分节注释改为指向 0042 和裁定。锁清单从 79 条减到 78 条。注意第 6 行原本在“原六 driver（判据逐字不变）”节里，现在这一节只剩五条。
31: ## 2. 提交表
35: | f49db3d7e7 | 合并锁 + 锁清单（第四处裁改断言，附裁定链接） |
39: ## 3. 验证
42: - 负控 @8c3ffa120f：新锁 FAILED，红在 driver.rs:2093“完整候选未保留，retained=320877 仅为 raw”（`/tmp/nc-remote/20261020-104813-8c3ffa120f.log`），命中目标断言。随后已回退。
44:   - `uo (s_session,s_capture_native_testonly)` success，锁 **78 PASS**；
45:   - `uo (s_session)` success，1 PASS，其余 77 条 NOT_LISTED（testonly 锁照例不在这一组）。
47: - 作废的一次 CI：run 39361049484。第一次 revert 命令参数写错没有执行，这次 CI 实际跑在负控 8c3ffa120f 上，testonly 组多条 FAIL。这些红是恒释放负控连带造成的，与 0039 负控 1 的现象一致，不是合并锁的结果。
49: ## 4. 未闭合项
51: - **开放风险**：“8MiB 足够”没有证明（【05】）。按裁定随整票验收的资源读数一起报告。新锁的 eprintln 已备好字段，采读数需要加 `--nocapture`。
52: - **driver-six 冻结 CI 仍列旧名**：`.github/scripts/run-1488-driver-ci.py:30` 的 TESTS 还列着 `audit_owner_write_budget_releases_candidate_after_successful_normal_append`。这个 workflow 只在 `validate/1488-driver-six` 分支 push 时触发，绑定了已审源哈希（REVIEWED_SOURCE 9160fc9dee，SOURCE_FILES 里的 driver.rs sha256 早已不是现值），而且 CI 文件按
```
