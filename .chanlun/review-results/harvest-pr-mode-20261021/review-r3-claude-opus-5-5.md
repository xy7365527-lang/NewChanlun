# #1660 / PR #1661 非作者独审第 3 轮（claude-opus-5-5，只读）

- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318）；作者为 harvest owner 代理（非 Claude），本审查非作者。
- 审查对象：第 1 轮 `284c748844`，第 2 轮 `5c0cd67c32`（含 r1 处置），第 3 轮尾部 `5c0cd67c32..963a5e569e`。
- 审查提示原文：`prompt-r3.md`。日期：2026-10-21。

---

结论：PASS

这次追加的提交 `963a5e569e` 共改 2 个文件（+26/−12）。第 2 轮提的 L1、L2、L3、L8 都已修好，修法正确。L6 修了一半，剩下的只是输出措辞问题。没有引入 HIGH 或 MEDIUM 问题。

## 第 2 轮意见处置

| # | 处置 | 核对 |
|---|---|---|
| L1 假 gh 不理会 `--state` | 已修 | 现在没传 `--state all` 时，假 gh 只返回 OPEN 的 PR（`harvest.test.sh:110-112`）。`harvest.sh:107` 是分成两个参数传的 `--state all`，和这个匹配写法一致。如果去掉它，109 那条已合入 PR 的记录会被过滤掉，分支就会被推回远端，测试 109 的"没有被推回去"断言会变红，所以能抓住这种回退。 |
| L2 禁止动作检查只覆盖了第一轮 | 已修 | 第 4、5、6、7 节在清空日志前都会先追加到 `gh.all.log`，第一轮、第二轮、dry run 和轮换标记那一轮全部收齐。新加的 `pr list ≥ 20` 断言也有效：第一轮最多只有 13 个票分支，单靠一轮到不了 20。 |
| L3 没测快进推送 | 已修 | 第二轮前给 101 追加了一个提交，并补上新一轮的 implementer/reviewer 记录。随后断言远端已快进到本地最新提交，输出是 `pushed; open PR exists`。这走的正是 `harvest.sh:125-127` 都不命中、`need_push=1` 的路径。 |
| L6 输出措辞误导 | 部分修复 | `:157-159` 现在会区分 `pushed`、`up to date` 和 `dry run`。还剩一点：dry run 下每行开头仍然是 `PUSHED`，`pushed_only` 计数照样加 1；dry run 时也会把 "up to date" 盖成 "dry run"。只是措辞问题。 |
| L8 测试不隔离 | 已修 | 轮换标记路径改为可以用 `HARVEST_TOKEN_MARKERS` 注入。`run_h` 只指向测试里的临时路径，开发机上如果真有这个标记，也不会再让测试误红。 |

## 新发现（都是 LOW）

- **N1** `harvest.sh:76`：`for f in $TOKEN_MARKERS` 没加引号。如果路径里有空格，会被拆成几段，导致标记检查失效（这时是放行，不是拦截）。现在 plist 里的宿主路径 `/Users/silencehan/Projects/NewChanlun-sandcastle-host` 没有空格，不受影响。
- **N2** 只要设了 `HARVEST_TOKEN_MARKERS`（比如设成一个不存在的路径），就会整体替换默认的两条路径，绕过轮换标记检查。这个变量只能由操作者在环境里设置，launchd plist 没设它；设成空串时会回退到默认值。可以接受。另外，写死的第二条绝对路径现在没有测试覆盖，这是改成可注入后正常的代价。

## 安全不变量

- 这次 diff 没有加任何新的 git 或 gh 动作。推送仍然只有 `:151` 一处，写法是 `git push -q origin "$sha:refs/heads/$br"`，不带 `-f`，也不带 `+`。
- 没有出现关票、合入 PR、推 main、force push、删分支或跑 cargo 的路径。
- 第 7 节对脚本源码的静态检查仍然通过。

## 看过 / 跑过

- **看过**：
  - 第 2 轮报告全文；
  - `git diff 5c0cd67c32..HEAD`；
  - `harvest.sh:60-169`；
  - `harvest.test.sh:1-135`；
  - harvest 的 launchd plist（它的环境变量里没有 `HARVEST_TOKEN_MARKERS`）；
  - `.sandcastle/` 下所有 `HARVEST_*` 的引用。
- **跑过**：`bash .sandcastle/harvest.test.sh`，pass=55，fail=0。
- **没做**：按要求没跑 cargo、没调用 gh、没改任何文件。这一轮也没有用 `/bin/bash` 单独再跑一遍测试。
