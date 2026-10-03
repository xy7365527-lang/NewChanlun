import DCSemantics
import DirectionalChangeLemmas

/- #1467 独立关系的证明；DCSemantics.lean已先冻结，不能为证明改定义。 -/
namespace DCSpec

theorem score_flip (d : Dir) (x : Int) : score (flip d) x = -score d x := by
  cases d <;> simp [flip, score]

theorem score_injective (d : Dir) {a b : Int} (h : score d a = score d b) : a = b := by
  cases d <;> simp_all [score]

theorem extreme_exists (p : Nat → Int) (d : Dir) (s r : Nat) (hs : s ≤ r) :
    ∃ e, Extreme p d s r e := by
  let q := fun k => score d (p (s+k))
  let k := OrderBaseDC.maxIndex q (r-s)
  obtain ⟨hk, hm, hf⟩ := OrderBaseDC.maxIndex_spec q (r-s)
  change k ≤ r-s at hk
  refine ⟨s+k, by omega, by omega, ?_, ?_⟩
  · intro j hj hJ
    have h := hm (j-s) (by omega)
    have hj' : s+(j-s) = j := by omega
    simpa [q, hj', k] using h
  · intro j hj hJ hp
    have h := hf (j-s) (by omega)
    have hj' : s+(j-s) = j := by omega
    apply h
    change score d (p (s+(j-s))) = score d (p (s+k))
    rw [hj', hp]

theorem extreme_unique {p : Nat → Int} {d : Dir} {s r e f : Nat}
    (he : Extreme p d s r e) (hf : Extreme p d s r f) : e = f := by
  have h₁ := he.2.2.1 f hf.1 hf.2.1
  have h₂ := hf.2.2.1 e he.1 he.2.1
  have hp : p e = p f := score_injective d (by omega)
  by_cases h : e < f
  · exact False.elim (hf.2.2.2 e he.1 h hp)
  · by_cases h' : f < e
    · exact False.elim (he.2.2.2 f hf.1 h' hp.symm)
    · omega

theorem first_unique {P : Nat → Prop} {s t : Nat} (hs : First P s) (ht : First P t) :
    s = t := by
  by_cases h : s < t
  · exact False.elim (ht.2 s h hs.1)
  · by_cases h' : t < s
    · exact False.elim (hs.2 t h' ht.1)
    · omega

theorem first_or_none (P : Nat → Prop) (n : Nat) :
    (∃ t, t ≤ n ∧ First P t) ∨ (∀ t, t ≤ n → ¬ P t) := by
  classical
  induction n with
  | zero =>
    by_cases h : P 0
    · exact Or.inl ⟨0, by omega, h, by intro j hj; omega⟩
    · right
      intro t ht
      have he : t = 0 := by omega
      simpa [he] using h
  | succ n ih =>
    rcases ih with ⟨t, ht, hf⟩ | hn
    · exact Or.inl ⟨t, by omega, hf⟩
    · by_cases h : P (n+1)
      · exact Or.inl ⟨n+1, by omega, h, fun j hj => hn j (by omega)⟩
      · right
        intro t ht
        by_cases he : t = n+1
        · simpa [he] using h
        · exact hn t (by omega)

theorem wide_of_score_pair {p : Nat → Int} {delta : Int} (d : Dir)
    {a b t : Nat} (ha : a ≤ t) (hb : b ≤ t)
    (h : delta ≤ score d (p a) - score d (p b)) : Wide p delta t := by
  cases d with
  | up => exact ⟨a, b, ha, hb, h⟩
  | down =>
    simp only [score] at h
    exact ⟨b, a, hb, ha, by omega⟩

theorem drop_wide {p : Nat → Int} {delta : Int} {d : Dir} {s t : Nat}
    (h : Drop p delta d s t) : Wide p delta t := by
  obtain ⟨_, k, _, hk, hv⟩ := h
  exact wide_of_score_pair d (by omega) (by omega) hv

theorem initial_exists {p : Nat → Int} {delta : Int} {c : Nat}
    (hd : 0 < delta) (hc : First (Wide p delta) c) :
    ∃ d s, Initial p delta d s c := by
  obtain ⟨a, b, ha, hb, hv⟩ := hc.1
  have hpos : 0 < c := by
    apply Classical.byContradiction
    intro hn
    have hA : a = 0 := by omega
    have hB : b = 0 := by omega
    rw [hA, hB] at hv
    omega
  by_cases he : a = c
  · obtain ⟨s, hs⟩ := extreme_exists p .down 0 c (by omega)
    have hm := hs.2.2.1 b (by omega) hb
    simp only [score] at hm
    exact ⟨.up, s, hc, hs, by simp only [score]; rw [he] at hv; omega⟩
  · by_cases he' : b = c
    · obtain ⟨s, hs⟩ := extreme_exists p .up 0 c (by omega)
      have hm := hs.2.2.1 a (by omega) ha
      simp only [score] at hm
      exact ⟨.down, s, hc, hs, by simp only [score]; rw [he'] at hv; omega⟩
    · exact False.elim (hc.2 (c-1) (by omega) ⟨a,b,by omega,by omega,hv⟩)

theorem initial_seeded {p : Nat → Int} {delta : Int} {d : Dir} {s c n : Nat}
    (hd : 0 < delta) (hi : Initial p delta d s c) (hc : c ≤ n) :
    Seeded p delta d s n := by
  have hsc : s < c := by
    have hle := hi.2.1.2.1
    have hv := hi.2.2
    apply Classical.byContradiction
    intro h
    have he : s = c := by omega
    rw [he] at hv
    omega
  refine ⟨c, hsc, hc, hi.2.2, ?_, ?_⟩
  · intro t ht hdrop
    by_cases htc : t < c
    · exact hi.1.2 t htc (drop_wide hdrop)
    · have hEq : t = c := by omega
      subst t
      obtain ⟨_, k, _, hk, hv⟩ := hdrop
      have hs := hi.2.2
      apply hi.1.2 (c-1) (by omega)
      exact wide_of_score_pair d (a:=k) (b:=s) (by omega) (by omega) (by omega)
  · intro j _ hj
    have h := hi.2.1.2.2.1 j (by omega) hj
    simp only [score_flip] at h
    omega

theorem initial_unique {p : Nat → Int} {delta : Int} {d d' : Dir} {s s' c c' : Nat}
    (hd : 0 < delta) (h : Initial p delta d s c) (h' : Initial p delta d' s' c') :
    d' = d ∧ s' = s ∧ c' = c := by
  have hcc : c' = c := first_unique h'.1 h.1
  subst c'
  have hsc : s < c := by
    obtain ⟨v, hsv, hvc, _, _, _⟩ := initial_seeded hd h (Nat.le_refl c)
    omega
  have hs'c : s' < c := by
    obtain ⟨v, hsv, hvc, _, _, _⟩ := initial_seeded hd h' (Nat.le_refl c)
    omega
  by_cases hd' : d' = d
  · subst d'
    exact ⟨rfl, extreme_unique h'.2.1 h.2.1, rfl⟩
  · have hflip : d' = flip d := by cases d <;> cases d' <;> simp_all [flip]
    have hv := h.2.2
    have hv' := h'.2.2
    rw [hflip, score_flip, score_flip] at hv'
    apply False.elim
    apply h.1.2 (c-1) (by omega)
    exact wide_of_score_pair d (a:=s') (b:=s) (by omega) (by omega) (by omega)

theorem turn_seeded {p : Nat → Int} {delta : Int} {d : Dir} {s e t n : Nat}
    (hd : 0 < delta) (ht : t ≤ n) (hf : First (Drop p delta d s) t)
    (he : Extreme p d s (t-1) e) : Seeded p delta (flip d) e n := by
  have het : e < t := by have hst := hf.1.1; have hh := he.2.1; omega
  obtain ⟨_, k₀, hsk, hkt, hv⟩ := hf.1
  have hmax := he.2.2.1 k₀ hsk (by omega)
  have hfall : delta ≤ score d (p e) - score d (p t) := by omega
  have bound : ∀ j, e ≤ j → j ≤ t → score d (p j) ≤ score d (p e) := by
    intro j hj hJ
    by_cases h : j = t
    · subst j; omega
    · exact he.2.2.1 j (by have hh := he.1; omega) (by omega)
  refine ⟨t, het, ht, ?_, ?_, ?_⟩
  · simp only [score_flip]; omega
  · intro u hu hdrop
    obtain ⟨_, k, hek, hku, hv'⟩ := hdrop
    simp only [score_flip] at hv'
    have hub := bound u (by omega) hu
    by_cases hke : k = e
    · subst k; omega
    · apply hf.2 k (by omega)
      refine ⟨by have hh := he.1; omega, e, he.1, by omega, ?_⟩
      omega
  · intro j hj hJ
    have h := bound j hj hJ
    simp only [score_flip]; omega

theorem turn_progress {p : Nat → Int} {delta : Int} {d : Dir} {s e t n : Nat}
    (hd : 0 < delta) (hs : Seeded p delta d s n)
    (hf : First (Drop p delta d s) t) (he : Extreme p d s (t-1) e) : s < e := by
  obtain ⟨c, hsc, _, hv, hn, _⟩ := hs
  have hct : c < t := by
    apply Classical.byContradiction
    intro h
    exact hn t (by omega) hf.1
  have hmax := he.2.2.1 c (by omega) (by omega)
  have hse := he.1
  apply Classical.byContradiction
  intro h
  have hes : e = s := by omega
  rw [hes] at hmax
  omega

theorem run_exists {p : Nat → Int} {delta : Int} {n : Nat} (hd : 0 < delta)
    (s : Nat) (d : Dir) (hs : s ≤ n) (hseed : Seeded p delta d s n) :
    ∃ output, Run p delta n d s output := by
  have aux : ∀ m s d, n-s = m → s ≤ n → Seeded p delta d s n →
      ∃ output, Run p delta n d s output := by
    intro m
    induction m using Nat.strongRecOn with
    | ind m ih =>
      intro s d hm hsn hseed
      rcases first_or_none (Drop p delta d s) n with ⟨t, ht, hf⟩ | hn
      · obtain ⟨e, he⟩ := extreme_exists p d s (t-1) (by have hst := hf.1.1; omega)
        have hse := turn_progress hd hseed hf he
        have hen : e ≤ n := by have hh := he.2.1; omega
        have hnew := turn_seeded hd ht hf he
        obtain ⟨rest, hr⟩ := ih (n-e) (by omega) e (flip d) rfl hen hnew
        exact ⟨_, Run.next ht hf he hse hr⟩
      · obtain ⟨e, he⟩ := extreme_exists p d s n hsn
        exact ⟨_, Run.stop hn he⟩
  exact aux (n-s) s d rfl hs hseed

theorem run_unique {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s : Nat}
    {output : List Unit × Tail} (h : Run p delta n d s output) :
    ∀ other, Run p delta n d s other → other = output := by
  induction h with
  | stop hn he =>
    intro other hh
    cases hh with
    | stop hn' he' =>
      have hEq := extreme_unique he' he
      cases hEq
      rfl
    | next ht hf he' hp hr => exact False.elim (hn _ ht hf.1)
  | next ht hf he hp hr ih =>
    intro other hh
    cases hh with
    | stop hn' he' => exact False.elim (hn' _ ht hf.1)
    | next ht' hf' he' hp' hr' =>
      have htime := first_unique hf' hf
      cases htime
      have hEq := extreme_unique he' he
      cases hEq
      have hrest := ih _ hr'
      cases hrest
      rfl

theorem whole_exists (p : Nat → Int) (delta : Int) (hd : 0 < delta) (len : Nat) :
    ∃ result, Whole p delta len result := by
  cases len with
  | zero => exact ⟨.empty, Whole.empty⟩
  | succ n =>
    rcases first_or_none (Wide p delta) n with ⟨c, hc, hf⟩ | hn
    · obtain ⟨d, s, hi⟩ := initial_exists hd hf
      have hseed := initial_seeded hd hi hc
      have hsn : s ≤ n := by have hh := hi.2.1.2.1; omega
      obtain ⟨output, hr⟩ := run_exists hd s d hsn hseed
      exact ⟨_, Whole.started hc hi hr⟩
    · obtain ⟨lo, hl⟩ := extreme_exists p .down 0 n (by omega)
      obtain ⟨hi, hh⟩ := extreme_exists p .up 0 n (by omega)
      exact ⟨_, Whole.waiting hn hl hh⟩

theorem whole_unique {p : Nat → Int} {delta : Int} {len : Nat} {r₁ r₂ : Result}
    (hd : 0 < delta) (h₁ : Whole p delta len r₁) (h₂ : Whole p delta len r₂) : r₂ = r₁ := by
  cases h₁ with
  | empty => cases h₂; rfl
  | waiting hn hl hh =>
    cases h₂ with
    | waiting hn' hl' hh' =>
      have hlo := extreme_unique hl' hl
      have hhi := extreme_unique hh' hh
      cases hlo
      cases hhi
      rfl
    | started hc hi hr => exact False.elim (hn _ hc hi.1.1)
  | started hc hi hr =>
    cases h₂ with
    | waiting hn hl hh => exact False.elim (hn _ hc hi.1.1)
    | started hc' hi' hr' =>
      obtain ⟨hdir, hstart, htime⟩ := initial_unique hd hi hi'
      cases hdir
      cases hstart
      cases htime
      have hout := run_unique hr _ hr'
      cases hout
      rfl

/-- DC-CANON-v0的完整存在唯一性。定义与前提固定于DCSemantics。 -/
theorem dc_canon : DC_CANON_TARGET := by
  intro p delta hd len
  obtain ⟨result, hr⟩ := whole_exists p delta hd len
  exact ⟨result, hr, fun _ ho => whole_unique hd hr ho⟩

end DCSpec

#print axioms DCSpec.dc_canon
