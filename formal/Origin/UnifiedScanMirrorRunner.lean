/-
Origin/UnifiedScanMirrorRunner.lean

#1080 Phase 3 的机器执行层。Rust 提取器生成只含原始输入与 Rust wire 的 Lean
fixture，本文件现场执行完整镜面、逐字段记录 mismatch path，并以非零退出拒绝任何 mismatch。
-/

import Origin.UnifiedScanMirrorBridge
import Std.Data.HashSet

namespace NewChanlun.Origin.UnifiedScanMirrorRunner

open UnifiedScanMirrorBridge

structure RuntimeCheck where
  path : String
  port : String
  level : Nat
  side : String
  terminal : String
  comparedFields : Nat
  passed : Bool
  failedFields : List String := []
  failedDetails : List (String × String × String) := []
  coveredPaths : List String := []
deriving Repr

def failedPath (path : String) (holds : Bool) : List String :=
  if holds then [] else [path]

def failedDetail (path expected actual : String) (holds : Bool) :
    List (String × String × String) :=
  if holds then [] else [(path, expected, actual)]

def optionMismatchPaths (path : String) (rust : Option alpha) (lean : Option beta)
    (payload : String -> alpha -> beta -> List String) : List String :=
  match rust, lean with
  | none, none => []
  | some r, some l => payload path r l
  | _, _ => [s!"{path}.tag"]

def optionCoveredPaths (path : String) (rust : Option alpha) (lean : Option beta)
    (payload : String -> alpha -> beta -> List String) : List String :=
  s!"{path}.tag" :: match rust, lean with
    | some r, some l => payload path r l
    | _, _ => []

def listMismatchPathsAux (path : String) (index : Nat) (rust : List alpha) (lean : List beta)
    (payload : String -> alpha -> beta -> List String) : List String :=
  match rust, lean with
  | [], [] => []
  | r :: rs, l :: ls => payload s!"{path}[{index}]" r l ++
      listMismatchPathsAux path (index + 1) rs ls payload
  | _, _ => [s!"{path}.length"]

def listCoveredItemPaths (path : String) (rust : List alpha) (lean : List beta)
    (payload : String -> alpha -> beta -> List String) : List String :=
  match rust, lean with
  | r :: rs, l :: ls => payload s!"{path}.item" r l ++ listCoveredItemPaths path rs ls payload
  | _, _ => []

def listCoveredPaths (path : String) (rust : List alpha) (lean : List beta)
    (payload : String -> alpha -> beta -> List String) : List String :=
  [s!"{path}.length", s!"{path}.canonical_order"] ++
    listCoveredItemPaths path rust lean payload

def flatListCoveredPaths (path : String) (rust : List alpha) (lean : List beta)
    (payload : String -> alpha -> beta -> List String) : List String :=
  match rust, lean with
  | r :: rs, l :: ls => payload path r l ++ flatListCoveredPaths path rs ls payload
  | _, _ => []

def elementMismatchPaths (path : String) (rust : RustElementIdExtraction)
    (lean : ElementIdMirror) : List String :=
  failedPath s!"{path}.level" (rust.level == lean.level) ++
  failedPath s!"{path}.ordinal" (rust.ordinal == lean.ordinal)

def intervalMismatchPaths (path : String) (rust lean : Interval) : List String :=
  failedPath s!"{path}.left" (rust.left == lean.left) ++
  failedPath s!"{path}.right" (rust.right == lean.right)

def centerMismatchPaths (path : String) (rust : RustCenterExtraction)
    (lean : CenterFrame) : List String :=
  failedPath s!"{path}.start_index" (rust.startIndex == lean.startIndex) ++
  failedPath s!"{path}.end_index" (rust.endIndex == lean.endIndex) ++
  failedPath s!"{path}.zd" (rust.zd == lean.zd) ++
  failedPath s!"{path}.zg" (rust.zg == lean.zg) ++
  failedPath s!"{path}.dd" (rust.dd == lean.dd) ++
  failedPath s!"{path}.gg" (rust.gg == lean.gg)

def cpStructureMismatchPaths (path : String) (rust : RustCpStructureIdentityExtraction)
    (lean : CpStructureIdentityMirror) : List String :=
  failedPath s!"{path}.level" (rust.level == lean.level) ++
  elementMismatchPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  elementMismatchPaths s!"{path}.departure_move_id" rust.departureMoveId lean.departureMoveId ++
  optionMismatchPaths s!"{path}.terminal_move_id" rust.terminalMoveId lean.terminalMoveId
    elementMismatchPaths ++
  failedPath s!"{path}.source_start" (rust.sourceStart == lean.sourceStart) ++
  optionMismatchPaths s!"{path}.source_end" rust.sourceEnd lean.sourceEnd
    (fun p r l => failedPath p (r == l))

def thirdCpMismatchPaths (path : String) (rust : RustThirdClassInCpExtraction)
    (lean : ThirdClassInCpMirror) : List String :=
  elementMismatchPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  elementMismatchPaths s!"{path}.cp_departure_move_id" rust.cpDepartureMoveId lean.cpDepartureMoveId ++
  elementMismatchPaths s!"{path}.departure_move_id" rust.departureMoveId lean.departureMoveId ++
  elementMismatchPaths s!"{path}.retest_move_id" rust.retestMoveId lean.retestMoveId ++
  intervalMismatchPaths s!"{path}.departure_interval" rust.departureInterval lean.departureInterval ++
  intervalMismatchPaths s!"{path}.retest_interval" rust.retestInterval lean.retestInterval ++
  failedPath s!"{path}.point_source_index" (rust.pointSourceIndex == lean.pointSourceIndex) ++
  failedPath s!"{path}.side" (rust.side.toLean == lean.side)

def trendContextMismatchPaths (path : String) (rust : RustTrendContextExtraction)
    (lean : TrendContextMirror) : List String :=
  elementMismatchPaths s!"{path}.predecessor_center_id" rust.predecessorCenterId lean.predecessorCenterId ++
  elementMismatchPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  failedPath s!"{path}.direction" (rust.direction.toLean == lean.direction)

def newExtremeMismatchPaths (path : String) (rust : RustNewExtremeExtraction)
    (lean : NewExtremeInDirectionMirror) : List String :=
  elementMismatchPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  failedPath s!"{path}.direction" (rust.direction.toLean == lean.direction) ++
  failedPath s!"{path}.reference_price" (rust.referencePrice == lean.referencePrice) ++
  failedPath s!"{path}.extreme_price" (rust.extremePrice == lean.extremePrice) ++
  elementMismatchPaths s!"{path}.extreme_move_id" rust.extremeMoveId lean.extremeMoveId ++
  failedPath s!"{path}.confirm_src" (rust.confirmSrc == lean.confirmSrc)

def internalCentersMismatchPaths (path : String) (rust : RustInternalCentersExtraction)
    (lean : InternalSublevelCentersMirror) : List String :=
  failedPath s!"{path}.c_level" (rust.cLevel == lean.cLevel) ++
  listMismatchPathsAux s!"{path}.center_ids" 0 rust.centerIds lean.centerIds elementMismatchPaths

def completedTrendMismatchPaths (path : String) (rust : RustCompletedTrendExtraction)
    (lean : CompletedTrendDecompositionMirror) : List String :=
  failedPath s!"{path}.direction" (rust.direction.toLean == lean.direction) ++
  listMismatchPathsAux s!"{path}.center_ids" 0 rust.centerIds lean.centerIds elementMismatchPaths ++
  elementMismatchPaths s!"{path}.closing_successor_move_id"
    rust.closingSuccessorMoveId lean.closingSuccessorMoveId ++
  failedPath s!"{path}.confirm_src" (rust.confirmSrc == lean.confirmSrc)

def evidenceMismatchPaths (path : String) (rust : RustFullTrendEvidenceExtraction)
    (lean : FullTrendQualificationEvidenceMirror) : List String :=
  optionMismatchPaths s!"{path}.trend_context" rust.trendContext lean.trendContext trendContextMismatchPaths ++
  optionMismatchPaths s!"{path}.new_extreme" rust.newExtremeInDirection lean.newExtremeInDirection
    newExtremeMismatchPaths ++
  optionMismatchPaths s!"{path}.internal_centers" rust.internalSublevelCenters
    lean.internalSublevelCenters internalCentersMismatchPaths ++
  optionMismatchPaths s!"{path}.completed_decomposition" rust.completedTrendDecomposition
    lean.completedTrendDecomposition completedTrendMismatchPaths ++
  optionMismatchPaths s!"{path}.review_move_id" rust.decompositionReviewMoveId
    lean.decompositionReviewMoveId elementMismatchPaths ++
  optionMismatchPaths s!"{path}.review_src" rust.decompositionReviewSrc lean.decompositionReviewSrc
    (fun p r l => failedPath p (r == l))

def qualifiedMismatchPaths (path : String) (rust : RustFullTrendQualifiedExtraction)
    (lean : FullTrendCQualifiedMirror) : List String :=
  trendContextMismatchPaths s!"{path}.trend_context" rust.trendContext lean.trendContext ++
  thirdCpMismatchPaths s!"{path}.third_class" rust.thirdClassInsideC lean.thirdClassInsideC ++
  newExtremeMismatchPaths s!"{path}.new_extreme" rust.newExtremeInDirection lean.newExtremeInDirection ++
  internalCentersMismatchPaths s!"{path}.internal_centers" rust.internalSublevelCenters
    lean.internalSublevelCenters ++
  completedTrendMismatchPaths s!"{path}.completed_decomposition" rust.completedTrendDecomposition
    lean.completedTrendDecomposition ++
  failedPath s!"{path}.confirm_src" (rust.confirmSrc == lean.confirmSrc)

def cpFullMismatchPaths (path : String) (rust : RustCpFullExtraction)
    (lean : CpOwnershipFullMirror) : List String :=
  failedPath s!"{path}.b_center_index" (rust.bCenterIndex == lean.bCenterIndex) ++
  elementMismatchPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  centerMismatchPaths s!"{path}.b_center" rust.bCenter lean.bCenter ++
  optionMismatchPaths s!"{path}.departure_move_id" rust.departureMoveId lean.departureMoveId
    elementMismatchPaths ++
  optionMismatchPaths s!"{path}.departure_interval" rust.departureInterval lean.departureInterval
    intervalMismatchPaths ++
  failedPath s!"{path}.lifecycle" (rust.lifecycle.toLean == lean.lifecycle) ++
  optionMismatchPaths s!"{path}.confirm_src" rust.cpCertificateConfirmSrc lean.cpCertificateConfirmSrc
    (fun p r l => failedPath p (r == l)) ++
  optionMismatchPaths s!"{path}.c_structure" rust.cStructure lean.cStructure cpStructureMismatchPaths ++
  optionMismatchPaths s!"{path}.third_class" rust.thirdClassInC lean.thirdClassInC thirdCpMismatchPaths ++
  optionMismatchPaths s!"{path}.full_evidence" rust.fullTrendEvidence lean.fullTrendEvidence
    evidenceMismatchPaths ++
  optionMismatchPaths s!"{path}.full_qualified" rust.fullTrendCQualified lean.fullTrendCQualified
    qualifiedMismatchPaths

def thirdEntryMismatchPaths (path : String)
    (rust : Option RustThirdClassEntryExtraction)
    (lean : Option ThirdClassEntryIdentityMirror) : List String :=
  match rust, lean with
  | none, none => []
  | some r, some l =>
      failedPath s!"{path}.center_si" (r.centerSi == l.centerSi) ++
      failedPath s!"{path}.center_zd" (r.centerZd == l.centerZd) ++
      failedPath s!"{path}.center_zg" (r.centerZg == l.centerZg) ++
      failedPath s!"{path}.leave.left" (r.leaveLeft == l.leaveInterval.left) ++
      failedPath s!"{path}.leave.right" (r.leaveRight == l.leaveInterval.right) ++
      failedPath s!"{path}.retest.left" (r.retestLeft == l.retestInterval.left) ++
      failedPath s!"{path}.retest.right" (r.retestRight == l.retestInterval.right)
  | _, _ => [s!"{path}.presence"]

