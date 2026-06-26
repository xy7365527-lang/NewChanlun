/-
  买卖点完全分类 = 端点非空标签集 totality（009/010 + maimai + 603 §2.3）
  ★codex 异质审计修正版 R2（session 019eff21，R2 发现 6/7 已吸收：level_id + NoDup + 全局互斥定理）

  缠论定理（已结算）：
  - 买卖点完备性定理（009，maimai.md:48，第21课）："只有第一、二、三类。" → 三构造子穷尽。
  - 升跌完备性定理（010，maimai.md:50）："任何向上下跌从三类买卖点某一类开始/结束。"
    → 任何运动起止点 = 某级别 BSP 标签集非空（totality）。
  - 互斥/重合（maimai.md:170）：1B/2B 不重合、1B/3B 不重合、**2B/3B 可重合**（V 型反转）。

  ★codex 审计修正：
  - R1 硬修正2：BSP 不是互斥 sum type——2B/3B 可重合，须非空标签集。
  - R2#6：`BSPLabel` 加 `level`（某级别三类 BSP，多级别共振）；标签集加 `NoDup`（无重复 [2B,2B]）。
  - R2#7：`WellFormedBSPLabelSet` + **全局定理**——任意合法端点满足 1B 互斥律（非仅 vReversal 平凡）。

  ★★task #60 两层架构（与 sibling 工位 Strict/Trend.lean 同构，避免 Formal↔Strict 循环依赖）：
  本文件（Formal lib，Layer1 + 本地 (A)/(B)/(C)）**不** import `Strict.Classification`——
  否则与 `Strict/Trend.lean`（已 import `Formal.*`）形成 Formal↔Strict 双库循环依赖（Lake 无法构建）。
  本文件自包含证明：互斥三分粒度失败的诚实裁定 (A) + 失败结构刻画 (B)
  + 第三类本征子域穷尽互斥 + 双射三件套 (C)。
  Canonical 内核实例化（(D) 段：`Strict.Classifies` / `Strict.SemanticQuotient`）置于
  **`Strict/BSP.lean`**（Strict lib，import 本文件 + `Strict.Classification`）——依赖方向单向 Strict→Formal。
-/

namespace Formal.BSPLabels

inductive Side where
  | buy
  | sell
deriving DecidableEq, Repr

inductive BSPType where
  | type1 | type2 | type3
deriving DecidableEq, Repr

open BSPType

/--
  ★买卖点标签（codex R2#6：加 `level`）。一个 (level, type, side) 三元组。
  `level` 表达"某级别三类 BSP"（升跌完备性 010"某级别"+ 多级别共振）。
-/
structure BSPLabel where
  level : Nat
  type : BSPType
  side : Side
deriving DecidableEq, Repr

/--
  ★端点 BSP 标签集（totality + NoDup，codex R1 硬修正2 + R2#6）。
  - `labels`：端点持有的全部标签。
  - `nonempty`：升跌完备性 010——非空（端点必是某类买卖点）。
  - `nodup`：无重复标签（R2#6：禁 [2B,2B]）。
-/
structure BSPLabelSet where
  labels : List BSPLabel
  nonempty : labels ≠ []
  nodup : labels.Nodup

namespace BSPLabelSet

/-- 是否含某级别某类某向的买卖点。 -/
def has (s : BSPLabelSet) (lvl : Nat) (t : BSPType) (sd : Side) : Bool :=
  s.labels.any (fun l => l.level = lvl ∧ l.type = t ∧ l.side = sd)

end BSPLabelSet

/--
  ★升跌完备性 totality（010，L0）：每个端点标签集非空。
  totality 由 `nonempty` 字段强制（构造 BSPLabelSet 必证 labels 非空）。
-/
theorem bsp_totality (s : BSPLabelSet) : s.labels ≠ [] := s.nonempty

/-- ★标签去重（R2#6）：标签集无重复标签。 -/
theorem bsp_nodup (s : BSPLabelSet) : s.labels.Nodup := s.nodup

/--
  ★买卖点完备性（009，L0）：任意标签类型属三构造子之一（无第四类）。
