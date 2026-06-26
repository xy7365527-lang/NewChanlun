/-
Origin/ForceInterface.lean — 力度抽象接口（背驰判据的 L0 接口层 + L2 边界划线，task #124）

★工位定位（#124 力度 L2-interface，形式化主线收尾）：
  committed Origin/Divergence.lean 把 `Force`（MACD 柱子面积）作为抽象标量，背驰判据
  `IsDivergence d = d.forceC.area < d.forceA.area` 直接消费 Force——但**力度从何而来**留在外
  （Divergence 标 still-MISSING-C："无任何 Origin 模块从 K 线算力度"；#119 SubLevelDescent 把
  `divPair` 设为显式参数，诚实把力度数据来源留在外）。

  本文件把「力度 measure」形式化为**抽象 interface** `ForceMeasure`：一个从任意走势载体 `α`
  到 `Force` 的函数 + 它必须满足的**公理性质**（力度序的逻辑结构）。背驰判据经此 interface
  表达，对齐 committed `Divergence.IsDivergence`。这给出「背驰的形式化独立于具体 MACD 实现」
  的严格表达：任意满足 interface 公理的 ForceMeasure 都使背驰判据 well-defined。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- 第24课（背驰-买卖点定理，博文最终权威）：趋势背驰——同向两段，后段力度弱于前段
  （MACD 柱子面积 C < A）。
- §9（背驰，知识库）：力度比较；趋势背驰 vs 盘整背驰。
- §11（区间套力度比较，知识库）：低级别背驰是本级别背驰的必要条件。
- committed Origin/Divergence.lean：`Force`（area : Nat 抽象标量）+ `IsDivergence d`
  （forceC < forceA）——本文件经 interface 重表达此判据，结构对齐。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）+ L0/L2 边界精确划线
═══════════════════════════════════════════════════════════════════════════
本文件 = **interface（L0）**，明确与 **MACD 计算引擎（L2）** 划界：

  ┌─────────────────────────────────────────────────────────────────────┐
  │ L0（本文件实装）：力度比较的**逻辑性质**                              │
  │   · ForceMeasure interface：走势载体 α → Force 的函数 + 公理性质      │
  │     （力度序与 Force.area 全序对齐：单调/反对称/可比/与 < 兼容）。     │
  │   · 背驰判据经 interface 表达（`IsDivergenceVia`），对齐              │
  │     committed Divergence.IsDivergence。                              │
  │   · 证：任意满足公理的 ForceMeasure ⟹ 背驰判据 well-defined +        │
  │     互斥穷尽 + 与具体 Force 标量判据等价（独立于 MACD 实现）。        │
  ├─────────────────────────────────────────────────────────────────────┤
  │ L2（本文件**不实装**，诚实标 still-MISSING-C）：具体 MACD 面积计算    │
  │   · 从 K 线序列计算 MACD 柱子面积：EMA(12)/EMA(26) → DIF →           │
  │     DEA(9) → 柱 = 2·(DIF − DEA) → 同向段面积积分。                   │
  │   · 这是数值引擎层（浮点/积分/参数），不是力度比较的逻辑性质。        │
  │   · 形式化侧只给 interface + 公理（本文件）；计算侧诚实标 L2 不实装   │
  │     不冒充（formalization-validity-domain：L0 接口不冒充 L2 数值有效）。│
  └─────────────────────────────────────────────────────────────────────┘

`lake env lean Origin/ForceInterface.lean` 通过 = 力度 interface 的公理逻辑性质 + 背驰判据
经 interface well-defined + 与具体标量判据等价 在定义层成立，**不是**任何「某 ForceMeasure
实例在真实 K 线上算出的力度有效」的实证断言（那需 L2 MACD 引擎 + 真实数据，本文件不冒充）。

★诚实标注（no-patch-mentality / no-声明膨胀）：
  · interface 的公理性质**非空洞恒真**——给反退化见证（§4）：一个不满足力度序的伪 measure
    被 interface 拒绝（无法构造满足公理的实例）。若公理可被任意函数平凡满足，则 interface
    无内容（声明膨胀）；反退化见证证明公理真能否决伪 measure。
  · MACD 计算诚实标 L2 不实装——本文件无任何 `Force` 的 K 线计算，`ForceMeasure.measure`
    是抽象函数参数（由外部 L2 引擎提供），不藏 MACD 计算进 def 假装算了。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。
-/

