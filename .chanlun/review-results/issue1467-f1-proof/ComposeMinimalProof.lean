import ComposeMinimalSpec

namespace ComposeMinimal

open Formal.RecursiveConstruction

theorem source_sound :
    PairwiseRel (fun m w => UpperMoveSound lower 0 m w) uppers witnesses :=
  ⟨⟨by decide,by decide,rfl⟩,⟨by decide,by decide,rfl⟩,True.intro⟩

theorem witness : WITNESS_TARGET := by
  have wf : ∀ m ∈ uppers, WellFormed m := by
    intro m hm
    simp only [uppers,List.mem_cons,List.not_mem_nil,or_false] at hm
    rcases hm with rfl | rfl
    · exact ComposeAudit.o0_wf
    · exact ComposeAudit.o1_wf
  have relation : MovesComposedFrom lower 0 uppers :=
    ⟨by simp [uppers],wf,by decide,witnesses,rfl,
      by change 0 < 1 ∧ True; decide,source_sound⟩
  exact ⟨CenterFrame.witnessNine,rfl,rfl,relation,source_sound,
    by decide,by decide,by decide⟩

theorem lower_bound : LOWER_BOUND_TARGET := by
  intro xs ys lvl ht h
  have count := h.2.2.1
  omega

theorem exact_root : WITNESS_TARGET ∧ LOWER_BOUND_TARGET := ⟨witness,lower_bound⟩

end ComposeMinimal

#print axioms ComposeMinimal.exact_root
