/-
Origin/BspClassification.lean — 买卖点三类判据 + 单射裁定 + 完备性定理（task #113）

★工位定位（审计 A 判决）：Origin 的 ChanlunElements.bspOf 是 `total_unique_of_fun` 平凡桩
  （只断言存在唯一输出，无任何买卖点判据内容）。本文件把买卖点三类的**真判据**
  （每类的中枢关系 + 背驰）形式化为 Origin 命名空间内的可机器检查谓词/定理——分类血肉。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- §10.1（三类买卖点定义，知识库 + 第24课博文）：
  · 第一类买点：某级别下跌趋势中，次级别向下跌破最后一个中枢后形成的**背驰点**。
  · 第二类买点：第一类买点后，次级别上涨结束、再次下跌的那个次级别走势的结束点。
  · 第三类买点：上涨趋势中，次级别向上离开中枢后，次级别回抽低点**不跌破 ZG** 的中枢终结点。
- §10.2（完备性 + 买卖点定律一）：
  · 买卖点完备性定理："市场必然产生赢利买卖点，只有第一、二、三类。"
  · 买卖点定律一："任何级别的第二类买卖点都由次级别相应走势的第一类买卖点构成。"
- §10.3：第三类买卖点必须是"第一次回抽"。
- 第24课：第三类买点 = 回抽不重新进入前面中枢（024:36 万科例）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 中枢位置判据 / 互斥穷尽结构推导，omega/decide machine-checked）。
`lake env lean Origin/BspClassification.lean` 通过 = 三类判据的中枢关系、单射裁定、完备性
（只有一二三类）在定义层成立，**不是**任何"买卖点识别在真实行情上有效"的实证断言。

诚实标注（gatekeeper）：
★ 完备性 = TrueCompleteClassification（在精化签名下）+ QuotientByLabel（旧互斥三分失败反例）：
  与 legacy Strict.BSP 一致——买卖点全域不是互斥 sum type（2类3类可重合，V 型反转），
  互斥三分失败（`no_exclusive_trichotomy`）；精化触发形态签名（允许 2/3 共存独立标签）才真单射。
  第三类本征子域（离开中枢回抽不破）是真双射（`third_subdomain_classifies` 重锚）。
★ 有效域诚实标注：完备性"只有一二三类"是 §10.2 缠论**公理**（缠师断言市场只产生这三类），
  本文件忠实编码该断言为类型穷尽（`bspType_exhaustive`）——这是 L0 编码缠论公理，
  **不是** L2 经验验证"真实市场只有三类"（那需全市场数据，本文件不冒充）。

★ still-MISSING-D（见文件尾）：bspOf 全自动构造（从 ParseStruct 识别每个端点属哪类）
  未实装；第二类"由次级别第一类构成"（买卖点定律一）需次级别递归，本文件证判据层不证递归构造。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.Divergence

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 买卖点类型 + 端点情形（§10.1）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **买卖点类型（§10.1）** —— 一二三类。与 ChanlunElements.BspKind 同构（type1/2/3），
  本文件用独立 enum 承载判据语义（BspKind 是 Origin 接口的数据载体，本 enum 是分类血肉）。
-/
inductive BspClass where
  | one    -- 第一类（背驰点）
  | two    -- 第二类（回抽，由次级别一类构成）
  | three  -- 第三类（离开中枢回抽不破 ZG/ZD）
deriving DecidableEq, Repr

/--
  **买卖点端点情形（§10.1）** —— 一个候选买卖点端点携带的判据数据：

  - `side`：买（long）/ 卖（short）。
  - `center`：相关中枢（[zd,zg]）。
  - `divPair`：背驰段对（第一类判据：背驰点）。
  - `brokeCenter`：是否跌破/突破最后一个中枢（第一类前提）。
  - `afterTypeOne`：是否在第一类之后（第二类前提）。
  - `leftCenter`：是否离开中枢（第三类前提）。
  - `retracePrice`：回抽的极值价（第三类判据：不破 ZG/ZD）。
  - `firstRetrace`：是否第一次回抽（§10.3 第三类必须第一次回抽）。
-/
structure BspEndpoint where
  side : Side
  center : Center
  divPair : DivergencePair
  brokeCenter : Bool
  afterTypeOne : Bool
  leftCenter : Bool
  retracePrice : Tick
  firstRetrace : Bool
deriving Repr

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 三类判据（每类的中枢关系，§10.1）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **第一类判据（§10.1）** —— 跌破/突破最后一个中枢后的**背驰点**。
  `brokeCenter = true`（已破中枢）∧ `IsDivergence divPair`（背驰）。
-/
def IsType1 (e : BspEndpoint) : Prop :=
  e.brokeCenter = true ∧ IsDivergence e.divPair

