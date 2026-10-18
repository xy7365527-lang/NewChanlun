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
  · 分界点两侧两元素**不存在**包含关系（071:42）。
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

instance (a b : FeatureElem) : Decidable (Overlaps a b) := by
  unfold Overlaps; exact inferInstanceAs (Decidable (_ ∧ _))

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
  **★第71课分界点禁包含（071:42）** —— 假设转折点前后两元素**不存在**包含关系。

  形式化：分界点前元素 `pre` 与分界点后第一元素 `post`（不同特征序列）互不包含。
  ★这正是 §3 `Contains` 不混入"同一特征序列"前提的原因——跨序列包含被此处显式禁止。
-/
def BoundaryNoInclusion (pre post : FeatureElem) : Prop :=
  ¬ Contains pre post ∧ ¬ Contains post pre

/--
  **★分界点禁包含反退化见证（L0，非平凡）**：若 `pre=[0,10]` 含 `post=[2,8]`，则违反禁包含。
  这证明 `BoundaryNoInclusion` 真能否决（与平凡真对比）——第71:42 的力度呈现约束有内容。
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

/-! ═══════════════════════════════════════════════════════════════
    § 6.1 第一种情况：先破判据与转折点窗三态（#1490 X-31，#1495）
    ═══════════════════════════════════════════════════════════════

    口径来源：`.chanlun/definitions/xianduan.md` §7.1 第 1／3／4 条与 §15 X-31；
    #1490 裁定 Q2-c（先破判据与严格分型谓词的等价性推导）。
    原文锚：`docs/chanlun/text/blog/071-第71课.md` :30／:32／:38／:40／:42【正文】
    （第一种情况的确认方式；转折点前后两元素之间不作包含处理）。

    · 第一种情况确认＝先破判据：候选转折点 b 之后按序只看同向特征序列元素；
      向上段里先出现低点跌破 b.low 的元素即确认，先出现高点超过 b.high 的元素
      即候选被新极值杀掉，被 b 包含的元素跳过，前缀结束仍未分出则窗开放。
      向下段镜像。
    · 转折点窗三态：CONFIRMED／KILLED_BY_NEW_EXTREME／OPEN_AT_PREFIX_END，
      对应 `TurnWindowState.confirmed／killedByNewExtreme／openAtPrefixEnd`。
    · 等价引理：c 取窗中首个未被跳过的元素（故 b 不包含 c），且无并列输入
      （c 与 b 的相关端点价不等）时，先破判据与第 67 课严格分型谓词
      `IsTopFractal`／`IsBottomFractal` 等价；并列情形（c 与 b 同高／同低）
      由先破判据给出裁决，不要求与严格谓词等价。
    · #799：本节第一种情况只按先破判据确认；严格谓词保持原样，用于本节等价锁，
      不作为确认失败后的备用判据。既有 SegmentEndUp/Down 与历史消费者未迁移，
      本节不声称全 Lean 构造链已接入这一新定义（#1495 的范围是定义与引理）。

    #1419 同树边界 D_FIRST_RUNTIME_BINDING：固定 545e 的 CONSTRUCTION_RULE 虽为
    segment-second/2，feature_seq.rs::append 仍先 push 试探分型、失败后包含合并，
    scan_trigger 仍取 is_fractal_and_gap 的严格分型；没有本节转折点窗的状态载体。
    因此本节落实已裁 #1490 定义，不冒充当前 Rust 第一种情况的行为等价证明。
    还缺冻结 a/b、窗成员身份、首次出窗与旧段延续到正式段端的桥；不是第二终结的 K1。
-/

/-- **第一种情况确认＝先破判据·向上段（xianduan.md §7.1 第 1 条，#1490 X-31）**

转折点 `b` 是向上段特征序列的最高元素：`a.high < b.high`，其后元素 `c` 未创新高
（`c.high ≤ b.high`，并列不算创新高），且先跌破 `b.low`（`c.low < b.low`）。 -/
def FirstBreakConfirmsUp (a b c : FeatureElem) : Prop :=
  a.high < b.high ∧ c.high ≤ b.high ∧ c.low < b.low

/-- **第一种情况确认＝先破判据·向下段（对偶）** -/
def FirstBreakConfirmsDown (a b c : FeatureElem) : Prop :=
  b.low < a.low ∧ b.low ≤ c.low ∧ b.high < c.high

instance (a b c : FeatureElem) : Decidable (FirstBreakConfirmsUp a b c) := by
  unfold FirstBreakConfirmsUp; infer_instance

instance (a b c : FeatureElem) : Decidable (FirstBreakConfirmsDown a b c) := by
  unfold FirstBreakConfirmsDown; infer_instance

/-- **转折点窗三态（xianduan.md §7.1 第 3／4 条，#1490 X-31）**

* `confirmed`：转折点之后先破对侧端点，第一种情况确认；
* `killedByNewExtreme`：先出现新极值，原转折点作废；
* `openAtPrefixEnd`：走完函数所接收的列表仍未裁出。投影列表不能冒充原始笔前缀；
  原始前缀须用下文 rawTurnWindowState，保留反向尾笔的越界信息（#1419 独审 P2）。 -/
inductive TurnWindowState
  | confirmed
  | killedByNewExtreme
  | openAtPrefixEnd
  deriving DecidableEq, Repr

/-- 给定同向投影列表的向上段转折点窗分类：先破新高 ⟹ killed；先破 `b.low` ⟹ confirmed；
被 `b` 包含的元素跳过；走完给定列表 ⟹ open。此处没有原始前缀完备性保证。
输入按时间排列、与 b 同向（向下笔），不把窗成员合并进 b。若某笔同时越过两端，
其起点 high 在终点 low 之前到达，故先判 killed；笔方向前提沿用 §1 的调用方约束。 -/
def turnWindowStateUp (b : FeatureElem) : List FeatureElem → TurnWindowState
  | [] => .openAtPrefixEnd
  | x :: rest =>
    if b.high < x.high then .killedByNewExtreme
    else if x.low < b.low then .confirmed
    else turnWindowStateUp b rest

/-- 向下段转折点窗（对偶）：先破新低 ⟹ killed；先破 `b.high` ⟹ confirmed。 -/
def turnWindowStateDown (b : FeatureElem) : List FeatureElem → TurnWindowState
  | [] => .openAtPrefixEnd
  | x :: rest =>
    if x.low < b.low then .killedByNewExtreme
    else if b.high < x.high then .confirmed
    else turnWindowStateDown b rest

theorem firstBreakUp_iff_topFractal (a b c : FeatureElem)
    (hnc : ¬ Contains b c) (hne : c.high ≠ b.high) :
    FirstBreakConfirmsUp a b c ↔ IsTopFractal a b c := by
  unfold FirstBreakConfirmsUp IsTopFractal
  unfold Contains at hnc
  constructor
  · intro ⟨h1, h2, _⟩
    exact ⟨h1, by (try simp only [Tick] at *); omega⟩
  · intro ⟨h1, h2⟩
    have h3 : ¬ (b.low ≤ c.low) := fun hle => hnc ⟨hle, by (try simp only [Tick] at *); omega⟩
    exact ⟨h1, by (try simp only [Tick] at *); omega, by (try simp only [Tick] at *); omega⟩

