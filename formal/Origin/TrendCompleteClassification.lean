/-
Origin/TrendCompleteClassification.lean

Trend classification starts before the strategy.  A current state is either an
unfinished tail, a consolidation, an upward trend, or a downward trend.  There
is no fourth completed trend kind.
-/

import Origin.ChanlunElements

namespace NewChanlun.Origin

inductive TrendClass where
  | unfinished
  | consolidation
  | trendUp
  | trendDown
deriving DecidableEq, Repr

def TrendClass.mirror : TrendClass -> TrendClass
  | TrendClass.unfinished => TrendClass.unfinished
  | TrendClass.consolidation => TrendClass.consolidation
  | TrendClass.trendUp => TrendClass.trendDown
  | TrendClass.trendDown => TrendClass.trendUp

theorem trend_mirror_involutive (c : TrendClass) : c.mirror.mirror = c := by
  cases c <;> rfl

inductive TrendOrder where
  | falling
  | flat
  | rising
deriving DecidableEq, Repr

def TrendOrder.mirror : TrendOrder -> TrendOrder
  | TrendOrder.falling => TrendOrder.rising
  | TrendOrder.flat => TrendOrder.flat
  | TrendOrder.rising => TrendOrder.falling

theorem trend_order_mirror_involutive (o : TrendOrder) : o.mirror.mirror = o := by
  cases o <;> rfl

def chooseTrendByOrder
    (complete hasTwoCenters : Bool) (order : TrendOrder) : TrendClass :=
  if complete then
    if hasTwoCenters then
      match order with
      | TrendOrder.rising => TrendClass.trendUp
      | TrendOrder.falling => TrendClass.trendDown
      | TrendOrder.flat => TrendClass.consolidation
    else
      TrendClass.consolidation
  else
    TrendClass.unfinished

theorem chooseTrendByOrder_flat_two_centers :
    chooseTrendByOrder true true TrendOrder.flat = TrendClass.consolidation := by
  rfl

theorem chooseTrendByOrder_mirror
    (complete hasTwoCenters : Bool) (order : TrendOrder) :
    chooseTrendByOrder complete hasTwoCenters order.mirror =
      (chooseTrendByOrder complete hasTwoCenters order).mirror := by
  cases complete <;> cases hasTwoCenters <;> cases order <;> rfl

def chooseTrend (complete hasTwoCenters up : Bool) : TrendClass :=
  if complete then
    if hasTwoCenters then
      if up then TrendClass.trendUp else TrendClass.trendDown
    else
      TrendClass.consolidation
  else
    TrendClass.unfinished

def TrendSpec (complete hasTwoCenters up : Bool) : TrendClass -> Prop
  | TrendClass.unfinished => complete = false
  | TrendClass.consolidation => complete = true /\ hasTwoCenters = false
  | TrendClass.trendUp => complete = true /\ hasTwoCenters = true /\ up = true
  | TrendClass.trendDown => complete = true /\ hasTwoCenters = true /\ up = false

theorem trend_spec_iff_chosen
    (complete hasTwoCenters up : Bool) (c : TrendClass) :
    TrendSpec complete hasTwoCenters up c <->
      c = chooseTrend complete hasTwoCenters up := by
  cases complete <;> cases hasTwoCenters <;> cases up <;>
  cases c <;> simp [TrendSpec, chooseTrend]

theorem trend_complete_unique
    (complete hasTwoCenters up : Bool) :
    ExistsUnique (fun c => TrendSpec complete hasTwoCenters up c) := by
  refine ⟨chooseTrend complete hasTwoCenters up, ?_, ?_⟩
  · exact
      (trend_spec_iff_chosen complete hasTwoCenters up
        (chooseTrend complete hasTwoCenters up)).2 rfl
  · intro c hc
    exact (trend_spec_iff_chosen complete hasTwoCenters up c).1 hc

theorem trend_classes_disjoint
    {complete hasTwoCenters up : Bool} {a b : TrendClass}
    (ha : TrendSpec complete hasTwoCenters up a)
    (hb : TrendSpec complete hasTwoCenters up b) :
    a = b := by
  have ha' := (trend_spec_iff_chosen complete hasTwoCenters up a).1 ha
  have hb' := (trend_spec_iff_chosen complete hasTwoCenters up b).1 hb
  exact ha'.trans hb'.symm

theorem trend_no_fourth_class
    (c : TrendClass) :
    c = TrendClass.unfinished \/
    c = TrendClass.consolidation \/
    c = TrendClass.trendUp \/
    c = TrendClass.trendDown := by
  cases c <;> simp

/--
  客观两中枢规格：解析状态中已出现的中枢数量 ≥ 2（第17课趋势定义
  「至少两个以上依次同向中枢」的计数侧必要条件，依次同向侧由更细的中枢关系判据承载）。
  供 `TrendClassifier.twoCentersSpec` 实例化——把「两中枢」判定接到可客观核查的数据上。
