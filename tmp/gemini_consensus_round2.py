#!/usr/bin/env python3
"""Gemini 3.1 Pro: Round 2 — 看到 Codex 的评审意见，回应分歧，追求共识。"""

import os
from google import genai

client = genai.Client(api_key=os.environ.get("GOOGLE_API_KEY", "AIzaSyD8eB39hxIxqsEk6sIwSpuD4nAc7OgufdQ"))

PROMPT = r"""你是严格的代数拓扑学家。这是共识讨论的第二轮。

## Round 1 你的评审摘要
你评审了编排者的分层采纳决断，主要结论：
1. 分层决断 Layer 1/2/3 无懈可击
2. "条形码是亏格的精细化"——核心正确但"遗忘函子"用词错误，应为"去范畴化"
3. T8 验证需仿射规范化（同一纤维上比较）
4. δ_k gauge choice 由瓶颈稳定性定理保证
5. T1（Extended PH ≡ Zigzag）必须暗含在 Layer 1 的工程底层
6. 建议引入 scikit-tda/GUDHI 保证数学精确性

## Codex 5.3 的 Round 1 评审摘要

Codex 给出以下评审：

### 对编排者洞察的评审
1. **洞察1**：条件成立，但 g(Σ(W)) = #{bars > τ} - 1 公式不是普适的代数拓扑公式。经典闭曲面 g = β₁/2。需明确构造前提。
2. **洞察2**：工程价值判断合理。但"严格递减"太强，应降格为"带容差的单调下降"再检验。
3. **洞察3**：同意搁置 T6。
4. **洞察4**：核心正确。严格版本 = 稳定性定理 + "同一底空间/可比空间 + 有界扰动"条件。

### Codex 的关键补充
- **建议 T7（B_{k+1} ⊆ Trim(B_k)）也放进 Layer 1**——计算成本低，可早期发现递归崩坏
- **T8 的 failure modes**：分解边界抖动导致短条带爆发、波动率 regime shift、跨层不可比、W₁被噪声主导、标签前视偏差
- **"条形码→亏格"的范畴论表述**：映射链
  FiltTop → PersMod → Bar_p → ℕ → ℤ
  其中 N_τ(B) = #{I ∈ B : len(I) > τ}，φ(n) = n-1 或 n/2（取决于构造）
- **T6 搁置但保留为"未来规范接口"**

---

## 你需要回应的分歧点

1. **你说"去范畴化"，Codex说"因子分解映射链"**——这两个表述等价吗？有没有一个更精确？

2. **Codex 建议 T7 进 Layer 1**——你同意吗？你的原始评审没提到 T7。

3. **T8 验证：你说"仿射规范化"，Codex 列出5个 failure modes**——你们的验证前提条件一致吗？你是否同意 Codex 的 failure modes？需要补充吗？

4. **g 公式的普适性**：Codex 指出 g = #{bars > τ} - 1 不普适（经典 g = β₁/2），需明确构造前提。你同意吗？

5. **你建议引入 scikit-tda/GUDHI，Codex 没提**——这是工程建议还是数学必要性？

**目标：达成共识。** 对每个分歧点，明确说"同意"或"不同意+理由"。
如果你对 Codex 的所有评审都无异议，明确声明"我对 Codex 的评审无异议，双方共识达成"。
"""

print("Calling Gemini 3.1 Pro: consensus round 2...")
response = client.models.generate_content(
    model="gemini-3.1-pro-preview",
    contents=PROMPT,
)
print(response.text)
