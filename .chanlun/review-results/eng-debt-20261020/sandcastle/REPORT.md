# #1588 工程债清零 · sandcastle 分片报告（③ 评审提示撑爆、④ 工蜂本机跑 cargo）

- 分片：sandcastle；分支 `eng-debt-1588/sandcastle`（基于 origin/main `df6e680930`）
- 作者：Claude Code（实施者）。本报告不含非作者独审结论，独审由主控另行安排。
- 涉及文件：`.sandcastle/{review-context.mts, review-context.test.mts, cargo-guard.mts, cargo-guard.test.mts, main.mts, main-swarm.mts, review-prompt.md, implement-prompt.md, merge-prompt.md, CODING_STANDARDS.md, README.md}`、`.github/workflows/ci.yml`（末尾新增 `sandcastle-ts` job）。
- 未碰并行分支的文件：`.sandcastle/codex-child.ts`、`.sandcastle/delivery-bundle.ts`（p3fix / PR #1611）、`.sandcastle/run-wayfinder-engine.sh`、`scripts/wayfinder_engine.mts`（wayfinder）。

---

## ③ 评审工蜂把整段 diff 塞进提示，#1406 撑爆后静默结束

### 现象

#1406 实装工蜂产出 37 个提交后，评审工蜂启动 4 秒即失败，日志只有一行 `Prompt is too long`。
`workers.jsonl` 停在 `reviewer/started → sandbox/closed`，没有任何失败事件；票保持已认领，
票面无评论，STATUS.md 看上去像「评审进行过」。

### 复现命令与输出

宿主侧现场（只读）：

```console
$ cat /Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/logs/sandcastle-issue-1406-reviewer.log
--- Run started: 2026-10-20T14:36:22.072Z ---
Iteration 1/1
Setting up sandbox
Setting up sandbox done (0.1s)
Expanding shell expressions
  git diff main...sandcastle/issue-1406 → ~393611 tokens
  git log main..sandcastle/issue-1406 --oneline → ~575 tokens
  gh issue view 1406 --json title,body → ~1587 tokens
Expanding shell expressions done (1.5s)
Agent started
Prompt is too long

$ grep -n '"ticket":1406' .../.sandcastle/logs/workers.jsonl | tail -5
557:{"ts":"2026-10-20T14:18:24.916Z","ticket":1406,"branch":"sandcastle/issue-1406","phase":"claim","status":"claimed"}
558:{"ts":"2026-10-20T14:18:29.319Z","ticket":1406,"branch":"sandcastle/issue-1406","phase":"implementer","status":"started"}
559:{"ts":"2026-10-20T14:36:22.051Z","ticket":1406,"branch":"sandcastle/issue-1406","phase":"implementer","status":"completed","commits":37}
560:{"ts":"2026-10-20T14:36:22.053Z","ticket":1406,"branch":"sandcastle/issue-1406","phase":"reviewer","status":"started"}
561:{"ts":"2026-10-20T14:36:26.728Z","ticket":1406,"branch":"sandcastle/issue-1406","phase":"sandbox","status":"closed"}
```

同一分支上对比旧形态与新形态的提示体积（`/tmp/nc-1406-render.mts`，只读，issue 正文用占位文本、不联网）：

```console
$ npx tsx /tmp/nc-1406-render.mts
old: git diff main...sandcastle/issue-1406 chars=1574443 est_tokens=415441 ceil(len/4)=393611
new: rendered prompt chars=19118 est_tokens=5411 limit=60000
commits=37 files=90
```

`ceil(len/4)=393611` 与 sandcastle 日志里的 `~393611 tokens` 一致，确认就是这一段展开撑爆的。
新形态在同一分支上只有约 5.4k 估算 token，为上限的 9%。

### 根因

1. `.sandcastle/review-prompt.md` 用 sandcastle 的 shell 展开 ``!`git diff {{TARGET_BRANCH}}...{{BRANCH}}` ``
   把整段 diff 原样塞进提示，没有任何体积上限。分支一大就超过模型上下文。
