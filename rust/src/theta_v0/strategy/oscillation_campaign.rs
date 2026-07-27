//! 每仓 campaign（SPEC #287 T4，issue #294）：把 #293/#354 的 `ShortDiffAccount` + TW 桥升格为
//! **每仓各一 campaign** 的完整生命周期——开仓生（`notional_in`=本仓成本基）→ 短差冲减成本
//! （sizing=当时持仓 1/3，#348 裁定，可复检）→ 到 0 转移（`RecoverCapital`，复用
//! `closed_loop::transition::stage_progression` 单一来源，#124 裁定4）→ 全平死（`ClearCampaign`，
//! 挂起随死）。
//!
//! ## 分工边界（两台阶段机分立，SPEC #287 决议）
//!
//! - **结构阶段机**（[`center_oscillation_trade::CenterOscillationBook`]，#274/#292）：管「许不许
//!   做短差」——中枢生死驱动挂起开合，不读本模块任何状态。
//! - **经济阶段机**（本模块 [`OscillationCampaign`]）：管「本金收到哪」——消费短差盈亏桶
//!   （[`short_diff_bucket::ShortDiffAccount`]）产出的 TW 事件，推进 `TStage`。两者互不越权：
//!   本模块不读中枢生死，`CenterOscillationBook` 不读 `TwState`。短差盈亏桶是唯一桥。
//!
//! ## 粒度：每仓各一（SPEC #287 决议2，issue #294 验收①②）
//!
//! 「仓」=本级别的核心持仓（与 [`center_oscillation_trade::CenterOscillationBook`] 同粒度，
//! 每级别一台）——不按中枢身份分（同一级别可能历经多个中枢的生灭，仓位延续），亦不按窗口
//! 聚合（窗口级汇总只是派生读数，见 [`CampaignBook`] 文档）。生命周期：
//! - **开仓生**：本仓成本基快照从「空仓」（`units()==0`）变为「有持仓」（`units()>0`）——
//!   campaign 开局注资 `notional_in = holding = cost_basis()`（缠师口径「成本入账」）。
//! - **全平死**：本仓成本基快照回到「空仓」——`TwEvent::ClearCampaign`（挂起随死：此时短差
//!   盈亏桶若仍有挂起在途量，不强求先收口，承诺随仓位一起作废——持仓已不存在，往返无从谈起）。
//!
//! ## 到 0 转移（issue #294 验收③，#124 裁定4：单一来源）
//!
//! `RecoverCapital`/`EnterEarning` 的派发**不重新实现判据**——委托
//! [`closed_loop::transition::stage_progression`]（生产 I_Θ 组合层与本模块共用同一算子，不得
//! 镜像重写）。κ=0 基线（[`RiskPolicy::baseline`]，M7 冻结口径，PDF §10 canonical 最小规范）。
//!
//! ## P2-D（issue #294 范围⑤）：TW/R 双账入口对齐
//!
//! [`short_diff_bucket::ShortDiffAccount::record_and_apply_dual`]（本票新增，[`short_diff_bucket`]
//! 同文件追加方法——不改动 #293/#354 既有 `record_and_apply` 签名/行为，纯新增）把同一笔
//! `TwEvent::Realize(d)` 镜像进 `LedgerEvent::Realize(d)`，对齐
//! `closed_loop::transition::transition_adapter` 的双账本写回节奏（同一次成交的利润分量，TW 账本
//! `free` 与 R=Π-A-W 账本 `pi` 各自独立入账，两账本不同构 #90/674号，不混称）。短差场景无
//! `Allocate`（不改变持仓资本化额——本仓成本基本体不动，修7）——故只镜像 `Realize` 分量。
//!
//! ## High-2（issue #294 范围⑥）：快照/账本接口形状裁定
//!
//! 单一快照类型 [`short_diff_bucket::CoreCostBasisSnapshot`] 同时驱动两处消费：
//! [`CampaignBook::sync_position`]（开仓/全平生死判据）与
//! [`OscillationCampaign::reduce_units`]（sizing 现算派生）——不存在第二种快照形状。本裁定钉死
//! 「一个快照接口，两个消费点，零派生副本」，防止未来某一消费点悄悄另立一套字段子集导致两处
//! 读到不一致的持仓视图（同构 #354 P1-A `avg_cost` 结构性消解的精神：单一只读源，禁两份重算）。

use super::super::classifier::center_lifecycle::CenterId;
use super::super::closed_loop::state::RiskMode;
use super::super::closed_loop::transition::{cash_sound_gate, stage_progression, TransitionError};
use super::center_oscillation_trade::{CenterOscillationAction, SuspensionTerminationSource, TriggerError};
use super::ledger::{tw_step, LedgerComp, RiskPolicy, TwEvent, TwState};
use super::short_diff_bucket::{CoreCostBasisSnapshot, ShortDiffAccount, ShortDiffViolation};
use super::voice::VoiceSide;
use std::collections::BTreeMap;

/// 本模块 typed 拒绝（无静默兜底，与 [`ShortDiffViolation`]/[`TransitionError`] 同规格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignViolation {
    /// 短差盈亏桶层拒绝（[`ShortDiffAccount::record_and_apply_dual`] 透传）。
    ShortDiff(ShortDiffViolation),
    /// 阶段推进事件（`RecoverCapital`/`EnterEarning`）被 closed_loop 通道拒绝——生产路径
    /// （`stage_progression` 只在 barrier 过关/free 足额时派事件）恒不触发，此分支覆盖外部
    /// 注入/边界态防御性核验。
    StageTransition(TransitionError),
    /// 本级尚无开局 campaign（[`CampaignBook::sync_position`] 未见过 units>0 的快照）——
    /// 对空仓级别调用动作是接线错误，不静默创建幽灵 campaign。
    NoActiveCampaign,
    /// sizing 现算派生（当时持仓 1/3，整数除法）落到 0 或以下——当场拒绝，不静默钳制为 1
    /// （持仓过小时「1/3」本就无意义，照实拒绝优于编造最小单位）。
    SizingRoundsToZero { held: i64 },
    /// ★#380 项四（ADR 补充七「挂起归属可观测」，#367 项三 A 裁）：来源中枢后续买点触发
    /// `Replenish`，但**货已满**（挂起在途量=0，无货可回补）——照实记「触发但货满」，不与
    /// [`Self::SizingRoundsToZero`]（sizing 取整到 0，真正的持仓过小场景）混计：前者是「结构
    /// 说回补、仓位本就满」的预期经济场景，后者是记账层的量纲异常。该信号对主策略的加仓语义
    /// 不受影响（本模块只观测，不干预）。
    ReplenishWhileFull,
    /// ★#383（ADR 补充八题二）：**阶段三唯一硬门**——等金额回补腿落账后桶现金为负。
    ///
    /// 「阶段一看住股，阶段三看住钱」：阶段三 sizing=`floor(桶现金÷回补价)`，取整已保证
    /// `回补金额 ≤ 可用现金` ⟹ 桶现金恒不为负；真为负即 sizing/记账被改坏，当场拒绝、整笔不落账
    /// （生产路径恒不触发，同 [`CampaignViolation::StageTransition`] 的防御性核验定性）。
    ///
    /// **有效域**（本票裁量，ADR 补充八未逐字覆盖，如实标注）：两条现金口径按**阶段**分域，
    /// 非按批次分域——
    /// - 阶段一/二（`earning_active()==false`）：#380 项一口径不变，`free<0` 的真实亏损往返
    ///   **放行入账**（旧口径整笔回滚会让亏钱的往返在账本里根本不存在）；
    /// - 阶段三（`earning_active()==true`）：本门生效，**每一笔**（含题三「按卖出时锁定」仍走
    ///   等量口径的旧账收口腿）都不得让桶现金转负——补充八题二把它定为阶段三**唯一**硬门，
    ///   按阶段整体生效比按批次分流更贴「阶段一看住股，阶段三看住钱」的原话。
    ///   代价：阶段三里一笔亏损的旧账收口会被整笔拒绝、挂起继续挂着（违规显式失败，不静默）。
    EarningCashUnsound { free: i64 },
    /// ★#383：阶段三等金额回补触发，但**现金池买不起一股**（`pool < price`，取整到 0）且无
    /// 阶段一锁定批次可等量收口——预期经济场景（回补价远高于卖出价 / 零头尚未累计够），
    /// 照实分流计数，不与 [`Self::SizingRoundsToZero`]（持仓过小的量纲异常）或
    /// [`Self::ReplenishWhileFull`]（货本就满）混计。挂起继续挂着，不静默补齐。
    EarningSizingRoundsToZero { pool: i64, price: i64 },
}

