import BookQuoteSpec

namespace BookQuote

open MovingQuote (Book Quote Event Step Update QuoteOf BestBid BestAsk)

theorem update_count (act price amount before after) (h : Update act price amount before after) (p : Int) :
    if act = .add then after.count p = before.count p + (List.replicate amount price).count p
    else before.count p = after.count p + (List.replicate amount price).count p := by
  by_cases ha : act = .add
  · simp only [Update,ha,if_true] at h
    simp only [ha,if_true]
    rw [h]
    simp [List.count_append,Nat.add_comm]
  · simp only [Update,ha,if_false] at h
    simp only [ha,if_false]
    rcases h with ⟨pre,post,hb,he⟩
    rw [hb,he]
    simp [List.count_append,Nat.add_assoc,Nat.add_comm,Nat.add_left_comm]

theorem update_preserves {act price amount a b aa bb}
    (hab : SameInventory a b) (ha : Update act price amount a aa)
    (hb : Update act price amount b bb) : SameInventory aa bb := by
  intro p
  have h0 := hab p
  have h1 := update_count _ _ _ _ _ ha p
  have h2 := update_count _ _ _ _ _ hb p
  by_cases hact : act = .add
  · simp only [hact,if_true] at h1 h2
    omega
  · simp only [hact,if_false] at h1 h2
    omega

theorem step_preserves {e a b aa bb} (hab : SameBook a b)
    (ha : Step e a aa) (hb : Step e b bb) : SameBook aa bb := by
  cases side : e.core.bidSide with
  | true =>
    have h1 : Update e.core.action e.price e.core.amount a.bids aa.bids ∧ aa.asks = a.asks := by simpa [side] using ha.2
    have h2 : Update e.core.action e.price e.core.amount b.bids bb.bids ∧ bb.asks = b.asks := by simpa [side] using hb.2
    refine ⟨update_preserves hab.1 h1.1 h2.1,?_⟩
    simpa only [h1.2,h2.2] using hab.2
  | false =>
    have h1 : Update e.core.action e.price e.core.amount a.asks aa.asks ∧ aa.bids = a.bids := by simpa [side] using ha.2
    have h2 : Update e.core.action e.price e.core.amount b.asks bb.asks ∧ bb.bids = b.bids := by simpa [side] using hb.2
    refine ⟨?_,update_preserves hab.2 h1.1 h2.1⟩
    simpa only [h1.2,h2.2] using hab.1

theorem inventory_mem {xs ys} (h : SameInventory xs ys) (p : Int) : p ∈ xs ↔ p ∈ ys := by
  rw [← List.count_pos_iff,← List.count_pos_iff,h p]

theorem same_symm {a b} (h : SameBook a b) : SameBook b a :=
  ⟨fun p => (h.1 p).symm,fun p => (h.2 p).symm⟩

theorem quote_forward {a b q} (h : SameBook a b) (hq : QuoteOf a q) : QuoteOf b q := by
  refine ⟨⟨?_,?_⟩,⟨?_,?_⟩,hq.2.2⟩
  · exact (inventory_mem h.1 q.bid).mp hq.1.1
  · intro p hp
    exact hq.1.2 p ((inventory_mem h.1 p).mpr hp)
  · exact (inventory_mem h.2 q.ask).mp hq.2.1.1
  · intro p hp
    exact hq.2.1.2 p ((inventory_mem h.2 p).mpr hp)

theorem read_unique {a b q r} (h : SameBook a b) (hq : Read a q) (hr : Read b r) : q = r := by
  cases q with
  | none =>
    cases r with
    | none => rfl
    | some r => exact False.elim (hq ⟨r,quote_forward (same_symm h) hr⟩)
  | some q =>
    cases r with
    | none => exact False.elim (hr ⟨q,quote_forward h hq⟩)
    | some r => exact congrArg some (MovingQuote.quote_unique (quote_forward h hq) hr)

theorem counts : COUNT_TARGET :=
  ⟨update_count,fun _ _ _ _ _ => step_preserves,fun _ _ _ _ => read_unique⟩

theorem prefix_counts {es fs n bs cs} (hp : es.take n = fs.take n)
    (h0 : SameBook (bs 0) (cs 0)) (hb : Through es bs n) (hc : Through fs cs n) :
    ∀ i, i ≤ n → SameBook (bs i) (cs i) := by
  intro i
  induction i with
  | zero => intro _; exact h0
  | succ i ih =>
    intro hi
    have hin : i < n := by omega
    rcases hb i hin with ⟨e,he,hs⟩
    rcases hc i hin with ⟨f,hf,ht⟩
    have hv := congrArg (fun xs : List Event => xs[i]?) hp
    rw [List.getElem?_take_of_lt hin,List.getElem?_take_of_lt hin,he,hf] at hv
    have ef : e = f := Option.some.inj hv
    subst f
    exact step_preserves (ih (by omega)) hs ht

