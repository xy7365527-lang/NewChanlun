import Origin.TangencyPolicyIO
import Origin.SegmentFeatureSeq
import Claim10_SegmentV1
import Claim5_ConstitutiveLadder

open NewChanlun.Origin

/-- 特征序列元素：用于核对 closed/open 的相切边界。 -/
def fe (lo hi : Int) (h : lo ≤ hi) : FeatureElem := { low := lo, high := hi, valid := h }

/-- Claim10 的区间输入。 -/
def ci (lo hi : Int) (h : lo ≤ hi) : Formal.Phase2.SegmentV1.Interval := { lo := lo, hi := hi, valid := h }

/-- 构造一个良构顶分型，极值由调用者给出。 -/
def mkTopFractal (ext : Int) : Formal.Phase2.ConstitutiveLadder.Fractal := {
  left := { hi := ext - 2, lo := ext - 2, valid := by omega },
  mid := { hi := ext, lo := ext - 1, valid := by omega },
  right := { hi := ext - 2, lo := ext - 2, valid := by omega },
  kind := .top,
  ext := ext
}

/-- 构造一个良构底分型，极值由调用者给出。 -/
def mkBottomFractal (ext : Int) : Formal.Phase2.ConstitutiveLadder.Fractal := {
  left := { hi := ext + 2, lo := ext + 2, valid := by omega },
  mid := { hi := ext + 1, lo := ext, valid := by omega },
  right := { hi := ext + 2, lo := ext + 2, valid := by omega },
  kind := .bottom,
  ext := ext
}

/-- 构造一笔方向与两端分型均良构的上笔。 -/
def mkUpStroke (lo hi : Int) (_h : lo < hi) : Formal.Phase2.ConstitutiveLadder.Stroke := {
  startFractal := mkBottomFractal lo,
  endFractal := mkTopFractal hi,
  direction := .up,
  confirmed := true
}

/--
三笔公共区间只在价格 10 相切：`max lo = min hi = 10`。
因此 `closed` 的 `≤` 成立，`open` 的 `<` 不成立；其它良构条件也逐笔满足。
-/
def tangentClaim5Segment : Formal.Phase2.ConstitutiveLadder.Segment := {
  strokes := [
    mkUpStroke 5 10 (by decide),
    mkUpStroke 10 20 (by decide),
    mkUpStroke 0 10 (by decide)
  ],
  direction := .up,
  confirmed := true
}

-- 同一相切输入在 closed/open 下的定义层结果相反，且是构建期断言而非打印。
example : ¬ HasGap .closed (fe 5 10 (by decide)) (fe 10 20 (by decide)) := by decide
example : HasGap .open (fe 5 10 (by decide)) (fe 10 20 (by decide)) := by decide
example : Overlaps .closed (fe 5 10 (by decide)) (fe 10 20 (by decide)) := by decide
example : ¬ Overlaps .open (fe 5 10 (by decide)) (fe 10 20 (by decide)) := by decide

-- Claim10 保持 closed；Claim5 分别以 closed/open 真实求值同一三笔相切输入。
example : Formal.Phase2.SegmentV1.classifyTermination .closed
    (ci 5 10 (by decide)) (ci 10 20 (by decide)) = .firstKind := by decide
example : Formal.Phase2.SegmentV1.classifyTermination .open
    (ci 5 10 (by decide)) (ci 10 20 (by decide)) = .secondKind := by decide
example : Formal.Phase2.ConstitutiveLadder.Segment.wellFormedV0 .closed tangentClaim5Segment := by
  unfold Formal.Phase2.ConstitutiveLadder.Segment.wellFormedV0
  simp [tangentClaim5Segment, mkUpStroke, mkBottomFractal, mkTopFractal,
    Formal.Phase2.ConstitutiveLadder.Stroke.wellFormed,
    Formal.Phase2.ConstitutiveLadder.Fractal.wellFormed,
    Formal.Phase2.ConstitutiveLadder.Stroke.lo,
    Formal.Phase2.ConstitutiveLadder.Stroke.hi]
  omega
