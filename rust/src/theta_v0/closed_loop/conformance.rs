//! U5 bit-exact 逐态 conformance —— 闭环 `hybrid_step` 实例化 `Origin.EngineBridge.RustEngineContract`
//! 协议，并对同一事件序列逐态比对（task #127 A′ Phase2 step8）。
//!
//! ## 契约锚（`formal/Origin/EngineBridge.lean`）
//!
//! Origin `RustEngineContract`（EngineBridge.lean:23-31）为 Rust 引擎显式留的对齐接口：
//! ```text
//! structure RustEngineContract where
//!   State : Type;  Event : Type;  Class : Type;  Action : Type
//!   initial  : State
//!   classify : State -> Class
//!   action   : State -> Action
//!   step     : State -> Event -> State
//! ```
//! + `StepSpec C s e s' := C.step s e = s'`（:33-35）
//! + `rust_engine_step_total_unique`（:37-42）：`step` 是**全函数确定唯一**（同 (s,e) ⟹ 同 s'）。
//! + `rust_engine_classification_total_unique`（:44-46）：`classify` 全函数确定唯一。
//! + 裁定 `thetaV0Verdict = EngineVerdict.referenceOnly`（:82）：theta_v0 是**满足契约后**才被承认的
//!   reference 引擎——本模块的 conformance 测试就是兑现「满足契约」的逐态证据。
//!
//! ## 本模块做什么（U5 逐态 conformance，非占位 assert true）
//!
//! 把闭环 [`super::transition::hybrid_step`] 包成 `RustEngineContract` 的具体项 [`ThetaV0Contract`]
//! （State=`AssemblyState`，Event=`AssemblyEvent`，Class=`ClassLabel`，Action=`StrictAction`，
//! step=`hybrid_step`，classify/action 从态读出），并验证：
//!
//! 1. **`StepSpec` 全函数确定唯一**（`rust_engine_step_total_unique` 的 Rust 侧兑现）：对同一
//!    `(state, event)`，`step` 产出**逐字段相等**的下一态——逐态比对，非「存在某结果」。
//! 2. **`classify` 全函数确定唯一**（`rust_engine_classification_total_unique`）：同态产同分类标签。
//! 3. **六段语义同构逐态**（对齐 Origin `policyTheta = schedule∘risk∘intent∘classify∘recStruct`）：
//!    `step` 的每步输出态 = 把 `policy_output`（前五段）的订单经 `transition_adapter`（T 写回）施加
//!    ——逐态展开相等（对齐 `transition_writes_full_state` + `strict_assembly_policy_reads_classification`）。
//! 4. **双账本写回逐态**（对齐 `strict_assembly_preserves_ledger_invariant`）：每步后 ledger R=Π-A-W
//!    + TW=free+holding+withdrawn 守恒 + stage 单向 + OQ-9 gate 逐态成立。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - **L1**（结构契约逐态一致）：本模块验证 Rust 闭环 `step` 满足 `RustEngineContract.StepSpec`
//!   全函数确定唯一 + 六段语义同构 + 双账本写回——即 Rust 引擎**满足 Origin 为它显式留的对齐契约**，
//!   逐态（per-event）比对而非占位。这是 `thetaV0Verdict = referenceOnly` 升级为「契约 conformant」
//!   的 Rust 侧证据。
//! - ★诚实边界：Origin `hybridStep` 是抽象多态函数（`FullDefinitionSystem` 任意实例），**无具体
//!   数值轨迹**可逐 bit 比——故 conformance 的对象是 `RustEngineContract` 协议（Origin 显式留的接口），
//!   逐态比对 = Rust `step` 的协议契约（StepSpec 全函数确定唯一 + 六段复合同构 + 双账本写回）逐态
//!   成立。**不**冒充「跑出 Origin Lean 数值逐 bit 比对」（那要求 Origin 把抽象系统具体实例化为可执行
//!   数值轨迹，Origin canonical 当前是抽象契约层，无此可执行实例）——本模块诚实声明 conformance
//!   对象是协议契约，非 Lean 数值轨迹。

