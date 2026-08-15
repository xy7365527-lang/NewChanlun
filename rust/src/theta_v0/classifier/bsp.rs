//! 买卖点 bit-vector 判据（reference-theta-v0.md:34-36）。
//!
//! ## 契约重锚（legacy Formal/BSPLabels + Strict/BSP → `Origin.BspClassification`）
//!
//! - 端点语义 ↔ `Origin.BspClassification.BspEndpoint`：相对中枢的拓扑/历史语义字段
//!   （side/center/divPair/brokeCenter/afterTypeOne/leftCenter/retracePrice/firstRetrace）。
//! - 三类判据 ↔ `Origin.BspClassification.{IsType1,IsType2,IsType3Buy,IsType3Sell}`（结构谓词）：
//!   · `IsType1` = `brokeCenter ∧ IsDivergence divPair`（破中枢 + 背驰，第24课第一类）。
//!   · `IsType2` = `afterTypeOne ∧ ¬brokeCenter`（1 类后回调，§10.1 第二类）。
//!   · `IsType3Buy` = `side=long ∧ leftCenter ∧ firstRetrace ∧ center.zg < retracePrice`（离开后回试不破 ZG）。
//! - bit-vector 输出 ↔ `BspClass`/`types::BspBits`（非互斥 subset，2/3 类可共存）。
//!
//! ## 互斥结构（`Origin.BspClassification.no_exclusive_trichotomy` 已证，不可冒充互斥三分）
//!
//! - 1B/2B 互斥：afterTypeOne 前提冲突。
//! - 1B/3B 互斥：leftCenter 前提冲突。
//! - **2B/3B 可共存**（`no_exclusive_trichotomy`，V 型反转）：bit-vector 容许同位为 1。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0（给定端点语义后）：三类判据是 `EndpointSituation` 字段的确定布尔合取。端点语义
//! 字段（belowLastCenter/afterFirstBuy/...）由 Θ_signal 从走势结构 + 背驰判据计算——
//! 这些是 Θ-参数化运行输入（reference-theta-v0.md:34-37），不由缠论结构无参数导出。

use super::super::types::{BspBits, Center, Side, Tick};
use super::divergence::ForceProxies;

/// ★#218 面 A：owner 归属载体（spec owner-attribution-fix-20260724 ID-1；#211 路线 B +
/// v2 教义修订——归属/确认分层：确认 = 次级别区间套背驰证点成立，归属 = 同级别点属于谁，
/// 两层各用各的结构对象，不可混载）。
///
/// owner 判同按载体形态分两族（判同机制在 nest.rs 核，spec ID-2，全部精确等值无容差）：
/// - [`OwnerRef::Center`]：中枢参照（一/三类）——核心区间 `(zd, zg)` 带判同（中枢身份 =
///   其定义内容价格带；事件侧 B 由 `b_center_start` 当查找键在账本 centers 查出，序号
///   只当键不当身份）。
/// - [`OwnerRef::Type1Anchor`]：点参照（二类）——该走势一类点身份锚的**坐标**
///   （source_index = `find_second_type_structure` 识别的 i1 第一类离开走势终点 =
///   该走势终点极值点，区间套恰好存在）；锚 =（极值价, 合并组锚）由 nest 核内经 T1
///   供给线 oracle 判定时解析（提取层无分型/包含供给，只载坐标，零新增解析）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerRef {
    /// 中枢参照：一类 = 被破的最后中枢；三类 = 所离开回抽的中枢（归属走势的最新中枢）。
    Center(Center),
    /// 点参照：该走势一类点锚坐标（source_index；二类点归属参照——回拉不破一类极值，
    /// 与本级别中枢无关，v2 教义修订）。
    Type1Anchor(usize),
}

/// 端点语义状态（契约锚 `Origin.BspClassification.BspEndpoint`）。
///
/// 端点相对中枢的拓扑/历史语义——区分三类买卖点的真实判据被编码为字段（对齐 `BspEndpoint` 的
/// brokeCenter/afterTypeOne/leftCenter/firstRetrace 等）。这些字段由 Θ_signal 从走势结构（中枢/
/// 趋势/背驰）计算后填充；本结构是判据的纯输入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointSituation {
    /// 此端点之前是否已出现同向第一类买卖点（2 类前提，reference:35）。
    pub after_first_buy: bool,
    /// 此端点是否是「第一类后再次回调/反弹」的结束点（2 类结构，reference:35）。
    pub is_pullback_end: bool,
    /// 之前是否有「次级别走势离开中枢」（3 类前提，reference:36）。
    pub left_center: bool,
    /// 离开后回试/回抽是否「不跌破 ZG / 不升破 ZD」（3 类判据，reference:36）。
    pub retrace_not_reenter: bool,
    /// 端点是否在最后一个中枢下方（1 类买点在中枢下方背驰端点，reference:34）。
    pub below_last_center: bool,
    /// 端点是否处于卖侧（镜像买侧；buy=false 表示这是买点语义，true 表示卖点语义）。
    ///
    /// ★镜像（reference:34-36 "卖镜像"）：1卖=趋势顶背驰端点、2卖=1卖后反弹结束、
    /// 3卖=下离后回抽不入。本字段区分买/卖向，结构字段语义对买卖对称复用。
    pub is_sell_side: bool,
}

