import DCFiniteCertificates
import MovingQuoteSpec
import Origin.ForceVelocity

namespace ClockEdge

open NewChanlun.Origin

def path : Nat → Int
  | 0 => 100 | 1 => 104 | 2 => 102 | 3 => 108 | _ => 106
def units : List DCSpec.Unit := [⟨.up,0,1,2⟩,⟨.down,1,2,3⟩,⟨.up,2,3,4⟩]
def tail : DCSpec.Tail := ⟨.down,3,4⟩
def result : DCSpec.Result := .started .up 0 1 (units,tail)

def clock (slowing : Bool) : Nat → Nat
  | 0 => 0
  | 1 => if slowing then 1 else 4
  | 2 => if slowing then 2 else 5
  | 3 => if slowing then 8 else 6
  | _ => if slowing then 9 else 7

/-- 只审计这份明确的数值适配，不宣称DC单元就是原文笔。 -/
def fromUnit (t : Nat → Nat) (u : DCSpec.Unit) : Stroke :=
  ⟨(match u.direction with | .up => .up | .down => .down),
   t u.start,t u.finish,path u.start,path u.finish⟩
def strokes (slowing : Bool) : List Stroke := units.map (fromUnit (clock slowing))
def geometry (s : Stroke) := (s.direction,s.startPrice,s.endPrice)
def reference : List Stroke := [⟨.up,10,11,100,101⟩]

def CLOCK_TARGET : Prop :=
  DCSpec.Whole path 2 5 result ∧
  (∀ b, ∀ i j : Fin 5, i.val < j.val → clock b i.val < clock b j.val) ∧
  (∀ b, ∀ s ∈ strokes b, s.WellFormed) ∧
  (strokes true).map geometry = (strokes false).map geometry ∧
  impulse (strokes true) = -3 ∧ impulse (strokes false) = 5 ∧
  IsImpulseDivergence reference (strokes true) ∧
  ¬ IsImpulseDivergence reference (strokes false)

def zeroStamp (a b : Int) (i : Nat) : Stroke := ⟨.up,i,i,a,b⟩
def ZERO_TARGET : Prop :=
  (∀ a b i, ¬ (zeroStamp a b i).WellFormed ∧ (zeroStamp a b i).velocity = 0) ∧
  (zeroStamp 100 102 340).startPrice < (zeroStamp 100 102 340).endPrice

def spread (q : MovingQuote.Quote) : Int := q.ask-q.bid
def twiceMid (q : MovingQuote.Quote) : Int := q.bid+q.ask
def sign (pos : Bool) : Int := if pos then 1 else -1
def EDGE_TARGET : Prop :=
  (∀ pos q r start len,
    let s := MovingQuote.leg pos q r start len
    2*(s.endPrice-s.startPrice) =
      twiceMid r-twiceMid q + sign pos*(spread q+spread r)) ∧
  (∀ q start len, q.bid < q.ask → 0 < len →
    (MovingQuote.leg true q q start len).startIndex < (MovingQuote.leg true q q start len).endIndex ∧
    (MovingQuote.leg true q q start len).startPrice < (MovingQuote.leg true q q start len).endPrice ∧
    (MovingQuote.leg true q q start len).endPrice -
      (MovingQuote.leg true q q start len).startPrice = spread q)

end ClockEdge