-/
theorem bsp_type_completeness (l : BSPLabel) :
    l.type = type1 ∨ l.type = type2 ∨ l.type = type3 := by
  cases h : l.type
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/--
  ★端点合法性（codex R2#7：WellFormedBSPLabelSet）。

  一个合法端点标签集（同级别 lvl、同方向 sd）满足 maimai.md:170 互斥律：
  若含 1B/1S，则不含同级同向的 2 类与 3 类（1B 在中枢下、3B 在中枢上，不可重合）。
  2B/3B 之间不受此约束（可共存）。这是对"哪些重合合法"的精确刻画。
-/
def WellFormedBSPLabelSet (s : BSPLabelSet) : Prop :=
  ∀ lvl sd, s.has lvl type1 sd = true →
    s.has lvl type2 sd = false ∧ s.has lvl type3 sd = false

/--
  ★2B/3B 可重合（codex 硬修正2，maimai.md:170 V 型反转）：合法实例。
-/
def vReversalEndpoint : BSPLabelSet where
  labels := [⟨1, type2, Side.buy⟩, ⟨1, type3, Side.buy⟩]
  nonempty := by simp
  nodup := by simp

theorem twoB_threeB_can_coincide :
    vReversalEndpoint.has 1 type2 Side.buy = true
    ∧ vReversalEndpoint.has 1 type3 Side.buy = true := by
  constructor <;> simp [BSPLabelSet.has, vReversalEndpoint]

/--
  ★全局互斥律定理（codex R2#7：任意合法端点满足，非仅 vReversal 平凡）。

  对 **任意** 满足 `WellFormedBSPLabelSet` 的端点 `s`，若含同级同向 1B 则不含同级同向 2B/3B。
  这是把 maimai.md:170 互斥律从"定义"提升为"对所有合法端点成立的定理"——
  codex R2#7 要求的全局性质，不再是 vReversal 个例的平凡满足。
-/
theorem wf_type1_exclusive (s : BSPLabelSet) (hwf : WellFormedBSPLabelSet s)
    (lvl : Nat) (sd : Side) (h1 : s.has lvl type1 sd = true) :
    s.has lvl type2 sd = false ∧ s.has lvl type3 sd = false :=
  hwf lvl sd h1

/--
  ★vReversal 是合法端点（满足 WellFormedBSPLabelSet）：2B/3B 共存不违反互斥律。
  它不含 1B，故互斥律前提为假，平凡满足——确认"2B/3B 重合"是合法的（与 1B 重合才非法）。
-/
theorem vReversal_wellformed : WellFormedBSPLabelSet vReversalEndpoint := by
  intro lvl sd h1
  simp [BSPLabelSet.has, vReversalEndpoint] at h1

/-
  ═══════════════════════════════════════════════════════════════════════════
  C4 诚实失败刻画（task #60，认识论 L0）
  ═══════════════════════════════════════════════════════════════════════════

  靶子（最严双射标准，见 tmp/chanlun-strict-classification-standard.md 第一部分A）：
  构造不变量 I : X → P，证「不变性 + 完备性(商集单射) + 可实现性」⟹ 诱导双射 X/∼ ≅ P。

  **两个粒度，两个裁定（codex#1 规则的严格兑现，绝不混淆——codex 审计 R3 吸收）**：

  | 目标 P | 形式 | C4 裁定 | 原因 |
  |--------|------|---------|------|
  | **互斥三分** {一∣二∣三}（恰好一格） | 全函数 X→CoarseType | **失败**（本节裁定） | 2B/3B 重合点无像 |
  | **非空标签集**（V 重合=持 [2B,3B]） | situationLabels: X→List | 可区分重合点（无塌缩） | 标签集容许多标签 |

  关键区分：买卖点分类在**非空标签集**粒度下并**不**因 2B/3B 重合而塌缩——
  `situationLabels` 把 V 型重合点映到 `[2B,3B]`，与纯 2B 的 `[2B]` 不同（见
  `situationLabels_separates_coincidence`）。真正失败的是把买卖点当**互斥 sum type**
  （type1∣type2∣type3 三选一）：互斥三分要求恰好一格，重合点同时占二/三两格，没有像。
  差距矩阵 C4「{6 标签}太粗」指的正是这个**互斥 sum type 粒度**，不是非空标签集。

  本节不强证全局双射（no-patch-mentality：绝不为"看起来完全"隐藏 2B/3B 重合）。三件事：
    (A) 严格裁定**互斥三分失败**：定义互斥三分商映射 `coarseClass : X → Option CoarseType`
        （"恰好一格"分类尝试），证它在重合点 `x_2b3b` 取 `none`（无像）⟹ 不存在把端点
        语义按 {一∣二∣三} 互斥分类的全函数。同时诚实证：标签集粒度**可区分**重合点。
    (B) 刻画失败结构——重合见证 + 重合的充要拓扑条件（V 型反转：1B 后凌厉上破最后中枢
        + 回抽不入该中枢，maimai.md:174 / 第21课 L58）。
    (C) 第三类**本征子域** `AfterBreakRetrace`：限定 X 为「离开中枢后回试/回抽不入中枢」，
        在该子域定义不变量 I' 并证 third_exhaustive + third_exclusive + I' **单射**——
        第三类在其本征子域内是真互斥穷尽 + 双射（局部 Layer2 成立）。

  认识论：全部 L0（从 maimai.md 已结算定义结构推导，无经验数据）。
