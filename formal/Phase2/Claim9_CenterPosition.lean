/-
  中枢位置三态真完全分类（Phase 2 claim9）
  第49课（2007-03-22）中枢三态 + zhongshu.md:82-90 + 603 范式（实数三歧推论 L0）

  缠论定理（已结算，zhongshu.md v1.0 含第49课）：
  - 第49课原文（zhongshu.md:84-88）："对任何级别的走势中枢，无非有三种情况：
      1. 当下在该中枢之中 (ZG-ZD)；
      2. 当下在该中枢之下（小于 ZD）；
      3. 当下在该中枢之上（大于 ZG）。"
  - 这是"当下价格点 p 相对单个中枢核心区间 [ZD, ZG] 的位置"的完全分类（位置三态），
    与第20课"前后两中枢发展关系"三分（延伸/新生/扩展，已由 CenterTrichotomy 形式化）
    是 **不同的完全分类**——前者分类「一个点相对一个区间」，后者分类「两个区间的关系」。

  范式角色（603 §三 / 实数三歧 trichotomy 的推论，L0）：
  - 第49课明确用「小于 ZD」「大于 ZG」刻画之下/之上，故端点 p=ZD、p=ZG 归「之中」——
    弱不等式核心区间 [ZD, ZG]（闭区间），之下/之上用严格不等式。
  - 边界约定与下游 maimai 第三类买点判定的口径关系（codex 审计 session 019effb9 确认）：
    spec `maimai_rules_v1.md:242` 第三类买点严格实现为「回试 low > ZG → 3B 成立」（严格 `>`），
    与本模块 `IsAbove := zg < p`（严格 `>`）一致——p=ZG 归 within（核心闭区间），破 ZG（p>ZG）才算之上。
    ★边界口径提示（非定义冲突，不上浮）：若下游把口语「不跌破 ZG」误读为弱口径 `low ≥ ZG`
    并称 p=ZG 为「中枢外/上方」，则与本位置三态在端点 ZG 处不同（本模块判 within）。该差异是
    **下游第三类买点事件谓词的口径问题**，不是第49课位置三态定义问题——位置三态忠实第49课
    「大于 ZG」严格不等式，下游不得用弱口径直接复用 `IsAbove`（codex 审计明确裁定，不修本定义）。

  本模块把「价格点相对中枢区间的位置」建模为对实数点 p 与区间端点 [zd, zg] 比较的
  **完全分类**：below(p<zd) / within(zd≤p≤zg) / above(p>zg)。证明三态
  穷尽（totality：任意点必属其一）+ 恰好其一（exactly one：两两互斥）。
  这里的「穷尽」是实数线相对闭区间的三歧 trichotomy 推论（L0，omega 机器可检验）。

  认识论：L0（定义内蕴，继承第49课已结算定义；不依赖数据）。
  Lean build 通过 = 逻辑正确（L0），不是实证有效域，不得膨胀（formalization-validity-domain）。
-/

import Formal.CenterTrichotomy

namespace Formal.CenterPosition

open Formal.CenterTrichotomy (Center)

/--
  中枢位置三态（第49课，位置完全分类）。

  价格点相对中枢核心区间 [ZD, ZG] 的位置，三构造子穷尽（第49课"无非有三种情况"）：
  - `below`（之下，第49课情况2）：当下小于 ZD。
  - `within`（之中，第49课情况1）：当下在 [ZD, ZG] 闭区间内（含端点）。
  - `above`（之上，第49课情况3）：当下大于 ZG。

  ★边界约定：第49课原文以「小于 ZD / 大于 ZG」刻画之下/之上 ⟹ 之中是闭区间 [ZD, ZG]，
  端点 ZD、ZG 归 within。与 maimai.md:133 第三类买点「不破 ZG 算中枢外/上方」互补
  （破才算入，临界点归核心闭区间）。
-/
inductive RelativePosition where
  | below    -- 之中枢之下（p < ZD）
  | within   -- 之中枢之中（ZD ≤ p ≤ ZG）
  | above    -- 之中枢之上（p > ZG）
deriving DecidableEq, Repr

open RelativePosition

/-! ## 三态谓词（显式刻画第49课三种情况） -/

/-- 之下谓词（第49课情况2）：当下价格 `p` 小于中枢核心下沿 ZD。 -/
def IsBelow (c : Center) (p : Int) : Prop := p < c.zd
/-- 之中谓词（第49课情况1）：当下价格 `p` 在闭核心区间 [ZD, ZG] 内。 -/
def IsWithin (c : Center) (p : Int) : Prop := c.zd ≤ p ∧ p ≤ c.zg
/-- 之上谓词（第49课情况3）：当下价格 `p` 大于中枢核心上沿 ZG。 -/
def IsAbove (c : Center) (p : Int) : Prop := c.zg < p

