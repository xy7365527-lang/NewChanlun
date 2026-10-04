import {readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
const dir=fileURLToPath(new URL('.',import.meta.url));
const inputs=JSON.parse(readFileSync(dir+'controls-v1.json')).controls;
const results=JSON.parse(readFileSync(dir+'results-v1.json')).results;
const list=xs=>'['+xs.join(',')+']',bool=x=>x?'true':'false',opt=x=>x===null?'none':`some ${x}`;
let s=`import DirectOutputSourcePathProof
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

`;
for(let z=0;z<inputs.length;z++){
 const c=inputs[z],r=results[z],last=r.prefixes.at(-1),N=c.events.length,ns=z?'NoneControl':'Main';
 s+=`namespace ${ns}\n`;
 c.events.forEach((e,i)=>{s+=`def e${i+1} : Event := ⟨⟨${bool(e.side==='bid')},.${e.action},1⟩,${e.price}⟩\n`;});
 s+=`def es : List Event := ${list(c.events.map((_,i)=>'e'+(i+1)))}\n`;
 s+='def books : Nat → Book\n';last.states.forEach((b,i)=>{s+=`  | ${i} => ⟨${list(b.bids.map(o=>o.price))},${list(b.asks.map(o=>o.price))}⟩\n`;});s+=`  | _ => ⟨[],[]⟩\n`;
 s+=`def qs : List (Option Quote) := ${list(last.quotes.map(q=>q?`some ⟨${q.bid},${q.ask}⟩`:'none'))}\n`;
 c.events.forEach((e,i)=>{const st=last.steps[i];const upd=e.action==='add'?'rfl':`⟨${list(st.updateWitness.pre)},${list(st.updateWitness.post)},rfl,rfl⟩`;
 s+=`theorem step${i+1} : Step e${i+1} (books ${i}) (books ${i+1}) := by\n  exact ⟨by decide,${'⟨'+upd+',rfl⟩'}⟩\n`;});
 s+=`theorem through : BookQuote.Through es books es.length := by\n  intro i hi\n  have hc : ${Array.from({length:N},(_,i)=>'i = '+i).join(' ∨ ')} := by change i < ${N} at hi; omega\n  rcases hc with ${Array.from({length:N},()=> 'rfl').join(' | ')}\n`;
 for(let i=0;i<N;i++)s+=`  · exact ⟨e${i+1},rfl,step${i+1}⟩\n`;
 s+=`theorem reads : BookQuote.Reads books qs := by\n  intro i oq h\n  have hb : i < qs.length := (List.getElem?_eq_some_iff.mp h).1\n  have hc : ${Array.from({length:N+1},(_,i)=>'i = '+i).join(' ∨ ')} := by change i < ${N+1} at hb; omega\n  rcases hc with ${Array.from({length:N+1},()=> 'rfl').join(' | ')}\n`;
 last.quotes.forEach((q,i)=>{s+=`  · have ho : oq = ${q?`some (⟨${q.bid},${q.ask}⟩ : Quote)`:'none'} := Option.some.inj h.symm\n    subst oq\n`;
 s+=q?'    change QuoteOf _ _\n    decide\n':`    change ¬ ∃ q, QuoteOf (books ${i}) q\n    rintro ⟨q,hq⟩\n    simpa [QuoteOf,BestBid,books] using hq.1.1\n`;});
 s+=`def expected : Nat → List (Nat × Nat × Nat × Option Nat × Bool × List Event × Option (Bool × Nat × Nat × Int × Int))\n`;
 for(const p of r.prefixes){s+=`  | ${p.n} => ${list(p.outputs.map(o=>`(${o.start},${o.end},${o.firstKnown},${opt(o.knownAt)},${bool(o.pressure===1)},${list(o.eventIds.map(id=>'e'+(c.events.findIndex(e=>e.id===id)+1)))},${o.geometry?`some (${bool(o.pressure===1)},${o.start},${o.end},${o.geometry.startValue},${o.geometry.endValue})`:'none'})`))}\n`;}
 s+=`  | _ => []\ntheorem prefix_fields (n : Fin ${N+1}) :
    (construct (es.take n.val) (qs.take (n.val+1))).map (List.map view) = some (expected n.val) := by
  have hc : ${Array.from({length:N+1},(_,i)=>'n.val = '+i).join(' ∨ ')} := by have h := n.isLt; omega
  rcases hc with ${Array.from({length:N+1},(_,i)=> 'h'+i).join(' | ')}
`;
 for(let i=0;i<=N;i++)s+=`  · rw [h${i}]; rfl\n`;
 s+=`theorem prefix_source (n : Fin ${N+1}) :
    BookQuote.Through (es.take n.val) books (es.take n.val).length ∧
    BookQuote.Reads books (qs.take (n.val+1)) ∧
    (qs.take (n.val+1)).length = (es.take n.val).length+1 := by
  have hn : n.val ≤ es.length := by have h := n.isLt; change n.val ≤ ${N}; omega
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
  · have hq : n.val+1 ≤ qs.length := by have h := n.isLt; change n.val+1 ≤ ${N+1}; omega
    simp only [List.length_take, Nat.min_eq_left hn, Nat.min_eq_left hq]
end ${ns}
`;
}
s+=`def WITNESS_TARGET : Prop :=
  (∀ n : Fin 9, BookQuote.Through (Main.es.take n.val) Main.books (Main.es.take n.val).length ∧ BookQuote.Reads Main.books (Main.qs.take (n.val+1)) ∧ (Main.qs.take (n.val+1)).length = (Main.es.take n.val).length+1 ∧ (construct (Main.es.take n.val) (Main.qs.take (n.val+1))).map (List.map view) = some (Main.expected n.val)) ∧
  (∀ n : Fin 8, BookQuote.Through (NoneControl.es.take n.val) NoneControl.books (NoneControl.es.take n.val).length ∧ BookQuote.Reads NoneControl.books (NoneControl.qs.take (n.val+1)) ∧ (NoneControl.qs.take (n.val+1)).length = (NoneControl.es.take n.val).length+1 ∧ (construct (NoneControl.es.take n.val) (NoneControl.qs.take (n.val+1))).map (List.map view) = some (NoneControl.expected n.val))

theorem witness_root : WITNESS_TARGET := by
  constructor
  · intro n; exact ⟨(Main.prefix_source n).1,(Main.prefix_source n).2.1,(Main.prefix_source n).2.2,Main.prefix_fields n⟩
  · intro n; exact ⟨(NoneControl.prefix_source n).1,(NoneControl.prefix_source n).2.1,(NoneControl.prefix_source n).2.2,NoneControl.prefix_fields n⟩
end Stage57Direct
#print axioms Stage57Direct.primitive_strict
#print axioms Stage57Direct.witness_root
`;
writeFileSync(dir+'DirectLifecycle.lean',s);
