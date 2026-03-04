#!/usr/bin/env python3
"""Codex 5.3 High: 评审编排者的分层采纳决断 + Gemini方案，给出意见。"""

import os
import openai

client = openai.OpenAI(
    api_key=os.environ.get("OPENAI_API_KEY", "REDACTED")
)

PROMPT = r"""你是严格的代数拓扑学家。你之前和 Gemini 3.1 Pro 各自独立设计了缠论统一递归拓扑方案。
现在编排者（对缠论有深刻理解的人类）给出了他的分层采纳决断。请严格评审。

## 你的方案摘要（6定理）
- T1: Extended PH ≡ Zigzag（模块同构）
- T2: Trend_k ≅ E(X_{k+1})（走势类型=上级笔自然双射）[递归]
- T3: 递归 PH 稳定性（d_B ≤ Σε_j）[递归]
- T4: Leray-谱序列递归收敛 [递归]
- T5: δ-Morse = 严格笔模式（阈值分离）
- T6: 中枢 H¹ 阻碍（条件版）
- 两个"无法严格化"点

## Gemini 3.1 Pro 方案摘要（5定理）
- T1: Morse-Zigzag 交换律
- T2: 中枢数 = dim H⁰(X_k, C_k)
- T3: Leray 级别一致性（R¹=0 → 层同构）[递归]
- T4: 递归条形码偏序（B_{k+1} ⊆ Trim(B_k)）[递归]
- T5: 背驰 = Wasserstein-1 严格递减

## 综合方案（8定理）

| # | 定理 | 来源 |
|---|------|------|
| T1 | Extended PH ≡ Zigzag | Codex |
| T2 | Morse-Zigzag 交换律 | Gemini |
| T3 | 中枢 ↔ zigzag 3-支撑条带 | Codex |
| T4 | δ-Morse = 严格笔模式 | Codex |
| T5 | Trend_k ≅ E(X_{k+1}) | Codex [递归] |
| T6 | Leray R¹=0 → 层同构 | 双方 [递归] |
| T7 | B_{k+1} ⊆ Trim(B_k) | 双方 [递归] |
| T8 | 背驰 = Wasserstein-1 递减 | Gemini [递归] |

---

## 编排者的分层采纳决断（核心——你需要严格评审这个）

### 洞察1：条形码是亏格的精细化

> "条形码才是严格工具——它不仅记录'有几个中枢'，还记录每个中枢的生命周期（区间 [b_i, d_i]）。
> 亏格（genus）= #{长寿命条形码条带} 是条形码的计数摘要。
> 精确关系：g(Σ(W)) = #{[b_i, d_i] | d_i - b_i > τ} - 1
> 条形码 → 亏格是遗忘函子（丢弃生命周期信息，只保留计数）。"

### 洞察2：T8（背驰 = Wasserstein-1 递减）是最有价值的定理

> "如果 T8 在真实数据上被验证，它将是一个 killer feature：
> 结构性背驰判据，不依赖 MACD/RSI 等指标，纯粹由拓扑递归映射定义。
> 但需要先在已知背驰的真实行情数据上验证。"

### 洞察3：T6（Leray R¹=0）是过早的

> "T6 要求指定层的底空间、stalk、限制映射。
> 当前代码还没有这些概念的实现。
> 没有工程需求驱动——R¹=0 判据在当前阶段没有可计算的目标。"

### 洞察4：δ_k 从哪来？

> "三算子方案的真正问题：Morse 阈值 δ_k 是一个参数 = gauge choice。
> 条形码中长寿命的部分（长度 > 2Σε_j）是 gauge invariant，
> 短寿命的不是。这和001号谱系的发现一致：不同参数产出不同分解，
> 但大结构（中枢数、趋势方向）保持。"

### 分层采纳决断

**第一层（现在做）: T3 + T4 + T5**
- T3: 中枢与 zigzag 条带一一对应
- T4: δ-Morse 与严格笔模式一致
- T5: Trend_k ≅ E(X_{k+1})（走势类型=上级笔递归同构）
- 在现有代码上实现：
  - DecompositionFingerprint 加 barcode 字段
  - gauge_equivalence_report 用 bottleneck 距离

**第二层（验证后决定）: T8**
- 背驰 = Wasserstein-1 递减
- 在真实行情数据上验证后决定是否纳入

**第三层（搁置）: T6**
- Leray 条件——过早，等工程需求驱动

### 亏格在分层中的位置

> "亏格不会被条形码替代——它是条形码的结算。
> 条形码属于过程态——它描述'各中枢还活着吗'的全时动态。
> 亏格属于结算态——它把动态凝结为一个整数。
> 在实现上：
> 条形码 → 第一层（作为 DecompositionFingerprint 的字段）
> 亏格 → 等条形码验证后，作为条形码的结算步骤"

---

## 你的任务

1. **逐条评审**编排者的5个洞察——哪些数学上正确，哪些需要修正？
2. **评审分层决断**——Layer 1/2/3 的划分是否合理？有没有遗漏？
3. **"条形码是亏格的精细化"**——这个说法数学上精确吗？给出严格的范畴论或代数拓扑表述。
4. **T8 的可验证性**——在实际数据上验证需要什么前提？可能的failure modes是什么？
5. **δ_k = gauge choice 的后果**——"长寿命 = gauge invariant"在什么条件下严格成立？给出精确陈述。
6. **你的方案中 T6（H¹ 阻碍条件版）被搁置——你同意吗？**如果不同意，给出理由。

**不允许"精神一致"等模糊表述。如果某处你同意，说明为什么同意（数学依据）。
如果你不同意，给出反例或修正。**
"""

print("Calling Codex 5.3 High: consensus round 1 (reviewing editor's decision)...")
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
