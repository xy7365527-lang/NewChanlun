import ReferenceBaseFixture
import Origin.SegmentAutoConstruct
import Origin.CenterComplete

namespace ReferenceBase
open NewChanlun.Origin

def price (i : Nat) : Int := prices[i]?.getD 0
def qty (i : Nat) : Int := bidQuantities[i]?.getD 0
def stroke (i : Nat) : Stroke := strokes[i]?.getD ⟨.up,0,0,0,0⟩

def SOURCE_TARGET : Prop :=
  prices.length = 95 ∧ bidQuantities.length = 95 ∧ 0 < askQuantity ∧
  (∀ i : Fin 95, 0 < qty i.val ∧ 9900 < price i.val ∧ price i.val < 11200 ∧
    11200 * qty i.val + 9900 * askQuantity = price i.val * (qty i.val + askQuantity)) ∧
  (∀ i : Fin 94, qty (i.val+1) ≠ qty i.val ∧
    (qty (i.val+1) < qty i.val → 0 < qty i.val - qty (i.val+1) ∧
      qty i.val - qty (i.val+1) < qty i.val))

def top (i : Nat) : Prop := price (i-1) < price i ∧ price (i+1) < price i
def bottom (i : Nat) : Prop := price i < price (i-1) ∧ price i < price (i+1)
def StrokeSource (s : Stroke) : Prop :=
  0 < s.startIndex ∧ s.startIndex+3 < s.endIndex ∧ s.endIndex+1 < prices.length ∧
  s.startPrice = price s.startIndex ∧ s.endPrice = price s.endIndex ∧
  (match s.direction with
    | .up => bottom s.startIndex ∧ top s.endIndex ∧ s.startPrice < s.endPrice
    | .down => top s.startIndex ∧ bottom s.endIndex ∧ s.endPrice < s.startPrice)

def STROKE_TARGET : Prop :=
  strokes.length = 23 ∧ (∀ i : Fin 94, price i.val ≠ price (i.val+1)) ∧
  (∀ i : Fin 23, StrokeSource (stroke i.val) ∧
    (stroke i.val).startIndex = 1+4*i.val ∧ (stroke i.val).endIndex = 5+4*i.val) ∧
  (∀ i : Fin 22, (stroke i.val).endIndex = (stroke (i.val+1)).startIndex ∧
    (stroke i.val).endPrice = (stroke (i.val+1)).startPrice ∧
    (stroke i.val).direction ≠ (stroke (i.val+1)).direction)

def s0 : Segment := ⟨.up,1,21,10000,11000⟩
def s1 : Segment := ⟨.down,21,49,11000,10100⟩
def s2 : Segment := ⟨.up,49,69,10100,10800⟩
def input0 := strokes.take 8
def input1 := (strokes.drop 5).take 10
def input2 := (strokes.drop 12).take 8
def pairs (d : Direction) (ss : List Stroke) := (buildFeatureSeq d ss).map (fun e => (e.low,e.high))
def d0 : SegEndData := ⟨.up,FeatureElem.ofStroke (stroke 3),FeatureElem.ofStroke (stroke 5),
  FeatureElem.ofStroke (stroke 7),[],10000,11000⟩
def d1 : SegEndData := ⟨.down,FeatureElem.ofStroke (stroke 10),FeatureElem.ofStroke (stroke 12),
  FeatureElem.ofStroke (stroke 14),[],11000,10100⟩
def d2 : SegEndData := ⟨.up,FeatureElem.ofStroke (stroke 15),FeatureElem.ofStroke (stroke 17),
  FeatureElem.ofStroke (stroke 19),[],10100,10800⟩

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

def FEATURE_TARGET : Prop :=
  pairs .up input0 = [(10300,10500),(10600,10800),(10700,11000),(10500,10900)] ∧
  pairs .down input1 = [(10700,10900),(10500,10700),(10300,10500),(10100,10400),(10200,10600)] ∧
  pairs .up input2 = [(10200,10400),(10400,10600),(10500,10800),(10300,10700)] ∧
  NoAdjacentContainment .up input0 ∧ NoAdjacentContainment .down input1 ∧ NoAdjacentContainment .up input2 ∧
  SelectedBy d0 input0 ∧ SelectedBy d1 input1 ∧ SelectedBy d2 input2 ∧
  SegEndComplete d0 ∧ SegEndComplete d1 ∧ SegEndComplete d2 ∧
  ¬ HasGap d0.e1 d0.e2 ∧ ¬ HasGap d1.e1 d1.e2 ∧ ¬ HasGap d2.e1 d2.e2 ∧
  d0.e1.low < d0.e2.low ∧ d0.e3.low < d0.e2.low ∧
  d1.e2.high < d1.e1.high ∧ d1.e2.high < d1.e3.high ∧
  d2.e1.low < d2.e2.low ∧ d2.e3.low < d2.e2.low

def SpanSource (s : Segment) (startAt count : Nat) : Prop :=
  count ≥ 3 ∧ count % 2 = 1 ∧
  (stroke startAt).startIndex = s.startIndex ∧ (stroke (startAt+count-1)).endIndex = s.endIndex ∧
  s.startPrice = price s.startIndex ∧ s.endPrice = price s.endIndex ∧
  (∀ i : Fin count,
    min s.startPrice s.endPrice ≤ (stroke (startAt+i.val)).startPrice ∧
    (stroke (startAt+i.val)).startPrice ≤ max s.startPrice s.endPrice ∧
    min s.startPrice s.endPrice ≤ (stroke (startAt+i.val)).endPrice ∧
    (stroke (startAt+i.val)).endPrice ≤ max s.startPrice s.endPrice)

def initialCore (startAt : Nat) : Prop :=
  let a := FeatureElem.ofStroke (stroke startAt)
  let b := FeatureElem.ofStroke (stroke (startAt+1))
  let c := FeatureElem.ofStroke (stroke (startAt+2))
  max a.low (max b.low c.low) < min a.high (min b.high c.high)

def GEOMETRY_TARGET : Prop :=
  SpanSource s0 0 5 ∧ SpanSource s1 5 7 ∧ SpanSource s2 12 5 ∧
  initialCore 0 ∧ initialCore 5 ∧ initialCore 12 ∧
  s0.endIndex = s1.startIndex ∧ s1.endIndex = s2.startIndex ∧
  s0.endPrice = s1.startPrice ∧ s1.endPrice = s2.startPrice ∧
  s0.endPrice = d0.e2.high ∧ s1.endPrice = d1.e2.low ∧ s2.endPrice = d2.e2.high ∧
  (∀ i : Fin 8, (stroke i.val).endIndex < 38) ∧
  (∀ i : Fin 15, (stroke i.val).endIndex < 66) ∧
  (∀ i : Fin 20, (stroke i.val).endIndex < 86) ∧
  CenterConfirmedComplete s0 s1 s2 ∧ computeZD s0 s1 s2 = 10100 ∧ computeZG s0 s1 s2 = 10800

end ReferenceBase
