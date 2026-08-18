/-
Origin/ScanAssemblyBridge.lean — Rust↔Lean 统一扫描提取对拍协议（#1087 第一切片）

本桥沿 EngineBridge / BspEventBridge / LedgerBridge 的做法：Rust 是被检实现，Lean 镜像给出契约，
桥谓词逐字段比对。它不是 Rust 绑定、不是生产第二判定线，也不宣称尚未运行的窗口已经 0 mismatch。

Rust 提取器后续必须逐 checkpoint/末根为每级输出：
1. CandDeltaEvent 第一切片字段（level/side/两个确认别名/I(A)/I(C)/λ_C/enter_src/cand_delta），
   锚 recursive_tower.rs:1306-1357（struct :1312）、2395-2415；
2. λ_C 输入素材（稳定排序后的 segment 行、同步置换后的 anchor、中枢核心/右端、触发段起止），
   锚 signal.rs:1654-1857（fn :1654）与 recursive_tower.rs:2182-2346（fn :2168）；
3. 与 centers 1:1 的 cp_ownership 稳定身份和 Pending/Closed，锚 pipeline.rs:64-66（字段 :66）、
   recursive_tower.rs:514-552、1863-2015。

行号订正均以 main @ e793b01a81 为准。dirty invalidation（recursive_tower.rs:1603-1641，写回
位于 :1626）开启新 revision；同 revision 对拍才消费 stable lifecycle 单调定理。

第一切片明确不对拍 `CandDeltaEvent` 的 P2/完整 c_p 快照字段：`c_interval_full`、`b_parent`、
`c_structure`、`third_class_in_c`、`cp_certificate_confirm_src`、`full_trend_c_qualified`、
`full_trend_evidence`、`cp_ownership` edge（recursive_tower.rs:1331-1346）。这些字段不被当作
“已逐字段签收”，将在后续切片随对应 Lean 结构语义加入；本票的“逐字段”仅指下方
`RustEventExtraction` 明列的第一切片字段。

同样未进第一切片：recursive_tower.rs:2261-2303 的 pan-only 诊断事件及 `pan_div_diag` 字段。
它不能经输入输出同值透传冒充 Lean 独立判定；后续切片须先镜像诊断支，再把它加入事件列表对拍。

Lean 侧先跑 `assembleCandDelta` / `CpOwnership.advance`，再用本文件两个 parity 谓词逐字段判等。
-/

import Origin.ScanAssemblyMirror

namespace NewChanlun.Origin.ScanAssemblyBridge

open NewChanlun.Origin.ScanAssemblyMirror

/-- Rust wire `Direction` 标签；不复用 Lean 镜像枚举，避免同源类型让桥退化。 -/
inductive RustDirectionTag where
  | up
  | down
deriving DecidableEq, Repr

def RustDirectionTag.toLean : RustDirectionTag -> Direction
  | RustDirectionTag.up => Direction.up
  | RustDirectionTag.down => Direction.down

/-- Rust wire `Side` 标签。 -/
inductive RustSideTag where
  | long
  | short
deriving DecidableEq, Repr

def RustSideTag.toLean : RustSideTag -> Side
  | RustSideTag.long => Side.long
  | RustSideTag.short => Side.short

/-- Rust wire `CpLifecycleStatus` 标签。 -/
inductive RustCpLifecycleTag where
  | pending
  | closed
deriving DecidableEq, Repr

def RustCpLifecycleTag.toLean : RustCpLifecycleTag -> CpLifecycle
  | RustCpLifecycleTag.pending => CpLifecycle.pending
  | RustCpLifecycleTag.closed => CpLifecycle.closed

/-- Rust 提取器在稳定排序/同步置换之后输出的 segment 行。 -/
structure RustSegmentExtraction where
  direction : RustDirectionTag
  startIndex : Nat
  endIndex : Nat
  endPrice : Int
  anchor : Option RustDirectionTag
deriving DecidableEq, Repr

def RustSegmentExtraction.toLean (row : RustSegmentExtraction) : SegmentRow :=
  { direction := row.direction.toLean
    startIndex := row.startIndex
    endIndex := row.endIndex
    endPrice := row.endPrice
    anchor := row.anchor.map RustDirectionTag.toLean }

