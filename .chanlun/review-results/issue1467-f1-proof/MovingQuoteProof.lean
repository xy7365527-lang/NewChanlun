import MovingQuoteWireSpec

namespace MovingQuote

open NewChanlun.Origin

theorem group_sound (es : List Event) : Runs es (group es) := by
  induction es with
  | nil => exact Runs.nil
  | cons e es ih =>
    cases hg : group es with
    | nil =>
      rw [hg] at ih
      cases ih
      exact Runs.one e
    | cons b bs =>
      rw [hg] at ih
      by_cases he : positive e = positive b.first
      · simpa [group,hg,he] using Runs.same e ih he
      · simpa [group,hg,he] using Runs.different e ih he

theorem runs_determined {es bs} (h : Runs es bs) : group es = bs := by
  induction h with
  | nil => rfl
  | one e => rfl
  | same e h he ih => simp [group,ih,he]
  | different e h he ih => simp [group,ih,he]

theorem uniform_single (e : Event) : Uniform ⟨e,[]⟩ := by simp [Uniform,events]
theorem uniform_prepend {e : Event} {b : Block} (he : positive e = positive b.first)
    (h : Uniform b) : Uniform (prepend e b) := by
  intro x hx
  change x ∈ e::events b at hx
  rcases List.mem_cons.mp hx with hx | hx
  · subst x; rfl
  · exact (h x hx).trans he.symm

theorem runs_properties {es bs} (h : Runs es bs) :
    decode bs = es ∧ (∀ b ∈ bs, Uniform b) ∧ Separated bs := by
  induction h with
  | nil => simp [decode,Separated]
  | one e => exact ⟨rfl,by intro b hb; simpa using (List.mem_singleton.mp hb ▸ uniform_single e),by trivial⟩
  | @same e es b bs h he ih =>
    refine ⟨?_,?_,?_⟩
    · change e::decode (b::bs) = e::es
      rw [ih.1]
    · intro b' hb'
      rcases List.mem_cons.mp hb' with hb' | hb'
      · subst b'; exact uniform_prepend he (ih.2.1 b (by simp))
      · exact ih.2.1 b' (List.mem_cons_of_mem _ hb')
    · cases bs with
      | nil => trivial
      | cons c bs =>
        change positive e ≠ positive c.first ∧ Separated (c::bs)
        exact ⟨by rw [he]; exact ih.2.2.1,ih.2.2.2⟩
  | @different e es b bs h he ih =>
    refine ⟨?_,?_,he,ih.2.2⟩
    · change e::decode (b::bs) = e::es
      rw [ih.1]
    · intro b' hb'
      rcases List.mem_cons.mp hb' with hb' | hb'
      · subst b'; exact uniform_single e
      · exact ih.2.1 b' hb'

theorem run_contract : RUN_TARGET := by
  refine ⟨?_,?_⟩
  · intro es
    exact ⟨group es,group_sound es,fun other ho => (runs_determined ho).symm⟩
  · intro es bs h
    exact runs_properties h

theorem bid_unique {xs p q} (hp : BestBid xs p) (hq : BestBid xs q) : p = q := by
  have a := hp.2 q hq.1
  have b := hq.2 p hp.1
  omega
theorem ask_unique {xs p q} (hp : BestAsk xs p) (hq : BestAsk xs q) : p = q := by
  have a := hp.2 q hq.1
  have b := hq.2 p hp.1
  omega
theorem quote_unique {b q r} (hq : QuoteOf b q) (hr : QuoteOf b r) : q = r := by
  have hb := bid_unique hq.1 hr.1
  have ha := ask_unique hq.2.1 hr.2.1
  cases q
  cases r
  simp_all

theorem add_subset {p n xs ys} (h : Update .add p n xs ys) : ∀ x ∈ xs, x ∈ ys := by
  change ys = List.replicate n p ++ xs at h
  intro x hx
  rw [h]
  simp [hx]
