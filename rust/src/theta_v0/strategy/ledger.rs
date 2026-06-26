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
//! ## TW 三阶段 / OQ-9 gate 的 Origin 锚点缺位（no-workaround 诚实声明）
//!
//! `TwState`/`tw_step`/`TStage` 三阶段/`is_legal_from` OQ-9 gate **在 Origin canonical 中无对应
//! 锚点**：`formal/Origin/` 六层闭包（SourceAxioms/ChanlunElements/CompleteClassification/
//! TrendCompleteClassification/FullDefinitionStrategy/TraceProjection/BehaviorQuotient/
//! FiniteTraceQuotient）只含 `R=Π-A-W` 账本（FullDefinitionStrategy.LedgerState），**不含**
//! `TW=free+holding+withdrawn` 守恒 / 取本金三阶段 / OQ-9 单向 gate。
//!
//! 故 TW/OQ-9 层的契约锚点**仍指向 legacy `Tlayers/Accounting/TotalWealth.lean`**——这是诚实
//! 声明的有效域边界，**不是** workaround：不臆造不存在的 Origin TW 接口、不把 TW 语义硬塞进
//! Origin LedgerState（二者不同构，#90 已证）。Origin TW 端口（把 TW 三阶段/OQ-9 重锚为 Origin
//! canonical 结构）尚未存在 → TW 契约重锚**诚实延后**至 Origin TW 端口落地后。
//!
//! ## 双账本并置（task #90 不同构裁定 → task #93 闭环扩维）
//!
//! `R=Π-A-W`（收益表视角，Origin canonical）与 `TW=free+holding+withdrawn`（现金流+持仓视角，
//! legacy Tlayers）**不同构**（#90 已证：外生价格维度 / 状态空间结构 / 守恒律切换三条阻断）。
//! 完整持仓系统两者都需要——故 closed_loop 把两者并置在 AssemblyState，T 同步线程化两者
//! （见 transition.rs）。重锚后并置不变：R=Π-A-W 锚 Origin，TW/OQ-9 锚 legacy（待 Origin 端口）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：Rust 类型/算子与 Origin/legacy 定义结构对齐 = 验证管线
//!   正确性，零信息增量）。`cargo test` 通过 = 双账本不变量在每个算子下结构成立，**不**是
//!   任何缠论盈利 / 实盘有效声明（那是 L2/L3，真实数据回测才可否证）。重锚到 Origin **不**
//!   提升等级——锚点换 canonical 来源仍是 L0/L1（验管线非验缠论假设）。
//! - 不变量 `R=Π-A-W`（ledger，Origin）+ `TW=free+holding+withdrawn`（tw，legacy）+ stage
//!   单向（OQ-9，legacy）都是**结构恒等**（同价 c 固定下成立），不证账本数值反映实盘真实盈亏。
//!
//! ## 契约锚点源（只读，不改 .lean）
//!
//! - LedgerComp / LedgerEvent / ledger_step → **`Origin.FullDefinitionStrategy.LedgerState`**
//!   （LedgerState/mkLedger/ledgerStep/ledger_invariant_preservation，:184-203）。
//! - TwState / TStage / TwEvent / tw_step / OQ-9 gate → legacy `Tlayers/Accounting/TotalWealth.lean`
//!   （Origin 锚点缺位，见上「TW 三阶段 / OQ-9 gate 的 Origin 锚点缺位」诚实声明）。

/// 取本金三阶段 `TStage`（镜像 Lean `TotalWealth.TStage`，缠师第31课）。
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
    /// 阶段序（镜像 Lean `TStage.rank`）：单向不可逆迁移的偏序载体。
    pub fn rank(self) -> u32 {
        match self {
            TStage::CostReduction => 0,
            TStage::CapitalRecovered => 1,
            TStage::EarningShares => 2,
        }
    }
}

/// 单向阶段推进（镜像 Lean `advanceTo`）：rank 只前进，不回退。
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

