//! 狭义短差动作本体（SPEC #274 T2，issue #292）。
//!
//! ADR 0001 修正案一/补充二裁定 + #292 前置约束五条（#337 评审论证，2026-07-26）：
//! - **A**：触发用主格——`alive_center()`（T1 [`center_lifecycle::CenterEventMachine`]）语义
//!   未变，本模块直接消费其结果，不另立第二套「在场」判据。
//! - **B**（★★#414 改判，2026-07-27 用户裁定，ADR 补充十一）：superseded
//!   （[`center_lifecycle::DeathForm::ArenaTermination`]，被链推进取代）**不再是挂起的终结触发
//!   源**——中枢的终结唯一 = 三类买卖点（定理三），取代只是「在场中枢换了一个」（中枢层更替），
//!   不是中枢的死。原中枢四边框为历史事实（ZG/ZD + 起止边冻结），其三类点判据在死后继续适用，故**挂起
//!   延续**（不随取代清除），等原中枢的三类买卖点到达时按 #366 两终局清算（三买→回补、三卖→
//!   核销）。本裁定修正 ADR 补充六「取代 = 在场终结，挂起随之终结」的账目层表述（中枢层不变）。
//! - **C**：终结动作**幂等**——主流程常态是同一实例先收在场终结、后收教义死亡两条信号
//!   （#337 容读法），第二条信号必须是 no-op，不得重复产出/panic。
//! - **D**：挂起按**中枢身份**（[`center_lifecycle::CenterId`]，经 `killed_center_id()`）匹配，
//!   不按当前在场——链已推进换代后，终结事件仍须命中它所指的那个挂起实例。
//! - **E**：接住首次教义死亡——即便某实例从未被记录过在场终结（无前置 Superseded），一旦
//!   收到教义死亡（Broken）也要正确终结，不依赖「先 superseded 才能终结」的隐含前提。
//! - **F**（issue #292 续修，二轮评审浮出）：链**重基**（[`center_lifecycle::ChainConsumed::Rebased`]）
//!   到达时，挂起按身份三元组核对重基后的新链——身份仍在链上⟹**跟随迁移**（挂起状态原样保留；
//!   本机挂起表键本身就是身份，不含链下标侧车，故迁移是保状态的 no-op）；身份从新链上消失⟹
//!   **终结**（[`SuspensionTerminationSource::RebaseVanished`]，不回补；★#472 按未闭合减出
//!   核销）。禁悬空——[`CenterOscillationBook::on_chain_rebase`] 尾部机检断言：处理后任何仍
//!   挂起的身份都必须在新链上，不留「既非迁移又非终结」的第三态。
//!
//! ## 范围边界（与 T3/#293、T4/#294 分工）
//!
//! 本票只产出**动作决策**（Reduce/Replenish）与**挂起状态机**（出口二分），不触碰账本/成本基
//! ——「本仓成本基不动，短差往返盈亏记该级账内『短差盈亏』标签桶」的**记账实装**（TwEvent::
//! ShortDiff 成本基划转 + 恒仓断言）是 #293（T3）的职责；「每仓 campaign 粒度」是 #294（T4）
//! 的职责。本模块的 `CenterOscillationAction` 是这两票的输入契约，不是它们的实现。
//!
//! ## 命名纪律（票面条款 9）
//!
//! 机制名 = [`CenterOscillationAction`] / [`CenterOscillationTrigger`] / [`CenterOscillationBook`]
//! ——与 `ReverseOpen`/`OscillationBook`（#282 已删的 S6 配对子腿账面形态）严格区分；
//! 禁止任何形式复用「ShortDiff」作类型/字段命名（S6 已废止形态的名字不得回魂）。

use super::super::classifier::center_lifecycle::{CenterId, CenterLifecycleEvent};
use super::super::types::{Center, Side, Tick};
use super::oscillation::{BoundarySide, PanDivTrigger};
use super::voice::VoiceSide;
use std::collections::{BTreeMap, BTreeSet};

/// 触发构造的 typed 拒绝（无静默兜底）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerError {
    /// 本级中枢不在场（`alive_center()` 为 `None`）——A 裁定：无主格即无触发。
    CenterNotAlive,
    /// 次级别信号方向为 Flat——触发必须有向。
    FlatSignal,
    /// ★#366（判据对齐缠师原文，裁定 2026-07-27）：信号方向圈定的候选边界侧与价格不符——
    /// 次级别卖点但价格**未向上离开中枢**（`price < ZG`），或次级别买点但价格**未向下离开
    /// 中枢**（`price > ZD`）。桶名沿用（#292 既有读数口径连续），但判据已从「中轴二分
    /// （上半区/下半区）」换成「离开中枢（ZG/ZD）」——「上半区/下半区」博文全库 0 命中，
    /// 属自研规则冒充缠论判据（090 纪律），本票废除。依据：80 课「中枢震荡的卖点都是出现在
    /// 向上离开中枢时」、33 课「每次向下离开中枢只要出现底背驰，那就可以介入」、20 课中枢
    /// 区间 [ZD, ZG] 定义、49 课「中枢上方减、下方增」。
    PriceOutsideZone,
    /// ★#366：回补侧的「中枢不下移」前置过滤——价格向下离开中枢且有次级别底背驰，但本级
    /// 中枢链**已下移**（新中枢整体低于前中枢）⟹ 拒绝回补。对应 89 课「跌下来，如果不形成
    /// 中枢下移，而最多只是中枢扩展，那么就在次级别下跌的背驰时候买」——下移即趋势延续，
    /// 不是震荡回补的场景。只约束买点侧（高抛侧无对应原文条件，不臆造对称门）。
    ///
    /// ★实现的是原文的**零前视历史代理**（「上次是否已下移」），不是原文的前瞻问句（「这次
    /// 跌完会不会下移」）——替换声明见 [`CenterDrift`] 的教义替换声明段，不得简读为「依据
    /// 89 课」。
    CenterMovedDown,
}

/// ★#366：本级中枢链的**位移态**——「中枢不下移」前置过滤（89 课）的唯一输入。
///
/// 判据（20/30 课中枢区间口径）：当前在场中枢与其链上前驱**无重叠且整体更低**
/// （`alive.zg < prev.zd`）⟹ [`Self::MovedDown`]（中枢下移）；有重叠（中枢扩展/延伸）或
/// 链上无前驱 ⟹ [`Self::NoDownShift`]。判定在调用方（`fill.rs::step_center_oscillation`
/// 读本级链与 `alive_center()` 的链下标）——本类型只承载已判定的结论，不在本模块重算第二套
/// 链视图（同 A 裁定「身份唯一源=调用方传入」的精神）。
///
/// ★★教义替换声明（评审 §1.3 要求，2026-07-27 补录；**只声明，不改行为**）：本判据**不是**
/// 89 课原句的直译，是它的**历史代理**，两者不是同一个量——
///
/// - **原文（前瞻）**：「跌下来，**如果不形成中枢下移**，而最多只是中枢扩展，那么就在次级别
///   下跌的背驰时候买」——问的是**这一次跌下去之后会不会造出一个新的更低中枢**，是对当前
///   这次下跌**去向**的判断。
/// - **实现（历史）**：`alive.zg < prev.zd` 问的是**当前在场中枢相对它链上前驱是否已经下移**
///   ——是**已完成**的位移，不是对本次下跌结果的判断。
/// - **工程理由**：零前视约束（本引擎逐 bar 因果推进，决策 bar 上「这次跌完会不会形成新中枢」
///   在结构上尚未确定，读它即前视）。「已下移⟹趋势延续中⟹本次下跌大概率仍不是震荡回补」
///   是可辩护的代理，但它是**代理**，不是原文条件本身。
/// - **纪律位置**：090「自研规则不得冒充缠论判据」对本条同样适用——本票正是为清除同类问题
///   而开，故此处如实标注为「89 课的零前视代理」，不写成「依据 89 课」了事。代理与原文的
///   差距（如「已下移但本次跌完只是扩展」的样本被误拒）是**已知有效域限制**，可复检。
///
/// ★边沿决策披露（评审 §1.4，与 `CenterOscillationTrigger::new` 的 `price == ZG/ZD` 含端点
/// 同级别）：下移判定用**严格小于** `alive.zg < prev.zd`——两中枢**恰好相接**（`zg == prev.zd`，
/// 无重叠但也无间隙）时判 [`Self::NoDownShift`]（**放行**回补）。理由：20 课中枢区间为闭区间
/// `[ZD, ZG]`，端点相接时两区间在端点处仍有交点，按「有重叠=扩展/延伸」一侧归属，与同票
/// `price == ZG/ZD` 计入离开的含端点惯例同向。是实现决策，可复检。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterDrift {
    /// 未下移：中枢扩展/延伸，或链上无前驱可比。
    NoDownShift,
    /// 已下移：当前在场中枢整体低于链上前驱（`zg < prev.zd`）。
    MovedDown,
}

/// 触发事件：本级中枢在场（主格，A 裁定）+ 次级别买卖点（B 裁定：触发主信号源，盘背降为
/// 辅助参考、非必要条件）+ 边界侧（独立价格判据，B 裁定细化，2026-07-26）。不携任何账面
/// 语义（无数量、无成本基）——动作决策见 [`CenterOscillationBook::on_trigger`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterOscillationTrigger {
    level: u32,
    center: CenterId,
    boundary_side: BoundarySide,
    signal_side: VoiceSide,
    source_index: usize,
}

impl CenterOscillationTrigger {
    /// 唯一构造点：本级 `alive_center()` 身份（A 裁定）+ 次级别买卖点信号方向 + 该点价格
    /// + 本级中枢链位移态 → 边界侧。
    ///
    /// ★★#366 判据对齐缠师原文（裁定 2026-07-27，A 方案）：**废中轴二分**（「上半区/下半区」
    /// 博文全库 0 命中，自研规则不得冒充缠论判据，090 纪律），改回「离开中枢」形式——位置
    /// 参照是**中枢区间 [ZD, ZG] 整体**（20 课定义），不是它的某个内部分界：
    ///
    /// - 次级别卖点（`Short`，即次级别上涨背驰）候选边界=`Above`（高抛减仓）——仅当价格
    ///   **向上离开中枢**（`price >= ZG`，冲上沿 ZG 方向）才放行。依据：80 课「中枢震荡的
    ///   卖点都是出现在向上离开中枢时」；89 课「冲不起来，在次级别上涨背驰的时候卖」。
    /// - 次级别买点（`Long`，即次级别底背驰）候选边界=`Below`（回补）——须同时满足两条：
    ///   ① 价格**向下离开中枢**（`price <= ZD`，跌向下沿 ZD）；② 本级中枢**不下移**
    ///   （`drift == NoDownShift`）。依据：89 课「跌下来，如果不形成中枢下移，而最多只是
    ///   中枢扩展，那么就在次级别下跌的背驰时候买」（★条件②实装的是该原文的**零前视历史
    ///   代理**，非原句直译——替换声明见 [`CenterDrift`]）；33 课「每次向下离开中枢只要出现
    ///   底背驰，那就可以介入」。
    /// - 不满足位置判据 ⟹ `Err(PriceOutsideZone)`；位置成立但中枢已下移 ⟹
    ///   `Err(CenterMovedDown)`。两者整体拒绝构造（不是"仍构造但方向作废"），且**分列**
    ///   ——先判位置后判下移，使 `CenterMovedDown` 桶只计「本可成立的回补被下移否掉」，
    ///   读数不被「价格根本不在下沿」的样本污染。
    /// - 边沿恰等（`price == ZG` / `price == ZD`）计入离开（`>=`/`<=` 含端点，非开区间）
    ///   ——「冲上沿 ZG 方向」的实现决策，可复检；与 #292 旧判据「中轴恰等两侧均计入」的
    ///   含端点惯例一致。
    /// - **同级别的第二个边沿决策**（评审 §1.4 补披露）：下移判定用严格小于
    ///   `alive.zg < prev.zd`，两中枢**恰好相接**（`zg == prev.zd`）时判 `NoDownShift`＝**放行**
    ///   回补——同样是含端点惯例的一侧归属，实现决策、可复检，详见 [`CenterDrift`]。
    ///
    /// **无门（机械断言）**：签名唯一输入 = 本级在场身份 + 链位移态 + 次级别信号事实（方向+
    /// 价格+坐标），不接受、也无法接受任何次级别**账户/仓位状态**——编译期即杜绝互斥门的可能。
    pub fn new(
        level: u32,
        alive: Option<CenterId>,
        drift: CenterDrift,
        signal_side: VoiceSide,
        price: Tick,
        source_index: usize,
    ) -> Result<Self, TriggerError> {
        let center = alive.ok_or(TriggerError::CenterNotAlive)?;
        let boundary_side = match signal_side {
            VoiceSide::Long => BoundarySide::Below,
            VoiceSide::Short => BoundarySide::Above,
            VoiceSide::Flat => return Err(TriggerError::FlatSignal),
        };
        // 位置判据 = 离开中枢区间（20 课 [ZD, ZG] 整体），非中轴二分。
        let left_center = match boundary_side {
            BoundarySide::Above => price >= center.zg,
            BoundarySide::Below => price <= center.zd,
        };
        if !left_center {
            return Err(TriggerError::PriceOutsideZone);
        }
        // 「中枢不下移」前置过滤（89 课）——只约束回补侧，高抛侧无对应原文条件。
        if matches!(boundary_side, BoundarySide::Below) && matches!(drift, CenterDrift::MovedDown) {
            return Err(TriggerError::CenterMovedDown);
        }
        Ok(Self::from_parts(level, center, boundary_side, signal_side, source_index))
    }