theorem remove_subset {act p n xs ys} (hn : act ≠ .add) (h : Update act p n xs ys) :
    ∀ x ∈ ys, x ∈ xs := by
  unfold Update at h
  rw [if_neg hn] at h
  rcases h with ⟨pre,post,hx,hy⟩
  subst xs
  subst ys
  intro x hx
  simp only [List.mem_append] at hx ⊢
  rcases hx with hx | hx <;> simp_all
theorem bid_subset {xs ys p q} (hs : ∀ x ∈ xs, x ∈ ys)
    (hp : BestBid xs p) (hq : BestBid ys q) : p ≤ q := hq.2 p (hs p hp.1)
theorem ask_subset {xs ys p q} (hs : ∀ x ∈ xs, x ∈ ys)
    (hp : BestAsk xs p) (hq : BestAsk ys q) : q ≤ p := hq.2 p (hs p hp.1)

theorem quote_step {e a b q r} (h : Step e a b) (hq : QuoteOf a q) (hr : QuoteOf b r) :
    Monotone (positive e) q r := by
  cases side : e.core.bidSide with
  | false =>
    have hs : Update e.core.action e.price e.core.amount a.asks b.asks ∧ b.bids = a.bids :=
      by simpa [side] using h.2
    have hb : q.bid = r.bid := bid_unique hq.1 (by simpa only [hs.2] using hr.1)
    by_cases ha : e.core.action = .add
    · have hu : Update .add e.price e.core.amount a.asks b.asks := by simpa only [ha] using hs.1
      have hl := ask_subset (add_subset hu) hq.2.1 hr.2.1
      have hp : positive e = false := by
        cases act : e.core.action <;> simp_all [positive,DirectQuote.positive]
      rw [hp]
      change r.bid ≤ q.bid ∧ r.ask ≤ q.ask
      exact ⟨by omega,hl⟩
    · have hl := ask_subset (remove_subset ha hs.1) hr.2.1 hq.2.1
      have hp : positive e = true := by
        cases act : e.core.action <;> simp_all [positive,DirectQuote.positive]
      rw [hp]
      change q.bid ≤ r.bid ∧ q.ask ≤ r.ask
      exact ⟨by omega,hl⟩
  | true =>
    have hs : Update e.core.action e.price e.core.amount a.bids b.bids ∧ b.asks = a.asks :=
      by simpa [side] using h.2
    have he : q.ask = r.ask := ask_unique hq.2.1 (by simpa only [hs.2] using hr.2.1)
    by_cases ha : e.core.action = .add
    · have hu : Update .add e.price e.core.amount a.bids b.bids := by simpa only [ha] using hs.1
      have hl := bid_subset (add_subset hu) hq.1 hr.1
      have hp : positive e = true := by
        cases act : e.core.action <;> simp_all [positive,DirectQuote.positive]
      rw [hp]
      change q.bid ≤ r.bid ∧ q.ask ≤ r.ask
      exact ⟨hl,by omega⟩
    · have hl := bid_subset (remove_subset ha hs.1) hr.1 hq.1
      have hp : positive e = false := by
        cases act : e.core.action <;> simp_all [positive,DirectQuote.positive]
      rw [hp]
      change r.bid ≤ q.bid ∧ r.ask ≤ q.ask
      exact ⟨hl,by omega⟩

theorem step_contract : STEP_TARGET := ⟨fun _ _ _ => quote_unique,fun _ _ _ _ _ => quote_step⟩

theorem flow_quotes {pos a q b r es} (h : Flow pos a q b r es) : QuoteOf a q ∧ QuoteOf b r := by
  induction h with
  | nil b q hq => exact ⟨hq,hq⟩
  | step e a q b r c s es hs hq hr hp hf ih => exact ⟨hq,ih.2⟩
theorem flow_monotone {pos a q b r es} (h : Flow pos a q b r es) : Monotone pos q r := by
  induction h with
  | nil b q hq => cases pos <;> simp [Monotone]
  | step e a q b r c s es hs hq hr hp hf ih =>
    have hm := quote_step hs hq hr
    rw [hp] at hm
    cases pos <;> simp [Monotone] at hm ih ⊢ <;> omega

