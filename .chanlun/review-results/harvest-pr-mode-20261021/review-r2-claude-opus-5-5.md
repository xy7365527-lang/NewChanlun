# #1660 / PR #1661 非作者独审第 2 轮（claude-opus-5-5，只读）

- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318）；作者为 harvest owner 代理（非 Claude），本审查非作者。
- 审查对象：第 1 轮 `284c748844`，第 2 轮 `5c0cd67c32`（含 r1 处置），第 3 轮尾部 `5c0cd67c32..963a5e569e`。
- 审查提示原文：`prompt-r2.md`。日期：2026-10-21。

---

结论：PASS

第 1 轮的 6 条 MEDIUM 全部修了，修法也对。10 条 LOW 里修了 6 条，未修的 4 条都只影响进度或可观测性，不影响安全。重新通读了整个 `harvest.sh`，仍然没有任何路径会关票、合入、推 main、force push、删分支或跑 cargo。本轮新发现都是 LOW，主要是测试还有几处覆盖缺口，都不卡合入。

## 第 1 轮意见处置

| # | 第 1 轮意见 | 处置 | 核对结果 |
|---|---|---|---|
| M1 | PR 标题不做关闭关键词检查 | 已修 | `harvest.sh:139-143` 在推送前检查 `pr_title`，`:175` 开 PR 前再查一次（多查一次，无害）。测试 112 用票标题 "Fix #1588 follow-up"，断言 FAIL 且没推送。如果删掉 `:140`，`:175` 会在推送之后才拦下，`! remote_has` 断言会变红，所以能抓住这种回退。 |
| M2 | `CLOSE_RE` 漏了 issues URL 写法 | 已修 | `:33` 加了 `https?://github[.]com/…/issues/[0-9]+` 分支。测试 `:21-28` 有 10 条正例、10 条反例，含 URL、`perf(#1629):`、`closed-form #3`。 |
| M3 | 推的是分支名不是固定 SHA，评审判据和推送之间有时间窗 | 已修 | `:88` 一开始就钉住 `sha`。祖先判定、`rev-list`、分叉判定和推送（`:148` 写成 `"$sha:refs/heads/$br"`）都用这个值。顺序也对：先钉 SHA，再查 `reviewer_done`，所以钉住的提交对应的 implementer started 记录一定已经能读到。 |
| M4 | 先推送、后查 PR 状态，会复活被删掉的分支 | 已修 | `pr list` 挪到了 `:104-110`，在 `ls-remote` 和推送之前。只有已关闭或已合入的 PR 时直接 SKIP。测试 109（MERGED PR、远端分支已删）断言分支没被推回去。 |
| M5 | `set -e` 中途退出会绕过失败台账 | 已修（大部分） | 开头的 fetch 失败（`:76`）、`ls-remote` 失败（`:113`）、远端分支 fetch 失败（`:118`）都会记一条 `note_fail`，进台账。还剩几处没有保护的赋值，见新发现 L5。 |
| M6 | 测试大多在复现实现本身 | 大部分已修 | 已经做到的：<br>• 假 gh 用真 jq 执行 `--jq`（106 的 blocker 里混了一个 CLOSED，断言计数是 2）；<br>• 有状态地记住已开的 PR，第二轮是真跑，断言不重复开 PR、不重复评论；<br>• 断言 PR 正文含 `Refs #101` 且不含关闭关键词；<br>• 宿主 main 先领先 origin 一个提交（`:84`），再断言 origin main 没变；<br>• 新增已合入 PR、远端领先（110）、远端领先且带关键词（111）、open PR 的远端 head 被改写（114）等用例。<br>仍然缺的：远端落后、可快进的情况；对 `--state all` 的约束；跨轮的禁止动作检查，见 L1–L3。 |
| L | 正则依赖 locale | 已修 | `has_close_kw` 固定用 `LC_ALL=C`（`:71`），测试的 `kw` 也一样。没有中文贴着关键词的用例。 |
| L | `${br##*-}` 会把 `issue-1660-2` 当成票 #2 | 已修 | 改成 `:82` 的 `${br#sandcastle/issue-}`，再要求全是数字。测试 `1660-2` 覆盖了。 |
| L | `refname:short` 遇到同名 tag 会出错；`sort -k3` 不起作用 | 已修 | 改用 `:78` 的 `%(refname)` 去掉前缀，再用普通 `sort`。 |
| L | 拿不到 `nodes` 时 blocker 回退成 0 | 已修 | `:94` 现在回退到 `totalCount`，和 `scripts/wayfinder_engine.mts:450-454` 的 `openBlockerCount` 口径一致。这条回退分支没有测试覆盖。 |
| L | 远端领先时，多出来的提交不查关键词 | 已修 | `:135` 对 `origin/main..$head_sha` 做检查。测试 111 覆盖了。 |
| L | 运行时不检查 `GH_TOKEN_ROTATION_REQUIRED` | 已修 | `:73-75` 检查两处路径，和 `main.mts:29`、`run-claim-loop.sh:11` 一致。命中时整轮不动。测试第 6 节断言命中时一次 gh 都不调。 |
| L | 登记评论失败一次就永远补不上 | **未修** | `:199-201` 仍然只打一行 WARN，不进台账。下一轮看到已有 open PR，只走"只推送"路径。可以接受：PR 本身已经开了，人能看到。 |
| L | 永久 SKIP 的情况不进台账 | **未修** | 包括票读取失败（`:95`）、PR 已关闭或已合入（`:108`）、python3 缺失。最需要注意的是 gh 认证整体失效、但还没有轮换标记的时候：所有分支都在 `:95` SKIP，结果 exit 0，静默不干活。可以接受（只影响进度），建议以后补。 |
| L | 关键词失败每 15 分钟弹一次通知 | 未修（按设计） | fail-loud 本来就是这么设计的，可以接受。 |
| L | `completed` 不代表评审通过；`no_commit` 会卡住分支 | 未改（仅说明） | PR 正文措辞仍是 "Reviewer worker: completed"，表述准确。 |

