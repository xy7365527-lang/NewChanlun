# Phase 2 审计讨论异质质询上下文

## 质询任务

对以下四个议题进行异质否定质询（challenge 模式），产出结构化质询报告。

---

## 背景：Phase 2 完成状态

Phase 2 否定/扬弃拓扑升级已完成实现 + enrichment 执行 + 三轮精度校准。156 测试全部通过。

**当前数据快照（三轮校准后）：**

| 指标 | 校准前 | 校准后 | 变化 |
|------|--------|--------|------|
| duplicates | 110 | 71 | -35% |
| confirmed_duplicate | 51 | 26 | -49% |
| potential_birth | 59 | 45 | -24% |
| conflicts | 4 | 4 | 0 |
| missing_dependency | 613 | 613 | 0 |
| structural_only | 160 | 160 | 0 |
| phantom_negation | 12 | 12 | 0 |
| undeclared_negation | 0 | 0 | 0 |
| active_cycle_rank | 1404 | 1404 | 0 |
| active_β₀ | 2 | 2 | 0 |
| concepts_defined | 1881 | 1140 | -39% |
| relations_written | 3333 | 2592 | -22% |

**已结算相关谱系：**
- 273号：否定拓扑代数（边有效性标记，有向图范畴）
- 271号：审讯终止检测升级（拓扑否定边等价检测）
- 306号：Phase 2 图不变量实现（compute_graph_invariants + TOPOLOGICAL_RELATIONS 常量）
- 307号：enrichment 首次执行（structural_only 703→160，missing_dep 0→613，cycle_rank 654→1404）

**关键设计约束（已结算）：**
- 273号裁定：Phase 2 范围限定在有向图范畴
- TOPOLOGICAL_RELATIONS = 15种关系类型的 frozenset（拓扑关系集）
- VALIDITY_CAPABLE_RELATIONS = 可被 invalidate 的关系子集
- compute_graph_invariants：iterative Tarjan SCC（圈秩）+ BFS（β₀）

---

## 议题1：`inherits` 关系类型引入

### 编排者提案

613 条 missing_dependency 不是数据质量问题——它是一张**概念沉降图**。

三种概念连接的区分：
- **depends_on**（结构性）：显式声明，删掉→论证链断裂
- **references**（指示性）：正文 NNN号引用，删掉→标注没了，论证链还在
- **inherits**（基底性）：正文使用概念但未标注来源。概念已内化为思维基底

声称：inherits 是最危险的依赖形式——不可见的依赖。

**工程方案：**
- `inherits` 进入 `RELATION_TYPES`，order=2
- 从 613 条 missing_dependency 中，通过概念词匹配确认的转标为 `inherits`
- `detect_concept_conflicts()` 扩展检测范围到 inherits
- **不纳入 `VALIDITY_CAPABLE_RELATIONS`**（继承关系不可被 invalidate——概念沉降不可逆）

**支撑数据：**
- 613 条 missing_dependency = 概念沉降候选池
- 24 条隐式否定（structural_only 再抽样中发现）= inherits + direction:counter
- 概念密度枢纽（020号 73 概念）= 最大的概念沉降中心

**需要讨论的子问题：**
1. inherits 是否应该纳入 TOPOLOGICAL_RELATIONS？（影响图不变量计算）
2. inherits 边在 Phase 3 的 directed flag complex 中如何处理？
3. 概念词匹配的精度——613条中哪些是真正的概念继承、哪些只是松散词汇复用？
4. inherits 边方向性：A inherits B = A→B（A 继承 B）

### 质询要求

**质询点1：**613 条 missing_dependency 被解读为"概念沉降图"——这是否对 missing_dependency 的过度解读？missing_dependency 的原始定义是"正文引用但未声明 depends_on"。声称这613条"全部是概念沉降候选"——原始数据有没有其他解读？

**质询点2：**概念词匹配从 references→defines 推断 inherits，精度问题。从"A正文引用了B正文定义过的词"推断"A inherits B"，这条推理链有多少误判空间？有没有反例（A引用了B的词但实际上不依赖B的概念）？

**质询点3：**inherits 不纳入 VALIDITY_CAPABLE_RELATIONS 的理由是"概念沉降不可逆"。反例：如果被继承的概念本身被否定了（B被C否定），A inherits B 的关系还有效吗？如果概念沉降不可逆，那继承者是否永远携带了一个被否定的概念基底？

**质询点4：**inherits 应不应该纳入 TOPOLOGICAL_RELATIONS？如果纳入，613条 missing_dependency 转标为 inherits 会让 cycle_rank 显著上升（inherits 边是否会形成新的 SCC？）。如果不纳入，图不变量的语义完整性——一个关键的隐性依赖关系不进入图，图不变量还能捕捉系统性风险吗？

---

## 议题2：71 duplicates 分析

### 当前数据

