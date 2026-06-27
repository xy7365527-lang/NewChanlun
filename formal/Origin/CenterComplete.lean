/-
Origin/CenterComplete.lean — 完整中枢识别判据接入（task #118，B′ 升级；637号 A→B 主塔迁移）

★工位定位（#116 still-MISSING-B′ 缺口）：CenterConstruction.lean 的 `centerHolds` 是中枢识别的
  **几何必要条件封装**——查全三段核心非空（`computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`，口径 B），
  它良构且终止，但**不**等于第六节完整中枢识别判据（缺方向交替维度）。本文件把完整判据
  （三段次级别走势**方向交替** + **全三段核心非空**）接入为可机器检查的**完整中枢确认谓词**
  `CenterConfirmedComplete`，并证：
    (1) 完整判据下识别 = 真中枢（`centerConfirmed_iff_trueCenterCore`）；
    (2) 完整判据**严格细化**几何必要条件封装——存在三段使 centerHolds 成立而完整判据正确拒绝
        （反退化见证：同向三段无方向交替，升 B′ 从必要条件到缠论-忠实）。

═══════════════════════════════════════════════════════════════════════════
★中枢核心区间口径 A→B 迁移（637号谱系裁决，一级权威第17课答疑严格公式）
═══════════════════════════════════════════════════════════════════════════
一级权威（缠师第17课答疑）：三个连续次级别走势 A、B、C 高低点 a1/a2, b1/b2, c1/c2，中枢区间 =
`(max(a2,b2,c2), min(a1,b1,c1))`——**全三段重叠**（口径 B）。误口径 A（前两段 `min(g₁,g₂)/
max(d₁,d₂)`）只在第三段贯穿核心时与 B 重合。CenterConstruction 的 computeZD/computeZG 已迁 B
（三参全三段）；本文件判据层随之迁 B。

★第三段贯穿吸收进核心非空（codex 异质核验 L0 等价，637号；rust center.rs:25-32 同源坐实）：
旧 A 口径完整判据三支 [方向交替 ∧ 前两段核心非空 ∧ 第三段贯穿 `ThirdSpansCore`] 与 B 口径两支
[方向交替 ∧ 全三段核心非空 `computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`] **接受/拒绝集严格相等**：
恒等式 `max(d₁,d₂,d₃) ≤ min(g₁,g₂,g₃)` ⟺ [前两段核心非空 ∧ 第三段落入前两段核心]，其中交叉不等式
`d₃ ≤ g₃` 由段不变量 `segLow ≤ segHigh`（segLow_le_segHigh）永真补齐。故 B 口径下 `ThirdSpansCore`
被全三段核心非空**蕴含**——保留它作独立判据支 = 声明膨胀（no-patch 禁止）。本文件将原
`ThirdSpansCore` 主判据合取支**降为派生 lemma** `thirdSpansFrontTwoCore_of_coreNonEmpty`
（全三段核心非空 → 第三段贯穿前两段核心，换名换位置换用途）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链）
═══════════════════════════════════════════════════════════════════════════
- §6.1（走势中枢定义，缠论知识库.md:242-243）：
  · "走势中枢：某级别走势类型中，被**至少三个连续次级别走势类型所重叠的部分**。"（242）
  · "典型形态：**下-上-下 或 上-下-上**。"（243）——这是**方向交替**（s1.dir = s3.dir ≠ s2.dir）。
- §6.3（中枢区间，知识库:253）：
  · "走势中枢由'**前三个连续次级别走势类型的重叠部分**'确定，并据此构造 [ZD,ZG]。"
  · ★关键："重叠部分"是**三段共同**重叠（贯穿三段）——口径 B 的 `computeZD s1 s2 s3 ≤
    computeZG s1 s2 s3` 即"三段有共同重叠区间"，自然含第三段（不需独立 ThirdSpansCore 支）。
- §6.4（ZG/ZD/GG/DD 公式，知识库:258-270 + 第17课答疑）：ZG=min(g₁,g₂,g₃)，ZD=max(d₁,d₂,d₃)
  （**全三段**定核心，口径 B）；GG=max(gₙ)，DD=min(dₙ)（三段聚合外缘）。本文件复用
  CenterConstruction 的 compute* 函数（已三参全三段）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构判据 / decide 反退化见证，不依赖数据）。A→B 迁移是 L0 接受谓词等价重构
（接受集不变，第三段贯穿吸收进全三段核心非空）+ 核心数值收窄（B 收窄到三段交，`ZD_B≥ZD_A`、
`ZG_B≤ZG_A`，非语义变更）。
`lake env lean Origin/CenterComplete.lean` 通过 = 完整中枢判据（口径 B）的良构性 + "完整判据 =
真中枢核心"的双射 + "完整判据严格细化几何必要条件"的反退化，在定义层成立，**不是**任何"按完整
判据识别的中枢真对应缠论权威标注"的实证断言（L2+，须真实 K 线）。