impl From<ShortDiffViolation> for CampaignViolation {
    fn from(v: ShortDiffViolation) -> Self {
        CampaignViolation::ShortDiff(v)
    }
}

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
    fn push(&mut self, center: CenterId, units: i64, cash_booked: i64, earning: bool) {
        self.batches.push(SuspensionBatch {
            center,
            units,
            seq: self.next_seq,
            cash_booked,
            earning,
        });
        self.next_seq += 1;
    }

    /// ★#383：当前挂起在途量按**卖出时锁定的 sizing 模式**分列 `(阶段一等量, 阶段三等金额)`
    /// ——回补 sizing 的分流依据（题三「旧账按卖出时锁定，中途不变规矩」）。
    fn open_units_by_mode(&self) -> (i64, i64) {
        let earning: i64 = self.batches.iter().filter(|b| b.earning).map(|b| b.units).sum();
        let legacy: i64 = self.batches.iter().filter(|b| !b.earning).map(|b| b.units).sum();
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
    fn write_off(&mut self, center: CenterId) -> (i64, i64) {
        let units: i64 = self.batches.iter().filter(|b| b.center == center).map(|b| b.units).sum();
        let cash: i64 =
            self.batches.iter().filter(|b| b.center == center).map(|b| b.cash_booked).sum();
        self.batches.retain(|b| b.center != center);
        (units, cash)
    }

    /// `Replenish` 收口：**同中枢优先**（按到达序遍历该中枢的批次），余量再按**时间序**冲抵
    /// 其余中枢的批次。返回逐批冲抵明细（观测证据）；冲抵完的批次移除。
    ///
    /// `units` 由调用方保证 ≤ [`Self::open_units`]（生产侧 `replenish_plan` 的 `close_units` 恰取全部待收口量，短差桶
    /// 的 [`ShortDiffViolation::OverReplenish`] 是同一约束的记账层防线）——若仍有余量未冲抵，
    /// 照实返回已冲抵部分，不静默造批次。
    fn cover(&mut self, center: CenterId, units: i64) -> Vec<CoverAssignment> {
        let mut remaining = units;
        let mut assignments = Vec::new();
        for same_center in [true, false] {
            if remaining <= 0 {
                break;
            }
            for b in self.batches.iter_mut() {
                if remaining <= 0 {
                    break;
                }
                if (b.center == center) != same_center {
                    continue;
                }
                let take = remaining.min(b.units);
                if take <= 0 {
                    continue;
                }
                b.units -= take;
                remaining -= take;
                assignments.push(CoverAssignment { center: b.center, units: take, seq: b.seq, same_center });
            }
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
struct ReplenishPlan {
    /// 冲抵挂起在途量的收口股数（阶段一等量族 + 阶段三等金额族买得起的那部分）。
    close_units: i64,
    /// 超出挂起在途量、用桶现金新买的**净增股数**（阶段三挣股数；等量族恒 0）。
    extra_units: i64,
    /// 本次从等金额现金池中扣除的金额（`bought · price`；零头留池）。
    pool_spent: i64,
    /// 本次等金额腿**实际买到**的股数（`floor(池÷价)`；`0`=池买不起一股 ⟹ 等金额腿未成交）
    /// ——witness「等金额回补次数」只数真成交的那些，不把「只收口了等量族」的回补计进去。
    earning_bought: i64,
}

impl ReplenishPlan {
    /// `Reduce` 分支的惰性方案（回补侧字段全 0——高抛侧不受阶段影响，补充八题一）。
    fn inert() -> Self {
        Self { close_units: 0, extra_units: 0, pool_spent: 0, earning_bought: 0 }
    }
}

/// campaign 生命周期事件（[`CampaignBook::sync_position`] 产出，观测/witness 用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignLifecycleEvent {
    /// 开仓生：本仓成本基快照从空仓变为有持仓。★#381：`side`=本 campaign 的持仓侧。
    Opened { level: u32, side: VoiceSide, notional_in: i64 },
    /// 全平死：本仓成本基快照回到空仓——`suspended_units_forfeited`=死亡时短差盈亏桶尚挂起
    /// 的在途量（挂起随死，非 0 时是「未及收口即随仓终结」的照实记录，非违规）。
    Died { level: u32, side: VoiceSide, suspended_units_forfeited: i64 },
}

/// 单仓 campaign（[`TwState`] + [`LedgerComp`] 双账本并置 + [`ShortDiffAccount`] 短差桥）。
///
/// 不可变模式（coding-style）：[`apply_action`](Self::apply_action) 返回新实例，不就地修改；
/// 生死进出由 [`CampaignBook`] 持有/移除整份实例承载（campaign 边界=实例边界）。
/// ★#380：`short_diff`（含开局冻结快照）仍是 sizing 基准；新增 `current_units`（当时真实持仓，
/// 逐 bar 由 [`CampaignBook::sync_position`] 刷新，项二防线读当前的唯一来源）、`opened_bar`
/// （开局 bar，项三「开局以来 bar 数」的基准）、`suspension`（挂起归属账，项四）。
/// `suspension` 含 `Vec` ⟹ 本类型不再 `Copy`（仅 `Clone`），生死进出语义不变。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OscillationCampaign {
    tw: TwState,
    ledger: LedgerComp,
    short_diff: ShortDiffAccount,
    /// 当时真实持仓（分侧现值，生产侧=`balance_side(Core{level}, Long)`）——**只**供防线校验，
    /// 不参与 sizing（sizing 恒读开局冻结快照，ADR 补充七 #367 项一 C 裁）。
    current_units: i64,
    /// campaign 开局 bar（项三 witness 的「开局以来 bar 数」基准）。
    opened_bar: usize,
    suspension: SuspensionAttribution,
    /// ★#381：本 campaign 的持仓侧——空头侧短差为多头镜像（减=回补空头、补=加回空头），
    /// 镜像的记账实现在 [`ShortDiffAccount`]（同侧构造，一本账不跨侧）。
    side: VoiceSide,
    /// ★#383：本 campaign 的 **bar 时钟**——由 [`CampaignBook::sync_position`] 逐 bar 刷新
    /// （生产侧 `drive_campaign_wiring` 对每个 (级别, 侧) 每 bar 必调一次，且在消费本 bar 动作
    /// **之前**）。`apply_action` 不另收 bar 入参，一切「当前 bar」读数取本字段——单一时钟源，
    /// 避免同一 bar 内两处各传一次 bar 而不一致。
    last_sync_bar: usize,
    /// ★#383（ADR 补充八题三）：派发 `EnterEarning` 的那根 bar（`None`=尚未进阶段三）。
    /// **次 bar 换模式**：`last_sync_bar > earning_event_bar` 才生效（事件 bar 收盘后才确认，
    /// 同 bar 换会让同一根 bar 用两本尺）——判据见 [`Self::earning_active`]。
    earning_event_bar: Option<usize>,
    /// ★#383（补充八题一「零头留桶累计」）：阶段三**等金额回补的现金池**——阶段三锁定的
    /// `Reduce` 把该笔桶实收现金记入本池，等金额回补按 `floor(池÷回补价)` 取股数并从池中扣
    /// `股数·回补价`，取整零头**留在池里累计进下次**。
    ///
    /// 与 [`short_diff_bucket::ShortDiffBucket::realized_cash`] 的关系同 `suspension` 与
    /// `open_units`：桶记标量总现金（记账口径），本池只是其中「阶段三待回补」那一份的**算量
    /// 基准**（sizing 口径），不另立第二套记账。
    earning_pool: i64,
}

impl OscillationCampaign {
    /// 开仓生（[`CampaignBook::sync_position`] 唯一构造点）：`notional_in = holding =
    /// snapshot.cost_basis()`（缠师口径「成本入账」——本仓已投入的成本即本 campaign 退本金目标）。
    /// `free=0`（尚未任何短差冲减，现金积累从 0 起步，「现金积累=成本」口径的起点）。
    fn open(snapshot: CoreCostBasisSnapshot, bar: usize, side: VoiceSide) -> Self {
        Self {
            tw: TwState {
                holding: snapshot.cost_basis(),
                notional_in: snapshot.cost_basis(),
                ..TwState::initial()
            },
            ledger: LedgerComp::initial(snapshot.cost_basis()),
            short_diff: ShortDiffAccount::new_side(snapshot, side),
            current_units: snapshot.units(),
            opened_bar: bar,
            suspension: SuspensionAttribution::default(),
            side,
            last_sync_bar: bar,
            earning_event_bar: None,
            earning_pool: 0,
        }
    }

    /// ★#383（补充八题三）：阶段三等金额 sizing 是否**已生效**——`EnterEarning` 的**次 bar**
    /// 起为真（同 bar 不换：事件 bar 收盘后才确认，与「取数时点=前 bar 收盘」既有口径一致）。
    pub fn earning_active(&self) -> bool {
        self.earning_event_bar.is_some_and(|b| self.last_sync_bar > b)
    }

    /// 派发 `EnterEarning` 的那根 bar（`None`=尚未进阶段三；观测/见证读）。
    pub fn earning_event_bar(&self) -> Option<usize> {
        self.earning_event_bar
    }

    /// 阶段三等金额回补现金池（含累计零头；观测读数，见字段文档）。
    pub fn earning_pool(&self) -> i64 {
        self.earning_pool
    }

    /// 本 campaign 的持仓侧（#381 只读观测）。
    pub fn side(&self) -> VoiceSide {
        self.side
    }

    /// campaign 开局 bar（项三 witness 读）。
    pub fn opened_bar(&self) -> usize {
        self.opened_bar
    }

    /// 开局以来 bar 数（`bar` 早于开局时诚实返回 0，不构造负数）。
    pub fn bars_since_open(&self, bar: usize) -> usize {
        bar.saturating_sub(self.opened_bar)
    }

    /// 当时真实持仓（项二防线读的现值，逐 bar 由 `sync_position` 刷新）。
    pub fn current_units(&self) -> i64 {
        self.current_units
    }

    /// 挂起归属账（项四只读观测）。
    pub fn suspension(&self) -> &SuspensionAttribution {
        &self.suspension
    }

    pub fn tw(&self) -> TwState {
        self.tw
    }

    pub fn ledger(&self) -> LedgerComp {
        self.ledger
    }

    pub fn short_diff(&self) -> ShortDiffAccount {
        self.short_diff
    }

    /// sizing=当时持仓 1/3（issue #348 用户裁定「阶段一降成本 sizing=固定 1/3」，整数除法，
    /// ★可复检——原型口径零发明，如有原文/数据依据可升级）。
    fn reduce_units(&self) -> i64 {
        self.short_diff.cost_basis().units() / 3
    }

    /// ★#383（ADR 补充八题一/题三）：一次 `Replenish` 的**分模式 sizing 方案**。
    ///
    /// 本方法取代 #348 的 `replenish_units`（原口径「全额买回当前挂起在途量」= 本方法在
    /// 「无阶段三批次」时的取值，回归锁见测试 `replenish_plan_in_stage_one_is_full_open_units`）。
    ///
    /// 挂起批次按**卖出时锁定**的模式分两族（题三「中途不变规矩」）：
    /// - **阶段一锁定族**（`legacy_open`）：等量买回，逐字节沿用 #348 口径（恒仓约束硬——
    ///   同股数进出，每次触发即整轮收口）；
    /// - **阶段三锁定族**（`earning_open`）：等金额买回——`bought = floor(现金池 ÷ 回补价)`
    ///   （题一：抛出所得现金全额回补、现金不透支、零头留桶累计）。
    ///   `bought > earning_open` ⟹ 多出来的就是**净增股数**（挣股数，[`Self::extra_units`]）；
    ///   `bought < earning_open`（回补价高于卖出价，同一笔钱买不回同样多的货）⟹ 只收口买得起
    ///   的那部分，其余照实继续挂着，不静默补齐。
    ///
    /// 现金池为 0 或该族无挂起（`earning_open == 0`）⟹ 等金额腿整体不成立（`bought = 0`），
    /// 避免在「货已满」时凭空拿池里的零头买股（`ReplenishWhileFull` 语义不被绕过）。
    ///
    /// **已知口径边界（如实标注）**：本方法按模式**分族算量**，而挂起冲抵
    /// （[`SuspensionAttribution::cover`]）按「同中枢优先、余按时间序」**不看模式**——当
    /// `bought < earning_open`（买不回同样多）导致只能部分收口时，实际被冲抵的批次可能落在
    /// 另一族。总量与现金池扣减恒正确（池是标量，非按批次挂账），下一次回补按彼时**实际
    /// 剩余**的批次重算，故不会漂移；受影响的只是「哪一批先收口」这一归属观测。
    /// 现金池不随批次核销（#366 `write_off_unclosed`）归还——核销掉的减出腿其现金已实收进桶，
    /// 池里的那份留着给后续等金额回补用（不冲销，同 [`UnclosedReduction`] 精神）。
    fn replenish_plan(&self, price: i64) -> ReplenishPlan {
        let (legacy_open, earning_open) = self.suspension.open_units_by_mode();
        let bought = if earning_open > 0 && price > 0 {
            (self.earning_pool / price).max(0)
        } else {
            0
        };
        let earning_close = bought.min(earning_open);
        ReplenishPlan {
            close_units: legacy_open + earning_close,
            extra_units: bought - earning_close,
            pool_spent: bought * price,
            earning_bought: bought,
        }
    }

    /// 单次动作应用：sizing 现算派生 → 短差盈亏桶记账 + TW/R 双账入口（P2-D）→ 阶段推进
    /// （barrier-gated，单一来源 [`stage_progression`]）。
    ///
    /// 不可变：返回 `(新 campaign 实例, 产出)`，调用方（[`CampaignBook`]）负责替换/持有新实例。
    pub fn apply_action(
        &self,
        action: CenterOscillationAction,
        price: i64,
        risk_mode: RiskMode,
        source_center: CenterId,
    ) -> Result<(Self, CampaignOutcome), CampaignViolation> {
        // ★#383（补充八题一）：高抛侧维持持仓 1/3（与阶段一同尺，阶段变化只改回补侧）；
        // 回补侧按挂起批次**卖出时锁定**的模式分流（[`Self::replenish_plan`]）。
        let (units, plan) = match action {
            CenterOscillationAction::Reduce => (self.reduce_units(), ReplenishPlan::inert()),
            CenterOscillationAction::Replenish => {
                let plan = self.replenish_plan(price);
                (plan.close_units, plan)
            }
        };
        // ★#380 项四：「触发但货满」——买点触发回补但挂起在途量=0（仓位本就满），照实分流为
        // 独立 typed 拒绝，不混入 sizing 取整异常。
        if matches!(action, CenterOscillationAction::Replenish) {
            if self.short_diff.bucket().open_units() == 0 {
                return Err(CampaignViolation::ReplenishWhileFull);
            }
            // ★#383：有挂起、但等金额腿的现金池买不起一股且无等量族可收口——照实分流。
            if units == 0 && plan.extra_units == 0 {
                return Err(CampaignViolation::EarningSizingRoundsToZero {
                    pool: self.earning_pool,
                    price,
                });
            }
        }
        // 高抛侧 1/3 取整到 0 的量纲异常（回补侧的三种零量场景已在上方各自分流，不复用本桶）。
        if matches!(action, CenterOscillationAction::Reduce) && units <= 0 {
            return Err(CampaignViolation::SizingRoundsToZero {
                held: self.short_diff.cost_basis().units(),
            });
        }
        let mut short_diff = self.short_diff;
        // ★#366 订正（评审 §2.4）：本笔**桶实收现金**取记账前后桶 `realized_cash` 之差——
        // 唯一算料来自桶自身，本模块不重算第二套公式（多头 `units·price` / 空头
        // `units·(2·avg_cost−price)` 的两侧差异因此自动正确，见 `SuspensionBatch::cash_booked`）。
        let cash_before = short_diff.bucket().realized_cash();
        // ★#380 项二：sizing 基准=开局冻结快照（上方 `reduce_units`），防线基准=当时真实持仓
        // （`self.current_units`，逐 bar 由 `sync_position` 刷新）——两个基准分开传。
        let (tw_after_action, ledger_after_action) = short_diff.record_and_apply_dual(
            &self.tw,
            &self.ledger,
            action,
            units,
            plan.extra_units,
            price,
            self.current_units,
        )?;
        // ★#383（补充八题二）：阶段三唯一硬门——等金额腿落账后桶现金不得为负。放在记账**之后**
        // 读最终态、拒绝时整笔不落账（`self` 不可变，`short_diff` 是本地副本，返回 Err 即丢弃）。
        // 有效域见 [`CampaignViolation::EarningCashUnsound`]：**阶段三生效后的每一笔**（含旧账
        // 收口腿）都受本门约束，阶段一/二 仍走 #380 项一的亏损放行入账——两门按阶段分域。
        if self.earning_active() && tw_after_action.free < 0 {
            return Err(CampaignViolation::EarningCashUnsound { free: tw_after_action.free });
        }
        let cash_delta = short_diff.bucket().realized_cash() - cash_before;
        // ★#380 项四：记账放行后同步挂起归属账（Reduce 入队带中枢标签；Replenish 同中枢优先
        // 冲抵、余按时间序）——与短差桶标量逐笔同步，不另立第二套记账。
        // ★#383：Reduce 把**卖出时**的 sizing 模式锁进批次；阶段三锁定的减出把桶实收现金记入
        // 等金额现金池，回补时按 `floor(池÷价)` 取量并扣 `股数·价`（零头留池累计）。
        let mut suspension = self.suspension.clone();
        let mut earning_pool = self.earning_pool;
        let cover = match action {
            CenterOscillationAction::Reduce => {
                let earning = self.earning_active();
                suspension.push(source_center, units, cash_delta, earning);
                if earning {
                    earning_pool += cash_delta;
                }
                Vec::new()
            }
            CenterOscillationAction::Replenish => {
                earning_pool -= plan.pool_spent;
                suspension.cover(source_center, units)
            }
        };

        // ★到 0 转移（issue #294 验收③）：委托单一来源 stage_progression（#124 裁定4）——
        // 不镜像重写 barrier 判据。κ=0 基线（M7 冻结口径）。
        let policy = RiskPolicy::baseline();
        let (tw_final, stage_event) = match stage_progression(&policy, &tw_after_action, risk_mode) {
            Some(ev) => {
                if !ev.is_legal_from(&tw_after_action) {
                    return Err(CampaignViolation::StageTransition(TransitionError::Oq9Illegal {
                        event: ev,
                        stage: tw_after_action.stage,
                    }));
                }
                let advanced = tw_step(&tw_after_action, ev);
                let advanced =
                    cash_sound_gate(advanced).map_err(CampaignViolation::StageTransition)?;
                (advanced, Some(ev))
            }
            None => (tw_after_action, None),
        };

        let next = Self {
            tw: tw_final,
            ledger: ledger_after_action,
            short_diff,
            current_units: self.current_units,
            opened_bar: self.opened_bar,
            suspension,
            side: self.side,
            last_sync_bar: self.last_sync_bar,
            // ★#383（题三）：`EnterEarning` 派发即记事件 bar；模式**次 bar** 才生效
            // （[`Self::earning_active`]）——同 bar 不换尺。
            earning_event_bar: match stage_event {
                Some(TwEvent::EnterEarning) => Some(self.last_sync_bar),
                _ => self.earning_event_bar,
            },
            earning_pool,
        };
        let outcome = CampaignOutcome {
            units: units + plan.extra_units,
            tw: tw_final,
            ledger: ledger_after_action,
            stage_event,
            cover,
            // ★#380 项一：本次落账后桶现金为负 ⟹ 亏损往返如实入账（旧口径此笔整笔回滚）。
            loss_accounted: tw_after_action.free < 0,
            earning_replenish: plan.earning_bought > 0,
            earning_units_gained: plan.extra_units,
        };
        Ok((next, outcome))
    }

    /// ★#366（补充裁定 2026-07-27）：**未闭合减出核销**——三卖终局（多头侧；空头侧镜像为
    /// 三买终局）下不回补，把该来源中枢的挂起批次从在途量核销，产出货缺口/桶实收现金两笔分开
    /// 的呈报（[`UnclosedReduction`]）。不构造任何回补动作、不产任何 `TwEvent`（不冲销，见
    /// [`ShortDiffAccount::write_off_unclosed`]）。
    ///
    /// 该中枢无挂起批次 ⟹ `Ok(None)`（无事可核销，非错误——高抛后已自然收口，或该侧本就
    /// 没做过短差）。归属账与桶标量恒等（两者逐笔同步推进），故桶层核销恒不越界；越界即
    /// 记账错误，typed 上报不静默。
    fn write_off_unclosed(
        &self,
        level: u32,
        center: CenterId,
    ) -> Result<(Self, Option<UnclosedReduction>), CampaignViolation> {
        let mut suspension = self.suspension.clone();
        let (units_gap, cash_booked) = suspension.write_off(center);
        if units_gap <= 0 {
            return Ok((self.clone(), None));
        }
        let mut short_diff = self.short_diff;
        short_diff.write_off_unclosed(units_gap)?;
        let next = Self { short_diff, suspension, ..self.clone() };
        Ok((
            next,
            Some(UnclosedReduction {
                level,
                side: self.side,
                center,
                units_gap,
                cash_booked,
            }),
        ))
    }

    /// 全平死：`TwEvent::ClearCampaign`（挂起随死——不核验 `assert_conserved`，全平即终结，
    /// 未收口的挂起在途量随之作废，见 [`CampaignLifecycleEvent::Died`]）。
    ///
    /// `ClearCampaign` 在本模块恒合法：本模块只用 `ShortDiff`/`Realize`（[`TwEvent::is_legal_from`]
    /// 对 `ClearCampaign` 的唯一门槛 `open_legacy_legs==0` 恒满足——本模块从不构造
    /// `OpenShareLeg`/`CloseShareLeg`）。
    fn close(self) -> (TwState, i64) {
        debug_assert!(
            TwEvent::ClearCampaign.is_legal_from(&self.tw),
            "本模块从不用 legacy 腿构造子，ClearCampaign 前置 open_legacy_legs==0 恒满足"
        );
        (tw_step(&self.tw, TwEvent::ClearCampaign), self.short_diff.bucket().open_units())
    }
}

/// 每仓 campaign 账簿（issue #294 验收①②）：BTreeMap 按 level 键——与
/// [`center_oscillation_trade::CenterOscillationBook`]（每级别一台）同粒度、同确定性迭代序理由
/// （#292 H1 先例：`BTreeMap` 非 `HashMap`，避免哈希序跨进程/跨版本不确定）。
///
/// 窗口级汇总（如「本窗口总共几个 campaign 到过 II」）**只是派生读数**——从 `campaigns` 现算
/// （遍历/过滤/计数），本结构不额外维护累计计数器（避免双份状态漂移，同 `state.rs` phase 派生
/// 写回的精神）。
#[derive(Debug, Default)]
pub struct CampaignBook {
    campaigns: BTreeMap<(u32, VoiceSide), OscillationCampaign>,
}

impl CampaignBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// ★#381：键改 `(level, side)`——多空并存时同级两侧各自独立一本 campaign。
    pub fn campaign(&self, level: u32, side: VoiceSide) -> Option<&OscillationCampaign> {
        self.campaigns.get(&(level, side))
    }

    /// 当前存活 campaign 数（诊断/派生读数——非累计维护，见结构体文档）。
    pub fn active_count(&self) -> usize {
        self.campaigns.len()
    }

    /// 同步本级持仓快照 ⟹ 生死判据（High-2 裁定：本方法与 [`OscillationCampaign::reduce_units`]
    /// 共用同一 [`CoreCostBasisSnapshot`] 接口形状，零派生副本）：
    /// - 空仓→有持仓：开仓生（[`CampaignLifecycleEvent::Opened`]）。
    /// - 有持仓→空仓：全平死（[`CampaignLifecycleEvent::Died`]，挂起随死）。
    /// - 其余（仍空/仍持仓）：no-op（`None`）——本模块不处理加仓重算，唯二合法写口在别处
    ///   （#197 G3 既有机制，同 [`short_diff_bucket`] 既有范围边界声明）。
    ///
    /// ★#380 项二：「仍持仓」分支不再是纯 no-op——刷新 campaign 的 `current_units`（当时真实
    /// 持仓，防线校验基准）；开局冻结快照（sizing 基准）与 `notional_in` 均不动。
    /// ★#380 项三：`bar` 入参=当前 bar，开局时记为 campaign 的 `opened_bar`。
    pub fn sync_position(
        &mut self,
        level: u32,
        side: VoiceSide,
        snapshot: CoreCostBasisSnapshot,
        bar: usize,
    ) -> Option<CampaignLifecycleEvent> {
        let key = (level, side);
        let currently_open = self.campaigns.contains_key(&key);
        match (currently_open, snapshot.units() > 0) {
            (false, true) => {
                self.campaigns.insert(key, OscillationCampaign::open(snapshot, bar, side));
                Some(CampaignLifecycleEvent::Opened {
                    level,
                    side,
                    notional_in: snapshot.cost_basis(),
                })
            }
            (true, false) => {
                let campaign = self.campaigns.remove(&key).expect("contains_key 刚核验为 true");
                let (_final_tw, suspended_units_forfeited) = campaign.close();
                Some(CampaignLifecycleEvent::Died { level, side, suspended_units_forfeited })
            }
            (true, true) => {
                // 仍持仓：刷新防线基准（当时真实持仓），不产生生死事件、不动冻结快照。
                // ★#383：同时推进 campaign 的 bar 时钟（`last_sync_bar`）——阶段三「次 bar 换
                // 模式」的唯一时点来源，见 [`OscillationCampaign::earning_active`]。
                if let Some(campaign) = self.campaigns.get_mut(&key) {
                    campaign.current_units = snapshot.units();
                    campaign.last_sync_bar = bar;
                }
                None
            }
            (false, false) => None,
        }
    }

    /// 单次动作应用（透传 [`OscillationCampaign::apply_action`]）——本级须已开局
    /// （[`CampaignViolation::NoActiveCampaign`]：空仓级别调用是接线错误，不静默创建幽灵 campaign）。
    pub fn apply_action(
        &mut self,
        level: u32,
        side: VoiceSide,
        action: CenterOscillationAction,
        price: i64,
        risk_mode: RiskMode,
        source_center: CenterId,
    ) -> Result<CampaignOutcome, CampaignViolation> {
        let key = (level, side);
        let campaign = self.campaigns.get(&key).ok_or(CampaignViolation::NoActiveCampaign)?;
        let (next, outcome) = campaign.apply_action(action, price, risk_mode, source_center)?;
        self.campaigns.insert(key, next);
        Ok(outcome)
    }

    /// ★#366（补充裁定 2026-07-27）：**未闭合减出核销**入口——三卖终局（多头侧；空头侧镜像）
    /// 的清算落点，透传 [`OscillationCampaign::write_off_unclosed`]。
    ///
    /// 返回 `Ok(None)` 有两种诚实情形，均非错误、均不新增 typed 拒绝：① 该 (级别, 侧) 当前
    /// 无 campaign（该侧空仓——结构信号独立于持仓的既有「无门」设计，同
    /// [`CampaignViolation::NoActiveCampaign`] 的「预期经济场景」定性）；② 有 campaign 但该
    /// 来源中枢无挂起批次（高抛后已自然收口）。调用方（`fill.rs::drive_campaign_wiring`）把两
    /// 情形合并计入「无可核销」观测桶，不与真实核销读数混计。
    pub fn write_off_unclosed(
        &mut self,
        level: u32,
        side: VoiceSide,
        center: CenterId,
    ) -> Result<Option<UnclosedReduction>, CampaignViolation> {
        let key = (level, side);
        let Some(campaign) = self.campaigns.get(&key) else { return Ok(None) };
        let (next, reduction) = campaign.write_off_unclosed(level, center)?;
        self.campaigns.insert(key, next);
        Ok(reduction)
    }
}

