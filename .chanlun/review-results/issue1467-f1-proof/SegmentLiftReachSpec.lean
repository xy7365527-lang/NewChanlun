import SegmentLiftReachFixture
import Origin.SegmentAutoConstruct
import RetileProof

namespace SegmentLiftReach
open NewChanlun.Origin
open CenterAttempt (ObservedUnit)

def price (i : Nat) : Int := prices[i]?.getD 0
def qty (i : Nat) : Int := bidQuantities[i]?.getD 0
def stroke (i : Nat) : Stroke := strokes[i]?.getD ⟨.up,0,0,0,0⟩
def endpoint (i : Nat) : Int := macroPrices[i]?.getD 0
def cut (i : Nat) : Nat := 46+28*i
def seg (i : Nat) : Segment :=
  ⟨if i%2=0 then .up else .down,1+28*i,1+28*(i+1),endpoint i,endpoint (i+1)⟩

def SOURCE_TARGET : Prop :=
  prices.length = 283 ∧ bidQuantities.length = 283 ∧
  (∀ i : Fin 283, 0 < qty i.val ∧ 12000 < price i.val ∧ price i.val < 17000 ∧
    let d := qty i.val + 10000
    let n := 17000*qty i.val + 12000*10000
    2*d*price i.val ≤ 2*n+d ∧ 2*n+d < 2*d*(price i.val+1)) ∧
  (∀ i : Fin 282, qty i.val ≠ qty (i.val+1) ∧
    (qty (i.val+1) < qty i.val → 0 < qty i.val-qty (i.val+1) ∧ qty i.val-qty (i.val+1) < qty i.val))

def top (i : Nat) : Prop := price (i-1) < price i ∧ price (i+1) < price i
def bottom (i : Nat) : Prop := price i < price (i-1) ∧ price i < price (i+1)
def StrokeSource (s : Stroke) : Prop :=
  0 < s.startIndex ∧ s.startIndex+3 < s.endIndex ∧ s.endIndex+1 < prices.length ∧
  s.startPrice = price s.startIndex ∧ s.endPrice = price s.endIndex ∧
  (match s.direction with
    | .up => bottom s.startIndex ∧ top s.endIndex ∧ s.startPrice < s.endPrice
    | .down => top s.startIndex ∧ bottom s.endIndex ∧ s.endPrice < s.startPrice)

def STROKE_TARGET : Prop :=
  strokes.length = 70 ∧ (∀ i : Fin 282, price i.val ≠ price (i.val+1)) ∧
  (∀ i : Fin 70, StrokeSource (stroke i.val) ∧
    (stroke i.val).startIndex = 1+4*i.val ∧ (stroke i.val).endIndex = 5+4*i.val) ∧
  (∀ i : Fin 69, (stroke i.val).endIndex = (stroke (i.val+1)).startIndex ∧
    (stroke i.val).endPrice = (stroke (i.val+1)).startPrice ∧
    (stroke i.val).direction ≠ (stroke (i.val+1)).direction)

def input (i : Nat) : List Stroke := (strokes.drop (7*i)).take 10
def datum (i : Nat) : SegEndData :=
  ⟨(seg i).direction,FeatureElem.ofStroke (stroke (7*i+5)),FeatureElem.ofStroke (stroke (7*i+7)),
   FeatureElem.ofStroke (stroke (7*i+9)),[],(seg i).startPrice,(seg i).endPrice⟩

def SelectedBy (d : SegEndData) (ss : List Stroke) : Prop :=
  match extractSegEndData d.dir d.startPrice d.endPrice ss with
  | none => False
  | some actual => actual.dir = d.dir ∧ actual.startPrice = d.startPrice ∧ actual.endPrice = d.endPrice ∧
      actual.e1.low = d.e1.low ∧ actual.e1.high = d.e1.high ∧
      actual.e2.low = d.e2.low ∧ actual.e2.high = d.e2.high ∧
      actual.e3.low = d.e3.low ∧ actual.e3.high = d.e3.high

def NoAdjacentContainment (d : Direction) (ss : List Stroke) : Prop :=
  let es := (extractFeatureStrokes d ss).map FeatureElem.ofStroke
  ∀ i : Fin (es.length-1),
    let a := es[i.val]?.getD (FeatureElem.ofStroke (stroke 0))
    let b := es[i.val+1]?.getD (FeatureElem.ofStroke (stroke 0))
    ¬ Contains a b ∧ ¬ Contains b a

