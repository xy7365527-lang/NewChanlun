# 收据 workers/baseline-0080/REPORT.md

- 原件：`workers/baseline-0080/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`28cfbebc84bd7e7a20e94a7d716c0611b7af211b2f49b83f8b3d65d1b4f8bcbe`
- 字节数：2763；行数：39

## 关键行（行号: 原文）

```
1: # baseline-0080 REPORT：#1488 完整路径冻结基线
7: ## 结论
9: | 基线 | 双跑 | 冻结值（末代） | checkpoint 后库 sha256 |
12: | S86，终面 3ee05a2713 d2f9 | p1≡p2，same-binary 逐字段全等 | 与 main 公开面全等（cross-binary PASS） | `ddb1d5b0…c1c9` |
13: | MIN12 v3，main c944 | main1≡main4，compare_arms PASS，两清单逐字段全等 | qcut3 `93911ed1…4f32`，cut-52 | `5c893f3f…c628` |
17: 节点 784,774→560,069。与 S5 receipts（INVARIANT 31/31，CONDITIONAL 2/5）一致。
19: ## 过程中的发现
32: ## G_L 与双跑方式
36: ## 未做 / 限制
```