theorem firstBreakDown_iff_bottomFractal (a b c : FeatureElem)
    (hnc : ¬ Contains b c) (hne : c.low ≠ b.low) :
    FirstBreakConfirmsDown a b c ↔ IsBottomFractal a b c := by
  unfold FirstBreakConfirmsDown IsBottomFractal
  unfold Contains at hnc
  constructor
  · intro ⟨h1, h2, _⟩
    exact ⟨h1, by (try simp only [Tick] at *); omega⟩
  · intro ⟨h1, h2⟩
    have h3 : ¬ (c.high ≤ b.high) := fun hle => hnc ⟨by (try simp only [Tick] at *); omega, hle⟩
    exact ⟨h1, by (try simp only [Tick] at *); omega, by (try simp only [Tick] at *); omega⟩

theorem firstBreakUp_tie_confirms (a b c : FeatureElem)
    (h1 : a.high < b.high) (h2 : c.high = b.high) (h3 : c.low < b.low) :
    FirstBreakConfirmsUp a b c ∧ ¬ IsTopFractal a b c := by
  unfold FirstBreakConfirmsUp IsTopFractal
  exact ⟨⟨h1, by (try simp only [Tick] at *); omega, h3⟩, fun ⟨_, h⟩ => by (try simp only [Tick] at *); omega⟩

theorem firstBreakDown_tie_confirms (a b c : FeatureElem)
    (h1 : b.low < a.low) (h2 : c.low = b.low) (h3 : b.high < c.high) :
    FirstBreakConfirmsDown a b c ∧ ¬ IsBottomFractal a b c := by
  unfold FirstBreakConfirmsDown IsBottomFractal
  exact ⟨⟨h1, by (try simp only [Tick] at *); omega, h3⟩, fun ⟨_, h⟩ => by (try simp only [Tick] at *); omega⟩

theorem turnWindowStateUp_contained_skip (b x : FeatureElem) (rest : List FeatureElem)
    (h : Contains b x) :
    turnWindowStateUp b (x :: rest) = turnWindowStateUp b rest := by
  unfold Contains at h
  have h1 : ¬ b.high < x.high := by (try simp only [Tick] at *); omega
  have h2 : ¬ x.low < b.low := by (try simp only [Tick] at *); omega
  simp only [turnWindowStateUp, if_neg h1, if_neg h2]

theorem turnWindowStateDown_contained_skip (b x : FeatureElem) (rest : List FeatureElem)
    (h : Contains b x) :
    turnWindowStateDown b (x :: rest) = turnWindowStateDown b rest := by
  unfold Contains at h
  have h1 : ¬ x.low < b.low := by (try simp only [Tick] at *); omega
  have h2 : ¬ b.high < x.high := by (try simp only [Tick] at *); omega
  simp only [turnWindowStateDown, if_neg h1, if_neg h2]

theorem turnWindowStateUp_singleton_confirmed (b c : FeatureElem) :
    turnWindowStateUp b [c] = .confirmed ↔ c.high ≤ b.high ∧ c.low < b.low := by
  by_cases h1 : b.high < c.high <;> by_cases h2 : c.low < b.low <;>
    simp [turnWindowStateUp, h1, h2] <;> (try simp only [Tick] at *) <;> omega

theorem turnWindowStateDown_singleton_confirmed (b c : FeatureElem) :
    turnWindowStateDown b [c] = .confirmed ↔ b.low ≤ c.low ∧ b.high < c.high := by
  by_cases h1 : c.low < b.low <;> by_cases h2 : b.high < c.high <;>
    simp [turnWindowStateDown, h1, h2] <;> (try simp only [Tick] at *) <;> omega

theorem turnWindowStateUp_singleton_iff_firstBreak (a b c : FeatureElem)
    (hab : a.high < b.high) :
    turnWindowStateUp b [c] = .confirmed ↔ FirstBreakConfirmsUp a b c := by
  rw [turnWindowStateUp_singleton_confirmed]
  unfold FirstBreakConfirmsUp
  exact ⟨fun h => ⟨hab, h⟩, fun h => h.2⟩

theorem turnWindowStateDown_singleton_iff_firstBreak (a b c : FeatureElem)
    (hab : b.low < a.low) :
    turnWindowStateDown b [c] = .confirmed ↔ FirstBreakConfirmsDown a b c := by
  rw [turnWindowStateDown_singleton_confirmed]
  unfold FirstBreakConfirmsDown
  exact ⟨fun h => ⟨hab, h⟩, fun h => h.2⟩

theorem turnWindowStateUp_append_of_ne_open (b : FeatureElem) (xs ys : List FeatureElem)
    (h : turnWindowStateUp b xs ≠ .openAtPrefixEnd) :
    turnWindowStateUp b (xs ++ ys) = turnWindowStateUp b xs := by
  induction xs with
  | nil => exact (h rfl).elim
  | cons x rest ih =>
    simp only [List.cons_append, turnWindowStateUp] at h ⊢
    by_cases h1 : b.high < x.high
    · simp [h1]
    · by_cases h2 : x.low < b.low
      · simp [h1, h2]
      · simp only [if_neg h1, if_neg h2] at h ⊢
        exact ih h

theorem turnWindowStateDown_append_of_ne_open (b : FeatureElem) (xs ys : List FeatureElem)
    (h : turnWindowStateDown b xs ≠ .openAtPrefixEnd) :
    turnWindowStateDown b (xs ++ ys) = turnWindowStateDown b xs := by
  induction xs with
  | nil => exact (h rfl).elim
  | cons x rest ih =>
    simp only [List.cons_append, turnWindowStateDown] at h ⊢
    by_cases h1 : x.low < b.low
    · simp [h1]
    · by_cases h2 : b.high < x.high
      · simp [h1, h2]
      · simp only [if_neg h1, if_neg h2] at h ⊢
        exact ih h

theorem turnWindowState_exhaustive (s : TurnWindowState) :
    s = .confirmed ∨ s = .killedByNewExtreme ∨ s = .openAtPrefixEnd := by
  cases s <;> simp

/-- #1419：第一种情况同一特征笔越过两端时，新极值优先（向上旧段的特征笔向下）。 -/
theorem turnWindowStateUp_both_breached (b c : FeatureElem) (rest : List FeatureElem)
    (hh : b.high < c.high) (_hl : c.low < b.low) :
    turnWindowStateUp b (c :: rest) = .killedByNewExtreme := by
  simp [turnWindowStateUp, hh]

/-- 向下旧段的特征笔向上；同笔先到 low 一侧的新极值，亦作废。 -/
theorem turnWindowStateDown_both_breached (b c : FeatureElem) (rest : List FeatureElem)
    (hl : c.low < b.low) (_hh : b.high < c.high) :
    turnWindowStateDown b (c :: rest) = .killedByNewExtreme := by
  simp [turnWindowStateDown, hl]

