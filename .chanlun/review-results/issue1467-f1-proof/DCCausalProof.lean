import DCGeometryProof

namespace DCSpec

theorem drop_transfer {p q : Nat → Int} {delta : Int} {d : Dir} {s t k : Nat}
    (heq : ∀ i, i ≤ k → p i = q i) (ht : t ≤ k)
    (h : Drop p delta d s t) : Drop q delta d s t := by
  obtain ⟨hst, a, hsa, hat, hv⟩ := h
  refine ⟨hst, a, hsa, hat, ?_⟩
  rw [← heq a (by omega), ← heq t ht]
  exact hv

theorem wide_transfer {p q : Nat → Int} {delta : Int} {t k : Nat}
    (heq : ∀ i, i ≤ k → p i = q i) (ht : t ≤ k)
    (h : Wide p delta t) : Wide q delta t := by
  obtain ⟨a,b,ha,hb,hv⟩ := h
  refine ⟨a,b,ha,hb,?_⟩
  rw [← heq a (by omega), ← heq b (by omega)]
  exact hv

theorem extreme_transfer {p q : Nat → Int} {d : Dir} {s r e k : Nat}
    (heq : ∀ i, i ≤ k → p i = q i) (hr : r ≤ k)
    (h : Extreme p d s r e) : Extreme q d s r e := by
  obtain ⟨hse,her,hm,hf⟩ := h
  refine ⟨hse,her,?_,?_⟩
  · intro j hj hJ
    rw [← heq j (by omega), ← heq e (by omega)]
    exact hm j hj hJ
  · intro j hj hJ hp
    apply hf j hj hJ
    rw [heq j (by omega), heq e (by omega)]
    exact hp

theorem first_drop_transfer {p q : Nat → Int} {delta : Int} {d : Dir} {s t k : Nat}
    (heq : ∀ i, i ≤ k → p i = q i) (ht : t ≤ k)
    (h : First (Drop p delta d s) t) : First (Drop q delta d s) t := by
  refine ⟨drop_transfer heq ht h.1, ?_⟩
  intro j hj hdrop
  apply h.2 j hj
  exact drop_transfer (fun i hi => (heq i hi).symm) (by omega) hdrop

theorem initial_transfer {p q : Nat → Int} {delta : Int} {d : Dir} {s c k : Nat}
    (heq : ∀ i, i ≤ k → p i = q i) (hc : c ≤ k)
    (h : Initial p delta d s c) : Initial q delta d s c := by
  refine ⟨⟨wide_transfer heq hc h.1.1, ?_⟩, extreme_transfer heq hc h.2.1, ?_⟩
  · intro j hj hw
    exact h.1.2 j hj (wide_transfer (fun i hi => (heq i hi).symm) (by omega) hw)
  · rw [← heq c hc, ← heq s (by have hs := h.2.1.2.1; omega)]
    exact h.2.2

theorem filter_no_turn {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s k : Nat}
    {output : List Unit × Tail} (hd : 0 < delta) (hr : Run p delta n d s output)
    (hn : ∀ t, t ≤ k → ¬ Drop p delta d s t) :
    output.1.filter (fun u => u.knownAt ≤ k) = [] := by
  apply List.filter_eq_nil_iff.mpr
  intro u hu
  have h := run_known_after hd hr k hn u hu
  simpa using (show ¬ u.knownAt ≤ k by omega)

theorem run_prefix {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s : Nat}
    {output : List Unit × Tail} (hd : 0 < delta) (hr : Run p delta n d s output) :
    ∀ (q : Nat → Int) m k other, k ≤ n → k ≤ m →
      (∀ i, i ≤ k → p i = q i) → Run q delta m d s other →
      output.1.filter (fun u => u.knownAt ≤ k) =
      other.1.filter (fun u => u.knownAt ≤ k) := by
  induction hr with
  | stop hn he =>
    intro q m k other hkn hkm heq ho
    symm
    apply filter_no_turn hd ho
    intro t ht hdrop
    apply hn t (by omega)
    exact drop_transfer (fun i hi => (heq i hi).symm) ht hdrop
  | @next d s e t rest ht hf he hp hr ih =>
    intro q m k other hkn hkm heq ho
    by_cases htk : t ≤ k
    · have hfq := first_drop_transfer heq htk hf
      cases ho with
      | stop hn' he' => exact False.elim (hn' t (by omega) hfq.1)
      | next ht' hf' he' hp' hr' =>
        have htime := first_unique hf' hfq
        cases htime
        have heq' := extreme_unique he' (extreme_transfer heq (by omega) he)
        cases heq'
        have hh := ih q m k _ hkn hkm heq hr'
        simpa [List.filter_cons, htk] using congrArg (List.cons ⟨d,s,e,t⟩) hh
    · have hnp : ∀ u, u ≤ k → ¬ Drop p delta d s u := by
        intro u hu
        exact hf.2 u (by omega)
      have hnq : ∀ u, u ≤ k → ¬ Drop q delta d s u := by
        intro u hu hdrop
        exact hnp u hu (drop_transfer (fun i hi => (heq i hi).symm) hu hdrop)
      exact (filter_no_turn hd (Run.next ht hf he hp hr) hnp).trans
        (filter_no_turn hd ho hnq).symm

theorem whole_no_wide {p : Nat → Int} {delta : Int} {len k : Nat} {result : Result}
    (hd : 0 < delta) (h : Whole p delta len result) (hk : k < len)
    (hn : ∀ t, t ≤ k → ¬ Wide p delta t) :
    (units result).filter (fun u => u.knownAt ≤ k) = [] := by
  cases h with
  | empty => omega
  | waiting hn' hl hh => rfl
  | started hc hi hr =>
    apply filter_no_turn hd hr
    intro t ht hdrop
    exact hn t ht (drop_wide hdrop)

/-- DC-CAUSAL-v0：任意后缀的所有已确认单元前缀一致，含确认时刻。 -/
theorem dc_causal : DC_CAUSAL_TARGET := by
  intro p q delta hd len₁ len₂ k r₁ r₂ hk₁ hk₂ heq h₁ h₂
  cases h₁ with
  | empty => omega
  | @waiting n lo hi hn hl hh =>
    change [] = (units r₂).filter _
    symm
    apply whole_no_wide hd h₂ hk₂
    intro t ht hw
    exact hn t (by omega) (wide_transfer (fun i hi => (heq i hi).symm) ht hw)
  | @started n d s c output hc hi hr =>
    by_cases hck : c ≤ k
    · have hiq := initial_transfer heq hck hi
      cases h₂ with
      | empty => omega
      | waiting hn' hl' hh' => exact False.elim (hn' c (by omega) hiq.1.1)
      | @started m d' s' c' other hc' hi' hr' =>
        obtain ⟨hdir,hstart,htime⟩ := initial_unique hd hiq hi'
        cases hdir
        cases hstart
        cases htime
        exact run_prefix hd hr q m k other (by omega) (by omega) heq hr'
    · have hnp : ∀ t, t ≤ k → ¬ Wide p delta t := by
        intro t ht
        exact hi.1.2 t (by omega)
      have hnq : ∀ t, t ≤ k → ¬ Wide q delta t := by
        intro t ht hw
        exact hnp t ht (wide_transfer (fun i hi => (heq i hi).symm) ht hw)
      exact (whole_no_wide hd (Whole.started hc hi hr) hk₁ hnp).trans
        (whole_no_wide hd h₂ hk₂ hnq).symm

end DCSpec

#print axioms DCSpec.dc_causal