    /// 已核验字段的直接构造（跳过 `new` 的价格半区判据）——供 `from_pan_div_trigger` 使用，
    /// 因为 `PanDivTrigger` 不携价格，边界侧只能原样传导，不适用独立价格判据。
    fn from_parts(
        level: u32,
        center: CenterId,
        boundary_side: BoundarySide,
        signal_side: VoiceSide,
        source_index: usize,
    ) -> Self {
        Self { level, center, boundary_side, signal_side, source_index }
    }

    /// #292 接线点二：`oscillation::PanDivTrigger`（#282 保留触发链）→ 本触发的转换（票面
    /// 「五要素映射」：级别/信号方向/中枢身份/边界侧/盘背证据 → 本触发的四项构造输入）。
    ///
    /// - **级别**：`pan_div.level()` 原样传入。
    /// - **信号方向**：`pan_div.signal_side()` 原样传入。
    /// - **边界侧**：`pan_div.boundary_side()` 原样传入——`PanDivTrigger` 不携价格，无法套用
    ///   `new` 的独立价格判据（B 裁定细化只约束生产开启臂的直接构造点），故本转换绕过 `new`
    ///   经 `from_parts` 直接构造；两处仍是同一 `Long→Below`/`Short→Above` 方向约定
    ///   （`pan_div_conversion_boundary_side_matches_source` 核对相等），只是本路径不做价格校验。
    /// - **中枢身份**：**不**取 `pan_div.center()`（那只是 `OscillationCenterRef` 坐标投影，
    ///   缺 zd/zg 核心区间，不是 `CenterId`）——按 A 裁定，身份唯一源 = 调用方传入的本级
    ///   `alive_center()`（经 `CenterId::of` 投影），不复用 PanDiv 自带坐标另立第二套「在场」
    ///   判据。`alive=None` ⟹ `Err(CenterNotAlive)`（与 `new` 同一失败面）。
    /// - **盘背证据**：`pan_div.evidence()` 降格为 `source_index`（`reference().source_index()`）
    ///   ——本触发类型面不携证据类型本身，只留可追溯坐标。
    ///
    /// ★★#292 触发源改码（用户裁定 2026-07-26，票面 B）：生产开启臂的触发主信号源已改为
    /// **次级别买卖点**（`CenterOscillationTrigger::new` 直收次级别信号方向+价格，见上）——
    /// 盘背**非必要条件**，本方法降格为**可选辅助转换**（保留 API，供未来盘背过滤/标注等辅助
    /// 用途；`fill.rs::step_center_oscillation` 生产驱动路径已不再调用它，唯一驱动源是次级别
    /// 买卖点）。
    pub fn from_pan_div_trigger(
        pan_div: PanDivTrigger,
        alive: Option<CenterId>,
    ) -> Result<Self, TriggerError> {
        let center = alive.ok_or(TriggerError::CenterNotAlive)?;
        Ok(Self::from_parts(
            pan_div.level(),
            center,
            pan_div.boundary_side(),
            pan_div.signal_side(),
            pan_div.evidence().reference().source_index(),
        ))
    }

    pub const fn level(self) -> u32 {
        self.level
    }

    pub const fn center(self) -> CenterId {
        self.center
    }

    pub const fn boundary_side(self) -> BoundarySide {
        self.boundary_side
    }

    pub const fn signal_side(self) -> VoiceSide {
        self.signal_side
    }

    pub const fn source_index(self) -> usize {
        self.source_index
    }
}

/// 本级账动作（票面条款 9：与 `ReverseOpen` 族严格区分，禁 `ShortDiff` 命名）。
///
/// - `Reduce`（上沿高抛）：本级账记卖出，实现盈亏进短差盈亏标签桶（T3 接线）；本仓成本基不动。
/// - `Replenish`（下沿回补）：本级账记买入；本仓成本基不动（唯二出口=全平清零/加仓重算，本模块
///   两者都不触碰）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterOscillationAction {
    Reduce,
    Replenish,
}

/// #292 门控接线可观测轨迹（fill.rs 开启臂产出）：一条「触发 → 减补动作」的可见记录。
/// 只是接线验收证据——真实记账（本仓成本基/短差盈亏标签桶）是 T3（#293）的职责，本记录
/// 不携任何账面语义。门关（`center_oscillation.enabled=false`）⟹ 本类型全程不构造（诚实空）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterOscillationActionRecord {
    pub bar: usize,
    pub level: u32,
    pub center: CenterId,
    pub action: CenterOscillationAction,
    /// ★#381：本条动作归属的**持仓侧**（`Long`=多头 campaign，`Short`=空头 campaign）——
    /// 同一次触发对两侧产出**镜像**动作（上沿：多头减/空头补；下沿：多头补/空头减），故
    /// 「级别」不再足以定位记账对象，须与 [`super::oscillation_campaign::CampaignBook`] 的
    /// `(level, side)` 键同形状。`Flat` 不构造（空仓无 campaign 可记）。
    pub side: VoiceSide,
}

/// 挂起短差的终结来源（ADR 补充二 + #292 前置约束五条 + #292 续修）。三源同走「终结」出口，
/// 互不重叠。
///
/// ★★#414 桶退役（2026-07-27 用户裁定，ADR 补充十一）：`Superseded` 变体**已删除**——被取代
/// 不是中枢的死（中枢终结唯一 = 三类买卖点），故它不再是任何挂起的终结来源；命中挂起时产
/// [`SuspensionContinuation`]（延续，不终结）。witness 的 `("侧","superseded")` 桶随之退役，
/// 接替它的是 `suspension_continued_count`（正面读数，见
/// [`super::oscillation_campaign::CampaignWiringWitness`]）。变体删除而非保留=让教义可机检：
/// 「Superseded 是终结来源」在类型层已不可表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionTerminationSource {
    /// 本级三类**买**点破坏（`Broken{breaker_side: Long}`）：终结伴随一次收手回补——
    /// 「高抛后出三类买点则于三类买点处回补」（049/068），回补动作来自这条真实证书本身，
    /// 不是凭空生成（幽灵回补=0 反例锚的正面形态）。
    BrokenByThirdClassBuy,
    /// 本级三类**卖**点破坏（`Broken{breaker_side: Short}`）：终结不回补——
    /// 「出三类卖点则不回补」，仓位留待新证书才可能再开（SPEC Out of Scope 已如实标注为
    /// 未细化口径，本机按此执行）。
    BrokenByThirdClassSell,
    /// ★#292 续修（二轮评审浮出的挂起悬空泄漏修复）：链重基（[`center_lifecycle::ChainConsumed::Rebased`]）
    /// 后，挂起对应的中枢身份已不在新链上——该实例连「被取代」的记录都没有，是工程重基这一
    /// 侧信道的失踪。终结不回补，★#472 按未闭合减出核销且禁任何形式复活；本来源是工程警报桶。
    /// 判据 = [`CenterOscillationBook::on_chain_rebase`] 逐挂起身份核对重基后的新链。
    RebaseVanished,
}

/// ★#366/#472/#489：一次终结的**清算终局**——闭合与未闭合减出核销分账，不煮一锅。
/// 教义三类点按是否收手回补二分；工程失踪 `RebaseVanished` 归入未闭合减出核销。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationSettlement {
    /// **闭合终局**（多头遇三类买点 / 空头遇三类卖点）：收手回补（哪怕回补价高于卖出价）
    /// → 亏损如实入账（#380 项一 `short_diff_cash_gate` 通道）→ 往返闭合 → 转持股，中途
    /// 不再短差直到新中枢形成（73 课：第三买点不回补即可能错过中枢上移；49 课）。
    CoverAndClose,
    /// **未闭合减出**：不回补，挂起**核销**——货缺口 `units_gap` 与现金 `cash_booked`
    /// **分开呈报，不冲销、不装没发生**。教义路径为多头三类卖点 / 空头三类买点；工程路径
    /// 保留 `RebaseVanished`。核销落点见
    /// [`super::oscillation_campaign::CampaignBook::write_off_unclosed`]。
    WriteOffUnclosed,
}

/// ★★#414（2026-07-27 用户裁定，ADR 补充十一）：一条「挂起**延续**」记录——`Superseded`
/// （被链推进取代）命中某侧挂起时的产出。
///
/// 与 [`SuspensionOutcome`] 的分野是教义性的、不是措辞差异：终结产出会把挂起从表里摘掉并进入
/// 清算（回补/核销），本记录**不动挂起表**——挂起原样留着，等原中枢的三类买卖点到达时
/// 才按 #366 两终局清算。故本类型不携 `settlement`/`cover_action`（此刻没有任何清算发生），
/// 只是可观测证据（witness `suspension_continued_count`），使「81% 的挂起改道延续」在产物级可读。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuspensionContinuation {
    pub center: CenterId,
    pub side: VoiceSide,
}

/// ★#414：一条中枢生命周期事件对挂起表的**全部**产出——终结与延续两路**分列**。
///
/// 分列而非合并为一个 `Vec`：两者对账目的含义相反（终结=挂起离场并清算；延续=挂起仍在场、
/// 一分钱不动），混进同一序列会让下游把「什么都没发生」记成一次归宿。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SuspensionEventOutcome {
    /// 本事件终结的挂起（逐侧各一条）。
    pub terminations: Vec<SuspensionOutcome>,
    /// ★#414：本事件使其**延续**的挂起（当前唯一来源 = `Superseded`）。
    pub continuations: Vec<SuspensionContinuation>,
}

/// 一次终结的产出：身份 + 来源 + 清算终局 + 是否伴随一次收手回补动作（仅 `CoverAndClose`
/// 终局非 `None`，且仅当该身份此前确在挂起中）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuspensionOutcome {
    pub center: CenterId,
    pub source: SuspensionTerminationSource,
    pub cover_action: Option<CenterOscillationAction>,
    /// ★#381：本条终结归属的持仓侧——挂起表分侧独立（同一中枢可两侧各自挂起），终结须逐侧
    /// 产出。收手回补的镜像口径见 [`CenterOscillationBook::on_lifecycle_event`]。
    pub side: VoiceSide,
    /// ★#366：本条终结的清算终局（两终局分账，见 [`TerminationSettlement`]）。
    pub settlement: TerminationSettlement,
}

/// ★#366/#472：一条「未闭合减出」核销请求——终结产出
/// （[`TerminationSettlement::WriteOffUnclosed`]）到 campaign 记账层的接线契约。本类型不携
/// 任何金额（货缺口/现金盈余由 campaign 侧的挂起归属账现算，不在结构侧另立第二套账）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnclosedWriteOffRequest {
    pub bar: usize,
    pub level: u32,
    pub center: CenterId,
    pub side: VoiceSide,
}

/// ★#366/#472/#489：某一持仓侧遇某一终结来源时的清算终局——能收手回补的即闭合终局；
/// 教义三类点不回补，或命中工程失踪 `RebaseVanished`，均为未闭合减出核销。
fn settlement_for(
    side: VoiceSide,
    source: SuspensionTerminationSource,
) -> TerminationSettlement {
    match source {
        SuspensionTerminationSource::BrokenByThirdClassBuy
        | SuspensionTerminationSource::BrokenByThirdClassSell => {
            if cover_action_for(side, source).is_some() {
                TerminationSettlement::CoverAndClose
            } else {
                TerminationSettlement::WriteOffUnclosed
            }
        }
        SuspensionTerminationSource::RebaseVanished => {
            TerminationSettlement::WriteOffUnclosed
        }
    }
}

/// ★#381：某一持仓侧遇某一终结来源时是否伴随一次收手回补——多空**镜像**。
///
/// - 多头侧：三类**买**点破坏 ⟹ 收手回补（「高抛后出三类买点则于三类买点处回补」049/068）；
///   三类卖点不回补（★#366：改判为**未闭合减出核销**，见 [`TerminationSettlement`]——「不
///   回补」不变，但不再是「装没发生」的静默作废）。
/// - 空头侧：三类**卖**点破坏 ⟹ 收手加回空头（结构续跌，空头敞口须补回）；三类买点不加回
///   （多头侧「三卖不回补」的逐字镜像，同样走未闭合减出核销）。
/// - `RebaseVanished` 两侧一律不回补，并按 #472 走未闭合减出核销。
fn cover_action_for(
    side: VoiceSide,
    source: SuspensionTerminationSource,
) -> Option<CenterOscillationAction> {
    let covers = match side {
        VoiceSide::Short => matches!(source, SuspensionTerminationSource::BrokenByThirdClassSell),
        VoiceSide::Long | VoiceSide::Flat => {
            matches!(source, SuspensionTerminationSource::BrokenByThirdClassBuy)
        }
    };
    covers.then_some(CenterOscillationAction::Replenish)
}