三轮校准历史：
1. 停用词方案（被编排者否定——"停用词也是有意义的"）
2. 赋值过滤器（_VALUE_ASSIGNMENT_RE）——过滤日期/路径/代码字面量/URL 等值型定义
3. frontmatter 元数据字段过滤 + NNN号引用过滤

**剩余 71 duplicates 分类：**
Top terms: 推论(8), 四分法分类(8), 结论(7), 模式识别(6), 语法记录(5), 语法记录候选(5), 根因(4), 父(4), 特殊性(4), 建议(4)

其中：
- 26 个 confirmed_duplicate（2/3+ 阈值）
- 45 个 potential_birth（低于 2/3 阈值）

### 质询要求

**质询点1：**"推论"、"结论"、"建议" 等一般性词汇——编排者否定了停用词方案（"停用词也是有意义的"）。但这三个词在知识系统中确实扮演双重角色：既是领域概念，又是认识论词汇。过滤它们 vs 保留它们的代价是什么？保留意味着什么信号噪音？

**质询点2：**26 个 confirmed_duplicate 的威胁等级——"同一词在多个谱系中出现"到底是概念冲突还是概念复用？"四分法分类"在 018号（四分法）和其他谱系中各自独立定义——这是冲突（不兼容的定义）还是复用（同一框架在不同语境的应用）？如何区分？

**质询点3：**potential_birth vs confirmed_duplicate 的 2/3 阈值有没有理论依据？为什么是2/3而不是1/2或3/4？这个阈值是在什么基础上确定的？如果把阈值调整到1/2，会有多少 potential_birth 变成 confirmed_duplicate？

---

## 议题3：cycle_rank=1404 与 Phase 3

### 数据

enrichment 前 cycle_rank=654，enrichment 后 1404（+115%）。三轮精度校准后仍为 1404——过滤影响的是 defines 数量（-39%），但 SCC 结构未变。

**Phase 3 预期：**
- β₁^Δ（directed flag complex 1维 Betti 数）远小于 1404
- 传递三角形（A→B, B→C, A→C）被 2-单纯形填充
- 如果引入 inherits 边，cycle_rank 会进一步上升

### 质询要求

**质询点1：**defines 减少 39% 但 cycle_rank 不变——这说明什么？可能的解读：(a) 被过滤的 defines 都是树枝末端（不参与 SCC），(b) SCC 由 references/depends_on 等其他类型的边驱动，defines 只是叶节点。这两种解读的含义不同，如何区分？

**质询点2：**Phase 3 的 β₁^Δ 是否需要区分边类型构建不同权重的单纯形？现有方案对所有 TOPOLOGICAL_RELATIONS 等权处理——但 depends_on 是强约束（论证依赖），references 是弱约束（指示引用），inherits 如果引入是更强的基底约束。等权处理是否会让 β₁^Δ 的语义混乱？

**质询点3（综合）：**cycle_rank=1404 配合 β₀=2（两个连通分量），给出了什么样的图结构画像？1404 条独立环路的系统是否本质上无法通过拓扑手段检测到"危险"的环路——因为一切都是环路？

---

## 议题4：domain 规范

### 编排者草案

候选 domain 枚举：
- `chanlun_single` — 缠论单标的分析
- `chanlun_multi` — 四矩阵资本流转
- `architecture` — 系统架构/方法论
- `metatheory` — 谱系学/元理论
- `consensus` — 共识仪式/诊断

推断策略（旧谱系）：概念密度分布 + 社区检测

### 质询要求

**质询点1：**5 个候选 domain 的边界够清晰吗？`architecture`（系统架构/方法论）和 `metatheory`（谱系学/元理论）之间的边界在哪里？CLAUDE.md 本身是 architecture 还是 metatheory？RTAS（递归拓扑异步自指蜂群）的设计是 architecture 还是 metatheory？

**质询点2：**"概念密度分布 + 社区检测"推断旧谱系的 domain——这条推理链的可靠性如何？社区检测算法（如 Louvain/Girvan-Newman）在有向图上的适用性问题：这些算法通常设计用于无向图，用在有向谱系图上会不会产生语义失真？

**质询点3：**如果没有 domain 字段，Phase 3 折叠/稳定态检测具体会遇到什么困难？domain 字段的缺失是否是 Phase 3 的真实阻塞项，还是可以先用其他信号（如社区检测结果、已有的 type 字段）近似？

---

## 约束背景

- 谱系不能抹去信息（编排者明确要求）
- Phase 2 范围限定在有向图范畴（273号裁定）
- 12 phantom negation + 48 分散否定 = 已知检测盲区，标记为 Phase 3/4 LLM 辅助提取场景
- inherits 提案需要在 Phase 2/Phase 3 边界上定位

---

## 输出格式要求

对每个议题产出：
1. 质询点（具体说明什么论证步骤被质询）
2. 否定性论证（为什么该论证可能不成立，边界条件、反例）
3. 建议（如果否定成立，应该怎样修正原提案；或者否定不成立的论证）

总长度控制在 8KB 以内。
