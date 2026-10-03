import ComposeAuditSpec

namespace ComposeAudit

open Formal.RecursiveConstruction

theorem pack_base (p : Nat → Int) (a b c : DCSpec.Unit)
    (h : WindowOverlap (embed p a) (embed p b) (embed p c)) :
    WellFormed (pack3 0 (embed p a) (embed p b) (embed p c)) :=
  window_wellFormed _ _ _ 0 h rfl rfl rfl
    (by simp [embed,WellFormed]) (by simp [embed,WellFormed]) (by simp [embed,WellFormed])

theorem o0_wf : WellFormed o0 := pack_base _ _ _ _ (by decide)
theorem o1_wf : WellFormed o1 := pack_base _ _ _ _ (by decide)
theorem o2_wf : WellFormed o2 := pack_base _ _ _ _ (by decide)

theorem overlap_wf : ∀ m ∈ overlapUppers, WellFormed m := by
  intro m hm
  simp only [overlapUppers,List.mem_cons,List.not_mem_nil,or_false] at hm
  rcases hm with rfl | rfl | rfl
  · exact o0_wf
  · exact o1_wf
  · exact o2_wf

theorem overlap_sound :
    PairwiseRel (fun m w => UpperMoveSound lowerC 0 m w) overlapUppers overlapWitnesses := by
  exact ⟨⟨by decide,by decide,rfl⟩,⟨by decide,by decide,rfl⟩,
    ⟨by decide,by decide,rfl⟩,True.intro⟩

theorem overlap_relation : MovesComposedFrom lowerC 0 overlapUppers := by
  exact ⟨by simp [overlapUppers],overlap_wf,by decide,
    overlapWitnesses,rfl,by change 0 < 1 ∧ 1 < 2 ∧ True; decide,overlap_sound⟩

theorem overlap : OVERLAP_TARGET := by
  exact ⟨CenterFrame.witnessNine,rfl,overlap_relation,overlap_sound,
    by decide,by decide,by decide,by decide⟩

theorem r0_wf : WellFormed r0 := pack_base _ _ _ _ (by decide)
theorem r1_wf : WellFormed r1 := pack_base _ _ _ _ (by decide)
theorem r2_wf : WellFormed r2 := pack_base _ _ _ _ (by decide)

theorem range_wf : ∀ m ∈ rangeUppers, WellFormed m := by
  intro m hm
  simp only [rangeUppers,List.mem_cons,List.not_mem_nil,or_false] at hm
  rcases hm with rfl | rfl | rfl
  · exact r0_wf
  · exact r1_wf
  · exact r2_wf

theorem false_parent_wf : WellFormed falseParent :=
  window_wellFormed r0 r1 r2 1 (by decide) rfl rfl rfl r0_wf r1_wf r2_wf

theorem range : RANGE_TARGET := by
  refine ⟨LiftBoundary.witness,range_wf,by decide,by decide,by decide,
    false_parent_wf,by decide,?_⟩
  unfold LiftBoundary.strictLift
  change ¬ UpgradeResearch.Valid (⟨10040,10040⟩ : UpgradeResearch.Interval)
  unfold UpgradeResearch.Valid
  decide

theorem exact_root : OVERLAP_TARGET ∧ RANGE_TARGET := ⟨overlap,range⟩

end ComposeAudit

#print axioms ComposeAudit.exact_root
