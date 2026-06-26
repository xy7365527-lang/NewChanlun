/-
Origin/TrendCompleteClassification.lean

Trend classification starts before the strategy.  A current state is either an
unfinished tail, a consolidation, an upward trend, or a downward trend.  There
is no fourth completed trend kind.
-/

import Origin.ChanlunElements

namespace NewChanlun.Origin

inductive TrendClass where
  | unfinished
  | consolidation
  | trendUp
  | trendDown
deriving DecidableEq, Repr

def TrendClass.mirror : TrendClass -> TrendClass
  | TrendClass.unfinished => TrendClass.unfinished
  | TrendClass.consolidation => TrendClass.consolidation
  | TrendClass.trendUp => TrendClass.trendDown
  | TrendClass.trendDown => TrendClass.trendUp

theorem trend_mirror_involutive (c : TrendClass) : c.mirror.mirror = c := by
  cases c <;> rfl

inductive TrendOrder where
  | falling
  | flat
  | rising
deriving DecidableEq, Repr

def TrendOrder.mirror : TrendOrder -> TrendOrder
  | TrendOrder.falling => TrendOrder.rising
  | TrendOrder.flat => TrendOrder.flat
  | TrendOrder.rising => TrendOrder.falling

theorem trend_order_mirror_involutive (o : TrendOrder) : o.mirror.mirror = o := by
  cases o <;> rfl

def chooseTrendByOrder
    (complete hasTwoCenters : Bool) (order : TrendOrder) : TrendClass :=
  if complete then
    if hasTwoCenters then
      match order with
      | TrendOrder.rising => TrendClass.trendUp
      | TrendOrder.falling => TrendClass.trendDown
      | TrendOrder.flat => TrendClass.consolidation
    else
      TrendClass.consolidation
  else
    TrendClass.unfinished

theorem chooseTrendByOrder_flat_two_centers :
    chooseTrendByOrder true true TrendOrder.flat = TrendClass.consolidation := by
  rfl

theorem chooseTrendByOrder_mirror
    (complete hasTwoCenters : Bool) (order : TrendOrder) :
    chooseTrendByOrder complete hasTwoCenters order.mirror =
      (chooseTrendByOrder complete hasTwoCenters order).mirror := by
  cases complete <;> cases hasTwoCenters <;> cases order <;> rfl

def chooseTrend (complete hasTwoCenters up : Bool) : TrendClass :=
  if complete then
    if hasTwoCenters then
      if up then TrendClass.trendUp else TrendClass.trendDown
    else
      TrendClass.consolidation
  else
    TrendClass.unfinished

def TrendSpec (complete hasTwoCenters up : Bool) : TrendClass -> Prop
  | TrendClass.unfinished => complete = false
  | TrendClass.consolidation => complete = true /\ hasTwoCenters = false
  | TrendClass.trendUp => complete = true /\ hasTwoCenters = true /\ up = true
  | TrendClass.trendDown => complete = true /\ hasTwoCenters = true /\ up = false

theorem trend_spec_iff_chosen
    (complete hasTwoCenters up : Bool) (c : TrendClass) :
    TrendSpec complete hasTwoCenters up c <->
      c = chooseTrend complete hasTwoCenters up := by
  cases complete <;> cases hasTwoCenters <;> cases up <;>
  cases c <;> simp [TrendSpec, chooseTrend]

theorem trend_complete_unique
    (complete hasTwoCenters up : Bool) :
    ExistsUnique (fun c => TrendSpec complete hasTwoCenters up c) := by
  refine ⟨chooseTrend complete hasTwoCenters up, ?_, ?_⟩
  · exact
      (trend_spec_iff_chosen complete hasTwoCenters up
        (chooseTrend complete hasTwoCenters up)).2 rfl
  · intro c hc
    exact (trend_spec_iff_chosen complete hasTwoCenters up c).1 hc

theorem trend_classes_disjoint
    {complete hasTwoCenters up : Bool} {a b : TrendClass}
    (ha : TrendSpec complete hasTwoCenters up a)
    (hb : TrendSpec complete hasTwoCenters up b) :
    a = b := by
  have ha' := (trend_spec_iff_chosen complete hasTwoCenters up a).1 ha
  have hb' := (trend_spec_iff_chosen complete hasTwoCenters up b).1 hb
  exact ha'.trans hb'.symm

theorem trend_no_fourth_class
    (c : TrendClass) :
    c = TrendClass.unfinished \/
    c = TrendClass.consolidation \/
    c = TrendClass.trendUp \/
    c = TrendClass.trendDown := by
  cases c <;> simp

structure TrendClassifier where
  isComplete : ParseStruct -> Bool
  hasTwoCenters : ParseStruct -> Bool
  isUp : ParseStruct -> Bool

def TrendClassifier.classify (C : TrendClassifier) (s : ParseStruct) : TrendClass :=
  chooseTrend (C.isComplete s) (C.hasTwoCenters s) (C.isUp s)

theorem trend_classifier_total_unique (C : TrendClassifier) (s : ParseStruct) :
    ExistsUnique (fun c => C.classify s = c) := by
  refine ⟨C.classify s, rfl, ?_⟩
  intro c hc
  exact hc.symm

theorem trend_mirror_equivariant
    (classA classB : ParseStruct -> TrendClass)
    (mirrorState : ParseStruct -> ParseStruct)
    (h : forall s, classB (mirrorState s) = (classA s).mirror)
    (s : ParseStruct) :
    classB (mirrorState s) = (classA s).mirror :=
  h s

structure RecursiveLevelSystem where
  State : Nat -> Type
  lift : (n : Nat) -> State n -> State (n + 1)

def RecursiveLevelSystem.at (R : RecursiveLevelSystem)
    (base : R.State 0) : (n : Nat) -> R.State n
  | 0 => base
  | n + 1 => R.lift n (R.at base n)

theorem recursive_level_self_similar
    (R : RecursiveLevelSystem) (base : R.State 0) (n : Nat) :
    R.at base (n + 1) = R.lift n (R.at base n) := by
  rfl

end NewChanlun.Origin
