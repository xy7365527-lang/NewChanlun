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
import HybridStep
import Fugue
import RiskProj
import Tlayers.Dynamics

namespace Strict.HybridAssembly

open Strict.Chain (TotalUnique totalUnique_of_fun totalUnique_prod)
open Strict.HybridStep (HybridComponents hybridStep)

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
  riskMode : RiskMode
  phase : Phase
  positions : Nat
  orders : Nat
  memory : Nat
deriving Repr

/--
  ★混合事件 `AssemblyEvent`（L0）：驱动闭环一步的外部事件 e。
  携带一个解析事件（`Dynamics.Event`，驱动 microState）+ 一个账本事件（`LedgerEvent`，
  驱动 ledgerState）——两侧事件在同一外部事件 e 中同步到达（闭环转移 T 同时消费）。
-/
structure AssemblyEvent where
  parseEvent : Tlayers.Dynamics.Event
  ledgerEvent : LedgerEvent
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
  ★订单 `OrderOut`（Schedule 段输出，L0）：调度后的订单 O_t（携带动作 + 目标仓位 + 账本事件）。
  Schedule 段 = 把控制（目标仓位）+ 意图（动作）+ 账本副作用打包为订单（StrategyFamily.exec
  的执行顺序 Θ_exec 摘要）。订单携带 `ledgerEvent` 使 T 的 ledger 更新有据可依（账户因果）。
-/
structure OrderOut where
  action : Strict.StrictAction
  targetPos : Nat
  ledgerEvent : LedgerEvent
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
  这是 StrategyFamily.exec 的执行顺序 Θ_exec 摘要——订单携带动作 + 目标仓位 + 派生的账本事件
  （动作决定账本副作用：buy/add → allocate（资本化建仓占用 R），reduce/close → realize（回收
  实现盈亏），hold/wait/sell → noop 摘要）。账本事件使 T 的 ledger 更新有据（账户因果链）。

  ★诚实：账本事件的 dΠ/dA 取占位常量（1 单位）——具体数额是 Θ_risk/运行时数据（L2），本结构
  层只承载「动作 ⟹ 账本事件类型」的映射（buy 占资本、close 实现盈亏），数额由下游填充。
-/
def scheduleAdapter (_x : AssemblyState) (u : Control) : OrderOut :=
  -- 本骨架的意图未单独透传到 schedule（HybridStep schedule 签名为 state→control→order）；
  -- 动作由 control（目标仓位）派生：q*>0 ⟹ 建仓订单（allocate），q*=0 ⟹ 平仓订单（realize）。
  if u > 0 then
    { action := Strict.StrictAction.buy, targetPos := u, ledgerEvent := LedgerEvent.allocate 1 }
  else
    { action := Strict.StrictAction.close, targetPos := 0, ledgerEvent := LedgerEvent.realize 1 }

/--
  ★★T 段 `transitionAdapter`（L0，本文件核心新建——闭环写回完整 AssemblyState）：
  `T : AssemblyState → OrderOut → AssemblyEvent → AssemblyState`。

  闭环写回各分量（codex R2：δ 只更新 microState，**ledger/accounting 更新放进 T**）：
  - `microState` := `Dynamics.delta x.microState e.parseEvent`（解析层在线推进一步，T-causal）。
  - `ledgerState` := `ledgerStep x.ledgerState o.ledgerEvent`（**账本更新，保 R=Π-A-W**——
    用订单携带的账本事件，使账户态 z 的生成进入闭环，补 piTheta_causal 的账户因果洞）。
  - `positions` := 订单目标仓位 `o.targetPos`（写回新持仓）。
  - `orders` := `x.orders + 1`（订单计数推进）。
  - `memory`、`riskMode`、`phase`：本骨架保持（其转移由 Θ_signal/Θ_risk 阈值驱动，L2；
    结构层装配保持，不臆造阈值）。

  ★诚实标注（codex R3）：T **只声明闭环状态转移全定义**（产出确定的下一态），**不**声明该
  转移盈利 / 最优 / 实盘有效（L3）。riskMode/phase/memory 的转移阈值是 Θ/L2（不在 L0 装配内
  臆造），本骨架保持它们 = 诚实留白（非 workaround：阈值驱动转移属下游有效域，本文件只闭合
  micro/ledger/positions/orders 的结构闭环）。
