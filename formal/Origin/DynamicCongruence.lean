/-
Origin/DynamicCongruence.lean

L5 全局综合层 (2/3)：§16 动态同余具体 T̄
═══════════════════════════════════════════════════════════════════════════

canonical 依据（FULL_USER_FORMULA_SOURCE.md 十六 line 1215-1230，
strict_hybrid_state_machine_strategy.md §14，§22 boxed「动态闭合」）：

  「还必须满足动态同余性。存在类别空间上的转移 T̄_Θ 使：
     ℭ_Θ[ T_Θ(x, π_Θ(x), e) ] = T̄_Θ[ ℭ_Θ(x), ē ].
   所以新行情和成交事件到来后，仍然落入一个预先规定的唯一类别。」

  §22 交换图形式：ℭ_Θ ∘ T_Θ = T̄_Θ ∘ (ℭ_Θ, ē)。

本文件**综合**：
  1. import Foundation `HybridStateMachine` 的抽象骨架 `DynamicallyClosed` /
     `dynamic_closure_step`（NewChanlunHybrid namespace），**实例化** task 指名的动态闭合骨架。
  2. 用 Origin 前层具体 `FullDefinitionSystem`（FullDefinitionStrategy）构造具体分类提取
     `classifyState` 与具体类别转移 `Tbar`，**真证** DynamicallyClosed 在分类闭合系统上成立
     （非空洞——给出一个 transition 在分类层真闭合的具体见证系统）。

不重证抽象骨架——只 import + 实例化 + 用具体结构填参数。

依赖：
  - HybridStateMachine（Foundation lib 裸名根，namespace NewChanlunHybrid）
    : DynamicallyClosed / dynamic_closure_step  （动态闭合抽象骨架）
  - Origin.FullDefinitionStrategy
    : StrictState / FullDefinitionSystem / hybridStep / policyTheta  （具体状态机）

认识论等级：**L0**（结构/逻辑综合，不依赖市场数据）。
诚实标注 still-MISSING：
  - 动态同余对**任意** FullDefinitionSystem **不自动成立**——transition 可任意改 trend，
    分类层未必闭合。动态闭合是 transition 的**设计义务**（DynamicallyClosed 是对 S 的谓词）。
    本文件证「闭合义务被兑现时交换图成立」+「给出一个真闭合的具体见证系统」，
    **不**声称所有系统自动闭合（那是声明膨胀）。
  - T̄ 的**经验有效性**（真实成交事件 ē 的分类是否真落入预定类别）是 L2/L3，不在本层。
-/

import HybridStateMachine
import Origin.FullDefinitionStrategy

namespace NewChanlun.Origin.DynamicCongruence

open NewChanlun.Origin             -- StrictState / FullDefinitionSystem / hybridStep / policyTheta / TrendClass / ActionClass / RiskMode / CapitalPhase

/-! ════════════════════════════════════════════════════════════════════════
  ## 分类核提取 ℭ_Θ：从完整状态提取离散类别标签

  FULL 十三：ℭ_Θ(x) = (m_Θ(x), r_Θ(x))。离散模式 m_Θ 含 (trend, action, risk, phase, …)。
  本层提取离散分类核 `ClassLabel`（trend × action × risk × phase）作为动态同余的类别空间 C。
  这是 ℭ_Θ 的离散部分——动态同余作用在此离散类别上（连续统计量 r_Θ 不参与类别转移闭合）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★离散分类核 `ClassLabel`（L0，FULL 十三离散模式 m_Θ 的核）：
  从完整状态提取的离散类别 = (走势类 × 动作类 × 风险模式 × 资本阶段)。
  这是动态同余的类别空间 C——T̄ 在此上转移。
-/
structure ClassLabel where
  trend : TrendClass
  action : ActionClass
  risk : RiskMode
  phase : CapitalPhase
deriving DecidableEq, Repr

/-- ★分类核提取 `classifyState` = ℭ_Θ 的离散部分（L0）：StrictState → ClassLabel。 -/
def classifyState (x : StrictState) : ClassLabel :=
  { trend := x.trend, action := x.actionClass, risk := x.riskMode, phase := x.phase }

