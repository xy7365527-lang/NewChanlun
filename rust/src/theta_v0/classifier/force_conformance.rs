//! 力度 conformance（rust 半）—— MACD 段面积实例化 `Origin.ForceInterface.ForceMeasure` 接口
//! 并验证 `mono`/`faithful` 公理（task #127 force-conformance 工位 rust 半）。
//!
//! ## 契约锚（`formal/Origin/ForceInterface.lean`）
//!
//! Origin `ForceMeasure` 力度抽象接口（ForceInterface.lean）：
//! ```text
//! structure ForceMeasure (α : Type u) where
//!   measure  : α → Force                                       -- 力度（携 area）
//!   strength : α → Nat                                         -- 强度标量
//!   mono     : ∀ a b, strength a ≤ strength b → (measure a).area ≤ (measure b).area
//!   faithful : ∀ a b, (measure a).area < (measure b).area → strength a ≤ strength b
//! def IsDivergenceVia (fm) (a c) : Prop := (fm.measure c).area < (fm.measure a).area
//! theorem divergenceVia_implies_strength_le ... := fm.faithful c a hdiv
//! ```
//! `Origin.Divergence.IsDivergence`（Divergence.lean:83）：`d.forceC.area < d.forceA.area`——
//! 后段力度面积严格小于前段 ⟹ 背驰。本文件把 MACD 段面积（`divergence::segment_macd_area`）
//! 包成满足 `ForceMeasure` 接口语义（`mono`/`faithful` 公理）的 rust 适配器。
//!
//! ## ★Lean 半状态与 bit-exact 对齐（no-workaround 诚实声明）
//!
//! - **Lean 半已存在**：`formal/Origin/ForceConformance.lean` 已落地，`lake env lean` 通过（L0）。
//!   内容：conformance 关系代数性质（自反/对称/传递）+ `MacdForceMeasureWitness`（L2 假设载体）+
//!   `MacdConformsTo`（未证命题）。
//! - **Rust↔Lean bit-exact 对齐仍是 L2 未验证假设**：`MacdConformsTo` 在 Lean 中是 Prop（未证
//!   定理），`MacdForceMeasureWitness` 是显式 L2 假设（ForceConformance.lean 不构造其实例）。
//!   「rust MACD 是 ForceMeasure 合法实例 + 与 Lean 判据一致」需真实 K 线 L2 验证，当前未验证。
//! - **本文件（rust 半）独立成立**：MACD area 适配器满足 ForceMeasure 接口的 mono/faithful
//!   公理，L1 逐例验证。rust 半不依赖 Lean 半，Lean 半的 L2 前件由外部数据验证工位填充。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - **L1**（rust 半）：MACD 段面积适配器满足 `ForceMeasure` 接口语义（mono/faithful 公理在 Rust
//!   侧逐例验证）= 验证管线正确性，零信息增量。**不**是「MACD 背驰预测在真实行情有效」（L2/L3）。
//! - Rust↔Lean bit-exact 对齐（force-conformance 完整目标）阻塞于 Lean 半未存在——标待对齐依赖。

use super::divergence;

/// 力度度量结果（对齐 `Origin.ForceInterface.Force`：携 area 标量）。
///
/// `Origin.Force` 是携 `area : Nat`（或可比标量）的结构；本 Rust 用 `f64` area（MACD 段面积在
/// 浮点域，隔离在 divergence 模块）+ 整数 `strength`（量化的强度序，满足 ForceMeasure 接口的
/// `strength : α → Nat`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Force {
    /// 力度面积（MACD 段 `|hist|` 之和，浮点域）。对齐 `Origin.Force.area`。
    pub area: f64,
}

/// MACD 段面积力度度量（实例化 `Origin.ForceInterface.ForceMeasure` 接口，rust 半）。
///
/// 把一个**同向段**（bar 区间 `[start,end]`）映射到力度——`measure` 取 MACD 段面积
/// （`divergence::segment_macd_area`），`strength` 取非负有限 `f64` 的 IEEE-754 位序；对该值域，
/// `to_bits()` 是到 `u64` 的精确序嵌入，满足 `ForceMeasure.strength : α → Nat`。
///
/// ★`mono`/`faithful` 公理（`Origin.ForceMeasure`）：本适配器不再做有碰撞的 round 量化；
/// `strength = area.to_bits()` 在非负有限域精确保序，故两条公理均按原式成立、没有误差窗。详见
/// [`ForceMeasureAdapter::mono_holds`] / [`ForceMeasureAdapter::faithful_holds`] 的逐例验证。
#[derive(Debug, Clone, Copy)]
pub struct ForceMeasureAdapter;

