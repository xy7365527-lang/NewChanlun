# runner.rs seam 拆分设计（design-it-twice）

- 日期：2026-07-21
- 对象：`rust/src/theta_v0/backtest/runner.rs`（worktree `/tmp/kimi-nest-mainline` 实测 **8498 行**）
- 来源：architecture-review（`/tmp/architecture-review-theta-v0-20260720.html`）判 runner.rs 为 Strong 重构候选（god module）；SPEC #73 B 线要求以 dual_ledger.rs 为第二 adapter 先例做 seam 拆分。
- 词汇：`codebase-design` skill（interface / depth / seam / adapter / leverage / locality / deletion test）。
- 约束：本文档只做设计，不改 `rust/` 任何文件；迁移按**文件内 expand→contract 节拍**设计（本项目禁 git mutation，每步以 `cargo test` 绿为完成判据）。

---

## 0. 实测锚点（行号以 worktree 为准）

### 0.1 runner.rs 内部区段图（8498 行 = ~4472 生产码 + 4473–8498 测试 ~4026 行）

| 行号区间 | 内容 | 归属倾向 |
|---|---|---|
| 1–60 | 模块 doc + imports | — |
| 63–115 | `pub struct RunResult`（~20 字段） | 装配层 |
| 117–246 | strict-nest sidecar（`StrictNestSidecarSummary`/`Collector`/`summarize_*`，env `THETA_STRICT_NEST_SIDECAR` 门控，只读旁路） | 外化 |
| 275–374 | `run_theta_v0`（#[deprecated] F-01 前视） | 装配层 |
| 376–498 | `run_theta_v0_dual`（#[deprecated]） | 装配层 |
| 500–553 | `run_theta_v0_pi` / `_chi` / `_chi_shrink` 薄入口 | 装配层 |
| 554–731 | `run_theta_v0_pi_inner`（管线装配：classify→fill loop→metrics→RunResult） | 装配层 |
| 656, 698 | `pub struct OverlayRunResult` / `VoiceExecRunSummary` | 装配层 |
| 731–782 | `voice_exec_gate` / `nest_cert_gate_enabled` env 门 | 准入 |
| 783–904 | `run_theta_v0_pi_overlay`（overlay/voice 臂装配） | 装配层 |
| 905–1061 | `bsp_bits_disc`、`newly_confirmed_step`（确认-bar 部署 diff） | 信号 |
| 960–1061 | `NestGateStats`、`nest_gate_admit`（区间套/选证准入门） | 准入 |
| 1062–1232 | `k_theta_risk_gate`、`kappa_policy_resolved`/`kappa_priority_resolve` | 准入 |
| 1186–… | `pub struct ChiFilterCtx<'a>`（χ_t 阈值过滤） | 准入 |
| 1233–1282 | `pi_theta_fill_loop` / `pi_theta_fill_loop_voice`（薄 wrapper） | fill |
| 1283–2500 | **`pi_theta_fill_loop_overlay`——god 本体（~1200 行）**：决策路径（`pi_theta_step_traced`）+ χ 过滤 + nest gate + `k_theta_risk_gate` + base_units + `prev_active` thread + 净额账本（`apply_order`）+ **TW 账本线程**（1409–1430 初始化、1560–1600 ②'成本基 ShortDiff / ②''Realize、2068–2449 OpenShareLeg/CloseShareLeg/TwStepCtx）+ **typed 账本**（`LedgerOpen`→`TypedTrade`，含 A9 generation 高水位 1400–1407）+ overlay 只读旁路 + voice_exec 投影臂 + **R 分解装配**（2449–2460 + 守恒 debug_assert） | fill + 账本 |
| 2505–2546 | `entry_structural_stop`、`candidate_stop_dist` | 信号 |
| 2547–2592 | `typed_ledger_from_bars`（pub(super)，训练侧入口） | 账本 |
| 2593–2659 | `run_closed_loop`、`buy_and_hold_return` | 装配层 |
| 2671–2747 | `pub struct TypedTrade` + `TYPED_TRADE_SCHEMA_VERSION=4` | 账本 |
| 2748–2875 | `LedgerOpen`、`OpsemEntrySnapshot` | 账本/外化 |
| 2876–3158 | `OpsemDump`（`from_env`/`at_dir`/`mark_entry`/`mark_exit`/`write_trade`/`write_tower_event`/`diff_tower`/`flush`/`Drop`，env `OPSEM_DUMP_DIR` 门控 + cfg(test) 线程局部注入点） | 外化 |
| 3160–3299 | ~15 个字符串化 helper（`opt_u8_str`/`lex_top3_json`/`exit_type_str`/`t_stage_str`/`voice_side_str`/`vertical_str`/`operation_role_str`/`risk_mode_str`/`eta_bucket_str`/`force_state_str`…）——全部只服务 OPSEM dump | 外化 |
| 3302–3352 | `struct FillOutput`（equity/returns/双口径 pnls/trades/typed_ledger/tw_final/r_decomp） | fill |
| 3353–3619 | `plan_and_fill_mtm`（v1 recognize 净额 MtM fill loop） | fill |
| 3620–3644 | `FillOutputDual`、`LegFillRec` | fill（dual） |
| 3645–3702 | `apply_voice_fill_dual` | fill（dual） |
| 3703–4098 | `plan_and_fill_mtm_dual`（双账本 fill loop，dual_ledger.rs 的第二调用方） | fill（dual） |
| 4099–4201 | `track_position_transition`、`apply_voice_fill` | fill |
| 4202–4275 | `simulate_fills`（骨架权益推进） | fill |
| 4276–4461 | `pub(crate) apply_order` / `apply_fill` / `FillOutcome`（先平后开、成本对称、A' realized 结算） | fill（执行原语） |
| 4462–4472 | `bar_returns` | fill |
| 4473–8498 | `mod tests`（~4000 行，含 L2/L3 真实数据否证测试） | 测试 |

