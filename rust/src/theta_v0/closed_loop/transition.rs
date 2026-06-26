//! 闭环转移 `hybrid_step`——契约锚 **`Origin.FullDefinitionStrategy`** 的六段数据流 +
//! `transition` + `hybridStep`（x_t ─e→ x_{t+1} 单一闭环；task #102 A′ Phase2 step8 重锚）。
//!
//! ## 契约重锚（legacy Strict.HybridAssembly → Origin canonical）
//!
//! 闭环转移的契约锚点指向 Origin `FullDefinitionSystem`（FullDefinitionStrategy.lean:214-247）：
//!
//! - 六段数据流 → Origin 六字段 `recStruct → classify → intent → risk → schedule → transition`
//!   （:219-224）。本文件 rec/risk/schedule/transition 各 adapter 对齐对应 Origin 字段语义。
//! - 策略复合 `policy_output` → `Origin.policyTheta`（:226-227）：
//!   `policyTheta = schedule (risk (intent (classify (recStruct e))))`。
//! - 闭环单步 `hybrid_step` → `Origin.hybridStep`（:229-230）：`hybridStep = transition x (policyTheta x e) e`，
//!   由 `hybrid_step_complete_unique`（:232-236）证确定唯一、`transition_writes_full_state`（:244-247）
//!   证 T 写回完整下一态、`policy_factors_through_classification`（:238-242）证经分类瓶颈分解。
//! - R=Π-A-W 双账本写回 → `Origin.ledgerStep` + `ledger_invariant_preservation`（见 strategy/ledger.rs）。
//! - **TW/OQ-9 写回 → `Origin.TotalWealth.twStep` + `oq9inv_preserved`**（#127 native port 已落地，
//!   tw_step/OQ-9 gate 锚 Origin canonical；见 ledger.rs 模块头「TW 三阶段 / OQ-9 gate 契约 → Origin.TotalWealth」）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：闭环转移 + 双账本写回与 Origin/legacy 定义对齐 = 验证管线
//!   正确性，零信息增量）。`cargo test` 通过 = 闭环每步保 R=Π-A-W + TW 守恒 + stage 单向 + OQ-9
//!   gate + 双账本真线程化（非恒等挂件），**不**是缠论盈利/实盘有效声明。重锚到 Origin **不**
//!   提升等级（仍 L0/L1，验管线非验缠论假设）。
//!
//! ## 闭环装配（消除开环单帧，gap-map U3 核心）
//!
//! Origin `hybridStep S x e = S.transition x (policyTheta S x e) e`：六段
//! recStruct→classify→intent→risk→schedule→transition 复合为单一转移，T 写回完整 `StrictState`
//! （`transition_writes_full_state`）。本文件把它实装为运行引擎的 [`hybrid_step`]（对齐 Origin
//! `hybridStep`）：每 bar `x = hybrid_step(x, e)`，micro_state/ledger_state/tw_state/positions/orders
//! 每 bar 真更新喂回（消除 runner.rs 旧版「account 构造一次不喂回」的开环单帧）。
//!
//! ## 两层分工（Origin 结构 vs Rust 真实引擎，诚实声明）
//!
//! - **本文件（transition.rs）**：闭环**结构**——对齐 Origin 六段数据流的结构形式（确定选择器 +
//!   T 写回 + 双账本同步线程化 + OQ-9 gate）。risk/schedule 段是 Origin `risk`/`schedule` 字段的
//!   有限网格确定选择器实例（确定唯一仓位 → OrderOut 携带双账本事件）。
//! - **runner.rs**：在每 bar 的可见窗口上调用真实引擎链（recognize→plan_orders），产真实订单流。
//!   两层是 Origin「闭环结构」与 Rust「真实引擎」的对齐分工——本文件保证闭环每步的结构不变量，
//!   runner 保证真实订单流喂入闭环。
//!
//! ## U5 bit-exact conformance 诚实延后（no-workaround）
//!
//! 本文件各测试验证**结构不变量对齐**（保 R=Π-A-W / TW 守恒 / stage 单向 / OQ-9 gate / 真线程化），
//! 与 Origin/legacy 定理结构一一对应。闭环 `hybrid_step` 与 Origin `hybridStep`（及 EngineBridge/
//! RustEngineContract，由并行 #101 重锚）的 **bit-exact 逐态对齐证**（U5）**诚实延后**——本文件
//! 不冒充已 conformant（重锚契约语义 ≠ 已过 bit-exact conformance）。

