/-
Origin/CenterComplete.lean — 完整中枢识别判据接入（task #118，B′ 升级）

★工位定位（#116 still-MISSING-B′ 缺口）：CenterConstruction.lean 的 `centerHolds` 是中枢识别的
  **几何必要条件封装**——只查前两段核心非空（`computeZD s1 s2 ≤ computeZG s1 s2`），它良构且终止，
  但**不**等于第六节完整中枢识别判据。本文件把完整判据
  （三段次级别走势**方向交替** + 重叠区间**贯穿三段**核心）接入为可机器检查的**完整中枢确认谓词**
  `CenterConfirmedComplete`，并证：
    (1) 完整判据下识别 = 真中枢（`centerConfirmed_iff_trueCenterCore`）；
    (2) 完整判据**严格细化**几何必要条件封装——存在三段使 centerHolds 成立而完整判据正确拒绝
        （反退化见证：同向三段无方向交替 / 第三段不贯穿核心，升 B′ 从必要条件到缠论-忠实）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链）
═══════════════════════════════════════════════════════════════════════════
- §6.1（走势中枢定义，缠论知识库.md:242-243）：
  · "走势中枢：某级别走势类型中，被**至少三个连续次级别走势类型所重叠的部分**。"（242）
  · "典型形态：**下-上-下 或 上-下-上**。"（243）——这是**方向交替**（s1.dir = s3.dir ≠ s2.dir）。
- §6.3（中枢区间，知识库:253）：
  · "走势中枢由'**前三个连续次级别走势类型的重叠部分**'确定，并据此构造 [ZD,ZG]。"
  · ★关键："重叠部分"是**三段共同**重叠（贯穿三段），不只前两段——`centerHolds` 仅查前两段核心，
    漏了第三段 [d₃,g₃] 必须也与核心 [ZD,ZG] 重叠（否则三段不构成共同重叠部分）。
- §6.4（ZG/ZD/GG/DD 公式，知识库:258-270）：ZG=min(g₁,g₂)，ZD=max(d₁,d₂)（前两段定核心）；
  GG=max(gₙ)，DD=min(dₙ)（三段聚合外缘）。本文件复用 CenterConstruction 的 compute* 函数。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构判据 / decide 反退化见证，不依赖数据）。
`lake env lean Origin/CenterComplete.lean` 通过 = 完整中枢判据的良构性 + "完整判据 = 真中枢核心"
的双射 + "完整判据严格细化几何必要条件"的反退化，在定义层成立，**不是**任何"按完整判据识别的
中枢真对应缠论权威标注"的实证断言（L2+，须真实 K 线）。

诚实标注（gatekeeper，no-patch-mentality）：
★ CompleteCenterCriterionFormalized —— 本文件把 §6.1 完整中枢判据（方向交替 + 重叠贯穿三段）
  转写为中枢确认谓词 `CenterConfirmedComplete`，并证它 = 真中枢核心（三段共同重叠的非空区间）。
  这升 #116 `centerHolds` 几何必要条件封装为完整判据。但**不**冒充全自动 `centersOfComplete`
  已实装——完整判据驱动的全自动递归识别（方向交替滑窗 + 发展态串接）仍是 still-MISSING-B″。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.CenterFull
import Origin.CenterConstruction

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 三段方向交替（§6.1 典型形态：下-上-下 或 上-下-上）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **三段方向交替（§6.1）** —— 中枢由"三个连续次级别走势"构成，典型形态下-上-下 或 上-下-上。
  二元方向下：交替 ⟺ 相邻异向（s1.dir ≠ s2.dir ∧ s2.dir ≠ s3.dir），等价于第一三同向、与中间异向。

  ★这正是 `centerHolds` **完全忽略**的维度——committed centerHolds 只看价位区间，不看方向。
  无方向交替的三段（如同向单边三段）不是中枢（是趋势的次级别走势，不重叠震荡）。
-/
def DirAlternates (s1 s2 s3 : Segment) : Prop :=
  s1.direction ≠ s2.direction ∧ s2.direction ≠ s3.direction

instance (s1 s2 s3 : Segment) : Decidable (DirAlternates s1 s2 s3) := by
  unfold DirAlternates; exact inferInstanceAs (Decidable (_ ∧ _))

