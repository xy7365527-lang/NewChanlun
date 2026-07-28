# rust 侧 orphan 逐件过五档树清点（#413）

- 日期：2026-07-27
- 票：#413（`wayfinder:research`，Part of map #59）
- 判据来源：#225 五档树（**不重裁，直接套用**）
- 工作面：worktree `/tmp/kimi-nest-mainline`；主仓零写入；全程只读，零 git mutation
- 扫描目标树：**`main`**（trunk = 「已入库」的权威面），经 `git archive main` 只读导出到 `/tmp`
- 交叉面：worktree 分支 `kimi-nest-mainline-20260717`（HEAD `9e3cd25fd6`，相对 main 76 ahead / 117 behind）

---

## 0. 结论先行

**除已知的 `NestLifecycleBook` 一件外，另查出 11 件「已入库但无生产调用点」的 rust 资产，合计 12 件。**

用大白话说：这 12 处代码都已经提交进仓库、也都能编译，但**整个程序跑起来的时候没有任何一条路径会走到它们**。它们分三种情况：

1. **其实在干活（5 件）**——它们自己带的测试会去调用生产代码并逐项核对，等于是「生产代码的体检仪」。生产改坏了这些测试会红。这类不是孤儿，登记即完。
2. **真的闲置、且端到端确实缺这一块（4 件）**——该接线。其中 1 件已知（#402/#409 在跑）、1 件是 map #59 明列组件的正项（应新出接线票）、2 件已有归属票或需先裁范围。
3. **真的闲置、但不在路线图上（3 件）**——留档不装。

**「删」档 0 件资产**（无任何「留着会误导后来人」的实体资产）。另发现**误导性文字 1 处**（`nautilus/mod.rs` 模块头自称「本模块不编译进 crate」，实际早已注册），按 #225 属订正范畴，是否按删档处理须编排者终审。

**「前瞻」档 0 件，且这是结构性的**：#225 前瞻档要求「Lean 已证 + **rust 未实装**」，而本票范围按定义就是「rust 已实装但无调用点」——两者互斥。⟹ 前瞻档在 rust 侧清点上必然空集，不是漏查。

### 档位分布

| 档 | 件数 | 明细 |
|---|---|---|
| 1 留 | 5 | six_state / force_conformance / closed_loop·conformance / strategy·mutex（边界） / nautilus·IntegrationPath |
| 2 接 | 4 | **nest_lifecycle（已知已判）** / **interval_necessity（建议新出票）** / center_lifecycle·terminates_suspension（已有 #292 归属） / first_retrace_replay（边界，需先裁范围） |
| 3 前瞻 | 0 | 结构性空集（见上） |
| 4 归档 | 3 | voice_eat（边界） / theta_v0·complete 子树（边界） / unified_necessity·T55（已有用户裁决） |
| 5 删 | 0 | 资产 0 件；误导性文字 1 处待编排者终审 |

**边界组 4 件 + 主干-分支口径分歧 1 项**，见 §4，未强行定档。

---

## 1. 清点口径（含从已知件校准的过程）

### 1.1 三条识别口径并用（票体 §「要产出的东西」）

| 口径 | 实施 |
|---|---|
| ① 零引用 | 全量提取 `rust/src` 每个文件**顶层**（第 0 列）`pub fn/struct/enum/trait/const/static/type`，对每个名字在 `rust/src` + `rust/tests` 全量 token 索引里查外部出现。**先剥注释与字符串字面量**再建索引（票体明载「排除仅出现在注释里的命中」——本次实测有 3 件正是这么差点被误判为「有调用」：`force_conformance::Force`、`nautilus::IntegrationPath`、`nest_lifecycle::NestLifecycleBook`，它们在别处的唯一出现全是 doc 注释里的 `[`链接`]`）。 |
| ② 自述措辞 | `grep` 「出切片 / 接线待定 / 参照实装 / 生产 bin 接线 / 未接线 / 无生产调用者 / 尚未接入 / 待接入」等 15 个变体。 |
| ③ dead_code | `grep 'allow(dead_code)'` 全量 25 处逐条过；「仅为躲警告而 pub」的模块级形态见 §1.3。 |

### 1.2 校准（口径正确性的自证）

第一版扫描把**所有** `pub` 项（含 `impl` 块内方法）都算进去，结果 `nest_lifecycle.rs` **没有**出现在孤儿名单里——因为它的 `impl` 方法叫 `new`/`get`/`len`/`is_empty`/`event`/`entries`，这些名字在全库到处都是，造成假的「有调用」。

改为**只取顶层项**后，`nest_lifecycle.rs` 的 15 个顶层项（`LifecycleKey` L79 … `NestLifecycleBook` L496 … `provide_pan_live_windows` L862）**全部零外部引用**，与票体已知结论一致 ⟹ **口径校准通过**。本报告全部结论建立在这一口径上。

