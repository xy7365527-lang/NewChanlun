import DirectOutputSpec

namespace DirectOutput
open MovingQuote NewChanlun.Origin

def render (qs : List (Option Quote)) (l : Located) : Output :=
  ⟨l,readout (positive l.block.first) l.start
    ((qs.drop l.start).take ((events l.block).length+1))⟩

theorem output_shape {es qs out} (h : construct es qs = some out) :
    out = (locate 0 (group es)).map (render qs) := by
  unfold construct at h
  split at h
  · exact (Option.some.inj h).symm
  · cases h

theorem member_data {es qs out x} (h : construct es qs = some out) (hx : x ∈ out) :
    x.located ∈ locate 0 (group es) ∧ x = render qs x.located := by
  rw [output_shape h] at hx
  rcases List.mem_map.mp hx with ⟨l,hl,he⟩
  subst x
  exact ⟨hl,rfl⟩

theorem input_eventAt (es : List Event) : LocatedProof.input (eventAt es) es.length = es := by
  apply List.ext_getElem?; intro i
  by_cases hi : i < es.length
  · rw [LocatedProof.input_at _ _ _ hi]
    simp [eventAt,List.getElem?_eq_getElem hi]
  · have hn : es.length ≤ i := by omega
    simp [List.getElem?_eq_none hn, List.getElem?_eq_none (show
      (LocatedProof.input (eventAt es) es.length).length ≤ i by
        simpa [LocatedProof.input_length] using hn)]

theorem good {es l} (hl : l ∈ locate 0 (group es)) :
    LocatedProof.Good (eventAt es) es.length l := by
  apply LocatedProof.program.1
  simpa [LocatedProof.output,input_eventAt] using hl

theorem locate_source : ∀ (blocks : List Block) (pre : List Event) (l : Located),
    l ∈ locate pre.length blocks →
    ∃ a b, pre ++ decode blocks = a ++ events l.block ++ b ∧
      a.length = l.start ∧ l.finish = l.start+(events l.block).length ∧ l.block ∈ blocks
  | [],pre,l,hl => by simp [locate] at hl
  | b::bs,pre,l,hl => by
    change l ∈ LocatedProof.placed pre.length b bs :: locate (pre.length+(events b).length) bs at hl
    rcases List.mem_cons.mp hl with he | hl
    · subst l
      exact ⟨pre,decode bs,by simp [decode,List.append_assoc,LocatedProof.placed],rfl,rfl,by simp [LocatedProof.placed]⟩
    · have hl' : l ∈ locate (pre ++ events b).length bs := by simpa using hl
      rcases locate_source bs (pre ++ events b) l hl' with ⟨a,c,he,hs,hf,hb⟩
      exact ⟨a,c,by simpa [decode,List.append_assoc] using he,hs,hf,List.mem_cons_of_mem _ hb⟩

theorem source {es l} (hl : l ∈ locate 0 (group es)) : Source es l := by
  rcases locate_source (group es) [] l hl with ⟨a,b,he,hs,hf,hb⟩
  have hp := runs_properties (group_sound es)
  exact ⟨a,b,by simpa [hp.1] using he,hs,hf,hp.2.1 _ hb⟩

theorem completed_iff {es l} (hl : l ∈ locate 0 (group es)) :
    Completed (eventAt es) es.length l.start l.finish ↔ l.knownAt = some (l.finish+1) := by
  have hg := good hl
  constructor
  · intro hc
    cases hk : l.knownAt with
    | none => have hn := hg.2.2.2.1.mp hk; have ht := hc.2.1; omega
    | some k => rw [hg.2.2.2.2 k hk]
  · intro hk
    apply LocatedProof.completed_of_span hg.1
    have hn : l.finish ≠ es.length := by
      intro he
      have hz := hg.2.2.2.1.mpr he
      rw [hk] at hz
      cases hz
    have hle := hg.1.2.1
    omega