/// 生产接线观测统计（issue #357 验收③：enabled=true wf8 真实验收——减补动作出现且归属正确、
/// `CenterNotAlive` 丢弃率、挂起归宿、campaign 生死事件均须产物级可见）。
///
/// 门关（`center_oscillation.enabled=false`）⟹ 本结构全程不构造/不喂入，恒 `default()`
/// （诚实空，同 [`super::center_oscillation_trade::CenterOscillationActionRecord`] 先例）——
/// 累计只是**观测读数**，不参与任何裁决/门控，接入/移除不改变生产行为。
#[derive(Debug, Default, Clone)]
pub struct CampaignWiringWitness {
    /// 次级别买卖点触发候选构造尝试总数（`Ok`+`Err` 之和，丢弃率分母）。
    pub trigger_attempts: usize,
    /// `TriggerError::CenterNotAlive` 丢弃数（A 裁定：本级中枢不在场，丢弃率分子）。
    pub dropped_center_not_alive: usize,
    /// 其余触发构造拒绝（`FlatSignal`/`PriceOutsideZone`）丢弃数——与 `CenterNotAlive` 分列，
    /// 不混入同一桶（丢弃成因不同，混计会掩盖「中枢不在场」这一验收关注点）。
    /// ★#366：`PriceOutsideZone` 的**判据**已换（中轴二分 → 离开中枢 ZG/ZD），桶名与归属
    /// 不变——本桶读数在 #366 前后不可跨版本比较（SPEC #386 §6「PriceOutsideZone 桶读数变化
    /// 写入报告」的对照对象）。
    pub dropped_other_trigger: usize,
    /// ★#366：回补侧「中枢不下移」前置过滤（89 课）拒绝数——价格已向下离开中枢、次级别底
    /// 背驰已在，但本级中枢链已下移 ⟹ 不回补。独立分桶（不并入 `dropped_other_trigger`）：
    /// 这是本票新增判据的唯一产物级读数，混入既有桶就无法判定新过滤实际拦了多少。
    pub dropped_center_moved_down: usize,
    /// 减/补动作按 (级别, 持仓侧, 动作) 分桶计数——issue #357 验收「归属正确」的产物级见证。
    /// ★#381：键增持仓侧维（`"long"`/`"short"`）——同一次触发对两侧产出镜像动作，级别+动作
    /// 已不足以定位记账对象。
    pub action_by_level: BTreeMap<(u32, &'static str, &'static str), usize>,
    /// 挂起终结来源分桶（挂起归宿：`BrokenByThirdClassBuy`/`.._Sell`/`Reset`/`Superseded`/
    /// `RebaseVanished` 五源，见 [`super::center_oscillation_trade::SuspensionTerminationSource`]）。
    /// ★#381 关票修复：键增持仓侧维 `(侧, 来源)`——`on_lifecycle_event`/`on_chain_rebase` 现在
    /// 对同一事件**逐侧**各产一条 `SuspensionOutcome`，不分侧则多空并存时计数翻倍且无法归属
    /// （破本 ADR 补充九自立的「两侧不得相加」）。
    pub suspension_by_source: BTreeMap<(&'static str, &'static str), usize>,
    /// campaign 开仓生事件计数（[`CampaignLifecycleEvent::Opened`]）。
    /// ★#381 关票修复：改分侧分桶（键=`"long"`/`"short"`）——事件本身已带 `side`，标量相加是
    /// 跨侧求和，既破「两侧不得相加」，也使多头侧生死零漂移不可逐条核验。
    pub lifecycle_opened: BTreeMap<&'static str, usize>,
    /// campaign 全平死事件计数（[`CampaignLifecycleEvent::Died`]）。★#381：同上，改分侧分桶。
    pub lifecycle_died: BTreeMap<&'static str, usize>,
    /// `apply_action` 遇 [`CampaignViolation::NoActiveCampaign`] 的计数——★非接线错误：
    /// `CenterOscillationBook`（#292 T2）的减/补决策独立于本级 Core 持仓状态（「无门」设计，
    /// 见该模块文档），故结构信号在本级空仓时触发是**真实经济场景**（结构说做、但当前无仓可
    /// 操作），非 bug。真实生产数据（wf8 实测）此项非零属预期读数，报告层照实呈现即可，
    /// 不得断言恒 0（`other_violation_count` 才是真正的接线错误警报）。
    ///
    /// ★#381 桶退役注记：`unsupported_short_position_count`（#357 关票条件 C 落的
    /// 「本级持有空头仓位、但本接线只支持多头 campaign」缺口桶）随空头 campaign 实装**退役**
    /// ——空头侧持仓现在正常开局 `(level, Short)` campaign 并正常记账，不再存在「有仓但认不出」
    /// 这一形态；空头侧的 `NoActiveCampaign` 与多头侧同义（真空仓，预期经济场景），归入
    /// [`Self::no_active_campaign_count`]。★#381：改**分侧**分桶（键=`"long"`/`"short"`）——
    /// 两侧同义但归属不同，混计后多空并存时无法判读是哪一侧空仓。
    pub no_active_campaign_count: BTreeMap<&'static str, usize>,
    /// `apply_action` 遇 [`CampaignViolation::ShortDiff`]`(`[`ShortDiffViolation::ChannelRejected`]`(`
    /// [`TransitionError::CashUnsound`]`{ holding<0, .. }`」的计数——★同样非接线错误，而是
    /// **预算耗尽的真实经济场景**：`OscillationCampaign::reduce_units`/`replenish_plan`
    /// （#294/#348 既有落地代码，本票不改动）按**campaign 开局时冻结的总量**算 sizing（非
    /// 「当前剩余可减仓量」），而 `CampaignBook` 按**级别**（非按中枢）聚合——同级多个中枢各自
    /// 独立触发 `Reduce` 会共享同一份 `holding` 预算；当连续同向触发次数超过冻结总量所能支撑
    /// 的轮次（如原始持仓 3 等分后再遇第 4 次 `Reduce`），`cash_sound_gate` 显式拒绝（`holding`
    /// 将变负）——这正是「违规显式失败」设计纪律的产物级见证（非静默钳制/吸收），资源约束下的
    /// 诚实拒绝，非本票引入的接线缺陷。
    ///
    /// ★issue #357 关票条件 B（拆因）：`resource_exhausted_count`（单桶）曾把
    /// [`TransitionError`] 内层错误下划线丢弃——本路径 OQ-9 恒真（`ShortDiff`/`Realize` 的
    /// `is_legal_from` 恒真），故必为 `CashUnsound`；但 `CashUnsound` 有两条含义完全不同的
    /// 成因：`holding<0`（本字段，连续 Reduce 打穿冻结预算）与 `free<0`
    /// （回补价高于卖出价的亏损往返——★#380 项一后该族改为放行入账，原
    /// `resource_exhausted_free_negative_count` 桶退役，接替它的正面读数见
    /// [`Self::loss_round_trip_accounted_count`]）。
    /// 拆桶后归因才可从读数上直接判定，不再靠推断。★#381：改分侧分桶（同上）。
    pub resource_exhausted_holding_negative_count: BTreeMap<&'static str, usize>,
    /// ★#380 项一（ADR 补充七，#367 项四 A 裁）：**亏损往返如实入账**的计数——`Reduce` 使
    /// `free += units·p_sell`、`Replenish` 使 `free -= units·p_buy`，`p_buy > p_sell`（高抛低吸
    /// 反着做）即桶现金转负。旧口径（#357）此路径被 `cash_sound_gate` 拒绝、整笔回滚，落
    /// `resource_exhausted_free_negative_count` 桶——**该桶本票退役**（新通道
    /// [`super::short_diff_bucket`] 的 `short_diff_cash_gate` 下已不可达；若仍到达即为记账错误，
    /// 归入 `other_violation_by_kind` 的 `cash_unsound_free_negative_unreachable`）。本桶接替
    /// 观测同一现象的**正面**读数：这些往返现在如实进账，`realized_cash` 收负。
    ///
    /// ★#381：改**分侧**分桶——两侧不得相加。严格语义仍是「落账后桶现金为负的动作次数」，
    /// 但空头侧的**单腿**桶现金不等于单笔成交现金（`Realize` 镜像，见
    /// [`super::short_diff_bucket::ShortDiffBucket::realized_cash`] 的口径注记），故两侧读数
    /// 同名不同义，报告层须分列呈现、不得相加。
    pub loss_round_trip_accounted_count: BTreeMap<&'static str, usize>,
    /// ★#380 项四：`Replenish` 触发但**货已满**（挂起在途量=0）的计数——
    /// [`CampaignViolation::ReplenishWhileFull`]，预期经济场景（结构说回补、仓位本就满），
    /// 不与 `other_violation_count`（记账错误警报）混计。★#381：改分侧分桶。
    pub replenish_triggered_but_full_count: BTreeMap<&'static str, usize>,
    /// ★#380 项二：防线（读**当时真实持仓**）拒绝的超卖计数——
    /// [`ShortDiffViolation::UnitsExceedCostBasis`]。按冻结快照算出的 sizing 超过当时真实持仓
    /// （主仓其间被减仓）⟹ 「不卖没有的货」显式拒绝。这是**预期读数**（基准与现值分离的必然
    /// 副产物），非接线错误，故与 `other_violation_count` 分列。★#381：改分侧分桶。
    pub defense_units_exceed_current_holding_count: BTreeMap<&'static str, usize>,
    /// ★#380 项三：阶段推进事件（`RecoverCapital`/`EnterEarning`）计数（见
    /// [`Self::stage_events`] 的逐条明细）。
    pub stage_recover_capital_count: usize,
    pub stage_enter_earning_count: usize,
    /// ★#380 项三：阶段推进事件逐条明细（campaign 标识=级别 + 开局以来 bar 数 + 金额）——
    /// `fill.rs` 旧版 `Ok(_outcome) => {}` 把 `stage_event` 整个丢弃，#368 切换开关因此无物可读。
    pub stage_events: Vec<StageEventRecord>,
    /// ★#383（#383 报告层三项之二）：**等金额回补次数**（分侧）——回补动作中含阶段三锁定批次
    /// 的次数。与 `action_by_level` 的 `replenish` 桶是包含关系（后者含两种口径），分列呈报。
    pub earning_replenish_count: BTreeMap<&'static str, usize>,
    /// ★#383（报告层三项之一，补充八题二「股数不设门 ⟹ 改作观测读数」）：阶段三**累计净增
    /// 股数**（分侧）——每次等金额回补里超出挂起在途量、用桶现金新买的那部分之和。两侧不得相加。
    pub earning_units_gained: BTreeMap<&'static str, i64>,
    /// ★#383（报告层三项之三）：**切换时点读数**——键 `(级别, 侧)`，值 = 等金额 sizing
    /// **生效**的 bar（= `EnterEarning` 事件 bar + 1，题三「次 bar 换模式」）。
    pub earning_mode_switch_bar: BTreeMap<(u32, &'static str), usize>,
    /// ★#383：阶段三硬门（桶现金永不为负）被触发的计数（分侧）——生产路径恒 0
    /// （`floor` 取整已保证不透支），非 0 = sizing/记账被改坏的真正警报。
    pub earning_cash_unsound_count: BTreeMap<&'static str, usize>,
    /// ★#383：等金额回补触发但现金池买不起一股的计数（分侧）——预期经济场景，独立分桶。
    pub earning_sizing_rounds_to_zero_count: BTreeMap<&'static str, usize>,
    /// ★#380 项四：回补冲抵按「同中枢/异中枢」分桶（同中枢优先的产物级见证）。
    /// ★#381：键增持仓侧维——`(侧, "same_center"|"other_center")`，多空并存时冲抵归属可判读
    /// （SPEC #386 §2「witness 呈现归属与冲抵顺序」对两侧对称适用）。
    pub cover_by_side: BTreeMap<(&'static str, &'static str), usize>,
    /// ★#366（补充裁定 2026-07-27）：**未闭合减出**核销的分侧计数（三卖终局条数）。
    pub unclosed_write_off_count: BTreeMap<&'static str, usize>,
    /// ★#366：核销的**货缺口**累计（分侧，单位=股数）——与下方桶实收现金**分列呈报，不相减**
    /// （相减＝冲销＝装没发生，见 [`UnclosedReduction`]）。
    pub unclosed_write_off_units_gap: BTreeMap<&'static str, i64>,
    /// ★#366：核销的**桶实收现金**累计（分侧，单位=现金）——见上，不与货缺口冲销。
    ///
    /// ★口径订正（2026-07-27，评审 §2.4）：本桶旧名 `unclosed_write_off_cash_surplus`、旧口径
    /// `Σ units·price`，空头侧值与符号双错（空头「减」是买回＝现金支出，非「卖出成交额」）。
    /// 现口径 = 各减出腿记进桶 `realized_cash` 的实收增量之和，两侧同名不同义的分列声明见
    /// [`UnclosedReduction`]。**跨版本不可比**：本桶在 #366 订正前后不是同一个量。
    pub unclosed_write_off_cash_booked: BTreeMap<&'static str, i64>,
    /// ★#366：三卖终局到达但**无可核销**的分侧计数（该侧空仓无 campaign，或该中枢本就没有
    /// 挂起批次）——照实计数，不与真实核销读数混计（零读数照实亦是 #384 终验的对照项）。
    pub unclosed_write_off_nothing_to_settle: BTreeMap<&'static str, usize>,
    /// `apply_action` 遇其余 [`CampaignViolation`]（`AvgCostMismatch`/`NonPositiveUnits`/
    /// `OverReplenish`/`UnclosedRoundTrip`/`UnitsExceedCostBasis`/`StageTransition`/
    /// `SizingRoundsToZero`）的计数——诊断用，生产路径正常接线下恒 0（非 0 = 真正的接线/
    /// 记账逻辑错误，与 `no_active_campaign_count`/`resource_exhausted_count` 的「预期读数」
    /// 性质不同，不得混桶）。
    pub other_violation_count: usize,
    /// `other_violation_count` 的细分诊断分桶（变体名标签，如 `sizing_rounds_to_zero`/
    /// `short_diff_units_exceed_cost_basis` 等）——定位非 0 读数的具体成因用。
    /// ★#381：键增持仓侧维 `(侧, 变体名)`。
    pub other_violation_by_kind: BTreeMap<(&'static str, &'static str), usize>,
}

/// ★#381：持仓侧的稳定标签（witness 分桶键/明细用，`&'static str` 与既有动作标签同规格）。
fn side_label(side: VoiceSide) -> &'static str {
    match side {
        VoiceSide::Long => "long",
        VoiceSide::Short => "short",
        VoiceSide::Flat => "flat",
    }
}

