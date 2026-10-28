# ADR 0055：候选组逐项结算的生产读法、v3 legacy 空集验收与运行时缓存归类

> 正本：本文件是 #1572 候选组逐项结算（生产读法、共享块归属、组净额分支删除、v3 legacy E 侧空映射、运行时缓存归类与丢弃）的正本。
> 最后裁定：#1786（2026-10-28，#1323 Notes N-1 预授权 T 类判定）
> 受影响代码清单：见本文「受影响代码清单」节（`request_audit.rs` / `audit_owner.rs` / `driver.rs` / 持有者文件只读访问器，实施归 #1572 owner，须在 #1640 合入后的基线上）

**状态**：已采纳，文书待 main 合入。执行已签的 v4 资源合同 `v4:25/:28/:30`，不改任何已签数值，不豁免任何已批合同反例；不改 v4 合同正本，不改其他 ADR。
**裁定依据**：#1786 resolution 评论。原件：草稿 `/Volumes/AgentStorage/decisions/1786-manual/draft.md`（两轮返修 28AH、28AJ），考古 `archaeology.md` / `archaeology.json`、分级 `classify.json`、独审 `review.json`（同目录，verdict PASS，class T）；独审链 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1786-INDEPENDENT-REVIEW-28AF.md` → `-28AI.md` → `-28AK.md`。
**行号基线**：main `5123c9e273`；PR #1756 head `ac52446c14`（下称 **W**，`rust/src/bin/s_session_v3/` 下文件简写为文件名）；PR #1757（#1640）head `43bf13e28f`（下称 **S1**）；v4 指 `/Volumes/AgentStorage/issue1488-ticket-manager-20261018/control/1488-unified-budget-ownership-interface-contract-v4.md`。实施在 #1640 合入后的基线上重定位行号。

## 背景

#1572 续作（`/Volumes/AgentStorage/issue1572-cont-28y/IMPL-REPORT.md`）在测试探针里把暖 driver 的候选组在 n=2、n=3 上逐项闭合：Σphysical = L0（2698597、3310261），M 行为 128。entry 2 的 U = 98031（capture 3552、audited 90851、generator 560、auxiliary 3068），entry 3 的 U = 190132（3552、181772、864、3944）。生产冻结路径的候选组仍是一条聚合行 `complete_candidate.charge.total`（`audit_owner.rs:1512`），走组净额 `covered = min(L0, contract)`（`request_audit.rs:341-343`），同一 entry 2 的生产组 U = 0。

组净额让 audited 超出合同的 90851 B 借用其他项的合同余额，正是 v4:25「不用max补差」与 v4:30「每一份都单列。共享不是减少合同费用的依据」所禁止的；`seal` 的逐项分支（`request_audit.rs:299-331`）与组净额分支（`:341-344`）接在同一个 if-else 上，逐项读数缺失时由净额接管放行，落在 #799 判别式上。#1488 评论 [6275492818](https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6275492818) 已把组级 U 定为过渡终态，逐项拆分交 #1572。

待裁：Q1a 读法（每请求遍历 vs 增量字段）、Q1b U 消耗 A 是否改变已签数值或准入、Q1c 实施基线、Q2 v3 legacy 恒空时如何验收、Q3 两份运行时缓存的归类与丢弃。

## 裁定

### D1 读法：每请求遍历许可求和

候选组的持有者——audited（`state::AuditedState` 的 raw 与 historical）、slot（`VerifiedHistoricalSlot`）、`CommittedRawState`（fee/set/source/pending_set）、`GenerationRuntime`、`aux_index::Runtime`——各加只读许可字节访问器。形态同 `audit_owner/q.rs:1047` `q::Facts::permit_bytes`：只遍历、只读 `NativeReservation::bytes`，不签发、不转移、不分配。生产冻结在同一真实父账、同一静止点上调用这些访问器（v4:25「同一真实父账和同一锁保护下」），候选组全部 11 项改走 `item_measured`。析构探针只做对拍，生产与审计共用一个判据（#804）。**禁止**「L0 − 已知项 = 余量」的残差口径（#1572 评论 6441911507 已否）。