/// ★#679 D1b：一次重基里对某个旧身份的谱系查簿结论（域层只见本枚举，不依赖
/// [`crate::theta_v0::lineage_book`] 的具体类型——判定规则在域层，证书来源在接线层）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineageVerdict {
    /// 构造证书给出唯一 1→1 连续边（`relation=continued_1to1` 且四布尔全真）。
    Continued(CenterId),
    /// 本 bar 本级没有该旧身份的连续边。
    NoCert,
    /// 同一旧身份有多个候选后继——歧义。
    Ambiguous,
    /// 簿记的 bar 与本次重基的 bar 不符——工程错误，不做跨 bar 猜测。
    BarMismatch,
}

/// ★#679 D1b：谱系查簿接口。实现在 [`crate::theta_v0::lineage_book::LineageView`]。
pub trait LineageLookup {
    fn lookup(&self, old: CenterId) -> LineageVerdict;
}

/// ★#679 D1b：一次重基的迁移/拒迁分桶读数（票面「迁移计数分桶」的域层载体）。
///
/// **口径（评审 #679 订正：原文声称的「四子桶之和恒 = kept」「`migrated+kept` = 锚个数」是假
/// 不变量，已撤回）**：[`Self::kept`] 按 **(侧, 锚)** 计——摘表时逐 `(VoiceSide, CenterId)` 计数，
/// 同一锚若两侧同时挂起，核销时计两次；五个 fail-closed 子桶（[`Self::no_cert`] /
/// [`Self::target_absent`] / [`Self::ambiguous`] / [`Self::bar_mismatch`] / [`Self::duplicate_claim`]）
/// 按**锚**计，一个锚只落一个子桶一次——每个拒迁锚都有确定原因，不留「无归因的核销」，但子桶之和
/// 是**去重锚数**，只在没有锚双侧同时挂起时才与 `kept` 相等（wf8 单侧样本下恰好相等，不能当
/// 通用不变量外推）。`migrated` 计的是迁移**次数**（同一锚可跨 bar 多次被救下再迁），不是锚数，
/// 与 `kept` 不构成可相加验证的恒等式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RebaseMigrationTally {
    /// 凭 1→1 构造证书成功迁移身份锚的挂起数（`rebase_lineage_migrated`）。
    pub migrated: usize,
    /// 谱系不可达、保持 `RebaseVanished` 核销的挂起数（`rebase_vanished_kept`）。
    pub kept: usize,
    /// fail closed：无构造证书边。
    pub no_cert: usize,
    /// fail closed：证书给出的新身份不在重基后的新链上。
    pub target_absent: usize,
    /// fail closed：歧义（多候选后继）。
    pub ambiguous: usize,
    /// fail closed：簿 bar 不匹配。
    pub bar_mismatch: usize,
    /// fail closed：多个旧锚被判到同一新身份（重复 lineage claim）。
    pub duplicate_claim: usize,
}

/// 挂起短差状态机（每级别一台，与 T1 [`center_lifecycle::CenterEventMachine`] 同粒度）。
///
/// 身份 → 是否挂起：只存 `Suspended` 身份；不存在于表中 = 未挂起（含「从未挂起过」与
/// 「已终结」两种情形——两者对外行为相同：回补请求一律拒绝，终结信号一律 no-op，天然满足
/// C 裁定的幂等要求，无需额外的 `Terminated` 哨兵态）。
///
/// ★#292 H1（域层评审）：`BTreeMap` 非 `HashMap`——挂起身份会被确定性迭代并投影到重基终结、
/// 多侧归宿等 `Vec` 输出；`HashMap::keys()` 迭代序依赖默认哈希（跨进程/跨版本不确定）。
/// `BTreeMap` 按 `CenterId` 派生序确定性迭代，wf8 bit-exact 回归不受哈希实现变化影响
/// （exit.rs `step_active_set_with_subtree_close` 的
/// `HashSet` 反例：那里只做 `.contains()` membership 查询、从不迭代输出，故哈希序无关；
/// 本处迭代序直接进产出 `Vec` 顺序，二者边界正在于「迭代是否进输出」）。
///
/// ★#381：挂起表键改 `(VoiceSide, CenterId)`——多空并存时同一中枢可在两侧各自挂起，各按
/// 各自的边沿开局/收口（镜像口径见 [`Self::on_trigger_side`]），两侧互不覆盖、互不污染。
/// `VoiceSide` 在前保证同侧条目在 `BTreeMap` 中相邻（迭代序仍确定性，H1 理由不变）。
///
/// ★#487：值保存首次挂起时的完整中枢框（含冻结的 `end_index`）。`CenterId` 刻意不含会随延伸
/// 改写的右边，不能反过来承担四边框快照；`None` 只服务不掌握完整框的旧 API/域层单测，生产
/// 接线一律经 [`Self::on_trigger_side_bound`] 写入 `Some`。
///
/// ★★#679 D1b（双层身份，调研 `rebase-identity-d1-research-20260728.md` §4.2 方案 C）：挂起表的
/// 键是**稳定锚**（首次绑定时的身份），不随重基改写；重基后的新身份经
/// [`Self::on_chain_rebase_lineage`] 用 1→1 构造证书**别名**回该锚（[`Self::anchor_of`]）。
/// 这样一次迁移只改一张别名表，而
/// ① 首次绑定冻结的四边框（本表的值）、
/// ② campaign 挂起归属账的来源中枢标签（`oscillation_campaign::SuspensionBatch::center`）、
/// ③ 历史绑定三类证书的去重键（`fill::HistoricalSeenKey::owner`）
/// 三者全都保持自洽——不必在同一事务里并发迁移三处状态（那才是原子性风险的来源）。
/// 锚在两侧挂起都消失后随别名一起回收（[`Self::gc_anchor`]）。
#[derive(Debug, Default)]
pub struct CenterOscillationBook {
    level: u32,
    suspended: BTreeMap<(VoiceSide, CenterId), Option<Center>>,
    /// ★#679 D1b：**当前身份 → 稳定锚**。只由重基谱系迁移写入；恒等映射不入表
    /// （`resolve` 查不到即返回原值）。
    anchor_of: BTreeMap<CenterId, CenterId>,
    /// ★#679 D1b：**稳定锚 → 当前身份**（`anchor_of` 的逆）。重基核对时用它取「这个锚现在
    /// 应该长什么样」，再拿去和新链比对。
    current_of: BTreeMap<CenterId, CenterId>,
}

impl CenterOscillationBook {
    pub fn new(level: u32) -> Self {
        Self {
            level,
            suspended: BTreeMap::new(),
            anchor_of: BTreeMap::new(),
            current_of: BTreeMap::new(),
        }
    }

    /// ★#679 D1b：把**当前身份**折回**稳定锚**。未经谱系迁移的身份恒等返回。
    ///
    /// 这是挂起表的唯一 key 归一入口：触发、生命周期事件、挂起查询三条路径都先经它，
    /// 于是「重基后拿新身份来的事件」与「拿首次绑定旧框来的历史事件」命中同一条挂起。
    pub fn resolve(&self, id: CenterId) -> CenterId {
        self.anchor_of.get(&id).copied().unwrap_or(id)
    }

    /// 该锚两侧都不再挂起 ⟹ 回收它的别名（防止别名表随时间单调增长，也防已终结的
    /// 锚把后来的同值新身份误吸过去）。
    fn gc_anchor(&mut self, anchor: CenterId) {
        if self.suspended.keys().any(|(_, id)| *id == anchor) {
            return;
        }
        if let Some(cur) = self.current_of.remove(&anchor) {
            self.anchor_of.remove(&cur);
        }
    }

    pub const fn level(&self) -> u32 {
        self.level
    }

    /// 当前挂起的身份数（诊断/测试用）。
    pub fn suspended_count(&self) -> usize {
        self.suspended.len()
    }

    /// ★#466 D0：确定序读取当时全部分侧挂起身份。只供 env-gated OPSEM 重基事务观测；
    /// 返回副本，调用方不能借此改挂起表。
    pub fn suspended_identities(&self) -> Vec<(VoiceSide, CenterId)> {
        self.suspended.keys().copied().collect()
    }

    /// ★#487：仍有挂起绑定的完整旧框，按 `CenterId` 去重、确定序返回。
    ///
    /// 同一中枢若两侧先后挂起，第二侧沿用第一侧已经冻结的框；因此这里不会把一个身份投成两个
    /// 时间框。旧 API 产生的无框挂起不冒充四边证候选。
    pub fn suspended_frames(&self) -> Vec<Center> {
        let mut frames = BTreeMap::new();
        for ((_, id), frame) in self.suspended.iter() {
            if let Some(center) = frame {
                frames.entry(*id).or_insert(*center);
            }
        }
        frames.into_values().collect()
    }

    /// 多头侧挂起查询（#381 前既有语义；分侧查询见 [`Self::is_suspended_side`]）。
    pub fn is_suspended(&self, center: CenterId) -> bool {
        self.is_suspended_side(VoiceSide::Long, center)
    }

    /// ★#381：分侧挂起查询。★#679 D1b：入参先经 [`Self::resolve`] 折回稳定锚——重基后拿
    /// 新身份来问与拿首次绑定旧身份来问，答案相同。
    pub fn is_suspended_side(&self, side: VoiceSide, center: CenterId) -> bool {
        self.suspended.contains_key(&(side, self.resolve(center)))
    }

    /// 处理一次触发 ⟹ 减/补动作二选一，或幽灵回补拒绝（`None`）。
    ///
    /// - 上沿高抛（`Above`）：无论此前是否已挂起，均记一次 `Reduce`（重复触碰上沿=继续高抛，
    ///   挂起态置位/保持）。
    /// - 下沿回补（`Below`）：**仅当**该中枢身份当前处于挂起中才放行 `Replenish`（回补出口之一）
    ///   ——否则返回 `None`（P6 反例锚：全平/毁中枢后无本级买点证书不许回补，即「幽灵回补」）。
    ///
    /// **无门**：本函数不读取、也无法读取任何次级别账户状态——签名唯一输入是触发事实本身。
    ///
    /// ★#381：本方法=多头侧入口（语义逐字节不变），空头侧走 [`Self::on_trigger_side`]。
    pub fn on_trigger(&mut self, trigger: CenterOscillationTrigger) -> Option<CenterOscillationAction> {
        self.on_trigger_side(VoiceSide::Long, trigger)
    }

    /// ★#381：分侧触发处理——空头侧是多头侧的**镜像**（票面「减=回补空头、补=加回空头」）。
    ///
    /// | 持仓侧 | 开局腿（`Reduce`） | 收口腿（`Replenish`，受挂起门） |
    /// |---|---|---|
    /// | `Long` | 上沿高抛卖出 | 下沿回补买入 |
    /// | `Short` | **下沿**回补空头（买回，空头获利了结） | **上沿**加回空头（重新卖空） |
    ///
    /// 两侧共用同一条判据结构（开局腿无门恒放行并置位挂起；收口腿仅当该侧该身份在挂起中才
    /// 放行，禁「幽灵回补/幽灵加空」），只是开局边沿相反——`Flat` 无 campaign 可记，视同
    /// 多头侧不构造（调用方 `fill.rs::step_center_oscillation` 只喂 `Long`/`Short`）。
    pub fn on_trigger_side(
        &mut self,
        side: VoiceSide,
        trigger: CenterOscillationTrigger,
    ) -> Option<CenterOscillationAction> {
        self.on_trigger_side_impl(side, trigger, None)
    }

    /// ★#487 生产接线入口：开局腿把完整中枢框绑定到挂起；重复触发只延续首次冻结框。
    pub fn on_trigger_side_bound(
        &mut self,
        side: VoiceSide,
        trigger: CenterOscillationTrigger,
        center: Center,
    ) -> Option<CenterOscillationAction> {
        assert_eq!(
            CenterId::of(&center),
            trigger.center(),
            "挂起四边框必须与触发 CenterId 精确同源"
        );
        self.on_trigger_side_impl(side, trigger, Some(center))
    }

