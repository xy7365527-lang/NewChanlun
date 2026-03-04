# 上下文耗尽问题：诊断与解决方案设计

## 任务说明

你（Codex）将对一个 LLM 蜂群系统（multi-agent swarm）中的**上下文耗尽（context window exhaustion）**问题进行严格诊断，并评估可能的解决方案。

这不是通用 LLM 应用建议——你必须结合本蜂群的具体架构约束给出精确分析。

---

## 系统架构背景

### 蜂群基本结构

这是一个运行在 Claude Code 平台上的**递归拓扑异步自指蜂群**（RTAS）：
- 每个 agent 节点（Lead/工位/子蜂群）运行在独立的 Claude Code session 中
- 每个节点有独立的 context window（~200K tokens）
- 节点间通过工具调用（TeamCreate/SendMessage/TaskUpdate 等）通信
- 持久化状态写入文件系统（.chanlun/sessions/, .chanlun/genealogy/ 等）

### Context Window 的结构性压力

每个节点的 context window 组成：
1. **CLAUDE.md 基因组**（约 2-3K tokens，每次对话强制加载）
2. **Rules 文件集**（约 5-8K tokens，每次强制加载）
3. **Skill 文件**（按需加载，每个约 2-5K tokens）
4. **对话历史**（随工作推进不断增长）
5. **工具调用结果**（文件读取内容、Bash 输出等，可能很大）
6. **谱系文件读取**（每次引用可增加几百到几千 tokens）

### 已知的热启动机制（现有方案）

系统已有 L1/L2 热启动：
- **L1（compact）**：上下文压缩时，precompact-save.sh hook 自动写入 session 文件，压缩后继续
- **L2（新对话）**：新对话执行 /ceremony，读取 session 文件恢复状态

precompact-save.sh 采集：定义基底、谱系状态、活跃蜂群、中断点，写入 .chanlun/sessions/{timestamp}-session.md

### ceremony 原子链约束（224号/225号）

ceremony 步骤 7→8→9→10 是不可中断的原子链：
- push（7）→ rescan（8）：原子
- rescan（8）→ evaluate/spawn/terminate（9-10）：原子

上下文耗尽若在原子链中间发生，理论上破坏原子性。

### 递归终止条件（274号）

**废除了** depth_budget 全局截断和 max_rescan_depth 安全阀。
递归终止条件仅为：
1. 原子性：当前任务不可分解 → 直接执行
2. 不动点：rescan 未产生新工位 → ceremony 终止

**274号明确声明**："context window 耗尽 → 触发 compaction → 下一轮恢复继续。这是暂停，不是终止。"

但这个声明是否在工程上严格保持，需要诊断。

### Lead 中断根因分析（226号）

226号将 Lead 中断分为三类：
- **类型A**：Bash 后断裂（已修，hook 覆盖）
- **类型B**：纯文本后断裂（平台限制，只能缓解）
- **类型C**：角色边界僭越（规则层缓解）

类型B 的根因：Claude Code 没有 PostOutput hook，LLM 完成文本输出后没有强制执行点。

### 三层无状态问题

ceremony 协议（顺序状态机）vs 执行层（LLM，无状态）vs hook（无状态事件驱动）。
这三层都是无状态的，但 ceremony 要求状态维护。

---

## 问题描述

### 上下文耗尽的具体表现

1. **工位中途中断**：工位在任务未完成时触发 compaction，compaction 后上下文摘要可能丢失关键的局部状态
2. **Lead ceremony 中断**：ceremony 原子链（步骤7-10）在中间触发 compaction
3. **递归深度受限**：每层子蜂群加载的基因组/rules/skills 使可用 context 减少，实际可用深度 < 理论设计深度
4. **谱系写入质量下降**：context 接近上限时，谱系引用和结果包六要素质量下降（省略或敷衍）
5. **Codex review 中断**：异质审查工位调用 Codex API 时，context 中需要保留完整的被审代码+定义上下文

### 核心矛盾

274号声明"context 耗尽不是终止条件而是 compaction 触发条件"，但这依赖：
1. compaction 机制本身可靠运行（平台层）
2. compaction 后恢复的状态足够完整（L1 热启动机制质量）
3. 原子链在 compaction 边界处理正确（原子性保证）