**共享块归属规则**：被多个持有者经 `Rc`/`NativeRc` 共享的物理分配，在候选组里恰计一次，计在合同字段已覆盖它的那一项。在案共享块只有一处：

- `AuditedState.historical` 与 `slot.result` 指向的同一个 `ImmutableHistoricalAudit`（含 `NativeRc` 值分配及其内部各 heap 块所持许可）归 `candidate.audited.historical_and_raw`。该项合同 `historical_and_raw_capacity` 本就含 `slot.result.heap_bytes()`（`audit_owner.rs:3075`）；`detach_slot`（`audit_owner.rs:3391` 起，`:3404-3408`）摘掉 slot 后这块只剩 audited 持有，归属不变；析构探针现行顺序（slot 先于 audited，`audit_owner.rs:1385-1421`）也把它记在 audited 那一步。
- `candidate.slot` 的访问器只计 `binding`、`capture`、`source_pending`，不计 `slot.result`。slot 存在时，访问器先核 `slot.result` 与 `AuditedState.historical`（`HistoricalOwner::Shared`）是同一分配（`ptr_eq`）；不是即返回 Err、冻结拒绝，不另设「各自计」分支。冷、热两条保留路径（`state.rs:360-371`、`:417-428`）都保证二者同一。
- 实施中发现其他共享块，须先在本 ADR 登记归属才可合入。双计使 Σphysical > L0，由 `seal`「有项重复计入，拒绝冻结」（`request_audit.rs:325`）拒绝；漏计由「未列入清单的来源」（`:318`）拒绝。两种失败都走入口序列的丢候选路径，不放行。

**删除聚合行与组净额分支**（#799）：删除 `request_audit.rs:108-112` 的 LISTED 行 `complete_candidate.charge.total` 及其在 `old_pool_partition`（`audit_owner.rs:1511-1515`）中的调用；`seal` 删去组净额分支（`request_audit.rs:341-344`），有 Ledger 的组（`ON_PARENT`、`KEEPER_PARENT`）没有实测项一律拒绝。`seal` 已先要求 L0 ≥ M，不存在需要放行的「组内无项且 L0 为 0」。`unpartitioned-old-pool`（`ELSEWHERE`，无父账，只按合同额）不受本条影响。

增量字段方案（分配层按持有者打标签、增量维护计数）被否：它是同一数量的第二本账，改动落在 `s_content/capture_allocation` 与 #1640 改动最密处，在遍历成为瓶颈前只增加成本。它保留为退场去向。

### D2 U 消耗 A，已签数值与准入不变

逐项 U 消耗 A，组 U ≡ Σ逐项U。合同额、T/E/O/A 定义（v4:7）、S7、16/8 MiB、1024、128 一律不改；`CompleteCharge` 各字段的计算不改。v4:28 同样禁止借本票提高合同额吸收 U。回复层准入不翻转：v4:13「预算不足仅放弃候选保留时，严格审计和原回复仍成功」，v4:44 反例 F 全文保持；放不下时丢的是保留不是请求（W `audit_owner.rs:1565-1588`、`request_audit.rs:650-656`、`driver.rs:449-451`）。第八门仍为 `AwaitingPolicy`（ADR 0042:36/:60）。

**余量论证**：在案唯一断言「候选仍在」的锁 `audit_owner_write_budget_retains_candidate_within_retention_cap_and_parent_limit`（W `driver.rs:3912-3983`）设 `total = legacy + held + 8 MiB`（`:3925`），在 n=2 上断言保留。逐项 U 最大 190132 B，不到 8 MiB 的 1/44。其余已签场景锁断言的是回复与布局，不涉候选保留（独审 28AF 第 1 点逐处核过）。

**升 P 触发**：实施后出现以下任一情况，立即停手、不改断言，带读数把本条转为 P 交用户：

- 「验收」节 L3 所列已签场景锁中任一由 PASS 变 FAIL；
- #1640 验收的 S86@256MiB 86/86、MIN12@256MiB 52/52 Committed 不再成立。

