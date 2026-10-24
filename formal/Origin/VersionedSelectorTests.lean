import Origin.SelectorPolicyIO
import Origin.IntervalNestCertificate
import Origin.NestingCertificate

open Strict.Nest
open NewChanlun.Origin.NestingCertificate
open NewChanlun.Origin.IntervalNestCertificate

def i (e s x : Nat) : Strict.Nest.Interval := { endTime := e, startTime := s, idx := x }
def c : Candidate := { direction := .down, comparison := .trend true true, interval := { lo := 0, hi := 20 } }
def r (e s x : Nat) : RankedCandidate := { candidate := c, key := i e s x }

#eval selectΘWithKeys defaultSelectorKeys [i 10 3 0, i 10 7 1, i 10 7 2]
#eval selectΘWithKeys .StartAscIdxAsc [i 10 3 0, i 10 7 1]
#eval selectΘWithKeys .StartDescIdxDesc [i 10 7 1, i 10 7 9]
#eval SelThetaRanked defaultSelectorPolicy .buy { lo := 1, hi := 5 } [r 10 3 0, r 10 7 2, r 10 7 1]

#eval do
  let p ← readSelectorPolicy "../rust/src/theta_v0/policy_records/selector-keys.json"
  if p.recordId == "selector-keys" && p.version == 1 &&
      p.keys == defaultSelectorKeys then
    IO.println "SELECTOR_POLICY_IO_OK"
  else
    IO.ofExcept (Except.error "selector policy mismatch")

#eval do
  let path := "/tmp/selector-end-ascending.json"
  IO.FS.writeFile path "{\"id\":\"selector-keys\",\"version\":2,\"fields\":{\"end_time\":{\"value\":\"ascending\"},\"start_time\":{\"value\":\"descending\"},\"idx\":{\"value\":\"ascending\"}}}"
  try
    let _ ← readSelectorPolicy path
    IO.ofExcept (Except.error "endTime=ascending was not rejected")
  catch e =>
    if (IO.Error.toString e).contains "end_time" then IO.println "SELECTOR_NEGATIVE_OK" else throw e