example : turnWindowStateUp ⟨0, 10, by decide⟩ [⟨2, 8, by decide⟩] = .openAtPrefixEnd := by
  decide

example : turnWindowStateUp ⟨0, 10, by decide⟩ [⟨-2, 10, by decide⟩] = .confirmed := by
  decide

example : turnWindowStateUp ⟨0, 10, by decide⟩ [⟨5, 12, by decide⟩] = .killedByNewExtreme := by
  decide

example : turnWindowStateDown ⟨0, 10, by decide⟩ [⟨0, 12, by decide⟩] = .confirmed := by
  decide

/-- 首个出窗元素。此前元素全是窗成员；其区间不并入 b（#1490 §7.1 第 3 条）。
这给等价引理的 `¬ Contains b c` 提供可达域来源，而不是另设过滤条件。 -/
def firstOutsideWindow (b : FeatureElem) : List FeatureElem → Option FeatureElem
  | [] => none
  | x :: rest => if Contains b x then firstOutsideWindow b rest else some x

theorem firstOutsideWindow_not_contains (b c : FeatureElem) (xs : List FeatureElem)
    (h : firstOutsideWindow b xs = some c) : ¬ Contains b c := by
  induction xs with
  | nil => simp [firstOutsideWindow] at h
  | cons x rest ih =>
    by_cases hx : Contains b x
    · exact ih (by simpa [firstOutsideWindow, hx] using h)
    · have hc : x = c := by simpa [firstOutsideWindow, hx] using h
      exact hc ▸ hx

theorem turnWindowStateUp_firstOutside (b c : FeatureElem) (xs : List FeatureElem)
    (h : firstOutsideWindow b xs = some c) :
    turnWindowStateUp b xs = turnWindowStateUp b [c] := by
  induction xs with
  | nil => simp [firstOutsideWindow] at h
  | cons x rest ih =>
    by_cases hx : Contains b x
    · rw [turnWindowStateUp_contained_skip b x rest hx]
      exact ih (by simpa [firstOutsideWindow, hx] using h)
    · have hc : x = c := by simpa [firstOutsideWindow, hx] using h
      subst c
      have hout : b.high < x.high ∨ x.low < b.low := by
        simp only [Contains, Tick] at hx ⊢
        omega
      rcases hout with hh | hl
      · simp [turnWindowStateUp, hh]
      · simp [turnWindowStateUp, hl]

theorem turnWindowStateDown_firstOutside (b c : FeatureElem) (xs : List FeatureElem)
    (h : firstOutsideWindow b xs = some c) :
    turnWindowStateDown b xs = turnWindowStateDown b [c] := by
  induction xs with
  | nil => simp [firstOutsideWindow] at h
  | cons x rest ih =>
    by_cases hx : Contains b x
    · rw [turnWindowStateDown_contained_skip b x rest hx]
      exact ih (by simpa [firstOutsideWindow, hx] using h)
    · have hc : x = c := by simpa [firstOutsideWindow, hx] using h
      subst c
      have hout : x.low < b.low ∨ b.high < x.high := by
        simp only [Contains, Tick] at hx ⊢
        omega
      rcases hout with hl | hh
      · simp [turnWindowStateDown, hl]
      · simp [turnWindowStateDown, hh]

/-- 投影列表 OPEN 正好是该列表每个元素都在 b 范围内；相等不构成破（071:32）。
未进入列表的反向尾笔仍可能越过 P，不能据本定理宣布原始前缀 OPEN（#1419）。 -/
theorem turnWindowStateUp_open_iff (b : FeatureElem) (xs : List FeatureElem) :
    turnWindowStateUp b xs = .openAtPrefixEnd ↔ ∀ x ∈ xs, Contains b x := by
  induction xs with
  | nil => simp [turnWindowStateUp]
  | cons x rest ih =>
    by_cases hh : b.high < x.high <;> by_cases hl : x.low < b.low <;>
      simp [turnWindowStateUp, hh, hl, Contains, Tick, ih] <;>
      simp only [Tick] at * <;> omega

theorem turnWindowStateDown_open_iff (b : FeatureElem) (xs : List FeatureElem) :
    turnWindowStateDown b xs = .openAtPrefixEnd ↔ ∀ x ∈ xs, Contains b x := by
  induction xs with
  | nil => simp [turnWindowStateDown]
  | cons x rest ih =>
    by_cases hl : x.low < b.low <;> by_cases hh : b.high < x.high <;>
      simp [turnWindowStateDown, hh, hl, Contains, Tick, ih] <;>
      simp only [Tick] at * <;> omega

/-- 给定同向投影列表的第一种确认：a、b 无缺口，b 是候选峰，列表内先破确认。
这里带入列表内的先后；不含被方向过滤丢弃的笔，不能冒称完整原始前缀（#1419）。 -/
def FirstKindConfirmsUp (a b : FeatureElem) (xs : List FeatureElem) : Prop :=
  ¬ HasGap a b ∧ a.high < b.high ∧ turnWindowStateUp b xs = .confirmed

/-- 给定同向投影列表的第一种确认之向下对偶。 -/
def FirstKindConfirmsDown (a b : FeatureElem) (xs : List FeatureElem) : Prop :=
  ¬ HasGap a b ∧ b.low < a.low ∧ turnWindowStateDown b xs = .confirmed

/-- 在真实首个出窗元素上，无并列时先破确认等价于 067 严格顶分型。
`¬ Contains` 由首个出窗的定义证明；缺口条件在两边完全相同。 -/
theorem firstKindUp_iff_topFractal (a b c : FeatureElem) (xs : List FeatureElem)
    (hfirst : firstOutsideWindow b xs = some c) (hne : c.high ≠ b.high) :
    FirstKindConfirmsUp a b xs ↔ ¬ HasGap a b ∧ IsTopFractal a b c := by
  unfold FirstKindConfirmsUp
  rw [turnWindowStateUp_firstOutside b c xs hfirst,
    turnWindowStateUp_singleton_confirmed]
  change (¬ HasGap a b ∧ FirstBreakConfirmsUp a b c) ↔ _
  rw [firstBreakUp_iff_topFractal a b c (firstOutsideWindow_not_contains b c xs hfirst) hne]

theorem firstKindDown_iff_bottomFractal (a b c : FeatureElem) (xs : List FeatureElem)
    (hfirst : firstOutsideWindow b xs = some c) (hne : c.low ≠ b.low) :
    FirstKindConfirmsDown a b xs ↔ ¬ HasGap a b ∧ IsBottomFractal a b c := by
  unfold FirstKindConfirmsDown
  rw [turnWindowStateDown_firstOutside b c xs hfirst,
    turnWindowStateDown_singleton_confirmed]
  change (¬ HasGap a b ∧ FirstBreakConfirmsDown a b c) ↔ _
  rw [firstBreakDown_iff_bottomFractal a b c
    (firstOutsideWindow_not_contains b c xs hfirst) hne]

