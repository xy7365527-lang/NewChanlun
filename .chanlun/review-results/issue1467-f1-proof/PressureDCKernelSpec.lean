import PressureDCSpec

namespace PressureDCKernel
open PressureDC MovingQuote

structure State where
  up : Bool
  extremePrice : Int
  extremeIndex : Nat
  deriving DecidableEq, Repr

structure Result where
  state : State
  finishedAt : Option Nat
  deriving DecidableEq, Repr

def initDC (a b : Int) : Option State :=
  if a < b then some ⟨true,b,1⟩ else if b < a then some ⟨false,b,1⟩ else none

-- Ordinary directional-change update at integer delta=1, with latest tie.
def dcUpdate (i : Nat) (x : Int) (s : State) : Result :=
  if s.up then
    if s.extremePrice ≤ x then ⟨⟨true,x,i⟩,none⟩
    else if 1 ≤ s.extremePrice-x then ⟨⟨false,x,i⟩,some s.extremeIndex⟩
    else ⟨s,none⟩
  else
    if x ≤ s.extremePrice then ⟨⟨false,x,i⟩,none⟩
    else if 1 ≤ x-s.extremePrice then ⟨⟨true,x,i⟩,some s.extremeIndex⟩
    else ⟨s,none⟩

def localUpdate (i : Nat) (x : Int) (s : State) : Result :=
  let d := recover s.up s.extremePrice x
  ⟨⟨d,x,i⟩,if d = s.up then none else some s.extremeIndex⟩

def KERNEL_TARGET : Prop :=
  (∀ s i x, dcUpdate i x s = localUpdate i x s ∧
    (dcUpdate i x s).state.extremePrice = x ∧ (dcUpdate i x s).state.extremeIndex = i) ∧
  (∀ e a b q r, Step e a b → QuoteOf a q → QuoteOf b r →
    initDC (select (!(positive e)) q) (select (positive e) r) =
      some ⟨positive e,select (positive e) r,1⟩)

end PressureDCKernel
