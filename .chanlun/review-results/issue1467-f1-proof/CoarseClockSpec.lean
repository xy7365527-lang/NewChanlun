import ClockEdgeProof

namespace CoarseClock

open NewChanlun.Origin

abbrev Record := List (Int × Nat)
def Strict (t : Nat → Nat) : Prop := ∀ i j : Fin 5, i.val < j.val → t i.val < t j.val
def record (t : Nat → Nat) : Record :=
  (List.range 5).map (fun i => (ClockEdge.path i,t i / 10))
def Compatible (o : Record) (t : Nat → Nat) : Prop := Strict t ∧ record t = o
def primitives (t : Nat → Nat) := ClockEdge.units.map (ClockEdge.fromUnit t)
def Div (t : Nat → Nat) : Prop := IsImpulseDivergence ClockEdge.reference (primitives t)
instance (t : Nat → Nat) : Decidable (Div t) := by unfold Div; infer_instance
def decision (t : Nat → Nat) : Bool := decide (Div t)
def shared : Record := record (ClockEdge.clock true)

def Sound (judge : Record → Option Bool) : Prop :=
  ∀ t, Strict t → ∀ value, judge (record t) = some value → value = decision t

def WITNESS_TARGET : Prop :=
  DCSpec.Whole ClockEdge.path 2 5 ClockEdge.result ∧
  (DCSpec.units ClockEdge.result).length = 3 ∧
  (∀ b, Strict (ClockEdge.clock b)) ∧
  (∀ b, ∀ i : Fin 5, ClockEdge.clock b i.val / 10 = 0) ∧
  record (ClockEdge.clock true) = record (ClockEdge.clock false) ∧
  Compatible shared (ClockEdge.clock true) ∧ Compatible shared (ClockEdge.clock false) ∧
  decision (ClockEdge.clock true) = true ∧ decision (ClockEdge.clock false) = false

def AMBIGUITY_TARGET : Prop :=
  (∃ t, Compatible shared t ∧ Div t) ∧
  (∃ t, Compatible shared t ∧ ¬ Div t)

def NO_SELECTOR_TARGET : Prop :=
  ¬ ∃ judge : Record → Bool, ∀ t, Strict t → judge (record t) = decision t

def PARTIAL_TARGET : Prop :=
  ∀ judge : Record → Option Bool, Sound judge → judge shared = none

end CoarseClock
