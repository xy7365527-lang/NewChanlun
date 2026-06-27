//! # 完整状态/事件 schema（17 分量 `x_t` + 8 元组 `e_{t+1}`）——FULL 结果包 §3 + §20 零遗漏实装
//!
//! 契约锚 **`formal/Origin/CompleteStateEvent.lean`**（`NewChanlun.Origin.CompleteStateEvent`）：
//! - [`state::CompleteState`] ↔ Lean `CompleteState`（§3 完整状态 17 分量，line 133-144 boxed）。
//! - [`event::ExternalEvent`] ↔ Lean `ExternalEvent`（§20 外部事件 8 元组，line 1416-1429 boxed）。
//! - [`TransitionTheta`] ↔ Lean `TransitionTheta`（§20 转移接口 `T_Θ(x,O,e)`，line 1433-1438）。
//!
//! ## 补全的两个覆盖缺口（cov-rust-impl 报告）
//!
//! - **E1 状态~50%**：`closed_loop::state::AssemblyState` 是 6 分量摘要态（声部/订单/记忆折叠为 `u64`
//!   计数，σ_r/ω/E/Cash/ν 缺字段）。本模块 [`state::CompleteState`] 逐分量显式化 17 分量（零摘要）。
//! - **D7 事件仅价格笔**：`closed_loop::state::MicroEvent` 仅 `NewBar`/`NewStroke`（纯价格/笔在线增量）。
//!   本模块 [`event::ExternalEvent`] 显式化 8 元组（补 Fill/Reject/Fee/Funding/MarginUpdate/
//!   BorrowUpdate/CorpAction 7 类经纪/会计/公司行为外部输入）。
//!
//! ## 与摘要态/微事件的关系（诚实声明，非补丁）
//!
//! 本模块**不替换**闭环引擎的 `AssemblyState`/`MicroEvent`——后者是 `hybridStep` 闭环的工程化简态/
//! 内部增量字母表，服务在线推进。本模块的完整 schema 服务「与 FULL §3/§20 零遗漏对照」。完整态→摘要态、
//! 外部事件→微事件均为**遗忘投影**（摘要/微事件的每分量可从完整 schema 导出，反之不能）。详见各子模块头。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! 全模块 = **L0**（纯结构 schema：FULL 文字 ↦ Rust 类型，逐字段对齐 Lean `CompleteStateEvent`，零信息
//! 增量）。`cargo build`/`cargo test` 绿 = 类型自洽 + 记账恒等可承载，**非**缠论盈利/实盘有效声明。

pub mod state;
pub mod event;

pub use state::{
    CapitalPhase, CompleteState, Ledger, OpenOrder, OrderAction, OrderPhase, RecStruct,
    RootDirection, SignalMemory, VenueState, VoiceForest, VoiceState,
};
pub use event::{
    BorrowUpdate, CorpAction, ExternalEvent, Fee, Fill, Funding, MarginUpdate, Reject,
};

/// 转移接口 `T_Θ(x_t, O_{t+1}, e_{t+1}) = x_{t+1}`（契约锚 `Origin.CompleteStateEvent.TransitionTheta`，
/// FULL §20 line 1433-1438）。
///
/// ★这是**接口签名**（函数类型 trait），不是某个具体 Θ 的转移实现。`orders` = 策略发的未完成订单
/// （§20 line 1447「策略唯一决定的是订单」），`event` = 市场返回的外部事件（含 `Fill`），二者一起把
/// `state` 映到唯一的下一状态。
///
/// §20 line 1442-1445 的 `∀ x,O,e, ∃! x'=T_Θ(x,O,e)`（存在唯一）对任意函数实现平凡成立（函数求值
/// 确定性）——对齐 Lean `transition_exists_unique`。具体 Θ 的转移实例化（含会计/声部/风险逻辑）由
/// 闭环工位承载，本接口只给签名。
pub trait TransitionTheta {
    fn step(
        &self,
        state: &CompleteState,
        orders: &[OpenOrder],
        event: &ExternalEvent,
    ) -> CompleteState;
}

#[cfg(test)]
mod tests {
    use super::state::*;
    use super::event::*;
    use super::super::types::{Bar, MoveKind};

    /// 初始完整态：记账恒等 `R=Π-A-W` 成立 + 开局 flat/PhaseI/零仓零单零记忆 + 17 分量全在场。
    #[test]
    fn complete_state_initial_invariants() {
        let x = CompleteState::initial(1_000_000);
        // 记账恒等（FULL §3 line 166-170，委托 LedgerComp::inv_holds）。
        assert!(x.ledger_identity_holds(), "初始完整态满足 R=Π-A-W");
        // 17 分量全在场的代表性检查。
        assert_eq!(x.root_dir, RootDirection::Flat);
        assert_eq!(x.phase, CapitalPhase::PhaseI);
        assert_eq!(x.initial_capital, 1_000_000);
        assert_eq!(x.equity, 1_000_000);
        assert_eq!(x.cash, 1_000_000);
        assert!(x.history.is_empty());
        assert!(x.open_orders.is_empty());
        assert_eq!(x.memory.last_bar_seen, -1);
        assert!(x.venue.running && x.venue.venue_open && x.venue.borrowable);
        // 开局只有根声部（未激活空仓）。
        assert_eq!(x.voices.voices.len(), 1);
        assert_eq!(x.voices.root_index, 0);
        assert_eq!(x.voices.parent, vec![None]);
    }

