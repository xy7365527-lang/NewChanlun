import DirectOutputSourcePathProof
namespace Stage57Direct
open MovingQuote NewChanlun.Origin
set_option maxRecDepth 4096
set_option maxHeartbeats 2000000

def view (x : Output) : Nat × Nat × Nat × Option Nat × Bool × List Event × Option (Bool × Nat × Nat × Int × Int) :=
  (x.located.start,x.located.finish,x.located.firstKnown,x.located.knownAt,positive x.located.block.first,events x.located.block,
    x.geometry.map (fun g => (g.direction == .up,g.startIndex,g.endIndex,g.startPrice,g.endPrice)))

/-- 每份 actual some 几何的严格方向与真实来源跨度；不另证旧包络。 -/
theorem primitive_strict {es bs qs out} (ht : BookQuote.Through es bs es.length)
    (hr : BookQuote.Reads bs qs) (hlen : qs.length = es.length+1)
    (hc : construct es qs = some out) {x} (hx : x ∈ out) {g} (hg : x.geometry = some g) :
    g.startIndex = x.located.start ∧ g.endIndex = x.located.finish ∧
    g.startIndex < g.endIndex ∧
    (if positive x.located.block.first then g.direction = .up ∧ g.startPrice < g.endPrice
     else g.direction = .down ∧ g.endPrice < g.startPrice) := by
  rcases DirectOutput.hull_contract es bs qs out ht hr hlen hc x hx g hg with
    ⟨q,r,hq,hrq,hqb,hrb,hf,hg',_⟩
  have hp := MovingQuote.leg_contract.1 _ _ _ _ _ _ hf (by simp [events]) x.located.start
  have hd := DirectOutput.member_data hc hx
  rcases DirectOutput.source hd.1 with ⟨pre,post,hsrc,hs,he,hu⟩
  rw [hg']
  cases hpos : positive x.located.block.first <;> simp only [hpos,leg, Bool.false_eq_true, ↓reduceIte] at hp ⊢
  all_goals exact ⟨True.intro,he.symm,hp.1,True.intro,hp.2⟩

namespace Main
def e1 : Event := ⟨⟨true,.add,1⟩,101⟩
def e2 : Event := ⟨⟨true,.cancel,1⟩,101⟩
def e3 : Event := ⟨⟨true,.add,1⟩,101⟩
def e4 : Event := ⟨⟨true,.cancel,1⟩,101⟩
def e5 : Event := ⟨⟨true,.cancel,1⟩,100⟩
def e6 : Event := ⟨⟨false,.add,1⟩,99⟩
def e7 : Event := ⟨⟨true,.add,1⟩,2⟩
def e8 : Event := ⟨⟨true,.cancel,1⟩,2⟩
def es : List Event := [e1,e2,e3,e4,e5,e6,e7,e8]
def books : Nat → Book
  | 0 => ⟨[100,1],[102]⟩
  | 1 => ⟨[101,100,1],[102]⟩
  | 2 => ⟨[100,1],[102]⟩
  | 3 => ⟨[101,100,1],[102]⟩
  | 4 => ⟨[100,1],[102]⟩
  | 5 => ⟨[1],[102]⟩
  | 6 => ⟨[1],[99,102]⟩
  | 7 => ⟨[2,1],[99,102]⟩
  | 8 => ⟨[1],[99,102]⟩
  | _ => ⟨[],[]⟩
def qs : List (Option Quote) := [some ⟨100,102⟩,some ⟨101,102⟩,some ⟨100,102⟩,some ⟨101,102⟩,some ⟨100,102⟩,some ⟨1,102⟩,some ⟨1,99⟩,some ⟨2,99⟩,some ⟨1,99⟩]
theorem step1 : Step e1 (books 0) (books 1) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step2 : Step e2 (books 1) (books 2) := by
  exact ⟨by decide,⟨⟨[],[100,1],rfl,rfl⟩,rfl⟩⟩
theorem step3 : Step e3 (books 2) (books 3) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step4 : Step e4 (books 3) (books 4) := by
  exact ⟨by decide,⟨⟨[],[100,1],rfl,rfl⟩,rfl⟩⟩
theorem step5 : Step e5 (books 4) (books 5) := by
  exact ⟨by decide,⟨⟨[],[1],rfl,rfl⟩,rfl⟩⟩
theorem step6 : Step e6 (books 5) (books 6) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step7 : Step e7 (books 6) (books 7) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step8 : Step e8 (books 7) (books 8) := by
  exact ⟨by decide,⟨⟨[],[1],rfl,rfl⟩,rfl⟩⟩
theorem through : BookQuote.Through es books es.length := by
  intro i hi
  have hc : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 ∨ i = 7 := by change i < 8 at hi; omega
  rcases hc with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
  · exact ⟨e1,rfl,step1⟩
  · exact ⟨e2,rfl,step2⟩
  · exact ⟨e3,rfl,step3⟩
  · exact ⟨e4,rfl,step4⟩
  · exact ⟨e5,rfl,step5⟩
  · exact ⟨e6,rfl,step6⟩
  · exact ⟨e7,rfl,step7⟩
  · exact ⟨e8,rfl,step8⟩
theorem reads : BookQuote.Reads books qs := by
  intro i oq h
  have hb : i < qs.length := (List.getElem?_eq_some_iff.mp h).1
  have hc : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 ∨ i = 7 ∨ i = 8 := by change i < 9 at hb; omega
  rcases hc with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
  · have ho : oq = some (⟨100,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨101,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨100,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨101,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨100,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨1,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨1,99⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨2,99⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨1,99⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
def expected : Nat → List (Nat × Nat × Nat × Option Nat × Bool × List Event × Option (Bool × Nat × Nat × Int × Int))
  | 0 => []
  | 1 => [(0,1,1,none,true,[e1],some (true,0,1,100,102))]
  | 2 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,none,false,[e2],some (false,1,2,102,100))]
  | 3 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,some 3,false,[e2],some (false,1,2,102,100)),(2,3,3,none,true,[e3],some (true,2,3,100,102))]
  | 4 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,some 3,false,[e2],some (false,1,2,102,100)),(2,3,3,some 4,true,[e3],some (true,2,3,100,102)),(3,4,4,none,false,[e4],some (false,3,4,102,100))]
  | 5 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,some 3,false,[e2],some (false,1,2,102,100)),(2,3,3,some 4,true,[e3],some (true,2,3,100,102)),(3,5,4,none,false,[e4,e5],some (false,3,5,102,1))]
  | 6 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,some 3,false,[e2],some (false,1,2,102,100)),(2,3,3,some 4,true,[e3],some (true,2,3,100,102)),(3,6,4,none,false,[e4,e5,e6],some (false,3,6,102,1))]
  | 7 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,some 3,false,[e2],some (false,1,2,102,100)),(2,3,3,some 4,true,[e3],some (true,2,3,100,102)),(3,6,4,some 7,false,[e4,e5,e6],some (false,3,6,102,1)),(6,7,7,none,true,[e7],some (true,6,7,1,99))]
  | 8 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,some 3,false,[e2],some (false,1,2,102,100)),(2,3,3,some 4,true,[e3],some (true,2,3,100,102)),(3,6,4,some 7,false,[e4,e5,e6],some (false,3,6,102,1)),(6,7,7,some 8,true,[e7],some (true,6,7,1,99)),(7,8,8,none,false,[e8],some (false,7,8,99,1))]
  | _ => []
