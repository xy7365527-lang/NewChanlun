#!/usr/bin/env python3
"""Call OpenAI gpt-5.3-codex to review global_capital_flow_v3."""

import os
from pathlib import Path
import openai

client = openai.OpenAI(
    api_key=os.environ.get("OPENAI_API_KEY", "REDACTED")
)

# Read inputs
doc = Path("tmp/global_capital_flow_v3.txt").read_text(encoding="utf-8")
briefing = Path("C:/Users/hanju/Downloads/briefing_for_agents.md").read_text(encoding="utf-8")
consensus = Path(".chanlun/sessions/fragments/global-capital-flow-bilateral.md").read_text(encoding="utf-8")
knowledge = Path("缠论知识库.md").read_text(encoding="utf-8")

PROMPT = f"""你是一个严格的数学审查员和代码架构师，同时对缠论（缠中说禅的技术分析理论）有深入理解。

## 缠论知识库（供你参考缠论的严格定义）

{knowledge}

## 质询要求

{briefing}

## 前三轮质询共识（供参考，不要被它约束）

{consensus}

## 被质询文档（全文）

{doc}

## 输出要求

请用中文输出，包含：
1. **逐章审查意见**：对每一章的每一个命题判断严格性
2. **推演A**：循环约束对方向的数学分析（rAC = rAB + rBC，瞬时→走势方向传递条件）
3. **推演B**：比价MACD的精确数学表达式，和A/B各自MACD的关系
4. **推演C**：27种配置完备性验证，派生边允许方向推导
5. **代码可行性评估**：基于缠论形式化代码库（包含关系处理→分型→笔→线段→中枢→走势类型→背驰→买卖点的完整管线），评估文档架构的实现路径
6. **修改建议**：具体改什么、改成什么

数学推导要严格。不要客气。文档的目的是被攻击到无法再被攻击为止。
"""

print("Calling gpt-5.3-codex via responses API...")
response = client.responses.create(
    model="gpt-5.3-codex",
    input=PROMPT,
    max_output_tokens=16384,
)

result = response.output_text
print(f"Got {len(result)} chars")

Path("tmp/codex-inquiry-v3-r2.md").write_text(
    f"# Codex 5.3 质询报告：全球资本流转 v3\n\n{result}",
    encoding="utf-8",
)
print("Written to tmp/codex-inquiry-v3-r2.md")
