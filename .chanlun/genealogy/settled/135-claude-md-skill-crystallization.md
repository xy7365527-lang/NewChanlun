---
id: '135'
title: CLAUDE.md 单体→skill 集合结晶 + 073b平台约束前提否定
type: 行动
status: 已结算
date: 2026-02-22
depends_on:
  - '089'   # 存在论位置（扬弃）
  - '095'   # 真递归是默认模式
  - '096'   # 统一约束
  - '097'   # 五特征 DAG 模板
  - '134'   # v37审计 P0修复（P0-5 = 本号执行对象）
related:
  - '014'   # 分布式指令架构
  - '073b'  # 被否定的 Trampoline 前提
  - '075'   # 结构能力→skill 事件驱动
  - '105'   # 递归无条件架构要求
  - '117'   # 三框架组合理解
negation_source: v38-swarm RTAS执行 + 编排者确认平台无限制
negation_form: 行动（P0-5执行 + 073b前提否定，不携带信息差）
negates:
  - '073b'  # 编排者确认：Agent Teams 平台支持子蜂群递归，073b "subagent不能spawn sub-subagent" 前提为假
negated_by: []
provenance: "[新缠论]"
---

# 135号：CLAUDE.md 单体→skill 集合结晶 + 073b平台约束前提否定

**类型**: 行动（l'acte）
**状态**: 已结算
**日期**: 2026-02-22

## 两个事件

### 事件A：CLAUDE.md 拆分（P0-5 执行）

CLAUDE.md 从 259 行单体文件→~80 行最小引导文件。
原则11推论的实现："元编排本身也是 skill 的集合"。

**新增 5 个 skill**：

| Skill | 内容 | 行数 |
|-------|------|------|
| core-principles | 原则0-7：蜂群基础语法 | 19 |
| domain-conventions | 检索/级别/溯源/输出风格 | 34 |
| domain-principles | 原则8/13/14：缠论域语法 | 14 |
| swarm-architecture | 原则9-12/15-17 + 五约束 + 递归行为 + 热启动 | 122 |
| project-topology | 知识仓库/谱系目录/指令架构 | 35 |

CLAUDE.md 保留：存在论位置 + Language + 资料入口 + 三级权威链 + skill索引表 + 可用命令。

规则的三层分布式存在形式（096号）现在完全实现：
- CLAUDE.md = 基因组声明层（含 skill 路径索引）
- hooks = 免疫系统强制层
- skills = 可执行知识结晶知识层

### 事件B：073b 平台约束前提否定

编排者明确确认："我们用的就是 agent team，平台根本没有这个限制。"
并补充："这就是为什么我直接取消了 subagent 的接口。"

073b 号谱系的核心前提——"Claude Code 平台：subagent 不能 spawn sub-subagent"——**被事实否定**。

**影响**：
- 073b Trampoline 模式不是"平台约束下的退化"，而是**早期误判**
- 095号"真递归是默认模式"的前提更加坚实：不仅是存在论要求，平台也完全支持
- dispatch-dag.yaml recursion_rules 中残留的 Trampoline 表述已修正

## 修改清单

| 文件 | 改动 |
|------|------|
| CLAUDE.md | 259行→~80行，保留引导 + skill索引 |
| .claude/skills/core-principles/SKILL.md | 新建 |
| .claude/skills/domain-conventions/SKILL.md | 新建 |
| .claude/skills/domain-principles/SKILL.md | 新建 |
| .claude/skills/swarm-architecture/SKILL.md | 新建 |
| .claude/skills/project-topology/SKILL.md | 新建 |
| .chanlun/dispatch-dag.yaml | recursion_rules 修正（Trampoline→真递归） |

## 边界条件

- 如果 skill 加载机制失败（Claude Code 平台变更）→ skill 内容需回迁 CLAUDE.md
- 如果新 skill 之间出现内容重叠或矛盾 → 需要重新划分边界

## 下游推论

1. 蜂群的每个新会话不再加载 259 行基因组，而是按需加载相关 skill
2. 073b 否定后，所有引用 "Trampoline" 或 "平台限制" 的文档需清理
3. 子蜂群递归的实现障碍从"平台不支持"变为"执行模式选择"——没有技术障碍