    fn on_trigger_side_impl(
        &mut self,
        side: VoiceSide,
        trigger: CenterOscillationTrigger,
        center: Option<Center>,
    ) -> Option<CenterOscillationAction> {
        assert_eq!(
            trigger.level(),
            self.level,
            "触发级别必须匹配本机级别（跨级误喂是接线错误，不是本机决策范围）"
        );
        debug_assert!(
            !matches!(side, VoiceSide::Flat),
            "★#381：`Flat` 无 campaign 可记，不是本机的合法输入侧——调用方只喂 Long/Short。\
             此处显式钉死，避免「静默按多头处理」把接线错误伪装成正常多头行为"
        );
        // 开局边沿：多头=上沿（高抛），空头=下沿（回补空头）——镜像的唯一分歧点。
        let open_edge = match side {
            VoiceSide::Short => BoundarySide::Below,
            VoiceSide::Long | VoiceSide::Flat => BoundarySide::Above,
        };
        // ★#679 D1b：触发带的是**新链当前身份**，挂起表键是首次绑定的稳定锚——先折回。
        // 未经谱系迁移时 `resolve` 恒等 ⟹ 逐字节与旧行为相同。
        let anchor = self.resolve(trigger.center());
        if trigger.boundary_side() == open_edge {
            let frozen = self
                .suspended
                .iter()
                .find_map(|((_, id), frame)| (*id == anchor).then_some(*frame).flatten())
                .or(center);
            self.suspended
                .entry((side, anchor))
                .and_modify(|frame| {
                    if frame.is_none() {
                        *frame = frozen;
                    }
                })
                .or_insert(frozen);
            Some(CenterOscillationAction::Reduce)
        } else if self.suspended.remove(&(side, anchor)).is_some() {
            self.gc_anchor(anchor);
            Some(CenterOscillationAction::Replenish)
        } else {
            None
        }
    }

    /// 消费一条 T1 中枢生命周期事件 ⟹ 0 或多条终结/延续产出。Reset 只广播，恒为 no-op；
    /// Broken 至多终结同一身份的两侧挂起，Superseded 至多延续两侧挂起。
    ///
    /// 身份匹配走 `event.killed_center_id()`（D 裁定：按身份，不按当前在场）；`Born` 事件与
    /// 未挂起的身份均 no-op（C 裁定：幂等——目标身份不在挂起表中，天然产出空 Vec）。
    ///
    /// ★#381：挂起表分侧后，同一事件可同时终结两侧的挂起——逐侧各产一条
    /// [`SuspensionOutcome`]（带 `side`）。收手回补按侧**镜像**：多头侧于三类**买**点收手回补
    /// （结构续涨须补回货，既有口径不动），空头侧于三类**卖**点收手加回空头（结构续跌须补回
    /// 空头）；反向的那类点各自只终结不回补（多头「三卖不回补」的镜像）。
    ///
    /// ★★#414（ADR 补充十一）：`Superseded` 改走**延续**支——不摘挂起、不清算，逐侧产一条
    /// [`SuspensionContinuation`]（`continuations`）。产出类型因此从 `Vec<SuspensionOutcome>`
    /// 改为 [`SuspensionEventOutcome`]（终结/延续两路分列，理由见该类型文档）。
    ///
    /// ★**有效域（如实标注，090）**：#487 后生产接线把首次挂起的完整旧框交给四边配对；配对证
    /// 通过且精确 `CenterId` 仍在挂起表时，即使已滑出 `prev_slot` 也经 historical-bound 产
    /// `Broken`。没有完整框的旧 API 挂起、框配对失败、或重基已让身份消失者不获此授权；前两者
    /// 继续等待，后者仍由 `RebaseVanished` 工程警报出口处置。
    pub fn on_lifecycle_event(&mut self, event: &CenterLifecycleEvent) -> SuspensionEventOutcome {
        let Some(raw_id) = event.killed_center_id() else {
            // Born 或 Reset（Reset 即使带活中枢漏发见证，也没有死亡身份）。
            return SuspensionEventOutcome::default();
        };
        // ★#679 D1b：折回稳定锚——重基迁移后事件带的是新链身份，挂起表的键仍是首次绑定锚。
        let id = self.resolve(raw_id);
        // ★#414：被取代 ⟹ 挂起**延续**（不摘表、不清算），逐侧产一条延续记录。
        if matches!(event, CenterLifecycleEvent::Superseded { .. }) {
            return SuspensionEventOutcome {
                terminations: Vec::new(),
                continuations: [VoiceSide::Long, VoiceSide::Short]
                    .into_iter()
                    .filter(|side| self.suspended.contains_key(&(*side, id)))
                    .map(|side| SuspensionContinuation { center: id, side })
                    .collect(),
            };
        }
        let source = match event {
            CenterLifecycleEvent::Broken { breaker_side: Side::Long, .. } => {
                SuspensionTerminationSource::BrokenByThirdClassBuy
            }
            CenterLifecycleEvent::Broken { breaker_side: Side::Short, .. } => {
                SuspensionTerminationSource::BrokenByThirdClassSell
            }
            CenterLifecycleEvent::Superseded { .. }
            | CenterLifecycleEvent::Reset { .. }
            | CenterLifecycleEvent::Born { .. } => {
                return SuspensionEventOutcome::default();
            }
        };
        // 逐侧终结（该身份在某侧未挂起 ⟹ 该侧幂等 no-op，不产出）。侧序固定 Long→Short，
        // 与挂起表的 `BTreeMap` 迭代序同锚（确定性，H1 理由不变）。
        let terminations: Vec<SuspensionOutcome> = [VoiceSide::Long, VoiceSide::Short]
            .into_iter()
            .filter(|side| self.suspended.remove(&(*side, id)).is_some())
            .map(|side| SuspensionOutcome {
                center: id,
                source,
                cover_action: cover_action_for(side, source),
                side,
                settlement: settlement_for(side, source),
            })
            .collect();
        // ★#679 D1b：两侧都不再挂起 ⟹ 回收该锚的谱系别名（无迁移时恒为 no-op）。
        self.gc_anchor(id);
        SuspensionEventOutcome { terminations, continuations: Vec::new() }
    }

    /// 消费一次链**重基**（[`center_lifecycle::ChainConsumed::Rebased`]）⟹ 0 或多条终结产出
    /// （F 裁定，issue #292 续修）。
    ///
    /// 重基是工程再同步（前缀分叉/该级塔缓存全量重置），不是教义生死——本机不产 `Reset`/`Broken`/
    /// `Superseded` 之外的第三种教义事件，只按**身份**核对新链：
    /// - 挂起身份仍在新链上（任意下标，不要求仍是链尾/容读格）⟹ **跟随迁移**：挂起状态原样
    ///   保留（本机挂起表的键本身就是身份三元组，不含链下标侧车，迁移不改变任何字段，是
    ///   保状态的 no-op，故本函数不为它产出任何 `SuspensionOutcome`）。
    /// - 挂起身份不在新链上 ⟹ **终结**（[`SuspensionTerminationSource::RebaseVanished`]）：
    ///   不回补，按 #472 未闭合减出核销。
    ///
    /// **禁悬空（机检断言）**：处理后仍挂起的身份必须全部在新链上——不留「既非迁移又非终结」
    /// 的第三态；这是本函数的构造性不变量（逐身份要么留要么删），断言只是把它显式钉死。
    pub fn on_chain_rebase(&mut self, chain: &[Center]) -> Vec<SuspensionOutcome> {
        self.on_chain_rebase_lineage(chain, None).0
    }

    /// ★★#679 D1b：带**构造谱系证书**的重基核对——[`Self::on_chain_rebase`] 的超集。
    ///
    /// 判据分三层，逐层 fail closed：
    ///
    /// 1. 锚的**当前身份**（经 [`Self::current_of`]，首次重基前 = 锚自身）仍在新链上 ⟹ 原样保留
    ///    （既有「跟随迁移」语义，无需证书）；
    /// 2. 当前身份不在新链上 ⟹ 查 `lineage`。**只有**拿到
    ///    [`LineageVerdict::Continued`]（证书判 `relation=continued_1to1` 且
    ///    functional∧injective∧unique∧bijective 全真）**且**该新身份确实出现在重基后的新链上、
    ///    **且**没有第二个锚同时认领它，才迁移身份锚（`rebase_lineage_migrated`）；
    /// 3. 其余一切情形（无证书 / 歧义 / bar 不匹配 / 目标不在新链 / 重复认领 / `lineage=None`）
    ///    ⟹ 保持既有 `RebaseVanished` 核销（`rebase_vanished_kept`）+ 分桶计数。**禁 fail-open**。
    ///
    /// 迁移**只改身份锚**：挂起表的键、值（首次绑定冻结的四边框）、campaign 挂起归属账的来源
    /// 中枢标签一律不动——清算仍按 ADR 补充十四拿旧框判（教义口径，票 #679）。
    ///
    /// **禁悬空（机检断言）**：处理后仍挂起的每个锚，其当前身份都必须在新链上。
    pub fn on_chain_rebase_lineage(
        &mut self,
        chain: &[Center],
        lineage: Option<&dyn LineageLookup>,
    ) -> (Vec<SuspensionOutcome>, RebaseMigrationTally) {
        let mut tally = RebaseMigrationTally::default();
        if self.suspended.is_empty() {
            return (Vec::new(), tally);
        }
        let chain_ids: BTreeSet<CenterId> = chain.iter().map(CenterId::of).collect();
        // 锚集合（去侧去重，确定序——`BTreeMap` 键序）。判据不分侧：身份是否还在构造链上是
        // 结构事实，与持仓侧无关（#381 起两侧共用同一锚）。
        let mut anchors: Vec<CenterId> = Vec::new();
        for (_, id) in self.suspended.keys() {
            if !anchors.contains(id) {
                anchors.push(*id);
            }
        }

        let mut vanished_anchors: Vec<CenterId> = Vec::new();
        // 本次重基已被认领的新身份 → 认领它的锚（防两个旧锚认领同一个新身份）。
        let mut claimed: BTreeMap<CenterId, CenterId> = BTreeMap::new();
        // 先算全部迁移决定，再统一落账——避免边算边改 `current_of` 影响后续锚的判定。
        let mut migrations: Vec<(CenterId, CenterId, CenterId)> = Vec::new(); // (锚, 旧当前, 新当前)

        for anchor in anchors {
            let current = self.current_of.get(&anchor).copied().unwrap_or(anchor);
            if chain_ids.contains(&current) {
                continue; // 仍在链上：既有「跟随迁移」路径，一动不动。
            }
            let Some(book) = lineage else {
                // 反证臂（`THETA_REBASE_MIGRATE_SKIP=1`）或未接谱系簿 ⟹ 旧行为。
                tally.no_cert += 1;
                vanished_anchors.push(anchor);
                continue;
            };
            match book.lookup(current) {
                LineageVerdict::Continued(new_id) if !chain_ids.contains(&new_id) => {
                    // 证书说它延续到 new_id，但 new_id 不在生命周期层收到的新链上——两侧不一致，
                    // 不是可信的过继，照 fail-closed 走核销。
                    tally.target_absent += 1;
                    vanished_anchors.push(anchor);
                }
                LineageVerdict::Continued(new_id) => match claimed.entry(new_id) {
                    std::collections::btree_map::Entry::Vacant(v) => {
                        v.insert(anchor);
                        migrations.push((anchor, current, new_id));
                    }
                    std::collections::btree_map::Entry::Occupied(_) => {
                        // 重复 lineage claim：两个旧锚同时指向一个新身份 ⟹ 非单射，全部拒绝。
                        tally.duplicate_claim += 1;
                        vanished_anchors.push(anchor);
                    }
                },
                LineageVerdict::NoCert => {
                    tally.no_cert += 1;
                    vanished_anchors.push(anchor);
                }
                LineageVerdict::Ambiguous => {
                    tally.ambiguous += 1;
                    vanished_anchors.push(anchor);
                }
                LineageVerdict::BarMismatch => {
                    tally.bar_mismatch += 1;
                    vanished_anchors.push(anchor);
                }
            }
        }

        for (anchor, old_current, new_current) in migrations {
            if old_current != anchor {
                // 中间修订身份退役：它已不在链上，留着只会让别名表单调增长。
                self.anchor_of.remove(&old_current);
            }
            self.anchor_of.insert(new_current, anchor);
            self.current_of.insert(anchor, new_current);
            tally.migrated += 1;
        }

        // 核销：逐（侧, 锚）摘表，产 `RebaseVanished`（不回补，按 #472 未闭合减出核销）。
        let vanished_keys: Vec<(VoiceSide, CenterId)> = self
            .suspended
            .keys()
            .copied()
            .filter(|(_, id)| vanished_anchors.contains(id))
            .collect();
        for key in &vanished_keys {
            self.suspended.remove(key);
        }
        tally.kept = vanished_keys.len();
        for anchor in &vanished_anchors {
            self.gc_anchor(*anchor);
        }
        let outcomes: Vec<SuspensionOutcome> = vanished_keys
            .into_iter()
            .map(|(side, center)| SuspensionOutcome {
                center,
                source: SuspensionTerminationSource::RebaseVanished,
                cover_action: None,
                side,
                settlement: settlement_for(side, SuspensionTerminationSource::RebaseVanished),
            })
            .collect();
        debug_assert!(
            self.suspended.keys().all(|(_, id)| {
                chain_ids.contains(&self.current_of.get(id).copied().unwrap_or(*id))
            }),
            "禁悬空：重基核对后任何仍挂起的锚，其当前身份都必须在新链上（迁移分支的机检不变量）"
        );
        (outcomes, tally)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::types::{Center, Tick};

    fn cid(start_index: usize, zd: Tick, zg: Tick) -> CenterId {
        CenterId::of(&Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index, end_index: start_index + 50 })
    }

