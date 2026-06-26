/-
  Foundation/CompleteClassificationLimits.lean
  ── 「↔ 极限定理」：行为极小完全分类 `Classify x = Classify y ↔ BehEquiv Trace x y`
     对**非平凡缠论 trace** 不可作 canonical 核（task #91, 共有缺口3, 方向 B = 严格综合）

  ════════════════════════════════════════════════════════════════════════
  接口决策（codex binding 论断的形式化冻结）
  ════════════════════════════════════════════════════════════════════════

  `HybridStateMachine.lean` 的
    `CompleteMinimalClassification Trace Classify : ∀ x y, Classify x = Classify y ↔ BehEquiv Trace x y`
  是一条**抽象元理论**（对任意 `X Ω TraceOut C` 成立的双向核）。codex 异质审计裁定：
  它**不可作缠论 canonical 接缝**——缠论分类标签（趋势/盘整/未完成；一类/二类/三类买卖点）
  比价格轨迹行为**粗**，故 ↔ 的 **→ 方向（同类 ⟹ 同行为）对非平凡缠论 trace 失败**。

  canonical 接缝继续指向（不动，本文件不改）：
  - `Foundation/CompleteClassification.lean`：ExistsUnique（递归级唯一 recSpec_complete_unique /
    recAt_causal）+ 优先级互斥穷尽（priority_class_complete_unique）+ 全局声部唯一
    （global_class_complete_unique）+ 策略流水线存在唯一（strategy_spec_total_unique）。
  - `Strict/BSP.lean`：`no_global_classifies`（全域互斥分类不存在）+ `third_subdomain_classifies`
    （第三类本征子域真双射）+ `global_only_label_quotient`（按标签商分类）。

  `CompleteMinimalClassification` 在 `HybridStateMachine.lean` 已降级标注为
  **可选元理论 / 标签核模型**（见该文件 line 50 附近注释），不作缠论 canonical seam。

  ════════════════════════════════════════════════════════════════════════
  物证（615 概念分离的直接见证，非新假设）
  ════════════════════════════════════════════════════════════════════════

  `Formal/BSPLabels.lean` 已证（L0 结构事实）：
  - `twoB_threeB_can_coincide` (line 105)：2B/3B 可重合（maimai.md:170 V 型反转）⟹ 缠论
    端点语义 X 上存在两个**语义不同**的端点 `x_2bOnly`（仅二买）/ `x_2b3b`（二买且三买）。
  - `situationLabels_separates_coincidence` (line 328)：`situationLabels x_2bOnly = [2B]`
    ≠ `situationLabels x_2b3b = [2B,3B]`——它们在**标签集粒度**行为可区分（无塌缩）。

  `Strict/BSP.lean` 已 canonical 化的**具体缠论分类器** `IGlobal : EndpointSituation → BSPType`
  （主导类别标签，优先级 一>二>三）把这两个语义不同的端点判为**同类 type2**
  （二者 `belowLastCenter=false` ∧ `afterFirstBuy=true` ⟹ 都落 type2）——这正是「缠论分类比
  行为粗」的物质形态：粗分类抹掉了 leftCenter（是否离开中枢）这一可观测差异。

  ════════════════════════════════════════════════════════════════════════
  本文件的极限定理（认识论 L0：从已结算定义结构推导 + 谱系物证，零 L2）
  ════════════════════════════════════════════════════════════════════════

  以「能区分 2B/3B 的缠论可观测 trace」`chanlunTrace`（读出 leftCenter）为 witness：
  1. `iglobal_identifies_coincidence`：`IGlobal x_2bOnly = IGlobal x_2b3b`（缠论分类器判同类）。
  2. `chanlunTrace_separates_coincidence`：`¬ BehEquiv chanlunTrace x_2bOnly x_2b3b`（行为不同）。
  3. `iglobal_not_complete_minimal`（**主极限定理**）：`¬ CompleteMinimalClassification
     chanlunTrace IGlobal`——具体缠论分类器**不是**行为极小完全分类器（→ 方向 fail）。
  4. `chanlun_classify_not_minimal_general`（**一般化**）：对任意把这对端点判同类的缠论
     classify + 任意区分它们的 trace，↔ 均失败——失败不依赖 IGlobal 的特定选择。
  5. `degenerate_trace_makes_iff_hold`（**退化对偶 / 有效域边界**）：仅当 trace 退化为
     不区分这对端点（如 `Trace = IGlobal` 自身，无信息）时 ↔ 才可能空泛成立——确认
     有效域严格小于定义域：↔ 仅在 trace 塌缩到 classify 粒度的退化情形成立。
  6. `iglobal_violates_reverse` + `both_directions_fail`（**← 方向对称裁定**，lesson 0001
     「↔ 两个方向都兑现?」的完整否定）：← 方向（同行为⟹同类）也 fail——witness
     `y_below`/`y_notBelow`（leftCenter 同=行为同，belowLastCenter 异 ⟹ IGlobal 异）。
     缠论分类与 trace 行为是**正交**等价关系（互不包含），↔ 双向皆假。
  7. `faithful_home_is_label_quotient_not_behavior_minimal`（**忠实归宿连接**）：缠论分类器
     的合格归宿是 `Strict.SemanticQuotient`（QuotientByLabel 诚实降级），**不构造假**
     `CompleteMinimalClassification` 实例（全仓该谓词仅作引理假设参数，从未被兑现）。

  谱系：615（严格完全分类 Layer1⊊Layer2 概念分离）→ 本定理是 615 在「行为极小性」维度
  的延伸物证。231（有效域 ≠ 定义域）：↔ 的定义域=任意 trace，有效域=退化 trace。
  这是**有效域收缩**（codex binding 论断的兑现），不是裁决失败——↔ 仍是合法元理论，
  只是其经验有效域（缠论非平凡 trace）为空。

  禁 sorry/admit/axiom。纯 Prop/Type，无 Mathlib。Lean build 通过 = 逻辑正确（L0）。
