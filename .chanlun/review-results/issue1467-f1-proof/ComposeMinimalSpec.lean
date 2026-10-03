import ComposeAuditProof

namespace ComposeMinimal

open Formal.RecursiveConstruction

def lower := ComposeAudit.lowerC.take 6
def uppers := [ComposeAudit.o0,ComposeAudit.o1]
def witnesses := [ComposeAudit.witnessAt lower 0,ComposeAudit.witnessAt lower 1]

def WITNESS_TARGET : Prop :=
  CenterFrame.NINE_TARGET ∧ lower.length = 6 ∧ uppers.length = 2 ∧
  MovesComposedFrom lower 0 uppers ∧
  PairwiseRel (fun m w => UpperMoveSound lower 0 m w) uppers witnesses ∧
  ComposeAudit.consumedPositions witnesses = [0,1,2,1,2,3] ∧
  ¬ (ComposeAudit.consumedPositions witnesses).Nodup ∧
  (∀ i ∈ [4,5], i ∉ ComposeAudit.consumedPositions witnesses)

def LOWER_BOUND_TARGET : Prop := ∀ (xs ys : List Move) (lvl : Nat),
  2 ≤ ys.length → MovesComposedFrom xs lvl ys → 6 ≤ xs.length

end ComposeMinimal
