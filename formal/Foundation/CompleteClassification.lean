/-!
Lean 4 formalization for a strict complete-classification strategy skeleton.

This file proves the parts that are purely mathematical:

1. A priority decision classification is a genuine partition.
2. A recursive level construction by total functions is unique at every level.
3. Per-voice complete classifications compose into a unique global
   multi-voice classification.
4. If every strategy stage is total and single-valued as a relation, then the
   whole Rec -> Class -> Voice -> Risk -> Exec strategy has a unique order.

The Chanlun-specific predicates and construction functions are intentionally
parameters. The proof states exactly which obligations the implementation must
discharge.
-/

namespace NewChanlun

universe u

def ExistsUnique {A : Type u} (p : A -> Prop) : Prop :=
  Exists (fun x => p x /\ forall y, p y -> y = x)

/-! ## Priority classification -/

inductive PriorityClass where
  | emergencyExit
  | ancestorClosed
  | localStop
  | reverseNest
  | sameNest
  | hold
deriving DecidableEq, Repr

section Priority

variable {X : Type}
variable (P1 P2 P3 P4 P5 : X -> Prop)

def IsPriorityClass (x : X) : PriorityClass -> Prop
  | PriorityClass.emergencyExit => P1 x
  | PriorityClass.ancestorClosed => Not (P1 x) /\ P2 x
  | PriorityClass.localStop => Not (P1 x) /\ Not (P2 x) /\ P3 x
  | PriorityClass.reverseNest =>
      Not (P1 x) /\ Not (P2 x) /\ Not (P3 x) /\ P4 x
  | PriorityClass.sameNest =>
      Not (P1 x) /\ Not (P2 x) /\ Not (P3 x) /\ Not (P4 x) /\ P5 x
  | PriorityClass.hold =>
      Not (P1 x) /\ Not (P2 x) /\ Not (P3 x) /\ Not (P4 x) /\ Not (P5 x)

def choosePriorityClass
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] (x : X) : PriorityClass :=
  if P1 x then
    PriorityClass.emergencyExit
  else if P2 x then
    PriorityClass.ancestorClosed
  else if P3 x then
    PriorityClass.localStop
  else if P4 x then
    PriorityClass.reverseNest
  else if P5 x then
    PriorityClass.sameNest
  else
    PriorityClass.hold

theorem priority_class_iff_chosen
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] (x : X) (c : PriorityClass) :
    IsPriorityClass P1 P2 P3 P4 P5 x c <->
      c = choosePriorityClass P1 P2 P3 P4 P5 x := by
  by_cases h1 : P1 x <;>
  by_cases h2 : P2 x <;>
  by_cases h3 : P3 x <;>
  by_cases h4 : P4 x <;>
  by_cases h5 : P5 x <;>
  cases c <;>
  simp [IsPriorityClass, choosePriorityClass, h1, h2, h3, h4, h5]

theorem priority_class_complete_unique
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] (x : X) :
    ExistsUnique (fun c => IsPriorityClass P1 P2 P3 P4 P5 x c) := by
  refine ⟨choosePriorityClass P1 P2 P3 P4 P5 x, ?_, ?_⟩
  · exact
      (priority_class_iff_chosen P1 P2 P3 P4 P5 x
        (choosePriorityClass P1 P2 P3 P4 P5 x)).2 rfl
  · intro c hc
    exact (priority_class_iff_chosen P1 P2 P3 P4 P5 x c).1 hc

theorem priority_classes_disjoint
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] {x : X}
    {c d : PriorityClass}
    (hc : IsPriorityClass P1 P2 P3 P4 P5 x c)
    (hd : IsPriorityClass P1 P2 P3 P4 P5 x d) :
    c = d := by
  have hc' := (priority_class_iff_chosen P1 P2 P3 P4 P5 x c).1 hc
  have hd' := (priority_class_iff_chosen P1 P2 P3 P4 P5 x d).1 hd
  exact hc'.trans hd'.symm

end Priority

/-! ## Recursive level construction -/

section LevelRecursion

