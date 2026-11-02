# 收据 workers/unified-ownership-implementation-0018/MOVEIN-ACKNOWLEDGMENT-PROOF.md

- 原件：`workers/unified-ownership-implementation-0018/MOVEIN-ACKNOWLEDGMENT-PROOF.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`4e1c37627150271ca011bd1cad2c0cae480470669fadc88b93c326c229a0cfd6`
- 字节数：12222；行数：297

## 关键行（行号: 原文）

```
1: # MoveIn 追认证明文档
3: ## 问题陈述
9: ## MoveIn 机制分析
11: ### 时间线
29: ### MoveIn 覆盖的载荷来源
39: #### Capture 构造路径（state.rs:156-192）
54: #### Legacy → Native 转换（s_content.rs:3463-3550）
99: ### 关键断裂点：original_fees 在 v3 中为 None
101: #### AuditCapture 默认构造（s_content.rs:3390-3401）
112: #### original_fees 设置路径
120: **结论**：s_session_v3 路径中，`original_fees` 始终为 `None`，**L3527 转换的 Plain Vec 没有附加任何许可**。
122: ## 分配时序证明
124: ### Plain Vec 的分配点
144: ### MoveIn 的事后预留
176: ## v4 合同条款对比
178: ### v4 合同禁止条款（第 10 条）
182: ### MoveIn 是否违反？
189: ### 与 adopt 的区别
196: ## 修复路径建议
200: ### 改造方案
214: ### 实施难点
220: ### 当前建议
223: > **当前判定**：向独审提交证明，说明 MoveIn 属于追认，需要更大范围改造才能修复。
229: ## 逐分配点证明（反证）
237: ### 需要覆盖的字节清单（来自 L1719-1738）
244: ### 逐项检查
246: #### 1. node bodies capacity（Plain Vec）
252: **结论**：body capacity **无真实许可覆盖**。
254: #### 2. HashMap index payload（转换时分配）
259: **结论**：HashMap 索引 **有真实许可覆盖**。
261: #### 3. codec roots capacity
281: ### 反证失败
285: **最终结论**：MoveIn **属于追认**，违反 v4 合同第 10 条。
287: ## 提交独审
297: - **若要求本轮修复**：明确扩大写集范围及授权边界。
```
