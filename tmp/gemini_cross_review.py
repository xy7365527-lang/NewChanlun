#!/usr/bin/env python3
"""Gemini 3.1 Pro: cross-review Codex 5.3's inquiry report (slim prompt)."""

import google.generativeai as genai
import os
from pathlib import Path

API_KEY = os.environ.get("GOOGLE_API_KEY", "AIzaSyD8eB39hxIxqsEk6sIwSpuD4nAc7OgufdQ")
genai.configure(api_key=API_KEY)

model = genai.GenerativeModel("gemini-2.5-pro")

codex_report = Path("tmp/codex-inquiry-v3-r2.md").read_text(encoding="utf-8")
gemini_report = Path("tmp/gemini-inquiry-v3.md").read_text(encoding="utf-8")

PROMPT = f"""你是 Gemini 3.1 Pro。你已经完成了对全球资本流转 v3 文档的独立质询（你的报告在下面）。

现在你需要交叉审查 Codex 5.3 的质询报告。不需要回看原始文档——你已经读过了。

## 你自己的质询报告

{gemini_report}

## Codex 5.3 的质询报告（你要审查的）

{codex_report}

## 任务

1. 逐项审查 Codex 的审查意见：哪些你同意？哪些你不同意？为什么？
2. 审查 Codex 的三个推演（A/B/C）：数学推导是否正确？有没有遗漏或错误？
3. 审查 Codex 的修改建议：哪些合理？哪些过度或不足？
4. 找出你们之间的分歧点，给出你的判断

用中文输出。严格。
"""

print("Calling Gemini 3.1 Pro for cross-review (slim)...")
response = model.generate_content(
    PROMPT,
    generation_config=genai.GenerationConfig(
        max_output_tokens=16384,
        temperature=0.2,
    ),
)

result = response.text
print(f"Got {len(result)} chars")

Path("tmp/gemini-cross-review-v3.md").write_text(
    f"# Gemini 交叉审查 Codex 报告\n\n{result}",
    encoding="utf-8",
)
print("Written to tmp/gemini-cross-review-v3.md")
