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
use super::super::strategy::ledger::{
    ledger_step, tw_step, LedgerEvent, RiskPolicy, TStage, TwEvent, TwState,
};
use super::super::types::StrictAction;
use super::state::{micro_delta, AssemblyState, MicroEvent, Phase, RiskMode};

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
    /// ★GAP3 补桥（丢弃点2 修复）：本 bar 外生市价（整数 tick 域，成交/重估侧消费）。
    ///
    /// 事件字母表原只承载 `parse_event`（解析层只需 rising 布尔——分型/笔与价格幅度无关），价格幅度
    /// 在到达账本前被投影掉（codex 审计丢弃点1/2）。补桥在**账本层**（非解析层）承载幅度：`price` 由
    /// 成交侧（`schedule_adapter` 的 `affordable = free/price` 现金约束）与重估侧（`transition_adapter`
    /// 的 `Revalue` 浮盈入账）消费。`price=1`=单位价归一（L0 同价模型，值模型退化为旧单位模型）；
    /// `price=0`=不可交易 bar（无成交、无重估）。
    pub price: i64,
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
/// Control → Order`）：把控制（目标仓位）+ 意图（动作）打包为订单，**由真实仓位增量派生双账本事件**。
///
/// ★★codex #1 根因修复（幽灵买入）：账本事件由**真实仓位增量** `Δ = target_pos − x.positions` 派生，
/// **不**由「每 bar 每个 Buy 意图」无条件派生。旧版对 Buy/Add 无条件派 `ShortDiff(-1)`——但闭环里
/// intent 每 bar 恒 Buy 而 `risk_adapter` 首 bar 后恒返 `positions.max(1)=1`（仓位不再增），导致
/// holding 被「幽灵买入」逐 bar 累积（时间门控），而非真实仓位增长（仓位门控）。修复后 Δ=0（仓位
/// 未变）⟹ 无资金转移（`Noop` + `ShortDiff(0)`），holding 只在真实建仓（Δ>0）时增长。
///
/// 动作→账本副作用映射（**Δ 驱动**，账户因果链，TW 守恒）：
/// - **Δ>0（真实建仓）**⟹ `Allocate(Δ)`（资本化建仓占用 R）+ `ShortDiff(-Δ)`（free→holding 买入，
///   花 Δ 单位 free 换 Δ 单位 holding，TW 守恒——非恒等）。
/// - **Δ<0（真实减/平仓）**⟹ `Realize(|Δ|)`（回收实现盈亏）+ `ShortDiff(+|Δ|)`（holding→free 卖出，
///   |Δ| 单位 holding 变回 free 现金，TW 守恒——非恒等）。**卖出=holding→free**（现金回流），与
///   「退本金 free→withdrawn」是不同转移：退本金由 [`stage_progression`] 阶段机派生，schedule 不派
///   （codex #3 根因：schedule 不再把「卖出」误当「退本金」，避免从空 free 借本金）。
/// - **Δ=0（仓位不变，含 Hold/Wait 或 max(1) 饱和）**⟹ `Noop` + `ShortDiff(0)`（TW 不变零转移）。
///
/// ★OQ-9 gate（契约锚 `Origin.TotalWealth.LegalTransition`，#127 native port）：本段派生的 tw_event
/// 只取 `ShortDiff`（不是 OpenShareLeg/CloseShareLeg/RecoverCapital/EnterEarning），故恒合法且不开/
/// 不闭 legacy 腿——OQ-9 gate 自动满足（见 `assert_oq9_legal`）。
///
/// ★★现金约束（codex 复审#1 + GAP3 补桥维度修复）：**建仓额受可用 free 上界约束**——
/// `affordable_units = min(Δ, free/price)`（现金能买的单位数）。
///
/// **维度修复（codex 独立发现，丢弃点3）**：旧 `min(Δ, free)` 把「单位数增量 Δ」与「现金 free」直接取
/// min，**隐含单价=1**（维度错误：Δ 是单位数，free 是现金/价值）。正确上界是「现金能买的单位数」=
/// `free/price`。`price` 由事件承载（`AssemblyEvent.price`，丢弃点2 修复）：
/// - `price ≤ 0`（不可交易 bar）⟹ 不成交（避免除零，且不可交易 bar 无成交是正确退化）。
/// - `price = 1`（L0 单位价归一）⟹ `free/price = free` ⟹ **退化为旧单位模型**（值模型的 L0 特例）。
/// - `price > 1`（L2 真实市价）⟹ 单位成交额 = `filled_delta · price`（值模型：holding = Σunits·成本价）。
///
/// 若 free=0 或现金不足一单位 ⟹ affordable_units=0 ⟹ 不建仓。保证 `ShortDiff(-cost_flow)` 后 `free ≥ 0`
/// **全路径成立**：买入不透支现金，卖出只增 free。写回 `target_pos` = 实际成交后仓位（holding/positions/
/// free 三者一致）。
///
/// ★诚实：`holding` 是**成本基价值**（Σ 成交单位数·成交价）；未实现浮盈不入 holding，由 `Revalue`
/// 单独入 free（见 transition_adapter 重估步）。tw_event 与 ledger_event 双侧由同一实际成交额派生。
fn schedule_adapter(x: &AssemblyState, intent: StrictAction, target_pos: u64, price: i64) -> OrderOut {
    // 请求仓位增量 Δ = target_pos − positions（i64 域，可正可负）。
    let requested_delta = target_pos as i64 - x.positions as i64;
    // ★现金约束（维度修复）：建仓（Δ>0）受可用 free 上界——affordable_units = min(Δ, free/price)。
    // price≤0（不可交易）⟹ 不成交；减/平仓（Δ≤0）不受现金约束（卖出生成现金）。
    let filled_delta = if price <= 0 {
        0
    } else if requested_delta > 0 {
        requested_delta.min(x.tw_state.free.max(0) / price)
    } else {
        requested_delta
    };
    // 实际成交后仓位 = positions + filled_delta（holding/positions/free 三者一致）。
    let filled_pos = (x.positions as i64 + filled_delta).max(0) as u64;
    // 成交现金流（free⇄holding 转移的**价值**）：
    // - 建仓（Δ≥0）：按现价成为成本基 ⟹ cost_flow = Δ·price。
    // - 减/平仓（Δ<0）：按**持仓均成本基**移出 holding ⟹ cost_flow = Δ·(holding/positions)。
    //   现价浮盈已由 `Revalue` 计入 free（不重复计价），卖出只归还成本基 ⟹ **holding 恒≥0**（|Δ|≤positions
    //   ⟹ |Δ|·avg_cost ≤ holding，不下溢）。若按现价扣（holding−|Δ|·price）则在现价>成本价时下溢（bug）。
    //   price=1 时 avg_cost = holding/positions = 1 ⟹ 退化为旧单位模型。
    let cost_flow = if filled_delta >= 0 {
        filled_delta * price
    } else {
        let avg_cost = if x.positions > 0 { x.tw_state.holding / x.positions as i64 } else { 0 };
        filled_delta * avg_cost
    };
    let (ledger_event, tw_event) = if filled_delta > 0 {
        // 真实建仓（现金充足部分）：花 free 换 holding（free→holding 值），free 退后 ≥0。
        (LedgerEvent::Allocate(cost_flow), TwEvent::ShortDiff(-cost_flow))
    } else if filled_delta < 0 {
        // 真实减/平仓：卖出 holding 回 free（holding→free 值）。
        (LedgerEvent::Realize(-cost_flow), TwEvent::ShortDiff(-cost_flow))
    } else {
        // 仓位不变（含 Hold/Wait / max(1) 饱和 / 现金不足一单位 / 不可交易 bar）：无资金转移。
        (LedgerEvent::Noop, TwEvent::ShortDiff(0))
    };
    OrderOut {
        action: intent,
        target_pos: filled_pos,
        ledger_event,
        tw_event,
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
    // Schedule 段：打包订单 + 派生双账本事件（e.price 承载外生市价——现金约束 affordable=free/price）。
    schedule_adapter(x, intent, target_pos, e.price)
}

/// 闭环转移错误 `TransitionError`（GAP3 codex R3 §9.3：illegal 转移拦截 = **release 语义 `Result::Err`**，
/// 非 `assert!`/`debug_assert!` panic）。
///
/// ★codex R3 §9.3 根因（panic 非 release 错误处理语义）：`assert!` 拦截非法转移 = 进程 abort（release
/// 下 crash 而非可恢复错误）。裁决要求把 illegal `tw_event` 拦截写成 `Result::Err`——调用方决定如何处理
/// 非法输入（而非无条件崩溃）。非法性是**关系型**（`(tw_event, tw_state)` 组合，同一 event 在不同 stage
/// 合法性不同——见 `TwEvent::is_legal_from`），**无法在 `OrderOut` 构造时静态排除**（类型层不可构造不
/// 适用），故走 `Result::Err` 路径（PDF §9.3 二选一的可行分支）。
///
/// 两类非法（transition_adapter 的两道 gate）：
/// - `Oq9Illegal`：OQ-9 gate 违反（`tw_event` 在当前 `stage` 非法，如 EarningShares 阶段开 legacy 腿）。
/// - `CashUnsound`：现金-sound gate 违反（转移后 TW 三量出现负值——透支现金 / 空池借本金）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    /// OQ-9 gate 违反：`event` 在 `stage` 下非法（`TwEvent::is_legal_from` 返 false）。
    Oq9Illegal { event: TwEvent, stage: TStage },
    /// 现金-sound 违反：转移后 TW 三量出现负值（透支 / 空池借本金）。
    CashUnsound { free: i64, holding: i64, withdrawn: i64 },
}