def forceFeatureMismatchPaths (path : String)
    (rust : RustForceFeatureBitsExtraction) (lean : ForceFeatureBitsMirror) : List String :=
  failedPath s!"{path}.macd_area_bits" (rust.macdAreaBits == lean.macdAreaBits) ++
  failedPath s!"{path}.dif_peak_bits" (rust.difPeakBits == lean.difPeakBits) ++
  failedPath s!"{path}.price_amplitude" (rust.priceAmplitude == lean.priceAmplitude) ++
  failedPath s!"{path}.price_speed_bits" (rust.priceSpeedBits == lean.priceSpeedBits)

def forceMismatchPaths (path : String)
    (rust : Option RustForceProxiesBitsExtraction)
    (lean : Option ForceProxiesBitsMirror) : List String :=
  match rust, lean with
  | none, none => []
  | some r, some l => forceFeatureMismatchPaths s!"{path}.seg_a" r.segA l.segA ++
      forceFeatureMismatchPaths s!"{path}.seg_c" r.segC l.segC
  | _, _ => [s!"{path}.presence"]

def ownerMismatchPaths (path : String)
    (rust : Option RustOwnerRefExtraction) (lean : Option OwnerRefMirror) : List String :=
  match rust, lean with
  | none, none => []
  | some (.center r), some (.center l) => centerMismatchPaths s!"{path}.center" r l
  | some (.type1Anchor r), some (.type1Anchor l) =>
      failedPath s!"{path}.type1_anchor" (r == l)
  | some _, some _ => [s!"{path}.variant"]
  | _, _ => [s!"{path}.presence"]

def bspMismatchPaths (path : String)
    (rust : RustBspPointExtraction) (lean : BspPointMirror) : List String :=
  failedPath s!"{path}.source_index" (rust.sourceIndex == lean.sourceIndex) ++
  failedPath s!"{path}.bits.buy1" (rust.bits.buy1 == lean.bits.buy1) ++
  failedPath s!"{path}.bits.buy2" (rust.bits.buy2 == lean.bits.buy2) ++
  failedPath s!"{path}.bits.buy3" (rust.bits.buy3 == lean.bits.buy3) ++
  failedPath s!"{path}.bits.sell1" (rust.bits.sell1 == lean.bits.sell1) ++
  failedPath s!"{path}.bits.sell2" (rust.bits.sell2 == lean.bits.sell2) ++
  failedPath s!"{path}.bits.sell3" (rust.bits.sell3 == lean.bits.sell3) ++
  thirdEntryMismatchPaths s!"{path}.bits.third_class_entry" rust.bits.thirdClassEntry
    lean.bits.thirdClassEntry ++
  failedPath s!"{path}.pivot_low" (rust.pivotLow == lean.pivotLow) ++
  failedPath s!"{path}.pivot_high" (rust.pivotHigh == lean.pivotHigh) ++
  ownerMismatchPaths s!"{path}.owner" rust.owner lean.owner ++
  optionMismatchPaths s!"{path}.struct_break_dir" rust.structBreakDir lean.structBreakDir
    (fun p r l => failedPath p (r.toLean == l)) ++
  forceMismatchPaths s!"{path}.force" rust.force lean.force ++
  optionMismatchPaths s!"{path}.retrace_breaks_type1"
    rust.retraceBreaksType1 lean.retraceBreaksType1 (fun p r l => failedPath p (r == l))

def thirdEntryFieldCount : Option RustThirdClassEntryExtraction -> Nat
  | none => 1
  | some _ => 8

def ownerFieldCount : Option RustOwnerRefExtraction -> Nat
  | none => 1
  | some (.center _) => 8
  | some (.type1Anchor _) => 3

def forceFieldCount : Option RustForceProxiesBitsExtraction -> Nat
  | none => 1
  | some _ => 9

def optionScalarFieldCount : Option alpha -> Nat
  | none => 1
  | some _ => 2

/-- 实际执行的 O-01 叶字段数：Option tag 与 union variant 都独立计数。 -/
def bspFieldCount (rust : RustBspPointExtraction) : Nat :=
  3 + 6 + thirdEntryFieldCount rust.bits.thirdClassEntry + ownerFieldCount rust.owner +
    optionScalarFieldCount rust.structBreakDir + forceFieldCount rust.force +
    optionScalarFieldCount rust.retraceBreaksType1

def bspOptionFieldCount : Option RustBspPointExtraction -> Nat
  | none => 1
  | some point => 1 + bspFieldCount point

def rustBspToMirror (rust : RustBspPointExtraction) : BspPointMirror :=
  rust.toLean

def rustGradeToMirror (rust : RustGradeExtraction) : FirstClassGradeRecordMirror :=
  { level := rust.level, sourceIndex := rust.sourceIndex, side := rust.side.toLean,
    centerStart := rust.centerStart, centerEnd := rust.centerEnd,
    centerZd := rust.centerZd, centerZg := rust.centerZg, grade := rust.grade.toLean }

def rustPanToMirror (rust : RustPanDivExtraction) : PanDivCertMirror :=
  { sourceIndex := rust.sourceIndex, side := rust.side.toLean
    centerStart := rust.centerStart, centerEnd := rust.centerEnd
    centerZd := rust.centerZd, centerZg := rust.centerZg
    centerDd := rust.centerDd, centerGg := rust.centerGg
    segA := { left := rust.segALeft, right := rust.segARight }
    segC := { left := rust.segCLeft, right := rust.segCRight } }

def rustCandidateToMirror (rust : RustCandidateExtraction) : CandidateObservationMirror :=
  let key : CandidateKeyMirror :=
    { ruleVersion := rust.ruleVersion
      level := rust.level
      kind := rust.kind.toLean
      side := rust.side.toLean
      previousCenterStart := rust.previousCenterStart
      parentCenterStart := rust.parentCenterStart
      parentZd := rust.parentZd
      parentZg := rust.parentZg
      segA := { left := rust.segALeft, right := rust.segARight }
      lambdaC := rust.lambdaC }
  let predicates : StructuralPredicates :=
    { direction := rust.direction, comparable := rust.comparable, extreme := rust.extreme }
  { key := key
    kind := rust.observationKind.toLean
    centerIds := rust.centerIds
    candidateGroupId := rust.candidateGroupId
    pairId := rust.pairId
    predicates := predicates
    extremeProof := { left := rust.extremeProofLeft, right := rust.extremeProofRight }
    thirdClassProof := rust.thirdClassProof
    interval := { left := rust.intervalLeft, right := rust.intervalRight }
    state := rust.state.toLean, firstProvableAt := rust.firstProvableAt,
    confirmedAt := rust.confirmedAt }

def candidateMismatchPaths (path : String)
    (rust : RustCandidateExtraction) (lean : CandidateObservationMirror) : List String :=
  failedPath s!"{path}.key.rule_version" (rust.ruleVersion == lean.key.ruleVersion) ++
  failedPath s!"{path}.key.level" (rust.level == lean.key.level) ++
  failedPath s!"{path}.key.kind" (rust.kind.toLean == lean.key.kind) ++
  failedPath s!"{path}.kind" (rust.observationKind.toLean == lean.kind) ++
  failedPath s!"{path}.key.side" (rust.side.toLean == lean.key.side) ++
  optionMismatchPaths s!"{path}.key.previous_center_start" rust.previousCenterStart
    lean.key.previousCenterStart (fun p r l => failedPath p (r == l)) ++
  failedPath s!"{path}.key.parent.start" (rust.parentCenterStart == lean.key.parentCenterStart) ++
  failedPath s!"{path}.key.parent.zd" (rust.parentZd == lean.key.parentZd) ++
  failedPath s!"{path}.key.parent.zg" (rust.parentZg == lean.key.parentZg) ++
  failedPath s!"{path}.key.seg_a.left" (rust.segALeft == lean.key.segA.left) ++
  failedPath s!"{path}.key.seg_a.right" (rust.segARight == lean.key.segA.right) ++
  failedPath s!"{path}.key.c_start" (rust.lambdaC == lean.key.lambdaC) ++
  optionMismatchPaths s!"{path}.center_ids" rust.centerIds lean.centerIds
    (fun p r l => failedPath s!"{p}.left" (r.1 == l.1) ++
      failedPath s!"{p}.right" (r.2 == l.2)) ++
  failedPath s!"{path}.candidate_group_id" (rust.candidateGroupId == lean.candidateGroupId) ++
  failedPath s!"{path}.pair_id" (rust.pairId == lean.pairId) ++
  failedPath s!"{path}.structural_predicates.direction"
    (rust.direction == lean.predicates.direction) ++
  failedPath s!"{path}.structural_predicates.comparable"
    (rust.comparable == lean.predicates.comparable) ++
  failedPath s!"{path}.structural_predicates.extreme"
    (rust.extreme == lean.predicates.extreme) ++
  failedPath s!"{path}.extreme_proof.left" (rust.extremeProofLeft == lean.extremeProof.left) ++
  failedPath s!"{path}.extreme_proof.right" (rust.extremeProofRight == lean.extremeProof.right) ++
  optionMismatchPaths s!"{path}.third_class_proof" rust.thirdClassProof lean.thirdClassProof
    (fun p r l => failedPath p (r == l)) ++
  failedPath s!"{path}.interval.left" (rust.intervalLeft == lean.interval.left) ++
  failedPath s!"{path}.interval.right" (rust.intervalRight == lean.interval.right) ++
  failedPath s!"{path}.state" (rust.state.toLean == lean.state) ++
  optionMismatchPaths s!"{path}.first_provable_at" rust.firstProvableAt lean.firstProvableAt
    (fun p r l => failedPath p (r == l)) ++
  optionMismatchPaths s!"{path}.confirmed_at" rust.confirmedAt lean.confirmedAt
    (fun p r l => failedPath p (r == l))

/-- O-04 的实际叶字段数；所有 Option tag 单列，`center_ids` 的双元素逐项计。 -/
def candidateFieldCount (rust : RustCandidateExtraction) : Nat :=
  21 + optionScalarFieldCount rust.previousCenterStart +
    (match rust.centerIds with | none => 1 | some _ => 3) +
    optionScalarFieldCount rust.thirdClassProof + optionScalarFieldCount rust.firstProvableAt +
    optionScalarFieldCount rust.confirmedAt

def panMismatchPaths (path : String) (rust : RustPanDivExtraction)
    (lean : PanDivCertMirror) : List String :=
  failedPath s!"{path}.source_index" (rust.sourceIndex == lean.sourceIndex) ++
  failedPath s!"{path}.side" (rust.side.toLean == lean.side) ++
  failedPath s!"{path}.center.start" (rust.centerStart == lean.centerStart) ++
  failedPath s!"{path}.center.end" (rust.centerEnd == lean.centerEnd) ++
  failedPath s!"{path}.center.zd" (rust.centerZd == lean.centerZd) ++
  failedPath s!"{path}.center.zg" (rust.centerZg == lean.centerZg) ++
  failedPath s!"{path}.center.dd" (rust.centerDd == lean.centerDd) ++
  failedPath s!"{path}.center.gg" (rust.centerGg == lean.centerGg) ++
  failedPath s!"{path}.seg_a.left" (rust.segALeft == lean.segA.left) ++
  failedPath s!"{path}.seg_a.right" (rust.segARight == lean.segA.right) ++
  failedPath s!"{path}.seg_c.left" (rust.segCLeft == lean.segC.left) ++
  failedPath s!"{path}.seg_c.right" (rust.segCRight == lean.segC.right)

def gradeValueMismatchPaths (path : String) (rust : RustGradeExtractionValue)
    (lean : Grade) : List String :=
  match rust, lean with
  | .present leaveLeft leaveRight retestLeft retestRight, .present leave retest =>
      failedPath s!"{path}.leave.left" (leaveLeft == leave.left) ++
      failedPath s!"{path}.leave.right" (leaveRight == leave.right) ++
      failedPath s!"{path}.retest.left" (retestLeft == retest.left) ++
      failedPath s!"{path}.retest.right" (retestRight == retest.right)
  | .missing rustReason, .missing leanReason =>
      failedPath s!"{path}.reason" (rustReason.toLean == leanReason)
  | _, _ => [s!"{path}.variant"]

