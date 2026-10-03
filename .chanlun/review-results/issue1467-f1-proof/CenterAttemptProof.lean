import CenterAttemptSpec
import DCFiniteCertificates

namespace CenterAttempt

open NewChanlun.Origin

theorem view_causal : VIEW_CAUSAL_TARGET := by
  intro p q delta hd len₁ len₂ t r₁ r₂ ht₁ ht₂ hp hr₁ hr₂
  have hu := DCSpec.dc_causal p q delta hd len₁ len₂ t r₁ r₂ ht₁ ht₂ hp hr₁ hr₂
  have heq : visible p r₁ t = visible q r₂ t := by
    unfold visible
    rw [hu]
    apply List.map_congr_left
    intro u hmem
    have hm := List.mem_filter.mp hmem
    have hg := (DCSpec.dc_geometry q delta hd len₂ r₂ hr₂).1 u hm.1
    have hk : u.knownAt ≤ t := by simpa using hm.2
    have hf : u.finish ≤ t := by have hh := hg.2.1; omega
    have hs : u.start ≤ t := by have hh := hg.1; omega
    simp only [DCOriginBridge.embed, hp u.start hs, hp u.finish hf]
  exact ⟨heq, fun _ => by rw [heq]⟩

theorem type3_bridge : TYPE3_BRIDGE_TARGET := by
  intro xs start h
  unfold firstPairUp at h
  split at h
  · rename_i a b c leave ret rest heq
    refine ⟨a,b,c,leave,ret,rest,heq,h,?_⟩
    have hh := h
    rcases hh with ⟨hseed,hab,hbc,hcl,hlr,hup,hdown,hleave,hret⟩
    simp only [IsType3Buy, pairEndpoint]
    simp [hup,hleave,hcl,hlr,hret]
  · cases h

theorem wholeA : DCSpec.Whole pathA 2 7 resultA := by
  apply DCSpec.Whole.started (n:=6) (d:=.down) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide), DCFinite.extreme_sound (by decide), by decide⟩
  · refine DCSpec.Run.next (n:=6) (d:=.down) (s:=0) (e:=1) (t:=2) (rest:=(unitsA.drop 1,tailA)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=6) (d:=.up) (s:=1) (e:=2) (t:=3) (rest:=(unitsA.drop 2,tailA)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=6) (d:=.down) (s:=2) (e:=3) (t:=4) (rest:=(unitsA.drop 3,tailA)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=6) (d:=.up) (s:=3) (e:=4) (t:=5) (rest:=(unitsA.drop 4,tailA)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=6) (d:=.down) (s:=4) (e:=5) (t:=6) (rest:=(unitsA.drop 5,tailA)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.up) (s:=5) (e:=6) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem wholeB : DCSpec.Whole pathB 2 11 resultB := by
  apply DCSpec.Whole.started (n:=10) (d:=.down) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide), DCFinite.extreme_sound (by decide), by decide⟩
  · refine DCSpec.Run.next (n:=10) (d:=.down) (s:=0) (e:=1) (t:=2) (rest:=(unitsB.drop 1,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=1) (e:=2) (t:=3) (rest:=(unitsB.drop 2,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=2) (e:=3) (t:=4) (rest:=(unitsB.drop 3,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=3) (e:=4) (t:=5) (rest:=(unitsB.drop 4,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=4) (e:=6) (t:=7) (rest:=(unitsB.drop 5,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=6) (e:=8) (t:=9) (rest:=(unitsB.drop 6,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=8) (e:=9) (t:=10) (rest:=(unitsB.drop 7,tailB)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.up) (s:=9) (e:=10) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem witness : WITNESS_TARGET := by
  refine ⟨wholeA,wholeB,by decide,rfl,rfl,?_,?_,?_,by decide,by decide,by decide,by decide⟩
  · simp [firstPairUp,visible,resultA,DCSpec.units,unitsA,a0,a1,a2,leaveA,retA]
  · change UpPair (obs pathA a0) (obs pathA a1) (obs pathA a2) (obs pathA leaveA) (obs pathA retA)
    unfold UpPair Joined CenterConfirmedComplete DirAlternates
    decide
  · change ¬ UpPair (obs pathB a0) (obs pathB a1) (obs pathB a2) (obs pathB leaveA) (obs pathB retB)
    unfold UpPair Joined CenterConfirmedComplete DirAlternates
    decide

theorem exact_root : VIEW_CAUSAL_TARGET ∧ TYPE3_BRIDGE_TARGET ∧ WITNESS_TARGET :=
  ⟨view_causal,type3_bridge,witness⟩

end CenterAttempt

#print axioms CenterAttempt.exact_root
