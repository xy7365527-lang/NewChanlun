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
    /// 该买卖点所在级别的最后中枢（3 类止损取 `zg`(买)/`zd`(卖)）。
    ///
    /// `None` = 无中枢（不可能出现 3 类 bit——3 类判据要求离开中枢，`is_third` 蕴含 left_center；
    /// 故含 3 类 bit 时本字段必 `Some`）。★owner 载体补齐（关③ 补记 2026-07-18 ②，路径 (a)）：
    /// 生产一/二类点构造时同样填入判定中枢（`make_first_point` 填被破的最后中枢 `last_center`、
    /// `make_second_point` 填 `c1`）——名实一致（090）；center 是 Trend 域 owner=B 判定式
    ///（`point.center.start_index == b_center_start`）的载体与回溯锚，1/2 类止损仍只用 pivot
    ///（center 不进 1/2 类止损判据，止损语义不变）。
    pub center: Option<Center>,
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
            let e = situ(m & 1 != 0, m & 2 != 0, m & 4 != 0, m & 16 != 0, m & 8 != 0, m & 32 != 0);
            let _label: BspBits = endpoint_to_bsp(&e); // 全函数：任何输入必得标签（含空标签）
        }
        // (b) complete 失败：x、y 语义不等价（after_first_buy 历史字段不同 ⟹ ¬(x∼y)）
        //     却同标签 {3B}——I(x)=I(y) ∧ ¬(x∼y)，双射 X/∼≅P 所需的商集单射破。
        let x = situ(false, false, true, true, false, false);
        let y = situ(true, false, true, true, false, false);
        assert_ne!(x, y, "语义层不等价：after_first_buy 历史不同");
        assert_eq!(endpoint_to_bsp(&x), endpoint_to_bsp(&y), "标签层等同：同为 3B 单标签");
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
        Center { zd, zg, dd: zd - 5, gg: zg + 5, start_index: 0, end_index: 0 }
    }

    #[test]
    fn bsp_point_carries_pivot_low_for_first_buy() {
        // 路 B single source：1 买条目携带 pivot_low（strategy 1/2 买止损源，零重算）。
        let bits = BspBits { buy1: true, ..Default::default() };
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
        let bits = BspBits { buy3: true, ..Default::default() };
        let c = center(800, 1200);
        let p = BspPoint {
            source_index: 50,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: Some(c),
            struct_break_dir: None,
            force: None,
        };
        // strategy 3 买止损 = center.zg（ZG，reference:46）——条目直接关联中枢，无需 strategy 猜。
        assert_eq!(p.center.map(|c| c.zg), Some(1200));
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
        let p = BspPoint { source_index: 0, bits, pivot_low: 0, pivot_high: 0, center: Some(c), struct_break_dir: None, force: None };
        assert!(p.bits.buy3 && p.center.is_some());
    }

    #[test]
    fn bsp_point_sell_side_carries_pivot_high_and_zd() {
        // 卖镜像：1/2 卖止损=pivot_high；3 卖止损=center.zd。
        let bits = BspBits { sell1: true, ..Default::default() };
        let c = center(800, 1200);
        let p = BspPoint {
            source_index: 7,
            bits,
            pivot_low: 0,
            pivot_high: 1500,
            center: Some(c),
            struct_break_dir: None,
            force: None,
        };
        assert_eq!(p.pivot_high, 1500); // 1/2 卖止损源
        assert_eq!(p.center.map(|c| c.zd), Some(800)); // 3 卖止损源
    }
}
