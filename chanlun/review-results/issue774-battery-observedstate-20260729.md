# #774 issue550_event_battery 编译断裂修复

## 断点清单

`cargo check --all-targets` 定位到 **1 处** 断点（票面预期"可能不止一处"，实测唯一）：

- `rust/src/bin/issue550_event_battery.rs:610`
  `error[E0308]: mismatched types … expected \`ObservedState\`, found \`CandidateState\``
  测试辅助函数 `observation()` 构造 `CandidateObservation { state: CandidateState::Provisional, .. }`，
  字段 `CandidateObservation.state` 的实际类型是 `ObservedState`（`cand_event/observe.rs:39`）。

## 映射对照

`#634` 段3把「候选状态」拆成两个类型（`cand_event/key.rs:54-102`）：

| 旧（bin 里误用） | 新（字段实际类型） | 关系 |
|---|---|---|
| `CandidateState::Provisional/Unresolved/Confirmed/Invalidated`（四态，事件簿域） | `ObservedState::Provisional/Unresolved/Confirmed`（三态，观察侧域，无 `Invalidated`） | `ObservedState` 是 `CandidateState` 去掉 `Invalidated` 的子域；`From<ObservedState> for CandidateState` 单射提升（`key.rs:93-102`） |

`Invalidated` 是事件簿自产终态（唯一构造点 `book::invalidate`），不是外部观察能提供的输入——`ObservedState` 类型层面就不携带它，测试构造的 `CandidateObservation` 只需要落在三态子域内即可，语义零改动（原意就是"结构宽候选成立"）。

修复：
1. 测试模块 `use` 块补 `ObservedState`（`cand_event/mod.rs:36` 已导出，无需改 classifier）。
2. `:610` 由 `CandidateState::Provisional` 改为 `ObservedState::Provisional`。

bin 文件里其余 6 处 `CandidateState` 用法（343/356/372-377/427/538 行）操作的是 `CandidateEvent.state`（`key.rs:159/238`，类型确实是 `CandidateState`），不受影响，未碰。

## 漏网原因

`cargo check`（不带 `--all-targets`）只编 lib + 各 bin 的**可执行目标**，不编 `#[cfg(test)] mod tests` 这类 bin 内嵌测试目标（`cargo test --bin <name>` 才会编）。`#634` 合流验收当时大概率只跑了裸 `cargo check`/`cargo test --lib`，bin 内测试目标编译错误未触发，红码带入 main。

## 防再漏建议

合流验收清单里把 `cargo check` 固定改为 `cargo check --all-targets`（一次性覆盖 lib/bin/test/example/bench 全部目标），尤其是涉及 classifier 内部类型拆分/改名这类跨边界改动，必须过一遍全目标编译再放行。

## 三件套读数

- `cargo check --all-targets`：0 error（既存 warning 不变）。
- `cargo test --lib`：2604 passed / 1 failed / 138 ignored——失败为既存基线红 `theta_v0::env_registry::tests::no_stray_env_literals_outside_registry`（`wverify_run/m8.rs`/`report.rs` 裸 env 字面量，与本票无关；`git stash` 验证改动前后同样失败，零新增）。
- `cargo test --bin issue550_event_battery`：1 passed，绿。

## 改动范围

仅 `rust/src/bin/issue550_event_battery.rs`（2 行：1 处 import 补充 + 1 处枚举替换）。classifier 任何文件未碰（#668 在飞隔离要求满足）。
