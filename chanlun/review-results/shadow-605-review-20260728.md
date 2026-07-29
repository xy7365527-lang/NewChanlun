# 影子评审：#605 死路径定档 + p123 两长函数拆分 + 措辞订正

- 日期：2026-07-29（工位时区 2026-07-28 夜班收口）
- 评审车：claude（opus），**未参与 #605 实装**，新上下文独立评审，全程单线程（无子代理、无后台任务）
- 对象：`d6fc792ac1`（代码）+ `b16822885d`（报告），工位 `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- 权限自核：全程只读 + 跑测试；**未改任何源文件**（唯一写入 = 本报告 + `/tmp/shadow605-*` 临时产物；曾 `touch rust/src/bin/p123_fast_replay.rs` 仅改 mtime 触发重编，`git status` 复核内容未变）；**零 git mutation**
- 隔离面自核：开工 `git status` 显示并行未提交面 `rust/src/bin/p100_cert_bsp_recon.rs`、`rust/src/theta_v0/classifier/retrace_ledger/log.rs`（#621）——全程未碰；`ledger_kernel/`、`retrace_ledger/`、`level_view*` 未读未改

---

## 0. 结论先行

**代码面：通过。** `d6fc792ac1` 的两个文件改动经独立复现与逐段抽验，**零行为变化成立**，死路径**零删除**属实，`≤50 行` 的验收要求**实际达标**（比报告自称的还好）。

**报告面：有 1 项 HIGH + 2 项 MEDIUM。** 最重一条是 §1 定档建议表的「建议 5 删」**连带面评估不完整**——它把删除代价写成「删三件 + 几个测试夹具」，实际是**连带迁移 12 个 #421/#426 生命周期验收测试（F1–F9c）**，且这些测试喂书走的 provider 通道与生产**不是同一个**，不能 1:1 换夹具。编排者若按该表做终审，会系统性低估删除成本。

| 等级 | 条目 | 面 |
|---|---|---|
| HIGH-1 | 「5 删」连带面漏计 12 个 F 系列验收测试 + 通道语义不等价 | 报告 §1 |
| MEDIUM-1 | 「指纹开工态=收口态逐字一致」无开工态物证（留存基线日志是**编译失败**日志） | 报告 §4.2 |
| MEDIUM-2 | 基线 HEAD 引用错位——§2.1 映射表标 `9e3cd25fd6`，实际坐标系是 `02ef5f93b1` | 报告 §0/§2.1 |
| LOW-1 | 自报行数与「全部 ≤50 行」自相矛盾，且比实测差（自报 52/60，实测 45/21） | 报告 §0.2/§4.5 + commit message |
| LOW-2 | S-H2 点名的那句宣称原样保留，订正做成了并列而非替换 | `nest_lifecycle.rs:2277` |
| LOW-3 | 模块头「全库唯一调用点在 `#[cfg(test)]`」对 `leg_as_segment_copy` 不成立 | `nest_lifecycle.rs:2256` |
| LOW-4 | §1 全用改前坐标未标注，且用改前行号指认本轮**新增**的注释落位；「`mod tests` 起于 2548」两版都不对 | 报告 §1 |

---

## 1. 独立复现结果（逐项照实报数）

### 1.1 `cargo test --lib` 指纹 — 对上

```
test result: FAILED. 2130 passed; 1 failed; 137 ignored; 0 measured; 0 filtered out
failures: theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
  （src/theta_v0/classifier/signal.rs:3418，摘要 left 16618955402698307653 / right 10432481772907336594）
```

与报告/commit 自称的 `2130 passed / 1 failed / 137 ignored` **逐字一致**，唯一红即 #491（既知基线红，与本票两文件无关）。
注：本次跑在带并行未提交面的工作树上（`p100_cert_bsp_recon.rs`、`retrace_ledger/log.rs`），前者是 bin 不入 lib，后者入 lib 但编译通过且计数吻合。

### 1.2 #533 字节护栏 — 全绿

```
cargo test --release --test issue533_p123_byte_guardrail -- --ignored --nocapture
test p123_replay_2000bars_byte_exact ... ok
test p123_replay_large_windows_hash_guardrail ... ok
test result: ok. 2 passed; 0 failed
```