2. 失败不显式：`main.mts` 的 Phase 2 只在成功后写 `reviewer/completed`，抛错时 `finally` 只写
   `sandbox/closed`，进程以 1 退出。`claim-loop.mts` 的回滚判据在出现 `implementer/started` 后
   不回滚（`claim-loop.mts:83`），`health-monitor.mts:96` 又把 `sandbox/closed` 当终态。
   结果是没有失败记录、没有通知，票一直挂在已认领状态。
3. `main-swarm.mts`（docker 蜂群）的评审也用同一模板，同样有这个问题。

### 处置：修

- 新增 `.sandcastle/review-context.mts`，在宿主侧渲染评审提示：
  - 提示只放变更摘要：`git diff --stat=200 main...branch`、`git log --oneline main..branch`、
    `git diff --name-status main...branch`，再加原始 issue（`gh issue view --json title,body`）。**不放 diff 正文。**
  - `review-prompt.md` 改写：去掉所有 `` !`…` `` 展开，换成占位符；新增「读 diff 的方法」一节，
    要求评审工蜂按文件（`git diff main...branch -- <路径>`，大文件 `| sed -n` 分段）或按提交
    （`git show --stat <sha>` / `git show <sha> -- <路径>`）读 diff，覆盖清单里每个文件，并在答复里列出已看文件。
  - 渲染结果以 inline `prompt` 交给 `sandbox.run`。sandcastle 不展开 inline prompt，所以提交标题或
    issue 正文里的 `` !`…` `` 和 `{{…}}` 都按原文保留，不会被执行或二次替换（`renderTemplate` 只替换一趟，有测试锁住）。
  - 体积闸：`REVIEW_PROMPT_MAX_TOKENS = 60_000`（估算方法：ASCII 4 字符计 1 token，非 ASCII 1 字符计 1 token，偏保守）。
    超限抛 `ReviewPromptTooLargeError`（`exitCode = 4`），**不截断**。截断会让评审在没有提示的情况下漏看内容。
  - 失败通道沿用仓内已有机制：
    1. 写一条结构化记录到 `workers.jsonl`：`{phase:"reviewer", status:"failed", reason:"prompt_too_large", est_tokens, limit_tokens, chars, notified}`。
       watcher.sh 会按 (ticket, phase) 取最新事件写进 STATUS.md，所以 STATUS.md 会显示 `[reviewer] failed`。
    2. 在原票发评论（`gh issue comment --body-file -`，复用 main.mts 的 `GH_CLEAN_ENV`）。评论写明分支、估算值与上限、
       workers.jsonl 记录位置，以及补派命令 `SANDCASTLE_REVIEW_ONLY=<branch>:<issue> npx tsx .sandcastle/main.mts`。
       通知本身失败时不吞掉原错误，记录里写 `notified:false`。
    3. 进程非零退出：主循环设 `process.exitCode = 4` 后停止；REVIEW_ONLY 模式下 `process.exit(4)`。
  - 评审运行期的其它失败（agent 报错、超时等）也补写 `reason:"run_error"` 记录并发票面评论，然后原样抛出。
    评论里**不贴原始错误串**，防止路径或命令泄到 GitHub；原始错误只写在本机 workers.jsonl 里，截前 300 字符。
- 认领策略：超限时**保留认领**，不回滚。回滚会让 claim-loop 重新拾取这张票、重跑实装，结果还是同一个超大分支。
  这种情况交给人来定：拆分支，或调大上限后用 REVIEW_ONLY 补派。claim-loop 现有判据本来就不在
  `implementer/started` 之后回滚，本次没有改 claim-loop。
- `main-swarm.mts` 的评审路径接同一套逻辑：宿主侧渲染、体积闸、失败记录与评论、`exitCode = 4`。

