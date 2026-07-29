# 影子评审：#576 段 1 classifier/mod.rs 拆分

- 日期：2026-07-29
- 评审车：claude opus 影子评审（新上下文，**未参与 #576 实装**）；全程单线程，无子代理、无后台任务
- 权限自核：只读 + 跑测试；未改任何源文件；零 git mutation（无 add/commit/stash/checkout/rebase/push）
- 对象：`ccd0830df5` / `573644425d` / `074d67e6b9` / `8db5a94a2b` / `fe69f2460b`
- 评审面：`rust/src/theta_v0/classifier/mod.rs` + 拆出的 14 个新文件（**排除** `retrace_ledger/`、`ledger_kernel/`）
- 被审报告：`chanlun/review-results/issue576-classifier-mod-split-20260729.md`
- 开工 `git status` 自核：脏文件 8 个，全部落在 `retrace_ledger/` + `p100_cert_bsp_recon.rs` + `review-results/`（#622/#623 并行车面），**与本评审面零交集**

---

## 0. 结论（先行）

**段 1 的核心断言「零行为变化的纯移动」独立复现成立，可放行。**

七项复验全部通过，其中三项我用了比实施报告**更强**的口径（隔离双树、保序子序列、rustdoc 通道），
结论未被推翻，反而消除了报告 §6.4 自认的一处证据混淆。

| 分级 | 数 | 摘要 |
|---|---|---|
| HIGH | **0** | 零可执行语句行差异；零可见性真放宽；零测试丢失；p123 逐字节相同 |
| MEDIUM | **1** | 「零新增警告」的验证通道不含 rustdoc；实测 rustdoc 有 4 条回归（3 条 doc-link 真断裂 + 1 条渲染），其中 1 条由报告登记为「无代码的注释重排」的 `//`→`//!` 变换**直接引起** |
| LOW | **4** | 报告内四处计数/表述与实测不符（均不影响结论） |

---

## 1. 独立复现逐项（照实报数）

### 复验环境（隔离双树，消除并行车混淆）

实施报告 §6.4 自认了一处约束：基线与收口跑在不同的「他人未提交面」上（#622/#623 在同一
worktree 内推进），故 2144→2175 的漂移只能靠归因、不能靠对照消除。

我改用**隔离双树**消除该混淆：

- `treeA` = `git archive 73c7b34bea`（= `ccd0830df5^`，纯基线提交态）
- `treeB` = `treeA` **逐文件覆盖**为 HEAD 的 15 个 classifier 文件，其余一字不动

两树只差 #576 的 15 个文件，`retrace_ledger/` 等并行车面在两侧**完全同态**。

基线锚点核对：`73c7b34bea:.../classifier/mod.rs` 的 blob = `8d09c1eeaa1726281bb797b2569796d1f08806f5`
= 报告声称的 `a9756715b1` 侧 blob，**同一对象**，3922 行 ✓。

---

### ① 零可执行语句行差异 —— **通过**，且我加做了保序检验

规范化 = `strip()` 后丢空行，多重集差（脚本 `/tmp/shadow576-claude/normdiff.py`）：

| 项 | 实测 | 报告 |
|---|---|---|
| 基线非空行 | 3716 | 3716 ✓ |
| HEAD 15 文件非空行 | 3787 | 3787 ✓ |
| 基线独有（多重集） | **130** | 报告写「3716」（标签错，见 LOW-1） |
| HEAD 独有（多重集） | **201** | 报告写「3787」（同上） |

差集归类（穷举，无第七类）：

| 类 | 基线独有 | HEAD 独有 |
|---|---|---|
| `pub(super)` 配对消解 | — | 54 |
| 注释/文档行（含新模块头） | 47 | 77 |
| `mod` / `use` / `pub use` 声明 | 17 | 61 |
| 模块开闭括号 | 5 | 1 |
| **未归类残差** | **7** | **8** |

未归类残差 15 条**全量列出并逐条配对**，零剩余：

| 基线 | HEAD | 性质 |
|---|---|---|
| `cfg: &super::config::MacdConfig,` ×2 | `cfg: &super::super::config::MacdConfig,` ×2 | `super::` 深度 +1 |
| `let cfg = super::super::config::MacdConfig::default();` ×2 | `let cfg = super::super::super::…` ×2 | 深度 +1 |
| `merged_bars: &[super::types::Bar],` | `merged_bars: &[super::super::types::Bar],` | 深度 +1 |
| `CpScanOwnership, ElementId, LeveledMove, WinMeta,`<br>`compose_level, descend_leveled, …` | `compose_level, compose_level_resume, …`<br>`project_to_units, CpScanOwnership, …, WindowScanCursor,` | `use` 列表合并重排 |
| — | `cand_delta_entry_tower, cand_delta_tower, …` | 新写 `pub use` 续行 |