/-- 窗成员若被当作 c，严格分型可以成立而先破尚未确认；说明可达域不可省略。 -/
example : IsTopFractal ⟨0, 8, by decide⟩ ⟨2, 10, by decide⟩ ⟨3, 9, by decide⟩ ∧
    turnWindowStateUp ⟨2, 10, by decide⟩ [⟨3, 9, by decide⟩] = .openAtPrefixEnd := by
  constructor
  · unfold IsTopFractal; decide
  · decide

/-- 确认与作废都取首次出窗；相同元素集合的相反时间顺序给出不同终态。 -/
example : turnWindowStateUp ⟨0, 10, by decide⟩
    [⟨2, 12, by decide⟩, ⟨-2, 8, by decide⟩] = .killedByNewExtreme ∧
    turnWindowStateUp ⟨0, 10, by decide⟩
    [⟨-2, 8, by decide⟩, ⟨2, 12, by decide⟩] = .confirmed := by
  decide

/-! #1419 / #1490：原始笔前缀与同向投影之间的边界。
以下输入是 b 之后按时间排列的全部已知笔，不含 b。分类函数不构造候选 a/b，
也不证明这些笔来自真实 K 线；候选来源、连续性和段端门仍属于 D_FIRST_RUNTIME_BINDING。
反向笔（包括原始前缀最后一笔）不能确认，但其 P 侧原始极值不能被投影丢弃。 -/

/-- 按实际 Stroke.direction 过滤，端点价格经既有 ofStroke 投影；不以奇偶代替方向。 -/
def projectStrokes (d : Direction) : List Stroke → List FeatureElem
  | [] => []
  | s :: rest =>
      if s.direction = d then FeatureElem.ofStroke s :: projectStrokes d rest
      else projectStrokes d rest

/-- 同一投影判据的方向入口。 -/
def projectedTurnWindow (d : Direction) (b : FeatureElem) (xs : List FeatureElem) :
    TurnWindowState :=
  match d with
  | .up => turnWindowStateUp b xs
  | .down => turnWindowStateDown b xs

/-- 原方向严格越过冻结 P；相等不破。两端极值取原始 Stroke，不取合并后的区间。 -/
def rawCrossesP (d : Direction) (b : FeatureElem) (s : Stroke) : Prop :=
  match d with
  | .up => b.high < (FeatureElem.ofStroke s).high
  | .down => (FeatureElem.ofStroke s).low < b.low

instance (d : Direction) (b : FeatureElem) (s : Stroke) : Decidable (rawCrossesP d b s) := by
  cases d <;> unfold rawCrossesP <;> infer_instance

/-- 第一种单笔观察：与 b 同向者使用原投影判据；其他笔只查 P，不参与 confirmed。 -/
def rawTurnObservation (d : Direction) (b : FeatureElem) (s : Stroke) : TurnWindowState :=
  if s.direction = d.flip then projectedTurnWindow d b [FeatureElem.ofStroke s]
  else if rawCrossesP d b s then .killedByNewExtreme else .openAtPrefixEnd

/-- 消费完整的给定原始笔前缀，第一终态吸收；不合并或改写 b。 -/
def rawTurnWindowState (d : Direction) (b : FeatureElem) : List Stroke → TurnWindowState
  | [] => .openAtPrefixEnd
  | s :: rest =>
      match rawTurnObservation d b s with
      | .confirmed => .confirmed
      | .killedByNewExtreme => .killedByNewExtreme
      | .openAtPrefixEnd => rawTurnWindowState d b rest

/-- 原始前缀 OPEN 的逐笔前件：所有笔不越 P，且同向笔也不越 b 的终点。 -/
def RawTurnSafe (d : Direction) (b : FeatureElem) (s : Stroke) : Prop :=
  ¬ rawCrossesP d b s ∧
    (s.direction = d.flip → Contains b (FeatureElem.ofStroke s))

theorem rawTurnObservation_open_iff (d : Direction) (b : FeatureElem) (s : Stroke) :
    rawTurnObservation d b s = .openAtPrefixEnd ↔ RawTurnSafe d b s := by
  cases d <;> by_cases hd : s.direction = Direction.down <;>
    by_cases hu : s.direction = Direction.up <;>
    simp [rawTurnObservation, projectedTurnWindow, turnWindowStateUp, turnWindowStateDown,
      RawTurnSafe, rawCrossesP, Contains, Direction.flip, hd, hu, Tick] <;> omega

theorem rawTurnWindowState_open_iff (d : Direction) (b : FeatureElem) (xs : List Stroke) :
    rawTurnWindowState d b xs = .openAtPrefixEnd ↔ ∀ s ∈ xs, RawTurnSafe d b s := by
  induction xs with
  | nil => simp [rawTurnWindowState]
  | cons s rest ih =>
    have hs := rawTurnObservation_open_iff d b s
    cases ho : rawTurnObservation d b s <;>
      simp [rawTurnWindowState, ho, ih, ← hs]

theorem rawTurnObservation_reverse_not_confirmed (d : Direction) (b : FeatureElem) (s : Stroke)
    (h : s.direction ≠ d.flip) : rawTurnObservation d b s ≠ .confirmed := by
  simp only [rawTurnObservation, if_neg h]
  split <;> simp

theorem rawTurnObservation_crosses_killed (d : Direction) (b : FeatureElem) (s : Stroke)
    (h : rawCrossesP d b s) : rawTurnObservation d b s = .killedByNewExtreme := by
  cases d <;> by_cases hd : s.direction = Direction.down <;>
    by_cases hu : s.direction = Direction.up <;>
    simp_all [rawTurnObservation, projectedTurnWindow, turnWindowStateUp,
      turnWindowStateDown, rawCrossesP, Direction.flip]

theorem rawTurnWindowState_append_of_ne_open (d : Direction) (b : FeatureElem)
    (xs ys : List Stroke) (h : rawTurnWindowState d b xs ≠ .openAtPrefixEnd) :
    rawTurnWindowState d b (xs ++ ys) = rawTurnWindowState d b xs := by
  induction xs with
  | nil => exact (h rfl).elim
  | cons s rest ih =>
    cases ho : rawTurnObservation d b s <;>
      simp only [List.cons_append, rawTurnWindowState, ho] at h ⊢
    exact ih h

/-- 先前原始前缀仍开放时，下一笔原始越界立刻作废，包括被过滤的反向尾笔。 -/
theorem rawTurnWindowState_append_of_open (d : Direction) (b : FeatureElem)
    (xs ys : List Stroke) (h : rawTurnWindowState d b xs = .openAtPrefixEnd) :
    rawTurnWindowState d b (xs ++ ys) = rawTurnWindowState d b ys := by
  induction xs with
  | nil => rfl
  | cons s rest ih =>
    cases ho : rawTurnObservation d b s <;>
      simp only [List.cons_append, rawTurnWindowState, ho] at h ⊢
    exact ih h

