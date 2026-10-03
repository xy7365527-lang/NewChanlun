import Lean
import OrderBaseBoundary
import PrimitiveSemantics
import ClockAndGeometry
import DirectionalChangeLemmas

/- #1467 研究声明锁导出器。读取刚编译的研究模块，不进入生产形式化库。 -/
open Lean Elab Command

deriving instance Repr for ReducibilityHints, ConstantVal, AxiomVal,
  DefinitionVal, TheoremVal, OpaqueVal, InductiveVal, ConstructorVal,
  RecursorRule, RecursorVal, QuotKind, QuotVal, ConstantInfo

run_cmd do
  let env ← getEnv
  let targets := #[`OrderBaseBoundary, `PrimitiveSemantics,
    `ClockAndGeometry, `DirectionalChangeLemmas]
  let declarations := env.constants.toList.filter fun (n, _) =>
    match env.getModuleIdxFor? n with
    | some i => targets.contains env.header.moduleNames[i.toNat]!
    | none => false
  if declarations.isEmpty then throwError "声明锁没有读到任何目标声明"
  let names := declarations.map Prod.fst |>.toArray.qsort (·.toString < ·.toString)
  for n in names do
    let some ci := env.find? n | throwError "声明消失：{n}"
    -- 使用完整Expr表示，不做会隐藏隐式参数/宇宙层级的美化输出。
    -- 定理仅锁类型；所有非定理锁完整ConstantInfo，含定义体与归纳构造信息。
    let body := match ci with
      | .thmInfo v => s!"theorem {repr v.levelParams} {repr v.type}"
      | other => s!"{repr other}"
    let axioms ← collectAxioms n
    let axioms := axioms.qsort (·.toString < ·.toString)
    for a in axioms do
      unless #[`propext, `Quot.sound, `Classical.choice].contains a do
        throwError "不允许的假定：{n} 依赖 {a}"
    liftIO <| IO.println s!"DECL {n}\n{body}\nAXIOMS {repr axioms}"
  -- 引入模块集合也锁定；依赖的实际编译对象hash由外层脚本逐一保存。
  for n in env.header.moduleNames do
    liftIO <| IO.println s!"IMPORT {n}"
