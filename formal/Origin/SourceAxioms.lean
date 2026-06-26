/-
Origin/SourceAxioms.lean

The origin layer starts from the strict mathematical reading of complete
classification.  Source-specific Chanlun obligations are parameters or theorem
hypotheses.  This file intentionally contains no Lean `axiom`.
-/

namespace NewChanlun.Origin

universe u v w

def ExistsUnique {A : Type u} (p : A -> Prop) : Prop :=
  Exists (fun x => p x /\ forall y, p y -> y = x)

def Total {A : Type u} {B : Type v} (R : A -> B -> Prop) : Prop :=
  forall a, Exists (fun b => R a b)

def SingleValued {A : Type u} {B : Type v} (R : A -> B -> Prop) : Prop :=
  forall a b c, R a b -> R a c -> b = c

def TotalUnique {A : Type u} {B : Type v} (R : A -> B -> Prop) : Prop :=
  forall a, ExistsUnique (fun b => R a b)

theorem total_unique_of_fun {A : Type u} {B : Type v} (f : A -> B) :
    TotalUnique (fun a b => f a = b) := by
  intro a
  refine ⟨f a, rfl, ?_⟩
  intro y hy
  exact hy.symm

theorem total_and_single_of_total_unique {A : Type u} {B : Type v}
    {R : A -> B -> Prop} (h : TotalUnique R) :
    Total R /\ SingleValued R := by
  constructor
  · intro a
    rcases h a with ⟨b, hb, _⟩
    exact ⟨b, hb⟩
  · intro a b c hb hc
    rcases h a with ⟨d, _, hd⟩
    exact (hd b hb).trans (hd c hc).symm

inductive Side where
  | long
  | short
deriving DecidableEq, Repr

inductive Direction where
  | up
  | down
deriving DecidableEq, Repr

def Direction.flip : Direction -> Direction
  | Direction.up => Direction.down
  | Direction.down => Direction.up

theorem direction_flip_involutive (d : Direction) : d.flip.flip = d := by
  cases d <;> rfl

def MirrorEquivariant {X : Type u} {Y : Type v}
    (mx : X -> X) (my : Y -> Y) (f : X -> Y) : Prop :=
  forall x, f (mx x) = my (f x)

def Causal {Hist _Event State : Type u}
    (prefixEq : Hist -> Hist -> Prop) (classify : Hist -> State) : Prop :=
  forall h h', prefixEq h h' -> classify h = classify h'

structure FixedTheta where
  data : Type
  parse : Type
  scale : Type
  signal : Type
  mirror : Type
  voice : Type
  phase : Type
  ledger : Type
  leverage : Type
  risk : Type
  execution : Type
  tieBreak : Type

inductive SourceKind where
  | pastedText
  | pdfScreenshot
  | chanlunText
  | repositoryReference
  | designParameter
deriving DecidableEq, Repr

structure SourceClaim where
  id : String
  kind : SourceKind
  statement : String

end NewChanlun.Origin
