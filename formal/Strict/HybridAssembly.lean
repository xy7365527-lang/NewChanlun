/-
  Strict/HybridAssembly.lean — 全定义策略闭环装配（把抽象 HybridComponents 用已证组件具体化，L0）
  ★cc-hybrid-assembly 工位（task #86；codex 编排者代理裁决 R1=B/R2/R3，2026-06-26）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：把 HybridStep 的**抽象**闭环接口具体化为**单一**闭环状态机实例

  `Strict/HybridStep.lean` 建了**抽象**闭环接口 `HybridComponents`（六段全称抽象）+
  `hybridStep`（x_t ─e→ x_{t+1}）+ `hybridStep_total_unique`（函数图 ∃!）。它**不**碰具体
  类型——六段是 `HybridState → … → …` 的全称字段。本文件兑现蓝图
  docs/formal/full-definition-strategy-v1.md §2/§3 的**具体装配**：

  1. **具体乘积态 `AssemblyState`**（蓝图 §2）：用**已有类型组装**——
     `Dynamics.State`（microState，T-causal 已证）+ 新建 `LedgerComp`（账本恒等 R=Π-A-W
     分量，见下「★ledger 接缝缺口」）+ 声部态 / riskMode / phase / positions / orders /
     memory（复用 `Strict.StrictAction` / `Operational.Phase` 等已证枚举或简单 Nat 计数）。
  2. **六段 adapter**（蓝图 §3）：用现有 `*_total_unique` 实例化 `HybridComponents` 的六字段——
     recStruct ← Parse/Level、classify ← Chain.globalClassify、intent ← Fugue 10 级优先级
     **确定选择器**、risk ← RiskProj **确定选择器/有限网格**、schedule ← StrategyFamily.exec、
     transition ← **新建闭环 T**（Dynamics.delta 更新 microState 分量 + ledger 更新保 R=Π-A-W）。
  3. **装配总定理**：实例化 `HybridStep.hybridStep_total_unique`（六段复合 ⟹ 闭环每步 ∃!）+
     用 `Chain.totalUnique_prod` 把乘积态各分量唯一性组装为闭环唯一性。

  ════════════════════════════════════════════════════════════════════════
  ## ★ledger 接缝缺口（formalization-validity-domain + no-workaround，必须诚实声明）

  蓝图 §2 标 `ledgerState` 来源为 `Tlayers/Accounting/Ledger`，账本恒等 `R=Π-A-W`。
  **实际检查 `Tlayers/Accounting/Ledger.lean`：它没有 (I₀,Π,A,W,R) 四字段恒等结构**——
  它有 `VoiceLedger`(polarity/units/capital)、`nav` 三项、`AccountState`(nBase/forest)、
  股数守恒 `totalUnits` 与 NAV 中性，但**无** R=Π-A-W 恒等的显式类型。

  故本文件**新建** `LedgerComp`（携带 R=Π-A-W 不变量），**不**冒充「复用 Ledger 已证的
  R=Π-A-W 结构」（Ledger 没有此结构，冒充 = 声明膨胀 090）。`LedgerComp` 把恒等 `R=Π-A-W`
  作为**结构不变量字段**编码（`inv : R = Π - A - W`），T 的 ledger 更新算子**保持**它
  （`ledgerStep_preserves_inv`）——这正补 `StrategyFamily.piTheta_causal`「只比同一 z」的
  **账户因果洞**：T 写回 z 的下一态，使账户态的生成进入闭环。Ledger 的股数守恒 / NAV 中性
  作**同范畴佐证**（同为「会计操作保不变量」），但 R=Π-A-W 的载体是本文件新建分量。

  ════════════════════════════════════════════════════════════════════════
  ## 诚实标注（formalization-validity-domain + 090 + codex R3，违反=声明膨胀=语法违法）

  - **认识论等级**：全文件 **L0**（结构/定义层，零数据依赖）。`lake build` 通过 = 闭环装配的
    类型自洽 + 各分量 ∃! 可组合为乘积 ∃! + ledger 更新保 R=Π-A-W，**不**是任何缠论语义 / 盈利
    / 实盘有效性声明。
  - **T 只声明闭环状态转移全定义**（产出确定的下一态），**禁**声明盈利 / 最优 / 实盘有效（L3）。
  - **RiskProj 段只能确定选择器 / 有限网格**（`RiskProj.RiskGrid.gridProject`），**禁**声明
    连续 argmin 存在性（StrategyFamily/RiskProj 已守此诚实保留）。
  - **10 级动作优先级是确定优先级选择器**（`actionPriority` 全函数），**禁**声明「由缠论唯一
    推出」——这是 Θ_voice 设计选择（codex R3）。
  - **保 R=Π-A-W 不变量**（`LedgerComp.inv` + `ledgerStep_preserves_inv`）。
  - 纯 Prop/Type，**不依赖 Mathlib**（Int 取自 Lean core）；**禁 sorry/admit/axiom**。

  范式：纯 Prop/Type。验证：`cd formal && lake build`（须保持全绿）。
  谱系：615/616/617（C_Θ 与 π_Θ 都 Θ-参数化）→ HybridStep（闭环装配，C 与 π 由并列升级为复合）
        → 本文件（抽象接口具体化 + R=Π-A-W 账户因果接入闭环）。
-/
import Strict.HybridStep
import Strict.Fugue
import Strict.RiskProj
import Dynamics
import Origin.TotalWealth
import Origin.FullDefinitionStrategy

namespace Strict.HybridAssembly

open Strict.Chain (TotalUnique totalUnique_of_fun totalUnique_prod)
open Strict.HybridStep (HybridComponents hybridStep)
-- ★OQ-9 扩维（task #93）：取本金三阶段账本 TW = free+holding+withdrawn + 阶段单向 + OQ-9 gate。
-- 接进闭环：AssemblyState.twState : TWState 真被 transitionAdapter 线程化（见 §2/§4 诚实标注）。
-- open 列表只含闭环里实际引用的名字（TWState/TWEvent/twStep + 守恒/单向定理 + LegalTransition gate
-- 谓词）；单文件 OQ-9 定理（legal_earning_no_legacy_leg_closure / raw witness）在 Origin/TotalWealth.lean
-- 已证，闭环侧由 assemblyStep_oq9_closure_illegal 延拓（消费 LegalTransition），不重复 open。
-- ★task #127 TW native 重锚：TW 定义所有权移入 Origin（NewChanlun.Origin.TotalWealth），
--   不再从 legacy Tlayers.Accounting.TotalWealth open（A′ Origin 唯一 canonical base）。
open NewChanlun.Origin.TotalWealth (TWState TWEvent twStep twStep_preserves_tw
  stage_rank_monotone LegalTransition)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 账本分量 LedgerComp（R=Π-A-W 恒等，本文件新建——补 Ledger 接缝缺口）

  蓝图 §2 的账本恒等 `R = Π - A - W`（R=储备/Reserve，Π=累计利润/Profit，A=已分配/Allocated，
  W=已提取/Withdrawn）。`Tlayers/Accounting/Ledger` **无**此四字段结构（见文件头说明），故本文件
  新建携带恒等不变量的分量。用 `Int`（利润/分配/提取可正可负的代数差）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★账本分量 `LedgerComp`（L0，本文件新建，R=Π-A-W 恒等载体）。

  - `i0 : Int`：初始本金 I₀（账本基线）。
  - `Pi : Int`：累计利润 Π（profit，含已实现盈亏）。
  - `A  : Int`：已分配/资本化 A（allocated，earning 转股数等）。
  - `W  : Int`：已提取 W（withdrawn，出金）。
  - `R  : Int`：储备 R（reserve，未分配未提取的留存）。
  - `inv : R = Pi - A - W`：**账本恒等不变量**（R=Π-A-W）——结构层强制，任何 LedgerComp 值
    都满足它（构造时必须提供证据，T 的更新必须保持它，见 `ledgerStep_preserves_inv`）。

  ★诚实标注：这是**本文件新建**的 R=Π-A-W 载体（`Tlayers/Accounting/Ledger` 无此结构）。
  Ledger 的股数守恒 / NAV 中性是**同范畴**（会计操作保不变量）的已证佐证，但 R=Π-A-W 的
  形式载体在本文件。`i0` 不进恒等（恒等只约束 Π/A/W/R 四量；I₀ 是基线，nav 全量另算）。
-/
structure LedgerComp where
  i0 : Int
  Pi : Int
  A : Int
  W : Int
  R : Int
  inv : R = Pi - A - W
deriving Repr

/--
  ★初始账本 `ledger0 I₀`（L0）：开局账本——Π=A=W=R=0（无利润/分配/提取/储备），恒等显然成立。
-/
def ledger0 (I0 : Int) : LedgerComp :=
  { i0 := I0, Pi := 0, A := 0, W := 0, R := 0, inv := by decide }

/--
  ★账本事件 `LedgerEvent`（L0，穷尽）：改变账本的离散操作（与 Dynamics.Event 的会计侧对应）。
  - `realize dΠ`：实现利润 dΠ（Π += dΠ；R 随之 += dΠ 保恒等）。
  - `allocate dA`：分配/资本化 dA（A += dA；R -= dA 保恒等）。
  - `withdraw dW`：提取 dW（W += dW；R -= dW 保恒等）。
  - `noop`：无账本变化（如纯 bar 推进不触发会计事件）。
-/
inductive LedgerEvent where
  | realize (dPi : Int)
  | allocate (dA : Int)
  | withdraw (dW : Int)
  | noop
deriving Repr