/// ★#380 项三：一条阶段推进事件的见证记录（campaign 标识=级别；`bars_since_open`=开局以来
/// bar 数；`amount`=事件金额，`RecoverCapital(w)`=退本金额；`EnterEarning` 无载荷 ⟹ 记 0）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageEventRecord {
    pub level: u32,
    /// ★#381：本事件所属 campaign 的持仓侧。
    pub side: &'static str,
    pub bar: usize,
    pub bars_since_open: usize,
    pub kind: &'static str,
    pub amount: i64,
}

impl CampaignWiringWitness {
    pub fn new() -> Self {
        Self::default()
    }

    /// ★#380 项三：记一次阶段推进事件（`RecoverCapital`/`EnterEarning`）——其余 `TwEvent`
    /// 不是阶段推进事件，不记（`stage_progression` 只派这两种，此处是防御性 match）。
    pub fn record_stage_event(
        &mut self,
        level: u32,
        side: VoiceSide,
        bar: usize,
        bars_since_open: usize,
        event: TwEvent,
    ) {
        let (kind, amount) = match event {
            TwEvent::RecoverCapital(w) => {
                self.stage_recover_capital_count += 1;
                ("recover_capital", w)
            }
            // `EnterEarning` 是无载荷变体（相变本身即事件），金额记 0——不编造数值。
            TwEvent::EnterEarning => {
                self.stage_enter_earning_count += 1;
                // ★#383 报告层「切换时点读数」：等金额 sizing 的**生效** bar = 事件 bar + 1
                // （题三「次 bar 换模式」），与 [`OscillationCampaign::earning_active`] 同判据。
                self.earning_mode_switch_bar.insert((level, side_label(side)), bar + 1);
                ("enter_earning", 0)
            }
            _ => return,
        };
        self.stage_events.push(StageEventRecord {
            level,
            side: side_label(side),
            bar,
            bars_since_open,
            kind,
            amount,
        });
    }

    /// ★#380 项一：记一次「亏损往返如实入账」（本次动作落账后桶现金为负）。
    pub fn record_loss_accounted(&mut self, side: VoiceSide) {
        *self.loss_round_trip_accounted_count.entry(side_label(side)).or_insert(0) += 1;
    }

    /// ★#383：记一次**等金额回补**（次数 + 本次净增股数；`gained` 可为 0——回补价高于卖出价
    /// 时同一笔钱买不回同样多的货，照实记 0 不编造）。
    pub fn record_earning_replenish(&mut self, side: VoiceSide, gained: i64) {
        let sl = side_label(side);
        *self.earning_replenish_count.entry(sl).or_insert(0) += 1;
        *self.earning_units_gained.entry(sl).or_insert(0) += gained;
    }

    /// ★#380 项四：记一次回补的逐批冲抵明细（同中枢/异中枢分桶，见证「同中枢优先」）。
    pub fn record_cover(&mut self, side: VoiceSide, cover: &[CoverAssignment]) {
        for c in cover {
            let bucket = if c.same_center { "same_center" } else { "other_center" };
            *self.cover_by_side.entry((side_label(side), bucket)).or_insert(0) += 1;
        }
    }

    /// 记一次触发构造尝试的结果（`Ok`/`Err` 分布，丢弃率分母+分子）。
    pub fn record_trigger_result<T>(&mut self, result: &Result<T, TriggerError>) {
        self.trigger_attempts += 1;
        match result {
            Ok(_) => {}
            Err(TriggerError::CenterNotAlive) => self.dropped_center_not_alive += 1,
            // ★#366：新判据的专桶，不并入 `dropped_other_trigger`。
            Err(TriggerError::CenterMovedDown) => self.dropped_center_moved_down += 1,
            Err(TriggerError::FlatSignal) | Err(TriggerError::PriceOutsideZone) => {
                self.dropped_other_trigger += 1
            }
        }
    }

    /// ★#366：记一次「未闭合减出」核销（货缺口与桶实收现金**分列**累计，不相减）。
    pub fn record_write_off(&mut self, reduction: &UnclosedReduction) {
        let sl = side_label(reduction.side);
        *self.unclosed_write_off_count.entry(sl).or_insert(0) += 1;
        *self.unclosed_write_off_units_gap.entry(sl).or_insert(0) += reduction.units_gap;
        *self.unclosed_write_off_cash_booked.entry(sl).or_insert(0) += reduction.cash_booked;
    }

    /// ★#366：记一次「三卖终局到达但无可核销」（该侧空仓，或该中枢无挂起批次）。
    pub fn record_write_off_nothing_to_settle(&mut self, side: VoiceSide) {
        *self.unclosed_write_off_nothing_to_settle.entry(side_label(side)).or_insert(0) += 1;
    }

    /// 记一次挂起终结来源（挂起归宿分桶）。★#381 关票修复：按 (持仓侧, 来源) 分桶——
    /// `side` 即该终结所在 `SuspensionOutcome` 的持仓侧，调用点已在手。
    pub fn record_suspension_source(
        &mut self,
        side: VoiceSide,
        source: SuspensionTerminationSource,
    ) {
        let label = match source {
            SuspensionTerminationSource::BrokenByThirdClassBuy => "broken_by_third_class_buy",
            SuspensionTerminationSource::BrokenByThirdClassSell => "broken_by_third_class_sell",
            SuspensionTerminationSource::Reset => "reset",
            SuspensionTerminationSource::Superseded => "superseded",
            SuspensionTerminationSource::RebaseVanished => "rebase_vanished",
        };
        *self.suspension_by_source.entry((side_label(side), label)).or_insert(0) += 1;
    }

    /// 记一次动作按 (级别, 动作) 分桶（归属正确性见证）。
    pub fn record_action(&mut self, level: u32, side: VoiceSide, action: CenterOscillationAction) {
        let label = match action {
            CenterOscillationAction::Reduce => "reduce",
            CenterOscillationAction::Replenish => "replenish",
        };
        *self.action_by_level.entry((level, side_label(side), label)).or_insert(0) += 1;
    }

    /// 记一次 campaign 生死事件。★#381 关票修复：按事件自带的 `side` 分侧累计（两侧不得相加）。
    pub fn record_lifecycle(&mut self, event: CampaignLifecycleEvent) {
        match event {
            CampaignLifecycleEvent::Opened { side, .. } => {
                *self.lifecycle_opened.entry(side_label(side)).or_insert(0) += 1;
            }
            CampaignLifecycleEvent::Died { side, .. } => {
                *self.lifecycle_died.entry(side_label(side)).or_insert(0) += 1;
            }
        }
    }