诚实标注（gatekeeper，no-patch-mentality）：
★ CompleteCenterCriterionFormalized —— 本文件把 §6.1 完整中枢判据（方向交替 + 全三段核心非空）
  转写为中枢确认谓词 `CenterConfirmedComplete`（口径 B 两支），并证它 = 真中枢核心（三段共同重叠的
  非空区间）。这升 #116 `centerHolds` 几何必要条件封装为完整判据。但**不**冒充全自动
  `centersOfComplete` 已实装——完整判据驱动的全自动递归识别（方向交替滑窗 + 发展态串接）仍是
  still-MISSING-B″。

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
    § 2. 前两段核心 + 第三段贯穿前两段核心（派生谓词，637号 A→B 降级）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **前两段核心上沿 ZG_A = min(g₁,g₂)（口径 A 派生量，仅本节派生谓词用）** —— 第三段贯穿判定的参照。
  口径 B canonical 核心是 `computeZG s1 s2 s3`（全三段）；本量 `frontTwoZG` 仅用于 `ThirdSpansFrontTwoCore`
  派生谓词（查第三段相对前两段核心的位置），不是主判据。
-/
def frontTwoZG (s1 s2 : Segment) : Tick := tmin (segHigh s1) (segHigh s2)
/-- **前两段核心下沿 ZD_A = max(d₁,d₂)（口径 A 派生量，仅本节派生谓词用）**。 -/
def frontTwoZD (s1 s2 : Segment) : Tick := tmax (segLow s1) (segLow s2)

/--
  **第三段贯穿前两段核心（派生谓词，637号 A→B 降级，非主判据支）** —— 第三段区间
  [segLow s3, segHigh s3] 与**前两段**核心 [ZD_A, ZG_A] 重叠。

  ★口径 A→B 降级（637号）：旧 A 口径完整判据把"第三段贯穿"作独立合取支（§6.3 三段共同重叠）。
  口径 B 迁移后 canonical 核心已是**全三段** `[computeZD s1 s2 s3, computeZG s1 s2 s3]`，第三段贯穿
  语义被全三段核心非空**吸收**：`computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`（核心非空）⟺
  [前两段核心非空 ∧ 第三段贯穿前两段核心]（codex L0 等价，`coreNonEmpty_iff_frontTwo_and_thirdSpans`）。
  故本谓词**不再**是 `CenterConfirmedComplete` 的独立判据支——降为**派生谓词**：供测试见证 A/B 等价 +
  上级几何路径复用。闭区间相切算贯穿。
-/
def ThirdSpansFrontTwoCore (s1 s2 s3 : Segment) : Prop :=
  segLow s3 ≤ frontTwoZG s1 s2 ∧ frontTwoZD s1 s2 ≤ segHigh s3

instance (s1 s2 s3 : Segment) : Decidable (ThirdSpansFrontTwoCore s1 s2 s3) := by
  unfold ThirdSpansFrontTwoCore; exact inferInstanceAs (Decidable (_ ∧ _))

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 完整中枢确认谓词（§6.1：方向交替 ∧ 全三段核心非空，口径 B 两支）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **完整中枢确认谓词（§6.1/§6.3 完整判据，口径 B 637号）** —— 三段 s1 s2 s3 构成真中枢 ⟺
  两条**全部**成立：

  1. **方向交替**（§6.1 典型形态下-上-下/上-下-上）——`DirAlternates s1 s2 s3`。
  2. **全三段核心非空**（§6.3/§6.4 口径 B：核心 = 前三段重叠部分 `[max(三段低),min(三段高)]`
     非空 ⟺ 三段有共同重叠区间）——`computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`。

  ★口径 A→B 迁移（637号）：旧 A 口径三支 [方向交替 ∧ 前两段核心非空 ∧ ThirdSpansCore] 与本 B 口径
  两支 **接受/拒绝集严格相等**——第三段贯穿被全三段核心非空吸收（`coreNonEmpty_iff_frontTwo_and_thirdSpans`
  L0 等价）。保留第三段作独立支 = 声明膨胀（no-patch 禁止），故降为派生谓词
  `ThirdSpansFrontTwoCore`（§2）。这把 #116 `centerHolds`（仅条件2，几何必要条件）升级为完整判据：
  方向交替（条件1）是 `centerHolds` 完全缺失的——本文件补齐。
-/
def CenterConfirmedComplete (s1 s2 s3 : Segment) : Prop :=
  DirAlternates s1 s2 s3 ∧
  computeZD s1 s2 s3 ≤ computeZG s1 s2 s3

