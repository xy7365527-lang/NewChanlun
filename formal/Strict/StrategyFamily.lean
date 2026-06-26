/-
  Strict/StrategyFamily.lean — π_Θ 策略族（标准第6部分完全应对的严格深化，L0）
  ★cc-strategyfamily 工位（task #69；598/603/615 谱系 + 编排者新理论结论）

  topo_address: swarm/严格完全分类/strategyfamily ｜ parent: Lead

  ── 核心理论结论（编排者） ──
  **缠论完全分类本身不能推出唯一策略，只能推出一族策略 {π_Θ}。**

  分类 I : H → S 只回答「当前属于什么结构」（结构状态），**不能**唯一决定仓位 / 杠杆 /
  成本阈值 / 止损 / 冲突优先级——这些是 Θ（风险公理）参数，**不在 S 中**。
  给定 Θ 后，π_Θ 才**全定义**且对每个状态产出唯一动作。

  本文件是对 `Strict/Op.lean`（standard §6 `Strategy : S → StrictAction` 全定义骨架）的
  **真扩展**：Op.lean 把动作语义钉到 7 标签层（数量/价格/账户转移标在标签外，
  `op_quantity_out_of_layer2`）；本文件把「标签外内容」升级为「给定 Θ 后的完整订单语义」，
  并给出**形式化见证/分离**：分类不唯一决定策略选择（策略空间非单点，Θ 作为额外输入），
  故分类 ⊬ 唯一策略。★诚实限定：本文件**不**证「无任何从分类到 Θ 的产生机制」的全局
  不可导定理——它证的是「策略空间本身非单点 + 分类不在其间选 + 因子化≠产生」的形式化见证
  （Θ 是 extra-缠论设计选择，不由分类内部结构唯一钉死）。

  ── 两个元定理（本文件的核心交付） ──
  1. `classification_does_not_choose_unique_policy`：分类不推出唯一策略
     （S 非空 + 动作 ≥ 2 元 ⟹ 策略空间 S → A 至少有两个不同元素）。
     ★诚实强度（codex 自审）：这是核心理论结论的**必要条件见证**（策略空间非单点），
     **不引用**分类器内部结构——「分类不在策略间选」由「策略空间本身非单点 + 类型层无法
     收缩」承载，非「用 Classifies 结构推出非唯一」的内生定理。「⊢ 一族 {π_Θ}」的正面内容
     由元定理 2 + `StrategyFamily` 承载。
  2. `given_theta_total_unique`：给定 Θ ⟹ π_Θ 全定义 + 唯一（+ 因果由 `piTheta_causal`）。
     ★诚实强度：全定义/唯一是「函数图 ∃!」（piTheta 是 Lean 全函数，自动成立，最弱必要侧）；
     **实质的因果约束**在 `piTheta_causal`（非平凡，依赖 `recog_causal` 前提，因果负担全压在
     recog 上，未覆盖账户 z 的生成因果）。Θ 固定后对每历史 × 状态产出唯一订单。

  ── 关键区分（codex Q1）：分类能因子化已给策略，不能产生策略 ──
  `FiberConstant I f`（f 在 I 的每个 fiber 上常值）+ `Factors I f π`（f = π ∘ I）：
  若 I 对每个 s realized，则已给定 fiber-constant 的 f 的因子 π **唯一**——分类能把
  已给的 fiber-constant 历史策略**唯一因子化**为标签策略；但 f 本身**不由 I 产生**
  （I 只给标签，f 的选择需 extra-缠论输入）。这精确分离「因子化」与「产生」。

  ── 诚实标注（formalization-validity-domain + 615 gatekeeper） ──
  ★标签：**OperationalSemanticsOnly**（操作语义，非真完全分类）
        + **RuntimeGuard**（运行时守卫）
        + 盈利/最优性 = **EmpiricalDomain**（L3 经验有效域）
        + subkind = **StrategyFamilyGivenTheta**（给定 Θ 的确定执行策略族）。
  ★**禁标 TrueCompleteClassification**——π_Θ 不是分类定理，是给定 Θ 的全定义操作语义。
    （`piTheta_not_true_classification` 在**本文件标签类型** `PolicySubkind` 上证此——本文件
    标签 API 不提供冒充途径；这不是跨库全局禁标的强保证，诚实限定见该定理注释。）

  本文件**证**（L0，machine-checked）：给定 Θ ⟹ π_Θ 全定义 / 唯一 / 因果。
  本文件**不证**：
  - ✗ Θ 来自缠论——Θ（ρ/β/γ/w/κ/成本/止损/优先级/调度）是 extra-缠论设计选择，
       全部标 **Θ-参数化，非 L0 可导**（不由分类 I 推出）。
  - ✗ 收益最优 / 盈利——需另加收益分布 / 成本 / 效用（L3 EmpiricalDomain，不由此 L0 声称）。

  ── 风险投影的诚实裁定（codex Q2 关键修正） ──
  `0 ∈ 𝓚`（零仓可行）只保证可行集**非空**，**不**保证 argmin（最优投影）存在。
  故本文件把 `RiskProjector` 抽象为**带证明的确定选择器**（project + feasible + unique），
  **不先碰凸分析**（紧性/下半连续/强制性等存在性条件留后续实例化为格点+字典序版）。
  这避免「冒充 argmin 存在性」（声明膨胀，090号）。

  ── 多空双开（codex Q3） ──
  本文件**不加** `Q⁺ Q⁻ = 0` 禁令——结构上**不禁止**多空双开。
  （若要证「确实存在双开可行点」需额外容量/上限假设，本文件不声称。）

  范式：纯 Prop/Type，不依赖 Mathlib（继承 Strict 库自包含约束，`∃!` 显式展开）。
  禁 sorry/admit/axiom。验证：`cd formal && lake env lean Strict/StrategyFamily.lean`。
