# #633 实施报告：`classify_with_tower_incremental` 765 行函数设计级分解 + 存量超限函数同批

- 日期：2026-07-29
- 分支/工位：`kimi-nest-mainline-20260717` @ `/tmp/kimi-nest-mainline`
- 执行：claude opus 实施层（单线程主上下文，无子代理、无后台任务）
- 票：#633（上游 #576 段 1 收口登记）
- 先例口径：#576（机械证据四件套）、#359（纯移动登记变换）

---

## 1. 次序保持证明（先证后拆——票面硬要求）

分解的唯一风险是**改变可观察次序**。本节把 765 行函数里全部次序敏感点逐一列出、给出保持条件，
并声明哪些点因此**不拆**。证明不了的点不拆——这是本票的执行边界。

### 1.1 次序敏感点全枚举（T1–T14）

下表的「原行号」指基线 `797c9ad35c:rust/src/theta_v0/classifier/incremental.rs`。

| # | 次序点 | 原行 | 约束 | 保持方式 |
|---|---|---|---|---|
| T1 | `cache.clear()`（段账本回缩）**必须先于** `l0_units_cache` 构建 | 80–82 → 87–95 | #613/#609 F2：颠倒则 `l0_units` 被清空而 `tower[0]` 满载，只读契约破裂（BTC 100k @as_of=71040 实测） | 两段同属 `prepare_l0_inputs`，函数内相对次序逐字保持 |
| T2 | `Rc::make_mut(&mut cache.l0_units_cache)` **必须先于** `Rc::clone` 出借 | 91 → 99 | make_mut 时 `strong_count==1` 才走原地 `truncate+extend` O(tail)；先 clone 则计数为 2 ⟹ 每 bar 写时复制（O(n²) 回归，非 bit-exact 破裂但标度回退） | 两句同属 `build_l0_units_cache` + 紧随其后的 clone；clone 留在 `prepare_l0_inputs` 骨架，调用序不变 |
| T3 | `Rc::make_mut(&mut cache.moves_tower_l0)` **必须先于** `Rc::clone` | 156 → 164 | 同 T2 | 同 T2（`rebuild_l0_tower` 后紧跟 clone） |
| T4 | `forest_dirty_l0` 逐值比对 **必须先于** L0 塔重建 | 141–153 → 154–163 | 比对读的是 `cache.moves_tower_l0` 的**旧值**；重建后比对恒为 false ⟹ E1 判据失效、`forest_epoch` 漏 bump ⟹ 下游 `TreeCache` 假命中陈旧森林 | 拆为 `l0_tower_tail_changed`（只读）与 `rebuild_l0_tower`（只写），骨架内调用序固定 |
| T5 | 空 L0 早返回 **必须先于** `last_l0_segments_len` / `l0_confirmed_len` 写入 | 117–120 → 122–124 | 空 L0 走 `cache.clear()`（两字段归零）后返回；若先写后判，clear 会把刚写的值清掉——语义等价但 `clear()` 的 `generation`/`forest_epoch` bump 次序会与写入交错 | `prepare_l0_inputs` 返回 `Option`，`None` 分支逐字保留原早返回块 |
| T6 | `update_closes_cache` **必须先于** `mem::take(&mut cache.closes)` | 179–181 → 182 | take 前 cache.closes 必须已是本 bar 的值；顺序颠倒 ⟹ take 走上 bar 的旧值、update 写进被 take 空的槽 | 三行连续留在**主函数**，不下沉 |
| T7 | `mem::take` 出借（closes/close_src/closes_tick）**必须先于** `compute_macd_hist_incremental(&closes, …, cache)` | 182–186 → 188–190 | take 的**唯一目的**是解除 `&cache.closes` 与 `&mut cache` 的别名；不 take 则该调用编译失败 | 同 T6，留在主函数 |
| T8 | `stable_len = cache.macd_state_len` **必须后于** `compute_macd_hist_incremental` | 188–190 → 199 | 该函数写 `macd_state_len`；先读则拿到上 bar 水线 ⟹ `AreaCache` 会缓存 unstable tail 的 area，污染未来 bar（`tower_cache.rs` `AreaCache` 文档明禁） | 同 T6，留在主函数 |
| T9 | `mem::take(area_cache)` 出借 / `into_inner()` 放回，与循环体的包夹关系 | 200 → 773 | `RefCell<AreaCache>` 必须在**栈上活过整个循环**（`divergence_of` 闭包按 `&RefCell` 捕获）；放回必须在循环结束后 | `RefCell` 变量留在主函数栈帧，循环函数只接 `&RefCell<AreaCache>`；放回四行逐字保留在循环调用之后 |
| T10 | 缓冲放回（770–773）**必须先于** `generation`/`forest_epoch` bump 与末尾 `debug_assert!` | 770–773 → 787–817 | 放回是 `&mut cache` 的可变借用，bump 同样；NLL 下 `hist`/`dif`（借 `cache.macd_hist`/`macd_dif`）的最后使用点必须早于放回 | 放回、bump、assert 三段逐字保留在主函数尾部，次序不变 |
| T11 | `tower_snapshots.push(mem::take(&mut moves_tower))` 与 `moves_tower` 重赋值的窗口 | 585 → 766 | take 后 `moves_tower` 是空 `Rc` 直至循环尾重赋；中间**不得有读**（原码 550–558 `advance_cp_lifecycles(&moves_tower)` 在 585 之前，是最后一次读） | `moves_tower` 保持为循环骨架的局部 `mut` 变量；`advance_cp_lifecycles` 归入 585 之前的 `scan_level_tail`，take 留在骨架 |
| T12 | `Rc::make_mut(&mut lc.projected_units)`（stage 09）依赖上一 bar 同级 `units` 已 drop | 716–720 | 跨 bar：上 bar 的 `units` 在上 bar 循环尾/函数返回时 drop ⟹ 本 bar `strong_count==1` ⟹ 原地 O(tail) | `units` 保持为循环骨架局部变量、不进任何跨 bar 结构；被抽函数按值/按引用接收，drop 时点不变 |
| T13 | `levels.push(LevelState{..})` **必须先于** stage 09/10 读 `levels.last().moves` | 699–706 → 726/746 | `moves` 在 590 产出后于 699 被 move 进 `LevelState`，之后只能从 `levels.last()` 读 | `assemble_level_state` 返回 `LevelState`，骨架 push 后再调 `derive_next_level_inputs(&levels, …)` |
| T14 | `#[cfg(test)]` 探针（`oracle_probe::*`）的调用**次数与位置** | 251/318/527/757/800 | 探针是计数器/直方图，多调一次即污染 oracle 断言 | 每个探针留在其原逻辑位置的被抽函数内，条件表达式逐字保留 |

