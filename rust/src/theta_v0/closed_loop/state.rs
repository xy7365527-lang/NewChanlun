//! 闭环完整态 `AssemblyState`——契约锚 `Origin.FullDefinitionStrategy.StrictState`（乘积态扩展）。
//!
//! ## 契约重锚（legacy Strict.HybridAssembly → Origin canonical）
//!
//! 闭环态对齐 Origin `StrictState`（FullDefinitionStrategy.lean:205-212：`parsed × trend ×
//! actionClass × riskMode × phase × ledger`）——本 Rust `AssemblyState` 是其乘积态扩展（额外承载
//! micro/tw/positions/orders/memory 分量，使 Rust 闭环引擎可线程化）。Origin `hybridStep` 在
//! `StrictState` 上单步推进；本 `AssemblyState` 是 Rust 闭环引擎的 `StrictState` 实例承载。
//! 其中 R=Π-A-W `ledger_state` 锚 Origin `LedgerState`；TW/OQ-9 `tw_state` 锚 `Origin.TotalWealth`
//! （#127 native port 已落地，两端均 Origin canonical）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：乘积态 + 枚举与 Origin/legacy 定义对齐 = 验证管线正确性，
//!   零信息增量）。`cargo test` 通过 = 闭环态的类型自洽 + 双账本不变量可承载，**不**是缠论
//!   盈利/实盘有效声明。重锚到 Origin **不**提升等级（仍 L0/L1）。
//!
//! ## 乘积态（蓝图 §2，复用已有类型 + 新建分量）
//!
//! `AssemblyState` = microState（引擎在线推进态）× ledgerState（LedgerComp R=Π-A-W）×
//! twState（TwState 取本金三阶段）× riskMode × phase × positions × orders × memory。
//!
//! ## microState 的契约对齐（关键诚实声明，非补丁）
//!
//! Origin canonical 的解析是 `ChanlunElements.ElementPipeline.parse : List Bar → ParseStruct`
//! （ChanlunElements.lean:114-130），**批量**吃整个 bar 序列、由 `parse_total_unique`（:132-134）
//! 证全函数唯一。Rust 引擎的解析同样是**批量重算**（`parser::parse_layer` 吃整个 bar 序列，
//! 契约锚 Origin `ElementPipeline.parse`）。本文件的 [`MicroState`] 是闭环引擎的**在线推进态**：
//! 它承载**截至第 t 根 bar 的引擎可见窗口长度** `bars_seen`——每 bar 推进 `bars_seen += 1`，闭环
//! 转移在 `bars[..bars_seen]` 前缀上重跑 Origin `parse`/classify。
//!
//! ★诚实标注（no-workaround）：Origin canonical 的解析是**批量** `ElementPipeline.parse`，**无**逐
//! bar 增量 `delta` 对应物（增量推进=全量重算前缀的左折叠等价定理在 legacy `Dynamics.classify_step`
//! 中有，Origin canonical 尚未单列该增量定理）。故 batch-on-prefix 的「增量=全量重算前缀」等价**当前
//! 锚 Origin `parse` 的批量唯一性**（前缀上确定 ⟹ 同前缀同结果），逐 bar 增量左折叠定理的 Origin
//! canonical 形式**诚实延后**——本文件不冒充已有 Origin 增量定理。
//!
//! 此外 [`MicroState`] 的四个在线摘要字段（bar_count/stroke_count/last_stroke_dir/pending_rise）由
//! [`micro_delta`] 按 [`MicroEvent`]（newBar/newStroke）推进——这是闭环引擎在线态的结构承载，与
//! `bars_seen` 一起构成完整在线态（喂给 Origin `parse` 的前缀窗口推进器）。

use super::super::types::Direction;

/// 解析事件 `MicroEvent`（闭环在线增量字母表；驱动 Origin `ElementPipeline.parse` 的前缀窗口推进）。
///
/// ★诚实标注：这是 Rust 闭环引擎的在线增量事件类型，无直接 Origin canonical 对应（Origin 解析是
/// 批量 `parse`，不展开增量事件字母表）——它是把批量 `parse` 接入逐 bar 闭环的推进器输入。
///
/// 缠论在线推进只有两类增量事件：
/// - `NewBar(rising)`：新 K 线到达（rising = 该 bar 收涨，T₅ 离散时刻推进最小单元）。
/// - `NewStroke(dir)`：新笔确认（dir = 笔方向 up/down，笔是走势最小构成元）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroEvent {
    NewBar(bool),
    NewStroke(Direction),
}

/// 解析级微状态 `MicroState`（闭环引擎 T-causal 在线分类摘要；推进 Origin `parse` 的可见前缀窗口）。
///
/// 字段（四个在线摘要 + `bars_seen` 可见窗口长度）：
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
    /// 初始微状态 s₀（空历史：无 bar、无笔、无尾部、零可见窗口；喂 Origin `parse []` 起点）。
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

/// 状态转移 `micro_delta`（闭环在线推进器，全函数且确定）。
///
/// 对 [`MicroEvent`] 两个构造子都有分支（穷尽），对任意 [`MicroState`] 返回确定的新状态
/// （无未定义洞；确定性对齐 Origin `parse_total_unique` 的「同前缀同结果」唯一性，逐 bar 推进
/// 可见窗口后在前缀上重跑 Origin `parse`）：
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

/// 风险模式 `RiskMode`（契约锚 `Origin.FullDefinitionStrategy.RiskMode`，五态）。
///
/// 对齐 Origin `RiskMode`（FullDefinitionStrategy.lean:108-114：insolvent/liquidation/deleverage/
/// closeOnly/normal，由 `chooseRiskMode` + `risk_mode_complete_unique` 证确定唯一）。
///
/// ★诚实标注：模式枚举是结构层（μ 的**取值阈值**是 Θ_risk 参数，非缠论可导；本枚举只承载五态，
/// 对齐 Origin 五构造子）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskMode {
    Insolvent,
    Liquidation,
    Deleverage,
    CloseOnly,
    Normal,
}

