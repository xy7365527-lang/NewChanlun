# PR #1615 非作者独审（#1588 sandcastle 分片：评审提示摘要渲染+体积闸、本机 cargo 垫片）

- 评审者：Claude Fable 5（claude-fable-5-1），非作者，只读评审，未改代码、未评论 PR、未合入。
- 对象：PR #1615 `eng-debt-1588/sandcastle`，head `5d6f30c6f8`，base `origin/main` = `df6e680930`（merge-base 一致，分支领先 2 个提交）。
- 工位：`/Volumes/AgentStorage/eng-debt-20261020/sandcastle`。
- 备注：评审提示中 PR 号与模型标识字段被截断为乱码；PR 号由分支 `eng-debt-1588/sandcastle` 经 `gh pr list --head` 反查确定，模型标识沿用本目录既有命名。

## 结论：PASS-WITH-NITS

两项缺陷（③ 评审提示撑爆静默失败、④ 工蜂本机跑 cargo）按分片提示 `prompts/sandcastle.md` 的要求修复到位，关键主张经独立复核均成立；未发现掩盖问题、降覆盖或误改行为。剩余为低严重度的收尾建议，不阻塞合入。

## 轴①：正确性 / 风险

### 核心机制复核（均成立）

1. **inline prompt 不再做 `` !`cmd` `` 展开**：`node_modules/@ai-hero/sandcastle/dist/index.js:2390-2400` `isInlinePrompt → resolvedPrompt = rawPrompt`，`:2522` `skipPromptExpansion: isInlinePrompt`，`:397` 据此跳过 `preprocessPrompt`。提交标题 / issue 正文里的反引号不会被执行，测试 `review-context.test.mts` 已锁。
2. **noSandbox 的 env 覆盖生效**：`chunk-62WN33RK.js:13` `{ ...process.env, ...createOptions.env }`，PATH 前置垫片目录会真正进入工蜂进程；`claudeCode` provider env 默认 `{}`（`index.js:3417`），与沙盒 env 无键冲突（`mergeProviderEnv` 不会抛）。
3. **Claude Code 确实读取 `CLAUDE_ENV_FILE`**：本机 `claude` 2.1.221 二进制含 `process.env.CLAUDE_ENV_FILE; if(r) try { readFile(r) … "Session environment loaded from CLAUDE_ENV_FILE"`。
4. **失败通道真实可见**：`watcher.sh:28-30` 按 (ticket, phase) 取最新事件，`reviewer/failed` 会作为单独一行进 STATUS.md；`claim-loop.mts:83` 在 `implementer/started` 之后不回滚，`:278-282` exit≠0 触发 `rollbackStuckClaims` 对本票为 no-op，与 PR「保留认领不回滚」描述一致。
5. **merge-prompt 改分支名 ref 的理由成立**：本机实验 `git bundle create x <sha> HEAD` 的 bundle 头只有 `HEAD`，远端 `git fetch bundle '+refs/*:…'` 后 `git cat-file -e <sha>` 返回 NOT FOUND；换 `main` 则 FOUND。与 `~/.local/bin/nc-remote` 第 13-18 行的取法吻合。`NC_REPO` / `NC_TARGET` 两个变量 nc-remote 均支持（第 7 行、第 18 行）。
6. **未碰并行分支文件**：diff 文件清单不含 `codex-child.ts`、`delivery-bundle.ts`、`run-wayfinder-engine.sh`、`wayfinder_engine.mts`。
7. **不降覆盖**：无测试删除；新增 14 例；新增 CI job `sandcastle-ts` 把此前仅本机跑的 `.sandcastle` 测试纳入 CI。

### 发现（按严重度）

