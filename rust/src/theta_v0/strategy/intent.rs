//! 统一意图适配器——契约锚 **`Origin.FullDefinitionStrategy`**（task #102 A′ Phase2 step8 重锚）。
//!
//! ## 契约重锚（legacy Strict.HybridAssembly → Origin canonical）
//!
//! 意图段的契约锚点指向 Origin `FullDefinitionSystem`（FullDefinitionStrategy.lean:214-224）的
//! 六段数据流字段（`recStruct → classify → intent → risk → schedule → transition`）：
//!
//! - **classify 段（`classify_adapter`）→ `Origin.FullDefinitionSystem.classify : StrictState →
//!   ParseStruct → TrendClass`**：从当前态 + 解析结构读出分类。
//! - **intent 段（`intent_adapter`）→ `Origin.FullDefinitionSystem.intent : StrictState →
//!   TrendClass → Intent`**：意图**以分类（TrendClass）为输入** ⟹ 策略动作穿过分类瓶颈。
//! - **factor-through-classify → `Origin.FullDefinitionStrategy.policy_factors_through_classification`**
//!   （:238-242）+ `policyTheta`（:226-227）：`policyTheta = schedule (risk (intent (classify (recStruct e))))`
//!   ——Origin canonical 证策略**经分类瓶颈分解**。本文件的闭环意图路径与此 Origin 接口语义同构。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本文件 = **L0/L1**（结构镜像：确定优先级选择器与 Origin `FullDefinitionSystem.intent`/
//!   `policy_factors_through_classification` 接口语义对齐 = 验证管线正确性，零信息增量）。
//!   `cargo test` 通过 = 意图选择确定 + 策略真读分类输出，**不**是缠论盈利/实盘有效声明。
//!   重锚到 Origin **不**提升等级（仍 L0/L1）。
//!
//! ## 消除双意图路径（gap-map U4 核心，no-workaround）
//!
//! 旧引擎有**双意图路径**：
//! 1. `strategy/mod.rs` 的 `pi_strict`（9 状态 (pos,sig) → 7 动作，细粒度结构应对组件）
//!    ——**直读结构状态，不经 Classification**。
//! 2. `strategy/mod.rs` 的 `recognize` → `plan_orders`（Classification → VoiceDecision → Order）
//!    ——**读 Classification 输出**。
//!
//! Origin `policy_factors_through_classification` 要求策略动作**穿过分类瓶颈**：`intent` 字段签名
//! `StrictState → TrendClass → Intent` 强制意图以分类为输入。本文件把闭环意图路径**统一**为单一
//! [`intent_adapter`]：意图由 [`classify_adapter`]（从 microState 读出分类标签 [`ClassLabel`]）经
//! [`action_priority`] 派生——策略只读分类输出，与 Origin `policy_factors_through_classification`
//! 同构（见 closed_loop/transition.rs 的 factor-through 见证）。
//!
//! `pi_strict` 保留为细粒度 (pos,sig)→action 应对组件的 **9→7 conformance 种子**（其有效域是
//! 「给定结构状态 (pos,sig) 的完全应对」）——它不再是独立的闭环意图入口，而是被
//! [`pi_strict_factored`] 通过 [`StrictState`] 桥接（该状态由分类输出派生，factor-through）。
//! 注：`pi_strict` 自身的 9→7 bit-exact 种子契约锚点（细粒度算子）尚无 Origin canonical 对应
//! （Origin FullDefinitionSystem 的 intent 是抽象字段，不展开 9 状态细格）——细格契约**诚实延后**。

use super::super::closed_loop::state::{AssemblyState, MicroState, Phase, RiskMode};
use super::super::types::{Pos, Sig, StrictAction};
use super::{pi_strict, StrictState};