/// TW 账本态 `TwState`（镜像 Lean `TotalWealth.TWState`，模型B 取本金三阶段）。
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

    /// 总财富 `TW = free + holding + withdrawn`（镜像 Lean `TWState.tw`，守恒量）。
    pub fn tw(&self) -> i64 {
        self.free + self.holding + self.withdrawn
    }
}

/// TW 账本事件 `TWEvent`（镜像 Lean `TotalWealth.TWEvent`，对齐 t_engine 三阶段算子 + OQ-9）。
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
    /// OQ-9 合法转移谓词 `LegalTransition`（镜像 Lean `TotalWealth.LegalTransition`，
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

/// TW 更新 `tw_step`（镜像 Lean `TotalWealth.twStep`，全函数，**保 TW 守恒**）。
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
/// 它与 [`TwState`] **不同构**（#90 已证），双账本并置——R=Π-A-W 锚 Origin，TW 锚 legacy。
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
    //  TwState TW 守恒 + stage 单向（锚 legacy TotalWealth.twStep_preserves_tw /
    //  stage_rank_monotone——Origin 锚点缺位，见模块头「TW 三阶段 / OQ-9 gate 的 Origin 锚点缺位」）
    // ──────────────────────────────────────────────────────────────────────

    /// ★tw_step 保 TW 守恒（镜像 Lean `twStep_preserves_tw`）：逐事件 TW=free+holding+withdrawn 不变。
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

    /// ★stage 单向不可逆（镜像 Lean `stage_rank_monotone`）：非 clearCampaign 算子下 rank 只增不减。
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

    /// ★EarningShares 不可回退（镜像 Lean `earning_no_regress`）：earning 态非 clearCampaign 算子保持 earning。
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
    //  OQ-9 gate（锚 legacy TotalWealth.LegalTransition / LegalEnterEarning——Origin 锚点缺位）
    // ──────────────────────────────────────────────────────────────────────

    /// OQ-9 入口证书：EnterEarning 须 open_legacy_legs==0（镜像 Lean `LegalEnterEarning`）。
    #[test]
    fn oq9_enter_earning_gate() {
        let with_leg = TwState { open_legacy_legs: 1, ..TwState::initial() };
        let no_leg = TwState { open_legacy_legs: 0, ..TwState::initial() };
        assert!(!TwEvent::EnterEarning.is_legal_from(&with_leg), "带 legacy 腿进 earning 非法");
        assert!(TwEvent::EnterEarning.is_legal_from(&no_leg), "无 legacy 腿进 earning 合法");
    }

    /// ★OQ-9 核心：earning 阶段开 legacy 腿非法（镜像 Lean LegalTransition openShareLeg 修正1）。
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

    /// CloseShareLeg 须有腿（open_legacy_legs>=1），否则幽灵腿非法（镜像 Lean 修正4(c)）。
    #[test]
    fn oq9_no_ghost_close() {
        let no_leg = TwState { open_legacy_legs: 0, ..TwState::initial() };
        let one_leg = TwState { open_legacy_legs: 1, ..TwState::initial() };
        assert!(!TwEvent::CloseShareLeg(0).is_legal_from(&no_leg), "无腿可闭 = 幽灵非法");
        assert!(TwEvent::CloseShareLeg(0).is_legal_from(&one_leg), "有腿可闭合法");
    }

    /// ★OQ-9 端B（镜像 Lean `legal_earning_no_legacy_leg_closure`）：合法进 earning 后闭 legacy 腿非法。
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

    /// ClearCampaign 须先清 legacy 腿（镜像 Lean 修正4(b)）。
    #[test]
    fn oq9_clear_campaign_gate() {
        let with_leg = TwState { open_legacy_legs: 1, ..TwState::initial() };
        let no_leg = TwState { open_legacy_legs: 0, ..TwState::initial() };
        assert!(!TwEvent::ClearCampaign.is_legal_from(&with_leg), "带腿清 campaign 非法（不绕 gate）");
        assert!(TwEvent::ClearCampaign.is_legal_from(&no_leg), "无腿清 campaign 合法");
    }
}
