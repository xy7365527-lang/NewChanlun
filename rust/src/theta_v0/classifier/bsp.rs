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

use super::super::types::{BspBits, Center, Tick};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// 故含 3 类 bit 时本字段必 `Some`）。1/2 类只用 pivot，center 可为 `None`。
    pub center: Option<Center>,
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
        let p = BspPoint { source_index: 0, bits, pivot_low: 0, pivot_high: 0, center: Some(c) };
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
        };
        assert_eq!(p.pivot_high, 1500); // 1/2 卖止损源
        assert_eq!(p.center.map(|c| c.zd), Some(800)); // 3 卖止损源
    }
}