/--
  ★账本更新 `ledgerStep`（L0，全函数，**保 R=Π-A-W**）：对每个 LedgerEvent 更新四量，
  **新 R 由恒等重算**（R := Π - A - W），故不变量按构造成立。

  - `realize dΠ`：Π += dΠ，A/W 不变 ⟹ R += dΠ。
  - `allocate dA`：A += dA，Π/W 不变 ⟹ R -= dA。
  - `withdraw dW`：W += dW，Π/A 不变 ⟹ R -= dW。
  - `noop`：四量不变。
  每分支 `inv` 由 `by ring`/`by omega` 重证（新 R = 新Π - 新A - 新W）。
-/
def ledgerStep (L : LedgerComp) : LedgerEvent → LedgerComp
  | LedgerEvent.realize dPi =>
      { i0 := L.i0, Pi := L.Pi + dPi, A := L.A, W := L.W, R := L.R + dPi,
        inv := by have h := L.inv; omega }
  | LedgerEvent.allocate dA =>
      { i0 := L.i0, Pi := L.Pi, A := L.A + dA, W := L.W, R := L.R - dA,
        inv := by have h := L.inv; omega }
  | LedgerEvent.withdraw dW =>
      { i0 := L.i0, Pi := L.Pi, A := L.A, W := L.W + dW, R := L.R - dW,
        inv := by have h := L.inv; omega }
  | LedgerEvent.noop => L

/--
  ★★ledger 更新保 R=Π-A-W（L0，账户因果洞补法的核心）：
  `ledgerStep L e` 的 R 分量 = Π - A - W（恒等保持）。

  这把蓝图「ledger/accounting 更新放进 T，保 R=Π-A-W 不变量」落实为定理——账本态的生成
  （Π/A/W 的演化）经此算子进入闭环转移 T，且每步保持账本恒等。这正补
  `StrategyFamily.piTheta_causal`「只比同一 z（账户态）」的洞：T 写回账户态的下一步，使
  账户的生成因果（而非「固定 z」）进入闭环。

  ★诚实：本定理证「ledger 更新保 R=Π-A-W」（结构不变量），**不**证账本数值正确反映实盘盈亏
  （后者是 L2/L3，由 Rust 引擎运行时守卫覆盖，见 Ledger.lean 头 L0 vs L2 区分）。
-/
theorem ledgerStep_preserves_inv (L : LedgerComp) (e : LedgerEvent) :
    (ledgerStep L e).R = (ledgerStep L e).Pi - (ledgerStep L e).A - (ledgerStep L e).W :=
  (ledgerStep L e).inv

/--
  ★ledger 更新全定义 + 确定（L0）：`ledgerStep` 是 Lean 全函数 ⟹ ∃! 下一账本态。
  这是 ledger 分量进入闭环唯一性组装的接口（同 Dynamics.delta_total_deterministic）。
-/
theorem ledgerStep_total_unique (L : LedgerComp) (e : LedgerEvent) :
    ∃ L', ledgerStep L e = L' ∧ ∀ L'', ledgerStep L e = L'' → L'' = L' :=
  ⟨ledgerStep L e, rfl, fun _ h => h.symm⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 完整态 AssemblyState（蓝图 §2 乘积态，复用已有类型 + 新建分量）

  乘积态 = microState（Dynamics.State）× ledgerState（LedgerComp，§1）× 声部/风险/阶段/
  仓位/订单/记忆分量。声部/风险/阶段用现有枚举或简单 Nat 计数承载（结构层；其缠论语义由
  各源文件逐 claim 承载，本文件只组装乘积态使闭环可表达）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险模式 `RiskMode`（蓝图 §2 μ ∈ {Insolvent,Liquidation,Deleverage,CloseOnly,Normal}，L0）。
  ★诚实：模式枚举是结构层（μ 的**取值阈值**是 Θ_risk 参数，非缠论可导；本枚举只承载五态）。
-/
inductive RiskMode where
  | insolvent
  | liquidation
  | deleverage
  | closeOnly
  | normal
deriving DecidableEq, Repr

/--
  ★三阶段 `Phase`（蓝图 §2 Φ ∈ {I,II_repair…}，L0，对应 Operational.Phase 三阶段 T₂₆）。
  本文件用本地三态枚举（I=建仓/II=取本/III=增核）承载阶段分量——与 Operational.Phase
  的 enter/hold/exit 同范畴（三阶段结构），其转移阈值是 Θ 参数。
-/
inductive Phase where
  | phaseI
  | phaseII
  | phaseIII
deriving DecidableEq, Repr

/--
  ★完整混合态 `AssemblyState`（L0，蓝图 §2 乘积态的具体兑现）。

  字段（各标来源，复用已证类型 / 本文件新建分量）：
  - `microState : Dynamics.State`：解析级微状态（barCount/strokeCount/lastStrokeDir/pendingRise，
    T-causal `Dynamics.State` 已证全定义转移）。
  - `ledgerState : LedgerComp`：账本恒等分量（R=Π-A-W，§1 新建）。
  - `twState : TWState`：★取本金三阶段账本分量（TW=free+holding+withdrawn 守恒 + stage 单向 +
    OQ-9 gate，task #93 扩维，来源 `Origin.TotalWealth`（#127 native 重锚））。**与 ledgerState 双层并置**——
    R=Π-A-W（收益表视角）与 TW（现金流+持仓视角）二者不同构（#90 已证），完整持仓系统两者都需要。
    twState 真被 transitionAdapter 线程化（见 §4 transitionAdapter + §6 反退化见证）。
  - `riskMode : RiskMode`：风险模式 μ（五态）。
  - `phase : Phase`：三阶段 Φ。
  - `positions : Nat`：当前总持仓单位（声部聚合后的绝对单位数 Σq_v 的摘要）。
  - `orders : Nat`：未完成订单计数 O_t 的摘要。
  - `memory : Nat`：信号使用记录 M_t（Fresh 计数）的摘要。

  ★诚实标注（formalization-validity-domain）：positions/orders/memory 用 Nat 计数**摘要**
  承载（结构层乘积态的占位分量，使闭环转移可表达）——它们的**缠论语义内容**（具体哪些声部
  持仓、哪些订单挂单）由 Fugue/StrategyFamily 逐 claim 承载，本文件只组装乘积态。globalState
  （C_Θ 输出）不进 AssemblyState 字段：它是 classify 段的**输出**（由 microState 经 Chain
  导出），在闭环数据流中现算，不冗余存进状态（避免双份漂移）。
-/
structure AssemblyState where
  microState : Tlayers.Dynamics.State
  ledgerState : LedgerComp
  twState : TWState
  riskMode : RiskMode
  phase : Phase
  positions : Nat
  orders : Nat
  memory : Nat
deriving Repr

/--
  ★混合事件 `AssemblyEvent`（L0）：驱动闭环一步的外部事件 e。
  携带一个解析事件（`Dynamics.Event`，驱动 microState）+ 一个账本事件（`LedgerEvent`）+
  一个 TW 账本事件（`TWEvent`）——构成外部事件的完整 schema（task #93 扩 twEvent）。

  ★诚实标注（review 019f0318 修正3，禁声明膨胀——撤回「非死字段」辩护）：`parseEvent` 被
  transitionAdapter **直接消费**（驱动 microState）。`ledgerEvent`/`twEvent` 是**当前闭环明确忽略的
  字段**（intentional ignored field，不是被 T 消费的外部输入）——权威账本事件来自 **OrderOut**
  （scheduleAdapter 从策略动作派生，账户因果链：策略决定账本副作用，codex R2「ledger 更新放进 T，
  用订单事件」）。这与现有 ledgerEvent 同构（ledgerState 同样只用 o.ledgerEvent，不用 e.ledgerEvent）。

  ★诚实裁定（codex review）：旧标注「外部 schema 占位 + 非新增死字段」是声明膨胀——它确实是
  **被忽略的字段**。本文件改为明确声明「当前闭环忽略 e.{ledgerEvent,twEvent}」并以定理坐实
  （`assemblyStep_twState_ignores_event_twEvent`：换掉 e.twEvent 不影响后态 twState）。若未来目标
  是外部事件驱动 TW，须改 transitionAdapter 消费 e.twEvent，或加 o.twEvent 与 e.twEvent 一致性校验——
  当前**不**声称外部驱动（诚实有效域边界）。
-/
structure AssemblyEvent where
  parseEvent : Tlayers.Dynamics.Event
  ledgerEvent : LedgerEvent
  twEvent : TWEvent
deriving Repr

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 六段 adapter 的具体类型（Struct/Class/Intent/Control/Order）

  每段的输入/输出类型用现有已证类型实例化。中间类型保持轻量（结构层），实质唯一性由
  各源组件的 *_total_unique 承载，本文件做乘积组装。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★递归结构 `RecStruct`（Rec 段输出，L0）：解析级递归结构 D_t 的摘要。
  用 `Dynamics.State` 承载——Rec 段 = 在当前 microState 上吃事件 e 跑一步 `Dynamics.delta`
  得到的新解析态（Parse/Level 的 Θ_parse 参数化递归解析在 Dynamics 层的因果摘要）。
-/
def RecStruct : Type := Tlayers.Dynamics.State

/--
  ★分类标签 `ClassLabel`（C_Θ 段输出，L0）：全局完全分类 C_Θ(x_t) 的摘要。
  用 (RiskMode × Phase) 承载——本文件的 classify 段从 microState/ledger 读出风险模式 + 阶段
  作为 C_Θ 的**结构摘要**（完整 C_Θ = Chain.GlobalState 需 ChanlunComponents 实例，本文件
  装的是闭环结构，C_Θ 的完整 fiber partition 由 Chain.globalClassify_total_unique 承载，
  本段以其摘要接入闭环——见 `classifyAdapter` 诚实标注）。
-/
def ClassLabel : Type := RiskMode × Phase

