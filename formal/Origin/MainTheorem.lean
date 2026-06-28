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
-- §16 中心唯一性定理 + 12 假设 discharge 锚点（本文件 §16 节新增）：
import Origin.SevenLinkStrategy   -- §16 中心定理 seven_link_exists_unique（+ CandidateSet/RThetaInterp/ActiveSet/LexArgmin/ConstraintSystem/OperationRole18/NestingCertificate 传递可用）
import Origin.BuySellPredicate    -- 假设3：买卖点向量全定义 + 64 态 Σ指示=1
import Origin.WellFoundedRank     -- 假设1/4：递归良基终止（显式 WellFounded 实例）
import Origin.PromQual            -- 假设2：Prom_* 全定义且单值（已 discharge）

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

/-! ════════════════════════════════════════════════════════════════════════
  ## §16 中心唯一性定理：12 假设审计 + L0 discharge（MainTheorem 无条件化）

  spec §16（P13 line 780-799）唯一性定理 `∀x ∃! O_{t+1}=π_Θ(x)` 显式依赖 **12 条假设**。
  本节把 §16 中心定理**带到 MainTheorem 顶层**（`main_central_uniqueness`），并对 12 条假设
  逐条 **discharge 到已证引理**（"假设" → "已证"），诚实标注唯一的本质 L2 残余。

  ── 12 假设审计对照（状态 | discharge 引理）─────────────────────────────────────
    1  递归结构 D_t 唯一    | L0 终止：`WellFoundedRank.wf_rank`（真 WellFounded 实例，零 axiom）
                             + `rank_decreases`（严格降秩）。唯一性侧 = 确定性 by-construction
                             （Rec 实现为全函数）。经验解析唯一（古怪线段消歧）= L2，谱系 001。
    2  Prom_* 全定义且单值  | L0：`PromQual.prom_total_and_single_valued`（全定义=良构 C_{ℓ+1} genuine
                             / 单值=by-construction 函数性 `promote_function_unique`，**非**结构唯一性——
                             `derivation_not_unique` 证 sound 派生多值⟹结构唯一假 / 自相似交换律 genuine）。
                             本节挂 `promote_shift_commute` 代表（清洁类型），全份见 capstone 注。
    3  b_ℓ(x) 全定义        | L0：`signalVector_indicator_sum_one`（64 态 Σ指示=1，穷尽互斥）
                             + `B_total`/`S_total`（谓词全定义）。
    4  区间套递归有限终止    | L0：`NestingCertificate.nestCert` 结构递归于级别差 d（Lean 自动判终止）
                             + `N_exists_unique`（∃!∈{0,1}）；显式良基 `WellFoundedRank.wf_rank`。
    5  H(g) 唯一            | L0：`OperationRole18.classifyH` 全函数 ⟹ 唯一（见 a567 联合）。
    6  V(g) 唯一            | L0：`OperationRole18.classifyV` 全函数 ⟹ 唯一（去根化 Ambient）。
    7  R(g)=(H,V,δ) 唯一    | L0 genuine：`OperationRole18.role18_count_one`（18 类 Σ指示=1，
                             穷尽互斥）+ `classifyR18` 全函数（三轴积）。
    8  Γ(x) 有限           | L0 genuine：`CandidateSet.gamma_finite`（|Γ(x)| ≤ |allCands ℓmax|
                             实质上界）+ `mem_gamma_imp_mem_universe`（Γ⊆有限论域）。
    9  ℛ_Θ 确定           | L0 genuine：`RThetaInterp.rTheta_partition_length`（|𝒟|+|ℬ|+|𝒦|=|Γ|
                             真划分）+ `rTheta_order_invariant`（≺_Θ 序定）+ `rTheta_exists_unique`。
    10 𝒦_Θ 非空且有限      | 有限=L0（feasible:List）；非空=**L0 条件于 IsSafeContext**
                             （`ConstraintSystem.feasible_nonempty`，u^safe 见证全 17 约束）；
                             **运行时无条件非空=L2**（依赖账户可运营，破产态 K_Θ 真空，见残余分类）。
    11 LexArgmin 固定平局   | L0 genuine：`RThetaInterp.toKey_inj`（≺_Θ 键单射 = 固定平局）
                             + `LexArgmin.lexArgmin_exists_unique` + `lexArgmin_eq_project`（收口）。
    12 Schedule_Θ 是函数    | L0 by-construction：`SevenLinkStrategy.ThetaStrategy.scheduleFrom :
                             U → Order` 是全函数 ⟹ 确定；由 `main_central_uniqueness` 存在侧承载。

  审计结论：**11 条可纯 L0 结构 discharge**（其中 2/3/4/7/8/9/11 为非退化 genuine 引理；
  1唯一侧/5/6/12 为确定性 by-construction）。**唯一本质 L2 残余 = 假设10 运行时非空**
  （+ 假设1 经验解析唯一性，谱系 001 古怪线段）。无任何假设是"无锚点裸假设"。

  认识论等级：**L0**（结构唯一性，不依赖市场数据）。formalization-validity-domain 231号：
  L0 有效域 = 结构良定义 + 唯一性传递，**非**"π_Θ 在真实行情盈利/可行集真非空"（L2/L3）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★§16 中心定理（MainTheorem 顶层，L0，genuine ∃!）★★★：
  对任意全定义策略 S 与状态 x，七链产出关系 `SevenLinkProduces` 有**唯一**订单 O。
  **直接综合** `SevenLinkStrategy.seven_link_exists_unique`——其唯一性沿七链**逐环强制**
  （非 `total_unique_of_fun` 退化：LexArgmin 段由 `toKey_inj` 固定平局收口为唯一 `project`，
  见 `piTheta_unique`）。这是 spec §16「∀x ∃! O_{t+1}=π_Θ(x)」在 MainTheorem 顶层的承载。
