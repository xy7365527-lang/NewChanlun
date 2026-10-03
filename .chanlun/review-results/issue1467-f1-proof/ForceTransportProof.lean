import ForceTransportSpec

namespace ForceTransport

open NewChanlun.Origin

theorem last_append (a b : Stroke) (as bs : List Stroke) :
    lastStroke a (as ++ b :: bs) = lastStroke b bs := by
  induction as generalizing a with
  | nil => rfl
  | cons x xs ih => exact ih x

theorem read_summary (xs : List Stroke) : read (summarize xs) = impulse xs := by
  cases xs <;> rfl

theorem summary_append (xs ys : List Stroke) :
    summarize (xs ++ ys) = join (summarize xs) (summarize ys) := by
  cases xs with
  | nil => rfl
  | cons a as =>
    cases ys with
    | nil => simp [summarize, join]
    | cons b bs => simp only [List.cons_append, summarize, join, last_append]

theorem join_associative (a b c : Option Ends) :
    join (join a b) c = join a (join b c) := by
  cases a <;> cases b <;> cases c <;> rfl

theorem grouped_summary (parts : List (List Stroke)) :
    grouped parts = summarize parts.flatten := by
  induction parts with
  | nil => rfl
  | cons xs rest ih =>
    change join (summarize xs) (grouped rest) = summarize (xs ++ rest.flatten)
    rw [ih, summary_append]

theorem grouped_impulse (parts : List (List Stroke)) :
    read (grouped parts) = impulse parts.flatten := by
  rw [grouped_summary, read_summary]

theorem composition : COMPOSITION_TARGET := by
  refine ⟨read_summary, summary_append, join_associative, grouped_summary, grouped_impulse, ?_⟩
  intro p q h
  rw [grouped_impulse, grouped_impulse, h]

theorem seam : SEAM_TARGET := by
  intro a b as bs
  simp only [List.cons_append, impulse, last_append]
  symm
  simp only [Rat.sub_eq_add_neg]
  calc
    _ = ((lastStroke a as).velocity + -(lastStroke a as).velocity) +
        (b.velocity + -b.velocity) + ((lastStroke b bs).velocity + -a.velocity) := by ac_rfl
    _ = _ := by simp only [Rat.add_neg_cancel, Rat.zero_add]

theorem singleton_zero (s : Stroke) : impulse [s] = 0 := by
  simp [impulse, lastStroke, Rat.sub_self]

theorem witnesses : WITNESS_TARGET := by
  simp only [WITNESS_TARGET, ValidChain, ValidStroke, Joined, Stroke.WellFormed,
    leftSlow, leftFast, right]
  decide +kernel

theorem scalar_insufficient :
    ¬ ∃ merge : Rat → Rat → Rat,
      ∀ (a b : List Stroke), a ≠ [] → b ≠ [] → ValidChain (a ++ b) →
        merge (impulse a) (impulse b) = impulse (a ++ b) := by
  rintro ⟨merge,h⟩
  have slow := h [leftSlow] [right] (by simp) (by simp) witnesses.1
  have fast := h [leftFast] [right] (by simp) (by simp) witnesses.2.1
  rw [singleton_zero, singleton_zero] at slow fast
  have slowValue : impulse ([leftSlow] ++ [right]) = -3 := by decide +kernel
  have fastValue : impulse ([leftFast] ++ [right]) = -5 := by decide +kernel
  rw [slowValue] at slow
  rw [fastValue] at fast
  have bad : (-3 : Rat) = -5 := slow.symm.trans fast
  exact (by decide : (-3 : Rat) ≠ -5) bad

theorem loss : LOSS_TARGET := ⟨singleton_zero,scalar_insufficient⟩

theorem exact_root : COMPOSITION_TARGET ∧ SEAM_TARGET ∧ LOSS_TARGET ∧ WITNESS_TARGET :=
  ⟨composition,seam,loss,witnesses⟩

end ForceTransport

#print axioms ForceTransport.exact_root
