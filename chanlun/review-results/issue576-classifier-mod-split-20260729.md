# #576 实施报告：classifier/mod.rs 按域拆分（段 1 完成，段 2/3 阻塞登记）

- 日期：2026-07-29
- 分支/工位：`kimi-nest-mainline-20260717` @ `/tmp/kimi-nest-mainline`
- 执行：claude opus 实施层（单线程主上下文，无子代理）
- 票：#576（含 #612 并入、`cand_event.rs` 扩项登记）
- 先例口径：#359（coverage.rs 6909 → 18 文件，机械证据口径）

---

## 0. 结论（先说范围事实）

**票面三段范围里，只有段 1（mod.rs 拆分）在本工位分支上有输入。段 2/3 的输入文件在本分支不存在。**

| 段 | 内容 | 状态 |
|---|---|---|
| 1 | `classifier/mod.rs` 按域拆至各文件 ≤800 | **已完成**，5 个 commit |
| 2 | `cand_event.rs` 按域拆生产代码 | **阻塞**（文件不在本分支，见 §5） |
| 3 | #612 观察/事件类型分离（`CandidateObservation` 去 `Invalidated`） | **阻塞**（同上，类型不在本分支） |

段 1 与段 2/3 无数据依赖（275 局部依赖：mod.rs 拆分不消费 `cand_event.rs` 的任何输出），
故段 1 全量做完，未因段 2/3 阻塞而停。

---

## 1. commit SHA 列表（逐个可编译）

| # | SHA | 内容 | 编译 |
|---|-----|------|------|
| 批1 | `ccd0830df5` | 三个内联模块外迁文件：`cp_replay_diagnostics` / `stage_profile` / `oracle_probe` | ✅ |
| 批2 | `573644425d` | 增量塔缓存段 → `tower_cache.rs` | ✅ |
| 批3 | `074d67e6b9` | 分类主管线 → `pipeline.rs`；Cand^δ 驱动器 → `cand_delta.rs` | ✅ |
| 批4 | `8db5a94a2b` | 增量塔入口 → `incremental.rs`；第二类组装 → `sublevel.rs` | ✅ |
| 批5 | `fe69f2460b` | 测试夹具段 → `tests/` 四文件；`incremental_profile.rs`；mod.rs 收口 | ✅ |

每个 commit 后均实跑 `cargo build --release --lib` + `cargo test --release --lib` + p123 20k 对拍。
具名 `git add`，无 `-A` / `.`，无任何 git mutation（无 push/reset/rebase/checkout/stash/amend）。

---

## 2. 拆分映射（原行段 → 新文件）

基线 `a9756715b1:rust/src/theta_v0/classifier/mod.rs` = **3922 行**
（票面写 4697 行，那是 `ticket-550` 分支上的行数——含 #550 的 +961，未并入本分支；见 §5）。