**这条比报告自己写的证据更强，值得点明**：该门的 golden（`rust/tests/fixtures/issue533_p123_*`）在 `d6fc792ac1` 中**未被触碰**（该 commit 只动 2 个文件），门内逐字节比对的是 2000-bar 全文 stdout/dump/P116 三面 + 20k/100k 的 stdout/dump SHA-256 + P116 全文。也就是说拆分后的产物与 **#602 时代落盘的 golden** 仍逐字节相同——这是比「与本轮 /tmp 基线对拍」更早、更独立的锚。

### 1.3 20k/100k 手工对拍 — stdout 零 diff，stderr 仅计时字段

```
cmp p123_20000_stdout  ↔ /tmp/issue605-baseline/p123_20k_stdout.before.txt   → 完全相同
cmp p123_100000_stdout ↔ /tmp/issue605-baseline/p123_100k_stdout.before.txt  → 完全相同
diff stderr：唯一差异 P123_SPARSE_SUMMARY 的 prefix_s=（20k 0.119↔0.215；100k 3.490↔6.552）
             其余 17 个计数字段（triggers/reevals/pan_hits/…）逐值相同
```

报告 §4.4 的描述**属实**。

### 1.4 编译

`touch` 后 `cargo build --release --bin p123_fast_replay`：**p123 自身零警告**（输出中无任何锚在 `p123_fast_replay.rs` 的 warning，也无 `bin "p123_fast_replay" generated N warnings` 汇总行）。lib 侧 37 条警告为既有面，非本票引入。

### 1.5 死路径零删除 — 属实

三个名字全部仍在：`leg_as_segment_copy`（`nest_lifecycle.rs:2263`）、`PanLiveRun`（`:2280`）、`provide_replay_live_windows`（`:2296`）。
全库调用点复验（`grep -rn --include=*.rs`）：

| 件 | 调用/构造点 | 位置性质 |
|---|---|---|
| `provide_replay_live_windows` | `nest_lifecycle.rs:3887` | `#[cfg(test)] mod tests`（起于 `:2503`）内 |
| `PanLiveRun` | 签名 `:2297`/`:3882`；构造 `:3951,4003,4076,4139,4179,4211,4244,4292,4330,4371,4891`（11 处） | 除两处签名外全在 tests 内 |
| `leg_as_segment_copy` | `nest_lifecycle.rs:2302` | **非测试代码**（在 `provide_replay_live_windows` 体内），仅传递可达测试 |

「仅测试可达」的结论成立；`leg_as_segment_copy` 的**直接**调用点位置见 LOW-3。

---

## 2. 拆分等价性抽验（5 段 + 全量规范化筛查）

### 2.1 逐段抽验（原文对拍 `d6fc792ac1~1` 导出的 `/tmp/shadow605-base/p123_old.rs`）

**段 A — `main()` 的审计计数时机（副作用序）**：旧 `audit.views += prefix_views; audit.snapshots += book.candidates.len();` 紧跟 prefix pass、**早于** `observe_snapshot` 往 `book.candidates` 追加。新版这两行搬进 `run_prefix_pass_and_report`（`:766-767`），仍在同一位置、仍早于 `observe_snapshot`。**若搬到 observe_snapshot 之后，`P123_PROVIDER snapshots=` 会漂——没漂，序正确。** 打印顺序 YIELD/CERT/D3 → BASELINE/PROVIDER/SNAPSHOT → BIT_EXACT/R7 → MISSED 完全保持。

**段 B — `compute_bit_diff` / `print_bit_diff_levels`（计数与打印分离）**：旧代码先算 4 个 diff 计数再条件打印逐级行；新版拆两函数后，`compute_bit_diff` 逐行照抄（含 `_ => old_semantic_diff += 1` 的缺级臂），`print_bit_diff_levels` 把 `if classification_diff != 0` 反相为早返回。两者与 `compute_old_semantic_counts` 的**相对顺序未变**（diff 计数 → 逐级打印 → `cand_delta_tower_cached`）。

