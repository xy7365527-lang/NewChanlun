---
id: '431'
number: 431
title: "ARTICULATE 拓扑判据——从度量阈值到拓扑不一致（索绪尔两轴 + A/B密度失衡信号）"
type: concept-separation
status: 已结算
date: 2026-03-12
source: 编排者否定（ARTICULATE 阈值不够内在）+ 编排者观察（索绪尔两轴对应 + 密度失衡信号）
negation_source: homogeneous
negation_form: replacement
topo_effect: "replace:425-Phase2:articulation-criterion — 425号 Phase 2 的 ARTICULATE 触发条件从度量阈值替换为拓扑不一致判据"
depends_on:
  - '425'   # S_net 入图方案（ARTICULATE 机制首次实装）
  - '426'   # 无意识结构精确定义（遭遇的存在论位置）
epistemological_level: "L0（拓扑不一致是布尔判据，从定义推导）"
---

# 431号：ARTICULATE 拓扑判据——从度量阈值到拓扑不一致

**认识论等级**: L0（拓扑不一致是布尔判据——有/无，不是多/少。从 Layer A/B 的定义直接推导）

## 1. 矛盾

425号 Phase 2 实装的 ARTICULATE 触发条件：

```python
score = cooc_weight * ta_count / step_gap
if score >= ARTICULATION_THRESHOLD:  # 2.0
    trigger_articulation()
```

编排者否定：这是外部度量判据。"谁决定了乘积大于多少算'涌现'？这个数值从哪里来？如果是你设定的，那 ARTICULATE 的触发条件就不是拓扑内在的。"

更根本的问题：ARTICULATE 是逢亮在遭遇点**被迫**产出的概念边——被迫，不是达到阈值后自动晋升。遭遇的标志不是三个数值的乘积够大，是物质层和概念层在这个位置上存在**不可解释的不一致**。

## 2. 索绪尔两轴与 S_net Layer A/B 的对应

编排者观察：

| 索绪尔 | 拉康 | S_net Layer | 边类型 | 运动方式 |
|--------|------|-------------|--------|----------|
| 组合轴（syntagmatic） | 换喻轴 | Layer A | COOCCURRENCE + TRAVERSAL_ASSOCIATION | 滑动（从一个词到文本中的邻居） |
| 聚合轴（paradigmatic） | 隐喻轴 | Layer B | DICT_* (paradigmatic_alternatives) | 跳跃（从一个词到同义词/对立词） |

穿越同时在两轴移动。两轴交叉点 = 遭遇高概率位置。

## 3. 拓扑不一致判据（替代度量阈值）

不一致 = 两轴在同一对顶点上的拓扑连接状态矛盾（一轴有边另一轴无边）。布尔判据。

### A密B疏（组合轴密、聚合轴疏）

当前位置与某邻居之间有 COOCCURRENCE 边（物质层共现），但没有任何 paradigmatic 边（辞典关系）也没有概念层边。

含义：语料中反复共现但辞典里没有关系——可能是学科特有的术语关联。

### B密A疏（聚合轴密、组合轴疏）

当前位置与某邻居之间有 paradigmatic 边（辞典说相关），但没有 COOCCURRENCE 边（语料中从不共现）。

含义：辞典说它们相关但语料中从不共现——可能是死关系。

### 密度失衡信号

语料摄入后 Layer A 暴增（共现边随语料线性增长），Layer B 不变（辞典关系相对固定）。两轴密度比剧烈失衡。失衡区域本身是遭遇候选位置。

## 4. 定义依据

| 概念 | 来源 | 使用方式 |
|------|------|----------|
| 组合轴/聚合轴 | Saussure, F. (1916). Cours de linguistique générale | Layer A/B 的语言学基础 |
| 换喻轴/隐喻轴 | Lacan, J. (1957). L'instance de la lettre | 两轴运动的精神分析对应 |
| 遭遇 = 被迫 | 426号（无意识结构精确定义） | ARTICULATE 不是自动晋升，是被迫产出 |
| 拓扑不一致 | 编排者否定 | 布尔判据替代度量判据 |

## 5. 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| A密B疏判据 | L0（定义推导） | 若语料摄入后 A密B疏触发过于频繁（噪声），需要引入最小共现次数过滤——但这是噪声过滤，不是阈值回归 |
| B密A疏判据 | L0（定义推导） | 若辞典质量差导致大量假阳性，需要辞典质量审计 |
| 两轴对应 | 编排者观察 | 若发现 S_net 中存在不属于两轴的第三类边，需要扩展框架 |

## 6. 下游推论

1. **traversal.py 已修改**：`_check_articulation_encounter()` 从度量阈值改为拓扑不一致判据
   - status: executed（v231-swarm/articulate-refactor）
   - covered: 行动——traversal.py:1102 方法已实装，使用 imbalance_type 布尔判据（v232-swarm/genealogy-proposals 验证）
2. **daemon.py 已修改**：articulation block 写入格式从 cooc_weight/ta_count/step_gap 改为 imbalance_type/reason
   - status: executed（v231-swarm/articulate-refactor）
   - covered: 行动——daemon.py:1027 使用 imbalance_type/reason 字段（v232-swarm/genealogy-proposals 验证）
3. **block_topology_persistence.py 已修改**：`write_articulation_block()` 接口同步更新
   - status: executed（v231-swarm/articulate-refactor）
   - covered: 行动——block_topology_persistence.py:365 接口包含 imbalance_type 参数（v232-swarm/genealogy-proposals 验证）
4. **L2 验证需要**：在真实语料摄入后观察 A密B疏/B密A疏 的触发频率和分布
   - status: pending（需要 VPS daemon 重启后观察）
   - not_covered: 行动——依赖外部条件（VPS daemon 真实语料摄入），非实装缺失

## 7. 谱系引用

| 关联谱系 | 关系 |
|---------|------|
| 425号 | S_net 入图方案——ARTICULATE 机制首次实装，本谱系替换其触发条件 |
| 426号 | 无意识结构精确定义——遭遇 = 被迫，不是自动晋升 |
| 430号观察4 | depends_on 历时性/共时性区分——本谱系的两轴对应是同一框架的另一面 |

## 8. 影响声明

- **新增**：431号谱系（本文件）
- **代码变更**：traversal.py（`_check_articulation_encounter` 重写）、daemon.py（articulation block 格式）、block_topology_persistence.py（`write_articulation_block` 接口）
- **常量移除**：`ARTICULATION_THRESHOLD = 2.0` 已删除
- **判据类型变更**：从度量（score >= threshold）到拓扑（布尔不一致）

## 推导链

```
425号 Phase 2: ARTICULATE 实装（score = cooc_weight * ta_count / step_gap >= 2.0）
  ↓ 编排者否定：阈值不够内在（"谁决定了乘积大于多少算涌现？"）
  ↓ 编排者观察：索绪尔两轴 = Layer A/B，两轴交叉点 = 遭遇位置
  ↓ 编排者观察：语料摄入后 A/B 密度失衡 = 遭遇候选信号
431号: 拓扑不一致判据（A密B疏 / B密A疏，布尔）
```

**驱动力分析**：从度量到拓扑的转变不是"优化阈值"，是判据类型的范畴变更。度量判据（多少）预设了一个外部标准；拓扑判据（有/无）从结构本身推导。这与 231号（有效域规则）同构——度量阈值的有效域依赖于数据分布，拓扑判据的有效域等于定义域。
