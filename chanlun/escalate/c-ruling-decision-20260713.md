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
5. 【已裁 2026-07-13】孤儿段归属：采**方案 C —— 显式追加式 `Unassigned` 账本**。禁止沉默孤儿（方案 A），禁止无证据强制吸收（方案 B）。
   - 依据一（#61）：三方案判定矩阵六维度 C 全强；成因分类学表明漏段大头为「当前数据下真不可组合」，另有尾段 4 例属在线待定。见 `chanlun/review-results/q5-orphan-attribution-research-20260713.md` §2-§3。
   - 依据二（#62）：完全分类原则原文调研——C 的三值语义（`Assigned / Unassigned / Tombstoned`）本身即对孤儿状态空间的完全分类；A 违反完备性与无含糊性（chan99/0013:29），B 违反判据纯度（blog/084:50「归纳性、划分不唯一」）与当下性。见 `chanlun/review-results/q5-complete-classification-principle-20260713.md`。
   - 约束：账本按 `(target_level, lower_id)` per-level 建账（完全分类是级别性的，chan99/0013:29）；assignment 与 host supersede 用同一 correlation 原子追加，旧行不覆盖；区间套查询仍只消费 Assigned+Completed，coverage gap 显式返回；`ORPHAN_TOMBSTONED` 仅在 #61 §4.3 严格合取条件下允许。
   - 定位：本裁决解决的是**记账语义**（消灭幽灵状态、计数可重放、可审计完整），不改变孤儿段的客观存在；其能否转正取决于组装器规则演化（#53/#55 及组装器生产化）。

## 生效边界

- 本裁决不修改塔代码与 Lean 形式化；组装器生产化、#54 计数重放、消费入口迁移按后续任务推进。
- `.chanlun/definitions/level_recursion.md` 的 `Move[k]` 语义维持定义口径不变；需补一份 WindowUnit/CompletedMove 口径映射注释（后续任务）。
