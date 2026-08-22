/-
Origin/UnifiedScanMirrorBridge.lean

#1080 第一阶段：3a 统一扫描装配语义的 Lean 镜像与 Rust 提取对拍协议。

本文件只镜像当前生产装配语义，不参与生产判定，也不增删缠论判据。镜面分三段：
1. resume prelude：有序/平行数组守卫、最近中枢、首匹配语义、A 段缓存、lambda_C episode；
2. 四产口：BspPoint（3a 内只含一/三类）、PanDivCert、FirstClassGradeRecord、
   CandidateObservation 及同 episode 腿归约；
3. LevelState.cp_ownership：初始化、dirty 失效、稳定重继承、Pending/Closed 推进。

桥接形态复用 Origin 既有先例：Rust wire 与 Lean 镜像是独立类型，BridgesTo/Parity 逐字段校验。
本阶段单产口 Check 会现场运行对应 Lean 函数；完整 raw-input→merged-driver 防绕过入口尚未接入。

认识论等级 L0：编译只证明本文件的定义内性质。真实 Rust 窗口提取执行器尚未接入时，不得把本文件
宣称为跨语言窗口已签收。
-/

namespace NewChanlun.Origin.UnifiedScanMirrorBridge

/-! ## 1. 公共值域与 resume prelude -/

inductive Direction where
  | up
  | down
deriving DecidableEq, Repr

inductive Side where
  | long
  | short
deriving DecidableEq, Repr

structure Interval where
  left : Nat
  right : Nat
deriving DecidableEq, Repr

structure SegmentRow where
  direction : Direction
  startIndex : Nat
  endIndex : Nat
  endPrice : Int
  anchor : Option Direction
  departureEnd : Option Nat
deriving DecidableEq, Repr

structure CenterFrame where
  startIndex : Nat
  endIndex : Nat
  zd : Int
  zg : Int
deriving DecidableEq, Repr

def segmentStartsOrdered : List SegmentRow -> Bool
  | [] => true
  | [_] => true
  | first :: second :: rest =>
      decide (first.startIndex <= second.startIndex) &&
        segmentStartsOrdered (second :: rest)

def centerEndsStrict : List CenterFrame -> Bool
  | [] => true
  | [_] => true
  | first :: second :: rest =>
      decide (first.endIndex < second.endIndex) && centerEndsStrict (second :: rest)

def anchorsAreStructural (rows : List SegmentRow) : Bool :=
  rows.all fun row => row.anchor == some row.direction

/-- 生产 resume 入口的形状守卫；full 神谕的稳定排序不在本函数内冒充热路步骤。 -/
def orderedResumeGuard (rows : List SegmentRow) (centers : List CenterFrame) : Bool :=
  segmentStartsOrdered rows && centerEndsStrict centers && anchorsAreStructural rows

theorem ordered_resume_guard_rejects_unsorted_segments :
    orderedResumeGuard
      [{ direction := Direction.up, startIndex := 9, endIndex := 10, endPrice := 2,
         anchor := some Direction.up, departureEnd := none },
       { direction := Direction.down, startIndex := 3, endIndex := 4, endPrice := 1,
         anchor := some Direction.down, departureEnd := none }]
      [] = false := by
  decide

def nearestConfirmedCenterIdxAux
    (centers : List CenterFrame) (segmentStart : Nat) (next : Nat)
    (last : Option Nat) : Option Nat :=
  match centers with
  | [] => last
  | center :: rest =>
      if center.endIndex <= segmentStart then
        nearestConfirmedCenterIdxAux rest segmentStart (next + 1) (some next)
      else
        last

/-- `partition_point(end_index <= segment.start_index) - 1` 的关系等价函数。 -/
def nearestConfirmedCenterIdx (centers : List CenterFrame) (segmentStart : Nat) : Option Nat :=
  nearestConfirmedCenterIdxAux centers segmentStart 0 none

def NearestConfirmedBy (centers : List CenterFrame) (segmentStart index : Nat) : Prop :=
  nearestConfirmedCenterIdx centers segmentStart = some index

theorem nearest_confirmed_center_unique
    (centers : List CenterFrame) (segmentStart first second : Nat)
    (hFirst : NearestConfirmedBy centers segmentStart first)
    (hSecond : NearestConfirmedBy centers segmentStart second) :
    first = second := by
  unfold NearestConfirmedBy at hFirst hSecond
  exact Option.some.inj (hFirst.symm.trans hSecond)

def nearestCenterWitnesses : List CenterFrame :=
  [{ startIndex := 0, endIndex := 2, zd := 10, zg := 20 },
   { startIndex := 3, endIndex := 5, zd := 11, zg := 21 },
   { startIndex := 6, endIndex := 9, zd := 12, zg := 22 }]

theorem nearest_confirmed_center_selects_latest_prefix :
    nearestConfirmedCenterIdx nearestCenterWitnesses 6 = some 1 := by
  decide

def sameCenterKey (first second : CenterFrame) : Bool :=
  first.endIndex == second.endIndex && first.zd == second.zd && first.zg == second.zg

def firstMatchIdxAux (centers : List CenterFrame) (target : CenterFrame) (index : Nat) : Option Nat :=
  match centers with
  | [] => none
  | center :: rest =>
      if sameCenterKey center target then some index
      else firstMatchIdxAux rest target (index + 1)

