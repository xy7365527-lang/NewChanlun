# #644 LEE 归因只读层接线 + wverify_run 死文件 + 余项——实施报告

- 日期：2026-07-29
- 工位：`/private/tmp/wt-644`，分支 `ticket-644`（off main tip，见 gitStatus 快照 `e7deab5f0c`）
- 参照面：`/private/tmp/kimi-nest-mainline`（全程只读，未 build/未 test，仅 `git show`/`Read`）
- 执行车：本会话（前台、单线程、禁子代理、禁后台任务，按票面纪律）
- 票据：#644；阻塞源 #693（已裁定②「接口形状=kimi 实装为准」）；阻塞 #755（level_order/level_risk sizing）
- 基线：`cargo test --lib` = **2594 passed / 0 failed / 138 ignored**（票面给定「2444/0/138」是开票时点快照，
  分支推进后实测为 2594——与 #614 合并报告 §0「移动靶」同一处境，按当前 tip 复算）
- 终态：`cargo test --lib` = **2595 passed / 0 failed / 138 ignored**（+1 = 本票新增 `lee_readonly_layer_does_not_perturb_net_result_644` 回归锁；**零新增红**）
- 额外验证（超出票面 `cargo test --lib` 要求，因发现真实回归而登记，见「意外发现」节）：
  `cargo check --all-targets` 与 `cargo check --all-targets --features backtest_bin` 均 **0 error**

---

## 边界声明（先讲清楚再看逐件）

本票只接**归因只读层**：`level_ledger`（M1）/ `level_clock`（M3 事件钟，只读累计）/ `level_attrib`
（归因算子，只读诊断）。**`level_order`/`level_risk`（M2/M4 sizing，物理订单量生成 + 级别风险帽）
不在本票**——kimi 侧 `pi_theta_fill_loop_overlay` 实际上把 `level_order`/`plan_level_gated_order`
**无条件**（非 `Option` 门控）内建在主循环里，调用它就等于同时接入 M2/M3 的订单流分叉
（kimi 自己的模块头原话：「M3 起订单流与 M0 分叉，不得借 M2 的 bit-exact 蒙混」）。这意味着
**不能照抄 kimi `pi_theta_fill_loop_overlay` 的整体结构**——本票的"语义重放"是从中**只挑出**
真正只读、不改 `order`/`cash`/`units` 的三段（level_ledger.step / level_clock.collect_ticks+observe /
新写的 level_attrib 诊断读数），显式跳过 `level_order`/`plan_level_gated_order` 那整段。

---

## 件 1：LEE 归因只读层接线

### kimi 位点 → main 重放位点

| 能力 | kimi 位点 | main 重放位点 | 处置 |
|---|---|---|---|
| M1 `LevelLedgerMirror` 参数线程 | `fill.rs:859`（`pi_theta_fill_loop_overlay` 签名新增 `level_ledger: Option<&mut LevelLedgerMirror>`，位于 `overlay` 与 `voice_exec` 之间） | `fill.rs:3417-3418`（同名同位——`overlay` 与 `voice_exec` 之间） | **移植（语义重放，接口形状=kimi）** |
| M1 mirror `.step()` | `fill.rs:1649-1667`（紧随 overlay `.step()` 之后，`if let Some(ll) = level_ledger.as_deref_mut()`） | `fill.rs`（overlay step 块后，`if let Some(ov) = overlay.as_deref() { ... ll.observe_lee_net(ov.net()) }`） | **移植，逐字同构** |
| M1 mirror `force_flat` | `fill.rs:1964-1968` | `fill.rs`（overlay force_flat 之后，同 `forced_flat_anchor`） | **移植** |
| M3 `collect_ticks`＋`LevelClockStats::observe` | `fill.rs:1570-1600` | `fill.rs`（`shadow_book.observe_and_compare` 之后、overlay step 之前） | **移植，逐字同构**（bsp_levels/closed/opened/silent/overlay/risk 六通道字段名与 main `coverage::StepTrace`/`interp::ActiveLeg`/`interp::Candidate` 完全同名同型，零改写） |
| M3 `plan_level_gated_order`（`level_order.regate` 门控 + 订单出口） | `fill.rs:639-850`、调用点 `fill.rs:1616-1632` | **不移植** | **越界排除（归 #755）**——这段直接改写 `*order`，是决策路径 |
| M2/M4 `level_order`/`level_risk` 局部变量与全部消费点 | `fill.rs:897-902` 等 | **不移植** | **越界排除（归 #755）** |
| M3 `pan_levels` 收集源 | kimi `pan_candidates`（kimi 自有的候选收集 Vec，main 无对应结构——main #282/#292 已删账面 apply/子腿叠加出口） | main 新写：在 main 现存 PanDivCert 首见+过门循环（`gate_pan_div_for_production` 通过分支）内 `pan_levels.push(lvl as u32)` | **接口形状对齐但取数源不同**——kimi 用其专属 `pan_candidates` 列表，main 直接在既有循环内联收集，语义等价（"首见并过门"判据相同），字段形状（`Vec<u32>`）与 `collect_ticks` 签名要求一致 |
| M2/M3 归因算子 `attribute_total` 的**只读**调用点 | **kimi 无此调用点**（kimi 仅在 `level_order::plan_gated` 内部消费 `attribute_total`，全程决策路径内） | main 新写：`step_trace` 产出后，`basis=level_nets(sep_legs,lot)`、`total=standard_p_star.round()`，调用 `attribute_total` 只累计 `residual`/`rescaled` 计数，`_out` 丢弃 | **无 kimi 对应位点，本票自行设计的只读诊断**（用已有的生产 M0 目标 `standard_p_star` 做归因分母，不产生任何决策副作用；理由见下） |

