//! ★LEE M3 级别事件钟 `clock_ℓ`（multi-level-native-execution-design-20260719 §C.2 变化部分①
//! 「每级独立事件钟」 / §D 迁移表 M3 行 / §F 未决项②「clock_ℓ 事件集的最小完备定义」）。
//!
//! ## 本文件做什么：定义「哪些时点是级别 ℓ 的钟点」，**不**决定仓位
//!
//! M3 定义（设计文档 §D 迁移表逐字）：「Ledger_ℓ 只在 clock_ℓ 事件时点重估目标；无事件 bar
//! 目标=前值」。本模块只产**事件集**（`Vec<LevelClockTick>`）；目标的门控重估在
//! [`super::level_order::LevelOrderLedger::regate`]，订单投影在 `fill.rs`。
//!
//! ## 前置烤：clock_ℓ 事件集的最小完备定义（§F②，实装前逐字段对齐落档）
//!
//! 「最小完备」的判据不是教义枚举，而是**构造性**的：clock_ℓ 必须恰好覆盖使级别账本的
//! **结构目标** `net_ℓ`（[`super::level_ledger::level_nets`]）发生变化的全部来源，且不含
//! 不改变它的来源。据此逐字段对齐 `classifier::LevelState`（`classifier/mod.rs:178-208`
//! 六字段）与执行侧生产路径（`backtest/fill.rs` π loop）：
//!
//! | # | `LevelState` 字段 | 语义 | 进入执行侧决策的路径 | 驱动 `net_ℓ`？ | 入 clock_ℓ v1 |
//! |---|---|---|---|---|---|
//! | 1 | `bsp: Rc<Vec<BspPoint>>` | E_ℓ 买卖点 bit-vector | `newly_confirmed_step`（`backtest/signal.rs:38`）按 `(ℓ, source_index, bits_disc)` append-only diff → `step_gamma` → `interpret` 三桶 → `AncOK` → `legs` → `sep_legs` → `net_ℓ` | **是**（唯一直接源） | **是** [`LevelEventKind::BspConfirmed`] |
//! | 2 | `moves: Vec<MoveBlock>` | D_ℓ 走势分解块（「段完成」的载体） | `newly_confirmed_step` **置空**；`strategy/` 与 `backtest/` 生产路径对 `.moves` **零读点**（全仓 grep 确认） | 否（**不可观测**） | **否**（缺口，见下「照实登记的缺口」） |
//! | 3 | `centers: Rc<Vec<Center>>` | 中枢序列（「中枢生灭」的载体） | `newly_confirmed_step` **置空**；执行侧唯一读点 `fill.rs:986` 取 `levels[ℓ−1].centers` 作 pan_div 门的**跨级**参照 `sub_centers`，非本级事件 | 否（本级中枢生灭无载体） | **否**（缺口） |
//! | 4 | `cp_ownership: Rc<Vec<CpScanOwnership>>` | 中枢生命周期 Pending→Closed | `newly_confirmed_step` **置空**；唯一消费者 `backtest/opsem_dump.rs:72`（诊断导出，不进判定） | 否 | **否** |
//! | 5 | `pan_div: Rc<Vec<PanDivCert>>` | 盘整背驰证书（**非**买卖点，零 six-bit） | 走**全量** `classification_i`（非 step）→ `pan_div_state.observe_raw(ℓ, cert)` 首见去重 → 门 → `prepare` → 改写账户层净目标 `p_star_final`（`fill.rs:1104/1117`） | 改**账户层目标**，不改 `net_ℓ` | **是** [`LevelEventKind::PanDivCert`]（默认 config 惰性，但接线真实存在） |
//! | 6 | `level_projection: Option<..>` | #110 投影层 | 门关默认 `None`；「只描述不判定」零判定消费 | 否 | **否** |
//!
//! 六字段之外，另有两条**不经 `LevelState`** 的 `net_ℓ` 变化源——不登记则「最小完备」不成立：
//!
//! | 源 | 语义 | 驱动 `net_ℓ`？ | 入 clock_ℓ v1 |
//! |---|---|---|---|
//! | `StepTrace.{closed, opened, silent_drops, overlay_closes, risk_exits}`（`coverage.rs` StepTrace） | 该级腿的生命周期落点（反向关闭 / 准入开仓 / AncOK 连带剪 / P2 CloseOverlay / P1 force_flat 强平） | **是**（腿集合即 `sep_legs` 的定义域） | **是**（按 `ActiveLeg.level` 分级；`risk_exits` 归风控类，见下） |
//! | `base_units = equity_nav / px`（`fill.rs:770`） | 每 bar 的 NAV/价漂移；`q_units = base_units × w_depth × w_dir` | **是**——`net_ℓ` 因此**每 bar 都在变，与结构事件无关** | **否**——这正是 M3 要门控掉的「bar tick 碎片」（设计文档 §B.1 根因一） |
//! | `StepTrace.tw_event`（P3 RecoverCapital / P4 EnterEarning） | TW 账本相位事件 | **否（抑制器，非产生器）**——见下 | **否** |
//! | `pan_div_state.signed_live_child_units()` | P7/P9 中枢震荡在飞子腿的账户层净目标分量 | **否（不经 `net_ℓ`）**，但改**账户层结构目标** | **否**（其生灭已由 `PanDivCert` 覆盖；数值经 `plan_level_gated_order` ① 显式接入结构目标，不走 clock） |
//!
//! `tw_event` 需单独论证才能说「最小完备」成立：它成立时 gamma 全部推迟 record 桶、屏蔽
//! P5..P10（`coverage.rs` StepTrace 文档逐字），本 bar 因此**不产生** `opened`。也就是说它
//! **抑制**本该发生的 clock 事件，而不是产生新的 `net_ℓ` 取值——已有腿的 `net_ℓ` 不因它改变。
//! 抑制器不需要自己成为钟点：被抑制的 bar 上「无事件 ⟹ 保前值」正是期望行为。故不入
//! clock_ℓ v1，但必须在此登记，否则「覆盖全部 `net_ℓ` 变化源」的论证不完整。
//!
//! `base_units` 那一行是本次烤的**核心结论**：`net_ℓ` 逐 bar 变化的主因不是结构事件，而是
//! `base_units` 的价格/NAV 漂移。M3 的门控切断的正是这条**非结构**通道——这给了「无事件 bar
//! 目标=前值」一个构造性的、可证伪的含义（而非仅仅是节流优化）。
//!
//! ### 照实登记的缺口（090 反声明膨胀；**不**冒充完备）
//!
//! 1. **「段完成」与「中枢生灭」在现行实装的执行侧不可观测**（字段 2/3/4 全部被
//!    `newly_confirmed_step` 置空，且生产路径零读点）。设计文档 §C.2 伪码注释所列
//!    「BSP 生灭/中枢生灭/段完成」三类中，后两类在 v1 **无载体**。本模块因此声明的是
//!    **「相对现行执行侧可观测面的最小完备」**，不是「相对缠论教义的最小完备」。补齐这两类
//!    需要先在 `newly_confirmed_step` 增设 centers/moves 的 append-only diff 通道
//!    （跨票域，M3 不做——那会改变候选生成从而改变 M0 语义）。
//! 2. **「BSP 灭」无载体**：`newly_confirmed_step` 是 append-only（`seen` 只增不删），BSP 失效
//!    （E2E-D5 `Invalidated`）在 rust 侧不存在（盘点 `e2e-existing-implementation-synthesis-20260720.md:24`
//!    「BSP Invalidated 结构事件不存在」）。本模块的「灭」侧由**腿生命周期**（LegClosed/
//!    LegSilentDrop/LegOverlayClose）承载——这是执行侧的灭，不是结构侧的灭，二者**不等价**。
//! 3. **E2E 五钟只有 1/5 实装**：`observed/first_provable/structure_end/confirmed/invalidated`
//!    五钟（`mainline-merged-roadmap-20260717.md:75` E2E-N5）中，rust 仅有 `judge_at` 单钟
//!    一钟多职（`classifier/level_view.rs:489`、`classifier/nest.rs:419`），其余四钟字段级零命中
//!    （同盘点 :24「缺 4/5」）。更要紧的是：现存的 `judge_at` 载体挂在 **nest 证书门**
//!    （`backtest/admission.rs` 的 `events_by_level`，`THETA_NEST_CERT_GATE` **默认关**）⟹
//!    生产默认路径上**根本没有** `judge_at` 实例可与 clock_ℓ 逐点对拍。
//!
//!    因此本票交付的**不是**「clock_ℓ 与五钟的时点一致性」，而是弱一级的
//!    **「clock_ℓ 满足与 `judge_at` 同一条首次观察纪律」**（因果 + 首次唯一 + 钟点不虚高，
//!    见 `runner.rs` 的 `lee_m3_clock_obeys_first_observation_discipline`）。「首次唯一」的
//!    **有效域是类型化身份** `(ℓ, source_index, bits_disc)`（上表第 1 行的 diff 键），**不是**
//!    锚点 `(ℓ, source_index)`——同一锚点上 1/2/3 类点可共存（`signal.rs::bsp_bits_disc` 文档），
//!    #361 在 3500-bar 上实测坐实（同 bar 一锚三类，无跨 bar 渐进重分类）。锚点级多响
//!    **不**使钟点虚高：BSP 通道按级别取（`fill.rs` 的 `!ls.bsp.is_empty()`）再经
//!    [`LevelClockTicks`] 的 `(level, kind)` BTreeSet 去重 ⟹ 同级同 bar n 点恒塌缩为 1 tick。
//!    「一致性实证」这一条**未兑现**，
//!    照实登记为缺口：兑现它需要先落 E2E-N5 的四钟载体（跨票域），或把 nest 门纳入默认路径
//!    （改变 M0 语义，M3 明确不做）。
//!
//! ## 结构钟 vs 风控钟（§F③「风控门与事件门控的优先级形式化」的落点）
//!
//! 票体硬约束：「事件门控只门控结构交易，不门控风控」。形式化为**两个不同的域**，因此
//! 二者之间**不存在优先级冲突**（无需仲裁规则）：
//!
//! - **门控域** = 结构目标 `plan_ℓ` 的**重估**（何时把 `net_ℓ` 写进 `plan_ℓ`）——由 clock_ℓ 决定；
//! - **风控域** = `plan → 订单`的**投影**（`𝒦_Θ` 收窄：`force_flat ⟹ {0}`、`stop_long ⟹ 禁净多`）
//!   ——每 bar 无条件施加，由 `k_theta_risk_gate` 决定，**不读** clock_ℓ。
//!
//! 稀疏性因此必须按域分别表述（否则会把风控订单误判为门控失效）：
//!
//! ```text
//! {i : order_units(i) ≠ 0} ⊆ (∪_ℓ clock_ℓ^struct) ∪ RiskTick ∪ CapTick
//! ```
//!
//! 其中 `RiskTick` = 风控门非全开的 bar（[`LevelEventKind::LegRiskExit`] 同源），
//! `CapTick` = `𝒦_Θ` 帽 binding 使投影目标偏离结构目标的 bar。在既无风控触发又无帽 binding
//! 的 bar 上退化为严格 `⊆ ∪_ℓ clock_ℓ^struct`——这是可实证的强形式，见 `runner.rs` 验收。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! - **L0**：事件集定义本身（给定 `classification_step` + `StepTrace` 后是全函数）；
//! - **L1**：稀疏性 `{order≠0} ⊆ ∪clock ∪ Risk ∪ Cap` 与门控前后的订单流差异幅度——两者都
//!   实测于**合成** `random_walk_dataset`（管线正确性验证，可证伪但输入自造）⟹ 按
//!   `formalization-validity-domain` 表一律 **L1**，**不是 L2**（L2 需真实行情数据）。差异
//!   幅度**纯观测无断言**（M3 起订单流与 M0 分叉，差异不为 0 是契约本身，不是缺陷；**不得**
//!   以 M2 的 bit-exact 蒙混，设计文档 §D M3 逐字）。
//! - **L2/L3 一律缺席**：本票未在任何真实行情上验证 M3，不作任何跨标的/跨时段声明。
//!
//! 本模块**不声明 alpha**（v3 硬禁令：历史/合成数据只验代码正确性与不变量）。