-/

import HybridStateMachine
import Formal.BSPLabels
import Strict.BSP

namespace CompleteClassificationLimits

open NewChanlunHybrid
open Formal.BSPLabels
open Formal.BSPLabels.BSPType
open Formal.BSPLabels.EndpointSituation
open Strict.BSP

/-! ════════════════════════════════════════════════════════════════════════
    § 缠论可观测 trace —— 区分 2B/3B 的最小见证
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★缠论可观测 trace（X = EndpointSituation, Ω = Unit, TraceOut = Bool, L0）。

  读出端点是否「离开了最后中枢」（`leftCenter`）——这是缠论中**真实可观测**的轨迹特征：
  第三类买卖点的判据正是「次级别走势离开中枢」（maimai.md:133/145）。它对任何观察窗口
  `ω : Unit` 返回 `x.leftCenter`，从而把「未离开中枢的纯二买」与「离开中枢后回抽不入的
  二三买重合」在行为上**区分开**。

  这不是人造的区分函数——`leftCenter` 是 `EndpointSituation` 已有的语义字段（X 侧自由度），
  缠论分类器 `IGlobal` 恰恰**没有读它**（IGlobal 只看 belowLastCenter / afterFirstBuy），
  这正是「分类比行为粗」的根源。
-/
def chanlunTrace : EndpointSituation → Unit → Bool :=
  fun x _ => x.leftCenter

/-! ════════════════════════════════════════════════════════════════════════
    § 极限定理 1-2：缠论分类器判同类，但缠论 trace 区分行为
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★极限定理 1（L0）：具体缠论分类器 `IGlobal` 把两个**语义不同**的端点判为**同类**。

  `x_2bOnly`（仅二买，未离开中枢）与 `x_2b3b`（二买且三买，离开中枢后回抽不入）二者
  `belowLastCenter = false` ∧ `afterFirstBuy = true` ⟹ `IGlobal` 都返回 `type2`。
  缠论分类标签把它们抹平到同一格——「分类比行为粗」的可判定见证。
-/
theorem iglobal_identifies_coincidence :
    IGlobal x_2bOnly = IGlobal x_2b3b := by decide

