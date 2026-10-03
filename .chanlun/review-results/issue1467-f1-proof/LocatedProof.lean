import LocatedSpec

namespace LocatedProof

open MovingQuote (Event Block Located positive events decode Uniform Separated group locate Completed)

theorem input_length (p : Nat → Event) (n : Nat) : (input p n).length = n := by simp [input]
theorem input_at (p : Nat → Event) (n i : Nat) (h : i < n) : (input p n)[i]? = some (p i) := by
  simp [input,List.getElem?_range h]

theorem event_after {p n} {pre post : List Event} {e : Event}
    (h : input p n = pre ++ e::post) : p pre.length = e := by
  have hn := congrArg List.length h
  simp only [input_length,List.length_append,List.length_cons] at hn
  have ht : pre.length < n := by omega
  have hx := congrArg (fun xs : List Event => xs[pre.length]?) h
  rw [input_at p n pre.length ht,List.getElem?_append_right (Nat.le_refl _)] at hx
  simp only [Nat.sub_self,List.getElem?_cons_zero] at hx
  exact Option.some.inj hx

theorem event_inside {p n} {pre mid post : List Event} {i : Nat}
    (h : input p n = pre ++ mid ++ post) (hs : pre.length ≤ i)
    (he : i < pre.length+mid.length) : p i ∈ mid := by
  have hn := congrArg List.length h
  simp only [input_length,List.length_append] at hn
  have ht : i < n := by omega
  have hm : i-pre.length < mid.length := by omega
  have hx := congrArg (fun xs : List Event => xs[i]?) h
  rw [List.append_assoc] at hx
  rw [input_at p n i ht,List.getElem?_append_right hs,List.getElem?_append_left hm] at hx
  exact List.mem_of_getElem? hx.symm

def Prev (p : Nat → Event) (s : Nat) : List Block → Prop
  | [] => True
  | b::_ => s = 0 ∨ positive (p (s-1)) ≠ positive b.first

def placed (start : Nat) (b : Block) (bs : List Block) : Located :=
  ⟨b,start,start+(events b).length,start+1,
    if bs.isEmpty then none else some (start+(events b).length+1)⟩

theorem head_span {p n} {pre after : List Event} {b : Block}
    (hi : input p n = pre ++ events b ++ after) (hu : Uniform b)
    (hl : pre.length = 0 ∨ positive (p (pre.length-1)) ≠ positive b.first)
    (hr : after = [] ∨ ∃ e rest, after = e::rest ∧ positive b.first ≠ positive e) :
    Span p n pre.length (pre.length+(events b).length) := by
  have hs : p pre.length = b.first := event_after (by simpa [List.append_assoc,events] using hi)
  have hn := congrArg List.length hi
  simp only [input_length,List.length_append] at hn
  have hb : 0 < (events b).length := by simp [events]
  have hc : ∀ i, pre.length ≤ i → i < pre.length+(events b).length →
      positive (p i) = positive b.first := fun i h1 h2 => hu _ (event_inside hi h1 h2)
  refine ⟨by omega,by omega,?_,?_,?_⟩
  · simpa only [hs] using hl
  · intro i h1 h2
    simpa only [hs] using hc i h1 h2
  · rcases hr with he | ⟨e,rest,he,hne⟩
    · left
      simp only [he,List.length_nil] at hn
      omega
    · right
      have hin : input p n = (pre ++ events b) ++ e::rest := by simpa [List.append_assoc,he] using hi
      have hx := event_after hin
      simp only [List.length_append] at hx
      have hy := hc (pre.length+(events b).length-1) (by omega) (by omega)
      intro h
      exact hne (hy.symm.trans (h.trans (congrArg positive hx)))

theorem head_good {p n pre b bs}
    (hi : input p n = pre ++ decode (b::bs)) (hu : Uniform b)
    (hsep : Separated (b::bs)) (hl : Prev p pre.length (b::bs)) :
    Good p n (placed pre.length b bs) := by
  have his : input p n = pre ++ events b ++ decode bs := by
    simpa only [decode,List.flatMap_cons,List.append_assoc] using hi
  have hr : decode bs = [] ∨ ∃ e rest, decode bs = e::rest ∧ positive b.first ≠ positive e := by
    cases bs with
    | nil => exact Or.inl rfl
    | cons c cs => exact Or.inr ⟨c.first,c.rest ++ decode cs,rfl,hsep.1⟩
  have hspan := head_span his hu hl hr
  have hn := congrArg List.length his
  simp only [input_length,List.length_append] at hn
  cases bs with
  | nil =>
    have he : pre.length+(events b).length = n := by simp [decode] at hn; omega
    refine ⟨hspan,rfl,rfl,?_,?_⟩
    · simp [placed,he]
    · intro k hk; simp [placed] at hk
  | cons c cs =>
    have hpos : 0 < (decode (c::cs)).length := by simp [decode,events]
    have he : pre.length+(events b).length ≠ n := by omega
    refine ⟨hspan,rfl,rfl,?_,?_⟩
    · simp [placed,he]
    · intro k hk
      exact (Option.some.inj hk).symm

