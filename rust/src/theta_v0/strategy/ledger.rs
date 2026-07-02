//! 双账本分量——契约锚点 = Origin canonical（task #102 A′ Phase2 step8 重锚）。
//!
//! ## 契约重锚（legacy Strict/Tlayers → Origin canonical）
//!
//! A′ Phase1（#97）已把 `formal/Origin/` 立为唯一 canonical base，legacy `Strict/Tlayers/
//! Foundation` 降为待重锚 reference。本文件的契约语义**锚点指向 Origin**（不再以 legacy
//! Strict/Tlayers 为接口权威）：
//!
//! - **R=Π-A-W 账本（LedgerComp）契约 → `Origin.FullDefinitionStrategy.LedgerState`**：Origin
//!   把恒等作为结构不变量字段 `inv : R = Pi - A - W`（FullDefinitionStrategy.lean:184-190），
//!   `ledgerStep`（:195-196）的每个算子由 `mkLedger` 显式重算 R，`ledger_invariant_preservation`
//!   （:198-203）证保恒等。本 Rust `LedgerComp`/`ledger_step`/`inv_holds` 镜像该 Origin 接口语义。
//!   接口别名见 `OriginAdapters/StrictPipeline.lean` `FullDefinitionIface`。
//!
//! ## TW 三阶段 / OQ-9 gate 契约 → `Origin.TotalWealth`（task #127 native port 已落地，重锚完成）
//!
//! ★A′ Phase2 #127 重锚（Origin-TW #103 = native）：取本金三阶段 TW 已 **native port 进 Origin
//! canonical base** —— `formal/Origin/TotalWealth.lean`（`namespace NewChanlun.Origin.TotalWealth`）
//! 在 Origin 命名空间**重新定义 + 重新证明**全部 TW 模型（不 import legacy，零偷渡，机器验证无
//! sorry/admit/axiom）。故 `TwState`/`tw_step`/`TStage`/`is_legal_from` OQ-9 gate 的契约锚点
//! **从 legacy `Tlayers/Accounting/TotalWealth.lean` 重锚到 `Origin.TotalWealth`**：
//!
//! - `TwState` ↔ `Origin.TotalWealth.TWState`（free/holding/withdrawn/notionalIn/stage/openLegacyLegs/
//!   cumNetCash 字段逐一对齐，TotalWealth.lean:94-102）。
//! - `TStage` ↔ `Origin.TotalWealth.TStage`（costReduction/capitalRecovered/earningShares，:63-67）；
//!   `rank` ↔ `Origin.TotalWealth.TStage.rank`（:73-76）。
//! - `TwEvent` ↔ `Origin.TotalWealth.TWEvent`（shortDiff/openShareLeg/closeShareLeg/recoverCapital/
//!   enterEarning/clearCampaign 六构造子，:120-127）。
//! - `tw_step` ↔ `Origin.TotalWealth.twStep`（:159-171，逐分支语义对齐，保 `twStep_preserves_tw`:178）。
//! - `tw()` ↔ `Origin.TotalWealth.TWState.tw`（free+holding+withdrawn，:105）。
//! - `advance_to` ↔ `Origin.TotalWealth.advanceTo`（:133-134，rank 单向 `advanceTo_rank_ge`:137）。
//! - `is_legal_from` ↔ `Origin.TotalWealth.LegalTransition`（:261-266，OQ-9 gate）+ `LegalEnterEarning`
//!   （:249）；OQ-9 不变量 ↔ `OQ9Inv`（:272-273）+ `oq9inv_preserved`（:296）+ trace 级 `oq9inv_trace`（:343）。
//!
//! ★不同构裁定保留（#90）：R=Π-A-W 与取本金三阶段 TW **不同构**（`Origin.TotalWealth` §3 三反例
//! `not_isomorphic_stage_collapses`/`_conservation_switches`/`_exogenous_price` 在 Origin native
//! 定义上重新成立）——故 TW **不合并进** `Origin.LedgerState`，两独立 native 结构并置（见双账本并置节）。
//! legacy `Tlayers/Accounting/TotalWealth.lean` 此后降为待清理 reference（不再被 canonical 路径引用）。
//!
//! ## 双账本并置（task #90 不同构裁定 → task #93 闭环扩维 → #127 两端均锚 Origin）
//!
//! `R=Π-A-W`（收益表视角，`Origin.FullDefinitionStrategy.LedgerState`）与
//! `TW=free+holding+withdrawn`（现金流+持仓视角，`Origin.TotalWealth.TWState`）**不同构**
//! （#90 已证，`Origin.TotalWealth` §3 三反例在 native 定义上重新成立：外生价格维度 / stage
//! 不可逆 vs 操作可逆 / 守恒律切换三条阻断）。完整持仓系统两者都需要——故 closed_loop 把两者
//! 并置在 AssemblyState，T 同步线程化两者（见 transition.rs）。#127 重锚后**两端均锚 Origin
//! canonical**：R=Π-A-W 锚 `Origin.FullDefinitionStrategy.LedgerState`，TW/OQ-9 锚
//! `Origin.TotalWealth`（不再有 legacy 锚点缺位）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：Rust 类型/算子与 Origin/legacy 定义结构对齐 = 验证管线
//!   正确性，零信息增量）。`cargo test` 通过 = 双账本不变量在每个算子下结构成立，**不**是
//!   任何缠论盈利 / 实盘有效声明（那是 L2/L3，真实数据回测才可否证）。重锚到 Origin **不**
//!   提升等级——锚点换 canonical 来源仍是 L0/L1（验管线非验缠论假设）。
//! - 不变量 `R=Π-A-W`（ledger，`Origin.FullDefinitionStrategy.LedgerState`）+
//!   `TW=free+holding+withdrawn`（tw，`Origin.TotalWealth.TWState`）+ stage 单向（OQ-9，
//!   `Origin.TotalWealth`）都是**结构恒等**（同价 c 固定下成立），不证账本数值反映实盘真实盈亏。
//!
//! ## 契约锚点源（只读，不改 .lean；两端均 Origin canonical）
//!
//! - LedgerComp / LedgerEvent / ledger_step → **`Origin.FullDefinitionStrategy.LedgerState`**
//!   （LedgerState/mkLedger/ledgerStep/ledger_invariant_preservation，:184-203）。
//! - TwState / TStage / TwEvent / tw_step / OQ-9 gate → **`Origin.TotalWealth`**
//!   （TWState/TStage/TWEvent/twStep/LegalTransition/OQ9Inv，#127 native port，见上「TW 三阶段 /
//!   OQ-9 gate 契约 → Origin.TotalWealth」重锚声明）。

