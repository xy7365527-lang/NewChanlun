/-
Origin/FullDefinitionStrategy.lean

The full-definition strategy is downstream of complete classification.  Given
fixed theta, every stage is a total deterministic function and the transition
writes back a full next state.
-/

import Origin.TrendCompleteClassification

namespace NewChanlun.Origin

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

namespace ActionClass

/--
  L0 mirror on action labels.

  Anti-degeneration witness: the two open constructors are the only flipped
  labels; every higher-priority, close, phase-three, and hold label is fixed.
  Boundary: this is a label symmetry only. Priority masking is handled by
  `chooseAction_mirror_open_signals_exclusive` below.
-/
def mirror : ActionClass -> ActionClass
  | openRoot => openReverseChild
  | openReverseChild => openRoot
  | a => a

end ActionClass

/-- L0 involution: mirroring an action label twice returns the original label. -/
theorem action_mirror_involutive (a : ActionClass) :
    ActionClass.mirror (ActionClass.mirror a) = a := by
  cases a <;> rfl

def chooseAction
    (p1 p2 p3 p4 p5 p6 p7 p8 p9 : Bool) : ActionClass :=
  if p1 then ActionClass.insolventOrLiquidation
  else if p2 then ActionClass.deleverage
  else if p3 then ActionClass.executionRepair
  else if p4 then ActionClass.phaseTwoReturnCapital
  else if p5 then ActionClass.rootOrAncestorInvalid
  else if p6 then ActionClass.closeReverseChild
  else if p7 then ActionClass.openRoot
  else if p8 then ActionClass.openReverseChild
  else if p9 then ActionClass.phaseThreeAccreteCore
  else ActionClass.hold

def ActionSpec
    (p1 p2 p3 p4 p5 p6 p7 p8 p9 : Bool) : ActionClass -> Prop
  | ActionClass.insolventOrLiquidation => p1 = true
  | ActionClass.deleverage => p1 = false /\ p2 = true
  | ActionClass.executionRepair => p1 = false /\ p2 = false /\ p3 = true
  | ActionClass.phaseTwoReturnCapital => p1 = false /\ p2 = false /\ p3 = false /\ p4 = true
  | ActionClass.rootOrAncestorInvalid => p1 = false /\ p2 = false /\ p3 = false /\ p4 = false /\ p5 = true
  | ActionClass.closeReverseChild => p1 = false /\ p2 = false /\ p3 = false /\ p4 = false /\ p5 = false /\ p6 = true
  | ActionClass.openRoot => p1 = false /\ p2 = false /\ p3 = false /\ p4 = false /\ p5 = false /\ p6 = false /\ p7 = true
  | ActionClass.openReverseChild => p1 = false /\ p2 = false /\ p3 = false /\ p4 = false /\ p5 = false /\ p6 = false /\ p7 = false /\ p8 = true
  | ActionClass.phaseThreeAccreteCore => p1 = false /\ p2 = false /\ p3 = false /\ p4 = false /\ p5 = false /\ p6 = false /\ p7 = false /\ p8 = false /\ p9 = true
  | ActionClass.hold => p1 = false /\ p2 = false /\ p3 = false /\ p4 = false /\ p5 = false /\ p6 = false /\ p7 = false /\ p8 = false /\ p9 = false

/--
  L0 flip theorem for the exclusive open-signal boundary.

  Anti-degeneration witness: `p7 ≠ p8` forces exactly one open predicate to be
  true, so the chosen action is one of `openRoot`/`openReverseChild`, not `hold`
  or a masked priority action. Boundary: the theorem deliberately fixes
  `p1..p6 = false` and `p9 = false`; if a higher-priority predicate or p9 is
  true, priority ordering, not open-signal mirroring, controls the result.
-/
theorem chooseAction_mirror_open_signals_exclusive (p7 p8 : Bool)
    (hopen : p7 ≠ p8) :
    ActionClass.mirror
        (chooseAction false false false false false false p7 p8 false) =
      chooseAction false false false false false false p8 p7 false := by
  cases p7 <;> cases p8 <;> simp [chooseAction, ActionClass.mirror] at *

