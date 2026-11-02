namespace NewChanlun.Origin

/-- 相切是否算重合。`closed` 承 #246；`open` 是合法备选，不得混用两种缺口定义。 -/
inductive Tangency where
  | closed
  | open
deriving DecidableEq, Repr

/-- `segment-second` 的纯记录：实际值 + record id/version。 -/
structure SegmentConstructionPolicy where
  tangency : Tangency
  recordId : String
  version : Nat
deriving Repr

/-- 当前 `segment-second/2` 的显式测试记录，不是缺记录时的备用值。 -/
def defaultSegmentConstructionPolicy : SegmentConstructionPolicy := {
  tangency := .closed,
  recordId := "segment-second",
  version := 2
}

end NewChanlun.Origin