/// 取本金三阶段 `TStage`（契约锚 `Origin.TotalWealth.TStage`，缠师第31课）。
///
/// 单向不可逆迁移（OQ-9）：CostReduction(0) → CapitalRecovered(1) → EarningShares(2)。
/// `rank` 把三阶段映到 {0,1,2}，是单向偏序的载体（rank 只增不减，见 [`tw_step`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TStage {
    /// ① 降成本：短差，Σ|units| 守恒（"买入多少卖出多少不增仓"）。
    CostReduction,
    /// ② 退本金：本金部分/全额移出在险池（free→withdrawn）。
    CapitalRecovered,
    /// ③ 增股数：本金已全退，纯利润买更多 units，Σ|units| 单调增。
    EarningShares,
}

impl TStage {
    /// 阶段序（契约锚 `Origin.TotalWealth.TStage.rank`）：单向不可逆迁移的偏序载体。
    pub fn rank(self) -> u32 {
        match self {
            TStage::CostReduction => 0,
            TStage::CapitalRecovered => 1,
            TStage::EarningShares => 2,
        }
    }
}

/// 单向阶段推进（契约锚 `Origin.TotalWealth.advanceTo`）：rank 只前进，不回退。
///
/// `current.rank <= target.rank` ⟹ 推到 target；否则保持 current（不下降）。
/// 这正是 OQ-9 单向不可逆的算子形式——任何推进都不能降低 rank。
fn advance_to(current: TStage, target: TStage) -> TStage {
    if current.rank() <= target.rank() {
        target
    } else {
        current
    }
}

/// TW 账本态 `TwState`（契约锚 `Origin.TotalWealth.TWState`，模型B 取本金三阶段）。
///
/// 字段（对齐 `t_engine.rs:208` TPositionEngine 会计分量）：
/// - `free`：自由现金/在险池（NAV 的现金部分）。
/// - `holding`：当前持仓市值 Σ units·c（含**外生价格** c；同价操作下守恒，见不同构理由1）。
/// - `withdrawn`：已退本金（退本金阶段移出在险池的现金，clear 时归还 free）。
/// - `notional_in`：本 campaign 投入本金 K（退本金目标；enter 记 m·c，clear 重置 0）。
/// - `stage`：当前阶段（带历史/路径依赖的状态机分量，单向不可逆）。
/// - `open_legacy_legs`：未闭合 **legacy ShareConserving（降成本）腿**计数（OQ-9 矛盾载体，
///   与 stage 正交——stage 是相位，open_legacy_legs 是「还有几条降成本期遗留短差腿未平」）。
/// - `cum_net_cash`：净现金口径累加（cost_basis 符号的结构载体，design §2.2）。与 TW 三量
///   正交——是口径量（累计净现金贡献），不进 `TW=free+holding+withdrawn`。
///
/// ★诚实标注（formalization-validity-domain）：`holding` 用整数承载市值摘要（结构层；真实
/// holding=Σunits·c 含外生 c，c 的跨 bar 变动 = 盈亏，是 L2/L3 数据，本 L0 结构层把同价
/// 操作下的市值作整数量承载，不臆造价格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwState {
    pub free: i64,
    pub holding: i64,
    pub withdrawn: i64,
    pub notional_in: i64,
    pub stage: TStage,
    pub open_legacy_legs: u32,
    pub cum_net_cash: i64,
}

impl TwState {
    /// 初始 TW 态（campaign 开局：CostReduction 阶段，无腿/无净现金）。
    pub fn initial() -> TwState {
        TwState {
            free: 0,
            holding: 0,
            withdrawn: 0,
            notional_in: 0,
            stage: TStage::CostReduction,
            open_legacy_legs: 0,
            cum_net_cash: 0,
        }
    }

    /// 总财富 `TW = free + holding + withdrawn`（契约锚 `Origin.TotalWealth.TWState.tw`，守恒量）。
    pub fn tw(&self) -> i64 {
        self.free + self.holding + self.withdrawn
    }

    /// 最坏损失 `L^wc`（契约锚 PDF §10 `L^wc_{t+1}`，L0 缠论「降成本」语义）：**仍在险的本金** =
    /// `notional_in − withdrawn`（原始投入本金减已退回本金），下界 0。
    ///
    /// 缠师第31课「成本为0后」：退本金推进 ⟹ 在险本金递减 ⟹ L^wc→0（本金全退=无本金在险=真正拉抬
    /// 不需要花钱）。这比「持仓市值 holding」更贴合降成本语义——holding 随建仓单调增（不反映本金退出），
    /// 而在险本金 `notional_in−withdrawn` 随退本金单调减，正是 barrier `η≥η⋆` 在增股数阶段可满足的根因
    /// （withdrawn≥notional_in ⟹ L^wc=0 ⟹ η⋆=κ·Q ⟹ κ=0 时 η⋆=0 ⟹ η≥0 恒过 barrier）。
    ///
    /// ★诚实（formalization-validity-domain）：真实 L^wc 含波动率/回撤分布（L2/L3）——本 L0 结构层取
    /// 「在险本金」作最坏损失代理（同价下本金退出即在险额减少），不臆造价格分布。
    pub fn l_wc(&self) -> i64 {
        (self.notional_in - self.withdrawn).max(0)
    }

    /// 本 campaign 名义敞口 `Q`（契约锚 PDF §10 `Q_t`）：退本金目标基线 = 已记录本金 `notional_in`。
    pub fn notional(&self) -> i64 {
        self.notional_in
    }
}

