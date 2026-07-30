//! 五操作的群作用类型化（架构 §4.4/§5）。
//!
//! GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3——名分：现役（详见
//! `spiral/mod.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! ## 类型系统层面的诚实（架构 §2.3/§4.4）
//! `Enter/Flip/CostReduce/Cover` 携带 `GroupAction` 语义；`Liquidate` 显式**不携带**
//! ——A 强平在类型上就**不是**群作用（市场被动作用），gap G2 在编译期显形（`group_action`
//! 对 `Liquidate` 返回 `None`），而非藏在命令式 for 循环里。这是 §2.3 "A 进不了群" 的
//! 群论形式：合法操作单子 `M = {h_fwd, τ_seam, σ⁻¹}` 三生成元，`A ∉ M`。
//!
//! ## 操作 → 群元素 → Δr 映射（架构 §5.7）
//! | 操作 | 群元素 | Δr | 实现（engine/accounting）|
//! |------|--------|-----|--------------------------|
//! | F `Enter` | σ 塔起点 @ source | 跨级别定位 | `accounting::make_root` |
//! | C `Flip` | τ（`τ²=e`）| φ=0 翻 ε | engine `GroupAction::ChiralSeam.apply` |
//! | E `CostReduce` | σ⁻¹（r↦r−1）| **Δr=−1 直接落点** | `accounting::try_spawn_cost_gated`（σ⁻¹∘τ）|
//! | D `Cover` | σ 逆向闭合 | Δr=−1 闭合在子层 | `accounting::close_voice`（recover）|
//! | A `Liquidate` | **非群** | **缺 Δr（G2）** | `accounting::close_voice`（liq）|

use super::state::GroupAction;

/// 五操作（架构 §4.4）。前四个是 `GroupAction` 的解释；A 是非群（市场被动，gap G2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// F：根 voice 诞生 @ source（σ 塔起点，§5.1）。
    Enter { source: usize },
    /// C：τ 作用（root flip，φ=0，M=N，§5.2）。
    Flip { rid: usize },
    /// E：σ⁻¹（spawn 子 voice @ r−1，Δr=−1 落点，§5.3）。
    CostReduce { parent: usize },
    /// D：σ 逆向闭合（子层 type1 向心确认返父，§5.4）。
    Cover { vid: usize },
    /// A：⚠ **非 GroupAction**（capital 耗尽，市场作用，gap G2，§5.5）。
    Liquidate { vid: usize },
}

impl Operation {
    /// 操作对应的合法群生成元（`M = {h_fwd, τ_seam, σ⁻¹}`）。
    ///
    /// **`Liquidate` 返回 `None`**——A 强平不是群元素（市场被动），gap G2 在类型层显形。
    /// F `Enter` 返回 `RadialAscend`（σ 塔向上生长的起点语义）；C `Flip` 返回
    /// `ChiralSeam`（τ）；E `CostReduce` / D `Cover` 返回 `RadialDescend`（σ⁻¹，Δr=−1）。
    pub fn group_action(self) -> Option<GroupAction> {
        match self {
            Operation::Enter { .. } => Some(GroupAction::RadialAscend),
            Operation::Flip { .. } => Some(GroupAction::ChiralSeam),
            Operation::CostReduce { .. } => Some(GroupAction::RadialDescend),
            Operation::Cover { .. } => Some(GroupAction::RadialDescend),
            // A 强平：非群（市场被动作用，gap G2 显形）。
            Operation::Liquidate { .. } => None,
        }
    }

    /// 是否纯群操作（`A ∉ M` ⟹ false——诚实标记，不伪装成群作用）。
    pub fn is_group_operation(self) -> bool {
        self.group_action().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquidate_is_not_group_operation() {
        // gap G2：A 强平在类型上非群作用（合法操作单子 M 不含 A）。
        assert!(!Operation::Liquidate { vid: 0 }.is_group_operation());
        assert_eq!(Operation::Liquidate { vid: 0 }.group_action(), None);
    }

    #[test]
    fn four_ops_are_group_actions() {
        // F/C/E/D 全部携带 GroupAction（M = {h_fwd, τ_seam, σ⁻¹}）。
        assert!(Operation::Enter { source: 3 }.is_group_operation());
        assert!(Operation::Flip { rid: 0 }.is_group_operation());
        assert!(Operation::CostReduce { parent: 0 }.is_group_operation());
        assert!(Operation::Cover { vid: 0 }.is_group_operation());
    }

    #[test]
    fn cost_reduce_and_cover_are_sigma_inverse() {
        // E/D 是 σ⁻¹（Δr=−1）——H¹ 生成元。
        assert_eq!(
            Operation::CostReduce { parent: 0 }.group_action(),
            Some(GroupAction::RadialDescend)
        );
        assert_eq!(
            Operation::Cover { vid: 0 }.group_action(),
            Some(GroupAction::RadialDescend)
        );
    }

    #[test]
    fn flip_is_chiral_seam() {
        assert_eq!(
            Operation::Flip { rid: 0 }.group_action(),
            Some(GroupAction::ChiralSeam)
        );
    }
}
