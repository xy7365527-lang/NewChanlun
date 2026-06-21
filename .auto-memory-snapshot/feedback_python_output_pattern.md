---
name: Python output capture pattern
description: Never use stdout; write results to file; Desktop Commander can't capture Unicode output
type: feedback
---

强制模式：写结果到文件（RESULT_FILE），执行后读取文件内容。不用stdout。

**Why:** Desktop Commander无法可靠捕获python stdout（尤其含Unicode如Ørsted等）。不知道这个模式的task每次都浪费5-10轮诊断。
**How to apply:** 所有python脚本输出写文件，执行后read_file读取。
