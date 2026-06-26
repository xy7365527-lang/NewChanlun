//! 闭环完整态 `AssemblyState`——镜像 Lean `Strict/HybridAssembly.lean:227-236`。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：乘积态 + 枚举与 Lean 定义对齐 = 验证管线正确性，零信息增量）。
//!   `cargo test` 通过 = 闭环态的类型自洽 + 双账本不变量可承载，**不**是缠论盈利/实盘有效声明。
//!
//! ## 乘积态（蓝图 §2，复用已有类型 + 新建分量）
//!
//! `AssemblyState` = microState（引擎在线推进态）× ledgerState（LedgerComp R=Π-A-W）×
//! twState（TwState 取本金三阶段）× riskMode × phase × positions × orders × memory。
//!
//! ## microState 的镜像对齐（关键诚实声明，非补丁）
//!
//! Lean 的 microState 是抽象 `Dynamics.State`（barCount/strokeCount/... 在线摘要），由
//! `Dynamics.delta` 逐 bar 推进。Rust 引擎的解析是**批量重算**（`parser::parse_layer` 吃整个
//! bar 序列）。本文件的 [`MicroState`] 忠实对齐 Lean delta「逐 bar 推进」：它承载**截至第 t 根
//! bar 的引擎可见窗口长度** `bars_seen`——每 bar 推进 `bars_seen += 1`，闭环转移在 `bars[..bars_seen]`
//! 前缀上重跑 parse/classify。Lean `Dynamics.classify_step`（C(h⌢e)=δ(C h,e)）已证**增量推进 =
//! 全量重算前缀**，故 batch-on-prefix 是 delta 左折叠的忠实实装（不是补丁——它正是 foldl 语义）。
//!
//! 此外 [`MicroState`] 镜像 Lean `Dynamics.State` 的四个在线摘要字段（bar_count/stroke_count/
//! last_stroke_dir/pending_rise），由 [`micro_delta`] 按 [`MicroEvent`]（newBar/newStroke）推进——
//! 这是 Lean delta 的 bit-exact Rust 镜像，与 `bars_seen` 一起构成完整在线态。

use super::super::types::Direction;

/// 解析事件 `MicroEvent`（镜像 Lean `Dynamics.Event`，穷尽在线增量字母表）。
///
/// 缠论在线推进只有两类增量事件：
/// - `NewBar(rising)`：新 K 线到达（rising = 该 bar 收涨，T₅ 离散时刻推进最小单元）。
/// - `NewStroke(dir)`：新笔确认（dir = 笔方向 up/down，笔是走势最小构成元）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroEvent {
    NewBar(bool),
    NewStroke(Direction),
}

/// 解析级微状态 `MicroState`（镜像 Lean `Dynamics.State`，T-causal 在线分类摘要）。
///
/// 字段（镜像 Lean `Dynamics.State` 四字段 + `bars_seen` 可见窗口长度）：
/// - `bar_count`：已处理 K 线数（T₅ 离散时刻 = 已消费事件计数，单调不减）。
/// - `stroke_count`：已确认笔数（笔级结构积累）。
/// - `last_stroke_dir`：最近一笔方向（`None` = 尚无笔；未完成尾部状态承载）。
/// - `pending_rise`：自上一笔以来的净涨 bar 计数（未完成走势尾部，下一笔确认时清零）。
/// - `bars_seen`：引擎可见的 bar 窗口长度（闭环在 `bars[..bars_seen]` 前缀上重跑 parse/classify，
///   见模块头 microState 对齐说明）。每 bar 推进 +1，使引擎的结构识别进入逐 bar 闭环。
///
/// 所有字段从**已出现**的事件读出（因果性的状态侧根源：状态不含任何未来信息）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MicroState {
    pub bar_count: u64,
    pub stroke_count: u64,
    pub last_stroke_dir: Option<Direction>,
    pub pending_rise: u64,
    pub bars_seen: usize,
}

