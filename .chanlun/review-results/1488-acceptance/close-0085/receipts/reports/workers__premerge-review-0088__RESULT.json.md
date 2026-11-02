# 收据 workers/premerge-review-0088/RESULT.json

- 原件：`workers/premerge-review-0088/RESULT.json`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`cf62c8abc291d9ba3ce2597afe662937a6c1e9eca41a7b3413163b8534b15243`
- 字节数：5828；行数：64

## 关键行（行号: 原文）

```
4:   "verdict": "CONDITIONAL",
22:       {"id": "P3-1", "location": "rust/src/bin/s_session_v2/content_publication.rs:2445", "summary": "尾部消息“保守持有到父预算结束”已陈旧；缺 drop(actual) 后 live==before 全额还账断言"},
23:       {"id": "P3-2", "location": "rust/src/s_content.rs:8130", "summary": "第十九处在场 strong_count>1 可收紧为精确计数"},
24:       {"id": "P3-3", "location": "rust/src/s_content.rs:8701", "summary": "测试名 releases_physical_body_only 与新断言（drop 后全额 0）不符"},
25:       {"id": "P3-4", "location": "rust/src/bin/s_session_v2/staged_nodes.rs:669-723; rust/src/bin/s_session_v3/state.rs:2893; rust/src/bin/s_session_v2/content_bi.rs:3868-3881", "summary": "改断言/改夹具未入裁改清单；本审判定未放宽，应列为新增裁改断言交负责人"},
26:       {"id": "P3-5", "location": "rust/src/theta_v0/parser/inclusion.rs:1780,1877,1900,1962", "summary": "delta/rewind 测试参照 process_inclusion_with_facts 在 main 已改走 IncrFactsState（inclusion.rs:612），参照不独立；应改比 _flat（:631）"},
27:       {"id": "P3-6", "location": ".github/workflows/ci.yml:617,741", "summary": "upload-artifact 缺 overwrite: true（main 约定 ci.yml:287）"},
28:       {"id": "P3-7", "location": "rust/src/bin/s_session_v3/state.rs:2893", "summary": "main #1394 测试在 native feature 下期望缺陷，应回报 main"}
43:     "target": "target-r88",
```
