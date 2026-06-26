/-
  Origin/StrategyFamily.lean — π_Θ 策略族（port 到 Origin canonical base）
  ★task #114, A′ 策略组件港入 Origin canonical base（审计 B：π_Θ 策略族 MISSING in Origin）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**。审计 B 判决：策略族 π_Θ **MISSING in
  Origin，只在 legacy Strict/StrategyFamily.lean**。Origin 的 `FullDefinitionStrategy.lean` 有
  `FullDefinitionSystem` + `policyTheta`（**单个**全定义策略闭环），但缺「分类 ⊬ 唯一策略 ⟹
  一族 {π_Θ}」的族结构与两元定理。本文件把它**重锚到 Origin 类型**：复用 Origin `SourceAxioms`
  的 `Causal`/`Side`、Origin `FullDefinitionStrategy` 的 `FullDefinitionSystem`/`policyTheta`，
  内部 import Origin.RiskProj/Origin.ClassifierFamily（#114 同批港入件），**不** import legacy。

  ── 核心理论结论（编排者，继承 legacy）──────────────────────────────────────
  **缠论完全分类本身不能推出唯一策略，只能推出一族策略 {π_Θ}。**
  分类只回答「当前属于什么结构」（结构状态），**不能**唯一决定仓位/杠杆/成本阈值/止损/
  冲突优先级——这些是 Θ（风险公理）参数，**不在 S 中**。给定 Θ 后，π_Θ 才**全定义**且对每
  状态产出唯一动作。

  ── 两个元定理（本文件核心交付，挂 Origin 命名空间）──────────────────────────
  1. `classification_does_not_choose_unique_policy`：分类不推出唯一策略
     （S 非空 + 动作 ≥2 元 ⟹ 策略空间 S → A 至少两个不同元素）。
  2. `given_theta_total_unique`：给定 Θ ⟹ π_Θ 全定义 + 唯一（+ 因果由 `piTheta_causal`，
     用 Origin canonical `Causal` 谓词陈述无前视）。

  ── 与 Origin FullDefinitionSystem 的对接（族成员 = Origin 单步闭环）──────────
  Origin `FullDefinitionStrategy.lean` 的 `policyTheta S x e` 是**单个** Θ 固定后的全定义订单
  产出。本文件证「Origin `FullDefinitionSystem` 的族（按 Param 索引）每成员全定义/唯一」
  （`originPolicyFamily`），把 Origin 既有单步闭环**正式纳入策略族结构**——族 = Param 索引的
  全体 Origin FullDefinitionSystem 策略。这坐实「完全分类 ⊢ 一族 {π_Θ}」在 Origin 上成立。

  ── 诚实标注（formalization-validity-domain + gatekeeper）──────────────────────
  ★标签：OperationalSemanticsOnly + RuntimeGuard + 盈利/最优 EmpiricalDomain（L3）+
    subkind StrategyFamilyGivenTheta。**禁标 TrueCompleteClassification**——π_Θ 不是分类定理。
  本文件**证**（L0）：给定 Θ ⟹ π_Θ 全定义/唯一/因果 + 分类 ⊬ 唯一策略 + 因子化≠产生。
  本文件**不证**：Θ 来自缠论（Θ 是 extra-缠论设计选择）；收益最优/盈利（L3 EmpiricalDomain）。

  ── 依赖方向（单向无环，不 import #113、不 import legacy Strict）──────────────
  StrategyFamily → {Origin.FullDefinitionStrategy, Origin.RiskProj, Origin.ClassifierFamily}。
  （RiskProj/ClassifierFamily 是 #114 同批 own 件，内部 import 合法。）
  验证：`cd formal && lake env lean Origin/StrategyFamily.lean`。禁 sorry/admit/axiom。

  谱系：legacy Strict/StrategyFamily.lean（cc-strategyfamily #69）→ #96/#97（A′ canonical）→
        本文件 #114（π_Θ 策略族港入 Origin）。
-/

import Origin.FullDefinitionStrategy
import Origin.RiskProj
import Origin.ClassifierFamily