`use` 列表里新出现的两个名字 `compose_level_resume` / `WindowScanCursor` **不是新增导入**——
基线 `base_mod.rs:792` 有 `use recursive_tower::{compose_level_resume, WindowScanCursor};`
（段内 `use`，Rust 模块级作用域），HEAD 只是把它合并进顶部大 `use`（`mod.rs:143-146`）。

**零可执行语句行差异 CONFIRMED。**

#### 我加做的：保序（子序列）检验

多重集相等只证「行集合相同」，**不排除顺序重排**——报告的机械口径在这一维上有缺口。
补做：剔除已登记新增类（`//!` 头、`use`/`mod` 声明）、还原 `pub(super)` 与 `super::` 深度后，
检验每个新文件的代码行序列是否为基线序列的**保序子序列**：

```
cand_delta.rs      209  0    incremental.rs        779  0    tower_cache.rs        512  0
pipeline.rs        323  0    sublevel.rs           213  0    stage_profile.rs      107  0
oracle_probe.rs     93  0    cp_replay_diagnostics  49  0    incremental_profile.rs 63  0
tests/mod.rs        19  0    tests/cache_and_units 131  0    tests/classify_basics 209  0
tests/incremental_tower 363 0  tests/level_signals  454  0
合计代码行 3524，保序失配 0
```

**3524 行全部保序。** 这比报告的多重集口径严格，结论一致。

#### 可见性核验（独立脚本，非读报告）

`/tmp/shadow576-claude/vischeck.py` 逐项比对基线 mod.rs 与 HEAD 15 文件的项级可见性：

- 可见性不变 119 项 / 放宽 20 项 / **收窄 0** / 基线中无同名 9 项（全为新 `mod` 声明）
- 20 项放宽**全部**是 `PRIVATE → pub(super)`；**非 `PRIVATE→pub(super)` 的真放宽 = 0 处**
- 基线 mod.rs 顶层 `pub` 项恰 10 个（`base_mod.rs:187,221,512,527,547,581,617,710,905,1595`），
  HEAD `mod.rs:119/123-125/130/134` 的 `pub use` 恰好这 10 个——**零遗漏、零多余**
- `pub(super)` 语法上界即 `classifier` 子树，编译器强制；`TowerCache` 虽是对外 `pub` 类型，
  其 15 个字段为 `pub(super)`，对 `classifier` 外仍不可见 ✓

**可见性口径「保全非放宽」CONFIRMED。**

---

### ② 341 引用点零改写 —— **零改写 CONFIRMED**，计数口径见 LOW-2

五个 commit 的 `--stat` 逐个核对，改动面**全部**落在 `classifier/` 顶层：

- `ccd0830df5`：mod.rs + 3 新文件
- `573644425d`：mod.rs + tower_cache.rs
- `074d67e6b9`：mod.rs + pipeline.rs + cand_delta.rs
- `8db5a94a2b`：mod.rs + incremental.rs + sublevel.rs + **pipeline.rs (+1 行)**
- `fe69f2460b`：mod.rs + incremental_profile.rs + tests/ 五文件

`8db5a94a2b` 对 `pipeline.rs` 的那 +1 行经查是 `use super::sublevel::extract_second_for_level;`
（`pipeline.rs:6`）——`sublevel` 外迁的必要 import 补正，属 C3 类，零行为。

**仓外零文件被触碰 ⟹ 全仓引用点零改写，成立。**

### ③ 测试名 IDENTICAL —— **通过（隔离口径下比报告更干净）**

```
treeA: cargo test --release --lib -- --list  →  2281 项
treeB: 同上                                  →  2281 项
原始名 diff：62 行，全部是 classifier::tests 的 30 项加了子模块前缀
剥前缀后多重集比较：差异 0 条，A==B == True
```

前缀分布 `cache_and_units` 4 / `classify_basics` 10 / `incremental_tower` 8 / `level_signals` 8 = 30 ✓。