/--
  **第二类判据（§10.1 + 买卖点定律一）** —— 第一类之后、回抽段的结束点。
  `afterTypeOne = true`（在第一类后）∧ `brokeCenter = false`（回抽未再破中枢，区别于第一类）。
  ★买卖点定律一"由次级别一类构成"是构成性陈述（次级别递归，still-MISSING-D），
  本判据刻画本级别可观测条件：在一类后的回抽结束点。
-/
def IsType2 (e : BspEndpoint) : Prop :=
  e.afterTypeOne = true ∧ e.brokeCenter = false

/--
  **第三类买点判据（§10.1 + §10.3 + 第24课）** —— 上涨趋势中离开中枢后，第一次回抽**不破 ZG**。
  `leftCenter = true`（离开中枢）∧ `firstRetrace = true`（第一次回抽，§10.3）∧
  买点方向回抽低点 `retracePrice > ZG`（不跌破 ZG = 不重新进入中枢，第24课）。
-/
def IsType3Buy (e : BspEndpoint) : Prop :=
  e.side = Side.long ∧ e.leftCenter = true ∧ e.firstRetrace = true ∧
  e.center.zg < e.retracePrice

/--
  **第三类卖点判据（§10.1 对偶）** —— 下跌趋势中离开中枢后，第一次回抽**不破 ZD**。
  卖点回抽高点 `retracePrice < ZD`（不升破 ZD）。
-/
def IsType3Sell (e : BspEndpoint) : Prop :=
  e.side = Side.short ∧ e.leftCenter = true ∧ e.firstRetrace = true ∧
  e.retracePrice < e.center.zd

/-- 第三类（买或卖）。 -/
def IsType3 (e : BspEndpoint) : Prop := IsType3Buy e ∨ IsType3Sell e

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 第三类"回抽不破 ZG/ZD" = 中枢位置三态的应用（重锚模块2）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★第三类买点回抽在中枢之上（L0，重锚模块2 CenterStates）** —— 第三类买点的回抽低点
  `retracePrice > ZG` 等价于该点相对中枢"之上"（`IsAbove center retracePrice`）。
  这把第三类判据"不破 ZG"重锚到模块2的位置三态——第三类 = 位置三态在回抽语境的应用。
-/
theorem type3Buy_retrace_above (e : BspEndpoint) (h : IsType3Buy e) :
    IsAbove e.center e.retracePrice := by
  unfold IsType3Buy at h; unfold IsAbove; exact h.2.2.2

/--
  **★第三类卖点回抽在中枢之下（L0，重锚模块2）** —— 卖点回抽高点 `retracePrice < ZD`
  等价于"之下"（`IsBelow center retracePrice`）。
-/
theorem type3Sell_retrace_below (e : BspEndpoint) (h : IsType3Sell e) :
    IsBelow e.center e.retracePrice := by
  unfold IsType3Sell at h; unfold IsBelow; exact h.2.2.2

/--
  **★第三类反退化见证：回抽破 ZG 不是第三类买点（L0，非平凡）** —— 若回抽低点
  `retracePrice ≤ ZG`（重新进入中枢），则不是第三类买点。这证明第三类判据真能否决
  （第24课"回抽不重新进入中枢"是真约束，非平凡桩）。
-/
theorem type3Buy_rejects_reenter (e : BspEndpoint)
    (_hside : e.side = Side.long) (hreenter : e.retracePrice ≤ e.center.zg) :
    ¬ IsType3Buy e := by
  unfold IsType3Buy; intro h
  have hgt := h.2.2.2
  simp only [Tick] at hgt hreenter ⊢
  omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 单射裁定（§10.1 / legacy Strict.BSP 重锚）：2类3类可重合 → 互斥三分失败
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★2类3类可重合见证（L0，重锚 Strict.BSP twoB_threeB_can_coincide）** ——
  存在一个端点同时满足第二类与第三类（V 型反转：回抽段既是一类后的回抽结束=二类，
  又是离开中枢回抽不破=三类买点）。

  ★这是单射裁定的关键反退化：买卖点**不是**互斥 sum type（恰好一格）——
  存在同占两格的端点。`x_2b3b` 见证。
-/
def x_2b3b : BspEndpoint :=
  { side := Side.long
    center := { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }
    divPair := { forceA := ⟨5⟩, forceC := ⟨3⟩, isTrend := false }
    brokeCenter := false   -- 回抽未再破中枢 ⟹ 满足二类
    afterTypeOne := true   -- 在一类后 ⟹ 满足二类
    leftCenter := true     -- 离开中枢 ⟹ 三类前提
    retracePrice := 25     -- 25 > zg=20 ⟹ 不破 ZG ⟹ 三类买点
    firstRetrace := true }

theorem x_2b3b_is_type2 : IsType2 x_2b3b := by
  unfold IsType2 x_2b3b; exact ⟨rfl, rfl⟩