use std::collections::BTreeSet;

/// `clock_ℓ` 的事件类型（六结构类 + 一风控类；见模块头对齐表）。
///
/// 分类依据是**域**而非严重性：`is_structural()==true` 的类进入稀疏性分母
/// （「订单时点 ⊆ 事件时点并集」的右端），风控类不进——风控每 bar 生效，
/// 把它算进结构事件集会让稀疏性验收自动通过（平凡）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LevelEventKind {
    /// E1 结构：本 bar 首次确认的该级买卖点（`newly_confirmed_step` 的 append-only diff）。
    BspConfirmed,
    /// E2 结构：该级腿被 `interpret` 规则2 反向关闭（`StepTrace.closed`）。
    LegClosed,
    /// E3 结构：该级候选经 AncOK 真正准入 `next_active`（`StepTrace.opened`）。
    LegOpened,
    /// E4 结构：该级腿不在 close 桶却从 active 消失（AncOK 连带剪 / Stale prune，
    /// `StepTrace.silent_drops`）。
    LegSilentDrop,
    /// E5 结构：P2 CloseOverlay 关闭的该级重叠腿（`StepTrace.overlay_closes`）。
    LegOverlayClose,
    /// E6 结构：该级盘整背驰证书首见并过门（改写账户层净目标，不改 `net_ℓ`）。
    PanDivCert,
    /// E7 **风控**：P1 `force_flat` 强平清空的该级腿（`StepTrace.risk_exits`）。
    ///
    /// 归风控域：它由 `k_theta_risk_gate` 而非结构信号触发。计入 clock_ℓ（目标必须重估——
    /// 腿真的没了），但 `is_structural()==false` ⟹ **不**进稀疏性分母。
    LegRiskExit,
}

