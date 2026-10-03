import UpgradeSpec
import DCFiniteCertificates

namespace UpgradeResearch

theorem common_core : COMMON_CORE_TARGET := by
  intro a b c seed hvalid ha hb hc
  have hlo : max (max a.lo b.lo) c.lo ≤ seed.lo :=
    Int.max_le.mpr ⟨Int.max_le.mpr ⟨ha.1,hb.1⟩,hc.1⟩
  have hhi : seed.hi ≤ min (min a.hi b.hi) c.hi :=
    Int.le_min.mpr ⟨Int.le_min.mpr ⟨ha.2,hb.2⟩,hc.2⟩
  refine ⟨⟨hlo,hhi⟩,?_⟩
  change max (max a.lo b.lo) c.lo < min (min a.hi b.hi) c.hi
  have hv : seed.lo < seed.hi := hvalid
  omega

theorem witness_whole : DCSpec.Whole witnessPath 2 11 witnessResult := by
  apply DCSpec.Whole.started (n:=10) (d:=.up) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide), DCFinite.extreme_sound (by decide), by decide⟩
  · refine DCSpec.Run.next (n:=10) (d:=.up) (s:=0) (e:=1) (t:=2) (rest:=(witnessUnits.drop 1,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=1) (e:=2) (t:=3) (rest:=(witnessUnits.drop 2,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=2) (e:=3) (t:=4) (rest:=(witnessUnits.drop 3,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=3) (e:=4) (t:=5) (rest:=(witnessUnits.drop 4,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=4) (e:=5) (t:=6) (rest:=(witnessUnits.drop 5,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=5) (e:=6) (t:=7) (rest:=(witnessUnits.drop 6,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=6) (e:=7) (t:=8) (rest:=(witnessUnits.drop 7,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.down) (s:=7) (e:=8) (t:=9) (rest:=(witnessUnits.drop 8,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=10) (d:=.up) (s:=8) (e:=9) (t:=10) (rest:=(witnessUnits.drop 9,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.down) (s:=9) (e:=10) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem counterexample : COUNTEREXAMPLE_TARGET := by
  have hw : WeakPremises witnessPath := by
    unfold WeakPremises Valid StrictTouch
    decide
  have hbad : ¬ Valid (tripleAt witnessPath 6) := by
    unfold Valid
    decide
  refine ⟨witness_whole, rfl, hw, by decide, by decide, ?_⟩
  intro h
  exact hbad (h witnessPath witnessResult witness_whole rfl hw ⟨2,by decide⟩)

theorem exact_root : COMMON_CORE_TARGET ∧ COUNTEREXAMPLE_TARGET := ⟨common_core,counterexample⟩

end UpgradeResearch

#print axioms UpgradeResearch.exact_root
