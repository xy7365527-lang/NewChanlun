/-
Origin/CenterStates.lean — 中枢三态 + 中枢发展三态 + 中心定理一/二（task #113）

★工位定位（审计 A 判决）：Origin 的 `Center` 结构只有 {zd, zg, startIndex, endIndex, valid}，
  **无位置态、无 GG/DD 外缘、无发展态判据**。本文件把中枢的**分类血肉**形式化为 Origin
  命名空间内的可机器检查谓词/定理——位置三态、发展三态、中心定理一/二。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链）
═══════════════════════════════════════════════════════════════════════════
- 第49课（中枢位置三态，博文最终权威）：
  · "1.当下在该中枢之中 (ZG-ZD)。2.当下在该中枢之下（小于ZD)。3.当下在该中枢之上（大于ZG)。"（049:20-24）
- §6.4（GG/G/D/DD/ZG/ZD 定义，知识库 + chan99 第八节）：
  · GG = max(gₙ)，G = min(gₙ)，D = max(dₙ)，DD = min(dₙ)；ZG = min(g₁,g₂)，ZD = max(d₁,d₂)。
- §6.5 中心定理一（中枢延伸）：
  · "走势中枢的延伸等价于任意区间[dₙ，gₙ]与[ZD，ZG]有重叠。"（chan99 第八节:15）
- §6.5 中心定理二（前后同级别两中枢关系 → 趋势/扩展）：
  · "后 GG < 前 DD ⟺ 下跌及其延续；后 DD > 前 GG ⟺ 上涨及其延续；……⟺ 形成高级别的走势中枢。"
- §6.7 发展三态：
  · "走势中枢的延伸：等价于任意区间[dₙ,gₙ]与[ZD，ZG]有重叠。表现为盘整。
     走势中枢的新生：即形成趋势（上涨或下跌）。
     走势中枢的扩展：即形成高级别的走势中枢。"

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 整数三歧 trichotomy / 结构推导，不依赖数据，omega machine-checked）。
`lake env lean Origin/CenterStates.lean` 通过 = 位置三态/发展三态/两定理的逻辑双射与互斥穷尽
在定义层成立，**不是**任何"中枢识别在真实行情上有效"的实证断言。

诚实标注（gatekeeper）：
★ TrueCompleteClassification —— 位置三态（点相对 [ZD,ZG]）+ 发展三态（两同级别新生中枢关系）
  是真完全分类（互斥穷尽 + 双射），legacy Strict.Center 已证，本文件重锚到 Origin.Center。
★ 有效域诚实标注（重锚 legacy Strict/Center.lean 的裁定，谱系 256/264/608）：
  - 发展三态分类 A 的有效域 = **核心已分离的两同级别新生中枢对**（排除中心定理一延伸——核心重叠）；
    声明全 Center×Center 上完全分类 = 有效域膨胀（禁止）。
  - 位置三态是**静态划分**，不蕴含操作序（之下→之中→之上的递归穿越未形式化，由 OnlineSystem 定）。

★ still-MISSING-B（见文件尾）：本文件用扩展的 `CenterWithOuter`（Origin.Center + GG/DD 外缘）
  形式化定理；但 Origin.ChanlunElements.Center **不含 GG/DD 字段**（centersOf 平凡桩不产外缘）。
  从线段序列**自动计算 GG/DD**（centersOf 全自动构造）未在本文件证——本文件证判据层，不冒充构造层。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 中枢位置三态（第49课）：点相对 [ZD, ZG] 的之下/之中/之上
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **中枢位置标签（第49课）** —— 当下价格相对中枢区间的三种位置。
-/
inductive CenterPosition where
  | below   -- 之下（小于 ZD）
  | within  -- 之中（[ZD, ZG]）
  | above   -- 之上（大于 ZG）
deriving DecidableEq, Repr

