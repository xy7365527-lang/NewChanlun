# #622 S2 修复轮收口报告（claude sonnet）

> 日期：2026-07-29；性质：修复轮 stdout 收口报告原文（影子评审 HIGH-1/M1/M2/M3）
> 原始日志：/tmp/issue622-fixround-20260729.log

## 收口报告

**1. Commit**：`d8d56a93e2` — `fix(theta): #622 S2 修复轮——改口处死知情时护栏 + 三 MEDIUM 钉正（影子评审）`，5 文件 +303/-33。

**2. 指纹对照**：
- 基线：`cargo test --lib` 2180/0/137；`cargo test --lib retrace_ledger` 97/0；
- 收口：`cargo test --lib` **2185/0/137**（+5）；`cargo test --lib retrace_ledger` **102/0**（+5）；retrace_ledger 模块零新增 warning；book.rs 741 行（<800）。

**3. 逐项落位**：

| 项 | 落位 | 测试 |
|---|---|---|
| HIGH-1 | `book.rs:kill_as_rebased` 落锤前校验 `as_of ≥ entry.last_as_of`，违反 → `Err(RetraceRejection::RebaseAsOfBehindGate)`（新变体，`adapter.rs`）+ 新拒收码（`audit.rs`）；`guard_single_active`/`reconcile_window` 均改签名传播 `?` | `tests/rebase.rs`：`observe_collision_rebase_with_as_of_behind_registration_is_rejected_fail_loud`（p9b）、`..._behind_advanced_gate_but_after_registration_is_rejected`（p2 弱变体）、`reconcile_window_rejects_as_of_behind_gate_fail_loud`（p9） |
| MEDIUM-1 | `advance_existing` 先算 `is_pending`，两拍守卫仅 `is_pending && outcome.is_some()` 时施加；doc 声明「裁定三字面优先」 | `tests/two_pass_evidence.rs::contradictory_late_evidence_after_terminal_is_silently_absorbed_not_rejected` |
| MEDIUM-2 | 删 `RetraceLedger.center_rebased` 字段，`alarms()` 改调 `count_center_rebased()`（现算 `book` 中 `Invalidated{CenterRebased}` 条目数） | `tests/rebase.rs::center_rebased_survives_fold_journal_round_trip` |
| MEDIUM-3 | `RetraceStep` 新增 `killed: Option<RetraceKey>`，`step()`/`guard_single_active`/`reconcile_window` 全线贯通；未加消费方专用面（评估后判定「便宜就做」，已做） | `tests/rebase.rs::same_departure_different_window_kills_old_as_rebased_and_registers_new` 扩展断言 `step.killed` + `step.delta` 内容 |

**4. 偏离/存疑**（需明确报）：
- 票面「绝不碰」列出了 `adapter/log/audit`，但 HIGH-1 要求的「新拒收码」按本模块既有架构（`DeadCenterReentry`/`TerminalEvidenceContradictsRegistration` 先例）必须落在 `adapter.rs` 的 `RetraceRejection` 枚举 + `audit.rs` 的 `RetraceRejectionCode` 枚举——脱离这两处无法实现「fail-loud 拒收报错 + 新拒收码」这一明确要求。开工前 `git status` 已确认这两个文件当时无并发改动，故按既有模式做了最小增量（各加一枚变体），未改动任何既有逻辑。这是两条票面指示的直接冲突，我按「新拒收码」的字面要求判定为必要动作，如实报出，未擅自问询。
- MEDIUM-1 涉及的 `dead_center.rs` 既有「一致证据迟到吸收」测试（`same_identity_late_input_after_confirmed_is_silently_absorbed_not_rejected`）保持绿，未改动该文件。
- 未触碰 `portal.rs`/`tests/short_retrace.rs`/`tests/standby.rs`/`ledger_kernel`/`#576` 面，`git status` 复核确认。
