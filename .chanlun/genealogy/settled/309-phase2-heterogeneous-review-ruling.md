---
id: '309'
number: 309
type: consensus
title: "Phase 2 异质审查裁定——四议题处置 + 概念注册表提出 + 关系分层引入"
date: "2026-03-02"
depends_on: ['308', '307', '273', '231', '090']
status: 已结算
---

# 309号：Phase 2 异质审查裁定——四议题处置 + 概念注册表提出 + 关系分层引入

## 递归判断

任务不可分解：单一谱系写入任务，内容是编排者对四议题的裁定记录。扁平退化特例。

## 背景

本 session 的核心产出是编排者对 Gemini+Codex 异质审查四议题的逐条裁定。Gemini 作为 challenger 提出致命点，Codex 作为 reviewer 提供交叉评审，编排者基于双方意见做出最终裁定。

## 议题1：inherits

### 接受的致命点

- inherits 定义（"最危险的依赖"——被继承概念出错时，所有继承者静默携带错误）与不纳入 VALIDITY_CAPABLE_RELATIONS 的工程设计自相矛盾。如果 inherits 是"最危险的依赖"，那么它恰恰应该参与有效性传播——否则"最危险"只是修辞而非工程语义。
- "显式提到的通常不是潜意识的基底"——检测方法（词匹配）与语义定义（隐式内化）矛盾。inherits 的语义是"B 的基底概念被 A 内化为先验"，但词匹配只能检测显式引用，不能检测隐式内化。

### 否定的方案

- Gemini 的二选一方案（要么纳入 VALIDITY_CAPABLE 要么删除 inherits 类型）被否定。正确设计是：B 被否定时，所有 `A inherits B` 生成 warn 级 annotation（非自动 invalidation）。这保留了 inherits 的语义价值（标记潜在风险），同时避免了自动失效传播的精度问题。

### 裁定

- inherits 作为关系类型**保留**，Phase 2 **不实现**
- 613 条 missing_dependency 保持为 info 级 mismatch，不升格
- inherits 真正实现留给 Phase 3/4（LLM 辅助语义检测）
- 提出**概念注册表**（concept registry）：只有被至少一个区块 `defines` 过的概念词才能触发 inherits 检测。概念注册表是 inherits 实现的前置条件

## 议题2：duplicates

### 接受的致命点

- "推论/结论/建议"是文档结构标签不是概念实体。这些词在每个谱系条目中都出现，但它们标记的是文档结构（section heading），不是领域概念。将它们作为 duplicate 检测对象是范畴错误。

### 裁定

- 取消计数阈值（N=2 vs N>=3 的争论不再相关），所有 N>=2 进入 duplicate 检测
- 分类从 duplicate 改为 **needs_review**——表明需要人工确认是真实重复还是多义词
- 多义性 = 折叠（同一词在不同域中有不同语义），需域信息确认——回到议题4（domain）

## 议题3：cycle_rank

### 接受的分层方案

关系分两层：

- **Layer 1（逻辑层）**：`depends_on` + `negates` + `revises` + `supersedes`——这些关系携带实质性语义修正
- **Layer 2（导航层）**：Layer 1 + `references` + `defines` + `refines`——完整的导航图

### 裁定

- Phase 3 的 `flag complex` 只基于 **Layer 1**——这是实质性修正。原设计基于全量活跃关系，现在缩减为逻辑层关系。beta_1^Delta 的语义更纯净：只计算实质性修正关系中的独立环路
- 否定"弗兰肯斯坦单纯形"的过度工程化方案（加权复形/多维复形）——分层已解决核心问题，无需引入额外的代数结构

## 议题4：domain

### 接受的方案

- domain 下沉到 **concept 级**（非 block 级）。一个区块可以包含多个概念，每个概念可以属于不同的域。block 级 domain 标注粒度太粗。

### 否定的方案

- 社区检测方案被否定。自动化的社区检测在小规模知识图谱上不可靠，且产出的 domain 边界缺乏语义解释。改为**手动分类 + 编排者确认**。

