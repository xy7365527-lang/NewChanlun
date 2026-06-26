/-
Origin/FiniteTraceQuotient.lean

Finite-corpus behaviour quotient for runtime audits.

This is deliberately weaker than `BehaviorQuotient.lean`: it proves an exact
fibre contract only for a supplied finite list of futures.  This matches the
Rust `FiniteOrderTraceBehaviorClass` audit shape and is not an all-futures L1
claim.
-/

import Origin.BehaviorQuotient

namespace NewChanlun.Origin

universe u v w

/-- Behaviour equivalence restricted to a supplied finite future corpus. -/
def CorpusBehEquiv {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) (corpus : List Omega) (x y : X) : Prop :=
  ∀ omega, omega ∈ corpus -> trace x omega = trace y omega

/-- The finite-corpus class is exactly the trace signature over that corpus. -/
def finiteCorpusClassify {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) (corpus : List Omega) (x : X) : List TraceOut :=
  corpus.map (fun omega => trace x omega)

/-- The finite-corpus classifier has exactly the bounded corpus behaviour fibres. -/
theorem finite_corpus_fiber_iff
    {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) (corpus : List Omega) (x y : X) :
    finiteCorpusClassify trace corpus x = finiteCorpusClassify trace corpus y <->
      CorpusBehEquiv trace corpus x y := by
  induction corpus with
  | nil =>
      simp [finiteCorpusClassify, CorpusBehEquiv]
  | cons omega rest ih =>
      simp [finiteCorpusClassify, CorpusBehEquiv]

/-- Same finite class implies same traces for every future in the corpus. -/
theorem finite_corpus_same_class_same_behavior
    {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) {corpus : List Omega} {x y : X}
    (h : finiteCorpusClassify trace corpus x = finiteCorpusClassify trace corpus y) :
    CorpusBehEquiv trace corpus x y :=
  (finite_corpus_fiber_iff trace corpus x y).1 h

/-- Same traces over the finite corpus imply the same finite class. -/
theorem finite_corpus_behavior_same_class
    {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) {corpus : List Omega} {x y : X}
    (h : CorpusBehEquiv trace corpus x y) :
    finiteCorpusClassify trace corpus x = finiteCorpusClassify trace corpus y :=
  (finite_corpus_fiber_iff trace corpus x y).2 h

/-- Empty corpora collapse every state: a useful boundary condition. -/
theorem finite_corpus_empty_collapses
    {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) (x y : X) :
    finiteCorpusClassify trace [] x = finiteCorpusClassify trace [] y := by
  rfl

/-- A tiny anti-degeneration witness for nonempty finite corpora. -/
def finiteToyTrace (x : Bool) (_future : Unit) : Bool := x

/-- The one-future corpus separates `true` and `false`. -/
theorem finite_corpus_toy_separates_true_false :
    finiteCorpusClassify finiteToyTrace [()] true ≠
      finiteCorpusClassify finiteToyTrace [()] false := by
  simp [finiteCorpusClassify, finiteToyTrace]

/-- Concrete non-collapse witness for the finite-corpus classifier. -/
theorem finite_corpus_toy_has_two_classes :
    ∃ x y : Bool,
      finiteCorpusClassify finiteToyTrace [()] x ≠
        finiteCorpusClassify finiteToyTrace [()] y := by
  exact ⟨true, false, finite_corpus_toy_separates_true_false⟩

end NewChanlun.Origin
