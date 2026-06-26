/-
Origin/ClassificationGuardrail.lean

A′ Phase2 (task #100): re-anchor task #91「完全分类 ↔ 极限 + 诚实降级」to the
Origin canonical base.

  ════════════════════════════════════════════════════════════════════════
  存在论位置（A′ canonical 重锚，不是 legacy 复用）
  ════════════════════════════════════════════════════════════════════════

  `Origin.CompleteClassification` 把「完全分类」钉为一条**双向义务**：
    `CompleteClassifier.complete : ∀ x y, classify x = classify y ↔ BehEquiv trace x y`
  即分类器的纤维（同类类）必须**恰好等于**行为等价类（`BehEquiv`）。

  task #91 的极限结论（legacy `Foundation/CompleteClassificationLimits.lean` 的
  `both_directions_fail`）证：**缠论分类与价格轨迹行为正交，↔ 双向皆假**。本文件把这一
  结论**在 Origin canonical 类型上重新兑现**——witness、分类器、trace 全部落在
  `Origin.TrendClass` / `Origin.TrendClassifier` / `Origin.CompleteClassifier` 上，
  不 import legacy（`HybridStateMachine` / `Formal.BSPLabels` / `Strict.BSP`）。

  ★为何不 import 旧 `CompleteClassificationLimits`（A′ 严格性裁定）：
  旧文件 `import HybridStateMachine + Formal.BSPLabels + Strict.BSP`——把它拉进 `Origin.*`
  会让整个 legacy 树传递 import 回 Origin canonical 闭包（lakefile 故意排除的，line 77-83）。
  「重锚 Origin」= 在 Origin 自身类型上重证结构事实，**不是**把 Origin 重新锚回 legacy。
  故本文件零 legacy import；任务允许（「可」）import 旧文件但不强制，严格读法选择重证。

  ════════════════════════════════════════════════════════════════════════
  两件产出（task #100）
  ════════════════════════════════════════════════════════════════════════

  (1) **guardrail「禁 label 伪装 CompleteClassifier」**：任何只按 label 区分状态的分类器，
      当 trace 观测到 label 未读的可观测维度时，**不满足** Origin `CompleteClassifier` 的
      ↔ 完备性义务（`label_only_not_complete_classifier`）。这是「禁止用 label 商冒充行为
      极小完全分类」的机器守卫。

  (2) **Origin canonical 下重述 `both_directions_fail`**：对非平凡缠论 trend trace，↔ 的
      → 方向（同类⟹同行为）与 ← 方向（同行为⟹同类）**都失败**
      （`origin_both_directions_fail`）——缠论 trend 分类与 trace 行为是正交等价关系。

  ════════════════════════════════════════════════════════════════════════
  认识论等级：L0（纯结构，零 L2）
  ════════════════════════════════════════════════════════════════════════

  全部定理从 Origin 已结算定义 + 可判定 witness（`decide` / 构造）推导，不依赖任何真实
  缠论数据。**不声称 ↔ 在真实缠论 trace 上经验成立**（谱系 615/231：有效域 ≠ 定义域）。
  反退化见证：witness 是真实端点配置（两个语义不同的状态），↔ 双向皆假是机器可检验的
  否定性结果（缩小有效域边界），不是「未能否证」的确认性结果。

  禁 sorry/admit/axiom。纯 Prop/Type，无 Mathlib。Lean build 通过 = 逻辑正确（L0）。
-/

import Origin.TrendCompleteClassification
import Origin.CompleteClassification
import Origin.BehaviorQuotient

namespace NewChanlun.Origin

namespace Guardrail

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 最小缠论状态 —— 让「label 比行为粗」可判定见证
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★最小缠论 trend 状态（X 侧，可判定）。

  两个布尔自由度：
  - `twoCenters`：是否已构成两个中枢（趋势成立的缠论判据，maimai 趋势定义）。
  - `leftCenter`：次级别走势是否**离开了最后中枢**（第三类买卖点的真实可观测特征，
    maimai.md:133/145）——这是 label 分类器**不读**的轨迹维度。

  这两个维度正交：`leftCenter` 区分了「趋势内」与「趋势末段离开中枢」，但一个只看
  `twoCenters` 来判趋势/盘整的 label 分类器对它视而不见——「分类比行为粗」的根源。
-/
structure TrendState where
  twoCenters : Bool
  leftCenter : Bool
deriving DecidableEq, Repr

/-! ════════════════════════════════════════════════════════════════════════
    § 2. label-only 分类器 vs 行为 trace —— 两者读不同维度
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★label-only 分类器（只读 `twoCenters`，把状态映到 Origin `TrendClass`）。

  这是「按 label 区分」的精确形式：分类只依赖 `twoCenters` 一个字段——有两中枢判 `trendUp`，
  否则判 `consolidation`。它**不读** `leftCenter`。这正是缠论里「趋势 / 盘整」这层粗标签的
  形态：它分得出趋势和盘整，但分不出「趋势末段是否已离开中枢」。
-/
def labelClassify (s : TrendState) : TrendClass :=
  if s.twoCenters then TrendClass.trendUp else TrendClass.consolidation

/--
  ★行为 trace（读出 `leftCenter`，Ω = Unit，TraceOut = Bool）。

  这是缠论**真实可观测**的轨迹特征（第三类买卖点判据 = 离开中枢）。它把 label 抹掉的
  `leftCenter` 维度暴露给行为等价 `BehEquiv`。非平凡：它不退化为 `labelClassify`（读了
  label 不读的字段），故是 task 督导要求的「非平凡缠论 trace」。
-/
def behaviorTrace : TrendState -> Unit -> Bool :=
  fun s _ => s.leftCenter

/-! ════════════════════════════════════════════════════════════════════════
    § 3. → 方向 witness：同 label，异行为（label 比行为粗）
    ════════════════════════════════════════════════════════════════════════ -/

/-- 趋势内（两中枢，未离开中枢）。 -/
def s_inTrend : TrendState := { twoCenters := true, leftCenter := false }

/-- 趋势末段（两中枢，已离开中枢）。 -/
def s_leftTrend : TrendState := { twoCenters := true, leftCenter := true }

/--
  ★→ witness（L0）：label-only 分类器把这两个语义不同的状态判为**同类**。
  二者 `twoCenters = true` ⟹ `labelClassify` 都返回 `trendUp`——label 抹平 `leftCenter` 差异。
-/
theorem label_identifies_in_left :
    labelClassify s_inTrend = labelClassify s_leftTrend := by decide

/--
  ★→ witness（L0）：行为 trace **区分**这两个状态。
  `behaviorTrace s_inTrend () = false`（未离开）≠ `behaviorTrace s_leftTrend () = true`（已离开）
  ⟹ `¬ BehEquiv behaviorTrace s_inTrend s_leftTrend`。
-/
theorem behavior_separates_in_left :
    ¬ BehEquiv behaviorTrace s_inTrend s_leftTrend := by
  intro h
  have := h ()
  simp [behaviorTrace, s_inTrend, s_leftTrend] at this

/-! ════════════════════════════════════════════════════════════════════════
    § 4. ← 方向 witness：同行为，异 label（label 比行为细 / 正交）
    ════════════════════════════════════════════════════════════════════════ -/

/-- 盘整（无两中枢，未离开中枢）。 -/
def s_consolidation : TrendState := { twoCenters := false, leftCenter := false }

/--
  ★← witness（L0）：`s_inTrend` 与 `s_consolidation` 行为**相同**。
  二者 `leftCenter = false` ⟹ `behaviorTrace` 对每个 ω 都返回 `false` ⟹ 行为等价。
-/
theorem behavior_identifies_trend_consolidation :
    BehEquiv behaviorTrace s_inTrend s_consolidation := by
  intro _; rfl

/--
  ★← witness（L0）：label-only 分类器把这两个**行为相同**的状态判为**不同类**。
  `labelClassify s_inTrend = trendUp ≠ consolidation = labelClassify s_consolidation`
  ⟹ 同行为却不同 label——label 沿 `twoCenters` 维度切分，与 trace 行为正交。
-/
theorem label_separates_trend_consolidation :
    labelClassify s_inTrend ≠ labelClassify s_consolidation := by decide

/-! ════════════════════════════════════════════════════════════════════════
    § 5. 产出 (1)：guardrail —— 禁 label-only 伪装 Origin CompleteClassifier
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★guardrail 主定理（L0）：**不存在**以 `behaviorTrace` 为 trace、`labelClassify` 为
  classify 的 Origin `CompleteClassifier` 实例。

  反证：若有这样的 `CompleteClassifier`，其 `complete` 义务给出
    `classify s_inTrend = classify s_leftTrend ↔ BehEquiv behaviorTrace s_inTrend s_leftTrend`。
  由 → witness（`label_identifies_in_left`）左边成立 ⟹ 右边 `BehEquiv` 成立——
  但 `behavior_separates_in_left` 证右边不成立，矛盾。

  ⟹ label-only 分类器**无法**兑现 Origin `CompleteClassifier` 的 ↔ 完备性义务（当 trace
  观测到 label 未读的维度时）。这是「禁 label 伪装 CompleteClassifier」的机器守卫：
  `CompleteClassifier` 的 `complete` 字段是**真双向义务**，不是同义反复——只按 label 区分
  的分类器若其纤维粗于行为，则构造不出该字段（type 不可居留）。
-/
theorem label_only_not_complete_classifier :
    ¬ ∃ C : CompleteClassifier TrendState TrendClass Unit Bool,
        C.classify = labelClassify ∧ C.trace = behaviorTrace := by
  rintro ⟨C, hcls, htr⟩
  -- 从 complete 义务的 → 方向取「同类 ⟹ 同行为」
  have hbeh : BehEquiv C.trace s_inTrend s_leftTrend :=
    same_class_same_behavior C (by rw [hcls]; exact label_identifies_in_left)
  rw [htr] at hbeh
  exact behavior_separates_in_left hbeh

/--
  ★guardrail 一般化（L0）：失败不依赖 `labelClassify` 的特定选择。对**任意**把
  `s_inTrend`/`s_leftTrend` 判同类的分类器 `f`（任何不读 `leftCenter` 的 label-only 分类器
  都如此），都构造不出以 `behaviorTrace` 为 trace 的 Origin `CompleteClassifier`。

  即「禁 label 伪装」是**结构性**守卫：根源是 label 纤维粗于行为（同类不同行为），与具体
  label 函数无关。任何「label-only classify + 区分 leftCenter 的 trace」组合都被守卫拦截。
-/
theorem coarse_classify_not_complete_classifier
    {Class : Type} (f : TrendState -> Class)
    (hsame : f s_inTrend = f s_leftTrend) :
    ¬ ∃ C : CompleteClassifier TrendState Class Unit Bool,
        C.classify = f ∧ C.trace = behaviorTrace := by
  rintro ⟨C, hcls, htr⟩
  have hbeh : BehEquiv C.trace s_inTrend s_leftTrend :=
    same_class_same_behavior C (by rw [hcls]; exact hsame)
  rw [htr] at hbeh
  exact behavior_separates_in_left hbeh

/-! ════════════════════════════════════════════════════════════════════════
    § 6. label-only 分类器的诚实归宿 —— 行为商把它细化（不构造假 CompleteClassifier）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★诚实降级见证（L0）：label-only 分类器的合格归宿不是「行为极小完全分类」，而是它**真包含**
  在行为商之中——即存在两个状态行为相同（`behaviorQuotientClassify` 判同类）但 label 不同。

  这把「label 商 ≠ 行为商」钉为正面构造：行为商（`Origin.BehaviorQuotient` 的 canonical
  `CompleteClassifier`）把 `s_inTrend` 与 `s_consolidation` 判**同类**（同 leftCenter 行为），
  而 label-only 把它们判**不同类**——label 商既不细化也不粗化行为商，二者正交。
  诚实路径：label-only 分类器只能声明为「按 label 商分类」，不可冒充行为完全性。
-/
theorem label_quotient_orthogonal_to_behavior_quotient :
    behaviorQuotientClassify behaviorTrace s_inTrend
        = behaviorQuotientClassify behaviorTrace s_consolidation
    ∧ labelClassify s_inTrend ≠ labelClassify s_consolidation :=
  ⟨behavior_quotient_behavior_same_class behaviorTrace
      behavior_identifies_trend_consolidation,
   label_separates_trend_consolidation⟩

/-! ════════════════════════════════════════════════════════════════════════
    § 7. 产出 (2)：Origin canonical 下重述 both_directions_fail（↔ 双向皆假）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★Origin both_directions_fail（L0）：在非平凡缠论 trend trace `behaviorTrace` 下，
  label-only 分类器 `labelClassify` 与行为 `BehEquiv` 的 ↔ 的**两个方向都不成立**。

  - → 方向（同类 ⟹ 同行为）fail：witness `s_inTrend`/`s_leftTrend`——同 label `trendUp`
    但 trace 行为不同（一个未离开中枢，一个已离开）。
  - ← 方向（同行为 ⟹ 同类）fail：witness `s_inTrend`/`s_consolidation`——同行为（同 leftCenter）
    但 label 不同（`trendUp` ≠ `consolidation`）。

  这是 lesson 0001 锋利问题「↔ 两个方向都兑现?」的**完整否定答案**：两个方向都未兑现。
  缠论 trend 分类既不比行为细（→ fail），也不比行为粗（← fail）——它沿与价格轨迹行为
  **正交**的语义维度（中枢数量）切分，与 trace 行为（离开中枢）是两个不可比的等价关系。
  这把 legacy `both_directions_fail`（over `IGlobal`/`chanlunTrace`/`EndpointSituation`）
  在 Origin canonical 类型（`TrendClass`/`TrendState`/`BehEquiv`）上原样重锚。
-/
theorem origin_both_directions_fail :
    (¬ ∀ x y : TrendState,
        labelClassify x = labelClassify y → BehEquiv behaviorTrace x y)
    ∧ (¬ ∀ x y : TrendState,
        BehEquiv behaviorTrace x y → labelClassify x = labelClassify y) := by
  constructor
  · -- → 方向 fail
    intro hfwd
    exact behavior_separates_in_left
      (hfwd s_inTrend s_leftTrend label_identifies_in_left)
  · -- ← 方向 fail
    intro hrev
    exact label_separates_trend_consolidation
      (hrev s_inTrend s_consolidation behavior_identifies_trend_consolidation)

/--
  ★接口冻结定理（L0，本文件总结性陈述）：存在 Origin trend 分类器 `labelClassify` 与非平凡
  缠论 trace `behaviorTrace`，及两个语义不同状态 `x y`，使
    `labelClassify x = labelClassify y`（同类）∧ `¬ BehEquiv behaviorTrace x y`（行为不同）。

  ⟹ 该 (trace, classify) 构造不出 Origin `CompleteClassifier`（guardrail 主定理）。
  冻结接口决策：**Origin `CompleteClassifier` 的 ↔ 对非平凡缠论 trend trace 不可兑现，
  label-only 分类器不作 canonical 行为完全分类核**——它的诚实归宿是「按 label 商分类」。
-/
theorem iff_fails_on_nontrivial_origin_trend_trace :
    ∃ (trace : TrendState -> Unit -> Bool) (classify : TrendState -> TrendClass)
      (x y : TrendState),
        classify x = classify y
        ∧ ¬ BehEquiv trace x y
        ∧ ¬ ∃ C : CompleteClassifier TrendState TrendClass Unit Bool,
              C.classify = classify ∧ C.trace = trace :=
  ⟨behaviorTrace, labelClassify, s_inTrend, s_leftTrend,
    label_identifies_in_left,
    behavior_separates_in_left,
    label_only_not_complete_classifier⟩

end Guardrail

end NewChanlun.Origin
