# task #76（D7）：#56 例2 firstRetrace 重开复核

日期：2026-07-14  
分支：`task-76-d7-case2`  
基点与报告 HEAD：`bc33aedd4f0475c6d0cbb6643f03b35c35c0c06d`  
裁定：**(a) 冲突消解——严格域内不成立，例2挂起可关闭。**

## 1. 结论

- **【已验证】** 在 D1 `project_extended_windows()` 显式 exact-three 投影与 D2 `C2VersionTuple::auto_pairing()` 组装出的严格 C2 域内，#54 首对 `L3#273→L3#274` 在原判定时点 `2906047` 没有两个已完成对象；到后续时点两者又归入同一个 CompletedMove，因而不满足“不同、相邻、已完成”。
- **【已验证】** #54 后续成功对 `L3#276→L3#277` 在 `2943378` 也未进入严格 pair 域：`L3#276` 可归入已完成 C2 move，但 `L3#277` 在该 raw-prefix 的 D1 投影中尚不存在。
- **【已验证】** 因两组 WindowUnit pair 都在对象映射层 fail closed，#54 的“合法定向重入后同 `CpId` 晚成功”没有在严格域内重现；本例没有生成可供 firstRetrace 消费的严格 pair，也没有生成 `RETEST_REENTERS / SUPERSEDE / RESTART / SUCCESS` 生命周期事件。
- **【推断】** 例2原挂起的明确先决条件已经完成，结果排除了其作为严格 C2 firstRetrace 反例的资格；可按选项 (a) 关闭例2挂起。这里不否定 #54 WindowUnit 几何记录，只裁定它不能跨对象宇宙直接充当 C2 CompletedMove 证据。

## 2. 输入、版本与复演边界

- **【已验证】** 数据输入为只读 `/Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json`，SHA-256 为 `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`。
- **【已验证】** 两个 raw-prefix 判定时点分别为 `as_of=2906047` 与 `as_of=2943378`；探针从实际 1m bars 依次运行 `parse_layer → classify_with_tower → D1 project_extended_windows → decompose → D2 assemble_level_view`，未注入历史终局对象。
- **【已验证】** D2 使用 `auto_pairing()`，version tuple 保持三字段 fail-closed 绑定：投影 `central-ggdd-v1`、方向 `move-block-direction-v1`、配对 `move-block-ac-v1`；没有复活存档方向 provider。
- **【已验证】** 区域投影在 `P76_CONTEXT_START=2700000 / 2750000 / 2800000` 三个上下文起点下给出相同的目标 seed、move 完成状态与 A/C pair；下表采用默认 `2800000` 的局部编号。
- **【已验证】** 将整层全部历史窗口一次性交给 D1 会在更早的无关 seed 上以 `InvalidSeed { index: 21 }` fail closed；本次按查询坐标域选取上下文后复演，三种上下文边界结果一致。该全层输入限制不改变目标区域对象映射，但应与本例裁定分开记录。

## 3. #54 对象 ↔ D1 seed ↔ C2 对象映射

### 3.1 首对，`judge_at=2906047`

| #54 WindowUnit 对象 | D1 exact-three seed / 区间锚 | C2 对象（该时点） | 严格 pair 资格 |
|---|---|---|---|
| `L3#273 [2884261..2896661]` | `L3#273 [2884261..2888441]`；原区间锚由 #52 固定为 `[2884261..2896661]` | `g3 Trend/Up Pending(TerminalLegNotDivergent) [2873577..2888441]` 与 `g4 Consolidation/None Pending(AwaitingSubsequentMove) [2884261..2901685]` 均未完成 | **【已验证】** `NoCompletedMove` |
| `L3#274 [2896743..2906047]` | raw-prefix 开放窗口为 `[2896743..2905681]`；D1 只取前三项，seed 为 `[2896743..2901685]` | `g4 Consolidation/None Pending(AwaitingSubsequentMove) [2884261..2901685]` | **【已验证】** `NoCompletedMove` |

