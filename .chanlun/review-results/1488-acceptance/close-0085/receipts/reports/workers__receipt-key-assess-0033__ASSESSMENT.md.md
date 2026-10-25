# 收据 workers/receipt-key-assess-0033/ASSESSMENT.md

- 原件：`workers/receipt-key-assess-0033/ASSESSMENT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`86c3227b529da9d4f6d2b6e2118487ad68ef831dcac9111303dc9140db2373e8`
- 字节数：11912；行数：126

## 关键行（行号: 原文）

```
1: # #1488 评估 0033：同一次写的收据合并为精确 key
3: 基线：checkout `receipt-assess-checkout` detached ea363d5070。三个诊断/原型提交 d0dddf2146、b686d7ab00、2f432502d2 已逐一 revert（d5ac771b2d / 7fab350988 / b80fe41578），`git diff ea363d5070 HEAD` 为空。主工位未动。
5: ## 结论（先说）
12: ## 1. 实测读数
29: ### 字节成本（现状 vs 合并）
43: ## 2. 最小实施方案
45: ### 数据结构（effects.rs）
54: ### 改动点
67: ### 逐收据核对如何保留
78: ### 需证明不变的锁/语义
82: - construction_coverage 系列、P2-2/P3-b start_on 首块账（首块现在含 events 首块 `vec_bytes::<u64>(32)`，需同步首块常量）、close_stages / check_commit 的 ordered_events 口径。
85: ### 需新增的锁
95: ## 3. write_budget 剩余失败（不属 key 粒度）
109: - 合并 key 后 journal 峰值约 0.8 MB，只占父域很小一部分。即使把 journal 降到 0，originals 树的 2×3.46 MB 依然超限，所以与本评估的 key 合并正交。修复方向（留给主控另行裁定）：让 originals 树许可失败走既有的软降级或撤候选；或者降低 tree_slots 上界与重许可时的双持。
111: ## 4. 风险
116: - 首块账常量（start_on / P2-2 / P3-b）多了 events 首块，需要同步，否则首块断言会漂。
119: ## 附：分组口径对照
```