## 新发现

### HIGH
无。

### MEDIUM
无。

### LOW

- **L1：假 gh 不理会 `--state`，抓不住去掉 `--state all` 的回退**（`.sandcastle/harvest.test.sh:108-110`）
  - 真实的 `gh pr list` 默认只返回 open 的 PR。如果 `harvest.sh:104` 丢掉 `--state all`，109 这种已合入 PR 的分支会被重新推上去。
  - 假 gh 不管有没有这个参数都返回 fixture，所以测试 109 照样会绿。
  - 建议：假 gh 在没收到 `--state all` 时，过滤掉非 OPEN 的 PR。

- **L2：第 7 节的禁止动作检查只覆盖了第一轮**（`.sandcastle/harvest.test.sh:157`、`:167`、`:173`、`:179`）
  - `:167` 和 `:173` 先后清空了 `$FAKE_GH_LOG`，第二轮真跑和 dry run 的调用记录都丢了。
  - 结果 `gh.all.log` 里只有第一轮的记录，加上第 6 节那一轮（本来就是空的）。"never closes / merges / cargo" 这几条断言实际上没检查第二轮和 dry run。
  - 建议：清空前把日志追加到 `gh.all.log`。

- **L3：没有测"远端存在且落后于本地、可快进推送"**（`.sandcastle/harvest.test.sh`）
  - 102 的远端分支原本不存在。107 和 114 是分叉，110 和 111 是远端领先。
  - `harvest.sh:124` 的快进路径没被走到。如果它被改成误报分叉，结果是 fail-safe，但测试抓不住。

- **L4：open PR 的远端 head 被改写后，本地的新提交会被静默搁置**（`.sandcastle/harvest.sh:125-128`）
  - 本地和 open PR 的远端 head 分叉时，只打一行 SKIP，不进台账。测试 `:150` 明确断言了不进台账，所以这是有意设计。
  - 后果：如果之后新一轮评审在旧的本地提交上继续提交，这些提交永远推不上去，也没人知道。只影响进度。

- **L5：开 PR 前后还有几处赋值没有保护，失败时会被 `set -e` 直接退出**（`.sandcastle/harvest.sh:159-161`、`:189`）
  - 涉及 `shas=$(git rev-list …)`、`commit_lines=$(git log …)` 和两处 `mktemp`。
  - `:159-161` 失败发生在推送之后、开 PR 之前；`:189` 失败发生在开 PR 之后、登记评论之前。两种情况都会整轮退出，不写台账。
  - 实际很难触发：`head_sha` 对应的对象这时已经确认在本地。

- **L6：输出措辞有误导**（`.sandcastle/harvest.sh:154`）
  - `need_push=0` 或 dry run 时也会打印 `PUSHED`，dry run 下会先后出现 `DRY push` 和 `PUSHED` 两行。只是措辞问题。