-/

import Strict.Classification
import Strict.Causal
import Strict.OpenTail

namespace Strict.StrategyFamily

open Strict (PrefixEq)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 元定理 1：分类不推出唯一策略（核心理论结论的 L0 兑现）

  分类 I : H → S 给出结构标签，但「策略」是 S → A（标签到动作）。只要 S 非空且
  动作集 A 至少有两个不同元素 a ≠ b，则策略空间 S → A 至少有两个不同元素
  （常 a 策略 ≠ 常 b 策略）——分类本身**不在它们之间做选择**。选择需 extra-缠论的 Θ。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★标签策略 `LabelPolicy S A`（codex Q1）：从结构标签 `S` 到动作 `A` 的全函数。
  分类 I 只诱导**可选策略空间** `S → A`，不是唯一策略。
-/
def LabelPolicy (S A : Type) : Type := S → A

/--
  ★分类诱导的历史动作 `inducedAction`（codex Q1）：给定 label policy π 后，
  历史 h 的动作 = `π (I h)`——分类经标签策略落到历史动作。
  关键：π 是**额外输入**，I 本身不提供 π。
-/
def inducedAction {H S A : Type} (I : H → S) (π : S → A) : H → A :=
  fun h => π (I h)

/--
  ★★★元定理 1（核心理论结论的 L0 兑现）★★★：
  `classification_does_not_choose_unique_policy` — 分类不推出唯一策略。

  若结构标签类型 `S` 非空（`Inhabited S`）且动作集存在两个不同元素 `a ≠ b`，
  则策略空间 `S → A` **至少有两个不同元素** `π₁ ≠ π₂`。

  含义：分类 I : H → S 只给「当前属于什么结构」（标签），即使 S 完全确定，
  策略（S → A）仍有 ≥ 2 个全定义选项。**分类本身不在它们之间选**——选择需 Θ。

  ★诚实标注证明强度（codex 自审，避免声明膨胀 / no-patch-mentality 禁令5）：
  本定理证的是「完全分类 ⊬ 唯一策略」的**必要条件见证**——策略空间 `S → A` 非单点
  （≥ 2 个全定义策略）。它**不引用** `Strict.Classifies`/分类器 `I` 的内部结构（参数里
  没有 I），故**不是**「用了分类结构推出非唯一」的内生定理，而是「策略空间本身就非单点，
  分类无论怎样都无法在 Lean 类型层把它收缩为唯一选择」的**模型论见证**。
  「⊢ 一族策略 {π_Θ}」的正面内容由元定理 2（`given_theta_total_unique`，给定 Θ ⟹ 唯一）
  + `StrategyFamily`（Param 索引族）承载，**不**由本定理单独兑现。

  ★前提的非退化性：A 至少 2 元（`hne : a ≠ b`）+ S 非空（`Inhabited S`，使常值策略在某点
  可区分）。若 A 单元素或 S 空，策略空间退化为单点（此时无可操作内容）。真实操盘
  A ⊇ {buy, wait, …}（≥2 元）⟹ 前提满足。
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
  存在 π₁ π₂ 使 `inducedAction I π₁ ≠ inducedAction I π₂`——即便分类 I 固定，
  诱导的历史动作仍随策略选择而变（分类不锁定历史动作）。需 H 非空（有历史可区分）+
  I 满射到某可区分标签。这里用最简形式：H 经 I 命中 default 标签即可区分。
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
  ## §2 因子化 vs 产生（codex Q1 关键区分）

  分类**能**把已给定的 fiber-constant 历史策略**唯一因子化**为标签策略；
  但分类**不能产生**该策略（f 的选择需 extra-缠论输入）。这精确分离两件事。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★fiber 常值 `FiberConstant I f`（codex Q1）：历史策略 `f : H → A` 在分类 `I` 的
  每个 fiber 上常值——`I h = I h' → f h = f h'`（同标签历史给同动作）。
  这是「f 可经标签因子化」的充要前提。
-/
def FiberConstant {H S A : Type} (I : H → S) (f : H → A) : Prop :=
  ∀ h h', I h = I h' → f h = f h'

/--
  ★因子化 `Factors I f π`（codex Q1）：标签策略 `π : S → A` 是历史策略 `f` 的因子——
  `∀ h, f h = π (I h)`（f = π ∘ I）。即 f 经 I 因子化为标签层策略 π。
-/
def Factors {H S A : Type} (I : H → S) (f : H → A) (π : S → A) : Prop :=
  ∀ h, f h = π (I h)

/--
  ★因子存在（L0，codex Q1）：若 `I` 满射（每个标签 s 都被某历史 realized）且
  `f` fiber-constant，则存在因子 π。

  构造：对每个标签 s，取一个见证历史 `hs`（`I hs = s`），令 `π s = f hs`。
  fiber-constant 保证 π 良定义（不同见证给同值）。
  ★注：本项目不依赖 Mathlib 的选择公理记号，故用 `realized : ∀ s, ∃ h, I h = s` 的
  显式见证 + `Classical.choice`（Lean core 自带）提取见证历史。
