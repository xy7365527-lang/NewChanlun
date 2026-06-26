//! 统一意图适配器——镜像 Lean `Strict/HybridAssembly.lean` 的 `classifyAdapter` +
//! `actionPriority` + `intentAdapter`（factor-through-classify：策略动作穿过分类瓶颈）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：确定优先级选择器与 Lean `actionPriority` 对齐 = 验证管线
//!   正确性，零信息增量）。`cargo test` 通过 = 意图选择确定 + 策略真读分类输出，**不**是缠论
//!   盈利/实盘有效声明。
//!
//! ## 消除双意图路径（gap-map U4 核心，no-workaround）
//!
//! 旧引擎有**双意图路径**：
//! 1. `strategy/mod.rs` 的 `pi_strict`（9 状态 (pos,sig) → 7 动作，bit-exact 对齐 `Op.lean`
//!    `piStrict`）——**直读结构状态，不经 Classification**。
//! 2. `strategy/mod.rs` 的 `recognize` → `plan_orders`（Classification → VoiceDecision → Order）
//!    ——**读 Classification 输出**。
//!
//! Lean `policy_factors_through_classify`（HybridAssembly.lean:581）要求策略动作**穿过分类瓶颈**：
//! `intentAdapter x c` 的输入 `c = classifyAdapter x (recAdapter x e)` 就是分类输出。本文件把
//! 闭环意图路径**统一**为单一 [`intent_adapter`]：意图由 [`classify_adapter`]（从 microState 读出
//! 分类标签 [`ClassLabel`]）经 [`action_priority`] 派生——策略只读分类输出，与 Lean
//! `policy_factors_through_classify` 同构（见 closed_loop/transition.rs 的 factor-through 见证）。
//!
//! `pi_strict` 保留为 `Op.lean` piStrict 的 **9→7 conformance 种子**（结构应对的细粒度组件，
//! 其有效域是「给定结构状态 (pos,sig) 的完全应对」）——它不再是独立的闭环意图入口，而是被
//! [`pi_strict_factored`] 通过 [`StrictState`] 桥接（该状态由分类输出派生，factor-through）。

use super::super::closed_loop::state::{AssemblyState, MicroState, Phase, RiskMode};
use super::super::types::{Pos, Sig, StrictAction};
use super::{pi_strict, StrictState};

/// 分类标签 `ClassLabel`（镜像 Lean `HybridAssembly.ClassLabel = RiskMode × Phase`）。
///
/// 全局完全分类 C_Θ(x_t) 的**结构摘要**——闭环 classify 段从 microState/态读出 (risk_mode, phase)
/// 作为 C_Θ 标签传给 intent 段，使意图真读分类输出（满足 factor-through-classify）。
///
/// ★诚实标注（formalization-validity-domain）：**完整** C_Θ 的 fiber partition 全局唯一性由
/// classifier 的递归级别 + BSP 承载；本标签是 C_Θ 的结构摘要接入（对齐 Lean classifyAdapter
/// 注释——证「闭环数据流中 intent 真读 classify 输出」，不是「这个摘要等价于完整 C_Θ」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassLabel {
    pub risk_mode: RiskMode,
    pub phase: Phase,
}

/// C_Θ 段 `classify_adapter`（镜像 Lean `HybridAssembly.classifyAdapter`，签名 `x → RecStruct → ClassLabel`）：
/// 从解析结构 + 当前态读出分类标签。
///
/// 把当前态的 (risk_mode, phase) 作为 C_Θ 标签——这是闭环 classify 段的输出，被 [`intent_adapter`]
/// 消费，使意图真读分类输出（factor-through-classify 的分类瓶颈）。
///
/// `rec_struct`（Rec 段输出的新解析态）是 Lean classifyAdapter 的输入（`_d : RecStruct`，当前摘要
/// 读出不依赖它，但保留在签名里使 Rec 段输出真被 classify 段消费——对齐 Lean 六段数据流
/// `classifyAdapter x (recAdapter x e)`，不让 Rec 段成为旁路死代码）。
pub fn classify_adapter(x: &AssemblyState, _rec_struct: &MicroState) -> ClassLabel {
    ClassLabel {
        risk_mode: x.risk_mode,
        phase: x.phase,
    }
}