**段 C — `recompute_stems_if_due`（可变性搬迁 + 借用重取）**：旧循环体内 `l1_view`/`l2_view` 在 bar 顶部取一次，既喂 `recompute_lifecycle_window_stems` 又喂 `PAN_LIVE_RECOMPUTE` 落盘行。新版拆成 `compute_lifecycle_due_keys`（只带键，不带 `units` 借用面）+ `recompute_and_merge_stems` 内重取 + 落盘前**再重取一次**。
**该等价性我独立核到根**：`TowerCache` 的 `level_scan_units` / `level_scan_cursor` / `tower_confirmed_len` / `freeze_boundary` / `causal_series` / `macd_dif` / `forest_epoch` 全部是 `&self`（`classifier/mod.rs:997/1011/1032/1083/1098/1108/1114`），且 `TowerCache` 结构体**无任何 `Cell`/`RefCell`/`Mutex`/`UnsafeCell` 字段**（即无内部可变性）；三次取值之间唯一可能改写 cache 的是 `classify_with_tower_incremental`，它在 bar 顶部、三次取值之前。**重取 ≡ 传参成立**，报告 §2.2 的「关键工程决策」注记核实无误。
执行序也逐条对上：`before` 快照 → lower/upper 重算 → merge/sort/dedup → seg_ledger 观测（`if lower_due` 门保留）→ HIT/MISS 行 → RECOMPUTE 行 → 写回两把 last key。

**段 D — `reevaluate_run` / `record_reevaluation_result`（可变借用拆分后的读取时机）**：`structure_generation` 与 `get_or_insert_run_entry` 的两个 gen 参数在 `evaluate_run` **之前**读；`entry.update(self_gen, lower_gen, …)` 的两个 gen 在 `evaluate_run` **之后**读——与旧版逐字同序。`evaluate_run` 只经 `ConfirmResidence.store` 改 `confirm_cursors`、经 `PanResidence.memo` 改 `entry.pan_memo`，不碰 `self_gen`/`lower_gen`，故前后读同值。`fresh_levels.insert(level)` 从旧的「计数之后、`entry.update` 之前」搬到调用方 `reevaluate_run(...)?` 之后——两处都在 `?` 之后，错误路径行为一致；`fresh_levels` 在本 trigger 结束前无读者，位置无语义。

**段 E — `feed_lifecycle_and_checkpoint`（逐 bar 尾段）**：`completion_events` 的「不到期即空 Vec」、checkpoint 门 `ckpt_every > 0 && index > 0 && index % ckpt_every == 0`、进度门 `index > 0 && index % 500_000 == 0` 全部反相为早返回，语义等价。
**`P123_PREFIX_PROGRESS` 需专门交代**：旧格式串用内联捕获 `views={views}`，新版改成位置参数 `views={}` 并在第 5 位传 `state.views`——占位符与实参一一核对后**输出逐字相同**。但该行只在 `index % 500_000 == 0` 触发，**20k/100k/2000 三档护栏窗口全部够不着**，即这一行的等价性**只有阅读证据、没有运行证据**。登记，不作为缺陷。

**段 F — `shadow_check_run`（未被任何护栏覆盖的分支）**：`P123_SHADOW` 未设时整条不执行，护栏窗口均未开。逐行读对：强制 `evaluate_run(…, None, None)` → 按 `pending` 过滤 → `shadow_checks += 1` → 长度与 `(EventKey, divergence_confirmed)` 逐元比对 → `shadow_mismatches <= 20` 时打印。`applied` 切片从「evaluate_run 之后取」改为「调用方先取后传」，其间 `out` 无写入，内容同值。等价。

### 2.2 全量规范化筛查（防眼看漏）

把旧 `run_targeted_prefix_pass`（`1086-1691`，606 行）与新拆分体（`1235-2297`）各自剥掉 `state.`/`&mut`/缩进/注释/纯括号行后排序取差集：

- **旧有新无**方向：全部可归为①局部 `let mut X` → 结构体字段初始化、②`(start, end)` → `range` 形参改名、③`views += 1` → `*views += 1`、④块内联 → 函数调用。**无一条业务语句消失。**
- **新有旧无**方向：全部为函数签名/形参/调用行/反相守卫。**无一条业务语句被塞进来。**

### 2.3 `≤50 行` 实测（LOW-1 的依据）

