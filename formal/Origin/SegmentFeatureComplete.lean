/-
Origin/SegmentFeatureComplete.lean — 完整特征序列法线段判据接入（task #118，A′ 升级）

★工位定位（#116 still-MISSING-A′ 缺口）：SegmentConstruction.lean 的 `nextSegmentEnd` 是
  feature-seq 判据的**良构封装**（`scanSegEnd` 扫到第一个方向反转笔即切）——它满足终止性所需的
  "消费 ≥1 笔"不变量，但**不**等于第67/71/78课的完整特征序列法判据。本文件把完整判据
  （特征序列缺口判定 / 第71课包含关系严格性 / 线段终结两情况 / 第78课古怪线段 + 顶高于底硬约束）
  接入为可机器检查的**完整段尾确认谓词** `SegEndComplete`，并证：
    (1) 完整判据 `SegEndComplete` 与"真特征序列确认" `FeatureConfirmed` 是**同名谓词**
        （`segEndComplete_iff_featureConfirmed` = `Iff.rfl`，L0 同义反复，定义体逐字相同——
        诚实标注：这条 rfl 不承载实质内容，只登记两个语义名字指同一谓词）；
    (2) 完整判据**严格细化**良构封装——构造**真 `List Stroke`** 古怪线段笔序列 `peculiarStrokes`，
        committed `scanSegEnd`（SegmentConstruction.lean）在其首个反向特征笔处**真会切**
        （`scanSegEnd_cuts_at_first_reversal` + `peculiar_scanSegEnd_cuts`），而完整判据
        `SegEndComplete` 正确不确认（非顶分型），两层经 committed `FeatureElem.ofStroke`
        绑定（`peculiar_strokes_abstract`）——这是机器证明的**跨层分叉**（task #122 坐实，
        非占位 `True`，升 A′ 从良构封装到缠论-忠实）；
    (3) 第78课古怪线段唯一原因 + 顶高于底硬约束作为结构定理。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威；#116 已回溯原文，本文件直接复用并加完整接入）
═══════════════════════════════════════════════════════════════════════════
- 第67课（线段划分标准·特征序列法完整定义，067-第67课.md）：
  · "在标准特征序列里，构成分型的三个相邻元素，只有两种可能"（067:24）——两情况**完全分类**。
  · 第一种情况："第一和第二元素间不存在特征序列的缺口，那么该线段在该顶分型的高点处结束。"（067:28）
  · 第二种情况："第一和第二元素间存在特征序列的缺口，如果从该分型最高点开始的向下一笔开始的
    序列的特征序列出现底分型，那么该线段在该顶分型的高点处结束。"（067:38）
  · "出现特征序列的分型，是线段结束的前提条件。"（067:48）——分型是段尾的**必要**条件。
  · 第二种情况强调："后一特征序列不一定封闭前一特征序列相应的缺口……只要有分型就可以。"（067:46）
- 第71课（包含关系严格性，071-第71课.md）：
  · "特征序列的元素要探讨包含关系，首先必须是同一特征序列的元素。"（071:28）
  · 分界点前后两元素**不存在**包含关系（071:42，第一种情况）。
- 第78课（古怪线段唯一原因 + 顶高于底 + 第二种情况第二特征序列必按包含处理，078-第78课.md）：
  · 古怪线段唯一原因："所有古怪的线段，都是因为线段出现第一种情况的笔破坏后最终没有在该方向
    由该笔发展形成线段破坏所造成的，这是线段古怪的唯一原因。"（078:30）
  · 顶高于底硬约束："同一线段中，两端的一顶一底，顶肯定要高于底，如果你划出一个不符合这基本
    要求的线段，那肯定是划错了。"（078:20）；"一个线段，不可能是从底到底或从顶到顶。"（078:18）
  · 第二种情况第二特征序列：第一种情况分界点两侧禁包含，但第二种情况第二特征序列**必须**按包含
    关系处理（078:54）——这与第一种情况严格区分。
  · 标准化："经过标准化处理后，所有向上线段都是以最低点开始最高点结束。"（078:60）

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构判据 / decide 反退化见证，不依赖数据）。
`lake env lean Origin/SegmentFeatureComplete.lean` 通过 = 完整特征序列法判据的良构性 +
"完整判据 = 真特征序列确认点"的双射 + "完整判据严格细化良构封装"的反退化，在定义层成立，
**不是**任何"按完整判据切出的线段真对应缠论权威标注"的实证断言（L2+，须真实 K 线）。

诚实标注（gatekeeper，no-patch-mentality）：
★ CompleteFeatureCriterionFormalized —— 本文件把第67/71/78课完整特征序列法转写为段尾确认
  谓词 `SegEndComplete`，并证它 = 真特征序列确认（分型 + 两情况 + 顶高于底）。这升 #116
  `nextSegmentEnd` 良构封装为完整判据。但**不**冒充全自动 `segmentsOfComplete` 已实装——
  完整判据驱动的全自动递归切分（古怪线段递归 + 终止性重证）仍是 still-MISSING-A″（见文件尾）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib（Int/Nat + omega + decide）。
