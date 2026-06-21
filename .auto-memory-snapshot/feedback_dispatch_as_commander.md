---
name: Dispatch as commander
description: Decompose before dispatching; Dispatch is the general, subtasks are worker bees; wave-based parallel execution
type: feedback
---

每次dispatch前必须分解任务。四步骤：能否拆分→粒度→权重（轻/重模型）→波次结构（独立任务并行、波间综合）。

**Why:** 用户明确的元编排原则，适用于所有领域。
**How to apply:** 模式"将军与工蜂"——Dispatch分解+综合，subtask执行。不做简单转发。
