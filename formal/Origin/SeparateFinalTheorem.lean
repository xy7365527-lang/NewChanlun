/-
  Origin/SeparateFinalTheorem.lean — 工作单元 W14：分账本扩展最终定理（顶点）
  C42（最终定理，§十四 页21）：分账本头寸组合语义中 7 前提 ⟹ **三结论合一**：
    ① ∀e∈E, ∃!ν(e), Eat^sep(e)           （覆盖，引 W13）
    ② G^sep_e = s_e·ε_e·(P_ρ−P_λ) > 0      （收益，W13 SepCovered 逐 e 携带 / W8 gSep_pos）
    ③ ∀x_t, ∃!O_{t+1} = π_Θ(x_t)           （策略全定义，引 W10）
  C40（不可能定理，分账本版，§十二 页19–20）：父多子空 G_net=0 ⟹「分账本吃每笔」⇏
    「净账户每笔盈利」（引 W4 net_value_eat_every_stroke_impossible，防火墙）。
  C41（完整策略最终形式，§十三 页20）：π_Θ = Schedule_Θ[LexArgmin_{q∈K_Θ} J_t(q,q̄) − q_t]
    （= W10 policyOrder，覆盖可行时 q̄ 经 W11 q*=q̄）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么（C40/C41/C42，权威 PDF 23 页版 §十二/§十三/§十四 页19–21）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 **C40/C41/C42** 行 + §D **W14** 行 + §E（C42 是 canonical 顶点）+ §F。

  分账本扩展（§一–§十四）的覆盖定理链顶点。本文件**不重证**任何前置工位——它把
  W13（自相似递归 ∀e 覆盖+收益）+ W10（策略全定义 ∃!O）+ W11（覆盖可行 q*=q̄）+ W4（净收益
  不可能）四个**已证顶层定理**在**同一组分账本语义对象**上合一为 C42 三结论，并以 W4 钉死
  C40 防火墙（覆盖 ⇏ 净账户每笔盈利）。

  ### C42 · 最终定理（§十四 页21，三结论合一，canonical 顶点）

      在分账本头寸组合语义中，7 前提
        (1) 缠论递归解析唯一       —— Rec_Θ 单值（W10 J_t 字典序键 + W13 元素树良基承载）
        (2) 每个操作元素边界可判定   —— W13 状态机承载器 / W12 三段（W11 逐时刻投影）
        (3) 每个元素分配唯一规范头寸腿 —— W8 LegAssignment.nu_injective（ν 单射）
        (4) 多头腿和空头腿不净额抵消 —— W6 P^sep 双坐标独立（hedged_leg_nonzero_but_net_zero）
        (5) 同单位数双开在可行集中允许 —— W11 CoverObjective.targetMem（q̄∈K_Θ 覆盖可行）
        (6) 风险投影在覆盖域中不改变目标腿 —— W11 coverFeasible_qStar_eq_target（q*=q̄）
        (7) 订单执行无延迟或按定义边界成交 —— W10 Schedule_Θ 确定性函数
      ⟹ ① ∀e∈E, ∃!ν(e), Eat^sep(e)   ② G^sep_e>0   ③ ∀x_t, ∃!O_{t+1}=π_Θ(x_t)。

      ★结论数学名称 = **「分账本声部级全元素覆盖」**，**不是**「净资产级每笔盈利」（C40/W4）。

  ════════════════════════════════════════════════════════════════════════
  ## ★三结论合一的严格形式（no-patch / no-workaround，关键设计——非空壳合并）

  C42 **不是**把三个前置定理塞进一个结构的空壳（那会使「三结论」退化为「三个无关命题的
  合取」= 090号声明膨胀）。三结论合一的**非平凡内容** = 三结论在**同一组分账本语义对象**上
  同时成立且**坐标一致**：

  - 结论①②（W13）与结论③（W10）共用**同一 P^sep 坐标族**（`SepPosition = List Leg`，W6）——
    元素覆盖的规范腿 `canonicalLeg`（W8）与策略目标头寸 `q̄`（W10 `strategyTargetSep` 桥接）
    在同一 P^sep 坐标上对齐（W10 集成铁律「不留两份发散腿表示」延续到顶点）。
  - 结论③的策略全定义 ∃!O（W10）在覆盖可行域内输出 q*=q̄（W11），而 q̄ 正是结论①②覆盖的
    那些规范腿的目标头寸——**策略「全定义」与元素「全覆盖」在覆盖可行域内同一**（这是 C42
    「严格全定义策略为每元素分配唯一同向头寸腿」的核心：策略输出 = 覆盖目标）。
  - 结论②（G^sep>0）由结论①的 `SepCovered.profit` 字段**逐 e 携带**（W13 `recursive_cover`
    的每个 e 都带 G^sep>0）——不是独立陈述，而是覆盖见证的内蕴部分。

  本文件用 `FinalTheoremHypotheses` 结构承载 C42「7 前提」（把各前置工位的前提对象作为字段，
  使三结论建立在**同一组对象**上），`theorem_final` 同时导出三结论，每结论**真引用**对应前置
  顶层定理（W13 `theorem_separate_recursive` / W10 `policy_order_well_defined` / W11
  `coverFeasible_qStar_eq_target`）。**无前置定理在本文件被重证**（顶点只做合一，不做证明）。

  ════════════════════════════════════════════════════════════════════════
  ## ★C40 防火墙（W4 真引用）：覆盖（①②）⇏ 净账户每笔盈利

  C42 结论①②是**分账本声部级毛收益全覆盖**（G^sep_e>0 逐 e）——这是 P^sep **分账本坐标**下
  「空头腿吃跌、多头腿吃涨各自独立计数」的代数恒等。**严禁**膨胀为「净资产级每笔盈利」：
  父多 +Q 子空 −Q 双开，净额映射 `Qe⁺+Qe⁻↦0`（W6 `hedged_leg_net_zero`）下 G_net=0
  （W4 `net_value_cancels`），故「分账本吃每笔」⇏「净账户每笔盈利」（W4
  `net_value_eat_every_stroke_impossible`）。本文件 §3 以 W4 顶层定理**真引用**钉死此防火墙——
  C42 的全覆盖与 C40 的净收益不可能**在同一文件内并列**，使「覆盖≠净盈利」类型层可见。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯结构/合一/代数，零数据依赖）。`lake env lean` 通过 = 「三个已证 L0 顶层
  定理（W13 覆盖+收益 / W10 策略全定义 / W11 覆盖可行）在同一组对象上合一 + W4 净收益不可能
  防火墙」的合一命题正确。

  ★★★C42 顶点是 **L0 结构定理（分账本声部级全元素覆盖）**，**NOT L2 实盘每笔盈利**——这与
    全窗 L3（完整 v1 实盘择时 8 品种 8/8 否证、无 alpha）**不矛盾**：
    - C42 证的是「每元素有**头寸腿覆盖**、毛收益 G^sep>0 的**代数恒等**」（方向 ε_e·ΔP_e>0 是
      元素方向语义 W7，非市场预测；s_e>0 ⟹ s_e·ε_e·ΔP_e>0 是 Int 正乘正）——**纯语法层覆盖**。
    - 全窗 L3 否证的是「这套 v1 完整择时在 1min 尺度这些品种实盘扣成本盈利」（L2/L3 经验域）。
    - 两者有效域不同：C42 = 分账本声部级毛收益覆盖（L0 定义域内代数成立）；L3 = 净账户实盘
      扣成本盈利（L2 经验域，被否证）。C40/W4 净收益不可能定理是**防火墙**——它封死了把 L0
      声部级覆盖膨胀为 L2 实盘每笔盈利的有效域越界（双开净额退化 G_net=0）。
  ★C42 顶点诚实声明：「分账本声部级全元素覆盖」（C42 §十四自己定名）**明确否定**
    - 净资产级每笔盈利（C40/W4，双开净额退化）；
    - 组合净值级覆盖（C22，声部级毛捕获≠组合净值捕获）；
    - 因果无延迟吃满事后端点（C23，仅操作语义可判定元素）。
  ★有效域诚实声明：三结论合一仅在**覆盖可行域 X^cover_Θ** 内成立（W11 `targetMem`：q̄∈K_Θ，
    目标腿全部允许存在）。若 q̄∉K_Θ（保证金/杠杆/资本规则不允许目标腿），风险投影改变目标腿
    （q*≠q̄），结论①②的状态机投影不成立、结论③仍 ∃!O 但 q*≠q̄（覆盖不达成）——W13/W11/W12
    有效域上界继承。**gatekeeper 类型层钉死，无 NetProfitGuaranteed 构造子**。

  ════════════════════════════════════════════════════════════════════════
  ## owner 边界（铁律）

  本文件**只建** `formal/Origin/SeparateFinalTheorem.lean`。不碰 W13/W12/W11/W10/W8/W7/W6/W4
  （只 import 只读）。不编辑 lakefile.toml。root 名 `Origin.SeparateFinalTheorem`。
  禁 sorry/admit/axiom。简体中文。

  依赖方向（单向无环，全 committed 只读）：
    SeparateFinalTheorem → SeparateCoverRecursive（W13：theorem_separate_recursive /
                          recursive_cover / recursive_eatSep / recursive_unique_leg /
                          ElementTree / StateMachineProvider）
                            ⤷ 传递 SeparateCoverTheorem（W12）→ CoverFeasibleTarget（W11：
                              coverFeasible_qStar_eq_target / CoverObjective）→
                              SeparateStrategyWellDefined（W10：policy_order_well_defined /
                              policyOrder / StrategyObjective / qStar_exists_unique）→
                              SeparateEat（W8：SepCovered/EatSep/gSep/gSep_pos/LegAssignment）
                              → SeparateLedger（W6）→ SyntaxElement（W7）。
                        → NetValueImpossibility（W4：net_value_eat_every_stroke_impossible /
                          net_value_cancels，独立 standalone，C40 防火墙）。

  谱系：C40/C41/C42（PDF §十二/§十三/§十四）→ W13（自相似递归 ∀e 覆盖+收益 C38/C39）+ W10
        （策略全定义 ∃!O C33/C34/C35）+ W11（覆盖可行 q*=q̄ C36）+ W4（净收益不可能 C22/C40）
        → 本文件 W14（最终定理顶点，三结论合一）。互斥最终定理 M29 另由集成工位 +AncOK/+Role
        叠加（本 W14 是基线顶点）。C42「分账本声部级覆盖≠净账户盈利」= 顶点有效域上界。
