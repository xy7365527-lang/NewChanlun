#!/usr/bin/env python3
"""Gemini 3.1 Pro: verify editor's genus construction + recursive topology."""

import os

from google import genai

client = genai.Client(api_key=os.environ["GOOGLE_API_KEY"])

PROMPT = r"""你是一个严格的代数拓扑学家。请对以下**编排者提出的亏格构造**进行严格数学验证。
不允许"类比"、"精神一致"等模糊表述。如果某处有错，直说"这里有错，原因是..."

## 编排者的亏格构造

### 背景
我们正在将缠论（一种股票技术分析理论）严格拓扑化。已有的代码实现了：
- 0-cell = 分型（局部极值点），1-cell = 笔（连接相邻分型的有向边）
- 线段 = 1-chain（笔的子链）
- 中枢 = 三段闭区间有限交 [ZD, ZG]，其中 ZD = max(d₁, d₂), ZG = min(g₁, g₂)
- 走势类型 = 趋势（≥2 同向中枢）或盘整（1 中枢）
- 级别递归 = 滤子 F₀ ⊂ F₁ ⊂ F₂ ...

之前（Gemini审计中）我们否定了 π₁ 在此场景的应用，理由是"走势不是环路"。
编排者现在修正这个判断：

### 走势曲面 Σ(W) 的构造

在 (t, p) 平面上：

1. 价格路径是一条从 (t₀, p₀) 到 (t_n, p_n) 的折线（有限段线性插值）
2. 每个中枢 [ZD_k, ZG_k] 在时间轴上对应一个矩形区域 R_k = [t_k^start, t_k^end] × [ZD_k, ZG_k]
3. 定义走势曲面：
   Σ(W) = price_path ∪ (∪_k R_k) 在 (t,p) 平面中的管状邻域 (tubular neighborhood)
   即取这些集合的 ε-邻域后取闭包的边界

**关键声明**：
- 每个中枢矩形 R_k 为曲面增加一个环柄（handle）
- 因此 g(Σ(W)) = n_centers（中枢数）对于趋势走势
- 盘整只有 1 个中枢 → g = 1（亏格为 1 的环面）
- 无中枢（纯单向运动）→ g = 0（球面）

### π₁ 分类

- π₁(Σ_g) = ⟨a₁,b₁,...,a_g,b_g | [a₁,b₁]⋯[a_g,b_g] = 1⟩
- 每对 (a_k, b_k) 对应一个中枢
- a_k = 绕中枢的价格方向环路
- b_k = 绕中枢的时间方向环路
- 走势类型 = 亏格 = ½ rank π₁(Σ(W))

### 亏格作为 gauge invariant

声称：亏格是最强的 gauge invariant——在不同笔模式（wide/strict/new）下保持。
理由：不同笔模式改变笔的数量和端点位置，但中枢的数量和区间关系（哪些笔区间有非空交集）在合理的笔模式变化下保持不变。

## 你的任务

### 1. 构造的良定义性
Σ(W) = tubular neighborhood of (price_path ∪ ∪_k R_k) 是否给出一个良定义的紧致定向曲面（带边界）？
具体地：这个构造在 ℝ² 中的管状邻域，其边界是否是一个光滑（或分段线性）的闭曲面？

### 2. 亏格计算
g(Σ(W)) = n_centers 是否严格成立？
每个中枢矩形是否确实增加一个环柄？有没有例外情况（例如两个中枢矩形重叠时）？

### 3. π₁ 分类的严格性
π₁(Σ_g) 的呈示是否确实恢复了走势类型分类？
a_k, b_k 作为中枢的两个环路生成元——这在几何上是否有意义？

### 4. gauge invariance
亏格在不同笔模式下保持的声明是否可证明？
需要什么前提条件？在什么情况下可能失效？

### 5. 递归拓扑
缠论是递归的（级别递归 F₀ ⊂ F₁ ⊂ ...）。这个亏格构造如何与递归结合？
具体地：如果在第 k 级别构造 Σ_k(W)，那么 g(Σ_k) 和 g(Σ_{k+1}) 之间是否有严格的关系？

### 6. 与你之前设计的方案的关系
你之前设计了 Extended PH + 层理论 + 离散 Morse 的方案。亏格构造与这些方案是互补的、冗余的、还是矛盾的？

## Codex 5.3 同时在验证同一构造。给出独立的严格判断。

**如果构造有错，直接指出。如果某个声明需要额外条件才能成立，给出精确条件。
不允许"大致成立"——要么成立（给证明），要么不成立（给反例），要么需要条件（给条件）。**
"""

print("Calling Gemini 3.1 Pro: genus construction + recursive topology verification...")
response = client.models.generate_content(
    model="gemini-3.1-pro-preview",
    contents=PROMPT,
)
print(response.text)
