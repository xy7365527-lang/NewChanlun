/-
  Strict/CThetaWiring.lean — #665 C_Θ 具体接线回填

  本文件只做组合，不重证既有数学：
  1. 把 `ClassificationFamily` 的抽象 C_Θ 具体化为已固定 Θ 下的
     `RContext → RLevel` 分类器；
  2. 用 `LevelState` 的六个逐态 iff 组装非标签核的语义谓词，并填满
     `Strict.Classifies` 的 total/sound/complete/disjoint/realized 五项义务；
  3. 把标准②递归分解、③δ/因果、⑤未完成分支、⑥π 完全应对的既有定理
     以字段引用装入一个证据包；
  4. 把 #661 裁定的逐格 gatekeeper 终态钉为类型级映射。

  认识论与有效域（090 / #144）：
  - 全部新定理为 L0 组合定理；Θ 的选择不由缠论无参数推出。
  - `RContext` 已物化 Θ_parse/Θ_signal 的输出（最后中枢与 3B/3S 判定）。
    `FixedRTheta` 的单构造子只表示“本文件接入这一份已固定 Θ”，不声称 Θ 唯一。
  - 四格原生对象域彼此不同：递归格用 Move，因果格用 LocatedEvent 流，
    未完成格用 OpenHist，操作格用 StrictState。证据包是标准六部分的并列 adapter，
    不声称这些定理都是 `rlevelOf` 的同域性质。
  - ②格的 `Strict.Recursive.moveDecomp` 取 `RelT = Eq`，只负责镜像树 round-trip；
    真正的 ∼ₙ 边界另由 `Strict.Decomp` 承载。gauge 前商 > 1 的多义性与
    gauge 后纤维内截面唯一同时进入字段，绝不把后者冒充前者。
  - ③格 δ 的全定义只在 `ValidEvent` 忠实域成立，不提升为裸 `Side` 全域命题。
  - ⑤格唯一的是当下状态与已发生延伸的分支归属，不预测唯一未来终局。
  - ⑥格只到 OperationalSemanticsOnly；数量维度边界
    `op_quantity_out_of_layer2` 原样引用，不升级为 Layer2 双射。

  盈亏、最优、守恒经验侧不在本文件范围。
-/

import Strict.ClassificationFamily
import Strict.LevelState
import Strict.Recursive
import Strict.Decomp
import Strict.Causal
import Strict.OpenTail
import Strict.Op
import Formal.EvalSoundness

namespace Strict.CThetaWiring

open Formal.TrendTrichotomy (Direction)
open Formal.RecursiveConstruction (Move)
open Formal.EvalSoundness (GroupSpec groupBySpecs leaves)
open Formal.BSPLabels (Side)
open Tlayers.Signal (PositionState LocatedEvent step run)
open Tlayers.Signal.PositionState (flat)
open Formal.OperationalSemantics (Exec I OpEqSemantic)

/-! ## 1. 固定 Θ 的 RLevel 分类器族 -/

/--
  本文件接入的固定 Θ 标识。

  Θ_parse/Θ_signal 的运行结果已经进入 `RContext.lastCenter/b3/s3`；这里的单构造子
  只选择这份既有 `rlevelOf` 实现，不把 Θ 的全空间错误坍缩为唯一值。
-/
inductive FixedRTheta where
  | certified
deriving DecidableEq, Repr

/--
  C_Θ 的具体 `ClassifierFamily` 实例：标签集固定为已证六态 `RLevel`，
  分类函数固定为 `rlevelOf`。
-/
def rlevelCTheta :
    Strict.ClassificationFamily.ClassifierFamily
      Strict.LevelState.RContext FixedRTheta where
  State := fun _ => Strict.LevelState.RLevel
  C := fun _ => Strict.LevelState.rlevelOf

