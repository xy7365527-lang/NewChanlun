//! 战役记账族（#1191 D01 职责块自 `oscillation_campaign.rs` 迁出，零行为）。
//!
//! 承载 [`CampaignOutcome`]/[`SuspensionBatch`]/[`CoverAssignment`]/[`SuspensionAttribution`]/
//! [`UnclosedReduction`]/[`ReplenishPlan`]/[`DeathWriteOff`] 及各自 impl。消费面经
//! `oscillation_campaign` 重导出保持原路径；记账语义经 [`super::campaign_book::CampaignBook`]
//! 的测试套驱动覆盖（本模块无直接单元测试，纯移动零行为）。

use super::super::classifier::center_lifecycle::CenterId;
use super::ledger::{LedgerComp, TwEvent, TwState};
use super::voice::VoiceSide;

// 文档链引用（rustdoc 解析用；无代码消费）。
#[allow(unused_imports)]
use super::oscillation_campaign::{CampaignWiringWitness, OscillationCampaign};
#[allow(unused_imports)]
use super::short_diff_bucket::{self, ShortDiffViolation};

/// 一次动作应用的产出：本次真实成交 units + 应用后 TW 态 + 若本次触发阶段推进则携带该事件
/// （`RecoverCapital`=到 0 转移证据，`EnterEarning`=进增股数证据；`None`=本次未推进阶段）。
///
/// ★#380：新增 `cover`（本次回补的逐批冲抵明细，项四挂起归属）与 `loss_accounted`
/// （本次动作落账后桶现金为负 ⟹ 亏损往返如实入账，项一）——纯观测产出，不参与任何裁决。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignOutcome {
    pub units: i64,
    pub tw: TwState,
    pub ledger: LedgerComp,
    pub stage_event: Option<TwEvent>,
    /// 本次 `Replenish` 的逐批冲抵明细（同中枢优先、余按时间序；`Reduce` 恒空）。
    pub cover: Vec<CoverAssignment>,
    /// ★#380 项一：本次动作落账后桶现金为负（`tw.free<0`）——旧口径下这笔整笔被拒并回滚，
    /// 新口径如实入账，此标志是「亏损入账」的产物级见证（替代退役的
    /// `resource_exhausted_free_negative_count` 桶）。
    pub loss_accounted: bool,
    /// ★#383：本次动作是**等金额回补**（回补量按 `floor(桶现金÷回补价)` 算出，而非等量买回）
    /// ——阶段三切换后、且本次冲抵的挂起批次里含阶段三锁定批次时为真。
    pub earning_replenish: bool,
    /// ★#383：本次回补的**净增股数**（超出挂起在途量、用桶现金新买的那部分；非等金额腿恒 0）
    /// ——补充八题二「股数不设门，净增股数进 witness 观测」的产物级读数。
    pub earning_units_gained: i64,
    /// ★#383 订正（2026-07-27 用户裁定，ADR 补充十「到 0 判据 = 挂起空 ∧ free≥本金」）：
    /// 本次动作落账后 `free` 已够本金（`stage_progression` 会派事件），但桶挂起在途量非空
    /// ⟹ 阶段推进被**前置拦下**（`stage_event` 记 `None`）。纯观测产出，不参与任何裁决——
    /// 「到 0 掺水被拦」不静默，生产侧计入
    /// [`CampaignWiringWitness::profit_ready_but_suspended`]。
    pub stage_progress_suspended: bool,
}

