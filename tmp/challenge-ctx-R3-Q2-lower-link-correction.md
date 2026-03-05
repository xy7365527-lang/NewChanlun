# R3-Q2 第三轮：下链接公式精确化

## 上一轮结论（R2 摘要）

上一轮（R2）建立了以下框架：
- Goresky-MacPherson 主定理：$\Delta H_k \cong \tilde{H}_{k-1}(l^-(p); \mathbb{Z})$
- 下链接 $l^-(p) = \{x \in l(p) \mid f(x) < f(p)\}$
- R2 对最小例子 A→B→C 折叠 A∼C 的计算：
  - 折叠点 $v = [AC]$ 的下链接 $l^-(v) = \emptyset$（极小值，$\Delta\beta_0 = +1$）
  - $B$ 的下链接 $l^-(B) = \{q_1, q_2\}$（两个离散点，$\tilde{H}_0 = \mathbb{Z}$，$\Delta\beta_1 = +1$）
  - 法锥拓扑效力完全由整数 $m = |l^-(p)|$ 决定
  - **R2 简化公式**：$\Delta\beta_1 = m - 1$（$m \geq 2$ 时）

## 编排者的精确化修正（本轮输入）

**修正1：下链接可以有边**

R2 的计算默认 $l^-(p)$ 是点集（$m$ 个离散点）。这在最简单情形下成立，但一般情况下**下链接可以有边**。

具体场景：折叠点的某些邻居之间彼此也相连（例如：两个区块都被第三个 depends_on，它们之间还有 reference 关系）。在这种情况下，$l^-(p)$ 不是离散点集，而是一个有边的图（1 维单纯复形）。

**后果**：
- $l^-(p)$ 有 $m$ 个顶点，但不是 $m$ 个独立点
- 设 $l^-(p)$ 有 $c$ 个连通分量（$c \leq m$）
- $\tilde{H}_0(l^-(p)) = \mathbb{Z}^{c-1}$（缩减 0 阶同调秩 = $c - 1$，不是 $m - 1$）
- 因此 **$\Delta\beta_1 = \text{rank}(\tilde{H}_0(l^-(p))) = c - 1$**（不是 $m - 1$）

**修正1 的正确公式**：
$$\Delta\beta_1 = \text{rank}(\tilde{H}_0(l^-(p)))$$

其中 $\text{rank}(\tilde{H}_0) = c - 1$，$c$ 是 $l^-(p)$ 的连通分量数。

**修正2：自环是规则不是例外**

R2 在边界条件中提到"若识别顶点时产生自环边，度数计算需减去这些边"——这是正确的，但处理方式不够。自环不是"需要减去的例外"，而是**最常见的折叠场景**。

具体场景：被折叠的两个顶点之间如果有 supersedes/modifies 边（A supersedes B，折叠 A∼B），折叠后 A→B 变为 $v \to v$ 即**自环**。

**后果**：
- 自环 $v \to v$ 在链复形中满足 $\partial(e_\text{loop}) = v - v = 0$
- 即 $e_\text{loop} \in \ker(\partial_1)$，对 $H_1$ 的贡献为 $+1$
- **每条自环贡献 $+1$ 到 $\beta_1$**，独立于下链接的贡献

**修正2 的正确公式**：自环贡献必须整合进总公式，不能"单独处理"。

## 本轮需要推导的问题

### 问题1：完整公式

结合修正1和修正2，给出折叠操作的完整 $\Delta\beta_1$ 公式：

$$\Delta\beta_1 = \text{rank}(\tilde{H}_0(l^-(p))) + (\text{自环数})$$

需要展开：
- $\tilde{H}_0(l^-(p))$ 的秩与连通分量数 $c$ 的关系
- 自环数的计算方式（从折叠前的边集）
- 完整公式的严格推导

### 问题2：两个最小例子的完整计算

**例1（下链接有边）**：

图：A→B→C，且 B→C 有额外 reference 边（即 B 和 C 之间有两条边，或 B→C 是 reference，B 和 C 同时都被 A depends_on）。

实际简化：A→B，A→C，B→C（A depends_on B，A depends_on C，B reference C）。折叠 A∼C。

- 折叠点 $v = [AC]$
- 折叠后：$v \to B$（原 A→B），$v \to v$（原 A→C 变自环？不对——A→C 是 A 到 C，C 被识别为 $v$，所以 A→C 变为 $v \to v$）
- 等等，需要更仔细分析

编排者指定的例1：
> A→B→C，B→C有reference边，折叠A∼C。下链接 l⁻(p) 有两个顶点且它们之间有边——c=1，Δβ₁=0（不是m-1=1）

请重新分析：
- 图结构：A→B（depends_on），B→C（depends_on），B→C（reference）—— 这会产生多重边？
- 或者：A→C（depends_on），B→C（depends_on），B→C（reference）
- 折叠 A∼C，折叠点 $v = [AC]$
- 下链接 $l^-(v)$：在折叠后图中，$f(x) < f(v)$ 的邻居——如果 $v$ 是最小值则 $l^-(v) = \emptyset$
- 但编排者说"下链接 l⁻(p) 有两个顶点且它们之间有边，c=1，Δβ₁=0"——这意味着 $l^-(v)$ 不为空

需要 Gemini 澄清：编排者的例1中，折叠点不是极小值，而是某个中间点。Morse 函数 $f$ 如何定义使得 $l^-(v)$ 有两个顶点且有边。

**例2（自环）**：

图：A→B，A supersedes B（即有一条 supersedes 边 A→B）。折叠 A∼B。

- 折叠后 $v = [AB]$
- 原边 A→B（supersedes）折叠后变为 $v \to v$（自环）
- 原边 A→B（depends_on，如果有）也变为自环？
- 每条 A→B 类型的边都变为自环，每条自环贡献 $+1$ 到 $\beta_1$
- 编排者说：Δβ₁=+1（来自自环，不来自下链接）

### 问题3：L2 验证的预测值公式

给定 relations.jsonl 和一个折叠事件（指定哪两个节点被识别），如何计算预测的 $\Delta\beta_1$？

需要给出：
1. 识别自环数的算法（在折叠前的边集中找 A→B 且 A 和 B 都在折叠集中的边）
2. 识别下链接 $l^-(p)$ 的算法（在折叠后图中找 $f(x) < f(p)$ 的邻居，需要先定义 Morse 函数 $f$）
3. 计算 $l^-(p)$ 的连通分量数 $c$ 的算法
4. 完整预测公式：$\Delta\beta_1^{\text{pred}} = (c-1) + n_\text{loop}$

## 约束

- 纯代数拓扑框架，不引入质料域术语
- 区块是抽象节点，关系是有类型有向边
- 认识论等级标注（L0 = 代数推导，L1 = 合成验证，L2 = 真实数据验证）
- 产出 ≤ 8KB

## 上下文补充

**relations.jsonl 的边类型**（从 block-topology 目录推断）：
- depends_on
- supersedes
- modifies
- reference
- 其他（需要 Gemini 从文件读取确认）

**折叠场景**（最相关的）：
- 同一概念的新旧版本（A supersedes B）→ A∼B 折叠 → 必然产生自环
- 等价概念（A modifies B）→ A∼B 折叠 → 可能产生自环
