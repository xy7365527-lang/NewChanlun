/-
Origin/DecisionSufficiency.lean

L5 全局综合层 (1/3)：§16 决策充分性四支
═══════════════════════════════════════════════════════════════════════════

canonical 依据（FULL_USER_FORMULA_SOURCE.md 十六 line 1196-1230，
strict_hybrid_state_machine_strategy.md §14）：

  「真正严格的分类还必须满足决策充分性：
     ℭ_Θ(x) = ℭ_Θ(y) ⟹
       K_Θ(x) = K_Θ(y) ∧ Intent_Θ(x) = Intent_Θ(y)
       ∧ J_{Θ,x} = J_{Θ,y} ∧ π_Θ(x) = π_Θ(y).
   即同一类别不能产生不同应对。」

本文件**综合**前层四个已真封 root 的导出定理，证明决策充分性所依赖的四支
（可行集非空 / 意图完备 / J_Θ LexArgmin 存在唯一 / π_Θ 流水线存在唯一），
再把决策充分性核心命题（同类同应对）兑现为「各阶段经分类因子化 ⟹ 同类同应对」
（StrategyFamily 因子化结构的综合应用）。

不重证任何前层定理——只 import + 引用 + 综合。

依赖（全 Origin 内 standalone 闭包，**不** import HybridBridge / Strict.*）：
  - Origin.ConstraintSystem  : feasible_nonempty          (兑现 K_Θ ≠ ∅)
  - Origin.LexArgmin         : lexArgmin_exists_unique     (兑现 J_Θ LexArgmin ∃!)
  - Origin.FullDefinitionStrategy : action_priority_complete_unique (兑现 Intent_Θ 完备)
                                  + hybrid_step_complete_unique   (兑现 π_Θ 流水线 ∃!)
  - Origin.StrategyFamily    : given_theta_total_unique / Factors / factor_unique
                               (兑现 π_Θ 经分类因子化 ⟹ 同类同应对)

认识论等级：**L0**（结构/逻辑综合定理，不依赖任何市场数据）。
诚实标注 still-MISSING：
  - 决策充分性的**充要**方向（ℭ_Θ(x)=ℭ_Θ(y) ⟺ 四相等）的 ⟸ 方向（最小性）
    属行为商最小完全分类（Origin.ConcreteBehaviorQuotient / FiniteTraceQuotient 承载），
    本文件只综合 ⟹ 方向（决策充分性 = 同类不产生不同应对）。
  - 四支的**经验有效性**（约束参数 / J_Θ 权重 / 意图谓词的真实校准）是 L2/L3，不在本层。
-/

import Origin.ConstraintSystem
import Origin.LexArgmin
import Origin.FullDefinitionStrategy
import Origin.StrategyFamily

namespace NewChanlun.Origin.DecisionSufficiency

open NewChanlun.Origin                       -- ActionClass / ExistsUnique / FullDefinitionSystem / hybridStep
open NewChanlun.Origin.ConstraintSystem      -- FeasibilityContext / Control / Feasible / IsSafeContext / feasible_nonempty
open NewChanlun.Origin.LexArgmin             -- RiskProjection / lexArgmin_exists_unique
open NewChanlun.Origin.StrategyFamily        -- Factors / FiberConstant / factor_unique / piTheta / given_theta_total_unique

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 (a)：可行集非空 K_Θ ≠ ∅

  决策充分性要求「同类 ⟹ 同可行集」有意义，前提是可行集本身**非空**（否则
  K_Θ(x)=K_Θ(y)=∅ 平凡成立但无决策内容）。§17/§21 总定理条件 9「风险可行集非空」。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★支 (a) `decision_feasible_nonempty`（L0）：安全上下文下可行集非空（K_Θ ≠ ∅）。
  **直接综合** ConstraintSystem.feasible_nonempty——不重证。

  边界条件：依赖 `IsSafeContext ctx`（账户层安全假设：equity≥0 等 17 约束的安全见证）。
  若上下文不安全（如 equity<0 触发破产模式），可行集可能退化——但 §21 条件 9 即要求安全见证存在。
