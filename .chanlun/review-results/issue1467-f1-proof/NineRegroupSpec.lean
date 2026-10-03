import LiftBoundaryProof

namespace NineRegroup

open NewChanlun.Origin
open CenterAttempt (ObservedUnit)
open UpgradeResearch (Interval)

-- Independent source relation: gaps may be retained and block sizes are not fixed.
def SourceTriplet {α : Type} (xs a b c : List α) : Prop :=
  ∃ pre gap₁ gap₂ post : List α,
    pre ++ a ++ gap₁ ++ b ++ gap₂ ++ c ++ post = xs ∧
    3 ≤ a.length ∧ 3 ≤ b.length ∧ 3 ≤ c.length

def SLICE_TARGET : Prop := ∀ (α : Type) (xs a b c : List α),
  xs.length = 9 → SourceTriplet xs a b c →
    a = xs.take 3 ∧ b = (xs.drop 3).take 3 ∧ c = xs.drop 6 ∧
    a ++ b ++ c = xs ∧ a.length = 3 ∧ b.length = 3 ∧ c.length = 3

def GAP_TARGET : Prop := ∀ (α : Type) (xs pre a gap₁ b gap₂ c post : List α),
  xs.length = 9 → pre ++ a ++ gap₁ ++ b ++ gap₂ ++ c ++ post = xs →
  3 ≤ a.length → 3 ≤ b.length → 3 ≤ c.length →
    pre = [] ∧ gap₁ = [] ∧ gap₂ = [] ∧ post = []

def bounds : List ObservedUnit → Option Interval
  | [] => none
  | x :: xs => some (xs.foldl
      (fun acc u => ⟨min acc.lo (segLow u.2), max acc.hi (segHigh u.2)⟩)
      ⟨segLow x.2, segHigh x.2⟩)

def Carries (xs : List ObservedUnit) (s : Segment) : Prop :=
  bounds xs = some ⟨segLow s, segHigh s⟩

def input : List ObservedUnit := CenterFrame.members LiftBoundary.f3
def first := input.take 3
def second := (input.drop 3).take 3
def third := input.drop 6

def GeometryLift (xs : List ObservedUnit) : Prop :=
  ∃ a b c : List ObservedUnit,
    SourceTriplet xs a b c ∧ ∃ sa sb sc : Segment,
      Carries a sa ∧ Carries b sb ∧ Carries c sc ∧
      CenterConfirmedComplete sa sb sc

def NONEMPTY_TARGET : Prop :=
  SourceTriplet input first second third ∧
  bounds first = some ⟨10040,10060⟩ ∧
  bounds second = some ⟨10030,10055⟩ ∧
  bounds third = some ⟨10030,10040⟩

def NO_REGROUP_TARGET : Prop := ¬ GeometryLift input

end NineRegroup
