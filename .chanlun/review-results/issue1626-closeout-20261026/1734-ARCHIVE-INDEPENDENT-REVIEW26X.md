# PR #1734 文书归档非作者独审

## 裁决

**文书内容：PASS。合入门尚未满足。** 21 个新增文件都是 `.chanlun/` 下的原件、日志、索引或 roster。没有改代码、判据或 ADR。15 个复制原件逐字节等于来源。索引的引用全部能解析，数字也和原件一致。候选、历史和 main 三层分得清楚，关票尾差没有被说成已完成。PR head 的 CI 还在跑，这次 PASS 只管文书内容，不替代 CI 和用户的 main 批准。

有一项 MEDIUM 发现。它不影响 PASS，建议合入前补一句索引。

- reviewer：`/root/review1734_archive26x`，非作者，工位只读。
- 冻结 head：`636600a07eb832c404212e7263dc3ca593c80a78`（本地 `git rev-parse HEAD`、远端 `ls-remote` 和 PR `headRefOid` 三处一致）。
- base：`df7beb0c8da5d8f3b6053f85813b86613dcc6849`，是 head 的祖先，中间只有一个提交 `636600a07e docs(1626): 归档最终独审与主线验收证据`。
- 现场时间：`2026-10-26T05:51:11Z`（gh 读取）。报告时间：`2026-10-26T05:53:12Z`（`date -u`）。

## 实际核查与证据

### 一、diff 范围

`git diff --name-status base..head`：21 个文件全部是 `A`，共 +851 行。20 个在 `.chanlun/review-results/issue1626-closeout-20261026/` 下，另 1 个是 `.chanlun/agent-roster-1626-closeout-20261026.md`。`rust/ formal/ src/ tests/ scripts/ docs/` 下零改动。

### 二、原件与来源逐字节 `cmp`

15 个复制件全部 `SAME`：

| 仓内文件 | 来源 |
| --- | --- |
| `1626-85b866-FULL-REVIEW.md` | `/Volumes/AgentStorage/issue1626-85b866-final-review-20261025/REVIEW.md` |
| `1626-b936311-X38-REVIEW.md`、`1626-b936311-lean-targeted.log` | `/Volumes/AgentStorage/issue1626-b936311-final-review-20261025/{REVIEW.md,lean-targeted.log}` |
| `1626-f1f336-BSP-REVIEW.md` | `/Volumes/AgentStorage/issue1626-f1f336-anchor-review-20261025/REVIEW.md` |
| `1626-bsp-anchor-validation/{default,backtest-bin}.{log,exit}`（4 个） | `/Volumes/AgentStorage/issue1626-bsp-anchor-20261025/` 同名文件 |
| `1693-INTEGRATION-REVIEW.md`、`1693-MAIN-CI-TERMINAL26T.md`、`1693-MAIN-INDEPENDENT-REVIEW26B.md`、`1693-MAIN-INTEGRATION26B.md`、`1693-MAIN-REVIEW-CONSUMPTION26B.md`、`1693-integration-remote.log`、`1626-WORKTREE-HANDLES26U.txt` | `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/` 同名文件 |
| `1693-main-integration26b-validation/20261026-103749-2554f669bd.log`、`…/20261026-104044-2fee8af14f.log` | 同目录 `1693-main-integration26b-validation/` 同名文件 |

作者新写的是 `README.md`、`1626-WORKTREE-DISPOSITION26U.md` 和 roster。CI JSON 是 gh 输出，下面第五节核了它的内容。

### 三、索引引用与数字

用脚本解析 README 的 15 个相对链接，在 head 树里全部存在，含 `../issue1626-versioned-fields-20261025/IMPLEMENTATION.md` 和两个子目录。另有两个 URL，也已现场核过。

README 的每项数字都对上了原件：