### 1.3 为什么这些会长期藏住（机制说明）

Rust 编译器对 **lib crate 里的 `pub` 项不报 `dead_code`**——它假定这些是给外部用户用的 API。本仓 `rust` 是 lib crate（`crate-type = ["cdylib","rlib"]`），所有这 12 件都在 `pub mod` 链上 ⟹ **编译器永远不会警告**。这就是票体口径③「仅为躲 dead_code 警告而 pub」的模块级形态：整个模块 `pub` 出去，警告自动消失，人也就看不见了。这解释了为什么 `NestLifecycleBook` 是「偶然撞出来的」——没有任何自动信号会指出它。

### 1.4 「活消费方」的判定线（从已知件反推，明示以便复核）

#225 档1 的「有活消费方（rust 测试 / fixture / 生产引用在跑）」需要一条落地线，因为本票 12 件里有 10 件**自带 `#[cfg(test)]`**。已知件给了这条线：

- `NestLifecycleBook` 自带 11 个测试且全绿，却被判**档2「接」而非档1「留」** ⟹ **同文件内只验证自身的测试，不算活消费方**。
- 反过来，若模块的 `#[cfg(test)]` **跨模块调用生产函数并断言二者一致**（差分 / parity / conformance 守卫），则该模块是「生产代码的活检查器」，生产回归会打红它 ⟹ 有真实活性 ⟹ **档1「留」**。

本报告对每件都标注它属于哪一侧，证据 = 其 test mod 的 `use` 行与断言对象。**这条线是我从已知件推出的应用规则，不是 #225 原文**——若编排者认为「凡在跑的测试都算活消费方」，则 §3 的 4 件「留」不变，但已知件 `nest_lifecycle` 的「接」判也须一并复议（两者同构）。

### 1.5 覆盖范围与非目标

- 覆盖：`rust/src` 全部 247 个 `.rs`（含 37 个 `bin/`），`rust/tests` 8 个集成测试文件。
- 消费方也查了 `rust/` 之外：`git grep` 12 件的全部符号名到全仓非 rust 路径，命中 **30 个文件全部是 `.md` 报告 / `.jsonl` 事件流**，**零代码消费方**（无 Python / pyo3 / 脚本调用）。
- **非目标**：函数体内的局部死代码、`impl` 块内单个未用方法的长尾。后者数量在百件量级（多为 helper / 结构对称保留字段），逐件登记会淹没信号；本报告只在其**自述措辞或 `#[allow(dead_code)]` 命中**时收录（即口径②③命中者，见 §3 的第 10–12 件）。这是刻意的范围裁剪，登记在此以免被误读为「已穷尽到方法粒度」。

---

## 2. 逐件清单 + 定档（总表）

行号均为 `main` 树 `rust/src/` 下的位置。

| # | 资产 | 位置 | 规模 | 档 | 一句话理由 |
|---|---|---|---|---|---|
| 1 | `NestLifecycleBook`（活假设状态机） | `theta_v0/classifier/nest_lifecycle.rs:496` | 1804 行 | **接** | 已知件；Destination 明列组件，接线三题 #402 / 探针 #409 在跑 |
| 2 | 区间套必要条件塔内原生 | `theta_v0/classifier/interval_necessity.rs:34/52/67/130` | 257 行 | **接** | Destination「N3 塔内原生」正项，已实现只差回写裁定 ⟹ **建议新出接线票** |
| 3 | `CenterLifecycleEvent::terminates_suspension` | `theta_v0/classifier/center_lifecycle.rs:426` | 1 方法 | **接** | 自述「本方法当前无生产调用者，见票面边界」，动作归 #292 ⟹ 已有归属票，不另出 |
| 4 | D7 firstRetrace 只读复核原语 | `theta_v0/classifier/first_retrace_replay.rs:11–137`（9 项） | 571 行 | **接（边界）** | 与 Destination「买卖点生命周期」相邻但非明列项 ⟹ 须先裁范围，见 §4 |
| 5 | 六态/信号位 canonical 对照层 | `theta_v0/classifier/six_state.rs:29/42/54/70/84` | 264 行 | **留** | 其测试调生产 `rlevel_of`/`endpoint_to_bsp` 做 parity 断言 = 活体检仪 |
| 6 | 力度 conformance 适配器 | `theta_v0/classifier/force_conformance.rs:45/60` | 246 行 | **留** | 其测试调生产 `divergence::segment_macd_area`/`segments_diverge` 验 mono/faithful 公理 |
| 7 | `ThetaV0Contract` 逐态 conformance | `theta_v0/closed_loop/conformance.rs:68` | 276 行 | **留** | 其测试跑生产 `hybrid_step_baseline`/`policy_output`/`transition_adapter` 逐态比对 |
| 8 | 全互斥买卖点解释器 oracle | `theta_v0/strategy/mutex.rs:79/90/145/164/203/214/235` | 708 行 | **留（边界）** | 模块自述「零生产消费者，保留用途 = 对拍参照」+ 已有 codex-q2-d1 §Q2 裁定；但 main 上连 oracle 的外部消费方也已消失，见 §4 |
| 9 | `IntegrationPath` 路径选择枚举 | `theta_v0/nautilus/mod.rs:81` | 1 枚举 | **留** | 自述「enum 占位，保留仅为设计文档锚点，对齐四分法『选择』类」= 有意的待裁定锚 |
| 10 | 完整状态/事件 schema 子树 | `theta_v0/complete/{mod,state,event}.rs` | 650 行（3 文件） | **归档（边界）** | 整棵子树零外部消费；FULL §3/§20 零遗漏对照的 L0 schema 镜像，不在 Destination 清单上 |
| 11 | C06 声部吃到判定 `Eat(v,b)` | `theta_v0/classifier/voice_eat.rs:54/81/108/127/162` | 382 行 | **归档（边界）** | spec C06 定义镜像，不在 Destination 清单上；但有一个测试断言了生产塔的性质，见 §4 |
| 12 | T55 比价双线定位观测 | `trading/unified_necessity.rs:848/861/881` | ~55 行（3 项） | **归档** | 自带注释「M2 未接入；`#[allow(dead_code)]` = **用户裁决『代码先写，等 M2 接入』**」⟹ 已有裁定，不重裁 |

