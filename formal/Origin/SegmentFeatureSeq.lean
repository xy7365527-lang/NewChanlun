/-
Origin/SegmentFeatureSeq.lean — 线段/笔的特征序列法（task #113，缠论分类血肉港入 Origin）

★工位定位（审计 A 判决）：Origin 的 ChanlunElements.segmentsOf 是 `total_unique_of_fun`
  平凡桩（只断言"存在唯一输出"，不含任何线段判据内容）。本文件把线段划分的**真判据**
  （特征序列法）形式化为 Origin 命名空间内的可机器检查谓词/定理——分类血肉，挂 Origin 接口。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- 第67课（线段划分标准·特征序列法完整定义）：
  · "序列X1X2…Xn成为以向上笔开始线段的特征序列；序列S1S2…Sn成为以向下笔开始线段的
    特征序列。特征序列两相邻元素间没有重合区间，称为该序列的一个缺口。"（067:18）
  · 对偶性：向上线段取**向下笔**为特征序列元素，向下线段取**向上笔**为特征序列元素（067:16-18）。
  · 顶高于底硬约束："同一线段中，两端的一顶一底，顶肯定要高于底。"（067:92）
  · 第一种情况：第一、二元素间**无缺口** ⟹ 在分型极值处结束（067:26-28）。
  · 第二种情况：第一、二元素间**有缺口**，须从分型极值起的反向序列出现对偶分型才确认（067:37-38）。
- 第71课（特征序列包含关系严格性）：
  · "特征序列的元素要探讨包含关系，首先必须是同一特征序列的元素。"（071:28）
  · 分界点两侧两元素**不存在**包含关系（071:40）。
- 第78课（古怪线段 + 顶高于底）：
  · "以向上笔开始的线段一定结束于向上笔……一个线段，不可能是从底到底或从顶到顶。"（078:18）
  · "顶肯定要高于底，如果你划出一个不符合这基本要求的线段，那肯定是划错了。"（078:20）
- 第81课（新笔定义，《忽闻台风可休市》）：
  · "顶分型中最高K线和底分型的最低K线之间（不包括这两K线），不考虑包含关系，
    至少有3根（包括3根）以上K线。"（081:110-111）

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构判据，不依赖数据）。`lake env lean Origin/SegmentFeatureSeq.lean`
通过 = 特征序列法判据的良构性、缺口/包含/两情况的逻辑关系、顶高于底硬约束在定义层成立，
**不是**任何"线段划分在真实行情上有效"的实证断言（那是 L2+，须真实数据）。

诚实标注（gatekeeper）：
★ StructurePartitionOnly —— 本文件把第67/71/78课"特征序列法"的**结构形式**转写为
  Origin 谓词；证的是这些谓词的逻辑关系（缺口⊻无缺口、顶高于底的不变量），**不证**
  某个具体笔序列真对应缠论权威标注的线段（那需要真实 K 线 L2 验证）。

★ still-MISSING-A（见文件尾 § 8）：判据层 formalized，但 segmentsOf 全自动构造层
  （非包含处理递归 + 分型识别递归的终止性/唯一性）未在本文件证。本文件不冒充全实装。

禁 sorry/admit/axiom。范式：纯 Prop/Type，不依赖 Mathlib（与 ChanlunElements 同范式，Int/Nat + omega）。
不使用 min/max（Mathlib 缺失时无 simp 引理）——区间端点用 if 显式构造或 omega 直推。
-/

import Origin.ChanlunElements

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 特征序列元素（第67课对偶性）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **特征序列元素** —— 一个特征序列元素由一根笔（`Stroke`）抽象出价格区间 `[low, high]`。

  第67课对偶性：向上线段的特征序列由**向下笔**构成，向下线段的特征序列由**向上笔**构成。
  本结构只暴露特征序列分型/缺口/包含判定所需的几何数据（区间上下界）。
-/
structure FeatureElem where
  low : Tick
  high : Tick
  valid : low ≤ high
deriving Repr

/--
  **从一根笔抽象特征序列元素** —— 取笔两端价的低/高作为区间（if 显式，不用 min/max）。

  ★对偶性（第67课）由调用方保证：构造向上线段的特征序列时只喂向下笔，反之亦然。
-/
def FeatureElem.ofStroke (s : Stroke) : FeatureElem :=
  if h : s.startPrice ≤ s.endPrice then
    { low := s.startPrice, high := s.endPrice, valid := h }
  else
    { low := s.endPrice, high := s.startPrice, valid := Int.le_of_not_le h }

