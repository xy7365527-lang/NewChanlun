/-
Origin/ScanAssemblyMirror.lean — 3a 统一扫描装配语义镜像（#1087 第二切片）

本模块只钉生产 Rust 在 main @ 355b839c29 已有的装配语义，不增删任何交易判据，也不参与
生产判定。事实锚均按该提交逐行复核：
* scan.rs:8-28：共享 per-segment 素材、双域 sink、扫描后候选归约与 D2/P1 边界；
* scan.rs:39-49：MergedScanOutput 的四件输出；
* scan.rs:95-125、137-228：结构方向锚、冻结边界、共享逐段扫描、prefix/tail 与最终归约；
* scan.rs:276-390：第一类/盘背/第三类与候选腿的唯一逐段装配核；
* decompose.rs:45-63、198-254：MoveBlock 与三项 pointwise ownership 查询；
* signal.rs:221-228、242-257、266-325：最近 confirmed 中枢与 FirstStructuralGates；
* signal.rs:749-863、1150-1161、1911-1922：grade、PanDivCert 与冻结公式。

★D2 字段族边界（scan.rs:19-28）：CandDeltaEvent 与 cp_ownership 是独立于四件扫描输出的
P1 侧车。本镜像分别钉住其事件装配和 stable-revision 生命周期投影，不从四件输出反推；
pan_div_diag 仍是诊断字段，不进入事件判定或四件输出。

认识论等级 L0：这里只镜像既有确定性装配。lake build 证明 Lean 内部定义与定理成立，不等于
Rust 已完成跨语言对拍，更不证明交易有效性；指定窗签收属于后续执行切片。

本文件从原始 SegmentRow/CenterFrame/MoveBlockFrame 独立重算 prelude 的排序守卫、方向锚、
ownership 趋势门、first_match_idx、A episode+包络缓存与 λ_C episode；从逐段未后处理 sink 独立
重算四输出的稳定排序、候选 FNV 身份、同 key 归约和 Pan 投影。验收桥不提供 Lean 期望输出。
底层线段/中枢构造与 MACD 力度判据仍由 Rust 生产扫描形成逐段 sink，不在 Lean 中重裁交易判据。
-/

namespace NewChanlun.Origin.ScanAssemblyMirror

abbrev Interval := Nat × Nat
abbrev FloatBits := Nat

inductive Direction where
  | up
  | down
deriving DecidableEq, Repr

inductive Side where
  | long
  | short
deriving DecidableEq, Repr

inductive MoveKind where
  | trend
  | consolidation
deriving DecidableEq, Repr

/-- Rust `decompose::MoveBlock` 中 prelude ownership 查询消费的字段。 -/
structure MoveBlockFrame where
  startCenter : Nat
  endCenter : Nat
  kind : MoveKind
  direction : Option Direction
  levelLift : Nat
deriving DecidableEq, Repr

/-- Rust `Center` 的六个可观察字段。 -/
structure CenterFrame where
  zd : Int
  zg : Int
  dd : Int
  gg : Int
  startIndex : Nat
  endIndex : Nat
deriving DecidableEq, Repr

/-- 稳定时间序的一行段素材；生产方向锚由本行 direction 唯一导出。 -/
structure SegmentRow where
  direction : Direction
  startIndex : Nat
  endIndex : Nat
  startPrice : Int
  endPrice : Int
deriving DecidableEq, Repr

/-! ## §1 λ_C episode 定界 -/

def reenters (center : CenterFrame) (departureDir : Direction) (row : SegmentRow) : Prop :=
  row.direction ≠ departureDir ∧
    match departureDir with
    | Direction.down => center.zd ≤ row.endPrice
    | Direction.up => row.endPrice ≤ center.zg

instance (center : CenterFrame) (departureDir : Direction) (row : SegmentRow) :
    Decidable (reenters center departureDir row) := by
  cases departureDir <;> unfold reenters <;> infer_instance

def episodeWindow (rows : List SegmentRow) (center : CenterFrame) (untilStart : Nat) :
    List SegmentRow :=
  rows.filter fun row => decide (center.endIndex ≤ row.startIndex ∧ row.startIndex ≤ untilStart)

def lastReentryBoundary (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) : Nat :=
  rows.foldl
    (fun boundary row => if reenters center departureDir row then row.endIndex else boundary)
    0