import Origin.Divergence

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 力度 measure 抽象接口（ForceMeasure）：走势载体 → Force + 公理性质

    ★L0 边界：interface 只规定「力度比较满足的逻辑性质」，不规定「力度怎么从 K 线算」。
    `measure : α → Force` 是抽象函数参数——具体实例（如真实 MACD 面积）由外部 L2 引擎填充。
    本节给 interface 的公理性质：它们刻画「力度序」必须满足的逻辑结构（与 Force.area 全序
    对齐）。这些性质是 L0（逻辑），不依赖任何数据。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **力度 measure 抽象接口（L0，§9 力度序的逻辑结构）** —— 一个力度 measure 把走势载体 `α`
  的元素映到 `Force`（MACD 柱子面积抽象标量），并满足力度比较的公理性质。

  ★字段（接口承诺）：
  - `measure : α → Force`：抽象力度函数（从走势载体到力度标量）。**不规定计算方式**——
    具体计算（MACD 面积，L2）由外部引擎提供。这是 interface 的核心抽象：力度比较的逻辑
    独立于力度的来源。
  - `strength : α → Nat`：走势载体携带的「内在强度序」代理（如段的几何幅度/动量代理）。
    interface 要求 `measure` 与此内在序**单调对齐**（见 `mono`）——这是公理的**内容**：
    力度 measure 不能任意，必须尊重走势的内在强度序。
  - `mono`：**单调公理**——内在强度更大 ⟹ 力度不更小（`strength a ≤ strength b →
    (measure a).area ≤ (measure b).area`）。这是 interface 的非平凡约束：拒绝「内在更强但
    力度更弱」的伪 measure（反退化见证 §4）。
  - `faithful`：**忠实公理**——力度严格序蕴含内在强度严格序的可比性
    （`(measure a).area < (measure b).area → strength a ≤ strength b`）。这保证背驰判据
    （力度 C < A）读出的力度衰减对应内在强度不增——力度序不脱离内在序凭空判背驰。

  ★为何 `strength` 而非直接对比两个抽象 measure：interface 的内容必须**可被伪 measure 违反**
  才非空洞。若只要求「measure 是函数」，任意函数都满足（恒真，声明膨胀）。引入走势内在强度
  序 `strength` 作为 measure 必须尊重的**外部锚**，公理 `mono`/`faithful` 才有否决力——
  违反单调的伪 measure 无法构造合法 ForceMeasure 实例（§4 反退化见证）。
-/
structure ForceMeasure (α : Type u) where
  measure : α → Force
  strength : α → Nat
  mono : ∀ a b : α, strength a ≤ strength b → (measure a).area ≤ (measure b).area
  faithful : ∀ a b : α, (measure a).area < (measure b).area → strength a ≤ strength b

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 背驰判据经 interface 表达（对齐 committed Divergence.IsDivergence）

    committed `Divergence.IsDivergence d = d.forceC.area < d.forceA.area` 直接消费 Force。
    本节用 ForceMeasure 把背驰判据表达在走势载体 α 上：给定 measure + 前段 a / 后段 c，
    背驰 ⟺ `(measure c).area < (measure a).area`（后段力度严格弱于前段）。这是「背驰判据
    in terms of force interface」——判据不依赖 measure 的具体计算，只依赖 measure 给出的
    Force 比较。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **经 interface 的背驰判据（L0，对齐 §9/第24课）** —— 给定力度 measure `fm` + 前段 `a`、
  后段 `c`，背驰 ⟺ 后段力度严格弱于前段：`(fm.measure c).area < (fm.measure a).area`。

  这是 committed `Divergence.IsDivergence`（forceC < forceA）的 interface 形式：把
  `forceA`/`forceC` 替换为 `fm.measure a`/`fm.measure c`。判据**独立于 measure 的具体计算**
  （MACD 实现），只读 measure 输出的 Force。
-/
def IsDivergenceVia {α : Type u} (fm : ForceMeasure α) (a c : α) : Prop :=
  (fm.measure c).area < (fm.measure a).area

instance {α : Type u} (fm : ForceMeasure α) (a c : α) : Decidable (IsDivergenceVia fm a c) := by
  unfold IsDivergenceVia; exact inferInstanceAs (Decidable (_ < _))

/--
  **经 interface 的力度延续（L0，对偶）** —— 后段力度 ≥ 前段：`(fm.measure a).area ≤
  (fm.measure c).area`（力度不衰减，趋势延续）。对齐 committed `Divergence.IsContinuation`。