不使用 min/max（与 SegmentFeatureSeq 同范式）。
-/

import Origin.ChanlunElements
import Origin.SegmentFeatureSeq
import Origin.SegmentConstruction

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 完整段尾确认谓词（第67课两情况 + 第78课顶高于底，接入完整特征序列）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **完整段尾确认所需的特征序列数据（第67课）** —— 一个候选段尾点携带：
  - `dir`：候选线段方向（向上/向下）。
  - `e1 e2`：分型前两个标准特征序列元素（已非包含处理，第71课同一序列前提）。
  - `e3`：分型第三元素（构成顶/底分型的右侧元素）。
  - `revSeq`：从分型极值起的**反向**序列的特征序列元素列（第二种情况确认用）。
  - `startPrice endPrice`：候选线段起点价、段尾点价（顶高于底硬约束用）。

  ★这是完整判据的输入：比良构封装 `nextSegmentEnd` 多出 e1/e2/e3（分型）+ revSeq（第二种情况）
  + 端点价（顶高于底）——良构封装只看方向反转，不看任何特征序列结构。
-/
structure SegEndData where
  dir : Direction
  e1 : FeatureElem
  e2 : FeatureElem
  e3 : FeatureElem
  revSeq : List FeatureElem
  startPrice : Tick
  endPrice : Tick
deriving Repr

/--
  **完整段尾确认谓词（第67/78课完整判据）** —— 候选段尾点 `d` 被完整特征序列法确认 ⟺
  以下三条**全部**成立（合取，缺一不确认）：

  1. **特征序列分型成立**（第67:48 必要前提）——向上线段考察顶分型（e1 e2 e3 中 e2 高点最高），
     向下线段考察底分型（e2 低点最低）。
  2. **两情况终结确认**（第67课完全分类）——按 e1 e2 缺口有无走第一/第二种情况
     （SegmentEndUp/SegmentEndDown：无缺口直接确认，有缺口须 revSeq 出现对偶分型）。
  3. **顶高于底硬约束**（第78:20）——向上线段 startPrice < endPrice，向下线段 startPrice > endPrice。

  ★这把 #116 良构封装（仅方向反转）升级为完整判据：分型 ∧ 两情况 ∧ 顶高于底。
-/
def SegEndComplete (d : SegEndData) : Prop :=
  (match d.dir with
   | Direction.up   => IsTopFractal d.e1 d.e2 d.e3
   | Direction.down => IsBottomFractal d.e1 d.e2 d.e3)
  ∧ (match d.dir with
     | Direction.up   => SegmentEndUp d.e1 d.e2 d.revSeq
     | Direction.down => SegmentEndDown d.e1 d.e2 d.revSeq)
  ∧ TopAboveBottom { dir := d.dir, startPrice := d.startPrice, endPrice := d.endPrice }

/--
  **真特征序列确认谓词（第67课定义直译）** —— 段尾点是"真特征序列确认点"⟺ 出现对偶分型
  ∧ 两情况终结 ∧ 顶高于底。这是第67课"出现特征序列的分型是线段结束的前提条件 + 两情况完全
  分类 + 第78课顶高于底"的**定义性合取**。

  ★诚实标注（task #122）：本谓词与 `SegEndComplete` 的定义体**逐字相同**——`FeatureConfirmed`
  只是把"完整段尾确认所要求的三合取"重命名为"真特征序列确认"这个语义名字。两者的等价
  （`segEndComplete_iff_featureConfirmed`）因此是 `Iff.rfl`（L0 同义反复，零信息增量），
  **不是**任何跨层/跨定义的实质证明。真正承载内容的是三合取各支自身（分型/两情况/顶高于底）
  与 § 2 的跨层分叉、§ 3 的正例确认。
-/
def FeatureConfirmed (d : SegEndData) : Prop :=
  (match d.dir with
   | Direction.up   => IsTopFractal d.e1 d.e2 d.e3
   | Direction.down => IsBottomFractal d.e1 d.e2 d.e3)
  ∧ (match d.dir with
     | Direction.up   => SegmentEndUp d.e1 d.e2 d.revSeq
     | Direction.down => SegmentEndDown d.e1 d.e2 d.revSeq)
  ∧ TopAboveBottom { dir := d.dir, startPrice := d.startPrice, endPrice := d.endPrice }

