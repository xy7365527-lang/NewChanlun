import Origin.SegmentAutoConstruct
import Origin.SegmentFeatureComplete
import Origin.CenterComplete
import Origin.ForceVelocity
namespace Stage60
open NewChanlun.Origin
instance : Inhabited Stroke := ⟨⟨.up,0,0,0,0⟩⟩
instance : Inhabited FeatureElem := ⟨FeatureElem.ofStroke (default : Stroke)⟩
def positivePrices : List Int := [10050,10000,10500,11000,11500,12000,11750,11500,11250,11000,12000,13000,14000,15000,14500,14000,13500,13000,13750,14500,15250,16000,15625,15250,14875,14500,14750,15000,15250,15500,14875,14250,13625,13000,13250,13500,13750,14000,13500,13000,12500,12000,12375,12750,13125,13500,13250,13000,12750,12500,13000,13500,14000,14500,14125,13750,13375,13000,13500,14000,14500,15000,14750,14500,14250,14000,14200,14400,14600,14800,14350,13900,13450,13000,13250,13500,13750,14000,13625,13250,12875,12500,12750,13000,13250,13500,13375,13250,13125,13000,14200,15400,16600,17800,17725,17650,17575,17500,17625,17750,17875,18000,17900,17800,17700,17600,17675,17750,17825,17900,17675,17450,17225,17000,17125,17250,17375,17500,16625,15750,14875,14000,14025]
def positive : List Stroke := [⟨.up,1,5,10000,12000⟩,
⟨.down,5,9,12000,11000⟩,
⟨.up,9,13,11000,15000⟩,
⟨.down,13,17,15000,13000⟩,
⟨.up,17,21,13000,16000⟩,
⟨.down,21,25,16000,14500⟩,
⟨.up,25,29,14500,15500⟩,
⟨.down,29,33,15500,13000⟩,
⟨.up,33,37,13000,14000⟩,
⟨.down,37,41,14000,12000⟩,
⟨.up,41,45,12000,13500⟩,
⟨.down,45,49,13500,12500⟩,
⟨.up,49,53,12500,14500⟩,
⟨.down,53,57,14500,13000⟩,
⟨.up,57,61,13000,15000⟩,
⟨.down,61,65,15000,14000⟩,
⟨.up,65,69,14000,14800⟩,
⟨.down,69,73,14800,13000⟩,
⟨.up,73,77,13000,14000⟩,
⟨.down,77,81,14000,12500⟩,
⟨.up,81,85,12500,13500⟩,
⟨.down,85,89,13500,13000⟩,
⟨.up,89,93,13000,17800⟩,
⟨.down,93,97,17800,17500⟩,
⟨.up,97,101,17500,18000⟩,
⟨.down,101,105,18000,17600⟩,
⟨.up,105,109,17600,17900⟩,
⟨.down,109,113,17900,17000⟩,
⟨.up,113,117,17000,17500⟩,
⟨.down,117,121,17500,14000⟩]
def positiveInput0 := (positive.drop 0).take 8
def positiveD0 : SegEndData := ⟨.up,FeatureElem.ofStroke (positive[3]!),FeatureElem.ofStroke (positive[5]!),FeatureElem.ofStroke (positive[7]!),[],10000,16000⟩
def positiveInput1 := (positive.drop 5).take 8
def positiveD1 : SegEndData := ⟨.down,FeatureElem.ofStroke (positive[8]!),FeatureElem.ofStroke (positive[10]!),FeatureElem.ofStroke (positive[12]!),[],16000,12000⟩
def positiveInput2 := (positive.drop 10).take 8
def positiveD2 : SegEndData := ⟨.up,FeatureElem.ofStroke (positive[13]!),FeatureElem.ofStroke (positive[15]!),FeatureElem.ofStroke (positive[17]!),[],12000,15000⟩
def positiveInput3 := (positive.drop 15).take 8
def positiveD3 : SegEndData := ⟨.down,FeatureElem.ofStroke (positive[18]!),FeatureElem.ofStroke (positive[20]!),FeatureElem.ofStroke (positive[22]!),[],15000,12500⟩
def positiveInput4 := (positive.drop 20).take 8
def positiveD4 : SegEndData := ⟨.up,FeatureElem.ofStroke (positive[23]!),FeatureElem.ofStroke (positive[25]!),FeatureElem.ofStroke (positive[27]!),[],12500,18000⟩
def negativePrices : List Int := [10050,10000,10500,11000,11500,12000,11750,11500,11250,11000,12000,13000,14000,15000,14500,14000,13500,13000,13750,14500,15250,16000,15625,15250,14875,14500,14750,15000,15250,15500,14875,14250,13625,13000,13250,13500,13750,14000,13500,13000,12500,12000,12375,12750,13125,13500,13250,13000,12750,12500,13000,13500,14000,14500,14125,13750,13375,13000,13500,14000,14500,15000,14750,14500,14250,14000,14200,14400,14600,14800,14350,13900,13450,13000,13250,13500,13750,14000,13625,13250,12875,12500,12750,13000,13250,13500,13375,13250,13125,13000,14200,15400,16600,17800,17350,16900,16450,16000,16500,17000,17500,18000,17900,17800,17700,17600,17675,17750,17825,17900,17675,17450,17225,17000,17125,17250,17375,17500,16625,15750,14875,14000,14025]
def negative : List Stroke := [⟨.up,1,5,10000,12000⟩,
⟨.down,5,9,12000,11000⟩,
⟨.up,9,13,11000,15000⟩,
⟨.down,13,17,15000,13000⟩,
⟨.up,17,21,13000,16000⟩,
⟨.down,21,25,16000,14500⟩,
⟨.up,25,29,14500,15500⟩,
⟨.down,29,33,15500,13000⟩,
⟨.up,33,37,13000,14000⟩,
⟨.down,37,41,14000,12000⟩,
⟨.up,41,45,12000,13500⟩,
⟨.down,45,49,13500,12500⟩,
⟨.up,49,53,12500,14500⟩,
⟨.down,53,57,14500,13000⟩,
⟨.up,57,61,13000,15000⟩,
⟨.down,61,65,15000,14000⟩,
⟨.up,65,69,14000,14800⟩,
⟨.down,69,73,14800,13000⟩,
⟨.up,73,77,13000,14000⟩,
⟨.down,77,81,14000,12500⟩,
⟨.up,81,85,12500,13500⟩,
⟨.down,85,89,13500,13000⟩,
⟨.up,89,93,13000,17800⟩,
⟨.down,93,97,17800,16000⟩,
⟨.up,97,101,16000,18000⟩,
⟨.down,101,105,18000,17600⟩,
⟨.up,105,109,17600,17900⟩,
⟨.down,109,113,17900,17000⟩,
⟨.up,113,117,17000,17500⟩,
⟨.down,117,121,17500,14000⟩]
def negativeInput0 := (negative.drop 0).take 8
def negativeD0 : SegEndData := ⟨.up,FeatureElem.ofStroke (negative[3]!),FeatureElem.ofStroke (negative[5]!),FeatureElem.ofStroke (negative[7]!),[],10000,16000⟩
def negativeInput1 := (negative.drop 5).take 8
def negativeD1 : SegEndData := ⟨.down,FeatureElem.ofStroke (negative[8]!),FeatureElem.ofStroke (negative[10]!),FeatureElem.ofStroke (negative[12]!),[],16000,12000⟩
def negativeInput2 := (negative.drop 10).take 8
def negativeD2 : SegEndData := ⟨.up,FeatureElem.ofStroke (negative[13]!),FeatureElem.ofStroke (negative[15]!),FeatureElem.ofStroke (negative[17]!),[],12000,15000⟩
def negativeInput3 := (negative.drop 15).take 8
def negativeD3 : SegEndData := ⟨.down,FeatureElem.ofStroke (negative[18]!),FeatureElem.ofStroke (negative[20]!),FeatureElem.ofStroke (negative[22]!),[],15000,12500⟩
def negativeInput4 := (negative.drop 20).take 8
def negativeD4 : SegEndData := ⟨.up,FeatureElem.ofStroke (negative[23]!),FeatureElem.ofStroke (negative[25]!),FeatureElem.ofStroke (negative[27]!),[],12500,18000⟩
def b := positive.take 5
def c := (positive.drop 20).take 5
def cn := (negative.drop 20).take 5
def s1 : Segment := ⟨.down,21,41,16000,12000⟩
def s2 : Segment := ⟨.up,41,61,12000,15000⟩
def s3 : Segment := ⟨.down,61,81,15000,12500⟩
def SelectedBy (d : SegEndData) (ss : List Stroke) : Prop :=
 match extractSegEndData d.dir d.startPrice d.endPrice ss with
 | none => False
 | some a => a.dir = d.dir ∧ a.startPrice = d.startPrice ∧ a.endPrice = d.endPrice ∧
     a.e1.low = d.e1.low ∧ a.e1.high = d.e1.high ∧ a.e2.low = d.e2.low ∧ a.e2.high = d.e2.high ∧ a.e3.low = d.e3.low ∧ a.e3.high = d.e3.high

