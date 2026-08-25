import Origin.NestingCertificate

namespace NewChanlun.Origin.CertificateChainParity

open NewChanlun.Origin.NestingCertificate

def trendBuy : Candidate :=
  { direction := .down
    comparison := .trend true true
    interval := { lo := 10, hi := 20 } }

def wrongDirection : Candidate :=
  { trendBuy with direction := .up }

def pointWithoutOuterExtreme : Candidate :=
  { direction := .down
    comparison := .type23 (.consolidation true true)
    interval := { lo := 8, hi := 22 } }

def xzdWithoutExtreme : Candidate :=
  { direction := .down
    comparison := .xiaozhuandaBridge (.trend true false)
    interval := { lo := 5, hi := 25 } }

example : DivCand .buy trendBuy = true := by decide
example : DivCand .buy wrongDirection = false := by decide
example : DivCand .buy pointWithoutOuterExtreme = true := by decide
example : DivCand .buy xzdWithoutExtreme = true := by decide

example : Cand .buy { lo := 12, hi := 18 } [wrongDirection, trendBuy] = true := by decide
example : Cand .buy { lo := 2, hi := 30 } [trendBuy] = false := by decide
example : Cand .buy { lo := 12, hi := 18 } [] = false := by decide

def adjacentPassed : EvidenceEdge := { adjacent := true, predicatePassed := true }
def skippedPassed : EvidenceEdge := { adjacent := false, predicatePassed := true }

example : Strict true [adjacentPassed, adjacentPassed] = true := by decide
example : Strict true [adjacentPassed, skippedPassed] = false := by decide
example : Strict false [adjacentPassed] = false := by decide

/-- Rust `NestTurnClass` 四类的 L0 镜像标签。 -/
inductive TurnClass where
  | nestedConfirmed
  | xiaozhuandaCandidate
  | execEvidenceOnly
  | deferOrphan
deriving DecidableEq, Repr

/--
  四类分派只读证书 `confirmed` sidecar 与 044:30 必要条件结果；`orphan=true` 表示事件未被证书覆盖。
  候选类不携确认语义，维持 044:30 封锁。
-/
def classifyTurn (confirmed : List Bool) (xzdNecessary orphan : Bool) : TurnClass :=
  if orphan then .deferOrphan
  else match confirmed.head? with
    | some true => .nestedConfirmed
    | some false => if xzdNecessary then .xiaozhuandaCandidate else .execEvidenceOnly
    | none => .execEvidenceOnly

example : classifyTurn [true, true] false false = .nestedConfirmed := by decide
example : classifyTurn [false, true] true false = .xiaozhuandaCandidate := by decide
example : classifyTurn [false, true] false false = .execEvidenceOnly := by decide
example : classifyTurn [] false true = .deferOrphan := by decide

def boolJson : Bool → String
  | true => "true"
  | false => "false"

def boolListJson (values : List Bool) : String :=
  "[" ++ String.intercalate "," (values.map boolJson) ++ "]"

def turnClassJson : TurnClass → String
  | .nestedConfirmed => "nested_confirmed"
  | .xiaozhuandaCandidate => "xiaozhuanda_candidate"
  | .execEvidenceOnly => "exec_evidence_only"
  | .deferOrphan => "defer_orphan"

/-- Cargo 常驻对拍读取的 Lean 机器导出 JSON。 -/
def certificateChainParityJson : String :=
  let nestedConfirmed := [true, true]
  let xzdConfirmed := [false, true]
  "{\"confirmed_vectors\":{\"nested\":" ++ boolListJson nestedConfirmed ++
    ",\"xiaozhuanda\":" ++ boolListJson xzdConfirmed ++
    "},\"turn_classes\":{\"nested\":\"" ++ turnClassJson (classifyTurn nestedConfirmed false false) ++
    "\",\"xiaozhuanda\":\"" ++ turnClassJson (classifyTurn xzdConfirmed true false) ++
    "\",\"exec\":\"" ++ turnClassJson (classifyTurn xzdConfirmed false false) ++
    "\",\"orphan\":\"" ++ turnClassJson (classifyTurn [] false true) ++
    "\"},\"strict\":{\"closed_adjacent\":" ++ boolJson (Strict true [adjacentPassed, adjacentPassed]) ++
    ",\"closed_skip\":" ++ boolJson (Strict true [adjacentPassed, skippedPassed]) ++
    ",\"open_adjacent\":" ++ boolJson (Strict false [adjacentPassed]) ++ "}}"

#eval IO.println certificateChainParityJson

end NewChanlun.Origin.CertificateChainParity