### D3 基线：#1640 合入后的 main，新增边 #1572 blocked by #1640

成边理由只有一条：`OldPoolPartition` 在 main 上挂 `s_capture_native_testonly`，该 feature 没有生产资格（main `rust/Cargo.toml:81-82`）；S1 才改挂 `s_incremental`（S1 `request_audit.rs:73-75`、`Cargo.toml:84-87`，`driver.rs:465-478` 令 `total = legacy + 第八门`）。#1572 要的是「生产冻结逐项」，生产候选组在 #1640 之前不存在；在 testonly 门控上先切产不出这一行为，被否。这与 ADR 0042:145「#1572 与切片 2 只是文件交叠，不加边」不冲突：那一条判的是切片 2，只否定以文件交叠成边。

访问器可提前在 #1572 分支起草，限纯新增 fn，#1640 合入后重定位（`state.rs`、`construction_state.rs`、`generation.rs`、`aux_index.rs`、`prepared.rs` 在 #1757 中都有改动）。

**代价**：PR #1757 现为 CONFLICTING，#1640 被 #1774 阻塞，#1572 与它阻塞的 TB-07（#1366）随之后延，长度取决于 #1774 的签署与 #1757 的冲突解决。

**写集**：#1572 写集允许超出 v4:52-63 与 v4:81-89 的 #1488 六文件，延伸到上述持有者文件，仅限新增只读访问器；v4:48 与 v4:89「`audit_owner::startup`、`state::audit_with_budget` 两个函数体不改」继续有效。D5 的丢弃入口改动限 `audit_owner.rs` 与 `driver.rs`。

### D4 v3 legacy：E 侧空映射下满足

v3 生产路径上 E 侧映射是空映射，#1572 验收的「E 侧逐项映射」与 v4:38 反例 C 对 E 侧的要求都在空集上满足；本裁定不豁免任何已批合同反例。不造非空 fixture，不加 E 角色分区入口。

- **合同侧**（E 合同额 = `cache::retained_bytes()` 各加数）恒为 0：`s_session_v2/cache.rs:292-302` `Memo::default` 在非测试构建下 `limit = 0`；`:884-891` `configure` 是唯一置非零入口，其非测试调用方只有 v2 `serve_with_policy`（`s_session_v2/mod.rs:2586`），v3 调用点全在测试模块；`s_structure_session.rs:5706-5709` 先执行 v3 dispatch；`cache.rs:1523/1733/1887/2044/2170` 在 `limit == 0` 时早退。暖态把 memo 开到 268435456，v3 静止点仍为 0。由锁 L5 证。
- **物理侧**另有至少两处不受 `limit` 门控、在 native 工作域上分配的路径：`cache.rs:1096-1107` `MixedCacheTransaction::start`（v3 生产调用点 `admission_anchor.rs:882`）与 `cache.rs:1132-1158` `begin_capture_typed`/`begin_capture_mixed`。它们对 E 合同贡献 0；落在候选 L0 上的字节由 L1 的 Σphysical = L0 判据兜住，非零即「未列入清单的来源」，冻结拒绝、丢候选（保守方向）。实测两处均为 0。

### D5 运行时缓存：归 Q/effects/working set，随候选同窗丢弃

`prepared.runtime.generator` 与 `prepared.runtime.auxiliary` 归「Q/effects/working set」（v4:30），合同额 0，U = physical，即 W `request_audit.rs:128-129` 与 `audit_owner.rs:2797-2812` 的现状。两者是跨请求驻留在候选父账上的增量生成工作态（`prepared.rs:120-127`、`generation.rs:352`、`aux_index.rs:507-515`），合同布局 `CompleteCharge` 中无对应字段。新增合同项按物理额计入 O 被否（违反 v4:28，且把真实新增分配洗成旧费）；归「Scratch/Native共享块/控制区」不采纳（该条目指共享块与控制区，标签之争，不影响数值）。