example : ¬ Formal.Phase2.ConstitutiveLadder.Segment.wellFormedV0 .open tangentClaim5Segment := by
  unfold Formal.Phase2.ConstitutiveLadder.Segment.wellFormedV0
  simp [tangentClaim5Segment, mkUpStroke, mkBottomFractal, mkTopFractal,
    Formal.Phase2.ConstitutiveLadder.Stroke.wellFormed,
    Formal.Phase2.ConstitutiveLadder.Fractal.wellFormed,
    Formal.Phase2.ConstitutiveLadder.Stroke.lo,
    Formal.Phase2.ConstitutiveLadder.Stroke.hi]
  omega

/-- 读取失败测试不会把“意外接受”误记为通过。 -/
def expectTangencyReadFailure (path required marker : String) : IO Unit := do
  try
    let _ ← readTangencyPolicy path
    throw (IO.Error.userError "UNEXPECTED_ACCEPT")
  catch e =>
    let text := IO.Error.toString e
    if text.contains "UNEXPECTED_ACCEPT" then
      throw e
    else if required == "" || text.contains required then
      IO.println marker
    else
      throw e

-- 现役 closed 记录必须进入 Claim10；Claim5 的 closed/open 行为由上方独立构建期断言锁定。
#eval do
  let p ← readTangencyPolicy "../rust/src/theta_v0/policy_records/segment-second.json"
  let termination := Formal.Phase2.SegmentV1.classifyTermination p.tangency
    (ci 5 10 (by decide)) (ci 10 20 (by decide))
  if p.recordId == "segment-second" && p.version == 2 && p.tangency == Tangency.closed &&
      termination == Formal.Phase2.SegmentV1.TerminationCase.firstKind then
    IO.println "TANGENCY_POLICY_IO_OK"
  else
    IO.ofExcept (Except.error "tangency policy was not consumed by Claim10/Claim5")

-- 合法 open 新版本必须改变同一三笔相切输入的结果，不允许成为无效备用值。
#eval do
  let path := "/tmp/tangency-open-20261025.json"
  IO.FS.writeFile path "{\"id\":\"segment-second\",\"version\":3,\"fields\":{\"tangency\":{\"value\":\"open\"}}}"
  let p ← readTangencyPolicy path
  let termination := Formal.Phase2.SegmentV1.classifyTermination p.tangency
    (ci 5 10 (by decide)) (ci 10 20 (by decide))
  if p.recordId == "segment-second" && p.version == 3 && p.tangency == Tangency.open &&
      termination == Formal.Phase2.SegmentV1.TerminationCase.secondKind then
    IO.println "TANGENCY_OPEN_MUTATION_OK"
  else
    IO.ofExcept (Except.error "open record did not change Claim5 tangent behavior")

#eval expectTangencyReadFailure "/tmp/tangency-record-does-not-exist-20261025.json" "" "TANGENCY_MISSING_REJECT_OK"
#eval do
  let path := "/tmp/tangency-null-20261025.json"
  IO.FS.writeFile path "null"
  expectTangencyReadFailure path "" "TANGENCY_NULL_REJECT_OK"
#eval do
  let path := "/tmp/tangency-awaiting-20261025.json"
  IO.FS.writeFile path "{\"id\":\"segment-second\",\"version\":2,\"fields\":{\"tangency\":{\"value\":\"AwaitingPolicy\"}}}"
  expectTangencyReadFailure path "AwaitingPolicy" "TANGENCY_AWAITING_REJECT_OK"
#eval do
  let path := "/tmp/tangency-invalid-20261025.json"
  IO.FS.writeFile path "{\"id\":\"segment-second\",\"version\":2,\"fields\":{\"tangency\":{\"value\":\"diagonal\"}}}"
  expectTangencyReadFailure path "InvalidPolicy" "TANGENCY_INVALID_REJECT_OK"