### 1.2 借用面证明（分解引入的新约束）

原码在同一栈帧内同时持有：

- `hist: &[f64] = &cache.macd_hist`、`dif: &[f64] = &cache.macd_dif`（共享借用，行 191/193）
- `lc: &mut cache.levels[level_idx]`（可变借用，行 262）

这依赖 Rust 的**字段不相交借用**（disjoint field borrow，原码行 192 注释已声明）。
把循环外提为函数时，签名**不能**接 `cache: &mut TowerCache`——那会把整个 `cache` 可变借走、
与 `hist`/`dif` 冲突。故循环函数按字段接参：

```
fn build_level_tower(
    l0: &ParseLayer, config: &ThetaConfig,
    levels_cache: &mut Vec<LevelCache>,      // ← 只借 cache.levels
    prelude: L0Prelude,
    series: &LevelSeries<'_>,                // ← 持 &cache.macd_hist / &cache.macd_dif
) -> TowerBuild
```

调用点 `build_level_tower(l0, config, &mut cache.levels, prelude, &series)` 的两个借用路径
`cache.levels` 与 `cache.macd_{hist,dif}` 不相交，borrowck 通过。返回类型 `TowerBuild` **不含
任何借用** ⟹ 调用结束即释放全部借用，随后的 `cache.closes = closes`（T10）合法。

**这比原码更严格**（原码里 `hist` 的借用靠 NLL 推断结束点，新码靠类型系统保证），不放宽任何面。

### 1.3 注释归属（行数收敛的合法杠杆，非"凑数"）

`#576` §4 的函数行数口径是**函数体物理行**（`incremental.rs` 820 = 765 函数 + 34 行**函数文档** + 21 行头/use/static
⟹ 文档注释不计入函数行数）。原码 765 行里约 55% 是行内 `//` 设计推导注释。

分解时这些注释**按其描述的代码去向搬迁**，且凡是描述「这段代码为什么必须这样」的整段推导，
提升为被抽函数的 `///` doc 注释——这是它们的正确归属（函数契约），不是删除、不是折叠。
**零条注释被丢弃**（§4 差集核验逐条落账）。

### 1.4 不拆点登记