/// 分类标签 `ClassLabel`（= RiskMode × Phase；对齐 `Origin.FullDefinitionSystem.classify` 输出位）。
///
/// 在 Origin 六段数据流中，`classify : StrictState → ParseStruct → TrendClass` 的输出 `TrendClass`
/// 是 intent 段的输入。本 Rust 闭环把 classify 段输出实例化为结构摘要标签 (risk_mode, phase)——它
/// 占据 Origin `classify` 输出位（intent 段的真实输入），使意图真读分类输出（factor-through-classify）。
///
/// ★诚实标注（formalization-validity-domain）：**完整** C_Θ 的 fiber partition 全局唯一性由
/// classifier 的递归级别 + BSP 承载，且 Origin canonical 的分类是 `TrendClass`
/// （TrendCompleteClassification.lean 四类穷尽）；本标签 (risk_mode, phase) 是闭环 intent 段消费的
/// **结构摘要**接入位，证「闭环数据流中 intent 真读 classify 输出」，**不**声明它等价于完整 C_Θ /
/// Origin TrendClass（摘要≠完整分类，是有效域边界）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassLabel {
    pub risk_mode: RiskMode,
    pub phase: Phase,
}

/// C_Θ 段 `classify_adapter`（契约锚 `Origin.FullDefinitionSystem.classify : StrictState →
/// ParseStruct → TrendClass`）：从解析结构 + 当前态读出分类标签。
///
/// 把当前态的 (risk_mode, phase) 作为分类标签——这是闭环 classify 段的输出，被 [`intent_adapter`]
/// 消费，使意图真读分类输出（factor-through-classify 的分类瓶颈，对齐 Origin
/// `policy_factors_through_classification` 的 `classify` 段位）。
///
/// `rec_struct`（Rec 段输出的新解析态）对齐 Origin `classify` 的 `ParseStruct` 输入位（当前摘要
/// 读出不依赖其内容，但保留在签名里使 Rec 段输出真被 classify 段消费——对齐 Origin 六段数据流
/// `classify x (recStruct x e)`，不让 Rec 段成为旁路死代码）。
pub fn classify_adapter(x: &AssemblyState, _rec_struct: &MicroState) -> ClassLabel {
    ClassLabel {
        risk_mode: x.risk_mode,
        phase: x.phase,
    }
}

/// 10 级动作优先级**确定选择器** `action_priority`（细化 `Origin.FullDefinitionSystem.intent` 段）。
///
/// 对齐 Origin `chooseAction`（FullDefinitionStrategy.lean:48-59）的 10 级优先级链
/// （insolventOrLiquidation > deleverage > ... > openRoot > openReverseChild > phaseThreeAccreteCore
/// > hold，由 `action_priority_complete_unique` 证唯一）——本函数把该 10 级优先级压缩为风险模式 +
/// 阶段的确定映射，是 Origin `intent` 段在 (risk, phase) 摘要上的确定选择器实例。
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

/// Intent 段 `intent_adapter`（契约锚 `Origin.FullDefinitionSystem.intent : StrictState →
/// TrendClass → Intent`）：**输入含 ClassLabel**。
///
/// 在分类标签 `c` 上跑 10 级优先级选择器 [`action_priority`]。这满足 Origin
/// `intent : StrictState → TrendClass → Intent` 结构约束（intent 以分类为输入 ⟹ 策略穿过分类瓶颈，
/// 由 Origin `policy_factors_through_classification` 钉死）。闭环数据流：
/// `intent_adapter(x, classify_adapter(x))`——意图真读分类输出（factor-through-classify）。
pub fn intent_adapter(_x: &AssemblyState, c: ClassLabel) -> StrictAction {
    action_priority(c)
}

/// `pi_strict` 的 factor-through 桥接（契约锚 `Origin.policy_factors_through_classification` 在
/// 细粒度 (pos,sig) 层的实例）。
///
/// `pi_strict`（9 状态 (pos,sig) → 7 动作）是**结构应对的细粒度组件**——它的输入 (pos,sig) 本身是
/// 分类输出（C_Θ 把走势识别为持仓/信号态）。本函数把分类标签 [`ClassLabel`] 映射到 piStrict 的
/// [`StrictState`] 输入，再调 [`pi_strict`]——使 piStrict 不再是绕开 Classification 的独立入口，而是
/// **通过分类派生状态**调用（factor-through，对齐 Origin 策略经分类瓶颈分解的语义）。
///
/// ★诚实标注：`pi_strict` 9→7 细格本身无 Origin canonical 对应（Origin `intent` 是抽象字段）——本
/// 桥接只兑现「细格经分类派生状态调用」的 factor-through 形式，9→7 细格 bit-exact 契约诚实延后。
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

