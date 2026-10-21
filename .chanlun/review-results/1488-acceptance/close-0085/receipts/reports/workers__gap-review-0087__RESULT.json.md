# 收据 workers/gap-review-0087/RESULT.json

- 原件：`workers/gap-review-0087/RESULT.json`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`664ee3d3a30f79e12f759ba371dcc27db814d4c13731c7f49159183842ac6154`
- 字节数：7242；行数：89

## 关键行（行号: 原文）

```
12:       "verdict": "CONDITIONAL",
14:         {"sev": "P2", "id": "1-F-1", "loc": "rust/src/bin/s_session_v2/cache.rs:396-435 (026e3e97b0)", "summary": "remove_cert_admitted 先消 certs_bytes/清 batch_headers，后在 :428 insert_admitted 才可能 Err；Err 时 payload 不在任何字节账，thread_local MEMO 跨请求存活，make_room_admitted 少算可超 limit 准入（仅 testonly 臂可达）"},
15:         {"sev": "P2", "id": "1-F-2", "loc": "rust/src/bin/s_session_v3/audit_store.rs:63", "summary": "逐插入×depth 保守计费原样存活（与 368e2bcb13 同型，959d 后零提交）；已登记 #1612，仅确认残留"},
16:         {"sev": "P3", "id": "1-F-3", "loc": "rust/src/bin/s_session_v2/cache.rs:273/320/357/394/574/831/879/1136/1215", "summary": "None 包装器 expect 在 native-bound 树混用时可 panic；外部调用仅 cert_commit_admitted，未确证（子代理原报 P2，本体降级）"},
17:         {"sev": "P3", "id": "1-F-4", "loc": "rust/src/bin/s_session_v3/audit_store.rs:193-198", "summary": "NativeMap::insert 用 release assert 代替 Err"},
18:         {"sev": "P3", "id": "1-F-5", "loc": "rust/src/bin/s_session_v3/audit_owner/native.rs:2334", "summary": "测试模块残留 NOT_RUN 注释"},
19:         {"sev": "P3", "id": "1-F-6", "loc": "rust/src/bin/s_session_v3/audit_store.rs:92-104", "summary": "NativeTree 节点许可仅 clear/drop 返还（设计确认）"}
26:       "verdict": "PASS",
28:         {"sev": "P3", "id": "2-P3-1", "loc": "rust/src/bin/s_session_v3/aux_index.rs:724-726/785-787/851-853", "summary": "未变行跳过顺带跳过本代对象 verify_many_ref_fresh/verify_witness_id 交叉核，防御深度收窄"},
29:         {"sev": "P3", "id": "2-P3-2", "loc": "d4686c7a09 supersedes Δ_R", "summary": "依赖 raw 只追加 + frontier 同事务两不变量，成立，建议写入合同注释"},
30:         {"sev": "P3", "id": "2-P3-3", "loc": "bi 锁", "summary": "冷暖臂同一 sync() 路径，非独立 oracle，由规范闭包核验兜底"},
31:         {"sev": "P3", "id": "2-P3-4", "loc": "负控证据", "summary": "10-20 14:07–15:23 批属并发窗口，已由 10-21 04:1x target-r71 清洁批全数复现"}
38:       "verdict": "PASS",
40:         {"sev": "P3", "id": "3-P3-1", "loc": "rust/src/bin/s_session_v3/cross_request_source_cache.rs:1092,1341", "summary": "B8/B9 peak>scope_before 可被先前峰值满足，缺绑定零计费型负控"},
41:         {"sev": "P3", "id": "3-P3-2", "loc": "998ce0218b", "summary": "cap142 红形态为 :1189 codec 工作域分配失败，与提交消息 BudgetExceeded 措辞不符"},
42:         {"sev": "P3", "id": "3-P3-3", "loc": "cross_request_source_cache.rs:1184", "summary": "143MiB 为 MiB 粒度最小值（实测峰 149,307,139B）"},
43:         {"sev": "P3", "id": "3-P3-4", "loc": "#1488 票面", "summary": "B11 生产不可达证明经本审核码成立，但未见主控明文追认"},
44:         {"sev": "P3", "id": "3-P3-5", "loc": "rust/src/bin/s_session_v3/audit_owner.rs:2264", "summary": "q oracle 与被测共用 heap_bytes() 的残余共模"}
51:       "verdict": "PASS",
53:         {"sev": "P3", "id": "4-P3-1", "loc": "rust/src/bin/s_session_v3/prepared.rs:2266", "summary": "共享父后 parent.peak()>baseline 对 gen2–4 近乎恒真（32c5ec7c04 区分力损失，作者已披露）"},
54:         {"sev": "P3", "id": "4-P3-2", "loc": "rust/src/bin/s_session_v3/object_index.rs:4967", "summary": "oracle 撤回行每桶至多一行，桶内顺序未锁（非回归）"},
55:         {"sev": "P3", "id": "4-P3-3", "loc": "audit_store.rs:63 / business_json.rs:1903", "summary": "×depth 计费与 368e2bcb13 修后口径不一致（#1612）"}
62:       "verdict": "CONDITIONAL",
64:         {"sev": "P2", "id": "5-P2-1", "loc": "rust/src/bin/s_session_v3/driver.rs:5270 (809710da97)", "summary": "6 个红负控全被增量臂生产自检拦截，跨臂对拍断言（:5297/:5306/:5314-5321/:5331）与 Counts 对拍（:5303）在合入形态无变异证据"},
65:         {"sev": "P2", "id": "5-P2-2", "loc": "rust/src/bin/s_session_v3/driver.rs:5314-5334 (809710da97)", "summary": "节点表口径在 g20 真红后由全表逐位放宽为可达闭包逐位，冷/增量存储层分叉（g28 冷臂多 162 不可达节点）只记数不锁，需负责人确认"},
66:         {"sev": "P3", "id": "5-P3-1", "loc": "prepare_source_audit object_phase/dirty", "summary": "增量分支本数据形态不可达、无锁覆盖（已转 D6/#1491）"},
67:         {"sev": "P3", "id": "5-P3-2", "loc": "a4f1902f21", "summary": "N3c 探针跑在共享 target，由 N3h 佐证"},
68:         {"sev": "P3", "id": "5-P3-3", "loc": "rust/src/bin/s_session_v3/driver.rs:5142-5148", "summary": "s4_reachable tag→offset 表硬编码，未知 tag 静默不展开"},
69:         {"sev": "P3", "id": "5-P3-4", "loc": "rust/src/bin/s_session_v3/audit_owner.rs:4820", "summary": "read_end().unwrap(); actual 同行，未 fmt"}
75:     "target": "target-r87 (serial, --release, features s_session,s_capture_native_testonly)",
```