impl LevelEventKind {
    /// 该事件是否属**结构**域（进稀疏性分母）。风控域（[`Self::LegRiskExit`]）返回 `false`。
    pub fn is_structural(self) -> bool {
        !matches!(self, LevelEventKind::LegRiskExit)
    }
}

/// 单个钟点（级别 + 类型）。同一 bar 同一级别可有多个 tick（多类事件并发）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelClockTick {
    /// 级别 ℓ（= `ElementId.level` / `ActiveLeg.level` / `LevelState` 下标，**同一口径**，
    /// 与 M1/M2 的分桶键 `id.level` ≡ formation_level 单源；不新增口径）。
    pub level: u32,
    /// 事件类型。
    pub kind: LevelEventKind,
}

/// 单 bar 的钟点集（确定序：按 `(level, kind)` 升序去重）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelClockTicks {
    ticks: BTreeSet<LevelClockTick>,
}

impl LevelClockTicks {
    /// 空钟（本 bar 无任何级别事件 ⟹ 全级别 hold）。
    pub fn empty() -> Self {
        LevelClockTicks::default()
    }

    /// 登记一个钟点（幂等——同 `(level, kind)` 重复登记不重复计数）。
    pub fn insert(&mut self, level: u32, kind: LevelEventKind) {
        self.ticks.insert(LevelClockTick { level, kind });
    }

