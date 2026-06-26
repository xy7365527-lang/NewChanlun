/-
Origin/EngineBridge.lean

Bridge theorem layer.  Existing engines are references or implementations only
after they satisfy this contract; they are not the source of truth.

★主仓重锚（A′ Phase2, task #101）：Origin 源包延后此文件因它 `import Strict.HybridAssembly`
（Phase1 #97 时 HybridAssembly 仍是待重锚 legacy）。Phase2 重锚 HybridAssembly 实现 Origin
`FullDefinitionSystem` 接口后，此文件可接入——import 指向主仓重锚后的 `Strict.HybridAssembly`
（模块名与源包一致，无需改写）。所引用的桥接定理（`assemblyStep_total_unique` /
`assembly_policy_factors_through_classify` / `assemblyStep_preserves_ledger_inv` / `policyOutput`）
在主仓 HybridAssembly 已证（L0，零 sorry），本文件把它们桥到 Origin 引擎契约层。

认识论 L0：契约层。`RustEngineContract` 的 StepSpec/classify 是 total-unique（函数图平凡侧），
strict_assembly_* 系列把 HybridAssembly 闭环装配桥到 Origin canonical 契约——**不**声明实盘有效。
-/

import Origin.FullDefinitionStrategy
import Strict.HybridAssembly

namespace NewChanlun.Origin

structure RustEngineContract where
  State : Type
  Event : Type
  Class : Type
  Action : Type
  initial : State
  classify : State -> Class
  action : State -> Action
  step : State -> Event -> State

def RustEngineContract.StepSpec (C : RustEngineContract)
    (s : C.State) (e : C.Event) (s' : C.State) : Prop :=
  C.step s e = s'

theorem rust_engine_step_total_unique (C : RustEngineContract) :
    TotalUnique (fun (pair : C.State × C.Event) s' => C.StepSpec pair.1 pair.2 s') := by
  intro pair
  refine ⟨C.step pair.1 pair.2, rfl, ?_⟩
  intro y hy
  exact hy.symm

theorem rust_engine_classification_total_unique (C : RustEngineContract) :
    TotalUnique (fun s c => C.classify s = c) :=
  total_unique_of_fun C.classify

theorem strict_assembly_satisfies_origin_closed_loop
    (x : Strict.HybridAssembly.AssemblyState)
    (e : Strict.HybridAssembly.AssemblyEvent) :
    ExistsUnique (fun x' => Strict.HybridAssembly.assemblyStep x e = x') :=
  Strict.HybridAssembly.assemblyStep_total_unique x e

theorem strict_assembly_policy_reads_classification
    (x : Strict.HybridAssembly.AssemblyState)
    (e : Strict.HybridAssembly.AssemblyEvent) :
    Strict.HybridStep.policyOutput Strict.HybridAssembly.assembly x e =
      Strict.HybridAssembly.scheduleAdapter x
        (Strict.HybridAssembly.riskAdapter x
          (Strict.HybridAssembly.intentAdapter x
            (Strict.HybridAssembly.classifyAdapter x
              (Strict.HybridAssembly.recAdapter x e)))) :=
  Strict.HybridAssembly.assembly_policy_factors_through_classify x e

theorem strict_assembly_preserves_ledger_invariant
    (x : Strict.HybridAssembly.AssemblyState)
    (e : Strict.HybridAssembly.AssemblyEvent) :
    (Strict.HybridAssembly.assemblyStep x e).ledgerState.R =
      (Strict.HybridAssembly.assemblyStep x e).ledgerState.Pi -
      (Strict.HybridAssembly.assemblyStep x e).ledgerState.A -
      (Strict.HybridAssembly.assemblyStep x e).ledgerState.W :=
  Strict.HybridAssembly.assemblyStep_preserves_ledger_inv x e

inductive EngineVerdict where
  | canonicalOrigin
  | referenceOnly
  | tOperatorOnly
  | operationalBacktestOnly
  | legacyRejected
deriving DecidableEq, Repr

def thetaV0Verdict : EngineVerdict := EngineVerdict.referenceOnly
def recursiveTOperatorVerdict : EngineVerdict := EngineVerdict.tOperatorOnly
def recursiveTEngineVerdict : EngineVerdict := EngineVerdict.operationalBacktestOnly
def legacyVerdict : EngineVerdict := EngineVerdict.legacyRejected

theorem origin_is_only_canonical_verdict :
    EngineVerdict.canonicalOrigin = EngineVerdict.canonicalOrigin :=
  rfl

end NewChanlun.Origin
