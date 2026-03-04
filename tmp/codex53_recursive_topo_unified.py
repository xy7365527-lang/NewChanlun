#!/usr/bin/env python3
"""Codex 5.3 High: design unified recursive topology scheme (seeing Gemini's work)."""

import openai

client = openai.OpenAI(
    api_key="REDACTED"
)

PROMPT = r"""你是一个严格的代数拓扑学家。你之前独立设计了缠论拓扑化方案（zigzag PH + 层理论 + 离散 Morse，6个定理）。

现在 Gemini 3.1 Pro 也独立设计了方案。以下是 Gemini 方案的关键差异和 5 个定理。

## Gemini 3.1 Pro 方案（关键差异）

### Gemini 的核心洞察：Extended PH + 谱序列
Gemini 使用 **Extended Persistent Homology**（升序 sublevel + 降序 superlevel 相对同调），
声称这比标准 sublevel 更完整地捕捉极大值和极小值。

Gemini 构造了**Alexandroff 拓扑**作为层的底空间，
并使用**谱序列收敛定理**处理级别递归。

### Gemini 5 个定理

**定理1（DMT分型等价定理）**
Forman 离散向量场 V 的临界顶点集 Crit₀(V) 精确等价于缠论底分型集合，
Crit₁(V) 精确等价于顶分型集合。计算 O(N)。

**定理2（持久度过滤下的笔模式同构）**
存在临界值 δ_strict，使得 Crit^{δ_strict}_*(V) 在单纯同调意义下同构于
"老笔（Strict Pattern）"生成的 CW 复形顶点集。O(N log N)。

**定理3（中枢的层上同调阻碍定理）**
由 e₁+e₂+e₃ 构成的链 c 构成中枢的必要条件是：
H¹(X_c, F) ≠ 0（非平凡的拓扑扭曲）。
趋势 ⟺ H¹=0，中枢 ⟺ H¹≠0。

**定理4（不同级别递归的谱序列收敛定理）**
设 X₀ ⊂ X₁ ⊂ X₂ ⊂ ... 为级别对应的 CW 复形滤子。
存在第一象限谱序列 E_r^{p,q}，E₁ 页为 H^q(X_p, F_p)，
收敛于 H^{p+q}(X_∞, F_∞)。由于 1D 复形，谱序列在 E₂ 退化。O(M³)。

**定理5（强弱不变量分类与稳定性）**
若 ‖f-f'‖_∞ ≤ ε 且存在 (b,d) 使得 d-b > 2ε，
则该 H₀/H₁ 生成元在所有笔模式中保持存在。

### Gemini 范畴论统一
- 分类空间函子 R：时间序列 → Reeb Graph（Extended PH 驱动）
- 层化函子 S：Reeb Graph → 层，H¹ 识别中枢
- Morse 简化函子 M_δ：CW → CW 自函子

---

## 亏格构造已被双方否定

编排者之前提出"走势曲面 Σ(W)"亏格构造，你和 Gemini 都严格否定了它。不再考虑。

---

## 你的任务

缠论是**递归的**——级别递归 F₀ ⊂ F₁ ⊂ F₂ ...
第 k 级的走势类型 = 第 k+1 级的"笔"。

**设计一个最严格的、统一的递归拓扑方案。** 要求：

### 1. 综合你和 Gemini 的方案
你有 zigzag PH + Leray 级别一致性。Gemini 有 Extended PH + 谱序列收敛。
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
Gemini 3.1 Pro 同时在设计同一方案。你们的独立设计之后会对比。**
"""

print("Calling Codex 5.3 High: unified recursive topology scheme...")
response = client.responses.create(
    model="gpt-5.3-codex",
    reasoning={"effort": "high"},
    input=PROMPT,
)
for item in response.output:
    if item.type == "message":
        for content in item.content:
            if content.type == "output_text":
                print(content.text)