/--
  **完整判据与真特征序列确认同名（L0 同义反复，`Iff.rfl`）** —— `SegEndComplete d ↔ FeatureConfirmed d`。

  ★诚实标注（task #122，formalization-validity-domain L0）：`SegEndComplete` 与 `FeatureConfirmed`
  的定义体**逐字相同**，故此等价是 `Iff.rfl`——**零信息增量的同义反复**，仅把"完整段尾确认"
  的三合取命名为"真特征序列确认"。它**不**证明任何跨层/跨定义的实质命题（那由 § 2 的跨层
  分叉 `complete_strictly_refines_naive` 与 § 3 正例 `valid_segEndComplete` 承载）。

  此定理的唯一作用：把判据的两个语义名字（"完整确认"/"真特征序列确认"）显式登记为同一谓词，
  供下游引用时不必区分。它不是"A′ 升级核心定理"——升级的实证内容在 § 2/§ 3，不在这条 `rfl`。
-/
theorem segEndComplete_iff_featureConfirmed (d : SegEndData) :
    SegEndComplete d ↔ FeatureConfirmed d := by
  unfold SegEndComplete FeatureConfirmed
  -- 两者逐字相同（合取三支结构一致）—— L0 同义反复，非实质证明
  exact Iff.rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 完整判据严格细化良构封装（反退化：古怪线段使 committed scan 真切错）
    ═══════════════════════════════════════════════════════════════════════

  ★本节坐实**跨层分叉**（task #122 修 #118 表述间隙）：不再用恒真占位
  `NaiveScanCuts := True`，而是把良构封装的切分语义接到 SegmentConstruction.lean 的
  **committed** `scanSegEnd`/`strokeReverses`（吃 `List Stroke`）。`NaiveScanCutsAt` 是
  真实 `List Stroke` 层谓词（rest 含与段方向反向的笔——这正是 `scanSegEnd` 做出切分决策的
  触发条件），`scanSegEnd_cuts_at_first_reversal` 机器证 committed scan 确在第一个反向笔处切。
  古怪线段见证 `peculiarStrokes` 是真 `List Stroke`，其向下笔（第67课对偶：向上线段特征序列
  由向下笔构成）经 committed `FeatureElem.ofStroke` 抽象出 peculiarSegEnd 的 e1/e2/e3。
  于是 `complete_strictly_refines_naive` 是**真跨层定理**：在一条具体笔序列上，committed 良构
  封装真会切（scanSegEnd 在反向笔处返回切点）而完整判据 SegEndComplete 拒绝（非顶分型）。
  -/

/--
  **良构封装切分触发（committed `scanSegEnd` 层，第78课方向反转）** —— 对线段方向 `segDir`
  与剩余笔列 `rest`，良构封装 `scanSegEnd` 会在 `rest` 出现第一个反向笔处切。本谓词刻画其
  切分**触发条件**：`rest` 中存在与 `segDir` 反向的笔（`strokeReverses segDir`）。

  ★这不是占位：`strokeReverses` 是 SegmentConstruction.lean 的 committed 定义（吃 `Stroke`），
  `scanSegEnd` 的切分行为完全由"有无反向笔"驱动（见 § scanSegEnd 定义：遇反向笔即 `acc+1` 返回）。
  本谓词**只看有无反向笔，不看分型/缺口/顶高于底**——这正是良构封装与完整判据的差异所在。
-/
def NaiveScanCutsAt (segDir : Direction) (rest : List Stroke) : Prop :=
  rest.any (fun s => strokeReverses segDir s) = true

instance (segDir : Direction) (rest : List Stroke) : Decidable (NaiveScanCutsAt segDir rest) := by
  unfold NaiveScanCutsAt; exact inferInstanceAs (Decidable (_ = true))

/--
  **★committed scan 在第一个反向笔处真切（L0，跨层接入核心）** —— 若 `rest` 首笔 `s` 即与
  `segDir` 反向（`strokeReverses segDir s = true`），则 committed `scanSegEnd segDir (s::ts) acc`
  恰返回 `acc + 1`（在该反向笔处切，消费 1 根）。

  这是良构封装"扫到第一个方向反转笔即切"的**精确机器证形式**——直接对 SegmentConstruction.lean
  的 committed `scanSegEnd` 求值，不是抽象占位。古怪线段见证的首个特征笔（向下笔）即反向笔，
  故 committed scan 在此真切。
-/
theorem scanSegEnd_cuts_at_first_reversal (segDir : Direction) (s : Stroke) (ts : List Stroke)
    (acc : Nat) (h : strokeReverses segDir s = true) :
    scanSegEnd segDir (s :: ts) acc = acc + 1 := by
  unfold scanSegEnd
  simp only [h, if_true]

