#!/usr/bin/env python3
"""Call Gemini 3.1 Pro Preview (thinking) via new google-genai SDK."""

from google import genai

client = genai.Client(api_key="AIzaSyD8eB39hxIxqsEk6sIwSpuD4nAc7OgufdQ")

AUDIT_PROMPT = r"""你是一个严格的数学审计员。以下是一个代码项目对缠论（一种技术分析理论）实现的拓扑语义标注。

你的任务：逐个审计每个模块的拓扑标注，判断数学准确性和边界诚实度。

评级标准：
- PASS：结构映射准确，映射的边界诚实，无遗漏
- WARN：方向正确但有术语不精确或边界描述不完整
- FAIL：数学错误、概念滥用、或边界不诚实

**重要**：请区分"标注已承认的限度"和"标注未声明的错误"。如果标注在"映射的边界"中已明确否定某概念，而你的质疑恰好指向该已否定的内容，则不应判 FAIL。

请逐模块给出评级和理由。

## 模块1: a_inclusion.py — 商空间

### 结构映射
包含处理是带邻接约束的区间包含等价类的商空间构造。

- raw K线序列 X 上的偏序关系 ≤ 由"包含"定义：a ≤ b ⟺ [a.low, a.high] ⊆ [b.low, b.high]
- 等价关系 ~：x ~ y ⟺ x 与 y 有包含关系且相邻（传递闭包后形成等价类）
- merge_inclusion() 是商映射 π: X → X/~，将每个等价类映射到一个 merged bar
- merged_to_raw 是商映射的纤维结构：记录每个 merged bar 对应的 raw 范围
- 方向状态 dir_state 决定"向上取并/向下取交"——等价类代表元的极值选择规则

### 映射的边界
商映射 π 是精确的——merge_inclusion 确实计算了等价类的代表元。
但不应称 Alexandrov 拓扑——Alexandrov 拓扑在有限离散序列上是平凡的（每个点既开又闭）。
真正的拓扑内容在于商映射的纤维结构（merged_to_raw = π 的逐纤维分解），不在开集。

---

## 模块2: a_fractal.py — Morse 临界点一维类比

### 结构映射
分型是一维价格函数的局部极值点——Morse 临界点的离散一维类比。

- merged bar 序列上的价格函数 f: {merged bars} → ℝ 是离散函数
- 顶分型 = 局部极大值点（index-1 临界点的类比）
- 底分型 = 局部极小值点（index-0 临界点的类比）
- 双条件（h > h_prev AND h > h_next AND l > l_prev AND l > l_next）= 严格局部极值条件

### 映射的边界
这里的"Morse"是连续 Morse 理论的一维类比，不是 Forman 离散 Morse 理论的精确实例。Forman 理论需要 CW 复形上的梯度向量场配对结构，分型检测不具备这个结构。类比的有效部分：严格不等式条件排除了退化，和 Morse 非退化条件精神一致。

---

## 模块3: a_stroke.py — CW 1-cell

### 结构映射
笔是 CW 复形的 1-cell 构造。

- 分型 = 0-cell（CW 复形的顶点集）
- 笔 = 1-cell（连接两个 0-cell 的边）
- 胶合映射 φ: ∂D¹ → X⁰ 由 (i0, i1) 定义
- 连续性保证 strokes[i].i1 == strokes[i+1].i0 = CW 胶合条件
- 顶底交替 = 定向性

### 映射的边界
CW 1-cell 类比精确。限度：CW 复形允许自环，笔不允许（顶底交替）。这是 CW 复形的受限特例——定向 1 维 CW 复形。

---

## 模块4: a_segment_v1.py — 1-chain / 子复形

### 结构映射
线段是1维CW复形上的子复形（1-chain）。

- 笔序列构成 1 维 CW 复形 X¹
- 线段 = X¹ 的子复形 σ = {连续笔链}
- 断段操作 = 子复形的分割（subdivision）

### 映射的边界
线段不是2-cell——整个构造在1维骨架上操作。特征序列的"分型"和K线分型不是同一层。

---

## 模块5: a_center_v0.py — 闭区间族的有限交

### 结构映射
中枢是闭区间族的有限交——三段价格区间的公共交集。

- ZG = min(g₁, g₂), ZD = max(d₁, d₂) = 交集的精确计算
- 三段重叠条件 = 三个闭区间的非空交集

### 映射的边界
中枢是 3 段闭区间的交集——不是 FIP 的一般实例。称紧致性是过度命名。准确描述：闭区间族的构造性有限交。

---

## 模块6: a_trendtype_v0.py — 路径空间组合分类

### 结构映射
走势类型是路径空间上的有限组合分类。
- 走势 = 从起点到终点的有向路径
- 走势类型由中枢组合数据决定

### 映射的边界
走势不是环路。π₁ 不直接适用。分类是有限组合分类，不经过代数拓扑。辫群是可能更精确的类比但未展开。

---

## 模块7: a_recursive_engine.py — 分层构造

### 结构映射
级别递归是自下而上的分层构造（stratification）。
- RecursiveLevel(k) = 第 k 层 stratum
- 分层是内禀的（由数据生长）

### 映射的边界
不是 Whitney 分层——Whitney 条件需要光滑流形。这是组合学意义上的层级分解。

---

## 模块8: a_topology.py — 转换函数框架

### 结构映射
- 分解空间 D(X) = {所有合法分解}
- 转换函数 T: D(X) → D(X) 是自映射
- 等价关系 A ~ B ⟺ ∃T: T(A) = B
- 不变量 I: D(X)/~ → V 是商映射

### 映射的边界
转换函数通过运行两次管线隐式定义，不是显式同构。不变量是候选，不是已证明的定理。

---

请按以下格式输出：

对每个模块：
### 模块N: [文件名] — 评级: [PASS/WARN/FAIL]
- 结构映射评价: ...
- 映射的边界评价: ...
- 具体问题（如有）: ...

最后给出汇总表和总评。"""

print("Calling Gemini 3.1 Pro Preview (thinking model)...")
response = client.models.generate_content(
    model="gemini-3.1-pro-preview",
    contents=AUDIT_PROMPT,
)
print(response.text)
