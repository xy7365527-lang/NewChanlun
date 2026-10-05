import Std

namespace Stage67

def NeedsEarlierClosed (α : Type) (rank : α → Nat) (closed : α → Prop) : Prop :=
  ∀ x, closed x → ∃ b, closed b ∧ rank b < rank x

theorem no_closed_if_all_need_earlier_closed
    (α : Type) (rank : α → Nat) (closed : α → Prop)
    (h : NeedsEarlierClosed α rank closed) : ∀ x, ¬ closed x := by
  have step : ∀ n, ∀ x, rank x = n → ¬ closed x := by
    intro n
    induction n using Nat.strong_induction_on with
    | h n ih =>
      intro x hx hc
      obtain ⟨b, hb, hlt⟩ := h x hc
      exact ih (rank b) (by simpa [hx] using hlt) b rfl hb
  intro x
  exact step (rank x) x rfl

def RawClosed (n : Nat) : Prop := n = 0
def MovementClosed (n : Nat) : Prop := n = 1

theorem raw_root_boundary_model :
    MovementClosed 1 ∧
    (∀ x, MovementClosed x → ∃ b, RawClosed b ∧ b < x ∧ ¬ MovementClosed b) := by
  constructor
  · rfl
  · intro x hx
    have hx' : x = 1 := hx
    subst x
    exact ⟨0, rfl, by decide, by decide⟩

end Stage67

#print Stage67.no_closed_if_all_need_earlier_closed
#print axioms Stage67.no_closed_if_all_need_earlier_closed
#print Stage67.raw_root_boundary_model
#print axioms Stage67.raw_root_boundary_model
