# #633 影子评审报告：`classify_with_tower_incremental` 765 行函数分解 + 存量超限函数同批

- 日期：2026-07-29
- 评审车：claude opus 影子评审（**未参与 #633 实装**，新上下文独立复现）
- 工位：`/tmp/kimi-nest-mainline` @ `7b589a7347`（分支 `kimi-nest-mainline-20260717`）
- 约束遵守：单线程（零子代理、零后台任务）；只读源码 + 独立影子树跑测试；**零 git mutation**；
  未碰并行线未提交面（`rust/src/bin/p100_cert_bsp_recon.rs` 等）、未碰 `retrace_ledger/`、未碰 `level_view*`
- 复现基座：
  - 基线树 `/tmp/shadow633/base` = `git archive HEAD` + 把 #633 触碰的 8 个源文件回填到 `d9f1860124`
    （= 批1 `6af5314a79^`）、删除 `incremental/` 目录 —— **同他人面**（#630/#635/#631 的改动全部保留）
  - 收口树 `/tmp/shadow633/head` = `git archive HEAD`（纯 HEAD，剔除工作区脏文件）
  - 两树各自独立 `CARGO_TARGET_DIR`，互不污染，也未触碰工位的 `rust/target`
- 基线合法性自核：`d9f1860124:incremental.rs` 与实施报告声称的基线 `797c9ad35c:incremental.rs`
  blob 同为 `16a6b5221a0d419329b2fe4381196bfc645da3c2`；且 `git log d9f1860124..HEAD -- <8 个文件>`
  **只有 #633 的 9 个 commit**，无并行车混入 ⟹ 回填基线是干净的单变量对照。

---

## 结论（先行）

**通过。零 HIGH。** 票面三项范围全部交付，机械证据我这边**全部独立复现且比实施报告更强**：

| 复现项 | 实施报告声称 | 我实测 | 判 |
|---|---|---|---|
| 失败集 / 测试数 | 2205 / 0 / 138（release） | **两树均 2205 passed / 0 failed / 138 ignored**（debug，`debug_assert!` 全开，比 release 更严） | ✅ 且更强 |
| 测试名集合 | 2343 项 `diff` IDENTICAL | **两树各 2343 项，`diff` 空** | ✅ |
| p123 20k 对拍 | sha1 `738cb880eb…` | **两树 stdout sha1 均 `738cb880eb2c3437c3920d4811913e14fc5663ef`，stdout/stderr `diff` 均空，EXIT=0** | ✅ |
| 警告集 | 38 → 38 | **两树 `cargo build --lib` 逐条同集，唯一差异是路径 `incremental.rs` → `incremental/invalidate.rs`** | ✅（数字口径见 LOW-1） |
| 函数 ≤50 | 本票面零超限 | **收口面独立测量零超限，最长 49（`scan_and_extend_level`）；基线 11 处超限逐一复现，行数 765/141/75/54/61/56/55/164/66/68/51 与报告 §5 表逐个吻合** | ✅ |
| 文件 ≤800 | 5 文件 185–367 | **`incremental/` 5 文件 184/193/211/314/366；本票全部触碰文件最大 604（`tower_cache.rs`）** | ✅ |
| `#[allow(dead_code)]` ×2 必要性 | 不加会新增警告 | **剥掉两处 attribute 后重编，两条 `never read` 警告实测触发**（`mod.rs:308` / `scan.rs:38`） | ✅ 必要 |
| `build_level_projection` 合一 | 两份逐字同构 | **基线 `incremental.rs:689-698` 与 `pipeline.rs:285-294` 逐字相同（含实参 `levels.len() as u32` / `&bsp` / `&l0.fractals` / `&l0.merged_bars`）** | ✅ |
| 禁 git mutation | 仅 commit ×9 | **`git reflog -n 25` 全为 `commit:`，无 reset/checkout/rebase/amend；`git stash list` 无今日条目** | ✅ |