    /// 记一次 `apply_action` 拒绝。★#381：`is_short_side_held` 入参随
    /// `unsupported_short_position_count` 桶一同退役——空头侧已有自己的 `(level, Short)`
    /// campaign，`NoActiveCampaign` 在两侧同义（该侧真空仓，结构信号独立于持仓的预期场景）。
    ///
    /// 其余变体按既定分桶（`ShortDiff(ChannelRejected(CashUnsound))` 拆 holding/free 两子桶，
    /// issue #357 关票条件 B；其余是真正的接线/记账错误警报，见字段文档，不得混桶）。
    pub fn record_violation(&mut self, side: VoiceSide, violation: CampaignViolation) {
        let sl = side_label(side);
        match violation {
            CampaignViolation::NoActiveCampaign => {
                *self.no_active_campaign_count.entry(sl).or_insert(0) += 1;
            }
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { holding, .. },
            )) if holding < 0 => {
                *self.resource_exhausted_holding_negative_count.entry(sl).or_insert(0) += 1;
            }
            // ★#380 项四：「触发但货满」——预期经济场景，独立分桶。
            CampaignViolation::ReplenishWhileFull => {
                *self.replenish_triggered_but_full_count.entry(sl).or_insert(0) += 1;
            }
            // ★#380 项二：防线（读当时真实持仓）拒绝的超卖——预期读数，独立分桶。
            CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis { .. }) => {
                *self.defense_units_exceed_current_holding_count.entry(sl).or_insert(0) += 1;
            }
            // ★#383：阶段三硬门（桶现金永不为负）——生产恒 0，非 0 即警报，但归属明确故独立
            // 分桶（不埋进 `other_violation_by_kind` 的混合桶里）。
            CampaignViolation::EarningCashUnsound { .. } => {
                *self.earning_cash_unsound_count.entry(sl).or_insert(0) += 1;
            }
            // ★#383：等金额腿买不起一股——预期经济场景，独立分桶。
            CampaignViolation::EarningSizingRoundsToZero { .. } => {
                *self.earning_sizing_rounds_to_zero_count.entry(sl).or_insert(0) += 1;
            }
            other => {
                self.other_violation_count += 1;
                let label = match other {
                    CampaignViolation::NoActiveCampaign => "no_active_campaign_unreachable",
                    CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                        TransitionError::CashUnsound { free, .. },
                    )) if free < 0 => {
                        // ★#380 项一：free<0 现由 `short_diff_cash_gate` 放行入账（亏损往返），
                        // 本路径**已不可达**——旧 `resource_exhausted_free_negative_count` 桶随之
                        // 退役；若仍到达即为通道被改坏的真正警报，落其余违规桶而非资源桶。
                        "cash_unsound_free_negative_unreachable"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                        TransitionError::CashUnsound { .. },
                    )) => {
                        // holding<0/free<0 已在上方分支处理；三量皆非负时 cash_sound_gate 本不该
                        // 拒绝——若触达此分支属真正异常，照实归类而非静默吞入某个资源桶。
                        "resource_exhausted_cash_sound_neither_negative_unreachable"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                        TransitionError::Oq9Illegal { .. },
                    )) => {
                        // OQ-9 在本路径恒真（ShortDiff/Realize 的 is_legal_from 恒真）——若触达
                        // 说明合法性表被破坏，是真正的接线/记账错误，不得归入资源耗尽桶。
                        "resource_exhausted_oq9_illegal_unexpected"
                    }
                    CampaignViolation::SizingRoundsToZero { .. } => "sizing_rounds_to_zero",
                    CampaignViolation::ShortDiff(ShortDiffViolation::NonPositiveUnits(_)) => {
                        "short_diff_non_positive_units"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::OverReplenish { .. }) => {
                        "short_diff_over_replenish"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::UnclosedRoundTrip { .. }) => {
                        "short_diff_unclosed_round_trip"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::AvgCostMismatch { .. }) => {
                        "short_diff_avg_cost_mismatch"
                    }
                    // 下两条已在上方分支各自分桶（#380 项二/项四），此处只为穷尽性。
                    CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis { .. }) => {
                        "short_diff_units_exceed_cost_basis_unreachable"
                    }
                    CampaignViolation::ReplenishWhileFull => "replenish_while_full_unreachable",
                    // 下两条已在上方分支各自分桶（#383），此处只为穷尽性。
                    CampaignViolation::EarningCashUnsound { .. } => {
                        "earning_cash_unsound_unreachable"
                    }
                    CampaignViolation::EarningSizingRoundsToZero { .. } => {
                        "earning_sizing_rounds_to_zero_unreachable"
                    }
                    // ★#366：核销越界＝归属账与桶标量失同步（两者逐笔同步推进，生产恒不
                    // 触达）——真正的记账错误警报，不得归入任何「预期读数」桶。
                    CampaignViolation::ShortDiff(ShortDiffViolation::WriteOffExceedsOpen {
                        ..
                    }) => "short_diff_write_off_exceeds_open",
                    CampaignViolation::StageTransition(_) => "stage_transition",
                };
                *self.other_violation_by_kind.entry((sl, label)).or_insert(0) += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ledger::TStage;
    use super::super::short_diff_bucket::CoreCostBasisSnapshot;
    use super::super::voice::VoiceSide;

    fn snapshot(units: i64, cost_basis: i64) -> CoreCostBasisSnapshot {
        CoreCostBasisSnapshot::new(units, cost_basis)
    }

    /// 来源中枢身份（#380 项四）：`start_index` 区分不同中枢实例，其余字段固定。
    fn cid(start_index: usize) -> CenterId {
        CenterId::of(&crate::theta_v0::types::Center {
            zd: 100,
            zg: 200,
            dd: 98,
            gg: 202,
            start_index,
            end_index: start_index + 50,
        })
    }

    // ── 生命周期：开仓生 / 全平死 ────────────────────────────────────────

    #[test]
    fn sync_position_opens_campaign_on_transition_from_flat_to_held() {
        let mut book = CampaignBook::new();
        let outcome = book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        assert_eq!(outcome, Some(CampaignLifecycleEvent::Opened { level: 0, side: VoiceSide::Long, notional_in: 3_000 }));
        let campaign = book.campaign(0, VoiceSide::Long).expect("开仓后应存在 campaign");
        assert_eq!(campaign.tw().notional_in, 3_000, "notional_in=本仓成本基（缠师口径成本入账）");
        assert_eq!(campaign.tw().holding, 3_000, "holding=本仓成本基（与 CoreCostBasisSnapshot 同步）");
        assert_eq!(campaign.tw().stage, TStage::CostReduction, "开局即降成本阶段");
    }

    #[test]
    fn sync_position_is_noop_while_still_flat_or_still_held() {
        let mut book = CampaignBook::new();
        assert_eq!(book.sync_position(0, VoiceSide::Long, snapshot(0, 0), 0), None, "仍空仓 ⟹ no-op");
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        assert_eq!(book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0), None, "仍持仓 ⟹ no-op（加仓重算不在本模块范围）");
        assert_eq!(book.active_count(), 1);
    }

    #[test]
    fn sync_position_kills_campaign_on_transition_to_flat() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        let outcome = book.sync_position(0, VoiceSide::Long, snapshot(0, 0), 0);
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died { level: 0, side: VoiceSide::Long, suspended_units_forfeited: 0 }),
            "无挂起在途量的全平死亡"
        );
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "全平后 campaign 实例被移除");
        assert_eq!(book.active_count(), 0);
    }

    /// ★挂起随死（issue #294 验收②「全平=campaign 终结含挂起随死」）：全平死亡时若短差盈亏桶
    /// 仍有挂起在途量（未及收口的半轮往返），死亡照实记录该量，不强求先 assert_conserved。
    #[test]
    fn sync_position_death_forfeits_open_suspended_units_without_requiring_conservation() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap(); // 卖 100（1/3）
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().short_diff().bucket().open_units(), 100, "挂起在途量=100（半轮往返）");
        let outcome = book.sync_position(0, VoiceSide::Long, snapshot(0, 0), 0); // 仓位全平（未先回补）
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died { level: 0, side: VoiceSide::Long, suspended_units_forfeited: 100 }),
            "挂起随死：全平死亡照实记录未收口的挂起量，非违规"
        );
        assert!(book.campaign(0, VoiceSide::Long).is_none());
    }

    #[test]
    fn each_level_has_independent_campaign_no_cross_book_offset() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        book.sync_position(1, VoiceSide::Long, snapshot(500, 20_000), 0);
        assert_eq!(book.active_count(), 2);
        book.sync_position(0, VoiceSide::Long, snapshot(0, 0), 0);
        assert_eq!(book.active_count(), 1, "0 级死亡不影响 1 级 campaign（禁跨仓冲减，每级一本账强制）");
        assert!(book.campaign(1, VoiceSide::Long).is_some());
    }

    // ── sizing=当时持仓 1/3（issue #348） ───────────────────────────────

    #[test]
    fn reduce_sizes_to_one_third_of_currently_held_units() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10
        let outcome = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
        assert_eq!(outcome.units, 100, "sizing=当时持仓(300)/3=100（#348 裁定，整数除法）");
    }

    #[test]
    fn replenish_sizes_to_full_open_units_hard_conservation_in_cost_reduction() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap(); // 卖 100
        let outcome = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 9, RiskMode::Normal, cid(0)).unwrap();
        assert_eq!(outcome.units, 100, "阶段一恒仓约束硬：Replenish 全额买回挂起在途量（同股数进出）");
        assert!(
            book.campaign(0, VoiceSide::Long).unwrap().short_diff().assert_conserved().is_ok(),
            "整轮收口 ⟹ 恒仓断言过"
        );
    }

    #[test]
    fn sizing_rounding_to_zero_is_explicit_violation_not_silent_minimum() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(2, 20), 0); // 持仓仅 2 ⟹ 2/3=0
        assert_eq!(
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 15, RiskMode::Normal, cid(0)),
            Err(CampaignViolation::SizingRoundsToZero { held: 2 }),
            "1/3 取整到 0 时显式拒绝，不静默钳制为 1"
        );
    }

    #[test]
    fn action_on_level_without_open_campaign_is_explicit_wiring_error() {
        let mut book = CampaignBook::new();
        assert_eq!(
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 10, RiskMode::Normal, cid(0)),
            Err(CampaignViolation::NoActiveCampaign)
        );
    }

    // ── 本仓成本基不动（修7）+ P2-D 双账入口对齐 ──────────────────────────

    #[test]
    fn cost_basis_untouched_and_ledger_pi_mirrors_tw_realize() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10
        let start_tw = book.campaign(0, VoiceSide::Long).unwrap().tw(); // 开局 tw()=holding(3000)+free(0)+withdrawn(0)=3000
        let original_cost_basis = book.campaign(0, VoiceSide::Long).unwrap().short_diff().cost_basis();

        let reduce = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
        assert_eq!(reduce.ledger.pi, 100 * (12 - 10), "R 账本 Π 镜像本轮 Realize=units·(price−avg_cost)=200");
        assert!(reduce.ledger.inv_holds(), "R 账本恒等 R=Π-A-W 全程成立");

        let replenish = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 9, RiskMode::Normal, cid(0)).unwrap();
        assert_eq!(
            replenish.ledger.pi,
            100 * (12 - 10) + 100 * (10 - 9),
            "R 账本 Π 累计两笔 Realize（P2-D：TW/R 双账入口对齐，同一 Realize 分量镜像）"
        );
        assert_eq!(
            replenish.tw.tw() - start_tw.tw(),
            replenish.ledger.pi,
            "campaign 开局以来 TW 净漂移=ΣRealize=R 账本 Π（两账本各自入账同一已实现值，非同构但同源）"
        );

        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap().short_diff().cost_basis(),
            original_cost_basis,
            "本仓成本基（均价口径）全程不动（修7）"
        );
    }

    // ── 首见证：到 0 转移（issue #294 验收④，阶段机第一次真正到达 II） ────

    /// ★首见证核心用例：构造能到 0 的场景（多轮高抛低吸，模拟震荡+趋势窗内反复高抛低吸积累
    /// 现金）——断言 `RecoverCapital` 派发（阶段机到达 `CapitalRecovered`=II）、恒仓守恒逐笔、
    /// 本仓成本基不动、campaign 全平即死亡。
    ///
    /// 场景：本仓成本基 3_000（300 股，avg_cost=10）。每轮高抛低吸赚取「卖价−买价」的净现金，
    /// 需要 free 累计到 ≥ notional_in(3_000) 才能触发足额退本金（`stage_progression` 的
    /// cash-tight 门：`free≥recover_target`，见 `closed_loop::transition` 文档）。每轮 sizing=
    /// 100（300/3），净赚 4/股（卖12买8）⟹ 每轮 400，需 8 轮（3200≥3000）达门槛。
    #[test]
    fn first_witness_campaign_reaches_recover_capital_stage_ii() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().tw().stage, TStage::CostReduction, "起点：阶段机在 I（降成本）");

        let mut recovered_at_round = None;
        for round in 1..=8 {
            let sell = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
            assert_eq!(sell.units, 100, "每轮 sizing 恒=当时持仓(300)/3=100（本仓成本基不动，见下）");
            // 每轮卖出后立即核验恒仓状态：本轮挂起=100（尚未回补）。
            assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().short_diff().bucket().open_units(), 100);

            let cover = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0)).unwrap();
            assert_eq!(cover.units, 100, "整轮收口：买回等量 100（阶段一恒仓约束硬）");
            assert!(
                book.campaign(0, VoiceSide::Long).unwrap().short_diff().assert_conserved().is_ok(),
                "第 {round} 轮往返闭合 ⟹ 恒仓断言逐笔过"
            );

            if let Some(TwEvent::RecoverCapital(_)) = cover.stage_event {
                recovered_at_round = Some(round);
                break;
            }
        }

        let round = recovered_at_round.expect("★首见证：8 轮内必须派发 RecoverCapital（阶段机到达 II）");
        assert_eq!(round, 8, "净赚 4/股 x100 units x8 轮=3200≥notional_in(3000) ⟹ 第 8 轮足额退本金");

        let campaign = book.campaign(0, VoiceSide::Long).expect("退本金不终结 campaign（仓位仍在，只是阶段推进）");
        assert_eq!(campaign.tw().stage, TStage::CapitalRecovered, "★阶段机第一次真正到达 II");
        assert_eq!(campaign.tw().withdrawn, 3_000, "退本金额=notional_in（足额一次性退回）");
        assert_eq!(campaign.tw().l_wc(), 0, "在险本金归零（缠师「成本为 0」的现金口径落地）");
        assert_eq!(
            campaign.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "本仓成本基（均价口径）全程不动——「成本为 0」是现金口径,非均价口径（修7）"
        );

        // 全平即 campaign 死亡（联动 #274 出口规则：仓位归零时终结，不因阶段推进而提前终结）。
        let death = book.sync_position(0, VoiceSide::Long, snapshot(0, 0), 0);
        assert_eq!(death, Some(CampaignLifecycleEvent::Died { level: 0, side: VoiceSide::Long, suspended_units_forfeited: 0 }));
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "全平即 campaign 死亡");
    }

    /// 趋势窗对照：卖出与买回同价（11，无价差可赚）——每轮 `Reduce` 的 `Realize=units·(price−
    /// avg_cost)=100·(11−10)=100` 与 `Replenish` 的 `Realize=units·(avg_cost−price)=100·(10−11)
    /// =−100` 恰好相消，本轮净 Realize=0——现金积累不到 notional_in ⟹ 阶段机停留在 I（不到 0，
    /// 诚实退化，非 bug——见 `closed_loop::transition`「free 不足 ⟹ 等待」注记）。
    #[test]
    fn trend_window_without_enough_realized_profit_stays_at_stage_one() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10
        for _ in 0..8 {
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 11, RiskMode::Normal, cid(0)).unwrap();
            let cover = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 11, RiskMode::Normal, cid(0)).unwrap();
            assert_eq!(cover.stage_event, None, "同价往返净 Realize=0 ⟹ free 不积累 ⟹ 不推进");
        }
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().tw().stage, TStage::CostReduction, "趋势单边同价窗：诚实停留在 I，非硬凑到 0");
    }

    // ── CampaignWiringWitness（issue #357 验收③：产物级见证读数） ──────────

    #[test]
    fn witness_tallies_trigger_drop_rate_by_error_kind() {
        use super::super::center_oscillation_trade::TriggerError;
        let mut w = CampaignWiringWitness::new();
        w.record_trigger_result::<()>(&Ok(()));
        w.record_trigger_result::<()>(&Err(TriggerError::CenterNotAlive));
        w.record_trigger_result::<()>(&Err(TriggerError::CenterNotAlive));
        w.record_trigger_result::<()>(&Err(TriggerError::FlatSignal));
        w.record_trigger_result::<()>(&Err(TriggerError::PriceOutsideZone));
        assert_eq!(w.trigger_attempts, 5, "丢弃率分母=全部尝试次数");
        assert_eq!(w.dropped_center_not_alive, 2, "CenterNotAlive 单独分桶（丢弃率分子）");
        assert_eq!(w.dropped_other_trigger, 2, "FlatSignal/PriceOutsideZone 合桶（非 CenterNotAlive 关注点）");
    }

    #[test]
    fn witness_tallies_action_by_level_and_suspension_source_and_lifecycle() {
        let mut w = CampaignWiringWitness::new();
        w.record_action(0, VoiceSide::Long, CenterOscillationAction::Reduce);
        w.record_action(0, VoiceSide::Long, CenterOscillationAction::Reduce);
        w.record_action(1, VoiceSide::Long, CenterOscillationAction::Replenish);
        assert_eq!(w.action_by_level.get(&(0, "long", "reduce")), Some(&2), "归属正确：level 0 两次 Reduce 分桶计数");
        assert_eq!(w.action_by_level.get(&(1, "long", "replenish")), Some(&1), "level 1 Replenish 独立分桶，不与 level 0 混计");

        w.record_suspension_source(VoiceSide::Long, SuspensionTerminationSource::BrokenByThirdClassBuy);
        w.record_suspension_source(VoiceSide::Long, SuspensionTerminationSource::Superseded);
        w.record_suspension_source(VoiceSide::Long, SuspensionTerminationSource::Superseded);
        w.record_suspension_source(VoiceSide::Short, SuspensionTerminationSource::Superseded);
        assert_eq!(w.suspension_by_source.get(&("long", "broken_by_third_class_buy")), Some(&1));
        assert_eq!(w.suspension_by_source.get(&("long", "superseded")), Some(&2), "挂起归宿分桶按来源独立累计");
        // ★#381 关票修复：同一来源两侧独立分桶——多空并存时不得相加成 3（同一事件逐侧各产一条
        // `SuspensionOutcome`，混计即计数翻倍且无法归属）。
        assert_eq!(w.suspension_by_source.get(&("short", "superseded")), Some(&1), "空头侧同来源独立成桶");
        assert_eq!(
            w.suspension_by_source.get(&("short", "broken_by_third_class_buy")),
            None,
            "空头侧未发生的来源不出现（不被多头侧读数污染）"
        );

        w.record_lifecycle(CampaignLifecycleEvent::Opened { level: 0, side: VoiceSide::Long, notional_in: 3_000 });
        w.record_lifecycle(CampaignLifecycleEvent::Died { level: 0, side: VoiceSide::Long, suspended_units_forfeited: 0 });
        w.record_lifecycle(CampaignLifecycleEvent::Opened { level: 0, side: VoiceSide::Short, notional_in: 1_000 });
        // ★#381 关票修复：生死两轴分侧——事件已带 `side`，标量累加即跨侧求和。
        assert_eq!(w.lifecycle_opened.get("long"), Some(&1));
        assert_eq!(w.lifecycle_opened.get("short"), Some(&1), "空头侧开局独立计数，不与多头侧相加");
        assert_eq!(w.lifecycle_died.get("long"), Some(&1));
        assert_eq!(w.lifecycle_died.get("short"), None, "空头侧未死 ⟹ 该侧桶不出现（非 0 值混入）");

        assert!(w.no_active_campaign_count.is_empty());
        assert_eq!(w.other_violation_count, 0, "接线正常路径下其余通道拒绝计数恒 0");
        w.record_violation(VoiceSide::Long, CampaignViolation::NoActiveCampaign);
        assert_eq!(w.no_active_campaign_count.get("long"), Some(&1), "NoActiveCampaign ⟹ 该侧真空仓的预期经济场景，按侧分桶");
        assert_eq!(w.other_violation_count, 0, "不误落入其余违规桶");
        w.record_violation(VoiceSide::Long, CampaignViolation::SizingRoundsToZero { held: 2 });
        assert_eq!(w.other_violation_count, 1, "SizingRoundsToZero 是真正的记账异常，落其余桶");
    }

    /// ★#381：`unsupported_short_position_count` 桶退役的回归锁——空头侧持仓现有自己的
    /// `(level, Short)` campaign，`NoActiveCampaign` 在两侧同义（该侧真空仓的预期场景），
    /// 一律落 `no_active_campaign_count`，不再有第二个「有仓但认不出」的分流桶。
    /// （原 #357 关票条件 C 用例 `witness_splits_no_active_campaign_by_short_side_holding`
    /// 随桶一同退役——它断言的分流行为已不存在。）
    #[test]
    fn no_active_campaign_is_single_bucket_after_short_campaign_support() {
        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, CampaignViolation::NoActiveCampaign);
        w.record_violation(VoiceSide::Short, CampaignViolation::NoActiveCampaign);
        assert_eq!(w.no_active_campaign_count.get("long"), Some(&1), "两侧同义、但按侧分列");
        assert_eq!(w.no_active_campaign_count.get("short"), Some(&1), "空头侧不再落已退役的未支持桶");
        assert_eq!(w.other_violation_count, 0, "非记账错误，不落警报桶");
    }

    /// ★issue #357 关票条件 B：`ChannelRejected(CashUnsound)` 拆子桶——不再丢弃
    /// `TransitionError` 内层错误，归因可直接从读数判定。★#380 项一续：`free<0` 子桶退役
    /// （该路径改为放行入账，见本用例尾部）。
    #[test]
    fn witness_buckets_holding_negative_and_flags_retired_free_negative_path() {
        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, 
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { free: 10, holding: -5, withdrawn: 0 },
            )),
        );
        assert_eq!(w.resource_exhausted_holding_negative_count.get("long"), Some(&1), "holding<0 ⟹ 预算耗尽子桶（按侧）");
        assert_eq!(w.other_violation_count, 0, "holding<0 落资源耗尽子桶，不误落其余违规桶");

        // ★#380 项一：free<0 路径经 `short_diff_cash_gate` 放行入账后**已不可达**——旧
        // `resource_exhausted_free_negative_count` 桶随之退役；若仍到达即为通道被改坏的真正
        // 警报，落其余违规桶（而非静默吞入某个资源桶）。
        w.record_violation(VoiceSide::Long, 
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { free: -3, holding: 100, withdrawn: 0 },
            )),
        );
        assert_eq!(w.other_violation_count, 1, "free<0 拒绝现已不可达 ⟹ 到达即警报");
        assert_eq!(
            w.other_violation_by_kind.get(&("long", "cash_unsound_free_negative_unreachable")),
            Some(&1),
            "退役桶的残余路径按不可达标签归类，不复用旧资源桶名"
        );
    }

    // ── #380 项一：亏损往返如实入账 ─────────────────────────────────────

    /// ★#380 项一核心用例（campaign 层）：卖 100@12 后急拉回补 100@50（买贵 40/股）——旧口径
    /// 被 `cash_sound_gate` 整笔拒绝回滚（亏钱的往返在账本里根本不存在，报告层上偏），新口径
    /// 如实入账：`realized_cash` 收负、桶现金（`tw.free`）为负、`loss_accounted` 置真。
    #[test]
    fn loss_round_trip_is_accounted_with_negative_bucket_cash() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        let cover = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 50, RiskMode::Normal, cid(1))
            .expect("★亏损往返如实入账——不再整笔拒绝");
        assert!(cover.loss_accounted, "本次落账后桶现金为负 ⟹ 亏损入账标志置真");
        assert_eq!(cover.tw.free, 100 * 12 - 100 * 50, "桶现金=-3800（短差累计倒贴）");
        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap().short_diff().bucket().realized_cash(),
            100 * 12 - 100 * 50,
            "报告层短差盈亏如实收负（不再系统性上偏）"
        );
        assert!(
            book.campaign(0, VoiceSide::Long).unwrap().short_diff().assert_conserved().is_ok(),
            "亏损往返同样闭合（恒仓断言不受影响）"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_loss_accounted(VoiceSide::Long);
        assert_eq!(w.loss_round_trip_accounted_count.get("long"), Some(&1), "亏损入账计数桶接替退役的 free<0 拒绝桶（按侧）");
    }

    // ── #380 项二：sizing 基准冻结 vs 防线读当前 ─────────────────────────

    /// ★#380 项二核心用例：campaign 开局冻结 300 股（sizing 基准恒为 300/3=100），其间主仓被减
    /// 到 50 股——`sync_position` 刷新 `current_units`（不产生生死事件、不动冻结快照），
    /// `Reduce` 按冻结基准算 100 但当时真实持仓只有 50 ⟹ 防线显式拒绝（超卖），
    /// `held` 报当前值 50。
    #[test]
    fn defense_reads_current_holding_while_sizing_stays_frozen() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().current_units(), 300, "开局：当前持仓=冻结快照");

        // 主仓被减到 50（仍持仓 ⟹ 无生死事件），冻结快照与 notional_in 均不动。
        assert_eq!(book.sync_position(0, VoiceSide::Long, snapshot(50, 500), 3), None, "仍持仓 ⟹ 无生死事件");
        let campaign = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(campaign.current_units(), 50, "防线基准刷新为当时真实持仓");
        assert_eq!(campaign.short_diff().cost_basis(), snapshot(300, 3_000), "sizing 基准=开局冻结快照，不动");
        assert_eq!(campaign.tw().notional_in, 3_000, "notional_in 不因持仓变动而改写");

        let result = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1));
        assert_eq!(
            result,
            Err(CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis {
                held: 50,
                attempted: 100,
            })),
            "按冻结基准算量(100)>当时真实持仓(50) ⟹ 「不卖没有的货」显式拒绝"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, result.unwrap_err());
        assert_eq!(w.defense_units_exceed_current_holding_count.get("long"), Some(&1), "防线拒绝落独立分桶（按侧，预期读数）");
        assert_eq!(w.other_violation_count, 0, "不混入接线错误警报桶");
    }

    // ── #380 项三：stage_event 见证 ─────────────────────────────────────

    /// ★#380 项三核心用例：阶段推进事件（`RecoverCapital`）落 witness——计数 + campaign 标识
    /// （级别）+ 开局以来 bar 数 + 金额。旧版 `fill.rs` 的 `Ok(_outcome) => {}` 把该事件整个
    /// 丢弃，#368 切换开关无物可读。
    ///
    /// ★#383 评审挂账（本用例手调 `record_stage_event` 绕过了生产接线）：本用例保留为
    /// **记录形状**的锁；「生产路径确实会把该事件记进 witness」现由 `fill.rs` 的
    /// `drive_campaign_wiring_lands_stage_events_and_earning_readings_end_to_end` 端到端覆盖
    /// （整支经 `drive_campaign_wiring`，无任何 witness 手调）。
    #[test]
    fn stage_event_lands_in_witness_with_level_and_bars_since_open() {
        let mut book = CampaignBook::new();
        let mut w = CampaignWiringWitness::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 5); // 开局 bar=5
        let mut recovered_bar = None;
        for round in 1..=8u32 {
            let bar = 5 + round as usize * 10;
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
            let cover = book
                .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(1))
                .unwrap();
            if let Some(ev) = cover.stage_event {
                let bars_since_open = book.campaign(0, VoiceSide::Long).unwrap().bars_since_open(bar);
                w.record_stage_event(0, VoiceSide::Long, bar, bars_since_open, ev);
                recovered_bar = Some(bar);
                break;
            }
        }
        let bar = recovered_bar.expect("8 轮内必派 RecoverCapital（#294 首见证场景）");
        assert_eq!(w.stage_recover_capital_count, 1, "RecoverCapital 计数");
        assert_eq!(w.stage_enter_earning_count, 0, "本场景不到 III");
        assert_eq!(
            w.stage_events,
            vec![StageEventRecord {
                level: 0,
                side: "long",
                bar,
                bars_since_open: bar - 5,
                kind: "recover_capital",
                amount: 3_000,
            }],
            "逐条明细：级别/bar/开局以来 bar 数/金额（=notional_in 足额退回）"
        );
    }

    // ── #380 项四：挂起归属（中枢标签 + 同中枢优先冲抵 + 触发但货满） ────

    /// ★#380 项四核心用例：三笔 `Reduce` 分属两个中枢（cid1 seq0、cid2 seq1、cid1 seq2），
    /// 一次 cid1 触发的全额回补 ⟹ 冲抵顺序=**同中枢优先**（seq0、seq2）**余按时间序**（seq1）。
    #[test]
    fn cover_order_is_same_center_first_then_time_order() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10，sizing=100
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(2)).unwrap();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();

        let campaign = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(
            campaign.suspension().batches().iter().map(|b| (b.center, b.units, b.seq)).collect::<Vec<_>>(),
            vec![(cid(1), 100, 0), (cid(2), 100, 1), (cid(1), 100, 2)],
            "挂起批次带来源中枢标签，按到达序排列"
        );
        assert_eq!(
            campaign.suspension().open_units(),
            campaign.short_diff().bucket().open_units(),
            "归属账总量与短差桶标量恒等（同一总量的两种口径，非两套记账）"
        );

        let outcome = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 9, RiskMode::Normal, cid(1))
            .unwrap();
        assert_eq!(outcome.units, 300, "阶段一恒仓约束硬：全额买回挂起在途量");
        assert_eq!(
            outcome.cover,
            vec![
                CoverAssignment { center: cid(1), units: 100, seq: 0, same_center: true },
                CoverAssignment { center: cid(1), units: 100, seq: 2, same_center: true },
                CoverAssignment { center: cid(2), units: 100, seq: 1, same_center: false },
            ],
            "同中枢（cid1）两批优先按到达序冲抵，余量再按时间序冲抵 cid2"
        );
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().suspension().open_units(), 0, "全额回补 ⟹ 挂起清空");

        let mut w = CampaignWiringWitness::new();
        w.record_cover(VoiceSide::Long, &outcome.cover);
        assert_eq!(w.cover_by_side.get(&("long", "same_center")), Some(&2), "同中枢冲抵计数（按侧）");
        assert_eq!(w.cover_by_side.get(&("long", "other_center")), Some(&1), "异中枢冲抵计数（分列，不混桶）");
    }

    /// ★#380 项四：来源中枢后续买点触发回补、但**货已满**（挂起在途量=0）⟹ typed
    /// `ReplenishWhileFull` + 独立分桶「触发但货满」，不与 sizing 取整异常混计、不落接线错误桶。
    #[test]
    fn replenish_while_full_is_typed_and_bucketed_separately() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        let result = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 9, RiskMode::Normal, cid(1));
        assert_eq!(result, Err(CampaignViolation::ReplenishWhileFull), "无挂起可回补 ⟹ 触发但货满");

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, CampaignViolation::ReplenishWhileFull);
        assert_eq!(w.replenish_triggered_but_full_count.get("long"), Some(&1), "「触发但货满」独立分桶如实计数（按侧）");
        assert_eq!(w.other_violation_count, 0, "非记账错误，不落警报桶");
        w.record_violation(VoiceSide::Long, CampaignViolation::SizingRoundsToZero { held: 2 });
        assert_eq!(w.other_violation_count, 1, "sizing 取整异常仍是警报，两者不混计");
        assert_eq!(w.replenish_triggered_but_full_count.get("long"), Some(&1), "货满桶不被误增");
    }

    // ── ★#366 未闭合减出核销（补充裁定 2026-07-27）：货缺口/桶实收现金分列，不冲销 ────

    /// 核心用例：cid(1) 高抛两笔 + cid(2) 高抛一笔后，cid(1) 三卖终局 ⟹ 只核销 cid(1) 的两批
    /// （货缺口 200 股、桶实收现金 200·12=2400），cid(2) 的挂起**不连坐**（那个中枢未死，
    /// 「挂起继续等」）。货缺口与桶实收现金**分列返回，不相减**。
    #[test]
    fn write_off_settles_only_the_terminated_center_and_reports_gap_and_cash_separately() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0); // avg_cost=10，sizing=100
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(2)).unwrap();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();

        let reduction = book
            .write_off_unclosed(0, VoiceSide::Long, cid(1))
            .expect("核销不应报错")
            .expect("cid(1) 有两批挂起可核销");
        assert_eq!(reduction.units_gap, 200, "货缺口=cid(1) 两批减出股数");
        assert_eq!(
            reduction.cash_booked,
            2_400,
            "多头侧桶实收=Σ[units·avg_cost+units·(price−avg_cost)]=Σ units·price=200·12（与货缺口分列，不相减）"
        );
        assert_eq!(reduction.center, cid(1));
        assert_eq!(reduction.side, VoiceSide::Long);

        let campaign = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(
            campaign.suspension().batches().iter().map(|b| (b.center, b.units)).collect::<Vec<_>>(),
            vec![(cid(2), 100)],
            "只核销终局中枢那两批，cid(2) 的挂起继续等"
        );
        assert_eq!(
            campaign.suspension().open_units(),
            campaign.short_diff().bucket().open_units(),
            "核销后归属账与桶标量仍恒等（同步推进，不失同步）"
        );
        assert_eq!(campaign.short_diff().bucket().written_off_units(), 200, "核销量留痕（不装没发生）");
    }

    /// ★不冲销的行为化断言：核销**不产任何 TW 事件**——TW 三量与 `realized_cash` 在核销前后
    /// 逐字节不变。冲销（造一笔虚拟回补把货补平）会改动 `holding`/`free`，本测试正是它的反例锚。
    #[test]
    fn write_off_does_not_offset_anything_tw_and_cash_are_byte_identical() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        let before = book.campaign(0, VoiceSide::Long).unwrap().clone();

        book.write_off_unclosed(0, VoiceSide::Long, cid(1)).unwrap().expect("有挂起可核销");
        let after = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(after.tw(), before.tw(), "核销不产 TwEvent ⟹ TW 三量不变（货缺口留在账上）");
        assert_eq!(after.ledger(), before.ledger(), "R 账本同样不动");
        assert_eq!(
            after.short_diff().bucket().realized_cash(),
            before.short_diff().bucket().realized_cash(),
            "桶实收现金照留（减出腿当时已入账），不被核销冲掉"
        );
        assert_eq!(after.short_diff().bucket().open_units(), 0, "在途量归位（承诺已终局，不再等回补）");
    }

    /// 无挂起可核销的两种诚实情形均返回 `Ok(None)`（非错误）：① 该中枢本就没有挂起批次；
    /// ② 该 (级别, 侧) 无 campaign（空仓）。
    #[test]
    fn write_off_with_nothing_to_settle_is_ok_none_not_an_error() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        assert_eq!(book.write_off_unclosed(0, VoiceSide::Long, cid(1)).unwrap(), None, "该中枢无挂起批次");
        assert_eq!(book.write_off_unclosed(0, VoiceSide::Short, cid(1)).unwrap(), None, "该侧无 campaign");
    }

    /// 核销后**同中枢的幽灵回补路径**也随之关闭：在途量已归位 ⟹ 再来一次 `Replenish` 落
    /// 「触发但货满」桶（`ReplenishWhileFull`），不会凭空补回已核销的货。
    #[test]
    fn replenish_after_write_off_is_rejected_as_full_not_resurrected() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        book.write_off_unclosed(0, VoiceSide::Long, cid(1)).unwrap().unwrap();
        assert_eq!(
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 9, RiskMode::Normal, cid(1)),
            Err(CampaignViolation::ReplenishWhileFull),
            "已核销的货不得被后续回补复活"
        );
    }

    /// 空头侧对称：核销口径对两侧同样适用（键含侧，各自独立）。
    ///
    /// ★口径订正（2026-07-27，评审 §2.4）：现金那一笔不是 `Σ units·price`（旧值 800）——空头
    /// 「减」= 回补空头（买回），成交腿是现金**支出** `units·price=800`，把它记成「卖出成交额/
    /// 现金盈余」值与符号双错。正确口径 = **桶实收现金**：`ShortDiff(units·avg_cost)=1000`
    /// （在险成本基 holding→free）+ `Realize(units·(avg_cost−price))=100·(10−8)=200`
    /// = `units·(2·avg_cost−price)=1200`。本测试同时对桶 `realized_cash` 增量取锚——两者恒等
    /// 是「现金一笔取桶实收、不另立公式」的行为化保证，改断言即须同时改语义。
    #[test]
    fn write_off_applies_symmetrically_to_short_side() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 0); // avg_cost=10，sizing=100
        let cash_before = book.campaign(0, VoiceSide::Short).unwrap().short_diff().bucket().realized_cash();
        book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(1)).unwrap();
        let cash_after = book.campaign(0, VoiceSide::Short).unwrap().short_diff().bucket().realized_cash();
        assert_eq!(
            cash_after - cash_before,
            100 * (2 * 10 - 8),
            "空头减出腿桶实收=units·avg_cost+units·(avg_cost−price)=units·(2·avg_cost−price)"
        );

        let reduction = book.write_off_unclosed(0, VoiceSide::Short, cid(1)).unwrap().expect("空头侧同样可核销");
        assert_eq!(reduction.side, VoiceSide::Short);
        assert_eq!(reduction.units_gap, 100);
        assert_eq!(reduction.cash_booked, 1_200, "桶实收=100·(2·10−8)=1200，非成交额 100·8=800");
        assert_eq!(reduction.cash_booked, cash_after - cash_before, "现金一笔恒等于桶实收增量，不另立公式");
        assert_ne!(reduction.cash_booked, 100 * 8, "空头侧不得回退到 Σ units·price 旧口径");
    }

    // ── #381：空头 campaign（键含侧 + 镜像减补 + 多空并存不污染） ──────────

    /// ★#381 验收①②：纯空头持仓开局 campaign 并按**镜像**记账——空头侧「减」=回补空头
    /// （买回，跌了才赚：`Realize=units·(avg_cost−price)`），「补」=加回空头（重新卖空：
    /// `Realize=units·(price−avg_cost)`）。整轮往返累计 = `units·(p_补 − p_减)`，即低吸高抛
    /// 回加的真实盈利；`ShortDiff` 两腿与多头侧逐字节相同（成本基划转与方向无关，往返相消）。
    #[test]
    fn short_campaign_mirrors_long_with_reduce_as_cover_and_replenish_as_re_short() {
        let mut book = CampaignBook::new();
        let opened = book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 0); // avg_cost=10
        assert_eq!(
            opened,
            Some(CampaignLifecycleEvent::Opened { level: 0, side: VoiceSide::Short, notional_in: 3_000 }),
            "空头侧开局生（生死事件带侧）"
        );
        let campaign = book.campaign(0, VoiceSide::Short).expect("空头 campaign 存在");
        assert_eq!(campaign.side(), VoiceSide::Short);
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "多头侧无仓 ⟹ 无多头 campaign");
        let start_tw = campaign.tw();

        // 减=回补空头 @8（低于均价 10 ⟹ 空头获利）。
        let reduce = book
            .apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(1))
            .unwrap();
        assert_eq!(reduce.units, 100, "sizing 口径不分侧：当时持仓(300)/3");
        assert_eq!(reduce.ledger.pi, 100 * (10 - 8), "空头减 Realize=units·(avg_cost−price)=+200（跌了才赚）");

        // 补=加回空头 @12（高于均价 ⟹ 卖得更高，同样为赚）。
        let replenish = book
            .apply_action(0, VoiceSide::Short, CenterOscillationAction::Replenish, 12, RiskMode::Normal, cid(1))
            .unwrap();
        assert_eq!(replenish.units, 100, "阶段一恒仓约束硬：全额加回挂起在途量");
        assert_eq!(
            replenish.ledger.pi,
            100 * (10 - 8) + 100 * (12 - 10),
            "空头补 Realize=units·(price−avg_cost)=+200（镜像多头补侧）"
        );
        assert_eq!(
            replenish.tw.tw() - start_tw.tw(),
            100 * (12 - 8),
            "整轮 TW 漂移=units·(p_补−p_减)=400，即低吸高抛回加的真实盈利"
        );

        let after = book.campaign(0, VoiceSide::Short).unwrap();
        assert_eq!(after.short_diff().bucket().realized_cash(), 100 * (12 - 8), "收口后累计=该侧真实盈利");
        assert!(after.short_diff().assert_conserved().is_ok(), "同股数进出 ⟹ 恒仓断言过");
        assert_eq!(after.tw().holding, start_tw.holding, "ShortDiff 两腿相消 ⟹ 在险成本基回原值（与多头侧同构）");
        assert_eq!(after.short_diff().cost_basis(), snapshot(300, 3_000), "本仓成本基全程不动（修7，两侧同）");
    }

    /// ★#381 镜像的反面：空头侧「减」在价格**高于**均价时如实亏损入账（涨了回补空头=亏），
    /// 与多头侧的亏损入账口径（#380 项一）对称适用——不整笔拒绝、不静默上偏。
    #[test]
    fn short_campaign_loss_is_accounted_symmetrically() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 0); // avg_cost=10
        book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(1))
            .unwrap();
        let cover = book
            .apply_action(0, VoiceSide::Short, CenterOscillationAction::Replenish, 4, RiskMode::Normal, cid(1))
            .expect("★亏损往返如实入账（两侧对称）");
        assert_eq!(cover.ledger.pi, 100 * (10 - 8) + 100 * (4 - 10), "加回价低于均价 ⟹ 本腿 Realize 为负");
        assert_eq!(
            book.campaign(0, VoiceSide::Short).unwrap().short_diff().bucket().realized_cash(),
            100 * (4 - 8),
            "整轮累计=units·(p_补−p_减)=−400，报告层如实收负"
        );
    }

    /// ★#381 验收③：多空并存各自独立生命周期——同级两侧各一本 campaign，`notional_in` 各按
    /// 各侧，一侧全平不牵连另一侧。
    #[test]
    fn long_and_short_campaigns_are_independent_per_side() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        book.sync_position(0, VoiceSide::Short, snapshot(50, 1_000), 0);
        assert_eq!(book.active_count(), 2, "同级两侧各一本账（键含侧，不覆盖）");
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().tw().notional_in, 3_000);
        assert_eq!(book.campaign(0, VoiceSide::Short).unwrap().tw().notional_in, 1_000);

        // 两侧各减一次：sizing 各按各侧冻结快照，互不冲抵。
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1))
            .unwrap();
        book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(1))
            .unwrap();
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().short_diff().bucket().open_units(), 100);
        assert_eq!(
            book.campaign(0, VoiceSide::Short).unwrap().short_diff().bucket().open_units(),
            16,
            "空头侧 sizing=50/3=16，不被多头侧的 100 污染"
        );

        let died = book.sync_position(0, VoiceSide::Short, snapshot(0, 0), 9);
        assert_eq!(
            died,
            Some(CampaignLifecycleEvent::Died { level: 0, side: VoiceSide::Short, suspended_units_forfeited: 16 }),
            "空头侧全平死（挂起随死，带侧）"
        );
        assert_eq!(book.active_count(), 1);
        assert!(book.campaign(0, VoiceSide::Long).is_some(), "多头侧不受牵连");
    }

    /// ★#381：witness 的动作分桶与阶段事件明细均带持仓侧——两侧同级同动作不再混桶。
    #[test]
    fn witness_action_and_stage_records_carry_side() {
        let mut w = CampaignWiringWitness::new();
        w.record_action(0, VoiceSide::Long, CenterOscillationAction::Reduce);
        w.record_action(0, VoiceSide::Short, CenterOscillationAction::Reduce);
        w.record_action(0, VoiceSide::Short, CenterOscillationAction::Reduce);
        assert_eq!(w.action_by_level.get(&(0, "long", "reduce")), Some(&1));
        assert_eq!(w.action_by_level.get(&(0, "short", "reduce")), Some(&2), "同级同动作按侧分列");

        w.record_stage_event(0, VoiceSide::Short, 12, 7, TwEvent::RecoverCapital(1_000));
        assert_eq!(
            w.stage_events,
            vec![StageEventRecord {
                level: 0,
                side: "short",
                bar: 12,
                bars_since_open: 7,
                kind: "recover_capital",
                amount: 1_000,
            }],
            "阶段事件明细带侧"
        );
    }

    /// ★#381（评审补齐）：#380 四项观测面对两侧**分侧可归属**——多空并存时同一批读数不再混计。
    /// SPEC #386 §2「witness 呈现归属与冲抵顺序」与 §3「§2 全部口径对两侧对称适用」的产物级锁。
    #[test]
    fn witness_380_observations_are_attributable_per_side() {
        let mut w = CampaignWiringWitness::new();
        // 项一（亏损入账）/ 项二（防线超卖）/ 项四（货满 + 冲抵归属）各喂两侧。
        w.record_loss_accounted(VoiceSide::Long);
        w.record_loss_accounted(VoiceSide::Short);
        w.record_loss_accounted(VoiceSide::Short);
        assert_eq!(w.loss_round_trip_accounted_count.get("long"), Some(&1));
        assert_eq!(w.loss_round_trip_accounted_count.get("short"), Some(&2), "两侧不相加（同名不同义）");

        w.record_violation(
            VoiceSide::Short,
            CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis {
                held: 5,
                attempted: 9,
            }),
        );
        assert_eq!(w.defense_units_exceed_current_holding_count.get("short"), Some(&1));
        assert_eq!(w.defense_units_exceed_current_holding_count.get("long"), None, "多头侧不被误增");

        w.record_violation(VoiceSide::Short, CampaignViolation::ReplenishWhileFull);
        assert_eq!(w.replenish_triggered_but_full_count.get("short"), Some(&1));

        let cover = vec![
            CoverAssignment { center: cid(1), units: 10, seq: 0, same_center: true },
            CoverAssignment { center: cid(2), units: 10, seq: 1, same_center: false },
        ];
        w.record_cover(VoiceSide::Short, &cover);
        assert_eq!(w.cover_by_side.get(&("short", "same_center")), Some(&1), "冲抵归属带侧");
        assert_eq!(w.cover_by_side.get(&("short", "other_center")), Some(&1));
        assert_eq!(w.cover_by_side.get(&("long", "same_center")), None, "多头侧冲抵桶不被空头污染");

        assert_eq!(w.other_violation_count, 0, "以上全为预期读数，非警报");
    }

    // ── #383：阶段三 EarningShares 等金额回补 ────────────────────────────

    /// 把一本多头 campaign 驱到 `EarningShares`（阶段三）并让模式**生效**（次 bar）。
    ///
    /// 场景同 #294 首见证：成本基 3_000（300 股 @10），每轮 `Reduce`@12 / `Replenish`@8 净赚
    /// 400 ⟹ 第 8 轮触发 `RecoverCapital(3000)`；第 9 轮的 `Reduce` 上 `EnterReady` 成立
    /// （W≥I0 ∧ legs=0 ∧ RiskNormal ∧ κ=0 基线 η⋆=0）⟹ 派 `EnterEarning`。
    /// 返回 `(book, 事件 bar, 生效 bar)`——事件 bar 上模式**尚未**生效（题三：同 bar 不换尺）。
    fn book_at_earning_stage() -> (CampaignBook, usize, usize) {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        for round in 1..=8usize {
            book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), round);
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
            book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0)).unwrap();
        }
        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap().tw().stage,
            TStage::CapitalRecovered,
            "前置：8 轮后阶段机在 II"
        );
        // 第 9 轮的 Reduce 触发 EnterEarning（事件 bar=9）。
        let event_bar = 9usize;
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), event_bar);
        let out = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(out.stage_event, Some(TwEvent::EnterEarning), "第 9 轮 Reduce 上派 EnterEarning");
        (book, event_bar, event_bar + 1)
    }

    /// ★题三（切换时点）：`EnterEarning` 的**事件 bar 上不换尺**，次 bar 才换——事件 bar 上
    /// 落下的挂起批次因此按**阶段一等量**口径收口（旧账按卖出时锁定）。
    #[test]
    fn earning_mode_switches_on_next_bar_not_the_event_bar() {
        let (mut book, event_bar, effective_bar) = book_at_earning_stage();
        let c = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(c.earning_event_bar(), Some(event_bar), "事件 bar 已记录");
        assert!(!c.earning_active(), "★同 bar 不换尺：事件 bar 上等金额 sizing 尚未生效");
        assert_eq!(c.earning_pool(), 0, "事件 bar 的减出按阶段一锁定 ⟹ 不进等金额现金池");

        // 事件 bar 上落下的挂起（100 股 @12）在同 bar 收口 ⟹ 等量买回 100，无净增股数。
        let cover = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(cover.units, 100, "旧账按卖出时锁定：阶段一卖出 ⟹ 回补永远等量");
        assert!(!cover.earning_replenish, "非等金额回补");
        assert_eq!(cover.earning_units_gained, 0, "等量口径无净增股数");

        // 次 bar：sync 推进 bar 时钟 ⟹ 模式生效。
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar);
        assert!(book.campaign(0, VoiceSide::Long).unwrap().earning_active(), "★次 bar 换模式");
    }

    /// ★题一（等金额 sizing）核心用例：阶段三生效后 `Reduce` 100@12 收 1_200 现金，
    /// `Replenish`@8 ⟹ `floor(1200/8)=150` 股——收口 100 + **净增 50 股**（挣股数）；
    /// 高抛侧仍是持仓 1/3（100，与阶段一同尺，只改回补侧）。
    #[test]
    fn earning_replenish_sizes_by_equal_cash_and_gains_shares() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        // 先把事件 bar 的旧账（等量口径）收口，避免与等金额族混在同一次回补里。
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0)).unwrap();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar);

        let sell = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(sell.units, 100, "高抛侧维持持仓 1/3（阶段三只改回补侧，补充八题一）");
        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap().earning_pool(),
            1_200,
            "阶段三锁定的减出把桶实收现金(100·12)记入等金额现金池"
        );

        let free_before = book.campaign(0, VoiceSide::Long).unwrap().tw().free;
        let cover = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(cover.units, 150, "★等金额回补 floor(1200/8)=150 股（抛出所得现金全额回补）");
        assert!(cover.earning_replenish, "本次是等金额口径");
        assert_eq!(cover.earning_units_gained, 50, "★净增 50 股 = 150−100（挣股数）");
        assert_eq!(cover.tw.free, free_before - 1_200, "桶现金恰扣 150·8=1200（现金不透支）");
        let c = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(c.earning_pool(), 0, "整除 ⟹ 无零头");
        assert!(c.short_diff().assert_conserved().is_ok(), "挂起在途量归零（收口腿等量冲抵）");
        assert_eq!(
            c.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "本仓成本基（均价口径）仍不动——增股数是旁路账本读数，不改主账本（修7 不变）"
        );
    }

    /// ★题一（零头留桶累计）：回补价 7 ⟹ `floor(1200/7)=171` 股、耗 1_197，**零头 3 留桶**，
    /// 下一轮与新减出的现金合并计算。
    #[test]
    fn earning_replenish_leaves_remainder_in_pool_for_next_round() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0)).unwrap();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar);

        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
        let cover = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 7, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(cover.units, 171, "floor(1200/7)=171");
        assert_eq!(cover.earning_units_gained, 71);
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().earning_pool(), 3, "★零头 3 留桶");

        // 下一轮：新减出 1_200 + 零头 3 = 1_203 ⟹ floor(1203/8)=150（零头累计进下次算量）。
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar + 1);
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().earning_pool(), 1_203, "零头累计进下次");
        let cover2 = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(cover2.units, 150, "floor(1203/8)=150");
        assert_eq!(book.campaign(0, VoiceSide::Long).unwrap().earning_pool(), 3, "1203−150·8=3 再留桶");
    }

    /// ★题三（旧账按卖出时锁定）：阶段三生效**前**卖出的挂起 + 生效**后**卖出的挂起并存时，
    /// 同一次回补里前者走等量、后者走等金额——两族分开算量后合并成交。
    #[test]
    fn replenish_splits_legacy_and_earning_batches_by_sell_time_lock() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        // 事件 bar 的挂起（100 股，阶段一锁定）**不收口**，留到阶段三与新挂起并存。
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar);
        // 阶段三锁定的第二笔减出：受累计防线约束（open 100 + 本笔 100 ≤ 当前 300）。
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        let c = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(c.suspension().open_units(), 200, "两族挂起并存");
        assert_eq!(c.earning_pool(), 1_200, "只有阶段三锁定那笔进池");

        let cover = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(1))
            .unwrap();
        // 等量族 100（旧账锁定）+ 等金额族 floor(1200/8)=150（收口 100 + 净增 50）。
        assert_eq!(cover.units, 250, "★两族分开算量：100（等量）+150（等金额）");
        assert_eq!(cover.earning_units_gained, 50);
        assert!(book.campaign(0, VoiceSide::Long).unwrap().short_diff().assert_conserved().is_ok(), "两族均已收口");
    }

    /// ★题二（阶段三唯一硬门）：桶现金永不为负——违规显式失败，整笔不落账、状态不变。
    ///
    /// 构造：阶段三生效后留着阶段一锁定的挂起（100 股 @12 卖出），在**远高于卖出价**的 900
    /// 上收口 ⟹ 该等量腿要付 90_000，桶现金转负。阶段一/二 下这是 #380 的「亏损如实入账」，
    /// 阶段三下被硬门拒绝（两门按阶段分域，见 `CampaignViolation::EarningCashUnsound`）。
    #[test]
    fn stage_three_hard_gate_rejects_negative_bucket_cash_without_mutating_state() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar);
        let before = book.campaign(0, VoiceSide::Long).unwrap().clone();

        let result = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 900, RiskMode::Normal, cid(0));
        assert!(
            matches!(result, Err(CampaignViolation::EarningCashUnsound { free }) if free < 0),
            "★阶段三硬门：桶现金转负 ⟹ 显式失败（实得 {result:?}）"
        );
        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap(),
            &before,
            "违规显式失败 ⟹ 整笔不落账，campaign 状态逐字段不变"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, result.unwrap_err());
        assert_eq!(w.earning_cash_unsound_count.get("long"), Some(&1), "硬门独立分桶");
        assert_eq!(w.other_violation_count, 0, "归属明确，不落混合警报桶");
    }

    /// ★#383：等金额腿现金池买不起一股（`pool < price`）且无等量族可收口 ⟹ 独立 typed 拒绝，
    /// 不与「货已满」/「持仓过小」混计；挂起继续挂着，不静默补齐。
    #[test]
    fn earning_replenish_below_one_share_is_its_own_typed_rejection() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 8, RiskMode::Normal, cid(0)).unwrap();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), effective_bar);
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();

        // 池=1_200，回补价 5_000 ⟹ floor(1200/5000)=0 股。
        let result = book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 5_000, RiskMode::Normal, cid(0));
        assert_eq!(
            result,
            Err(CampaignViolation::EarningSizingRoundsToZero { pool: 1_200, price: 5_000 })
        );
        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap().suspension().open_units(),
            100,
            "挂起继续挂着（不静默补齐、不当作已收口）"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, result.unwrap_err());
        assert_eq!(w.earning_sizing_rounds_to_zero_count.get("long"), Some(&1));
        assert_eq!(w.replenish_triggered_but_full_count.get("long"), None, "不与「货已满」混计");
        assert_eq!(w.other_violation_count, 0, "预期经济场景，非警报");
    }

    /// ★回归锁：阶段一（无阶段三批次）时回补量 = 全部挂起在途量——#348 口径逐字节不变。
    #[test]
    fn replenish_plan_in_stage_one_is_full_open_units() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Long, snapshot(300, 3_000), 0);
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(0)).unwrap();
        book.apply_action(0, VoiceSide::Long, CenterOscillationAction::Reduce, 12, RiskMode::Normal, cid(1)).unwrap();
        let cover = book
            .apply_action(0, VoiceSide::Long, CenterOscillationAction::Replenish, 9, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(cover.units, 200, "阶段一：全额买回挂起在途量（同股数进出）");
        assert!(!cover.earning_replenish);
        assert_eq!(cover.earning_units_gained, 0);
    }

    /// ★witness 报告层三项：等金额回补次数 / 累计净增股数 / 切换时点（生效 bar = 事件 bar+1）。
    #[test]
    fn witness_reports_earning_counts_gained_units_and_switch_bar() {
        let mut w = CampaignWiringWitness::new();
        w.record_stage_event(0, VoiceSide::Long, 9, 9, TwEvent::EnterEarning);
        assert_eq!(w.stage_enter_earning_count, 1);
        assert_eq!(
            w.earning_mode_switch_bar.get(&(0, "long")),
            Some(&10),
            "★切换时点读数=事件 bar+1（次 bar 换模式）"
        );

        w.record_earning_replenish(VoiceSide::Long, 50);
        w.record_earning_replenish(VoiceSide::Long, 71);
        w.record_earning_replenish(VoiceSide::Short, 12);
        assert_eq!(w.earning_replenish_count.get("long"), Some(&2), "等金额回补次数（分侧）");
        assert_eq!(w.earning_units_gained.get("long"), Some(&121), "累计净增股数=50+71");
        assert_eq!(w.earning_units_gained.get("short"), Some(&12), "两侧分列，不得相加");
    }

    // ── #366/#381 评审挂账：空头侧阶段机零覆盖补齐 ────────────────────────

    /// ★评审挂账（空头阶段机零覆盖）：**空头 campaign 推进到阶段 II**（`RecoverCapital`）。
    ///
    /// 空头镜像（ADR 补充九）：「减=回补空头（买回）」赚 `avg_cost−price`，「补=加回空头
    /// （卖出）」赚 `price−avg_cost` ⟹ 整轮 Σ Realize = `units·(p_补 − p_减)`，低吸高抛回加
    /// 才是空头侧的盈利形态。本仓在险市值 3_000（300 份 @10），每轮减@8 / 补@12 净赚
    /// `100·4=400` ⟹ 第 8 轮足额退本金（3_200 ≥ 3_000），与多头侧同定理、同轮次。
    #[test]
    fn short_side_campaign_reaches_recover_capital_stage_ii() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 0);
        assert_eq!(book.campaign(0, VoiceSide::Short).unwrap().tw().stage, TStage::CostReduction);

        let mut recovered_at_round = None;
        for round in 1..=8usize {
            let sell = book
                .apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(0))
                .unwrap();
            assert_eq!(sell.units, 100, "空头侧 sizing 同尺：在险份数 300/3");
            let cover = book
                .apply_action(0, VoiceSide::Short, CenterOscillationAction::Replenish, 12, RiskMode::Normal, cid(0))
                .unwrap();
            assert!(
                book.campaign(0, VoiceSide::Short).unwrap().short_diff().assert_conserved().is_ok(),
                "第 {round} 轮往返闭合"
            );
            if let Some(TwEvent::RecoverCapital(_)) = cover.stage_event {
                recovered_at_round = Some(round);
                break;
            }
        }
        assert_eq!(recovered_at_round, Some(8), "★空头侧同样在第 8 轮足额退本金（与多头侧同定理）");
        let c = book.campaign(0, VoiceSide::Short).unwrap();
        assert_eq!(c.tw().stage, TStage::CapitalRecovered, "★空头阶段机第一次到达 II");
        assert_eq!(c.tw().withdrawn, 3_000, "退本金额=notional_in（足额一次性退回）");
        assert_eq!(c.tw().l_wc(), 0, "空头侧在险本金同样归零");
        assert_eq!(c.side(), VoiceSide::Short);
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "多头侧不因空头侧推进而被凭空开局");
    }

    /// ★评审挂账续：空头 campaign 继续推进到**阶段 III** 并做等金额回补——空头侧「补=加回
    /// 空头」的等金额算料同样取**桶实收现金**（ADR 补充九已定的两侧对称符号锚），净增的是
    /// 空头份数。
    #[test]
    fn short_side_campaign_enters_earning_and_gains_short_units() {
        let mut book = CampaignBook::new();
        book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 0);
        for round in 1..=8usize {
            book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), round);
            book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(0)).unwrap();
            book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Replenish, 12, RiskMode::Normal, cid(0)).unwrap();
        }
        book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 9);
        let out = book
            .apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(out.stage_event, Some(TwEvent::EnterEarning), "★空头阶段机到达 III");
        book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Replenish, 12, RiskMode::Normal, cid(0)).unwrap();

        book.sync_position(0, VoiceSide::Short, snapshot(300, 3_000), 10);
        assert!(book.campaign(0, VoiceSide::Short).unwrap().earning_active(), "次 bar 生效（两侧同规矩）");
        // 空头侧「减=回补空头@8」的桶实收 = units·avg_cost + units·(avg_cost−price)
        // = 100·10 + 100·2 = 1_200（补充九口径：单腿桶实收≠单笔成交现金）。
        book.apply_action(0, VoiceSide::Short, CenterOscillationAction::Reduce, 8, RiskMode::Normal, cid(0)).unwrap();
        assert_eq!(book.campaign(0, VoiceSide::Short).unwrap().earning_pool(), 1_200, "空头侧算料=桶实收现金");
        let cover = book
            .apply_action(0, VoiceSide::Short, CenterOscillationAction::Replenish, 10, RiskMode::Normal, cid(0))
            .unwrap();
        assert_eq!(cover.units, 120, "floor(1200/10)=120 份");
        assert_eq!(cover.earning_units_gained, 20, "★空头侧净增 20 份（挣的是空头份数）");
        assert!(cover.earning_replenish);
    }
}
