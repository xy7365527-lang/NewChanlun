# #1585 工位处置归档索引

[已完成处置报告](1585-CLOSEOUT-DISPOSITION26.md) 逐字复制仓外原件，记录 2026-10-26 的三处工位移除、四个本地分支删除和恢复验证。报告中的“目前仅在仓外”“入 git 门尚未满足”“没有自建 PR”均是报告生成时的历史状态，不代表本归档后的状态。原件不回写。

本次新增报告和本索引，绑定 #1709，名分为世代宪法 §5 的工作草稿。既有 [P1 返修评审索引](p1-review-index-20261026.md) 和历史报告保持原文。

## 技术交付与文书合入分开验收

[PR #1664](https://github.com/xy7365527-lang/NewChanlun/pull/1664) 已合入，落地提交为 `7fe402cd79b0afdad87ed2a828d6d75fb8ade07b`。[CI 40259428149](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40259428149) 的 `headSha` 为同一 SHA，状态为 `completed/success`。这些状态由本归档单元现场回读，证明该技术交付，不代替本次归档候选的独审和必要 CI。

本次归档候选仍待非作者异家族独审、必要 CI 与用户 main 合入批准。交付进入 main 后，主控才能按交付纪律核验关票门并引用实际落地提交与仓内报告路径。本次不关闭 #1585 或 #1709。#1279 在完整六项 Destination 验收前保持 open。

## 恢复包继续保存在仓外

仓外原件为 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1585-CLOSEOUT-DISPOSITION26.md`。恢复目录为 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1585-closeout-recovery26/`，属于本机恢复依赖，不是仓内相对链接。

- `RECOVERY.md` 说明独立读取历史及原路径恢复的条件。
- `bundle-verification.json` 记录空裸仓导入和四个 ref 的精确恢复结果。
- `disposition.json` 记录实际处置命令及其退出码。
- `host.bundle`、`main.bundle`、两个已验证裸仓和恢复脚本留在该目录，不入 git。

本次只归档已完成处置，不重新删除或恢复工位。
