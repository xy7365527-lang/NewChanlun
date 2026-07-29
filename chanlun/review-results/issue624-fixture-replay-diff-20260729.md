# #624 S4 验收环：对拍报告 + golden 锚 + 消费方登记 + 旧模块处置

> 日期：2026-07-29
> 票：#624（S4，父票 #575；spec #620；语义契约 #574）
> 工位：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
> 基线 commit：`1cab40f9d8`；本票 commit 链：`3180f8f446` → `148c814312` → `66cce932dc` →
> `8b8905def2` → `1eb76f104b`
> 编排者 2026-07-29 终审：`first_retrace_replay.rs` 处置 = **A（5 删）**

## 统计口径行

| 项 | 值 |
|---|---|
| 旧模块 fixtures 总数（`#[test]` 逐条枚举） | **9** |
| 其中 firstRetrace 语义面 | 6（F3–F8） |
| 其中非本语义面（C2 版本元组 / 投影 / MoveBlock） | 3（F1/F2/F9） |
| 对拍 PASS | **4**（F4/F6/F7/F8，落 5 个新测试——F7 正/误两面各一） |
| 对拍 FAIL | **0** |
| 不覆盖（裁定 A 后新账本无对应面） | **2**（F3/F5，seed → CompletedMove 映射面） |
| 迁往本主模块 | 1（F1 → `level_view/tests/projection_pairing.rs`） |
| 已被本主模块既有测试覆盖 | 1（F2） |
| 零覆盖损失（同义反复夹具） | 1（F9） |
| lib 测试数 | 2195 → **2197** passed（+11 新增 / −9 旧 fixture）；0 failed；137 → 138 ignored |
| golden 锚 | 11 条修订 / 12 行 JSONL / 3608 字节 / digest `0x808b849b9ba1d112` |
| 消费方接线点 | **0 / 3**（三档全部显式登记为未接线） |
| 删除 | `first_retrace_replay.rs` 572 行 + `classifier/mod.rs` 注册 3 行 |

## 一 · fixtures 全量枚举（对拍基线表，先枚举后对拍）

旧模块 `rust/src/theta_v0/classifier/first_retrace_replay.rs`（HEAD 前 572 行，`e942e1d3c1` 入库，
#76 D7 例 2）测试模块共 9 条 `#[test]`，逐条编号如下。「输入」「旧输出」两列取自删除前源码，
非事后追述。

| # | fixture 名 | 输入 | 旧输出 | 面 |
|---|---|---|---|---|
| F1 | `lv_case2_auto_pairing_tuple_keeps_all_three_versions_pinned` | `C2VersionTuple::auto_pairing()` | 三个 provider 版本 = `CENTRAL_GGDD_V1` / `MOVE_BLOCK_AC_V1` / `EXTENDED_TO_EXACT_THREE_V3`；`validate().is_ok()` | `level_view` |
| F2 | `lv_case2_projection_uses_exact_three_not_mutable_window_tail_anchors` | 两个 4 腿 `LeveledMove` 窗口 | `project_extended_windows` 产 2 seeds，端点 = exact-three（不被扩展尾污染） | `level_view` |
| F3 | `lv_case2_first_pair_has_no_completed_pair_at_first_judge_time` | 2 seeds（273/274）+ 单个 `Pending` move | `Err(StrictPairError::NoCompletedMove(id(273)))` | firstRetrace（严格 pair 映射） |
| F4 | `lv_case2_first_pair_collapses_into_same_completed_c2_move_later` | 4 seeds + 3 moves，273/274 同属 move 0 | `Err(StrictPairError::SameMove(0))` | firstRetrace（严格 pair 映射） |
| F5 | `lv_case2_late_pair_has_completed_leave_but_missing_retest_seed` | 2 seeds（275/276），查 276→277 | `Err(StrictPairError::MissingSource(id(277)))` | firstRetrace（严格 pair 映射） |
| F6 | `lv_case2_same_identity_late_success_is_rejected_after_reentry` | identity(dep=3) + [RetestReenters(3→4), Success(3→4)] | `Err(RetraceLifecycleError::FirstRetraceConsumed(identity))` | firstRetrace（消费一次性） |
| F7 | `lv_case2_new_departure_supersedes_then_restarts_with_new_identity` | identity(dep=3) + [RetestReenters(3→4), Success(5→6)] | `Ok([RetestReenters{old,4}, Supersede{old}, Restart{new}, Success{new,6}])` | firstRetrace（Supersede→Restart 纪律） |
| F8 | `lv_case2_without_strict_pairs_emits_no_lifecycle_events` | identity + `[]` | `Ok(vec![])` | firstRetrace（空输入） |
| F9 | `lv_case2_fixture_move_blocks_keep_d3_direction_tristate` | 两个字面 `MoveBlock` | `dir == None` / `Some(Down)`（构造即断言） | `decompose`（同义反复） |