/// 风险政策 `RiskPolicy`（契约锚 PDF §10 `Θ_risk` / §11 Lean `structure RiskPolicy`）。
///
/// **不可识别性定理2（k的条件.pdf 编排者裁决）**：`κ` **不是价格可推的值**，是**声明式风险政策
/// 参数**——不由缠论/价格识别，由 operator 声明（可 walk-forward 优化）。故本结构承载 `κ` 作配置
/// knob，**不**从数据估计。
///
/// - `kappa`（κ）：状态依赖 barrier `η⋆ = L^wc + κ·Q` 的额外安全缓冲系数。**κ≥0**（不变量
///   [`RiskPolicy::kappa_nonneg`]）。默认 **κ=0**（最小基线：`η⋆=L^wc`，仅覆盖最坏损失无额外缓冲，
///   PDF §10 canonical 最小规范）——用于 acc-GAP3 可达性测试。
///
/// ★与 `RiskConfig.kappa`（config.rs：**成本倍数** κ=2.0，sizing 用）**是不同的 κ**——本 `RiskPolicy.
/// kappa` 是**barrier 缓冲系数**（风险政策），二者同名不同义（PDF §10 barrier κ vs sizing κ）。
/// 定点整数承载（bit-exact，barrier 比较在整数域；κ 用 i64 缩放系数，避免浮点非确定性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskPolicy {
    /// barrier 缓冲系数 κ（**≥0，构造时强制**；默认 0=最小基线）。私有字段——**唯一构造闸是**
    /// [`RiskPolicy::baseline`]（恒 κ=0）/ [`RiskPolicy::try_new`]（拒负 κ），二者都保证 κ≥0，使
    /// κ≥0 成为 **constructor-only 类型不变量**（对齐 Lean `RiskPolicy.kappa_nonneg` 证明字段）。
    ///
    /// ★codex R3 §9.4（类型边界闭合）：字段**模块私有** + 仅 `baseline`/`try_new` 构造 ⟹ **负 κ 的
    /// `RiskPolicy` 值在任何路径都不存在**（含本模块非构造路径与全 crate）。**不保留任何反向见证**
    /// （旧版模块内测试用 struct literal 构造 `RiskPolicy { kappa: -1 }` 已删——那使类型边界在测试
    /// 可见性下未闭，codex R3 判为漏点）。负 κ 拒绝的证据由**外部 API `try_new(-1)=None`** 承载
    /// （正向：外部构造闸拒负），非「构造非法值再验谓词」的反向见证。这是 constructor-only pattern
    /// （PDF §9.4 二选一的可行分支——`is_legal_from` 式关系型非法用 Result，此处 κ≥0 是绝对约束用
    /// constructor-only 类型不变量）。
    kappa: i64,
}

impl RiskPolicy {
    /// 最小基线政策（κ=0，PDF §10 canonical 默认）：`η⋆=L^wc`，仅覆盖最坏损失无额外缓冲。
    pub fn baseline() -> RiskPolicy {
        RiskPolicy { kappa: 0 }
    }

    /// **构造校验入口 `try_new`（codex #5：κ≥0 Rust 不变量）**：κ<0 ⟹ `None`（负缓冲=不覆盖 L^wc
    /// =非法，不可构造）。κ≥0 ⟹ `Some(RiskPolicy)`。这把 Lean 侧 `kappa_nonneg` 证明字段的语义
    /// 在 Rust 侧兑现为**构造时拒绝**——负 κ 的 RiskPolicy 值根本不存在（不是运行时检查后放行）。
    pub fn try_new(kappa: i64) -> Option<RiskPolicy> {
        if kappa >= 0 {
            Some(RiskPolicy { kappa })
        } else {
            None
        }
    }

    /// κ 只读访问（字段私有，barrier 缓冲系数 ≥0 由构造保证）。
    pub fn kappa(&self) -> i64 {
        self.kappa
    }

    /// κ≥0 不变量（契约锚 PDF §11 `kappa_nonneg`）：**构造时已强制**（[`try_new`](Self::try_new)
    /// 拒绝负 κ，[`baseline`](Self::baseline) 恒 κ=0）——本谓词恒真，是不变量的可观测断言。
    pub fn kappa_nonneg(&self) -> bool {
        self.kappa >= 0
    }

    /// 状态依赖 barrier `η⋆(s) = L^wc(s) + κ·Q(s)`（契约锚 PDF §10 `η⋆(x_t)=L^wc_{t+1}+κ·Q_t`）。
    ///
    /// 进入 EarningShares 的**在险权益门槛**：权益 η 须 ≥ η⋆ 才允许相变（barrier 保证覆盖最坏损失 +
    /// κ 倍名义缓冲）。κ=0 ⟹ η⋆=L^wc（最小基线，仅覆盖最坏损失）。
    ///
    /// ★有界算术（codex #6：Lean 无界 Int vs Rust i64 wrap）：`κ·Q` 在 **i128 中间域**计算再夹回
    /// i64（`saturating`）——避免 release 下 i64 乘法 wrap（wrap 会让巨额 η⋆ 环绕成小值 ⟹ barrier
    /// 误过）。i128 对现实量级（κ、Q ≤ 数百万）足够承载精确乘积，饱和只在极端溢出时兜底（失败安全：
    /// 溢出 ⟹ η⋆=i64::MAX ⟹ barrier 不过，不误放行）。
    pub fn eta_star(&self, s: &TwState) -> i64 {
        let kappa_q = (self.kappa as i128) * (s.notional() as i128);
        let eta = (s.l_wc() as i128) + kappa_q;
        eta.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    /// **EnterEarning 合法性谓词 `EnterReady`（严格 EnterReady，契约锚 PDF §10 步骤3 + Lean
    /// `LegalEnterEarning`）**：`S=II ∧ W≥I0 ∧ openLegacyLegs=0 ∧ RiskNormal ∧ η≥η⋆`。
    ///
    /// 比单纯 `W≥I0` **强得多**——五合取（OQ-9 腿空 + 阶段=退本金 + 本金已全退 W≥I0 + 风控正常 +
    /// 在险权益过 barrier）。`i0`=本金基线 I₀，`risk_normal`=风控模式正常（由调用方从 RiskMode 判）。
    ///
    /// - `S=II`：`stage==CapitalRecovered`（已进退本金阶段，rank≥1；退本金完成才谈进增股数）。
    /// - `W≥I0`：`withdrawn ≥ i0`（本金已全额退出在险池——增股数阶段前提「本金全退」，缠师第31课）。
    /// - `openLegacyLegs=0`：无未闭合 legacy 降成本腿（OQ-9 入口证书，`LegalEnterEarning`）。
    /// - `RiskNormal`：风控正常（非破产/清算/去杠杆）。
    /// - `η≥η⋆`：在险权益 `tw()` 过 barrier `eta_star`（覆盖最坏损失 + κ 缓冲）。
    pub fn enter_ready(&self, s: &TwState, i0: i64, risk_normal: bool) -> bool {
        s.stage == TStage::CapitalRecovered
            && s.withdrawn >= i0
            && s.open_legacy_legs == 0
            && risk_normal
            && s.tw() >= self.eta_star(s)
    }

    /// **BuyCore 合法性谓词（契约锚 PDF §10 步骤4 定理1 充要）**：
    /// `a_n + L^wc_{n+1} + κ·ΔQ_n ≤ η_n + g_n − κ·Q_n`。
    ///
    /// 增股数阶段建核仓（BuyCore）的充要合法条件——建仓额 `a_n` 加上建仓后最坏损失加上 κ 倍名义
    /// 增量，须 ≤ 当前在险权益 `η_n` 加已实现收益 `g_n` 减 κ 倍当前名义。移项即 barrier 约束
    /// （PDF 定理1：BuyCore 保 κ-floor 不变量 `η ≥ η⋆`）。
    ///
    /// 参数（PDF §10 记号）：`a_n`=建仓额、`l_wc_next`=建仓后 L^wc_{n+1}、`delta_q`=ΔQ_n 名义增量、
    /// `eta_n`=当前在险权益、`g_n`=已实现收益、`q_n`=当前名义 Q_n。
    ///
    /// ★有界算术（codex #6）：LHS/RHS 在 **i128 中间域**求值再比较——避免 i64 加乘 wrap（Lean 侧
    /// `buyCore_preserves_kappa_floor` 是无界 Int 代数移项，Rust 用 i128 承载现实量级的精确值，与
    /// Lean 语义对齐；比较本身无溢出风险，i128 加乘对 ≤ 数百万量级的输入恒精确）。
    pub fn buy_core_legal(
        &self,
        a_n: i64,
        l_wc_next: i64,
        delta_q: i64,
        eta_n: i64,
        g_n: i64,
        q_n: i64,
    ) -> bool {
        let k = self.kappa as i128;
        let lhs = (a_n as i128) + (l_wc_next as i128) + k * (delta_q as i128);
        let rhs = (eta_n as i128) + (g_n as i128) - k * (q_n as i128);
        lhs <= rhs
    }
}

/// TW 账本事件 `TWEvent`（契约锚 `Origin.TotalWealth.TWEvent`，对齐 t_engine 三阶段算子 + OQ-9）。
///
/// - `ShortDiff(d_cash)`：降成本短差（free⇄holding 同价转换，TW 不变；CostReduction 阶段）。
///   `d_cash>0`=卖出（holding→free），`d_cash<0`=买回（free→holding）。
/// - `OpenShareLeg`：开一条 legacy ShareConserving 降成本短差腿（open_legacy_legs += 1）。
/// - `CloseShareLeg(profit)`：闭合一条 legacy 腿（open_legacy_legs -= 1，profit 落 cum_net_cash）。
///   **OQ-9 矛盾的精确载体**：profit<0（亏损闭合）净抽出本金——若此时 stage 已 EarningShares，
///   亏损闭合让净现金口径成本回正（端A），但 stage 单向不回退（端B）。
/// - `RecoverCapital(w)`：退本金（free→withdrawn，移出在险池 w）。触发/推进 CapitalRecovered。
/// - `EnterEarning`：本金全退后切 EarningShares（单向不可逆相变，无资金变动）。
/// - `ClearCampaign`：campaign 结束（withdrawn→free 归还，stage 重置 CostReduction，legacy
///   腿/cum_net_cash 清零）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwEvent {
    ShortDiff(i64),
    OpenShareLeg,
    CloseShareLeg(i64),
    RecoverCapital(i64),
    EnterEarning,
    ClearCampaign,
}