**额外关闭了报告自登记的一条缺口**：报告 §7.5 说「本票未跑全量执行（含集成测试），无可比基线」。
我在同他人面下对**两树跑了 `cargo test --no-fail-fast`（lib + 全部 41 个集成测试二进制）**，
41 行 `test result` 逐行 `diff` **只差两处耗时秒数**（`6.26s`/`6.43s`、`0.05s`/`0.04s`），
计数逐项相同 ⟹ **集成测试面同样零行为变化**，该登记项可关。

---

## 1. 次序保持（HIGH 门槛）——逐段对照结论：**零次序疑点**

方法：把基线 765 行函数按可观察副作用逐语句编号，再沿收口调用链（`mod.rs` → `tower.rs` →
`invalidate.rs`/`scan.rs`/`assemble.rs`）展开，逐点对齐。

### 1.1 L0 序幕（T1–T5）

| 基线行 | 动作 | 收口位置 | 判 |
|---|---|---|---|
| 80–82 | 回缩 `cache.clear()` | `mod.rs:161` `reset_cache_on_segment_shrink` | ✅ 第一位 |
| 87–95 | `00_l0_units_build`（`Rc::make_mut`） | `mod.rs:162` `build_l0_units_cache` | ✅ 在 clear 之后 |
| 98–99 | `00b` `Rc::clone` 出借 | `mod.rs:165-166` | ✅ 在 `make_mut` 之后（T2 保持） |
| 102–114 | `DIAG_L0UNITS` 对拍 | `mod.rs:167` `diag_l0_units_parity` | ✅ |
| 117–120 | 空 L0 → `clear()` + 早返回 | `mod.rs:170-173` 返回 `None` + `mod.rs:255-258` 早返回 | ✅ 在两个水线写入之前（T5 保持） |
| 122/124 | `last_l0_segments_len` / `l0_confirmed_len` 落账 | `mod.rs:175-177` | ✅ |
| 141–153 | `forest_dirty_l0` 读**旧** `moves_tower_l0` | `mod.rs:179` `l0_tower_tail_changed`（只读） | ✅ **在重建之前**（T4 保持——这是本票最要命的一条） |
| 154–163 | `01_l0_tower_rebuild`（`Rc::make_mut`） | `mod.rs:180` `rebuild_l0_tower` | ✅ |
| 164 | `Rc::clone(&cache.moves_tower_l0)` | `mod.rs:181` | ✅ 在 `make_mut` 之后（T3 保持） |

### 1.2 逐级循环体（基线 244–767 → `process_level`）

基线 38 个可观察步逐一对齐收口，**次序完全一致**，摘关键点：

- 03 `frontier_compare` → 置 `cascade_reset` → 取 `dirty_e.min(local_e)` → `if cascade_reset { forest_dirty=true; 失效 }`
  → **`lc.last_input_len = units.len()`** → 04 `cached_units_copy`。
  `apply_frontier_invalidation` 读的是**旧** `lc.last_input_len`（`invalidate.rs:51`），写在
  `tower.rs:176`——两者次序若颠倒会让 len-shrink 判据永假，**收口保持了正确次序**。
- `retain_level_prefix`（P>0 支）内部：`wm` 读取 + cursor 重建 + O2 哨兵 `debug_assert`
  → `second_cache_anchors`（**在 `upper_moves.truncate(p)` 之前**读 `upper_moves[b2_count-1].end_index`）
  → 四个 `truncate(p)` → 水线 `min(p)` → `decompose_state.reset()` → bsp/pan 清 → `truncate_second_cache`
  → `projected_units.truncate(p)`。与基线 348–401 **逐句同序**。
- `pop_frontier_window`：`resume_start` 与 `had_window` 在 pop **之前**读；`upper = um[keep..].to_vec()`
  在 `um.truncate(keep)` **之前**捕获。与基线 432–461 同序。
