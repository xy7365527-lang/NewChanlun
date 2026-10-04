import MovingQuoteProof

namespace DirectHull
open MovingQuote NewChanlun.Origin

/-- An L1 spread price at a source split. Original Flow, book states and events
are retained; no output segment or candidate hull occurs in this relation. -/
def Support (pos : Bool) (a : Book) (q : Quote) (b : Book) (r : Quote)
    (es : List Event) (x : Int) : Prop :=
  ∃ pre post mid v, es = pre ++ post ∧
    Flow pos a q mid v pre ∧ Flow pos mid v b r post ∧ v.bid ≤ x ∧ x ≤ v.ask

def ExactHull (S : Int → Prop) (lo hi : Int) : Prop :=
  S lo ∧ S hi ∧ ∀ x, S x → lo ≤ x ∧ x ≤ hi

def SPLIT_TARGET : Prop := ∀ pos a q b r es, Flow pos a q b r es →
  ∀ pre post, es = pre ++ post → ∃ mid v, Flow pos a q mid v pre ∧ Flow pos mid v b r post

def HULL_TARGET : Prop := ∀ pos a q b r es, Flow pos a q b r es → ∀ start,
  ExactHull (Support pos a q b r es)
    (segLow (leg pos q r start es.length)) (segHigh (leg pos q r start es.length))

def deep : Event := ⟨⟨true,.add,1⟩,90⟩
def reverse : Event := ⟨⟨false,.add,1⟩,110⟩
def b0 : Book := ⟨[100],[102]⟩
def b1 : Book := ⟨[90,100],[102]⟩
def b2 : Book := ⟨[90,100],[110,102]⟩
def q : Quote := ⟨100,102⟩
def p (i : Nat) : Event := if i = 0 then deep else reverse

/-- A confirmed, nonempty direct block has exact L1 hull [100,102], yet a
real source event and real post-state bid lie at 90 outside that hull. -/
def LIMIT_TARGET : Prop :=
  Step deep b0 b1 ∧ Step reverse b1 b2 ∧
  QuoteOf b0 q ∧ QuoteOf b1 q ∧ QuoteOf b2 q ∧
  ValidBook b0 ∧ ValidBook b1 ∧ ValidBook b2 ∧
  Flow true b0 q b1 q [deep] ∧ Completed p 2 0 1 ∧
  (locate 0 (group [deep,reverse])).map Located.knownAt = [some 2,none] ∧
  segLow (leg true q q 0 1) = 100 ∧ segHigh (leg true q q 0 1) = 102 ∧
  deep.price ∈ b1.bids ∧ deep.price < segLow (leg true q q 0 1) ∧
  ¬ Support true b0 q b1 q [deep] deep.price

end DirectHull