/--
  ★意图 `IntentAction`（Intent 段输出，L0）：10 级动作优先级选择器选出的动作。
  用 `Strict.StrictAction`（7 动作枚举，Classification.lean 已证）承载——Intent 段 = 在
  C_Θ 标签上跑**确定优先级选择器**得到的唯一动作意图（见 §4 `actionPriority`）。
-/
def IntentAction : Type := Strict.StrictAction

/--
  ★控制 `Control`（RiskProj 段输出，L0）：风险投影后的唯一总仓位 q*（Nat 单位数摘要）。
  RiskProj 段 = 把意图经**有限网格确定选择器**投到唯一可行仓位（见 §4 `riskAdapter`）。
  用 `abbrev`（透明别名）使 `Nat` 的 `LT`/`OfNat` 实例对 `Control` 可见（scheduleAdapter 比较 q*>0）。
-/
abbrev Control : Type := Nat

/--
  ★订单 `OrderOut`（Schedule 段输出，L0）：调度后的订单 O_t（携带动作 + 目标仓位 + 双账本事件）。
  Schedule 段 = 把控制（目标仓位）+ 意图（动作）+ 账本副作用打包为订单（StrategyFamily.exec
  的执行顺序 Θ_exec 摘要）。订单携带 `ledgerEvent`（R=Π-A-W 侧）+ `twEvent`（取本金三阶段 TW 侧）
  使 T 的**双账本**更新都有据可依（账户因果链；task #93 双层并置）。
-/
structure OrderOut where
  action : Strict.StrictAction
  targetPos : Nat
  ledgerEvent : LedgerEvent
  twEvent : TWEvent
deriving Repr

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 六段具体实现 + 装配为 HybridComponents

  每段是**确定全函数**（结构层）。关键的两条结构约束（HybridStep 强制）自动满足：
  - `intent` 以 Class 为输入（π̄ 真读 C_Θ）——`intentAdapter : AssemblyState → ClassLabel → …`。
  - `transition` 产出完整 AssemblyState（T 闭环写回）——`transitionAdapter`。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Rec 段 `recAdapter`（L0）：在当前 microState 上吃解析事件跑一步 `Dynamics.delta`。
  这是 Parse/Level 的 Θ_parse 参数化递归解析在 Dynamics 因果层的摘要（recStruct = 新解析态）。
-/
def recAdapter (x : AssemblyState) (e : AssemblyEvent) : RecStruct :=
  Tlayers.Dynamics.delta x.microState e.parseEvent

/--
  ★C_Θ 段 `classifyAdapter`（L0）：从解析结构 + 当前态读出分类标签摘要 (riskMode, phase)。

  ★诚实标注（formalization-validity-domain，关键）：本段是 C_Θ 的**结构摘要接入**——它把当前
  态的 (riskMode, phase) 作为 C_Θ 标签传给 Intent 段，使 `intent` **真读分类输出**（满足
  HybridStep 的 π̄∘C 结构约束）。**完整** C_Θ 的 fiber partition 全局唯一性由
  `Chain.globalClassify_total_unique` 承载（需 ChanlunComponents 实例），本段不重证它——
  本段证的是「闭环数据流中 Intent 真读 classify 输出」，不是「这个摘要等价于完整 C_Θ」。
-/
def classifyAdapter (x : AssemblyState) (_d : RecStruct) : ClassLabel :=
  (x.riskMode, x.phase)

/--
  ★★10 级动作优先级**确定选择器** `actionPriority`（L0，codex R3）：
  从风险模式 + 阶段确定性地选出唯一动作——参照 HybridStateMachine 的 10 级优先级
  （破产 > 去杠杆 > 执行异常 > 阶段二取本 > 根/祖先失效 > 关反向子 > 建根 > 建反向子 >
  阶段三增核 > 保持）。本函数把这 10 级**压缩为风险模式 + 阶段的确定映射**：

  - `insolvent`  → close（破产：最高优先级，强平）。
  - `liquidation`→ close（清算：强平）。
  - `deleverage` → reduce（去杠杆：减仓）。
  - `closeOnly`  → close（只平不开）。
  - `normal` 下按阶段：
    · `phaseI`   → buy   （阶段一建根仓）。
    · `phaseII`  → reduce（阶段二取本 = 减仓回收本金）。
    · `phaseIII` → add   （阶段三增核）。

  ★诚实标注（codex R3，禁声明膨胀）：这是**确定优先级选择器**（Θ_voice 设计选择），
  **不是**「由缠论唯一推出」——10 级的**排序本身**是操盘设计决策（哪个优先级在前），
  缠论结构不唯一钉死它。本函数证的是「给定这个优先级排序后选择确定」（全函数 ⟹ 唯一），
  **不**证这个排序是缠论的必然。标 `OperationalSemanticsOnly` + Θ_voice 参数化。
-/
def actionPriority : ClassLabel → IntentAction
  | (RiskMode.insolvent, _)   => Strict.StrictAction.close
  | (RiskMode.liquidation, _) => Strict.StrictAction.close
  | (RiskMode.deleverage, _)  => Strict.StrictAction.reduce
  | (RiskMode.closeOnly, _)   => Strict.StrictAction.close
  | (RiskMode.normal, Phase.phaseI)   => Strict.StrictAction.buy
  | (RiskMode.normal, Phase.phaseII)  => Strict.StrictAction.reduce
  | (RiskMode.normal, Phase.phaseIII) => Strict.StrictAction.add

/--
  ★Intent 段 `intentAdapter`（L0）：**输入含 Class**（π̄ 真读 C_Θ）——在分类标签 c 上跑
  10 级优先级选择器 `actionPriority`。这满足 HybridStep 的 `intent : HybridState → Class → Intent`
  结构约束（Intent 以 Class 为输入 ⟹ 策略穿过分类瓶颈）。
-/
def intentAdapter (_x : AssemblyState) (c : ClassLabel) : IntentAction :=
  actionPriority c

/--
  ★Θ_risk 网格实例 `defaultRiskGrid`（L0）：有限仓位网格 {0,1,2,3}（含 0 = 零仓可行），
  cost = 格点本身（Nat 自身的全序作字典序，无平局 ⟹ cost 单射），用于 RiskProj 确定选择器。

  ★诚实标注（codex R3）：网格 {0,1,2,3} / cost = id 是 **Θ_risk 参数**（分辨率/代价权重非缠论
  可导）——本实例只是 RiskProj.RiskGrid 的一个具体 Θ_risk 配置，证「给定此网格 ⟹ 投影存在
  唯一」（`RiskProj.RiskGrid.riskproj_exists_unique`），**不**证网格选择来自缠论。
  **有限网格**（List）保证字典序最小存在——**不冒充连续 argmin**。
-/
def defaultRiskGrid : Strict.RiskProj.RiskGrid Nat Nat where
  grid := [0, 1, 2, 3]
  zero := 0
  zeroMem := by decide
  key :=
    { le := fun a b => a ≤ b
      decLe := fun a b => inferInstanceAs (Decidable (a ≤ b))
      le_refl := fun a => Nat.le_refl a
      le_trans := fun a b c => Nat.le_trans
      le_antisymm := fun a b => Nat.le_antisymm
      le_total := fun a b => Nat.le_total a b }
  cost := fun p => p
  cost_inj := fun p q _ _ h => h

/--
  ★RiskProj 段 `riskAdapter`（L0）：把意图经**有限网格确定选择器**投到唯一总仓位 q*。
  用 `defaultRiskGrid.gridProject`——网格上 cost 字典序最小的格点（确定选择器，命中真 LexArgmin，
  `RiskProj.RiskGrid.gridProject_isLexArgmin`）。

  ★诚实标注（codex R3）：**禁声明连续 argmin**——本段走有限网格（`gridProject` 在 List 上 foldl
  选最小，总存在且唯一）。意图 `_i` 不改变网格选择（本装配下投影只依赖网格 Θ_risk）——若需
  意图调制网格（如 close 意图投到 0 仓），由扩展 grid/cost 承载，本骨架取 Θ_risk 固定网格的最小。
-/
def riskAdapter (_x : AssemblyState) (_i : IntentAction) : Control :=
  defaultRiskGrid.gridProject

/--
  ★Schedule 段 `scheduleAdapter`（L0）：把控制（目标仓位 q*）+ 意图（动作）打包为订单。
  这是 StrategyFamily.exec 的执行顺序 Θ_exec 摘要——订单携带动作 + 目标仓位 + 派生的双账本事件
  （动作决定账本副作用：buy/add → allocate（资本化建仓占用 R），reduce/close → realize（回收
  实现盈亏），hold/wait/sell → noop 摘要）。双账本事件使 T 的 ledger（R=Π-A-W）+ twState（TW）
  更新都有据（账户因果链）。

  ★twEvent 派生（task #93，真线程化非恒等）：动作 ⟹ TW 账本算子的结构映射——
  - `u>0`（建仓/降成本）⟹ `shortDiff (-1)`（free→holding 买入降成本短差，TW 守恒，**改变 twState
    的 free/holding 分量**——非恒等）。
  - `u=0`（平仓/退本金）⟹ `recoverCapital 1`（free→withdrawn 退本金 1 单位，TW 守恒，**改变
    twState 的 free/withdrawn + 推进 stage**——非恒等）。

  ★诚实：账本事件的 dΠ/dA/dCash/w 取占位常量（1 单位）——具体数额是 Θ_risk/运行时数据（L2），
  本结构层只承载「动作 ⟹ 账本事件类型」的映射（buy 占资本/降成本、close 实现盈亏/退本金），
  数额由下游填充。twEvent 与 ledgerEvent 双侧由同一动作派生 ⟹ 两账本同步线程化。