/--
scan.rs:95-113 的生产契约把 anchor 固定为 `Some(row.direction)`，所以这里只保留 self-anchor
算法，不再接受一份可与段方向分叉的外部 anchor 列表。
-/
def firstSelfAnchoredStartAfter (rows : List SegmentRow) (departureDir : Direction)
    (boundary : Nat) : Option Nat :=
  match rows with
  | [] => none
  | row :: rest =>
      if row.direction = departureDir ∧ boundary ≤ row.startIndex then
        some row.startIndex
      else
        firstSelfAnchoredStartAfter rest departureDir boundary

def episodeStart (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart : Nat) : Option Nat :=
  let window := episodeWindow rows center untilStart
  firstSelfAnchoredStartAfter window departureDir
    (lastReentryBoundary window center departureDir)

structure EpisodeBounds where
  left : Nat
  right : Nat
deriving DecidableEq, Repr

def episodeBounds (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart triggerEnd : Nat) : Option EpisodeBounds :=
  (episodeStart rows center departureDir untilStart).map fun lambdaC =>
    { left := lambdaC, right := triggerEnd }

def EpisodeDelimitedBy (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart triggerEnd : Nat) (bounds : EpisodeBounds) : Prop :=
  episodeBounds rows center departureDir untilStart triggerEnd = some bounds

/-- 动机：同一段序列、中枢、方向与触发段不能装配出两个 λ_C episode。 -/
theorem episode_boundary_unique
    (rows : List SegmentRow) (center : CenterFrame) (departureDir : Direction)
    (untilStart triggerEnd : Nat) (first second : EpisodeBounds)
    (hFirst : EpisodeDelimitedBy rows center departureDir untilStart triggerEnd first)
    (hSecond : EpisodeDelimitedBy rows center departureDir untilStart triggerEnd second) :
    first = second := by
  unfold EpisodeDelimitedBy at hFirst hSecond
  exact Option.some.inj (hFirst.symm.trans hSecond)

/-! ## §2 cand_delta 事件装配 -/

/-- 生产结构候选门的提取结果；事件装配只消费既有门结果，不重裁判据。 -/
structure CandDeltaFacts where
  accepted : Bool
  buy1 : Bool
  sell1 : Bool
deriving DecidableEq, Repr

def CandDeltaFacts.value (facts : CandDeltaFacts) : Bool :=
  facts.buy1 || facts.sell1

/-- 事件装配输入与当前 self-anchor 段素材共用同一 `SegmentRow`。 -/
structure EventAssemblyInput where
  level : Nat
  side : Side
  divergenceConfirmSrc : Nat
  aInterval : EpisodeBounds
  center : CenterFrame
  departureDir : Direction
  untilStart : Nat
  triggerEnd : Nat
  rows : List SegmentRow
  predicate : CandDeltaFacts
deriving DecidableEq, Repr

/-- 3a/P1 侧车 `CandDeltaEvent` 的装配投影；兼容别名保留以供逐字段桥接。 -/
structure CandDeltaEvent where
  level : Nat
  side : Side
  divergenceConfirmSrc : Nat
  confirmSrc : Nat
  interval : EpisodeBounds
  aInterval : EpisodeBounds
  cEpisodeStart : Nat
  cEpisodeInterval : EpisodeBounds
  enterSrc : Nat
  candDelta : Bool
deriving DecidableEq, Repr

/--
结构候选门拒绝或 self-anchor episode 无来源时不产事件；成功时三个 C episode 别名只由
`episodeBounds` 这一 writer 装配，确认时点保持为独立的 divergence checkpoint。
-/
def assembleCandDelta (input : EventAssemblyInput) : Option CandDeltaEvent :=
  if input.predicate.accepted then
    match episodeBounds input.rows input.center input.departureDir input.untilStart input.triggerEnd with
    | none => none
    | some bounds =>
        some
          { level := input.level
            side := input.side
            divergenceConfirmSrc := input.divergenceConfirmSrc
            confirmSrc := input.divergenceConfirmSrc
            interval := bounds
            aInterval := input.aInterval
            cEpisodeStart := bounds.left
            cEpisodeInterval := bounds
            enterSrc := bounds.left
            candDelta := input.predicate.value }
  else
    none

/-! ## §3 共享 per-segment 中间记录 -/