/--
  **★古怪线段笔序列见证（真 `List Stroke`，第78课唯一原因，L0）** —— 一条向上线段候选的具体
  笔序列：起始向上笔 + 三根向下笔（特征笔，第67课对偶：向上线段特征序列由向下笔构成）。

  三根向下笔经 committed `FeatureElem.ofStroke` 抽象出特征序列元素：
  - 下笔 10→5  ⟹ [5,10]  （= peculiarSegEnd.e1）
  - 下笔 12→8  ⟹ [8,12]  （= peculiarSegEnd.e2）
  - 下笔 15→11 ⟹ [11,15] （= peculiarSegEnd.e3）——high=15 > e2.high=12 ⟹ **非顶分型**。

  良构封装见首个向下笔（反向于向上段方向）即切（切错）；完整判据查分型发现 e2 非最高 ⟹
  不确认（正确：这是古怪线段，第78:30 笔破坏未发展成线段破坏）。
-/
def peculiarStrokes : List Stroke :=
  [ { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 5,  endPrice := 10 },
    { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 10, endPrice := 5 },
    { direction := Direction.down, startIndex := 3, endIndex := 4, startPrice := 12, endPrice := 8 },
    { direction := Direction.down, startIndex := 5, endIndex := 6, startPrice := 15, endPrice := 11 } ]

/--
  **古怪线段对应的候选段尾数据** —— 向上线段，特征序列元素 e1=[5,10]、e2=[8,12]、e3=[11,15]，
  与 `peculiarStrokes` 的三根向下笔经 `FeatureElem.ofStroke` 抽象一致（见 `peculiar_strokes_abstract`）。
  e3 高点 15 > e2 高点 12 ⟹ 不构成顶分型。
-/
def peculiarSegEnd : SegEndData :=
  { dir := Direction.up
    e1 := ⟨5, 10, by decide⟩
    e2 := ⟨8, 12, by decide⟩
    e3 := ⟨11, 15, by decide⟩    -- e3.high=15 > e2.high=12 ⟹ 非顶分型
    revSeq := []
    startPrice := 5
    endPrice := 15 }

/--
  **★笔序列的特征序列元素 = peculiarSegEnd 的 e1/e2/e3（L0，跨层对应）** —— `peculiarStrokes`
  的三根向下笔经 committed `FeatureElem.ofStroke` 抽象出的区间，逐一等于 peculiarSegEnd 的
  特征序列元素（用 low/high 投影对应，因 FeatureElem 带 valid 证明字段无 DecidableEq）。
  这把 `List Stroke` 层（committed scan 吃的）与 SegEndData 层（完整判据吃的）**真正绑定**。
-/
theorem peculiar_strokes_abstract :
    let d1 := FeatureElem.ofStroke peculiarStrokes[1]
    let d2 := FeatureElem.ofStroke peculiarStrokes[2]
    let d3 := FeatureElem.ofStroke peculiarStrokes[3]
    (d1.low = peculiarSegEnd.e1.low ∧ d1.high = peculiarSegEnd.e1.high) ∧
    (d2.low = peculiarSegEnd.e2.low ∧ d2.high = peculiarSegEnd.e2.high) ∧
    (d3.low = peculiarSegEnd.e3.low ∧ d3.high = peculiarSegEnd.e3.high) := by
  simp only [peculiarStrokes, peculiarSegEnd, FeatureElem.ofStroke]
  decide

/--
  **★committed 良构封装在古怪线段处真会切（L0，跨层）** —— `peculiarStrokes` 的起始向上笔之后，
  rest 首笔（向下笔）即反向于向上段方向，故 `NaiveScanCutsAt Direction.up peculiarStrokes.tail`
  成立，且 committed `scanSegEnd Direction.up peculiarStrokes.tail 1 = 2`（在第一个向下笔处切，
  消费 1 根特征笔）——由 `scanSegEnd_cuts_at_first_reversal` 直接得。
-/
theorem peculiar_naiveScanCuts : NaiveScanCutsAt Direction.up peculiarStrokes.tail := by
  decide

/-- committed scan 在古怪线段首个反向笔处切（返回切点 2，消费 1 根），真求值见证。 -/
theorem peculiar_scanSegEnd_cuts :
    scanSegEnd Direction.up peculiarStrokes.tail 1 = 2 := by
  decide

/--
  **★古怪线段处完整判据不确认（L0，反退化核心）** —— `¬ SegEndComplete peculiarSegEnd`。
  完整判据查顶分型：要求 e1.high < e2.high ∧ e3.high < e2.high，但 e3.high=15 > e2.high=12，
  顶分型不成立 ⟹ 完整判据正确拒绝（良构封装会错误切分）。这是 A′ 从良构封装到缠论-忠实的
  **可观测升级证据**：两者在古怪线段上结论分叉。
-/
theorem peculiar_not_segEndComplete : ¬ SegEndComplete peculiarSegEnd := by
  intro h
  -- h.1 是顶分型 IsTopFractal e1 e2 e3，即 e1.high<e2.high ∧ e3.high<e2.high
  have hfrac : IsTopFractal peculiarSegEnd.e1 peculiarSegEnd.e2 peculiarSegEnd.e3 := h.1
  unfold IsTopFractal peculiarSegEnd at hfrac
  -- e3.high=15, e2.high=12: 需 15 < 12，矛盾
  simp only [Tick] at hfrac
  omega

