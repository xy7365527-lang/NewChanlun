import Lean
import Origin.Tangency

namespace NewChanlun.Origin

private def fail (msg : String) : IO α := IO.ofExcept (Except.error msg)

private def jsonString (j : Lean.Json) (key : String) : IO String := do
  let value ← IO.ofExcept (j.getObjVal? key)
  IO.ofExcept value.getStr?

private def jsonNat (j : Lean.Json) (key : String) : IO Nat := do
  let value ← IO.ofExcept (j.getObjVal? key)
  IO.ofExcept value.getNat?

private def tangencyField (fields : Lean.Json) : IO String := do
  try
    let entry ← IO.ofExcept (fields.getObjVal? "tangency")
    jsonString entry "value"
  catch _ =>
    fail "AwaitingPolicy{segment-second.tangency}：字段未签或缺值"

/-- 读取现役 `segment-second.json`，缺记录/未签/非法值均不回落默认。 -/
def readTangencyPolicy (path : String) : IO SegmentConstructionPolicy := do
  let text ← IO.FS.readFile path
  let root ← IO.ofExcept (Lean.Json.parse text)
  let recordId ← jsonString root "id"
  if recordId != "segment-second" then
    fail ("InvalidPolicy{segment-second.id}：得 " ++ recordId)
  let version ← jsonNat root "version"
  if version == 0 then
    fail "InvalidPolicy{segment-second.version}：须为正整数"
  let fields ← IO.ofExcept (root.getObjVal? "fields")
  let raw ← tangencyField fields
  let tangency ← match raw with
    | "closed" => pure Tangency.closed
    | "open" => pure Tangency.open
    | other => fail ("InvalidPolicy{segment-second.tangency}：得 " ++ other)
  let policy : SegmentConstructionPolicy := { tangency := tangency, recordId := recordId, version := version }
  pure policy

end NewChanlun.Origin
