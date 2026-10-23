# #1660 / PR #1661 非作者独审第 1 轮（claude-opus-5-5，只读）

- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318）；作者为 harvest owner 代理（非 Claude），本审查非作者。
- 审查对象：第 1 轮 `284c748844`，第 2 轮 `5c0cd67c32`（含 r1 处置），第 3 轮尾部 `5c0cd67c32..963a5e569e`。
- 审查提示原文：`prompt-r1.md`。日期：2026-10-21。

---

结论：PASS

`harvest.sh` 没有任何路径会关票、合入、推 main、force push、删分支或跑 cargo。票 #1660 的验收项都已实现。下面的 MEDIUM 问题不卡合入，但建议在装 launchd 之前补上：前两条（PR 标题检查、URL 写法）各改一行就能堵住；其余几条是关闭/合并 PR 的处理顺序、`set -e` 中途退出和测试覆盖不足。

## HIGH

无。

逐条看了所有会改动外部状态的地方：
- **推送**：只有 `.sandcastle/harvest.sh:114` 一处，写法是 `git push -q origin "$br:refs/heads/$br"`。不带 `-f`，也不带 `+`。`$br` 只来自 `refs/heads/sandcastle/issue-*`，不可能是 main。
- **gh 调用**：只有 `issue view`、`pr list`、`pr create`、`issue comment` 四种，都带显式 `--repo`，并在中性目录里执行。
- **删分支、关票、合入、cargo**：都没有。旧版的 `git branch -qD`、worktree 和 cherry-pick 已经全部删掉。

## MEDIUM

1. **PR 标题不做关闭关键词检查**（`.sandcastle/harvest.sh:158`，标题来自 `:87`）
   - 标题是 `Sandcastle #$n: $title`，`$title` 直接取票面标题。`:148` 只检查了正文文件。
   - 本仓库用 squash 合入（最近的提交都带 `(#1593)` 这类后缀），所以 PR 标题会变成 main 上的提交标题。
   - 例子：票标题是 "Fix #1588 follow-up" 时，人工合入那一刻 GitHub 会自动关掉 #1588。
   - 修法：把 `--title` 的内容也过一遍 `CLOSE_RE`，命中就 fail-loud。

2. **`CLOSE_RE` 漏了完整 URL 写法**（`.sandcastle/harvest.sh:33`）
   - GitHub 认 `Fixes https://github.com/<owner>/<repo>/issues/123` 这种写法；正则只覆盖了 `#n` 和 `owner/repo#n`。
   - 我实际测了：`Fixes https://github.com/o/r/issues/5` 没有命中。
   - 其余写法都命中了：大小写、`closes: #12`、`Fix:#9`、`FIXES owner/repo#3`、`Fixes: o/r#18`、`Resolved #4`、多个连写。
   - 下面这些没有误伤：`perf(#1629): x`、`chore(probe): #1630`、`fix(#1629): x`、`prefix #7`、`fixed-point #8`、`refs #15`、`closing #20`。
   - 建议在正则里加一个可选分支 `https?://github\.com/[^/[:space:]]+/[^/[:space:]]+/issues/[0-9]+`。

3. **评审判据之后才推送，中间有时间窗，推的是分支名不是固定 SHA**（`.sandcastle/harvest.sh:80`、`:92`、`:100`、`:114`）
   - `reviewer_done` 和关闭关键词检查（`msgs`）先跑；`:100` 才读 `local_sha`；`:114` 推的是 `$br`，按推送那一刻解析。
   - 中间隔着两次 gh 网络调用。如果这段时间里新一轮 implementer 开始在同名分支上提交，推上去的就是没评审、也没过关键词检查的提交。
   - 修法：在循环开头一次性 `rev-parse` 出 SHA。`rev-list`、`log`、分叉判定和推送都用这个 SHA，推送写成 `"$sha:refs/heads/$br"`。

4. **先推送、后查 PR 状态，会把已合并或已关闭 PR 被删掉的分支重新推上去**（`.sandcastle/harvest.sh:98-131`）
   - 按设计，squash 合入后票仍保持 open（正文写的是 `Refs`），分支也不是 origin/main 的祖先。
   - 如果仓库开了"合并后删除 head 分支"，远端分支消失，`remote_sha` 为空，于是 `need_push=1`，分支被推回远端，然后才在 `:128` 判定 SKIP。
   - 人工关闭 PR 并删分支的情况也一样。
   - 修法：把 `pr list` 移到推送前面；有 CLOSED 或 MERGED 的 PR 时，直接 SKIP，不推。
   - 仓库是否开了自动删分支，我没有核实（没调用 gh）。

5. **`set -euo pipefail` 下的中途退出会绕过失败台账**（`.sandcastle/harvest.sh:64`、`:99`）
   - `git fetch -q origin main` 失败，或 `remote_sha=$(git ls-remote … | cut -f1)` 在网络抖动时失败，脚本会立刻退出。
   - 这样剩下的分支都不处理，`harvest-failures.md` 不写，也没有通知；launchd 下只在 stderr 日志里留一行。
   - 另外，claim-loop 和 harvest 在同一个宿主克隆里并发 fetch，可能碰上锁冲突，触发同样的退出。
   - 修法：对 `ls-remote` 用 `|| { note_fail …; continue; }`；对开头的 fetch 显式失败，并写台账。

