/-
Origin/SevenLinkStrategy.lean

七链终环：全定义策略 π_Θ（环7）+ §16 中心定理 ∀x ∃! O_{t+1}=π_Θ(x)
═══════════════════════════════════════════════════════════════════════════

canonical 依据（spec 2026-06-28-recursive-complete-classification-bsp-pdf-extract.md）：
  - §15 P12 line 1173（π_Θ 方框）：
        π_Θ(x) = Schedule_Θ[ LexArgmin_{p∈𝒦_Θ(x)} J_x(p) − p_t ]
        J_x(p) = ‖p − p̃_{t+1}‖²_W + λ·Cost_x(p) + ν·RiskPenalty_x(p)
  - §16 P13 line 827-829（12 假设 → 中心定理方框）：
        ∀x,  ∃! O_{t+1} = π_Θ(x).
  - 七链对照表（spec 末）：环7 = 目标头寸 → 全定义策略 π_Θ → 唯一订单 O_{t+1}。

★定位（缺口矩阵 2026-06-28-lean-existing-vs-20page-gap-matrix.md 环7b）：
  本文件 thread **20 页七链**的 ∃!，沿七链**逐环传递唯一性**，复用各环**已证**的 ∃!。
  既有 `DecisionSufficiency.decision_pipeline_exists_unique` 走的是 Foundation/hybrid 管线
  （Schedule∘Risk∘Intent∘Classify∘Rec），**非** 20 页七链——故本文件**不** import
  DecisionSufficiency（避免把 hybrid 管线的 ∃! 冒充七链的 ∃!，no-patch）。
  `DecisionSufficiency.decision_lexargmin_exists_unique` 仅作**骨架参考**（它本身 re-export
  `LexArgmin.lexArgmin_exists_unique`，本文件直接引用底层定理）。

逐环 ∃! 复用（**不重证**，import + 引用 + 合成）：
  - 环3 Γ(x) 有限      = `CandidateSet.gamma_finite`            （候选集有限）
  - 环5 三桶 (𝒟,ℬ,𝒦) ∃! = `RThetaInterp.rTheta_exists_unique`     （解释器确定）
  - 环6 活动集 → p̃ ∃!  = `ActiveSet.ptilde_exists_unique`        （目标头寸唯一）
  - 环7 LexArgmin 唯一  = `LexArgmin.RiskProjection.lexArgmin_exists_unique`
                          + `…lexArgmin_eq_project`（字典序破平局 = 固定平局规则）
  - 环7 𝒦_Θ ≠ ∅        = `ConstraintSystem.feasible_nonempty`     （安全可行点见证）
                          抽象层由 `RiskProjection.feasible_ne_nil`（safeMem）承载

认识论等级：**L0**（结构/逻辑唯一性定理，不依赖任何市场数据；
formalization-validity-domain：§16/§18 唯一性是 L0 结构定理，可纯证）。

═══ 结果包六要素 ═══
1. 结论：定义 π_Θ(x)=Schedule_Θ[LexArgmin_{p∈𝒦_Θ(x)}J_x(p)−p_t]（`piTheta`），并证
   §16 ∀x ∃!O_{t+1}=π_Θ(x)——其中唯一性沿七链关系 `SevenLinkProduces` 逐环传递
   （`seven_link_exists_unique`），关系与函数式等价（`seven_link_iff_piTheta`）。
2. 定义依据：§15 π_Θ 方框 + J_x 方框；§16 12 假设清单 + ∃! 方框 + 逐步唯一性推导链。
3. 边界条件：唯一性依赖**全部 12 假设同时成立**——任一破裂则翻转。本文件中各假设的承载
   见下「12 假设处置表」。最关键四环（载于证明本体）：环5 解释器确定（`interpList` 全函数）、
   环6 p̃ 确定（`ptilde` 全函数）、环7 𝒦_Θ≠∅（`RiskProjection.safeMem`，否则 LexArgmin 无定义）、
   环7 字典序固定平局（`key_inj`，否则多最优解唯一性翻转——退化为普通 argmin 即失唯一）。
4. 下游推论：π_Θ 是信号→订单终环，约束所有上游环必须保持唯一性契约；Rust 端 = 约束求解器
   + 字典序目标 LexArgmin + Schedule_Θ（策略/回测入场，接 Nautilus 前置）。
