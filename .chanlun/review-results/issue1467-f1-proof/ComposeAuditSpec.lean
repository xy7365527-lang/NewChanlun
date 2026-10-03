import Formal.RecursiveConstruction
import LiftBoundaryProof

namespace ComposeAudit

open Formal.RecursiveConstruction

def embed (p : Nat → Int) (u : DCSpec.Unit) : Move :=
  .segment (match u.direction with | .up => .up | .down => .down)
    (min (p u.start) (p u.finish)) (max (p u.start) (p u.finish))

def pack3 (lvl : Nat) (a b c : Move) : Move :=
  .compose [a,b,c] (windowCenters [a,b,c]) (lvl+1)

def lowerC : List Move := CenterFrame.unitsC.map (embed CenterFrame.pathC)
def o0 := pack3 0 (embed CenterFrame.pathC CenterAttempt.a0)
  (embed CenterFrame.pathC CenterAttempt.a1) (embed CenterFrame.pathC CenterAttempt.a2)
def o1 := pack3 0 (embed CenterFrame.pathC CenterAttempt.a1)
  (embed CenterFrame.pathC CenterAttempt.a2) (embed CenterFrame.pathC CenterFrame.c3)
def o2 := pack3 0 (embed CenterFrame.pathC CenterAttempt.a2)
  (embed CenterFrame.pathC CenterFrame.c3) (embed CenterFrame.pathC CenterFrame.c4)
def overlapUppers : List Move := [o0,o1,o2]
def witnessAt (xs : List Move) (start : Nat) : UpperMoveWitness :=
  ⟨start,3,windowCenters ((xs.drop start).take 3)⟩
def overlapWitnesses : List UpperMoveWitness :=
  [witnessAt lowerC 0,witnessAt lowerC 1,witnessAt lowerC 2]
def consumedPositions (ws : List UpperMoveWitness) : List Nat :=
  ws.flatMap (fun w => (List.range w.len).map (fun j => w.start+j))

def OVERLAP_TARGET : Prop :=
  CenterFrame.NINE_TARGET ∧ lowerC.length = 9 ∧
  MovesComposedFrom lowerC 0 overlapUppers ∧
  PairwiseRel (fun m w => UpperMoveSound lowerC 0 m w) overlapUppers overlapWitnesses ∧
  consumedPositions overlapWitnesses = [0,1,2,1,2,3,2,3,4] ∧
  ¬ (consumedPositions overlapWitnesses).Nodup ∧
  (∀ i ∈ [5,6,7,8], i ∉ consumedPositions overlapWitnesses) ∧
  (canonicalWindows lowerC).map UpperMoveWitness.start = [0,3,6]

def r0 := pack3 0 (embed LiftBoundary.path LiftBoundary.u0)
  (embed LiftBoundary.path LiftBoundary.u1) (embed LiftBoundary.path LiftBoundary.u2)
def r1 := pack3 0 (embed LiftBoundary.path LiftBoundary.u3)
  (embed LiftBoundary.path LiftBoundary.u4) (embed LiftBoundary.path LiftBoundary.u5)
def r2 := pack3 0 (embed LiftBoundary.path LiftBoundary.u6)
  (embed LiftBoundary.path LiftBoundary.u7) (embed LiftBoundary.path LiftBoundary.u8)
def rangeUppers : List Move := [r0,r1,r2]
def falseParent := pack3 1 r0 r1 r2

def centerOuters : Move → List (Int × Int)
  | .segment _ _ _ => []
  | .compose _ cs _ => cs.map (fun c => (c.dd,c.gg))
def centerCores : Move → List (Int × Int)
  | .segment _ _ _ => []
  | .compose _ cs _ => cs.map (fun c => (c.zd,c.zg))

def RANGE_TARGET : Prop :=
  LiftBoundary.WITNESS_TARGET ∧
  (∀ m ∈ rangeUppers, WellFormed m) ∧
  rangeUppers.map centerOuters = [[(10040,10060)],[(10030,10055)],[(10030,10040)]] ∧
  rangeUppers.map Move.interval = [(0,10060),(0,10055),(0,10040)] ∧
  WindowOverlap r0 r1 r2 ∧ WellFormed falseParent ∧
  centerCores falseParent = [(0,10040)] ∧
  ¬ LiftBoundary.strictLift LiftBoundary.cells

end ComposeAudit