/--
  从价格点与中枢端点判定位置三态（第49课原文的可判定实现）。

  判据（zhongshu.md:84-88）：
  - p < ZD → 之下（below）
  - ZD ≤ p ≤ ZG → 之中（within）
  - p > ZG → 之上（above）
-/
def classify (c : Center) (p : Int) : RelativePosition :=
  if p < c.zd then below
  else if c.zg < p then above
  else within

/-! ## classify 忠实于三态谓词 -/

/-- `classify = below` 忠实于之下谓词。 -/
theorem classify_eq_below (c : Center) (p : Int) :
    classify c p = below ↔ IsBelow c p := by
  unfold classify IsBelow
  by_cases h : p < c.zd
  · simp [h]
  · simp only [h, if_false]
    by_cases h2 : c.zg < p <;> simp [h2]

/-- `classify = above` 忠实于之上谓词。 -/
theorem classify_eq_above (c : Center) (p : Int) :
    classify c p = above ↔ IsAbove c p := by
  unfold classify IsAbove
  by_cases h : p < c.zd
  · -- p < zd < zg ⟹ ¬ zg < p，故 classify=below≠above 且谓词假
    have hzd := c.core_valid
    simp only [h, if_true]
    constructor
    · intro hc; exact absurd hc (by decide)
    · intro hg; omega
  · simp only [h, if_false]
    by_cases h2 : c.zg < p <;> simp [h2]

/-- `classify = within` 忠实于之中谓词。 -/
theorem classify_eq_within (c : Center) (p : Int) :
    classify c p = within ↔ IsWithin c p := by
  unfold classify IsWithin
  by_cases h : p < c.zd
  · simp only [h, if_true]
    constructor
    · intro hc; exact absurd hc (by decide)
    · intro ⟨h1, _⟩; omega
  · simp only [h, if_false]
    by_cases h2 : c.zg < p
    · simp only [h2, if_true]
      constructor
      · intro hc; exact absurd hc (by decide)
      · intro ⟨_, h3⟩; omega
    · simp only [h2, if_false]
      constructor
      · intro _; exact ⟨by omega, by omega⟩
      · intro _; trivial

/-! ## 真完全分类：穷尽 + 恰好其一 -/

/--
  ★中枢位置三态真完全分类（构造子穷尽，L0）：classify 全函数对任意点返回三态之一。

  第49课"无非有三种情况"的机器证明——classify 是全函数，值域恰为 {below, within, above}。
-/
theorem position_trichotomy (c : Center) (p : Int) :
    classify c p = below ∨ classify c p = within ∨ classify c p = above := by
  unfold classify
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inr rfl)
    · exact Or.inr (Or.inl rfl)

/--
  ★三态谓词穷尽（totality）：任意点 p 必满足之下/之中/之上谓词之一。

  实数线相对闭区间 [ZD, ZG] 的三歧 trichotomy 推论（L0，omega）。
  第49课"无非有三种情况"在谓词层的完整覆盖。
-/
theorem position_predicates_total (c : Center) (p : Int) :
    IsBelow c p ∨ IsWithin c p ∨ IsAbove c p := by
  unfold IsBelow IsWithin IsAbove
  omega

/-! ### 两两互斥（pairwise disjoint） -/

/-- 之下 / 之中 互斥（p<ZD 与 ZD≤p 不可同真）。 -/
theorem below_within_disjoint (c : Center) (p : Int) :
    ¬ (IsBelow c p ∧ IsWithin c p) := by
  unfold IsBelow IsWithin
  intro ⟨h1, h2, _⟩
  omega

/-- 之下 / 之上 互斥（p<ZD 与 ZG<p，由 ZD<ZG 不可同真）。 -/
theorem below_above_disjoint (c : Center) (p : Int) :
    ¬ (IsBelow c p ∧ IsAbove c p) := by
  unfold IsBelow IsAbove
  intro ⟨h1, h2⟩
  have hzd := c.core_valid
  omega

/-- 之中 / 之上 互斥（p≤ZG 与 ZG<p 不可同真）。 -/
theorem within_above_disjoint (c : Center) (p : Int) :
    ¬ (IsWithin c p ∧ IsAbove c p) := by
  unfold IsWithin IsAbove
  intro ⟨⟨_, h2⟩, h3⟩
  omega

/--
  ★恰好其一（exactly one）：任意点满足且仅满足三态谓词之一。

  穷尽（position_predicates_total）+ 两两互斥的合取——第49课位置三态是
  实数线相对中枢核心区间的 **真划分**（partition）。
-/
theorem position_exactly_one (c : Center) (p : Int) :
    (IsBelow c p ∧ ¬ IsWithin c p ∧ ¬ IsAbove c p)
    ∨ (¬ IsBelow c p ∧ IsWithin c p ∧ ¬ IsAbove c p)
    ∨ (¬ IsBelow c p ∧ ¬ IsWithin c p ∧ IsAbove c p) := by
  unfold IsBelow IsWithin IsAbove
  have hzd := c.core_valid
  omega

end Formal.CenterPosition
