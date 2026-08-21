/-
Origin/ScanAssemblyBridge.lean — Rust↔Lean 统一扫描四件输出提取协议（#1087 第二切片）

本桥沿 EngineBridge/BspEventBridge 的模式：Rust 是被检实现，wire 标签与 wire record 独立于
Lean 镜像类型，再逐字段 decode。它不是 Rust 绑定，也不宣称尚未执行的窗口已经 0 mismatch。

提取器在每个 checkpoint、每一级必须输出 merged_scan_resume 当次返回的完整快照：
1. points：scan.rs:43-45，逐项覆盖 BspPoint 全字段；这里只含扫描产生的一/三类，禁止拿
   pipeline 后续追加二类的 LevelState.bsp 冒充；
2. panDivs：scan.rs:46 与 signal.rs:1150-1161 的全部字段；
3. grades：scan.rs:47 与 signal.rs:749-863 的全部字段；
4. observations：scan.rs:48、224-228 的扫描后归约+Pan 投影最终列表，不得输出 cached legs。

四件嵌套字段的定义锚同样按 main @ 355b839c29 核过：BspPoint/OwnerRef 在 bsp.rs:43-49、
139-206，Center/BspBits/ThirdClassEntryIdentity 在 types.rs:125-150、201-214，ForceFeatures/
ForceProxies 在 divergence.rs:417-440，CandidateKey/状态/结构谓词在 cand_event/key.rs:21-52、
87-137，CandidateObservation 与归约在 cand_event/observe.rs:24-44、84-145。

checkpoint 输出不是 delta，不暴露 cached_* 内部状态。末根也没有特殊 flush/finalize：它只是最后
一次相同扫描语义的完整快照，外层 isTerminalRoot 只标提取位置、不改变 decode。

★D2/P1 边界（scan.rs:19-28）：CandDeltaEvent 与 cp_ownership 不在四件输出内；本桥为它们提供
独立 wire 与逐字段检查。pan_div_diag 仍不改变 Cand 真值，但作为 19 字段事件的一员逐位核对。
c_p stable-revision 单调定理留在 ScanAssemblyMirror；完整 P2/c_p 附着另走 raw closure 桥。

本桥另把原始 rows/centers/blocks 交给 `recomputeRustPrelude`，不从 Rust prelude 输出回填 Lean；
CandDelta 给 event 前 raw 判据分量与 Rust 事件；c_p 给 before、原始 leave/retest 几何与 Rust after，
不存在 `accepted`/`closureWitness` 成品布尔输入。四输出只给生产扫描的逐段、未排序、未归约 sink，Lean 通过 `recomputeMergedOutput` 独立执行排序、同 key 腿归约、
Pan 投影和 FNV 身份。桥中没有 Rust legacy oracle，也不接收预制的 Lean 期望输出。
-/

import Origin.ScanAssemblyMirror

namespace NewChanlun.Origin.ScanAssemblyBridge

open NewChanlun.Origin.ScanAssemblyMirror

/-! ## §1 独立 Rust wire 标签 -/

inductive RustDirectionTag where | up | down
deriving DecidableEq, Repr

def RustDirectionTag.toLean : RustDirectionTag → Direction
  | RustDirectionTag.up => Direction.up
  | RustDirectionTag.down => Direction.down

inductive RustSideTag where | long | short
deriving DecidableEq, Repr

def RustSideTag.toLean : RustSideTag → Side
  | RustSideTag.long => Side.long
  | RustSideTag.short => Side.short

inductive RustCandidateKindTag where | trend | pan
deriving DecidableEq, Repr

def RustCandidateKindTag.toLean : RustCandidateKindTag → CandidateKind
  | RustCandidateKindTag.trend => CandidateKind.trend
  | RustCandidateKindTag.pan => CandidateKind.pan

inductive RustObservedStateTag where | provisional | unresolved | confirmed
deriving DecidableEq, Repr

def RustObservedStateTag.toLean : RustObservedStateTag → ObservedState
  | RustObservedStateTag.provisional => ObservedState.provisional
  | RustObservedStateTag.unresolved => ObservedState.unresolved
  | RustObservedStateTag.confirmed => ObservedState.confirmed

inductive RustCpLifecycleTag where | pending | closed
deriving DecidableEq, Repr

def RustCpLifecycleTag.toLean : RustCpLifecycleTag → CpLifecycle
  | RustCpLifecycleTag.pending => CpLifecycle.pending
  | RustCpLifecycleTag.closed => CpLifecycle.closed

inductive RustT3InCGradeReasonTag where
  | missingLeave | missingRetest | sameDirection | leaveNotOutside | retestReentered
deriving DecidableEq, Repr

def RustT3InCGradeReasonTag.toLean : RustT3InCGradeReasonTag → T3InCGradeReason
  | RustT3InCGradeReasonTag.missingLeave => T3InCGradeReason.missingLeave
  | RustT3InCGradeReasonTag.missingRetest => T3InCGradeReason.missingRetest
  | RustT3InCGradeReasonTag.sameDirection => T3InCGradeReason.sameDirection
  | RustT3InCGradeReasonTag.leaveNotOutside => T3InCGradeReason.leaveNotOutside
  | RustT3InCGradeReasonTag.retestReentered => T3InCGradeReason.retestReentered

/-! ## §2 公共 wire 值 -/

structure RustIntervalExtraction where
  left : Nat
  right : Nat
deriving DecidableEq, Repr

def RustIntervalExtraction.toLean (value : RustIntervalExtraction) : Interval :=
  (value.left, value.right)

structure RustCenterExtraction where
  zd : Int
  zg : Int
  dd : Int
  gg : Int
  startIndex : Nat
  endIndex : Nat
deriving DecidableEq, Repr

def RustCenterExtraction.toLean (center : RustCenterExtraction) : CenterFrame :=
  { zd := center.zd, zg := center.zg, dd := center.dd, gg := center.gg
    startIndex := center.startIndex, endIndex := center.endIndex }

