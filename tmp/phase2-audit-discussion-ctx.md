# Phase 2 审计讨论 Context（Gemini + Codex 异质审查）

## 背景

Phase 2 否定/扬弃拓扑升级已完成实现 + enrichment 执行 + 三轮精度校准。156 测试全部通过。
以下是真实谱系数据（307 个区块）上的完整审计信号，需要讨论处理策略。

三轮精度校准历史：
1. 停用词方案（被编排者否定——"停用词也是有意义的"）
2. 赋值过滤器（_VALUE_ASSIGNMENT_RE）——过滤日期/路径/代码字面量/URL 等值型定义
3. frontmatter 元数据字段过滤（_FRONTMATTER_METADATA_FIELDS）+ NNN号引用过滤

## 当前数据快照（三轮校准后）

| 指标 | 校准前 | 校准后 | 变化 | 性质 |
|------|--------|--------|------|------|
| duplicates | 110 | 71 | -35% | 全是真实概念复用（推论8/四分法8/结论7/模式识别6） |
| confirmed_duplicate | 51 | 26 | -49% | 元数据伪重复已清除 |
| potential_birth | 59 | 45 | -24% | 折叠诞生候选 |
| conflicts | 4 | 4 | 0 | 3 个独立 stale_reference 模式 |
| missing_dependency | 613 | 613 | 0 | 正文引用但未声明 depends_on（概念沉降图） |
| structural_only | 160 | 160 | 0 | depends_on 但正文无引用（方向性裂隙） |
| phantom_negation | 12 | 12 | 0 | frontmatter negates 但正文否定分散在推导链中 |
| undeclared_negation | 0 | 0 | 0 | 一致 |
| active_cycle_rank | 1404 | 1404 | 0 | enrichment 引入的循环复杂度 |
| active_β₀ | 2 | 2 | 0 | 主图(316节点) + 孤立岛(3节点=182号共识仪式) |
| concepts_defined | 1881 | 1140 | -39% | 过滤伪概念后 |
| relations_written | 3333 | 2592 | -22% | 过滤伪概念后 |

## 六项审计结果（并行 agent 执行）

### 1. β₀=2 孤立岛（设计特征，非缺陷）
3 节点 = 182号共识仪式（consensus+residue+tension）。两条独立因果链：缠论概念推导 vs 系统诊断共识。Phase 3 计算 β₁ 时需决定：主图还是全图？

### 2. 250号后否定分布（结构特征）
否定表达从二元否定（negates 边）演化为三层机制（negation_source + four_category + validity）。48 个谱系包含分散在推导链中的否定内容——当前标签层基本看不到。标记为 Phase 3/4 的 LLM 辅助提取场景。

### 3. Modification.kind 分布
11 refines: 3 refine + 8 unknown + 0 revises。unknown 73% → conflict 检测信噪比低。
编排者决定：unknown 保持纳入检测范围（保守方向），但 warning 降为 info 级。

### 4. 12 phantom negation
全是假阳性。否定内容存在于正文但分散在推导链、纠正对比、"与X的关系"等段落中。检测器 heading 匹配太严格。与 250号后否定演化是同一问题。

### 5. 160 structural_only 再抽样
65% 结构性继承（-19%）, 15% 隐式否定（NEW，24条）, 20% 过期 frontmatter。
24 条隐式否定是 inherits 提案的直接验证数据——应标为 inherits + direction:counter。

### 6. 概念密度分布
枢纽集中：48% 高密度区块贡献 77.5% 概念。Top: 020号(73 概念)。Phase 3 flag complex 构建时高密度节点可能产生组合爆炸。

## 议题1（核心）：`inherits` 关系类型引入

### 编排者提案

613 条 missing_dependency 不是数据质量问题——它是一张**概念沉降图**。

三种概念连接的区分：
- **depends_on**（结构性）：显式声明"我知道我在依赖谁"。删掉它 → 论证链断裂
- **references**（指示性）：正文 NNN号编号引用。删掉它 → 引用标注没了，论证链还在
- **inherits**（基底性）：正文使用概念但未标注来源。概念已内化为思维基底。你不知道它在那里，直到被继承的概念被否定，所有继承者的论证全部动摇

inherits 是最危险的依赖形式：不可见的依赖。当前 `detect_concept_conflicts()` 的 stale_reference 检测只查 references 和 depends_on——inherits 完全在盲区。