def NoContainment (d : Direction) (ss : List Stroke) : Prop :=
 let es := (extractFeatureStrokes d ss).map FeatureElem.ofStroke
 ∀ i : Fin (es.length-1),
  let a := es[i.val]!
  let b := es[i.val+1]!
  ¬ Contains a b ∧ ¬ Contains b a

def StrokeSource (ps : List Int) (s : Stroke) : Prop :=
 s.startIndex > 0 ∧ s.endIndex + 1 < ps.length ∧ s.endIndex = s.startIndex + 4 ∧ s.WellFormed ∧
 s.startPrice = ps[s.startIndex]! ∧ s.endPrice = ps[s.endIndex]! ∧
 (match s.direction with
 | .up => ps[s.startIndex-1]! > s.startPrice ∧ ps[s.startIndex+1]! > s.startPrice ∧ ps[s.endIndex-1]! < s.endPrice ∧ ps[s.endIndex+1]! < s.endPrice ∧ s.startPrice < s.endPrice
 | .down => ps[s.startIndex-1]! < s.startPrice ∧ ps[s.startIndex+1]! < s.startPrice ∧ ps[s.endIndex-1]! > s.endPrice ∧ ps[s.endIndex+1]! > s.endPrice ∧ s.endPrice < s.startPrice)

