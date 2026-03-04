#!/usr/bin/env python3
"""Gemini 3.1 Pro: design unified recursive topology scheme (seeing Codex's work)."""

from google import genai

client = genai.Client(api_key="AIzaSyD8eB39hxIxqsEk6sIwSpuD4nAc7OgufdQ")

PROMPT = r"""你是一个严格的代数拓扑学家。你之前独立设计了缠论拓扑化方案（Extended PH + 层理论 + 离散 Morse，5个定理）。

现在 Codex 5.3 也独立设计了方案。以下是 Codex 方案的关键差异和 6 个定理。

## Codex 5.3 方案（关键差异）

### Codex 的核心洞察：中枢 zigzag 模
Codex 指出：**标准 sublevel H₀ 无法严格刻画中枢**——中枢是"λ 同时落在多个 I_j"的层值(interlevel)约束，活跃集合随 λ 非单调，非一参数单调滤子对象。

因此 Codex 定义了**中枢 zigzag 模**：
- 对 λ∈ℝ，定义活跃笔图 G_λ：顶点 j ⟺ λ∈I_j，边 (j,j+1) ⟺ λ∈I_j∩I_{j+1}
- 取 zigzag 序列 G_{μ₁} ↔ G_{μ₂} ↔ ⋯ ↔ G_{μ_{L-1}}
- 诱导 Z_f: r ↦ H₀(G_{μ_r}; k)
- 由 Crawley-Boevey 分解为区间模条形码 Dgm₀^{zz}(f)

### Codex 6 个定理

**定理1（Sublevel 0D 稳定性）**
d_B(Dgm₀^{sub}(f), Dgm₀^{sub}(g)) ≤ ‖f-g‖_∞

**定理2（三笔中枢与 zigzag 条带一一对应）**
对每个 j，C_j ≠ ∅ ⟺ 存在唯一极大 3-支撑条带 J_j，且 J_j = C_j = [ZD_j, ZG_j]

**定理3（中枢条带稳定性）**
d_B(Dgm₀^{zz}(f), Dgm₀^{zz}(g)) ≤ 2‖f-g‖_∞

**命题4（层截面-中枢刻画）**
dim_k P(τ_j) = |T ∩ C_j|（三角胞腔 stalk 维数 = 中枢宽度）

**定理5（级别一致性 / Leray）**
若 R¹π_*P_{ℓ+1} = 0，则 H^q(X_ℓ, P_ℓ) ≅ H^q(X_{ℓ+1}, P_{ℓ+1}), q=0,1

**定理6（Morse 阈值筛选正确性）**
取消全部 pers < τ 的配对后，剩余临界点与 pers ≥ τ 配对一一对应
若 A_{m₁} ⊆ A_{m₂}, τ_{m₁} ≥ τ_{m₂}，则 |Crit(m₁)| ≥ |Crit(m₂)|

### Codex 统一流程
(f,K) →[Morse简化] K^{(τ,m)} →[提笔] {I_j} → { zigzag H₀ → Dgm₀^{zz} ; 层 P → (H⁰,H¹) }

---

## 亏格构造已被双方否定

编排者之前提出"走势曲面 Σ(W) = tubular neighborhood，g = n_centers"的亏格构造。
你和 Codex 都独立严格否定了它：
- ℝ² 管状邻域边界是 1 维的
- 实心矩形不产生亏格
- π₁ 呈示用错对象
结论：亏格构造不可用。

---

## 你的任务

缠论是**递归的**——级别递归 F₀ ⊂ F₁ ⊂ F₂ ...

**设计一个最严格的、统一的递归拓扑方案。** 要求：

### 1. 综合你和 Codex 的方案
你的方案有 Extended PH + 谱序列收敛定理。Codex 有 zigzag PH + Leray 级别一致性。
**给出统一方案**——不是列出两套，而是严格说明哪些定理保留、哪些冗余、哪些互补。

### 2. 递归结构的严格处理
缠论的递归是：第 k 级的走势类型 = 第 k+1 级的"笔"。
这在数学上对应什么？给出严格定义：
- 递归映射 φ_k: X_k → X_{k+1} 的精确构造
- φ_k 如何与 PH / 层 / Morse 交互？
- 递归层级之间的不变量关系是什么？

### 3. 递归不变量
哪些不变量在递归层级变化时保持？哪些不保持？
给出严格命题。

### 4. Python 模块设计
统一的计算架构（类名、方法名、输入输出类型）。
支持递归——能在任意级别 k 上计算所有不变量。

### 5. 严格定理（至少 5 个）
每个定理必须：精确陈述 + 证明草案 + 计算复杂度 + 缠论解释
**特别要求**：至少 2 个定理直接涉及递归（跨级别关系）。

**不允许"类比"、"精神一致"等模糊表述。
如果某处无法严格化，明确说"此处无法严格化，原因是..."
Codex 5.3 同时在设计同一方案。你们的独立设计之后会对比。**
"""

print("Calling Gemini 3.1 Pro: unified recursive topology scheme...")
response = client.models.generate_content(
    model="gemini-3.1-pro-preview",
    contents=PROMPT,
)
print(response.text)