## 二 · 语义映射表（前置交付：先立映射，再对拍）

| 旧词汇（`first_retrace_replay`） | 新词汇（`retrace_ledger`） | 关系 |
|---|---|---|
| `StrictCompletedPair` | `StrictCompletedPair`（**迁入** `retrace_ledger/mod.rs`） | 同一类型，derive 与字段一个 bit 不改 |
| `RetraceOutcome{RetestReenters, Success}` | `RetraceOutcome{RetestReenters, Success}`（**迁入**） | 同上 |
| `RetraceIdentity{center: CoordinateWindow, departure_move_index}` | `RetraceKey{frame: CenterFrame(四条边), departure_move_index}` | **加强**：窗口二元组 → 四条边快照（ZG/ZD + 起止时间边，#574 裁定一） |
| `strict_completed_pair()` + `unique_completed_move()` | **无对应面**（上游 provider 职责） | 不覆盖，见 §四 C |
| `StrictPairError::{MissingSource, NoCompletedMove, AmbiguousCompletedMove}` | **无对应面** | 不覆盖，见 §四 C |
| `StrictPairError::{SameMove, NotAdjacent}` | `RetraceRejection::NotAdjacent` | 两码合一（判据同为 `retest == leave + 1`） |
| `RetraceLifecycleEvent::RetestReenters` | `RetraceRevisionKind::NotConstituted{RetestReentered}`（终态修订，进日志） | 名分升级：只读事件 → 从未成立族终态 + 四行语义（裁定三） |
| `RetraceLifecycleEvent::Success` | `RetraceRevisionKind::Confirmed` + `ThirdPointPack` + `CenterDeathCertificate` | 名分升级：只读事件 → 终态 + 成立档 + 死亡证明 |
| `RetraceLifecycleEvent::Supersede{old}` | **无事件名**（旧候选在判败那拍已落锤成终态） | 有意改变，见 §四 B-3 |
| `RetraceLifecycleEvent::Restart{new}` | 新档注册 + `RetraceRevisionKind::Restarted{previous}` 谱系载荷 | 有意改变：瞬态事件 → append-only 留档（裁定三「不走桥迁移」） |
| `RetraceLifecycleError::FirstRetraceConsumed` | 终态静默吸收 + `RetraceAlarms::late_absorbed` + audit `LateAbsorbed` | 有意改变：报错 → 静默吸收 + 警报（裁定三） |
| `RetraceLifecycleError::PairDepartureMismatch` | `RetraceRejection::ActiveCandidateNotSettled` + audit `ResidualRejected` | 有意改变：错误码改名 + 归注册期拒收计数（裁定二） |
| `replay_first_retrace()`（瞬态自动机，返回事件 `Vec`） | `RetraceLedger::observe()`（持久账本，返回 `RetraceStep` + 进 journal） | 有意改变：无状态 replay → append-only 唯一真相（裁定六） |
| 无（旧模块无时间概念） | 三钟（出生 / 落锤 / 门卫）+ 每条修订带 `as_of` | 新增（裁定五） |
| 无 | `NotConstitutedReason::CenterRebased`（引擎改口处死） | 新增（裁定一） |
| 无 | 死人挂号拒收 `DeadCenterReentry` | 新增（裁定四） |

## 三 · 逐条对拍结果

对拍落点：`rust/src/theta_v0/classifier/retrace_ledger/tests/replay_parity.rs`（5 个测试，全绿）。

