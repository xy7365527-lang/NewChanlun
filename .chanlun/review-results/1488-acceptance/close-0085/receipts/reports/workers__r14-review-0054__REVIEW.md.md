# 收据 workers/r14-review-0054/REVIEW.md

- 原件：`workers/r14-review-0054/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`610cfa22b7fadb6dc7ab5b42cd337350ae94aa6ed7a623417885c06e860cba85`
- 字节数：5814；行数：46

## 关键行（行号: 原文）

```
1: # #1488 r14 推导期望（第五处裁改断言）非作者独审（0054）
3: - 被审：`git diff 57f9680c4e c2bdf9b662 -- rust/ .github/`（实施 b8208e722d + N1/N2 负控及其 revert）
4: - 依据：主控裁定 (b)（#1488 评论 6283672661）；0052 P1-1 / P2-1；作者报告 workers/r14-0053/REPORT.md
5: - 评审方式：只读。没有运行新的测试，所有结论都基于作者和 CI 的原始日志、源码和算术复核。
7: ## 结论
9: **通过（APPROVED_WITH_FINDINGS）。** 推导没有依赖被测的计费路径；sealed 恒等式按 +54（2L）处理，结果正确；N1/N2 负控日志真实；80/80 锁和 CI 39373166910 属实；改动只涉及 audit_owner.rs 一个文件。另有一条 P2，需要主控确认负控的口径；它不阻塞合入门。
11: ## 逐项核对
13: | # | 核对项 | 结论 | 证据 |
15: | 1 | 推导独立于被测代码 | PASS | 期望值 = `(33_357+L)+97·803` 和 `526_876+2L`。L 只由 `CURRENT_CONTRACT` 三个字段的 `len()` 求和得到，这三个字段是**输入串**，不是计费输出；推导过程中没有调用 `SourceClaim`、正文计费或 `native_source_claims_*`；节点数取测试常量 `R14_NODES`，实测 `nodes` 只用来做交叉核。被测量 `claims.1` 和 `sealed` 都是实测值，因此不会构成自我对照 |
16: | 2 | sealed 恒等式 +54 口径（0052 P2-1） | PASS | `sealed_before_p2a_for(L)=526_876+2L`：L=86 时为 527,048，L=113 时为 527,102（+54）。合并面实测 sealed 399,101：527,102−399,101=128,001=111,361+16,640 ✓。代入后，这条恒等式实际把 sealed 锁在 398,988+L（L=86 时为 399,074，与旧文档一致），所以不是恒真式。常量复核：33,443−86=33,357；527,048−172=526,876 ✓ |
17: | 3 | 串长 | PASS | 当前 schema/policy/domain 长度为 10/59/44，L=113；旧值为 10/18/58，L=86；ΔL=+27 ✓ |
18: | 4 | N2 还原负控（两串还原后回到 111,334 且通过） | PASS | `2f8aa72060` 只改了 tb02b.rs 的 policy/knowledge_domain 两行。日志 `.chanlun/review-results/1488-acceptance/receipts/20261020-130308-2f8aa72060.md` 中 sha 一致，`R14_P2B_DROP` 为 claims 111334、contract_len 86、sealed 399074、sealed_before_p2a 527048，测试 ok |
19: | 5 | N1 改一处变红 | PASS（口径见 P2-1） | `dd03f0381b` 只改一行（domain 改为 `""`）。日志 `.chanlun/review-results/1488-acceptance/receipts/20261020-125926-dd03f0381b.md` 中 sha 一致，contract_len 69，在 audit_owner.rs:4306 报 left 111361 / right 111317，结果 FAILED |
20: | 6 | 负控已 revert | PASS | b8208e722d 与 c2bdf9b662 的 tree 都是 `2e46c05d…`，二者相同 |
21: | 7 | 锁 80/80 | PASS | `.chanlun/review-results/1488-acceptance/receipts/20261020-130409-c2bdf9b662.md` 的 sha 为 c2bdf9b662，结果 80 passed / 0 failed；锁清单 80 条，与此前 1baf57e760/868b8eb1df/87e9965b3b 的 80 条清单逐条相同，含 r14 |
22: | 8 | CI 39373166910 | PASS | `gh run view` 显示 headSha c2bdf9b662，conclusion success，两个 job 都 success；locks.tsv 中组1 为 80 PASS，组0 为 1 PASS + 79 NOT_LISTED |
23: | 9 | 其余代码无改动 | PASS | 57f9680c4e..c2bdf9b662 全仓 diff 只有 audit_owner.rs（+48/−8）；r14 以外只在 `seal_census_check` 文档补了一行注释；② 的 16,640 断言未动 |
25: ## 发现
27: ### P2-1｜裁定中“仅改一处时锁须变红”的口径需要主控确认（不阻塞）
30: - 另一种读法：裁定的语境是“两字符串还原”负控，所以“仅改一处”也可以理解为**只还原 tb02b 中的一个串**。在 (b) 的设计下，期望值会跟着 `CURRENT_CONTRACT` 变化，这种情况下锁会保持**绿**。例如只还原 policy 时 L=72，期望值和实测值都是 111,320（推断，未实跑）。这正是 (b) 想要的效果，但和后一种读法的字面要求相反。
33: ### P3-1｜L 没有计入 `semantic_version`（`stroke::RULE`）
35: - 现状：#1405 中 `new-bi-dual-coordinate/1` 改为 `/2`，长度都是 24，对当前数值没有影响。裁定要求“policy、knowledge_domain **等计入项**逐项求和”。如果以后 RULE 的长度变了（例如变成 `/10`），推导不会跟着变，锁会变红（结果偏保守，不会漏报）。
38: ### P3-2｜常驻的“旧串手算基准”断言只是常量的算术自洽
44: ## 证据引用迁移（0077 P30，2026-10-21 追加）
46: 本文原引用的 3 份 `/tmp/nc-remote/<run-id>.log` 已改为仓内收据 `.chanlun/review-results/1488-acceptance/receipts/<run-id>.md`（相对工位 checkout 根；关键行 + 整文件 sha256，原日志名与 run-id 一一对应）。索引见 `.chanlun/review-results/1488-acceptance/receipts/INDEX.md`。
```
