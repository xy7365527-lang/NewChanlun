# #1818 旧工位处置 46O

结论：授权范围内两处旧工位和两条旧分支已处置完毕。恢复引用和 bundle 均可读，证据目录、共享 stash 和 45F 新工位都保留。#1818 尚未关票，还缺合后 main CI 结论（run 41378588037）和关票簿记。

依据：gates.md `cleanup1818-old45f` 已 resolved，用户批准“PR1843 合入后”处置这两处工位与两条分支。前提已满足，PR1843@55d012bf 于 2026-11-02T02:35:52Z 合入 main `d77a89b572ba3b3896d486f0fa5d02753d29a6f0`。

## 内容已在 main

- `b636af32` 是 `8ce09b7b` 的祖先。
- main 上的 ADR0059 只比 `8ce09b7b` 多一行状态订正（写明 41W 增量独审 PASS），另增了 #1818 的评审归档。两条分支没有 main 缺失的内容。

## 恢复入口

- 主仓 `/Users/silencehan/Projects/NewChanlun`：`refs/archive/issue1818/b636af32`、`refs/archive/issue1818/8ce09b7b`。
- 宿主仓 `/Users/silencehan/Projects/NewChanlun-sandcastle-host`：`refs/archive/issue1818/b636af32`。
- bundle：`/Volumes/AgentStorage/close1818-cleanup46o/issue1818-old-branches.bundle`。`git bundle verify` 结果为 okay，含上述两个引用，历史完整。
- 恢复命令：`git fetch <bundle> refs/archive/issue1818/*:refs/archive/issue1818/*`。

## 逐对象

| 对象 | 处置前 | 动作 | 处置后 |
|---|---|---|---|
| 工位 `/Volumes/AgentStorage/decisions/1818/land-wt`（宿主仓） | HEAD 在 decision/1818 = b636af32，工作区干净（含忽略文件），无进程占用 | `git worktree remove`（未加 force） | 已不存在；`decisions/1818/` 下的证据文件保留 |
| 工位 `/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`（主仓） | HEAD 在 codex/1818-adr0059-publish = 8ce09b7b，工作区干净，无进程占用 | 先查 list_artifacts，本任务未登记此工位，不适用 archive_worktree；改用 `git worktree remove`（未加 force） | 已不存在 |
| 本地分支 decision/1818（宿主仓） | b636af32 | squash 合入导致 `-d` 判为未合并，在已保全的前提下用 `branch -D` 删除 | 已删 |
| 本地分支 codex/1818-adr0059-publish（主仓） | 8ce09b7b | `branch -d` | 已删 |
| 远端 decision/1818 | b636af32 | `git push origin --delete` | 已删，ls-remote 为空 |
| 远端 codex/1818-adr0059-publish | 处置前远端已不存在，主仓只剩过期跟踪引用 | `fetch --prune` 清掉过期跟踪引用 | 已不存在 |

## 保留未动

- 45F 工位 `/Volumes/AgentStorage/decision1818-archive45f/worktree` 及其分支 codex/1818-closeout-archive45f。
- 共享 stash，主仓 32 条。
- 证据目录。

没有 force-push，没有改服务、隔离或共享台账，本机没有运行 Rust。

## 关票前仍需

- 合后 main CI run 41378588037 和 DevSkim run 41378587740（`d77a89b5`）的最终结论。
- 按关票门发 resolution 关闭 #1818，并在 #1323 的 Decisions so far 补记。
- 45F 工位与分支 codex/1818-closeout-archive45f 是这次合入的作者工位，其处置不在本次授权范围内，另行处理。