theorem rawTurnWindowState_open_then_kill (d : Direction) (b : FeatureElem)
    (xs : List Stroke) (s : Stroke)
    (hopen : rawTurnWindowState d b xs = .openAtPrefixEnd) (hkill : rawCrossesP d b s) :
    rawTurnWindowState d b (xs ++ [s]) = .killedByNewExtreme := by
  rw [rawTurnWindowState_append_of_open d b xs [s] hopen]
  simp [rawTurnWindowState, rawTurnObservation_crosses_killed d b s hkill]

/-- 只有在所有被丢弃笔均不越 P 时，投影列表才足以替代原始前缀。
这是充分的映射前件，绝非无条件“完整前缀”定理；终态后的笔也要求安全故前件非必要。 -/
theorem rawTurnWindowState_eq_projection (d : Direction) (b : FeatureElem) (xs : List Stroke)
    (hsafe : ∀ s ∈ xs, s.direction ≠ d.flip → ¬ rawCrossesP d b s) :
    rawTurnWindowState d b xs = projectedTurnWindow d b (projectStrokes d.flip xs) := by
  induction xs with
  | nil => cases d <;> rfl
  | cons s rest ih =>
    have hr : ∀ t ∈ rest, t.direction ≠ d.flip → ¬ rawCrossesP d b t :=
      fun t ht => hsafe t (by simp [ht])
    have hi := ih hr
    by_cases hd : s.direction = d.flip
    · cases d <;>
        simp only [rawTurnWindowState, rawTurnObservation, projectStrokes, hd, if_pos,
          projectedTurnWindow, turnWindowStateUp, turnWindowStateDown] at hi ⊢ <;>
        split <;> simp_all <;> split <;> simp_all
    · have hn := hsafe s (by simp) hd
      simpa [rawTurnWindowState, rawTurnObservation, projectStrokes, hd, hn] using hi

/-- #1490 第一种原始前缀确认；与旧 FirstKindConfirmsUp 的投影域分开命名。 -/
def FirstKindRawConfirmsUp (a b : FeatureElem) (xs : List Stroke) : Prop :=
  ¬ HasGap a b ∧ a.high < b.high ∧ rawTurnWindowState .up b xs = .confirmed

def FirstKindRawConfirmsDown (a b : FeatureElem) (xs : List Stroke) : Prop :=
  ¬ HasGap a b ∧ b.low < a.low ∧ rawTurnWindowState .down b xs = .confirmed

theorem firstKindRawUp_iff_projected (a b : FeatureElem) (xs : List Stroke)
    (h : ∀ s ∈ xs, s.direction ≠ Direction.down → ¬ rawCrossesP .up b s) :
    FirstKindRawConfirmsUp a b xs ↔ FirstKindConfirmsUp a b (projectStrokes .down xs) := by
  unfold FirstKindRawConfirmsUp FirstKindConfirmsUp
  rw [rawTurnWindowState_eq_projection .up b xs h]
  rfl

theorem firstKindRawDown_iff_projected (a b : FeatureElem) (xs : List Stroke)
    (h : ∀ s ∈ xs, s.direction ≠ Direction.up → ¬ rawCrossesP .down b s) :
    FirstKindRawConfirmsDown a b xs ↔ FirstKindConfirmsDown a b (projectStrokes .up xs) := by
  unfold FirstKindRawConfirmsDown FirstKindConfirmsDown
  rw [rawTurnWindowState_eq_projection .down b xs h]
  rfl

/-- 独审 P2 反例：a=[0,8]，b=10→2，已知反向尾笔=2→12。
投影为空仍 OPEN；原始前缀已越 P=10，必须 KILLED。 -/
theorem rawTurnWindowState_reverse_tail_counterexample :
    ¬ HasGap ⟨0, 8, by decide⟩ ⟨2, 10, by decide⟩ ∧
    turnWindowStateUp ⟨2, 10, by decide⟩
      (projectStrokes .down [⟨.up, 1, 2, 2, 12⟩]) = .openAtPrefixEnd ∧
    rawTurnWindowState .up ⟨2, 10, by decide⟩
      [⟨.up, 1, 2, 2, 12⟩] = .killedByNewExtreme := by
  decide

/-- 相等不破；反向尾笔即使几何区间越 b 终点，也没有确认资格。后半是非连续负控。 -/
theorem rawTurnWindowState_reverse_tail_controls :
    rawTurnWindowState .up ⟨2, 10, by decide⟩ [⟨.up, 1, 2, 2, 10⟩] = .openAtPrefixEnd ∧
    rawTurnWindowState .up ⟨2, 10, by decide⟩ [⟨.up, 1, 2, 1, 9⟩] = .openAtPrefixEnd ∧
    rawTurnWindowState .down ⟨-10, -2, by decide⟩
      [⟨.down, 1, 2, -2, -12⟩] = .killedByNewExtreme := by
  decide

/-- 连续交替的正控：反向笔止于 P，随后同向笔先破终点而确认；已确认不被后尾推翻。 -/
theorem rawTurnWindowState_confirmed_prefix_control :
    rawTurnWindowState .up ⟨2, 10, by decide⟩
      [⟨.up, 1, 2, 2, 10⟩, ⟨.down, 2, 3, 10, 1⟩, ⟨.up, 3, 4, 1, 12⟩] = .confirmed := by
  decide

/-! #1419：固定 545e / 112e 的 second_fold 计算投影。
复用 Stroke 的五个字段、FeatureElem.ofStroke、Contains、mergeInclusion 和既有分型谓词；
不把分型输入留作 oracle。标准元素以逆序存储，head 对应 Rust Vec.last。
本节覆盖数值/方向/成员/终态计算；Rust 的 steps/checks 全历史、FNV 指纹、游标撤销/恢复
以及机器整数编码的精化仍未完成，详见 v5 报告的 D_SECOND_FOLD_REFINEMENT 分项。 -/

namespace SecondFold

instance (a b c : FeatureElem) : Decidable (IsTopFractal a b c) := by
  unfold IsTopFractal; infer_instance

instance (a b c : FeatureElem) : Decidable (IsBottomFractal a b c) := by
  unfold IsBottomFractal; infer_instance

structure Candidate where
  segDir : Direction
  bStroke : Nat
  extreme : Tick
  deriving Repr

/-- 对应 FeatureElement 的 high/low/stroke_idx/stroke_indices；members 保持时间顺序。 -/
structure Element where
  range : FeatureElem
  strokeIdx : Nat
  members : List Nat
  deriving Repr

/-- 每一原始笔推进索引，只有同向笔输出元素；strokeIdx 是笔数组索引而非 K 线索引。 -/
def indexedProjection (d : Direction) (from : Nat) : List Stroke → List Element
  | [] => []
  | s :: rest =>
      if s.direction = d then
        ⟨FeatureElem.ofStroke s, from, [from]⟩ :: indexedProjection d (from + 1) rest
      else indexedProjection d (from + 1) rest

