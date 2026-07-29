# #623 S3 收口报告——门户补齐：备战档只读 + 短差档三锁 + 失败处置通知

> 日期：2026-07-29；性质：实施车收口报告（claude sonnet 实施车）
> 工位：`/tmp/kimi-nest-mainline`；分支 `kimi-nest-mainline-20260717`；全程单线程（无子代理、无后台任务）
> 并行声明：本票与 #622（S2：引擎改口/死人挂号/两拍证据守卫/audit 流）在**同一物理工作目录**并行
> 施工（非独立 worktree）。过程中触发一次真实的 git 暂存竞态，已在「偏离」节完整记录 + 已订正，
> 订正结果已在隔离 worktree 独立复测通过。

## 1. commit SHA 列表

| SHA | 性质 | 说明 |
|---|---|---|
| `5ea50828300bc3ae17aa057f48b6401705b61840` | 主体实装 | portal.rs 新建 + tests/standby.rs + tests/short_retrace.rs 新建 + mod.rs/tests/mod.rs/tests/portal.rs 追加。**已知问题**：提交时因 `git commit -- <pathspec>` 的隐式重新 add 语义，误将 #622 在工作树中的未提交在制品内容（`pub mod audit` 注册、`NotConstitutedReason::CenterRebased` 变体形状改动、mod.rs 大段文档重写、tests/mod.rs 四个 `mod` 声明）一并卷入，导致本 commit **单独检出不可编译**（E0583：`audit`/`rebase_audit`/`dead_center`/`two_pass_evidence` 模块声明存在但文件缺失） |
| `69cdbb5d53888a5f9beae0c768b493eccedb77bc` | 订正 | 订正 `tests/mod.rs`：移除误吞的 4 个 `mod` 声明（`rebase`/`dead_center`/`rebase_audit`/`two_pass_evidence`），恢复为本票 6 个声明（`clocks`/`log_replay`/`portal`/`short_retrace`/`standby`/`state_machine`） |
| `73c7b34bea676b38fd5be5a1acf3e37f716f372e` | 订正 | 订正 `mod.rs`：移除误吞的 `pub mod audit`/`pub use audit::{...}` 及关联文档重写，恢复为「S1 基线（`a9756715b1`）+ 本票 clean 两处追加（`pub mod portal` + `pub use portal::{...}`）」 |

**三提交合计对本票的净效果** = `git diff a9756715b1 73c7b34bea -- rust/src/theta_v0/classifier/retrace_ledger/` 应恰等于「新增 portal.rs + tests/standby.rs + tests/short_retrace.rs + mod.rs 两行追加 + tests/mod.rs 两行追加 + tests/portal.rs 三行追加」，零涉及 #622 内容（已用隔离 worktree 独立验证，见 §2）。

## 2. 指纹对照

**开工基线**（HEAD `a9756715b1`，S1/#621 交付之上）：`cargo test --lib` = **2130 passed / 1 failed / 137 ignored**；唯一红 `extract_signals_bit_exact_digest_guard`（#491 在案恒红）。

**收口独立验证**（`git worktree add --detach /tmp/verify-623b 73c7b34bea`，与主工作目录完全隔离，独立 `CARGO_TARGET_DIR`，不受 #622 并行车工作树波动影响）：

| 项 | 结果 |
|---|---|
| `cargo build --lib --tests` | 零 error |
| `cargo test --lib` | **2144 passed / 0 failed / 137 ignored** |
| `cargo test --lib retrace_ledger` | **61 passed**（= 48 S1 既有 + **13 本票新增**，单列：standby 5 + short_retrace 8） |
| `cargo test --doc retrace_ledger` | **1 passed**（`StandbyWatch` 的 `compile_fail` 隔离证明，见 AC①） |
| retrace_ledger 新增编译警告 | **0**（`cargo build --lib 2>&1 \| grep -ic "retrace_ledger\|portal.rs"` = 0） |

**全局 +14 而非 +13 的说明**：`2130 → 2144` 净增 14，但本票只新增 13 个 `retrace_ledger` 测试；差额 1 来自**同期并行落地的 #491**（`f6cd18aebc` #610「GOLDEN 诚实重锚」直接改动 `signal.rs`，与本票无关，该 commit 在本票两个订正 commit 之前已并入分支历史）——它使 baseline 唯一红转绿，但未新增测试用例，故 `1 failed → 0 failed` 与 `2130 → 2144`（+14，其中 +13 本票 +1 来自其余并行测试净增）两条口径独立成立，互不矛盾。retrace_ledger 作用域内的 +13 是本票唯一可归因的净增（§475 先例：并行线污染须核并注明，不计入本票）。

