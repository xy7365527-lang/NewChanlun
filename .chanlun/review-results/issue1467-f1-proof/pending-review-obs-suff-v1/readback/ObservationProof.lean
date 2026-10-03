import ObservationSpec

namespace ObservationResearch

theorem factorization : FACTORIZATION_TARGET := by
  classical
  intro X S A obs target
  constructor
  · intro ⟨f, hf⟩ x y hxy
    have hs : (⟨obs x, ⟨x,rfl⟩⟩ : Observed obs) = ⟨obs y, ⟨y,rfl⟩⟩ :=
      Subtype.ext hxy
    exact (hf x).symm.trans ((congrArg f hs).trans (hf y))
  · intro h
    refine ⟨fun s => target (Classical.choose s.property), ?_⟩
    intro x
    exact h _ x (Classical.choose_spec (show ∃ y, obs y = obs x from ⟨x,rfl⟩))

theorem collision_obstruction : COLLISION_TARGET := by
  intro X S A obs target x y hobs htarget hfactor
  exact htarget ((factorization X S A obs target).mp hfactor x y hobs)

theorem exact_root : FACTORIZATION_TARGET ∧ COLLISION_TARGET :=
  ⟨factorization, collision_obstruction⟩

end ObservationResearch