theorem located_data (p : Nat → Event) (n : Nat) : ∀ (bs : List Block) (pre : List Event),
    input p n = pre ++ decode bs → (∀ b ∈ bs, Uniform b) → Separated bs → Prev p pre.length bs →
    ∀ l ∈ locate pre.length bs, Good p n l
  | [],pre,hi,hu,hsep,hl,l,hmem => by simp [locate] at hmem
  | b::bs,pre,hi,hu,hsep,hl,l,hmem => by
    have hb := hu b (by simp)
    change l ∈ placed pre.length b bs :: locate (pre.length+(events b).length) bs at hmem
    rcases List.mem_cons.mp hmem with hmem | hmem
    · subst l
      exact head_good hi hb hsep hl
    · have hi' : input p n = (pre ++ events b) ++ decode bs := by simpa [decode,List.append_assoc] using hi
      have hu' : ∀ c ∈ bs, Uniform c := fun c hc => hu c (List.mem_cons_of_mem b hc)
      have hs' : Separated bs := by cases bs with | nil => trivial | cons c cs => exact hsep.2
      have hl' : Prev p (pre ++ events b).length bs := by
        cases bs with
        | nil => trivial
        | cons c cs =>
          right
          have hbpos : 0 < (events b).length := by simp [events]
          have hin : input p n = pre ++ events b ++ decode (c::cs) := by
            simpa only [decode,List.flatMap_cons,List.append_assoc] using hi
          have he : positive (p ((pre ++ events b).length-1)) = positive b.first := by
            apply hb
            apply event_inside hin <;> simp only [List.length_append] <;> omega
          exact fun h => hsep.1 (he.symm.trans h)
      have hmem' : l ∈ locate (pre ++ events b).length bs := by simpa only [List.length_append] using hmem
      exact located_data p n bs (pre ++ events b) hi' hu' hs' hl' l hmem'

theorem covers : ∀ (bs : List Block) (start i : Nat), start ≤ i → i < start+(decode bs).length →
    ∃ l ∈ locate start bs, l.start ≤ i ∧ i < l.finish
  | [],start,i,hs,he => by simp [decode] at he; omega
  | b::bs,start,i,hs,he => by
    by_cases h : i < start+(events b).length
    · exact ⟨placed start b bs,by simp [locate,placed],hs,h⟩
    · have he' : i < (start+(events b).length)+(decode bs).length := by
        simpa only [decode,List.flatMap_cons,List.length_append,Nat.add_assoc] using he
      rcases covers bs (start+(events b).length) i (by omega) he' with ⟨l,hl,h1,h2⟩
      exact ⟨l,List.mem_cons_of_mem _ hl,h1,h2⟩

theorem program : PROGRAM_TARGET := by
  constructor
  · intro p n l hl
    have h := MovingQuote.runs_properties (MovingQuote.group_sound (input p n))
    have hi : input p n = [] ++ decode (group (input p n)) := by simpa using h.1.symm
    apply located_data p n (group (input p n)) [] hi h.2.1 h.2.2 ?_ l hl
    cases group (input p n) <;> simp [Prev]
  · intro p n i hi
    have h := (MovingQuote.runs_properties (MovingQuote.group_sound (input p n))).1
    have hn : (decode (group (input p n))).length = n := by rw [h,input_length]
    exact covers (group (input p n)) 0 i (by omega) (by simpa [hn] using hi)

theorem span_of_completed {p n s e} (h : Completed p n s e) : Span p n s e := by
  rcases h with ⟨hs,he,hl,hu,hr⟩
  exact ⟨hs,by omega,hl,fun i hi hie => hu ⟨i,hie⟩ hi,Or.inr hr⟩

