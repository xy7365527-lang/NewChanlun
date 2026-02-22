---
id: '096'
title: 蜂群规则的分布式存在形式——完全禁用孤立 subagent，无例外
type: 选择
status: 已结算
date: 2026-02-22
depends_on:
  - '089'   # 元编排扬弃——CLAUDE.md 是基因组
  - '090'   # 严格性是蜂群语法规则
  - '093'   # 五约束有向依赖图
  - '095'   # Agent Team 真递归——唯一例外 Explore
negation_source: 编排者 INTERRUPT + Gemini decide（异质决断）
negation_form: expansion（095号"唯一例外"条款被否定——架构纯粹性优先于实用性）
negates: []
negated_by: []
---

# 096号：蜂群规则的分布式存在形式——完全禁用孤立 subagent，无例外

**类型**: 选择
**状态**: 已结算
**日期**: 2026-02-22
**前置**: 089-meta-orchestration-aufhebung-audit, 090-strictness-grammar-rule, 093-five-constraints-topology, 095-agent-team-recursive-swarm
**negation_source**: 编排者 INTERRUPT("写在 prompt 里就不是分布式的"+"不要有例外") + Gemini decide（选项 D）

## 编排者约束（触发本号的 INTERRUPT）

1. "如果你写在 prompt 里，那么它就不是分布式的，也就无法变成真严格递归拓扑异步自指蜂群"
2. "不要有例外"——subagent 架构与 Agent Team 架构本质不同，不允许在规则中为 subagent 留出例外通道

## 决断结论

**选项 D（完全禁用 Task(Explore)，搜索改用 Glob/Grep/Read 直接工具或 team 内 spawn 搜索 teammate）**

### 被排除的选项

- **B**：编排者直接排除（Explore 例外不允许存在）
- **A**：Explore 走完整 team 流程——这仍然是 Task(Explore)，只是加了 team_name。但根本问题不是 team_name，而是 Explore 这种孤立 subagent 架构与 Agent Team 架构的本质差异。
- **C**：封装快捷模式——补丁思维（090号），用复杂度掩盖架构问题。

## Gemini 推理链（摘要）

1. **概念优先于代码**：Agent Team 架构（teammate 拓扑）与 Subagent 架构（孤立 Task 调用）是两种本质不同的结构。编排者已明确"subagent 的架构跟 agent team 完全不同"。

2. **语法规则的绝对性**：在 `agent-team-enforce.sh` 中为 Explore 留下例外，意味着"必须使用 Team"从语法规则降级为软性建议。例外是系统拓扑腐化的开始（与 090号同构）。

3. **能力替代的完备性**：Explore 的"多轮自主搜索"能力：
   - 若需要多轮自主 → 应是 team 内专职 Teammate 的职责（是真正的子任务）
   - 若只需单次搜索 → Glob/Grep/Read 足够

4. **倒逼拓扑纯粹**：禁用 Task(Explore) 彻底切断退回旧 Subagent 架构的退路，强制在面对复杂搜索需求时使用正确蜂群拓扑。

## 关键分析

### CLAUDE.md 作为基因组是否已解决分布式规则传递？

**是。**

089号谱系（已结算）明确：CLAUDE.md 是蜂群基因组，平台自动加载到每个 agent（包括 teammate）。
因此：
- 规则的分布式存在形式 = CLAUDE.md（基因组）+ hooks（免疫系统）+ skills（可执行知识结晶）
- 三层各司其职：CLAUDE.md 声明原则、hooks 强制语法、skills 提供可执行流程
- team-structural-inject.sh 的规则注入部分是**冗余的中心化注入**，违反分布式原则
- 应修正为 skill 索引器（告知可用的 skill 及其路径），不含规则传递也不是信息提示
- 原则11推论："元编排本身也是 skill 的集合——编排能力分布式地结晶在 skill 中"

### hooks 是否已解决强制层？

**是。**

agent-team-enforce.sh 对所有 agent 的 Task 调用自动触发（PreToolUse），不依赖 prompt。这是真正的分布式强制——每层递归都受同样的强制约束。

### "不要有例外"的严格含义

