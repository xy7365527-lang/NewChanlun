import ComposeMinimalProof

namespace SourceHull

open UpgradeResearch (Interval Valid core3)

inductive Role where
  | candidate | retained | active
  deriving DecidableEq, Repr

structure Part where
  role : Role
  start : Nat
  len : Nat
  deriving DecidableEq, Repr

inductive Cover : Nat → Nat → List Part → Prop where
  | done (pos : Nat) : Cover pos pos []
  | step (p : Part) {finish : Nat} {rest : List Part} :
      0 < p.len → Cover (p.start+p.len) finish rest → Cover p.start finish (p::rest)

def positions (parts : List Part) : List Nat :=
  parts.flatMap (fun p => List.range' p.start p.len)

def COVER_TARGET : Prop := ∀ start finish parts, Cover start finish parts →
  start ≤ finish ∧ positions parts = List.range' start (finish-start) ∧
  (positions parts).Nodup

def merge (a b : Interval) : Interval := ⟨min a.lo b.lo,max a.hi b.hi⟩
def hull : List Interval → Option Interval
  | [] => none
  | a::xs => match hull xs with
      | none => some a
      | some b => some (merge a b)

def ExactHull (xs : List Interval) (h : Interval) : Prop :=
  (∀ x ∈ xs, h.lo ≤ x.lo ∧ x.hi ≤ h.hi) ∧
  (∃ x ∈ xs, x.lo = h.lo) ∧ (∃ x ∈ xs, x.hi = h.hi)

def HULL_TARGET : Prop :=
  (∀ xs, hull xs = none ↔ xs = []) ∧
  (∀ xs h, hull xs = some h ↔ ExactHull xs h) ∧
  (∀ xs a b, ExactHull xs a → ExactHull xs b → a = b) ∧
  (∀ xs h, hull xs = some h → (∀ x ∈ xs, Valid x) → Valid h)

def badParts : List Part := ComposeMinimal.witnesses.map
  (fun w => ⟨.candidate,w.start,w.len⟩)
def keptNine : List Part := [⟨.candidate,0,3⟩,⟨.candidate,3,3⟩,⟨.retained,6,3⟩]
def withTail : List Part := keptNine ++ [⟨.active,9,2⟩]

def intervals (xs : List CenterAttempt.ObservedUnit) : List Interval :=
  xs.map (fun x => ⟨NewChanlun.Origin.segLow x.2,NewChanlun.Origin.segHigh x.2⟩)
def realFirst := hull (intervals (CenterFrame.members LiftBoundary.f3 |>.take 3))
def realSecond := hull (intervals (CenterFrame.members LiftBoundary.f3 |>.drop 3 |>.take 3))
def realThird := hull (intervals (CenterFrame.members LiftBoundary.f3 |>.drop 6))

def WITNESS_TARGET : Prop :=
  ComposeMinimal.WITNESS_TARGET ∧ ComposeAudit.RANGE_TARGET ∧
  ¬ Cover 0 6 badParts ∧ Cover 0 9 keptNine ∧ Cover 0 11 withTail ∧
  Retile.weakCells.map Retile.accepted = [true,true,false] ∧
  realFirst = some ⟨10040,10060⟩ ∧ realSecond = some ⟨10030,10055⟩ ∧
  realThird = some ⟨10030,10040⟩ ∧
  ¬ Valid (core3 ⟨10040,10060⟩ ⟨10030,10055⟩ ⟨10030,10040⟩) ∧
  hull [] = none ∧ hull [⟨-9,-6⟩,⟨-8,-3⟩] = some ⟨-9,-3⟩

end SourceHull