/--
  ★极限定理 2（L0）：缠论可观测 trace `chanlunTrace` **区分**这两个端点的行为。

  `chanlunTrace x_2bOnly · = false`（未离开中枢）≠ `chanlunTrace x_2b3b · = true`（离开中枢）
  ⟹ `¬ BehEquiv chanlunTrace x_2bOnly x_2b3b`。即在缠论真实可观测的 leftCenter 维度上，
  这两个端点行为**不**等价——`situationLabels_separates_coincidence` 在 trace/行为层的对应。
-/
theorem chanlunTrace_separates_coincidence :
    ¬ BehEquiv chanlunTrace x_2bOnly x_2b3b := by
  intro h
  -- h : ∀ ω, chanlunTrace x_2bOnly ω = chanlunTrace x_2b3b ω；取 ω = ()，得 false = true
  have := h ()
  simp [chanlunTrace, x_2bOnly, x_2b3b] at this

/-! ════════════════════════════════════════════════════════════════════════
    § 极限定理 3：主定理 —— 缠论分类器不是行为极小完全分类器（→ 方向 fail）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★主极限定理（L0）：具体缠论分类器 `IGlobal` **不**满足
  `CompleteMinimalClassification chanlunTrace IGlobal`。

  反证：若它满足 ↔，则由 → 方向（`behavior_same_class` 的逆 / `same_class_same_behavior`）：
  `IGlobal x_2bOnly = IGlobal x_2b3b`（定理 1）⟹ `BehEquiv chanlunTrace x_2bOnly x_2b3b`。
  但定理 2 证 `¬ BehEquiv chanlunTrace x_2bOnly x_2b3b`——矛盾。

  ⟹ 在能区分 2B/3B 的缠论可观测 trace 下，`IGlobal` 的同类类不蕴含行为等价：
  **缠论分类（按标签）≠ 行为极小完全分类（按 trace 行为）**。这是 codex binding 论断
  「↔ 不可作缠论 canonical 核」的核心物证——缠论标签的等价核**真包含**行为等价核。
-/
theorem iglobal_not_complete_minimal :
    ¬ CompleteMinimalClassification chanlunTrace IGlobal := by
  intro hmin
  -- same_class_same_behavior：同类 ⟹ 同行为（↔ 的 → 方向）
  have hbeh : BehEquiv chanlunTrace x_2bOnly x_2b3b :=
    same_class_same_behavior hmin iglobal_identifies_coincidence
  exact chanlunTrace_separates_coincidence hbeh

/-! ════════════════════════════════════════════════════════════════════════
    § 极限定理 4：一般化 —— 失败不依赖 IGlobal 的特定选择
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★一般化极限定理（L0）：对**任意**满足下列两条的缠论分类器 `Classify` 与 trace `Trace`：
  - `Classify x_2bOnly = Classify x_2b3b`（把这对语义不同端点判同类——缠论标签比行为粗的
    任何分类器都如此，因为标签不读 leftCenter 与 isPullbackEnd 的全部组合）；
  - `¬ BehEquiv Trace x_2bOnly x_2b3b`（trace 能区分它们的行为——任何读到 leftCenter
    或区分 [2B]/[2B,3B] 的可观测都能做到）；

  则 `¬ CompleteMinimalClassification Trace Classify`。

  即：↔ 的失败是**结构性**的（源于 2B/3B 重合点 + 行为可区分），不依赖 `IGlobal` 的特定
  优先级选择。任何「缠论分类器 + 区分重合点的 trace」组合都使 ↔ 的 → 方向 fail。
  这把主极限定理从「IGlobal 个例」提升为「对所有粗于行为的缠论分类器成立的定理」。
-/
theorem chanlun_classify_not_minimal_general
    {TraceOut C : Type} (Trace : EndpointSituation → Unit → TraceOut)
    (Classify : EndpointSituation → C)
    (hsame : Classify x_2bOnly = Classify x_2b3b)
    (hsep : ¬ BehEquiv Trace x_2bOnly x_2b3b) :
    ¬ CompleteMinimalClassification Trace Classify := by
  intro hmin
  exact hsep (same_class_same_behavior hmin hsame)

