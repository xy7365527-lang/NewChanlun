import RetileProof
import Formal.CenterTrichotomy

namespace CenterExit

open NewChanlun.Origin
open CenterAttempt (ObservedUnit obs visible)

structure ExitRecord where
  frozen : CenterFrame.Frame
  departure : ObservedUnit
  remaining : List ObservedUnit
  availableAt : Nat
  nextOffset : Nat

def release (f : CenterFrame.Frame) (leave ret : ObservedUnit) (rest : List ObservedUnit) : ExitRecord :=
  ⟨f,leave,ret::rest,max f.knownAt (max leave.1.knownAt ret.1.knownAt),(CenterFrame.members f).length+1⟩

def Released (side : Side) (f : CenterFrame.Frame) (leave ret : ObservedUnit)
    (rest : List ObservedUnit) (out : ExitRecord) : Prop :=
  CenterFrame.pair side f leave ret ∧ (CenterFrame.members f).length < 9 ∧
  out.frozen = f ∧ out.departure = leave ∧ out.remaining = ret::rest ∧
  out.availableAt = max f.knownAt (max leave.1.knownAt ret.1.knownAt) ∧
  out.nextOffset = (CenterFrame.members f).length+1

def RELEASE_TARGET : Prop :=
  (∀ side f leave ret rest, CenterFrame.pair side f leave ret → (CenterFrame.members f).length < 9 →
    ∃ out, Released side f leave ret rest out ∧ ∀ other, Released side f leave ret rest other → other = out) ∧
  (∀ side f leave ret rest out, Released side f leave ret rest out →
    CenterFrame.members out.frozen ++ [out.departure] ++ out.remaining =
      CenterFrame.members f ++ leave::ret::rest ∧
    f.knownAt ≤ out.availableAt ∧ leave.1.knownAt ≤ out.availableAt ∧ ret.1.knownAt ≤ out.availableAt) ∧
  (∀ side f leave ret rest out, CenterFrame.Returned side f leave ret → ¬ Released side f leave ret rest out)

def path (trend : Bool) : Nat → Int
  | 0 => 10040 | 1 => 10020 | 2 => 10032 | 3 => 10024
  | 4 => 10050 | 5 => 10042 | 6 => 10046 | 7 => 10060
  | 8 => if trend then 10048 else 10036 | 9 => 10056 | _ => 10050
def u5 : DCSpec.Unit := ⟨.up,5,7,8⟩
def u6 : DCSpec.Unit := ⟨.down,7,8,9⟩
def u7 : DCSpec.Unit := ⟨.up,8,9,10⟩
def units := CenterAttempt.unitsA ++ [u5,u6,u7]
def tail : DCSpec.Tail := ⟨.down,9,10⟩
def result : DCSpec.Result := .started .down 0 1 (units,tail)
def initialTriple (b : Bool) : Retile.Triple :=
  ⟨obs (path b) CenterAttempt.a0,obs (path b) CenterAttempt.a1,obs (path b) CenterAttempt.a2⟩
def nextTriple (b : Bool) : Retile.Triple :=
  ⟨obs (path b) CenterAttempt.retA,obs (path b) u5,obs (path b) u6⟩
def frame (b : Bool) := CenterFrame.initial (initialTriple b).a (initialTriple b).b (initialTriple b).c 4
def leave (b : Bool) := obs (path b) CenterAttempt.leaveA
def ret (b : Bool) := obs (path b) CenterAttempt.retA
def exitAt (b : Bool) := release (frame b) (leave b) (ret b) []

def centerFrom (t : Retile.Triple) (h : Retile.Local t) : Formal.CenterTrichotomy.Center :=
  let z := centerFullOfConfirmed t.a.2 t.b.2 t.c.2 h
  { dd := z.dd,zd := z.core.zd,zg := z.core.zg,gg := z.gg
    core_valid := h.2,outer_lo := z.outer_lo,outer_hi := z.outer_hi }

def initialLocal (b : Bool) : Retile.Local (initialTriple b) := by
  cases b <;> unfold Retile.Local CenterConfirmedComplete DirAlternates <;> decide
def nextLocal (b : Bool) : Retile.Local (nextTriple b) := by
  cases b <;> unfold Retile.Local CenterConfirmedComplete DirAlternates <;> decide
def initialCenter (b : Bool) := centerFrom (initialTriple b) (initialLocal b)
def nextCenter (b : Bool) := centerFrom (nextTriple b) (nextLocal b)

def FORK_TARGET : Prop :=
  (∀ b, DCSpec.Whole (path b) 2 11 result) ∧
  (∀ b, ∀ i : Fin 11, 0 < CenterAttempt.quantity (path b) i.val ∧
    CenterAttempt.projected (CenterAttempt.quantity (path b) i.val) = path b i.val) ∧
  (∀ i, i ≤ 6 → path true i = path false i) ∧
  visible (path true) result 6 = visible (path false) result 6 ∧
  (∀ b, CenterFrame.members (frame b) ++ [leave b,ret b] = visible (path b) result 6 ∧
    Released .long (frame b) (leave b) (ret b) [] (exitAt b)) ∧
  exitAt true = exitAt false ∧ (exitAt true).availableAt = 6 ∧ (exitAt true).nextOffset = 4 ∧
  (∀ b, (visible (path b) result 9).drop 4 = Retile.tripleMembers (nextTriple b)) ∧
  (∀ b, Formal.CenterTrichotomy.SameLevelNewCenterPair (initialCenter b) (nextCenter b)) ∧
  Formal.CenterTrichotomy.classify (initialCenter true) (nextCenter true) = .upContinuation ∧
  Formal.CenterTrichotomy.IsUpContinuation (initialCenter true) (nextCenter true) ∧
  Formal.CenterTrichotomy.classify (initialCenter false) (nextCenter false) = .levelExpansion ∧
  Formal.CenterTrichotomy.IsLevelExpansion (initialCenter false) (nextCenter false)

def NO_EARLY_CHOICE_TARGET : Prop :=
  ¬ ∃ choose : List ObservedUnit → Formal.CenterTrichotomy.CenterRelation,
    choose (visible (path true) result 6) = Formal.CenterTrichotomy.classify (initialCenter true) (nextCenter true) ∧
    choose (visible (path false) result 6) = Formal.CenterTrichotomy.classify (initialCenter false) (nextCenter false)

end CenterExit
