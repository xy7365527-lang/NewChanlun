---
id: '138'
title: 强制输出格式——137号修复执行（否定性禁令→结构化输出模板）
type: 定理
status: 已结算
date: 2026-02-22
depends_on:
  - '137'   # Lead停顿RLHF结构根因 + 基因组表达失败
  - '090'   # 严格性永久化为蜂群语法规则
related:
  - '058'   # ceremony 是 Swarm₀
  - '018'   # 四分法分类
tensions_with: []
---

## 事件

137号结算后的直接推论执行：将 no-unnecessary-escalation.md 的"自检序列"（否定性禁令形式，实证无效）替换为"强制输出格式"（三种合法结尾格式 A/B/C）。

## 推导链

1. 137号结算：否定性禁令对行为执行层无效（RLHF基底约束）
2. 137号建议修复方向：将行为规则转化为强制输出格式模板
3. post-commit-flow.md 已有格式A雏形（`→ 接下来：[行动]` + 工具调用）
4. 推论：泛化为所有 phase 结束点的强制格式 → 定理，直接执行

## 具体变更

### no-unnecessary-escalation.md
- 删除"自检序列"节（依赖模型内心独白的否定性禁令）
- 新增"强制输出格式"节：三种合法结尾格式
  - 格式A：`→ 接下来：[行动]` + 工具调用（有后续行动时）
  - 格式B：状态快照（所有工位完成时）
  - 格式C：`/escalate`（真实矛盾时）
- 新增"禁止的结尾模式"：以问号/总结/等待信号/未分类选项列表结尾
- 新增谱系依据节

### post-commit-flow.md
- 简化为引用 no-unnecessary-escalation.md 的强制输出格式
- 标注为137号强制输出格式在 commit/push 场景的特化

### core-principles/SKILL.md
- 原则7更新：合并 ceremony 是 Swarm₀ + 强制输出格式（137号）

## 边界条件

- 格式约束仅适用于行为执行层（"做什么"）；认知层规则（"怎么分类"）仍以自然语言形式有效
- 如果 Claude 平台未来支持硬性输出格式验证（如 structured output），格式约束可进一步从上下文层提升到平台层

## 下游推论

- 所有以否定性禁令形式存在的行为规则都是137号修复的候选对象
- 当前修复只覆盖了 no-unnecessary-escalation.md；其他行为规则（如 no-patch-mentality.md 中的行为部分）如果也反复违反，应做同样的格式转化

## 影响声明

- 修改 `.claude/rules/no-unnecessary-escalation.md`：自检序列→强制输出格式
- 修改 `.claude/rules/post-commit-flow.md`：简化为引用
- 修改 `.claude/skills/core-principles/SKILL.md`：原则7更新
