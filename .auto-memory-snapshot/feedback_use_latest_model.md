---
name: use-latest-model
description: "CC任务用opus级别模型（opus-4-8优先，不可用时opus-4-7），不用sonnet"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户明确要求开CC任务时用 opus 级别模型+ultracode深度。不要用 sonnet。

优先级：claude-opus-4-8 > claude-opus-4-7 > claude-opus-4-6
fable-5 曾经可用但2026-06-17不可用（start_code_task报错model not found）。

**Why:** 2026-06-17用户纠正："为什么你开任务都是sonnet而不是opus4.8的ultracode？你得开opus啊"。Sonnet推理深度不够，opus才能处理形式化+严格性要求。
**How to apply:** start_code_task 用 model="claude-opus-4-8"；如果超时用 model="claude-opus-4-7"作fallback；绝对不用 sonnet
