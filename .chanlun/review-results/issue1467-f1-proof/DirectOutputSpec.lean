import BookQuoteProof
import LocatedProof
import DirectHullProof

namespace DirectOutput
open MovingQuote NewChanlun.Origin

/-- The finite source, with an irrelevant total extension after its last event. -/
def eventAt (es : List Event) (i : Nat) : Event := es[i]?.getD e0

/-- Original event ownership, including outputs whose geometry is absent. -/
def Source (es : List Event) (l : Located) : Prop :=
  ∃ pre post, es = pre ++ events l.block ++ post ∧ pre.length = l.start ∧
    l.finish = l.start + (events l.block).length ∧ Uniform l.block

def SOURCE_TARGET : Prop := ∀ es qs out, construct es qs = some out →
  out.map Output.located = locate 0 (group es) ∧
  decode (out.map (fun x => x.located.block)) = es ∧
  (∀ x ∈ out, Source es x.located ∧
    LocatedProof.Good (eventAt es) es.length x.located ∧
    (Completed (eventAt es) es.length x.located.start x.located.finish ↔
      x.located.knownAt = some (x.located.finish+1)))

/-- Every source index belongs to precisely one original output; None is retained. -/
def COVER_TARGET : Prop := ∀ es qs out, construct es qs = some out →
  ∀ i, i < es.length → ∃ x, x ∈ out ∧ x.located.start ≤ i ∧ i < x.located.finish ∧
    ∀ y, y ∈ out → y.located.start ≤ i → i < y.located.finish → y = x

/-- Stronger than the confirmed-only goal: every actual some output is bound to
the original books, quotes and owned events, including the current open block. -/
def HULL_TARGET : Prop := ∀ es bs qs out,
  BookQuote.Through es bs es.length → BookQuote.Reads bs qs →
  qs.length = es.length+1 → construct es qs = some out →
  ∀ x ∈ out, ∀ s, x.geometry = some s →
    ∃ q r, qs[x.located.start]? = some (some q) ∧
      qs[x.located.finish]? = some (some r) ∧
      QuoteOf (bs x.located.start) q ∧ QuoteOf (bs x.located.finish) r ∧
      Flow (positive x.located.block.first) (bs x.located.start) q
        (bs x.located.finish) r (events x.located.block) ∧
      s = leg (positive x.located.block.first) q r x.located.start
        (events x.located.block).length ∧
      DirectHull.ExactHull
        (DirectHull.Support (positive x.located.block.first)
          (bs x.located.start) q (bs x.located.finish) r (events x.located.block))
        (segLow s) (segHigh s)

/-- Adjacent original outputs share one source cut and one source quote lookup.
When both geometries exist, their endpoint prices coincide. No filtering occurs. -/
def ADJACENT_TARGET : Prop := ∀ es bs qs out,
  BookQuote.Through es bs es.length → BookQuote.Reads bs qs →
  qs.length = es.length+1 → construct es qs = some out →
  ∀ pre x y post, out = pre ++ x::y::post →
    x.located.finish = y.located.start ∧
    positive y.located.block.first = !(positive x.located.block.first) ∧
    qs[x.located.finish]? = qs[y.located.start]? ∧
    ∀ s t, x.geometry = some s → y.geometry = some t → s.endPrice = t.startPrice

end DirectOutput