/-- Rust 最近已确认中枢的 episode 定界投影。 -/
structure RustCenterExtraction where
  endIndex : Nat
  zd : Int
  zg : Int
deriving DecidableEq, Repr

def RustCenterExtraction.toLean (center : RustCenterExtraction) : CenterFrame :=
  { endIndex := center.endIndex, zd := center.zd, zg := center.zg }

/-- Rust `judge_first_cached` 返回值的第一切片投影。 -/
structure RustPredicateExtraction where
  accepted : Bool
  buy1 : Bool
  sell1 : Bool
deriving DecidableEq, Repr

def RustPredicateExtraction.toLean (facts : RustPredicateExtraction) : CandDeltaFacts :=
  { accepted := facts.accepted, buy1 := facts.buy1, sell1 := facts.sell1 }

/--
Rust→Lean 装配输入协议。Rust 输出 post-prelude 素材；Lean 只经 `toLean` 解码后运行
`ScanAssemblyMirror.assembleCandDelta`。
-/
structure RustAssemblyInputExtraction where
  level : Nat
  side : RustSideTag
  divergenceConfirmSrc : Nat
  aIntervalLeft : Nat
  aIntervalRight : Nat
  center : RustCenterExtraction
  departureDir : RustDirectionTag
  untilStart : Nat
  triggerEnd : Nat
  rows : List RustSegmentExtraction
  predicate : RustPredicateExtraction
deriving DecidableEq, Repr

def RustAssemblyInputExtraction.toLean (input : RustAssemblyInputExtraction) : EventAssemblyInput :=
  { level := input.level
    side := input.side.toLean
    divergenceConfirmSrc := input.divergenceConfirmSrc
    aInterval := { left := input.aIntervalLeft, right := input.aIntervalRight }
    center := input.center.toLean
    departureDir := input.departureDir.toLean
    untilStart := input.untilStart
    triggerEnd := input.triggerEnd
    rows := input.rows.map RustSegmentExtraction.toLean
    predicate := input.predicate.toLean }

/-- Rust 序列化的一条 CandDeltaEvent 提取记录。 -/
structure RustEventExtraction where
  level : Nat
  side : RustSideTag
  divergenceConfirmSrc : Nat
  confirmSrc : Nat
  intervalLeft : Nat
  intervalRight : Nat
  aIntervalLeft : Nat
  aIntervalRight : Nat
  cEpisodeStart : Nat
  cEpisodeLeft : Nat
  cEpisodeRight : Nat
  enterSrc : Nat
  candDelta : Bool
deriving DecidableEq, Repr

/--
事件对拍协议：不用结构整体相等掩盖字段遗漏，每个 Rust 输出字段都明确对应 Lean 镜像字段。
-/
def EventParity (rust : RustEventExtraction) (lean : CandDeltaEvent) : Prop :=
  rust.level = lean.level ∧
  rust.side.toLean = lean.side ∧
  rust.divergenceConfirmSrc = lean.divergenceConfirmSrc ∧
  rust.confirmSrc = lean.confirmSrc ∧
  rust.intervalLeft = lean.interval.left ∧
  rust.intervalRight = lean.interval.right ∧
  rust.aIntervalLeft = lean.aInterval.left ∧
  rust.aIntervalRight = lean.aInterval.right ∧
  rust.cEpisodeStart = lean.cEpisodeStart ∧
  rust.cEpisodeLeft = lean.cEpisodeInterval.left ∧
  rust.cEpisodeRight = lean.cEpisodeInterval.right ∧
  rust.enterSrc = lean.enterSrc ∧
  rust.candDelta = lean.candDelta

/-- Rust `LevelState.cp_ownership` 单项提取记录。 -/
structure RustCpExtraction where
  level : Nat
  bCenterOrdinal : Nat
  departureMoveOrdinal : Option Nat
  sourceStart : Option Nat
  lifecycle : RustCpLifecycleTag
deriving DecidableEq, Repr

/-- c_p 对拍协议：稳定身份四字段与生命周期逐字段相等。 -/
def CpParity (rust : RustCpExtraction) (lean : CpOwnership) : Prop :=
  rust.level = lean.level ∧
  rust.bCenterOrdinal = lean.bCenterOrdinal ∧
  rust.departureMoveOrdinal = lean.departureMoveOrdinal ∧
  rust.sourceStart = lean.sourceStart ∧
  rust.lifecycle.toLean = lean.lifecycle