/-- 之下：`p < ZD`（第49课"当下在该中枢之下，小于ZD"）。 -/
def IsBelow (c : Center) (p : Tick) : Prop := p < c.zd
/-- 之中：`ZD ≤ p ≤ ZG`（第49课"当下在该中枢之中，(ZG-ZD)"，闭区间）。 -/
def IsWithin (c : Center) (p : Tick) : Prop := c.zd ≤ p ∧ p ≤ c.zg
/-- 之上：`ZG < p`（第49课"当下在该中枢之上，大于ZG"）。 -/
def IsAbove (c : Center) (p : Tick) : Prop := c.zg < p

/-- 位置判定函数（点 ↦ 标签）。 -/
def classifyPosition (c : Center) (p : Tick) : CenterPosition :=
  if p < c.zd then CenterPosition.below
  else if c.zg < p then CenterPosition.above
  else CenterPosition.within

/--
  **★位置三态穷尽（L0，第49课）** —— 任一点相对中枢必落三态之一。
  依赖中枢不变量 `c.valid : zd ≤ zg`（闭区间良构）。整数三歧（trichotomy）。
-/
theorem position_total (c : Center) (p : Tick) :
    IsBelow c p ∨ IsWithin c p ∨ IsAbove c p := by
  have hv := c.valid
  simp only [IsBelow, IsWithin, IsAbove, Tick] at *
  omega

/--
  **★位置三态互斥（L0）** —— 一个点不能同时落两个不同位置。
-/
theorem position_disjoint (c : Center) (p : Tick) :
    ¬ (IsBelow c p ∧ IsWithin c p) ∧
    ¬ (IsBelow c p ∧ IsAbove c p) ∧
    ¬ (IsWithin c p ∧ IsAbove c p) := by
  have hv := c.valid
  simp only [IsBelow, IsWithin, IsAbove, Tick] at *
  refine ⟨?_, ?_, ?_⟩ <;> rintro ⟨h1, h2⟩ <;> omega

/--
  **★位置三态完备（complete，L0）** —— 谓词成立 ⟺ classifyPosition 给该标签。
  这是位置三态作为**完全分类**的双射核心（与 legacy Strict.Center position_pointwise_classifies 同构）。
-/
theorem classifyPosition_below_iff (c : Center) (p : Tick) :
    classifyPosition c p = CenterPosition.below ↔ IsBelow c p := by
  have hv : c.zd ≤ c.zg := c.valid
  unfold classifyPosition IsBelow
  constructor
  · intro h
    by_cases h1 : p < c.zd
    · exact h1
    · by_cases h2 : c.zg < p <;> simp only [h1, h2, if_true, if_false] at h <;>
        exact absurd h (by decide)
  · intro h
    simp only [if_pos h]

theorem classifyPosition_above_iff (c : Center) (p : Tick) :
    classifyPosition c p = CenterPosition.above ↔ IsAbove c p := by
  have hv : c.zd ≤ c.zg := c.valid
  unfold classifyPosition IsAbove
  constructor
  · intro h
    by_cases h1 : p < c.zd
    · simp only [h1, if_true] at h; exact absurd h (by decide)
    · by_cases h2 : c.zg < p
      · exact h2
      · simp only [h1, h2, if_false] at h; exact absurd h (by decide)
  · intro h
    have h1 : ¬ p < c.zd := by simp only [Tick] at hv h ⊢; omega
    simp only [if_neg h1, if_pos h]

theorem classifyPosition_within_iff (c : Center) (p : Tick) :
    classifyPosition c p = CenterPosition.within ↔ IsWithin c p := by
  have hv : c.zd ≤ c.zg := c.valid
  unfold classifyPosition IsWithin
  constructor
  · intro h
    by_cases h1 : p < c.zd
    · simp only [h1, if_true] at h; exact absurd h (by decide)
    · by_cases h2 : c.zg < p
      · simp only [h1, h2, if_false, if_true] at h; exact absurd h (by decide)
      · refine ⟨?_, ?_⟩ <;> (simp only [Tick] at h1 h2 ⊢; omega)
  · intro h
    have hd : c.zd ≤ p := h.1
    have hu : p ≤ c.zg := h.2
    have h1 : ¬ p < c.zd := by simp only [Tick] at hd ⊢; omega
    have h2 : ¬ c.zg < p := by simp only [Tick] at hu ⊢; omega
    simp only [if_neg h1, if_neg h2]

