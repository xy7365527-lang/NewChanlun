import DirectReachFixture
import ExitSupportProof

namespace DirectReach

open MovingQuote
open NewChanlun.Origin

inductive Steps : List Book → List Event → Prop where
  | nil (b : Book) : Steps [b] []
  | cons {a b : Book} {bs : List Book} {e : Event} {es : List Event} :
      Step e a b → Steps (b::bs) es → Steps (a::b::bs) (e::es)

def state (i : Fin 39) : Book :=
  DirectReachFixture.books[i.val]'(by simpa [DirectReachFixture.books] using i.isLt)
def quote (i : Fin 39) : Quote :=
  DirectReachFixture.quotes[i.val]'(by simpa [DirectReachFixture.quotes] using i.isLt)
def seg (i : Fin 8) : Segment :=
  DirectReachFixture.expectedLegs[i.val]'(by simpa [DirectReachFixture.expectedLegs] using i.isLt)
def cuts : List (Fin 39) := [0,6,10,12,21,23,30,37,38]
def start (i : Fin 8) : Fin 39 := cuts[i.val]'(by have h:=i.isLt; simp only [cuts,List.length_cons,List.length_nil]; omega)
def finish (i : Fin 8) : Fin 39 := cuts[i.val+1]'(by have h:=i.isLt; simp only [cuts,List.length_cons,List.length_nil]; omega)
def blockEvents (i : Fin 8) : List Event :=
  (DirectReachFixture.events.drop (start i).val).take ((finish i).val-(start i).val)
def sign (i : Fin 8) : Bool := decide (i.val % 2 = 1)
def located := locate 0 (group DirectReachFixture.events)
def done := located.filter (fun l => l.knownAt.isSome)

def eventAt (i : Nat) : Event :=
  DirectReachFixture.events[i]?.getD ⟨⟨true,.add,1⟩,10020⟩

def TRAJECTORY_TARGET : Prop :=
  Steps DirectReachFixture.books DirectReachFixture.events ∧
  DirectReachFixture.events.length = 38 ∧ DirectReachFixture.books.length = 39 ∧
  (∀ i : Fin 39, QuoteOf (state i) (quote i) ∧ ValidBook (state i)) ∧
  (∀ i : Fin 8, Flow (sign i) (state (start i)) (quote (start i))
    (state (finish i)) (quote (finish i)) (blockEvents i))

def GROUP_TARGET : Prop :=
  Runs DirectReachFixture.events (group DirectReachFixture.events) ∧
  (∀ bs, Runs DirectReachFixture.events bs → bs = group DirectReachFixture.events) ∧
  decode (group DirectReachFixture.events) = DirectReachFixture.events ∧
  (group DirectReachFixture.events).map (fun b => (events b).length) = [6,4,2,9,2,7,7,1] ∧
  located.map (·.knownAt) = [some 7,some 11,some 13,some 22,some 24,some 31,some 38,none] ∧
  done.length = 7 ∧
  (∀ i : Fin 7, let j : Fin 8 := ⟨i.val,by have h:=i.isLt; omega⟩
    Completed eventAt ((finish j).val+1) (start j).val (finish j).val ∧
    ¬ Completed eventAt (finish j).val (start j).val (finish j).val)

def segView (s : Segment) := (s.direction,s.startIndex,s.endIndex,s.startPrice,s.endPrice)
def outputView := (construct DirectReachFixture.events (DirectReachFixture.quotes.map some)).map
  (fun out => out.map (fun o => o.geometry.map segView))

def firstLocal : CenterConfirmedComplete (seg 0) (seg 1) (seg 2) := by
  unfold CenterConfirmedComplete DirAlternates
  decide
def nextLocal : CenterConfirmedComplete (seg 4) (seg 5) (seg 6) := by
  unfold CenterConfirmedComplete DirAlternates
  decide
def center3 (a b c : Segment) (h : CenterConfirmedComplete a b c) : Formal.CenterTrichotomy.Center :=
  let z := centerFullOfConfirmed a b c h
  { dd:=z.dd,zd:=z.core.zd,zg:=z.core.zg,gg:=z.gg,
    core_valid:=h.2,outer_lo:=z.outer_lo,outer_hi:=z.outer_hi }
def firstCenter := center3 (seg 0) (seg 1) (seg 2) firstLocal
def nextCenter := center3 (seg 4) (seg 5) (seg 6) nextLocal

def GEOMETRY_TARGET : Prop :=
  outputView = some (DirectReachFixture.expectedLegs.map (fun s => some (segView s))) ∧
  Formal.CenterTrichotomy.SameLevelNewCenterPair firstCenter nextCenter ∧
  Formal.CenterTrichotomy.IsLevelExpansion firstCenter nextCenter ∧
  (firstCenter.dd,firstCenter.zd,firstCenter.zg,firstCenter.gg) = (10020,10024,10032,10040) ∧
  (nextCenter.dd,nextCenter.zd,nextCenter.zg,nextCenter.gg) = (10036,10042,10050,10060)

def OBSTRUCTION_TARGET : Prop :=
  (¬ ∃ a b c, NineRegroup.SourceTriplet done a b c) ∧ ExitSupport.GROW_TARGET ∧
  (blockEvents 3).length = 9 ∧ (start 3).val = 12 ∧ (finish 3).val = 21

end DirectReach
