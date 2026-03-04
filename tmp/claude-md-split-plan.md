# CLAUDE.md → Skill 集合拆分方案

## 分析摘要

**当前 CLAUDE.md**：259 行，包含 17 个内容节。
**现有 skill**：7 个（meta-orchestration, sub-swarm-ceremony, orchestrator-proxy, knowledge-crystallization, spec-execution-gap, math-tools, gemini-math）。
**拆分策略**：按逻辑内聚性将实质内容提取为 5 个新 skill，CLAUDE.md 保留最小引导。

---

## CLAUDE.md 保留内容

拆分后 CLAUDE.md 约 80-100 行，只保留以下内容：

| 保留节 | 原行号 | 保留理由 |
|--------|--------|----------|
| 存在论位置（089号谱系） | L1-12 | 基因组身份声明——蜂群的本体论锚点，每个节点必须在创世时内化 |
| Language | L14-15 | 全局语言指令，1 行 |
| 缠论资料入口 | L17-23 | 路径索引，所有工位的参考入口 |
| 来源权威性（三级权威链） | L25-49 | 参考基准——所有工位判断权威性的依据，不可拆出 |
| 可用命令 | L216-223 | 命令入口索引 |
| **新增：Skill 索引表** | - | 指向所有拆分出的 skill 文件 |

**删除/移入 skill 的节**：检索/引用原则、级别口径、概念溯源标签、输出风格、元编排规则（含核心原则0-17）、五约束有向依赖图、递归节点默认行为、知识仓库映射、谱系目录、分布式指令架构、结构修改模式、热启动机制。

---

## Skill 文件列表

| # | Skill 名称 | 文件路径 | 包含内容（CLAUDE.md 原节标题） | 预估行数 |
|---|-----------|---------|--------------------------|---------|
| 1 | `domain-conventions` | `.claude/skills/domain-conventions/SKILL.md` | 检索/引用原则 + 级别口径 + 概念溯源标签 + 输出风格 | ~80 |
| 2 | `core-principles` | `.claude/skills/core-principles/SKILL.md` | 核心原则 0-7（蜂群基础语法） | ~120 |
| 3 | `domain-principles` | `.claude/skills/domain-principles/SKILL.md` | 核心原则 8, 13, 14（缠论域规则：对象否定对象 + 偏序集 + 合法/非法） | ~60 |
| 4 | `swarm-architecture` | `.claude/skills/swarm-architecture/SKILL.md` | 核心原则 9-12, 15-17 + 五约束有向依赖图 + 递归节点默认行为 + 结构修改模式 + 热启动机制 | ~250 |
| 5 | `project-topology` | `.claude/skills/project-topology/SKILL.md` | 知识仓库映射 + 谱系目录 + 分布式指令架构 | ~60 |

**总计**：~570 行分布在 5 个 skill 中（均在 80-250 行范围，符合 200-400 行目标，最大不超 800 行）。

---

## 现有 Skill 冲突检查

| 现有 Skill | 与新 Skill 的关系 | 处理 |
|------------|------------------|------|
| `meta-orchestration` | 包含质询序列、概念分离等——与 `core-principles` 互补不重叠（原则层 vs 流程层） | 无冲突 |
| `sub-swarm-ceremony` | 原则15的实现细节——`swarm-architecture` 声明原则，sub-swarm-ceremony 提供执行流程 | 无冲突，swarm-architecture 中引用指向 |
| `orchestrator-proxy` | 原则12的实现细节——`swarm-architecture` 声明原则，orchestrator-proxy 提供协议 | 无冲突，swarm-architecture 中引用指向 |
| `knowledge-crystallization` | 原则11的实现细节——`swarm-architecture` 声明原则，knowledge-crystallization 提供流程 | 无冲突，swarm-architecture 中引用指向 |
| `spec-execution-gap` | 独立检测模式 | 无冲突 |
| `math-tools` / `gemini-math` | 领域工具 | 无冲突 |

---

## 每个 Skill 的内容大纲

### Skill 1: `domain-conventions`

