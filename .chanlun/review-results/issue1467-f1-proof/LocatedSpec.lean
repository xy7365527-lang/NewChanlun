import MovingQuoteProof

namespace LocatedProof

open MovingQuote (Event Block Located positive events decode Uniform Separated group locate Completed)

def input (p : Nat → Event) (n : Nat) : List Event := (List.range n).map p
def output (p : Nat → Event) (n : Nat) : List Located := locate 0 (group (input p n))

def Span (p : Nat → Event) (n s e : Nat) : Prop :=
  s < e ∧ e ≤ n ∧ (s = 0 ∨ positive (p (s-1)) ≠ positive (p s)) ∧
  (∀ i, s ≤ i → i < e → positive (p i) = positive (p s)) ∧
  (e = n ∨ positive (p (e-1)) ≠ positive (p e))

def Good (p : Nat → Event) (n : Nat) (l : Located) : Prop :=
  Span p n l.start l.finish ∧ l.firstKnown = l.start+1 ∧
  l.finish = l.start+(events l.block).length ∧
  (l.knownAt = none ↔ l.finish = n) ∧
  (∀ k, l.knownAt = some k → k = l.finish+1)

def PROGRAM_TARGET : Prop :=
  (∀ p n l, l ∈ output p n → Good p n l) ∧
  (∀ p n i, i < n → ∃ l ∈ output p n, l.start ≤ i ∧ i < l.finish)

def MATCH_TARGET : Prop := ∀ p n s e,
  Completed p n s e ↔ ∃ l ∈ output p n,
    l.start = s ∧ l.finish = e ∧ l.knownAt = some (e+1)

def Seen (p : Nat → Event) (n t s e : Nat) : Prop :=
  ∃ l ∈ output p n, l.start = s ∧ l.finish = e ∧
    l.knownAt = some (e+1) ∧ e+1 ≤ t

def CAUSAL_TARGET : Prop := ∀ (p q : Nat → Event) n m t,
  t ≤ n → t ≤ m → (∀ i, i < t → p i = q i) → ∀ s e,
    Seen p n t s e ↔ Seen q m t s e

def demo (i : Nat) : Event := MovingQuote.es[i]?.getD MovingQuote.e5
def other (i : Nat) : Event := MovingQuote.es[i]?.getD MovingQuote.e1
def confirmed (p : Nat → Event) (n t : Nat) : List Located :=
  (output p n).filter (fun l => match l.knownAt with
    | none => false | some k => decide (k ≤ t))

def WITNESS_TARGET : Prop :=
  (output demo 6).map Located.knownAt = [some 2,some 3,some 4,some 6,none] ∧
  ¬ Completed demo 1 0 1 ∧ Completed demo 2 0 1 ∧
  ¬ Completed demo 6 5 6 ∧ Completed other 7 5 6 ∧
  confirmed demo 8 6 = confirmed other 8 6

end LocatedProof