/-- `classifyPosition` 给出的标签确实满足对应谓词（sound，L0），由三 iff 导出。 -/
theorem classifyPosition_sound (c : Center) (p : Tick) :
    (classifyPosition c p = CenterPosition.below → IsBelow c p) ∧
    (classifyPosition c p = CenterPosition.within → IsWithin c p) ∧
    (classifyPosition c p = CenterPosition.above → IsAbove c p) :=
  ⟨(classifyPosition_below_iff c p).mp,
   (classifyPosition_within_iff c p).mp,
   (classifyPosition_above_iff c p).mp⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 中枢外缘 GG/DD + 中心定理一（延伸）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **带外缘的中枢（§6.4）** —— Origin.Center（[zd,zg] 核心）+ 外缘 GG/DD。

  - `core`：Origin.Center，携带 `zd ≤ zg`。
  - `dd`：所有 Z 走势段的最低低点 DD（外缘下界）。
  - `gg`：所有 Z 走势段的最高高点 GG（外缘上界）。
  - 不变量：`dd ≤ zd ≤ zg ≤ gg`（外缘包含核心，§6.4 GG≥ZG≥ZD≥DD）。
-/
structure CenterWithOuter where
  core : Center
  dd : Tick
  gg : Tick
  outer_lo : dd ≤ core.zd
  outer_hi : core.zg ≤ gg
deriving Repr

/--
  **中心定理一·延伸条件（§6.5）** —— "中枢的延伸等价于任意区间 [dₙ,gₙ] 与 [ZD,ZG] 有重叠。"
  一个新次级别走势段 `[segLo, segHi]` 与中枢核心 `[zd,zg]` 重叠 ⟺ `segLo ≤ zg ∧ zd ≤ segHi`。
-/
def CenterExtension (c : CenterWithOuter) (segLo segHi : Tick) : Prop :=
  segLo ≤ c.core.zg ∧ c.core.zd ≤ segHi

/--
  **中心定理一·破坏条件（§6.5 逆）** —— "若有 Zₙ 使得 dₙ > ZG 或 gₙ < ZD，则必然产生
  高级别走势中枢或趋势及延续。"段整体脱离核心 ⟺ `segLo > zg ∨ segHi < zd`。
-/
def CenterBroken (c : CenterWithOuter) (segLo segHi : Tick) : Prop :=
  segLo > c.core.zg ∨ segHi < c.core.zd

/--
  **★中心定理一（L0，§6.5）** —— 延伸与破坏互斥穷尽：一个段相对中枢核心，要么延伸（重叠）
  要么破坏（脱离）——`¬延伸 ↔ 破坏`。这把中心定理一形式化为 machine-checked 双歧。
  （要求段良构 `segLo ≤ segHi`。）
-/
theorem center_theorem_one (c : CenterWithOuter) (segLo segHi : Tick)
    (_hseg : segLo ≤ segHi) :
    ¬ CenterExtension c segLo segHi ↔ CenterBroken c segLo segHi := by
  have hv := c.core.valid
  simp only [CenterExtension, CenterBroken, Tick] at *
  constructor
  · intro h
    rcases Decidable.not_and_iff_not_or_not.mp h with h' | h'
    · exact Or.inl (by omega)
    · exact Or.inr (by omega)
  · rintro (h | h) ⟨h1, h2⟩ <;> omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 中心定理二（前后同级别两中枢关系 → 趋势/扩展）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **中心定理二·上涨延续（§6.5）** —— "后 DD > 前 GG ⟺ 上涨及其延续。"
  后中枢外缘下界 > 前中枢外缘上界（完全在上方，无外缘重叠）。