### 裁定

- architecture/metatheory **不合并**——两者的关注点不同（system_architecture 关注工程实现，metatheory 关注元编排方法论）
- domain 枚举最终定为六个：
  1. `chanlun` — 缠论领域概念
  2. `capital_flow` — 资金流向/市场分析
  3. `system_architecture` — 系统工程/代码架构
  4. `metatheory` — 元编排方法论
  5. `consensus` — 共识仪式/异质审查
  6. `implementation` — Phase 实现/工程执行

## 下游推论

1. Phase 3 `flag complex` 的输入从"全量活跃关系"缩减为"Layer 1 逻辑关系"——beta_1^Delta 的语义更纯净。这是对 Phase 3 实现规格的实质性修正
2. 概念注册表（concept registry）是 inherits 实现的前置条件——Phase 3 需要先建立 defines 关系的反向索引
3. domain 标注粒度确定为 concept 级——Phase 3 的折叠检测逻辑需要重新设计，从 block 级标注改为 concept 级标注
4. 文档结构词过滤列表需要持续维护——可能需要结晶为配置文件（与 308号观察1 中的"过滤操作判断依据层级"规则一致）

## 边界条件

- 六个 domain 枚举是当前谱系规模（309条）下的分类。随着谱系增长，可能需要细分（如 chanlun 细分为 chanlun_definition / chanlun_application）
- Layer 1/Layer 2 分层假设 `references`/`defines`/`refines` 不携带实质性修正语义——如果后续发现某些 refines 实际上是实质性修正（如改变定义边界），分层需要修订
- 概念注册表依赖 defines 关系的提取质量——如果 defines 的召回率低，注册表会不完整，inherits 检测的覆盖率会受限
- needs_review 分类意味着 duplicate 检测不再是全自动的——需要人工（编排者）确认步骤

## 影响声明

- 本谱系记录的裁定直接影响 Phase 3 的实现规格：flag complex 的输入范围、domain 标注粒度、inherits 实现时序
- 概念注册表是新的架构组件，未在之前的 Phase 设计中出现
- 关系分层（Layer 1/Layer 2）是对 block-topology 关系模型的结构性扩展

## 谱系引用

- 308号：异质审查启动的元观察（本裁定的直接前置）
- 307号：enrichment 首次执行（613 missing_dep 的发现触发了 inherits 议题）
- 273号：有向图范畴裁定（关系类型的范畴基础）
- 231号：形式化有效域规则（L0-L3 认识论等级——inherits 的 L2+ 验证需求）
- 090号：严格性语法规则（否定过度工程化方案的依据）

### 下游推论解决记录（v134-swarm session）

- 推论1（Phase 3 flag complex 输入缩减为 Layer 1）：**direction_changed** — 编排者已取消原 Phase 3 方案（directed flag complex → β₁^Δ 全部取消）。新方向：traverse.py 查询接口 + 偶遇记录 + Morse 地形 + 概念注册表导出。Layer 1 cycle_rank = 0 已确认（310号补充记录），逻辑层是 DAG，flag complex 不再适用
- 推论2（概念注册表是 inherits 前置条件）：**direction_changed** — inherits Phase 2 不实现（309号裁定自身已确认）。编排者新方向将概念注册表从 inherits 前置条件转变为 Phase 3 独立导出功能，不再与 inherits 绑定
- 推论3（domain 标注粒度 concept 级）：**direction_changed** — 编排者否定社区检测方案（"domain 社区检测不做"）。domain 下沉到 concept 级的裁定保留，但自动化实现路径取消。六域枚举（chanlun/capital_flow/system_architecture/metatheory/consensus/implementation）作为静态配置保留，不在 Phase 3 近期路径中
- 推论4（文档结构词过滤列表维护）：**resolved** — `_STRUCTURAL_MARKERS` frozenset 已在 310号 session（commit 489ba76）中实现于 concept_extractor.py，18 个结构标记。与 `_FRONTMATTER_METADATA_FIELDS`（17 字段）共同构成双层结构过滤
