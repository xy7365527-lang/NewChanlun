import SeedAssemblySpec
import DCFiniteCertificates

namespace SeedAssembly

open NewChanlun.Origin

def tripleCore (a b c d : Int) : Prop :=
  max (max (min a b) (min b c)) (min c d) <
    min (min (max a b) (max b c)) (max c d)

theorem four_intervals (a b c d e : Int)
    (h : (a < b ∧ c < b ∧ c < d ∧ e < d) ∨
      (b < a ∧ b < c ∧ d < c ∧ d < e)) :
    tripleCore a b c d ∨ tripleCore b c d e := by
  rcases h with ⟨h1,h2,h3,h4⟩ | ⟨h1,h2,h3,h4⟩ <;> unfold tripleCore <;> omega

theorem tmin_eq_min (a b : Int) : tmin a b = min a b := by
  unfold tmin
  simp only [Tick] at *
  split <;> omega
theorem tmax_eq_max (a b : Int) : tmax a b = max a b := by
  unfold tmax
  simp only [Tick] at *
  split <;> omega
theorem embed_low (p : Nat → Int) (u : DCSpec.Unit) :
    segLow (DCOriginBridge.embed p u) = min (p u.start) (p u.finish) := by
  unfold segLow DCOriginBridge.embed
  simp only [Tick] at *
  split <;> omega
theorem embed_high (p : Nat → Int) (u : DCSpec.Unit) :
    segHigh (DCOriginBridge.embed p u) = max (p u.start) (p u.finish) := by
  unfold segHigh DCOriginBridge.embed
  simp only [Tick] at *
  split <;> omega

theorem dc_four_core {p : Nat → Int} {delta : Int} {len : Nat} {result : DCSpec.Result}
    (hd : 0 < delta) (hw : DCSpec.Whole p delta len result)
    (pre : List DCSpec.Unit) (a b c d : DCSpec.Unit) (rest : List DCSpec.Unit)
    (hseq : DCSpec.units result = pre ++ a :: b :: c :: d :: rest) :
    LocalCore p a b c ∨ LocalCore p b c d := by
  have geom := DCSpec.dc_geometry p delta hd len result hw
  have ga := geom.1 a (by simp [hseq])
  have gb := geom.1 b (by simp [hseq])
  have gc := geom.1 c (by simp [hseq])
  have gd := geom.1 d (by simp [hseq])
  have hab := geom.2.2 pre a b (c::d::rest) hseq
  have hbc := geom.2.2 (pre ++ [a]) b c (d::rest) (by simpa [List.append_assoc] using hseq)
  have hcd := geom.2.2 (pre ++ [a,b]) c d rest (by simpa [List.append_assoc] using hseq)
  have alt : (p a.start < p a.finish ∧ p b.finish < p a.finish ∧
      p b.finish < p c.finish ∧ p d.finish < p c.finish) ∨
      (p a.finish < p a.start ∧ p a.finish < p b.finish ∧
      p c.finish < p b.finish ∧ p c.finish < p d.finish) := by
    have av := ga.2.2.2.1
    have bv := gb.2.2.2.1
    have cv := gc.2.2.2.1
    have dv := gd.2.2.2.1
    rw [← hab.1] at bv
    rw [← hbc.1] at cv
    rw [← hcd.1] at dv
    cases ha : a.direction <;> cases hb : b.direction <;>
      cases hc : c.direction <;> cases hd' : d.direction <;>
      simp_all [DCSpec.flip, DCSpec.score] <;> omega
  have core := four_intervals (p a.start) (p a.finish) (p b.finish) (p c.finish) (p d.finish) alt
  have numeric :
      computeZD (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) <
        computeZG (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) ∨
      computeZD (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) (DCOriginBridge.embed p d) <
        computeZG (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) (DCOriginBridge.embed p d) := by
    simpa only [computeZD, computeZG, tmin_eq_min, tmax_eq_max, embed_low, embed_high,
      ← hab.1, ← hbc.1, ← hcd.1, tripleCore] using core
  rcases numeric with h | h
  · exact Or.inl (DCOriginBridge.dc_three_center hd hw pre a b c (d::rest) hseq h)
  · exact Or.inr (DCOriginBridge.dc_three_center hd hw (pre ++ [a]) b c d rest
      (by simpa [List.append_assoc] using hseq) h)

theorem first_core_unique {p : Nat → Int} {a b c d : DCSpec.Unit} {i j : Fin 2}
    (hi : FirstCore p a b c d i) (hj : FirstCore p a b c d j) : i = j := by
  apply Fin.ext
  apply Classical.byContradiction
  intro hne
  have hlt : i.val < j.val ∨ j.val < i.val := by omega
  rcases hlt with h | h
  · exact hj.2 i h hi.1
  · exact hi.2 j h hj.1

theorem seed_bound : SEED_BOUND_TARGET := by
  classical
  intro p delta hd len result hw pre a b c d rest hseq
  have both := dc_four_core hd hw pre a b c d rest hseq
  have hexists : ∃ i : Fin 2, FirstCore p a b c d i := by
    by_cases h0 : LocalCore p a b c
    · refine ⟨⟨0,by decide⟩, ?_, ?_⟩
      · simpa [CoreAt] using h0
      · intro j hj
        change j.val < 0 at hj
        omega
    · have h1 := both.resolve_left h0
      refine ⟨⟨1,by decide⟩, ?_, ?_⟩
      · simpa [CoreAt] using h1
      · intro j hj
        have hj0 : j.val = 0 := by omega
        simpa [CoreAt,hj0] using h0
  obtain ⟨i,hi⟩ := hexists
  exact ⟨i,hi,fun j hj => first_core_unique hj hi⟩

theorem witness_whole : DCSpec.Whole witnessPath 1 6 witnessResult := by
  apply DCSpec.Whole.started (n:=5) (d:=.up) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide), DCFinite.extreme_sound (by decide), by decide⟩
  · refine DCSpec.Run.next (n:=5) (d:=.up) (s:=0) (e:=1) (t:=2) (rest:=(witnessUnits.drop 1,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=5) (d:=.down) (s:=1) (e:=2) (t:=3) (rest:=(witnessUnits.drop 2,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=5) (d:=.up) (s:=2) (e:=3) (t:=4) (rest:=(witnessUnits.drop 3,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=5) (d:=.down) (s:=3) (e:=4) (t:=5) (rest:=(witnessUnits.drop 4,witnessTail)) (by decide) (DCFinite.firstDrop_sound (by decide)) (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.up) (s:=4) (e:=5) (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem witness : SHARP_WITNESS_TARGET := by
  have bad : ¬ LocalCore witnessPath u0 u1 u2 := by
    unfold LocalCore CenterConfirmedComplete DirAlternates
    decide
  have good : LocalCore witnessPath u1 u2 u3 := by
    unfold LocalCore CenterConfirmedComplete DirAlternates
    decide
  refine ⟨witness_whole, rfl, bad, good, ?_, by decide, by decide, by decide⟩
  refine ⟨by simpa [CoreAt] using good, ?_⟩
  intro j hj
  have hj0 : j.val = 0 := by omega
  simpa [CoreAt,hj0] using bad

theorem exact_root : SEED_BOUND_TARGET ∧ SHARP_WITNESS_TARGET := ⟨seed_bound,witness⟩

end SeedAssembly

#print axioms SeedAssembly.exact_root