inductive Decision
  | confirmed
  | killedByNewExtreme
  deriving DecidableEq, Repr

structure Kill where
  strokeIdx : Nat
  price : Tick
  candidateExtreme : Tick
  deriving Repr

structure State where
  fromStroke : Nat
  throughStroke : Nat
  consumedLen : Nat
  rawRev : List Element
  standardRev : List Element
  dirState : Option Direction
  watermark : Option Tick
  decision : Option Decision
  matched : Option (Element × Element × Element)
  killedBy : Option Kill
  deriving Repr

/-- second_fold.rs:129：向上旧段初始 Down；向下旧段 None（有效合并方向为 Up）。 -/
def initial (c : Candidate) : State :=
  { fromStroke := c.bStroke + 1, throughStroke := c.bStroke, consumedLen := 0
    rawRev := [], standardRev := [], dirState := if c.segDir = .up then some .down else none
    watermark := none, decision := none, matched := none, killedBy := none }

def effectiveUp (ds : Option Direction) : Bool := decide (ds ≠ some .down)

/-- feature_seq.rs:289 apply_inclusion：只修改末元素，互含不改 dir_state。 -/
def includeElem (ds : Option Direction) (x : Element) : List Element → List Element × Option Direction
  | [] => ([x], ds)
  | last :: rest =>
      if Contains last.range x.range ∨ Contains x.range last.range then
        ({ range := mergeInclusion (effectiveUp ds) last.range x.range
           strokeIdx := x.strokeIdx, members := last.members ++ x.members } :: rest, ds)
      else
        let nextDir :=
          if last.range.high < x.range.high ∧ last.range.low < x.range.low then some .up
          else if x.range.high < last.range.high ∧ x.range.low < last.range.low then some .down
          else ds
        (x :: last :: rest, nextDir)

/-- Rust 的最近三元素按 a,b,c 顺序检验；不限制只顶或只底。 -/
def tailFractal : List Element → Bool
  | c :: b :: a :: _ => decide (IsTopFractal a.range b.range c.range ∨
      IsBottomFractal a.range b.range c.range)
  | _ => false

def tailTriple : List Element → Option (Element × Element × Element)
  | c :: b :: a :: _ => some (a, b, c)
  | _ => none

def reach (d : Direction) (x : FeatureElem) : Tick :=
  match d with
  | .up => x.high
  | .down => x.low

def breached (c : Candidate) (x : FeatureElem) : Prop :=
  match c.segDir with
  | .up => c.extreme < x.high
  | .down => x.low < c.extreme

instance (c : Candidate) (x : FeatureElem) : Decidable (breached c x) := by
  unfold breached; split <;> infer_instance

def updateWatermark (d : Direction) (old : Option Tick) (r : Tick) : Option Tick :=
  match old with
  | none => some r
  | some w => some (match d with
      | .up => if w ≥ r then w else r
      | .down => if w ≤ r then w else r)

/-- 固定第二种次序：包含后分型先于原始越界；x 未经合并，用于 reach 与 kill。 -/
def consumeFeature (c : Candidate) (st : State) (i : Nat) (x : FeatureElem) : State :=
  let incoming : Element := ⟨x, i, [i]⟩
  let merged := includeElem st.dirState incoming st.standardRev
  let next := { st with rawRev := incoming :: st.rawRev, standardRev := merged.1
                       dirState := merged.2
                       watermark := updateWatermark c.segDir st.watermark (reach c.segDir x) }
  if tailFractal merged.1 then
    { next with decision := some .confirmed, matched := tailTriple merged.1 }
  else if breached c x then
    { next with decision := some .killedByNewExtreme
                killedBy := some ⟨i, reach c.segDir x, c.extreme⟩ }
  else next

/-- 所有原始笔占一个预算单位；与旧段反向者不改几何、水位或终态（Rust :196）。 -/
def consume (c : Candidate) (st : State) (s : Stroke) : State :=
  let i := st.fromStroke + st.consumedLen
  let advanced := { st with consumedLen := st.consumedLen + 1, throughStroke := i }
  if s.direction = c.segDir then consumeFeature c advanced i (FeatureElem.ofStroke s)
  else advanced

/-- Rust while 的 decided 守卫与 step 合成；已终结态不再消费输入。 -/
def step (c : Candidate) (st : State) (s : Stroke) : State :=
  if st.decision = none then consume c st s else st

def run (c : Candidate) (st : State) : List Stroke → State
  | [] => st
  | s :: rest => run c (step c st s) rest

inductive Coverage
  | complete
  | incompleteResourceBound
  deriving DecidableEq, Repr

structure Result where
  state : State
  semantic : Option TurnWindowState
  coverage : Coverage
  deriving Repr

def terminal : Decision → TurnWindowState
  | .confirmed => .confirmed
  | .killedByNewExtreme => .killedByNewExtreme

/-- second_fold.rs:302 finish：覆盖不足且未决时没有语义，不得冒充 OPEN。 -/
def finish (st : State) (available budget : Nat) : Result :=
  match st.decision with
  | some t => ⟨st, some (terminal t), .complete⟩
  | none => if budget < available then ⟨st, none, .incompleteResourceBound⟩
      else ⟨st, some .openAtPrefixEnd, .complete⟩

/-- 对固定原始前缀从 b+1 起读真实 Stroke；预算按双向笔数，不是过滤后的元素数。
None 对应 Rust window=0。这里只形式化不改写的完整输入；末笔暂算/撤销属于游标义务。 -/
def evaluate (c : Candidate) (strokes : List Stroke) (grant : Option Nat) : Result :=
  let suffix := strokes.drop (c.bStroke + 1)
  let budget := match grant with | none => suffix.length | some n => n
  finish (run c (initial c) (suffix.take budget)) suffix.length budget

/-- 端点抽象与 Rust stroke_high_low 的分支逐值相同，含相等端点。 -/
theorem ofStroke_rust_endpoints (s : Stroke) :
    (FeatureElem.ofStroke s).high =
      (if s.startPrice ≥ s.endPrice then s.startPrice else s.endPrice) ∧
    (FeatureElem.ofStroke s).low =
      (if s.startPrice ≥ s.endPrice then s.endPrice else s.startPrice) := by
  by_cases h : s.startPrice ≤ s.endPrice <;> by_cases g : s.startPrice ≥ s.endPrice <;>
    simp [FeatureElem.ofStroke, h, g] <;> simp only [Tick] at * <;> omega

theorem includeElem_merge (ds : Option Direction) (x last : Element) (rest : List Element)
    (h : Contains last.range x.range ∨ Contains x.range last.range) :
    includeElem ds x (last :: rest) =
      ({ range := mergeInclusion (effectiveUp ds) last.range x.range
         strokeIdx := x.strokeIdx, members := last.members ++ x.members } :: rest, ds) := by
  simp [includeElem, h]

