import RetilePacketSpec

namespace RetilePacket

open CenterAttempt (ObservedUnit)

theorem cell_size (time start : Nat) (t : Retile.Triple) :
    (cellPacket time start t).part.len = (cellPacket time start t).source.length ∧
    (cellPacket time start t).source ≠ [] := by
  simp [cellPacket,Retile.tripleMembers]

theorem tail_cover (time start : Nat) (xs : List ObservedUnit) (h : 0 < xs.length) :
    SourceHull.Cover start (start+xs.length) [(tailPacket time start xs).part] :=
  SourceHull.Cover.step (tailPacket time start xs).part h
    (SourceHull.Cover.done (start+xs.length))

theorem fidelity_data : ∀ (xs : List ObservedUnit) (time start : Nat),
    recover (assemble time start xs) = xs ∧
    SourceHull.Cover start (start+xs.length) ((assemble time start xs).map Packet.part) ∧
    cells (assemble time start xs) = (Retile.retile time xs).1 ∧
    (∀ p ∈ assemble time start xs, p.part.len = p.source.length ∧ p.source ≠ [])
  | [],time,start => by
      refine ⟨rfl,?_,rfl,?_⟩
      · simpa [assemble] using SourceHull.Cover.done start
      · intro p hp; simp [assemble] at hp
  | [a],time,start => by
      refine ⟨rfl,tail_cover time start [a] (by simp),rfl,?_⟩
      intro p hp
      simp only [assemble,List.mem_singleton] at hp
      subst p
      simp [tailPacket]
  | [a,b],time,start => by
      refine ⟨rfl,tail_cover time start [a,b] (by simp),rfl,?_⟩
      intro p hp
      simp only [assemble,List.mem_singleton] at hp
      subst p
      simp [tailPacket]
  | a::b::c::rest,time,start => by
      have ih := fidelity_data rest time (start+3)
      refine ⟨?_,?_,?_,?_⟩
      · change [a,b,c] ++ recover (assemble time (start+3) rest) = a::b::c::rest
        rw [ih.1]
        rfl
      · have hc := SourceHull.Cover.step (cellPacket time start ⟨a,b,c⟩).part
          (by simp [cellPacket]) ih.2.1
        have he : (start+3)+rest.length = start+(a::b::c::rest).length := by
          simp only [List.length_cons]
          omega
        rw [he] at hc
        exact hc
      · change Retile.classify time ⟨a,b,c⟩ :: cells (assemble time (start+3) rest) = _
        rw [ih.2.2.1]
        rfl
      · intro p hp
        change p ∈ cellPacket time start ⟨a,b,c⟩ :: assemble time (start+3) rest at hp
        rcases List.mem_cons.mp hp with hp | hp
        · subst p
          exact cell_size _ _ _
        · exact ih.2.2.2 p hp

theorem fidelity : FIDELITY_TARGET := by
  intro time start xs
  exact fidelity_data xs time start

theorem latest_member : ∀ (xs : List ObservedUnit) (u : ObservedUnit),
    u ∈ xs → u.1.knownAt ≤ latest xs
  | [],u,h => by simp at h
  | a::xs,u,h => by
      rcases List.mem_cons.mp h with h | h
      · subst u
        change a.1.knownAt ≤ max a.1.knownAt (latest xs)
        omega
      · have ih := latest_member xs u h
        change u.1.knownAt ≤ max a.1.knownAt (latest xs)
        omega

theorem nonempty_hull (xs : List ObservedUnit) (hn : xs ≠ []) :
    ∃ h, SourceHull.hull (SourceHull.intervals xs) = some h ∧
      SourceHull.ExactHull (SourceHull.intervals xs) h := by
  have hne : SourceHull.intervals xs ≠ [] := by
    cases xs with
    | nil => contradiction
    | cons a xs => simp [SourceHull.intervals]
  cases hv : SourceHull.hull (SourceHull.intervals xs) with
  | none => exact False.elim (hne ((SourceHull.none_iff _).mp hv))
  | some h => exact ⟨h,rfl,SourceHull.hull_sound _ _ hv⟩

def Good (time : Nat) (p : Packet) : Prop :=
  time ≤ p.availableAt ∧ (∀ u ∈ p.source, u.1.knownAt ≤ p.availableAt) ∧
  ∃ h, p.outer = some h ∧ SourceHull.ExactHull (SourceHull.intervals p.source) h

theorem cell_good (time start : Nat) (t : Retile.Triple) : Good time (cellPacket time start t) := by
  refine ⟨?_,?_,nonempty_hull _ (by simp [Retile.tripleMembers])⟩
  · change time ≤ Retile.knownAt (Retile.classify time t)
    rw [Retile.known_classify]
    unfold Retile.stamp
    omega
  · intro u hu
    change u ∈ [t.a,t.b,t.c] at hu
    simp only [List.mem_cons,List.not_mem_nil,or_false] at hu
    rcases hu with rfl | rfl | rfl
    all_goals
      change _ ≤ Retile.knownAt (Retile.classify time t)
      rw [Retile.known_classify]
      unfold Retile.stamp
      omega

theorem tail_good (time start : Nat) (xs : List ObservedUnit) (hn : xs ≠ []) :
    Good time (tailPacket time start xs) := by
  refine ⟨?_,?_,nonempty_hull _ hn⟩
  · change time ≤ max time (latest xs)
    omega
  · intro u hu
    have h := latest_member xs u hu
    change u.1.knownAt ≤ max time (latest xs)
    omega

theorem packets_data : ∀ (xs : List ObservedUnit) (time start : Nat) (p : Packet),
    p ∈ assemble time start xs → Good time p
  | [],time,start,p,h => by simp [assemble] at h
  | [a],time,start,p,h => by
      simp only [assemble,List.mem_singleton] at h
      subst p
      exact tail_good _ _ _ (by simp)
  | [a,b],time,start,p,h => by
      simp only [assemble,List.mem_singleton] at h
      subst p
      exact tail_good _ _ _ (by simp)
  | a::b::c::rest,time,start,p,h => by
      change p ∈ cellPacket time start ⟨a,b,c⟩ :: assemble time (start+3) rest at h
      rcases List.mem_cons.mp h with h | h
      · subst p
        exact cell_good _ _ _
      · exact packets_data rest time (start+3) p h

theorem packets : PACKET_TARGET := by
  intro time start xs p hp
  exact packets_data xs time start p hp

theorem roles : ROLE_TARGET := by
  intro time start t
  by_cases h : Retile.Local t
  · simp [cellPacket,Retile.accepted_classify,h]
  · simp [cellPacket,Retile.accepted_classify,h]

theorem witness : WITNESS_TARGET := by
  refine ⟨Retile.witness,LiftBoundary.witness,by decide,?_,by decide,
    by decide,by decide,?_,by decide,by decide,?_⟩
  · exact (fidelity_data Retile.weakInput 10 0).1
  · have hc : cells boundary = LiftBoundary.cells :=
      (fidelity_data (CenterFrame.members LiftBoundary.f3) 10 0).2.2.1
    rw [hc]
    unfold LiftBoundary.strictLift
    change ¬ UpgradeResearch.Valid (⟨10040,10040⟩ : UpgradeResearch.Interval)
    unfold UpgradeResearch.Valid
    decide
  · exact (fidelity_data shortInput 10 0).1

theorem exact_root : FIDELITY_TARGET ∧ PACKET_TARGET ∧ ROLE_TARGET ∧ WITNESS_TARGET :=
  ⟨fidelity,packets,roles,witness⟩

end RetilePacket

#print axioms RetilePacket.exact_root