use super::super::strategy::intent::{classify_adapter, intent_adapter, ClassLabel};
use super::super::strategy::ledger::{ledger_step, tw_step, LedgerEvent, TwEvent, TwState};
use super::super::types::StrictAction;
use super::state::{micro_delta, AssemblyState, MicroEvent};

/// 混合事件 `AssemblyEvent`（契约锚 `Origin.FullDefinitionSystem.Event`）：驱动闭环一步的外部事件 e。
///
/// 对齐 Origin `hybridStep S x e` 的 `e : S.Event` 输入位。携带一个解析事件（[`MicroEvent`]，驱动 micro_state）。
///
/// ★诚实标注：Origin `FullDefinitionSystem.Event` 是抽象 `Type` 字段（由具体系统实例化）。本 Rust
/// 镜像**只保留被消费的 `parse_event` 字段**——权威账本事件由 [`schedule_adapter`] 从动作派生
/// （账户因果链：策略决定账本副作用），不在 Event 里携带被忽略的账本事件字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssemblyEvent {
    pub parse_event: MicroEvent,
}

/// 订单 `OrderOut`（契约锚 `Origin.FullDefinitionSystem.Order`，Schedule 段输出）。
///
/// 对齐 Origin `schedule : StrictState → Control → Order` 的 `Order` 输出位。携带动作 + 目标仓位 +
/// 双账本事件——使 T 的双账本（ledger R=Π-A-W 锚 Origin.LedgerState + tw_state TW 锚 Origin.TotalWealth）更新都有据
/// （账户因果链；task #93 双层并置）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderOut {
    pub action: StrictAction,
    pub target_pos: u64,
    pub ledger_event: LedgerEvent,
    pub tw_event: TwEvent,
}

/// Rec 段 `rec_adapter`（契约锚 `Origin.FullDefinitionSystem.recStruct : StrictState → Event →
/// ParseStruct`）：在当前 micro_state 上跑一步 micro_delta，产新解析态。
///
/// 对齐 Origin `recStruct` 字段（解析事件 → 新 ParseStruct）。Θ_parse 参数化解析的在线推进摘要
/// （rec_struct = 新解析态，喂给 classify 段，对齐 Origin `classify x (recStruct x e)`）。
fn rec_adapter(x: &AssemblyState, e: &AssemblyEvent) -> super::state::MicroState {
    micro_delta(&x.micro_state, e.parse_event)
}

/// RiskProj 段 `risk_adapter`（契约锚 `Origin.FullDefinitionSystem.risk : StrictState → Intent →
/// Control`）：意图经**有限网格确定选择器**投到唯一目标仓位。
///
/// 对齐 Origin `risk` 字段（Intent → Control，本闭环的 Control = 目标仓位）。有限网格确定选择器
/// （网格 {0,1,2,3} 上 cost 字典序最小格点，总存在且唯一）：
/// - 开仓侧意图（Buy/Add）⟹ 投到正仓位（网格非零格点）。
/// - 平仓/减仓侧意图（Close/Reduce/Sell）⟹ 投到 0（清仓格点）。
/// - 保持/观望（Hold/Wait）⟹ 保持当前仓位（不调仓）。
///
/// ★诚实标注（formalization-validity-domain，codex R3）：网格选择是 Θ_risk 参数（分辨率/代价权重
/// 非缠论可导）——本段是 Origin `risk` 字段的有限网格确定选择器实例（List 上选最小，总存在且唯一），
/// **不冒充连续 argmin**。真实 sizing（risk::size_position）由 runner 在真实订单流路径调用，本结构
/// 段是闭环的确定仓位投影。
fn risk_adapter(x: &AssemblyState, intent: StrictAction) -> u64 {
    // 有限网格确定选择器（镜像 gridProject 的确定语义）。
    match intent {
        // 开仓侧：投到正仓位（网格最小正格点 1 = 建一个单位，结构层确定选择器）。
        StrictAction::Buy | StrictAction::Add => x.positions.max(1),
        // 平仓侧：投到 0（清仓格点）。
        StrictAction::Close | StrictAction::Sell => 0,
        // 减仓：投到当前仓位减一格（不低于 0）。
        StrictAction::Reduce => x.positions.saturating_sub(1),
        // 保持/观望：保持当前仓位（不调仓）。
        StrictAction::Hold | StrictAction::Wait => x.positions,
    }
}

