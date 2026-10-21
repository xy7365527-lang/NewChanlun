# PR #1592 尾部补审（#1588 ci-time）：1e4858d4fd..3fee4c1aff

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1592
- 上一次完整评审：`1592-claude-fable-5.md`（PASS-WITH-NITS，覆盖到 `1e4858d4fd`，当时 base `1d70db893f`）
- 本次评审面：作者按 nit 追加的尾部提交 `2549200dff`（nit 1–3 + P3 A2）与 `3fee4c1aff`（证据文件）。分支其间 rebase 到新 main `24c2fa068f`。
- 评审模型：claude-fable-5；日期 2026-10-20；只读，未改代码、未推送、未评论。

## 结论：PASS

四条 nit 均按建议修了且修法正确；`!cancelled()` 替换在 ci.yml 内 10 处全改、语义与 GitHub 官方推荐一致；并发组 run_id 兜底的理由成立；`overwrite: true` 作用域只限本片；verify 缺 `executed.tsv` 的结构化失败有自测且本机通过；证据文字抽查属实；rebase 未改变任何原提交内容；无越界改动。两条信息项不阻塞。

## rebase 保真与评审面界定

- `git range-diff 1e4858d4fd~3..1e4858d4fd 24c2fa068f..3fee4c1aff`：原三个提交 `744722236e/5054bc12e4/1e4858d4fd` 对应 `4eb0c8ad80/31e89af2b1/18dd4c5914` 全部为 `=`（内容逐字相同）；新增两个提交 `2549200dff`、`3fee4c1aff`。
- `git diff --stat 1d70db893f 24c2fa068f -- .github .chanlun/review-results/eng-debt-20261020/ci-time` 为空：main 在旧 base→新 base 之间没有碰过本 PR 的路径，因此 `git diff 1e4858d4fd 3fee4c1aff -- <PR 路径>` 与 `git diff 18dd4c5914 3fee4c1aff -- <PR 路径>` 字节相同（`cmp` 一致）。
- 尾部 diff 只有 4 个文件：`ci.yml`（+24/−13）、`test-shard.py`（+5/−1）、`test_test_shard.py`（+9）、`ci-time-1588.md`（+50/−7）。无其它路径。
- 证据文件说第二轮 CI 跑的是 `54ed5be736`（rebase 到 `a0522447ca` 时的修复提交）；之后分支再次 rebase 到 `24c2fa068f`。`git range-diff a0522447ca..54ed5be736 24c2fa068f..2549200dff` 四个提交全 `=`，`git diff 54ed5be736 2549200dff -- <PR 路径>` 为空，`a0522447ca..24c2fa068f` 不碰 `.github`。所以 run 39409029909 对当前 `.github` 内容有效；且 HEAD `3fee4c1aff` 自身另有一个 success 的 PR run 39425283861。

## 逐条核实

### nit 1：`overwrite: true`（ci.yml:287）
- 按建议加在分片上传步。artifact 名为 `s-session-shard-${{ matrix.shard }}`，每片唯一，overwrite 只替换本片自己首次尝试的产物，不会碰其它片。coverage job 用 `pattern: s-session-shard-*` 下载，单片 re-run 时其余三片产物仍是首次尝试的，合并后 4 目录齐全。正确。

### nit 2：分片 `timeout-minutes` 90 → 110（ci.yml:254）
- 按建议放宽，注释改为「只作挂死保护，看实测时长不看本值」，与评审意见一致。其余 job 的 timeout 未动（diff 中无其它 timeout 变更）。第二轮 run 最长片 58m38s，余量充足。

### nit 3：verify 缺 `executed.tsv` 结构化失败（test-shard.py:167-170）
- 缺文件走 `fail("Shard executed.tsv missing", ...)` → `::error title=...` + `sys.exit(1)`，不再抛 traceback。放在 `read_text` 之前、`check_partition` 之后，顺序合理。
- 自测 `test_verify_rejects_shard_without_execution_record`：跑 3 片后删 shard-1 的 executed.tsv，断言 rc==1、stderr 含 `::error title=Shard executed.tsv missing::`、不含 `Traceback`。本机 `python3 .github/scripts/test_test_shard.py`（Python 3.14.6）→ **Ran 11 tests … OK**；CI rust-check 日志同样 `Ran 11 tests … OK`。

### P3 A2：`if: always()` → `if: ${{ !cancelled() }}`
- `git grep -n "always()" 3fee4c1aff -- .github/workflows/ci.yml` 只剩第 316 行注释，10 处 `if:` 全改：test job 覆盖率上传 1、分片上传 1、coverage job 级 1、extras 5 步、s-observer 2 步。与证据「全部 10 处」一致。
- 语义：GitHub 表达式文档明确写「要在成功或失败后运行、但取消后不继续，用 `if: ${{ !cancelled() }}`」，即本改法。失败路径不变：前一步失败后续步仍跑、job 结论由失败步决定；分片失败（非取消）时 coverage job 仍跑并因缺目录判红。
- 其它 workflow（ci-macos.yml、devskim.yml、agent-*.yml）仍有 `always()`，但证据文件明确范围是「ci.yml 里全部 10 处」，不属本 PR 越界，也不算遗漏。

