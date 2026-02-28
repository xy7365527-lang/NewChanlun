#!/usr/bin/env python3
"""Codex/GPT segment diagnosis: 71 strokes -> 1 segment compression analysis.

Uses OpenAI o3 (fallback gpt-4o) to independently diagnose whether
the segment engine's behavior on SPY 60min data (874 bars -> 71 strokes -> 1 segment)
is correct or a bug.

Output: tmp/codex-segment-diagnosis.md
"""

import json
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def read_file(rel_path: str) -> str:
    """Read a file relative to repo root."""
    p = REPO_ROOT / rel_path
    if not p.exists():
        return f"[FILE NOT FOUND: {rel_path}]"
    return p.read_text(encoding="utf-8")


def build_prompt() -> str:
    """Build the full diagnosis prompt with all context."""

    segment_engine_v1 = read_file("src/newchan/a_segment_v1.py")
    segment_v0_types = read_file("src/newchan/a_segment_v0.py")
    validation_json = read_file("tmp/real-data-multi-tf-validation.json")

    # Extract key lesson excerpts (shortened for token efficiency)
    lesson_62_excerpt = """缠论第62课核心定义：
- 线段至少由三笔组成
- 顶分型：第二K线高点是相邻三K线高点中最高的，低点也是最高的
- 底分型：第二K线低点是相邻三K线低点中最低的，高点也是最低的
- 两个相邻的顶和底之间构成一笔"""

    lesson_67_excerpt = """缠论第67课：线段划分标准
- 以向上笔开始的线段，特征序列 = 向下笔的序列 X1X2...Xn
- 以向下笔开始的线段，特征序列 = 向上笔的序列 S1S2...Sn
- 特征序列两相邻元素间没有重合区间 = 该序列的一个缺口
- 把特征序列每个元素看成K线，做包含处理 → 标准特征序列
- 向上笔开始的线段只考察顶分型；向下笔开始的线段只考察底分型

第一种情况：
特征序列的分型中，第一和第二元素间不存在缺口 → 线段在该分型高/低点处结束

第二种情况：
特征序列的分型中，第一和第二元素间存在缺口 → 需要从该分型最高/低点开始构建第二特征序列（同向笔），第二特征序列出现分型则线段结束"""

    lesson_71_excerpt = """缠论第71课：线段划分标准的再分辨
- 特征序列的元素要讨论包含关系，必须是同一特征序列的元素
- 从转折点开始，如果第一笔就破坏了前线段，进而该笔延伸出三笔来，
  其中第三笔破了第一笔的结束位置，那么新的线段一定形成
- 第三笔完全在第一笔的范围内时，分不出方向，等待最终突破方向
- 假设转折点前后两元素不存在包含关系（不同性质）
- 假设转折点后的分型元素可以应用包含关系"""

    lesson_78_excerpt = """缠论第78课：继续说线段的划分
- 同一线段中，两端的一顶一底，顶肯定要高于底（硬约束）
- 所有古怪线段都是因为第一种情况的笔破坏后最终没有形成线段破坏
- 线段出现笔破坏后，如果后续向下线段没破该向上笔的底，则线段B被破坏
- 对第二种情况的第二特征序列分型判断，必须严格按包含关系处理
  （第一种情况中假设分界点两边不能包含处理，但第二种情况可以）"""

    prompt = f"""你是一位精通缠论的量化分析专家。请对以下段引擎（线段引擎）的行为进行独立诊断。

## 问题描述

SPY 60分钟数据处理结果：
- 874 bars → 71 strokes（笔） → 1 segment（线段）
- 唯一的线段：s0=0, s1=70, direction="down", confirmed=false, high=697.84, low=634.92, stroke_span=70

71笔只产出1条线段，这是否是bug？

## 验证数据

```json
{validation_json}
```

注意对比：
- daily 数据：502 bars → 36 strokes → 3 segments（合理）
- weekly 数据：105 bars → 6 strokes → 1 segment（可接受）
- 60min 数据：874 bars → 71 strokes → 1 segment（待诊断）
- daily(126 bars)：9 strokes → 1 segment

## 段引擎源码（v1 特征序列法）

### a_segment_v1.py（主算法）
```python
{segment_engine_v1}
```

### a_segment_v0.py（数据类型定义）
```python
{segment_v0_types}
```

## 缠论原文定义（关键课程摘要）

{lesson_62_excerpt}

{lesson_67_excerpt}

{lesson_71_excerpt}

{lesson_78_excerpt}

## 诊断要求

请逐步分析以下问题：

1. **特征序列构建逻辑审查**：
   - `_FeatureSeqState.append()` 的包含处理是否正确？
   - `dir_state` 的初始化和更新是否符合缠论原文？
   - 向下段的特征序列应该是向上笔——代码是否正确选取了反向笔？

2. **分型检测逻辑审查**：
   - `scan_trigger()` 中分型检测的条件是否正确？
   - 向下段应该找底分型——代码是否只找了目标分型？
   - 缺口判断和第二特征序列的处理是否正确？

3. **断段条件审查**：
   - `_try_trigger_segment()` 中的验证条件是否过于严格？
   - L78硬约束（顶高于底）的前置验证是否可能导致合法断段被跳过？
   - 结算锚验证（新段前三笔必须有重叠）是否可能阻止合法断段？
   - `min_seg_strokes` 约束是否合理？

4. **71笔→1段的可能原因**：
   - 是否存在特征序列包含处理过度合并，导致分型无法形成？
   - 是否存在分型检测到但被后续条件拒绝的情况？
   - TAIL_WINDOW = 7 是否可能导致有效分型被忽略？
   - `_find_overlap_start` 从哪里开始？如果起始位置不是stroke 0会怎样？

5. **结论判定**：
   - 71→1 是 bug 还是正确行为？
   - 如果是 bug，指出具体哪个条件判断有问题
   - 如果正确，解释为什么 71 笔在缠论定义下只能产出 1 段

请给出完整的推理链。"""

    return prompt


