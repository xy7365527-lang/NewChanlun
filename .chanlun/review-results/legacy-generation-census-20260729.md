# #753（C7）前代族名分核定第一批——五族引用核查 + 名分表 + 处置方案

- 日期：2026-07-29
- 票据：[#753](https://github.com/xy7365527-lang/NewChanlun/issues/753)（parent #743，blocked_by #745）
- 名分程序：`docs/adr/0004-generation-constitution.md`（世代宪法，2026-07-28 裁 + 2026-07-29 补充一「新生入口待驱动」第五格）
- 程序样板：#745（C6，`.chanlun/review-results/issue745-impl-20260729.md`）
- 性质：**纯调研，零改动**（本报告文件除外）。全程亲手 grep，未派子代理。

---

## 0. 全局背景核查（决定后续判据的关键事实）

```
grep -n "^\s*\(pub \)\?mod " rust/src/lib.rs
```
`lib.rs` 五族全部 `pub mod`（spiral/fugue_v3/recursive_t/theta_v0/trading），文档注释原文（design 阶段用语，已过期）：spiral/fugue_v3/recursive_t 三处均写「尚未接入 PyO3」——**核实为假**：三族的 `ffi.rs` 均已在 `lib.rs:2436-2449` 的 `#[pymodule] fn newchan_rust` 里注册（`PySpiralStream`/`run_spiral`、`PyFugueV3Stream`/`run_fugue_v3`、`run_recursive_t`/`PyTFugueStream`/`run_t_fugue`/`PyRecStream`）。doc 注释是历史遗留，不可作名分依据，以下全部按 grep 实证判定。

```
for m in bi_engine bi_zhongshu_bsp buysellpoint c_segment_verify divergence fractal level macd moves orchestrator ph segment segment_layers stroke zhongshu; do
  grep -rn "crate::$m\b\|super::$m\b" rust/src/theta_v0 --include=*.rs | wc -l
done
```
π（`theta_v0`，ADR-0004 裁定的唯一现役引擎）对顶层散件（bi_engine/buysellpoint/segment/zhongshu/divergence/moves/…）**零引用**。唯一疑似命中（`super::divergence`/`crate::segment` 各若干处）核实后是 `theta_v0::classifier::divergence.rs`（theta_v0 自己的同名子模块）与 `theta_v0::classifier::segment` 系列命中，与顶层 `crate::divergence`/`crate::segment` 是**两套完全独立、同名不同实体的模块**——theta_v0 全自包含，不依赖任何前代族代码。

`rust/tests/`（顶层集成测试目录）对五族全部**零引用**；`rust/src/bin/` 44 个探针 bin 对五族全部**零引用**（`theta_backtest.rs` 命中的 `trading` 字样是 doc 注释里的 `nautilus-{trading,model,backtest}`——Python `nautilus_trader` 生态命名，非 `crate::trading`，误报已排除）。

**顶层散件本身互相耦合成一个整体**：`bi_engine→stroke→segment→zhongshu→buysellpoint→divergence→macd→moves→orchestrator`（Phase 2/3/4/7 Python 等价引擎）是 spiral/fugue_v3/recursive_t/trading 四族共同的底座——四族的 `stroke::`/`zhongshu::`/`buysellpoint::`/`divergence::` 引用遍布 30+ 文件（证据见 §1.5）。**顶层散件不能孤立处置**：删它等于连根拔掉四族。

---

## 1. 五族引用核查

### 1.1 `spiral/`（3149 行，12 文件，D∞ v2 螺旋引擎）

- **生产（theta_v0/π）**：0（§0 已证）。
- **族内消费者**：`fugue_v3/{axis,mod,operate,observe,morphology,engine}.rs` 六处 `use crate::spiral::signal::{...}`（fugue_v3 复用 spiral 信号层，fugue_v3 自身也是前代族，非 π 路径）。
- **PyO3**：`lib.rs:2437-2438` 注册 `PySpiralStream`/`run_spiral`。
- **Python 调用方**：`trading_system/compare_spiral_unn.py:60` `nr.SpiralStream(...)`——`git log -1 -- trading_system/compare_spiral_unn.py` = `2026-06-21`（ADR-0004 裁定 2026-07-28 之前 5 周，冷）。
- **测试**：6 处内部 `#[cfg(test)] mod tests`（自测，无外部测试文件引用）。
- **bin/**：0。
- **文档**：`docs/spiral_engine_v2_architecture.md`（自引用设计文档）、`docs/unified_recursive_operator_T.md`（被后继架构文档提及）。
- **关键自述**（`spiral/mod.rs:15-16`）：「v2 **不删除** `trading::unified_necessity`（保留为 bit-exact 对照基线，直到 v2 验收通过）」——spiral 自己把 `trading::unified_necessity` 标记为对照件；但 spiral 本身已是前代族，此对照关系的存续意义已随 ADR-0004 过期。

### 1.2 `fugue_v3/`（2042 行，11 文件，D∞ word 处理器 / 赋格引擎 v3）

- **生产（theta_v0/π）**：0。
- **族内消费者**：`recursive_t/{stream,ffi,t_engine_run,t_engine,rec_engine,prove_guards}.rs` 六文件重度 `use crate::fugue_v3::{accounting, layer::{FugueResult,Layer}, prove::prove_nav_neutral, ...}`——**fugue_v3 是 recursive_t 里「T 引擎」子簇（见 1.3b）的会计基座**，非独立可删单元。
- **PyO3**：`lib.rs:2440-2441` 注册 `PyFugueV3Stream`/`run_fugue_v3`；三份 trading_system python 脚本均**未**调用（§0 python 调用方核查，无 `nr.FugueV3Stream`/`nr.run_fugue_v3` 命中）——PyO3 导出面已无实际调用方。
- **测试**：2 处内部 `mod tests`。
- **bin/**：0。
- **文档**：`docs/three_stages_accounting_design.md`、`docs/prove_guards_migration.md`、`docs/rec_engine_implementation_audit.md`、`docs/btc_engine_evolution_baseline.md`（均为 v3→T 演进期设计/审计文档）。
- **`git log -1 -- rust/src/fugue_v3`** = `2026-06-21`（冷，5 周）。

### 1.3 `recursive_t/`（14673 行，16 文件——**票面已警示的命名陷阱：一个目录装了两个不同世代的实体**）

`git log -1 -- rust/src/recursive_t` = `2026-07-29`（今天，来自 #614「kimi-nest-mainline 并轨」合并，非新功能开发——该合并把 kimi 线的目录重排并入 main，不代表族内代码语义仍在演进）。目录内部按依赖关系可清楚切成两个不相交子簇：

**(a) 原生「统一递归算子 T」standalone 子簇**（`center.rs`/`trend.rs`/`divergence.rs`/`operator.rs`/`types.rs`/`mod.rs` 的 `iterate()` 驱动器，共 2611 行）：
- 不依赖 `fugue_v3`（`grep -c "fugue_v3::" center.rs trend.rs divergence.rs operator.rs types.rs` = 0）。
- PyO3：`lib.rs:2442` `run_recursive_t`——实测该函数体（`ffi.rs:74`）调用的是 `iterate(units, perfection)`，即这个 standalone 子簇。
- **Python 调用方：0**（三份 trading_system 脚本 grep `nr\.` 调用清单里没有 `run_recursive_t`）——PyO3 导出但零实际调用证据。
- **生产（theta_v0）：0**。

**(b) 「T 递归自相似架构 v2」/T 引擎子簇**（`rec_engine.rs`/`rec_driver.rs`/`rec_stream.rs`/`t_engine.rs`/`t_engine_run.rs`/`stream.rs`/`ffi.rs`/`prove_guards.rs`/`backtest.rs`/`backtest_run.rs`，共 12062 行——**这才是记忆库「T 引擎 BTC 权威基线」「T 递归自相似架构 v2」指认的实体**，与 (a) 同名不同物）：
- 重度依赖 `fugue_v3::accounting`/`Layer`（§1.2 已证）+ `trading::types::{Polarity, MAX_LADDER, INITIAL_CAPITAL}`。
- PyO3：`PyTFugueStream`/`run_t_fugue`（走 `t_engine.rs`）+ `PyRecStream`（走 `rec_engine.rs`/`rec_stream.rs`）。
- **`PyTFugueStream` 有真实 Python 调用方**：`trading_system/backtest_t_fugue.py:166` `nr.TFugueStream(...)`——这是历史上的 NautilusTrader 生产回测路径（`git log -1` = `2026-06-21`，冷，先于 ADR-0004 五周）。
- **`PyRecStream`/`rec_engine`/`rec_stream` 零 Python 调用方**——`rec_btc`（`rec_stream.rs:1290`）、`t_engine_8x3`（`t_engine_run.rs:151`）均为 `#[cfg(test)]` 内部自测函数，非外部驱动；对照记忆「rec_engine.rs = rec_btc 驱动的 Rust 内部 harness 路径，非 NT 生产路径」，grep 实证与记忆一致。
- **生产（theta_v0）：0**。
- **文档**：`docs/rec_t_8x3_l3_verification.md`、`docs/prove_guards_migration.md`、`docs/btc_engine_evolution_baseline.md`。

### 1.4 `trading/`（29923 行，30 文件，「有机赋格 v2」——**票面未点名但本次核查发现的第二个命名陷阱**）

`git log -1 -- rust/src/trading` = `2026-07-29`（今天，来自 #638「ThirdPointBook 登记账」——**这是真实的当日新开发，不是并轨噪音**）。30 个文件全部 `mod`-声明在 `trading/mod.rs`（无族内孤儿），但引用面完全不均质：

- **28 个「有机赋格 v2」文件**（`master.rs`/`runner.rs`/`positional.rs`/`tape.rs`/`allocator.rs`/`unified_necessity.rs`/`unified_voice.rs`/`unified_osc.rs`/`unified_recursive.rs`/`ledger.rs`/`config.rs`/`level_operating_unit.rs`/`fatigue_gate.rs`/`trend_exhaustion.rs`/`trade_behavior.rs`/`depth_ref.rs`/`consolidation_ablation.rs`/`sublevel_confirmation_ablation.rs`/`axiom_voice.rs`/`dual_voice.rs`/`isolated_fugue.rs`/`nested_fugue.rs`/`nested_interval_fugue.rs`/`positioning_chain_fugue.rs`/`recursive_nested_fugue.rs`/`recursive_position.rs`/`positional_fusion.rs`/`types.rs`）：**生产（theta_v0）0**；PyO3 经 `lib.rs:1723-2307` 多处（`trading::config::variant`/`trading::runner::run_organic`/`trading::recursive_position::run_recursive`/`trading::positional::run_positional`/`trading::unified_necessity::UnnStreamCore`）导出；Python 调用方：`backtest_unn_stream.py`（`nr.UnnStream`/`nr.run_positional_rust`，冷，2026-06-21）+ `compare_spiral_unn.py`（同上）。族内被 spiral/fugue_v3/recursive_t 四族共同当基础设施用（`center_book`/`depth_ref`/`positional`/`tape`/`types` 遍布 30+ 处引用，见 §0）。

- **`center_book.rs`（833 行）**：**双重身份**——(i) 作为 v2 遗留基础设施，被 `runner.rs`/`positional.rs`/`unified_necessity.rs`/`unified_voice.rs`/`unified_osc.rs`/`unified_recursive.rs`/`allocator.rs`/`depth_ref.rs`/`axiom_voice.rs`/`dual_voice.rs`/`isolated_fugue.rs`/`nested_fugue.rs`/`nested_interval_fugue.rs`/`positioning_chain_fugue.rs`/`recursive_nested_fugue.rs`/`level_operating_unit.rs`/`positional_fusion.rs` **16 个 trading/ 内部文件** + `spiral/signal.rs` 共 17 处消费，是 v2/spiral 世代的核心账本类型；(ii) 其 `consume_death_certificate` 方法被 **ADR-0004 补充一原文点名**「自本裁定起判为『新生入口待驱动』」——`theta_v0/classifier/retrace_ledger/mod.rs:32` doc 注释登记消费入口已立（#637），但 grep 实证 theta_v0 内**没有一处真实 `use`/函数调用**（只有 doc 注释里的类型名提及），驱动链归 #575 后续票，票面口径="生产代码里类型出现即达标"非"已被调用"。

- **`third_point_book.rs`（604 行）**：**整份新生**——`git log` 显示为 2026-07-29 当日因 #638 新建（非老代码），是 π 消费面在 `trading/` 目录下的新落点，同样被 ADR-0004 补充一原文点名「`trading::ThirdPointBook` 自本裁定起判为『新生入口待驱动』」。grep 实证：全仓仅 `theta_v0/classifier/retrace_ledger/mod.rs` 的 doc 注释提及类型名，无真实 `use` 导入/调用，驱动链归 #575。

---

## 2. 名分表（ADR-0004 五态判据）

| 族 / 文件 | 判定 | 理由（对照件须答"对照什么、差在哪"） |
|---|---|---|
| `spiral/`（全部 12 文件） | **deprecated 待退役** | ADR-0004 裁 π 唯一现役后前代族；生产 0，仅族内互引 + 1 处冷 python 调用（5 周无改动）；PyO3 面仍导出非纯孤儿 |
| `fugue_v3/`（全部 11 文件） | **deprecated 待退役** | 生产 0，PyO3 导出但零 python 调用方；是 recursive_t(b) 子簇的会计依赖，物理上不能早于 recursive_t(b) 单独删除 |
| `recursive_t/`(a) standalone T 算子（center/trend/divergence/operator/types/mod-iterate，2611行） | **deprecated 待退役** | PyO3 `run_recursive_t` 导出但零 python 调用证据；生产 0；与同目录的 (b) 是两个不同世代实体，判定须分文件不可按目录笼统处理 |
| `recursive_t/`(b) T 引擎（rec_engine/rec_driver/rec_stream/t_engine/t_engine_run/stream/ffi/prove_guards/backtest/backtest_run，12062行） | **deprecated 待退役** | `t_engine.rs` 一支曾是 NT 生产回测路径（`backtest_t_fugue.py` 真实调用），但该 python 脚本 2026-06-21 后未再改动，早于 ADR-0004（2026-07-28）裁 π 唯一现役；`rec_engine`/`rec_stream` 一支从未离开 Rust 内部 harness（`rec_btc` 自测），从未 NT 化 |
| `trading/` 28 个 v2 文件 | **deprecated 待退役** | 生产 0；PyO3 导出 + 2 处冷 python 调用（2026-06-21）；是 spiral/fugue_v3/recursive_t 四族共同基础设施，删除顺序须最后 |
| `trading/center_book.rs` | **拆分判定**：结构体本身随 28 个 v2 文件走 deprecated；`consume_death_certificate` 方法 = **ADR-0004 补充一「新生入口待驱动」**（原文点名） | 对照 ADR-0004 补充一：有票号(#637) ∧ 生产代码已引用类型(doc注册) ∧ 生产零调用者 ∧ 驱动链归 #575 后续票在案——四条机械判据全满足 |
| `trading/third_point_book.rs` | **新生入口待驱动**（ADR-0004 补充一原文点名） | 同上，票号 #638；整份文件当日新建，非老代码误判 |
| 顶层散件（`bi_engine.rs`/`bi_zhongshu_bsp.rs`/`buysellpoint.rs`/`c_segment_verify.rs`/`divergence.rs`/`fractal.rs`/`level.rs`/`macd.rs`/`moves.rs`/`orchestrator.rs`/`ph.rs`/`segment.rs`/`segment_layers.rs`/`segment_tangency_tests.rs`/`stroke.rs`/`zhongshu.rs`，11449行） | **对照件**——对照什么：是 Phase 2/3/4/7「Rust↔Python 批量等价」基线引擎本身（`lib.rs:1-18` 头注释自述），历史上是被对照对象非对照者；与现役差在哪：π（theta_v0）完全自包含未复用一行；本次判定它继续持"对照件"角色是因为**它同时是 spiral/fugue_v3/recursive_t/trading 四族的编译期硬依赖**（stroke::/zhongshu::/buysellpoint::/divergence:: 遍布 30+ 处），非独立可判 deprecated——删除顺序必须晚于四族全部处置完毕 |

---

## 3. 处置方案（三选逐族，物理执行归后续票，本票只产方案）

| 族 | 方案 | 理由 |
|---|---|---|
| `spiral/` | **留原位标 GUARD-ROLE**（沿 #745 先例：`GUARD-ROLE: deprecated-pending-retirement` 头部标记 + 名分四要素） | fugue_v3 仍族内引用未断，先标记不物理动，待 fugue_v3/recursive_t 处置票排定后再定删除时点 |
| `fugue_v3/` | **留原位标 GUARD-ROLE** | 是 recursive_t(b) 的硬依赖，必须与 recursive_t(b) 同批处置，不可单独先删 |
| `recursive_t/`(a) standalone | **留原位标 GUARD-ROLE**，但可优先进入下一批"纯孤儿候选"复核 | 零 python 调用证据强于 (b)，删除阻力最小，但仍需切票时二次核查确认 PyO3 导出面无外部隐藏调用方（本票未做全仓外可见性穷举） |
| `recursive_t/`(b) T 引擎 | **留原位标 GUARD-ROLE** | `t_engine.rs` 支曾是 NT 生产路径，物理删除前须与用户确认 `backtest_t_fugue.py` 是否已确定废弃（非本票裁量范围） |
| `trading/` 28 个 v2 文件 | **集中 `legacy/` 目录**（`rust/src/legacy/trading/`） | 数据量最大（29923行）且是四族共同基础设施，先归堆比标记更能让下游"删四族"的执行票看清边界；但须等 spiral/fugue_v3/recursive_t 三族先各自决定去留，避免 legacy/ 目录内又生成新的跨目录耦合 |
| `trading/center_book.rs`（除 `consume_death_certificate`） / `trading/third_point_book.rs` | **不处置，留原位** | 前者结构体随 28 文件走 legacy/ 但 `consume_death_certificate` 方法需保留在可达路径；后者是 π 消费面新生入口，物理删除会破坏 #575/#638 后续票的落点——执行票必须显式排除这两处 |
| 顶层散件 16 文件 | **留原位标 GUARD-ROLE**（不集中不删） | 四族全部处置完毕前是它们的编译期依赖，过早移动会导致中间态编译失败；待四族清空后单独开票复核是否仍需保留（此时才真正沦为孤儿） |

---

## 4. 追加范围：kimi 线孤儿簇整簇名分（#745 评审 MED 登记，逐件核查）

```
grep -rn "^\s*\(pub \)\?mod cand_delta;\|^\s*\(pub \)\?mod tower_cache;\|^\s*\(pub \)\?mod sublevel;" rust/src --include=*.rs
```
→ **零命中**：`cand_delta.rs`/`tower_cache.rs`/`sublevel.rs` 全仓无 `mod` 声明，编译树外孤儿，与 #745 已删的 `pipeline.rs` 同批 kimi 线拆分残留。

| 文件 | 现状 | 名分 | 处置 |
|---|---|---|---|
| `theta_v0/classifier/cand_delta.rs`（含 `cand_delta_tower_with_series`，:146） | `use super::pipeline::{segment_to_unit, unit_to_segment}`（:7，指向 #745 已删文件，**若被纳入编译树会直接编译失败**）；内含第二份 Cand^δ 判据驱动器（`cand_delta_tower`/`cand_delta_tower_with_series`），与现役 `theta_v0/classifier/cand_predicate.rs`（`DivCand^δ_{Θ,ℓ}`，`pub mod cand_predicate;` 已在 mod.rs:99 声明）判据重复 | **孤儿待删** | 删除——现役判据唯一份在 `cand_predicate.rs`，两份不能并存（判据零分叉铁律） |
| `theta_v0/classifier/tower_cache.rs` | 无 `mod` 声明；:291 附近含指向已删 `pipeline.rs` 的过期文档（#745 评审已登记）；被 `sublevel.rs:4` 引用（同簇内部互引，非编译树） | **孤儿待删** | 删除 |
| `theta_v0/classifier/sublevel.rs` | 无 `mod` 声明；`use super::tower_cache::AreaCache`（:4，簇内互引） | **孤儿待删** | 删除 |
| `theta_v0/classifier/incremental/`（目录：`invalidate.rs`/`mod.rs`/`scan.rs`/`tower.rs`/`assemble.rs`） | 无 `mod incremental;`（外部声明）指向此目录；`mod.rs:6` 仍 `use super::pipeline::{build_level_projection, segment_to_unit}` | **孤儿待删（整目录）** | 删除——**命名陷阱须在执行票里显式标注**：`theta_v0::backtest::incremental`（`backtest/mod.rs:51` `pub mod incremental;`）是完全独立的现役模块（`IncrementalClassifier`，被 `runner.rs`/`econ_positive.rs`/`l3_delta_r_alpha.rs`/7个bin探针生产调用），执行者必须先确认删除目标是 `classifier/incremental/` 而非 `backtest/incremental.rs`，避免误删现役 |
| `theta_v0/classifier/tests/`（目录：`cache_and_units.rs`/`classify_basics.rs`/`incremental_tower.rs`/`level_signals.rs`/`mod.rs`） | `classifier/mod.rs:3049` 是**内联** `#[cfg(test)] mod tests { ... }`（花括号体，非 `mod tests;` 指向目录），该目录从未被任何 `mod` 声明纳入编译树，被自身内联 `mod tests` 完全遮蔽；目录内 3 个文件仍 `use super::super::pipeline::...`（#745 已删）+ `use super::super::cand_delta::...`/`tower_cache::...`（本簇同伴） | **孤儿待删（整目录）** | 删除——遮蔽目录零编译可达，且引用链条全部指向本簇其余孤儿件，无独立留存价值 |
| `theta_v0/classifier/incremental_profile.rs` | 无 `mod incremental_profile;`（外部声明）；内容与 `classifier/mod.rs:6118` 内联 `mod incremental_profile { ... }` 块几乎逐字重复（doc 注释开头完全一致） | **孤儿待删（重复件）** | 删除——mod.rs 内联块已是唯一存活版本，独立文件是被内联覆盖前的旧稿 |

**簇内小结**：全簇 6 个单元（3 文件 + 2 目录 + 1 重复文件）无一被编译树声明，`use super::pipeline::` 残留引用有 5 处（cand_delta.rs:7、incremental/mod.rs:6、tests/{cache_and_units,level_signals,incremental_tower}.rs 各 1 处）全部指向 #745 已删的真孤儿，**判据实现全仓独一份在 `cand_predicate.rs`**（本次核查确认闭合，可在 #753 或其执行票收口该项验收）。整簇建议**一次性删除**，无需分批（无对外引用、无编译依赖、内部互引闭环，删除是纯零行为操作）。

---

## 5. 执行票切票建议

物理处置（改代码）归后续票，本票（#753）只产方案。建议切票顺序（先易后难，先孤儿后前代族，先族内后跨族）：

1. **票 N+1（孤儿簇整删，零行为，风险最低）**：删 §4 六单元（`cand_delta.rs`/`tower_cache.rs`/`sublevel.rs`/`classifier/incremental/`/`classifier/tests/`/`incremental_profile.rs`）。验收线：`cargo build --lib` 无新增错误 + `cargo test --lib theta_v0` 通过数不降 + wf8 金标准 `cmp=0`（沿 #745 验收模板）。**可独立于其余四族先行**，不阻塞任何族的处置。
2. **票 N+2（recursive_t(a) standalone T 算子复核+处置）**：先做一轮更深的"PyO3 外部可见性"复核（本票受限于亲手 grep 未做 C-ABI 全量穷举），确认 `run_recursive_t` 真无外部调用后，标 GUARD-ROLE 或直接删除（2611行，四族里体量最小、耦合最少，删除阻力最低，适合先手练手）。
3. **票 N+3（spiral/ + fugue_v3/ + recursive_t(b) 三族联合标记 GUARD-ROLE）**：三族互相耦合（fugue_v3 依赖 spiral 信号层，recursive_t(b) 依赖 fugue_v3 会计层），必须同批处理，不能拆票单独动一族——否则中间态编译失败。验收线：全部加 GUARD-ROLE 头标记（沿 #745 四要素模板：名分/对照什么/与现役差在哪/禁回灌），零行为不删代码，先建立"这三族已挂起待删"的机械可查状态。
4. **票 N+4（trading/ 28 个 v2 文件集中 legacy/）**：必须排在票 N+3 之后（等 spiral/fugue_v3/recursive_t 三族的去留定了，legacy/ 目录边界才不会再变）。执行时须显式排除 `center_book.rs::consume_death_certificate` 与整份 `third_point_book.rs`（§3 已标注，误删会破坏 #575/#637/#638 后续票落点）。
5. **票 N+5（顶层散件复核）**：待票 N+2~N+4 全部落地、四族真正清空或迁移后，顶层散件（bi_engine/segment/zhongshu/buysellpoint/…）才失去"四族依赖"的存在理由，此时单独开票复核是否仍需保留为对照件，或降格孤儿待删。**不要在四族未清空前动它**，会导致中间态编译崩溃。

---

*本报告基于亲手 grep/Read 核查，未使用 Task/子代理。全部命令与命中行号见正文各节代码块与引用；未在此报告中逐条粘贴的中间命令输出可用报告中给出的 grep 表达式在仓库当前 HEAD（`6e4193bf4e`）复现。*

## 订正（2026-07-29，#761 实装发现 + 影子评审坐实）

§1.3（recursive_t 族）判「standalone T 算子可独立删除」**有误**：遗漏反向依赖核查——standalone T 算子 6 文件（center/trend/divergence/operator/types/mod::iterate）被同目录 T 引擎（b）的**生产代码**（stream.rs/rec_stream.rs/backtest.rs 等）直接调用，删除会打断 (b) 编译。订正后处置 = **GUARD-ROLE 留档不删**（#761 已落地，零逻辑改动）；E3（#762）处置 T 引擎时须连同此基座一起裁（耦合同批原则覆盖到它）。
