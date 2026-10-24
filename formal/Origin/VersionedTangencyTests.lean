import Origin.TangencyPolicyIO
import Origin.SegmentFeatureSeq
import Claim10_SegmentV1
import Claim5_ConstitutiveLadder

open NewChanlun.Origin

def fe (lo hi : Int) (h : lo ≤ hi) : FeatureElem := { low := lo, high := hi, valid := h }
def ci (lo hi : Int) (h : lo ≤ hi) : Formal.Phase2.SegmentV1.Interval := { lo := lo, hi := hi, valid := h }

-- 同一相切输入在 closed/open 下真值相反。
#eval decide (HasGap .closed (fe 5 10 (by decide)) (fe 10 20 (by decide)))
#eval decide (HasGap .open (fe 5 10 (by decide)) (fe 10 20 (by decide)))
#eval decide (Overlaps .closed (fe 5 10 (by decide)) (fe 10 20 (by decide)))
#eval decide (Overlaps .open (fe 5 10 (by decide)) (fe 10 20 (by decide)))

-- Claim10 的 closed 与 Claim5 的 open 各自保持原调用语义。
#eval Formal.Phase2.SegmentV1.classifyTermination .closed
  (ci 5 10 (by decide)) (ci 10 20 (by decide))
#check Formal.Phase2.ConstitutiveLadder.Segment.wellFormedV0 .open

#eval do
  let p ← readTangencyPolicy "../rust/src/theta_v0/policy_records/segment-second.json"
  let _claim10 := Formal.Phase2.SegmentV1.classifyTermination p.tangency
    (ci 5 10 (by decide)) (ci 10 20 (by decide))
  let _claim5 := Formal.Phase2.ConstitutiveLadder.Segment.wellFormedV0 p.tangency
  if p.recordId == "segment-second" && p.version == 2 && p.tangency == Tangency.closed then
    IO.println "TANGENCY_POLICY_IO_OK"
  else
    IO.ofExcept (Except.error "tangency policy mismatch")

#eval do
  let path := "/tmp/tangency-awaiting.json"
  IO.FS.writeFile path "{\"id\":\"segment-second\",\"version\":2,\"fields\":{\"tangency\":{\"value\":\"AwaitingPolicy\"}}}"
  try
    let _ ← readTangencyPolicy path
    IO.ofExcept (Except.error "AwaitingPolicy was not rejected")
  catch e =>
    if (IO.Error.toString e).contains "AwaitingPolicy" then IO.println "TANGENCY_NEGATIVE_OK" else throw e