/--
  **特征序列方向对偶谓词（第67课）** —— 线段方向 `d` 的特征序列元素笔方向必须是 `d.flip`。

  向上线段（`up`）→ 特征序列元素是向下笔（`down`）；向下线段（`down`）→ 向上笔（`up`）。
-/
def IsFeatureStroke (segDir : Direction) (s : Stroke) : Prop :=
  s.direction = segDir.flip

theorem isFeatureStroke_up (s : Stroke) :
    IsFeatureStroke Direction.up s ↔ s.direction = Direction.down := by
  unfold IsFeatureStroke Direction.flip; rfl

theorem isFeatureStroke_down (s : Stroke) :
    IsFeatureStroke Direction.down s ↔ s.direction = Direction.up := by
  unfold IsFeatureStroke Direction.flip; rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 缺口（第67课）：两相邻元素无重合区间
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **缺口（第67课）** —— "特征序列两相邻元素间没有重合区间，称为该序列的一个缺口。"
  无重合 ⟺ `a.high < b.low ∨ b.high < a.low`。
-/
def HasGap (a b : FeatureElem) : Prop :=
  a.high < b.low ∨ b.high < a.low

instance (a b : FeatureElem) : Decidable (HasGap a b) := by
  unfold HasGap; exact inferInstanceAs (Decidable (_ ∨ _))

/--
  **重合（缺口的否定）** —— 两区间有公共点 ⟺ `a.low ≤ b.high ∧ b.low ≤ a.high`。
-/
def Overlaps (a b : FeatureElem) : Prop :=
  a.low ≤ b.high ∧ b.low ≤ a.high

/-- 缺口与重合互斥穷尽（L0，决定第一/第二种情况的二歧）。 -/
theorem gap_iff_not_overlap (a b : FeatureElem) :
    HasGap a b ↔ ¬ Overlaps a b := by
  simp only [HasGap, Overlaps, Tick]
  constructor
  · rintro (h | h) ⟨h1, h2⟩ <;> omega
  · intro h
    -- h : ¬(a.low ≤ b.high ∧ b.low ≤ a.high)；用可判定性拆为析取后 omega 各支闭合
    rcases Decidable.not_and_iff_not_or_not.mp h with h' | h'
    · exact Or.inr (by omega)
    · exact Or.inl (by omega)

/-- 缺口对称（无重合是对称关系）。 -/
theorem gap_symm (a b : FeatureElem) : HasGap a b ↔ HasGap b a := by
  simp only [HasGap, Tick]; constructor <;> (rintro (h | h)) <;> omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 包含关系（第67/71课）：同一特征序列内的非包含处理
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **包含关系（第67课非包含处理）** —— 元素 `a` 被 `b` 包含 ⟺ `b.low ≤ a.low ∧ a.high ≤ b.high`。

  ★第71课严格性前提（071:28）：包含关系**只在同一特征序列的元素之间**有意义。本谓词是纯几何
  包含；"同一特征序列"前提由序列层携带（`StandardFeatureSeq`），分界点两侧禁包含见 § 5。
-/
def Contains (b a : FeatureElem) : Prop :=
  b.low ≤ a.low ∧ a.high ≤ b.high

instance (b a : FeatureElem) : Decidable (Contains b a) := by
  unfold Contains; exact inferInstanceAs (Decidable (_ ∧ _))

/--
  **包含处理（第67课）** —— 向上特征序列处理（高高合并）取 `[较大 low, 较大 high]`；
  向下处理（低低合并）取 `[较小 low, 较小 high]`。用 if 显式，不用 min/max。
-/
def mergeInclusion (processUp : Bool) (a b : FeatureElem) : FeatureElem :=
  if processUp then
    { low := if a.low ≥ b.low then a.low else b.low
      high := if a.high ≥ b.high then a.high else b.high
      valid := by
        have ha := a.valid; have hb := b.valid
        simp only [Tick] at *
        split <;> split <;> omega }
  else
    { low := if a.low ≤ b.low then a.low else b.low
      high := if a.high ≤ b.high then a.high else b.high
      valid := by
        have ha := a.valid; have hb := b.valid
        simp only [Tick] at *
        split <;> split <;> omega }

/-- 向上包含处理后高点是两高点的较大者（高高，L0）。 -/
theorem merge_up_high (a b : FeatureElem) :
    (mergeInclusion true a b).high = (if a.high ≥ b.high then a.high else b.high) := by
  unfold mergeInclusion; rfl

