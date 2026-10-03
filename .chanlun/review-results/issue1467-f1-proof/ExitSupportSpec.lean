import ExitLiftProof

namespace ExitSupport

def ContainsSpan (outer inner : SourceHull.Part) : Prop :=
  outer.start ≤ inner.start ∧ inner.start+inner.len ≤ outer.start+outer.len
instance (a b : SourceHull.Part) : Decidable (ContainsSpan a b) := by
  unfold ContainsSpan
  infer_instance

def NoReuse (a b c : SourceHull.Part) : Prop :=
  a.start+a.len ≤ b.start ∧ b.start+b.len ≤ c.start

def NONEMPTY_TARGET : Prop :=
  ContainsSpan ExitLift.oldPart ExitLift.oldPart ∧
  ContainsSpan ExitLift.nextPart ExitLift.nextPart ∧
  NoReuse ExitLift.oldPart ExitLift.gapPart ExitLift.nextPart

def GROW_TARGET : Prop :=
  ∀ a b c : SourceHull.Part,
    ContainsSpan a ExitLift.oldPart → ContainsSpan c ExitLift.nextPart →
    NoReuse a b c → b.len ≤ 1

def SPLIT_TARGET : Prop :=
  (ExitLift.packets.map (·.part)).any
    (fun p => decide (ContainsSpan p ExitLift.nextPart)) = false

end ExitSupport
