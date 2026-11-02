# 收据 workers/squash-review-0091/RESULT.json

- 原件：`workers/squash-review-0091/RESULT.json`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`16f2eaed9cf01ecf49001aad4ed11dd019298c4764a2843d614fc8bbeaa4aaf0`
- 字节数：4557；行数：49

## 关键行（行号: 原文）

```
3:   "verdict": "FAIL",
22:       "severity": "P1",
24:       "summary": "P58 fmt moved content_producer.rs raw_bar_at binding line 3109→3114; bsp whitelist anchor not updated; no_stray_bsp_binding_rewrites_outside_registry fails",
30:       "severity": "P2",
32:       "summary": "28 unconditional allow lines with reason '各构建组合均无使用方（可能只在其他 testonly feature 下用）' are false or speculative; main used precise cfg_attr",
37:       "severity": "P2",
39:       "summary": "native profile (s_session,s_capture_native_testonly) RUSTFLAGS=-D warnings check --all-targets fails; main had 0 warnings there, squash has 59",
42:     {"id": "P3-1", "severity": "P3", "file": "rust/src/bin/s_session_v3/request_audit.rs:478; rust/src/s_content.rs:8009", "summary": "fetch_update reintroduced; main #1632 renamed all to try_update"},
43:     {"id": "P3-2", "severity": "P3", "file": "rust/src/bin/s_structure_session.rs:867", "summary": "parse_batch_value deleted by main, revived by ours with allow"},
44:     {"id": "P3-3", "severity": "P3", "file": "rust/src/bin/s_session_v3/object_index.rs:38", "summary": "whole use group cfg_attr(not(test), allow(unused_imports)) coarser than main"},
45:     {"id": "P3-4", "severity": "P3", "file": "rust/src/s_content.rs:~8180", "summary": "strong_count == 268 exact count is brittle; document derivation"},
46:     {"id": "P3-5", "severity": "P3", "file": ".chanlun/review-results/1488-acceptance/close-0085/CLOSE-DECLARATION.md:73,90", "summary": "stale line refs (mod.rs:2489→2465, :2593→2574, cache.rs:779→781, s_structure_session.rs:5478→5483/5486) and main 21b145250e ci.yml reference"}
```
