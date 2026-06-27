/-
Origin/MainTheorem.lean

L5 全局综合层 (3/3)：§21 最终总定理（封顶综合）
═══════════════════════════════════════════════════════════════════════════

canonical 依据（FULL_USER_FORMULA_SOURCE.md 二十一 line 1451-1530，
strict_hybrid_state_machine_strategy.md §17，§22 六 boxed）：

  §21 总定理：在 13 个条件下，
    ∀ x_t ∈ X_Θ, ∀ e_{t+1} ∈ E_Θ(x_t),
    ∃! (D_t, c_t, ũ_{t+1}, u*_{t+1}, O_{t+1}, x_{t+1}) 使
      D_t = Rec_Θ(h_t)
      c_t = ℭ_Θ(x_t, D_t)
      ũ_{t+1} = Intent_Θ(c_t)
      u*_{t+1} = LexArgmin_{u∈K_Θ} J_Θ(u, ũ)
      O_{t+1} = Schedule_Θ(x_t, u*)
      x_{t+1} = T_Θ(x_t, O_{t+1}, e_{t+1}).

  §22 六 boxed（最压缩严格定义）：
    互斥穷尽 / 每类唯一应对 / 动态闭合 / 多空镜像 / 尺度自相似 / 最小行为分类。

本文件**封顶综合**——S_Θ 九元组完整闭环：
  (X_Θ, E_Θ, Rec_Θ, ℭ_Θ, Intent_Θ, K_Θ, J_Θ, Schedule_Θ, T_Θ)。

综合三大支：
  1. **全链镜像等变**（σ→−σ）：综合 FullDefinitionStrategy.hybrid_step_mirror_equivariant
     + Foundation NewChanlunHybrid.classified_policy_mirror_equivariant。
  2. **动态闭合保账本恒等** R=Π−A−W：综合 FullDefinitionStrategy.ledger_invariant_preservation
     + LedgerState.inv（每个合法状态账本恒等 + transition 保持）。
  3. **S_Θ 九元组完整闭环存在唯一**：**实例化** task 指名的
     NewChanlunHybrid.hybrid_step_complete_unique（六阶段 Rec→Class→Intent→Risk→Schedule→T 单步 ∃!）
     + FullDefinitionStrategy.hybrid_step_complete_unique（Origin 具体单步 ∃!）。

不重证任何前层定理——只 import + 引用 + 综合实例化。

依赖：
  - HybridStateMachine（Foundation lib 裸名根，namespace NewChanlunHybrid）
    : hybrid_step_complete_unique（六阶段抽象骨架）/ classified_policy_mirror_equivariant
  - Origin.FullDefinitionStrategy
    : StrictState / FullDefinitionSystem / hybridStep / hybrid_step_complete_unique
      / hybrid_step_mirror_equivariant / ledger_invariant_preservation / LedgerState
  - Origin.DecisionSufficiency  : 四支（K_Θ≠∅/Intent/J/π）
  - Origin.DynamicCongruence    : 动态同余交换图

认识论等级：**L0**（结构/逻辑综合，不依赖市场数据）。
诚实标注 still-MISSING（§18「能证明与不能证明」line 605-619）：
  - 总定理证明的是**形式完备性**：每个合法状态有且只有一个类别/意图/控制/订单/下一状态。
  - **不**证明：盈利 / 必然回本 / 必然达负成本 / 必然增单位。这些是 L2/L3 条件性不变量，
    只在实际成交/成本/市场路径/风险上界等额外假设满足时成立——**不在本层**。
  - 镜像等变 / 动态闭合的**前提义务**（mirrorState 满足等变、transition 满足闭合）是对 Θ 的
    设计义务（L0 逻辑可满足，L2 真实校准）——本层证「义务兑现 ⟹ 结论」，不声称自动兑现。
-/

import HybridStateMachine
import Origin.FullDefinitionStrategy
import Origin.DecisionSufficiency
import Origin.DynamicCongruence

namespace NewChanlun.Origin.MainTheorem

open NewChanlun.Origin             -- StrictState / FullDefinitionSystem / hybridStep / LedgerState / hybrid_step_* / ledger_invariant_preservation / ExistsUnique

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 1：S_Θ 九元组完整闭环存在唯一

  §21 总定理核心：单步混合流水线 Rec→Class→Intent→Risk→Schedule→T 产唯一下一状态。
  实例化 task 指名的 Foundation 抽象骨架 + Origin 具体单步。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★支 1 `main_hybrid_step_exists_unique`（L0，§21 总定理闭环 ∃!）★★：
  给定全定义系统 S、状态 x、事件 e，混合单步产**唯一**下一状态。
  **直接综合** FullDefinitionStrategy.hybrid_step_complete_unique——不重证。

  这是 S_Θ 九元组闭环的存在唯一兑现：x_{t+1} = T_Θ(x_t, π_Θ(x_t), e_{t+1}) 存在且唯一。