impl EndpointSituation {
    /// 第一类判据（reference:34；`IsFirst`）：中枢下方 ∧ 无前置一买 ∧ 未离开中枢。
    ///
    /// 背驰前提：1买的「底背驰」结构由 Θ_signal 在填充 `below_last_center` 前已校验
    /// （趋势末段相对前同向段面积严格变小，见 divergence.rs）——本判据是背驰确认后的端点。
    pub fn is_first(&self) -> bool {
        self.below_last_center && !self.after_first_buy && !self.left_center
    }
    /// 第二类判据（reference:35；`IsSecond`）：1买后 ∧ 回调结束点。
    pub fn is_second(&self) -> bool {
        self.after_first_buy && self.is_pullback_end
    }
    /// 第三类判据（reference:36；`IsThird`）：离开中枢 ∧ 回试不入。
    pub fn is_third(&self) -> bool {
        self.left_center && self.retrace_not_reenter
    }
}

/// 端点语义 → 买卖点 bit-vector（reference:34-36；非互斥 subset）。
///
/// 逐类判据置位：`is_first/is_second/is_third` 各独立置对应买/卖位（按 `is_sell_side`）。
/// ★非互斥（BSP.lean 核心）：2B/3B 可同时为 1（V 型反转）——bit-vector 不强制恰一格。
///
/// 边界条件：所有判据为假 ⟹ 全零 BspBits（非买卖点端点，不是退化错误——升跌完备性
/// 010 仅对真实运动端点 totality，普通端点可空标签）。
pub fn endpoint_to_bsp(e: &EndpointSituation) -> BspBits {
    let mut bits = BspBits::default();
    if e.is_sell_side {
        bits.sell1 = e.is_first();
        bits.sell2 = e.is_second();
        bits.sell3 = e.is_third();
    } else {
        bits.buy1 = e.is_first();
        bits.buy2 = e.is_second();
        bits.buy3 = e.is_third();
    }
    bits
}