-/
def IsContinuationVia {α : Type u} (fm : ForceMeasure α) (a c : α) : Prop :=
  (fm.measure a).area ≤ (fm.measure c).area

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. well-definedness：任意满足 interface 的 ForceMeasure ⟹ 背驰判据良构

    ★#124 核心证明：背驰的形式化**独立于具体 MACD 实现**——给定任意满足 interface 公理的
    ForceMeasure，背驰判据 well-defined（互斥穷尽 + 与 committed 标量判据等价）。这把「背驰
    判据不依赖力度怎么算」形式化为：判据的逻辑性质对**所有**合法 ForceMeasure 一致成立。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★背驰互斥穷尽经 interface（L0，§9）** —— 对任意 ForceMeasure `fm` + 走势对 `a c`，
  背驰（C<A）与延续（A≤C）互斥穷尽。这把 committed `divergence_dichotomy` 提升到 interface
  层：**任意**合法 measure 下背驰判据都二歧——背驰的二歧结构独立于 measure 的具体计算。
-/
theorem divergenceVia_dichotomy {α : Type u} (fm : ForceMeasure α) (a c : α) :
    IsDivergenceVia fm a c ∨ IsContinuationVia fm a c := by
  unfold IsDivergenceVia IsContinuationVia; omega

/--
  **★背驰·延续互斥经 interface（L0，§9）** —— 不能同时背驰与延续（对齐 committed
  `divergence_continuation_disjoint`）。任意合法 measure 下成立。
-/
theorem divergenceVia_continuation_disjoint {α : Type u} (fm : ForceMeasure α) (a c : α) :
    ¬ (IsDivergenceVia fm a c ∧ IsContinuationVia fm a c) := by
  unfold IsDivergenceVia IsContinuationVia; rintro ⟨h1, h2⟩; omega

/--
  **★interface 背驰 ⟹ committed Force 标量背驰（L0，对齐桥接）** —— 经 interface 判定的
  背驰，等价于在 committed `Divergence.IsDivergence` 上构造的 `DivergencePair`（forceA =
  measure a，forceC = measure c）的背驰。这坐实「interface 判据对齐 committed 判据」——
  二者是同一个 Force 比较，只是 forceA/forceC 由 measure 提供。
-/
theorem divergenceVia_iff_pair {α : Type u} (fm : ForceMeasure α) (a c : α) (isTrend : Bool) :
    IsDivergenceVia fm a c ↔
      IsDivergence { forceA := fm.measure a, forceC := fm.measure c, isTrend := isTrend } := by
  unfold IsDivergenceVia IsDivergence; exact Iff.rfl

/--
  **★背驰判据独立于 measure 计算方式（L0，#124 核心）** —— 若两个 ForceMeasure `fm₁ fm₂`
  在走势对 `a c` 上给出**相同的力度输出**（measure 值相等），则它们对 `a c` 的背驰判定一致
  ——无论二者内部如何计算 measure（一个用 MACD 面积，一个用别的力度代理）。

  这把「背驰的形式化独立于具体 MACD 实现」形式化为**外延性**：背驰判定只依赖 measure 的
  输出（Force），不依赖 measure 的实现。两个输出相同的 measure 给出相同背驰判定。
-/
theorem divergenceVia_depends_only_on_output {α : Type u} (fm₁ fm₂ : ForceMeasure α) (a c : α)
    (ha : fm₁.measure a = fm₂.measure a) (hc : fm₁.measure c = fm₂.measure c) :
    IsDivergenceVia fm₁ a c ↔ IsDivergenceVia fm₂ a c := by
  unfold IsDivergenceVia; rw [ha, hc]

/--
  **★力度序公理使背驰读出内在强度衰减（L0，faithful 公理的应用）** —— 经 interface 背驰
  （后段力度 C < 前段 A）蕴含后段内在强度 `strength c ≤ strength a`（后段内在不强于前段）。

  这是 interface 公理 `faithful` 的非平凡推论：力度衰减对应内在强度不增——背驰判据不脱离
  走势内在序凭空判断。**用到公理** ⟹ 公理非空洞（若去掉 faithful，此定理不可证）。