/-- full 神谕兼容重复三元组时的“首匹配”语义。生产 resume 在严格 end 序下直接用 c_idx。 -/
def firstMatchIdx (centers : List CenterFrame) (target : CenterFrame) : Option Nat :=
  firstMatchIdxAux centers target 0

def FirstMatchedBy (centers : List CenterFrame) (target : CenterFrame) (index : Nat) : Prop :=
  firstMatchIdx centers target = some index

theorem first_match_unique
    (centers : List CenterFrame) (target : CenterFrame) (first second : Nat)
    (hFirst : FirstMatchedBy centers target first)
    (hSecond : FirstMatchedBy centers target second) :
    first = second := by
  unfold FirstMatchedBy at hFirst hSecond
  exact Option.some.inj (hFirst.symm.trans hSecond)

def duplicateCenterKeyWitnesses : List CenterFrame :=
  [{ startIndex := 0, endIndex := 5, zd := 10, zg := 20 },
   { startIndex := 1, endIndex := 5, zd := 10, zg := 20 }]

theorem first_match_keeps_first_duplicate_key :
    firstMatchIdx duplicateCenterKeyWitnesses
      { startIndex := 99, endIndex := 5, zd := 10, zg := 20 } = some 0 := by
  decide

inductive MoveKind where
  | trend
  | consolidation
deriving DecidableEq, Repr

structure MoveBlock where
  startCenter : Nat
  endCenter : Nat
  kind : MoveKind
  direction : Option Direction
  levelLift : Nat
deriving DecidableEq, Repr

def incomingBlockAt (blocks : List MoveBlock) (centerIndex : Nat) : Option MoveBlock :=
  match blocks with
  | [] => none
  | block :: rest =>
      if block.startCenter < centerIndex && centerIndex <= block.endCenter then some block
      else incomingBlockAt rest centerIndex

/-- `center_own_dir_at`：C0 无入边；其余只从入边所属 Trend 块取得方向。 -/
def resolveTrendGate (blocks : List MoveBlock) (centerIndex : Nat) : Option Direction :=
  if centerIndex = 0 then none
  else
    match incomingBlockAt blocks centerIndex with
    | some block => if block.kind = MoveKind.trend then block.direction else none
    | none => none

/-- `center_block_kind_at == Consolidation && center_block_lift_at == 0`。 -/
def resolveConsolidationLiftZero (blocks : List MoveBlock) (centerIndex : Nat) : Bool :=
  let owned :=
    if centerIndex = 0 then blocks.head?
    else incomingBlockAt blocks centerIndex
  match owned with
  | some block => block.kind == MoveKind.consolidation && block.levelLift == 0
  | none => false

def trendBlockWitness : MoveBlock :=
  { startCenter := 0, endCenter := 2, kind := MoveKind.trend,
    direction := some Direction.down, levelLift := 0 }

theorem trend_gate_reads_incoming_relation :
    resolveTrendGate [trendBlockWitness] 2 = some Direction.down := by
  decide

theorem trend_gate_rejects_c0 :
    resolveTrendGate [trendBlockWitness] 0 = none := by
  rfl

def consolidationBlockWitness : MoveBlock :=
  { startCenter := 0, endCenter := 0, kind := MoveKind.consolidation,
    direction := none, levelLift := 0 }

theorem consolidation_gate_accepts_c0_lift_zero :
    resolveConsolidationLiftZero [consolidationBlockWitness] 0 = true := by
  decide

structure ASegment where
  span : Interval
  envelopeLow : Int
  envelopeHigh : Int
deriving DecidableEq, Repr

structure ACacheEntry where
  centerIndex : Nat
  value : Option ASegment
deriving DecidableEq, Repr

def lookupACache (entries : List ACacheEntry) (centerIndex : Nat) : Option (Option ASegment) :=
  match entries with
  | [] => none
  | entry :: rest =>
      if entry.centerIndex = centerIndex then some entry.value
      else lookupACache rest centerIndex

/-- 命中即复用；miss 才消费同 `(prev_center,last_center,trend_dir)` 的预算结果。 -/
def resolveASegment
    (entries : List ACacheEntry) (centerIndex : Nat) (recomputed : Option ASegment) :
    Option ASegment :=
  match lookupACache entries centerIndex with
  | some cached => cached
  | none => recomputed

theorem a_segment_cache_hit_is_transparent
    (entry : ACacheEntry) (rest : List ACacheEntry) (fresh : Option ASegment) :
    resolveASegment (entry :: rest) entry.centerIndex fresh = entry.value := by
  unfold resolveASegment lookupACache
  simp

def reenters (center : CenterFrame) (departureDir : Direction) (row : SegmentRow) : Prop :=
  row.direction != departureDir &&
    match departureDir with
    | Direction.down => decide (center.zd <= row.endPrice)
    | Direction.up => decide (row.endPrice <= center.zg)

instance (center : CenterFrame) (departureDir : Direction) (row : SegmentRow) :
    Decidable (reenters center departureDir row) := by
  unfold reenters
  infer_instance

def episodeWindow (rows : List SegmentRow) (center : CenterFrame) (untilStart : Nat) :
    List SegmentRow :=
  rows.filter fun row => decide (center.endIndex <= row.startIndex && row.startIndex <= untilStart)