| # | 新账本对应面 | 落测 | 判定 |
|---|---|---|---|
| F1 | 非本语义面 | 迁入 `level_view/tests/projection_pairing.rs::auto_pairing_tuple_keeps_all_three_versions_pinned` | **迁移**（零覆盖损失） |
| F2 | 非本语义面 | 本主模块既有 `projection_uses_only_first_three_and_is_immutable_from_extension_tail` 已覆盖同一断言 | **已覆盖**（零覆盖损失） |
| F3 | 无 | — | **不覆盖**（§四 C） |
| F4 | `RetraceRejection::NotAdjacent` | `f4_leave_and_retest_on_the_same_move_is_rejected_as_not_adjacent` | **PASS** |
| F5 | 无 | — | **不覆盖**（§四 C） |
| F6 | 终态静默吸收 + 警报 | `f6_same_identity_late_success_after_reentry_is_absorbed_not_errored` | **PASS**（有意改变） |
| F7 | 新档 + `Restarted{previous}` 载荷 | `f7_new_departure_after_reentry_opens_restarted_entry_instead_of_supersede_event` | **PASS**（有意改变） |
| F7′ | `ActiveCandidateNotSettled` | `f7_new_departure_before_the_old_one_settles_is_rejected` | **PASS**（有意改变） |
| F8 | 空账本 + 空日志 + 零警报 | `f8_without_any_observation_the_ledger_and_log_stay_empty` | **PASS**（等价） |
| F9 | 构造即断言，无被测生产代码 | — | **零覆盖损失**（同义反复夹具） |

**FAIL 数 = 0。** 无一条 fixture 在新账本上得到与旧模块相矛盾（而非「有意改变」）的行为。

## 四 · 差异全枚举

### A · 新账本已覆盖且行为等价

1. **严格相邻**（`retest == leave + 1`）：旧 `strict_completed_pair` 最后一道判据 = 新
   `adapter::admit_input` 的不紧邻桶（补充十五「任一失败即 None，禁止后扫替代配对」）。
2. **消费一次性**：一个身份只判一次——旧「`consumed` 置位后拒绝」= 新「终态后零修订、留档一个 bit
   不动」。行为结果同（不会有第二次判决），仅**拒绝形式**改变（见 B-1）。
3. **空输入 → 空输出**：F8 逐位等价。

### B · 有意改变（各有裁定依据，非漂移）

1. **晚到成功：报错 → 静默吸收 + 警报**（F6）。依据 #574 裁定三「终态后同身份迟到输入静默吸收 +
   警报计数（不炸管道）」；落点 `book.rs::advance_existing`。可观测面从 `Err` 移到
   `RetraceAlarms::late_absorbed` + audit `LateAbsorbed` 事件——**信息未丢失，改换承载流**。
2. **换 departure 而旧档未判完：`PairDepartureMismatch` → `ActiveCandidateNotSettled`**（F7′）。
   判据同构（未获重启许可即换 departure），依据裁定二「未判完而来新 departure = provider 有病 →
   报错拒收」；错误码改名并入注册期拒收计数。
3. **`Supersede` 事件名不留**（F7）。旧模块因是瞬态自动机、旧身份没有终态存储，才需要一条
   `Supersede` 显式宣告退位；新账本里旧候选在判败那一拍已 `NotConstituted{RetestReentered}` 落锤
   进日志——**退位已由终态本身承载**，再发一条事件即裁定六反对的双真相。新一代的谱系由新档的
   `Restarted{previous}` 载荷记住（裁定三：Restart 是新档，不走桥迁移）。
4. **`SameMove` 与 `NotAdjacent` 两码合一**（F4）。新适配器只有紧邻一条判据，`leave == retest`
   自然落进 `NotAdjacent`。诊断粒度略降（不再区分「同一根」与「隔了几根」），载荷仍带两个索引，
   可现场区分。
5. **身份加强**：`CoordinateWindow`（两元组）→ `CenterFrame`（四条边 ZG/ZD + 起止时间边）。
   裁定一：判案锚是框，价格只对该框 ZG/ZD——旧二元组不足以判「回抽回没回」。
6. **瞬态 → 持久**：`Vec<Event>` 返回值 → append-only 修订日志（唯一真相）+ 三钟 + JSONL 外化 +
   折叠恢复。裁定五 / 六新增面，旧模块零对应。

### C · 不覆盖（裁定 A 的已知代价，显式登记）

