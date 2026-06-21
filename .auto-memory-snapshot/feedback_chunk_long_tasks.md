---
name: Chunk long file operations
description: Break long edits into segments, add hardness enforcement to prevent task freezes
type: feedback
---

编辑长文件时必须分段，每次一步，每步后验证。

**Why:** 任务反复因单次处理长文件而冻结/卡住。
**How to apply:** task prompt中明确"ONE change at a time, verify after each write"；python-docx脚本每个section单独写；报告生成每次最多25行。
