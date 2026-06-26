/-
  Strict/LevelState.lean — 每级参数化完全分类：R 6 态 + 买卖点 bit-vector + LevelState
  ★cc-levelstate 工位（π_Θ 推导第 3-4 步，RTAS 严格完全分类蜂群）

  ════════════════════════════════════════════════════════════════════════
  本文件的两个独立分类（不可混淆——这是本工位的核心诚实点）
  ════════════════════════════════════════════════════════════════════════

  缠论"每级状态"由两个**结构上不同**的分类拼成，它们的代数性质相反：

  | 分类对象 | 形式 | 互斥穷尽? | 范式 | 依据 |
  |----------|------|-----------|------|------|
  | **R：位置态** 相对最后中枢 Z_ℓ | 6 态 sum type `RLevel` | **是**（Σ𝟙=1） | 互斥三分的状态机精化 | Claim9 三态：within→I；above/below 各按 3B/3S 二分；+⊥ |
  | **E：信号** 买卖点 | bit-vector `BSPVector`={0,1}⁶ | **否**（2/3 类可共存） | 非互斥 subset | BSPLabels twoB_threeB_can_coincide |

  ★★关键区分（编排者明确："买卖点不能强行 6 互斥，2/3 类可共存"）：
  位置态 R 是互斥的（一个点相对一个中枢只能在一个位置），故是 sum type；
  买卖点 E 是非互斥的（一个端点可同时是 2 买与 3 买，V 型反转），故是 bit-vector。
  把 E 也做成 6 互斥类是**错误**——会冒充 BSPLabels 已证否的互斥性
  （`twoB_threeB_can_coincide` / `bsp_not_exclusive_trichotomy`）。bit-vector 是正面形式：
  买卖点是 subset，不是 partition。

  ════════════════════════════════════════════════════════════════════════
  R 6 态 `RLevel = {⊥, I, U⁰, U¹, D⁰, D¹}`（相对最后中枢 Z_ℓ）
  ════════════════════════════════════════════════════════════════════════

  - `bot`（⊥）：无最后中枢（ExistsZ = false）——尚未形成中枢，位置无定义。
  - `inside`（I）：当下在最后中枢核心区间 [ZD, ZG] 内（Claim9 within，闭区间）。
  - `aboveNo3B`（U⁰）：当下在 Z_ℓ 上方（Claim9 above，p > ZG）且**无**第三类买点。
  - `aboveB3`（U¹）：当下在 Z_ℓ 上方且**有**第三类买点（3B 成立，离开中枢回试不入）。
  - `belowNo3S`（D⁰）：当下在 Z_ℓ 下方（Claim9 below，p < ZD）且**无**第三类卖点。
  - `belowS3`（D¹）：当下在 Z_ℓ 下方且**有**第三类卖点（3S 成立）。

  这是 Claim9 中枢位置三态（below/within/above）的**状态机精化**（非统一乘积）：
  - within 直接 ⟶ I（**不**按第三类拆分）；
  - above 按第三类买点有/无裂为 U⁰/U¹（2 态）；
  - below 按第三类卖点有/无裂为 D⁰/D¹（2 态）；
  - 再并入 ⊥（无中枢）。计数：1(within) + 2(above) + 2(below) + 1(⊥) = 6 态。
  ★注意非 3×2+1：within 不拆，只有 above/below 各按 Bool 二分，故是「逐态精化」非「全三态统一乘 Bool」。

  ════════════════════════════════════════════════════════════════════════
  诚实标注（gatekeeper）
  ════════════════════════════════════════════════════════════════════════

  ★标签：**StructurePartitionOnly**（R 6 态是结构事实——给定最后中枢与第三类事件后，
        位置态的 fiber 划分是 Θ-参数化全函数的原像划分，**非**语义双射；
        E bit-vector 是 subset 编码，非分类）。

  ★Θ-参数化运行分类（codex 研究结论，ClassificationFamily.lean 同构）：
  R 6 态**不**由缠论结构公理单独导出——它依赖两个 Θ 参数：
  - **Θ_parse**：「最后中枢 Z_ℓ」的定义（哪个中枢是"最后"、边界 ZD/ZG 在哪——
    `Center` 已固化 ZD/ZG，但"最后中枢"的选取是 Θ_parse 的 canonical 选择器）。
  - **Θ_signal**：第三类买点/卖点事件判据（3B/3S 在哪触发——离开中枢回试不入的阈值）。
  缠论公理给「中枢位置三态」（Claim9，无参数 L0），但「相对**最后**中枢 + 带**第三类信号**
  的 6 态」需要 Θ_parse + Θ_signal。给定 (Z_ℓ, B₃判定, S₃判定) 后，`rlevelOf` 是全函数，
  其 6 态划分自动互斥穷尽（`rlevel_exhaustive_exclusive`）。**给定** Θ 后 L0；Θ 本身非缠论可导。

  认识论等级：全部 **L0**（给定 Θ 后从 Claim9 已结算定义 + Bool 穷尽推导，不依赖经验数据）。
  formalization-validity-domain：Lean build 通过 = 给定 Θ 后逻辑正确（L0），
  **不**膨胀为「6 态是缠论无参数完全分类」（前件是 Θ）。

  ★依赖方向（单向 Strict→{Formal, Phase2}，无环）：
  本文件 import `Classification`（内核）+ `Claim9_CenterPosition`（位置三态）
  + `Formal.BSPLabels`（买卖点标签）。三者均**不** import Strict。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1⊊Layer2 概念分离）
       → 616（完全分类⊬唯一策略）→ 617（C 与 π 都 Θ-参数化）。
       Claim9（中枢位置三态）+ BSPLabels（买卖点非互斥标签集，twoB_threeB_can_coincide）。
