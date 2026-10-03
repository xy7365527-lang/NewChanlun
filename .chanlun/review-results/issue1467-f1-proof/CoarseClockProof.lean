import CoarseClockSpec

namespace CoarseClock

theorem strict_clocks (b : Bool) : Strict (ClockEdge.clock b) :=
  ClockEdge.clock_changes_force.2.1 b

theorem records_equal : record (ClockEdge.clock true) = record (ClockEdge.clock false) := by
  decide

theorem fine_true : Div (ClockEdge.clock true) :=
  ClockEdge.clock_changes_force.2.2.2.2.2.2.1

theorem fine_false : ¬ Div (ClockEdge.clock false) :=
  ClockEdge.clock_changes_force.2.2.2.2.2.2.2

theorem decisions : decision (ClockEdge.clock true) = true ∧
    decision (ClockEdge.clock false) = false := by
  constructor
  · simp [decision,fine_true]
  · simp [decision,fine_false]

theorem compat_true : Compatible shared (ClockEdge.clock true) :=
  ⟨strict_clocks true,rfl⟩
theorem compat_false : Compatible shared (ClockEdge.clock false) :=
  ⟨strict_clocks false,records_equal.symm⟩

theorem witness : WITNESS_TARGET := by
  refine ⟨ClockEdge.whole,rfl,strict_clocks,?_,records_equal,compat_true,compat_false,decisions.1,decisions.2⟩
  intro b
  cases b <;> decide

theorem ambiguous : AMBIGUITY_TARGET :=
  ⟨⟨ClockEdge.clock true,compat_true,fine_true⟩,
   ⟨ClockEdge.clock false,compat_false,fine_false⟩⟩

theorem no_selector : NO_SELECTOR_TARGET := by
  rintro ⟨judge,h⟩
  have ht := h (ClockEdge.clock true) (strict_clocks true)
  have hf := h (ClockEdge.clock false) (strict_clocks false)
  rw [decisions.1,records_equal] at ht
  rw [decisions.2] at hf
  have bad : true = false := ht.symm.trans hf
  cases bad

theorem partial_unknown : PARTIAL_TARGET := by
  intro judge h
  cases hv : judge shared with
  | none => rfl
  | some value =>
    have ht := h (ClockEdge.clock true) (strict_clocks true) value hv
    have hf : judge (record (ClockEdge.clock false)) = some value := by
      rw [← records_equal]
      exact hv
    have hc := h (ClockEdge.clock false) (strict_clocks false) value hf
    rw [decisions.1] at ht
    rw [decisions.2] at hc
    have bad : true = false := ht.symm.trans hc
    cases bad

theorem exact_root : WITNESS_TARGET ∧ AMBIGUITY_TARGET ∧ NO_SELECTOR_TARGET ∧ PARTIAL_TARGET :=
  ⟨witness,ambiguous,no_selector,partial_unknown⟩

end CoarseClock

#print axioms CoarseClock.exact_root