**定位**：缠论项目的领域约定集合。任何工位在处理缠论内容时需遵循的约定。

**内容**：
1. **检索/引用原则**（原 L51-56）
   - 不整本引用，优先小文件
   - 先检索关键词再打开文件
   - 权威性遵循三级权威链（引用 CLAUDE.md 保留的权威性节）
2. **级别口径**（原 L58-61）
   - 级别 = 递归层级（level_id）
   - 禁止用时间周期替代级别
3. **概念溯源标签**（原 L63-72）
   - [旧缠论] / [旧缠论:隐含] / [旧缠论:选择] / [新缠论] 四种标签
   - 溯源框架引用
4. **输出风格**（原 L74-79）
   - Insight 格式模板

**Frontmatter**：
```yaml
name: domain-conventions
description: >
  缠论项目领域约定。处理缠论内容（定义、分析、编码）时自动激活。
  包含检索原则、级别口径、溯源标签、输出风格。
```

---

### Skill 2: `core-principles`

**定位**：蜂群的基础语法——所有工位必须遵循的核心原则。这些原则定义了蜂群"是什么"。

**内容**：
1. **原则0：蜂群能修改一切，包括自身**（原 L86）
   - 修改范围、安全保障三机制（git/ESC/产出物验证）
2. **原则1：概念优先于代码**（原 L87）
3. **原则2：不绕过矛盾**（原 L88）— 引用 rules/no-workaround.md
4. **原则3：所有产出必须可质询**（原 L89）— 引用 rules/result-package.md
5. **原则4：谱系必须维护，且谱系优先于汇总**（原 L90）
6. **原则5：定义变更推荐通过仪式**（原 L91）
7. **原则6：推论自动结算（四分法）**（原 L92-97）
   - 定理/选择/语法记录/行动 四种类型及处理方式
8. **原则7：ceremony 是 Swarm0**（原 L98）
   - commit 后行为、递归进入规则

**Frontmatter**：
```yaml
name: core-principles
description: >
  蜂群基础语法（原则0-7）。所有蜂群工位在 ceremony 后自动激活。
  定义蜂群的基本行为规则：修改权、概念优先、矛盾处理、质询、谱系、仪式、四分法、递归。
```

---

### Skill 3: `domain-principles`

**定位**：缠论域特有的原则——将缠论的结构性洞见提升为蜂群语法规则。

**内容**：
1. **原则8：对象否定对象**（原 L99）
   - 否定的唯一来源、语法规则性质
   - 引用 005a/005b 谱系
2. **原则13：缠论空间是偏序集/有向图**（原 L116-118）
   - DAG 建模、术语替换（拓扑手术→图重连）
   - 浮点数阈值非法
   - 引用 068号谱系
3. **原则14：合法/非法替代概率/胜率**（原 L119-121）
   - 走势描述语言无概率概念
   - 引用 067号谱系

**Frontmatter**：
```yaml
name: domain-principles
description: >
  缠论域语法规则（原则8/13/14）。处理缠论走势建模或分析时自动激活。
  对象否定对象、偏序集建模、合法/非法替代概率。
```

---

### Skill 4: `swarm-architecture`

**定位**：蜂群架构的完整描述——从原则到约束到实现模式。这是最大的 skill，因为蜂群架构本身是高内聚的知识块。

**内容**：

**A. 架构原则**（从 CLAUDE.md 核心原则提取）
1. **原则9：热启动保障蜂群持续自动化**（原 L100）
2. **原则10：蜂群是默认工作模式**（原 L101）
3. **原则11：上下文中的知识有走势结构**（原 L102-110）
   - 扩张/收缩/结晶/重新加载
   - 三维结晶
   - 推论：元编排 = skill 集合
   - 引用 knowledge-crystallization skill
4. **原则12：编排者代理**（原 L111-115）
   - 引用 orchestrator-proxy skill
5. **原则15：拓扑异步自指蜂群是默认架构**（原 L122-134）
   - 真递归、三框架组合、统一约束、三层分布式存在、子蜂群流程
   - 引用 sub-swarm-ceremony skill
