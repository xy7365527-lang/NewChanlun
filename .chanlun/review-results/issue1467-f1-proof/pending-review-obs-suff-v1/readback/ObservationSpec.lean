import Std

namespace ObservationResearch

def Observed {X S : Type} (obs : X → S) := {s : S // ∃ x, obs x = s}

def FiberConstant {X S A : Type} (obs : X → S) (target : X → A) : Prop :=
  ∀ x y, obs x = obs y → target x = target y

def FactorsOnImage {X S A : Type} (obs : X → S) (target : X → A) : Prop :=
  ∃ f : Observed obs → A, ∀ x, f ⟨obs x, ⟨x,rfl⟩⟩ = target x

def FACTORIZATION_TARGET : Prop :=
  ∀ (X S A : Type) (obs : X → S) (target : X → A),
    FactorsOnImage obs target ↔ FiberConstant obs target

def COLLISION_TARGET : Prop :=
  ∀ (X S A : Type) (obs : X → S) (target : X → A) (x y : X),
    obs x = obs y → target x ≠ target y → ¬ FactorsOnImage obs target

end ObservationResearch