### P3 A2：workflow 级 `concurrency`（ci.yml:14-16）
- `group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}`、`cancel-in-progress: ${{ github.event_name == 'pull_request' }}`。
- pull_request 事件：`pull_request.number` 为真值 → 同一 PR 的所有 run 同组 `ci-CI-<n>`，cancel-in-progress 为 true，新推送取消旧 run。✓
- push main / workflow_dispatch：`github.event.pull_request` 为 null，表达式对 null 取属性返回 null → 回落 `run_id`，每个 run 独占一组，cancel-in-progress 为 false。✓
- 作者偏离指示（指示的回落值是 `github.ref`）的理由成立：GitHub 文档写明「同一并发组内任何已有 pending 的 job/workflow 会被取消」，与 cancel-in-progress 无关。若用 `github.ref`，连续合入 main 时夹在中间的那次 run 会被顶替取消，违背「push main 不取消」。run_id 兜底是正确修法。
- 实测：分支上 39416162112（11:36 起，约 59 分钟）与 39425283861（12:49 起）相隔 73 分钟，前者已自然跑完，故本轮没有机会观察到取消；取消逻辑靠语义核实。

### 证据文字抽查（run 39409029909）
- `gh run view 39409029909`：headSha `54ed5be736`，event pull_request，conclusion success。
- shard 0 `10:35:38→11:34:16` = **58m38s** ✓；workflow `10:35:36→11:34:26`（约 59 分钟）✓；rust-check `10:35:38→10:38:28` = 2m50s ✓；coverage `11:34:19→11:34:26` = 7s ✓。
- shard 0 日志 grep：`Xeon(R) Platinum 8370C`、`finished in 3373.34s` ✓。rust-check 日志：`Ran 11 tests in 1.062s` / `OK` ✓。
- 以上与证据文件条目 7 表格逐字一致。

## 发现

1. **信息 / 不阻塞** — `.github/scripts/test-shard.py:168` 注释「缺它说明该片中途被杀（超时、OOM 等）」与 `!cancelled()` 的交互：
   - OOM 杀的是 cargo 子进程，`cmd_run` 在 `proc.wait()` 之后无条件写 `executed.tsv`，所以 OOM 场景文件是**存在**的（走 `cargo_status` 非零返回），不会进这个分支。
   - 分片 job 超时时 GitHub 的处理是取消该 job（`timeout-minutes` 文档用词为 cancels）；job 内 `cancelled()` 随之为 true，上传步（现为 `!cancelled()`）被跳过，该片根本没有 artifact；coverage job 即便运行也走的是既有的 `Shard artifacts missing`，而不是新加的 `executed.tsv missing`。
   - 因此新分支在当前配置下几乎不可达（仅在 python 进程被杀而 job 未被取消的罕见情形）。它仍是无害的防御分支，自测有效，不影响票面；上一轮评审第 16 行「分片被取消/超时时 coverage 照样跑」在 `!cancelled()` 之后也不再成立，但任何一种情形下 run 都不是绿。若作者希望超时后仍能拿到分片计划产物做事后排障，可只对分片上传步保留 `always()`；是否这么做由作者/主控裁定，不要求改。
2. **信息 / 不阻塞** — 证据文件条目 7 写的修复提交是 `54ed5be736`，与现 HEAD 的 `2549200dff` 不同名（第二次 rebase 所致），文件未提第二次 rebase 和 HEAD 的 run 39425283861。已核实两者内容逐字相同、HEAD 自身 run 为 success，证据有效；读者按 SHA 对不上时以本文此条为注。

## 我实际复核过的命令

1. `git range-diff 1e4858d4fd~3..1e4858d4fd 24c2fa068f..3fee4c1aff`（3 个 `=`，2 个新增）；`git range-diff a0522447ca..54ed5be736 24c2fa068f..2549200dff`（4 个 `=`）。
2. `git diff --stat 1d70db893f 24c2fa068f -- .github .chanlun/review-results/eng-debt-20261020/ci-time`（空）；`git diff --stat a0522447ca 24c2fa068f -- .github`（空）；`git diff 54ed5be736 2549200dff -- <PR 路径>`（空）。
3. `git diff 1e4858d4fd 3fee4c1aff -- .github .chanlun/review-results/eng-debt-20261020/ci-time` 与 `git diff 18dd4c5914 3fee4c1aff -- …` `cmp` 相同；逐行阅读全部 diff。
4. `git grep -n "always()" 3fee4c1aff -- .github/workflows/ci.yml`（仅注释 1 处）；`… -- .github/workflows`（其它 workflow 保留，不在范围）。
5. 读 HEAD 的 `test-shard.py` `fail()`、`cmd_run`、`cmd_verify` 与 ci.yml 第 1–20、245–313 行。
6. `python3 .github/scripts/test_test_shard.py` → Ran 11 tests … OK（Python 3.14.6）。
7. `gh run view 39409029909 --repo xy7365527-lang/NewChanlun --json headSha,conclusion,status,event,createdAt,updatedAt,jobs`；`gh run list --branch eng-debt-1588/ci-time --workflow CI`；`gh run view --job 118581929602 --log | grep "Ran .* tests"`；shard 0 job 日志 grep 机型与 `finished in`。
8. GitHub 文档：workflow-syntax 的 concurrency 节（同组 pending 顶替规则、示例含 `github.run_id` 兜底）；expressions 的状态函数节（`!cancelled()` 为官方推荐写法）。