### 测试输出

`.sandcastle/review-context.test.mts`（夹具：临时 git 仓，分支 3 个提交，每个提交新增约 40 万字符的文件，
diff 正文带标记串；提交标题和 issue 正文里故意放 `` !`…` `` 与 `{{BRANCH}}`）：

```console
$ npx tsx --test --test-reporter=spec .sandcastle/review-context.test.mts .sandcastle/cargo-guard.test.mts
[FAIL-loud #1588] 评审提示估算 ~1059 tokens（2496 字符）超上限 500，拒绝启动评审工蜂
[#1588] 评审失败通知（gh issue comment）失败：Error: gh 不可达
[FAIL-loud #1588] 评审提示估算 ~1056 tokens（2487 字符）超上限 10，拒绝启动评审工蜂
✔ 超大 diff 夹具：提示只含摘要、不含 diff 正文，体积在上限内
✔ 模板回归锁：review-prompt.md 不再用 `!`git diff ...`` 把整段 diff 展开进提示
✔ 超限：显式失败（专用非零退出码）+ 结构化失败记录 + 通知被调用
✔ 超限且通知失败：仍抛原错，记录里 notified=false
✔ 未超限：不通知、不写失败记录
✔ 运行期失败：落 run_error 记录并通知，票面评论不贴原始错误串
✔ renderTemplate：缺值即报错；单趟替换不二次展开
✔ estimateTokens：ASCII 4 字符一 token，中文按字计（保守）
（cargo-guard 6 例见 ④）
ℹ tests 14  ℹ pass 14  ℹ fail 0  ℹ skipped 0
```

上面三行 `[FAIL-loud …]` / `[#1588] …` 是超限用例按设计打出的 stderr，不是测试失败。
「超大 diff 夹具」一例先断言夹具本身的整段 diff 估算超过上限的 4 倍，确保测的确实是 #1406 那种形态。

---

## ④ 实装工蜂在 macOS 本机跑 cargo check / test

### 现象

#1406 实装工蜂（`main.mts` 的 noSandbox 路径，直接在 macOS 宿主上跑 Claude Code）在本机 worktree 里
执行了 11 次 cargo，违反「本机禁 cargo」约定：

```console
$ grep -n "cargo \(check\|test\|build\|clippy\)" .../.sandcastle/logs/sandcastle-issue-1406-implementer.log | cut -c1-200
86:Bash(cd /Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1406/rust; cargo check --locked --features s_session --lib --bins --tests 2>&1 | ...
87:Bash(cd .../sandcastle-issue-1406/rust; cargo check --locked --features s_session --lib --bins --tests 2>&1 | grep -E '^error|Finished|could not compile' | head; cargo check --locked --
89:Bash(cd .../sandcastle-issue-1406/rust; cargo test --locked --features s_session --lib -- economic_session session_transport session_protocol > /tmp/1406-lib-test.log 2>&1; echo EXIT $?
95:Bash(cd .../sandcastle-issue-1406/rust; ... cargo test --locked --features s_session --lib -- per
96:Bash(cd .../sandcastle-issue-1406/rust; cargo test --locked --features s_session --lib -- queued_unix_job_keeps_ingress_response_deadline_in_actor ...
97:Bash(cd .../sandcastle-issue-1406/rust; cargo test --locked --features s_session --bin s_structure_session -- q_content q_snapshot q_watch q_resources ...
100:Bash(cd .../sandcastle-issue-1406/rust; git checkout HEAD -- ... && cargo test --locked --features s_session --bin s_st...
$ grep -c "cargo " .../sandcastle-issue-1406-implementer.log
11
```

### 根因

1. 提示层：`implement-prompt.md` 的 Verify 步、`review-prompt.md` 的评审流程、`merge-prompt.md` 第 3 步和
   `CODING_STANDARDS.md` 的 Rust 闸都**要求**跑 `cargo check`，但都没有写「本机禁」，也没有给出 nc-remote 这条路。
   这些提示写于 docker 沙盒时代，那时容器里跑 cargo 是对的。