variable {H : Type}
variable (D : Nat -> Type)
variable (F0 : H -> D 0)
variable (Fstep : (n : Nat) -> D n -> D (n + 1))

def recAt : (n : Nat) -> H -> D n
  | 0, h => F0 h
  | n + 1, h => Fstep n (recAt n h)

def RecSpec (h : H) : (n : Nat) -> D n -> Prop
  | 0, d => d = F0 h
  | n + 1, d =>
      Exists (fun prev : D n => RecSpec h n prev /\ d = Fstep n prev)

theorem recAt_spec (h : H) (n : Nat) :
    RecSpec D F0 Fstep h n (recAt D F0 Fstep n h) := by
  induction n with
  | zero =>
      simp [RecSpec, recAt]
  | succ n ih =>
      exact ⟨recAt D F0 Fstep n h, ih, rfl⟩

theorem recSpec_complete_unique (h : H) (n : Nat) :
    ExistsUnique (fun d : D n => RecSpec D F0 Fstep h n d) := by
  induction n with
  | zero =>
      refine ⟨F0 h, ?_, ?_⟩
      · rfl
      · intro y hy
        exact hy
  | succ n ih =>
      rcases ih with ⟨prev, hprev, hprevUnique⟩
      refine ⟨Fstep n prev, ?_, ?_⟩
      · exact ⟨prev, hprev, rfl⟩
      · intro y hy
        rcases hy with ⟨prev', hprev', hyEq⟩
        have hprevEq : prev' = prev := hprevUnique prev' hprev'
        subst prev'
        exact hyEq