impl TwEvent {
    /// OQ-9 合法转移谓词 `LegalTransition`（契约锚 `Origin.TotalWealth.LegalTransition`，
    /// review 019f0318 修正后的 legal 层）。
    ///
    /// 哪些 `(s, e)` 合法（raw 层 [`tw_step`] 对所有 (s,e) 都有定义，包括非法的；本谓词只标注
    /// 合法子集——raw 层保留非法路径作 violation witness，legal 层排除它）：
    /// - `EnterEarning`：须 `open_legacy_legs == 0`（清空 legacy 腿后才能进，OQ-9 入口证书）。
    /// - `OpenShareLeg`：须 `stage.rank < EarningShares.rank`（修正1：earning 阶段不再开 legacy 腿）。
    /// - `CloseShareLeg`：须 `open_legacy_legs >= 1`（有腿才能闭——否则凭空闭不存在的腿 = 幽灵）。
    /// - `ClearCampaign`：须 `open_legacy_legs == 0`（修正4(b)：结束前先清 legacy 腿，不绕 gate）。
    /// - 其余（ShortDiff/RecoverCapital）：恒合法（不动 legacy 腿/stage 的资金转移）。
    pub fn is_legal_from(&self, s: &TwState) -> bool {
        match self {
            TwEvent::EnterEarning => s.open_legacy_legs == 0,
            TwEvent::OpenShareLeg => s.stage.rank() < TStage::EarningShares.rank(),
            TwEvent::CloseShareLeg(_) => s.open_legacy_legs >= 1,
            TwEvent::ClearCampaign => s.open_legacy_legs == 0,
            TwEvent::ShortDiff(_) | TwEvent::RecoverCapital(_) => true,
        }
    }
}

/// TW 更新 `tw_step`（契约锚 `Origin.TotalWealth.twStep`，全函数，**保 TW 守恒**）。
///
/// 同价 c 固定下 free/holding/withdrawn 间转移，三量之和 TW 不变（同价中性，design §6.3 L0）。
/// raw 层全函数——对所有 (s, e) 都有定义（包括非法的 EnterEarning，保留 violation witness）；
/// 合法性由 [`TwEvent::is_legal_from`] 标注。
///
/// 不可变模式（coding-style）：返回新 [`TwState`]，不就地修改。
pub fn tw_step(s: &TwState, e: TwEvent) -> TwState {
    match e {
        // holding→free 转 d_cash（free+=d, holding-=d）⟹ TW 不变。
        TwEvent::ShortDiff(d_cash) => TwState {
            free: s.free + d_cash,
            holding: s.holding - d_cash,
            ..*s
        },
        // 开 legacy 降成本短差腿（无 TW 三量变动）⟹ TW 不变。
        TwEvent::OpenShareLeg => TwState {
            open_legacy_legs: s.open_legacy_legs + 1,
            ..*s
        },
        // 闭合 legacy 腿：open_legacy_legs 饱和减 + cum_net_cash += profit（净现金口径，端A
        // 载体）⟹ TW 三量不变（profit 进 cum_net_cash 口径量，不进 TW 财富量）⟹ TW 不变。
        TwEvent::CloseShareLeg(profit) => TwState {
            open_legacy_legs: s.open_legacy_legs.saturating_sub(1),
            cum_net_cash: s.cum_net_cash + profit,
            ..*s
        },
        // free→withdrawn 转 w（withdrawn+=w, free-=w）⟹ TW 不变；stage 推进 CapitalRecovered（单向）。
        TwEvent::RecoverCapital(w) => TwState {
            free: s.free - w,
            withdrawn: s.withdrawn + w,
            stage: advance_to(s.stage, TStage::CapitalRecovered),
            ..*s
        },
        // 纯 stage 切换到 EarningShares（单向），无资金变动 ⟹ TW 不变。raw 层无入口守卫
        // （即使 open_legacy_legs>0 也允许切，保留 violation witness 路径；合法性见 is_legal_from）。
        TwEvent::EnterEarning => TwState {
            stage: advance_to(s.stage, TStage::EarningShares),
            ..*s
        },
        // campaign 结束：withdrawn→free 归还（TW 不变）；stage 重置 + legacy 腿/cum_net_cash 清零。
        // 注意：campaign 边界，新 campaign 起点，**非回退**（stage 单向性是 campaign 内性质）。
        TwEvent::ClearCampaign => TwState {
            free: s.free + s.withdrawn,
            holding: s.holding,
            withdrawn: 0,
            notional_in: 0,
            stage: TStage::CostReduction,
            open_legacy_legs: 0,
            cum_net_cash: 0,
        },
    }
}

