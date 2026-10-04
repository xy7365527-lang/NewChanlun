import MovingQuoteProof
import DCSemantics

namespace PressureDC
open MovingQuote

def select (pos : Bool) (q : Quote) : Int := if pos then q.ask else q.bid
def recover (previous : Bool) (a b : Int) : Bool :=
  if a < b then true else if b < a then false else previous

structure Input where
  event : Event
  afterBook : Book
  afterQuote : Quote

def Legal (book : Book) (q : Quote) : List Input → Prop
  | [] => QuoteOf book q
  | x::xs => Step x.event book x.afterBook ∧ QuoteOf book q ∧ QuoteOf x.afterBook x.afterQuote ∧
      Legal x.afterBook x.afterQuote xs

def samples (xs : List Input) : List Int := xs.map (fun x => select (positive x.event) x.afterQuote)
def eventWord (xs : List Input) : List Bool := xs.map (fun x => positive x.event)
def decodeDirections (previous : Bool) (a : Int) : List Int → List Bool
  | [] => []
  | b::bs => let d := recover previous a b; d :: decodeDirections d b bs

def observedWord (q : Quote) : List Input → List Bool
  | [] => []
  | x::xs => decodeDirections false (select (!(positive x.event)) q) (samples (x::xs))

-- On integer prices, delta=1 reaches threshold at every strict reverse step.
-- Equal prices keep the previous direction and belong to its latest endpoint.
def dcGroups : List Bool → List (Bool × Nat)
  | [] => []
  | d::ds => match dcGroups ds with
    | [] => [(d,1)]
    | (old,n)::rest => if d = old then (d,n+1)::rest else (d,1)::(old,n)::rest

structure View where
  positive : Bool
  start : Nat
  finish : Nat
  firstKnown : Nat
  knownAt : Option Nat
  deriving DecidableEq, Repr

def views (start : Nat) : List (Bool × Nat) → List View
  | [] => []
  | (d,n)::rest =>
    ⟨d,start,start+n,start+1,if rest.isEmpty then none else some (start+n+1)⟩ :: views (start+n) rest

def actualView (x : Located) : View :=
  ⟨positive x.block.first,x.start,x.finish,x.firstKnown,x.knownAt⟩
def dcViews (q : Quote) (xs : List Input) : List View := views 0 (dcGroups (observedWord q xs))
def actualViews (xs : List Input) : List View :=
  (locate 0 (group (xs.map (·.event)))).map actualView
def groupShape (b : Block) : Bool × Nat := (positive b.first,(events b).length)

def PROJECTION_TARGET : Prop :=
  (∀ e a b q r old, Step e a b → QuoteOf a q → QuoteOf b r →
    recover old (select old q) (select (positive e) r) = positive e) ∧
  (∀ e a b q r old, Step e a b → QuoteOf a q → QuoteOf b r →
    recover old (select (!(positive e)) q) (select (positive e) r) = positive e ∧
    1 ≤ (if positive e then select (positive e) r - select (!(positive e)) q
      else select (!(positive e)) q - select (positive e) r))

def TRACE_TARGET : Prop := ∀ book q xs, Legal book q xs →
  observedWord q xs = eventWord xs ∧ dcViews q xs = actualViews xs

def dcLeg (d : Bool) (q r : Quote) (start len : Nat) : NewChanlun.Origin.Segment :=
  ⟨if d then .up else .down,start,start+len,select (!d) q,select d r⟩
def GEOMETRY_TARGET : Prop := ∀ d q r start len,
  dcLeg d q r start len = MovingQuote.leg d q r start len

def tiePath : Nat → Int
  | 0 => 100 | 1 => 102 | 2 => 102 | _ => 100
def tieQuote : Quote := ⟨100,102⟩
def tieEvent (d : Bool) : Event := ⟨⟨d,.add,1⟩,if d then 100 else 102⟩
def tieInputs : List Input :=
  [⟨tieEvent true,⟨[100,100],[102]⟩,tieQuote⟩,
   ⟨tieEvent true,⟨[100,100,100],[102]⟩,tieQuote⟩,
   ⟨tieEvent false,⟨[100,100,100],[102,102]⟩,tieQuote⟩]
def TIE_TARGET : Prop :=
  Legal ⟨[100],[102]⟩ tieQuote tieInputs ∧
  observedWord tieQuote tieInputs = [true,true,false] ∧
  dcViews tieQuote tieInputs = [⟨true,0,2,1,some 3⟩,⟨false,2,3,3,none⟩] ∧
  DCSpec.Extreme tiePath .up 0 2 1 ∧ ¬ DCSpec.Extreme tiePath .up 0 2 2

end PressureDC