def gradeMismatchPaths (path : String) (rust : RustGradeExtraction)
    (lean : FirstClassGradeRecordMirror) : List String :=
  failedPath s!"{path}.level" (rust.level == lean.level) ++
  failedPath s!"{path}.source_index" (rust.sourceIndex == lean.sourceIndex) ++
  failedPath s!"{path}.side" (rust.side.toLean == lean.side) ++
  failedPath s!"{path}.center.start" (rust.centerStart == lean.centerStart) ++
  failedPath s!"{path}.center.end" (rust.centerEnd == lean.centerEnd) ++
  failedPath s!"{path}.center.zd" (rust.centerZd == lean.centerZd) ++
  failedPath s!"{path}.center.zg" (rust.centerZg == lean.centerZg) ++
  gradeValueMismatchPaths s!"{path}.grade" rust.grade lean.grade

def centerCoveredPaths (path : String) (_ : RustCenterExtraction) (_ : CenterFrame) : List String :=
  [s!"{path}.start_index", s!"{path}.end_index", s!"{path}.zd", s!"{path}.zg",
   s!"{path}.dd", s!"{path}.gg"]

def elementCoveredPaths (path : String) (_ : RustElementIdExtraction)
    (_ : ElementIdMirror) : List String :=
  [s!"{path}.level", s!"{path}.ordinal"]

def intervalCoveredPaths (path : String) (_ _ : Interval) : List String :=
  [s!"{path}.left", s!"{path}.right"]

def thirdEntryCoveredPaths (path : String) (rust : Option RustThirdClassEntryExtraction)
    (lean : Option ThirdClassEntryIdentityMirror) : List String :=
  optionCoveredPaths path rust lean fun p _ _ =>
    [s!"{p}.center_si", s!"{p}.center_zd", s!"{p}.center_zg",
     s!"{p}.leave.left", s!"{p}.leave.right", s!"{p}.retest.left", s!"{p}.retest.right"]

def ownerCoveredPaths (path : String) (rust : Option RustOwnerRefExtraction)
    (lean : Option OwnerRefMirror) : List String :=
  s!"{path}.tag" :: match rust, lean with
    | some (.center r), some (.center l) =>
        s!"{path}.variant" :: centerCoveredPaths s!"{path}.center" r l
    | some (.type1Anchor _), some (.type1Anchor _) =>
        [s!"{path}.variant", s!"{path}.type1_anchor"]
    | some _, some _ => [s!"{path}.variant"]
    | _, _ => []

def forceFeatureCoveredPaths (path : String) : List String :=
  [s!"{path}.macd_area_bits", s!"{path}.dif_peak_bits", s!"{path}.price_amplitude",
   s!"{path}.price_speed_bits"]

def forceCoveredPaths (path : String) (rust : Option RustForceProxiesBitsExtraction)
    (lean : Option ForceProxiesBitsMirror) : List String :=
  optionCoveredPaths path rust lean fun p _ _ =>
    forceFeatureCoveredPaths s!"{p}.seg_a" ++ forceFeatureCoveredPaths s!"{p}.seg_c"

def scalarOptionCoveredPaths (path : String) (rust : Option alpha) (lean : Option beta) : List String :=
  optionCoveredPaths path rust lean fun p _ _ => [s!"{p}.value"]

def bspCoveredPaths (path : String) (rust : RustBspPointExtraction)
    (lean : BspPointMirror) : List String :=
  [s!"{path}.source_index", s!"{path}.bits.buy1", s!"{path}.bits.buy2",
   s!"{path}.bits.buy3", s!"{path}.bits.sell1", s!"{path}.bits.sell2",
   s!"{path}.bits.sell3"] ++
  thirdEntryCoveredPaths s!"{path}.bits.third_class_entry" rust.bits.thirdClassEntry
    lean.bits.thirdClassEntry ++
  [s!"{path}.pivot_low", s!"{path}.pivot_high"] ++
  ownerCoveredPaths s!"{path}.owner" rust.owner lean.owner ++
  scalarOptionCoveredPaths s!"{path}.struct_break_dir" rust.structBreakDir lean.structBreakDir ++
  forceCoveredPaths s!"{path}.force" rust.force lean.force ++
  scalarOptionCoveredPaths s!"{path}.retrace_breaks_type1"
    rust.retraceBreaksType1 lean.retraceBreaksType1

def bspOptionCoveredPaths (path : String) (rust : Option RustBspPointExtraction)
    (lean : Option BspPointMirror) : List String :=
  optionCoveredPaths path rust lean bspCoveredPaths

def panCoveredPaths (path : String) (_ : RustPanDivExtraction)
    (_ : PanDivCertMirror) : List String :=
  [s!"{path}.source_index", s!"{path}.side", s!"{path}.center.start_index",
   s!"{path}.center.end_index", s!"{path}.center.zd", s!"{path}.center.zg",
   s!"{path}.center.dd", s!"{path}.center.gg", s!"{path}.seg_a.left",
   s!"{path}.seg_a.right", s!"{path}.seg_c.left", s!"{path}.seg_c.right"]

def gradeCoveredPaths (path : String) (rust : RustGradeExtraction)
    (lean : FirstClassGradeRecordMirror) : List String :=
  let base :=
    [s!"{path}.level", s!"{path}.source_index", s!"{path}.side",
     s!"{path}.center_start_index", s!"{path}.center_end_index", s!"{path}.center_zd",
     s!"{path}.center_zg", s!"{path}.grade.tag"]
  base ++ match rust.grade, lean.grade with
    | .present _ _ _ _, .present _ _ =>
        [s!"{path}.grade.present.leave.left", s!"{path}.grade.present.leave.right",
         s!"{path}.grade.present.retest.left", s!"{path}.grade.present.retest.right"]
    | .missing _, .missing _ => [s!"{path}.grade.missing.reason"]
    | _, _ => []

def candidateCoveredPaths (path : String) (rust : RustCandidateExtraction)
    (lean : CandidateObservationMirror) : List String :=
  let base :=
    [s!"{path}.key.rule_version", s!"{path}.key.level", s!"{path}.key.kind",
     s!"{path}.kind", s!"{path}.key.side"]
  base ++
  scalarOptionCoveredPaths s!"{path}.key.previous_center_start"
    rust.previousCenterStart lean.key.previousCenterStart ++
  [s!"{path}.key.parent.start", s!"{path}.key.parent.zd", s!"{path}.key.parent.zg",
   s!"{path}.key.seg_a.left", s!"{path}.key.seg_a.right", s!"{path}.key.c_start"] ++
  optionCoveredPaths s!"{path}.center_ids" rust.centerIds lean.centerIds (fun p _ _ =>
    [s!"{p}.left", s!"{p}.right"]) ++
  [s!"{path}.candidate_group_id", s!"{path}.pair_id",
   s!"{path}.structural_predicates.direction", s!"{path}.structural_predicates.comparable",
   s!"{path}.structural_predicates.extreme", s!"{path}.extreme_proof.left",
   s!"{path}.extreme_proof.right"] ++
  scalarOptionCoveredPaths s!"{path}.third_class_proof" rust.thirdClassProof lean.thirdClassProof ++
  [s!"{path}.interval.left", s!"{path}.interval.right", s!"{path}.state"] ++
  scalarOptionCoveredPaths s!"{path}.first_provable_at" rust.firstProvableAt
    lean.firstProvableAt ++
  scalarOptionCoveredPaths s!"{path}.confirmed_at" rust.confirmedAt lean.confirmedAt

def rustMergedToMirror (rust : RustMergedScanExtraction) : MergedScanOutputMirror :=
  { points := rust.points.map rustBspToMirror
    panDivs := rust.panDivs.map rustPanToMirror
    grades := rust.grades.map rustGradeToMirror
    observations := rust.observations.map rustCandidateToMirror }

def rustCpToMirror (rust : RustCpExtraction) (certificate : Option ClosureCertificate) :
    CpOwnershipMirror :=
  let identity : CpIdentity :=
    { level := rust.level
      bCenterOrdinal := rust.bCenterOrdinal
      departureMoveOrdinal := rust.departureMoveOrdinal
      sourceStart := rust.sourceStart }
  { identity := identity
    lifecycle := rust.lifecycle.toLean, certificate := certificate }

/-- `segment_force_l` 非负有限值的 IEEE-754 bit 序与数值序一致；缺值不判背驰。 -/
def forceLDivergedBits (forceA forceC : Option Nat) : Bool :=
  match forceA, forceC with
  | some a, some c =>
      let sign := 9223372036854775808
      let aNegative := decide (sign <= a)
      let cNegative := decide (sign <= c)
      if aNegative != cNegative then cNegative
      else if cNegative then decide (a < c) else decide (c < a)
  | _, _ => false

/-- 从 Rust 提取的固定首对 raw segment 现场重跑 T3-in-c 分级。 -/
def gradeFixedFirstPair (center : CenterFrame) (trend : Direction)
    (leave retest : Option SegmentRow) : Grade :=
  match leave, retest with
  | none, _ => Grade.missing GradeReason.missingLeave
  | some _, none => Grade.missing GradeReason.missingRetest
  | some l, some r =>
      if l.direction = r.direction then Grade.missing GradeReason.sameDirection
      else
        let leaveOutside := match trend with
          | Direction.up => decide (center.zg < l.endPrice)
          | Direction.down => decide (l.endPrice < center.zd)
        if !leaveOutside then Grade.missing GradeReason.leaveNotOutside
        else
          let retestOutside := match trend with
            | Direction.up => decide (center.zg < r.endPrice)
            | Direction.down => decide (r.endPrice < center.zd)
          if !retestOutside then Grade.missing GradeReason.retestReentered
          else Grade.present { left := l.startIndex, right := l.endIndex }
            { left := r.startIndex, right := r.endIndex }

def checkPrelude (path : String) (level : Nat)
    (rows : List SegmentRow) (centers : List CenterFrame) : RuntimeCheck :=
  { path := path, port := "prelude", level := level, side := "NotYetKnown",
    terminal := "NoCpApplicable", comparedFields := rows.length * 6 + centers.length * 4 + 3,
    passed := orderedResumeGuard rows centers
    coveredPaths := ["P01.segment_start_order", "P01.center_end_strict_order",
      "P02.anchor_structural_direction", "P02.parallel_array_length"] }

/-- 空产口也必须进入运行时清单；`length` 是 Rust serializer 的直接读数，不冒充内容 parity。 -/
def checkPortEnumeration (path port : String) (level count : Nat) : RuntimeCheck :=
  { path := s!"{path}.length={count}", port := port, level := level,
    side := "NotYetKnown", terminal := "NoCpApplicable", comparedFields := 1, passed := true }

def checkFirst (path : String) (level : Nat) (side : String)
    (input : FirstProjectionInput) (rust : Option RustBspPointExtraction) : RuntimeCheck :=
  let lean := judgeFirstFromGates input
  let failures := match rust, lean with
    | none, none => []
    | some r, some l => bspMismatchPaths path r l
    | _, _ => [s!"{path}.presence"]
  let details := failedDetail path (reprStr lean) (reprStr (rust.map rustBspToMirror)) failures.isEmpty
  let covered := match rust, lean with
    | some r, some l => bspCoveredPaths "O01" r l
    | _, _ => []
  { path := path, port := "points", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := bspOptionFieldCount rust,
    passed := failures.isEmpty, failedFields := failures, failedDetails := details
    coveredPaths := covered }

def checkThird (path : String) (level : Nat) (side : String)
    (input : ThirdProjectionInput) (rust : Option RustBspPointExtraction) : RuntimeCheck :=
  let lean := judgeThirdPoint input
  let failures := match rust, lean with
    | none, none => []
    | some r, some l => bspMismatchPaths path r l
    | _, _ => [s!"{path}.presence"]
  let covered := match rust, lean with
    | some r, some l => bspCoveredPaths "O01" r l
    | _, _ => []
  { path := path, port := "points", level := level, side := side
    terminal := "NoCpApplicable", comparedFields := bspOptionFieldCount rust
    passed := failures.isEmpty, failedFields := failures, coveredPaths := covered }