def lastReentryBoundary (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) : Nat :=
  rows.foldl
    (fun boundary row => if reenters center departureDir row then row.endIndex else boundary)
    0

def firstAnchoredStartAfter (rows : List SegmentRow) (departureDir : Direction)
    (boundary : Nat) : Option Nat :=
  match rows with
  | [] => none
  | row :: rest =>
      if row.anchor = some departureDir && boundary <= row.startIndex then some row.startIndex
      else firstAnchoredStartAfter rest departureDir boundary

/-- 当前 episode 最后回中枢边界之后，首个获同向结构锚的段起点。 -/
def lambdaCEpisode (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart : Nat) : Option Nat :=
  let window := episodeWindow rows center untilStart
  firstAnchoredStartAfter window departureDir (lastReentryBoundary window center departureDir)

def EpisodeDelimitedBy (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart lambdaC : Nat) : Prop :=
  lambdaCEpisode rows center departureDir untilStart = some lambdaC

theorem lambda_c_episode_unique
    (rows : List SegmentRow) (center : CenterFrame) (departureDir : Direction)
    (untilStart first second : Nat)
    (hFirst : EpisodeDelimitedBy rows center departureDir untilStart first)
    (hSecond : EpisodeDelimitedBy rows center departureDir untilStart second) :
    first = second := by
  unfold EpisodeDelimitedBy at hFirst hSecond
  exact Option.some.inj (hFirst.symm.trans hSecond)

def lambdaCenterWitness : CenterFrame :=
  { startIndex := 0, endIndex := 5, zd := 10, zg := 20 }

def lambdaRowsWitness : List SegmentRow :=
  [{ direction := Direction.down, startIndex := 6, endIndex := 7, endPrice := 5,
     anchor := some Direction.down, departureEnd := none },
   { direction := Direction.up, startIndex := 8, endIndex := 9, endPrice := 10,
     anchor := some Direction.up, departureEnd := none },
   { direction := Direction.down, startIndex := 10, endIndex := 11, endPrice := 4,
     anchor := some Direction.down, departureEnd := none }]

theorem lambda_c_reentry_restarts_episode :
    lambdaCEpisode lambdaRowsWitness lambdaCenterWitness Direction.down 10 = some 10 := by
  decide

/-! ## 2. 唯一结构门与四产口 -/

structure GateInput where
  center : CenterFrame
  trendDirection : Direction
  segment : SegmentRow
  aSegment : Option ASegment
  lambdaC : Option Nat
  aMapped : Bool
  cMapped : Bool
deriving DecidableEq, Repr

structure FirstStructuralGates where
  side : Side
  segA : Interval
  lambdaC : Nat
  aMapped : Bool
  cMapped : Bool
  extreme : Bool
deriving DecidableEq, Repr

def FirstStructuralGates.comparable (gates : FirstStructuralGates) : Bool :=
  gates.aMapped && gates.cMapped

def brokeCenterInTrend (input : GateInput) : Option Side :=
  match input.segment.anchor, input.trendDirection with
  | some Direction.down, Direction.down =>
      if input.segment.endPrice < input.center.zd then some Side.long else none
  | some Direction.up, Direction.up =>
      if input.center.zg < input.segment.endPrice then some Side.short else none
  | _, _ => none

/-- Rust `first_structural_gates` 的短路次序：破中枢 → A → lambda_C；映射失败留在 gates。 -/
def firstStructuralGates (input : GateInput) : Option FirstStructuralGates :=
  match brokeCenterInTrend input, input.aSegment, input.lambdaC with
  | some side, some a, some lambdaC =>
      let extreme :=
        match input.trendDirection with
        | Direction.down => decide (input.segment.endPrice < a.envelopeLow)
        | Direction.up => decide (a.envelopeHigh < input.segment.endPrice)
      some
        { side := side
          segA := a.span
          lambdaC := lambdaC
          aMapped := input.aMapped
          cMapped := input.cMapped
          extreme := extreme }
  | _, _, _ => none

theorem structural_gate_rejects_missing_a (input : GateInput) (h : input.aSegment = none) :
    firstStructuralGates input = none := by
  unfold firstStructuralGates
  rw [h]
  cases brokeCenterInTrend input <;> rfl

inductive GradeReason where
  | missingLeave
  | missingRetest
  | sameDirection
  | leaveNotOutside
  | retestReentered
deriving DecidableEq, Repr

inductive Grade where
  | present (leaveInterval : Interval) (retestInterval : Interval)
  | missing (reason : GradeReason)
deriving DecidableEq, Repr

structure FirstProjectionInput where
  level : Nat
  center : CenterFrame
  segment : SegmentRow
  gates : FirstStructuralGates
  diverged : Bool
  t3Present : Bool
  grade : Grade
deriving DecidableEq, Repr

/-- 3a 内的一类/结构候选与第三类共用的最小逐产口镜面；二类由 pipeline 07b 另装。 -/
structure BspPointMirror where
  sourceIndex : Nat
  side : Side
  type1Bit : Bool
  type3Bit : Bool
  structureBreak : Bool
  ownerCenterStart : Nat
deriving DecidableEq, Repr

