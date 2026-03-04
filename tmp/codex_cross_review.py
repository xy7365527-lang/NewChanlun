#!/usr/bin/env python3
"""Codex 5.3: cross-review Gemini 3.1 Pro's inquiry report."""

import os
from pathlib import Path
import openai

client = openai.OpenAI(
    api_key=os.environ.get("OPENAI_API_KEY", "REDACTED")
)

codex_report = Path("tmp/codex-inquiry-v3-r2.md").read_text(encoding="utf-8")
gemini_report = Path("tmp/gemini-inquiry-v3.md").read_text(encoding="utf-8")
doc = Path("tmp/global_capital_flow_v3.txt").read_text(encoding="utf-8")

PROMPT = f"""你是 Codex 5.3。你已经完成了对全球资本流转 v3 文档的独立质询。

现在你需要交叉审查 Gemini 3.1 Pro 的质询报告。

## 你自己的质询报告（供参考）

{codex_report}

## Gemini 3.1 Pro 的质询报告（你要审查的）

{gemini_report}

## 原始文档（供回查）

{doc}

## 任务

1. 逐项审查 Gemini 的审查意见：哪些你同意？哪些你不同意？为什么？
2. 审查 Gemini 的三个推演（A/B/C）：数学推导是否正确？有没有遗漏或错误？
3. 审查 Gemini 的修改建议：哪些合理？哪些过度或不足？
4. 找出你们之间的分歧点，给出你的判断

用中文输出。严格。
"""

print("Calling Codex 5.3 for cross-review...")
response = client.responses.create(
    model="gpt-5.3-codex",
    input=PROMPT,
    max_output_tokens=16384,
)

result = response.output_text
print(f"Got {len(result)} chars")

Path("tmp/codex-cross-review-v3.md").write_text(
    f"# Codex 交叉审查 Gemini 报告\n\n{result}",
    encoding="utf-8",
)
print("Written to tmp/codex-cross-review-v3.md")
