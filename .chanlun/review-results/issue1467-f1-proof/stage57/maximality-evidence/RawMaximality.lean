import Lean.Elab.Tactic.Omega

namespace Stage57

structure Closed (α : Type) (le : α → α → Prop) where
  low : α
  high : α
  valid : le low high

structure Seed (α : Type) (le : α → α → Prop) where
  identity : Nat
  core : Closed α le
  start : Nat

structure Tape (α : Type) (le : α → α → Prop) where
  range : Nat → Closed α le
  sealAt : Nat → Nat
  endAt : Nat → Nat
  seal_strict : ∀ {i j}, i < j → sealAt i < sealAt j
  end_before_seal : ∀ i, endAt i < sealAt i

variable {α : Type} {le : α → α → Prop}

def Touch (a b : Closed α le) : Prop := le a.low b.high ∧ le b.low a.high

def TouchSpan (h : Tape α le) (K : Seed α le) (k : Nat) : Prop :=
  K.start + 2 ≤ k ∧ ∀ i, K.start ≤ i → i ≤ k → Touch (h.range i) K.core

def RawTerminal (h : Tape α le) (K : Seed α le) (j r τ : Nat) : Prop :=
  TouchSpan h K j ∧ r = j + 1 ∧ h.sealAt r ≤ τ ∧ ¬Touch (h.range r) K.core

def PreservesThrough (h h' : Tape α le) (r : Nat) : Prop :=
  ∀ i, i ≤ r → h'.range i = h.range i ∧ h'.sealAt i = h.sealAt i ∧
    h'.endAt i = h.endAt i

theorem raw_maximal {h : Tape α le} {K : Seed α le} {j r τ : Nat}
    (ht : RawTerminal h K j r τ) : ∀ k, j < k → ¬TouchSpan h K k := by
  rcases ht with ⟨⟨hs, _⟩, hr, _, hd⟩
  intro k hjk hk
  apply hd
  apply hk.2 r <;> omega

theorem terminal_unique {h : Tape α le} {K : Seed α le} {j r τ : Nat}
    (ht : RawTerminal h K j r τ) :
    ∀ q r' τ', RawTerminal h K q r' τ' → q = j := by
  intro q r' τ' hq
  rcases Nat.lt_trichotomy q j with hlt | heq | hgt
  · exact False.elim (raw_maximal hq j hlt ht.1)
  · exact heq
  · exact False.elim (raw_maximal ht q hgt hq.1)

theorem terminal_preserved {h h' : Tape α le} {K : Seed α le} {j r τ : Nat}
    (ht : RawTerminal h K j r τ) (hp : PreservesThrough h h' r) :
    RawTerminal h' K j r τ := by
  rcases ht with ⟨⟨hs, ha⟩, hr, hk, hd⟩
  refine ⟨⟨hs, ?_⟩, hr, ?_, ?_⟩
  · intro i hsi hij
    rw [(hp i (by omega)).1]
    exact ha i hsi hij
  · rw [(hp r (Nat.le_refl r)).2.1]
    exact hk
  · rw [(hp r (Nat.le_refl r)).1]
    exact hd

theorem first_check {h : Tape α le} {K : Seed α le} {j r τ : Nat}
    (ht : RawTerminal h K j r τ) :
    RawTerminal h K j r (h.sealAt r) ∧
    (∀ t, t < h.sealAt r → ¬RawTerminal h K j r t) ∧
    h.endAt j < h.sealAt r := by
  refine ⟨⟨ht.1, ht.2.1, Nat.le_refl _, ht.2.2.2⟩, ?_, ?_⟩
  · intro t hlt ht'
    exact Nat.not_le_of_gt hlt ht'.2.2.1
  · have hjr : j < r := by have := ht.2.1; omega
    exact Nat.lt_trans (h.end_before_seal j) (h.seal_strict hjr)

theorem terminal_root (α : Type) (le : α → α → Prop) (h : Tape α le) (K : Seed α le) (j r τ : Nat)
    (ht : RawTerminal h K j r τ) :
    (∀ k, j < k → ¬TouchSpan h K k) ∧
    (∀ q r' τ', RawTerminal h K q r' τ' → q = j) ∧
    (∀ h', PreservesThrough h h' r →
      RawTerminal h' K j r τ ∧ ∀ k, j < k → ¬TouchSpan h' K k) ∧
    RawTerminal h K j r (h.sealAt r) ∧
    (∀ t, t < h.sealAt r → ¬RawTerminal h K j r t) ∧
    h.endAt j < h.sealAt r := by
  refine ⟨raw_maximal ht, terminal_unique ht, ?_, first_check ht⟩
  intro h' hp
  have ht' := terminal_preserved ht hp
  exact ⟨ht', raw_maximal ht'⟩

#print terminal_root
#print axioms terminal_root
end Stage57
