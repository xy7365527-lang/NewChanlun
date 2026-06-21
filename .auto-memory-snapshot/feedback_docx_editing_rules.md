---
name: DOCX editing atomic rules
description: One fix per task, pre-built python scripts, PowerShell execution (not REPL)
type: feedback
---

三条强制规则：①每个task只做一个fix；②提前写好python脚本模板；③直接执行脚本，不用REPL。

**Why:** 七个结构性修复打包成一个Wave 1 task导致卡循环。
**How to apply:** docx编辑任务拆分为单一修复粒度。
