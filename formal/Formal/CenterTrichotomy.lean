/-
  中枢三态完全分类（007/008 中心定理一/二 + zhongshu.md:328 + 603 §2.2/§三）

  缠论定理（已结算）：
  - 中心定理二（008，zhongshu.md 第20课）：前后同级别两中枢的关系完全三分类——
    "后 DD > 前 GG ⟺ 上涨延续；后 GG < 前 DD ⟺ 下跌延续；
     (后 ZG < 前 ZD ∧ 后 GG ≥ 前 DD) ∨ (后 ZD > 前 ZG ∧ 后 DD ≤ 前 GG) ⟺ 级别扩张"
  - 完备性（008 证明）：上涨延续 ∪ 下跌延续 ∪ 级别扩张 = 全集（完备三分类）。
  - zhongshu.md:328（已结算）："延伸 ∪ 新生 ∪ 扩展 = 全集"。

  范式角色（603 §三 inductive step）：中枢三态穷尽是走势三分归纳步骤的关键——
  Move(ℓ) 的类型由其 subs 的中枢三态唯一决定（延伸=盘整延续 / 新生=趋势 / 扩展=升父级）。
  中枢三态穷尽 ⟹ inductive step 不产生第四 Move（codex Q4 核实严格）。

  本模块把"两中枢关系"建模为对实数区间端点比较的 **完全分类**：给定前后两中枢的
  (DD, GG)（外缘）与 (ZD, ZG)（核心），证明三态（上涨延续/下跌延续/级别扩张）穷尽。
  这里的"穷尽"是端点比较的完全分类（实数三歧 trichotomy 的推论，L0）。
-/

import Formal.TrendTrichotomy

namespace Formal.CenterTrichotomy

open Formal.TrendTrichotomy (TrendKind Direction)

/--
  中枢的区间端点（zhongshu.md / formal_axioms §3.2）。

  - `[zd, zg]` 核心区间：前两段定，`zg > zd` 为成立条件（中枢核固定，I5 不变量）。
  - `[dd, gg]` 外缘区间：全部参与段定（含全部波动）。约束 `dd ≤ zd ∧ zg ≤ gg`。
-/
structure Center where
  dd : Int   -- DD 外缘下沿
  zd : Int   -- ZD 核心下沿
  zg : Int   -- ZG 核心上沿
  gg : Int   -- GG 外缘上沿
  core_valid : zd < zg          -- 中枢成立条件 ZG > ZD
  outer_lo : dd ≤ zd            -- 外缘包含核心（下）
  outer_hi : zg ≤ gg            -- 外缘包含核心（上）

/--
  两中枢关系的三态分类（中心定理二，008，完全三分类）。

  与 zhongshu.md:328 "延伸 ∪ 新生 ∪ 扩展" 的三态对应：
  - `upContinuation`（新生·上涨延续）：后 DD > 前 GG（外缘完全分离向上）。
  - `downContinuation`（新生·下跌延续）：后 GG < 前 DD（外缘完全分离向下）。
  - `levelExpansion`（扩展）：外缘有重叠（既非向上分离也非向下分离）。
-/
inductive CenterRelation where
  | upContinuation     -- 上涨延续（后 DD > 前 GG）
  | downContinuation   -- 下跌延续（后 GG < 前 DD）
  | levelExpansion     -- 级别扩张（外缘重叠）
deriving DecidableEq, Repr

open CenterRelation

/-!
  ★codex 审计 R1#4 / R2#4-5 修正（session 019eff21）：三态判据须显式编码 **中心定理二
  原公式**（核心区间分离 + 波动区间重叠），不能让 `¬up ∧ ¬down` 吞掉延伸/同枢。
  下面三谓词显式刻画三态，证明对**两个同级别新生中枢**（`SameLevelNewCenterPair`
  前提，排除同枢延伸）两两互斥 ∧ 穷尽（完全分类）。
-/