theorem includeElem_append (ds : Option Direction) (x last : Element) (rest : List Element)
    (h : ¬ (Contains last.range x.range ∨ Contains x.range last.range)) :
    (includeElem ds x (last :: rest)).1 = x :: last :: rest := by
  simp [includeElem, h]

theorem includeElem_merge_direction (ds : Option Direction) (x last : Element)
    (rest : List Element) (h : Contains last.range x.range ∨ Contains x.range last.range) :
    (includeElem ds x (last :: rest)).2 = ds := by
  simp [includeElem, h]

theorem effectiveUp_none : effectiveUp none = true := by decide
theorem effectiveUp_down : effectiveUp (some .down) = false := by decide

theorem tailFractal_iff (a b c : Element) (rest : List Element) :
    tailFractal (c :: b :: a :: rest) = true ↔
      IsTopFractal a.range b.range c.range ∨ IsBottomFractal a.range b.range c.range := by
  simp [tailFractal]

/-- 合并保留其余标准元素及旧成员身份；价格有效性由 FeatureElem.valid 携带。 -/
theorem includeElem_merge_members (ds : Option Direction) (x last : Element) (rest : List Element)
    (h : Contains last.range x.range ∨ Contains x.range last.range) :
    ((includeElem ds x (last :: rest)).1.map Element.members) =
      (last.members ++ x.members) :: rest.map Element.members := by
  simp [includeElem, h]

theorem consumeFeature_confirmed_first (c : Candidate) (st : State) (i : Nat) (x : FeatureElem)
    (h : tailFractal (includeElem st.dirState ⟨x, i, [i]⟩ st.standardRev).1 = true) :
    (consumeFeature c st i x).decision = some .confirmed := by
  simp [consumeFeature, h]

theorem consumeFeature_killed (c : Candidate) (st : State) (i : Nat) (x : FeatureElem)
    (hf : tailFractal (includeElem st.dirState ⟨x, i, [i]⟩ st.standardRev).1 = false)
    (hb : breached c x) :
    (consumeFeature c st i x).decision = some .killedByNewExtreme ∧
    (consumeFeature c st i x).killedBy = some ⟨i, reach c.segDir x, c.extreme⟩ := by
  simp [consumeFeature, hf, hb]

theorem consumeFeature_open (c : Candidate) (st : State) (i : Nat) (x : FeatureElem)
    (hs : st.decision = none)
    (hf : tailFractal (includeElem st.dirState ⟨x, i, [i]⟩ st.standardRev).1 = false)
    (hb : ¬ breached c x) : (consumeFeature c st i x).decision = none := by
  simp [consumeFeature, hf, hb, hs]

/-- 水位由合并前原始极值更新；即使本步确认或 kill，该水位仍保留。 -/
theorem consumeFeature_watermark (c : Candidate) (st : State) (i : Nat) (x : FeatureElem) :
    (consumeFeature c st i x).watermark =
      updateWatermark c.segDir st.watermark (reach c.segDir x) := by
  unfold consumeFeature
  split <;> first | rfl | (split <;> rfl)

theorem updateWatermark_up_bound (old : Option Tick) (r : Tick) :
    ∃ w, updateWatermark .up old r = some w ∧ r ≤ w ∧
      (∀ v, old = some v → v ≤ w) := by
  cases old with
  | none => exact ⟨r, rfl, by omega, by simp⟩
  | some v =>
    by_cases h : v ≥ r
    · refine ⟨v, by simp [updateWatermark, h], by omega, ?_⟩
      intro w hw; cases hw; omega
    · refine ⟨r, by simp [updateWatermark, h], by omega, ?_⟩
      intro w hw; cases hw; omega

theorem updateWatermark_down_bound (old : Option Tick) (r : Tick) :
    ∃ w, updateWatermark .down old r = some w ∧ w ≤ r ∧
      (∀ v, old = some v → w ≤ v) := by
  cases old with
  | none => exact ⟨r, rfl, by omega, by simp⟩
  | some v =>
    by_cases h : v ≤ r
    · refine ⟨v, by simp [updateWatermark, h], by omega, ?_⟩
      intro w hw; cases hw; omega
    · refine ⟨r, by simp [updateWatermark, h], by omega, ?_⟩
      intro w hw; cases hw; omega

theorem consumeFeature_position (c : Candidate) (st : State) (i : Nat) (x : FeatureElem) :
    (consumeFeature c st i x).consumedLen = st.consumedLen ∧
    (consumeFeature c st i x).fromStroke = st.fromStroke ∧
    (consumeFeature c st i x).throughStroke = st.throughStroke := by
  unfold consumeFeature
  split <;> first | exact ⟨rfl, rfl, rfl⟩ | (split <;> exact ⟨rfl, rfl, rfl⟩)

theorem consumeFeature_raw (c : Candidate) (st : State) (i : Nat) (x : FeatureElem) :
    (consumeFeature c st i x).rawRev = ⟨x, i, [i]⟩ :: st.rawRev := by
  unfold consumeFeature
  split <;> first | rfl | (split <;> rfl)

theorem step_position (c : Candidate) (st : State) (s : Stroke) (h : st.decision = none) :
    (step c st s).consumedLen = st.consumedLen + 1 ∧
    (step c st s).fromStroke = st.fromStroke ∧
    (step c st s).throughStroke = st.fromStroke + st.consumedLen := by
  simp only [step, h, if_pos, consume]
  split
  · exact consumeFeature_position c _ _ _
  · exact ⟨rfl, rfl, rfl⟩

theorem step_reverse (c : Candidate) (st : State) (s : Stroke)
    (h : st.decision = none) (hd : s.direction ≠ c.segDir) :
    step c st s = { st with consumedLen := st.consumedLen + 1
                           throughStroke := st.fromStroke + st.consumedLen } := by
  simp [step, consume, h, hd]

theorem step_decided (c : Candidate) (st : State) (s : Stroke) (h : st.decision ≠ none) :
    step c st s = st := by
  simp [step, h]

theorem run_append (c : Candidate) (st : State) (xs ys : List Stroke) :
    run c st (xs ++ ys) = run c (run c st xs) ys := by
  induction xs generalizing st with
  | nil => rfl
  | cons s rest ih => simp only [List.cons_append, run]; exact ih _

theorem run_decided (c : Candidate) (st : State) (xs : List Stroke) (h : st.decision ≠ none) :
    run c st xs = st := by
  induction xs with
  | nil => rfl
  | cons s rest ih => simpa [run, step_decided c st s h] using ih

/-- 任意完整已消费前缀到终态后，追加任何笔都不再改变本计算投影。 -/
theorem run_terminal_prefix (c : Candidate) (st : State) (xs ys : List Stroke)
    (h : (run c st xs).decision ≠ none) : run c st (xs ++ ys) = run c st xs := by
  rw [run_append]
  exact run_decided c (run c st xs) ys h

