/-
Origin/LedgerBridge.lean

★A′ Phase2 双账本桥 / **兼容层**（task #101 桥 + task #127 TW native 重锚后定位）：把 #93 双账本
扩维 + OQ-9 gate（取本金三阶段 TW + stage 单向 + OQ-9 入口证书）挂到 Origin canonical 接口。

════════════════════════════════════════════════════════════════════════
## ★定位（task #127 TW native 重锚后）：本文件 = compatibility/bridge theorem 层

TW 定义所有权（TWState/TWEvent/twStep + #90 不同构 + OQ-9 全套）已 native 重锚进 **Origin.TotalWealth**
（NewChanlun.Origin.TotalWealth，task #127）。本文件**不再**从 legacy 路径 import/open
`Tlayers.Accounting.TotalWealth`——它 open 的是 `NewChanlun.Origin.TotalWealth`（Origin native）。

本文件的角色降为 **bridge/compatibility 定理层**：它**不定义**任何 TW 结构，只把 Origin.TotalWealth
的 native 定理 + HybridAssembly 闭环定理**桥接/暴露**在 `NewChanlun.Origin.LedgerBridge` 命名空间
（坐实「双账本两端都在 Origin 接口下成立」）。所有桥定理的证明体直接引下游已证定理（无新增 TW 语义）。

════════════════════════════════════════════════════════════════════════
## 本文件做什么：把 #93 成果重锚 Origin，证「双账本两端都在 Origin 接口下成立」

A′ 把 `formal/Origin/` 定为唯一 canonical base（#97）。`Strict/HybridAssembly.lean`（§8）已
**实现 Origin `FullDefinitionSystem` 接口**（`originFullDef` 见证 + 三项接口义务定理）。本文件
把 #93 的**双账本**（R=Π-A-W ledger ⊕ 取本金三阶段 TW）+ OQ-9 gate 桥到 Origin：

- **R=Π-A-W 端**（Origin 单账本 `LedgerState`）：Origin 接口见证 `originFullDef` 的 T 复用 Origin
  `ledgerStep`，闭环保恒等（`HybridAssembly.originFullDef_transition_preserves_ledger_inv`）。
- **TW 端**（取本金三阶段 `TWState`，#90 与 R=Π-A-W 不同构）：HybridAssembly 自有闭环 `assemblyStep`
  的 twState 真被线程化；TW 保持仅在零实现盈亏子域成立，stage 单向 + OQ-9 gate 仍按各自前提成立。
  本文件把这些桥到 Origin 命名空间的引擎契约层。

════════════════════════════════════════════════════════════════════════
## ★#90 不同构在 Origin 上的语义形式（no-workaround，关键）

Origin `FullDefinitionSystem.StrictState` 携带的 `LedgerState` 是**单账本** R=Π-A-W（4 字段，无
twState）。这**不是缺陷**——#90 已证 R=Π-A-W 与取本金三阶段 TW **不同构**，故 Origin 单账本接口
**结构上不能吸收 TW**（吸收 = 抹掉 #90）。双账本的严格形式 = **两个账本各在其层**：
- Origin `StrictState.ledger`（R=Π-A-W）由 Origin 接口承载（originFullDef）；
- 取本金三阶段 TW 由 HybridAssembly `AssemblyState.twState` 闭环承载（#90 第二端）。

本文件**不**把 TW 塞进 Origin `LedgerState`（#90 否证之），而是证：在 Origin canonical base 之下，
**两个账本的不变量按各自有效域成立且并置**——R=Π-A-W 端（Origin 接口）+ TW/stage/OQ-9 端
（HybridAssembly 闭环；TW 保持限零实现盈亏）。这尊重 #90 不同构（两端不糊成一个），同时让两端
都锚到 Origin。

════════════════════════════════════════════════════════════════════════
## 认识论等级（formalization-validity-domain，强制标注）