- `prefix_count = lc.upper_moves.len()` 在 pop **之后**取（`tower.rs:228`），与基线 465 同。
- T11：`tower_snapshots.push(mem::take(&mut st.moves_tower))`（`tower.rs:186`）在
  `advance_cp_lifecycles`（本级对 `moves_tower` 的最后一次读，`tower.rs:255`）**之后**、
  在 `assemble_level_state`（`tower.rs:188`）**之前**。与基线 585 位置同。
- T13：`st.levels.push(level)` 在读 `st.levels.last().moves`（`tower.rs:194`）之前。与基线 699→726/746 同。
- `level_ordinal` 取 `st.levels.len()`（`tower.rs:189`，push 前）＝基线 `levels.len() as u32`（push 前）。
- `forest_dirty |= tail_upper != popped.upper` 在 `extend_level_tail` **消费** `tail_upper` 之前。

### 1.3 尾部（T10）

缓冲放回四行 → `bump_tower_epochs` → `debug_assert_l0_units_in_sync`，与基线 769–817 同序。
`bump_tower_epochs` 内 `generation` bump 在 `forest_epoch` bump 之前、`on_forest_dirty` 探针在最后，
与基线 787/796/800 同序。

### 1.4 探针与标签的次数/位置

- `oracle_probe::*` 五个探针，两树各出现 1 次，且门条件逐字相同（`level_idx < levels_cache.len()` /
  `level_idx + 1 < levels_cache.len()` / `p as f64 / lc.centers.len() as f64` / `tail_upper.len()` /
  三元 `on_forest_dirty` 实参）。
- `stage_profile::time("…")` 标签 16 条，两树排序后 `diff` **空**，且各自所在的分支（`07c` 在 memo 命中支、
  `07a_*` 在 L0/L≥1 双支）未变 ⟹ 计时分桶零漂移。

**结论：次序保持这一 HIGH 门槛通过，无任何降级项。**

---

## 2. 纯移动核验（去空白/去注释规范化差集）

### 2.1 代码行多重集

| 组 | base→final | 仅基线独有 | 仅收口独有 | 人工逐条核 |
|---|---|---|---|---|
| incremental（1→5 文件） | 443→706 | 121 | 384 | 全落结构/路径/调用点/可见性/等价改写/slice 参数化 |
| pipeline | 175→225 | 23 | 73 | 同上 |
| tower_cache | 211→229 | 7 | 25 | 同上 |
| cand_delta | 169→177 | **0** | 8 | 纯抽函数 |
| tests + profile | 768→836 | 52 | 120 | 同上 + 见 MEDIUM-1 |

（数字与实施报告 §6.3 略有 ±2 差异，是注释/空行归一口径不同所致，不构成分歧。）

**零条判据 / 阈值 / 算子出现在差集里**——我逐条核过所有含比较运算符、数值常量、`assert` 的行：
`< min_parts`、`< lc.last_input_len`、`consumed + 2`、`partition_point(|w| w.read_end_src < dirty_e)`、
`b.source_index <= cut`、`ratio_at_max < 0.7`、`saturating_sub(pop_n)`、`last_window_emitted.max(1)`
等全部逐字保留在收口。

### 2.2 字符串字面量多重集（比报告更严的一层）

对 8 个基线文件 vs 12 个收口文件抽全部字符串字面量做多重集差（135 vs 128）：

- **incremental 组单独看**：仅基线独有 2 条，都是 `"本级 LevelState 已 push"`——基线在
  726/736/746 三处各写一次，收口把 `parent_blocks` 提为一个绑定 ⟹ 只剩 1 次。**同一条消息，非改写**。
  其余（16 个 `stage_profile` 标签、全部 `debug_assert!` 消息、`DIAG_L0UNITS` / `THETA_CASCADE_EPROBE` /
  `THETA_CASCADE_FULLCLEAR` 三个 env 名、`[DIAG-L0UNITS]` 诊断格式串）**逐字节相同**。
- **测试/profile 组**：6 条格式串被参数化改写 —— 见 **MEDIUM-1**。

### 2.3 注释