-/
theorem main_hybrid_step_exists_unique
    (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) :
    ExistsUnique (fun x' => hybridStep S x e = x') :=
  hybrid_step_complete_unique S x e

/--
  ★★支 1 抽象骨架实例化 `main_pipeline_exists_unique_via_foundation`
  （L0，实例化 task 指名 NewChanlunHybrid.hybrid_step_complete_unique）★★：
  给定六阶段关系 RecR/ClassR/IntentR/RiskR/ScheduleR/TransR 各自存在唯一（六阶段单值义务），
  则完整六阶段流水线输出 (D,c,i,u,o,xnext) **存在唯一**。

  **直接实例化** Foundation NewChanlunHybrid.hybrid_step_complete_unique（六阶段抽象骨架）——
  这正是 §21 总定理 ∃!(D_t, c_t, ũ, u*, O, x_{t+1}) 的抽象形式。本层把它对接为 Origin 顶层定理。

  ★诚实标注：六阶段单值义务（hRec…hTrans）是**前提**——它们由前层各 root 兑现
  （hRec ← 级别递归良基 / hClass ← 走势分类 ∃! / hIntent ← action_priority_complete_unique /
  hRisk ← lexArgmin_exists_unique / hSchedule ← Schedule 全函数 / hTrans ← transition 全函数）。
  本定理证「六义务兑现 ⟹ 闭环 ∃!」，是总定理的**综合骨架**。
-/
theorem main_pipeline_exists_unique_via_foundation
    {D C I U O XNext : Type}
    (RecR : StrictState → D → Prop)
    (ClassR : StrictState → D → C → Prop)
    (IntentR : StrictState → C → I → Prop)
    (RiskR : StrictState → I → U → Prop)
    (ScheduleR : StrictState → U → O → Prop)
    (TransR : StrictState → O → Unit → XNext → Prop)
    (hRec : ∀ x : StrictState, ExistsUnique (fun d : D => RecR x d))
    (hClass : ∀ (x : StrictState) (d : D), ExistsUnique (fun c : C => ClassR x d c))
    (hIntent : ∀ (x : StrictState) (c : C), ExistsUnique (fun i : I => IntentR x c i))
    (hRisk : ∀ (x : StrictState) (i : I), ExistsUnique (fun u : U => RiskR x i u))
    (hSchedule : ∀ (x : StrictState) (u : U), ExistsUnique (fun o : O => ScheduleR x u o))
    (hTrans : ∀ (x : StrictState) (o : O) (e : Unit),
      ExistsUnique (fun xnext : XNext => TransR x o e xnext))
    (x : StrictState) (e : Unit) :
    NewChanlunHybrid.ExistsUnique (fun out =>
      NewChanlunHybrid.HybridStepSpec RecR ClassR IntentR RiskR ScheduleR TransR x e out) :=
  NewChanlunHybrid.hybrid_step_complete_unique
    RecR ClassR IntentR RiskR ScheduleR TransR
    hRec hClass hIntent hRisk hSchedule hTrans x e

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 2：全链镜像等变（σ → −σ）

  §22 boxed「多空镜像」：ℭ_{MΘ} ∘ M_X = M_S ∘ ℭ_Θ。
  全链等变：镜像状态镜像事件的单步 = 单步的镜像。FULL §7 line 192-219。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★支 2 `main_mirror_equivariant`（L0，§22「多空镜像」全链等变）★★：
  给定镜像义务（镜像状态镜像事件的单步 = 单步结果的镜像），全链镜像等变成立——
  hybridStep S (mirrorState x) (mirrorEvent e) = mirrorState (hybridStep S x e)。
  **直接综合** FullDefinitionStrategy.hybrid_step_mirror_equivariant——不重证。

  ★诚实标注：镜像义务 h（∀x e, …）是**前提**——它编码 σ→−σ 全链等变（U↔D / B_i↔S_i /
  +1↔−1）。本定理证「义务兑现 ⟹ 全链等变」，义务真实兑现需 Θ_mirror 满足镜像规则（L0 可满足）。
-/
theorem main_mirror_equivariant
    (S : FullDefinitionSystem)
    (mirrorState : StrictState → StrictState)
    (mirrorEvent : S.Event → S.Event)
    (h : ∀ x e, hybridStep S (mirrorState x) (mirrorEvent e) =
      mirrorState (hybridStep S x e))
    (x : StrictState) (e : S.Event) :
    hybridStep S (mirrorState x) (mirrorEvent e) =
      mirrorState (hybridStep S x e) :=
  hybrid_step_mirror_equivariant S mirrorState mirrorEvent h x e