impl Default for ForceMeasureAdapter {
    fn default() -> Self {
        ForceMeasureAdapter
    }
}

impl ForceMeasureAdapter {
    /// 构造精确序适配器。
    pub const fn new() -> Self {
        ForceMeasureAdapter
    }

    /// `measure : α → Force`（契约锚 `Origin.ForceMeasure.measure`）：段 → 力度（MACD 段面积）。
    ///
    /// `hist` 是 MACD hist 序列（`divergence::compute_macd` 输出）；`(start, end)` 是同向段闭区间
    /// bar 索引。返回 `Force { area = segment_macd_area(hist, start, end) }`。
    pub fn measure(&self, hist: &[f64], start: usize, end: usize) -> Force {
        let area = divergence::segment_macd_area(hist, start, end);
        assert!(
            area.is_finite() && area >= 0.0,
            "ForceMeasure area 必须是非负有限数，收到 {area}"
        );
        Force { area }
    }

    /// `strength : α → Nat`：非负有限 area 的精确序嵌入。
    ///
    /// IEEE-754 对所有非负有限 `f64` 的无符号位序与数值序一致；不同 area 不碰撞。
    pub fn strength(&self, hist: &[f64], start: usize, end: usize) -> u64 {
        let area = self.measure(hist, start, end).area;
        area.to_bits()
    }

    /// `IsDivergenceVia fm a c`（契约锚 `Origin.ForceInterface.IsDivergenceVia` +
    /// `Origin.Divergence.IsDivergence`）：后段 c 力度面积**严格**小于前段 a ⟹ 背驰。
    ///
    /// `(measure c).area < (measure a).area`（逐字对齐 Origin `IsDivergence : forceC.area < forceA.area`）。
    /// 这与 [`divergence::segments_diverge`] 同语义（后者直接比面积）——本函数经 ForceMeasure 接口表达。
    pub fn is_divergence_via(
        &self,
        hist: &[f64],
        seg_a: (usize, usize),
        seg_c: (usize, usize),
    ) -> bool {
        let area_a = self.measure(hist, seg_a.0, seg_a.1).area;
        let area_c = self.measure(hist, seg_c.0, seg_c.1).area;
        area_c < area_a
    }

    /// `mono` 公理逐例验证（契约锚 `Origin.ForceMeasure.mono`）：`strength a ≤ strength b ⟹
    /// area a ≤ area b`（精确，无量化误差窗）。
    ///
    /// 返回该两段对是否满足 mono（用于测试逐例验证接口公理）。
    pub fn mono_holds(&self, hist: &[f64], a: (usize, usize), b: (usize, usize)) -> bool {
        let sa = self.strength(hist, a.0, a.1);
        let sb = self.strength(hist, b.0, b.1);
        let area_a = self.measure(hist, a.0, a.1).area;
        let area_b = self.measure(hist, b.0, b.1).area;
        // mono：strength 是精确序嵌入，前件成立时必须逐字满足 area_a ≤ area_b。
        if sa <= sb {
            area_a <= area_b
        } else {
            true // 前件不成立 ⟹ 蕴含平凡真
        }
    }

