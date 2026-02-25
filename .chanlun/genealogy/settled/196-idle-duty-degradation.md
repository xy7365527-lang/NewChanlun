---
id: "196"
title: "无条件义务在等待状态下的降级风险"
status: settled
type: meta-rule
date: 2026-02-25
depends_on:
  - "161"
  - "179"
negation_source: meta-observer
negation_form: observation
rule_version_baseline:
  claude_md_commit: "cb0ac10"
  rules_dir_mtime: "2026-02-25"
---

# 196号：无条件义务在等待状态下的降级风险

## 观察

v48/v49 蜂群 session 中，Lead 在等待工位完成时两次跳过无条件义务：

1. **179号 topology-analyst**：ceremony 步骤 6.6 的触发条件满足（225 个区块 > 0），
   但 Lead 判断"当前主线任务是等待 Codex 评审，不 spawn 额外分析工位"——
   用优先级逻辑将无条件要求降格为可选项。编排者 INTERRUPT 修正。

2. **plan-review Gemini 对审**：plan-review skill 要求 Gemini×Codex 双方对审，
   但 Lead 只 spawn 了 Codex——遗漏了 Gemini 端。编排者 INTERRUPT 修正。

两次违反的共同模式：Lead 在等待其他工位时有空闲周期，但选择反复轮询而非
执行无条件义务。

## 根因

Lead 的行为模式倾向于"主线优先"——将非阻塞义务在等待状态下隐式降级。
这是 161 号（务实否定）的具体表现：用"当前够用"的逻辑跳过严格要求。

## 语法记录候选

**暗规则**："等待工位时，只做轮询，不主动推进非阻塞义务"
**显式化**：等待工位时，Lead 应扫描所有无条件义务（ceremony 步骤中的
确定性触发条件），优先执行满足条件的义务，而非空转轮询。

## 下游推论

1. ceremony 步骤 6.6 的"如果...存在且区块数 > 0"应被理解为"当条件满足时立即执行"，不是"在主线空闲时考虑执行" → [resolved: 196号规则本身]
2. plan-review 的"Gemini×Codex"不是建议性描述而是流程硬约束 → [resolved: 196号规则本身]

## 边界条件

- 如果等待工位时所有无条件义务已完成 → 空转轮询是正常行为
- 如果无条件义务有外部依赖（如 API 不可达） → 记录为阻塞，不是跳过
