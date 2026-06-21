---
name: File write verification
description: Always verify file writes with get_file_info; VM paths don't reach host filesystem
type: feedback
---

task session写文件后必须立即用get_file_info验证非零大小。

**Why:** 9个WF1 task + WF2-7全部声称写入了文件，但目标路径上一个文件都不存在（写到了VM本地路径）。
**How to apply:** 每次Write后立即验证。沙盒路径 ≠ 用户本机路径。