/-- `ASegCache` 的值：I(A) span 与同一 span 的 b 价格包络。 -/
structure ASegmentEnvelope where
  span : Interval
  bEnvelope : Int × Int
deriving DecidableEq, Repr

/-- `signal::FirstStructuralGates` 的逐字段镜像。 -/
structure FirstStructuralGates where
  side : Side
  segA : Interval
  lambdaC : Nat
  aIdx : Option Interval
  cIdx : Option Interval
  extreme : Bool
deriving DecidableEq, Repr

/--
scan.rs:137-185/307-332 在一次逐段判定内唯一算出的共享素材。`directionAnchor` 在生产路径必须
等于 `some segment.direction`；第三类的 leave 锚从前一条记录的该字段读取（scan.rs:373-383）。
`aSegmentEnvelope` 以 nearestConfirmedCenterIdx 为 cache key；候选归约结果不在本记录中。
-/
structure PerSegmentMaterial where
  segIdx : Nat
  segment : SegmentRow
  nearestConfirmedCenterIdx : Nat
  trendGate : Option (Nat × Direction)
  currentLevelConsolidation : Bool
  aSegmentEnvelope : Option ASegmentEnvelope
  lambdaC : Option Nat
  firstStructuralGates : Option FirstStructuralGates
  directionAnchor : Option Direction
  departureEnd : Option Nat
deriving DecidableEq, Repr

def PerSegmentMaterial.productionAnchorAligned (material : PerSegmentMaterial) : Prop :=
  material.directionAnchor = some material.segment.direction

/-! ## §3a prelude 的独立重算 -/

def segmentRowsSorted : List SegmentRow → Bool
  | [] | [_] => true
  | first :: second :: rest =>
      decide (first.startIndex ≤ second.startIndex) && segmentRowsSorted (second :: rest)

def centerFramesSorted : List CenterFrame → Bool
  | [] | [_] => true
  | first :: second :: rest =>
      decide (first.endIndex < second.endIndex) && centerFramesSorted (second :: rest)

def directionAnchors (rows : List SegmentRow) : List (Option Direction) :=
  rows.map (fun row => some row.direction)

def ownershipBlockAt (blocks : List MoveBlockFrame) (centerIndex : Nat) : Option MoveBlockFrame :=
  if centerIndex = 0 then
    blocks.head?
  else
    blocks.find? fun block => decide (block.startCenter < centerIndex ∧ centerIndex ≤ block.endCenter)

def trendGateAt (blocks : List MoveBlockFrame) (centerIndex : Nat) : Option Direction :=
  if centerIndex = 0 then none else (ownershipBlockAt blocks centerIndex).bind (·.direction)

def trendGates (centerCount : Nat) (blocks : List MoveBlockFrame) : List (Option Direction) :=
  (List.range centerCount).map (trendGateAt blocks)

def sameCenterTriple (left right : CenterFrame) : Bool :=
  decide (left.endIndex = right.endIndex ∧ left.zd = right.zd ∧ left.zg = right.zg)

def firstCenterMatchIndex (centers : List CenterFrame) (target : CenterFrame) : Option Nat :=
  let rec go (index : Nat) : List CenterFrame → Option Nat
    | [] => none
    | current :: rest =>
        if sameCenterTriple current target then some index else go (index + 1) rest
  go 0 centers

def firstMatchIndices (centers : List CenterFrame) : List (Option Nat) :=
  centers.map (firstCenterMatchIndex centers)

def aEpisodeWindow (rows : List SegmentRow) (previous current : CenterFrame) : List SegmentRow :=
  rows.filter fun row =>
    decide (previous.endIndex ≤ row.startIndex ∧ row.startIndex < current.endIndex)

def matchingEpisodeSpan (rows : List SegmentRow) (direction : Direction)
    (lambda : Nat) : Option Interval :=
  rows.foldl
    (fun span row =>
      if row.direction = direction ∧ lambda ≤ row.startIndex then
        match span with
        | none => some (row.startIndex, row.endIndex)
        | some current => some (current.1, row.endIndex)
      else span)
    none

def moveRangeEnvelope (rows : List SegmentRow) (span : Interval) : Option (Int × Int) :=
  rows.foldl
    (fun envelope row =>
      if span.1 ≤ row.startIndex ∧ row.endIndex ≤ span.2 then
        let low := min row.startPrice row.endPrice
        let high := max row.startPrice row.endPrice
        match envelope with
        | none => some (low, high)
        | some current => some (min current.1 low, max current.2 high)
      else envelope)
    none

