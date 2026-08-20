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

★D2 字段族边界（scan.rs:19-28）：CandDeltaEvent 的 c_interval_full/b_parent/c_structure/
third_class_in_c/cp_certificate_confirm_src/full_trend_c_qualified/full_trend_evidence 与
pan_div_diag 在 3a 期间由 P1/cp_ownership 侧车独立计算，不经本生产扫描。本镜像不从四件输出
臆造这些字段，也不替 P1 定义独立事件装配器。

认识论等级 L0：这里只镜像既有确定性装配。lake build 证明 Lean 内部定义与定理成立，不等于
Rust 已完成跨语言对拍，更不证明交易有效性；指定窗签收属于后续执行切片。
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

/-! ## §2 共享 per-segment 中间记录 -/

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

/-! ## §3 四件输出的字段形状 -/

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

/-! ## §4 双域 sink 与扫描后归约 -/

/-- 每段共享 material 喂出的两域结果；candidateLegs 是未经同 episode 归约的 Trend 腿。 -/
structure PerSegmentEmission where
  material : PerSegmentMaterial
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

/-! ## §5 c_p 生命周期（D2/P1 侧车边界，不是四件输出） -/

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

/--
核对 recursive_tower.rs:1864-2015：同一 stable revision 内 Pending 只保持或闭合，Closed 吸收。
recursive_tower.rs:1595-1666 的 dirty 回退属于新 revision，明确不在此前提中。
-/
theorem stable_advance_monotone (before after : CpLifecycle) (h : StableAdvance before after) :
    before.rank ≤ after.rank := by
  rcases h with ⟨closureWitness, rfl⟩
  cases before <;> cases closureWitness <;> decide
end NewChanlun.Origin.ScanAssemblyMirror