**删档：0 件资产。** 误导性文字 1 处，见 §5。

---

## 3. 逐件证据

### 第 1 件 · `NestLifecycleBook`（已知件，仅作口径锚，不重查）

`theta_v0/classifier/nest_lifecycle.rs`，1804 行，15 个顶层项全部零外部代码引用。全库仅 2 处出现 `NestLifecycleBook` 字样：

```
rust/src/theta_v0/classifier/mod.rs:79:  /// V3 活假设状态机：NestLifecycleBook sidecar 注册表（...）   ← 注释
rust/src/theta_v0/classifier/nest_lifecycle.rs:496: pub struct NestLifecycleBook {  ← 定义自身
```

自述措辞（口径②，模块头 L9/L21/L47/L49）：「生产 bin 接线（卡 §6.2 伪码的 prefix 循环投产）**出切片**」「Unresolved **出切片**」「`provide_pan_live_windows` … 是调用方按 `PanLiveWindow` 契约喂入的**参照实装**」。入库 commit `9dbf339f1f`（#231 重建）。档 = **接**，下游 #402 / #409 已在跑。

---

### 第 2 件 · `interval_necessity.rs` —— 本票最实的新发现，档「接」

**位置**：`theta_v0/classifier/interval_necessity.rs`，257 行，入库 commit `6075840687`（task-106）。

**零调用证据**：4 个顶层项 `IntervalNecessity`(L34) / `IntervalNecessitySummary`(L52) / `interval_necessity_tower`(L67) / `summarize`(L130) 全部零外部引用。`git grep -w interval_necessity_tower main -- rust/` 的 5 条命中**全在本文件内**（L67 定义 + L198/221/234/252 自测）。另单独核查了同期探针 `bin/p108_interval_probe.rs`——**不引用它**（`grep 'interval_necessity\|IntervalNecessity' bin/p108_interval_probe.rs` 空）。

**对照 Destination 的判断依据**：map #59 Destination 明列组件清单第二项 = **「N3 塔内原生」**。本模块头第一行即：

> 「# 区间套必要条件——**递归塔原生实现**（条款 9，任务 #106）」
> 「cert-bsp-binding-ruling 条款 9：区间套必要条件应在**递归塔内部原生实现**（次级别构件、塔内时钟）；nest 管线保持独立对照实现身份」

⟹ 这是 Destination 组件清单上的**正项**，不是相邻项。四条档2 判据逐条过：无消费方 ✓ / 对端到端有补充相 ✓（N3 明列）/ rust 侧已实现 ✓（257 行含 4 个自测全绿）/ 可近期接线 ✓（只消费 `Classification`，无新依赖）。

**接线前须解的已知悬置**（模块头自陈，接线票应承接而非绕过）：

> 「这是**必要条件的检查器**，不是过滤器：verdict 不回写 `BspBits`、不改任何判据 bit（#102 归因未定前，级别归属结论不得用于方向不对称策略——开放条款）。**过滤/硬门须另经主人裁定**。」

⟹ 接线票的第一题就是「接成只读观测量，还是接成硬门」，后者需编排者裁定。

**⚠ 树面提醒**：本文件**只存在于 main**，worktree 分支 `kimi-nest-mainline-20260717` 无此文件。接线工作面须以 main 为准或先合流。

