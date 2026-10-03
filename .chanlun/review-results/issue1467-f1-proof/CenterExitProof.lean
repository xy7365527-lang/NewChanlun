import CenterExitSpec
import DCFiniteCertificates

namespace CenterExit

open NewChanlun.Origin
open CenterAttempt (ObservedUnit obs visible)

theorem released_release {side f leave ret rest}
    (hp : CenterFrame.pair side f leave ret) (hn : (CenterFrame.members f).length < 9) :
    Released side f leave ret rest (release f leave ret rest) :=
  ⟨hp,hn,rfl,rfl,rfl,rfl,rfl⟩

theorem released_unique {side f leave ret rest a b}
    (ha : Released side f leave ret rest a) (hb : Released side f leave ret rest b) : a = b := by
  rcases ha with ⟨_,_,haf,had,har,hat,has⟩
  rcases hb with ⟨_,_,hbf,hbd,hbr,hbt,hbs⟩
  cases a
  cases b
  simp_all

theorem release_contract : RELEASE_TARGET := by
  refine ⟨?_,?_,?_⟩
  · intro side f leave ret rest hp hn
    refine ⟨release f leave ret rest,released_release hp hn,?_⟩
    intro other ho
    exact released_unique ho (released_release hp hn)
  · intro side f leave ret rest out ho
    rcases ho with ⟨_,_,hf,hd,hr,ht,_⟩
    refine ⟨?_,?_,?_,?_⟩
    · rw [hf,hd,hr]
      simp
    all_goals rw [ht]; omega
  · intro side f leave ret rest out hr he
    exact CenterFrame.no_success_of_returned hr he.1

theorem whole (b : Bool) : DCSpec.Whole (path b) 2 11 result := by
  cases b
  all_goals
    apply DCSpec.Whole.started (n:=10) (d:=.down) (s:=0) (c:=1) (by decide)
    · exact ⟨DCFinite.firstWide_sound (by decide),DCFinite.extreme_sound (by decide),by decide⟩
    · refine DCSpec.Run.next (n:=10) (d:=.down) (s:=0) (e:=1) (t:=2) (rest:=(units.drop 1,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.up) (s:=1) (e:=2) (t:=3) (rest:=(units.drop 2,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.down) (s:=2) (e:=3) (t:=4) (rest:=(units.drop 3,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.up) (s:=3) (e:=4) (t:=5) (rest:=(units.drop 4,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.down) (s:=4) (e:=5) (t:=6) (rest:=(units.drop 5,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.up) (s:=5) (e:=7) (t:=8) (rest:=(units.drop 6,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.down) (s:=7) (e:=8) (t:=9) (rest:=(units.drop 7,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      refine DCSpec.Run.next (n:=10) (d:=.up) (s:=8) (e:=9) (t:=10) (rest:=(units.drop 8,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
      exact DCSpec.Run.stop (d:=.down) (s:=9) (e:=10) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem fork : FORK_TARGET := by
  have quantity : ∀ b, ∀ i : Fin 11, 0 < CenterAttempt.quantity (path b) i.val ∧
      CenterAttempt.projected (CenterAttempt.quantity (path b) i.val) = path b i.val := by
    intro b
    cases b <;> decide
  have hprefix : ∀ i, i ≤ 6 → path true i = path false i := by
    have h : ∀ i : Fin 7, path true i.val = path false i.val := by decide
    intro i hi
    exact h ⟨i,by omega⟩
  have exits : ∀ b, CenterFrame.members (frame b) ++ [leave b,ret b] = visible (path b) result 6 ∧
      Released .long (frame b) (leave b) (ret b) [] (exitAt b) := by
    intro b
    refine ⟨by cases b <;> rfl,?_⟩
    apply released_release
    · cases b <;> unfold CenterFrame.pair CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates <;> decide
    · cases b <;> decide
  refine ⟨whole,quantity,hprefix,rfl,exits,rfl,rfl,rfl,?_,?_,by decide,?_,by decide,?_⟩
  · intro b
    cases b <;> rfl
  · intro b
    unfold Formal.CenterTrichotomy.SameLevelNewCenterPair
    cases b <;> decide
  · unfold Formal.CenterTrichotomy.IsUpContinuation
    decide
  · unfold Formal.CenterTrichotomy.IsLevelExpansion
    decide

theorem no_early_choice : NO_EARLY_CHOICE_TARGET := by
  rintro ⟨choose,ht,he⟩
  have hview : visible (path true) result 6 = visible (path false) result 6 := rfl
  have htrend : Formal.CenterTrichotomy.classify (initialCenter true) (nextCenter true) = .upContinuation := by decide
  have hexpand : Formal.CenterTrichotomy.classify (initialCenter false) (nextCenter false) = .levelExpansion := by decide
  rw [htrend] at ht
  rw [hexpand] at he
  rw [hview] at ht
  have bad : Formal.CenterTrichotomy.CenterRelation.upContinuation = .levelExpansion := ht.symm.trans he
  cases bad

theorem exact_root : RELEASE_TARGET ∧ FORK_TARGET ∧ NO_EARLY_CHOICE_TARGET :=
  ⟨release_contract,fork,no_early_choice⟩

end CenterExit

#print axioms CenterExit.exact_root
