/-
Origin/UnifiedScanMirrorBridge.lean

#1080 Phase 3：3a 统一扫描装配语义的完整 Lean 镜像与 Rust 提取对拍协议。

本文件只镜像当前生产装配语义，不参与生产判定，也不增删缠论判据。镜面分三段：
1. resume prelude：有序/平行数组守卫、最近中枢、首匹配语义、A 段缓存、lambda_C episode；
2. 四产口：BspPoint（3a 内只含一/三类）、PanDivCert、FirstClassGradeRecord、
   CandidateObservation 及同 episode 腿归约；
3. LevelState.cp_ownership：初始化、dirty 失效、稳定重继承、Pending/Closed 推进。

桥接形态复用 Origin 既有先例：Rust wire 与 Lean 镜像是独立类型，BridgesTo/Parity 逐字段校验。
单产口 Check 现场运行对应 Lean 函数；`mergedScanMirror` 另锁 confirmed-prefix 四缓存、tail
重判、逐产口稳定排序及 observation 归约后的 `(interval,key)` canonical 顺序。

认识论等级 L0：编译只证明本文件的定义内性质。只有真实 Rust 窗口提取执行器逐字段通过时，才可
宣称对应窗口跨语言签收。
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
deriving DecidableEq, Repr, Ord

structure Interval where
  left : Nat
  right : Nat
deriving DecidableEq, Repr, Ord

structure SegmentRow where
  direction : Direction
  startIndex : Nat
  endIndex : Nat
  startPrice : Int := 0
  endPrice : Int
  anchor : Option Direction
  departureEnd : Option Nat
deriving DecidableEq, Repr

/-- `merged_scan_resume` receives geometry and its two parallel arrays separately. -/
structure RawSegmentFrame where
  direction : Direction
  startIndex : Nat
  endIndex : Nat
  startPrice : Int
  endPrice : Int
deriving DecidableEq, Repr

structure RawStrokeFrame where
  direction : Direction
  startIndex : Nat
  endIndex : Nat
  startPrice : Int
  endPrice : Int
deriving DecidableEq, Repr

inductive DivergenceGaugeMirror where
  | forceL
  | macdArea
  | thetaDom
  | conjunction
  | thetaLex
deriving DecidableEq, Repr

structure CenterFrame where
  startIndex : Nat
  endIndex : Nat
  zd : Int
  zg : Int
  /-- Owner/c_p certificate parity also carries the outer envelope. -/
  dd : Int := 0
  gg : Int := 0
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

/-- Batch-shared market series: one serialized value per checkpoint, referenced by every level. -/
structure MergedScanRawSeries where
  histBits : List Nat
  difBits : List Nat
  closesTicks : List Int
  closeSrc : List Nat
  gauge : DivergenceGaugeMirror
  strokes : List RawStrokeFrame
deriving Repr

/-- Exact level-local input boundary of production `merged_scan_resume`, excluding mutable cache. -/
structure MergedScanResumeRaw where
  level : Nat
  centers : List CenterFrame
  segments : List RawSegmentFrame
  anchorDirs : Option (List (Option Direction))
  departureEnds : Option (List Nat)
  blocks : List MoveBlock
  prefixCount : Nat
  dirtyE : Nat
deriving Repr

def attachAnchorsExact : List RawSegmentFrame -> List (Option Direction) -> Option (List SegmentRow)
  | [], [] => some []
  | segment :: segments, anchor :: anchors => do
      let rest ← attachAnchorsExact segments anchors
      let row : SegmentRow :=
        { direction := segment.direction
          startIndex := segment.startIndex
          endIndex := segment.endIndex
          startPrice := segment.startPrice
          endPrice := segment.endPrice
          anchor := anchor
          departureEnd := none }
      pure (row :: rest)
  | _, _ => none

def attachSelfAnchors (segments : List RawSegmentFrame) : List SegmentRow :=
  segments.map fun segment =>
    { direction := segment.direction
      startIndex := segment.startIndex
      endIndex := segment.endIndex
      startPrice := segment.startPrice
      endPrice := segment.endPrice
      anchor := some segment.direction
      departureEnd := none }

def attachDepartureEndsExact : List SegmentRow -> List Nat -> Option (List SegmentRow)
  | [], [] => some []
  | segment :: segments, departureEnd :: departureEnds => do
      let rest ← attachDepartureEndsExact segments departureEnds
      pure ({ segment with departureEnd := some departureEnd } :: rest)
  | _, _ => none

/-- P-02 keeps both parallel-array length checks observable before any segment judgment. -/
def materializeResumeSegments (raw : MergedScanResumeRaw) : Option (List SegmentRow) := do
  let anchored ← match raw.anchorDirs with
    | none => some (attachSelfAnchors raw.segments)
    | some anchors => attachAnchorsExact raw.segments anchors
  match raw.departureEnds with
  | none => some anchored
  | some departureEnds => attachDepartureEndsExact anchored departureEnds

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

def firstIndexGeAux (values : List Nat) (target index : Nat) : Option Nat :=
  match values with
  | [] => none
  | value :: rest => if target <= value then some index else firstIndexGeAux rest target (index + 1)

def lastIndexLeAux (values : List Nat) (target index : Nat) (last : Option Nat) : Option Nat :=
  match values with
  | [] => last
  | value :: rest =>
      if value <= target then lastIndexLeAux rest target (index + 1) (some index) else last

/-- Production `map_src_range_to_close_idx`, including its empty/invalid interval behavior. -/
def mapSrcRangeToCloseIdx (closeSrc : List Nat) (startIndex endIndex : Nat) : Option Interval :=
  if endIndex < startIndex then none
  else do
    let lo ← firstIndexGeAux closeSrc startIndex 0
    let hi ← lastIndexLeAux closeSrc endIndex 0 none
    if lo <= hi then some { left := lo, right := hi } else none

/-- Exact endpoint envelope for all complete segments inside a source-index span. -/
def moveRangeEnvelope (rows : List SegmentRow) (span : Interval) : Option (Int × Int) :=
  let inside := rows.filter fun row => span.left <= row.startIndex && row.endIndex <= span.right
  inside.foldl (fun acc row =>
    let lo := min row.startPrice row.endPrice
    let hi := max row.startPrice row.endPrice
    match acc with
    | none => some (lo, hi)
    | some old => some (min old.1 lo, max old.2 hi)) none

/-- P-07 miss computation: current post-reentry departure episode between adjacent centers. -/
def locateDepartureMoveA (rows : List SegmentRow) (previous current : CenterFrame)
    (trend : Direction) : Option ASegment := do
  let window := rows.filter fun row =>
    previous.endIndex <= row.startIndex && row.startIndex < current.endIndex
  let boundary := lastReentryBoundary window previous trend
  let anchored := window.filter fun row =>
    row.anchor == some trend && boundary <= row.startIndex
  let first ← anchored.head?
  let last ← anchored.getLast?
  let span : Interval := { left := first.startIndex, right := last.endIndex }
  let envelope ← moveRangeEnvelope rows span
  pure { span := span, envelopeLow := envelope.1, envelopeHigh := envelope.2 }

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

structure ResolvedFirstGate where
  gates : FirstStructuralGates
  aIndex : Option Interval
  cIndex : Option Interval
deriving DecidableEq, Repr

/-- P-08/P-09 from raw series: λ_C and both close-index mappings are computed in Lean. -/
def resolveFirstGateRaw (rows : List SegmentRow) (closeSrc : List Nat)
    (center : CenterFrame) (trend : Direction) (segment : SegmentRow)
    (aSegment : Option ASegment) : Option ResolvedFirstGate := do
  let a ← aSegment
  let lambdaC ← lambdaCEpisode rows center trend segment.startIndex
  let aIndex := mapSrcRangeToCloseIdx closeSrc a.span.left a.span.right
  let cIndex := mapSrcRangeToCloseIdx closeSrc lambdaC segment.endIndex
  let gates ← firstStructuralGates
    { center := center, trendDirection := trend, segment := segment
      aSegment := some a, lambdaC := some lambdaC
      aMapped := aIndex.isSome, cMapped := cIndex.isSome }
  pure { gates := gates, aIndex := aIndex, cIndex := cIndex }

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

structure ForceFeatureBitsMirror where
  macdAreaBits : Nat
  difPeakBits : Nat
  priceAmplitude : Int
  priceSpeedBits : Nat
deriving DecidableEq, Repr

structure ForceProxiesBitsMirror where
  segA : ForceFeatureBitsMirror
  segC : ForceFeatureBitsMirror
deriving DecidableEq, Repr

/-- `force_features` 的独立 raw slice；IEEE-754 输入只以 bits 过线。 -/
structure ForceSegmentRawMirror where
  histBits : List Nat
  difBits : List Nat
  closes : List Int
  direction : Direction
deriving DecidableEq, Repr

def floatsOfBits (bits : List Nat) : List Float :=
  bits.map fun value => Float.ofBits (UInt64.ofNat value)

def sameColorAreaRaw (hist : List Float) (direction : Direction) : Float :=
  hist.foldl (fun area value =>
    match direction with
    | .up => if 0.0 < value then area + value else area
    | .down => if value < 0.0 then area + value.abs else area) 0.0

def difPeakRaw (dif : List Float) (direction : Direction) : Float :=
  match direction with
  | .up => dif.foldl (fun peak value => if peak < value then value else peak) 0.0
  | .down =>
      (dif.foldl (fun trough value => if value < trough then value else trough) 0.0).abs

def rawPriceAmplitude (closes : List Int) : Int :=
  match closes.head?, closes.getLast? with
  | some first, some last => Int.ofNat (last - first).natAbs
  | _, _ => 0

def rawPriceSpeed (closes : List Int) : Float :=
  let amplitude := Int64.ofInt (rawPriceAmplitude closes) |>.toFloat
  let span := UInt64.ofNat (max (closes.length - 1) 1) |>.toFloat
  amplitude / span

/-- 逐算术步骤镜像 Rust `force_features`，结果再转回四个 wire 字段。 -/
def forceFeatureFromRaw (raw : ForceSegmentRawMirror) : ForceFeatureBitsMirror :=
  { macdAreaBits := (sameColorAreaRaw (floatsOfBits raw.histBits) raw.direction).toBits.toNat
    difPeakBits := (difPeakRaw (floatsOfBits raw.difBits) raw.direction).toBits.toNat
    priceAmplitude := rawPriceAmplitude raw.closes
    priceSpeedBits := (rawPriceSpeed raw.closes).toBits.toNat }

def closedSlice (values : List alpha) (span : Interval) : List alpha :=
  if span.right < span.left || values.length <= span.right then []
  else (values.drop span.left).take (span.right - span.left + 1)

def forceFeatureFromSeries (histBits difBits : List Nat) (closes : List Int)
    (span : Interval) (direction : Direction) : ForceFeatureBitsMirror :=
  forceFeatureFromRaw
    { histBits := closedSlice histBits span
      difBits := closedSlice difBits span
      closes := closedSlice closes span
      direction := direction }

def sameDirectionHistPeakRaw (hist : List Float) (direction : Direction) : Float :=
  match direction with
  | .up => hist.foldl (fun peak value => if peak < value then value else peak) 0.0
  | .down => (hist.foldl (fun trough value => if value < trough then value else trough) 0.0).abs

