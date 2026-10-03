import DirectQuoteSpec

namespace DirectQuote

open NewChanlun.Origin

instance (q : Queues) : Decidable (PositiveQueues q) := by unfold PositiveQueues; infer_instance
def validHistoryDecidable (q : Queues) : (es : List Event) → Decidable (ValidHistory q es)
  | [] => by unfold ValidHistory; infer_instance
  | e::es => by
    unfold ValidHistory
    letI := validHistoryDecidable (advance q e) es
    infer_instance
instance (q : Queues) (es : List Event) : Decidable (ValidHistory q es) := validHistoryDecidable q es
instance (p : Nat → Event) (n s e : Nat) : Decidable (Completed p n s e) := by
  unfold Completed; infer_instance
instance (a b c : Segment) : Decidable (CenterConfirmedComplete a b c) := by
  unfold CenterConfirmedComplete; infer_instance

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

theorem completed_transport {p q : Nat → Event} {n s e : Nat}
    (hp : ∀ i, i < n → p i = q i) (h : Completed p n s e) : Completed q n s e := by
  rcases h with ⟨hse,hen,hleft,hmiddle,hright⟩
  have hsn : s < n := by omega
  refine ⟨hse,hen,?_,?_,?_⟩
  · rcases hleft with hzero | hneq
    · exact Or.inl hzero
    · right
      simpa only [hp (s-1) (by omega), hp s hsn] using hneq
  · intro i hi
    simpa only [hp i.val (by have hh:=i.isLt; omega),hp s hsn] using hmiddle i hi
  · simpa only [hp (e-1) (by omega),hp e hen] using hright

theorem causal : CAUSAL_TARGET := by
  intro p q n s e hp
  exact ⟨completed_transport hp,completed_transport (fun i hi => (hp i hi).symm)⟩

theorem leg_range (bid ask : Int) (h : bid < ask) (start : Nat) (b : Block) :
    (leg bid ask start b).startIndex < (leg bid ask start b).endIndex ∧
    segLow (leg bid ask start b) = bid ∧ segHigh (leg bid ask start b) = ask := by
  have hle : bid ≤ ask := by omega
  have hn : ¬ ask ≤ bid := by omega
  cases hb : positive b.first <;>
    simp [leg,hb,segLow,segHigh,events,Tick,hle,hn] <;> omega

theorem leg_dir_ne (bid ask : Int) (s t : Nat) (a b : Block)
    (h : positive a.first ≠ positive b.first) :
    (leg bid ask s a).direction ≠ (leg bid ask t b).direction := by
  cases ha : positive a.first <;> cases hb : positive b.first <;> simp_all [leg]

theorem geometry : GEOMETRY_TARGET := by
  refine ⟨leg_range,?_,?_⟩
  · intro bid ask start a b h
    cases ha : positive a.first <;> cases hb : positive b.first <;> simp_all [leg]
  · intro bid ask h start a b c hab hbc
    dsimp
    have ra := leg_range bid ask h start a
    have rb := leg_range bid ask h (start+(events a).length) b
    have rc := leg_range bid ask h (start+(events a).length+(events b).length) c
    have hlo : computeZD (leg bid ask start a) (leg bid ask (start+(events a).length) b)
        (leg bid ask (start+(events a).length+(events b).length) c) = bid := by
      simp [computeZD,ra.2.1,rb.2.1,rc.2.1,tmax]
    have hhi : computeZG (leg bid ask start a) (leg bid ask (start+(events a).length) b)
        (leg bid ask (start+(events a).length+(events b).length) c) = ask := by
      simp [computeZG,ra.2.2,rb.2.2,rc.2.2,tmin]
    refine ⟨⟨⟨leg_dir_ne bid ask _ _ a b hab,leg_dir_ne bid ask _ _ b c hbc⟩,?_⟩,hlo,hhi⟩
    rw [hlo,hhi]
    exact h

theorem witness : WITNESS_TARGET := by
  refine ⟨by decide,by decide,by decide,rfl,rfl,by decide,by decide,by decide,
    by decide,by decide,by decide,?_⟩
  intro qty hqty
  simp at hqty
  rcases hqty with rfl | rfl <;> decide

theorem exact_root : RUN_TARGET ∧ CAUSAL_TARGET ∧ GEOMETRY_TARGET ∧ WITNESS_TARGET :=
  ⟨run_contract,causal,geometry,witness⟩

end DirectQuote

#print axioms DirectQuote.exact_root