**丢弃做法**：候选被丢弃时，`prepared::Runtime` 在同一个释放窗口内一并丢弃。`AuditConnection` 的丢弃入口（`discard_after_error`，`audit_owner.rs:1293-1311`）在「读静止点 live」与 `settle_release`（`audit_owner/native.rs:809-822`）之间、`clear_candidates` 之后执行 `Runtime::discard`（`prepared.rs:259-264`），使运行时归还与候选归还一并从静止点标记中扣除。`AuditConnection` 拿不到 `runtime`，由 `Driver` 把 `&mut self.runtime` 传入（两字段可分借；先例 W `driver.rs:3334`）。两处调用点都走这个入口：

1. 入口重分区：`entry_old_pool_partition` 内的 `self.discard_after_error()`（W `audit_owner.rs:1575`；S1 `:1409`），由 W `driver.rs:453-455`（S1 `:479-481`）调用；
2. 请求出错：W `driver.rs:489`（S1 `:515`）的 `self.conn.discard_after_error()`。

**理由**：候选丢弃后其父账不再是任何组的 home（`complete = None`，分区以 `P::new(None)` 建，`request_audit.rs:134-150`；`attest` 与 `seal` 的静止点核对只核 home，`:191-201`、`:287-292`），残留在该账上的 generator/auxiliary 许可既不被静止点核对，也不在任何分区里。D1 访问器按持有者遍历许可求和、不按账过滤，会把它们计入下一候选 ON_PARENT 组的 Σphysical，而该组 L0 不含它们，`seal` 以「有项重复计入」（`request_audit.rs:323-328`）拒绝，入口序列再丢候选，一次丢候选放大成每个入口都丢。「同一窗口」是实现约束，不是拒绝依据。publish 出错路径已有的 `self.discard()`（`prepared.rs:288/:321/:555`、`audit_owner.rs:683`）与 0065 放弃路径的 `runtime.discard()`（`audit_owner.rs:725/:732`）保留不动，由 L4 的出错变体核实叠加后下次入口仍能封口。

**代价**：丢候选的请求本已走冷路径；运行时一并丢弃，下一请求的增量生成态也冷启，多付一次生成态重建，只在边缘预算或出错时发生。

### D6 读数交接给 #1641

切换后在 S86 末代给出逐项 U 读数（item、contract、physical、U），作为 #1641 签署候选保留帽与第八门的依据。保留帽与第八门的数值属 P，仍由 #1641 交用户签署，本 ADR 不代填。#1641 的签署读数须在逐项口径下取得；若 #1572 未合入即签，签后须按逐项口径复测。

## 退场条件

- **D1 遍历读法**：#1481 方案三使每请求审计达到 O(Δ) 时，遍历改为分配层增量计数，并加「计数 ≡ 遍历」锁。登记到 #1481，不加 blocking 边。
- **D4 空映射**（任一成立即失效，须回 #1786 补 E 侧逐项映射）：
  1. v3 任何非测试路径调用 `cache::configure`，或 `Memo::default` 的非测试 limit 不再为 0；
  2. L5 变红；
  3. L1 因 D4 物理侧所列路径留有非零字节而变红。

  #1481 让 Memo 退出写者准入路径后，E 行整体消失，本条随之退役。

## 受影响代码清单

行号指 W；切换实施在 #1640 基线上重定位。Lean / Python：无。