def locateASegmentEnvelope (rows : List SegmentRow) (previous current : CenterFrame)
    (direction : Direction) : Option ASegmentEnvelope :=
  let window := aEpisodeWindow rows previous current
  let lambda := firstSelfAnchoredStartAfter window direction
    (lastReentryBoundary window previous direction)
  lambda.bind fun start =>
    (matchingEpisodeSpan window direction start).bind fun span =>
      (moveRangeEnvelope rows span).map fun envelope => { span := span, bEnvelope := envelope }

def aSegmentEntries (rows : List SegmentRow) (centers : List CenterFrame)
    (gates : List (Option Direction)) : List (Option ASegmentEnvelope) :=
  let rec go (previous : CenterFrame) : List CenterFrame → List (Option Direction) →
      List (Option ASegmentEnvelope)
    | [], _ => []
    | _ :: rest, [] => none :: go previous rest []
    | current :: rest, gate :: gateRest =>
        let value := gate.bind (locateASegmentEnvelope rows previous current)
        value :: go current rest gateRest
  match centers, gates with
  | [], _ => []
  | _ :: rest, [] => none :: rest.map (fun _ => none)
  | first :: rest, _ :: gateRest => none :: go first rest gateRest

structure PreludeOutput where
  segmentsSorted : Bool
  centersSorted : Bool
  anchors : List (Option Direction)
  trendGate : List (Option Direction)
  firstMatchIdx : List (Option Nat)
  aSegments : List (Option ASegmentEnvelope)
deriving DecidableEq, Repr

def recomputePrelude (rows : List SegmentRow) (centers : List CenterFrame)
    (blocks : List MoveBlockFrame) : PreludeOutput :=
  let gates := trendGates centers.length blocks
  { segmentsSorted := segmentRowsSorted rows
    centersSorted := centerFramesSorted centers
    anchors := directionAnchors rows
    trendGate := gates
    firstMatchIdx := firstMatchIndices centers
    aSegments := aSegmentEntries rows centers gates }

/-! ## §4 四件输出的字段形状 -/

structure ThirdClassEntryIdentity where
  centerSi : Nat
  centerZd : Int
  centerZg : Int
  leaveInterval : Interval
  retestInterval : Interval
deriving DecidableEq, Repr

structure BspBits where
  buy1 : Bool
  buy2 : Bool
  buy3 : Bool
  sell1 : Bool
  sell2 : Bool
  sell3 : Bool
  thirdClassEntry : Option ThirdClassEntryIdentity
deriving DecidableEq, Repr

inductive OwnerRef where
  | center (value : CenterFrame)
  | type1Anchor (sourceIndex : Nat)
deriving DecidableEq, Repr

/-- f64 经桥以 IEEE-754 原始位模式提取，避免十进制往返改变 payload。 -/
structure ForceFeatures where
  macdAreaBits : FloatBits
  difPeakBits : FloatBits
  priceAmplitude : Int
  priceSpeedBits : FloatBits
deriving DecidableEq, Repr

structure ForceProxies where
  segA : ForceFeatures
  segC : ForceFeatures
deriving DecidableEq, Repr

structure BspPoint where
  sourceIndex : Nat
  bits : BspBits
  pivotLow : Int
  pivotHigh : Int
  center : Option OwnerRef
  structBreakDir : Option Side
  force : Option ForceProxies
  retraceBreaksType1 : Option Bool
deriving DecidableEq, Repr

structure PanDivCert where
  sourceIndex : Nat
  side : Side
  center : CenterFrame
  segA : Interval
  segC : Interval
deriving DecidableEq, Repr

inductive T3InCGradeReason where
  | missingLeave
  | missingRetest
  | sameDirection
  | leaveNotOutside
  | retestReentered
deriving DecidableEq, Repr

inductive T3InCGrade where
  | present (leaveInterval retestInterval : Interval)
  | missing (reason : T3InCGradeReason)
deriving DecidableEq, Repr

structure FirstClassGradeRecord where
  level : Nat
  sourceIndex : Nat
  side : Side
  centerStartIndex : Nat
  centerEndIndex : Nat
  centerZd : Int
  centerZg : Int
  grade : T3InCGrade