**未触碰面**（回应票面禁区）：`git diff a9756715b1 73c7b34bea -- rust/.../retrace_ledger/adapter.rs rust/.../book.rs rust/.../log.rs` = 零行；`rust/.../ledger_kernel/`、`nest_lifecycle.rs` 全程未列入任何一次 `git add`。

## 3. AC 逐条 → 测试名映射

### 备战档（AC①：未决可见可枚举；类型面隔离不可消费成买入信号）

| AC 子项 | 落位 | 测试 |
|---|---|---|
| 未决候选可枚举（位置 + 快照 + 出生钟） | `RetraceLedger::standby`/`standby_watch` + `StandbyWatch` | `standby_portal_enumerates_pending_with_position_and_frame_snapshot` |
| 只放 Provisional，Confirmed/Invalidated 不进 | `watch_of` 过滤 | `confirmed_and_failed_candidates_never_enter_the_standby_portal` |
| 按身份键序确定性 | `entries()`（BTreeMap 序） | `standby_portal_is_deterministic_across_centers` |
| 单点查询与枚举一致 | `standby_watch(key)` | `standby_portal_enumerates_pending_with_position_and_frame_snapshot`（内含断言） |
| 字段表结构性无 `retest_end`（回抽未走完诚实缺席） | `StandbyWatch` 五字段穷尽解构 | `standby_watch_has_no_retest_position_field_by_construction` |
| **类型面隔离——编译期证明** | `TradableSignal` 标记 trait（仅 `ThirdPointPack` 实现） | 模块文档内 `compile_fail` doctest（`portal.rs` `StandbyWatch` 文档块，`cargo test --doc` 覆盖，见 §2） |
| **类型面隔离——正面对照** | 成立档满足门闸、备战档不满足 | `established_pack_satisfies_tradable_signal_gate_standby_watch_cannot` |

### 短差档（AC②：亚型签固定；键唯一冲突拒绝；重复消费幂等；账平断言进验收行）

| AC 子项 | 落位 | 测试 |
|---|---|---|
| `NotConstituted{RetestReentered}` → 短差记录，亚型签固定 | `ShortRetraceRecord`/`PanDivSubtype`（单变体） | `short_retrace_portal_captures_retest_reentered_with_fixed_subtype` |
| 判败：中枢未破坏，快照仍原框（未转正） | 同上（`record.frame == center`） | 同上 |
| Confirmed/Provisional 不进短差档 | `short_retrace_record_of` 过滤 | `success_and_pending_never_enter_short_retrace_portal` |
| **锁一：键唯一冲突拒绝** | `ShortRetracePortal::admit` | `admit_rejects_duplicate_identity` |
| **锁二：一一对应去重** | `ShortRetracePortal::sync` | `sync_is_one_to_one_with_ledger_failures` |
| **锁三：账平断言（进验收行）** | `ShortRetracePortal::balances_with` | `portal_balances_with_ledger_failure_count` |
| 幂等消费（不重复处置） | `ShortRetracePortal::consume` | `consume_is_idempotent_and_does_not_double_process` |

### 失败处置通知（AC③：载荷齐；名义标注通知非信号）

| AC 子项 | 落位 | 测试 |
|---|---|---|
| 载荷 = 身份 + 位置 + 知情时 | `FailureDisposalNotice`（`identity`/`position`/`known_as_of`） | `failure_disposal_notice_carries_identity_position_and_knowledge_time` |
| 逐条判败各发一份，直接派生口径与登记口径一致 | `RetraceLedger::failure_disposal_notices` + `ShortRetracePortal::disposal_notices` | `disposal_notices_are_emitted_for_every_registered_short_retrace_record` |
| 名义「通知非信号」（对任一判败发出，不判断是否已入场） | 模块文档 + `FailureDisposalNotice` 文档 | 结构性声明（账本不读取任何外部持仓状态字段，总禁区未破——`portal.rs` 全文零引用交易层类型） |

### Standards / 两轴 硬杠