/-- Rust prelude 排序后的 segment 行；方向锚按生产契约由本行 direction 唯一导出。 -/
structure RustSegmentExtraction where
  direction : RustDirectionTag
  startIndex : Nat
  endIndex : Nat
  startPrice : Int
  endPrice : Int
deriving DecidableEq, Repr

def RustSegmentExtraction.toLean (row : RustSegmentExtraction) : SegmentRow :=
  { direction := row.direction.toLean
    startIndex := row.startIndex
    endIndex := row.endIndex
    startPrice := row.startPrice
    endPrice := row.endPrice }

inductive RustMoveKindTag where | trend | consolidation
deriving DecidableEq, Repr

def RustMoveKindTag.toLean : RustMoveKindTag → MoveKind
  | RustMoveKindTag.trend => MoveKind.trend
  | RustMoveKindTag.consolidation => MoveKind.consolidation

structure RustMoveBlockExtraction where
  startCenter : Nat
  endCenter : Nat
  kind : RustMoveKindTag
  direction : Option RustDirectionTag
  levelLift : Nat
deriving DecidableEq, Repr

def RustMoveBlockExtraction.toLean (block : RustMoveBlockExtraction) : MoveBlockFrame :=
  { startCenter := block.startCenter, endCenter := block.endCenter, kind := block.kind.toLean
    direction := block.direction.map RustDirectionTag.toLean, levelLift := block.levelLift }

structure RustASegmentEnvelopeExtraction where
  span : RustIntervalExtraction
  low : Int
  high : Int
deriving DecidableEq, Repr

def RustASegmentEnvelopeExtraction.toLean
    (value : RustASegmentEnvelopeExtraction) : ASegmentEnvelope :=
  { span := value.span.toLean, bEnvelope := (value.low, value.high) }

structure RustPreludeInputExtraction where
  rows : List RustSegmentExtraction
  centers : List RustCenterExtraction
  blocks : List RustMoveBlockExtraction
deriving DecidableEq, Repr

structure RustPreludeOutputExtraction where
  segmentsSorted : Bool
  centersSorted : Bool
  anchors : List (Option RustDirectionTag)
  trendGate : List (Option RustDirectionTag)
  firstMatchIdx : List (Option Nat)
  aSegments : List (Option RustASegmentEnvelopeExtraction)
deriving DecidableEq, Repr

def RustPreludeOutputExtraction.toLean (output : RustPreludeOutputExtraction) : PreludeOutput :=
  { segmentsSorted := output.segmentsSorted, centersSorted := output.centersSorted
    anchors := output.anchors.map (Option.map RustDirectionTag.toLean)
    trendGate := output.trendGate.map (Option.map RustDirectionTag.toLean)
    firstMatchIdx := output.firstMatchIdx
    aSegments := output.aSegments.map (Option.map RustASegmentEnvelopeExtraction.toLean) }

def recomputeRustPrelude (input : RustPreludeInputExtraction) : PreludeOutput :=
  recomputePrelude (input.rows.map RustSegmentExtraction.toLean)
    (input.centers.map RustCenterExtraction.toLean)
    (input.blocks.map RustMoveBlockExtraction.toLean)

def PreludeParity (rust : RustPreludeOutputExtraction) (lean : PreludeOutput) : Prop :=
  rust.toLean = lean

instance preludeParityDecidable (rust : RustPreludeOutputExtraction) (lean : PreludeOutput) :
    Decidable (PreludeParity rust lean) := by
  unfold PreludeParity
  infer_instance

/-! ## §3 event / cp_ownership 侧车 -/

inductive RustCandDeltaKindTag where
  | trend
  | pan
deriving DecidableEq, Repr

def RustCandDeltaKindTag.toLean : RustCandDeltaKindTag → CandDeltaKind
  | RustCandDeltaKindTag.trend => CandDeltaKind.trend
  | RustCandDeltaKindTag.pan => CandDeltaKind.pan

structure RustElementIdentityExtraction where
  level : Nat
  ordinal : Nat
deriving DecidableEq, Repr

def RustElementIdentityExtraction.toLean (value : RustElementIdentityExtraction) : ElementIdentity :=
  { level := value.level, ordinal := value.ordinal }

structure RustParentCenterIdentityExtraction where
  centerIndex : Nat
  centerId : RustElementIdentityExtraction
  sourceInterval : RustIntervalExtraction
  zd : Int
  zg : Int
deriving DecidableEq, Repr

def RustParentCenterIdentityExtraction.toLean
    (value : RustParentCenterIdentityExtraction) : ParentCenterIdentity :=
  { centerIndex := value.centerIndex, centerId := value.centerId.toLean
    sourceInterval := value.sourceInterval.toLean, zd := value.zd, zg := value.zg }

structure RustCpStructureIdentityExtraction where
  level : Nat
  bCenterId : RustElementIdentityExtraction
  departureMoveId : RustElementIdentityExtraction
  terminalMoveId : Option RustElementIdentityExtraction
  sourceStart : Nat
  sourceEnd : Option Nat
deriving DecidableEq, Repr

def RustCpStructureIdentityExtraction.toLean
    (value : RustCpStructureIdentityExtraction) : CpStructureIdentity :=
  { level := value.level, bCenterId := value.bCenterId.toLean
    departureMoveId := value.departureMoveId.toLean
    terminalMoveId := value.terminalMoveId.map RustElementIdentityExtraction.toLean
    sourceStart := value.sourceStart, sourceEnd := value.sourceEnd }

structure RustThirdClassInCpExtraction where
  bCenterId : RustElementIdentityExtraction
  cpDepartureMoveId : RustElementIdentityExtraction
  departureMoveId : RustElementIdentityExtraction
  retestMoveId : RustElementIdentityExtraction
  departureInterval : RustIntervalExtraction
  retestInterval : RustIntervalExtraction
  pointSourceIndex : Nat
  side : RustSideTag
