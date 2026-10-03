import DCSemantics

namespace UpgradeResearch

structure Interval where
  lo : Int
  hi : Int
  deriving DecidableEq, Repr

def Covers (outer inner : Interval) : Prop := outer.lo ≤ inner.lo ∧ inner.hi ≤ outer.hi
def Valid (x : Interval) : Prop := x.lo < x.hi
def StrictTouch (a b : Interval) : Prop := max a.lo b.lo < min a.hi b.hi
def core3 (a b c : Interval) : Interval :=
  ⟨max (max a.lo b.lo) c.lo, min (min a.hi b.hi) c.hi⟩
def unitOf (p : Nat → Int) (i : Nat) : Interval := ⟨min (p i) (p (i+1)), max (p i) (p (i+1))⟩
def tripleAt (p : Nat → Int) (i : Nat) : Interval := core3 (unitOf p i) (unitOf p (i+1)) (unitOf p (i+2))
def WeakPremises (p : Nat → Int) : Prop :=
  Valid (tripleAt p 0) ∧ ∀ i : Fin 9,
    (if i.val % 2 = 0 then 2 ≤ p (i.val+1)-p i.val else 2 ≤ p i.val-p (i.val+1)) ∧
    StrictTouch (tripleAt p 0) (unitOf p i.val)

def AutomaticSuccess : Prop :=
  ∀ (p : Nat → Int) (r : DCSpec.Result), DCSpec.Whole p 2 11 r →
    (DCSpec.units r).length = 9 → WeakPremises p →
    ∀ k : Fin 3, Valid (tripleAt p (3*k.val))

def COMMON_CORE_TARGET : Prop := ∀ (a b c seed : Interval),
  Valid seed → Covers a seed → Covers b seed → Covers c seed →
  Covers (core3 a b c) seed ∧ Valid (core3 a b c)

def witnessPath : Nat → Int
  | 0 => 10025 | 1 => 10045 | 2 => 10025 | 3 => 10045
  | 4 => 10030 | 5 => 10050 | 6 => 10040 | 7 => 10043
  | 8 => 10035 | 9 => 10037 | _ => 10033

def witnessUnits : List DCSpec.Unit :=
  [⟨.up,0,1,2⟩,⟨.down,1,2,3⟩,⟨.up,2,3,4⟩,
   ⟨.down,3,4,5⟩,⟨.up,4,5,6⟩,⟨.down,5,6,7⟩,
   ⟨.up,6,7,8⟩,⟨.down,7,8,9⟩,⟨.up,8,9,10⟩]
def witnessTail : DCSpec.Tail := ⟨.down,9,10⟩
def witnessResult : DCSpec.Result := .started .up 0 1 (witnessUnits,witnessTail)
def quantity (i : Nat) : Int :=
  let x := witnessPath i - 10000
  (x*10000 + (200-x)/2)/(200-x)
def projected (qb : Int) : Int :=
  (2*(10200*qb+10000*10000)+(qb+10000))/(2*(qb+10000))

def COUNTEREXAMPLE_TARGET : Prop :=
  DCSpec.Whole witnessPath 2 11 witnessResult ∧
  (DCSpec.units witnessResult).length = 9 ∧ WeakPremises witnessPath ∧
  (tripleAt witnessPath 6).hi < (tripleAt witnessPath 6).lo ∧
  (∀ i : Fin 11, 0 < quantity i.val ∧ projected (quantity i.val) = witnessPath i.val) ∧
  ¬ AutomaticSuccess

end UpgradeResearch