/-- RLevel 六态的独立语义谓词族；各分支直接落到 `LevelState` 已证谓词。 -/
def RLevelMeaning :
    Strict.LevelState.RLevel → Strict.LevelState.RContext → Prop
  | .bot, ctx => Strict.LevelState.IsBot ctx
  | .inside, ctx => Strict.LevelState.IsInside ctx
  | .aboveNo3B, ctx => Strict.LevelState.IsAboveNo3B ctx
  | .aboveB3, ctx => Strict.LevelState.IsAboveB3 ctx
  | .belowNo3S, ctx => Strict.LevelState.IsBelowNo3S ctx
  | .belowS3, ctx => Strict.LevelState.IsBelowS3 ctx

/--
  C_Θ 标签与六态语义逐态等价。

  证明逐构造子引用 `rlevelOf_eq_*`，不是把谓词定义为标签相等的标签核。
-/
theorem rlevelCTheta_label_semantics
    (ctx : Strict.LevelState.RContext) (label : Strict.LevelState.RLevel) :
    rlevelCTheta.C FixedRTheta.certified ctx = label ↔
      RLevelMeaning label ctx := by
  change Strict.LevelState.rlevelOf ctx = label ↔ RLevelMeaning label ctx
  cases label with
  | bot => exact Strict.LevelState.rlevelOf_eq_bot ctx
  | inside => exact Strict.LevelState.rlevelOf_eq_inside ctx
  | aboveNo3B => exact Strict.LevelState.rlevelOf_eq_aboveNo3B ctx
  | aboveB3 => exact Strict.LevelState.rlevelOf_eq_aboveB3 ctx
  | belowNo3S => exact Strict.LevelState.rlevelOf_eq_belowNo3S ctx
  | belowS3 => exact Strict.LevelState.rlevelOf_eq_belowS3 ctx

/--
  每个 RLevel 语义标签都有一个具体 `RContext` 见证。

  这里的 realized 只针对裸 `RContext` 类型域：人工构造的 lastCenter/b3/s3 组合可承载六态。
  它不证明这些上下文都能由实际 parse/signal 运行管线到达；engine reachability 不在本票。
-/
theorem rlevelMeaning_realized (label : Strict.LevelState.RLevel) :
    ∃ ctx : Strict.LevelState.RContext, RLevelMeaning label ctx := by
  let c : Formal.CenterTrichotomy.Center :=
    ⟨0, 1, 2, 3, by decide, by decide, by decide⟩
  cases label with
  | bot =>
      exact ⟨{ lastCenter := none, price := 0, b3 := false, s3 := false }, rfl⟩
  | inside =>
      refine ⟨{ lastCenter := some c, price := 1, b3 := false, s3 := false }, ?_⟩
      simp [RLevelMeaning, Strict.LevelState.IsInside,
        Formal.CenterPosition.IsWithin, c]
  | aboveNo3B =>
      refine ⟨{ lastCenter := some c, price := 3, b3 := false, s3 := false }, ?_⟩
      simp [RLevelMeaning, Strict.LevelState.IsAboveNo3B,
        Formal.CenterPosition.IsAbove, c]
  | aboveB3 =>
      refine ⟨{ lastCenter := some c, price := 3, b3 := true, s3 := false }, ?_⟩
      simp [RLevelMeaning, Strict.LevelState.IsAboveB3,
        Formal.CenterPosition.IsAbove, c]
  | belowNo3S =>
      refine ⟨{ lastCenter := some c, price := 0, b3 := false, s3 := false }, ?_⟩
      simp [RLevelMeaning, Strict.LevelState.IsBelowNo3S,
        Formal.CenterPosition.IsBelow, c]
  | belowS3 =>
      refine ⟨{ lastCenter := some c, price := 0, b3 := false, s3 := true }, ?_⟩
      simp [RLevelMeaning, Strict.LevelState.IsBelowS3,
        Formal.CenterPosition.IsBelow, c]

/--
  固定 Θ 下 RLevel 是带语义的 `Classifies`：五项义务全部机器见证。

  此结论的有效域是已物化 Θ 的裸 `RContext` 类型域，不推出运行管线可达、Θ 无参数唯一
  或收益性。因此沿用 `LevelState` 的 StructurePartitionOnly gate；#661 授予
  TrueCompleteClassification 的终态仅落在下方②/③/⑤三格。