deriving DecidableEq, Repr

def RustThirdClassInCpExtraction.toLean (value : RustThirdClassInCpExtraction) : ThirdClassInCp :=
  { bCenterId := value.bCenterId.toLean, cpDepartureMoveId := value.cpDepartureMoveId.toLean
    departureMoveId := value.departureMoveId.toLean, retestMoveId := value.retestMoveId.toLean
    departureInterval := value.departureInterval.toLean, retestInterval := value.retestInterval.toLean
    pointSourceIndex := value.pointSourceIndex, side := value.side.toLean }

structure RustTrendContextExtraction where
  predecessorCenterId : RustElementIdentityExtraction
  bCenterId : RustElementIdentityExtraction
  direction : RustDirectionTag
deriving DecidableEq, Repr

def RustTrendContextExtraction.toLean (value : RustTrendContextExtraction) : TrendContext :=
  { predecessorCenterId := value.predecessorCenterId.toLean, bCenterId := value.bCenterId.toLean
    direction := value.direction.toLean }

structure RustNewExtremeExtraction where
  bCenterId : RustElementIdentityExtraction
  direction : RustDirectionTag
  referencePrice : Int
  extremePrice : Int
  extremeMoveId : RustElementIdentityExtraction
  confirmSrc : Nat
deriving DecidableEq, Repr

def RustNewExtremeExtraction.toLean (value : RustNewExtremeExtraction) : NewExtremeInDirection :=
  { bCenterId := value.bCenterId.toLean, direction := value.direction.toLean
    referencePrice := value.referencePrice, extremePrice := value.extremePrice
    extremeMoveId := value.extremeMoveId.toLean, confirmSrc := value.confirmSrc }

structure RustInternalCentersExtraction where
  cLevel : Nat
  centerIds : List RustElementIdentityExtraction
deriving DecidableEq, Repr

def RustInternalCentersExtraction.toLean
    (value : RustInternalCentersExtraction) : InternalSublevelCenters :=
  { cLevel := value.cLevel, centerIds := value.centerIds.map RustElementIdentityExtraction.toLean }

structure RustCompletedTrendExtraction where
  direction : RustDirectionTag
  centerIds : List RustElementIdentityExtraction
  closingSuccessorMoveId : RustElementIdentityExtraction
  confirmSrc : Nat
deriving DecidableEq, Repr

def RustCompletedTrendExtraction.toLean
    (value : RustCompletedTrendExtraction) : CompletedTrendDecomposition :=
  { direction := value.direction.toLean
    centerIds := value.centerIds.map RustElementIdentityExtraction.toLean
    closingSuccessorMoveId := value.closingSuccessorMoveId.toLean
    confirmSrc := value.confirmSrc }

structure RustFullTrendEvidenceExtraction where
  trendContext : Option RustTrendContextExtraction
  newExtremeInDirection : Option RustNewExtremeExtraction
  internalSublevelCenters : Option RustInternalCentersExtraction
  completedTrendDecomposition : Option RustCompletedTrendExtraction
  decompositionReviewMoveId : Option RustElementIdentityExtraction
  decompositionReviewSrc : Option Nat
deriving DecidableEq, Repr

def RustFullTrendEvidenceExtraction.toLean
    (value : RustFullTrendEvidenceExtraction) : FullTrendQualificationEvidence :=
  { trendContext := value.trendContext.map RustTrendContextExtraction.toLean
    newExtremeInDirection := value.newExtremeInDirection.map RustNewExtremeExtraction.toLean
    internalSublevelCenters := value.internalSublevelCenters.map RustInternalCentersExtraction.toLean
    completedTrendDecomposition := value.completedTrendDecomposition.map RustCompletedTrendExtraction.toLean
    decompositionReviewMoveId := value.decompositionReviewMoveId.map RustElementIdentityExtraction.toLean
    decompositionReviewSrc := value.decompositionReviewSrc }

structure RustFullTrendQualifiedExtraction where
  trendContext : RustTrendContextExtraction
  thirdClassInsideC : RustThirdClassInCpExtraction
  newExtremeInDirection : RustNewExtremeExtraction
  internalSublevelCenters : RustInternalCentersExtraction
  completedTrendDecomposition : RustCompletedTrendExtraction
  confirmSrc : Nat
deriving DecidableEq, Repr

def RustFullTrendQualifiedExtraction.toLean
    (value : RustFullTrendQualifiedExtraction) : FullTrendCQualified :=
  { trendContext := value.trendContext.toLean, thirdClassInsideC := value.thirdClassInsideC.toLean
    newExtremeInDirection := value.newExtremeInDirection.toLean
    internalSublevelCenters := value.internalSublevelCenters.toLean
    completedTrendDecomposition := value.completedTrendDecomposition.toLean
    confirmSrc := value.confirmSrc }

structure RustCandDeltaCpEdgeExtraction where
  bCenterId : RustElementIdentityExtraction
  cpDepartureMoveId : RustElementIdentityExtraction
  cpSourceStart : Nat
deriving DecidableEq, Repr

def RustCandDeltaCpEdgeExtraction.toLean (value : RustCandDeltaCpEdgeExtraction) : CandDeltaCpEdge :=
  { bCenterId := value.bCenterId.toLean, cpDepartureMoveId := value.cpDepartureMoveId.toLean
    cpSourceStart := value.cpSourceStart }

structure RustPredicateExtraction where
  kind : RustCandDeltaKindTag
  structuralCandidate : Bool
  direction : Bool
  comparable : Bool
  extreme : Bool
  buy1 : Bool
  sell1 : Bool
  panDiverges : Bool
deriving DecidableEq, Repr

def RustPredicateExtraction.toLean (facts : RustPredicateExtraction) : CandDeltaFacts :=
  { kind := facts.kind.toLean, structuralCandidate := facts.structuralCandidate
    direction := facts.direction, comparable := facts.comparable, extreme := facts.extreme
    buy1 := facts.buy1, sell1 := facts.sell1, panDiverges := facts.panDiverges }

