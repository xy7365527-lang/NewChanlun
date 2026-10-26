# 收据 workers/merge2-review-0084/REVIEW.md

- 原件：`workers/merge2-review-0084/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`437470775eb76ffcbf2ae6db8bc170ff1738a31401cff6533123e67fafad1195`
- 字节数：13903；行数：142

## 关键行（行号: 原文）

```
1: # #1488 第二次合并面非作者独审（0084）
3: - 评审对象：merge 提交 `026e3e97b04d0a54056830b87de438445edab9cf`（第一父 `26391ae34f` = #1488 源，第二父 `6c6acd5fe0` = main；merge-base `6badaa6049`）
7: - 主控裁定 D8：合并面上的语义冲突以 main 语义为准
9: ## 结论：CONDITIONAL
11: 合并面本身干净：没有文本冲突，两侧都没有被静默丢弃，没有隐性语义冲突，也**没有合并引入的失败（无 P1）**。
12: 但合入 main（squash 闸）之前必须先闭合两项 P2：
18: ## 1. 合并面逐块
37: ## 2. 隐性语义冲突
46: - **#1597 SourceCoords**：在评审时点确认**没有进 6c6acd5fe0**（merged tree 中 `git grep SourceCoords` 为空，作者“无需替换”在这个 main 点上成立）。但它已经以 PR #1628（`d4e6a20230`，2026-10-21T05:11Z）进入当前 origin/main，见 P2-1。
48: ## 3. ci.yml 合并结果
61: ## 4. 测试证据核实与失败归属
63: - **CI 39546640289**：`gh run view` 确认 workflow 为「#1488 统一所有权 编译与具名锁迭代」，headSha 是 026e3e97b0，结论 success，两组 uo job 都是 success。收据里共 101 个 PASS（组 0 是 4 PASS + 93 NOT_LISTED，组 1 是 97 PASS），锁清单有效行数 97 → 锁 97/97 属实。
66:   宽对拍子集（`/tmp/m83/wide-merge.txt`，745 行）是 706 ok / 23 FAILED / 16 ignored；`wide-failed-base.txt` 与 `wide-failed-merge.txt` 用 diff 比较，逐项相同。
72: | content_bi::tests::raw_coords_recover_after_refused_write_with_same_roots | FAILED（content_bi.rs:3882，两组 features） | ok（两组） | FAILED（同一处） | #1488 源自带；main 上为绿 |
73: | content_publication::tests::native_publication_begin_and_consumers_share_parent_scope（cfg native） | FAILED（content_publication.rs:2406） | ok | FAILED（同一处） | #1488 源自带；main 上为绿 |
74: | staged_nodes::staged_native_lease_uses_overlay_… | FAILED（staged_nodes.rs:765，left 0 / right 13） | 不存在 | FAILED（同一处同值） | #1488 源自带 |
75: | staged_nodes::staged_scoped_copies_hold_distinct_permits_until_payload_drop | FAILED（:675） | 不存在 | FAILED（同一处） | #1488 源自带 |
76: | staged_nodes::staged_selective_cut_follows_sql_parent_and_keeps_unselected_payload_charged | FAILED（:721 减法溢出） | 不存在 | FAILED（同一处） | #1488 源自带 |
77: | lib theta_v0::classifier::bsp::tests::no_stray_bsp_binding_rewrites_outside_registry | FAILED（36/1；锚 content_producer.rs:3109 不在白名单，白名单写的是 :3090，见 bsp.rs:679） | ok（37/37） | FAILED（36/1，同一处） | #1488 源自带。content_producer.rs 与 bsp.rs 在 merge 中都取 #1488 侧 blob，main 没有改。行号漂移来自分支上的 727b3bfb45 等提交 |
85:   结论：**没有任何一条是合并引入的 → 无 P1**。宽外 5 条和 bsp 锚都是 #1488 源自带，其中 3 条（content_bi、content_publication、bsp）在 main 上是绿锁，见 P2-2。
87: ## 5. 非测试 cargo check
95: ## 6. 生产 release（`--features s_session --bin s_structure_session`，去 build-id 并 strip）
97: | ref | nobid sha256 | 字节数 |
108:   - 结论：**差异全部来自 main 侧提交**。
114: ## Findings
116: ### P2-1 本 merge 相对当前 main 已过期；#1597 SourceCoords 的替换义务已经生效
123: ### P2-2 #1488 源自带 6 条红，其中 3 条在 main 上是绿锁；squash 后 main ci.yml 会变红
136:   #1594 报告（8933cca597，REPORT.md:60）已裁定：#1488 线上的红「合入 main 之前必须清零…不能以『基线既有』为由带进 main」。
137: - 不是合并引入，所以不判 P1；但它挡在 squash/合入闸前面。state 那条（整 bin 中红、单独运行绿）同样要在合入前给出归属或修复。
139: ### P3-1 #1488 两个 CI job 没有跟上 main #1588 的 job 约定
141: - 反例：main #1588 给所有 Rust job 都设了 `timeout-minutes`（兼作 #1406 挂死保护），并按 PR #1485 的 Codex P2 把 `always()` 收紧为 `!cancelled()`。#1488 这两个 job 没有设 timeout（默认 360 分钟），上传步骤仍是 `always()`；在新的 PR `cancel-in-progress` 下，被取消的 run 仍会执行上传。
```