    /// 本 bar 全部钟点（`(level, kind)` 升序，确定序）。
    pub fn iter(&self) -> impl Iterator<Item = &LevelClockTick> {
        self.ticks.iter()
    }

    /// 本 bar 有任意（结构或风控）钟点的级别集合——[`super::level_order::LevelOrderLedger::regate`]
    /// 的门控输入。
    pub fn ticked_levels(&self) -> BTreeSet<u32> {
        self.ticks.iter().map(|t| t.level).collect()
    }

    /// 本 bar 有**结构**钟点的级别集合（稀疏性分母；风控类不计）。
    pub fn structural_levels(&self) -> BTreeSet<u32> {
        self.ticks
            .iter()
            .filter(|t| t.kind.is_structural())
            .map(|t| t.level)
            .collect()
    }

    /// 本 bar 是否存在任何结构钟点（稀疏性判据左端的取反）。
    pub fn has_structural(&self) -> bool {
        self.ticks.iter().any(|t| t.kind.is_structural())
    }

    /// 本 bar 是否存在风控钟点（`RiskTick` 的 clock 侧证据）。
    pub fn has_risk(&self) -> bool {
        self.ticks.iter().any(|t| !t.kind.is_structural())
    }

    /// 钟点总数。
    pub fn len(&self) -> usize {
        self.ticks.len()
    }