-/

import Classification          -- Strict 内核（命名空间 = Strict）：Classifies / SemanticQuotient
import Claim9_CenterPosition    -- 分类B：中枢位置三态（命名空间 = Formal.CenterPosition）
import Formal.BSPLabels         -- 买卖点标签集（命名空间 = Formal.BSPLabels）

namespace Strict.LevelState

open Formal.CenterTrichotomy (Center)
open Formal.CenterPosition
open Formal.BSPLabels

/-! ════════════════════════════════════════════════════════════════════════
    § Part 1 — R 6 态：相对最后中枢 Z_ℓ 的位置状态机（互斥穷尽）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★R 6 态（相对最后中枢 Z_ℓ 的位置态，互斥 sum type）。

  Claim9 中枢位置三态（below/within/above）的状态机精化：
  - above 按第三类买点有/无 ⟶ U⁰(aboveNo3B) / U¹(aboveB3)；
  - below 按第三类卖点有/无 ⟶ D⁰(belowNo3S) / D¹(belowS3)；
  - within ⟶ I(inside)；并入 ⊥(bot, 无中枢)。

  ★互斥性来源（非态计数）：位置（Claim9 三态互斥）、第三类事件（确定 Bool）、
  中枢存在性（确定 some/none）——任两不同态的判据在某一维度冲突，故 6 态互斥
  （见 `rlevel_exhaustive_exclusive`）。
-/
inductive RLevel where
  | bot         -- ⊥：无最后中枢（ExistsZ = false）
  | inside      -- I：当下在 Z_ℓ 核心区间 [ZD, ZG] 内（Claim9 within）
  | aboveNo3B   -- U⁰：Z_ℓ 上方（p > ZG）且无第三类买点
  | aboveB3     -- U¹：Z_ℓ 上方且有第三类买点（3B 成立）
  | belowNo3S   -- D⁰：Z_ℓ 下方（p < ZD）且无第三类卖点
  | belowS3     -- D¹：Z_ℓ 下方且有第三类卖点（3S 成立）
deriving DecidableEq, Repr

open RLevel

/--
  ★最后中枢上下文（Θ-参数化运行输入，L0）。

  R 6 态的判定不只看价格点——它需要三个 Θ-参数化输入：
  - `lastCenter : Option Center`：最后中枢 Z_ℓ（`none` = 无中枢 ⟶ ⊥；`some c` = 有中枢 c）。
    「哪个中枢是最后」是 Θ_parse 选择器的产物（缠论不无参数地钉死"最后"）。
  - `price : Int`：当下价格点 p（相对 Z_ℓ 核心区间定位）。
  - `b3 : Bool`：第三类买点事件是否成立（Θ_signal 判据：离开中枢上破后回试不入）。
  - `s3 : Bool`：第三类卖点事件是否成立（Θ_signal 判据：离开中枢下破后回抽不入）。

  ★这是 `ClassificationFamily.ClassifierFamily` 的 `X`（运行对象）在本级位置态上的实例：
  给定 Θ（= 如何选最后中枢 + 如何判第三类信号）后，从 `RContext` 到 `RLevel` 是全函数。
