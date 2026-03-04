#!/usr/bin/env python3
"""Codex: Phase 3 穿越基础设施 Round 2 讨论（编排者裁定后）。"""

import os
from pathlib import Path
import openai

client = openai.OpenAI(
    api_key=os.environ.get("OPENAI_API_KEY"),
)

ctx = Path("tmp/phase3-discuss-r2-ctx.md").read_text(encoding="utf-8")

PROMPT = f"""你是 Codex（OpenAI 高级推理模型）。你在参与一个谱系知识仓库的穿越基础设施方案讨论。

Round 1 讨论已完成并达成 5 项共识，编排者已对 3 项分歧做出裁定。现在基于确定参数，细化 5 个组件的精确实现规格。

## 讨论上下文

{ctx}

## 你的任务

逐焦点讨论（共 5 个焦点），给出具体的实现建议。你的视角侧重：
1. **工程严谨性**：函数签名、数据结构、错误处理、边界情况
2. **增量 vs 全量的权衡**：在 1024 区块 / 5197 关系的规模下，哪些优化是必要的，哪些是过度工程
3. **接口一致性**：5 个组件之间的数据流是否顺畅，类型是否对齐
4. **可测试性**：每个组件如何编写单元测试

对每个焦点：
- 给出你推荐的方案（含函数签名和数据结构）
- 列出你不确定的假设
- 标注与 Gemini 可能产生分歧的点

产出控制在 8KB 以内。用中文回复，代码示例用 Python。
"""

print("Calling Codex for Phase 3 Round 2 discuss...")
response = client.responses.create(
    model="o3",
    input=PROMPT,
    max_output_tokens=16384,
)

result = response.output_text
print(f"Got {len(result)} chars")

Path("tmp/codex-phase3-discuss-r2.md").write_text(
    f"# Codex: Phase 3 Round 2 实现规格讨论\n\n{result}",
    encoding="utf-8",
)
print("Written to tmp/codex-phase3-discuss-r2.md")
