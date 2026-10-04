import DirectHullSpec

namespace DirectHull
open MovingQuote NewChanlun.Origin

theorem split_flow {pos a q b r es} (h : Flow pos a q b r es) :
    ∀ pre post, es = pre ++ post → ∃ mid v, Flow pos a q mid v pre ∧ Flow pos mid v b r post := by
  induction h with
  | nil b q hq =>
    intro pre post he
    have hp : pre = [] := (List.append_eq_nil_iff.mp he.symm).1
    have ht : post = [] := (List.append_eq_nil_iff.mp he.symm).2
    subst pre; subst post
    exact ⟨b,q,Flow.nil b q hq,Flow.nil b q hq⟩
  | step e a q b r c s es hs hq hr hp hf ih =>
    intro pre post he
    cases pre with
    | nil =>
      simp only [List.nil_append] at he
      subst post
      exact ⟨a,q,Flow.nil a q hq,Flow.step e a q b r c s es hs hq hr hp hf⟩
    | cons e' pre =>
      simp only [List.cons_append, List.cons.injEq] at he
      rcases he with ⟨he,ht⟩
      subst e'
      rcases ih pre post ht with ⟨mid,v,hl,hrest⟩
      exact ⟨mid,v,Flow.step e a q b r mid v pre hs hq hr hp hl,hrest⟩

theorem support_bounds {pos a q b r es x} (h : Support pos a q b r es x) :
    (if pos then q.bid else r.bid) ≤ x ∧ x ≤ (if pos then r.ask else q.ask) := by
  rcases h with ⟨pre,post,mid,v,_,hl,hr,hlo,hhi⟩
  have ml := flow_monotone hl
  have mr := flow_monotone hr
  cases pos <;> simp [Monotone] at ml mr ⊢ <;> omega

theorem hull {pos a q b r es} (h : Flow pos a q b r es) (start : Nat) :
    ExactHull (Support pos a q b r es)
      (segLow (leg pos q r start es.length)) (segHigh (leg pos q r start es.length)) := by
  have hq := (flow_quotes h).1
  have hr := (flow_quotes h).2
  have mono := flow_monotone h
  have sqb : Support pos a q b r es q.bid :=
    ⟨[],es,a,q,rfl,Flow.nil a q hq,h,by omega,by have := hq.2.2; omega⟩
  have sqa : Support pos a q b r es q.ask :=
    ⟨[],es,a,q,rfl,Flow.nil a q hq,h,by have := hq.2.2; omega,by omega⟩
  have srb : Support pos a q b r es r.bid :=
    ⟨es,[],b,r,by simp,h,Flow.nil b r hr,by omega,by have := hr.2.2; omega⟩
  have sra : Support pos a q b r es r.ask :=
    ⟨es,[],b,r,by simp,h,Flow.nil b r hr,by have := hr.2.2; omega,by omega⟩
  cases pos with
  | false =>
    have lt : r.bid < q.ask := by have := hq.2.2; simp [Monotone] at mono; omega
    simp only [leg, Bool.false_eq_true, ↓reduceIte, segLow, segHigh]
    simp only [show ¬ q.ask ≤ r.bid by omega, show q.ask ≥ r.bid by omega, ↓reduceIte]
    exact ⟨srb,sqa,fun x hx => support_bounds hx⟩
  | true =>
    have lt : q.bid < r.ask := by have := hq.2.2; simp [Monotone] at mono; omega
    simp only [leg, ↓reduceIte, segLow, segHigh]
    simp only [show q.bid ≤ r.ask by omega, show ¬ q.bid ≥ r.ask by omega, ↓reduceIte]
    exact ⟨sqb,sra,fun x hx => support_bounds hx⟩

theorem limit : LIMIT_TARGET := by
  have hd : Step deep b0 b1 := by simp [Step,deep,b0,b1,Update]
  have hv : Step reverse b1 b2 := by simp [Step,reverse,b1,b2,Update]
  have hq0 : QuoteOf b0 q := by decide
  have hq1 : QuoteOf b1 q := by decide
  have hq2 : QuoteOf b2 q := by decide
  have hf : Flow true b0 q b1 q [deep] :=
    Flow.step deep b0 q b1 q b1 q [] hd hq0 hq1 (by rfl) (Flow.nil _ _ hq1)
  have hc : Completed p 2 0 1 := by
    unfold Completed DirectQuote.Completed p DirectQuote.positive deep reverse
    decide
  refine ⟨hd,hv,hq0,hq1,hq2,by decide,by decide,by decide,hf,hc,by decide,
    by decide,by decide,by decide,by decide,?_⟩
  intro hs
  have hb := support_bounds hs
  change 100 ≤ 90 ∧ 90 ≤ 102 at hb
  omega

theorem exact_root : SPLIT_TARGET ∧ HULL_TARGET ∧ LIMIT_TARGET :=
  ⟨fun _ _ _ _ _ _ => split_flow,fun _ _ _ _ _ _ => hull,limit⟩

end DirectHull
