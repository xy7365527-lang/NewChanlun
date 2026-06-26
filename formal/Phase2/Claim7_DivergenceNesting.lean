/-
  背驰 / 区间套完全分类（Phase 2 claim7）
  002 背驰-买卖点定理 + 003 区间套定理 + beichi + 603 §五（L_confirm=构造子字段）

  缠论定理（已结算）：
  - 背驰-买卖点定理（002，beichi 第24课）：任一背驰必制造某级别买卖点；任一买卖点必源自
    某级别背驰（双向蕴含/充要 at 某级别）。
  - 区间套定理（003，beichi 第27课）：大级别转折点由不同级别背驰段逐级收缩范围确定，
    D_n ⊃ D_{n-1} ⊃ ... ⊃ D_1，转折点 ∈ D_1。

  范式（603 §五 + 沿 Phase 1 终余代数）：
  - 背驰不是独立轴——它是 **第一类 BSP 的力度判据**（趋势背驰必产生第一类，盘整背驰不产生）。
    上轮轴范式的 "L_confirm/δ" 是第一类背驰的 **区间套递归深度构造子字段**（beichi.md:368），
    by construction 已含，不是待发现的独立维度（603 §五，codex Q3 PASS）。
  - 区间套是 **级别递归的直接应用**——逐级收缩 = 终余代数沿级别向下 unfold 的有限严格下降。
    嵌套范围严格收缩 ⟹ 有限终止（到最低级别背驰段完成），与 Phase 1 r* 自然终止同构。

  本模块形式化：
  1. 背驰 = 力度衰减判据（趋势背驰 → 第一类 BSP；盘整背驰 → 不产生 BSP），完全分类。
     ★codex 审计修正（Q1）：背驰类型不是自由 sum type——它由 **走势三分（TrendKind）投影**而成
     （趋势 ⟹ 趋势背驰、盘整 ⟹ 盘整背驰），无超出 Move 类型的自由度。这坐实"背驰是 Move
     类型的字段而非独立轴"——见 `divergenceOfTrend` + `divergence_kind_no_free_dof`。
  2. 区间套 = 严格收缩的有限嵌套链（每级范围真子集 + 级别递减），终止性证明。
     ★codex 审计修正（Q3）：补"链长受首段 level 控制"的正式 theorem（`nesting_chain_length_le`），
     不再让"链有限"停留在注释声明（去声明膨胀）。
  3. L_confirm = 区间套深度，是构造子字段（绑定真实嵌套链长度），非独立轴。

  认识论：L0（定义内蕴，继承 002/003/beichi 已结算定理）。
-/

import Formal.TrendTrichotomy

namespace Formal.DivergenceNesting

open Formal.TrendTrichotomy (TrendKind isTrend)

/-! ## 背驰完全分类（趋势背驰 / 盘整背驰，beichi + qushi） -/

/--
  ★背驰类型完全分类（beichi / qushi.md：趋势背驰 vs 盘整背驰，二构造子穷尽）。

  - `trendDivergence`：趋势背驰——必然终结趋势，**产生第一类买卖点**（beichi 第24课 + qushi）。
  - `consolidationDivergence`：盘整背驰——力度衰竭不一定终结走势，**不产生三类买卖点**
    （maimai #4 已结算：盘整背驰非买卖点）。

  这是 beichi 的力度判据二分——背驰发生在"趋势"还是"盘整"中，决定是否产生 BSP。
  无第三种背驰类型（背驰只在趋势/盘整两种走势类型中发生，走势三分的 trend/consolidation 二分）。

  ★codex 审计修正（Q1）：这个二分**不携带超出走势三分的自由度**——它由 `TrendKind`
  唯一投影（见 `divergenceOfTrend`）。`DivergenceKind` 作为独立 inductive 仅是该投影的**陪域**，
  不是与 Move/BSP 并列竞争的独立轴（`divergence_kind_no_free_dof` 证明无额外自由度）。
-/
inductive DivergenceKind where
  | trendDivergence         -- 趋势背驰（必产生第一类 BSP）
  | consolidationDivergence -- 盘整背驰（不产生 BSP）
deriving DecidableEq, Repr

open DivergenceKind