namespace NewChanlun.Origin.StrategyFamily

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 元定理 1：分类不推出唯一策略（核心理论结论的 L0 兑现）

  分类 I : H → S 给结构标签，但「策略」是 S → A。只要 S 非空且动作集 A 至少两个不同元素
  a ≠ b，则策略空间 S → A 至少两个不同元素——分类本身**不在它们之间做选择**。选择需 Θ。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★标签策略 `LabelPolicy S A`：从结构标签 S 到动作 A 的全函数。分类只诱导可选策略空间。 -/
def LabelPolicy (S A : Type) : Type := S → A

/-- ★分类诱导的历史动作 `inducedAction`：给定 label policy π，历史 h 的动作 = π (I h)。
    关键：π 是**额外输入**，I 本身不提供 π。 -/
def inducedAction {H S A : Type} (I : H → S) (π : S → A) : H → A :=
  fun h => π (I h)

/--
  ★★★元定理 1（核心理论结论的 L0 兑现）★★★：
  `classification_does_not_choose_unique_policy` — 分类不推出唯一策略。

  若结构标签 S 非空（Inhabited S）且动作集存在两个不同元素 a ≠ b，则策略空间 S → A **至少
  两个不同元素** π₁ ≠ π₂。含义：分类只给标签，即使 S 完全确定，策略仍有 ≥2 个全定义选项。

  ★诚实标注证明强度（避免声明膨胀）：本定理证「完全分类 ⊬ 唯一策略」的**必要条件见证**——
  策略空间 S → A 非单点。它**不引用**分类器 I 的内部结构，是「策略空间本身非单点，分类无论
  怎样都无法在 Lean 类型层把它收缩为唯一选择」的**模型论见证**。「⊢ 一族 {π_Θ}」的正面内容
  由元定理 2 + `StrategyFamily` 承载。
-/
theorem classification_does_not_choose_unique_policy
    {S A : Type} [Inhabited S] {a b : A} (hne : a ≠ b) :
    ∃ π₁ π₂ : S → A, π₁ ≠ π₂ := by
  refine ⟨fun _ => a, fun _ => b, ?_⟩
  intro h
  have hp := congrFun h default
  exact hne hp

/--
  ★元定理 1 的历史动作版（L0）：分类 + 两个不同标签策略 ⟹ 诱导的历史动作族不同。
  即便分类 I 固定，诱导的历史动作仍随策略选择而变（分类不锁定历史动作）。需 H 非空。
-/
theorem classification_induces_policy_family
    {H S A : Type} [Inhabited H] {a b : A} (hne : a ≠ b)
    (I : H → S) :
    ∃ π₁ π₂ : S → A, inducedAction I π₁ ≠ inducedAction I π₂ := by
  refine ⟨fun _ => a, fun _ => b, ?_⟩
  intro h
  have hp := congrFun h default
  unfold inducedAction at hp
  exact hne hp

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 因子化 vs 产生（关键区分）

  分类**能**把已给定的 fiber-constant 历史策略**唯一因子化**为标签策略；但分类**不能产生**该
  策略（f 的选择需 extra-缠论输入）。这精确分离两件事。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★fiber 常值 `FiberConstant I f`：历史策略 f 在 I 的每个 fiber 上常值（同标签历史同动作）。 -/
def FiberConstant {H S A : Type} (I : H → S) (f : H → A) : Prop :=
  ∀ h h', I h = I h' → f h = f h'

/-- ★因子化 `Factors I f π`：标签策略 π 是历史策略 f 的因子（f = π ∘ I）。 -/
def Factors {H S A : Type} (I : H → S) (f : H → A) (π : S → A) : Prop :=
  ∀ h, f h = π (I h)

/--
  ★因子存在（L0）：若 I 满射（每标签 realized）且 f fiber-constant，则存在因子 π。
  对每标签 s 取见证历史 hs（I hs = s），令 π s = f hs。fiber-constant 保证良定义。
  用 `Classical.choose`（Lean core 自带）提取见证历史。