-/
theorem divergenceVia_implies_strength_le {α : Type u} (fm : ForceMeasure α) (a c : α)
    (hdiv : IsDivergenceVia fm a c) :
    fm.strength c ≤ fm.strength a := by
  unfold IsDivergenceVia at hdiv
  exact fm.faithful c a hdiv

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 反退化见证：合法 measure 实例 + 伪 measure 被 interface 拒绝

    ★no-声明膨胀核心：interface 公理**非空洞恒真**。本节给两侧见证：
    (A) 一个**满足**公理的合法 ForceMeasure 实例（interface 可被满足，不是不可能的约束）。
    (B) 一个**违反力度序**的伪 measure（内在更强但力度更弱）**无法**构造合法 ForceMeasure
        ——证明 interface 公理真能否决伪 measure（公理有内容，非平凡桩）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **合法力度 measure 见证（满足公理，§4-A）** —— 走势载体取 `Nat`（直接以内在强度为载体），
  `measure n = ⟨n⟩`（力度 = 内在强度，恒等对齐），`strength n = n`。这是最简的合法 measure：
  measure 与内在强度恒等对齐，单调与忠实公理平凡成立（`mono`/`faithful` 由 `le_refl`/同序）。
-/
def identityForceMeasure : ForceMeasure Nat where
  measure n := ⟨n⟩
  strength n := n
  mono := by intro a b h; exact h
  faithful := by intro a b h; exact Nat.le_of_lt h

/-- **★合法 measure 上真背驰见证（C=2 < A=8）** —— interface 判据在合法实例上真跑通。 -/
theorem witness_divergenceVia :
    IsDivergenceVia identityForceMeasure 8 2 := by
  unfold IsDivergenceVia identityForceMeasure; decide

/-- **★合法 measure 上力度延续见证（C=8 ≥ A=3 ⟹ 非背驰）** —— 判据真能否决非背驰。 -/
theorem witness_not_divergenceVia :
    ¬ IsDivergenceVia identityForceMeasure 3 8 := by
  unfold IsDivergenceVia identityForceMeasure; decide

/--
  **★伪 measure 违反力度序（反退化数据，§4-B）** —— 一个候选「力度函数」：内在强度更大的
  走势却被赋更小的力度（强度 0 ↦ 力度 100，强度 1 ↦ 力度 0——内在更强，力度反更弱）。
  这是**违反单调公理**的力度赋值。
-/
def badMeasure : Nat → Force
  | 0 => ⟨100⟩
  | _ => ⟨0⟩

/--
  **★伪 measure 被 interface 拒绝（L0，#124 反退化核心）** —— **不存在**一个合法 ForceMeasure
  实例，其 `measure` 是 `badMeasure` 且 `strength` 是恒等序（`strength n = n`）。

  论证：若存在，则 `mono` 要求 `strength 0 ≤ strength 1 → (badMeasure 0).area ≤ (badMeasure 1).area`，
  即 `0 ≤ 1 → 100 ≤ 0`，但 `100 ≤ 0` 假——矛盾。故 interface 公理**真能否决** badMeasure。

  ★这证明 interface 公理**非空洞恒真**：若公理可被任意 measure 平凡满足，则 badMeasure
  也能构造合法实例；本定理证它不能——公理有内容（单调约束真否决违反力度序的伪 measure）。
  这是 no-声明膨胀的硬见证：interface 不是「任意函数都满足」的空壳。
-/
theorem badMeasure_rejected :
    ¬ ∃ fm : ForceMeasure Nat, fm.measure = badMeasure ∧ (∀ n, fm.strength n = n) := by
  rintro ⟨fm, hmeas, hstr⟩
  -- mono 实例化在 0 ≤ 1：strength 0 = 0 ≤ 1 = strength 1 ⟹ measure 0 ≤ measure 1
  have hle : fm.strength 0 ≤ fm.strength 1 := by rw [hstr 0, hstr 1]; decide
  have hmono := fm.mono 0 1 hle
  -- 但 measure = badMeasure：measure 0 = ⟨100⟩，measure 1 = ⟨0⟩ ⟹ 100 ≤ 0，矛盾
  rw [hmeas] at hmono
  simp only [badMeasure] at hmono
  omega

/--
  **★伪 measure 违反忠实公理的对偶见证（L0）** —— badMeasure 上「力度 0 < 100」
  （measure 1 < measure 0）但内在强度 1 > 0——若被 `faithful` 接受会要求 `strength 1 ≤
  strength 0` 即 `1 ≤ 0`，假。这从 `faithful` 侧再次否决 badMeasure（双公理冗余否决，
  确证公理组非空洞）。