/-- ★背驰二分完全分类（构造子穷尽，L0）。 -/
theorem divergence_dichotomy (d : DivergenceKind) :
    d = trendDivergence ∨ d = consolidationDivergence := by
  cases d
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  ★背驰类型由走势三分投影（codex 审计修正 Q1：背驰是 Move 类型的字段，非独立轴）。

  `divergenceOfTrend : TrendKind → DivergenceKind` 把走势三分（上涨/下跌/盘整）**唯一映射**
  为背驰类型：
  - `upTrend` / `downTrend`（趋势）⟹ `trendDivergence`（趋势背驰）；
  - `consolidation`（盘整）⟹ `consolidationDivergence`（盘整背驰）。

  这正是 beichi 第24课"没有趋势，就没有背驰；盘整中只有盘整背驰"的形式化——背驰类型
  **不是自由选择的轴**，是其所在走势的 `TrendKind` 的确定性函数（投影）。
-/
def divergenceOfTrend : TrendKind → DivergenceKind
  | TrendKind.upTrend => trendDivergence
  | TrendKind.downTrend => trendDivergence
  | TrendKind.consolidation => consolidationDivergence

/--
  ★背驰类型无额外自由度（codex 审计修正 Q1，L0）：背驰类型完全由走势三分决定。

  实质内容（非同义反复）：背驰类型 `trendDivergence` ⟺ 其投影来源走势 `t` 是趋势
  （`isTrend t = true`）。即 `divergenceOfTrend` 在 `DivergenceKind` 上的取值与
  `isTrend` 在 `TrendKind` 上的取值**双向等价**——背驰类型携带的信息恰好等于"走势是否趋势"，
  没有多出来的自由度。这关死 codex 指出的"`DivergenceKind` 形式上可被当自由轴滥用"：
  在 Move/走势语境下，背驰类型是 `isTrend` 的同构像，由 `divergenceOfTrend` 钉死。
-/
theorem divergence_kind_no_free_dof (t : TrendKind) :
    divergenceOfTrend t = trendDivergence ↔ isTrend t = true := by
  cases t <;> simp [divergenceOfTrend, isTrend]

/-- ★趋势（上涨/下跌）⟹ 趋势背驰（beichi 第24课：趋势才有背驰）。 -/
theorem trend_gives_trend_divergence (t : TrendKind) (h : isTrend t = true) :
    divergenceOfTrend t = trendDivergence :=
  (divergence_kind_no_free_dof t).mpr h

/-- ★盘整 ⟹ 盘整背驰（beichi 第24课：盘整中只有盘整背驰）。 -/
theorem consolidation_gives_consolidation_divergence :
    divergenceOfTrend TrendKind.consolidation = consolidationDivergence := rfl

/--
  ★背驰是否产生第一类 BSP（002 背驰-买卖点 + maimai #4）。
  趋势背驰 ⟹ 产生第一类 BSP（true）；盘整背驰 ⟹ 不产生（false）。
  这形式化"背驰是第一类 BSP 的力度判据"——背驰不是独立信号，是 BSP 的触发条件。
-/
def producesType1BSP : DivergenceKind → Bool
  | trendDivergence => true
  | consolidationDivergence => false

/--
  ★趋势背驰判定（codex/复核审计修正：去声明膨胀）：`producesType1BSP d ↔ d = trendDivergence`。

  这是 `producesType1BSP` 的定义判定（趋势背驰 ⟺ 产生第一类 BSP；盘整背驰不产生，maimai #4）。
  **诚实标注**：本定理 **不是** 002 完整充要的形式化——002"背驰 ⟺ 某级别买卖点"涉及
  "某级别"（区间套，见下）+ 三类买卖点 + 双向（买卖点 ⟹ 某级别背驰）。本定理只覆盖
  "趋势背驰 ⟹ 第一类 BSP"这一方向一支。002 的"某级别"由区间套实现（下），
  "买卖点 ⟹ 背驰"另一半 + "小转大"（本级完成由次级别背驰传播触发，zoushi:236/beichi:189）
  **不在本模块形式化范围**（诚实标注，formalization-validity-domain）。
-/
theorem trend_divergence_iff_type1 (d : DivergenceKind) :
    producesType1BSP d = true ↔ d = trendDivergence := by
  cases d <;> simp [producesType1BSP]

