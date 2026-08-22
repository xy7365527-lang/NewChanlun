/-
Origin/UnifiedScanMirrorRunner.lean

#1080 Phase 2 的机器执行层。Rust 提取器生成只含原始输入与 Rust wire 的 Lean
fixture，本文件负责现场执行 Phase 1 镜面定义并以非零退出拒绝任何 mismatch。
-/

import Origin.UnifiedScanMirrorBridge

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
deriving Repr

def rustBspToMirror (rust : RustBspPointExtraction) : BspPointMirror :=
  { sourceIndex := rust.sourceIndex, side := rust.side.toLean,
    type1Bit := rust.type1Bit, type3Bit := rust.type3Bit,
    structureBreak := rust.structureBreak, ownerCenterStart := rust.ownerCenterStart }

def rustGradeToMirror (rust : RustGradeExtraction) : FirstClassGradeRecordMirror :=
  { level := rust.level, sourceIndex := rust.sourceIndex, side := rust.side.toLean,
    centerStart := rust.centerStart, centerEnd := rust.centerEnd,
    centerZd := rust.centerZd, centerZg := rust.centerZg, grade := rust.grade.toLean }

def rustCandidateToMirror (rust : RustCandidateExtraction) : CandidateObservationMirror :=
  let key : CandidateKeyMirror :=
    { level := rust.level
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
    predicates := predicates
    interval := { left := rust.intervalLeft, right := rust.intervalRight }
    state := rust.state.toLean, firstProvableAt := rust.firstProvableAt,
    confirmedAt := rust.confirmedAt }

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
    passed := orderedResumeGuard rows centers }

/-- 空产口也必须进入运行时清单；`length` 是 Rust serializer 的直接读数，不冒充内容 parity。 -/
def checkPortEnumeration (path port : String) (level count : Nat) : RuntimeCheck :=
  { path := s!"{path}.length={count}", port := port, level := level,
    side := "NotYetKnown", terminal := "NoCpApplicable", comparedFields := 1, passed := true }

def checkFirst (path : String) (level : Nat) (side : String)
    (input : FirstProjectionInput) (rust : RustBspPointExtraction) : RuntimeCheck :=
  { path := path, port := "points", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 6,
    passed := (judgeFirstFromGates input).map (rustBspToMirror rust == ·) |>.getD false }

def checkGrade (path : String) (level : Nat) (side : String)
    (input : FirstProjectionInput) (rust : Option RustGradeExtraction) : RuntimeCheck :=
  { path := path, port := "grades", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 12,
    passed := match rust, assembleFirstClassGrade input with
      | none, none => true
      | some value, some mirror => rustGradeToMirror value == mirror
      | _, _ => false }

def checkTrendObservation (path : String) (level : Nat) (side : String)
    (previous parent : CenterFrame) (segment : SegmentRow)
    (gates : FirstStructuralGates) (rust : RustCandidateExtraction) : RuntimeCheck :=
  { path := path, port := "observations", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 18,
    passed := rustCandidateToMirror rust == makeTrendObservation level previous parent segment gates }

def candidateMemberParity
    (rust : RustCandidateExtraction) (lean : List CandidateObservationMirror) : Bool :=
  lean.any fun candidate => rustCandidateToMirror rust == candidate

def checkReduction (path : String) (level : Nat) (side : String)
    (legs : List CandidateObservationMirror) (rust : List RustCandidateExtraction) : RuntimeCheck :=
  let lean := reduceStructuralLegs legs
  let sameLength := rust.length == lean.length
  let allPresent := rust.all fun candidate => candidateMemberParity candidate lean
  { path := path, port := "reduction", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := rust.length * 18 + 1,
    passed := sameLength && allPresent }

def checkPan (path : String) (level : Nat) (side : String)
    (input : PanProjectionInput) (rust : RustPanDivExtraction) : RuntimeCheck :=
  { path := path, port := "pan", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 9,
    passed := (assemblePanDivCert input).map
      (fun mirror => rust.sourceIndex == mirror.sourceIndex && rust.side.toLean == mirror.side &&
        rust.centerStart == mirror.centerStart && rust.centerZd == mirror.centerZd &&
        rust.centerZg == mirror.centerZg && rust.segALeft == mirror.segA.left &&
        rust.segARight == mirror.segA.right && rust.segCLeft == mirror.segC.left &&
        rust.segCRight == mirror.segC.right) |>.getD false }

def checkPanObservation (path : String) (level : Nat) (side : String)
    (cert : PanDivCertMirror) (rust : RustCandidateExtraction) : RuntimeCheck :=
  { path := path, port := "observations", level := level, side := side,
    terminal := "NoCpApplicable", comparedFields := 18,
    passed := rustCandidateToMirror rust == makePanObservation level cert }

def checkCpAttachment (path : String) (level : Nat) (terminal : String)
    (rust : RustCpExtraction) (lean : CpOwnershipMirror) : RuntimeCheck :=
  { path := path, port := "cp", level := level, side := "NotYetKnown",
    terminal := terminal, comparedFields := 6,
    passed := rustCpToMirror rust lean.certificate == lean }

def mismatchFields (checks : List RuntimeCheck) : Nat :=
  checks.foldl (fun total check => if check.passed then total else total + check.comparedFields) 0

def comparedFields (checks : List RuntimeCheck) : Nat :=
  checks.foldl (fun total check => total + check.comparedFields) 0

def portCount (checks : List RuntimeCheck) (port : String) : Nat :=
  (checks.filter fun check => check.port == port).length

def emitReport (windowId : String) (checks : List RuntimeCheck) : IO Unit := do
  let failures := checks.filter fun check => !check.passed
  IO.println s!"S2_LEAN_SUMMARY\t{windowId}\t{checks.length}\t{comparedFields checks}\t{mismatchFields checks}"
  for port in ["prelude", "points", "pan", "grades", "observations", "reduction", "cp"] do
    IO.println s!"S2_LEAN_PORT\t{windowId}\t{port}\t{portCount checks port}"
  for failure in failures.take 20 do
    IO.eprintln s!"S2_LEAN_MISMATCH\t{failure.path}\t{failure.port}\tL{failure.level}\t{failure.side}\t{failure.terminal}"
  if !failures.isEmpty then
    throw <| IO.userError s!"{windowId}: {failures.length} Lean mirror records mismatched"

theorem reduction_runtime_check_counts_length
    (path side : String) (level : Nat) (legs : List CandidateObservationMirror)
    (rust : List RustCandidateExtraction) :
    (checkReduction path level side legs rust).comparedFields = rust.length * 18 + 1 := by
  rfl

theorem cp_runtime_check_is_field_parity
    (path terminal : String) (level : Nat) (rust : RustCpExtraction)
    (lean : CpOwnershipMirror) :
    (checkCpAttachment path level terminal rust lean).passed =
      (rustCpToMirror rust lean.certificate == lean) := by
  rfl

theorem force_l_missing_never_diverges (c : Option Nat) :
    forceLDivergedBits none c = false := by
  cases c <;> rfl

theorem grade_fixed_pair_missing_leave (center : CenterFrame) (trend : Direction)
    (retest : Option SegmentRow) :
    gradeFixedFirstPair center trend none retest = Grade.missing GradeReason.missingLeave := by
  cases retest <;> rfl

end NewChanlun.Origin.UnifiedScanMirrorRunner