-/
-- [gk:StructurePartitionOnly]
def rlevelCTheta_classifies :
    Strict.Classifies
      Strict.LevelState.RContext
      Strict.LevelState.RLevel
      (rlevelCTheta.C FixedRTheta.certified)
      RLevelMeaning where
  total := fun ctx =>
    ⟨rlevelCTheta.C FixedRTheta.certified ctx,
      (rlevelCTheta_label_semantics ctx _).mp rfl⟩
  sound := fun ctx =>
    (rlevelCTheta_label_semantics ctx _).mp rfl
  complete := fun {ctx label} h =>
    (rlevelCTheta_label_semantics ctx label).mpr h
  disjoint := fun {ctx label₁ label₂} h₁ h₂ =>
    ((rlevelCTheta_label_semantics ctx label₁).mpr h₁).symm.trans
      ((rlevelCTheta_label_semantics ctx label₂).mpr h₂)
  realized := rlevelMeaning_realized

/-- `LevelState` 原六态互斥穷尽定理的直接引用锚。 -/
theorem rlevel_exhaustive_exclusive_wired (ctx : Strict.LevelState.RContext) :
    (Strict.LevelState.IsBot ctx ∨ Strict.LevelState.IsInside ctx
      ∨ Strict.LevelState.IsAboveNo3B ctx ∨ Strict.LevelState.IsAboveB3 ctx
      ∨ Strict.LevelState.IsBelowNo3S ctx ∨ Strict.LevelState.IsBelowS3 ctx)
    ∧ (Strict.LevelState.IsBot ctx →
      ¬ Strict.LevelState.IsInside ctx ∧ ¬ Strict.LevelState.IsAboveNo3B ctx
        ∧ ¬ Strict.LevelState.IsAboveB3 ctx ∧ ¬ Strict.LevelState.IsBelowNo3S ctx
        ∧ ¬ Strict.LevelState.IsBelowS3 ctx)
    ∧ (Strict.LevelState.IsInside ctx →
      ¬ Strict.LevelState.IsAboveNo3B ctx ∧ ¬ Strict.LevelState.IsAboveB3 ctx
        ∧ ¬ Strict.LevelState.IsBelowNo3S ctx ∧ ¬ Strict.LevelState.IsBelowS3 ctx)
    ∧ (Strict.LevelState.IsAboveNo3B ctx →
      ¬ Strict.LevelState.IsAboveB3 ctx ∧ ¬ Strict.LevelState.IsBelowNo3S ctx
        ∧ ¬ Strict.LevelState.IsBelowS3 ctx)
    ∧ (Strict.LevelState.IsAboveB3 ctx →
      ¬ Strict.LevelState.IsBelowNo3S ctx ∧ ¬ Strict.LevelState.IsBelowS3 ctx)
    ∧ (Strict.LevelState.IsBelowNo3S ctx → ¬ Strict.LevelState.IsBelowS3 ctx) :=
  Strict.LevelState.rlevel_exhaustive_exclusive ctx

/-! ## 2. 标准②：递归分解证据（Eval + 截面 + 正式多义边界） -/

/--
  ②格证据。

  `mirrorSoundUnique` 只说 `Recursive.DecompTree ↔ Move` 镜像 round-trip（其 RelT=Eq）；
  `evalSound` 给非平凡 leaves/groupBySpecs 重构；`gaugeSectionUnique` 与
  `preGaugeAmbiguity` 则共同钉死“gauge 后截面唯一、gauge 前商仍真多义”的正式边界。