**D1 seed → 唯一 CompletedMove 映射**（旧 `strict_completed_pair` / `unique_completed_move`，
80 行）及其四个 fail-closed 错误码 `MissingSource` / `NoCompletedMove` /
`AmbiguousCompletedMove` / `SameMove`（后者部分并入 `NotAdjacent`，见 B-4）——**新账本没有对应面，
随裁定 A 一并删除**。

- **去向判断**：该原语的职责是「把 D1 seed 证明到唯一一根 Completed C2 Move 上」，属**投影 /
  provider 层**（`level_view`），不属账本层——买卖点账本的入口契约（#574 裁定一/二）是
  「观察携带**已配对**的严格相邻 pair」，账本不重做上游的映射证明。这一分工在
  `retrace_ledger/mod.rs` 的 `StrictCompletedPair` 文档里已写死。
- **代价如实**：删除后，仓内**不再有**任何一处执行「seed 唯一归属 + 均 Completed」这条 fail-closed
  证明。旧模块零生产调用点（`rust-orphan-five-tier-inventory-20260727.md` §第 4 件已确证
  `ext_raw_refs = []`），故删除**不改变任何现有生产行为**；但若将来要给账本接真实 provider，
  这条证明须在 provider 侧重新落地——本报告即该缺口的在案登记。
- **测试面**：F3/F5 两条随被测函数一并删除。**无未测代码残留**（被测对象已不存在），非「删测试
  保绿」。

## 五 · golden 锚（②）

- **位置**：`rust/tests/fixtures/issue624_retrace_ledger_events.golden.jsonl`（12 行 = 1 行
  provenance 头 + 11 条修订；3608 字节）。位置对齐仓内 golden 先例
  （`issue533_m8_*.golden.jsonl`，同目录同后缀），读取方式对齐
  `wverify_run.rs::assert_m8_dump_matches_golden`（`CARGO_MANIFEST_DIR` + `tests/fixtures`）。
- **测试**：`retrace_ledger/tests/golden_log.rs`（5 个活测试 + 1 个 `#[ignore]` 重生成入口）。
- **场景覆盖（六面）**：

  | 拍 | 知情时 | 场景 | 进日志？ |
  |---|---:|---|---|
  | ① | 500 | 注册（未决，向上离开 ⟹ 三类买点候选） | ✓ `Registered` + `SnapshotPinned` |
  | ② | 600 | 判败（回抽重回快照框内） | ✓ `NotConstituted{RetestReentered}` |
  | ③ | 700 | Restart 新档 | ✓ `Registered` + `SnapshotPinned` + `Restarted{previous}` |
  | ④ | 800 | 引擎改口处死（`reconcile_window` 显式核对通道） | ✓ `NotConstituted{CenterRebased}` |
  | ⑤ | 900 | 判胜（快照转正 + 死亡证明） | ✓ `Registered` + `SnapshotPinned` + `Restarted` + `Confirmed` |
  | ⑥ | 1000 | 同身份迟到输入静默吸收 | ✗ audit 流 |
  | ⑦ | 1100 | 死人挂号拒收 | ✗ audit 流 |

  ⑥⑦ **不进 golden 是裁定六的可观测证据**，不是覆盖缺口——它们是「未被采纳的输入」，不改状态、
  结构上不进折叠路径；由 `golden_scenario_covers_the_two_audit_only_faces` 从警报计数与 audit 流
  两侧正面钉死。
- **三层锚**：
  1. **字节**：活账日志经 `JsonlRetraceLogStore::append_all` 外化后与仓内 golden 逐字节相等
     （`golden_event_log_matches_the_repo_anchor`）；
  2. **指纹**：`digest_records` 的 FNV-1a 前缀指纹钉成常量 `0x808b849b9ba1d112`；篡改可检性**逐条
     验证**——对 11 条记录各改一次 `as_of` 都断言指纹变（`golden_log_digest_is_pinned_and_tamper_evident`）；
  3. **重放一致**：从磁盘经真实 JSONL 读回边界 `load()` → `fold()`，逐条比对三态 / 留档 / 出生钟 /
     落锤钟 / Restart 谱系 / 判败原因码 + 成立档 + 短差档，并跑全量 `assert_invariants()`
     （`golden_log_folds_back_to_the_anchored_ledger_state`）。