def judgeFirstFromGates (input : FirstProjectionInput) : Option BspPointMirror :=
  if input.gates.extreme && input.gates.comparable then
    some
      { sourceIndex := input.segment.departureEnd.getD input.segment.endIndex
        side := input.gates.side
        type1Bit := input.diverged && input.t3Present
        type3Bit := false
        structureBreak := true
        ownerCenterStart := input.center.startIndex }
  else
    none

theorem first_projection_keeps_structural_candidate
    (input : FirstProjectionInput) (point : BspPointMirror)
    (h : judgeFirstFromGates input = some point) :
    point.structureBreak = true := by
  unfold judgeFirstFromGates at h
  split at h
  · cases h
    rfl
  · contradiction

structure ThirdProjectionInput where
  center : CenterFrame
  leave : SegmentRow
  retest : SegmentRow
  firstRetrace : Bool
deriving DecidableEq, Repr

def judgeThirdPoint (input : ThirdProjectionInput) : Option BspPointMirror :=
  if input.firstRetrace then
    match input.leave.anchor, input.retest.direction with
    | some Direction.up, Direction.down =>
        if input.center.zg < input.leave.endPrice && input.center.zg < input.retest.endPrice then
          some
            { sourceIndex := input.retest.endIndex, side := Side.long, type1Bit := false,
              type3Bit := true, structureBreak := false,
              ownerCenterStart := input.center.startIndex }
        else none
    | some Direction.down, Direction.up =>
        if input.leave.endPrice < input.center.zd && input.retest.endPrice < input.center.zd then
          some
            { sourceIndex := input.retest.endIndex, side := Side.short, type1Bit := false,
              type3Bit := true, structureBreak := false,
              ownerCenterStart := input.center.startIndex }
        else none
    | _, _ => none
  else
    none

def thirdBuyWitness : ThirdProjectionInput :=
  { center := { startIndex := 1, endIndex := 5, zd := 10, zg := 20 }
    leave :=
      { direction := Direction.up, startIndex := 6, endIndex := 7, endPrice := 30,
        anchor := some Direction.up, departureEnd := none }
    retest :=
      { direction := Direction.down, startIndex := 8, endIndex := 9, endPrice := 25,
        anchor := some Direction.down, departureEnd := none }
    firstRetrace := true }

theorem third_projection_sets_only_third_bit :
    judgeThirdPoint thirdBuyWitness =
      some
        { sourceIndex := 9, side := Side.long, type1Bit := false, type3Bit := true,
          structureBreak := false, ownerCenterStart := 1 } := by
  decide

structure PanStructure where
  sourceIndex : Nat
  side : Side
  center : CenterFrame
  segA : Interval
  segC : Interval
deriving DecidableEq, Repr

structure PanProjectionInput where
  consolidation : Bool
  liftZero : Bool
  comparable : Bool
  diverged : Bool
  panStructure : Option PanStructure
deriving DecidableEq, Repr

structure PanDivCertMirror where
  sourceIndex : Nat
  side : Side
  centerStart : Nat
  centerZd : Int
  centerZg : Int
  segA : Interval
  segC : Interval
deriving DecidableEq, Repr

def assemblePanDivCert (input : PanProjectionInput) : Option PanDivCertMirror :=
  if input.consolidation && input.liftZero && input.comparable && input.diverged then
    input.panStructure.map fun pan =>
      { sourceIndex := pan.sourceIndex
        side := pan.side
        centerStart := pan.center.startIndex
        centerZd := pan.center.zd
        centerZg := pan.center.zg
        segA := pan.segA
        segC := pan.segC }
  else
    none

theorem pan_projection_preserves_both_intervals
    (input : PanProjectionInput) (cert : PanDivCertMirror)
    (h : assemblePanDivCert input = some cert) :
    ∃ pan, input.panStructure = some pan ∧ cert.segA = pan.segA ∧
      cert.segC = pan.segC := by
  unfold assemblePanDivCert at h
  split at h
  · cases hs : input.panStructure with
    | none => simp [hs] at h
    | some pan =>
        simp [hs] at h
        subst cert
        exact ⟨pan, rfl, rfl, rfl⟩
  · simp at h

structure FirstClassGradeRecordMirror where
  level : Nat
  sourceIndex : Nat
  side : Side
  centerStart : Nat
  centerEnd : Nat
  centerZd : Int
  centerZg : Int
  grade : Grade
deriving DecidableEq, Repr

/-- 分级记录在 T3-in-c 二次门之前按 `diverged` 捕获 Present/Missing 两域。 -/
def assembleFirstClassGrade (input : FirstProjectionInput) : Option FirstClassGradeRecordMirror :=
  if input.diverged then
    some
      { level := input.level
        sourceIndex := input.segment.departureEnd.getD input.segment.endIndex
        side := input.gates.side
        centerStart := input.center.startIndex
        centerEnd := input.center.endIndex
        centerZd := input.center.zd
        centerZg := input.center.zg
        grade := input.grade }
  else
    none

theorem grade_projection_does_not_require_t3_present
    (input : FirstProjectionInput) (record : FirstClassGradeRecordMirror)
    (hDiverged : input.diverged = true)
    (h : assembleFirstClassGrade input = some record) :
    record.grade = input.grade := by
  unfold assembleFirstClassGrade at h
  simp [hDiverged] at h
  cases h
  rfl