-/
theorem factor_exists {H S A : Type} (I : H → S) (f : H → A)
    (hfc : FiberConstant I f)
    (hrealized : ∀ s, ∃ h, I h = s) :
    ∃ π : S → A, Factors I f π := by
  refine ⟨fun s => f (Classical.choose (hrealized s)), ?_⟩
  intro h
  have hwit : I (Classical.choose (hrealized (I h))) = I h :=
    Classical.choose_spec (hrealized (I h))
  exact (hfc h (Classical.choose (hrealized (I h))) hwit.symm)

/--
  ★★因子唯一（L0，关键定理）：若 I 满射（每标签 realized），则 fiber-constant 历史策略 f 的
  因子 π **唯一**——任意两个因子处处相等。这是「分类能**唯一因子化**已给 fiber-constant 策略」。

  ★但这**不**意味分类产生了 f——f 是**输入**（已给定的 fiber-constant 历史策略），因子化只是
  把已有 f 重新表达为标签层。f 的选择本身需 extra-缠论输入。**因子化 ≠ 产生**。
-/
theorem factor_unique {H S A : Type} (I : H → S) (f : H → A)
    (hrealized : ∀ s, ∃ h, I h = s)
    (π₁ π₂ : S → A) (h1 : Factors I f π₁) (h2 : Factors I f π₂) :
    ∀ s, π₁ s = π₂ s := by
  intro s
  obtain ⟨h, hIh⟩ := hrealized s
  rw [← hIh]
  rw [← h1 h, ← h2 h]

/--
  ★★分类不产生策略（L0，因子化 ≠ 产生）：分类 I **本身不在** fiber-constant 历史策略之间
  做选择——只要动作集 ≥2 元，存在两个**不同**的 fiber-constant 历史策略 f₁ ≠ f₂，它们都在 I 上
  fiber-constant，但 I 不偏好任一个。

  与 `factor_unique` 对照：给定 f，因子 π 唯一（**因子化**确定）；但**给定 I，f 不唯一**
  （**产生**不确定）。分类提供因子化机制，不提供策略选择——选择需 Θ。
-/
theorem classification_does_not_produce_strategy
    {H S A : Type} [Inhabited H] {a b : A} (hne : a ≠ b) (I : H → S) :
    ∃ f₁ f₂ : H → A,
      FiberConstant I f₁ ∧ FiberConstant I f₂ ∧ f₁ ≠ f₂ := by
  refine ⟨fun _ => a, fun _ => b, ?_, ?_, ?_⟩
  · intro h h' _; rfl
  · intro h h' _; rfl
  · intro hcontra
    have := congrFun hcontra default
    exact hne this

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 Θ（风险公理）+ piTheta + 元定理 2（给定 Θ ⟹ 全定义 / 唯一 / 因果）

  Θ 把分类标签外的内容（recog / target / riskProj / exec）参数化。给定 Θ 后 π_Θ 全定义 +
  唯一 + 因果。因果用 Origin canonical `Causal`（`SourceAxioms.Causal`，抽象 prefixEq）陈述——
  这比 legacy 固定 PrefixEq 更忠实 Origin（Origin 参数化前缀关系，不固定）。

  ★风险投影用 Origin.RiskProj 的确定选择器（#114 同批 port）：`RiskGrid.gridProject` 在有限
  网格上**真兑现**唯一总仓位，非抽象留空。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Θ 风险公理结构 `Theta`（L0，port 到 Origin）。

  Θ 把「分类标签 S 外」的执行内容参数化为四个全函数 + 一项因果证明义务：
  - `recog : H → Z → D`：识别/决策核（从历史 h 与账户 Z 读出决策意图 D）。
  - `target : D → Z → Target`：决策 + 账户 → 目标仓位/风险目标。
  - `riskProj : Origin.RiskProj.RiskGrid Pos K`：风险投影网格（#114 Origin.RiskProj 确定选择器）。
  - `proj : Target → Pos`：把 Target 投到网格可行 Pos（与 riskProj.gridProject 衔接的目标映射）。
  - `exec : D → Z → Pos → Order`：执行映射（决策 + 账户 + 投影仓位 → 订单）。
  - `prefixEq : H → H → Prop`：历史前缀等价（Origin `Causal` 的前缀关系，参数化非固定）。
  - `recog_causal`：recog 因果无前视——同前缀 + 同账户 ⟹ 同决策（Origin `Causal` 形式）。

  ★字段名 `recog`（不用 `rec`）：`rec` 是 Lean structure 自动递归子保留名。
  ★诚实标注：recog/target/exec/proj 的**具体内容**（风险阈值/成本模型/止损/调度）全部是
  **Θ-参数化，非 L0 可导**——不由分类推出，是 extra-缠论设计选择。本结构只证「给定这些
  Θ-组件 ⟹ 组合出的 π_Θ 全定义/唯一/因果」。
