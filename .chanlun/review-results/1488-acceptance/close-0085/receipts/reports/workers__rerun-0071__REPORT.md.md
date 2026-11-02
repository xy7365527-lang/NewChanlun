# 收据 workers/rerun-0071/REPORT.md

- 原件：`workers/rerun-0071/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`9fde0ee30574cf2a3cc3a13e56386245ac3b7c39a6349bdf9c545e0bda356928`
- 字节数：7546；行数：72

## 关键行（行号: 原文）

```
1: # #1488 0071 并发窗口证据私有 target 复跑报告
3: ## 1. 口径
6: - 做法：清单里的原提交在 NewChanlun 和本工位都还在，所以**直接 checkout 原提交、跑原命令**，没有在当前 HEAD 上重建补丁。远端 ref 为 `refs/uo1488/r71-<项>`，指向原 SHA。
8: - 比对方式：test result 计数；panic 位置；panic 消息与 left/right（PID 除外，逐字比较）；`--nocapture` 时再比 R14 普查行；宽对拍和冲突模块按逐测试 ok/FAILED/ignored 集合 diff。
9: - 不重跑的项（窗口内取得、但已有 GitHub CI 复证）：0047 终版锁 79/80@57f9680c4e（CI 39370285846）；0053 全锁 80/80@c2bdf9b662（CI 39373166910）；S3 锁 84/84@bba547492d（CI 39384343438）。
11: ## 2. 0057 r14 生产变异（直接 checkout 原提交，r14 `--exact --nocapture`）
13: | 项 | SHA | 原结论（并发窗口） | 复跑结论 | 复跑日志 | 一致 |
25: ## 3. S3 负控 N1–N5（原提交，原锁命令）
27: | 项 | SHA | 原结论 | 复跑结论 | 复跑日志 | 一致 |
34: | 正控（补） | bba547492d | CI 39384343438 已复证 | 四锁 4/4 ok（同 target 配对正控） | .chanlun/review-results/1488-acceptance/receipts/20261021-041710-bba547492d.md | ✔ |
38: ## 4. 0053 r14 推导 N1/N2 与 0047 合并后结果
40: | 项 | SHA | 原结论 | 复跑结论 | 复跑日志 | 一致 |
51: ## 5. grep「并发窗口」的其他条目
58: | 项 | SHA | 原结论 | 复跑结论 | 复跑日志 | 一致 |
62: ## 6. 结论
70: ## 证据引用迁移（0077 P30，2026-10-21 追加）
72: 本文原引用的 21 份 `/tmp/nc-remote/<run-id>.log` 已改为仓内收据 `.chanlun/review-results/1488-acceptance/receipts/<run-id>.md`（相对工位 checkout 根；关键行 + 整文件 sha256，原日志名与 run-id 一一对应）。索引见 `.chanlun/review-results/1488-acceptance/receipts/INDEX.md`。
```