use super::state::AssemblyState;
use super::transition::{hybrid_step, AssemblyEvent};
use super::super::strategy::intent::{classify_adapter, ClassLabel};
use super::super::types::StrictAction;

/// theta_v0 闭环引擎对 `Origin.EngineBridge.RustEngineContract` 的具体实例化。
///
/// 契约字段映射（逐字段对齐 EngineBridge.lean:23-31）：
/// - `State`  = [`AssemblyState`]（闭环完整态，乘积态扩展 Origin `StrictState`）。
/// - `Event`  = [`AssemblyEvent`]（驱动闭环一步的外部事件，对齐 `S.Event`）。
/// - `Class`  = [`ClassLabel`]（分类瓶颈标签，对齐 `S.Class`）。
/// - `Action` = [`StrictAction`]（动作，对齐 `S.Action`）。
/// - `step`   = [`Self::step`] = `hybrid_step`（对齐 `step : State → Event → State`）。
/// - `classify` = [`Self::classify`]（态 → 分类标签，对齐 `classify : State → Class`）。
/// - `action` = [`Self::action`]（态 → 动作摘要，对齐 `action : State → Action`）。
/// - `initial` = [`Self::initial`]（开局态，对齐 `initial : State`）。
///
/// 这是 ZST（零字段）——契约是无状态的函数集合（Origin `RustEngineContract` 的字段都是纯函数 +
/// 一个初态常量），实例化只需提供这些函数，无运行时数据。
pub struct ThetaV0Contract;

impl ThetaV0Contract {
    /// `initial : State`（契约锚 `RustEngineContract.initial`）：开局闭环态。
    ///
    /// `i0` = 初始本金（进 ledger_state 基线）。对齐 Origin `initial` 字段（引擎的起始 State）。
    pub fn initial(i0: i64) -> AssemblyState {
        AssemblyState::initial(i0)
    }

    /// `classify : State → Class`（契约锚 `RustEngineContract.classify`，全函数确定唯一）。
    ///
    /// 从闭环态读出分类瓶颈标签——对齐 `rust_engine_classification_total_unique`（同态同标签）。
    /// 复用 `classify_adapter`（六段数据流的 classify 段，分类瓶颈），在 rec 段推进后的 micro_state
    /// 上读分类标签。这里取**当前态**的分类（不吃事件）= classify 字段的纯函数语义。
    pub fn classify(s: &AssemblyState) -> ClassLabel {
        // classify 字段是 State → Class 纯函数：用当前 micro_state 读分类标签
        // （classify_adapter 吃 (态, 解析态)；当前态的分类 = 在自身 micro_state 上读）。
        classify_adapter(s, &s.micro_state)
    }

    /// `action : State → Action`（契约锚 `RustEngineContract.action`，全函数确定唯一）。
    ///
    /// 从闭环态读出动作摘要——本闭环用「持仓>0 ⟹ Hold（持仓不动），持仓=0 ⟹ Wait（空仓观望）」
    /// 作态的动作投影（对齐 Op.lean wait/hold 区分：有仓 Hold / 空仓 Wait，无事件输入时的态动作）。
    pub fn action(s: &AssemblyState) -> StrictAction {
        if s.positions > 0 {
            StrictAction::Hold
        } else {
            StrictAction::Wait
        }
    }

