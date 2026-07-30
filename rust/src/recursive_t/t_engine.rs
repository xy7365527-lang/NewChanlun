//! **T 操作层引擎**（flat：单核心 + 区间套 sink/recover 短差执行层，含次级别空头腿）。
//!
//! GUARD-ROLE: t-engine-flat-branch-live-python-caller——名分：现役（详见 `stream.rs`
//! 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! ## 架构状态（2026-06-20 编排者裁决：递归重写为正式方向）
//! 本文件是**当前 flat 实现**：`layers: Vec<Layer>`（绝对 ladder 数组）+ 中央 `route_bsp`。它对
//! T 算子递归的「模拟」会产生绝对/相对裂缝（三失效点：买<卖 / 891noop / 50%平空率）。**正式重构
//! 方向是递归自相似架构**（每级别一个 T 实例、父子局部通信、级别涌现 spawn、空头绑回调走势节点）——
//! 设计见 `docs/recursive_t_architecture_v2.md`。本 flat 引擎是该递归设计的退化特例（单实例多 ladder
//! + 单 free 池），在递归落码前继续作为回测基线。
//!
//! ## 次级别短差 = 减仓 + 开反向空头（编排者裁决保留空头腿，2026-06-20）
//! 曾有"sink=减仓到现金、删空头"的改动尝试——**已被编排者否定并回退**（原文第26课:34"先卖后买与
//! 先买后卖效果一样"、27课:136-140"次级别对冲"支持次级别开空；−89.7% 穿仓是 flat 架构的绝对/相对
//! 错配，不是"开了太多空头"，递归架构把空头绑回调走势节点即正确管理）。故 sink/recover 保留空头腿。
//!
//! ## 三个 τ 原子（不硬编码四步循环，四步是涌现序列）
//! - **sink**（子级 j 收反父向 BSP，父级 P 活跃）：`reduce_at(P, m=u_P/3)` + `add_at(j, m, flip(d_P))`。
//!   父级减 1/3，次级别开反向短差 m。Δexposure：父 u_P→2u_P/3，子 0→u_P/3 反向 ⟹ 净敞口 = 2/3 父 − 1/3 父。
//! - **recover**（子级 j 收同父向 BSP，j 持反父向短差）：`reduce_at(j, m=u_j)` + `add_at(P, m, d_P)`。
//!   次级别走势完成 ⇒ **整条短差一次性平清**（全量 m=u_j，非 1/3 配额；编排者裁决 2026-06-19 方案②，
//!   543号开放轴#1），资金全额升回父级 ⟹ 核心仓恢复 sink 前水平。空头短差高开低平的 realized pnl 即降成本。
//! - **drain**（子级持**同父向**遗留仓，中间级别插入后出现）：反父向 BSP ⇒ `reduce_at(j, u_j/3)` 减暴露
//!   （父级主导不翻转），排空后回归纯短差。
//!
//! ## 核心仓（最高活跃级别，独立翻转）
//! 无活跃祖先的 BSP = 核心级信号：空仓 ⇒ enter（用全部 free）；同向且更高空 ladder ⇒ ascend（relabel
//! 上移，骑乘最高涌现级别，无新资金）；反向 ⇒ flip（clear 全塔到现金 + 同 ladder 反向 enter）。
//!
//! ## 仓位递归（几何塔，由 sink sizing 自然涌现）
//! sink 转移 = 父级 units/3 ⟹ 次级别 = 核心 1/3、次次级别 = 1/9 …… 相邻级别方向相反（手性交替，
//! sink 穿 ε=−1）。注（emergence 后）：手性交替仅在**连续占用段内**成立——`emergence_upgrade` 把核心
//! relabel 上移会留下 idle 间隙，间隙两侧由 sink 填入的腿可同向（`max_chiral_same_dir` 观测之，非 panic）。
//!
//! ## 会计（单一共享 free 池，复用 `fugue_v3::accounting`）
//! NAV = free + Σ_k sign(d_k)·u_k·c。每个 reduce_at/add_at 在同价 c 上 NAV 中性 ⟹ 总 NAV 逐 bar 守恒
//! （`prove_nav_neutral`）。Σ|units| 仅 enter/flip/clear/liq 改，sink/recover 只级间转移（守恒）。
//!
//! ## 边界算子：1x 逐仓强平（每层独立，作用于任意持仓含短差腿）
//! 多头 c≤basis/2 平掉；空头 c≥2·basis 爆仓。强平 → 该层归零现金。
//!
//! ## 认识论等级
//! - sink/recover/flip 会计 NAV 中性 / 几何塔涌现 / 手性交替（连续占用段内）/ 区间套 top-down：**L0**；
//! - sink sizing 1/3（MOBILE_FRAC）/ recover 全量了结（方案②）/ core 升降编排 / BSP fire 时机：**L2**；
//!   回测 alpha：**L3**（可否证）。

use crate::trading::types::{Polarity, INITIAL_CAPITAL, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::{
    active_voice_count, add_at, exposure, flip, mobile_quota, nav, reduce_at,
};
use crate::fugue_v3::layer::{FugueResult, Layer};
use crate::fugue_v3::prove::prove_nav_neutral;
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, SUB_LIQ_FACTOR};

use super::prove_guards::{
    prove_relabel_invariant, prove_sigma_quota, prove_sink_descends, OpTrigger, ProveGuards,
};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
pub const BASE_LADDER: usize = LADDER_MOVE;

// ════════════════════════════ 本 bar 信号视图 ════════════════════════════

/// 本 bar 信号视图（从 T 全塔 BSP diff 构造，stream 层填充）。
///
/// 索引空间 = **ladder**（= T level + BASE_LADDER）。纯 BSP 驱动：不区分 type1/2/3（都是
/// 「该级别买/卖点」），级别涌现信息隐含在「哪个 ladder 有 BSP」中。
#[derive(Debug, Clone)]
pub struct TSignalView {
    /// 本 bar 该 ladder 是否新增**任意**买点（type1/2/3 不分）。
    pub buy: [bool; MAX_LADDER],
    /// 本 bar 该 ladder 是否新增**任意**卖点。
    pub sell: [bool; MAX_LADDER],
    /// 本 bar 该 ladder 是否新增 **type1 买点**（底背驰 = 下跌走势终完美）。
    /// 走势完成信号（campaign 边界重定义，546号死锁解锁）：与 `buy` 同源去重的**子集**（仅 type1），
    /// 故 flat/rec **bit-exact 对称**——同一结构完成事件在两引擎产同一标记（仅 ladder/level 索引偏移）。
    pub t1buy: [bool; MAX_LADDER],
    /// 本 bar 该 ladder 是否新增 **type1 卖点**（顶背驰 = 上涨走势终完美）。
    pub t1sell: [bool; MAX_LADDER],
    /// 本 bar T 迭代**涌现上界**：`(ladder, 操作极性)`——最高已诞生上级单元的 ladder +
    /// 其走势方向对应极性（向上走势=Long 归属 / 向下=Short 归属）。
    ///
    /// **自下而上仓位涌现**的信号：操作层据此把核心仓 relabel 升级归属到该 ladder（不新建仓），
    /// 不必等该级别 BSP fire。`None` = 本 bar 无重跑或塔无 completed 走势（不触发升级）。
    /// stream 层在段门控重跑时填充（涌现只随结构变化发生）。
    pub emergent_top: Option<(usize, Polarity)>,
}

impl TSignalView {
    /// 空信号（无 BSP）。
    pub fn empty() -> Self {
        TSignalView {
            buy: [false; MAX_LADDER],
            sell: [false; MAX_LADDER],
            t1buy: [false; MAX_LADDER],
            t1sell: [false; MAX_LADDER],
            emergent_top: None,
        }
    }
}

impl Default for TSignalView {
    fn default() -> Self {
        Self::empty()
    }
}

// ════════════════════════════ 持仓三阶段（缠师第31课，docs/three_stages_accounting_design.md）════════════════════════════

/// T 引擎持仓成本三阶段（缠师第31课 line 24/28/36，过程—状态—过程）。
///
/// **不复用 `types.rs::LedgerPhase`（两变体）**——那被 `trading::ledger::OrganicLedger` 用于与 Python
/// 的 P5 bit-exact 契约，加第三变体会破坏契约。T 引擎 standalone，定义自己的三变体（设计文档 §1.3 裁决）。
///
/// - `CostReduction`（① 降成本）：`realized_campaign < notional_in`。短差降成本，Σ|units| 守恒（恒仓，
///   第31课"买入多少就是卖出多少不增加仓位"）。
/// - `CapitalRecovered`（② 退本金）：`realized_campaign ≥ notional_in` 触发。把等于初始本金 K 的现金移出
///   在险池（free→withdrawn），第31课"原来投入的资金就全部收回来了"。在 free 不足时停留本态逐步抽出。
/// - `EarningShares`（③ 增股数）：本金已全额退出。纯利润买更多 units，Σ|units| 单调增（AmountConserving，
///   第31课"卖出多少资金就买入多少资金……仓位是增加的"）。**单向不可逆**（OQ-9 定理）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TStage {
    CostReduction,
    CapitalRecovered,
    EarningShares,
}

impl TStage {
    pub fn as_u8(self) -> u8 {
        match self {
            TStage::CostReduction => 0,
            TStage::CapitalRecovered => 1,
            TStage::EarningShares => 2,
        }
    }
}

