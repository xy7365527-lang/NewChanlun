import SourceHullSpec

namespace SourceHull

open UpgradeResearch (Interval Valid core3)

theorem range_nodup (s n : Nat) : (List.range' s n).Nodup := by
  induction n generalizing s with
  | zero => simp
  | succ n ih =>
    rw [List.range'_succ]
    apply List.nodup_cons.mpr
    constructor
    · intro h
      rcases List.mem_range'.mp h with ⟨i,hi,he⟩
      omega
    · exact ih (s+1)

theorem cover_data {start finish parts} (h : Cover start finish parts) :
    start ≤ finish ∧ positions parts = List.range' start (finish-start) := by
  induction h with
  | done pos => exact ⟨Nat.le_refl _,by simp [positions]⟩
  | @step p finish rest hlen h ih =>
    refine ⟨by omega,?_⟩
    change List.range' p.start p.len ++ positions rest = _
    rw [ih.2,List.range'_append_1]
    congr 1
    omega

theorem cover : COVER_TARGET := by
  intro start finish parts h
  have hd := cover_data h
  refine ⟨hd.1,hd.2,?_⟩
  rw [hd.2]
  exact range_nodup _ _

theorem singleton (a : Interval) : ExactHull [a] a := by
  refine ⟨?_,⟨a,by simp,rfl⟩,⟨a,by simp,rfl⟩⟩
  intro x hx
  simp only [List.mem_singleton] at hx
  subst x
  exact ⟨by omega,by omega⟩

theorem merged {xs : List Interval} {b : Interval} (h : ExactHull xs b) (a : Interval) :
    ExactHull (a::xs) (merge a b) := by
  refine ⟨?_,?_,?_⟩
  · intro x hx
    rcases List.mem_cons.mp hx with hx | hx
    · subst x
      dsimp [merge]
      omega
    · have hb := h.1 x hx
      dsimp [merge]
      omega
  · rcases h.2.1 with ⟨x,hx,he⟩
    by_cases ha : a.lo ≤ b.lo
    · refine ⟨a,by simp,?_⟩
      dsimp [merge]
      omega
    · refine ⟨x,List.mem_cons_of_mem a hx,?_⟩
      dsimp [merge]
      omega
  · rcases h.2.2 with ⟨x,hx,he⟩
    by_cases ha : b.hi ≤ a.hi
    · refine ⟨a,by simp,?_⟩
      dsimp [merge]
      omega
    · refine ⟨x,List.mem_cons_of_mem a hx,?_⟩
      dsimp [merge]
      omega

theorem none_iff (xs : List Interval) : hull xs = none ↔ xs = [] := by
  cases xs with
  | nil => simp [hull]
  | cons a xs => cases hx : hull xs <;> simp [hull,hx]

theorem hull_sound (xs : List Interval) : ∀ h, hull xs = some h → ExactHull xs h := by
  induction xs with
  | nil => intro h hh; simp [hull] at hh
  | cons a xs ih =>
    intro h hh
    cases hx : hull xs with
    | none =>
      have hn := (none_iff xs).mp hx
      subst xs
      have he : a = h := Option.some.inj (by simpa [hull] using hh)
      subst h
      exact singleton a
    | some b =>
      have he : merge a b = h := Option.some.inj (by simpa [hull,hx] using hh)
      subst h
      exact merged (ih b hx) a

theorem unique {xs : List Interval} {a b : Interval}
    (ha : ExactHull xs a) (hb : ExactHull xs b) : a = b := by
  have hba : b.lo ≤ a.lo := by
    rcases ha.2.1 with ⟨x,hx,he⟩
    have hc := hb.1 x hx
    omega
  have hab : a.lo ≤ b.lo := by
    rcases hb.2.1 with ⟨x,hx,he⟩
    have hc := ha.1 x hx
    omega
  have habh : a.hi ≤ b.hi := by
    rcases ha.2.2 with ⟨x,hx,he⟩
    have hc := hb.1 x hx
    omega
  have hbah : b.hi ≤ a.hi := by
    rcases hb.2.2 with ⟨x,hx,he⟩
    have hc := ha.1 x hx
    omega
  have hlo : a.lo = b.lo := by omega
  have hhi : a.hi = b.hi := by omega
  cases a
  cases b
  simp_all

theorem hull_contract : HULL_TARGET := by
  refine ⟨none_iff,?_,?_,?_⟩
  · intro xs h
    constructor
    · exact hull_sound xs h
    · intro he
      cases hv : hull xs with
      | none =>
        have hx := (none_iff xs).mp hv
        subst xs
        rcases he.2.1 with ⟨x,hx,_⟩
        simp at hx
      | some g =>
        have hg := unique (hull_sound xs g hv) he
        simpa [hg] using hv
  · intro xs a b ha hb
    exact unique ha hb
  · intro xs h hh hv
    have he := hull_sound xs h hh
    rcases he.2.1 with ⟨x,hx,hlo⟩
    have hc := he.1 x hx
    have hp := hv x hx
    unfold Valid at *
    omega

theorem witness : WITNESS_TARGET := by
  have bad : ¬ Cover 0 6 badParts := by
    intro h
    have hn : positions badParts ≠ List.range' 0 6 := by decide
    exact hn (by simpa using (cover_data h).2)
  have kept : Cover 0 9 keptNine :=
    Cover.step ⟨.candidate,0,3⟩ (by decide)
      (Cover.step ⟨.candidate,3,3⟩ (by decide)
        (Cover.step ⟨.retained,6,3⟩ (by decide) (Cover.done 9)))
  have tail : Cover 0 11 withTail :=
    Cover.step ⟨.candidate,0,3⟩ (by decide)
      (Cover.step ⟨.candidate,3,3⟩ (by decide)
        (Cover.step ⟨.retained,6,3⟩ (by decide)
          (Cover.step ⟨.active,9,2⟩ (by decide) (Cover.done 11))))
  refine ⟨ComposeMinimal.witness,ComposeAudit.range,bad,kept,tail,
    by decide,by decide,by decide,by decide,?_,rfl,by decide⟩
  unfold Valid core3
  decide

theorem exact_root : COVER_TARGET ∧ HULL_TARGET ∧ WITNESS_TARGET :=
  ⟨cover,hull_contract,witness⟩

end SourceHull

#print axioms SourceHull.exact_root