证明体是 **L0**（结构/定义层）；`tw_preserved_in_loop` 另消费外部提供的零实现盈亏前提，其生产对应
属 L2，当前 Lean 状态/事件无法内生判定。`lake env lean` 通过 = R=Π-A-W 恒等、零实现盈亏子域的
TW 保持、stage 单向与 OQ-9 入口证书保持在各自有效域成立；**不**是缠论盈利 / 实盘有效声明（L3）。
禁 sorry/admit/axiom，不依赖 Mathlib。

谱系：#42（账本 watch-item）→ #86（HybridAssembly LedgerComp R=Π-A-W 新建）→ #90（R=Π-A-W vs
      取本金三阶段 TW 不同构裁定，machine-checked）→ #93（双账本扩维 + OQ-9 gate 接入 HybridAssembly
      闭环）→ #97（Origin canonical base）→ #101（双账本桥：两端都锚 Origin）→ 本文件 #127（TW native
      重锚进 Origin.TotalWealth，本文件降为 compatibility/bridge theorem 层，open Origin native 而非 legacy）。
-/

import Origin.FullDefinitionStrategy
import Strict.HybridAssembly

namespace NewChanlun.Origin.LedgerBridge

open NewChanlun.Origin (FullDefinitionSystem StrictState LedgerState mkLedger ledgerStep
  hybridStep policyTheta ledger_invariant_preservation hybrid_step_complete_unique
  policy_factors_through_classification)
open Strict.HybridAssembly (AssemblyState AssemblyEvent assembly assemblyStep originFullDef)
open NewChanlun.Origin.TotalWealth (TWState TWEvent twStep TStage LegalTransition
  OQ9Inv LegalChain runLegal)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 R=Π-A-W 端在 Origin 接口下成立（Origin 单账本，originFullDef 承载）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★R=Π-A-W 端 · Origin 接口闭环保恒等（L0，桥①）：
  Origin 接口见证 `originFullDef` 的闭环转移后，ledger 分量满足 Origin 账本恒等 R=Π-A-W。
  直接引 HybridAssembly §8 的接口义务定理——把它暴露在 Origin.LedgerBridge 命名空间，坐实
  「R=Π-A-W 账本在 Origin `FullDefinitionSystem` 接口下逐步保持」。
-/
theorem origin_ledger_inv_preserved (x : StrictState) (e : originFullDef.Event) :
    (hybridStep originFullDef x e).ledger.R =
      (hybridStep originFullDef x e).ledger.Pi
        - (hybridStep originFullDef x e).ledger.A
        - (hybridStep originFullDef x e).ledger.W :=
  Strict.HybridAssembly.originFullDef_transition_preserves_ledger_inv x e

/--
  ★R=Π-A-W 端 · Origin `ledgerStep` 元定理对齐（L0，桥①·原子侧）：
  Origin canonical 的 `ledgerStep`（StrictState.ledger 用的同一算子）对任意增量保 R=Π-A-W。
  引 Origin `ledger_invariant_preservation`——R=Π-A-W 端的原子保持律就是 Origin canonical 自身。
-/
theorem origin_ledgerStep_preserves_inv (L : LedgerState) (dPi dA dW : Int) :
    (ledgerStep L dPi dA dW).R =
      (ledgerStep L dPi dA dW).Pi - (ledgerStep L dPi dA dW).A - (ledgerStep L dPi dA dW).W :=
  ledger_invariant_preservation L dPi dA dW

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 TW 端在 HybridAssembly 闭环下成立（TW 保持限零实现盈亏子域）

  取本金三阶段 TW（#90 与 R=Π-A-W 不同构）由 HybridAssembly `AssemblyState.twState` 闭环承载。
  本节把 #93 的 twState 线程化 / 零实现盈亏时 TW 保持 / stage 单向 / OQ-9 gate 桥到
  Origin.LedgerBridge 命名空间。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★TW 端 · twState 真被 HybridAssembly 闭环线程化（L0，桥②·结构侧，反退化见证①）：
  HybridAssembly 闭环 `assemblyStep` 后的 twState = twStep 真消费订单 twEvent 产出（非恒等挂件）。
  桥 `HybridAssembly.assemblyStep_threads_twState`——TW 端在闭环里真动。