def checkGrade (path : String) (level : Nat) (side : String)
    (input : FirstProjectionInput) (rust : Option RustGradeExtraction) : RuntimeCheck :=
  let lean := assembleFirstClassGrade input
  let failures := match rust, lean with
    | none, none => []
    | some value, some mirror => gradeMismatchPaths path value mirror
    | _, _ => [s!"{path}.presence"]
  let details := failedDetail path (reprStr lean) (reprStr (rust.map rustGradeToMirror))
    failures.isEmpty
  { path := path, port := "grades", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 12,
    passed := failures.isEmpty, failedFields := failures, failedDetails := details
    coveredPaths := match rust, lean with
      | some value, some mirror => gradeCoveredPaths "O03" value mirror
      | _, _ => [] }

def checkTrendObservation (path : String) (level : Nat) (side : String)
    (previous parent : CenterFrame) (segment : SegmentRow)
    (gates : FirstStructuralGates) (rust : RustCandidateExtraction) : RuntimeCheck :=
  let lean := makeTrendObservation level previous parent segment gates
  let failures := candidateMismatchPaths path rust lean
  { path := path, port := "observations", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := candidateFieldCount rust,
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := candidateCoveredPaths "O04" rust lean }

def candidateMemberParity
    (rust : RustCandidateExtraction) (lean : List CandidateObservationMirror) : Bool :=
  lean.any fun candidate => rustCandidateToMirror rust == candidate

def checkReduction (path : String) (level : Nat) (side : String)
    (legs : List CandidateObservationMirror) (rust : List RustCandidateExtraction) : RuntimeCheck :=
  let lean := reduceStructuralLegsCanonical legs
  let failures := listMismatchPathsAux path 0 rust lean candidateMismatchPaths
  { path := path, port := "reduction", level := level, side := side,
    terminal := "NoCpApplicable",
    comparedFields := rust.foldl (fun total item => total + candidateFieldCount item) 1,
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := flatListCoveredPaths "O04" rust lean candidateCoveredPaths }

def checkPan (path : String) (level : Nat) (side : String)
    (input : PanProjectionInput) (rust : RustPanDivExtraction) : RuntimeCheck :=
  let lean := assemblePanDivCert input
  let failures := match lean with
    | some mirror => panMismatchPaths path rust mirror
    | none => [s!"{path}.presence"]
  { path := path, port := "pan", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 12
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := match lean with
      | some mirror => panCoveredPaths "O02" rust mirror
      | none => [] }

def checkPanObservation (path : String) (level : Nat) (side : String)
    (cert : PanDivCertMirror) (rust : RustCandidateExtraction) : RuntimeCheck :=
  let lean := makePanObservation level cert
  let failures := candidateMismatchPaths path rust lean
  { path := path, port := "observations", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := candidateFieldCount rust,
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := candidateCoveredPaths "O04" rust lean }

def cpLifecycleConsistencyPaths (path : String) (rust : RustCpFullExtraction) : List String :=
  let cp := rust.toLean
  let identity := match cp.cStructure with
    | none => []
    | some c =>
        failedPath s!"{path}.c_structure.b_center_id" (c.bCenterId == cp.bCenterId) ++
        failedPath s!"{path}.c_structure.departure_move_id"
          (some c.departureMoveId == cp.departureMoveId)
  let thirdIdentity := match cp.thirdClassInC with
    | none => []
    | some third =>
        failedPath s!"{path}.third_class.b_center_id" (third.bCenterId == cp.bCenterId) ++
        failedPath s!"{path}.third_class.cp_departure_move_id"
          (some third.cpDepartureMoveId == cp.departureMoveId)
  let qualified := match cp.fullTrendCQualified with
    | none => []
    | some full =>
        failedPath s!"{path}.full_qualified.third_class"
          (some full.thirdClassInsideC == cp.thirdClassInC) ++
        match cp.fullTrendEvidence with
        | none => [s!"{path}.full_qualified.evidence_presence"]
        | some evidence =>
            failedPath s!"{path}.full_qualified.trend_context"
              (evidence.trendContext == some full.trendContext) ++
            failedPath s!"{path}.full_qualified.new_extreme"
              (evidence.newExtremeInDirection == some full.newExtremeInDirection) ++
            failedPath s!"{path}.full_qualified.internal_centers"
              (evidence.internalSublevelCenters == some full.internalSublevelCenters) ++
            failedPath s!"{path}.full_qualified.completed_decomposition"
              (evidence.completedTrendDecomposition == some full.completedTrendDecomposition)
  let lifecycle := match cp.lifecycle with
    | CpLifecycle.pending =>
        failedPath s!"{path}.pending.confirm_src" cp.cpCertificateConfirmSrc.isNone ++
        failedPath s!"{path}.pending.third_class" cp.thirdClassInC.isNone ++
        failedPath s!"{path}.pending.full_qualified" cp.fullTrendCQualified.isNone ++
        match cp.cStructure with
        | none => []
        | some c => failedPath s!"{path}.pending.terminal_open" c.terminalMoveId.isNone ++
            failedPath s!"{path}.pending.source_end_open" c.sourceEnd.isNone
    | CpLifecycle.closed =>
        failedPath s!"{path}.closed.confirm_src" cp.cpCertificateConfirmSrc.isSome ++
        failedPath s!"{path}.closed.c_structure" cp.cStructure.isSome ++
        failedPath s!"{path}.closed.third_class" cp.thirdClassInC.isSome ++
        match cp.cStructure with
        | none => []
        | some c => failedPath s!"{path}.closed.terminal" c.terminalMoveId.isSome ++
            failedPath s!"{path}.closed.source_end" c.sourceEnd.isSome
  identity ++ thirdIdentity ++ qualified ++ lifecycle

def cpStructureCoveredPaths (path : String) (rust : RustCpStructureIdentityExtraction)
    (lean : CpStructureIdentityMirror) : List String :=
  [s!"{path}.level"] ++
  elementCoveredPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  elementCoveredPaths s!"{path}.departure_move_id" rust.departureMoveId lean.departureMoveId ++
  optionCoveredPaths s!"{path}.terminal_move_id" rust.terminalMoveId lean.terminalMoveId
    elementCoveredPaths ++ [s!"{path}.source_start"] ++
  scalarOptionCoveredPaths s!"{path}.source_end" rust.sourceEnd lean.sourceEnd

def thirdCpCoveredPaths (path : String) (rust : RustThirdClassInCpExtraction)
    (lean : ThirdClassInCpMirror) : List String :=
  elementCoveredPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  elementCoveredPaths s!"{path}.cp_departure_move_id" rust.cpDepartureMoveId
    lean.cpDepartureMoveId ++
  elementCoveredPaths s!"{path}.departure_move_id" rust.departureMoveId lean.departureMoveId ++
  elementCoveredPaths s!"{path}.retest_move_id" rust.retestMoveId lean.retestMoveId ++
  intervalCoveredPaths s!"{path}.departure_interval" rust.departureInterval lean.departureInterval ++
  intervalCoveredPaths s!"{path}.retest_interval" rust.retestInterval lean.retestInterval ++
  [s!"{path}.point_source_index", s!"{path}.side"]

def trendContextCoveredPaths (path : String) (rust : RustTrendContextExtraction)
    (lean : TrendContextMirror) : List String :=
  elementCoveredPaths s!"{path}.predecessor_center_id" rust.predecessorCenterId
    lean.predecessorCenterId ++
  elementCoveredPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  [s!"{path}.direction"]

def newExtremeCoveredPaths (path : String) (rust : RustNewExtremeExtraction)
    (lean : NewExtremeInDirectionMirror) : List String :=
  elementCoveredPaths s!"{path}.b_center_id" rust.bCenterId lean.bCenterId ++
  [s!"{path}.direction", s!"{path}.reference_price", s!"{path}.extreme_price"] ++
  elementCoveredPaths s!"{path}.extreme_move_id" rust.extremeMoveId lean.extremeMoveId ++
  [s!"{path}.confirm_src"]

def internalCentersCoveredPaths (path : String) (rust : RustInternalCentersExtraction)
    (lean : InternalSublevelCentersMirror) : List String :=
  [s!"{path}.c_level"] ++
  listCoveredPaths s!"{path}.center_ids" rust.centerIds lean.centerIds elementCoveredPaths

def completedTrendCoveredPaths (path : String) (rust : RustCompletedTrendExtraction)
    (lean : CompletedTrendDecompositionMirror) : List String :=
  [s!"{path}.direction"] ++
  listCoveredPaths s!"{path}.center_ids" rust.centerIds lean.centerIds elementCoveredPaths ++
  elementCoveredPaths s!"{path}.closing_successor_move_id" rust.closingSuccessorMoveId
    lean.closingSuccessorMoveId ++ [s!"{path}.confirm_src"]

def evidenceCoveredPaths (path : String) (rust : RustFullTrendEvidenceExtraction)
    (lean : FullTrendQualificationEvidenceMirror) : List String :=
  optionCoveredPaths s!"{path}.trend_context" rust.trendContext lean.trendContext
    trendContextCoveredPaths ++
  optionCoveredPaths s!"{path}.new_extreme" rust.newExtremeInDirection lean.newExtremeInDirection
    newExtremeCoveredPaths ++
  optionCoveredPaths s!"{path}.internal_centers" rust.internalSublevelCenters
    lean.internalSublevelCenters internalCentersCoveredPaths ++
  optionCoveredPaths s!"{path}.completed_decomposition" rust.completedTrendDecomposition
    lean.completedTrendDecomposition completedTrendCoveredPaths ++
  optionCoveredPaths s!"{path}.decomposition_review_move_id" rust.decompositionReviewMoveId
    lean.decompositionReviewMoveId elementCoveredPaths ++
  scalarOptionCoveredPaths s!"{path}.decomposition_review_src" rust.decompositionReviewSrc
    lean.decompositionReviewSrc

def qualifiedCoveredPaths (path : String) (rust : RustFullTrendQualifiedExtraction)
    (lean : FullTrendCQualifiedMirror) : List String :=
  trendContextCoveredPaths s!"{path}.trend_context" rust.trendContext lean.trendContext ++
  thirdCpCoveredPaths s!"{path}.third_class" rust.thirdClassInsideC lean.thirdClassInsideC ++
  newExtremeCoveredPaths s!"{path}.new_extreme" rust.newExtremeInDirection
    lean.newExtremeInDirection ++
  internalCentersCoveredPaths s!"{path}.internal_centers" rust.internalSublevelCenters
    lean.internalSublevelCenters ++
  completedTrendCoveredPaths s!"{path}.completed_decomposition" rust.completedTrendDecomposition
    lean.completedTrendDecomposition ++ [s!"{path}.confirm_src"]

def cpFullCoveredPaths (rust : RustCpFullExtraction) (lean : CpOwnershipFullMirror) : List String :=
  ["cp.b_center_index"] ++
  elementCoveredPaths "cp.b_center_id" rust.bCenterId lean.bCenterId ++
  centerCoveredPaths "cp.b_center" rust.bCenter lean.bCenter ++
  optionCoveredPaths "cp.departure_move_id" rust.departureMoveId lean.departureMoveId
    elementCoveredPaths ++
  optionCoveredPaths "cp.departure_interval" rust.departureInterval lean.departureInterval
    intervalCoveredPaths ++ ["cp.lifecycle"] ++
  scalarOptionCoveredPaths "cp.cp_certificate_confirm_src" rust.cpCertificateConfirmSrc
    lean.cpCertificateConfirmSrc ++
  optionCoveredPaths "cp.c_structure" rust.cStructure lean.cStructure cpStructureCoveredPaths ++
  optionCoveredPaths "cp.third_class_in_c" rust.thirdClassInC lean.thirdClassInC
    thirdCpCoveredPaths ++
  optionCoveredPaths "cp.full_trend_evidence" rust.fullTrendEvidence lean.fullTrendEvidence
    evidenceCoveredPaths ++
  optionCoveredPaths "cp.full_trend_c_qualified" rust.fullTrendCQualified
    lean.fullTrendCQualified qualifiedCoveredPaths