/--
  ★支 2 抽象骨架对接 `main_classified_mirror_via_foundation`
  （L0，对接 Foundation classified_policy_mirror_equivariant）：
  分类策略 π = policy ∘ classify 的镜像等变——
  classB(M_X x) = M_C(classA x) ∧ policyB(M_C c) = M_U(policyA c) ⟹
  classifiedPolicy classB policyB (M_X x) = M_U(classifiedPolicy classA policyA x)。
  **直接综合** Foundation NewChanlunHybrid.classified_policy_mirror_equivariant——不重证。

  这把镜像等变在「分类 + 应对」的分解层兑现（§22 ℭ_{MΘ}∘M_X = M_S∘ℭ_Θ 的策略侧）。
-/
theorem main_classified_mirror_via_foundation
    {C U : Type}
    (classA classB : StrictState → C) (policyA policyB : C → U)
    (MX : StrictState → StrictState) (MC : C → C) (MU : U → U)
    (hClass : ∀ x, classB (MX x) = MC (classA x))
    (hPolicy : ∀ c, policyB (MC c) = MU (policyA c))
    (x : StrictState) :
    NewChanlunHybrid.classifiedPolicy classB policyB (MX x) =
      MU (NewChanlunHybrid.classifiedPolicy classA policyA x) :=
  NewChanlunHybrid.classified_policy_mirror_equivariant
    classA classB policyA policyB MX MC MU hClass hPolicy x

/-! ════════════════════════════════════════════════════════════════════════
  ## 支 3：动态闭合保账本恒等 R = Π − A − W

  §3 line 82-88 / §10 line 290-294 / §17 条件 8「资本记账守恒」：
  R_t = Π_t − A_t − W_t 是合法状态条件，T_Θ 必须保持它（除非外部事件明确改账目）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★支 3 `main_ledger_invariant`（L0，§17 条件 8「资本记账守恒」）★★：
  每个合法账本状态满足恒等 R = Π − A − W。
  **直接综合** LedgerState.inv（账本恒等是结构不变量，构造时即保证）。
-/
theorem main_ledger_invariant (L : LedgerState) :
    L.R = L.Pi - L.A - L.W :=
  L.inv

/--
  ★★支 3 `main_ledger_step_preserves_invariant`（L0，§17 条件 8 + §22「动态闭合」账本侧）★★：
  账本转移（任意 ΔΠ/ΔA/ΔW）**保持**恒等 R = Π − A − W。
  **直接综合** FullDefinitionStrategy.ledger_invariant_preservation——不重证。

  这是动态闭合的账本侧：T_Θ 在账本上的作用保持核心恒等（除非外部事件明确改账目）。
-/
theorem main_ledger_step_preserves_invariant (L : LedgerState) (dPi dA dW : Int) :
    (ledgerStep L dPi dA dW).R =
      (ledgerStep L dPi dA dW).Pi -
      (ledgerStep L dPi dA dW).A -
      (ledgerStep L dPi dA dW).W :=
  ledger_invariant_preservation L dPi dA dW

/-! ════════════════════════════════════════════════════════════════════════
  ## 总定理合题：S_Θ 九元组完整闭环

  把三大支 + 决策充分性四支 + 动态同余打包为单一总定理结构 `MainTheoremClosure`——
  一个系统同时见证：闭环存在唯一 + 全链镜像等变 + 账本守恒 + 决策充分 + 动态同余。
  这是 §21 总定理 + §22 六 boxed 的形式化封顶。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★总定理闭环结构 `MainTheoremClosure`（L0，§21 + §22 封顶综合）★★：
  S_Θ 九元组完整闭环的见证——一个系统 S 配齐镜像/账本/动态闭合义务。
  - `system`：全定义系统 S（承载 Rec_Θ/ℭ_Θ/Intent_Θ/Schedule_Θ/T_Θ）。
  - `mirrorState` / `mirrorEvent` / `mirrorObligation`：全链镜像等变义务（§22 多空镜像）。
  - `eventClass` / `Tbar` / `congruenceObligation`：动态同余义务（§22 动态闭合）。

  ★诚实标注：义务（mirrorObligation / congruenceObligation）是**前提**——它们由 Θ 设计兑现
  （L0 逻辑可满足，DynamicCongruence.witness 证可兑现非空洞）。本结构封装「义务齐备的系统」。
-/
structure MainTheoremClosure where
  system : FullDefinitionSystem
  mirrorState : StrictState → StrictState
  mirrorEvent : system.Event → system.Event
  mirrorObligation : ∀ x e,
    hybridStep system (mirrorState x) (mirrorEvent e) =
      mirrorState (hybridStep system x e)
  eventClass : system.Event → DynamicCongruence.ClassLabel
  Tbar : DynamicCongruence.ClassLabel → DynamicCongruence.ClassLabel → DynamicCongruence.ClassLabel
  congruenceObligation :
    DynamicCongruence.IsDynamicallyCongruent system eventClass Tbar

