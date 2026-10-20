# PR #1615 尾部补审（#1588 sandcastle 分片）：5d6f30c6f8..8d15dffa9c

- 评审者：Claude Fable 5（claude-fable-5-1），非作者，只读；未改代码、未评论 PR、未合入。
- 上一次完整评审：`reviews/1615-claude-fable-5.md`（覆盖到 `5d6f30c6f8`，结论 PASS-WITH-NITS，N1–N6）。
- 本次范围：`git diff 5d6f30c6f8 8d15dffa9c`，仅 1 个提交 `8d15dffa9c`，9 个文件（+700 / −86）：
  `.sandcastle/{README.md, cargo-guard.mts, cargo-guard.test.mts, main-swarm.mts, main.mts, review-context.mts, review-context.test.mts, review-prompt.md}` 与
  `.chanlun/review-results/eng-debt-20261020/sandcastle/REPORT.md`。
- PR #1615 当前 head = `8d15dffa9c`（gh 核对），state OPEN。

## 结论：PASS

四条 nits（N1–N4）均按评审建议或更稳妥的等价方案处理到位，N5/N6 已转票 #1618；未发现新引入的缺陷，未越界改动并行分支文件；抽查的证据文字（第二轮回归 187/187）本机原样复现。下列 T1–T4 为 INFO 级观察，不阻塞。

## 逐条核实

### N1（渲染期非超限异常静默）→ 已修 ✔

- `review-context.mts:321-335` 新增 `runGuardedReview(opts, channels, run)`：`prepareReviewPrompt` 与 `run` 回调包在同一 try/catch；`reason` 先置 `prepare_error`，渲染成功后改 `run_error`；`ReviewPromptTooLargeError` 由 `prepareReviewPrompt` 已记 `prompt_too_large`，此处只上抛不重复记。
- `main.mts:258-281` 与 `main-swarm.mts:130-151` 都改用它，`recordReviewerRunFailure` / `prepareReviewPrompt` 不再被两处直接调用（import 已移除，typecheck 通过）。`main.mts` 的 `reviewer/started` 事件移进 `run` 回调，渲染失败时不会留下孤立的 `started`，只落一条 `failed reason=prepare_error`。
- 票面评论按 reason 分两种标题（`REVIEWER_FAILURE_HEADLINE`），原始错误串只进 jsonl（截 300 字），与上一轮「不贴命令行/路径」口径一致。
- 测试：渲染期 git / gh / 基准解析三种抛错各落 `prepare_error`（参数化 3 例）、通知失败 `notified=false`、运行期 `run_error` 只记一条、超限只记一条、成功不写记录，共 7 例，与 REPORT 描述一致。

### N2（`{{ISSUE}}` 在 ``` 围栏内）→ 已修 ✔

- `review-prompt.md:31-35` 改为 `<issue>…</issue>`；`neutralizeIssueText` 把正文里的 `</issue` 改写成 `<\/issue`（不区分大小写），`renderTemplate` 单趟替换保证正文里的 `{{X}}` 不被二次展开。
- 顺带新增 `formatIssueJson`：`gh issue view --json title,body` 的单行 JSON 还原为「标题：… + 空行 + 正文」后再进块。这是一处 nits 之外的行为变化（评审工蜂此前拿到的是转义后的单行 JSON），但与 N2 直接相关、REPORT 已如实登记为「顺带修复」，JSON 解析失败会经 `runGuardedReview` 落 `prepare_error`（FAIL-loud 而非静默）。接受。
- 测试：正文含 ```bash 围栏 / 未闭合围栏 / 字面 `</issue>` `</ISSUE >` / `{{BRANCH}}` 时 `<issue>` 只开闭各一次、块外围栏行数为偶数、`# 读 diff 的方法` 在 `</issue>` 之后；`formatIssueJson` 单测；模板锁断言。

### N3（宿主 main 过旧混入别线提交 + 注释过时）→ 已修 ✔（方案与建议不同，更稳）

- 上轮建议「`main` 落后 `origin/main` 即 FAIL-loud」；作者改为 `resolveReviewBase`（`review-context.mts:116-155`）：渲染前 `git fetch origin +refs/heads/main:refs/remotes/origin/main`，以 **origin/main 的提交 SHA** 作 diff 基准（模板新增 `{{BASE}}`，三处 `git diff/log` 命令及「读 diff 的方法」全部改用 `{{BASE}}`，并在开头明令不要用本地 `main`）；宿主 main **落后**只打日志、不改检出；宿主 main **领先/分叉**（含 origin/main 没有的提交）才 FAIL-loud。
  - 该偏离合理：落后场景本就是「基准该取新 main」，直接用 SHA 比强制作者先刷新宿主更少副作用，且钉死 SHA 避免评审期间 ref 漂移；领先场景确实无法判定基准，FAIL-loud 正确。三点理由在源码注释与 REPORT 均有记录。
  - fetch 失败 2s/4s/8s 指数退避最多 4 次，之后抛错 → `prepare_error`。