theorem recAt_causal
    (ObsEq : H -> H -> Prop)
    (hF0 : forall {h h' : H}, ObsEq h h' -> F0 h = F0 h')
    {h h' : H} (hh : ObsEq h h') (n : Nat) :
    recAt D F0 Fstep n h = recAt D F0 Fstep n h' := by
  induction n with
  | zero =>
      simpa [recAt] using hF0 hh
  | succ n ih =>
      simp [recAt, ih]

end LevelRecursion

/-! ## Global multi-voice classification -/

section GlobalVoice

variable {X Voice LocalClass : Type}
variable (Local : Voice -> X -> LocalClass -> Prop)
variable (hLocal : forall (v : Voice) (x : X),
  ExistsUnique (fun c : LocalClass => Local v x c))

def GlobalClassSpec (x : X) (g : Voice -> LocalClass) : Prop :=
  forall v : Voice, Local v x (g v)

noncomputable def globalClass (x : X) : Voice -> LocalClass :=
  fun v => Classical.choose (hLocal v x)

theorem globalClass_spec (x : X) (v : Voice) :
    Local v x ((globalClass Local hLocal x) v) := by
  unfold globalClass
  exact (Classical.choose_spec (hLocal v x)).1

theorem global_class_complete_unique
    (hLocal : forall (v : Voice) (x : X),
      ExistsUnique (fun c : LocalClass => Local v x c))
    (x : X) :
    ExistsUnique (fun g : Voice -> LocalClass => GlobalClassSpec Local x g) := by
  classical
  refine ⟨globalClass Local hLocal x, ?_, ?_⟩
  · intro v
    exact globalClass_spec Local hLocal x v
  · intro g hg
    funext v
    rcases hLocal v x with ⟨c, hc, hUnique⟩
    have hgEq : g v = c := hUnique (g v) (hg v)
    have hChosenEq : (globalClass Local hLocal x) v = c :=
      hUnique ((globalClass Local hLocal x) v)
        (globalClass_spec Local hLocal x v)
    exact hgEq.trans hChosenEq.symm

end GlobalVoice

/-! ## Relational strategy pipeline -/

section Strategy

variable {X D C Q QStar O : Type}
variable (RecR : X -> D -> Prop)
variable (ClassR : X -> D -> C -> Prop)
variable (VoiceR : X -> C -> Q -> Prop)
variable (RiskR : X -> Q -> QStar -> Prop)
variable (ExecR : X -> QStar -> O -> Prop)

def StrategySpec (x : X) (o : O) : Prop :=
  Exists (fun d : D =>
  Exists (fun c : C =>
  Exists (fun q : Q =>
  Exists (fun qstar : QStar =>
    RecR x d /\
    ClassR x d c /\
    VoiceR x c q /\
    RiskR x q qstar /\
    ExecR x qstar o))))

theorem strategy_spec_total_unique
    (hRec : forall x : X, ExistsUnique (fun d : D => RecR x d))
    (hClass : forall (x : X) (d : D), RecR x d ->
      ExistsUnique (fun c : C => ClassR x d c))
    (hVoice : forall (x : X) (c : C), ClassR x
      (Classical.choose (hRec x)) c ->
      ExistsUnique (fun q : Q => VoiceR x c q))
    (hRisk : forall (x : X) (q : Q), VoiceR x
      (Classical.choose
        (hClass x (Classical.choose (hRec x))
          (Classical.choose_spec (hRec x)).1)) q ->
      ExistsUnique (fun qstar : QStar => RiskR x q qstar))
    (hExec : forall (x : X) (qstar : QStar), RiskR x
      (Classical.choose
        (hVoice x
          (Classical.choose
            (hClass x (Classical.choose (hRec x))
              (Classical.choose_spec (hRec x)).1))
          (Classical.choose_spec
            (hClass x (Classical.choose (hRec x))
              (Classical.choose_spec (hRec x)).1)).1)) qstar ->
      ExistsUnique (fun o : O => ExecR x qstar o)) :
    forall x : X, ExistsUnique
      (fun o : O => StrategySpec RecR ClassR VoiceR RiskR ExecR x o) := by
  classical
  intro x
  let d : D := Classical.choose (hRec x)
  have hd : RecR x d := (Classical.choose_spec (hRec x)).1
  let c : C := Classical.choose (hClass x d hd)
  have hc : ClassR x d c := (Classical.choose_spec (hClass x d hd)).1
  let q : Q := Classical.choose (hVoice x c hc)
  have hq : VoiceR x c q := (Classical.choose_spec (hVoice x c hc)).1
  let qstar : QStar := Classical.choose (hRisk x q hq)
  have hqstar : RiskR x q qstar := (Classical.choose_spec (hRisk x q hq)).1
  let o : O := Classical.choose (hExec x qstar hqstar)
  have ho : ExecR x qstar o := (Classical.choose_spec (hExec x qstar hqstar)).1
  refine ⟨o, ?_, ?_⟩
  · exact ⟨d, c, q, qstar, hd, hc, hq, hqstar, ho⟩
  · intro o' hs
    rcases hs with ⟨d', c', q', qstar', hd', hc', hq', hqstar', ho'⟩
    rcases hRec x with ⟨d0, hd0, hdUnique⟩
    have hdEq : d' = d := by
      have h1 : d' = d0 := hdUnique d' hd'
      have h2 : d = d0 := hdUnique d hd
      exact h1.trans h2.symm
    subst d'
    rcases hClass x d hd with ⟨c0, hc0, hcUnique⟩
    have hcEq : c' = c := by
      have h1 : c' = c0 := hcUnique c' hc'
      have h2 : c = c0 := hcUnique c hc
      exact h1.trans h2.symm
    subst c'
    rcases hVoice x c hc with ⟨q0, hq0, hqUnique⟩
    have hqEq : q' = q := by
      have h1 : q' = q0 := hqUnique q' hq'
      have h2 : q = q0 := hqUnique q hq
      exact h1.trans h2.symm
    subst q'
    rcases hRisk x q hq with ⟨qs0, hqs0, hqsUnique⟩
    have hqsEq : qstar' = qstar := by
      have h1 : qstar' = qs0 := hqsUnique qstar' hqstar'
      have h2 : qstar = qs0 := hqsUnique qstar hqstar
      exact h1.trans h2.symm
    subst qstar'
    rcases hExec x qstar hqstar with ⟨o0, ho0, hoUnique⟩
    have hoEq : o' = o := by
      have h1 : o' = o0 := hoUnique o' ho'
      have h2 : o = o0 := hoUnique o ho
      exact h1.trans h2.symm
    exact hoEq

end Strategy

end NewChanlun