/-- 向下包含处理后低点是两低点的较小者（低低，L0）。 -/
theorem merge_down_low (a b : FeatureElem) :
    (mergeInclusion false a b).low = (if a.low ≤ b.low then a.low else b.low) := by
  unfold mergeInclusion; rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 顶高于底硬约束（第67/78课）：线段端点的不变量
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **线段端点签名** —— 一个线段的方向 + 起点价 + 终点价。

  第78课：以向上笔开始的线段一定结束于向上笔——向上线段起点是底、终点是顶；
  向下线段起点是顶、终点是底。不可能从底到底或从顶到顶。
-/
structure SegEndpoints where
  dir : Direction
  startPrice : Tick
  endPrice : Tick
deriving Repr

/--
  **顶高于底硬约束（第67:92 / 078:20）** —— "同一线段中，两端的一顶一底，顶肯定要高于底。"

  向上线段：起点（底）< 终点（顶）⟺ `startPrice < endPrice`。
  向下线段：起点（顶）> 终点（底）⟺ `startPrice > endPrice`。
  违反此约束的"线段"是划错的——本谓词是合法线段的**必要条件**。
-/
def TopAboveBottom : SegEndpoints → Prop
  | { dir := Direction.up, startPrice := s, endPrice := e } => s < e
  | { dir := Direction.down, startPrice := s, endPrice := e } => s > e

/--
  **★顶高于底反退化见证（L0，非平凡）**：一个"从底到底"的伪线段违反硬约束。

  反退化例（第78课"不可能从底到底"）：声称向上线段（dir=up）但 startPrice=10 ≥ endPrice=10
  ⟹ ¬ TopAboveBottom。这证明 `TopAboveBottom` 不是平凡真——真排除了划错的线段
  （与 `total_unique_of_fun` 平凡桩对比：桩对任何输出都成立）。
-/
theorem topAboveBottom_rejects_flat_up :
    ¬ TopAboveBottom { dir := Direction.up, startPrice := 10, endPrice := 10 } := by
  unfold TopAboveBottom; decide

/-- ★反退化见证2：一个合法向上线段（10→20）满足硬约束。 -/
theorem topAboveBottom_accepts_valid_up :
    TopAboveBottom { dir := Direction.up, startPrice := 10, endPrice := 20 } := by
  unfold TopAboveBottom; decide

/-- ★反退化见证3：向下线段从顶到顶（20→20）违反硬约束。 -/
theorem topAboveBottom_rejects_flat_down :
    ¬ TopAboveBottom { dir := Direction.down, startPrice := 20, endPrice := 20 } := by
  unfold TopAboveBottom; decide

/--
  **硬约束与方向的镜像对偶（L0）** —— 把 SegEndpoints 镜像（方向翻转 + 价格取负）后，
  顶高于底约束保持。这是第78课方向对偶在不变量层的体现。
-/
def SegEndpoints.mirror (s : SegEndpoints) : SegEndpoints :=
  { dir := s.dir.flip, startPrice := -s.startPrice, endPrice := -s.endPrice }

theorem topAboveBottom_mirror (s : SegEndpoints) :
    TopAboveBottom s ↔ TopAboveBottom s.mirror := by
  cases s with
  | mk dir sp ep =>
    cases dir <;>
      simp only [TopAboveBottom, SegEndpoints.mirror, Direction.flip, Tick] <;>
      constructor <;> intro h <;> omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 标准特征序列 + 分界点禁包含（第71课严格性）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **标准特征序列（第67课）** —— 一列特征序列元素，携带"同一特征序列"前提（第71:28）。
  - `elems`：非包含处理后的元素列。
  - `processUp`：处理方向（向上线段的特征序列向上处理取高 / 向下取低）。
-/
structure StandardFeatureSeq where
  elems : List FeatureElem
  processUp : Bool
deriving Repr

/--
  **已非包含处理（第67课"标准特征序列"）** —— 相邻元素无包含关系（标准化不变量）。
-/
def IsStandardized : List FeatureElem → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => ¬ Contains a b ∧ ¬ Contains b a ∧ IsStandardized (b :: rest)

/--
  **★第71课分界点禁包含（071:40）** —— 假设转折点前后两元素**不存在**包含关系。

  形式化：分界点前元素 `pre` 与分界点后第一元素 `post`（不同特征序列）互不包含。
  ★这正是 §3 `Contains` 不混入"同一特征序列"前提的原因——跨序列包含被此处显式禁止。
-/
def BoundaryNoInclusion (pre post : FeatureElem) : Prop :=
  ¬ Contains pre post ∧ ¬ Contains post pre

/--
  **★分界点禁包含反退化见证（L0，非平凡）**：若 `pre=[0,10]` 含 `post=[2,8]`，则违反禁包含。
  这证明 `BoundaryNoInclusion` 真能否决（与平凡真对比）——第71:40 的力度呈现约束有内容。