/// ★#380 项四：一次 `Reduce` 产生的挂起批次——在途量 + **来源中枢标签** + 到达序（时间序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuspensionBatch {
    pub center: CenterId,
    pub units: i64,
    /// 到达序（campaign 内单调递增）——「余按时间序」的确定性载体（非哈希/非中枢序）。
    pub seq: u64,
    /// ★#366（订正 2026-07-27，评审 §2.4）：本批减出腿**桶实收现金**——该次 `Reduce` 使
    /// [`short_diff_bucket::ShortDiffBucket::realized_cash`] 增加的金额，即「未闭合减出」核销
    /// 时现金那一笔的唯一算料（见 [`UnclosedReduction::cash_booked`]）。
    ///
    /// 取值方式 = 记账放行**前后**读桶 `realized_cash` 之差（[`OscillationCampaign::apply_action`]）
    /// ——不在本模块重算第二套公式，故两侧口径恒与桶一致：
    /// - 多头「减=卖出」：`units·avg_cost`（成本基 holding→free）+ `units·(price−avg_cost)`
    ///   （已实现盈亏）= `units·price`，恰为卖出成交额（现金流入）。
    /// - 空头「减=回补空头（买回）」：`units·avg_cost` + `units·(avg_cost−price)`
    ///   = `units·(2·avg_cost−price)`，**不等于** `units·price`，且成交腿本身是现金**支出**
    ///   ——旧口径按 `Σ units·price` 记且命名为「卖出成交额」在空头侧值与符号双错。
    ///
    /// 桶只记标量总现金（无分批分解），故该分量随批次记在归属账侧（观测口径，不另立记账）。
    pub cash_booked: i64,
    /// ★#383（ADR 补充八题三「旧账按卖出时锁定」）：本批**减出那一刻**的 sizing 模式——
    /// `false`=阶段一（回补等量买回），`true`=阶段三（回补按等金额 `floor(现金÷价)`）。
    /// 每笔往返的规矩在卖出时锁死，中途不变：切换前挂着的批次即便在阶段三收口也仍走等量口径。
    pub earning: bool,
}

/// ★#380 项四：一次 `Replenish` 对某个挂起批次的冲抵明细。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverAssignment {
    /// 被冲抵批次的来源中枢。
    pub center: CenterId,
    pub units: i64,
    pub seq: u64,
    /// 该批次的来源中枢是否与本次回补触发的中枢相同（`true`=同中枢优先段冲抵的）。
    pub same_center: bool,
}

/// ★#380 项四（ADR 补充七「挂起归属可观测」，#367 项三 A 裁）：挂起归属账——campaign 粒度仍
/// **按级别**（#294 决议 2 不动），但挂起在途量按**来源中枢**分批标签化，收口顺序=
/// **同中枢优先、余按时间序**。
///
/// 与 [`short_diff_bucket::ShortDiffBucket::open_units`] 的关系：桶记标量总量（记账口径，恒仓
/// 断言的唯一来源），本账记同一总量的**归属分解**（观测口径）——两者逐笔同步推进，
/// [`Self::open_units`] 与桶的 `open_units()` 恒等（见测试
/// `suspension_attribution_total_tracks_bucket_open_units`）。不另立第二套记账（同 High-2
/// 「一个快照接口，零派生副本」精神）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SuspensionAttribution {
    batches: Vec<SuspensionBatch>,
    next_seq: u64,
}

impl SuspensionAttribution {
    /// 当前挂起在途总量（=各批次之和，与短差桶标量恒等）。
    pub fn open_units(&self) -> i64 {
        self.batches.iter().map(|b| b.units).sum()
    }

    /// 当前挂起批次（按到达序，只读观测）。
    pub fn batches(&self) -> &[SuspensionBatch] {
        &self.batches
    }

    /// `Reduce` 入队：新批次带来源中枢标签 + 本笔桶实收现金（#366 核销算料）+ 卖出时锁定的
    /// sizing 模式（#383 补充八题三），追加在时间序末尾。
    pub(super) fn push(&mut self, center: CenterId, units: i64, cash_booked: i64, earning: bool) {
        self.batches.push(SuspensionBatch {
            center,
            units,
            seq: self.next_seq,
            cash_booked,
            earning,
        });
        self.next_seq += 1;
    }

    /// ★#414 项三：**某来源中枢**的挂起在途量——挂起绑定来源中枢后，回补的算量/放行判据都只
    /// 看这一个中枢的批次（不再拿别的中枢的挂起当自己的货）。
    pub(super) fn open_units_of(&self, center: CenterId) -> i64 {
        self.batches
            .iter()
            .filter(|b| b.center == center)
            .map(|b| b.units)
            .sum()
    }