- **变更纪律**：写进模块头（对齐 `tests/issue533_p123_byte_guardrail.rs` 先例）——只允许因已审阅、
  故意的行为变化更新，须同提交说明原因；禁为变绿静默重生成。附变更登记表与重生成命令。

## 六 · 消费方登记（③）

登记表同时落进 `retrace_ledger/mod.rs` 模块头 §消费方登记（代码侧携带声明，不只在报告里）。

| # | 消费面（本账产出） | 消费方 | 接线状态 | 接线票 / 缺口 |
|---:|---|---|---|---|
| 1 | `CenterDeathCertificate`（`death_certificate(anchor)` + `ThirdPointPack::death_certificate`） | 中枢生命周期账 `crate::trading::center_book::CenterBook` | **未接线** | `CenterBook` 现从 BSP 事件锚自行 diff 派生 `CenterEvent`，不读本证明；接通归中枢账的票（裁定一「发中枢账登记 Broken」） |
| 2 | `ThirdPointPack`（`established()` / `established_pack()`） | 交易层（`crate::trading` 线） | **未接线** | 迟到三类点过滤 #587 已裁归交易层自理；本账不进口外部状态（裁定八总禁区） |
| 2′ | `StandbyWatch`（`standby()`）——同一消费方的备战面 | 交易层（盯次级别回切入点，024:36） | **未接线** | 禁区：不许被消费成买入信号；类型面隔离由 `portal::TradableSignal` 编译期把关（`compile_fail` doctest 在案） |
| 3 | `ShortRetraceRecord` / `ShortRetracePortal` / `FailureDisposalNotice`（亚型签 `PanDivSubtype`） | 盘背短差通道（`signal::locate_pan_div_structure` 一线的 #606/#607 票） | **未接线** | 本账只供判败事件源（spec §出界：pan_div_diag 实装不在本线） |

**生产接线点计数：0 / 3**，与仓内实况一致——`grep -rn "retrace_ledger" --include=*.rs src/` 在
`src/theta_v0/classifier/mod.rs:92` 的模块注册行之外**零命中**。这是「无消费方不接生产」
（#575 验收）的正面兑现：账本已建、门户已开、消费方已具名，接线各归各票，不在本票偷接。

## 七 · 旧模块处置（④）

- **终审**：编排者 2026-07-29 裁 **A（5 删）**。
- **删除面**：`rust/src/theta_v0/classifier/first_retrace_replay.rs`（572 行）+
  `classifier/mod.rs` 的模块注册 3 行（2 行 doc + 1 行 `pub mod`）。**未删任何其他文件**。
- **引用点核查**（删除前全仓 `grep`）：`.rs` 侧命中 4 处，全在 `retrace_ledger` 内且全为
  `StrictCompletedPair` / `RetraceOutcome` 两枚域词汇的 `use`——已随删除迁入
  `retrace_ledger/mod.rs`。`closed_loop/{buy,sell}.rs` 的 `first_retrace` 字段与 Lean 侧同名语义
  与本模块无关，**未碰**。
- **删除段 diff 摘要**（commit `8b8905def2`）：

  ```
   .../classifier/first_retrace_replay.rs    | 572 ---------------------
   rust/src/theta_v0/classifier/mod.rs       |   3 -
   .../retrace_ledger/adapter.rs             |   5 +-
   .../retrace_ledger/mod.rs                 |  40 +-
   .../retrace_ledger/tests/mod.rs           |   1 -
   5 files changed, 38 insertions(+), 583 deletions(-)
  ```
- **删除后全绿证据**：`cargo test --lib` → `2196 passed; 0 failed; 138 ignored`（该 commit 时点；
  其后 `1eb76f104b` 拆分超长测试函数，收口值 2197）。零引用残留，`cargo check --all-targets` 通过。

## 八 · 验证链