| 段 | 原行 | 不拆理由 |
|---|---|---|
| `update_closes_cache` → `mem::take` ×3 → `compute_macd_hist_incremental` → `stable_len` → `mem::take(area_cache)` | 179–200 | T6/T7/T8 三条次序约束互锁，且 take 的所有权必须与放回（770–773）同栈帧。下沉到子函数需把 4 个 `Vec` 所有权来回搬运两次，drop 时点与借用窗口都会变——**证明不了等价，故不拆**，逐字留在主函数 |
| 缓冲放回 + epoch bump + 末尾 `debug_assert!` | 769–817 | T10：三段的相对次序与 NLL 借用结束点耦合；总计 12 行代码，拆无收益。`bump_tower_epochs` 仅把 787–798 的两个 `if` 收进函数（次序内部保持），末尾 assert 独立成 `debug_assert_l0_units_in_sync` |

---

## 2. 分解方案（三层，每层函数 ≤50）

> **本节是开工前的方案（先证后拆的「拆」侧），落盘早于实施**。实装过程中有三处命名/结构调整，
> 以 §4 的实装映射为准：`level_cache_full_clear`/`level_cache_retain_prefix` 实名
> `clear_level_cache`/`retain_level_prefix`；`scan_level_tail` 实名 `scan_and_extend_level`
> 且吸收了 `advance_confirmed_watermark` 与两条护栏；`NextLevelInputs` 载体在批 6 引入
> `TowerLoop` 后被删除（`process_level` 改为就地推进携带量，不再需要返回载体）。
> 方案里的每一条**次序约束**在实装中一条未改。


```
classify_with_tower_incremental                      ← 入口（≤50）
├── prepare_l0_inputs → Option<L0Prelude>            ← 序幕（≤50）
│   ├── reset_cache_on_segment_shrink                (T1)
│   ├── build_l0_units_cache → usize                 (T2)
│   ├── diag_l0_units_parity                         (DIAG_L0UNITS 探针)
│   ├── l0_tower_tail_changed → bool                 (T4 读侧)
│   └── rebuild_l0_tower                             (T4 写侧, T3)
├── [不拆] closes/close_src/closes_tick/area_cache 出借 + MACD  (T6–T9)
├── build_level_tower → TowerBuild                   ← 逐级循环骨架（≤50）
│   ├── apply_frontier_invalidation                  (行 264–403)
│   │   ├── level_cache_full_clear                   (P=0 支)
│   │   └── level_cache_retain_prefix                (P>0 支)
│   │       ├── rebuild_cursor_for_retained_prefix   (含 O2 哨兵 debug_assert)
│   │       └── truncate_second_cache                (b2 分离锚)
│   ├── sync_cached_units                            (行 405–418)
│   ├── scan_level_tail → LevelScan                  (行 420–579)
│   │   ├── pop_frontier_window → PoppedFrontier     (行 432–461)
│   │   ├── reinherit_cp_lifecycles → usize          (行 485–521)
│   │   └── advance_confirmed_watermark              (行 560–573)
│   ├── assemble_level_state → LevelState            (行 587–706)
│   │   ├── extract_level_bsp                        (行 599–680)
│   │   │   ├── extract_first_third_for_level_input  (L0/L≥1 双支)
│   │   │   └── —（07b/memo 命中支留骨架）
│   │   └── build_level_projection                   (行 684–698)
│   └── derive_next_level_inputs → NextLevelInputs   (行 708–748)
└── [不拆] 缓冲放回 → bump_tower_epochs → debug_assert_l0_units_in_sync  (T10)
```

新增数据类型（全部私有、全部 `#[derive(Debug)]`-free 的纯载体，无行为）：

- `L0Prelude { l0_units, moves_tower_l0, l0_dirty_from, forest_dirty_l0 }`
- `LevelSeries<'a> { hist, dif, closes_tick, close_src, area_cache, stable_len }`
- `TowerBuild { levels, tower_snapshots, cascade_reset, did_extend, forest_dirty, forest_dirty_l0 }`
- `PoppedFrontier { upper, cp, had_window, resume_start }`
- `LevelScan { prefix_count, lifecycle_scan_from(内部消化) }`
- `NextLevelInputs { units, anchors }`

---

## 3. commit SHA 列表（逐个可编译 + 逐个 p123 对拍）