    /// 本 bar 无任何钟点。
    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }
}

/// 逐 bar 钟点累计读数（**release 可见**，非 `debug_assert`；#289 影子评审 MED ① 同款纪律）。
///
/// 「稀疏」若只有定义没有读数，等于没有证据——本结构让稀疏度在 release 跑批里可读：
/// `n_bars_with_structural_tick / n_bars` 就是结构钟相对 bar 全集的稀疏度。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelClockStats {
    /// 观测到的决策点数（分母）。
    pub n_decisions: u64,
    /// 至少有一个**结构**钟点的决策点数（**非平凡性证据**：>0 才说明钟真的在响）。
    pub n_bars_with_structural_tick: u64,
    /// 至少有一个**风控**钟点的决策点数。
    pub n_bars_with_risk_tick: u64,
    /// 钟点总数（跨 bar 累计，含风控类）。
    pub n_ticks_total: u64,
    /// 逐类计数（下标 = 类判别式；确定序读出见 [`Self::kind_count`]）。
    counts: [u64; 7],
}

impl LevelClockStats {
    fn kind_slot(kind: LevelEventKind) -> usize {
        match kind {
            LevelEventKind::BspConfirmed => 0,
            LevelEventKind::LegClosed => 1,
            LevelEventKind::LegOpened => 2,
            LevelEventKind::LegSilentDrop => 3,
            LevelEventKind::LegOverlayClose => 4,
            LevelEventKind::PanDivCert => 5,
            LevelEventKind::LegRiskExit => 6,
        }
    }

    /// 逐决策点登记。
    pub fn observe(&mut self, ticks: &LevelClockTicks) {
        self.n_decisions += 1;
        if ticks.has_structural() {
            self.n_bars_with_structural_tick += 1;
        }
        if ticks.has_risk() {
            self.n_bars_with_risk_tick += 1;
        }
        for t in ticks.iter() {
            self.n_ticks_total += 1;
            self.counts[Self::kind_slot(t.kind)] += 1;
        }
    }

    /// 某类事件的累计钟点数。
    pub fn kind_count(&self, kind: LevelEventKind) -> u64 {
        self.counts[Self::kind_slot(kind)]
    }

    /// 结构钟稀疏度证据成立：钟真的响过（>0）**且**严格稀疏于 bar 全集（< n_decisions）。
    ///
    /// 两侧都要——恒不响（0）说明门控在空转，每 bar 都响（==n）说明门控无效（退化回 M0 bar tick）。
    pub fn sparsity_witnessed(&self) -> bool {
        self.n_decisions > 0
            && self.n_bars_with_structural_tick > 0
            && self.n_bars_with_structural_tick < self.n_decisions
    }
}