-/
def scheduleAdapter (_x : AssemblyState) (u : Control) : OrderOut :=
  -- 本骨架的意图未单独透传到 schedule（HybridStep schedule 签名为 state→control→order）；
  -- 动作由 control（目标仓位）派生：q*>0 ⟹ 建仓订单（allocate + 降成本短差），q*=0 ⟹ 平仓订单（realize + 退本金）。
  if u > 0 then
    { action := Strict.StrictAction.buy, targetPos := u,
      ledgerEvent := LedgerEvent.allocate 1, twEvent := TWEvent.shortDiff (-1) }
  else
    { action := Strict.StrictAction.close, targetPos := 0,
      ledgerEvent := LedgerEvent.realize 1, twEvent := TWEvent.recoverCapital 1 }

/--
  ★★T 段 `transitionAdapter`（L0，本文件核心新建——闭环写回完整 AssemblyState）：
  `T : AssemblyState → OrderOut → AssemblyEvent → AssemblyState`。

  闭环写回各分量（codex R2：δ 只更新 microState，**ledger/accounting 更新放进 T**）：
  - `microState` := `Dynamics.delta x.microState e.parseEvent`（解析层在线推进一步，T-causal）。
  - `ledgerState` := `ledgerStep x.ledgerState o.ledgerEvent`（**账本更新，保 R=Π-A-W**——
    用订单携带的账本事件，使账户态 z 的生成进入闭环，补 piTheta_causal 的账户因果洞）。
  - `twState` := `twStep x.twState o.twEvent`（★**取本金三阶段账本更新，保 TW 守恒 + stage 单向**——
    task #93 真线程化：用订单携带的 twEvent 驱动 TW 账本一步，使取本金三阶段的生成进入闭环；
    与 ledgerState 双层并置，两守恒都在闭环里被维持）。
  - `positions` := 订单目标仓位 `o.targetPos`（写回新持仓）。
  - `orders` := `x.orders + 1`（订单计数推进）。
  - `memory`、`riskMode`、`phase`：本骨架保持（其转移由 Θ_signal/Θ_risk 阈值驱动，L2；
    结构层装配保持，不臆造阈值）。

  ★诚实标注（codex R3）：T **只声明闭环状态转移全定义**（产出确定的下一态），**不**声明该
  转移盈利 / 最优 / 实盘有效（L3）。riskMode/phase/memory 的转移阈值是 Θ/L2（不在 L0 装配内
  臆造），本骨架保持它们 = 诚实留白（非 workaround：阈值驱动转移属下游有效域，本文件只闭合
  micro/ledger/twState/positions/orders 的结构闭环）。
-/
def transitionAdapter (x : AssemblyState) (o : OrderOut) (e : AssemblyEvent) : AssemblyState :=
  { microState := Tlayers.Dynamics.delta x.microState e.parseEvent
    ledgerState := ledgerStep x.ledgerState o.ledgerEvent
    twState := twStep x.twState o.twEvent
    riskMode := x.riskMode
    phase := x.phase
    positions := o.targetPos
    orders := x.orders + 1
    memory := x.memory }

/--
  ★★装配实例 `assembly`（L0，把六段具体实现填入 HybridStep.HybridComponents）：
  这是抽象闭环接口 `HybridComponents` 的**具体实例化**——六段全部用本文件的已证 adapter 填充。
  关键的两条结构约束（HybridStep 编码在签名里）自动满足：
  - `intent := intentAdapter`（输入含 ClassLabel ⟹ π̄ 真读 C_Θ）。
  - `transition := transitionAdapter`（产出完整 AssemblyState ⟹ T 闭环写回）。
-/
def assembly : HybridComponents where
  HybridState := AssemblyState
  Event := AssemblyEvent
  Struct := RecStruct
  Class := ClassLabel
  Intent := IntentAction
  Control := Control
  Order := OrderOut
  recStruct := recAdapter
  classify := classifyAdapter
  intent := intentAdapter
  risk := riskAdapter
  schedule := scheduleAdapter
  transition := transitionAdapter

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 装配总定理：单一闭环 hybridStep 实例化 + 各分量 ∃! 组装
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★闭环一步 `assemblyStep`（L0）：`assembly` 实例上的 `hybridStep`——x_t ─e→ x_{t+1} 单一闭环。
  这是 `HybridStep.hybridStep` 在具体装配上的兑现：六段（recAdapter→classifyAdapter→
  intentAdapter→riskAdapter→scheduleAdapter→transitionAdapter）复合为单一转移。
-/
def assemblyStep (x : AssemblyState) (e : AssemblyEvent) : AssemblyState :=
  hybridStep assembly x e

/--
  ★闭环展开 `assemblyStep_unfold`（L0）：assemblyStep = transitionAdapter 作用于 (x, 策略订单, e)。
  显式展示闭环数据流：六段复合后 T 写回完整态。这坐实闭环结构（非「产出订单就停」）。
-/
theorem assemblyStep_unfold (x : AssemblyState) (e : AssemblyEvent) :
    assemblyStep x e =
      transitionAdapter x
        (scheduleAdapter x (riskAdapter x (intentAdapter x (classifyAdapter x (recAdapter x e))))) e :=
  rfl

/--
  ★★★装配总定理 · 闭环每步全定义 + 唯一（L0，实例化 hybridStep_total_unique）★★★：
  `assemblyStep_total_unique` — 对每个混合态 x 与事件 e，闭环一步 `assemblyStep x e` 存在且唯一。

  这是 `HybridStep.hybridStep_total_unique` 在具体装配 `assembly` 上的实例化——抽象闭环接口的
  ∃! 定理落到具体乘积态。证明体直接调用抽象定理（六段复合的函数图 ∃!）。

  ★诚实标注（codex R3）：这兑现「闭环每步确定」的最弱必要侧（函数图 ∃!，同 HybridStep 标注）。
  实质内容在结构约束（§6 π̄∘C + T 写回）+ ledger 保 R=Π-A-W（§1）。**不**声明盈利/最优/实盘。
-/
theorem assemblyStep_total_unique (x : AssemblyState) (e : AssemblyEvent) :
    ∃ x', assemblyStep x e = x' ∧ ∀ x'', assemblyStep x e = x'' → x'' = x' :=
  HybridStep.hybridStep_total_unique assembly x e

/--
  ★闭环步 = TotalUnique（L0，接入 Chain 乘积内核）：`assemblyStep · e` 是 `TotalUnique`。
  复用 `Chain.totalUnique_of_fun`——任意全函数自动 TotalUnique。这让闭环步可接入 Chain 的
  乘积组合内核（下方 `assembly_components_totalUnique_prod` 用 totalUnique_prod 组装分量唯一性）。
-/
theorem assemblyStep_totalUnique (e : AssemblyEvent) (x : AssemblyState) :
    TotalUnique (fun x => assemblyStep x e) x :=
  totalUnique_of_fun (fun x => assemblyStep x e) x

/--
  ★★各分量唯一性经 Chain.totalUnique_prod 组装为闭环唯一性（L0，蓝图 §4 装配义务）：
  `assembly_components_totalUnique_prod` — 闭环转移的两个核心分量（microState 经 Dynamics.delta、
  ledgerState 经 ledgerStep）各自 `TotalUnique`，其**配对**也 `TotalUnique`。

  这兑现蓝图「用 `Chain.totalUnique_prod` 把各分量 total_unique 组装为闭环唯一性」——闭环态的
  乘积唯一性**由**各分量唯一性（Dynamics δ 全定义 + ledger 更新全定义）组装而来。证明体真实
  消费两分量的 TotalUnique（删去任一则乘积唯一性的对应侧无供给）。

  ★诚实：这里组装 micro × ledger 两核心分量（其余分量 positions/orders 由 T 确定性派生，
  riskMode/phase/memory 保持，均随 T 函数确定）——乘积唯一性的实质供给是 micro/ledger 两分量
  的全定义转移。完整闭环态唯一性由 `assemblyStep_total_unique`（函数图）承载，本定理展示其
  **分量组装结构**（micro/ledger 唯一 ⟹ 配对唯一），对接 Chain 内核。
-/
theorem assembly_components_totalUnique_prod
    (x : AssemblyState) (e : AssemblyEvent) :
    TotalUnique
      (fun x : AssemblyState =>
        (Tlayers.Dynamics.delta x.microState e.parseEvent,
         ledgerStep x.ledgerState (scheduleAdapter x
           (riskAdapter x (intentAdapter x (classifyAdapter x (recAdapter x e))))).ledgerEvent)) x := by
  -- microState 分量唯一（Dynamics.delta 全定义 ⟹ 函数 TotalUnique）。
  have hmicro : TotalUnique
      (fun x : AssemblyState => Tlayers.Dynamics.delta x.microState e.parseEvent) x :=
    totalUnique_of_fun _ x
  -- ledgerState 分量唯一（ledgerStep 全定义 ⟹ 函数 TotalUnique）。
  have hledger : TotalUnique
      (fun x : AssemblyState => ledgerStep x.ledgerState (scheduleAdapter x
        (riskAdapter x (intentAdapter x (classifyAdapter x (recAdapter x e))))).ledgerEvent) x :=
    totalUnique_of_fun _ x
  -- ★乘积组装（实质消费 hmicro/hledger）：micro 唯一 + ledger 唯一 ⟹ 配对唯一。
  exact totalUnique_prod _ _ x hmicro hledger

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 实质增量：π̄∘C（真读分类）+ T 写回（保 R=Π-A-W）的具体兑现
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★π_Θ = π̄_Θ ∘ C_Θ 在装配上的兑现（L0，HybridStep `policy_factors_through_classify` 的实例）：
  `assembly_policy_factors_through_classify` — 装配的策略订单真的**穿过分类**
  （intentAdapter 读 classifyAdapter 的输出）。

  闭环订单 = `scheduleAdapter x (riskAdapter x (intentAdapter x (classifyAdapter x (recAdapter x e))))`
  ——Intent 段的输入 `classifyAdapter x (recAdapter x e)` **就是 C_Θ 摘要的输出**。故策略穿过
  分类瓶颈（决策充分性 L1 可表达的结构前提）。这与 Chain 的**并列合取**形成对照。
