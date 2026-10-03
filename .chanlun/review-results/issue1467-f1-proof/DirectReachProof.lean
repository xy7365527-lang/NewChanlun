import DirectReachSpec

set_option maxRecDepth 4096
set_option maxHeartbeats 1000000

namespace DirectReach

open MovingQuote
open NewChanlun.Origin

theorem addBid_step (p : Int) (bids asks : List Int) :
    Step ⟨⟨true,.add,1⟩,p⟩ ⟨bids,asks⟩ ⟨p::bids,asks⟩ :=
  ⟨by change (0 : Nat) < 1; decide,rfl,rfl⟩
theorem addAsk_step (p : Int) (bids asks : List Int) :
    Step ⟨⟨false,.add,1⟩,p⟩ ⟨bids,asks⟩ ⟨bids,p::asks⟩ :=
  ⟨by change (0 : Nat) < 1; decide,rfl,rfl⟩
theorem cancelBid_step (p : Int) (bids asks : List Int) :
    Step ⟨⟨true,.cancel,1⟩,p⟩ ⟨p::bids,asks⟩ ⟨bids,asks⟩ :=
  ⟨by change (0 : Nat) < 1; decide,⟨[],bids,rfl,rfl⟩,rfl⟩
theorem cancelAsk_step (p : Int) (bids asks : List Int) :
    Step ⟨⟨false,.cancel,1⟩,p⟩ ⟨bids,p::asks⟩ ⟨bids,asks⟩ :=
  ⟨by change (0 : Nat) < 1; decide,⟨[],asks,rfl,rfl⟩,rfl⟩

theorem all_quotes : ∀ i : Fin 39, QuoteOf (state i) (quote i) ∧ ValidBook (state i) := by
  decide
theorem quote_good (i : Fin 39) : QuoteOf (state i) (quote i) := (all_quotes i).1

-- 逐事件与逐块证明由冻结fixture的操作表生成，接下方尾段。

theorem steps : Steps DirectReachFixture.books DirectReachFixture.events := by
  unfold DirectReachFixture.books DirectReachFixture.events
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact addBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact cancelBid_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact addAsk_step _ _ _
  apply Steps.cons
  · exact cancelAsk_step _ _ _
  exact Steps.nil _

theorem flow0 : Flow (sign 0) (state (start 0)) (quote (start 0))
    (state (finish 0)) (quote (finish 0)) (blockEvents 0) := by
  apply MovingQuote.Flow.step (b:=state 1) (r:=quote 1)
  · exact cancelBid_step _ _ _
  · exact quote_good 0
  · exact quote_good 1
  · decide
  apply MovingQuote.Flow.step (b:=state 2) (r:=quote 2)
  · exact cancelBid_step _ _ _
  · exact quote_good 1
  · exact quote_good 2
  · decide
  apply MovingQuote.Flow.step (b:=state 3) (r:=quote 3)
  · exact cancelBid_step _ _ _
  · exact quote_good 2
  · exact quote_good 3
  · decide
  apply MovingQuote.Flow.step (b:=state 4) (r:=quote 4)
  · exact addAsk_step _ _ _
  · exact quote_good 3
  · exact quote_good 4
  · decide
  apply MovingQuote.Flow.step (b:=state 5) (r:=quote 5)
  · exact addAsk_step _ _ _
  · exact quote_good 4
  · exact quote_good 5
  · decide
  apply MovingQuote.Flow.step (b:=state 6) (r:=quote 6)
  · exact addAsk_step _ _ _
  · exact quote_good 5
  · exact quote_good 6
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 6)

theorem flow1 : Flow (sign 1) (state (start 1)) (quote (start 1))
    (state (finish 1)) (quote (finish 1)) (blockEvents 1) := by
  apply MovingQuote.Flow.step (b:=state 7) (r:=quote 7)
  · exact cancelAsk_step _ _ _
  · exact quote_good 6
  · exact quote_good 7
  · decide
  apply MovingQuote.Flow.step (b:=state 8) (r:=quote 8)
  · exact cancelAsk_step _ _ _
  · exact quote_good 7
  · exact quote_good 8
  · decide
  apply MovingQuote.Flow.step (b:=state 9) (r:=quote 9)
  · exact addBid_step _ _ _
  · exact quote_good 8
  · exact quote_good 9
  · decide
  apply MovingQuote.Flow.step (b:=state 10) (r:=quote 10)
  · exact addBid_step _ _ _
  · exact quote_good 9
  · exact quote_good 10
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 10)

