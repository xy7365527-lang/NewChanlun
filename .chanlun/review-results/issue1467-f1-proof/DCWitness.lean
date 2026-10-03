import DCCausalProof

namespace DCSpec

def witnessPath (i : Nat) : Int := if i = 1 then 3 else 0
def witnessResult : Result :=
  .started .up 0 1 ([⟨.up,0,1,2⟩], ⟨.down,1,2⟩)

/-- [0,3,0]、δ=2的一个完成上行单元，确认在2而非端点1。 -/
theorem nonempty_witness : Whole witnessPath 2 3 witnessResult := by
  apply Whole.started (n:=2) (d:=.up) (s:=0) (c:=1) (by omega)
  · refine ⟨⟨⟨1,0,by omega,by omega,by decide⟩, ?_⟩, ?_, by decide⟩
    · intro j hj hw
      obtain ⟨a,b,ha,hb,hv⟩ := hw
      have hA : a = 0 := by omega
      have hB : b = 0 := by omega
      rw [hA,hB] at hv
      omega
    · refine ⟨by omega, by omega, ?_, ?_⟩
      · intro j hj hJ
        by_cases h : j = 1
        · simp [h, score, flip, witnessPath]
        · have hz : j = 0 := by omega
          simp [hz, score, flip, witnessPath]
      · intro j hj hJ; omega
  · apply Run.next (n:=2) (d:=.up) (s:=0) (e:=1) (t:=2)
      (rest:=([], ⟨.down,1,2⟩)) (by omega)
    · refine ⟨⟨by omega,1,by omega,by omega,by decide⟩, ?_⟩
      intro j hj hdrop
      obtain ⟨hst,k,hk,hkj,hv⟩ := hdrop
      have hJ : j = 1 := by omega
      have hK : k = 0 := by omega
      simp [hJ,hK,score,witnessPath] at hv
    · refine ⟨by omega,by omega,?_,?_⟩
      · intro j hj hJ
        by_cases h : j = 1
        · simp [h,score,witnessPath]
        · have hz : j = 0 := by omega
          simp [hz,score,witnessPath]
      · intro j hj hJ
        have hz : j = 0 := by omega
        simp [hz,witnessPath]
    · omega
    · apply Run.stop
      · intro t ht hdrop
        obtain ⟨hst,k,hk,hkt,hv⟩ := hdrop
        have hT : t = 2 := by omega
        have hK : k = 1 := by omega
        simp [hT,hK,score,flip,witnessPath] at hv
      · refine ⟨by omega,by omega,?_,?_⟩
        · intro j hj hJ
          by_cases h : j = 1
          · simp [h,score,flip,witnessPath]
          · have hz : j = 2 := by omega
            simp [hz,score,flip,witnessPath]
        · intro j hj hJ
          have hz : j = 1 := by omega
          simp [hz,witnessPath]

theorem nonempty_witness_has_one_unit : (units witnessResult).length = 1 := rfl

end DCSpec

#print axioms DCSpec.nonempty_witness