/// OQ-9 gate 判定（契约锚 `Origin.TotalWealth.LegalTransition`，#127 native port）：tw_event 在当前
/// tw_state 下是否合法。
///
/// OpenShareLeg 在 stage=EarningShares 非法。本判定检查订单携带的 tw_event 是否当前态下的合法转移
/// （schedule_adapter 只派生 `ShortDiff`，恒合法，故对订单 tw_event 恒真；阶段推进事件
/// RecoverCapital/EnterEarning 由 stage_progression 单独在推进侧检查。它是引擎层「禁非法转移」的
/// 显式落实，对齐 `Origin.TotalWealth.oq9inv_preserved` 的单步保持——TW/OQ-9 的 Origin canonical
/// 重锚已由 #127 `Origin.TotalWealth` native port 落地）。
fn oq9_legal(tw_state: &TwState, tw_event: TwEvent) -> bool {
    tw_event.is_legal_from(tw_state)
}

/// ★三阶段推进算子 `stage_progression`（GAP3 根因修复：schedule_adapter 从不派 EnterEarning ⟹
/// stage 恒 CostReduction ⟹ EarningShares 不可达）。契约锚 PDF §10 步骤2/3 + `Origin.TotalWealth`
/// 单向阶段迁移（CostReduction→CapitalRecovered→EarningShares）。
///
/// 从当前 tw_state + RiskPolicy barrier 派生**下一个阶段推进事件**（`None`=本 bar 不推进阶段）：
/// - **CostReduction 阶段**：持仓市值 `holding` 累积到 ≥ 名义基线 `notional_in`（降成本期建仓过基线
///   ⟹ 本金可退）⟹ 派 `RecoverCapital(notional_in)`（free→withdrawn，退本金，推进 CapitalRecovered）。
/// - **CapitalRecovered 阶段**：`EnterReady` 严格谓词成立（S=II ∧ W≥I0 ∧ legacy 腿=0 ∧ RiskNormal ∧
///   η≥η⋆）⟹ 派 `EnterEarning`（切 EarningShares，单向不可逆相变，本金全退后纯利润增股数）。
/// - **EarningShares 阶段**：已到顶阶段 ⟹ 不再推进（`None`）。
///
/// ★这是 GAP3 的**结构修复**（非补丁）：把「阶段推进」从缺失补成 barrier-gated 显式算子——EnterEarning
/// 只在 barrier 过关时派生（κ=0 基线：η⋆=L^wc；κ>0：额外缓冲）。可证伪：κ=0 基线下若 barrier 仍不过
/// ⟹ EarningShares 不可达=结构性缺口（照实，非硬凑）。
///
/// `risk_mode`=当前风控（EnterReady 的 RiskNormal 门）。**I₀（EnterReady 的 W≥I0 门）取 campaign 本地
/// 本金 `s.notional_in`**（本 campaign 原始名义 = 退本金目标），**非账户 NAV**——「本金全退」是 campaign
/// 本地性质（退回本 campaign 投入的本金），与账户总 NAV 无关（避免把账户 NAV 误当 campaign 本金 ⟹
/// 巨额门 ⟹ 永不满足）。
fn stage_progression(policy: &RiskPolicy, s: &TwState, risk_mode: RiskMode) -> Option<TwEvent> {
    let risk_normal = matches!(risk_mode, RiskMode::Normal);
    match s.stage {
        // 降成本：持仓累积过名义基线 ⟹ 退本金（free→withdrawn），推进 CapitalRecovered。
        // notional_in>0（campaign 已开：funded_campaign 开局注资记 notional_in）时 holding≥notional_in
        // 才退——保证退本金有真实持仓支撑（非凭空退）。notional_in=0（未开 campaign）⟹ 不退（inert）。
        //
        // ★★codex #3 根因修复（负 free 退本金）：退本金额 `w` 受 **可用 free 上界约束**
        // ——`w = min(退本金目标, free)`，使 `RecoverCapital(w)` 后 `free ≥ 0` 恒成立（不从空/负 free
        // 借本金）。退本金目标 = `notional_in − withdrawn`（尚未退回的本金）。**若 free=0（建仓耗尽现金，
        // 纯累积模型）⟹ w=0 ⟹ 不派事件**（照实：退本金需真实 free 现金，纯累积无 free 可退——这是
        // 缠师「降成本=短差（买卖等量）」而非「纯建仓」的结构后果，见测试 `pure_accumulation_no_sound_recovery`）。
        TStage::CostReduction => {
            if s.notional_in > 0 && s.holding >= s.notional_in {
                let recover_target = (s.notional_in - s.withdrawn).max(0);
                // ★★足额一次性退本金（GAP3 补桥必需）：仅当 free ≥ recover_target（可 sound 退回**全部**
                // 尚未退本金）时才派 `RecoverCapital(recover_target)`；否则等待（None）让重估浮盈继续累积 free。
                //
                // 理由：`RecoverCapital` 单次即推进 CostReduction→CapitalRecovered（一次性相变），而 EnterReady
                // 严格谓词要求 `withdrawn ≥ notional_in`（本金全退）。若做**部分**退本金 ⟹ withdrawn<notional_in
                // 即搁浅 CapitalRecovered（该 stage 不再退本金，无回退通道）⟹ EnterReady 永假。故足额门是达
                // EarningShares 的必要条件，且保 `free ≥ 0`（recover_target ≤ free）。free 不足（含 free=0 纯累积、
                // 浮盈未达本金额）⟹ 等待（照实：退本金需真实可用 free，见 `pure_accumulation_no_sound_recovery`）。
                if recover_target > 0 && s.free >= recover_target {
                    Some(TwEvent::RecoverCapital(recover_target))
                } else {
                    None
                }
            } else {
                None
            }
        }
        // 退本金：EnterReady 严格谓词成立 ⟹ 进增股数（barrier-gated 单向相变）。I₀=campaign 本金
        // notional_in（本金全退 ⟺ withdrawn≥notional_in ⟹ L^wc=0）。
        TStage::CapitalRecovered => {
            if policy.enter_ready(s, s.notional_in, risk_normal) {
                Some(TwEvent::EnterEarning)
            } else {
                None
            }
        }
        // 增股数：顶阶段，不再推进。
        TStage::EarningShares => None,
    }
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
/// - `phase` := `phase_from_stage(tw_next.stage)`（**由取本金阶段派生写回**，codex #2 根因修复：
///   phase 是 stage 的确定函数，非独立状态——stage 推进即驱动 phase 推进，消除双相位漂移）。
/// - `memory`、`risk_mode`：本骨架保持（μ 取值阈值是 Θ_signal/Θ_risk，L2；结构层装配保持，不臆造
///   阈值——诚实留白，非 workaround：阈值驱动 μ 转移属下游有效域，本文件闭合 micro/ledger/tw_state/
///   positions/orders/phase 的结构闭环）。
///
/// ★诚实标注（codex R3）：T **只声明闭环状态转移全定义**（产出确定的下一态），**不**声明该转移
/// 盈利/最优/实盘有效（L3）。
///
/// ★三阶段推进（GAP3 修复）：base tw_step（订单派生事件）后，再经 [`stage_progression`]（barrier-gated）
/// 派生**阶段推进事件**并 tw_step 一次——stage_progression **仅在有 sound 资金源**（cash-tight
/// 退本金 w≤free）时推进；L0 同价 funded campaign 下无源 ⟹ 恒不推进（EarningShares 结构不可达，见
/// `earning_shares_structurally_unreachable_from_campaign_tw_conserved`）。两个 tw_step 都保 TW 守恒 +
/// stage 单向 + OQ-9 gate + 出口现金-sound 检查。
///
/// ★★codex R3 §9.3（release 语义）：两道 gate（OQ-9 + 现金-sound）从 `assert!` panic 改为
/// **`Result::Err`**——非法转移返回 [`TransitionError`]（release 下可恢复错误，非 abort），调用方决定
/// 处理。**生产路径恒 `Ok`**（hybrid_step→schedule 只派 `ShortDiff`——恒合法；cash 约束 + stage_progression
/// w≤free ⟹ 出口三量恒非负），故 `Err` 仅对**外部注入非法 `OrderOut`** 触发（pub transition_adapter +
/// pub OrderOut 外部注入面）。这使「free≥0 全路径」成为出口不变量：**任何返回 `Ok(x)` 的转移，`x` 的
/// TW 三量恒非负**（否则返 `Err`，不产出非法态）。
pub fn transition_adapter(
    x: &AssemblyState,
    o: &OrderOut,
    e: &AssemblyEvent,
    policy: &RiskPolicy,
) -> Result<AssemblyState, TransitionError> {
    // OQ-9 gate：订单 tw_event 必须合法（schedule_adapter 保证只派生 ShortDiff——恒合法）。
    // ★codex R3 §9.3：非法 ⟹ `Result::Err`（release 语义，非 panic）。外部注入非法 tw_event 时返
    // Err 而非 abort；生产路径（schedule 只派 ShortDiff）恒 Ok，对生产零行为影响。
    if !oq9_legal(&x.tw_state, o.tw_event) {
        return Err(TransitionError::Oq9Illegal { event: o.tw_event, stage: x.tw_state.stage });
    }
    // base tw_step（订单派生事件：schedule 只派 ShortDiff——Δ 驱动的 free⇄holding 值转移）。
    let tw_after_order = tw_step(&x.tw_state, o.tw_event);
    // ★★GAP3 补桥重估步（丢弃点1/2/3 联合修复：价格幅度打通到 TW 账本）：把外生市价浮盈入账为可分配
    // 权益。unrealized = positions·price − holding（成本基）；只对**超过 hwm_gain 高水位的增量**派
    // `Revalue(credit)`，credit = max(0, unrealized − hwm_gain) ⟹ free 单调不减于重估侧 ⟹ **free≥0**
    // （价格回撤不透支现金，出口 cash-sound gate 不触发）。语义分层：
    // - price=1（L0 单位价归一）⟹ holding=成本基=positions·1 ⟹ unrealized=0 ⟹ credit=0 ⟹ Revalue 不派发
    //   ⟹ **退化为旧 L0 守恒语义**（TW 守恒，结构不可达定理不受影响）。
    // - price>1 且浮盈创新高（L2 变价）⟹ credit>0 ⟹ Revalue 使 TW 增 credit ⟹ free 累积 ⟹ 三阶段机可
    //   推进（达 EarningShares 的可达通道，acc-GAP3）。
    let tw_after_revalue = if e.price > 0 {
        let unrealized = (o.target_pos as i64) * e.price - tw_after_order.holding;
        let credit = (unrealized - tw_after_order.hwm_gain).max(0);
        if credit > 0 {
            tw_step(&tw_after_order, TwEvent::Revalue(credit))
        } else {
            tw_after_order
        }
    } else {
        tw_after_order
    };
    // ★阶段推进（GAP3）：barrier-gated 派生 RecoverCapital→EnterEarning。stage_progression 在重估后的态
    // 上评估——L2 浮盈入 free 后满足「足额退本金 free≥recover_target」时推进；L0 同价（credit=0）下无源 ⟹
    // 恒不推进（照实不可达）。
    let tw_next = match stage_progression(policy, &tw_after_revalue, x.risk_mode) {
        Some(stage_event) => {
            // ★codex R3 §9.3：阶段推进事件（RecoverCapital/EnterEarning）非法 ⟹ Err（RecoverCapital
            // raw 恒合法；EnterEarning 由 enter_ready 要求 legs==0 ⟹ 与 is_legal_from 一致，故生产恒
            // 合法）。release 语义拒非法阶段推进。
            if !oq9_legal(&tw_after_revalue, stage_event) {
                return Err(TransitionError::Oq9Illegal { event: stage_event, stage: tw_after_revalue.stage });
            }
            tw_step(&tw_after_revalue, stage_event)
        }
        None => tw_after_revalue,
    };
    // ★★现金-sound gate（codex 复审二轮致命1 根因：唯一 chokepoint，覆盖全部调用方）：转移后 TW 三量
    // 必须非负（free/holding/withdrawn ≥ 0）。这不只堵 schedule_adapter 生产路径，还堵 **pub
    // transition_adapter + pub OrderOut 的外部注入面**：外部即使传 ShortDiff(-1)（透支现金）或
    // RecoverCapital(1)（空池借本金）绕过 OQ-9 gate（这两类 raw 恒合法），也在此**返回 Err**（codex R3
    // §9.3：release 语义），而非 panic 或静默写负 free/holding。生产路径（schedule cash-约束 +
    // stage_progression w≤free）恒过 ⟹ 生产恒 Ok。这使「free≥0 全路径」成为 transition_adapter 出口
    // 不变量（返回 Ok(x) ⟹ x 的 TW 三量非负；否则 Err，不产出非法态）。
    if tw_next.free < 0 || tw_next.holding < 0 || tw_next.withdrawn < 0 {
        return Err(TransitionError::CashUnsound {
            free: tw_next.free,
            holding: tw_next.holding,
            withdrawn: tw_next.withdrawn,
        });
    }
    Ok(AssemblyState {
        micro_state: micro_delta(&x.micro_state, e.parse_event),
        ledger_state: ledger_step(&x.ledger_state, o.ledger_event),
        tw_state: tw_next,
        risk_mode: x.risk_mode,
        // ★codex #2 根因修复（phase/TStage 脱钩）：phase **由 tw_state.stage 派生写回**，不再原样
        // 保留 x.phase。TStage 是资本相位的唯一真值状态机（CostReduction=建仓期/CapitalRecovered=
        // 取本期/EarningShares=增股数期）；旧版 phase 恒 PhaseI 与 stage 分裂 ⟹ 即使 stage 到
        // EarningShares，classify→intent 仍读 PhaseI ⟹ 恒 Buy。派生写回后 stage 推进即驱动 phase
        // 推进 ⟹ intent 随相位变（PhaseII→Reduce/PhaseIII→Add），消除双相位漂移（state.rs「不冗余
        // 存双份」原则的兑现：phase 是 stage 的确定函数 [`phase_from_stage`]，非独立状态）。
        phase: phase_from_stage(tw_next.stage),
        positions: o.target_pos,
        orders: x.orders + 1,
        memory: x.memory,
    })
}

/// 资本相位 `Phase` 由取本金阶段 `TStage` 派生（codex #2 根因修复：消除双相位漂移）。
///
/// `TStage` 是资本相位的**唯一真值状态机**（单向 CostReduction→CapitalRecovered→EarningShares）；
/// `Phase` 是其在 classify→intent 数据流上消费的三态摘要，二者一一对应（非独立状态，避免双份漂移）：
/// - `CostReduction`（降成本/建仓期）→ `PhaseI`（建根仓 ⟹ intent=Buy）。
/// - `CapitalRecovered`（退本金期）→ `PhaseII`（取本减仓 ⟹ intent=Reduce）。
/// - `EarningShares`（增股数期）→ `PhaseIII`（增核加仓 ⟹ intent=Add）。
pub fn phase_from_stage(stage: TStage) -> Phase {
    match stage {
        TStage::CostReduction => Phase::PhaseI,
        TStage::CapitalRecovered => Phase::PhaseII,
        TStage::EarningShares => Phase::PhaseIII,
    }
}

/// ★★闭环一步 `hybrid_step`（契约锚 `Origin.FullDefinitionStrategy.hybridStep`：
/// `hybridStep S x e = S.transition x (policyTheta S x e) e`）。
///
/// x_t ─e→ x_{t+1} 单一闭环：六段（rec→classify→intent→risk→schedule→transition）复合为单一转移，
/// 由 Origin `hybrid_step_complete_unique` 证确定唯一。rec 段在 transition_adapter 的 micro_delta 中
/// 体现；前五段经 [`policy_output`]（= Origin `policyTheta`）产订单；transition 写回。
///
/// 这是引擎的闭环核心——runner 每 bar 调 `x = hybrid_step(x, e, policy)`，micro_state/ledger_state/
/// tw_state/positions/orders 每 bar 真更新喂回（消除开环单帧，对齐 Origin `transition_writes_full_state`）。
///
/// `policy`=RiskPolicy（barrier κ；驱动三阶段推进 RecoverCapital→EnterEarning，GAP3 修复）。κ=0 基线用
/// [`hybrid_step_baseline`]（PDF §10 最小规范）。
///
/// ★codex R3 §9.3：透传 [`transition_adapter`] 的 `Result`——非法转移（外部注入非法 OrderOut）返
/// [`TransitionError`]。生产路径（policy_output→schedule 只派 ShortDiff）恒 `Ok`；调用方（如
/// `run_closed_loop`）在生产边界用 `.expect("生产路径恒 Ok")` 收敛，把「生产恒合法」变成显式契约。
pub fn hybrid_step(
    x: &AssemblyState,
    e: &AssemblyEvent,
    policy: &RiskPolicy,
) -> Result<AssemblyState, TransitionError> {
    let order = policy_output(x, e);
    transition_adapter(x, &order, e, policy)
}

/// 闭环一步的 **κ=0 最小基线**便捷入口（PDF §10 canonical 默认 `RiskPolicy::baseline()`）。
///
/// `η⋆=L^wc`（仅覆盖最坏损失无额外缓冲）。这是 GAP3 可达性测试的基线政策——若 κ=0 下 EarningShares
/// 仍不可达 ⟹ 结构性缺口（照实报，非硬凑）。
///
/// ★codex R3 §9.3：透传 `Result`（生产路径恒 `Ok`；见 [`hybrid_step`]）。
pub fn hybrid_step_baseline(
    x: &AssemblyState,
    e: &AssemblyEvent,
) -> Result<AssemblyState, TransitionError> {
    hybrid_step(x, e, &RiskPolicy::baseline())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state::{MicroState, Phase, RiskMode};
    use super::super::super::strategy::ledger::TStage;
    use super::super::super::types::Direction;

    fn bar_event(rising: bool) -> AssemblyEvent {
        // price=1：L0 单位价归一（值模型退化为旧单位模型，重估 credit=0，测试语义不变）。
        AssemblyEvent { parse_event: MicroEvent::NewBar(rising), price: 1 }
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
        assert_eq!(
            hybrid_step_baseline(&x, &e),
            transition_adapter(&x, &order, &e, &RiskPolicy::baseline())
        );
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
        assert_eq!(policy_output(&x, &e), schedule_adapter(&x, intent, target, e.price));
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
            x = hybrid_step_baseline(&x, &bar_event(i % 2 == 0)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
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
            x = hybrid_step_baseline(&x, &bar_event(i % 3 == 0)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
            assert_eq!(x.tw_state.tw(), tw0, "bar {} 后 TW 守恒被破坏", i);
        }
    }

    /// ★闭环每步 stage 单向不减（契约锚 `Origin.TotalWealth.stage_rank_monotone`——#127 native port）。
    #[test]
    fn hybrid_step_stage_monotone() {
        let mut x = AssemblyState::initial(1_000_000);
        for i in 0..50 {
            let prev_rank = x.tw_state.stage.rank();
            x = hybrid_step_baseline(&x, &bar_event(i % 2 == 0)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
            assert!(
                prev_rank <= x.tw_state.stage.rank(),
                "bar {} 后 stage 回退",
                i
            );
        }
    }

    /// ★OQ-9 gate 闭环保持（契约锚 `Origin.TotalWealth.oq9inv_preserved` / `oq9inv_trace`——#127 native port）：
    /// schedule_adapter 只派生 ShortDiff（不开/不闭 legacy 腿）⟹ open_legacy_legs 恒 0。
    #[test]
    fn hybrid_step_oq9_gate_preserved() {
        let mut x = AssemblyState::initial(1_000_000);
        assert_eq!(x.tw_state.open_legacy_legs, 0);
        for i in 0..50 {
            x = hybrid_step_baseline(&x, &bar_event(i % 2 == 0)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
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
        let x2 = hybrid_step_baseline(&earning, &bar_event(true)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
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
        let x1 = hybrid_step_baseline(&x, &e).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
        assert_eq!(x1.tw_state, tw_step(&x.tw_state, order.tw_event));
    }

    /// ★micro_state 真被推进（非恒等）：闭环后 bars_seen + bar_count 真增长。
    #[test]
    fn hybrid_step_threads_micro_state() {
        let x = AssemblyState::initial(1_000_000);
        let x1 = hybrid_step_baseline(&x, &bar_event(true)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
        assert_eq!(x1.micro_state.bar_count, 1, "bar_count 真推进");
        assert_eq!(x1.micro_state.bars_seen, 1, "bars_seen 真推进");
        assert_eq!(x1.orders, 1, "orders 计数真推进");
        // 与构造一次不更新对比：连续 3 bar ⟹ bar_count=3（每 bar 真喂回）。
        let x2 = hybrid_step_baseline(&x1, &bar_event(false)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
        let x3 = hybrid_step_baseline(&x2, &bar_event(true)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
        assert_eq!(x3.micro_state.bar_count, 3, "3 bar 闭环 ⟹ bar_count=3（喂回非构造一次）");
        assert_eq!(x3.orders, 3, "3 bar ⟹ orders=3");
    }

    /// ★ledger_state 真被线程化（非恒等）：**现金充足**的开仓侧动作 ⟹ Allocate ⟹ A/R 真变。
    /// codex 复审#1 后：买入受 free 约束，故用 funded_campaign（free=Q>0）使首 bar 真建仓（Δ=1 现金足）。
    #[test]
    fn hybrid_step_threads_ledger_state() {
        let x = AssemblyState::funded_campaign(1_000_000, 8); // free=8 ⟹ Buy Δ=1 现金充足 ⟹ Allocate(1)
        let x1 = hybrid_step_baseline(&x, &bar_event(true)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
        // Allocate(1) ⟹ A += 1, R -= 1（首 bar 仓位 0→1，现金 8>0 ⟹ 真成交）。
        assert_eq!(x1.ledger_state.a, 1, "Allocate ⟹ A 真变（非恒等）");
        assert_eq!(x1.ledger_state.r, -1, "Allocate ⟹ R 真变");
        assert!(x1.ledger_state.inv_holds(), "R=Π-A-W 保持");
        assert!(x1.tw_state.free >= 0, "买入后 free≥0（8-1=7，codex 复审#1 现金约束）");
    }

    /// ★codex/lead round-2 回归：**未注资 initial（free=0）闭环 free 从不为负**——Buy Δ=1 被现金约束
    /// 到 0（filled=min(1, free=0)=0），positions 不推进，free 恒 0（旧版漏网 free=-1 已堵）。
    #[test]
    fn unfunded_initial_never_negative_free() {
        let mut x = AssemblyState::initial(1_000_000); // free=0, positions=0
        for i in 0..20 {
            x = hybrid_step_baseline(&x, &bar_event(i % 2 == 0)).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
            assert!(x.tw_state.free >= 0, "bar {i}: free={} 变负（buy 透支现金漏网）", x.tw_state.free);
            assert_eq!(x.positions, 0, "free=0 ⟹ Buy 被现金约束 ⟹ positions 不推进");
        }
    }

    /// micro_delta 与 Dynamics.delta bit-exact（newStroke 清零 pending_rise 经 AssemblyEvent）。
    #[test]
    fn hybrid_step_micro_event_stroke() {
        let x = AssemblyState {
            micro_state: MicroState { pending_rise: 5, ..MicroState::initial() },
            ..AssemblyState::initial(1_000_000)
        };
        let e = AssemblyEvent { parse_event: MicroEvent::NewStroke(Direction::Up), price: 1 };
        let x1 = hybrid_step_baseline(&x, &e).expect("生产恒 Ok（schedule 只派 ShortDiff + cash 约束）");
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
        assert_eq!(hybrid_step_baseline(&x, &e), hybrid_step_baseline(&x, &e));
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★codex R3 §9.3：release 语义 Result::Err 路径（外部注入非法 OrderOut ⟹ Err，非 panic）
    // ──────────────────────────────────────────────────────────────────────

    /// ★9.3 生产路径恒 Ok（release 语义回归）：hybrid_step（policy_output→schedule 只派 ShortDiff）在
    /// 任意态/事件下恒返 `Ok`——release 下不 panic 不 Err，正常闭环。
    #[test]
    fn transition_production_path_always_ok() {
        let mut x = AssemblyState::funded_campaign(1_000_000, 8);
        for i in 0..50 {
            let r = hybrid_step(&x, &bar_event(i % 2 == 0), &RiskPolicy::baseline());
            assert!(r.is_ok(), "生产路径 bar {i} 返 Err（应恒 Ok）：{:?}", r);
            x = r.unwrap();
        }
    }

    /// ★9.3 OQ-9 gate Err 路径（外部注入非法 OrderOut）：EarningShares 阶段注入 `OpenShareLeg`
    /// tw_event（`is_legal_from` 返 false，rank 2<2 假）⟹ transition_adapter 返
    /// `Err(Oq9Illegal)`——**非 panic**（release 可恢复错误）。
    #[test]
    fn transition_oq9_illegal_returns_err() {
        let x = AssemblyState {
            tw_state: TwState { stage: TStage::EarningShares, ..TwState::initial() },
            ..AssemblyState::initial(1_000_000)
        };
        // 外部注入非法订单：EarningShares 阶段开 legacy 腿（OQ-9 gate 拒）。
        let illegal = OrderOut {
            action: StrictAction::Hold,
            target_pos: x.positions,
            ledger_event: LedgerEvent::Noop,
            tw_event: TwEvent::OpenShareLeg,
        };
        let r = transition_adapter(&x, &illegal, &bar_event(true), &RiskPolicy::baseline());
        assert_eq!(
            r,
            Err(TransitionError::Oq9Illegal { event: TwEvent::OpenShareLeg, stage: TStage::EarningShares }),
            "外部注入 EarningShares+OpenShareLeg ⟹ Err(Oq9Illegal)（release 语义，非 panic）"
        );
    }

    /// ★9.1/9.3 现金-sound gate Err 路径（外部注入透支订单）：free=0 态注入 `ShortDiff(-1)`
    /// （`tw_step` 语义 `ShortDiff(d): free+=d, holding-=d` ⟹ d=-1 ⟹ free 0→-1, holding 0→+1，
    /// raw 恒合法过 OQ-9，但转移后 free=-1 透支）⟹ 返 `Err(CashUnsound)`——**非 panic**。这钉死
    /// 「free≥0 全 transition 出口不变量」（codex §9.1）：**任何**返 Ok 的转移 TW 三量非负，否则 Err
    /// （不产出非法态）。
    #[test]
    fn transition_cash_unsound_returns_err() {
        let x = AssemblyState::initial(1_000_000); // free=0
        // 外部注入透支订单：ShortDiff(-1) ⟹ free 0→-1（透支现金），holding 0→+1（raw 合法过 OQ-9）。
        let overdraft = OrderOut {
            action: StrictAction::Buy,
            target_pos: x.positions + 1,
            ledger_event: LedgerEvent::Allocate(1),
            tw_event: TwEvent::ShortDiff(-1),
        };
        let r = transition_adapter(&x, &overdraft, &bar_event(true), &RiskPolicy::baseline());
        assert_eq!(
            r,
            Err(TransitionError::CashUnsound { free: -1, holding: 1, withdrawn: 0 }),
            "外部注入透支 ShortDiff(-1) 从 free=0 ⟹ Err(CashUnsound{{free:-1}})（release 语义，非 panic）"
        );
    }
}
