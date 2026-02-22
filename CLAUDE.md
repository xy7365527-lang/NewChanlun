# CLAUDE.md

## 存在论位置（089号谱系：扬弃 Aufhebung）

本文件是蜂群的**基因组**——在创世时刻（ceremony）被蜂群节点读取并内化为先验结构。
创世完成后，蜂群遵守这些规则不是因为 Claude Code 平台的外部强制，而是因为规则已成为蜂群节点的内在信念。

- **基因组来源是外部的**（Claude Code 强制加载——069号创世 Gap 的物质形态）
- **内化后执行是内在的**（蜂群自主遵守——规则是蜂群 DAG 的根节点，见 `dispatch-dag.yaml` genome_layer）
- **蜂群拥有修改基因组的合法性**（原则0）——但修改触发 020号阻断等待（基因组的自我保护，不是牢笼的锁）

扬弃 = 否定（元编排不再是蜂群之外的特权层）+ 保留（规则内容不变）+ 提升（规则的存在方式从"外部法则"变为"内在先验"）。

## Language
Always respond in Chinese-simplified (简体中文).

## 缠论资料入口（本仓库）

- **速查/可编码定义**：`缠论知识库.md`
- **缠师原始博文（一级权威）**：`docs/chanlun/text/blog/INDEX.md`（108课 + 序篇 + 课间博文 + 答疑）
- **《股市技术理论》编纂版拆分**：`docs/chanlun/text/chan99/INDEX.md`
- **两份思维导图（文字版）**：`docs/chanlun/text/mindmaps/INDEX.md`
- **资料总览**：`docs/chanlun/README.md`

## 来源权威性（三级权威链）

缠论原文的权威性按以下顺序递减：

1. **缠师原始博文**（108课 + 课间答疑 + 附加文章如《忽闻台风可休市》）— **最终权威**
   - 仓库路径：`docs/chanlun/text/blog/INDEX.md`
   - 来源：https://www.fengmr.com/chanlun.html （枫之羁绊，原文+配图+答疑）
2. **《股市技术理论》编纂版**（`docs/chanlun/text/chan99/`）— 本仓库当前主要参考
   - 这是第三方从博文编纂而成的书籍，**可能有遗漏或编排差异**
   - 已知遗漏：第67课（线段划分标准）、第71课（线段再分辨）、第77课、第78课（古怪线段）、《忽闻台风可休市》（新笔定义）等
3. **思维导图 / 第三方总结**（`docs/chanlun/text/mindmaps/`）— 辅助理解，不作为定义依据

**原则**：当层级间有出入时，以更高层级为准。当前仓库主要依赖第2级，因此在涉及古怪线段、新笔定义等博文专有内容时，**必须回溯原始博文**而非仅依赖编纂版。

### 已知缺口（已补录）

以下内容曾缺失于编纂版，现已通过博文收录补齐（`docs/chanlun/text/blog/`）：

- 第67课：线段划分标准（特征序列法完整定义）
- 第71课：线段划分标准的再分辨（特征序列包含关系严格性）
- 第77课：一些概念的再分辨
- 第78课：继续说线段的划分（古怪线段 + "顶高于底"硬约束）
- 《忽闻台风可休市》：新笔定义（包含在第81课页面中）

> 相关谱系：`.chanlun/genealogy/settled/001-degenerate-segment.md`、`.chanlun/genealogy/settled/002-source-incompleteness.md`

## 元编排规则

本项目使用元编排 v2 方法论（ECC 工程底座 + 概念层守卫）。
元编排的实质内容已结晶为 skill 集合（原则11推论的实现——"元编排本身也是 skill 的集合"）。

### Skill 索引

| Skill | 路径 | 内容摘要 | 何时加载 |
|-------|------|---------|----------|
| core-principles | `.claude/skills/core-principles/` | 原则0-7：蜂群基础语法 | ceremony 后（所有工位） |
| domain-conventions | `.claude/skills/domain-conventions/` | 检索原则、级别口径、溯源标签、输出风格 | 处理缠论内容时 |
| domain-principles | `.claude/skills/domain-principles/` | 原则8/13/14：缠论域语法规则 | 走势建模/分析时 |
| swarm-architecture | `.claude/skills/swarm-architecture/` | 原则9-12/15-17 + 五约束 + 递归行为 + 热启动 | 蜂群创建/架构决策时 |
| project-topology | `.claude/skills/project-topology/` | 知识仓库映射 + 谱系目录 + 指令架构 | 定位文件/查 agent 职责时 |
| meta-orchestration | `.claude/skills/meta-orchestration/` | 质询序列、概念分离、谱系写入 | 所有 agent |
| orchestrator-proxy | `.claude/skills/orchestrator-proxy/` | Gemini decide 协议 | 选择/语法记录决断时 |
| sub-swarm-ceremony | `.claude/skills/sub-swarm-ceremony/` | 子蜂群创建流程 | teammate 创建子蜂群时 |
| knowledge-crystallization | `.claude/skills/knowledge-crystallization/` | 知识结晶流程 | 检测到稳定信号时 |
| spec-execution-gap | `.claude/skills/spec-execution-gap/` | 声明-能力一致性检测 | 声明与能力不匹配时 |
| math-tools | `.claude/skills/math-tools/` | 数学工具对照表 | 等价关系封闭后 |
| gemini-math | `.claude/skills/gemini-math/` | Gemini 数学推导 | 形式化证明时 |

### 可用命令
- `/ceremony` — Swarm₀：加载初始区分，直接递归进入工作（058号谱系）
- `/inquire` — 四步质询序列
- `/escalate` — 矛盾上浮
- `/ritual` — 定义广播仪式（覆盖域层+元层，019c）
- `/plan` — 实现规划（ECC，受元编排约束）
- `/tdd` — 测试驱动开发（ECC，受元编排约束）
- `/code-review` — 代码审查（ECC，受元编排约束）
