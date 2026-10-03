import RetileProof

namespace LiftBoundary

open NewChanlun.Origin
open CenterAttempt (ObservedUnit obs visible)
open UpgradeResearch (Interval Valid StrictTouch core3)

def outerCore (cells : List Retile.Cell) : Option Interval :=
  match cells with
  | [a,b,c] => match Retile.centerOf a, Retile.centerOf b, Retile.centerOf c with
      | some x, some y, some z => some (core3 ⟨x.dd,x.gg⟩ ⟨y.dd,y.gg⟩ ⟨z.dd,z.gg⟩)
      | _,_,_ => none
  | _ => none

def strictLift (cells : List Retile.Cell) : Prop :=
  match outerCore cells with
  | some i => Valid i
  | none => False

def closedTouches (f : CenterFrame.Frame) : Prop :=
  (CenterFrame.members f).all (fun u =>
    decide (segLow u.2 ≤ CenterFrame.zg f ∧ CenterFrame.zd f ≤ segHigh u.2)) = true

def AutomaticStrictLift : Prop :=
  ∀ (p : Nat → Int) (delta : Int), 0 < delta → ∀ len result,
    DCSpec.Whole p delta len result →
    ∀ side before leave ret after,
      CenterFrame.Absorb side before leave ret (.retilePending after) →
      CenterFrame.members after = visible p result after.knownAt →
      closedTouches after →
      (Retile.retile after.knownAt (CenterFrame.members after)).2 = [] →
      (Retile.retile after.knownAt (CenterFrame.members after)).1.map Retile.accepted = [true,true,true] →
      strictLift (Retile.retile after.knownAt (CenterFrame.members after)).1

def INTERSECTION_TARGET : Prop := ∀ (a b c : Interval),
  Valid a → Valid b → Valid c →
    (Valid (core3 a b c) ↔ StrictTouch a b ∧ StrictTouch b c ∧ StrictTouch a c)

def path : Nat → Int
  | 0 => 10040 | 1 => 10060 | 2 => 10040 | 3 => 10055
  | 4 => 10030 | 5 => 10040 | 6 => 10035 | 7 => 10040
  | 8 => 10030 | 9 => 10040 | _ => 10036
def u0 : DCSpec.Unit := ⟨.up,0,1,2⟩
def u1 : DCSpec.Unit := ⟨.down,1,2,3⟩
def u2 : DCSpec.Unit := ⟨.up,2,3,4⟩
def u3 : DCSpec.Unit := ⟨.down,3,4,5⟩
def u4 : DCSpec.Unit := ⟨.up,4,5,6⟩
def u5 : DCSpec.Unit := ⟨.down,5,6,7⟩
def u6 : DCSpec.Unit := ⟨.up,6,7,8⟩
def u7 : DCSpec.Unit := ⟨.down,7,8,9⟩
def u8 : DCSpec.Unit := ⟨.up,8,9,10⟩
def units : List DCSpec.Unit := [u0,u1,u2,u3,u4,u5,u6,u7,u8]
def tail : DCSpec.Tail := ⟨.down,9,10⟩
def result : DCSpec.Result := .started .up 0 1 (units,tail)
def f0 : CenterFrame.Frame := CenterFrame.initial (obs path u0) (obs path u1) (obs path u2) 4
def f1 : CenterFrame.Frame := CenterFrame.extend f0 (obs path u3) (obs path u4)
def f2 : CenterFrame.Frame := CenterFrame.extend f1 (obs path u5) (obs path u6)
def f3 : CenterFrame.Frame := CenterFrame.extend f2 (obs path u7) (obs path u8)
def cells : List Retile.Cell := (Retile.retile f3.knownAt (CenterFrame.members f3)).1
def rest : List ObservedUnit := (Retile.retile f3.knownAt (CenterFrame.members f3)).2

def WITNESS_TARGET : Prop :=
  DCSpec.Whole path 2 11 result ∧
  CenterFrame.Absorb .short f0 (obs path u3) (obs path u4) (.active f1) ∧
  CenterFrame.Absorb .short f1 (obs path u5) (obs path u6) (.active f2) ∧
  CenterFrame.Absorb .short f2 (obs path u7) (obs path u8) (.retilePending f3) ∧
  CenterFrame.members f3 = visible path result 10 ∧ f3.knownAt = 10 ∧
  (CenterFrame.members f3).length = 9 ∧ closedTouches f3 ∧ rest = [] ∧
  cells.map Retile.accepted = [true,true,true] ∧
  cells.map Retile.numbers = [some (10040,10055,10040,10060),
    some (10035,10040,10030,10055),some (10035,10040,10030,10040)] ∧
  outerCore cells = some ⟨10040,10040⟩ ∧ ¬ strictLift cells ∧
  (path 0 < path 3 ∧ path 6 < path 3 ∧ path 6 < path 9) ∧
  (∀ i : Fin 11, 0 < CenterAttempt.quantity path i.val ∧
    CenterAttempt.projected (CenterAttempt.quantity path i.val) = path i.val)

def ORIGIN_REJECTION_TARGET : Prop := ∀ a b c : Segment,
  segLow a = 10040 → segHigh a = 10060 →
  segLow b = 10030 → segHigh b = 10055 →
  segLow c = 10030 → segHigh c = 10040 → ¬ CenterConfirmedComplete a b c

end LiftBoundary