### 为什么 `level_attrib` 没有可字面重放的位点

`strategy::level_attrib::attribute_total` 在 kimi 树里**唯一**的调用点是 `level_order::plan_gated`
内部（决策路径），fill.rs 从未独立调用它。要满足票面「level_attrib 逐 bar 归因记账」的字面要求
同时不越过只读边界，本票新增了一处独立只读诊断：把生产 M0 已经算出的净额目标
`standard_p_star`（`pi_theta_step_traced_with_risk_seeds` 返回值，此前在 main 是**未消费的死值**，
`rustc` 曾报 `unused variable: standard_p_star`）按 `level_nets(sep_legs)` 结构基准做归因分解，
只统计「决策点数 / 落残差桶次数 / 经比例缩放次数」三个计数，`out` 本身丢弃不落任何账本。

### 是否读、是否改——逐条自证

- 不读 `order`：三段代码均不引用 `order` 变量。
- 不写 `order`/`cash`/`units`/`p_t`：`grep` 确认三段代码体内无对这些变量的赋值。
- `level_ledger=None` ⟹ 整段 `if let Some(...)` 跳过，bit-exact 回归锁（kimi 原有纪律，main 逐字保留）。
- `level_clock`/`level_attrib` 两段本身**无 `Option` 门控**（kimi 的 `level_clock_stats` 同样无门控，
  纯 O(1) 累加，不构成成本/风险）；但两段读数不被任何下游决策消费（`clock_ticks` 只喂
  `level_clock_stats.observe`，**不**喂 `ticks.ticked_levels()` 去门控任何目标重估）。

### 机器可查证据（非仅 prose）

新增单测 `runner.rs::lee_readonly_layer_does_not_perturb_net_result_644`：60-bar 合成锯齿数据上，
`run_theta_v0_pi_overlay`（level_ledger 已接线）与纯净额 `run_theta_v0_pi` 的 `n_orders`/`trades`
（逐字段 `PartialEq`）/`strat_return` **逐位相等**；同时断言 `level_ledger`/`level_clock` 确有非零
观测（非死代码），`level_clock.n_decisions == level_attrib_n_bars`（两个只读累计器必须在同一
tradable-bar 守卫下产生相同决策点计数——结构自洽性锁）。原有 `run_theta_v0_pi_overlay_reconciles_and_bit_exact_net`
用例在本次接线后仍绿，进一步印证 overlay 臂本身也未被本次改动扰动。

---

## 件 2：wverify_run 死文件——逐个判定

`backtest/mod.rs:63` 只 `mod wverify_run;`（指向 `wverify_run.rs` 文件，`required = ["backtest_bin"]`
无关——lib target 默认编译），`wverify_run/` 目录下四个文件**无任何 `mod`/`#[path]` 声明**引用
（全仓 `grep` 确认零消费方），Rust 不编译未声明模块，故留树不影响编译/测试——纯并线债。