/// 买卖点条目（reference:34-36,46；**结构止损价的 single source**）。
///
/// ★single-source 裁定（Lead 接口契约，路 B）：结构止损价是 classifier 识别买卖点时的
/// **结构副产物**——识别"这是 1 买"时已定位那个底分型/线段端点的 pivot 极值（整数 tick）。
/// classifier 是该结构事实的唯一来源；strategy **不**从 bars 重算 pivot（避免两处结构逻辑
/// 漂移，违反严格性）。本条目逐字承载 strategy `StopInput` 所需字段，下游零重算直接构造。
///
/// 结构止损规则（reference:46，由 Θ_risk 选取，strategy 消费）：
/// - 1/2 买止损 = `pivot_low`；3 买止损 = `center.zg`（ZG）。
/// - 1/2 卖止损 = `pivot_high`；3 卖止损 = `center.zd`（ZD）。镜像。
///
/// ## `PartialEq`/`Eq` 手写排除 `force`（β^div 力度铁律，beta-route Task #115）
///
/// `force` 含 `f64`（`ForceProxies` 无 `Eq`）⟹ 不能进 `derive(Eq)`。更本质地：`force` **绝不参与
/// BspPoint 的相等/去重/分桶**——它是旁挂的力度 proxy（经 `Candidate.force` 透传由 selector
/// `z_of_candidate` 读，支配序进 `MuClass.force_state` 第 8 维），**不进** `class_index`/`BspBits`/结构相等。故手写
/// `PartialEq` 逐字段比较**除 force 外全部**（同 `struct_break_dir` 精神但更强：force 连相等都不参与），
/// `Eq` 为标记 impl（其余字段 usize/BspBits/Tick/Option<Center>/Option<Side> 均 Eq，比较自反）。
/// 后果：force 仅 Some↔None 之差的两点相等（去重/bit-exact assert_eq 忽略 force）；但 `Debug` 含 force
/// （derive）⟹ `bit_exact_battery_digest` GOLDEN 因新字段翻转（诚实重算，同 struct_break_dir 先例——
/// 不自定义 Debug 隐藏 force 假装未变）。
#[derive(Debug, Clone, Copy)]
pub struct BspPoint {
    /// 候选点在 L0 原始 K 序的位置（平局裁决 + 回溯定位，reference:16）。
    pub source_index: usize,
    /// 买卖点 bit-vector（非互斥 subset，2/3 类可共存）。
    pub bits: BspBits,
    /// 该买卖点 pivot low（1/2 类买点结构止损源；底分型/线段端点的极值 tick）。
    pub pivot_low: Tick,
    /// 该买卖点 pivot high（1/2 类卖点结构止损源；顶分型/线段端点的极值 tick）。
    pub pivot_high: Tick,
    /// 该买卖点的 owner 归属载体（[`OwnerRef`]；3 类止损取中枢 `zg`(买)/`zd`(卖) 经
    /// [`OwnerRef::Center`] 读出）。
    ///
    /// `None` = 无载体（不可能出现 3 类 bit——3 类判据要求离开中枢，`is_third` 蕴含 left_center；
    /// 故含 3 类 bit 时本字段必 `Some(OwnerRef::Center(_))`）。载体按点类分（#218 面 A，
    /// 归属/确认分层）：一类 = 被破的最后中枢（`make_first_point`）、三类 = 所离开回抽的
    /// 中枢（`make_third_point`）均载 `OwnerRef::Center`；二类 = 该走势一类点身份锚坐标
    /// （`make_second_point` 载 `OwnerRef::Type1Anchor`——v2 教义修订：二类归属参照是该走势
    /// 一类点，回拉不破一类极值，与本级别中枢无关；旧载次级别中枢 c1 = 归属层混载确认层
    /// 对象，级别错配根因，已裁定修填）。1/2 类止损仍只用 pivot（载体不进 1/2 类止损判据，
    /// 止损语义不变）。
    pub center: Option<OwnerRef>,
    /// ★P2-R2（p2-plan-20260701.md 分叉1 + codex-review-20260701-2251 护栏1/2）：破中枢结构方向源。
    ///
    /// `None` = 非破中枢结构候选（第二/三类端点，或未破最后中枢的段）；
    /// `Some(Long/Short)` = 破最后中枢的趋势方向（买侧向下破=Long，卖侧向上破=Short），
    /// 由 `signal::judge_first_cached` 在 `broke ∧ 037:20破极值 ∧ trend ∧ A/C可配对` 时置——**与 macd_c_lt_a
    /// （背驰）无关**，只要几何上破了最后中枢且破 b 包络极值（p117 037:20，语义精确化：破中枢∧破 b 极值方向）就置。
    ///
    /// ## 为什么需要（选择偏差消除的实质，p2-plan §1-§2）
    ///
    /// 破中枢候选若 C≥A（MACD 面积未衰减）⟹ `below_last_center=false` ⟹ 零 buy1/sell1 bit ⟹
    /// `candidate_dir` 消歧为 `Flat` ⟹ 被 μ 门/信号收集按 `dir==Flat` 跳过 ⟹ 从不进 μ 样本。
    /// 本字段让 `interp::candidate_dir` 在**零六 bit**时用它恢复 Long/Short 方向，使这些破中枢
    /// 候选进样本（下游 χ selector 可学习/否证 MACD 是否有用），不再被 MACD C≥A 预删。
    ///
    /// ## bit-exact 铁律（codex 护栏1/2，**关键**）
    ///
    /// 本字段**绝不进** `BspBits`/`MuClass`/`class_index()`/分桶 key——它只在 `candidate_dir`
    /// 消歧层被读，`class_index()` 仍只读六 bit（types.rs:229）。buy1/sell1 判据**完全不动**
    /// （仍仅 macd_c_lt_a 才置）。新增字段进入 `Debug` ⟹ `bit_exact_battery_digest` GOLDEN 翻转
    /// （因 FNV 对 `{pts:?}` 计算）——这是**诚实的**受控代价（GOLDEN 已重算 + 逐 case 证六 bit
    /// 逐字段不变，signal.rs digest guard），不通过隐藏字段假装未变（禁止声明膨胀）。
    pub struct_break_dir: Option<Side>,
    /// ★β^div 力度支配态 proxy（beta-route Task #115，force_state 生产热路由）。
    ///
    /// `Some` = 一类趋势背驰候选（A/C 段可配对）的 A/C 段 [`ForceProxies`]（5 proxy：MACD 面积/DIF
    /// 峰/振幅/速度），由 `signal::judge_first_cached` 在有 dif/closes_tick 输入时算得；`None` = 二/
    /// 三类（无 A/C 对）或未接线路径。经 `Candidate.force` 透传（A6 #159，assemble_gamma 系纯透传），
    /// selector `z_of_candidate` 读它，调 `ForceProxies::force_state()`（唯一支配序原语）填
    /// `MuClass.force_state` 第 8 维——统计层（collect_signals）与生产 π fill loop（entry_z/χ 查询）
    /// 自此同经此路（fullz 置换 records 的 force_state 不再恒 None）。
    ///
    /// ★铁律：**不进** `PartialEq`/`Eq`/`class_index`/`BspBits`/分桶 key（见结构头 `PartialEq` 手写
    /// 说明）——纯旁挂力度量，不改任何结构相等/去重/分桶语义。
    pub force: Option<ForceProxies>,
}