-/
def IsUpTrend (prev next : CenterWithOuter) : Prop := next.dd > prev.gg

/--
  **中心定理二·下跌延续（§6.5）** —— "后 GG < 前 DD ⟺ 下跌及其延续。"
  后中枢外缘上界 < 前中枢外缘下界。
-/
def IsDownTrend (prev next : CenterWithOuter) : Prop := next.gg < prev.dd

/--
  **中心定理二·扩展为高级别中枢（§6.5）** —— 核心已分离（新生）但外缘重叠 ⟹ 形成高级别中枢。
  两支：后核心在前核心之上（`prev.zg < next.core.zd`）且外缘重叠（`next.dd ≤ prev.gg`）；
  或后核心在前核心之下（`next.core.zg < prev.zd`）且外缘重叠（`prev.dd ≤ next.gg`）。
-/
def IsExpansion (prev next : CenterWithOuter) : Prop :=
  (prev.core.zg < next.core.zd ∧ next.dd ≤ prev.gg) ∨
  (next.core.zg < prev.core.zd ∧ prev.dd ≤ next.gg)

/--
  **同级别新生中枢对前提（核心已分离）** —— 后中枢核心整体在前核心之上或之下
  （`prev.zg < next.zd ∨ next.zg < prev.zd`）。这是发展三态分类 A 的有效域
  （排除中心定理一延伸——核心重叠，谱系 256/264 重锚）。
-/
def SeparatedNewPair (prev next : CenterWithOuter) : Prop :=
  prev.core.zg < next.core.zd ∨ next.core.zg < prev.core.zd

/--
  **★中心定理二三歧穷尽（L0，§6.5，有效域=核心已分离新生对）** —— 在核心已分离前提下，
  前后两同级别中枢关系必落三态之一：上涨 / 下跌 / 扩展。
  ★这是发展三态的核心判据（重锚 legacy Strict.Center trichotomy_predicates_total）。
-/
theorem center_theorem_two_total (prev next : CenterWithOuter)
    (hsep : SeparatedNewPair prev next) :
    IsUpTrend prev next ∨ IsDownTrend prev next ∨ IsExpansion prev next := by
  have hp := prev.core.valid; have hn := next.core.valid
  have hpl := prev.outer_lo; have hph := prev.outer_hi
  have hnl := next.outer_lo; have hnh := next.outer_hi
  simp only [IsUpTrend, IsDownTrend, IsExpansion, SeparatedNewPair, Tick] at *
  rcases hsep with hup | hdown
  · -- 后核心在前核心之上：要么外缘也分离（上涨）要么外缘重叠（扩展支1）
    by_cases hgap : next.dd > prev.gg
    · exact Or.inl hgap
    · refine Or.inr (Or.inr (Or.inl ⟨hup, ?_⟩))
      simp only [Tick] at hgap ⊢; omega
  · -- 后核心在前核心之下：要么外缘分离（下跌）要么外缘重叠（扩展支2）
    by_cases hgap : next.gg < prev.dd
    · exact Or.inr (Or.inl hgap)
    · refine Or.inr (Or.inr (Or.inr ⟨hdown, ?_⟩))
      simp only [Tick] at hgap ⊢; omega

/--
  **★中心定理二·上涨与扩展互斥（L0）** —— 上涨（外缘完全分离）与扩展（外缘重叠）不能同时成立。
-/
theorem up_expansion_disjoint (prev next : CenterWithOuter) :
    ¬ (IsUpTrend prev next ∧ IsExpansion prev next) := by
  have hp : prev.core.zd ≤ prev.core.zg := prev.core.valid
  have hn : next.core.zd ≤ next.core.zg := next.core.valid
  have hpl : prev.dd ≤ prev.core.zd := prev.outer_lo
  have hph : prev.core.zg ≤ prev.gg := prev.outer_hi
  have hnl : next.dd ≤ next.core.zd := next.outer_lo
  have hnh : next.core.zg ≤ next.gg := next.outer_hi
  unfold IsUpTrend IsExpansion
  rintro ⟨hup, (⟨h1, h2⟩ | ⟨h1, h2⟩)⟩ <;>
    (simp only [Tick] at hup h1 h2 hp hn hpl hph hnl hnh ⊢; omega)