/--
  **真中枢核心谓词（§6.1 定义直译，口径 B 637号）** —— 三段构成真中枢的核心 ⟺ 方向交替
  （三个连续次级别走势）∧ 存在非空区间 [ZD,ZG] 被三段共同重叠（全三段核心非空，口径 B）。这是
  §6.1"被至少三个连续次级别走势类型所重叠的部分"的**定义性合取**，与 `CenterConfirmedComplete`
  同构（下面证相等，字面同一定义）。
-/
def TrueCenterCore (s1 s2 s3 : Segment) : Prop :=
  DirAlternates s1 s2 s3 ∧
  computeZD s1 s2 s3 ≤ computeZG s1 s2 s3

/--
  **★完整判据 = 真中枢核心（L0，B′ 升级核心定理）** —— `CenterConfirmedComplete ↔ TrueCenterCore`。

  这正是 #116 still-MISSING-B′ 要求证的"完整判据下 centersOf 识别 = 真中枢"在**判据层**的严格
  形式：完整中枢确认谓词与 §6.1 定义直译的真中枢核心谓词**逻辑等价**（口径 B 下两边字面同一定义）。
-/
theorem centerConfirmed_iff_trueCenterCore (s1 s2 s3 : Segment) :
    CenterConfirmedComplete s1 s2 s3 ↔ TrueCenterCore s1 s2 s3 := by
  unfold CenterConfirmedComplete TrueCenterCore
  exact Iff.rfl

/--
  **核心非空等价的纯数值抽象引理（L0，637号 A→B 恒等式的代数核）** —— 抽象掉 segLow/segHigh，
  在 tmin/tmax 层面证：`max3(d₁,d₂,d₃) ≤ min3(g₁,g₂,g₃)` ⟺ [前两段核心非空 `max(d₁,d₂)≤min(g₁,g₂)`
  ∧ 第三段贯穿前两段核心 `d₃≤min(g₁,g₂) ∧ max(d₁,d₂)≤g₃`]，前提段不变量 `dᵢ≤gᵢ`。

  ★交叉不等式 `d₃≤g₃` 由 `h3 : d3 ≤ g3` 永真补齐——这是 9 个 pairwise 不等式中第 9 个被段不变量
  吸收的那个（rust center.rs:25-32 同源 codex L0 核验）。
