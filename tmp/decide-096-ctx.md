# Gemini Decide 上下文：096号——蜂群规则的分布式存在形式

## 决断类型：选择

## 编排者约束（最高优先级）

1. "如果你写在 prompt 里，那么它就不是分布式的，也就无法变成真严格递归拓扑异步自指蜂群"
2. "不要有例外"：subagent 的架构跟 agent team 完全不同
3. B 选项已被编排者明确排除

## 已有的平台分布式机制（关键背景）

Claude Code 平台已经提供了分布式规则传递：
- **CLAUDE.md**：每个 agent（包括 teammate）启动时平台自动加载——这是 089号识别的"基因组"（来源外部，内化后执行内在）
- **hooks**：PreToolUse/PostToolUse/Stop 对所有 agent 自动触发，不依赖 prompt
- **skills**：通过 Skill 工具按需加载

因此：
- CLAUDE.md 中写的规则，递归每一层的 agent 都自动拥有（无需 prompt 注入）
- hooks 中写的强制逻辑，递归每一层都自动执行（无需 prompt 注入）
- **team-structural-inject.sh 是中心化注入的补充层，问题是它是否冗余或是否矛盾分布式原则**

## 当前实现状态

三个相关的 hook/文件：

### agent-team-enforce.sh（PreToolUse on Task）
```
逻辑：
- 检查 Task 调用是否包含 team_name
- 没有 team_name → block
- 例外：subagent_type == "Explore" → 允许
```

### team-structural-inject.sh（PostToolUse on TeamCreate）
```
逻辑：
- TeamCreate 后，向 Lead 的 prompt 注入结构信息
- 提示可用的 structural skill
- 提示 095号真递归架构要求
这是"通过 prompt 注入传递规则"的实例——编排者说这不是分布式的。
```

## 四个选项的精确分析

### A：完全依赖平台分布式机制
- CLAUDE.md + hooks + skills 是规则的存在形式
- 删除 team-structural-inject.sh 的 prompt 注入逻辑（或降为纯信息性提示，不含强制规则）
- agent-team-enforce.sh 无例外（Explore 也必须通过 team）
- Explore 的替代方案：在当前 team 内 spawn 一个 teammate 执行搜索任务

**优点**：真正分布式，规则在文件系统（CLAUDE.md + hooks），不在 prompt
**缺点**：Explore 走完整 team 流程开销大；Explore 作为临时搜索不是蜂群节点——强制 team 化意味着每次读文件前都要 TeamCreate + TeamDelete

### C：封装 Explore 的 team 化快捷模式
- 创建自动化封装（skill 或 hook）：TeamCreate(临时team) → Task(Explore, team) → TeamDelete
- 从外部看没有例外，但实现上有一层包装
**优点**：架构声明纯粹（无例外），实用性保留
**缺点**：实现复杂度高，"团队外观下的个人作业"仍然存在（只是包了一层 team 壳）

### D：完全禁用孤立 subagent，Explore 功能用直接工具替代
- Task(Explore) 完全禁止
- 需要搜索时：用 Glob/Grep/Read 工具在当前 agent 内完成，或在已有 team 中 spawn 搜索 teammate
**优点**：最简单，无例外，无特殊处理，无 team 开销
**缺点**：Explore 能做多轮自主搜索（边搜边判断下一步），Glob/Grep/Read 不能做多轮自主；但真正的多轮自主搜索本来就应该是 team 内的 teammate

## 关键分析维度

### 维度1：CLAUDE.md 作为基因组，是否已经解决分布式规则传递？

089号谱系（已结算）明确：CLAUDE.md 是蜂群的基因组，平台自动加载到每个 agent。这意味着：
- CLAUDE.md 中的"严格使用 Agent Team（095号）"规则，每个子蜂群的 agent 都自动拥有
- 不需要通过 team-structural-inject.sh 的 prompt 注入来传递此规则

结论：**是，CLAUDE.md 已经解决了规则的分布式传递**。team-structural-inject.sh 的规则传递功能是冗余的，且违反了分布式原则。

### 维度2：hooks 是否已经解决强制层？

agent-team-enforce.sh 触发所有 agent 的 Task 调用（包括 teammate），这是真正的分布式强制——每层递归都受同样的强制，不依赖 prompt。

结论：**是，hooks 已经解决了分布式强制**。

### 维度3：Explore 例外是否真的破坏了架构纯粹性？

分析：
- Explore 类型 agent 是"只读搜索工具"，不携带治理责任，不产生持久化产出，不是蜂群节点
- 但编排者说"不要有例外"，且"subagent 的架构跟 agent team 完全不同"
- D 选项完全消除例外：Task(Explore) 禁止，搜索用 Glob/Grep/Read 或在 team 内 spawn

问题：Explore 的"多轮自主搜索"能力是否不可替代？
- 如果需要多轮自主搜索 → 这应该是 team 内的 teammate 职责，不是孤立 subagent
- 如果只需要单次搜索 → Glob/Grep/Read 足够

结论：**Explore 的多轮自主搜索能力，正确的实现是在 team 内 spawn 搜索 teammate，而不是孤立 subagent。**

### 维度4："不要有例外"的严格含义

与 005b 号（对象否定对象是语法规则）、090号（严格性是蜂群语法规则）同构：
- 例外 = 在语法规则内留下语法漏洞
- 任何"例外"都意味着：在某些条件下，规则不成立
- 如果规则是"所有 Task 调用必须携带 team_name"，那么 Explore 例外意味着规则实际上是"大多数 Task 调用必须携带 team_name"

"大多数"不是语法规则，是软性建议。编排者要求的是语法规则。

## 五约束框架分析

| 约束 | A | D |
|------|---|---|
| 1a 物理持久化 | 规则持久化于 CLAUDE.md/hooks（文件系统），不在 prompt | 同 A |
| 1b 符号可解释性 | CLAUDE.md 是可解释的符号系统 | 同 A |
| 2 规则先在性 | CLAUDE.md 在 spawn 前加载 ✅ | 同 A |
| 3 执行不可自观性 | hooks 异步检查（PostToolUse）| 同 A |
| 4 异质验证必要性 | hooks 作为异质强制层 | 同 A，但 Explore 的多轮搜索能力损失 |

## 当前已有的直接工具

Claude Code 提供的直接工具（不需要 Task/subagent）：
- Glob：文件模式匹配
- Grep：内容搜索
- Read：文件读取

这些工具在任何 agent（包括 teammate）内都可用，覆盖 Explore 的基本用途。

## 请 Gemini 决断

在 A 和 D 之间选择：

- **选项 A**：完全依赖平台分布式机制（CLAUDE.md + hooks），Explore 必须走 team 流程（team 内 spawn）
- **选项 D**：完全禁用 Task(Explore)，搜索改用 Glob/Grep/Read，或在已有 team 内 spawn 搜索 teammate

注意 C 选项（封装快捷模式）的问题：封装是补丁思维（090号），它用复杂度掩盖了架构问题，而不是直面架构本质。

关键判断：D 选项是否让蜂群失去不可替代的能力？如果 Glob/Grep/Read + team 内 spawn 能覆盖所有合理用途，则 D 是更严格的选择。