theorem flow2 : Flow (sign 2) (state (start 2)) (quote (start 2))
    (state (finish 2)) (quote (finish 2)) (blockEvents 2) := by
  apply MovingQuote.Flow.step (b:=state 11) (r:=quote 11)
  · exact cancelBid_step _ _ _
  · exact quote_good 10
  · exact quote_good 11
  · decide
  apply MovingQuote.Flow.step (b:=state 12) (r:=quote 12)
  · exact addAsk_step _ _ _
  · exact quote_good 11
  · exact quote_good 12
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 12)

theorem flow3 : Flow (sign 3) (state (start 3)) (quote (start 3))
    (state (finish 3)) (quote (finish 3)) (blockEvents 3) := by
  apply MovingQuote.Flow.step (b:=state 13) (r:=quote 13)
  · exact cancelAsk_step _ _ _
  · exact quote_good 12
  · exact quote_good 13
  · decide
  apply MovingQuote.Flow.step (b:=state 14) (r:=quote 14)
  · exact cancelAsk_step _ _ _
  · exact quote_good 13
  · exact quote_good 14
  · decide
  apply MovingQuote.Flow.step (b:=state 15) (r:=quote 15)
  · exact cancelAsk_step _ _ _
  · exact quote_good 14
  · exact quote_good 15
  · decide
  apply MovingQuote.Flow.step (b:=state 16) (r:=quote 16)
  · exact cancelAsk_step _ _ _
  · exact quote_good 15
  · exact quote_good 16
  · decide
  apply MovingQuote.Flow.step (b:=state 17) (r:=quote 17)
  · exact addBid_step _ _ _
  · exact quote_good 16
  · exact quote_good 17
  · decide
  apply MovingQuote.Flow.step (b:=state 18) (r:=quote 18)
  · exact addBid_step _ _ _
  · exact quote_good 17
  · exact quote_good 18
  · decide
  apply MovingQuote.Flow.step (b:=state 19) (r:=quote 19)
  · exact addBid_step _ _ _
  · exact quote_good 18
  · exact quote_good 19
  · decide
  apply MovingQuote.Flow.step (b:=state 20) (r:=quote 20)
  · exact addBid_step _ _ _
  · exact quote_good 19
  · exact quote_good 20
  · decide
  apply MovingQuote.Flow.step (b:=state 21) (r:=quote 21)
  · exact addBid_step _ _ _
  · exact quote_good 20
  · exact quote_good 21
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 21)

theorem flow4 : Flow (sign 4) (state (start 4)) (quote (start 4))
    (state (finish 4)) (quote (finish 4)) (blockEvents 4) := by
  apply MovingQuote.Flow.step (b:=state 22) (r:=quote 22)
  · exact cancelBid_step _ _ _
  · exact quote_good 21
  · exact quote_good 22
  · decide
  apply MovingQuote.Flow.step (b:=state 23) (r:=quote 23)
  · exact addAsk_step _ _ _
  · exact quote_good 22
  · exact quote_good 23
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 23)

theorem flow5 : Flow (sign 5) (state (start 5)) (quote (start 5))
    (state (finish 5)) (quote (finish 5)) (blockEvents 5) := by
  apply MovingQuote.Flow.step (b:=state 24) (r:=quote 24)
  · exact cancelAsk_step _ _ _
  · exact quote_good 23
  · exact quote_good 24
  · decide
  apply MovingQuote.Flow.step (b:=state 25) (r:=quote 25)
  · exact cancelAsk_step _ _ _
  · exact quote_good 24
  · exact quote_good 25
  · decide
  apply MovingQuote.Flow.step (b:=state 26) (r:=quote 26)
  · exact cancelAsk_step _ _ _
  · exact quote_good 25
  · exact quote_good 26
  · decide
  apply MovingQuote.Flow.step (b:=state 27) (r:=quote 27)
  · exact cancelAsk_step _ _ _
  · exact quote_good 26
  · exact quote_good 27
  · decide
  apply MovingQuote.Flow.step (b:=state 28) (r:=quote 28)
  · exact addBid_step _ _ _
  · exact quote_good 27
  · exact quote_good 28
  · decide
  apply MovingQuote.Flow.step (b:=state 29) (r:=quote 29)
  · exact addBid_step _ _ _
  · exact quote_good 28
  · exact quote_good 29
  · decide
  apply MovingQuote.Flow.step (b:=state 30) (r:=quote 30)
  · exact addBid_step _ _ _
  · exact quote_good 29
  · exact quote_good 30
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 30)

