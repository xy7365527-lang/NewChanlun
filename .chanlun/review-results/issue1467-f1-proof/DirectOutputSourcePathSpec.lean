import DirectOutputSpec

namespace DirectOutput
open MovingQuote NewChanlun.Origin

/-- The L1 spread support of the supplied history itself, at inclusive source cuts. -/
def ActualSupport (bs : Nat → Book) (qs : List (Option Quote)) (start finish : Nat)
    (z : Int) : Prop :=
  ∃ k q, start ≤ k ∧ k ≤ finish ∧ qs[k]? = some (some q) ∧
    QuoteOf (bs k) q ∧ q.bid ≤ z ∧ z ≤ q.ask

def ACTUAL_PATH_TARGET : Prop := ∀ es bs qs out,
  BookQuote.Through es bs es.length → BookQuote.Reads bs qs →
  qs.length = es.length+1 → construct es qs = some out →
  ∀ x ∈ out, ∀ s, x.geometry = some s →
    (∀ k, x.located.start ≤ k → k ≤ x.located.finish →
      ∃ q, qs[k]? = some (some q) ∧ QuoteOf (bs k) q) ∧
    DirectHull.ExactHull (ActualSupport bs qs x.located.start x.located.finish)
      (segLow s) (segHigh s)

end DirectOutput