def optionCount (payload : alpha -> Nat) : Option alpha -> Nat
  | none => 1
  | some value => 1 + payload value

def elementCount (_ : RustElementIdExtraction) : Nat := 2
def intervalCount (_ : Interval) : Nat := 2
def trendContextCount (_ : RustTrendContextExtraction) : Nat := 5
def newExtremeCount (_ : RustNewExtremeExtraction) : Nat := 8
def internalCentersCount (centers : RustInternalCentersExtraction) : Nat :=
  2 + centers.centerIds.length * 2
def completedTrendCount (completed : RustCompletedTrendExtraction) : Nat :=
  5 + completed.centerIds.length * 2
def thirdInCpCount (_ : RustThirdClassInCpExtraction) : Nat := 14
def fullEvidenceCount (evidence : RustFullTrendEvidenceExtraction) : Nat :=
  optionCount trendContextCount evidence.trendContext +
  optionCount newExtremeCount evidence.newExtremeInDirection +
  optionCount internalCentersCount evidence.internalSublevelCenters +
  optionCount completedTrendCount evidence.completedTrendDecomposition +
  optionCount elementCount evidence.decompositionReviewMoveId +
  optionCount (fun _ => 1) evidence.decompositionReviewSrc
def fullQualifiedCount (qualified : RustFullTrendQualifiedExtraction) : Nat :=
  trendContextCount qualified.trendContext + thirdInCpCount qualified.thirdClassInsideC +
  newExtremeCount qualified.newExtremeInDirection +
  internalCentersCount qualified.internalSublevelCenters +
  completedTrendCount qualified.completedTrendDecomposition + 1

def cpFullDynamicFieldCount (rust : RustCpFullExtraction) : Nat :=
  1 + elementCount rust.bCenterId + 6 +
  optionCount elementCount rust.departureMoveId + optionCount intervalCount rust.departureInterval +
  1 + optionCount (fun _ => 1) rust.cpCertificateConfirmSrc +
  optionCount (fun c => 1 + elementCount c.bCenterId + elementCount c.departureMoveId +
    optionCount elementCount c.terminalMoveId + 1 + optionCount (fun _ => 1) c.sourceEnd) rust.cStructure +
  optionCount thirdInCpCount rust.thirdClassInC +
  optionCount fullEvidenceCount rust.fullTrendEvidence +
  optionCount fullQualifiedCount rust.fullTrendCQualified

/-- Full attachment: independent Rust wire vs expected mirror plus lifecycle consistency. -/
def checkCpAttachment (path : String) (level : Nat) (terminal : String)
    (rust : RustCpFullExtraction) (expected : CpOwnershipFullMirror) : RuntimeCheck :=
  let failures := cpFullMismatchPaths path rust expected ++ cpLifecycleConsistencyPaths path rust
  let parity := failures.isEmpty
  let details := failedDetail path (reprStr expected) (reprStr rust.toLean) parity
  let side := match rust.thirdClassInC.map (fun third => third.side.toLean) with
    | none => "NotYetKnown"
    | some Side.long => "Long"
    | some Side.short => "Short"
  { path := path, port := "cp", level := level, side := side,
    terminal := terminal, comparedFields := cpFullDynamicFieldCount rust,
    passed := failures.isEmpty, failedFields := failures, failedDetails := details
    coveredPaths := cpFullCoveredPaths rust expected }

def cpTransitionCheck (port path terminal : String) (level : Nat)
    (expected : CpOwnershipFullMirror) (after : RustCpFullExtraction) : RuntimeCheck :=
  let failures := cpFullMismatchPaths s!"{path}.after" after expected
  let passed := failures.isEmpty
  { path := path, port := port, level := level
    side := match after.thirdClassInC.map (fun third => third.side.toLean) with
      | some Side.long => "Long" | some Side.short => "Short" | none => "NotYetKnown"
    terminal := terminal, comparedFields := cpFullDynamicFieldCount after
    passed := passed, failedFields := failures
    failedDetails := failedDetail s!"{path}.after" (reprStr expected) (reprStr after.toLean) passed
    coveredPaths := cpFullCoveredPaths after expected }

def checkCpInit (path : String) (bCenterId : RustElementIdExtraction)
    (center : RustCenterExtraction) (departureMoveId : Option RustElementIdExtraction)
    (departureInterval : Option Interval) (after : RustCpFullExtraction) : RuntimeCheck :=
  cpTransitionCheck "cp_init" path "Pending" bCenterId.level
    (initCpFull bCenterId.toLean center.toLean
      (departureMoveId.map RustElementIdExtraction.toLean) departureInterval) after

def checkCpDirty (path : String) (dirtyFrom : Nat)
    (before after : RustCpFullExtraction) : RuntimeCheck :=
  let expected := invalidateCpFull before.toLean dirtyFrom
  cpTransitionCheck "cp_dirty" path
    (if expected.lifecycle = .closed then "Closed" else "Pending") before.bCenterId.level expected after

def checkCpReinherit (path : String) (dirtyFrom : Nat)
    (rebuilt : RustCpFullExtraction) (prior : Option RustCpFullExtraction)
    (after : RustCpFullExtraction) : RuntimeCheck :=
  let expected := match prior with
    | none => rebuilt.toLean
    | some old => reinheritCpFull rebuilt.toLean old.toLean dirtyFrom
  cpTransitionCheck "cp_reinherit" path
    (if expected.lifecycle = .closed then "Closed" else "Pending") rebuilt.bCenterId.level expected after

def checkCpAdvance (path : String) (before : RustCpFullExtraction)
    (raw : Option CpAdvanceRawMirror) (after : RustCpFullExtraction) : RuntimeCheck :=
  let expected := advanceCpFull before.toLean raw
  cpTransitionCheck "cp_advance" path
    (if expected.lifecycle = .closed then "Closed" else "Pending") before.bCenterId.level expected after

def checkCpReview (path : String) (before : RustCpFullExtraction)
    (centers : List CenterFrame) (visibleMoves : List CpMoveRawMirror)
    (after : RustCpFullExtraction) : RuntimeCheck :=
  let expected := reviewCpFull before.toLean centers visibleMoves
  cpTransitionCheck "cp_review" path "Closed" before.bCenterId.level expected after

structure RustFourCacheExtraction where
  points : List RustBspPointExtraction
  panDivs : List RustPanDivExtraction
  grades : List RustGradeExtraction
  candidateLegs : List RustCandidateExtraction
  cachedCount : Nat
deriving Repr

/-- Rust `FrontierCapture::{confirmed_append,tail}` 的四产口只读 wire。 -/
structure RustScanEmissionExtraction where
  points : List RustBspPointExtraction
  panDivs : List RustPanDivExtraction
  grades : List RustGradeExtraction
  candidateLegs : List RustCandidateExtraction
deriving Repr

def RustScanEmissionExtraction.toLean
    (emission : RustScanEmissionExtraction) : ScanRowEmission :=
  { points := emission.points.map rustBspToMirror
    panDivs := emission.panDivs.map rustPanToMirror
    grades := emission.grades.map rustGradeToMirror
    candidateLegs := emission.candidateLegs.map rustCandidateToMirror }

def RustFourCacheExtraction.toLean (cache : RustFourCacheExtraction) : FourCacheMirror :=
  { points := cache.points.map rustBspToMirror
    panDivs := cache.panDivs.map rustPanToMirror
    grades := cache.grades.map rustGradeToMirror
    candidateLegs := cache.candidateLegs.map rustCandidateToMirror
    cachedCount := cache.cachedCount }

def fourCacheMismatchPaths (path : String) (rust : RustFourCacheExtraction)
    (lean : FourCacheMirror) : List String :=
  listMismatchPathsAux s!"{path}.points" 0 rust.points lean.points bspMismatchPaths ++
  listMismatchPathsAux s!"{path}.pan_divs" 0 rust.panDivs lean.panDivs panMismatchPaths ++
  listMismatchPathsAux s!"{path}.grades" 0 rust.grades lean.grades gradeMismatchPaths ++
  listMismatchPathsAux s!"{path}.candidate_legs" 0 rust.candidateLegs lean.candidateLegs
    candidateMismatchPaths ++
  failedPath s!"{path}.cached_count" (rust.cachedCount == lean.cachedCount)

def scanEmissionMismatchPaths (path : String) (rust : RustScanEmissionExtraction)
    (lean : ScanRowEmission) : List String :=
  listMismatchPathsAux s!"{path}.points" 0 rust.points lean.points bspMismatchPaths ++
  listMismatchPathsAux s!"{path}.pan_divs" 0 rust.panDivs lean.panDivs panMismatchPaths ++
  listMismatchPathsAux s!"{path}.grades" 0 rust.grades lean.grades gradeMismatchPaths ++
  listMismatchPathsAux s!"{path}.candidate_legs" 0 rust.candidateLegs lean.candidateLegs
    candidateMismatchPaths

def scanEmissionCoveredPaths (path : String) (rust : RustScanEmissionExtraction)
    (lean : ScanRowEmission) : List String :=
  listCoveredPaths s!"{path}.points" rust.points lean.points bspCoveredPaths ++
  listCoveredPaths s!"{path}.pan_divs" rust.panDivs lean.panDivs panCoveredPaths ++
  listCoveredPaths s!"{path}.grades" rust.grades lean.grades gradeCoveredPaths ++
  listCoveredPaths s!"{path}.candidate_legs" rust.candidateLegs lean.candidateLegs
    candidateCoveredPaths

def mergedOutputMismatchPaths (path : String) (rust : RustMergedScanExtraction)
    (lean : MergedScanOutputMirror) : List String :=
  listMismatchPathsAux s!"{path}.points" 0 rust.points lean.points bspMismatchPaths ++
  listMismatchPathsAux s!"{path}.pan_divs" 0 rust.panDivs lean.panDivs panMismatchPaths ++
  listMismatchPathsAux s!"{path}.grades" 0 rust.grades lean.grades gradeMismatchPaths ++
  listMismatchPathsAux s!"{path}.observations" 0 rust.observations lean.observations
    candidateMismatchPaths

def fourCacheCoveredPaths (path : String) (rust : RustFourCacheExtraction)
    (lean : FourCacheMirror) : List String :=
  listCoveredPaths s!"{path}.points" rust.points lean.points bspCoveredPaths ++
  listCoveredPaths s!"{path}.pan_divs" rust.panDivs lean.panDivs panCoveredPaths ++
  listCoveredPaths s!"{path}.grades" rust.grades lean.grades gradeCoveredPaths ++
  listCoveredPaths s!"{path}.candidate_legs" rust.candidateLegs lean.candidateLegs
    candidateCoveredPaths ++ [s!"{path}.cached_count"]

def mergedOutputCoveredPaths (path : String) (rust : RustMergedScanExtraction)
    (lean : MergedScanOutputMirror) : List String :=
  listCoveredPaths s!"{path}.points" rust.points lean.points bspCoveredPaths ++
  listCoveredPaths s!"{path}.pan_divs" rust.panDivs lean.panDivs panCoveredPaths ++
  listCoveredPaths s!"{path}.grades" rust.grades lean.grades gradeCoveredPaths ++
  listCoveredPaths s!"{path}.candidate_legs" rust.observations lean.observations
    candidateCoveredPaths

def centerFrameCoveredPaths (path : String) : List String :=
  [s!"{path}.start_index", s!"{path}.end_index", s!"{path}.zd", s!"{path}.zg",
   s!"{path}.dd", s!"{path}.gg"]

def repeatedPaths (count : Nat) (paths : List String) : List String :=
  (List.range count).flatMap fun _ => paths

