import Lean
import Strict.Nest

namespace Strict.Nest

private def fail (msg : String) : IO α := IO.ofExcept (Except.error msg)

private def jsonString (j : Lean.Json) (key : String) : IO String := do
  let value ← IO.ofExcept (j.getObjVal? key)
  IO.ofExcept value.getStr?

private def jsonNat (j : Lean.Json) (key : String) : IO Nat := do
  let value ← IO.ofExcept (j.getObjVal? key)
  IO.ofExcept value.getNat?

private def fieldString (fields : Lean.Json) (name : String) : IO String := do
  try
    let entry ← IO.ofExcept (fields.getObjVal? name)
    jsonString entry "value"
  catch _ =>
    fail ("AwaitingPolicy{selector-keys." ++ name ++ "}：字段未签或缺值")

private def direction (name value : String) : IO Bool := do
  match value with
  | "descending" => pure true
  | "ascending" => pure false
  | other => fail ("InvalidPolicy{selector-keys." ++ name ++ "}：非法方向 " ++ other)

/-- 读取 `selector-keys.json`。`endTime=ascending` 在值域外，明确拒绝。 -/
def readSelectorPolicy (path : String) : IO SelectorPolicy := do
  let text ← IO.FS.readFile path
  let root ← IO.ofExcept (Lean.Json.parse text)
  let recordId ← jsonString root "id"
  if recordId != "selector-keys" then
    fail ("InvalidPolicy{selector-keys.id}：得 " ++ recordId)
  let version ← jsonNat root "version"
  if version == 0 then
    fail "InvalidPolicy{selector-keys.version}：须为正整数"
  let fields ← IO.ofExcept (root.getObjVal? "fields")
  let endTime ← fieldString fields "end_time"
  if endTime != "descending" then
    fail ("InvalidPolicy{selector-keys.end_time}：升序尚未批准，得 " ++ endTime)
  let startTime ← fieldString fields "start_time"
  let idx ← fieldString fields "idx"
  let startTimeDescending ← direction "start_time" startTime
  let idxDescending ← direction "idx" idx
  let keys : SelectorKeys := match startTimeDescending, idxDescending with
    | true, true => .StartDescIdxDesc
    | true, false => .StartDescIdxAsc
    | false, true => .StartAscIdxDesc
    | false, false => .StartAscIdxAsc
  let policy : SelectorPolicy := { keys := keys, recordId := recordId, version := version }
  pure policy

end Strict.Nest