| 项 | 结果 |
|---|---|
| 文件 ≤800 行 | `portal.rs` 267 / `tests/standby.rs` 98 / `tests/short_retrace.rs` 160，全 ≤800 |
| 函数 ≤50 行 | 最长 22 行（测试），生产侧最长 18 行（`short_retrace_record_of`） |
| retrace_ledger 零新增警告 | 见 §2 |
| 既有测试全绿（S1 48 条零改动） | `cargo test --lib retrace_ledger` 61/61（含既有 48 条） |
| 禁区：不进口外部状态 | `portal.rs` 全部类型只引用 `retrace_ledger` 内部域词汇（`RetraceKey`/`RetraceSide`/`CenterFrame`/`RetracePoint`），零外部依赖 |
| 禁区：不动 S1/S2 行为面 | `book.rs`/`adapter.rs`/`log.rs` 零改动（§2 diff 证实） |

## 4. 偏离 / 存疑

1. **【已订正，请编排侧复核】commit 归属竞态——`git commit -- <pathspec>` 隐式重新 add 陷阱**：首个提交
   （`5ea5082830`）为避免与 #622 并行车争抢 `mod.rs`/`tests/mod.rs`，我原以为「先用 `git hash-object`/
   `git update-index --cacheinfo` 把干净 blob 定向暂存进索引，再 `git commit -m ... -- <显式路径列表>`」
   足以隔离并行车的工作树在制品改动。**判断有误**：`git commit` 携带 pathspec 时，会对该 pathspec
   隐式执行一次「用当前工作树内容重新 add」，覆盖我此前手工暂存的 blob——于是提交时刻若并行车
   工作树恰好处于某个中间态，那个中间态就被一并卷入。本票命中两次（`mod.rs` 与 `tests/mod.rs`
   各一次），已用两个订正 commit（`69cdbb5d53`/`73c7b34bea`）分别修复，订正后用**独立 git worktree**
   （`/tmp/verify-623b`，与主工作目录完全隔离、独立 `CARGO_TARGET_DIR`）复测确认 `73c7b34bea` 单独
   检出可编译、`cargo test --lib` 全绿（§2）。**请编排侧确认**：(a) 三 commit 的现状是否可接受
   （即中间态 `5ea5082830` 单独检出不可编译，但 `73c7b34bea` 作为最终态可编译且干净），或要求我
   进一步处理（如 squash，但项目惯例明确不要求、且 squash 需要改写历史，本票禁 git mutation 之下
   我未做）；(b) 我改用「先 `git commit`（不带 pathspec，只提交索引里已确认的内容）」的方式已避免
   同一陷阱二次命中，此后再无类似事故。
2. **短差档三锁的实现层级**：#574 契约与 #606 S1 先例把「三锁」表述为账本层纪律；本票把三锁实现在
   一个**新增的下游接收器**（`ShortRetracePortal`）而非直接嵌入 `RetraceLedger` 本体——因为
   `RetraceLedger`（`book.rs`）本身按 `RetraceKey` 去重已是内核保证的既有事实（一个身份只有一条
   终态记录），账本层面并不存在「重复登记」的可能性；三锁真正有意义的地方是**下游消费者**在多次
   读取/多次同步时是否会重复处理。故键唯一/一一对应去重/账平断言三条纪律实现在消费端
   `ShortRetracePortal` 而不是 `RetraceLedger::short_retrace_records()`（后者是纯函数式只读派生，
   本身天然幂等无状态）。**若编排侧认为三锁必须钉在账本本体而非消费端接收器，需要另行设计**（但
   那将与「账本只读派生 + 消费方自理状态」的既有架构（成立档同一模式）不一致，请裁）。
3. **类型面隔离证明形式**：采用 rustdoc 内置 `compile_fail` doctest（`cargo test --doc` 覆盖，零新增
   依赖）而非 `trybuild`/`static_assertions` 等专用 crate（仓内无先例、无该依赖）。这是「等价证明
   测试」而非独立第三方工具验证；`compile_fail` 的局限——只保证「这段代码编译失败」，不保证失败
   原因恰好是「缺少 trait 实现」（例如若未来代码重构导致该行因为其他原因编译失败，测试仍会通过，
   但已不是原本要证的事）。如需更精确的失败原因断言，需要 `trybuild`（校验 `.stderr` 输出），
   本票判断该增量投入超出 sonnet 常规档范围，未引入。
4. **`ShortRetracePortal`/`StandbyWatch` 均未接生产消费方**：与 S1 同一登记口径（LOW-3 先例）——
   spec 用户故事 22「无消费方不接生产」把消费方登记归 #575，本票 AC 未含此项，故不判本票违规，
   登记在案。
