import DCCanonicalProof

namespace DCSpec

theorem turn_no_flip_before {p : Nat → Int} {delta : Int} {d : Dir} {s e t : Nat}
    (hd : 0 < delta) (hf : First (Drop p delta d s) t)
    (he : Extreme p d s (t-1) e) : ∀ u, u ≤ t → ¬ Drop p delta (flip d) e u := by
  obtain ⟨_, k₀, hsk, hkt, hv⟩ := hf.1
  have hmax := he.2.2.1 k₀ hsk (by omega)
  have hfall : delta ≤ score d (p e) - score d (p t) := by omega
  intro u hu hdrop
  obtain ⟨_, k, hek, hku, hv'⟩ := hdrop
  simp only [score_flip] at hv'
  have hub : score d (p u) ≤ score d (p e) := by
    by_cases h : u = t
    · subst u; omega
    · exact he.2.2.1 u (by have hh := he.1; omega) (by omega)
  by_cases hke : k = e
  · subst k; omega
  · apply hf.2 k (by omega)
    exact ⟨by have hh := he.1; omega, e, he.1, by omega, by omega⟩

theorem run_known_after {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s : Nat}
    {output : List Unit × Tail} (hd : 0 < delta) (hr : Run p delta n d s output) :
    ∀ c, (∀ t, t ≤ c → ¬ Drop p delta d s t) →
      ∀ u ∈ output.1, c < u.knownAt := by
  induction hr with
  | stop hn he => simp
  | @next d s e t rest ht hf he hp hr ih =>
    intro c hc u hu
    have hct : c < t := by
      apply Classical.byContradiction
      intro h
      exact hc t (by omega) hf.1
    simp only [List.mem_cons] at hu
    rcases hu with rfl | hu
    · exact hct
    · have h := ih t (turn_no_flip_before hd hf he) u hu
      omega

theorem turn_geometry {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s e t : Nat}
    (hd : 0 < delta) (hs : Seeded p delta d s n) (ht : t ≤ n)
    (hf : First (Drop p delta d s) t) (he : Extreme p d s (t-1) e) :
    UnitGeometry p delta (n+1) ⟨d,s,e,t⟩ := by
  have hse := turn_progress hd hs hf he
  have het : e < t := by have hh := he.2.1; have hst := hf.1.1; omega
  obtain ⟨c, hsc, hcn, hv, hn, hlo⟩ := hs
  have hct : c < t := by
    apply Classical.byContradiction
    intro h
    exact hn t (by omega) hf.1
  have hmax := he.2.2.1 c (by omega) (by omega)
  refine ⟨hse, het, by dsimp; omega, by dsimp; omega, ?_⟩
  intro j hj hJ
  dsimp at hj hJ ⊢
  refine ⟨?_, he.2.2.1 j hj (by omega)⟩
  by_cases hjc : j ≤ c
  · exact hlo j hj hjc
  · apply Classical.byContradiction
    intro h
    apply hf.2 j (by omega)
    exact ⟨by omega, c, by omega, by omega, by omega⟩

theorem run_first_shape {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s : Nat}
    {output : List Unit × Tail} {a : Unit} {rest : List Unit}
    (hr : Run p delta n d s output) (h : output.1 = a :: rest) :
    a.direction = d ∧ a.start = s := by
  cases hr with
  | stop hn he => simp at h
  | next ht hf he hp hr =>
    have ha := (List.cons.inj h).1
    cases ha
    exact ⟨rfl, rfl⟩

theorem run_geometry {p : Nat → Int} {delta : Int} {n : Nat} {d : Dir} {s : Nat}
    {output : List Unit × Tail} (hd : 0 < delta) (hr : Run p delta n d s output) :
    Seeded p delta d s n →
    (∀ u ∈ output.1, UnitGeometry p delta (n+1) u) ∧
    output.1.Pairwise (fun a b => a.knownAt < b.knownAt) ∧
    ∀ pre a b post, output.1 = pre ++ a :: b :: post →
      a.finish = b.start ∧ flip a.direction = b.direction := by
  induction hr with
  | stop hn he =>
    intro _
    refine ⟨by simp, by simp, ?_⟩
    intro pre a b post h
    have hh := congrArg List.length h
    simp at hh
  | @next d s e t rest ht hf he hp hr ih =>
    intro hs
    obtain ⟨hg, htimes, hlinks⟩ := ih (turn_seeded hd ht hf he)
    refine ⟨?_, ?_, ?_⟩
    · intro u hu
      simp only [List.mem_cons] at hu
      rcases hu with rfl | hu
      · exact turn_geometry hd hs ht hf he
      · exact hg u hu
    · exact List.Pairwise.cons (run_known_after hd hr t (turn_no_flip_before hd hf he)) htimes
    · intro pre a b post h
      cases pre with
      | nil =>
        simp only [List.nil_append] at h
        obtain ⟨ha, htail⟩ := List.cons.inj h
        obtain ⟨hbdir, hbstart⟩ := run_first_shape hr htail
        cases ha
        exact ⟨hbstart.symm, hbdir.symm⟩
      | cons x pre =>
        simp only [List.cons_append] at h
        exact hlinks pre a b post (List.cons.inj h).2

/-- DC-GEOMETRY-v0，包括全部已完成单元和接续关系。 -/
theorem dc_geometry : DC_GEOMETRY_TARGET := by
  intro p delta hd len result h
  cases h with
  | empty =>
    refine ⟨by simp [units], by simp [units], ?_⟩
    intro pre a b post he
    have hlen := congrArg List.length he
    simp [units] at hlen
  | waiting hn hl hh =>
    refine ⟨by simp [units], by simp [units], ?_⟩
    intro pre a b post he
    have hlen := congrArg List.length he
    simp [units] at hlen
  | started hc hi hr => exact run_geometry hd hr (initial_seeded hd hi hc)

end DCSpec

#print axioms DCSpec.dc_geometry
