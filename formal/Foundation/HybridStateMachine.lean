/-!
NewChanlun strict hybrid state machine skeleton.

This file formalizes the mathematical core of the upgraded specification:

* priority partitions for overlapped action predicates;
* deterministic capital phase and margin/risk mode classifiers;
* behavior equivalence as an equivalence relation;
* "minimal complete classification" as equality with behavior equivalence;
* mirror equivariance of a classified policy;
* unique one-step output of the Rec -> Class -> Intent -> Risk -> Schedule -> T
  pipeline, under total single-valued stage obligations.

The market semantics, Chanlun parsing rules, costs, margin formulas, and
execution details are parameters. The proof states the exact obligations that
an implementation must discharge.
-/

namespace NewChanlunHybrid

universe u

def ExistsUnique {A : Type u} (p : A -> Prop) : Prop :=
  Exists (fun x => p x /\ forall y, p y -> y = x)

/-! ## Generic behavior equivalence -/

section Behavior

variable {X Omega TraceOut : Type}

def BehEquiv (Trace : X -> Omega -> TraceOut) (x y : X) : Prop :=
  forall omega : Omega, Trace x omega = Trace y omega

theorem beh_refl (Trace : X -> Omega -> TraceOut) (x : X) :
    BehEquiv Trace x x := by
  intro omega
  rfl

theorem beh_symm (Trace : X -> Omega -> TraceOut) {x y : X} :
    BehEquiv Trace x y -> BehEquiv Trace y x := by
  intro h omega
  exact (h omega).symm

theorem beh_trans (Trace : X -> Omega -> TraceOut) {x y z : X} :
    BehEquiv Trace x y -> BehEquiv Trace y z -> BehEquiv Trace x z := by
  intro hxy hyz omega
  exact (hxy omega).trans (hyz omega)

/--
  行为极小完全分类（`Classify x = Classify y ↔ BehEquiv Trace x y`）。

  ★★降级标注（task #91, 共有缺口3, 615/231 谱系）——**可选元理论 / 标签核模型，不作缠论
  canonical seam**：

  本谓词是对**任意** `X Ω TraceOut C` 成立的抽象双向核。codex 异质审计裁定它**不可作缠论
  canonical 接缝**——缠论分类标签（趋势/盘整/未完成；一/二/三类买卖点）比价格轨迹行为**粗**，
  故 ↔ 的 **→ 方向（同类 ⟹ 同行为）对非平凡缠论 trace 失败**。极限定理见
  `Foundation/CompleteClassificationLimits.lean`：
  - `iglobal_not_complete_minimal`：具体缠论分类器 `IGlobal` 在区分 2B/3B 的缠论可观测
    trace 下不满足本谓词（→ 方向 fail，witness=`x_2bOnly`/`x_2b3b` 2B/3B 重合点）。
  - `degenerate_trace_makes_iff_hold`：本谓词仅当 trace 退化为 classify 自身（零信息）时
    才空泛成立——有效域（退化 trace）严格小于定义域（任意 trace）。

  缠论 canonical 接缝**改指向**（本谓词不再承载 canonical 地位，但保留为元理论不删除）：
  - `Foundation/CompleteClassification.lean`：递归级唯一（recSpec_complete_unique /
    recAt_causal）+ 优先级互斥穷尽（priority_class_complete_unique）+ 全局声部唯一
    （global_class_complete_unique）+ 策略流水线存在唯一（strategy_spec_total_unique）。
  - `Strict/BSP.lean`：`no_global_classifies`（全域互斥分类不存在）+ `third_subdomain_classifies`
    （第三类本征子域真双射）+ `global_only_label_quotient`（按标签商分类）。

  以下三引理（`same_class_same_behavior` / `behavior_same_class` /
  `distinct_classes_behavior_separated`）仍是本谓词的合法逻辑推论——它们刻画的是「**若**某
  (Trace, Classify) 满足本谓词，则…」的条件性质，不预设缠论 trace 满足本谓词。
-/
def CompleteMinimalClassification
    (Trace : X -> Omega -> TraceOut) (Classify : X -> C) : Prop :=
  forall x y : X, Classify x = Classify y <-> BehEquiv Trace x y

theorem same_class_same_behavior
    {Trace : X -> Omega -> TraceOut} {Classify : X -> C}
    (hmin : CompleteMinimalClassification Trace Classify)
    {x y : X} (h : Classify x = Classify y) :
    BehEquiv Trace x y :=
  (hmin x y).1 h

theorem behavior_same_class
    {Trace : X -> Omega -> TraceOut} {Classify : X -> C}
    (hmin : CompleteMinimalClassification Trace Classify)
    {x y : X} (h : BehEquiv Trace x y) :
    Classify x = Classify y :=
  (hmin x y).2 h