deriving DecidableEq, Repr

inductive CandidateKind where
  | trend
  | pan
deriving DecidableEq, Repr

structure ParentFingerprint where
  centerStart : Nat
  zd : Int
  zg : Int
deriving DecidableEq, Repr

structure CandidateKey where
  ruleVersion : Nat
  level : Nat
  kind : CandidateKind
  side : Side
  previousCenterStart : Option Nat
  parent : ParentFingerprint
  segA : Interval
  cStart : Nat
deriving DecidableEq, Repr

structure StructuralPredicates where
  direction : Bool
  comparable : Bool
  extreme : Bool
deriving DecidableEq, Repr

inductive ObservedState where
  | provisional
  | unresolved
  | confirmed
deriving DecidableEq, Repr

structure CandidateObservation where
  key : CandidateKey
  kind : CandidateKind
  centerIds : Option (Nat × Nat)
  candidateGroupId : Nat
  pairId : Nat
  structuralPredicates : StructuralPredicates
  extremeProof : Interval
  thirdClassProof : Option Nat
  interval : Interval
  state : ObservedState
  firstProvableAt : Option Nat
  confirmedAt : Option Nat
deriving DecidableEq, Repr

def StructuralPredicates.resolvedState (predicates : StructuralPredicates) : ObservedState :=
  if predicates.direction && predicates.comparable && predicates.extreme then
    ObservedState.provisional
  else
    ObservedState.unresolved

def firstSome {α : Type} (left right : Option α) : Option α :=
  match left with
  | some value => some value
  | none => right

/-- cand_event/observe.rs:121-146 的同 key 两腿归约。 -/
def mergeCandidateLegs (acc leg : CandidateObservation) : CandidateObservation :=
  let carrier := if leg.interval.2 > acc.interval.2 then leg else acc
  let predicates : StructuralPredicates :=
    { direction := carrier.structuralPredicates.direction
      comparable := carrier.structuralPredicates.comparable
      extreme := acc.structuralPredicates.extreme || leg.structuralPredicates.extreme }
  let state := predicates.resolvedState
  let inheritedClock := firstSome acc.firstProvableAt leg.firstProvableAt
  let firstProvableAt :=
    match state with
    | ObservedState.provisional => firstSome inheritedClock (some carrier.interval.2)
    | _ => inheritedClock
  { carrier with
      structuralPredicates := predicates
      state := state
      firstProvableAt := firstProvableAt }

def upsertCandidateLeg (leg : CandidateObservation) :
    List CandidateObservation → List CandidateObservation
  | [] => [leg]
  | current :: rest =>
      if current.key = leg.key then
        mergeCandidateLegs current leg :: rest
      else
        current :: upsertCandidateLeg leg rest

/-- 同 key 左折叠；最终 `(interval,key)` 排序仍由唯一 post-scan sort 算子执行。 -/
def reduceStructuralLegs (legs : List CandidateObservation) : List CandidateObservation :=
  legs.foldl (fun reduced leg => upsertCandidateLeg leg reduced) []

/-! ## §5 双域 sink 与扫描后归约 -/

/-- 每段共享 material 喂出的两域结果；candidateLegs 是未经同 episode 归约的 Trend 腿。 -/
structure PerSegmentEmission where
  material : PerSegmentMaterial
  bspPoints : List BspPoint
  panDivCerts : List PanDivCert
  firstClassGrades : List FirstClassGradeRecord
  candidateLegs : List CandidateObservation
deriving DecidableEq, Repr

/--
跨语言签收的最小输入。Rust 生产扫描按段暴露四个尚未后处理的 sink；Lean 自己完成排序、
同 key 腿归约、Pan 投影和 FNV 身份。它不是 Rust legacy oracle，也不包含期望的最终输出。
-/
structure ScanSinkEmission where
  bspPoints : List BspPoint
  panDivCerts : List PanDivCert
  firstClassGrades : List FirstClassGradeRecord
  candidateLegs : List CandidateObservation
deriving DecidableEq, Repr

structure ScanSinks where
  points : List BspPoint
  panDivs : List PanDivCert
  grades : List FirstClassGradeRecord
  candidateLegs : List CandidateObservation
deriving DecidableEq, Repr

def emptyScanSinks : ScanSinks :=
  { points := [], panDivs := [], grades := [], candidateLegs := [] }