/// ★clock_ℓ 事件集判定（给定输入后的**全函数**，L0）。
///
/// 输入是执行侧生产路径**已有**的三份产出（不新增第二查法，不重算结构）：
///
/// - `newly_confirmed_bsp_levels`：`newly_confirmed_step` 输出中 `bsp` 非空的级别下标
///   （`backtest/signal.rs:38` 的 append-only diff 结果，**唯一** BSP 事件源）；
/// - `trace`：本步 `StepTrace` 的腿生命周期五通道（`closed/opened/silent_drops/
///   overlay_closes/risk_exits`），按 `ActiveLeg.level` / `Candidate.level` 分级；
/// - `pan_div_levels`：本 bar 首见并过门的盘整背驰证书所在级别。
///
/// 顺序无关（结果是集合），确定序由 [`LevelClockTicks`] 的 `BTreeSet` 保证。
pub fn collect_ticks(
    newly_confirmed_bsp_levels: &[u32],
    closed_levels: &[u32],
    opened_levels: &[u32],
    silent_drop_levels: &[u32],
    overlay_close_levels: &[u32],
    risk_exit_levels: &[u32],
    pan_div_levels: &[u32],
) -> LevelClockTicks {
    let mut ticks = LevelClockTicks::empty();
    for &l in newly_confirmed_bsp_levels {
        ticks.insert(l, LevelEventKind::BspConfirmed);
    }
    for &l in closed_levels {
        ticks.insert(l, LevelEventKind::LegClosed);
    }
    for &l in opened_levels {
        ticks.insert(l, LevelEventKind::LegOpened);
    }
    for &l in silent_drop_levels {
        ticks.insert(l, LevelEventKind::LegSilentDrop);
    }
    for &l in overlay_close_levels {
        ticks.insert(l, LevelEventKind::LegOverlayClose);
    }
    for &l in risk_exit_levels {
        ticks.insert(l, LevelEventKind::LegRiskExit);
    }
    for &l in pan_div_levels {
        ticks.insert(l, LevelEventKind::PanDivCert);
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★事件类型的域划分（§F③ 形式化的落点）：六结构类进稀疏性分母，风控类不进。
    #[test]
    fn risk_exit_is_the_only_non_structural_kind() {
        let all = [
            LevelEventKind::BspConfirmed,
            LevelEventKind::LegClosed,
            LevelEventKind::LegOpened,
            LevelEventKind::LegSilentDrop,
            LevelEventKind::LegOverlayClose,
            LevelEventKind::PanDivCert,
            LevelEventKind::LegRiskExit,
        ];
        let non_struct: Vec<_> = all.iter().filter(|k| !k.is_structural()).collect();
        assert_eq!(
            non_struct,
            vec![&LevelEventKind::LegRiskExit],
            "风控域恰一类"
        );
        assert_eq!(
            all.iter().filter(|k| k.is_structural()).count(),
            6,
            "结构域六类"
        );
    }

    /// ★钟点集确定序 + 幂等：同 `(level, kind)` 重复登记不产生重复项，迭代按 `(level, kind)` 升序。
    #[test]
    fn ticks_are_deduped_and_deterministically_ordered() {
        let mut t = LevelClockTicks::empty();
        t.insert(2, LevelEventKind::LegOpened);
        t.insert(1, LevelEventKind::BspConfirmed);
        t.insert(2, LevelEventKind::LegOpened); // 重复
        t.insert(1, LevelEventKind::LegClosed);
        assert_eq!(t.len(), 3, "幂等去重");
        let seq: Vec<_> = t.iter().copied().collect();
        assert_eq!(
            seq,
            vec![
                LevelClockTick {
                    level: 1,
                    kind: LevelEventKind::BspConfirmed
                },
                LevelClockTick {
                    level: 1,
                    kind: LevelEventKind::LegClosed
                },
                LevelClockTick {
                    level: 2,
                    kind: LevelEventKind::LegOpened
                },
            ],
            "按 (level, kind) 升序"
        );
    }

    /// ★结构/风控级别集分离：风控 tick 使级别进 `ticked_levels`（目标须重估）但**不**进
    /// `structural_levels`（不进稀疏性分母）。
    #[test]
    fn risk_tick_gates_target_but_not_sparsity_denominator() {
        let mut t = LevelClockTicks::empty();
        t.insert(3, LevelEventKind::LegRiskExit);
        assert_eq!(
            t.ticked_levels(),
            [3u32].into_iter().collect(),
            "风控 tick ⟹ 该级目标须重估"
        );
        assert!(t.structural_levels().is_empty(), "风控 tick 不进结构级别集");
        assert!(!t.has_structural(), "无结构钟点");
        assert!(t.has_risk(), "有风控钟点");
    }

    /// ★`collect_ticks` 是给定输入后的全函数：七通道逐一落对应类型，级别原样透传。
    #[test]
    fn collect_ticks_maps_each_channel_to_its_kind() {
        let t = collect_ticks(&[0], &[1], &[2], &[3], &[4], &[5], &[6]);
        let got: Vec<_> = t.iter().copied().collect();
        assert_eq!(
            got,
            vec![
                LevelClockTick {
                    level: 0,
                    kind: LevelEventKind::BspConfirmed
                },
                LevelClockTick {
                    level: 1,
                    kind: LevelEventKind::LegClosed
                },
                LevelClockTick {
                    level: 2,
                    kind: LevelEventKind::LegOpened
                },
                LevelClockTick {
                    level: 3,
                    kind: LevelEventKind::LegSilentDrop
                },
                LevelClockTick {
                    level: 4,
                    kind: LevelEventKind::LegOverlayClose
                },
                LevelClockTick {
                    level: 5,
                    kind: LevelEventKind::LegRiskExit
                },
                LevelClockTick {
                    level: 6,
                    kind: LevelEventKind::PanDivCert
                },
            ]
        );
        assert!(
            collect_ticks(&[], &[], &[], &[], &[], &[], &[]).is_empty(),
            "全空 ⟹ 空钟"
        );
    }

    /// ★稀疏度见证的**双侧**要求：恒不响（空转）与每 bar 都响（退化回 bar tick）都不算见证。
    #[test]
    fn sparsity_witness_requires_both_nonempty_and_strictly_sparse() {
        let mut never = LevelClockStats::default();
        for _ in 0..10 {
            never.observe(&LevelClockTicks::empty());
        }
        assert!(!never.sparsity_witnessed(), "钟恒不响 ⟹ 门控空转，不算见证");

        let mut always = LevelClockStats::default();
        for _ in 0..10 {
            let mut t = LevelClockTicks::empty();
            t.insert(1, LevelEventKind::BspConfirmed);
            always.observe(&t);
        }
        assert!(
            !always.sparsity_witnessed(),
            "每 bar 都响 ⟹ 退化回 M0 bar tick，不算见证"
        );

        let mut sparse = LevelClockStats::default();
        for i in 0..10 {
            let mut t = LevelClockTicks::empty();
            if i % 3 == 0 {
                t.insert(1, LevelEventKind::BspConfirmed);
            }
            sparse.observe(&t);
        }
        assert!(sparse.sparsity_witnessed(), "响过且严格稀疏 ⟹ 见证成立");
        assert_eq!(sparse.n_decisions, 10);
        assert_eq!(sparse.n_bars_with_structural_tick, 4);
        assert_eq!(sparse.kind_count(LevelEventKind::BspConfirmed), 4);
    }

    /// ★风控钟不进结构稀疏度分子（否则风控频繁触发会让稀疏性验收平凡通过）。
    #[test]
    fn risk_ticks_do_not_inflate_structural_sparsity() {
        let mut s = LevelClockStats::default();
        for _ in 0..10 {
            let mut t = LevelClockTicks::empty();
            t.insert(0, LevelEventKind::LegRiskExit);
            s.observe(&t);
        }
        assert_eq!(s.n_bars_with_risk_tick, 10);
        assert_eq!(s.n_bars_with_structural_tick, 0, "风控不计入结构分子");
        assert!(!s.sparsity_witnessed(), "只有风控钟 ⟹ 结构钟见证不成立");
        assert_eq!(s.kind_count(LevelEventKind::LegRiskExit), 10);
    }
}
