# 收据 workers/runner-evidence-0051/REVIEW.md

- 原件：`workers/runner-evidence-0051/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`85aca0313359e1586e1a2a0aa0b79c16447deeda16a26f1804c855e78d12cb8f`
- 字节数：7254；行数：56

## 关键行（行号: 原文）

```
1: # #1488 六 driver runner 运行证据非作者核验（0051）
4: - 只读：checkout 保持 detached `5a76881eee`，`git status` 为空；没有提交，没有 push。临时文件放在 `/tmp/r0051/`，核验脚本是 `/tmp/r0051/check.py`（sha256 `4b736aa6…1284`）
5: - **结论：PASS_EVIDENCE**，没有阻断项
7: ## 1. artifact 下载与 digest：一致
8: - API artifact `12240198080`：size 910448，digest `sha256:18e0e85ef97d34523a5d41cd507affc26c750152a6e797ba046fe202b573b870`，未过期。
9: - 在 `/Users/silencehan/Projects/NewChanlun` 中用 `gh api …/artifacts/12240198080/zip` 重新拉取原始 zip，`shasum -a 256` 结果为 `18e0e85e…b870`，和 API digest 一致，也和作业日志里 upload-artifact 打印的 “SHA256 digest of uploaded artifact zip” 一致。日志显示上传了 124 个文件、910448 字节。
12: ## 2. RESULT.json 中六条 PASS 与 tests/0N 原日志：一致
13: - RESULT 中 `verdict=CANDIDATE_SIX_PASS`，`passed=6 failed=0 not_run=0`；`unreaped_groups=[]`，`signals_observed.count=0`，`test_timeouts_observed=0`；没有 `error` 或 `finalization_error`。整个运行从开始到收尾耗时 254.4 s，在 2370 s 封口时限以内。
19:   - `test.stderr` 都是 0 字节。stdout 和 stderr 里没有 `panicked`、`FAILED`、`failures:`、`error[`、`thread '`。
20:   - `after-0N.output.json` 中 `status=PASS`，`receipt` 与 `test.json` 逐字段相等，没有 `output_error`，`diagnostics.count=0`，`scratch_remaining=[]`，`binary_unchanged=true`。
23: ## 3. list 唯一性：成立
26: ## 4. FILES.json / FILES.SEAL.json 封存：一致
27: - artifact 共 124 个文件。排除项正好是 runner `FINAL_EXCLUSIONS` 中实际存在的 6 个：`FILES.json`、`FILES.SEAL.json`、`RESULT.json`、`control/inventory.{json,stdout,stderr}`。其余 118 个文件和 FILES.json 的条目集合相等，逐个文件复算的 sha256 和 bytes 都是 0 不符。
29: - `sha256(FILES.json) == FILES.SEAL.files_sha256 == RESULT.files_sha256 == 031749c9…8bb9`，`SEAL.count = 118`。
32: ## 5. source 身份（580 哈希）：一致
34: - 对 7f0b61c 和 fff166540e 两个提交对象，各自逐文件 `git cat-file blob | sha256` 复算，580×2 全部 0 不符。
35: - `head`、`run_id=39368457887`、`attempt=1`、reviewed_source、reviewed_tree、legacy sha256 都与常量和 API 一致。`ci_files` 两个哈希等于 7f0b61c 中这两份文件的 sha256：runner `ef06d5cb…c688`，workflow `bc305524…d605`。
38: ## 6. binary 前后哈希：一致
39: - `verify-build.output.json.binary` 与 `RESULT.binary` 相等，sha256 为 `fa950a1d…eb7d`。所有 spec 中的 `binary` 都等于它。
43: ## 7. 单作业原日志：没有被吞掉的失败
45: - runner 步骤本身不向控制台输出内容，它在 `bash -e` 下成功结束，因此进程 exit 0。这与 RESULT 的 `good → return 0` 分支一致：如果 RESULT 不是 PASS，runner 会返回 1，步骤会变红。
48: ## 8. runner 文件 blob 与独审 0050：一致
54: ## 附注（非阻断）
55: - RESULT 自身声明的范围仍然有效：`candidate_only=true`，`full_234_and_ci=NOT_REPLACED`，`production_v4=BLOCK_NOT_MERGEABLE`，`supervisor_os_eintr_actual=NOT_RUN`，`synchronous_io_hard_deadline=NOT_QUALIFIED`。本次核验只证明这六条在远端精确候选上的运行证据完整且一致，不扩展为全量 234 测试或 CI 通过，也不代表 v4 可以合入。
```
