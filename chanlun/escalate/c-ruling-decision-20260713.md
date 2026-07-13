# C 裁决决定：C2 —— 定义语义为准，经消费层组装器实现，不动塔

- 日期：2026-07-13
- 裁决人：P0
- 材料：chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md（#57）
- 支撑证据：#58 组装器 spec+原型+前缀稳定性硬门；#59 入场时点审计；原文语义立场（终结证据口径，「恰三段窗口」无原文对应）
- 待补证据：#60 C1/C2 全历史行为差异重放（进行中；若结果与本裁决前提冲突，须回呈 P0）

## 对材料第 7 节 5 问的裁定

1. 生产上要求「完成的走势类型」的消费者，其法定对象为 `CompletedMove[k]`（消费层 as-of 组装器视图）。塔产物正名为 `WindowUnit[k]`，两者禁止同名无注释互引。
2. 「窗口封闭」不得替代「走势类型完成」进入递归构件资格。
3. 作废（C1 专属问题）。
4. 授权 b 路线双轨分层：塔保持不可变 WindowUnit 构造层；完成走势只由消费层 as-of 纯函数组装器产生；三道无 repaint 防线（前缀稳定性 property test、entry_bar ≥ judge_at+1 硬门、supersede 事件账本）为强制项。
5. 【未裁】孤儿段归属：待 P0 确认。执行侧建议为显式 `Unassigned` 账本（追加式，禁止沉默孤儿，禁止强制吸收）。

## 生效边界

- 本裁决不修改塔代码与 Lean 形式化；组装器生产化、#54 计数重放、消费入口迁移按后续任务推进。
- `.chanlun/definitions/level_recursion.md` 的 `Move[k]` 语义维持定义口径不变；需补一份 WindowUnit/CompletedMove 口径映射注释（后续任务）。