theorem distinct_classes_behavior_separated
    {Trace : X -> Omega -> TraceOut} {Classify : X -> C}
    (hmin : CompleteMinimalClassification Trace Classify)
    {x y : X} (hneq : Classify x ≠ Classify y) :
    Not (BehEquiv Trace x y) := by
  intro hb
  exact hneq ((hmin x y).2 hb)

end Behavior

/-! ## Ten-way action priority partition -/

inductive ActionClass where
  | insolventOrLiquidation
  | deleverage
  | executionRepair
  | phaseTwoReturnCapital
  | rootOrAncestorInvalid
  | closeReverseChild
  | openRoot
  | openReverseChild
  | phaseThreeAccreteCore
  | hold
deriving DecidableEq, Repr

section ActionPriority

def ActionSpec
    (P1 P2 P3 P4 P5 P6 P7 P8 P9 : Bool) : ActionClass -> Prop
  | ActionClass.insolventOrLiquidation => P1 = true
  | ActionClass.deleverage => P1 = false /\ P2 = true
  | ActionClass.executionRepair => P1 = false /\ P2 = false /\ P3 = true
  | ActionClass.phaseTwoReturnCapital =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = true
  | ActionClass.rootOrAncestorInvalid =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = false /\ P5 = true
  | ActionClass.closeReverseChild =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = false /\
      P5 = false /\ P6 = true
  | ActionClass.openRoot =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = false /\
      P5 = false /\ P6 = false /\ P7 = true
  | ActionClass.openReverseChild =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = false /\
      P5 = false /\ P6 = false /\ P7 = false /\ P8 = true
  | ActionClass.phaseThreeAccreteCore =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = false /\
      P5 = false /\ P6 = false /\ P7 = false /\ P8 = false /\ P9 = true
  | ActionClass.hold =>
      P1 = false /\ P2 = false /\ P3 = false /\ P4 = false /\
      P5 = false /\ P6 = false /\ P7 = false /\ P8 = false /\ P9 = false

def chooseAction
    (P1 P2 P3 P4 P5 P6 P7 P8 P9 : Bool) : ActionClass :=
  if P1 then ActionClass.insolventOrLiquidation
  else if P2 then ActionClass.deleverage
  else if P3 then ActionClass.executionRepair
  else if P4 then ActionClass.phaseTwoReturnCapital
  else if P5 then ActionClass.rootOrAncestorInvalid
  else if P6 then ActionClass.closeReverseChild
  else if P7 then ActionClass.openRoot
  else if P8 then ActionClass.openReverseChild
  else if P9 then ActionClass.phaseThreeAccreteCore
  else ActionClass.hold

theorem action_spec_iff_chosen
    (P1 P2 P3 P4 P5 P6 P7 P8 P9 : Bool) (c : ActionClass) :
    ActionSpec P1 P2 P3 P4 P5 P6 P7 P8 P9 c <->
      c = chooseAction P1 P2 P3 P4 P5 P6 P7 P8 P9 := by
  cases P1 <;> cases P2 <;> cases P3 <;> cases P4 <;> cases P5 <;>
  cases P6 <;> cases P7 <;> cases P8 <;> cases P9 <;>
  cases c <;> simp [ActionSpec, chooseAction]

theorem action_partition_complete_unique
    (P1 P2 P3 P4 P5 P6 P7 P8 P9 : Bool) :
    ExistsUnique
      (fun c => ActionSpec P1 P2 P3 P4 P5 P6 P7 P8 P9 c) := by
  refine ⟨chooseAction P1 P2 P3 P4 P5 P6 P7 P8 P9, ?_, ?_⟩
  · exact
      (action_spec_iff_chosen P1 P2 P3 P4 P5 P6 P7 P8 P9
        (chooseAction P1 P2 P3 P4 P5 P6 P7 P8 P9)).2 rfl
  · intro c hc
    exact (action_spec_iff_chosen P1 P2 P3 P4 P5 P6 P7 P8 P9 c).1 hc

theorem action_classes_disjoint
    {P1 P2 P3 P4 P5 P6 P7 P8 P9 : Bool} {c d : ActionClass}
    (hc : ActionSpec P1 P2 P3 P4 P5 P6 P7 P8 P9 c)
    (hd : ActionSpec P1 P2 P3 P4 P5 P6 P7 P8 P9 d) :
    c = d := by
  have hc' := (action_spec_iff_chosen P1 P2 P3 P4 P5 P6 P7 P8 P9 c).1 hc
  have hd' := (action_spec_iff_chosen P1 P2 P3 P4 P5 P6 P7 P8 P9 d).1 hd
  exact hc'.trans hd'.symm

end ActionPriority

/-! ## Capital phase classifier -/