structure RustAssemblyInputExtraction where
  level : Nat
  side : RustSideTag
  divergenceConfirmSrc : Nat
  aIntervalLeft : Nat
  aIntervalRight : Nat
  center : RustCenterExtraction
  departureDir : RustDirectionTag
  untilStart : Nat
  triggerEnd : Nat
  rows : List RustSegmentExtraction
  predicate : RustPredicateExtraction
  cIntervalFull : Option RustIntervalExtraction
  bParent : Option RustParentCenterIdentityExtraction
  cStructure : Option RustCpStructureIdentityExtraction
  thirdClassInC : Option RustThirdClassInCpExtraction
  fullTrendCQualified : Option RustFullTrendQualifiedExtraction
  fullTrendEvidence : Option RustFullTrendEvidenceExtraction
  cpOwnership : Option RustCandDeltaCpEdgeExtraction
  panDivDiag : Bool
deriving DecidableEq, Repr

def RustAssemblyInputExtraction.toLean (input : RustAssemblyInputExtraction) : EventAssemblyInput :=
  { level := input.level, side := input.side.toLean
    divergenceConfirmSrc := input.divergenceConfirmSrc
    aInterval := { left := input.aIntervalLeft, right := input.aIntervalRight }
    center := input.center.toLean, departureDir := input.departureDir.toLean
    untilStart := input.untilStart, triggerEnd := input.triggerEnd
    rows := input.rows.map RustSegmentExtraction.toLean, predicate := input.predicate.toLean
    cIntervalFull := input.cIntervalFull.map RustIntervalExtraction.toLean
    bParent := input.bParent.map RustParentCenterIdentityExtraction.toLean
    cStructure := input.cStructure.map RustCpStructureIdentityExtraction.toLean
    thirdClassInC := input.thirdClassInC.map RustThirdClassInCpExtraction.toLean
    fullTrendCQualified := input.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean
    fullTrendEvidence := input.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean
    cpOwnership := input.cpOwnership.map RustCandDeltaCpEdgeExtraction.toLean
    panDivDiag := input.panDivDiag }

structure RustEventExtraction where
  level : Nat
  side : RustSideTag
  divergenceConfirmSrc : Nat
  confirmSrc : Nat
  intervalLeft : Nat
  intervalRight : Nat
  aIntervalLeft : Nat
  aIntervalRight : Nat
  cEpisodeStart : Nat
  cEpisodeLeft : Nat
  cEpisodeRight : Nat
  cIntervalFull : Option RustIntervalExtraction
  bParent : Option RustParentCenterIdentityExtraction
  cStructure : Option RustCpStructureIdentityExtraction
  thirdClassInC : Option RustThirdClassInCpExtraction
  cpCertificateConfirmSrc : Option Nat
  fullTrendCQualified : Option RustFullTrendQualifiedExtraction
  fullTrendEvidence : Option RustFullTrendEvidenceExtraction
  cpOwnership : Option RustCandDeltaCpEdgeExtraction
  enterSrc : Nat
  candDelta : Bool
  panDivDiag : Bool
deriving DecidableEq, Repr

/-- 19 个生产字段逐项比对；嵌套证据也走完整结构等式。 -/
def EventParity (rust : RustEventExtraction) (lean : CandDeltaEvent) : Prop :=
  rust.level = lean.level ∧ rust.side.toLean = lean.side ∧
  rust.divergenceConfirmSrc = lean.divergenceConfirmSrc ∧ rust.confirmSrc = lean.confirmSrc ∧
  rust.intervalLeft = lean.interval.left ∧ rust.intervalRight = lean.interval.right ∧
  rust.aIntervalLeft = lean.aInterval.left ∧ rust.aIntervalRight = lean.aInterval.right ∧
  rust.cEpisodeStart = lean.cEpisodeStart ∧ rust.cEpisodeLeft = lean.cEpisodeInterval.left ∧
  rust.cEpisodeRight = lean.cEpisodeInterval.right ∧
  rust.cIntervalFull.map RustIntervalExtraction.toLean = lean.cIntervalFull ∧
  rust.bParent.map RustParentCenterIdentityExtraction.toLean = lean.bParent ∧
  rust.cStructure.map RustCpStructureIdentityExtraction.toLean = lean.cStructure ∧
  rust.thirdClassInC.map RustThirdClassInCpExtraction.toLean = lean.thirdClassInC ∧
  rust.cpCertificateConfirmSrc = lean.cpCertificateConfirmSrc ∧
  rust.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean = lean.fullTrendCQualified ∧
  rust.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean = lean.fullTrendEvidence ∧
  rust.cpOwnership.map RustCandDeltaCpEdgeExtraction.toLean = lean.cpOwnership ∧
  rust.enterSrc = lean.enterSrc ∧ rust.candDelta = lean.candDelta ∧
  rust.panDivDiag = lean.panDivDiag

instance eventParityDecidable (rust : RustEventExtraction) (lean : CandDeltaEvent) :
    Decidable (EventParity rust lean) := by
  unfold EventParity
  infer_instance

/-- raw 判据由 Lean 决定产/拒；Rust 事件只在右侧作为被检输出。 -/
def EventCheck (rustInput : RustAssemblyInputExtraction)
    (rustEvent : Option RustEventExtraction) : Prop :=
  match rustEvent, assembleCandDelta rustInput.toLean with
  | none, none => True
  | some rust, some lean => EventParity rust lean
  | _, _ => False

instance eventCheckDecidable (rustInput : RustAssemblyInputExtraction)
    (rustEvent : Option RustEventExtraction) : Decidable (EventCheck rustInput rustEvent) := by
  cases rustEvent <;> cases hLean : assembleCandDelta rustInput.toLean <;>
    simp only [EventCheck, hLean] <;> infer_instance