/// Schedule 段 `schedule_adapter`（契约锚 `Origin.FullDefinitionSystem.schedule : StrictState →
/// Control → Order`）：把控制（目标仓位）+ 意图（动作）打包为订单，派生双账本事件。
///
/// 对齐 Origin `schedule` 字段（Control → Order）。动作→账本副作用映射（账户因果链）：
/// - 开仓侧（Buy/Add，目标仓位 > 当前）⟹ `Allocate`（资本化建仓占用 R）+ `ShortDiff(-1)`
///   （free→holding 买入降成本短差，TW 守恒，改变 tw_state free/holding 分量——非恒等）。
/// - 平仓/减仓侧（Close/Reduce/Sell，目标仓位 ≤ 当前）⟹ `Realize`（回收实现盈亏）+
///   `RecoverCapital(1)`（free→withdrawn 退本金 1 单位，TW 守恒，改变 tw_state free/withdrawn +
///   推进 stage——非恒等）。
/// - 保持/观望（Hold/Wait）⟹ `Noop`（无账本变化）+ `ShortDiff(0)`（TW 不变的零转移）。
///
/// ★OQ-9 gate（契约锚 `Origin.TotalWealth.LegalTransition`，#127 native port）：`OpenShareLeg` 在
/// stage=EarningShares **非法**（引擎层禁该转移）。本段派生的 tw_event 只取 `ShortDiff`/`RecoverCapital`
/// （都不是 OpenShareLeg/CloseShareLeg），故不开 legacy 腿也不闭 legacy 腿——OQ-9 gate 自动满足
/// （开 legacy 腿的非法转移不被本段产生，见 `assert_oq9_legal`）。
///
/// ★诚实：账本事件的 dΠ/dA/dCash/w 取占位常量（1 单位）——具体数额是 Θ_risk/运行时数据（L2），
/// 本结构层只承载「动作 ⟹ 账本事件类型」的映射，数额由下游填充。tw_event 与 ledger_event 双侧由
/// 同一动作派生 ⟹ 两账本同步线程化。
fn schedule_adapter(x: &AssemblyState, intent: StrictAction, target_pos: u64) -> OrderOut {
    match intent {
        StrictAction::Buy | StrictAction::Add => OrderOut {
            action: intent,
            target_pos,
            ledger_event: LedgerEvent::Allocate(1),
            tw_event: TwEvent::ShortDiff(-1),
        },
        StrictAction::Close | StrictAction::Sell | StrictAction::Reduce => OrderOut {
            action: intent,
            target_pos,
            ledger_event: LedgerEvent::Realize(1),
            tw_event: TwEvent::RecoverCapital(1),
        },
        StrictAction::Hold | StrictAction::Wait => OrderOut {
            action: intent,
            target_pos: x.positions, // 不调仓
            ledger_event: LedgerEvent::Noop,
            tw_event: TwEvent::ShortDiff(0),
        },
    }
}

/// 策略订单输出 `policy_output`（契约锚 `Origin.FullDefinitionStrategy.policyTheta`）：六段前五段复合产订单。
///
/// `schedule_adapter(x, risk_adapter(x, intent_adapter(x, classify_adapter(x, rec_adapter(x, e)))), ...)`
/// ——Rec 段产新解析态喂给 classify 段，意图穿过分类瓶颈（intent_adapter 读 classify_adapter 输出，
/// factor-through-classify），再经 risk 投影 + schedule 打包。这与 Origin `policyTheta = schedule (risk
/// (intent (classify (recStruct e))))` 五段复合同构，由 Origin `policy_factors_through_classification` 钉死。
pub fn policy_output(x: &AssemblyState, e: &AssemblyEvent) -> OrderOut {
    // Rec 段：在当前 micro_state 上吃事件跑一步，产新解析态 rec_struct。
    let rec_struct = rec_adapter(x, e);
    // Classify 段：从解析结构 + 当前态读出分类标签（Rec 段输出真被 classify 消费）。
    let label: ClassLabel = classify_adapter(x, &rec_struct);
    // Intent 段：意图穿过分类瓶颈（读 classify 输出）。
    let intent: StrictAction = intent_adapter(x, label);
    // Risk 段：意图经有限网格确定选择器投到唯一目标仓位。
    let target_pos: u64 = risk_adapter(x, intent);
    // Schedule 段：打包订单 + 派生双账本事件。
    schedule_adapter(x, intent, target_pos)
}