-/

/-! ### (A) 互斥三分粒度失败 + 标签集粒度可区分 -/

/--
  ★端点语义状态（X 侧对象，L0）。

  这不是标签，是端点**相对中枢的拓扑/历史语义**——maimai.md 用来区分三类买卖点的
  真实判据被编码为字段：

  - `afterFirstBuy`：此端点之前是否已出现同向第一类买卖点（2 类的前提，maimai.md:117/119）。
  - `isPullbackEnd`：此端点是否是「第一类后再次回调/反弹」的结束点（2 类的结构，maimai.md:117）。
  - `leftCenter`：之前是否有「次级别走势离开中枢」（3 类的前提，maimai.md:133/145）。
  - `retraceNotReenter`：离开后的回试/回抽是否「不跌破 ZG / 不升破 ZD」（3 类的判据，maimai.md:133/145）。
  - `belowLastCenter`：端点是否在最后一个中枢**下方**（1 类买点在中枢下方，maimai.md:173）。
  - `sd`：买/卖方向。

  这些字段是 X 侧的**语义自由度**——互斥三分（恰好一格）比它粗（失败的根源）。
-/
structure EndpointSituation where
  afterFirstBuy : Bool
  isPullbackEnd : Bool
  leftCenter : Bool
  retraceNotReenter : Bool
  belowLastCenter : Bool
  sd : Side
deriving DecidableEq, Repr

namespace EndpointSituation

/-- 第一类买卖点判据（maimai.md:103/109）：下跌/上涨趋势末段背驰，端点在最后中枢下方（买）。 -/
def IsFirst (x : EndpointSituation) : Prop :=
  x.belowLastCenter = true ∧ x.afterFirstBuy = false ∧ x.leftCenter = false

/-- 第二类买卖点判据（maimai.md:117/127）：第一类后，再次回调/反弹的结束点。 -/
def IsSecond (x : EndpointSituation) : Prop :=
  x.afterFirstBuy = true ∧ x.isPullbackEnd = true

/-- 第三类买卖点判据（maimai.md:133/145）：离开中枢后回试/回抽不跌破 ZG / 不升破 ZD。 -/
def IsThird (x : EndpointSituation) : Prop :=
  x.leftCenter = true ∧ x.retraceNotReenter = true

instance (x : EndpointSituation) : Decidable (IsFirst x) := by
  unfold IsFirst; infer_instance
instance (x : EndpointSituation) : Decidable (IsSecond x) := by
  unfold IsSecond; infer_instance
instance (x : EndpointSituation) : Decidable (IsThird x) := by
  unfold IsThird; infer_instance

end EndpointSituation

open EndpointSituation

/--
  ★非空标签集粒度的不变量 `situationLabels`（X → List BSPLabel，L0）。

  端点 ↦ 它持有的全部买卖点类型标签——这是 `BSPLabelSet` 模型在 X 侧的投影。
  对每个满足的类型判据加入对应标签（同级别 lvl=1，方向取 x.sd）。

  **关键诚实点**（codex 审计 R3）：此粒度下 V 型重合点持有 `[2B,3B]`，与纯 2B 的
  `[2B]` **不同**——标签集**不**塌缩。即在标签集粒度，2B/3B 重合不造成信息损失
  （见 `situationLabels_separates_coincidence`）。真正失败的是更粗的互斥三分
  （见 `coarseClass`）。本函数返回裸 `List`（端点也可能空标签=非买卖点，
  totality 仅对真实运动端点成立，见 maimai.md:50 升跌完备性，此处不强加 nonempty）。