namespace MainTheoremClosure

variable (M : MainTheoremClosure)

/-- ★★总定理支 1：闭环存在唯一（§21 ∃! 下一状态）。 -/
theorem step_exists_unique (x : StrictState) (e : M.system.Event) :
    ExistsUnique (fun x' => hybridStep M.system x e = x') :=
  main_hybrid_step_exists_unique M.system x e

/-- ★★总定理支 2：全链镜像等变（§22 多空镜像）。 -/
theorem mirror_equivariant (x : StrictState) (e : M.system.Event) :
    hybridStep M.system (M.mirrorState x) (M.mirrorEvent e) =
      M.mirrorState (hybridStep M.system x e) :=
  main_mirror_equivariant M.system M.mirrorState M.mirrorEvent M.mirrorObligation x e

/-- ★★总定理支 3：动态闭合交换图（§22 动态闭合，ℭ∘T = T̄∘(ℭ,ē)）。 -/
theorem dynamic_closure (x : StrictState) (e : M.system.Event) :
    DynamicCongruence.classifyState (hybridStep M.system x e) =
      M.Tbar (DynamicCongruence.classifyState x) (M.eventClass e) :=
  DynamicCongruence.dynamic_congruence_commutes M.system M.eventClass M.Tbar
    M.congruenceObligation x e

/--
  ★★★总定理 `main_theorem`（L0，§21 + §22 封顶）★★★：
  S_Θ 九元组闭环系统同时满足：
    (1) 闭环存在唯一（§21）；
    (2) 全链镜像等变（§22 多空镜像）；
    (3) 动态闭合交换图（§22 动态闭合）。
  对所有 (x, e) 三者同时成立——这是「自相似、镜像等变、良基递归、动态闭合的确定性混合控制系统」
  的形式化封顶（FULL §22 line 1601-1610 最终最严格数学名称）。
-/
theorem main_theorem (x : StrictState) (e : M.system.Event) :
    (ExistsUnique (fun x' => hybridStep M.system x e = x')) ∧
    (hybridStep M.system (M.mirrorState x) (M.mirrorEvent e) =
      M.mirrorState (hybridStep M.system x e)) ∧
    (DynamicCongruence.classifyState (hybridStep M.system x e) =
      M.Tbar (DynamicCongruence.classifyState x) (M.eventClass e)) :=
  ⟨M.step_exists_unique x e, M.mirror_equivariant x e, M.dynamic_closure x e⟩

end MainTheoremClosure

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 能证明与不能证明（诚实分层，FULL line 605-619）

  类型层把「形式完备性」与「盈利保证」分离——总定理证前者，**拒绝**声明后者。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★总定理标签 `MainTheoremTag`（gatekeeper，§18 诚实分层）。
  - `FormalCompletenessL0`：总定理证「每个合法状态唯一类别/意图/控制/订单/下一状态」（§18 能证明）。
  - `ObligationsArePremises`：镜像/账本/动态闭合义务是前提（Θ 设计兑现，L0 可满足 L2 校准）。
  - `NoProfitGuarantee`：**不**证盈利/必然回本/必然负成本/必然增单位（§18 不能证明，L2/L3）。
  - `EmpiricalValidityOutOfScope`：条件性不变量需实际成交/成本/市场路径假设（L2/L3，不在本层）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把形式完备标为经验有效。
-/
inductive MainTheoremTag where
  | FormalCompletenessL0
  | ObligationsArePremises
  | NoProfitGuarantee
  | EmpiricalValidityOutOfScope
deriving DecidableEq, Repr

/-- ★总定理认识论等级 = L0（形式完备综合，不冒充盈利保证）。 -/
def mainTheoremLevel : MainTheoremTag := MainTheoremTag.FormalCompletenessL0

/--
  ★★gatekeeper 定理 `main_theorem_proves_no_profit`（L0，§18 能证明与不能证明的边界）★★：
  总定理的任何标签都**不是** TrueCompleteClassification——它证形式完备性
  （唯一类别/意图/控制/订单/下一状态），**不**证盈利/回本/负成本/增单位（那是 L2/L3 条件性不变量）。
  这是 §18「不能仅凭形式完备性证明盈利」的类型层兑现。
-/
theorem main_theorem_proves_no_profit (t : MainTheoremTag) :
    t = MainTheoremTag.FormalCompletenessL0 ∨
    t = MainTheoremTag.ObligationsArePremises ∨
    t = MainTheoremTag.NoProfitGuarantee ∨
    t = MainTheoremTag.EmpiricalValidityOutOfScope := by
  cases t <;> simp

end NewChanlun.Origin.MainTheorem