theorem run_none_initial (c : Candidate) (st : State) (xs : List Stroke)
    (h : (run c st xs).decision = none) : st.decision = none := by
  by_cases hs : st.decision = none
  · exact hs
  · rw [run_decided c st xs hs] at h
    exact (hs h).elim

/-- 未决到前缀末端时实际消费数等于所有原始笔数，包含被方向过滤的笔。 -/
theorem run_undecided_consumed (c : Candidate) (st : State) (xs : List Stroke)
    (h : (run c st xs).decision = none) :
    (run c st xs).consumedLen = st.consumedLen + xs.length := by
  induction xs generalizing st with
  | nil => simp [run]
  | cons s rest ih =>
    have hs := run_none_initial c st (s :: rest) h
    have hp := (step_position c st s hs).1
    have hi := ih (step c st s) h
    simp only [run, List.length_cons]
    omega

/-- 未决前缀的 raw_elements 精确等于实际方向投影；索引含被过滤笔的位置。
没有把“输入等于某 oracle”作为假设，右侧直接从原始 Stroke 列表构造。 -/
theorem run_undecided_raw (c : Candidate) (st : State) (xs : List Stroke)
    (h : (run c st xs).decision = none) :
    (run c st xs).rawRev =
      (indexedProjection c.segDir (st.fromStroke + st.consumedLen) xs).reverse ++ st.rawRev := by
  induction xs generalizing st with
  | nil => simp [run, indexedProjection]
  | cons s rest ih =>
    have hs := run_none_initial c st (s :: rest) h
    have hp := step_position c st s hs
    have hn : (step c st s).fromStroke + (step c st s).consumedLen =
        st.fromStroke + st.consumedLen + 1 := by
      rw [hp.1, hp.2.1]
      omega
    simp only [run]
    rw [ih (step c st s) h, hn]
    by_cases hd : s.direction = c.segDir
    · have hr : (step c st s).rawRev =
          ⟨FeatureElem.ofStroke s, st.fromStroke + st.consumedLen,
            [st.fromStroke + st.consumedLen]⟩ :: st.rawRev := by
        simp only [step, hs, if_pos, consume, hd]
        exact consumeFeature_raw c _ _ _
      rw [hr]
      simp [indexedProjection, hd, List.reverse_cons, List.append_assoc]
    · rw [step_reverse c st s hs hd]
      simp [indexedProjection, hd]

/-- 从已消费原始元素按时间折叠极值；空列为none，不引入价格0初值。 -/
def watermarkOfRaw (d : Direction) : List Element → Option Tick
  | [] => none
  | x :: rest => updateWatermark d (watermarkOfRaw d rest) (reach d x.range)

theorem step_watermark_of_raw (c : Candidate) (st : State) (s : Stroke)
    (h : st.watermark = watermarkOfRaw c.segDir st.rawRev) :
    (step c st s).watermark = watermarkOfRaw c.segDir (step c st s).rawRev := by
  by_cases hs : st.decision = none
  · by_cases hd : s.direction = c.segDir
    · simp only [step, hs, if_pos, consume, hd]
      rw [consumeFeature_watermark, consumeFeature_raw]
      simp only [watermarkOfRaw]
      exact congrArg (fun old => updateWatermark c.segDir old
        (reach c.segDir (FeatureElem.ofStroke s))) h
    · simpa [step, consume, hs, hd] using h
  · simpa [step, hs] using h

theorem run_watermark_of_raw (c : Candidate) (st : State) (xs : List Stroke)
    (h : st.watermark = watermarkOfRaw c.segDir st.rawRev) :
    (run c st xs).watermark = watermarkOfRaw c.segDir (run c st xs).rawRev := by
  induction xs generalizing st with
  | nil => exact h
  | cons s rest ih =>
    exact ih (step c st s) (step_watermark_of_raw c st s h)

/-- 包括提前终态的所有消费前缀：水位恰为实际已消费原始元素的方向极值折叠。 -/
theorem run_initial_watermark (c : Candidate) (xs : List Stroke) :
    (run c (initial c) xs).watermark =
      watermarkOfRaw c.segDir (run c (initial c) xs).rawRev := by
  exact run_watermark_of_raw c (initial c) xs rfl

theorem finish_resource_not_open (st : State) (n budget : Nat)
    (hs : st.decision = none) (hb : budget < n) :
    (finish st n budget).semantic = none ∧
    (finish st n budget).coverage = .incompleteResourceBound := by
  simp [finish, hs, hb]

theorem finish_open_iff (st : State) (n budget : Nat) :
    (finish st n budget).semantic = some .openAtPrefixEnd ↔
      st.decision = none ∧ n ≤ budget := by
  cases h : st.decision with
  | none => simp [finish, h]; omega
  | some d => cases d <;> simp [finish, h, terminal]

theorem finish_decided_complete (st : State) (n budget : Nat) (d : Decision)
    (h : st.decision = some d) :
    (finish st n budget).semantic = some (terminal d) ∧
    (finish st n budget).coverage = .complete := by
  simp [finish, h]

/-- 连续交替笔：第三个第二序列元素同时出底分型并越 P，确认先于 kill。 -/
theorem same_stroke_confirmation_priority :
    (evaluate ⟨.up, 0, 10⟩
      [⟨.down, 0, 1, 10, 1⟩, ⟨.up, 1, 2, 1, 4⟩,
       ⟨.down, 2, 3, 4, 0⟩, ⟨.up, 3, 4, 0, 3⟩,
       ⟨.down, 4, 5, 3, 2⟩, ⟨.up, 5, 6, 2, 11⟩] none).semantic = some .confirmed := by
  decide

/-- 向下包含抹去新 high，原始 reach=12 仍使 P=10 失效，水位不能读合并 high=8。 -/
theorem merged_extreme_still_kills :
    let r := evaluate ⟨.up, 0, 10⟩
      [⟨.down, 0, 1, 10, 2⟩, ⟨.up, 1, 2, 2, 8⟩,
       ⟨.down, 2, 3, 8, 1⟩, ⟨.up, 3, 4, 1, 12⟩] none
    r.semantic = some .killedByNewExtreme ∧ r.state.watermark = some 12 ∧
      r.state.standardRev.map (fun e => e.range.high) = [8] ∧
      r.state.standardRev.map Element.members = [[1, 3]] := by
  decide

/-- 预算按双向原始笔计数；未覆盖最后的反向笔时不可报 OPEN。 -/
theorem budget_counts_reverse_stroke :
    let c : Candidate := ⟨.up, 0, 10⟩
    let xs : List Stroke :=
      [⟨.down, 0, 1, 10, 2⟩, ⟨.up, 1, 2, 2, 8⟩, ⟨.down, 2, 3, 8, 3⟩]
    (evaluate c xs (some 1)).semantic = none ∧
      (evaluate c xs (some 1)).coverage = .incompleteResourceBound ∧
      (evaluate c xs (some 2)).semantic = some .openAtPrefixEnd ∧
      (evaluate c xs (some 2)).state.consumedLen = 2 := by
  decide

end SecondFold

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