### 0.2 外部硬引用实测（grep `runner::`，剔除 runner.rs 自身）

**代码引用（10 文件，~30 处）**：

| 被引符号 | 引用方（文件:行） |
|---|---|
| `run_theta_v0_pi` | `bin/theta_backtest.rs:30`、`incremental.rs:686`、`l3_fullwindow.rs:41`、`l3_pi_falsify.rs:39`、`l3_pi_probe.rs:27`、`wverify_run.rs:1092` |
| `run_theta_v0_pi_chi` | `l3_delta_r_alpha.rs:68`、`mu_estimator.rs:550` |
| `run_theta_v0_pi_overlay` | `bin/theta_overlay.rs:18`、`wverify_run.rs:1201` |
| `run_theta_v0` + `run_theta_v0_dual` + `RunResult` | `wverify_run.rs:1478`（use 组） |
| `RunResult` | `l3_delta_r_alpha.rs:332, 859` |
| `TypedTrade` | `l3_delta_r_alpha.rs:168, 191`、`selector.rs:295` |
| `typed_ledger_from_bars` | `l3_delta_r_alpha.rs:254, 1988, 2186`、`wverify_run.rs:1829` |
| `newly_confirmed_step` | `l3_pi_probe.rs:366, 486, 581` |
| `run_closed_loop` | `incremental.rs:556`（测试 import） |
| `apply_order` | `dual_ledger.rs:472`（D8 嵌入恒等 bit-exact 对拍测试） |

**doc-comment 引用（不计代码耦合，但改名时要同步）**：`strategy/interp.rs:1054`、`l3_fullwindow.rs:3`、`closed_loop/transition.rs:563`、`strategy/coverage.rs:2880`、`tests/theta_v0_perf_profile.rs:15,21`。