inductive CapitalPhase where
  | phaseI
  | phaseII
  | repair
  | protectedPhase
  | accretive
deriving DecidableEq, Repr

section CapitalPhaseSection

def PhaseSpec
    (returned pendingOrReady reserveBelow reserveBelowAccrete : Bool) :
    CapitalPhase -> Prop
  | CapitalPhase.phaseI =>
      returned = false /\ pendingOrReady = false
  | CapitalPhase.phaseII =>
      returned = false /\ pendingOrReady = true
  | CapitalPhase.repair =>
      returned = true /\ reserveBelow = true
  | CapitalPhase.protectedPhase =>
      returned = true /\ reserveBelow = false /\ reserveBelowAccrete = true
  | CapitalPhase.accretive =>
      returned = true /\ reserveBelow = false /\ reserveBelowAccrete = false

def choosePhase
    (returned pendingOrReady reserveBelow reserveBelowAccrete : Bool) :
    CapitalPhase :=
  if returned then
    if reserveBelow then CapitalPhase.repair
    else if reserveBelowAccrete then CapitalPhase.protectedPhase
    else CapitalPhase.accretive
  else
    if pendingOrReady then CapitalPhase.phaseII
    else CapitalPhase.phaseI

theorem phase_spec_iff_chosen
    (returned pendingOrReady reserveBelow reserveBelowAccrete : Bool)
    (p : CapitalPhase) :
    PhaseSpec returned pendingOrReady reserveBelow reserveBelowAccrete p <->
      p = choosePhase returned pendingOrReady reserveBelow reserveBelowAccrete := by
  cases returned <;> cases pendingOrReady <;>
  cases reserveBelow <;> cases reserveBelowAccrete <;>
  cases p <;> simp [PhaseSpec, choosePhase]

theorem phase_complete_unique
    (returned pendingOrReady reserveBelow reserveBelowAccrete : Bool) :
    ExistsUnique
      (fun p => PhaseSpec returned pendingOrReady reserveBelow reserveBelowAccrete p) := by
  refine ⟨choosePhase returned pendingOrReady reserveBelow reserveBelowAccrete, ?_, ?_⟩
  · exact
      (phase_spec_iff_chosen returned pendingOrReady reserveBelow
        reserveBelowAccrete
        (choosePhase returned pendingOrReady reserveBelow reserveBelowAccrete)).2 rfl
  · intro p hp
    exact
      (phase_spec_iff_chosen returned pendingOrReady reserveBelow
        reserveBelowAccrete p).1 hp

end CapitalPhaseSection

/-! ## Margin/risk mode classifier -/

inductive RiskMode where
  | insolvent
  | liquidation
  | deleverage
  | closeOnly
  | normal
deriving DecidableEq, Repr

section RiskMode

def RiskModeSpec
    (M0 rawM1 rawM2 rawM3 : Bool) : RiskMode -> Prop
  | RiskMode.insolvent => M0 = true
  | RiskMode.liquidation => M0 = false /\ rawM1 = true
  | RiskMode.deleverage => M0 = false /\ rawM1 = false /\ rawM2 = true
  | RiskMode.closeOnly =>
      M0 = false /\ rawM1 = false /\ rawM2 = false /\ rawM3 = true
  | RiskMode.normal =>
      M0 = false /\ rawM1 = false /\ rawM2 = false /\ rawM3 = false

def chooseRiskMode (M0 rawM1 rawM2 rawM3 : Bool) : RiskMode :=
  if M0 then RiskMode.insolvent
  else if rawM1 then RiskMode.liquidation
  else if rawM2 then RiskMode.deleverage
  else if rawM3 then RiskMode.closeOnly
  else RiskMode.normal

theorem risk_mode_spec_iff_chosen
    (M0 rawM1 rawM2 rawM3 : Bool) (m : RiskMode) :
    RiskModeSpec M0 rawM1 rawM2 rawM3 m <->
      m = chooseRiskMode M0 rawM1 rawM2 rawM3 := by
  cases M0 <;> cases rawM1 <;> cases rawM2 <;> cases rawM3 <;>
  cases m <;> simp [RiskModeSpec, chooseRiskMode]

theorem risk_mode_complete_unique (M0 rawM1 rawM2 rawM3 : Bool) :
    ExistsUnique (fun m => RiskModeSpec M0 rawM1 rawM2 rawM3 m) := by
  refine ⟨chooseRiskMode M0 rawM1 rawM2 rawM3, ?_, ?_⟩
  · exact
      (risk_mode_spec_iff_chosen M0 rawM1 rawM2 rawM3
        (chooseRiskMode M0 rawM1 rawM2 rawM3)).2 rfl
  · intro m hm
    exact (risk_mode_spec_iff_chosen M0 rawM1 rawM2 rawM3 m).1 hm

end RiskMode