/--
  **★中心定理二·下跌与扩展互斥（L0）** -/
theorem down_expansion_disjoint (prev next : CenterWithOuter) :
    ¬ (IsDownTrend prev next ∧ IsExpansion prev next) := by
  have hp : prev.core.zd ≤ prev.core.zg := prev.core.valid
  have hn : next.core.zd ≤ next.core.zg := next.core.valid
  have hpl : prev.dd ≤ prev.core.zd := prev.outer_lo
  have hph : prev.core.zg ≤ prev.gg := prev.outer_hi
  have hnl : next.dd ≤ next.core.zd := next.outer_lo
  have hnh : next.core.zg ≤ next.gg := next.outer_hi
  unfold IsDownTrend IsExpansion
  rintro ⟨hdown, (⟨h1, h2⟩ | ⟨h1, h2⟩)⟩ <;>
    (simp only [Tick] at hdown h1 h2 hp hn hpl hph hnl hnh ⊢; omega)

/--
  **★中心定理二·上涨与下跌互斥（L0）** —— 后中枢不能同时完全在前之上又完全在前之下。
-/
theorem up_down_disjoint (prev next : CenterWithOuter) :
    ¬ (IsUpTrend prev next ∧ IsDownTrend prev next) := by
  have hpl := prev.outer_lo; have hph := prev.outer_hi
  have hnl := next.outer_lo; have hnh := next.outer_hi
  have hp := prev.core.valid; have hn := next.core.valid
  simp only [IsUpTrend, IsDownTrend, Tick] at *
  rintro ⟨hup, hdown⟩; omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 中枢发展三态（§6.7）：延伸 / 新生 / 扩展 —— 完全分类
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **中枢发展标签（§6.7）** —— 中枢的三种发展状态。
  - `extension`：延伸（新段与核心重叠，表现为盘整）。
  - `newBirth`：新生（形成趋势，上涨或下跌——外缘完全分离）。
  - `expansion`：扩展（形成高级别中枢——核心分离但外缘重叠）。
-/
inductive CenterDevelopment where
  | extension
  | newBirth
  | expansion
deriving DecidableEq, Repr

/--
  **发展态判定（前中枢 + 后中枢/新段，§6.7）** —— 给定前中枢 `prev` 与后中枢 `next`：
  - 核心重叠（¬分离）→ 延伸（中心定理一：[dₙ,gₙ] 与 [ZD,ZG] 重叠）。
  - 核心分离 + 外缘分离 → 新生（中心定理二：趋势）。
  - 核心分离 + 外缘重叠 → 扩展（中心定理二：高级别中枢）。
-/
def classifyDevelopment (prev next : CenterWithOuter) : CenterDevelopment :=
  if prev.core.zg < next.core.zd ∨ next.core.zg < prev.core.zd then
    -- 核心已分离
    if next.dd > prev.gg ∨ next.gg < prev.dd then CenterDevelopment.newBirth
    else CenterDevelopment.expansion
  else
    CenterDevelopment.extension

/-- 延伸谓词：核心重叠（¬ 分离）。 -/
def DevExtension (prev next : CenterWithOuter) : Prop := ¬ SeparatedNewPair prev next
/-- 新生谓词：核心分离 ∧ 外缘分离（趋势）。 -/
def DevNewBirth (prev next : CenterWithOuter) : Prop :=
  SeparatedNewPair prev next ∧ (IsUpTrend prev next ∨ IsDownTrend prev next)