隔离树下**总数 2281 = 2281 不变**，这独立坐实了报告 §3(3) 对 +31 条的归因
（全部属 `retrace_ledger::tests::`，隔离掉即消失）。

### ④ 失败集 —— **通过**

| 树 | `cargo test --release --lib --no-fail-fast` |
|---|---|
| treeA（纯基线） | **2144 passed / 0 failed / 137 ignored** |
| treeB（仅 #576 改动） | **2144 passed / 0 failed / 137 ignored** |
| 当前工作区（HEAD + 他人未提交面） | **2185 passed / 0 failed / 137 ignored** |

隔离双树**逐字相同**，失败集两侧均空，无需归因。
工作区 2185 与报告收口的 2175 相差 10，是报告落盘后 #623 修复轮继续加测试所致——全绿，无红需归因。

### ⑤ p123 对拍 —— **通过**

```
P116_MAX_BARS=20001 ./target/release/p123_fast_replay analysis/data_cache/btc_1m_full.json
treeA: EXIT=0  sha1(stdout) = 738cb880eb2c3437c3920d4811913e14fc5663ef
treeB: EXIT=0  sha1(stdout) = 738cb880eb2c3437c3920d4811913e14fc5663ef
stdout diff：空（16 行门行逐字相同）
stderr diff：唯一差异 prefix_s=0.116 vs 0.114（墙钟耗时，非语义）
```

门行含 `P123_YIELD candidates=50 … terminal_confirmed=6`、`P123_CERT caliber_A=8 caliber_B=6`、
`P123_BIT_EXACT … classification_total_diff=1` 全部不变 ✓，与报告声称的 sha1 一致。

### ⑥ `#[cfg(test)]` 探针 —— **通过（在 crate 内且真计数）**

票面点名的 `event_probe` 属 `cand_event.rs`，该文件在本分支不存在（报告 §5 已证）。
本分支的同型对象是 `oracle_probe` + `cp_replay_diagnostics`，逐项核实：

- `mod.rs:110-111` 保持 `#[cfg(test)] pub mod oracle_probe;` —— **仍在 lib crate 内**，非跨 crate
- 写入侧 5 处全部 `#[cfg(test)]` 门控且逐字移动：
  `incremental.rs:251`（`on_minparts_break`）、`:318`（`on_cascade_event`）、`:527`（`on_pop_rescan`）、
  `:757`（`on_empty_break`）、`:800`（`on_forest_dirty`）
- 消费侧断言是**硬计数**，非静默废断言：
  `backtest/incremental.rs:663` `assert!(p.t_eq1 > 0, "fixture2 未触发 had_emitted_window pop T==1…")`
  `backtest/incremental.rs:664` `assert!(p.t_gt1 > 0, "…frontier 值改写路径覆盖为零")`
- 这两条 always-run 断言在 treeA/treeB 均 `ok` ⟹ 拆分后探针链路**真触发、真计数** ✓
- `cp_replay_diagnostics` 的守卫测试 `p52_frontier_diagnostics_are_level_scoped_and_resettable`
  迁至 `tests/cache_and_units.rs`，两树均通过 ✓

`tests/` 子模块留在本 crate 内（`mod.rs:149-150` `#[cfg(test)] mod tests;`），
子文件经 `use super::super::*` + 显式 `super::super::pipeline::…` 访问 `pub(super)` 项——
`classifier::tests::*` 是 `classifier` 的后代，可见域合法，无越界。

### ⑦ `incremental.rs` 构成核 —— **属实（报告 20 行头应为 21，见 LOW-4）**

```
文件总行           820
文件头 1..21       21 行（//! 模块文档 4 + 4 个 use + 两个 #[cfg(test)] 开关静态及其文档）
函数文档 22..55    34 行
函数体 56..820     765 行（pub fn classify_with_tower_incremental）
```

765 + 必要头 ⟹ 纯移动确实无法收敛到 ≤800。报告「不为凑行数做有风险的函数切分」的
`no-patch-mentality` 第 6 条援引成立。两个开关静态 `CASCADE_EPROBE` / `CASCADE_FULLCLEAR`
为文件内私有，全仓其余命中皆为 env 名字符串/注释，无跨模块消费 ✓。

---

## 2. MEDIUM-1：「零新增警告」的验证通道不含 rustdoc，实测 4 条回归

**证据**（`cargo doc --no-deps --lib`，两树对比）：

```
treeA: newchan_rust (lib doc) generated 357 warnings
treeB: newchan_rust (lib doc) generated 359 warnings
```