6. **原则16：理论洞见影响设计，代码自足表达**（原 L135-137）
7. **原则17：严格性是蜂群的语法规则**（原 L138-142）
   - 引用 rules/no-patch-mentality.md

**B. 五约束有向依赖图**（原 L144-165）
- 五个约束表
- 拓扑结构（核心环路 + 审计层）
- 异步自指的约束来源
- 三个不可消除的 Gap

**C. 递归节点默认行为**（原 L167-184）
- 无阻碍执行列表
- 阻断等待列表

**D. 结构修改模式**（原 L225-231）
- meta-observer 流程
- 审查与合并

**E. 热启动机制**（原 L233-258）
- 三级恢复保障表
- L1 compact 恢复
- L2 跨会话热启动
- 设计目标

**Frontmatter**：
```yaml
name: swarm-architecture
description: >
  蜂群架构完整描述。创建蜂群、评估架构决策、或理解蜂群行为时激活。
  包含原则9-12/15-17、五约束有向依赖图、递归节点行为、结构修改模式、热启动机制。
```

---

### Skill 5: `project-topology`

**定位**：项目文件拓扑——知识仓库映射、谱系目录结构、分布式指令架构。工位需要定位文件或理解项目结构时加载。

**内容**：
1. **知识仓库映射**（原 L186-191）
   - 速查定义、域对象 schema、规则规范、原文参考
2. **谱系目录**（原 L193-196）
   - pending/settled 目录
   - 三种状态
