/-
Origin/CompleteClassification.lean

Complete classification is not a list of cases.  It is a classifier whose
fibres are exactly behaviour-equivalence classes.
-/

import Origin.SourceAxioms

namespace NewChanlun.Origin

universe u v w

def BehEquiv {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) (x y : X) : Prop :=
  forall omega, trace x omega = trace y omega

theorem beh_refl {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) (x : X) :
    BehEquiv trace x x := by
  intro omega
  rfl

theorem beh_symm {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) {x y : X} :
    BehEquiv trace x y -> BehEquiv trace y x := by
  intro h omega
  exact (h omega).symm

theorem beh_trans {X : Type u} {Omega : Type v} {TraceOut : Type w}
    (trace : X -> Omega -> TraceOut) {x y z : X} :
    BehEquiv trace x y -> BehEquiv trace y z -> BehEquiv trace x z := by
  intro hxy hyz omega
  exact (hxy omega).trans (hyz omega)

structure CompleteClassifier
    (X : Type u) (Class : Type v) (Omega : Type w) (TraceOut : Type u) where
  classify : X -> Class
  trace : X -> Omega -> TraceOut
  complete : forall x y, classify x = classify y <-> BehEquiv trace x y

theorem same_class_same_behavior
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut)
    {x y : X} (h : C.classify x = C.classify y) :
    BehEquiv C.trace x y :=
  (C.complete x y).1 h

theorem behavior_same_class
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut)
    {x y : X} (h : BehEquiv C.trace x y) :
    C.classify x = C.classify y :=
  (C.complete x y).2 h

theorem distinct_classes_behavior_separated
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut)
    {x y : X} (hneq : C.classify x ≠ C.classify y) :
    Not (BehEquiv C.trace x y) := by
  intro hb
  exact hneq ((C.complete x y).2 hb)

theorem complete_classification_unique_class
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut) (x : X) :
    ExistsUnique (fun c => C.classify x = c) := by
  refine ⟨C.classify x, rfl, ?_⟩
  intro y hy
  exact hy.symm

theorem complete_classification_disjoint
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut)
    {x : X} {a b : Class}
    (ha : C.classify x = a) (hb : C.classify x = b) :
    a = b :=
  ha.symm.trans hb

structure FinitePartition (X : Type u) (Class : Type v) where
  inClass : X -> Class -> Prop
  totalUnique : forall x, ExistsUnique (fun c => inClass x c)

theorem finite_partition_total
    {X : Type u} {Class : Type v} (P : FinitePartition X Class) :
    Total P.inClass := by
  intro x
  rcases P.totalUnique x with ⟨c, hc, _⟩
  exact ⟨c, hc⟩

theorem finite_partition_disjoint
    {X : Type u} {Class : Type v} (P : FinitePartition X Class)
    {x : X} {a b : Class}
    (ha : P.inClass x a) (hb : P.inClass x b) :
    a = b := by
  rcases P.totalUnique x with ⟨c, _, huniq⟩
  exact (huniq a ha).trans (huniq b hb).symm

def ClassifierPartition
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut) : FinitePartition X Class where
  inClass x c := C.classify x = c
  totalUnique := complete_classification_unique_class C

theorem complete_classification_behavior_quotient
    {X : Type u} {Class : Type v} {Omega : Type w} {TraceOut : Type u}
    (C : CompleteClassifier X Class Omega TraceOut)
    (x y : X) :
    (ClassifierPartition C).inClass x (C.classify y) <->
      BehEquiv C.trace x y := by
  unfold ClassifierPartition
  exact C.complete x y

end NewChanlun.Origin