-/
theorem tw_threaded_in_loop (x : AssemblyState) (e : AssemblyEvent) :
    (assemblyStep x e).twState
      = twStep x.twState
          (Strict.HybridAssembly.scheduleAdapter x
            (Strict.HybridAssembly.riskAdapter x
              (Strict.HybridAssembly.intentAdapter x
                (Strict.HybridAssembly.classifyAdapter x
                  (Strict.HybridAssembly.recAdapter x e))))).twEvent :=
  Strict.HybridAssembly.assemblyStep_threads_twState x e

/--
  ★★TW 端 · 零实现盈亏子域保持 TW=free+holding+withdrawn（L0 证明 + L2 外部前提，桥②）：

  无条件版已被生产反例否定（rust Realize 写回两账本、TW 变化 +10/−6 测试在案，2026-07-24 #240
  漂移账）；本定理自 2026-07-25 起为零实现盈亏子域形式。

  在当前 Rust transition 中，`productionRealizedPnl = 0` 意味着不触发 `TwEvent::Realize`；此时只剩
  成本基 TW 事件，复用 Lean 原子域的 `assemblyStep_preserves_tw`。

  ★090 诚实边界：当前 Lean `AssemblyEvent` / `OrderOut` / `TWEvent` 没有 `realized_pnl` /
  `TwEvent::Realize` 对应概念，故不能从 `x`、`e` 内生计算或校验此前提。`productionRealizedPnl`
  及其零证书须由生产桥外部提供；本定理不声明 Lean 已建立 Rust realized-PnL 的端到端对应。
-/
theorem tw_preserved_in_loop (x : AssemblyState) (e : AssemblyEvent)
    (productionRealizedPnl : Int) (hzero : productionRealizedPnl = 0) :
    (assemblyStep x e).twState.tw = x.twState.tw := by
  cases hzero
  exact Strict.HybridAssembly.assemblyStep_preserves_tw x e

/--
  ★★TW 端 · 闭环 stage 单向不可逆（L0，桥②，OQ-9 端B 闭环形式）：
  HybridAssembly 闭环每步 stage rank 只增不减（campaign 内）——CostReduction→CapitalRecovered→
  EarningShares 不回退。桥 `HybridAssembly.assemblyStep_stage_monotone`。
-/
theorem tw_stage_monotone_in_loop (x : AssemblyState) (e : AssemblyEvent) :
    x.twState.stage.rank ≤ (assemblyStep x e).twState.stage.rank :=
  Strict.HybridAssembly.assemblyStep_stage_monotone x e

/--
  ★★TW 端 · OQ-9 gate 闭环保持（L0，桥②，OQ-9 入口证书）：
  进入闭环前 OQ-9 入口证书成立（openLegacyLegs=0），闭环转移后仍成立。桥
  `HybridAssembly.assemblyStep_oq9_gate_preserved`——OQ-9「相位锁死」是 Origin base 下的闭环性质。
-/
theorem tw_oq9_gate_preserved_in_loop (x : AssemblyState) (e : AssemblyEvent)
    (hgate : x.twState.openLegacyLegs = 0) :
    (assemblyStep x e).twState.openLegacyLegs = 0 :=
  Strict.HybridAssembly.assemblyStep_oq9_gate_preserved x e hgate

/--
  ★★TW 端 · OQ-9 端B 闭环里是定理（L0，桥②，扩维消解的 Origin 形式）：
  闭环后态（gate 保持，openLegacyLegs=0）上「闭合旧 legacy 腿」不是合法转移（消费 Origin base 下的
  `LegalTransition`）。桥 `HybridAssembly.assemblyStep_oq9_closure_illegal`——「亏损腿闭合回正」路径
  在 Origin canonical base 的合法闭环里恒不可达（端B 是闭环定理，非一次性声明）。