1030 条基线注释行里 1030−101=929 条的归一化文本在收口语料中原样可寻；101 条被**改写**
（加 `（T1）` 之类的次序标注、`函数末 debug_assert` → `[`classify_with_tower_incremental`]` doc link、
半角 `:` → 全角 `：`、跨行重排后措辞微调）。逐条抽查未见语义丢失。另有 6 处仓内行号交叉引用被删除
——见 **LOW-3**。

---

## 3. 存量 10 处抽验（抽 3）

| # | 函数 | 抽验结论 |
|---|---|---|
| 2 | `pipeline.rs::classify_impl` 141→50 | 逐句对齐 `full_l0_inputs` / `full_macd_series` / `compose_full_level` / `extract_full_level_bsp` / `build_level_projection`。**唯一次序位移**：`units_anchors = Vec::new()` 从「空 L0 判据之前」挪到「之后」——`Vec::new()` 无副作用、早返回路径不可观察，等价。`debug_assert_eq!(centers, centers_w, "…")` 消息与 `advance_cp_lifecycles(…, 1)` 的常数 `1` 逐字保留 |
| 3 | `tower_cache.rs::compute_macd_hist_incremental` 75→38 | `macd_boundary_shortcut` 把两个 `return;` 提升为 `-> bool` 的 `return true` + 尾 `false`，调用方 `if …{ return; }`——三分支覆盖与副作用逐条同；`macd_resume_state` 的三支（truncate 复用 / clear+rebuild / 全量）与基线 `if/else` 结构逐句同；`resume_from = macd_state_len.min(confirmed_len).min(stable_prefix)` 与尾部续推循环逐字未动 |
| 6 | `tests/incremental_tower.rs::incremental_tower_scaling_dominates_full_synthetic` 68→41 | `synthetic_segments(n)` / `closes` 仍在 `t0` **之前**构造（计时边界未移）；`layer_at` 闭包每次迭代构造的 `ParseLayer` 与基线内联字面量逐字段同；阈值 `0.7`、断言消息、`eprintln!` 格式串逐字保留 |

另：#5 `type1_funnel_census_btc` 164→38 的共享夹具 `census_btc_dataset(tag, cfg)` 的 tag 实参
（`"census"` / `"funnel"`）与基线两处各自的前缀一一对应，诊断输出等价（见 MEDIUM-1）。

---

## 4. 发现清单

### MEDIUM-1｜实施报告 §6.3「登记变换 6 类穷举、无第七类」与「所有串逐字保留」两句超出实测

`chanlun/review-results/issue633-incremental-fn-decomposition-20260729.md:272,288`

差集里实际存在**第 7 类未登记变换：格式串/构造式改写**，全在测试/profile 面：

1. `tests/level_signals.rs`：`eprintln!("[census] window={s}..{e}")` / `("[funnel] …")`
   → 共享夹具内 `eprintln!("[{tag}] window={s}..{e}")` + 调用点传 `"census"` / `"funnel"`
   （3 组格式串 ×2 = 6 条字面量被改写）；
2. `incremental_profile.rs:54` → `:39`：`eprintln!("n={n} done: full={:.2}s inc={:.2}s", *full_times.last().unwrap(), *inc_times.last().unwrap())`
   → `eprintln!("n={n} done: full={t_full:.2}s inc={t_inc:.2}s")`（位置实参 → 内联具名实参）；
3. `tests/level_signals.rs`：2 处 `format!("…")` → `"…".to_string()`。

**行为影响：零**——我逐条核过三类的输出等价（tag 代换后前缀逐字相同；`t_full`/`t_inc` 就是同轮
push 进 `full_times`/`inc_times` 的那两个值；`format!` 与 `to_string` 同产 `String`），
且这三处全在 `#[ignore]` 的诊断/profile 测试里，不进 2205 的执行面。

**问题在证据陈述**：报告同时写了「**穷举**，无第七类」和「所有 `debug_assert!` 的条件与消息、所有
`oracle_probe::*` 探针的门与实参、所有 `stage_profile::time` 标签串均**逐字保留**」。前两类
（assert / 探针 / stage 标签）我实测确实逐字保留；但字符串字面量多重集在测试面上是
**base 独有 12 条 / 收口独有 6 条**，不是逐字保留。按 `no-patch-mentality` 第 5 条（声明膨胀），
应把第 7 类补登记，并把「逐字保留」的作用域收窄到「生产面 + assert/探针/stage 标签」。

