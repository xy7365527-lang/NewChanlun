# 收据 workers/p24-0078/REVIEW.md

- 原件：`workers/p24-0078/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`17464cc28d0bc5b3cc311feaea0c4432926396c280d1fe7ecede77d07935924b`
- 字节数：10402；行数：67

## 关键行（行号: 原文）

```
1: # #1488 0078：P24 非作者复核：冷根残留报错后 writer 不退役，现在还能不能被观察到
5: - 已读：P3-REGISTER P24 行；uo-independent-review-0030 P3-ζ（REVIEW.md:137-139）；`12635bd142`（wide16-0058）；degrade-0065/REPORT.md §4（:76）；fix-0072/REPORT.md。
7: ## 结论
9: **verdict = UNOBSERVABLE**（只针对 P24 本身）。
13: 3. 所以 P24 不需要新增锁，建议按「接受并注明」结案。下面 §4 给了一个可选的轻量锁形态。
15: **另外发现一个相邻缺陷（不是 P24，建议新开登记，§5）**：冷根失败（`left == 0`）之后，本请求在没有候选的情况下写入，`before_write` 会撤销 Carry 租约，而且不会续租。之后只要候选重新出生，同一个 writer 上的每个 ingest 都会报 `StaleWriter：construction资格失效：Carry租约已撤销`，ready 照常成功。探针 H 不做任何注入就能复现。这件事和退不退役无关，退役只会让情况更糟。
17: ## 1. 代码路径（3ee05a2713）
35: ## 2. 远端探针证据
50: ## 3. D(16M)/F/G 里的 StaleWriter 是怎么来的（排除它与"不退役"有关）
53: - F 里"跨 driver 污染"（k=0 泄漏、k=1 的新 driver 出现 STALE）是探针本身造成的：在 1M 下首跑先自然失败，`first.and_then(...)` 不会调用 `inject_failure`，于是 `FAIL_NEXT` 一直挂着，留到下一个 driver 的第一次冷根才被消费，这就等同于 `inject16M@0`。真实进程里没有这个开关。
55: ## 4. P24 处置建议
61: ## 5. 相邻新发现（不是 P24）：冷根失败后 Carry 租约不续，候选重新出生后 ingest 恒报 StaleWriter
67: - 建议：新开 P 条目或交主控裁定。可能的修法是在无候选发布提交后，按 `renew_lease_after_abandon` 的同一组条件续租（`ended.complete.is_none()` 且 owner 无 construction/complete、未退役、autocommit）。锁可以直接用探针 H 的形态：1M 自然冷根失败 → 改 16M → ingest 必须 Committed 且与 ordinary oracle 一致。
```