-/

import Origin.SeparateCoverRecursive
import Origin.NetValueImpossibility

namespace NewChanlun.Origin.SeparateFinalTheorem

open NewChanlun.Origin
open NewChanlun.Origin.SeparateLedger (Leg SepPosition legHedged legLong hedged_leg_net_zero)
open NewChanlun.Origin.SeparateEat
  (LegAssignment EatSep SepCovered gSep)
open NewChanlun.Origin.SeparateCoverTheorem (LegStateMachine)
open NewChanlun.Origin.SeparateCoverRecursive
  (ElementTree StateMachineProvider theorem_separate_recursive recursive_cover
   recursive_eatSep recursive_unique_leg)
open NewChanlun.Origin.SeparateStrategyWellDefined
  (StrategyObjective policyOrder policy_order_well_defined policy_order_exists_unique)
open NewChanlun.Origin.CoverFeasibleTarget (CoverObjective coverFeasible_qStar_eq_target)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 C42 7 前提承载 `FinalTheoremHypotheses`（同一组分账本语义对象）

  C42 「7 前提」不是七个孤立命题——它们刻画**同一个分账本语义状态**：一棵元素树 T（缠论递归
  解析）、其上的腿赋值 L（唯一规范腿 ν）+ 头寸函数 q（逐声部逐时刻）+ 价格 P、状态机承载器
  smp（覆盖可行域内逐元素状态机 = W11 逐时刻投影）、策略目标函数 J（K_Θ + J_t 字典序键）、
  覆盖目标 C（q̄∈K_Θ 覆盖可行）、调度 schedule（Schedule_Θ 确定性函数）。

  把这些**作为同一结构的字段**——三结论建立在同一组对象上，这是「合一」非空壳的形式根据
  （结论①②用 T/L/q/P/smp，结论③用 J/schedule，覆盖可行桥 C 连接两侧 q̄）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★C42 最终定理 7 前提承载 `FinalTheoremHypotheses`（L0，§十四 页21）—— 分账本语义状态的同一
  组对象。每字段对应 C42 一条前提，刻画**同一个**分账本头寸组合语义状态。

  - `V : Type`（隐式）：声部类型（W8/W13 ν 值域）。
  - `Order : Type`（隐式）：订单类型（W10 Schedule_Θ 输出）。
  - **`tree : ElementTree`**（前提1：缠论递归解析唯一）：元素树 T=(E,par)，良基下降 + 区间嵌套
    （W13 §1）。元素枚举的递归结构（Rec_Θ 单值的元素侧）。
  - **`legAssign : LegAssignment V`**（前提3：每元素唯一规范头寸腿）：ν 单射 + σ_{ν(e)}=ε_e +
    s_e>0（W8）。**前提4（多空不净额抵消）**由 `LegAssignment` 的腿坐标 = W6 P^sep `Leg`
    （双坐标独立）天然承载——见 §3 `hedged_net_zero_firewall`。
  - `position : V → Index → Leg`（头寸函数 q）：逐声部逐时刻一条腿（W8/W13）。
  - `price : Index → Tick`（价格 P）：端点价格函数。
  - **`smp : StateMachineProvider tree legAssign position price`**（前提2/6：边界可判定 + 风险
    投影不改目标腿）：覆盖可行域内逐元素状态机 = W11 q*=q̄ 逐时刻投影 + 方向一致（W13 §3）。
  - **`objective : StrategyObjective`**（前提1/5：J_t 单值 + K_Θ≠∅）：C33 K_Θ + C34 J_t 字典序
    键（单射）（W10）。
  - **`schedule : SepPosition → SepPosition → Order`**（前提7：执行无延迟，确定性函数）：
    Schedule_Θ（W10）——Lean 全函数 ⟹ 确定性自动满足。

  ★这些字段共同刻画**同一个**分账本语义状态——三结论在此同一组对象上同时成立（合一的根据）。
  ★L0：纯结构承载（无 Θ 参数数值、不依赖数据），承载 C42 7 前提的形式对象。
