# #1672 关闭来源与 #1406 承接对账 27U

## 结论

#1672 是被 GitHub 关键字自动关闭的，不是有意关票；关闭时四道关票门未满足，已于本轮恢复 OPEN 并附说明。#1406 已追加一条 AC1/AC5 承接评论。父票 #1406、#1279 未动，未删工位或分支，未改隔离。

## 关闭来源

- 关闭事件：2026-10-27T00:48:18Z，timeline `closed` 携 `commit_id=b9a84b01467772d847f9d222207a305136761bf4`，即 PR #1702 squash 合入后 2 秒，`stateReason=COMPLETED`，无任何关票评论。
- 触发文字：squash 提交正文 `* fix: #1672 按BSD stat读取载体文件系统`。`fix: #n` 是 GitHub 关闭关键字（`.sandcastle/harvest.sh:33-34` 的 CLOSE_RE 也把这种写法列为会关票）。harvest.sh 的防护只覆盖它自己推送的分支；PR #1702 的分支是 `codex/issue-1672-fin2-hfs`，未经 harvest，防护没起作用。
- PR 正文写 `Part of #1672`，`closingIssuesReferences` 为空，说明没有人打算以合入关票。

## 逐门判定（docs/agents/delivery-discipline.md）

| 门 | 状态 | 依据 |
|---|---|---|
| 1 resolution 引用落地 hash | 未满足 | 关闭时无评论 |
| 2 已并入 main | 满足 | `b9a84b01`，对应候选 `e7a9668a` |
| 3 声明在关票时为真 | 未满足 | 无声明可核 |
| 4 逐 commit 可编译 | 待 owner 声明 | squash 单提交入 main，按模板写即可 |
| 5 工位处置 | 未满足 | `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree`（`codex/issue-1672-fin2-hfs` @ e7a9668a，工作区干净）仍在；远端分支已不在 ls-remote 中 |
| 6 报告入仓 | 未满足 | origin/main 无任何 `*1672*` 路径；原件只在本目录 `1672-RUNTIME-ACCEPTANCE27.md`、`1672-INTEGRATED-INDEPENDENT27.md`、`1702-INTEGRATION-INDEPENDENT27Q.md`、`1672-32M-evidence27/` |
| 7 评审面==关票面 | 满足 | `1702-INTEGRATION-INDEPENDENT27Q.md` 审的正是 `e7a9668a`，静态 PASS |
| origin/main CI 条款 | 未满足 | 见下节 |

只因 CI 未终态不足以重开；重开的依据是第 1、3、5、6 门同时缺失。

## main CI 实查（本轮 gh run view）

- CI macOS 40428705046 @ b9a84b01：in_progress。`issue1716-real-s`、`issue1717-real-s` success；`macos-session` 仍在跑，其中 `FIN/2 WE7 — four independent HFS+ carriers（#1672）` 与 `Archive FIN/2 originals and carrier cleanup (#1672)` 两步 completed/success。这是 FIN/2 首次在新 head 上的动态证据。
- 常规 CI 40428705199 @ b9a84b01：queued 状态，已完成的 job 全部 success；`rust-test-s-session` shard 0/1、`extras (native)` 在跑，`extras (owned)` 排队。

## 已写入 GitHub

- #1672 恢复 OPEN，评论列出自动关闭原因、已成立项、四项缺口与恢复条件。
- #1406 评论 https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6431366625 ：#1702 合入为 AC1（实际 Capture/Resolve/Apply 有限基线）与 AC5（独立账簿 oracle、四域恢复、原信封重送对拍）提供首个实例；不覆盖 AC4 正式会话入口/浏览器核对与真实 S 全链（#1673）；不可据此勾选；容量读数（`1672-32M-evidence27/final-run/artifact/storage.jsonl` 等）绑定 run 40389540213。#1406 合入前最近的评论都早于 00:48，没有重复。

## 恢复条件与下一步（owner：#1672 现有 owner / 主控派发）

1. 等 40428705046、40428705199 终态。任一失败交原 owner 返修，不重跑。
2. 把业务验收、集成独审与 run 40389540213 关键原件入仓到 `.chanlun/review-results/issue1672-*/`（纯文档提交，走现有文书 main 门）。
3. 处置 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree`（工作区干净，HEAD 已在 main 的 squash 中）。
4. 按模板发布关票声明：`resolution=b9a84b01…，已并入 main；主线 origin/main @ b9a84b01；CI run：<两条链接> → <结论>；工位=已删；报告=<仓内路径>；评审面==关票面（e7a9668a，27Q）`，然后关闭。
5. 防再发：squash 合入任何非 harvest 分支时，逐提交标题会拼进 squash 正文，其中的 `fix: #n` 照样关票。建议主控合入时用 `gh pr merge --squash --body "<不含关闭关键字的说明>"` 覆盖正文。本报告不改脚本。
