import DCOriginBridge
import DCCausalProof
import Origin.BspClassification

namespace CenterAttempt

open NewChanlun.Origin

abbrev ObservedUnit := DCSpec.Unit × Segment

def visible (p : Nat → Int) (r : DCSpec.Result) (t : Nat) : List ObservedUnit :=
  ((DCSpec.units r).filter (fun u => u.knownAt ≤ t)).map
    (fun u => (u, DCOriginBridge.embed p u))

def Joined (a b : ObservedUnit) : Prop := a.1.finish = b.1.start

instance (a b : ObservedUnit) : Decidable (Joined a b) := by
  unfold Joined
  infer_instance

def UpPair (a b c leave ret : ObservedUnit) : Prop :=
  CenterConfirmedComplete a.2 b.2 c.2 ∧
  Joined a b ∧ Joined b c ∧ Joined c leave ∧ Joined leave ret ∧
  leave.2.direction = .up ∧ ret.2.direction = .down ∧
  computeZG a.2 b.2 c.2 < leave.2.endPrice ∧
  computeZG a.2 b.2 c.2 < segLow ret.2

/-- The pair is the immediate one after this fixed three-member seed. -/
def firstPairUp (xs : List ObservedUnit) (start : Nat) : Prop :=
  match xs.drop start with
  | a::b::c::leave::ret::_ => UpPair a b c leave ret
  | _ => False

def VIEW_CAUSAL_TARGET : Prop :=
  ∀ (p q : Nat → Int) (delta : Int), 0 < delta →
    ∀ len₁ len₂ t r₁ r₂, t < len₁ → t < len₂ →
      (∀ i, i ≤ t → p i = q i) →
      DCSpec.Whole p delta len₁ r₁ → DCSpec.Whole q delta len₂ r₂ →
      visible p r₁ t = visible q r₂ t ∧
      ∀ start, firstPairUp (visible p r₁ t) start ↔ firstPairUp (visible q r₂ t) start

/-- Auxiliary Type1/Type2 fields are not evidence; this endpoint is used only by IsType3Buy. -/
def pairEndpoint (a b c leave ret : ObservedUnit) (h : UpPair a b c leave ret) : BspEndpoint where
  side := .long
  center := ⟨computeZD a.2 b.2 c.2, computeZG a.2 b.2 c.2,
    a.2.startIndex, c.2.endIndex, Int.le_of_lt h.1.2⟩
  divPair := ⟨⟨0⟩,⟨0⟩,false⟩
  brokeCenter := false
  afterTypeOne := false
  leftCenter := decide (leave.2.direction = .up ∧ computeZG a.2 b.2 c.2 < leave.2.endPrice)
  retracePrice := segLow ret.2
  firstRetrace := decide (Joined c leave ∧ Joined leave ret)

def TYPE3_BRIDGE_TARGET : Prop :=
  ∀ xs start, firstPairUp xs start →
    ∃ a b c leave ret rest, xs.drop start = a::b::c::leave::ret::rest ∧
      ∃ h : UpPair a b c leave ret, IsType3Buy (pairEndpoint a b c leave ret h)

def pathA : Nat → Int
  | 0 => 10040 | 1 => 10020 | 2 => 10032 | 3 => 10024
  | 4 => 10050 | 5 => 10042 | _ => 10046

def pathB : Nat → Int
  | 0 => 10040 | 1 => 10020 | 2 => 10032 | 3 => 10024
  | 4 => 10050 | 5 => 10042 | 6 => 10030 | 7 => 10034
  | 8 => 10060 | 9 => 10045 | _ => 10055

def a0 : DCSpec.Unit := ⟨.down,0,1,2⟩
def a1 : DCSpec.Unit := ⟨.up,1,2,3⟩
def a2 : DCSpec.Unit := ⟨.down,2,3,4⟩
def leaveA : DCSpec.Unit := ⟨.up,3,4,5⟩
def retA : DCSpec.Unit := ⟨.down,4,5,6⟩
def retB : DCSpec.Unit := ⟨.down,4,6,7⟩
def laterLeaveB : DCSpec.Unit := ⟨.up,6,8,9⟩
def laterRetB : DCSpec.Unit := ⟨.down,8,9,10⟩
def unitsA : List DCSpec.Unit := [a0,a1,a2,leaveA,retA]
def unitsB : List DCSpec.Unit := [a0,a1,a2,leaveA,retB,laterLeaveB,laterRetB]
def tailA : DCSpec.Tail := ⟨.up,5,6⟩
def tailB : DCSpec.Tail := ⟨.up,9,10⟩
def resultA : DCSpec.Result := .started .down 0 1 (unitsA,tailA)
def resultB : DCSpec.Result := .started .down 0 1 (unitsB,tailB)
def obs (p : Nat → Int) (u : DCSpec.Unit) : ObservedUnit := (u,DCOriginBridge.embed p u)

def quantity (p : Nat → Int) (i : Nat) : Int :=
  let x := p i - 10000
  (x*10000 + (200-x)/2)/(200-x)
def projected (qb : Int) : Int :=
  (2*(10200*qb+10000*10000)+(qb+10000))/(2*(qb+10000))

def WITNESS_TARGET : Prop :=
  DCSpec.Whole pathA 2 7 resultA ∧ DCSpec.Whole pathB 2 11 resultB ∧
  (∀ i : Fin 6, pathA i.val = pathB i.val) ∧
  visible pathA resultA 5 = visible pathB resultB 5 ∧
  (visible pathA resultA 5).length = 4 ∧
  ¬ firstPairUp (visible pathA resultA 5) 0 ∧
  firstPairUp (visible pathA resultA 6) 0 ∧
  ¬ firstPairUp (visible pathB resultB 10) 0 ∧
  computeZG (DCOriginBridge.embed pathB a0) (DCOriginBridge.embed pathB a1)
    (DCOriginBridge.embed pathB a2) < (DCOriginBridge.embed pathB laterLeaveB).endPrice ∧
  computeZG (DCOriginBridge.embed pathB a0) (DCOriginBridge.embed pathB a1)
    (DCOriginBridge.embed pathB a2) < segLow (DCOriginBridge.embed pathB laterRetB) ∧
  (∀ i : Fin 7, 0 < quantity pathA i.val ∧ projected (quantity pathA i.val) = pathA i.val) ∧
  (∀ i : Fin 11, 0 < quantity pathB i.val ∧ projected (quantity pathB i.val) = pathB i.val)

end CenterAttempt
