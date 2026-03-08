---
id: '393'
number: 393
title: "fold猜想初始图验证——f小=fold成立 + Jaccard 44x + f=0充分条件 + 392号P1修正"
type: correction
status: settled
date: 2026-03-06
source: CC 直接数据分析（初始图 vs 变化后图的测量区分）
depends_on:
  - '391'
  - '392'
epistemological_level: L2
---

# 393号：fold 猜想初始图验证——392号P1否定是测量错误

## 结论

原始猜想"f(v,w)小 = fold候选"在**初始图**上成立。392号的P1否定（fold f=10.21 > random f=7.45）是因为在引擎操作**变化后的图**上计算f——图被fold/negate/sublate改变后，fold对的邻域结构已不同于初始状态。

在初始图上重新计算：

| 指标 | fold (n=45) | negate (n=83) | random邻居 (n=100) |
|------|-------------|---------------|---------------------|
| Jaccard | **0.622** | 0.140 | 0.014 |
| c (下链接连通分量) | 3.56 | 13.36 | 7.32 |
| f (预测Δβ₁) | **2.93** | 15.17 | 7.46 |
| 交集大小 | **10.62** | 4.52 | 0.31 |
| f=0 比例 | **40% (18/45)** | — | **0% (0/100)** |

三个区间确认：f低(fold,2.93) → f中(random,7.46) → f高(negate,15.17)。

## 定义依据

- f(v,w) = (c-1) + n_loop，391号Q-R3-7
- Jaccard = |N(v)∩N(w)| / |N(v)∪N(w)|
- 45个fold对、83个negate对从500步实验（seed=42）中提取
- 100个随机邻居对从最大连通分量中采样

## 边界条件

1. 初始图上的f值和变化后图上的f值不同——引擎操作改变拓扑后，同一对顶点的邻域结构变化
2. 测量必须在初始图上进行，不是实时图上（392号的错误根因）
3. 45个fold对可能不够大——需要更多步的实验增加样本量
4. Jaccard=0.622的阈值是否是fold的必要条件？可能有低Jaccard的fold——需要看f=0但Jaccard低的对是否存在

## 下游推论

1. **392号P1修正**：P1从"否定"修正为"支持"——f小=fold在初始图上成立 [audited: v198, consumed]
2. **f=0是fold的强充分条件**：40% fold对f=0，0% random对f=0。f⁻¹(0)是fold候选的过滤器 [audited: v198, consumed — 方法论记录]
3. **Jaccard是fold最强的单指标**：0.622 vs 0.014 = 44倍效应。邻域重叠度直接编码结构等价性 [audited: v198, consumed — 方法论记录]
4. **negate_a规则可以用f值替代Morse critical过滤**：f>阈值 → negate候选，f<阈值 → fold候选。阈值由f的分布自然确定（如f=5） [audited: v198, deferred — 未实施替代]
5. **测量时机原则**：拓扑指标必须在操作前的图上计算，不在操作后的图上计算 [audited: v198, consumed — 方法论记录]

## 谱系引用

- 391号：f/g代数分析（f=0充要条件、f≤g-1不等式）
- 392号：数据验证（P1否定——本号修正为测量错误）
- 388号：negate_a架构bug修正（Morse critical过滤——可能被f值替代）

## 影响声明

修正392号P1结论。确认原始猜想在初始图上成立。新增Jaccard作为fold最强单指标。确认f=0是fold强充分条件。建议negate_a规则可用f值替代Morse critical过滤。
