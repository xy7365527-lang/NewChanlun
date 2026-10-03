import Std

/- #1467：R_W-DC独立首次命中/极值语义的基础引理，非Rust全程序精化证明。 -/
namespace OrderBaseDC

def FirstHit (P : Nat → Prop) (k : Nat) : Prop := P k ∧ ∀ j, j < k → ¬ P j

theorem first_hit_unique (P : Nat → Prop) (k l : Nat)
    (hk : FirstHit P k) (hl : FirstHit P l) : k = l := by
  by_cases hkl : k < l
  · exact False.elim (hl.2 k hkl hk.1)
  · by_cases hlk : l < k
    · exact False.elim (hk.2 l hlk hl.1)
    · omega

def MaxSpec (p : Nat → Int) (n k : Nat) : Prop :=
  k ≤ n ∧ (∀ j, j ≤ n → p j ≤ p k) ∧ ∀ j, j < k → p j ≠ p k

def maxIndex (p : Nat → Int) : Nat → Nat
  | 0 => 0
  | n+1 => if p (maxIndex p n) < p (n+1) then n+1 else maxIndex p n

theorem maxIndex_spec (p : Nat → Int) (n : Nat) : MaxSpec p n (maxIndex p n) := by
  induction n with
  | zero =>
    refine ⟨by simp [maxIndex], ?_, ?_⟩
    · intro j hj
      have h : j = 0 := by omega
      simp [h, maxIndex]
    · intro j hj
      simp [maxIndex] at hj
  | succ n ih =>
    obtain ⟨hk, hm, hf⟩ := ih
    by_cases h : p (maxIndex p n) < p (n+1)
    · simp only [maxIndex, if_pos h]
      refine ⟨by omega, ?_, ?_⟩
      · intro j hj
        by_cases he : j = n+1
        · simp [he]
        · have hjn : j ≤ n := by omega
          have hv := hm j hjn
          omega
      · intro j hj
        have hjn : j ≤ n := by omega
        have hv := hm j hjn
        omega
    · simp only [maxIndex, if_neg h]
      refine ⟨by omega, ?_, hf⟩
      intro j hj
      by_cases he : j = n+1
      · rw [he]
        omega
      · exact hm j (by omega)

theorem max_spec_unique (p : Nat → Int) (n k l : Nat)
    (hk : MaxSpec p n k) (hl : MaxSpec p n l) : k = l := by
  have h1 := hk.2.1 l hl.1
  have h2 := hl.2.1 k hk.1
  have hp : p k = p l := by omega
  by_cases hkl : k < l
  · exact False.elim (hl.2.2 k hkl hp)
  · by_cases hlk : l < k
    · exact False.elim (hk.2.2 l hlk hp.symm)
    · omega

/-- 未初始化区间宽度不足δ时，同一新点不可能同时触发上下两种首方向。 -/
theorem initial_hits_exclusive (lo hi p delta : Int)
    (hd : 0 < delta) (hr : hi-lo < delta) :
    ¬ (delta ≤ p-lo ∧ delta ≤ hi-p) := by omega

end OrderBaseDC

#print axioms OrderBaseDC.first_hit_unique
#print axioms OrderBaseDC.maxIndex_spec
#print axioms OrderBaseDC.max_spec_unique
#print axioms OrderBaseDC.initial_hits_exclusive