/// 三阶段 `Phase`（部分对齐 `Origin.FullDefinitionStrategy.CapitalPhase`，Φ 三阶段 T₂₆）。
///
/// I=建仓 / II=取本 / III=增核——三阶段持仓相位结构，其转移阈值是 Θ 参数（结构层承载，不臆造阈值）。
///
/// ★诚实标注（no-workaround，部分对齐）：Origin canonical 的 `CapitalPhase`
/// （FullDefinitionStrategy.lean:145-151）是**五态**（phaseI/phaseII/repair/protectedPhase/
/// accretive，由 `chooseCapitalPhase` + `capital_phase_complete_unique` 证唯一），本 Rust `Phase`
/// 是**三态**摘要（PhaseI/PhaseII/PhaseIII）。三态 Phase **不**与 Origin 五态 CapitalPhase 双射
/// （Origin 把 phaseIII 细分为 repair/protected/accretive 三个 returned 后子相位）——本枚举只承载
/// 三阶段摘要，**不**声明等价于 Origin CapitalPhase。三态→五态的精化对齐契约**诚实延后**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    PhaseI,
    PhaseII,
    PhaseIII,
}

use super::super::strategy::ledger::{LedgerComp, TwState};

/// 完整混合态 `AssemblyState`（契约锚 `Origin.FullDefinitionStrategy.StrictState`，乘积态扩展）。
///
/// Origin `StrictState`（FullDefinitionStrategy.lean:205-212）= `parsed × trend × actionClass ×
/// riskMode × phase × ledger`；本 Rust 乘积态是其闭环引擎扩展（额外 micro/tw/positions/orders/memory）。
///
/// 字段（各标来源，复用已证类型 / 新建分量）：
/// - `micro_state`：解析级微状态（[`MicroState`]，T-causal 在线推进态，推进 Origin `parse` 前缀窗口）。
/// - `ledger_state`：账本恒等分量（[`LedgerComp`]，R=Π-A-W，契约锚 `Origin.LedgerState`）。
/// - `tw_state`：取本金三阶段账本分量（[`TwState`]，TW 守恒 + stage 单向 + OQ-9 gate，契约锚
///   `Origin.TotalWealth.TWState`）。与 ledger_state **双层并置**——R=Π-A-W（收益表视角）与 TW
///   （现金流+持仓视角）二者不同构（#90 已证，`Origin.TotalWealth` §3 三反例 native 重证），
///   完整持仓系统两者都需要。tw_state 真被 transition 线程化（见 transition.rs）。
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
    /// 初始闭环态（Origin `StrictState` 实例的开局态）：空微态 + 初始双账本 + Normal/PhaseI + 零仓/零单/零记忆。
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

    /// ★**已注资 campaign 开局态**（GAP3：三阶段推进需 TW 账本被注资，否则 `tw()==0`/`holding==0`
    /// ⟹ 三阶段机恒 inert）。契约锚 `Origin.TotalWealth`（campaign 开局注资：本金 `notional` 入 free
    /// 在险池，记 `notional_in`=退本金目标基线）。
    ///
    /// 与 [`initial`](Self::initial) 的区别：`initial` 是零 TW 空 campaign（`tw()==0`，三阶段机 inert）；
    /// 本构造子开一个 `notional` 单位的 campaign（`free=notional, notional_in=notional`）——使
    /// ledger/仓位在真实建仓（现金充足）时真线程化（非 inert 空转）。
    ///
    /// ★诚实（codex 复审 + GAP3 裁定 A' 后有效域收窄）：注资**不**使 L0 同价闭环达 EarningShares——
    /// L0 同价下平仓 PnL≡0（`TwEvent::Realize` 分量恒 0）⟹ TW 守恒（=notional）⟹ 退本金前提
    /// `holding≥notional` 与 cash-tight `free>0` 互斥（见 runner `earning_shares_unreachable_l0_
    /// same_price_zero_pnl`，L0 同价无盈亏定理）；L2 变价下已实现利润经 Realize 真入 TW ⟹ 可达
    /// （生产见证 `pi_loop_realized_profit_reaches_earning_shares`）。
    ///
    /// `i0`=本金基线（进 ledger.i0，EnterReady 的 W≥I0 门）；`notional`=本 campaign 名义敞口 Q（退本金
    /// 目标；建仓 `holding` 累积到 ≥notional 触发退本金）。TW 守恒：`tw()=notional`（全在 free）。
    pub fn funded_campaign(i0: i64, notional: i64) -> AssemblyState {
        AssemblyState {
            tw_state: TwState { free: notional, notional_in: notional, ..TwState::initial() },
            ..AssemblyState::initial(i0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// micro_delta 全定义且确定（确定性对齐 Origin `parse_total_unique` 的同前缀同结果）：同输入同输出。
    #[test]
    fn micro_delta_deterministic() {
        let s = MicroState::initial();
        let e = MicroEvent::NewBar(true);
        assert_eq!(micro_delta(&s, e), micro_delta(&s, e));
    }

    /// NewBar 推进 bar_count + bars_seen + pending_rise（闭环在线推进器 newBar 分支）。
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

    /// NewStroke 推进 stroke_count + 记录方向 + 清零 pending_rise（闭环在线推进器 newStroke 分支）。
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

    /// bar_count 单调不减（闭环在线推进器：可见前缀窗口只增不减）。
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