**翻转条件**：若这三类改写中任一条的输出不等价（例如 tag 传反、`t_full`/`t_inc` 绑错轮次），
则从证据陈述问题升级为行为问题。我已逐条排除。

### MEDIUM-2｜§7.5 登记的「集成测试无可比基线」不成立，且已被我实测推翻

`chanlun/review-results/issue633-incremental-fn-decomposition-20260729.md:320-321`

报告以「基线取的是 `--lib`，全量执行无可比基线」为由不跑集成测试。但**同一份回填基线法**
对集成测试完全适用，成本 24 秒：我在两树各跑 `cargo test --no-fail-fast`，41 个测试二进制的
`test result` 行逐行 `diff` 仅差两处耗时数字，计数全同。

这不是行为缺陷（结果是绿的），但**理由是错的**——把可推进的证据面登记为不可推进。
按 `no-unnecessary-escalation` 格式B 的 `blocked` 条款（652 号），登记不可推进须证明依赖关系；
此处不存在这个依赖。建议把 §7.5 的登记改为「已补跑，两树 41 个测试二进制逐项相同」。

### LOW-1｜「lib 警告 38 → 38」的数字在任何标准 cargo 命令下复现不出

`chanlun/review-results/issue633-incremental-fn-decomposition-20260729.md:248,297`

实测基线树：`cargo build --lib` = 37、`cargo build --release --lib` = 37、`cargo check --lib` = 37、
`cargo test --lib --no-run` = 53。**没有一个是 38**。最可能的口径是
`… | grep -c '^warning'`——37 条正文 + 1 行 `` `newchan_rust` (lib) generated 37 warnings `` 汇总 = 38。

**不变量本身成立**：两树警告集逐条同（我按「警告文案 + 源文件」双维度 diff），
唯一差异是那条既有 `unused_mut` 的位置从 `incremental.rs` 迁到 `incremental/invalidate.rs`。
建议报告把口径写成「37 条 + 1 行汇总」，否则下一个复核车会以为对不上。

### LOW-2｜`stable_len` / `area_cache` 与 `hist`/`dif` 借用的相对次序发生了未登记的交换

基线 `incremental.rs:191-200`：`hist` 借用 → `dif` 借用 → `stable_len` 读 → `area_cache` take。
收口 `incremental/mod.rs:270-279`：`stable_len` 读 → `area_cache` take → `LevelSeries` 里才建 `hist`/`dif` 借用。

**行为影响：零**——四者互不相交（`macd_hist`/`macd_dif`/`macd_state_len`/`area_cache` 是四个字段），
`hist`/`dif` 是纯读引用、`stable_len` 是 `Copy` 读、`area_cache` 的 `mem::take` 不触碰前三者；
T8（`stable_len` 必须后于 MACD 增量）与 T9（`RefCell` 活过整个循环）两条硬约束都保持了。

**问题在陈述**：报告 §1.1 只登记了 T8/T9 两条约束，dispatch 结论行写的是「行为一个字节都没变」、
§1.4 写「179–200 逐字留在主函数」。实际是「逐字留在主函数，但内部四句重排了」。
建议在 §1.4 补一句「四句内部次序有一次已证无害的交换」。

### LOW-3｜6 处仓内行号交叉引用被静默删除，与 §1.3「零条注释被丢弃」措辞不符

基线注释里的 `line 832` / `line 843` / `line 854` / `line 855` / `line 912` / `line 1069`
（`incremental.rs` 内自指行号）在收口中全部消失。

**这是改善而非损失**——基线 `incremental.rs` 只有 820 行，却引用 `line 912` / `line 1069`，
说明这些引用在基线里**早已失效**（是更早某版残留）。删掉是对的。