def call_openai(prompt: str) -> str:
    """Call OpenAI API with the diagnosis prompt."""
    try:
        from openai import OpenAI
    except ImportError:
        print("ERROR: openai package not installed", file=sys.stderr)
        sys.exit(1)

    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        print("ERROR: OPENAI_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    client = OpenAI(api_key=api_key)

    # Try o3 first, fallback to gpt-4o
    models = ["o3", "gpt-4o"]
    last_error = None

    for model in models:
        try:
            print(f"Calling OpenAI {model}...", file=sys.stderr)
            if model == "o3":
                response = client.chat.completions.create(
                    model=model,
                    messages=[
                        {"role": "user", "content": prompt},
                    ],
                    max_completion_tokens=16000,
                )
            else:
                response = client.chat.completions.create(
                    model=model,
                    messages=[
                        {
                            "role": "system",
                            "content": "你是一位精通缠论（缠中说禅的技术分析理论）和量化编程的专家。请用中文回答。",
                        },
                        {"role": "user", "content": prompt},
                    ],
                    max_tokens=8000,
                    temperature=0.2,
                )

            content = response.choices[0].message.content
            model_used = model
            usage = response.usage
            print(
                f"Success with {model}. Tokens: prompt={usage.prompt_tokens}, "
                f"completion={usage.completion_tokens}",
                file=sys.stderr,
            )
            return f"**模型**: {model_used}\n\n{content}"

        except Exception as e:
            last_error = e
            print(f"Model {model} failed: {e}", file=sys.stderr)
            continue

    return f"ERROR: All models failed. Last error: {last_error}"


def main() -> None:
    print("Building diagnosis prompt...", file=sys.stderr)
    prompt = build_prompt()

    print(f"Prompt length: {len(prompt)} chars", file=sys.stderr)

    result = call_openai(prompt)

    # Write output
    output_path = REPO_ROOT / "tmp" / "codex-segment-diagnosis.md"
    header = """# Codex/GPT 段引擎诊断报告

> 异质诊断：OpenAI o3/gpt-4o 对段引擎 71 strokes → 1 segment 压缩的独立分析
> 生成方式：`scripts/codex_segment_diagnosis.py`

---

"""
    output_path.write_text(header + result, encoding="utf-8")
    print(f"Output written to: {output_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