| 原 mod.rs 行段 | 内容 | 新文件 | 行数 |
|---|---|---|---|
| 1–121（保留） | 模块头文档 + 20 个 `pub mod` 声明 + `pub use turn_class::…` | `mod.rs` | **152** |
| 109–174 | `pub mod cp_replay_diagnostics { … }`（P52 frontier 只读计数器） | `cp_replay_diagnostics.rs` | 65 |
| 185–532 | `LevelState` / `Classification` / `segment_to_unit` / `unit_to_segment` / `extract_first_third_for_level` / `detect_centers_{complete,geometric,with}` / `classify_level` / `classify_impl` / `classify` / `classify_with_tower` | `pipeline.rs` | 356 |
| 534–747 | `cand_delta_tower{,_cached,_entry_tower}` / `cache_series_ok` / `cand_delta_tower_with_series` / `cp_recall_upper_bound_audit` | `cand_delta.rs` | 223 |
| 749–1329 | 增量塔 API 段头 + `LevelCache` / `AreaCache` / `TowerCache`（13 个 pub 方法）/ `update_closes_cache` / `compute_macd_hist_incremental` / `rebuild_macd_state_to` | `tower_cache.rs` | 581 |
| 1331–1455 | `pub mod stage_profile { … }`（阶段计时插桩） | `stage_profile.rs` | 122 |
| 1457–1559 | `#[cfg(test)] pub mod oracle_probe { … }`（A3 证书 oracle 探针） | `oracle_probe.rs` | 101 |
| 54–63 + 1561–2359 | `CASCADE_EPROBE` / `CASCADE_FULLCLEAR` 两开关静态 + `classify_with_tower_incremental` | `incremental.rs` | **820**（超限，§4） |
| 2361–2578 | `extract_second_for_level` / `second_for_parent` / `extract_second_resume` / `sublevel_diverges` / `cached_segment_area` / `rmove_direction` | `sublevel.rs` | 224 |
| 2580–3846 | `#[cfg(test)] mod tests { … }`（1267 行）按域拆 5 文件 | `tests/` | 见下 |
| ├ 2604–2613, 2668–2693 | 共享夹具 `seg` / `bars_from_closes` + 子模块声明 | `tests/mod.rs` | 30 |
| ├ 2587–2603, 2614–2667, 2694–2763 | p52 诊断 / l0_units 同步 / MACD dif bit-exact / cand cache 守卫 | `tests/cache_and_units.rs` | 149 |
| ├ 2764–3228 | B2 端到端 / 单元⇄线段互逆 / 级别-N 一三类 / Q7 裁定C / 两个 BTC 普查 | `tests/level_signals.rs` | 484 |
| ├ 3229–3450 | classify 基础（空层 / 自然终止 / 中枢成型 / l_max / 三类信号 / tower 等价） | `tests/classify_basics.rs` | 225 |
| └ 3451–3846 | 增量塔 bit-exact + cascade + forest_epoch + 标度 | `tests/incremental_tower.rs` | 402 |
| 3848–3922 | `#[cfg(test)] mod incremental_profile { … }` | `incremental_profile.rs` | 73 |

合计 15 个文件，4007 行（原 3922 行 + 85 行 = 模块头/`use`/`mod` 声明的必要增量）。

### 可见性口径（保全，非放宽）

原 mod.rs 里的私有项，可见域 = 「`classifier` 模块**及其后代**」。迁入子模块后若保持 `fn`（私有），
可见域会**收窄**为「该子模块及其后代」，跨文件调用编译失败。故一律标注 `pub(super)`——
`classifier::X::item` 的 `pub(super)` 恰等于「`classifier` 及其后代可见」，与拆分前**逐字等价**。

共标注：`pipeline.rs` 8 个 fn、`cand_delta.rs` 2 个 fn、`sublevel.rs` 6 个 fn、
`tower_cache.rs` 1 struct + 1 type + 2 fn + 30 个结构体字段（`LevelCache` 19 + `TowerCache` 11）。
`rebuild_macd_state_to` 仅本文件内消费，保持私有（不做无谓放宽）。

`impl TowerCache` 的 13 个方法原本就是 `pub`，未动。

---

## 3. 机械证据四件套

### (1) 字节级纯移动 — 行级规范化核验

把基线 `a9756715b1` 的 mod.rs 与拆分后 15 个文件全部做「去首尾空白 + 丢空行」规范化后取多重集差：

- 原文件独有 3716 → 差集全部落在 6 类已登记变换上；
- 新文件独有 3787 → 同上；
- **零条可执行语句行（表达式/赋值/控制流/断言）出现在差集中**。

差集的 6 类（穷举，无第七类）：
1. 内联模块的 `// ═══` 段注释 → 所在文件的 `//!` 模块文档（22 行注释重排，无代码）；
2. `pub(super)` 可见性标注（50 处，§2 已列）；
3. 新增的 `mod` / `pub use` / `use` 声明行；
4. `super::` 路径随模块下沉一级补正（`tower_cache.rs` 2 处、`tests/*.rs` 12 处；文档行不动）；
5. 被摘除的 `mod X {` 开括号与 5 个配对 `}`；
6. 新写的模块头文档。