-/
structure RecursiveGridEvidence : Prop where
  mirrorSoundUnique :
    (∀ h,
      Strict.Recursive.moveDecomp.Eval
          (Strict.Recursive.moveDecomp.D (Strict.Recursive.moveDecomp.d h)) =
        Strict.Recursive.moveDecomp.d h) ∧
    (∀ h t,
      Strict.Recursive.moveDecomp.ValidDecomp
          (Strict.Recursive.moveDecomp.d h) t →
        Strict.Recursive.moveDecomp.RelT t
          (Strict.Recursive.moveDecomp.D (Strict.Recursive.moveDecomp.d h)))
  evalSound :
    ∀ (specs : List GroupSpec) (xs : List Move),
      (∀ x ∈ xs, ∃ (d : Direction) (lo hi : Int), x = Move.segment d lo hi) →
      (groupBySpecs specs xs).flatMap leaves = xs
  gaugeSectionUnique :
    ∀ (h : List Move) (cands : List Move) (c₁ c₂ : Move),
      (∀ t ∈ cands, Strict.Decomp.ValidDecomp h t) →
      Strict.Decomp.GaugeNormal cands c₁ →
      Strict.Decomp.GaugeNormal cands c₂ →
      c₁ = c₂ ∧ Strict.Decomp.ValidDecomp h c₁ ∧ Strict.Decomp.ValidDecomp h c₂
  preGaugeAmbiguity :
    ∃ (h : List Move) (t₁ t₂ : Move),
      Strict.Decomp.SummaryI t₁ = h ∧
      Strict.Decomp.SummaryI t₂ = h ∧
      Strict.Decomp.SummarySimN t₁ t₂ ∧
      ¬ Strict.Decomp.StructEqN t₁ t₂

-- [gk:TrueCompleteClassification]
def recursiveGridEvidence : RecursiveGridEvidence where
  mirrorSoundUnique := Strict.Recursive.L2_decomp_sound_and_unique
  evalSound := Formal.EvalSoundness.eval_sound_roundtrip
  gaugeSectionUnique := Strict.Decomp.gauge_section_unique
  preGaugeAmbiguity := Strict.Decomp.decomp_ambiguity_witness

/--
  ②格正式边界声明：同一个已接线证据同时给出 gauge 后纤维截面唯一与
  gauge 前 `StructEqN` 商多义见证。故 TrueCompleteClassification 标签只适用于
  #661 裁定的“规范截面终态”，不得回读成原商无条件唯一。
-/
-- [gk:TrueCompleteClassification]
theorem recursive_gauge_boundary :
    (∀ (h : List Move) (cands : List Move) (c₁ c₂ : Move),
      (∀ t ∈ cands, Strict.Decomp.ValidDecomp h t) →
      Strict.Decomp.GaugeNormal cands c₁ →
      Strict.Decomp.GaugeNormal cands c₂ →
      c₁ = c₂ ∧ Strict.Decomp.ValidDecomp h c₁ ∧ Strict.Decomp.ValidDecomp h c₂) ∧
    (∃ (h : List Move) (t₁ t₂ : Move),
      Strict.Decomp.SummaryI t₁ = h ∧
      Strict.Decomp.SummaryI t₂ = h ∧
      Strict.Decomp.SummarySimN t₁ t₂ ∧
      ¬ Strict.Decomp.StructEqN t₁ t₂) :=
  ⟨recursiveGridEvidence.gaugeSectionUnique,
    recursiveGridEvidence.preGaugeAmbiguity⟩

/-! ## 3. 标准③：δ 忠实域 + 因果无前视证据 -/