| 文件 | 判定 | 理由 |
|---|---|---|
| `report.rs` | **清理（已删除）** | main `wverify_run.rs:35/46` 已含**逐字同名**的 `force_state_code`/`force_state_label`——纯重复，零净增能力 |
| `m8.rs` | **清理（已删除）** | main `wverify_run.rs:918/927` 已含**逐字同名**的 `q4_shift_back_6m`/`q4_prev_day`；M8 e2e OOS 报表能力已内联在 main 自有的 `m8_e2e_all_systems_oos`（`wverify_run.rs:1206`）——kimi 侧是同一功能的另一种模块组织（拆分 vs 内联），非新能力，与 ⚠HIGH-2（classifier 内联/拆分二选一，另开票裁定）同类，本票范围内按「main 已是更大工作量胜方」处置 |
| `tests.rs` | **清理（已删除）** | 首行 `use super::m8::*` 硬依赖 kimi 侧 `m8.rs` 的目录化模块布局，`m8.rs` 已判清理 ⟹ 该文件失去宿主，无法独立启用 |
| `issue71_chi_gamma.rs` | **清理（已删除）** | 唯一**非纯重复**的一个（真实 BTC arm0-3 χ/OpsemDump/GammaDump 隔离验证 harness，main 无对应能力）。但：① 其验证对象 χ 门本身已被 `chanlun/escalate/chi-line-falsification-ruling-20260728.md` 正式撤销（文件自己头部承认）；② main 已通过 ⚠MED-5（#614 合并时手工移植）拥有 `GammaDump`/`OpsemDump` 钩子的**独立**测试覆盖（`gamma_dump.rs::gamma_dump_chi_consistency`/`gamma_dump_env_gated_bit_exact`/`gamma_dump_schema_on_synthetic` 等，本次 `cargo test --lib` 内已验证绿）；③ 启用需先核对 main 内部 API（`OpsemDump`/`GammaDump`/`ChiFilterCtx` 等）签名是否与 kimi 版本兼容，属独立工程量。三条合起来：保留价值边际，启用成本不小，判**清理**；「历史证据不因仪器撤销而失效」的纪律由 git 历史 + kimi-nest-mainline 树自身满足，不需要本票在 main 里再留一份不会被执行的死拷贝 |

删除后 `strategy/coverage/sizing.rs:251` 原有一处指向 `wverify_run/m8.rs`、`report.rs` 的 doc 引用
一并订正（改为指向 main 自有 `m8_e2e_all_systems_oos` 等价能力，不再指向已删文件）。

---

## 件 3：余项 hunks——逐项判定

### #571 四类 carrier-only 关闭事件统一 drain → **放弃移植，登记为独立跟进项**

调查发现：main **已经有**自己独立的 #571 实装（`open_ledger.rs`：`OpenTable`/`OpenKey{carrier,generation}`/
`drain_carrier`/`settle_reverse`/`settle_silent`/`settle_risk`/`settle_overlay`，`mod open_ledger;`
在 `backtest/mod.rs:83` 已声明），但**这套机制没有被 fill.rs 实际消费**——`fill.rs:3552` 的
`open_trades` 仍是裸 `HashMap<ElementId, LedgerOpen>`，四类关闭事件循环（`closed`/`silent_drops`/
`risk_exits`/`overlay_closes`）用的都是 `.remove(&leg.id)`，未走 `drain_carrier`。

**这不是遗漏，是 main 自己已经裁决过的设计**：`fill.rs:3590-3594` 的注释明确记载「同一 carrier
close→reopen 时 `open_trades.insert` 二次覆盖」这个确切场景，并引用 **`codex a9-posnode 裁定 C`**
——main 选择的解法是 `gen_hiwater` 高水位表让 `position_node_id` 四元组不碰撞（下游身份唯一），
容忍 `open_trades` 本身被覆盖，依赖 #200「先平后开」的同 bar 施加顺序保证不会出现两个「活」的
同 carrier 条目同时需要区分。

kimi 的 `drain_carrier` 式重写是对这条**已裁决**架构的正面复议（要不要引入 generation 维的
carrier 隔离表），不是"接一条只读旁路"，是改写决策路径核心的仓位归属账本——直接越出 #644
"只读接线"边界，且需要先证明/推翻 `codex a9-posnode 裁定 C` 的适用范围。**判定：不在本票移植，
登记为独立跟进项**（建议单独开票，范围＝#571 kimi 版 drain_carrier 语义 vs main 版 codex a9-posnode
裁定 C 的等价性/优劣裁定）。