| 函数 | 报告/commit 自称 | 实测 |
|---|---|---|
| `main()` | 378→**52** | 651–695 = **45 行** |
| `run_targeted_prefix_pass()` | 606→**约 60** | 1302–1322 = **21 行** |

全文件扫描 `>50 行` 的函数只剩 7 个，全部是既有面（`recompute_lifecycle_window_stems` 277、`observe_snapshot` 97、`collect_snapshot_candidates` 81、`observe_certificates` 72、`evaluate_run` 54，加两个 `l2/l3_..._watermark` 测试函数 61/63）——即报告 §4.5 点名的 #532 覆盖面。**本轮新拆出的助手函数无一超过 50 行。** 结论：验收达标，只是报告把自己的成绩写差了（且按其自报的 52 反而超标，见 LOW-1）。

---

## 3. Findings

### HIGH-1 — 「5 删」的连带面漏计 12 个 F 系列验收测试，且通道语义不等价

**证据**

- 报告 §1 对三件的连带面只写到「全库唯一调用点是本文件 `#[cfg(test)] mod tests` 内的夹具」「另 11 处构造点…全部位于同一 `mod tests` 内」，未点名任何一个测试。
- 实际结构：`nest_lifecycle.rs:3880` 的测试助手 `feed_prefix_phases(&mut book, runs: &[PanLiveRun], …)` 在 `:3887` 调 `provide_replay_live_windows`，是**唯一**的消费者；它自身有 **25 个调用点**，分布在 **12 个验收测试**里：
  `f1_replay_feed_live_channel_emits_observation` / `f2_channel_switch_sets_structure_completed_and_stops_extension` / `f3_channel_switch_force_jump_and_q1_order_are_visible` / `f4_two_falsification_terminals_land_distinct_reasons` / `f5_retrograde_prefix_rejected_with_zero_book_change` / `f6_feed_exit_is_sidecar_events_equivalent` / `f7_terminal_skip_equals_feeding_terminal_identity` / `f8_completion_channel_takes_consolidation_domain_only` / `f9_completion_force_unavailable_is_explicit_audit` / `f9a_same_trigger_observe_then_complete_is_flash_lifetime` / `f9b_first_completion_counts_after_provisional_force_overtake` / `f9c_completion_only_first_signal_is_recorded_as_flash`。
  这不是「几个夹具」，是 **#421/#426 生命周期账本 feed 契约的整套验收证据**（换轨、力度反超、两类证伪终局、倒退拒收、sidecar 等价、闪现寿命…）。
- 更关键：这套测试喂进 `NestLifecycleBook` 的活窗，是 `provide_replay_live_windows` → `provide_pan_live_windows`（`:1603`，**遍历已完成 lower legs 找 C**）产的；生产走的是 `provide_active_pan_live_windows`（`:1804`，**active frontier 通道**）。二者的分野被 `nest_lifecycle.rs:1757` 的 doc 明写为「**#523 根因**」的分水岭，签名也不同（后者是单 run + `active` 段 + typed `PanLiveOutcome`）。

**失败场景**：编排者读 §1「零其他消费者」「删除必须同批次进行」，判定为一次低成本清理并批「删」。执行工位随即发现要么整删 12 个验收测试（#421/#426 的验收证据归零），要么把 25 个调用点迁到 `provide_active_pan_live_windows`——而后者不是换个函数名，是换一条在 #523 根因维度上语义不同的通道，需要重造夹具并重新论证每条断言仍成立。批复所依据的成本量级与实际相差一个数量级。

**建议**：§1 表补第 4 列「连带面」，逐件列出：(a) 受影响测试清单（12 条，指名）；(b) 迁移路径是否 1:1（不是，附 `:1757` 的分水岭 doc 与两个 provider 的签名差）；(c) 迁移后覆盖是升是降（迁到生产通道其实是**升**——现状是验收套跑在生产不用的通道上，这一点本身就该作为「删」的正面理由写进表里，而不是被略去）。

---

### MEDIUM-1 — 「指纹开工态 = 收口态，逐字一致」缺开工态物证

**证据**