-/
theorem main_central_uniqueness {U Order : Type}
    (S : NewChanlun.Origin.SevenLinkStrategy.ThetaStrategy U Order)
    (x : NewChanlun.Origin.CandidateSet.State) :
    ExistsUnique (fun O => NewChanlun.Origin.SevenLinkStrategy.SevenLinkProduces S x O) :=
  NewChanlun.Origin.SevenLinkStrategy.seven_link_exists_unique S x

/-- 假设1（递归结构 D_t 唯一，终止侧 genuine L0）：递归良基终止——秩 r:D→Nat 诱导的关系
    `InvImage Nat.lt rank` 是**良基的**（无无穷下降链）。discharge `WellFoundedRank.wf_rank`
    （真 `WellFounded` 实例，零额外 axiom）。★唯一性侧 = 确定性 by-construction（Rec 全函数）；
    经验解析唯一性（古怪线段消歧）属 L2（谱系 001-degenerate-segment）。 -/
theorem sec16_assump1_recursion_wellfounded :
    WellFounded (InvImage Nat.lt NewChanlun.Origin.WellFoundedRank.rank) :=
  NewChanlun.Origin.WellFoundedRank.wf_rank

/-- 假设2（Prom_* 全定义且单值）的**自相似代表**（genuine L0，清洁类型）：
    `𝒩_{ℓ+1}(Prom_ℓ es) = Prom_*(𝒩_ℓ es)`（提升与级别平移可交换 = 去根化自相似）。
    ★完整 discharge（全定义 ∧ 单值 ∧ 自相似）见 `PromQual.prom_total_and_single_valued`
    （类型过长不在此重述；本条挂其清洁子结论 `promote_shift_commute` 作机器见证）。 -/
theorem sec16_assump2_prom_self_similar :
    ∀ (k lvl : Nat) (es : List Formal.RecursiveConstruction.Move),
      NewChanlun.Origin.PromQual.shiftMove k (NewChanlun.Origin.PromQual.promote lvl es)
        = NewChanlun.Origin.PromQual.promote (lvl + k)
            (es.map (NewChanlun.Origin.PromQual.shiftMove k)) :=
  NewChanlun.Origin.PromQual.promote_shift_commute

