---
id: "014"
title: "SKILL.md 指令卡组拆分"
status: "已结算"
type: "元编排进化"
date: "2026-02-17"
depends_on: []
related: ["010", "012", "013"]
negated_by: []
negates: []
---

# 014 — SKILL.md 指令卡组拆分

- **状态**: 已结算（定理：架构前提变更的逻辑必然推论）
- **类型**: 元编排进化
- **日期**: 2026-02-17

## 矛盾描述

SKILL.md（633行）违反了自身教导的设计原则：
- v3.3 核心原则 = "一个 agent 一件事，复杂度通过 agent 数量而非单个 agent 质量扩展"
- SKILL.md 实际行为 = 单文件塞进所有工位指令，每个 agent 加载全部内容
- 一个做K线解析的 teammate 不需要知道"原文考古原则"和"元编排自身的进化"

SKILL.md 同时扮演两个不兼容角色：
1. 给 Lead 看的总纲（需要全部章节）
2. 给每个 teammate 看的工位指令（只需要禁令+结果包+质询序列）

## 推导链

1. **前提变更**：Agent Teams 提供共享文件系统 + SendMessage + rules/skills 继承
2. **推论**：全局视野不再是某个 agent 的特权，而是文件系统提供的公共基础设施
3. **推论**：胖中枢（Lead 承担 8 项实质性职责）在 Agent Teams 下是反模式——它把本可以并行的工作串行化
4. **推论**：SKILL.md 作为单一加载单元，与 Agent Teams 的分布式架构矛盾
5. **结论**：SKILL.md 必须拆分为指令卡组，每个 agent 只加载相关的卡片

这是定理（架构前提变更 → 架构决策必须变更），不是选择。

## 解决方案

### Lead 退化为薄路由层

| 原 Lead 职责 | 下放给 | Lead 只剩 |
|-------------|--------|----------|
| 谱系比对 | 谱系维护工位 | 收到工位结论，转发 |
| 上浮判断 | 各工位自带上浮条件 | 收到上浮请求，转给编排者 |
| 仪式执行 | 仪式由编排者触发 | 收到编排者决断，转给相关工位 |
| 冷/热启动 | ceremony 自身逻辑 | 被 ceremony 唤起 |
| Session 摘要 | 各工位自写退出状态 | 汇总（机械操作） |
| 扩张收缩 | 拓扑工位（新增） | 收到建议，转给编排者确认 |
| 结构工位汇总 | 各工位直接报告 | 无 |
| 轴线汇报 | Lead 唯一保留的实质职责 | 向编排者报告当前状态 |

### 指令卡组架构

| 文件 | 加载者 | 行数目标 | 实现方式 |
|------|--------|---------|----------|
| `SKILL.md`（核心卡） | 所有 agent | ~100 | 保留为 skill |
| `meta-lead.md` | Lead | ~50 | `.claude/agents/` |
| `source-auditor.md` | 源头审计工位 | ~80 | `.claude/agents/` |
| `genealogist.md` | 谱系维护工位 | ~60 | `.claude/agents/` |
| `meta-observer.md` | 元规则观测工位 | ~50 | `.claude/agents/` |
| `quality-guard.md` | 质量守卫工位 | ~40 | `.claude/agents/` |
| `topology-manager.md` | 拓扑管理工位 | ~70 | `.claude/agents/` |
| `methodology-v3.3.md` | 参考文档 | 633+ | `references/` |

### 附带决断：放弃 subagent 模式兼容

subagent 要求胖中枢+单文件，Agent Teams 要求薄路由+分布式指令。两种模式在同一文件中不可兼容。

## 谱系链接

- 前置：013（蜂群工位二分法）— 结构工位/任务工位的区分是卡组拆分的概念基础
- 前置：012（谱系是发现引擎）— 谱系工位需要独立指令卡
- 关联：010（构造层 vs 分类层）— 分层思想在此应用于元编排自身

## 影响

- `.claude/skills/meta-orchestration/SKILL.md` → 精简为核心卡（~100行）
- `.claude/agents/` → 新增 6 个结构工位 agent 定义
- `.claude/skills/meta-orchestration/references/` → 新增完整方法论归档
- `CLAUDE.md` → 更新元编排章节
- Serena 记忆 → 更新 meta-orchestration-rules
- **不可逆**：放弃 subagent 模式兼容

## 来源

- [新缠论] — 编排者在对比两个 v3.3 版本后的架构批判
- 两个版本的好部分在拆分中保留（见矛盾描述中的清单）
