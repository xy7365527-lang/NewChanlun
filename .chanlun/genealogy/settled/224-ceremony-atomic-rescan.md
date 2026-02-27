---
id: '224'
number: 224
title: ceremony push→rescan 原子性断裂——用户消息中断 ceremony 序列
type: meta-rule
status: 已结算
date: 2026-02-27
depends_on:
  - '137'   # 否定性禁令对行为执行层无效
  - '143'   # commit 后总结是 RLHF 停顿点
  - '218'   # Lead 并行化
tensions_with: []
topo_effect: "post-commit-flow.md 新增 push→rescan 原子性规则"
---

# 224号：ceremony push→rescan 原子性断裂

## 现象

Lead 在 ceremony 序列执行中反复"断掉"：push 完成后（步骤7），不执行 rescan（步骤8），而是：
- 输出格式B（"无待做行动"）
- 回应编排者消息
- 输出总结段落后停下来

编排者观察："你现在就断掉了"。

## 根因

1. **用户消息中断 ceremony 序列**：编排者消息作为 system-reminder 到达时，Lead 放弃 ceremony 序列去回应消息。push→rescan 之间没有原子性保证。
2. **否定性禁令无效**（137号）：lead-parallel-dispatch.md 禁止"等待确认再 re-scan"，但否定性禁令对行为执行层无效——Lead 不是"等待确认"，而是"被消息吸引走了"。
3. **post-commit-flow.md 缺口**：只规定了"commit/push 后在同一个输出中完成总结和下一步行动"，但没有显式提到 rescan 是 push 后的强制下一步。

## 修复

在 post-commit-flow.md 中新增正面格式约束（137号要求）：

1. push 后唯一合法输出：`已 push。→ 接下来：rescan` + 工具调用
2. ceremony 执行中收到的用户消息在 ceremony 完成后统一回应
3. ceremony 序列不可被用户消息中断

## 边界条件

- 如果编排者发送 INTERRUPT（显式中断指令），ceremony 可以被中断——INTERRUPT 优先级高于原子性
- 普通消息（讨论、方向指示）不构成 INTERRUPT

## 下游推论

1. 所有 ceremony 的退出路径（步骤2干净终止、步骤10不动点）都需要检查是否有未回应的用户消息
2. 如果 ceremony 完成后有积压的用户消息，按时间顺序统一回应
