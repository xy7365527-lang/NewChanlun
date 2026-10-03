import MovingQuoteProof

namespace BookQuote

open MovingQuote (Book Quote Event Step Update QuoteOf BestBid BestAsk)

def SameInventory (xs ys : List Int) : Prop := ∀ p, xs.count p = ys.count p
def SameBook (a b : Book) : Prop := SameInventory a.bids b.bids ∧ SameInventory a.asks b.asks
def Read (b : Book) : Option Quote → Prop
  | some q => QuoteOf b q
  | none => ¬ ∃ q, QuoteOf b q

def Through (es : List Event) (bs : Nat → Book) (n : Nat) : Prop :=
  ∀ i, i < n → ∃ e, es[i]? = some e ∧ Step e (bs i) (bs (i+1))
def Reads (bs : Nat → Book) (qs : List (Option Quote)) : Prop :=
  ∀ i q, qs[i]? = some q → Read (bs i) q

def COUNT_TARGET : Prop :=
  (∀ act price amount before after, Update act price amount before after → ∀ p,
    if act = .add then after.count p = before.count p + (List.replicate amount price).count p
    else before.count p = after.count p + (List.replicate amount price).count p) ∧
  (∀ e a b aa bb, SameBook a b → Step e a aa → Step e b bb → SameBook aa bb) ∧
  (∀ a b q r, SameBook a b → Read a q → Read b r → q = r)

def PREFIX_TARGET : Prop := ∀ es fs n bs cs,
  es.take n = fs.take n → SameBook (bs 0) (cs 0) →
  Through es bs n → Through fs cs n → ∀ i, i ≤ n →
    SameBook (bs i) (cs i) ∧ ∀ q r, Read (bs i) q → Read (cs i) r → q = r

def WIRE_TARGET : Prop := ∀ es bs cs qs rs,
  SameBook (bs 0) (cs 0) → Through es bs es.length → Through es cs es.length →
  qs.length = es.length+1 → rs.length = es.length+1 → Reads bs qs → Reads cs rs →
    qs = rs ∧ MovingQuote.construct es qs = MovingQuote.construct es rs

def cancel100 : Event := ⟨⟨true,.cancel,1⟩,100⟩
def cancel101 : Event := ⟨⟨true,.cancel,1⟩,101⟩
def execute100 : Event := ⟨⟨true,.execute,1⟩,100⟩
def events : List Event := [cancel100,cancel101,execute100]
def bookA : Nat → Book
  | 0 => ⟨[100,101,100],[103]⟩
  | 1 => ⟨[101,100],[103]⟩
  | 2 => ⟨[100],[103]⟩
  | _ => ⟨[],[103]⟩
def bookB : Nat → Book
  | 0 => ⟨[100,100,101],[103]⟩
  | 1 => ⟨[100,101],[103]⟩
  | 2 => ⟨[100],[103]⟩
  | _ => ⟨[],[103]⟩
def quotes : List (Option Quote) := [some ⟨101,103⟩,some ⟨101,103⟩,some ⟨100,103⟩,none]

def WITNESS_TARGET : Prop :=
  bookA 0 ≠ bookB 0 ∧ SameBook (bookA 0) (bookB 0) ∧
  Through events bookA 3 ∧ Through events bookB 3 ∧
  bookA 1 ≠ bookB 1 ∧ SameBook (bookA 1) (bookB 1) ∧
  Reads bookA quotes ∧ Reads bookB quotes ∧
  (∀ i : Fin 4, MovingQuote.ValidBook (bookA i.val) ∧ MovingQuote.ValidBook (bookB i.val)) ∧
  Read (bookA 3) none ∧ Read (bookB 3) none ∧
  (∃ out, MovingQuote.construct events quotes = some out ∧
    out.map (fun p => p.geometry.isSome) = [false] ∧
    MovingQuote.decode (out.map (fun p => p.located.block)) = events)

end BookQuote