def initialCore (startAt : Nat) : Prop :=
  let a := FeatureElem.ofStroke (stroke startAt)
  let b := FeatureElem.ofStroke (stroke (startAt+1))
  let c := FeatureElem.ofStroke (stroke (startAt+2))
  max a.low (max b.low c.low) < min a.high (min b.high c.high)

def SourceRange (i : Nat) : Prop :=
  (stroke (7*i)).startIndex = (seg i).startIndex ∧ (stroke (7*i+6)).endIndex = (seg i).endIndex ∧
  (stroke (7*i)).direction = (seg i).direction ∧ (stroke (7*i+6)).direction = (seg i).direction ∧
  (seg i).startPrice = price (seg i).startIndex ∧ (seg i).endPrice = price (seg i).endIndex ∧
  (seg i).endIndex = (stroke (7*i+7)).startIndex ∧
  (∀ j : Fin 283, (seg i).startIndex ≤ j.val → j.val ≤ (seg i).endIndex →
    segLow (seg i) ≤ price j.val ∧ price j.val ≤ segHigh (seg i)) ∧
  (∀ j : Fin (7*i+10), (stroke j.val).endIndex < cut i)

def SEGMENT_TARGET : Prop := ∀ i : Fin 9,
  SourceRange i.val ∧ initialCore (7*i.val) ∧ initialCore (7*i.val+7) ∧
  SelectedBy (datum i.val) (input i.val) ∧
  NoAdjacentContainment (seg i.val).direction (input i.val) ∧
  ¬ HasGap (datum i.val).e1 (datum i.val).e2 ∧
  (if i.val%2=0 then
    (datum i.val).e1.low < (datum i.val).e2.low ∧ (datum i.val).e3.low < (datum i.val).e2.low ∧
      (seg i.val).endPrice = (datum i.val).e2.high
   else (datum i.val).e2.high < (datum i.val).e1.high ∧ (datum i.val).e2.high < (datum i.val).e3.high ∧
      (seg i.val).endPrice = (datum i.val).e2.low) ∧
  SegEndComplete (datum i.val)

-- Metadata reuse only. These certified segment spans are not asserted to be DC units.
def unit (i : Nat) : ObservedUnit :=
  (⟨if i%2=0 then .up else .down,(seg i).startIndex,(seg i).endIndex,cut i⟩,seg i)
def units : List ObservedUnit := (List.range 9).map unit
def f0 := CenterFrame.initial (unit 0) (unit 1) (unit 2) (cut 2)
def f1 := CenterFrame.extend f0 (unit 3) (unit 4)
def f2 := CenterFrame.extend f1 (unit 5) (unit 6)
def f3 := CenterFrame.extend f2 (unit 7) (unit 8)
def cells := (Retile.retile f3.knownAt (CenterFrame.members f3)).1
def rest := (Retile.retile f3.knownAt (CenterFrame.members f3)).2

def FRAME_TARGET : Prop :=
  CenterFrame.Absorb .short f0 (unit 3) (unit 4) (.active f1) ∧
  CenterFrame.Absorb .short f1 (unit 5) (unit 6) (.active f2) ∧
  CenterFrame.Absorb .short f2 (unit 7) (unit 8) (.retilePending f3) ∧
  CenterFrame.members f3 = units ∧ f3.knownAt = 270 ∧
  CenterFrame.zd f3 = 14000 ∧ CenterFrame.zg f3 = 15500 ∧
  (∀ i : Fin 9, segLow (seg i.val) ≤ 15500 ∧ 14000 ≤ segHigh (seg i.val)) ∧
  cells.map Retile.accepted = [true,true,true] ∧ rest = [] ∧
  cells.map Retile.numbers = [some (14000,15500,14000,16000),
    some (13500,14000,13000,15500),some (13500,14000,13000,14000)] ∧
  cells.map Retile.knownAt = [270,270,270] ∧ Retile.recover cells rest = units

def REJECTION_TARGET : Prop := ∀ a b c : Segment,
  segLow a = 14000 → segHigh a = 16000 →
  segLow b = 13000 → segHigh b = 15500 →
  segLow c = 13000 → segHigh c = 14000 → ¬ CenterConfirmedComplete a b c

end SegmentLiftReach