- **【已验证】** 该时点输入统计为 `raw_bars=2906048`、`merged=1943715`、`levels=6`、所选 `L3 windows=9`、`L2 lower=1305`、D1 `seeds=9`、D2 `blocks=5`、自动 `pairs=2`。
- **【已验证】** 原 WindowUnit 的 `L3#274` 终点 `2906047` 在严格 raw-prefix 主树中尚未形成；D1 seed 又只消费前三项，因此不能把可变窗口尾锚倒灌为已完成 C2 retest。

### 3.2 后续观察，`judge_at=2943378`

| #54 WindowUnit 对象 | D1 exact-three seed / 区间锚 | C2 对象（该时点） | 严格 pair 资格 |
|---|---|---|---|
| `L3#273 [2884261..2896661]` | `[2884261..2888441]` | `g4 Consolidation/None Completed(SubsequentMove) [2884261..2901685]` | **【已验证】** 与 `#274` 折叠到同一 move |
| `L3#274 [2896743..2906047]` | `[2896743..2901685]` | 同一个 `g4` | **【已验证】** 首对返回 `SameMove`，不是不同相邻对象 |
| `L3#275 [2906049..2913414]` | `[2906049..2909293]` | `g6 Trend/Up Completed(TerminalDivergence) [2906049..2934387]` | **【已验证】** 可唯一归入 completed `g6` |
| `L3#276 [2927357..2934387]` | `[2927357..2934387]` | 同一个 `g6` | **【已验证】** 后续对 leave 已完成，但不与缺失对象组成 pair |
| `L3#277 [2935555..2943378]` | raw-prefix 中不存在 | 无 | **【已验证】** `MissingSource(L3#277)` |

- **【已验证】** 该时点输入统计为 `raw_bars=2943379`、`merged=1970689`、`levels=6`、所选 `L3 windows=11`、`L2 lower=1324`、D1 `seeds=11`、D2 `blocks=7`、自动 `pairs=4`。
- **【已验证】** D2 在目标区域实际给出 `Down move_start=2896743, A=[2906049..2908034], C=[2927357..2936993]` 与 `Up move_start=2906049, A=[2920176..2927282], C=[2936993..2942773]`；它们证明自动 provider 已工作，但不补造 `L3#277`，也不把 `#273/#274` 拆成两个 CompletedMove。
- **【推断】** exact-three 主树的裁判对象应取 D1 seed 与 D2 CompletedMove，而不能用 #54 可变长 WindowUnit 的相同 ordinal/后成终点替代；这与既有“两个对象宇宙不可混算”的裁定边界一致。

## 4. firstRetrace 与对象重启语义重放

- **【已验证】** 新增 `strict_completed_pair()`：每个 source seed 必须唯一属于一个 CompletedMove，且 leave/retest 必须是不同、正向相邻的 CompletedMove；缺 source、无完成对象、多重归属、同一对象、不相邻均显式 fail closed。
- **【已验证】** 新增 `replay_first_retrace()`：同一 `(center, departure CompletedMove)` 只消费第一次严格回试；`RETEST_REENTERS` 消费旧身份，随后同 departure 的静默晚成功返回 `FirstRetraceConsumed`。
- **【已验证】** 若后续是不同 departure 的合格严格 pair，事件序被锁为 `SUPERSEDE(old) → RESTART(new)`，成功归属新 identity；不能继续沿用旧 `CpId`。
- **【已验证】** 例2两组输入均未通过 `strict_completed_pair()`，所以严格重放得到空事件序列；#54 的同 `CpId` 晚成功没有在 event 层重现。

## 5. `lv_case2_` 看守及各自锁定语义