/--
  ★一般化推论（L0）：`chanlunTrace` + 任意把这对端点判同类的缠论分类器 ⟹ ↔ fail。
  把定理 4 的 `Trace` 固定为缠论可观测 `chanlunTrace`，`hsep` 自动由定理 2 满足。
-/
theorem chanlunTrace_breaks_any_coarse_classify
    {C : Type} (Classify : EndpointSituation → C)
    (hsame : Classify x_2bOnly = Classify x_2b3b) :
    ¬ CompleteMinimalClassification chanlunTrace Classify :=
  chanlun_classify_not_minimal_general chanlunTrace Classify hsame
    chanlunTrace_separates_coincidence

/-! ════════════════════════════════════════════════════════════════════════
    § 极限定理 5：退化对偶 —— ↔ 的有效域 = 退化 trace（231 有效域 ≠ 定义域）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★退化 trace（L0）：把 trace 直接取为分类器 `IGlobal` 自身（`Trace x _ = IGlobal x`）。
  这是「trace 退化为 classify」的精确形式——观察轨迹不携带任何 classify 之外的信息。
-/
def degenerateTrace (Classify : EndpointSituation → BSPType) :
    EndpointSituation → Unit → BSPType :=
  fun x _ => Classify x

/--
  ★退化对偶 / 有效域边界（L0）：当 trace 退化为分类器自身时，↔ **空泛成立**。

  `CompleteMinimalClassification (degenerateTrace Classify) Classify`：此时
  `BehEquiv (degenerateTrace Classify) x y ↔ (∀ ω, Classify x = Classify y) ↔ Classify x = Classify y`
  （Unit 只有一个观察点），与 `Classify x = Classify y` 同义反复 ⟹ ↔ 两向均平凡成立。

  这正是有效域边界（231 有效域 ≠ 定义域）：↔ 的**定义域**是任意 trace，但其经验有效域
  **退化**到「trace = classify」的零信息情形。对**非平凡**缠论 trace（如 `chanlunTrace`
  读出 classify 未读的 leftCenter），↔ 失败（定理 3）。
  ⟹ ↔ 不可作缠论 canonical 核：canonical 核必须对真实可观测 trace 成立，而 ↔ 仅对退化
  trace 成立——codex binding 论断的形式化兑现。
-/
theorem degenerate_trace_makes_iff_hold (Classify : EndpointSituation → BSPType) :
    CompleteMinimalClassification (degenerateTrace Classify) Classify := by
  intro x y
  constructor
  · -- Classify x = Classify y ⟹ ∀ ω, degenerateTrace x ω = degenerateTrace y ω
    intro hc _
    simp [degenerateTrace, hc]
  · -- BehEquiv ⟹ 取 ω = () 得 Classify x = Classify y
    intro hb
    have := hb ()
    simpa [degenerateTrace] using this

/-! ════════════════════════════════════════════════════════════════════════
    § 极限定理 6：← 方向（行为极小性）的对称裁定 —— lesson 0001「两个方向都兑现?」
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★← 方向成立的充分条件（L0）：`Classify` 通过 `Trace` 的行为分解（行为决定分类）。

  `ClassifyFactorsThroughBehavior Trace Classify` := 行为等价 ⟹ 分类相等
  （= `CompleteMinimalClassification` 的 ← 方向 `behavior_same_class` 的逐点形式）。
  这是 lesson 0001 锋利问题「↔ 两个方向都兑现?」的 ← 方向：当且仅当分类是行为的函数时，
  「同行为 ⟹ 同类」成立。缠论分类器**一般不满足**此条件（见 `iglobal_violates_reverse`）。
-/
def ClassifyFactorsThroughBehavior
    {TraceOut C : Type} (Trace : EndpointSituation → Unit → TraceOut)
    (Classify : EndpointSituation → C) : Prop :=
  ∀ x y : EndpointSituation, BehEquiv Trace x y → Classify x = Classify y