2. 环境层：#1547 起 `main.mts` 改用 `noSandbox()`，工蜂直接继承宿主 `process.env`，
   `/opt/homebrew/bin/cargo` 就在 PATH 上，没有任何拦截。

### 处置：修

- 提示层：`implement-prompt.md`、`review-prompt.md`、`merge-prompt.md`、`CODING_STANDARDS.md` 都写明本机禁
  cargo / rustc，并给出两条验证路：
  - 先 commit，再执行
    `NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=sandcastle-issue-<N> ~/.local/bin/nc-remote sandcastle/issue-<N> rust cargo check --locked`。
    其中 `NC_REPO` 必须给：sandcastle 宿主是独立 clone，nc-remote 默认指向主仓。`NC_TARGET` 是每个工蜂私有的 target 目录。
  - 或者把分支交给编排层推送、走 CI。
  - 远端不可达时，在答复里写明「Rust 未验证」，不得绕过垫片。
  - `merge-prompt.md` 的 ref 用分支名，不用 `HEAD`。实测 `git bundle create x <sha> HEAD` 的 bundle 头只有 `HEAD`，
    而 nc-remote 远端只取 `+refs/*`，没见过的提交会取不到。
- 环境层：新增 `.sandcastle/cargo-guard.mts`。
  - `installCargoGuard()` 每次运行在临时目录生成拦截脚本，覆盖 `cargo rustc rustup rustdoc clippy-driver cargo-clippy cargo-fmt cargo-miri cargo-nextest cargo-llvm-cov`。
    脚本向 stderr 打印 nc-remote / CI 两条指引，以 **exit 87** 退出，stdout 为空。
  - `cargoGuardEnv()` 生成三项环境变量：
    - `PATH`：把垫片目录前置；
    - `CLAUDE_ENV_FILE`：指向 `export PATH='<垫片>':"$PATH"`。Claude Code 每次执行 Bash 前都会 source 这个文件，
      防止 login shell 或 path_helper 把 `/opt/homebrew/bin` 重新排到前面；
    - `NC_LOCAL_CARGO_GUARD=1`。
  - `main.mts` 在 `noSandbox({ env })` 上合入这三项，实装、评审两段都生效。
  - nc-remote 不受影响：它的本地段只做 git bundle、scp、ssh，cargo 只出现在 ssh heredoc 里，在远端 WSL 执行
    （远端会 `source ~/.cargo/env`）。
- 已知边界（如实登记，见「遗留」）：垫片只拦按名字的调用。用绝对路径（`/opt/homebrew/bin/cargo`、`~/.cargo/bin/cargo`）
  仍能绕过，目前只靠提示约束。docker 蜂群 `main-swarm.mts` 没有挂垫片：容器 PATH 不同，容器里也没有 nc-remote。

### 测试与演练输出

单测（`.sandcastle/cargo-guard.test.mts`，env 构造与 main.mts 相同：`{...process.env, ...cargoGuardEnv(guard)}`）：

```console
✔ 工蜂环境下 `cargo --version` 被拦截：非零退出 + 指引指向 nc-remote
✔ 所有垫片工具（cargo/rustc/rustup/cargo-* …）都拦截，且可执行
✔ login shell 重排 PATH 后（垫片被挤到后面），source CLAUDE_ENV_FILE 仍把垫片顶回最前
✔ 垫片不影响非 Rust 工具（git / node 照常）
✔ localSegment：只剥 heredoc 与注释
✔ nc-remote 本地段不调用 cargo / rustc / rustup（垫片不影响远端执行）
```

最后一例在没有 `~/.local/bin/nc-remote` 的机器上（如 CI runner）会 skip 并给出说明。本机实跑没有 skip。