theorem prefix_fields (n : Fin 9) :
    (construct (es.take n.val) (qs.take (n.val+1))).map (List.map view) = some (expected n.val) := by
  have hc : n.val = 0 ∨ n.val = 1 ∨ n.val = 2 ∨ n.val = 3 ∨ n.val = 4 ∨ n.val = 5 ∨ n.val = 6 ∨ n.val = 7 ∨ n.val = 8 := by have h := n.isLt; omega
  rcases hc with h0 | h1 | h2 | h3 | h4 | h5 | h6 | h7 | h8
  · rw [h0]; rfl
  · rw [h1]; rfl
  · rw [h2]; rfl
  · rw [h3]; rfl
  · rw [h4]; rfl
  · rw [h5]; rfl
  · rw [h6]; rfl
  · rw [h7]; rfl
  · rw [h8]; rfl
theorem prefix_source (n : Fin 9) :
    BookQuote.Through (es.take n.val) books (es.take n.val).length ∧
    BookQuote.Reads books (qs.take (n.val+1)) ∧
    (qs.take (n.val+1)).length = (es.take n.val).length+1 := by
  have hn : n.val ≤ es.length := by have h := n.isLt; change n.val ≤ 8; omega
  refine ⟨?_,?_,?_⟩
  · intro i hi
    have hi' : i < n.val := Nat.lt_of_lt_of_le hi (List.length_take_le _ _)
    have hi'' : i < es.length := Nat.lt_of_lt_of_le hi' hn
    rcases through i hi'' with ⟨e,he,hs⟩
    exact ⟨e,by rw [List.getElem?_take_of_lt hi']; exact he,hs⟩
  · intro i q h
    have hi : i < (qs.take (n.val+1)).length := (List.getElem?_eq_some_iff.mp h).1
    have hi' : i < n.val+1 := Nat.lt_of_lt_of_le hi (List.length_take_le _ _)
    exact reads i q (by rw [List.getElem?_take_of_lt hi'] at h; exact h)
  · have hq : n.val+1 ≤ qs.length := by have h := n.isLt; change n.val+1 ≤ 9; omega
    simp only [List.length_take, Nat.min_eq_left hn, Nat.min_eq_left hq]
end Main
namespace NoneControl
def e1 : Event := ⟨⟨true,.add,1⟩,101⟩
def e2 : Event := ⟨⟨true,.cancel,1⟩,101⟩
def e3 : Event := ⟨⟨true,.cancel,1⟩,100⟩
def e4 : Event := ⟨⟨true,.add,1⟩,2⟩
def e5 : Event := ⟨⟨true,.add,1⟩,3⟩
def e6 : Event := ⟨⟨true,.cancel,1⟩,3⟩
def e7 : Event := ⟨⟨true,.add,1⟩,4⟩
def es : List Event := [e1,e2,e3,e4,e5,e6,e7]
def books : Nat → Book
  | 0 => ⟨[100],[102]⟩
  | 1 => ⟨[101,100],[102]⟩
  | 2 => ⟨[100],[102]⟩
  | 3 => ⟨[],[102]⟩
  | 4 => ⟨[2],[102]⟩
  | 5 => ⟨[3,2],[102]⟩
  | 6 => ⟨[2],[102]⟩
  | 7 => ⟨[4,2],[102]⟩
  | _ => ⟨[],[]⟩
def qs : List (Option Quote) := [some ⟨100,102⟩,some ⟨101,102⟩,some ⟨100,102⟩,none,some ⟨2,102⟩,some ⟨3,102⟩,some ⟨2,102⟩,some ⟨4,102⟩]
theorem step1 : Step e1 (books 0) (books 1) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step2 : Step e2 (books 1) (books 2) := by
  exact ⟨by decide,⟨⟨[],[100],rfl,rfl⟩,rfl⟩⟩
theorem step3 : Step e3 (books 2) (books 3) := by
  exact ⟨by decide,⟨⟨[],[],rfl,rfl⟩,rfl⟩⟩
theorem step4 : Step e4 (books 3) (books 4) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step5 : Step e5 (books 4) (books 5) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem step6 : Step e6 (books 5) (books 6) := by
  exact ⟨by decide,⟨⟨[],[2],rfl,rfl⟩,rfl⟩⟩
theorem step7 : Step e7 (books 6) (books 7) := by
  exact ⟨by decide,⟨rfl,rfl⟩⟩
theorem through : BookQuote.Through es books es.length := by
  intro i hi
  have hc : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 := by change i < 7 at hi; omega
  rcases hc with rfl | rfl | rfl | rfl | rfl | rfl | rfl
  · exact ⟨e1,rfl,step1⟩
  · exact ⟨e2,rfl,step2⟩
  · exact ⟨e3,rfl,step3⟩
  · exact ⟨e4,rfl,step4⟩
  · exact ⟨e5,rfl,step5⟩
  · exact ⟨e6,rfl,step6⟩
  · exact ⟨e7,rfl,step7⟩
theorem reads : BookQuote.Reads books qs := by
  intro i oq h
  have hb : i < qs.length := (List.getElem?_eq_some_iff.mp h).1
  have hc : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 ∨ i = 7 := by change i < 8 at hb; omega
  rcases hc with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
  · have ho : oq = some (⟨100,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨101,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨100,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = none := Option.some.inj h.symm
    subst oq
    change ¬ ∃ q, QuoteOf (books 3) q
    rintro ⟨q,hq⟩
    simpa [QuoteOf,BestBid,books] using hq.1.1
  · have ho : oq = some (⟨2,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨3,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨2,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
  · have ho : oq = some (⟨4,102⟩ : Quote) := Option.some.inj h.symm
    subst oq
    change QuoteOf _ _
    decide
def expected : Nat → List (Nat × Nat × Nat × Option Nat × Bool × List Event × Option (Bool × Nat × Nat × Int × Int))
  | 0 => []
  | 1 => [(0,1,1,none,true,[e1],some (true,0,1,100,102))]
  | 2 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,2,2,none,false,[e2],some (false,1,2,102,100))]
  | 3 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,3,2,none,false,[e2,e3],none)]
  | 4 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,3,2,some 4,false,[e2,e3],none),(3,4,4,none,true,[e4],none)]
  | 5 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,3,2,some 4,false,[e2,e3],none),(3,5,4,none,true,[e4,e5],none)]
  | 6 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,3,2,some 4,false,[e2,e3],none),(3,5,4,some 6,true,[e4,e5],none),(5,6,6,none,false,[e6],some (false,5,6,102,2))]
  | 7 => [(0,1,1,some 2,true,[e1],some (true,0,1,100,102)),(1,3,2,some 4,false,[e2,e3],none),(3,5,4,some 6,true,[e4,e5],none),(5,6,6,some 7,false,[e6],some (false,5,6,102,2)),(6,7,7,none,true,[e7],some (true,6,7,2,102))]
  | _ => []