| 项 | 结果 |
|---|---|
| 开工实测基线 | `cargo test --lib` → **2195 passed / 0 failed / 137 ignored** |
| 收口 | `cargo test --lib` → **2197 passed / 0 failed / 138 ignored**（+11 新增 / −9 旧 fixture；ignored +1 = golden 重生成入口） |
| 全量 `--no-fail-fast` | 全部 target `test result: ok`，零红 |
| `cargo test --doc retrace_ledger` | `compile_fail` doctest（`StandbyWatch` 不实现 `TradableSignal`）仍生效，1 passed |
| `cargo check --all-targets` | Finished，零错误 |
| 零新增警告 | lib-test 警告数 53（与基线同）；`cargo doc` 输出中引用本票所改文件的警告 **0** 条（`retrace_ledger` 下仅 `book.rs:341` / `log.rs:65` 两条**先存**的私有项链接警告，均非本票所改行） |
| Standards 硬杠 | 新增文件最长函数 34 / 33 行（≤50）；最长文件 `golden_log.rs` 269 行、`retrace_ledger/mod.rs` 521 行（≤800）；错误 fail-loud；常量具名（`GOLDEN_DIGEST` / `GOLDEN_RECORD_COUNT`） |

## 结果包六要素

1. **结论**：#624 四件事全部落地——① 9 条 fixtures 全量枚举 + 语义映射表前置 + 逐条对拍（4 PASS /
   0 FAIL / 2 不覆盖 / 3 非本语义面）；② 事件日志 golden 三层锚（字节 / 指纹 / 重放），六场景覆盖；
   ③ 三消费方登记表落码落报告，0/3 显式标未接线；④ 旧模块按裁定 A 删除，两枚域词汇迁入账本。
2. **定义依据**：#574 契约裁定一（身份 = 四条边快照 + departure）、裁定二（单活跃候选 fail-loud）、
   裁定三（三态 + 判败四行 + 终态迟到静默吸收 + Restart 新档记前任）、裁定四（死人挂号拒收）、
   裁定五（三钟）、裁定六（append-only 日志唯一真相 + 警报另记 audit 流）、裁定七（缺席 ≠ 消失）、
   裁定八（三档门户）；spec `spec-kernel-and-retrace-ledger-20260728.md` §T3 章「first_retrace_replay
   处置随票落，其 fixtures 转作对拍基线」；#575 验收「无消费方不接生产」。
3. **边界条件**（结论翻转条件）：
   - 若判定「seed → 唯一 CompletedMove 映射」属**账本**职责而非 provider 职责，则 §四 C 的「不覆盖」
     从「已知代价」升级为**缺口**，裁定 A 须回退为替换（把该原语迁入账本入口）；
   - 若判定迟到晚成功必须 fail-loud（否定裁定三静默吸收），则 F6 的「有意改变」翻转为**行为回归**；
   - 若 golden 剧本被认为未覆盖某必需场景（如同窗口重确认零动作、门卫钟倒退），则 §五 的六场景
     覆盖声明须扩容——现锚只声明这六面，不声明「全场景」。
4. **下游推论**：
   - 三消费方接线票（中枢账 / 交易层 / 盘背通道）各自接通时，可直接以本票 golden 作回归锚——
     接线不得改变这 11 条修订的字节；
   - provider 侧接真实 `level_view` 时须补回 §四 C 登记的映射证明，否则账本入口只能信任上游；
   - `retrace_ledger::{StrictCompletedPair, RetraceOutcome}` 现为该语义的**唯一**仓内定义点，
     后续任何 firstRetrace 相关实装以此为准。
5. **谱系引用**：#465 调研 §Q2 三桶（`issue465-nest-lifecycle-reusability-20260728.md`：(b) 桶列出
   「买卖点特有、不能随账本骨架丢掉」的 8 项，本票逐项落位——其中 seed 映射与四类 fail-closed 码
   按裁定 A 归 provider 侧，登记在 §四 C）；#574 教义研究（`issue574-reentry-doctrine-research-20260728.md`
   §「同框禁后扫复活」「新一代必须新身份」——`FirstRetraceConsumed` / `Supersede→Restart` 的原文
   背书）；孤儿盘点 `rust-orphan-five-tier-inventory-20260727.md` §第 4 件（零外部引用的确证，
   裁定 A 的事实前提）。**已发生概念分离**：`Supersede` 事件名的消失（B-3）是「瞬态自动机 →
   持久账本」范式切换的必然推论，不是语义丢失。
6. **影响声明**：改动 9 个文件（+524 / −584 行）——新增 `retrace_ledger/tests/{replay_parity,
   golden_log}.rs` 与 `tests/fixtures/issue624_retrace_ledger_events.golden.jsonl`；改
   `retrace_ledger/{mod,adapter}.rs`（域词汇迁入 + 消费方登记 doc）、`retrace_ledger/tests/mod.rs`、
   `level_view/tests/projection_pairing.rs`（F1 迁入）、`classifier/mod.rs`（删注册行）；删
   `classifier/first_retrace_replay.rs`。**零生产行为变更**：被删模块零生产调用点，账本 0/3 未接线，
   本票不新增任何生产路径。