/// OQ-9 gate 守卫（契约锚 `Origin.TotalWealth.LegalTransition`，#127 native port）：tw_event 在当前
/// tw_state 下必须合法。
///
/// OpenShareLeg 在 stage=EarningShares 非法。本守卫断言订单携带的 tw_event 是当前态下的合法转移
/// （schedule_adapter 只派生 ShortDiff/RecoverCapital，二者恒合法，故此守卫恒成立；它是引擎层
/// 「禁非法转移」的显式落实，对齐 `Origin.TotalWealth.oq9inv_preserved` 的单步保持——TW/OQ-9 的
/// Origin canonical 重锚已由 #127 `Origin.TotalWealth` native port 落地）。
fn assert_oq9_legal(tw_state: &TwState, tw_event: TwEvent) -> bool {
    tw_event.is_legal_from(tw_state)
}

/// ★★T 段 `transition_adapter`（契约锚 `Origin.FullDefinitionSystem.transition : StrictState →
/// Order → Event → StrictState`，闭环写回完整态）。
///
/// 对齐 Origin `transition` 字段（Order → Event → 下一 StrictState），由 Origin
/// `transition_writes_full_state`（hybridStep = transition x (policyTheta x e) e）证 T 写回完整下一态。
/// 闭环写回各分量（δ 只更新 micro_state，**ledger/accounting 更新放进 T**）：
/// - `micro_state` := `micro_delta(x.micro_state, e.parse_event)`（解析层在线推进一步，T-causal）。
/// - `ledger_state` := `ledger_step(x.ledger_state, o.ledger_event)`（**账本更新，保 R=Π-A-W**，契约锚
///   `Origin.ledgerStep` + `ledger_invariant_preservation`——用订单携带的账本事件，使账户态生成进入
///   闭环，补账户因果洞）。
/// - `tw_state` := `tw_step(x.tw_state, o.tw_event)`（**取本金三阶段账本更新，保 TW 守恒 + stage 单向**
///   ——task #93 真线程化：用订单携带的 tw_event 驱动 TW 账本一步；与 ledger_state 双层并置）。
/// - `positions` := `o.target_pos`（写回新持仓）。
/// - `orders` := `x.orders + 1`（订单计数推进）。
/// - `memory`、`risk_mode`、`phase`：本骨架保持（转移阈值是 Θ_signal/Θ_risk，L2；结构层装配保持，
///   不臆造阈值——诚实留白，非 workaround：阈值驱动转移属下游有效域，本文件闭合
///   micro/ledger/tw_state/positions/orders 的结构闭环）。
///
/// ★诚实标注（codex R3）：T **只声明闭环状态转移全定义**（产出确定的下一态），**不**声明该转移
/// 盈利/最优/实盘有效（L3）。
pub fn transition_adapter(x: &AssemblyState, o: &OrderOut, e: &AssemblyEvent) -> AssemblyState {
    // OQ-9 gate：订单 tw_event 必须合法（schedule_adapter 保证只派生合法转移）。
    debug_assert!(
        assert_oq9_legal(&x.tw_state, o.tw_event),
        "OQ-9 gate 违反：tw_event {:?} 在 stage {:?} 非法",
        o.tw_event,
        x.tw_state.stage
    );
    AssemblyState {
        micro_state: micro_delta(&x.micro_state, e.parse_event),
        ledger_state: ledger_step(&x.ledger_state, o.ledger_event),
        tw_state: tw_step(&x.tw_state, o.tw_event),
        risk_mode: x.risk_mode,
        phase: x.phase,
        positions: o.target_pos,
        orders: x.orders + 1,
        memory: x.memory,
    }
}