inductive CandidateKind where
  | trend
  | pan
deriving DecidableEq, Repr

inductive CandidateState where
  | unresolved
  | provisional
  | confirmed
deriving DecidableEq, Repr

structure CandidateKeyMirror where
  level : Nat
  kind : CandidateKind
  side : Side
  previousCenterStart : Option Nat
  parentCenterStart : Nat
  parentZd : Int
  parentZg : Int
  segA : Interval
  lambdaC : Nat
deriving DecidableEq, Repr

structure StructuralPredicates where
  direction : Bool
  comparable : Bool
  extreme : Bool
deriving DecidableEq, Repr

def StructuralPredicates.resolvedState (predicates : StructuralPredicates) : CandidateState :=
  if predicates.direction && predicates.comparable && predicates.extreme then
    CandidateState.provisional
  else
    CandidateState.unresolved

structure CandidateObservationMirror where
  key : CandidateKeyMirror
  predicates : StructuralPredicates
  interval : Interval
  state : CandidateState
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def makeTrendObservation
    (level : Nat) (previous parent : CenterFrame) (segment : SegmentRow)
    (gates : FirstStructuralGates) : CandidateObservationMirror :=
  let predicates : StructuralPredicates :=
    { direction := true, comparable := gates.comparable, extreme := gates.extreme }
  let state := predicates.resolvedState
  { key :=
      { level := level, kind := CandidateKind.trend, side := gates.side,
        previousCenterStart := some previous.startIndex, parentCenterStart := parent.startIndex,
        parentZd := parent.zd, parentZg := parent.zg, segA := gates.segA,
        lambdaC := gates.lambdaC }
    predicates := predicates
    interval := { left := gates.lambdaC, right := segment.endIndex }
    state := state
    firstProvableAt := if state = CandidateState.provisional then some segment.endIndex else none
    confirmedAt := none }

def makePanObservation (level : Nat) (cert : PanDivCertMirror) : CandidateObservationMirror :=
  { key :=
      { level := level, kind := CandidateKind.pan, side := cert.side,
        previousCenterStart := none, parentCenterStart := cert.centerStart,
        parentZd := cert.centerZd, parentZg := cert.centerZg,
        segA := cert.segA, lambdaC := cert.segC.left }
    predicates := { direction := true, comparable := true, extreme := true }
    interval := cert.segC
    state := CandidateState.confirmed
    firstProvableAt := some cert.sourceIndex
    confirmedAt := none }

theorem trend_observation_first_clock_is_conditional
    (level : Nat) (previous parent : CenterFrame) (segment : SegmentRow)
    (gates : FirstStructuralGates) :
    (makeTrendObservation level previous parent segment gates).firstProvableAt =
      if (makeTrendObservation level previous parent segment gates).state =
          CandidateState.provisional then some segment.endIndex else none := by
  rfl

theorem pan_observation_is_confirmed
    (level : Nat) (cert : PanDivCertMirror) :
    (makePanObservation level cert).state = CandidateState.confirmed := by
  rfl

/-- 同 key episode 腿归约：右端较大者承载，Extreme 析取，首证钟只早不晚。 -/
def mergeStructuralLeg
    (acc leg : CandidateObservationMirror) : CandidateObservationMirror :=
  let carrier := if acc.interval.right < leg.interval.right then leg else acc
  let predicates : StructuralPredicates :=
    { carrier.predicates with extreme := acc.predicates.extreme || leg.predicates.extreme }
  let state := predicates.resolvedState
  let firstClock :=
    match acc.firstProvableAt with
    | some value => some value
    | none =>
        match leg.firstProvableAt with
        | some value => some value
        | none => if state = CandidateState.provisional then some carrier.interval.right else none
  { carrier with predicates := predicates, state := state, firstProvableAt := firstClock }

def insertStructuralLeg
    (leg : CandidateObservationMirror) : List CandidateObservationMirror ->
    List CandidateObservationMirror
  | [] => [leg]
  | acc :: rest =>
      if acc.key = leg.key then mergeStructuralLeg acc leg :: rest
      else acc :: insertStructuralLeg leg rest

def reduceStructuralLegs (legs : List CandidateObservationMirror) : List CandidateObservationMirror :=
  legs.foldl (fun acc leg => insertStructuralLeg leg acc) []

theorem merge_structural_leg_extreme_is_or
    (acc leg : CandidateObservationMirror) :
    (mergeStructuralLeg acc leg).predicates.extreme =
      (acc.predicates.extreme || leg.predicates.extreme) := by
  unfold mergeStructuralLeg
  split <;> rfl

structure MergedScanOutputMirror where
  points : List BspPointMirror
  panDivs : List PanDivCertMirror
  grades : List FirstClassGradeRecordMirror
  observations : List CandidateObservationMirror
deriving DecidableEq, Repr

/-! ## 3. c_p ownership 生命周期 -/

inductive CpLifecycle where
  | pending
  | closed
deriving DecidableEq, Repr

structure CpIdentity where
  level : Nat
  bCenterOrdinal : Nat
  departureMoveOrdinal : Option Nat
  sourceStart : Option Nat
deriving DecidableEq, Repr

