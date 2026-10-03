import DirectQuoteProof

namespace MovingQuote

open NewChanlun.Origin

structure Event where
  core : DirectQuote.Event
  price : Int
  deriving DecidableEq, Repr
def positive (e : Event) : Bool := DirectQuote.positive e.core

structure Block where
  first : Event
  rest : List Event
  deriving DecidableEq, Repr
def events (b : Block) := b.first :: b.rest
def decode (bs : List Block) := bs.flatMap events
def prepend (e : Event) (b : Block) : Block := ⟨e,b.first::b.rest⟩
inductive Runs : List Event → List Block → Prop where
  | nil : Runs [] []
  | one (e : Event) : Runs [e] [⟨e,[]⟩]
  | same (e : Event) {es b bs} : Runs es (b::bs) → positive e = positive b.first →
      Runs (e::es) (prepend e b::bs)
  | different (e : Event) {es b bs} : Runs es (b::bs) → positive e ≠ positive b.first →
      Runs (e::es) (⟨e,[]⟩::b::bs)
def group : List Event → List Block
  | [] => []
  | e::es => match group es with
      | [] => [⟨e,[]⟩]
      | b::bs => if positive e = positive b.first then prepend e b::bs else ⟨e,[]⟩::b::bs
def Uniform (b : Block) : Prop := ∀ e ∈ events b, positive e = positive b.first
def Separated : List Block → Prop
  | [] => True | [_] => True
  | a::b::bs => positive a.first ≠ positive b.first ∧ Separated (b::bs)
def RUN_TARGET : Prop :=
  (∀ es, ∃ bs, Runs es bs ∧ ∀ other, Runs es other → other = bs) ∧
  (∀ es bs, Runs es bs → decode bs = es ∧ (∀ b ∈ bs, Uniform b) ∧ Separated bs)

structure Book where
  bids : List Int
  asks : List Int
  deriving DecidableEq, Repr
structure Quote where
  bid : Int
  ask : Int
  deriving DecidableEq, Repr
def BestBid (xs : List Int) (p : Int) : Prop := p ∈ xs ∧ ∀ x ∈ xs, x ≤ p
def BestAsk (xs : List Int) (p : Int) : Prop := p ∈ xs ∧ ∀ x ∈ xs, p ≤ x
def QuoteOf (b : Book) (q : Quote) : Prop := BestBid b.bids q.bid ∧ BestAsk b.asks q.ask ∧ q.bid < q.ask
def ValidBook (b : Book) : Prop := ∀ p ∈ b.bids, ∀ a ∈ b.asks, p < a
def Update (action : DirectQuote.Action) (price : Int) (amount : Nat)
    (before after : List Int) : Prop :=
  if action = .add then after = List.replicate amount price ++ before
  else ∃ pre post, before = pre ++ List.replicate amount price ++ post ∧ after = pre ++ post
def Step (e : Event) (a b : Book) : Prop := 0 < e.core.amount ∧
  if e.core.bidSide then Update e.core.action e.price e.core.amount a.bids b.bids ∧ b.asks = a.asks
  else Update e.core.action e.price e.core.amount a.asks b.asks ∧ b.bids = a.bids
def Monotone (pos : Bool) (a b : Quote) : Prop :=
  if pos then a.bid ≤ b.bid ∧ a.ask ≤ b.ask else b.bid ≤ a.bid ∧ b.ask ≤ a.ask
def STEP_TARGET : Prop :=
  (∀ b q r, QuoteOf b q → QuoteOf b r → q = r) ∧
  (∀ e a b q r, Step e a b → QuoteOf a q → QuoteOf b r → Monotone (positive e) q r)

inductive Flow (pos : Bool) : Book → Quote → Book → Quote → List Event → Prop where
  | nil (b q) : QuoteOf b q → Flow pos b q b q []
  | step (e a q b r c s es) :
      Step e a b → QuoteOf a q → QuoteOf b r → positive e = pos →
      Flow pos b r c s es → Flow pos a q c s (e::es)

def leg (pos : Bool) (q r : Quote) (start len : Nat) : Segment :=
  if pos then ⟨.up,start,start+len,q.bid,r.ask⟩ else ⟨.down,start,start+len,q.ask,r.bid⟩
def quoteReady : Option Quote → Bool
  | none => false
  | some q => decide (q.bid < q.ask)
def readout (pos : Bool) (start : Nat) (qs : List (Option Quote)) : Option Segment :=
  if qs.length < 2 then none else
  if !qs.all quoteReady then none else
  match qs.head?,qs.getLast? with
  | some (some q),some (some r) =>
      if (if pos then q.bid < r.ask else r.bid < q.ask)
      then some (leg pos q r start (qs.length-1)) else none
  | _,_ => none