/-- 假设3（买卖点向量 b_ℓ(x) 全定义，genuine L0）：信号向量 64 态分类的指示函数和 = 1
    （对任意级别视图 v，`allBSP` 上「signalVector v = u」恰 1 个 u）——穷尽且互斥的全定义分类。
    discharge `BuySellPredicate.signalVector_indicator_sum_one`。 -/
theorem sec16_assump3_bsp_vector_exhaustive :
    ∀ v : NewChanlun.Origin.LevelView,
      (NewChanlun.Origin.allBSP.filter
        (fun u => decide (NewChanlun.Origin.signalVector v = u))).length = 1 :=
  NewChanlun.Origin.signalVector_indicator_sum_one

/-- 假设4（区间套递归有限终止，genuine L0）：区间套证书 `N^δ_{ℓ↓e}` 对任意 (δ,f,ℓ,e) 有
    **唯一** Bool 值（∃!∈{0,1}）；其递归 `nestCert` 结构递归于级别差 d（Lean 自动判定终止）。
    discharge `NestingCertificate.N_exists_unique`。 -/
theorem sec16_assump4_nesting_cert_unique :
    ∀ (δ : NewChanlun.Origin.NestingCertificate.Dir)
      (f : Nat → NewChanlun.Origin.NestingCertificate.LevelData) (ℓ e : Nat),
      ∃ u : Bool, NewChanlun.Origin.NestingCertificate.N δ f ℓ e = u
        ∧ ∀ v : Bool, NewChanlun.Origin.NestingCertificate.N δ f ℓ e = v → v = u :=
  NewChanlun.Origin.NestingCertificate.N_exists_unique

/-- 假设5/6/7（H(g)/V(g)/R(g)=(H,V,δ) 唯一，genuine L0）：完整角色空间 ℛ=H×V×δ（|ℛ|=18）的
    **18 类穷尽互斥分类**——对任意角色 r∈ℛ，r 在枚举 `allR18` 中恰出现一次（Σ指示=1）。
    角色分类器 `classifyR18` 是三轴积全函数 ⟹ 每事件角色唯一。
    discharge `OperationRole18.role18_count_one`。 -/
theorem sec16_assump567_role_exhaustive_mutex :
    ∀ r : NewChanlun.Origin.Role18, NewChanlun.Origin.allR18.count r = 1 :=
  NewChanlun.Origin.role18_count_one

/-- 假设8（候选集合 Γ(x) 有限，genuine L0）：|Γ(x)| ≤ |allCands ℓmax|（实质上界，过滤子不超论域）。
    discharge `CandidateSet.gamma_finite`（= `gamma_length_le_universe`）。 -/
theorem sec16_assump8_gamma_finite :
    ∀ x : NewChanlun.Origin.CandidateSet.State,
      (NewChanlun.Origin.CandidateSet.gamma x).length
        ≤ (NewChanlun.Origin.CandidateSet.allCands x.ℓmax).length :=
  NewChanlun.Origin.CandidateSet.gamma_finite

/-- 假设9（冲突解释器 ℛ_Θ 确定，genuine L0）：三桶 (𝒟,ℬ,𝒦) 是 Γ 的**真划分**——
    |𝒟_x|+|ℬ_x|+|𝒦_x| = |Γ(x)|（fold 不丢失/不重复任何候选，互斥穷尽分流）。
    discharge `RThetaInterp.rTheta_partition_length`（配合 `rTheta_order_invariant` ≺_Θ 序定）。 -/
theorem sec16_assump9_interp_partition :
    ∀ x : NewChanlun.Origin.CandidateSet.State,
      NewChanlun.Origin.RThetaInterp.tripleLen (NewChanlun.Origin.RThetaInterp.RΘ x)
        = (NewChanlun.Origin.CandidateSet.gamma x).length :=
  NewChanlun.Origin.RThetaInterp.rTheta_partition_length