    /// ★#383：当前挂起在途量按**卖出时锁定的 sizing 模式**分列 `(阶段一等量, 阶段三等金额)`
    /// ——回补 sizing 的分流依据（题三「旧账按卖出时锁定，中途不变规矩」）。
    /// ★#414 项三：只统计**同来源中枢**的批次（异中枢的挂起不参与本次回补的算量）。
    pub(super) fn open_units_by_mode(&self, center: CenterId) -> (i64, i64) {
        let same = |b: &&SuspensionBatch| b.center == center;
        let earning: i64 = self
            .batches
            .iter()
            .filter(same)
            .filter(|b| b.earning)
            .map(|b| b.units)
            .sum();
        let legacy: i64 = self
            .batches
            .iter()
            .filter(same)
            .filter(|b| !b.earning)
            .map(|b| b.units)
            .sum();
        (legacy, earning)
    }

    /// ★#366：**未闭合减出核销**——移除该来源中枢的全部挂起批次，返回 `(货缺口, 桶实收现金)`
    /// ＝ `(Σ units, Σ cash_booked)`（订正 2026-07-27，评审 §2.4：现金一笔取**桶实收**，
    /// 非 `Σ units·price`——后者在空头侧值与符号双错，见 [`SuspensionBatch::cash_booked`]）。
    /// 两者**分开返回、不相减**（不冲销，见 [`UnclosedReduction`]）；无该中枢批次时返回
    /// `(0, 0)`（无事可核销，非错误）。
    ///
    /// 核销范围 = **同来源中枢**的批次（与 [`Self::cover`] 的「同中枢优先」同一归属口径）
    /// ——终结事件按中枢身份到达（#292 D 裁定），只核销它所指的那个中枢的减出，不连坐其他
    /// 中枢仍在等回补的挂起（那些中枢未死，「挂起继续等」，#366 补充裁定）。
    pub(super) fn write_off(&mut self, center: CenterId) -> (i64, i64) {
        let units: i64 = self
            .batches
            .iter()
            .filter(|b| b.center == center)
            .map(|b| b.units)
            .sum();
        let cash: i64 = self
            .batches
            .iter()
            .filter(|b| b.center == center)
            .map(|b| b.cash_booked)
            .sum();
        self.batches.retain(|b| b.center != center);
        (units, cash)
    }

    /// ★#441（ADR 补充十二）：**死亡吞挂起核销**——移除**全部**来源中枢的挂起批次，返回
    /// `(货缺口, 桶实收现金, 被吞的来源中枢个数)`。与 [`Self::write_off`] 同口径同算料，只是
    /// 范围从「单个中枢」放大到「整本账」：campaign 死＝主仓全平，「回补进主仓」对**所有**
    /// 中枢的挂起同时灭失，不能只核销其中一个而把其余留成无主欠账。
    ///
    /// 无批次时返回 `(0, 0, 0)`（无事可核销，非错误）。
    /// ★#719（L19②）：返回值第三项由中枢**个数**扩为逐（中枢）核销明细
    /// `Vec<(CenterId, units_gap, cash_booked)>`——同中枢多批次先按到达序聚合（每中枢一行）。
    /// 个数 = `breakdown.len()`。witness 桶规格不变（仍只分侧，#441 复审订正不动），本明细只
    /// 服务观测门 dump（`THETA_DEATH_WO_DUMP`，见 `OscillationCampaign::close` 调用侧）。
    pub(super) fn write_off_all(&mut self) -> (i64, i64, Vec<(CenterId, i64, i64)>) {
        let units: i64 = self.batches.iter().map(|b| b.units).sum();
        let cash: i64 = self.batches.iter().map(|b| b.cash_booked).sum();
        // 去重按**相等性**逐个查（批次按到达序、同中枢可不相邻 ⟹ `dedup` 只去相邻会多计）；
        // 一个 campaign 同时挂着的中枢数极少，线性查找足够且不引入排序/哈希序依赖。
        let mut centers: Vec<CenterId> = Vec::new();
        for b in &self.batches {
            if !centers.contains(&b.center) {
                centers.push(b.center);
            }
        }
        let breakdown: Vec<(CenterId, i64, i64)> = centers
            .iter()
            .map(|&cid| {
                let u: i64 = self
                    .batches
                    .iter()
                    .filter(|b| b.center == cid)
                    .map(|b| b.units)
                    .sum();
                let c: i64 = self
                    .batches
                    .iter()
                    .filter(|b| b.center == cid)
                    .map(|b| b.cash_booked)
                    .sum();
                (cid, u, c)
            })
            .collect();
        self.batches.clear();
        (units, cash, breakdown)
    }

