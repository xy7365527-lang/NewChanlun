import Lean
import DCOriginBridge

open Lean Elab Command in
run_cmd do
  for n in #[`DCOriginBridge.dc_three_center, `DCOriginBridge.own_child_core,
      `DCOriginBridge.inherited_core_not_CenterFull,
      `DCOriginBridge.connector_geometric_witness] do
    let axioms ← collectAxioms n
    for a in axioms do
      unless #[`propext, `Classical.choice, `Quot.sound].contains a do
        throwError "桥接验收拒绝：{n} 依赖 {a}"
    let some ci := (← getEnv).find? n | throwError "定理缺失：{n}"
    liftIO <| IO.println s!"ACCEPTED {n}\nTYPE {repr ci.type}\nAXIOMS {repr axioms}"
