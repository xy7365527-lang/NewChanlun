/-
Origin/ScanAssemblyMirror.lean — 3a 统一扫描装配语义镜像（#1087 第一切片）

本模块只钉生产 Rust 在 main @ e793b01a81 已有的装配语义，不给判据增删条件，也不参与生产判定。
事实锚（e793 行号订正：票面 1644/1287/2142/63 分别不是目标定义行）：
* signal.rs:1654-1860：`extract_signals_with_hist_anchored` 及统一扫描 prelude、逐段扫描；
* recursive_tower.rs:1306-1357：`CandDeltaEvent`（struct 起于 :1312）的规范字段与兼容别名；
* recursive_tower.rs:2161-2433：`level_cand_delta`（fn 起于 :2168）的谓词消费、事件装配与排序；
* recursive_tower.rs:514-552、1863-2015：c_p 对象从 Pending 到 Closed，Closed 吸收；
* pipeline.rs:64-66：`LevelState.cp_ownership`（字段位于 :66）是与 centers 1:1 的生命周期投影。

有效域：recursive_tower.rs:1603-1641 的 dirty invalidation 会在 :1626 把旧 Closed 对象重置
Pending。故本文件的单调/吸收定理只管同一 stable revision 内 `advance_cp_lifecycles` 的推进；
dirty 重算开启新 revision，不属于 `StableAdvance`，不得把定理膨胀成任意全局快照单调。

第一切片不建模 recursive_tower.rs:2261-2303 的 pan-only 诊断分支（`cand_delta=false`、
`pan_div_diag=true`），也不把 `pan_div_diag` 透传成 Lean 判定。该诊断事件与其列表并入规则留后续切片。

认识论等级 L0：这是上述既有确定性计算的类型/函数镜像。`lake build` 只证明本镜像内部定义与
定理成立，不证明 Rust 已与本镜像对拍，也不证明交易有效性。跨语言签收协议在
Origin.ScanAssemblyBridge；指定窗 0 mismatch 属后续执行切片。
-/

namespace NewChanlun.Origin.ScanAssemblyMirror

/-! ## §1 λ_C episode 定界 -/

/-- 扫描方向；序列化标签固定为 Rust `Direction::{Up,Down}`。 -/
inductive Direction where
  | up
  | down
deriving DecidableEq, Repr

/-- Rust 稳定排序并同步置换方向锚之后的一行扫描素材。 -/
structure SegmentRow where
  direction : Direction
  startIndex : Nat
  endIndex : Nat
  endPrice : Int
  anchor : Option Direction
deriving DecidableEq, Repr

/-- 最近已确认中枢对 episode 定界所需的核心与右端。 -/
structure CenterFrame where
  endIndex : Nat
  zd : Int
  zg : Int
deriving DecidableEq, Repr

/--
反向段是否回到中枢核心内侧。这个谓词只切 episode；离开段资格仍由 `anchor` 决定，二者不互换。
-/
def reenters (center : CenterFrame) (departureDir : Direction) (row : SegmentRow) : Prop :=
  row.direction ≠ departureDir ∧
    match departureDir with
    | Direction.down => center.zd ≤ row.endPrice
    | Direction.up => row.endPrice ≤ center.zg

instance (center : CenterFrame) (departureDir : Direction) (row : SegmentRow) :
    Decidable (reenters center departureDir row) := by
  cases departureDir <;> unfold reenters <;> infer_instance

/-- Rust λ_C 窗口：中枢右端起至当前触发段起点止，保持 prelude 的稳定时间序。 -/
def episodeWindow (rows : List SegmentRow) (center : CenterFrame) (untilStart : Nat) :
    List SegmentRow :=
  rows.filter fun row => decide (center.endIndex ≤ row.startIndex ∧ row.startIndex ≤ untilStart)

/-- 窗口内最后一个回中枢段的右端；没有回中枢段时以 0 表示无边界。 -/
def lastReentryBoundary (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) : Nat :=
  rows.foldl
    (fun boundary row => if reenters center departureDir row then row.endIndex else boundary)
    0

/-- 边界之后第一个获方向锚的同向段起点。 -/
def firstAnchoredStartAfter (rows : List SegmentRow) (departureDir : Direction)
    (boundary : Nat) : Option Nat :=
  match rows with
  | [] => none
  | row :: rest =>
      if row.anchor = some departureDir ∧ boundary ≤ row.startIndex then
        some row.startIndex
      else
        firstAnchoredStartAfter rest departureDir boundary

/--
λ_C：当前 episode 中，最后一个回中枢边界之后首个获同向锚的段起点。
无合法同向体返回 `none`，不伪造坐标。
-/
def episodeStart (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart : Nat) : Option Nat :=
  let window := episodeWindow rows center untilStart
  firstAnchoredStartAfter window departureDir (lastReentryBoundary window center departureDir)

/-- I(C) = [λ_C, 触发段右端]；左右端均为 source-index。 -/
structure EpisodeBounds where
  left : Nat
  right : Nat
deriving DecidableEq, Repr