/-- 两侧 c_p 列表必须同长、同位且每项满足 `CpParity`。 -/
def CpListParity : List RustCpExtraction -> List CpOwnership -> Prop
  | [], [] => True
  | rust :: rustRest, lean :: leanRest => CpParity rust lean ∧ CpListParity rustRest leanRest
  | _, _ => False

/-- 单条底层关系：事件可缺省但两侧必须同缺省；checkpoint 规范入口不得用它替代列表检查。 -/
def SingleRecordParity
    (rustEvent : Option RustEventExtraction) (leanEvent : Option CandDeltaEvent)
    (rustCp : List RustCpExtraction) (leanCp : List CpOwnership) : Prop :=
  (match rustEvent, leanEvent with
   | none, none => True
   | some rust, some lean => EventParity rust lean
   | _, _ => False) ∧
  CpListParity rustCp leanCp

/--
规范事件检查入口：调用者只提供 Rust 的输入/输出提取；Lean 输出必须由镜像函数现算，不接受任意
`leanEvent` 注入。Rust 无事件时 Lean 也必须为 `none`。
-/
def EventCheck (rustInput : RustAssemblyInputExtraction)
    (rustEvent : Option RustEventExtraction) : Prop :=
  match rustEvent, assembleCandDelta rustInput.toLean with
  | none, none => True
  | some rust, some lean => EventParity rust lean
  | _, _ => False

/--
事件列表签收：Rust 输入与输出必须同长同序，每一对都经 `EventCheck input (some output)`；
任一侧额外/缺失事件立即为 False。
-/
def EventListCheck :
    List RustAssemblyInputExtraction -> List RustEventExtraction -> Prop
  | [], [] => True
  | input :: inputRest, output :: outputRest =>
      EventCheck input (some output) ∧ EventListCheck inputRest outputRest
  | _, _ => False

/--
同一 stable revision 内的一项 c_p 推进检查：before 先对齐，再由 Lean
`CpOwnership.advance` 现算 after。dirty invalidation 不得调用本关系，必须开启新 revision。
-/
def StableCpCheck (rustBefore rustAfter : RustCpExtraction) (leanBefore : CpOwnership)
    (closureWitness : Bool) : Prop :=
  CpParity rustBefore leanBefore ∧ CpParity rustAfter (leanBefore.advance closureWitness)

/-- 同位、同长的 stable-revision c_p 推进列表。 -/
def StableCpListCheck :
    List RustCpExtraction -> List RustCpExtraction -> List CpOwnership -> List Bool -> Prop
  | [], [], [], [] => True
  | rustBefore :: rustBeforeRest, rustAfter :: rustAfterRest,
      leanBefore :: leanBeforeRest, witness :: witnessRest =>
      StableCpCheck rustBefore rustAfter leanBefore witness ∧
        StableCpListCheck rustBeforeRest rustAfterRest leanBeforeRest witnessRest
  | _, _, _, _ => False

/--
checkpoint 规范检查：事件侧只能走 `EventCheck` 的镜像现算；c_p 侧只能走同 revision 的 stable advance。
-/
def CheckpointCheck (rustInputs : List RustAssemblyInputExtraction)
    (rustEvents : List RustEventExtraction)
    (rustCpBefore rustCpAfter : List RustCpExtraction)
    (leanCpBefore : List CpOwnership) (closureWitnesses : List Bool) : Prop :=
  EventListCheck rustInputs rustEvents ∧
    StableCpListCheck rustCpBefore rustCpAfter leanCpBefore closureWitnesses

/-- 更直接的协议投影：对拍成立即 λ_C（Rust c_episode_start）等于 Lean λ_C。 -/
theorem event_parity_lambda_c
    (rust : RustEventExtraction) (lean : CandDeltaEvent) (h : EventParity rust lean) :
    rust.cEpisodeStart = lean.cEpisodeStart :=
  h.2.2.2.2.2.2.2.2.1

theorem cp_parity_forces_lifecycle_equal
    (rust : RustCpExtraction) (lean : CpOwnership) (h : CpParity rust lean) :
    rust.lifecycle.toLean = lean.lifecycle :=
  h.2.2.2.2

end NewChanlun.Origin.ScanAssemblyBridge