但 §1.3 写「**零条注释被丢弃**（§4 差集核验逐条落账）」，而 §6.3 的差集只核了**代码行**，
注释行没进差集口径 ⟹ 这句话没有对应的核验证据。准确表述应是
「零条注释**内容**被丢弃；6 处失效行号引用被删除；101 条注释在搬迁中被重排/改写」。

---

## 5. 我这边**未**覆盖的面（有效域声明）

按 `formalization-validity-domain`：

1. **p123 对拍的有效域是 20k bar 的 BTC 单窗**（L2 等级）。20k 覆盖不到的分支（例如
   `retain_level_prefix` 的某些 P 值、`macd_resume_state` 的 `state_len > resume_from` 截断支）
   字节证据不覆盖；旁证是 2205 单测（debug 构建，全部 `debug_assert!` 生效）+ 41 个集成测试二进制。
2. **性能面未复现**。报告 §3 的边界条件「若某个抽出函数改变 `Rc::make_mut` 处的 `strong_count`
   ⟹ 标度回退」我只做了**静态推理**（逐个 `Rc` 携带量的持有者与 drop 时点对照，结论是未变；
   唯一差异是 `TowerLoop` 的 `units`/`moves_tower` 在 `build_level_tower` 返回时即 drop，
   比基线在函数尾 drop **更早**，只会让 `make_mut` 更容易走原地路径），**没有跑标度实验**。
   若要闭这条，需跑 `profile_incremental_tower_real_scaling`（`#[ignore]`，真实大规模）两树对照。
3. **未做异质源（codex/gemini）交叉复核**——本票是单车影子评审。

---

## 6. 结果包六要素

1. **结论**：#633 通过。次序保持（HIGH 门槛）零疑点；纯移动核验在生产面零判据/阈值/断言消息漂移；
   四项机械证据我全部独立复现且更强（debug 构建跑测试、全量集成测试面补齐）。
   计 0 HIGH / 2 MEDIUM / 3 LOW，MEDIUM 与 LOW **全部是证据陈述的准确性问题，无一条改变行为结论**。
2. **定义依据**：#576/#359 的机械证据口径（规范化纯移动差集 + 测试名集合 IDENTICAL + 对拍指纹）；
   Rust 字段不相交借用（`&mut cache.levels` 与 `&cache.macd_{hist,dif}` 可共存）——这是循环外提
   为函数的可行性前提，我在 `mod.rs:272-281` 的调用点核实成立；Rust 可见性规则「私有项对定义模块
   及其后代可见」——`incremental/*` 里的 `pub(super)` 语法上界就是 `incremental` 子树，
   全仓 `pub` 面实测只剩 `mod.rs:249` 一条 `pub fn classify_with_tower_incremental`，对外 API 零变化。
3. **边界条件（本结论会翻转的条件）**：
   - 若 MEDIUM-1 三类格式串改写中任一条输出不等价 ⟹ 从陈述问题升级为行为问题（已逐条排除）；
   - 若存在 20k bar 未覆盖且被 2205 单测 + 41 个集成测试二进制也未覆盖的分支，且该分支上次序被改
     ⟹ 本报告的「零次序疑点」有效域不覆盖它（我的次序核验是**静态逐点对齐**，覆盖全部代码路径，
     不依赖运行覆盖率——故这条翻转要求我的静态对齐有遗漏）；
   - 若 `Rc` 强引用计数在某处实际发生了变化（我只做静态推理）⟹ 标度可能回退，bit-exact 仍成立；
   - 若回填基线不等价于真基线（例如 #630/#635 曾改过 #633 的 8 个文件）⟹ 单变量对照不成立。
     已用 `git log d9f1860124..HEAD -- <8 文件>` 排除（只有 #633 的 commit）。
4. **下游推论**：#633 可关。`incremental/` 的 5 文件面此后可单函数评审、单函数对拍；
   `build_level_projection` 单一来源后 #110 投影层的口径漂移面从 2 处降到 1 处。
   本报告新增一条可复用资产：**回填基线法同样适用于集成测试面**（成本 24 秒），
   #576 §6.5 与 #633 §7.5 两次登记的「全量执行无可比基线」在后续票里不应再作为跳过理由。