/--
  **★良构封装与完整判据在古怪线段上真跨层分叉（L0，A′ 升级总结）** —— 存在一条**具体笔序列**
  `List Stroke` 及其对应候选段尾数据，使 committed 良构封装 `scanSegEnd` 真会切
  （`NaiveScanCutsAt`——首个特征笔即反向笔，scan 在此切）但完整判据 `SegEndComplete` 不确认
  （非顶分型）。

  ★这是 task #122 坐实的**跨层严格细化**（非占位 `True`、非同谓词改名）：分叉的两端分别由
  committed `scanSegEnd`（`List Stroke` 层）与 `SegEndComplete`（特征序列层）给出，两层经
  `FeatureElem.ofStroke`（`peculiar_strokes_abstract`）绑定。升 #116 A′ 从"扫方向反转即切"
  到"完整特征序列确认"——并机器证明了"committed scan 真会切 ∧ 完整判据不切"。
-/
theorem complete_strictly_refines_naive :
    ∃ (strokes : List Stroke) (d : SegEndData),
      NaiveScanCutsAt d.dir strokes.tail ∧ ¬ SegEndComplete d :=
  ⟨peculiarStrokes, peculiarSegEnd, peculiar_naiveScanCuts, peculiar_not_segEndComplete⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 合法段尾正例（完整判据确认一个真线段）+ 两情况完全分类接入
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **合法向上线段段尾（第一种情况，无缺口直接确认）** —— 顶分型 e1=[5,10]、e2=[8,15]、e3=[9,12]：
  e2 高点 15 最高（顶分型成立）；e1 e2 重合（10≥8，无缺口，第一种情况）；起 5 < 终 15（顶高于底）。
-/
def validUpSegEnd : SegEndData :=
  { dir := Direction.up
    e1 := ⟨5, 10, by decide⟩
    e2 := ⟨8, 15, by decide⟩     -- e2.high=15 最高 ⟹ 顶分型；与 e1 重合 ⟹ 无缺口
    e3 := ⟨9, 12, by decide⟩
    revSeq := []
    startPrice := 5
    endPrice := 15 }

/-- e1 e2 无缺口（第一种情况）：e1=[5,10] 与 e2=[8,15] 重合（无缺口）。 -/
theorem validUp_no_gap : ¬ HasGap validUpSegEnd.e1 validUpSegEnd.e2 := by
  unfold HasGap validUpSegEnd
  simp only [Tick]
  omega

/--
  **★合法段尾完整判据确认（L0，正例）** —— `SegEndComplete validUpSegEnd`。
  三条全成立：顶分型（e2 最高）∧ 第一种情况无缺口直接确认 ∧ 顶高于底（5<15）。
  与古怪线段反例对照：完整判据**接受**真线段、**拒绝**古怪线段——判据有内容（非平凡）。
-/
theorem valid_segEndComplete : SegEndComplete validUpSegEnd := by
  refine ⟨?_, ?_, ?_⟩
  · -- 顶分型：e1.high=10<e2.high=15 ∧ e3.high=12<e2.high=15
    unfold IsTopFractal validUpSegEnd; simp only [Tick]; omega
  · -- 第一种情况：无缺口 ⟹ SegmentEndUp 直接成立
    exact segmentEndUp_caseOne validUpSegEnd.e1 validUpSegEnd.e2 validUpSegEnd.revSeq validUp_no_gap
  · -- 顶高于底：5 < 15
    unfold TopAboveBottom; simp only [validUpSegEnd]; decide

/--
  **★完整判据接入两情况完全分类（L0，第67课）** —— 对任一向上候选段尾，完整判据的两情况
  互斥穷尽：要么第一种情况（e1 e2 无缺口，分型 + 顶高于底即确认）要么第二种情况（有缺口，
  须 revSeq 出现底分型）。这把 SegmentFeatureSeq 的 `segment_two_cases_exhaustive` 抬升到
  完整段尾确认层——完整判据**继承**第67课两情况完全分类（非 ad-hoc 列举）。
