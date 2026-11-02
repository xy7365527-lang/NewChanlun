# #1488 遗留分支与文书工位的最终处置

2026-10-27 按用户批准的 `cleanup1488-remaining-refs` 执行，窗口为 `2026-10-27T09:47:55Z` 至 `09:49:58Z`。文书工位已普通移除，本地 35 条、远端 21 条 #1488 分支按精确 tip 条件删除，失败 0 条。每条被删 tip 都在独立恢复库有具名引用。

## 本目录文件

| 文件 | 内容 | 与外盘原件的关系 |
|---|---|---|
| [1488-FINAL-DISPOSITION27AN.md](1488-FINAL-DISPOSITION27AN.md) | 动作前核对、逐类处置结果、恢复命令、历史限制 | 逐字节相同 |
| [1488-FINAL-DISPOSITION27AN-local.tsv](1488-FINAL-DISPOSITION27AN-local.tsv) | 35 条本地分支的名称、预期 tip、结果 | 逐字节相同 |
| [1488-FINAL-DISPOSITION27AN-remote-push.txt](1488-FINAL-DISPOSITION27AN-remote-push.txt) | 远端 21 条删除的 `git push --porcelain` 回执 | 仅去掉 GitHub 横幅 4 行的行尾空白 |
| [1488-REF-PRESERVATION27R.tsv](1488-REF-PRESERVATION27R.tsv) | 54 条历史分支名到完整 commit 的映射及回读值 | 仅把 CRLF 行尾改为 LF |
| [1488-REF-PRESERVATION27R.md](1488-REF-PRESERVATION27R.md) | 54 条具名保全的方法与读数 | 逐字节相同 |

外盘原件仍在 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/`，未改动。两处行尾调整只为通过 `git diff --check`，字段与内容不变。

## 恢复入口

恢复库为 `/Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git`，共 938 条引用。被删分支在以下命名空间：

- `refs/archive/issue1488-branch-local/<完整分支名>`，35 条，含两条文书分支 `codex/1488-closeout-evidence-20261026@86c83ee4886db171d50d5c161e4b56542d04a24f` 与 `codex/1488-six-disposition-archive-20261027@11b5597fb581f85706d1f2214aaf08fc01ec8d95`。
- `refs/archive/issue1488-branch-remote/<完整分支名>`，21 条。同名本地与远端分支 tip 可能不同，分别保存。

恢复命令见 [处置报告](1488-FINAL-DISPOSITION27AN.md) 的“处置结果”节。十个历史 `relations.jsonl` LFS 版本的源库本就缺失，不可恢复；`export.git` 不是恢复入口。

## 勘误

处置报告的结论写“#1488 关票门只剩主控发布 resolution 并关闭”，关票模板的报告字段引用外盘绝对路径。两处都漏了一步。`docs/agents/delivery-discipline.md` 关票门第 6 项要求本票实体报告已入 git、以仓内相对路径引用，外盘或绝对路径视同未入仓。当时处置报告只在外盘，所以关票前还需把它入仓。本目录及其合入 main 补上这一步。原报告保持原样，以下模板取代其中的关票模板。

## 关票模板

本目录的归档 PR 合入 main 后，填入其 merge SHA 再发布：

`关票声明：resolution=8bbe4499d983e674d435261b207a6556715600b4（实现，PR1711），文书=ce7b29c52bfc01abe4fe80cf3b1f70a11aa1cf32（PR1725）、fcfdb0f71042d7634bb44002d8b915de414ee6e5（PR1744，合后 CI 40428652952 success）与 <本归档 PR merge SHA>，均已并入 main；本评论各项在此刻均已为真；逐 commit 可编译性=实现单 squash，文书纯文档豁免；工位=42 归档容器已处置，文书工位 issue1488-closeout-doc-20261026 已普通移除，遗留 branch 本地 35 条、远端 21 条已按精确 tip 条件删除，名字到 commit 的映射保全于 restore.git 的 refs/archive/issue1488-branch-local/* 与 refs/archive/issue1488-branch-remote/*；报告=.chanlun/review-results/1488-acceptance/resume-20261025/REPORT.md、.chanlun/review-results/issue1488-closeout-20261026/README.md、.chanlun/review-results/issue1488-closeout-20261026/final-disposition27an/README.md；评审面==关票面。限制：十个历史 relations.jsonl LFS 版本源库本就缺失，不可恢复；export.git 不是恢复入口。#1279 保持 OPEN。`

## 保留未动

6 个 `nc1488*` reference-coding 工位、主仓 14 条 `refs/archive/issue1488-retired-*`、32 条共享 stash、恢复库原有 936 条引用、主仓及证据目录均保留。