/-!
  ## 区间套完全分类（003，逐级收缩的有限严格下降）

  ★有效域诚实标注（codex/复核审计修正，formalization-validity-domain）：
  本模块的 `DivSegment` + `Nested` 形式化区间套的 **几何收缩骨架**（范围严格收缩 + 级别递减）。
  003 嵌套条件3"背驰独立成立"（每级背驰在该级别走势中独立满足）+ 终止条件2/3
  （背驰段完成、力度否定）属 **背驰语义层**（需 DivSegment 携带背驰段标记/方向/力度），
  **不在本模块形式化范围**。本模块声明的是"逐级收缩定位"的几何骨架（忠于 beichi:132），
  不声明完整 003 嵌套语义（有效域 < 定义域，诚实标注，非膨胀）。
-/

/--
  背驰段（区间套的一级）：时间范围 [lo, hi] + 级别。
  区间套在不同级别找背驰段，逐级收缩范围。
-/
structure DivSegment where
  lo : Int
  hi : Int
  level : Nat
  valid : lo ≤ hi

/--
  ★区间套嵌套关系（003，beichi 第27课）：低级别背驰段 **严格收缩** 于高级别。

  `Nested outer inner` ⟺：
  - 范围收缩：`outer.lo ≤ inner.lo ∧ inner.hi ≤ outer.hi`（inner ⊆ outer）；
  - ★严格：`inner.hi - inner.lo < outer.hi - outer.lo`（范围真收缩，beichi 嵌套条件1）；
  - ★级别**严格递减**（codex 审计 Q5 文字统一）：`inner.level < outer.level`——
    beichi 嵌套条件2 只要求"每次递推降级"，**允许跳过无背驰段的级别**（非相邻 +1）。
    注释与代码统一为 `<`（去 codex Q5 指出的"注释 +1 / 代码 < "矛盾）。
-/
def Nested (outer inner : DivSegment) : Prop :=
  outer.lo ≤ inner.lo
  ∧ inner.hi ≤ outer.hi
  ∧ (inner.hi - inner.lo) < (outer.hi - outer.lo)
  ∧ inner.level < outer.level   -- ★codex 审计修正：级别**递减**（非相邻 +1）——003/beichi 只要求逐级下降，可跳过无背驰段级别

/--
  ★区间套链（D_n ⊃ D_{n-1} ⊃ ... ⊃ D_1，003）：相邻段满足 Nested 的有限链。
-/
def NestingChain : List DivSegment → Prop
  | [] => True
  | [_] => True
  | outer :: inner :: rest => Nested outer inner ∧ NestingChain (inner :: rest)

/--
  ★区间套严格收缩（003，L0）：链中每级范围严格小于上一级。

  从 `Nested` 的严格收缩条件 + 级别递减，得嵌套链范围严格递减——这形式化
  "D_n ⊃ D_{n-1} ⊃ ... ⊃ D_1" 的真包含。严格收缩 ⟹ 链有限（不能无限嵌套），
  对应 beichi 第27课"级别有限，收缩到最低级别背驰段即最精确定位"。
-/
theorem nested_strictly_shrinks (outer inner : DivSegment) (h : Nested outer inner) :
    (inner.hi - inner.lo) < (outer.hi - outer.lo) := h.2.2.1

/--
  ★区间套相邻级别严格递减（003，终余代数有限下降）：内层级别 < 外层级别。

  本定理只声明它实际证明的一阶性质：相邻两段级别严格递减。**链长有限的全局界**
  由下面的 `nesting_chain_length_le` 单独证明（codex 审计 Q3：不在此注释里声明无证明的链长界）。
-/
theorem nested_level_decreases (outer inner : DivSegment) (h : Nested outer inner) :
    inner.level < outer.level := h.2.2.2

/--
  ★区间套相邻级别严格递减（链版本）：链首两段级别严格递减。
-/
theorem nesting_chain_level_strict (outer inner : DivSegment) (rest : List DivSegment)
    (h : NestingChain (outer :: inner :: rest)) :
    inner.level < outer.level :=
  (h.1).2.2.2