| # | 严重度 | 位置 | 理由 | 建议 |
|---|---|---|---|---|
| N1 | LOW（偏 MEDIUM） | `.sandcastle/main.mts:250-271` | `runReviewer` 只对 `runWithResume` 的异常调 `recordReviewerRunFailure`；`prepareReviewPrompt` 阶段的非超限异常（`gh issue view` 失败、`git diff` 失败等）会原样抛出，主循环 `:358` 直接 rethrow → exit 1，`workers.jsonl` 只剩 `sandbox/closed`，无 `reviewer/failed`、无票面评论。这正是本票要消灭的「静默结束」形态，只是换了触发原因（旧形态同样静默，故不是回归，但目标未完全覆盖）。 | 把 `prepareReviewPrompt` 也纳入同一 try/catch：非 `ReviewPromptTooLargeError` 的异常走 `recordReviewerRunFailure`（gh 不可达时 `notified=false` 仍能落 jsonl）。`main-swarm.mts:578-588` 同理。 |
| N2 | LOW | `.sandcastle/review-prompt.md:27-31` | `{{ISSUE}}` 放在 ``` 围栏内，issue 正文自身含 ``` 围栏时会提前闭合，摘要区块结构错乱（不影响体积闸与执行安全，只影响可读性）。 | 改用 `<issue>…</issue>` 标签或缩进块包裹。 |
| N3 | LOW | `.sandcastle/main.mts:57-62, 246` | `assertHostBranchIsMain` 的注释仍说「评审 TARGET_BRANCH 是 sandcastle 内置 promptArg」；评审面已改为宿主侧 `hostGit()` 以 `REVIEW_TARGET_BRANCH="main"` 取摘要，校验的真正理由变成「`git diff main...branch` 在宿主 cwd 解析，宿主 main 须是干净且最新的 main」。本机实测：本工位 `main` 落后 `origin/main` 时同一 #1406 分支渲染直接超 60k（exit 4），说明宿主 main 过旧会把别线提交混入摘要（claim-loop `refreshHostMain()` 在生产路径上先刷新，故生产无影响）。 | 更新注释措辞；可考虑在 `runReviewer` 前比对 `main` 与 `origin/main`，落后即 FAIL-loud。 |
| N4 | LOW | `.sandcastle/cargo-guard.mts:382` | 每次运行 `mkdtempSync` 生成垫片目录，无清理；长期 claim-loop 下 `$TMPDIR` 积累小目录（每份约 11 个小文件）。 | 进程退出时 `rmSync(root, {recursive:true})` 或复用固定路径。 |
| N5 | INFO | `.sandcastle/health-monitor.mts:96` | `TERMINAL` 含 `sandbox/closed`，`reviewer/failed` 之后仍有 `sandbox/closed`，health-monitor 不会把它当告警源。REPORT.md 遗留项 5 已如实登记。 | 转票处理（与遗留项 4「实装阶段失败同样静默」合并）。 |
| N6 | INFO | `.sandcastle/main-swarm.mts:163-176` | 蜂群路径超限时设 `exitCode=4` 但循环继续（该票产出不进 merge），且 docker 蜂群未挂垫片。REPORT.md 已登记为遗留（蜂群休眠中）。 | 无需本 PR 处理。 |

无 HIGH / MEDIUM 阻塞项。

## 轴②：是否满足 #1588 票面验收

- #1588 票面验收条目（LEDGER、DevSkim 转绿、基线失败测试、CI 时长、每项修复非作者独审）由主控分片承担；本 PR 对应「第 5 项：各票 P3/观察项中当前就错的部分」里的 sandcastle 分片（③④），分片题面见 `prompts/sandcastle.md`。
- 对照分片题面逐项：
  - ③ 摘要化提示（`--stat` / 提交列表 / `--name-status`）✔；按文件/按提交读 diff 的指引 ✔；体积上限（估算 60k token）✔；超限显式失败：exit 4 ✔、`workers.jsonl` 结构化记录 ✔、沿用现成通知通道（STATUS.md via watcher + gh 票面评论）✔；测试：超大 diff 夹具断言不含正文且在上限内 ✔、超限断言显式失败与 mock 通知被调用 ✔。
  - ④ 提示明写禁本机 cargo 并给 nc-remote / CI 路径（implement / review / merge / CODING_STANDARDS 四处）✔；沙盒层 PATH 前置垫片 ✔；测试 `cargo --version` 被拦且指向 nc-remote ✔；断言 nc-remote 本地段不调 cargo ✔（runner 无 nc-remote 时设计内 skip）。
  - 证据文件 `.chanlun/review-results/eng-debt-20261020/sandcastle/REPORT.md` 含现象、复现命令与输出、根因、处置、测试/演练输出 ✔，结构与 LEDGER 要求的「现象/复现/根因/处置」一致，可直接并入 LEDGER。
  - 提交、推送、开 PR 到 main、CI 通过 ✔；未合入 ✔。