/// 手写 `PartialEq`（排除 `force`，见 [`BspPoint`] 头 β^div 力度铁律说明）。
///
/// ★#455 诚实更新（#434 grilling 交棒件）：曾经的塔级别下标标注分量已随其字段一并删除——
/// 该字段全仓恒为 0、无下游级别语义消费者，仅有恒真 equality 分量及 Debug/digest 机械消费。
/// 级别身份的正主是 `LevelProjectionLayer.identity.level`（`projection.rs`），非本结构存储事实
/// （`CONTEXT.md`「级别身份」词条）。`PartialEq` 由此回到**级别参照系外置后的结构相等**
/// 的本分管辖，不再假装承载跨级身份。
impl PartialEq for BspPoint {
    fn eq(&self, other: &Self) -> bool {
        self.source_index == other.source_index
            && self.bits == other.bits
            && self.pivot_low == other.pivot_low
            && self.pivot_high == other.pivot_high
            && self.center == other.center
            && self.struct_break_dir == other.struct_break_dir
        // force 不参与——旁挂力度 proxy 不改结构相等（去重/bit-exact assert_eq 忽略之）。
    }
}
/// 标记 `Eq`：除 force 外全字段均 `Eq`，手写 `eq` 自反/对称/传递（force 恒不参与 ⟹ 关系合法）。
impl Eq for BspPoint {}

/// 单源绑定查询族（issue #747 C1，spec #756）：收敛「(level, source_index[, confirm_side]) →
/// BspPoint/存在性」历史 17+ 处手写复制（`.chanlun/review-results/arch-survey-e2e-fable-20260729.md`
/// §1.2 全枚举）。语义 = 生产 4 处手写规则逐字节并集——`nest.rs:681` 除外：该处是 ADR-0005
/// GUARD-ROLE 对照臂（判据 crate 禁引 nest 产物是禁令方向，不禁止 nest.rs 反过来调同 crate
/// 内的本族 helper；不改调是 GUARD-ROLE「保持独立实现」纪律本身的要求，非 ADR-0005 禁令所迫），
/// 语义对齐 [`bind_turn`] 但不改调该处调用（照实登记差异，不强并，见票内「先核语义是否逐字
/// 同构」条；订正见 nest.rs:681 函数头注释与 `.chanlun/review-results/issue747-impl-*.md` §3）。
///
/// 三个家族成员对应三种不逐字同构的历史手写形态（未强并为一）：
/// - [`bsp_at`]：纯 `source_index` 等值 find-first，无 side 过滤
///   （`signal.rs::entry_structural_stop` / `fill.rs::entry_stop_reverse_dump_row` 并集）；
/// - [`bsp_bit_at`]：`source_index` 等值 + 自定义 bit 谓词求**存在性**（find-first 换 any——
///   同锚点可多类点共存，语义与 `bsp_at` 不可互换，`econ_positive.rs::xzd_type2_confirmed` 单源）；
/// - [`bind_turn`]：`source_index` 等值 + `confirm_side` find-first——**诊断/单测面**的绑定规则
///   单源体（`nest.rs:690` 的 `TerminalMatch::Exact` 分支仅诊断 bin 与单测可达；生产恒走
///   `TerminalMatch::CWindow` 窗口臂 + `min_by_key`，与本函数不同形，不冒充「生产原型」——
///   订正见 nest.rs:681 函数头注释），供其余同形态诊断复制点对齐，nest.rs 自身不改调。
///
/// newtype 边界（issue #747 C1「时刻分组键 vs 身份 join 类型层分开」条）：**执行降级，不做**。
/// 试点曾引入 `BspSourceIndex(pub usize)` newtype 覆盖 [`bsp_at`] 两处生产身份 join 调用点，
/// 但 `Candidate.source_index`（`strategy/interp.rs`）同一字段在 88+ 处消费点上身兼「时刻分组键」
/// 与「身份 join」两种语义（`strategy/mod.rs:543/759` 的时刻分组过滤即读同一字段），真正的类型层
/// 分开需改字段类型触达全部消费点，改动面远超两处 wrap/unwrap 的字面覆盖，跨边界收益不抵改动面
/// ——降级为不做，试点已回退（详见 `.chanlun/review-results/issue747-impl-*.md` §3，spec #756
/// C1 该条已同步降级登记）。

/// 按 `source_index` 精确等值查首个 [`BspPoint`]（无 side 过滤）——单源函数体，见族头注释。
pub fn bsp_at(bsp: &[BspPoint], source_index: usize) -> Option<&BspPoint> {
    bsp.iter().find(|p| p.source_index == source_index)
}

/// 按 `source_index` 精确等值 + bit 谓词求存在性——单源函数体，见族头注释。
pub fn bsp_bit_at(bsp: &[BspPoint], source_index: usize, pred: impl Fn(&BspBits) -> bool) -> bool {
    bsp.iter()
        .any(|p| p.source_index == source_index && pred(&p.bits))
}

/// 按 `turn_source` 精确等值 + `confirm_side` 查首个 [`BspPoint`]——单源函数体，见族头注释。
pub fn bind_turn(bsp: &[BspPoint], turn_source: usize, side: Side) -> Option<&BspPoint> {
    bsp.iter()
        .find(|p| p.source_index == turn_source && p.bits.confirm_side(side))
}

