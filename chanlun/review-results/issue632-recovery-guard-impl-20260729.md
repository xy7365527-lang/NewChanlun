# #632 落锤护栏实施车收口报告（claude sonnet）

> 日期：2026-07-29；性质：实施车 stdout 收口报告原文
> 原始日志：/tmp/issue632-dispatch-20260729.log

## 收口输出

**1. commit SHA + stat**：`8b791e3ead`，6 文件，+352/−7（`adapter.rs` +14、`audit.rs` +5/−2、`book.rs` +50、`log.rs` +37/−4、`tests/mod.rs` +5/−1、新建 `tests/recovery_guard.rs` +248）。

**2. 恢复点形状选择与理由**：选**显式恢复标记**（`RetraceLedger::fold_recovered(provenance, records, recovery_as_of)`），而非「fold 完成时 max(留档下界)」。理由：留档下界本身就是门卫钟的退回值，任何仅从 journal 计算得出的量必然 ≤ 留档下界——MEDIUM-1 命中的恰是「留档下界」与「重启前最后已知知情时」之间那段日志本就不知道的区间（影子探针 6：journal 只留到 as_of=500，陈旧输入是 600）。用 journal 算出的量做恢复点等于用同一把尺子量自己，探针场景仍会放行。恢复点因此只能来自 journal 之外：调用方独立维护、不随本账本持久化边界漂移的进度标记（例如重放驱动自身的续跑断点）。`recovery_floor: Option<usize>` 默认 `None`（`new()` 活账 / 未升级的普通 `fold()`），护栏零约束，纯增设不改既有路径。

**3. 指纹对照**：开工基线 `cargo test --lib` = 2185/0/137；收口 = **2195/0/137**（+10，即新增测试数）。`--lib retrace_ledger` 子集：102 → **112**（+10）。`cargo test --no-fail-fast`：41 个 target 全绿，零 FAILED（含 doc-tests）。`cargo build --lib` / `cargo doc --lib --no-deps`：零新增警告、零新增 broken-link。

**4. 测试名清单**（`tests/recovery_guard.rs`，10 条）：
- `plain_fold_without_recovery_point_still_lets_stale_knowledge_settle`（普通 fold 历史行为不变）
- `recovered_settle_with_as_of_behind_recovery_point_is_rejected_fail_loud`（探针 6 复现：拒收+警报）
- `identity_can_still_settle_normally_once_as_of_catches_up_to_recovery_point`
- `settle_exactly_at_recovery_point_is_admitted`（边界：恰等放行）
- `settle_one_below_recovery_point_is_still_rejected`（边界镜像）
- `already_terminal_identity_is_unaffected_by_recovery_point_late_absorption_still_applies`（终态不受影响）
- `live_ledger_has_no_recovery_point_and_settles_unconstrained` / `plain_fold_leaves_recovery_floor_unset`（活账不设防）
- `brand_new_identity_settling_below_recovery_point_on_first_observation_is_rejected` / `_succeeds`（`register_new` 通道覆盖）

**5. doc 订正落位**：`log.rs` 模块头「门卫钟的可恢复性」节（原 28-31 行）改为「护栏后口径」段——原「完整后果声明」保持不动作为历史说明，新增段落声明护栏落地后陈旧落锤已被拦下，护栏是叠加层而非对原声明的否定。`adapter.rs` 新增 `RetraceRejection::SettleBehindRecoveryPoint` 变体注释、`audit.rs` 四类警报表述追加本护栏。

**6. 偏离/存疑**：
- `restore()`（快照缓存快路径）未同步接 `fold_recovered`——票面范围只提到「fold 完成时」，`restore()` 的 `FullReplay` 分支仍走无恢复点的 `fold`。若生产恢复路径经 `restore()` 而非直接 `fold_recovered`，护栏不会被触达，需 S4/消费方接入时一并登记（类似 #621 LOW-3「零生产调用点」的性质，非本票违规，登记在案）。
- `kill_as_rebased`/`reconcile_window`（引擎改口处死路径）未叠加本护栏——该路径已有独立的 `RebaseAsOfBehindGate` 保护（比对该身份自己的门卫钟，不是恢复点），与 MEDIUM-1 命中的 `judge()` 落锤路径是不同关注点，本票未触碰，保持 #622 现状。