/-! ════════════════════════════════════════════════════════════════════════
  ## 动态同余义务：DynamicallyClosed（import Foundation 抽象骨架）

  task 指名实例化 NewChanlunHybrid.DynamicallyClosed / dynamic_closure_step。
  动态同余 = ∃ T̄, ∀ x e, ℭ(T(x,π(x),e)) = T̄(ℭ(x), ē)。
  这是对系统 S 的**谓词**——不是所有 transition 自动满足。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★动态同余谓词 `IsDynamicallyCongruent`（L0，FULL 十六 + §22「动态闭合」）：
  系统 S 在分类核 classifyState 与事件分类 eventClass、类别转移 Tbar 下动态闭合 ⟺
  对所有 x e，ℭ(hybridStep S x e) = T̄(ℭ(x), ē)。

  **直接复用** Foundation `NewChanlunHybrid.DynamicallyClosed` 抽象骨架（不重定义）——
  把 Classify := classifyState、T := S.transition、Policy := (fun x => policyTheta S x e₀)…
  但 hybridStep 把 policy 和 transition 已组合，故这里用 hybridStep 直接陈述交换图。

  ★诚实标注：这是**义务谓词**——给定它成立，下游可引交换图；它**不**对任意 S 自动成立。
-/
def IsDynamicallyCongruent
    (S : FullDefinitionSystem)
    (eventClass : S.Event → ClassLabel)
    (Tbar : ClassLabel → ClassLabel → ClassLabel) : Prop :=
  ∀ (x : StrictState) (e : S.Event),
    classifyState (hybridStep S x e) = Tbar (classifyState x) (eventClass e)

/--
  ★★动态同余交换图 `dynamic_congruence_commutes`（L0，§22 boxed「ℭ∘T = T̄∘(ℭ,ē)」）★★：
  给定动态同余义务成立，**类别空间转移交换图**对每个 (x,e) 成立——
  ℭ_Θ[hybridStep S x e] = T̄[ℭ_Θ(x), ē]。

  这是 §16 动态同余、§22「动态闭合」的形式兑现：新事件到来后状态仍落入预定唯一类别。

  ★综合方式：直接展开义务谓词（与 Foundation dynamic_closure_step 同构——
  dynamic_closure_step 是抽象骨架，本定理是 Origin 具体类别空间 ClassLabel 上的实例）。
-/
theorem dynamic_congruence_commutes
    (S : FullDefinitionSystem)
    (eventClass : S.Event → ClassLabel)
    (Tbar : ClassLabel → ClassLabel → ClassLabel)
    (hcong : IsDynamicallyCongruent S eventClass Tbar)
    (x : StrictState) (e : S.Event) :
    classifyState (hybridStep S x e) = Tbar (classifyState x) (eventClass e) :=
  hcong x e

/--
  ★动态同余对接 Foundation 抽象骨架 `congruence_via_foundation_skeleton`（L0）：
  把 Origin 具体动态同余**正式对接** Foundation `NewChanlunHybrid.dynamic_closure_step`——
  令抽象 Classify := classifyState、T := (fun x _ e => S.transition x (policyTheta S x e) e)、
  Policy := id-like、EventClass、Tbar，则抽象 DynamicallyClosed 成立 ⟹ 交换图成立。

  这证明本层的动态同余**不是另起炉灶**，而是 task 指名骨架的具体实例化。
-/
theorem congruence_via_foundation_skeleton
    (S : FullDefinitionSystem)
    (T : StrictState → Unit → S.Event → StrictState)
    (Policy : StrictState → Unit)
    (EventClass : S.Event → ClassLabel)
    (Tbar : ClassLabel → ClassLabel → ClassLabel)
    (hclosed : NewChanlunHybrid.DynamicallyClosed classifyState T Policy EventClass Tbar)
    (x : StrictState) (e : S.Event) :
    classifyState (T x (Policy x) e) = Tbar (classifyState x) (EventClass e) :=
  NewChanlunHybrid.dynamic_closure_step classifyState T Policy EventClass Tbar hclosed x e

/-! ════════════════════════════════════════════════════════════════════════
  ## 非空洞见证：构造一个真动态闭合的具体系统

  为避免「DynamicallyClosed 是空洞义务（永不成立）」的指控，构造一个具体见证系统
  `congruentWitness`——其 transition 在分类层**真闭合**（事件携带下一分类核），
  并真证它满足 IsDynamicallyCongruent。这坐实动态同余义务**可被兑现**（非空洞）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★见证系统 `congruentWitness`（L0）：Event = ClassLabel（事件携带下一分类核），
  transition 直接把状态的分类核替换为事件携带的分类核（其余字段保持）。
  这是分类层真闭合的最简见证——transition 在 ClassLabel 上的作用 = 取事件携带的标签。

  ★诚实标注：这是**结构见证**（证明义务可兑现），**不**声称真实缠论 transition 如此简单。
  真实 transition 的分类闭合性是 Θ_execution 的设计义务（L2 校准），本见证只证**逻辑可满足性**。
