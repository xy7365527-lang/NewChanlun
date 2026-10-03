import Origin.ForceVelocity

namespace ForceTransport

open NewChanlun.Origin

/-- 保留真实首末笔，而不由聚合价格区间重造一笔。 -/
structure Ends where
  first : Stroke
  last : Stroke

def summarize : List Stroke → Option Ends
  | [] => none
  | a :: rest => some ⟨a, lastStroke a rest⟩

def join : Option Ends → Option Ends → Option Ends
  | none, b => b
  | a, none => a
  | some a, some b => some ⟨a.first, b.last⟩

def read : Option Ends → Rat
  | none => 0
  | some s => s.last.velocity - s.first.velocity

def grouped (parts : List (List Stroke)) : Option Ends :=
  (parts.map summarize).foldr join none

def ValidStroke (s : Stroke) : Prop :=
  s.WellFormed ∧ 0 < s.startPrice ∧ 0 < s.endPrice ∧
  (match s.direction with
   | .up => s.startPrice < s.endPrice
   | .down => s.endPrice < s.startPrice)

def Joined (a b : Stroke) : Prop :=
  a.endIndex = b.startIndex ∧ a.endPrice = b.startPrice ∧ a.direction ≠ b.direction

def ValidChain : List Stroke → Prop
  | [] => True
  | [s] => ValidStroke s
  | a :: b :: rest => ValidStroke a ∧ Joined a b ∧ ValidChain (b :: rest)

def leftSlow : Stroke := ⟨.up,0,2,100,104⟩
def leftFast : Stroke := ⟨.up,0,2,96,104⟩
def right : Stroke := ⟨.down,2,4,104,102⟩

def COMPOSITION_TARGET : Prop :=
  (∀ xs, read (summarize xs) = impulse xs) ∧
  (∀ xs ys, summarize (xs ++ ys) = join (summarize xs) (summarize ys)) ∧
  (∀ a b c, join (join a b) c = join a (join b c)) ∧
  (∀ parts, grouped parts = summarize parts.flatten) ∧
  (∀ parts, read (grouped parts) = impulse parts.flatten) ∧
  (∀ p q, p.flatten = q.flatten → read (grouped p) = read (grouped q))

def SEAM_TARGET : Prop :=
  ∀ a b as bs,
    impulse ((a :: as) ++ (b :: bs)) =
      impulse (a :: as) + (b.velocity - (lastStroke a as).velocity) + impulse (b :: bs)

def LOSS_TARGET : Prop :=
  (∀ s, impulse [s] = 0) ∧
  (¬ ∃ merge : Rat → Rat → Rat,
    ∀ (a b : List Stroke), a ≠ [] → b ≠ [] → ValidChain (a ++ b) →
      merge (impulse a) (impulse b) = impulse (a ++ b))

def WITNESS_TARGET : Prop :=
  ValidChain [leftSlow,right] ∧ ValidChain [leftFast,right] ∧
  impulse [leftSlow] = 0 ∧ impulse [leftFast] = 0 ∧ impulse [right] = 0 ∧
  impulse [leftSlow,right] = -3 ∧ impulse [leftFast,right] = -5 ∧
  impulse ([leftSlow] ++ [right]) ≠ impulse [leftSlow] + impulse [right] ∧
  read (grouped [[leftSlow],[],[right]]) = -3

end ForceTransport