    /// 根方向数值投影 -1/0/+1（对齐 Lean `RootDirection.toInt`）。
    #[test]
    fn root_direction_to_int() {
        assert_eq!(RootDirection::Short.to_int(), -1);
        assert_eq!(RootDirection::Flat.to_int(), 0);
        assert_eq!(RootDirection::Long.to_int(), 1);
    }

    /// 子声部方向递归 `σ_v=σ_r·(-1)^{d(v)}`（FULL §8 line 466）：根 long → 子 short → 孙 long。
    #[test]
    fn voice_direction_recursion() {
        let mut x = CompleteState::initial(1_000_000);
        x.root_dir = RootDirection::Long;
        assert_eq!(x.voice_direction(0), 1, "根 d=0: σ_r=+1");
        assert_eq!(x.voice_direction(1), -1, "子 d=1: 反向");
        assert_eq!(x.voice_direction(2), 1, "孙 d=2: 同根");
    }

    /// 声部树一致性 · 祖先闭合（FULL §9 line 517：子激活蕴含父激活）。
    #[test]
    fn voice_forest_ancestor_closed() {
        // 根激活、子激活 → 闭合成立。
        let ok = VoiceForest {
            voices: vec![
                VoiceState { q: 5, active: true, phase: OrderPhase::Held },
                VoiceState { q: 5, active: true, phase: OrderPhase::Held },
            ],
            parent: vec![None, Some(0)],
            root_index: 0,
        };
        assert!(ok.ancestor_closed());
        // 根未激活、子激活 → 违反闭合。
        let bad = VoiceForest {
            voices: vec![
                VoiceState { q: 0, active: false, phase: OrderPhase::Flat },
                VoiceState { q: 5, active: true, phase: OrderPhase::Held },
            ],
            parent: vec![None, Some(0)],
            root_index: 0,
        };
        assert!(!bad.ancestor_closed());
    }

    /// 声部树一致性 · 精确同单位双开（FULL §9 line 525：a_v=1 ⟹ q_v=q_{p(v)}）。
    #[test]
    fn voice_forest_exact_unit_match() {
        // 激活子声部同单位 → 成立。
        let ok = VoiceForest {
            voices: vec![
                VoiceState { q: 5, active: true, phase: OrderPhase::Held },
                VoiceState { q: 5, active: true, phase: OrderPhase::Held },
            ],
            parent: vec![None, Some(0)],
            root_index: 0,
        };
        assert!(ok.exact_unit_match());
        // 激活子声部异单位 → 违反。
        let bad = VoiceForest {
            voices: vec![
                VoiceState { q: 5, active: true, phase: OrderPhase::Held },
                VoiceState { q: 3, active: true, phase: OrderPhase::Held },
            ],
            parent: vec![None, Some(0)],
            root_index: 0,
        };
        assert!(!bad.exact_unit_match());
    }

    /// 8 元组外部事件构造 + 纯行情退化（cov-rust-impl D7 旧「仅价格笔」= 完整 8 元组的切片）。
    #[test]
    fn external_event_8_tuple_and_price_only() {
        let bar = Bar {
            source_index: 0,
            timestamp: 100,
            open: 10,
            high: 12,
            low: 9,
            close: 11,
            volume: 1000,
            untradable: false,
        };
        // 纯行情退化：fill=None + 零费用 + 无公司行为。
        let e = ExternalEvent::price_only(bar);
        assert!(e.fill.is_none());
        assert_eq!(e.fee.amount, 0);
        assert_eq!(e.funding.amount, 0);
        assert!(!e.reject.rejected);
        assert_eq!(e.corp_action, CorpAction::none_action());
        // 完整 8 元组：带成交/费用/保证金/借券/公司行为。
        let full = ExternalEvent {
            bar,
            fill: Some(Fill { price: 11, qty: 5 }),
            reject: Reject { rejected: false, order_ref: 7 },
            fee: Fee { amount: 2 },
            funding: Funding { amount: -1 },
            margin_update: MarginUpdate { new_margin_used: 50 },
            borrow_update: BorrowUpdate { borrowable: true, borrow_cost: 3 },
            corp_action: CorpAction { split_num: 2, split_den: 1, dividend: 0 },
        };
        assert_eq!(full.fill.unwrap().qty, 5);
        assert_eq!(full.margin_update.new_margin_used, 50);
        assert_eq!(full.corp_action.split_num, 2);
    }

    /// `D_t` 递归结构承载走势序列（MoveKind + 起止索引，零臆造 Move 类型）。
    #[test]
    fn rec_struct_carries_moves() {
        let mut x = CompleteState::initial(1_000_000);
        x.rec_struct.moves.push((MoveKind::Trend, 0, 10));
        x.rec_struct.moves.push((MoveKind::Consolidation, 10, 20));
        assert_eq!(x.rec_struct.moves.len(), 2);
        assert_eq!(x.rec_struct.moves[0].0, MoveKind::Trend);
    }
}