-/
theorem boundaryNoInclusion_rejects_contained :
    ¬ BoundaryNoInclusion ⟨0, 10, by decide⟩ ⟨2, 8, by decide⟩ := by
  intro h
  exact h.1 ⟨by decide, by decide⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 特征序列分型 + 线段终结的两种情况（第67课核心判据）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **特征序列顶分型（第67:22）** —— 三相邻标准特征序列元素 `e1 e2 e3`，中间 `e2` 高点最高 ⟹ 顶分型。
  "以向上笔开始的线段的特征序列，只考察顶分型。"
-/
def IsTopFractal (e1 e2 e3 : FeatureElem) : Prop :=
  e1.high < e2.high ∧ e3.high < e2.high

/--
  **特征序列底分型（第67:22）** —— 中间 `e2` 低点最低 ⟹ 底分型。
  "以向下笔开始的线段，只考察底分型。"
-/
def IsBottomFractal (e1 e2 e3 : FeatureElem) : Prop :=
  e2.low < e1.low ∧ e2.low < e3.low

/--
  **特征序列中存在对偶分型** —— 元素列中存在连续三元素构成所需分型（顶/底）。
  用于第二种情况：反向序列须出现对偶分型才确认线段终结。
-/
def FractalInSeq (wantTop : Bool) : List FeatureElem → Prop
  | a :: b :: c :: rest =>
      (if wantTop then IsTopFractal a b c else IsBottomFractal a b c)
        ∨ FractalInSeq wantTop (b :: c :: rest)
  | _ => False

/--
  **线段终结判据·向上（第67课两种情况）** —— 向上线段（特征序列只考察顶分型）的
  前两特征序列元素 `e1 e2`（在顶分型中）+ 反向（向下起）后续序列 `revSeq`：

  - **第一种情况（067:26-28）**：`e1 e2` 间**无缺口** ⟹ 线段在顶分型高点结束（直接确认）。
  - **第二种情况（067:37-38）**：`e1 e2` 间**有缺口** ⟹ 须 `revSeq` 出现**底分型**才确认。

  ★这是 `segmentsOf` 真判据的核心：缺口的有无决定确认路径，不是平凡桩。
-/
def SegmentEndUp (e1 e2 : FeatureElem) (revSeq : List FeatureElem) : Prop :=
  if HasGap e1 e2 then FractalInSeq false revSeq else True

/-- 向下线段终结判据（对偶：底分型 + 缺口时反向顶分型确认）。 -/
def SegmentEndDown (e1 e2 : FeatureElem) (revSeq : List FeatureElem) : Prop :=
  if HasGap e1 e2 then FractalInSeq true revSeq else True

/--
  **★第一种情况无条件确认（L0）** —— `e1 e2` 无缺口 ⟹ `SegmentEndUp` 成立（不依赖反向序列）。
  这把"第一种情况直接结束"形式化为定理（067:26-28）。
-/
theorem segmentEndUp_caseOne (e1 e2 : FeatureElem) (revSeq : List FeatureElem)
    (h : ¬ HasGap e1 e2) : SegmentEndUp e1 e2 revSeq := by
  unfold SegmentEndUp; simp only [if_neg h]

/--
  **★第二种情况须反向分型（L0）** —— `e1 e2` 有缺口 ⟹ `SegmentEndUp` 成立**当且仅当**
  反向序列出现底分型（067:37-38）——缺口时不能直接确认，这是真判据（非平凡桩）。
-/
theorem segmentEndUp_caseTwo (e1 e2 : FeatureElem) (revSeq : List FeatureElem)
    (h : HasGap e1 e2) : SegmentEndUp e1 e2 revSeq ↔ FractalInSeq false revSeq := by
  unfold SegmentEndUp; simp only [if_pos h]

/--
  **★缺口决定两情况互斥穷尽（L0，反退化）** —— 任一 `(e1,e2)` 要么属第一种情况（无缺口，
  无条件确认）要么属第二种情况（有缺口，须反向分型）——互斥且穷尽。

  这证明两种情况的划分是**完全分类**（不遗漏、不重叠），不是 ad-hoc 列举。