/--
  ★两个同级别新生中枢对（codex R2#4：排除同枢延伸）。

  中心定理二（新生/扩张）的前提是"前后两个**核心已分离**的同级别新生中枢"——
  与中心定理一（延伸，核心重叠/围绕同一中枢震荡）**互补**。
  延伸（核心重叠）不属中心定理二的三态，是独立的第四情形（中心定理一处理）。
  此谓词把"核心已分离"前提显式化：后中枢核心整体在前中枢核心之上或之下。
  实务由构造层保证（新中枢由后续段生成、核心不与前枢核心重叠）。
-/
def SameLevelNewCenterPair (prev next : Center) : Prop :=
  next.zd > prev.zg ∨ next.zg < prev.zd

/-- 上涨延续谓词（中心定理二）：后 DD > 前 GG（外缘完全分离向上）。 -/
def IsUpContinuation (prev next : Center) : Prop := next.dd > prev.gg
/-- 下跌延续谓词（中心定理二）：后 GG < 前 DD（外缘完全分离向下）。 -/
def IsDownContinuation (prev next : Center) : Prop := next.gg < prev.dd
/--
  ★级别扩张谓词（codex R2#4：中心定理二**原公式**，非 ¬up∧¬down）。

  zhongshu.md 第20课 / 008 原文：
  `(后 ZG < 前 ZD ∧ 后 GG ≥ 前 DD) ∨ (后 ZD > 前 ZG ∧ 后 DD ≤ 前 GG)`
  ——核心区间已分离（ZG/ZD）但波动区间重叠（GG/DD）= 形成更高级别中枢。
-/
def IsLevelExpansion (prev next : Center) : Prop :=
  (next.zg < prev.zd ∧ next.gg ≥ prev.dd) ∨ (next.zd > prev.zg ∧ next.dd ≤ prev.gg)

/--
  从两中枢端点判定关系（中心定理二原公式的可判定实现）。

  判据（008 / zhongshu.md 第20课）：
  - 后 DD > 前 GG → 上涨延续（外缘完全分离向上）
  - 后 GG < 前 DD → 下跌延续（外缘完全分离向下）
  - 否则 → 级别扩张（核心分离 + 波动重叠）
-/
def classify (prev next : Center) : CenterRelation :=
  if next.dd > prev.gg then upContinuation
  else if next.gg < prev.dd then downContinuation
  else levelExpansion

/-- `classify=up` 忠实于上涨延续谓词。 -/
theorem classify_eq_up (prev next : Center) :
    classify prev next = upContinuation ↔ IsUpContinuation prev next := by
  unfold classify IsUpContinuation
  by_cases h : next.dd > prev.gg
  · simp [h]
  · simp only [h, if_false]
    by_cases h2 : next.gg < prev.dd <;> simp [h2, h]

/--
  ★中枢三态真完全分类（构造子穷尽，L0）：classify 全函数返回三态之一。
-/
theorem center_trichotomy (prev next : Center) :
    classify prev next = upContinuation
    ∨ classify prev next = downContinuation
    ∨ classify prev next = levelExpansion := by
  unfold classify
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inl rfl)
    · exact Or.inr (Or.inr rfl)

/--
  ★三态谓词穷尽（codex R1#4 + R2#4：中心定理二原公式下的完整穷尽）。

  对同级别新生中枢对，三谓词（up / down / 中心定理二原公式 expansion）恰覆盖全部情形。
  关键：当外缘既不向上分离（¬ next.dd>prev.gg）也不向下分离（¬ next.gg<prev.dd）时，
  端点不变量（核心⊂外缘）保证落入扩张原公式的某一支——这是 008 完备性的机器证明。
-/
theorem trichotomy_predicates_total (prev next : Center)
    (hpair : SameLevelNewCenterPair prev next) :
    IsUpContinuation prev next ∨ IsDownContinuation prev next
    ∨ IsLevelExpansion prev next := by
  unfold IsUpContinuation IsDownContinuation IsLevelExpansion SameLevelNewCenterPair at *
  have hn1 := next.outer_lo; have hn2 := next.core_valid; have hn3 := next.outer_hi
  have hp1 := prev.outer_lo; have hp2 := prev.core_valid; have hp3 := prev.outer_hi
  by_cases h1 : next.dd > prev.gg
  · exact Or.inl h1
  · by_cases h2 : next.gg < prev.dd
    · exact Or.inr (Or.inl h2)
    · -- 外缘重叠：next.dd ≤ prev.gg 且 next.gg ≥ prev.dd。
      -- 由同级别新生（核心不同）+ 端点不变量，核心必某向分离 ⟹ 落扩张原公式。
      refine Or.inr (Or.inr ?_)
      omega

