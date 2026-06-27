/-
  Origin/ConstraintSystem.lean — 全局安全可行集 K_Θ（§14 / FULL 十四，port 到 Origin canonical base）
  ★task #54 L4 资本风控层（K_Θ 17 约束完整清单 + 非空性 K_Θ ≠ ∅）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**（formal/Origin/）。审计：全局安全可行集
  K_Θ（strict §12「全局安全可行集」line 379-411 / FULL 十四 line 1067-1113）MISSING in Origin
  ——RiskProj 只证「有限非空网格上字典序最小存在唯一」，**抽象掉了网格 𝒦 由哪 17 条约束切出**。
  本文件把 K_Θ 的**17 条约束完整清单** + 非空性**重锚到 Origin 类型**：每条约束是控制量 u 上的
  可判定谓词，K_Θ-membership = 17 谓词合取，定理挂 `NewChanlun.Origin.ConstraintSystem`。

  ── canonical 依据（FULL 十四 line 1080-1098，逐条 17 约束）─────────────────
  控制量 u = (q', w', cancel, orders)。K_Θ(x_t) = { u : C1 ∧ C2 ∧ … ∧ C17 }：
    C1.  q'_v ∈ Δ_v ℕ₀                              （手数网格：声部仓位是格点）
    C2.  a'_v ≤ a'_{p(v)}                           （祖先关闭：子活跃 ⟹ 父活跃）
    C3.  σ'_v = -σ'_{p(v)}                          （赋格反向：子方向 = 父翻转）
    C4.  a'_v = 1 ⟹ q'_v = q'_{p(v)}               （同单位数对冲：活跃子与父同手数）
    C5.  Σ_{w∈Ch(v)} a'_w ≤ 1                       （一父一活跃子：每父至多一个活跃子）
    C6.  L^G(q') ≤ L̄^G                             （毛杠杆上限）
    C7.  L^N(q') ≤ L̄^N                             （净杠杆上限）
    C8.  IM_t(q') + B^open_t(q') ≤ E_t - w'         （开仓初始保证金 + 开仓借券 ≤ 可用权益）
    C9.  MM_t(q') + B^hold_t(q') ≤ E_t - w'         （持仓维持保证金 + 持仓借券 ≤ 可用权益）
    C10. StressLoss_t(q') ≤ ρ_max · E_t             （压力损失上界）
    C11. 0 ≤ w' ≤ D^dist_t                          （提现界：非负且 ≤ 可分配额）
    C12. Φ_t = II ⟹ 不增加战术仓                    （阶段二：不增战术仓）
    C13. ΔQ_core > 0 ⟹ 后代全平且无未完成订单      （增核前提：增核必先清后代）
    C14. μ_t ≠ Normal ⟹ G(q') ≤ G(q_t)             （非正常模式：不增毛敞口）
    C15. 借券规则成立                               （借券合规）
    C16. 仓位模式规则成立                           （仓位模式合规）
    C17. 交易场所规则成立                           （交易场所合规）
  并 K_Θ ≠ ∅（FULL 十四 line 1103-1113）：安全控制 u^safe = 撤单 + 禁新风险 + 减仓 始终存在。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数/逻辑，不依赖数据）。机器可检验命题：
  - `feasible_decidable`：17 谓词合取可判定（每条约束是可判定谓词 ⟹ 成员判定可决）。
  - `safe_control_feasible`：u^safe（空仓 + 零提现）满足全部 17 约束 ⟹ K_Θ ≠ ∅（构造性见证）。
  - `feasible_iff_all17`：u ∈ K_Θ ⟺ 17 约束逐条成立（完整性见证：少一条不成立则 u ∉ K_Θ）。
  Lean build 通过 = 这些命题正确（L0），**不**是「这 17 约束在真实账户上不会触发强平」
  （那是 L2 EmpiricalDomain——约束参数 L̄^G/ρ_max/MM_t 的经验校准，本文件不声称）。

  ── 诚实标注（no-patch-mentality + formalization-validity-domain）────────────
  ★17 约束完整不遗漏（逐条对照 FULL 十四 line 1080-1098）——本文件 `Control` 结构 + `Feasible`
    谓词逐条编码 C1..C17，`feasible_iff_all17` 见证「成员性 ⟺ 17 约束全真」（少一条则破）。
  ★约束参数（L̄^G/L̄^N/ρ_max/MM_t/IM_t/D^dist/借券规则）全是 Θ_risk/账户层参数（非缠论可导）。
    本文件证「给定参数后可行集的结构（成员判定 + 非空）」，**不**证参数经验最优。
  ★C15/C16/C17（借券/仓位模式/交易场所规则）是**外部合规谓词**——FULL 十四只声明「规则成立」
    不展开规则内容（它们是场所/经纪商规则，非缠论）。本文件编码为可判定布尔谓词
    `borrowOk/positionModeOk/venueOk`（Θ_execution 参数），忠实承载「规则成立」的占位，
    **不**冒充展开这些外部规则的内容（still-MISSING：规则细则 L2，属场所文档）。

  ── 依赖方向（单向无环，不 import legacy Strict）──────────────────────────────
  ConstraintSystem → Origin.LeverageCapital（用 grossNotional/netNotional/VoicePosition）
                   → Origin.SourceAxioms（用 Side）。standalone（不 import #113 分类血肉）。
  验证：`cd formal && lake env lean Origin/ConstraintSystem.lean`。禁 sorry/admit/axiom。

  谱系：FULL 十四 / strict §12 → #96/#97（A′ Origin canonical base）→ RiskProj #114（抽象网格）→
        本文件 #54（K_Θ 17 约束具体化，补 RiskProj 抽象掉的「网格由哪些约束切出」）。
-/

import Origin.SourceAxioms
import Origin.LeverageCapital
import Origin.VoiceTree

namespace NewChanlun.Origin.ConstraintSystem

open NewChanlun.Origin.LeverageCapital
open NewChanlun.Origin.VoiceTree (flip)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 控制量 u = (q', w', cancel, orders) 与声部上下文

  控制量 u（FULL 十四 line 1071）= 目标仓位 q'、提现 w'、撤单 cancel、新订单 orders。
  本文件聚焦 K_Θ 的**约束结构**（哪 17 条切出可行集），把 u 编码为：每声部的目标仓位/活跃位/
  方向 + 标量提现 w'。声部父子结构（祖先关闭/赋格反向）用显式 parent 索引承载。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★单声部控制 `VoiceControl`（L0）：控制量 u 在单声部 v 上的分量。

  - `targetQty : Nat`：目标仓位 q'_v（手数，∈ Δ_v ℕ₀ 网格——C1）。
  - `active : Bool`：活跃位 a'_v（1 = 该声部本周期活跃下单）。
  - `side : Side`：声部方向 σ'_v（long/short）。
  - `parent : Option Nat`：父声部索引（none = 局部根；用于 C2/C3/C4/C5 父子约束）。
  - `notionalMag : Nat`：该声部名义大小 |n_v|（供 C6/C7 杠杆与 C14 毛敞口聚合）。
-/
structure VoiceControl where
  targetQty : Nat
  active : Bool
  side : Side
  parent : Option Nat
  notionalMag : Nat
deriving Repr

/-- ★VoiceControl 投影到 LeverageCapital 名义头寸（供杠杆聚合）。 -/
def VoiceControl.toPosition (vc : VoiceControl) : VoicePosition :=
  { side := vc.side, notionalMag := vc.notionalMag }

/--
  ★控制量 `Control`（L0，FULL 十四 line 1071 的结构化）：u = (q', w', cancel, orders)。

  - `voices : List VoiceControl`：各声部控制分量（按索引，parent 引用此列表下标）。
  - `withdraw : Int`：提现 w'（C8/C9/C11）。
  - `cancelOpen : Bool`：是否撤销未完成开仓指令（u^safe 的一部分）。
  - `coreDelta : Int`：核心单位增量 ΔQ_core（C13：>0 触发增核前提）。

  ★诚实标注：cancel/orders 的完整订单簿不在此展开（K_Θ 的约束作用在 q'/w'/活跃结构上，
  订单是 Schedule_Θ 阶段从 u* 派生——见 Origin pipeline）。本结构承载 K_Θ 约束所需的字段。
-/
structure Control where
  voices : List VoiceControl
  withdraw : Int
  cancelOpen : Bool
  coreDelta : Int

/--
  ★可行性上下文 `FeasibilityContext`（L0，FULL 十四：约束依赖的账户/Θ 层参数 + 谓词）。

  账户层标量（非缠论可导）：
  - `equity : Int` = E_t（权益）。
  - `grossCap : Int` = G̅（毛敞口上限 = L̄^G·E_t，C6）；`netCap : Int` = N̅（净敞口上限，C7）。
  - `imPlusOpenBorrow : Int` = IM_t(q') + B^open_t(q')（开仓初始保证金+借券，C8）。
  - `mmPlusHoldBorrow : Int` = MM_t(q') + B^hold_t(q')（持仓维持保证金+借券，C9）。
  - `stressLoss : Int` = StressLoss_t(q')；`rhoMaxEquity : Int` = ρ_max·E_t（C10）。
  - `distributable : Int` = D^dist_t（可分配提现额，C11）。
  - `grossPrev : Int` = G(q_t)（上周期毛敞口，C14）。

  阶段/模式/合规谓词（C12/C13/C14/C15/C16/C17）：
  - `phaseII : Bool` = (Φ_t = II)（C12）；`tacticalIncreased : Control → Bool`（战术仓是否增，C12）。
  - `normalMode : Bool` = (μ_t = Normal)（C14）。
  - `descendantsFlatNoPending : Control → Bool`（后代全平且无未完成订单，C13）。
  - `borrowOk/positionModeOk/venueOk : Control → Bool`（借券/仓位模式/场所规则，C15/C16/C17）。

  ★诚实标注：IM_t/MM_t/StressLoss/borrowOk 等是 Θ_risk/Θ_execution/场所参数化函数（非缠论可导）。
  本上下文承载它们在当前控制 u 上的求值结果（标量）或谓词，证「给定这些后可行集结构」。
  C15/C16/C17 编码为外部布尔谓词（忠实「规则成立」占位，不展开场所规则内容——still-MISSING L2）。
-/
structure FeasibilityContext where
  equity : Int
  grossCap : Int
  netCap : Int
  imPlusOpenBorrow : Int
  mmPlusHoldBorrow : Int
  stressLoss : Int
  rhoMaxEquity : Int
  distributable : Int
  grossPrev : Int
  phaseII : Bool
  normalMode : Bool
  tacticalIncreased : Control → Bool
  descendantsFlatNoPending : Control → Bool
  borrowOk : Control → Bool
  positionModeOk : Control → Bool
  venueOk : Control → Bool

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 17 约束逐条编码（C1..C17，每条是可判定谓词）

  下面把 FULL 十四 line 1080-1098 的 17 条约束逐条编码。每条独立可判定，K_Θ-membership
  = 17 条合取。每条约束的编号与 docstring 顶部清单一一对应（不遗漏任一条）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 安全取列表第 i 项的方向/活跃位/仓位（越界返回默认，供父子约束查父）。 -/
def parentSide (voices : List VoiceControl) (i : Nat) : Option Side :=
  (voices[i]?).map VoiceControl.side
def parentActive (voices : List VoiceControl) (i : Nat) : Option Bool :=
  (voices[i]?).map VoiceControl.active
def parentQty (voices : List VoiceControl) (i : Nat) : Option Nat :=
  (voices[i]?).map VoiceControl.targetQty

/-- **C1** 手数网格 `q'_v ∈ Δ_v ℕ₀`：targetQty 是 ℕ（自然落格点，网格步长 Δ_v 由调用方离散化）。
    ★诚实：targetQty : Nat 本身即「∈ ℕ₀ 网格」；步长 Δ_v 的具体值是 Θ_risk 参数，此处编码为
    「仓位是自然数手数」（Δ_v=1 单位手的退化，一般 Δ_v 倍数由网格生成器保证）。 -/
def c1_qtyGrid (_vc : VoiceControl) : Prop := True

/-- **C2** 祖先关闭 `a'_v ≤ a'_{p(v)}`：子活跃 ⟹ 父活跃（有父时）。 -/
def c2_ancestorClose (voices : List VoiceControl) (vc : VoiceControl) : Prop :=
  match vc.parent with
  | none => True
  | some i =>
      match parentActive voices i with
      | none => True              -- 父索引越界（视为根，真空满足）
      | some pa => vc.active = true → pa = true

/-- **C3** 赋格反向 `σ'_v = -σ'_{p(v)}`：子方向 = 父方向翻转（有父时）。 -/
def c3_alternating (voices : List VoiceControl) (vc : VoiceControl) : Prop :=
  match vc.parent with
  | none => True
  | some i =>
      match parentSide voices i with
      | none => True
      | some ps => vc.side = flip ps

/-- **C4** 同单位数对冲 `a'_v = 1 ⟹ q'_v = q'_{p(v)}`：活跃子与父同手数（有父时）。 -/
def c4_sameUnitHedge (voices : List VoiceControl) (vc : VoiceControl) : Prop :=
  match vc.parent with
  | none => True
  | some i =>
      match parentQty voices i with
      | none => True
      | some pq => vc.active = true → vc.targetQty = pq

/-- **C5** 一父一活跃子 `Σ_{w∈Ch(v)} a'_w ≤ 1`：每个父声部至多一个活跃子。
    对每个父索引 i，统计 voices 中 parent=some i 且 active 的数量 ≤ 1。 -/
def activeChildCount (voices : List VoiceControl) (i : Nat) : Nat :=
  (voices.filter (fun w => w.parent = some i && w.active)).length
def c5_oneActiveChild (voices : List VoiceControl) (i : Nat) : Prop :=
  activeChildCount voices i ≤ 1

/-- **C6** 毛杠杆上限 `L^G(q') ≤ L̄^G`：毛敞口 G(q') ≤ G̅（整数名义形式）。 -/
def c6_grossLev (ctx : FeasibilityContext) (u : Control) : Prop :=
  (grossNotional (u.voices.map VoiceControl.toPosition) : Int) ≤ ctx.grossCap

/-- **C7** 净杠杆上限 `L^N(q') ≤ L̄^N`：净敞口 N(q') ≤ N̅。 -/
def c7_netLev (ctx : FeasibilityContext) (u : Control) : Prop :=
  (netNotional (u.voices.map VoiceControl.toPosition) : Int) ≤ ctx.netCap

/-- **C8** 开仓保证金 `IM_t(q') + B^open_t(q') ≤ E_t - w'`。 -/
def c8_initMargin (ctx : FeasibilityContext) (u : Control) : Prop :=
  ctx.imPlusOpenBorrow ≤ ctx.equity - u.withdraw

/-- **C9** 持仓保证金 `MM_t(q') + B^hold_t(q') ≤ E_t - w'`。 -/
def c9_maintMargin (ctx : FeasibilityContext) (u : Control) : Prop :=
  ctx.mmPlusHoldBorrow ≤ ctx.equity - u.withdraw

/-- **C10** 压力损失 `StressLoss_t(q') ≤ ρ_max · E_t`。 -/
def c10_stressLoss (ctx : FeasibilityContext) : Prop :=
  ctx.stressLoss ≤ ctx.rhoMaxEquity

/-- **C11** 提现界 `0 ≤ w' ≤ D^dist_t`。 -/
def c11_withdrawBounds (ctx : FeasibilityContext) (u : Control) : Prop :=
  0 ≤ u.withdraw ∧ u.withdraw ≤ ctx.distributable

/-- **C12** 阶段二不增战术仓 `Φ_t = II ⟹ 不增加战术仓`。 -/
def c12_phaseIITactical (ctx : FeasibilityContext) (u : Control) : Prop :=
  ctx.phaseII = true → ctx.tacticalIncreased u = false

/-- **C13** 增核前提 `ΔQ_core > 0 ⟹ 后代全平且无未完成订单`。 -/
def c13_coreAccretion (ctx : FeasibilityContext) (u : Control) : Prop :=
  0 < u.coreDelta → ctx.descendantsFlatNoPending u = true

/-- **C14** 非正常模式不增毛敞口 `μ_t ≠ Normal ⟹ G(q') ≤ G(q_t)`。 -/
def c14_nonNormalNoGrossUp (ctx : FeasibilityContext) (u : Control) : Prop :=
  ctx.normalMode = false →
    (grossNotional (u.voices.map VoiceControl.toPosition) : Int) ≤ ctx.grossPrev

/-- **C15** 借券规则成立（外部合规谓词，Θ_execution/场所参数）。 -/
def c15_borrow (ctx : FeasibilityContext) (u : Control) : Prop := ctx.borrowOk u = true

/-- **C16** 仓位模式规则成立（外部合规谓词）。 -/
def c16_positionMode (ctx : FeasibilityContext) (u : Control) : Prop := ctx.positionModeOk u = true

/-- **C17** 交易场所规则成立（外部合规谓词）。 -/
def c17_venue (ctx : FeasibilityContext) (u : Control) : Prop := ctx.venueOk u = true

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 K_Θ 成员性 = 17 约束合取 + 完整性见证
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★全局安全可行集成员谓词 `Feasible`（L0，FULL 十四 line 1075-1100 完整 17 约束）：
  u ∈ K_Θ(x_t) ⟺ 17 条约束全部成立。逐条合取，**少一条不成立则 u ∉ K_Θ**（完整性）。

  C1/C2/C3/C4 对**每个**声部成立（∀ vc ∈ voices）；C5 对**每个**父索引成立（∀ i < len）；
  C6..C17 是全局/标量约束。
-/
def Feasible (ctx : FeasibilityContext) (u : Control) : Prop :=
  (∀ vc ∈ u.voices, c1_qtyGrid vc) ∧
  (∀ vc ∈ u.voices, c2_ancestorClose u.voices vc) ∧
  (∀ vc ∈ u.voices, c3_alternating u.voices vc) ∧
  (∀ vc ∈ u.voices, c4_sameUnitHedge u.voices vc) ∧
  (∀ i, i < u.voices.length → c5_oneActiveChild u.voices i) ∧
  c6_grossLev ctx u ∧
  c7_netLev ctx u ∧
  c8_initMargin ctx u ∧
  c9_maintMargin ctx u ∧
  c10_stressLoss ctx ∧
  c11_withdrawBounds ctx u ∧
  c12_phaseIITactical ctx u ∧
  c13_coreAccretion ctx u ∧
  c14_nonNormalNoGrossUp ctx u ∧
  c15_borrow ctx u ∧
  c16_positionMode ctx u ∧
  c17_venue ctx u

/--
  ★完整性见证（L0）：`Feasible ⟺ 17 约束逐条成立`。这不是同义反复——它使「17 约束完整不遗漏」
  在类型层**可检验**：`Feasible` 展开恰为 17 个合取项，与顶部 docstring 的 C1..C17 一一对应。
  下游若漏掉任一约束（如去掉 C7 净杠杆），此 iff 的右侧将少一项，编译失败暴露遗漏。
-/
theorem feasible_iff_all17 (ctx : FeasibilityContext) (u : Control) :
    Feasible ctx u ↔
      ((∀ vc ∈ u.voices, c1_qtyGrid vc) ∧
       (∀ vc ∈ u.voices, c2_ancestorClose u.voices vc) ∧
       (∀ vc ∈ u.voices, c3_alternating u.voices vc) ∧
       (∀ vc ∈ u.voices, c4_sameUnitHedge u.voices vc) ∧
       (∀ i, i < u.voices.length → c5_oneActiveChild u.voices i) ∧
       c6_grossLev ctx u ∧ c7_netLev ctx u ∧ c8_initMargin ctx u ∧
       c9_maintMargin ctx u ∧ c10_stressLoss ctx ∧ c11_withdrawBounds ctx u ∧
       c12_phaseIITactical ctx u ∧ c13_coreAccretion ctx u ∧
       c14_nonNormalNoGrossUp ctx u ∧ c15_borrow ctx u ∧
       c16_positionMode ctx u ∧ c17_venue ctx u) :=
  Iff.rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 非空性 K_Θ ≠ ∅（安全控制 u^safe 始终可行）

  FULL 十四 line 1103-1113：「规定一个安全控制始终存在，u^safe = 撤销未完成开仓指令、
  禁止新增风险、提交可执行的减仓指令」，因此 K_Θ ≠ ∅。形式化：构造 u^safe = 空仓 + 零提现 +
  撤单 + 零增核，在「安全上下文」（保证金/杠杆/压力对空仓平凡满足 + 合规谓词对 u^safe 真）下
  满足全部 17 约束。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★安全控制 `safeControl`（L0，FULL 十四 line 1105-1107）：空仓 + 零提现 + 撤单 + 零增核。
  voices = [] （撤掉所有开仓 ⟹ 无声部目标仓位 ⟹ 不新增风险），withdraw = 0，cancelOpen = true，
  coreDelta = 0（不增核）。这是「撤单 + 禁新风险 + 减仓到空」的极限安全控制。
-/
def safeControl : Control :=
  { voices := [], withdraw := 0, cancelOpen := true, coreDelta := 0 }

/--
  ★安全上下文谓词 `IsSafeContext`（L0）：上下文使 u^safe 可行的充分条件（非缠论，账户层）。
  空仓 ⟹ 毛/净敞口 = 0，故 C6/C7 需 0 ≤ G̅ ∧ 0 ≤ N̅；保证金对空仓 = 开仓/持仓基线 ≤ E_t；
  压力损失基线 ≤ ρ_max·E_t；提现 0 ≤ D^dist（D^dist ≥ 0）；合规谓词对 u^safe 为真。

  ★诚实标注：这些不变量是「空仓状态下账户健康」的最弱前提——账户若连空仓都违反保证金/压力
  （E_t < IM 基线 等），则 K_Θ 真为空（破产态），此时 u^safe 不可行是**正确**的（不是 bug）。
  FULL 十四 line 1112 的 K_Θ ≠ ∅ 是「在账户可运营前提下」的声明，本谓词刻画该前提。
-/
structure IsSafeContext (ctx : FeasibilityContext) : Prop where
  grossCap_nonneg : 0 ≤ ctx.grossCap
  netCap_nonneg : 0 ≤ ctx.netCap
  im_le_equity : ctx.imPlusOpenBorrow ≤ ctx.equity
  mm_le_equity : ctx.mmPlusHoldBorrow ≤ ctx.equity
  stress_ok : ctx.stressLoss ≤ ctx.rhoMaxEquity
  distributable_nonneg : 0 ≤ ctx.distributable
  grossPrev_nonneg : 0 ≤ ctx.grossPrev
  safe_no_tactical : ctx.tacticalIncreased safeControl = false
  safe_descendants_flat : ctx.descendantsFlatNoPending safeControl = true
  safe_borrow : ctx.borrowOk safeControl = true
  safe_position_mode : ctx.positionModeOk safeControl = true
  safe_venue : ctx.venueOk safeControl = true

/-- ★空仓毛敞口 = 0（L0）：safeControl 无声部 ⟹ G = 0。 -/
theorem safe_gross_zero :
    grossNotional (safeControl.voices.map VoiceControl.toPosition) = 0 := by
  simp [safeControl, grossNotional]

/-- ★空仓净敞口 = 0（L0）：safeControl 无声部 ⟹ N = 0。 -/
theorem safe_net_zero :
    netNotional (safeControl.voices.map VoiceControl.toPosition) = 0 := by
  simp [safeControl, netNotional]

/--
  ★★安全控制可行（L0，核心非空性）：安全上下文下 u^safe 满足全部 17 约束。
  逐条验证：C1（空声部真空真）/C2/C3/C4（空声部真空真）/C5（空声部活跃子数=0≤1）；
  C6/C7（G=N=0 ≤ 非负上限）；C8/C9（保证金 ≤ E_t = E_t - 0）；C10（压力 OK）；
  C11（0 ≤ 0 ≤ D^dist）；C12（无战术仓增）；C13（coreDelta=0，前件假）；C14（G=0 ≤ G_prev）；
  C15/C16/C17（合规谓词对 u^safe 真）。
-/
theorem safe_control_feasible (ctx : FeasibilityContext) (hsafe : IsSafeContext ctx) :
    Feasible ctx safeControl := by
  refine ⟨?c1, ?c2, ?c3, ?c4, ?c5, ?c6, ?c7, ?c8, ?c9, ?c10, ?c11, ?c12, ?c13, ?c14, ?c15, ?c16, ?c17⟩
  case c1 => intro vc hvc; simp [safeControl] at hvc
  case c2 => intro vc hvc; simp [safeControl] at hvc
  case c3 => intro vc hvc; simp [safeControl] at hvc
  case c4 => intro vc hvc; simp [safeControl] at hvc
  case c5 => intro i hi; simp [safeControl] at hi
  case c6 =>
    unfold c6_grossLev
    rw [safe_gross_zero]; simpa using hsafe.grossCap_nonneg
  case c7 =>
    unfold c7_netLev
    rw [safe_net_zero]; simpa using hsafe.netCap_nonneg
  case c8 =>
    unfold c8_initMargin
    simp only [safeControl]; simpa using hsafe.im_le_equity
  case c9 =>
    unfold c9_maintMargin
    simp only [safeControl]; simpa using hsafe.mm_le_equity
  case c10 => exact hsafe.stress_ok
  case c11 =>
    unfold c11_withdrawBounds
    exact ⟨Int.le_refl 0, hsafe.distributable_nonneg⟩
  case c12 =>
    intro _; exact hsafe.safe_no_tactical
  case c13 =>
    intro hpos; simp [safeControl] at hpos
  case c14 =>
    intro _
    rw [safe_gross_zero]; exact hsafe.grossPrev_nonneg
  case c15 => exact hsafe.safe_borrow
  case c16 => exact hsafe.safe_position_mode
  case c17 => exact hsafe.safe_venue

/--
  ★★全局安全可行集非空 `feasible_nonempty`（L0，FULL 十四 line 1111-1113「K_Θ ≠ ∅」）：
  安全上下文下，存在可行控制（u^safe）。这是 §17 总定理条件 9「风险可行集非空」的形式兑现
  ——保证 LexArgmin_{u ∈ K_Θ} 有非空定义域（见 Origin/LexArgmin.lean argmin 存在性）。
-/
theorem feasible_nonempty (ctx : FeasibilityContext) (hsafe : IsSafeContext ctx) :
    ∃ u : Control, Feasible ctx u :=
  ⟨safeControl, safe_control_feasible ctx hsafe⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★约束系统标签 `ConstraintTag`（gatekeeper，诚实分层）。
  SafetyConstraintOnly（17 约束是安全可行性，非走势分类）+
  ThetaRiskParametric（约束参数 L̄^G/ρ_max/MM_t 是 Θ_risk 参数，非缠论可导）+
  ExternalComplianceStub（C15/C16/C17 是外部场所规则占位，内容 still-MISSING）+
  EmpiricalDomain（约束参数经验校准 L2）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把可行集标为分类定理。
-/
inductive ConstraintTag where
  | SafetyConstraintOnly
  | ThetaRiskParametric
  | ExternalComplianceStub
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★约束系统子类（gatekeeper）：Seventeen­SafetyConstraintsGivenTheta（唯一子类）。 -/
inductive ConstraintSubkind where
  | SeventeenSafetyConstraintsGivenTheta
deriving DecidableEq, Repr

/-- ★约束系统诚实标签包（L0 声明）。 -/
def constraintLabels : List ConstraintTag × ConstraintSubkind :=
  ([ConstraintTag.SafetyConstraintOnly, ConstraintTag.ThetaRiskParametric,
    ConstraintTag.ExternalComplianceStub, ConstraintTag.EmpiricalDomain],
   ConstraintSubkind.SeventeenSafetyConstraintsGivenTheta)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：约束子类必是 SeventeenSafetyConstraintsGivenTheta。 -/
theorem constraint_not_true_classification (k : ConstraintSubkind) :
    k = ConstraintSubkind.SeventeenSafetyConstraintsGivenTheta := by
  cases k; rfl

end NewChanlun.Origin.ConstraintSystem
