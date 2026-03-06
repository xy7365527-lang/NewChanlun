# R3 第二阶段：fold/negate 的内禀拓扑判据——从实验数据到数学构造

背景：R3 第一阶段建立了 fold = 分层Morse理论（Δβ₁ = (c-1) + n_loop），negate = Cerf分岔，sublate = 不稳定流形重构。实验：2524顶点真实图2000步穿越，43次成功fold，122次negate，116个settled环。

## Q-R3-7（代数分析）：Δβ₁ 作为 fold 判据

给定图 G = (V, E) 和两个顶点 v, w，定义 f(v,w) = (c-1) + n_loop（折叠{v,w}的预测Δβ₁，其中 c = 下链接 l⁻(p) 的连通分量数，n_loop = {v,w} 内部边数）。

问题：
1. f(v,w) = 0 的充要条件是什么？
2. f(v,w) 和 v、w 的邻域交集/对称差之间的精确关系？能否证明 f(v,w) ≤ |N(v) △ N(w)| - 1 + n_loop？
3. f 作为 V×V → ℤ 的函数是否满足离散 Morse 函数的类似物？

## Q-R3-8（代数分析）：独立路径数作为 negate 判据

定义 g(v,w) = v 到 w 的 vertex-disjoint paths 数（= min vertex cut by Menger）。

问题：
1. 添加否定边 w→v 后的 Δβ₁ 和 g(v,w) 之间什么关系？
2. g(v,w) 是否影响否定后环的稳健性？
3. g 和 f 之间是否有对偶/反相关关系？

## Q-R3-9（理论构造）：统一操作势函数

候选：Φ(v,w) = f(v,w) - λ·g(v,w)。Φ < 0 倾向 fold，Φ > 0 倾向 negate。

问题：
1. λ 是否有纯拓扑内禀定义？
2. Φ 的梯度流是否引导穿越到 fold 候选？
3. Φ 的极值点集合的拓扑性质？

## Q-R3-10（实验预测）

预测1：成功fold的平均 f(v,w) < 随机对的平均 f(v,w)
预测2：negate的平均 g(v,w) > 随机对的平均 g(v,w)
预测3：fold对和negate对在(f,g)空间中占据不同区域

所有分析在 L0 完成。Q-R3-10 由 CC 在数据上验证。