/-- 扩展谓词：核心分离 ∧ 外缘重叠（高级别中枢）。 -/
def DevExpansion (prev next : CenterWithOuter) : Prop :=
  SeparatedNewPair prev next ∧ IsExpansion prev next

/--
  **★发展三态穷尽（L0，§6.7）** —— 任一前后中枢对必落发展三态之一：延伸/新生/扩展。
  ★这是中枢发展的**完全分类**核心——把 §6.7 三态形式化为 machine-checked 穷尽。
-/
theorem development_total (prev next : CenterWithOuter) :
    DevExtension prev next ∨ DevNewBirth prev next ∨ DevExpansion prev next := by
  unfold DevExtension DevNewBirth DevExpansion
  by_cases hsep : SeparatedNewPair prev next
  · rcases center_theorem_two_total prev next hsep with hup | hdown | hexp
    · exact Or.inr (Or.inl ⟨hsep, Or.inl hup⟩)
    · exact Or.inr (Or.inl ⟨hsep, Or.inr hdown⟩)
    · exact Or.inr (Or.inr ⟨hsep, hexp⟩)
  · exact Or.inl hsep

/--
  **★发展三态·延伸与新生互斥（L0）** —— 延伸（核心重叠）与新生（核心分离）不能同时成立。
-/
theorem dev_extension_newBirth_disjoint (prev next : CenterWithOuter) :
    ¬ (DevExtension prev next ∧ DevNewBirth prev next) := by
  unfold DevExtension DevNewBirth
  rintro ⟨hne, hsep, _⟩; exact hne hsep

/--
  **★发展三态·延伸与扩展互斥（L0）** -/
theorem dev_extension_expansion_disjoint (prev next : CenterWithOuter) :
    ¬ (DevExtension prev next ∧ DevExpansion prev next) := by
  unfold DevExtension DevExpansion
  rintro ⟨hne, hsep, _⟩; exact hne hsep

/--
  **★发展三态·新生与扩展互斥（L0）** —— 新生（外缘分离/趋势）与扩展（外缘重叠）互斥。
-/
theorem dev_newBirth_expansion_disjoint (prev next : CenterWithOuter) :
    ¬ (DevNewBirth prev next ∧ DevExpansion prev next) := by
  unfold DevNewBirth DevExpansion
  rintro ⟨⟨_, hud⟩, ⟨_, hexp⟩⟩
  rcases hud with hup | hdown
  · exact up_expansion_disjoint prev next ⟨hup, hexp⟩
  · exact down_expansion_disjoint prev next ⟨hdown, hexp⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 反退化见证（具体缠论数值例子，非平凡桩）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 构造一个具体中枢（核心[10,20]，外缘[5,25]）用于见证。 -/
def sampleCenter : CenterWithOuter :=
  { core := { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }
    dd := 5, gg := 25, outer_lo := by decide, outer_hi := by decide }

/-- 后中枢核心[30,40]外缘[28,45]：外缘下界28 > 前外缘上界25 ⟹ 上涨新生（趋势）。 -/
def sampleUpNext : CenterWithOuter :=
  { core := { zd := 30, zg := 40, startIndex := 4, endIndex := 7, valid := by decide }
    dd := 28, gg := 45, outer_lo := by decide, outer_hi := by decide }

/-- 后中枢核心[22,32]外缘[18,38]：核心分离(前zg=20<后zd=22)但外缘重叠(后dd=18≤前gg=25) ⟹ 扩展。 -/
def sampleExpNext : CenterWithOuter :=
  { core := { zd := 22, zg := 32, startIndex := 4, endIndex := 7, valid := by decide }
    dd := 18, gg := 38, outer_lo := by decide, outer_hi := by decide }

/-- ★反退化见证：点 5 在 sampleCenter 核心之下（5 < zd=10）。 -/
theorem witness_below : classifyPosition sampleCenter.core 5 = CenterPosition.below := by
  unfold classifyPosition sampleCenter; decide