-/
theorem factor_exists {H S A : Type} (I : H → S) (f : H → A)
    (hfc : FiberConstant I f)
    (hrealized : ∀ s, ∃ h, I h = s) :
    ∃ π : S → A, Factors I f π := by
  -- 对每个标签 s，用 Classical.choice 提取见证历史，令 π s = f(见证)
  refine ⟨fun s => f (Classical.choose (hrealized s)), ?_⟩
  intro h
  -- I (Classical.choose (hrealized (I h))) = I h（见证的标签 = I h）
  have hwit : I (Classical.choose (hrealized (I h))) = I h :=
    Classical.choose_spec (hrealized (I h))
  -- fiber-constant：同标签 ⟹ f 同值
  exact (hfc h (Classical.choose (hrealized (I h))) hwit.symm)

/--
  ★★因子唯一（L0，codex Q1 关键定理）：若 `I` 满射（每标签 realized），则
  fiber-constant 历史策略 `f` 的因子 π **唯一**——任意两个因子 π₁ π₂ 处处相等。

  这是「分类能**唯一因子化**已给定的 fiber-constant 策略」的精确形式：给定 f，
  因子 π 由 `π s = f(I⁻¹ s 的任一见证)` 完全钉死（realized 保证每标签有见证）。

  ★但这**不**意味分类产生了 f——f 是**输入**（已给定的 fiber-constant 历史策略），
  因子化只是把已有的 f 重新表达为标签层。`f` 的选择本身需 extra-缠论输入（见
  `classification_does_not_produce_strategy`）。**因子化 ≠ 产生**。
-/
theorem factor_unique {H S A : Type} (I : H → S) (f : H → A)
    (hrealized : ∀ s, ∃ h, I h = s)
    (π₁ π₂ : S → A) (h1 : Factors I f π₁) (h2 : Factors I f π₂) :
    ∀ s, π₁ s = π₂ s := by
  intro s
  -- 取标签 s 的见证历史 h（I h = s）
  obtain ⟨h, hIh⟩ := hrealized s
  -- π₁ s = π₁ (I h) = f h = π₂ (I h) = π₂ s
  rw [← hIh]
  rw [← h1 h, ← h2 h]

/--
  ★★分类不产生策略（L0，codex Q1：因子化 ≠ 产生）：
  分类 I **本身不在** fiber-constant 历史策略之间做选择——只要动作集 ≥ 2 元，
  存在两个**不同的** fiber-constant 历史策略 `f₁ ≠ f₂`（常 a 与常 b），它们都
  在 I 上 fiber-constant，但 I 不偏好任一个。

  这与 `factor_unique` 对照：给定 f，因子 π 唯一（**因子化**确定）；但**给定 I，
  f 不唯一**（**产生**不确定）。分类提供因子化机制，不提供策略选择——选择需 Θ。
  这是元定理 1 在「历史策略」层的对应（§1 是「标签策略」层）。
-/
theorem classification_does_not_produce_strategy
    {H S A : Type} [Inhabited H] {a b : A} (hne : a ≠ b) (I : H → S) :
    ∃ f₁ f₂ : H → A,
      FiberConstant I f₁ ∧ FiberConstant I f₂ ∧ f₁ ≠ f₂ := by
  refine ⟨fun _ => a, fun _ => b, ?_, ?_, ?_⟩
  · intro h h' _; rfl   -- 常 a 显然 fiber-constant
  · intro h h' _; rfl   -- 常 b 显然 fiber-constant
  · intro hcontra
    have := congrFun hcontra default
    exact hne this

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 风险投影确定选择器（codex Q2 关键修正：非冒充 argmin 存在性）

  `0 ∈ 𝓚`（零仓可行）只保证可行集**非空**，**不**保证 argmin 存在。故把
  `RiskProjector` 抽象为**带证明的确定选择器**——project 给出可行的确定选择，且若某点
  是 LexArgmin（字典序最优）则它必 = project。**不先碰凸分析**（存在性条件留实例化）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险投影确定选择器 `RiskProjector Target Pos`（codex Q2，L0）。

  - `feasible : Target → Pos → Prop`：目标 y 下仓位 p 可行（满足风险约束 𝓚）。
  - `project : Target → Pos`：**确定**选择器——对每个目标给出唯一仓位。
  - `IsLexArgmin : Target → Pos → Prop`：p 是 y 下的字典序最优（抽象判据，不展开凸分析）。
  - `project_feasible`：投影结果可行（project y 满足 feasible y）。
  - `project_unique`：**若** p 可行且 lex-最优，则 p = project y——project 与 lex-argmin
    在「存在 lex-argmin 时」一致；但 project 的**全定义性不依赖** lex-argmin 存在
    （codex Q2：0∈𝓚 只保非空，argmin 可能不存在，故 project 是独立给定的确定选择器）。

  ★诚实标注：本结构**不**断言 project = argmin（argmin 可能不存在）。它断言：
  (1) project 全定义且可行（确定选择）；(2) 若 lex-argmin 存在则 project 命中它（一致性）。
  这避免「冒充 argmin 存在性」——project 是**带证明的确定选择器**，非「最优解存在性」声明。
-/
structure RiskProjector (Target Pos : Type) where
  feasible : Target → Pos → Prop
  IsLexArgmin : Target → Pos → Prop
  project : Target → Pos
  project_feasible : ∀ y, feasible y (project y)
  project_unique :
    ∀ y p, feasible y p → IsLexArgmin y p → p = project y

/--
  ★投影确定性（L0，codex Q2）：`project` 是函数 ⟹ 对每个目标给出**唯一**仓位。
  这是「确定选择器」的核心——∀ y, ∃! p, project y = p（全 + 唯一），不依赖 argmin 存在。
