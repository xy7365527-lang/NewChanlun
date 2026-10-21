# 收据 workers/r14-0053/REPORT.md

- 原件：`workers/r14-0053/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`46cea47ec763fa2be0d7b1479b5377fa2450607edda95c7af36655d780f8f09b`
- 字节数：5705；行数：66

## 关键行（行号: 原文）

```
1: # #1488 r14 期望值改由合同字符串长度推导（0053）报告
3: ## 1. 基本信息
10: | 最终 HEAD | `c2bdf9b662fc94a58f278da6660d585eb0233598`（tree `2e46c05d4ac227886718032f124f3c04d6ee7b04`，与实施提交 tree 相同） |
11: | 裁定 | 主控裁定 (b)，第五处裁改断言：https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6283672661 |
12: | 锁 | 快通道 80/80；CI run 39373166910 success（组1 80/80 PASS，组0 1 PASS + 79 NOT_LISTED，与基线同形） |
14: ## 2. 推导式与依据
17: `claims.1`（①来源合同额之和），以及恒等式 `P2-A前封存 − sealed = ① + ②` 中的 P2-A 前封存基准（原字面 527,048）。
23: - P2-A 前封存同时含正文物理许可一次和同额来源许可一次，所以合同串长在其中计两次（2L）。
27: - P2-A 前封存 = 526,876 + 2L。旧 L 时为 527,048。
28: - #1405 改动：policy `standard_raw_gap_3` → `standard_raw_gap_3;same-price/keep-earlier+extreme-region/1`（18→59，+41）；knowledge_domain `established_direction_without_extreme_identity_competition` → `established_direction_with_extreme_root_sets`（58→44，−14）；schema `s-new-bi/1`→`/2` 长度不变。ΔL = +27，① 从 111,3
31: **独立性**：L 由测试对 `crate::tb02b::CURRENT_CONTRACT` 的三个字段显式 `len()` 相加得到（按裁定条件取合同字段）。不调用产生合同额的 `SourceClaim::on_body`，也不读正文计费；节点数用测试内常量 `R14_NODES = 803`，不用实测 `nodes` 参与推导。② 的 16,640 与其余断言未改动。
33: ## 3. 交叉核与负控
37: | 节点数交叉核 | `assert_eq!(nodes, R14_NODES)`（803） | PASS |
38: | 旧串手算基准（测试内常驻） | 字面旧串 `contract_len(...) == 86`；推导 ① == 111,334；P2-A 前封存 == 527,048 | PASS |
39: | N1 变异负控（只改推导式一处） | `dd03f0381b`：current_len 漏掉 knowledge_domain | **FAIL** @audit_owner.rs:4306 「①来源合同额之和」left 111361 ≠ right 111317（`.chanlun/review-results/1488-acceptance/receipts/20261020-125926-dd03f0381b.md`）；已 revert `ffca623303` |
40: | N2 还原负控（真实还原两串） | `2f8aa72060`：tb02b 的 policy/knowledge_domain 改回旧值 | **PASS**；`--nocapture` 普查结果：claims 111334，contract_len 86，sealed 399074，sealed_before_p2a 527048（`.chanlun/review-results/1488-acceptance/receipts/20261020-130308-2f8aa72060.md`）；已 revert `c2bdf9b662` |
42: ## 4. 提交表
52: ## 5. 验证
54: - 快通道 r14：PASS（`.chanlun/review-results/1488-acceptance/receipts/20261020-125745-b8208e722d.md`）。
56: - CI `uo-ci-fixed.sh` run **39373166910**：结论 success。组 `uo (s_session)` 为 1 PASS + 79 NOT_LISTED（feature 门控，与基线同形）；组 `uo (s_session,s_capture_native_testonly)` 为 **80 PASS**，r14 PASS。原件在 `workers/uo-ci-runs/39373166910/`。编译告警都在本次写集之外，是既有告警。
58: ## 6. 未闭合项
60: - 0052 合并面独审需要覆盖本次改动的独立性（裁定要求，本工位不能自审）。
64: ## 证据引用迁移（0077 P30，2026-10-21 追加）
66: 本文原引用的 5 份 `/tmp/nc-remote/<run-id>.log` 已改为仓内收据 `.chanlun/review-results/1488-acceptance/receipts/<run-id>.md`（相对工位 checkout 根；关键行 + 整文件 sha256，原日志名与 run-id 一一对应）。索引见 `.chanlun/review-results/1488-acceptance/receipts/INDEX.md`。
```
