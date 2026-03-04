#!/usr/bin/env python3
"""Codex 5.3 High: design the strictest possible topologization scheme."""

import openai

client = openai.OpenAI(
    api_key="REDACTED"
)

PROMPT = r"""你是一个严格的代数拓扑学家。不要评估方向——**设计一个完整的、最严格的拓扑化方案**。

## 背景

缠论代码已有一维 CW 复形结构：
- 0-cell = 分型（局部极值点），1-cell = 笔（连接相邻分型的有向边）
- 线段 = 1-chain（笔的子链）
- 中枢 = 三段闭区间有限交 [ZD, ZG]
- 级别递归 = 滤子 F_0 ⊂ F_1 ⊂ F_2 ...
- 不同笔模式（wide/strict/new）对同一输入产出不同分解

## 你的任务

设计一个**完整的数学框架**，包含：

### Part 1: 持续同调方案（具体数学定义）

给出**精确的数学定义**：
1. 滤子的精确构造：对价格函数 f: {0,1,...,N} → ℝ，定义 sublevel set filtration
2. 持续同调模的精确定义：H_0 的条形码如何与缠论中枢对应？给出严格命题
3. 中枢的持续同调刻画：命题——长寿命条带与缠论中枢的精确对应
4. 不变量提取：稳定性定理如何保证鲁棒性？

### Part 2: 层理论方案（具体数学定义）

1. 底空间精确定义
2. 层的精确构造：价格层 F，限制映射
3. 层上同调 H^0, H^1 的精确含义
4. 级别一致性定理的严格表述

### Part 3: 离散 Morse 方案（具体数学定义）

1. 离散 Morse 函数的精确定义
2. 持久配对与缠论分型的关系
3. 自动筛选"结构性分型"vs"噪声分型"
4. 与缠论笔模式的关系

### Part 4: 统一框架

1. PH + Sheaf + Morse 如何统一？
2. 不变量层次结构
3. Python 模块设计（类名、方法名、输入输出类型）

### Part 5: 严格的定理陈述

给出至少 5 个严格的数学命题，每个附带：精确陈述、证明草案、计算复杂度、缠论语境解释

**Gemini 3.1 Pro 同时在设计同一方案。我们需要你的独立设计，之后会对比。
如果某处无法严格化，明确说"此处无法严格化，原因是..."
不允许"类比"、"精神一致"等模糊表述。**
"""

print("Calling Codex 5.3 High: strictest topologization scheme design...")
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