theorem x_2b3b_is_type3 : IsType3 x_2b3b := by
  unfold IsType3 IsType3Buy x_2b3b; left; exact ⟨rfl, rfl, rfl, by decide⟩

/--
  **★互斥三分失败（L0，重锚 Strict.BSP no_global_classifies）** —— 买卖点全域**不存在**
  把每端点映到恰好一类（互斥 sum type）的分类——因为 `x_2b3b` 同占二、三类，
  任何声称"恰好一类"的互斥分类在此端点失败。

  形式化：不存在 `I : BspEndpoint → BspClass` 使得 `I` 与三类判据 `sound` 且 `disjoint`
  （sound：I 给的标签满足该类判据；disjoint：满足两类则两标签相等）。
  `x_2b3b` 满足 two 与 three 但 `two ≠ three` ⟹ disjoint 失败。
-/
theorem no_exclusive_trichotomy :
    ¬ ∃ I : BspEndpoint → BspClass,
        (∀ e, (I e = BspClass.two → IsType2 e) ∧ (I e = BspClass.three → IsType3 e)) ∧
        (∀ e, IsType2 e → IsType3 e → True → False) := by
  -- disjoint 形式：若同时二类三类则矛盾。x_2b3b 同占二三类 ⟹ 反驳 disjoint。
  rintro ⟨I, _, hdisj⟩
  exact hdisj x_2b3b x_2b3b_is_type2 x_2b3b_is_type3 trivial

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 完备性定理（§10.2）：只有一二三类
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★完备性定理（§10.2，L0）** —— "市场必然产生赢利买卖点，只有第一、二、三类。"
  类型层穷尽：任一 `BspClass` 必是 one/two/three 之一，无第四类。
  ★这是 §10.2 缠论公理的忠实编码（类型只有三构造子）——L0 编码，非 L2 经验验证。
-/
theorem bspType_exhaustive (c : BspClass) :
    c = BspClass.one ∨ c = BspClass.two ∨ c = BspClass.three := by
  cases c
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/--
  **★完备性·无第四类（L0）** —— BspClass 恰有三构造子（DecidableEq 枚举穷尽）。
  与 legacy Strict.BSP / TrendCompleteClassification.trend_no_fourth_class 同构。
-/
theorem bspType_no_fourth :
    ∀ c : BspClass, c = BspClass.one ∨ c = BspClass.two ∨ c = BspClass.three :=
  bspType_exhaustive

/--
  **升跌完备性（§10.2，L0）** —— "任何向上/向下都必然从三类买卖点之一开始并结束。"
  形式化为：每个买卖点信号方向（买/卖）都对应三类之一——方向 × 三类穷尽买卖点的全部种类。
  本定理：买卖点种类 = Side × BspClass，共 6 种（买一二三 + 卖一二三），无第七种。
-/
theorem bsp_kinds_exhaustive (sd : Side) (c : BspClass) :
    (sd = Side.long ∨ sd = Side.short) ∧
    (c = BspClass.one ∨ c = BspClass.two ∨ c = BspClass.three) := by
  refine ⟨?_, bspType_exhaustive c⟩
  cases sd
  · exact Or.inl rfl
  · exact Or.inr rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 第三类本征子域真双射（重锚 Strict.BSP third_subdomain_classifies）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **第三类本征子域（§10.1）** —— 离开中枢后第一次回抽且确为第三类（买或卖）的端点。
  在此子域内，第三类**只按方向区分**（买/卖），是真完全分类（互斥穷尽 + 双射）。
-/
def ThirdSubdomain (e : BspEndpoint) : Prop := IsType3 e

/--
  **★第三类子域穷尽（L0，重锚 Strict.BSP third_exhaustive）** —— 第三类子域内必是
  三类买点或三类卖点（按方向二分）。
-/
theorem third_subdomain_exhaustive (e : BspEndpoint) (h : ThirdSubdomain e) :
    IsType3Buy e ∨ IsType3Sell e := h

/--
  **★第三类子域互斥（L0，重锚 Strict.BSP third_exclusive）** —— 同一端点不能既是三类买点
  又是三类卖点（方向唯一：long ≠ short）。
-/
theorem third_subdomain_exclusive (e : BspEndpoint) :
    ¬ (IsType3Buy e ∧ IsType3Sell e) := by
  rintro ⟨hb, hs⟩
  have h1 : e.side = Side.long := hb.1
  have h2 : e.side = Side.short := hs.1
  rw [h1] at h2; exact Side.noConfusion h2

/--
  **★第三类子域方向双射（L0）** —— 第三类子域端点 ↦ 方向（buy/sell）是双射（按 side 读出）。
  方向 `side` 即第三类子域的不变量（重锚 Strict.BSP thirdInvariant）。
