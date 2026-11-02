# #1488 剩余工位与分支处置完成

执行窗口 `2026-10-27T09:47:55Z` 至 `2026-10-27T09:49:58Z`（`date -u`）。授权：用户批准主控提请的 `cleanup1488-remaining-refs`。#1488 当前仍 OPEN，关票由主控发布。

## 结论

27Q 步骤 2、3 已全部完成。文书工位已普通移除，35 条本地分支与 21 条远端分支已按精确 tip 条件删除，失败 0 条。每条被删 tip 都在独立恢复库有具名引用，逐条回读一致。#1488 关票门只剩主控发布 resolution 并关闭。

## 动作前核对

- 33 条本地、21 条远端历史 tip 与 `1488-REF-PRESERVATION27R.tsv` 逐条相同，无变动项。远端读数取自 `git ls-remote`。
- 文书工位 `/Volumes/AgentStorage/issue1488-closeout-doc-20261026` 检出 `codex/1488-six-disposition-archive-20261027@11b5597fb581f85706d1f2214aaf08fc01ec8d95`。tracked 干净，只有 7 个 ignored `__pycache__` 目录，内容全部是 `.pyc`。`lsof +D` 只见本代理自身的 shell，原 gh PID 41286 已不在。
- 其余分支均无 worktree 检出。
- PR1744 MERGED，head `11b5597f`，merge `fcfdb0f71042d7634bb44002d8b915de414ee6e5`，合后 CI 40428652952 completed/success（headSha 即 fcfdb0f7）。PR1725 MERGED，head `86c83ee4`，merge `ce7b29c52bfc01abe4fe80cf3b1f70a11aa1cf32`。两个 merge 均是当前 main `00fdf0dd` 的祖先。
- 两条文书分支 tip 与各自 merge commit 在 PR 改动文件上 `git diff` 为空，内容完整入 main。远端 `codex/1488-six-disposition-archive-20261027` 动作前已不存在（合入时自动删除），`codex/1488-closeout-evidence-20261026` 远端此前已不存在。

## 处置结果

| 对象 | 结果 | 恢复入口 |
|---|---|---|
| 两条文书分支 tip | 先 `fetch` 新建具名引用，回读 `86c83ee4886db171d50d5c161e4b56542d04a24f`、`11b5597fb581f85706d1f2214aaf08fc01ec8d95`，连通性检查无缺失 | `restore.git` 的 `refs/archive/issue1488-branch-local/codex/1488-closeout-evidence-20261026`、`…/codex/1488-six-disposition-archive-20261027` |
| 文书工位 | 删除 7 个 pycache 目录内的 `.pyc` 后，`git worktree remove` 非 force 成功，目录已不存在 | 内容在 main（ce7b29c5、fcfdb0f7）及上行具名引用 |
| 本地 35 条（33 历史 + 2 文书） | 逐条 `git update-ref -d refs/heads/<name> <预期tip>`，35 条 deleted；`refs/heads` 下 1488 前缀剩 0 | `refs/archive/issue1488-branch-local/<完整分支名>` |
| 远端 21 条 | 一次非原子 `git push`，每条带 `--force-with-lease=refs/heads/<name>:<预期tip>` 的删除，21 条 `[deleted]`；`ls-remote` 复读剩 0，`fetch --prune` 后跟踪引用剩 0 | `refs/archive/issue1488-branch-remote/<完整分支名>` |

恢复库引用从 936 增至 938，新增仅两条文书引用。`issue1488-branch-local` 35 条、`issue1488-branch-remote` 21 条，按 TSV 54 条逐条回读不一致数 0。逐条结果见 `1488-FINAL-DISPOSITION27AN-local.tsv`，远端回执原文见 `1488-FINAL-DISPOSITION27AN-remote-push.txt`。

恢复某个分支到新名字、不覆盖现有分支：

```bash
git -C /目标仓 fetch /Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git \
  refs/archive/issue1488-branch-remote/codex/1488-uo-ci-20261019:refs/heads/recovered/1488-uo-ci-20261019
```

保留未动的对象：6 个 `nc1488*` reference-coding 工位（仍在 `git worktree list`）、主仓 14 条 `refs/archive/issue1488-retired-*`、共享 stash 32 条、恢复库原有 936 条引用、主仓与全部证据目录。没有停服务、改隔离或改 store。

## 历史限制

十个历史 `relations.jsonl` LFS 版本的源库在本轮之前就已缺失，不可恢复。恢复入口是 `restore.git`，`export.git` 不是恢复入口。

## 关票模板（主控发布）

`关票声明：resolution=8bbe4499d983e674d435261b207a6556715600b4（实现，PR1711），文书=ce7b29c52bfc（PR1725）与 fcfdb0f71042（PR1744，合后 CI 40428652952 success），均已并入 main；本评论各项在此刻均已为真；逐 commit 可编译性=实现单 squash，文书纯文档豁免；工位=42 归档容器已处置，文书工位 issue1488-closeout-doc-20261026 已普通移除；遗留 branch 本地 35 条、远端 21 条已按精确 tip 条件删除，名字到 commit 的映射保全于 /Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git 的 refs/archive/issue1488-branch-local/* 与 refs/archive/issue1488-branch-remote/*；报告=.chanlun/review-results/1488-acceptance/resume-20261025/REPORT.md、.chanlun/review-results/issue1488-closeout-20261026/README.md、/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1488-FINAL-DISPOSITION27AN.md；评审面==关票面。限制：十个历史 relations.jsonl LFS 版本源库本就缺失，不可恢复；export.git 不是恢复入口。#1279 保持 OPEN。`

## 剩余真实缺口

- 主控发布上述 resolution 并关闭 #1488。
- 关闭后向 #1491、#1640、#1552 交付 27Q“下游消费”表中的指针与关票评论链接，由唯一 wayfinder_engine 放行。#1527 有既有 owner，不重派。
- 本报告仅存于外盘，未入仓；需入 git 时走普通文书归档。
