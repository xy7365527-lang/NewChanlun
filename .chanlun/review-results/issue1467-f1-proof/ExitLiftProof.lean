import ExitLiftSpec

namespace ExitLift

open CenterAttempt (ObservedUnit)

theorem whole : DCSpec.Whole path 2 12 result := by
  apply DCSpec.Whole.started (n:=11) (d:=.down) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide),DCFinite.extreme_sound (by decide),by decide⟩
  · refine DCSpec.Run.next (n:=11) (d:=.down) (s:=0) (e:=1) (t:=2) (rest:=(allUnits.drop 1,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.up) (s:=1) (e:=2) (t:=3) (rest:=(allUnits.drop 2,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.down) (s:=2) (e:=3) (t:=4) (rest:=(allUnits.drop 3,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.up) (s:=3) (e:=4) (t:=5) (rest:=(allUnits.drop 4,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.down) (s:=4) (e:=5) (t:=6) (rest:=(allUnits.drop 5,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.up) (s:=5) (e:=7) (t:=8) (rest:=(allUnits.drop 6,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.down) (s:=7) (e:=8) (t:=9) (rest:=(allUnits.drop 7,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.up) (s:=8) (e:=9) (t:=10) (rest:=(allUnits.drop 8,tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=11) (d:=.down) (s:=9) (e:=10) (t:=11) (rest:=([],tail))
      (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.up) (s:=10) (e:=11)
      (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem prefix_evidence : PREFIX_TARGET := by
  refine ⟨whole,by decide,by decide,rfl,rfl,rfl,rfl,rfl,rfl,
    CenterExit.initialLocal false,CenterExit.nextLocal false,?_⟩
  unfold Formal.CenterTrichotomy.IsLevelExpansion
  decide

theorem min_source (α : Type) (xs a b c : List α) (h : NineRegroup.SourceTriplet xs a b c) :
    9 ≤ xs.length := by
  rcases h with ⟨pre,gap₁,gap₂,post,hc,ha,hb,hd⟩
  have he := congrArg List.length hc
  simp only [List.length_append] at he
  omega

theorem source_bound : SOURCE_TARGET := by
  refine ⟨min_source,?_,?_⟩
  · rintro ⟨a,b,c,h⟩
    have hn := min_source ObservedUnit (seen 9) a b c h
    have hl : (seen 9).length = 7 := rfl
    omega
  · rintro ⟨a,b,c,h⟩
    have hn := min_source ObservedUnit (seen 10) a b c h
    have hl : (seen 10).length = 8 := rfl
    omega

theorem head_start {start finish p ps} (h : SourceHull.Cover start finish (p::ps)) :
    p.start = start := by
  cases h
  rfl

theorem after_head {start finish p ps} (h : SourceHull.Cover start finish (p::ps)) :
    SourceHull.Cover (p.start+p.len) finish ps := by
  cases h with
  | step _ _ ht => exact ht

theorem frozen : FROZEN_TARGET := by
  constructor
  · exact SourceHull.Cover.step oldPart (by decide)
      (SourceHull.Cover.step gapPart (by decide)
        (SourceHull.Cover.step nextPart (by decide) (SourceHull.Cover.done 7)))
  · intro finish middle rest h
    have hm := after_head h
    have hs : middle.start = 3 := head_start hm
    have hn := after_head hm
    have he : 4 = middle.start+middle.len := head_start hn
    exact ⟨hs,by omega,by omega⟩

theorem late : LATE_TARGET := by
  have hf := RetilePacket.fidelity 11 0 (seen 11)
  refine ⟨hf.1,?_,by decide,by decide,by decide,by decide,by decide,by decide,?_⟩
  · have hl : (seen 11).length = 9 := rfl
    simpa only [packets,hl,Nat.zero_add] using hf.2.1
  · unfold UpgradeResearch.Valid
    decide

theorem exact_root : PREFIX_TARGET ∧ SOURCE_TARGET ∧ FROZEN_TARGET ∧ LATE_TARGET :=
  ⟨prefix_evidence,source_bound,frozen,late⟩

end ExitLift

#print axioms ExitLift.exact_root