/-- rejected raw case 的结构槽；判据真假不参与，防止 event 被另一 accepted case 消费后假绿。 -/
def sameRawEventSlot (input : RustAssemblyInputExtraction) (rust : RustEventExtraction) : Bool :=
  let basic :=
    rust.level == input.level && rust.side.toLean == input.side.toLean &&
    rust.divergenceConfirmSrc == input.divergenceConfirmSrc &&
    rust.confirmSrc == input.divergenceConfirmSrc &&
    rust.aIntervalLeft == input.aIntervalLeft && rust.aIntervalRight == input.aIntervalRight
  match episodeBounds input.toLean.rows input.toLean.center input.toLean.departureDir
      input.untilStart input.triggerEnd with
  | some bounds =>
      basic && rust.intervalLeft == bounds.left && rust.intervalRight == bounds.right &&
      rust.cEpisodeStart == bounds.left && rust.cEpisodeLeft == bounds.left &&
      rust.cEpisodeRight == bounds.right && rust.enterSrc == bounds.left
  | none => basic && rust.intervalRight == input.triggerEnd

/-- 从生产 event 列表中恰好消费一个与 Lean 重算 event 全字段相等的元素。 -/
def consumeEvent (expected : CandDeltaEvent) :
    List RustEventExtraction → Option (List RustEventExtraction)
  | [] => none
  | rust :: rest =>
      if EventParity rust expected then some rest
      else (consumeEvent expected rest).map (fun remaining => rust :: remaining)

/-- 所有 rejected raw case 都必须在原始 production event 列表中找不到同一结构槽。 -/
def rejectedRawCasesHaveNoEvents (inputs : List RustAssemblyInputExtraction)
    (rustEvents : List RustEventExtraction) : Bool :=
  inputs.all fun input =>
    match assembleCandDelta input.toLean with
    | none => !(rustEvents.any (sameRawEventSlot input))
    | some _ => true

/--
列表级 fail-closed 双射门：accepted raw case 各消费一个 event，rejected case 不消费；所有 raw case
处理完成后生产 event 列表必须为空。因此重复、额外、漏产和一份 event 被多 case 共用都失败。
-/
def eventBijectionCheckBool :
    List RustAssemblyInputExtraction → List RustEventExtraction → Bool
  | [], rustEvents => rustEvents.isEmpty
  | input :: inputRest, rustEvents =>
      match assembleCandDelta input.toLean with
      | none =>
          if rustEvents.any (sameRawEventSlot input) then false
          else eventBijectionCheckBool inputRest rustEvents
      | some expected =>
          match consumeEvent expected rustEvents with
          | none => false
          | some remaining => eventBijectionCheckBool inputRest remaining

/-- Production CandDelta event 列表与 event 前 raw case 列表的 fail-closed 双射。 -/
def EventBijectionCheck (inputs : List RustAssemblyInputExtraction)
    (rustEvents : List RustEventExtraction) : Prop :=
  rustEvents.Nodup ∧ rejectedRawCasesHaveNoEvents inputs rustEvents = true ∧
  eventBijectionCheckBool inputs rustEvents = true

instance eventBijectionCheckDecidable (inputs : List RustAssemblyInputExtraction)
    (rustEvents : List RustEventExtraction) : Decidable (EventBijectionCheck inputs rustEvents) := by
  unfold EventBijectionCheck
  infer_instance

structure RustCpObjectExtraction where
  bCenterIndex : Nat
  bCenterId : RustElementIdentityExtraction
  bCenter : RustCenterExtraction
  departureMoveId : Option RustElementIdentityExtraction
  departureInterval : Option RustIntervalExtraction
  lifecycle : RustCpLifecycleTag
  cpCertificateConfirmSrc : Option Nat
  cStructure : Option RustCpStructureIdentityExtraction
  thirdClassInC : Option RustThirdClassInCpExtraction
  fullTrendEvidence : Option RustFullTrendEvidenceExtraction
  fullTrendCQualified : Option RustFullTrendQualifiedExtraction
deriving DecidableEq, Repr

def RustCpObjectExtraction.toLean (value : RustCpObjectExtraction) : CpObject :=
  { bCenterIndex := value.bCenterIndex, bCenterId := value.bCenterId.toLean
    bCenter := value.bCenter.toLean
    departureMoveId := value.departureMoveId.map RustElementIdentityExtraction.toLean
    departureInterval := value.departureInterval.map RustIntervalExtraction.toLean
    lifecycle := value.lifecycle.toLean, cpCertificateConfirmSrc := value.cpCertificateConfirmSrc
    cStructure := value.cStructure.map RustCpStructureIdentityExtraction.toLean
    thirdClassInC := value.thirdClassInC.map RustThirdClassInCpExtraction.toLean
    fullTrendEvidence := value.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean
    fullTrendCQualified := value.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean }

structure RustCpClosureEvidenceExtraction where
  cpDepartureMoveId : RustElementIdentityExtraction
  cpStart : Nat
  leave : RustSegmentExtraction
  retest : RustSegmentExtraction
  leaveAnchor : Option RustDirectionTag
  leaveMoveId : RustElementIdentityExtraction
  retestMoveId : RustElementIdentityExtraction
  fullTrendEvidence : Option RustFullTrendEvidenceExtraction
  fullTrendCQualified : Option RustFullTrendQualifiedExtraction
deriving DecidableEq, Repr

def RustCpClosureEvidenceExtraction.toLean
    (value : RustCpClosureEvidenceExtraction) : CpClosureEvidence :=
  { cpDepartureMoveId := value.cpDepartureMoveId.toLean, cpStart := value.cpStart
    leave := value.leave.toLean, retest := value.retest.toLean
    leaveAnchor := value.leaveAnchor.map RustDirectionTag.toLean
    leaveMoveId := value.leaveMoveId.toLean, retestMoveId := value.retestMoveId.toLean
    fullTrendEvidence := value.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean
    fullTrendCQualified := value.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean }

