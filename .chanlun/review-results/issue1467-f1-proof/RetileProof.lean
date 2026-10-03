import RetileSpec

namespace Retile

open NewChanlun.Origin
open CenterAttempt (ObservedUnit obs)

theorem partition_sound : ∀ xs, Cut xs (partition xs).1 (partition xs).2
  | [] => Cut.tail [] (by decide)
  | [a] => Cut.tail [a] (by simp)
  | [a,b] => Cut.tail [a,b] (by simp)
  | a::b::c::rest => Cut.block a b c (partition_sound rest)

theorem cut_determines {xs ts tail} (h : Cut xs ts tail) : partition xs = (ts,tail) := by
  induction h with
  | tail xs short =>
    cases xs with
    | nil => rfl
    | cons a xs =>
      cases xs with
      | nil => rfl
      | cons b xs =>
        cases xs with
        | nil => rfl
        | cons c rest => simp at short; omega
  | block a b c h ih => simp [partition,ih]

theorem cut_properties {xs ts tail} (h : Cut xs ts tail) :
    flatten ts ++ tail = xs ∧ tail.length < 3 ∧ xs.length = 3*ts.length + tail.length := by
  induction h with
  | tail xs short => exact ⟨rfl,short,by simp⟩
  | @block a b c xs ts tail h ih =>
    refine ⟨?_,ih.2.1,?_⟩
    · change a::b::c::(flatten ts ++ tail) = a::b::c::xs
      rw [ih.1]
    · simp only [List.length_cons]
      have hc := ih.2.2
      omega

theorem source_classify (time : Nat) (t : Triple) : sourceOf (classify time t) = t := by
  unfold classify
  split <;> rfl
theorem known_classify (time : Nat) (t : Triple) : knownAt (classify time t) = stamp time t := by
  unfold classify
  split <;> rfl
theorem accepted_classify (time : Nat) (t : Triple) : accepted (classify time t) = decide (Local t) := by
  by_cases h : Local t <;> simp [accepted,classify,centerOf,h]

theorem flattened_classify (time : Nat) (ts : List Triple) :
    (ts.map (classify time)).flatMap (fun c => tripleMembers (sourceOf c)) = flatten ts := by
  induction ts with
  | nil => rfl
  | cons t ts ih =>
    simp only [List.map_cons,List.flatMap_cons,source_classify,flatten]
    simpa [flatten] using congrArg (fun rest => tripleMembers t ++ rest) ih

theorem recover_retile (time : Nat) (xs : List ObservedUnit) :
    recover (retile time xs).1 (retile time xs).2 = xs := by
  unfold recover retile
  rw [flattened_classify]
  exact (cut_properties (partition_sound xs)).1

theorem partition_contract : PARTITION_TARGET := by
  refine ⟨?_,?_,recover_retile⟩
  · intro xs
    refine ⟨(partition xs).1,(partition xs).2,partition_sound xs,?_⟩
    intro ts tail h
    have he := cut_determines h
    exact ⟨congrArg Prod.fst he.symm,congrArg Prod.snd he.symm⟩
  · intro xs ts tail h
    exact cut_properties h

theorem cell_contract : CELL_TARGET := by
  intro time t
  refine ⟨source_classify time t,accepted_classify time t,?_,?_,?_,?_,?_⟩
  · rw [known_classify]; unfold stamp; omega
  · rw [known_classify]; unfold stamp; omega
  · rw [known_classify]; unfold stamp; omega
  · rw [known_classify]; unfold stamp; omega
  · intro z hz
    unfold classify at hz
    split at hz
    · rename_i h
      have he : centerFullOfConfirmed t.a.2 t.b.2 t.c.2 h = z := Option.some.inj hz
      subst z
      exact ⟨rfl,rfl,rfl,rfl,rfl,rfl⟩
    · simp [centerOf] at hz

theorem dc_reject : DC_REJECT_TARGET := by
  intro p delta hd len result hw pre a b c rest hseq time
  let t : Triple := ⟨obs p a,obs p b,obs p c⟩
  change accepted (classify time t) = false ↔ _
  rw [accepted_classify]
  by_cases h : Local t
  · have hc : computeZD (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) <
        computeZG (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) := h.2
    simp only [decide_eq_false_iff_not]
    simp only [Tick] at *
    constructor
    · intro hn; exact False.elim (hn h)
    · intro hn; omega
  · have hc : computeZG (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) ≤
        computeZD (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) := by
      apply Classical.byContradiction
      intro hn
      have hp : computeZD (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) <
          computeZG (DCOriginBridge.embed p a) (DCOriginBridge.embed p b) (DCOriginBridge.embed p c) := by
        simp only [Tick] at *; omega
      exact h (DCOriginBridge.dc_three_center hd hw pre a b c rest hseq hp)
    simp [h,hc]

theorem witness : WITNESS_TARGET := by
  refine ⟨CenterFrame.witnessNine,UpgradeResearch.counterexample,rfl,rfl,by decide,
    by decide,by decide,?_,rfl,rfl,by decide,by decide,by decide,?_⟩
  · exact recover_retile _ _
  · exact recover_retile _ _

theorem exact_root : PARTITION_TARGET ∧ CELL_TARGET ∧ DC_REJECT_TARGET ∧ WITNESS_TARGET :=
  ⟨partition_contract,cell_contract,dc_reject,witness⟩

end Retile

#print axioms Retile.exact_root