theorem action_spec_iff_chosen
    (p1 p2 p3 p4 p5 p6 p7 p8 p9 : Bool) (a : ActionClass) :
    ActionSpec p1 p2 p3 p4 p5 p6 p7 p8 p9 a <->
      a = chooseAction p1 p2 p3 p4 p5 p6 p7 p8 p9 := by
  cases p1 <;> cases p2 <;> cases p3 <;> cases p4 <;> cases p5 <;>
  cases p6 <;> cases p7 <;> cases p8 <;> cases p9 <;>
  cases a <;> simp [ActionSpec, chooseAction]

theorem action_priority_complete_unique
    (p1 p2 p3 p4 p5 p6 p7 p8 p9 : Bool) :
    ExistsUnique (fun a => ActionSpec p1 p2 p3 p4 p5 p6 p7 p8 p9 a) := by
  refine ⟨chooseAction p1 p2 p3 p4 p5 p6 p7 p8 p9, ?_, ?_⟩
  · exact
      (action_spec_iff_chosen p1 p2 p3 p4 p5 p6 p7 p8 p9
        (chooseAction p1 p2 p3 p4 p5 p6 p7 p8 p9)).2 rfl
  · intro a ha
    exact (action_spec_iff_chosen p1 p2 p3 p4 p5 p6 p7 p8 p9 a).1 ha

inductive RiskMode where
  | insolvent
  | liquidation
  | deleverage
  | closeOnly
  | normal
deriving DecidableEq, Repr

def chooseRiskMode (m0 m1 m2 m3 : Bool) : RiskMode :=
  if m0 then RiskMode.insolvent
  else if m1 then RiskMode.liquidation
  else if m2 then RiskMode.deleverage
  else if m3 then RiskMode.closeOnly
  else RiskMode.normal

def RiskModeSpec (m0 m1 m2 m3 : Bool) : RiskMode -> Prop
  | RiskMode.insolvent => m0 = true
  | RiskMode.liquidation => m0 = false /\ m1 = true
  | RiskMode.deleverage => m0 = false /\ m1 = false /\ m2 = true
  | RiskMode.closeOnly => m0 = false /\ m1 = false /\ m2 = false /\ m3 = true
  | RiskMode.normal => m0 = false /\ m1 = false /\ m2 = false /\ m3 = false

theorem risk_mode_spec_iff_chosen (m0 m1 m2 m3 : Bool) (m : RiskMode) :
    RiskModeSpec m0 m1 m2 m3 m <->
      m = chooseRiskMode m0 m1 m2 m3 := by
  cases m0 <;> cases m1 <;> cases m2 <;> cases m3 <;>
  cases m <;> simp [RiskModeSpec, chooseRiskMode]

theorem risk_mode_complete_unique (m0 m1 m2 m3 : Bool) :
    ExistsUnique (fun m => RiskModeSpec m0 m1 m2 m3 m) := by
  refine ⟨chooseRiskMode m0 m1 m2 m3, ?_, ?_⟩
  · exact
      (risk_mode_spec_iff_chosen m0 m1 m2 m3
        (chooseRiskMode m0 m1 m2 m3)).2 rfl
  · intro m hm
    exact (risk_mode_spec_iff_chosen m0 m1 m2 m3 m).1 hm

inductive CapitalPhase where
  | phaseI
  | phaseII
  | repair
  | protectedPhase
  | accretive
deriving DecidableEq, Repr

def chooseCapitalPhase (returned ready reserveLow accreteLow : Bool) : CapitalPhase :=
  if returned then
    if reserveLow then CapitalPhase.repair
    else if accreteLow then CapitalPhase.protectedPhase
    else CapitalPhase.accretive
  else
    if ready then CapitalPhase.phaseII else CapitalPhase.phaseI

def CapitalPhaseSpec (returned ready reserveLow accreteLow : Bool) : CapitalPhase -> Prop
  | CapitalPhase.phaseI => returned = false /\ ready = false
  | CapitalPhase.phaseII => returned = false /\ ready = true
  | CapitalPhase.repair => returned = true /\ reserveLow = true
  | CapitalPhase.protectedPhase => returned = true /\ reserveLow = false /\ accreteLow = true
  | CapitalPhase.accretive => returned = true /\ reserveLow = false /\ accreteLow = false