def RecomputeCpClosureCheck (rustBefore rustAfter : RustCpObjectExtraction)
    (raw : RustCpClosureEvidenceExtraction) : Prop :=
  rustAfter.toLean = closeCpFromRaw rustBefore.toLean raw.toLean

instance recomputeCpClosureCheckDecidable (rustBefore rustAfter : RustCpObjectExtraction)
    (raw : RustCpClosureEvidenceExtraction) :
    Decidable (RecomputeCpClosureCheck rustBefore rustAfter raw) := by
  unfold RecomputeCpClosureCheck
  infer_instance

/-! ## §4 points / BspPoint -/

structure RustThirdClassEntryExtraction where
  centerSi : Nat
  centerZd : Int
  centerZg : Int
  leaveInterval : RustIntervalExtraction
  retestInterval : RustIntervalExtraction
deriving DecidableEq, Repr

def RustThirdClassEntryExtraction.toLean
    (entry : RustThirdClassEntryExtraction) : ThirdClassEntryIdentity :=
  { centerSi := entry.centerSi, centerZd := entry.centerZd, centerZg := entry.centerZg
    leaveInterval := entry.leaveInterval.toLean, retestInterval := entry.retestInterval.toLean }

structure RustBspBitsExtraction where
  buy1 : Bool
  buy2 : Bool
  buy3 : Bool
  sell1 : Bool
  sell2 : Bool
  sell3 : Bool
  thirdClassEntry : Option RustThirdClassEntryExtraction
deriving DecidableEq, Repr

def RustBspBitsExtraction.toLean (bits : RustBspBitsExtraction) : BspBits :=
  { buy1 := bits.buy1, buy2 := bits.buy2, buy3 := bits.buy3
    sell1 := bits.sell1, sell2 := bits.sell2, sell3 := bits.sell3
    thirdClassEntry := bits.thirdClassEntry.map RustThirdClassEntryExtraction.toLean }

inductive RustOwnerExtraction where
  | center (value : RustCenterExtraction)
  | type1Anchor (sourceIndex : Nat)
deriving DecidableEq, Repr

def RustOwnerExtraction.toLean : RustOwnerExtraction → OwnerRef
  | RustOwnerExtraction.center value => OwnerRef.center value.toLean
  | RustOwnerExtraction.type1Anchor sourceIndex => OwnerRef.type1Anchor sourceIndex

/-- f64 字段必须由 Rust `to_bits()` 输出为无损自然数，不走十进制字符串。 -/
structure RustForceFeaturesExtraction where
  macdAreaBits : Nat
  difPeakBits : Nat
  priceAmplitude : Int
  priceSpeedBits : Nat
deriving DecidableEq, Repr

def RustForceFeaturesExtraction.toLean
    (features : RustForceFeaturesExtraction) : ForceFeatures :=
  { macdAreaBits := features.macdAreaBits, difPeakBits := features.difPeakBits
    priceAmplitude := features.priceAmplitude, priceSpeedBits := features.priceSpeedBits }

structure RustForceProxiesExtraction where
  segA : RustForceFeaturesExtraction
  segC : RustForceFeaturesExtraction
deriving DecidableEq, Repr

def RustForceProxiesExtraction.toLean (force : RustForceProxiesExtraction) : ForceProxies :=
  { segA := force.segA.toLean, segC := force.segC.toLean }

/-- force 虽不进 Rust 手写 PartialEq，仍必须提取。 -/
structure RustBspPointExtraction where
  sourceIndex : Nat
  bits : RustBspBitsExtraction
  pivotLow : Int
  pivotHigh : Int
  center : Option RustOwnerExtraction
  structBreakDir : Option RustSideTag
  force : Option RustForceProxiesExtraction
  retraceBreaksType1 : Option Bool
deriving DecidableEq, Repr

def RustBspPointExtraction.toLean (point : RustBspPointExtraction) : BspPoint :=
  { sourceIndex := point.sourceIndex, bits := point.bits.toLean
    pivotLow := point.pivotLow, pivotHigh := point.pivotHigh
    center := point.center.map RustOwnerExtraction.toLean
    structBreakDir := point.structBreakDir.map RustSideTag.toLean
    force := point.force.map RustForceProxiesExtraction.toLean
    retraceBreaksType1 := point.retraceBreaksType1 }

/-! ## §5 panDivs / PanDivCert -/

structure RustPanDivCertExtraction where
  sourceIndex : Nat
  side : RustSideTag
  center : RustCenterExtraction
  segA : RustIntervalExtraction
  segC : RustIntervalExtraction
deriving DecidableEq, Repr

def RustPanDivCertExtraction.toLean (cert : RustPanDivCertExtraction) : PanDivCert :=
  { sourceIndex := cert.sourceIndex, side := cert.side.toLean, center := cert.center.toLean
    segA := cert.segA.toLean, segC := cert.segC.toLean }

/-! ## §6 grades / FirstClassGradeRecord -/

inductive RustT3InCGradeExtraction where
  | present (leaveInterval retestInterval : RustIntervalExtraction)
  | missing (reason : RustT3InCGradeReasonTag)
deriving DecidableEq, Repr

def RustT3InCGradeExtraction.toLean : RustT3InCGradeExtraction → T3InCGrade
  | RustT3InCGradeExtraction.present leaveInterval retestInterval =>
      T3InCGrade.present leaveInterval.toLean retestInterval.toLean
  | RustT3InCGradeExtraction.missing reason => T3InCGrade.missing reason.toLean

structure RustFirstClassGradeExtraction where
  level : Nat
  sourceIndex : Nat
  side : RustSideTag
  centerStartIndex : Nat
  centerEndIndex : Nat
  centerZd : Int
  centerZg : Int
  grade : RustT3InCGradeExtraction
deriving DecidableEq, Repr