---

### 第 3 件 · `center_lifecycle.rs::terminates_suspension`，档「接」（已有归属，不另出票）

**位置**：`theta_v0/classifier/center_lifecycle.rs:426`。

**证据**（口径②，自述措辞）：模块头 L44 与方法 doc L425 两处明写：

```
//! 判据与事件形态在本模块立；**动作（#292 减补/挂起）本票不接**
//!   （[`CenterLifecycleEvent::terminates_suspension`] 当前无生产调用者）。
```
```
    /// **本票只立判据与事件形态，不接 #292 动作**（本方法当前无生产调用者，见票面边界）。
    pub fn terminates_suspension(&self) -> bool {
```

`grep -w terminates_suspension` 全库 3 条命中：L44 注释、L426 定义、L1277 本文件自测。**零生产调用点属实**。

**定档**：档2「接」，但**接线归属已经存在**（#292 减补/挂起动作票），本票不另出接线票，登记即可。所在模块 `center_lifecycle.rs` 其余部分（`CenterEventMachine`/`CenterLifecycleEvent`/`ChainConsumed` 等）**有活生产消费方**，属档1，不在本清单。

---

### 第 4 件 · `first_retrace_replay.rs`，档「接（边界）」

**位置**：`theta_v0/classifier/first_retrace_replay.rs`，571 行，入库 commit `e942e1d3c1`（#76 D7 例2）。

**零调用证据**：9 个顶层项 `StrictCompletedPair`(L11) / `StrictPairError`(L17) / `strict_completed_pair`(L35) / `RetraceIdentity`(L89) / `RetraceOutcome`(L95) / `RetraceObservation`(L101) / `RetraceLifecycleEvent`(L107) / `RetraceLifecycleError`(L125) / `replay_first_retrace`(L137) 全部零外部引用，且**连注释里都无外部提及**（`ext_raw_refs = []`，比第 1/6/9 件更彻底）。

**test mod 性质**：其 6 个 `lv_case2_*` 测试用生产类型（`LevelAsOfView`/`MoveBlock`/`LeveledMove`）**构造夹具**，但断言对象全是本模块自己的 `strict_completed_pair`/`replay_first_retrace` 返回值 ⟹ 按 §1.4 的线，**不算活消费方**，与已知件同型。

**对照 Destination**：模块自述是「D7 `firstRetrace` **只读复核原语**」，做的是 seed 身份的 `RETEST_REENTERS / SUPERSEDE / RESTART` 纪律复演——这与 Destination 第八项「买卖点生命周期（接线）」**语义相邻**，但 D7/firstRetrace 并非该项的明文内容。

⟹ **落「接」但标边界**：档2 四条里前三条（无消费方 / rust 已实现 / 可近期接线）都成立，第四条「对端到端有补充相」取决于「买卖点生命周期接线」的范围界定，我无权自裁。见 §4-B。

---

### 第 5 件 · `six_state.rs`，档「留」

**位置**：`theta_v0/classifier/six_state.rs`，264 行，入库 `42dd08d3d3`。5 个顶层项零外部引用。

**为什么是「留」而非「接」**：其 test mod 直接 `use super::level_state::{rlevel_of, RContext, RLevel}` 与 `super::bsp::{endpoint_to_bsp, EndpointSituation}`（均为**生产函数**），13 个测试里 8 个名为 `*_parity`，逐条断言「生产 `rlevel_of` 的输出 == Lean `TrendSixState` 构造子的 canonical 名」。模块头自陈其身份：

> 「本模块**不重复实装**——它建立 rust 实装 ↔ Lean `Origin.TrendSixState` 的 **canonical 命名对照 + parity 断言**」

⟹ 生产侧 `RLevel` 顺序/语义一改，这些测试立刻红。它是**生产的活体检仪**，不是待接的零件。**假孤岛滤出，登记即完。**

---

### 第 6 件 · `force_conformance.rs`，档「留」

**位置**：`theta_v0/classifier/force_conformance.rs`，246 行，入库 `4318b4efad`。2 个顶层项 `Force`(L45)/`ForceMeasureAdapter`(L60) 零外部引用。

**注意反面例证（口径①的注释陷阱）**：`Force` 一词在 `backtest/mu_estimator.rs`、`classifier/divergence.rs`、`classifier/mod.rs` 三处出现——**全在注释里**。若不剥注释就会判成「有 3 个调用点」而漏掉本件。这正是票体点名的那类误判。

**为什么是「留」**：文件级 `use super::divergence;`（生产模块），8 个测试里 `measure_equals_segment_area` / `is_divergence_via_matches_segments_diverge` / `mono_axiom_holds_on_real_macd` / `faithful_axiom_holds_on_real_macd` 直接把生产 `divergence::segment_macd_area`/`segments_diverge` 喂进去验 Lean `ForceMeasure` 的 mono/faithful 公理。生产背驰口径一变即红 ⟹ 活体检仪。