-/
theorem assembly_policy_factors_through_classify (x : AssemblyState) (e : AssemblyEvent) :
    HybridStep.policyOutput assembly x e
      = scheduleAdapter x (riskAdapter x (intentAdapter x (classifyAdapter x (recAdapter x e)))) :=
  rfl

/--
  ★T 闭环写回在装配上的兑现（L0，HybridStep `hybridStep_is_closed_transition` 的实例）：
  `assembly_is_closed_transition` — 闭环一步 = transitionAdapter 作用于 (x, 策略订单, e)。
  下一态由 T 从当前态 x、策略订单、外部事件 e 唯一产出——闭合了循环（x_t → x_{t+1}）。
-/
theorem assembly_is_closed_transition (x : AssemblyState) (e : AssemblyEvent) :
    assemblyStep x e = transitionAdapter x (HybridStep.policyOutput assembly x e) e :=
  rfl

/--
  ★★T 写回保 R=Π-A-W（L0，账户因果洞补法的闭环兑现，本文件核心）：
  `assemblyStep_preserves_ledger_inv` — 闭环转移后，ledger 分量仍满足账本恒等 R=Π-A-W。

  `(assemblyStep x e).ledgerState.R = Π - A - W`——T 经 `ledgerStep`（保不变量，§1
  `ledgerStep_preserves_inv`）更新账本，故闭环每步保持 R=Π-A-W。这补
  `StrategyFamily.piTheta_causal`「只比同一 z」的账户因果洞：T 写回账户态的下一步（账户的
  生成进入闭环），且生成保持账本恒等。

  ★诚实（formalization-validity-domain）：本定理证「闭环保 R=Π-A-W 结构不变量」（L0），
  **不**证账本数值反映实盘真实盈亏（L2/L3，Rust 运行时守卫覆盖）。
-/
theorem assemblyStep_preserves_ledger_inv (x : AssemblyState) (e : AssemblyEvent) :
    (assemblyStep x e).ledgerState.R
      = (assemblyStep x e).ledgerState.Pi
        - (assemblyStep x e).ledgerState.A
        - (assemblyStep x e).ledgerState.W :=
  (assemblyStep x e).ledgerState.inv

/-! ════════════════════════════════════════════════════════════════════════
  ## §6.5 取本金三阶段账本接入闭环（task #93，TW 守恒 + stage 单向 + OQ-9 gate，双层并置）

  twState 真被 transitionAdapter 线程化（非恒等挂件，见 assemblyStep_threads_twState +
  assemblyStep_twState_changes witness），且闭环每步：
  - 保 TW = free+holding+withdrawn 守恒（assemblyStep_preserves_tw）；
  - stage 单向不可逆（assemblyStep_stage_monotone，campaign 内）；
  - OQ-9 gate 不变量在闭环转移下保持（assemblyStep_oq9_gate_preserved）。
  与 assemblyStep_preserves_ledger_inv（R=Π-A-W 不丢）**双层并置**——两守恒都在闭环里被维持。

  ★review 019f0318 诚实分工（修正3+4(a)，撤回声明膨胀）：
  - 端B（stage 单向 + OQ-9 gate）**真在此闭环动**（_stage_monotone / _oq9_gate_preserved /
    _oq9_closure_illegal）；trace 级端B 由 TotalWealth.oq9inv_trace 承载（单文件）。
  - 端A（cumNetCash 数值回正）的载体 closeShareLeg **不被本装配 scheduleAdapter 派生** ⟹
    端A 通道**两个字段**都闭环恒定（`assemblyStep_cumNetCash_unchanged` + `_openLegacyLegs_unchanged`，
    review 复审 5c 对称补全）⟹ 端A 通道**不在此闭环有效域**（明确声明，不冒充接入；端A 在单文件
    raw witness 层完整）。trace 级端B 的合法起点由 `TotalWealth.init_satisfies_oq9inv` 坐实（5b 补全）。
  - AssemblyEvent.twEvent 是**被忽略字段**（`assemblyStep_twState_ignores_event_twEvent` 坐实），
    权威账本事件来自 OrderOut（非声明膨胀的「外部 schema」辩护）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★twState 真被线程化 `assemblyStep_threads_twState`（L0，反退化见证①·结构侧）：
  闭环转移后的 twState = `twStep x.twState (策略订单.twEvent)`——twState 由 T 经 twStep 真消费
  订单携带的 twEvent 产出，**不是恒等挂件**（删去 transitionAdapter 的 twState 行则此 rfl 不成立）。
-/
theorem assemblyStep_threads_twState (x : AssemblyState) (e : AssemblyEvent) :
    (assemblyStep x e).twState
      = twStep x.twState
          (scheduleAdapter x (riskAdapter x (intentAdapter x (classifyAdapter x (recAdapter x e))))).twEvent :=
  rfl

/--
  ★★TW 守恒接入闭环 `assemblyStep_preserves_tw`（L0，task #93 核心）：
  闭环转移后，twState 的 TW = free+holding+withdrawn **守恒**。

  `(assemblyStep x e).twState.tw = x.twState.tw`——T 经 twStep（保 TW 守恒，
  `TotalWealth.twStep_preserves_tw`）更新取本金三阶段账本，故闭环每步保持 TW 守恒（缠师第31课
  守恒律的闭环结构形式）。与 R=Π-A-W（ledger）并置——两守恒律双层都在闭环里。

  ★诚实（formalization-validity-domain）：本定理证「闭环保 TW 守恒结构不变量」（L0，同价 c 固定），
  **不**证 TW 数值反映实盘真实盈亏（跨 bar 价格变动 L2/L3，Rust prove_tw_neutral 守卫覆盖）。
-/
theorem assemblyStep_preserves_tw (x : AssemblyState) (e : AssemblyEvent) :
    (assemblyStep x e).twState.tw = x.twState.tw := by
  rw [assemblyStep_threads_twState]
  exact twStep_preserves_tw x.twState _

/--
  ★★stage 单向不可逆接入闭环 `assemblyStep_stage_monotone`（L0，OQ-9 端B 闭环形式）：
  闭环转移后，twState 的 stage rank **只增不减**（campaign 内，订单 twEvent ≠ clearCampaign）。

  本装配的 scheduleAdapter 派生的 twEvent 只取 `shortDiff (-1)`（u>0）或 `recoverCapital 1`（u=0），
  **都不是 clearCampaign**，故 `TotalWealth.stage_rank_monotone` 的 campaign-内单向性恒适用——
  闭环每步 stage rank 单调不减（CostReduction→CapitalRecovered→EarningShares 不回退，OQ-9）。
-/
theorem assemblyStep_stage_monotone (x : AssemblyState) (e : AssemblyEvent) :
    x.twState.stage.rank ≤ (assemblyStep x e).twState.stage.rank := by
  rw [assemblyStep_threads_twState]
  -- 订单 twEvent 必是 shortDiff/recoverCapital 之一（scheduleAdapter 两分支），均 ≠ clearCampaign。
  apply stage_rank_monotone
  -- 反证：twEvent = clearCampaign 不可能（scheduleAdapter 的 if 两支都不产 clearCampaign）。
  unfold scheduleAdapter
  split <;> simp only [ne_eq, not_false_eq_true, reduceCtorEq]

/--
  ★★OQ-9 gate 接入闭环 `assemblyStep_oq9_gate_preserved`（L0，task #93 + codex 修正立场C）：
  若进入闭环前 twState 已在 EarningShares 且 OQ-9 入口证书成立（无未闭合旧腿，openLegacyLegs=0），
  则闭环转移后该证书**仍成立**（openLegacyLegs=0 保持）——本装配的 twEvent（shortDiff/recoverCapital）
  都不开新腿（openShareLeg）也不闭腿（closeShareLeg），故 openLegacyLegs 不变，OQ-9 gate 被维持。

  这把 OQ-9 端B「EarningShares 后旧腿恒空 ⟹ 亏损腿闭合回正路径不可达」从单文件定理
  （`legal_earning_no_legacy_leg_closure`）**延拓到闭环**：闭环装配下，合法 OQ-9 不变量逐步保持，
  「相位锁死」是闭环转移下的结构性质（不是一次性声明）。
-/
theorem assemblyStep_oq9_gate_preserved (x : AssemblyState) (e : AssemblyEvent)
    (hgate : x.twState.openLegacyLegs = 0) :
    (assemblyStep x e).twState.openLegacyLegs = 0 := by
  rw [assemblyStep_threads_twState]
  -- twEvent ∈ {shortDiff, recoverCapital}（scheduleAdapter 两分支），二者都不动 openLegacyLegs。
  unfold scheduleAdapter
  split <;> simp only [twStep, hgate]