-/
theorem tw_oq9_closure_illegal_in_loop (x : AssemblyState) (e : AssemblyEvent) (p : Int)
    (hgate : x.twState.openLegacyLegs = 0) :
    ¬ LegalTransition (assemblyStep x e).twState (TWEvent.closeShareLeg p) :=
  Strict.HybridAssembly.assemblyStep_oq9_closure_illegal x e p hgate

/--
  ★★TW 端 · OQ-9 trace 级不变量（L0，桥②·trace 级端B）：
  从满足 OQ9Inv 的初态出发，经任意合法 trace 到 earning 阶段，legacy 腿恒为 0——raw witness 形状
  （earning ∧ legacy 腿=1）在 Origin base 下任何合法 trace 上不可达。桥 TotalWealth.oq9inv_trace 的
  不可达推论，把 trace 级端B 暴露在 Origin.LedgerBridge 命名空间。
-/
theorem tw_oq9_legacy_unreachable_in_earning (s : TWState) (trace : List TWEvent)
    (hinv : OQ9Inv s) (hchain : LegalChain s trace)
    (hst : (runLegal s trace).stage = TStage.earningShares) :
    (runLegal s trace).openLegacyLegs = 0 :=
  NewChanlun.Origin.TotalWealth.oq9_legacy_leg_unreachable_in_earning s trace hinv hchain hst

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 双账本并置 · #90 不同构在 Origin 上被尊重（不糊成一个）

  双账本两端都锚 Origin（R=Π-A-W 在 Origin 接口 / TW 在 HybridAssembly 闭环），但 #90 已证它们
  不同构——本节把不同构的核心反例（TW 三量相同但 stage 不同 ⟹ B→A 投影非单射）桥到 Origin
  命名空间，坐实「双账本桥**不**抹掉不同构」（两端各在其层，非二合一）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★#90 不同构在 Origin base 下被尊重（L0，桥③，督导锋利问题①直答）：
  存在两个 twState 态，TW 三量分量完全相同（free/holding/withdrawn 都相等）、TW 总量相同，仅 stage
  不同（costReduction vs earningShares）——故任何「只读 TW 三量」的 TW→R=Π-A-W 投影**塌缩**二者
  （非单射）。这证 stage 维度承载 R=Π-A-W 无法表达的不可逆信息：双账本桥不是把 TW 折进单账本，
  单账本（R=Π-A-W）无 stage 维度可承载这个区分。桥 `TotalWealth.not_isomorphic_stage_collapses`。
-/
theorem two_ledger_non_isomorphic_stage_collapses : ∃ (s₁ s₂ : TWState),
    s₁.free = s₂.free ∧ s₁.holding = s₂.holding ∧ s₁.withdrawn = s₂.withdrawn
    ∧ s₁.stage ≠ s₂.stage ∧ s₁.tw = s₂.tw :=
  NewChanlun.Origin.TotalWealth.not_isomorphic_stage_collapses