theorem source_contract : SOURCE_TARGET := by
  intro es qs out hc
  have hmap : out.map Output.located = locate 0 (group es) := by
    rw [output_shape hc]
    simp [List.map_map,render,Function.comp_def]
  refine ⟨hmap,?_,?_⟩
  · have hblocks := congrArg (List.map Located.block) hmap
    simp only [List.map_map,Function.comp_def,locate_blocks] at hblocks
    rw [hblocks]
    exact (runs_properties (group_sound es)).1
  · intro x hx
    have hm := (member_data hc hx).1
    exact ⟨source hm,good hm,completed_iff hm⟩

theorem locate_lower : ∀ (bs : List Block) start l, l ∈ locate start bs → start ≤ l.start
  | [],start,l,hl => by simp [locate] at hl
  | b::bs,start,l,hl => by
    change l ∈ LocatedProof.placed start b bs :: locate (start+(events b).length) bs at hl
    rcases List.mem_cons.mp hl with he | hl
    · subst l; exact Nat.le_refl _
    · have h := locate_lower bs _ l hl
      omega

theorem locate_start_unique : ∀ (bs : List Block) start l m,
    l ∈ locate start bs → m ∈ locate start bs → l.start = m.start → l = m
  | [],start,l,m,hl,hm,he => by simp [locate] at hl
  | b::bs,start,l,m,hl,hm,he => by
    change l ∈ LocatedProof.placed start b bs :: locate (start+(events b).length) bs at hl
    change m ∈ LocatedProof.placed start b bs :: locate (start+(events b).length) bs at hm
    have hp : 0 < (events b).length := by simp [events]
    rcases List.mem_cons.mp hl with hl | hl <;> rcases List.mem_cons.mp hm with hm | hm
    · exact hl.trans hm.symm
    · subst l
      have hb := locate_lower bs _ m hm
      change start = m.start at he
      omega
    · subst m
      have hb := locate_lower bs _ l hl
      change l.start = start at he
      omega
    · exact locate_start_unique bs _ l m hl hm he

theorem cover_contract : COVER_TARGET := by
  intro es qs out hc i hi
  have hdec := (runs_properties (group_sound es)).1
  rcases LocatedProof.covers (group es) 0 i (by omega) (by simpa [hdec] using hi) with
    ⟨l,hl,hs,he⟩
  let x := render qs l
  have hx : x ∈ out := by rw [output_shape hc]; exact List.mem_map.mpr ⟨l,hl,rfl⟩
  refine ⟨x,hx,hs,he,?_⟩
  intro y hy hys hye
  have hd := member_data hc hy
  have hu := LocatedProof.span_unique (good hd.1).1 (good hl).1 ⟨hys,hye⟩ ⟨hs,he⟩
  have heq := locate_start_unique (group es) 0 y.located l hd.1 hl hu.1
  rw [hd.2,heq]

theorem readout_some {pos start qs s} (h : readout pos start qs = some s) :
    qs.all quoteReady = true ∧ ∃ q r, qs.head? = some (some q) ∧
      qs.getLast? = some (some r) ∧ s = leg pos q r start (qs.length-1) := by
  unfold readout at h
  split at h
  · cases h
  · split at h
    · cases h
    · rename_i hall
      have ha : qs.all quoteReady = true := by
        cases hh : qs.all quoteReady <;> simp_all
      refine ⟨ha,?_⟩
      split at h
      · rename_i q r hq hr
        by_cases hlt : (if pos then q.bid < r.ask else r.bid < q.ask)
        · rw [if_pos hlt] at h
          exact ⟨q,r,hq,hr,(Option.some.inj h).symm⟩
        · rw [if_neg hlt] at h
          cases h
      · cases h

def quoteAt (qs : List (Option Quote)) (i : Nat) : Quote := (qs[i]?.join).getD q0

theorem quoteAt_eq {qs i q} (h : qs[i]? = some (some q)) : quoteAt qs i = q := by
  simp [quoteAt,h]