impl MicroState {
    /// 初始微状态 s₀（镜像 Lean `Dynamics.s0`，空历史：无 bar、无笔、无尾部、零可见窗口）。
    pub fn initial() -> MicroState {
        MicroState {
            bar_count: 0,
            stroke_count: 0,
            last_stroke_dir: None,
            pending_rise: 0,
            bars_seen: 0,
        }
    }
}

/// 状态转移 `micro_delta`（镜像 Lean `Dynamics.delta`，全函数且确定）。
///
/// 对 [`MicroEvent`] 两个构造子都有分支（穷尽），对任意 [`MicroState`] 返回确定的新状态
/// （无未定义洞，对齐 Lean `delta_total_deterministic`）：
/// - `NewBar(rising)`：bar 计数 +1；可见窗口 +1；涨 bar 累积 pending_rise（未完成尾部）。
/// - `NewStroke(dir)`：笔计数 +1；记录笔方向；清零 pending_rise（尾部被新笔吸收 = 结构完成一段）。
///
/// ★`bars_seen` 仅随 `NewBar` 推进（笔确认不增加可见 bar 窗口——笔是已见 bar 上的结构事件）。
/// 不可变模式：返回新 [`MicroState`]，不就地修改。
pub fn micro_delta(s: &MicroState, e: MicroEvent) -> MicroState {
    match e {
        MicroEvent::NewBar(rising) => MicroState {
            bar_count: s.bar_count + 1,
            pending_rise: if rising { s.pending_rise + 1 } else { s.pending_rise },
            bars_seen: s.bars_seen + 1,
            ..*s
        },
        MicroEvent::NewStroke(dir) => MicroState {
            stroke_count: s.stroke_count + 1,
            last_stroke_dir: Some(dir),
            pending_rise: 0,
            ..*s
        },
    }
}

/// 风险模式 `RiskMode`（镜像 Lean `HybridAssembly.RiskMode`，蓝图 §2 μ 五态）。
///
/// ★诚实标注：模式枚举是结构层（μ 的**取值阈值**是 Θ_risk 参数，非缠论可导；本枚举只承载五态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskMode {
    Insolvent,
    Liquidation,
    Deleverage,
    CloseOnly,
    Normal,
}

/// 三阶段 `Phase`（镜像 Lean `HybridAssembly.Phase`，蓝图 §2 Φ 三阶段 T₂₆）。
///
/// I=建仓 / II=取本 / III=增核——与 `Operational.Phase` 的 enter/hold/exit 同范畴（三阶段
/// 结构），其转移阈值是 Θ 参数（结构层承载，不臆造阈值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    PhaseI,
    PhaseII,
    PhaseIII,
}

use super::super::strategy::ledger::{LedgerComp, TwState};

/// 完整混合态 `AssemblyState`（镜像 Lean `HybridAssembly.AssemblyState`，蓝图 §2 乘积态）。
///
/// 字段（各标来源，复用已证类型 / 新建分量）：
/// - `micro_state`：解析级微状态（[`MicroState`]，T-causal 在线推进态）。
/// - `ledger_state`：账本恒等分量（[`LedgerComp`]，R=Π-A-W）。
/// - `tw_state`：取本金三阶段账本分量（[`TwState`]，TW 守恒 + stage 单向 + OQ-9 gate）。
///   与 ledger_state **双层并置**——R=Π-A-W（收益表视角）与 TW（现金流+持仓视角）二者不同构
///   （Lean #90 已证），完整持仓系统两者都需要。tw_state 真被 transition 线程化（见 transition.rs）。
/// - `risk_mode`：风险模式 μ（五态）。
/// - `phase`：三阶段 Φ。
/// - `positions`：当前总持仓单位（声部聚合后的绝对单位数 Σq_v 的摘要）。
/// - `orders`：未完成订单计数 O_t 的摘要。
/// - `memory`：信号使用记录 M_t（Fresh 计数）的摘要。
///
/// ★诚实标注（formalization-validity-domain）：positions/orders/memory 用计数**摘要**承载
/// （结构层乘积态的占位分量，使闭环转移可表达）——其**缠论语义内容**（具体哪些声部持仓、
/// 哪些订单挂单）由 Fugue/StrategyFamily 逐 claim 承载，本结构只组装乘积态。globalState
/// （C_Θ 输出）不进字段：它是 classify 段的**输出**（由 micro_state 经引擎导出），在闭环数据流中
/// 现算，不冗余存进状态（避免双份漂移，对齐 Lean 注释）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssemblyState {
    pub micro_state: MicroState,
    pub ledger_state: LedgerComp,
    pub tw_state: TwState,
    pub risk_mode: RiskMode,
    pub phase: Phase,
    pub positions: u64,
    pub orders: u64,
    pub memory: u64,
}

