//! Θ_voice + Θ_risk + Θ_exec 子模块（reference-theta-v0.md:39-54）——部分实装 + 子任务。
//!
//! ## 范围（bit-exact 对齐 `Strict/Fugue.lean` / `RiskProj.lean` / `Op.lean` /
//! `StrategyFamily.lean`）
//!
//! 声部树 + 风险投影 + sizing + 执行。给定 Θ ⟹ 订单 O_{t+1} 唯一（StrategyFamily 元定理）。
//!
//! ## 已 bit-exact 实装（本骨架内）
//!
//! - [`pi_strict`]：完全应对策略 π（`StrictState → StrictAction` 全函数），bit-exact
//!   对齐 `Strict/Op.lean` 的 `piStrict`——9 状态（3 持仓 × 3 信号）→ 7 动作，含 wait/hold
//!   区分。这是 strategy 的可验证种子（其余子任务围绕它构建声部树/风险/执行）。
//!
//! ## 子任务清单（TaskCreate 分解）
//!
//! 1. **声部树**（:39-42，对齐 `Strict/Fugue.lean`）：根=当前最高有效决策级别 L*；最多
//!    3 层（config `max_depth`）；`σ_v=(-1)^depth`，子须 `σ_child=-σ_parent`；4 互斥动作态
//!    {close,open,hold,wait} `Σ𝟙=1`；级联关闭。资金帽深度权重（config `depth_weights`）。
//! 2. **风险投影**（:44-47，对齐 `Strict/RiskProj.lean`）：单声部风险 ρ、父子比 β、总名义
//!    上限 γ、成本倍数 κ（全 config）。结构止损（1/2买=pivot low，3买=ZG；卖镜像）。
//!    sizing：`qty=min(floor(ρ*NAV/(|entry-stop|+κ*cost)), floor(w_depth*γ*NAV/entry),
//!    parent_cap)`；`qty<=0` 不交易；lot=config。唯一总仓位 + 风险模式 μ_t 唯一。
//! 3. **执行**（:49-54）：延迟 `entry_delay_bars` 根，下一根 open 成交；费用（commission/
//!    slippage/tax，全 config）；止损成交规则；不可交易过滤；冲突顺序（止损/退出先于开仓；
//!    高 level 先；同 level 按 1/2/3；平局 timestamp/source_index）。
//!
//! ## 接口契约

use super::config::ThetaConfig;
use super::types::{Pos, Sig, StrictAction};

/// 完整结构状态 Sₗ（Strict/Op.lean `StrictState`：持仓 × 信号 = 9 状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictState {
    pub pos: Pos,
    pub sig: Sig,
}

/// 完全应对策略 π（**bit-exact 对齐 `Strict/Op.lean` `piStrict`**）。
///
/// 9 状态（3 持仓 × 3 信号）→ 7 动作全函数。每一格的映射逐字对齐 Op.lean：
/// - flat+buySide → Buy（建仓）
/// - flat+sellSide → Wait（空仓遇卖侧，裸空非缠论 §4.4，观望不动）
/// - flat+none → Wait（空仓无信号 = 等待，**非 Hold**，Op.lean codex#2 关键修正）
/// - long+buySide → Add（降成本买回）
/// - long+sellSide → Reduce（降成本减仓）
/// - long+none → Hold（持多不动，有仓位）
/// - short+buySide → Close（平空头腿）
/// - short+sellSide → Sell（空头翻转/止损）
/// - short+none → Hold（持空不动，有仓位）
///
/// 全函数：match 穷尽 9 格（Rust 编译器静态保证穷尽性，对齐 Lean total 证明义务）。
/// 边界条件：此映射只依赖 `(pos,sig)`，不依赖 config——若 Θ 引入新动作态/状态会翻转
/// （须 change request，不在 Θ v0 内）。
pub fn pi_strict(s: StrictState) -> StrictAction {
    match (s.pos, s.sig) {
        (Pos::Flat, Sig::BuySide) => StrictAction::Buy,
        (Pos::Flat, Sig::SellSide) => StrictAction::Wait,
        (Pos::Flat, Sig::None) => StrictAction::Wait,
        (Pos::Long, Sig::BuySide) => StrictAction::Add,
        (Pos::Long, Sig::SellSide) => StrictAction::Reduce,
        (Pos::Long, Sig::None) => StrictAction::Hold,
        (Pos::Short, Sig::BuySide) => StrictAction::Close,
        (Pos::Short, Sig::SellSide) => StrictAction::Sell,
        (Pos::Short, Sig::None) => StrictAction::Hold,
    }
}

/// Θ_voice + Θ_risk + Θ_exec 顶层入口（声部树 → 风险投影 → 订单）。
///
/// **骨架占位**：当前只暴露 `pi_strict` 种子；声部树/风险/执行待子任务实装。签名先冻结
/// 为 `(classification, config) -> Vec<Order>`（Order 见 types）；本占位返回空订单流。
///
/// 注：完整签名将以 classifier 的 `Classification` 为输入——为避免骨架阶段引入未实装的
/// 数据流耦合，此处先不接线（子任务实装时填充 + 接入 classifier 输出）。
pub fn plan_orders(_config: &ThetaConfig) -> Vec<super::types::Order> {
    // [子任务 TaskCreate：strategy 实装] 声部树 + 风险投影 + 执行，
    // bit-exact 对齐 Strict/{Fugue,RiskProj,StrategyFamily}.lean。
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bit-exact 对齐 Op.lean：9 状态全枚举 → 7 动作映射逐格验证（golden table）。
    /// 这是 Op.lean `piStrict` 的 Rust conformance fixture——任一格漂移 = bit-exact 失败。
    #[test]
    fn pi_strict_matches_op_lean_nine_states() {
        use Pos::*;
        use Sig::*;
        use StrictAction::*;
        let cases = [
            (Flat, BuySide, Buy),
            (Flat, SellSide, Wait),
            (Flat, Sig::None, Wait),
            (Long, BuySide, Add),
            (Long, SellSide, Reduce),
            (Long, Sig::None, Hold),
            (Short, BuySide, Close),
            (Short, SellSide, Sell),
            (Short, Sig::None, Hold),
        ];
        for (pos, sig, expected) in cases {
            assert_eq!(pi_strict(StrictState { pos, sig }), expected);
        }
    }

    /// Op.lean wait/hold 区分关键不变量：flat 永不返回 Hold，long/short+none 永远 Hold。
    #[test]
    fn wait_only_for_flat_hold_only_for_held_positions() {
        use Pos::*;
        use Sig::*;
        // flat 三态都不是 Hold（空仓观望 = Wait）。
        for sig in [BuySide, SellSide, Sig::None] {
            assert_ne!(pi_strict(StrictState { pos: Flat, sig }), StrictAction::Hold);
        }
        // 持仓 + 无信号 = Hold。
        assert_eq!(
            pi_strict(StrictState { pos: Long, sig: Sig::None }),
            StrictAction::Hold
        );
        assert_eq!(
            pi_strict(StrictState { pos: Short, sig: Sig::None }),
            StrictAction::Hold
        );
    }
}