---

## 修订补记（2026-07-29，影子评审 LOW 触发，编排侧落）

**§四 C 缺口登记补一条双实现风险位**：`rust/src/bin/p83_yield_remeasure.rs:383-393` 存在近邻统计实现（按 seed 在 moves 的 `center_indices` 中是否存在计数 `unassigned_projected`）——它不是 seed→CompletedMove 唯一性/fail-closed 证明（无 MissingSource/NoCompletedMove/AmbiguousCompletedMove 语义、非 fail-closed），不反证本报告 §四 C「唯一性证明仓内已消失」的断言；但 #640 落 provider 侧证明时须核对该统计实现，防双实现分叉。（影子评审 `shadow-624-t3-acceptance-review-20260729.md` LOW-3 指出，#640 票面已注明。）

## 修订补记二（2026-07-29，对拍补测——F7/F7′ 同 center 输入复刻，影子评审 MEDIUM-1）

**偏离登记**：§一 基线表登记的 F7 旧输入是 `new = RetraceIdentity { center: old.center, departure_move_index: 5 }`——新旧身份**共享同一个 `center`**（旧模块 Restart 分支 `active = RetraceIdentity { center: active.center, .. }` 同样明写 center 不变）。§三落测的 `f7_new_departure_after_reentry_opens_restarted_entry_instead_of_supersede_event`（F7）与
`f7_new_departure_before_the_old_one_settles_is_rejected`（F7′）用的却是 `frame(1_200)` → `frame(1_400)`（同锚异框，`start_index` 相同但 `end_index` 不同），两表之间隐含的「同输入」在这两条上并不成立——本报告发出时未声明这一偏离，即影子评审 `shadow-624-t3-acceptance-review-20260729.md` MEDIUM-1。

**补测**：`rust/src/theta_v0/classifier/retrace_ledger/tests/replay_parity.rs` 追加两个测试，逐条复刻旧 fixture 的真实输入面（同一个 `frame(1_200)` 贯穿两次 `observe`，departure 3→5，不换 frame）：

- `f7_same_center_new_departure_after_reentry_opens_restarted_entry_instead_of_supersede_event`——同中枢版 F7：断言新档 `restarted_from() == Some(key_of(center, 3))`、词汇序 `Registered → SnapshotPinned → Restarted{previous} → Confirmed`、前任档 `state == Invalidated` 三项与异框版逐位相同；
- `f7_prime_same_center_new_departure_before_the_old_one_settles_is_rejected`——同中枢版 F7′：断言 `RetraceRejection::ActiveCandidateNotSettled{active: key_of(center,3), incoming_departure_move_index:5}`、`book.len()==1`、`alarms().registration_rejected==1` 与异框版逐位相同。

**结果**：`cargo test --lib retrace_ledger` 124 passed（基线 122 + 本次 2 条新增，0 failed，1 ignored 不变）；`cargo test --lib` 全量 2199 passed（2197 + 2，0 failed，138 ignored 不变）；既有测试零改动。同中枢与异框两组断言逐位一致，**验证 MEDIUM-1 指出的静态推理成立**——`guard_single_active`（`book.rs:264-266`）判据只看前档终态、`link_restart`（`book.rs:369`）按 `anchor = start_index` 路由（`mod.rs:196-198`），两者均不读 `end_index`，故同锚异框与同锚同框在当前实现下行为等价；本补测把这条此前「零测试覆盖」的旧 fixture 真实输入面钉为回归锚，MEDIUM-1 描述的漂移风险（未来若 `guard_single_active` 分支序改动到读 frame 全量而非仅锚）现由这两条测试覆盖。

- **谱系引用**：MEDIUM-1（`shadow-624-t3-acceptance-review-20260729.md` §七）；#574 裁定一（身份=四条边快照）、裁定三（Restart 新档记前任、静默吸收）。
- **影响声明**：仅追加 2 个 `#[test]`，零改动既有测试与生产代码；`replay_parity.rs` 内其余 5 条测试逐字未动。