-/
def situationLabels (x : EndpointSituation) : List BSPLabel :=
  (if x.IsSecond then [⟨1, type2, x.sd⟩] else [])
  ++ (if x.IsThird then [⟨1, type3, x.sd⟩] else [])
  ++ (if x.IsFirst then [⟨1, type1, x.sd⟩] else [])

/--
  ★互斥三分类型（"恰好一格"分类的目标 P）。
  把买卖点当互斥 sum type：每个端点应恰好落入 first ∣ second ∣ third 之一。
-/
inductive CoarseType where
  | first | second | third
deriving DecidableEq, Repr

/--
  ★互斥三分商映射尝试（X → Option CoarseType，L0）。

  这是「把端点按 {一∣二∣三} 互斥三分」的判定函数。要成为合法互斥三分，它必须是
  **全函数**（每端点有像，恰好一格）。定义：恰好满足一类判据时返回对应格，否则 `none`
  （多于一格=重合无法分类 / 零格=非买卖点）。`x_2b3b` 同时满足二/三类 ⟹ `none`。
-/
def coarseClass (x : EndpointSituation) : Option CoarseType :=
  match decide x.IsFirst, decide x.IsSecond, decide x.IsThird with
  | true,  false, false => some CoarseType.first
  | false, true,  false => some CoarseType.second
  | false, false, true  => some CoarseType.third
  | _,     _,     _     => none

/--
  ★见证·X 侧两个**语义不同**的端点（L0）。

  两者都是「2 买」（afterFirstBuy ∧ isPullbackEnd），方向同为 buy；
  差异在第三类语义自由度：
  - `x_2bOnly`：回调结束点未离开中枢 → 仅 2 买（强势第二类，maimai.md:123 中枢上方/内/下方之一）。
  - `x_2b3b`  ：1 买后凌厉上破最后中枢、回抽不入中枢 → 2 买与 3 买**重合**（V 型反转，maimai.md:174）。
-/
def x_2bOnly : EndpointSituation :=
  { afterFirstBuy := true, isPullbackEnd := true,
    leftCenter := false, retraceNotReenter := false,
    belowLastCenter := false, sd := Side.buy }

def x_2b3b : EndpointSituation :=
  { afterFirstBuy := true, isPullbackEnd := true,
    leftCenter := true, retraceNotReenter := true,
    belowLastCenter := false, sd := Side.buy }

/--
  ★两个见证端点的类判据值（L0，供下游定理引用）。

  `x_2bOnly`：二买、非三买。`x_2b3b`：二买且三买。两者语义不同（leftCenter 不同）。
-/
theorem witnesses_classes :
    x_2bOnly ≠ x_2b3b
    ∧ x_2bOnly.IsSecond ∧ ¬ x_2bOnly.IsThird
    ∧ x_2b3b.IsSecond ∧ x_2b3b.IsThird := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · intro h; simp [x_2bOnly, x_2b3b] at h
  · exact ⟨rfl, rfl⟩
  · intro h; exact absurd h.1 (by decide)
  · exact ⟨rfl, rfl⟩
  · exact ⟨rfl, rfl⟩

/--
  ★诚实裁定核心见证：互斥三分商映射在重合点无像（L0）。

  `coarseClass x_2b3b = none`：V 型重合点同时满足二/三类判据，落不进"恰好一格"。
  这是"互斥三分非全函数"的可判定见证——互斥三分粒度下分类**失败**。
-/
theorem coarseClass_none_at_coincidence : coarseClass x_2b3b = none := by decide

/--
  ★诚实裁定主定理：不存在把端点语义按 {一∣二∣三} 互斥三分的全函数（L0）。

  互斥三分要求每个端点**恰好**落入一格（标准第一部分B #1：Σ 𝟙 = 1）。
  但 `x_2b3b` 同时满足 IsSecond 与 IsThird（2 买/3 买重合），违反"恰好一格"。
  ⟹ {一∣二∣三}不是端点语义 X 上的互斥三分。

  这是 codex#1 规则的严格兑现："凡只在标签层成立的分类必须命名为按标签商分类，
  不得冒充 X/∼ ≅ P"。买卖点分类是**按非空标签集商分类**（端点持有的标签集，
  V 重合=持 [2B,3B]），**不是**互斥 sum type 三分。