逐条 diff（treeB 新增 4 / 消失 2）：

| # | 警告 | 位置 | 基线态 |
|---|---|---|---|
| 1 | `unresolved link to `super::strategy::interp::TreeCache`` | `tower_cache.rs:246` | 基线 `mod.rs:950/962/994` **可解析** |
| 2 | `unresolved link to `segment_to_unit`` | `tower_cache.rs:291` | 基线 `mod.rs:1039` 可解析（warn「links to private item」）|
| 3 | `unresolved link to `classify_impl`` | `cand_delta.rs:13` | 基线 `mod.rs:537` 可解析（同上）|
| 4 | `unclosed HTML tag `LeveledMove`` | `tower_cache.rs:11` | 基线 `mod.rs:761` **是 `//` 普通注释，rustdoc 不解析** |

**失败场景**：
- 第 1 条最实：`[`super::strategy::interp::TreeCache`]` 是「谁在下游消费 `generation`/`forest_epoch`」
  的**可追溯性锚点**（一处 doc 里出现 3 次）。模块下沉一级后 `super` 从 `theta_v0` 变成 `classifier`，
  链接死掉。将来重构 `TreeCache` 时，从 `TowerCache` 侧反查不到消费者。
- 第 2/3 条：`l0_units` / `cand_delta_tower` 的公开文档指向的实现锚点断链。
- 第 4 条的成因值得单列：报告 §3(1) 把 `//` 段注释 → `//!` 模块文档登记为**变换类 1「22 行注释重排，无代码」**。
  但 `//` → `//!` 并非无害的格式变换——它把原本**不进 rustdoc 解析域**的注释提升为 rustdoc 输入，
  于是 `base_mod.rs:761` 里裸写的 `Vec<Vec<LeveledMove>>` 被当作未闭合 HTML 标签。
  即：该变换有报告未识别的副作用面。

**为什么是 MEDIUM 不是 HIGH**：运行时行为零变化（p123 逐字节相同 + 2144 测试逐字相同已证），
损失仅在文档可用性与可追溯性。

**为什么不是 LOW**：报告 §4 的小节标题就是「零新增警告」，正文「lib 37 / lib-test 53，与基线同数同集」
未限定 lint 通道，读者会读成「拆分零副作用」。按 `formalization-validity-domain`，
这是**有效域（rustc build/test）小于声明覆盖面（"零新增警告"）**的实例。

我复验了报告已声明的两个通道，**均属实**：
- lib 警告：treeA/treeB 逐条相同（69 种去重文本，`diff` 空）✓
- lib-test 警告：两树均 53 ✓

**建议处置**：4 条各是一行文档修（`crate::theta_v0::strategy::interp::TreeCache` 绝对路径 /
`super::pipeline::segment_to_unit` / `super::pipeline::classify_impl` / 给 `Vec<Vec<LeveledMove>>`
加反引号），零行为风险。若本票坚持「零行为变化纯移动」不夹带任何非移动改动，
则应把 4 条登记为已知回归并另票收，**不应留在「零新增警告」的表述之下**。

---

## 3. LOW（四条，均不影响结论）

### LOW-1：§3(1) 把「总非空行数」写成了「独有行数」

报告原文：「原文件独有 3716 → 差集全部落在 6 类…；新文件独有 3787 → 同上」。
3716/3787 是两侧**非空行总数**（我复算完全一致），实际多重集**独有**数为 **130 / 201**。
数字本身对，标签错——按字面读会以为差集有 3716 条待归类。

### LOW-2：§3(2) 引用点合计 341 的 grep 口径未说明，复算得 326

我用 `grep -rn "classifier::<名字>\b" rust/src rust/tests` 复算：

| 项 | 我 | 报告 | | 项 | 我 | 报告 |
|---|---|---|---|---|---|---|
| `Classification` | 118 | 126 | | `TowerCache` | 53 | 53 ✓ |
| `classify_with_tower` | 48 | 48 ✓ | | `classify_with_tower_incremental` | 30 | 30 ✓ |
| `LevelState` | **23** | 30 | | `stage_profile` | 15 | 15 ✓ |
| 其余 7 项 | 全部一致 | | | **合计** | **326** | 341 |

差额集中在 `Classification`（-8）与 `LevelState`（-7），推测是 `use classifier::{…, LevelState, …}`
花括号列表形式的计法差异。**核心断言「零改写」不依赖这张表**——它由「五 commit 零外部文件」硬证（见 §1②）。
建议报告补一句 grep 命令原文，使该表可复现。

