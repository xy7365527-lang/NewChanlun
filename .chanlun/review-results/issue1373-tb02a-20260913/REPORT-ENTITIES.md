# #1373：既有报告实体与新集成边界

本件仅补齐已有报告实体并连接仓内索引。已逐字节复制 **20 份、259149 字节**：机械清单中的8项核心小件和12项既有小型来源/版本/扫描绑定，SHA-256全部与仓外原件一致。

这些报告及收据覆盖旧候选 `e7fe54e8cd041e18ea8e473a41a5621cbe90b693`（或报告各自明确的更早冻结源，且机械清单已核到该旧候选）。它们不宣称已验证当前与 main `abf5c0f8491fc066a0f2bc47ab4defdb120035c1` 合并后的源码。

[EVIDENCE-INDEX.json](EVIDENCE-INDEX.json) 的新增 `delivery_entity_update.entities` 逐项给出仓内路径、原始保全路径、SHA、大小和原索引ID；原113项冻结条目不改写。[REPORT.md](REPORT.md) 与 [CI-ADDENDUM.md](CI-ADDENDUM.md) 保持历史字节。

| 实体 | 仓内文件 | 字节 |
|---|---|---:|
| final-devskim-triage | [reports/ci-devskim-final/TRIAGE-SUMMARY.json](reports/ci-devskim-final/TRIAGE-SUMMARY.json) | 4393 |
| reused-triage-binding | [reports/ci-devskim-final/REUSED-TRIAGE-BINDING.json](reports/ci-devskim-final/REUSED-TRIAGE-BINDING.json) | 3418 |
| triage-rows | [reports/ci-devskim/TRIAGE-ROWS.json](reports/ci-devskim/TRIAGE-ROWS.json) | 151158 |
| rust-independent-result | [reports/independent-review-final-rust/RESULT.json](reports/independent-review-final-rust/RESULT.json) | 966 |
| q-independent-review | [reports/independent-q-review-post-fix/REVIEW.json](reports/independent-q-review-post-fix/REVIEW.json) | 1857 |
| harness-independent-review | [reports/ROOT-HARNESS-INDEPENDENT-REVIEW.json](reports/ROOT-HARNESS-INDEPENDENT-REVIEW.json) | 9595 |
| ci-rust-independent-review | [reports/ci-rust-review/REVIEW.json](reports/ci-rust-review/REVIEW.json) | 8726 |
| final-ci-record | [reports/ci-monitor-final/CI-FINAL.json](reports/ci-monitor-final/CI-FINAL.json) | 10061 |
| final-devskim-triage-text | [reports/ci-devskim-final/TRIAGE-SUMMARY.md](reports/ci-devskim-final/TRIAGE-SUMMARY.md) | 4485 |
| final-source | [reports/FINAL-SOURCE.json](reports/FINAL-SOURCE.json) | 5883 |
| rust-test-source | [reports/rust-checks/f1/TEST-SOURCE.json](reports/rust-checks/f1/TEST-SOURCE.json) | 1338 |
| rust-independent-preservation | [reports/independent-review-final-rust/PRESERVATION-MANIFEST.json](reports/independent-review-final-rust/PRESERVATION-MANIFEST.json) | 16379 |
| q-independent-manifest | [reports/independent-q-review-post-fix/MANIFEST.json](reports/independent-q-review-post-fix/MANIFEST.json) | 6787 |
| ci-rust-readonly-evidence | [reports/ci-rust-review/READONLY-EVIDENCE.json](reports/ci-rust-review/READONLY-EVIDENCE.json) | 2048 |
| final-scan-checkout | [reports/ci-devskim-final/CHECKOUT-BINDING.json](reports/ci-devskim-final/CHECKOUT-BINDING.json) | 453 |
| final-scan-diff | [reports/ci-devskim-final/DIFF-VS4408.json](reports/ci-devskim-final/DIFF-VS4408.json) | 2222 |
| final-scan-gate | [reports/ci-devskim-final/GATE-VERIFICATION.json](reports/ci-devskim-final/GATE-VERIFICATION.json) | 446 |
| final-scan-index | [reports/ci-devskim-final/EVIDENCE-INDEX.json](reports/ci-devskim-final/EVIDENCE-INDEX.json) | 25461 |
| final-scan-freeze | [reports/ci-devskim-final/REPORT-FREEZE.json](reports/ci-devskim-final/REPORT-FREEZE.json) | 2820 |
| final-scan-verified | [reports/FINAL-SCAN-RECEIPT-VERIFIED.json](reports/FINAL-SCAN-RECEIPT-VERIFIED.json) | 653 |

现成原件中保留的绝对保全路径和相对大证据引用继续指向其原始保全上下文；不能把它们理解成所有SARIF、数据库、截图或逐run目录也已复制。大型SARIF和原始保全目录没有纳入本次复制。

最终旧候选的 CI run 34765582958 四项成功，DevSkim run 34765582923 仍为 FAILURE；完整result多重集相同及122行用途分诊来自原报告。本次只复制并核hash，不重跑scanner、业务测试或独评。

## 新集成必须补入的证据

| 范围 | 待补项目 |
|---|---|
| `集成源与冲突解决` | 冻结最终集成commit/tree及完整差异，绑定父候选e7fe54e与main abf5c0f；明确继承未变源和本次变更。 |
| `s_session/browser/index.html` | 补合并冲突解决后的定向浏览器验证与独立差异审查，绑定最终HTML/client/测试源SHA；原Q独评只覆盖其旧冻结字节。 |
| `s_session/s_service_control.py` | 补合并冲突解决后的运行控制定向验证与独立差异审查，绑定最终源SHA；原驱动独评不自动覆盖合并后的变化。 |
| `.chanlun/agent-roster-2026-09-13.md` | 由集成主控完成名册冲突处理；本报告补齐工位不处理或改写roster。 |
| `最终集成CI与扫描` | 使用最终集成head的真实CI/scanner记录；保留旧34765582958/34765582923的e7fe54e身份。扫描红灯仍要最终候选实体分诊与显式例外，不把旧run改写为新head通过。 |
| `实体交付` | 主控提交后核20份报告实体及索引均在最终head且SHA相同；对新增报告尾部明确覆盖范围，再完成原有合入批准流程。 |

上述三个冲突路径来自本次开工时WI3的真实未合并路径清单，只记录交棒范围；本工位未修改这些文件。后续集成如果还有其他代码或检查入口变化，应按冻结差异纳入对应定向验证和独评。

当前状态：旧实体已落入工作树、待主控暂存提交；新集成的定向验证与独评待补。本件不授予合入，不关闭#1373、#1360或#1323。