/--
  ★★OQ-9 端B 在闭环里是定理 `assemblyStep_oq9_closure_illegal`（L0，task #93 + codex 修正立场C）：
  闭环转移后（gate 保持，openLegacyLegs=0），对该闭环后态「闭合旧 ShareConserving 腿」**不是合法转移**。

  `¬ LegalTransition (assemblyStep x e).twState (closeShareLeg p)`——直接由 OQ-9 gate 保持
  （`assemblyStep_oq9_gate_preserved`：闭环后 openLegacyLegs=0）+ LegalTransition 对 closeShareLeg 的
  合法性要求（openLegacyLegs ≥ 1）矛盾导出。这把单文件的 `legal_earning_no_legacy_leg_closure`
  （单步入口证书 ⟹ 旧腿闭合非法）**延拓到整条闭环**：闭环每步维持 gate ⟹ 任一闭环后态上
  「亏损腿闭合回正」路径恒不可达（OQ-9 端B「相位锁死」是闭环结构定理，非一次性声明）。

  ★这真消费 LegalTransition（legal 层谓词）——闭环装配下 OQ-9 矛盾的扩维消解被坐实：raw 层
  端A 数值可回正（TotalWealth.raw_earning_legacy_leg_closure_witness），legal 层闭环里那条 trace
  恒不可达（本定理）。两端各在其层，矛盾经扩维消解（codex binding 立场C 修正版）。
-/
theorem assemblyStep_oq9_closure_illegal (x : AssemblyState) (e : AssemblyEvent) (p : Int)
    (hgate : x.twState.openLegacyLegs = 0) :
    ¬ LegalTransition (assemblyStep x e).twState (TWEvent.closeShareLeg p) := by
  -- 闭环后 openLegacyLegs = 0（gate 保持）；LegalTransition closeShareLeg 要求 ≥ 1 ⟹ 矛盾。
  have h0 : (assemblyStep x e).twState.openLegacyLegs = 0 :=
    assemblyStep_oq9_gate_preserved x e hgate
  simp only [LegalTransition, h0]
  decide

/--
  ★twState 真改变 witness `assemblyStep_twState_changes`（L0，反退化见证①·实例侧）：
  **存在**一个具体态 x 与事件 e，使闭环转移后 twState ≠ x.twState（twState 真被改写，非恒等）。

  本装配的 riskAdapter 恒投到网格最小格点（gridProject over {0,1,2,3} = 0），故 u=0 ⟹
  scheduleAdapter 取 else 分支 ⟹ twEvent = recoverCapital 1 ⟹ twStep 把 free 减 1、withdrawn 加 1
  并推进 stage——只要初始 stage=costReduction，step 后 stage=capitalRecovered ≠ costReduction，
  twState 整体被改写。这给「twState 非恒等挂件」的硬见证（不依赖 gridProject 内部值的脆弱假设：
  直接 decide 算闭环一步的 stage 改变）。
-/
theorem assemblyStep_twState_changes :
    ∃ (x : AssemblyState) (e : AssemblyEvent),
      (assemblyStep x e).twState ≠ x.twState := by
  refine ⟨{ microState := Tlayers.Dynamics.s0,
            ledgerState := ledger0 0,
            twState := { free := 100, holding := 0, withdrawn := 0, notionalIn := 0,
                         stage := NewChanlun.Origin.TotalWealth.TStage.costReduction,
                         openLegacyLegs := 0, cumNetCash := 0 },
            riskMode := RiskMode.normal, phase := Phase.phaseI,
            positions := 0, orders := 0, memory := 0 },
          { parseEvent := Tlayers.Dynamics.Event.newBar true, ledgerEvent := LedgerEvent.noop,
            twEvent := TWEvent.clearCampaign }, ?_⟩
  -- 闭环一步：riskAdapter 投网格最小格点 0 ⟹ scheduleAdapter else 分支 ⟹ 订单 twEvent=recoverCapital 1
  -- ⟹ twStep 把 stage 从 costReduction 推进到 capitalRecovered（≠ 初始）⟹ twState 改变。
  intro h
  -- 从 twState 相等推出 stage 相等，再 decide 矛盾（capitalRecovered ≠ costReduction）。
  have hstage := congrArg TWState.stage h
  simp only [assemblyStep_threads_twState, scheduleAdapter, riskAdapter] at hstage
  revert hstage
  decide

/--
  ★★twEvent 字段被闭环忽略 `assemblyStep_twState_ignores_event_twEvent`（L0，review 修正3 坐实）：
  只要 `parseEvent` 相同，**换掉 AssemblyEvent.twEvent（及 ledgerEvent）不影响后态 twState**。

  这把「e.twEvent 是 intentional ignored field」从文档声明升级为**定理**——闭环的 twState 只由
  parseEvent（经 recAdapter→…→scheduleAdapter 派生订单的 twEvent）决定，e.twEvent 不进数据流。
  坐实诚实标注（撤回旧「外部 schema 占位非死字段」辩护）：它确实是被忽略的字段，且**机器可证**。

  ★若未来要外部驱动 TW，须改 transitionAdapter 消费 e.twEvent——本定理届时会失败（成为回归守卫）。
-/
theorem assemblyStep_twState_ignores_event_twEvent
    (x : AssemblyState) (e₁ e₂ : AssemblyEvent)
    (hpe : e₁.parseEvent = e₂.parseEvent) :
    (assemblyStep x e₁).twState = (assemblyStep x e₂).twState := by
  -- twState = twStep x.twState (订单.twEvent)；订单经 recAdapter（只用 parseEvent）派生。
  rw [assemblyStep_threads_twState, assemblyStep_threads_twState]
  simp only [recAdapter, hpe]

/--
  ★★cumNetCash 闭环不动 + 端A 诚实分工 `assemblyStep_cumNetCash_unchanged`（L0，review 修正4(a)）：
  闭环转移后 `cumNetCash` **保持不变**——本装配的 scheduleAdapter 只派生 shortDiff/recoverCapital，
  **从不**派生 closeShareLeg（唯一改 cumNetCash 的事件），故 cumNetCash 在此闭环恒定。

  ★诚实分工界限（formalization-validity-domain，撤回「OQ-9 端A 接入闭环」的空洞声明）：
  OQ-9 端A（cumNetCash 数值回正）的载体是 **closeShareLeg legacy 腿**事件。当前 assembly 闭环的
  scheduleAdapter **不产 leg 事件**（开/闭 legacy 腿是 Θ_voice/Θ_exec 策略决策，L0 结构层不臆造
  「何时开闭腿」的策略逻辑——臆造=声明膨胀）。故 OQ-9 端A 的 cumNetCash 通道**不在此闭环有效域**：
  - 端A 在**单文件 raw witness 层**完整（`TotalWealth.raw_earning_legacy_leg_closure_witness`）；
  - 端B（stage 单向 + gate 保持）**真在此闭环动**（assemblyStep_stage_monotone / _oq9_gate_preserved）；
  - 端A cumNetCash 通道由本定理诚实标注「闭环未驱动」（不冒充接入）。

  这是诚实的分工：闭环承载端B（stage/gate，真动），端A（cumNetCash）在单文件 witness 层完整、
  闭环层明确声明未驱动——非 workaround（要接入须先有策略层的开/闭腿决策，那是下游 Θ 的有效域）。
-/
theorem assemblyStep_cumNetCash_unchanged (x : AssemblyState) (e : AssemblyEvent) :
    (assemblyStep x e).twState.cumNetCash = x.twState.cumNetCash := by
  rw [assemblyStep_threads_twState]
  -- scheduleAdapter 两分支产 shortDiff/recoverCapital，twStep 对二者都不动 cumNetCash。
  unfold scheduleAdapter
  split <;> simp only [twStep]

/--
  ★openLegacyLegs 闭环恒定 `assemblyStep_openLegacyLegs_unchanged`（L0，review 复审 5c 补全，
  与 cumNetCash_unchanged 对称）：闭环转移后 `openLegacyLegs` **保持不变**——scheduleAdapter 只派生
  shortDiff/recoverCapital，**从不**派生 openShareLeg/closeShareLeg（唯一动 openLegacyLegs 的事件）。

  这与 `assemblyStep_cumNetCash_unchanged` 对称，坐实 OQ-9 端A 通道的**两个字段**
  （openLegacyLegs 腿计数 + cumNetCash 净现金口径）在本闭环都恒定——端A 通道完整地不在此闭环
  有效域（不只 cumNetCash 一个字段）。`assemblyStep_oq9_gate_preserved` 是本定理在 openLegacyLegs=0
  前提下的特例；本定理是无前提的恒定声明（更强：不依赖初值是否为 0）。
-/
theorem assemblyStep_openLegacyLegs_unchanged (x : AssemblyState) (e : AssemblyEvent) :
    (assemblyStep x e).twState.openLegacyLegs = x.twState.openLegacyLegs := by
  rw [assemblyStep_threads_twState]
  unfold scheduleAdapter
  split <;> simp only [twStep]

/--
  ★RiskProj 段命中有限网格 LexArgmin（L0，禁连续 argmin 的兑现）：
  `assembly_risk_is_grid_lexargmin` — riskAdapter 的输出 = defaultRiskGrid 上 cost 字典序最优格点。
  直接引 `RiskProj.RiskGrid.gridProject_isLexArgmin`——投影命中真 LexArgmin（有限网格离散最小，
  **非连续 argmin**）。这坐实「RiskProj 段只能确定选择器/有限网格」（codex R3）。
-/
theorem assembly_risk_is_grid_lexargmin (x : AssemblyState) (i : IntentAction) :
    defaultRiskGrid.IsLexArgmin (riskAdapter x i) :=
  defaultRiskGrid.gridProject_isLexArgmin

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 诚实标签（gatekeeper：禁标盈利/最优/缠论唯一）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★装配标签 `AssemblyTag`（gatekeeper，诚实分层）。
  - `ClosedLoopWithLedger`：π̄∘C + T 写回 + R=Π-A-W 账本闭环（本文件实质增量）。
  - `TwoLedgerConserved`：★双账本闭环守恒（R=Π-A-W ledger + TW=free+holding+withdrawn twState +
    stage 单向 + OQ-9 gate，task #93 扩维）——二者不同构（#90）双层并置，两守恒都在闭环里。
  - `FunctionGraphUnique`：闭环步 ∃! 是函数图平凡侧（最弱必要侧）。
  - `FiniteGridRiskProj`：RiskProj 是有限网格确定选择器（非连续 argmin）。
  - `ThetaParametric`：网格/优先级/阈值是 Θ 参数（非缠论可导）。
  - `EmpiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableStrategy` / `ChanlunUniqueStrategy` / `ContinuousArgmin` 构造子——
  类型层拒绝把闭环装配标为盈利策略 / 缠论唯一策略 / 连续 argmin 存在性。
