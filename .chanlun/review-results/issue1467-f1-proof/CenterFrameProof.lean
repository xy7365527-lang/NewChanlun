import CenterFrameSpec
import DCFiniteCertificates

namespace CenterFrame

open NewChanlun.Origin
open CenterAttempt (ObservedUnit Joined obs visible)

theorem recorded_extend (f : Frame) (l r : ObservedUnit) : Recorded f l r (extend f l r) :=
  ⟨rfl,rfl,rfl,rfl,rfl,rfl⟩

theorem recorded_eq {f g : Frame} {l r : ObservedUnit} (h : Recorded f l r g) : g = extend f l r := by
  cases f
  cases g
  simp_all [Recorded, extend]

theorem absorb_unique {side : Side} {f : Frame} {l r : ObservedUnit} {o₁ o₂ : Outcome}
    (h₁ : Absorb side f l r o₁) (h₂ : Absorb side f l r o₂) : o₁ = o₂ := by
  cases o₁ with
  | active g₁ =>
    cases o₂ with
    | active g₂ =>
      rw [recorded_eq h₁.2.2.1, recorded_eq h₂.2.2.1]
    | retilePending g₂ =>
      have a := h₁.2.2.2
      have b := h₂.2.2.2
      rw [recorded_eq h₁.2.2.1] at a
      rw [recorded_eq h₂.2.2.1] at b
      omega
  | retilePending g₁ =>
    cases o₂ with
    | active g₂ =>
      have a := h₁.2.2.2
      have b := h₂.2.2.2
      rw [recorded_eq h₁.2.2.1] at a
      rw [recorded_eq h₂.2.2.1] at b
      omega
    | retilePending g₂ =>
      rw [recorded_eq h₁.2.2.1, recorded_eq h₂.2.2.1]

theorem no_success_of_returned {side : Side} {f : Frame} {l r : ObservedUnit}
    (h : Returned side f l r) : ¬ pair side f l r := by
  intro hp
  cases side <;> simp only [Returned,pair] at h hp <;>
    rcases h with ⟨_,_,_,_,hlo,hhi⟩ <;>
    rcases hp with ⟨_,_,_,_,hs⟩ <;>
    simp only [Tick] at * <;> omega

theorem update_contract : UPDATE_TARGET := by
  refine ⟨?_,?_,?_,?_⟩
  · intro a b c leave ret t
    simp [pair,PairBase,initial,right,zg,CenterAttempt.UpPair,and_assoc]
  · intro side f l r hret hcount
    by_cases hg : (members (extend f l r)).length < 9
    · have hs : Absorb side f l r (.active (extend f l r)) :=
        ⟨hret,hcount,recorded_extend f l r,hg⟩
      exact ⟨_,hs,fun other ho => absorb_unique ho hs⟩
    · have hn : 9 ≤ (members (extend f l r)).length := by omega
      have hs : Absorb side f l r (.retilePending (extend f l r)) :=
        ⟨hret,hcount,recorded_extend f l r,hn⟩
      exact ⟨_,hs,fun other ho => absorb_unique ho hs⟩
  · intro side f l r out h
    have heq : frameOf out = extend f l r := by
      cases out <;> exact recorded_eq h.2.2.1
    dsimp
    rw [heq]
    refine ⟨by simp [members,extend,List.append_assoc],rfl,rfl,?_,?_,?_,?_⟩
    · simp only [extend]; omega
    · simp only [extend]; omega
    · simp only [extend]; omega
    · intro pre post
      simp [members,extend,List.append_assoc]
  · intro side f l r h
    exact no_success_of_returned h

theorem witnessB : B_TARGET := by
  have hr : Returned .long b0 (obs CenterAttempt.pathB CenterAttempt.leaveA) (obs CenterAttempt.pathB CenterAttempt.retB) := by
    unfold Returned PairBase Joined CenterConfirmedComplete DirAlternates
    decide
  have hs : Absorb .long b0 (obs CenterAttempt.pathB CenterAttempt.leaveA) (obs CenterAttempt.pathB CenterAttempt.retB) (.active b1) :=
    ⟨hr,by decide,recorded_extend _ _ _,by decide⟩
  refine ⟨CenterAttempt.wholeB,hs,rfl,rfl,rfl,rfl,no_success_of_returned hr,?_,?_⟩
  · unfold pair PairBase Joined CenterConfirmedComplete DirAlternates
    decide
  · unfold pair PairBase Joined CenterConfirmedComplete DirAlternates
    decide

theorem wholeC : DCSpec.Whole pathC 2 11 resultC := by
  apply DCSpec.Whole.started (n:=10) (d:=.down) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide), DCFinite.extreme_sound (by decide), by decide⟩
  · refine DCSpec.Run.next (n:=10) (d:=.down) (s:=0) (e:=1) (t:=2) (rest:=(unitsC.drop 1,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=1) (e:=2) (t:=3) (rest:=(unitsC.drop 2,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=2) (e:=3) (t:=4) (rest:=(unitsC.drop 3,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=3) (e:=4) (t:=5) (rest:=(unitsC.drop 4,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=4) (e:=5) (t:=6) (rest:=(unitsC.drop 5,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=5) (e:=6) (t:=7) (rest:=(unitsC.drop 6,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=6) (e:=7) (t:=8) (rest:=(unitsC.drop 7,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=7) (e:=8) (t:=9) (rest:=(unitsC.drop 8,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=8) (e:=9) (t:=10) (rest:=(unitsC.drop 9,tailC)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.up) (s:=9) (e:=10) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem witnessNine : NINE_TARGET := by
  have h0 : Returned .long f0 (obs pathC c3) (obs pathC c4) := by
    unfold Returned PairBase Joined CenterConfirmedComplete DirAlternates; decide
  have h1 : Returned .long f1 (obs pathC c5) (obs pathC c6) := by
    unfold Returned PairBase Joined CenterConfirmedComplete DirAlternates; decide
  have h2 : Returned .long f2 (obs pathC c7) (obs pathC c8) := by
    unfold Returned PairBase Joined CenterConfirmedComplete DirAlternates; decide
  have s0 : Absorb .long f0 (obs pathC c3) (obs pathC c4) (.active f1) :=
    ⟨h0,by decide,recorded_extend _ _ _,by decide⟩
  have s1 : Absorb .long f1 (obs pathC c5) (obs pathC c6) (.active f2) :=
    ⟨h1,by decide,recorded_extend _ _ _,by decide⟩
  have s2 : Absorb .long f2 (obs pathC c7) (obs pathC c8) (.retilePending f3) :=
    ⟨h2,by decide,recorded_extend _ _ _,by decide⟩
  have noactive : ¬ ∃ g, Absorb .long f2 (obs pathC c7) (obs pathC c8) (.active g) := by
    intro ⟨g,hg⟩
    have hc := hg.2.2.2
    rw [recorded_eq hg.2.2.1] at hc
    change 9 < 9 at hc
    omega
  exact ⟨wholeC,s0,s1,s2,noactive,rfl,rfl,rfl,rfl,rfl,rfl,rfl,rfl,by decide⟩

theorem exact_root : UPDATE_TARGET ∧ B_TARGET ∧ NINE_TARGET :=
  ⟨update_contract,witnessB,witnessNine⟩

end CenterFrame

#print axioms CenterFrame.exact_root