-/
theorem bsp_not_exclusive_trichotomy :
    ¬ (∀ x : EndpointSituation,
        (x.IsFirst ∧ ¬x.IsSecond ∧ ¬x.IsThird)
        ∨ (¬x.IsFirst ∧ x.IsSecond ∧ ¬x.IsThird)
        ∨ (¬x.IsFirst ∧ ¬x.IsSecond ∧ x.IsThird)) := by
  intro h
  have hx := h x_2b3b
  -- x_2b3b 同时是二买与三买，三种"恰好一格"情形均矛盾
  rcases hx with ⟨_, _, h3⟩ | ⟨_, _, h3⟩ | ⟨_, h2, _⟩
  · exact h3 ⟨rfl, rfl⟩
  · exact h3 ⟨rfl, rfl⟩
  · exact h2 ⟨rfl, rfl⟩

/--
  ★标签集粒度可区分重合点（诚实对偶，L0）。

  与互斥三分失败相对：在**非空标签集**粒度，`situationLabels` 把纯 2 买映到 `[2B]`、
  把 V 型重合映到 `[2B,3B]`——两者**不同**。即 2B/3B 重合在标签集粒度**不**塌缩
  （无信息损失）。这证实差距矩阵 C4 的失败专属于互斥 sum type 粒度，非标签集粒度。
-/
theorem situationLabels_separates_coincidence :
    situationLabels x_2bOnly ≠ situationLabels x_2b3b := by decide

/-! ### (B) 失败结构刻画：重合见证 + 充要拓扑条件 -/

/--
  ★重合见证（L0）：存在端点同时是二买与三买。
  这是 `twoB_threeB_can_coincide`（标签层）在**语义对象层**的对应——
  失败结构的本体：X 中确有对象落在 IsSecond ∩ IsThird。
-/
theorem second_third_coincide_witness :
    ∃ x : EndpointSituation, x.IsSecond ∧ x.IsThird :=
  ⟨x_2b3b, ⟨rfl, rfl⟩, ⟨rfl, rfl⟩⟩

/--
  ★重合的充要拓扑条件（L0，maimai.md:174 / 第21课 L58 的形式化）。

  原文："只有第二类买点与第三类买点是可能产生重合的，这种情况就是：第一类买点
  出现后，一个次级别的走势凌厉地直接上破前面下跌的最后一个中枢，然后在其上产生
  一个次级别的回抽不触及该中枢。"

  形式化：端点 x 同时是二买且三买 **当且仅当**
    (afterFirstBuy ∧ isPullbackEnd)  -- 第一类后的回调结束点（二买结构）
    ∧ (leftCenter ∧ retraceNotReenter) -- 离开最后中枢 + 回抽不入中枢（三买判据=凌厉上破后不触及）

  这把"何时重合"从模糊文字钉死为端点字段的合取——失败结构被完全刻画，
  不是无法分析的黑箱（no-workaround：矛盾被精确描述而非吞掉）。
-/
theorem coincidence_iff (x : EndpointSituation) :
    (x.IsSecond ∧ x.IsThird)
    ↔ (x.afterFirstBuy = true ∧ x.isPullbackEnd = true
        ∧ x.leftCenter = true ∧ x.retraceNotReenter = true) := by
  constructor
  · rintro ⟨⟨h1, h2⟩, ⟨h3, h4⟩⟩
    exact ⟨h1, h2, h3, h4⟩
  · rintro ⟨h1, h2, h3, h4⟩
    exact ⟨⟨h1, h2⟩, ⟨h3, h4⟩⟩

/--
  ★1 买与 2 买不重合（maimai.md:173，L0）：第一类在中枢下方且无前置一买，
  第二类要求有前置一买——前提互斥。
-/
theorem first_second_disjoint (x : EndpointSituation) :
    ¬ (x.IsFirst ∧ x.IsSecond) := by
  rintro ⟨⟨_, hf, _⟩, ⟨hs, _⟩⟩
  rw [hf] at hs; exact absurd hs (by decide)

