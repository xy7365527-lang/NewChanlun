import Origin.ScanAssemblyBridge

namespace NewChanlun.Origin.ScanAssemblyNegative
open ScanAssemblyMirror
open ScanAssemblyBridge

/-- #1087 RED-1：raw sink 无 ID；Lean 必须仅由 CandidateKey 独立重算两个稳定 ID。 -/
def rawKey : CandidateKey :=
  { ruleVersion := 1, level := 0, kind := CandidateKind.trend, side := Side.long
    previousCenterStart := some 1
    parent := { centerStart := 2, zd := 3, zg := 4 }
    segA := (5, 6), cStart := 7 }

def rawLeg : CandidateLeg :=
  { key := rawKey, kind := CandidateKind.trend, centerIds := some (1, 2)
    structuralPredicates := { direction := true, comparable := true, extreme := true }
    extremeProof := (5, 6), thirdClassProof := none, interval := (7, 9)
    state := ObservedState.provisional, firstProvableAt := some 9, confirmedAt := none }

def recomputed : MergedScanOutput := recomputeMergedOutput 0
  [{ bspPoints := [], panDivCerts := [], firstClassGrades := [], candidateLegs := [rawLeg] }]

example : recomputed.observations.map (fun observation =>
    (observation.candidateGroupId, observation.pairId)) =
    [(candidateStableId rawKey fnvOffsetBasis, candidateStableId rawKey pairIdSeed)] := by
  native_decide


def rawCenter : CenterFrame :=
  { zd := 100, zg := 200, dd := 90, gg := 210, startIndex := 3, endIndex := 8 }

def rawRows : List SegmentRow :=
  [{ direction := Direction.down, startIndex := 9, endIndex := 11,
     startPrice := 150, endPrice := 80 }]

def acceptedFacts : CandDeltaFacts :=
  { kind := CandDeltaKind.trend, structuralCandidate := true, direction := true
    comparable := true, extreme := true, buy1 := true, sell1 := false, panDiverges := false }

def eventInput (facts : CandDeltaFacts) : EventAssemblyInput :=
  { level := 0, side := Side.long, divergenceConfirmSrc := 11
    aInterval := { left := 3, right := 5 }, center := rawCenter
    departureDir := Direction.down, untilStart := 9, triggerEnd := 11, rows := rawRows
    predicate := facts, cIntervalFull := none, bParent := none, cStructure := none
    thirdClassInC := none, fullTrendCQualified := none, fullTrendEvidence := none
    cpOwnership := none, panDivDiag := true }

/-- #1087 RED-2a：拒绝样本必须由 Lean 原始分量判空，不能由 Rust `accepted` 强置。 -/
example : assembleCandDelta (eventInput { acceptedFacts with extreme := false }) = none := by
  native_decide

/-- #1087 RED-2b：accepted 样本的 19 字段由 Lean 装配；pan 诊断与 Cand 真值不能互相反推。 -/
example : assembleCandDelta (eventInput acceptedFacts) = some
    { level := 0, side := Side.long, divergenceConfirmSrc := 11, confirmSrc := 11
      interval := { left := 9, right := 11 }, aInterval := { left := 3, right := 5 }
      cEpisodeStart := 9, cEpisodeInterval := { left := 9, right := 11 }
      cIntervalFull := none, bParent := none, cStructure := none, thirdClassInC := none
      cpCertificateConfirmSrc := none, fullTrendCQualified := none, fullTrendEvidence := none
      cpOwnership := none, enterSrc := 9, candDelta := true, panDivDiag := true } := by
  native_decide


def cpId (level ordinal : Nat) : ElementIdentity := { level := level, ordinal := ordinal }

def cpBeforeFull : CpObject :=
  { bCenterIndex := 0, bCenterId := cpId 1 2, bCenter := rawCenter
    departureMoveId := some (cpId 1 3), departureInterval := some (9, 11)
    lifecycle := CpLifecycle.pending, cpCertificateConfirmSrc := none
    cStructure := none, thirdClassInC := none, fullTrendEvidence := none
    fullTrendCQualified := none }

def cpLeave : SegmentRow :=
  { direction := Direction.down, startIndex := 9, endIndex := 11,
    startPrice := 150, endPrice := 80 }

def cpRetest : SegmentRow :=
  { direction := Direction.up, startIndex := 11, endIndex := 13,
    startPrice := 80, endPrice := 90 }

def cpClosureRawContext : EventRawContext :=
  { centers := [rawCenter]
    rows := []
    anchors := []
    cpScan := []
    unitMoves := [] }

def cpRawValid : CpClosureEvidence :=
  { context := cpClosureRawContext, visibleUnitMoveCount := 0
    cpDepartureMoveId := cpId 1 3, cpStart := 9
    leave := cpLeave, retest := cpRetest
    leaveAnchor := some Direction.down, leaveMoveId := cpId 1 3, retestMoveId := cpId 1 4 }