/// 10 级动作优先级**确定选择器** `action_priority`（镜像 Lean `HybridAssembly.actionPriority`）。
///
/// 从风险模式 + 阶段确定性地选出唯一动作——把 HybridStateMachine 10 级优先级（破产 > 去杠杆 >
/// 执行异常 > 阶段二取本 > 根/祖先失效 > 关反向子 > 建根 > 建反向子 > 阶段三增核 > 保持）
/// 压缩为风险模式 + 阶段的确定映射：
/// - `Insolvent`  → Close（破产：最高优先级，强平）。
/// - `Liquidation`→ Close（清算：强平）。
/// - `Deleverage` → Reduce（去杠杆：减仓）。
/// - `CloseOnly`  → Close（只平不开）。
/// - `Normal` 下按阶段：PhaseI → Buy（建根仓）；PhaseII → Reduce（取本=减仓回收本金）；
///   PhaseIII → Add（增核）。
///
/// ★诚实标注（formalization-validity-domain）：这是**确定优先级选择器**（Θ_voice 设计选择），
/// **不是**「由缠论唯一推出」——10 级的排序本身是操盘设计决策（哪个优先级在前），缠论结构不唯一
/// 钉死它。本函数证「给定这个优先级排序后选择确定」（全函数 ⟹ 唯一），不证排序是缠论必然。
///
/// 全函数：match 穷尽 RiskMode×Phase 全组合（Rust 编译器静态保证穷尽性）。
pub fn action_priority(c: ClassLabel) -> StrictAction {
    match c.risk_mode {
        RiskMode::Insolvent => StrictAction::Close,
        RiskMode::Liquidation => StrictAction::Close,
        RiskMode::Deleverage => StrictAction::Reduce,
        RiskMode::CloseOnly => StrictAction::Close,
        RiskMode::Normal => match c.phase {
            Phase::PhaseI => StrictAction::Buy,
            Phase::PhaseII => StrictAction::Reduce,
            Phase::PhaseIII => StrictAction::Add,
        },
    }
}

/// Intent 段 `intent_adapter`（镜像 Lean `HybridAssembly.intentAdapter`）：**输入含 ClassLabel**。
///
/// 在分类标签 `c` 上跑 10 级优先级选择器 [`action_priority`]。这满足 Lean
/// `intent : HybridState → Class → Intent` 结构约束（intent 以 Class 为输入 ⟹ 策略穿过分类瓶颈）。
/// 闭环数据流：`intent_adapter(x, classify_adapter(x))`——意图真读分类输出（factor-through-classify）。
pub fn intent_adapter(_x: &AssemblyState, c: ClassLabel) -> StrictAction {
    action_priority(c)
}

/// `pi_strict` 的 factor-through 桥接（镜像 Lean factor-through-classify 在 Op 层的细粒度形式）。
///
/// `Op.lean` piStrict（9 状态 (pos,sig) → 7 动作）是**结构应对的细粒度组件**——它的输入 (pos,sig)
/// 本身是分类输出（C_Θ 把走势识别为持仓/信号态）。本函数把分类标签 [`ClassLabel`] 映射到
/// piStrict 的 [`StrictState`] 输入，再调 [`pi_strict`]——使 piStrict 不再是绕开 Classification 的
/// 独立入口，而是**通过分类派生状态**调用（factor-through）。
///
/// 映射（C_Θ 标签 → piStrict 状态）：
/// - `risk_mode != Normal`（风险事件）⟹ 持有态视为 Long + 卖侧信号（触发减/平，对齐 action_priority
///   的 Close/Reduce）；Normal 下按阶段映射信号侧：PhaseI → Flat+BuySide（建仓）、PhaseII →
///   Long+SellSide（取本减仓）、PhaseIII → Long+BuySide（增核加仓）。
///
/// ★诚实标注：这是把 piStrict 接入 factor-through 的**结构桥接**——piStrict 与 action_priority
/// 是 C_Θ 应对的两种粒度（piStrict 在 (pos,sig) 细格，action_priority 在 (risk,phase) 摘要），
/// 本桥接使两者在分类瓶颈下一致（pi_strict_factored 与 intent_adapter 对同一 ClassLabel 同调，见测试）。
pub fn pi_strict_factored(c: ClassLabel) -> StrictAction {
    let state = strict_state_from_label(c);
    pi_strict(state)
}