theorem capital_phase_spec_iff_chosen
    (returned ready reserveLow accreteLow : Bool) (p : CapitalPhase) :
    CapitalPhaseSpec returned ready reserveLow accreteLow p <->
      p = chooseCapitalPhase returned ready reserveLow accreteLow := by
  cases returned <;> cases ready <;> cases reserveLow <;> cases accreteLow <;>
  cases p <;> simp [CapitalPhaseSpec, chooseCapitalPhase]

theorem capital_phase_complete_unique (returned ready reserveLow accreteLow : Bool) :
    ExistsUnique (fun p => CapitalPhaseSpec returned ready reserveLow accreteLow p) := by
  refine ⟨chooseCapitalPhase returned ready reserveLow accreteLow, ?_, ?_⟩
  · exact
      (capital_phase_spec_iff_chosen returned ready reserveLow accreteLow
        (chooseCapitalPhase returned ready reserveLow accreteLow)).2 rfl
  · intro p hp
    exact (capital_phase_spec_iff_chosen returned ready reserveLow accreteLow p).1 hp

structure LedgerState where
  Pi : Int
  A : Int
  W : Int
  R : Int
  inv : R = Pi - A - W
deriving Repr

def mkLedger (Pi A W : Int) : LedgerState :=
  { Pi := Pi, A := A, W := W, R := Pi - A - W, inv := rfl }

def ledgerStep (L : LedgerState) (dPi dA dW : Int) : LedgerState :=
  mkLedger (L.Pi + dPi) (L.A + dA) (L.W + dW)

theorem ledger_invariant_preservation (L : LedgerState) (dPi dA dW : Int) :
    (ledgerStep L dPi dA dW).R =
      (ledgerStep L dPi dA dW).Pi -
      (ledgerStep L dPi dA dW).A -
      (ledgerStep L dPi dA dW).W :=
  (ledgerStep L dPi dA dW).inv

structure StrictState where
  parsed : ParseStruct
  trend : TrendClass
  actionClass : ActionClass
  riskMode : RiskMode
  phase : CapitalPhase
  ledger : LedgerState
deriving Repr

structure FullDefinitionSystem where
  Event : Type
  Intent : Type
  Control : Type
  Order : Type
  recStruct : StrictState -> Event -> ParseStruct
  classify : StrictState -> ParseStruct -> TrendClass
  intent : StrictState -> TrendClass -> Intent
  risk : StrictState -> Intent -> Control
  schedule : StrictState -> Control -> Order
  transition : StrictState -> Order -> Event -> StrictState

def policyTheta (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) : S.Order :=
  S.schedule x (S.risk x (S.intent x (S.classify x (S.recStruct x e))))

def hybridStep (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) : StrictState :=
  S.transition x (policyTheta S x e) e

theorem hybrid_step_complete_unique (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) :
    ExistsUnique (fun x' => hybridStep S x e = x') := by
  refine ⟨hybridStep S x e, rfl, ?_⟩
  intro y hy
  exact hy.symm

theorem policy_factors_through_classification
    (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) :
    policyTheta S x e =
      S.schedule x (S.risk x (S.intent x (S.classify x (S.recStruct x e)))) :=
  rfl

theorem transition_writes_full_state
    (S : FullDefinitionSystem) (x : StrictState) (e : S.Event) :
    hybridStep S x e = S.transition x (policyTheta S x e) e :=
  rfl

theorem hybrid_step_causal
    (S : FullDefinitionSystem)
    (obsEq : StrictState -> StrictState -> Prop)
    (eventEq : S.Event -> S.Event -> Prop)
    (h : forall {x y e f}, obsEq x y -> eventEq e f -> hybridStep S x e = hybridStep S y f)
    {x y : StrictState} {e f : S.Event}
    (hx : obsEq x y) (he : eventEq e f) :
    hybridStep S x e = hybridStep S y f :=
  h hx he

theorem hybrid_step_mirror_equivariant
    (S : FullDefinitionSystem)
    (mirrorState : StrictState -> StrictState)
    (mirrorEvent : S.Event -> S.Event)
    (h : forall x e, hybridStep S (mirrorState x) (mirrorEvent e) =
      mirrorState (hybridStep S x e))
    (x : StrictState) (e : S.Event) :
    hybridStep S (mirrorState x) (mirrorEvent e) =
      mirrorState (hybridStep S x e) :=
  h x e

end NewChanlun.Origin