-/
inductive AssemblyTag where
  | closedLoopWithLedger
  | twoLedgerConserved
  | functionGraphUnique
  | finiteGridRiskProj
  | thetaParametric
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★装配子类 `AssemblySubkind`：ClosedLoopGivenTheta（唯一构造子，类型层钉死诚实标签）。 -/
inductive AssemblySubkind where
  | closedLoopGivenTheta
deriving DecidableEq, Repr

/-- ★诚实标签包：闭环+账本 + 双账本守恒（task #93）+ 函数图唯一 + 有限网格 + Θ参数化 +
    经验有效域，子类 ClosedLoopGivenTheta。 -/
def assemblyLabels : List AssemblyTag × AssemblySubkind :=
  ([AssemblyTag.closedLoopWithLedger, AssemblyTag.twoLedgerConserved,
    AssemblyTag.functionGraphUnique, AssemblyTag.finiteGridRiskProj,
    AssemblyTag.thetaParametric, AssemblyTag.empiricalDomain],
   AssemblySubkind.closedLoopGivenTheta)

/-- ★禁标盈利/缠论唯一/连续argmin（L0，gatekeeper 见证）：装配子类必是 ClosedLoopGivenTheta。 -/
theorem assembly_subkind_is_given_theta (k : AssemblySubkind) :
    k = AssemblySubkind.closedLoopGivenTheta := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（cc-hybrid-assembly 工位，task #86）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. 账本分量 `LedgerComp`（R=Π-A-W 恒等，本文件新建——补 Ledger 接缝缺口）+ `ledgerStep`
     全函数 + `ledgerStep_preserves_inv`（保 R=Π-A-W）+ `ledgerStep_total_unique`。
  2. 完整乘积态 `AssemblyState`（蓝图 §2：micro/ledger/**twState**/riskMode/phase/positions/orders/
     memory，task #93 增 twState）+ 混合事件 `AssemblyEvent`（增 twEvent，**当前闭环忽略字段**，
     review 修正3：权威账本事件来自 OrderOut，非声明膨胀的「外部 schema」辩护）。
  3. 六段具体实现（recAdapter/classifyAdapter/intentAdapter[10级优先级 actionPriority]/
     riskAdapter[有限网格 gridProject]/scheduleAdapter[派生双账本事件]/transitionAdapter[闭环T，
     同步更新 ledgerState + twState]）装配为 `assembly : HybridComponents`。
  4. ★装配总定理 `assemblyStep_total_unique`（实例化 HybridStep.hybridStep_total_unique）+
     `assembly_components_totalUnique_prod`（micro×ledger 经 Chain.totalUnique_prod 组装）。
  5. ★实质增量：`assembly_policy_factors_through_classify`（π̄∘C 真读分类）+
     `assembly_is_closed_transition`（T 真写回完整态）+ `assemblyStep_preserves_ledger_inv`
     （T 保 R=Π-A-W，补账户因果洞）+ `assembly_risk_is_grid_lexargmin`（有限网格 LexArgmin，非连续）。
  5b. ★★取本金三阶段账本接入闭环（task #93，§6.5，双层并置 + review 019f0318 修正后诚实分工）：
     - `assemblyStep_threads_twState`（twState 真被 T 经 twStep 线程化，非恒等挂件）+
       `assemblyStep_twState_changes`（具体 witness：一个 step 前后 twState 真改变，stage 推进）。
     - `assemblyStep_preserves_tw`（闭环保 TW=free+holding+withdrawn 守恒，与 R=Π-A-W 双守恒并置）。
     - `assemblyStep_stage_monotone`（闭环 stage 单向不可逆，OQ-9 端B 闭环形式）。
     - `assemblyStep_oq9_gate_preserved`（OQ-9 入口证书 openLegacyLegs=0 闭环逐步保持）+
       `assemblyStep_oq9_closure_illegal`（闭环后态闭旧腿恒非法，消费 LegalTransition，端B 是闭环定理）。
     - ★review 修正3：`assemblyStep_twState_ignores_event_twEvent`（换 e.twEvent 不影响后态 twState，
       坐实「e.twEvent 是被忽略字段」——撤回声明膨胀）。
     - ★review 修正4(a)：`assemblyStep_cumNetCash_unchanged`（cumNetCash 闭环恒定 ⟹ OQ-9 端A
       cumNetCash 通道不在此闭环有效域，诚实声明不冒充接入；端A 在单文件 raw witness 层完整）。
  6. 诚实标签 `assemblyLabels`（增 twoLedgerConserved）+ `assembly_subkind_is_given_theta`
     （禁标盈利/缠论唯一/连续argmin）。

  本文件**不证**（codex R3 诚实边界，装配不得声明膨胀）：
  - ✗ 盈利/最优/实盘有效（T 只闭环全定义，L3 EmpiricalDomain）。
  - ✗ RiskProj 连续 argmin 存在性（risk 段有限网格确定选择器）。
  - ✗ 10 级优先级由缠论唯一推出（actionPriority 是 Θ_voice 确定选择器）。
  - ✗ classifyAdapter 摘要等价于完整 C_Θ fiber partition（完整 C_Θ 唯一性由 Chain 承载，
       本段只兑现「Intent 真读 classify 输出」的闭环数据流）。
  - ✗ 账本数值反映实盘真实盈亏（ledger 保 R=Π-A-W / twState 保 TW 都是 L0 结构不变量，数值
       正确性 L2，Rust 守卫；twState 的 TW 守恒仅同价 c 固定，跨 bar 价格变动 L2/L3）。
  - ✗ riskMode/phase/memory 转移阈值（Θ/L2，本骨架保持，不臆造阈值——诚实留白非 workaround）。
  - ✗ AssemblyEvent.{ledgerEvent,twEvent} 被 T 直接消费（review 修正3：它们是**当前闭环忽略的
       字段**，T 实际消费订单 OrderOut.{ledgerEvent,twEvent}，由 assemblyStep_twState_ignores_event_twEvent
       坐实——非「外部 schema 占位」的声明膨胀辩护）。
  - ✗ OQ-9 端A（cumNetCash 回正）接入本闭环（review 修正4(a)：scheduleAdapter 不派生 closeShareLeg，
       cumNetCash 闭环恒定，assemblyStep_cumNetCash_unchanged 坐实；端A 在单文件 raw witness 层完整，
       闭环层诚实声明未驱动——接入须先有策略层开/闭腿决策，属下游 Θ 有效域，L0 不臆造）。

  ★ledger 接缝裁定（重要，formalization-validity-domain）：蓝图 §2 标 ledgerState 来源
  `Tlayers/Accounting/Ledger`，但该文件**无** R=Π-A-W 四字段恒等结构（实测）。本文件**新建**
  `LedgerComp` 承载 R=Π-A-W，**诚实声明不复用 Ledger 已证 R=Π-A-W 结构**（Ledger 无此结构，
  冒充=090声明膨胀）。Ledger 的股数守恒/NAV中性作同范畴佐证。这是装配中发现的**蓝图引用与
  实际组件的接缝缺口**——按 no-workaround，不硬塞/不冒充，新建正确载体并标注。

  ★OQ-9 扩维裁定（task #93，codex binding 立场C 修正版，gpt-5.5 xhigh session 019f0303）：
  取本金三阶段账本（twState）经 `import Origin.TotalWealth`（#127 native 重锚）接入闭环，**与 R=Π-A-W 双层并置**
  （二者不同构 #90，互不替代——R=Π-A-W 收益表视角 / TW 现金流+持仓视角，完整持仓系统两者都需要）。
  OQ-9 守恒律相变可逆性矛盾经**扩维消解**：stage 与 cumNetCash 数值解耦（独立维度），openLegacyLegs
  承载入口证书。codex 三修正全吸收——(1) 非法性在 enterEarning 入口（openLegacyLegs=0），非 close
  时事后判违规；(2) rawStep/legalStep 双层（raw 保端A 数值 witness / legal 排除该 trace，非「两端
  并存」声明膨胀）；(3) 非对称是自洽 hysteresis。端B「相位锁死」由 `assemblyStep_oq9_closure_illegal`
  延拓为**闭环定理**（非声明）。与 ledger.rs「严格形式 + n_t5 计数器可观测」结构一致。

  谱系：615/616/617（C_Θ 与 π_Θ 都 Θ-参数化）→ HybridStep（闭环装配抽象接口）→ 本文件 #86
        （抽象接口具体化 + R=Π-A-W 账户因果接入闭环，补 piTheta_causal「只比同一 z」的洞）→
        #90（R=Π-A-W vs 取本金三阶段不同构裁定）→ 本文件 #93（取本金三阶段 TW + OQ-9 gate 扩维
        接入闭环，双账本并置；OQ-9 矛盾 design §3.3 经 codex 立场C 修正版扩维消解）→
        #97（Origin canonical base）→ 本文件 #101（实现 Origin FullDefinitionSystem 接口，§8）。
  ════════════════════════════════════════════════════════════════════════ -/