def appendScanSinks (left right : ScanSinks) : ScanSinks :=
  { points := left.points ++ right.points
    panDivs := left.panDivs ++ right.panDivs
    grades := left.grades ++ right.grades
    candidateLegs := left.candidateLegs ++ right.candidateLegs }

/-- 逐段阶段只 append 四个 sink；不在此归约 CandidateObservation。 -/
def collectScanSinks (emissions : List PerSegmentEmission) : ScanSinks :=
  emissions.foldl
    (fun acc emission =>
      { points := acc.points ++ emission.bspPoints
        panDivs := acc.panDivs ++ emission.panDivCerts
        grades := acc.grades ++ emission.firstClassGrades
        candidateLegs := acc.candidateLegs ++ emission.candidateLegs })
    emptyScanSinks

structure MergedScanOutput where
  points : List BspPoint
  panDivs : List PanDivCert
  grades : List FirstClassGradeRecord
  observations : List CandidateObservation
deriving DecidableEq, Repr

/--
生产已有的规范化算子接口。它们分别对应 scan.rs:220-228 的三项 source_index 排序、
reduce_structural_legs、PanDivCert 投影与最终 (interval,key) 排序；本镜像不重裁这些判据。
-/
structure PostScanOperators where
  sortPoints : List BspPoint → List BspPoint
  sortPanDivs : List PanDivCert → List PanDivCert
  sortGrades : List FirstClassGradeRecord → List FirstClassGradeRecord
  candidateGroupId : CandidateKey → Nat
  pairId : CandidateKey → Nat
  sortObservations : List CandidateObservation → List CandidateObservation

/-! ## §5a 生产后处理算子的独立 Lean 实现 -/

/-- 严格比较下的稳定插入排序；相等项后插，保持 Rust `sort_by_key` 的到达顺序。 -/
def orderedInsert {α : Type} (strictlyBefore : α → α → Bool) (value : α) : List α → List α
  | [] => [value]
  | current :: rest =>
      if strictlyBefore value current then
        value :: current :: rest
      else
        current :: orderedInsert strictlyBefore value rest

def stableSortBy {α : Type} (strictlyBefore : α → α → Bool) (values : List α) : List α :=
  values.foldl (fun sorted value => orderedInsert strictlyBefore value sorted) []

def compareThen (first second : Ordering) : Ordering :=
  match first with
  | Ordering.eq => second
  | other => other

def compareOptionNat : Option Nat → Option Nat → Ordering
  | none, none => Ordering.eq
  | none, some _ => Ordering.lt
  | some _, none => Ordering.gt
  | some left, some right => compare left right

def CandidateKind.rank : CandidateKind → Nat
  | CandidateKind.trend => 0
  | CandidateKind.pan => 1

def Side.rank : Side → Nat
  | Side.long => 0
  | Side.short => 1

/-- Rust `#[derive(Ord)] CandidateKey` 的字段序；Int 按有符号顺序比较。 -/
def compareCandidateKey (left right : CandidateKey) : Ordering :=
  compareThen (compare left.ruleVersion right.ruleVersion)
    (compareThen (compare left.level right.level)
      (compareThen (compare left.kind.rank right.kind.rank)
        (compareThen (compare left.side.rank right.side.rank)
          (compareThen (compareOptionNat left.previousCenterStart right.previousCenterStart)
            (compareThen (compare left.parent.centerStart right.parent.centerStart)
              (compareThen (compare left.parent.zd right.parent.zd)
                (compareThen (compare left.parent.zg right.parent.zg)
                  (compareThen (compare left.segA.1 right.segA.1)
                    (compareThen (compare left.segA.2 right.segA.2)
                      (compare left.cStart right.cStart))))))))))

def compareInterval (left right : Interval) : Ordering :=
  compareThen (compare left.1 right.1) (compare left.2 right.2)

def observationStrictlyBefore (left right : CandidateObservation) : Bool :=
  compareThen (compareInterval left.interval right.interval)
    (compareCandidateKey left.key right.key) == Ordering.lt

/-- Rust `u64` wrapping domain represented as a bounded Nat. -/
def u64Modulus : Nat := 18446744073709551616

def fnvPrime : Nat := 1099511628211

def fnvOffsetBasis : Nat := 14695981039346656037