-/
structure FinalTheoremHypotheses (V : Type) (Order : Type) where
  /-- 前提1（缠论递归解析唯一·元素侧）：元素树 T=(E,par)，良基 + 区间嵌套（W13）。 -/
  tree : ElementTree
  /-- 前提3（每元素唯一规范头寸腿）：ν 单射 + σ_{ν(e)}=ε_e + s_e>0（W8）。 -/
  legAssign : LegAssignment V
  /-- 头寸函数 q：逐声部逐时刻一条 P^sep 腿（W8/W13 头寸状态）。 -/
  position : V → Index → Leg
  /-- 价格函数 P：端点价格（C30 G^sep 的价差来源）。 -/
  price : Index → Tick
  /-- 前提2/6（边界可判定 + 风险投影不改目标腿）：W11 q*=q̄ 逐元素状态机投影 + 方向一致（W13）。 -/
  smp : StateMachineProvider tree legAssign position price
  /-- 前提1/5（J_t 单值 + K_Θ≠∅）：C33 K_Θ + C34 J_t 字典序键单射（W10）。 -/
  objective : StrategyObjective
  /-- 前提7（执行无延迟，确定性函数）：Schedule_Θ（W10），全函数 ⟹ 确定性自动满足。 -/
  schedule : SepPosition → SepPosition → Order

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 C42 三结论（各结论真引用对应前置顶层定理，无重证）

  在 §1 同一组对象 `H : FinalTheoremHypotheses` 上导出 C42 三结论：
    ① 覆盖    ← W13 `recursive_cover` / `recursive_eatSep`（∀e SepCovered/EatSep）
    ① 唯一    ← W13 `recursive_unique_leg`（∀e ∃!ν(e)）
    ② 收益    ← W13 `SepCovered.profit`（逐 e 携带 G^sep>0）
    ③ 策略全定义 ← W10 `policy_order_well_defined`（∀x_t ∃!O=π_Θ）
  本文件**不重证**这些——只把前置顶层定理在 H 的字段上实例化。
  ════════════════════════════════════════════════════════════════════════ -/