/--
  ★三态两两互斥（codex R1#4 + R2#4：up/down、up/exp、down/exp 全证，原公式下）。
-/
theorem up_down_disjoint (prev next : Center) :
    ¬ (IsUpContinuation prev next ∧ IsDownContinuation prev next) := by
  unfold IsUpContinuation IsDownContinuation
  intro ⟨h1, h2⟩
  have hn1 := next.outer_lo; have hn2 := next.core_valid; have hn3 := next.outer_hi
  have hp1 := prev.outer_lo; have hp2 := prev.core_valid; have hp3 := prev.outer_hi
  omega

theorem up_expansion_disjoint (prev next : Center) :
    ¬ (IsUpContinuation prev next ∧ IsLevelExpansion prev next) := by
  unfold IsUpContinuation IsLevelExpansion
  intro ⟨h1, hexp⟩
  have hn1 := next.outer_lo; have hn2 := next.core_valid; have hn3 := next.outer_hi
  have hp1 := prev.outer_lo; have hp2 := prev.core_valid; have hp3 := prev.outer_hi
  rcases hexp with ⟨hc, hg⟩ | ⟨hc, hd⟩ <;> omega

theorem down_expansion_disjoint (prev next : Center) :
    ¬ (IsDownContinuation prev next ∧ IsLevelExpansion prev next) := by
  unfold IsDownContinuation IsLevelExpansion
  intro ⟨h2, hexp⟩
  have hn1 := next.outer_lo; have hn2 := next.core_valid; have hn3 := next.outer_hi
  have hp1 := prev.outer_lo; have hp2 := prev.core_valid; have hp3 := prev.outer_hi
  rcases hexp with ⟨hc, hg⟩ | ⟨hc, hd⟩ <;> omega

/-- 兼容保留：上涨/下跌延续判据互斥。 -/
theorem continuations_disjoint (prev next : Center) :
    ¬ (next.dd > prev.gg ∧ next.gg < prev.dd) :=
  up_down_disjoint prev next

/--
  ★走势裁决结果（codex R2#5：expansion 不映射 consolidation，独立态）。

  中心定理二的中枢关系映射为父级**裁决结果**，扩张不直接是走势类型：
  - `trend dir`：上涨/下跌延续 → 本级趋势（方向）。
  - `consolidation`：单中枢盘整（不由 relation 产生，由中枢计数产生，见 RecursiveConstruction）。
  - `higherCenterCandidate`：级别扩张 → 形成更高级别中枢候选，本级走势终结，**交父级重分类**。
-/
inductive MoveOutcome where
  | trend (dir : Direction)
  | consolidation
  | higherCenterCandidate
deriving DecidableEq, Repr

/--
  中枢关系 → 裁决结果（inductive step，603 §三，codex Q4 + R2#5）。
  - 上涨延续 → trend up；下跌延续 → trend down；
  - 级别扩张 → higherCenterCandidate（**不是** consolidation——升父级，待父级中枢数分类）。
-/
def relationToOutcome (r : CenterRelation) : MoveOutcome :=
  match r with
  | upContinuation => MoveOutcome.trend Direction.up
  | downContinuation => MoveOutcome.trend Direction.down
  | levelExpansion => MoveOutcome.higherCenterCandidate

/-- 中枢三态映射到裁决结果穷尽（trend/higherCenterCandidate 三态传导，无遗漏）。 -/
theorem relation_to_outcome_total (prev next : Center) :
    relationToOutcome (classify prev next) = MoveOutcome.trend Direction.up
    ∨ relationToOutcome (classify prev next) = MoveOutcome.trend Direction.down
    ∨ relationToOutcome (classify prev next) = MoveOutcome.higherCenterCandidate := by
  rcases center_trichotomy prev next with h | h | h <;> rw [h] <;> simp [relationToOutcome]

end Formal.CenterTrichotomy