| 批 | SHA | 内容 | lib 编译 | p123 20k |
|---|-----|------|---|---|
| 1 | `6af5314a79` | 序幕分解：L0 侧五步抽函数（T1–T5） | ✅ | 零 diff |
| 2 | `891d24dd27` | cascade 失效段分解（9 项） | ✅ | 零 diff |
| 3 | `be75dc590c` | 扫描/继承/水线段分解（8 项） | ✅ | 零 diff |
| 4 | `cd8dd00430` | BSP/装配/投影段分解（6 fn + 2 载体） | ✅ | 零 diff |
| 5 | `4f2ab99f4f` | 主循环外提 `build_level_tower` + 尾部收口 | ✅ | 零 diff |
| 6 | `21cf2dd442` | 收敛至全函数 ≤50（`TowerLoop` + 循环骨架三分） | ✅ | 零 diff |
| 7 | `e9aa2e727a` | `incremental.rs` 按域拆为目录模块（5 文件 ≤800） | ✅ | 零 diff |
| 8 | `df18cbc4b9` | 存量超限函数·生产面 4 处 | ✅ | 零 diff |
| 9 | `477284668c` | 存量超限函数·测试/profile 面 6 处 | ✅ | 零 diff |

具名 `git add`（无 `-A`/`.`），无任何 git mutation（无 push/reset/rebase/checkout/restore/stash/amend/branch/clean）。

---

## 4. 分解映射

### 4.1 `classify_with_tower_incremental` 765 → 45（37 个函数，全部 ≤50）

基线 `797c9ad35c:incremental.rs` = 820 行单文件（765 行函数 + 34 行函数文档 + 21 行头/use/static）。
收口 = `incremental/` 目录模块 5 文件、1273 行、37 个函数，**最大 49 行**。

| 新文件 | 行 | 函数 | 最大 | 承载 |
|---|---|---|---|---|
| `mod.rs` | 367 | 9 | 45 | 模块头/use/两开关 static + `L0Prelude` 族（`reset_cache_on_segment_shrink` / `build_l0_units_cache` / `diag_l0_units_parity` / `l0_tower_tail_changed` / `rebuild_l0_tower` / `prepare_l0_inputs`）+ 入口 `classify_with_tower_incremental` + `TowerBuild` + `bump_tower_epochs` + `debug_assert_l0_units_in_sync` |
| `tower.rs` | 315 | 8 | 49 | `TowerCarry` / `TowerLoop` / `LevelCtx` + `new_tower_loop` / `tower_loop_into_build` / 两个 `truncate_levels_on_*_break` + `build_level_tower`(32) → `process_level`(31) → `scan_and_extend_level`(49) / `assemble_level_state`(38) |
| `invalidate.rs` | 212 | 9 | 23 | `apply_frontier_invalidation` / `local_dirty_source` / `invalidate_level_cache` / `clear_level_cache` / `retain_level_prefix` / `rebuild_cursor_for_retained_prefix` / `second_cache_anchors` / `truncate_second_cache` / `cascade_eprobe_on` |
| `assemble.rs` | 185 | 4 | 46 | `LevelSeries` / `LevelBspInputs` + `extract_level_bsp`(46) / `extract_first_third_tail` / `project_next_level_units` / `derive_units_anchors` |
| `scan.rs` | 194 | 7 | 42 | `sync_cached_units` / `PoppedFrontier` + `pop_frontier_window` / `reinherit_cp_lifecycles`(42) / `extend_level_tail` / `advance_confirmed_watermark` / 两条 `debug_assert_*` |

原 765 行函数的**三层结构**：

```
classify_with_tower_incremental (45)
├── prepare_l0_inputs (25) ─┬ reset_cache_on_segment_shrink (5)     T1
│                           ├ build_l0_units_cache (9)              T2
│                           ├ diag_l0_units_parity (16)
│                           ├ l0_tower_tail_changed (12)            T4 读
│                           └ rebuild_l0_tower (12)                 T3/T4 写
├── [不拆] 四缓冲 mem::take + MACD 增量 + stable_len              T6–T9
├── build_level_tower (32)
│   └── process_level (31)
│       ├── apply_frontier_invalidation (23) ─┬ local_dirty_source (16)
│       │                                     └ invalidate_level_cache (19)
│       │                                        ├ clear_level_cache (16)
│       │                                        └ retain_level_prefix (22)
│       │                                           ├ rebuild_cursor_for_retained_prefix (14)
│       │                                           └ second_cache_anchors (11) + truncate_second_cache (8)
│       ├── sync_cached_units (12)
│       ├── scan_and_extend_level (49) ─┬ pop_frontier_window (29)
│       │                               ├ reinherit_cp_lifecycles (42)
│       │                               ├ extend_level_tail (14)
│       │                               ├ advance_confirmed_watermark (11)
│       │                               └ 两条 debug_assert_* (8/7)
│       ├── assemble_level_state (38) ──┬ extract_level_bsp (46) → extract_first_third_tail (29)
│       │                               └ build_level_projection (16, 已上提 pipeline.rs)
│       └── project_next_level_units (19) + derive_units_anchors (5)
└── [不拆] 四缓冲放回 → bump_tower_epochs (16) → debug_assert_l0_units_in_sync (10)   T10
```

