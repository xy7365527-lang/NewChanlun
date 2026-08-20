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
独立 wire 与逐字段检查，不从 scan wire 提取或推导。pan_div_diag 仍是诊断字段，不进入判定桥。
c_p stable-revision 单调定理留在 ScanAssemblyMirror，c_p 不是 MergedScanOutput 的第五件输出。

本桥另把原始 rows/centers/blocks 交给 `recomputeRustPrelude`，不从 Rust prelude 输出回填 Lean；
CandDelta 只给装配输入与 Rust 事件，c_p 只给 before+witness 与 Rust after。四输出只给生产扫描的
逐段、未排序、未归约 sink，Lean 通过 `recomputeMergedOutput` 独立执行排序、同 key 腿归约、
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

structure RustPredicateExtraction where
  accepted : Bool
  buy1 : Bool
  sell1 : Bool
deriving DecidableEq, Repr

def RustPredicateExtraction.toLean (facts : RustPredicateExtraction) : CandDeltaFacts :=
  { accepted := facts.accepted, buy1 := facts.buy1, sell1 := facts.sell1 }

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
deriving DecidableEq, Repr

def RustAssemblyInputExtraction.toLean (input : RustAssemblyInputExtraction) : EventAssemblyInput :=
  { level := input.level
    side := input.side.toLean
    divergenceConfirmSrc := input.divergenceConfirmSrc
    aInterval := { left := input.aIntervalLeft, right := input.aIntervalRight }
    center := input.center.toLean
    departureDir := input.departureDir.toLean
    untilStart := input.untilStart
    triggerEnd := input.triggerEnd
    rows := input.rows.map RustSegmentExtraction.toLean
    predicate := input.predicate.toLean }

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
  enterSrc : Nat
  candDelta : Bool
deriving DecidableEq, Repr

/-- 事件字段逐项比对；不用事件结构整体相等掩盖漏字段。 -/
def EventParity (rust : RustEventExtraction) (lean : CandDeltaEvent) : Prop :=
  rust.level = lean.level ∧
  rust.side.toLean = lean.side ∧
  rust.divergenceConfirmSrc = lean.divergenceConfirmSrc ∧
  rust.confirmSrc = lean.confirmSrc ∧
  rust.intervalLeft = lean.interval.left ∧
  rust.intervalRight = lean.interval.right ∧
  rust.aIntervalLeft = lean.aInterval.left ∧
  rust.aIntervalRight = lean.aInterval.right ∧
  rust.cEpisodeStart = lean.cEpisodeStart ∧
  rust.cEpisodeLeft = lean.cEpisodeInterval.left ∧
  rust.cEpisodeRight = lean.cEpisodeInterval.right ∧
  rust.enterSrc = lean.enterSrc ∧
  rust.candDelta = lean.candDelta

instance eventParityDecidable (rust : RustEventExtraction) (lean : CandDeltaEvent) :
    Decidable (EventParity rust lean) := by
  unfold EventParity
  infer_instance

structure RustCpExtraction where
  level : Nat
  bCenterOrdinal : Nat
  departureMoveOrdinal : Option Nat
  sourceStart : Option Nat
  lifecycle : RustCpLifecycleTag
deriving DecidableEq, Repr

def RustCpExtraction.toLean (value : RustCpExtraction) : CpOwnership :=
  { level := value.level, bCenterOrdinal := value.bCenterOrdinal
    departureMoveOrdinal := value.departureMoveOrdinal, sourceStart := value.sourceStart
    lifecycle := value.lifecycle.toLean }

/-- c_p 稳定身份四字段与生命周期逐项比对。 -/
def CpParity (rust : RustCpExtraction) (lean : CpOwnership) : Prop :=
  rust.level = lean.level ∧
  rust.bCenterOrdinal = lean.bCenterOrdinal ∧
  rust.departureMoveOrdinal = lean.departureMoveOrdinal ∧
  rust.sourceStart = lean.sourceStart ∧
  rust.lifecycle.toLean = lean.lifecycle

def CpListParity : List RustCpExtraction → List CpOwnership → Prop
  | [], [] => True
  | rust :: rustRest, lean :: leanRest =>
      CpParity rust lean ∧ CpListParity rustRest leanRest
  | _, _ => False

/-- Lean 事件必须从提取输入现算；两侧同缺省或逐字段相等。 -/
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

def EventListCheck :
    List RustAssemblyInputExtraction → List (Option RustEventExtraction) → Prop
  | [], [] => True
  | input :: inputRest, output :: outputRest =>
      EventCheck input output ∧ EventListCheck inputRest outputRest
  | _, _ => False

/-- dirty invalidation 不得走此关系；after 必须是同一 stable revision 的镜像推进结果。 -/
def StableCpCheck (rustBefore rustAfter : RustCpExtraction) (leanBefore : CpOwnership)
    (closureWitness : Bool) : Prop :=
  CpParity rustBefore leanBefore ∧ CpParity rustAfter (leanBefore.advance closureWitness)

/-- 验收入口只给 Rust before 原始对象与 closure witness；Lean 自己推进，Rust after 是被检侧。 -/
def RecomputeStableCpCheck (rustBefore rustAfter : RustCpExtraction)
    (closureWitness : Bool) : Prop :=
  CpParity rustAfter (rustBefore.toLean.advance closureWitness)

instance recomputeStableCpCheckDecidable (rustBefore rustAfter : RustCpExtraction)
    (closureWitness : Bool) : Decidable (RecomputeStableCpCheck rustBefore rustAfter closureWitness) := by
  unfold RecomputeStableCpCheck CpParity
  infer_instance

def StableCpListCheck :
    List RustCpExtraction → List RustCpExtraction → List CpOwnership → List Bool → Prop
  | [], [], [], [] => True
  | rustBefore :: rustBeforeRest, rustAfter :: rustAfterRest,
      leanBefore :: leanBeforeRest, witness :: witnessRest =>
      StableCpCheck rustBefore rustAfter leanBefore witness ∧
        StableCpListCheck rustBeforeRest rustAfterRest leanBeforeRest witnessRest
  | _, _, _, _ => False

def CheckpointSidecarCheck (rustInputs : List RustAssemblyInputExtraction)
    (rustEvents : List (Option RustEventExtraction))
    (rustCpBefore rustCpAfter : List RustCpExtraction)
    (leanCpBefore : List CpOwnership) (closureWitnesses : List Bool) : Prop :=
  EventListCheck rustInputs rustEvents ∧
    StableCpListCheck rustCpBefore rustCpAfter leanCpBefore closureWitnesses

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

/-- Rust 生产扫描逐段发出的未排序、未归约 sink；不是 legacy oracle 或最终输出。 -/
structure RustScanSinkEmissionExtraction where
  bspPoints : List RustBspPointExtraction
  panDivCerts : List RustPanDivCertExtraction
  firstClassGrades : List RustFirstClassGradeExtraction
  candidateLegs : List RustCandidateObservationExtraction
deriving DecidableEq, Repr

def RustScanSinkEmissionExtraction.toLean
    (emission : RustScanSinkEmissionExtraction) : ScanSinkEmission :=
  { bspPoints := emission.bspPoints.map RustBspPointExtraction.toLean
    panDivCerts := emission.panDivCerts.map RustPanDivCertExtraction.toLean
    firstClassGrades := emission.firstClassGrades.map RustFirstClassGradeExtraction.toLean
    candidateLegs := emission.candidateLegs.map RustCandidateObservationExtraction.toLean }

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
