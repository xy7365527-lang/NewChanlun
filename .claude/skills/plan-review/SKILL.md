---
name: plan-review
description: >
  Plan 阶段多模型对审协议。Opus 4.6 出方案，Codex 5.3 High 评审，严格对审直到达成共识后方案定稿。
  修改循环前移到纯文本阶段——不写代码就把方案打磨到位，执行阶段几乎一次通过。
genealogy_source: "160"
---

# Plan 对审 Skill（多模型协作）

## 触发条件

蜂群工位在 Plan 阶段（使用 planner agent 或 EnterPlanMode 后）产出方案时自动激活。

## 工作流程

### Phase 1: Opus 出方案

- planner agent（Opus 4.6）产出实现方案
- 方案以 Markdown 格式输出，包含：需求分析、架构决策、实现步骤、边界条件

### Phase 2: Codex 评审

- 将方案文本传给 Codex（codex-5.3，reasoning_effort: high）
- Codex 从以下角度评审：
  1. 逻辑自洽性：方案各步骤间是否矛盾
  2. 边界遗漏：是否有未考虑的边界条件
  3. 定义忠实度：方案是否忠实于缠论定义（如涉及）
  4. 实现可行性：是否有技术障碍未识别
  5. 过度工程：是否有不必要的复杂度

### Phase 3: 严格对审直到达成共识

- 每轮：Codex 提出质疑 -> Opus 回应并修改方案 -> Codex 再评审
- 终止条件：**Opus 和 Codex 达成共识**——Codex 对方案的所有质疑都被 Opus 回应且 Codex 明确确认满意（沉默不等于同意——必须有显式确认）
- 如果 6 轮仍未达成共识：**不终止，走 `/escalate`**——说明双方分歧点，由编排者决断
- 每轮结果自动写入 `.chanlun/review-results/plan-review-{timestamp}-round{N}.md`

### Phase 4: 方案定稿

- 最终方案 = 经过多轮对审的修正版
- 输出标签：`[plan-reviewed]` 标记方案已通过多模型对审

## 关键约束

- **Plan 对审是忠实谱系、实现谱系的重要节点——不允许在未达成共识时强制通过**
- **无需编排者审批**：Opus 和 Codex 之间的多轮对审完全自动化，编排者事后看结果（158号第三步：取消编排者作为最终决断者的位置）
- **达成共识的定义**：Codex 对方案的所有质疑都被 Opus 回应，且 Codex 明确确认满意——"无新质疑"不等于同意，必须有显式确认
- 修改循环在纯文本阶段完成——不写代码
- 严格对审直到达成共识，执行阶段几乎一次通过
- 对审结果持久化到 `.chanlun/review-results/`
- 6 轮未达成共识时走 `/escalate`（上浮双方分歧点），不强制终止

## 与 codex-challenger 的关系

本 skill 定义 Plan 阶段的多模型对审流程。实际的 Codex 调用通过 codex-challenger agent 执行（review 模式，subject 为方案文本）。
codex-challenger 的 `plan_review` 触发事件对应本 skill 的 Phase 2。

## 边界条件

- 如果 Codex API 不可用，方案退化为单模型产出（标记 `[plan-unreviewed]`）
- 纯技术性变更（格式调整、依赖升级等不涉及架构决策的操作）可跳过多轮对审
- 6 轮未达成共识时上浮（`/escalate`），不强制终止——防止无限循环的同时保证共识质量