theorem leg_contract : LEG_TARGET := by
  refine ⟨?_,?_,?_⟩
  · intro pos a q b r es hf hn start
    have hq := (flow_quotes hf).1.2.2
    have hm := flow_monotone hf
    have hl : 0 < es.length := by cases es <;> simp_all
    cases pos with
    | false =>
      change start < start+es.length ∧ r.bid < q.ask
      have hb := hm.1
      exact ⟨by omega,by omega⟩
    | true =>
      change start < start+es.length ∧ q.bid < r.ask
      have ha := hm.2
      exact ⟨by omega,by omega⟩
  · intro pos q r s start n m
    cases pos <;> rfl
  · intro q start b
    rfl

theorem causal : CAUSAL_TARGET := by
  intro p q n s e hp
  apply DirectQuote.causal
  intro i hi
  exact congrArg Event.core (hp i hi)

instance (b : Book) (q : Quote) : Decidable (QuoteOf b q) := by
  unfold QuoteOf BestBid BestAsk
  infer_instance
instance (b : Book) : Decidable (ValidBook b) := by
  unfold ValidBook
  infer_instance

theorem witness : WITNESS_TARGET := by
  have s0 : Step e0 (book 0) (book 1) :=
    ⟨by decide,⟨[],[104],rfl,rfl⟩,rfl⟩
  have s1 : Step e1 (book 1) (book 2) := ⟨by decide,rfl,rfl⟩
  have s2 : Step e2 (book 2) (book 3) := ⟨by decide,rfl,rfl⟩
  have s3 : Step e3 (book 3) (book 4) :=
    ⟨by decide,⟨[],[100],rfl,rfl⟩,rfl⟩
  have s4 : Step e4 (book 4) (book 5) :=
    ⟨by decide,⟨[],[],rfl,rfl⟩,rfl⟩
  have s5 : Step e5 (book 5) (book 6) := ⟨by decide,rfl,rfl⟩
  have noquote : ¬ ∃ q, QuoteOf (book 5) q := by
    rintro ⟨q,hq⟩
    have h := hq.1.1
    simp [book] at h
  refine ⟨s0,s1,s2,s3,s4,s5,?_,?_,?_,?_,?_,?_,?_,noquote,?_,?_,?_,?_,?_,?_,?_,?_,?_,?_,?_,?_⟩
  all_goals first | exact (runs_properties (group_sound es)).1 | rfl | decide

theorem locate_blocks (start : Nat) (bs : List Block) :
    (locate start bs).map Located.block = bs := by
  induction bs generalizing start with
  | nil => rfl
  | cons b bs ih => simp [locate,ih]

theorem output_contract : OUTPUT_TARGET := by
  intro es qs
  constructor
  · rintro ⟨out,hc,_,_⟩
    by_cases h : qs.length = es.length+1
    · exact h
    · simp [construct,h] at hc
  · intro h
    let out := (locate 0 (group es)).map (fun l =>
      (⟨l,readout (positive l.block.first) l.start
        ((qs.drop l.start).take ((events l.block).length+1))⟩ : Output))
    refine ⟨out,by simp [construct,h,out],?_,?_⟩
    · simp [out,List.map_map,Function.comp_def]
    · simpa [out,List.map_map,Function.comp_def,locate_blocks] using (runs_properties (group_sound es)).1

theorem wire_witness : WIRE_WITNESS_TARGET := by
  let out := (locate 0 (group es)).map (fun l =>
    (⟨l,readout (positive l.block.first) l.start
      ((quotes.drop l.start).take ((events l.block).length+1))⟩ : Output))
  refine ⟨out,rfl,?_,?_,?_,by decide⟩
  all_goals dsimp [out]; decide

theorem exact_root : RUN_TARGET ∧ STEP_TARGET ∧ LEG_TARGET ∧ CAUSAL_TARGET ∧
    WITNESS_TARGET ∧ OUTPUT_TARGET ∧ WIRE_WITNESS_TARGET :=
  ⟨run_contract,step_contract,leg_contract,causal,witness,output_contract,wire_witness⟩

end MovingQuote

#print axioms MovingQuote.exact_root