**演练 A：真实 Claude Code 的 Bash 工具。** 脚本 `/tmp/nc-guard-drill.mts`：env 为宿主 env 去掉 `CLAUDE*`/`ANTHROPIC*` 后，
合入 `cliProxyEnv()` 与 `cargoGuardEnv`，与 main.mts 同款，只在内存里传，不打印。
然后用 `claude --print --permission-mode bypassPermissions --model claude-opus-5-5`，让模型在 Bash 工具里执行
`command -v cargo; cargo --version; echo CARGO_EXIT=$?`。

```console
$ npx tsx /tmp/nc-guard-drill.mts full          # PATH 前置 + CLAUDE_ENV_FILE（生产形态）
variant=full guardBin=/var/folders/.../T/nc-cargo-guard-eP6KXL/bin claude_exit=0
/var/folders/.../T/nc-cargo-guard-eP6KXL/bin/cargo
[nc-local-cargo-guard #1588] 本机（macOS 宿主）禁止运行 cargo：本仓约定本机不跑 cargo build/test/check 等重型验证。
Rust 验证二选一：
  1) 远端（windows-5090 WSL）：先 git commit，再
       NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=<工蜂唯一名，如 sandcastle-issue-N> \
         ~/.local/bin/nc-remote <已提交的 ref，如 sandcastle/issue-N> rust cargo check --locked
     日志拷回 /tmp/nc-remote/；ref 必须是已提交的提交。
  2) 推分支开 PR 走 CI（工蜂不 push 时交编排层）。
不要用绝对路径绕过本垫片。
CARGO_EXIT=87

$ npx tsx /tmp/nc-guard-drill.mts envfile-only  # PATH 不动，只给 CLAUDE_ENV_FILE（单独验证该机制）
variant=envfile-only guardBin=/var/folders/.../T/nc-cargo-guard-P7q6ob/bin claude_exit=0
/var/folders/.../T/nc-cargo-guard-P7q6ob/bin/cargo
[nc-local-cargo-guard #1588] 本机（macOS 宿主）禁止运行 cargo：……（同上）
CARGO_EXIT=87
```

结论：两种形态下，Claude Code Bash 工具里的 `cargo` 都解析到垫片，并以 87 被拦下。其中只给 `CLAUDE_ENV_FILE` 的
那组单独证明了「每次 Bash 前重新前置」这一层确实生效。

**演练 B：nc-remote 不受影响。** 垫片前置到 PATH 后，本机 `cargo` 被拦，同一个 shell 里 nc-remote 仍能在远端跑 cargo：

```console
$ export PATH="<guard bin>:$PATH"; command -v cargo; cargo --version; echo "local cargo exit=$?"
/var/folders/.../T/nc-cargo-guard-1B7avv/bin/cargo
[nc-local-cargo-guard #1588] 本机（macOS 宿主）禁止运行 cargo：……
local cargo exit=87
$ NC_REPO="$PWD" NC_TARGET=sandcastle-guard-drill ~/.local/bin/nc-remote eng-debt-1588/sandcastle rust cargo --version
ref=eng-debt-1588/sandcastle sha=6021c6b668623a7bee53b0d435d1078f69c5c0ac sub=rust target=/home/hanju/nc-remote/sandcastle-guard-drill cmd=cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)
EXIT=0 SECONDS=0
log: /tmp/nc-remote/20261020-232305-6021c6b668.log
```

---

## 全量回归与 CI

- 本机：`npm run typecheck` 通过；`npx tsx --test .sandcastle/*.test.*` 结果为 `tests 167 / pass 167 / fail 0 / skipped 0`
  （原有 6 个测试文件与新增 2 个）。
- CI：`.github/workflows/ci.yml` 末尾新增 `sandcastle-ts` job（`npm ci --ignore-scripts`、`npm run typecheck`、
  `npx tsx --test .sandcastle/*.test.ts .sandcastle/*.test.mts`）。此前 `.sandcastle` 的 TS 测试在 CI 上没有覆盖。
  CI run 链接与结论见 PR 描述。