---

### 第 7 件 · `closed_loop/conformance.rs`，档「留」

**位置**：`theta_v0/closed_loop/conformance.rs`，276 行，入库 `4318b4efad`。唯一顶层项 `ThetaV0Contract`(L68) 零外部引用。

**为什么是「留」**：文件级 `use super::transition::{hybrid_step_baseline, AssemblyEvent}` + `use super::super::strategy::intent::{classify_adapter, ClassLabel}`（全生产）。6 个测试 `step_spec_total_unique_per_state` / `classify_total_unique_per_state` / `step_factors_through_policy_then_transition_per_event` / `dual_ledger_writeback_invariants_per_event` / `full_trace_state_by_state_reproducible` 逐态跑生产闭环并比对。模块头自陈这是兑现 Lean `EngineBridge.RustEngineContract` 的「满足契约」逐态证据 ⟹ 活 conformance 守卫。

---

### 第 8 件 · `strategy/mutex.rs`，档「留（边界）」

**位置**：`theta_v0/strategy/mutex.rs`，708 行，入库 `ff88d81bc9`。7 个顶层项（`MutexClass` L79 / `Predicates` L90 / `mutex_class` L145 / `StepPredicateCtx` L164 / `ActionBucket` L203 / `bridge_bucket` L214 / `predicates_of` L235）零外部引用。（第 8 项 `const M` L73 是单字母 token，全库误命中 42 文件，已作假阳性剔除。）

**口径②自述命中，且已有裁定**（模块头 L62–L67）：

> 「## 身份：D1 等价测试 oracle，**非生产路径**（codex-q2-d1 §Q2 裁定 + 裁定4 反装饰约束）
> 本模块 `mutex_class` **零生产消费者**——生产裁决走 interp fold + I_Θ 组合层。保留用途 = P1..P10 等价 property test 对拍参照。」

⟹ 它「无生产调用点」是**设计意图，且经 codex-q2-d1 §Q2 正式裁定**。#225 判据不重裁，此处同理：既有裁定不重裁。

**「留」的活性证据**：test mod L163 `use super::interp::{interpret, theta_key}` + L162 `use super::exec::reverse_signal`（生产），`shadow_fold_bucket_equivalence` 做 oracle-vs-生产的差分对拍。

**边界所在**：见 §4-A——main 上它的**外部消费方已经消失**（分支上还在）。

---

### 第 9 件 · `nautilus/mod.rs::IntegrationPath`，档「留」（含误导性文字 1 处）

**位置**：`theta_v0/nautilus/mod.rs:81`，入库 `9c70c4494f`。

**零调用证据**：全库 2 处命中，一处是 `theta_v0/mod.rs:100` 的**注释**（`见 mod 头 IntegrationPath`），一处是 L81 定义自身。

**为什么是「留」**：其 doc 明写这是**有意的**设计锚：

> 「★这是 enum 占位，**不是运行时分支**——路径选择是构建期的依赖/crate 拓扑决策…保留此 enum 仅为设计文档锚点（让骨架显式记录待裁定的选择点，对齐四分法「选择」类）。」

⟹ 「记录一个待裁定的选择点」本身就是它的用途，删/归档都会丢掉这个记录。档1 登记即完。

**注意**：本件是**项级**孤儿。所在模块 `theta_v0/nautilus/` 的其余部分（`bar_adapter`/`theta_strategy`/`backtest_engine`/`order_adapter`/`strategy`）**有活消费方**（`bin/theta_backtest.rs` 经 `backtest_bin` feature 链接），不是孤岛。

**⚠ 连带发现（误导性文字）**：见 §5。

---

### 第 10 件 · `theta_v0/complete/` 整棵子树，档「归档（边界）」

**位置**：`theta_v0/complete/mod.rs`(198) + `state.rs`(315) + `event.rs`(137) = **650 行**，入库 `a28a079fd7`。

**零调用证据（子树级）**：三个核心类型 `CompleteState` / `ExternalEvent` / `OpenOrder` 在 `rust/src` + `rust/tests` 的**全部命中都落在 `theta_v0/complete/` 内部**（`grep -rn -w 'CompleteState|ExternalEvent|OpenOrder' --include='*.rs' . | grep -v '/theta_v0/complete/'` → 空）。顶层 trait `TransitionTheta`(mod.rs:48) 同样零引用——**从未被任何类型 `impl`**。

⟹ 这不是「一个模块里有个死类型」，是**整棵 3 文件子树没有任何外部消费方**。这是本票规模第二大的发现（仅次于已知件）。