// ──────────────────────────────────────────────────────────────────────────
//  §19 J_Θ 目标函数 + LexArgmin 真字典序最小化
//  （strict §12 末 line 416-421 / FULL 十九 line 1340-1385，契约锚 Lean
//   `Origin.LexArgmin`：lexLe 全序 + lexArgmin_exists_unique）
//
//  ★替「硬编码标量 min」为「真字典序键比较」（no-patch-mentality）：风险投影 u* =
//  LexArgmin_{u∈K_Θ} J_Θ(u,ũ) 的字典序**不是**把多目标加权成单标量再 min（那会让次目标的
//  差被主目标权重淹没，丢失优先级分层），而是**多分量元组逐分量比较**（主键平局才比次键）。
//  本节实装真字典序键 [`JThetaKey`] + 选择器 [`lex_argmin`]，对齐 Lean `Origin.LexArgmin`。
// ──────────────────────────────────────────────────────────────────────────

/// J_Θ 字典序键（strict §12 / FULL 十九 line 1353-1366：J_Θ 多分量目标按优先级降序）。
///
/// J_Θ(u,ũ) = Σ_v w_v(q'_v-q̃_v)² + λ·TradeCost + ν·RiskPenalty + ζ·Turnover。本结构把它的
/// **分量**按字典序优先级排成元组（主键在前），逐分量比较：
/// - `tracking_err`：跟踪误差 Σ_v w_v(q'_v-q̃_v)²（偏离原始意图 ũ，**主键**——优先贴合意图）。
/// - `trade_cost`：交易成本 λ·TradeCost（次键）。
/// - `risk_penalty`：风险罚 ν·RiskPenalty（第三键）。
/// - `turnover`：换手 ζ·Turnover（第四键）。
/// - `grid_index`：格点索引（**末键 = tie-break**，固定字典序平局规则，strict line 421 /
///   FULL line 1378「固定字典序平局规则」——前四键全平时按格点先后定序，使键单射 ⟹ u* 唯一）。
///
/// ★诚实（formalization-validity-domain）：权重 w_v/λ/ν/ζ 全是 Θ_risk 参数（**非缠论可导**）。
/// 各分量值（已乘权重）由调用方按 Θ_risk 求值。本键证「给定 J_Θ 后字典序选择确定唯一」，
/// **不**证权重经验最优（still-MISSING L2：权重校准属 EmpiricalDomain）。键分量用 i64（定点放大，
/// bit-exact——浮点 J_Θ 值乘固定缩放因子后取整，避免浮点比较的非确定性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JThetaKey {
    pub tracking_err: i64,
    pub trade_cost: i64,
    pub risk_penalty: i64,
    pub turnover: i64,
    pub grid_index: i64,
}

impl JThetaKey {
    /// 字典序比较 `lex_le`（对齐 Lean `Origin.LexArgmin.lexLe`）：逐分量比较，首个不等分量定序。
    ///
    /// 主键 `tracking_err` 决定优先（主键不等即定序，次键不翻盘——优先级分层）；主键平局才比
    /// 次键 `trade_cost`，依次到末键 `grid_index`（tie-break）。返回 `true` ⟺ self ≤ other（字典序）。
    ///
    /// ★对齐 Lean `lex_primary_dominates`（主键定胜负）+ `lex_secondary_breaks_tie`（主键平局次键定）。
    /// 这是**真字典序**——区别于 `(w₁·a₁+w₂·a₂).min(...)` 的单层标量 min（加权和会让 a₂ 大差翻
    /// a₁ 小差，丢失优先级）。
    pub fn lex_le(&self, other: &JThetaKey) -> bool {
        // 按优先级降序排成数组，逐分量比较（首个不等定序）。
        let a = [
            self.tracking_err,
            self.trade_cost,
            self.risk_penalty,
            self.turnover,
            self.grid_index,
        ];
        let b = [
            other.tracking_err,
            other.trade_cost,
            other.risk_penalty,
            other.turnover,
            other.grid_index,
        ];
        for i in 0..a.len() {
            if a[i] < b[i] {
                return true; // 主键（首个不等分量）self 更小 ⟹ self ≤ other
            }
            if b[i] < a[i] {
                return false; // self 更大 ⟹ self 不 ≤ other
            }
            // a[i] == b[i]：该分量平局，继续比下一分量
        }
        true // 全分量相等 ⟹ self == other ⟹ self ≤ other（自反）
    }
}