theorem flow_indexed (evs : List Event) (pos : Bool) (books : Nat → Book) (quotes : Nat → Quote)
    (hs : ∀ i e, evs[i]? = some e → Step e (books i) (books (i+1)))
    (hq : ∀ i, i ≤ evs.length → QuoteOf (books i) (quotes i))
    (hp : ∀ e ∈ evs, positive e = pos) :
    Flow pos (books 0) (quotes 0) (books evs.length) (quotes evs.length) evs := by
  induction evs generalizing books quotes with
  | nil => exact Flow.nil _ _ (hq 0 (by simp))
  | cons e es ih =>
    have hr : Flow pos (books 1) (quotes 1) (books (es.length+1)) (quotes (es.length+1)) es := by
      apply ih (fun i => books (i+1)) (fun i => quotes (i+1))
      · intro i f hf
        exact hs (i+1) f (by simpa using hf)
      · intro i hi
        exact hq (i+1) (by simp; omega)
      · intro f hf
        exact hp f (List.mem_cons_of_mem _ hf)
    exact Flow.step e _ _ _ _ _ _ es (hs 0 e rfl) (hq 0 (by simp))
      (hq 1 (by simp)) (hp e (by simp)) hr

theorem owned_lookup {pre mid post : List Event} {i : Nat} (hi : i < mid.length) :
    (pre ++ mid ++ post)[pre.length+i]? = mid[i]? := by
  rw [List.append_assoc,List.getElem?_append_right (by omega)]
  simp only [Nat.add_sub_cancel_left,List.getElem?_append_left hi]

theorem hull_contract : HULL_TARGET := by
  intro es bs qs out ht hr hlen hc x hx s hg
  have hd := member_data hc hx
  rcases source hd.1 with ⟨pre,post,hsrc,hstart,hfinish,hu⟩
  have hn := congrArg List.length hsrc
  simp only [List.length_append] at hn
  let window := (qs.drop x.located.start).take ((events x.located.block).length+1)
  have hwlen : window.length = (events x.located.block).length+1 := by
    simp only [window,List.length_take,List.length_drop]
    apply Nat.min_eq_left
    omega
  have hwget : ∀ i, i ≤ (events x.located.block).length →
      window[i]? = qs[x.located.start+i]? := by
    intro i hi
    rw [List.getElem?_take_of_lt (by omega),List.getElem?_drop]
  have hread : readout (positive x.located.block.first) x.located.start window = some s := by
    have hh := congrArg Output.geometry hd.2
    exact hh.symm.trans hg
  rcases readout_some hread with ⟨hall,q,r,hq0,hrlast,hseg⟩
  have hq : qs[x.located.start]? = some (some q) := by
    rw [List.head?_eq_getElem?,hwget 0 (by omega)] at hq0
    simpa using hq0
  have hrq : qs[x.located.finish]? = some (some r) := by
    rw [List.getLast?_eq_getElem?,hwlen] at hrlast
    simp only [Nat.add_sub_cancel] at hrlast
    rw [hwget _ (Nat.le_refl _)] at hrlast
    simpa only [hfinish] using hrlast
  have hfull : ∀ i, i ≤ (events x.located.block).length →
      QuoteOf (bs (x.located.start+i)) (quoteAt qs (x.located.start+i)) := by
    intro i hi
    have hib : i < window.length := by omega
    have hw := List.getElem?_eq_getElem hib
    have ready := List.all_eq_true.mp hall window[i] (List.getElem_mem hib)
    cases hwi : window[i] with
    | none => simp [hwi,quoteReady] at ready
    | some v =>
      rw [hwi,hwget i hi] at hw
      have hv : QuoteOf (bs (x.located.start+i)) v := hr _ _ hw
      simpa only [quoteAt_eq hw] using hv
  have hsteps : ∀ i e, (events x.located.block)[i]? = some e →
      Step e (bs (x.located.start+i)) (bs (x.located.start+(i+1))) := by
    intro i e hei
    have hib : i < (events x.located.block).length := (List.getElem?_eq_some_iff.mp hei).1
    have hei' : es[x.located.start+i]? = some e := by
      rw [hsrc,← hstart,owned_lookup hib]
      exact hei
    rcases ht (x.located.start+i) (by omega) with ⟨e',he',hst⟩
    have he : e' = e := Option.some.inj (he'.symm.trans hei')
    subst e'
    simpa only [Nat.add_assoc] using hst
  have hf := flow_indexed (events x.located.block) (positive x.located.block.first)
    (fun i => bs (x.located.start+i)) (fun i => quoteAt qs (x.located.start+i)) hsteps hfull hu
  have hflow : Flow (positive x.located.block.first) (bs x.located.start) q
      (bs x.located.finish) r (events x.located.block) := by
    simpa only [Nat.add_zero,← hfinish,quoteAt_eq hq,quoteAt_eq hrq] using hf
  have hs : s = leg (positive x.located.block.first) q r x.located.start
      (events x.located.block).length := by simpa only [hwlen,Nat.add_sub_cancel] using hseg
  refine ⟨q,r,hq,hrq,(flow_quotes hflow).1,(flow_quotes hflow).2,hflow,hs,?_⟩
  rw [hs]
  exact DirectHull.hull hflow x.located.start