def RustFirstClassGradeExtraction.toLean
    (record : RustFirstClassGradeExtraction) : FirstClassGradeRecord :=
  { level := record.level, sourceIndex := record.sourceIndex, side := record.side.toLean
    centerStartIndex := record.centerStartIndex, centerEndIndex := record.centerEndIndex
    centerZd := record.centerZd, centerZg := record.centerZg, grade := record.grade.toLean }

/-! ## §7 observations / CandidateObservation -/

structure RustParentFingerprintExtraction where
  centerStart : Nat
  zd : Int
  zg : Int
deriving DecidableEq, Repr

def RustParentFingerprintExtraction.toLean
    (parent : RustParentFingerprintExtraction) : ParentFingerprint :=
  { centerStart := parent.centerStart, zd := parent.zd, zg := parent.zg }

structure RustCandidateKeyExtraction where
  ruleVersion : Nat
  level : Nat
  kind : RustCandidateKindTag
  side : RustSideTag
  previousCenterStart : Option Nat
  parent : RustParentFingerprintExtraction
  segA : RustIntervalExtraction
  cStart : Nat
deriving DecidableEq, Repr

def RustCandidateKeyExtraction.toLean (key : RustCandidateKeyExtraction) : CandidateKey :=
  { ruleVersion := key.ruleVersion, level := key.level, kind := key.kind.toLean
    side := key.side.toLean, previousCenterStart := key.previousCenterStart
    parent := key.parent.toLean, segA := key.segA.toLean, cStart := key.cStart }

structure RustStructuralPredicatesExtraction where
  direction : Bool
  comparable : Bool
  extreme : Bool
deriving DecidableEq, Repr

def RustStructuralPredicatesExtraction.toLean
    (predicates : RustStructuralPredicatesExtraction) : StructuralPredicates :=
  { direction := predicates.direction, comparable := predicates.comparable
    extreme := predicates.extreme }

structure RustCandidateObservationExtraction where
  key : RustCandidateKeyExtraction
  kind : RustCandidateKindTag
  centerIds : Option RustIntervalExtraction
  candidateGroupId : Nat
  pairId : Nat
  structuralPredicates : RustStructuralPredicatesExtraction
  extremeProof : RustIntervalExtraction
  thirdClassProof : Option Nat
  interval : RustIntervalExtraction
  state : RustObservedStateTag
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def RustCandidateObservationExtraction.toLean
    (observation : RustCandidateObservationExtraction) : CandidateObservation :=
  { key := observation.key.toLean, kind := observation.kind.toLean
    centerIds := observation.centerIds.map RustIntervalExtraction.toLean
    candidateGroupId := observation.candidateGroupId, pairId := observation.pairId
    structuralPredicates := observation.structuralPredicates.toLean
    extremeProof := observation.extremeProof.toLean
    thirdClassProof := observation.thirdClassProof, interval := observation.interval.toLean
    state := observation.state.toLean, firstProvableAt := observation.firstProvableAt
    confirmedAt := observation.confirmedAt }

/-- raw sink 腿不含 Rust 派生 ID；ID 只在 Lean post-scan 由 key 生成。 -/
structure RustCandidateLegExtraction where
  key : RustCandidateKeyExtraction
  kind : RustCandidateKindTag
  centerIds : Option RustIntervalExtraction
  structuralPredicates : RustStructuralPredicatesExtraction
  extremeProof : RustIntervalExtraction
  thirdClassProof : Option Nat
  interval : RustIntervalExtraction
  state : RustObservedStateTag
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def RustCandidateLegExtraction.toLean
    (leg : RustCandidateLegExtraction) : CandidateLeg :=
  { key := leg.key.toLean, kind := leg.kind.toLean
    centerIds := leg.centerIds.map RustIntervalExtraction.toLean
    structuralPredicates := leg.structuralPredicates.toLean
    extremeProof := leg.extremeProof.toLean, thirdClassProof := leg.thirdClassProof
    interval := leg.interval.toLean, state := leg.state.toLean
    firstProvableAt := leg.firstProvableAt, confirmedAt := leg.confirmedAt }

/-- Rust 生产扫描逐段发出的未排序、未归约 sink；不是 legacy oracle 或最终输出。 -/
structure RustScanSinkEmissionExtraction where
  bspPoints : List RustBspPointExtraction
  panDivCerts : List RustPanDivCertExtraction
  firstClassGrades : List RustFirstClassGradeExtraction
  candidateLegs : List RustCandidateLegExtraction
deriving DecidableEq, Repr

def RustScanSinkEmissionExtraction.toLean
    (emission : RustScanSinkEmissionExtraction) : ScanSinkEmission :=
  { bspPoints := emission.bspPoints.map RustBspPointExtraction.toLean
    panDivCerts := emission.panDivCerts.map RustPanDivCertExtraction.toLean
    firstClassGrades := emission.firstClassGrades.map RustFirstClassGradeExtraction.toLean
    candidateLegs := emission.candidateLegs.map RustCandidateLegExtraction.toLean }

def recomputeRustScanSinkEmissions (level : Nat)
    (emissions : List RustScanSinkEmissionExtraction) : MergedScanOutput :=
  recomputeMergedOutput level (emissions.map RustScanSinkEmissionExtraction.toLean)

/-! ## §8 checkpoint / 末根完整快照 -/

structure RustMergedScanOutputExtraction where
  points : List RustBspPointExtraction
  panDivs : List RustPanDivCertExtraction
  grades : List RustFirstClassGradeExtraction
  observations : List RustCandidateObservationExtraction
deriving DecidableEq, Repr

def RustMergedScanOutputExtraction.toLean
    (output : RustMergedScanOutputExtraction) : MergedScanOutput :=
  { points := output.points.map RustBspPointExtraction.toLean
    panDivs := output.panDivs.map RustPanDivCertExtraction.toLean
    grades := output.grades.map RustFirstClassGradeExtraction.toLean
    observations := output.observations.map RustCandidateObservationExtraction.toLean }

structure ScanSnapshot where
  checkpointSrc : Nat
  level : Nat
  isTerminalRoot : Bool
  output : MergedScanOutput