    fn broken(center_id: CenterId, breaker_side: Side) -> CenterLifecycleEvent {
        let c = Center {
            zd: center_id.zd,
            zg: center_id.zg,
            dd: center_id.zd - 2,
            gg: center_id.zg + 2,
            start_index: center_id.start_index,
            end_index: center_id.start_index + 50,
        };
        CenterLifecycleEvent::Broken { level: 0, center: c, chain_index: 0, breaker_source_index: 999, breaker_side }
    }

    fn superseded(center_id: CenterId) -> CenterLifecycleEvent {
        let c = Center {
            zd: center_id.zd,
            zg: center_id.zg,
            dd: center_id.zd - 2,
            gg: center_id.zg + 2,
            start_index: center_id.start_index,
            end_index: center_id.start_index + 50,
        };
        CenterLifecycleEvent::Superseded { level: 0, center: c, chain_index: 0, by_chain_index: 1 }
    }

    fn reset_with(died: Option<CenterId>) -> CenterLifecycleEvent {
        let died_center = died.map(|id| Center {
            zd: id.zd,
            zg: id.zg,
            dd: id.zd - 2,
            gg: id.zg + 2,
            start_index: id.start_index,
            end_index: id.start_index + 50,
        });
        CenterLifecycleEvent::Reset {
            level: 0,
            died_center,
            died_chain_index: died.map(|_| 0),
            trigger_source_index: 500,
            trigger_side: Side::Long,
        }
    }

    // ── 触发（A 裁定 + 边界映射） ──────────────────────────────────────────

    #[test]
    fn trigger_rejects_when_center_not_alive() {
        assert_eq!(
            CenterOscillationTrigger::new(0, None, CenterDrift::NoDownShift, VoiceSide::Long, 0, 10),
            Err(TriggerError::CenterNotAlive),
            "A 裁定：无主格即无触发"
        );
    }