这三个条件中哪一个是最脆弱的？哪一个目前还没有严格保证？

---

## 需要你诊断的问题

### 问题1：context 耗尽的主要消耗来源

分析以下因素对 context 的消耗量和可控性：
- 基因组 + rules 强制加载（不可减少）
- Skill 按需加载（当前策略是什么？有多少冗余？）
- 对话历史（LLM 决策质量 vs context 保留量的权衡）
- 文件读取结果（ceremony_scan.py 输出大小、谱系文件集体读取）
- 工具调用链（多轮 Bash/Read 调用的累积）

**关键问题**：哪些消耗是必要的，哪些是浪费？

### 问题2：现有 L1/L2 热启动机制的严格性评估

L1（compact）的 session 文件仅保存：定义基底版本号、谱系计数、蜂群成员列表、中断点。

评估这是否足够：
- 一个工位在完成任务的 60% 时触发 compaction，session 文件能还原它需要继续的什么状态？
- ceremony Lead 在步骤8（rescan 完成，评估结果尚未输出）时触发 compaction，恢复后能否正确继续步骤9？
- 子蜂群在 TeamCreate 后、工位 spawn 前触发 compaction，子蜂群创建状态如何恢复？

### 问题3：Checkpoint-Resume 方案评估

一个可能的方案：工位在 context 接近上限时，主动 checkpoint 局部状态到文件，然后 spawn 新工位从 checkpoint 恢复。

评估这个方案：
- 优势：局部状态显式化，不依赖 compaction 机制
- 劣势：什么？工程复杂性如何？
- 与现有 L1/L2 热启动的关系：是替代还是补充？
- 实现的最小可行版本是什么？

### 问题4：Prompt 瘦身策略

以下策略哪些在蜂群架构下是可行的：
a. 减少 CLAUDE.md 大小（但这是"基因组"，随意删减会丢失必要信息）
b. 减少 rules 文件集（但规则是语法约束，删减有风险）
c. Skills 按需加载而非预先加载（当前是按需，但触发条件是否过于宽松？）
d. 减少谱系引用的深度（只引用直接前置，不引用二阶）
e. 压缩 ceremony_scan.py 的输出格式（当前 JSON 输出包含多少冗余字段？）

### 问题5：原子链 + Context 耗尽的边界情况

具体场景：ceremony 步骤7（git push）执行完成，步骤8（rescan）开始执行时 context 耗尽触发 compaction。

分析：
- compaction 后，系统如何知道当前处于 ceremony 步骤8还是步骤9？
- 现有 `.chanlun/.ceremony-step` 文件（226号下游推论4提到，但不确定是否已实现）是否提供了这个状态？
- 如果没有，这个边界情况如何处理？

---

## 已有的架构约束（必须遵守）

你的方案必须在以下约束内：

1. **274号**：不引入新的全局截断参数（depth_budget 已废除）
2. **090号**：严格性是语法规则，不允许"先保留旧逻辑，加个新分支"补丁
3. **226号**：类型B中断（纯文本后）是平台限制，不是可修复的 bug——方案不应假设这个问题可以完全解决
4. **057号**：LLM 不是状态机——方案不应依赖 LLM 本身维护精确的执行状态
5. **218号**：Lead 并行化——方案不应引入新的串行阻塞点

---

## 输出格式要求

请按以下结构输出诊断结果：

### §1. Context 消耗主要来源诊断（问题1）
列出可观测/可推断的主要消耗来源，按严重性排序，标注可控性（可减少/不可减少/视情况）。

### §2. 现有热启动机制缺口（问题2）
逐一分析三个具体场景（工位60%完成/ceremony步骤8/子蜂群创建中），给出严格判断：当前机制是否足够？如果不够，缺口在哪？

### §3. Checkpoint-Resume 方案评估（问题3）
给出完整的优/劣分析，以及最小可行实现方案（可以是伪代码级别的工程设计）。

### §4. Prompt 瘦身可行策略排序（问题4）
对 a-e 五个策略按可行性和收益排序，给出理由。

### §5. 原子链 + Context 耗尽边界（问题5）
给出严格的分析和修复方案（如果当前有缺口）。

### §6. 综合建议
优先级排序：哪几个改进最高 ROI？哪些是必须的，哪些是可选的？