### (2) pub 项原样导出（全仓引用路径零变化）

`mod.rs` 用 `pub use` 把全部对外项按**原路径**重新导出，仓内引用点计数（`rust/src` + `rust/tests`）：

| 项 | 引用点 | 项 | 引用点 |
|---|---|---|---|
| `classifier::Classification` | 126 | `classifier::TowerCache` | 53 |
| `classifier::classify_with_tower` | 48 | `classifier::classify_with_tower_incremental` | 30 |
| `classifier::LevelState` | 30 | `classifier::stage_profile` | 15 |
| `classifier::cand_delta_tower_cached` | 11 | `classifier::classify` | 11 |
| `classifier::oracle_probe` | 10 | `classifier::cp_replay_diagnostics` | 4 |
| `classifier::cand_delta_tower` / `cand_delta_entry_tower` / `cp_recall_upper_bound_audit` | 各 1 | | |

**全仓零处引用路径改写**（本票 diff 只触碰 `classifier/` 内我面的文件，无一处外部调用点被改）。

### (3) 测试函数名集合 IDENTICAL（剥模块前缀）

`classifier::tests` 的 30 项叶名，拆分前后 `diff` **空**：

```
diff base-classifier-tests-leaf.txt final-classifier-tests-leaf.txt   → IDENTICAL (30 项)
```

模块前缀分布：`cache_and_units` 4 / `classify_basics` 10 / `incremental_tower` 8 / `level_signals` 8 = 30。

全仓测试名集合（2281 → 2312）剥前缀后差异 31 条，**逐条属 `retrace_ledger::tests::`
（#622/#623 并行车在本 session 内落的新测试，非我面）**：`rebase_audit` 12 / `rebase` 9 /
`two_pass_evidence` 5 / `dead_center` 5。

### (4) 失败集逐字一致 + p123 对拍

| 项 | 基线（开工，`a9756715b1` + 他人未提交态） | 收口 |
|---|---|---|
| `cargo test --release --lib` | **2144 passed / 0 failed / 137 ignored** | **2175 passed / 0 failed / 137 ignored** |
| 失败集 | **空** | **空**（逐字一致：两侧均无 FAILED 行） |
| p123 20k stdout sha1 | `738cb880eb2c3437c3920d4811913e14fc5663ef` | `738cb880eb2c3437c3920d4811913e14fc5663ef`，`diff` 零 |
| lib 警告 | 37 | 37（零新增） |
| lib-test 警告 | 53 | 53（零新增） |
| `cargo test --release --no-run`（全目标，含集成测试） | — | 0 error |

passed 从 2144 → 2175（+31）的漂移已在 (3) 逐条归因到并行车的 `retrace_ledger` 面；
`git status` 源码 sha 快照对比同样显示漂移只在 `retrace_ledger/tests/` 六个文件。

p123 命令：`P116_MAX_BARS=20001 ./target/release/p123_fast_replay analysis/data_cache/btc_1m_full.json`，
EXIT=0，16 行门行逐字相同（`P123_YIELD candidates=50 … terminal_confirmed=6`、
`P123_CERT caliber_A=8 caliber_B=6`、`P123_BIT_EXACT … classification_total_diff=1` 等全部不变）。

---

## 4. Standards 轴

### 文件 ≤800

14/15 达标。**1 项超限，单列登记**：

| 文件 | 行数 | 登记理由 |
|---|---|---|
| `incremental.rs` | **820** | 文件内容 = 单个 765 行函数 `classify_with_tower_incremental` + 34 行函数文档 + 20 行头/`use`/两个 `#[cfg(test)]` 开关静态。纯移动不可能收敛到 800。 |

不做非纯移动的切分：该函数的实际债是「函数 ≤50 行」，拆文件治不了；把它切成
「640 行主体 + 145 行前奏」既不满足 50 行，又要重排 `mem::take`/`&mut cache` 的借用与
drop 次序——为凑一个行数阈值引入行为风险，属补丁思维（`no-patch-mentality` 第 6 条「妥协方案」）。
收敛须走设计级分解，另票。