/// ★★闭环一步 `hybrid_step`（契约锚 `Origin.FullDefinitionStrategy.hybridStep`：
/// `hybridStep S x e = S.transition x (policyTheta S x e) e`）。
///
/// x_t ─e→ x_{t+1} 单一闭环：六段（rec→classify→intent→risk→schedule→transition）复合为单一转移，
/// 由 Origin `hybrid_step_complete_unique` 证确定唯一。rec 段在 transition_adapter 的 micro_delta 中
/// 体现；前五段经 [`policy_output`]（= Origin `policyTheta`）产订单；transition 写回。
///
/// 这是引擎的闭环核心——runner 每 bar 调 `x = hybrid_step(x, e)`，micro_state/ledger_state/tw_state/
/// positions/orders 每 bar 真更新喂回（消除开环单帧，对齐 Origin `transition_writes_full_state`）。
pub fn hybrid_step(x: &AssemblyState, e: &AssemblyEvent) -> AssemblyState {
    let order = policy_output(x, e);
    transition_adapter(x, &order, e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state::{MicroState, Phase, RiskMode};
    use super::super::super::strategy::ledger::TStage;
    use super::super::super::types::Direction;

    fn bar_event(rising: bool) -> AssemblyEvent {
        AssemblyEvent { parse_event: MicroEvent::NewBar(rising) }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  闭环结构：六段复合 + T 写回（契约锚 Origin.hybridStep / transition_writes_full_state）
    // ──────────────────────────────────────────────────────────────────────

    /// 闭环展开（契约锚 `Origin.transition_writes_full_state`）：hybrid_step = transition_adapter(x, policy_output(x), e)。
    #[test]
    fn hybrid_step_unfold() {
        let x = AssemblyState::initial(1_000_000);
        let e = bar_event(true);
        let order = policy_output(&x, &e);
        assert_eq!(hybrid_step(&x, &e), transition_adapter(&x, &order, &e));
    }

    /// ★factor-through-classify（契约锚 `Origin.FullDefinitionStrategy.policy_factors_through_classification`）：
    /// policy_output 真穿过分类——意图由 classify_adapter 输出派生。
    #[test]
    fn policy_factors_through_classify() {
        let x = AssemblyState::initial(1_000_000); // Normal/PhaseI
        let e = bar_event(true);
        let rec_struct = micro_delta(&x.micro_state, e.parse_event);
        let label = classify_adapter(&x, &rec_struct);
        let intent = intent_adapter(&x, label);
        let target = risk_adapter(&x, intent);
        assert_eq!(policy_output(&x, &e), schedule_adapter(&x, intent, target));
        // 初始 Normal/PhaseI ⟹ intent=Buy ⟹ 订单是建仓侧。
        assert_eq!(policy_output(&x, &e).action, StrictAction::Buy);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  闭环每步保不变量（R=Π-A-W 锚 Origin.ledger_invariant_preservation；TW/stage 锚 Origin.TotalWealth）
    // ──────────────────────────────────────────────────────────────────────

    /// ★闭环每步保 R=Π-A-W（契约锚 `Origin.FullDefinitionStrategy.ledger_invariant_preservation`）。
    #[test]
    fn hybrid_step_preserves_ledger_inv() {
        let mut x = AssemblyState::initial(1_000_000);
        assert!(x.ledger_state.inv_holds());
        // 多 bar 闭环：每步后 ledger 恒等必须成立。
        for i in 0..50 {
            x = hybrid_step(&x, &bar_event(i % 2 == 0));
            assert!(
                x.ledger_state.inv_holds(),
                "bar {} 后 ledger 破坏 R=Π-A-W: {:?}",
                i, x.ledger_state
            );
        }
    }

    /// ★闭环每步保 TW 守恒（契约锚 `Origin.TotalWealth.twStep_preserves_tw`——#127 native port）。
    #[test]
    fn hybrid_step_preserves_tw() {
        let mut x = AssemblyState::initial(1_000_000);
        let tw0 = x.tw_state.tw();
        for i in 0..50 {
            x = hybrid_step(&x, &bar_event(i % 3 == 0));
            assert_eq!(x.tw_state.tw(), tw0, "bar {} 后 TW 守恒被破坏", i);
        }
    }

    /// ★闭环每步 stage 单向不减（契约锚 `Origin.TotalWealth.stage_rank_monotone`——#127 native port）。
    #[test]
    fn hybrid_step_stage_monotone() {
        let mut x = AssemblyState::initial(1_000_000);
        for i in 0..50 {
            let prev_rank = x.tw_state.stage.rank();
            x = hybrid_step(&x, &bar_event(i % 2 == 0));
            assert!(
                prev_rank <= x.tw_state.stage.rank(),
                "bar {} 后 stage 回退",
                i
            );
        }
    }

    /// ★OQ-9 gate 闭环保持（契约锚 `Origin.TotalWealth.oq9inv_preserved` / `oq9inv_trace`——#127 native port）：
    /// schedule_adapter 只派生 ShortDiff/RecoverCapital（不开/不闭 legacy 腿）⟹ open_legacy_legs 恒 0。
    #[test]
    fn hybrid_step_oq9_gate_preserved() {
        let mut x = AssemblyState::initial(1_000_000);
        assert_eq!(x.tw_state.open_legacy_legs, 0);
        for i in 0..50 {
            x = hybrid_step(&x, &bar_event(i % 2 == 0));
            assert_eq!(
                x.tw_state.open_legacy_legs, 0,
                "bar {} 后 OQ-9 gate 被破坏（开了 legacy 腿）",
                i
            );
        }
        // earning 阶段假设：若闭环推进到 earning，gate 仍保持 legacy 腿=0。
        // 构造 earning 起点态验证 gate 在 earning 下保持。
        let earning = AssemblyState {
            tw_state: TwState { stage: TStage::EarningShares, ..TwState::initial() },
            ..AssemblyState::initial(1_000_000)
        };
        let x2 = hybrid_step(&earning, &bar_event(true));
        assert_eq!(x2.tw_state.open_legacy_legs, 0, "earning 下 OQ-9 gate 保持");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★反退化见证：双账本 + micro 真被线程化（非恒等挂件）
    // ──────────────────────────────────────────────────────────────────────

    /// ★tw_state 真被线程化（契约锚 `Origin.TotalWealth.twStep`——#127 native port，非恒等挂件）：
    /// 闭环后 tw_state = tw_step(x.tw_state, policy_output(x).tw_event)——非恒等挂件。
    #[test]
    fn hybrid_step_threads_tw_state() {
        let x = AssemblyState::initial(1_000_000);
        let e = bar_event(true);
        let order = policy_output(&x, &e);
        let x1 = hybrid_step(&x, &e);
        assert_eq!(x1.tw_state, tw_step(&x.tw_state, order.tw_event));
    }

    /// ★micro_state 真被推进（非恒等）：闭环后 bars_seen + bar_count 真增长。
    #[test]
    fn hybrid_step_threads_micro_state() {
        let x = AssemblyState::initial(1_000_000);
        let x1 = hybrid_step(&x, &bar_event(true));
        assert_eq!(x1.micro_state.bar_count, 1, "bar_count 真推进");
        assert_eq!(x1.micro_state.bars_seen, 1, "bars_seen 真推进");
        assert_eq!(x1.orders, 1, "orders 计数真推进");
        // 与构造一次不更新对比：连续 3 bar ⟹ bar_count=3（每 bar 真喂回）。
        let x2 = hybrid_step(&x1, &bar_event(false));
        let x3 = hybrid_step(&x2, &bar_event(true));
        assert_eq!(x3.micro_state.bar_count, 3, "3 bar 闭环 ⟹ bar_count=3（喂回非构造一次）");
        assert_eq!(x3.orders, 3, "3 bar ⟹ orders=3");
    }

    /// ★ledger_state 真被线程化（非恒等）：开仓侧动作 ⟹ Allocate ⟹ A/R 真变。
    #[test]
    fn hybrid_step_threads_ledger_state() {
        let x = AssemblyState::initial(1_000_000); // Normal/PhaseI ⟹ Buy ⟹ Allocate(1)
        let x1 = hybrid_step(&x, &bar_event(true));
        // Allocate(1) ⟹ A += 1, R -= 1。
        assert_eq!(x1.ledger_state.a, 1, "Allocate ⟹ A 真变（非恒等）");
        assert_eq!(x1.ledger_state.r, -1, "Allocate ⟹ R 真变");
        assert!(x1.ledger_state.inv_holds(), "R=Π-A-W 保持");
    }

    /// micro_delta 与 Dynamics.delta bit-exact（newStroke 清零 pending_rise 经 AssemblyEvent）。
    #[test]
    fn hybrid_step_micro_event_stroke() {
        let x = AssemblyState {
            micro_state: MicroState { pending_rise: 5, ..MicroState::initial() },
            ..AssemblyState::initial(1_000_000)
        };
        let e = AssemblyEvent { parse_event: MicroEvent::NewStroke(Direction::Up) };
        let x1 = hybrid_step(&x, &e);
        assert_eq!(x1.micro_state.stroke_count, 1);
        assert_eq!(x1.micro_state.pending_rise, 0, "新笔吸收尾部");
        assert_eq!(x1.micro_state.last_stroke_dir, Some(Direction::Up));
    }

    /// 闭环确定性（契约锚 `Origin.FullDefinitionStrategy.hybrid_step_complete_unique`）：同态同事件同后态。
    #[test]
    fn hybrid_step_deterministic() {
        let x = AssemblyState {
            risk_mode: RiskMode::Normal,
            phase: Phase::PhaseII,
            ..AssemblyState::initial(1_000_000)
        };
        let e = bar_event(true);
        assert_eq!(hybrid_step(&x, &e), hybrid_step(&x, &e));
    }
}