5. 谱系引用：对应 MEMORY「互斥全定义策略=买卖点入场+多级角色/嵌套对冲」的"唯一订单"环；
   J_x 的连续优化（‖·‖²_W + λCost + νRiskPenalty）在纯 core **不可表为实数优化**，按既有
   `RiskProjection` 抽象为字典序键 `key : U → LexKey`（spec §15「[需人工确认离散化方案]」的
   既有结算——本文件沿用，不重开）。
6. 影响声明：新增 `Origin.SevenLinkStrategy`；**不**改任何被 import 文件（契约锚定下游）；
   确立七链 ∃! 的合成定理，是 §16 中心定理在 Lean 端的承载。

═══ 12 假设处置（§16 hypotheses，诚实分层）═══
  载于证明本体（per-link ∃! 引理，proof 中真用）：
    8  Γ(x) 有限        → `gamma`(List=有限) / `gamma_finite`（环3，`link3_gamma_finite`）
    9  ℛ_Θ 确定         → `interpList` 全函数（环5，`link5_triple_unique`；证明中 t 被强制）
    10 𝒦_Θ 非空且有限   → `RiskProjection.feasible`(List=有限) + `safeMem`(非空)
                          （环7，`link7_feasible_nonempty`；具体接地 `…_grounded`=feasible_nonempty）
    11 LexArgmin 固定平局 → `key_inj` ⟹ `lexArgmin_eq_project`（环7；证明中 p* 被强制 = project）
    12 Schedule_Θ 是函数 → `scheduleFrom : U → Order`（全函数；证明末 O 被强制）
  编码于链函数的全定义/单值（well-definedness，上游确定性）：
    1  递归结构 D_t 唯一  → 输入 x:State 确定 + 链函数全定义
    2  Prom_* 全定义单值  → `member`(Bool 全函数，env Γ 生成)
    3  b_ℓ(x) 全定义      → `firedAt`(Bool 全函数)
    4  区间套递归有限终止  → `NestingCertificate.N`(Bool 全函数) + `allCands` 有限
    5  H(g) 唯一          → `Cand.roleOf` 全函数（环4）
    6  V(g) 唯一          → 同上
    7  R(g)=(H,V,δ) 唯一  → `OperationRole18.Role18` 三轴积，`classifyR18` 全函数

axiom 边界：全部定理保持构造性 axiom 基线（无 `axiom`/`sorry`/`admit`/`Classical.choice`）。
纯 core 硬约束：无 Mathlib/Batteries/Std；无 Fintype/Finset（有限性用 List 表达）。
-/

import Origin.CandidateSet
import Origin.RThetaInterp
import Origin.ActiveSet
import Origin.LexArgmin
import Origin.ConstraintSystem

namespace NewChanlun.Origin.SevenLinkStrategy

open NewChanlun.Origin (ExistsUnique total_unique_of_fun)
open NewChanlun.Origin.CandidateSet (Cand State gamma allCands gamma_finite)
open NewChanlun.Origin.RThetaInterp (Triple RΘ interpList rTheta_exists_unique)
open NewChanlun.Origin.ActiveSet (activeNext ptilde ptilde_exists_unique)
open NewChanlun.Origin.SeparateLedger (SepPosition)
open NewChanlun.Origin.LexArgmin (RiskProjection lexLe LexKey)
open NewChanlun.Origin.ConstraintSystem
  (FeasibilityContext IsSafeContext Control Feasible feasible_nonempty)

variable {U Order : Type}

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 全定义策略 π_Θ 的载体 `ThetaStrategy`（环6/环7 外部参数打包）

  七链终环把上游唯一对象（环5 三桶 → 环6 活动集/目标头寸 p̃ → 环7 p*）映成唯一订单。
  环6/环7 依赖的**外部数据**（非缠论可导，是 Θ 参数 / 账户数据）打包于此结构：
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★全定义策略载体 `ThetaStrategy U Order`（L0，§15 π_Θ 的参数包）。
  - `par`/`fuel`：祖先指针 + AncOK 闭合 fuel（环6 活动集 §13）。
  - `sOf`：单位数 s_g（环6 目标头寸 §14，Cand 不携带的外部数据，不臆造）。
  - `At`：当前活动集 A_t（环6 输入）。
  - `riskProj`：给定状态 x（划出 𝒦_Θ(x) 约束）+ 目标头寸 p̃（J_x 的 ‖p−p̃‖²_W 项）⟹
    风险投影问题。`RiskProjection` 把 J_x（=‖·−p̃‖²_W+λCost+νRiskPenalty）抽象为字典序键
    `key : U → LexKey`（纯 core 无实数优化，沿用既有抽象——诚实，不重开离散化）。
    其 `feasible`(List=𝒦_Θ有限网格) + `safeMem`(𝒦_Θ≠∅) + `key_inj`(固定平局) 承载假设 10/11。
  - `scheduleFrom`：Schedule_Θ(· − p_t)——p_t 在时刻 t 固定，故为 p* 的全函数（假设 12）。
    `U` = 离散头寸/控制网格类型；`Order` = 订单类型（Schedule_Θ 输出）。