-/
theorem RiskProjector.project_existsUnique {Target Pos : Type}
    (R : RiskProjector Target Pos) (y : Target) :
    ∃ p, R.project y = p ∧ ∀ p', R.project y = p' → p' = p :=
  ⟨R.project y, rfl, fun _ heq => heq.symm⟩

/--
  ★投影可行（L0，codex Q2）：`project y` 始终落在可行集内（feasible y (project y)）。
  确定选择器的可行性保证——选出的仓位满足风险约束（不越界）。
-/
theorem RiskProjector.project_in_feasible {Target Pos : Type}
    (R : RiskProjector Target Pos) (y : Target) :
    R.feasible y (R.project y) :=
  R.project_feasible y

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 Θ（风险公理）+ piTheta + 元定理 2（给定 Θ ⟹ 全定义 / 唯一 / 因果）

  Θ 把分类标签外的内容（rec / target / riskProj / exec）参数化。给定 Θ 后，
  π_Θ : H → Z → Order 全定义 + 唯一 + 因果。Θ 的内容是 extra-缠论设计选择（不由分类导出）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★同前缀 `SamePrefix`（复用 Classification.lean 的 `PrefixEq` 风格）。

  历史 `H` 经一个「事件流读出」`hist : H → (Nat → E)` 与时刻 `t : H → Nat` 投影到
  事件流前缀。`SamePrefix h h'` ⟺ 两历史的事件流在各自时刻及之前逐点相等——
  这是缠论「无前视」（当下只依赖已出现数据）的因果前提（同 Causal.lean §4）。

  ★为保持与 Causal.lean 一致且自包含，这里 `SamePrefix` 直接定义在抽象事件流上：
  给定流读出 `read : H → Nat → E`，`SamePrefix read t h h'` = 两历史前 t 个事件相等。