- 结论：满足本分片验收；票面整体验收由主控汇总。

## 我实际复核过的事实与命令

1. `gh pr list --head eng-debt-1588/sandcastle` → #1615；`gh pr view 1615`、`gh pr diff 1615`、`gh issue view 1588`。
2. `git fetch origin main`；`git log origin/main..HEAD` = 2 个提交；`git merge-base origin/main HEAD` = `df6e680930`。
3. 本机重跑：`npm run typecheck` 通过；`npx tsx --test .sandcastle/*.test.ts .sandcastle/*.test.mts` → `tests 167 / pass 167 / fail 0 / skipped 0`（与 PR 描述一致）。
4. #1406 对照（只读，取 `host/issue-1406`，target `origin/main`，issue 正文用真实 `gh issue view 1406`）：`old: chars=1574443 ceil(len/4)=393611`（与 #1406 评审日志 `~393611 tokens` 完全一致）；`new: chars=25447 est_tokens=8062 limit=60000`，提示不含 `diff --git`，37 个提交齐全。PR 报的 19118/5411 是用占位 issue 正文算的，差异来源明确。
5. 演练 A 独立复现（`envfile-only` 变体：不改 PATH，只给 `CLAUDE_ENV_FILE`，真实 `claude --print --permission-mode bypassPermissions --model claude-opus-5-5`，cwd `/tmp`）：Bash 工具里 `command -v cargo` → 垫片路径，`cargo --version` 打出 nc-remote 指引，`CARGO_EXIT=87`。
6. HEAD-bundle 实验（临时仓）：`git bundle list-heads` 仅 `HEAD`；远端式 `git fetch bundle '+refs/*:refs/nc-remote/x/*'` 后 `git cat-file -e <sha>` NOT FOUND；分支名 bundle 则 FOUND。
7. sandcastle 库源码：`dist/index.js:2390-2400, 2522, 397`（inline prompt 跳过展开）；`chunk-62WN33RK.js:13`（noSandbox env 合并）；`index.js:3417`（claudeCode env 默认空）。
8. `claude` 2.1.221 二进制 `grep CLAUDE_ENV_FILE` 命中读取逻辑。
9. CI run 39448904664（head `5d6f30c6f8`）：conclusion success；job test / rust-check / s-observer / fixture-drift / sandcastle-ts 均 success，macos-session skipped；sandcastle-ts 日志 `# tests 167 / pass 166 / fail 0 / skipped 1`。DevSkim run 39448904692 失败，日志 55575 行中 `review-context|cargo-guard|review-prompt|main-swarm|main.mts|implement-prompt|merge-prompt` 命中 0 条，确认与本 PR 文件无关。
10. 读源：`.sandcastle/main.mts` 全文、`watcher.sh`、`claim-loop.mts:270-282`、`health-monitor.mts:96`、`~/.local/bin/nc-remote` 全文、`review-prompt.md`、REPORT.md 全文。
11. 未做：未重跑演练 B（nc-remote 远端 `cargo --version`），因本 PR 不改 Rust 源码，且 nc-remote 脚本静态核已覆盖；未在生产 main.mts 全链路起真实工蜂。