def LEG_TARGET : Prop :=
  (∀ pos a q b r es, Flow pos a q b r es → es ≠ [] → ∀ start,
    (leg pos q r start es.length).startIndex < (leg pos q r start es.length).endIndex ∧
    (if pos then (leg pos q r start es.length).startPrice < (leg pos q r start es.length).endPrice
     else (leg pos q r start es.length).endPrice < (leg pos q r start es.length).startPrice)) ∧
  (∀ pos q r s start n m,
    (leg pos q r start n).endPrice = (leg (!pos) r s (start+n) m).startPrice) ∧
  (∀ q start b, leg (DirectQuote.positive b.first) q q start (DirectQuote.events b).length =
    DirectQuote.leg q.bid q.ask start b)

structure Located where
  block : Block
  start : Nat
  finish : Nat
  firstKnown : Nat
  knownAt : Option Nat
  deriving DecidableEq, Repr
def locate (start : Nat) : List Block → List Located
  | [] => []
  | b::bs =>
      let finish := start+(events b).length
      ⟨b,start,finish,start+1,if bs.isEmpty then none else some (finish+1)⟩ :: locate finish bs
def Completed (p : Nat → Event) (n s e : Nat) := DirectQuote.Completed (fun i => (p i).core) n s e
def CAUSAL_TARGET : Prop := ∀ (p q : Nat → Event) n s e,
  (∀ i, i < n → p i = q i) → (Completed p n s e ↔ Completed q n s e)

def e0 : Event := ⟨⟨false,.cancel,1⟩,102⟩
def e1 : Event := ⟨⟨false,.add,1⟩,103⟩
def e2 : Event := ⟨⟨true,.add,1⟩,101⟩
def e3 : Event := ⟨⟨true,.cancel,1⟩,101⟩
def e4 : Event := ⟨⟨true,.execute,1⟩,100⟩
def e5 : Event := ⟨⟨true,.add,1⟩,100⟩
def es : List Event := [e0,e1,e2,e3,e4,e5]
def book : Nat → Book
  | 0 => ⟨[100],[102,104]⟩ | 1 => ⟨[100],[104]⟩
  | 2 => ⟨[100],[103,104]⟩ | 3 => ⟨[101,100],[103,104]⟩
  | 4 => ⟨[100],[103,104]⟩ | 5 => ⟨[],[103,104]⟩
  | _ => ⟨[100],[103,104]⟩
def q0 : Quote := ⟨100,102⟩
def q1 : Quote := ⟨100,104⟩
def q2 : Quote := ⟨100,103⟩
def q3 : Quote := ⟨101,103⟩
def q4 : Quote := ⟨100,103⟩
def q6 : Quote := ⟨100,103⟩

def WITNESS_TARGET : Prop :=
  Step e0 (book 0) (book 1) ∧ Step e1 (book 1) (book 2) ∧
  Step e2 (book 2) (book 3) ∧ Step e3 (book 3) (book 4) ∧
  Step e4 (book 4) (book 5) ∧ Step e5 (book 5) (book 6) ∧
  (∀ i : Fin 7, ValidBook (book i.val)) ∧
  QuoteOf (book 0) q0 ∧ QuoteOf (book 1) q1 ∧ QuoteOf (book 2) q2 ∧
  QuoteOf (book 3) q3 ∧ QuoteOf (book 4) q4 ∧ QuoteOf (book 6) q6 ∧
  (¬ ∃ q, QuoteOf (book 5) q) ∧
  decode (group es) = es ∧ (group es).map (fun b => (events b).length) = [1,1,1,2,1] ∧
  (locate 0 (group es)).map (·.firstKnown) = [1,2,3,4,6] ∧
  (locate 0 (group es)).map (·.knownAt) = [some 2,some 3,some 4,some 6,none] ∧
  readout false 3 [some q3,some q4,none] = none ∧
  readout true 5 [none,some q6] = none ∧
  readout true 0 [some q0,some q1] = some (leg true q0 q1 0 1) ∧
  (leg true q0 q1 0 1).endPrice = (leg false q1 q2 1 1).startPrice ∧
  (leg true q0 q1 0 1).endPrice ≠ (leg false q2 q2 1 1).startPrice ∧
  CenterConfirmedComplete (leg true q0 q1 0 1) (leg false q1 q2 1 1) (leg true q2 q3 2 1) ∧
  computeZD (leg true q0 q1 0 1) (leg false q1 q2 1 1) (leg true q2 q3 2 1) = 100 ∧
  computeZG (leg true q0 q1 0 1) (leg false q1 q2 1 1) (leg true q2 q3 2 1) = 103

end MovingQuote