-/
theorem segment_two_cases_exhaustive (e1 e2 : FeatureElem) (revSeq : List FeatureElem) :
    (¬ HasGap e1 e2 ∧ SegmentEndUp e1 e2 revSeq) ∨
    (HasGap e1 e2 ∧ (SegmentEndUp e1 e2 revSeq ↔ FractalInSeq false revSeq)) := by
  by_cases h : HasGap e1 e2
  · exact Or.inr ⟨h, segmentEndUp_caseTwo e1 e2 revSeq h⟩
  · exact Or.inl ⟨h, segmentEndUp_caseOne e1 e2 revSeq h⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 新笔定义（第81课《忽闻台风可休市》）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **新笔判据数据（第81:110-111）** —— 顶/底分型极值 K 线下标 + 其间 K 线数 + 是否共用 K 线。
-/
structure NewStrokeData where
  topIdx : Index
  bottomIdx : Index
  barsBetween : Nat
  noSharedBar : Bool
deriving Repr

/--
  **新笔成立（第81课）** —— 不共用 K 线 ∧ 中间至少 3 根 K 线：
  1. 顶/底分型经包含处理后不共用 K 线（与旧笔一致，绝不放松，081:110）。
  2. 顶分型最高 K 线与底分型最低 K 线之间（不含），不考虑包含关系，至少 3 根（081:111）。
-/
def IsNewStroke (d : NewStrokeData) : Prop :=
  d.noSharedBar = true ∧ d.barsBetween ≥ 3

/--
  **★新笔 vs 旧笔放宽（L0，反退化）** —— 中间恰 3 根 K 线 + 不共用 ⟹ 新笔成立。
  旧笔要求"独立K线"（更严），新笔只要 ≥3 根（不论包含）。这把"放宽"形式化为判据差。
-/
theorem isNewStroke_accepts_three :
    IsNewStroke { topIdx := 0, bottomIdx := 5, barsBetween := 3, noSharedBar := true } := by
  unfold IsNewStroke; exact ⟨rfl, Nat.le_refl 3⟩

/-- ★新笔拒绝不足 3 根（L0，反退化）：中间 2 根 ⟹ 不成立（081:111 下边界）。 -/
theorem isNewStroke_rejects_two :
    ¬ IsNewStroke { topIdx := 0, bottomIdx := 4, barsBetween := 2, noSharedBar := true } := by
  intro h; exact absurd h.2 (by decide)

/-- ★新笔拒绝共用 K 线（L0，反退化）：条件1绝不放松（081:110）。 -/
theorem isNewStroke_rejects_shared :
    ¬ IsNewStroke { topIdx := 0, bottomIdx := 5, barsBetween := 5, noSharedBar := false } := by
  intro h; exact absurd h.1 (by decide)

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. still-MISSING 诚实声明（no-patch-mentality / no-workaround）
    ═══════════════════════════════════════════════════════════════════════

  ★still-MISSING-A（segmentsOf 全自动构造层）：
    本文件形式化了线段划分的**判据层**（特征序列对偶 / 缺口 / 包含 / 两情况 / 顶高于底 /
    新笔）——每个判据都是真谓词（反退化见证证明非平凡）。但 ChanlunElements.segmentsOf
    要求 `List Stroke → List Segment` 的**全自动构造**：从原始笔序列
      (1) 按方向对偶提取特征序列元素，(2) 递归做非包含处理得标准特征序列，
      (3) 识别顶/底分型，(4) 按缺口有无走第一/第二种情况确认终结，(5) 切出线段。
    其中 (2) 非包含处理递归 + (3)(4) 分型识别递归的**终止性证明**与**输出唯一性**
    （`segmentsOf` 非平凡实例化所需）**未在本文件证**。本文件**不声称**已实装全自动
    `segmentsOf`——只声称判据层 formalized。把判据冒充为全实装 = 声明膨胀（禁止）。

  ★边界条件（结论翻转条件）：
    - 顶高于底硬约束在 `Tick = Int` 上严格不等式成立；若改用允许相等的价格度量，
      平的端点是否算线段须重新裁定（当前严格 `<`/`>`，平端点拒绝）。
    - 第二种情况"反向序列出现对偶分型"假设反向序列已正确构造；若反向序列构造错误
      （still-MISSING-A），`SegmentEndUp` 的 `FractalInSeq` 见证可能虚假成立——
      判据层正确不蕴含构造层正确。

  ★下游推论：
    - BspClassification（模块4）的"次级别走势"以线段为载体——线段判据正确是买卖点判据有意义的前提。
    - CenterStates（模块2）的中枢由"至少三个连续次级别走势（线段）重叠"构成——线段是中枢的构成元素。

  ★谱系引用：`.chanlun/genealogy/settled/001-degenerate-segment.md`（退化线段谱系）+
    `002-source-incompleteness.md`（编纂版缺第67/71/78课，已回溯博文补齐）。
-/

end NewChanlun.Origin