def p10RawCoveredPaths (raw : MergedScanResumeRaw) (_cacheBefore : RustFourCacheExtraction)
    (valid : Bool) : List String :=
  let segmentPaths := ["P10.segment_end_indices.length", "P10.segment_end_indices.canonical_order"] ++
    repeatedPaths raw.segments.length ["P10.segment_end_indices.item.value"]
  let centerPaths := ["P10.centers.length", "P10.centers.canonical_order"] ++
    repeatedPaths raw.centers.length (centerFrameCoveredPaths "P10.centers.item")
  let freezePayload := if 2 <= raw.prefixCount && (raw.centers[raw.prefixCount - 2]?).isSome
    then ["P10.freeze_boundary_src.value"] else []
  ["P10.cached_count_before", "P10.prefix_count", "P10.dirty_e",
   "P10.freeze_boundary_src.tag"] ++ freezePayload ++ segmentPaths ++ centerPaths ++
  if valid then
    ["P01.segment_start_order", "P01.center_end_strict_order",
     "P02.anchor_structural_direction", "P02.parallel_array_length",
     "P03.nearest_confirmed_center", "P04.incoming_trend_gate",
     "P05.consolidation_lift_zero", "P06.first_match_identity",
     "P07.a_segment_cache", "P08.lambda_c_episode", "P09.structural_gate", "P10.stable_seg"]
  else []

def bspListFieldCount (points : List RustBspPointExtraction) : Nat :=
  points.foldl (fun total point => total + bspFieldCount point) 1

def candidateListFieldCount (observations : List RustCandidateExtraction) : Nat :=
  observations.foldl (fun total observation => total + candidateFieldCount observation) 1

def panListFieldCount (panDivs : List RustPanDivExtraction) : Nat :=
  1 + panDivs.length * 12

def gradeFieldCount (grade : RustGradeExtraction) : Nat :=
  7 + match grade.grade with
    | .present _ _ _ _ => 5
    | .missing _ => 2

def gradeListFieldCount (grades : List RustGradeExtraction) : Nat :=
  grades.foldl (fun total grade => total + gradeFieldCount grade) 1

def mergedOutputFieldCount (output : RustMergedScanExtraction) : Nat :=
  bspListFieldCount output.points + panListFieldCount output.panDivs +
    gradeListFieldCount output.grades + candidateListFieldCount output.observations

def fourCacheFieldCount (cache : RustFourCacheExtraction) : Nat :=
  bspListFieldCount cache.points + panListFieldCount cache.panDivs +
    gradeListFieldCount cache.grades + candidateListFieldCount cache.candidateLegs + 1

def scanEmissionFieldCount (emission : RustScanEmissionExtraction) : Nat :=
  bspListFieldCount emission.points + panListFieldCount emission.panDivs +
    gradeListFieldCount emission.grades + candidateListFieldCount emission.candidateLegs

def checkMergedScan (path : String) (series : MergedScanRawSeries) (raw : MergedScanResumeRaw)
    (cacheBefore : RustFourCacheExtraction) (rustStableSeg : Nat)
    (rustConfirmedAppend : RustScanEmissionExtraction) (cacheAfter : RustFourCacheExtraction)
    (rustTail : RustScanEmissionExtraction) (rustOutput : RustMergedScanExtraction) : RuntimeCheck :=
  let expected := mergedScanResumeMirror series raw cacheBefore.toLean
  let failures := match expected with
    | none => [s!"{path}.raw_input_guard"]
    | some lean =>
        failedPath s!"{path}.stable_seg" (rustStableSeg == lean.stableSeg) ++
        scanEmissionMismatchPaths s!"{path}.confirmed_append"
          rustConfirmedAppend lean.confirmedAppend ++
        fourCacheMismatchPaths s!"{path}.cache_after" cacheAfter lean.cacheAfter ++
        scanEmissionMismatchPaths s!"{path}.tail" rustTail lean.tail ++
        mergedOutputMismatchPaths s!"{path}.output" rustOutput lean.output
  let details := failedDetail path (reprStr expected)
    (reprStr (rustStableSeg, rustConfirmedAppend.toLean, cacheAfter.toLean,
      rustTail.toLean, rustMergedToMirror rustOutput)) failures.isEmpty
  let covered := p10RawCoveredPaths raw cacheBefore expected.isSome ++ match expected with
    | none => []
    | some lean =>
        scanEmissionCoveredPaths "P10.confirmed_append" rustConfirmedAppend lean.confirmedAppend ++
        fourCacheCoveredPaths "P10.cache_after" cacheAfter lean.cacheAfter ++
        scanEmissionCoveredPaths "P10.tail" rustTail lean.tail ++
        mergedOutputCoveredPaths "P10.final_output" rustOutput lean.output
  { path := path, port := "merged_scan", level := raw.level, side := "NotYetKnown"
    terminal := "NoCpApplicable"
    comparedFields := 1 + scanEmissionFieldCount rustConfirmedAppend +
      fourCacheFieldCount cacheAfter + scanEmissionFieldCount rustTail +
      mergedOutputFieldCount rustOutput
    passed := failures.isEmpty, failedFields := failures, failedDetails := details
    coveredPaths := covered }

/-- Checkpoint 四产口从 raw frontier 输入现场重算；禁止调用方回填 memo 列表作 expected。 -/
def checkCheckpointOutput (path : String) (series : MergedScanRawSeries)
    (raw : MergedScanResumeRaw) (cacheBefore : RustFourCacheExtraction)
    (rust : RustMergedScanExtraction) : RuntimeCheck :=
  let expected := (mergedScanResumeMirror series raw cacheBefore.toLean).map (fun value => value.output)
  let failures := match expected with
    | none => [s!"{path}.raw_input_guard"]
    | some lean => mergedOutputMismatchPaths s!"{path}.output" rust lean
  let passed := failures.isEmpty
  let details := failedDetail s!"{path}.four_port_list_parity" (reprStr expected)
    (reprStr (rustMergedToMirror rust)) passed
  let covered := p10RawCoveredPaths raw cacheBefore expected.isSome ++ match expected with
    | none => []
    | some lean => mergedOutputCoveredPaths "P10.final_output" rust lean
  { path := path, port := "checkpoint_output", level := raw.level, side := "NotYetKnown"
    terminal := "NoCpApplicable"
    comparedFields := mergedOutputFieldCount rust
    passed := failures.isEmpty, failedFields := failures, failedDetails := details
    coveredPaths := covered }

def rawThirdRustCenterWitness : RustCenterExtraction :=
  { startIndex := 1, endIndex := 5, zd := 10, zg := 20, dd := 0, gg := 0 }

def rawThirdRustEntryWitness : RustThirdClassEntryExtraction :=
  { centerSi := 1, centerZd := 10, centerZg := 20
    leaveLeft := 6, leaveRight := 7, retestLeft := 8, retestRight := 9 }

def rawThirdRustBitsWitness : RustBspBitsExtraction :=
  { buy3 := true, thirdClassEntry := some rawThirdRustEntryWitness }

def rawThirdRustPointWitness : RustBspPointExtraction :=
  { sourceIndex := 9
    bits := rawThirdRustBitsWitness
    pivotLow := 25
    pivotHigh := 0
    owner := some (.center rawThirdRustCenterWitness)
    structBreakDir := none
    force := none
    retraceBreaksType1 := none }

def rawThirdEmptyCacheWitness : RustFourCacheExtraction :=
  { points := [], panDivs := [], grades := [], candidateLegs := [], cachedCount := 0 }

def rawThirdEmptyEmissionWitness : RustScanEmissionExtraction :=
  { points := [], panDivs := [], grades := [], candidateLegs := [] }

def rawThirdTailWitness : RustScanEmissionExtraction :=
  { points := [rawThirdRustPointWitness], panDivs := [], grades := [], candidateLegs := [] }

def rawThirdOutputWitness : RustMergedScanExtraction :=
  { points := [rawThirdRustPointWitness], panDivs := [], grades := [], observations := [] }

def rawThirdRunnerSmoke : RuntimeCheck :=
  checkMergedScan "raw.third.smoke" rawThirdSeriesWitness rawThirdResumeWitness
    rawThirdEmptyCacheWitness 0 rawThirdEmptyEmissionWitness rawThirdEmptyCacheWitness
    rawThirdTailWitness rawThirdOutputWitness

theorem raw_third_runner_smoke_passes : rawThirdRunnerSmoke.passed = true := by
  native_decide

/-- 07b 不进入 3a 镜面；这里只锁 pipeline 的 `direct ++ second07b` 稳定合并。 -/
def checkPipelinePointComposition (path : String) (level : Nat)
    (direct second07b pipeline : List RustBspPointExtraction) : RuntimeCheck :=
  let expected := stableSortBySource BspPointMirror.sourceIndex
    ((direct ++ second07b).map rustBspToMirror)
  let failures := listMismatchPathsAux path 0 pipeline expected bspMismatchPaths
  let directCovered := listCoveredPaths "P10.memo.direct_3a_points" direct
    (direct.map rustBspToMirror) bspCoveredPaths
  let secondCovered := listCoveredPaths "P10.memo.second_07b_points" second07b
    (second07b.map rustBspToMirror) bspCoveredPaths
  let pipelineCovered := listCoveredPaths "P10.memo.pipeline_points" pipeline expected bspCoveredPaths
  { path := path, port := "pipeline_07b_composition", level := level
    side := "NotYetKnown", terminal := "NoCpApplicable"
    comparedFields := bspListFieldCount pipeline
    passed := failures.isEmpty, failedFields := failures
    failedDetails := failedDetail path (reprStr expected)
      (reprStr (pipeline.map rustBspToMirror)) failures.isEmpty
    coveredPaths := directCovered ++ secondCovered ++ pipelineCovered ++
      ["P10.memo.pipeline_07b_composition"] }

/-! ## Deterministic schema coverage fixtures

这些记录仍走与窗口数据相同的逐字段 parity 入口；它们只补稀有 union/Option 载荷覆盖，
不计作生产提取记录，也不替代四窗 0 mismatch。
-/

def asCoverageFixture (name : String) (check : RuntimeCheck) : RuntimeCheck :=
  { check with
    path := s!"coverage.{name}"
    port := "coverage_fixture"
    side := "Fixture"
    terminal := "Fixture" }

def coverageCenter : CenterFrame :=
  { startIndex := 10, endIndex := 20, zd := 100, zg := 120, dd := 90, gg := 130 }

def coverageSegment : SegmentRow :=
  { direction := .down, startIndex := 21, endIndex := 30, endPrice := 80
    anchor := some .down, departureEnd := some 29 }

def coverageGates : FirstStructuralGates :=
  { side := .long, segA := { left := 1, right := 9 }, lambdaC := 21
    aMapped := true, cMapped := true, extreme := true }

def coverageFirstInput (grade : Grade) : FirstProjectionInput :=
  { level := 0, center := coverageCenter, segment := coverageSegment, gates := coverageGates
    diverged := true, t3Present := true, grade := grade }

def coverageAnchorLean : BspPointMirror :=
  { sourceIndex := 41, bits := {}, pivotLow := 0, pivotHigh := 0
    owner := some (.type1Anchor 37), structBreakDir := none, force := none
    retraceBreaksType1 := some true }

def checkBspCoverageFixture (name : String) (rust : RustBspPointExtraction)
    (expected : BspPointMirror) : RuntimeCheck :=
  let failures := bspMismatchPaths s!"coverage.{name}" rust expected
  { path := s!"coverage.{name}", port := "coverage_fixture", level := 0
    side := "Fixture", terminal := "Fixture", comparedFields := bspFieldCount rust
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := bspCoveredPaths "O01" rust expected }

def coverageThirdFailure : ThirdProjectionInput :=
  { thirdBuyWitness with retest := { thirdBuyWitness.retest with endPrice := 20 } }

def coverageGradeCheck (name : String) (grade : Grade) (rustGrade : RustGradeExtraction) :
    RuntimeCheck :=
  asCoverageFixture name <| checkGrade name 0 "Fixture" (coverageFirstInput grade)
    (some rustGrade)

def coverageCandidateKey : CandidateKeyMirror :=
  { ruleVersion := 1, level := 2, kind := .trend, side := .short
    previousCenterStart := some 11, parentCenterStart := 21, parentZd := 100, parentZg := 120
    segA := { left := 1, right := 9 }, lambdaC := 22 }

def coverageCandidateLean : CandidateObservationMirror :=
  { key := coverageCandidateKey
    kind := .trend, centerIds := some (3, 4), candidateGroupId := 101, pairId := 202
    predicates := { direction := true, comparable := true, extreme := true }
    extremeProof := { left := 5, right := 6 }, thirdClassProof := some 7
    interval := { left := 22, right := 30 }, state := .confirmed
    firstProvableAt := some 30, confirmedAt := none }