/--
  ★嵌套链段级别一致受首段界控制（codex 审计 Q3 辅助引理，L0）。

  若链 `c` 满足 `NestingChain` 且首段（若存在）级别 ≤ `b`，则链中**每一段**级别 ≤ `b`。
  这是级别沿链严格递减（每相邻 `< `）的直接推论——后续段级别只会更小。
  独立自包含归纳，不用 `where` 互递归（避免半成品证明结构）。
-/
theorem nestingChain_all_level_le :
    ∀ (c : List DivSegment), NestingChain c → ∀ (b : Nat),
      (match c with | [] => True | s :: _ => s.level ≤ b) →
      ∀ s ∈ c, s.level ≤ b
  | [], _, _, _, s, hs => by simp at hs
  | [hd], _, b, hhead, s, hs => by
      simp only [List.mem_singleton] at hs; subst hs; exact hhead
  | outer :: inner :: rest, h, b, hhead, s, hs => by
      rcases List.mem_cons.mp hs with hs | hs
      · subst hs; exact hhead
      · -- s ∈ inner :: rest：子链首段 inner.level < outer.level ≤ b ⟹ inner.level ≤ b
        have hstep : inner.level < outer.level := (h.1).2.2.2
        have hinner_le : inner.level ≤ b := by omega
        exact nestingChain_all_level_le (inner :: rest) h.2 b hinner_le s hs

/--
  ★嵌套链有界（codex 审计修正 Q3，L0）：合法嵌套链的长度受**首段级别**控制。

  把 codex 指出的"链长 ≤ 最高级别 + 1 / 不能无限嵌套"从**注释声明**提升为**正式定理**
  （去声明膨胀）。形式陈述：对任意合法嵌套链 `c`，给定级别上界 `b`（首段级别即可作上界，
  由 `nestingChain_all_level_le`），链长 `c.length ≤ b + 1`。这正是"区间套收缩到最低级别
  即终止"（beichi 第27课"级别有限不能精确到一点"）的机器证明，与 Phase 1 r* 终余代数的
  有限下降同构。证明对链结构归纳：级别沿链严格 -1 步进，自然数下降有限 ⟹ 链长受预算 b+1 限。
-/
theorem nesting_chain_length_le :
    ∀ (c : List DivSegment), NestingChain c →
      ∀ (b : Nat), (∀ s ∈ c, s.level ≤ b) → c.length ≤ b + 1
  | [], _, _, _ => by simp
  | [_], _, b, _ => by
      simp only [List.length_singleton]; omega
  | outer :: inner :: rest, h, b, hb => by
      have hstep : inner.level < outer.level := (h.1).2.2.2
      have houter : outer.level ≤ b := hb outer (by simp)
      -- 子链 (inner::rest) 首段 inner.level ≤ b - 1（因 inner < outer ≤ b），故所有段 ≤ b-1。
      have hinner_le : inner.level ≤ b - 1 := by omega
      have hsub_bound : ∀ s ∈ (inner :: rest), s.level ≤ b - 1 :=
        nestingChain_all_level_le (inner :: rest) h.2 (b - 1) hinner_le
      have IH := nesting_chain_length_le (inner :: rest) h.2 (b - 1) hsub_bound
      simp only [List.length_cons] at IH ⊢
      omega

/--
  ★区间套终止性推论（codex 审计 Q3，L0）：用首段级别作上界，链长 ≤ 首段级别 + 1。

  这是 `nesting_chain_length_le` 取 `b = head.level` 的实例——"不能无限嵌套"的最直接陈述：
  非空合法嵌套链的长度被其首段（最高级别）级别 + 1 限制。
-/
theorem nesting_chain_bounded_by_head (outer : DivSegment) (rest : List DivSegment)
    (h : NestingChain (outer :: rest)) :
    (outer :: rest).length ≤ outer.level + 1 :=
  nesting_chain_length_le (outer :: rest) h outer.level
    (nestingChain_all_level_le (outer :: rest) h outer.level (Nat.le_refl _))

/-! ## L_confirm = 区间套深度构造子字段（603 §五，非独立轴） -/

