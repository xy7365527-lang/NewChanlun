# 收据 workers/runner-0046/REPORT.md

- 原件：`workers/runner-0046/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`94df5824ac4f898eb392db885da0bfc08b0ff784d0b4c40a92f72b01674bd243`
- 字节数：3769；行数：42

## 关键行（行号: 原文）

```
1: # runner-0046 报告：六 driver runner 适配 v4 冻结源
8: ## 起点核对
9: - 两份 CI 文件在 HEAD 与 `5cb7253d46` 上 blob 相同（runner `62c8183eab`，workflow `0cccbd752d`），即确为先前独审、跑在 5cb7253d 上的版本。
12: ## 差异逐项
25: 其余未动：`CI_FILES`、`LEGACY_RUNNER`、`LEGACY_SHA256`、`FEATURES`、`DISK_MIN_BYTES`、TESTS 其余五条及顺序、监督 / reap / 超时 / 封存 / 发布逻辑、workflow 的超时、artifact 等部分。
27: ## SOURCE_FILES 计算方法
28: 范围沿用原脚本口径的三个文件（键集合不变）。分两路计算，结果一致：
32: ## legacy runner
33: `.github/scripts/run-1488-delta-ci.py` 仍在；`shasum -a 256` = `2c94e9e8354bd3fe369067210054db903df019cf9ce8c1345a6ec6d107b297dc`，与 `LEGACY_SHA256` 一致；`git diff 9160fc9d fff166540e` 里没有它，所以常量保持原值。
35: ## 自检结果
```