deriving DecidableEq, Repr

structure RustScanSnapshotExtraction where
  checkpointSrc : Nat
  level : Nat
  isTerminalRoot : Bool
  output : RustMergedScanOutputExtraction
deriving DecidableEq, Repr

def RustScanSnapshotExtraction.toLean (snapshot : RustScanSnapshotExtraction) : ScanSnapshot :=
  { checkpointSrc := snapshot.checkpointSrc, level := snapshot.level
    isTerminalRoot := snapshot.isTerminalRoot, output := snapshot.output.toLean }

/-- points 单项逐字段桥；所有嵌套值均走 `DecidableEq`，不使用 Rust 手写 PartialEq。 -/
def PointParity (rust : RustBspPointExtraction) (lean : BspPoint) : Bool :=
  decide
    (rust.sourceIndex = lean.sourceIndex ∧
      rust.bits.toLean = lean.bits ∧
      rust.pivotLow = lean.pivotLow ∧
      rust.pivotHigh = lean.pivotHigh ∧
      rust.center.map RustOwnerExtraction.toLean = lean.center ∧
      rust.structBreakDir.map RustSideTag.toLean = lean.structBreakDir ∧
      rust.force.map RustForceProxiesExtraction.toLean = lean.force ∧
      rust.retraceBreaksType1 = lean.retraceBreaksType1)

/-- panDivs 单项逐字段桥。 -/
def PanDivParity (rust : RustPanDivCertExtraction) (lean : PanDivCert) : Bool :=
  decide
    (rust.sourceIndex = lean.sourceIndex ∧
      rust.side.toLean = lean.side ∧
      rust.center.toLean = lean.center ∧
      rust.segA.toLean = lean.segA ∧
      rust.segC.toLean = lean.segC)

/-- grades 单项逐字段桥。 -/
def GradeParity (rust : RustFirstClassGradeExtraction) (lean : FirstClassGradeRecord) : Bool :=
  decide
    (rust.level = lean.level ∧
      rust.sourceIndex = lean.sourceIndex ∧
      rust.side.toLean = lean.side ∧
      rust.centerStartIndex = lean.centerStartIndex ∧
      rust.centerEndIndex = lean.centerEndIndex ∧
      rust.centerZd = lean.centerZd ∧
      rust.centerZg = lean.centerZg ∧
      rust.grade.toLean = lean.grade)

/-- observations 单项逐字段桥。 -/
def ObservationParity
    (rust : RustCandidateObservationExtraction) (lean : CandidateObservation) : Bool :=
  decide
    (rust.key.toLean = lean.key ∧
      rust.kind.toLean = lean.kind ∧
      rust.centerIds.map RustIntervalExtraction.toLean = lean.centerIds ∧
      rust.candidateGroupId = lean.candidateGroupId ∧
      rust.pairId = lean.pairId ∧
      rust.structuralPredicates.toLean = lean.structuralPredicates ∧
      rust.extremeProof.toLean = lean.extremeProof ∧
      rust.thirdClassProof = lean.thirdClassProof ∧
      rust.interval.toLean = lean.interval ∧
      rust.state.toLean = lean.state ∧
      rust.firstProvableAt = lean.firstProvableAt ∧
      rust.confirmedAt = lean.confirmedAt)

/-- 四类列表均为同长、同序的逐项协议；任一侧多项即失败。 -/
def PointsParity : List RustBspPointExtraction → List BspPoint → Bool
  | [], [] => true
  | rust :: rustRest, lean :: leanRest =>
      PointParity rust lean && PointsParity rustRest leanRest
  | _, _ => false

def PanDivsParity : List RustPanDivCertExtraction → List PanDivCert → Bool
  | [], [] => true
  | rust :: rustRest, lean :: leanRest =>
      PanDivParity rust lean && PanDivsParity rustRest leanRest
  | _, _ => false

def GradesParity : List RustFirstClassGradeExtraction → List FirstClassGradeRecord → Bool
  | [], [] => true
  | rust :: rustRest, lean :: leanRest =>
      GradeParity rust lean && GradesParity rustRest leanRest
  | _, _ => false

def ObservationsParity :
    List RustCandidateObservationExtraction → List CandidateObservation → Bool
  | [], [] => true
  | rust :: rustRest, lean :: leanRest =>
      ObservationParity rust lean && ObservationsParity rustRest leanRest
  | _, _ => false

/--
四件输出不经整体结构等式捷径，分别走各自同长同序、逐字段列表桥。验收调用方必须把
`recomputeRustScanSinkEmissions` 的 Lean 重算结果放在右侧，不能把 Rust 最终输出 decode 后回填。
-/
def MergedOutputParity
    (rust : RustMergedScanOutputExtraction) (lean : MergedScanOutput) : Prop :=
  PointsParity rust.points lean.points = true ∧
  PanDivsParity rust.panDivs lean.panDivs = true ∧
  GradesParity rust.grades lean.grades = true ∧
  ObservationsParity rust.observations lean.observations = true

instance mergedOutputParityDecidable
    (rust : RustMergedScanOutputExtraction) (lean : MergedScanOutput) :
    Decidable (MergedOutputParity rust lean) := by
  unfold MergedOutputParity
  infer_instance

def SnapshotParity (rust : RustScanSnapshotExtraction) (lean : ScanSnapshot) : Prop :=
  rust.checkpointSrc = lean.checkpointSrc ∧
  rust.level = lean.level ∧
  rust.isTerminalRoot = lean.isTerminalRoot ∧
  MergedOutputParity rust.output lean.output

def SnapshotListParity : List RustScanSnapshotExtraction → List ScanSnapshot → Prop
  | [], [] => True
  | rust :: rustRest, lean :: leanRest =>
      SnapshotParity rust lean ∧ SnapshotListParity rustRest leanRest
  | _, _ => False

end NewChanlun.Origin.ScanAssemblyBridge
