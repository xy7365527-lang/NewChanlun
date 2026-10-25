# 收据 workers/journal-independent-review-0038/REVIEW.md

- 原件：`workers/journal-independent-review-0038/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`e9e442a7c825327d5588835a271e18b640980213da67be0114bb80805bad03ff`
- 字节数：14099；行数：134

## 关键行（行号: 原文）

```
1: # #1488 0038 独立评审：journal key 合并 + originals 增量许可（非作者）
4: - 被审差异：`git diff 69e2568ea4 cc9fe7e1d6 -- rust/ .github/`，共 3 个文件：`effects.rs`（+369/−58 量级）、`content_manifest.rs`、`.github/1488-uo-locks.txt`
5: - 依据：主控裁定 issuecomment-6278995984 第二、三条；任务 `control/1488-journal-0035.md`；`workers/journal-merge-implementation-0035/{REPORT,NOTES}.md`；后续裁定 issuecomment-6280706052（write_budget 走 (c)）
8: ## 结论：**PASS_WITH_FINDINGS**
10: - P1：0
11: - P2：1 项。write_budget 锁仍红，裁定第三条要求的“两锁须在 CI 转绿”没有闭合；该项已上报，并已按 (c) 另行裁定
14: 被审的代码改动本身没有发现会导致误判通过、少计分配或改动业务语义的缺陷。
18: ## 逐项
20: ### 1. effects 仍逐收据可核 —— PASS
40: - 结论：不存在溢出或截断后被判通过的路径
42: ### 2. 第三处裁改断言 —— PASS（附 P3-1）
45: - 提交 `aa2d954a35` 的消息写明了裁定号和“本票第三处”；票上评论 6280703988 也已登记
47:   - N1 `737f1479de`：`if set && (fresh || inserted)`，即只丢 Noop 事件。日志 `/tmp/nc-remote/20261020-082649-737f1479de.log:2229-2237`，ordered 锁 FAILED，消息“不可因摘要相同丢Noop occurrence”；篡改锁也 FAILED
48:   - N3 `23160fd40d`：去掉闭合核。`…082830…log:2229`，篡改锁仍为 ok，说明当时缺尾部用例。补上 `f752905884` 后做 N3' `3bc4fe09f4`：`…083017…log:2230-2236`，FAILED，panic 在 `effects.rs:2726`。“去闭合核→转红”属实
49:   - 三个负控都已 revert，HEAD 的源码与 `f752905884` 一致
51: ### 3. journal-coexist-cap 与只计增长块 —— PASS
61: ### 4. originals 增量许可 —— PASS（附 P3-3）
63: - `tree_slots` 对 len 线性（`audit_retention.rs:145-152`：`len × (11·size_of<(K,V)>() + 16·usize)`），所以 `Σ tree_slots(1) = tree_slots(n)`，结论严格成立
67: - 负控 N2 `844ddd55df`：`…083314…log:2229-2236` FAILED，“峰值不得含整树重复许可：peak=4968 live=3432 per=1536”。已 revert 到 `cc9fe7e1d6`
68: - 我在 HEAD 补跑了 content_manifest 与 content_store 两个模块：42 passed / 0 failed / 2 ignored（`/tmp/nc-remote/20261020-085905-cc9fe7e1d6.log`）。补跑原因见 P3-4
70: ### 5. 禁区未动 + cargo check —— PASS
73: - effects.rs 中的改动只在 Journal 容器、收据登记、stage 坐标和 validate 范围内
77: ### 6. write_budget 剩余失败的解释 —— 结论成立，因果论证证据不足（P3-2）
94: ### 7. 测试证据 —— PASS
96: - CI run 39349246070：`gh run view` 显示 headSha=`cc9fe7e1d6e2…`，conclusion=failure。两个 job 分别是 testonly 组 failure、s_session 组 success
97: - artifact `workers/uo-ci-runs/39349246070/*-1-cc9fe…/locks.tsv`：73 PASS、1 FAIL(101)，失败的是 write_budget。新增和改动的 6 条锁都是 PASS
98: - `*-0-…`：1 PASS、73 NOT_LISTED
100: - 宽对拍：基线 `075335`（69e2568ea4）225P/18F，HEAD `083702` 228P/17F
106: ## Findings
108: ### P2-1　write_budget 锁仍红，裁定第三条“两锁须在 CI 转绿”未闭合
110: - 反例：CI 39349246070 中 `audit_owner_write_budget_releases_candidate_after_successful_normal_append` FAIL(101)，left 3602154 right 0
111: - 处置：作者没有越界修改，已经上报；主控已在 issuecomment-6280706052 裁定走 (c)，即订正夹具、补正控、非作者独审。本段不能以全绿结项，需等 (c) 的返修和复审
113: ### P3-1　ordered 锁中 `tail[1].material_epoch > tail[0].material_epoch` 已变为恒真
120: ### P3-2　REPORT §6 因果表述过强，且后续裁定的一处前提需要订正
122: - 裁定前提订正：issuecomment-6280706052 说“夹具拧错了旋钮（audit_cache_source_bytes vs complete_retention_cap − raw_retained）”。实际上 `complete_retention_cap()` 就是由该夹具设的 `audit_cache_source_bytes` 推出来的：`request_audit.rs:2194-2200` 为 `total − max(cache, legacy)`
126: ### P3-3　originals merge 失败出口在 `originals.insert` 之后
131: ### P3-4　宽对拍过滤器不含 content_manifest，篡改锁注释与用例不符
```