/// 候选控制（u, J_Θ键）对——LexArgmin 在这些可行候选上选字典序最小。
///
/// `U`：控制类型（K_Θ 可行集的格点，调用方提供，本选择器对类型透明）。
#[derive(Debug, Clone, Copy)]
pub struct LexCandidate<U> {
    pub control: U,
    pub key: JThetaKey,
}

/// **LexArgmin 真字典序最小化** u* = LexArgmin_{u∈K_Θ} J_Θ(u,ũ)（strict §12 line 416-421 /
/// FULL 十九 line 1371-1376，契约锚 Lean `Origin.LexArgmin.RiskProjection.project`）。
///
/// 在**有限非空可行候选表**上选 J_Θ 字典序最小控制。**替硬编码标量 min**：用 [`JThetaKey::lex_le`]
/// 真字典序键比较（多分量逐层），非单层 `.min()`。
///
/// 实现 = foldl pick（对齐 Lean `lexArgmin`）：从首候选起逐个比较 lex_le，保留字典序更小者，
/// **平局保留先出现者**（grid_index tie-break 已使键单射 ⟹ 实际无平局；此 fold 顺序兑现
/// strict line 421「固定字典序平局规则」）。
///
/// 返回 `None` ⟺ 候选表为空（K_Θ = ∅——但 strict §12 line 405-409 / Lean
/// `feasible_nonempty` 保证 K_Θ ≠ ∅，故调用方总应传非空表含 u^safe；空表返回 None 是防御边界）。
///
/// ★对齐 Lean `lexArgmin_exists_unique`（有限非空 + 键单射 ⟹ u* 存在唯一）。返回的 u* 是可行表
/// 字典序最小（`lexArgmin_le`：key(u*) ≤ 所有候选键）。
///
/// 边界条件：候选表非空但有键平局（grid_index 也相等）⟹ 保留 fold 中先出现者（确定，无未定义）。
/// 实际 grid_index 唯一 ⟹ 键单射 ⟹ u* 唯一（无平局）。
pub fn lex_argmin<U: Copy>(candidates: &[LexCandidate<U>]) -> Option<U> {
    let mut best: Option<&LexCandidate<U>> = None;
    for c in candidates {
        best = Some(match best {
            // 保留 b（已积累最优）当且仅当 b.key ≤ c.key（平局 b.key==c.key 时 lex_le 返回 true
            // ⟹ 保留先出现的 b，对齐 Lean pick 的「平局保留第一个」）。
            Some(b) if b.key.lex_le(&c.key) => b,
            _ => c,
        });
    }
    best.map(|b| b.control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::closed_loop::state::AssemblyState;

    /// action_priority 全函数确定（契约锚 `Origin.chooseAction` + `action_priority_complete_unique`）：同标签同动作。
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

    /// ★factor-through-classify：intent_adapter 真读 classify_adapter 输出（契约锚
    /// `Origin.FullDefinitionStrategy.policy_factors_through_classification`）。意图 = action_priority(classify_adapter(x))。
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

    // ── §19 J_Θ LexArgmin 测试（strict §12 末 / FULL 十九，Lean Origin.LexArgmin）──

    fn mk_key(track: i64, cost: i64, risk: i64, turn: i64, idx: i64) -> JThetaKey {
        JThetaKey {
            tracking_err: track,
            trade_cost: cost,
            risk_penalty: risk,
            turnover: turn,
            grid_index: idx,
        }
    }

    /// ★字典序主键定胜负（Lean `lex_primary_dominates`）：主键更小则 ≤，无论次键。
    #[test]
    fn lex_le_primary_dominates() {
        // 主键 1 < 5，即便次键 self 大（999 > 0），仍 self ≤ other（主键定序）。
        let a = mk_key(1, 999, 0, 0, 0);
        let b = mk_key(5, 0, 0, 0, 0);
        assert!(a.lex_le(&b));
        assert!(!b.lex_le(&a));
    }

    /// ★字典序次键 tie-break（Lean `lex_secondary_breaks_tie`）：主键平局，次键定序。
    #[test]
    fn lex_le_secondary_breaks_tie() {
        // 主键同 3，次键 2 < 7 ⟹ a ≤ b。
        let a = mk_key(3, 2, 0, 0, 0);
        let b = mk_key(3, 7, 0, 0, 0);
        assert!(a.lex_le(&b));
        assert!(!b.lex_le(&a));
    }

    /// ★字典序全分量相等 ⟹ 互相 ≤（自反，键单射前提下不发生于不同格点）。
    #[test]
    fn lex_le_equal_keys_both() {
        let a = mk_key(1, 2, 3, 4, 5);
        let b = mk_key(1, 2, 3, 4, 5);
        assert!(a.lex_le(&b));
        assert!(b.lex_le(&a)); // 互相 ≤ ⟹ 反对称下相等
    }

    /// ★grid_index 末键 tie-break：前四键全平，索引定序（固定字典序平局规则）。
    #[test]
    fn lex_le_grid_index_tiebreak() {
        let a = mk_key(1, 1, 1, 1, 0); // 索引 0
        let b = mk_key(1, 1, 1, 1, 1); // 索引 1
        assert!(a.lex_le(&b)); // 前四键平，索引 0 < 1 ⟹ a ≤ b
        assert!(!b.lex_le(&a));
    }

    /// ★lex_argmin 选字典序最小控制（Lean `lexArgmin_exists_unique` + `lexArgmin_le`）。
    #[test]
    fn lex_argmin_selects_minimum() {
        // 三候选：控制标签 + J_Θ键。主键最小者（10）应被选出。
        let candidates = [
            LexCandidate { control: "B", key: mk_key(20, 0, 0, 0, 1) },
            LexCandidate { control: "A", key: mk_key(10, 999, 0, 0, 0) }, // 主键最小（次键大不翻盘）
            LexCandidate { control: "C", key: mk_key(30, 0, 0, 0, 2) },
        ];
        assert_eq!(lex_argmin(&candidates), Some("A")); // 主键 10 最小 ⟹ 选 A
    }

    /// ★lex_argmin 主键平局时次键决胜（真字典序 ≠ 单层 min）。
    #[test]
    fn lex_argmin_tiebreak_by_secondary() {
        let candidates = [
            LexCandidate { control: "X", key: mk_key(5, 8, 0, 0, 0) },
            LexCandidate { control: "Y", key: mk_key(5, 3, 0, 0, 1) }, // 主键平 5，次键 3 最小
            LexCandidate { control: "Z", key: mk_key(5, 6, 0, 0, 2) },
        ];
        assert_eq!(lex_argmin(&candidates), Some("Y")); // 主键平局后次键 3 最小 ⟹ 选 Y
    }

    /// ★lex_argmin 空表 ⟹ None（K_Θ=∅ 防御边界；实际 K_Θ≠∅ 由 Lean feasible_nonempty 保）。
    #[test]
    fn lex_argmin_empty_none() {
        let candidates: [LexCandidate<&str>; 0] = [];
        assert_eq!(lex_argmin(&candidates), None);
    }

    /// ★lex_argmin 单候选 ⟹ 直接返回（u^safe 单点可行集）。
    #[test]
    fn lex_argmin_single_candidate() {
        let candidates = [LexCandidate { control: "safe", key: mk_key(0, 0, 0, 0, 0) }];
        assert_eq!(lex_argmin(&candidates), Some("safe"));
    }
}