def forceDominated (a c : ForceFeatureBitsMirror) : Bool :=
  let aArea := Float.ofBits (UInt64.ofNat a.macdAreaBits)
  let cArea := Float.ofBits (UInt64.ofNat c.macdAreaBits)
  let aDif := Float.ofBits (UInt64.ofNat a.difPeakBits)
  let cDif := Float.ofBits (UInt64.ofNat c.difPeakBits)
  let aSpeed := Float.ofBits (UInt64.ofNat a.priceSpeedBits)
  let cSpeed := Float.ofBits (UInt64.ofNat c.priceSpeedBits)
  let weaker := cArea < aArea || cDif < aDif || c.priceAmplitude < a.priceAmplitude ||
    cSpeed < aSpeed
  let stronger := aArea < cArea || aDif < cDif || a.priceAmplitude < c.priceAmplitude ||
    aSpeed < cSpeed
  weaker && !stronger

def forceThetaLex (a c : ForceFeatureBitsMirror) : Bool :=
  let aDif := Float.ofBits (UInt64.ofNat a.difPeakBits)
  let cDif := Float.ofBits (UInt64.ofNat c.difPeakBits)
  if cDif != aDif then cDif < aDif
  else
    let aArea := Float.ofBits (UInt64.ofNat a.macdAreaBits)
    let cArea := Float.ofBits (UInt64.ofNat c.macdAreaBits)
    cArea < aArea

def strokeVelocityRaw (stroke : RawStrokeFrame) : Float :=
  let bars := UInt64.ofNat (stroke.endIndex - stroke.startIndex + 1) |>.toFloat
  let delta := Int64.ofInt (stroke.endPrice - stroke.startPrice) |>.toFloat
  let sign := match stroke.direction with | .up => 1.0 | .down => -1.0
  sign * delta / bars

def segmentForceLRaw (strokes : List RawStrokeFrame) (span : Interval) : Option Float := do
  let inside := strokes.filter fun stroke =>
    span.left <= stroke.endIndex && stroke.startIndex <= span.right
  let first ← inside.head?
  let last ← inside.getLast?
  pure (strokeVelocityRaw last - strokeVelocityRaw first)

def confirmDivergenceRaw (gauge : DivergenceGaugeMirror) (a c : ForceFeatureBitsMirror)
    (strokes : List RawStrokeFrame) (aSource cSource : Interval) : Bool :=
  let aArea := Float.ofBits (UInt64.ofNat a.macdAreaBits)
  let cArea := Float.ofBits (UInt64.ofNat c.macdAreaBits)
  let macdWeak := cArea < aArea
  match gauge with
  | .forceL =>
      match segmentForceLRaw strokes aSource, segmentForceLRaw strokes cSource with
      | some la, some lc => lc < la
      | _, _ => false
  | .macdArea => macdWeak
  | .thetaDom => forceDominated a c
  | .conjunction => macdWeak && forceDominated a c
  | .thetaLex => forceThetaLex a c

structure FirstProjectionInput where
  level : Nat
  center : CenterFrame
  segment : SegmentRow
  gates : FirstStructuralGates
  diverged : Bool
  t3Present : Bool
  grade : Grade
  forceSegA : Option ForceSegmentRawMirror := none
  forceSegC : Option ForceSegmentRawMirror := none
deriving DecidableEq, Repr

def assembleForceProxies (a c : Option ForceSegmentRawMirror) : Option ForceProxiesBitsMirror :=
  match a, c with
  | some segA, some segC =>
      some { segA := forceFeatureFromRaw segA, segC := forceFeatureFromRaw segC }
  | _, _ => none

structure ThirdClassEntryIdentityMirror where
  centerSi : Nat
  centerZd : Int
  centerZg : Int
  leaveInterval : Interval
  retestInterval : Interval
deriving DecidableEq, Repr

structure BspBitsMirror where
  buy1 : Bool := false
  buy2 : Bool := false
  buy3 : Bool := false
  sell1 : Bool := false
  sell2 : Bool := false
  sell3 : Bool := false
  thirdClassEntry : Option ThirdClassEntryIdentityMirror := none
deriving DecidableEq, Repr

inductive OwnerRefMirror where
  | center (value : CenterFrame)
  | type1Anchor (sourceIndex : Nat)
deriving DecidableEq, Repr

/-- 3a 直接产口的完整 `BspPoint` 镜面；二类仍由 pipeline 07b 另装。 -/
structure BspPointMirror where
  sourceIndex : Nat
  bits : BspBitsMirror
  pivotLow : Int
  pivotHigh : Int
  owner : Option OwnerRefMirror
  structBreakDir : Option Side
  force : Option ForceProxiesBitsMirror
  retraceBreaksType1 : Option Bool
deriving DecidableEq, Repr

/-! ### 2.1 07b production second-signal raw mirror

`extract_second_signals` consumes recursive `RMove::Compose` legs plus two frozen upstream
oracles (divergence and source coordinate).  The diagnostic wire below carries exactly those raw
values.  This mirror deliberately does not consume the Rust output while deciding the result.
-/

structure SecondLegRawMirror where
  direction : Direction
  lo : Int
  hi : Int
  diverged : Bool
  sourceIndex : Nat
deriving DecidableEq, Repr

def secondLegBreaksCenter (center : CenterFrame) (side : Side)
    (leg : SecondLegRawMirror) : Bool :=
  match side with
  | .long => decide (leg.lo < center.zd)
  | .short => decide (center.zg < leg.hi)

def secondLegIsType1 (center : CenterFrame) (side : Side)
    (leg : SecondLegRawMirror) : Bool :=
  secondLegBreaksCenter center side leg && leg.diverged

/-- Production first-match semantics: the pullback is the first immediate successor of the first
complete type-one departure.  A type-one departure in the final slot has no pullback and stops the
search, exactly as `find_second_type_structure` does. -/
def firstSecondLegPair (center : CenterFrame) (side : Side) :
    List SecondLegRawMirror -> Option (SecondLegRawMirror × SecondLegRawMirror)
  | [] => none
  | [_] => none
  | first :: second :: rest =>
      if secondLegIsType1 center side first then some (first, second)
      else firstSecondLegPair center side (second :: rest)

def secondBits (side : Side) : BspBitsMirror :=
  match side with
  | .long => { buy2 := true }
  | .short => { sell2 := true }

def secondPointPrice (side : Side) (pullback : SecondLegRawMirror) : Int :=
  match side with
  | .long => pullback.lo
  | .short => pullback.hi

def secondRetraceBreaksType1 (side : Side)
    (departure pullback : SecondLegRawMirror) : Bool :=
  match side with
  | .long => decide (pullback.lo < departure.lo)
  | .short => decide (departure.hi < pullback.hi)

/-- Independent 07b/Type1Anchor recomputation of production `extract_second_signals`. -/
def extractSecondSignalsMirror (center : CenterFrame) (side : Side) (_level : Nat)
    (legs : List SecondLegRawMirror) : List BspPointMirror :=
  match firstSecondLegPair center side legs with
  | none => []
  | some (departure, pullback) =>
      let bits := secondBits side
      let price := secondPointPrice side pullback
      [{ sourceIndex := pullback.sourceIndex
         bits := bits
         pivotLow := if bits.buy2 then price else 0
         pivotHigh := if bits.sell2 then price else 0
         owner := some (.type1Anchor departure.sourceIndex)
         structBreakDir := none
         force := none
         retraceBreaksType1 := some (secondRetraceBreaksType1 side departure pullback) }]

theorem second_production_long_witness :
    extractSecondSignalsMirror
      { startIndex := 0, endIndex := 0, zd := 0, zg := 4, dd := -2, gg := 6 }
      .long 1
      [{ direction := .down, lo := -10, hi := -2, diverged := true, sourceIndex := 30 },
       { direction := .up, lo := -8, hi := 3, diverged := false, sourceIndex := 42 },
       { direction := .up, lo := 1, hi := 5, diverged := false, sourceIndex := 50 }] =
      [{ sourceIndex := 42, bits := { buy2 := true }, pivotLow := -8, pivotHigh := 0,
         owner := some (.type1Anchor 30), structBreakDir := none, force := none,
         retraceBreaksType1 := some false }] := by
  decide

def firstBits (side : Side) (confirmed : Bool) : BspBitsMirror :=
  match side with
  | Side.long => { buy1 := confirmed }
  | Side.short => { sell1 := confirmed }

def judgeFirstFromGates (input : FirstProjectionInput) : Option BspPointMirror :=
  if input.gates.extreme && input.gates.comparable then
    let confirmed := input.diverged && input.t3Present
    let bits := firstBits input.gates.side confirmed
    some
      { sourceIndex := input.segment.departureEnd.getD input.segment.endIndex
        bits := bits
        pivotLow := if bits.buy1 then input.segment.endPrice else 0
        pivotHigh := if bits.sell1 then input.segment.endPrice else 0
        owner := some (.center input.center)
        structBreakDir := some input.gates.side
        force := assembleForceProxies input.forceSegA input.forceSegC
        retraceBreaksType1 := none }
  else
    none

theorem first_projection_keeps_structural_candidate
    (input : FirstProjectionInput) (point : BspPointMirror)
    (h : judgeFirstFromGates input = some point) :
    point.structBreakDir = some input.gates.side := by
  unfold judgeFirstFromGates at h
  split at h
  · cases h
    rfl
  · contradiction

structure ThirdProjectionInput where
  center : CenterFrame
  leave : SegmentRow
  retest : SegmentRow
deriving DecidableEq, Repr

def judgeThirdPoint (input : ThirdProjectionInput) : Option BspPointMirror :=
  match input.leave.anchor, input.retest.direction with
  | some Direction.up, Direction.down =>
        if input.center.zg < input.leave.endPrice && input.center.zg < input.retest.endPrice then
          some
            { sourceIndex := input.retest.endIndex
              bits :=
                { buy3 := true
                  thirdClassEntry := some
                    { centerSi := input.center.startIndex
                      centerZd := input.center.zd, centerZg := input.center.zg
                      leaveInterval := { left := input.leave.startIndex, right := input.leave.endIndex }
                      retestInterval := { left := input.retest.startIndex, right := input.retest.endIndex } } }
              pivotLow := input.retest.endPrice, pivotHigh := 0
              owner := some (.center input.center), structBreakDir := none
              force := none, retraceBreaksType1 := none }
        else none
  | some Direction.down, Direction.up =>
        if input.leave.endPrice < input.center.zd && input.retest.endPrice < input.center.zd then
          some
            { sourceIndex := input.retest.endIndex
              bits :=
                { sell3 := true
                  thirdClassEntry := some
                    { centerSi := input.center.startIndex
                      centerZd := input.center.zd, centerZg := input.center.zg
                      leaveInterval := { left := input.leave.startIndex, right := input.leave.endIndex }
                      retestInterval := { left := input.retest.startIndex, right := input.retest.endIndex } } }
              pivotLow := 0, pivotHigh := input.retest.endPrice
              owner := some (.center input.center), structBreakDir := none
              force := none, retraceBreaksType1 := none }
        else none
  | _, _ => none

def thirdBuyWitness : ThirdProjectionInput :=
  { center := { startIndex := 1, endIndex := 5, zd := 10, zg := 20 }
    leave :=
      { direction := Direction.up, startIndex := 6, endIndex := 7, endPrice := 30,
        anchor := some Direction.up, departureEnd := none }
    retest :=
      { direction := Direction.down, startIndex := 8, endIndex := 9, endPrice := 25,
        anchor := some Direction.down, departureEnd := none }
    }

theorem third_projection_sets_only_third_bit :
    judgeThirdPoint thirdBuyWitness =
      some
        { sourceIndex := 9
          bits :=
            { buy3 := true
              thirdClassEntry := some
                { centerSi := 1, centerZd := 10, centerZg := 20
                  leaveInterval := { left := 6, right := 7 }
                  retestInterval := { left := 8, right := 9 } } }
          pivotLow := 25, pivotHigh := 0
          owner := some (.center thirdBuyWitness.center), structBreakDir := none
          force := none, retraceBreaksType1 := none } := by
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
  centerEnd : Nat
  centerZd : Int
  centerZg : Int
  centerDd : Int
  centerGg : Int
  segA : Interval
  segC : Interval