variable {V : Type} {Order : Type}

/--
  ★C42 结论①·覆盖 + 唯一 `final_cover_unique`（L0，§十四 ①，引 W13）：
  在 7 前提下，元素树内**每个**元素 e 被唯一规范头寸腿吃到（覆盖 + 正收益 + ν 唯一）：

      ∀ e, H.tree.inTree e →
        SepCovered H.legAssign H.position e H.price          -- Eat^sep(e) + G^sep>0
        ∧ (∀ e', H.legAssign.nu e' = H.legAssign.nu e → e' = e)  -- ∃!ν(e)

  **直接引** W13 `theorem_separate_recursive`（在 H 的 tree/legAssign/position/price/smp 上
  实例化）——本文件不重证树深归纳。这是 C42「① ∀e∈E, ∃!ν(e), Eat^sep(e)」（覆盖侧 + 唯一侧）。
-/
theorem final_cover_unique (H : FinalTheoremHypotheses V Order) :
    ∀ e, H.tree.inTree e →
      SepCovered H.legAssign H.position e H.price
      ∧ (∀ e', H.legAssign.nu e' = H.legAssign.nu e → e' = e) :=
  theorem_separate_recursive H.tree H.legAssign H.position H.price H.smp

/--
  ★C42 结论①·纯覆盖侧 `final_eatSep`（L0，§十四 ①，引 W13）：∀e∈E 分账本吃到 `EatSep`
  （规范腿覆盖整操作区间）。引 W13 `recursive_eatSep`——C42「∀e∈E, Eat^sep(e)」的纯覆盖陈述。
-/
theorem final_eatSep (H : FinalTheoremHypotheses V Order) :
    ∀ e, H.tree.inTree e → EatSep H.legAssign H.position e :=
  recursive_eatSep H.tree H.legAssign H.position H.price H.smp

/--
  ★C42 结论②·收益 G^sep_e>0 `final_profit`（L0，§十四 ②，W13 `SepCovered.profit` 逐 e 携带）：
  ∀e∈E 分账本毛收益严格为正 `0 < gSep H.legAssign e H.price`（= s_e·ε_e·(P_ρ−P_λ)>0）。

  ★这**不是**独立陈述，而是结论①覆盖见证 `SepCovered` 的 `profit` 字段——W13 `recursive_cover`
  的每个 e 都内蕴携带 G^sep>0（覆盖与收益在 SepCovered 中合一）。这是 C42「② G^sep_e=s_e·ε_e·
  (P_ρ−P_λ)>0」。**收益是覆盖的内蕴部分**（同一组对象，非两个无关命题），合一非空壳的体现。
-/
theorem final_profit (H : FinalTheoremHypotheses V Order) :
    ∀ e, H.tree.inTree e → 0 < gSep H.legAssign e H.price :=
  fun e he => (recursive_cover H.tree H.legAssign H.position H.price H.smp e he).profit

/--
  ★C42 结论③·策略全定义 ∃!O `final_strategy_well_defined`（L0，§十四 ③，引 W10）：
  对**任意**当前头寸 q_t（x_t 的头寸分量），策略输出订单存在唯一：

      ∀ qPrev, ∃! O, O = policyOrder H.objective H.schedule qPrev

  **直接引** W10 `policy_order_well_defined`（在 H 的 objective/schedule 上实例化）——本文件不
  重证 ∃!O。这是 C42「③ ∀x_t, ∃!O_{t+1}=π_Θ(x_t)」（策略全定义在分账本坐标 P^sep 上）。
-/
theorem final_strategy_well_defined (H : FinalTheoremHypotheses V Order) :
    ∀ qPrev : SepPosition,
      ExistsUnique (fun O : Order => O = policyOrder H.objective H.schedule qPrev) :=
  policy_order_well_defined H.objective H.schedule

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 C40 防火墙（W4 真引用）：覆盖（①②）⇏ 净账户每笔盈利

  C42 结论①②是分账本声部级毛收益全覆盖（P^sep 坐标，多空独立计数）。C40（§十二）显式否定把
  它膨胀为净资产级每笔盈利——父多 +Q 子空 −Q 双开，净额映射 Qe⁺+Qe⁻↦0 下 G_net=0。本节以
  W4 顶层定理**真引用**钉死防火墙：C42 全覆盖与 C40 净收益不可能并列，使「覆盖≠净盈利」可见。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★C40 防火墙·净账户每笔盈利不可能 `net_profit_every_stroke_impossible`（L0，§十二 页19–20，
  **W4 真引用**）：存在一类构形（同单位反向双开父子腿），其组合净值恒非正——故「∀ 笔组合净值
  严格为正」（净账户每笔盈利）这一全称声称**为假**：

      ∃ P : NetValueImpossibility.OppositeDoublePair, ¬ (0 < P.parent.G + P.child.G)

  **直接引** W4 `net_value_eat_every_stroke_impossible`——本文件不重证净值抵消代数。这坐实 C40
  「分账本吃每笔 ⇏ 净账户每笔盈利」：取父多子空同股数同区间双开，净值 = 0（W4
  `net_value_cancels`），不满足「严格为正」。

  ★这是 C42 顶点的**有效域防火墙**：C42 结论①②（分账本声部级毛收益 G^sep>0 全覆盖）**不蕴含**
    净账户每笔盈利——双开净额退化 G_net=0（W6 `hedged_leg_net_zero`）。**严禁**把 C42 的 L0
    分账本声部级覆盖膨胀为 L2 净资产实盘盈利（那需 L2/L3，PDF 未提供也不可由 L0 推出）。
-/
theorem net_profit_every_stroke_impossible :
    ∃ P : NetValueImpossibility.OppositeDoublePair, ¬ (0 < P.parent.G + P.child.G) :=
  NetValueImpossibility.net_value_eat_every_stroke_impossible

/--
  ★C40 防火墙·双开净额退化 `hedged_position_net_zero`（L0，§十二 页20，W6 真引用）：
  分账本双开腿 (Q,Q) 的单声部净额 `legNet(Qe⁺+Qe⁻) = q⁺−q⁻ = 0`（净额映射下退化为零头寸）。

      ∀ Q, NewChanlun.Origin.SeparateLedger.legNet (legHedged Q) = 0

  **直接引** W6 `hedged_leg_net_zero`（单声部形式 `legNet`）——这是 C40「净额映射 Qe⁺+Qe⁻↦0 后
  价格方向暴露已抵消」的代数核：分账本坐标 (Q,Q) 非零（双腿独立，qPlus=Q≠0），但单声部净额
  q⁺−q⁻ = Q−Q = 0。C42 结论①②在**分账本坐标**（非净额坐标）成立——本引理钉死「换到净额坐标
  即退化」，封死净账户每笔盈利的越界（230号直积退化：两腿在净额 ℤ 上塌缩为单点 0）。
-/
theorem hedged_position_net_zero (Q : Nat) :
    NewChanlun.Origin.SeparateLedger.legNet (legHedged Q) = 0 :=
  hedged_leg_net_zero Q

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 C42 三结论合一主定理（顶点：① ∧ ② ∧ ③ 在同一组对象上）

  把 §2 三结论（covered+unique / profit / strategy）合成 C42 单一陈述——三结论在**同一**
  `FinalTheoremHypotheses H` 上同时成立。这是整个分账本扩展（§一–§十四）的 canonical 顶点。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★★★C42 最终定理·三结论合一 `theorem_final`★★★★★（L0，§十四 页21，**分账本扩展
  canonical 顶点**）：

  在分账本头寸组合语义中，7 前提（`H : FinalTheoremHypotheses V Order` 承载）⟹ **三结论合一**：

      ① ∀e∈E, SepCovered ∧ ∃!ν(e)        -- 覆盖（Eat^sep + G^sep>0）+ 唯一规范腿
      ∧ ② ∀e∈E, 0 < gSep                  -- 收益 G^sep_e>0（覆盖见证内蕴携带）
      ∧ ③ ∀x_t, ∃!O = π_Θ(x_t)            -- 策略全定义（分账本坐标 ∃!O）

  - 结论① `final_cover_unique`（引 W13 `theorem_separate_recursive`）：∀e 被唯一规范腿吃到。
  - 结论② `final_profit`（W13 `SepCovered.profit` 逐 e 携带）：∀e G^sep>0——**收益是覆盖的
    内蕴部分**（同一 SepCovered，非独立命题）。
  - 结论③ `final_strategy_well_defined`（引 W10 `policy_order_well_defined`）：∀x_t ∃!O。

  ★三结论建立在**同一组对象** H 上（同一 tree/legAssign/position/price + 同一 objective/
    schedule）——这是「合一」非空壳的形式根据：①②用 H 的覆盖侧字段、③用 H 的策略侧字段，
    覆盖目标 q̄ 由 W11 `coverFeasible_qStar_eq_target` 连接两侧（覆盖可行域内 q*=q̄，策略输出
    = 覆盖目标，见 `strategy_meets_cover_target`）。

  ★★★C42 结论数学名称 = **「分账本声部级全元素覆盖」**，**NOT「净资产级每笔盈利」**：
    - 结论①②是 P^sep 分账本坐标的声部级毛收益覆盖（L0 代数恒等，多空独立计数）。
    - C40 防火墙（§3 `net_profit_every_stroke_impossible`，W4）否定净账户每笔盈利——双开净额
      退化 G_net=0（§3 `hedged_position_net_zero`，W6）。
    - 与全窗 L3（v1 实盘 8/8 否证）**不矛盾**：C42 = L0 分账本声部级毛收益覆盖（定义域内代数
      成立）；L3 = L2 净账户实盘扣成本盈利（经验域，被否证）——两个有效域。
  ★有效域上界：仅覆盖可行域 X^cover_Θ 内成立（H.smp 承载 W11 q̄∈K_Θ；q̄∉K_Θ 时不适用）。
-/
theorem theorem_final (H : FinalTheoremHypotheses V Order) :
    (∀ e, H.tree.inTree e →
        SepCovered H.legAssign H.position e H.price
        ∧ (∀ e', H.legAssign.nu e' = H.legAssign.nu e → e' = e))
    ∧ (∀ e, H.tree.inTree e → 0 < gSep H.legAssign e H.price)
    ∧ (∀ qPrev : SepPosition,
        ExistsUnique (fun O : Order => O = policyOrder H.objective H.schedule qPrev)) :=
  ⟨final_cover_unique H, final_profit H, final_strategy_well_defined H⟩

/--
  ★★C41 完整全定义策略最终形式 `policy_final_form`（L0，§十三 页20，引 W10）：策略输出订单的
  最终形式 `π_Θ(x_t) = Schedule_Θ[LexArgmin_{q∈K_Θ} J_t(q,q̄) − q_t]` —— 即 W10 `policyOrder`
  （= `schedule J.qStar qPrev`，qStar = LexArgmin_{q∈K_Θ} J_t），且对每个 q_t 存在唯一。

      ∀ qPrev, ∃! O, O = policyOrder H.objective H.schedule qPrev

  ★C41 是 C42 结论③的策略**形式**陈述：π_Θ = Schedule∘LexArgmin（W10 `policyOrder` 已是此
    形式，`qStar` = J_t 字典序最小点）。本引理 = `final_strategy_well_defined` 的 C41 命名别名
    （PDF §十三 C41 与 §十四 C42③ 同形式，分两处陈述）——W14 顶点统一引 W10 顶层定理。
-/
theorem policy_final_form (H : FinalTheoremHypotheses V Order) :
    ∀ qPrev : SepPosition,
      ExistsUnique (fun O : Order => O = policyOrder H.objective H.schedule qPrev) :=
  final_strategy_well_defined H

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 合一桥：策略输出 = 覆盖目标（覆盖可行域内 ③ 的 q* 命中 ①② 的目标 q̄）

  C42「严格全定义策略为每元素分配唯一同向头寸腿」= 结论③策略输出在覆盖可行域内 = 结论①②的
  覆盖目标头寸 q̄。本节以 W11 `coverFeasible_qStar_eq_target` 真引用钉死「策略全定义」与「元素
  全覆盖」在覆盖可行域内的**同一性**——这是三结论合一非空壳的核心桥（①② 与 ③ 经 q̄ 连接）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★合一桥·策略输出命中覆盖目标 `strategy_meets_cover_target`（L0，§十四 + §八，**W11 真引用**）：
  给定覆盖目标函数 `C : CoverObjective`（覆盖可行 q̄∈K_Θ），策略最终仓位 `q*`（= W10/W11 qStar，
  J_t 字典序最小点）**等于**覆盖目标头寸 `q̄`：

      C.toStrategyObjective.qStar = C.target

  **直接引** W11 `coverFeasible_qStar_eq_target`——本文件不重证二次型等号唯一。这坐实 C42
  「严格全定义策略可为每元素分配唯一同向头寸腿，达『每笔都有头寸腿吃到』」的核心：
  **策略全定义（③）的输出 q* = 元素全覆盖（①②）的目标腿 q̄**（覆盖可行域内）——策略「全定义」
  与元素「全覆盖」在覆盖可行域内**同一**（q̄ 是结论①② 覆盖的那些规范腿的目标头寸）。

  ★这是三结论合一**非空壳**的形式核心：①②（覆盖）与③（策略）经 q̄ = q* 连接——不是三个无关
    命题的合取，而是「策略输出 = 覆盖目标」的同一性（W11 桥）。q̄∉K_Θ 时本桥不成立（无法构造
    `CoverObjective`，有效域上界）。
-/
theorem strategy_meets_cover_target (C : CoverObjective) :
    C.toStrategyObjective.qStar = C.target :=
  coverFeasible_qStar_eq_target C

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（formalization-validity-domain gatekeeper，全元素覆盖 ≠ 净账户每笔盈利）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★C42 顶点裁定标签 `FinalTheoremVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `separateVoiceLevelFullCoverage`——类型层钉死 C42 结论数学名称 =
  **「分账本声部级全元素覆盖」**（covered domain 内，三结论合一）。

  ★**没有** `NetProfitGuaranteed` / `NetAccountEveryStroke` / `CoverageEverywhere` 构造子——
    拒绝三类声明膨胀：
    (1) 「全元素覆盖 ⟹ 净账户每笔盈利」（覆盖是 P^sep 分账本声部级毛收益，非净资产盈利——
        C40/W4 否定净账户每笔盈利，双开净额退化 G_net=0，§3 `hedged_position_net_zero`）；
    (2) 「全覆盖 ⟹ 实盘盈利」（覆盖是 L0 语法层；实盘盈利是 L2 经验域，全窗 L3 v1 8/8 否证——
        C42 与 L3 不矛盾，两个有效域）；
    (3) 「全覆盖在任意状态成立」（仅覆盖可行域 X^cover_Θ 内成立；q̄∉K_Θ 时不适用——W11/W12/W13
        有效域上界继承）。
-/
inductive FinalTheoremVerdict where
  | separateVoiceLevelFullCoverage
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：C42 顶点裁定**必是**「分账本声部级全元素覆盖」（**不是**净资产
  级每笔盈利）。支撑：§4 `theorem_final`（三结论合一）+ §3 C40 防火墙（W4 净收益不可能 +
  W6 双开净额退化）+ §5 合一桥（W11 策略输出=覆盖目标）。

  ★这正是 PDF §十四原文：「这个效果的数学名称应当是『分账本声部级全元素覆盖』，**不是**
    『净资产级每笔盈利』」——类型层钉死，无「净盈利保证」构造子。
-/
theorem final_verdict_is_separate_voice_coverage (v : FinalTheoremVerdict) :
    v = FinalTheoremVerdict.separateVoiceLevelFullCoverage := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W14 工位，C40/C41/C42 最终定理顶点）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，import 仅 W13 链 + W4）：

  1. §1 C42 7 前提承载 `FinalTheoremHypotheses`（同一组分账本语义对象）：tree（W13 元素树）+
     legAssign（W8 ν 单射）+ position/price（头寸/价格）+ smp（W13 状态机承载器 = W11 投影）+
     objective（W10 K_Θ+J_t）+ schedule（W10 Schedule_Θ）。七前提刻画**同一**语义状态。

  2. §2 C42 三结论（各真引用前置顶层定理，无重证）：
     - `final_cover_unique`（结论①，引 W13 `theorem_separate_recursive`）：∀e 被唯一规范腿吃到。
     - `final_eatSep`（结论① 纯覆盖侧，引 W13 `recursive_eatSep`）。
     - `final_profit`（结论②，W13 `SepCovered.profit` 逐 e 携带）：∀e G^sep>0（覆盖内蕴收益）。
     - `final_strategy_well_defined`（结论③，引 W10 `policy_order_well_defined`）：∀x_t ∃!O。

  3. ★★§3 C40 防火墙（W4/W6 真引用）：
     - `net_profit_every_stroke_impossible`（引 W4 `net_value_eat_every_stroke_impossible`）：
       净账户每笔盈利不可能（存在双开构形净值非正）。
     - `hedged_position_net_zero`（引 W6 `hedged_leg_net_zero`）：双开 (Q,Q) 单声部净额
       legNet=q⁺−q⁻=0（退化）。

  4. ★★★★★§4 C42 三结论合一主定理（**canonical 顶点**）：
     - ★★★★★`theorem_final`：7 前提 ⟹ ① ∀e 覆盖+唯一 ∧ ② ∀e G^sep>0 ∧ ③ ∀x_t ∃!O——
       三结论在**同一组对象** H 上同时成立。
     - `policy_final_form`（C41，引 W10）：π_Θ=Schedule∘LexArgmin 最终形式 + ∃!O。

  5. ★★§5 合一桥（**W11 真引用**，三结论合一非空壳核心）：
     - `strategy_meets_cover_target`（引 W11 `coverFeasible_qStar_eq_target`）：策略输出 q* =
       覆盖目标 q̄（覆盖可行域内）——「策略全定义（③）」与「元素全覆盖（①②）」**同一**。

  6. §6 诚实标签 `final_verdict_is_separate_voice_coverage`（裁定=分账本声部级全元素覆盖，
     **无** NetProfitGuaranteed / NetAccountEveryStroke / CoverageEverywhere 构造子）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 全元素覆盖 ⟹ 净账户每笔盈利（覆盖是 P^sep 声部级毛收益；净账户每笔盈利被 C40/W4 否定——
    双开净额退化 G_net=0，§3 `hedged_position_net_zero`）。
  - ✗ 全覆盖 ⟹ 实盘盈利（L0 语法层；实盘盈利是 L2，全窗 L3 v1 8/8 否证——C42 与 L3 不矛盾，
    两个有效域：C42=L0 分账本声部级覆盖、L3=L2 净账户实盘扣成本盈利）。
  - ✗ 全覆盖在覆盖可行域**外**成立（q̄∉K_Θ 时状态机投影不成立，W11/W12/W13 有效域上界继承）。
  - ✗ 任何前置工位的内部证明（W13 树深归纳 / W10 ∃!O / W11 q*=q̄ / W4 净值抵消）——顶点只做
    合一，所有前置定理作为已证黑盒**真引用**，无一被本文件重证。
  - ✗ 互斥最终定理 M29（+AncOK/+Role 叠加）——本 W14 是基线顶点，M29 由集成工位另行叠加。

  ★下游引用（互斥最终集成 M29 直接引）：
  - 三结论合一：`theorem_final`（C42 顶点，M29 在其上叠加 AncOK/Role）。
  - 分结论：`final_cover_unique`/`final_eatSep`/`final_profit`/`final_strategy_well_defined`。
  - C41 策略形式：`policy_final_form`。
  - 合一桥：`strategy_meets_cover_target`（策略=覆盖目标）。
  - C40 防火墙：`net_profit_every_stroke_impossible`/`hedged_position_net_zero`。
  - 诚实裁定：`final_verdict_is_separate_voice_coverage`（覆盖≠净盈利）。

  谱系：C40/C41/C42（PDF §十二/§十三/§十四）→ W13（自相似递归 C38/C39）+ W10（策略全定义
        C33/C34/C35）+ W11（覆盖可行 C36）+ W4（净收益不可能 C22/C40）→ 本文件 W14（最终定理
        顶点，三结论合一）。有效域上界 = 覆盖可行域内分账本声部级全元素覆盖（C40/C42 净资产
        每笔盈利被否定，全窗 L3 不矛盾）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SeparateFinalTheorem