3. **分布式指令架构**（原 L198-214）
   - 指令卡组表（SKILL.md/agents/*.md/references/*.md）
   - 设计原则：一个 agent 一件事

**Frontmatter**：
```yaml
name: project-topology
description: >
  项目文件拓扑与指令架构。需要定位文件、理解目录结构、或查找 agent 职责时激活。
  包含知识仓库映射、谱系目录、分布式指令卡组。
```

---

## 重写后的 CLAUDE.md 结构预览

```markdown
# CLAUDE.md

## 存在论位置（089号谱系：扬弃 Aufhebung）
[保留原文，~12行]

## Language
Always respond in Chinese-simplified (简体中文).

## 缠论资料入口（本仓库）
[保留原文，~6行]

## 来源权威性（三级权威链）
[保留原文，含已知缺口，~25行]

## 元编排规则

本项目使用元编排 v2 方法论（ECC 工程底座 + 概念层守卫）。
元编排的实质内容已结晶为 skill 集合（原则11推论的实现）。

### Skill 索引

| Skill | 路径 | 内容摘要 | 何时加载 |
|-------|------|---------|----------|
| domain-conventions | `.claude/skills/domain-conventions/SKILL.md` | 检索原则、级别口径、溯源标签、输出风格 | 处理缠论内容时 |
| core-principles | `.claude/skills/core-principles/SKILL.md` | 原则0-7：蜂群基础语法 | ceremony 后（所有工位） |
| domain-principles | `.claude/skills/domain-principles/SKILL.md` | 原则8/13/14：缠论域语法规则 | 走势建模/分析时 |
| swarm-architecture | `.claude/skills/swarm-architecture/SKILL.md` | 原则9-12/15-17 + 五约束 + 递归行为 + 热启动 | 蜂群创建/架构决策时 |
| project-topology | `.claude/skills/project-topology/SKILL.md` | 知识仓库映射 + 谱系目录 + 指令架构 | 定位文件/查 agent 职责时 |
| meta-orchestration | `.claude/skills/meta-orchestration/SKILL.md` | 质询序列、概念分离、谱系写入 | 所有 agent（已有） |
| orchestrator-proxy | `.claude/skills/orchestrator-proxy/SKILL.md` | Gemini decide 协议 | 选择/语法记录决断时（已有） |
| sub-swarm-ceremony | `.claude/skills/sub-swarm-ceremony/SKILL.md` | 子蜂群创建流程 | teammate 创建子蜂群时（已有） |
| knowledge-crystallization | `.claude/skills/knowledge-crystallization/SKILL.md` | 知识结晶流程 | 检测到稳定信号时（已有） |
| spec-execution-gap | `.claude/skills/spec-execution-gap/SKILL.md` | 声明-能力一致性检测 | 声明与能力不匹配时（已有） |
| math-tools | `.claude/skills/math-tools/SKILL.md` | 数学工具对照表 | 等价关系封闭后（已有） |
| gemini-math | `.claude/skills/gemini-math/SKILL.md` | Gemini 数学推导 | 形式化证明时（已有） |

### 可用命令
- `/ceremony` — Swarm0：加载初始区分，直接递归进入工作
- `/inquire` — 四步质询序列
- `/escalate` — 矛盾上浮
- `/ritual` — 定义广播仪式
- `/plan` — 实现规划
- `/tdd` — 测试驱动开发
- `/code-review` — 代码审查
```

预估行数：~85 行。

---

## 覆盖完整性验证

| CLAUDE.md 原节 | 去向 | 状态 |
|---------------|------|------|
| 存在论位置 | CLAUDE.md 保留 | ✅ |
| Language | CLAUDE.md 保留 | ✅ |
| 缠论资料入口 | CLAUDE.md 保留 | ✅ |
| 来源权威性 | CLAUDE.md 保留 | ✅ |
| 已知缺口 | CLAUDE.md 保留（权威性子节） | ✅ |
| 检索/引用原则 | → domain-conventions | ✅ |
| 级别口径 | → domain-conventions | ✅ |
| 概念溯源标签 | → domain-conventions | ✅ |
| 输出风格 | → domain-conventions | ✅ |
| 核心原则0-7 | → core-principles | ✅ |
| 核心原则8 | → domain-principles | ✅ |
| 核心原则9-12 | → swarm-architecture | ✅ |
| 核心原则13-14 | → domain-principles | ✅ |
| 核心原则15-17 | → swarm-architecture | ✅ |
| 五约束有向依赖图 | → swarm-architecture | ✅ |
| 递归节点默认行为 | → swarm-architecture | ✅ |
| 知识仓库映射 | → project-topology | ✅ |
| 谱系目录 | → project-topology | ✅ |
| 分布式指令架构 | → project-topology | ✅ |
| 可用命令 | CLAUDE.md 保留 | ✅ |
| 结构修改模式 | → swarm-architecture | ✅ |
| 热启动机制 | → swarm-architecture | ✅ |

**全部 22 个内容单元已覆盖，无遗漏。**

---

## 设计决策说明

### 为什么是 5 个新 skill 而非更多/更少？

**合并依据**：
- 原则0-7 是蜂群的"语法基础"——任何工位都需要，高内聚 → 合并为 `core-principles`
- 原则8/13/14 是缠论域特有的语法规则——只在处理走势时需要 → 合并为 `domain-principles`
- 原则9-12/15-17 + 约束/行为/修改/热启动 都是蜂群架构的不同切面——理解蜂群时需要一起加载 → 合并为 `swarm-architecture`
- 检索/级别/溯源/输出风格 是项目级约定——处理缠论内容时一并需要 → 合并为 `domain-conventions`
- 知识仓库/谱系/指令架构 是项目文件结构——定位文件时一并需要 → 合并为 `project-topology`

**拆分依据**（为什么不继续合并）：
- `core-principles`（所有工位必须）vs `domain-principles`（缠论工作时才需要）——加载场景不同
- `swarm-architecture`（架构层）vs `core-principles`（语法层）——抽象层级不同
- `domain-conventions`（约定）vs `domain-principles`（原则）——约束力度不同（约定是操作惯例，原则是语法规则）

### 为什么 swarm-architecture 最大（~250行）？

蜂群架构是一个高度内聚的知识块：五约束推导→架构原则→递归行为→修改模式→热启动，这些是同一结构的不同投影。拆分它们会破坏理解的完整性——读其中任一部分都需要其他部分的上下文。250 行仍在 200-400 行的推荐范围内。
