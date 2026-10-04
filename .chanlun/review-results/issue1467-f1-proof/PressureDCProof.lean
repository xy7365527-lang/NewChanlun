import PressureDCSpec

set_option maxRecDepth 100000
set_option maxHeartbeats 10000000

namespace PressureDC
open MovingQuote

theorem recover_monotone (old d : Bool) (q r : Quote)
    (hspread : q.bid < q.ask) (hm : Monotone d q r) :
    recover old (select old q) (select d r) = d := by
  cases old with
  | false =>
    cases d with
    | false =>
      change r.bid ≤ q.bid ∧ r.ask ≤ q.ask at hm
      have hnot : ¬ q.bid < r.bid := by omega
      simp [select,recover,hnot]
    | true =>
      change q.bid ≤ r.bid ∧ q.ask ≤ r.ask at hm
      have hlt : q.bid < r.ask := by omega
      simp [select,recover,hlt]
  | true =>
    cases d with
    | false =>
      change r.bid ≤ q.bid ∧ r.ask ≤ q.ask at hm
      have hlt : r.bid < q.ask := by omega
      have hnot : ¬ q.ask < r.bid := by omega
      simp [select,recover,hlt,hnot]
    | true =>
      change q.bid ≤ r.bid ∧ q.ask ≤ r.ask at hm
      have hnot : ¬ r.ask < q.ask := by omega
      simp [select,recover,hnot]

theorem recover_initial (old d : Bool) (q r : Quote)
    (hspread : q.bid < q.ask) (hm : Monotone d q r) :
    recover old (select (!d) q) (select d r) = d ∧
    1 ≤ (if d then select d r - select (!d) q else select (!d) q - select d r) := by
  cases d with
  | false =>
    change r.bid ≤ q.bid ∧ r.ask ≤ q.ask at hm
    have hlt : r.bid < q.ask := by omega
    have hnot : ¬ q.ask < r.bid := by omega
    simp only [Bool.not_false,select,Bool.false_eq_true,↓reduceIte]
    constructor
    · simp [recover,hlt,hnot]
    · omega
  | true =>
    change q.bid ≤ r.bid ∧ q.ask ≤ r.ask at hm
    have hlt : q.bid < r.ask := by omega
    simp only [Bool.not_true,select,Bool.false_eq_true,↓reduceIte]
    constructor
    · simp [recover,hlt]
    · omega

theorem projection_checked : PROJECTION_TARGET := by
  constructor
  · intro e a b q r old hs hq hr
    exact recover_monotone old (positive e) q r hq.2.2 (quote_step hs hq hr)
  · intro e a b q r old hs hq hr
    exact recover_initial old (positive e) q r hq.2.2 (quote_step hs hq hr)

theorem decode_tail (xs : List Input) : ∀ book q old, Legal book q xs →
    decodeDirections old (select old q) (samples xs) = eventWord xs := by
  induction xs with
  | nil => intros; rfl
  | cons x xs ih =>
    intro book q old h
    rcases h with ⟨hs,hq,hr,ht⟩
    simp only [samples,List.map_cons,decodeDirections,eventWord]
    rw [recover_monotone old (positive x.event) q x.afterQuote hq.2.2 (quote_step hs hq hr)]
    exact congrArg (List.cons (positive x.event)) (ih x.afterBook x.afterQuote (positive x.event) ht)

theorem observed_word {book q xs} (h : Legal book q xs) : observedWord q xs = eventWord xs := by
  cases xs with
  | nil => rfl
  | cons x xs =>
    rcases h with ⟨hs,hq,hr,ht⟩
    simp only [observedWord,samples,List.map_cons,decodeDirections,eventWord]
    rw [(recover_initial false (positive x.event) q x.afterQuote hq.2.2 (quote_step hs hq hr)).1]
    exact congrArg (List.cons (positive x.event)) (decode_tail xs x.afterBook x.afterQuote (positive x.event) ht)

theorem group_shape (es : List Event) : dcGroups (es.map positive) = (group es).map groupShape := by
  induction es with
  | nil => rfl
  | cons e es ih =>
    simp only [List.map_cons,dcGroups]
    rw [ih]
    cases hg : group es with
    | nil => simp [group,hg,groupShape,events]
    | cons b bs =>
      by_cases he : positive e = positive b.first
      · simp [group,hg,groupShape,events,prepend,he,Nat.add_comm]
      · simp [group,hg,groupShape,events,he]

theorem views_project (bs : List Block) (start : Nat) :
    views start (bs.map groupShape) = (locate start bs).map actualView := by
  induction bs generalizing start with
  | nil => rfl
  | cons b bs ih =>
    cases bs <;> simp_all [views,locate,groupShape,actualView]

theorem trace_checked : TRACE_TARGET := by
  intro book q xs h
  have hw := observed_word h
  refine ⟨hw,?_⟩
  unfold dcViews actualViews
  rw [hw]
  have hs : dcGroups (eventWord xs) = (group (xs.map (·.event))).map groupShape := by
    simpa [eventWord,List.map_map,Function.comp_def] using group_shape (xs.map (·.event))
  rw [hs]
  exact views_project _ 0

theorem geometry_checked : GEOMETRY_TARGET := by
  intro d q r start len
  cases d <;> rfl

theorem tie_checked : TIE_TARGET := by
  unfold TIE_TARGET
  refine ⟨?_,?_,?_,?_,?_⟩
  · simp [Legal,tieInputs,tieEvent,tieQuote,Step,Update,QuoteOf,BestBid,BestAsk]
  · decide
  · decide
  · unfold DCSpec.Extreme
    refine ⟨by decide,by decide,?_,?_⟩
    · intro j hj hj2
      have casesJ : j = 0 ∨ j = 1 ∨ j = 2 := by omega
      rcases casesJ with h | h | h <;> subst j <;> decide
    · intro j hj hlt
      have : j = 0 := by omega
      subst j
      decide
  · intro h
    have hne := h.2.2.2 1 (by decide) (by decide)
    exact hne rfl

theorem exact_root : PROJECTION_TARGET ∧ TRACE_TARGET ∧ GEOMETRY_TARGET ∧ TIE_TARGET :=
  ⟨projection_checked,trace_checked,geometry_checked,tie_checked⟩

end PressureDC