-/
theorem badMeasure_rejected_faithful :
    ¬ ∃ fm : ForceMeasure Nat, fm.measure = badMeasure ∧ (∀ n, fm.strength n = n) := by
  rintro ⟨fm, hmeas, hstr⟩
  -- faithful 实例化在 a=1, b=0：measure 1 = ⟨0⟩ < ⟨100⟩ = measure 0 ⟹ strength 1 ≤ strength 0
  have hlt : (fm.measure 1).area < (fm.measure 0).area := by
    rw [hmeas]; simp only [badMeasure]; decide
  have hfaithful := fm.faithful 1 0 hlt
  rw [hstr 1, hstr 0] at hfaithful
  omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. still-MISSING 诚实声明 + 结果包六要素 + L0/L2 边界精确划线
    ═══════════════════════════════════════════════════════════════════════

  ═══════════════════════════════════════════════════════════════════════
  ★L0/L2 边界精确划线（#124 核心交付）
  ═══════════════════════════════════════════════════════════════════════
  本文件 = 力度 **interface（L0）**。划界如下：

  ┌── L0（本文件实装，纯逻辑，不依赖数据）──────────────────────────────┐
  │ · `ForceMeasure α`：力度 measure 抽象接口（走势载体 → Force + 公理）  │
  │   ——力度比较的逻辑性质（单调 mono / 忠实 faithful）。                 │
  │ · `IsDivergenceVia` / `IsContinuationVia`：背驰判据经 interface，    │
  │   对齐 committed `Divergence.IsDivergence`（`divergenceVia_iff_pair`）。│
  │ · well-definedness：任意满足公理的 ForceMeasure ⟹ 背驰判据互斥穷尽   │
  │   （`divergenceVia_dichotomy`）+ 独立于 measure 计算方式             │
  │   （`divergenceVia_depends_only_on_output`：外延性）。               │
  │ · 反退化：合法 measure 实例（identityForceMeasure）+ 伪 measure 被   │
  │   interface 拒绝（`badMeasure_rejected` / `_faithful`）——公理非空洞。 │
  └──────────────────────────────────────────────────────────────────────┘
  ┌── L2（本文件**不实装**，still-MISSING-C，诚实标）────────────────────┐
  │ · 具体 MACD 面积计算：K 线 → EMA(12)/EMA(26) → DIF → DEA(9) →       │
  │   柱 = 2·(DIF−DEA) → 同向段面积积分 = `ForceMeasure.measure` 的真实  │
  │   填充。这是数值引擎层（浮点/积分/参数），**非力度比较的逻辑性质**。 │
  │ · 本文件 `ForceMeasure.measure` 是抽象函数参数——具体 MACD 实例由    │
  │   外部 L2 引擎提供，本文件不藏 MACD 计算（no-声明膨胀）。            │
  │ · 形式化侧只给 interface + 公理（本文件 L0）；计算侧诚实标 L2 不实装  │
  │   不冒充（formalization-validity-domain：L0 接口不冒充 L2 数值有效）。│
  └──────────────────────────────────────────────────────────────────────┘

  ═══════════════════════════════════════════════════════════════════════
  ★still-MISSING（诚实开口，no-声明膨胀）
  ═══════════════════════════════════════════════════════════════════════
  · **still-MISSING-C（MACD L2 引擎，根因，承接 Divergence.lean）**：无任何 Origin 模块从
    K 线序列计算 MACD 柱子面积。本文件 `ForceMeasure.measure` 抽象，真实 MACD 计算引擎
    （EMA/DIF/DEA + 同向段面积积分）是 L2 数值层，**未实装**——这是形式化主线的下一缺口，
    明确指向 L2 引擎层，不是 L0 定义层。补上 MACD 引擎后，`ForceMeasure` 可由真实力度实例化，
    `divergenceVia_*` 定理直接适用（interface 不需改动，只补 measure 实例）。
  · **内在强度序 `strength` 的几何来源**：本文件用 `strength : α → Nat` 作 measure 必须尊重的
    内在锚（使公理非空洞），但「走势内在强度怎么从几何算」（幅度/动量代理）未在本文件实装
    ——那是走势几何层（与 SegmentFeatureSeq/CenterStates 的几何量对接），本文件设为接口字段。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：力度 measure 抽象接口 `ForceMeasure α`（走势载体 → Force + 单调/忠实公理）+
     背驰判据经 interface（`IsDivergenceVia`，对齐 committed `Divergence.IsDivergence`）+
     well-definedness（任意合法 measure ⟹ 互斥穷尽 + 独立于计算方式）+ 反退化（合法实例 +
     伪 measure 被公理拒绝），全 L0 零 sorry。MACD 计算诚实标 L2 不实装（still-MISSING-C）。
  2. 定义依据：§9 力度比较（力度序）+ 第24课趋势背驰（后段力度弱于前段，MACD 面积 C<A）+
     committed Divergence.IsDivergence（forceC<forceA）。输入特征：背驰判据只读 measure 输出的
     Force 比较 ⟹ 判据满足「独立于 measure 计算」（`divergenceVia_depends_only_on_output`）；
     measure 必须尊重走势内在强度序 ⟹ 公理 mono/faithful 满足「力度序非任意」（反退化否决伪 measure）。
  3. 边界条件（结论翻转）：
     · 背驰判据用严格 `<`（后段力度严格弱于前段），临界相等归延续（对齐 committed Divergence）。
       若改判据为「≤ 算背驰」（盘整背驰宽松口径），临界归属翻转，须重裁——与 committed
       Divergence 边界条件一致。
     · interface 公理选 `mono`（内在强度更大 ⟹ 力度不更小）+ `faithful`（力度严格序 ⟹ 内在序
       可比）。若放松为「measure 是任意函数」（去掉 mono/faithful），则 `badMeasure_rejected`
       失效——伪 measure 可构造合法实例，interface 退化为空壳（声明膨胀）。当前公理保证非空洞。
     · 若 still-MISSING-C 补上 MACD 引擎后，真实 measure 实例在某标的上算出的力度若违反内在
       强度单调（如背驰段 measure 与几何幅度反向），则该实例**不满足 interface 公理**——
       L2 数据可否证「具体 MACD 力度尊重内在强度序」这一假设（formalization-validity-domain：
       L2 否定性结果入口）。本文件 L0 只保证 interface 逻辑自洽，不保证某 L2 实例满足公理。
  4. 下游推论：
     · interface well-defined ⟹ committed Divergence 的 `IsDivergence`（直接消费 Force）可视为
       `IsDivergenceVia identityForceMeasure'`（measure 直取 pair 的 forceA/forceC）的特例——
       背驰判据有统一接口表达，#119 SubLevelDescent 的 `divPair` 显式参数可重表达为
       「外部提供的 ForceMeasure 实例 + 走势对」（接口化力度数据来源）。
     · 力度数据来源接口化 ⟹ L2 MACD 引擎只需实现 `ForceMeasure`（满足公理），即可插入全部
       背驰/买卖点判据（BspClassification 第一类 = 背驰点）——interface 是 L0/L2 的接缝。
     · 卡点明确为 L2 MACD 引擎（still-MISSING-C）⟹ 形式化主线力度侧的 L0 接口已闭合，
       下一步是 L2 数值引擎（不是 L0 定义层）——形式化主线力度侧收尾。
  5. 谱系引用：本文件承接 committed Divergence.lean still-MISSING-C（Force/MACD 抽象标量无 K 线
     计算引擎）+ #119 SubLevelDescent（`divPair` 显式参数留力度来源在外）。无新概念分离——
     `ForceMeasure` 是 committed `Force` 的接口化封装（measure 输出 Force，判据对齐 IsDivergence），
     不引入新定义冲突。力度序公理（mono/faithful）是 §9「力度比较」逻辑结构的显式化，非新概念。
     不确定是否有更早的「力度 measure 接口 vs 计算」概念分离谱系——若 genealogist 有相关记录
     （L0 接口/L2 引擎边界的先例，如 222/223/230 有效域≠定义域三例），本文件 L0/L2 划线与其同模式
     （interface 定义域 = 所有走势载体，有效域 = 满足公理的 measure；MACD L2 有效性是独立的经验问题）。
  6. 影响声明：新增 `Origin.ForceInterface` 模块，import Origin.Divergence（只读，build on committed
     Force/IsDivergence/IsContinuation，不改 committed 类型）。无反向依赖，无命名冲突
     （namespace `NewChanlun.Origin`，新名 `ForceMeasure`/`IsDivergenceVia`/`IsContinuationVia`/
     `identityForceMeasure`/`badMeasure` 与 committed 不碰撞）。
     ★待 Lead 登记 root：`Origin.ForceInterface`（lakefile Origin lib roots 追加）。
     不编辑 lakefile（报 Lead 登记）。`lake env lean Origin/ForceInterface.lean` 单文件验证。
-/

end NewChanlun.Origin
