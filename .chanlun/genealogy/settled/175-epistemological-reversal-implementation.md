---
id: "175"
title: "认识论反转落地——谱系驱动架构 + Gemini作为谱系概念层异质否定者"
type: 语法记录
status: 已结算
date: 2026-02-23
depends_on: ["174"]
negates: []
negated_by: []
---

# 175号：认识论反转落地——谱系驱动架构 + Gemini作为谱系概念层异质否定者

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

### 二、Gemini 的位置：概念层异质否定扩展为谱系概念层异质否定

174号确立"谱系是生成引擎"后，谱系质量直接决定蜂群行动质量。因此谱系需要概念层的异质否定。

编排者明确：概念层异质否定 = Gemini，Codex 只负责代码层。

Gemini 从"概念层异质否定源"扩展为"概念层异质否定源（含谱系质询）"：
- 新增触发条件：`genealogy_settlement` 事件——新 settled 条目写入时自动触发 Gemini 概念层质询
- 审查维度：谱系推导链自洽性、概念分离完整性、声明-能力一致性、边界条件充分性
- Codex 保持代码层异质否定（155号），不参与谱系结算审查

变更文件：`.chanlun/dispatch-dag.yaml`（gemini-challenger triggers/edges/coordinates 更新，codex-challenger 移除 genealogy_settlement）

### 三、dispatch-dag.yaml 更新

1. `gemini-challenger` 的 `triggers` 增加 `genealogy_settlement` 事件
2. `skill_flow_edges` 增加 `genealogist → gemini-challenger` 边（type: genealogy_settlement）
3. `event_edges` 增加 `genealogy_settlement → gemini-challenger` 边（action: genealogy_conceptual_review）
4. `genealogical_coordinates` 中 `gemini-challenger` 的 origin 和 domain 更新（增加175号）
5. `codex-challenger` 移除 `genealogy_settlement` 触发——Codex 只负责代码层（155号）
6. 顶层 `genealogy_ref` 增加 175号

这实现了"谱系结晶 → 概念层异质否定 → 可能的新矛盾 → 新谱系"的递归循环——谱系的生成性通过 Gemini 概念层异质否定获得质量保证。

变更文件：`.chanlun/dispatch-dag.yaml`

## 边界条件

- genealogy 概念层质询依赖 Gemini API 可用性——不可用时退化为无概念层异质审查（与其他 Gemini 质询的退化策略一致）
- 谱系概念层异质审查由 Gemini 负责，Codex 负责代码层异质审查——两者分工明确
- 触发频率：每次 genealogy_settlement 都触发，但 priority 为 medium（低于 topo_effect 的 high）

## 下游推论

1. 当 Gemini 概念层谱系质询发现谱系缺陷时，产出应走 `/escalate` 路径（定义冲突）或直接修复（逻辑错误）
2. review 结果持久化到 `.chanlun/review-results/gemini-genealogy-review-{timestamp}.md`

## 谱系依据

- 174号：谱系即生成引擎——认识论反转的理论声明
- 155号：Codex 异质代码审查——保持代码层职责
- 075号：结构能力 = skill（事件驱动）
- 089号：元编排的严格扬弃

## 影响声明

- 新增谱系 175号
- 修改 `.claude/commands/ceremony.md`：谱系驱动措辞 + 174/175号谱系引用
- 修改 `.chanlun/dispatch-dag.yaml`：gemini-challenger triggers/edges/coordinates 更新（增加 genealogy_settlement），codex-challenger 移除 genealogy_settlement