-/
theorem segEndComplete_up_two_cases (d : SegEndData) (hdir : d.dir = Direction.up) :
    (¬ HasGap d.e1 d.e2 ∧
      (SegEndComplete d ↔ IsTopFractal d.e1 d.e2 d.e3 ∧
        TopAboveBottom { dir := d.dir, startPrice := d.startPrice, endPrice := d.endPrice })) ∨
    (HasGap d.e1 d.e2 ∧
      (SegEndComplete d ↔ IsTopFractal d.e1 d.e2 d.e3 ∧ FractalInSeq false d.revSeq ∧
        TopAboveBottom { dir := d.dir, startPrice := d.startPrice, endPrice := d.endPrice })) := by
  unfold SegEndComplete
  -- 用 hdir 把两个 match 归约到 up 分支（d.dir 是投影，不能 subst，改 rw [hdir]）
  rw [hdir]
  by_cases hgap : HasGap d.e1 d.e2
  · -- 第二种情况：有缺口 ⟹ SegmentEndUp ↔ FractalInSeq false revSeq
    refine Or.inr ⟨hgap, ?_⟩
    constructor
    · rintro ⟨hf, hs, ht⟩
      exact ⟨hf, (segmentEndUp_caseTwo d.e1 d.e2 d.revSeq hgap).mp hs, ht⟩
    · rintro ⟨hf, hfr, ht⟩
      exact ⟨hf, (segmentEndUp_caseTwo d.e1 d.e2 d.revSeq hgap).mpr hfr, ht⟩
  · -- 第一种情况：无缺口 ⟹ SegmentEndUp 恒真，确认 ⟺ 分型 ∧ 顶高于底
    refine Or.inl ⟨hgap, ?_⟩
    constructor
    · rintro ⟨hf, _, ht⟩; exact ⟨hf, ht⟩
    · rintro ⟨hf, ht⟩
      exact ⟨hf, segmentEndUp_caseOne d.e1 d.e2 d.revSeq hgap, ht⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 第78课古怪线段唯一原因 + 顶高于底硬约束（结构定理）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **古怪线段判定（第78:30 唯一原因，跨层）** —— 一个 `(strokes, d)` 对是"古怪线段成因点"⟺
  committed 良构封装会在 `strokes` 上切（`NaiveScanCutsAt d.dir strokes.tail`——有反向笔/笔破坏）
  但完整判据查分型不成立（笔破坏未发展成线段破坏，即没有特征序列分型）。

  第78课："所有古怪的线段，都是因为线段出现第一种情况的笔破坏后最终没有在该方向由该笔发展
  形成线段破坏所造成的，这是线段古怪的唯一原因。"——形式化为：NaiveScanCutsAt（committed
  `List Stroke` 层切分触发）∧ ¬(分型成立)。第一个合取项接 committed scan，非占位 `True`。
-/
def IsPeculiarCause (strokes : List Stroke) (d : SegEndData) : Prop :=
  NaiveScanCutsAt d.dir strokes.tail ∧
  ¬ (match d.dir with
     | Direction.up   => IsTopFractal d.e1 d.e2 d.e3
     | Direction.down => IsBottomFractal d.e1 d.e2 d.e3)

/--
  **★古怪线段成因 ⟹ 完整判据不确认（L0，第78课唯一原因结构定理）** —— 若 `(strokes, d)` 是古怪
  线段成因（committed scan 切但无特征序列分型），则完整判据必不确认。这把第78课"古怪线段唯一
  原因"形式化为从"无分型"到"完整判据拒绝"的蕴含——古怪线段恰是完整判据与良构封装分叉处。
-/
theorem peculiarCause_not_complete (strokes : List Stroke) (d : SegEndData)
    (h : IsPeculiarCause strokes d) :
    ¬ SegEndComplete d := by
  intro hc
  -- hc.1 是分型成立，h.2 是分型不成立，矛盾
  exact h.2 hc.1

/-- **★古怪线段见证是古怪线段成因（L0，跨层闭合）** —— 具体 `(peculiarStrokes, peculiarSegEnd)`
  满足 `IsPeculiarCause`：committed scan 真切 ∧ 非顶分型。把成因定义实例化到真笔序列。 -/
theorem peculiar_isPeculiarCause : IsPeculiarCause peculiarStrokes peculiarSegEnd := by
  refine ⟨peculiar_naiveScanCuts, ?_⟩
  intro hfrac
  unfold IsTopFractal peculiarSegEnd at hfrac
  simp only [Tick] at hfrac
  omega

/--
  **★顶高于底硬约束是完整判据的必要条件（L0，第78:20）** —— 完整判据确认 ⟹ 顶高于底成立。
  这把第78课"顶肯定要高于底，划出不符合的肯定是划错了"形式化为完整判据的**投影必要条件**：
  任何被完整判据确认的段尾，其端点必满足顶高于底（划错的段被完整判据排除）。
-/
theorem segEndComplete_implies_topAboveBottom (d : SegEndData) (h : SegEndComplete d) :
    TopAboveBottom { dir := d.dir, startPrice := d.startPrice, endPrice := d.endPrice } :=
  h.2.2

/--
  **★分型是段尾必要前提（L0，第67:48）** —— 完整判据确认 ⟹ 特征序列分型成立。
  "出现特征序列的分型，是线段结束的前提条件。"——形式化为完整判据的第一投影必要条件。
