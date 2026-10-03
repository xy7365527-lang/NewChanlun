import TurnAuditSpec

namespace TurnAudit

open NewChanlun.Origin

theorem no_turn_without_payload_div (T : TrendInstance)
    (h : ¬ IsDivergence T.divPair) (succ : TrendInstance → Option TrendInstance) :
    ¬ Turn_q succ T := by
  intro ht
  exact h ht.1

theorem coverage : COVERAGE_TARGET := by
  refine ⟨no_turn_without_payload_div,by decide,by decide,by decide,?_,?_⟩
  · exact ⟨nextQ,rfl,rfl⟩
  · exact no_turn_without_payload_div qNoDiv (by decide) suppliedNext

theorem self : SELF_TARGET := by
  refine ⟨?_,?_,rfl⟩
  · intro T _
    exact ⟨T,rfl,rfl⟩
  · exact ⟨by decide,T1,rfl,rfl⟩

theorem no_forward_self (a : Located) : ¬ Forward a a := by
  intro h
  have ht := h.1
  have he := h.2.1
  omega

theorem different_occurrence : Forward firstOccurrence laterOccurrence := by
  unfold Forward
  decide

theorem no_payload_judge :
    ¬ ∃ judge : TrendInstance → TrendInstance → Bool,
      ∀ a b : Located, judge a.payload b.payload = decide (Forward a b) := by
  rintro ⟨judge,h⟩
  have hf := h firstOccurrence laterOccurrence
  have hs := h firstOccurrence firstOccurrence
  have yes : decide (Forward firstOccurrence laterOccurrence) = true :=
    decide_eq_true different_occurrence
  have no : decide (Forward firstOccurrence firstOccurrence) = false :=
    decide_eq_false (no_forward_self firstOccurrence)
  rw [yes] at hf
  rw [no] at hs
  change judge T1 T1 = true at hf
  change judge T1 T1 = false at hs
  have bad : true = false := hf.symm.trans hs
  cases bad

theorem identity : IDENTITY_TARGET :=
  ⟨no_forward_self,different_occurrence,rfl,no_payload_judge⟩

theorem exact_root : COVERAGE_TARGET ∧ SELF_TARGET ∧ IDENTITY_TARGET :=
  ⟨coverage,self,identity⟩

end TurnAudit

#print axioms TurnAudit.exact_root