与 005b号（对象否定对象是语法规则）、090号（严格性是蜂群语法规则）完全同构：
- 例外 = 在语法规则内留下语法漏洞
- "所有 Task 调用必须携带 team_name" + Explore 例外 = "大多数 Task 调用必须携带 team_name"
- "大多数"不是语法规则，是软性建议

## 需要执行的修改

### 1. agent-team-enforce.sh
删除 Explore 例外：
```bash
# 删除以下代码段：
# 例外：Explore 类型（只读搜索，不是蜂群节点）
if subagent_type == 'Explore':
    sys.exit(0)
```
修改 block 原因说明：移除 Explore 例外的描述。

### 2. team-structural-inject.sh
修正为 skill 索引器：hook 语义从"信息性提示"提升为"强制导向 sub-swarm-ceremony skill"。
规则的三层存在形式：CLAUDE.md（原则）+ hooks（语法守卫/Skill索引器）+ skills（操作流程结晶）。
hook 输出必须明确指向 .claude/skills/sub-swarm-ceremony/SKILL.md，强制 agent 读取。
（096号修正——Gemini decide，选项C，2026-02-22）

### 3. CLAUDE.md 原则15
删除"唯一例外：Explore 类型 agent（只读搜索，不是蜂群节点）"。

### 4. 使用模式更新
凡需要文件搜索：
- 简单搜索 → Glob/Grep/Read 直接工具
- 多轮自主搜索 → 在当前 team 内 spawn 搜索 teammate（这本来就是子任务，属于 team 范畴）

## 边界条件

Gemini 识别的翻转条件：如果 Glob/Grep/Read 工具在实际运行中因上下文窗口限制或缺乏基础的关联跳转能力，导致无法完成最基础的复合搜索；同时 Spawn Teammate 的初始化开销导致高频轻量级搜索场景出现严重性能问题——此时应推翻此决策，考虑在平台底层引入轻量级只读游走机制（而非 prompt 或 hook 例外）。

## 下游推论

1. agent-team-enforce.sh 无例外——hook 变成无条件的语法守卫
2. 规则分布式存在形式确认：CLAUDE.md（基因组）+ hooks（免疫系统）+ skills（可执行知识结晶），不在 prompt 中
3. team-structural-inject.sh 从"信息性提示"修正为"skill 索引器"——告知可用 skill 及路径（编排者 INTERRUPT 修正）
4. 蜂群中不再有"孤立 subagent"的概念——Task 调用语义统一为"spawn teammate into team"

## 推导链

1. 095号确立 Agent Team 真递归为默认模式，保留 Explore 例外
2. 编排者 INTERRUPT："写在 prompt 里就不是分布式的" + "不要有例外"
3. 分析 CLAUDE.md（089号基因组）已实现规则的分布式传递
4. 分析 hooks 已实现分布式强制
5. Gemini decide → 选项 D：完全禁用 Task(Explore)
6. 异质质询验证：定义回溯✅ + 反例构造✅ + 推论检验✅
7. ∴ 096号结算：无例外，规则分布式存在于 CLAUDE.md + hooks + skills
8. 编排者 INTERRUPT 修正："蜂群存在的方式是 skill"——096号补充 skills 层为第三维度

## 谱系链接

- **095号**（Agent Team 真递归）→ 096号否定了 095号的"唯一例外：Explore"条款
- **089号**（基因组扬弃）→ 确认 CLAUDE.md 作为分布式规则存在形式的充分性
- **090号**（严格性是语法规则）→ "不要有例外"的理论依据
- **016号**（运行时强制层）→ hooks 作为分布式强制的实现机制

## 影响声明

修改：
- `.claude/hooks/agent-team-enforce.sh`：删除 Explore 例外
- `.claude/hooks/team-structural-inject.sh`：修正为 skill 索引器（强制导向 sub-swarm-ceremony skill，096号修正）
- `.chanlun/dispatch-dag.yaml` knowledge_templates：注册 sub-swarm-ceremony skill（096号修正）
- `CLAUDE.md` 原则15：删除"唯一例外：Explore"
- 095号谱系：标注 negated_by: 096（Explore 例外条款）