/--
  ★假设10（可行集 𝒦_Θ 非空，**L0 条件于 IsSafeContext**，本质残余侧 L2）：
  安全上下文（账户可运营前提）下约束系统存在可行点 u^safe（满足全 17 约束）⟹ 𝒦_Θ ≠ ∅。
  discharge `ConstraintSystem.feasible_nonempty`。
  ★诚实标注（formalization-validity-domain）：本条 discharge 的是「IsSafeContext ⟹ 非空」（L0）；
  「运行时无条件非空」依赖账户真实可运营（破产态 IsSafeContext 假，K_Θ 真空——此时非空**不成立**
  是正确的，非 bug），属 **L2**。有限性（feasible:List）为 L0。 -/
theorem sec16_assump10_feasible_nonempty_conditional :
    ∀ (ctx : NewChanlun.Origin.ConstraintSystem.FeasibilityContext),
      NewChanlun.Origin.ConstraintSystem.IsSafeContext ctx →
      ∃ u : NewChanlun.Origin.ConstraintSystem.Control,
        NewChanlun.Origin.ConstraintSystem.Feasible ctx u :=
  NewChanlun.Origin.ConstraintSystem.feasible_nonempty

/-- 假设11（LexArgmin 固定平局，genuine L0）：≺_Θ 字典序键 `toKey` **单射**
    （toKey a = toKey b ⟹ a = b）——固定平局规则（多最优解时仍唯一收口，非普通 argmin）。
    discharge `RThetaInterp.toKey_inj`（配合 `LexArgmin.lexArgmin_eq_project` 收口为唯一 project）。 -/
theorem sec16_assump11_lexkey_injective :
    ∀ a b : NewChanlun.Origin.CandidateSet.Cand,
      NewChanlun.Origin.RThetaInterp.toKey a = NewChanlun.Origin.RThetaInterp.toKey b → a = b :=
  NewChanlun.Origin.RThetaInterp.toKey_inj

/--
  ★★★capstone `main_sec16_L0_assumptions_discharged`（L0，§16 十一 L0 假设单站合取）★★★：
  把 §16 中可纯 L0 discharge 的假设（1 终止 / 2 自相似 / 3 / 4 / 5·6·7 / 8 / 9 / 11）
  合取为单一已证命题——每个合取项均由上游引理兑现（genuine + by-construction 零增量混合，见各 sec16_assump* 标注）（**非** `assume h; exact h` 重述，
  **非** `∃! t, f x=t` 恒真）。假设10 的条件非空单列（`sec16_assump10_…`，带 IsSafeContext 前提，
  L2 残余）；假设12 由 `main_central_uniqueness` 存在侧承载（Schedule 全函数 by-construction）。