### 函数 ≤50

**11 处超限，全部为存量（逐字移动，未改一行）**，登记如下：

| 文件:行 | 函数 | 行数 |
|---|---|---|
| `incremental.rs:56` | `classify_with_tower_incremental` | 765 |
| `pipeline.rs:183` | `classify_impl` | 141 |
| `tower_cache.rs:483` | `compute_macd_hist_incremental` | 75 |
| `cand_delta.rs:129` | `cand_delta_tower_with_series` | 54 |
| `incremental_profile.rs:13` | `profile_incremental_tower_real_scaling` | 61 |
| `tests/level_signals.rs:321` | `type1_funnel_census_btc` | 164 |
| `tests/incremental_tower.rs:335` | `incremental_tower_scaling_dominates_full_synthetic` | 68 |
| `tests/incremental_tower.rs:172` | `cascade_reset_on_frontier_interior_rewrite` | 66 |
| `tests/level_signals.rs:22` | `end_to_end_second_buy_via_l1_l2_geometric` | 56 |
| `tests/level_signals.rs:256` | `level_signal_census_btc` | 55 |
| `tests/cache_and_units.rs:33` | `l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink` | 51 |

本票口径是零行为变化的纯移动，函数级收敛不在其内。

### 零新增警告

lib 37 / lib-test 53，与基线同数同集。中途出现的 4 处新警告（未用 import）当场清掉，
未留到 commit。

---

## 5. 段 2/3 阻塞：数据依赖证明

票面段 2/3 的输入是 `rust/src/theta_v0/classifier/cand_event.rs`（票面写 1120 行）
与其中的 `CandidateObservation` 类型、`event_probe` 探针、`cea839011a` 的运行期 panic 锁。

**这些在本工位分支上不存在。** 实测：

```
find rust/src -name "cand_event*"        → 无
grep -r "CandidateObservation" rust/src  → 0 命中
grep -r "event_probe"          rust/src  → 0 命中
grep -r "cand_event"           rust/src  → 2 命中（recursive_tower.rs 的两个测试函数名，非该文件）
```

血缘：`cand_event.rs` 由 `a353a45700` / `2c2214d30c`（#550）引入，活在
`ticket-550` / `ticket-551` / `ticket-552` / `ticket-553` 四个分支上。
它们与 `kimi-nest-mainline-20260717` 在 `7b4547b623` 分叉，对侧 35 提交、本侧 56 提交，**未并入**：

```
git merge-base --is-ancestor a353a45700 HEAD  → NO
git merge-base --is-ancestor cea839011a HEAD  → NO   （#612 要退役的 panic 锁）
git diff --stat $(git merge-base HEAD ticket-553) ticket-553 -- rust/src/theta_v0/classifier/
  → cand_event.rs | 1417 ++++++++++++++++++   （相对分叉点是纯新增）
```

同理，票面「现状 mod.rs 4697 行」也是 `ticket-550` 侧的行数；本分支基线 3922 行，
差额 775 行即 #550 那一批未并入的内容。

**为什么不自行并入**：(a) 本票授权只覆盖 commit，明禁 merge/rebase/checkout 等 git mutation；
(b) 并入 35 个提交是携带行为变更的功能合并，与本票「零行为变化纯移动」的验收口径互斥，
不能塞进同一票。

**解阻条件**：`ticket-55x` 线并入本分支后，段 2/3 可直接接着做——设计已在票面评论钉死
（按域拆 key/types + event book + observation 三文件、各带 `mod tests`；不走「迁 rust/tests/
独立 crate」路线，因 `event_probe` 是 `#[cfg(test)]`，跨 crate 会静默废断言）。
本票段 1 的 `tests/` 拆法已按同一硬事实执行：子模块留在本 crate 内，探针照常计数。

**#612 验收项现状记录**：本分支 `Invalidated` 观察侧构造点 = 0（因为 `CandidateObservation`
本身 0 命中）。这是「类型不存在」而非「类型分离已完成」，不作为 #612 的通过证据。

---