5. **谱系引用**：`no-patch-mentality` 第 5 条（声明膨胀）→ MEDIUM-1 / LOW-1 / LOW-2 / LOW-3
   均属证据陈述强于实测；`no-unnecessary-escalation` 格式B `blocked` 条款（652 号局部依赖判定）
   → MEDIUM-2 把不存在依赖的证据面登记为不可推进；`formalization-validity-domain`
   → §5 明确本报告的有效域（20k 单窗 L2 + 静态借用推理，未含标度实验、未含异质源交叉）；
   `coding-style` 不可变性注记（2026-07-27/07-28 编排者裁定）→ `TowerCache`/`TowerLoop` 的就地推进
   属性能累加器与单线程循环携带量，不适用不可变要求，#633 未改变这一性质。
6. **影响声明**：本次评审**零源文件改动、零 git mutation**，只新增本报告一个文件。
   复现产物全部在 `/tmp/shadow633/`（两棵影子树 + 两个独立 target 目录 + 差集脚本），
   未触碰工位的 `rust/target`，未触碰并行线未提交面（`p100_cert_bsp_recon.rs`）、
   `retrace_ledger/`、`level_view*`。

---

## 附：复现命令（可原样重跑）

```bash
# 影子树
mkdir -p /tmp/shadow633/{base,head}
cd /tmp/kimi-nest-mainline && git archive HEAD | tar -x -C /tmp/shadow633/base
cd /tmp/kimi-nest-mainline && git archive HEAD | tar -x -C /tmp/shadow633/head
mv /tmp/shadow633/base/rust/src/theta_v0/classifier/incremental /tmp/shadow633/incremental_new_backup
for f in incremental cand_delta pipeline tower_cache incremental_profile; do
  git -C /tmp/kimi-nest-mainline show d9f1860124:rust/src/theta_v0/classifier/$f.rs \
    > /tmp/shadow633/base/rust/src/theta_v0/classifier/$f.rs; done
for f in cache_and_units incremental_tower level_signals; do
  git -C /tmp/kimi-nest-mainline show d9f1860124:rust/src/theta_v0/classifier/tests/$f.rs \
    > /tmp/shadow633/base/rust/src/theta_v0/classifier/tests/$f.rs; done

# 失败集 / 测试名 / 全量面
(cd /tmp/shadow633/base/rust && CARGO_TARGET_DIR=/tmp/shadow633/target      cargo test --no-fail-fast)
(cd /tmp/shadow633/head/rust && CARGO_TARGET_DIR=/tmp/shadow633/target-head cargo test --no-fail-fast)
(cd /tmp/shadow633/base/rust && CARGO_TARGET_DIR=/tmp/shadow633/target      cargo test --lib -- --list) | grep ': test$' | sort > /tmp/shadow633/list_base.txt
(cd /tmp/shadow633/head/rust && CARGO_TARGET_DIR=/tmp/shadow633/target-head cargo test --lib -- --list) | grep ': test$' | sort > /tmp/shadow633/list_head.txt
diff /tmp/shadow633/list_{base,head}.txt   # 空

# p123 20k 对拍
(cd /tmp/shadow633/base/rust && CARGO_TARGET_DIR=/tmp/shadow633/target      cargo build --release --bin p123_fast_replay)
(cd /tmp/shadow633/head/rust && CARGO_TARGET_DIR=/tmp/shadow633/target-head cargo build --release --bin p123_fast_replay)
DATA=/Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json
P116_MAX_BARS=20001 /tmp/shadow633/target/release/p123_fast_replay      "$DATA" | shasum
P116_MAX_BARS=20001 /tmp/shadow633/target-head/release/p123_fast_replay "$DATA" | shasum
# 两者均 738cb880eb2c3437c3920d4811913e14fc5663ef
```