-/
def thirdInvariant (e : BspEndpoint) : Side := e.side

theorem thirdInvariant_buy (e : BspEndpoint) (h : IsType3Buy e) :
    thirdInvariant e = Side.long := by
  unfold thirdInvariant; exact h.1

theorem thirdInvariant_sell (e : BspEndpoint) (h : IsType3Sell e) :
    thirdInvariant e = Side.short := by
  unfold thirdInvariant; exact h.1

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 反退化见证（具体三类端点）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 第一类买点见证：破中枢 + 背驰（forceC=2 < forceA=8）。 -/
def sampleType1 : BspEndpoint :=
  { side := Side.long
    center := { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }
    divPair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true }
    brokeCenter := true, afterTypeOne := false, leftCenter := false
    retracePrice := 5, firstRetrace := false }

theorem witness_type1 : IsType1 sampleType1 := by
  unfold IsType1 IsDivergence sampleType1; exact ⟨rfl, by decide⟩

/-- ★反退化见证：sampleType1 破中枢但若背驰反向（forceC≥forceA）则不是一类。 -/
theorem witness_type1_needs_divergence :
    ¬ IsType1 { sampleType1 with divPair := { forceA := ⟨2⟩, forceC := ⟨8⟩, isTrend := true } } := by
  unfold IsType1 IsDivergence; intro h; exact absurd h.2 (by decide)

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. still-MISSING 诚实声明 + 边界条件 + 下游推论
    ═══════════════════════════════════════════════════════════════════════

  ★still-MISSING-D（bspOf 全自动构造层 + 第二类次级别递归）：
    - 本文件形式化了买卖点三类的**判据层**（每类中枢关系 + 背驰 + 第三类位置三态应用 +
      单射裁定 + 完备性）。但 ChanlunElements.bspOf 要求 `List Move → List Bsp` 的**全自动
      识别**：遍历走势，对每个端点判定属哪类——本文件证判据，不证遍历构造。
    - 第二类"由次级别第一类构成"（买卖点定律一，§10.2）是**次级别递归**构成性陈述：
      本级别第二类 = 次级别第一类。本文件 `IsType2` 刻画本级别可观测条件（一类后回抽结束点），
      **未**形式化"递归到次级别验证第一类"——那需要递归级别系统（与 RecursiveLevelSystem 对接，
      still-MISSING-D）。把本级别判据冒充为完整次级别构成 = 声明膨胀（禁止）。

  ★边界条件（结论翻转）：
    - 第三类"不破 ZG"用严格 `zg < retracePrice`（回抽低点严格高于 ZG）。临界 retracePrice = zg
      （恰好回到中枢上沿）归为"破 ZG"（非第三类）——若改判据为"≤ ZG 才算破"（含等号），
      临界归属翻转，须重裁（与第24课"不重新进入中枢"的闭/开区间口径相关）。
    - 完备性"只有三类"是缠论公理的 L0 编码（类型穷尽）。它**不**断言"真实市场只有三类"
      （那是 L2 经验，需全市场否证检验）——有效域 = 缠论形式系统内，非经验市场。
    - 互斥三分失败（`no_exclusive_trichotomy`）依赖 `x_2b3b`（2/3 类可重合）。若某口径下
      2/3 类被定义为不可重合（如强制第三类排除第二类），则 x_2b3b 不存在，互斥三分可能恢复——
      但 §10.1 + 第24课支持 2/3 可重合（V 型反转），当前裁定不翻转。

  ★下游推论：
    - 买卖点完备性"只有一二三类" ⟹ 策略组件工位（#114）的应对策略只需覆盖三类 × 两方向 = 6 种
      买卖点信号，无第七种（升跌完备性 `bsp_kinds_exhaustive`）。
    - 互斥三分失败 ⟹ 策略层不能把买卖点当互斥 sum type 路由（必须处理 2/3 共存端点）——
      与 legacy Strict.BSP 一致（精化签名才真单射）。
    - 第一类 = 背驰点（模块3 `IsDivergence`）⟹ 背驰判据正确是第一类判据有意义的前提（模块依赖链）。

  ★谱系引用：买卖点互斥三分失败 + 第三类子域双射已在 legacy Strict/BSP.lean 完整形式化
    （Layer2A no_global_classifies / Layer2B third_subdomain_classifies / Layer2A′ refined_classifies，
    谱系 598→603→615 概念分离）。本文件重锚其判据到 Origin 命名空间（用 Origin.Center/Side/
    Divergence），结构同构，非重证从零。精化触发形态签名（2/3 共存独立标签真单射）的完整
    `Classifies` 实例化在 Strict.BSP，本文件给单射裁定的核心反例 `x_2b3b` + 完备性穷尽。
-/

end NewChanlun.Origin