    #[test]
    fn trigger_rejects_flat_signal() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Flat, 0, 10),
            Err(TriggerError::FlatSignal)
        );
    }

    #[test]
    fn trigger_maps_signal_side_to_boundary_same_convention_as_pan_div() {
        let id = cid(5, 100, 200);
        let buy = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, id.zd, 10).unwrap();
        assert_eq!(buy.boundary_side(), BoundarySide::Below, "次级别买点=下沿回补试探");
        let sell = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 11).unwrap();
        assert_eq!(sell.boundary_side(), BoundarySide::Above, "次级别卖点=上沿高抛试探");
    }

    // ── ★#366 判据对齐缠师原文：离开中枢（ZG/ZD）+ 中枢不下移，废中轴二分 ────────

    /// 状态①：向上离开中枢（`price >= ZG`）的卖点 ⟹ 高抛（`Above`）。80 课「中枢震荡的
    /// 卖点都是出现在向上离开中枢时」。
    #[test]
    fn sell_signal_leaving_center_upward_produces_above_boundary() {
        let id = cid(5, 100, 200); // 中枢区间 [100, 200]
        let t = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, 201, 10).unwrap();
        assert_eq!(t.boundary_side(), BoundarySide::Above);
    }

    /// ★废中轴二分的行为化断言：中枢**内部**（旧判据的「上半区」，中轴 150 与 ZG 200 之间）
    /// 的卖点在新判据下**被拒**——旧判据放行 151，新判据要求 `price >= ZG=200`。这条测试就是
    /// 「上半区/下半区」自研规则被废除的可执行证据（090：自研规则不得冒充缠论判据）。
    #[test]
    fn sell_signal_inside_center_upper_half_is_rejected_after_366() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, 151, 10),
            Err(TriggerError::PriceOutsideZone),
            "中轴以上但未离开中枢 ⟹ 新判据拒绝（旧中轴二分会放行）"
        );
    }

    /// 状态②：未向上离开中枢的卖点 ⟹ 整体拒绝（不是"仍构造但方向作废"）。
    #[test]
    fn sell_signal_without_leaving_center_is_rejected() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, 99, 10),
            Err(TriggerError::PriceOutsideZone)
        );
    }

    /// 状态③：向下离开中枢（`price <= ZD`）+ 中枢不下移的买点 ⟹ 回补（`Below`）。
    /// 33 课「每次向下离开中枢只要出现底背驰，那就可以介入」。
    #[test]
    fn buy_signal_leaving_center_downward_produces_below_boundary() {
        let id = cid(5, 100, 200);
        let t = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, 99, 10).unwrap();
        assert_eq!(t.boundary_side(), BoundarySide::Below);
    }

    /// ★废中轴二分（买侧）：中枢内部的「下半区」买点（中轴 150 与 ZD 100 之间）新判据下被拒。
    #[test]
    fn buy_signal_inside_center_lower_half_is_rejected_after_366() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, 149, 10),
            Err(TriggerError::PriceOutsideZone),
            "中轴以下但未离开中枢 ⟹ 新判据拒绝（旧中轴二分会放行）"
        );
    }

    /// 状态④：未向下离开中枢的买点 ⟹ 整体拒绝。
    #[test]
    fn buy_signal_without_leaving_center_is_rejected() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, 201, 10),
            Err(TriggerError::PriceOutsideZone)
        );
    }

    /// 边沿恰等（`price == ZG` / `price == ZD`）计入离开（含端点，非开区间——实现决策，可复检）。
    #[test]
    fn price_exactly_at_center_edge_counts_as_leaving() {
        let id = cid(5, 100, 200);
        let sell = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap();
        assert_eq!(sell.boundary_side(), BoundarySide::Above, "price == ZG 计入向上离开（>=）");
        let buy = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, id.zd, 11).unwrap();
        assert_eq!(buy.boundary_side(), BoundarySide::Below, "price == ZD 计入向下离开（<=）");
    }

    /// ★#366「中枢不下移」前置过滤（89 课）：价格已向下离开中枢、底背驰在，但中枢已下移
    /// ⟹ 拒绝回补（下移=趋势延续，非震荡回补场景）。
    #[test]
    fn buy_signal_is_rejected_when_center_moved_down() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::MovedDown, VoiceSide::Long, 99, 10),
            Err(TriggerError::CenterMovedDown),
            "89 课：形成中枢下移则不在次级别底背驰处买"
        );
    }

    /// 「中枢不下移」只约束**回补侧**——高抛侧无对应原文条件，不臆造对称门。
    #[test]
    fn sell_signal_is_unaffected_by_center_moved_down() {
        let id = cid(5, 100, 200);
        let t = CenterOscillationTrigger::new(0, Some(id), CenterDrift::MovedDown, VoiceSide::Short, 201, 10)
            .expect("下移不约束高抛侧");
        assert_eq!(t.boundary_side(), BoundarySide::Above);
    }

    /// 判据顺序（读数纯净性）：价格根本不在下沿的买点即便遇下移，也报 `PriceOutsideZone`
    /// 而非 `CenterMovedDown`——`CenterMovedDown` 桶只计「本可成立的回补被下移否掉」。
    #[test]
    fn position_predicate_precedes_down_shift_filter_in_error_attribution() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), CenterDrift::MovedDown, VoiceSide::Long, 150, 10),
            Err(TriggerError::PriceOutsideZone),
            "位置不成立优先归因，避免污染下移桶读数"
        );
    }

    // ── 减补动作 ──────────────────────────────────────────────────────────

    #[test]
    fn upper_boundary_touch_emits_reduce_and_opens_suspension() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let t = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap();
        assert_eq!(book.on_trigger(t), Some(CenterOscillationAction::Reduce));
        assert!(book.is_suspended(id));
    }

    #[test]
    fn repeated_upper_boundary_touches_keep_emitting_reduce() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let t1 = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap();
        let t2 = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 20).unwrap();
        assert_eq!(book.on_trigger(t1), Some(CenterOscillationAction::Reduce));
        assert_eq!(book.on_trigger(t2), Some(CenterOscillationAction::Reduce), "重复触碰上沿=继续高抛");
        assert!(book.is_suspended(id));
    }

    #[test]
    fn lower_boundary_touch_while_suspended_covers() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let reduce = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap();
        let cover = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, id.zd, 20).unwrap();
        book.on_trigger(reduce);
        assert_eq!(book.on_trigger(cover), Some(CenterOscillationAction::Replenish));
        assert!(!book.is_suspended(id), "回补出口=挂起清空");
    }

    /// ★幽灵回补=0（原型 P6 反例锚）：未挂起时下沿信号不产任何动作。
    #[test]
    fn ghost_replenish_without_suspension_is_rejected() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let cover = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, id.zd, 20).unwrap();
        assert_eq!(book.on_trigger(cover), None, "无挂起证书 ⟹ 禁复活");
        assert_eq!(book.suspended_count(), 0);
    }

    // ── 挂起出口二分（5 场景，票面「验收」条款） ───────────────────────────

    /// 出口①：下沿回补（正常出口）——已在 `lower_boundary_touch_while_suspended_covers` 覆盖，
    /// 此处补一条端到端多轮（高抛→回补→再高抛→再回补）确认状态机可重复进出。
    #[test]
    fn suspension_can_reopen_after_a_full_cover_cycle() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let reduce = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap();
        let cover = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Long, id.zd, 20).unwrap();
        assert_eq!(book.on_trigger(reduce), Some(CenterOscillationAction::Reduce));
        assert_eq!(book.on_trigger(cover), Some(CenterOscillationAction::Replenish));
        assert_eq!(book.on_trigger(reduce), Some(CenterOscillationAction::Reduce), "回补后可再度高抛");
        assert_eq!(book.on_trigger(cover), Some(CenterOscillationAction::Replenish));
    }

    /// 出口②：三类买点处收手回补——终结伴随一次真实回补（幽灵回补=0 的正面形态：回补来自
    /// 这条破坏证书本身，不是凭空生成）。
    #[test]
    fn suspension_exit_third_class_buy_terminates_with_cover() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let outcomes = book.on_lifecycle_event(&broken(id, Side::Long)).terminations;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::BrokenByThirdClassBuy);
        assert_eq!(outcomes[0].cover_action, Some(CenterOscillationAction::Replenish));
        assert!(!book.is_suspended(id), "终结=挂起清空");
    }

    /// ★#489：Reset 只广播走势类型终结，不终结挂起、不回补、不核销。
    #[test]
    fn reset_broadcast_keeps_suspension_pending() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let out = book.on_lifecycle_event(&reset_with(Some(id)));
        assert!(out.terminations.is_empty(), "Reset 不是中枢死亡证明 ⟹ 不产终结");
        assert!(out.continuations.is_empty(), "Reset 不是中枢更替 ⟹ 不伪造延续事件");
        assert!(book.is_suspended(id), "Reset 广播后挂起仍绑定原中枢，等待自己的三类点");
    }

    /// ★#489 反例锁：Reset 广播不清空整场；当前主格与历史冻结框的挂起都继续等各自三类点。
    #[test]
    fn reset_broadcast_keeps_entire_level_suspension_book() {
        let stale_suspended = cid(5, 100, 200); // 早于当前在场的挂起遗留（D 裁定：按身份不按在场）
        let currently_alive = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(stale_suspended), CenterDrift::NoDownShift, VoiceSide::Short, stale_suspended.zg, 10).unwrap());
        book.on_trigger(CenterOscillationTrigger::new(0, Some(currently_alive), CenterDrift::NoDownShift, VoiceSide::Short, currently_alive.zg, 20).unwrap());
        assert_eq!(book.suspended_count(), 2);
        let outcomes = book.on_lifecycle_event(&reset_with(Some(currently_alive))).terminations;
        assert!(outcomes.is_empty(), "Reset 不是任一中枢的死亡证明 ⟹ 零终结");
        assert_eq!(book.suspended_count(), 2, "广播不清空当前或历史挂起");
        assert!(book.is_suspended(stale_suspended));
        assert!(book.is_suspended(currently_alive));
    }

    /// 出口④：三类卖点终结——不回补，按未闭合减出核销。
    #[test]
    fn suspension_exit_third_class_sell_terminates_without_cover() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let outcomes = book.on_lifecycle_event(&broken(id, Side::Short)).terminations;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::BrokenByThirdClassSell);
        assert_eq!(outcomes[0].cover_action, None);
        assert!(!book.is_suspended(id));
    }

    // ── ★★#414（ADR 补充十一）：Superseded 非终结——挂起延续到原中枢三类买卖点清算 ──────

    /// ★#414 核心用例（改判既有出口⑤）：被链推进取代**不终结**挂起——不产终结、不清算，
    /// 挂起原样留在表里，只产一条延续记录。教义：中枢终结唯一 = 三类买卖点，取代只是在场
    /// 中枢换了一个（中枢层更替），不是中枢的死。
    #[test]
    fn superseded_continues_suspension_instead_of_terminating() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let out = book.on_lifecycle_event(&superseded(id));
        assert!(out.terminations.is_empty(), "取代不是死法 ⟹ 零终结产出");
        assert_eq!(
            out.continuations,
            vec![SuspensionContinuation { center: id, side: VoiceSide::Long }],
            "命中的挂起逐侧产一条延续记录（观测面，不是归宿）"
        );
        assert!(book.is_suspended(id), "★挂起延续：不随取代清除，等原中枢三类点清算");
    }

    /// ★#414：延续后**原中枢的三类买点**到达 ⟹ 走 #366 闭合终局（收手回补，哪怕贵了）。
    /// 这是票面项 2「原中枢三类点出现时按两终局清算」的端到端最小证据（多头侧）。
    #[test]
    fn continued_suspension_settles_at_original_center_third_class_buy() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        assert!(book.on_lifecycle_event(&superseded(id)).terminations.is_empty());
        // 原中枢（区间已冻结为历史事实）的三类买点到达。
        let out = book.on_lifecycle_event(&broken(id, Side::Long));
        assert_eq!(out.terminations.len(), 1, "延续的挂起在原中枢三类点上终结");
        assert_eq!(out.terminations[0].settlement, TerminationSettlement::CoverAndClose);
        assert_eq!(
            out.terminations[0].cover_action,
            Some(CenterOscillationAction::Replenish),
            "三买 ⟹ 回补（73 课：不回补即可能错过中枢上移）"
        );
        assert!(!book.is_suspended(id));
    }

    /// ★#414：延续后**原中枢的三类卖点**到达 ⟹ 走 #366 核销终局（不回补，未闭合减出分列呈报）。
    #[test]
    fn continued_suspension_settles_at_original_center_third_class_sell() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        book.on_lifecycle_event(&superseded(id));
        let out = book.on_lifecycle_event(&broken(id, Side::Short));
        assert_eq!(out.terminations.len(), 1);
        assert_eq!(out.terminations[0].settlement, TerminationSettlement::WriteOffUnclosed);
        assert_eq!(out.terminations[0].cover_action, None, "三卖 ⟹ 不回补，核销");
        assert!(!book.is_suspended(id));
    }

    /// ★#414：取代事件命中**未挂起**的身份 ⟹ 既不终结也不延续（零产出），不编造观测读数。
    #[test]
    fn superseded_without_suspension_produces_nothing() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let out = book.on_lifecycle_event(&superseded(id));
        assert!(out.terminations.is_empty());
        assert!(out.continuations.is_empty(), "没有挂起可延续 ⟹ 不记延续");
    }

    /// ★#414：延续在**两侧**各自成立（多空并存时逐侧一条，不跨侧求和，ADR 补充九纪律）。
    #[test]
    fn superseded_continuation_is_recorded_per_side() {
        let mut book = CenterOscillationBook::new(0);
        let c = cid(0, 100, 200);
        let above = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Above, VoiceSide::Short, 1);
        let below = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Below, VoiceSide::Long, 2);
        book.on_trigger_side(VoiceSide::Long, above);
        book.on_trigger_side(VoiceSide::Short, below);
        let out = book.on_lifecycle_event(&superseded(c));
        assert_eq!(
            out.continuations,
            vec![
                SuspensionContinuation { center: c, side: VoiceSide::Long },
                SuspensionContinuation { center: c, side: VoiceSide::Short },
            ],
            "逐侧各一条（侧序固定 Long→Short，确定性）"
        );
        assert_eq!(book.suspended_count(), 2, "两侧挂起都延续");
    }

    /// 终结后幽灵回补=0；反过来，Reset 不是终结，故同一笔真实挂起仍允许随后下沿回补。
    /// ★#414：`superseded` 从本表移除——它已不是终结出口（延续后挂起仍在，下沿信号本就该
    /// 放行回补，那正是「延续等清算」的正常形态，见 `superseded_continues_suspension_*`）。
    #[test]
    fn doctrinal_termination_rejects_ghost_while_reset_keeps_real_cover_live() {
        let id = cid(5, 100, 200);
        let cover_attempt = CenterOscillationTrigger::new(
            0,
            Some(id),
            CenterDrift::NoDownShift,
            VoiceSide::Long,
            id.zd,
            30,
        )
        .unwrap();

        let mut terminated = CenterOscillationBook::new(0);
        terminated.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        terminated.on_lifecycle_event(&broken(id, Side::Short));
        assert_eq!(terminated.on_trigger(cover_attempt), None, "三类点终结后拒绝幽灵回补");

        let mut broadcast_only = CenterOscillationBook::new(0);
        broadcast_only.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        broadcast_only.on_lifecycle_event(&reset_with(Some(id)));
        assert_eq!(
            broadcast_only.on_trigger(cover_attempt),
            Some(CenterOscillationAction::Replenish),
            "Reset 后仍是原挂起的真实回补，不是幽灵"
        );
    }


    // ── ★#366 清算两终局（补充裁定 2026-07-27）：三买闭合 / 三卖未闭合减出，分账不煮一锅 ──

    /// 多头侧：三类**买**点终局 ⟹ `CoverAndClose`（收手回补 → 往返闭合 → 转持股）；
    /// 三类**卖**点终局 ⟹ `WriteOffUnclosed`（不回补，挂起核销分列呈报）。两终局互斥且穷尽
    /// 覆盖教义三类点，不再混同为「一律终结放弃回补」。
    #[test]
    fn long_side_two_settlements_split_by_third_class_point_kind() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let buy_out = book.on_lifecycle_event(&broken(id, Side::Long)).terminations;
        assert_eq!(buy_out[0].settlement, TerminationSettlement::CoverAndClose);
        assert_eq!(buy_out[0].cover_action, Some(CenterOscillationAction::Replenish));

        let mut book2 = CenterOscillationBook::new(0);
        book2.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let sell_out = book2.on_lifecycle_event(&broken(id, Side::Short)).terminations;
        assert_eq!(
            sell_out[0].settlement,
            TerminationSettlement::WriteOffUnclosed,
            "三卖终局=未闭合减出核销，不是静默作废"
        );
        assert_eq!(sell_out[0].cover_action, None, "核销不构造回补动作");
    }

    /// 空头侧镜像：三类**卖**点 ⟹ 闭合（加回空头）；三类**买**点 ⟹ 未闭合减出核销。
    #[test]
    fn short_side_two_settlements_are_mirror_of_long_side() {
        let c = cid(0, 100, 200);
        let below = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Below, VoiceSide::Long, 2);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger_side(VoiceSide::Short, below);
        let sell_out = book.on_lifecycle_event(&broken(c, Side::Short)).terminations;
        assert_eq!(sell_out[0].settlement, TerminationSettlement::CoverAndClose);

        let mut book2 = CenterOscillationBook::new(0);
        book2.on_trigger_side(VoiceSide::Short, below);
        let buy_out = book2.on_lifecycle_event(&broken(c, Side::Long)).terminations;
        assert_eq!(buy_out[0].settlement, TerminationSettlement::WriteOffUnclosed);
    }

    /// ★#489：撤销 #472 的 Reset 核销半边；RebaseVanished 身份永久消失，仍按终态核销。
    #[test]
    fn reset_does_not_settle_while_rebase_vanished_still_writes_off_unclosed() {
        let id = cid(5, 100, 200);
        let mut reset_book = CenterOscillationBook::new(0);
        reset_book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let reset_out = reset_book.on_lifecycle_event(&reset_with(Some(id))).terminations;
        assert!(reset_out.is_empty(), "Reset 不产生任何清算终局");
        assert!(reset_book.is_suspended(id), "Reset 后挂起继续等待三类点");

        let mut rebase_book = CenterOscillationBook::new(0);
        rebase_book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let rebase_out = rebase_book.on_chain_rebase(&[]);
        assert_eq!(rebase_out.len(), 1);
        assert_eq!(rebase_out[0].source, SuspensionTerminationSource::RebaseVanished);
        assert_eq!(rebase_out[0].settlement, TerminationSettlement::WriteOffUnclosed);
        assert!(!rebase_book.is_suspended(id), "身份消失后按终态核销并摘挂起");
    }

    /// ★「高抛不回头时中枢未死，挂起继续等」（#366 补充裁定教义链）：无终结事件到达时，挂起
    /// 既不被回补也不被核销；只有 `RebaseVanished` 工程失踪另行走核销。
    #[test]
    fn suspension_keeps_waiting_while_center_alive() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        // Born 事件（中枢未死）——no-op，挂起原样。
        let out = book.on_lifecycle_event(&CenterLifecycleEvent::Born {
            level: 0,
            center: Center { zd: id.zd, zg: id.zg, dd: id.zd - 2, gg: id.zg + 2, start_index: id.start_index, end_index: id.start_index + 50 },
            chain_index: 0,
        }).terminations;
        assert!(out.is_empty(), "中枢未死 ⟹ 无终局产出");
        assert!(book.is_suspended(id), "挂起继续等，不提前终局");
    }

    // ── 幂等（C 裁定） ────────────────────────────────────────────────────

    /// ★#414 改判：主流程常态（同一实例先收在场终结 superseded、后收教义死亡 broken）现在
    /// 是**延续 → 清算**两拍，不再是「终结 + no-op」。第二条信号才是真终局——这正是本票
    /// 「触发点从取代时改到三类点时」的时序证据。
    ///
    /// 幂等本身仍成立（`same_event_fed_twice_is_idempotent` 与本条第三拍）：清算之后同一
    /// 身份再来任何信号都是 no-op。
    #[test]
    fn superseded_then_doctrinal_death_settles_on_the_second_signal() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let first = book.on_lifecycle_event(&superseded(id));
        assert!(first.terminations.is_empty(), "第一条信号（取代）不终结");
        assert_eq!(first.continuations.len(), 1, "记一次延续");
        let second = book.on_lifecycle_event(&broken(id, Side::Long)).terminations;
        assert_eq!(second.len(), 1, "第二条信号（容读法放行的教义死亡）才是终局");
        let third = book.on_lifecycle_event(&broken(id, Side::Long)).terminations;
        assert!(third.is_empty(), "清算之后再来同一信号必须 no-op（幂等不变）");
    }

    /// 同一 Broken 事件被重复喂入（同型防御性重放）也必须幂等。
    #[test]
    fn same_event_fed_twice_is_idempotent() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        let ev = broken(id, Side::Long);
        let first = book.on_lifecycle_event(&ev).terminations;
        assert_eq!(first.len(), 1);
        let second = book.on_lifecycle_event(&ev).terminations;
        assert!(second.is_empty());
    }

    // ── 身份匹配（D 裁定） ────────────────────────────────────────────────

    /// 挂起按中枢身份匹配，不按当前在场：链已推进（新中枢在场）后，终结事件仍须命中它所指的
    /// 旧挂起身份，不会误终结新在场的那一个。
    #[test]
    fn suspension_matches_by_center_identity_not_current_alive() {
        let old_id = cid(5, 100, 200);
        let new_id = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(old_id), CenterDrift::NoDownShift, VoiceSide::Short, old_id.zg, 10).unwrap());
        // 链推进：新中枢在场，也开一笔挂起。
        book.on_trigger(CenterOscillationTrigger::new(0, Some(new_id), CenterDrift::NoDownShift, VoiceSide::Short, new_id.zg, 100).unwrap());
        assert_eq!(book.suspended_count(), 2);
        // 终结事件声明身份 = old_id（即便当前在场早已是 new_id）。
        let outcomes = book.on_lifecycle_event(&broken(old_id, Side::Short)).terminations;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].center, old_id, "命中的是载体所指的旧身份");
        assert!(!book.is_suspended(old_id));
        assert!(book.is_suspended(new_id), "新在场身份不受影响（连坐=违拒杀优先于错杀）");
    }

    // ── 接住首次教义死亡（E 裁定） ────────────────────────────────────────

    /// 即便某实例从未收到过在场终结（Superseded）记录，一旦收到教义死亡（Broken）也要正确
    /// 终结——不依赖「先 superseded 才能终结」的隐含前提。
    #[test]
    fn first_doctrinal_death_without_prior_supersede_terminates_correctly() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        // 无任何 superseded 事件历史，直接喂教义死亡。
        let outcomes = book.on_lifecycle_event(&broken(id, Side::Short)).terminations;
        assert_eq!(outcomes.len(), 1);
        assert!(!book.is_suspended(id));
    }

    // ── 链重基（F 裁定，issue #292 续修：禁悬空） ────────────────────────────

    fn center_of(id: CenterId) -> Center {
        Center {
            zd: id.zd,
            zg: id.zg,
            dd: id.zd - 2,
            gg: id.zg + 2,
            start_index: id.start_index,
            end_index: id.start_index + 50,
        }
    }

    /// 状态①跟随迁移：重基后新链仍含该挂起身份（哪怕不再是链尾/容读格，只要任意下标命中）
    /// ⟹ 挂起状态原样保留，不产任何终结。
    #[test]
    fn rebase_migrates_suspension_when_identity_still_on_new_chain() {
        let id = cid(5, 100, 200);
        let other = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        assert!(book.is_suspended(id));
        let new_chain = vec![center_of(id), center_of(other)];
        let outcomes = book.on_chain_rebase(&new_chain);
        assert!(outcomes.is_empty(), "身份仍在新链上⟹跟随迁移，不产终结");
        assert!(book.is_suspended(id), "迁移=挂起状态原样保留");
    }

    /// 状态②终结：重基后新链不再含该挂起身份 ⟹ 与 Superseded 同形态终结，不回补。
    #[test]
    fn rebase_terminates_suspension_when_identity_vanishes_from_new_chain() {
        let id = cid(5, 100, 200);
        let survivor = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        book.on_trigger(CenterOscillationTrigger::new(0, Some(survivor), CenterDrift::NoDownShift, VoiceSide::Short, survivor.zg, 20).unwrap());
        assert_eq!(book.suspended_count(), 2);
        let new_chain = vec![center_of(survivor)];
        let outcomes = book.on_chain_rebase(&new_chain);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].center, id);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::RebaseVanished);
        assert_eq!(outcomes[0].cover_action, None, "重基悬空终结不回补，按未闭合减出核销");
        assert!(!book.is_suspended(id), "终结=挂起清空");
        assert!(book.is_suspended(survivor), "存活身份不受连坐");
    }

    /// 状态③禁悬空机检：混合场景（多身份，部分迁移/部分终结）——处理后任何仍挂起的身份都
    /// 必须在新链上，逐一核对不留第三态；同时验证终结产出恰好覆盖消失的那些身份，不多不少。
    #[test]
    fn rebase_reconciliation_leaves_no_dangling_identity() {
        let stays = cid(5, 100, 200);
        let vanishes_a = cid(50, 150, 250);
        let vanishes_b = cid(80, 160, 260);
        let new_arrival = cid(900, 500, 600);
        let mut book = CenterOscillationBook::new(0);
        for (id, seed) in [(stays, 10usize), (vanishes_a, 20), (vanishes_b, 30)] {
            book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, seed).unwrap());
        }
        assert_eq!(book.suspended_count(), 3);
        let new_chain = vec![center_of(stays), center_of(new_arrival)];
        let outcomes = book.on_chain_rebase(&new_chain);
        let terminated: std::collections::BTreeSet<CenterId> =
            outcomes.iter().map(|o| o.center).collect();
        assert_eq!(
            terminated,
            [vanishes_a, vanishes_b].into_iter().collect(),
            "终结产出恰好=消失的身份集合，不多不少"
        );
        for o in &outcomes {
            assert_eq!(o.source, SuspensionTerminationSource::RebaseVanished);
            assert_eq!(o.cover_action, None);
        }
        let chain_ids: std::collections::BTreeSet<CenterId> =
            new_chain.iter().map(CenterId::of).collect();
        assert!(book.is_suspended(stays));
        assert!(!book.is_suspended(vanishes_a));
        assert!(!book.is_suspended(vanishes_b));
        assert_eq!(book.suspended_count(), 1, "只剩迁移的那一个");
        assert!(chain_ids.contains(&stays), "迁移身份必须在新链上（否则是第三态：悬空未归因）");
    }

    // ── ★#679 D1b：挂起随谱系迁移（LineageBook 接入） ─────────────────────────

    /// 测试用查簿桩：一张固定的 `旧身份 → 结论` 表。
    struct StubLineage(Vec<(CenterId, LineageVerdict)>);

    impl LineageLookup for StubLineage {
        fn lookup(&self, old: CenterId) -> LineageVerdict {
            self.0
                .iter()
                .find(|(k, _)| *k == old)
                .map(|(_, v)| *v)
                .unwrap_or(LineageVerdict::NoCert)
        }
    }

    /// 在 `book` 上挂起 `id`（多头持仓侧的开局腿 = 上沿高抛，触发源是次级别**卖**点），
    /// 并绑定完整四边框。
    fn suspend_bound(book: &mut CenterOscillationBook, id: CenterId, seed: usize) {
        let trigger = CenterOscillationTrigger::new(
            0,
            Some(id),
            CenterDrift::NoDownShift,
            VoiceSide::Short,
            id.zg,
            seed,
        )
        .unwrap();
        assert_eq!(
            book.on_trigger_side_bound(VoiceSide::Long, trigger, center_of(id)),
            Some(CenterOscillationAction::Reduce)
        );
    }

    /// ★迁移族：旧身份不在新链上，但有 1→1 构造证书且新身份确实在新链上 ⟹ **不核销**，
    /// 身份锚随谱系迁移；新旧两个身份都能问到同一条挂起（`resolve` 折回锚）。
    #[test]
    fn lineage_certificate_migrates_suspension_instead_of_writing_off() {
        let old = cid(5, 100, 200);
        let new = cid(5, 90, 200); // 同 seed 窗重算，第三源外缘修订 ⟹ zd 漂移（wf8 seq=3 型）
        let mut book = CenterOscillationBook::new(0);
        suspend_bound(&mut book, old, 10);
        let lineage = StubLineage(vec![(old, LineageVerdict::Continued(new))]);
        let (outcomes, tally) =
            book.on_chain_rebase_lineage(&[center_of(new)], Some(&lineage as &dyn LineageLookup));
        assert!(outcomes.is_empty(), "有 1→1 证书 ⟹ 不产 RebaseVanished");
        assert_eq!(tally, RebaseMigrationTally { migrated: 1, ..Default::default() });
        assert_eq!(book.suspended_count(), 1, "挂起保留");
        assert!(book.is_suspended(new), "新链身份可问到这条挂起（谱系别名）");
        assert!(book.is_suspended(old), "首次绑定身份仍可问到（历史绑定路径拿旧框来问）");
        assert_eq!(book.resolve(new), old, "身份锚 = 首次绑定身份，campaign 归属账标签不变");
    }

    /// ★迁移族续：迁移**只迁身份锚**——首次绑定冻结的四边框不动（ADR 补充十四：清算仍拿旧框判），
    /// 且迁移后拿**新链身份**来的教义死亡事件仍能终结这条挂起，清算落在旧锚上。
    #[test]
    fn migrated_suspension_keeps_frozen_frame_and_settles_on_the_stable_anchor() {
        let old = cid(5, 100, 200);
        let new = cid(5, 90, 200);
        let mut book = CenterOscillationBook::new(0);
        suspend_bound(&mut book, old, 10);
        let frozen_before = book.suspended_frames();
        let lineage = StubLineage(vec![(old, LineageVerdict::Continued(new))]);
        let _ = book.on_chain_rebase_lineage(&[center_of(new)], Some(&lineage as &dyn LineageLookup));
        assert_eq!(
            book.suspended_frames(),
            frozen_before,
            "迁移只迁身份锚：首次绑定冻结的四边框一个字段都不许被新修订框覆盖"
        );
        // 新链身份带来的三类买点破坏 ⟹ 终结，且 outcome 落在稳定锚上（= campaign 批次标签）。
        let out = book.on_lifecycle_event(&broken(new, Side::Long)).terminations;
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].center, old, "清算落在稳定锚，不落在重基后的新身份");
        assert_eq!(out[0].source, SuspensionTerminationSource::BrokenByThirdClassBuy);
        assert_eq!(book.suspended_count(), 0);
        assert_eq!(book.resolve(new), new, "锚回收后别名一并退役，不留悬挂映射");
    }

    /// ★拒迁族：证书判非连续（split/removed/歧义）或干脆没有 ⟹ 保持既有 `RebaseVanished` 核销。
    #[test]
    fn non_continued_certificate_keeps_rebase_vanished_write_off() {
        let old = cid(5, 100, 200);
        let survivor = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        suspend_bound(&mut book, old, 10);
        suspend_bound(&mut book, survivor, 20);
        // 簿里根本没有 old 的连续边（= wf8 seq=66 型：seed 右移，构造窗撤出）。
        let lineage = StubLineage(vec![]);
        let (outcomes, tally) = book.on_chain_rebase_lineage(
            &[center_of(survivor)],
            Some(&lineage as &dyn LineageLookup),
        );
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].center, old);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::RebaseVanished);
        assert_eq!(outcomes[0].cover_action, None, "核销不回补（#472 未闭合减出）");
        assert_eq!(
            tally,
            RebaseMigrationTally { kept: 1, no_cert: 1, ..Default::default() },
            "拒迁必须有确定归因，不留无归因的核销"
        );
        assert!(!book.is_suspended(old));
        assert!(book.is_suspended(survivor), "存活身份不受连坐");
    }

    /// ★fail-closed 族：四条「有证书但不可信」的路径逐条拒绝过继，各自分桶。
    ///
    /// - `target_absent`：证书说延续到 X，但生命周期层收到的新链里没有 X；
    /// - `ambiguous`：同一旧身份有多个候选后继；
    /// - `bar_mismatch`：簿记 bar 与本次重基不符（跨 bar 不做猜测）；
    /// - `duplicate_claim`：两个旧锚同时认领同一个新身份（非单射）。
    #[test]
    fn fail_closed_paths_refuse_adoption_and_bucket_each_reason() {
        let ghost = cid(5, 100, 200); // 证书指向一个不在新链上的身份
        let ambiguous = cid(50, 150, 250);
        let stale_bar = cid(80, 160, 260);
        let claim_a = cid(90, 170, 270);
        let claim_b = cid(95, 175, 275);
        let survivor = cid(700, 300, 400);
        let contested = cid(701, 310, 410); // claim_a / claim_b 都指向它
        let mut book = CenterOscillationBook::new(0);
        for (i, id) in [ghost, ambiguous, stale_bar, claim_a, claim_b, survivor]
            .into_iter()
            .enumerate()
        {
            suspend_bound(&mut book, id, 10 + i);
        }
        let lineage = StubLineage(vec![
            (ghost, LineageVerdict::Continued(cid(999, 1, 2))), // 不在新链上
            (ambiguous, LineageVerdict::Ambiguous),
            (stale_bar, LineageVerdict::BarMismatch),
            (claim_a, LineageVerdict::Continued(contested)),
            (claim_b, LineageVerdict::Continued(contested)),
        ]);
        let new_chain = vec![center_of(survivor), center_of(contested)];
        let (outcomes, tally) =
            book.on_chain_rebase_lineage(&new_chain, Some(&lineage as &dyn LineageLookup));
        // claim_a 先到先得（`BTreeMap` 键序确定），claim_b 判重复认领。
        let terminated: std::collections::BTreeSet<CenterId> =
            outcomes.iter().map(|o| o.center).collect();
        assert_eq!(
            terminated,
            [ghost, ambiguous, stale_bar, claim_b].into_iter().collect(),
            "四条 fail-closed 路径全部保持核销"
        );
        assert_eq!(
            tally,
            RebaseMigrationTally {
                migrated: 1,
                kept: 4,
                no_cert: 0,
                target_absent: 1,
                ambiguous: 1,
                bar_mismatch: 1,
                duplicate_claim: 1,
            }
        );
        assert!(book.is_suspended(contested), "唯一合法认领者迁移成功");
        assert_eq!(book.resolve(contested), claim_a);
        assert!(book.is_suspended(survivor), "本来就在链上的不受影响");
        assert_eq!(book.suspended_count(), 2);
    }

    /// ★反证开关：`lineage=None`（生产由 `THETA_REBASE_MIGRATE_SKIP=1` 触发）⟹ 逐字与旧行为相同。
    #[test]
    fn skip_switch_restores_legacy_rebase_vanished_behaviour() {
        let old = cid(5, 100, 200);
        let new = cid(5, 90, 200);
        let mut with_book = CenterOscillationBook::new(0);
        let mut without = CenterOscillationBook::new(0);
        suspend_bound(&mut with_book, old, 10);
        suspend_bound(&mut without, old, 10);
        let lineage = StubLineage(vec![(old, LineageVerdict::Continued(new))]);
        let (migrated, _) =
            with_book.on_chain_rebase_lineage(&[center_of(new)], Some(&lineage as &dyn LineageLookup));
        let (legacy, tally) = without.on_chain_rebase_lineage(&[center_of(new)], None);
        assert!(migrated.is_empty());
        assert_eq!(legacy.len(), 1, "SKIP ⟹ 仍按旧行为核销");
        assert_eq!(legacy[0].source, SuspensionTerminationSource::RebaseVanished);
        assert_eq!(tally, RebaseMigrationTally { kept: 1, no_cert: 1, ..Default::default() });
        // 旧入口（无 lineage 参数）与 SKIP 臂同语义。
        let mut plain = CenterOscillationBook::new(0);
        suspend_bound(&mut plain, old, 10);
        assert_eq!(plain.on_chain_rebase(&[center_of(new)]).len(), 1);
    }

    /// ★多跳：连续两次重基各有一条 1→1 证书 ⟹ 锚一路不变，中间修订身份的别名退役。
    #[test]
    fn successive_rebases_chain_through_the_same_stable_anchor() {
        let a = cid(5, 100, 200);
        let b = cid(5, 90, 200);
        let c = cid(5, 80, 200);
        let mut book = CenterOscillationBook::new(0);
        suspend_bound(&mut book, a, 10);
        let l1 = StubLineage(vec![(a, LineageVerdict::Continued(b))]);
        let _ = book.on_chain_rebase_lineage(&[center_of(b)], Some(&l1 as &dyn LineageLookup));
        let l2 = StubLineage(vec![(b, LineageVerdict::Continued(c))]);
        let (out, tally) =
            book.on_chain_rebase_lineage(&[center_of(c)], Some(&l2 as &dyn LineageLookup));
        assert!(out.is_empty());
        assert_eq!(tally.migrated, 1);
        assert_eq!(book.resolve(c), a, "锚恒为首次绑定身份");
        assert_eq!(book.resolve(b), b, "中间修订身份的别名已退役（不留单调增长的映射）");
        assert!(book.is_suspended(c));
        assert!(book.is_suspended(a));
    }

    /// 空挂起表上的重基是 no-op（不 panic，不产任何终结）——防御性边界。
    #[test]
    fn rebase_on_empty_suspension_book_is_noop() {
        let mut book = CenterOscillationBook::new(0);
        let new_chain = vec![center_of(cid(5, 100, 200))];
        let outcomes = book.on_chain_rebase(&new_chain);
        assert!(outcomes.is_empty());
        assert_eq!(book.suspended_count(), 0);
    }

    // ── 无门（机械断言） ──────────────────────────────────────────────────

    /// 无互斥门：两个级别的账各自独立，互不读对方状态——本级触发/终结只影响本级挂起表。
    /// （编译期证据见 `on_trigger`/`on_lifecycle_event` 签名不接受任何跨级/次级别账户参数；
    /// 本测试补充运行期证据：级别 0 的全部操作对级别 1 的挂起表零影响。）
    #[test]
    fn no_mutex_gate_between_levels_each_book_is_independent() {
        let id = cid(5, 100, 200);
        let mut book0 = CenterOscillationBook::new(0);
        let book1 = CenterOscillationBook::new(1);
        book0.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap());
        book0.on_lifecycle_event(&broken(id, Side::Long));
        assert_eq!(book1.suspended_count(), 0, "本级动作对另一级别挂起表零影响");
    }

    #[test]
    #[should_panic(expected = "触发级别必须匹配本机级别")]
    fn trigger_level_mismatch_is_a_wiring_error_not_silent() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(1);
        let t = CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, 10).unwrap();
        book.on_trigger(t);
    }

    // ── #292 接线点二：PanDivTrigger → CenterOscillationTrigger 转换 ─────────

    use super::super::oscillation::{ConsolidationDivergenceEvidence, OscillationCenterRef, OscillationEvidenceRef};

    fn pan_div(level: u32, signal_side: VoiceSide, source_index: usize) -> PanDivTrigger {
        PanDivTrigger::from_gated_pan_div(
            level,
            signal_side,
            OscillationCenterRef::new(10, 30), // 坐标投影：与 CenterId 无关（A 裁定不复用它）
            ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(source_index, 0)),
        )
        .unwrap()
    }

    /// 五要素映射：级别/信号方向/盘背证据(source_index) 原样传导；中枢身份取调用方传入的
    /// `alive`（不取 `pan_div.center()` 坐标投影——A 裁定单一在场判据）。
    #[test]
    fn pan_div_conversion_maps_five_elements() {
        let alive = cid(5, 100, 200);
        let pd = pan_div(3, VoiceSide::Short, 777);
        let t = CenterOscillationTrigger::from_pan_div_trigger(pd, Some(alive)).unwrap();
        assert_eq!(t.level(), 3, "级别原样传导");
        assert_eq!(t.signal_side(), VoiceSide::Short, "信号方向原样传导");
        assert_eq!(t.center(), alive, "中枢身份=调用方传入的 alive，非 pan_div 坐标投影");
        assert_eq!(t.source_index(), 777, "盘背证据降格为 source_index 原样传导");
    }

    /// 边界侧核对：转换产出的 boundary_side 与来源 `PanDivTrigger.boundary_side()` 必然相等
    /// （两处同一映射：Long→Below / Short→Above）。
    #[test]
    fn pan_div_conversion_boundary_side_matches_source() {
        let alive = cid(5, 100, 200);
        let buy = pan_div(1, VoiceSide::Long, 10);
        let buy_source_boundary = buy.boundary_side();
        let mapped_buy = CenterOscillationTrigger::from_pan_div_trigger(buy, Some(alive)).unwrap();
        assert_eq!(mapped_buy.boundary_side(), buy_source_boundary);
        assert_eq!(mapped_buy.boundary_side(), BoundarySide::Below);

        let sell = pan_div(1, VoiceSide::Short, 11);
        let sell_source_boundary = sell.boundary_side();
        let mapped_sell = CenterOscillationTrigger::from_pan_div_trigger(sell, Some(alive)).unwrap();
        assert_eq!(mapped_sell.boundary_side(), sell_source_boundary);
        assert_eq!(mapped_sell.boundary_side(), BoundarySide::Above);
    }

    /// A 裁定：本级无在场中枢（`alive=None`）⟹ 转换拒绝，与 `new` 同一失败面，不新增静默兜底。
    #[test]
    fn pan_div_conversion_rejects_when_no_alive_center() {
        let pd = pan_div(2, VoiceSide::Long, 20);
        assert_eq!(
            CenterOscillationTrigger::from_pan_div_trigger(pd, None),
            Err(TriggerError::CenterNotAlive)
        );
    }

    // ── H1 确定性回归：BTreeMap 迭代序 ────────────────────────────────────

    /// ★#489：多身份挂起面对 Reset 一律保留；乱序插入后身份枚举仍按 BTreeMap 确定序，
    /// 证明广播既不产终结，也不改挂起表。
    #[test]
    fn reset_broadcast_preserves_multi_suspension_book_and_deterministic_order() {
        let mid = cid(50, 100, 200);
        let low = cid(5, 10, 20);
        let high = cid(900, 500, 600);
        let mut book = CenterOscillationBook::new(0);
        // 刻意乱序插入（mid → high → low），核对输出与插入序无关，只与 CenterId 派生序有关。
        for (id, seed) in [(mid, 10usize), (high, 20), (low, 30)] {
            book.on_trigger(CenterOscillationTrigger::new(0, Some(id), CenterDrift::NoDownShift, VoiceSide::Short, id.zg, seed).unwrap());
        }
        let outcomes = book.on_lifecycle_event(&reset_with(None)).terminations;
        assert!(outcomes.is_empty(), "Reset 广播不产任何多身份终结");
        assert_eq!(
            book.suspended_identities(),
            vec![
                (VoiceSide::Long, low),
                (VoiceSide::Long, mid),
                (VoiceSide::Long, high),
            ],
            "挂起身份完整保留，且枚举序由 BTreeMap 固定"
        );
    }

    // ── #381：空头侧镜像触发（减=回补空头、补=加回空头） ─────────────────

    /// ★#381 核心用例：空头侧触发是多头侧的**镜像**——开局腿（`Reduce`=回补空头）落在
    /// **下沿**（`Below`，价格跌向 ZD 时空头获利了结），收口腿（`Replenish`=加回空头）落在
    /// **上沿**（`Above`）且同样受挂起门约束（未挂起的上沿触碰 ⟹ `None`，禁「幽灵加空」）。
    #[test]
    fn short_side_trigger_is_mirror_of_long_side() {
        let mut book = CenterOscillationBook::new(0);
        let c = cid(0, 100, 200);

        // 空头侧：上沿在未挂起时不放行（镜像多头侧「下沿未挂起不回补」的幽灵门）。
        let above = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Above, VoiceSide::Short, 1);
        assert_eq!(book.on_trigger_side(VoiceSide::Short, above), None, "空头侧未挂起 ⟹ 上沿不放行加空");

        // 空头侧开局腿=下沿 Reduce（回补空头）。
        let below = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Below, VoiceSide::Long, 2);
        assert_eq!(
            book.on_trigger_side(VoiceSide::Short, below),
            Some(CenterOscillationAction::Reduce),
            "空头侧下沿=减（回补空头），镜像多头侧上沿=减（高抛）"
        );
        assert!(book.is_suspended_side(VoiceSide::Short, c), "空头侧挂起置位");
        assert!(!book.is_suspended(c), "多头侧挂起表不受空头侧触发污染（分侧独立）");

        // 空头侧收口腿=上沿 Replenish（加回空头）。
        assert_eq!(
            book.on_trigger_side(VoiceSide::Short, above),
            Some(CenterOscillationAction::Replenish),
            "空头侧上沿=补（加回空头）"
        );
        assert!(!book.is_suspended_side(VoiceSide::Short, c), "收口后挂起清除");
    }

    /// ★#381：多空并存时两侧挂起表互不污染——同一中枢可同时在多头侧与空头侧各自挂起，
    /// 各自按各自的边沿收口。
    #[test]
    fn long_and_short_suspensions_are_independent_per_side() {
        let mut book = CenterOscillationBook::new(0);
        let c = cid(0, 100, 200);
        let above = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Above, VoiceSide::Short, 1);
        let below = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Below, VoiceSide::Long, 2);

        assert_eq!(book.on_trigger_side(VoiceSide::Long, above), Some(CenterOscillationAction::Reduce));
        assert_eq!(book.on_trigger_side(VoiceSide::Short, below), Some(CenterOscillationAction::Reduce));
        assert_eq!(book.suspended_count(), 2, "两侧各一条挂起（键含侧，不相互覆盖）");

        assert_eq!(
            book.on_trigger_side(VoiceSide::Long, below),
            Some(CenterOscillationAction::Replenish),
            "多头侧下沿收口"
        );
        assert_eq!(book.suspended_count(), 1, "只收多头侧那条");
        assert!(book.is_suspended_side(VoiceSide::Short, c), "空头侧挂起仍在");
    }

    /// ★#381：终结的收手回补对两侧**镜像**——多头侧收手于三类**买**点（结构续涨须补回货），
    /// 空头侧收手于三类**卖**点（结构续跌须加回空头）；反向的那类点各自只终结不回补。
    #[test]
    fn termination_cover_action_mirrors_by_side() {
        let mut book = CenterOscillationBook::new(0);
        let c = cid(0, 100, 200);
        let above = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Above, VoiceSide::Short, 1);
        let below = CenterOscillationTrigger::from_parts(0, c, BoundarySide::Below, VoiceSide::Long, 2);
        book.on_trigger_side(VoiceSide::Long, above);
        book.on_trigger_side(VoiceSide::Short, below);

        let outcomes = book.on_lifecycle_event(&broken(c, Side::Long)).terminations; // 三类买点破坏
        assert_eq!(outcomes.len(), 2, "两侧挂起同时终结");
        let long_out = outcomes.iter().find(|o| o.side == VoiceSide::Long).expect("多头侧产出");
        let short_out = outcomes.iter().find(|o| o.side == VoiceSide::Short).expect("空头侧产出");
        assert_eq!(
            long_out.cover_action,
            Some(CenterOscillationAction::Replenish),
            "多头侧三类买点收手回补（既有口径不变）"
        );
        assert_eq!(short_out.cover_action, None, "空头侧遇三类买点只终结不加回（镜像「三卖不回补」）");

        // 镜像方向：三类卖点 ⟹ 空头侧收手加回、多头侧只终结。
        let mut book2 = CenterOscillationBook::new(0);
        book2.on_trigger_side(VoiceSide::Long, above);
        book2.on_trigger_side(VoiceSide::Short, below);
        let outcomes2 = book2.on_lifecycle_event(&broken(c, Side::Short)).terminations;
        let long2 = outcomes2.iter().find(|o| o.side == VoiceSide::Long).expect("多头侧产出");
        let short2 = outcomes2.iter().find(|o| o.side == VoiceSide::Short).expect("空头侧产出");
        assert_eq!(long2.cover_action, None, "多头侧三类卖点不回补（既有口径不变）");
        assert_eq!(
            short2.cover_action,
            Some(CenterOscillationAction::Replenish),
            "空头侧三类卖点收手加回空头（镜像）"
        );
    }
}