/// 按 `source_index` 精确等值取**全部**命中点（诊断面枚举用——同锚点多类点共存，`.find()`
/// 只取首个会丢信息，`bin/p112_trend_predicate_caseaudit.rs::at_turn` 单源，issue #747 C1）。
/// 族内独立成员：不与 `bsp_at`（find-first）/`bsp_bit_at`（谓词 any）同构，逐字保留原过滤语义。
pub fn bsp_all_at(bsp: &[BspPoint], source_index: usize) -> impl Iterator<Item = &BspPoint> {
    bsp.iter().filter(move |p| p.source_index == source_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn situ(
        after_first: bool,
        pullback: bool,
        left: bool,
        retrace: bool,
        below: bool,
        sell: bool,
    ) -> EndpointSituation {
        EndpointSituation {
            after_first_buy: after_first,
            is_pullback_end: pullback,
            left_center: left,
            retrace_not_reenter: retrace,
            below_last_center: below,
            is_sell_side: sell,
        }
    }

    #[test]
    fn first_buy_judgment_bit_exact() {
        // IsType1 = below ∧ ¬after_first ∧ ¬left（Origin.BspClassification.IsType1 见证）。
        let e = situ(false, false, false, false, true, false);
        assert!(e.is_first());
        let bits = endpoint_to_bsp(&e);
        assert!(bits.buy1 && !bits.buy2 && !bits.buy3);
    }

    #[test]
    fn second_buy_judgment_bit_exact() {
        // IsType2 = after_first ∧ pullback（Origin.BspClassification.IsType2 见证）。
        let e = situ(true, true, false, false, false, false);
        assert!(e.is_second());
        let bits = endpoint_to_bsp(&e);
        assert!(!bits.buy1 && bits.buy2 && !bits.buy3);
    }

    #[test]
    fn third_buy_judgment_bit_exact() {
        // IsType3 = left ∧ retrace（Origin.BspClassification.IsType3Buy/IsType3Sell 见证）。
        let e = situ(false, false, true, true, false, false);
        assert!(e.is_third());
        let bits = endpoint_to_bsp(&e);
        assert!(!bits.buy1 && !bits.buy2 && bits.buy3);
    }

    #[test]
    fn two_three_buy_coincide_bit_exact() {
        // ★2B/3B 可重合（V 型反转，x_2b3b）：after_first ∧ pullback ∧ left ∧ retrace。
        let e = situ(true, true, true, true, false, false);
        assert!(e.is_second() && e.is_third());
        let bits = endpoint_to_bsp(&e);
        // bit-vector 容许 [2B,3B] 同位为 1（非互斥 subset）。
        assert!(bits.buy2 && bits.buy3);
        assert!(!bits.buy1); // 1B 与 2/3 互斥（below_last_center=false）
    }

    #[test]
    fn first_second_mutually_exclusive_bit_exact() {
        // 1B 与 2B 不重合（first_second_disjoint）：is_first 要求 ¬after_first，
        // is_second 要求 after_first ⟹ 不可同真。
        let e1 = situ(false, false, false, false, true, false); // 1B
        assert!(e1.is_first() && !e1.is_second());
        let e2 = situ(true, true, false, false, true, false); // after_first=true ⟹ 非 1B
        assert!(!e2.is_first() && e2.is_second());
    }

    #[test]
    fn sell_side_mirrors_buy_side() {
        // 卖镜像：同结构字段 + is_sell_side=true ⟹ 置 sell 位。
        let e = situ(false, false, true, true, false, true); // 3 类，卖侧
        assert!(e.is_third());
        let bits = endpoint_to_bsp(&e);
        assert!(bits.sell3 && !bits.buy3);
    }

    #[test]
    fn no_signal_yields_empty_bits() {
        // 所有判据假 ⟹ 全零 bit-vector（非买卖点端点）。
        let e = situ(false, false, false, false, false, false);
        assert_eq!(endpoint_to_bsp(&e), BspBits::default());
    }

    /// ★615 定理（codex 裁决⑤ §615，codex-ritual-resubmit-20260702.md）：
    /// 构造子穷尽（ConstructorExhaustiveLayer1，603 定义「所有输入落入某构造子无遗漏」）
    /// 不蕴含 Layer2 complete。Lean `twoB_threeB_can_coincide` 只闭合
    /// `ExclusiveSumLayer1 ⊊ Layer2`；本定理补齐 ConstructorExhaustive 版本：
    /// `endpoint_to_bsp` 对全输入空间穷尽（语法无逃逸，Layer1 成立），却违反
    /// Layer2 complete（I(x)=I(y) ⟹ x∼y，其中 ∼ = 端点语义字段等同，非标签核——
    /// 若取标签核则落入 QuotientByLabel 同义反复，615 边界条件1）。
    #[test]
    fn theorem_615_constructor_exhaustive_not_complete() {
        // (a) 穷尽性（Layer1）：全部 2^6 端点语义组合都落入某 BspBits 标签，无逃逸。
        for m in 0u8..64 {
            let e = situ(
                m & 1 != 0,
                m & 2 != 0,
                m & 4 != 0,
                m & 16 != 0,
                m & 8 != 0,
                m & 32 != 0,
            );
            let _label: BspBits = endpoint_to_bsp(&e); // 全函数：任何输入必得标签（含空标签）
        }
        // (b) complete 失败：x、y 语义不等价（after_first_buy 历史字段不同 ⟹ ¬(x∼y)）
        //     却同标签 {3B}——I(x)=I(y) ∧ ¬(x∼y)，双射 X/∼≅P 所需的商集单射破。
        let x = situ(false, false, true, true, false, false);
        let y = situ(true, false, true, true, false, false);
        assert_ne!(x, y, "语义层不等价：after_first_buy 历史不同");
        assert_eq!(
            endpoint_to_bsp(&x),
            endpoint_to_bsp(&y),
            "标签层等同：同为 3B 单标签"
        );
        let bits = endpoint_to_bsp(&x);
        assert!(bits.buy3 && !bits.buy1 && !bits.buy2);
    }

    /// property：1B 与 2/3B 互斥（任意端点，wf_type1_exclusive）。
    #[test]
    fn property_first_excludes_second_third() {
        // 枚举所有 2^5 字段组合（买侧），验证 is_first ⟹ ¬is_second ∧ ¬is_third。
        for bits in 0u8..32 {
            let e = situ(
                bits & 1 != 0,
                bits & 2 != 0,
                bits & 4 != 0,
                bits & 8 != 0,
                bits & 16 != 0,
                false,
            );
            if e.is_first() {
                assert!(!e.is_second(), "1B 与 2B 互斥");
                assert!(!e.is_third(), "1B 与 3B 互斥");
            }
        }
    }

    fn center(zd: Tick, zg: Tick) -> Center {
        Center {
            zd,
            zg,
            dd: zd - 5,
            gg: zg + 5,
            start_index: 0,
            end_index: 0,
        }
    }

    #[test]
    fn bsp_point_carries_pivot_low_for_first_buy() {
        // 路 B single source：1 买条目携带 pivot_low（strategy 1/2 买止损源，零重算）。
        let bits = BspBits {
            buy1: true,
            ..Default::default()
        };
        let p = BspPoint {
            source_index: 42,
            bits,
            pivot_low: 1000,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        };
        // strategy 1 买止损 = pivot_low（reference:46）——直接读，不从 bars 重算。
        assert_eq!(p.pivot_low, 1000);
        assert!(p.bits.buy1);
    }

    #[test]
    fn bsp_point_carries_center_for_third_buy() {
        // 路 B single source：3 买条目携带 center（strategy 3 买止损=center.zg，无歧义定位）。
        let bits = BspBits {
            buy3: true,
            ..Default::default()
        };
        let c = center(800, 1200);
        let p = BspPoint {
            source_index: 50,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: Some(OwnerRef::Center(c)),
            struct_break_dir: None,
            force: None,
        };
        // strategy 3 买止损 = center.zg（ZG，reference:46）——条目直接关联中枢，无需 strategy 猜。
        // （#218 面 A 载体形态：Center 变体读出，机械适配。）
        assert_eq!(
            p.center.and_then(|o| match o {
                OwnerRef::Center(c) => Some(c.zg),
                _ => None,
            }),
            Some(1200)
        );
        assert!(p.bits.buy3);
    }

    #[test]
    fn bsp_point_third_buy_always_has_center() {
        // 不变量：含 3 类 bit ⟹ center 必 Some（is_third 蕴含 left_center，离开中枢才 3 类）。
        // 此处验证构造契约——3 类买卖点的 BspPoint 由 classifier 顶层填充时 center 非 None。
        let e = situ(false, false, true, true, false, false); // is_third=true
        let bits = endpoint_to_bsp(&e);
        assert!(bits.buy3, "前提：这是 3 买端点");
        // classifier 顶层为含 3 类 bit 的端点填 center=Some（离开的那个中枢）——契约见 mod.rs。
        // 本测试锁定语义：strategy 读 3 类止损时 center 必可用。
        let c = center(800, 1200);
        let p = BspPoint {
            source_index: 0,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: Some(OwnerRef::Center(c)),
            struct_break_dir: None,
            force: None,
        };
        assert!(p.bits.buy3 && p.center.is_some());
    }

    #[test]
    fn bsp_point_sell_side_carries_pivot_high_and_zd() {
        // 卖镜像：1/2 卖止损=pivot_high；3 卖止损=center.zd。
        let bits = BspBits {
            sell1: true,
            ..Default::default()
        };
        let c = center(800, 1200);
        let p = BspPoint {
            source_index: 7,
            bits,
            pivot_low: 0,
            pivot_high: 1500,
            center: Some(OwnerRef::Center(c)),
            struct_break_dir: None,
            force: None,
        };
        assert_eq!(p.pivot_high, 1500); // 1/2 卖止损源
                                        // 3 卖止损源 center.zd（Center 变体读出，#218 面 A 载体形态机械适配）。
        assert_eq!(
            p.center.and_then(|o| match o {
                OwnerRef::Center(c) => Some(c.zd),
                _ => None,
            }),
            Some(800)
        );
    }

    fn minimal_point(source_index: usize, bits: BspBits) -> BspPoint {
        BspPoint {
            source_index,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        }
    }

    /// issue #747 C1：单源绑定查询族——`bsp_at` 纯等值 find-first，无 side 过滤。
    #[test]
    fn bsp_at_finds_first_by_source_index_only() {
        let pts = vec![
            minimal_point(
                5,
                BspBits {
                    sell1: true,
                    ..Default::default()
                },
            ),
            minimal_point(
                7,
                BspBits {
                    buy1: true,
                    ..Default::default()
                },
            ),
        ];
        assert_eq!(bsp_at(&pts, 7).map(|p| p.source_index), Some(7));
        assert!(bsp_at(&pts, 999).is_none());
    }

    /// issue #747 C1：`bsp_bit_at` = 等值 + bit 谓词求存在性——同锚点多类点共存时用 any 而非
    /// find-first（`econ_positive.rs::xzd_type2_confirmed` 单源语义，见族头注释）。
    #[test]
    fn bsp_bit_at_checks_existence_not_first_match() {
        let pts = vec![
            minimal_point(
                3,
                BspBits {
                    buy1: true,
                    ..Default::default()
                },
            ),
            minimal_point(
                3,
                BspBits {
                    buy2: true,
                    ..Default::default()
                },
            ),
        ];
        // 首个同 source_index 条目是 buy1，但 buy2 在第二条目——find-first 会漏，any 不会。
        assert!(bsp_bit_at(&pts, 3, |b| b.buy2));
        assert!(!bsp_bit_at(&pts, 3, |b| b.sell1));
        assert!(!bsp_bit_at(&pts, 999, |b| b.buy1));
    }

    /// issue #747 C1：`bind_turn` = 等值 + `confirm_side` find-first（`nest.rs:681` 生产绑定规则
    /// 原型语义单源；nest.rs 自身保持独立实现不改调，仅登记口径见其函数头注释）。
    #[test]
    fn bind_turn_requires_confirm_side() {
        let pts = vec![minimal_point(
            7,
            BspBits {
                buy1: true,
                ..Default::default()
            },
        )];
        assert!(bind_turn(&pts, 7, Side::Long).is_some());
        assert!(bind_turn(&pts, 7, Side::Short).is_none());
        assert!(bind_turn(&pts, 8, Side::Long).is_none());
    }

    /// issue #747 C1：`bsp_all_at` 取全部同 source_index 命中点（诊断枚举用，不同于 find-first）。
    #[test]
    fn bsp_all_at_collects_every_match() {
        let pts = vec![
            minimal_point(
                3,
                BspBits {
                    buy1: true,
                    ..Default::default()
                },
            ),
            minimal_point(
                3,
                BspBits {
                    buy2: true,
                    ..Default::default()
                },
            ),
            minimal_point(
                4,
                BspBits {
                    sell1: true,
                    ..Default::default()
                },
            ),
        ];
        let at3: Vec<_> = bsp_all_at(&pts, 3).collect();
        assert_eq!(at3.len(), 2);
        assert!(bsp_all_at(&pts, 999).next().is_none());
    }

    /// 全枚举对拍守卫（issue #747 spec #756 Testing Decisions「17+ 处调用点全枚举对拍」，评审
    /// review-747.log 条目①先例：`env_registry::tests::no_stray_env_literals_outside_registry`）：
    /// 全仓（本文件除外）grep 手写绑定规则形态——`.find(|x| x.source_index == …)` /
    /// `.any(|x| … x.source_index == …)` / `.filter(|x| x.source_index == …)`——命中点必须逐一
    /// 落在下方白名单（file:line）内，否则说明有新的手写复制点绕过了单源 API 族，测试见红。
    /// 新增/移动白名单外命中点须先并入 `bsp_at`/`bsp_bit_at`/`bind_turn`/`bsp_all_at`，或明确
    /// 登记进本白名单并写清不并入理由——证据是「真收口」而非「多一份影子清单」。
    #[test]
    fn no_stray_bsp_binding_rewrites_outside_registry() {
        // 白名单（file:line，相对 rust/src）：均已核实为非「本族绑定规则」手写复制点。
        const WHITELIST: &[&str] = &[
            // #747 合并态（main a4c24e86d1）行号随合入改动重排，逐条重核，理由不变：
            //
            // 真复制点已改调单源 API：
            // bin/p_issue668_bsp_key_truth.rs:266（`.any(source_index==anchor_idx && bit)`）
            // 已改调 `bsp_bit_at`，不再命中本 grep——不登记进白名单。
            //
            // Fractal 类型的单源函数体（作用于 `Fractal`，非 `BspPoint`，不入本族）。
            "theta_v0/parser/fractal.rs:82",
            // #[ignore] 死探针（已被 runner::newly_confirmed_step 取代，issue #747 条目6 登记不入
            // 18 处枚举）。（#885 S4-d：上方 slice_step 增一行 ⟹ 596→597，同条目重登记。）
            "theta_v0/backtest/l3_pi_probe.rs:597",
            // ADR-0005 GUARD-ROLE 对照臂原型语句本体（单列登记，不改调，见函数头注释订正）。
            "theta_v0/classifier/nest.rs:718",
            // #885 S4-d：`LevelState::first_class_grade_at`/`Classification::otherwise_domain_at`
            // 单源查询函数体（作用于 `FirstClassGradeRecord` 分级记录族，非 `BspPoint`，与本族
            // 三件并行不同型；同 projection.rs:215 先例——查询实现全仓各仅一份，非复制点）。
            // （#881 S3：上方 `pub mod operation;` 增 3 行 ⟹ 218→221 / 250→253，同条目重登记。
            // 2026-08-16 镜像追推后 CI 首跑再位移（221→224 / 253→256）：逐条重核内容不变
            // ——两访问器函数体依旧，同条目重登记。）
            "theta_v0/classifier/mod.rs:224",
            "theta_v0/classifier/mod.rs:256",
            // `CrossLevelConfirmationQuery::entry_at` 单源函数体（作用于 `TripleAnchorEntry`，
            // 与本族 `BspPoint` 三件并行不同型，见 projection.rs 函数头注释）。
            "theta_v0/classifier/projection.rs:215",
            // #[cfg(test)] mod 内断言用字面量 source_index 过滤——测试夹具，非生产绑定规则。
            // （#885 S4-d：本文件上方新增注释/参数/测试 ⟹ 行号整体位移，逐条重核重登记，理由不变；
            // 3209/4358 为本票新增测试夹具过滤，同类登记。
            // 2026-08-04 再位移（+5/+1）：#885 收尾 rustfmt 尾巴（4fe13e12d3）折行推移，逐条重核
            // 内容不变（同为测试夹具 source_index 过滤），理由不变。#881 S3：mod.rs 测试区上方
            // 增集成测试 ⟹ 该点 4359→4548，同条目重登记。）
            "theta_v0/classifier/signal.rs:2561",
            "theta_v0/classifier/signal.rs:2845",
            "theta_v0/classifier/signal.rs:3214",
            "theta_v0/classifier/signal.rs:3353",
            "theta_v0/classifier/signal.rs:3386",
            "theta_v0/classifier/signal.rs:3642",
            "theta_v0/classifier/signal.rs:3706",
            "theta_v0/classifier/mod.rs:4551",
            // 「时刻分组键」过滤（spec §12 语义，合法契约非泄漏，非身份 join，不入本族）。
            // （2026-08-16 镜像追推后 CI 首跑位移 551→553 / 773→775：逐条重核内容不变——同为
            // `gamma_x` 时刻分组过滤，同条目重登记。mod.rs:4548→4551 测试夹具点同款处理。）
            "theta_v0/strategy/mod.rs:553",
            "theta_v0/strategy/mod.rs:775",
        ];

        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let output = std::process::Command::new("grep")
            .args([
                "-rnE",
                r"\.(find|any|filter)\(\s*\|[a-zA-Z_]+\|[^)]*\.source_index\s*==",
                "--include=*.rs",
            ])
            .arg(&src_dir)
            .output()
            .expect("grep 不可执行（完备性单测依赖系统 grep）");
        let src_dir_prefix = format!("{}/", src_dir.display());
        let whitelist: std::collections::BTreeSet<&str> = WHITELIST.iter().copied().collect();
        let mut hits: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some(rest) = line.strip_prefix(&src_dir_prefix) else {
                continue;
            };
            let Some((path, tail)) = rest.split_once(':') else {
                continue;
            };
            let Some((lineno, _content)) = tail.split_once(':') else {
                continue;
            };
            // 本文件（bsp.rs）持有单源函数体本身，不算「手写复制点」，排除自身。
            if path == "theta_v0/classifier/bsp.rs" {
                continue;
            }
            hits.insert(format!("{path}:{lineno}"));
        }
        let stray: Vec<&String> = hits
            .iter()
            .filter(|h| !whitelist.contains(h.as_str()))
            .collect();
        assert!(
            stray.is_empty(),
            "以下命中点是白名单外的手写绑定规则复制点，应改调 bsp_at/bsp_bit_at/bind_turn/bsp_all_at，\
             或登记进 WHITELIST 并写明理由：{stray:?}"
        );
        let stale: Vec<&&str> = whitelist.iter().filter(|w| !hits.contains(**w)).collect();
        assert!(
            stale.is_empty(),
            "以下白名单条目未在当前 grep 命中中出现（代码已迁移/删除，白名单应同步清理）：{stale:?}"
        );
    }
}
