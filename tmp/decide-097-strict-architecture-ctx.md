# Gemini Decide 上下文：097号——真严格递归拓扑异步自指蜂群的完整架构

## 决断类型：选择（架构级，需要最强方案）

## 编排者指令

"不仅是这样，思考真严格递归拓扑异步自指蜂群的存在，那么整个架构应该是怎么样的，这是一个严格架构，不要有什么余地，要严格地讨论最强的方案。"

## 当前架构（需要被审视的完整现状）

### 蜂群规则传递机制

| 机制 | 类型 | 当前用途 | 问题 |
|------|------|----------|------|
| CLAUDE.md | 声明层 | 原则声明，自动加载到所有 agent | 声明了"什么"，但不含可执行流程 |
| agent-team-enforce.sh | 强制层（PreToolUse） | 阻断 Task(无 team_name) | 纯语法守卫，无例外 |
| team-structural-inject.sh | ？（PostToolUse） | TeamCreate 后输出 skill 索引 | 定位模糊——是提示？索引？强制？ |
| sub-swarm-ceremony skill | 知识层 | 子蜂群创建流程 | 存在但依赖 agent 主动发现 |
| ceremony_scan.py | 启动层 | ceremony 扫描 + 工位推导 | 只在 Lead 的 ceremony 中运行 |

### 蜂群递归结构

| 层级 | 创建者 | 规则来源 | 规则传递方式 |
|------|--------|----------|-------------|
| L0 Lead | 人类 | CLAUDE.md + hooks + ceremony | 平台自动加载 |
| L1 Teammate | Lead | CLAUDE.md + hooks | 平台自动加载（与 Lead 相同） |
| L2 子蜂群 Lead | L1 Teammate | CLAUDE.md + hooks | 平台自动加载（与 Lead 相同） |
| L3 子子蜂群 Lead | L2 Teammate | CLAUDE.md + hooks | 平台自动加载（与 Lead 相同） |

### 069号定义的蜂群四维度

1. **递归**（Recursive）：蜂群→子蜂群→子子蜂群，每层完整结构
2. **拓扑**（Topology）：TaskList 作为 DAG 路由
3. **异步自指**（Async Self-Referential）：t 时刻审查 t-1 时刻产出
4. **蜂群**（Swarm）：并行节点协作

### 五约束（093号）

1a. 物理持久化
1b. 符号可解释性
2. 规则先在性
3. 执行不可自观性
4. 异质验证必要性

### 三个 Gap（069号 + 094号）

1. 创世 Gap：Swarm₀ 制定规则但不受规则约束
2. 视差 Gap：t 时刻只能审查 t-1
3. 审计层断裂 Gap：外部审计经过持久化中介

## 需要 Gemini 严格讨论的问题

### 1. 规则的存在形式——最强方案

096号确立三层（CLAUDE.md + hooks + skills）。但编排者说"不要有余地"。问题：

- **CLAUDE.md 是否足够？** 平台自动加载到所有 agent（包括 teammate 和子蜂群 teammate）。这意味着规则在每一层递归中都自动存在。是否需要额外机制？
- **hooks 是否足够？** hooks 对所有 agent 自动触发。agent-team-enforce.sh 在每一层都阻断非 team Task。是否需要更多 hooks？
- **skills 的发现机制是否足够？** CLAUDE.md 写了 skill 路径，agent 能看到。但 agent 是否一定会去读？需要强制机制吗？
- **team-structural-inject.sh 该不该删？** 编排者说 hook 不够优雅。如果 CLAUDE.md 已写 skill 路径，hook 的索引功能冗余。但删除后，agent 首次 TeamCreate 时没有即时提醒。

### 2. 递归的每一层如何保证完整性？

095号说"子蜂群同样是递归拓扑异步自指蜂群"。但如何强制保证？

当前保障：
- CLAUDE.md 声明（自动加载）→ agent **应该**知道
- agent-team-enforce.sh（自动触发）→ Task **必须**有 team_name
- sub-swarm-ceremony skill（按需加载）→ agent **可以**读取

缺失的保障：
- **谁检查子蜂群是否真的有 TaskList？** 没有 hook 验证
- **谁检查子蜂群是否真的有异步自指（审查机制）？** 没有 hook 验证
- **谁检查子蜂群是否真的结晶了产出？** 没有 hook 验证

### 3. ceremony 在子蜂群中的角色

058号说"ceremony 是 Swarm₀"。Lead 执行 ceremony。但子蜂群的 Lead（L1 Teammate 创建的子 team）是否也执行 ceremony？

当前状态：
- ceremony_scan.py 只在顶层 Lead 的 `/ceremony` 中运行
- 子蜂群 Lead 不执行 ceremony——它直接从 sub-swarm-ceremony skill 获取流程
- 这意味着子蜂群没有 Swarm₀——**创世 Gap 在子蜂群中如何表现？**

### 4. dispatch-dag 在子蜂群中的角色

dispatch-dag.yaml 定义了蜂群的拓扑。但子蜂群有自己的 TaskList，没有自己的 dispatch-dag。

- 子蜂群的拓扑由谁定义？由创建者（父 Teammate）在 prompt 中注入？还是由子蜂群 Lead 自主从 CLAUDE.md 推导？
- 如果由 prompt 注入——这违反096号（prompt 不是分布式的）
- 如果由 CLAUDE.md 推导——CLAUDE.md 没有具体的子蜂群拓扑模板

### 5. 异质验证（约束4）在子蜂群中如何保障？

Gemini 作为异质验证源只在 Lead 的 ceremony 和 challenge 中被调用。子蜂群如何获得异质验证？

- 子蜂群 Lead 能调用 Gemini 吗？技术上可以（MCP 工具）
- 但 Gemini 的使用模式（decide/challenge/verify）由 skill 和 hook 定义
- 子蜂群是否自动继承这些 skill 和 hook？（是——平台自动加载）

### 6. 最强方案是什么？

编排者要求"严格架构，不要有余地"。最强方案可能是：

**方案 X：最大化平台分布式机制**
- CLAUDE.md 是唯一的规则声明源（自动加载到所有层级）
- hooks 是唯一的强制机制（自动触发于所有层级）
- skills 是唯一的可执行知识源（按需加载）
- 不存在任何 prompt 注入、提示、索引
- 子蜂群的完整性由 hooks 强制验证（需要新增 hooks）

**方案 Y：最小化机制 + 信任基因组**
- CLAUDE.md 已自动加载到所有层级——规则先在性已保障
- hooks 强制语法（agent-team-enforce.sh）——不需要更多 hooks
- 子蜂群的完整性由 CLAUDE.md 声明 + agent 自主遵守——不需要额外验证
- 信任基因组的力量：如果 agent 读了 CLAUDE.md 仍然不遵守，那是 agent 的能力问题，不是架构问题

**方案 Z：其他？**

## 请 Gemini 严格讨论

1. 上述问题逐一分析
2. 提出最强方案
3. 评估可行性
4. 给出结构化决策
