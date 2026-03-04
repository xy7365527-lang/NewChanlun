# Gemini Decide 上下文：096号——蜂群规则的分布式存在形式

## 决断类型：选择

## 背景

095号确立了 Agent Team 子蜂群真递归作为默认模式。编排者提出两个 INTERRUPT：

1. **规则存在形式问题**："如果你写在 prompt 里，那么它就不是分布式的，也就无法变成真严格递归拓扑异步自指蜂群"
2. **不要有例外**："subagent 的架构跟 agent team 完全不同"——Explore 的例外也不应存在

## 核心矛盾

当前方案用三种机制实现"真递归"：
1. `sub-swarm-ceremony/SKILL.md`——定义子蜂群四特征
2. `team-structural-inject.sh`——TeamCreate 后向 Lead prompt 注入规则
3. `agent-team-enforce.sh`——阻断非 team 的 Task 调用（Explore 除外）

**矛盾**：方式 2 是中心化注入（父→子 prompt 传递），不是分布式存在。子蜂群不是"自主拥有规则"，而是"被注入规则"。

## 已有的分布式机制

Claude Code 平台已经提供了分布式规则传递：
- **CLAUDE.md**：每个 agent（包括 teammate）启动时平台自动加载。这是 089 号识别的"基因组"
- **hooks**：PreToolUse/PostToolUse/Stop 对所有 agent 自动触发，不依赖 prompt
- **skills**：通过 Skill 工具按需加载

这意味着：
- CLAUDE.md 中写的规则，递归每一层的 agent 都自动拥有
- hooks 中写的强制逻辑，递归每一层都自动执行
- 不需要通过 prompt 注入来传递规则

## 四个选项

### A：完全依赖平台分布式机制
- 规则全部写在 CLAUDE.md + hooks + skills 中
- 删除 `team-structural-inject.sh` 的 prompt 注入逻辑
- `agent-team-enforce.sh` 无例外（Explore 也必须通过 team）
- 子蜂群的结构保障来自：(1) CLAUDE.md 原则15已声明真递归默认; (2) hooks 强制 Task 必须带 team_name; (3) TeamCreate 后 hook 提示可用 skills
- **优点**：真正分布式，规则存在于文件系统不是 prompt
- **缺点**：Explore 走 team 流程开销大（需要 TeamCreate → Task → TeamDelete）

### B：平台分布式 + Explore 特殊处理
- 同 A，但 Explore 在 agent-team-enforce.sh 中保留例外
- **优点**：实用性高，Explore 本来就是轻量只读工具
- **缺点**：架构不纯——编排者明确说"不要有例外"

### C：完全禁用 Task(无 team_name) + 用 Explore agent 的 team 化替代方案
- 创建一个 `explore-team` 的快捷模式：TeamCreate(临时team) → Task(Explore, team) → TeamDelete
- 封装在一个 hook 或 skill 中，自动化这三步
- **优点**：架构纯粹，无例外
- **缺点**：实现复杂度高，每次搜索都要走完整 team 生命周期

### D：完全禁用孤立 subagent，Explore 功能改用直接工具调用
- Task(Explore) 被完全禁止
- 需要搜索时，直接使用 Glob/Grep/Read 工具，或在当前 team 内 spawn 一个 teammate
- **优点**：最简单，无例外，无特殊处理
- **缺点**：失去 Explore agent 的自主搜索能力（它能做多轮搜索、自主判断搜索方向）

## 编排者约束

1. "不要有例外"——排除 B
2. "规则写在 prompt 里就不是分布式的"——排除当前方案
3. "subagent 架构跟 agent team 完全不同"——要求架构纯粹性

## 五约束框架分析

| 约束 | A | C | D |
|------|---|---|---|
| 1a 物理持久化 | 无影响 | 无影响 | 无影响 |
| 1b 符号可解释性 | CLAUDE.md 是可解释的 | 同 A | 同 A |
| 2 规则先在性 | ✅ CLAUDE.md 先于执行加载 | ✅ 同 A | ✅ 同 A |
| 3 执行不可自观性 | hooks 异步检查 | 同 A | 同 A |
| 4 异质验证必要性 | hooks 作为异质验证层 | 同 A | 失去 Explore 的独立搜索能力 |

## 请 Gemini 决断

在 A、C、D 三个选项中选择（B 被编排者约束排除）。

关键决策维度：
1. 架构纯粹性 vs 实用性
2. 分布式规则传递的严格实现
3. Explore 功能的替代方案是否充分
