import DirectOutputProof
import DirectOutputSourcePathSpec

namespace DirectOutput
open MovingQuote NewChanlun.Origin

theorem actual_quotes {es bs qs out x s}
    (hr : BookQuote.Reads bs qs) (hlen : qs.length = es.length+1)
    (hc : construct es qs = some out) (hx : x ∈ out) (hg : x.geometry = some s) :
    ∀ k, x.located.start ≤ k → k ≤ x.located.finish →
      ∃ q, qs[k]? = some (some q) ∧ QuoteOf (bs k) q := by
  have hd := member_data hc hx
  rcases source hd.1 with ⟨pre,post,hsrc,hstart,hfinish,_⟩
  have hn := congrArg List.length hsrc
  simp only [List.length_append] at hn
  let window := (qs.drop x.located.start).take ((events x.located.block).length+1)
  have hwlen : window.length = (events x.located.block).length+1 := by
    simp only [window,List.length_take,List.length_drop]
    apply Nat.min_eq_left
    omega
  have hread : readout (positive x.located.block.first) x.located.start window = some s := by
    have hh := congrArg Output.geometry hd.2
    exact hh.symm.trans hg
  have hall := (readout_some hread).1
  intro k hs he
  have hi : k-x.located.start < window.length := by omega
  have hw := List.getElem?_eq_getElem hi
  have ready := List.all_eq_true.mp hall window[k-x.located.start] (List.getElem_mem hi)
  cases hwi : window[k-x.located.start] with
  | none => simp [hwi,quoteReady] at ready
  | some v =>
    rw [hwi] at hw
    have heq : qs[k]? = some (some v) := by
      rw [List.getElem?_take_of_lt (by omega),List.getElem?_drop] at hw
      simpa only [Nat.add_sub_of_le hs] using hw
    exact ⟨v,heq,hr k (some v) heq⟩

theorem monotone_refl (pos : Bool) (q : Quote) : Monotone pos q q := by
  cases pos <;> simp [Monotone]

theorem monotone_trans {pos q r s} (h : Monotone pos q r) (g : Monotone pos r s) :
    Monotone pos q s := by
  cases pos <;> simp [Monotone] at h g ⊢ <;> omega

theorem monotone_between (pos : Bool) (f : Nat → Quote) (n : Nat)
    (hs : ∀ i, i < n → Monotone pos (f i) (f (i+1))) :
    ∀ i j, i ≤ j → j ≤ n → Monotone pos (f i) (f j) := by
  intro i j
  induction j with
  | zero =>
    intro hi hj
    have he : i = 0 := by omega
    subst i
    exact monotone_refl _ _
  | succ j ih =>
    intro hi hj
    by_cases hij : i ≤ j
    · exact monotone_trans (ih hij (by omega)) (hs j (by omega))
    · have he : i = j+1 := by omega
      subst i
      exact monotone_refl _ _

theorem actual_exact_root : ACTUAL_PATH_TARGET := by
  intro es bs qs out ht hr hlen hc x hx s hg
  have ha := actual_quotes hr hlen hc hx hg
  have hd := member_data hc hx
  rcases source hd.1 with ⟨pre,post,hsrc,hstart,hfinish,hu⟩
  have hn := congrArg List.length hsrc
  simp only [List.length_append] at hn
  rcases hull_contract es bs qs out ht hr hlen hc x hx s hg with
    ⟨q,r,hq,hrq,hqb,hrb,hf,hseg,_⟩
  have hqr : ∀ i, i ≤ (events x.located.block).length →
      QuoteOf (bs (x.located.start+i)) (quoteAt qs (x.located.start+i)) := by
    intro i hi
    rcases ha (x.located.start+i) (by omega) (by omega) with ⟨v,hv,hvb⟩
    simpa only [quoteAt_eq hv] using hvb
  have hsteps : ∀ i, i < (events x.located.block).length →
      Monotone (positive x.located.block.first) (quoteAt qs (x.located.start+i))
        (quoteAt qs (x.located.start+(i+1))) := by
    intro i hi
    rcases ht (x.located.start+i) (by omega) with ⟨e,he,hstep⟩
    have he' : (events x.located.block)[i]? = some e := by
      rw [hsrc,← hstart,owned_lookup hi] at he
      exact he
    have hp := hu e (List.mem_of_getElem? he')
    have hm := quote_step hstep (hqr i (by omega))
      (show QuoteOf (bs ((x.located.start+i)+1)) (quoteAt qs (x.located.start+(i+1))) by
        simpa only [Nat.add_assoc] using hqr (i+1) (by omega))
    simpa only [hp] using hm
  have hmono := monotone_between (positive x.located.block.first)
    (fun i => quoteAt qs (x.located.start+i)) (events x.located.block).length hsteps
  have hb : ∀ z, ActualSupport bs qs x.located.start x.located.finish z →
      (if positive x.located.block.first then q.bid else r.bid) ≤ z ∧
      z ≤ (if positive x.located.block.first then r.ask else q.ask) := by
    rintro z ⟨k,v,hks,hke,hkv,hvb,hlo,hhi⟩
    have hleft := hmono 0 (k-x.located.start) (by omega) (by omega)
    have hright := hmono (k-x.located.start) (events x.located.block).length (by omega) (by omega)
    have hidx : x.located.start+(k-x.located.start) = k := by omega
    simp only [Nat.add_zero,hidx,← hfinish,quoteAt_eq hq,quoteAt_eq hrq,quoteAt_eq hkv] at hleft hright
    cases hp : positive x.located.block.first <;> simp [hp,Monotone] at hleft hright ⊢ <;> omega
  have hse : x.located.start ≤ x.located.finish := by omega
  have sqb : ActualSupport bs qs x.located.start x.located.finish q.bid :=
    ⟨x.located.start,q,by omega,hse,hq,hqb,by omega,by have := hqb.2.2; omega⟩
  have sqa : ActualSupport bs qs x.located.start x.located.finish q.ask :=
    ⟨x.located.start,q,by omega,hse,hq,hqb,by have := hqb.2.2; omega,by omega⟩
  have srb : ActualSupport bs qs x.located.start x.located.finish r.bid :=
    ⟨x.located.finish,r,hse,by omega,hrq,hrb,by omega,by have := hrb.2.2; omega⟩
  have sra : ActualSupport bs qs x.located.start x.located.finish r.ask :=
    ⟨x.located.finish,r,hse,by omega,hrq,hrb,by have := hrb.2.2; omega,by omega⟩
  refine ⟨ha,?_⟩
  rw [hseg]
  have hm := flow_monotone hf
  cases hp : positive x.located.block.first with
  | false =>
    have lt : r.bid < q.ask := by have := hqb.2.2; simp [hp,Monotone] at hm; omega
    simp only [leg, Bool.false_eq_true, ↓reduceIte, segLow, segHigh]
    simp only [show ¬ q.ask ≤ r.bid by omega, show q.ask ≥ r.bid by omega, ↓reduceIte]
    exact ⟨srb,sqa,by simpa only [hp,Bool.false_eq_true,↓reduceIte] using hb⟩
  | true =>
    have lt : q.bid < r.ask := by have := hqb.2.2; simp [hp,Monotone] at hm; omega
    simp only [leg, ↓reduceIte, segLow, segHigh]
    simp only [show q.bid ≤ r.ask by omega, show ¬ q.bid ≥ r.ask by omega, ↓reduceIte]
    exact ⟨sqb,sra,by simpa only [hp,↓reduceIte] using hb⟩

end DirectOutput

#print axioms DirectOutput.actual_exact_root