/-- #1087 RED-3a：Lean 从原始 leave/retest 几何判定 Pending→Closed，并附着完整结构。 -/
example : (closeCpFromRaw cpBeforeFull cpRawValid).lifecycle = CpLifecycle.closed ∧
    (closeCpFromRaw cpBeforeFull cpRawValid).cpCertificateConfirmSrc = some 13 ∧
    (closeCpFromRaw cpBeforeFull cpRawValid).cStructure.isSome ∧
    (closeCpFromRaw cpBeforeFull cpRawValid).thirdClassInC.isSome := by
  native_decide

/-- #1087 RED-3b：回试重入中枢时必须保持 Pending；不能靠 witness=true 强闭合。 -/
example : closeCpFromRaw cpBeforeFull
    { cpRawValid with retest := { cpRawValid.retest with endPrice := 150 } } = cpBeforeFull := by
  native_decide

/-- #1087 review3 HIGH-2：raw 不得把任意 full-trend 成品证据回灌到 after。 -/
def review3ImpossibleEvidence : FullTrendQualificationEvidence :=
  { trendContext := none
    newExtremeInDirection := none
    internalSublevelCenters := none
    completedTrendDecomposition := none
    decompositionReviewMoveId := none
    decompositionReviewSrc := some 999999 }

def review3GarbageEvidenceAfter : CpObject :=
  { closeCpFromRaw cpBeforeFull cpRawValid with
      fullTrendEvidence := some review3ImpossibleEvidence }

example : closeCpFromRaw cpBeforeFull cpRawValid ≠ review3GarbageEvidenceAfter := by
  native_decide


/-- #1087 review2 RED-3c：raw retest ID 的 level/ordinal 早于 c_p 离开 move 时保持 Pending。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with retestMoveId := cpId 0 0 }).lifecycle = CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3d：leave/retest move level 必须与 c_p departure move level 一致。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with leaveMoveId := cpId 2 3 }).lifecycle = CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3e：retest move ordinal 不得早于 leave move ordinal。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with leaveMoveId := cpId 1 5, retestMoveId := cpId 1 4 }).lifecycle =
      CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3f：leave move ordinal 不得早于 c_p departure move ordinal。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with leaveMoveId := cpId 1 2 }).lifecycle = CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3g：leave.start 早于 c_p 起点时保持 Pending。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with leave := { cpRawValid.leave with startIndex := 8 } }).lifecycle =
      CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3h：raw departure move ID 必须与对象上的 ID 一致。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with cpDepartureMoveId := cpId 1 4 }).lifecycle = CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3i：raw cpStart 必须与对象 departure interval 左端一致。 -/
example : (closeCpFromRaw cpBeforeFull
    { cpRawValid with cpStart := 8 }).lifecycle = CpLifecycle.pending := by
  native_decide


/-- #1087 review2 RED-3j：对象缺 departure move ID 时不得闭合。 -/
example : (closeCpFromRaw { cpBeforeFull with departureMoveId := none }
    cpRawValid).lifecycle = CpLifecycle.pending := by
  native_decide

/-- #1087 review2 RED-3k：对象缺 departure interval 时不得闭合。 -/
example : (closeCpFromRaw { cpBeforeFull with departureInterval := none }
    cpRawValid).lifecycle = CpLifecycle.pending := by
  native_decide


def zeroBits : BspBits :=
  { buy1 := false, buy2 := false, buy3 := false, sell1 := false, sell2 := false,
    sell3 := false, thirdClassEntry := none }

def staleCache : FrontierCache :=
  { cachedCount := 2
    sinks := { points := [{ sourceIndex := 7, bits := zeroBits, pivotLow := 0, pivotHigh := 0,
                            center := none, structBreakDir := none, force := none,
                            retraceBreaksType1 := none }]
               panDivs := [], grades := [], candidateLegs := [] } }

/-- #1087 RED-4：冻结边界回缩必须锁步清掉四个 cache sink，旧 frontier 不得假绿。 -/
example : (resumeMergedOutput productionPostScanOperators 0 1 staleCache [] []).1.cachedCount = 1 ∧
    (resumeMergedOutput productionPostScanOperators 0 1 staleCache [] []).2.points = [] := by
  native_decide


def rustRow : RustSegmentExtraction :=
  { direction := RustDirectionTag.down, startIndex := 9, endIndex := 11,
    startPrice := 150, endPrice := 80 }

def rustPredicate : RustPredicateExtraction :=
  { kind := RustCandDeltaKindTag.trend, structuralCandidate := true,
    direction := true, comparable := true, extreme := true, buy1 := true,
    sell1 := false, panDiverges := false }