-/
private theorem core_equiv_abstract (d1 g1 d2 g2 d3 g3 : Tick)
    (h1 : d1 ≤ g1) (h2 : d2 ≤ g2) (h3 : d3 ≤ g3) :
    (tmax (tmax d1 d2) d3 ≤ tmin (tmin g1 g2) g3) ↔
    ((tmax d1 d2 ≤ tmin g1 g2) ∧ (d3 ≤ tmin g1 g2 ∧ tmax d1 d2 ≤ g3)) := by
  unfold tmax tmin
  simp only [Tick] at *
  constructor
  · intro h
    refine ⟨?_, ?_, ?_⟩ <;> (repeat' split) <;> (repeat' split at h) <;> omega
  · intro ⟨ha, hb, hc⟩
    (repeat' split) <;> (repeat' split at ha) <;> (repeat' split at hb) <;>
      (repeat' split at hc) <;> omega

/--
  **★第三段贯穿吸收进全三段核心非空（L0 等价，637号 A→B 迁移核心恒等式）** ——
  `computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`（口径 B 全三段核心非空）⟺
  [前两段核心非空 `frontTwoZD ≤ frontTwoZG` ∧ 第三段贯穿前两段核心 `ThirdSpansFrontTwoCore`]。

  ★这是 A→B 迁移合法性的机器证明（rust center.rs:25-32 同源 codex L0 核验的 Lean 重证）：
  - `max(d₁,d₂,d₃) ≤ min(g₁,g₂,g₃)` 展开为 9 个 pairwise 不等式（{d₁,d₂,d₃}×{g₁,g₂,g₃}）。
  - 其中 `d₃ ≤ g₃` 由段不变量 `segLow s3 ≤ segHigh s3`（segLow_le_segHigh）永真补齐。
  - 剩余 8 个 = [前两段核心非空 `d₁≤g₁ ∧ d₂≤g₂ ∧ d₁≤g₂ ∧ d₂≤g₁`] ∧ [第三段贯穿
    `d₃≤g₁ ∧ d₃≤g₂ ∧ d₁≤g₃ ∧ d₂≤g₃`]。后者即 `segLow s3 ≤ frontTwoZG ∧ frontTwoZD ≤ segHigh s3`。
  故保留 ThirdSpansFrontTwoCore 作独立判据支是冗余（声明膨胀）——B 两支已蕴含它。
-/
theorem coreNonEmpty_iff_frontTwo_and_thirdSpans (s1 s2 s3 : Segment) :
    (computeZD s1 s2 s3 ≤ computeZG s1 s2 s3) ↔
    ((frontTwoZD s1 s2 ≤ frontTwoZG s1 s2) ∧ ThirdSpansFrontTwoCore s1 s2 s3) := by
  unfold computeZD computeZG frontTwoZD frontTwoZG ThirdSpansFrontTwoCore
  exact core_equiv_abstract (segLow s1) (segHigh s1) (segLow s2) (segHigh s2) (segLow s3) (segHigh s3)
    (segLow_le_segHigh s1) (segLow_le_segHigh s2) (segLow_le_segHigh s3)

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 完整判据严格细化几何必要条件封装（口径 B 反退化：唯一缺失维=方向）
    ═══════════════════════════════════════════════════════════════════════

  ★口径 A→B 反退化语义变化（637号）：A 口径下 centerHolds 仅查前两段核心，完整判据在**两个正交
  维度**（方向 + 第三段贯穿）严格细化它。B 口径迁移后 `centerHolds`（三参全三段核心非空）已**吸收
  第三段贯穿**（`coreNonEmpty_iff_frontTwo_and_thirdSpans`）——第三段不贯穿在 B 下使全三段核心**空**，
  centerHolds 自身就拒绝。故 B 口径下 centerHolds 与完整判据的差异**只剩方向维度**：
  - 同向三段（无方向交替）：B centerHolds 成立（须三段全重叠）但完整判据拒绝——**唯一**严格细化维度。
  - 第三段不贯穿：B centerHolds **自身拒绝**（全三段核心空）——不再是 centerHolds/完整判据分叉的见证，
    而是"B 全三段核心非空判据吸收第三段贯穿"的见证（complete_b_absorbs_thirdSpan）。 -/

/--
  **★反退化见证段·同向全重叠三段（§6.1 无方向交替，L0，口径 B 唯一细化维证据）** —— 三段全向上
  且**三段共同重叠**（全三段核心非空，B centerHolds 成立）但无方向交替 ⟹ **不**是中枢（违 §6.1）。

  ★口径 B 重算（637号）：A 口径旧例 up(10→20)/up(18→25)/up(22→30) 在 B 下全三段核心
  ZD=max(10,18,22)=22 > ZG=min(20,25,30)=20 **空**——B centerHolds 自己就拒绝，不能作"centerHolds
  成立但完整判据拒绝"的见证。换**三段全重叠**同向例：
  s1=上(10→20)、s2=上(11→19)、s3=上(12→18)：
  - segHigh=20/19/18, segLow=10/11/12。
  - 全三段核心 ZG=min(20,19,18)=18, ZD=max(10,11,12)=12 ⟹ ZD=12 ≤ ZG=18，B centerHolds **成立**。
  - 但三段全向上（无方向交替）⟹ 单边上涨的三个次级别走势，**不是**中枢（是趋势）。
-/
def sameDirSeg1 : Segment := { direction := Direction.up, startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def sameDirSeg2 : Segment := { direction := Direction.up, startIndex := 1, endIndex := 2, startPrice := 11, endPrice := 19 }
def sameDirSeg3 : Segment := { direction := Direction.up, startIndex := 2, endIndex := 3, startPrice := 12, endPrice := 18 }

/-- 同向全重叠三段 B centerHolds 成立（全三段核心非空：ZD=12 ≤ ZG=18，口径 B）。 -/
theorem sameDir_centerHolds :
    computeZD sameDirSeg1 sameDirSeg2 sameDirSeg3 ≤ computeZG sameDirSeg1 sameDirSeg2 sameDirSeg3 := by
  unfold computeZD computeZG segLow segHigh tmax tmin sameDirSeg1 sameDirSeg2 sameDirSeg3
  decide

/--
  **★同向三段完整判据拒绝（L0，反退化核心）** —— `¬ CenterConfirmedComplete sameDirSeg1 sameDirSeg2 sameDirSeg3`。
  完整判据查方向交替（第一支）：要求 s1.dir ≠ s2.dir，但三段全 up ⟹ DirAlternates 不成立 ⟹ 完整
  判据正确拒绝（B centerHolds 因全三段重叠而成立，会误判为中枢）。这是 B 口径**唯一**严格细化维度
  （方向）的可观测证据：两者在同向全重叠三段上结论分叉。
-/
theorem sameDir_not_centerConfirmed :
    ¬ CenterConfirmedComplete sameDirSeg1 sameDirSeg2 sameDirSeg3 := by
  intro h
  -- h.1 是 DirAlternates（口径 B 第一支），要求 s1.dir ≠ s2.dir，但都是 up
  have halt : DirAlternates sameDirSeg1 sameDirSeg2 sameDirSeg3 := h.1
  unfold DirAlternates sameDirSeg1 sameDirSeg2 at halt
  exact halt.1 rfl

/--
  **★见证段·第三段不贯穿核心（§6.3 重叠不贯穿三段，L0，口径 B 吸收见证）** —— 方向交替成立但第三段
  脱离核心 ⟹ **B 全三段核心空**（centerHolds 自身拒绝），见证 B 吸收第三段贯穿（非 centerHolds/
  完整判据分叉）。

  s1=上(10→20)、s2=下(20→12)、s3=上(40→50)（方向交替 上-下-上）：
  - 前两段核心 [max(10,12), min(20,20)] = [12,20] 非空（A 口径前两段重叠）。
  - 但第三段 [40,50] 与前两段核心 [12,20] **无重叠**（segLow s3=40 > frontTwoZG=20）⟹ 不贯穿。
  - 口径 B 全三段核心：ZD=max(10,12,40)=40 > ZG=min(20,20,50)=20 ⟹ **核心空**（吸收第三段不贯穿）。
  - §6.1"被三个连续次级别走势所重叠的部分"不成立 ⟹ 不是中枢——B centerHolds 直接拒绝。
-/
def noSpanSeg1 : Segment := { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def noSpanSeg2 : Segment := { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 12 }
def noSpanSeg3 : Segment := { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 40, endPrice := 50 }

/-- 第三段不贯穿例：方向交替成立（上-下-上）。 -/
theorem noSpan_dirAlternates : DirAlternates noSpanSeg1 noSpanSeg2 noSpanSeg3 := by
  unfold DirAlternates noSpanSeg1 noSpanSeg2 noSpanSeg3; decide

/--
  **★口径 B 全三段核心空（第三段不贯穿被吸收，L0）** —— 第三段不贯穿例的全三段核心空
  `¬ (computeZD ≤ computeZG)`（ZD=40 > ZG=20）。这见证 B 口径 centerHolds（三参全三段）**自身**
  吸收第三段贯穿——A 口径需独立 ThirdSpansCore 支才能拒绝，B 口径全三段核心非空判据直接拒绝。
-/
theorem noSpan_b_core_empty :
    ¬ (computeZD noSpanSeg1 noSpanSeg2 noSpanSeg3 ≤ computeZG noSpanSeg1 noSpanSeg2 noSpanSeg3) := by
  unfold computeZD computeZG segLow segHigh tmax tmin noSpanSeg1 noSpanSeg2 noSpanSeg3
  decide

/--
  **★第三段不贯穿完整判据拒绝（L0）** —— `¬ CenterConfirmedComplete noSpanSeg1 noSpanSeg2 noSpanSeg3`。
  完整判据第二支查全三段核心非空（口径 B），但 ZD=40 > ZG=20 ⟹ 核心空 ⟹ 完整判据正确拒绝
  （第三段贯穿已吸收进核心非空，无需独立 ThirdSpansCore 支）。
-/
theorem noSpan_not_centerConfirmed :
    ¬ CenterConfirmedComplete noSpanSeg1 noSpanSeg2 noSpanSeg3 := by
  intro h
  -- h.2 是全三段核心非空（口径 B 第二支），但 ZD=40 > ZG=20
  exact noSpan_b_core_empty h.2

/--
  **★完整判据严格细化 centerHolds（L0，口径 B 唯一细化维=方向）** —— 存在三段使 B centerHolds 成立
  （`computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`，全三段核心非空）但完整判据拒绝（¬CenterConfirmedComplete）。
  口径 B 下见证是**同向全重叠三段**（方向维度缺失）——这是 B 口径唯一严格细化维度（第三段贯穿已被
  全三段核心非空吸收，不再是独立维度）。证明完整判据**严格细化** centerHolds（非改名）。
-/
theorem complete_strictly_refines_centerHolds :
    (∃ s1 s2 s3 : Segment,
      (computeZD s1 s2 s3 ≤ computeZG s1 s2 s3) ∧ ¬ CenterConfirmedComplete s1 s2 s3) := by
  -- 用同向全重叠三段见证（方向维度缺失——口径 B 唯一细化维）
  exact ⟨sameDirSeg1, sameDirSeg2, sameDirSeg3, sameDir_centerHolds, sameDir_not_centerConfirmed⟩

/--
  **★口径 B 全三段核心非空吸收第三段贯穿（L0，637号迁移可观测见证）** —— 第三段不贯穿三段
  `noSpanSeg*`：方向交替成立 ∧ 前两段核心非空（A 口径）∧ 第三段不贯穿前两段核心 ∧ **B 全三段核心空**。
  这见证 A→B 迁移把"第三段贯穿"从独立判据支吸收进全三段核心非空——A 需三支判其非中枢，B 两支
  （全三段核心空一支即拒）。`¬ ThirdSpansFrontTwoCore` 与 `¬ (computeZD ≤ computeZG)` 同时成立，
  正是 `coreNonEmpty_iff_frontTwo_and_thirdSpans` 等价的具体见证。
-/
theorem complete_b_absorbs_thirdSpan :
    DirAlternates noSpanSeg1 noSpanSeg2 noSpanSeg3 ∧
    (frontTwoZD noSpanSeg1 noSpanSeg2 ≤ frontTwoZG noSpanSeg1 noSpanSeg2) ∧
    ¬ ThirdSpansFrontTwoCore noSpanSeg1 noSpanSeg2 noSpanSeg3 ∧
    ¬ (computeZD noSpanSeg1 noSpanSeg2 noSpanSeg3 ≤ computeZG noSpanSeg1 noSpanSeg2 noSpanSeg3) := by
  refine ⟨noSpan_dirAlternates, ?_, ?_, noSpan_b_core_empty⟩
  · -- 前两段核心非空（A 口径）：max(10,12)=12 ≤ min(20,20)=20
    unfold frontTwoZD frontTwoZG segLow segHigh tmax tmin noSpanSeg1 noSpanSeg2; decide
  · -- 第三段不贯穿前两段核心：segLow s3=40 > frontTwoZG=20
    unfold ThirdSpansFrontTwoCore frontTwoZG frontTwoZD segLow segHigh tmin tmax noSpanSeg1 noSpanSeg2 noSpanSeg3
    decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 合法中枢正例（完整判据确认一个真中枢，下-上-下/上-下-上）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **合法中枢三段（§6.1 典型形态上-下-上，口径 B 全三段核心非空）** —— s1=上(10→20)、s2=下(20→12)、
  s3=上(12→22)（与 CenterConstruction.ovSeg* 同形）：方向交替 ∧ 全三段核心 [12,20] 非空
  （此 case 第三段 [12,22] 贯穿前两段核心 ⟹ 全三段核心 = 前两段核心 = [12,20]，A/B 重合）。
-/
def trueCenterSeg1 : Segment := { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def trueCenterSeg2 : Segment := { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 12 }
def trueCenterSeg3 : Segment := { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 12, endPrice := 22 }

/--
  **★合法中枢完整判据确认（L0，正例，口径 B 两支）** —— `CenterConfirmedComplete trueCenterSeg1 trueCenterSeg2 trueCenterSeg3`。
  两条全成立：方向交替（上-下-上）∧ 全三段核心非空（ZD=max(10,12,12)=12 ≤ ZG=min(20,20,22)=20，口径 B）。
  与同向/不贯穿反例对照：完整判据**接受**真中枢、**拒绝**伪中枢——判据有内容（非平凡）。
-/
theorem trueCenter_centerConfirmed :
    CenterConfirmedComplete trueCenterSeg1 trueCenterSeg2 trueCenterSeg3 := by
  refine ⟨?_, ?_⟩
  · -- 方向交替：上-下-上
    unfold DirAlternates trueCenterSeg1 trueCenterSeg2 trueCenterSeg3; decide
  · -- 全三段核心非空（口径 B）：ZD=max(10,12,12)=12 ≤ ZG=min(20,20,22)=20
    unfold computeZD computeZG segLow segHigh tmax tmin trueCenterSeg1 trueCenterSeg2 trueCenterSeg3; decide

/--
  **★完整判据可构造 CenterFull（L0，B′ 接入载体）** —— 被完整判据确认的三段可构造 CenterFull
  （复用 CenterConstruction.centerFromThree），其核心 = 真中枢核心。这把完整判据**接到** #116
  的 CenterFull 输出载体——完整判据确认的真中枢可承载外缘 GG/DD。`h.2` = 全三段核心非空（口径 B 第二支）。
-/
def centerFullOfConfirmed (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) : CenterFull :=
  centerFromThree s1 s2 s3 h.2

/-- ★合法中枢构造的 CenterFull 核心非空（L0，由完整判据第二支保证）。 -/
theorem trueCenter_full_core_valid :
    (centerFullOfConfirmed trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
      trueCenter_centerConfirmed).core.zd ≤
    (centerFullOfConfirmed trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
      trueCenter_centerConfirmed).core.zg :=
  (centerFullOfConfirmed trueCenterSeg1 trueCenterSeg2 trueCenterSeg3
    trueCenter_centerConfirmed).core.valid

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 完整判据投影必要条件（方向交替 / 全三段核心非空 / 第三段贯穿派生）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **★完整判据 ⟹ 方向交替（L0，§6.1 必要条件）** —— 真中枢必方向交替（首尾同向、与中间异向）。 -/
theorem centerConfirmed_implies_dirAlternates (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) : DirAlternates s1 s2 s3 := h.1

/-- **★完整判据 ⟹ 全三段核心非空（L0，§6.4 口径 B 必要条件）** —— 真中枢全三段核心 ZD ≤ ZG。 -/
theorem centerConfirmed_implies_coreNonEmpty (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) :
    computeZD s1 s2 s3 ≤ computeZG s1 s2 s3 := h.2

/--
  **★第三段贯穿前两段核心（A 降级为派生 lemma，637号）** —— 口径 B 全三段核心非空
  `computeZD s1 s2 s3 ≤ computeZG s1 s2 s3` ⟹ 第三段贯穿前两段核心 `ThirdSpansFrontTwoCore s1 s2 s3`。

  ★这是旧 A 口径独立判据支「第三段贯穿」的**降级形态**（codex 裁决"A 只能降级为证明用派生 lemma，
  换名/换位置/换用途"）：不再是 `CenterConfirmedComplete` 的合取支，而是从全三段核心非空**派生**
  出来的必要条件（由 `coreNonEmpty_iff_frontTwo_and_thirdSpans` 的左→右投影第二支）。
-/
theorem thirdSpansFrontTwoCore_of_coreNonEmpty (s1 s2 s3 : Segment)
    (h : computeZD s1 s2 s3 ≤ computeZG s1 s2 s3) : ThirdSpansFrontTwoCore s1 s2 s3 :=
  ((coreNonEmpty_iff_frontTwo_and_thirdSpans s1 s2 s3).mp h).2

/-- **★完整判据 ⟹ 第三段贯穿前两段核心（L0，§6.3 派生必要条件）** —— 真中枢第三段落入前两段核心重叠。 -/
theorem centerConfirmed_implies_thirdSpans (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) : ThirdSpansFrontTwoCore s1 s2 s3 :=
  thirdSpansFrontTwoCore_of_coreNonEmpty s1 s2 s3 h.2

/--
  **★完整判据强于 centerHolds（L0，单向蕴含，口径 B）** —— 完整判据确认 ⟹ centerHolds 成立
  （全三段核心非空是完整判据的真子条件）。反向不成立（反退化见证已证）——故完整判据**严格**强于 centerHolds。
-/
theorem centerConfirmed_implies_centerHolds (s1 s2 s3 : Segment)
    (h : CenterConfirmedComplete s1 s2 s3) :
    centerHolds s1 s2 s3 = true := by
  unfold centerHolds
  exact decide_eq_true h.2

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已升级（task #118 相对 #116 still-MISSING-B′；637号 A→B 主塔迁移）：
    - **完整判据 = 真中枢核心（口径 B 两支）**：`centerConfirmed_iff_trueCenterCore` 证完整中枢确认
      谓词与 §6.1 定义直译的真中枢核心谓词**逻辑等价**——升 #116 `centerHolds`（全三段核心非空）到
      完整判据（方向交替 ∧ 全三段核心非空）。第三段贯穿被全三段核心非空**吸收**（口径 B），不再是
      独立合取支（637号 no-patch：保留独立支 = 声明膨胀）。
    - **A→B 迁移核心恒等式**：`coreNonEmpty_iff_frontTwo_and_thirdSpans` 证全三段核心非空 ⟺
      [前两段核心非空 ∧ 第三段贯穿前两段核心]——交叉不等式 `d₃≤g₃` 由段不变量永真补齐。这是
      A 三支 ≡ B 两支接受集相等的机器证明（rust center.rs:25-32 同源 codex L0 核验重证）。
    - **完整判据严格细化 centerHolds（口径 B 唯一细化维=方向）**：`complete_strictly_refines_centerHolds`
      用**同向全重叠三段**（sameDirSeg*，无方向交替，B centerHolds 成立但完整判据拒绝）见证。口径 B
      下第三段贯穿已吸收进核心非空，**不再**是独立细化维度——故唯一维度=方向（A 口径有方向 + 第三段
      两维，B 口径合并为一维）。
    - **口径 B 吸收第三段贯穿见证**：`complete_b_absorbs_thirdSpan`（noSpanSeg*）见证第三段不贯穿 ⟺
      B 全三段核心空——A 需独立 ThirdSpansCore 支拒绝，B 全三段核心非空一支即拒。
    - **A 降级为派生 lemma**：`thirdSpansFrontTwoCore_of_coreNonEmpty`（B 全三段核心非空 ⟹ 第三段贯穿
      前两段核心）——旧 A 独立判据支「第三段贯穿」降级为从核心非空派生的必要条件（换名/换位置/换用途）。
    - **完整判据投影必要条件**：方向交替 / 全三段核心非空 / 第三段贯穿（派生）三条投影定理 +
      `centerConfirmed_implies_centerHolds`（完整判据严格强于 centerHolds 单向蕴含，口径 B）。
    - **接入 CenterFull 载体**：`centerFullOfConfirmed` 把完整判据确认的真中枢接到 #116 CenterFull
      输出载体（复用 centerFromThree，承载外缘 GG/DD；`h.2` = 全三段核心非空）。

  ★still-MISSING-B″（完整判据驱动的全自动递归识别，诚实开口）：
    本文件证**判据层**完整接入（完整中枢确认谓词 = 真中枢核心 + 严格细化几何必要条件）。但把
    完整判据 `CenterConfirmedComplete` 接入 CenterConstruction 的全自动递归 `centersOfComplete :
    List Segment → List CenterFull`——即滑窗时用方向交替 + 全三段核心非空判定每个三段窗口、且把相邻
    中枢对的发展态（CenterStates.classifyDevelopment：延伸/新生/扩展）串接成中枢序列——**未在本文件
    实装**。`centerConfirmed_implies_centerHolds`（完整判据 ⟹ centerHolds）说明：用完整判据替换
    centerHolds 后，成立支消费的段数不变（仍 3 段），故 CenterConstruction 的终止性骨架可复用，
    B″ 接入**可行**；但**不声称**已接入。把判据层接入冒充为全自动递归实装 = 声明膨胀（禁止）。

  ★边界条件（结论翻转条件）：
    - `CenterConfirmedComplete` 的合取**两支**（方向交替 ∧ 全三段核心非空，口径 B）缺一不确认。
      若某变体口径（§6.3 注：实战归纳变体）允许"只前两段重叠即识别"（提早识别三买，即回退口径 A），
      核心非空支退回前两段 `computeZD s1 s2`，第三段贯穿须重列为独立支——但那是 variant 口径，
      非原文 §6.1/第17课答疑严格公式——当前严格口径 B（全三段核心，原文）。
    - 方向交替用严格异向（s1.dir ≠ s2.dir）。若某口径允许"线段标准化后只看价位不看方向"
      （第78课标准化后线段当无内部结构部件），方向交替条件须重裁——但 §6.1 典型形态明确要求
      下-上-下/上-下-上，本文件取原文方向交替口径。
    - 全三段核心非空用闭区间（`computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`，单点核心 ZD=ZG 合法）；
      若要求严格 `ZD < ZG`（排除单点中枢），临界翻转。
    - 终止性可接入性（完整判据 ⟹ centerHolds ⟹ 成立支消费 3 段）是 B″ 接入的**前提条件**，
      非已实现保证。

  ★下游推论：
    - CenterConstruction.centersOf 当前用 centerHolds 几何必要条件（已口径 B 全三段）；B″ 接入完整
      判据后，识别的中枢序列将 = 真中枢序列（方向交替 + 全三段核心非空）。本文件提供判据层的正确性
      证明，是 B″ 接入的**前置依据**。
    - BspConstruction（买卖点）第三类"回抽不破 ZG/ZD"以真中枢的核心位置三态为基础——口径 B 收窄
      ZD/ZG（`ZD_B≥ZD_A`、`ZG_B≤ZG_A`）⟹ 下游消费 ZD/ZG 的位置/三买卖/边界数值随 B 收窄。
    - CenterStates.classifyDevelopment（发展三态）接相邻真中枢对——B′ 识别的真中枢是发展态串接的输入。

  ★谱系引用：升 CenterStates.lean § 6 still-MISSING-B（判据层 + GG/DD 字段）+ CenterConstruction.lean
    § 7 still-MISSING-B′（几何必要条件封装 → 完整判据）；**637号谱系：中枢核心区间口径 A→B 迁移**
    （一级权威第17课答疑严格公式 `(max(a2,b2,c2), min(a1,b1,c1))` 全三段重叠；误口径 A 前两段仅在
    第三段贯穿时与 B 重合）——A 口径独立判据支「第三段贯穿」被 B 全三段核心非空吸收，降级为派生 lemma。
    重锚 legacy Strict/Center.lean 中枢三态（谱系 256/264/608）到完整识别判据层。

  ★影响声明：
    - 修改 Origin.CenterComplete 模块（637号 A→B 主塔迁移），import ChanlunElements + CenterStates +
      CenterFull + CenterConstruction（全只读，不改任何上游模块）。
    - 不改 canonical 类型，无反向依赖。判据层口径 A→B：`CenterConfirmedComplete`/`TrueCenterCore`
      从三支降两支（第三段贯穿吸收）；`ThirdSpansCore` 删除，替换为派生谓词 `ThirdSpansFrontTwoCore`
      （查第三段相对前两段核心 [frontTwoZD,frontTwoZG]）；新增 `frontTwoZG/frontTwoZD`、
      `core_equiv_abstract`、`coreNonEmpty_iff_frontTwo_and_thirdSpans`、
      `thirdSpansFrontTwoCore_of_coreNonEmpty`、`noSpan_b_core_empty`、`complete_b_absorbs_thirdSpan`；
      `sameDirSeg*` witness 数值 B 重算（全重叠同向）；`complete_refines_via_thirdSpan` 删除
      （B 下第三段不再是独立细化维）。下游消费者（CenterConstruct/AutoAssign/Pipeline/rust）按新签名机械修复。
      待 Lead 全量真封（#93）。
-/

end NewChanlun.Origin