**误报剔除**：`runner::run_organic`（lib.rs:1717、trading/*.rs ×3）是 `trading::runner`，与本文件无关。architecture-review 的「50 处散布 17 文件」含 doc 引用与行内多处计数；代码级真实耦合 = **11 个符号 × 10 个文件**。

### 0.3 dual_ledger.rs 先例（SPEC #73 指定的第二 adapter）

526 行，interface 极小：`DualLedger{new, net_units, gross_units, equity}` + `FillOutcomeDual` + `apply_fill_dual` + `compatible_leg_orders`；实现自含；seam 真实性由 `dual_bitexact_embedding_vs_apply_fill`（:440，对 `apply_fill` 逐字节对拍）锁死。按 skill 语言：**一个 adapter 是假想 seam，两个 adapter 是真 seam**——净额 `apply_fill` 与 dual `apply_fill_dual` 已构成真 seam，拆分是把这个事实 seam 显式化，不是发明新抽象。

---

## 方案一：按数据流阶段切（信号 → 准入 → fill → 账本 → 外化，五 seam）

### 一句话

把 runner.rs 按管线阶段横切成五个深模块，runner.rs 退化为纯装配层（入口 fn + `RunResult` 装配 + `pub use` 门面）；账本线与 fill 是相邻两段 seam，TW/typed 账本作为 fill loop 的**输出物**归账本模块持有。

### 模块清单与 interface（pub fn 签名级）

#### M1 `backtest/signal.rs` —— 因果信号源 seam

```
pub type ClassifyAt<'a> = dyn FnMut(usize)
    -> (Classification, Vec<Rc<Vec<LeveledMove>>>, u64, u64) + 'a;
pub fn newly_confirmed_step(
    classification: &Classification, tower: &[Rc<Vec<LeveledMove>>],
    tower_gen: u64, forest_epoch: u64, seen: &mut SeenSet,
) -> Vec<Candidate>;                       // 现 runner.rs:929
pub fn entry_structural_stop(c: &Candidate, classification: &Classification) -> Option<Tick>;  // :2505
pub fn candidate_stop_dist(c: &Candidate, classification: &Classification) -> Option<f64>;     // :2532
```

- 深度：调用方只学「给我一个 bar index 的因果分类闭包 + 新确认候选 diff」，背后藏 seen-set append-only 判别（`bsp_bits_disc`）、K_i 森林段 O(1) 命中、塔代次判据。
- 既有消费方：`l3_pi_probe.rs` ×3 直接调 `newly_confirmed_step`（已把它当公共 seam 用）。

#### M2 `backtest/admission.rs` —— 准入门 seam

```
pub struct ChiFilterCtx<'a> { est: &'a MuEstimator, theta: f64, z_alpha: f64,
                              treat_empty_as_pass: bool, shrink_tau_sq: Option<f64> }  // :1186
pub struct NestGateStats { … }                                    // :960
pub fn nest_gate_admit(c: &Candidate, classification: &Classification,
                       tower: &[Rc<Vec<LeveledMove>>], hist: Option<&NestGateHistogram>,
                       stats: &mut NestGateStats) -> bool;        // :1000
pub struct KThetaRiskGate { force_flat: bool, stop_dirs: …, … }
pub fn k_theta_risk_gate(prev_active: &[ActiveLeg], classification: &Classification,
                         open: &LedgerOpenMap, px: Tick, i: usize) -> KThetaRiskGate;  // :1062
pub fn kappa_policy_resolved(config_policy: Option<RiskPolicy>) -> RiskPolicy;          // :1209
```

- 深度：三道门（χ 统计准入 / nest 结构准入 / k_Θ 风控收窄）共用一个 seam 语义——「候选/组合 → 放行或收窄 𝒦_Θ」。κ 优先序（env > config > baseline，A10 附则A 裁定接口冻结）藏在本模块内，fill loop 不再 import env 细节。
- 注意：`k_theta_risk_gate` 读 `LedgerOpen.entry_stop`（族A 冻结止损）⟹ 依赖 M4 的 `LedgerOpen` 类型；这是阶段间唯一反向类型依赖，处理见 §4。

#### M3 `backtest/fill.rs` —— 净额执行 seam（执行原语 + fill loop）

```
pub(crate) struct FillOutcome { requested_qty, executed_qty, closed_qty,
                                opened_qty, rejected_qty, realized, fee: f64 }
pub(crate) fn apply_order(o: &Order, px: f64, fee_rate: f64, cash: &mut f64,
                          units: &mut f64, entry_cost: &mut f64,
                          trade_pnls: &mut Vec<f64>) -> FillOutcome;   // :4276
pub(crate) fn apply_fill(delta, qty, close_only, px, fee_rate, …) -> FillOutcome;  // :4321
pub struct FillOutput { equity_curve, daily_returns, trade_pnls_realized,
                        trade_pnls_with_forced, trades, n_orders,
                        typed_ledger, tw_final, r_decomp }             // :3302
pub fn pi_theta_fill_loop(classify_at: F, bars: &[Bar], initial_nav: f64,
                          config: &ThetaConfig, chi: Option<ChiFilterCtx>) -> FillOutput;      // :1233
pub fn pi_theta_fill_loop_overlay(classify_at: F, bars, nav, config, chi,
                          overlay: Option<&mut OverlayState>,
                          voice_exec: Option<&mut VoiceExecBook>) -> FillOutput;               // :1283
pub fn plan_and_fill_mtm(decisions: &[VoiceDecision], bars, nav, config) -> FillOutput;        // :3353
pub fn simulate_fills(bars, orders, nav, config) -> (Vec<f64>, Vec<f64>, Vec<f64>);            // :4202
```

- 深度：本方案最深处。interface = 「decisions/分类闭包进，`FillOutput` 出」；实现 = ~1200 行 god loop + 先平后开 + 成本对称 + 现金约束 + 强平双口径。
- `apply_order`/`apply_fill`/`FillOutcome` 保持 `pub(crate)`：`dual_ledger.rs:472` 的 D8 嵌入恒等对拍经 `pub use` 门面继续可见，不扩大可见性。

#### M4 `backtest/ledger.rs` —— 账本线 seam（typed 腿账本 + TW 线程）

```
pub struct TypedTrade { … }                       // :2671（schema v4，含 exit_z/units/position_node_id）
pub const TYPED_TRADE_SCHEMA_VERSION: u32;        // :2745
pub(super) fn typed_ledger_from_bars(bars: &[Bar], config: &ThetaConfig) -> Vec<TypedTrade>;  // :2547
pub(crate) struct LedgerOpen { … }                // :2748（入场冻结止损/角色/身份四元组）
pub fn track_position_transition(…);              // :4099
/// TW 账本线程（新深化，现散在 fill loop 1409–2449 的 ~10 个 tw_step 调用点）：
pub struct TwLedgerThread { tw: TwState, seen_basis: i64, seen_realized: i64 }
impl TwLedgerThread {
    pub fn new(nav0: f64) -> Self;
    pub fn on_fill(&mut self, outcome: &FillOutcome, units: f64, entry_cost: f64);  // ②' ShortDiff + ②'' Realize
    pub fn on_bar(&mut self, ctx: TwStepCtx, ev: TwEvent);                          // ③ stage/leg 事件
    pub fn finish(self) -> TwState;
}
```

- 深度论证：`TwLedgerThread` 是本方案**唯一新增深化**——把 fill loop 内 10 处 `tw_step` 直调 + shadow 量化差分口径（`tw_seen_basis`/`tw_seen_realized` 截断误差有界不变量，清单⑥）收敛到一个对象。fill loop 从「懂 TW 守恒 + 量化口径 + 事件序硬边界（先成本基后利润）」退化为「每 fill/bar 各调一次」。这是本方案对 god loop 最实质的减肥。
- 既有消费方：`TypedTrade`（selector/l3_delta_r_alpha）、`typed_ledger_from_bars`（l3_delta_r_alpha/wverify_run）已是事实 seam。

#### M5 `backtest/opsem_dump.rs` —— 外化 seam

```
pub struct OpsemDump { … }
impl OpsemDump {
    pub fn from_env() -> Option<Self>;            // env OPSEM_DUMP_DIR + cfg(test) 线程局部注入
    pub fn mark_entry(&mut self, bar: usize);
    pub fn mark_exit(&mut self, bar: usize);
    pub fn write_trade(&mut self, t: &TypedTrade, open: &LedgerOpen,
                       trigger_bsp_class: Option<u8>) -> io::Result<()>;
    pub fn write_tower_event(&mut self, bar: usize, tower: &[Rc<Vec<LeveledMove>>]) -> io::Result<()>;
}
pub struct StrictNestSidecarCollector { … }       // :141（从 runner.rs 头部随迁）
pub struct StrictNestSidecarSummary { … }         // pub，RunResult.strict_nest_sidecar 的类型
pub struct StrictNestCertificateRecord { … }      // pub
```

- ~800 行（OpsemDump + OpsemEntrySnapshot + 15 个 `*_str` helper + strict-nest sidecar）整体迁移，零接口变更——全部是只读旁路，env-gated，天然独立。

#### 装配层 `runner.rs`（contract 后剩余 ~600 行）

```
pub struct RunResult { … }                        // 不动
pub struct OverlayRunResult { … }                 // 不动
pub struct VoiceExecRunSummary { … }              // 不动
pub fn run_theta_v0 / run_theta_v0_dual           // #[deprecated]，不动
pub fn run_theta_v0_pi / _pi_chi / _pi_chi_shrink // 薄 wrapper，不动
fn run_theta_v0_pi_inner(…) -> RunResult          // 装配：signal→admission→fill→ledger→metrics
pub fn run_theta_v0_pi_overlay(…) -> OverlayRunResult
pub fn run_closed_loop(bars, initial_nav) -> Option<AssemblyState>
// 过渡期门面（expand 阶段关键）：
pub use fill::{…}; pub use ledger::{TypedTrade, …}; pub use signal::newly_confirmed_step; …
```

### Deletion test

- 删 M5（opsem_dump）：复杂度（JSONL schema、tower diff、15 个字符串化、线程局部注入竞态修复史）会在 fill loop 内重现 ~800 行 ⟹ 通过。
- 删 M4（ledger）：`TypedTrade` schema 版本纪律 + TW 守恒/量化差分口径会散回 fill loop + selector + l3_delta_r_alpha + mu_estimator 四处 ⟹ 通过。
- 删 M2（admission）：三道门的 env 优先序、χ shrinkage、nest 直方图诊断散回 loop ⟹ 通过。
- 删 M1（signal）：seen-set/塔代次判据散回 loop + l3_pi_probe ⟹ 通过。
- 删 M3（fill）：整个执行语义散回 5 个 run_* 入口 ⟹ 通过。
- 反向检验（防过切）：M1 与 M2 合并也讲得通（都是「决策前」），但 nest_gate/χ 的消费点与 newly_confirmed 的消费点在 loop 内不同位置，分开后各自 interface 更小；保守起见第一刀不合并。

### 迁移顺序（expand→contract，每步 cargo test 可绿）

节拍原则：每步 = 一次**纯移动**（代码块剪切到新文件 + runner.rs 加 `pub use` 门面）→ `cargo test` 全绿（含 bit-exact 回归锁）→ 下一步。禁止在移动步里顺手改逻辑（090 纪律）。

1. **M5 opsem_dump.rs**（:2876–3158 + 3160–3299 helper + 117–246 sidecar）。纯叶子，零外部代码引用，风险最低，先验证节拍可行。
2. **M4 ledger.rs 类型层**：`TypedTrade`/schema const/`LedgerOpen`/`OpsemEntrySnapshot`（dump 用，从 M5 import）/`track_position_transition`。外部 3 文件经门面不受影响。
3. **M3a fill 执行原语**：`apply_order`/`apply_fill`/`FillOutcome`/`simulate_fills`/`bar_returns`。`dual_ledger.rs:472` 对拍经 `crate::theta_v0::backtest::runner::apply_order` 路径继续命中（`pub(crate) use` 保持路径）。
4. **M1 signal.rs**：`newly_confirmed_step`/`entry_structural_stop`/`candidate_stop_dist`/`ClassifyAt`。l3_pi_probe ×3 经门面。
5. **M2 admission.rs**：χ/nest/risk gate/κ 解析。依赖 M4 的 `LedgerOpen`——M4 先行已铺路。
6. **M3b fill loop 本体**（最难步，内部再拆两拍）：
   - expand 拍：`pi_theta_fill_loop_overlay` 函数体内聚出 `TwLedgerThread`（M4）替换 10 处 `tw_step` 直调——**不改行为**，shadow 口径逐行平移；`typed_ledger` 开/关腿段收为 `ledger.rs` 的 `open_leg/close_leg` 关联函数。此拍 cargo test 必须 bit-exact 绿（既有 `run_theta_v0_pi_overlay_reconciles_and_bit_exact_net` :5659、`opsem_dump_env_gated_bit_exact` :4967 等回归锁兜底）。
   - contract 拍：loop 本体 + `plan_and_fill_mtm` + `plan_and_fill_mtm_dual` + `FillOutput(Family)` 移入 fill.rs。
7. **contract 收尾**：runner.rs 只剩入口 + RunResult 装配 + `run_closed_loop`；测试 mod 留在 runner.rs（测试跨 seam 消费装配层，符合「interface 是测试面」——4000 行测试正是以 runner 入口为测试面写的）；`pub use` 门面保留一个版本周期并打 `#[deprecated]` 提示，随后逐文件把 10 个引用方改指新路径（每文件独立小步，独立 cargo test）。

### 50 处硬引用处置

- expand 全程：`pub use` 门面保证 10 个代码引用方 + 2 个 bin **零改动**。
- contract 后：11 个符号按上表逐文件迁移（`RunResult`/`TypedTrade` 等类型迁到 `backtest::ledger`/`backtest::runner` 新路径）；doc-comment 引用（interp.rs:1054 等 5 处）在同一 pass 同步改；`trading::runner::run_organic` 误报不动。

---

## 方案二：按账本所有权切（每条账本线自带 fill loop 的 adapter 簇）

### 一句话

seam 不是管线阶段而是**账本线**——净额账本 / 双账本 / 声部执行簿各自是一个 adapter 簇，拥有自己的 fill loop、成交原语和输出类型，三个簇在同一个 `LedgerLine` interface 后面可互换；dual_ledger.rs 先例（第二 adapter）从「并列函数」升格为「并列簇」。

### 顶层 seam

```
/// 一条账本线 = 一种持仓/成交语义的完整执行栈。同一决策帧流进，同族输出出。
pub trait LedgerLine {
    type Output;
    /// 消费决策帧（每 bar 的 pi_theta_step_traced 产出 + 因果分类），推进本线账本。
    fn on_bar(&mut self, frame: &DecisionFrame, bar: &Bar, px: f64);
    /// 窗口终点：强平、结算、产出。
    fn finalize(self) -> Self::Output;
}
pub struct DecisionFrame<'a> {   // 决策路径的唯一载体（三簇共享，见「共享决策核」）
    pub classification: &'a Classification,
    pub tower: &'a [Rc<Vec<LeveledMove>>],
    pub step_trace: &'a StepTrace,   // pi_theta_step_traced 产出（含 sep_legs/risk_exits/…）
    pub admitted: AdmittedSet,       // χ/nest/risk 三门之后的候选集
}
```

### 模块清单与 interface

#### A1 `backtest/lines/netting.rs` —— 净额账本簇（默认 adapter）

```
pub struct NettingLine { cash, units, entry_cost, tw: TwLedgerThread,
                         typed: TypedLedger, r_acc: RAccum, … }
impl NettingLine {
    pub fn new(initial_nav: f64, config: &ThetaConfig) -> Self;
}
impl LedgerLine for NettingLine { type Output = FillOutput; … }
// 本簇私有：apply_order / apply_fill / FillOutcome（从 runner 迁入，可见性降为簇内）
```

- 深度：interface = `new` + `on_bar` + `finalize`；实现 = 先平后开、成本对称、TW 线程、typed 账本、R 分解守恒断言。调用方（run_* 入口）不再知道 `FillOutcome` 存在。
- **方案一里 `pub(crate)` 的 apply_order 在本方案变为簇私有**——`dual_ledger.rs:472` 的 D8 对拍改为 `#[cfg(test)] pub(crate)` 通道或把对拍测试随迁本簇（推荐后者：测试跟着 seam 走）。

#### A2 `backtest/lines/dual.rs` —— 双账本簇（dual_ledger.rs 的自然归宿）

```
pub struct DualLine { ledger: DualLedger, leg_log: Vec<LegFillRec>, … }
impl LedgerLine for DualLine { type Output = FillOutputDual; … }
pub fn compatible_leg_orders(o: &Order, units: f64) -> Vec<LegOrder>;  // 从 dual_ledger.rs 平移
```

- `plan_and_fill_mtm_dual`（:3703）+ `apply_voice_fill_dual`（:3645）+ `FillOutputDual`/`LegFillRec`（:3620）全部入簇；`dual_ledger.rs` 本体不动（它已是深模块），本簇是它的执行臂。

#### A3 `backtest/lines/voice.rs` —— 声部执行簿簇（W1 声部独立执行臂）

```
pub struct VoiceLine { book: VoiceExecBook, shadow: NettingLine }  // 影子净额驱动决策层（bit-exact 锁）
impl LedgerLine for VoiceLine { type Output = (FillOutput, VoiceExecRunSummary); … }
```

- 现 :1257 `pi_theta_fill_loop_voice`（cfg(test)）+ overlay loop 内 `voice_exec=Some` 分支升格为独立簇。影子净额账本「原样跑」的 bit-exact 契约由**组合 NettingLine** 表达（而非 loop 内 if 分支）——这是本方案最漂亮的一笔：arm 变组合。

#### A4 `backtest/lines/mod.rs` —— 分发 + 共享决策核

```
/// 三簇共享的决策路径（pi_theta_step_traced + 三门 + prev_active thread）——
/// 本方案的诚实代价：决策核无法按账本线切开（声部臂注释明文「同一决策路径」），
/// 故独立成核，三簇 consume 它的 DecisionFrame。
pub fn drive<L: LedgerLine>(classify_at: F, bars: &[Bar], config: &ThetaConfig,
                            chi: Option<ChiFilterCtx>, line: &mut L) -> LineMeta;
```

- 装配层 `runner.rs`：`run_theta_v0_pi` = `drive(..., NettingLine::new(...))`；`run_theta_v0_dual` = `drive(..., DualLine::new(...))`；`run_theta_v0_pi_overlay` = `drive(..., VoiceLine::new(...))`。入口从「调 loop」变为「选 adapter」。

### Deletion test

- 删 A2（dual 簇）：`DualLedger` 失去唯一生产驱动方，双账本语义整体消失，复杂度不散回别处 ⟹ **簇是干净的所有权边界**（这是本方案优于方案一之处）。
- 删 A3（voice 簇）：声部执行语义整体消失（VOICE_EXEC=1 环境变量失去意义），净额簇零感知 ⟹ 通过。
- 删 A4（共享决策核）：χ/nest/risk 三门 + `pi_theta_step_traced` 调用 + `prev_active` 高水位/generation 逻辑要在**三个簇各复制一份**（~600 行 ×3）⟹ 通过，但这正是本方案的结构性软肋——见对比节。
- 删 A1（netting 簇）：生产路径消失；但注意 A3 的 shadow 依赖 A1 ⟹ 簇间有真实依赖，不是三个完全平行的 adapter。

### 迁移顺序（expand→contract）

1. expand：`LedgerLine` trait 以**默认实现空壳**落在 runner.rs 内（新文件 `lines/mod.rs`，`pub use` 零变更）；`DecisionFrame` 从 loop 内局部变量聚出（纯结构提取，不改行为，cargo test 绿）。
2. netting 簇收编：apply_order/apply_fill/FillOutcome/TW 线程/typed 账本迁入 `lines/netting.rs`（纯移动 + 门面），`pi_theta_fill_loop` 薄壳改为 `drive::<NettingLine>`。
3. dual 簇收编：:3620–4098 整体迁入 `lines/dual.rs`，`run_theta_v0_dual` 改走 `drive`。
4. voice 簇升格：loop 内 `voice_exec` 分支外提为 `VoiceLine` 组合 `NettingLine`；`VOICE_EXEC=1` 语义不变，bit-exact 回归锁（:5659、:5707 `voice_exec_event_driven_fills_decision_bitexact`）必须绿。
5. contract：loop 本体消灭（三簇 + drive 完全覆盖）；门面逐文件迁移引用方（同方案一 §7）。

### 50 处硬引用处置

同方案一的门面策略，但有一处实质差异：`apply_order` 从 `pub(crate)` 降簇私有后，`dual_ledger.rs:472` 的 D8 对拍**必须随迁**（测试跟 seam 走），这是本方案对既有文件唯一不可避免的触碰点。

---

## 对比与倾向

| 维度 | 方案一（阶段切） | 方案二（账本线切） |
|---|---|---|
| 与既有引用形状 | 引用方要的是 `run_theta_v0_pi`/`TypedTrade`/`newly_confirmed_step`——**按阶段分布**，方案一接口对号入座 | `TypedTrade`/`typed_ledger_from_bars`/`newly_confirmed_step` 不属于任何账本线，要塞进 A4 决策核，归属牵强 |
| 决策路径处置 | 天然落在 M1/M2（信号/准入段），无重复 | 必须承认 A4 共享决策核（决策路径三簇共用，:1257 注释明文「同一决策路径」）——seam 切到一半被共享核截断 |
| adapter seam 真实性 | fill 段内部仍有 netting/dual/voice 三臂在一个 loop 里 if 分支（seam 未显式化） | 三臂 = 三 adapter，skill「两个 adapter = 真 seam」完全兑现；voice shadow 从 if 分支变组合，最漂亮 |
| 新增深化 | `TwLedgerThread`（收敛 10 处 tw_step 散点） | `drive<L: LedgerLine>` 泛型分发 |
| 迁移风险 | 每步是纯叶子移动，god loop 最后碰 | 第 1 步就要从 god loop 提取 `DecisionFrame`——第一刀就开在最难的地方 |
| god loop 结局 | loop 本体仍在（~1200 行），但被 M4/M5 掏空到 ~700 行 | loop 本体消灭，拆进三簇 + drive |

**倾向：方案一为主干，在 fill 段内部吸收方案二的 adapter seam**（记「方案一+」）。理由：

1. **引用形状决定 seam 形状**。外部 11 个符号的引用方是按阶段消费的（探针要信号、μ 层要账本类型、bin 要入口），方案一的模块边界与既有消费图同构，迁移后每个引用方都有自然新家；方案二下 `TypedTrade`/`newly_confirmed_step` 无家可归（不属于任何账本线）。
2. **方案二的纯形态会退化**。决策路径三账本线共用是代码里写明的事实（「同一决策路径…禁第二裁决源」），纯方案二要么复制决策核 ×3（不可接受），要么立 A4 共享核——而立了共享核之后，A4 就是方案一的 M1+M2+M4 决策段，方案二退化为方案一外加一个 adapter 分发层。既然收敛到同一终点，选第一刀风险低的路径（方案一第 1 步是纯叶子移动；方案二第 1 步就切 god loop）。
3. **但方案二的洞察必须吸收**：fill 段内部的 netting/dual/voice 三臂是真 seam（dual_ledger.rs 先例 + W1 审计 §7），在方案一 M3/fill.rs 内部以 `LedgerLine` 式私有 seam 组织（expand 第 6 步做），让 god loop 收缩为三臂组合——depth 归方案一，adapter 显式化归方案二。
4. dual_ledger.rs 先例的读法：它是「小 interface + 自含实现 + bit-exact 对拍锁」的样板，证明的是**账本语义适合做成自含深模块**，方案一 M3/M4 与方案二 A1/A2 都兑现这一点；它不是按账本线切全局的证据——dual_ledger 自己也是从 runner 外部被 `plan_and_fill_mtm_dual` 驱动的，驱动方与被驱动方的分离恰恰是方案一的阶段 seam。

---

## 附：不变式（两方案共同遵守）

1. **bit-exact 优先于美观**：任何迁移步不得改变任何 run_* 入口的数值产出；既有回归锁（:4967 opsem env-gated bit-exact、:5659 overlay reconcile、:5707 voice bit-exact、dual_ledger:440 嵌入恒等）是每步的验收门。
2. **deprecated 边界不动**：`run_theta_v0`/`run_theta_v0_dual` 的 F-01 deprecated 语义（C1 票审定终态）在迁移中原样随迁，不解除、不扩大。
3. **可见性只缩不扩**：`pub(crate)`/`pub(super)` 项迁移后保持原可见性；`apply_order` 若降私有须同步迁移 D8 对拍测试（090：声明=能力）。
4. **090 纪律**：迁移步 = 纯移动；任何「顺手改进」（如合并 M1/M2、改 `ClassifyAt` 为泛型参数）单独立步、单独 cargo test。
5. **测试跟 seam 走**：runner.rs 内 ~4000 行 `mod tests` 第一版留在装配层（测试面 = run_* 入口，符合「interface 是测试面」）；只随各模块迁移**该模块私有类型的单元测试**（如 OpsemDump、DualLedger 对拍）。
