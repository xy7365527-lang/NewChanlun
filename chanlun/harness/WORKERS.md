# WORKERS.md — 工作者接线单（harness 路由，单一事实源）

> 任何被派发进本项目的 agent（codex/claude/kimi），开工前读完本文件即完成引导。
> 派发词从此只含：指向本文件的指针 + 任务专属段。
> 方法论出处：lopopolo/harness-engineering（观测缺口驱动、可执行约束 > 散文、能力/权限分离、证明纪律）。

## 1. 接线（读什么、写到哪）
- 路线图：`chanlun/plans/mainline-merged-roadmap-20260717.md`
- 花名册：`chanlun/agent-roster-20260715.md`（旧版 20260714 已废）
- 裁定库：`chanlun/escalate/`（裁定一经落档即为约束，不复议）
- 评审产物：写入 `chanlun/review-results/`（命名：`<主题>-<YYYYMMDD>.md`）
- 临时/大输出：写 `/tmp`，正文只留路径与摘要

## 2. 权限边界（可执行约束优先）
- **主仓代码禁写**：`/Users/silencehan/Projects/NewChanlun` 下 rust/ 等代码区只读；docs（chanlun/）可写。
- **cargo 禁建**：授权门已实装（rust/.cargo/config.toml → cargo_gate.sh，哨兵 `.chanlun/locks/CARGO_BAN`）。无哨兵放行则 build 直接拒绝——不要绕。
- **worktree 互斥**：一个 worktree 同时只允许一个写入者。进场前执行：
  `cat <worktree>/.chanlun/locks/WORKTREE_OWNER 2>/dev/null` —— 非空且不是你 → 换树或等待。开工写入自己的会话标识，退场删除。
- 判定歧义时：停下上报，不要自行扩权。

## 3. 证明纪律
- 结论必须 `文件:行` 归因；对账用 `RECON_PASS` / `RECON_FAIL` 显式落标。
- 不确定就写不确定；禁止把"没跑完"报成"通过"。
- 长任务必须带 EXIT 标记与心跳（见 `runbooks/long-run.md`），"进程存在"不算活着，"有心跳"才算。

## 4. 长任务
见 `chanlun/harness/runbooks/long-run.md`（R1–R5）。

## 5. 方法论原文库（vendored）
`chanlun/harness/vendor/harness-engineering/`（上游 commit 226c8d3，CC-BY-4.0，见 PROVENANCE.md）。
入口：`README.md`、`AGENTS.md`、`playbooks/improve-harness.md`（加新环的操作程序）、`playbooks/repository-review.md`、`docs/proof/`、`sources/ryan-notes.md`。
本项目转述（本文件与台账）与原文冲突时，以原文 + 本地裁定为准。

## 6. 派发词最小模板
```
读 chanlun/harness/WORKERS.md 并遵守其全部边界。
[任务专属段：目标 / 输入路径 / 产物路径 / 完成判据]
```