- `2554f669` 日志在 `:44/:57/:76` 分别是 20、6、12 passed，0 failed。`:2` 有 `Diff in …positional.rs:1978`，所以首轮 fmt 失败，README 照实写了。`:78` 是 `EXIT=0`。
- `2fee8af1` 日志在 `:89` 是 78 passed，`:97` 是 1 passed，`:105` 是 `cargo check` Finished，`:106` 是 `EXIT=0`，全程没有 `Diff in`。
- 20+6+12+78+1=117。复用前 38 项的依据是 `git diff 2554f669..2fee8af1`，只改了 `rust/src/trading/positional.rs:1981-1982` 的一处断行（+2/-2）。另外 `2fee8af1` 的父提交正是 `2554f669`。
- `1693-integration-remote.log` 在 `:112/:123/:131` 是 11、4、1 passed，`:133` 是 `EXIT=0`，与第一轮尾审 `:30` 一致。
- `backtest-bin.log` 是 2914 passed、0 failed、153 ignored，两个 `.exit` 都是 `0`。`default.log` 只有末段，README `:36` 也明说不能据此声明总测试数。
- 第一轮尾审 `:5` 写 PASS、`:8` 写候选 `3e587ab2…`。85b866 审查在 `:25` 判 BLOCK，`:65-67` 判 Q-4/Q-2/Q-6～Q-8 PASS。b936311 审查在 `:23` 判 PASS 且 X-38 HIGH 闭合。f1f336 审查在 `:23` 判 PASS。README `:30-32` 的描述与这些全部一致。

### 四、错误引用订正

原独审 `1693-MAIN-INDEPENDENT-REVIEW26B.md:59` 写的是 `rust/src/trading/recursive_tower.rs`，原文没动。`git ls-tree -r 2fee8af1` 里的真实路径是 `rust/src/theta_v0/classifier/recursive_tower.rs`。README `:26` 给出的复核命令我本地重跑了，即 `git diff --name-only 9f13b8ef… 2fee8af1… -- '*recursive_tower.rs' formal/`，输出为空、rc=0。作为对照，两个 head 之间的全量 diff 有 137 个路径，可见这条空输出确实来自路径过滤。消费报告 `:5` 也有同样的订正。

### 五、候选、历史与 main 的区分（现场 gh）

- PR #1693：`headRefOid=2fee8af1…`，`mergeCommit=df7beb0c…`，`mergedAt=2026-10-26T04:17:04Z`。与 README `:7` 和 26T `:3` 一致。
- main CI 40268271555：`headSha=df7beb0c…`，`completed/success`，`event=push`。仓内 JSON 有 20 个 job 是 success，1 个 skipped，没有失败或待执行的 job。
- #1626：`CLOSED/COMPLETED`。timeline 里的 closed 事件是 `2026-10-26T04:17:07Z`，`commit_id=df7beb0c…`，与 26T `:7` 一致。issue 字段 `closedAt` 是 `04:17:06Z`，差 1 秒，属于 API 字段口径不同，不是原件写错。
- README `:3`、`:18` 和 26T `:7` 都写了：自动关票不等于关票门通过，原件里的 OPEN 和待 CI 是形成时的状态。DISPOSITION `:27` 写了：tracker CLOSED 不能代替交付纪律第 5 条。PR 正文写的是 `Part of #1626`，`closingIssuesReferences=[]`，不会再触发关票。

### 六、工位处置复核（只读）

我现场重跑了一遍，三个工位的结论都与 DISPOSITION 一致：

| 工位 | HEAD | porcelain | ignored | 分类外 ignored | 是 `2fee8af1` 祖先 |
| --- | --- | --- | --- | --- | --- |
| `/Volumes/AgentStorage/issue1626-versioned-fields-20261025/worktree` | `f1f336dc…`（`sandcastle/issue-1626`） | 0 | 1314 | 0 | 是 |
| `/Volumes/AgentStorage/issue1693-integration-20261026/worktree` | `2fee8af1…`（`codex/1693-main-integration-20261026`） | 0 | 0 | 0 | 是 |
| `/Volumes/AgentStorage/pr1693-consume1723-20261026` | `9f13b8ef…`（detached） | 0 | 0 | 0 | 是 |

`git diff --name-only 2fee8af1 df7beb0c` 列出 12 个路径，都是 #1488/#1516 的收尾文书、roster 和 `.github/devskim-reviewed-1488.json`。限定到 `rust/ formal/ src/ tests/ scripts/ docs/adr/` 后为 0。三个工位都还在，没有被删。这次没有重跑 lsof，活句柄只采用原件 `05:39:51Z` 那一刻的观测。

### 七、diff-check

