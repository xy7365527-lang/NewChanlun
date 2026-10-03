import CenterAttemptProof

namespace CenterFrame

open NewChanlun.Origin
open CenterAttempt (ObservedUnit Joined obs visible)

structure Frame where
  a : ObservedUnit
  b : ObservedUnit
  c : ObservedUnit
  extra : List ObservedUnit
  revision : Nat
  knownAt : Nat

def members (f : Frame) : List ObservedUnit := [f.a,f.b,f.c] ++ f.extra
def right (f : Frame) : ObservedUnit := f.extra.getLast?.getD f.c
def zd (f : Frame) : Int := computeZD f.a.2 f.b.2 f.c.2
def zg (f : Frame) : Int := computeZG f.a.2 f.b.2 f.c.2
def initial (a b c : ObservedUnit) (t : Nat) : Frame := ⟨a,b,c,[],0,t⟩

def PairBase (f : Frame) (leave ret : ObservedUnit) : Prop :=
  CenterConfirmedComplete f.a.2 f.b.2 f.c.2 ∧
  Joined f.a f.b ∧ Joined f.b f.c ∧ Joined (right f) leave ∧ Joined leave ret

def pair (side : Side) (f : Frame) (leave ret : ObservedUnit) : Prop :=
  PairBase f leave ret ∧ match side with
  | .long => leave.2.direction = .up ∧ ret.2.direction = .down ∧
      zg f < leave.2.endPrice ∧ zg f < segLow ret.2
  | .short => leave.2.direction = .down ∧ ret.2.direction = .up ∧
      leave.2.endPrice < zd f ∧ segHigh ret.2 < zd f

def Returned (side : Side) (f : Frame) (leave ret : ObservedUnit) : Prop :=
  PairBase f leave ret ∧ match side with
  | .long => leave.2.direction = .up ∧ ret.2.direction = .down ∧
      zg f < leave.2.endPrice ∧ segLow ret.2 ≤ zg f ∧ zd f ≤ segHigh ret.2
  | .short => leave.2.direction = .down ∧ ret.2.direction = .up ∧
      leave.2.endPrice < zd f ∧ zd f ≤ segHigh ret.2 ∧ segLow ret.2 ≤ zg f

def extend (f : Frame) (leave ret : ObservedUnit) : Frame :=
  ⟨f.a,f.b,f.c,f.extra ++ [leave,ret],f.revision+1,max f.knownAt (max leave.1.knownAt ret.1.knownAt)⟩

/-- Independent field contract; no equality to the constructor function. -/
def Recorded (f : Frame) (leave ret : ObservedUnit) (g : Frame) : Prop :=
  g.a = f.a ∧ g.b = f.b ∧ g.c = f.c ∧ g.extra = f.extra ++ [leave,ret] ∧
  g.revision = f.revision + 1 ∧
  g.knownAt = max f.knownAt (max leave.1.knownAt ret.1.knownAt)

inductive Outcome where
  | active (frame : Frame)
  | retilePending (frame : Frame)

def frameOf : Outcome → Frame
  | .active f => f
  | .retilePending f => f

def Absorb (side : Side) (f : Frame) (leave ret : ObservedUnit) (out : Outcome) : Prop :=
  Returned side f leave ret ∧ (members f).length < 9 ∧
  match out with
  | .active g => Recorded f leave ret g ∧ (members g).length < 9
  | .retilePending g => Recorded f leave ret g ∧ 9 ≤ (members g).length

def UPDATE_TARGET : Prop :=
  (∀ a b c leave ret t, pair .long (initial a b c t) leave ret ↔ CenterAttempt.UpPair a b c leave ret) ∧
  (∀ side f leave ret, Returned side f leave ret → (members f).length < 9 →
    ∃ out, Absorb side f leave ret out ∧ ∀ other, Absorb side f leave ret other → other = out) ∧
  (∀ side f leave ret out, Absorb side f leave ret out →
    let g := frameOf out
    members g = members f ++ [leave,ret] ∧ zd g = zd f ∧ zg g = zg f ∧
    f.knownAt ≤ g.knownAt ∧ leave.1.knownAt ≤ g.knownAt ∧ ret.1.knownAt ≤ g.knownAt ∧
    (∀ pre post : List ObservedUnit, pre ++ members g ++ post = pre ++ members f ++ [leave,ret] ++ post)) ∧
  (∀ side f leave ret, Returned side f leave ret → ¬ pair side f leave ret)

