import Lean
import DCWitness

/- #1467 冻结目标的外部类型检查；证明文件不能自己宣布目标已经完成。 -/
example : DCSpec.DC_CANON_TARGET := DCSpec.dc_canon
example : DCSpec.DC_CAUSAL_TARGET := DCSpec.dc_causal
example : DCSpec.DC_GEOMETRY_TARGET := DCSpec.dc_geometry
example : DCSpec.Whole DCSpec.witnessPath 2 3 DCSpec.witnessResult :=
  DCSpec.nonempty_witness
example : (DCSpec.units DCSpec.witnessResult).length = 1 :=
  DCSpec.nonempty_witness_has_one_unit

open Lean Elab Command in
run_cmd do
  for n in #[`DCSpec.dc_canon, `DCSpec.dc_causal, `DCSpec.dc_geometry,
      `DCSpec.nonempty_witness, `DCSpec.nonempty_witness_has_one_unit] do
    let axioms ← collectAxioms n
    for a in axioms do
      unless #[`propext, `Classical.choice, `Quot.sound].contains a do
        throwError "验收拒绝：{n} 依赖 {a}"
    let some ci := (← getEnv).find? n | throwError "目标缺失：{n}"
    liftIO <| IO.println s!"ACCEPTED {n}\nTYPE {repr ci.type}\nAXIOMS {repr axioms}"

#print DCSpec.DC_CANON_TARGET
#print DCSpec.DC_CAUSAL_TARGET
#print DCSpec.DC_GEOMETRY_TARGET