/--
  ★第一类买卖点 + 区间套深度（603 §五：L_confirm = 嵌套深度，**绑定实际 chain**）。

  codex 审计修正（Phase 2）：`nestingDepth` 不能是自由字段（否则仍像独立轴）——
  **必须绑定实际区间套链**：`chain : List DivSegment` + `chain_ok : NestingChain chain`
  + `depth_eq : nestingDepth = chain.length`。这样 L_confirm 是从 **真实嵌套链** 读出的深度，
  不是凭空字段，关死"L_confirm 是待补独立轴"（603 §五，beichi.md:368）。

  side 携带方向（1B/1S，codex 审计修正：不抹掉买/卖方向）；level = 本级别。
-/
structure Type1BSPWithNesting where
  divergence : DivergenceKind
  /-- ★实际区间套链（codex 审计：L_confirm 绑定真实 chain，非自由字段）。 -/
  chain : List DivSegment
  /-- 链满足嵌套关系（严格收缩 + 级别递减）。 -/
  chain_ok : NestingChain chain
  /--
    ★区间套链非空（独立复核 + codex session 019effca 赘类修正）：`chain ≠ []`。

    spec 003:18-19"D_n ⊃ D_{n-1} ⊃ ... ⊃ D_1，转折点 ∈ D_1"——合法区间套实例**至少含 D_1**。
    缺此约束时 `NestingChain [] = True`（归纳基底恒真）会允许**空链 + nestingDepth=0** 的
    "带区间套的第一类 BSP"实例——这是缠论本体不允许的对象（空链不定位任何转折点），
    形式化允许了它 = **赘类**（no-workaround 违例）。本约束关死该赘类：第一类 BSP 的区间套
    链必非空（至少有最低级别背驰段 D_1，转折点落于其中）。
  -/
  chain_nonempty : chain ≠ []
  /-- L_confirm = 嵌套链长度（深度由真实 chain 决定）。 -/
  nestingDepth : Nat
  depth_eq : nestingDepth = chain.length
  /-- 买/卖方向（1B/1S，不抹掉，codex 审计修正）。 -/
  side : Bool   -- true=买点(1B), false=卖点(1S)
  /-- 本级别。 -/
  level : Nat
  /-- 第一类 BSP 必由趋势背驰产生（构造约束）。 -/
  fromTrendDivergence : divergence = trendDivergence

/--
  ★L_confirm 由真实区间套链决定（603 §五，codex 审计修正后，L0）：

  `nestingDepth = chain.length` 且 `chain` 满足 `NestingChain`——L_confirm 是从 **实际嵌套链**
  读出的深度，不是自由字段。伪造的 nestingDepth（与 chain 长度不符）不满足 `depth_eq`。
  这关死"L_confirm 是与构造子竞争的独立轴"——它绑定真实的区间套递归过程（602 否定链关闭）。
-/
theorem lconfirm_bound_to_chain (b : Type1BSPWithNesting) :
    b.nestingDepth = b.chain.length ∧ NestingChain b.chain :=
  ⟨b.depth_eq, b.chain_ok⟩

/--
  ★区间套深度 ≥ 1（独立复核 + codex 019effca 赘类修正，L0）：

  由 `chain_nonempty`（spec 003 要求至少含 D_1），L_confirm = nestingDepth ≥ 1——
  不存在"区间套深度为 0 的第一类 BSP"。这把"空链赘类已关死"提升为可被下游引用的正命题：
  任意合法 `Type1BSPWithNesting` 的区间套深度严格为正（至少有最低级别背驰段 D_1）。
-/
theorem lconfirm_depth_pos (b : Type1BSPWithNesting) : b.nestingDepth ≥ 1 := by
  rw [b.depth_eq]
  cases hc : b.chain with
  | nil => exact absurd hc b.chain_nonempty
  | cons _ _ => simp

/--
  ★第一类 BSP 由趋势背驰产生（002，带 side/level，不抹方向）。

  codex 审计修正：带 side（1B/1S）和 level——002"任一买卖点必源自某级别背驰"中的
  方向与级别不抹掉。本定理：第一类 BSP 的背驰必是趋势背驰（producesType1BSP=true），
  且其方向/级别由结构携带（不是无 side 的抽象 BSP）。
-/
theorem type1_from_trend_divergence (b : Type1BSPWithNesting) :
    producesType1BSP b.divergence = true := by
  rw [b.fromTrendDivergence]; rfl

end Formal.DivergenceNesting