-/
structure RContext where
  lastCenter : Option Center
  price : Int
  b3 : Bool
  s3 : Bool

namespace RContext

/-- 是否存在最后中枢（⊥ 态的判据：无中枢 ⟶ ⊥）。 -/
def ExistsZ (ctx : RContext) : Prop := ctx.lastCenter.isSome = true

instance (ctx : RContext) : Decidable ctx.ExistsZ := by
  unfold ExistsZ; infer_instance

end RContext

open RContext

/--
  ★R 6 态判定函数（给定 Θ 后的全函数 C_Θ，L0）。

  `rlevelOf ctx` 把运行上下文映射到 6 态之一：
  - 无中枢 ⟶ `bot`；
  - 有中枢 c 时，按 Claim9 `classify c p` 三态分流：
    · within ⟶ `inside`；
    · above ⟶ `b3` 真则 `aboveB3` 否则 `aboveNo3B`；
    · below ⟶ `s3` 真则 `belowS3` 否则 `belowNo3S`。

  ★全函数性：对每个 `ctx` 返回确定的 `RLevel`（Lean 全函数，编码"完全分类每级位置都有态"）。
-/
def rlevelOf (ctx : RContext) : RLevel :=
  match ctx.lastCenter with
  | none => bot
  | some c =>
    match Formal.CenterPosition.classify c ctx.price with
    | RelativePosition.within => inside
    | RelativePosition.above => if ctx.b3 then aboveB3 else aboveNo3B
    | RelativePosition.below => if ctx.s3 then belowS3 else belowNo3S

/-! ### 6 态谓词（显式刻画每态的判据，供互斥穷尽定理引用） -/