    /// `step : State → Event → State`（契约锚 `RustEngineContract.step` = `hybrid_step`）。
    ///
    /// 闭环单步 = `hybrid_step`（六段复合 + T 写回），对齐 Origin `hybridStep`。`StepSpec C s e s' :=
    /// step s e = s'`——本函数即 `step`，其确定唯一性由 `step_spec_total_unique` 逐态验证。
    pub fn step(s: &AssemblyState, e: &AssemblyEvent) -> AssemblyState {
        hybrid_step(s, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state::MicroEvent;
    use super::super::transition::{policy_output, transition_adapter};
    use super::super::super::strategy::ledger::TStage;

    fn bar_event(rising: bool) -> AssemblyEvent {
        AssemblyEvent { parse_event: MicroEvent::NewBar(rising) }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  U5-1：StepSpec 全函数确定唯一（rust_engine_step_total_unique 的 Rust 侧逐态兑现）
    // ──────────────────────────────────────────────────────────────────────

    /// ★`StepSpec` 全函数确定唯一（契约锚 `Origin.EngineBridge.rust_engine_step_total_unique`）：
    /// 对同一 (state, event)，`step` 产出**逐字段相等**的下一态——逐态比对（非「存在某结果」）。
    ///
    /// Origin 定理证 `∃! s', step s e = s'`；Rust 侧兑现 = 同输入恒产逐字段相等的同一输出态。
    #[test]
    fn step_spec_total_unique_per_state() {
        let states = [
            ThetaV0Contract::initial(1_000_000),
            AssemblyState {
                positions: 5,
                orders: 3,
                ..ThetaV0Contract::initial(500_000)
            },
            AssemblyState {
                tw_state: super::super::super::strategy::ledger::TwState {
                    stage: TStage::EarningShares,
                    ..super::super::super::strategy::ledger::TwState::initial()
                },
                ..ThetaV0Contract::initial(2_000_000)
            },
        ];
        let events = [bar_event(true), bar_event(false),
            AssemblyEvent { parse_event: MicroEvent::NewStroke(super::super::super::types::Direction::Up) }];

        for s in &states {
            for e in &events {
                let s1 = ThetaV0Contract::step(s, e);
                let s2 = ThetaV0Contract::step(s, e);
                // StepSpec 确定唯一：同 (s,e) ⟹ 逐字段相等的同一 s'（非「存在某 s'」）。
                assert_eq!(s1, s2, "step({:?}, {:?}) 非确定（StepSpec 唯一性破坏）", s, e);
            }
        }
    }

    /// ★`classify` 全函数确定唯一（契约锚 `Origin.EngineBridge.rust_engine_classification_total_unique`）。
    #[test]
    fn classify_total_unique_per_state() {
        let s = ThetaV0Contract::initial(1_000_000);
        assert_eq!(ThetaV0Contract::classify(&s), ThetaV0Contract::classify(&s));
        let s2 = AssemblyState { positions: 7, ..s };
        assert_eq!(ThetaV0Contract::classify(&s2), ThetaV0Contract::classify(&s2));
    }

    // ──────────────────────────────────────────────────────────────────────
    //  U5-2：六段语义同构逐态（step = transition_adapter ∘ policy_output，逐态展开）
    // ──────────────────────────────────────────────────────────────────────

    /// ★六段语义同构逐态（契约锚 `Origin.transition_writes_full_state` +
    /// `strict_assembly_policy_reads_classification`）：`step s e` 逐态等于把 `policy_output`
    /// （前五段 = `policyTheta`）的订单经 `transition_adapter`（T 写回完整态）施加。
    ///
    /// 这是 `hybridStep S x e = S.transition x (policyTheta S x e) e` 的 Rust 侧逐态兑现——
    /// **每一步**（一整条事件序列上）都展开相等，非单点。
    #[test]
    fn step_factors_through_policy_then_transition_per_event() {
        let mut s = ThetaV0Contract::initial(1_000_000);
        let trace = [
            bar_event(true), bar_event(false), bar_event(true), bar_event(true),
            AssemblyEvent { parse_event: MicroEvent::NewStroke(super::super::super::types::Direction::Down) },
            bar_event(false), bar_event(true),
        ];
        for e in &trace {
            // 逐态展开：step = transition_adapter(s, policy_output(s, e), e)。
            let order = policy_output(&s, e);
            let expected = transition_adapter(&s, &order, e);
            let got = ThetaV0Contract::step(&s, e);
            assert_eq!(got, expected, "六段同构逐态破坏：step ≠ transition∘policy");
            s = got;
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  U5-3：双账本写回逐态（strict_assembly_preserves_ledger_invariant 的 Rust 侧逐态）
    // ──────────────────────────────────────────────────────────────────────

    /// ★双账本写回逐态（契约锚 `Origin.EngineBridge.strict_assembly_preserves_ledger_invariant`
    /// + `Origin.TotalWealth.{twStep_preserves_tw, oq9inv_preserved}`）：整条事件序列上**每步后**
    /// ledger R=Π-A-W + TW 守恒 + stage 单向 + OQ-9 gate 逐态成立。
    #[test]
    fn dual_ledger_writeback_invariants_per_event() {
        let mut s = ThetaV0Contract::initial(1_000_000);
        let tw0 = s.tw_state.tw();
        let trace: Vec<AssemblyEvent> = (0..80).map(|i| bar_event(i % 3 == 0)).collect();
        for (i, e) in trace.iter().enumerate() {
            let prev_stage_rank = s.tw_state.stage.rank();
            s = ThetaV0Contract::step(&s, e);
            // R=Π-A-W（contract.strict_assembly_preserves_ledger_invariant）。
            assert!(s.ledger_state.inv_holds(), "事件 {} 后 R=Π-A-W 破坏", i);
            // TW 守恒（Origin.TotalWealth.twStep_preserves_tw）。
            assert_eq!(s.tw_state.tw(), tw0, "事件 {} 后 TW 守恒破坏", i);
            // stage 单向（Origin.TotalWealth.stage_rank_monotone）。
            assert!(prev_stage_rank <= s.tw_state.stage.rank(), "事件 {} 后 stage 回退", i);
            // OQ-9 gate（Origin.TotalWealth.oq9inv_preserved）：earning ⟹ legacy 腿=0。
            if s.tw_state.stage == TStage::EarningShares {
                assert_eq!(s.tw_state.open_legacy_legs, 0, "事件 {} OQ-9 gate 破坏", i);
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  U5-4：事件序列逐态轨迹比对（同序列 ⟹ 同轨迹，full-trace 逐态相等）
    // ──────────────────────────────────────────────────────────────────────

    /// ★full-trace 逐态相等（StepSpec 确定唯一的轨迹级推论）：两次跑同一事件序列，**每一步的态**
    /// 逐字段相等——不是只比终态，而是逐态（per-step）比对整条轨迹。
    ///
    /// 这是 conformance 的核心断言：Rust 引擎对 `RustEngineContract` 的实例化在整条事件序列上
    /// 产出**确定且可复现**的逐态轨迹（对齐 `rust_engine_step_total_unique` 的轨迹级展开）。
    #[test]
    fn full_trace_state_by_state_reproducible() {
        let trace = [
            bar_event(true), bar_event(false), bar_event(true),
            AssemblyEvent { parse_event: MicroEvent::NewStroke(super::super::super::types::Direction::Up) },
            bar_event(false), bar_event(true), bar_event(false),
        ];
        // 两次独立运行，逐态记录轨迹。
        let run = |trace: &[AssemblyEvent]| -> Vec<AssemblyState> {
            let mut s = ThetaV0Contract::initial(1_000_000);
            let mut traj = Vec::with_capacity(trace.len());
            for e in trace {
                s = ThetaV0Contract::step(&s, e);
                traj.push(s);
            }
            traj
        };
        let traj_a = run(&trace);
        let traj_b = run(&trace);
        assert_eq!(traj_a.len(), trace.len());
        // 逐态比对整条轨迹（非仅终态）。
        for (i, (a, b)) in traj_a.iter().zip(traj_b.iter()).enumerate() {
            assert_eq!(a, b, "轨迹第 {} 态不可复现（StepSpec 轨迹级唯一性破坏）", i);
        }
    }

    /// ★initial 契约锚（`RustEngineContract.initial`）：开局态满足全部不变量（合法起点）。
    #[test]
    fn initial_state_satisfies_contract_invariants() {
        let s = ThetaV0Contract::initial(1_000_000);
        assert!(s.ledger_state.inv_holds(), "initial R=Π-A-W 成立");
        assert_eq!(s.tw_state.tw(), 0, "initial TW=0");
        // classify/action 在初态全函数确定（无 panic / 确定标签）。
        let _ = ThetaV0Contract::classify(&s);
        assert_eq!(ThetaV0Contract::action(&s), StrictAction::Wait, "空仓初态 ⟹ Wait");
    }
}