- 本分支没有改 Rust 源码，所以没有用 nc-remote 跑 `cargo check`。演练 B 的 `cargo --version` 只用来验证远端通路。

## 遗留与建议转票（已转 #1618）

下列 5 项与非作者评审的 N5、N6 已汇总转到 **#1618**（先查重，无重复票）。

1. **GitHub Actions 评审链路有同类缺陷**：`.sandcastle/agent-workflows/shared/review-context.ts:158` 把
   `git diff main...HEAD` 整段塞进 agent-review / implement-pr 的提示，没有体积闸。
   这条链路由 `agent-*.yml` 驱动，与本分片的 main.mts / main-swarm.mts 执行面不同，所以本分片没有改。
   建议转票，直接复用 `review-context.mts` 的摘要渲染与闸。
2. **绝对路径绕过垫片**：`/opt/homebrew/bin/cargo`、`~/.cargo/bin/cargo` 仍可直接调用。彻底封堵需要 Claude Code
   的 PreToolUse hook（按命令文本拦截）或宿主侧权限隔离。建议转票评估；目前靠提示约束和本垫片挡住常见路径。
3. **docker 蜂群（main-swarm.mts）没有挂垫片**，只靠提示约束。该路径目前基本休眠；恢复启用时再在镜像层处理。
4. **实装阶段失败同样静默**：`main.mts` 实装工蜂抛错时只写 `sandbox/closed`，没有 `implementer/failed`；
   claim-loop 在 `implementer/started` 之后也不回滚。这与 ③ 同类，但不在本分片题面内，建议转票。
5. **health-monitor 不把 `reviewer/failed` 当告警源**：最新事件仍是终态 `sandbox/closed`。本分片的通知走票面评论加
   STATUS.md 的 `[reviewer] failed` 行；如果要让 health-monitor 也告警，需要另改，建议在第 4 条一并处理。

## 希望评审重点看的地方（非作者独审由主控安排）

- `review-context.mts`：上限取 60k 估算 token 是否合适；超限选择「失败而不截断」并保留认领、不回滚，是否符合编排预期；
  票面评论只写元数据、不贴原始错误，这个取舍是否合适。
- `main.mts`：`runWithResume` 的 `prompt` 联合类型展开后，inline prompt 加 `resumeSession` 多轮续跑是否仍然正确；
  主循环在超限时 `break` 后由 `finally` 收尾，REVIEW_ONLY 模式下先 `close` 再 `exit(4)`，两处的顺序。
- `cargo-guard.mts`：`CLAUDE_ENV_FILE` 会覆盖调用方可能已经设置的同名变量（宿主目前没有设置）；垫片清单是否需要补充工具。
- `review-prompt.md`：「按需读 diff」的指令能否保证评审覆盖所有文件（提示要求在答复里列出已看文件）。
- `ci.yml` 新增的 job 放在文件末尾。其它在途分支如果也改 ci.yml，合并时可能有文本冲突，需要主控协调。

---

## 第二轮：非作者评审 N1–N4 处置（评审记录 `reviews/1615-claude-fable-5.md`，PASS-WITH-NITS）

### N1 渲染期非超限失败仍静默 → 修

- 现象：`runReviewer` 只对 `runWithResume` 的异常调 `recordReviewerRunFailure`。`prepareReviewPrompt` 阶段的非超限异常（`gh issue view` 失败、`git diff` 失败等）会原样抛出，`workers.jsonl` 只剩 `sandbox/closed`。
- 处置：`review-context.mts` 新增 `runGuardedReview(opts, channels, run)`，把渲染和运行包进同一个 try/catch：
  - 超限：`prepareReviewPrompt` 已经记了 `prompt_too_large`，这里只原样上抛，不重复记；
  - 渲染期其它异常：记 `reason=prepare_error`；
  - `run` 回调抛错：记 `reason=run_error`。
  
  每种失败都写 `phase=reviewer status=failed`，原始错误串截 300 字只进 jsonl，票面评论不贴原文，然后原样上抛。`main.mts` 的 `runReviewer` 和 `main-swarm.mts` 的评审段都改用它。`reviewerRunFailureComment` / `recordReviewerRunFailure` 增加 `reason` 参数，默认值仍是 `run_error`。
