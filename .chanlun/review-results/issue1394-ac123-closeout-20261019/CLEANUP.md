# #1394 关票清理清单（2026-10-20）

## 已删 worktree（63 个 #1394 + sc-wt/1405pm、sc-wt/1420pm）
未提交改动与非忽略未跟踪文件逐个归档于 worktrees/<路径>/（uncommitted.patch、untracked.tar.zst），索引 worktrees/INDEX.tsv。

## 已删分支（本地 80，远端 3：codex/1394-integration-20261013、codex/1394-q-functional-clock-20261015、codex/1394-transport-revision-20261012；PR 分支 codex/1394-ac123-closeout-20261019 远端已随 PR 删除）
分支名与删前 SHA：branches-deleted.txt；其中 29 个不在 main 的分支已入 branches-not-in-main.bundle（basis = origin/main，git bundle verify 通过）。

## 保留（移交 #1552）
- codex/1394-acceptance-integration-20261018 @ 29cf83564d（含 codex/1394-product-capture-20261018）及 worktree /Volumes/AgentStorage/issue1394-sandcastle-integration-20261018：读侧适配器修复，#1552 票面已引用。
- detached worktree /Volumes/AgentStorage/issue1394-closure-last-byte-20261018 @ 99e819192b（C1，不在 main）：统一 source 1488→1419→C1 的组成。
- detached worktree /Volumes/AgentStorage/issue1394-real-contract-v2-20261018/producer-source @ b46（已在 main）：#1552 workbench 内。
- 其余 /Volumes/AgentStorage/issue1394-*-20261018 workbench 目录（非 git 工位）原样保留，供 #1552。

## 不属本票、未动
- issue1394-clean-verify（fugue_v3 D1 adapter，名含 1394 但内容属 D1 线）
- codex/1405-legacy-*、codex/1405-root-set-20261008、codex/1419-v3-int1405-20261016
