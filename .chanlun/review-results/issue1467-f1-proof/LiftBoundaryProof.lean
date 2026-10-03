import LiftBoundarySpec
import DCFiniteCertificates

namespace LiftBoundary

open NewChanlun.Origin
open CenterAttempt (ObservedUnit obs visible)
open UpgradeResearch (Interval Valid StrictTouch core3)

theorem intersection : INTERSECTION_TARGET := by
  intro a b c ha hb hc
  unfold Valid StrictTouch core3 at *
  dsimp at *
  omega

theorem whole : DCSpec.Whole path 2 11 result := by
  apply DCSpec.Whole.started (n:=10) (d:=.up) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide), DCFinite.extreme_sound (by decide), by decide⟩
  · refine DCSpec.Run.next (n:=10) (d:=.up) (s:=0) (e:=1) (t:=2) (rest:=(units.drop 1,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=1) (e:=2) (t:=3) (rest:=(units.drop 2,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=2) (e:=3) (t:=4) (rest:=(units.drop 3,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=3) (e:=4) (t:=5) (rest:=(units.drop 4,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=4) (e:=5) (t:=6) (rest:=(units.drop 5,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=5) (e:=6) (t:=7) (rest:=(units.drop 6,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=6) (e:=7) (t:=8) (rest:=(units.drop 7,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=7) (e:=8) (t:=9) (rest:=(units.drop 8,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=8) (e:=9) (t:=10) (rest:=(units.drop 9,tail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.down) (s:=9) (e:=10) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem witness : WITNESS_TARGET := by
  have r0 : CenterFrame.Returned .short f0 (obs path u3) (obs path u4) := by
    unfold CenterFrame.Returned CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates
    decide
  have r1 : CenterFrame.Returned .short f1 (obs path u5) (obs path u6) := by
    unfold CenterFrame.Returned CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates
    decide
  have r2 : CenterFrame.Returned .short f2 (obs path u7) (obs path u8) := by
    unfold CenterFrame.Returned CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates
    decide
  have s0 : CenterFrame.Absorb .short f0 (obs path u3) (obs path u4) (.active f1) :=
    ⟨r0,by decide,CenterFrame.recorded_extend _ _ _,by decide⟩
  have s1 : CenterFrame.Absorb .short f1 (obs path u5) (obs path u6) (.active f2) :=
    ⟨r1,by decide,CenterFrame.recorded_extend _ _ _,by decide⟩
  have s2 : CenterFrame.Absorb .short f2 (obs path u7) (obs path u8) (.retilePending f3) :=
    ⟨r2,by decide,CenterFrame.recorded_extend _ _ _,by decide⟩
  refine ⟨whole,s0,s1,s2,rfl,rfl,rfl,by unfold closedTouches; decide,rfl,by decide,by decide,by decide,?_,by decide,by decide⟩
  unfold strictLift
  change ¬ Valid (⟨10040,10040⟩ : Interval)
  unfold Valid
  decide

theorem origin_rejection : ORIGIN_REJECTION_TARGET := by
  intro a b c hal hah hbl hbh hcl hch h
  have hc := h.2
  simp [computeZD,computeZG,hal,hah,hbl,hbh,hcl,hch,tmin,tmax] at hc

theorem no_automatic_lift : ¬ AutomaticStrictLift := by
  intro h
  rcases witness with ⟨hw,h0,h1,h2,hvisible,hclock,hcount,htouch,htail,haccepted,hnumbers,hcore,hnot,hdir,hqty⟩
  exact hnot (h path 2 (by decide) 11 result hw .short f2 (obs path u7) (obs path u8) f3 h2 hvisible htouch htail haccepted)

theorem exact_root : INTERSECTION_TARGET ∧ WITNESS_TARGET ∧ ORIGIN_REJECTION_TARGET ∧ ¬ AutomaticStrictLift :=
  ⟨intersection,witness,origin_rejection,no_automatic_lift⟩

end LiftBoundary

#print axioms LiftBoundary.exact_root
