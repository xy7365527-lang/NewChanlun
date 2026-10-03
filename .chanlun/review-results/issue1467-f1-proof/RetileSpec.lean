import CenterFrameProof
import UpgradeProof

namespace Retile

open NewChanlun.Origin
open CenterAttempt (ObservedUnit obs)

structure Triple where
  a : ObservedUnit
  b : ObservedUnit
  c : ObservedUnit

def tripleMembers (t : Triple) : List ObservedUnit := [t.a,t.b,t.c]
def flatten (ts : List Triple) : List ObservedUnit := ts.flatMap tripleMembers

inductive Cut : List ObservedUnit → List Triple → List ObservedUnit → Prop where
  | tail (xs : List ObservedUnit) (short : xs.length < 3) : Cut xs [] xs
  | block (a b c : ObservedUnit) {xs ts tail} :
      Cut xs ts tail → Cut (a::b::c::xs) (⟨a,b,c⟩::ts) tail

def partition : List ObservedUnit → List Triple × List ObservedUnit
  | a::b::c::rest =>
      let next := partition rest
      (⟨a,b,c⟩::next.1,next.2)
  | rest => ([],rest)

def Local (t : Triple) : Prop := CenterConfirmedComplete t.a.2 t.b.2 t.c.2
instance (t : Triple) : Decidable (Local t) := by unfold Local CenterConfirmedComplete; infer_instance

inductive Cell where
  | localCore (source : Triple) (proof : Local source) (availableAt : Nat)
  | rejected (source : Triple) (proof : ¬ Local source) (availableAt : Nat)

def sourceOf : Cell → Triple
  | .localCore t _ _ => t
  | .rejected t _ _ => t
def knownAt : Cell → Nat
  | .localCore _ _ k => k
  | .rejected _ _ k => k
def centerOf : Cell → Option CenterFull
  | .localCore t h _ => some (centerFullOfConfirmed t.a.2 t.b.2 t.c.2 h)
  | .rejected _ _ _ => none
def accepted (c : Cell) : Bool := (centerOf c).isSome
def stamp (time : Nat) (t : Triple) : Nat :=
  max time (max t.a.1.knownAt (max t.b.1.knownAt t.c.1.knownAt))
def classify (time : Nat) (t : Triple) : Cell :=
  if h : Local t then .localCore t h (stamp time t) else .rejected t h (stamp time t)

def retile (time : Nat) (xs : List ObservedUnit) : List Cell × List ObservedUnit :=
  let p := partition xs
  (p.1.map (classify time),p.2)
def recover (cells : List Cell) (tail : List ObservedUnit) : List ObservedUnit :=
  cells.flatMap (fun c => tripleMembers (sourceOf c)) ++ tail
def numbers (c : Cell) : Option (Int × Int × Int × Int) :=
  (centerOf c).map (fun z => (z.core.zd,z.core.zg,z.dd,z.gg))

def PARTITION_TARGET : Prop :=
  (∀ xs, ∃ ts tail, Cut xs ts tail ∧
    ∀ otherTs otherTail, Cut xs otherTs otherTail → otherTs = ts ∧ otherTail = tail) ∧
  (∀ xs ts tail, Cut xs ts tail → flatten ts ++ tail = xs ∧
    tail.length < 3 ∧ xs.length = 3*ts.length + tail.length) ∧
  (∀ time xs, recover (retile time xs).1 (retile time xs).2 = xs)

def CELL_TARGET : Prop := ∀ time t,
  sourceOf (classify time t) = t ∧
  accepted (classify time t) = decide (Local t) ∧
  time ≤ knownAt (classify time t) ∧
  t.a.1.knownAt ≤ knownAt (classify time t) ∧
  t.b.1.knownAt ≤ knownAt (classify time t) ∧
  t.c.1.knownAt ≤ knownAt (classify time t) ∧
  (∀ z, centerOf (classify time t) = some z →
    z.core.zd = computeZD t.a.2 t.b.2 t.c.2 ∧ z.core.zg = computeZG t.a.2 t.b.2 t.c.2 ∧
    z.dd = computeDD t.a.2 t.b.2 t.c.2 ∧ z.gg = computeGG t.a.2 t.b.2 t.c.2 ∧
    z.core.startIndex = t.a.2.startIndex ∧ z.core.endIndex = t.c.2.endIndex)

def DC_REJECT_TARGET : Prop :=
  ∀ (p : Nat → Int) (delta : Int), 0 < delta → ∀ len result,
    DCSpec.Whole p delta len result →
    ∀ pre a b c rest, DCSpec.units result = pre ++ a::b::c::rest → ∀ time,
      accepted (classify time ⟨obs p a,obs p b,obs p c⟩) = false ↔
        computeZG (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) ≤
          computeZD (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c)

def cellsC := (retile CenterFrame.f3.knownAt (CenterFrame.members CenterFrame.f3)).1
def tailC := (retile CenterFrame.f3.knownAt (CenterFrame.members CenterFrame.f3)).2
def weakInput : List ObservedUnit := UpgradeResearch.witnessUnits.map (obs UpgradeResearch.witnessPath)
def weakCells := (retile 10 weakInput).1
def weakTail := (retile 10 weakInput).2

def WITNESS_TARGET : Prop :=
  CenterFrame.NINE_TARGET ∧ UpgradeResearch.COUNTEREXAMPLE_TARGET ∧
  cellsC.length = 3 ∧ tailC = [] ∧ cellsC.map accepted = [true,true,true] ∧
  cellsC.map numbers = [some (10024,10032,10020,10040),
    some (10030,10050,10024,10060),some (10028,10060,10026,10070)] ∧
  cellsC.map knownAt = [10,10,10] ∧ recover cellsC tailC = CenterFrame.members CenterFrame.f3 ∧
  weakCells.length = 3 ∧ weakTail = [] ∧ weakCells.map accepted = [true,true,false] ∧
  weakCells.map numbers = [some (10025,10045,10025,10045),some (10040,10045,10030,10050),none] ∧
  weakCells.map knownAt = [10,10,10] ∧ recover weakCells weakTail = weakInput

end Retile
