import Std

/- #1467：依赖信息时刻与端点事实的局部约束，非完整F₁证明。 -/
namespace OrderBaseClockGeometry

def latest (times : List Nat) : Nat := times.foldr Nat.max 0

/-- 所有组成信息已出现，当且仅当这些时刻的最大值不晚于当前时刻。 -/
theorem all_available_iff_latest (times : List Nat) (t : Nat) :
    latest times ≤ t ↔ ∀ x ∈ times, x ≤ t := by
  induction times with
  | nil => simp [latest]
  | cons a rest ih =>
    simp only [latest, List.foldr, List.mem_cons] at *
    constructor
    · intro h x hx
      rcases hx with hx | hx
      · subst x
        exact Nat.le_trans (Nat.le_max_left _ _) h
      · exact ih.mp (Nat.le_trans (Nat.le_max_right _ _) h) x hx
    · intro h
      have ha : a ≤ t := h a (Or.inl rfl)
      have hr : rest.foldr Nat.max 0 ≤ t := ih.mpr (fun x hx => h x (Or.inr hx))
      exact Nat.max_le_of_le_of_le ha hr

/-- 组成信息可用只是构造的必要下界，不能把较早的几何端点当确认时刻。 -/
theorem dependency_clock_lower_bound (times : List Nat) (observedAt : Nat)
    (h : ∀ x ∈ times, x ≤ observedAt) : latest times ≤ observedAt :=
  (all_available_iff_latest times observedAt).mpr h

def touchedPrice (i : Nat) : Nat := if i = 0 then 102 else 100

/-- 两个真实触价按102→100出现，不能倒排成100→102的有序端点见证。 -/
theorem no_upward_endpoint_witness_in_descending_touches (i j : Nat)
    (hi : i < 2) (_hj : j < 2)
    (hstart : touchedPrice i = 100) (hend : touchedPrice j = 102) : ¬ i < j := by
  have hi1 : i = 1 := by
    by_cases h : i = 0
    · simp [touchedPrice, h] at hstart
    · omega
  have hj0 : j = 0 := by
    by_cases h : j = 0
    · exact h
    · simp [touchedPrice, h] at hend
  omega

theorem both_extrema_are_observed : touchedPrice 0 = 102 ∧ touchedPrice 1 = 100 := by
  decide

end OrderBaseClockGeometry

#print axioms OrderBaseClockGeometry.dependency_clock_lower_bound
#print axioms OrderBaseClockGeometry.no_upward_endpoint_witness_in_descending_touches