**为什么是「归档」而非「接」**：模块头自陈其定位是与 FULL §3/§20 的**零遗漏对照**，并明确**不替换**生产闭环：

> 「本模块**不替换**闭环引擎的 `AssemblyState`/`MicroEvent`——后者是 `hybridStep` 闭环的工程化简态…本模块的完整 schema 服务「与 FULL §3/§20 零遗漏对照」。完整态→摘要态、外部事件→微事件均为**遗忘投影**。」
> 「全模块 = **L0**（纯结构 schema：FULL 文字 ↦ Rust 类型…零信息增量）」

`TransitionTheta` 更明确：「这是**接口签名**…具体 Θ 的转移实例化（含会计/声部/风险逻辑）**由闭环工位承载**，本接口只给签名。」

⟹ 有价值（FULL 覆盖度的对照凭证 + 17 分量/8 元组的显式化留档），但**不在 map #59 Destination 八项组件的任何一项上** ⟹ 档4「归档」：留档不装，不算装上。**按 #225 明文，不删。**

**边界所在**：见 §4-C。

---

### 第 11 件 · `voice_eat.rs`，档「归档（边界）」

**位置**：`theta_v0/classifier/voice_eat.rs`，382 行，入库 `89c23de3ac`。5 个顶层项 `ContainmentReport`(L54)/`verify_containment`(L81)/`dir_to_side`(L108)/`VoiceActivitySample`(L127)/`eat`(L162) 零外部引用。

**为什么是「归档」**：模块是 spec C06 `Eat(v,b) ⟺ ∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0` 的逐字定义镜像。它不在 Destination 八项组件的任何一项上（N2 力度门 / N3 塔内原生 / 活假设状态机 / LEE / χ 量纲 / C1 双开 / 5a5b 线形化 / 买卖点生命周期，均无「声部吃到判定」）。有价值（spec 覆盖凭证）但不在路线图 ⟹ 档4。

**边界所在**：见 §4-D。

---

### 第 12 件 · `trading/unified_necessity.rs` T55 三项，档「归档」（已有用户裁决）

**位置**：`trading/unified_necessity.rs:848`（`T55Observation` struct）/ `:861`（`impl`）/ `:881`（`prove_t55_dual_line_observe`）。

**口径③命中，且裁定已在案**（L845–848）：

> 「/// T55 比价双线定位观测（**M2 未接入**；`#[allow(dead_code)]` = **用户裁决"代码先写，等 M2 接入"**）。」
> 「**未接入 step**（M2-pending）——由 M2 选股层用真实比价走势 fire 流调用。」

**零调用证据**：`prove_t55_dual_line_observe` 全库 5 条命中 = L184 注释 + L882 定义 + L2052/2056/2060 本文件自测。

**定档**：**已有用户明确裁决（「代码先写，等 M2 接入」），不重裁。** 形式上等价于「等待路线图上的 M2 到位」，在 map #59（M1 端到端收口）范围内是档4「归档」——M2 选股层不在本图 Destination 上。**登记即完，不动。**

这是全库 25 处 `#[allow(dead_code)]` 里唯一一处「等待未来接线」型；其余 22 处均为 bin 探针内的归因字段保留（`p119`/`p111`/`p105`/`p89`/`p108`）、`trading/types.rs` 的结构对称字段、`bi_engine.rs`/`rec_driver.rs` 局部——均属正常工程保留，不构成资产级孤儿。

---

## 4. 边界组（4 件 + 1 项口径分歧，未强行定档）

### A · `strategy/mutex.rs`：主干与分支的消费方状态相反

- **main**：`MutexClass` 零消费方（含 `oscillation.rs`）。
- **分支 `kimi-nest-mainline-20260717`**：`strategy/oscillation.rs:16` 有 `use super::mutex::MutexClass;`，并在 L467/L515 定义 `mutex_class()` 映射，`protocol.rs:901` 也在用 ⟹ **有生产引用**。

**成因**：main 的 `oscillation.rs` 经 #282（#280 裁定 / ADR 0001 修正案一）把 `OscillationAction`/`OscillationApplyResult` 等账面形态整套删除，`mutex_class()` 映射随之消失；分支尚未承接该删除。

**拿不准在哪**：main 是更晚的状态，所以「消费方消失」是**真实的当前 trunk 事实**；但两线未合流，合流方向会决定 mutex.rs 最终是「档1 有外部消费方」还是「档1 仅剩自带 oracle」。**任一处置动作都应在两线合流后复核。** 我按 trunk 事实落「留」，因其自带差分测试独立成立，且 codex-q2-d1 §Q2 已裁其为 oracle 身份。

### B · `first_retrace_replay.rs`：Destination 归属需先裁范围

