---
id: '388'
number: 388
title: "negate_a 两个架构 bug 修正——自引用振荡 + Morse critical 过滤"
type: correction
status: 已结算
date: 2026-03-05
session: v168
source: v167-swarm/v168-swarm 实验（Phase 3 调试过程）
negation_source: homogeneous
negation_form: expansion
topo_effect: "split:negate_a:local — negate_a 原始实现分裂为两个：一个保留bidirectional检测框架，一个携带自引用振荡+hub效应两条违反记录"
depends_on: []
epistemological_level: L2
---

# 388号：negate_a 两个架构 bug 修正——自引用振荡 + Morse critical 过滤

**认识论等级**: L2（在真实谱系图2524顶点上发现并验证修正效果）

## 1. 结论

negate_a 操作的矛盾检测中存在两个架构级别的 bug，导致 69% 的检测结果是 false positive。

### Bug 1：自引用振荡

**问题**：引擎在 fold 过程中创建 syn_/anti_ 前缀的合成顶点，这些顶点与原始图的邻居产生 bidirectional 边，被后续的矛盾检测误判为新矛盾。本质是引擎在与自己的产物对话——69% 的操作属于此类。

**修正**：在矛盾检测时排除 syn_/anti_ 前缀顶点对。不参与矛盾检测的合成顶点仍保留在图中（它们是 fold 的合法产物）。

**效果**：84 次原始矛盾检测 → 7 次（-92%）

### Bug 2：Hub 效应（bidirectional ≠ 矛盾）

**问题**：bidirectional 边（A→B 且 B→A）被无条件判定为矛盾。但在 hub 节点（如 005b、010 等度≥4 的节点）上，bidirectional 边是网络拓扑的必然结果——两个节点互相引用不等于互相矛盾。

**编排者洞察**："K参数化不够内在"——最初的修正方案是用 K 阈值过滤 hub 节点的 bidirectional 边（度≥K 的节点跳过），但编排者否定了这一方案，因为 K 是外部参数，不来自拓扑内禀结构。

**修正**：使用 Morse terrain 的 critical/tree 标记替代 K 参数：
- 两条边都是 tree 边 → hub 内部冗余 → 跳过
- 至少一条是 critical 边 → 参与不可缩环 → 保留为矛盾候选

**关键性质**：零参数。critical/tree 分类完全来自 Discrete Morse Function 的梯度流结构，不依赖任何人工阈值。

### 综合效果

| 指标 | 修正前 | 修正后 |
|------|--------|--------|
| encounter 密度 | 71.4% | 62.0% |
| walk 步数 | 143 | 190 |
| settled cycles | 44 | 47 |
| synthetic negation | 84 | 7 |

## 2. 定义依据

- **Discrete Morse Function**（351号研究线）：每条边被梯度流标记为 critical（参与不可缩环路）或 tree（属于梯度树，可收缩）。这一分类是拓扑内禀的——同一图上不同的 DMF 会给出不同分类，但 critical 边的数量（β₁）是不变量
- **Hub 节点**（019d张力检查范围策略）：度≥4 的跨域桥接器。在谱系图中，hub 节点的 bidirectional 边是引用网络的自然产物，不携带矛盾语义

## 3. 边界条件

1. **syn_/anti_ 前缀排除是硬编码规则**：如果未来引擎的合成顶点命名约定改变，此过滤会失效。应由引擎维护一个"合成顶点集合"而非依赖前缀匹配
2. **Morse critical/tree 分类依赖 DMF 选择**：不同的 DMF 会给出不同的 critical/tree 分类。当前使用的 DMF 是否最优未经验证
3. **62% encounter 密度是否仍偏高**：7次 synthetic negation 中可能仍有 false positive。需要人工审核确认
4. **修正与384号收敛**：自引用振荡 bug 与384号中 β₁ 公式的自环项问题形式上同源——都是"系统与自身产物交互"的模式

## 4. 下游推论

1. **矛盾检测的零参数原则**：编排者否定 K 参数化后确立的原则——拓扑计算引擎的所有判断应来自拓扑内禀结构（如 Morse critical/tree），不依赖外部参数
2. **fold 质量依赖 negate_a 质量**：387号的实验结果（59次fold成功）建立在本修正之上。未修正时的 fold 质量不可信
3. **自引用振荡模式的泛化**：任何在图上做增量修改的操作都可能产生"与自身产物交互"的 false positive。这是拓扑计算引擎的通用设计约束

## 5. 谱系引用

- 384号：β₁公式修正中的自环项问题——与 Bug 1 形式同源（系统与自身产物交互）
- 351号：Morse 理论研究线——Bug 2 修正使用了 Morse terrain 的 critical/tree 分类
- 370号：Morse 有效域边界——critical/tree 分类的有效域问题（370号结论在此处被正面使用而非否定）
- 387号：Phase 3 真实谱系穿越——本修正直接影响387号的实验数据质量

## 6. 影响声明

- **拓扑计算引擎 negate_a 模块**：两处修正（合成顶点排除 + Morse critical 过滤）
- **零参数设计原则**：编排者否定 K 参数化后确立——所有拓扑判断来自内禀结构
- **387号实验数据**：本修正是387号数据可信性的前提条件