- **L7：开新 PR 时可能带上评审工蜂没看过的远端提交**（`.sandcastle/harvest.sh:122-123`）
  - 远端领先本地、又没有任何 PR 时，会以远端 head 开新 PR，PR 里包含评审工蜂没评过的远端提交，正文却写着 "Reviewer worker: completed"。
  - 这些提交已经过了关键词检查。触发条件是有人手动往 `sandcastle/issue-*` 推过提交，很少见。

- **L8：测试不是完全隔离的**（`.sandcastle/harvest.sh:73`）
  - 轮换标记的第二个路径是写死的绝对路径。如果开发机上真的存在 `/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED`，本地跑 `harvest.test.sh` 会整体变红。
  - 后果是误报失败，不会误报通过；CI 上不受影响。

### 安全不变量复核（全文重读）
- **推送**：只有 `:148` 一处，写法是 `git push -q origin "$sha:refs/heads/$br"`，不带 `-f`，也不带 `+`。`$br` 只来自 `refs/heads/sandcastle/issue-<数字>`，不可能是 main。`$sha` 是 `rev-parse --verify` 解析出的提交。
- **gh 调用**：只有 `issue view`、`pr list`、`pr create`、`issue comment` 四种，都经过 `gh_`，在中性目录执行并带显式 `--repo`。
- **其他**：没有 `issue close`、`pr merge`、`branch -d/-D`、`cargo`。`rm -f` 只删临时文件。`:118` 的 fetch 没有目标 refspec，只会更新 FETCH_HEAD 和远程跟踪引用。
- **`open_pr` 为空时的四种情况**：
  - 没有任何 PR：正常往下走，开 PR；
  - 只有已关闭或已合入的 PR：在推送前 SKIP；
  - 远端分叉：FAIL，不推；
  - 远端领先：不推，以远端 head 开 PR。

  `open_pr` 不为空时，标题检查按设计跳过。
- **`head_sha` 与 `sha` 的分工**：推送只用 `sha`；关键词检查、PR 正文和登记里的提交列表用 `head_sha`。这样分是对的。
- **dry run**：只有 fetch 会改本地远程跟踪引用，不推送、不开 PR、不评论、不写台账。
- **bash 3.2**：没有用到 3.2 之后才有的语法。

## 看过 / 跑过 / 未核实

- **看过**：
  - 第 1 轮报告；
  - `git diff 284c748844..5c0cd67c32`（`harvest.sh`、`harvest.test.sh`、`README.md`）；
  - 完整的 `harvest.sh`（228 行）和 `harvest.test.sh`（187 行）；
  - `harvest.test.mts`、`launchd/com.newchanlun.sandcastle-harvest.plist`（launchd 用 `/bin/bash` 跑）；
  - `scripts/wayfinder_engine.mts:450-454`、`.sandcastle/main.mts:250-309`；
  - 全仓库里 `GH_TOKEN_ROTATION_REQUIRED` 的所有引用。
- **跑过**：
  - `bash .sandcastle/harvest.test.sh`：pass=53 fail=0；
  - `/bin/bash .sandcastle/harvest.test.sh`：pass=53 fail=0。
- **未核实**：
  - **`harvest.sh` 本身在 bash 3.2 下能不能跑**：测试脚本是用 PATH 上的 `bash` 去调 `harvest.sh` 的（`harvest.test.sh:126`），所以用 `/bin/bash` 跑测试只能证明测试脚本本身兼容 3.2。我想用 `/bin/bash -n`、`/bin/bash --version` 和"把 `/bin` 放到 PATH 最前面"来确认，这些命令都没有获批。查了 `/opt/homebrew/bin`、`/usr/local/bin`、`~/.local/bin`，都没有 bash，所以 PATH 上的 `bash` 很可能就是 `/bin/bash`，但没有实证；
  - sandcastle 写下 reviewer `completed` 记录时，评审沙箱里的提交是否已经同步回宿主分支（`main.mts:282`，取决于 sandcastle 库的行为，没读库源码）；
  - 宿主 gh 版本对 `blockedBy` 的实际返回结构；
  - 仓库是否开了"合并后删除 head 分支"；
  - GitHub 对紧挨中文字符的关闭关键词是否真的会关票；
  - CI 上的实际运行结果；
  - launchd 实际加载情况（按要求没有调用 gh，也没有安装）。