// ════════════════════════════ T 操作层引擎 ════════════════════════════

/// **BSP 操作诊断**（纯观测，不改任何会计逻辑）：逐 BSP 路由后按操作类型计数 + 核心层
/// units 变化，回答「卖点触发时引擎做了什么（sink 减1/3 / drain 减1/3 / flip 翻空）」。
#[derive(Default, Clone, Copy)]
pub struct BspOpDiag {
    // ── 卖点信号 → 操作 ──
    pub sell_sink: u64, // 子级卖点→sink（父多真减仓1/3，下放次级别做空）= 正确短差
    pub sell_drain: u64, // 子级卖点→drain（减暴露1/3，不翻转）
    pub sell_recover: u64, // 子级卖点→recover（父空，平空头短差升回）
    pub sell_flip: u64, // 核心卖点→clear_all("flip")+enter(Short) = 清全塔翻空
    pub sell_ascend: u64, // 核心卖点→ascend（核心持空，更高 ladder 卖点骑乘）
    pub sell_enter: u64, // 全空卖点→enter(Short) 首次建空
    pub sell_noop: u64, // 卖点 no-op（不 pyramid）
    // ── 买点信号 → 操作（对照）──
    pub buy_sink: u64,
    pub buy_drain: u64,
    pub buy_recover: u64,
    pub buy_flip: u64,
    pub buy_ascend: u64,
    pub buy_enter: u64,
    pub buy_noop: u64,
    // ── 翻转细节（Q3：强牛做空）──
    pub flip_from_long: u64, // 卖点翻空时原核心**持多**（强牛中清牛市多头→做空，亏损风险）
    pub flip_from_short: u64, // 买点翻多时原核心持空
    pub flip_long_units_cleared: f64, // 翻空累计清掉的多头 units（核心仓被全清的量）
    // ── 核心层 units 变化（Q2）──
    pub sink_core_reduced_units: f64, // sink 中父级=最高活跃层（核心）时减的 units 累计（=Σ u_core/3）
    pub n_sink_on_core: u64,          // sink 父级就是核心层的次数（核心被短差直接减1/3）

    // ════ 「回来」诊断（编排者2026-06-20：卖是对的，问题在平空+做多）════
    /// 信号层密度（Q2：买点是否缺失）：各 ladder 收到的买/卖信号数（= view.buy/sell[j]）。
    pub buy_sig_by_ladder: [u64; MAX_LADDER],
    pub sell_sig_by_ladder: [u64; MAX_LADDER],
    /// 买点触发但**没 recover**（Q3：平了空就停 / 买点落在错误 level）：子级买点触发，
    /// 但 j 不持反父向短差 ⇒ 无空可平、无量升回父级 ⇒ noop（不做多回来）。
    pub buy_noop_no_short: u64,
    /// sink→recover 间隔（Q1：空头短差持仓时长 bar）。`last_sink_bar` 是内部状态（各 ladder
    /// 最近一次 sink 开空的 bar，-1=无）；recover 时累计 (recover_bar − sink_bar)。
    last_sink_bar: [i64; MAX_LADDER],
    pub recover_interval_sum: f64,
    pub recover_interval_count: u64,
    pub recover_interval_max: i64,
    /// 仍未平的 sink 空头（Q4：开了空没买回 = 核心未恢复）：sink 计数 − recover 计数（运行末）。
    /// 核心层 recover 升回的 units 累计（Q4：核心仓恢复量，对照 sink_core_reduced_units）。
    pub recover_core_restored_units: f64,
    pub n_recover_on_core: u64,

    // ════ 真空期/敞口诊断（编排者2026-06-20：绝对值敞口,零真空,绩效=Σ|涨跌幅|）════
    /// 各 bar 敞口态计数（四者和 = 总 bar 数）。真空 = 所有 layer units≈0（完全无方向）。
    pub n_vacuum_bars: u64, // long≈0 ∧ short≈0（完全无敞口）
    pub n_long_only_bars: u64,  // long>0 ∧ short≈0
    pub n_short_only_bars: u64, // long≈0 ∧ short>0
    pub n_both_bars: u64,       // long>0 ∧ short>0（多空同时）
    /// 首/末有敞口 bar（-1=从未）。建仓前的真空是 warmup，不算执行断链。
    pub first_active_bar: i64,
    pub last_active_bar: i64,
    /// 首次建仓后最长连续真空 run（bar 数）+ 内部当前 run 计数。
    pub max_vacuum_gap: i64,
    cur_vacuum_run: i64,
}

/// T 操作层引擎（双向耦合：单核心 + 区间套 sink/recover 短差执行层 + 持仓三阶段会计）。
///
/// `layers[k]` 是 ladder k 的净仓位（复用 `fugue_v3::Layer`：单一 direction，相邻级别方向相反）。
/// `free` 是**单一共享现金池**（初始 = `INITIAL_CAPITAL`）——sink/recover 在级别间转移资金，单池诚实。
///
/// ## 三阶段会计（持仓成本状态机，设计文档 `docs/three_stages_accounting_design.md`）
/// `notional_in`/`realized_campaign`/`withdrawn`/`earning_cash`/`stage` 跟踪当前 campaign（enter→clear）
/// 的成本演化。守恒律从"NAV 中性"升级为"**总财富中性**"：`TW = free + Σ持仓 + withdrawn`——退本金把
/// 现金移出在险池（free→withdrawn）使 in-system NAV 掉 K，但 TW 守恒（同价中性）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每 ladder 一个净仓位（索引 = ladder）。idle 时 units=0。
    layers: Vec<Layer>,
    /// 单一共享现金池（NAV = free + Σ sign(d)·u·c）。
    free: f64,
    last_close: f64,
    max_concurrent_seen: usize,

    // ── 持仓三阶段（缠师第31课）──
    /// 当前阶段（enter 起 CostReduction，clear 重置）。
    stage: TStage,
    /// 本 campaign 投入本金（enter 时 = m·c；clear 重置 0）。
    notional_in: f64,
    /// 核心仓有效持仓成本 per share（缠师"成本"）。enter=入场价；每次**核心多头 reduce**（高位卖出）
    /// 的 realized 直接 `core_cost_basis -= realized/remaining_long_units` 降成本（编排者裁决 2026-06-20
    /// 点1）；**短差腿（Short reduce）盈亏单独算，不入此**（点5）。穿 0 触发退本金。clear=NaN。
    core_cost_basis: f64,
    /// 本 campaign 入场时的 core_cost_basis（= 入场价）——诊断降成本进度用。clear=NaN。
    campaign_entry_cost: f64,
    /// 已退本金（退本金阶段移出在险池的现金；clear 时归还 free）。
    withdrawn: f64,
    /// EarningShares 阶段累计可增股数的纯利润（free 的子账：earning_cash ≤ free 不变量）。
    earning_cash: f64,
    /// 增股数开关（诊断 A/B 用）：env `T_NO_EARNING` 置位 ⇒ false（deploy_earning no-op，纯利润留 free
    /// 不买回 units）——量化增股数对收益的贡献（编排者 2026-06-20 任务3）。默认 true。
    enable_earning: bool,
    /// 三阶段总开关（基线 A/B）：env `T_NO_THREESTAGE` ⇒ false（退本金不触发，stage 永 CostReduction，
    /// 无全仓切换/无增股数 ⇒ 与三阶段前基线 bit-identical）。默认 true。
    enable_three_stage: bool,
    /// 核心走势完成清仓开关（546号死锁解锁的 A/B 消融门，与 rec_engine 对称）：env
    /// `T_NO_TREND_DONE_CLEAR` 置位 ⇒ false（走势完成不清仓 = 死锁基线），默认 true。
    enable_trend_done_clear: bool,
    /// BSP 操作诊断（纯观测，恒开，零逻辑影响）：逐 BSP 按操作类型计数 + 核心 units 变化。
    op_diag: BspOpDiag,
    /// prove 守卫族（编排者裁决 2026-06-21，与 rec_engine 对称）：BSP 触发归因（panic）+ sink/recover
    /// 平衡 / per-level 短差 pnl / 核心方向匹配（观测计数）。见 `prove_guards.rs`。
    guards: ProveGuards,
    /// 核心走势完成清仓次数（546号死锁解锁路径触发计数，纯观测，与 rec_engine 对称）。
    pub n_trend_done_clears: u64,
}

impl Default for TPositionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TPositionEngine {
    pub fn new() -> Self {
        let layers = (0..MAX_LADDER).map(Layer::idle).collect();
        TPositionEngine {
            res: FugueResult::default(),
            layers,
            free: INITIAL_CAPITAL,
            last_close: f64::NAN,
            max_concurrent_seen: 0,
            stage: TStage::CostReduction,
            notional_in: 0.0,
            core_cost_basis: f64::NAN,
            campaign_entry_cost: f64::NAN,
            withdrawn: 0.0,
            earning_cash: 0.0,
            enable_earning: std::env::var("T_NO_EARNING").is_err(), // 诊断 A/B：T_NO_EARNING 关增股数
            enable_three_stage: std::env::var("T_NO_THREESTAGE").is_err(), // A/B：关三阶段=基线
            enable_trend_done_clear: std::env::var("T_NO_TREND_DONE_CLEAR").is_err(), // A/B：关走势完成清仓=死锁基线
            op_diag: BspOpDiag {
                last_sink_bar: [-1; MAX_LADDER],
                first_active_bar: -1,
                last_active_bar: -1,
                ..BspOpDiag::default()
            },
            guards: ProveGuards::new(MAX_LADDER),
            n_trend_done_clears: 0,
        }
    }