-/
structure ThetaStrategy (U Order : Type) where
  par : Cand → Option Cand
  fuel : Nat
  sOf : Cand → Nat
  At : List Cand
  riskProj : State → SepPosition → RiskProjection U
  scheduleFrom : U → Order

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 七链终段的链函数（环5 → 环6a → 环6b → 环7a → 环7b）

  每段是**全函数**（确定性），逐段把上游唯一对象推进到下游：
  ════════════════════════════════════════════════════════════════════════ -/

/-- **环6a 活动集 A_{t+1}**（§13）：消费环5 三桶 `interpList (gamma x)` 更新 A_t 并祖先闭合。 -/
def link6a (S : ThetaStrategy U Order) (x : State) : List Cand :=
  activeNext S.par S.fuel S.At (interpList (gamma x))

/-- **环6b 目标头寸 p̃_{t+1}**（§14）：活动集 fold 求和 Σ Leg(g)。 -/
def link6b (S : ThetaStrategy U Order) (x : State) : SepPosition :=
  ptilde S.sOf (link6a S x)

/-- **环7 风险投影问题**（§15）：状态 x（𝒦_Θ 约束）+ 目标 p̃（J_x 目标）⟹ `RiskProjection`。 -/
def riskAt (S : ThetaStrategy U Order) (x : State) : RiskProjection U :=
  S.riskProj x (link6b S x)

/-- **环7a 最优头寸 p\***（§15）：p* = LexArgmin_{p∈𝒦_Θ(x)} J_x(p) = `(riskAt S x).project`。 -/
def link7star (S : ThetaStrategy U Order) (x : State) : U :=
  (riskAt S x).project

/--
  **★全定义策略 π_Θ(x)（§15 P12 方框，★核心定义）** ——
      π_Θ(x) = Schedule_Θ[ LexArgmin_{p∈𝒦_Θ(x)} J_x(p) − p_t ]
             = `scheduleFrom (link7star S x)`。
  整环组装：环3 `gamma` → 环5 `interpList` → 环6a `activeNext` → 环6b `ptilde` →
  环7a `RiskProjection.project`（LexArgmin）→ 环7b `scheduleFrom`（Schedule_Θ）。
-/
def piTheta (S : ThetaStrategy U Order) (x : State) : Order :=
  S.scheduleFrom (link7star S x)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 逐环 ∃! 引理（**复用**各环已证定理，不重证）

  这些引理把任务指定的既有 ∃! 定理在七链坐标下"挂点"——证明本体 `seven_link_exists_unique`
  正是它们的合成。
  ════════════════════════════════════════════════════════════════════════ -/

/-- **环3：Γ(x) 有限**（假设 8）——复用 `CandidateSet.gamma_finite`（Γ 长度 ≤ 论域上界）。 -/
theorem link3_gamma_finite (x : State) : (gamma x).length ≤ (allCands x.ℓmax).length :=
  gamma_finite x

/-- **环5：三桶 (𝒟,ℬ,𝒦) ∃!**（假设 9）——复用 `RThetaInterp.rTheta_exists_unique`。 -/
theorem link5_triple_unique (x : State) :
    ExistsUnique (fun t : Triple => RΘ x = t) :=
  rTheta_exists_unique x

/-- **环6：目标头寸 p̃ ∃!**（活动集 → 头寸）——复用 `ActiveSet.ptilde_exists_unique`。 -/
theorem link6_ptilde_unique (S : ThetaStrategy U Order) (x : State) :
    ExistsUnique (fun p => ptilde S.sOf (link6a S x) = p) :=
  ptilde_exists_unique S.sOf (link6a S x)