theorem flow6 : Flow (sign 6) (state (start 6)) (quote (start 6))
    (state (finish 6)) (quote (finish 6)) (blockEvents 6) := by
  apply MovingQuote.Flow.step (b:=state 31) (r:=quote 31)
  · exact cancelBid_step _ _ _
  · exact quote_good 30
  · exact quote_good 31
  · decide
  apply MovingQuote.Flow.step (b:=state 32) (r:=quote 32)
  · exact cancelBid_step _ _ _
  · exact quote_good 31
  · exact quote_good 32
  · decide
  apply MovingQuote.Flow.step (b:=state 33) (r:=quote 33)
  · exact cancelBid_step _ _ _
  · exact quote_good 32
  · exact quote_good 33
  · decide
  apply MovingQuote.Flow.step (b:=state 34) (r:=quote 34)
  · exact cancelBid_step _ _ _
  · exact quote_good 33
  · exact quote_good 34
  · decide
  apply MovingQuote.Flow.step (b:=state 35) (r:=quote 35)
  · exact addAsk_step _ _ _
  · exact quote_good 34
  · exact quote_good 35
  · decide
  apply MovingQuote.Flow.step (b:=state 36) (r:=quote 36)
  · exact addAsk_step _ _ _
  · exact quote_good 35
  · exact quote_good 36
  · decide
  apply MovingQuote.Flow.step (b:=state 37) (r:=quote 37)
  · exact addAsk_step _ _ _
  · exact quote_good 36
  · exact quote_good 37
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 37)

theorem flow7 : Flow (sign 7) (state (start 7)) (quote (start 7))
    (state (finish 7)) (quote (finish 7)) (blockEvents 7) := by
  apply MovingQuote.Flow.step (b:=state 38) (r:=quote 38)
  · exact cancelAsk_step _ _ _
  · exact quote_good 37
  · exact quote_good 38
  · decide
  exact MovingQuote.Flow.nil _ _ (quote_good 38)

theorem all_flows : ∀ i : Fin 8, Flow (sign i) (state (start i)) (quote (start i))
    (state (finish i)) (quote (finish i)) (blockEvents i) := by
  intro i0
  refine Fin.cases flow0 ?_ i0
  intro i1
  refine Fin.cases flow1 ?_ i1
  intro i2
  refine Fin.cases flow2 ?_ i2
  intro i3
  refine Fin.cases flow3 ?_ i3
  intro i4
  refine Fin.cases flow4 ?_ i4
  intro i5
  refine Fin.cases flow5 ?_ i5
  intro i6
  refine Fin.cases flow6 ?_ i6
  intro i7
  refine Fin.cases flow7 ?_ i7
  intro empty
  exact Fin.elim0 empty

theorem trajectory : TRAJECTORY_TARGET := ⟨steps,rfl,rfl,all_quotes,all_flows⟩

theorem grouping : GROUP_TARGET := by
  refine ⟨group_sound _,?_,?_,by decide,by decide,by decide,?_⟩
  · intro bs h
    exact (runs_determined h).symm
  · exact (runs_properties (group_sound DirectReachFixture.events)).1
  · unfold MovingQuote.Completed DirectQuote.Completed
    decide

theorem geometry : GEOMETRY_TARGET := by
  refine ⟨by rfl,?_,?_,by rfl,by rfl⟩
  · unfold Formal.CenterTrichotomy.SameLevelNewCenterPair
    decide
  · unfold Formal.CenterTrichotomy.IsLevelExpansion
    decide

theorem obstruction : OBSTRUCTION_TARGET := by
  refine ⟨?_,ExitSupport.cannot_grow_bridge,by rfl,by rfl,by rfl⟩
  rintro ⟨a,b,c,h⟩
  have hn := ExitLift.min_source Located done a b c h
  have hl : done.length = 7 := by decide
  omega

theorem exact_root : TRAJECTORY_TARGET ∧ GROUP_TARGET ∧ GEOMETRY_TARGET ∧ OBSTRUCTION_TARGET :=
  ⟨trajectory,grouping,geometry,obstruction⟩

end DirectReach

#print axioms DirectReach.exact_root