def pairIdSeed : Nat := 9521211207457086692

def intAsU64 (value : Int) : Nat :=
  Int.toNat (value % (Int.ofNat u64Modulus))

def fnvMixByte (hash byte : Nat) : Nat :=
  ((Nat.xor hash byte) * fnvPrime) % u64Modulus

def fnvMixU64 (hash value : Nat) : Nat :=
  (List.range 8).foldl
    (fun acc byteIndex => fnvMixByte acc ((value / (256 ^ byteIndex)) % 256))
    hash

def candidateStableId (key : CandidateKey) (seed : Nat) : Nat :=
  [ key.level,
    key.kind.rank,
    key.side.rank,
    if key.previousCenterStart.isSome then 1 else 0,
    key.previousCenterStart.getD 0,
    key.parent.centerStart,
    intAsU64 key.parent.zd,
    intAsU64 key.parent.zg,
    key.segA.1,
    key.segA.2,
    key.cStart ].foldl fnvMixU64 seed

/-- scan.rs/observe.rs 的排序与 FNV 身份算法；不从 Rust 期望输出注入函数值。 -/
def productionPostScanOperators : PostScanOperators :=
  { sortPoints := stableSortBy (fun left right => left.sourceIndex < right.sourceIndex)
    sortPanDivs := stableSortBy (fun left right => left.sourceIndex < right.sourceIndex)
    sortGrades := stableSortBy (fun left right => left.sourceIndex < right.sourceIndex)
    candidateGroupId := fun key => candidateStableId key fnvOffsetBasis
    pairId := fun key => candidateStableId key pairIdSeed
    sortObservations := stableSortBy observationStrictlyBefore }

def candidateRuleVersion : Nat := 1

/-- cand_event/observe.rs:235-277：从既有 PanDivCert 投影，不重判结构或力度。 -/
def panObservation (operators : PostScanOperators) (level : Nat)
    (cert : PanDivCert) : CandidateObservation :=
  let parent : ParentFingerprint :=
    { centerStart := cert.center.startIndex, zd := cert.center.zd, zg := cert.center.zg }
  let key : CandidateKey :=
    { ruleVersion := candidateRuleVersion, level := level, kind := CandidateKind.pan
      side := cert.side, previousCenterStart := none, parent := parent
      segA := cert.segA, cStart := cert.segC.1 }
  { key := key, kind := CandidateKind.pan, centerIds := none
    candidateGroupId := operators.candidateGroupId key
    pairId := operators.pairId key
    structuralPredicates := { direction := true, comparable := true, extreme := true }
    extremeProof := cert.segA, thirdClassProof := none, interval := cert.segC
    state := ObservedState.confirmed, firstProvableAt := some cert.sourceIndex
    confirmedAt := none }

def finalizeScanSinks (operators : PostScanOperators) (level : Nat)
    (sinks : ScanSinks) : MergedScanOutput :=
  let panDivs := operators.sortPanDivs sinks.panDivs
  let observations :=
    operators.sortObservations
      (reduceStructuralLegs sinks.candidateLegs ++ panDivs.map (panObservation operators level))
  { points := operators.sortPoints sinks.points
    panDivs := panDivs
    grades := operators.sortGrades sinks.grades
    observations := observations }

/--
唯一四件装配：先完整收集逐段 sink，再排序 BSP 三投影；候选腿只在扫描完成后归约，并在归约后
并入从 PanDivCert 投影的观察。prefix cache 与 frontier tail 都先表现为 emissions，故不会出现一套
cache 算法和一套 tail 算法。
-/
def assembleMergedOutput (operators : PostScanOperators) (level : Nat)
    (emissions : List PerSegmentEmission) : MergedScanOutput :=
  finalizeScanSinks operators level (collectScanSinks emissions)

def collectScanSinkEmissions (emissions : List ScanSinkEmission) : ScanSinks :=
  emissions.foldl
    (fun acc emission =>
      { points := acc.points ++ emission.bspPoints
        panDivs := acc.panDivs ++ emission.panDivCerts
        grades := acc.grades ++ emission.firstClassGrades
        candidateLegs := acc.candidateLegs ++ emission.candidateLegs })
    emptyScanSinks

