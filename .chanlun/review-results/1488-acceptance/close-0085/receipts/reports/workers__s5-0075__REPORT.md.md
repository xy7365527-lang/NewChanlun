# 收据 workers/s5-0075/REPORT.md

- 原件：`workers/s5-0075/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`24184a9a4d8895cf201db06525168542e89ac35e791fab6a93baac7ffcb050a6`
- 字节数：8527；行数：131

## 关键行（行号: 原文）

```
1: # #1488 0075 S5 终面运行报告
6: 输入是 `s86/` 下的冻结 S86 输入，各臂 run-manifest 都复记了 sha256。二进制 sha256 见 `s5/BINARIES.txt`
9: ## 结论一览
11: | 项 | 结论 |
13: | A① cross-binary（D1） | **PASS**：p1 对 r8a、p1 对 r8b、p2 对 r8a，INVARIANT 全等。CONDITIONAL 差异只有下面 3 项，请主控确认 |
14: | A① same-binary | **PASS**：p1 对 p2，INVARIANT 与 CONDITIONAL 全等（节点全集 560,069 = 560,069） |
21: ## 1. A①：CONDITIONAL 差异逐项说明（交主控确认）
28:    - 终面多出的节点数为 0（`receipts/nodeset-p1-vs-r8a.json`）。可达集同为 344,419，公开根与 basis 已在 INVARIANT 中全等。
32: 3. **binary_sha256**：预期不同。DDL 和 meta 环境项全等。
34: **比较器修补（需审阅者确认）。** `s86/compare_s86.py` 的 `diff_list` 原来对 list 元素做 `set()`，CONDITIONAL「逐代 stored」
38: ## 2. R1：不可达来源分解
68: - 读数：560,037 节点，不可达 215,618（38.50%）。n1b 对 r8a cross-binary PASS。
73: - d2 对 n1b 对拍 PASS，节点集相同（`nodeset-d2-vs-n1b.json`）。
78: ## 3. D7：8MiB 余量读数
107: ## 4. 偏差与限制
126: ## 交主控的事项
128: 1. 确认 A① 的 3 项 CONDITIONAL 差异说明。
```