/-- ★反退化见证：点 15 在 sampleCenter 核心之中（10 ≤ 15 ≤ 20）。 -/
theorem witness_within : classifyPosition sampleCenter.core 15 = CenterPosition.within := by
  unfold classifyPosition sampleCenter; decide

/-- ★反退化见证：点 30 在 sampleCenter 核心之上（20 < 30）。 -/
theorem witness_above : classifyPosition sampleCenter.core 30 = CenterPosition.above := by
  unfold classifyPosition sampleCenter; decide

/-- ★反退化见证：sampleCenter→sampleUpNext 是新生（上涨趋势，外缘分离）。 -/
theorem witness_newBirth :
    classifyDevelopment sampleCenter sampleUpNext = CenterDevelopment.newBirth := by
  unfold classifyDevelopment sampleCenter sampleUpNext; decide

/-- ★反退化见证：sampleCenter→sampleExpNext 是扩展（核心分离+外缘重叠）。 -/
theorem witness_expansion :
    classifyDevelopment sampleCenter sampleExpNext = CenterDevelopment.expansion := by
  unfold classifyDevelopment sampleCenter sampleExpNext; decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. still-MISSING 诚实声明 + 边界条件 + 下游推论
    ═══════════════════════════════════════════════════════════════════════

  ★still-MISSING-B（centersOf 全自动构造层 + GG/DD 字段缺失）：
    本文件用扩展结构 `CenterWithOuter`（Origin.Center 核心 + GG/DD 外缘）形式化定理。但
    Origin.ChanlunElements.Center **只有 {zd,zg,startIndex,endIndex,valid}，无 GG/DD 字段**。
    ChanlunElements.centersOf 要求 `List Segment → List Center` 的全自动构造，其中
    "从前三个连续次级别走势段计算 ZG=min(g₁,g₂)/ZD=max(d₁,d₂) 以及外缘 GG=max(gₙ)/DD=min(dₙ)"
    **未在本文件实装**（centersOf 仍是平凡桩，且 Center 结构需扩字段才能承载外缘）。
    本文件**不声称**已实装 centersOf——只声称中枢分类血肉（位置三态/发展三态/两定理）formalized。
    若要让 centersOf 非平凡，须扩 Center 结构（改 ChanlunElements，非本工位 own）或在 Origin
    新建 Center↔CenterWithOuter 投影 + 自动外缘计算函数（剩余工作，标 still-MISSING-B）。

  ★边界条件（结论翻转）：
    - 发展三态分类 A 的有效域 = 核心已分离的新生对（`SeparatedNewPair`）。延伸（中心定理一，
      核心重叠）作 `DevExtension` 单独处理，**不**进入中心定理二的三歧——若强行声称
      `center_theorem_two_total` 在全 Center×Center 上成立，total 会失败（核心重叠时既非上涨
      又非下跌，扩展支也要核心分离前提）。这是有效域 ≠ 定义域的诚实标注（谱系 256/264）。
    - 位置三态用闭区间（临界 p=zd 算之中，p=zg 算之中）。若改用开区间或 ℝ 测度，临界归属须重裁。

  ★下游推论：
    - BspClassification（模块4）第三类买卖点判据"回抽不破 ZG/ZD"直接用本文件 `IsAbove`/`IsBelow`/
      位置三态——第三类是位置三态在"离开中枢后回抽"语境下的应用。
    - Divergence（模块3）"任一背驰制造某级别买卖点"中的"某级别"由中枢级别（本文件中枢）确定。

  ★谱系引用：legacy Strict/Center.lean（中枢三态 Layer2 双射，谱系 608/007/008）已重锚到本文件
    Origin 命名空间；位置三态点位侧双射 = `classifyPosition_*_iff`，发展三态 = `development_total` +
    三互斥。重锚 = port（结构同构，命名空间从 Strict.Center → NewChanlun.Origin），非重证从零。
-/

end NewChanlun.Origin