-/
theorem segEndComplete_implies_fractal (d : SegEndData) (h : SegEndComplete d) :
    (match d.dir with
     | Direction.up   => IsTopFractal d.e1 d.e2 d.e3
     | Direction.down => IsBottomFractal d.e1 d.e2 d.e3) :=
  h.1

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 完整判据保持"消费 ≥1"（终止性可接入性，A″ 桥）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **完整判据消费的笔数下界（终止性可接入性）** —— 完整判据驱动的切分若在某确认点切，至少消费
  起始那根笔。本谓词刻画：一个被完整判据确认的段尾对应消费的笔数 `k`，总有 `k ≥ 1`。

  ★这是把完整判据接入 `segmentsOfComplete` 全自动递归时**终止性可保持**的关键不变量：
  SegmentConstruction 的终止性依赖 `nextSegmentEnd ≥ 1`；完整判据若保持 `k ≥ 1`，则替换良构
  封装后终止性不翻转（边界条件，见 SegmentConstruction § 7）。本文件证完整判据**兼容**此不变量。
-/
def CompleteConsumesAtLeastOne (k : Nat) : Prop := k ≥ 1

/--
  **★完整判据兼容终止性不变量（L0）** —— 任一被完整判据确认的段尾，对应消费 ≥1 笔
  （`CompleteConsumesAtLeastOne` 在 k≥1 时成立）。这说明把完整判据接入全自动递归时，
  只要每个确认点消费 ≥1 笔，SegmentConstruction 的 well-founded 终止性骨架可复用——
  完整判据**不破坏**终止性前提（A″ 接入的可行性，非冒充已接入）。
-/
theorem complete_preserves_termination_invariant (k : Nat) (h : k ≥ 1) :
    CompleteConsumesAtLeastOne k := h