def episodeBounds (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart triggerEnd : Nat) : Option EpisodeBounds :=
  (episodeStart rows center departureDir untilStart).map fun lambdaC =>
    { left := lambdaC, right := triggerEnd }

/-- `EpisodeDelimitedBy` 是试点定理使用的关系视图；它不另写一份定界算法。 -/
def EpisodeDelimitedBy (rows : List SegmentRow) (center : CenterFrame)
    (departureDir : Direction) (untilStart triggerEnd : Nat) (bounds : EpisodeBounds) : Prop :=
  episodeBounds rows center departureDir untilStart triggerEnd = some bounds

/--
试点定理（#1087）：同一扫描素材、同一中枢/方向/触发段上的 λ_C episode 定界唯一。
证明直接消费唯一 writer `episodeBounds`，无 `sorry`/额外公理。
-/
theorem episode_boundary_unique
    (rows : List SegmentRow) (center : CenterFrame) (departureDir : Direction)
    (untilStart triggerEnd : Nat) (first second : EpisodeBounds)
    (hFirst : EpisodeDelimitedBy rows center departureDir untilStart triggerEnd first)
    (hSecond : EpisodeDelimitedBy rows center departureDir untilStart triggerEnd second) :
    first = second := by
  unfold EpisodeDelimitedBy at hFirst hSecond
  exact Option.some.inj (hFirst.symm.trans hSecond)

/-! ## §2 cand_delta 谓词层与事件装配 -/

/-- Rust `Side::{Long,Short}` 的协议标签。 -/
inductive Side where
  | long
  | short
deriving DecidableEq, Repr

/--
Rust `judge_first_cached` 的提取结果。镜像不重裁判据：`accepted` 表示既有结构候选门已返回 Some；
`buy1`/`sell1` 是该返回值上的既有 bit。cand_delta 只从两 bit 派生，不设第二套判据。
-/
structure CandDeltaFacts where
  accepted : Bool
  buy1 : Bool
  sell1 : Bool
deriving DecidableEq, Repr

def CandDeltaFacts.value (facts : CandDeltaFacts) : Bool :=
  facts.buy1 || facts.sell1

/-- 第一切片装配所需、由生产扫描提取的输入；确认时点与 episode 右端保持独立。 -/
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

/-- `CandDeltaEvent` 第一切片镜像；兼容别名被显式保留，供逐字段对拍。 -/
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
事件装配：结构候选门拒绝或 λ_C 无来源时不产事件；成功时三个 episode 别名同源，确认时点独立。
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

theorem assembled_episode_aliases
    (input : EventAssemblyInput) (event : CandDeltaEvent)
    (h : assembleCandDelta input = some event) :
    event.interval = event.cEpisodeInterval ∧
      event.cEpisodeStart = event.cEpisodeInterval.left ∧
      event.enterSrc = event.cEpisodeStart := by
  unfold assembleCandDelta at h
  split at h
  · split at h
    · contradiction
    · cases h
      simp
  · contradiction

/-! ## §3 c_p 生命周期与 cp_ownership 投影 -/

inductive CpLifecycle where
  | pending
  | closed
deriving DecidableEq, Repr

def CpLifecycle.rank : CpLifecycle -> Nat
  | CpLifecycle.pending => 0
  | CpLifecycle.closed => 1

/-- 同一 stable revision 内，合法第三类闭合见证只允许 Pending→Closed；Closed 是吸收态。 -/
def advanceLifecycleInStableRevision
    (current : CpLifecycle) (closureWitness : Bool) : CpLifecycle :=
  match current with
  | CpLifecycle.pending => if closureWitness then CpLifecycle.closed else CpLifecycle.pending
  | CpLifecycle.closed => CpLifecycle.closed

/-- dirty invalidation 不构造本关系；它属于新 revision 的重算。 -/
def StableAdvance (before after : CpLifecycle) : Prop :=
  ∃ closureWitness, after = advanceLifecycleInStableRevision before closureWitness

theorem stable_advance_monotone (before after : CpLifecycle) (h : StableAdvance before after) :
    before.rank ≤ after.rank := by
  rcases h with ⟨closureWitness, rfl⟩
  cases before <;> cases closureWitness <;> decide

theorem lifecycle_monotone_in_stable_revision
    (current : CpLifecycle) (closureWitness : Bool) :
    current.rank ≤ (advanceLifecycleInStableRevision current closureWitness).rank := by
  cases current <;> cases closureWitness <;> decide

theorem closed_absorbing_in_stable_revision (closureWitness : Bool) :
    advanceLifecycleInStableRevision CpLifecycle.closed closureWitness = CpLifecycle.closed := by
  rfl

/-- `LevelState.cp_ownership` 单项所需的稳定对象身份与生命周期。 -/
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

theorem cp_ownership_lifecycle_monotone (ownership : CpOwnership) (closureWitness : Bool) :
    ownership.lifecycle.rank ≤ (ownership.advance closureWitness).lifecycle.rank :=
  lifecycle_monotone_in_stable_revision ownership.lifecycle closureWitness

end NewChanlun.Origin.ScanAssemblyMirror
