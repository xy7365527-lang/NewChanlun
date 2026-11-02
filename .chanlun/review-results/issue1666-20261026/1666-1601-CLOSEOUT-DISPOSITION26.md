# #1666 与 #1601 七工位已完成可恢复处置

核验结束于 2026-10-26 04:20 UTC。本轮绑定 #1709/#1323。七处原路径与 worktree 登记均已消失，六个指定本地分支均按精确旧 SHA 删除。两个自包含 bundle 已从空仓实际导入，十个恢复 ref 的 HEAD 与完整父链均与原仓相同。没有重跑 Rust。

恢复包位于 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1666-1601-closeout-recovery26/`。主控可运行该目录的 `verify_recovery.py` 复核。恢复命令在 `RECOVERY.md`，完整输出在 `VERIFY-RESULT.txt`。

## 七工位逐项结果

所有工位即时 HEAD 均与输入报告相同。tracked 与 untracked 在制品均为零。每个工位删除前的 `lsof +D` 均无输出，返回 1 且无 stderr。命令行进程检索无消费者。

| 对象 | 原路径 | 原 HEAD | 处置 |
|---|---|---|---|
|1666-wt|`/Volumes/AgentStorage/frontier-audit-20261022/wt-1666`|`238630af12b93e5635f7b43904620efccaf8f71c`|detached 历史入 host.bundle，非 force remove 成功|
|1666-archive|`/Volumes/AgentStorage/issue1666-review-archive-20261026`|`1bfb249feae6c13e73e9aeba0a67bbed09221ac1`|历史入 main.bundle，非 force remove 成功|
|1666-consume|`/Volumes/AgentStorage/issue1666-consume1723-20261026`|`23c90596253b74cb8f93f6e9efc164145726d080`|detached 历史入 main.bundle，非 force remove 成功|
|1601-rebase|`/Volumes/AgentStorage/rebase-pr1601-20261024`|`107336f3d4ae0e4b2b9b52d3916d62311685829c`|16 个 ignored 文件先移存，非 force remove 成功|
|1601-integration|`/Volumes/AgentStorage/issue1601-integration-20261026`|`5eb523f6a7b1e8ce8997addeda74470bb15f952a`|10 个 ignored 文件先移存，非 force remove 成功|
|1601-review|`/Volumes/AgentStorage/final-review-wave-20261025/1680/worktree`|`107336f3d4ae0e4b2b9b52d3916d62311685829c`|detached 历史入 main.bundle，非 force remove 成功|
|1601-consume|`/Volumes/AgentStorage/pr1680-consume1723-20261026`|`8635ad92cc2e1806d38ee9d3799dae59d2fff8e6`|detached 历史入 main.bundle，非 force remove 成功|

即时 owner 核对采用主控明确的七处退役许可、运行中代理列表、应用 pinned/cwd、Sandcastle 最新终态和本地进程。七处没有在飞 owner。`workers-evidence.txt` 保存 #1666 与 #1601 最新 sandbox closed 原行。`sidebar-owner-evidence.json` 保存 pinned/cwd 结果。`preflight.json` 保存逐工位原始状态与 lsof 结果。`process-evidence.txt` 为空命中记录。

`disposition.json` 逐对象记录删除时钟与缓存移存结果。七份 `*-git-admin.tar` 保存原 worktree 的 HEAD、index 和 logs。

## 六个本地分支已删除

每项均先确认未被任何已登记 worktree 检出，再执行 `git update-ref -d <ref> <旧SHA>`。随后 `show-ref --verify --quiet` 返回 1。完整记录在 `deleted-refs.json`。

| 原仓 | 本地 ref | 精确旧 SHA |
|---|---|---|
|host|`refs/heads/sandcastle/issue-1666`|`4088b1030a232848c94b8c65e1a808ebf7e5dfec`|
|host|`refs/heads/sandcastle/issue-1601`|`01a06da6d05376d83f3ea7a127c53c7760240f94`|
|main|`refs/heads/codex/1601-stroke-integration-20261026`|`06a095112fb6fbf94f90d55c19e027ad76c63830`|
|main|`refs/heads/codex/1666-review-archive-20261026`|`1bfb249feae6c13e73e9aeba0a67bbed09221ac1`|
|main|`refs/heads/sandcastle/issue-1601`|`107336f3d4ae0e4b2b9b52d3916d62311685829c`|
|main|`refs/heads/codex/1601-integration-20261026`|`5eb523f6a7b1e8ce8997addeda74470bb15f952a`|

host 指 `/Users/silencehan/Projects/NewChanlun-sandcastle-host`。main 指 `/Users/silencehan/Projects/NewChanlun`。同名 `sandcastle/issue-1601` 在两仓的历史不同，分别保存在两包中，没有用 squash 后的 main 替代工蜂历史。

## 恢复证据与缓存

`host.bundle` 为 518482179 字节，`main.bundle` 为 545405130 字节。两包的 `git bundle verify` 均报告完整历史，无先决对象要求。实际 `git init --bare` 空仓导入结果保留在 `host-restored.git` 与 `main-restored.git`。十个 ref 均核精确 HEAD、完整 `rev-list --parents` 和树对象可读。完整比较序列按仓名前缀存为 `host-refs-*-history.txt` 与 `main-refs-*-history.txt`，核验记录在两份 `*-verification.json`。

26 个 ignored 文件共 288246 字节，原样 rename 到恢复包的 `ignored/1601-rebase/` 与 `ignored/1601-integration/`。两处均含 `.pytest_cache/`、`tests/__pycache__/`、`trading_system/__pycache__/`。移存后逐目录文件路径与字节数相等，原工位 ignored 归零，再执行非 force remove。其余五处没有 ignored 文件。

`verify_recovery.py` 已实际通过。它复核十个恢复 ref 的完整父链、七个原路径与登记消失、六个分支消失，以及六个缓存目录的文件数和字节数。脚本首次暴露 host/main 同名 ref 的历史快照文件名冲突，已改成带原仓前缀并重新从原 SHA 与恢复仓逐行比较通过。原始 bundle、历史和 ref 未受该报告文件名问题影响。

## 主控剩余关票门

#1666 的本轮七子句第 5 项处置门已解除。主控已提供精确 main CI 40258689207@a74b3b9db5b7b3df0aeab938a45e4d50b9df8d8f completed/success。保留原独审五项 LOW、Linux 证据范围和 macOS 未实跑说明，由主控发布实际最终关票声明。

#1601 的旧四工位和四分支处置门已解除。主控已提供精确 main CI 40259436828@a46ab454a83db3a3d1ad5fc3f54ca2db1b394d87 completed/success。完整原始独审与初次集成独审入 git 仍由 PR #1728 文书 owner 完成。`/Volumes/AgentStorage/issue1601-review-archive-20261026` 及其 `docs/1601-review-archive-20261026` 分支不在本次七处允许集中，保持原状。主控消费 PR #1728 后，再核该文书工位的实际关票处置要求。

本轮没有关票、合 main、改 remote、改 S 账本、运行 Rust 或终止服务。#1585 归档 owner 与 #1527 在飞工位不在处置范围。没有使用 force、rm-rf 或 prune。

`df -h /` 前后均显示 Size 926Gi、Used 12Gi、Avail 137Gi。原输出在 `df-before.txt` 与 `df-after.txt`。本轮以可恢复处置为验收，不报告磁盘回收量。

采用 Prove It Works，选择空仓导入并核完整父链，而非只看 bundle 文件存在。采用 Sequence Work into Verifiable Units，逐工位完成移存、删除和消失检查后才处理下一处。采用 Build the Lever，留下可重跑的 `verify_recovery.py`。