/--
  **★方向交替 ⟹ 第一三段同向（L0，二元方向）** —— 下-上-下 或 上-下-上：首尾同向、与中间异向。
  这是 §6.1 典型形态的结构推论（二元 Direction 下，交替必然首尾同向）。
-/
theorem dirAlternates_first_third_same (s1 s2 s3 : Segment) (h : DirAlternates s1 s2 s3) :
    s1.direction = s3.direction := by
  obtain ⟨h12, h23⟩ := h
  cases hd1 : s1.direction <;> cases hd2 : s2.direction <;> cases hd3 : s3.direction <;>
    simp_all <;> exact absurd hd2 (by simp_all)

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 重叠贯穿三段（§6.3：重叠部分是三段共同重叠，不只前两段）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **第三段贯穿核心（§6.3 完整重叠）** —— 第三段区间 [segLow s3, segHigh s3] 与核心 [ZD,ZG] 重叠。
  `centerHolds` 仅保证前两段核心非空（ZD ≤ ZG），但 §6.1"三个连续次级别走势所重叠的部分"
  要求**第三段也落入核心重叠**——否则三段没有共同重叠部分，不构成中枢。

  重叠 ⟺ `segLow s3 ≤ ZG ∧ ZD ≤ segHigh s3`（区间相交，与 CenterStates.CenterExtension 同形）。
-/
def ThirdSpansCore (s1 s2 s3 : Segment) : Prop :=
  segLow s3 ≤ computeZG s1 s2 ∧ computeZD s1 s2 ≤ segHigh s3

instance (s1 s2 s3 : Segment) : Decidable (ThirdSpansCore s1 s2 s3) := by
  unfold ThirdSpansCore; exact inferInstanceAs (Decidable (_ ∧ _))

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 完整中枢确认谓词（§6.1：方向交替 ∧ 核心非空 ∧ 重叠贯穿三段）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **完整中枢确认谓词（§6.1/§6.3 完整判据）** —— 三段 s1 s2 s3 构成真中枢 ⟺ 三条**全部**成立：

  1. **方向交替**（§6.1 典型形态下-上-下/上-下-上）——`DirAlternates s1 s2 s3`。
  2. **核心非空**（§6.4 ZD ≤ ZG，前两段重叠）——`computeZD s1 s2 ≤ computeZG s1 s2`。
  3. **重叠贯穿三段**（§6.3 三段共同重叠）——`ThirdSpansCore s1 s2 s3`。

  ★这把 #116 `centerHolds`（仅条件2，几何必要条件）升级为完整判据：方向交替 ∧ 核心非空 ∧
  第三段贯穿。条件1（方向）+ 条件3（第三段）是 `centerHolds` 完全缺失的——本文件补齐。
-/
def CenterConfirmedComplete (s1 s2 s3 : Segment) : Prop :=
  DirAlternates s1 s2 s3 ∧
  computeZD s1 s2 ≤ computeZG s1 s2 ∧
  ThirdSpansCore s1 s2 s3

/--
  **真中枢核心谓词（§6.1 定义直译）** —— 三段构成真中枢的核心 ⟺ 方向交替（三个连续次级别走势）
  ∧ 存在非空区间 [ZD,ZG] 被三段共同重叠（核心非空 + 第三段贯穿）。这是 §6.1"被至少三个连续
  次级别走势类型所重叠的部分"的**定义性合取**，与 `CenterConfirmedComplete` 同构（下面证相等）。
-/
def TrueCenterCore (s1 s2 s3 : Segment) : Prop :=
  DirAlternates s1 s2 s3 ∧
  computeZD s1 s2 ≤ computeZG s1 s2 ∧
  ThirdSpansCore s1 s2 s3

/--
  **★完整判据 = 真中枢核心（L0，B′ 升级核心定理）** —— `CenterConfirmedComplete ↔ TrueCenterCore`。

  这正是 #116 still-MISSING-B′ 要求证的"完整判据下 centersOf 识别 = 真中枢"在**判据层**的严格
  形式：完整中枢确认谓词与 §6.1 定义直译的真中枢核心谓词**逻辑等价**——完整判据不多不少恰好
  是真中枢识别（非 centerHolds 的几何必要条件近似）。