- 报告 §4.2 标题即「全量测试指纹（开工态 = 收口态，逐字一致）」。
- 留存的唯一开工态物证 `/tmp/issue605-baseline/lib-test-baseline.log`（14551 字节）是一份**编译失败**日志：
  ```
  error[E0583]: file not found for module `portal`
    --> src/theta_v0/classifier/retrace_ledger/tests/mod.rs:17:1
  ...
  error: could not compile `newchan_rust` (lib test) due to 4 previous errors; 33 warnings emitted
  ```
  全文**没有任何 `test result:` 行**，因此不含 `2130 passed` 之类的开工态计数。
- 报告 §4.2 的注脚确实解释了「首次尝试时 #621 在提交中，编译不过；待 `9e44d98728` 落地后重跑恢复」——但**重跑的输出没有留存**，被覆盖或未落盘。
- 口径另有不一致：报告 §4.2 写 `cargo test --release --lib`，commit message 写 `cargo test --lib`。我复现的是 `cargo test --lib`（票面指定），计数吻合。

**失败场景**：日后有人质疑「#605 是否引入了新的红」，按报告索引去查 `lib-test-baseline.log`，拿到的是一份编译错误日志，无法证伪也无法证实「前后一致」。「一致」这半句目前只有实施车的口头声明。

**缓解（我这边补的）**：收口态指纹我已独立复现为 2130/1/137，唯一红 `extract_signals_bit_exact_digest_guard` 是 #491 既知基线红且锚在 `signal.rs`（本票未碰）。**结论本身站得住**，缺的是「开工态」那一半的可核证据。建议：要么补跑并留存开工态日志（可用 `git stash` 之外的隔离树重建），要么把 §4.2 标题改成实际能证的「收口态指纹 = 2130/1/137，唯一红 #491 与本票两文件无关」。

---

### MEDIUM-2 — 基线 HEAD 引用错位，§2.1 映射表按标签查不到

**证据**

- 报告 §0 写「基线 HEAD（开工核面）：`9e3cd25fd6` 系列」，§2.1 表头写「原行段（HEAD `9e3cd25fd6`）」，并给出 `652-667 / 669-684 / … / 996-1025` 与 `run_targeted_prefix_pass()`（原 `1230-1835`）。
- 实测 `9e3cd25fd6`（`docs(chanlun): 影子评审报告入库——#387 T1…`，2026-07-27 07:57）上：`fn main` 在 **406–704**（299 行），`fn run_targeted_prefix_pass` 在 **762**。表里的行段在这个 commit 上指向完全不同的代码。
- 真正的拆分前父提交是 `d6fc792ac1~1` = `02ef5f93b1`，其上 `main` = **651–1028**（378 行）、`run_targeted_prefix_pass` = **1086–1691**（606 行）——与表里的数字和报告自称的「378 行 / 606 行」**吻合**。
- 两个 blob 差距：`git diff --stat 9e3cd25fd6 02ef5f93b1 -- rust/src/bin/p123_fast_replay.rs` = **+1700 / −96**。

**失败场景**：审计者照 §0/§2.1 的标签 `git checkout 9e3cd25fd6` 去核映射表，12 条行段全部对不上，只能推翻整张表或重新自己找基线。

**订正**：§0 与 §2.1 的基线引用统一改为 `02ef5f93b1`（= `d6fc792ac1~1`）。`9e3cd25fd6` 若是工位开工那一刻的 HEAD，应另起一句说明「开工时 HEAD 为 X，但本表坐标系是拆分前父提交 `02ef5f93b1`」。

---

### LOW-1 — 自报行数与「全部 ≤50 行」自相矛盾，且比实测差

commit message 与报告 §0.2 同一句里同时写「`main()` 378→**52** 行」「`run_targeted_prefix_pass()` 606→**约 60** 行」和「**全部 ≤50 行**」——按其自报数，两个主目标函数都超标，与票面验收「两长函数拆 ≤50 行」直接冲突。
实测 `main()` = **45 行**（651–695）、`run_targeted_prefix_pass()` = **21 行**（1302–1322），且新拆出的助手函数无一 >50 行（见 §2.3）。**验收实际达标**，只需把自报数字改对。

### LOW-2 — S-H2 点名的那句宣称原样保留，订正做成了并列而非替换