def ATOMS : Prop := positive.length = 30 ∧ negative.length = 30 ∧ positivePrices.length = 123 ∧ negativePrices.length = 123 ∧
 (∀ i : Fin 30, StrokeSource positivePrices positive[i.val]!) ∧
 (∀ i : Fin 30, StrokeSource negativePrices negative[i.val]!)
def FEATURES : Prop := (SegEndComplete positiveD0 ∧ ¬ HasGap positiveD0.e1 positiveD0.e2 ∧ SelectedBy positiveD0 positiveInput0 ∧ NoContainment .up positiveInput0) ∧
(SegEndComplete positiveD1 ∧ ¬ HasGap positiveD1.e1 positiveD1.e2 ∧ SelectedBy positiveD1 positiveInput1 ∧ NoContainment .down positiveInput1) ∧
(SegEndComplete positiveD2 ∧ ¬ HasGap positiveD2.e1 positiveD2.e2 ∧ SelectedBy positiveD2 positiveInput2 ∧ NoContainment .up positiveInput2) ∧
(SegEndComplete positiveD3 ∧ ¬ HasGap positiveD3.e1 positiveD3.e2 ∧ SelectedBy positiveD3 positiveInput3 ∧ NoContainment .down positiveInput3) ∧
(SegEndComplete positiveD4 ∧ ¬ HasGap positiveD4.e1 positiveD4.e2 ∧ SelectedBy positiveD4 positiveInput4 ∧ NoContainment .up positiveInput4) ∧
(SegEndComplete negativeD0 ∧ ¬ HasGap negativeD0.e1 negativeD0.e2 ∧ SelectedBy negativeD0 negativeInput0 ∧ NoContainment .up negativeInput0) ∧
(SegEndComplete negativeD1 ∧ ¬ HasGap negativeD1.e1 negativeD1.e2 ∧ SelectedBy negativeD1 negativeInput1 ∧ NoContainment .down negativeInput1) ∧
(SegEndComplete negativeD2 ∧ ¬ HasGap negativeD2.e1 negativeD2.e2 ∧ SelectedBy negativeD2 negativeInput2 ∧ NoContainment .up negativeInput2) ∧
(SegEndComplete negativeD3 ∧ ¬ HasGap negativeD3.e1 negativeD3.e2 ∧ SelectedBy negativeD3 negativeInput3 ∧ NoContainment .down negativeInput3) ∧
(SegEndComplete negativeD4 ∧ ¬ HasGap negativeD4.e1 negativeD4.e2 ∧ SelectedBy negativeD4 negativeInput4 ∧ NoContainment .up negativeInput4)
def CORE : Prop := CenterConfirmedComplete s1 s2 s3 ∧ computeZD s1 s2 s3 = 12500 ∧ computeZG s1 s2 s3 = 15000
def FORCE : Prop := impulse b = 250 ∧ impulse c = -125 ∧ impulse cn = 250 ∧ IsImpulseDivergence b c ∧ ¬ IsImpulseDivergence b cn
end Stage60