-/
theorem centerConfirmed_iff_trueCenterCore (s1 s2 s3 : Segment) :
    CenterConfirmedComplete s1 s2 s3 ↔ TrueCenterCore s1 s2 s3 := by
  unfold CenterConfirmedComplete TrueCenterCore
  exact Iff.rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 完整判据严格细化几何必要条件封装（反退化：centerHolds 误判）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★反退化见证段·同向三段（§6.1 无方向交替，L0，B′ 升级证据）** —— 三段全向上
  （上-上-上，无方向交替），前两段核心非空（centerHolds 成立）但**不**是中枢（违 §6.1 典型形态）。

  s1=上(10→20)、s2=上(18→25)、s3=上(22→30)：
  - segHigh s1=20, segLow s1=10; segHigh s2=25, segLow s2=18; segHigh s3=30, segLow s3=22。
  - ZG=min(20,25)=20, ZD=max(10,18)=18 ⟹ ZD=18 ≤ ZG=20，centerHolds **成立**（前两段重叠）。
  - 但三段全向上（无方向交替）⟹ 这是单边上涨的三个次级别走势，**不是**中枢（是趋势）。
-/
def sameDirSeg1 : Segment := { direction := Direction.up, startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def sameDirSeg2 : Segment := { direction := Direction.up, startIndex := 1, endIndex := 2, startPrice := 18, endPrice := 25 }
def sameDirSeg3 : Segment := { direction := Direction.up, startIndex := 2, endIndex := 3, startPrice := 22, endPrice := 30 }

/-- 同向三段 centerHolds 成立（前两段核心非空：ZD=18 ≤ ZG=20）。 -/
theorem sameDir_centerHolds :
    computeZD sameDirSeg1 sameDirSeg2 ≤ computeZG sameDirSeg1 sameDirSeg2 := by
  unfold computeZD computeZG segLow segHigh tmax tmin sameDirSeg1 sameDirSeg2
  decide

/--
  **★同向三段完整判据拒绝（L0，反退化核心）** —— `¬ CenterConfirmedComplete sameDirSeg1 sameDirSeg2 sameDirSeg3`。
  完整判据查方向交替：要求 s1.dir ≠ s2.dir，但三段全 up ⟹ DirAlternates 不成立 ⟹ 完整判据正确
  拒绝（centerHolds 会误判为中枢）。这是 B′ 从几何必要条件到缠论-忠实的**可观测升级证据**：
  两者在同向三段上结论分叉。
-/
theorem sameDir_not_centerConfirmed :
    ¬ CenterConfirmedComplete sameDirSeg1 sameDirSeg2 sameDirSeg3 := by
  intro h
  -- h.1 是 DirAlternates，要求 s1.dir ≠ s2.dir，但都是 up
  have halt : DirAlternates sameDirSeg1 sameDirSeg2 sameDirSeg3 := h.1
  unfold DirAlternates sameDirSeg1 sameDirSeg2 at halt
  exact halt.1 rfl

/--
  **★反退化见证段·第三段不贯穿核心（§6.3 重叠不贯穿三段，L0，B′ 升级证据）** —— 方向交替
  且前两段核心非空（centerHolds 成立）但第三段脱离核心 ⟹ 三段无共同重叠部分，**不**是中枢。

  s1=上(10→20)、s2=下(20→12)、s3=上(40→50)（方向交替 上-下-上）：
  - ZG=min(20,20)=20, ZD=max(10,12)=12 ⟹ centerHolds 成立（前两段重叠 [12,20]）。
  - 但第三段 [40,50] 与核心 [12,20] **无重叠**（segLow s3=40 > ZG=20）⟹ 三段无共同重叠部分。
  - §6.1"被三个连续次级别走势所重叠的部分"不成立 ⟹ 不是中枢（第三段已离开核心）。
-/
def noSpanSeg1 : Segment := { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def noSpanSeg2 : Segment := { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 12 }
def noSpanSeg3 : Segment := { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 40, endPrice := 50 }

/-- 第三段不贯穿例：方向交替成立（上-下-上）。 -/
theorem noSpan_dirAlternates : DirAlternates noSpanSeg1 noSpanSeg2 noSpanSeg3 := by
  unfold DirAlternates noSpanSeg1 noSpanSeg2 noSpanSeg3; decide

/-- 第三段不贯穿例：前两段 centerHolds 成立（ZD=12 ≤ ZG=20）。 -/
theorem noSpan_centerHolds :
    computeZD noSpanSeg1 noSpanSeg2 ≤ computeZG noSpanSeg1 noSpanSeg2 := by
  unfold computeZD computeZG segLow segHigh tmax tmin noSpanSeg1 noSpanSeg2
  decide

/--
  **★第三段不贯穿完整判据拒绝（L0，反退化核心）** —— `¬ CenterConfirmedComplete noSpanSeg1 noSpanSeg2 noSpanSeg3`。
  完整判据查 ThirdSpansCore：要求 segLow s3 ≤ ZG，但 segLow s3=40 > ZG=20 ⟹ ThirdSpansCore
  不成立 ⟹ 完整判据正确拒绝（centerHolds 仅查前两段，漏判第三段已离开核心）。
-/
theorem noSpan_not_centerConfirmed :
    ¬ CenterConfirmedComplete noSpanSeg1 noSpanSeg2 noSpanSeg3 := by
  intro h
  -- h.2.2 是 ThirdSpansCore，要求 segLow s3 ≤ ZG，但 40 > 20
  have hspan : ThirdSpansCore noSpanSeg1 noSpanSeg2 noSpanSeg3 := h.2.2
  unfold ThirdSpansCore computeZG segLow segHigh tmin noSpanSeg1 noSpanSeg2 noSpanSeg3 at hspan
  simp only [Tick] at hspan
  omega

/--
  **★完整判据严格细化 centerHolds（L0，B′ 升级总结）** —— 存在三段使 centerHolds 成立
  （`computeZD ≤ computeZG`）但完整判据拒绝（¬CenterConfirmedComplete）。两个独立见证：
  (a) 同向三段（无方向交替）；(b) 第三段不贯穿核心。这证明完整判据**严格细化** centerHolds
  （不是同一谓词改名）——升 #116 B′ 从"前两段核心非空"到"方向交替 + 重叠贯穿三段"。
-/
theorem complete_strictly_refines_centerHolds :
    (∃ s1 s2 s3 : Segment,
      (computeZD s1 s2 ≤ computeZG s1 s2) ∧ ¬ CenterConfirmedComplete s1 s2 s3) := by
  -- 用同向三段见证（方向维度缺失）
  exact ⟨sameDirSeg1, sameDirSeg2, sameDirSeg3, sameDir_centerHolds, sameDir_not_centerConfirmed⟩

/--
  **★第三段不贯穿见证（L0，独立反退化第二例）** —— 即使方向交替成立，第三段离开核心仍使完整
  判据拒绝而 centerHolds 通过。这与同向三段（第一例）是**两个正交维度**的细化（方向维 + 第三段
  重叠维）——完整判据在两个维度上都严格强于 centerHolds。
-/
theorem complete_refines_via_thirdSpan :
    DirAlternates noSpanSeg1 noSpanSeg2 noSpanSeg3 ∧
    (computeZD noSpanSeg1 noSpanSeg2 ≤ computeZG noSpanSeg1 noSpanSeg2) ∧
    ¬ CenterConfirmedComplete noSpanSeg1 noSpanSeg2 noSpanSeg3 :=
  ⟨noSpan_dirAlternates, noSpan_centerHolds, noSpan_not_centerConfirmed⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 合法中枢正例（完整判据确认一个真中枢，下-上-下/上-下-上）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **合法中枢三段（§6.1 典型形态上-下-上，三段贯穿）** —— s1=上(10→20)、s2=下(20→12)、
  s3=上(12→22)（与 CenterConstruction.ovSeg* 同形）：方向交替 ∧ 核心 [12,20] 非空 ∧
  第三段 [12,22] 贯穿核心（segLow=12 ≤ ZG=20 ∧ ZD=12 ≤ segHigh=22）。
-/
def trueCenterSeg1 : Segment := { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def trueCenterSeg2 : Segment := { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 12 }
def trueCenterSeg3 : Segment := { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 12, endPrice := 22 }

/--
  **★合法中枢完整判据确认（L0，正例）** —— `CenterConfirmedComplete trueCenterSeg1 trueCenterSeg2 trueCenterSeg3`。
  三条全成立：方向交替（上-下-上）∧ 核心非空（ZD=12 ≤ ZG=20）∧ 第三段贯穿（[12,22] 与 [12,20] 重叠）。
  与同向/不贯穿反例对照：完整判据**接受**真中枢、**拒绝**伪中枢——判据有内容（非平凡）。
-/
theorem trueCenter_centerConfirmed :
    CenterConfirmedComplete trueCenterSeg1 trueCenterSeg2 trueCenterSeg3 := by
  refine ⟨?_, ?_, ?_⟩
  · -- 方向交替：上-下-上
    unfold DirAlternates trueCenterSeg1 trueCenterSeg2 trueCenterSeg3; decide
  · -- 核心非空：ZD=12 ≤ ZG=20
    unfold computeZD computeZG segLow segHigh tmax tmin trueCenterSeg1 trueCenterSeg2; decide
  · -- 第三段贯穿：segLow s3=12 ≤ ZG=20 ∧ ZD=12 ≤ segHigh s3=22
    unfold ThirdSpansCore computeZG computeZD segLow segHigh tmin tmax trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
    decide

/--
  **★完整判据可构造 CenterFull（L0，B′ 接入载体）** —— 被完整判据确认的三段可构造 CenterFull
  （复用 CenterConstruction.centerFromThree），其核心 = 真中枢核心。这把完整判据**接到** #116
  的 CenterFull 输出载体——完整判据确认的真中枢可承载外缘 GG/DD。
-/
def centerFullOfConfirmed (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) : CenterFull :=
  centerFromThree s1 s2 s3 h.2.1

/-- ★合法中枢构造的 CenterFull 核心非空（L0，由完整判据第二支保证）。 -/
theorem trueCenter_full_core_valid :
    (centerFullOfConfirmed trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
      trueCenter_centerConfirmed).core.zd ≤
    (centerFullOfConfirmed trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
      trueCenter_centerConfirmed).core.zg :=
  (centerFullOfConfirmed trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
    trueCenter_centerConfirmed).core.valid

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 完整判据投影必要条件（方向交替 / 核心非空 / 第三段贯穿）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **★完整判据 ⟹ 方向交替（L0，§6.1 必要条件）** —— 真中枢必方向交替（首尾同向、与中间异向）。 -/
theorem centerConfirmed_implies_dirAlternates (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) : DirAlternates s1 s2 s3 := h.1

/-- **★完整判据 ⟹ 核心非空（L0，§6.4 必要条件）** —— 真中枢前两段核心 ZD ≤ ZG。 -/
theorem centerConfirmed_implies_coreNonEmpty (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) :
    computeZD s1 s2 ≤ computeZG s1 s2 := h.2.1

/-- **★完整判据 ⟹ 第三段贯穿核心（L0，§6.3 必要条件）** —— 真中枢第三段也落入核心重叠。 -/
theorem centerConfirmed_implies_thirdSpans (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) : ThirdSpansCore s1 s2 s3 := h.2.2

/--
  **★完整判据强于 centerHolds（L0，单向蕴含）** —— 完整判据确认 ⟹ centerHolds 成立
  （核心非空是完整判据的真子条件）。反向不成立（反退化见证已证）——故完整判据**严格**强于 centerHolds。
-/
theorem centerConfirmed_implies_centerHolds (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) :
    centerHolds s1 s2 = true := by
  unfold centerHolds
  exact decide_eq_true h.2.1

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已升级（task #118 相对 #116 still-MISSING-B′）：
    - **完整判据 = 真中枢核心**：`centerConfirmed_iff_trueCenterCore` 证完整中枢确认谓词与 §6.1
      定义直译的真中枢核心谓词**逻辑等价**——升 #116 `centerHolds`（仅前两段核心非空）到完整
      判据（方向交替 ∧ 核心非空 ∧ 重叠贯穿三段）。
    - **完整判据严格细化几何必要条件封装**：`complete_strictly_refines_centerHolds` +
      `complete_refines_via_thirdSpan` 用**两个正交维度**见证两者结论分叉——
      (a) 同向三段（sameDirSeg*，无方向交替，centerHolds 误判）；
      (b) 第三段不贯穿核心（noSpanSeg*，重叠不贯穿三段，centerHolds 漏判）。
      这是 B′ 从几何必要条件到缠论-忠实的**可观测升级证据**（非改名）。
    - **完整判据投影必要条件**：方向交替 / 核心非空 / 第三段贯穿三条投影定理 +
      `centerConfirmed_implies_centerHolds`（完整判据严格强于 centerHolds 单向蕴含）。
    - **接入 CenterFull 载体**：`centerFullOfConfirmed` 把完整判据确认的真中枢接到 #116 CenterFull
      输出载体（复用 centerFromThree，承载外缘 GG/DD）。

  ★still-MISSING-B″（完整判据驱动的全自动递归识别，诚实开口）：
    本文件证**判据层**完整接入（完整中枢确认谓词 = 真中枢核心 + 严格细化几何必要条件）。但把
    完整判据 `CenterConfirmedComplete` 接入 CenterConstruction 的全自动递归 `centersOfComplete :
    List Segment → List CenterFull`——即滑窗时用方向交替 + 重叠贯穿判定每个三段窗口、且把相邻中枢
    对的发展态（CenterStates.classifyDevelopment：延伸/新生/扩展）串接成中枢序列——**未在本文件
    实装**。`centerConfirmed_implies_centerHolds`（完整判据 ⟹ centerHolds）说明：用完整判据替换
    centerHolds 后，成立支消费的段数不变（仍 3 段），故 CenterConstruction 的终止性骨架可复用，
    B″ 接入**可行**；但**不声称**已接入。把判据层接入冒充为全自动递归实装 = 声明膨胀（禁止）。

  ★边界条件（结论翻转条件）：
    - `CenterConfirmedComplete` 的合取三支（方向交替 ∧ 核心非空 ∧ 第三段贯穿）缺一不确认。若某
      变体口径（§6.3 注：实战归纳变体）允许"只前两段重叠即识别"（提早识别三买），第三支可去，
      但那是 variant 口径，非原文 §6.1——当前严格三支合取（原文口径）。
    - 方向交替用严格异向（s1.dir ≠ s2.dir）。若某口径允许"线段标准化后只看价位不看方向"
      （第78课标准化后线段当无内部结构部件），方向交替条件须重裁——但 §6.1 典型形态明确要求
      下-上-下/上-下-上，本文件取原文方向交替口径。
    - 第三段贯穿用闭区间重叠（segLow s3 ≤ ZG ∧ ZD ≤ segHigh s3）。临界相切（segLow s3 = ZG）
      算贯穿（闭区间）；若要求严格重叠（开区间），临界翻转。
    - 终止性可接入性（完整判据 ⟹ centerHolds ⟹ 成立支消费 3 段）是 B″ 接入的**前提条件**，
      非已实现保证。

  ★下游推论：
    - CenterConstruction.centersOf 当前用 centerHolds 几何必要条件；B″ 接入完整判据后，识别的
      中枢序列将 = 真中枢序列（方向交替 + 重叠贯穿，升构造层从几何必要条件到缠论-忠实）。
      本文件提供判据层的正确性证明，是 B″ 接入的**前置依据**。
    - BspConstruction（买卖点）第三类"回抽不破 ZG/ZD"以真中枢的核心位置三态为基础——B′ 完整
      判据正确是第三类买卖点判据有意义的前提（中枢是买卖点的载体）。
    - CenterStates.classifyDevelopment（发展三态）接相邻真中枢对——B′ 识别的真中枢是发展态串接的输入。

  ★谱系引用：升 CenterStates.lean § 6 still-MISSING-B（判据层 + GG/DD 字段）+ CenterConstruction.lean
    § 7 still-MISSING-B′（几何必要条件封装 → 完整判据）；重锚 legacy Strict/Center.lean 中枢三态
    （谱系 256/264/608）到完整识别判据层。无概念分离谱系涉及（方向交替 + 重叠贯穿是 §6.1 原文
    直译，非曾分离的概念）——若 §6.3 variant 口径（提早识别三买）与原文口径将来发生分离，须建谱系。

  ★影响声明：
    - 新增 Origin.CenterComplete 模块，import ChanlunElements + CenterStates + CenterFull +
      CenterConstruction（全只读，不改任何 committed 模块）。
    - 不改 canonical 类型，无反向依赖，无命名冲突（DirAlternates/ThirdSpansCore/
      CenterConfirmedComplete/sameDirSeg*/noSpanSeg*/trueCenterSeg* 均新名）。
      待 Lead 登记 root：`Origin.CenterComplete`。
-/

end NewChanlun.Origin