theorem completed_of_span {p n s e} (h : Span p n s e) (he : e < n) : Completed p n s e := by
  rcases h with ⟨hs,hen,hl,hu,hr⟩
  refine ⟨hs,he,hl,fun i hi => hu i.val hi i.isLt,?_⟩
  rcases hr with hr | hr
  · omega
  · exact hr

theorem span_unique {p n s e a b j} (h : Span p n s e) (g : Span p n a b)
    (hj : s ≤ j ∧ j < e) (gj : a ≤ j ∧ j < b) : s = a ∧ e = b := by
  rcases h with ⟨hse,hen,hl,hu,hr⟩
  rcases g with ⟨hab,hbn,gl,gu,gr⟩
  have hsa : s = a := by
    by_cases hneq : s = a
    · exact hneq
    · have hc : s < a ∨ a < s := by omega
      rcases hc with hc | hc
      · rcases gl with hz | hne
        · omega
        · exact False.elim (hne ((hu (a-1) (by omega) (by omega)).trans (hu a (by omega) (by omega)).symm))
      · rcases hl with hz | hne
        · omega
        · exact False.elim (hne ((gu (s-1) (by omega) (by omega)).trans (gu s (by omega) (by omega)).symm))
  refine ⟨hsa,?_⟩
  subst a
  by_cases hneq : e = b
  · exact hneq
  · have hc : e < b ∨ b < e := by omega
    rcases hc with hc | hc
    · rcases hr with hz | hne
      · omega
      · exact False.elim (hne ((gu (e-1) (by omega) (by omega)).trans (gu e (by omega) (by omega)).symm))
    · rcases gr with hz | hne
      · omega
      · exact False.elim (hne ((hu (b-1) (by omega) (by omega)).trans (hu b (by omega) (by omega)).symm))

theorem match_contract : MATCH_TARGET := by
  intro p n s e
  constructor
  · intro h
    have hs := h.1
    have he := h.2.1
    rcases program.2 p n s (by omega) with ⟨l,hl,hls,hle⟩
    have hg := program.1 p n l hl
    have hu := span_unique (span_of_completed h) hg.1 ⟨Nat.le_refl _,hs⟩ ⟨hls,hle⟩
    refine ⟨l,hl,hu.1.symm,hu.2.symm,?_⟩
    cases hk : l.knownAt with
    | none => have hx := hg.2.2.2.1.mp hk; omega
    | some k =>
      have hx := hg.2.2.2.2 k hk
      simpa [hx,← hu.2] using hk
  · rintro ⟨l,hl,hs,he,hk⟩
    have hg := program.1 p n l hl
    have hlt : l.finish < n := by
      by_cases hn : l.finish < n
      · exact hn
      · have hx : l.finish = n := by have hb := hg.1.2.1; omega
        have hz := hg.2.2.2.1.mpr hx
        rw [hk] at hz
        cases hz
    have h := completed_of_span hg.1 hlt
    simpa [hs,he] using h

theorem seen_iff {p n t s e} (ht : t ≤ n) : Seen p n t s e ↔ Completed p t s e := by
  constructor
  · rintro ⟨l,hl,hs,he,hk,hcut⟩
    have h := (match_contract p n s e).mpr ⟨l,hl,hs,he,hk⟩
    exact ⟨h.1,by omega,h.2.2.1,h.2.2.2.1,h.2.2.2.2⟩
  · intro h
    have hn : Completed p n s e := ⟨h.1,by have hx:=h.2.1; omega,h.2.2.1,h.2.2.2.1,h.2.2.2.2⟩
    rcases (match_contract p n s e).mp hn with ⟨l,hl,hs,he,hk⟩
    exact ⟨l,hl,hs,he,hk,by have hx:=h.2.1; omega⟩

theorem causal : CAUSAL_TARGET := by
  intro p q n m t hn hm hp s e
  rw [seen_iff hn,seen_iff hm]
  exact MovingQuote.causal p q t s e hp

theorem witness : WITNESS_TARGET := by
  refine ⟨by decide,?_,?_,?_,?_,by decide⟩
  all_goals unfold MovingQuote.Completed DirectQuote.Completed; decide

theorem exact_root : PROGRAM_TARGET ∧ MATCH_TARGET ∧ CAUSAL_TARGET ∧ WITNESS_TARGET :=
  ⟨program,match_contract,causal,witness⟩

end LocatedProof

#print axioms LocatedProof.exact_root