    /// `Replenish` 收口：**只冲抵同来源中枢**的批次（按到达序）。返回逐批冲抵明细（观测证据）；
    /// 冲抵完的批次移除。
    ///
    /// ★★#414 项三（ADR 补充十一，2026-07-27 用户裁定）：**禁异中枢回补静默冲抵**。旧口径是
    /// 「同中枢优先、**余按时间序**冲抵其余中枢」——余量那一段等于拿这次回补去把**别的**中枢
    /// 的作废挂起悄悄抹平（wf8 产物级证据 `cover_by_side ("long","other_center"):3`），比「装
    /// 没发生」更隐蔽：账上那笔未闭合减出既没核销、也没呈报，就消失了。现改为**只冲抵同中枢**
    /// ——挂起绑定来源中枢（#380 归属标签），出口只有两个：同中枢回补，或原中枢三类点清算
    /// （三买回补 / 三卖核销，#366 两终局）。
    ///
    /// `units` 由调用方保证 ≤ 该中枢的 [`Self::open_units_of`]（生产侧 `replenish_plan` 现按
    /// 同中枢算量，短差桶的 [`ShortDiffViolation::OverReplenish`] 是同一约束的记账层防线）
    /// ——若仍有余量未冲抵，照实返回已冲抵部分，不静默造批次、更不外溢到别的中枢。
    pub(super) fn cover(&mut self, center: CenterId, units: i64) -> Vec<CoverAssignment> {
        let mut remaining = units;
        let mut assignments = Vec::new();
        for b in self.batches.iter_mut() {
            if remaining <= 0 {
                break;
            }
            if b.center != center {
                continue; // ★#414：异中枢批次一律不动（旧口径在此外溢）。
            }
            let take = remaining.min(b.units);
            if take <= 0 {
                continue;
            }
            b.units -= take;
            remaining -= take;
            // `same_center` 恒 true（异中枢已在上面跳过）——字段保留是为让 witness 的
            // `("侧","other_center")` 桶继续存在并**恒 0**：它现在是本条纪律的警报读数
            // （非 0 = 中枢绑定被改坏），比删桶更能防回归。
            assignments.push(CoverAssignment {
                center: b.center,
                units: take,
                seq: b.seq,
                same_center: true,
            });
        }
        self.batches.retain(|b| b.units > 0);
        assignments
    }
}

/// ★#366（补充裁定 2026-07-27）：一次「未闭合减出」核销的呈报——**三卖终结**（多头侧；空头
/// 侧镜像为三买终结）不回补，挂起核销为两笔**分开呈报、不冲销**的读数：
///
/// - [`Self::units_gap`]（货缺口）：已减出、永不回补的股数——账面上 `holding` 里少掉的那份
///   成本基所对应的货。
/// - [`Self::cash_booked`]（桶实收现金）：这些减出腿当时**实际记进**
///   [`short_diff_bucket::ShortDiffBucket::realized_cash`] 的金额之和。
///
/// ★口径订正（2026-07-27，评审 §2.4）：本笔旧名 `cash_surplus`（现金盈余）、旧口径
/// `Σ units·price`（「卖出成交额」）**在空头侧值与符号双错**，现改为「桶实收现金」并逐批取
/// 桶增量（[`SuspensionBatch::cash_booked`]）。两侧同名不同义，按 ADR 补充九分列如下：
///
/// - **多头侧**：减=卖出，成交腿是现金**流入** `units·price`；桶实收 = `units·avg_cost`
///   （在险成本基 holding→free）+ `units·(price−avg_cost)`（已实现盈亏）= `units·price`。
///   两者数值恰好重合，旧口径在这一侧成立。
/// - **空头侧**：减=回补空头（买回），成交腿是现金**支出** `units·price`——叫「现金盈余」
///   「卖出成交额」在经济语义上是反的；桶实收 = `units·avg_cost` + `units·(avg_cost−price)`
///   = `units·(2·avg_cost−price)`，与 `units·price` 不等（旧口径按后者记，数值也错）。
///
/// 因此本字段的符号锚是**桶的现金增量**（恒为「这笔减出让桶多收了多少」），不是任一侧的成交
/// 额；成交腿的方向差异由上面两行分列声明承担，不靠字段名暗示。
///
/// 两笔**不相减**：净额是「高抛躲过下跌的真实盈亏」，只有等价格重新有定义（新中枢/新买点）
/// 才谈得上，此刻相减等于用当前价给未回补的货记一个虚拟成交＝装没发生。本类型只呈报，不裁决。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnclosedReduction {
    pub level: u32,
    pub side: VoiceSide,
    pub center: CenterId,
    /// 货缺口（核销掉的挂起在途股数，恒 >0——为 0 时不产出本记录）。
    pub units_gap: i64,
    /// 桶实收现金（这些减出腿记进 `realized_cash` 的金额之和，不与货缺口冲销；两侧口径见类型
    /// 文档的分列声明）。
    pub cash_booked: i64,
}