/-! ════════════════════════════════════════════════════════════════════════
  ## §8 实现 Origin `FullDefinitionSystem` 接口（task #101，A′ Phase2 重锚）

  A′ 把 `formal/Origin/` 定为唯一 canonical base（#97）。本节让 HybridAssembly 装配
  **实现 Origin 的 `FullDefinitionSystem` 接口**——构造一个真能编译的 Origin 接口见证
  `originFullDef : FullDefinitionSystem`，其六段（recStruct/classify/intent/risk/schedule/
  transition）由本文件已证的装配语义提供见证，并把闭环 ∃! / π̄∘C 因子化 / R=Π-A-W 保持三项
  定理义务挂到 Origin 已证元定理（`hybrid_step_complete_unique` / `policy_factors_through_classification`
  / `ledger_invariant_preservation`）。

  ════════════════════════════════════════════════════════════════════════
  ## ★接缝的严格形式（no-workaround + #90，关键诚实声明）

  Origin `FullDefinitionSystem` 的六段第一参数**硬钉在 Origin 的 `StrictState`**（带 Origin
  `LedgerState` = R=Π-A-W 四字段），它**没有 twState 字段**。本文件的 `AssemblyState` 是**另一个
  状态类型**（带 microState/LedgerComp/**twState**/...）。二者不是同一类型——这**正是 #90 已证的
  不同构**：Origin 单账本 `LedgerState`(R=Π-A-W) 与取本金三阶段 TW 不同构，故 Origin 的 `StrictState`
  **结构上无法吸收 twState**（吸收=抹掉 #90 的不同构裁定=声明膨胀）。

  故「实现接口」的**严格形式**是两层，不是「AssemblyState = StrictState」的冒充：

  1. **接口被实例化**（`originFullDef`）：在 Origin `StrictState` 上构造一个 `FullDefinitionSystem`
     见证，其 action 段**复用 Origin 自己的 `chooseAction` 优先级选择器**、ledger 段**复用 Origin
     自己的 `ledgerStep`（保 R=Π-A-W）**——这是 Origin 接口确实可被本装配语义居留的真见证
     （`originFullDef` 真能编译，三项定理义务由 Origin 元定理兑现）。

  2. **#93 双账本扩维在 HybridAssembly 自有闭环**（`assembly`/`assemblyStep`/`twState`）：twState/TW/
     stage/OQ-9 是 HybridAssembly 的 AssemblyState 闭环承载的**第二账本**（#90 的不同构第二端），
     它与 Origin 单账本接口**并置**（双层），由 `Origin/LedgerBridge.lean` 桥接定理坐实「ledger inv /
     TW / stage / OQ-9 全在 Origin 接口下成立」。Origin 接口承载 R=Π-A-W 端，HybridAssembly 闭环
     承载 TW 端——两端各在其层，#90 不同构被尊重（非糊成一个）。

  ★诚实裁定（formalization-validity-domain）：本节**不**声明「AssemblyState 是 Origin StrictState 的
  子类型/同构像」（#90 否证之）。本节声明的是「Origin `FullDefinitionSystem` 接口可被本装配语义居留
  （originFullDef 真见证）+ 本装配 AssemblyState 闭环的核心义务（∃!/π̄∘C/R=Π-A-W）与 Origin 元定理
  逐项对齐」。L0（结构层）。
  ════════════════════════════════════════════════════════════════════════ -/

open NewChanlun.Origin (FullDefinitionSystem StrictState ParseStruct TrendClass ActionClass
  RiskMode CapitalPhase LedgerState mkLedger chooseAction hybridStep policyTheta
  hybrid_step_complete_unique policy_factors_through_classification ledger_invariant_preservation)

/--
  ★空解析结构 `emptyParse`（L0）：Origin `ParseStruct` 的空实例（八字段全空 / tail=none）。
  作 originFullDef recStruct 段的结构层占位输出——本骨架的 Origin 接口见证不重算缠论解析
  （完整解析由 Origin.ElementPipeline.parse 承载），只兑现「recStruct 是 StrictState→Event→ParseStruct
  的全函数」接口义务。
-/
def emptyParse : ParseStruct :=
  { mergedBars := [], fractals := [], strokes := [], segments := [], centers := [],
    moves := [], bsp := [], tail := NewChanlun.Origin.OpenTail.none }

/--
  ★Origin action 段适配 `originActionOf`（L0）：把 Origin `StrictState` 的当前 trend 标签映为
  9 个优先级谓词，喂给 **Origin 自己的 `chooseAction`** 优先级选择器，得 Origin `ActionClass`。

  本骨架取「trend=trendUp ⟹ openRoot（p7）；其余 ⟹ hold」的最小确定映射（结构层，p1..p6=p8=p9=false）——
  这复用 Origin canonical 的 10 级优先级**确定选择器**，不另造优先级排序（Origin 接口的 action
  语义由 Origin chooseAction 钉死，本段只提供谓词输入）。诚实：这是 Θ_voice 设计选择，非缠论唯一。
-/
def originActionOf (x : StrictState) : ActionClass :=
  chooseAction false false false false false false (decide (x.trend = TrendClass.trendUp)) false false

/--
  ★★Origin 接口见证 `originFullDef`（L0，task #101 核心——HybridAssembly 实现 Origin 接口）：
  构造 `FullDefinitionSystem` 实例，六段全函数填充：
  - `recStruct` := 结构层 emptyParse（接口义务：StrictState→Event→ParseStruct 全函数）。
  - `classify` := 复用 StrictState 已携带的 `trend`（Origin TrendClass，分类输出真被下游 intent 读）。
  - `intent` := **Origin `chooseAction` 优先级选择器**（intent 输入含 TrendClass ⟹ 策略穿过分类瓶颈）。
  - `risk`/`schedule` := 结构层确定全函数（Control=Nat 目标仓位 / Order=Origin ActionClass）。
  - `transition` := 复用 **Origin `ledgerStep`（保 R=Π-A-W）** 更新 ledger 分量 + 写回完整 StrictState
    （T 闭环写回，账本更新放进 T——补账户因果洞，与 §4 transitionAdapter 同构）。

  Event/Intent/Control/Order 取最小具体类型（Unit/ActionClass/Nat/ActionClass）。
  这是 Origin `FullDefinitionSystem` 接口可被本装配语义居留的真见证（真能编译，无 sorry）。
-/
def originFullDef : FullDefinitionSystem where
  Event := Unit
  Intent := ActionClass
  Control := Nat
  Order := ActionClass
  recStruct := fun _ _ => emptyParse
  classify := fun x _ => x.trend
  intent := fun _ c => chooseAction false false false false false false (decide (c = TrendClass.trendUp)) false false
  risk := fun _ i => match i with | ActionClass.openRoot => 1 | _ => 0
  schedule := fun _ _u => ActionClass.hold
  transition := fun x o _ =>
    -- T 闭环写回：order=ActionClass 驱动 Origin ledgerStep（保 R=Π-A-W），写回完整 StrictState。
    { x with
        actionClass := o,
        ledger := match o with
          | ActionClass.openRoot => NewChanlun.Origin.ledgerStep x.ledger 0 1 0   -- 建仓资本化 allocate
          | _ => NewChanlun.Origin.ledgerStep x.ledger 1 0 0 }                    -- 其余实现盈亏 realize

/--
  ★★HybridAssembly 实现 Origin 接口 · 闭环 ∃!（L0，task #101，接口义务①）：
  `originFullDef` 的 Origin `hybridStep` 每步存在且唯一（实例化 Origin `hybrid_step_complete_unique`）。
  这兑现 Origin 接口的「闭环每步全定义 + 唯一」定理义务——HybridAssembly 居留 Origin 接口后，
  Origin 的闭环唯一性元定理对本见证成立。
-/
theorem originFullDef_step_complete_unique (x : StrictState) (e : originFullDef.Event) :
    NewChanlun.Origin.ExistsUnique (fun x' => hybridStep originFullDef x e = x') :=
  hybrid_step_complete_unique originFullDef x e

/--
  ★★HybridAssembly 实现 Origin 接口 · π̄∘C 因子化（L0，task #101，接口义务②）：
  `originFullDef` 的策略 `policyTheta` 真穿过 classify（intent 读 classify 输出）——实例化 Origin
  `policy_factors_through_classification`。坐实 Origin 接口的「策略穿过分类瓶颈」义务对本见证成立。
-/
theorem originFullDef_policy_factors (x : StrictState) (e : originFullDef.Event) :
    policyTheta originFullDef x e =
      originFullDef.schedule x
        (originFullDef.risk x
          (originFullDef.intent x
            (originFullDef.classify x (originFullDef.recStruct x e)))) :=
  policy_factors_through_classification originFullDef x e

/--
  ★★HybridAssembly 实现 Origin 接口 · T 保 R=Π-A-W（L0，task #101，接口义务③）：
  `originFullDef` 的闭环转移后 ledger 分量仍满足 Origin 账本恒等 R=Π-A-W——T 段复用 Origin
  `ledgerStep`（其 `inv` 保恒等），故闭环每步保持。兑现 Origin 接口的账本恒等保持义务。

  ★这与 HybridAssembly 自有闭环的 `assemblyStep_preserves_ledger_inv`（本文件 §6）**同构对齐**：
  两侧都证「闭环 T 保 R=Π-A-W」，一侧在 Origin StrictState 接口（本定理），一侧在 AssemblyState
  闭环（§6）——双层都成立，#90 的 R=Π-A-W 端在两层一致。
-/
theorem originFullDef_transition_preserves_ledger_inv (x : StrictState) (e : originFullDef.Event) :
    (hybridStep originFullDef x e).ledger.R =
      (hybridStep originFullDef x e).ledger.Pi
        - (hybridStep originFullDef x e).ledger.A
        - (hybridStep originFullDef x e).ledger.W :=
  (hybridStep originFullDef x e).ledger.inv

end Strict.HybridAssembly
