# 收据 workers/gap-review-0087/REVIEW.md

- 原件：`workers/gap-review-0087/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`672b81b63859c1300203b13f02d1061cee1b09c33181feaef0122250fd7eea4a`
- 字节数：13501；行数：143

## 关键行（行号: 原文）

```
1: # #1488 0087 评审链缺口补审（非作者独审，只读）
3: - 评审对象：REVIEW-CHAIN（close-0085）§2 的四段缺口 ①–④，以及 §4 的 accept-s4 分支 ⑤。
4: - 依据：主控裁定 issuecomment-6287615661（B1–B11 和夹具修复）、6298532692（第 17 处）；CLOSE-GATE-DRAFT §4 的十七处清单。
5: - 方法：五段各由一个只读评审子代理做静态逐 hunk 审读，各段细节见同目录 `seg-1.md` 到 `seg-5.md`。本体复核了关键结论：revert 树逐对机械校验、seg-1 F-1/F-3 读码、seg-5 生产 cfg 门。远端复跑只用 `NC_TARGET=target-r87`，串行执行（§6）。
8: ## 0. 总表
10: | 段 | 区间 | verdict | P1 | P2 | P3 |
12: | ① | `813e1f92b3..959d860065` 存活 hunk | **CONDITIONAL** | 0 | 2（F-1、F-2，F-2 已登记 #1612） | 4 |
13: | ② | `c2bdf9b662..bba547492d`（S3） | **PASS** | 0 | 0 | 4 |
14: | ③ | `df0b98f0f7..727b3bfb45`（第 6–16 处） | **PASS** | 0 | 0 | 5 |
15: | ④ | `78523300ad..44d312d7cf`（five-0067） | **PASS** | 0 | 0 | 3 |
16: | ⑤ | `c2bdf9b662..809710da97`（accept-s4） | **CONDITIONAL** | 0 | 2 | 4 |
17: | **总** | | **CONDITIONAL** | **0** | **4** | 20 |
19: ## 1. 段① WIP 检查点存活 hunk（CONDITIONAL）
24: - 以下八个文件在 959d 之后零提交、全量存活，此前从未被审过：audit_store.rs、audit_retention.rs、audit_json.rs、native_owned.rs、closed.rs、fragment.rs、path.rs、ref_owner.rs。
25: - 生产路径没有发现 P1。Native 账务被 `s_capture_native_testonly` feature 门隔离：生产臂对 `Some(work)` 一律拒绝；v2 cache 生产 `limit=0`，缓存停用。
28: - **P2 ①-F-1** `rust/src/bin/s_session_v2/cache.rs:396-435`（`remove_cert_admitted`）
33: - **P2 ①-F-2（已登记 #1612，本段只确认残留）** `rust/src/bin/s_session_v3/audit_store.rs:63`
37:   - 子代理原报 P2。本体复核后降为 P3：仓内对 v2 cache 的外部调用只有 `evidence_store.rs:1845` 的 `cert_commit_admitted`，没有找到 testonly 臂里 native-bound 树混用 None 入口的实际调用时序，所以未确证。
42: ## 2. 段② S3（PASS）
58: ## 3. 段③ 第 6–16 处裁改断言（PASS）
60: - **B1–B10**：改写与待裁表、裁定 6287615661 逐义一致。全部是 `>` 收紧为 `==`，没有删除有区分力的检查。
61:   - 每处注释都带裁定链接、原断言原文和 v4 条款。
78: ## 4. 段④ five-0067（PASS）
94: ## 5. 段⑤ accept-s4（CONDITIONAL）
107:   - HEAD 809710da97 撤掉 0064 探针后零残留。
108: - **负控**：6/6 个阶段负控在私有 target-s4 上真红，失败都在目标锁的预期位置，时间都在 15:38 之后。唯一例外是 N3c 探针跑在共享 target 上，结论有 N3h 佐证（P3）。
109: - **P2 ⑤-P2-1** `rust/src/bin/s_session_v3/driver.rs:5270`
113: - **P2 ⑤-P2-2** `driver.rs:5314-5334`
122: ## 6. 远端复跑（target-r87，串行，`--release --features s_session,s_capture_native_testonly --bin s_structure_session -- --exact`）
132: - 日志摘录和 sha256 在 `logs/` 下，完整日志在 `/tmp/nc-remote/`。
134: - 复跑抽样的局限：S4 锁只验了一条阶段负控（N3g）。这正好印证了 ⑤-P2-1：红点在增量臂的生产自检，不在跨臂对拍断言上。
136: ## 7. 总 verdict：CONDITIONAL（无 P1）
139: 1. ①-F-1：修正错误路径的转记顺序，或证明该 Err 路径不可达。只在 testonly 臂可达，不阻断生产，但 v4 合同路径的账务不应留洞。
141: 3. ⑤-P2-1：0086 并入前，补一条只让跨臂对拍断言变红的负控，或由负责人书面认可。
142: 4. ⑤-P2-2：负责人确认“可达闭包逐位”口径，存储层分叉转回收评估。
143: 5. 建议（P3，不阻断）：主控对 B11 不可达证明补一句明文追认（③-P3-4）。
```