### #289 LOW-1 窗口终点单源化 → **移植（已完成）**

kimi `fill.rs:1949-1968`：`forced_flat_anchor`（`(last_i, last_px)` 单源计算）供 overlay 与 LEE M1
镜像两处 force_flat 共用，避免「两段代码逐字重复同一推导，任一侧口径改动会静默漂移」。**本票在件 1
接线时恰好自己引入了这个反模式**（level_ledger 的 force_flat 与 overlay 的 force_flat 各自重算了
一次 `(0..n).rev().find(...)`）——已在同一批改动内用 kimi 原版写法订正：单源 `forced_flat_anchor`，
overlay 与 level_ledger 两处 `if let Some((last_i, last_px)) = forced_flat_anchor` 共用。**纯重构，
零行为改变**（`cargo test --lib` 前后计数一致）。

### runner.rs h12（970 行端到端做空腿测试）→ **放弃，登记理由**

kimi `runner.rs:6502-6526`（`short_leg_end_to_end_trade_marked_short`）测试目标是
`plan_and_fill_mtm`（v1 引擎 API）。核实：main 自己的 `#499`（`94e0837f0e refactor(backtest): #499
v1/dual 家族退役——删两代死引擎`）**已经是 main HEAD 的祖先**，`rust/src/theta_v0/backtest/runner.rs`
里 `grep -n "fn plan_and_fill_mtm"` **零命中**——v1 引擎在 main 侧也已被独立退役。移植目标函数在
两侧都不存在，**不是"取舍"问题，是移植对象已消失**。判定：**放弃**，无需另开票（v1 退役是双线
已经趋同的既成事实，同 #614 §8 记录的「d9f1860124（#631）level_origin 空转字段处置」一类"两线本已
趋同"情形）。

---

## 件 4：对拍（归因读数与 kimi 侧）

### 窗口/数据选择

BTC，1 分钟粒度，`2017-08-17 .. 2017-08-31`（**21360 bar**，满足票面「20k 起步」）。选择理由：
① 数据集首段，覆盖历史最早、波动明显的窗口（首 10 条离场声部 pnl_v 幅度均 > 10 万，非空转）；
② 避免选取需要额外论证代表性的"精选窗"，用数据集起点做可复现、无争议的默认选择。

### 为什么不能逐值对齐 kimi 侧数字

`kimi-nest-mainline` 是**封存只读**参照面，本票纪律禁止在其上 `cargo build`/`test`（会写
`target/`）。退一步：即便能跑，**逐值对齐本身也没有意义**——kimi 的 `run_theta_v0_pi_overlay`
无条件内建 `level_order`/`plan_level_gated_order`（M2/M3 决策路径），其净额订单流从 M3 起**按设计**
与 M0 分叉；main 侧本票严格排除这条路径，净额路径保持与纯 M0 bit-exact。两棵树在"要不要接
M2/M3 决策门控"这个分岔点上就已经不同源，逐值比较两侧的 `LEE-Net`/`clock`/`attrib` 数字没有对照
意义（分母决策路径都不同）。

### 自定等价口径：结构不变量复现，而非数值对齐

比较改为「同一组结构性断言在两侧是否同样成立」：

| 不变量 | kimi 侧声明（模块头/测试） | main 侧实测（本次 BTC 21360-bar 跑批） |
|---|---|---|
| LEE-Net 恒等 `Σ_ℓ net_ℓ ≡ N`（M1，构造性 L1） | `level_ledger.rs` 测试族在合成数据上逐 step 精确成立（整数手数求和，非 eps） | `obs=14386 max residual=0 max\|N\|=15603` ⟹ **PASS**（残差恒 0 且非平凡） |
| clock_ℓ 稀疏度非退化（M3，kimi 自称仅 L1——**从未在真实行情验证**，`level_clock.rs` 模块头原话「L2/L3 一律缺席：本票未在任何真实行情上验证 M3」） | 仅合成 `random_walk_dataset` 上验证稀疏区间 `0 < n_bars_with_structural_tick < n_decisions` | `14386 决策点 / 106 结构钟 / 11 风控钟` ⟹ **PASS**（响过且严格稀疏，0.74% 结构钟命中率）——**本次是该不变量首次在真实 BTC 数据上得到验证**，超出 kimi 自己认领的认识论等级 |
| 归因算子 `Σ_ℓ out_ℓ ≡ total` 整数精确（`attribute_total`，L0 构造性） | 单测覆盖全部三分支（恒等/缩放/残差桶），无经验数据要求 | `14386 决策点 / 0 落残差桶 / 2533 经比例缩放`（17.6% 的 bar 上 `Σbasis ≠ total`，触发比例缩放但从未触发"无结构基准可归因"的残差桶）——**PASS**（算子本身是全函数，恒等式在真实数据上同样精确成立；比例缩放的非零占比说明诊断有信息量，不是平凡空转） |