### 4.2 可见性口径（保全，非放宽）

跨文件调用项标 `pub(super)` —— 在 `incremental/*` 子文件里等于「对 `incremental` 及其后代可见」，
与拆分前「`incremental` 模块内私有」的可见域**逐字等价**（不外泄到 `classifier`）。
共标：`invalidate.rs` 1 fn、`scan.rs` 8 项（含 `PoppedFrontier` 4 字段）、
`assemble.rs` 6 项（含 `LevelSeries` 6 字段 + `LevelBspInputs` 5 字段）、`tower.rs` 1 fn。
`mod.rs` 内的项（`L0Prelude` / `TowerBuild` / 序幕族）保持私有——私有项对后代模块本就可见。

对外 API 面零变化：`pub fn classify_with_tower_incremental` 仍由 `classifier/mod.rs` 的
`pub use incremental::classify_with_tower_incremental;` 原路径导出，`mod incremental;` 声明一字未改。

---

## 5. 存量 10 处超限函数逐个处置

`#576` §4 全列 11 处（含本票主目标 `classify_with_tower_incremental`），**全部随本票分解，零豁免**。

| # | 函数 | 基线 | 收口 | 处置 |
|---|---|---|---|---|
| 1 | `incremental.rs` `classify_with_tower_incremental` | 765 | **45** | 主目标，§4.1 |
| 2 | `pipeline.rs` `classify_impl` | 141 | **50** | 抽 `full_l0_inputs` / `full_macd_series`(+`FullSeries`) / `compose_full_level` / `extract_full_level_bsp` |
| 3 | `tower_cache.rs` `compute_macd_hist_incremental` | 75 | **38** | 抽 `macd_boundary_shortcut`（空/单 bar 边界）/ `macd_resume_state`（state 复用三分支） |
| 4 | `cand_delta.rs` `cand_delta_tower_with_series` | 54 | **44** | 抽 `level_ge1_cand_inputs`（L≥1 投影→units→segs/anchors） |
| 5 | `tests/level_signals.rs` `type1_funnel_census_btc` | 164 | **38** | 抽 `census_btc_dataset`（与 #9 共享）/ `report_window_span_histogram` / `center_chain_stats`(+`CenterChainStats`) / `funnel_probe_series` / `funnel_level_geometry` / `report_funnel_line` / `report_top_region_sell1` |
| 6 | `tests/incremental_tower.rs` `incremental_tower_scaling_dominates_full_synthetic` | 68 | **41** | 抽 `time_full_vs_incremental_accumulation` |
| 7 | `tests/incremental_tower.rs` `cascade_reset_on_frontier_interior_rewrite` | 66 | **27** | 抽 `frontier_interior_rewrite_fixture` / `assert_frontier_rewrite_invisible_to_l1` |
| 8 | `tests/level_signals.rs` `end_to_end_second_buy_via_l1_l2_geometric` | 56 | **22** | 抽 `l1_l2_geometric_fixture`（9 段推导注释随夹具迁为其 doc） |
| 9 | `tests/level_signals.rs` `level_signal_census_btc` | 55 | **24** | 共享 `census_btc_dataset` + 抽 `report_level_ge1_endpoint_samples` |
| 10 | `tests/cache_and_units.rs` `l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink` | 51 | **30** | 抽 `segment_ledger_shrink_fixture` |
| 11 | `incremental_profile.rs` `profile_incremental_tower_real_scaling` | 61 | **27** | 抽 `time_real_bar_accumulation` / `report_real_scaling_exponents` |

**并行车撞面登记**：本票执行期间 #630（`level_view*`）/ #631（`signal.rs`/`bsp.rs`/`runner.rs`）/
#635（`retrace_ledger/book.rs`）在同一 worktree 上作业，但 §4 全列 11 处**无一落在这三车的面上**
（全部在 `incremental*` / `pipeline.rs` / `tower_cache.rs` / `cand_delta.rs` / `tests/`），故零跳过。