### 审计支撑数据

- 613 条 missing_dependency = 概念沉降候选池
- 24 条隐式否定（structural_only 再抽样中发现）= inherits + direction:counter
- 概念密度枢纽（020号 73 概念）= 最大的概念沉降中心

### 工程方案

- `inherits` 进入 `RELATION_TYPES`，order=2
- 从 613 条 missing_dependency 中，通过概念词匹配确认的转标为 `inherits`
- `detect_concept_conflicts()` 扩展检测范围到 inherits
- 不纳入 `VALIDITY_CAPABLE_RELATIONS`（继承关系不可被 invalidate——概念沉降不可逆）

### 需要讨论

1. inherits 是否应该纳入 TOPOLOGICAL_RELATIONS（影响图不变量计算）？
2. inherits 边在 Phase 3 的 directed flag complex 中如何处理？（与 depends_on/references 环路性质不同）
3. 概念词匹配的精度问题——613 条中哪些是真正的概念继承、哪些只是松散的词汇复用？需要什么级别的验证？
4. inherits 边是否应该有方向性？A inherits B 意味着 A 的正文使用了 B 中定义的概念——方向是 A→B（A 继承 B）

## 议题2：71 duplicates 的分析

### 三轮校准结果

| 轮次 | duplicates | confirmed | potential_birth | 变化原因 |
|------|-----------|-----------|-----------------|----------|
| 校准前 | 110 | 51 | 59 | enrichment 原始输出 |
| +赋值过滤 | 97 | 38 | 59 | 过滤日期/路径/代码/URL 值型定义 |
| +元数据+NNN号 | 71 | 26 | 45 | 过滤 frontmatter 字段 + 谱系引用号 |

### 剩余 71 duplicates 分类

Top terms: 推论(8), 四分法分类(8), 结论(7), 模式识别(6), 语法记录(5), 语法记录候选(5), 根因(4), 父(4), 特殊性(4), 建议(4)

这些是真正的概念复用——多个谱系区块独立定义同一概念。其中：
- "推论"、"结论" 是一般性词汇在不同语境下的实例化（类似"方法"、"模型"）
- "四分法分类"、"语法记录"、"语法记录候选" 是核心方法论概念在不同谱系中的演化定义
- "模式识别"、"根因" 是分析框架概念的多次使用

### 需要讨论

1. 一般性词汇（推论/结论/建议）是否应该加入新的"通用词"过滤？vs 保留作为概念演化信号？
2. 26 个 confirmed_duplicate 中有多少是真正的概念冲突（同一概念的不兼容定义）vs 合法的概念扩展？
3. 45 个 potential_birth 的验证策略——Phase 3 域信息确认前是否做进一步分类？

## 议题3：cycle_rank=1404 与 Phase 3

enrichment 前 cycle_rank=654，enrichment 后 1404（+115%）。references 和 defines 边大量创造新 SCC 环路。
三轮校准后仍为 1404——过滤影响的是 defines 数量（-39%），但 SCC 结构未变。

### Phase 3 预期

- β₁^Δ（directed flag complex 1维 Betti 数）远小于 1404
- 传递三角形（A→B, B→C, A→C）被 2-单纯形填充
- 如果引入 inherits 边，cycle_rank 会进一步上升，但 inherits 环路性质不同

### 需要讨论

1. inherits 边加入后 cycle_rank 的预期增量
2. Phase 3 的 directed flag complex 是否应该区分边类型？
3. domain 字段规范——当前 relations.jsonl 中 domain 字段使用量为 0

## 议题4：domain 规范草案

Phase 3 折叠/稳定态检测的前置条件。编排者初步提案：

候选 domain 枚举：
- `chanlun_single` — 缠论单标的分析
- `chanlun_multi` — 四矩阵资本流转
- `architecture` — 系统架构/方法论
- `metatheory` — 谱系学/元理论
- `consensus` — 共识仪式/诊断

推断策略（旧谱系）：概念密度分布 + 社区检测

## 约束

- 谱系不能抹去信息（编排者明确要求）
- Phase 2 范围限定在有向图范畴（273号裁定）
- inherits 提案需要在 Phase 2/Phase 3 边界上定位
- 12 phantom negation + 48 分散否定 = 已知检测盲区，标记为 Phase 3/4 LLM 辅助提取场景