- 测试（`review-context.test.mts`，共 7 例）：
  - mock git 抛错、mock gh（`fetchIssue`）抛错、基准解析抛错，三种情况都落 `prepare_error`、票面评论不含 `/secret/host/path`、工蜂不启动；
  - gh 通知也失败时 `notified=false`，抛渲染期原错；
  - 运行期失败只记一条 `run_error`；
  - 超限只记一条 `prompt_too_large`；
  - 成功时不写记录。

### N2 `{{ISSUE}}` 在 ``` 围栏内 → 修

- 模板改成 `<issue>\n{{ISSUE}}\n</issue>`。正文里字面的 `</issue` 改写成 `<\/issue`（不区分大小写），防止提前闭合。
- 顺带修复：`gh issue view --json title,body` 输出单行 JSON，正文换行是 `\n` 转义，旧提示给评审的是一整行转义串。新增 `formatIssueJson`，还原成「标题：… + 空行 + 正文」后再进 `<issue>` 块。只有这样，正文里的 ``` 围栏才是真实的多行结构，N2 的风险才实际存在，也才需要包裹。
- 测试：
  - issue 正文含 ```bash 围栏、未闭合围栏、字面 `</issue>` / `</ISSUE >`、`{{BRANCH}}` 时，`<issue>` 只开闭各一次，块外 ``` 行数为偶数，`# 读 diff 的方法` 在 `</issue>` 之后，文件清单围栏完整；
  - 模板锁断言 `<issue>` 包裹，`{{ISSUE}}` 前不是 ```；
  - `formatIssueJson` 单测。

### N3 宿主 main 过旧会混入别线提交 → 修（改用 origin/main 提交作 diff 基准）

- 方案选择：渲染前先 `git fetch origin +refs/heads/main:refs/remotes/origin/main`，用 **origin/main 的提交 SHA** 作基准（模板新增 `{{BASE}}`），不 fast-forward 宿主 main。理由：
  1. 三点 diff 取 merge-base。分支无论分自旧 main 还是新 main，与刚 fetch 的 origin/main 求出的 merge-base 都落在真实分叉点，摘要只含本分支提交。fast-forward 也能达到同样效果，但要求宿主 worktree 干净，而且会改动别的进程正在用的检出，这本是 claim-loop `refreshHostMain` 的职责。
  2. 钉死 SHA，评审整个运行期间基准都不随 ref 移动。评审工蜂的工作区和宿主共享 refs，它看到的本地 `main` 仍是旧的，所以提示里的读 diff 命令一律用 `{{BASE}}`，模板第 5–6 行明令不要用本地 main。
  3. 宿主 main 含 origin/main 没有的提交（本地未推或分叉）时，无法判断哪边是正确基准，直接 FAIL-loud，记为 `prepare_error`，不猜。
  4. fetch 失败按 2s/4s/8s 指数退避，最多 4 次，之后 FAIL-loud（记 `prepare_error`）。宿主 main 落后时只打一行日志，照常用 origin/main。
- `assertHostBranchIsMain` 的注释和报错文案已更新：评审不再依赖内置 TARGET_BRANCH，保留这个校验是因为 claim-loop 刷新宿主 main、内置 TARGET_BRANCH、`workers.jsonl` 相对路径都以宿主 cwd 为准。`REVIEW_TARGET_BRANCH` 的旧注释（「TARGET_BRANCH 同源」）已删。`main-swarm.mts` 同样接入了 `resolveReviewBase`；4 路并发 fetch 抢 ref 锁时，靠同一套重试处理。
- 测试（临时裸仓 origin + host 克隆 + other 克隆推别线提交，共 6 例）：
  - 复现：宿主 main 落后时，旧做法 `main...feature` 的摘要混入别线提交 B 和 other.txt；
  - `resolveReviewBase` 返回 B 的 SHA，摘要只含本票提交，提示里读 diff 命令为 `git diff <B>...feature`；宿主本地 main 仍停在 A，worktree 干净；
  - 分支分自旧 main 时，以新基准同样只含本票提交；
  - main 与 origin/main 一致时不打告警；
  - 宿主 main 领先时 FAIL-loud；
  - 远端不可达时退避序列为 `[10, 20]`（注入的 sleep），之后 FAIL-loud，经 `runGuardedReview` 落 `prepare_error`。
- #1406 演练（只读渲染，issue 正文用真实 `gh issue view 1406`；本工位共享仓的本地 main 落后 origin/main 400 个提交）：

  ```
  $ cd /tmp && tsx /tmp/nc-1406-render-r2.mts
  local main=3315ed1d59
  stale main base: ReviewPromptTooLargeError est_tokens=72243 chars=270493 exit=4
  [#1588] 宿主 main 落后 origin/main 400 个提交；评审基准改用 origin/main df6e680930（不改宿主检出）
  origin/main base=df6e680930: est_tokens=8184 chars=25747 limit=60000 commits=37 has_diff_body=false
  ```

### N4 垫片临时目录不清理 → 修（复用固定路径）

- 方案选择：垫片目录固定为 `$TMPDIR/nc-cargo-guard`，每次运行原地重写，不再每次 mkdtemp。理由：
  - 选「退出时删除」有风险：main.mts 异常退出时，noSandbox 工蜂子进程可能还活着，垫片目录一删，它的 `cargo` 就落回 `/opt/homebrew/bin`，拦截失效。固定路径永不删除，没有这个窗口。
  - 垫片内容只取决于路径本身，多个 main.mts 并发写出的字节相同。每个文件先写 `<名>.<pid>.tmp`，chmod 后 rename 覆盖，运行中的工蜂不会读到写了一半的垫片。
  - 目录是软链，或属主不是当前用户时，直接 FAIL-loud，防止 `/tmp` 共享时被人预占或劫持。
- 测试（`cargo-guard.test.mts`，共 5 例）：
  - 缺省路径固定，重复安装不在 `$TMPDIR` 新建目录；
  - 同一根目录反复安装原地覆盖，没有 `.tmp` 残留；
  - 垫片被改坏后，再次安装可恢复并继续拦截；
  - 软链预占时 FAIL-loud；
  - 3 个子进程各重装 40 次的同时，主进程连调 30 次 `cargo --version`，每次都返回 87。原有 4 个用例改为走缺省固定路径，跑测试本身也不再累积目录。
- 第一轮遗留：本机 `$TMPDIR` 下还有 20 个 `nc-cargo-guard-XXXXXX` 目录，是第一轮本地测试和演练留下的，生产 main.mts 没跑过这一版。新代码不再生成这类目录。本分片没有删除它们，避免误删别的进程可能在用的目录；需要时可人工 `rm -rf "$TMPDIR"/nc-cargo-guard-??????`。

### 第二轮回归

- 本机 `npm run typecheck` 通过；`npx tsx --test .sandcastle/*.test.ts .sandcastle/*.test.mts` 结果为 `tests 187 / pass 187 / fail 0 / skipped 0`（第一轮 167 例，新增 20 例，没有删减或跳过）。
- CI run 链接与结论见 PR #1615 描述。

### N5 / N6 / 遗留 1–5

已汇总转 **#1618**（标签 `debt`；查重关键词与结果写在票面）。