| 文件:行 | 动作 | owner |
|---|---|---|
| `request_audit.rs:51-55`（文档注释）、`:108-112`（LISTED 聚合行） | 注释改为「候选组生产逐项」；删除聚合行 `complete_candidate.charge.total` | #1572 |
| `request_audit.rs:341-344` `seal` 组净额分支 | 删除；有 Ledger 的组无实测项即拒绝（L10） | #1572 |
| `audit_owner.rs:1502-1563` `old_pool_partition` | 删去 `:1511-1515` 聚合行调用；候选组 11 项改 `item_measured`，取 D1 访问器读数 | #1572 |
| `audit_owner.rs:1293-1311` `discard_after_error`、`:1565-1588` `entry_old_pool_partition` | 同一释放窗口内执行 `Runtime::discard`；`entry_old_pool_partition` 接收 `&mut prepared::Runtime`（D5） | #1572 |
| `driver.rs:453-455`、`:489` | 把 `&mut self.runtime` 传入两处丢弃入口（D5） | #1572 |
| `state.rs:46` `AuditedState` | 新增只读许可字节访问器（raw + historical 共享块）；`:620` `audit_with_budget` 函数体不改 | #1572 |
| `request_audit.rs:554-580` `VerifiedHistoricalSlot` | 新增只读访问器：binding + capture + source_pending，不计 `result`；先核 `ptr_eq`，否则 Err（D1） | #1572 |
| `construction_state.rs:224` `CommittedRawState` | 新增只读访问器（fee/set/source/pending_set） | #1572 |
| `generation.rs:353` `GenerationRuntime`、`aux_index.rs:509` `Runtime`、`prepared.rs:120-127` | 新增只读访问器并经 `Runtime` 暴露 | #1572 |
| `audit_owner.rs:2797-2812`（runtime 两行）、`request_audit.rs:128-129` | 不改（D5 确认现状） | — |
| `audit_owner.rs:3026-3036`、`:3058-3098`、`:3391-3415`（`CompleteCharge` 计算） | 不改（D2、L6） | — |
| `rust/src/bin/s_session_v2/cache.rs` | 不改（D4） | — |
| `driver.rs` 测试区 | 新增 L1/L2 负控/L4/L5/L10 锁；L3 所列函数体零改；探针析构顺序保持 slot 先于 audited | #1572 |
| `.github/1488-uo-locks.txt` | 追加 L1、L2 三条负控、L4、L5、L10 锁名 | #1572 |

## 验收（落到 #1640 合入后的 #1572 实施分支）

命令经 nc-remote：`cargo test --locked --features s_session,s_capture_native_testonly --bin s_structure_session -- <名>`；生产构建另以 `--features s_session,s_incremental` 跑 L1。判据二值。