- `assertHostBranchIsMain` 注释与报错文案已更新（`main.mts:57-67, 78-83`），理由改为「claim-loop refreshHostMain / 内置 TARGET_BRANCH / workers.jsonl 相对路径都以宿主 cwd 为准」；`REVIEW_TARGET_BRANCH` 旧注释「TARGET_BRANCH 同源」已删。
- `main-swarm.mts` 同样接入。
- 测试：真实裸仓 origin + host 克隆 + other 克隆推别线提交的夹具，6 例（复现旧做法混入 / 新基准只含本票提交且不改宿主检出 / 分自旧 main / 一致不告警 / 领先 FAIL-loud / fetch 持续失败退避 `[10,20]` 后经 `runGuardedReview` 落 `prepare_error`）。

### N4（垫片临时目录累积）→ 已修 ✔（选固定路径而非退出时删除）

- `cargo-guard.mts:56-98`：固定 `$TMPDIR/nc-cargo-guard`，`mkdirSync(..., mode 0o700)` 后 `assertOwnDirectory`（lstat：非目录 / 软链 / 属主不符即 FAIL-loud），每个文件 `<名>.<pid>.tmp` 写全 + chmod 后 `rename` 覆盖。
- 不选「退出时删除」的理由成立：noSandbox 工蜂子进程可能比 main.mts 活得久，删目录即拦截失效。`guardScript(tool)` 内容与路径/进程无关、`claude-env.sh` 只依赖固定 binDir，故并发重写字节相同。
- 测试 5 例：缺省路径固定且不新建目录、反复安装无 `.tmp` 残留、改坏后恢复并仍拦截（exit 87）、软链预占 FAIL-loud、3 子进程 ×40 次并发重装期间主进程 30 次 `cargo --version` 全部 87。原 4 例改走固定路径。

### N5 / N6 / 遗留 1–5 → 已转票 ✔

- `gh issue view 1618`：OPEN，label `debt`，标题「[task] #1588 转票：sandcastle 失败可见性与本机 cargo 垫片余项（GH Actions 评审整段 diff、绝对路径绕过、蜂群、implementer/failed、health-monitor）」，与 REPORT「已转 #1618」相符。

## 新发现（本尾部引入；均 INFO，不阻塞）

| # | 严重度 | 位置 | 说明 |
|---|---|---|---|
| T1 | INFO | `review-context.mts:111-113, 134-136` | `sleepSync` 用 `Atomics.wait` 同步阻塞事件循环，fetch 连续失败时最多阻塞 2+4+8=14s。`main-swarm.mts` 单进程并发 4 票，此时其它票的 Promise 链也停。与现有宿主侧 `execFileSync` git/gh 调用同属阻塞模式，非回归；蜂群休眠中。 |
| T2 | INFO | `review-context.mts:344-347` | `formatIssueJson` 属 nits 之外的行为变化（见 N2），已登记；#1406 估算由 8062 → 8184 tokens，远低于 60k。 |
| T3 | INFO | `cargo-guard.mts:67-72` | 进程在 `writeFileSync(tmp)` 与 `renameSync` 之间被杀时会留下 `<tool>.<pid>.tmp`，不在 PATH 查找名内、无害，下次安装不清理。 |
| T4 | INFO | `cargo-guard.test.mts:121-130` | 「缺省路径固定」用例会实际改写生产用的 `$TMPDIR/nc-cargo-guard`；按设计字节相同、原子替换，与生产并跑无影响。 |

## 越界检查

- 文件清单只含 `.sandcastle/` 8 个文件与本分片 REPORT.md；未触碰 `codex-child.ts`、`delivery-bundle.ts`、`run-wayfinder-engine.sh`、`wayfinder_engine.mts`、`ci.yml`、Rust 源。
- 无测试删除、无 skip（187/187/0 skipped）。

## 证据抽查（1 处）：REPORT「第二轮回归」

REPORT 声称：`npm run typecheck` 通过；`npx tsx --test .sandcastle/*.test.ts .sandcastle/*.test.mts` → `tests 187 / pass 187 / fail 0 / skipped 0`（167 + 20，无删减跳过）。
本机在 `8d15dffa9c` 复跑：typecheck 通过；`# tests 187 / # pass 187 / # fail 0 / # skipped 0`，完全一致。新增例数核对：N1 7 + N2 2 + N3 6 + N4 5 = 20 ✔。

## 复核过的命令

1. `git diff --stat 5d6f30c6f8 8d15dffa9c`；`git log --oneline 5d6f30c6f8..8d15dffa9c`（1 提交）。
2. `git diff 5d6f30c6f8 8d15dffa9c -- <各文件>` 逐文件读；读 `review-context.mts` 全文、`cargo-guard.mts:15-52`（`guardScript` 不含路径/进程信息）。
3. `npm run typecheck`；`npx tsx --test .sandcastle/*.test.ts .sandcastle/*.test.mts` → 187/187/0/0。
4. `gh pr view 1615 --json headRefOid,state` → `8d15dffa9c…` / OPEN。
5. `gh issue view 1618 --json number,title,labels,state` → OPEN / debt。
6. `gh run list --commit 8d15dffa9c…`：CI run 39480668191 success；DevSkim run 39480668163 failure，其日志中 `review-context|cargo-guard|review-prompt|main-swarm|main.mts|README.md|REPORT.md` 命中 0 条（与上一轮相同，与本 PR 文件无关）。
7. 未做：未起真实工蜂跑生产 `main.mts` 全链路；未在本机复跑 #1406 渲染演练（上一轮已独立复现同口径数值，本轮只核 REPORT 新数值 8184 的数量级合理性）。