/--
  ★1 买与 3 买不重合（maimai.md:173，L0）：第一类要求未离开中枢（leftCenter=false），
  第三类要求已离开中枢（leftCenter=true）——直接互斥。
-/
theorem first_third_disjoint (x : EndpointSituation) :
    ¬ (x.IsFirst ∧ x.IsThird) := by
  rintro ⟨⟨_, _, hf⟩, ⟨ht, _⟩⟩
  rw [hf] at ht; exact absurd ht (by decide)

/-! ### (C) 第三类本征子域：互斥穷尽双射成立 -/

/--
  ★第三类本征子域（codex#1 建议，L0）：X 限定为「离开中枢后回试/回抽不入中枢」。

  这是第三类买卖点定义（maimai.md:133/145）的**前提合取**：
  - `leftCenter = true`：次级别走势离开了中枢。
  - `retraceNotReenter = true`：回试不跌破 ZG（买）/ 回抽不升破 ZD（卖）。

  在此子域内，端点必是第三类买卖点（买或卖二选一），三分坍缩为方向二分——
  第三类在其本征子域内是真互斥穷尽。
-/
def AfterBreakRetrace (x : EndpointSituation) : Prop :=
  x.leftCenter = true ∧ x.retraceNotReenter = true

/-- 在本征子域内，端点必为第三类（IsThird 与子域条件等价，L0）。 -/
theorem afterBreakRetrace_iff_isThird (x : EndpointSituation) :
    AfterBreakRetrace x ↔ x.IsThird := Iff.rfl

/-- 子域内三类买点判据（方向 buy）。 -/
def IsThirdBuy (x : EndpointSituation) : Prop :=
  x.IsThird ∧ x.sd = Side.buy

/-- 子域内三类卖点判据（方向 sell）。 -/
def IsThirdSell (x : EndpointSituation) : Prop :=
  x.IsThird ∧ x.sd = Side.sell

/--
  ★third_exhaustive（本征子域穷尽，L0）：本征子域内每个端点是三类买点或三类卖点。

  子域条件 = IsThird，方向 sd 必属 {buy, sell}（Side 两构造子穷尽）——
  ⟹ 端点必落入 IsThirdBuy ∪ IsThirdSell。这是第三类在本征子域内的"覆盖全部对象"。
-/
theorem third_exhaustive (x : EndpointSituation) (hx : AfterBreakRetrace x) :
    IsThirdBuy x ∨ IsThirdSell x := by
  have ht : x.IsThird := (afterBreakRetrace_iff_isThird x).mp hx
  cases hsd : x.sd
  · exact Or.inl ⟨ht, hsd⟩
  · exact Or.inr ⟨ht, hsd⟩

/--
  ★third_exclusive（本征子域互斥，L0）：三类买点与三类卖点不重合。

  方向 buy 与 sell 互斥（Side 两构造子不等）——同一端点不能既买又卖。
  这是第三类在本征子域内的"不同情况不重叠"。
-/
theorem third_exclusive (x : EndpointSituation) :
    ¬ (IsThirdBuy x ∧ IsThirdSell x) := by
  rintro ⟨⟨_, hbuy⟩, ⟨_, hsell⟩⟩
  rw [hbuy] at hsell; exact absurd hsell (by decide)

/--
  ★第三类本征子域互斥穷尽分区（标准第一部分B #1：Σ 𝟙 = 1，L0）。

  合并 third_exhaustive + third_exclusive：本征子域 X' = {x | AfterBreakRetrace x}
  内每个端点**恰好**落入 IsThirdBuy ∣ IsThirdSell 之一。
  这是第三类在本征子域内的互斥穷尽分区（不冒充双射——双射另见下方三件套）。
-/
theorem third_subdomain_partition (x : EndpointSituation) (hx : AfterBreakRetrace x) :
    (IsThirdBuy x ∨ IsThirdSell x) ∧ ¬ (IsThirdBuy x ∧ IsThirdSell x) :=
  ⟨third_exhaustive x hx, third_exclusive x⟩

/-! #### 第三类本征子域的 Layer2 双射（I' 三件套，不留越界声明） -/

/--
  ★子域不变量 I'（X' → Side，L0）：第三类买卖点 ↦ 它的方向。
  这是第三类本征子域上的分类不变量，目标 P' = Side = {三买(buy), 三卖(sell)}。
