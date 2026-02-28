#!/usr/bin/env python3
"""Gemini 异质诊断：段引擎 71 strokes -> 1 segment 压缩问题。

247号验证发现 SPY 60min 数据 874 bars -> 71 strokes -> 1 segment。
本脚本用 Gemini 2.5 Pro 对段引擎源码进行独立诊断。

用法:
    python scripts/gemini_segment_diagnosis.py

输出:
    tmp/gemini-segment-diagnosis.md
"""

import os
import sys
from pathlib import Path

# --- 配置 ---
API_KEY = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY", "")
if not API_KEY:
    print("ERROR: 请设置 GEMINI_API_KEY 或 GOOGLE_API_KEY 环境变量")
    sys.exit(1)

import google.generativeai as genai

genai.configure(api_key=API_KEY)
model = genai.GenerativeModel("gemini-2.5-pro")

# --- 读取源码和参考材料 ---
ROOT = Path(__file__).resolve().parent.parent


def read_file(rel_path: str) -> str:
    p = ROOT / rel_path
    if p.exists():
        return p.read_text(encoding="utf-8")
    return f"[文件不存在: {rel_path}]"


segment_v1_src = read_file("src/newchan/a_segment_v1.py")
segment_v0_src = read_file("src/newchan/a_segment_v0.py")
segment_engine_src = read_file("src/newchan/core/recursion/segment_engine.py")
validation_json = read_file("tmp/real-data-multi-tf-validation.json")
genealogy_247 = read_file(".chanlun/genealogy/settled/247-real-data-multi-tf-validation.md")

# 缠论原文 -- 线段相关课程
lesson_62 = read_file("docs/chanlun/text/blog/062-第62课.md")
lesson_65 = read_file("docs/chanlun/text/blog/065-第65课.md")
lesson_67 = read_file("docs/chanlun/text/blog/067-第67课.md")
lesson_71 = read_file("docs/chanlun/text/blog/071-第71课.md")
lesson_77 = read_file("docs/chanlun/text/blog/077-第77课.md")
lesson_78 = read_file("docs/chanlun/text/blog/078-第78课.md")

# 知识库中线段章节
knowledge_base = read_file("缠论知识库.md")

# --- 构造 Prompt ---
PROMPT = f"""你是一个精通缠论（缠中说禅的技术分析理论）的资深工程师和理论审查员。
你的任务是对一个段引擎（线段划分算法）进行独立诊断。

## 背景

247号谱系验证发现，SPY 60min 数据（874 bars）经过笔引擎处理后产生 71 根笔（strokes），
但段引擎（segment engine）只输出了 1 个线段（segment）。

这个线段的属性：
- s0=0, s1=70（覆盖所有71根笔）
- direction=down
- confirmed=false
- high=697.84, low=634.92
- stroke_span=70

问题：71根笔只产出1个线段，这是正确行为还是 bug？

## 缠论原文定义（线段相关）

### 第62课：分型、笔与线段
{lesson_62[:3000]}

### 第65课：再说说分型、笔、线段
{lesson_65[:3000]}

### 第67课：线段的划分标准（核心——特征序列法）
{lesson_67[:5000]}

### 第71课：线段划分标准的再分辨
{lesson_71[:4000]}

### 第77课：一些概念的再分辨
{lesson_77[:3000]}

### 第78课：继续说线段的划分（古怪线段 + 硬约束）
{lesson_78[:4000]}

## 缠论知识库摘要（线段章节）

{knowledge_base[knowledge_base.find("## 5. 线段"):knowledge_base.find("## 6. 走势中枢")]}

## 段引擎源码

### a_segment_v1.py（核心算法——特征序列法实现）
```python
{segment_v1_src}
```

### a_segment_v0.py（数据类型定义 + v0 参考实现）
```python
{segment_v0_src}
```

### segment_engine.py（事件驱动包装层）
```python
{segment_engine_src}
```

## 247号验证结果
```json
{validation_json}
```

## 247号谱系记录
{genealogy_247}

## 诊断要求

请按以下结构输出诊断结论（中文）：

### 1. 算法走查

对照缠论原文第67课的特征序列法定义，逐步走查 `segments_from_strokes_v1` 的执行路径：

1. 初始状态：seg_start, seg_dir 如何确定？
2. 特征序列构建：反向笔如何加入特征序列？包含处理的方向判断是否正确？
3. 分型检测：`_is_fractal_and_gap` 的判断条件是否与原文一致？
4. 断段触发：`_try_trigger_segment` 的三个验证条件（min_seg_strokes, L78硬约束, 结算锚）是否过于严格？
5. 特别关注：
   - 包含处理的方向判断（`dir_state`）是否正确？初始值的设置逻辑？
   - 第二种情况（有缺口时）的处理是否正确？
   - `TAIL_WINDOW = 7` 是否导致分型被遗漏？
   - 结算锚验证（`k + 2 >= n` 检查）是否过度阻止了合法的断段？

### 2. 71笔→1段的具体原因

分析71根笔的情况下，为什么特征序列始终无法产生有效的分型触发：
- 是特征序列的包含处理把大部分元素合并了？
- 还是分型检测条件过严？
- 还是断段验证条件（L78硬约束 / 结算锚）阻止了合法触发？
- 还是71根笔确实构成一个合法的巨大线段（理论上可能吗？）

### 3. Bug 还是正确行为？

给出明确判断：
- 如果是 bug，指出具体哪个条件判断有问题，以及修复建议
- 如果是正确行为，解释为什么71根笔只能产出1个线段

### 4. 与其他时间框架的对比

验证结果中：
- daily: 36 strokes -> 3 segments
- weekly: 6 strokes -> 1 segment
- daily(短): 9 strokes -> 1 segment

这些比例是否合理？是否存在系统性问题？

### 5. 修复建议（如果是 bug）

如果诊断为 bug，给出具体的修复建议：
- 哪些条件需要放松？
- 哪些逻辑需要修改？
- 修复后的预期行为是什么？

请务必基于缠论原文定义和源码进行严格推理，不要猜测。
"""

# --- 调用 Gemini ---
print("正在调用 Gemini 2.5 Pro 进行段引擎诊断...")
print(f"Prompt 长度: {len(PROMPT)} 字符")

response = model.generate_content(
    PROMPT,
    generation_config=genai.GenerationConfig(
        temperature=0.2,
        max_output_tokens=16384,
    ),
)

result_text = response.text

# --- 保存结果 ---
output_path = ROOT / "tmp" / "gemini-segment-diagnosis.md"
output_path.write_text(
    f"# Gemini 段引擎诊断报告\n\n"
    f"> 模型: gemini-2.5-pro\n"
    f"> 日期: 2026-02-28\n"
    f"> 背景: 247号验证 SPY 60min 71 strokes -> 1 segment\n\n"
    f"---\n\n"
    f"{result_text}\n",
    encoding="utf-8",
)

print(f"\n诊断结果已保存到: {output_path}")
print(f"输出长度: {len(result_text)} 字符")
print("\n--- 诊断结论摘要 ---")
# 输出前2000字符作为摘要
print(result_text[:2000])
