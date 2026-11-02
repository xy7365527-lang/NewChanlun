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
#eval do
  let policy ← readSelectorPolicy "../rust/src/theta_v0/policy_records/selector-keys.json"
  pure (SelThetaRanked policy .buy { lo := 1, hi := 5 } [r 10 3 0, r 10 7 2, r 10 7 1])

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

/-- 单节点证书输入；策略只由 `readNestChain` 统一注入。 -/
def singletonInput (cands : List Strict.Nest.Interval) : Strict.Nest.NestLevelInput :=
  { lvl := 0, cands := cands, candidateOK := true, confirmOK := true }

/-- 默认记录与旧选择器逐项相同。 -/
theorem chosen_default_matches_legacy :
    ({ lvl := 0,
       cands := [i 10 3 0, i 10 7 1, i 10 7 2],
       candidateOK := true, confirmOK := true,
       policy := defaultSelectorPolicy } : NestLevel).chosen =
      selectΘ [i 10 3 0, i 10 7 1, i 10 7 2] := rfl

/-- 第二份合法记录用于生产链变异；不是缺记录时的备用值。 -/
def alternateSelectorPolicy : Strict.Nest.SelectorPolicy := {
  keys := .StartAscIdxAsc,
  recordId := "selector-keys",
  version := 2
}

/-- 同一条生产证书链不允许混用多份 selector 记录。 -/
theorem mixed_selector_chain_is_rejected :
    chiBool 0 [witOp, { witMid with policy := alternateSelectorPolicy }, witExec] = false := by
  native_decide

/-- 读取失败测试不会把“意外接受”误记为通过。 -/
def expectReadFailure (path required marker : String) : IO Unit := do
  try
    let _ ← readSelectorPolicy path
    throw (IO.Error.userError "UNEXPECTED_ACCEPT")
  catch e =>
    let text := IO.Error.toString e
    if text.contains "UNEXPECTED_ACCEPT" then
      throw e
    else if required == "" || text.contains required then
      IO.println marker
    else
      throw e

#eval do
  let chain ← readNestChain "../rust/src/theta_v0/policy_records/selector-keys.json"
    [singletonInput [i 10 3 0, i 10 7 1, i 10 7 2]]
  match chain with
  | [level] =>
      if level.chosen == some (i 10 7 1) then
        IO.println "NEST_CHOSEN_DEFAULT_OK"
      else
        throw (IO.Error.userError "default NestLevel.chosen mismatch")
  | _ => throw (IO.Error.userError "unexpected chain shape")

#eval do
  let path := "/tmp/selector-start-ascending.json"
  IO.FS.writeFile path "{\"id\":\"selector-keys\",\"version\":2,\"fields\":{\"end_time\":{\"value\":\"descending\"},\"start_time\":{\"value\":\"ascending\"},\"idx\":{\"value\":\"ascending\"}}}"
  let chain ← readNestChain path [singletonInput [i 10 3 0, i 10 7 1, i 10 7 2]]
  match chain with
  | [level] =>
      if level.chosen == some (i 10 3 0) then
        IO.println "NEST_CHOSEN_MUTATION_OK"
      else
        throw (IO.Error.userError "record did not change NestLevel.chosen")
  | _ => throw (IO.Error.userError "unexpected chain shape")

#eval do
  let path := "/tmp/selector-start-ascending.json"
  let (result, policy) ← chiBoolWithRecord path 0
    [singletonInput [i 10 3 0, i 10 7 1, i 10 7 2]]
  if result && policy.version == 2 then
    IO.println "CHIBOOL_RECORD_OK"
  else
    throw (IO.Error.userError "chiBool did not consume the actual record")

#eval expectReadFailure "/tmp/selector-record-does-not-exist-20261025.json" "" "SELECTOR_MISSING_REJECT_OK"
#eval do
  let path := "/tmp/selector-null.json"
  IO.FS.writeFile path "null"
  expectReadFailure path "" "SELECTOR_NULL_REJECT_OK"
#eval do
  let path := "/tmp/selector-awaiting.json"
  IO.FS.writeFile path "{\"id\":\"selector-keys\",\"version\":1,\"fields\":{}}"
  expectReadFailure path "AwaitingPolicy" "SELECTOR_AWAITING_REJECT_OK"
#eval do
  let path := "/tmp/selector-invalid.json"
  IO.FS.writeFile path "{\"id\":\"selector-keys\",\"version\":1,\"fields\":{\"end_time\":{\"value\":\"descending\"},\"start_time\":{\"value\":\"diagonal\"},\"idx\":{\"value\":\"ascending\"}}}"
  expectReadFailure path "InvalidPolicy" "SELECTOR_INVALID_REJECT_OK"