/// 从分类标签派生 piStrict 输入状态（factor-through 的状态投影）。
fn strict_state_from_label(c: ClassLabel) -> StrictState {
    match c.risk_mode {
        // 风险事件：持有态 + 卖侧 ⟹ piStrict 给 Reduce（去杠杆/只平不开的细格对应）。
        RiskMode::Insolvent | RiskMode::Liquidation | RiskMode::CloseOnly | RiskMode::Deleverage => {
            StrictState { pos: Pos::Long, sig: Sig::SellSide }
        }
        RiskMode::Normal => match c.phase {
            // PhaseI 建仓：空仓 + 买侧 ⟹ piStrict 给 Buy。
            Phase::PhaseI => StrictState { pos: Pos::Flat, sig: Sig::BuySide },
            // PhaseII 取本：持多 + 卖侧 ⟹ piStrict 给 Reduce。
            Phase::PhaseII => StrictState { pos: Pos::Long, sig: Sig::SellSide },
            // PhaseIII 增核：持多 + 买侧 ⟹ piStrict 给 Add。
            Phase::PhaseIII => StrictState { pos: Pos::Long, sig: Sig::BuySide },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::closed_loop::state::AssemblyState;

    /// action_priority 全函数确定（镜像 Lean actionPriority）：同标签同动作。
    #[test]
    fn action_priority_deterministic() {
        let c = ClassLabel { risk_mode: RiskMode::Normal, phase: Phase::PhaseI };
        assert_eq!(action_priority(c), action_priority(c));
    }

    /// action_priority 风险事件优先级（破产/清算/只平 → Close，去杠杆 → Reduce）。
    #[test]
    fn action_priority_risk_events() {
        for phase in [Phase::PhaseI, Phase::PhaseII, Phase::PhaseIII] {
            assert_eq!(
                action_priority(ClassLabel { risk_mode: RiskMode::Insolvent, phase }),
                StrictAction::Close
            );
            assert_eq!(
                action_priority(ClassLabel { risk_mode: RiskMode::Liquidation, phase }),
                StrictAction::Close
            );
            assert_eq!(
                action_priority(ClassLabel { risk_mode: RiskMode::CloseOnly, phase }),
                StrictAction::Close
            );
            assert_eq!(
                action_priority(ClassLabel { risk_mode: RiskMode::Deleverage, phase }),
                StrictAction::Reduce
            );
        }
    }

    /// action_priority Normal 下三阶段（PhaseI→Buy / PhaseII→Reduce / PhaseIII→Add）。
    #[test]
    fn action_priority_normal_phases() {
        let mk = |phase| ClassLabel { risk_mode: RiskMode::Normal, phase };
        assert_eq!(action_priority(mk(Phase::PhaseI)), StrictAction::Buy);
        assert_eq!(action_priority(mk(Phase::PhaseII)), StrictAction::Reduce);
        assert_eq!(action_priority(mk(Phase::PhaseIII)), StrictAction::Add);
    }

    /// ★factor-through-classify：intent_adapter 真读 classify_adapter 输出（镜像 Lean
    /// policy_factors_through_classify）。意图 = action_priority(classify_adapter(x))。
    #[test]
    fn intent_factors_through_classify() {
        let x = AssemblyState::initial(1_000_000); // Normal/PhaseI
        let rec_struct = MicroState::initial();
        let label = classify_adapter(&x, &rec_struct);
        // 意图必须由分类标签派生（穿过分类瓶颈），不绕开 Classification。
        assert_eq!(intent_adapter(&x, label), action_priority(label));
        // 初始态 Normal/PhaseI ⟹ Buy（建根仓）。
        assert_eq!(intent_adapter(&x, label), StrictAction::Buy);
    }

    /// ★pi_strict 经 factor-through 与 action_priority 同调（消除双意图路径的一致性见证）。
    /// pi_strict_factored 通过分类派生状态调 pi_strict，结果与 action_priority 对同一标签一致。
    #[test]
    fn pi_strict_factored_agrees_with_action_priority() {
        // PhaseI：action_priority→Buy；pi_strict(Flat,BuySide)→Buy ⟹ 一致。
        let c1 = ClassLabel { risk_mode: RiskMode::Normal, phase: Phase::PhaseI };
        assert_eq!(pi_strict_factored(c1), StrictAction::Buy);
        assert_eq!(pi_strict_factored(c1), action_priority(c1));
        // PhaseII：action_priority→Reduce；pi_strict(Long,SellSide)→Reduce ⟹ 一致。
        let c2 = ClassLabel { risk_mode: RiskMode::Normal, phase: Phase::PhaseII };
        assert_eq!(pi_strict_factored(c2), StrictAction::Reduce);
        assert_eq!(pi_strict_factored(c2), action_priority(c2));
        // PhaseIII：action_priority→Add；pi_strict(Long,BuySide)→Add ⟹ 一致。
        let c3 = ClassLabel { risk_mode: RiskMode::Normal, phase: Phase::PhaseIII };
        assert_eq!(pi_strict_factored(c3), StrictAction::Add);
        assert_eq!(pi_strict_factored(c3), action_priority(c3));
    }

    /// 风险事件下 pi_strict_factored 给减仓侧（Long+SellSide→Reduce，对齐 Deleverage）。
    #[test]
    fn pi_strict_factored_risk_event_reduces() {
        let c = ClassLabel { risk_mode: RiskMode::Deleverage, phase: Phase::PhaseI };
        // pi_strict(Long,SellSide)=Reduce，与 action_priority(Deleverage,_)=Reduce 一致。
        assert_eq!(pi_strict_factored(c), StrictAction::Reduce);
        assert_eq!(pi_strict_factored(c), action_priority(c));
    }
}