    /// `faithful` 公理逐例验证（契约锚 `Origin.ForceMeasure.faithful`）：`area a < area b ⟹
    /// strength a ≤ strength b`（area 严格小 ⟹ 量化强度不大于）。
    pub fn faithful_holds(&self, hist: &[f64], a: (usize, usize), b: (usize, usize)) -> bool {
        let area_a = self.measure(hist, a.0, a.1).area;
        let area_b = self.measure(hist, b.0, b.1).area;
        if area_a < area_b {
            self.strength(hist, a.0, a.1) <= self.strength(hist, b.0, b.1)
        } else {
            true // 前件不成立 ⟹ 蕴含平凡真
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::config::MacdConfig;

    fn macd_hist(closes: &[f64]) -> Vec<f64> {
        divergence::compute_macd(closes, &MacdConfig::default()).hist
    }

    /// measure = MACD 段面积（契约锚 `Origin.ForceMeasure.measure`）。
    #[test]
    fn measure_equals_segment_area() {
        let fm = ForceMeasureAdapter::default();
        let hist = vec![1.0, -2.0, 3.0, -4.0];
        // [0,2]：|1|+|-2|+|3|=6（与 segment_macd_area 一致）。
        assert_eq!(fm.measure(&hist, 0, 2).area, 6.0);
    }

    /// strength 是 area 的精确序嵌入（契约锚 `Origin.ForceMeasure.strength`）。
    #[test]
    fn strength_is_exact_order_embedding() {
        let fm = ForceMeasureAdapter::default();
        let hist = vec![1.0, -2.0, 3.0, -4.0];
        // area([0,1])=3 ⟹ strength=3；area([0,3])=10 ⟹ strength=10。
        assert_eq!(fm.strength(&hist, 0, 1), 3.0f64.to_bits());
        assert_eq!(fm.strength(&hist, 0, 3), 10.0f64.to_bits());
    }

    /// BUG-11：`strength` 必须是 f64 area 的精确序嵌入；相近但不等的面积不得量化碰撞。
    #[test]
    fn strength_preserves_strict_order_for_close_areas() {
        let fm = ForceMeasureAdapter::default();
        let hist = vec![1.2, 1.1];
        let stronger = fm.strength(&hist, 0, 0);
        let weaker = fm.strength(&hist, 1, 1);
        assert!(stronger > weaker, "1.2 > 1.1 必须映成严格更大的 Nat strength");
        assert!(fm.mono_holds(&hist, (0, 0), (1, 1)));
        assert!(fm.faithful_holds(&hist, (1, 1), (0, 0)));
    }

    /// ★is_divergence_via 与 segments_diverge 同语义（契约锚 `Origin.IsDivergence`）。
    #[test]
    fn is_divergence_via_matches_segments_diverge() {
        let fm = ForceMeasureAdapter::default();
        let hist = vec![5.0, -5.0, 1.0, -1.0];
        // 前段 [0,1] 面积 10，后段 [2,3] 面积 2 ⟹ 背驰（2 < 10）。
        assert!(fm.is_divergence_via(&hist, (0, 1), (2, 3)));
        assert_eq!(
            fm.is_divergence_via(&hist, (0, 1), (2, 3)),
            divergence::segments_diverge(&hist, (0, 1), (2, 3)),
            "ForceMeasure 接口与直接面积比同语义"
        );
        // 反向不背驰。
        assert!(!fm.is_divergence_via(&hist, (2, 3), (0, 1)));
    }

    /// ★mono 公理逐例验证（契约锚 `Origin.ForceMeasure.mono`）：真实 MACD hist 上跨段对穷举。
    #[test]
    fn mono_axiom_holds_on_real_macd() {
        let fm = ForceMeasureAdapter::default();
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + ((i * 7 % 13) as f64)).collect();
        let hist = macd_hist(&closes);
        let n = hist.len();
        // 穷举段对验证 mono（strength a ≤ strength b ⟹ area a ⪅ area b）。
        for a_end in 0..n.min(10) {
            for b_end in 0..n.min(10) {
                assert!(
                    fm.mono_holds(&hist, (0, a_end), (0, b_end)),
                    "mono 公理破坏 a=[0,{}] b=[0,{}]", a_end, b_end
                );
            }
        }
    }

    /// ★faithful 公理逐例验证（契约锚 `Origin.ForceMeasure.faithful`）：area 严格小 ⟹ strength 不大于。
    #[test]
    fn faithful_axiom_holds_on_real_macd() {
        let fm = ForceMeasureAdapter::default();
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + ((i * 11 % 17) as f64)).collect();
        let hist = macd_hist(&closes);
        let n = hist.len();
        for a_end in 0..n.min(10) {
            for b_end in 0..n.min(10) {
                assert!(
                    fm.faithful_holds(&hist, (0, a_end), (0, b_end)),
                    "faithful 公理破坏 a=[0,{}] b=[0,{}]", a_end, b_end
                );
            }
        }
    }

    /// faithful 经 ForceMeasure 推出强度序（契约锚 `Origin.divergenceVia_implies_strength_le`）：
    /// 背驰（area c < area a）⟹ strength c ≤ strength a。
    #[test]
    fn divergence_via_implies_strength_le() {
        let fm = ForceMeasureAdapter::default();
        let hist = vec![5.0, -5.0, 1.0, -1.0];
        let seg_a = (0, 1); // area 10
        let seg_c = (2, 3); // area 2
        assert!(fm.is_divergence_via(&hist, seg_a, seg_c), "前提：背驰");
        // divergenceVia_implies_strength_le：背驰 ⟹ strength c ≤ strength a。
        let str_a = fm.strength(&hist, seg_a.0, seg_a.1);
        let str_c = fm.strength(&hist, seg_c.0, seg_c.1);
        assert!(str_c <= str_a, "背驰 ⟹ strength c ≤ strength a（faithful 推论）");
    }
}
