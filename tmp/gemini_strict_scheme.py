#!/usr/bin/env python3
"""Gemini 3.1 Pro: design the strictest possible topologization scheme."""

import os

from google import genai

client = genai.Client(api_key=os.environ["GOOGLE_API_KEY"])

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

给出**精确的数学定义**，不是方向评估：
1. 滤子的精确构造：对价格函数 f: {0,1,...,N} → ℝ，定义 sublevel set filtration X_t = f^{-1}(-∞, t]
2. 持续同调模的精确定义：H_0 的条形码如何与缠论中枢对应？给出严格的命题和证明草案
3. 中枢的持续同调刻画：命题——"价格函数 f 的 sublevel set H_0 条形码中，寿命 > τ 的条带与缠论中枢的对应关系"
4. 不变量提取：从条形码中提取哪些数值作为不变量？稳定性定理如何保证鲁棒性？

### Part 2: 层理论方案（具体数学定义）

1. 底空间的精确定义：笔 CW 复形 X 上的 Grothendieck 拓扑
2. 层的精确构造：价格层 F: Open(X)^op → Vect，每个开集 U 映射到什么？限制映射是什么？
3. 层上同调的计算：H^0(X, F) 和 H^1(X, F) 的精确含义
4. 级别一致性定理：命题——"H^1(X, F) ≠ 0 当且仅当存在级别冲突"的严格表述

### Part 3: 离散 Morse 方案（具体数学定义）

1. 离散 Morse 函数的精确定义：在笔 CW 复形上
2. 持久配对与缠论分型的关系
3. 简化算法：如何通过持久性阈值自动筛选"结构性分型"vs"噪声分型"
4. 与缠论笔模式的关系：命题——"持久性 > δ 的临界点集合与缠论分型集合的交集/差集"

### Part 4: 统一框架

1. 三个方案如何统一？是否存在一个包含 PH + Sheaf + Morse 的范畴论框架？
2. 不变量的层次结构：哪些是强不变量（在所有模式下保持）、哪些是弱不变量
3. 计算架构：一个统一的 Python 模块设计（类名、方法名、输入输出类型）

### Part 5: 严格的定理陈述

给出至少 5 个严格的数学命题（Proposition/Theorem），每个附带：
- 精确陈述
- 证明草案或证明策略
- 计算复杂度
- 在缠论语境中的解释

**要求**：
- 每个定义必须是集合论/范畴论层面可操作的
- 每个命题必须可证或可证伪
- 不允许"类比"、"精神一致"等模糊表述
- 如果某处无法严格化，明确说"此处无法严格化，原因是..."
"""

print("Calling Gemini 3.1 Pro: strictest topologization scheme design...")
response = client.models.generate_content(
    model="gemini-3.1-pro-preview",
    contents=PROMPT,
)
print(response.text)
