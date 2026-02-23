# 160号谱系：Plan 阶段多模型对审——修改循环前移

**id**: 160
**status**: 已结算
**type**: 语法记录
**date**: 2026-02-23
**前置**: 159-codex-discussion-persistence, 155-codex-heterogeneous-code-review

## 来源

编排者指令原文："标准流程应该是多模型对审，Plan阶段填坑。Opus4.6 plan mode出方案，Codex5.3 High评审。每个需求点五六轮质疑补全，执行几乎一次通过。修改循环前移到纯文本阶段，最划算。"

追加方针："在我们的真RTAS里这个plan不需要我来同意，让codex进行双向严格讨论。"

## 内容

核心原则：**修改循环前移到纯文本阶段最划算**。

蜂群标准工作流从"写代码后审查"修正为"Plan阶段多模型对审"：

1. **Plan 阶段**：Opus 4.6 plan mode 出方案
2. **Codex 评审**：Codex 5.3 High 评审方案（review 模式）
3. **多轮质疑补全**：5-6轮 Codex 质疑 + Opus 回应修改
4. **方案定稿**：经多轮对审打磨的方案，标记 `[plan-reviewed]`
5. **执行**：基于定稿方案写代码，几乎一次通过

### 否定关系

修正 159号的 `file_write` 触发方向。不是否定 159号的持久化机制（结果持久化保留），而是修正触发时机：从"代码写入后审查"修正为"Plan 阶段多模型对审"。

159号的实现（`ReviewResult.to_markdown()`、CLI 自动持久化）继续有效——对审结果的持久化载体不变，变的是触发时机。

## 实现

- `dispatch-dag.yaml`：codex-challenger 的 `file_write` 触发替换为 `plan_review` 触发
- `.claude/skills/plan-review/SKILL.md`：新建 Plan 对审 skill
- `.claude/agents/codex-challenger.md`：触发条件表格和标配审查模式章节更新
- 159号谱系：触发方向修正备注

## 边界条件

- 如果 Codex API 不可用，方案退化为单模型产出
- 纯技术性变更（格式调整、依赖升级）可跳过多轮对审
- 对审轮次硬上限 6 轮

## 下游推论

1. planner agent 工作流增加 Codex 评审循环
2. `plan-review` skill 注册到 dispatch-dag（event_skill_map 的 `plan_review` 事件）
3. 蜂群标准工作流变为：Plan（多模型对审）-> Execute（一次通过）-> Verify
4. `.chanlun/review-results/` 同时承载 Plan 对审结果和代码审查结果

## 谱系依据

- 159号：Codex 持久化调用（持久化机制保留，触发时机修正）
- 155号：Codex 代码层异质审查（Codex challenger 的引入）
- 062号：异质性作为结构性要素

## 影响声明

- 修改 `.chanlun/dispatch-dag.yaml`：codex-challenger 触发事件从 `file_write` 修正为 `plan_review`
- 新增 `.claude/skills/plan-review/SKILL.md`：Plan 对审 skill
- 修改 `.claude/agents/codex-challenger.md`：触发条件 + 新增多模型对审模式章节
- 修改 `.chanlun/genealogy/settled/159-codex-discussion-persistence.md`：增加 160号修正备注
- 新增 160号谱系