/// 账本分量 `LedgerComp`（契约锚 **`Origin.FullDefinitionStrategy.LedgerState`**，R=Π-A-W 恒等载体）。
///
/// Origin canonical 的 `LedgerState`（FullDefinitionStrategy.lean:184-190）把恒等作为**结构不变量
/// 字段** `inv : R = Pi - A - W`——任何 Origin LedgerState 值在类型层即携带恒等证据。本 Rust 结构
/// 镜像该接口语义（字段对应 `Origin.LedgerState.{Pi, A, W, R}` + 本金基线 `i0`）。
///
/// 字段（R=储备/Reserve，Π=累计利润/Profit，A=已分配/Allocated，W=已提取/Withdrawn）：
/// - `i0`：初始本金 I₀（账本基线，不进恒等——恒等只约束 Π/A/W/R 四量；Origin LedgerState 无此字段，
///   是 Rust 侧 NAV 基线扩展，与恒等正交）。
/// - `pi`：累计利润 Π（profit，含已实现盈亏）↔ `Origin.LedgerState.Pi`。
/// - `a`：已分配/资本化 A（allocated，earning 转股数等）↔ `Origin.LedgerState.A`。
/// - `w`：已提取 W（withdrawn，出金）↔ `Origin.LedgerState.W`。
/// - `r`：储备 R（reserve，未分配未提取的留存）↔ `Origin.LedgerState.R`。
///
/// **账本恒等不变量 `R = Π - A - W`**（结构层强制）↔ `Origin.LedgerState.inv`：任何 [`LedgerComp`]
/// 值都满足它（构造时显式重算 R，对齐 Origin `mkLedger` 的 `R := Pi - A - W`；ledger_step 的每个
/// 算子保持它，对齐 Origin `ledger_invariant_preservation`，见 [`ledger_step`] + 测试 `ledger_step_preserves_inv`）。
///
/// ★诚实标注：Origin `LedgerState` 用依赖类型字段 `inv` 在类型层钉死恒等；Rust 无依赖类型，故用
/// 构造时重算 R + 运行时谓词 [`inv_holds`] 共同承载（等价的结构强制，不是弱化）。
/// 它与 [`TwState`] **不同构**（#90 已证），双账本并置——R=Π-A-W 锚 Origin.LedgerState，TW 锚 Origin.TotalWealth。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerComp {
    pub i0: i64,
    pub pi: i64,
    pub a: i64,
    pub w: i64,
    pub r: i64,
}

impl LedgerComp {
    /// 初始账本 `ledger0(I0)`（契约锚 `Origin.FullDefinitionStrategy.mkLedger 0 0 0`）：开局账本
    /// ——Π=A=W=R=0，恒等显然成立（Origin `mkLedger` 的 `R := Pi - A - W = 0`）。
    pub fn initial(i0: i64) -> LedgerComp {
        LedgerComp { i0, pi: 0, a: 0, w: 0, r: 0 }
    }

    /// 账本恒等检查 `R == Π - A - W`（契约锚 `Origin.FullDefinitionStrategy.LedgerState.inv`）。
    ///
    /// Origin 把恒等作为**结构不变量字段** `inv`（构造时提供证据）；Rust 无依赖类型，故用运行时
    /// 谓词 + 构造时重算 R 共同保证（[`ledger_step`] 每分支显式重算 R 使恒等成立，对齐 Origin
    /// `mkLedger`/`ledgerStep`）。这个谓词是不变量的可观测断言（测试逐算子验证它恒为真）。
    pub fn inv_holds(&self) -> bool {
        self.r == self.pi - self.a - self.w
    }
}

/// 账本事件 `LedgerEvent`（契约锚 `Origin.FullDefinitionStrategy.ledgerStep` 的 dΠ/dA/dW 增量，穷尽）。
///
/// Origin `ledgerStep L dPi dA dW` 以三个独立增量驱动账本；本枚举把三增量拆为正交事件
/// （Realize=dΠ, Allocate=dA, Withdraw=dW, Noop=全零增量），逐事件保 Origin `ledger_invariant_preservation`。
///
/// - `Realize(d_pi)`：实现利润 dΠ（Π += dΠ；R 随之 += dΠ 保恒等）。
/// - `Allocate(d_a)`：分配/资本化 dA（A += dA；R -= dA 保恒等）。
/// - `Withdraw(d_w)`：提取 dW（W += dW；R -= dW 保恒等）。
/// - `Noop`：无账本变化（如纯 bar 推进不触发会计事件）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEvent {
    Realize(i64),
    Allocate(i64),
    Withdraw(i64),
    Noop,
}

/// 账本更新 `ledger_step`（契约锚 `Origin.FullDefinitionStrategy.ledgerStep`，全函数，**保 R=Π-A-W**）。
///
/// 对每个 [`LedgerEvent`] 更新四量，**新 R 由恒等重算**（R := Π - A - W，对齐 Origin `mkLedger`），
/// 故不变量按构造成立（对齐 Origin `ledger_invariant_preservation`）：
/// - `Realize(dΠ)`：Π += dΠ，A/W 不变 ⟹ R += dΠ。
/// - `Allocate(dA)`：A += dA，Π/W 不变 ⟹ R -= dA。
/// - `Withdraw(dW)`：W += dW，Π/A 不变 ⟹ R -= dW。
/// - `Noop`：四量不变。
///
/// 不可变模式（coding-style）：返回新 [`LedgerComp`]，不就地修改。
pub fn ledger_step(l: &LedgerComp, e: LedgerEvent) -> LedgerComp {
    match e {
        LedgerEvent::Realize(d_pi) => LedgerComp {
            pi: l.pi + d_pi,
            r: l.r + d_pi,
            ..*l
        },
        LedgerEvent::Allocate(d_a) => LedgerComp {
            a: l.a + d_a,
            r: l.r - d_a,
            ..*l
        },
        LedgerEvent::Withdraw(d_w) => LedgerComp {
            w: l.w + d_w,
            r: l.r - d_w,
            ..*l
        },
        LedgerEvent::Noop => *l,
    }
}