档2 的第四条判据「对端到端有补充相」要求它落在 Destination 组件清单上。D7 firstRetrace 是**买卖点 seed 身份的生命周期纪律**，与清单第八项「买卖点生命周期（接线）」语义相邻，但该项在 map #59 里未展开到 D7 粒度。

**拿不准在哪**：判它「接」= 我替编排者扩了「买卖点生命周期」的范围；判它「归档」= 我可能把端到端真需要的一块踢出路线图。**建议：先由编排者一句话裁定「买卖点生命周期接线」是否含 D7 firstRetrace 复核，再定档。** 我倾向「接」（571 行完整实现 + 零依赖新增 + 复核逻辑与 seed 身份直接相关），但这是倾向不是结论。

### C · `theta_v0/complete/` 子树：归档 vs 接，取决于 schema 完备性是否算端到端需求

我判「归档」的依据是它自陈 L0 纯 schema、不替换生产闭环、不在 Destination 八项上。

**拿不准在哪**：`AssemblyState` 是 6 分量摘要态（模块头自陈「σ_r/ω/E/Cash/ν 缺字段」），而 `CompleteState` 是 17 分量全量。若端到端后续需要完整会计态（如 LEE 级别原生执行的级别账本、χ 量纲的入场名义），这 650 行可能就是那块补充相，届时是「接」不是「归档」。**#225 硬约束是「前瞻禁止降级归档」**——本件不是前瞻档（rust 已实装），所以不触发该约束；但归档是可逆的（留档不装），若后续判定需要可再提。登记此风险以免日后重复发现。

### D · `voice_eat.rs`：其中一个测试确实断言了生产塔的性质

其 test mod L112 `use super::super::recursive_tower::{compose_level, ElementId}`（生产），且有一个测试带 doc：

```
    /// ★L2 确认（情况 A）：真实数据塔上 `verify_containment` 的 uncovered_elements = 0。
```

⟹ `verify_containment` 这一半**是**对生产递归塔的活检查（塔的覆盖性回归会打红它），符合 §1.4 的「留」侧；而 `eat`/`VoiceActivitySample`/`dir_to_side` 那一半是纯定义镜像，无任何消费，符合「归档」侧。

**拿不准在哪**：一个文件横跨两档。可选处置 = 拆分（`verify_containment` 归档1 留、`Eat` 定义归档4 归档），但拆分本身是改动，超出本票「只清点不处置」的边界。**按文件整体落「归档」并标此边界。**

### E · 主干 / 分支树面分歧（影响全表，非单件）

两树在 `rust/src` 上的文件集差 22 个（main 独有 11、分支独有 11），且互有 76/117 commit 未合。本表以 **main 为准**。分支面独立扫描结果：7 件孤儿，是 main 结果的子集加减——

- 分支**没有** `interval_necessity.rs`（第 2 件，main 独有）；
- 分支上 `mutex.rs` 有生产消费方（见 §4-A）；
- 分支扫描一度未列出 `voice_eat.rs`，经复核是**假阴性**——`runner.rs:1193` 有个局部闭包也叫 `eat`，污染了 token 索引；`ContainmentReport`/`VoiceActivitySample`/`verify_containment` 在分支上同样零引用 ⟹ **两树上 `voice_eat.rs` 都是孤儿**。（登记此项以示口径①在短名字上的已知局限。）

其余 6 件（nest_lifecycle / first_retrace_replay / force_conformance / six_state / closed_loop·conformance / complete·mod）**两树结论一致**。

---

## 5. 「删」档：0 件资产 + 1 处误导性文字（须编排者终审）

#225 删档是窄口径：**只收「留者将误导后来人」的内容**，「没用了」一律归档不删。逐件过下来，**12 件资产无一命中**——它们要么在跑（留）、要么该接（接）、要么是诚实标注了自身状态的留档（归档，且三件都自带准确的边界声明）。

**唯一命中「误导性」的是一段文字，不是一件资产**：

`theta_v0/nautilus/mod.rs` 模块头（★骨架状态节）写着：

> 「- **本模块不编译进 crate**（`lib.rs`/`theta_v0/mod.rs` **不注册它**，避免引入未解析符号）。
>    注册时机：依赖加入 + theta_v0 公开 API 解 test 门控后，报 Lead 登记。」
> 「- 真实 Nautilus trait 名/方法签名以**文档注释 + TODO** 锚定（不 `use nautilus_*`）。」

**实际状态与之相反**：

- `theta_v0/mod.rs:104` 有**无条件**的 `pub mod nautilus;`（且 L98–103 的 doc 正好描述了「注册它让 13 个 L0/L1 self-check 在 `cargo test` 下运行」——同一仓内两段 doc 直接打架）；
- `Cargo.toml` 已加入 `nautilus-model/-common/-trading/-backtest/-core` 五个真实依赖 + `nautilus` / `backtest_bin` 两个 feature；
- `nautilus/theta_strategy.rs`、`backtest_engine.rs` 均由 `#[cfg(feature = "nautilus")]` 门控并真实 `use nautilus_*`。