-/
structure Theta (H Z D Target Pos K Order : Type) where
  prefixEq : H → H → Prop
  recog : H → Z → D
  target : D → Z → Target
  riskProj : Origin.RiskProj.RiskGrid Pos K
  proj : Target → Pos
  exec : D → Z → Pos → Order
  /-- recog 因果无前视：同前缀 + 同账户 ⟹ 同决策（不偷看未来，Origin `Causal` 形式）。 -/
  recog_causal : ∀ (h h' : H) (z : Z),
    prefixEq h h' → recog h z = recog h' z

/--
  ★π_Θ 组合 `piTheta`（L0，port 到 Origin）：给定 Θ，从历史 h 与账户 z 产出唯一订单。
  组合链：recog h z → target d z → proj y → exec d z p。这是「给定 Θ 的完整操作语义」——把
  分类标签外的决策/目标/风险投影/执行串成**全定义**的订单产出函数。
-/
def piTheta {H Z D Target Pos K Order : Type}
    (θ : Theta H Z D Target Pos K Order) (h : H) (z : Z) : Order :=
  let d := θ.recog h z
  let y := θ.target d z
  let p := θ.proj y
  θ.exec d z p

/--
  ★★★元定理 2（给定 Θ ⟹ 全定义 + 唯一）★★★：
  `given_theta_total_unique` — 给定 Θ、历史 h、账户 z，**存在唯一**订单 o 使 piTheta θ h z = o。
  用 Origin canonical `ExistsUnique` 陈述。与元定理 1 对照：**分类不选，但给定 Θ 后唯一钉死**。

  ★诚实标注证明强度（避免声明膨胀）：函数图 ∃!——piTheta 是 Lean 全函数，故全定义（= 存在）与
  唯一（= 函数确定性）自动成立。内容**不是**「证明 Θ 使 π 变唯一」，而是「一旦把 Θ 组件定为全
  函数 + 确定选择器，组合 piTheta 即是全函数」的接口兑现。**实质非唯一性**在元定理 1；**实质
  因果约束**在 `piTheta_causal`（依赖 `recog_causal` 非平凡前提）。
-/
theorem given_theta_total_unique {H Z D Target Pos K Order : Type}
    (θ : Theta H Z D Target Pos K Order) (h : H) (z : Z) :
    ExistsUnique (fun o => piTheta θ h z = o) :=
  ⟨piTheta θ h z, rfl, fun _ heq => heq.symm⟩

/--
  ★★π_Θ 因果无前视（元定理 2 的因果侧，L0，Origin `Causal` 形式）：
  固定账户 z，π_Θ 的当下决策只依赖已出现数据（前缀），**不偷看未来**——
  `prefixEq h h' → piTheta θ h z = piTheta θ h' z`。这是「当下判断不能用未来数据」在策略层兑现：
  因果性沿 recog 传导到整条组合链（target/proj/exec 只读 recog 输出 d、账户 z、投影 p，不另引历史）。

  ★诚实标注因果负担承载：实质因果约束**全压在** `recog_causal`（Θ 证明义务，**非平凡前提**）。
  target/proj/exec 的因果性是**类型接口切断**的结果（签名 D→Z→… 不接受历史 H，无法偷读未来），
  **不是**对它们各自时间因果的独立证明。**未覆盖**：账户 z 的**生成**因果性（本定理比较同一个 z
  下两历史，不证 z 本身只依赖前缀——属账户层 Tlayers.Accounting 承载，本文件有效域外）。
-/
theorem piTheta_causal {H Z D Target Pos K Order : Type}
    (θ : Theta H Z D Target Pos K Order) (h h' : H) (z : Z)
    (hpre : θ.prefixEq h h') :
    piTheta θ h z = piTheta θ h' z := by
  unfold piTheta
  rw [θ.recog_causal h h' z hpre]

/--
  ★π_Θ 因果性对接 Origin canonical `Causal`（L0）：固定账户 z，`fun h => piTheta θ h z` 满足
  Origin `SourceAxioms.Causal θ.prefixEq`。把策略层因果**正式对接** Origin canonical Causal 谓词
  （非另起炉灶）——下游可经此与 Origin 其他 Causal 结论统一使用。
-/
theorem piTheta_isCausal {H Z D Target Pos K Order : Type}
    (θ : Theta H Z D Target Pos K Order) (z : Z) :
    @Causal H PUnit Order θ.prefixEq (fun h => piTheta θ h z) :=
  fun h h' hpre => piTheta_causal θ h h' z hpre

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 策略族结构 StrategyFamily + 从 Θ-族构造 + 对接 Origin FullDefinitionSystem
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★策略族结构 `StrategyFamily`（L0，port 到 Origin）：参数 Param 索引的策略族 {π_Θ}。
  - `π : Param → H → Z → Order`：参数化策略族——每个 Θ-参数给一个全定义策略。
  - `total_unique`：∀ θ h z, π θ 全定义且唯一（Origin `ExistsUnique`）。
  - `causal`：∀ θ，同前缀 ⟹ 同订单（Origin `Causal` 形式，每参数下因果无前视）。

  这是「完全分类 ⊢ 一族策略 {π_Θ}」的结构容器——分类不选 Θ（元定理 1），但每个 Θ ∈ Param
  都给一个全定义/唯一/因果策略（元定理 2）。族 = Param 索引的全体 π_Θ。
-/
structure StrategyFamily (H Z Param Order : Type) where
  prefixEq : H → H → Prop
  π : Param → H → Z → Order
  total_unique : ∀ θ h z, ExistsUnique (fun o => π θ h z = o)
  causal : ∀ θ z, @Causal H PUnit Order prefixEq (fun h => π θ h z)

/--
  ★从 Θ-族构造 StrategyFamily（L0）：给定一族 Θ（按 Param 索引），组装为 StrategyFamily，
  total_unique 与 causal 由元定理 2 + piTheta_isCausal 填充。要求所有 Θ 共享同一 prefixEq
  （因果性是历史结构属性，不随 Θ 变）。
-/
def familyOfTheta {H Z D Target Pos K Order : Type} {Param : Type}
    (prefixEq : H → H → Prop)
    (Θ : Param → Theta H Z D Target Pos K Order)
    (hpre : ∀ p, (Θ p).prefixEq = prefixEq) :
    StrategyFamily H Z Param Order where
  prefixEq := prefixEq
  π := fun p h z => piTheta (Θ p) h z
  total_unique := fun p h z => given_theta_total_unique (Θ p) h z
  causal := fun p z h h' hpre' => by
    have hcausal := piTheta_isCausal (Θ p) z h h'
    rw [hpre p] at hcausal
    exact hcausal hpre'

/--
  ★★对接 Origin `FullDefinitionSystem`（L0，A′ port 的核心对接）：
  `originPolicyFamily` — Origin `FullDefinitionStrategy.lean` 的 `policyTheta`（单步全定义闭环订单
  产出）的**族版**：按 Param 索引一族 Origin `FullDefinitionSystem`，每成员 policyTheta 全定义/唯一。

  这把 Origin 既有的**单个** policyTheta 闭环**正式纳入策略族结构**——族 = Param 索引的全体
  Origin FullDefinitionSystem 策略。坐实「完全分类 ⊢ 一族 {π_Θ}」在 Origin canonical 上成立
  （不另起炉灶，复用 Origin FullDefinitionSystem 的 policyTheta 作族成员）。

  ★每成员全定义/唯一：对固定 Param p、状态 x、事件 e，`(S p).policyTheta x e` 存在唯一
  （Origin policyTheta 是全函数，函数图平凡侧）。这是「Origin 单步闭环成族后仍全定义/唯一」。
-/
theorem originPolicyFamily {Param : Type}
    (S : Param → FullDefinitionSystem)
    (p : Param) (x : StrictState) (e : (S p).Event) :
    ExistsUnique (fun o => policyTheta (S p) x e = o) :=
  ⟨policyTheta (S p) x e, rfl, fun _ heq => heq.symm⟩

/--
  ★Origin 单步闭环成族 ⟹ 分类不选成员（L0，元定理 1 在 Origin FullDefinitionSystem 上的兑现）：
  给定一族 Origin FullDefinitionSystem，若某状态 x 在两不同事件下 policyTheta 产出可不同（即族
  内策略空间非单点的见证由 Param ≥2 给出），分类层（仅给 StrictState 标签）**不在族成员间选择**。

  ★诚实标注证明强度（避免声明膨胀）：见证内容**纯是 Param 非单点**（存在两不同参数）——
  族 `_S` 提供「Origin FullDefinitionSystem 族」的语境但**不参与证明体**（不同 Param 的 `(_S p).Event`
  类型相异，无法在同一 (x,e) 上比较两成员订单，故不在此层比较具体订单）。这是元定理 1（分类 ⊬
  唯一策略）在 Origin 族上的**索引层**实例化：分类（StrictState 标签）不选 Param（= Θ 选择），
  Param 非单点即族非单点。**实质的「给定 Θ 后唯一」**在 `originPolicyFamily`（每成员全定义/唯一）。
-/
theorem origin_classification_does_not_choose_member {Param : Type}
    (_S : Param → FullDefinitionSystem)
    {p q : Param} (hpq : p ≠ q) :
    ∃ p' q' : Param, p' ≠ q' :=
  ⟨p, q, hpq⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 多空双开（不加 Q⁺Q⁻=0 禁令）

  本文件不加「净仓为零」或「多空互斥」禁令——多空双开结构上允许（赋格声部树见
  Origin/VoiceTree.lean）。是否实际可行需额外容量/上限假设（EmpiricalDomain），本文件不声称。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★多空双开方向可表达（L0，对接 Origin Side）：Origin canonical `Side` 的 long 与 short
  是**不同**方向——多空双开在方向层可区分表达（不被类型坍缩为单态）。
  ★诚实标注：只证方向可区分（结构允许）。实际双开是否满足保证金/容量约束是 Θ-参数化风险
  约束（Origin/RiskProj.lean 的可行网格），属 EmpiricalDomain，不由此 L0 声称。
-/
theorem long_short_distinct : (Side.long : Side) ≠ Side.short := by
  intro h; cases h

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 标签声明（gatekeeper：禁标 TrueCompleteClassification，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★π_Θ 标签种类 `PolicyTag`（gatekeeper，诚实分层）。
  OperationalSemanticsOnly（操作语义，非分类定理）+ RuntimeGuard（运行时风险守卫）+
  EmpiricalDomain（盈利/最优 L3）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把 π_Θ 标为真完全分类。
-/
inductive PolicyTag where
  | OperationalSemanticsOnly
  | RuntimeGuard
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★π_Θ 子类（gatekeeper）：StrategyFamilyGivenTheta（唯一子类——给定 Θ 的策略族）。 -/
inductive PolicySubkind where
  | StrategyFamilyGivenTheta
deriving DecidableEq, Repr

/-- ★π_Θ 诚实标签包（L0 声明）。 -/
def piThetaLabels : List PolicyTag × PolicySubkind :=
  ([PolicyTag.OperationalSemanticsOnly, PolicyTag.RuntimeGuard, PolicyTag.EmpiricalDomain],
   PolicySubkind.StrategyFamilyGivenTheta)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：π_Θ 子类必是 StrategyFamilyGivenTheta。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝冒充真完全分类（非跨库强保证）。
    诚实内容：π_Θ 自我声明为分类的下游操作语义，而非分类本身。 -/
theorem piTheta_not_true_classification (k : PolicySubkind) :
    k = PolicySubkind.StrategyFamilyGivenTheta := by
  cases k; rfl

end NewChanlun.Origin.StrategyFamily