/--
验收入口：只接收逐段、未排序、未归约的 sink 输入；最终四件输出完全由 Lean 镜像重算。
-/
def recomputeMergedOutput (level : Nat) (emissions : List ScanSinkEmission) : MergedScanOutput :=
  finalizeScanSinks productionPostScanOperators level (collectScanSinkEmissions emissions)

/-- scan.rs:127-218 的四件锁步 frontier cache；这里缓存的是候选腿，不是归约后 observation。 -/
structure FrontierCache where
  cachedCount : Nat
  sinks : ScanSinks
deriving DecidableEq, Repr

/--
调用方按 scan.rs 的区间提供 `newlyStable` 与 `frontierTail`：前者对应
[cachedCount,stableSeg)，后者对应 [stableSeg,segments.len)。当冻结边界回缩时先锁步清空四 sink；
新稳定段推进进 cache，tail 只参与本次完整快照，不写 cache。
-/
def resumeMergedOutput (operators : PostScanOperators) (level stableSeg : Nat)
    (cache : FrontierCache) (newlyStable frontierTail : List PerSegmentEmission) :
    FrontierCache × MergedScanOutput :=
  let base := if cache.cachedCount > stableSeg then emptyScanSinks else cache.sinks
  let advanced := appendScanSinks base (collectScanSinks newlyStable)
  let nextCache : FrontierCache := { cachedCount := stableSeg, sinks := advanced }
  let snapshotSinks := appendScanSinks advanced (collectScanSinks frontierTail)
  (nextCache, finalizeScanSinks operators level snapshotSinks)

def AssembledFrom (operators : PostScanOperators) (level : Nat)
    (emissions : List PerSegmentEmission) (output : MergedScanOutput) : Prop :=
  assembleMergedOutput operators level emissions = output

/--
动机：共享逐段记录只能喂唯一双 sink，且候选归约只在扫描后走唯一 writer；同一完整输入不能
装配出两个不同四件快照。
-/
theorem scan_assembly_deterministic
    (operators : PostScanOperators) (level : Nat) (emissions : List PerSegmentEmission)
    (first second : MergedScanOutput)
    (hFirst : AssembledFrom operators level emissions first)
    (hSecond : AssembledFrom operators level emissions second) :
    first = second := by
  unfold AssembledFrom at hFirst hSecond
  exact hFirst.symm.trans hSecond

/-! ## §6 c_p 生命周期与 ownership 投影（P1 侧车，不是四件输出） -/

inductive CpLifecycle where
  | pending
  | closed
deriving DecidableEq, Repr

def CpLifecycle.rank : CpLifecycle → Nat
  | CpLifecycle.pending => 0
  | CpLifecycle.closed => 1

/-- 同一对象、同一 stable revision 内的唯一合法 writer；dirty invalidation 不调用本函数。 -/
def advanceLifecycleInStableRevision
    (current : CpLifecycle) (closureWitness : Bool) : CpLifecycle :=
  match current with
  | CpLifecycle.pending => if closureWitness then CpLifecycle.closed else CpLifecycle.pending
  | CpLifecycle.closed => CpLifecycle.closed

/-- dirty/cascade 依赖失效开启新 revision，因此不构造 `StableAdvance`。 -/
def StableAdvance (before after : CpLifecycle) : Prop :=
  ∃ closureWitness, after = advanceLifecycleInStableRevision before closureWitness

/-- `LevelState.cp_ownership` 单项的稳定对象身份与生命周期。 -/
structure CpOwnership where
  level : Nat
  bCenterOrdinal : Nat
  departureMoveOrdinal : Option Nat
  sourceStart : Option Nat
  lifecycle : CpLifecycle
deriving DecidableEq, Repr

def CpOwnership.advance (ownership : CpOwnership) (closureWitness : Bool) : CpOwnership :=
  { ownership with
      lifecycle := advanceLifecycleInStableRevision ownership.lifecycle closureWitness }

/--
核对 recursive_tower.rs:1864-2015：同一 stable revision 内 Pending 只保持或闭合，Closed 吸收。
recursive_tower.rs:1595-1666 的 dirty 回退属于新 revision，明确不在此前提中。
-/
theorem stable_advance_monotone (before after : CpLifecycle) (h : StableAdvance before after) :
    before.rank ≤ after.rank := by
  rcases h with ⟨closureWitness, rfl⟩
  cases before <;> cases closureWitness <;> decide
end NewChanlun.Origin.ScanAssemblyMirror
