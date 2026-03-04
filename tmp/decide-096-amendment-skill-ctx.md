# Gemini Decide 上下文：096号修正——蜂群规则的存在方式是 skill

## 决断类型：语法记录（编排者指出了已在运作但未显式化的规则）

## 背景

096号确立"蜂群规则的分布式存在形式 = CLAUDE.md + hooks"，将 team-structural-inject.sh 降为"信息性提示"。

编排者 INTERRUPT：
- "我的意思是不应该结晶成 skill 吗？怎么能是信息提示呢？"
- "蜂群存在的方式不是 skill 吗？"

## 分析

CLAUDE.md 原则11 已经定义了知识结晶的三个维度：session（时间）、skill（知识）、definitions（概念）。

蜂群的递归规则的存在应该是：
1. **CLAUDE.md**（基因组——声明层，所有 agent 自动拥有）
2. **hooks**（免疫系统——强制层，自动触发）
3. **skills**（可执行知识结晶——知识层，按需加载）

`sub-swarm-ceremony` skill 已经存在于 `.claude/skills/sub-swarm-ceremony/SKILL.md`。这个 skill 定义了子蜂群的完整创建流程和四特征。

问题在于：
- 096号把 team-structural-inject.sh 降为"信息提示"——这是不严格的
- 正确做法：team-structural-inject.sh 应该是 skill 触发点（提示 agent 有 sub-swarm-ceremony 可用），而不是"信息提示"
- 或者更根本地：skill 已经在 dispatch-dag 的 event_skill_map 中注册为 structural skill，事件触发时自动激活

## 选项

### A：team-structural-inject.sh 从"信息提示"修正为"skill 触发提示"
- 输出中明确告知 agent："sub-swarm-ceremony skill 可用，创建子蜂群时应使用此 skill"
- skill 本身包含完整的子蜂群创建规则
- 这是 075号事件驱动 skill 架构的正确执行

### B：删除 team-structural-inject.sh，完全依赖 skill 自动触发
- sub-swarm-ceremony 已在 dispatch-dag event_skill_map 中注册
- 理论上 agent 应该自主发现并使用 skill
- 但实际上 agent 可能不会主动搜索 skill

### C：三层存在形式显式化——修正 096号谱系
- 096号的"规则分布式存在 = CLAUDE.md + hooks"改为"CLAUDE.md + hooks + skills"
- 明确：CLAUDE.md 声明原则、hooks 强制语法、skills 提供可执行流程
- team-structural-inject.sh 继续作为 skill 提示器（不是规则注入，不是信息提示，而是 skill 索引）

## 请 Gemini 决断

这是一个语法记录类决断——编排者辨认出了"skill 是蜂群规则的存在方式"这条已在运作的规则。