`git diff --check base..head` 的 rc=2，共 5 处，全部在原件里：`1693-MAIN-INDEPENDENT-REVIEW26B.md:46`、`1693-integration-remote.log:1`、`20261026-103749-2554f669bd.log:1` 和 `:12`、`20261026-104044-2fee8af14f.log:1`。作者新写的三个文件都没有被报告。这和交接里说的「第46行及三份远端日志，共五处」相符。

### 八、PR #1734 现场状态

`OPEN`，非 draft，base 是 `main`，`MERGEABLE`，`mergeStateStatus=UNSTABLE`。CI 40275808455 里只有 `rust-check` 已经 pass，`macos-session` 是 skipping，其余 rust-test、`fixture-drift`、`sandcastle-ts`、`test`、`s-observer` 和 DevSkim 40275808418 都还是 pending。远端 main 已经推进到 `b0bab458…`。用 gh compare 查了 `df7beb0c...b0bab458`，没有动到本 PR 路径的文件。

## 发现

**MEDIUM-1：更早的三轮整票 BLOCK 没有进仓，索引也没提。** 在 85b866 之前，同一张票还有三份非作者终审，结论都是 FAIL/BLOCK：

- `/Volumes/AgentStorage/issue1626-85accb-final-review-20261025/REVIEW.md:40`，概念层 H-1；
- `/Volumes/AgentStorage/issue1626-d0162a-final-review-20261025/REVIEW.md:41`，Q-2 H-1；
- `/Volumes/AgentStorage/issue1626-fe1710-final-review-20261025/REVIEW.md:28`，X-38 HIGH-1，是 Lean 测试不在默认根、负控缺失。

这三份后来都被 85b866 和 b936311 解除。但仓内 85b866 原件、README 和 `IMPLEMENTATION.md` 都没有提到它们，读者会以为 85b866 是第一轮整票审查。已经进仓的部分都是真的，也没有篡改，最终候选的验收链本身是完整的，所以这条不构成 BLOCK。不过它没有完全满足「历史 BLOCK 如实保留」。建议作者在 README `:30` 前补一句，写明这三轮 BLOCK 的候选号、来源路径和解除它们的审查，或者把三份原件一起归档。这只是文书修改，由原作者负责。

**LOW-1：ignored 清单原件没有进仓。** DISPOSITION `:11` 的 1314 个和「分类外为 0」来自本机 `1626-WORKTREE-AUDIT26U.txt`（1350 行），仓里只放了活句柄原件。我现场重算数字一致。数字要等清理时再核，所以这条不用改。

**INFO：** 交付纪律第 5 条要求的工位清理还没做，#1279 和整个 Goal 也都不随本 PR 关闭。文书里已经如实写明，这里只是记录状态。

## 未核项

- PR #1734 head 的 CI 还没终结，我没有等到结果。合入前主控需要核必需检查的终态。
- 没有重跑 lsof 去核活句柄。按 DISPOSITION 的要求，清理 owner 动手前要重新核一次。
- 没有重跑 Rust 或 Lean，也没有在 macOS 上跑 cargo/rustc。实现和测试结论沿用已有的有效证据，没有重审。
- 没有核 `IMPLEMENTATION.md` 的正文，它在 main 上已存在，不在本 diff 范围内。

## 实际命令摘要

`git rev-parse HEAD`；`git diff --stat/--name-status/--check base..head`；`git merge-base --is-ancestor`；对 15 个原件逐个 `cmp`；用 Python 脚本解析 README 链接，并对照 `git ls-tree -r head`；对三份日志 `grep 'test result|EXIT|Diff in'`；`git diff 2554f669 2fee8af1`；`git ls-tree -r 2fee8af1 | grep recursive_tower`；`git diff --name-only 9f13b8ef 2fee8af1 -- '*recursive_tower.rs' formal/`；`gh pr view 1734/1693`；`gh pr checks 1734`；`gh run view 40268271555`；`gh issue view 1626`；`gh api …/issues/1626/timeline`；`gh api …/compare/df7beb0c...b0bab458`；`git ls-remote origin`；对三个工位跑 `git status --porcelain`、`ls-files --ignored` 和 `merge-base`。以上全部只读。我没有改任何候选文件，也没有改 GitHub 状态。
