# 收据 workers/fix-0072/REPORT.md

- 原件：`workers/fix-0072/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`18d8b30698bc078308b3b8c50fc22fce9f3afe9c94f8a1c90f622406e3ee9b9a`
- 字节数：8888；行数：85

## 关键行（行号: 原文）

```
1: # #1488 fix-0072 报告：候选 publication 降级判据改为类型化失败记录（按独审 0070 FAIL）
3: - 工位：budget-handoff-fix-v2-checkout，分支 `codex/1488-budget-full-chain-fix-v2-20261018`；起点 HEAD `5aff2d1b41`。
7: ## 1. 结论
11: | S86（testonly native，帽 268,435,456） | **86/86 Committed**；`compare_s86 --mode cross-binary` 对 r8a：**VERDICT PASS**，INVARIANT 全部 EQUAL（16 张表、公开根、可达闭包 344419/344419、输入 sha），final_snapshot_root 相同 |
12: | MIN12（同上） | **52/52 Committed**；compare_arms 与 nat2、ord2、**main1（main v3 臂 oracle）** 三组都是 **VERDICT PASS**（回执、digest、counts、structure_cut、rule_revision、rows_sha256 逐代相等） |
15: | 全锁 | **92/92 ok** |
22: ## 2. 设计（判据不读错误文字）
32: **判据**（`audit_owner.rs::publication_degrade_verdict`，testonly 门内）：
47: ## 3. 锁与负控
57: | `publication_degrade_verdict_only_for_pure_budget_shortfall`（audit_owner，重写） | **判据真值表**：7 种包装 × 单层/双层 × Budget/Allocation 都降级；闭包 Ok 的纯资源失败降级；F1、F1 变体、R2、unscoped Ledger + 过桥、unscoped Ledger + 闭包 Ok、作用域 Ledger、无资源失败都拒绝；嵌套 capture 合并；capture 外不记录 |
60: 负控（悬空提交 `refs/f72-red/*`，每个变体都跑上面 7 条，结果见 `red.out`、`red-f.out`；HEAD 7/7 绿）：
73: ## 4. 目标运行细节
75: - 二进制：testonly `3ee05a2713`，sha256 `4d141d8fc4c9621efd2591c04d6b5a9de497fb1dfa6f6aa090f1d5bf41049882`，存放在 `~/nc-remote/1488-accept/f72/bin/`。
77: - S86 的 CONDITIONAL 差异（s_content_nodes 全集 560037/784774、binary_sha256、逐代 stored_nodes）属于 cross-binary 的固有差异，与 fix-0069 F2-1 的归因相同；INVARIANT 全部相等。
78: - MIN12 对 v2 oracle 的 `oracle_counts_equal: 0` 属于 0065 §7 已经归因的 v2/v3 固有差异；逐位口径以 main1 臂为准，结果 PASS。
81: ## 5. 已知局限
```
