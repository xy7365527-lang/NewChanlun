import ClockEdgeSpec

namespace ClockEdge

open NewChanlun.Origin

theorem whole : DCSpec.Whole path 2 5 result := by
  apply DCSpec.Whole.started (n:=4) (d:=.up) (s:=0) (c:=1) (by decide)
  · exact ⟨DCFinite.firstWide_sound (by decide),DCFinite.extreme_sound (by decide),by decide⟩
  · refine DCSpec.Run.next (n:=4) (d:=.up) (s:=0) (e:=1) (t:=2)
      (rest:=(units.drop 1,tail)) (by decide) (DCFinite.firstDrop_sound (by decide))
      (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=4) (d:=.down) (s:=1) (e:=2) (t:=3)
      (rest:=(units.drop 2,tail)) (by decide) (DCFinite.firstDrop_sound (by decide))
      (DCFinite.extreme_sound (by decide)) (by decide) ?_
    refine DCSpec.Run.next (n:=4) (d:=.up) (s:=2) (e:=3) (t:=4)
      (rest:=(units.drop 3,tail)) (by decide) (DCFinite.firstDrop_sound (by decide))
      (DCFinite.extreme_sound (by decide)) (by decide) ?_
    exact DCSpec.Run.stop (d:=.down) (s:=3) (e:=4)
      (DCFinite.noDrop_sound (by decide)) (DCFinite.extreme_sound (by decide))

theorem clock_changes_force : CLOCK_TARGET := by
  refine ⟨whole,?_,?_,rfl,?_,?_,?_,?_⟩
  · intro b
    cases b <;> decide
  · intro b
    cases b <;> simp [strokes,units,fromUnit,clock,Stroke.WellFormed]
  all_goals decide +kernel

theorem zero_is_not_velocity : ZERO_TARGET := by
  constructor
  · intro a b i
    constructor
    · simp [zeroStamp,Stroke.WellFormed]
    · simp [zeroStamp,Stroke.velocity,Rat.div_def,Rat.inv_zero,Rat.mul_zero]
  · decide

theorem quote_edge_components : EDGE_TARGET := by
  constructor
  · intro pos q r start len
    cases pos with
    | false =>
      change 2*(r.bid-q.ask) = (r.bid+r.ask)-(q.bid+q.ask) +
        (-1)*((q.ask-q.bid)+(r.ask-r.bid))
      omega
    | true =>
      change 2*(r.ask-q.bid) = (r.bid+r.ask)-(q.bid+q.ask) +
        1*((q.ask-q.bid)+(r.ask-r.bid))
      omega
  · intro q start len hq hn
    change start < start+len ∧ q.bid < q.ask ∧ q.ask-q.bid = spread q
    refine ⟨by omega,hq,rfl⟩

theorem exact_root : CLOCK_TARGET ∧ ZERO_TARGET ∧ EDGE_TARGET :=
  ⟨clock_changes_force,zero_is_not_velocity,quote_edge_components⟩

end ClockEdge

#print axioms ClockEdge.exact_root
