# 守恒约束（Conservation Constraint）

**状态**: verified-with-caveats
**来源**: `.chanlun/definitions/liuzhuan.md` v1.0
**溯源**: [新缠论]
**验证**: Gemini 2026-02-20

## 陈述

四矩阵中所有顶点的净流量之和为零。

形式化：

```
设 G=(V, E) 为完全图 K₄，V = {EQUITY, REAL_ESTATE, COMMODITY, CASH}
对于任意边 e={u,w} ∈ E，定义流函数 f(e,x) 满足反对称性：
  f(e,u) + f(e,w) = 0

每个顶点 v 的净流量 net(v) = Σ f(e,v), e ∈ E, v ∈ e

守恒约束：Σ net(v) = 0, 对所有 v ∈ V
```

## 证明（Gemini 验证通过）

```
Σ_{v∈V} net(v)
= Σ_{v∈V} Σ_{e∈E, v∈e} f(e,v)
= Σ_{e∈E} Σ_{v∈e} f(e,v)          （交换求和顺序）
= Σ_{e={u,w}∈E} (f(e,u) + f(e,w))
= Σ_{e∈E} 0                         （反对称性）
= 0                                  ∎
```

## Gemini 异质质询：三个语义缺陷

### 缺陷 1：量 vs 方向（CRITICAL）

定理证明的是**方向指标**的代数和为零，不是**资本量**的守恒。
flow ∈ {-1, 0, +1} 丢弃了 magnitude 信息。

反例：A→B 巨量资金（flow=-1），B→C 微量资金（flow=-1），C→A 中量资金（flow=-1）。
离散模型中 net(A)=net(B)=net(C)=0，看似平衡。
真实资本中 B 积累了巨额资本，A 严重亏空。

**结论**：此定理表达的是"没有未配对的流向关系"，不是"资本不凭空产生"。

### 缺陷 2：封闭系统同义反复（HIGH）

反对称性 f(e,u)+f(e,w)=0 使得 Σnet(v)=0 **恒成立**。
"守恒破缺"在此模型内是不可能事件。
要检测外部泄漏，需引入第五节点 v_ext 或允许 f(e,u)+f(e,w)≠0。

### 缺陷 3：硬编码度数（MEDIUM）

原始定义 net(V) = Σ flow(eᵢ, V), i=1..3 硬编码了 K₄ 度数。
已修正为 Σ_{e∈E, v∈e}。

## 影响评估

- `check_conservation` 函数：已删除（050号谱系结算，方案A）。函数在数学上恒返回 True，语义空洞。
- 026号消歧函数：不受影响（基于纯资产子图方差，不依赖守恒约束）。
- "守恒破缺"信号：需要重新定义——可能应该检测的是 magnitude 加权后的不平衡，而非方向指标的不平衡。