#[cfg(test)]
mod tests {
    //! ★★模块级诚实标注（codex §9.2，照实 161/no-workaround）：本模块的机制单元测试
    //! （`eta_star_barrier` / `enter_ready_strict_conjunction` / `buy_core_legality` / `stage_rank_monotone`
    //! 等，在**手工构造态**上验 eta_star/enter_ready/buy_core/tw_step 谓词与守恒）均为
    //! **synthetic-state legality tests, NOT reachability tests** —— 它们验「给定前提则机制正确」，
    //! **不**证该前提态从 `funded_campaign` 起点可达。可达性由 `runner.rs::
    //! earning_shares_structurally_unreachable_from_campaign_tw_conserved` 的结构不可达定理界定
    //! （L0 同价 TW 守恒下 EarningShares 不可达）。**机制正确 ≠ 前提可达**（有效域区分，
    //! formalization-validity-domain）。不为过审硬凑「已获利」witness（codex 复审#2 判致命，已删）。
    use super::*;

    // ──────────────────────────────────────────────────────────────────────
    //  LedgerComp R=Π-A-W 不变量（契约锚 Origin.FullDefinitionStrategy.ledger_invariant_preservation）
    // ──────────────────────────────────────────────────────────────────────

    /// ledger0 恒等成立（契约锚 `Origin.mkLedger 0 0 0` 的 `inv := rfl`）。
    #[test]
    fn ledger_initial_inv_holds() {
        let l = LedgerComp::initial(1_000_000);
        assert!(l.inv_holds());
        assert_eq!(l.r, 0); // Π=A=W=R=0
    }

    /// ★ledger_step 保 R=Π-A-W（契约锚 `Origin.FullDefinitionStrategy.ledger_invariant_preservation`）：
    /// 逐事件验证恒等保持。
    #[test]
    fn ledger_step_preserves_inv() {
        let l0 = LedgerComp::initial(1_000_000);
        // 一串混合事件，每步后恒等必须成立。
        let events = [
            LedgerEvent::Realize(500),
            LedgerEvent::Allocate(200),
            LedgerEvent::Withdraw(100),
            LedgerEvent::Realize(-50), // 亏损（可负）
            LedgerEvent::Allocate(-30),
            LedgerEvent::Noop,
        ];
        let mut l = l0;
        for e in events {
            l = ledger_step(&l, e);
            assert!(l.inv_holds(), "ledger_step({:?}) 破坏 R=Π-A-W: {:?}", e, l);
        }
        // 终态数值核对：Π=500-50=450, A=200-30=170, W=100 ⟹ R=450-170-100=180。
        assert_eq!(l.pi, 450);
        assert_eq!(l.a, 170);
        assert_eq!(l.w, 100);
        assert_eq!(l.r, 180);
    }

