/-
Origin/BehaviorQuotient.lean

Concrete behaviour-quotient construction for the complete-classification start point.

This is L0: given an explicit trace semantics, the classifier into the quotient
`X / BehEquiv trace` has fibres exactly equal to behaviour-equivalence classes.
It is not an L1/L2 claim that a Rust label already realizes this quotient.
-/

import Origin.CompleteClassification

namespace NewChanlun.Origin

universe u v

/-- Behaviour equivalence induces a setoid on states. -/
def behSetoid {X : Type u} {Omega : Type v} {TraceOut : Type u}
    (trace : X -> Omega -> TraceOut) : Setoid X where
  r := BehEquiv trace
  iseqv := ⟨beh_refl trace, beh_symm trace, beh_trans trace⟩

/-- The canonical classifier whose labels are the actual behaviour-equivalence quotient. -/
def behaviorQuotientClassify {X : Type u} {Omega : Type v} {TraceOut : Type u}
    (trace : X -> Omega -> TraceOut) (x : X) : Quotient (behSetoid trace) :=
  Quotient.mk (behSetoid trace) x

/-- The canonical behaviour quotient is a complete classifier by construction. -/
def behaviorQuotientClassifier {X : Type u} {Omega : Type v} {TraceOut : Type u}
    (trace : X -> Omega -> TraceOut) :
    CompleteClassifier X (Quotient (behSetoid trace)) Omega TraceOut where
  classify := behaviorQuotientClassify trace
  trace := trace
  complete := by
    intro x y
    constructor
    · intro h
      exact Quotient.exact h
    · intro h
      exact Quotient.sound h

/-- Explicit fibre equivalence theorem for the canonical behaviour quotient. -/
theorem behavior_quotient_fiber_iff {X : Type u} {Omega : Type v} {TraceOut : Type u}
    (trace : X -> Omega -> TraceOut) (x y : X) :
    behaviorQuotientClassify trace x = behaviorQuotientClassify trace y <->
      BehEquiv trace x y :=
  (behaviorQuotientClassifier trace).complete x y

/-- Same quotient class implies identical trace behaviour for every future. -/
theorem behavior_quotient_same_class_same_behavior
    {X : Type u} {Omega : Type v} {TraceOut : Type u}
    (trace : X -> Omega -> TraceOut) {x y : X}
    (h : behaviorQuotientClassify trace x = behaviorQuotientClassify trace y) :
    BehEquiv trace x y :=
  (behavior_quotient_fiber_iff trace x y).1 h

/-- Identical trace behaviour for every future implies the same quotient class. -/
theorem behavior_quotient_behavior_same_class
    {X : Type u} {Omega : Type v} {TraceOut : Type u}
    (trace : X -> Omega -> TraceOut) {x y : X}
    (h : BehEquiv trace x y) :
    behaviorQuotientClassify trace x = behaviorQuotientClassify trace y :=
  (behavior_quotient_fiber_iff trace x y).2 h

/--
  A tiny anti-degeneration witness: the canonical behaviour quotient need not collapse every
  state into one class.
-/
def toyTrace (x : Bool) (_future : Unit) : Bool := x

/-- The toy trace separates `true` and `false`, so the quotient has at least two classes. -/
theorem toy_behavior_quotient_separates_true_false :
    behaviorQuotientClassify toyTrace true ≠ behaviorQuotientClassify toyTrace false := by
  intro h
  have hb := behavior_quotient_same_class_same_behavior toyTrace h
  have htf := hb ()
  simp [toyTrace] at htf

/-- Concrete non-collapse witness for the canonical quotient classifier. -/
theorem toy_behavior_quotient_has_two_classes :
    ∃ x y : Bool,
      behaviorQuotientClassify toyTrace x ≠ behaviorQuotientClassify toyTrace y := by
  exact ⟨true, false, toy_behavior_quotient_separates_true_false⟩

end NewChanlun.Origin