structure ClosureCertificate where
  confirmSource : Nat
  terminalMoveOrdinal : Nat
  sourceEnd : Nat
  thirdPointSource : Nat
  reviewMoveOrdinal : Option Nat
  fullyQualified : Bool
deriving DecidableEq, Repr

structure CpOwnershipMirror where
  identity : CpIdentity
  lifecycle : CpLifecycle
  certificate : Option ClosureCertificate
deriving DecidableEq, Repr

def initCpOwnership (identity : CpIdentity) : CpOwnershipMirror :=
  { identity := identity, lifecycle := CpLifecycle.pending, certificate := none }

def cpDependenciesStableBefore (ownership : CpOwnershipMirror) (dirtyFrom : Nat) : Bool :=
  match ownership.lifecycle, ownership.certificate with
  | CpLifecycle.pending, _ => true
  | CpLifecycle.closed, some certificate =>
      decide (certificate.terminalMoveOrdinal < dirtyFrom) &&
        match certificate.reviewMoveOrdinal with
        | none => true
        | some review => decide (review < dirtyFrom)
  | CpLifecycle.closed, none => false

/-- terminal 脏则回 Pending；只有 review 脏则保 Closed、清 review 资格并等待复核。 -/
def invalidateDirtyCp (ownership : CpOwnershipMirror) (dirtyFrom : Nat) : CpOwnershipMirror :=
  if cpDependenciesStableBefore ownership dirtyFrom then ownership
  else
    match ownership.lifecycle, ownership.certificate with
    | CpLifecycle.closed, some certificate =>
        if dirtyFrom <= certificate.terminalMoveOrdinal then
          initCpOwnership ownership.identity
        else
          { ownership with
              certificate := some
                { certificate with reviewMoveOrdinal := none, fullyQualified := false } }
    | _, _ => ownership

/-- frontier 重建同身份且旧依赖稳定时，继承整个对象态；否则保留新建对象。 -/
def reinheritCp (rebuilt prior : CpOwnershipMirror) (dirtyFrom : Nat) : CpOwnershipMirror :=
  if rebuilt.identity = prior.identity && cpDependenciesStableBefore prior dirtyFrom then
    prior
  else
    rebuilt

/-- Stable revision 内 Closed 吸收；Pending 只有收到合法第三类闭合证书才原子闭合。 -/
def advanceCpLifecycle
    (ownership : CpOwnershipMirror) (closure : Option ClosureCertificate) : CpOwnershipMirror :=
  match ownership.lifecycle, closure with
  | CpLifecycle.closed, _ => ownership
  | CpLifecycle.pending, none => ownership
  | CpLifecycle.pending, some certificate =>
      { ownership with lifecycle := CpLifecycle.closed, certificate := some certificate }

theorem cp_initializes_pending (identity : CpIdentity) :
    (initCpOwnership identity).lifecycle = CpLifecycle.pending ∧
      (initCpOwnership identity).certificate = none := by
  exact ⟨rfl, rfl⟩

theorem cp_closed_absorbing_in_stable_revision
    (ownership : CpOwnershipMirror) (closure : Option ClosureCertificate)
    (hClosed : ownership.lifecycle = CpLifecycle.closed) :
    advanceCpLifecycle ownership closure = ownership := by
  unfold advanceCpLifecycle
  rw [hClosed]

theorem cp_dirty_terminal_reopens_pending
    (identity : CpIdentity) (certificate : ClosureCertificate) (dirtyFrom : Nat)
    (hDirty : dirtyFrom <= certificate.terminalMoveOrdinal) :
    invalidateDirtyCp
      { identity := identity, lifecycle := CpLifecycle.closed, certificate := some certificate }
      dirtyFrom = initCpOwnership identity := by
  have hNotLt : ¬ certificate.terminalMoveOrdinal < dirtyFrom := Nat.not_lt_of_ge hDirty
  unfold invalidateDirtyCp cpDependenciesStableBefore
  simp [hDirty, hNotLt]

theorem cp_reinherit_stable_identity
    (rebuilt prior : CpOwnershipMirror) (dirtyFrom : Nat)
    (hIdentity : rebuilt.identity = prior.identity)
    (hStable : cpDependenciesStableBefore prior dirtyFrom = true) :
    reinheritCp rebuilt prior dirtyFrom = prior := by
  unfold reinheritCp
  simp [hIdentity, hStable]

/-! ## 4. 独立 Rust wire 与逐产口桥 -/

inductive RustDirectionTag where
  | up
  | down
deriving DecidableEq, Repr

def RustDirectionTag.toLean : RustDirectionTag -> Direction
  | RustDirectionTag.up => Direction.up
  | RustDirectionTag.down => Direction.down

inductive RustSideTag where
  | long
  | short
deriving DecidableEq, Repr

def RustSideTag.toLean : RustSideTag -> Side
  | RustSideTag.long => Side.long
  | RustSideTag.short => Side.short

inductive RustCandidateKindTag where
  | trend
  | pan
deriving DecidableEq, Repr

def RustCandidateKindTag.toLean : RustCandidateKindTag -> CandidateKind
  | RustCandidateKindTag.trend => CandidateKind.trend
  | RustCandidateKindTag.pan => CandidateKind.pan

inductive RustCandidateStateTag where
  | unresolved
  | provisional
  | confirmed
