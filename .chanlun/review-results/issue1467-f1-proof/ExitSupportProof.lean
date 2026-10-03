import ExitSupportSpec

namespace ExitSupport

theorem nonempty : NONEMPTY_TARGET := by
  unfold NONEMPTY_TARGET ContainsSpan NoReuse
  decide

theorem cannot_grow_bridge : GROW_TARGET := by
  intro a b c ha hc horder
  have haEnd : 3 ≤ a.start+a.len := ha.2
  have hcStart : c.start ≤ 4 := hc.1
  have hab := horder.1
  have hbc := horder.2
  omega

theorem old_next_split : SPLIT_TARGET := by
  unfold SPLIT_TARGET
  decide

theorem exact_root : ExitLift.PREFIX_TARGET ∧ ExitLift.SOURCE_TARGET ∧
    ExitLift.FROZEN_TARGET ∧ ExitLift.LATE_TARGET ∧
    NONEMPTY_TARGET ∧ GROW_TARGET ∧ SPLIT_TARGET :=
  ⟨ExitLift.prefix_evidence,ExitLift.source_bound,ExitLift.frozen,ExitLift.late,
   nonempty,cannot_grow_bridge,old_next_split⟩

end ExitSupport

#print axioms ExitSupport.exact_root