6. **测试大部分是在复现实现本身，抓不住几类回退**（`.sandcastle/harvest.test.sh`）
   - `:56-62`：假 gh 忽略 `--json`/`--jq`，直接返回预先拼好的 TSV。所以 `:83` 的 jq 表达式（`blockedBy`、`select(.state=="OPEN")`、字段名）完全没被执行。比如改成连已关闭的 blocker 一起数，测试照样全绿。
   - 没有覆盖 CLOSED/MERGED PR 的分支；也没有检查 PR 正文内容（含 `Refs #101`、不含关闭关键词）。正文文件在 `:159` 被删之前，假 gh 应该先拷一份。
   - 关键词只测了 `fix: #103` 一种写法，没有成组的正例和反例（比如 `perf(#1629):`）。
   - `:105` 的"origin main 没被改"比较的是 origin main 和宿主 main，但两者本来就相等。如果回退成推 HEAD 或推 main，结果是 no-op，测不出来。应该先让宿主 main 领先 origin 一个提交，再断言 origin 不变。
   - `:108-119` 的第二轮是 dry run，而且假 gh 没有状态，所以"跨轮不重复开 PR、不重复评论"这一点并没有真正测到（注释里也承认了）。
   - 以下几种情况也没有覆盖："已在 origin/main"、"没有新提交"、远端落后可快进、远端领先不覆盖。

## LOW

- `.sandcastle/harvest.sh:33`：正则依赖 locale。实测 `修复fix #22` 在 `LC_ALL=C` 下命中，在 UTF-8 下不命中（中文字符被当成 `[:alnum:]`）。plist 没设 `LANG`，所以 launchd 下跑的结果和交互式 shell 不一样。这种写法 GitHub 本身会不会关票，我没有核实。
- `.sandcastle/harvest.sh:73`：`${br##*-}` 取的是最后一个 `-` 后面的部分，所以 `sandcastle/issue-1660-2` 会被当成票 #2。建议改成 `${br#sandcastle/issue-}`，再要求全是数字。
- `.sandcastle/harvest.sh:72`：
  - 如果存在同名 tag，`%(refname:short)` 会变成 `heads/sandcastle/…`，推送目标就错了。建议用 `%(refname)` 再去掉前缀。
  - `sort -t- -k3 -n` 实际上不起作用（只有 2 个字段），等于按字典序排。这个问题旧版就有，只影响顺序。
- `.sandcastle/harvest.sh:83`：拿不到 `nodes` 时 blocker 数量算成 0。`scripts/wayfinder_engine.mts:450` 的 `openBlockerCount` 在这种情况下会回退到 `totalCount`，两边口径不一致。
- `.sandcastle/harvest.sh:172-174`：登记评论如果失败一次，就永远补不上了。下一轮看到已有 open PR，只会走"只推送"的路径。
- 永久 SKIP 的情况都不进台账：票读取一直失败、PR 已关闭需要人工决定，还有 python3 缺失时 `reviewer_done` 恒为假。
- 另一边，关闭关键词这种持续性失败每 15 分钟会弹一次通知、追加一次台账。
- `.sandcastle/harvest.sh:55-59`：`completed` 只代表"评审跑完没抛异常"（`.sandcastle/main.mts:282`；超过 `MAX_ITERATIONS` 没收到完成信号也会记 completed），不代表评审通过。PR 正文里写的 "Reviewer worker: completed" 措辞是准确的。
- 另外，新一轮 implementer 如果是 `no_commit` 就结束，上一轮已经评审过的分支会一直被卡住。这只影响进度，不影响安全。
- `.sandcastle/harvest.sh:104`：远端领先本地时不推送，PR 用的是远端 head，而远端多出来的提交没做关键词检查。
- `harvest.sh` 运行时不检查 `GH_TOKEN_ROTATION_REQUIRED`，只有安装脚本检查。`run-claim-loop.sh` 是运行时检查的。

## 实际看过 / 跑过 / 未核实

- **看过**：
  - `git diff 02b5b571eb...HEAD` 涉及的全部 7 个文件；
  - 旧版 `harvest.sh`；
  - `.sandcastle/main.mts:115-378`，确认 `workers.jsonl` 的写入语义：`ticket` 是数字，`implementer started` 在 `:348`，`reviewer completed` 在 `:282`，REVIEW_ONLY 路径不写 implementer 记录；
  - `scripts/wayfinder_engine.mts` 里 `blockedBy` 的结构和用法；
  - `ci.yml` 的 `sandcastle-ts` job（`npx tsx --test .sandcastle/*.test.mts` 会把 `harvest.test.mts` 带进 CI）；
  - 本 worktree 的 `.git/hooks/pre-push`（只有 git-lfs，不跑 cargo）。
- **跑过**：
  - `bash .sandcastle/harvest.test.sh`：pass=22 fail=0；
  - 用 `grep -Ei` 拿 `CLOSE_RE` 对 25 条样例做了正例/反例测试，另外测了 C 和 UTF-8 两种 locale。
- **没核实**：
  - bash 3.2 兼容性：想用 `/bin/bash` 加 `PATH=/usr/bin:/bin` 跑测试，命令没有获批，只能靠读代码判断（`<<<`、`$'\n'`、`[[ ]]` 在 3.2 里都支持）；
  - 宿主克隆的 hooks 和 `core.hooksPath`；
  - 宿主 gh 版本对 `issue view --json blockedBy` 的实际返回结构；
  - 仓库是否开了"合并后删除 head 分支"；
  - GitHub 对紧挨中文字符的关键词、对 `GH-n` 写法是否关票；
  - CI 上的实际运行结果；
  - launchd 实际加载情况（按要求没有安装）。