-/
def thirdInvariant (x : EndpointSituation) : Side := x.sd

/--
  ★子域语义等价 ∼'（X' 上的等价关系，L0）：方向相同。

  双射标准（标准第一部分A）取 ∼' = thirdInvariant 的核：x ∼' y ↔ I'(x) = I'(y)。
  这是"对什么相同的对象分类"在本征子域的回答——第三类只按方向区分。
-/
def ThirdEquiv (x y : EndpointSituation) : Prop := thirdInvariant x = thirdInvariant y

/--
  ★I' 不变性（标准第一部分A #1，L0）：∼' 等价的端点 I' 相等。
  由 ∼' 定义直接得（∼' = I' 的核），无须额外条件。
-/
theorem thirdInvariant_invariant {x y : EndpointSituation} (h : ThirdEquiv x y) :
    thirdInvariant x = thirdInvariant y := h

/--
  ★I' 完备性 / 商集单射（标准第一部分A #2，L0）：I' 相等 ⟹ ∼' 等价。

  这是 C4 全局失败的对偶——在本征子域内完备性**成立**：I' 把方向相同的端点判为等价，
  方向不同的判为不等价，无塌缩。结合不变性 ⟹ ∼' 恰是 I' 的核（X'/∼' ↪ P' 单射）。
-/
theorem thirdInvariant_complete {x y : EndpointSituation}
    (h : thirdInvariant x = thirdInvariant y) : ThirdEquiv x y := h

/--
  ★I' 可实现性 / 满射（标准第一部分A #3，L0）：P' 每个方向都有本征子域端点实现。

  buy 由 `x_2b3b`（已在子域且方向 buy）实现；sell 由一个三类卖点端点实现。
  ⟹ I' : X' → {三买,三卖} 满射。结合不变性 + 完备性 ⟹ **X'/∼' ≅ {三买,三卖} 双射**
  （标准第一部分A 三件全证）。这是诚实裁定的正面成果：全局互斥三分失败，
  第三类本征子域 Layer2 双射成立。
-/
theorem thirdInvariant_realizable (s : Side) :
    ∃ x : EndpointSituation, AfterBreakRetrace x ∧ thirdInvariant x = s := by
  cases s with
  | buy => exact ⟨x_2b3b, ⟨rfl, rfl⟩, rfl⟩
  | sell =>
      refine ⟨{ afterFirstBuy := false, isPullbackEnd := false,
                leftCenter := true, retraceNotReenter := true,
                belowLastCenter := false, sd := Side.sell }, ⟨rfl, rfl⟩, rfl⟩

/-
  ───────────────────────────────────────────────────────────────────────────
  修复路径（escalate 至编排者/codex 的真选择，见文末 SendMessage 给 Lead）：
  **全局互斥三分**（把买卖点当 type1∣type2∣type3 sum type）不可达——重合点无像。
  恢复"全局恰好一格双射"有两条互斥路径——
    (路径 R∼) 细化目标 P'：把"二买×三买重合"显式化为独立格，
              P' = {一买, 仅二买, 仅三买, 二三买重合}×{升跌}，使 coarseClass 在 P' 上全函数+单射
              （即用更细的 P' 容纳重合，与现有 situationLabels 标签集粒度同效）。
    (路径 RP) 粗化目标 P'：合并二买/三买为"非一类买点"，{一类买点, 非一类买点}×{升跌}
              恢复互斥三分（牺牲二/三类区分）。
  本文件**不替任何一条下结论**——P' 的真价值判断（"对什么相同的对象分类"）
  是选择类，必须 escalate（no-unnecessary-escalation 四分法：选择类→/escalate）。
  注：现状已有正面双射成果——非空标签集粒度无塌缩（situationLabels_separates_coincidence）
  + 第三类本征子域 Layer2 双射（thirdInvariant 三件套）。失败专属于全局互斥 sum type 粒度。

  Canonical 内核实例化（(D) 段）见 `Strict/BSP.lean`——把上述 (A)/(B)/(C) 的结论
  实例化为 `Strict.Classifies` / `Strict.SemanticQuotient`，依赖方向单向 Strict→Formal。
  ───────────────────────────────────────────────────────────────────────────
-/

end Formal.BSPLabels
