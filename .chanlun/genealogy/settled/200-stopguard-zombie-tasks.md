---
id: "200"
number: 200
type: 语法记录
status: 已结算
date: 2026-02-25
trigger: Stop-Guard 持续阻断（僵尸任务队列）
depends_on:
  - 186
---

# 200号：Plan 阶段任务未自动清理导致 Stop-Guard 僵尸阻断

## 结论

Plan 阶段（ExitPlanMode 前）创建的 TaskCreate 任务在实现完成后不会自动标记为 completed。
这导致 Stop-Guard 持续检测到"活跃任务"并阻断停止，即使所有工作已完成。

## 定义依据

- Stop-Guard 检查 `~/.claude/tasks/{session-id}/*.json` 中 status != completed 的任务
- Plan 阶段的任务是实现计划的分解，不是运行中的 agent
- 实现完成后这些任务变成僵尸——status 仍为 in_progress 但无对应的执行实体

## 边界条件

- 如果 Lead 在实现过程中逐步 TaskUpdate 每个任务为 completed，问题不会发生
- 但当 Lead 直接实现（不通过 team 分派）时，任务状态不会自动更新

## 下游推论

1. 实现完成后应检查并清理 plan 阶段的任务队列 → [resolved: post-plan-cleanup 工位实现]
2. 或者：Stop-Guard 应区分"plan 任务"和"agent 任务" → [resolved: 选择200号-1路线]
3. 186号（clean_terminate 计算时序）是同类问题的先例 → [resolved: depends_on 已引用]

## 修复

手动将 6 个任务标记为 completed。后续 session 应在 commit 后检查任务队列状态。

## 谱系引用

- 186号：clean_terminate 计算时序缺陷
