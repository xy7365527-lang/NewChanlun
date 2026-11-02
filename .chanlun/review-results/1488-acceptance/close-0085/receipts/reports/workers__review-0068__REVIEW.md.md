# 收据 workers/review-0068/REVIEW.md

- 原件：`workers/review-0068/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`458332a419cbf426ef69a5b2a606cb1c40b8336e01f1c8ff07e631dda3f97625`
- 字节数：10926；行数：131

## 关键行（行号: 原文）

```
1: # #1488 0065 降级修复非作者独审（review-0068）
3: - 评审对象：`727b3bfb45..78523300ad`（17 提交），作者报告 `workers/degrade-0065/REPORT.md`。
8: ## Verdict：PASS（附条件）
10: 六项核查的主体结论都成立：生产零变化、P1 只在预算不足时降级、Ledger 仍 fail-closed、锁和红证真实、S86/MIN12 结果属实。
16: ## 1. 生产不变
22: | 本审重建生产 release sha256 | `01b31fda…` | `4da7731d…` |
32: ## 2. P1 降级判定（含反例搜索）
59: ## 3–4. 锁、负控与红证（本审独立重跑）
61: - 全锁表 `78523300ad`：**87/87 PASS**（runs/20261021-023123-78523300ad.log），含 3 条新锁。
76: ## 5. 目标运行
79: - 本审在 WSL 用原工具独立重跑 `compare_s86.py d65/s86-nat2 s1/r8a --mode cross-binary`：**VERDICT PASS**，INVARIANT 31/31，CONDITIONAL 2/5。末代 snapshot 根 `7508b621…`、basis 根 `sc1-c195df59…` 两侧相同；可达闭包 344,419 / 344,419。
87: - 报告把 `s_content_nodes` 和 `stored_*` 的 CONDITIONAL 差异归因于“native 在放弃候选前的复用减少了中间节点写入”，与数据不符：
90: - 结论：差异主要来自二进制（#1488 分支对 main `6badaa6049`），不是 native 复用。cross-binary 口径允许这项差异，VERDICT 不变，但解释要改。
93: - 本审独立重跑 `compare_arms.py`，nat2↔ord2、nat2↔main1、ord2↔main1 三组都 **PASS**。
94: - 比较字段 digest/counts/structure_cut/rule_revision/rows_sha256 在 52 代上都没有 null，不存在空值相等的空洞；三臂各 52/52 Committed，回执 publication 都非空。
101: - 精确限定：main1 和 nat2 共用同一个 v3 直驱（信封改写为 s-session/3），严格说这项差异是“v3 + 本驱动改写”共有的，独立 v3 oracle 仍待主控裁定（报告 §9.3 已列出）。
103: ## 6. 冷候选保留与命中
116: ## 发现汇总
121: | F2 | 中 | CONFIRMED | S86 `s_content_nodes`/`stored_*` 差异的归因错误：约 97% 在放弃之后；MIN12 同二进制 native/ordinary 只差 144 节点。主因是二进制差异。VERDICT 不变 |
124: | I2 | 信息 | — | MIN12“v3 固有”依赖与 main1 共用的直驱；严格口径要独立 v3 oracle（主控裁定） |
127: ## 留痕
```