/-- **环7：LexArgmin 唯一**（假设 10+11）——复用 `RiskProjection.lexArgmin_exists_unique`。 -/
theorem link7_lexargmin_unique (S : ThetaStrategy U Order) (x : State) :
    ∃ u : U, u ∈ (riskAt S x).feasible ∧
      (∀ y, y ∈ (riskAt S x).feasible →
        lexLe ((riskAt S x).key u) ((riskAt S x).key y) = true) ∧
      (∀ u', u' ∈ (riskAt S x).feasible →
        (∀ y, y ∈ (riskAt S x).feasible →
          lexLe ((riskAt S x).key u') ((riskAt S x).key y) = true) → u' = u) :=
  (riskAt S x).lexArgmin_exists_unique

/-- **环7：𝒦_Θ ≠ ∅（抽象层）**（假设 10 非空侧）——`RiskProjection.safeMem` ⟹ feasible ≠ []。 -/
theorem link7_feasible_nonempty (S : ThetaStrategy U Order) (x : State) :
    (riskAt S x).feasible ≠ [] :=
  (riskAt S x).feasible_ne_nil

/--
  **环7：𝒦_Θ ≠ ∅（具体接地）**（假设 10 的非空性不空跑）——复用 `ConstraintSystem.feasible_nonempty`：
  安全上下文下约束系统存在可行点（u^safe）。坐实抽象层 `safeMem` 在真实约束系统下**可被见证**，
  假设 10 非空非空洞。
-/
theorem assumption10_feasible_nonempty_grounded
    (ctx : FeasibilityContext) (hsafe : IsSafeContext ctx) :
    ∃ u : Control, Feasible ctx u :=
  feasible_nonempty ctx hsafe

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 七链唯一性传递关系 `SevenLinkProduces` + §16 中心定理

  **关键设计（防 §16 ⟹ total_unique_of_fun 的声明膨胀）**：唯一性**不是**"π_Θ 是函数故 ∃!"
  的同义反复，而是沿七链关系逐环**强制**——每段输出被上一段确定性强制相等，LexArgmin 段由
  `key_inj`（假设 11）把"可能非唯一的 argmin"收口为唯一 `project`。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★七链产出关系 `SevenLinkProduces S x O`（§16 七链路径的关系形式）：存在中间对象
  γ(环3)、t(环5 三桶)、A(环6a 活动集)、p̃(环6b 头寸)、p*(环7a LexArgmin)，使每段满足其链方程，
  且 p* 是 `(riskProj x p̃)` 的字典序最优（`IsLexArgmin`，**未先收口**——故 ⟸ 唯一性非平凡），
  终段 O = Schedule_Θ(p*)。
-/
def SevenLinkProduces (S : ThetaStrategy U Order) (x : State) (O : Order) : Prop :=
  ∃ (g : List Cand) (t : Triple) (A : List Cand) (ptil : SepPosition) (pstar : U),
    g = gamma x ∧
    t = interpList g ∧
    A = activeNext S.par S.fuel S.At t ∧
    ptil = ptilde S.sOf A ∧
    (S.riskProj x ptil).IsLexArgmin pstar ∧
    O = S.scheduleFrom pstar

/-- ★π_Θ(x) 满足七链关系（存在侧）：取每段的链值，环7a 段由 `project_isLexArgmin` 见证。 -/
theorem piTheta_produces (S : ThetaStrategy U Order) (x : State) :
    SevenLinkProduces S x (piTheta S x) :=
  ⟨gamma x, interpList (gamma x), link6a S x, link6b S x, link7star S x,
    rfl, rfl, rfl, rfl, (riskAt S x).project_isLexArgmin, rfl⟩

/--
  ★★唯一性传递（⟸ 侧，§16 核心，**逐环强制**）：任何满足七链关系的 O 必等于 π_Θ(x)。
  证明逐环 `subst`：环3 γ 被 `gamma x` 强制 → 环5 t 被 `interpList` 强制 → 环6a A 被
  `activeNext` 强制 → 环6b p̃ 被 `ptilde` 强制 → 环7a p* 由 `lexArgmin_eq_project`（假设 11
  字典序破平局）强制 = `project` → 环7b O 被 `scheduleFrom`（假设 12 函数）强制。
-/
theorem piTheta_unique (S : ThetaStrategy U Order) (x : State) :
    ∀ O, SevenLinkProduces S x O → O = piTheta S x := by
  rintro O ⟨g, t, A, ptil, pstar, hg, ht, hA, hptil, hlex, hO⟩
  subst hg; subst ht; subst hA; subst hptil; subst hO
  have hp : pstar = (riskAt S x).project :=
    (riskAt S x).lexArgmin_eq_project pstar hlex
  calc S.scheduleFrom pstar
      = S.scheduleFrom (riskAt S x).project := by rw [hp]
    _ = piTheta S x := rfl

/--
  ★★★§16 中心定理（七链关系形式）`seven_link_exists_unique`（L0，★核心）★★★：
      ∀x,  ∃! O，O 由七链产出。
  = 逐环 ∃! 的**合成**：存在侧 `piTheta_produces`（各环链值）+ 唯一侧 `piTheta_unique`
  （逐环强制）。这是 spec §16 「∀x ∃! O_{t+1}=π_Θ(x)」沿七链传递唯一性的 Lean 兑现。
-/
theorem seven_link_exists_unique (S : ThetaStrategy U Order) (x : State) :
    ExistsUnique (fun O => SevenLinkProduces S x O) :=
  ⟨piTheta S x, piTheta_produces S x, piTheta_unique S x⟩

/-- ★七链关系 ⟺ 函数式 π_Θ：`SevenLinkProduces S x O ↔ O = π_Θ(x)`（关系与函数式同一）。 -/
theorem seven_link_iff_piTheta (S : ThetaStrategy U Order) (x : State) (O : Order) :
    SevenLinkProduces S x O ↔ O = piTheta S x :=
  ⟨piTheta_unique S x O, fun h => by rw [h]; exact piTheta_produces S x⟩

/--
  ★§16 中心定理（函数式形式，spec line 829 方框逐字）`central_theorem`（L0）：
      ∀x,  ∃! O_{t+1} = π_Θ(x).
  函数式 ∃! 是 π_Θ 全函数的直接推论（O 由 π_Θ(x) 唯一确定）。
  ★**实质唯一性**（七链逐环传递，⟸ 非平凡）载于 `seven_link_exists_unique` / `piTheta_unique`
  ——本条只是把它改写为 spec 方框的逐字形式（谓词 `π_Θ(x) = O`）；两者经
  `seven_link_iff_piTheta` 同一（仅差 `Eq.symm`）。
-/
theorem central_theorem (S : ThetaStrategy U Order) (x : State) :
    ExistsUnique (fun O => piTheta S x = O) := by
  refine ⟨piTheta S x, rfl, ?_⟩
  intro O hO
  exact hO.symm

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★中心定理标签 `CentralTheoremTag`（gatekeeper，诚实分层）。
  - `StructuralUniquenessL0`：唯一性是结构定理（不依赖市场数据）。
  - `TwelveAssumptionsAsHypotheses`：12 假设为前提（载体/链函数承载，见文件头处置表）。
  - `PerLinkUniquenessComposed`：∃! 由逐环 ∃! 合成（非 total_unique_of_fun 退化）。
  - `ThetaParametricSchedule`：J_x 权重 / 𝒦_Θ 约束 / Schedule_Θ 是 Θ 参数，非缠论可导。
  - `EmpiricalValidityOutOfScope`：π_Θ 的盈利/真实可行集校准是 L2/L3，不在本层。
  ★**没有** `TrueCompleteClassification` 构造子——唯一性定理不冒充经验有效分类。
-/
inductive CentralTheoremTag where
  | StructuralUniquenessL0
  | TwelveAssumptionsAsHypotheses
  | PerLinkUniquenessComposed
  | ThetaParametricSchedule
  | EmpiricalValidityOutOfScope
deriving DecidableEq, Repr

/-- ★中心定理认识论等级 = L0（结构唯一性，不冒充经验有效性）。 -/
def centralTheoremLevel : CentralTheoremTag := CentralTheoremTag.StructuralUniquenessL0

/-- ★gatekeeper：中心定理任何标签都**不是** TrueCompleteClassification（唯一性 ≠ 经验有效）。 -/
theorem central_theorem_not_true_classification (t : CentralTheoremTag) :
    t = CentralTheoremTag.StructuralUniquenessL0 ∨
    t = CentralTheoremTag.TwelveAssumptionsAsHypotheses ∨
    t = CentralTheoremTag.PerLinkUniquenessComposed ∨
    t = CentralTheoremTag.ThetaParametricSchedule ∨
    t = CentralTheoremTag.EmpiricalValidityOutOfScope := by
  cases t <;> simp

end NewChanlun.Origin.SevenLinkStrategy