def b0 : Frame := initial (obs CenterAttempt.pathB CenterAttempt.a0)
  (obs CenterAttempt.pathB CenterAttempt.a1) (obs CenterAttempt.pathB CenterAttempt.a2) 4
def b1 : Frame := extend b0 (obs CenterAttempt.pathB CenterAttempt.leaveA) (obs CenterAttempt.pathB CenterAttempt.retB)

def B_TARGET : Prop :=
  DCSpec.Whole CenterAttempt.pathB 2 11 CenterAttempt.resultB ∧
  Absorb .long b0 (obs CenterAttempt.pathB CenterAttempt.leaveA) (obs CenterAttempt.pathB CenterAttempt.retB) (.active b1) ∧
  members b0 = visible CenterAttempt.pathB CenterAttempt.resultB 4 ∧
  members b1 = visible CenterAttempt.pathB CenterAttempt.resultB 7 ∧
  b1.revision = 1 ∧ b1.knownAt = 7 ∧
  ¬ pair .long b0 (obs CenterAttempt.pathB CenterAttempt.leaveA) (obs CenterAttempt.pathB CenterAttempt.retB) ∧
  ¬ pair .long b0 (obs CenterAttempt.pathB CenterAttempt.laterLeaveB) (obs CenterAttempt.pathB CenterAttempt.laterRetB) ∧
  pair .long b1 (obs CenterAttempt.pathB CenterAttempt.laterLeaveB) (obs CenterAttempt.pathB CenterAttempt.laterRetB)

def pathC : Nat → Int
  | 0 => 10040 | 1 => 10020 | 2 => 10032 | 3 => 10024
  | 4 => 10050 | 5 => 10030 | 6 => 10060 | 7 => 10028
  | 8 => 10070 | 9 => 10026 | _ => 10030
def c3 : DCSpec.Unit := ⟨.up,3,4,5⟩
def c4 : DCSpec.Unit := ⟨.down,4,5,6⟩
def c5 : DCSpec.Unit := ⟨.up,5,6,7⟩
def c6 : DCSpec.Unit := ⟨.down,6,7,8⟩
def c7 : DCSpec.Unit := ⟨.up,7,8,9⟩
def c8 : DCSpec.Unit := ⟨.down,8,9,10⟩
def unitsC : List DCSpec.Unit := [CenterAttempt.a0,CenterAttempt.a1,CenterAttempt.a2,c3,c4,c5,c6,c7,c8]
def tailC : DCSpec.Tail := ⟨.up,9,10⟩
def resultC : DCSpec.Result := .started .down 0 1 (unitsC,tailC)
def f0 : Frame := initial (obs pathC CenterAttempt.a0) (obs pathC CenterAttempt.a1) (obs pathC CenterAttempt.a2) 4
def f1 : Frame := extend f0 (obs pathC c3) (obs pathC c4)
def f2 : Frame := extend f1 (obs pathC c5) (obs pathC c6)
def f3 : Frame := extend f2 (obs pathC c7) (obs pathC c8)

def NINE_TARGET : Prop :=
  DCSpec.Whole pathC 2 11 resultC ∧
  Absorb .long f0 (obs pathC c3) (obs pathC c4) (.active f1) ∧
  Absorb .long f1 (obs pathC c5) (obs pathC c6) (.active f2) ∧
  Absorb .long f2 (obs pathC c7) (obs pathC c8) (.retilePending f3) ∧
  (¬ ∃ g, Absorb .long f2 (obs pathC c7) (obs pathC c8) (.active g)) ∧
  (members f0).length = 3 ∧ (members f1).length = 5 ∧
  (members f2).length = 7 ∧ (members f3).length = 9 ∧
  members f1 = visible pathC resultC 6 ∧ members f2 = visible pathC resultC 8 ∧
  members f3 = visible pathC resultC 10 ∧ f3.knownAt = 10 ∧
  (∀ i : Fin 11, 0 < CenterAttempt.quantity pathC i.val ∧
    CenterAttempt.projected (CenterAttempt.quantity pathC i.val) = pathC i.val)

end CenterFrame
