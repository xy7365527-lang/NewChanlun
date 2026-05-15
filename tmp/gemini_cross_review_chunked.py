#!/usr/bin/env python3
"""
分段调用 Gemini API 完成交叉审查。
优先 gemini-3.1-pro-preview，503 时回退 gemini-2.5-pro。
"""

import sys
import time
import pathlib
import os

from google import genai

sys.stdout.reconfigure(line_buffering=True)

from google.genai import types

client = genai.Client(api_key=os.environ["GOOGLE_API_KEY"])

MODELS = ["gemini-3.1-pro-preview", "gemini-2.5-pro"]
GEN_CONFIG = types.GenerateContentConfig(
    max_output_tokens=8192,
    temperature=0.2,
    thinking_config=types.ThinkingConfig(thinking_budget=8192),
)

BASE = pathlib.Path(r"C:\Users\hanju\NewChanlun\tmp")
gemini_report = (BASE / "gemini-inquiry-v3.md").read_text(encoding="utf-8")
codex_report = (BASE / "codex-inquiry-v3-r2.md").read_text(encoding="utf-8")


def call_gemini(prompt: str, label: str) -> str:
    """尝试每个模型，每个模型重试3次，退避30/60/90s。"""
    for model_name in MODELS:
        print(f"[{label}] 尝试模型: {model_name}")
        for attempt in range(3):
            try:
                resp = client.models.generate_content(
                    model=model_name,
                    contents=prompt,
                    config=GEN_CONFIG,
                )
                text = resp.text
                print(f"[{label}] 成功 (model={model_name})，长度={len(text)} 字符")
                return text
            except Exception as e:
                wait = 30 * (attempt + 1)
                print(f"[{label}] 第{attempt+1}次失败 ({model_name}): {e}")
                if attempt < 2:
                    print(f"  等待 {wait}s 后重试...")
                    time.sleep(wait)
        print(f"[{label}] {model_name} 3次均失败，尝试下一个模型...")
    raise RuntimeError(f"[{label}] 所有模型均失败")


# ── Step 1 ──
prompt1 = f"""你是 Gemini，以下是你之前对《全球资本流转分析 v3》的独立质询报告。

请提取你的核心观点，输出 3-5 个要点，每个要点包含：
- 观点标题
- 核心论据（1-2句）
- 严格性判定（成立/不成立/条件成立）

只输出要点，不要重复原文。用中文回答。

---
{gemini_report}
"""

print("=" * 60)
print("Step 1: 总结 Gemini 核心观点")
print("=" * 60)
summary = call_gemini(prompt1, "Step1")
print()

# ── Step 2 ──
prompt2 = f"""你是 Gemini。以下是你对《全球资本流转分析 v3》的核心观点总结，以及另一位审查员（Codex 5.3）的独立质询报告。

请逐项审查 Codex 的意见：
1. 对每个 Codex 观点，判断：同意/不同意/部分同意
2. 如果不同意，给出你的反驳理由
3. 如果 Codex 发现了你遗漏的问题，明确承认
4. 如果你发现了 Codex 遗漏的问题，指出来

用中文回答。结构化输出。

## 你的核心观点总结
{summary}

## Codex 5.3 的质询报告
{codex_report}
"""

print("=" * 60)
print("Step 2: 逐项审查 Codex 报告")
print("=" * 60)
review = call_gemini(prompt2, "Step2")
print()

# ── Step 3 ──
prompt3 = f"""你是 Gemini。以下是你的核心观点总结和你对 Codex 5.3 报告的逐项审查结果。

请输出最终的交叉审查报告，格式如下：

# Gemini 3.1 Pro 交叉审查报告：全球资本流转 v3

## 一、共识区（双方一致认为的问题）
（列出双方都指出的关键问题）

## 二、分歧区（双方意见不同的地方）
（列出分歧点，给出你的最终判断和理由）

## 三、Gemini 独有发现（Codex 未覆盖）
（你发现但 Codex 遗漏的问题）

## 四、Codex 独有发现（Gemini 未覆盖）
（Codex 发现但你遗漏的问题，诚实承认）

## 五、综合严格性评级
（对文档整体给出评级和一句话总结）

## 六、优先修改建议（按紧迫度排序）
（合并双方建议，去重后按优先级排列）

用中文回答。保持严格、直接、不客气。

## 你的核心观点总结
{summary}

## 你对 Codex 报告的逐项审查
{review}
"""

print("=" * 60)
print("Step 3: 生成最终交叉审查报告")
print("=" * 60)
final_report = call_gemini(prompt3, "Step3")
print()

# ── 写入 ──
output_path = BASE / "gemini-cross-review-v3.md"
output_path.write_text(final_report, encoding="utf-8")
print(f"最终报告已写入: {output_path}")
print(f"报告长度: {len(final_report)} 字符")
