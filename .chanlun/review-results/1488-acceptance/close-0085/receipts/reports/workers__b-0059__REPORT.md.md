# 收据 workers/b-0059/REPORT.md

- 原件：`workers/b-0059/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`2f68876a188b39e5f9e812a3d70bfa41d7b799bef0d60372c487036334b91d37`
- 字节数：12258；行数：162

## 关键行（行号: 原文）

```
1: # #1488 0059 REPORT：B1–B10 落地 + B11 结论 + 冷根量化与方案
3: - 工位：budget-handoff-fix-v2-checkout，分支 codex/1488-budget-full-chain-fix-v2-20261018。起点 HEAD df0b98f0f7。
8: - 裁定：https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6287615661
10: ## 1. B1–B10 落地表（提交 460f7a6cfd）
12: 每处测试注释都写了四项：“第 N 处裁改断言”、裁定链接、原断言原文、v4 条款。改写内容取自 0058 b-proposal.patch，去掉了 B11 的四个 hunk。
31: - HEAD 跑 b10 过滤：10/10 绿（172054）。
37: - HEAD 全锁：84 passed（173019）。
39: ## 2. B11 结论：生产帽下不可达，已按“不可达分支”改写（提交 998ce0218b）
41: ### 2.1 不可达证明（逐条引代码）
47: 5. 结论：**在生产构建里，“同父完整审计峰约 149MiB”这条路径根本不会被编译进去，因此不可达。**
50: ### 2.2 改写（第十六处裁改断言，`cross_request_source_cache.rs` 中的 `r14_source_pool_normal_growth_and_sparse_cap_match_none`）
52: - 注释写明：裁定链接、0059 第 2 项的分支选择、原断言（f77fbfd6d9）原文，以及 v4 L9、L21、L42。
79: ### 2.3 回答主控：该规模下是 fail-closed 还是降级？
89: ## 3. 冷根风险量化（生产默认帽 256MiB，S86 / MIN12）
91: ### 3.1 结果
120: ### 3.2 峰值来源分解
134: ### 3.3 不改资源帽的修复方案（只写方案，不实施，由 R2 切片统一排）
136: - **P1（对齐 v4 L13，优先）**：publication 中可选候选的 journal/索引预算不足时，放弃候选保留，改走无候选发布并成功回复，而不是 StaleWriter 卡在 Begin。做法有两种：
139: - **P2**：把索引扩容估算改成精确值（hashbrown 实际 bucket 分配），不再用约 4 倍容量的保守估算。这能消除“父账还有余量却误判超帽”。
149: ## 4. 宽对拍与既有失败
157: ### 追记：998c 宽对拍（200824）
```
