import DirectOutputSourcePathProof

namespace IndependentChecks
open MovingQuote NewChanlun.Origin

theorem locate_nodup (blocks : List Block) (start : Nat) : (locate start blocks).Nodup := by
  induction blocks generalizing start with
  | nil => simp [locate]
  | cons b bs ih =>
    change (LocatedProof.placed start b bs :: locate (start+(events b).length) bs).Nodup
    apply List.nodup_cons.mpr
    constructor
    · intro hm
      have hb := DirectOutput.locate_lower bs _ _ hm
      change start+(events b).length ≤ start at hb
      have hp : 0 < (events b).length := by simp [events]
      omega
    · exact ih _

theorem output_nodup {es qs out} (h : construct es qs = some out) : out.Nodup := by
  rw [DirectOutput.output_shape h]
  have preserve : ∀ ls : List Located, ls.Nodup → (ls.map (DirectOutput.render qs)).Nodup := by
    intro ls
    induction ls with
    | nil => simp
    | cons a ls ih =>
      intro hn
      simp only [List.nodup_cons] at hn
      simp only [List.map_cons,List.nodup_cons]
      refine ⟨?_,ih hn.2⟩
      intro hm
      rcases List.mem_map.mp hm with ⟨b,hb,he⟩
      have hba : b = a := congrArg Output.located he
      exact hn.1 (hba ▸ hb)
  exact preserve _ (locate_nodup _ _)

def demo_es := [DirectHull.deep,DirectHull.reverse]
def demo_bs : Nat → Book
  | 0 => DirectHull.b0
  | 1 => DirectHull.b1
  | _ => DirectHull.b2
def demo_qs : List (Option Quote) := [some DirectHull.q,some DirectHull.q,some DirectHull.q]
def demo_out : List Output := (locate 0 (group demo_es)).map (DirectOutput.render demo_qs)

theorem demo_through : BookQuote.Through demo_es demo_bs demo_es.length := by
  intro i hi
  have hc : i = 0 ∨ i = 1 := by change i < 2 at hi; omega
  rcases hc with rfl | rfl
  · exact ⟨DirectHull.deep,rfl,DirectHull.limit.1⟩
  · exact ⟨DirectHull.reverse,rfl,DirectHull.limit.2.1⟩

theorem demo_reads : BookQuote.Reads demo_bs demo_qs := by
  intro i oq h
  have hb : i < demo_qs.length := (List.getElem?_eq_some_iff.mp h).1
  have hc : i = 0 ∨ i = 1 ∨ i = 2 := by change i < 3 at hb; omega
  rcases hc with rfl | rfl | rfl
  all_goals
    have ho : oq = some DirectHull.q := Option.some.inj h.symm
    subst oq
    change QuoteOf _ DirectHull.q
    decide

theorem demo_construct : construct demo_es demo_qs = some demo_out := rfl

theorem demo_nonvacuous :
    BookQuote.Through demo_es demo_bs demo_es.length ∧
    BookQuote.Reads demo_bs demo_qs ∧ demo_qs.length = demo_es.length+1 ∧
    construct demo_es demo_qs = some demo_out ∧ demo_out.length = 2 ∧
    demo_out.map (fun x => x.geometry.isSome) = [true,true] ∧
    demo_out.map (fun x => x.located.knownAt) = [some 2,none] ∧
    Completed (DirectOutput.eventAt demo_es) 2 0 1 ∧
    ¬ Completed (DirectOutput.eventAt demo_es) 2 1 2 := by
  refine ⟨demo_through,demo_reads,rfl,demo_construct,rfl,rfl,rfl,?_,?_⟩
  · unfold Completed DirectQuote.Completed DirectOutput.eventAt demo_es DirectHull.deep DirectHull.reverse DirectQuote.positive
    decide
  · unfold Completed DirectQuote.Completed
    omega

theorem none_retained :
    BookQuote.Through BookQuote.events BookQuote.bookA BookQuote.events.length ∧
    BookQuote.Reads BookQuote.bookA BookQuote.quotes ∧
    ∃ out, construct BookQuote.events BookQuote.quotes = some out ∧
      out.length = 1 ∧ out.map (fun x => x.geometry.isSome) = [false] ∧
      decode (out.map (fun x => x.located.block)) = BookQuote.events := by
  let out := (locate 0 (group BookQuote.events)).map (DirectOutput.render BookQuote.quotes)
  exact ⟨BookQuote.throughA,BookQuote.readsA,out,rfl,rfl,rfl,rfl⟩

theorem exact_checks :
    (∀ es qs out, construct es qs = some out → out.Nodup) ∧
    BookQuote.Through demo_es demo_bs demo_es.length ∧
    BookQuote.Reads demo_bs demo_qs ∧
    (∀ x ∈ demo_out, ∀ s, x.geometry = some s →
      (∀ k, x.located.start ≤ k → k ≤ x.located.finish →
        ∃ q, demo_qs[k]? = some (some q) ∧ QuoteOf (demo_bs k) q) ∧
      DirectHull.ExactHull (DirectOutput.ActualSupport demo_bs demo_qs x.located.start x.located.finish)
        (segLow s) (segHigh s)) := by
  exact ⟨fun _ _ _ => output_nodup,demo_through,demo_reads,
    DirectOutput.actual_exact_root demo_es demo_bs demo_qs demo_out
      demo_through demo_reads rfl demo_construct⟩
end IndependentChecks

#print axioms IndependentChecks.output_nodup
#print axioms IndependentChecks.demo_nonvacuous
#print axioms IndependentChecks.none_retained
#print axioms IndependentChecks.exact_checks
set_option pp.all true in
#print NewChanlun.Origin.Segment
set_option pp.all true in
#print NewChanlun.Origin.Tick
set_option pp.all true in
#print NewChanlun.Origin.Index
set_option pp.all true in
#print NewChanlun.Origin.segLow
set_option pp.all true in
#print NewChanlun.Origin.segHigh
set_option pp.all true in
#print DirectOutput.ACTUAL_PATH_TARGET