deriving DecidableEq, Repr

def panSideRaw (center : CenterFrame) (segment : SegmentRow) : Option Side :=
  match segment.direction with
  | .down => if segment.endPrice < center.zd then some .long else none
  | .up => if center.zg < segment.endPrice then some .short else none

def rowReenters (center : CenterFrame) (direction : Direction) (row : SegmentRow) : Bool :=
  row.direction != direction && match direction with
    | .down => decide (center.zd <= row.endPrice)
    | .up => decide (row.endPrice <= center.zg)

def locatePanStructureNarrow (rows : List SegmentRow) (center : CenterFrame)
    (segment : SegmentRow) : Option PanStructure := do
  let side ← panSideRaw center segment
  let direction := segment.direction
  let window := rows.filter fun row =>
    center.endIndex <= row.startIndex && row.startIndex <= segment.startIndex
  if !(window.any fun row => rowReenters center direction row && row.endIndex <= segment.startIndex)
    then none else pure ()
  let lambdaC ← lambdaCEpisode rows center direction segment.startIndex
  let anchors := window.filter fun row => row.direction = direction && row.endIndex <= lambdaC &&
    match direction with
    | .down => row.endPrice < center.zd
    | .up => center.zg < row.endPrice
  let aAnchor ← anchors.getLast?
  if !(window.any fun row => rowReenters center direction row &&
    aAnchor.endIndex <= row.startIndex && row.endIndex <= lambdaC) then none else pure ()
  let lambdaA ← lambdaCEpisode rows center direction aAnchor.startIndex
  let episodeEnd := match (window.filter fun row =>
    rowReenters center direction row && aAnchor.endIndex <= row.startIndex).head? with
    | some row => row.startIndex
    | none => lambdaC
  let aRows := window.filter fun row => row.direction = direction && lambdaA <= row.startIndex &&
    row.endIndex <= episodeEnd
  let rhoA ← aRows.getLast?.map (fun row => row.endIndex)
  let segA : Interval := { left := lambdaA, right := rhoA }
  let segC : Interval := { left := lambdaC, right := segment.endIndex }
  let result : PanStructure :=
    { sourceIndex := segment.endIndex
      side := side
      center := center
      segA := segA
      segC := segC }
  pure result

def locatePanStructureFront (rows : List SegmentRow) (center : CenterFrame)
    (segment : SegmentRow) : Option PanStructure := do
  let side ← panSideRaw center segment
  let lambdaC ← lambdaCEpisode rows center segment.direction segment.startIndex
  let front ← (rows.filter fun row =>
    row.direction = segment.direction && row.endIndex <= center.startIndex).getLast?
  let segA : Interval := { left := front.startIndex, right := front.endIndex }
  let segC : Interval := { left := lambdaC, right := segment.endIndex }
  let result : PanStructure :=
    { sourceIndex := segment.endIndex
      side := side
      center := center
      segA := segA
      segC := segC }
  pure result

def segmentsDivergeOrRaw (series : MergedScanRawSeries) (side : Side)
    (aIndex cIndex : Interval) : Bool :=
  let direction := match side with | .long => Direction.down | .short => Direction.up
  let aForce := forceFeatureFromSeries series.histBits series.difBits series.closesTicks
    aIndex direction
  let cForce := forceFeatureFromSeries series.histBits series.difBits series.closesTicks
    cIndex direction
  let aArea := Float.ofBits (UInt64.ofNat aForce.macdAreaBits)
  let cArea := Float.ofBits (UInt64.ofNat cForce.macdAreaBits)
  let aDif := Float.ofBits (UInt64.ofNat aForce.difPeakBits)
  let cDif := Float.ofBits (UInt64.ofNat cForce.difPeakBits)
  let aPeak := sameDirectionHistPeakRaw (floatsOfBits (closedSlice series.histBits aIndex)) direction
  let cPeak := sameDirectionHistPeakRaw (floatsOfBits (closedSlice series.histBits cIndex)) direction
  cArea < aArea || cDif < aDif || cPeak < aPeak

def judgePanRaw (rows : List SegmentRow) (series : MergedScanRawSeries)
    (center : CenterFrame) (segment : SegmentRow) : Option PanDivCertMirror := do
  let panStructure ← match locatePanStructureNarrow rows center segment with
    | some value => some value
    | none => locatePanStructureFront rows center segment
  let aIndex ← mapSrcRangeToCloseIdx series.closeSrc panStructure.segA.left panStructure.segA.right
  let cIndex ← mapSrcRangeToCloseIdx series.closeSrc panStructure.segC.left panStructure.segC.right
  if !segmentsDivergeOrRaw series panStructure.side aIndex cIndex then none
  else
    let result : PanDivCertMirror :=
      { sourceIndex := panStructure.sourceIndex
        side := panStructure.side
        centerStart := panStructure.center.startIndex
        centerEnd := panStructure.center.endIndex
        centerZd := panStructure.center.zd
        centerZg := panStructure.center.zg
        centerDd := panStructure.center.dd
        centerGg := panStructure.center.gg
        segA := panStructure.segA
        segC := panStructure.segC }
    pure result