/--
  ★双账本桥总见证 `DualLedgerVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `bothEndsAnchoredNonIsomorphic`——类型层钉死「双账本两端都锚 Origin **且**保持 #90
  不同构（两端各在其层）」。**没有** `Merged` / `RoughlyOneLedger` 构造子——拒绝「把 TW 折进 Origin
  单账本」的声明膨胀（#90 否证之）。
-/
inductive DualLedgerVerdict where
  | bothEndsAnchoredNonIsomorphic
deriving DecidableEq, Repr

/-- ★双账本桥裁定（L0，gatekeeper）：桥裁定必是「两端都锚 Origin 且非同构」。三组桥定理
    （§1 R=Π-A-W 端 / §2 TW 端〔TW 保持限零实现盈亏〕/ §3 不同构尊重）共同支撑。 -/
theorem dual_ledger_verdict_is_both_ends (v : DualLedgerVerdict) :
    v = DualLedgerVerdict.bothEndsAnchoredNonIsomorphic := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 Origin 接口三项义务对齐（HybridAssembly 实现 Origin 接口的桥侧确认）

  把 HybridAssembly §8 的三项 Origin 接口义务（闭环 ∃! / π̄∘C 因子化 / R=Π-A-W 保持）暴露在
  Origin.LedgerBridge 命名空间——坐实「HybridAssembly 实现 Origin FullDefinitionSystem 接口」
  在双账本桥层被确认。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★Origin 接口义务① · 闭环 ∃!（L0，桥④）：originFullDef 的 Origin hybridStep 每步存在唯一。 -/
theorem origin_interface_step_complete_unique (x : StrictState) (e : originFullDef.Event) :
    NewChanlun.Origin.ExistsUnique (fun x' => hybridStep originFullDef x e = x') :=
  Strict.HybridAssembly.originFullDef_step_complete_unique x e

/-- ★Origin 接口义务② · π̄∘C 因子化（L0，桥④）：originFullDef 策略真穿过 classify。 -/
theorem origin_interface_policy_factors (x : StrictState) (e : originFullDef.Event) :
    policyTheta originFullDef x e =
      originFullDef.schedule x
        (originFullDef.risk x
          (originFullDef.intent x
            (originFullDef.classify x (originFullDef.recStruct x e)))) :=
  Strict.HybridAssembly.originFullDef_policy_factors x e

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（cc-ledgerbridge 工位，task #101，A′ Phase2）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. R=Π-A-W 端在 Origin 接口下成立（§1）：`origin_ledger_inv_preserved`（Origin 接口闭环保恒等）+
     `origin_ledgerStep_preserves_inv`（Origin ledgerStep 原子保持律 = Origin canonical 自身）。
  2. TW 端在 HybridAssembly 闭环下按有效域成立（§2，#90 第二账本，并置 Origin 接口）：
     `tw_threaded_in_loop`（twState 真线程化）+ `tw_preserved_in_loop`（零实现盈亏子域保持）+
     `tw_stage_monotone_in_loop`（stage 单向）+ `tw_oq9_gate_preserved_in_loop`（OQ-9 gate 保持）+
     `tw_oq9_closure_illegal_in_loop`（端B 闭环定理，消费 LegalTransition）+
     `tw_oq9_legacy_unreachable_in_earning`（trace 级端B）。
  3. 双账本并置 + #90 不同构被尊重（§3）：`two_ledger_non_isomorphic_stage_collapses`（TW 三量相同但
     stage 不同 ⟹ 投影非单射）+ `dual_ledger_verdict_is_both_ends`（gatekeeper：两端都锚 Origin 且非
     同构，无 Merged 构造子）。
  4. Origin 接口三项义务对齐（§4）：`origin_interface_step_complete_unique`（闭环 ∃!）+
     `origin_interface_policy_factors`（π̄∘C 因子化）——坐实 HybridAssembly 实现 Origin 接口在桥层确认。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ TW 与 R=Π-A-W 可折成单账本（#90 否证之；本文件证两端各在其层，非二合一）。
  - ✗ 非零 realized PnL 下 TW 保持；生产 `TwEvent::Realize` 会使 TW 按已实现盈亏漂移。
  - ✗ Lean 当前能从 `AssemblyState` / `AssemblyEvent` 判定生产 `realized_pnl`（须外部桥提供）。
  - ✗ 盈利/最优/实盘有效（L3）。

  ★双账本桥裁定（重要）：#93 双账本（R=Π-A-W ⊕ 取本金三阶段 TW）+ OQ-9 gate 全部锚到 Origin
  canonical base——R=Π-A-W 端由 Origin `FullDefinitionSystem` 接口（originFullDef）承载，TW/stage/
  OQ-9 端由 HybridAssembly `AssemblyState` 闭环承载。两端并置且 #90 不同构被尊重（Origin 单账本
  `LedgerState` 结构上不吸收 TW，因 #90 不同构——非缺陷，是正确的层分离）。

  谱系：#42 → #86 → #90（不同构裁定）→ #93（双账本扩维入闭环）→ #97（Origin canonical）→
        本文件 #101（双账本桥：两端都锚 Origin，#90 不同构在 Origin 上被尊重）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.LedgerBridge