    pub fn result(&self) -> &FugueResult {
        &self.res
    }

    /// prove 守卫只读访问（验收报告/测试）。
    pub fn guards(&self) -> &ProveGuards {
        &self.guards
    }

    /// BSP 操作诊断快照（纯观测）。
    pub fn op_diag(&self) -> BspOpDiag {
        self.op_diag
    }

    pub fn n_trades(&self) -> usize {
        self.res.trades.len()
    }

    /// 当前阶段（三阶段会计观测）。
    pub fn stage(&self) -> TStage {
        self.stage
    }

    /// 三阶段会计快照: (stage, notional_in, withdrawn, earning_cash, core_cost_basis)。
    pub fn stage_snapshot(&self) -> (TStage, f64, f64, f64, f64) {
        (
            self.stage,
            self.notional_in,
            self.withdrawn,
            self.earning_cash,
            self.core_cost_basis,
        )
    }

    /// 核心仓有效持仓成本（缠师"成本"，可<0）。无 campaign ⇒ NaN。
    fn cost_basis(&self) -> f64 {
        self.core_cost_basis
    }

    /// 核心多头 units（Σ 多头 units）。三阶段成本归属的"核心仓"（降成本分母 = reduce 后剩余多头）。
    fn core_long_units(&self) -> f64 {
        self.layers
            .iter()
            .filter(|l| l.units > 1e-12 && l.direction == Polarity::Long)
            .map(|l| l.units)
            .sum()
    }

    /// 累计已实现 pnl（Σ mobile_realized_pnl_by_ladder）——realized 增量的来源。
    fn realized_total(&self) -> f64 {
        self.res.mobile_realized_pnl_by_ladder.iter().sum()
    }

    /// 总财富（free + Σ sign(d)·u·c + withdrawn）——三阶段守恒量（退本金把现金移出在险池，TW 守恒）。
    fn total_wealth(&self, c: f64) -> f64 {
        nav(&self.layers, self.free, c) + self.withdrawn
    }

    /// 总 NAV（free + Σ sign(d)·u·c）。
    fn total_nav(&self, c: f64) -> f64 {
        nav(&self.layers, self.free, c)
    }

    // ──────────────── 三阶段会计（编排者裁决 2026-06-20）：reduce realized → cost_basis ────────────────