def coverageCandidateNoneLean : CandidateObservationMirror :=
  { coverageCandidateLean with
    key := { coverageCandidateLean.key with previousCenterStart := none }
    centerIds := none
    predicates := { coverageCandidateLean.predicates with extreme := false }
    thirdClassProof := none
    firstProvableAt := none
    confirmedAt := some 31 }

def checkCandidateCoverageFixture (name : String) (rust : RustCandidateExtraction)
    (expected : CandidateObservationMirror) : RuntimeCheck :=
  let failures := candidateMismatchPaths s!"coverage.{name}" rust expected
  { path := s!"coverage.{name}", port := "coverage_fixture", level := rust.level
    side := "Fixture", terminal := "Fixture", comparedFields := candidateFieldCount rust
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := candidateCoveredPaths "O04" rust expected }

def covId (level ordinal : Nat) : ElementIdMirror := { level := level, ordinal := ordinal }
def coverageThirdCp : ThirdClassInCpMirror :=
  { bCenterId := covId 2 3, cpDepartureMoveId := covId 1 5
    departureMoveId := covId 1 6, retestMoveId := covId 1 7
    departureInterval := { left := 21, right := 25 }, retestInterval := { left := 26, right := 30 }
    pointSourceIndex := 30, side := .long }

def coverageCpStructure : CpStructureIdentityMirror :=
  { level := 1, bCenterId := covId 2 3, departureMoveId := covId 1 5
    terminalMoveId := some (covId 1 7), sourceStart := 21, sourceEnd := some 30 }

def coverageCpLean : CpOwnershipFullMirror :=
  let context : TrendContextMirror :=
    { predecessorCenterId := covId 2 2, bCenterId := covId 2 3, direction := .up }
  let extreme : NewExtremeInDirectionMirror :=
    { bCenterId := covId 2 3, direction := .up, referencePrice := 130, extremePrice := 140
      extremeMoveId := covId 1 6, confirmSrc := 25 }
  let internal : InternalSublevelCentersMirror :=
    { cLevel := 1, centerIds := [covId 1 5, covId 1 6] }
  let completed : CompletedTrendDecompositionMirror :=
    { direction := .up, centerIds := internal.centerIds
      closingSuccessorMoveId := covId 1 8, confirmSrc := 35 }
  let evidence : FullTrendQualificationEvidenceMirror :=
    { trendContext := some context, newExtremeInDirection := some extreme
      internalSublevelCenters := some internal, completedTrendDecomposition := some completed
      decompositionReviewMoveId := some (covId 1 8), decompositionReviewSrc := some 35 }
  { bCenterIndex := 3, bCenterId := covId 2 3, bCenter := coverageCenter
    departureMoveId := some (covId 1 5), departureInterval := some { left := 21, right := 25 }
    lifecycle := .closed, cpCertificateConfirmSrc := some 30
    cStructure := some coverageCpStructure
    thirdClassInC := some coverageThirdCp, fullTrendEvidence := some evidence
    fullTrendCQualified := some
      { trendContext := context, thirdClassInsideC := coverageThirdCp
        newExtremeInDirection := extreme, internalSublevelCenters := internal
        completedTrendDecomposition := completed, confirmSrc := 35 } }

/--
The extractor must serialize every `Rust*` field below independently.  Keeping the actual wire in
this argument prevents deterministic schema coverage from degenerating into Lean-to-Lean constants.
-/
structure P10CoverageCaseExtraction where
  series : MergedScanRawSeries
  raw : MergedScanResumeRaw
  stableSeg : Nat
  cacheBefore : RustFourCacheExtraction
  confirmedAppend : RustScanEmissionExtraction
  cacheAfter : RustFourCacheExtraction
  tail : RustScanEmissionExtraction
  output : RustMergedScanExtraction
deriving Repr

/-- Independent wire for the real production 07b diagnostic adapter.  `output` is compared only
after the Lean bridge has recomputed it from `center/side/level/legs`. -/
structure SecondProductionExtraction where
  center : RustCenterExtraction
  side : RustSideTag
  level : Nat
  legs : List SecondLegRawMirror
  output : List RustBspPointExtraction
deriving Repr

def checkSecondProduction (wire : SecondProductionExtraction) : RuntimeCheck :=
  let expected := extractSecondSignalsMirror wire.center.toLean wire.side.toLean wire.level wire.legs
  let failures := listMismatchPathsAux "second_production.output" 0 wire.output expected
    bspMismatchPaths
  let p10Covered := listCoveredPaths "P10.memo.second_07b_points" wire.output expected
    bspCoveredPaths
  let o01Covered := flatListCoveredPaths "O01" wire.output expected bspCoveredPaths
  { path := "coverage.p10.second_production", port := "second_production"
    level := wire.level
    side := match wire.side with | .long => "Long" | .short => "Short"
    terminal := "NoCpApplicable"
    comparedFields := bspListFieldCount wire.output
    passed := failures.isEmpty
    failedFields := failures
    failedDetails := failedDetail "second_production.output" (reprStr expected)
      (reprStr (wire.output.map rustBspToMirror)) failures.isEmpty
    coveredPaths := p10Covered ++ o01Covered }

structure CoverageFixtureExtraction where
  firstSome : Option RustBspPointExtraction
  firstNone : Option RustBspPointExtraction
  forceSegA : ForceSegmentRawMirror
  forceSegC : ForceSegmentRawMirror
  firstForce : Option RustBspPointExtraction
  ownerType1 : RustBspPointExtraction
  secondProduction : SecondProductionExtraction
  thirdSuccess : Option RustBspPointExtraction
  thirdFailure : Option RustBspPointExtraction
  gradePresent : RustGradeExtraction
  gradeMissingLeave : RustGradeExtraction
  gradeMissingRetest : RustGradeExtraction
  gradeSameDirection : RustGradeExtraction
  gradeLeaveNotOutside : RustGradeExtraction
  gradeRetestReentered : RustGradeExtraction
  candidateExtremeTrue : RustCandidateExtraction
  candidateExtremeFalse : RustCandidateExtraction
  cpFull : RustCpFullExtraction
  p10Cache : P10CoverageCaseExtraction
  p10First : P10CoverageCaseExtraction
  p10FirstTail : P10CoverageCaseExtraction
  p10Pan : P10CoverageCaseExtraction
  p10Third : P10CoverageCaseExtraction
  pipelineDirect : List RustBspPointExtraction
  pipelineSecond07b : List RustBspPointExtraction
  pipelineOutput : List RustBspPointExtraction
deriving Repr

def checkP10CoverageCase (name : String) (wire : P10CoverageCaseExtraction)
    (requiredPaths : List String) : RuntimeCheck :=
  let parity := checkMergedScan name wire.series wire.raw wire.cacheBefore wire.stableSeg
    wire.confirmedAppend wire.cacheAfter wire.tail wire.output
  let failures := parity.failedFields ++ requiredPaths
  { parity with
    path := s!"coverage.p10.{name}"
    port := "coverage_fixture"
    side := "Fixture"
    terminal := "Fixture"
    comparedFields := parity.comparedFields + requiredPaths.length
    passed := failures.isEmpty
    failedFields := failures }

def checkP10CacheCoverage (wire : P10CoverageCaseExtraction) : RuntimeCheck :=
  checkP10CoverageCase "cache_frontier_pan" wire <|
    failedPath "coverage.p10.cache.confirmed_append.pan_divs" (!wire.confirmedAppend.panDivs.isEmpty) ++
    failedPath "coverage.p10.cache.cache_after.pan_divs" (!wire.cacheAfter.panDivs.isEmpty) ++
    failedPath "coverage.p10.cache.output.pan_divs" (!wire.output.panDivs.isEmpty) ++
    failedPath "coverage.p10.cache.output.observations" (!wire.output.observations.isEmpty)

def checkP10FirstCoverage (wire : P10CoverageCaseExtraction) : RuntimeCheck :=
  checkP10CoverageCase "first_grade_candidate_confirmed" wire <|
    failedPath "coverage.p10.first.stable_positive" (0 < wire.stableSeg) ++
    failedPath "coverage.p10.first.confirmed_append.points" (!wire.confirmedAppend.points.isEmpty) ++
    failedPath "coverage.p10.first.confirmed_append.grades" (!wire.confirmedAppend.grades.isEmpty) ++
    failedPath "coverage.p10.first.confirmed_append.candidates"
      (!wire.confirmedAppend.candidateLegs.isEmpty) ++
    failedPath "coverage.p10.first.cache_after.points" (!wire.cacheAfter.points.isEmpty) ++
    failedPath "coverage.p10.first.cache_after.grades" (!wire.cacheAfter.grades.isEmpty) ++
    failedPath "coverage.p10.first.cache_after.candidates" (!wire.cacheAfter.candidateLegs.isEmpty) ++
    failedPath "coverage.p10.first.output.points" (!wire.output.points.isEmpty) ++
    failedPath "coverage.p10.first.output.grades" (!wire.output.grades.isEmpty) ++
    failedPath "coverage.p10.first.output.observations" (!wire.output.observations.isEmpty)

def checkP10FirstTailCoverage (wire : P10CoverageCaseExtraction) : RuntimeCheck :=
  checkP10CoverageCase "first_grade_candidate_tail" wire <|
    failedPath "coverage.p10.first_tail.stable_zero" (wire.stableSeg == 0) ++
    failedPath "coverage.p10.first_tail.tail.grades" (!wire.tail.grades.isEmpty) ++
    failedPath "coverage.p10.first_tail.tail.candidates" (!wire.tail.candidateLegs.isEmpty) ++
    failedPath "coverage.p10.first_tail.output.grades" (!wire.output.grades.isEmpty) ++
    failedPath "coverage.p10.first_tail.output.observations" (!wire.output.observations.isEmpty)

def checkP10PanCoverage (wire : P10CoverageCaseExtraction) : RuntimeCheck :=
  checkP10CoverageCase "pan_tail" wire <|
    failedPath "coverage.p10.pan.stable_zero" (wire.stableSeg == 0) ++
    failedPath "coverage.p10.pan.tail.pan_divs" (!wire.tail.panDivs.isEmpty) ++
    failedPath "coverage.p10.pan.output.pan_divs" (!wire.output.panDivs.isEmpty) ++
    failedPath "coverage.p10.pan.output.observations" (!wire.output.observations.isEmpty)

def checkP10ThirdCoverage (wire : P10CoverageCaseExtraction) : RuntimeCheck :=
  checkP10CoverageCase "third_tail" wire <|
    failedPath "coverage.p10.third.stable_zero" (wire.stableSeg == 0) ++
    failedPath "coverage.p10.third.tail.points" (!wire.tail.points.isEmpty) ++
    failedPath "coverage.p10.third.output.points" (!wire.output.points.isEmpty)

