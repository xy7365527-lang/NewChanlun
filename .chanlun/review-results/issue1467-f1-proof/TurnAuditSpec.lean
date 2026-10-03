import Origin.Turn

namespace TurnAudit

open NewChanlun.Origin

def qNoDiv : TrendInstance :=
  ⟨2,⟨⟨3⟩,⟨8⟩,true⟩⟩
def rDiv : TrendInstance :=
  ⟨1,⟨⟨8⟩,⟨2⟩,true⟩⟩
def nextQ : TrendInstance :=
  ⟨2,⟨⟨4⟩,⟨2⟩,false⟩⟩
def suppliedNext (_ : TrendInstance) : Option TrendInstance := some nextQ

def COVERAGE_TARGET : Prop :=
  (∀ T, ¬ IsDivergence T.divPair → ∀ succ, ¬ Turn_q succ T) ∧
  rDiv.level < qNoDiv.level ∧ Completed rDiv ∧ ¬ Completed qNoDiv ∧
  (∃ T', suppliedNext qNoDiv = some T' ∧ T'.level = qNoDiv.level) ∧
  ¬ Turn_q suppliedNext qNoDiv

def SELF_TARGET : Prop :=
  EventualHandover succSelf ∧ Turn_q succSelf T1 ∧ succSelf T1 = some T1

/-- 仅供检验被旧载荷投影丢弃的来源位置，不是完整走势类型。 -/
structure Located where
  payload : TrendInstance
  start : Nat
  finish : Nat

/-- 必要来源接续关系；不包含完成、真实成员或可知时刻的证书。 -/
def Forward (a b : Located) : Prop :=
  a.start < a.finish ∧ a.finish = b.start ∧ b.start < b.finish ∧
  a.payload.level = b.payload.level
instance (a b : Located) : Decidable (Forward a b) := by unfold Forward; infer_instance

def firstOccurrence : Located := ⟨T1,0,3⟩
def laterOccurrence : Located := ⟨T1,3,6⟩

def IDENTITY_TARGET : Prop :=
  (∀ a, ¬ Forward a a) ∧
  Forward firstOccurrence laterOccurrence ∧
  firstOccurrence.payload = laterOccurrence.payload ∧
  (¬ ∃ judge : TrendInstance → TrendInstance → Bool,
    ∀ a b : Located, judge a.payload b.payload = decide (Forward a b))

end TurnAudit