theorem prefix_fields (n : Fin 8) :
    (construct (es.take n.val) (qs.take (n.val+1))).map (List.map view) = some (expected n.val) := by
  have hc : n.val = 0 ∨ n.val = 1 ∨ n.val = 2 ∨ n.val = 3 ∨ n.val = 4 ∨ n.val = 5 ∨ n.val = 6 ∨ n.val = 7 := by have h := n.isLt; omega
  rcases hc with h0 | h1 | h2 | h3 | h4 | h5 | h6 | h7
  · rw [h0]; rfl
  · rw [h1]; rfl
  · rw [h2]; rfl
  · rw [h3]; rfl
  · rw [h4]; rfl
  · rw [h5]; rfl
  · rw [h6]; rfl
  · rw [h7]; rfl
theorem prefix_source (n : Fin 8) :
    BookQuote.Through (es.take n.val) books (es.take n.val).length ∧
    BookQuote.Reads books (qs.take (n.val+1)) ∧
    (qs.take (n.val+1)).length = (es.take n.val).length+1 := by
  have hn : n.val ≤ es.length := by have h := n.isLt; change n.val ≤ 7; omega
  refine ⟨?_,?_,?_⟩
  · intro i hi
    have hi' : i < n.val := Nat.lt_of_lt_of_le hi (List.length_take_le _ _)
    have hi'' : i < es.length := Nat.lt_of_lt_of_le hi' hn
    rcases through i hi'' with ⟨e,he,hs⟩
    exact ⟨e,by rw [List.getElem?_take_of_lt hi']; exact he,hs⟩
  · intro i q h
    have hi : i < (qs.take (n.val+1)).length := (List.getElem?_eq_some_iff.mp h).1
    have hi' : i < n.val+1 := Nat.lt_of_lt_of_le hi (List.length_take_le _ _)
    exact reads i q (by rw [List.getElem?_take_of_lt hi'] at h; exact h)
  · have hq : n.val+1 ≤ qs.length := by have h := n.isLt; change n.val+1 ≤ 8; omega
    simp only [List.length_take, Nat.min_eq_left hn, Nat.min_eq_left hq]
end NoneControl
def WITNESS_TARGET : Prop :=
  (∀ n : Fin 9, BookQuote.Through (Main.es.take n.val) Main.books (Main.es.take n.val).length ∧ BookQuote.Reads Main.books (Main.qs.take (n.val+1)) ∧ (Main.qs.take (n.val+1)).length = (Main.es.take n.val).length+1 ∧ (construct (Main.es.take n.val) (Main.qs.take (n.val+1))).map (List.map view) = some (Main.expected n.val)) ∧
  (∀ n : Fin 8, BookQuote.Through (NoneControl.es.take n.val) NoneControl.books (NoneControl.es.take n.val).length ∧ BookQuote.Reads NoneControl.books (NoneControl.qs.take (n.val+1)) ∧ (NoneControl.qs.take (n.val+1)).length = (NoneControl.es.take n.val).length+1 ∧ (construct (NoneControl.es.take n.val) (NoneControl.qs.take (n.val+1))).map (List.map view) = some (NoneControl.expected n.val))

theorem witness_root : WITNESS_TARGET := by
  constructor
  · intro n; exact ⟨(Main.prefix_source n).1,(Main.prefix_source n).2.1,(Main.prefix_source n).2.2,Main.prefix_fields n⟩
  · intro n; exact ⟨(NoneControl.prefix_source n).1,(NoneControl.prefix_source n).2.1,(NoneControl.prefix_source n).2.2,NoneControl.prefix_fields n⟩
end Stage57Direct
#print axioms Stage57Direct.primitive_strict
#print axioms Stage57Direct.witness_root