def coverageFixtureChecks (wire : CoverageFixtureExtraction) : List RuntimeCheck :=
  let forceInput : FirstProjectionInput :=
    { coverageFirstInput (.present { left := 1, right := 2 } { left := 3, right := 4 }) with
      forceSegA := some wire.forceSegA
      forceSegC := some wire.forceSegC }
  [ asCoverageFixture "o01.first_some" <| checkFirst "fixture" 0 "Fixture"
      (coverageFirstInput (.present { left := 1, right := 2 } { left := 3, right := 4 }))
      wire.firstSome
  , asCoverageFixture "o01.force_raw" <| checkFirst "fixture" 0 "Fixture"
      forceInput wire.firstForce
  , asCoverageFixture "o01.first_none" <| checkFirst "fixture" 0 "Fixture"
      { coverageFirstInput (.missing .missingLeave) with
        gates := { coverageGates with extreme := false } } wire.firstNone
  , checkBspCoverageFixture "o01.owner_type1_anchor" wire.ownerType1 coverageAnchorLean
  , asCoverageFixture "o01.third_success" <| checkThird "fixture" 0 "Fixture"
      thirdBuyWitness wire.thirdSuccess
  , asCoverageFixture "o01.third_failure" <| checkThird "fixture" 0 "Fixture"
      coverageThirdFailure wire.thirdFailure
  , coverageGradeCheck "o03.present" (.present { left := 1, right := 2 } { left := 3, right := 4 })
      wire.gradePresent
  , coverageGradeCheck "o03.missing_leave" (.missing .missingLeave) wire.gradeMissingLeave
  , coverageGradeCheck "o03.missing_retest" (.missing .missingRetest) wire.gradeMissingRetest
  , coverageGradeCheck "o03.same_direction" (.missing .sameDirection) wire.gradeSameDirection
  , coverageGradeCheck "o03.leave_not_outside" (.missing .leaveNotOutside)
      wire.gradeLeaveNotOutside
  , coverageGradeCheck "o03.retest_reentered" (.missing .retestReentered)
      wire.gradeRetestReentered
  , checkCandidateCoverageFixture "o04.extreme_true.third_some"
      wire.candidateExtremeTrue coverageCandidateLean
  , checkCandidateCoverageFixture "o04.extreme_false.third_none"
      wire.candidateExtremeFalse coverageCandidateNoneLean
  , asCoverageFixture "cp.full_qualified" <|
      checkCpAttachment "fixture" 2 "Closed" wire.cpFull coverageCpLean
  , checkP10CacheCoverage wire.p10Cache
  , checkP10FirstCoverage wire.p10First
  , checkP10FirstTailCoverage wire.p10FirstTail
  , checkP10PanCoverage wire.p10Pan
  , checkP10ThirdCoverage wire.p10Third
  , checkSecondProduction wire.secondProduction
  , asCoverageFixture "p10.checkpoint_populated" <| checkCheckpointOutput "fixture"
      wire.p10Cache.series wire.p10Cache.raw wire.p10Cache.cacheBefore wire.p10Cache.output
  , asCoverageFixture "p10.pipeline_nonempty" <| checkPipelinePointComposition "fixture"
      wire.p10First.raw.level wire.pipelineDirect wire.pipelineSecond07b wire.pipelineOutput ]

structure FourRcMemoExtraction where
  key : Nat
  cachedKey : Option Nat
  hit : Bool
  priorReused : List Bool
  afterPtrEq : List Bool
  beforeLengths : List Nat
  afterLengths : List Nat
  expectedPoints : List RustBspPointExtraction
  actualPoints : List RustBspPointExtraction
  expectedPanDivs : List RustPanDivExtraction
  actualPanDivs : List RustPanDivExtraction
  expectedGrades : List RustGradeExtraction
  actualGrades : List RustGradeExtraction
  expectedCandidateLegs : List RustCandidateExtraction
  actualCandidateLegs : List RustCandidateExtraction
deriving Repr

def fourRcValueMismatchPaths (path : String) (memo : FourRcMemoExtraction) : List String :=
  listMismatchPathsAux s!"{path}.values.points" 0 memo.expectedPoints
      (memo.actualPoints.map rustBspToMirror) bspMismatchPaths ++
    listMismatchPathsAux s!"{path}.values.pan_divs" 0 memo.expectedPanDivs
      (memo.actualPanDivs.map rustPanToMirror) panMismatchPaths ++
    listMismatchPathsAux s!"{path}.values.grades" 0 memo.expectedGrades
      (memo.actualGrades.map rustGradeToMirror) gradeMismatchPaths ++
    listMismatchPathsAux s!"{path}.values.candidate_legs" 0 memo.expectedCandidateLegs
      (memo.actualCandidateLegs.map rustCandidateToMirror) candidateMismatchPaths

def fourRcValueFieldCount (memo : FourRcMemoExtraction) : Nat :=
  bspListFieldCount memo.actualPoints + panListFieldCount memo.actualPanDivs +
    gradeListFieldCount memo.actualGrades + candidateListFieldCount memo.actualCandidateLegs

def memoNatListCoveredPaths (path : String) (values : List Nat) (itemsCompared : Bool) : List String :=
  [s!"{path}.length", s!"{path}.canonical_order"] ++
    if itemsCompared then repeatedPaths values.length [s!"{path}.item.value"] else []

def fourRcCoveredPaths (memo : FourRcMemoExtraction) : List String :=
  let keyPaths := ["P10.memo.key.centers", "P10.memo.key.upper_moves",
    "P10.memo.key.structures", "P10.memo.cached_key_before.tag"] ++
    if memo.cachedKey.isSome then
      ["P10.memo.cached_key_before.centers", "P10.memo.cached_key_before.upper_moves",
       "P10.memo.cached_key_before.structures"] else []
  let arrays := memoNatListCoveredPaths "P10.memo.prior_reused" (memo.priorReused.map Bool.toNat)
      memo.hit ++
    memoNatListCoveredPaths "P10.memo.after_ptr_eq" (memo.afterPtrEq.map Bool.toNat) memo.hit ++
    memoNatListCoveredPaths "P10.memo.before_lengths" memo.beforeLengths memo.hit ++
    memoNatListCoveredPaths "P10.memo.lengths" memo.afterLengths memo.hit
  let values := if memo.hit then
    listCoveredPaths "P10.memo.pipeline_points" memo.expectedPoints
      (memo.actualPoints.map rustBspToMirror) bspCoveredPaths ++
    listCoveredPaths "P10.memo.pan_divs" memo.expectedPanDivs
      (memo.actualPanDivs.map rustPanToMirror) panCoveredPaths ++
    listCoveredPaths "P10.memo.grades" memo.expectedGrades
      (memo.actualGrades.map rustGradeToMirror) gradeCoveredPaths ++
    listCoveredPaths "P10.memo.candidate_legs" memo.expectedCandidateLegs
      (memo.actualCandidateLegs.map rustCandidateToMirror) candidateCoveredPaths
    else []
  keyPaths ++ ["P10.memo.hit"] ++ arrays ++ values

/-- P-10 pipeline memo is one key and exactly four Rc values; a hit must reuse all four. -/
def checkFourRcMemo (path : String) (level : Nat) (memo : FourRcMemoExtraction) : RuntimeCheck :=
  let shape := memo.priorReused.length == 4 && memo.afterPtrEq.length == 4 &&
    memo.beforeLengths.length == 4 && memo.afterLengths.length == 4
  let expectedHit := memo.cachedKey == some memo.key
  let lockstep := if memo.hit then memo.priorReused.all id && memo.afterPtrEq.all id else true
  let lengthLockstep := if memo.hit then memo.beforeLengths == memo.afterLengths else true
  let valueFailures := if memo.hit then fourRcValueMismatchPaths path memo else []
  let failures := failedPath s!"{path}.four_slot_shape" shape ++
    failedPath s!"{path}.hit" (memo.hit == expectedHit) ++
    failedPath s!"{path}.lockstep_reuse" lockstep ++
    failedPath s!"{path}.length_lockstep" lengthLockstep ++ valueFailures
  { path := path, port := "four_rc_memo", level := level, side := "NotYetKnown"
    terminal := "NoCpApplicable"
    comparedFields := 10 + if memo.hit then fourRcValueFieldCount memo else 0
    passed := failures.isEmpty, failedFields := failures
    coveredPaths := fourRcCoveredPaths memo }

def mismatchFields (checks : List RuntimeCheck) : Nat :=
  checks.foldl (fun total check =>
    if check.passed then total
    else total + if check.failedFields.isEmpty then check.comparedFields else check.failedFields.length) 0

def checkMismatchFields (check : RuntimeCheck) : Nat :=
  if check.passed then 0
  else if check.failedFields.isEmpty then check.comparedFields else check.failedFields.length

def comparedFields (checks : List RuntimeCheck) : Nat :=
  checks.foldl (fun total check => total + check.comparedFields) 0

def portCount (checks : List RuntimeCheck) (port : String) : Nat :=
  (checks.filter fun check => check.port == port).length

def portComparedFields (checks : List RuntimeCheck) (port : String) : Nat :=
  (checks.filter fun check => check.port == port).foldl
    (fun total check => total + check.comparedFields) 0

def portMismatchFields (checks : List RuntimeCheck) (port : String) : Nat :=
  (checks.filter fun check => check.port == port).foldl
    (fun total check => total + checkMismatchFields check) 0

def portFailureCount (checks : List RuntimeCheck) (port : String) : Nat :=
  (checks.filter fun check => check.port == port && !check.passed).length

def emitReport (windowId : String) (checks : List RuntimeCheck) : IO Unit := do
  let failures := checks.filter fun check => !check.passed
  -- Real-window checks have unique per-record paths, but coverage is a schema-level
  -- certificate.  Keep one producing check per schema leaf in each chunk instead of
  -- repeating the same leaf for every record.  Named coverage fixtures remain
  -- separate evidence classes because Rust validates their exact production port.
  let mut coveredSeen : Std.HashSet (String × String) := {}
  let mut coveredPairs : Array (String × String) := #[]
  IO.println s!"S2_LEAN_SUMMARY\t{windowId}\t{checks.length}\t{comparedFields checks}\t{mismatchFields checks}"
  for port in ["prelude", "points", "pan", "grades", "observations", "reduction", "cp",
      "cp_init", "cp_dirty", "cp_reinherit", "cp_advance",
      "cp_review",
      "merged_scan", "checkpoint_output", "pipeline_07b_composition", "four_rc_memo",
      "coverage_fixture"] do
    IO.println s!"S2_LEAN_PORT\t{windowId}\t{port}\t{portCount checks port}"
    IO.println s!"S2_LEAN_PORT_MISMATCH\t{windowId}\t{port}\t{portComparedFields checks port}\t{portFailureCount checks port}\t{portMismatchFields checks port}"
  for check in checks do
    IO.println s!"S2_LEAN_LAYER\t{windowId}\tL{check.level}\t{check.side}\t{check.terminal}\t{check.port}\t{check.comparedFields}\t{checkMismatchFields check}"
    IO.println s!"S2_LEAN_CHECK\t{windowId}\t{check.path}\t{check.port}\t{if check.passed then 1 else 0}\t{check.comparedFields}\t{checkMismatchFields check}"
    for covered in check.coveredPaths do
      let evidenceClass := if check.path.startsWith "coverage." then check.path else "real_window"
      let seenKey := (evidenceClass, covered)
      if !coveredSeen.contains seenKey then
        coveredSeen := coveredSeen.insert seenKey
        coveredPairs := coveredPairs.push (check.path, covered)
  for pair in coveredPairs do
    IO.println s!"S2_LEAN_COVERED\t{pair.1}\t{pair.2}"
  for failure in failures.take 20 do
    IO.eprintln s!"S2_LEAN_MISMATCH\t{failure.path}\t{failure.port}\tL{failure.level}\t{failure.side}\t{failure.terminal}"
    for field in failure.failedFields.take 20 do
      IO.eprintln s!"S2_LEAN_FIELD_MISMATCH\t{field}"
    for detail in failure.failedDetails.take 20 do
      IO.eprintln s!"S2_LEAN_FIELD_DETAIL\t{detail.1}\texpected={detail.2.1}\tactual={detail.2.2}"
  if !failures.isEmpty then
    throw <| IO.userError s!"{windowId}: {failures.length} Lean mirror records mismatched"

theorem reduction_runtime_check_counts_length
    (path side : String) (level : Nat) (legs : List CandidateObservationMirror)
    (rust : List RustCandidateExtraction) :
    (checkReduction path level side legs rust).comparedFields =
      rust.foldl (fun total item => total + candidateFieldCount item) 1 := by
  rfl

theorem cp_runtime_check_uses_dynamic_recursive_count
    (path terminal : String) (level : Nat) (rust : RustCpFullExtraction)
    (expected : CpOwnershipFullMirror) :
    (checkCpAttachment path level terminal rust expected).comparedFields =
      cpFullDynamicFieldCount rust := by
  rfl

theorem force_l_missing_never_diverges (c : Option Nat) :
    forceLDivergedBits none c = false := by
  cases c <;> rfl

theorem grade_fixed_pair_missing_leave (center : CenterFrame) (trend : Direction)
    (retest : Option SegmentRow) :
    gradeFixedFirstPair center trend none retest = Grade.missing GradeReason.missingLeave := by
  cases retest <;> rfl

end NewChanlun.Origin.UnifiedScanMirrorRunner