def assemblePanDivCert (input : PanProjectionInput) : Option PanDivCertMirror :=
  if input.consolidation && input.liftZero && input.comparable && input.diverged then
    input.panStructure.map fun pan =>
      { sourceIndex := pan.sourceIndex
        side := pan.side
        centerStart := pan.center.startIndex
        centerEnd := pan.center.endIndex
        centerZd := pan.center.zd
        centerZg := pan.center.zg
        centerDd := pan.center.dd
        centerGg := pan.center.gg
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

def gradeFixedFirstPairRaw (rows : List SegmentRow) (center : CenterFrame)
    (trend : Direction) : Grade :=
  let tail := rows.dropWhile fun row => row.startIndex < center.endIndex
  match tail with
  | [] => .missing .missingLeave
  | [_] => .missing .missingRetest
  | leave :: retest :: _ =>
      if leave.direction = retest.direction then .missing .sameDirection
      else
        let leaveOutside := match trend with
          | .up => decide (center.zg < leave.endPrice)
          | .down => decide (leave.endPrice < center.zd)
        if !leaveOutside then .missing .leaveNotOutside
        else
          let retestOutside := match trend with
            | .up => decide (center.zg < retest.endPrice)
            | .down => decide (retest.endPrice < center.zd)
          if !retestOutside then .missing .retestReentered
          else .present { left := leave.startIndex, right := leave.endIndex }
            { left := retest.startIndex, right := retest.endIndex }

structure FirstRawJudgment where
  point : Option BspPointMirror
  grade : Option FirstClassGradeRecordMirror
deriving DecidableEq, Repr

/-- O-01/O-03 from the full raw series; no Rust-produced predicate boolean is consumed. -/
def judgeFirstRaw (level : Nat) (rows : List SegmentRow) (series : MergedScanRawSeries)
    (center : CenterFrame) (trend : Direction) (segment : SegmentRow)
    (resolved : ResolvedFirstGate) : FirstRawJudgment :=
  if !resolved.gates.extreme then { point := none, grade := none }
  else match resolved.aIndex, resolved.cIndex with
  | some aIndex, some cIndex =>
      let aForce := forceFeatureFromSeries series.histBits series.difBits series.closesTicks
        aIndex trend
      let cForce := forceFeatureFromSeries series.histBits series.difBits series.closesTicks
        cIndex trend
      let diverged := confirmDivergenceRaw series.gauge aForce cForce series.strokes
        resolved.gates.segA { left := resolved.gates.lambdaC, right := segment.endIndex }
      let grade := gradeFixedFirstPairRaw rows center trend
      let confirmed := diverged && match grade with | .present _ _ => true | .missing _ => false
      let bits := firstBits resolved.gates.side confirmed
      let point : BspPointMirror :=
        { sourceIndex := segment.departureEnd.getD segment.endIndex
          bits := bits
          pivotLow := if bits.buy1 then segment.endPrice else 0
          pivotHigh := if bits.sell1 then segment.endPrice else 0
          owner := some (.center center)
          structBreakDir := some resolved.gates.side
          force := if series.difBits.isEmpty || series.closesTicks.isEmpty then none
            else some { segA := aForce, segC := cForce }
          retraceBreaksType1 := none }
      let gradeRecord := if diverged then some
        { level := level
          sourceIndex := segment.departureEnd.getD segment.endIndex
          side := resolved.gates.side
          centerStart := center.startIndex
          centerEnd := center.endIndex
          centerZd := center.zd
          centerZg := center.zg
          grade := grade }
        else none
      { point := some point, grade := gradeRecord }
  | _, _ => { point := none, grade := none }

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
deriving DecidableEq, Repr, Ord

inductive CandidateState where
  | unresolved
  | provisional
  | confirmed
deriving DecidableEq, Repr

structure CandidateKeyMirror where
  ruleVersion : Nat
  level : Nat
  kind : CandidateKind
  side : Side
  previousCenterStart : Option Nat
  parentCenterStart : Nat
  parentZd : Int
  parentZg : Int
  segA : Interval
  lambdaC : Nat
deriving DecidableEq, Repr, Ord

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
  kind : CandidateKind
  centerIds : Option (Nat × Nat)
  candidateGroupId : Nat
  pairId : Nat
  predicates : StructuralPredicates
  extremeProof : Interval
  thirdClassProof : Option Nat
  interval : Interval
  state : CandidateState
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def candidateRuleVersion : Nat := 1
def fnvOffsetBasis : Nat := 14695981039346656037
def fnvPrime : Nat := 1099511628211
def pairIdSeed : Nat := 9521211207457086692
def u64Modulus : Nat := 18446744073709551616

def u64 (value : Nat) : Nat := value % u64Modulus

def intAsU64 (value : Int) : Nat := (value.emod u64Modulus).toNat

def u64BytesLE (value : Nat) : List Nat :=
  [0, 8, 16, 24, 32, 40, 48, 56].map fun shift =>
    (Nat.shiftRight (u64 value) shift) % 256

def fnvByte (hash byte : Nat) : Nat := u64 ((Nat.xor hash byte) * fnvPrime)

def fnvU64 (hash value : Nat) : Nat := (u64BytesLE value).foldl fnvByte hash

def candidateKindCode : CandidateKind -> Nat
  | .trend => 0
  | .pan => 1

def sideCode : Side -> Nat
  | .long => 0
  | .short => 1

/-- Production `stable_id`, including exact i64-to-u64 casts and wrapping FNV-1a. -/
def candidateStableId (key : CandidateKeyMirror) (seed : Nat) : Nat :=
  let values :=
    [key.level, candidateKindCode key.kind, sideCode key.side,
     if key.previousCenterStart.isSome then 1 else 0,
     key.previousCenterStart.getD 0, key.parentCenterStart,
     intAsU64 key.parentZd, intAsU64 key.parentZg,
     key.segA.left, key.segA.right, key.lambdaC]
  values.foldl fnvU64 seed

theorem fnv_one_byte_known_vector :
    fnvByte fnvOffsetBasis 97 = 12638187200555641996 := by
  decide

def makeTrendObservation
    (level : Nat) (previous parent : CenterFrame) (segment : SegmentRow)
    (gates : FirstStructuralGates) : CandidateObservationMirror :=
  let key : CandidateKeyMirror :=
    { ruleVersion := candidateRuleVersion
      level := level, kind := CandidateKind.trend, side := gates.side
      previousCenterStart := some previous.startIndex, parentCenterStart := parent.startIndex
      parentZd := parent.zd, parentZg := parent.zg, segA := gates.segA
      lambdaC := gates.lambdaC }
  let predicates : StructuralPredicates :=
    { direction := true, comparable := gates.comparable, extreme := gates.extreme }
  let state := predicates.resolvedState
  { key := key
    kind := CandidateKind.trend
    centerIds := some (previous.startIndex, parent.startIndex)
    candidateGroupId := candidateStableId key fnvOffsetBasis
    pairId := candidateStableId key pairIdSeed
    predicates := predicates
    extremeProof := gates.segA
    thirdClassProof := none
    interval := { left := gates.lambdaC, right := segment.endIndex }
    state := state
    firstProvableAt := if state = CandidateState.provisional then some segment.endIndex else none
    confirmedAt := none }

def makePanObservation (level : Nat) (cert : PanDivCertMirror) : CandidateObservationMirror :=
  let key : CandidateKeyMirror :=
    { ruleVersion := candidateRuleVersion
      level := level, kind := CandidateKind.pan, side := cert.side
      previousCenterStart := none, parentCenterStart := cert.centerStart
      parentZd := cert.centerZd, parentZg := cert.centerZg
      segA := cert.segA, lambdaC := cert.segC.left }
  { key := key
    kind := CandidateKind.pan
    centerIds := none
    candidateGroupId := candidateStableId key fnvOffsetBasis
    pairId := candidateStableId key pairIdSeed
    predicates := { direction := true, comparable := true, extreme := true }
    extremeProof := cert.segA
    thirdClassProof := none
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

def candidateCanonicalLe (first second : CandidateObservationMirror) : Bool :=
  match compare first.interval second.interval with
  | Ordering.lt => true
  | Ordering.gt => false
  | Ordering.eq => compare first.key second.key != Ordering.gt

/-- Production canonical `(interval,key)` ordering after per-key reduction. -/
def canonicalSortCandidates (observations : List CandidateObservationMirror) :
    List CandidateObservationMirror :=
  observations.mergeSort candidateCanonicalLe

def reduceStructuralLegsCanonical (legs : List CandidateObservationMirror) :
    List CandidateObservationMirror :=
  canonicalSortCandidates (reduceStructuralLegs legs)

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

/-! ## 2.1 P-10 confirmed-prefix four-cache frontier driver -/

/-- One segment's already judged four-port material.  The driver, not the caller, owns caching. -/
structure ScanRowEmission where
  points : List BspPointMirror := []
  panDivs : List PanDivCertMirror := []
  grades : List FirstClassGradeRecordMirror := []
  candidateLegs : List CandidateObservationMirror := []
deriving DecidableEq, Repr

structure FourCacheMirror where
  points : List BspPointMirror
  panDivs : List PanDivCertMirror
  grades : List FirstClassGradeRecordMirror
  candidateLegs : List CandidateObservationMirror
  cachedCount : Nat
deriving DecidableEq, Repr

def emptyFourCache : FourCacheMirror :=
  { points := [], panDivs := [], grades := [], candidateLegs := [], cachedCount := 0 }

def appendEmission (cache : FourCacheMirror) (row : ScanRowEmission) : FourCacheMirror :=
  { points := cache.points ++ row.points
    panDivs := cache.panDivs ++ row.panDivs
    grades := cache.grades ++ row.grades
    candidateLegs := cache.candidateLegs ++ row.candidateLegs
    cachedCount := cache.cachedCount }

def appendRows (cache : FourCacheMirror) (rows : List ScanRowEmission) : FourCacheMirror :=
  rows.foldl appendEmission cache

def flattenRows (rows : List ScanRowEmission) : ScanRowEmission :=
  let cache := appendRows emptyFourCache rows
  { points := cache.points, panDivs := cache.panDivs, grades := cache.grades
    candidateLegs := cache.candidateLegs }

/-- Prefix contraction clears all four caches and their one shared frontier. -/
def resetOvershotCache (cache : FourCacheMirror) (stableSeg : Nat) : FourCacheMirror :=
  if stableSeg < cache.cachedCount then emptyFourCache else cache

/-- Newly confirmed rows advance every cache under the same `cachedCount`. -/
def advanceConfirmedPrefix
    (cache : FourCacheMirror) (stableSeg : Nat) (rows : List ScanRowEmission) : FourCacheMirror :=
  let base := resetOvershotCache cache stableSeg
  let fresh := (rows.drop base.cachedCount).take (stableSeg - base.cachedCount)
  { appendRows base fresh with cachedCount := stableSeg }

def freezeBoundarySrc (centers : List CenterFrame) (prefixCount dirtyE : Nat) : Nat :=
  if prefixCount < 2 then 0
  else match centers[prefixCount - 2]? with
    | none => 0
    | some center => min center.endIndex dirtyE

def stableSegmentCount (rows : List SegmentRow) (eSrc : Nat) : Nat :=
  (List.takeWhile (fun row : SegmentRow => decide (row.endIndex < eSrc)) rows).length

def stableInsertBySource (source : alpha -> Nat) (value : alpha) : List alpha -> List alpha
  | [] => [value]
  | head :: tail =>
      if source value < source head then value :: head :: tail
      else head :: stableInsertBySource source value tail

/-- Rust stable `sort_by_key(source_index)`: equal keys retain push order. -/
def stableSortBySource (source : alpha -> Nat) (values : List alpha) : List alpha :=
  values.foldl (fun sorted value => stableInsertBySource source value sorted) []

/--
P-10 driver. Confirmed rows advance the four caches once; frontier tail rows are read from the
current input and rejudged on every call; only the returned view is canonically sorted/reduced.
-/
def mergedScanMirror
    (level stableSeg : Nat) (cache : FourCacheMirror) (rows : List ScanRowEmission) :
    FourCacheMirror × MergedScanOutputMirror :=
  let advanced := advanceConfirmedPrefix cache stableSeg rows
  let tail := flattenRows (rows.drop stableSeg)
  let points := stableSortBySource BspPointMirror.sourceIndex (advanced.points ++ tail.points)
  let panDivs := stableSortBySource PanDivCertMirror.sourceIndex (advanced.panDivs ++ tail.panDivs)
  let grades := stableSortBySource FirstClassGradeRecordMirror.sourceIndex
    (advanced.grades ++ tail.grades)
  let legs := advanced.candidateLegs ++ tail.candidateLegs
  let observations := canonicalSortCandidates
    (reduceStructuralLegs legs ++ panDivs.map (makePanObservation level))
  let output : MergedScanOutputMirror :=
    { points := points
      panDivs := panDivs
      grades := grades
      observations := observations }
  (advanced, output)

def mergedScanMirrorRaw
    (level prefixCount dirtyE : Nat) (centers : List CenterFrame) (segments : List SegmentRow)
    (cache : FourCacheMirror) (judgedRows : List ScanRowEmission) :
    Nat × FourCacheMirror × MergedScanOutputMirror :=
  let eSrc := freezeBoundarySrc centers prefixCount dirtyE
  let stableSeg := stableSegmentCount segments eSrc
  let result := mergedScanMirror level stableSeg cache judgedRows
  (stableSeg, result.1, result.2)

def optionList : Option alpha -> List alpha
  | none => []
  | some value => [value]

def emptyScanRow : ScanRowEmission :=
  { points := [], panDivs := [], grades := [], candidateLegs := [] }

def resolveRawAWithCache (entries : List ACacheEntry) (rows : List SegmentRow)
    (centers : List CenterFrame) (centerIndex : Nat) (trend : Direction) :
    Option ASegment × List ACacheEntry :=
  match lookupACache entries centerIndex with
  | some cached => (cached, entries)
  | none =>
      let fresh := match centers[centerIndex - 1]?, centers[centerIndex]? with
        | some previous, some current => locateDepartureMoveA rows previous current trend
        | _, _ => none
      (fresh, { centerIndex := centerIndex, value := fresh } :: entries)

def judgeRawSegmentAt (raw : MergedScanResumeRaw) (series : MergedScanRawSeries)
    (rows : List SegmentRow) (index : Nat) (aCache : List ACacheEntry) :
    List ACacheEntry × ScanRowEmission :=
  match rows[index]? with
  | none => (aCache, emptyScanRow)
  | some segment =>
      match nearestConfirmedCenterIdx raw.centers segment.startIndex with
      | none => (aCache, emptyScanRow)
      | some centerIndex =>
          match raw.centers[centerIndex]? with
          | none => (aCache, emptyScanRow)
          | some center =>
              let trendResult := match resolveTrendGate raw.blocks centerIndex with
                | none => (aCache, ([] : List BspPointMirror),
                    ([] : List FirstClassGradeRecordMirror),
                    ([] : List CandidateObservationMirror))
                | some trend =>
                    let resolvedA := resolveRawAWithCache aCache rows raw.centers centerIndex trend
                    let nextCache := resolvedA.2
                    match resolveFirstGateRaw rows series.closeSrc center trend segment resolvedA.1 with
                    | none => (nextCache, [], [], [])
                    | some resolved =>
                        let first := judgeFirstRaw raw.level rows series center trend segment resolved
                        let legs := match raw.centers[centerIndex - 1]? with
                          | none => []
                          | some previous =>
                              [makeTrendObservation raw.level previous center segment resolved.gates]
                        (nextCache, optionList first.point, optionList first.grade, legs)
              let panDivs := if resolveConsolidationLiftZero raw.blocks centerIndex then
                optionList (judgePanRaw rows series center segment) else []
              let thirdPoints := if index = 0 then [] else
                match rows[index - 1]? with
                | none => []
                | some leave =>
                    match nearestConfirmedCenterIdx raw.centers leave.startIndex with
                    | none => []
                    | some leaveCenterIndex =>
                        match raw.centers[leaveCenterIndex]? with
                        | none => []
                        | some leaveCenter =>
                            let firstPair := index = 1 || match rows[index - 2]? with
                              | none => true
                              | some before => before.startIndex < leaveCenter.endIndex
                            if firstPair then optionList (judgeThirdPoint
                              { center := leaveCenter, leave := leave, retest := segment }) else []
              (trendResult.1,
                { points := trendResult.2.1 ++ thirdPoints
                  panDivs := panDivs
                  grades := trendResult.2.2.1
                  candidateLegs := trendResult.2.2.2 })

def judgeRawSegments (raw : MergedScanResumeRaw) (series : MergedScanRawSeries)
    (rows : List SegmentRow) : List ScanRowEmission :=
  ((List.range rows.length).foldl (fun state index =>
    let next := judgeRawSegmentAt raw series rows index state.1
    (next.1, state.2 ++ [next.2])) (([] : List ACacheEntry), ([] : List ScanRowEmission))).2

structure MergedScanResumeResultMirror where
  stableSeg : Nat
  confirmedAppend : ScanRowEmission
  cacheAfter : FourCacheMirror
  tail : ScanRowEmission
  output : MergedScanOutputMirror
deriving DecidableEq, Repr

/-- Full raw-input P-01--P-10 mirror. Invalid release-contract inputs are rejected as `none`. -/
def mergedScanResumeMirror (series : MergedScanRawSeries) (raw : MergedScanResumeRaw)
    (cache : FourCacheMirror) : Option MergedScanResumeResultMirror := do
  let rows ← materializeResumeSegments raw
  if !orderedResumeGuard rows raw.centers then none
  else
    let judged := judgeRawSegments raw series rows
    let eSrc := freezeBoundarySrc raw.centers raw.prefixCount raw.dirtyE
    let stableSeg := stableSegmentCount rows eSrc
    let base := resetOvershotCache cache stableSeg
    let fresh := (judged.drop base.cachedCount).take (stableSeg - base.cachedCount)
    let confirmedAppend := flattenRows fresh
    let merged := mergedScanMirror raw.level stableSeg cache judged
    let tail := flattenRows (judged.drop stableSeg)
    let result : MergedScanResumeResultMirror :=
      { stableSeg := stableSeg
        confirmedAppend := confirmedAppend
        cacheAfter := merged.1
        tail := tail
        output := merged.2 }
    pure result

def rawThirdSeriesWitness : MergedScanRawSeries :=
  { histBits := [], difBits := [], closesTicks := [], closeSrc := []
    gauge := .forceL, strokes := [] }

def rawThirdCenterWitness : CenterFrame :=
  { startIndex := 1, endIndex := 5, zd := 10, zg := 20 }

def rawThirdLeaveWitness : RawSegmentFrame :=
  { direction := .up, startIndex := 6, endIndex := 7, startPrice := 20, endPrice := 30 }

def rawThirdRetestWitness : RawSegmentFrame :=
  { direction := .down, startIndex := 8, endIndex := 9, startPrice := 30, endPrice := 25 }

def forceLFirstStrokeWitness : RawStrokeFrame :=
  { direction := .up, startIndex := 0, endIndex := 1, startPrice := 0, endPrice := 4 }

def forceLLastStrokeWitness : RawStrokeFrame :=
  { direction := .down, startIndex := 2, endIndex := 4, startPrice := 4, endPrice := 1 }

def forceLStrokeWitnesses : List RawStrokeFrame :=
  [forceLFirstStrokeWitness, forceLLastStrokeWitness]

def rawThirdResumeWitness : MergedScanResumeRaw :=
  { level := 0
    centers := [rawThirdCenterWitness]
    segments := [rawThirdLeaveWitness, rawThirdRetestWitness]
    anchorDirs := none
    departureEnds := none
    blocks := []
    prefixCount := 0
    dirtyE := 0 }

def p10FirstSeriesWitness : MergedScanRawSeries :=
  { histBits :=
      [13835058055282163712, 13835058055282163712, 0, 0,
       13821547256400052224, 13821547256400052224, 13821547256400052224,
       13821547256400052224]
    difBits := [0, 0, 0, 0, 0, 0, 0, 0]
    closesTicks := [100, 75, 75, 80, 100, 50, 50, 55]
    closeSrc := [5, 6, 7, 8, 10, 11, 12, 13]
    gauge := .macdArea
    strokes := [] }

def p10FirstResumeWitness : MergedScanResumeRaw :=
  { level := 0
    centers :=
      [{ startIndex := 0, endIndex := 4, zd := 90, zg := 110, dd := 90, gg := 110 },
       { startIndex := 5, endIndex := 9, zd := 60, zg := 80, dd := 60, gg := 80 },
       { startIndex := 14, endIndex := 14, zd := 30, zg := 50, dd := 30, gg := 50 },
       { startIndex := 15, endIndex := 20, zd := 10, zg := 25, dd := 10, gg := 25 }]
    segments :=
      [{ direction := .down, startIndex := 5, endIndex := 6,
         startPrice := 100, endPrice := 75 },
       { direction := .up, startIndex := 7, endIndex := 8,
         startPrice := 75, endPrice := 80 },
       { direction := .down, startIndex := 10, endIndex := 11,
         startPrice := 100, endPrice := 50 },
       { direction := .up, startIndex := 12, endIndex := 13,
         startPrice := 50, endPrice := 55 }]
    anchorDirs := none
    departureEnds := none
    blocks :=
      [{ startCenter := 0, endCenter := 3, kind := .trend,
         direction := some .down, levelLift := 0 }]
    prefixCount := 4
    dirtyE := 14 }

def p10FirstTailResumeWitness : MergedScanResumeRaw :=
  { p10FirstResumeWitness with prefixCount := 0, dirtyE := 0 }

def p10PanSeriesWitness : MergedScanRawSeries :=
  { histBits :=
      [13835058055282163712, 13835058055282163712, 13835058055282163712,
       13821547256400052224, 13821547256400052224]
    difBits := [0, 0, 0, 0, 0]
    closesTicks := [20, 15, 5, 12, 5]
    closeSrc := [1, 2, 3, 6, 7]
    gauge := .macdArea
    strokes := [] }

def p10PanResumeWitness : MergedScanResumeRaw :=
  { level := 0
    centers := [{ startIndex := 4, endIndex := 5, zd := 10, zg := 20, dd := 5, gg := 25 }]
    segments :=
      [{ direction := .down, startIndex := 1, endIndex := 3,
         startPrice := 20, endPrice := 5 },
       { direction := .down, startIndex := 6, endIndex := 7,
         startPrice := 12, endPrice := 5 }]
    anchorDirs := none
    departureEnds := none
    blocks :=
      [{ startCenter := 0, endCenter := 0, kind := .consolidation,
         direction := none, levelLift := 0 }]
    prefixCount := 0
    dirtyE := 0 }

theorem source_map_gap_witness :
    mapSrcRangeToCloseIdx [0, 2, 5, 9] 3 6 = some { left := 2, right := 2 } := by
  decide

theorem force_raw_float_order_witness :
    forceFeatureFromSeries
      [4607182418800017408, 13830554455654793216, 4611686018427387904]
      [4607182418800017408, 4613937818241073152, 4611686018427387904]
      [10, 16] { left := 0, right := 1 } .up =
      { macdAreaBits := 4607182418800017408
        difPeakBits := 4613937818241073152
        priceAmplitude := 6
        priceSpeedBits := 4618441417868443648 } := by
  native_decide

theorem force_l_overlap_witness :
    (segmentForceLRaw forceLStrokeWitnesses
      { left := 0, right := 4 }).map (fun value => value.toBits.toNat) =
      some 13830554455654793216 := by
  native_decide

theorem stable_source_sort_keeps_equal_order :
    stableSortBySource (fun value : Nat × Nat => value.1) [(2, 10), (1, 20), (2, 30)] =
      [(1, 20), (2, 10), (2, 30)] := by
  decide

theorem raw_resume_third_smoke :
    (mergedScanResumeMirror rawThirdSeriesWitness rawThirdResumeWitness emptyFourCache).map
      (fun result => result.output.points.map BspPointMirror.sourceIndex) = some [9] := by
  native_decide

theorem p10_first_witness_reaches_confirmed_three_ports :
    (mergedScanResumeMirror p10FirstSeriesWitness p10FirstResumeWitness emptyFourCache).map
      (fun result =>
        (!result.confirmedAppend.points.isEmpty,
         !result.confirmedAppend.grades.isEmpty,
         !result.confirmedAppend.candidateLegs.isEmpty,
         result.tail == emptyScanRow)) = some (true, true, true, true) := by
  native_decide

theorem p10_first_tail_witness_reaches_tail_grade_candidate :
    (mergedScanResumeMirror p10FirstSeriesWitness p10FirstTailResumeWitness emptyFourCache).map
      (fun result =>
        (result.stableSeg, !result.tail.grades.isEmpty,
         !result.tail.candidateLegs.isEmpty, !result.output.grades.isEmpty,
         !result.output.observations.isEmpty)) = some (0, true, true, true, true) := by
  native_decide

theorem p10_pan_witness_reaches_tail :
    (mergedScanResumeMirror p10PanSeriesWitness p10PanResumeWitness emptyFourCache).map
      (fun result =>
        (result.stableSeg, !result.tail.panDivs.isEmpty,
         !result.output.panDivs.isEmpty)) = some (0, true, true) := by
  native_decide

theorem p10_third_witness_reaches_tail :
    (mergedScanResumeMirror rawThirdSeriesWitness rawThirdResumeWitness emptyFourCache).map
      (fun result =>
        (result.stableSeg, !result.tail.points.isEmpty,
         !result.output.points.isEmpty)) = some (0, true, true) := by
  native_decide

theorem freeze_boundary_prefix_lt_two_is_zero
    (centers : List CenterFrame) (prefixCount dirtyE : Nat) (h : prefixCount < 2) :
    freezeBoundarySrc centers prefixCount dirtyE = 0 := by
  simp [freezeBoundarySrc, h]

/-- Extraction adapter: Rust captures the newly-confirmed and tail material as two aggregates. -/
def mergedScanMirrorAggregated
    (level stableSeg : Nat) (cache : FourCacheMirror)
    (confirmedAppend tail : ScanRowEmission) : FourCacheMirror × MergedScanOutputMirror :=
  let base := resetOvershotCache cache stableSeg
  let advanced : FourCacheMirror :=
    { points := base.points ++ confirmedAppend.points
      panDivs := base.panDivs ++ confirmedAppend.panDivs
      grades := base.grades ++ confirmedAppend.grades
      candidateLegs := base.candidateLegs ++ confirmedAppend.candidateLegs
      cachedCount := stableSeg }
  let points := stableSortBySource BspPointMirror.sourceIndex (advanced.points ++ tail.points)
  let panDivs := stableSortBySource PanDivCertMirror.sourceIndex (advanced.panDivs ++ tail.panDivs)
  let grades := stableSortBySource FirstClassGradeRecordMirror.sourceIndex
    (advanced.grades ++ tail.grades)
  let observations := canonicalSortCandidates
    (reduceStructuralLegs (advanced.candidateLegs ++ tail.candidateLegs) ++
      panDivs.map (makePanObservation level))
  let output : MergedScanOutputMirror :=
    { points := points
      panDivs := panDivs
      grades := grades
      observations := observations }
  (advanced, output)

theorem p10_aggregate_adapter_keeps_shared_frontier
    (level stableSeg : Nat) (cache : FourCacheMirror) (confirmed tail : ScanRowEmission) :
    (mergedScanMirrorAggregated level stableSeg cache confirmed tail).1.cachedCount = stableSeg := by
  rfl

theorem p10_prefix_contraction_clears_all_four_caches
    (cache : FourCacheMirror) (stableSeg : Nat) (h : stableSeg < cache.cachedCount) :
    resetOvershotCache cache stableSeg = emptyFourCache := by
  simp [resetOvershotCache, h]

theorem p10_returned_cache_is_exactly_confirmed_prefix
    (level stableSeg : Nat) (cache : FourCacheMirror) (rows : List ScanRowEmission) :
    (mergedScanMirror level stableSeg cache rows).1 =
      advanceConfirmedPrefix cache stableSeg rows := by
  rfl

/-! ## 3. c_p ownership 生命周期 -/

structure ElementIdMirror where
  level : Nat
  ordinal : Nat
deriving DecidableEq, Repr

structure CpStructureIdentityMirror where
  level : Nat
  bCenterId : ElementIdMirror
  departureMoveId : ElementIdMirror
  terminalMoveId : Option ElementIdMirror
  sourceStart : Nat
  sourceEnd : Option Nat
deriving DecidableEq, Repr

structure ThirdClassInCpMirror where
  bCenterId : ElementIdMirror
  cpDepartureMoveId : ElementIdMirror
  departureMoveId : ElementIdMirror
  retestMoveId : ElementIdMirror
  departureInterval : Interval
  retestInterval : Interval
  pointSourceIndex : Nat
  side : Side
deriving DecidableEq, Repr

structure TrendContextMirror where
  predecessorCenterId : ElementIdMirror
  bCenterId : ElementIdMirror
  direction : Direction
deriving DecidableEq, Repr

structure NewExtremeInDirectionMirror where
  bCenterId : ElementIdMirror
  direction : Direction
  referencePrice : Int
  extremePrice : Int
  extremeMoveId : ElementIdMirror
  confirmSrc : Nat
deriving DecidableEq, Repr

structure InternalSublevelCentersMirror where
  cLevel : Nat
  centerIds : List ElementIdMirror
deriving DecidableEq, Repr

structure CompletedTrendDecompositionMirror where
  direction : Direction
  centerIds : List ElementIdMirror
  closingSuccessorMoveId : ElementIdMirror
  confirmSrc : Nat
deriving DecidableEq, Repr

structure FullTrendQualificationEvidenceMirror where
  trendContext : Option TrendContextMirror
  newExtremeInDirection : Option NewExtremeInDirectionMirror
  internalSublevelCenters : Option InternalSublevelCentersMirror
  completedTrendDecomposition : Option CompletedTrendDecompositionMirror
  decompositionReviewMoveId : Option ElementIdMirror
  decompositionReviewSrc : Option Nat
deriving DecidableEq, Repr

structure FullTrendCQualifiedMirror where
  trendContext : TrendContextMirror
  thirdClassInsideC : ThirdClassInCpMirror
  newExtremeInDirection : NewExtremeInDirectionMirror
  internalSublevelCenters : InternalSublevelCentersMirror
  completedTrendDecomposition : CompletedTrendDecompositionMirror
  confirmSrc : Nat
deriving DecidableEq, Repr

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

/-- Exact `CpScanOwnership` attachment, including valid open `c_structure` on Pending objects. -/
structure CpOwnershipFullMirror where
  bCenterIndex : Nat
  bCenterId : ElementIdMirror
  bCenter : CenterFrame
  departureMoveId : Option ElementIdMirror
  departureInterval : Option Interval
  lifecycle : CpLifecycle
  cpCertificateConfirmSrc : Option Nat
  cStructure : Option CpStructureIdentityMirror
  thirdClassInC : Option ThirdClassInCpMirror
  fullTrendEvidence : Option FullTrendQualificationEvidenceMirror
  fullTrendCQualified : Option FullTrendCQualifiedMirror
deriving DecidableEq, Repr

/-- Pending→Closed 的原子写入素材；不携带、也不能覆盖 `B_p` 身份字段。 -/
structure CpClosureMirror where
  confirmSrc : Nat
  cStructure : CpStructureIdentityMirror
  thirdClass : ThirdClassInCpMirror
  fullTrendEvidence : Option FullTrendQualificationEvidenceMirror
  fullTrendQualified : Option FullTrendCQualifiedMirror
deriving DecidableEq, Repr

structure CpMoveRawMirror where
  id : ElementIdMirror
  startIndex : Nat
  endIndex : Nat
  lo : Int
  hi : Int
  center : Option CenterFrame
deriving DecidableEq, Repr

structure CpAdvanceRawMirror where
  leave : SegmentRow
  retest : SegmentRow
  leaveMoveId : ElementIdMirror
  retestMoveId : ElementIdMirror
  centers : List CenterFrame
  visibleMoves : List CpMoveRawMirror
deriving DecidableEq, Repr

inductive CenterRelationMirror where
  | upContinuation | downContinuation | levelExpansion | coreOverlap
deriving DecidableEq, Repr

def classifyCenterRelation (previous next : CenterFrame) : CenterRelationMirror :=
  if previous.gg < next.dd then .upContinuation
  else if next.gg < previous.dd then .downContinuation
  else if (next.zg < previous.zd && previous.dd ≤ next.gg) ||
      (previous.zg < next.zd && next.dd ≤ previous.gg) then .levelExpansion
  else .coreOverlap

def CenterRelationMirror.trendDirection : CenterRelationMirror -> Option Direction
  | .upContinuation => some .up
  | .downContinuation => some .down
  | .levelExpansion | .coreOverlap => none

def adjacentAll (values : List alpha) (predicate : alpha -> alpha -> Bool) : Bool :=
  (values.zip values.tail).all fun pair => predicate pair.1 pair.2

def sequenceOptions : List (Option alpha) -> Option (List alpha)
  | [] => some []
  | none :: _ => none
  | some head :: tail => (sequenceOptions tail).map (head :: ·)

def trendContextFromRaw (centers : List CenterFrame) (bCenterIndex : Nat)
    (bCenterId : ElementIdMirror) : Option TrendContextMirror := do
  if bCenterIndex = 0 || bCenterId.ordinal = 0 then none else
  let previous ← centers[bCenterIndex - 1]?
  let current ← centers[bCenterIndex]?
  let direction ← (classifyCenterRelation previous current).trendDirection
  let predecessor : ElementIdMirror :=
    { level := bCenterId.level, ordinal := bCenterId.ordinal - 1 }
  let result : TrendContextMirror :=
    { predecessorCenterId := predecessor, bCenterId := bCenterId, direction := direction }
  pure result

def fullTrendEvidenceFromRaw (centers : List CenterFrame) (bCenterIndex : Nat)
    (bCenterId : ElementIdMirror) (cStructure : CpStructureIdentityMirror)
    (third : ThirdClassInCpMirror) (moves : List CpMoveRawMirror) :
    Option FullTrendQualificationEvidenceMirror := do
  let b ← centers[bCenterIndex]?
  if cStructure.bCenterId != bCenterId || third.bCenterId != bCenterId ||
      third.cpDepartureMoveId != cStructure.departureMoveId then none else
  let terminalId ← cStructure.terminalMoveId
  let start ← moves.findIdx? fun move => move.id == cStructure.departureMoveId
  let finish ← moves.findIdx? fun move => move.id == terminalId
  if finish < start then none else
  let components := (moves.drop start).take (finish - start + 1)
  let first ← components.head?
  let last ← components.getLast?
  let sourceEnd ← cStructure.sourceEnd
  if first.startIndex != cStructure.sourceStart || last.endIndex != sourceEnd ||
      !adjacentAll components (fun left right =>
        left.id.level == right.id.level && left.id.ordinal + 1 == right.id.ordinal) then none else
  let trendContext := trendContextFromRaw centers bCenterIndex bCenterId
  let newExtreme := trendContext.bind fun context =>
    (components.find? fun move => match context.direction with
      | .up => decide (b.gg < move.hi)
      | .down => decide (move.lo < b.dd)).map fun move =>
        { bCenterId := bCenterId, direction := context.direction
          referencePrice := match context.direction with | .up => b.gg | .down => b.dd
          extremePrice := match context.direction with | .up => move.hi | .down => move.lo
          extremeMoveId := move.id, confirmSrc := move.endIndex }
  let componentCenters := sequenceOptions (components.map CpMoveRawMirror.center)
  let internalCenters := componentCenters.bind fun _ =>
    if 2 ≤ components.length then some
      { cLevel := cStructure.level, centerIds := components.map CpMoveRawMirror.id }
    else none
  let allCenters := sequenceOptions (moves.map CpMoveRawMirror.center)
  -- Rust 的 `successor` 是 `unit_moves.get(...).zip(all_internal_centers.get(...))`：
  -- 任一可见 move 不是 Compose 时，review ID/src 也必须保持 None。
  let successorMove := allCenters.bind fun _ => moves[finish + 1]?
  let successorCenter := allCenters.bind fun values => values[finish + 1]?
  let reviewMoveId := successorMove.map CpMoveRawMirror.id
  let reviewSrc := successorMove.map CpMoveRawMirror.endIndex
  let completed := trendContext.bind fun context => internalCenters.bind fun internal =>
    allCenters.bind fun all => successorMove.bind fun successor => successorCenter.bind fun nextCenter =>
      let expected := match context.direction with
        | .up => CenterRelationMirror.upContinuation
        | .down => CenterRelationMirror.downContinuation
      let chain := (all.drop start).take (finish - start + 1)
      let chainOk := adjacentAll chain fun left right => classifyCenterRelation left right == expected
      let startsOk := if start = 0 then true else
        match all[start - 1]?, all[start]? with
        | some left, some right => classifyCenterRelation left right != expected
        | _, _ => false
      let endsOk := match all[finish]? with
        | some terminal => classifyCenterRelation terminal nextCenter != expected
        | none => false
      if chainOk && startsOk && endsOk then some
        { direction := context.direction, centerIds := internal.centerIds
          closingSuccessorMoveId := successor.id, confirmSrc := successor.endIndex }
      else none
  let result : FullTrendQualificationEvidenceMirror :=
    { trendContext := trendContext, newExtremeInDirection := newExtreme
      internalSublevelCenters := internalCenters, completedTrendDecomposition := completed
      decompositionReviewMoveId := reviewMoveId, decompositionReviewSrc := reviewSrc }
  pure result

def fullTrendQualifiedFromEvidence (third : ThirdClassInCpMirror)
    (evidence : FullTrendQualificationEvidenceMirror) : Option FullTrendCQualifiedMirror := do
  let context ← evidence.trendContext
  let extreme ← evidence.newExtremeInDirection
  let internal ← evidence.internalSublevelCenters
  let completed ← evidence.completedTrendDecomposition
  let result : FullTrendCQualifiedMirror :=
    { trendContext := context, thirdClassInsideC := third
      newExtremeInDirection := extreme, internalSublevelCenters := internal
      completedTrendDecomposition := completed
      confirmSrc := max third.pointSourceIndex (max extreme.confirmSrc completed.confirmSrc) }
  pure result

def cpClosureFromRaw (before : CpOwnershipFullMirror)
    (raw : CpAdvanceRawMirror) : Option CpClosureMirror := do
  let cpDepartureMoveId ← before.departureMoveId
  let departureSpan ← before.departureInterval
  if raw.leave.startIndex < departureSpan.left ||
      raw.leaveMoveId.level != cpDepartureMoveId.level ||
      raw.retestMoveId.level != cpDepartureMoveId.level ||
      raw.leaveMoveId.ordinal < cpDepartureMoveId.ordinal ||
      raw.retestMoveId.ordinal < raw.leaveMoveId.ordinal then none else
  let point ← judgeThirdPoint { center := before.bCenter, leave := raw.leave, retest := raw.retest }
  let third : ThirdClassInCpMirror :=
    { bCenterId := before.bCenterId, cpDepartureMoveId := cpDepartureMoveId
      departureMoveId := raw.leaveMoveId, retestMoveId := raw.retestMoveId
      departureInterval := { left := raw.leave.startIndex, right := raw.leave.endIndex }
      retestInterval := { left := raw.retest.startIndex, right := raw.retest.endIndex }
      pointSourceIndex := point.sourceIndex
      side := if point.bits.buy3 then .long else .short }
  let cStructure : CpStructureIdentityMirror :=
    { level := cpDepartureMoveId.level, bCenterId := before.bCenterId
      departureMoveId := cpDepartureMoveId, terminalMoveId := some raw.retestMoveId
      sourceStart := departureSpan.left, sourceEnd := some raw.retest.endIndex }
  let evidence := fullTrendEvidenceFromRaw raw.centers before.bCenterIndex before.bCenterId
    cStructure third raw.visibleMoves
  let result : CpClosureMirror :=
    { confirmSrc := point.sourceIndex, cStructure := cStructure, thirdClass := third
      fullTrendEvidence := evidence
      fullTrendQualified := evidence.bind (fullTrendQualifiedFromEvidence third) }
  pure result

def initCpFull (bCenterId : ElementIdMirror) (center : CenterFrame)
    (departureMoveId : Option ElementIdMirror) (departureInterval : Option Interval) :
    CpOwnershipFullMirror :=
  { bCenterIndex := bCenterId.ordinal, bCenterId := bCenterId, bCenter := center
    departureMoveId := departureMoveId, departureInterval := departureInterval
    lifecycle := .pending, cpCertificateConfirmSrc := none
    cStructure := match departureMoveId, departureInterval with
      | some moveId, some span => some
          { level := moveId.level, bCenterId := bCenterId, departureMoveId := moveId
            terminalMoveId := none, sourceStart := span.left, sourceEnd := none }
      | _, _ => none
    thirdClassInC := none, fullTrendEvidence := none, fullTrendCQualified := none }

def cpFullStableBefore (cp : CpOwnershipFullMirror) (dirtyFrom : Nat) : Bool :=
  if cp.lifecycle = .pending then true else
    let terminalStable := cp.cStructure.bind CpStructureIdentityMirror.terminalMoveId |>.any
      (fun id => id.ordinal < dirtyFrom)
    let reviewStable := cp.fullTrendEvidence.bind
      FullTrendQualificationEvidenceMirror.decompositionReviewMoveId |>.all
      (fun id => id.ordinal < dirtyFrom)
    terminalStable && reviewStable

def clearCpReview (evidence : FullTrendQualificationEvidenceMirror) :
    FullTrendQualificationEvidenceMirror :=
  { trendContext := evidence.trendContext
    newExtremeInDirection := evidence.newExtremeInDirection
    internalSublevelCenters := evidence.internalSublevelCenters
    completedTrendDecomposition := none
    decompositionReviewMoveId := none
    decompositionReviewSrc := none }

def invalidateCpFull (cp : CpOwnershipFullMirror) (dirtyFrom : Nat) : CpOwnershipFullMirror :=
  if cp.lifecycle != .closed || cpFullStableBefore cp dirtyFrom then cp else
    let terminalDirty := cp.cStructure.bind CpStructureIdentityMirror.terminalMoveId |>.all
      (fun id => dirtyFrom <= id.ordinal)
    if terminalDirty then initCpFull cp.bCenterId cp.bCenter cp.departureMoveId cp.departureInterval
    else
      { bCenterIndex := cp.bCenterIndex, bCenterId := cp.bCenterId, bCenter := cp.bCenter
        departureMoveId := cp.departureMoveId, departureInterval := cp.departureInterval
        lifecycle := cp.lifecycle, cpCertificateConfirmSrc := cp.cpCertificateConfirmSrc
        cStructure := cp.cStructure, thirdClassInC := cp.thirdClassInC
        fullTrendEvidence := Option.map clearCpReview cp.fullTrendEvidence
        fullTrendCQualified := none }

def sameCpIdentity (first second : CpOwnershipFullMirror) : Bool :=
  first.bCenterId == second.bCenterId && first.bCenter == second.bCenter &&
  first.departureMoveId == second.departureMoveId &&
  first.departureInterval == second.departureInterval

def reinheritCpFull (rebuilt prior : CpOwnershipFullMirror) (dirtyFrom : Nat) : CpOwnershipFullMirror :=
  if sameCpIdentity rebuilt prior && cpFullStableBefore prior dirtyFrom then prior else rebuilt

def advanceCpFull (before : CpOwnershipFullMirror) (raw : Option CpAdvanceRawMirror) :
    CpOwnershipFullMirror :=
  if before.lifecycle = .closed then before else
    match raw.bind (cpClosureFromRaw before) with
    | none => before
    | some value =>
        { before with
          lifecycle := .closed
          cpCertificateConfirmSrc := some value.confirmSrc
          cStructure := some value.cStructure
          thirdClassInC := some value.thirdClass
          fullTrendEvidence := value.fullTrendEvidence
          fullTrendCQualified := value.fullTrendQualified }

/-- Closed 对象见到 terminal 后继时，独立从当前 raw move 窗重算复核证书。 -/
def reviewCpFull (before : CpOwnershipFullMirror) (centers : List CenterFrame)
    (visibleMoves : List CpMoveRawMirror) : CpOwnershipFullMirror :=
  match before.lifecycle, before.cStructure, before.thirdClassInC with
  | .closed, some cStructure, some third =>
      let evidence := fullTrendEvidenceFromRaw centers before.bCenterIndex before.bCenterId
        cStructure third visibleMoves
      { before with
        fullTrendEvidence := evidence
        fullTrendCQualified := evidence.bind (fullTrendQualifiedFromEvidence third) }
  | _, _, _ => before

theorem cp_pending_can_retain_open_structure
    (ownership : CpOwnershipFullMirror) (openStructure : CpStructureIdentityMirror)
    (hLifecycle : ownership.lifecycle = CpLifecycle.pending)
    (hStructure : ownership.cStructure = some openStructure) :
    ownership.lifecycle = CpLifecycle.pending ∧ ownership.cStructure = some openStructure := by
  exact ⟨hLifecycle, hStructure⟩

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

structure RustCenterExtraction where
  startIndex : Nat
  endIndex : Nat
  zd : Int
  zg : Int
  dd : Int
  gg : Int
deriving DecidableEq, Repr

def RustCenterExtraction.toLean (center : RustCenterExtraction) : CenterFrame :=
  { startIndex := center.startIndex, endIndex := center.endIndex
    zd := center.zd, zg := center.zg, dd := center.dd, gg := center.gg }

structure RustThirdClassEntryExtraction where
  centerSi : Nat
  centerZd : Int
  centerZg : Int
  leaveLeft : Nat
  leaveRight : Nat
  retestLeft : Nat
  retestRight : Nat
deriving DecidableEq, Repr

def RustThirdClassEntryExtraction.toLean
    (entry : RustThirdClassEntryExtraction) : ThirdClassEntryIdentityMirror :=
  { centerSi := entry.centerSi, centerZd := entry.centerZd, centerZg := entry.centerZg
    leaveInterval := { left := entry.leaveLeft, right := entry.leaveRight }
    retestInterval := { left := entry.retestLeft, right := entry.retestRight } }

structure RustBspBitsExtraction where
  buy1 : Bool := false
  buy2 : Bool := false
  buy3 : Bool := false
  sell1 : Bool := false
  sell2 : Bool := false
  sell3 : Bool := false
  thirdClassEntry : Option RustThirdClassEntryExtraction := none
deriving DecidableEq, Repr

def RustBspBitsExtraction.toLean (bits : RustBspBitsExtraction) : BspBitsMirror :=
  { buy1 := bits.buy1, buy2 := bits.buy2, buy3 := bits.buy3
    sell1 := bits.sell1, sell2 := bits.sell2, sell3 := bits.sell3
    thirdClassEntry := bits.thirdClassEntry.map RustThirdClassEntryExtraction.toLean }

inductive RustOwnerRefExtraction where
  | center (value : RustCenterExtraction)
  | type1Anchor (sourceIndex : Nat)
deriving DecidableEq, Repr

def RustOwnerRefExtraction.toLean : RustOwnerRefExtraction -> OwnerRefMirror
  | .center value => .center value.toLean
  | .type1Anchor sourceIndex => .type1Anchor sourceIndex

structure RustForceFeatureBitsExtraction where
  macdAreaBits : Nat
  difPeakBits : Nat
  priceAmplitude : Int
  priceSpeedBits : Nat
deriving DecidableEq, Repr

def RustForceFeatureBitsExtraction.toLean
    (feature : RustForceFeatureBitsExtraction) : ForceFeatureBitsMirror :=
  { macdAreaBits := feature.macdAreaBits, difPeakBits := feature.difPeakBits
    priceAmplitude := feature.priceAmplitude, priceSpeedBits := feature.priceSpeedBits }

structure RustForceProxiesBitsExtraction where
  segA : RustForceFeatureBitsExtraction
  segC : RustForceFeatureBitsExtraction
deriving DecidableEq, Repr

def RustForceProxiesBitsExtraction.toLean
    (force : RustForceProxiesBitsExtraction) : ForceProxiesBitsMirror :=
  { segA := force.segA.toLean, segC := force.segC.toLean }

structure RustBspPointExtraction where
  sourceIndex : Nat
  bits : RustBspBitsExtraction
  pivotLow : Int
  pivotHigh : Int
  owner : Option RustOwnerRefExtraction
  structBreakDir : Option RustSideTag
  force : Option RustForceProxiesBitsExtraction
  retraceBreaksType1 : Option Bool
deriving DecidableEq, Repr

def RustBspPointExtraction.toLean (point : RustBspPointExtraction) : BspPointMirror :=
  { sourceIndex := point.sourceIndex, bits := point.bits.toLean
    pivotLow := point.pivotLow, pivotHigh := point.pivotHigh
    owner := point.owner.map RustOwnerRefExtraction.toLean
    structBreakDir := point.structBreakDir.map RustSideTag.toLean
    force := point.force.map RustForceProxiesBitsExtraction.toLean
    retraceBreaksType1 := point.retraceBreaksType1 }

def BspParity (rust : RustBspPointExtraction) (lean : BspPointMirror) : Prop :=
  rust.sourceIndex = lean.sourceIndex ∧ rust.bits.toLean = lean.bits ∧
  rust.pivotLow = lean.pivotLow ∧ rust.pivotHigh = lean.pivotHigh ∧
  rust.owner.map RustOwnerRefExtraction.toLean = lean.owner ∧
  rust.structBreakDir.map RustSideTag.toLean = lean.structBreakDir ∧
  rust.force.map RustForceProxiesBitsExtraction.toLean = lean.force ∧
  rust.retraceBreaksType1 = lean.retraceBreaksType1

structure RustPanDivExtraction where
  sourceIndex : Nat
  side : RustSideTag
  centerStart : Nat
  centerEnd : Nat
  centerZd : Int
  centerZg : Int
  centerDd : Int
  centerGg : Int
  segALeft : Nat
  segARight : Nat
  segCLeft : Nat
  segCRight : Nat
deriving DecidableEq, Repr

def PanParity (rust : RustPanDivExtraction) (lean : PanDivCertMirror) : Prop :=
  rust.sourceIndex = lean.sourceIndex ∧ rust.side.toLean = lean.side ∧
  rust.centerStart = lean.centerStart ∧ rust.centerEnd = lean.centerEnd ∧
  rust.centerZd = lean.centerZd ∧ rust.centerZg = lean.centerZg ∧
  rust.centerDd = lean.centerDd ∧ rust.centerGg = lean.centerGg ∧
  rust.segALeft = lean.segA.left ∧
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
  ruleVersion : Nat
  level : Nat
  kind : RustCandidateKindTag
  observationKind : RustCandidateKindTag
  side : RustSideTag
  previousCenterStart : Option Nat
  parentCenterStart : Nat
  parentZd : Int
  parentZg : Int
  segALeft : Nat
  segARight : Nat
  lambdaC : Nat
  centerIds : Option (Nat × Nat)
  candidateGroupId : Nat
  pairId : Nat
  direction : Bool
  comparable : Bool
  extreme : Bool
  extremeProofLeft : Nat
  extremeProofRight : Nat
  thirdClassProof : Option Nat
  intervalLeft : Nat
  intervalRight : Nat
  state : RustCandidateStateTag
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def CandidateParity (rust : RustCandidateExtraction) (lean : CandidateObservationMirror) : Prop :=
  rust.ruleVersion = lean.key.ruleVersion ∧ rust.level = lean.key.level ∧
  rust.kind.toLean = lean.key.kind ∧ rust.observationKind.toLean = lean.kind ∧
  rust.side.toLean = lean.key.side ∧ rust.previousCenterStart = lean.key.previousCenterStart ∧
  rust.parentCenterStart = lean.key.parentCenterStart ∧ rust.parentZd = lean.key.parentZd ∧
  rust.parentZg = lean.key.parentZg ∧ rust.segALeft = lean.key.segA.left ∧
  rust.segARight = lean.key.segA.right ∧ rust.lambdaC = lean.key.lambdaC ∧
  rust.centerIds = lean.centerIds ∧ rust.candidateGroupId = lean.candidateGroupId ∧
  rust.pairId = lean.pairId ∧
  rust.direction = lean.predicates.direction ∧ rust.comparable = lean.predicates.comparable ∧
  rust.extreme = lean.predicates.extreme ∧ rust.extremeProofLeft = lean.extremeProof.left ∧
  rust.extremeProofRight = lean.extremeProof.right ∧ rust.thirdClassProof = lean.thirdClassProof ∧
  rust.intervalLeft = lean.interval.left ∧
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

structure RustElementIdExtraction where
  level : Nat
  ordinal : Nat
deriving DecidableEq, Repr

def RustElementIdExtraction.toLean (id : RustElementIdExtraction) : ElementIdMirror :=
  { level := id.level, ordinal := id.ordinal }

structure RustCpStructureIdentityExtraction where
  level : Nat
  bCenterId : RustElementIdExtraction
  departureMoveId : RustElementIdExtraction
  terminalMoveId : Option RustElementIdExtraction
  sourceStart : Nat
  sourceEnd : Option Nat
deriving DecidableEq, Repr

def RustCpStructureIdentityExtraction.toLean
    (value : RustCpStructureIdentityExtraction) : CpStructureIdentityMirror :=
  { level := value.level, bCenterId := value.bCenterId.toLean
    departureMoveId := value.departureMoveId.toLean
    terminalMoveId := value.terminalMoveId.map RustElementIdExtraction.toLean
    sourceStart := value.sourceStart, sourceEnd := value.sourceEnd }

structure RustThirdClassInCpExtraction where
  bCenterId : RustElementIdExtraction
  cpDepartureMoveId : RustElementIdExtraction
  departureMoveId : RustElementIdExtraction
  retestMoveId : RustElementIdExtraction
  departureInterval : Interval
  retestInterval : Interval
  pointSourceIndex : Nat
  side : RustSideTag
deriving DecidableEq, Repr

def RustThirdClassInCpExtraction.toLean
    (value : RustThirdClassInCpExtraction) : ThirdClassInCpMirror :=
  { bCenterId := value.bCenterId.toLean
    cpDepartureMoveId := value.cpDepartureMoveId.toLean
    departureMoveId := value.departureMoveId.toLean
    retestMoveId := value.retestMoveId.toLean
    departureInterval := value.departureInterval, retestInterval := value.retestInterval
    pointSourceIndex := value.pointSourceIndex, side := value.side.toLean }

structure RustTrendContextExtraction where
  predecessorCenterId : RustElementIdExtraction
  bCenterId : RustElementIdExtraction
  direction : RustDirectionTag
deriving DecidableEq, Repr

def RustTrendContextExtraction.toLean (x : RustTrendContextExtraction) : TrendContextMirror :=
  { predecessorCenterId := x.predecessorCenterId.toLean, bCenterId := x.bCenterId.toLean
    direction := x.direction.toLean }

structure RustNewExtremeExtraction where
  bCenterId : RustElementIdExtraction
  direction : RustDirectionTag
  referencePrice : Int
  extremePrice : Int
  extremeMoveId : RustElementIdExtraction
  confirmSrc : Nat
deriving DecidableEq, Repr

def RustNewExtremeExtraction.toLean (x : RustNewExtremeExtraction) : NewExtremeInDirectionMirror :=
  { bCenterId := x.bCenterId.toLean, direction := x.direction.toLean
    referencePrice := x.referencePrice, extremePrice := x.extremePrice
    extremeMoveId := x.extremeMoveId.toLean, confirmSrc := x.confirmSrc }

structure RustInternalCentersExtraction where
  cLevel : Nat
  centerIds : List RustElementIdExtraction
deriving DecidableEq, Repr

def RustInternalCentersExtraction.toLean (x : RustInternalCentersExtraction) : InternalSublevelCentersMirror :=
  { cLevel := x.cLevel, centerIds := x.centerIds.map RustElementIdExtraction.toLean }

structure RustCompletedTrendExtraction where
  direction : RustDirectionTag
  centerIds : List RustElementIdExtraction
  closingSuccessorMoveId : RustElementIdExtraction
  confirmSrc : Nat
deriving DecidableEq, Repr

def RustCompletedTrendExtraction.toLean (x : RustCompletedTrendExtraction) : CompletedTrendDecompositionMirror :=
  { direction := x.direction.toLean, centerIds := x.centerIds.map RustElementIdExtraction.toLean
    closingSuccessorMoveId := x.closingSuccessorMoveId.toLean, confirmSrc := x.confirmSrc }

structure RustFullTrendEvidenceExtraction where
  trendContext : Option RustTrendContextExtraction
  newExtremeInDirection : Option RustNewExtremeExtraction
  internalSublevelCenters : Option RustInternalCentersExtraction
  completedTrendDecomposition : Option RustCompletedTrendExtraction
  decompositionReviewMoveId : Option RustElementIdExtraction
  decompositionReviewSrc : Option Nat
deriving DecidableEq, Repr

def RustFullTrendEvidenceExtraction.toLean (x : RustFullTrendEvidenceExtraction) : FullTrendQualificationEvidenceMirror :=
  { trendContext := x.trendContext.map RustTrendContextExtraction.toLean
    newExtremeInDirection := x.newExtremeInDirection.map RustNewExtremeExtraction.toLean
    internalSublevelCenters := x.internalSublevelCenters.map RustInternalCentersExtraction.toLean
    completedTrendDecomposition := x.completedTrendDecomposition.map RustCompletedTrendExtraction.toLean
    decompositionReviewMoveId := x.decompositionReviewMoveId.map RustElementIdExtraction.toLean
    decompositionReviewSrc := x.decompositionReviewSrc }

structure RustFullTrendQualifiedExtraction where
  trendContext : RustTrendContextExtraction
  thirdClassInsideC : RustThirdClassInCpExtraction
  newExtremeInDirection : RustNewExtremeExtraction
  internalSublevelCenters : RustInternalCentersExtraction
  completedTrendDecomposition : RustCompletedTrendExtraction
  confirmSrc : Nat
deriving DecidableEq, Repr

def RustFullTrendQualifiedExtraction.toLean (x : RustFullTrendQualifiedExtraction) : FullTrendCQualifiedMirror :=
  { trendContext := x.trendContext.toLean, thirdClassInsideC := x.thirdClassInsideC.toLean
    newExtremeInDirection := x.newExtremeInDirection.toLean
    internalSublevelCenters := x.internalSublevelCenters.toLean
    completedTrendDecomposition := x.completedTrendDecomposition.toLean, confirmSrc := x.confirmSrc }

/-- Independent Rust input wire for the five atomic closure payload fields. -/
structure RustCpClosureExtraction where
  confirmSrc : Nat
  cStructure : RustCpStructureIdentityExtraction
  thirdClass : RustThirdClassInCpExtraction
  fullTrendEvidence : Option RustFullTrendEvidenceExtraction
  fullTrendQualified : Option RustFullTrendQualifiedExtraction
deriving DecidableEq, Repr

def RustCpClosureExtraction.toLean (x : RustCpClosureExtraction) : CpClosureMirror :=
  { confirmSrc := x.confirmSrc, cStructure := x.cStructure.toLean
    thirdClass := x.thirdClass.toLean
    fullTrendEvidence := x.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean
    fullTrendQualified := x.fullTrendQualified.map RustFullTrendQualifiedExtraction.toLean }

/-- Independent Rust wire for every production `CpScanOwnership` top-level field. -/
structure RustCpFullExtraction where
  bCenterIndex : Nat
  bCenterId : RustElementIdExtraction
  bCenter : RustCenterExtraction
  departureMoveId : Option RustElementIdExtraction
  departureInterval : Option Interval
  lifecycle : RustCpLifecycleTag
  cpCertificateConfirmSrc : Option Nat
  cStructure : Option RustCpStructureIdentityExtraction
  thirdClassInC : Option RustThirdClassInCpExtraction
  fullTrendEvidence : Option RustFullTrendEvidenceExtraction
  fullTrendCQualified : Option RustFullTrendQualifiedExtraction
deriving DecidableEq, Repr

def RustCpFullExtraction.toLean (rust : RustCpFullExtraction) : CpOwnershipFullMirror :=
  { bCenterIndex := rust.bCenterIndex, bCenterId := rust.bCenterId.toLean
    bCenter := rust.bCenter.toLean
    departureMoveId := rust.departureMoveId.map RustElementIdExtraction.toLean
    departureInterval := rust.departureInterval, lifecycle := rust.lifecycle.toLean
    cpCertificateConfirmSrc := rust.cpCertificateConfirmSrc
    cStructure := rust.cStructure.map RustCpStructureIdentityExtraction.toLean
    thirdClassInC := rust.thirdClassInC.map RustThirdClassInCpExtraction.toLean
    fullTrendEvidence := rust.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean
    fullTrendCQualified := rust.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean }

def CpFullParity (rust : RustCpFullExtraction) (lean : CpOwnershipFullMirror) : Prop :=
  rust.bCenterIndex = lean.bCenterIndex ∧ rust.bCenterId.toLean = lean.bCenterId ∧
  rust.bCenter.toLean = lean.bCenter ∧
  rust.departureMoveId.map RustElementIdExtraction.toLean = lean.departureMoveId ∧
  rust.departureInterval = lean.departureInterval ∧ rust.lifecycle.toLean = lean.lifecycle ∧
  rust.cpCertificateConfirmSrc = lean.cpCertificateConfirmSrc ∧
  rust.cStructure.map RustCpStructureIdentityExtraction.toLean = lean.cStructure ∧
  rust.thirdClassInC.map RustThirdClassInCpExtraction.toLean = lean.thirdClassInC ∧
  rust.fullTrendEvidence.map RustFullTrendEvidenceExtraction.toLean = lean.fullTrendEvidence ∧
  rust.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean = lean.fullTrendCQualified

def CpFullParityBool (rust : RustCpFullExtraction) (lean : CpOwnershipFullMirror) : Bool :=
  rust.toLean == lean

theorem cp_full_parity_rejects_deep_structure_mismatch
    (rust : RustCpFullExtraction) (lean : CpOwnershipFullMirror)
    (hMismatch : rust.cStructure.map RustCpStructureIdentityExtraction.toLean ≠ lean.cStructure) :
    ¬ CpFullParity rust lean := by
  intro h
  rcases h with ⟨_, _, _, _, _, _, _, hStructure, _⟩
  exact hMismatch hStructure

theorem cp_full_parity_rejects_deep_qualified_mismatch
    (rust : RustCpFullExtraction) (lean : CpOwnershipFullMirror)
    (hMismatch : rust.fullTrendCQualified.map RustFullTrendQualifiedExtraction.toLean ≠
      lean.fullTrendCQualified) :
    ¬ CpFullParity rust lean := by
  intro h
  rcases h with ⟨_, _, _, _, _, _, _, _, _, _, hQualified⟩
  exact hMismatch hQualified

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

def CpFullListParity : List RustCpFullExtraction -> List CpOwnershipFullMirror -> Prop :=
  ListParity CpFullParity

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
      { sourceIndex := 1
        bits := { buy1 := true }
        pivotLow := 1, pivotHigh := 0, owner := none
        structBreakDir := some RustSideTag.short, force := none, retraceBreaksType1 := none }
      { sourceIndex := 1, bits := { buy1 := true }, pivotLow := 1, pivotHigh := 0
        owner := none, structBreakDir := some Side.long, force := none,
        retraceBreaksType1 := none } := by
  rintro ⟨_, _, _, _, _, hSide, _⟩
  cases hSide

end NewChanlun.Origin.UnifiedScanMirrorBridge