    /// 核算一次 reduce_at 的 realized（编排者裁决点1/点5）。`dir` = 被减层方向，`realized` = 该 reduce
    /// 的已实现 pnl（= (c−basis)×m）。
    /// - **Long reduce（核心高位卖出）= 降成本主力**：`core_cost_basis -= realized / 剩余多头 units`。
    ///   穿 0 ⇒ 退本金（CostReduction→CapitalRecovered，无额外交易，点2）。EarningShares 下入 earning_cash。
    /// - **Short reduce（短差腿做空→平空）= 单独核算**（点5）：不入 cost_basis；只观测 + EarningShares 入弹药。
    fn account_reduce(&mut self, dir: Polarity, realized: f64, c: f64) {
        match dir {
            Polarity::Long => match self.stage {
                TStage::CostReduction => {
                    let rem = self.core_long_units(); // reduce 后剩余多头（降成本分母）
                    if rem > 1e-9 && self.core_cost_basis.is_finite() {
                        self.core_cost_basis -= realized / rem;
                        // 诊断：降成本进度（(entry−cost)/entry，=1 ⇒ 成本归0）。
                        if self.campaign_entry_cost > 1e-9 {
                            let drop = ((self.campaign_entry_cost - self.core_cost_basis)
                                / self.campaign_entry_cost
                                * 1000.0)
                                .max(0.0) as u64;
                            self.res.max_core_gain_x1000 = self.res.max_core_gain_x1000.max(drop);
                        }
                        if self.core_cost_basis <= 0.0 && self.enable_three_stage {
                            // 退本金：cost_basis 穿 0（点2，纯状态切换，无额外交易）。
                            // T_NO_THREESTAGE 时不触发 ⇒ stage 永 CostReduction = 基线（cost_basis 仍记观测）。
                            self.stage = TStage::CapitalRecovered;
                            self.res.n_capital_recovered += 1;
                            self.try_withdraw_capital(c);
                        }
                    }
                }
                TStage::CapitalRecovered => self.try_withdraw_capital(c),
                TStage::EarningShares => {
                    if realized > 0.0 {
                        self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0));
                    }
                }
            },
            Polarity::Short => {
                // 短差腿单独核算（点5）：不入 cost_basis。
                self.res.short_leg_pnl += realized;
                if self.stage == TStage::EarningShares && realized > 0.0 {
                    self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0));
                }
            }
        }
    }

    /// **退本金**（缠师第31课"原来投入的资金就全部收回来了"）：把等于初始本金 K 的现金移出在险池
    /// （free→withdrawn）。free 不足则抽出可得部分，停留 CapitalRecovered 待后续 free 补足；全额抽出
    /// 后切 EarningShares（单向不可逆，OQ-9 定理）。TW 守恒（free→withdrawn 总财富不变）。
    fn try_withdraw_capital(&mut self, _c: f64) {
        let need = self.notional_in - self.withdrawn;
        if need <= 1e-9 {
            self.enter_earning();
            return;
        }
        let w = need.min(self.free.max(0.0));
        if w > 1e-12 {
            self.withdrawn += w;
            self.free -= w;
            self.res.total_withdrawn += w;
        }
        if self.withdrawn >= self.notional_in - 1e-9 {
            self.enter_earning();
        }
    }

    /// 进入增股数阶段：本金已全退。后续 reduce 的 realized 经 account_reduce 累入 earning_cash。
    fn enter_earning(&mut self) {
        if self.stage != TStage::EarningShares {
            self.stage = TStage::EarningShares;
        }
    }

    /// **增股数部署**（缠师第31课 line 169"抛出后跌回来把抛出的钱全补进去，买回来数量一定多了"）：
    /// EarningShares 阶段在买点把 earning_cash 全额买成核心层（`core`）新 units（AmountConserving，
    /// Σ|units| 单调增）。NAV 中性（free→units·c）。
    fn deploy_earning(&mut self, core: usize, bar: i64, c: f64) {
        if !self.enable_earning || self.stage != TStage::EarningShares || c <= 0.0 {
            return;
        }
        let cash = self.earning_cash.min(self.free.max(0.0));
        let q = cash / c;
        if !(q > 1e-12 && q.is_finite()) {
            return;
        }
        // core 必须空仓或已持多（add_at 层内单一方向不变量）。
        if self.layers[core].is_active() && self.layers[core].direction != Polarity::Long {
            return;
        }
        add_at(
            &mut self.layers,
            core,
            q,
            Polarity::Long,
            &mut self.free,
            c,
            bar,
            &mut self.res,
        );
        self.earning_cash -= q * c;
        self.res.n_earning_deploys += 1;
        self.res.earning_units_added += q; // 量化：增股数累计加的 units（任务3）
        self.res.earning_cash_deployed += q * c; // 量化：增股数累计部署的现金
    }

    /// campaign 结束（flip/clear/eod）：归还 withdrawn 到 free，重置三阶段状态。
    /// 诊断：累计 campaign 重置次数（= 翻转/清仓数，每次重置清零 core_cost_basis）。
    fn reset_campaign(&mut self) {
        // campaign 终点（核心走势完成）：prove_sink_recover_balance + prove_per_level_pnl（观测）。
        self.guards.campaign_end();
        if self.notional_in > 1e-9 {
            self.res.n_campaign_resets += 1;
        }
        self.free += self.withdrawn;
        self.withdrawn = 0.0;
        self.stage = TStage::CostReduction;
        self.notional_in = 0.0;
        self.core_cost_basis = f64::NAN;
        self.campaign_entry_cost = f64::NAN;
        self.earning_cash = 0.0;
    }

    /// 最高活跃级别（= 核心仓所在 ladder）。无活跃仓 → None。
    fn highest_active(&self) -> Option<usize> {
        (0..MAX_LADDER).rev().find(|&k| self.layers[k].is_active())
    }

    /// 级别 j 的最近活跃祖先（严格更高的第一个活跃 ladder）。无 → None（j 是核心级）。
    fn nearest_active_parent(&self, j: usize) -> Option<usize> {
        (j + 1..MAX_LADDER).find(|&k| self.layers[k].is_active())
    }

    /// 状态快照: (nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (f64, f64, f64, usize) {
        let c = if self.last_close.is_finite() {
            self.last_close
        } else {
            0.0
        };
        let (lu, su) = exposure(&self.layers);
        (self.total_nav(c), lu, su, active_voice_count(&self.layers))
    }

    // ──────────────── τ 原子（复用 accounting 会计）────────────────

    /// **enter**：核心级空仓首次建仓，用**全部 free** 建 `dir` 方向核心仓。
    /// 三阶段：记 `notional_in = m·c`（本 campaign 投入本金），stage 重置 CostReduction。
    fn enter(&mut self, j: usize, dir: Polarity, bar: i64, c: f64) {
        if c <= 0.0 {
            return;
        }
        // 新 campaign 起点：归还前一 campaign 残留的安全池本金到 free（如核心被强平未经 clear），
        // 再用全部 free 建仓——recovered capital 自动复投到新 campaign（缠师"资金不断增加参与种类"）。
        self.reset_campaign();
        let m = self.free / c;
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let spent = m * c;
        add_at(
            &mut self.layers,
            j,
            m,
            dir,
            &mut self.free,
            c,
            bar,
            &mut self.res,
        );
        self.res.n_entries_by_ladder[j] += 1;
        // 三阶段 campaign 起点：投入本金 = 建仓现金，core_cost_basis = 入场价（仅多头吸筹 campaign 有
        // 降成本语义；空头核心 cost_basis 仍记入场价但 Long-reduce 降成本对做空无意义，account_reduce 按
        // 被减层方向分流）。
        self.notional_in = spent;
        self.core_cost_basis = c;
        self.campaign_entry_cost = c;
        self.stage = TStage::CostReduction;
        self.earning_cash = 0.0;
        // prove_bsp_triggers_operation（panic）+ campaign 起点基线。
        self.guards.note_op("enter");
        self.guards.campaign_start();
    }

    /// **clear_all**：全塔平仓到现金（flip 前 / eod）+ 归还退本金、重置三阶段 campaign。
    fn clear_all(&mut self, bar: i64, c: f64, reason: &'static str) {
        self.guards.note_op("clear_all"); // prove_bsp_triggers_operation（panic）
        for k in 0..MAX_LADDER {
            let u = self.layers[k].units;
            if u > 1e-12 {
                reduce_at(
                    &mut self.layers,
                    k,
                    u,
                    &mut self.free,
                    c,
                    bar,
                    &mut self.res,
                    reason,
                );
            }
        }
        // campaign 结束：安全池本金归还 free（牛市结束清仓时连本带利都在 free），重置阶段。
        self.reset_campaign();
    }

    /// **ascend**：核心仓 relabel 上移（from→to，to 须 idle）——骑乘最高涌现级别，无新资金（NAV 不变）。
    /// 不变量：`to` 必须 idle（区间套核心上移不可覆盖活跃层，否则 silent 覆盖 = NAV 破裂）——
    /// release 也 fail-loud（守恒关键）。调用点 `route_bsp` 已保证 `!is_active(to)`。
    fn ascend(&mut self, from: usize, to: usize) {
        assert!(
            !self.layers[to].is_active(),
            "ascend 目标 ladder {to} 非 idle（units={}）：核心上移不可覆盖活跃层",
            self.layers[to].units
        );
        let (lu_pre, su_pre) = exposure(&self.layers); // 移植守卫（A5）：relabel 前敞口快照。
        let mut moved = self.layers[from];
        moved.ladder = to;
        self.layers[to] = moved;
        self.layers[from] = Layer::idle(from);
        let (lu_post, su_post) = exposure(&self.layers);
        prove_relabel_invariant(lu_pre, su_pre, lu_post, su_post); // ascend 是级别重标定非加仓，敞口必不变。
        self.guards.note_op("ascend"); // prove_bsp_triggers_operation（panic；BSP 或 emergence 触发）
    }

    /// **emergence_upgrade（自下而上仓位涌现）**：T 迭代涌现出更高级别（低级别走势完成 →
    /// 封装成上级单元）时，把核心仓 relabel 升级归属到涌现上界 `target_ladder`（= 复用
    /// `ascend`，无新资金、NAV 中性），**不必等该级别 BSP fire**——这是「自下而上」的实现，
    /// 补齐原引擎只有自上而下（区间套约束）的缺口。
    ///
    /// 缠论依据（第65课 `Move(k)≡Level-(k+1) 笔`）：你在低级别买点建的多头随走势发展，其低
    /// 级别走势类型组成更高级别的笔/段；该多头**本就是**高级别核心仓的组成，故升级 = 同一笔
    /// 仓位的级别重标定（relabel），非新建仓。
    ///
    /// 方向门控（H¹ 继承 + H⁰ 涌现方向校验）：仅当**核心仓操作极性 == 涌现走势方向对应极性**
    /// 时升级（持多∧涌现向上 / 持空∧涌现向下）。逆涌现方向的仓位不归属于该结构（等 BSP 翻转），
    /// 这才是「低级别同向走势组成高级别同向走势」的精确表达。
    ///
    /// 不变量复用 `ascend`：`cc = highest_active()` 是最高活跃层 ⟹ `target_ladder > cc` 必为
    /// idle，`ascend` 的「目标须 idle」断言自动满足。`highest_active = None`（全空）时无核心仓
    /// 可升，跳过——首仓仍由核心级 BSP `enter` 建立。
    ///
    /// 有效域上界：`target_ladder >= MAX_LADDER`（=11，即涌现 T-level ≥ 8）时跳过，核心停在原
    /// ladder（stream 侧 `ladder < MAX_LADDER` 守卫同样不写 emergent_top）。8 标的 25M bar 实测
    /// 涌现 T-level ≤ 5（ladder ≤ 8），**从未触达上界** ⟹ 此 cap 在当前数据有效域外（L2 读数，
    /// 非无条件不变量；若未来数据触达 level 8，此处是静默丢弃升级，需补观测计数）。
    fn emergence_upgrade(&mut self, target_ladder: usize, target_dir: Polarity) {
        self.guards.set_trigger(OpTrigger::Emergence); // 涌现是 ascend 的合法非 BSP 触发源
        if target_ladder >= MAX_LADDER {
            return;
        }
        if let Some(cc) = self.highest_active() {
            if self.layers[cc].direction == target_dir && cc < target_ladder {
                self.ascend(cc, target_ladder);
                self.res.n_emergence_upgrades += 1;
            }
        }
    }

    /// **sink @ (parent→sub)**（σ⁻¹∘τ）：父级减仓 m=u_P/3，次级别开 flip(d_P) 短差 m。父级真减仓（剩 2/3）。
    fn sink(&mut self, parent: usize, sub: usize, bar: i64, c: f64) {
        let u_p = self.layers[parent].units;
        let pdir = self.layers[parent].direction;
        let m = mobile_quota(u_p);
        if !(m > 1e-12 && m.is_finite()) || m > u_p + 1e-9 {
            return;
        }
        let mob = flip(pdir);
        // 子层方向相容（add_at 层内单一 direction 不变量）：sub 空 或 已是 mob 方向才可加。
        if self.layers[sub].is_active() && self.layers[sub].direction != mob {
            return;
        }
        // 同资本 sizing（编排者裁决 2026-06-21，与 rec_engine 逐字一致保 bit-exact）：开空 units =
        // reduce 释放的资本金额（m×parent.basis）在当前价 c 开空 = m×pb/c。价越高→同资本开的空头越少
        // →牛市空头累积减轻。pb 须在 reduce 前捕获（reduce 零化时 basis=NaN）；分组 (m*pb)/c 与 rec 相同。
        let pb = self.layers[parent].basis;
        let short_u = if pb.is_finite() && pb > 1e-12 {
            m * pb / c
        } else {
            m
        };
        if !(short_u > 1e-12 && short_u.is_finite()) {
            return;
        }
        let r0 = self.realized_total();
        // 移植守卫（L0，从 spiral/fugue_v3）：区间套向心下沉 sub<parent + σ-不变配额 m=u_P×MOBILE_FRAC。
        prove_sink_descends(parent, sub, bar);
        prove_sigma_quota(m, u_p, sub, bar);
        reduce_at(
            &mut self.layers,
            parent,
            m,
            &mut self.free,
            c,
            bar,
            &mut self.res,
            "reduce",
        );
        add_at(
            &mut self.layers,
            sub,
            short_u,
            mob,
            &mut self.free,
            c,
            bar,
            &mut self.res,
        );
        self.res.n_cycle_opens_by_ladder[parent] += 1;
        self.res.cross_level_closures += 1;
        // 核算父级 reduce 的 realized（父多=核心高位卖出降成本；父空=短差单独算）。
        self.account_reduce(pdir, self.realized_total() - r0, c);
        self.guards.note_op("sink"); // prove_bsp_triggers_operation（panic）
        self.guards.on_sink(); // prove_sink_recover_balance（campaign 内累计）
    }

    /// **recover @ (sub→parent)**（σ∘τ，ε 对称）：次级别走势完成 ⇒ **整条短差平清** m=u_sub（全量），
    /// 资金全额升回父级 d_P 方向。编排者裁决 2026-06-19（方案②，543号开放轴#1）：次级别买点=次级别走势
    /// **完成**（0/1 事件，非配额事件）⇒ 全量了结而非 σ-不变 1/3。
    fn recover(&mut self, parent: usize, sub: usize, bar: i64, c: f64) {
        let pdir = self.layers[parent].direction;
        let mob = flip(pdir);
        // ε 对称：sub 必须持反父向短差（mob）才能升回。
        if !self.layers[sub].is_active() || self.layers[sub].direction != mob {
            return;
        }
        let m = self.layers[sub].units; // 全量：次级别走势完成则整条短差平清（非 1/3 配额）
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        // 同资本反算（编排者裁决 2026-06-21，与 rec_engine 逐字一致保 bit-exact）：平空释放名义资本 =
        // m×sub.basis（= sink 时下放的核心资本）；归还核心 units = 该资本 / parent.basis ⟹ give = m×sb/pb
        // （parent.basis 不变时 = m，恢复原股数；deploy_earning 降本后 give>m 按资本恢复）。sb 须在
        // reduce(sub) 前捕获（reduce 零化 sub→basis=NaN）；分组 (m*sb)/pb 与 rec 相同。
        let sb = self.layers[sub].basis;
        let pb = self.layers[parent].basis;
        let give = if pb.is_finite() && pb > 1e-12 {
            m * sb / pb
        } else {
            m
        };
        if !(give > 1e-12 && give.is_finite()) {
            return;
        }
        let r0 = self.realized_total();
        // 移植守卫（L0）：recover 升回与 sink 向心配对，同守 sub<parent（次级别走势完成升回父级）。
        prove_sink_descends(parent, sub, bar);
        reduce_at(
            &mut self.layers,
            sub,
            m,
            &mut self.free,
            c,
            bar,
            &mut self.res,
            "recover",
        );
        add_at(
            &mut self.layers,
            parent,
            give,
            pdir,
            &mut self.free,
            c,
            bar,
            &mut self.res,
        );
        self.res.n_cycle_closes_by_ladder[parent] += 1;
        // 核算 sub reduce 的 realized：sub=反父向短差腿⇒ 短差单独算，**不入降成本**。
        let realized = self.realized_total() - r0;
        self.account_reduce(mob, realized, c);
        self.guards.note_op("recover"); // prove_bsp_triggers_operation（panic）
        self.guards.on_recover(sub, realized); // prove_sink_recover_balance + prove_per_level_pnl
                                               // ③ 增股数：EarningShares 阶段 recover = 买点，部署纯利润买更多核心 units。
        if pdir == Polarity::Long {
            self.deploy_earning(parent, bar, c);
        }
    }

    /// **drain @ j**：子级持同父向遗留仓 → 反父向 BSP 减暴露 1/3（不翻转，父级主导）。
    /// drain 的 realized 银行进 campaign：CostReduction 降 cost_basis，EarningShares 入 earning_cash。
    fn drain(&mut self, j: usize, bar: i64, c: f64) {
        let u = self.layers[j].units;
        let m = mobile_quota(u);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let jdir = self.layers[j].direction; // 同父向遗留仓（父多→j 多）⇒ Long-reduce 降成本
        let r0 = self.realized_total();
        // 移植守卫（L0）：drain 减暴露配额 σ-不变（m=u_j×MOBILE_FRAC，级别无关）。
        prove_sigma_quota(m, u, j, bar);
        reduce_at(
            &mut self.layers,
            j,
            m,
            &mut self.free,
            c,
            bar,
            &mut self.res,
            "drain",
        );
        let realized = self.realized_total() - r0;
        self.account_reduce(jdir, realized, c);
        self.guards.note_op("drain"); // prove_bsp_triggers_operation（panic）
        if jdir == Polarity::Short {
            self.guards.on_short_pnl(j, realized); // prove_per_level_pnl（同向遗留短差腿平仓）
        }
    }

    // ──────────────── BSP 路由（区间套 top-down）────────────────

    /// 单个 BSP（ladder j，is_buy）的路由：查父级 → 子级 sink/recover/drain；无父级 → 核心 enter/ascend/flip。
    fn route_bsp(&mut self, j: usize, is_buy: bool, bar: i64, c: f64) {
        self.guards.set_trigger(OpTrigger::Bsp); // 本路由触发的所有原子操作归因 BSP
        match self.nearest_active_parent(j) {
            // ── 子级（有活跃祖先 P）：区间套约束，永不独立翻转 ──
            Some(p) => {
                let pdir = self.layers[p].direction;
                let mob = flip(pdir); // 短差方向 = 反父向
                let is_reduce = match pdir {
                    Polarity::Long => !is_buy, // 父多：卖点=减仓信号
                    Polarity::Short => is_buy, // 父空：买点=减仓信号
                };
                if is_reduce {
                    // 反父向 BSP：sink（空/已是短差）或 drain（遗留同父向仓）。
                    if !self.layers[j].is_active() || self.layers[j].direction == mob {
                        let p_is_core = self.highest_active() == Some(p);
                        let m = mobile_quota(self.layers[p].units);
                        self.sink(p, j, bar, c);
                        let opened = self.layers[j].is_active() && self.layers[j].direction == mob;
                        let d = &mut self.op_diag;
                        if is_buy {
                            d.buy_sink += 1
                        } else {
                            d.sell_sink += 1
                        }
                        if opened {
                            d.last_sink_bar[j] = bar;
                        }
                        if p_is_core {
                            d.n_sink_on_core += 1;
                            d.sink_core_reduced_units += m;
                        }
                    } else {
                        self.drain(j, bar, c);
                        if is_buy {
                            self.op_diag.buy_drain += 1
                        } else {
                            self.op_diag.sell_drain += 1
                        }
                    }
                } else {
                    // 同父向 BSP：recover（j 持短差则平整条升回父级）；否则 no-op（不 pyramid）。
                    if self.layers[j].is_active() && self.layers[j].direction == mob {
                        let m = self.layers[j].units;
                        let p_is_core = self.highest_active() == Some(p);
                        let sb = self.op_diag.last_sink_bar[j];
                        self.recover(p, j, bar, c);
                        let d = &mut self.op_diag;
                        if is_buy {
                            d.buy_recover += 1
                        } else {
                            d.sell_recover += 1
                        }
                        if sb >= 0 {
                            let iv = bar - sb;
                            d.recover_interval_sum += iv as f64;
                            d.recover_interval_count += 1;
                            d.recover_interval_max = d.recover_interval_max.max(iv);
                            d.last_sink_bar[j] = -1;
                        }
                        if p_is_core {
                            d.n_recover_on_core += 1;
                            d.recover_core_restored_units += m;
                        }
                    } else if is_buy {
                        self.op_diag.buy_noop += 1;
                        self.op_diag.buy_noop_no_short += 1;
                    } else {
                        self.op_diag.sell_noop += 1
                    }
                }
            }
            // ── 核心级（无活跃祖先：j 是最高活跃或全空）：唯一独立翻转点 ──
            None => {
                let dir = if is_buy {
                    Polarity::Long
                } else {
                    Polarity::Short
                };
                match self.highest_active() {
                    None => {
                        self.enter(j, dir, bar, c); // 首次建仓
                        if is_buy {
                            self.op_diag.buy_enter += 1
                        } else {
                            self.op_diag.sell_enter += 1
                        }
                    }
                    Some(cc) => {
                        let cdir = self.layers[cc].direction;
                        if dir == cdir {
                            // 同向：更高空 ladder ⇒ ascend 骑乘；同 ladder ⇒ no-op。
                            if j > cc && !self.layers[j].is_active() {
                                self.ascend(cc, j);
                                if is_buy {
                                    self.op_diag.buy_ascend += 1
                                } else {
                                    self.op_diag.sell_ascend += 1
                                }
                            } else if is_buy {
                                self.op_diag.buy_noop += 1
                            } else {
                                self.op_diag.sell_noop += 1
                            }
                        } else {
                            // 反向：核心翻转（走势终完美→新走势）——清全塔 + 同 ladder 反向 enter。
                            // 诊断（观测）：翻转方向 + 清掉的多头 units（强牛中卖点翻空 = 清牛市多头做空）。
                            let long_cleared = self.core_long_units();
                            self.clear_all(bar, c, "flip");
                            self.enter(j, dir, bar, c);
                            let d = &mut self.op_diag;
                            if is_buy {
                                d.buy_flip += 1;
                                d.flip_from_short += 1; // 原核心持空（cdir≠Long）
                            } else {
                                d.sell_flip += 1;
                                if cdir == Polarity::Long {
                                    d.flip_from_long += 1; // 卖点翻空且原核心持多 = 强牛做空风险
                                    d.flip_long_units_cleared += long_cleared;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ──────────────── 单 bar 步进 ────────────────

    /// 单 bar 操作步进：强平 → BSP top-down 路由（高 ladder 先更新 ⟹ 子级读父级本 bar 最新状态）。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        self.guards.set_trigger(OpTrigger::None); // 本 bar 起始无触发源（强平不经原子函数）
                                                  // 守恒量 = 总财富（free + Σ持仓 + withdrawn）：退本金把现金移出在险池，TW 守恒（NAV 不守恒）。
        let tw_pre = self.total_wealth(c);

        // ── A. 边界算子：保证金强平，**按三阶段切换模式**（编排者裁决 2026-06-20）。
        //   强平 realized 也银行（损失抬高 cost_basis）。
        //   - CostReduction/CapitalRecovered → **逐仓**（每层独立 basis×SUB_LIQ_FACTOR）：本金在险，
        //     次级别短差亏损不侵蚀核心仓保证金（每层独立判，现状）。
        //   - EarningShares → **全仓**（账户级共享保证金）：本金已退安全池（withdrawn），仓位由纯利润支撑，
        //     仅账户净值（in-system NAV，不含 withdrawn）≤0 时连锁全平。1x 几何塔（核心多头 Σlong>Σshort
        //     主导）下 NAV≥0 恒成立 ⟹ 此条几乎不触发 = 短差腿回撤被共享池吸收、不被单层提前止损（立于
        //     不败使连锁爆仓风险可接受）。**有效域标注**：杠杆期货上账户级判据应为 NAV≤maintenance>0；
        //     此处 NAV≤0 是 1x 现货口径（withdrawn 已隔离不作保证金，故连锁只损失利润不触本金）。
        match self.stage {
            TStage::EarningShares => {
                // 全仓：账户级，所有层共享。in-system NAV≤0 ⇒ 连锁全平（principal 在 withdrawn 安全）。
                if self.total_nav(c) <= 0.0 {
                    for k in 0..MAX_LADDER {
                        let kdir = self.layers[k].direction;
                        let u = self.layers[k].units;
                        if u > 1e-12 {
                            let r0 = self.realized_total();
                            reduce_at(
                                &mut self.layers,
                                k,
                                u,
                                &mut self.free,
                                c,
                                bar,
                                &mut self.res,
                                "liq_cross",
                            );
                            self.res.n_liquidations_by_ladder[k] += 1;
                            let realized = self.realized_total() - r0;
                            self.account_reduce(kdir, realized, c);
                            if kdir == Polarity::Short {
                                self.guards.on_short_pnl(k, realized); // prove_per_level_pnl（强平空头腿）
                            }
                        }
                    }
                }
                // 否则：不做任何 per-layer 强平（共享保证金吸收单层回撤）。
            }
            _ => {
                // 逐仓：每层独立 basis 判（CostReduction / CapitalRecovered）。
                for k in 0..MAX_LADDER {
                    let l = self.layers[k];
                    if l.units > 1e-12 {
                        let liq = match l.direction {
                            Polarity::Long => c > 0.0 && c <= l.basis / SUB_LIQ_FACTOR,
                            Polarity::Short => c >= SUB_LIQ_FACTOR * l.basis,
                        };
                        if liq {
                            let r0 = self.realized_total();
                            reduce_at(
                                &mut self.layers,
                                k,
                                l.units,
                                &mut self.free,
                                c,
                                bar,
                                &mut self.res,
                                "liq",
                            );
                            self.res.n_liquidations_by_ladder[k] += 1;
                            // 强平 realized 按被平层方向核算（多头强平=亏损抬 cost_basis；空头腿强平=短差单独算）。
                            let realized = self.realized_total() - r0;
                            self.account_reduce(l.direction, realized, c);
                            if l.direction == Polarity::Short {
                                self.guards.on_short_pnl(k, realized); // prove_per_level_pnl（强平空头腿）
                            }
                        }
                    }
                }
            }
        }

        // ── A''. 核心走势完成 → 主动清仓（campaign 边界重定义，546号死锁解锁；与 rec 对称）──
        //   缠论依据（fengkong/chanlun-trading-system 退出条件 = 买入程序判断条件被否定 / 走势终完美）：
        //   核心仓骑的走势在**该 level 顶/底背驰（type1）**完成 ⇒ 买入逻辑被否定 ⇒ 清仓到现金。
        //   这是 enter 重建的**第三条路**，独立于 flip：clear_all → reset_campaign → highest_active None
        //   ⇒ 死锁（核心 units 几何衰减永不归零 → highest_active 恒 Some → enter 永不触发）解除，
        //   **下一个买点**经核心级 enter 重建（不在本 bar 反向 enter ⇒ 避免 545 做空陷阱）。
        //   触发源 = type1 BSP（走势完成的可观测形式）⇒ guards 归因 Bsp。**不动 EPS、不动几何衰减**。
        if let (true, Some(cc)) = (self.enable_trend_done_clear, self.highest_active()) {
            let core_trend_done = match self.layers[cc].direction {
                Polarity::Long => cc < MAX_LADDER && view.t1sell[cc], // 顶背驰：上涨核心走势终完美
                Polarity::Short => cc < MAX_LADDER && view.t1buy[cc], // 底背驰：下跌核心走势终完美
            };
            if core_trend_done {
                self.guards.set_trigger(OpTrigger::Bsp); // 走势完成 = type1 BSP 驱动（合法触发源）
                self.clear_all(bar, c, "trend_done");
                self.n_trend_done_clears += 1;
                // 全平到现金 ⇒ TW 中性（同价 c）。本 bar 不再 route_bsp（等下一买点 enter 重建）。
                prove_nav_neutral(tw_pre, self.total_wealth(c), bar);
                return;
            }
        }

        // ── A'. 自下而上仓位涌现升级（BSP 路由前：核心仓先骑乘涌现上界，本 bar 低级别 BSP
        //         随后以升级后的高级别核心为父级 → 走 sink/recover 短差，而非被误判为核心翻转）──
        if let Some((target_ladder, target_dir)) = view.emergent_top {
            self.emergence_upgrade(target_ladder, target_dir);
        }

        // ── B. BSP 路由：top-down（高 ladder 先，区间套约束自上而下）──
        for j in (0..MAX_LADDER).rev() {
            let b = view.buy[j];
            let s = view.sell[j];
            // 信号层密度（观测，Q2：各 ladder 买/卖信号数；冲突信号也计入，反映原始信号）。
            if b {
                self.op_diag.buy_sig_by_ladder[j] += 1;
            }
            if s {
                self.op_diag.sell_sig_by_ladder[j] += 1;
            }
            if b && s {
                continue; // 同 bar 同 ladder 买卖冲突 → 跳过（歧义）
            }
            if b {
                self.route_bsp(j, true, bar, c);
            } else if s {
                self.route_bsp(j, false, bar, c);
            }
        }

        // ── 守卫：全局总财富中性（同价 c 全部操作前后中性；退本金 free→withdrawn 在 TW 内守恒）──
        prove_nav_neutral(tw_pre, self.total_wealth(c), bar);

        // ── prove_direction_matches_trend（观测）：核心方向应 = 最高走势类型方向 ──
        // emergent_top 是本 bar 最高 completed 走势方向；highest_active 是核心仓方向。emergence_upgrade
        // 的同向 gate 应使二者匹配——失配 = 核心被低级别 BSP flip / 逆涌现未升级（已知强牛违反）。
        if let Some((_, edir)) = view.emergent_top {
            let core_dir = self.highest_active().map(|cc| self.layers[cc].direction);
            self.guards.check_direction(core_dir, edir);
        }

        // ── 观测 ──
        let (long_u, short_u) = exposure(&self.layers);
        if long_u > 0.0 {
            self.res.phys_long_bars += 1;
        }
        if short_u > 0.0 {
            self.res.phys_short_bars += 1;
        }
        // 真空期/敞口态（编排者：绝对值敞口,零真空）。四态互斥，和=总 bar。
        let has_long = long_u > 1e-12;
        let has_short = short_u > 1e-12;
        let d = &mut self.op_diag;
        match (has_long, has_short) {
            (false, false) => d.n_vacuum_bars += 1,
            (true, false) => d.n_long_only_bars += 1,
            (false, true) => d.n_short_only_bars += 1,
            (true, true) => d.n_both_bars += 1,
        }
        if has_long || has_short {
            if d.first_active_bar < 0 {
                d.first_active_bar = bar;
            }
            d.last_active_bar = bar;
            d.cur_vacuum_run = 0;
        } else if d.first_active_bar >= 0 {
            // 首次建仓后的真空（执行断链候选）：累计连续 run，记最长。
            d.cur_vacuum_run += 1;
            d.max_vacuum_gap = d.max_vacuum_gap.max(d.cur_vacuum_run);
        }
        for l in &self.layers {
            if l.units > 1e-12 && l.direction == Polarity::Short {
                self.res.short_held_bars_by_ladder[l.ladder.min(MAX_LADDER - 1)] += 1;
            }
        }
        // 相邻同向占用级别对数（T24 观测：几何塔应手性交替 ⟹ 同向对 = 异常提示，非 panic）。
        let mut chiral = 0u64;
        for k in 0..MAX_LADDER.saturating_sub(1) {
            let a = &self.layers[k];
            let b = &self.layers[k + 1];
            if a.units > 1e-12 && b.units > 1e-12 && a.direction == b.direction {
                chiral += 1;
            }
        }
        self.res.max_chiral_same_dir = self.res.max_chiral_same_dir.max(chiral);
        self.max_concurrent_seen = self
            .max_concurrent_seen
            .max(active_voice_count(&self.layers));

        if bar % EQUITY_SAMPLE_BARS == 0 {
            // 总财富（含已退本金的安全池）——退本金后 in-system NAV 会低估真实财富。
            self.res.equity.push((bar, self.total_wealth(c)));
            self.res.exposure_series.push((bar, long_u, short_u));
        }
    }

    /// 收尾（末 bar equity 补采样 + 全塔平仓到现金）。`last_bar=None`（零 bar）⇒ final_nav = free。
    pub fn finish(&mut self, last_bar: Option<i64>) {
        if let Some(lb) = last_bar {
            let c_last = self.last_close;
            if lb % EQUITY_SAMPLE_BARS != 0 {
                self.res.equity.push((lb, self.total_wealth(c_last)));
                let (lu, su) = exposure(&self.layers);
                self.res.exposure_series.push((lb, lu, su));
            }
            self.guards.set_trigger(OpTrigger::Eod); // eod 是 clear_all 的合法非 BSP 触发源
            self.clear_all(lb, c_last, "eod"); // clear_all → reset_campaign 归还 withdrawn 到 free
        }
        // 全平后 free = 净值（所有仓位转现金 + 已退本金归还）。
        self.res.final_nav = self.free;
        self.res.max_concurrent_voices = self.max_concurrent_seen as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.buy[k] = true;
        v
    }

    fn sell_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.sell[k] = true;
        v
    }

    // ──────────────── 核心仓 enter / flip / ascend ────────────────

    #[test]
    fn 核心_首个买点全仓建多() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        assert_eq!(eng.layers[6].direction, Polarity::Long);
        let units = INITIAL_CAPITAL / 100.0;
        assert!(
            (eng.layers[6].units - units).abs() < 1e-6,
            "全仓 {units} units（单一 free 池）"
        );
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "建仓 NAV 中性");
    }

    #[test]
    fn 核心_最高级别反向翻转清全塔() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心 Long@6
        eng.step(&sell_view(6), 10, 110.0); // 最高级别卖点 → 翻空
        assert_eq!(eng.layers[6].direction, Polarity::Short, "核心翻空");
        let (_, lu, su) = (0.0, exposure(&eng.layers).0, exposure(&eng.layers).1);
        assert_eq!(lu, 0.0, "多头清掉");
        assert!(su > 0.0, "建反向空");
    }

    #[test]
    fn 核心_同向更高级别ascend骑乘上移() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5
        let u5 = eng.layers[5].units;
        eng.step(&buy_view(7), 5, 100.0); // 更高级别同向买点 → ascend 上移到 7
        assert!(!eng.layers[5].is_active(), "原核心 5 已上移（idle）");
        assert_eq!(eng.layers[7].direction, Polarity::Long);
        assert!(
            (eng.layers[7].units - u5).abs() < 1e-9,
            "ascend 仅 relabel，units 不变（无新资金）"
        );
    }

    // ──────────────── 自下而上仓位涌现升级（emergence-upgrade）────────────────

    /// 带涌现上界的信号视图（自下而上升级测试用）。
    fn emergent_view(ladder: usize, dir: Polarity) -> TSignalView {
        let mut v = TSignalView::empty();
        v.emergent_top = Some((ladder, dir));
        v
    }

    #[test]
    fn 涌现升级_核心多头升到涌现ladder_方向一致() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4), 0, 100.0); // 核心 Long@4（全仓）
        let u4 = eng.layers[4].units;
        // T 迭代涌现出更高级别（ladder 7，向上走势）→ 核心仓 relabel 升级归属，不等高级别 BSP。
        eng.step(&emergent_view(7, Polarity::Long), 10, 105.0);
        assert!(!eng.layers[4].is_active(), "原核心 4 已升级（idle）");
        assert_eq!(
            eng.layers[7].direction,
            Polarity::Long,
            "核心归属到涌现 ladder 7"
        );
        assert!(
            (eng.layers[7].units - u4).abs() < 1e-9,
            "升级仅 relabel，units 不变（不新建仓）"
        );
        assert_eq!(eng.result().n_emergence_upgrades, 1);
        // NAV 中性：Long u4 @basis100，@105 ⟹ NAV = u4×105（升级不动钱）。
        let (navv, lu, su, _) = eng.snapshot();
        assert!((navv - u4 * 105.0).abs() < 1e-6, "升级 NAV 中性，得 {navv}");
        assert!((lu - u4).abs() < 1e-9 && su == 0.0, "多头敞口不变");
    }

    #[test]
    fn 涌现升级_方向不一致不升级() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4), 0, 100.0); // 核心 Long@4
                                          // 涌现向下走势（Short 归属）但核心持多 → 方向不一致，不升级（逆涌现方向不归属，等 BSP 翻转）。
        eng.step(&emergent_view(7, Polarity::Short), 10, 105.0);
        assert!(eng.layers[4].is_active(), "核心仍在原 ladder 4");
        assert!(!eng.layers[7].is_active(), "涌现 ladder 7 未被占用");
        assert_eq!(eng.result().n_emergence_upgrades, 0);
    }

    #[test]
    fn 涌现升级_全空时no_op() {
        let mut eng = TPositionEngine::new();
        // 无核心仓时涌现信号不建仓（首仓仍由核心级 BSP enter 建立）。
        eng.step(&emergent_view(7, Polarity::Long), 0, 100.0);
        assert!(eng.layers.iter().all(|l| !l.is_active()), "全空，无仓可升");
        assert_eq!(eng.result().n_emergence_upgrades, 0);
    }

    #[test]
    fn 涌现升级_已在更高ladder不下移() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(8), 0, 100.0); // 核心 Long@8
                                          // 涌现上界 ladder 6 < 核心 8 → 不下移（幂等：核心已足够高）。
        eng.step(&emergent_view(6, Polarity::Long), 10, 105.0);
        assert!(eng.layers[8].is_active(), "核心仍在 8");
        assert!(!eng.layers[6].is_active(), "不下移到 6");
        assert_eq!(eng.result().n_emergence_upgrades, 0);
    }

    #[test]
    fn 涌现升级后低级别卖点变子级sink() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4), 0, 100.0); // 核心 Long@4
        eng.step(&emergent_view(8, Polarity::Long), 10, 100.0); // 升级到核心 Long@8
        let u8 = eng.layers[8].units;
        // 升级后低级别 5 卖点：以核心 8（Long）为父级 → sink（父减仓 1/3 + 5 开反向短差），非翻转。
        eng.step(&sell_view(5), 20, 100.0);
        assert_eq!(
            eng.layers[8].direction,
            Polarity::Long,
            "核心仍持多（非误判为翻转）"
        );
        assert!(
            (eng.layers[8].units - u8 * 2.0 / 3.0).abs() < 1e-6,
            "父级真减仓到 2/3"
        );
        assert_eq!(
            eng.layers[5].direction,
            Polarity::Short,
            "次级别开反向短差（sink）"
        );
    }

    // ──────────────── 子级 sink（父级真减仓，核心新语义）────────────────

    #[test]
    fn sink_父持多子卖点父级真减仓三分之一() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心 Long@6
        let u6 = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 100.0); // 子级 5 卖点（父6持多）→ sink
        assert!(
            (eng.layers[6].units - u6 * 2.0 / 3.0).abs() < 1e-6,
            "父级减到 2/3，得 {}",
            eng.layers[6].units
        );
        assert_eq!(eng.layers[5].direction, Polarity::Short, "次级别开反向短差");
        assert!(
            (eng.layers[5].units - u6 / 3.0).abs() < 1e-6,
            "短差 = 1/3 父级 units（下放）"
        );
        let (lu, su) = exposure(&eng.layers);
        assert!((lu - u6 * 2.0 / 3.0).abs() < 1e-6 && (su - u6 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn recover_父持多子买点短差全平清升回父级() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        let u6_core = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 100.0);
        let u6_after_sink = eng.layers[6].units;
        let u5_short = eng.layers[5].units;
        eng.step(&buy_view(5), 20, 90.0);
        assert!(
            eng.layers[5].units < 1e-9,
            "短差全平清，得 {}",
            eng.layers[5].units
        );
        assert!(
            (eng.layers[6].units - (u6_after_sink + u5_short)).abs() < 1e-6,
            "父级全额升回核心仓"
        );
        assert!(
            (eng.layers[6].units - u6_core).abs() < 1e-6,
            "核心仓恢复 sink 前水平"
        );
        let mob = eng.result().mobile_realized_pnl_by_ladder[5];
        assert!(mob > 0.0, "短差降价回补盈利，得 {mob}");
    }

    #[test]
    fn 子级永不独立翻转_父持多子卖点是sink非独立做空() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        let u6 = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 100.0);
        assert!(
            (eng.layers[5].units - u6 / 3.0).abs() < 1e-6,
            "子级短差恰 1/3 父级（非独立全仓）"
        );
        assert!(
            eng.layers[6].units < u6,
            "父级被减仓（非独立做空时父级不动）"
        );
    }

    // ──────────────── 四步循环 + 手性几何塔 ────────────────

    #[test]
    fn 四步循环_完整齿轮咬合() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        let u6 = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 105.0);
        assert!((eng.layers[6].units - u6 * 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(eng.layers[5].direction, Polarity::Short);
        eng.step(&buy_view(5), 20, 100.0);
        assert_eq!(eng.layers[6].direction, Polarity::Long, "核心仍持多");
        eng.step(&sell_view(6), 30, 120.0);
        assert_eq!(eng.layers[6].direction, Polarity::Short, "核心方向变");
    }

    #[test]
    fn 手性几何塔_相邻级别方向相反() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0);
        let u5_at_sink = eng.layers[5].units;
        eng.step(&buy_view(4), 20, 100.0);
        assert_eq!(eng.layers[6].direction, Polarity::Long);
        assert_eq!(eng.layers[5].direction, Polarity::Short, "相邻反向");
        assert_eq!(eng.layers[4].direction, Polarity::Long, "手性交替");
        assert!(
            (eng.layers[4].units - u5_at_sink / 3.0).abs() < 1e-6,
            "次次级别 = sink时次级别 1/3"
        );
        assert!(
            (eng.layers[5].units - u5_at_sink * 2.0 / 3.0).abs() < 1e-6,
            "次级别被次次级别 sink 减到 2/3"
        );
    }

    // ──────────────── 守恒 / 边界 ────────────────

    #[test]
    fn 总nav守恒_sink_recover同价不创造价值() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心多
        eng.step(&sell_view(5), 5, 100.0); // sink（同价）
        eng.step(&buy_view(5), 10, 100.0); // recover（同价）
        eng.step(&sell_view(6), 15, 100.0); // 核心翻空（同价）
        let (navv, _, _, _) = eng.snapshot();
        assert!(
            (navv - INITIAL_CAPITAL).abs() < 1e-4,
            "同价全操作总 NAV 守恒，得 {navv}"
        );
    }

    #[test]
    fn 空输入零操作守恒() {
        let mut eng = TPositionEngine::new();
        for i in 0..5 {
            eng.step(&TSignalView::empty(), i, 100.0);
        }
        eng.finish(Some(4));
        assert!(eng.result().trades.is_empty(), "无信号无 trade");
        assert!(
            (eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-6,
            "总资金守恒"
        );
    }

    #[test]
    fn 核心多头强平到现金() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // Long@6 @100
        eng.step(&TSignalView::empty(), 10, 50.0); // c≤basis/2 强平多头
        assert!(!eng.layers[6].is_active(), "多头强平归零");
        assert_eq!(eng.res.n_liquidations_by_ladder[6], 1);
    }

    #[test]
    fn 短差空头价格翻倍爆仓() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0);
        assert_eq!(eng.layers[5].direction, Polarity::Short);
        eng.step(&TSignalView::empty(), 20, 200.0);
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1, "短差空腿强平");
        assert!(!eng.layers[5].is_active(), "短差归零");
    }

    #[test]
    fn 收尾全平到现金final_nav() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0);
        eng.finish(Some(10));
        // 全平后所有层 idle。
        assert!(eng.layers.iter().all(|l| !l.is_active()), "收尾全平");
        assert!(
            (eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-4,
            "同价全平 final_nav 守恒"
        );
    }

    // ──────────────── 持仓三阶段（缠师第31课）────────────────

    #[test]
    fn 三阶段_建仓即降成本阶段_notional记本金() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心 Long@6，notional = INITIAL
        assert_eq!(eng.stage(), TStage::CostReduction, "建仓即降成本阶段");
        let (_, notional, withdrawn, _, cb) = eng.stage_snapshot();
        assert!(
            (notional - INITIAL_CAPITAL).abs() < 1e-6,
            "notional = 建仓本金"
        );
        assert!(withdrawn.abs() < 1e-9, "初始无 withdrawn");
        assert!((cb - 100.0).abs() < 1e-6, "core_cost_basis = 入场价 100");
    }

    #[test]
    fn 三阶段_核心高位卖出降cost_basis_短差腿单独算() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        let cb0 = eng.cost_basis();
        eng.step(&sell_view(5), 10, 150.0);
        let cb1 = eng.cost_basis();
        assert!(cb1 < cb0 - 1.0, "核心高位卖出降 cost_basis：{cb1} < {cb0}");
        assert_eq!(eng.stage(), TStage::CostReduction, "单次远未退本金");
        let sl0 = eng.result().short_leg_pnl;
        eng.step(&buy_view(5), 20, 130.0);
        assert!(
            (eng.cost_basis() - cb1).abs() < 1e-6,
            "recover 平短差不改 cost_basis（短差腿单独算）"
        );
        assert!(
            (eng.result().short_leg_pnl - sl0).abs() > 1e-9,
            "短差腿 pnl 单独累计"
        );
    }

    /// 核心随价格上涨、sink 高位卖出降 cost_basis 穿零 → 退本金 → 增股数（核心 units 增长）。
    #[test]
    fn 三阶段_核心高位卖出穿零退本金再增股数() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // cost_basis=100, notional=100000, U=1000
        let u0 = eng.layers[6].units;
        let mut bar = 1;
        let mut p = 100.0_f64;
        let mut reached = false;
        for _ in 0..30 {
            p *= 1.4; // 价格持续上涨（强趋势）
            eng.step(&sell_view(5), bar, p); // sink：高位卖核心 → realized 降 cost_basis
            bar += 1;
            eng.step(&buy_view(5), bar, p * 0.85); // recover：平短差(单独算) + 回补核心
            bar += 1;
            if eng.stage() == TStage::EarningShares {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "核心高位卖出降成本应穿零退本金，末态 {:?} cost_basis={}",
            eng.stage(),
            eng.cost_basis()
        );
        assert!(eng.result().n_capital_recovered >= 1, "退本金触发");
        assert!(eng.withdrawn > 0.0, "本金移出安全池，得 {}", eng.withdrawn);
        // 增股数：EarningShares 阶段继续 sink/recover → recover 买点 deploy_earning 买更多核心。
        for _ in 0..10 {
            p *= 1.2;
            eng.step(&sell_view(5), bar, p);
            bar += 1;
            eng.step(&buy_view(5), bar, p * 0.85);
            bar += 1;
        }
        assert!(eng.result().n_earning_deploys >= 1, "增股数部署至少一次");
        assert!(
            eng.core_long_units() > u0 + 1e-6,
            "增股数后核心 units {} > 初始 {u0}",
            eng.core_long_units()
        );
    }

    #[test]
    fn 三阶段_总财富守恒_穿越全部阶段不创造价值() {
        // 每 bar prove_nav_neutral(tw_pre, tw_post) 守卫总财富（含 withdrawn）；此处验证末态 + 收尾。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        let mut bar = 1;
        let mut p = 100.0_f64;
        for _ in 0..20 {
            p *= 1.4;
            eng.step(&sell_view(5), bar, p);
            bar += 1;
            eng.step(&buy_view(5), bar, p * 0.85);
            bar += 1;
        }
        let tw = eng.total_wealth(p * 0.85);
        assert!(
            tw > INITIAL_CAPITAL,
            "总财富增长（趋势+短差，非凭空），得 {tw}"
        );
        eng.finish(Some(bar)); // 收尾全平 + 归还 withdrawn
        assert!(eng.layers.iter().all(|l| !l.is_active()), "收尾全平");
        assert!(eng.withdrawn.abs() < 1e-9, "withdrawn 已归还");
        assert!(
            eng.result().final_nav > INITIAL_CAPITAL,
            "最终财富 > 本金，得 {}",
            eng.result().final_nav
        );
    }

    /// 保证金模式耦合（编排者裁决 2026-06-20）：EarningShares 阶段切全仓——短差腿在 c≥2×basis
    /// 不被单层强平（共享池吸收），区别于 CostReduction 逐仓（`短差空头价格翻倍爆仓` 测试会爆）。
    /// 上涨序列驱动到 EarningShares（核心高位卖出降成本穿零退本金），返回 (last_bar, last_price)。
    fn drive_to_earning(eng: &mut TPositionEngine) -> (i64, f64) {
        eng.step(&buy_view(6), 0, 100.0);
        let mut bar = 1;
        let mut p = 100.0_f64;
        for _ in 0..30 {
            p *= 1.4;
            eng.step(&sell_view(5), bar, p);
            bar += 1;
            eng.step(&buy_view(5), bar, p * 0.85);
            bar += 1;
            if eng.stage() == TStage::EarningShares {
                break;
            }
        }
        (bar, p)
    }

    #[test]
    fn 三阶段_全仓模式_增股数阶段短差腿不单层强平() {
        let mut eng = TPositionEngine::new();
        let (mut bar, p) = drive_to_earning(&mut eng);
        assert_eq!(eng.stage(), TStage::EarningShares, "已进增股数（全仓模式）");
        eng.step(&sell_view(5), bar, p);
        bar += 1;
        let short_opened = eng.layers[5].is_active() && eng.layers[5].direction == Polarity::Short;
        let liq_before = eng.res.n_liquidations_by_ladder[5];
        eng.step(&TSignalView::empty(), bar, p * 2.5);
        if short_opened {
            assert_eq!(
                eng.res.n_liquidations_by_ladder[5], liq_before,
                "增股数阶段短差腿不单层强平（全仓共享池吸收，区别于逐仓 c≥2basis 爆仓）"
            );
            assert!(eng.layers[5].is_active(), "短差腿存活（未被逐仓止损）");
        }
    }

    #[test]
    fn 三阶段_flip重置campaign归还退本金() {
        // 进入退本金后核心反向翻转：clear_all 归还 withdrawn，重置阶段。
        let mut eng = TPositionEngine::new();
        let (bar, p) = drive_to_earning(&mut eng);
        assert_eq!(eng.stage(), TStage::EarningShares, "已进增股数");
        assert!(eng.withdrawn > 0.0, "有退本金在安全池");
        // 核心反向翻转（最高级别卖点）→ clear_all → reset_campaign。
        eng.step(&sell_view(6), bar, p * 0.5);
        assert_eq!(eng.stage(), TStage::CostReduction, "翻转后重置回降成本阶段");
        assert!(eng.withdrawn.abs() < 1e-9, "退本金已归还 free");
    }
}