-/
def twoCentersByCount (s : ParseStruct) : Bool :=
  decide (2 ≤ s.centers.length)

/--
  ★判定函数 soundness 契约（#871 第二项，修复 #857 缺口）。

  此前 `isComplete` / `hasTwoCenters` / `isUp` 三个判定函数全由调用者任意提供，
  Lean 不检查它们对不对。现改为：调用者**同时**提供其声称实现的客观规格
  （`completeSpec` / `twoCentersSpec` / `upSpec`）与三条逐点一致性证明
  （`complete_sound` / `twoCenters_sound` / `up_sound`）——判定不可再与规格脱钩。

  规格本身仍是显式前提（N-2 形态，不用 axiom）：其中「两中枢」规格可客观化为
  `twoCentersByCount`（`s.centers.length ≥ 2`）；「完成」的规格由背驰定义
  （beichi.md:228-232「完成由背驰定义」），而 Origin 层无力度数据，须由调用方
  显式给出（诚实开口，同 `Turn.lean` 对 `DivergencePair` 层级缺口的处置）。
-/
structure TrendClassifier where
  isComplete : ParseStruct -> Bool
  hasTwoCenters : ParseStruct -> Bool
  isUp : ParseStruct -> Bool
  /-- 调用方声明实现的「走势完成」客观规格（由背驰定义，beichi.md:228-232）。 -/
  completeSpec : ParseStruct -> Bool
  /-- 调用方声明实现的「至少两中枢」客观规格（可实例化为 `twoCentersByCount`）。 -/
  twoCentersSpec : ParseStruct -> Bool
  /-- 调用方声明实现的「趋势向上」客观规格。 -/
  upSpec : ParseStruct -> Bool
  /-- soundness：完成判定逐点等于完成规格。 -/
  complete_sound : ∀ s, isComplete s = completeSpec s
  /-- soundness：两中枢判定逐点等于两中枢规格。 -/
  twoCenters_sound : ∀ s, hasTwoCenters s = twoCentersSpec s
  /-- soundness：向上判定逐点等于向上规格。 -/
  up_sound : ∀ s, isUp s = upSpec s

def TrendClassifier.classify (C : TrendClassifier) (s : ParseStruct) : TrendClass :=
  chooseTrend (C.isComplete s) (C.hasTwoCenters s) (C.isUp s)

/-- soundness ⟹ classify 等于按客观规格选类（三判定逐点替换后 classify 的定义式）。 -/
theorem trend_classifier_classify_by_spec (C : TrendClassifier) (s : ParseStruct) :
    C.classify s = chooseTrend (C.completeSpec s) (C.twoCentersSpec s) (C.upSpec s) := by
  unfold TrendClassifier.classify
  rw [C.complete_sound s, C.twoCenters_sound s, C.up_sound s]

/-- soundness ⟹ 分类结果满足客观规格的 `TrendSpec`（规格-类别一致性，非空转：规格由调用方
    独立给出，三判定若不逐点忠实，本结论不成立——见下方删前件自查反例）。 -/
theorem trend_classifier_satisfies_spec (C : TrendClassifier) (s : ParseStruct) :
    TrendSpec (C.completeSpec s) (C.twoCentersSpec s) (C.upSpec s) (C.classify s) := by
  exact (trend_spec_iff_chosen (C.completeSpec s) (C.twoCentersSpec s) (C.upSpec s)
      (C.classify s)).2 (trend_classifier_classify_by_spec C s)

/-- 两中枢规格客观化：`twoCentersSpec = twoCentersByCount` 时，`hasTwoCenters` 逐点等于
    中枢计数 ≥ 2 的客观判据（law 字段把判定接到可核查数据上）。 -/
theorem trend_classifier_two_centers_by_count (C : TrendClassifier) (s : ParseStruct)
    (hspec : C.twoCentersSpec = twoCentersByCount) :
    C.hasTwoCenters s = decide (2 ≤ s.centers.length) := by
  rw [C.twoCenters_sound s, hspec]
  rfl

/-! ## 删前件自查（#871 纪律：前件删掉结论还成立就是空转） -/

/-- 删前件自查材料：无 law 字段的裸三判定（#871 修复前形态，与 `TrendClassifier` 去掉
    三规格三证明后的骨架同构）。 -/
structure RawTrendJudgments where
  isComplete : ParseStruct -> Bool
  hasTwoCenters : ParseStruct -> Bool
  isUp : ParseStruct -> Bool

/-- 裸判定的分类（与 `TrendClassifier.classify` 同构，但不带任何 soundness 契约）。 -/
def RawTrendJudgments.classify (J : RawTrendJudgments) (s : ParseStruct) : TrendClass :=
  chooseTrend (J.isComplete s) (J.hasTwoCenters s) (J.isUp s)

/-- 删前件自查用的解析状态见证（空状态即可——反例只关心判定与规格脱钩）。 -/
def emptyParse : ParseStruct :=
  { mergedBars := []
    fractals := []
    strokes := []
    segments := []
    centers := []
    moves := []
    bsp := []
    tail := OpenTail.none }