**顺带去重（超出票面范围，已单独登记）**：`build_level_projection` 原本在 `pipeline.rs`（全量路径）
与 `incremental` 内各有一份**逐字同构**的拷贝。批 8 把它上提到 `pipeline.rs`（`LevelState` 的定义处）
作为两条路径的单一来源——这是零行为变化的去重，消除了「#110 投影层两处同口径」的漂移面。

---

## 6. 机械证据

### 6.1 失败集 + 测试数（真基线：同一他人面下回填基线文件实测）

基线不是 `#576` 报告里的旧数（并行车已把他人面推进），故**在收口后把基线版本 `incremental.rs`
回填到工作区重跑一遍**（纯文件 `cp`，非 git mutation），取得同他人面下的真基线：

| 项 | 真基线 | 收口 |
|---|---|---|
| `cargo test --release --lib` | **2205 passed / 0 failed / 138 ignored** | **2205 passed / 0 failed / 138 ignored** |
| 失败集 | **空** | **空**（逐字一致：两侧均无 FAILED 行） |
| 测试名集合（`--list`，2343 项） | — | `diff` **IDENTICAL** |
| lib 警告 | **38** | **38** |
| `incremental*` 面警告 | 1（`unused_mut` on `p`） | 1（同一条，位置随文件迁移） |

测试数不减、名集合逐名一致 ⟹ 无测试被删/改名/漏跑。

### 6.2 p123 20k 对拍

`P116_MAX_BARS=20001 ./target/release/p123_fast_replay analysis/data_cache/btc_1m_full.json`

- 开工基线 sha1 = `738cb880eb2c3437c3920d4811913e14fc5663ef`（与 #576 报告同值）；
- 同他人面回填基线 sha1 = **同值**（证明并行车改动未污染本对拍面）；
- **9 个批次逐个 `diff` 零**，收口 sha1 = 同值。EXIT=0，16 行门行逐字相同。

### 6.3 纯移动核验（去空白规范化差集）

把基线与收口的对应文件做「去首尾空白 + 丢空行 + 代码行/注释行分离」的多重集差：

| 组 | 代码行 base→final | 仅基线独有 | 仅收口独有 | 非结构候选（人工逐条核） |
|---|---|---|---|---|
| incremental | 447→710 | 123 | 386 | 全落 6 类登记变换 |
| pipeline | 175→225 | 24 | 74 | 同上 |
| tower_cache | 211→229 | 7 | 25 | 同上 |
| cand_delta | 171→179 | 0 | 8 | 同上 |
| tests+profile | 782→850 | 55 | 123 | 同上 |

差集的**登记变换 6 类**（穷举，无第七类；零条新逻辑行）：

1. **函数边界行**：新增 `fn`/`}`/参数行/返回类型行、`struct` 定义与字段行；
2. **调用点替换**：原内联语句块 → 一行调用（含 `let x = f(...)` 与 `f(...)` 两型）；
3. **路径改名**（同一值、不同持有者）：局部量 → 参数/字段路径——
   `units` → `st.units` / `inputs.units`、`cascade_reset` → `carry.cascade_reset` / `*cascade_reset`、
   `hist`/`dif`/`closes_tick`/`close_src` → `series.*`、`n_up`/`lock_desc` → `chain.*`、
   `l0`/`config` → `ctx.*` / `inputs.*`；
4. **可见性/模块声明**：`pub(super)` 标注、`mod`/`use` 行；
5. **等价改写**（4 处，逐条列名）：
   - `diag_l0_units_parity`：`if env.is_ok() { … }` → `if env.is_err() { return; }`（early return）
   - `l0_tower_tail_changed`：`if … { true } else { … }` → `if … { return true; } …`（early return）
   - `apply_frontier_invalidation`：两个**同条件**相邻 `if`（置 `cascade_reset` / 算 `dirty_e`）合并为一（中间无副作用）
   - `cascade_eprobe_on`：循环外局部量 `eprobe_on` → 读同一 `OnceLock` 的函数（`get_or_init` 幂等，逐值等价）
6. **slice 参数化**：`*l0_units != full` → `l0_units != full.as_slice()`（`&Rc<Vec<T>>` 参数改 `&[T]`）。

**零条可执行语义行**（判据/阈值/算子/断言消息）出现在差集中：全部 `debug_assert!` 的条件与消息、
所有 `oracle_probe::*` 探针的门与实参、所有 `stage_profile::time` 标签串均逐字保留。

### 6.4 Standards 轴