-/
def SamePrefix {H E : Type} (read : H → Nat → E) (t : Nat) (h h' : H) : Prop :=
  PrefixEq (read h) (read h') t

/--
  ★Θ 风险公理结构 `Theta`（codex Q3，L0）。

  Θ 把「分类标签 S 外」的全部执行内容参数化为四个全函数 + 三项证明义务：
  - `recog : H → Z → D`：识别/决策核（从历史 h 与账户状态 Z 读出决策意图 D）。
  - `target : D → Z → Target`：决策 + 账户 → 目标仓位/风险目标。
  - `riskProj : RiskProjector Target Pos`：风险投影确定选择器（§3，把 Target 投到可行 Pos）。
  - `exec : D → Z → Pos → Order`：执行映射（决策 + 账户 + 投影仓位 → 订单）。

  证明义务（因果 + 全定义性，全部 L0）：
  - `recog_causal`：recog 因果无前视——同前缀 + 同账户 ⟹ 同决策（不偷看未来）。
  - `read`：历史的事件流读出（SamePrefix 的载体）。

  ★字段名 `recog`（不用 `rec`）：`rec` 是 Lean 每个 structure 自动生成的递归子保留名，
    不可作字段。`recog` = recognition（识别核），语义同 codex 骨架的 `rec`。

  ★诚实标注：`recog`/`target`/`exec`/`riskProj` 的**具体内容**（用什么风险阈值 ρ/β/γ、
  成本模型 κ、止损规则、调度）全部是 **Θ-参数化，非 L0 可导**——它们不由分类 I 推出，
  是 extra-缠论设计选择。本结构只证「给定这些 Θ-组件 ⟹ 组合出的 π_Θ 全定义/唯一/因果」。
-/
structure Theta (H Z D Target Pos Order E : Type) where
  read : H → Nat → E
  recog : H → Z → D
  target : D → Z → Target
  riskProj : RiskProjector Target Pos
  exec : D → Z → Pos → Order
  /-- recog 因果无前视：同时刻 t 下同前缀 + 同账户 ⟹ 同决策（不偷看未来）。 -/
  recog_causal : ∀ (t : Nat) (h h' : H) (z : Z),
    SamePrefix read t h h' → recog h z = recog h' z

/--
  ★π_Θ 组合 `piTheta`（codex Q3，L0）：给定 Θ，从历史 h 与账户状态 z 产出唯一订单。

  组合链：`recog h z` → `target d z` → `riskProj.project y` → `exec d z p`。
  这是「给定 Θ 的完整操作语义」——把分类标签外的决策/目标/风险投影/执行串成
  **全定义**的订单产出函数（standard §6 `Strategy` 的执行层深化）。
-/
def piTheta {H Z D Target Pos Order E : Type}
    (θ : Theta H Z D Target Pos Order E) (h : H) (z : Z) : Order :=
  let d := θ.recog h z
  let y := θ.target d z
  let p := θ.riskProj.project y
  θ.exec d z p

/--
  ★★★元定理 2（给定 Θ ⟹ 全定义 + 唯一）★★★：
  `given_theta_total_unique` — 给定 Θ，π_Θ 对每个历史 × 账户**全定义且唯一**。

  对任意 Θ、历史 h、账户状态 z，**存在唯一**订单 o 使 `piTheta θ h z = o`。

  这是「给定 Θ 后 π_Θ 全定义且对每状态产出唯一动作」的 L0 形式——与元定理 1
  （分类 ⊬ 唯一策略）形成对照：**分类不选，但给定 Θ 后选择被唯一钉死**。

  ★诚实标注证明强度（codex 自审，避免声明膨胀）：这是「函数图的 ∃!」——`piTheta` 是
  Lean 全函数，故全定义（= 存在）与唯一（= 函数确定性）**自动成立**，证明体仅 `rfl` +
  对称。它的内容**不是**「证明了 Θ 使 π 变唯一」的实质命题，而是「一旦把 Θ 的组件定为
  全函数（recog/target/exec）+ 确定选择器（riskProj.project），组合 piTheta 即是全函数」
  的接口兑现。**实质的非唯一性**（分类层 ≥2 策略）在元定理 1；**实质的因果约束**在
  `piTheta_causal`（依赖 `recog_causal` 这一**非平凡**前提）。本定理只兑现「Θ 固定后
  确定性」这一最弱必要侧。
-/
theorem given_theta_total_unique {H Z D Target Pos Order E : Type}
    (θ : Theta H Z D Target Pos Order E) (h : H) (z : Z) :
    ∃ o, piTheta θ h z = o ∧ ∀ o', piTheta θ h z = o' → o' = o :=
  ⟨piTheta θ h z, rfl, fun _ heq => heq.symm⟩

/--
  ★★π_Θ 因果无前视（元定理 2 的因果侧，L0）：
  给定 Θ，若两历史在时刻 t 同前缀且账户状态相同，则 π_Θ 产出相同订单。

  `SamePrefix θ.read t h h' → piTheta θ h z = piTheta θ h' z`——π_Θ 的当下决策只依赖
  已出现数据（前缀）+ 当下账户，**不偷看未来**。这是缠论「当下判断不能用未来数据」
  （T49 向心回溯）在策略层的兑现：因果性沿 recog 传导到整条组合链（target/riskProj/exec
  都只读 recog 的输出 d、账户 z、投影 p，不另引历史，故不另引未来）。

  ★诚实标注因果负担的承载（codex 自审）：实质的因果约束**全压在** `recog_causal`
  （Θ 的证明义务，**非平凡前提**——它要求 recog 真的只读前缀）。`target/riskProj/exec`
  的因果性是**类型接口切断**的结果（它们的签名 `D→Z→…` 不接受历史 H，故无法偷读未来），
  **不是**对它们各自时间因果的独立证明。**未覆盖**：账户状态 `z` 的**生成**因果性——本定理
  比较**同一个 z** 下的两历史，不证 z 本身只依赖前缀（z 的因果性需账户层 Tlayers.Accounting
  单独承载，属本文件有效域之外）。
-/
theorem piTheta_causal {H Z D Target Pos Order E : Type}
    (θ : Theta H Z D Target Pos Order E) (t : Nat) (h h' : H) (z : Z)
    (hpre : SamePrefix θ.read t h h') :
    piTheta θ h z = piTheta θ h' z := by
  unfold piTheta
  -- recog 因果 ⟹ d 相等；d 相等 ⟹ target/project/exec 整条链相等
  rw [θ.recog_causal t h h' z hpre]

/--
  ★策略族结构 `StrategyFamily`（codex Q3，L0）：参数 Θ 索引的策略族 {π_Θ}。

  - `π : Param → H → Z → Order`：参数化策略族——每个 Θ-参数给一个全定义策略。
  - `total_unique`：∀ θ h z, π θ 全定义且唯一（每个参数下都是确定策略）。
  - `causal`：∀ θ，同前缀 + 同账户 ⟹ 同订单（每个参数下都因果无前视）。

  这是「完全分类 ⊢ 一族策略 {π_Θ}」的结构容器——分类不选 Θ（元定理 1），但每个
  Θ ∈ Param 都给出一个全定义/唯一/因果的策略（元定理 2）。族 = Param 索引的全体 π_Θ。
-/
structure StrategyFamily (H Z Param Order : Type) where
  read : H → Nat → Bool      -- 抽象事件流读出（因果前提载体，Bool 为最小事件型占位）
  π : Param → H → Z → Order
  total_unique :
    ∀ θ h z, ∃ o, π θ h z = o ∧ ∀ o', π θ h z = o' → o' = o
  causal :
    ∀ θ (t : Nat) h h' z,
      SamePrefix read t h h' → π θ h z = π θ h' z

/--
  ★从 Θ-族构造 StrategyFamily（L0）：给定一族 Θ（按 Param 索引的 Theta），
  组装为 `StrategyFamily`，total_unique 与 causal 由元定理 2 + piTheta_causal 填充。

  ★这里 `read` 用 Bool 占位事件型，且要求所有 Θ 共享同一 `read`（同一历史的事件流读出
  对整族一致——因果性是历史结构属性，不随 Θ 变）。
-/
def familyOfTheta {H Z D Target Pos Order : Type} {Param : Type}
    (read : H → Nat → Bool)
    (Θ : Param → Theta H Z D Target Pos Order Bool)
    (hread : ∀ p, (Θ p).read = read) :
    StrategyFamily H Z Param Order where
  read := read
  π := fun p h z => piTheta (Θ p) h z
  total_unique := fun p h z => given_theta_total_unique (Θ p) h z
  causal := fun p t h h' z hpre => by
    apply piTheta_causal (Θ p) t h h' z
    -- SamePrefix read = SamePrefix (Θ p).read（hread 把共享 read 对齐到 Θ p 的 read）
    unfold SamePrefix at hpre ⊢
    rw [hread p]
    exact hpre

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 声部树 VoiceTree：赋格交替 σ_v = -σ_{p(v)} + 级联关闭

  多声部（赋格）结构：每个声部节点的方向 = 其父节点方向的翻转（σ_v = flip σ_{p(v)}）。
  级联关闭：父声部关闭 ⟹ 所有后代声部关闭（对无环 + 深度良基结构的祖先关系归纳；
  ★诚实：不强制载体有限/有根，只需无环 + 深度严格降，见 VoiceTree 结构注释）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部方向翻转 `flipSide`（赋格交替的算子）：long ↔ short（用 Bool：true=long, false=short）。
-/
def flipSide (s : Bool) : Bool := !s

/-- ★翻转对合（L0）：翻两次还原（方向两态）。 -/
theorem flipSide_flipSide (s : Bool) : flipSide (flipSide s) = s := by
  cases s <;> rfl

/-- ★翻转改变方向（L0）：flip s ≠ s（两态对合无不动点，赋格交替非平凡）。 -/
theorem flipSide_ne (s : Bool) : flipSide s ≠ s := by
  cases s <;> simp [flipSide]

/--
  ★声部树 `VoiceTree V`（codex Q3，L0）：无环 + 深度良基的多声部父链结构。

  - `parent : V → Option V`：父声部（`none` = 局部根）。
  - `side : V → Bool`：声部方向（true=long, false=short）。
  - `closed : V → Bool`：声部是否已关闭（仓位平掉）。
  - `depth : V → Nat`：节点深度——**无环良基**的编码（祖先深度严格更小）。
  - `alternating`：**赋格交替** `parent v = some p → side v = flipSide (side p)`
    （子声部方向 = 父声部翻转，σ_v = -σ_{p(v)} 的 L0 形式）。
  - `depth_decreasing`：**无环 + 良基** `parent v = some p → depth p < depth v`
    （父深度严格小于子——保证 parent 链严格降，无环，可对祖先关系归纳）。
  - `cascade_close`：**级联关闭** `parent v = some p → closed p = true → closed v = true`
    （父声部关闭蕴含其直接子声部关闭——级联的单步律，传递闭包由 `ancestor_closed` 证）。

  ★诚实标注结构强度（codex 自审，避免膨胀）：本结构**公理强制的**只有「无环」
  （`depth_decreasing` ⟹ `not_self_ancestor`）+「parent 链深度严格降」。它**不强制**
  「有限」（`V` 无 `Finite` 约束，可为无限载体）也**不强制**「有根」（不证每个节点经
  有限步 parent 必达某 `none` 局部根——`depth : Nat` 给出深度下界使每条 parent 链有限，
  但 `V` 全体可能有多个不连通的局部根）。级联关闭定理（`ancestor_closed`）只需「无环 +
  深度良基」即成立，不依赖「有限/有根」，故本结构的最小公理集已足够。
-/
structure VoiceTree (V : Type) where
  parent : V → Option V
  side : V → Bool
  closed : V → Bool
  depth : V → Nat
  /-- 赋格交替：子声部方向 = 父声部翻转（σ_v = -σ_{p(v)}）。 -/
  alternating : ∀ v p, parent v = some p → side v = flipSide (side p)
  /-- 无环 + 良基：父深度 < 子深度（parent 链深度严格降，故无环；不强制必达根）。 -/
  depth_decreasing : ∀ v p, parent v = some p → depth p < depth v
  /-- 级联关闭单步：父关 ⟹ 直接子关。 -/
  cascade_close : ∀ v p, parent v = some p → closed p = true → closed v = true

/--
  ★相邻声部反向（L0，赋格交替推论）：子声部方向 ≠ 父声部方向。
  `parent v = some p → side v ≠ side p`——相邻声部必反向（σ_v = -σ_{p(v)} ⟹ σ_v ≠ σ_{p(v)}）。
-/
theorem VoiceTree.adjacent_opposite {V : Type} (T : VoiceTree V)
    (v p : V) (hpar : T.parent v = some p) :
    T.side v ≠ T.side p := by
  rw [T.alternating v p hpar]
  exact flipSide_ne (T.side p)

/--
  ★祖先关系 `IsAncestor`（L0）：`IsAncestor T a v` ⟺ a 是 v 的祖先（经 ≥1 步 parent 链可达）。
  归纳定义：直接父是祖先（base）；祖先的祖先是祖先（step）。
-/
inductive IsAncestor {V : Type} (T : VoiceTree V) : V → V → Prop where
  | base : ∀ v p, T.parent v = some p → IsAncestor T p v
  | step : ∀ v p a, T.parent v = some p → IsAncestor T a p → IsAncestor T a v

/--
  ★★级联关闭（传递闭包，L0，codex Q3 核心）：祖先关闭 ⟹ 后代关闭。
  `IsAncestor T a v → closed a = true → closed v = true`——任一祖先声部关闭，则该后代
  声部关闭（不只直接父，整条祖先链的关闭都级联到后代）。

  证明：对 `IsAncestor` 归纳——
  - base（a 是 v 的直接父）：`cascade_close` 单步律直接给。
  - step（a 是 v 父 p 的祖先）：归纳假设给 p 关闭，再 `cascade_close` 从 p 到 v。
  这是「父声部关闭 ⟹ 所有后代声部关闭」的精确传递形式（无环 + 深度良基由
  `depth_decreasing` 保证 IsAncestor 归纳合法；`IsAncestor` 自身的归纳结构已足够，
  不依赖载体有限/有根）。
-/
theorem VoiceTree.ancestor_closed {V : Type} (T : VoiceTree V)
    (a v : V) (hanc : IsAncestor T a v) (hclosed : T.closed a = true) :
    T.closed v = true := by
  induction hanc with
  | base v p hpar =>
      exact T.cascade_close v p hpar hclosed
  | step v p a hpar _hanc_a_p ih =>
      -- ih : T.closed a = true → T.closed p = true（祖先 a 关 ⟹ p 关）
      exact T.cascade_close v p hpar (ih hclosed)

/--
  ★祖先深度严格更小（L0，无环良基的推论）：`IsAncestor T a v → depth a < depth v`。
  祖先的深度严格小于后代——这从 `depth_decreasing` 沿祖先链累积。
  保证 `IsAncestor` 无环（a 不可能是自己的祖先），级联关闭归纳良基。
-/
theorem VoiceTree.ancestor_depth_lt {V : Type} (T : VoiceTree V)
    (a v : V) (hanc : IsAncestor T a v) :
    T.depth a < T.depth v := by
  induction hanc with
  | base v p hpar =>
      exact T.depth_decreasing v p hpar
  | step v p a hpar _hanc ih =>
      exact Nat.lt_trans ih (T.depth_decreasing v p hpar)

/--
  ★声部不是自己的祖先（L0，无环见证）：`¬ IsAncestor T v v`。
  若 v 是自己的祖先，则 `depth v < depth v`（`ancestor_depth_lt`），矛盾。
  这关死声部树的环——级联关闭的传递归纳不会无限循环。
-/
theorem VoiceTree.not_self_ancestor {V : Type} (T : VoiceTree V) (v : V) :
    ¬ IsAncestor T v v := by
  intro h
  exact Nat.lt_irrefl (T.depth v) (T.ancestor_depth_lt v v h)

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 多空双开结构允许（codex Q3：不加 Q⁺Q⁻=0 禁令）

  本文件**不**加「净仓为零」或「多空互斥」的结构禁令——声部树允许同一时刻
  存在 long 声部与 short 声部并存（多空双开）。结构上不禁止；是否**实际可行**
  （存在双开可行点）需额外容量/上限假设，本文件不声称（EmpiricalDomain）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★多空双开结构允许（L0，codex Q3）：声部树**不禁止**同时存在 long 与 short 声部。

  存在一棵声部树 T，含两个未关闭声部 vL（side=long）与 vS（side=short）——
  即结构上多空双开是**可表达**的（不被类型/约束排除）。这与赋格交替不冲突：
  交替约束的是**父子**方向（σ_v = -σ_{p(v)}），不约束**非父子**声部的方向组合，
  故多空并存（在不同子树或同层）结构合法。

  ★诚实标注：这只证「结构允许」（双开可表达且满足所有 VoiceTree 公理）。它**不**证
  「双开总可行」——实际双开是否满足保证金/容量约束是 Θ-参数化的风险约束（riskProj 的
  feasible），属 EmpiricalDomain，不由此 L0 声称。
-/
theorem long_short_both_open_allowed :
    ∃ (T : VoiceTree Bool) (vL vS : Bool),
      T.side vL = true ∧ T.side vS = false ∧
      T.closed vL = false ∧ T.closed vS = false := by
  -- 用 V = Bool：vL = true（long 声部）, vS = false（short 声部），两者都是根（parent = none）
  refine ⟨{
    parent := fun _ => none          -- 两声部都是根（无父，故 alternating 真空满足）
    side := fun v => v                -- true=long, false=short
    closed := fun _ => false          -- 都未关闭
    depth := fun _ => 0               -- 都在根层
    alternating := by intro v p hpar; simp at hpar
    depth_decreasing := by intro v p hpar; simp at hpar
    cascade_close := by intro v p hpar; simp at hpar
  }, true, false, rfl, rfl, rfl, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 标签声明（formalization-validity-domain + 615 gatekeeper，诚实分层）

  把 π_Θ 的认识论标签钉为可下游引用的结构——**禁标 TrueCompleteClassification**。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★π_Θ 标签种类 `PolicyTag`（615 gatekeeper，诚实分层）。

  - `OperationalSemanticsOnly`：操作语义（给定 Θ 的全定义订单产出），**不是分类定理**。
  - `RuntimeGuard`：运行时守卫（风险约束/可行性在运行时由 riskProj.feasible 把守）。
  - `EmpiricalDomain`：经验有效域（盈利/最优性 = L3，需真实数据，不由 L0 声称）。

  ★**没有** `TrueCompleteClassification` 构造子——类型层就拒绝把 π_Θ 标为真完全分类。
  π_Θ 是「分类结构 + extra-缠论 Θ 后的确定执行策略族」（StrategyFamilyGivenTheta），
  不是分类本身。这坐实「完全分类 ⊬ 唯一策略」——策略族是分类的下游，不是分类的内容。
-/
inductive PolicyTag where
  | OperationalSemanticsOnly
  | RuntimeGuard
  | EmpiricalDomain
deriving DecidableEq, Repr

/--
  ★π_Θ 子类标记 `PolicySubkind`（615 gatekeeper）：StrategyFamilyGivenTheta。
  唯一子类——给定 Θ 的策略族。**没有** TrueCompleteClassification 子类（类型层拒绝冒充）。
-/
inductive PolicySubkind where
  | StrategyFamilyGivenTheta
deriving DecidableEq, Repr

/--
  ★π_Θ 的诚实标签包 `piThetaLabels`（L0 声明）：
  主标签 OperationalSemanticsOnly + 附标签 RuntimeGuard + 盈利/最优 EmpiricalDomain，
  子类 StrategyFamilyGivenTheta。这是 π_Θ 产出的认识论自我声明——下游引用此标签即知
  「这是给定 Θ 的操作语义，非真完全分类，盈利性需 L3 验证」。
-/
def piThetaLabels : List PolicyTag × PolicySubkind :=
  ([PolicyTag.OperationalSemanticsOnly, PolicyTag.RuntimeGuard, PolicyTag.EmpiricalDomain],
   PolicySubkind.StrategyFamilyGivenTheta)

/--
  ★禁标真完全分类（L0，615 gatekeeper 见证）：π_Θ 的子类标记必是 StrategyFamilyGivenTheta。
  `PolicySubkind` 只有 `StrategyFamilyGivenTheta` 一个构造子——任意子类标记必是它。

  ★诚实标注强度（codex 自审，避免膨胀）：这是**本文件本地标签类型**的平凡枚举定理——
  它保证「在 `PolicySubkind` 这个类型里，π_Θ 不可能被标成真完全分类」（因 `PolicySubkind`
  根本不含 TrueCompleteClassification 构造子）。它**不是**「全项目/跨库禁止冒充」的强保证
  （`TrueCompleteClassification` 是另一类型 `Strict.Classifies` 域的概念，不在此枚举内）。
  本定理的诚实内容：本文件的标签 API **不提供**冒充真完全分类的途径——π_Θ 自我声明为
  StrategyFamilyGivenTheta（分类的下游操作语义），而非分类本身。
-/
theorem piTheta_not_true_classification (k : PolicySubkind) :
    k = PolicySubkind.StrategyFamilyGivenTheta := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §8 有效域诚实标注总结（formalization-validity-domain + no-patch-mentality）

  本文件**忠实交付**（L0，machine-checked）：
  1. 元定理 1（分类 ⊬ 唯一策略）：`classification_does_not_choose_unique_policy`
     （标签策略层 ≥2 元）+ `classification_induces_policy_family`（历史动作族）+
     `classification_does_not_produce_strategy`（fiber-constant 历史策略 ≥2，分类不选）。
  2. 因子化 vs 产生：`factor_exists`/`factor_unique`（分类能**唯一因子化**已给 fiber-constant
     策略）对照 `classification_does_not_produce_strategy`（但分类**不产生**策略）。
  3. RiskProjector 确定选择器：`project_existsUnique`（全 + 唯一）+ `project_in_feasible`
     （可行）——**非冒充 argmin 存在性**（codex Q2：0∈𝓚 只保非空）。
  4. 元定理 2（给定 Θ ⟹ 全定义/唯一/因果）：`given_theta_total_unique` + `piTheta_causal` +
     `familyOfTheta`（组装 StrategyFamily）。
  5. 声部树赋格交替 + 级联关闭：`alternating`（σ_v=-σ_{p(v)}）+ `adjacent_opposite` +
     `ancestor_closed`（祖先关 ⟹ 后代关，对 IsAncestor 归纳）+ `not_self_ancestor`（无环）。
  6. 多空双开结构允许：`long_short_both_open_allowed`（不加 Q⁺Q⁻=0 禁令）。
  7. 诚实标签：`piThetaLabels`（OperationalSemanticsOnly + RuntimeGuard + EmpiricalDomain +
     StrategyFamilyGivenTheta）+ `piTheta_not_true_classification`（禁标 TrueCompleteClassification）。

  本文件**不声明**（有效域边界，诚实标注，避免膨胀）：
  - ✗ Θ 来自缠论——Θ（ρ/β/γ/w/κ/成本/止损/优先级/调度）是 **extra-缠论设计选择**，
       全部 Θ-参数化，**不由分类 I 推出**（元定理 1 + `classification_does_not_produce_strategy`
       正是证此：分类不产生策略选择）。
  - ✗ 收益最优 / 盈利——`piTheta` 只证全定义/唯一/因果（操作语义），**不证**它盈利或最优。
       盈利/最优需收益分布 + 成本 + 效用（L3 EmpiricalDomain，标 `EmpiricalDomain`，不由此 L0 声称）。
  - ✗ RiskProjector.project = argmin——project 是**确定选择器**（带证明），argmin 可能不存在
       （codex Q2：0∈𝓚 只保可行集非空）。`project_unique` 只在「lex-argmin 存在时」一致，
       project 的全定义性**不依赖** argmin 存在。**不冒充凸优化最优解存在性**。
  - ✗ 多空双开总可行——`long_short_both_open_allowed` 只证**结构允许**（可表达），不证
       实际双开满足保证金/容量约束（Θ-参数化风险约束，EmpiricalDomain）。
  - ✗ π_Θ 是真完全分类——`piTheta_not_true_classification` 证 π_Θ **禁标**
       TrueCompleteClassification。π_Θ 是分类的**下游**（给定 Θ 的操作语义），不是分类内容。

  ★与现有 Strict 库的关系：本文件**复用** `Strict.PrefixEq`（Classification.lean:119）的
    前缀风格定义 `SamePrefix`（因果前提），是 `Strict.Op`（standard §6 `Strategy` 标签层）的
    **执行层深化**——Op.lean 证「动作标签全定义」（`opStrategy_total`），本文件证「给定 Θ 的
    完整订单语义全定义/唯一/因果」+「分类 ⊬ 唯一策略，⊢ 一族 {π_Θ}」。两者互补
    （标签完全应对 + Θ-参数化执行族）。
-/

end Strict.StrategyFamily
