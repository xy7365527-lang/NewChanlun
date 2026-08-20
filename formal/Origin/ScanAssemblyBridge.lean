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

★D2/P1 边界（scan.rs:19-28）：CandDeltaEvent 快照字段与 pan_div_diag 不在四件输出内；它们由
P1/cp_ownership 独立计算。本桥不从 scan wire 提取或推导它们。c_p stable-revision 单调定理留在
ScanAssemblyMirror，但 c_p 不是 MergedScanOutput 的第五件输出。
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

/-! ## §3 points / BspPoint -/

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

/-! ## §4 panDivs / PanDivCert -/

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

/-! ## §5 grades / FirstClassGradeRecord -/

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

/-! ## §6 observations / CandidateObservation -/

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

/-! ## §7 checkpoint / 末根完整快照 -/

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

/-- 四列表的长度、顺序、嵌套字段全部参与相等。 -/
def SnapshotParity (rust : RustScanSnapshotExtraction) (lean : ScanSnapshot) : Prop :=
  rust.toLean = lean

def SnapshotListParity : List RustScanSnapshotExtraction → List ScanSnapshot → Prop
  | [], [] => True
  | rust :: rustRest, lean :: leanRest =>
      SnapshotParity rust lean ∧ SnapshotListParity rustRest leanRest
  | _, _ => False

end NewChanlun.Origin.ScanAssemblyBridge