deriving DecidableEq, Repr

def RustCandidateStateTag.toLean : RustCandidateStateTag -> CandidateState
  | RustCandidateStateTag.unresolved => CandidateState.unresolved
  | RustCandidateStateTag.provisional => CandidateState.provisional
  | RustCandidateStateTag.confirmed => CandidateState.confirmed

inductive RustGradeReasonTag where
  | missingLeave
  | missingRetest
  | sameDirection
  | leaveNotOutside
  | retestReentered
deriving DecidableEq, Repr

def RustGradeReasonTag.toLean : RustGradeReasonTag -> GradeReason
  | RustGradeReasonTag.missingLeave => GradeReason.missingLeave
  | RustGradeReasonTag.missingRetest => GradeReason.missingRetest
  | RustGradeReasonTag.sameDirection => GradeReason.sameDirection
  | RustGradeReasonTag.leaveNotOutside => GradeReason.leaveNotOutside
  | RustGradeReasonTag.retestReentered => GradeReason.retestReentered

inductive RustGradeExtractionValue where
  | present (leaveLeft leaveRight retestLeft retestRight : Nat)
  | missing (reason : RustGradeReasonTag)
deriving DecidableEq, Repr

def RustGradeExtractionValue.toLean : RustGradeExtractionValue -> Grade
  | RustGradeExtractionValue.present leaveLeft leaveRight retestLeft retestRight =>
      Grade.present { left := leaveLeft, right := leaveRight }
        { left := retestLeft, right := retestRight }
  | RustGradeExtractionValue.missing reason => Grade.missing reason.toLean

inductive RustCpLifecycleTag where
  | pending
  | closed
deriving DecidableEq, Repr

def RustCpLifecycleTag.toLean : RustCpLifecycleTag -> CpLifecycle
  | RustCpLifecycleTag.pending => CpLifecycle.pending
  | RustCpLifecycleTag.closed => CpLifecycle.closed

structure RustBspPointExtraction where
  sourceIndex : Nat
  side : RustSideTag
  type1Bit : Bool
  type3Bit : Bool
  structureBreak : Bool
  ownerCenterStart : Nat
deriving DecidableEq, Repr

def BspParity (rust : RustBspPointExtraction) (lean : BspPointMirror) : Prop :=
  rust.sourceIndex = lean.sourceIndex ∧ rust.side.toLean = lean.side ∧
  rust.type1Bit = lean.type1Bit ∧ rust.type3Bit = lean.type3Bit ∧
  rust.structureBreak = lean.structureBreak ∧ rust.ownerCenterStart = lean.ownerCenterStart

structure RustPanDivExtraction where
  sourceIndex : Nat
  side : RustSideTag
  centerStart : Nat
  centerZd : Int
  centerZg : Int
  segALeft : Nat
  segARight : Nat
  segCLeft : Nat
  segCRight : Nat
deriving DecidableEq, Repr

def PanParity (rust : RustPanDivExtraction) (lean : PanDivCertMirror) : Prop :=
  rust.sourceIndex = lean.sourceIndex ∧ rust.side.toLean = lean.side ∧
  rust.centerStart = lean.centerStart ∧ rust.centerZd = lean.centerZd ∧
  rust.centerZg = lean.centerZg ∧ rust.segALeft = lean.segA.left ∧
  rust.segARight = lean.segA.right ∧ rust.segCLeft = lean.segC.left ∧
  rust.segCRight = lean.segC.right

structure RustGradeExtraction where
  level : Nat
  sourceIndex : Nat
  side : RustSideTag
  centerStart : Nat
  centerEnd : Nat
  centerZd : Int
  centerZg : Int
  grade : RustGradeExtractionValue
deriving DecidableEq, Repr

def GradeParity (rust : RustGradeExtraction) (lean : FirstClassGradeRecordMirror) : Prop :=
  rust.level = lean.level ∧ rust.sourceIndex = lean.sourceIndex ∧
  rust.side.toLean = lean.side ∧ rust.centerStart = lean.centerStart ∧
  rust.centerEnd = lean.centerEnd ∧ rust.centerZd = lean.centerZd ∧
  rust.centerZg = lean.centerZg ∧ rust.grade.toLean = lean.grade

structure RustCandidateExtraction where
  level : Nat
  kind : RustCandidateKindTag
  side : RustSideTag
  previousCenterStart : Option Nat
  parentCenterStart : Nat
  parentZd : Int
  parentZg : Int
  segALeft : Nat
  segARight : Nat
  lambdaC : Nat
  direction : Bool
  comparable : Bool
  extreme : Bool
  intervalLeft : Nat
  intervalRight : Nat
  state : RustCandidateStateTag
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def CandidateParity (rust : RustCandidateExtraction) (lean : CandidateObservationMirror) : Prop :=
  rust.level = lean.key.level ∧ rust.kind.toLean = lean.key.kind ∧
  rust.side.toLean = lean.key.side ∧ rust.previousCenterStart = lean.key.previousCenterStart ∧
  rust.parentCenterStart = lean.key.parentCenterStart ∧ rust.parentZd = lean.key.parentZd ∧
  rust.parentZg = lean.key.parentZg ∧ rust.segALeft = lean.key.segA.left ∧
  rust.segARight = lean.key.segA.right ∧ rust.lambdaC = lean.key.lambdaC ∧
  rust.direction = lean.predicates.direction ∧ rust.comparable = lean.predicates.comparable ∧
  rust.extreme = lean.predicates.extreme ∧ rust.intervalLeft = lean.interval.left ∧
  rust.intervalRight = lean.interval.right ∧ rust.state.toLean = lean.state ∧
  rust.firstProvableAt = lean.firstProvableAt ∧ rust.confirmedAt = lean.confirmedAt