三条不变量在 main 只读层的真实 BTC 数据上**全部复现**，且 clock 不变量的验证等级（真实数据 L2）
**超过**了 kimi 自己在其原始实现里认领的等级（合成数据 L1）——这是本票读层接线相对 kimi 原始实装
的一个净增量，如实登记（非声明 alpha，纯管线/构造性验证等级提升）。

### 对拍读数字段清单（哪几个字段比、容差）

- `LeeNetWitness.{n_observations, max_abs_residual, max_abs_net}`：`max_abs_residual` 要求**恰好 0**
  （整数精确，非容差比较）；`max_abs_net` 只要求 `> 0`（非平凡性）。
- `LevelClockStats.{n_decisions, n_bars_with_structural_tick, n_bars_with_risk_tick}`：
  `sparsity_witnessed()` 要求 `0 < n_bars_with_structural_tick < n_decisions`（严格双侧，无容差）。
- `level_attrib_n_bars / level_attrib_n_residual_bars / level_attrib_n_rescaled_bars`：无预设阈值，
  纯观测登记（`attribute_total` 本身是构造性全函数，`Σ_ℓ out_ℓ ≡ total` 是逐 bar 内部精确恒等，
  不需要跨 bar 统计容差）。

---

## 接口冲突清单（与 main #355/#363/#369/#376 文档承诺对照）

`#355`/`#363`/`#369`/`#376` 四票经 `git log --oneline --all | grep "#<n>"` 核实，全部集中在
`level_order.rs`（M4 `cap_narrowed_levels` 逐级稀疏性判据/witness 文档），**零一处触及**
`level_ledger.rs`/`level_clock.rs`/`level_attrib.rs`。本票只接读层（M1/M3 只读/归因算子），
与这四票的文档承诺**零交叠、零冲突**——它们的接口出入点问题属于 `level_order`/M4 sizing 域，
应在 #755（level_order/level_risk 接线票）里核对，不在本票范围内。**接口冲突清单条数 = 0**。

与 #668（classifier 三文件，在飞）：本票改动面 = `backtest/fill.rs` + `backtest/runner.rs` +
`bin/theta_overlay.rs` + `strategy/coverage/sizing.rs`（1 行 doc）+ `wverify_run/` 四文件删除，
零触碰 `classifier/` 任一文件——**零交叠**。

---

## 意外发现（超出票面直接要求，一并处置）

`bin/theta_overlay.rs` 是 #614 合并时**整体取自 kimi 侧**的文件（`git log` 唯一相关提交即合并
提交本身），其中 `r.level_ledger`/`r.level_order`（M1/M2 结果包字段）在合并后的 main
`OverlayRunResult` 里**从未存在过**——`cargo check --all-targets --features backtest_bin`
（main CI `rust-check` job 的第二步）在本票开工前**处于红色**（`E0609: no field level_order`），
只是因为该 bin 需要 `required-features = ["backtest_bin"]`，默认 `cargo build --lib`/`cargo test --lib`
不编译它，故未在日常验收中暴露。本票接线后 `level_ledger`/`level_clock` 两字段满足了该文件的
一半引用，但 `level_order`（M2 sizing，本票不接）仍缺——按 #644 边界移除了该 CLI 输出里的
"LEE M2 订单归因"打印段（消费 `level_order` 的部分），改为打印本票新接的 M1/M3/归因诊断三段。
**`cargo check --all-targets` 与 `--features backtest_bin` 两条 CI 腿现均 0 error**（订正前置于本票
commit 之内，随「接线」批一并提交，理由：不修就没法验证接线，且这是票面「level_* 行为面有靶向
前后对照」验收条要求路过的必经门）。

---

## Commit 清单

见 `git log` ticket-644 分支，四组提交（接线 / 死文件 / 余项 / 报告），均带 `#644` 标记。