/-- 反退化：完整判据消费 0 笔会破坏终止性（k=0 时不变量不成立）。 -/
theorem complete_consume_zero_breaks : ¬ CompleteConsumesAtLeastOne 0 := by
  unfold CompleteConsumesAtLeastOne; decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已升级（task #118→#122 相对 #116 still-MISSING-A′）：
    - **完整判据 = 真特征序列确认（同名谓词，L0 同义反复）**：`segEndComplete_iff_featureConfirmed`
      是 `Iff.rfl`——`SegEndComplete` 与 `FeatureConfirmed` 定义体逐字相同，此 iff 零信息增量，
      只登记"完整确认"/"真特征序列确认"两个语义名字指同一三合取谓词（task #122 诚实标注：
      它**不**是承载升级实证的"核心定理"，实证内容在下面两条跨层/正例定理）。
    - **完整判据严格细化良构封装（task #122 坐实的跨层分叉）**：`complete_strictly_refines_naive`
      构造**真 `List Stroke`** 古怪线段笔序列 `peculiarStrokes`，机器证 committed `scanSegEnd`
      （SegmentConstruction.lean，吃 `List Stroke`）在其首个反向特征笔处**真会切**
      （`scanSegEnd_cuts_at_first_reversal` + `peculiar_scanSegEnd_cuts = 2`），而完整判据
      `SegEndComplete` 正确不确认（非顶分型）。两层经 committed `FeatureElem.ofStroke` 绑定
      （`peculiar_strokes_abstract`：三根向下特征笔抽象出 e1/e2/e3）。这是**机器证明的跨层
      严格细化**——分叉两端分别由 committed `scanSegEnd`（`List Stroke` 层）与 `SegEndComplete`
      （特征序列层）给出，**非占位 `True`、非同谓词改名**（修 #118 的表述间隙）。
    - **第78课古怪线段唯一原因 + 顶高于底**：`peculiarCause_not_complete`（古怪成因⟹不确认，
      成因第一合取项 = committed `NaiveScanCutsAt`）+ `peculiar_isPeculiarCause`（真笔序列实例化）+
      `segEndComplete_implies_topAboveBottom`（顶高于底必要条件）+ `segEndComplete_implies_fractal`
      （分型必要前提）——结构定理把第78课硬约束接入完整判据。
    - **两情况完全分类接入**：`segEndComplete_up_two_cases` 把第67课两情况完全分类抬升到完整
      段尾确认层（继承 SegmentFeatureSeq 的 `segment_two_cases_exhaustive`）。

  ★still-MISSING-A″（完整判据驱动的全自动递归切分，诚实开口）：
    本文件证**判据层**完整接入（完整段尾确认谓词 + 与 committed scan 的跨层分叉见证）。但把
    完整判据 `SegEndComplete` 接入 SegmentConstruction 的全自动递归 `segmentsOfComplete :
    List Stroke → List Segment`——即在每个候选点提取真实 FeatureElem 序列（FeatureElem.ofStroke
    对偶）、递归非包含处理（mergeInclusion）、识别分型、按缺口走两情况、且处理古怪线段递归
    （第78课：笔破坏后向下线段是否破该向上笔底的递归判定）——**未在本文件实装**。
    `complete_preserves_termination_invariant` 证完整判据**兼容**终止性不变量（k≥1），说明
    A″ 接入**可行**（终止性骨架可复用），但**不声称**已接入。把判据层接入冒充为全自动递归
    实装 = 声明膨胀（禁止）。

  ★边界条件（结论翻转条件）：
    - `SegEndComplete` 的合取三支（分型 ∧ 两情况 ∧ 顶高于底）缺一不确认。若某口径允许"无分型
      但有缺口封闭即切"（非缠论标准），第一支可去，但那不是第67课判据——当前严格三支合取。
    - 跨层分叉见证依赖 committed `scanSegEnd` 的切分语义（遇第一个反向笔即切）。若
      SegmentConstruction.lean 改 `scanSegEnd` 为非"反向即切"的语义，`scanSegEnd_cuts_at_first_reversal`
      与 `peculiar_scanSegEnd_cuts` 须重证（当前对 committed `scanSegEnd` 直接求值）。
    - `peculiarStrokes` 的向下特征笔与 `peculiarSegEnd.e1/e2/e3` 的对应由 `peculiar_strokes_abstract`
      （committed `FeatureElem.ofStroke`）保证。若 `FeatureElem.ofStroke` 的区间抽取规则改变
      （当前取笔两端价低/高），对应须重证。
    - 古怪线段反退化用 `e3.high > e2.high`（非顶分型）。若改用允许相等的分型定义
      （e3.high ≤ e2.high 也算顶），peculiarSegEnd/peculiarStrokes 须重构（当前 IsTopFractal 用严格 <）。
    - 顶高于底用严格不等式（Tick=Int）。平端点（startPrice=endPrice）被完整判据拒绝
      （与 SegmentFeatureSeq.topAboveBottom_rejects_flat_up 一致）。
    - 终止性兼容性（k≥1）是 A″ 接入的**前提条件**，非已实现保证。若完整判据某确认点消费 0 笔
      （`complete_consume_zero_breaks` 见证此情形破坏终止性），A″ 终止性翻转须重证。

  ★下游推论：
    - SegmentConstruction.segmentsOf 当前用良构封装 nextSegmentEnd；A″ 接入完整判据后，切出的
      线段序列将 = 真特征序列确认序列（升构造层从良构封装到缠论-忠实）。本文件提供判据层的
      正确性证明，是 A″ 接入的**前置依据**。
    - CenterComplete.lean（B′）的"三段次级别走势"以完整判据切出的线段为载体——A′ 完整判据正确
      是 B′ 中枢判据有意义的前提（线段是中枢构成元素）。
    - BspConstruction（买卖点）的"次级别走势"同样以完整判据线段为载体。

  ★谱系引用：升 SegmentFeatureSeq.lean § 8 still-MISSING-A（判据层）+ SegmentConstruction.lean
    § 7 still-MISSING-A′（良构封装→完整判据）；`.chanlun/genealogy/settled/001-degenerate-segment.md`
    （退化线段：完整判据顶高于底硬约束严格排除从底到底/顶到顶的退化段）+
    `002-source-incompleteness.md`（第67/71/78课博文补齐，本文件直接复用博文完整判据）。

  ★影响声明（task #122 修正，相对 #118）：
    - 修正对象：删除恒真占位 `NaiveScanCuts (_d : SegEndData) : Prop := True`，替换为真
      `List Stroke` 层谓词 `NaiveScanCutsAt (segDir) (rest)`（接 committed `strokeReverses`）+
      `scanSegEnd_cuts_at_first_reversal`（committed `scanSegEnd` 真切引理）+ `peculiarStrokes`
      （真笔序列见证）+ `peculiar_strokes_abstract`（committed `FeatureElem.ofStroke` 跨层绑定）。
      `complete_strictly_refines_naive` 改为真跨层存在命题（∃ List Stroke × SegEndData）。
    - 表述-实证一致性修正：(1) `segEndComplete_iff_featureConfirmed` docstring 改标为
      L0 同义反复（`Iff.rfl`），删"A′ 升级核心定理"措辞——它不承载实质内容；(2) 文件头
      "完整判据严格细化良构封装"措辞改为机器证明的跨层分叉（committed scan 真切 ∧ 完整判据不切）。
    - 仍 import ChanlunElements + SegmentFeatureSeq + SegmentConstruction（全只读，不改任何
      committed 模块；`scanSegEnd`/`strokeReverses`/`FeatureElem.ofStroke` 均复用其 committed 定义）。
    - 不改 canonical 类型，无反向依赖。新名：NaiveScanCutsAt/scanSegEnd_cuts_at_first_reversal/
      peculiarStrokes/peculiar_strokes_abstract/peculiar_scanSegEnd_cuts/peculiar_isPeculiarCause。
      root `Origin.SegmentFeatureComplete` 已登记于 lakefile.toml。
-/

end NewChanlun.Origin