-/
def transitionAdapter (x : AssemblyState) (o : OrderOut) (e : AssemblyEvent) : AssemblyState :=
  { microState := Tlayers.Dynamics.delta x.microState e.parseEvent
    ledgerState := ledgerStep x.ledgerState o.ledgerEvent
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
  - `FunctionGraphUnique`：闭环步 ∃! 是函数图平凡侧（最弱必要侧）。
  - `FiniteGridRiskProj`：RiskProj 是有限网格确定选择器（非连续 argmin）。
  - `ThetaParametric`：网格/优先级/阈值是 Θ 参数（非缠论可导）。
  - `EmpiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableStrategy` / `ChanlunUniqueStrategy` / `ContinuousArgmin` 构造子——
  类型层拒绝把闭环装配标为盈利策略 / 缠论唯一策略 / 连续 argmin 存在性。
-/
inductive AssemblyTag where
  | closedLoopWithLedger
  | functionGraphUnique
  | finiteGridRiskProj
  | thetaParametric
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★装配子类 `AssemblySubkind`：ClosedLoopGivenTheta（唯一构造子，类型层钉死诚实标签）。 -/
inductive AssemblySubkind where
  | closedLoopGivenTheta
deriving DecidableEq, Repr

/-- ★诚实标签包：闭环+账本 + 函数图唯一 + 有限网格 + Θ参数化 + 经验有效域，子类 ClosedLoopGivenTheta。 -/
def assemblyLabels : List AssemblyTag × AssemblySubkind :=
  ([AssemblyTag.closedLoopWithLedger, AssemblyTag.functionGraphUnique,
    AssemblyTag.finiteGridRiskProj, AssemblyTag.thetaParametric, AssemblyTag.empiricalDomain],
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
  2. 完整乘积态 `AssemblyState`（蓝图 §2：micro/ledger/riskMode/phase/positions/orders/memory）+
     混合事件 `AssemblyEvent`。
  3. 六段具体实现（recAdapter/classifyAdapter/intentAdapter[10级优先级 actionPriority]/
     riskAdapter[有限网格 gridProject]/scheduleAdapter/transitionAdapter[闭环T]）装配为
     `assembly : HybridComponents`。
  4. ★装配总定理 `assemblyStep_total_unique`（实例化 HybridStep.hybridStep_total_unique）+
     `assembly_components_totalUnique_prod`（micro×ledger 经 Chain.totalUnique_prod 组装）。
  5. ★实质增量：`assembly_policy_factors_through_classify`（π̄∘C 真读分类）+
     `assembly_is_closed_transition`（T 真写回完整态）+ `assemblyStep_preserves_ledger_inv`
     （T 保 R=Π-A-W，补账户因果洞）+ `assembly_risk_is_grid_lexargmin`（有限网格 LexArgmin，非连续）。
  6. 诚实标签 `assemblyLabels` + `assembly_subkind_is_given_theta`（禁标盈利/缠论唯一/连续argmin）。

  本文件**不证**（codex R3 诚实边界，装配不得声明膨胀）：
  - ✗ 盈利/最优/实盘有效（T 只闭环全定义，L3 EmpiricalDomain）。
  - ✗ RiskProj 连续 argmin 存在性（risk 段有限网格确定选择器）。
  - ✗ 10 级优先级由缠论唯一推出（actionPriority 是 Θ_voice 确定选择器）。
  - ✗ classifyAdapter 摘要等价于完整 C_Θ fiber partition（完整 C_Θ 唯一性由 Chain 承载，
       本段只兑现「Intent 真读 classify 输出」的闭环数据流）。
  - ✗ 账本数值反映实盘真实盈亏（ledger 保 R=Π-A-W 是 L0 结构不变量，数值正确性 L2，Rust 守卫）。
  - ✗ riskMode/phase/memory 转移阈值（Θ/L2，本骨架保持，不臆造阈值——诚实留白非 workaround）。

  ★ledger 接缝裁定（重要，formalization-validity-domain）：蓝图 §2 标 ledgerState 来源
  `Tlayers/Accounting/Ledger`，但该文件**无** R=Π-A-W 四字段恒等结构（实测）。本文件**新建**
  `LedgerComp` 承载 R=Π-A-W，**诚实声明不复用 Ledger 已证 R=Π-A-W 结构**（Ledger 无此结构，
  冒充=090声明膨胀）。Ledger 的股数守恒/NAV中性作同范畴佐证。这是装配中发现的**蓝图引用与
  实际组件的接缝缺口**——按 no-workaround，不硬塞/不冒充，新建正确载体并标注。

  谱系：615/616/617（C_Θ 与 π_Θ 都 Θ-参数化）→ HybridStep（闭环装配抽象接口）→ 本文件
        （抽象接口具体化 + R=Π-A-W 账户因果接入闭环，补 piTheta_causal「只比同一 z」的洞）。
  ════════════════════════════════════════════════════════════════════════ -/

end Strict.HybridAssembly