/-- 与规格脱钩的裸判定：`isUp` 恒 false，而规格要求恒 true。 -/
def lyingUpJudgments : RawTrendJudgments :=
  { isComplete := fun _ => true
    hasTwoCenters := fun _ => true
    isUp := fun _ => false }

/-- 删前件自查：裸判定下 classify 与规格类别分道扬镳（trendDown ≠ trendUp）——
    若没有三条 law 字段，「分类等于按规格选类」这个结论不成立。 -/
example :
    RawTrendJudgments.classify lyingUpJudgments emptyParse = TrendClass.trendDown
    ∧ chooseTrend true true true = TrendClass.trendUp := by
  constructor <;> rfl

/-- 删前件自查（`trend_classifier_classify_by_spec` 的 law 删除版）：裸判定下 classify
    与按规格选类不等——isUp 说谎把本应按 trendUp 的分类压成 trendDown。 -/
example :
    RawTrendJudgments.classify lyingUpJudgments emptyParse
      ≠ chooseTrend true true true := by
  decide

/-- 删前件自查（`trend_classifier_satisfies_spec` 的 law 删除版）：裸判定下分类不满足其
    声称的规格 `TrendSpec true true true`——说谎判定让「规格-类别一致性」不可证。 -/
example :
    ¬ TrendSpec true true true
      (RawTrendJudgments.classify lyingUpJudgments emptyParse) := by
  intro h
  simp [RawTrendJudgments.classify, lyingUpJudgments, TrendSpec, chooseTrend] at h

/-- law 字段承重见证：`isUp` 恒 false 的判定无法在 `upSpec = 恒 true` 下通过
    `up_sound`——脱钩判定被 law 字段在类型层拒绝。 -/
example :
    ¬ ∃ C : TrendClassifier, C.isUp = (fun _ => false) ∧ C.upSpec = (fun _ => true) := by
  rintro ⟨C, h⟩
  have hs := C.up_sound emptyParse
  rw [h.1, h.2] at hs
  simp at hs

/-- 两中枢规格反例材料：带两个合法中枢的解析状态。 -/
def twoCenterParse : ParseStruct :=
  { mergedBars := []
    fractals := []
    strokes := []
    segments := []
    centers := [{ zd := 1, zg := 2, startIndex := 0, endIndex := 1, valid := by decide },
                 { zd := 3, zg := 4, startIndex := 2, endIndex := 3, valid := by decide }]
    moves := []
    bsp := []
    tail := OpenTail.none }

example : twoCentersByCount twoCenterParse = true := by decide

/-- 删前件自查材料：忠实于「恒无两中枢」错误规格的分类器（两中枢判定恒 false）。 -/
def lyingTwoCentersClassifier : TrendClassifier :=
  { isComplete := fun _ => false
    hasTwoCenters := fun _ => false
    isUp := fun _ => false
    completeSpec := fun _ => false
    twoCentersSpec := fun _ => false
    upSpec := fun _ => false
    complete_sound := fun _ => rfl
    twoCenters_sound := fun _ => rfl
    up_sound := fun _ => rfl }

/-- 删前件自查：`trend_classifier_two_centers_by_count` 的前件（twoCentersSpec = 客观规格）
    删掉后结论不成立——在客观双中枢状态上，忠实于错误规格的 hasTwoCenters = false，
    而 `decide (2 ≤ centers.length) = true`（判定与客观中枢计数脱钩）。 -/
example : ¬ ∀ (C : TrendClassifier) (s : ParseStruct),
    C.hasTwoCenters s = decide (2 ≤ s.centers.length) := by
  intro h
  have hC := h lyingTwoCentersClassifier twoCenterParse
  change false = true at hC
  cases hC

theorem trend_classifier_total_unique (C : TrendClassifier) (s : ParseStruct) :
    ExistsUnique (fun c => C.classify s = c) := by
  refine ⟨C.classify s, rfl, ?_⟩
  intro c hc
  exact hc.symm

theorem trend_mirror_equivariant
    (classA classB : ParseStruct -> TrendClass)
    (mirrorState : ParseStruct -> ParseStruct)
    (h : forall s, classB (mirrorState s) = (classA s).mirror)
    (s : ParseStruct) :
    classB (mirrorState s) = (classA s).mirror :=
  h s

structure RecursiveLevelSystem where
  State : Nat -> Type
  lift : (n : Nat) -> State n -> State (n + 1)

def RecursiveLevelSystem.at (R : RecursiveLevelSystem)
    (base : R.State 0) : (n : Nat) -> R.State n
  | 0 => base
  | n + 1 => R.lift n (R.at base n)

theorem recursive_level_self_similar
    (R : RecursiveLevelSystem) (base : R.State 0) (n : Nat) :
    R.at base (n + 1) = R.lift n (R.at base n) := by
  rfl

end NewChanlun.Origin