def rustEventContext : RustEventRawContextExtraction :=
  { centers :=
      [{ zd := 100, zg := 200, dd := 90, gg := 210, startIndex := 3, endIndex := 8 }]
    rows := [rustRow]
    anchors := [some RustDirectionTag.down]
    cpScan := []
    unitMoves := [] }

def rustEventInput : RustAssemblyInputExtraction :=
  { context := rustEventContext, centerIndex := 0, segmentIndex := 0
    level := 0, side := RustSideTag.long, divergenceConfirmSrc := 11
    aIntervalLeft := 3, aIntervalRight := 5
    predicate := rustPredicate
    panDivDiag := true }

def wrongPanField : RustEventExtraction :=
  { level := 0, side := RustSideTag.long, divergenceConfirmSrc := 11, confirmSrc := 11
    intervalLeft := 9, intervalRight := 11, aIntervalLeft := 3, aIntervalRight := 5
    cEpisodeStart := 9, cEpisodeLeft := 9, cEpisodeRight := 11
    cIntervalFull := none, bParent := none, cStructure := none, thirdClassInC := none
    cpCertificateConfirmSrc := none, fullTrendCQualified := none, fullTrendEvidence := none
    cpOwnership := none, enterSrc := 9, candDelta := true, panDivDiag := false }

/-- #1087 RED-2c：任一 19 字段漂移都必须红；这里翻转 pan_div_diag。 -/
example : ¬ EventCheck rustEventInput (some wrongPanField) := by
  native_decide

def correctRustEvent : RustEventExtraction :=
  { wrongPanField with panDivDiag := true }

def extraRustEvent : RustEventExtraction :=
  { correctRustEvent with confirmSrc := 12 }

def rustRejectedInput : RustAssemblyInputExtraction :=
  { rustEventInput with predicate := { rustPredicate with extreme := false } }

/-- #1087 review2 RED-2d：列表级门接受 accepted raw case 与唯一生产 event 的一一对应。 -/
example : EventBijectionCheck [rustEventInput] [correctRustEvent] := by
  native_decide

/-- #1087 review2 RED-2e：同一生产 event 重复出现必须失败，不能被两个 `find` 共用。 -/
example : ¬ EventBijectionCheck [rustEventInput] [correctRustEvent, correctRustEvent] := by
  native_decide

/-- #1087 review2 RED-2f：没有 raw case 消费的额外生产 event 必须失败。 -/
example : ¬ EventBijectionCheck [rustEventInput] [correctRustEvent, extraRustEvent] := by
  native_decide

/-- #1087 review2 RED-2g：accepted raw case 漏产 event 必须失败。 -/
example : ¬ EventBijectionCheck [rustEventInput] [] := by
  native_decide

/-- #1087 review2 RED-2h：rejected raw case 必须恰好零 event。 -/
example : ¬ EventBijectionCheck [rustRejectedInput] [correctRustEvent] := by
  native_decide

/-- #1087 review2 RED-2i：即使另一 accepted case 消费了 event，rejected case 也不能共用同一槽。 -/
example : ¬ EventBijectionCheck [rustEventInput, rustRejectedInput] [correctRustEvent] := by
  native_decide

/-- #1087 review3 HIGH-1：production event 伪造的 B_p 必须被 Lean 从 raw 低层事实独立重算拒绝。 -/
def review3ImpossibleParent : RustParentCenterIdentityExtraction :=
  { centerIndex := 999
    centerId := { level := 9, ordinal := 999 }
    sourceInterval := { left := 888, right := 777 }
    zd := -42
    zg := -41 }

def review3ValidParent : RustCpScanBaseExtraction :=
  { bCenterIndex := 0
    bCenterId := { level := 0, ordinal := 42 }
    bCenter := { zd := 100, zg := 200, dd := 90, gg := 210, startIndex := 3, endIndex := 8 }
    departureMoveId := none
    departureInterval := none }

def review3ForgedParentInput : RustAssemblyInputExtraction :=
  { rustEventInput with
      context := { rustEventContext with cpScan := [review3ValidParent] } }

def review3ForgedParentEvent : RustEventExtraction :=
  { correctRustEvent with bParent := some review3ImpossibleParent }

example : ¬ EventBijectionCheck [review3ForgedParentInput] [review3ForgedParentEvent] := by
  native_decide

/-- 单层内部双射允许合法空层；非空覆盖只在窗口/阶段聚合门强制。 -/
example : EventBijectionCoreCheck [] [] := by
  native_decide

/-- #1087 review3 HIGH-3：真实验收域不得让空 raw/空 event 的真空双射假绿。 -/
example : ¬ EventBijectionCheck [] [] := by
  native_decide

end NewChanlun.Origin.ScanAssemblyNegative