| 项 | 结果 |
|---|---|
| 函数 ≤50 | ✅ 本票面 **零超限**（`incremental/` 37 个函数最大 49；`pipeline`/`tower_cache`/`cand_delta`/`tests`/`incremental_profile` 全部 ≤50） |
| 文件 ≤800 | ✅ `incremental/` 5 文件 185–367；`pipeline.rs` 411、`tower_cache.rs` 605、`cand_delta.rs` 237、`tests/level_signals.rs` 572、`tests/incremental_tower.rs` 415、`tests/cache_and_units.rs` 157、`incremental_profile.rs` 84 |
| 零新增警告 | ✅ 38 → 38，我面 1 条既有 `unused_mut`（`invalidate_level_cache` 的 `p`，仅 `cfg(test)` 分支重赋；就地加注说明为何不清理） |
| 无 mutation | ✅ 仅 commit（9 次），具名 add |

---

## 7. 偏离 / 存疑

1. **两处 `#[allow]` 是新增的**（登记）：`PoppedFrontier::had_window` 与
   `TowerBuild::forest_dirty_l0` 加 `#[allow(dead_code)]`——两者的唯一读点都在 `#[cfg(test)]`
   探针块内，release 构建下该块剥离 ⟹ 字段未读。它们在原码里是循环内的局部量（局部量无
   dead_code 检查），抽为结构体字段后才暴露；语义与读点均未变。不加 allow 会**新增**警告，
   与「零新增警告」冲突；改成 `#[cfg(test)]` 字段则要让结构体在两种 cfg 下形状不同，更差。
2. **`invalidate_level_cache` 的 `unused_mut` 警告被保留而非清理**：它是基线既有条目。清掉会让
   警告集从 38 变 37，破坏「警告集逐字一致」这条机械证据；清理它属于另一票（本票口径是零行为变化）。
3. **文件从 820 涨到 1273（+55%）后才拆目录**：涨幅来自函数签名/结构体定义/doc 展开，不是逻辑增加
   （§6.3 代码行 447→710 里，386 条收口独有全在结构与路径改名两类）。批 7 拆文件是为不让本票在
   改善「函数 ≤50」的同时恶化「文件 ≤800」——若不拆，本票会留下一个比修复前更严重的 Standards 缺口。
4. **`build_level_projection` 去重超出票面字面范围**：票面只说函数分解。但 `classify_impl` 要压到
   ≤50 就必须把那 10 行 `if config.level_projection.enabled { … }` 抽走，而 `incremental` 侧已有
   逐字同构的一份——再写第三份是明知故犯的重复。合并为单一来源是零行为变化（两份逐字相同），
   已在 §5 单列登记。
5. **p123 20k 的有效域不是全窗**（`formalization-validity-domain`）：20k bar 覆盖不到的分支，
   字节证据不覆盖。旁证是 2205 个单测 + `--lib --no-run` 全目标构建 + 测试名集合 IDENTICAL。
   本票未跑 `cargo test --release` 全量执行（含集成测试），理由同 #576 §6.5：基线取的是 `--lib`，
   全量执行无可比基线；本票 diff 面在 `classifier` 内部，对外路径零改写。
6. **基线是「他人未提交态 + 并行车推进态」**：开工时 lib 曾两度不可编译（#631 写 `bsp.rs` 的
   `level_origin`、#630 拆 `level_view.rs` 的中途态），本票在其间等待而非绕过。§6.1 的真基线用
   回填法在**同一他人面**下取得，消除了 #576 §6.4 登记的「基线与收口不是同一个他人面快照」的缺陷。
7. **`scan_and_extend_level` 49 行贴着上限**：它是本票分层的自然边界（frontier pop → compose →
   继承 → extend → cursor/水线，五步之间有严格次序耦合，再切会把 T11 的 `moves_tower` 读窗口
   拆到两个栈帧）。若日后该函数再长，正确的动作是把 compose 与 extend 之间的判据（`did_extend` /
   `forest_dirty`）连同其推导一起外提，而不是机械切半。

---

## 8. 结果包六要素

1. **结论**：`classify_with_tower_incremental` 765 → 45 行（37 个函数，最大 49，全部 ≤50），
   `incremental.rs` 拆为 5 文件目录模块（185–367 行）；`#576` §4 全列 11 处超限函数全部清零，零豁免。
   零行为变化：2205 passed/0 failed、测试名集合 IDENTICAL、p123 20k 逐字节相同、警告集 38→38。