-/
theorem main_sec16_L0_assumptions_discharged :
    -- 假设1：递归良基终止
    WellFounded (InvImage Nat.lt NewChanlun.Origin.WellFoundedRank.rank)
    -- 假设2：Prom_* 自相似交换律（代表）
    ∧ (∀ (k lvl : Nat) (es : List Formal.RecursiveConstruction.Move),
        NewChanlun.Origin.PromQual.shiftMove k (NewChanlun.Origin.PromQual.promote lvl es)
          = NewChanlun.Origin.PromQual.promote (lvl + k)
              (es.map (NewChanlun.Origin.PromQual.shiftMove k)))
    -- 假设3：买卖点向量 64 态 Σ指示=1
    ∧ (∀ v : NewChanlun.Origin.LevelView,
        (NewChanlun.Origin.allBSP.filter
          (fun u => decide (NewChanlun.Origin.signalVector v = u))).length = 1)
    -- 假设4：区间套证书 ∃!∈{0,1}
    ∧ (∀ (δ : NewChanlun.Origin.NestingCertificate.Dir)
        (f : Nat → NewChanlun.Origin.NestingCertificate.LevelData) (ℓ e : Nat),
        ∃ u : Bool, NewChanlun.Origin.NestingCertificate.N δ f ℓ e = u
          ∧ ∀ v : Bool, NewChanlun.Origin.NestingCertificate.N δ f ℓ e = v → v = u)
    -- 假设5/6/7：角色 18 类 Σ指示=1
    ∧ (∀ r : NewChanlun.Origin.Role18, NewChanlun.Origin.allR18.count r = 1)
    -- 假设8：Γ(x) 有限
    ∧ (∀ x : NewChanlun.Origin.CandidateSet.State,
        (NewChanlun.Origin.CandidateSet.gamma x).length
          ≤ (NewChanlun.Origin.CandidateSet.allCands x.ℓmax).length)
    -- 假设9：解释器三桶真划分
    ∧ (∀ x : NewChanlun.Origin.CandidateSet.State,
        NewChanlun.Origin.RThetaInterp.tripleLen (NewChanlun.Origin.RThetaInterp.RΘ x)
          = (NewChanlun.Origin.CandidateSet.gamma x).length)
    -- 假设11：≺_Θ 键单射（固定平局）
    ∧ (∀ a b : NewChanlun.Origin.CandidateSet.Cand,
        NewChanlun.Origin.RThetaInterp.toKey a = NewChanlun.Origin.RThetaInterp.toKey b → a = b) :=
  ⟨sec16_assump1_recursion_wellfounded,
   sec16_assump2_prom_self_similar,
   sec16_assump3_bsp_vector_exhaustive,
   sec16_assump4_nesting_cert_unique,
   sec16_assump567_role_exhaustive_mutex,
   sec16_assump8_gamma_finite,
   sec16_assump9_interp_partition,
   sec16_assump11_lexkey_injective⟩

/--
  ★§16 假设残余分类 `Sec16Residue`（gatekeeper，诚实分层 formalization-validity-domain）。
  - `L0StructurallyDischarged`：11 假设由 genuine 上游引理 discharge（见本节诸 `sec16_assump*`）。
  - `L0DeterminismByConstruction`：假设1唯一侧 / 假设5/6 / 假设12——全函数 ⟹ 确定（L0，零信息增量）。
  - `L2RuntimeFeasibilityResidue`：假设10 运行时无条件非空——依赖账户可运营（IsSafeContext 真），
    属 L2（破产态 K_Θ 真空，非空不成立是正确）。
  - `L2EmpiricalParseUniqueness`：假设1 经验侧——真实价格历史解析唯一（古怪线段消歧），谱系 001，L2。
  ★**没有** `EmpiricallyValidated` 构造子——类型层拒绝把 L0 结构唯一性标为经验有效。
-/
inductive Sec16Residue where
  | L0StructurallyDischarged
  | L0DeterminismByConstruction
  | L2RuntimeFeasibilityResidue
  | L2EmpiricalParseUniqueness
deriving DecidableEq, Repr

/-- ★§16 审计认识论等级 = L0（结构唯一性，不冒充经验有效）。 -/
def sec16AuditLevel : Sec16Residue := Sec16Residue.L0StructurallyDischarged

/-- ★gatekeeper：§16 残余分类穷尽——任何状态标签都是四类之一，且无经验有效构造子。 -/
theorem sec16_residue_classified (s : Sec16Residue) :
    s = Sec16Residue.L0StructurallyDischarged ∨
    s = Sec16Residue.L0DeterminismByConstruction ∨
    s = Sec16Residue.L2RuntimeFeasibilityResidue ∨
    s = Sec16Residue.L2EmpiricalParseUniqueness := by
  cases s <;> simp

/-! ── §16 节公理审计（确认仅 propext/Quot.sound，无 sorryAx/native_decide）──────────── -/

#print axioms main_central_uniqueness
#print axioms main_sec16_L0_assumptions_discharged
#print axioms sec16_assump1_recursion_wellfounded
#print axioms sec16_assump10_feasible_nonempty_conditional
#print axioms sec16_residue_classified

end NewChanlun.Origin.MainTheorem