theorem locate_adjacent : ∀ (blocks : List Block) (start : Nat), Separated blocks →
    ∀ pre x y post, locate start blocks = pre ++ x::y::post →
      x.finish = y.start ∧ positive y.block.first = !(positive x.block.first)
  | [],start,hs,pre,x,y,post,he => by
    have hh := congrArg List.length he
    simp [locate] at hh
  | b::bs,start,hs,pre,x,y,post,he => by
    change LocatedProof.placed start b bs :: locate (start+(events b).length) bs =
      pre ++ x::y::post at he
    cases pre with
    | nil =>
      simp only [List.nil_append,List.cons.injEq] at he
      rcases he with ⟨he,ht⟩
      subst x
      cases bs with
      | nil => simp [locate] at ht
      | cons c cs =>
        change LocatedProof.placed (start+(events b).length) c cs ::
          locate ((start+(events b).length)+(events c).length) cs = y::post at ht
        have hy := (List.cons.inj ht).1
        subst y
        refine ⟨rfl,?_⟩
        change positive c.first = !(positive b.first)
        have hn := hs.1
        cases hb : positive b.first <;> cases hc : positive c.first <;> simp_all
    | cons a pre =>
      simp only [List.cons_append,List.cons.injEq] at he
      have hs' : Separated bs := by cases bs with | nil => trivial | cons c cs => exact hs.2
      exact locate_adjacent bs _ hs' pre x y post he.2

theorem adjacent_contract : ADJACENT_TARGET := by
  intro es bs qs out ht hr hlen hc pre x y post he
  have hloc : locate 0 (group es) = pre.map Output.located ++
      x.located::y.located::post.map Output.located := by
    rw [← (source_contract es qs out hc).1,he]
    simp
  have hadj := locate_adjacent (group es) 0 (runs_properties (group_sound es)).2.2
    _ _ _ _ hloc
  refine ⟨hadj.1,hadj.2,by rw [hadj.1],?_⟩
  intro s t hs ht'
  have hx : x ∈ out := by rw [he]; simp
  have hy : y ∈ out := by rw [he]; simp
  rcases hull_contract es bs qs out ht hr hlen hc x hx s hs with
    ⟨q,r,hq,hrr,_,_,_,hss,_⟩
  rcases hull_contract es bs qs out ht hr hlen hc y hy t ht' with
    ⟨u,v,hu,hv,_,_,_,htt,_⟩
  have hru : r = u := by
    rw [hadj.1] at hrr
    exact Option.some.inj (Option.some.inj (hrr.symm.trans hu))
  subst u
  rw [hss,htt,hadj.2]
  cases positive x.located.block.first <;> rfl

theorem exact_root : SOURCE_TARGET ∧ COVER_TARGET ∧ HULL_TARGET ∧ ADJACENT_TARGET :=
  ⟨source_contract,cover_contract,hull_contract,adjacent_contract⟩

end DirectOutput

#print axioms DirectOutput.exact_root