| 看守 | 锁定语义 | 结果 |
|---|---|---|
| `lv_case2_auto_pairing_tuple_keeps_all_three_versions_pinned` | D1/D3/D2 三字段 version tuple 不得放松 | **【已验证】** PASS |
| `lv_case2_projection_uses_exact_three_not_mutable_window_tail_anchors` | D1 只消费前三项，不吸收可变窗口尾锚 | **【已验证】** PASS |
| `lv_case2_first_pair_has_no_completed_pair_at_first_judge_time` | 首判时点没有两个 CompletedMove | **【已验证】** PASS |
| `lv_case2_first_pair_collapses_into_same_completed_c2_move_later` | 后续完成不能把同一 move 误认成相邻 pair | **【已验证】** PASS |
| `lv_case2_late_pair_has_completed_leave_but_missing_retest_seed` | `#276` 完成不补造缺失的 `#277` | **【已验证】** PASS |
| `lv_case2_same_identity_late_success_is_rejected_after_reentry` | 旧 identity 的 firstRetrace 被消费后拒绝晚成功 | **【已验证】** PASS |
| `lv_case2_new_departure_supersedes_then_restarts_with_new_identity` | 新 departure 必须显式 SUPERSEDE/RESTART | **【已验证】** PASS |
| `lv_case2_without_strict_pairs_emits_no_lifecycle_events` | 对象映射失败时不得越层生成事件 | **【已验证】** PASS |
| `lv_case2_fixture_move_blocks_keep_d3_direction_tristate` | D3 方向只来自 MoveBlock.dir，保留三态 | **【已验证】** PASS |

- **【已验证】** `cargo test --release lv_case2_ --no-fail-fast`：`9 passed / 0 failed / 0 ignored`。

## 6. 实现边界

- **【已验证】** 新增 `rust/src/bin/p76_case2_replay.rs`，只读加载数据并打印 D1/D2 区域对象；不接生产订单路径。
- **【已验证】** 新增 `rust/src/theta_v0/classifier/first_retrace_replay.rs`，承载严格 pair 映射、生命周期纯函数及 9 条看守；`classifier/mod.rs` 只增加模块声明。
- **【已验证】** `c2_level_view.enabled` 默认仍为 `false`；`cargo test --release theta_v0_defaults_match_frozen_spec` 为 `1 passed / 0 failed`。
- **【推断】** 由于新增逻辑没有生产调用入口、默认开关仍关闭且全量回归零失败，默认全关订单轨保持既有 bit-exact 行为。

## 7. 验证清单

- **【已验证】** 全量命令 `cargo test --release --no-fail-fast`：跨全部 target 汇总 `1689 passed / 0 failed / 133 ignored`；对照基线 `1680 / 0 / 133`，净增 `+9 / +0 / +0`。其中 lib 为 `1635 passed / 0 failed / 127 ignored`，其余 target 合计补足上述总数。
- **【已验证】** `git diff --check`：零输出。
- **【已验证】** 受保护路径 `docs/formal-chain/` 的 44 项逐文件 SHA-256 索引前后相同，索引哈希均为 `57213e501571d63c41a4e86f8195af8c2ebca655cca7bbd5c3ccb002f6f0c0d9`。
- **【已验证】** 既有 `chanlun/` 54 项逐文件 SHA-256 索引前后相同，索引哈希均为 `e96e7f97d954d40ed6503f0092c793d6ad5d9e10758420c58b31b3bd476fabf4`；本文件是任务允许的唯一新增报告，不进入“既有文件”前后索引。
- **【已验证】** 用户硬约束写出的 `rust/src/theta_v0/classifier/mutex.rs` 在本 worktree 不存在；实际互斥实现 `rust/src/theta_v0/strategy/mutex.rs` 前后 SHA-256 均为 `5df7fbf375eba69b81bb02c00123fe9e0ede19bb69068a3cf54daddbfcb5502c`。
- **【已验证】** `git diff --name-only -- rust/src/theta_v0/strategy/mutex.rs docs/formal-chain` 零输出；排除本允许新增报告后，`git diff --name-only -- chanlun` 零输出。
- **【已验证】** 完成时未提交；HEAD 仍为 `bc33aedd4f0475c6d0cbb6643f03b35c35c0c06d`。

## 8. 最终裁定

- **【已验证】** 首 pair 在首判时点未完成、在后续时点又折叠为同一 CompletedMove；后续 pair 缺 retest source。两者均不满足严格 C2 pair 的必要条件。
- **【推断】** 采用选项 **(a) 冲突消解**：严格域内冲突不成立，#56 例2挂起可关闭；无需新增裁决菜单，也没有新的语义先决条件。