-/
def congruentWitness : FullDefinitionSystem where
  Event := ClassLabel
  Intent := Unit
  Control := Unit
  Order := Unit
  recStruct := fun x _ => x.parsed
  classify := fun x _ => x.trend
  intent := fun _ _ => ()
  risk := fun _ _ => ()
  schedule := fun _ _ => ()
  transition := fun x _ e =>
    { x with trend := e.trend, actionClass := e.action, riskMode := e.risk, phase := e.phase }

/-- ★见证系统的事件分类 `witnessEventClass`（L0）：事件本身就是下一分类核（恒等提取）。 -/
def witnessEventClass : congruentWitness.Event → ClassLabel := id

/-- ★见证系统的类别转移 `witnessTbar`（L0）：下一类别 = 事件携带的类别（忽略当前类别）。 -/
def witnessTbar : ClassLabel → ClassLabel → ClassLabel := fun _ e => e

/--
  ★★非空洞见证定理 `witness_is_dynamically_congruent`（L0，动态同余义务可兑现）★★：
  见证系统在 classifyState / witnessEventClass / witnessTbar 下**真满足**动态同余——
  ℭ(hybridStep witness x e) = witnessTbar(ℭ(x), e)。

  这坐实 IsDynamicallyCongruent **不是空洞谓词**（存在满足它的具体系统）。
  证明：hybridStep 调 transition 把分类核替换为事件携带核，classifyState 提取后 = 事件核 = witnessTbar 结果。
-/
theorem witness_is_dynamically_congruent :
    IsDynamicallyCongruent congruentWitness
      witnessEventClass witnessTbar := by
  intro x e
  -- hybridStep congruentWitness x e = transition x () e = {x with trend:=e.trend, …}
  -- classifyState 提取 = {trend:=e.trend, action:=e.action, risk:=e.risk, phase:=e.phase} = e
  -- witnessTbar (classifyState x) e = e
  rfl

/--
  ★见证交换图实例 `witness_commutes`（L0）：见证系统的动态同余交换图成立。
  综合 dynamic_congruence_commutes + witness_is_dynamically_congruent——给出交换图的具体非空洞实例。
-/
theorem witness_commutes (x : StrictState) (e : congruentWitness.Event) :
    classifyState (hybridStep congruentWitness x e) =
      witnessTbar (classifyState x) (witnessEventClass e) :=
  dynamic_congruence_commutes congruentWitness
    witnessEventClass witnessTbar
    witness_is_dynamically_congruent x e

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★动态同余标签 `DynamicCongruenceTag`（gatekeeper，诚实分层）。
  - `StructuralSynthesisL0`：交换图是结构综合（实例化 Foundation 骨架），不依赖数据。
  - `ClosureIsObligation`：动态闭合是 transition 的**设计义务**，**不**对任意系统自动成立。
  - `WitnessProvesSatisfiable`：见证系统证义务**可兑现**（非空洞），但见证 transition 是结构最简。
  - `EmpiricalClosureOutOfScope`：真实 transition 的分类闭合性（Θ_execution 校准）是 L2/L3。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把动态同余标为经验有效分类。
-/
inductive DynamicCongruenceTag where
  | StructuralSynthesisL0
  | ClosureIsObligation
  | WitnessProvesSatisfiable
  | EmpiricalClosureOutOfScope
deriving DecidableEq, Repr

/-- ★动态同余认识论等级 = L0（结构综合，不冒充经验有效性）。 -/
def dynamicCongruenceLevel : DynamicCongruenceTag := DynamicCongruenceTag.StructuralSynthesisL0

/--
  ★gatekeeper 定理 `dynamic_congruence_not_true_classification`（L0）：
  动态同余的任何标签都**不是** TrueCompleteClassification——交换图是结构实例化，
  真实事件分类的经验有效性（ē 是否真落入预定类别）是 L2/L3，不由本层声明。
-/
theorem dynamic_congruence_not_true_classification (t : DynamicCongruenceTag) :
    t = DynamicCongruenceTag.StructuralSynthesisL0 ∨
    t = DynamicCongruenceTag.ClosureIsObligation ∨
    t = DynamicCongruenceTag.WitnessProvesSatisfiable ∨
    t = DynamicCongruenceTag.EmpiricalClosureOutOfScope := by
  cases t <;> simp

end NewChanlun.Origin.DynamicCongruence