### LOW-3：§2 可见性口径的 `pub(super)` 计数少 4

报告：「pipeline 8 + cand_delta 2 + sublevel 6 + tower_cache（1 struct + 1 type + 2 fn + 30 字段，
其中 LevelCache 19 + TowerCache 11）= 50 处」。

实测 `grep -c "pub(super)"`：`pipeline.rs` 8 ✓ / `cand_delta.rs` 2 ✓ / `sublevel.rs` 6 ✓ /
`tower_cache.rs` **38**（LevelCache 19 ✓ + **TowerCache 15**（`tower_cache.rs:159,161,164,166,169,
174,180,185,192,195,197,201,213,228,231`）+ struct 1 + type 1 + fn 2）= **54 处**。

`TowerCache` 字段实为 15 不是 11。可见域仍是 `classifier` 子树，无行为影响。

### LOW-4：§4 `incremental.rs` 头部行数 20 应为 21

见 §1⑦ 实测。

---

## 4. 我复现不出问题、可给报告加分的三点

1. **隔离双树消除了报告 §6.4 自认的约束**：报告说「基线与收口不是同一个他人面快照，
   这是三车共用 worktree 的固有约束，非本票可消除」。用 `git archive` + 逐文件覆盖可以消除——
   两树 2144/0/137 逐字相同、2281 测试名相同、p123 sha1 相同。该约束**已被证明不影响结论**。
2. **保序子序列检验**（报告未做）：3524 代码行 0 失配，堵上了多重集口径的顺序维缺口。
3. **可见性的编译器级上界**：`pub(super)` 在子模块内的语法上界就是 `classifier` 子树，
   报告 §7.3 的边界条件「若某个 `pub(super)` 实际放宽了可见域」在 Rust 下不可能发生——
   该边界条件写得保守是对的，但它不是一个真的风险面；真风险面是**私有→`pub`/`pub(crate)`**，
   我独立扫了，**0 处**。

---

## 5. 未覆盖面（诚实声明）

- **p123 20k 的有效域不是全窗**（报告 §7.3 已自认，我同意）。我未跑更长窗口。
  旁证是 treeA/treeB 的 2144 个单测逐字相同 + 保序检验 0 失配。
- **未跑 `cargo test --release` 全量执行**（集成测试）。我跑了 `--lib` 全量。
  报告 §6.5 已声明该缺口并给了理由（无可比基线 + diff 面上零引用点变化），我复核该理由成立。
- **未评审 `retrace_ledger/`、`ledger_kernel/`**（#622/#623 面，任务书明确划出）。
- **段 2/3 的阻塞证明未独立复核**（`cand_event.rs` 不在本分支）——不在本次评审面内，
  但我顺手确认了 `find rust/src -name "cand_event*"` 无命中，与报告 §5 一致。

---

## 6. 结果包（简化版——纯技术性核实，不涉概念定义）

1. **结论**：#576 段 1 的「零行为变化纯移动」独立复现成立，可放行。
   0 HIGH / 1 MEDIUM（rustdoc 通道 4 条回归 + 「零新增警告」声明有效域）/ 4 LOW（报告内计数与表述）。
2. **边界条件（结论翻转的条件）**：
   - 若存在**不在 p123 20k 覆盖、也不在 2144 个单测覆盖**的分支，且该分支的行在拆分中被跨函数错置——
     我的保序检验（子序列，按文件）只保证**文件内保序**，不排除「同一行被搬到另一个函数的等价位置」。
     该残余风险由 rustc 编译 + 2144 测试兜底，未被机械证据直接排除。
   - 若 `use super::*` 的 glob 面在未来某个新增项上产生同名遮蔽，`pipeline`/`cand_delta`/`tower_cache`/
     `incremental`/`sublevel` 五个子模块的解析结果可能静默变化（报告 §6.3 已登记该设计选择）。
     本次拆分时点上编译零新增警告 ⟹ 当前无遮蔽。
   - 若 rustdoc 断链被认定为「行为」的一部分（例如文档站是交付物），MEDIUM-1 升为 HIGH。
3. **影响声明**：本评审**未改动任何源文件**，未做任何 git mutation。
   产出只有本文件。复验产物在 `/tmp/shadow576-claude/`（treeA/treeB/脚本/日志），仓外临时物。