/-- ⊥ 谓词：无最后中枢。 -/
def IsBot (ctx : RContext) : Prop := ctx.lastCenter = none
/-- I 谓词：有中枢且当下在核心区间内（Claim9 within）。 -/
def IsInside (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsWithin c ctx.price
/-- U⁰ 谓词：有中枢、当下在上方、无第三类买点。 -/
def IsAboveNo3B (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsAbove c ctx.price ∧ ctx.b3 = false
/-- U¹ 谓词：有中枢、当下在上方、有第三类买点。 -/
def IsAboveB3 (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsAbove c ctx.price ∧ ctx.b3 = true
/-- D⁰ 谓词：有中枢、当下在下方、无第三类卖点。 -/
def IsBelowNo3S (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsBelow c ctx.price ∧ ctx.s3 = false
/-- D¹ 谓词：有中枢、当下在下方、有第三类卖点。 -/
def IsBelowS3 (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsBelow c ctx.price ∧ ctx.s3 = true

/-! ### rlevelOf 忠实于 6 态谓词（每态判据与判定函数一致） -/

/-- 辅助：classify 三态忠实（Claim9 已证，这里桥接到本文件谓词）。 -/
private theorem classify_within_iff (c : Center) (p : Int) :
    Formal.CenterPosition.classify c p = RelativePosition.within
      ↔ Formal.CenterPosition.IsWithin c p :=
  Formal.CenterPosition.classify_eq_within c p

private theorem classify_above_iff (c : Center) (p : Int) :
    Formal.CenterPosition.classify c p = RelativePosition.above
      ↔ Formal.CenterPosition.IsAbove c p :=
  Formal.CenterPosition.classify_eq_above c p

private theorem classify_below_iff (c : Center) (p : Int) :
    Formal.CenterPosition.classify c p = RelativePosition.below
      ↔ Formal.CenterPosition.IsBelow c p :=
  Formal.CenterPosition.classify_eq_below c p

/-- `rlevelOf = bot ↔ IsBot`。 -/
theorem rlevelOf_eq_bot (ctx : RContext) : rlevelOf ctx = bot ↔ IsBot ctx := by
  unfold rlevelOf IsBot
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = inside ↔ IsInside`。 -/
theorem rlevelOf_eq_inside (ctx : RContext) : rlevelOf ctx = inside ↔ IsInside ctx := by
  unfold rlevelOf IsInside
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_within_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = aboveB3 ↔ IsAboveB3`。 -/
theorem rlevelOf_eq_aboveB3 (ctx : RContext) : rlevelOf ctx = aboveB3 ↔ IsAboveB3 ctx := by
  unfold rlevelOf IsAboveB3
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_above_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = aboveNo3B ↔ IsAboveNo3B`。 -/
theorem rlevelOf_eq_aboveNo3B (ctx : RContext) :
    rlevelOf ctx = aboveNo3B ↔ IsAboveNo3B ctx := by
  unfold rlevelOf IsAboveNo3B
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_above_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = belowS3 ↔ IsBelowS3`。 -/
theorem rlevelOf_eq_belowS3 (ctx : RContext) : rlevelOf ctx = belowS3 ↔ IsBelowS3 ctx := by
  unfold rlevelOf IsBelowS3
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_below_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = belowNo3S ↔ IsBelowNo3S`。 -/
theorem rlevelOf_eq_belowNo3S (ctx : RContext) :
    rlevelOf ctx = belowNo3S ↔ IsBelowNo3S ctx := by
  unfold rlevelOf IsBelowNo3S
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_below_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-! ### ★核心定理：R 6 态互斥穷尽（Σ𝟙 = 1） -/

/--
  ★R 6 态构造子穷尽（L0）：`rlevelOf` 全函数对任意上下文返回 6 态之一。

  「相对最后中枢的位置态无非这 6 种」的机器证明——rlevelOf 是全函数，
  值域恰为 {bot, inside, aboveNo3B, aboveB3, belowNo3S, belowS3}。
-/
theorem rlevel_constructor_exhaustive (ctx : RContext) :
    rlevelOf ctx = bot ∨ rlevelOf ctx = inside
    ∨ rlevelOf ctx = aboveNo3B ∨ rlevelOf ctx = aboveB3
    ∨ rlevelOf ctx = belowNo3S ∨ rlevelOf ctx = belowS3 := by
  unfold rlevelOf
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inl rfl)
    · split
      · exact Or.inr (Or.inr (Or.inr (Or.inl rfl)))
      · exact Or.inr (Or.inr (Or.inl rfl))
    · split
      · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl))))
      · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))

/--
  ★6 态谓词穷尽（totality，L0）：任意上下文必满足 6 态谓词之一。

  位置三态穷尽（Claim9 `position_trichotomy`）× Bool 二值穷尽（b3/s3）+ 中枢存在性二值
  ⟹ 6 态谓词覆盖全部上下文。
-/
theorem rlevel_predicates_total (ctx : RContext) :
    IsBot ctx ∨ IsInside ctx
    ∨ IsAboveNo3B ctx ∨ IsAboveB3 ctx
    ∨ IsBelowNo3S ctx ∨ IsBelowS3 ctx := by
  -- 由构造子穷尽 + 每态忠实引理桥接
  rcases rlevel_constructor_exhaustive ctx with
    h | h | h | h | h | h
  · exact Or.inl ((rlevelOf_eq_bot ctx).mp h)
  · exact Or.inr (Or.inl ((rlevelOf_eq_inside ctx).mp h))
  · exact Or.inr (Or.inr (Or.inl ((rlevelOf_eq_aboveNo3B ctx).mp h)))
  · exact Or.inr (Or.inr (Or.inr (Or.inl ((rlevelOf_eq_aboveB3 ctx).mp h))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ((rlevelOf_eq_belowNo3S ctx).mp h)))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr ((rlevelOf_eq_belowS3 ctx).mp h)))))

/--
  ★6 态两两互斥（pairwise disjoint，L0）：任意上下文最多满足一个态谓词。

  由 `rlevelOf` 是全函数（每态 iff `rlevelOf = 该构造子`）+ 构造子 `DecidableEq` 两两不等
  ⟹ 互斥。这里直接由 rlevelOf 等价桥接：若同时满足两态，则 rlevelOf 等于两个不同构造子，矛盾。
-/
theorem rlevel_pairwise_disjoint (ctx : RContext) :
    (IsBot ctx → ¬ IsInside ctx ∧ ¬ IsAboveNo3B ctx ∧ ¬ IsAboveB3 ctx
        ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsInside ctx → ¬ IsAboveNo3B ctx ∧ ¬ IsAboveB3 ctx
        ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsAboveNo3B ctx → ¬ IsAboveB3 ctx ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsAboveB3 ctx → ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsBelowNo3S ctx → ¬ IsBelowS3 ctx) := by
  -- 把每个谓词换成 `rlevelOf = 构造子`，再用构造子互不相等（rlevelOf ctx 是固定项）
  rw [← rlevelOf_eq_bot, ← rlevelOf_eq_inside, ← rlevelOf_eq_aboveNo3B,
      ← rlevelOf_eq_aboveB3, ← rlevelOf_eq_belowNo3S, ← rlevelOf_eq_belowS3]
  refine ⟨fun h => ?_, fun h => ?_, fun h => ?_, fun h => ?_, fun h => ?_⟩ <;>
    rw [h] <;>
    simp only [reduceCtorEq, not_false_eq_true, and_self]

/--
  ★★核心定理 `rlevel_exhaustive_exclusive`（Σ𝟙 = 1，L0）。

  R 6 态是 `RContext` 的**真划分**（partition）：任意上下文**恰好**满足一个态谓词。
  形式：穷尽（`rlevel_predicates_total`）+ 两两互斥（`rlevel_pairwise_disjoint`）的合取——
  即标准第一部分B #1「Σ 𝟙_{C_r} = 1」（6 个指示函数之和恒为 1）。

  ★这是 Claim9 中枢位置三态 `position_exactly_one`（below/within/above Σ𝟙=1）的
  **状态机精化**（逐态精化，非统一乘积）：within→inside（不拆）；above/below 各按
  3B/3S 二分；+⊥ ⟹ 1+2+2+1 = 6 态 Σ𝟙=1。
  互斥性来源（与态计数无关）：同一 ctx 下位置互斥（Claim9 三态互斥）、b3/s3 是确定 Bool、
  lastCenter 是确定 some/none——任两不同态的判据在某一维度直接冲突，故 6 态严格互斥穷尽。
-/
theorem rlevel_exhaustive_exclusive (ctx : RContext) :
    (IsBot ctx ∨ IsInside ctx ∨ IsAboveNo3B ctx ∨ IsAboveB3 ctx
      ∨ IsBelowNo3S ctx ∨ IsBelowS3 ctx)
    ∧ (IsBot ctx → ¬ IsInside ctx ∧ ¬ IsAboveNo3B ctx ∧ ¬ IsAboveB3 ctx
        ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsInside ctx → ¬ IsAboveNo3B ctx ∧ ¬ IsAboveB3 ctx
        ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsAboveNo3B ctx → ¬ IsAboveB3 ctx ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsAboveB3 ctx → ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsBelowNo3S ctx → ¬ IsBelowS3 ctx) :=
  ⟨rlevel_predicates_total ctx,
   (rlevel_pairwise_disjoint ctx).1,
   (rlevel_pairwise_disjoint ctx).2.1,
   (rlevel_pairwise_disjoint ctx).2.2.1,
   (rlevel_pairwise_disjoint ctx).2.2.2.1,
   (rlevel_pairwise_disjoint ctx).2.2.2.2⟩

/-! ════════════════════════════════════════════════════════════════════════
    § Part 2 — 买卖点 bit-vector：{0,1}⁶（非互斥，2/3 类可共存）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★买卖点 bit-vector（b₁,b₂,b₃,s₁,s₂,s₃，{0,1}⁶，L0）。

  六个独立布尔位——三类买点（b₁/b₂/b₃）+ 三类卖点（s₁/s₂/s₃）。
  ★★**非互斥**（编排者明确"不能强行 6 互斥，2/3 类可共存"）：
  bit-vector 容许多位同时为 1（如 b₂=b₃=1 = V 型反转 2 买/3 买重合），这是
  `BSPLabels.twoB_threeB_can_coincide` 的**正面形式**——买卖点是 subset/bit-vector，
  **不是**互斥 sum type 分类（`bsp_not_exclusive_trichotomy` 已证后者失败）。

  与 R 6 态（`RLevel` sum type，互斥）的存在论区别：位置态是 partition（恰好一格），
  信号是 subset（任意位组合）——两者代数性质相反，不可混淆。
-/
structure BSPVector where
  b1 : Bool   -- 第一类买点
  b2 : Bool   -- 第二类买点
  b3 : Bool   -- 第三类买点
  s1 : Bool   -- 第一类卖点
  s2 : Bool   -- 第二类卖点
  s3 : Bool   -- 第三类卖点
deriving DecidableEq, Repr

namespace BSPVector

/-- 全零向量（无任何买卖点信号——非运动端点）。 -/
def empty : BSPVector := ⟨false, false, false, false, false, false⟩

/-- 是否非空（至少一位为 1，即至少持有一个买卖点）。 -/
def NonEmpty (v : BSPVector) : Prop :=
  v.b1 = true ∨ v.b2 = true ∨ v.b3 = true
  ∨ v.s1 = true ∨ v.s2 = true ∨ v.s3 = true

instance (v : BSPVector) : Decidable (NonEmpty v) := by
  unfold NonEmpty; infer_instance

end BSPVector

open BSPVector

/--
  ★bit-vector 非空 = 升跌完备性（010，L0）的本级编码。

  `bspvector_wellformed`：一个端点持有的买卖点向量**非空** ⟺ 它确实是某类买卖点
  （升跌完备性 010：任何向上下跌从三类买卖点某一类开始/结束）。
  这里给出充要刻画：`NonEmpty v ↔ v ≠ empty`（非空 = 不是全零向量）。

  ★与 BSPLabels.bsp_totality（标签集 nonempty 字段）对应：标签集非空 ⟺ bit-vector 非空。
-/
theorem bspvector_wellformed (v : BSPVector) : v.NonEmpty ↔ v ≠ BSPVector.empty := by
  unfold BSPVector.NonEmpty BSPVector.empty
  cases v with
  | mk b1 b2 b3 s1 s2 s3 =>
    cases b1 <;> cases b2 <;> cases b3 <;> cases s1 <;> cases s2 <;> cases s3 <;>
      simp

/--
  ★bit-vector 容许 2/3 类共存（非互斥正面形式，L0）。

  存在一个非空向量同时 b₂=true 且 b₃=true（V 型反转 2 买/3 买重合）——
  这是 `BSPLabels.twoB_threeB_can_coincide` 在 bit-vector 模型的对应：
  bit-vector **不**强制互斥，2/3 类可同时为 1（若做成互斥 sum type 则此点无像）。
-/
def vReversalVector : BSPVector := ⟨false, true, true, false, false, false⟩

theorem bspvector_allows_2B3B_coincide :
    vReversalVector.b2 = true ∧ vReversalVector.b3 = true ∧ vReversalVector.NonEmpty := by
  refine ⟨rfl, rfl, ?_⟩
  unfold BSPVector.NonEmpty vReversalVector
  decide

/-! ### bit-vector ↔ BSPLabels 标签集对接 -/

/--
  ★bit-vector → 标签列表（给定级别 lvl，L0）。

  把六个位翻译为 `BSPLabel` 列表——每个为 true 的位生成对应 (lvl, type, side) 标签。
  这是 bit-vector 与 `BSPLabels.BSPLabelSet.labels` 的桥（subset 编码 ↔ 标签集）。
-/
def toLabels (v : BSPVector) (lvl : Nat) : List BSPLabel :=
  (if v.b1 then [⟨lvl, BSPType.type1, Side.buy⟩] else [])
  ++ (if v.b2 then [⟨lvl, BSPType.type2, Side.buy⟩] else [])
  ++ (if v.b3 then [⟨lvl, BSPType.type3, Side.buy⟩] else [])
  ++ (if v.s1 then [⟨lvl, BSPType.type1, Side.sell⟩] else [])
  ++ (if v.s2 then [⟨lvl, BSPType.type2, Side.sell⟩] else [])
  ++ (if v.s3 then [⟨lvl, BSPType.type3, Side.sell⟩] else [])

/--
  ★对接定理：bit-vector 非空 ⟺ 标签列表非空（升跌完备性的两模型等价，L0）。

  `NonEmpty v ↔ toLabels v lvl ≠ []`——bit-vector 模型的「非空」与 BSPLabels
  标签集模型的「labels ≠ []」（`bsp_totality`）逐位对应。这是两个买卖点模型
  （bit-vector subset / 标签集 List）在 totality 上的等价对接。
-/
theorem toLabels_nonempty_iff (v : BSPVector) (lvl : Nat) :
    v.NonEmpty ↔ toLabels v lvl ≠ [] := by
  unfold BSPVector.NonEmpty toLabels
  cases v with
  | mk b1 b2 b3 s1 s2 s3 =>
    cases b1 <;> cases b2 <;> cases b3 <;> cases s1 <;> cases s2 <;> cases s3 <;>
      simp

/--
  ★对接定理：bit-vector 含某买点位 ⟺ 标签列表含对应标签（L0）。

  以 b₃（第三类买点）为例：`v.b3 = true ↔ (lvl,type3,buy) ∈ toLabels v lvl`。
  确认 bit-vector 与标签集在「持有哪些买卖点」上语义一致（subset 成员 ↔ 标签成员）。
-/
theorem toLabels_mem_b3 (v : BSPVector) (lvl : Nat) :
    v.b3 = true ↔ (⟨lvl, BSPType.type3, Side.buy⟩ : BSPLabel) ∈ toLabels v lvl := by
  unfold toLabels
  cases v with
  | mk b1 b2 b3 s1 s2 s3 =>
    cases b1 <;> cases b2 <;> cases b3 <;> cases s1 <;> cases s2 <;> cases s3 <;>
      simp

/-! ════════════════════════════════════════════════════════════════════════
    § Part 3 — 每级完整状态 LevelState = (D_ℓ, R_ℓ, E_ℓ)
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★每级完整状态（L0）。

  缠论"某一级别 ℓ 的当下状态"由三个分量构成：
  - `decomp : Decomp`：解析/分解 D_ℓ（该级递归分解的当下规范结果，类型参数 `Decomp`
    由下游 Θ_parse 给出——本文件参数化，不内联 `DecompositionSystem`，避免重复定义）。
  - `position : RLevel`：位置态 R_ℓ（相对最后中枢 Z_ℓ 的 6 态之一，互斥穷尽）。
  - `signal : BSPVector`：买卖点信号 E_ℓ（bit-vector，非互斥 subset）。

  ★分量的代数性质不同（本文件核心诚实点）：
  - R_ℓ（`RLevel`）是 partition（`rlevel_exhaustive_exclusive`：Σ𝟙=1）；
  - E_ℓ（`BSPVector`）是 subset（`bspvector_allows_2B3B_coincide`：可共存）；
  把 E 也做成互斥类是错误（冒充 BSPLabels 已证否的互斥性）。

  Θ-参数化：`decomp` 的取值依赖 Θ_parse，`position` 依赖 Θ_parse(最后中枢)+Θ_signal(3B/3S)，
  `signal` 依赖 Θ_signal(各类买卖点判据)。三分量均 Θ-参数化运行分类（StructurePartitionOnly）。
-/
structure LevelState (Decomp : Type) where
  decomp : Decomp
  position : RLevel
  signal : BSPVector

namespace LevelState

/--
  ★每级状态的位置分量互斥穷尽（L0，由 Part 1 继承）。

  对任意从 `RContext` 构造的位置态，6 态恰好其一——`LevelState.position` 字段
  **完整继承** `rlevel_exhaustive_exclusive` 的划分性质（穷尽 + 全部 15 对两两互斥），
  名实相符（不缩减为部分互斥，避免声明膨胀）。
-/
theorem position_partition (ctx : RContext) :
    (IsBot ctx ∨ IsInside ctx ∨ IsAboveNo3B ctx ∨ IsAboveB3 ctx
      ∨ IsBelowNo3S ctx ∨ IsBelowS3 ctx)
    ∧ (IsBot ctx → ¬ IsInside ctx ∧ ¬ IsAboveNo3B ctx ∧ ¬ IsAboveB3 ctx
        ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsInside ctx → ¬ IsAboveNo3B ctx ∧ ¬ IsAboveB3 ctx
        ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsAboveNo3B ctx → ¬ IsAboveB3 ctx ∧ ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsAboveB3 ctx → ¬ IsBelowNo3S ctx ∧ ¬ IsBelowS3 ctx)
    ∧ (IsBelowNo3S ctx → ¬ IsBelowS3 ctx) :=
  rlevel_exhaustive_exclusive ctx

/--
  ★每级状态的信号分量非互斥（L0，由 Part 2 继承）。

  `LevelState.signal` 字段是 bit-vector，容许 2/3 类共存——与 position 字段
  的互斥性形成对照（partition vs subset）。
-/
theorem signal_non_exclusive :
    ∃ v : BSPVector, v.b2 = true ∧ v.b3 = true ∧ v.NonEmpty :=
  ⟨vReversalVector, bspvector_allows_2B3B_coincide⟩

end LevelState

end Strict.LevelState
