import Origin.CenterComplete

namespace DirectQuote

open NewChanlun.Origin

inductive Action where
  | add | cancel | execute
  deriving DecidableEq, Repr

structure Event where
  bidSide : Bool
  action : Action
  amount : Nat
  deriving DecidableEq, Repr

def positive (e : Event) : Bool :=
  if e.bidSide then e.action == .add else !(e.action == .add)

structure Queues where
  bid : Int
  ask : Int
  deriving DecidableEq, Repr

def delta (e : Event) : Int := if e.action = .add then e.amount else -(e.amount : Int)
def advance (q : Queues) (e : Event) : Queues :=
  if e.bidSide then ⟨q.bid + delta e,q.ask⟩ else ⟨q.bid,q.ask + delta e⟩
def PositiveQueues (q : Queues) : Prop := 0 < q.bid ∧ 0 < q.ask
def ValidHistory (q : Queues) : List Event → Prop
  | [] => PositiveQueues q
  | e::es => 0 < e.amount ∧ PositiveQueues q ∧ ValidHistory (advance q e) es

structure Block where
  first : Event
  rest : List Event
  deriving DecidableEq, Repr

def events (b : Block) : List Event := b.first :: b.rest
def decode (bs : List Block) : List Event := bs.flatMap events
def prepend (e : Event) (b : Block) : Block := ⟨e,b.first::b.rest⟩

inductive Runs : List Event → List Block → Prop where
  | nil : Runs [] []
  | one (e : Event) : Runs [e] [⟨e,[]⟩]
  | same (e : Event) {es b bs} : Runs es (b::bs) → positive e = positive b.first →
      Runs (e::es) (prepend e b :: bs)
  | different (e : Event) {es b bs} : Runs es (b::bs) → positive e ≠ positive b.first →
      Runs (e::es) (⟨e,[]⟩::b::bs)

def group : List Event → List Block
  | [] => []
  | e::es => match group es with
    | [] => [⟨e,[]⟩]
    | b::bs => if positive e = positive b.first then prepend e b::bs else ⟨e,[]⟩::b::bs

def Uniform (b : Block) : Prop := ∀ e ∈ events b, positive e = positive b.first
def Separated : List Block → Prop
  | [] => True
  | [_] => True
  | a::b::bs => positive a.first ≠ positive b.first ∧ Separated (b::bs)

def RUN_TARGET : Prop :=
  (∀ es, ∃ bs, Runs es bs ∧ ∀ other, Runs es other → other = bs) ∧
  (∀ es bs, Runs es bs → decode bs = es ∧ (∀ b ∈ bs, Uniform b) ∧ Separated bs)

/-- Event positions are zero-based; state/confirmation times count observed events. -/
def Completed (p : Nat → Event) (n s e : Nat) : Prop :=
  s < e ∧ e < n ∧
  (s = 0 ∨ positive (p (s-1)) ≠ positive (p s)) ∧
  (∀ i : Fin e, s ≤ i.val → positive (p i.val) = positive (p s)) ∧
  positive (p (e-1)) ≠ positive (p e)

def CAUSAL_TARGET : Prop := ∀ (p q : Nat → Event) n s e,
  (∀ i, i < n → p i = q i) → (Completed p n s e ↔ Completed q n s e)

structure Located where
  block : Block
  start : Nat
  finish : Nat
  knownAt : Option Nat
  deriving DecidableEq, Repr

def locate (start : Nat) : List Block → List Located
  | [] => []
  | b::bs =>
      let finish := start + (events b).length
      ⟨b,start,finish,if bs.isEmpty then none else some (finish+1)⟩ :: locate finish bs

/-- This is a declared quote-side readout, never a traded-price or touched-price path. -/
def leg (bid ask : Int) (start : Nat) (b : Block) : Segment :=
  if positive b.first then
    ⟨.up,start,start+(events b).length,bid,ask⟩
  else ⟨.down,start,start+(events b).length,ask,bid⟩

def GEOMETRY_TARGET : Prop :=
  (∀ bid ask, bid < ask → ∀ start b,
    (leg bid ask start b).startIndex < (leg bid ask start b).endIndex ∧
    segLow (leg bid ask start b) = bid ∧ segHigh (leg bid ask start b) = ask) ∧
  (∀ bid ask start a b, positive a.first ≠ positive b.first →
    (leg bid ask start a).endPrice = (leg bid ask (start+(events a).length) b).startPrice) ∧
  (∀ bid ask, bid < ask → ∀ start a b c,
    positive a.first ≠ positive b.first → positive b.first ≠ positive c.first →
    let sa := leg bid ask start a
    let sb := leg bid ask (start+(events a).length) b
    let sc := leg bid ask (start+(events a).length+(events b).length) c
    CenterConfirmedComplete sa sb sc ∧ computeZD sa sb sc = bid ∧ computeZG sa sb sc = ask)

def flow (qty : Nat) (i : Nat) : Event :=
  match i % 4 with
  | 0 => ⟨false,.cancel,qty⟩
  | 1 => ⟨true,.add,qty⟩
  | 2 => ⟨true,.cancel,qty⟩
  | _ => ⟨false,.add,qty⟩
def history (qty : Nat) : List Event := (List.range 20).map (flow qty)
def touch (bid ask : Int) (e : Event) : Int := if e.bidSide then bid else ask
def firstBlock (qty : Nat) : Block := ⟨flow qty 0,[flow qty 1]⟩
def secondBlock (qty : Nat) : Block := ⟨flow qty 2,[flow qty 3]⟩
def thirdBlock (qty : Nat) : Block := ⟨flow qty 4,[flow qty 5]⟩

def WITNESS_TARGET : Prop :=
  ValidHistory ⟨100,100⟩ (history 10) ∧ ValidHistory ⟨100,100⟩ (history 5) ∧
  history 10 ≠ history 5 ∧
  (group (history 10)).length = 10 ∧ (group (history 5)).length = 10 ∧
  (locate 0 (group (history 10))).map (·.knownAt) =
    [some 3,some 5,some 7,some 9,some 11,some 13,some 15,some 17,some 19,none] ∧
  Completed (flow 10) 20 0 2 ∧ ¬ Completed (flow 10) 2 0 2 ∧
  (events (firstBlock 10)).map (touch 100 102) = [102,100] ∧
  (leg 100 102 0 (firstBlock 10)).startPrice = 100 ∧
  (leg 100 102 0 (firstBlock 10)).endPrice = 102 ∧
  (∀ qty ∈ [5,10],
    CenterConfirmedComplete (leg 100 102 0 (firstBlock qty)) (leg 100 102 2 (secondBlock qty)) (leg 100 102 4 (thirdBlock qty)) ∧
    computeZD (leg 100 102 0 (firstBlock qty)) (leg 100 102 2 (secondBlock qty)) (leg 100 102 4 (thirdBlock qty)) = 100 ∧
    computeZG (leg 100 102 0 (firstBlock qty)) (leg 100 102 2 (secondBlock qty)) (leg 100 102 4 (thirdBlock qty)) = 102)

end DirectQuote