## 6. 偏离 / 存疑

1. **票面行数与实际不符**（4697 vs 3922；`cand_event.rs` 1120 行 vs 不存在）——已在 §5 归因到分支分叉，非误读。
2. **内联模块外迁必然去 4 空格缩进**，严格说不是「字节相同」。#359 同型先例亦然。
   代偿证据是 §3(1) 的行级规范化核验（去空白后零代码行差异）+ p123 字节对拍。
3. **`use super::*;` 的用法**：`pipeline` / `cand_delta` / `tower_cache` / `incremental` / `sublevel`
   五个私有子模块都用 `use super::*` 取 `mod.rs` 的共享导入面。这是为了让被移动的代码
   **完全不改一个标识符**就能解析——换成逐文件显式 import 会引入拼写/遗漏面。
   `mod.rs` 里因此保留了一个只被子模块消费的 `use` 块，已就地加注释说明它不是死导入。
4. **基线是「他人未提交态」**：开工时 `retrace_ledger/` 有 #622/#623 的在制品未提交，
   基线跑在那个状态上。收口时它们已 commit 且又加了 31 个测试。漂移已逐条归因（§3(3)），
   但严格说基线与收口不是同一个「他人面」快照——这是三车共用一个 worktree 的固有约束，
   非本票可消除。p123 对拍两端 sha1 完全相同，是对该约束的最强旁证。
5. **未跑 `cargo test --release` 全量执行**（只跑了 `--lib` 全量 + 全目标 `--no-run` 构建）。
   理由：开工基线取的是 `--lib`，全量执行没有可比基线；集成测试在本票 diff 面上无引用点变化
   （§3(2) 全仓零处路径改写）。若需要，可另跑一轮建立基线。

---

## 7. 结果包六要素

1. **结论**：`classifier/mod.rs` 3922 → 152 行，按域拆为 15 个文件，14/15 ≤800；零行为变化。
   段 2/3 因输入文件不在本分支而阻塞，附数据依赖证明。
2. **定义依据**：#359 的机械证据口径（字节级纯移动 + pub 项原样导出 + 测试名集合 IDENTICAL）；
   Rust 可见性规则「私有项对定义模块及其后代可见」——`pub(super)` 是该可见域在子模块内的等价表达。
3. **边界条件**（结论会翻转的条件）：
   - 若某个 `pub(super)` 实际放宽了可见域（例如被 `classifier` 外的模块访问）→ 「可见性保全」不成立。
     实测：`pub(super)` 的语法上界就是 `classifier` 子树，编译器强制，不可能外泄。
   - 若 p123 20k 覆盖不到某条被移动的分支 → 「零行为变化」的字节证据有效域不覆盖该分支。
     旁证是 2175 个单测 + `--no-run` 全目标构建，但 20k 窗确实不是全窗。
   - 若并行车在基线与收口之间改动了**我面之外但被我面调用**的代码 → 失败集/对拍的可比性下降。
     实测漂移仅在 `retrace_ledger/tests/`（纯测试面，不被 classifier 主体调用）。
4. **下游推论**：`classifier::*` 的全部对外路径不变 ⟹ 上游 30+ 个消费点（backtest / strategy /
   trading / bin）零改动；后续对 `classify_with_tower_incremental` 的设计级分解现在有了独立文件面，
   可单文件评审。
5. **谱系引用**：`no-patch-mentality` 第 6 条（妥协方案）→ `incremental.rs` 超限选择单列登记
   而非为凑行数做有风险的函数切分；275 局部依赖 → 段 2/3 阻塞不向段 1 传导；
   `formalization-validity-domain` → §7.3 明确 p123 20k 的有效域不是全窗。
6. **影响声明**：改动限于 `rust/src/theta_v0/classifier/` 下 15 个文件（1 改 + 14 新建）。
   未碰 `retrace_ledger/` / `ledger_kernel/` / `signal.rs` / `nest_lifecycle.rs` / `level_view*`
   的任何逻辑行，未碰任何 re-export 登记以外的他人面，未改动仓外任何文件。