/// ★#383：一次 `Replenish` 的分模式 sizing 方案（[`OscillationCampaign::replenish_plan`] 产出）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplenishPlan {
    /// 冲抵挂起在途量的收口股数（阶段一等量族 + 阶段三等金额族买得起的那部分）。
    pub(super) close_units: i64,
    /// 超出挂起在途量、用桶现金新买的**净增股数**（阶段三挣股数；等量族恒 0）。
    pub(super) extra_units: i64,
    /// 本次从等金额现金池中扣除的金额（`bought · price`；零头留池）。
    pub(super) pool_spent: i64,
    /// 本次等金额腿**实际买到**的股数（`floor(池÷价)`；`0`=池买不起一股 ⟹ 等金额腿未成交）
    /// ——witness「等金额回补次数」只数真成交的那些，不把「只收口了等量族」的回补计进去。
    pub(super) earning_bought: i64,
}

impl ReplenishPlan {
    /// `Reduce` 分支的惰性方案（回补侧字段全 0——高抛侧不受阶段影响，补充八题一）。
    pub(super) fn inert() -> Self {
        Self {
            close_units: 0,
            extra_units: 0,
            pool_spent: 0,
            earning_bought: 0,
        }
    }
}

/// ★#441（ADR 补充十二，2026-07-27 用户裁定）：**campaign 死亡吞挂起**的核销呈报。
///
/// campaign 是对主仓的短差账——主仓全平（持仓→空仓）即「回补进主仓」这条出路灭失，其名下
/// 未收口挂起按**三卖同族**清算：核销为「未闭合减出」，货缺口与桶实收现金**分开呈报、不
/// 冲销**（口径与 [`UnclosedReduction`] 逐字相同，两侧同名不同义的分列声明见该类型文档）。
///
/// 与 [`UnclosedReduction`] 的差别只在**范围**：三类点清算按中枢到达 ⟹ 只核销那一个中枢；
/// campaign 死亡 ⟹ 该 campaign 名下**全部**来源中枢的挂起一并核销（不留无主欠账），故本
/// 类型不带 `center` 字段，改记 [`Self::centers`]（被吞的来源中枢**个数**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathWriteOff {
    /// 货缺口（核销掉的挂起在途股数，恒 >0——为 0 时不产出本记录）。
    pub units_gap: i64,
    /// 桶实收现金（这些减出腿记进 `realized_cash` 的金额之和，**不与货缺口冲销**）。
    pub cash_booked: i64,
    /// ★#719（L19①）：短差桶拒绝了取自归属账的在途量核销（`write_off_unclosed` 返回 Err）。
    /// 结构性不可达（`units_gap` 与桶标量逐笔恒等，上方 `debug_assert` 守）——**true = 记账
    /// 错误**，witness 侧落 `other_violation_count` + `other_violation_by_kind` 的
    /// `death_write_off_bucket_rejected_unreachable` 警报桶（对齐 `cash_unsound_free_negative_
    /// unreachable` 既有先例）。
    pub bucket_rejected: bool,
    /// 被一并核销的来源中枢个数（≥1）——死亡吞的是整本账，非单个中枢。
    ///
    /// **类型层归属信息，当前不进 witness**（#441 复审订正）：无生产消费者，不落任何桶、不进
    /// 任何 dump，只被单测断言读（跨中枢核销断言 `centers: 2`）。产物级读得出「吞了多少股」，
    /// 读不出「吞了几个中枢」；按 (级别, 中枢) 分桶的加维留给 #442 统一定规格。
    pub centers: usize,
}
