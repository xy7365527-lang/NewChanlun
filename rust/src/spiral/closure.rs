//! H¹ 闭合（P-close）：三类闭合区分 + `Δr=−1` 兑现（架构 §6）。
//!
//! ## 闭合 = 否定之否定 = H¹ 的 `Δr=−1` 在次级别（架构 §6.1）
//! 势耗尽**不在开仓级别 r=k 原地闭合**（NR-1 否定同级别 σ=e 闭合），而是**向心下沉
//! 到次级别 r=k−1**（φ 归零）。`H¹(D∞,ℝ₋)≅ℝ`，生成元 = 跨级别位移率 `Δr`（定理 A1）。
//!
//! ## 三类闭合（关键：不能一律 Δr=−1，架构 §6.3）
//! | ClosureKind | 判据 | 兑现 |
//! |-------------|------|------|
//! | `CrossLevel` | sigma 型张力，势耗尽向心下沉 | `Δr=−1`（E spawn / D 子层回补）|
//! | `ValidityDomain` | 定义域>有效域，regime 依赖 | 诚实等级重述（不动 Δr）|
//! | `Category` | 两定理范畴混淆 | 范畴分层（根空头 MtM ⊥ 子空头 frozen）|
//!
//! 运行时硬守卫 `prove_cross_level_closure`（每次 CrossLevel 闭合 Δr=−1）在 `prove.rs`。

use crate::trading::types::Polarity;

/// 三类闭合（架构 §6.3）。`Δr=−1` 只对 `CrossLevel` 兑现。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosureKind {
    /// sigma 型：`Δr=−1` 投影（E 降成本 / D 子层回补 / close_voice）——H¹ 生成元。
    CrossLevel,
    /// 定义域>有效域：诚实等级重述（A 强平 regime，不兑现 Δr，标 regime 有效域）。
    ValidityDomain,
    /// 范畴分层：根空头 MtM ⊥ 子空头 frozen（T8），分别处理不强行统一。
    Category,
}

impl ClosureKind {
    /// 分类一个声部的离场闭合（架构 §6.2 step 2）。
    ///
    /// - 子 voice（非根）的自层 type1 向心确认回补 / 降成本 spawn ⇒ `CrossLevel`（Δr=−1）。
    /// - 根空头（parent=None ∧ Short）= MtM 外部负债 ⊥ 子空头 frozen ⇒ `Category`
    ///   （T8 范畴分层：根空头不嵌套降成本）。
    /// - 根的 A 强平（市场被动，regime 依赖）⇒ `ValidityDomain`。
    pub fn classify(is_root: bool, dir: Polarity, is_liquidation: bool) -> ClosureKind {
        if is_liquidation {
            // A 强平 = 市场被动作用，非群操作（gap G2）；regime 依赖有效域。
            ClosureKind::ValidityDomain
        } else if is_root && dir == Polarity::Short {
            // 根空头 MtM（外部负债）⊥ 子空头 frozen（内部）——范畴混淆，分层处理。
            ClosureKind::Category
        } else {
            // 子层向心下沉闭合（E/D），势耗尽 → 次级别 φ=0。
            ClosureKind::CrossLevel
        }
    }

    /// 是否兑现 `Δr=−1`（仅 CrossLevel）。
    pub fn yields_delta_r(self) -> bool {
        matches!(self, ClosureKind::CrossLevel)
    }
}

/// P-close 的 `Δr=−1` 落点：闭合在次级别 `r=K−1`（向心下沉，A1 H¹ 生成元）。
/// `K=0`（a0 底层）无次级别 ⇒ None（NR-5：σ⁻¹ 在 r=0 无像）。
pub fn p_close_descend_level(k: usize) -> Option<usize> {
    k.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_short_is_category() {
        // 根空头 = MtM 外部负债 ⊥ 子空头 frozen ⇒ Category（T8 不嵌套降成本）。
        assert_eq!(
            ClosureKind::classify(true, Polarity::Short, false),
            ClosureKind::Category
        );
    }

    #[test]
    fn sub_voice_is_cross_level() {
        // 子 voice 自层闭合 = Δr=−1 向心下沉 ⇒ CrossLevel。
        assert_eq!(
            ClosureKind::classify(false, Polarity::Short, false),
            ClosureKind::CrossLevel
        );
        assert!(ClosureKind::classify(false, Polarity::Long, false).yields_delta_r());
    }

    #[test]
    fn liquidation_is_validity_domain() {
        // A 强平 = 市场被动 regime ⇒ ValidityDomain（不兑现 Δr）。
        let k = ClosureKind::classify(true, Polarity::Short, true);
        assert_eq!(k, ClosureKind::ValidityDomain);
        assert!(!k.yields_delta_r());
    }

    #[test]
    fn descend_level_floor() {
        assert_eq!(p_close_descend_level(5), Some(4));
        assert_eq!(p_close_descend_level(0), None); // a0 无次级别
    }
}