/--
  ★← 方向定理（L0）：若 `Classify` 通过 `Trace` 行为分解，则 ← 方向（同行为⟹同类）成立。
  这是 `behavior_same_class` 的前提条件版——把「← 何时成立」精确钉为「分类是行为的函数」。
  注意这**不**蕴含 ↔（→ 方向仍可能 fail，见定理 3）——单向成立不等于双向兑现。
-/
theorem reverse_holds_iff_factors
    {TraceOut C : Type} (Trace : EndpointSituation → Unit → TraceOut)
    (Classify : EndpointSituation → C) :
    ClassifyFactorsThroughBehavior Trace Classify
    ↔ (∀ x y : EndpointSituation, BehEquiv Trace x y → Classify x = Classify y) :=
  Iff.rfl

/--
  ★← 方向反例端点（L0）：两个 `leftCenter` 相同（chanlunTrace 行为相同）但 `IGlobal`
  不同（belowLastCenter 不同）的端点。

  - `y_below`：在最后中枢下方（belowLastCenter=true）⟹ IGlobal=type1，leftCenter=false。
  - `y_notBelow`：不在中枢下方但 afterFirstBuy ⟹ IGlobal=type2，leftCenter=false。
  二者 leftCenter 同为 false ⟹ chanlunTrace 行为相同，但 IGlobal 判不同类。
-/
def y_below : EndpointSituation :=
  { afterFirstBuy := false, isPullbackEnd := false, leftCenter := false,
    retraceNotReenter := false, belowLastCenter := true, sd := Side.buy }

def y_notBelow : EndpointSituation :=
  { afterFirstBuy := true, isPullbackEnd := false, leftCenter := false,
    retraceNotReenter := false, belowLastCenter := false, sd := Side.buy }

/--
  ★← 方向被违反（L0）：`IGlobal` + `chanlunTrace` **不**满足 ← 方向。

  `y_below` 与 `y_notBelow` 行为相同（`BehEquiv chanlunTrace`，二者 leftCenter=false）
  但 `IGlobal y_below = type1 ≠ type2 = IGlobal y_notBelow`——同行为却判不同类。
  ⟹ 缠论分类器 `IGlobal` **不**是 chanlunTrace 行为的函数：← 方向（行为极小性）也 fail。

  这闭合 lesson 0001 的「↔ 两个方向都兑现?」——**两个方向都不兑现**：→ 方向由定理 3 否定
  （同类不同行为），← 方向由本定理否定（同行为不同类）。缠论分类器与 chanlunTrace 行为
  的两个等价核**互不包含**（既非更粗也非更细，而是正交切分）——↔ 双向皆假。
-/
theorem iglobal_violates_reverse :
    ¬ ClassifyFactorsThroughBehavior chanlunTrace IGlobal := by
  intro hfac
  -- y_below 与 y_notBelow 行为相同（leftCenter 均 false）
  have hbeh : BehEquiv chanlunTrace y_below y_notBelow := by
    intro _; rfl
  -- 但 IGlobal 判不同类（type1 ≠ type2）
  have hclass : IGlobal y_below = IGlobal y_notBelow := hfac y_below y_notBelow hbeh
  simp [IGlobal, y_below, y_notBelow] at hclass

/--
  ★双向皆假主定理（L0）：`IGlobal` + `chanlunTrace` 下 ↔ 的**两个方向都不成立**。

  - → 方向（同类⟹同行为）：由 `iglobal_not_complete_minimal`（定理 3）经 witness
    `x_2bOnly`/`x_2b3b` 否定。
  - ← 方向（同行为⟹同类）：由 `iglobal_violates_reverse` 经 witness `y_below`/`y_notBelow` 否定。

  这是 lesson 0001 锋利问题「↔ 两个方向都兑现?」的完整否定答案：**两个方向都未兑现**。
  缠论分类既不比行为细（→ fail），也不比行为粗（← fail）——它沿与价格轨迹行为**正交**
  的语义维度（中枢相对位置、第一类前置）切分，与 trace 行为是两个不可比的等价关系。
