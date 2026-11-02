# 收据 workers/p24-0078/RESULT.json

- 原件：`workers/p24-0078/RESULT.json`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`e6941fdc87ba0f79dd4fcd79b40c6a8344b706866378418106c93164446a69b9`
- 字节数：2473；行数：32

## 关键行（行号: 原文）

```
4:   "verdict": "UNOBSERVABLE",
5:   "summary": "残留非零的 fail-closed 分支（audit_owner.rs:1960-1969）在现路径下只能靠测试注入走到：188 次自然或最坏时点注入的冷根失败，left 全部为 0。强行注入残留后，只有触发它的那个请求被拒；同一 writer 的后续请求（包括原样重发）回复与基线一致，state_summary 相同；泄漏字节不进入 S7 分区，后续请求不被拒，账也没有错。",
24:     "verdict": "PLAUSIBLE",
25:     "summary": "冷根失败（left==0）后，无候选写口 before_write 撤销 Carry 租约且不续租（只有 renew_lease_after_abandon 和 pager 探针会续租）；候选重新出生后，同一 writer 上每个 ingest 都报 StaleWriter：Carry租约已撤销，ready 正常。",
```