2. **定义依据**：`#576`/`#359` 的机械证据口径（规范化纯移动差集 + 测试名集合 IDENTICAL + 对拍指纹）；
   Rust 可见性规则「私有项对定义模块及其后代可见」——`pub(super)` 是该可见域在子模块内的等价表达；
   Rust 字段不相交借用（disjoint field borrow）——`&mut cache.levels` 与 `&cache.macd_hist` 可共存，
   这是循环外提为函数的可行性前提（§1.2）。
3. **边界条件（结论会翻转的条件）**：
   - 若某个抽出函数改变了 `Rc::make_mut` 处的 `strong_count`（例如把某个 `Rc` 多存活一个作用域）
     ⟹ 原地 O(tail) 退化为写时复制，标度回退（bit-exact 仍成立但性能证据失效）。实测：全部
     `Rc` 携带量仍是同一栈帧的局部量，drop 时点未变（T2/T3/T11/T12）。
   - 若 p123 20k 覆盖不到某条被移动的分支 ⟹ 字节证据的有效域不覆盖该分支（§7.5）。
   - 若并行车在真基线与收口之间改动了**我面之外但被我面调用**的代码 ⟹ 对拍可比性下降。实测
     两次基线（开工 / 同他人面回填）p123 sha1 相同，说明该窗内并行车改动未触及本对拍面。
   - 若 `pub(super)` 实际放宽了可见域 ⟹「可见性保全」不成立。实测：`pub(super)` 的语法上界就是
     `incremental` 子树，编译器强制，不可能外泄到 `classifier`。
4. **下游推论**：`classifier::classify_with_tower_incremental` 的对外路径与签名不变 ⟹ 30+ 个消费点
   零改动；增量塔的每一步失效/扫描/装配现在有独立函数面与独立 doc，后续对 cascade 失效边界
   （on2w2 §3）或 07a/07b resume 的修改可单函数评审、单函数对拍，不必再读 765 行上下文。
   `build_level_projection` 单一来源后，#110 投影层的口径漂移面从 2 处降到 1 处。
5. **谱系引用**：`no-patch-mentality` 第 6 条（妥协方案）→ `#576` 当时拒绝「为凑行数切 640+145」是
   正确的，本票走的是它指出的「设计级分解」路线，且先证次序再拆；275 局部依赖 → 并行车的
   `level_view`/`signal`/`retrace_ledger` 面与本票无数据依赖，等待仅发生在「全仓可编译性」这一条
   真实的直接依赖上（验证链消费它），设计与实施未因此停摆；
   `formalization-validity-domain` → §7.5 明确 p123 20k 的有效域不是全窗；
   `coding-style` 不可变性注记（2026-07-27/07-28 编排者裁定）→ `TowerCache`/`TowerLoop` 的就地
   推进是性能累加器与单线程状态机，不适用不可变要求，本票只改结构不改这一性质。
6. **影响声明**：改动限于 `rust/src/theta_v0/classifier/` 下 12 个文件——
   删 1（`incremental.rs`）、新建 5（`incremental/{mod,tower,invalidate,scan,assemble}.rs`）、
   改 6（`pipeline.rs` / `tower_cache.rs` / `cand_delta.rs` / `incremental_profile.rs` /
   `tests/{level_signals,incremental_tower,cache_and_units}.rs`）。未碰 `classifier/mod.rs`
   （`mod incremental;` 声明形式使目录化零改动），未碰 `level_view*` / `signal.rs` / `bsp.rs` /
   `retrace_ledger/` / `nest_lifecycle.rs` 等并行车面，未改动仓外任何文件，未改任何对外 API。

---

## 修订补记（2026-07-29，影子评审 MEDIUM-1 触发，编排侧落）

**§6.3「登记变换 6 类穷举」补第 7 类 + 「逐字保留」收窄**：

- **第 7 类：格式串改写**（6 条，全部位于 #[ignore] 测试/profile 面）：`[census]`/`[funnel]` → `[{tag}]`、位置实参 → 内联具名、`format!` → `to_string`。影子评审逐条核过输出等价、行为零影响。
- 「所有串逐字保留」收窄为：**生产面 + assert/探针/stage 标签逐字保留**；字符串字面量多重集实测 base 独有 12 / 收口独有 6（即上述第 7 类），不再声称全量逐字。
- 影子评审 `shadow-633-review-20260729.md` MEDIUM-1 在案；另 MEDIUM-2（报告 §7.5 全量集成测试二进制缺口）已由影子**加强复现**（41 个集成测试二进制两树逐项相同）关闭。