`shadow-421-whole-20260728.md:59` 的 S-H2 明写「模块文档仍宣称其为『回放引擎循环内已具备的量』＝声明与实际不一致」。该句现位于 `nest_lifecycle.rs:2277`：

```rust
/// 单个 run 的行进中通道取数（回放引擎逐前缀评估循环内已具备的量）。      ← 未改
/// **仅供下方 `provide_replay_live_windows` 与其测试夹具使用——生产侧不构造本结构**（票 #605）。  ← 新增
```

被点名的原句一字未动，新增行以并列方式加在旁边。读者先读到的仍是那句被判为「声明与实际不一致」的表述。按 `no-patch-mentality` 的「诚实：代码能做什么就声明什么」，订正应替换宣称本身（例如「回放**适配层**逐前缀取数的形参载体」），而不是在其后追加反向限定。

### LOW-3 — 模块头「全库唯一调用点在 `#[cfg(test)]`」对 `leg_as_segment_copy` 不成立

`nest_lifecycle.rs:2256`（模块头）对**三件统一**断言「全库唯一调用点是本文件 `#[cfg(test)] mod tests` 内的夹具」。但 `leg_as_segment_copy` 的直接调用点在 `:2302`，位于 `provide_replay_live_windows` 函数体内——**非测试代码**，只是传递可达测试。
函数自身 doc（`:2262`「仅服务于下方 `provide_replay_live_windows`——该函数现仅测试可达」）说法准确。建议模块头那句加限定：「三件的**测试外调用闭包**只由 `provide_replay_live_windows` 一个入口进入，而该入口全库唯一调用点在 `#[cfg(test)] mod tests`」。

### LOW-4 — §1 全用改前坐标却未标注，且用它指认改后的落位；另有一处两版都错

先说清楚：§1 表格的行号**内部是一致的**，全部是**拆分前父提交 `02ef5f93b1`** 的坐标，并非我初判的 pre/post 混用——逐条复核如下。

| 报告 §1 写 | `02ef5f93b1`（父） | `d6fc792ac1`（现） |
|---|---|---|
| `leg_as_segment_copy` @ 2254 | 2254 ✓ | 2263 |
| `PanLiveRun` @ 2270 | 2270 ✓ | 2280 |
| `provide_replay_live_windows` @ 2284 | 2284 ✓ | 2296 |
| 唯一调用点 @ 3875 | 3875 ✓ | 3887 |
| 构造点起于 3939 | 3939 ✓ | 3951 |
| `lifecycle_leg_as_segment` @ `p123:1735` | 1735 ✓ | 2341 |
| **「`mod tests` 起于 2548 行区间」** | **2491 ✗** | **2503 ✗** |

两个实际问题：

1. **未标坐标系**：全表用父提交坐标，但表头只写位置不写版本；报告读者拿现 HEAD 去核，每一条都差 9–12 行（`p123` 那条差 606 行）。
2. **§1 尾段用改前坐标指认改后的落位**：该段写「**本轮已做**…见 `nest_lifecycle.rs:2248-2256`（模块头段落）、`:2268-2269`（`PanLiveRun` doc）、`:2280-2284`（`provide_replay_live_windows` doc）」——描述的是本轮**新增**的注释，却给的是**新增前**的行号。实测改后落位为 `:2248-2258` / `:2277-2278` / `:2290-2295`。这一处是真正的自指矛盾：新增内容在旧坐标系里根本不存在。
3. **`mod tests` 起于 2548 两版都不对**（父 2491 / 现 2503）。该数字只用于佐证「调用点在测试模块内」，结论不受影响（2503 < 3887 仍成立），但数字本身是错的。

---

## 4. 措辞订正三处的核证