| # | 验收项 | 判据 |
|---|---|---|
| L1 | 生产候选组逐项 | 新锁 `i1572_production_candidate_group_itemized_n2_n3` PASS：生产 `old_pool_partition` 在暖 n=2/n=3 上 `itemized == true`，11 项各有 `physical`，Σphysical = L0，组 U 分别为 98031/190132（或实施时实测值，且与 L2 探针逐项相等）；不存在名为 `complete_candidate.charge.total` 的项 |
| L2 | 同一读数与共享块恰计一次 | `i1572_warm_candidate_holder_drop_probe` 扩展后 PASS：析构顺序 slot 先于 audited；每项生产读数 == 该项析构下降；`candidate.audited.historical_and_raw` 读数含共享块、`candidate.slot` 读数不含 `slot.result`，二者之和等于两持有者析构下降之和。负控三条均被拒：`i1572_production_reader_omits_holder_rejected`（「未列入清单的来源」）、`i1572_shared_historical_counted_twice_rejected`（「有项重复计入」）、`i1572_shared_historical_counted_nowhere_rejected`（「未列入清单的来源」） |
| L3 | 已签场景不翻转 | 以下锁全部 PASS 且函数体零改动：`audit_owner_complete_optional_budget_releases_without_changing_reply`、`audit_owner_write_budget_retains_candidate_within_retention_cap_and_parent_limit`、`audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate`、`r13_cold_candidate_and_q_share_parent_and_preserve_complete_output`、`r13_optional_sql_rows_share_parent_without_changing_cells`，以及 `.github/1488-uo-locks.txt` 全清单；S86@256MiB 86/86、MIN12@256MiB 52/52 Committed 复跑成立。任一不成立即触发 D2 升 P |
| L4 | 丢弃后运行时不留无组许可 | 新锁 `i1572_discard_leaves_no_unlisted_runtime_cache` PASS，覆盖 (i) 入口重分区（构造 `carried() > keeper_cap`）与 (ii) 请求出错（含 publish 出错变体）。之后旧候选父账 live 为 0（父账已析构）或该账以 `item_measured` 列入分区且 Σ = L0；丢弃后立即读旧候选父账 live 为 0 或 `LedgerMark::current()` 为 None，`prepared::Runtime` 的 generator 与 auxiliary 为空；下一请求入口分区封口成功。负控：保留 generator 不丢的变体，下一入口被拒，报文「有项重复计入」（`request_audit.rs:325`） |
| L5 | v3 legacy 合同侧结构性为空 | 新锁 `i1572_v3_legacy_memo_structurally_empty` PASS：新起 `std::thread`、移除 `S_TEST_AUDIT_CACHE_SOURCE_BYTES`、不调 `configure`，依次跑暖 n=2/n=3、ready、冷根 fresh driver，每个静止点 `!cache::retention_enabled()` 且 `cache::retained_bytes() == 0`；负控：同一 helper 在 `configure(1 << 20)` 后返回 Err |
| L6 | 合同额不变 | 同一 head、同一暖 n=2/n=3 输入下，切换前后探针表 11 项 `contract` 列逐项逐位相等（取值来源即现探针 `contract_from` 字段表达式，W `audit_owner.rs:2728-2830`）；`git diff <base>..HEAD` 不触及 `CompleteCharge` 各字段与 `charge.total` 的计算；M 行合同仍为 `Budget::metadata_bytes()`。不以「11 项合同之和 == 聚合行」为判据（`:2836-2837`、`:3091-3098`） |
| L7 | 读数交接 | 实施报告含 S86 末代逐项表（item、contract、physical、U），路径登记在 #1572 与 #1641 |
| L8 | 分区仍封口 | `i1572_candidate_items_partition_reconciles_and_rejects_tampering`、`i1572_itemized_group_reconciles_and_rejects_unlisted_omitted_or_double_counted`、`i1572_candidate_items_reject_unlisted_runtime_caches` PASS |
| L9 | 正本 | 本文件含 #793 三行头、共享块归属规则与两组退场条件；`/Volumes/AgentStorage/decisions/adr-reservations.json` 含 `"1786": 55` |
| L10 | 净额分支已删 | 新锁 `i1572_ledger_group_without_measured_items_rejected` PASS：对 `ON_PARENT`、`KEEPER_PARENT` 各构造只有 `item`（无 `physical`）的组，`seal` 返回 Err；`request_audit.rs` 中不再出现 `l0.min(home.contract)` |

## 受影响票与边

| 票 | 变化 |
|---|---|
| #1572 | 新增 blocked by #1640；#1786 落地后从 blocked by 中移除；blocks #1366 不变 |
| #1640 | 新增 blocks #1572（同一条边）。提醒：#1640 原样继承入口丢候选序列（S1 `audit_owner.rs:1409`、`driver.rs:481/:515`），丢候选时 `prepared::Runtime` 不随之丢弃，generator/auxiliary 许可落在任何分区之外；该缺口由 #1572 按 D5 修，#1640 不必自修；合入后 `s_incremental` 构建在 #1572 合入前存在这一少计；#1572 合入 D1 访问器后若缺 D5，少计变为每个入口以「有项重复计入」被拒 |
| #1641 | 不加边；签署读数须在逐项口径下取得（D6） |
| #1645 | 不加边；#1572 生产读法取每请求遍历，#1481 方案三达到 O(Δ) 后改分配层增量计数，本票增量生产路径据此对接，不另立计量 |
| #1481 | 不加边；承接 D1 退场条件与 D4 退役条件 |
| #1366 | 不变（仍被 #1572 阻塞，随 #1640 后延） |

加边不成环：#1640 的上游为 #1774（其上游 #1775 已关）与 #1488（已关），不含 #1572 或 #1366。

## 授权

#1323 Notes N-1：T 类经独审 PASS 后可代理裁定。#1786 票面「本票不授权合 main、不改已签资源数值」：本 ADR 不改任何已签数值，不豁免任何已批合同反例，D2 升 P 触发保证一旦触及已签场景即交用户。
