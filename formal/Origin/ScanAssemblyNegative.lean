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

def cpRawValid : CpClosureEvidence :=
  { cpDepartureMoveId := cpId 1 3, cpStart := 9
    leave := cpLeave, retest := cpRetest
    leaveAnchor := some Direction.down, leaveMoveId := cpId 1 3, retestMoveId := cpId 1 4
    fullTrendEvidence := none, fullTrendCQualified := none }

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

def rustEventInput : RustAssemblyInputExtraction :=
  { level := 0, side := RustSideTag.long, divergenceConfirmSrc := 11
    aIntervalLeft := 3, aIntervalRight := 5
    center := { zd := 100, zg := 200, dd := 90, gg := 210, startIndex := 3, endIndex := 8 }
    departureDir := RustDirectionTag.down, untilStart := 9, triggerEnd := 11
    rows := [rustRow]
    predicate := rustPredicate
    cIntervalFull := none, bParent := none, cStructure := none, thirdClassInC := none
    fullTrendCQualified := none, fullTrendEvidence := none, cpOwnership := none
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

end NewChanlun.Origin.ScanAssemblyNegative