| 项 | 报告声明 | 我的复验 |
|---|---|---|
| 死路径三件 doc 对齐 | 本轮落地 | **属实**（`d6fc792ac1` 的 nest_lifecycle diff，+18/−4）。内容准确性见 LOW-2 / LOW-3 |
| `IdentityVanished = 0` 失效订正 | 已在 #559 提前完成，本轮无需动作 | **属实**。`nest_lifecycle.rs:71-80` 明写「旧不变量『`IdentityVanished = 0`』**已撤销**」并给出拆两类原因码的新不变量。全库检索：其余 `IdentityVanished` 出现全部是枚举变体/字段 doc/账本逻辑（`:311/368/389/411/435/438/442/563/1020/1082/1261/1285/1287/1302`、`p409_pan_live_probe.rs:472`）或带日期的历史评审/escalate 文档，**无任何活口径仍把 `=0` 断言为当前不变量**。报告「不改写历史快照」的处置正确 |
| #421/#427 票体正文订正 | 未过账，交订正评论草稿待授权 | **草稿与票面要求对齐**（#605 验收「措辞订正三处」）。但需明确：**编辑 GitHub issue 正文不是 git mutation**，票面禁的是 git mutation；实施车援引的是「外部可见动作先确认」这条会话安全默认，理由成立但与票面禁令不是同一条。结论：该项**验收未闭合**，报告已如实披露，需编排者一句话授权即可收口 |

---

## 5. 复验确认（非本票缺陷，仅记录）

工作树上 **47 个** `chanlun/review-results/` / `chanlun/escalate/` 已提交文件仍处于物理缺失（`git status` 显示 ` D`）状态，与报告 §0.4 的记录一致，**至本次评审结束仍未处置**。本评审车同样未做任何恢复动作（属 git mutation）。清单在 `/tmp/issue605-baseline/deleted-files-list.txt`。

---

## 6. 结果包六要素

1. **结论**：`d6fc792ac1` 代码面通过（零行为变化 + 零删除 + `≤50 行` 达标）；`b16822885d` 报告面 1 HIGH / 2 MEDIUM / 4 LOW，主症在定档建议表的连带面评估与两处引用错位。
2. **定义依据**：#605 票面验收三条（逐件定档 + 文档对齐 / 两长函数 ≤50 行 / 零行为变化 + #533 门 + 指纹对照）；#225 五档树；`no-patch-mentality`「诚实：代码能做什么就声明什么」（LOW-2/LOW-3）；`formalization-validity-domain` 认识论等级（本评审的字节等价证据为 L2：BTC 单标的 2000/20k/100k 三窗真实数据，不外推其它标的/窗口）。
3. **边界条件**（结论翻转条件）：
   - 代码面「零行为变化」翻转条件：`P123_SHADOW=1` 或 `P116_CKPT>0` 或 `bars ≥ 500_001` 的运行下出现产物漂移——这三条分支**均在护栏窗口之外**，本评审只有阅读证据（§2.1 段 E/F）。
   - HIGH-1 翻转条件：若 12 个 F 系列测试确有另一条不经 `feed_prefix_phases` 的等价覆盖路径（我未找到），则连带面确如报告所述可忽略。
   - MEDIUM-1 翻转条件：若开工态 `cargo test --lib` 的完整输出在别处留存并显示 2130/1/137，则该条撤回。
4. **下游推论**：三件的「删/不删」终审**不应**基于现 §1 表做出——补齐连带面后再裁。若最终裁「删」，需同批安排 12 个 F 系列测试向 `provide_active_pan_live_windows` 的迁移，并在迁移中单独论证 #523 分水岭维度上的断言仍成立。
5. **谱系引用**：S-H2（`shadow-421-whole-20260728.md:59`）、T-3（`shadow-527-review-20260728.md:329`）为本票死路径条目的上游；`issue421-acceptance-selfcheck-20260727.md:50/203`、`shadow-429/430-rereview-20260728.md` 的「授权保留」判定，本票主张订正——该订正主张我**认可其推理**（服务对象已死 ⟹ 保留理由悬空），但其成本估计不完整（HIGH-1）。#559 裁定（`IdentityVanished` 不变量替换）已核实落地。
6. **影响声明**：本评审**未改动任何源文件、未做任何 git 操作**。产出仅本报告一份，落 `chanlun/review-results/shadow-605-review-20260728.md`。跑过 `cargo test --lib`、`cargo test --release --test issue533_p123_byte_guardrail -- --ignored`、`cargo build --release --bin p123_fast_replay`、两次 `p123_fast_replay` 直跑（20k/100k，输出落 `/tmp/shadow605-out/`），均为读侧操作；`touch rust/src/bin/p123_fast_replay.rs` 只改 mtime，`git status` 已复核内容未变。
