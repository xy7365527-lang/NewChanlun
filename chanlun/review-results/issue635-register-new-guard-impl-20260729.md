# #635 register_new 护栏实施报告（claude sonnet）

> 日期：2026-07-29；性质：实施车 stdout 收口报告原文
> 原始日志：/tmp/issue635-dispatch-20260729.log

#635 收口报告:

**1. A/B 判定**:选 **方案 A**(叠加护栏),非"不可达"论证。

理由:`register_new` 内 `guard_single_active` 一旦命中"同 departure、窗口变"分支,会调用 `kill_as_rebased` 处死旧候选——这条路径在恢复语义下**可达**(#622 起就有生产用例,`rebase.rs` 里 `same_departure_different_window_kills_old_as_rebased_and_registers_new` 等测试即证明其常态触发),不存在可以论证"不可达"的空间。此前 #632 的恢复点护栏(`guard_settle_after_recovery`)只挂在 `judge` 产出判胜/判败的两处入口,未覆盖 `kill_as_rebased` 这条独立终态落账路径(它有自己的 `RebaseAsOfBehindGate`,但那只比对旧档自己的门卫钟,不比对调用方声明的恢复点)。

**实现**:在 `kill_as_rebased` 开头补一行 `self.guard_settle_after_recovery(key, as_of)?`。该函数同时被 `register_new→guard_single_active`(本票靶心)与 `reconcile_window`(票面未点名)共用,护栏落在共享函数体内,两路同时闭合——若只想护栏一路就必须在两个调用点各自复制判据,属 `no-patch-mentality` 明令禁止的补丁式重复,故判定为必然覆盖而非不当扩大范围。

**2. Commit SHA**:`2c8c3492a510dc0154bf9f89e49bad7f6901d596`

**3. 测试名清单**(新增 6 条,均在 `tests/rebase.rs`):
- `observe_collision_rebase_with_as_of_behind_recovery_point_is_rejected_fail_loud`
- `observe_collision_rebase_violating_both_gates_reports_recovery_point_first`
- `observe_collision_rebase_with_as_of_exactly_at_recovery_point_is_admitted`
- `reconcile_window_rejects_as_of_behind_recovery_point_fail_loud`
- `reconcile_window_admits_as_of_exactly_at_recovery_point`
- `plain_fold_without_recovery_point_still_lets_stale_as_of_kill_as_rebased`

**4. 指纹对照**:`cargo test --lib` 全量 **2205 passed / 0 failed / 138 ignored**(基线 2199 + 新增 6 条精确对应);`retrace_ledger` 模块内 130/130;`golden_log` 两条锚测试(`golden_event_log_matches_the_repo_anchor`、`golden_log_digest_is_pinned_and_tamper_evident`)均通过,事件日志/JSONL 指纹未漂移。`book.rs` 800 行,压在 `coding-style.md` 800 行上限内(未超)。0 新增编译警告。

**5. 偏离**:
- 两轴评审(异质 sonnet 子代理)发现 `book.rs` 曾一度达 802 行超界,已收紧注释压回 800;其余为判断性意见(doc 三处重复描述同一护栏语义的潜在 shotgun surgery、测试间共享建仓样板),未处理——与既有文件风格一致,非本次改动独有。
- Spec 轴指出"两门(恢复点护栏/门卫钟)皆违反时报哪个错误变体"此前未言明也无测试,已补一行 doc 说明 + 一条边界测试钉死(恢复点护栏先查)。
- 并行车(#630/#631/#633)在共享 worktree 中多次导致全库瞬时编译失败(`bsp.rs`/`incremental.rs`/`level_view_*` 中间态),均确认与 retrace_ledger 面无关,重试后拿到干净通过,未做任何越界修复。