impl AssemblyState {
    /// 初始闭环态（镜像 Lean 装配的开局态）：空微态 + 初始双账本 + Normal/PhaseI + 零仓/零单/零记忆。
    ///
    /// `i0`：初始本金（进 ledger_state.i0；NAV 绝对额由 runner 在 fill 侧另算，账本是结构分量）。
    pub fn initial(i0: i64) -> AssemblyState {
        AssemblyState {
            micro_state: MicroState::initial(),
            ledger_state: LedgerComp::initial(i0),
            tw_state: TwState::initial(),
            risk_mode: RiskMode::Normal,
            phase: Phase::PhaseI,
            positions: 0,
            orders: 0,
            memory: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// micro_delta 全定义且确定（镜像 Lean `delta_total_deterministic`）：同输入同输出。
    #[test]
    fn micro_delta_deterministic() {
        let s = MicroState::initial();
        let e = MicroEvent::NewBar(true);
        assert_eq!(micro_delta(&s, e), micro_delta(&s, e));
    }

    /// NewBar 推进 bar_count + bars_seen + pending_rise（镜像 Lean delta newBar 分支）。
    #[test]
    fn micro_delta_new_bar() {
        let s = MicroState::initial();
        let s1 = micro_delta(&s, MicroEvent::NewBar(true));
        assert_eq!(s1.bar_count, 1);
        assert_eq!(s1.bars_seen, 1);
        assert_eq!(s1.pending_rise, 1); // 收涨累积
        let s2 = micro_delta(&s1, MicroEvent::NewBar(false));
        assert_eq!(s2.bar_count, 2);
        assert_eq!(s2.bars_seen, 2);
        assert_eq!(s2.pending_rise, 1); // 收跌不累积
    }

    /// NewStroke 推进 stroke_count + 记录方向 + 清零 pending_rise（镜像 Lean delta newStroke 分支）。
    #[test]
    fn micro_delta_new_stroke() {
        let s = MicroState {
            pending_rise: 3,
            ..MicroState::initial()
        };
        let s1 = micro_delta(&s, MicroEvent::NewStroke(Direction::Up));
        assert_eq!(s1.stroke_count, 1);
        assert_eq!(s1.last_stroke_dir, Some(Direction::Up));
        assert_eq!(s1.pending_rise, 0); // 尾部被新笔吸收
        assert_eq!(s1.bars_seen, s.bars_seen); // 笔不增加可见 bar 窗口
    }

    /// bar_count 单调不减（镜像 Lean `delta_barCount_monotone`）。
    #[test]
    fn micro_delta_bar_count_monotone() {
        let s = MicroState::initial();
        for e in [
            MicroEvent::NewBar(true),
            MicroEvent::NewStroke(Direction::Up),
            MicroEvent::NewBar(false),
        ] {
            let s1 = micro_delta(&s, e);
            assert!(s1.bar_count >= s.bar_count);
        }
    }

    /// 初始闭环态：双账本不变量成立 + 开局 Normal/PhaseI/零仓。
    #[test]
    fn assembly_initial_invariants() {
        let x = AssemblyState::initial(1_000_000);
        assert!(x.ledger_state.inv_holds(), "初始 ledger 满足 R=Π-A-W");
        assert_eq!(x.tw_state.tw(), 0, "初始 TW=0");
        assert_eq!(x.risk_mode, RiskMode::Normal);
        assert_eq!(x.phase, Phase::PhaseI);
        assert_eq!((x.positions, x.orders, x.memory), (0, 0, 0));
        assert_eq!(x.ledger_state.i0, 1_000_000);
    }
}