/-! ## Mirror equivariance of classified policies -/

section Mirror

variable {X C U : Type}
variable (classA classB : X -> C)
variable (policyA policyB : C -> U)
variable (MX : X -> X) (MC : C -> C) (MU : U -> U)

def classifiedPolicy (classify : X -> C) (policy : C -> U) : X -> U :=
  fun x => policy (classify x)

theorem classified_policy_mirror_equivariant
    (hClass : forall x : X, classB (MX x) = MC (classA x))
    (hPolicy : forall c : C, policyB (MC c) = MU (policyA c))
    (x : X) :
    classifiedPolicy classB policyB (MX x) =
      MU (classifiedPolicy classA policyA x) := by
  unfold classifiedPolicy
  rw [hClass x]
  exact hPolicy (classA x)

end Mirror

/-! ## Dynamic closure on classification -/

section DynamicClosure

variable {X U E C Ebar : Type}
variable (Classify : X -> C)
variable (T : X -> U -> E -> X)
variable (Policy : X -> U)
variable (EventClass : E -> Ebar)
variable (Tbar : C -> Ebar -> C)

def DynamicallyClosed : Prop :=
  forall (x : X) (e : E),
    Classify (T x (Policy x) e) = Tbar (Classify x) (EventClass e)

theorem dynamic_closure_step
    (h : DynamicallyClosed Classify T Policy EventClass Tbar)
    (x : X) (e : E) :
    Classify (T x (Policy x) e) = Tbar (Classify x) (EventClass e) :=
  h x e

end DynamicClosure

/-! ## One-step deterministic hybrid pipeline -/

section HybridPipeline

variable {X E D C I U O XNext : Type}
variable (RecR : X -> D -> Prop)
variable (ClassR : X -> D -> C -> Prop)
variable (IntentR : X -> C -> I -> Prop)
variable (RiskR : X -> I -> U -> Prop)
variable (ScheduleR : X -> U -> O -> Prop)
variable (TransR : X -> O -> E -> XNext -> Prop)

def HybridStepSpec
    (x : X) (e : E)
    (out : D × C × I × U × O × XNext) : Prop :=
  let d := out.1
  let c := out.2.1
  let i := out.2.2.1
  let u := out.2.2.2.1
  let o := out.2.2.2.2.1
  let xnext := out.2.2.2.2.2
  RecR x d /\
  ClassR x d c /\
  IntentR x c i /\
  RiskR x i u /\
  ScheduleR x u o /\
  TransR x o e xnext

theorem hybrid_step_complete_unique
    (hRec : forall x : X, ExistsUnique (fun d : D => RecR x d))
    (hClass : forall (x : X) (d : D),
      ExistsUnique (fun c : C => ClassR x d c))
    (hIntent : forall (x : X) (c : C),
      ExistsUnique (fun i : I => IntentR x c i))
    (hRisk : forall (x : X) (i : I),
      ExistsUnique (fun u : U => RiskR x i u))
    (hSchedule : forall (x : X) (u : U),
      ExistsUnique (fun o : O => ScheduleR x u o))
    (hTrans : forall (x : X) (o : O) (e : E),
      ExistsUnique (fun xnext : XNext => TransR x o e xnext)) :
    forall (x : X) (e : E),
      ExistsUnique (fun out =>
        HybridStepSpec RecR ClassR IntentR RiskR ScheduleR TransR x e out) := by
  intro x e
  rcases hRec x with ⟨d, hd, hdUnique⟩
  rcases hClass x d with ⟨c, hc, hcUnique⟩
  rcases hIntent x c with ⟨i, hi, hiUnique⟩
  rcases hRisk x i with ⟨u, hu, huUnique⟩
  rcases hSchedule x u with ⟨o, ho, hoUnique⟩
  rcases hTrans x o e with ⟨xnext, hxnext, hxnextUnique⟩
  refine ⟨(d, c, i, u, o, xnext), ?_, ?_⟩
  · simp [HybridStepSpec, hd, hc, hi, hu, ho, hxnext]
  · intro out hout
    rcases out with ⟨d', c', i', u', o', xnext'⟩
    simp [HybridStepSpec] at hout
    rcases hout with ⟨hd', hc', hi', hu', ho', hxnext'⟩
    have dEq : d' = d := hdUnique d' hd'
    subst d'
    have cEq : c' = c := hcUnique c' hc'
    subst c'
    have iEq : i' = i := hiUnique i' hi'
    subst i'
    have uEq : u' = u := huUnique u' hu'
    subst u'
    have oEq : o' = o := hoUnique o' ho'
    subst o'
    have xnextEq : xnext' = xnext := hxnextUnique xnext' hxnext'
    subst xnext'
    rfl

end HybridPipeline

end NewChanlunHybrid
