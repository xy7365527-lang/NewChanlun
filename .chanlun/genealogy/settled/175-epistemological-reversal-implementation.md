---
id: "175"
title: "认识论反转落地——谱系驱动架构 + Codex作为谱系异质否定者"
type: 语法记录
status: 已结算
date: 2026-02-23
depends_on: ["174"]
negates: []
negated_by: []
---

# 175号：认识论反转落地——谱系驱动架构 + Codex作为谱系异质否定者

**id**: 175
**status**: 已结算
**type**: 语法记录
**date**: 2026-02-23
**前置**: 174（谱系即生成引擎）

## 来源

174号声明了认识论反转："不是谱系服务于RTAS，而是RTAS服务于谱系"。编排者追问"你实现了吗？包括codex的位置"。本条目记录认识论反转在代码/架构中的落地实现。

## 内容

### 一、ceremony.md：从过程式改为谱系驱动式

174号之前，ceremony.md 的描述是过程式的（步骤1→2→3→4→5→6），隐含假设是"RTAS 流程驱动蜂群"。

174号之后，ceremony.md 显式声明：
- **谱系决定做什么，RTAS决定怎么做**——这是 ceremony 的设计原则
- 步骤4 从"遍历 workstations"改为"谱系推导出的业务工位"——工位的来源是谱系状态，不是过程步骤
- 步骤6 从"汇报循环"改为"RTAS 递归——谱系的实现"——递归循环服务于谱系的生成性
- 谱系引用增加 174号和 175号

变更文件：`.claude/commands/ceremony.md`

### 二、Codex 的位置：从代码层异质否定扩展为谱系异质否定

174号确立"谱系是生成引擎"后，谱系质量直接决定蜂群行动质量。因此谱系需要与代码同等的异质否定。

Codex 从"代码层异质否定源"扩展为"异质否定源（代码层 + 谱系层）"：
- 新增第四种模式 `genealogy-review`：审查谱系推导链自洽性、概念分离完整性、声明-能力一致性、边界条件充分性
- 新增触发条件：`genealogy_settlement` 事件——新 settled 条目写入时自动触发
- 本体论位置更新：155号（代码层）+ 175号（谱系层）

变更文件：`.claude/agents/codex-challenger.md`

### 三、dispatch-dag.yaml 更新

1. `codex-challenger` 的 `triggers` 增加 `genealogy_settlement` 事件
2. `skill_flow_edges` 增加 `genealogist → codex-challenger` 边（type: genealogy_settlement）
3. `event_edges` 增加 `genealogy_settlement → codex-challenger` 边（action: genealogy_review）
4. `genealogical_coordinates` 中 `codex-challenger` 的 origin 和 domain 更新
5. 顶层 `genealogy_ref` 增加 175号

这实现了"谱系结晶 → 异质否定 → 可能的新矛盾 → 新谱系"的递归循环——谱系的生成性通过异质否定获得质量保证。

变更文件：`.chanlun/dispatch-dag.yaml`

## 边界条件

- genealogy-review 模式依赖 Codex API 可用性——不可用时退化为无异质审查（与代码审查的退化策略一致）
- 谱系异质审查不替代 Gemini 的概念层质询——Codex 审查谱系的逻辑自洽性，Gemini 质询概念的哲学/数学维度
- 触发频率：每次 genealogy_settlement 都触发，但 priority 为 medium（低于 topo_effect 的 high）

## 下游推论

1. 当 Codex genealogy-review 发现谱系缺陷时，产出应走 `/escalate` 路径（定义冲突）或直接修复（逻辑错误）
2. review 结果持久化到 `.chanlun/review-results/codex-genealogy-review-{timestamp}.md`

## 谱系依据

- 174号：谱系即生成引擎——认识论反转的理论声明
- 155号：Codex 异质代码审查——175号将其扩展到谱系层
- 075号：结构能力 = skill（事件驱动）
- 089号：元编排的严格扬弃

## 影响声明

- 新增谱系 175号
- 修改 `.claude/commands/ceremony.md`：谱系驱动措辞 + 174/175号谱系引用
- 修改 `.claude/agents/codex-challenger.md`：新增 genealogy-review 模式 + genealogy_settlement 触发
- 修改 `.chanlun/dispatch-dag.yaml`：codex-challenger triggers/edges/coordinates 更新