    /// Noop 不改账本（恒等保持，四量不变）。
    #[test]
    fn ledger_noop_identity() {
        let l = LedgerComp { i0: 1000, pi: 10, a: 3, w: 2, r: 5 };
        assert_eq!(ledger_step(&l, LedgerEvent::Noop), l);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  TwState TW 守恒 + stage 单向（契约锚 `Origin.TotalWealth.twStep_preserves_tw` /
    //  `stage_rank_monotone`——#127 native port，两端均锚 Origin canonical）
    // ──────────────────────────────────────────────────────────────────────

    /// ★tw_step 保 TW 守恒（契约锚 `Origin.TotalWealth.twStep_preserves_tw`）：逐事件 TW=free+holding+withdrawn 不变。
    #[test]
    fn tw_step_preserves_tw() {
        let s0 = TwState {
            free: 1000,
            holding: 500,
            withdrawn: 0,
            notional_in: 500,
            stage: TStage::CostReduction,
            open_legacy_legs: 0,
            cum_net_cash: 0,
        };
        let tw0 = s0.tw();
        let events = [
            TwEvent::ShortDiff(100),   // holding→free
            TwEvent::ShortDiff(-50),   // free→holding
            TwEvent::OpenShareLeg,     // 无 TW 变动
            TwEvent::CloseShareLeg(-20), // profit 进 cum_net_cash，不进 TW
            TwEvent::RecoverCapital(200), // free→withdrawn
            TwEvent::EnterEarning,     // 无资金变动
        ];
        let mut s = s0;
        for e in events {
            s = tw_step(&s, e);
            assert_eq!(s.tw(), tw0, "tw_step({:?}) 破坏 TW 守恒: tw={}", e, s.tw());
        }
        // clearCampaign 也守恒（withdrawn→free 归还）。
        let s_clear = tw_step(&s, TwEvent::ClearCampaign);
        assert_eq!(s_clear.tw(), tw0, "clearCampaign 破坏 TW 守恒");
        assert_eq!(s_clear.withdrawn, 0);
    }

    /// ★stage 单向不可逆（契约锚 `Origin.TotalWealth.stage_rank_monotone`）：非 clearCampaign 算子下 rank 只增不减。
    #[test]
    fn stage_rank_monotone() {
        let stages = [TStage::CostReduction, TStage::CapitalRecovered, TStage::EarningShares];
        let events = [
            TwEvent::ShortDiff(10),
            TwEvent::OpenShareLeg,
            TwEvent::CloseShareLeg(5),
            TwEvent::RecoverCapital(50),
            TwEvent::EnterEarning,
        ]; // 全部非 ClearCampaign
        for stage in stages {
            let s = TwState { stage, ..TwState::initial() };
            for e in events {
                let s2 = tw_step(&s, e);
                assert!(
                    s.stage.rank() <= s2.stage.rank(),
                    "stage 回退: {:?} --{:?}--> {:?}",
                    s.stage, e, s2.stage
                );
            }
        }
    }

    /// ★EarningShares 不可回退（契约锚 `Origin.TotalWealth.earning_no_regress`）：earning 态非 clearCampaign 算子保持 earning。
    #[test]
    fn earning_no_regress() {
        let s = TwState { stage: TStage::EarningShares, ..TwState::initial() };
        let events = [
            TwEvent::ShortDiff(10),
            TwEvent::OpenShareLeg,
            TwEvent::CloseShareLeg(5),
            TwEvent::RecoverCapital(50), // 试图推回 capitalRecovered（rank 1<2）⟹ advance_to 保持 earning
            TwEvent::EnterEarning,
        ];
        for e in events {
            assert_eq!(
                tw_step(&s, e).stage,
                TStage::EarningShares,
                "earning 在 {:?} 后回退",
                e
            );
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  OQ-9 gate（契约锚 `Origin.TotalWealth.LegalTransition` / `LegalEnterEarning`——#127 native port）
    // ──────────────────────────────────────────────────────────────────────

    /// OQ-9 入口证书：EnterEarning 须 open_legacy_legs==0（契约锚 `Origin.TotalWealth.LegalEnterEarning`）。
    #[test]
    fn oq9_enter_earning_gate() {
        let with_leg = TwState { open_legacy_legs: 1, ..TwState::initial() };
        let no_leg = TwState { open_legacy_legs: 0, ..TwState::initial() };
        assert!(!TwEvent::EnterEarning.is_legal_from(&with_leg), "带 legacy 腿进 earning 非法");
        assert!(TwEvent::EnterEarning.is_legal_from(&no_leg), "无 legacy 腿进 earning 合法");
    }

    /// ★OQ-9 核心：earning 阶段开 legacy 腿非法（契约锚 `Origin.TotalWealth.LegalTransition` openShareLeg 修正1）。
    #[test]
    fn oq9_no_open_leg_in_earning() {
        let earning = TwState { stage: TStage::EarningShares, ..TwState::initial() };
        let cost_red = TwState { stage: TStage::CostReduction, ..TwState::initial() };
        assert!(
            !TwEvent::OpenShareLeg.is_legal_from(&earning),
            "earning 阶段开 legacy 腿非法（rank 2 < 2 假）"
        );
        assert!(
            TwEvent::OpenShareLeg.is_legal_from(&cost_red),
            "costReduction 阶段开 legacy 腿合法"
        );
    }

    /// CloseShareLeg 须有腿（open_legacy_legs>=1），否则幽灵腿非法（契约锚 `Origin.TotalWealth` 修正4(c) `zero_close_is_ghost`）。
    #[test]
    fn oq9_no_ghost_close() {
        let no_leg = TwState { open_legacy_legs: 0, ..TwState::initial() };
        let one_leg = TwState { open_legacy_legs: 1, ..TwState::initial() };
        assert!(!TwEvent::CloseShareLeg(0).is_legal_from(&no_leg), "无腿可闭 = 幽灵非法");
        assert!(TwEvent::CloseShareLeg(0).is_legal_from(&one_leg), "有腿可闭合法");
    }

    /// ★OQ-9 端B（契约锚 `Origin.TotalWealth.legal_earning_no_legacy_leg_closure`）：合法进 earning 后闭 legacy 腿非法。
    /// 合法 EnterEarning ⟹ open_legacy_legs=0 ⟹ CloseShareLeg 的 ≥1 合法性为假 ⟹ 闭腿非法。
    #[test]
    fn oq9_earning_no_legacy_leg_closure() {
        // 合法进 earning 的前态：open_legacy_legs=0。
        let pre = TwState { open_legacy_legs: 0, ..TwState::initial() };
        assert!(TwEvent::EnterEarning.is_legal_from(&pre), "前提：合法进 earning");
        let post = tw_step(&pre, TwEvent::EnterEarning);
        assert_eq!(post.open_legacy_legs, 0, "进 earning 后 legacy 腿仍 0");
        // 端B：闭 legacy 腿非法（无腿可闭）。
        assert!(
            !TwEvent::CloseShareLeg(-100).is_legal_from(&post),
            "earning 后闭 legacy 腿非法（端B 相位锁死）"
        );
    }

    /// ClearCampaign 须先清 legacy 腿（契约锚 `Origin.TotalWealth.LegalTransition` 修正4(b)）。
    #[test]
    fn oq9_clear_campaign_gate() {
        let with_leg = TwState { open_legacy_legs: 1, ..TwState::initial() };
        let no_leg = TwState { open_legacy_legs: 0, ..TwState::initial() };
        assert!(!TwEvent::ClearCampaign.is_legal_from(&with_leg), "带腿清 campaign 非法（不绕 gate）");
        assert!(TwEvent::ClearCampaign.is_legal_from(&no_leg), "无腿清 campaign 合法");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  RiskPolicy barrier κ + EnterReady + BuyCore（契约锚 PDF §10-11，GAP3 κ-gated）
    // ──────────────────────────────────────────────────────────────────────

    /// κ≥0 不变量（PDF §11 `kappa_nonneg`）：baseline κ=0 满足；**外部构造闸 try_new 拒负 κ**（codex#5）。
    #[test]
    fn risk_policy_kappa_nonneg() {
        assert!(RiskPolicy::baseline().kappa_nonneg(), "baseline κ=0 满足 κ≥0");
        assert_eq!(RiskPolicy::baseline().kappa(), 0, "baseline κ=0（PDF §10 最小规范）");
        // ★外部构造闸 try_new：κ≥0 ⟹ Some（且 κ() 读回）；κ<0 ⟹ None（外部不可构造负 κ）。
        assert_eq!(RiskPolicy::try_new(3).map(|p| p.kappa()), Some(3), "try_new(3)=Some(κ=3)");
        assert_eq!(RiskPolicy::try_new(0).map(|p| p.kappa()), Some(0), "try_new(0)=Some(κ=0)");
        assert!(RiskPolicy::try_new(-1).is_none(), "★try_new(-1)=None（外部 API 拒负 κ，codex R3 §9.4）");
        // ★codex R3 §9.4：不保留反向见证（旧版 `RiskPolicy { kappa: -1 }` struct literal 已删）——
        // 负 κ 的 RiskPolicy 值在任何路径都不存在（constructor-only 类型不变量），κ≥0 拒绝证据由
        // 上面 try_new(-1)=None 正向承载，非「构造非法值验谓词」的反向见证（那使类型边界在测试可见性
        // 下未闭）。kappa_nonneg() 恒真（构造保证），baseline().kappa_nonneg() 正向断言已覆盖谓词。
    }

    /// L^wc = max(0, notional_in − withdrawn)（在险本金，退本金推进 ⟹ L^wc→0）。
    #[test]
    fn l_wc_is_at_risk_principal() {
        let s0 = TwState { notional_in: 100, withdrawn: 0, ..TwState::initial() };
        assert_eq!(s0.l_wc(), 100, "退本金前 L^wc=notional_in（全本金在险）");
        let s_half = TwState { notional_in: 100, withdrawn: 40, ..TwState::initial() };
        assert_eq!(s_half.l_wc(), 60, "退 40 ⟹ L^wc=60（在险本金递减）");
        let s_full = TwState { notional_in: 100, withdrawn: 100, ..TwState::initial() };
        assert_eq!(s_full.l_wc(), 0, "本金全退 ⟹ L^wc=0（缠师「成本为0」）");
        let s_over = TwState { notional_in: 100, withdrawn: 130, ..TwState::initial() };
        assert_eq!(s_over.l_wc(), 0, "超退 ⟹ L^wc=0（下界 0，不为负）");
    }

    /// 状态依赖 barrier η⋆ = L^wc + κ·Q（PDF §10）：κ=0 ⟹ η⋆=L^wc；κ>0 ⟹ 额外缓冲。
    #[test]
    fn eta_star_barrier() {
        let s = TwState { notional_in: 100, withdrawn: 30, ..TwState::initial() }; // L^wc=70, Q=100
        assert_eq!(RiskPolicy { kappa: 0 }.eta_star(&s), 70, "κ=0 ⟹ η⋆=L^wc=70");
        assert_eq!(RiskPolicy { kappa: 2 }.eta_star(&s), 70 + 2 * 100, "κ=2 ⟹ η⋆=70+200=270");
    }

    /// ★i128 有界域回归（codex 复审#6）：κ·Q 在 i64 域会 wrap 的量级，i128 中间域算精确值再 clamp
    /// 到 i64::MAX（**失败安全**：超界 ⟹ η⋆=i64::MAX ⟹ barrier 不过，不误放行）。**非** Lean 无界
    /// Int 的 bit-exact——是 Rust 有界失败安全近似（clamp 后语义 = 溢出即最严 barrier）。
    #[test]
    fn eta_star_bounded_no_wrap() {
        // κ=i64::MAX, Q=large ⟹ i64 乘法会 wrap 成小/负值（误放行）；i128 算真值 > i64::MAX ⟹ clamp。
        let big = TwState { notional_in: i64::MAX / 2, withdrawn: 0, ..TwState::initial() };
        let pol = RiskPolicy::try_new(i64::MAX).expect("κ=i64::MAX≥0 合法");
        // i64 直算 kappa*Q 会 wrap；i128 真值 = MAX·(MAX/2) ≫ i64::MAX ⟹ clamp 到 i64::MAX。
        assert_eq!(pol.eta_star(&big), i64::MAX, "★超界 ⟹ η⋆=i64::MAX（失败安全，非 wrap 成小值）");
        // buy_core_legal 同样 i128：巨额 a_n 不 wrap ⟹ 正确判非法（LHS≫RHS）。
        assert!(
            !pol.buy_core_legal(i64::MAX, 0, i64::MAX, 0, 0, 0),
            "★i128：巨额建仓 LHS 不 wrap ⟹ 正确判非法（非 wrap 误判合法）"
        );
    }

    /// ★EnterReady 严格五合取（PDF §10 步骤3）：五条件全真才 ready（比单纯 W≥I0 强得多）。
    ///
    /// ★★诚实标注（codex §9.2，照实 161/no-workaround）：**this is a synthetic-state legality test,
    /// not a reachability test.** 本测试在**显式构造**的 `ready` 态（free=0, holding=100, withdrawn=100
    /// ⟹ tw=200）上验证 `enter_ready` 谓词的五合取逻辑（给定前提则谓词正确）——它**不**声称该态从
    /// `funded_campaign` 起点可达。事实上 tw=200=2·notional 在 TW 守恒下从 campaign 起点（tw=Q）
    /// **不可达**（见 `runner.rs::earning_shares_structurally_unreachable_from_campaign_tw_conserved`：
    /// 退本金前提 holding≥Q 与 cash-tight free>0 在 TW=Q 下互斥）。有效域区分：**机制正确 ≠ 前提可达**
    /// ——本测试锚前者（谓词逻辑），可达性由 runner 不可达定理锚后者（L0 同价下 EarningShares 不可达，
    /// 真达需 L2 价格升值让已实现利润进 TW）。不为通过而硬凑「已获利」witness（codex 复审#2 判致命，已删）。
    #[test]
    fn enter_ready_strict_conjunction() {
        // 满足全部：stage=CapitalRecovered, withdrawn≥i0(100), legs=0, normal, η≥η⋆。
        // free=0, holding=100, withdrawn=100, notional_in=100 ⟹ tw=200; L^wc=0 ⟹ η⋆(κ=0)=0; 200≥0 ✓。
        // ★注：tw=200 是 synthetic 态（合法性验证用），非 campaign 可达态（见上「诚实标注」）。
        let ready = TwState {
            free: 0, holding: 100, withdrawn: 100, notional_in: 100,
            stage: TStage::CapitalRecovered, open_legacy_legs: 0, cum_net_cash: 0,
        };
        let pol = RiskPolicy::baseline();
        assert!(pol.enter_ready(&ready, 100, true), "五条件全满足 ⟹ EnterReady");
        // 破坏 S=II：stage=CostReduction ⟹ 不 ready。
        assert!(!pol.enter_ready(&TwState { stage: TStage::CostReduction, ..ready }, 100, true), "S≠II ⟹ 不 ready");
        // 破坏 W≥I0：withdrawn<i0 ⟹ 不 ready。
        assert!(!pol.enter_ready(&TwState { withdrawn: 99, ..ready }, 100, true), "W<I0 ⟹ 不 ready");
        // 破坏 legacy 腿=0：open_legacy_legs=1 ⟹ 不 ready（OQ-9 入口证书）。
        assert!(!pol.enter_ready(&TwState { open_legacy_legs: 1, ..ready }, 100, true), "有 legacy 腿 ⟹ 不 ready");
        // 破坏 RiskNormal：risk_normal=false ⟹ 不 ready。
        assert!(!pol.enter_ready(&ready, 100, false), "非 RiskNormal ⟹ 不 ready");
        // 破坏 η≥η⋆：κ 拉高 barrier 到 η 之上 ⟹ 不 ready（此处 L^wc=0，用 κ·Q 抬门）。
        assert!(
            !RiskPolicy { kappa: 3 }.enter_ready(&ready, 100, true),
            "η(200)<η⋆(0+3·100=300) ⟹ barrier 未过 ⟹ 不 ready"
        );
    }

    /// BuyCore 合法性（PDF §10 步骤4 定理1 充要）：a_n+L^wc+κΔQ ≤ η+g−κQ。
    #[test]
    fn buy_core_legality() {
        let pol = RiskPolicy { kappa: 1 };
        // a_n=10, l_wc_next=5, ΔQ=3, η=100, g=0, Q=20 ⟹ LHS=10+5+3=18, RHS=100+0-20=80 ⟹ 18≤80 ✓。
        assert!(pol.buy_core_legal(10, 5, 3, 100, 0, 20), "建仓额小 ⟹ BuyCore 合法");
        // a_n=90（大建仓）⟹ LHS=90+5+3=98 > RHS=80 ⟹ 非法（超 barrier）。
        assert!(!pol.buy_core_legal(90, 5, 3, 100, 0, 20), "建仓额过大 ⟹ 破 κ-floor ⟹ 非法");
    }
}