structure CausalGridEvidence : Prop where
  deltaTotalUnique :
    ∀ (s : PositionState) (e : Side) (_h : Strict.Causal.ValidEvent s e),
      ∃ s' : PositionState,
        step s e = some s' ∧
        ∀ s'' : PositionState, step s e = some s'' → s'' = s'
  noLookahead :
    ∀ (ω ω' : Nat → LocatedEvent) (t : Nat),
      Strict.PrefixEq ω ω' t →
      Strict.Causal.Cstream ω t = Strict.Causal.Cstream ω' t
  factorsThroughPrefix :
    ∀ (ω : Nat → LocatedEvent) (t : Nat),
      Strict.Causal.Cstream ω t =
        run flat (Strict.Causal.prefixList ω (t + 1))
  lookaheadNegativeControl :
    ∃ (ω ω' : Nat → LocatedEvent) (t : Nat),
      Strict.PrefixEq ω ω' t ∧
      Strict.Causal.lookahead ω t ≠ Strict.Causal.lookahead ω' t

-- [gk:TrueCompleteClassification]
def causalGridEvidence : CausalGridEvidence where
  deltaTotalUnique := Strict.Causal.delta_existsUnique
  noLookahead := Strict.Causal.causal_no_lookahead
  factorsThroughPrefix := Strict.Causal.cstream_factors_through_prefix
  lookaheadNegativeControl := Strict.Causal.lookahead_not_causal

/-! ## 4. 标准⑤：未完成走势的当下状态与未来分支集 -/

structure OpenTailGridEvidence : Prop where
  branchTotal :
    ∀ h h' : Strict.OpenTail.OpenHist,
      Strict.OpenTail.Ext h h' →
      ∃ b : Strict.OpenTail.Branch, Strict.OpenTail.branchPred h b h'
  branchDisjoint :
    ∀ (h h' : Strict.OpenTail.OpenHist) (b₁ b₂ : Strict.OpenTail.Branch),
      Strict.OpenTail.branchPred h b₁ h' →
      Strict.OpenTail.branchPred h b₂ h' →
      b₁ = b₂
  branchSound :
    ∀ (h : Strict.OpenTail.OpenHist) (b : Strict.OpenTail.Branch)
      (h' : Strict.OpenTail.OpenHist),
      Strict.OpenTail.branchPred h b h' →
      Strict.OpenTail.Ext h h'
  currentUnique :
    ∀ h : Strict.OpenTail.OpenHist,
      ∃ s : Strict.OpenTail.OpenState,
        Strict.OpenTail.current h = s ∧
        ∀ s' : Strict.OpenTail.OpenState,
          Strict.OpenTail.current h = s' → s' = s

-- [gk:TrueCompleteClassification]
def openTailGridEvidence : OpenTailGridEvidence where
  branchTotal := Strict.OpenTail.branch_total
  branchDisjoint := Strict.OpenTail.branch_disjoint
  branchSound := Strict.OpenTail.branch_sound
  currentUnique := Strict.OpenTail.current_unique

/-- `chanlunOpenTail` 已把 total/disjoint/sound 三件写入标准结构字段。 -/
-- [gk:TrueCompleteClassification]
theorem chanlunOpenTail_fields_wired :
    (∀ h h',
      Strict.OpenTail.chanlunOpenTail.Ext h h' →
      ∃ b, Strict.OpenTail.chanlunOpenTail.BranchPred h b h') ∧
    (∀ h h' b₁ b₂,
      Strict.OpenTail.chanlunOpenTail.BranchPred h b₁ h' →
      Strict.OpenTail.chanlunOpenTail.BranchPred h b₂ h' →
      b₁ = b₂) ∧
    (∀ h b h',
      Strict.OpenTail.chanlunOpenTail.BranchPred h b h' →
      Strict.OpenTail.chanlunOpenTail.Ext h h') :=
  ⟨Strict.OpenTail.chanlunOpenTail.branch_total,
    Strict.OpenTail.chanlunOpenTail.branch_disjoint,
    Strict.OpenTail.chanlunOpenTail.branch_sound⟩

/-! ## 5. 标准⑥：π 完全应对，但只到操作语义级 -/

structure OperationalGridEvidence : Prop where
  total :
    ∀ s : Strict.Op.StrictState,
      ∃ a : Strict.StrictAction, Strict.Op.opStrategy.π s = a
  actionCoverage :
    ∀ s : Strict.Op.StrictState,
      Strict.Op.piStrict s = Strict.StrictAction.buy ∨
      Strict.Op.piStrict s = Strict.StrictAction.sell ∨
      Strict.Op.piStrict s = Strict.StrictAction.add ∨
      Strict.Op.piStrict s = Strict.StrictAction.reduce ∨
      Strict.Op.piStrict s = Strict.StrictAction.hold ∨
      Strict.Op.piStrict s = Strict.StrictAction.close ∨
      Strict.Op.piStrict s = Strict.StrictAction.wait
  waitRealized :
    Strict.Op.piStrict ⟨Strict.Op.Pos.flat, Strict.Op.Sig.none⟩ =
      Strict.StrictAction.wait
  quantityOutsideLayer2 :
    ∃ x y : Exec, I x = I y ∧ ¬ OpEqSemantic x y

-- [gk:OperationalSemanticsOnly]
def operationalGridEvidence : OperationalGridEvidence where
  total := Strict.Op.opStrategy_total
  actionCoverage := Strict.Op.piStrict_in_action
  waitRealized := Strict.Op.wait_realizable
  quantityOutsideLayer2 := Strict.Op.op_quantity_out_of_layer2

/-! ## 6. gatekeeper 类型级终态与总接线证书 -/

inductive Grid where
  | recursiveDecomposition
  | causalTransition
  | openTailBranches
  | operationalResponse
deriving DecidableEq, Repr

inductive GridGate where
  | TrueCompleteClassification
  | OperationalSemanticsOnly
deriving DecidableEq, Repr

/-- #661 逐格裁定的唯一机器映射。 -/
def gridGate : Grid → GridGate
  | .recursiveDecomposition => .TrueCompleteClassification
  | .causalTransition => .TrueCompleteClassification
  | .openTailBranches => .TrueCompleteClassification
  | .operationalResponse => .OperationalSemanticsOnly

structure GridGateCertificate : Prop where
  recursive :
    gridGate .recursiveDecomposition = .TrueCompleteClassification
  causal :
    gridGate .causalTransition = .TrueCompleteClassification
  openTail :
    gridGate .openTailBranches = .TrueCompleteClassification
  operational :
    gridGate .operationalResponse = .OperationalSemanticsOnly

def gridGateCertificate : GridGateCertificate where
  recursive := rfl
  causal := rfl
  openTail := rfl
  operational := rfl

/--
  #665 总接线证书。

  `semanticClassification` 把 C_Θ 的 RLevel 标签语义化；
  `fiberPartition` 回接原 `ClassificationFamily` 骨架；
  其余四字段分别引用标准②/③/⑤/⑥的既有系统。
-/
structure CThetaWiringCertificate : Prop where
  semanticClassification :
    Strict.Classifies
      Strict.LevelState.RContext
      Strict.LevelState.RLevel
      (rlevelCTheta.C FixedRTheta.certified)
      RLevelMeaning
  fiberPartition :
    (∀ ctx, ∃ label,
      Strict.ClassificationFamily.θFiber
        rlevelCTheta FixedRTheta.certified label ctx) ∧
    (∀ ctx (label₁ label₂ : Strict.LevelState.RLevel),
      Strict.ClassificationFamily.θFiber
          rlevelCTheta FixedRTheta.certified label₁ ctx →
      Strict.ClassificationFamily.θFiber
          rlevelCTheta FixedRTheta.certified label₂ ctx →
      label₁ = label₂)
  recursive : RecursiveGridEvidence
  causal : CausalGridEvidence
  openTail : OpenTailGridEvidence
  operational : OperationalGridEvidence
  gates : GridGateCertificate

/-- C_Θ 具体实例 + 四格引用链的最终机器见证。 -/
theorem cTheta_wiring_complete : CThetaWiringCertificate where
  semanticClassification := rlevelCTheta_classifies
  fiberPartition :=
    Strict.ClassificationFamily.theta_fiber_partition
      rlevelCTheta FixedRTheta.certified
  recursive := recursiveGridEvidence
  causal := causalGridEvidence
  openTail := openTailGridEvidence
  operational := operationalGridEvidence
  gates := gridGateCertificate

end Strict.CThetaWiring