structure RustCpExtraction where
  level : Nat
  bCenterOrdinal : Nat
  departureMoveOrdinal : Option Nat
  sourceStart : Option Nat
  lifecycle : RustCpLifecycleTag
  certificatePresent : Bool
deriving DecidableEq, Repr

def CpParity (rust : RustCpExtraction) (lean : CpOwnershipMirror) : Prop :=
  rust.level = lean.identity.level ∧ rust.bCenterOrdinal = lean.identity.bCenterOrdinal ∧
  rust.departureMoveOrdinal = lean.identity.departureMoveOrdinal ∧
  rust.sourceStart = lean.identity.sourceStart ∧ rust.lifecycle.toLean = lean.lifecycle ∧
  rust.certificatePresent = lean.certificate.isSome

def ListParity (relation : alpha -> beta -> Prop) : List alpha -> List beta -> Prop
  | [], [] => True
  | first :: firstRest, second :: secondRest =>
      relation first second ∧ ListParity relation firstRest secondRest
  | _, _ => False

structure RustMergedScanExtraction where
  points : List RustBspPointExtraction
  panDivs : List RustPanDivExtraction
  grades : List RustGradeExtraction
  observations : List RustCandidateExtraction
deriving DecidableEq, Repr

/--
四个产口分别逐项、同长、同序；不能用合计数相等替代。
这是底层诊断关系，不是顶层防绕过入口；完整入口还必须从 Rust input wire 现场运行 merged scan driver。
-/
def OutputParity (rust : RustMergedScanExtraction) (lean : MergedScanOutputMirror) : Prop :=
  ListParity BspParity rust.points lean.points ∧
  ListParity PanParity rust.panDivs lean.panDivs ∧
  ListParity GradeParity rust.grades lean.grades ∧
  ListParity CandidateParity rust.observations lean.observations

def CpListParity : List RustCpExtraction -> List CpOwnershipMirror -> Prop :=
  ListParity CpParity

/-- 各产口规范入口：Lean 结果必须由对应镜像函数现场计算。 -/
def FirstBspCheck (input : FirstProjectionInput) (rust : Option RustBspPointExtraction) : Prop :=
  match rust, judgeFirstFromGates input with
  | none, none => True
  | some rustPoint, some leanPoint => BspParity rustPoint leanPoint
  | _, _ => False

def ThirdBspCheck (input : ThirdProjectionInput) (rust : Option RustBspPointExtraction) : Prop :=
  match rust, judgeThirdPoint input with
  | none, none => True
  | some rustPoint, some leanPoint => BspParity rustPoint leanPoint
  | _, _ => False

def PanCheck (input : PanProjectionInput) (rust : Option RustPanDivExtraction) : Prop :=
  match rust, assemblePanDivCert input with
  | none, none => True
  | some rustCert, some leanCert => PanParity rustCert leanCert
  | _, _ => False

def GradeCheck (input : FirstProjectionInput) (rust : Option RustGradeExtraction) : Prop :=
  match rust, assembleFirstClassGrade input with
  | none, none => True
  | some rustGrade, some leanGrade => GradeParity rustGrade leanGrade
  | _, _ => False

def TrendObservationCheck
    (level : Nat) (previous parent : CenterFrame) (segment : SegmentRow)
    (gates : FirstStructuralGates) (rust : RustCandidateExtraction) : Prop :=
  CandidateParity rust (makeTrendObservation level previous parent segment gates)

def PanObservationCheck
    (level : Nat) (cert : PanDivCertMirror) (rust : RustCandidateExtraction) : Prop :=
  CandidateParity rust (makePanObservation level cert)

def StableCpAdvanceCheck
    (before : CpOwnershipMirror) (closure : Option ClosureCertificate)
    (rustAfter : RustCpExtraction) : Prop :=
  CpParity rustAfter (advanceCpLifecycle before closure)

theorem output_parity_exposes_each_port
    (rust : RustMergedScanExtraction) (lean : MergedScanOutputMirror)
    (h : OutputParity rust lean) :
    ListParity BspParity rust.points lean.points ∧
    ListParity PanParity rust.panDivs lean.panDivs ∧
    ListParity GradeParity rust.grades lean.grades ∧
    ListParity CandidateParity rust.observations lean.observations :=
  h

theorem bsp_bridge_rejects_side_mismatch :
    ¬ BspParity
      { sourceIndex := 1, side := RustSideTag.short, type1Bit := true, type3Bit := false,
        structureBreak := true, ownerCenterStart := 0 }
      { sourceIndex := 1, side := Side.long, type1Bit := true, type3Bit := false,
        structureBreak := true, ownerCenterStart := 0 } := by
  rintro ⟨_, hSide, _⟩
  cases hSide

end NewChanlun.Origin.UnifiedScanMirrorBridge