⟹ 后来人读模块头会得到「这模块没接进来、依赖还没加」的错误结论，而事实是依赖已加、模块已注册、CLI `theta_backtest` 已在用。**这正是 #225 删档举的那类例子「过期副本自称与权威源一致」的同型。**

**处置建议（不执行，仅登记）**：这是 3 行文字的**订正**，不是删资产——正确动作是把「不编译进 crate / 不注册」改成实际的 feature 门控描述。**按 #225「落删档须经编排者终审」，此项提交编排者裁定是按「删（误导性内容）」立票，还是按普通文档订正随手清。** 本票不动它。

---

## 6. 「接」档优先级排序（按对端到端 Destination 的补充相）

| 序 | 资产 | 排序理由 | 下一步 |
|---|---|---|---|
| 1 | `NestLifecycleBook` | Destination 明列组件「活假设状态机」；接线三题已裁（#402），前置探针已出（#409） | 已在跑，无需新动作 |
| 2 | `interval_necessity.rs` | Destination 明列组件「**N3 塔内原生**」正项；257 行已实现、零新依赖、4 个自测绿；唯一悬置是「只读观测 vs 硬门」的回写裁定 | **建议新出接线票**（含前置 grilling：回写口径），走 #402 同路径 |
| 3 | `center_lifecycle::terminates_suspension` | Destination「买卖点生命周期」下游的挂起/减补动作，票面已声明边界 | **不另出票**，随 #292 接入时一并接线；本表仅作登记 |
| 4 | `first_retrace_replay.rs` | 与「买卖点生命周期」相邻但非明列项，补充相强度待裁 | **先裁范围**（§4-B），裁定含则出接线票，裁定不含则改落归档 |

**未在此列**：五件「留」档不需要接线（已在跑）；三件「归档」按 #225 留档不装。

---

## 7. 自查与诚实声明（090）

- **本票的正当产出包含「否定」**：五档树里「前瞻」档 **0 件**，「删」档资产 **0 件**——这两个零是查实的结论（前瞻档因判据互斥而结构性为空，删档因 12 件资产全部自带诚实的状态标注而无命中），不是没查。
- **未凑数**：第 5–9 件（six_state / force_conformance / closed_loop·conformance / mutex / IntegrationPath）都符合票体的 grep 口径，但逐件读下来它们各有活性或有既有裁定，因此落「留」而**没有**为了让报告显得有料而报成 orphan。第 12 件 T55 有明确用户裁决在案，同理不重裁。
- **未越界**：全程只读；未 commit / stash / checkout / reset；未接任何「接」档件；未删任何「删」档件；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入（对 main 的读取全部经 `git archive` 导出到 `/tmp` + `git grep <rev>`，不触主仓工作区）。本文件是本票唯一的写操作。
- **口径局限（登记，不掩饰）**：
  1. token 级 grep 对**短名字/常见名**有假阴性（已实测到 `eat`、`Force`、`M` 三例，均已人工复核订正）；对 `impl` 块内方法未做系统清点（见 §1.5），故本报告是**资产/模块粒度穷尽、方法粒度非穷尽**。
  2. 未编译验证（`cargo check`）——本票只读且工作面存在未提交改动，编译结论会被污染；全部结论基于静态引用图 + 逐件人工阅读。
  3. 「同文件测试算不算活消费方」这条线是我从已知件反推的（§1.4），非 #225 原文；若编排者改判，§3 的 4 件「留」与已知件的「接」须一并复议。
  4. main 与工作面分支未合流（§4-E），任何处置动作前须在合流后的树上复核。

---

## 附：可复现的扫描方法

```bash
# 1. 只读导出 trunk 的 rust 树（不触主仓工作区）
git archive main rust/ | tar -x -C /tmp/orphan-scan-main

# 2. 提取顶层 pub 项（第 0 列起），剥注释与字符串后建全量 token 索引，
#    对每个顶层项查 rust/src + rust/tests 内除自身文件外的出现
#    —— 文件的全部顶层项均零外部命中 ⟹ 模块级 orphan 候选
#    （脚本 /tmp/orphan_scan3.py，一次性分析产物，未入仓）

# 3. 口径② 自述措辞
grep -rn '出切片\|接线待定\|参照实装\|生产 bin 接线\|未接线\|无生产调用者\|尚未接入\|待接入' rust/src/

# 4. 口径③ dead_code
grep -rn 'allow(dead_code)' rust/src/

# 5. 逐件人工复核：模块头 / test mod 的 use 行与断言对象 / git log --diff-filter=A / 全仓非 rust 消费方
```
