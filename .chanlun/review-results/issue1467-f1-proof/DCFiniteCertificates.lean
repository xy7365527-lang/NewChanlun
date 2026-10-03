import DCSemantics

namespace DCFinite
open DCSpec

def drop (p : Nat → Int) (delta : Int) (d : Dir) (s t : Nat) : Prop :=
  ∃ k : Fin t, s ≤ k.val ∧ delta ≤ score d (p k.val) - score d (p t)
def wide (p : Nat → Int) (delta : Int) (t : Nat) : Prop :=
  ∃ a b : Fin (t+1), delta ≤ p a.val - p b.val
def extreme (p : Nat → Int) (d : Dir) (s r e : Nat) : Prop :=
  s ≤ e ∧ e ≤ r ∧
  (∀ j : Fin (r+1), s ≤ j.val → score d (p j.val) ≤ score d (p e)) ∧
  ∀ j : Fin (r+1), s ≤ j.val → j.val < e → p j.val ≠ p e
def firstDrop (p : Nat → Int) (delta : Int) (d : Dir) (s t : Nat) : Prop :=
  drop p delta d s t ∧ ∀ j : Fin t, ¬ drop p delta d s j.val
def firstWide (p : Nat → Int) (delta : Int) (t : Nat) : Prop :=
  wide p delta t ∧ ∀ j : Fin t, ¬ wide p delta j.val
def noDrop (p : Nat → Int) (delta : Int) (d : Dir) (s n : Nat) : Prop :=
  ∀ j : Fin (n+1), ¬ drop p delta d s j.val

instance (p) (delta) (d) (s t) : Decidable (drop p delta d s t) := by unfold drop; infer_instance
instance (p) (delta) (t) : Decidable (wide p delta t) := by unfold wide; infer_instance
instance (p) (d) (s r e) : Decidable (extreme p d s r e) := by unfold extreme; infer_instance
instance (p) (delta) (d) (s t) : Decidable (firstDrop p delta d s t) := by unfold firstDrop; infer_instance
instance (p) (delta) (t) : Decidable (firstWide p delta t) := by unfold firstWide; infer_instance
instance (p) (delta) (d) (s n) : Decidable (noDrop p delta d s n) := by unfold noDrop; infer_instance

theorem drop_iff (p) (delta) (d) (s t) : drop p delta d s t ↔ Drop p delta d s t := by
  constructor
  · intro ⟨k,hs,hv⟩
    exact ⟨by have hk:=k.isLt; omega,k.val,hs,k.isLt,hv⟩
  · intro ⟨_,k,hs,ht,hv⟩
    exact ⟨⟨k,ht⟩,hs,hv⟩

theorem wide_iff (p) (delta) (t) : wide p delta t ↔ Wide p delta t := by
  constructor
  · intro ⟨a,b,h⟩
    exact ⟨a.val,b.val,by have ha:=a.isLt; omega,by have hb:=b.isLt; omega,h⟩
  · intro ⟨a,b,ha,hb,h⟩
    exact ⟨⟨a,by omega⟩,⟨b,by omega⟩,h⟩

theorem extreme_sound {p d s r e} (h : extreme p d s r e) : Extreme p d s r e := by
  refine ⟨h.1,h.2.1,?_,?_⟩
  · intro j hs hr
    exact h.2.2.1 ⟨j,by omega⟩ hs
  · intro j hs he
    exact h.2.2.2 ⟨j,by have her:=h.2.1; omega⟩ hs he

theorem firstDrop_sound {p delta d s t} (h : firstDrop p delta d s t) : First (Drop p delta d s) t := by
  refine ⟨(drop_iff p delta d s t).mp h.1, ?_⟩
  intro j hj hd
  exact h.2 ⟨j,hj⟩ ((drop_iff p delta d s j).mpr hd)

theorem firstWide_sound {p delta t} (h : firstWide p delta t) : First (Wide p delta) t := by
  refine ⟨(wide_iff p delta t).mp h.1, ?_⟩
  intro j hj hd
  exact h.2 ⟨j,hj⟩ ((wide_iff p delta j).mpr hd)

theorem noDrop_sound {p delta d s n} (h : noDrop p delta d s n) :
    ∀ t, t ≤ n → ¬ Drop p delta d s t := by
  intro t ht hd
  exact h ⟨t,by omega⟩ ((drop_iff p delta d s t).mpr hd)

end DCFinite