theorem prefix_contract : PREFIX_TARGET := by
  intro es fs n bs cs hp h0 hb hc i hi
  have h := prefix_counts hp h0 hb hc i hi
  exact ⟨h,fun _ _ => read_unique h⟩

theorem wire_contract : WIRE_TARGET := by
  intro es bs cs qs rs h0 hb hc hq hr rb rc
  have he : qs = rs := by
    apply List.ext_getElem (by omega)
    intro i hi hj
    have hbq := rb i qs[i] (by simp [hi])
    have hcr := rc i rs[i] (by simp [hj])
    exact read_unique (prefix_counts rfl h0 hb hc i (by omega)) hbq hcr
  exact ⟨he,congrArg (MovingQuote.construct es) he⟩

theorem initial_same : SameBook (bookA 0) (bookB 0) := by
  constructor <;> intro p <;>
    simp [bookA,bookB,List.count_cons,Nat.add_assoc,Nat.add_comm,Nat.add_left_comm]

theorem throughA : Through events bookA 3 := by
  intro i hi
  have h : i = 0 ∨ i = 1 ∨ i = 2 := by omega
  rcases h with rfl | rfl | rfl
  · exact ⟨cancel100,rfl,⟨by decide,⟨[],[101,100],rfl,rfl⟩,rfl⟩⟩
  · exact ⟨cancel101,rfl,⟨by decide,⟨[],[100],rfl,rfl⟩,rfl⟩⟩
  · exact ⟨execute100,rfl,⟨by decide,⟨[],[],rfl,rfl⟩,rfl⟩⟩

theorem throughB : Through events bookB 3 := by
  intro i hi
  have h : i = 0 ∨ i = 1 ∨ i = 2 := by omega
  rcases h with rfl | rfl | rfl
  · exact ⟨cancel100,rfl,⟨by decide,⟨[],[100,101],rfl,rfl⟩,rfl⟩⟩
  · exact ⟨cancel101,rfl,⟨by decide,⟨[100],[],rfl,rfl⟩,rfl⟩⟩
  · exact ⟨execute100,rfl,⟨by decide,⟨[],[],rfl,rfl⟩,rfl⟩⟩

theorem noquoteA : Read (bookA 3) none := by
  rintro ⟨q,hq⟩
  have hm := hq.1.1
  simp [bookA] at hm
theorem noquoteB : Read (bookB 3) none := noquoteA

theorem readsA : Reads bookA quotes := by
  intro i q hq
  have hb : i < quotes.length := (List.getElem?_eq_some_iff.mp hq).1
  have hi : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 := by change i < 4 at hb; omega
  rcases hi with rfl | rfl | rfl | rfl
  · have h : q = some ⟨101,103⟩ := Option.some.inj hq.symm
    subst q
    change QuoteOf (bookA 0) ⟨101,103⟩
    decide
  · have h : q = some ⟨101,103⟩ := Option.some.inj hq.symm
    subst q
    change QuoteOf (bookA 1) ⟨101,103⟩
    decide
  · have h : q = some ⟨100,103⟩ := Option.some.inj hq.symm
    subst q
    change QuoteOf (bookA 2) ⟨100,103⟩
    decide
  · have h : q = none := Option.some.inj hq.symm
    subst q
    exact noquoteA

theorem readsB : Reads bookB quotes := by
  intro i q hq
  have hb : i < quotes.length := (List.getElem?_eq_some_iff.mp hq).1
  have hi : i ≤ 3 := by change i < 4 at hb; omega
  have hs := prefix_counts rfl initial_same throughA throughB i hi
  have ha := readsA i q hq
  cases q with
  | none =>
    rintro ⟨q,hv⟩
    exact ha ⟨q,quote_forward (same_symm hs) hv⟩
  | some q => exact quote_forward hs ha

theorem witness : WITNESS_TARGET := by
  refine ⟨by decide,initial_same,throughA,throughB,by decide,
    prefix_counts rfl initial_same throughA throughB 1 (by decide),
    readsA,readsB,by decide,noquoteA,noquoteB,?_⟩
  let out := (MovingQuote.locate 0 (MovingQuote.group events)).map (fun l =>
    (⟨l,MovingQuote.readout (MovingQuote.positive l.block.first) l.start
      ((quotes.drop l.start).take ((MovingQuote.events l.block).length+1))⟩ : MovingQuote.Output))
  refine ⟨out,rfl,?_,?_⟩
  all_goals dsimp [out]; decide

theorem exact_root : COUNT_TARGET ∧ PREFIX_TARGET ∧ WIRE_TARGET ∧ WITNESS_TARGET :=
  ⟨counts,prefix_contract,wire_contract,witness⟩

end BookQuote

#print axioms BookQuote.exact_root