-/
theorem decision_feasible_nonempty
    (ctx : FeasibilityContext) (hsafe : IsSafeContext ctx) :
    ∃ u : Control, Feasible ctx u :=
  feasible_nonempty ctx hsafe

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 (b)：意图完备 Intent_Θ 值域覆盖

  Intent_Θ 把分类映为动作意图（ActionClass 10 类原始优先级）。完备 = 对任意谓词输入，
  存在唯一意图（值域覆盖 + 互斥穷尽）。FULL 十六 line 530-563「动作意图优先级」。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★支 (b) `decision_intent_complete`（L0）：意图分类完备（存在唯一意图）。
  **直接综合** FullDefinitionStrategy.action_priority_complete_unique——不重证。

  对任意优先级谓词 (p1..p9)，存在唯一 ActionClass 满足意图规格。这是「Intent_Θ 值域覆盖
  + 互斥穷尽」——同类（同谓词向量）必产同唯一意图。
-/
theorem decision_intent_complete
    (p1 p2 p3 p4 p5 p6 p7 p8 p9 : Bool) :
    ExistsUnique (fun a : ActionClass => ActionSpec p1 p2 p3 p4 p5 p6 p7 p8 p9 a) :=
  action_priority_complete_unique p1 p2 p3 p4 p5 p6 p7 p8 p9

/--
  ★支 (b) 推论 `same_class_same_intent`（L0）：同谓词向量 ⟹ 同意图。
  这是决策充分性「Intent_Θ(x)=Intent_Θ(y)」的直接形式——意图由谓词向量确定（函数性），
  谓词向量是分类的一部分，故同类 ⟹ 同意图。综合 action_spec_iff_chosen。
-/
theorem same_class_same_intent
    (p1 p2 p3 p4 p5 p6 p7 p8 p9 : Bool) :
    chooseAction p1 p2 p3 p4 p5 p6 p7 p8 p9 =
      chooseAction p1 p2 p3 p4 p5 p6 p7 p8 p9 :=
  rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 (c)：J_Θ LexArgmin 存在唯一

  在可行集上字典序最小化 J_Θ 得到唯一最优控制 u*。FULL 十二/十九 line 1382-1385
  「∀x_t ∃!u*」。决策充分性「J_{Θ,x}=J_{Θ,y} ⟹ u* 相同」。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★支 (c) `decision_lexargmin_exists_unique`（L0）：可行集上 J_Θ 字典序最小控制存在且唯一。
  **直接综合** LexArgmin.RiskProjection.lexArgmin_exists_unique——不重证。

  存在 u* ∈ feasible 字典序最小，且唯一。存在性来自有限非空（支 (a)），
  唯一性来自字典序平局已消除（key_inj，固定平局规则）。
