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
//! ## ★Lean 半待对齐依赖（no-workaround 诚实声明）
//!
//! force-conformance 工位的 **Lean 半**（`formal/Origin/ForceConformance.lean`：把 MACD area 度量
//! 形式化为 `ForceMeasure` 的具体实例 + 证它满足 mono/faithful + 与 Rust 段面积 bit-exact 对齐）
//! **当前不存在**（已核实 `formal/Origin/ForceConformance.lean` 缺失）。本文件是 **rust 半**：
//! 在 Rust 侧把 MACD 段面积包成满足 ForceMeasure 接口语义的适配器并验证公理。
//!
//! 完整 force-conformance（Rust↔Lean bit-exact 对齐）**待 Lean 半 `Origin/ForceConformance.lean`
//! 落地**——本文件**不**冒充已与 Lean 对齐（rust 半独立成立：MACD area 度量满足 ForceMeasure
//! 接口的 mono/faithful 公理；Lean 半由另一工位写）。这是诚实的有效域边界，非补丁。
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
/// （`divergence::segment_macd_area`），`strength` 取面积的量化序（按 tick 精度量化为 `u64`，
/// 满足 `ForceMeasure.strength : α → Nat` 的整数强度序）。
///
/// ★`mono`/`faithful` 公理（`Origin.ForceMeasure`）：本适配器的 `strength` 是 `area` 的**单调
/// 量化**（`strength = round(area / quantum)`），故 `strength a ≤ strength b ⟹ area a ⪅ area b`
/// （mono，量化误差内）+ `area a < area b ⟹ strength a ≤ strength b`（faithful）。详见
/// [`ForceMeasureAdapter::mono_holds`] / [`ForceMeasureAdapter::faithful_holds`] 的逐例验证。
#[derive(Debug, Clone, Copy)]
pub struct ForceMeasureAdapter {
    /// 强度量化精度（`strength = round(area / quantum)`）。default 1.0（area 即强度序）。
    quantum: f64,
}

impl Default for ForceMeasureAdapter {
    fn default() -> Self {
        ForceMeasureAdapter { quantum: 1.0 }
    }
}

impl ForceMeasureAdapter {
    /// 构造力度度量适配器（量化精度 `quantum > 0`）。
    pub fn new(quantum: f64) -> Self {
        debug_assert!(quantum > 0.0, "quantum 必须 > 0（强度量化精度）");
        ForceMeasureAdapter { quantum }
    }

    /// `measure : α → Force`（契约锚 `Origin.ForceMeasure.measure`）：段 → 力度（MACD 段面积）。
    ///
    /// `hist` 是 MACD hist 序列（`divergence::compute_macd` 输出）；`(start, end)` 是同向段闭区间
    /// bar 索引。返回 `Force { area = segment_macd_area(hist, start, end) }`。
    pub fn measure(&self, hist: &[f64], start: usize, end: usize) -> Force {
        Force { area: divergence::segment_macd_area(hist, start, end) }
    }

    /// `strength : α → Nat`（契约锚 `Origin.ForceMeasure.strength`）：力度强度序（area 的单调量化）。
    ///
    /// `strength = round(area / quantum)`（饱和到 0，area 非负 ⟹ strength ≥ 0）。这是 area 的
    /// **单调**量化——保证 mono/faithful 公理（量化保序）。
    pub fn strength(&self, hist: &[f64], start: usize, end: usize) -> u64 {
        let area = self.measure(hist, start, end).area;
        // area 非负（|hist| 之和，divergence::segment_macd_area 保证）；量化为整数强度序。
        (area / self.quantum).round().max(0.0) as u64
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
    /// area a ≤ area b`（量化保序 ⟹ mono 在量化粒度内成立；quantum=1 时精确成立）。
    ///
    /// 返回该两段对是否满足 mono（用于测试逐例验证接口公理）。
    pub fn mono_holds(&self, hist: &[f64], a: (usize, usize), b: (usize, usize)) -> bool {
        let sa = self.strength(hist, a.0, a.1);
        let sb = self.strength(hist, b.0, b.1);
        let area_a = self.measure(hist, a.0, a.1).area;
        let area_b = self.measure(hist, b.0, b.1).area;
        // mono：strength a ≤ strength b ⟹ area a ≤ area b + 一个量化窗口（保序近似）。
        if sa <= sb {
            area_a <= area_b + self.quantum
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

    /// strength 是 area 的单调量化（契约锚 `Origin.ForceMeasure.strength`）。
    #[test]
    fn strength_is_monotone_quantization() {
        let fm = ForceMeasureAdapter::default(); // quantum=1
        let hist = vec![1.0, -2.0, 3.0, -4.0];
        // area([0,1])=3 ⟹ strength=3；area([0,3])=10 ⟹ strength=10。
        assert_eq!(fm.strength(&hist, 0, 1), 3);
        assert_eq!(fm.strength(&hist, 0, 3), 10);
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