-/
theorem both_directions_fail :
    (¬ ∀ x y : EndpointSituation, IGlobal x = IGlobal y → BehEquiv chanlunTrace x y)
    ∧ (¬ ∀ x y : EndpointSituation, BehEquiv chanlunTrace x y → IGlobal x = IGlobal y) := by
  constructor
  · -- → 方向 fail：x_2bOnly/x_2b3b 同类但行为不同
    intro hfwd
    exact chanlunTrace_separates_coincidence
      (hfwd x_2bOnly x_2b3b iglobal_identifies_coincidence)
  · -- ← 方向 fail：y_below/y_notBelow 同行为但不同类
    exact iglobal_violates_reverse

/-! ════════════════════════════════════════════════════════════════════════
    § 极限定理 7：忠实归宿 = QuotientByLabel 诚实降级（不构造假 CompleteClassifier 实例）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★忠实归宿定理（L0，615 诚实降级路径的接口连接）：缠论分类器 `IGlobal` 的合格归宿
  是 `Strict.SemanticQuotient`（按标签商分类 = QuotientByLabel），**不是**
  `CompleteMinimalClassification`（行为极小完全分类）。

  - 正面归宿成立：`global_only_label_quotient`（Strict/BSP.lean）已证 `IGlobal` 在标签核
    `∼_label = fun x y => IGlobal x = IGlobal y` 下满足 `Strict.SemanticQuotient`——
    这是 gatekeeper 标注的 **QuotientByLabel**（complete 是同义反复，无独立行为内容）。
  - 行为完全性归宿失败：`iglobal_not_complete_minimal`（定理 3）+ `iglobal_violates_reverse`
    （定理 6）已证 `IGlobal` 在任何区分 2B/3B 的 trace 下**双向**不满足
    `CompleteMinimalClassification`。

  ⟹ 忠实路径 = 把缠论分类标注为 QuotientByLabel（按标签商），**不声称行为完全性**，
  **不构造假 `CompleteMinimalClassification` 实例**（全仓 grep 证：该谓词从未被任何具体
  缠论分类器兑现为实例——仅作 `same_class_same_behavior` 等引理的假设参数 `hmin`）。
  这把 Lead 接口决策「忠实路径=615 Layer2 标准 + gatekeeper QuotientByLabel 降级」
  钉为机器可检验的连接定理。
-/
theorem faithful_home_is_label_quotient_not_behavior_minimal :
    Strict.SemanticQuotient EndpointSituation BSPType IGlobal
      (fun x y => IGlobal x = IGlobal y)
    ∧ ¬ CompleteMinimalClassification chanlunTrace IGlobal :=
  ⟨global_only_label_quotient, iglobal_not_complete_minimal⟩

/-! ════════════════════════════════════════════════════════════════════════
    § 接口结论 —— ↔ 不是缠论行为极小完全分类器（除非 trace 退化为 classify）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★接口冻结定理（L0，本文件的总结性陈述）：存在缠论分类器 `Classify`（取 `IGlobal`）
  与缠论可观测 trace `Trace`（取 `chanlunTrace`），以及两个语义不同端点 `x y`，使
    `Classify x = Classify y`（同类）∧ `¬ BehEquiv Trace x y`（行为不同）。

  ⟹ 该 (Trace, Classify) 不满足 `CompleteMinimalClassification`：缠论 classify 不是行为
  极小完全分类器。结合定理 5（退化 trace 下 ↔ 才成立），冻结接口决策：
  **`CompleteMinimalClassification` 的 ↔ 对非平凡缠论 trace 不成立，不作 canonical 核**。
-/
theorem iff_fails_on_nontrivial_chanlun_trace :
    ∃ (Trace : EndpointSituation → Unit → Bool) (Classify : EndpointSituation → BSPType)
      (x y : EndpointSituation),
        Classify x = Classify y
        ∧ ¬ BehEquiv Trace x y
        ∧ ¬ CompleteMinimalClassification Trace Classify :=
  ⟨chanlunTrace, IGlobal, x_2bOnly, x_2b3b,
    iglobal_identifies_coincidence,
    chanlunTrace_separates_coincidence,
    iglobal_not_complete_minimal⟩

end CompleteClassificationLimits