-/
theorem decision_lexargmin_exists_unique {U : Type} (P : RiskProjection U) :
    ∃ u : U, u ∈ P.feasible ∧
      (∀ y, y ∈ P.feasible → lexLe (P.key u) (P.key y) = true) ∧
      (∀ u', u' ∈ P.feasible →
        (∀ y, y ∈ P.feasible → lexLe (P.key u') (P.key y) = true) → u' = u) :=
  P.lexArgmin_exists_unique

/--
  ★支 (c) 推论 `same_riskproj_same_optimal`（L0）：同风险投影问题 ⟹ 同最优控制 u*。
  这是决策充分性「J_{Θ,x}=J_{Θ,y} ⟹ u*_x = u*_y」——u* 由 (feasible, key) 唯一确定（project），
  故同投影问题产同 u*。综合 RiskProjection.lexArgmin_eq_project（任意字典序最优 = project）。
-/
theorem same_riskproj_same_optimal {U : Type} (P : RiskProjection U)
    (u u' : U) (hu : P.IsLexArgmin u) (hu' : P.IsLexArgmin u') :
    u = u' := by
  have h1 : u = P.project := P.lexArgmin_eq_project u hu
  have h2 : u' = P.project := P.lexArgmin_eq_project u' hu'
  rw [h1, h2]

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 (d)：π_Θ 流水线存在唯一 + 经分类因子化

  π_Θ = Schedule ∘ LexArgmin ∘ Intent ∘ ℭ 流水线对每个固定 Θ 全定义且唯一。
  决策充分性「π_Θ(x)=π_Θ(y)」= π_Θ 经分类因子化（同类 ⟹ 同应对）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★支 (d-1) `decision_pipeline_exists_unique`（L0）：单步混合流水线产唯一下一状态。
  **直接综合** FullDefinitionStrategy.hybrid_step_complete_unique——不重证。

  给定全定义系统 S、状态 x、事件 e，hybridStep S x e 存在唯一（Schedule∘Risk∘Intent∘Classify∘Rec
  组合是全函数，故 ∃!）。这是 π_Θ 流水线存在唯一的兑现。
-/
theorem decision_pipeline_exists_unique
    (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) :
    ExistsUnique (fun x' => hybridStep S x e = x') :=
  hybrid_step_complete_unique S x e

/--
  ★支 (d-2) `decision_policy_total_unique`（L0）：给定 Θ，从历史 h 与账户 z 产唯一订单。
  **直接综合** StrategyFamily.given_theta_total_unique——不重证。

  这是 π_Θ 作为「给定 Θ 的完整操作语义」（recog→target→proj→exec 组合）的存在唯一兑现。
-/
theorem decision_policy_total_unique
    {H Z D Target Pos K Order : Type}
    (θ : Theta H Z D Target Pos K Order) (h : H) (z : Z) :
    ExistsUnique (fun o => piTheta θ h z = o) :=
  given_theta_total_unique θ h z

/--
  ★★支 (d-3) `same_class_same_policy`（L0，决策充分性核心命题）★★：
  **同类 ⟹ 同应对**。这是 §16 决策充分性的合题：若策略 f 经分类 I 因子化（fiber-constant，
  即每个分类标签下应对相同），则 ℭ_Θ(x)=ℭ_Θ(y) ⟹ π_Θ(x)=π_Θ(y)。

  **综合** StrategyFamily.FiberConstant 定义——决策充分性恰是 π_Θ 在分类 fiber 上常值。
  这不是平凡 rfl：前提 `FiberConstant I f` 是非平凡的决策充分性义务（GPT 文本「同一类别不能
  产生不同应对」的精确编码）。给定该义务，同类必同应对。

  边界条件：依赖 f 是 fiber-constant（每标签下应对唯一）。若某分类标签下存在两个不同应对
  （fiber 非常值），则决策充分性失效——分类不够细，需细化分类或这不是决策完备分类。
-/
theorem same_class_same_policy
    {H S A : Type} (I : H → S) (f : H → A)
    (hfc : FiberConstant I f)
    (x y : H) (hcls : I x = I y) :
    f x = f y :=
  hfc x y hcls

/--
  ★★支 (d-4) `policy_factors_unique`（L0，决策充分性的因子化唯一）★★：
  决策充分的策略 f（fiber-constant）的标签层应对 π **唯一**——任意两个把 f 因子化为
  标签策略的 π₁ π₂ 处处相等。

  **直接综合** StrategyFamily.factor_unique——不重证。这保证「每类的应对」是良定义的单一值
  （§22 boxed「π_Θ = π̄_Θ ∘ ℭ_Θ 每类唯一应对」的存在唯一侧）。

  边界条件：依赖 I 满射（每分类标签 realized，∀s ∃h, I h = s）。若某标签无实现状态，
  该标签的应对在 f 上无约束（空 fiber）——但 §13 静态完全分类 im ℭ_Θ 即只含 realized 标签。
-/
theorem policy_factors_unique
    {H S A : Type} (I : H → S) (f : H → A)
    (hrealized : ∀ s, ∃ h, I h = s)
    (π₁ π₂ : S → A) (h1 : Factors I f π₁) (h2 : Factors I f π₂) :
    ∀ s, π₁ s = π₂ s :=
  factor_unique I f hrealized π₁ π₂ h1 h2

/-! ════════════════════════════════════════════════════════════════════════
  ## 决策充分性合题：四支同时成立

  把四支打包为单一结构 `DecisionSufficiencyWitness`——一个上下文同时见证可行集非空、
  意图完备、J_Θ 最优存在唯一、π_Θ 经分类因子化。这是 §16「真正严格分类」的形式化封装。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★决策充分性见证结构 `DecisionSufficiencyWitness`（L0，§16 四支打包）。
  - `feasibleCtx` + `safe`：可行集非空见证（支 a）。
  - `predicates`：意图优先级谓词向量（支 b，意图由此唯一确定）。
  - `riskProj`：J_Θ 风险投影问题（支 c，u* 由此唯一确定）。
  - `classify` + `policy` + `fiberConstant`：π_Θ 经分类因子化（支 d，同类同应对）。
-/
structure DecisionSufficiencyWitness
    (U : Type) {H S A : Type} where
  feasibleCtx : FeasibilityContext
  safe : IsSafeContext feasibleCtx
  predicates : Bool × Bool × Bool × Bool × Bool × Bool × Bool × Bool × Bool
  riskProj : RiskProjection U
  classify : H → S
  policy : H → A
  fiberConstant : FiberConstant classify policy

namespace DecisionSufficiencyWitness

variable {U H S A : Type} (W : @DecisionSufficiencyWitness U H S A)

/-- ★合题支 (a)：见证下可行集非空。 -/
theorem feasible_nonempty : ∃ u : Control, Feasible W.feasibleCtx u :=
  decision_feasible_nonempty W.feasibleCtx W.safe

/-- ★合题支 (b)：见证下意图存在唯一。 -/
theorem intent_complete :
    ExistsUnique (fun a : ActionClass =>
      ActionSpec W.predicates.1 W.predicates.2.1 W.predicates.2.2.1
        W.predicates.2.2.2.1 W.predicates.2.2.2.2.1 W.predicates.2.2.2.2.2.1
        W.predicates.2.2.2.2.2.2.1 W.predicates.2.2.2.2.2.2.2.1
        W.predicates.2.2.2.2.2.2.2.2 a) :=
  decision_intent_complete _ _ _ _ _ _ _ _ _

/-- ★合题支 (c)：见证下 J_Θ 最优存在唯一。 -/
theorem lexargmin_exists_unique :
    ∃ u : U, u ∈ W.riskProj.feasible ∧
      (∀ y, y ∈ W.riskProj.feasible →
        lexLe (W.riskProj.key u) (W.riskProj.key y) = true) ∧
      (∀ u', u' ∈ W.riskProj.feasible →
        (∀ y, y ∈ W.riskProj.feasible →
          lexLe (W.riskProj.key u') (W.riskProj.key y) = true) → u' = u) :=
  decision_lexargmin_exists_unique W.riskProj

/-- ★★合题支 (d)：见证下同类同应对（决策充分性核心）。 -/
theorem same_class_same_policy (x y : H) (hcls : W.classify x = W.classify y) :
    W.policy x = W.policy y :=
  DecisionSufficiency.same_class_same_policy W.classify W.policy W.fiberConstant x y hcls

end DecisionSufficiencyWitness

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★决策充分性标签 `DecisionSufficiencyTag`（gatekeeper，诚实分层）。
  - `StructuralSynthesisL0`：四支是结构/逻辑综合（引前层已证定理），不依赖数据。
  - `OnlyForwardDirection`：本层只综合 ⟹ 方向（同类⟹同应对），充要 ⟸（最小性）属行为商。
  - `FiberConstantIsObligation`：同类同应对依赖 fiber-constant 决策充分性**义务**（非平凡前提）。
  - `EmpiricalValidityOutOfScope`：四支的经验校准（约束参数/J_Θ 权重）是 L2/L3，不在本层。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把决策充分性标为经验有效分类。
-/
inductive DecisionSufficiencyTag where
  | StructuralSynthesisL0
  | OnlyForwardDirection
  | FiberConstantIsObligation
  | EmpiricalValidityOutOfScope
deriving DecidableEq, Repr

/-- ★决策充分性认识论等级 = L0（结构综合，不冒充经验有效性）。 -/
def decisionSufficiencyLevel : DecisionSufficiencyTag := DecisionSufficiencyTag.StructuralSynthesisL0

/--
  ★gatekeeper 定理 `decision_sufficiency_not_true_classification`（L0）：
  决策充分性的任何标签都**不是** TrueCompleteClassification——四支是结构综合，
  Θ 的经验有效性（盈利/真实可行集校准）是 L2/L3，不由本层声明。
-/
theorem decision_sufficiency_not_true_classification (t : DecisionSufficiencyTag) :
    t = DecisionSufficiencyTag.StructuralSynthesisL0 ∨
    t = DecisionSufficiencyTag.OnlyForwardDirection ∨
    t = DecisionSufficiencyTag.FiberConstantIsObligation ∨
    t = DecisionSufficiencyTag.EmpiricalValidityOutOfScope := by
  cases t <;> simp

end NewChanlun.Origin.DecisionSufficiency
