/-
  Strict/CenterStrict.lean — 中枢三态 + 中枢位置三态 Layer2 升级（task #59, RTAS T-center 工位）

  上游标准：`tmp/chanlun-strict-classification-standard.md`（最严格完全分类标准）
            + `/tmp/codex_planruling_answer.md`（裁决1：∼ 选 B 结构等价）
            + `/tmp/codex_classification_answer.md` (b) 段（中枢位置三态点位侧双射 vs 走势段侧覆盖失败）
            + 谱系 608（中枢位置三态 = 点相对区间真划分）+ 007/008（中心定理一/二 settled）。

  本文件把两个 Layer1（构造子穷尽，已 machine-checked）升级为 Layer2（严格双射 Classifies）：
  - **分类A 中枢三态**（CenterTrichotomy，两中枢关系 up/down/expansion）→ Classifies on
    受限子类型 {pair // SameLevelNewCenterPair}（核心已分离，排除中心定理一延伸）。
  - **分类B 中枢位置三态**（CenterPosition，点 p 相对区间 below/within/above）→ Classifies on
    点位侧 Center × Int（干净双射）+ 走势段侧 ComparableToCenter 假设双射 + 覆盖失败反例刻画。

  ★∼ 选型裁定（codex 裁决1 "选 B 结构等价" 的精确化 / codex 异质审计 2026-06-25，内部 L0 推导）：
  在纯整数端点结构上，两个分类的任何让 SemanticQuotient.complete（I x = I y → x ∼ y）成立的
  等价 ∼ 都退化为 **标签核**（x ∼ y ↔ classify x = classify y）——SemanticQuotient 给出零独立
  数学内容（平凡商，标准 §A 警示 + kernel SemanticQuotient 注释）。"选 B 结构等价" 是
  SemanticQuotient 内部相对 A 标签相等 / C 操作语义的选项；当 SemanticQuotient 本身退化为平凡商，
  正确答案是 **不实例化 SemanticQuotient，直接实例化 Classifies（五项义务全证）**。点位侧/中枢三态
  的语义双射本质 = Classifies，不强行套 SemanticQuotient（no-patch-mentality：诚实命名）。
  边界条件：若端点改为 ℝ（实数），可定义基于测度的相对位置比值等价，SemanticQuotient 可能非平凡——
  当前端点为 Int，本裁定不翻转（标注于此，下游不得膨胀）。

  ★认识论等级（formalization-validity-domain 强制）：本文件全部 L0
  （继承第49课 / 中心定理二已结算定义 + 实数三歧 trichotomy，Lean machine-checked，omega）。
  lake build 通过 = 逻辑/双射正确（L0），不是实证有效域，不得膨胀。

  ★有效域诚实标注（codex 审计 + 任务 #44 / 谱系 256/264）：
  - 分类A 有效域 = SameLevelNewCenterPair 子类型（核心已分离），延伸（中心定理一，核心重叠）在
    有效域外，单独处理——声明全 Center×Center 上完全分类 = 有效域膨胀（禁止）。
  - 分类B 是**静态位置划分**，不蕴含操作序（之下→之中→之上的递归顺序未形式化）。穿越方向/
    穿越事件/操作时机须由 OnlineSystem/CausalOnlineSystem 状态转移定义，不在本分类有效域（256/264）。

  禁 sorry/admit/axiom。
-/

import Classification          -- task #57 T-kernel 共享内核（lake 模块名=Classification，命名空间=Strict）
import Formal.CenterTrichotomy  -- 分类A：两中枢关系三态（Phase1 脊柱）
import Claim9_CenterPosition    -- 分类B：中枢位置三态（Phase2 claim9，命名空间=Formal.CenterPosition）

namespace Strict.Center

open Formal.CenterTrichotomy
open Formal.CenterPosition

/-! ###########################################################################
    ## 分类B：中枢位置三态（点 p 相对中枢区间）— 点位侧干净双射 Classifies
    ###########################################################################

  codex#1 (b) 段：subject 是**点位/代表价**则 below/within/above 严格双射成立。
  对象 X = Center × Int（中枢 c + 价格点 p）。I = classify。P = 三态谓词。
  五项义务全证 ⟹ Classifies。语义 ∼ 本质是标签核 ⟹ 不实例化 SemanticQuotient（诚实命名）。
-/

/-- 位置谓词族（把 RelativePosition 标签映射到对 (c,p) 的谓词）。 -/
def PosPred : RelativePosition → (Center × Int) → Prop
  | RelativePosition.below  => fun x => IsBelow x.1 x.2
  | RelativePosition.within => fun x => IsWithin x.1 x.2
  | RelativePosition.above  => fun x => IsAbove x.1 x.2

/-- 不变量（点位侧）：从 (c,p) 取位置标签。 -/
def posI (x : Center × Int) : RelativePosition := Formal.CenterPosition.classify x.1 x.2

/--
  ★中枢位置三态点位侧严格双射（Classifies，L0）。

  codex#1 (b)：点位侧 below/within/above 严格双射成立。五项义务：
  - total：每个 (c,p) 至少落一类（position_predicates_total，omega）。
  - sound：posI 给出的标签确实满足谓词（classify_eq_* 的 mp 方向）。
  - complete：谓词蕴含标签相等（classify_eq_* 的 mpr 方向，最危险待证项✓）。
  - disjoint：一个点不能同时满足两个不同标签谓词（三 disjoint + core_valid）。
  - realized：每个标签有点见证（构造具体整数点）。
-/
theorem position_pointwise_classifies :
    Strict.Classifies (Center × Int) RelativePosition posI PosPred where
  total := by
    intro x
    rcases position_predicates_total x.1 x.2 with h | h | h
    · exact ⟨RelativePosition.below, h⟩
    · exact ⟨RelativePosition.within, h⟩
    · exact ⟨RelativePosition.above, h⟩
  sound := by
    intro x
    -- posI x = classify x.1 x.2；对三态分类，PosPred (classify ..) 即对应谓词，由 classify_eq_*
    unfold posI PosPred
    rcases position_trichotomy x.1 x.2 with h | h | h <;> rw [h]
    · exact (classify_eq_below x.1 x.2).mp h
    · exact (classify_eq_within x.1 x.2).mp h
    · exact (classify_eq_above x.1 x.2).mp h
  complete := by
    intro x c hP
    -- PosPred c x = 对应谓词；classify_eq_* mpr 给 classify = c
    cases c with
    | below  => exact (classify_eq_below x.1 x.2).mpr hP
    | within => exact (classify_eq_within x.1 x.2).mpr hP
    | above  => exact (classify_eq_above x.1 x.2).mpr hP
  disjoint := by
    intro x c₁ c₂ hP1 hP2
    -- 不同标签谓词互斥（below/within、below/above、within/above），同标签 rfl
    cases c₁ <;> cases c₂ <;>
      first
      | rfl
      | (exfalso; first
          | exact below_within_disjoint x.1 x.2 ⟨hP1, hP2⟩
          | exact below_within_disjoint x.1 x.2 ⟨hP2, hP1⟩
          | exact below_above_disjoint x.1 x.2 ⟨hP1, hP2⟩
          | exact below_above_disjoint x.1 x.2 ⟨hP2, hP1⟩
          | exact within_above_disjoint x.1 x.2 ⟨hP1, hP2⟩
          | exact within_above_disjoint x.1 x.2 ⟨hP2, hP1⟩)
  realized := by
    intro c
    cases c with
    | below  =>
      -- 中枢核心 [0,2]，点 p=-1 < 0 = zd ⟹ below
      refine ⟨(⟨-1, 0, 2, 3, by decide, by decide, by decide⟩, -1), ?_⟩
      show IsBelow _ _; unfold IsBelow; decide
    | within =>
      -- 点 p=1 ∈ [0,2] ⟹ within
      refine ⟨(⟨-1, 0, 2, 3, by decide, by decide, by decide⟩, 1), ?_⟩
      show IsWithin _ _; unfold IsWithin; decide
    | above  =>
      -- 点 p=5 > 2 = zg ⟹ above
      refine ⟨(⟨-1, 0, 2, 3, by decide, by decide, by decide⟩, 5), ?_⟩
      show IsAbove _ _; unfold IsAbove; decide

/-! ### ∼ 选型的机器见证：标签核 SemanticQuotient = 平凡商（不是独立语义双射）

  ★这把文件头"∼ 选型裁定"从注释声称升级为 **机器证明的判断**（no-patch-mentality：声明=实际）。
  codex 裁决1 "选 B 结构等价" 的精确化：在 Int 端点上，让 SemanticQuotient.complete
  （I x = I y → x ∼ y）成立的最细等价 ∼ 就是 **标签核**（posI x = posI y）。下面证明
  标签核确实诱导 SemanticQuotient——但 complete 在此是同义反复（`fun h => h`），
  即此 SemanticQuotient **零独立数学内容**（平凡商）。

  ⟹ 诚实结论：点位侧的语义双射本质 = `position_pointwise_classifies`（Classifies 五项义务），
  `position_label_kernel_quotient` 仅证明 "若强行用 SemanticQuotient 须取标签核且无内容"。
  这不是降级——是把"为何不实例化非平凡 SemanticQuotient"形式化为可质询的定理（标签核 complete = id）。
-/

/-- 点位侧标签核等价：(c,p) ∼ (c',p') ↔ 同位置标签。 -/
def PosLabelKernel (x y : Center × Int) : Prop := posI x = posI y

/--
  ★标签核诱导 SemanticQuotient（平凡商，L0）：标签核满足 equiv/invariant/complete/realized。
  关键：complete（posI x = posI y → PosLabelKernel x y）是 `fun h => h`——**同义反复**，
  这正是"平凡商无独立内容"的机器见证。下游不得把此 SemanticQuotient 当作非平凡语义双射引用。
-/
theorem position_label_kernel_quotient :
    Strict.SemanticQuotient (Center × Int) RelativePosition posI PosLabelKernel where
  equiv := ⟨fun _ => rfl, fun h => h.symm, fun h1 h2 => h1.trans h2⟩
  invariant := fun h => h               -- PosLabelKernel x y = (posI x = posI y) 直接给
  complete := fun h => h                -- ★同义反复 = 平凡商的机器见证（零独立内容）
  realized := by
    -- 每个标签可实现（复用 Classifies.realized 的见证）
    intro p
    obtain ⟨x, hx⟩ := position_pointwise_classifies.realized p
    exact ⟨x, position_pointwise_classifies.complete hx⟩

/-! ###########################################################################
    ## 分类B（续）：走势段侧 — ComparableToCenter 假设 + 覆盖失败反例
    ###########################################################################

  codex#1 (b)：subject 是**区间/走势段**（不是点）则三态未必覆盖——一个区间可以部分低于
  中枢、部分进入中枢（既非纯 below 也非纯 within）。须加 ComparableToCenter 假设：
  EntirelyBelow ∨ EntirelyInside ∨ EntirelyAbove。本节先刻画覆盖失败（无假设时反例存在），
  再在 ComparableToCenter 子类型上恢复三态双射。
-/

/-- 走势段（价格区间 [lo, hi]，lo ≤ hi）相对中枢的位置 subject。 -/
structure Segment where
  lo : Int
  hi : Int
  valid : lo ≤ hi
deriving Repr

/-- 区间整体在中枢核心之下：上沿 hi < zd（整段严格低于核心）。 -/
def EntirelyBelow (c : Center) (s : Segment) : Prop := s.hi < c.zd
/-- 区间整体在中枢核心之内：[lo,hi] ⊆ [zd,zg]。 -/
def EntirelyInside (c : Center) (s : Segment) : Prop := c.zd ≤ s.lo ∧ s.hi ≤ c.zg
/-- 区间整体在中枢核心之上：下沿 lo > zg（整段严格高于核心）。 -/
def EntirelyAbove (c : Center) (s : Segment) : Prop := c.zg < s.lo

/--
  ★可比较假设（codex#1 (b)）：区间相对中枢三态可分类的前提——整段落入三态之一。
  无此假设时区间可跨越中枢边界（部分 below + 部分 inside），三态不覆盖。
-/
def ComparableToCenter (c : Center) (s : Segment) : Prop :=
  EntirelyBelow c s ∨ EntirelyInside c s ∨ EntirelyAbove c s

/--
  ★覆盖失败反例（codex#1 (b) 的机器见证，L0）：存在中枢 c + 区间 s，使 s 既非整体之下、
  既非整体之内、也非整体之上——即 ¬ ComparableToCenter。区间跨越中枢下边界（lo<zd<hi 且 hi≤zg）。

  这证明走势段侧 **不能无条件三态分类**——必须显式携带 ComparableToCenter 假设。
  此为有效域诚实标注：三态对走势段的覆盖失败是真实的（非平凡），不是声明膨胀。
-/
theorem segment_coverage_can_fail :
    ∃ (c : Center) (s : Segment), ¬ ComparableToCenter c s := by
  -- 中枢核心 [0,4]；区间 [-2, 2] 跨越下边界 zd=0：lo=-2<0 但 hi=2∈[0,4]
  refine ⟨⟨-3, 0, 4, 6, by decide, by decide, by decide⟩, ⟨-2, 2, by decide⟩, ?_⟩
  unfold ComparableToCenter EntirelyBelow EntirelyInside EntirelyAbove
  -- hi=2 ≮ zd=0；¬(zd=0≤lo=-2)；zg=4 ≮ lo=-2 ⟹ 三支全假
  decide

/-- 走势段位置标签（区间版本，三态）。 -/
inductive SegPosition where
  | segBelow
  | segInside
  | segAbove
deriving DecidableEq, Repr

open SegPosition

/-- 走势段位置谓词族（在 ComparableToCenter 子类型上）。 -/
def SegPred (c : Center) : SegPosition → Segment → Prop
  | segBelow  => fun s => EntirelyBelow c s
  | segInside => fun s => EntirelyInside c s
  | segAbove  => fun s => EntirelyAbove c s

/--
  走势段位置判定（区间版 classify）：
  hi < zd → 之下 / lo > zg → 之上 / 否则 → 之内（须 ComparableToCenter 保证之内合法）。
-/
def segClassify (c : Center) (s : Segment) : SegPosition :=
  if s.hi < c.zd then segBelow
  else if c.zg < s.lo then segAbove
  else segInside

/--
  ★走势段三态双射（ComparableToCenter 子类型上的 Classifies，L0）。

  固定中枢 c。对象 = {s : Segment // ComparableToCenter c s}（携带可比较假设）。
  ★边界约定（与点位侧一致）：区间边界 hi=zd 不算之下（hi<zd 严格才之下），lo=zg 不算之上
  （zg<lo 严格才之上）——临界归之内（闭核心区间），与第49课"小于 ZD/大于 ZG"严格不等式一致。
-/
theorem segment_comparable_classifies (c : Center) :
    Strict.Classifies {s : Segment // ComparableToCenter c s} SegPosition
      (fun s => segClassify c s.val) (fun lbl s => SegPred c lbl s.val) where
  total := by
    intro s
    -- 由 ComparableToCenter（携带于子类型），整段落三态之一
    rcases s.property with h | h | h
    · exact ⟨segBelow, h⟩
    · exact ⟨segInside, h⟩
    · exact ⟨segAbove, h⟩
  sound := by
    intro s
    have hcmp := s.property
    have hzd := c.core_valid
    have hv := s.val.valid
    unfold ComparableToCenter EntirelyBelow EntirelyInside EntirelyAbove at hcmp
    unfold segClassify SegPred EntirelyBelow EntirelyInside EntirelyAbove
    by_cases h1 : s.val.hi < c.zd
    · -- segClassify=segBelow，谓词 EntirelyBelow = hi<zd = h1（simp 用 h1 直接闭目标）
      simp only [h1, if_true]
    · by_cases h2 : c.zg < s.val.lo
      · -- segClassify=segAbove，谓词 EntirelyAbove = zg<lo = h2（simp 用 h2 直接闭目标）
        simp only [h1, if_false, h2, if_true]
      · -- segClassify=segInside，谓词 EntirelyInside = zd≤lo ∧ hi≤zg
        --   ¬(hi<zd) ∧ ¬(zg<lo) + ComparableToCenter ⟹ 必是 EntirelyInside（omega 由 hcmp）
        simp only [h1, if_false, h2, if_false]
        omega
  complete := by
    intro s lbl hP
    have hzd := c.core_valid
    have hv := s.val.valid
    unfold segClassify SegPred EntirelyBelow EntirelyInside EntirelyAbove at *
    cases lbl with
    | segBelow  => simp only [hP, if_true]
    | segInside =>
      -- EntirelyInside ⟹ ¬(hi<zd) ∧ ¬(zg<lo) ⟹ segClassify=segInside
      obtain ⟨hl, hh⟩ := hP
      have h1 : ¬ s.val.hi < c.zd := by omega
      have h2 : ¬ c.zg < s.val.lo := by omega
      simp only [h1, if_false, h2, if_false]
    | segAbove  =>
      -- EntirelyAbove: zg<lo ⟹ ¬(hi<zd)（因 lo≤hi 且 zd<zg<lo≤hi）
      have h1 : ¬ s.val.hi < c.zd := by omega
      simp only [h1, if_false, hP, if_true]
  disjoint := by
    intro s lbl₁ lbl₂ hP1 hP2
    have hzd := c.core_valid
    have hv := s.val.valid
    unfold SegPred EntirelyBelow EntirelyInside EntirelyAbove at hP1 hP2
    cases lbl₁ <;> cases lbl₂ <;> first | rfl | (exfalso; omega)
  realized := by
    have hzd := c.core_valid  -- zd < zg，符号见证依赖此不变量
    intro lbl
    cases lbl with
    | segBelow  =>
      -- 区间 [zd-2, zd-1] 整体之下（hi=zd-1 < zd）。show 暴露求值后目标使 omega 看穿投影。
      refine ⟨⟨⟨c.zd - 2, c.zd - 1, by omega⟩, ?_⟩, ?_⟩
      · exact Or.inl (show c.zd - 1 < c.zd by omega)
      · show c.zd - 1 < c.zd; omega
    | segInside =>
      -- 区间 [zd, zd] ⊆ [zd, zg]（zd≤zd ∧ zd≤zg 由 zd<zg）
      refine ⟨⟨⟨c.zd, c.zd, by omega⟩, ?_⟩, ?_⟩
      · exact Or.inr (Or.inl (show c.zd ≤ c.zd ∧ c.zd ≤ c.zg by omega))
      · show c.zd ≤ c.zd ∧ c.zd ≤ c.zg; omega
    | segAbove  =>
      -- 区间 [zg+1, zg+2] 整体之上（lo=zg+1 > zg）
      refine ⟨⟨⟨c.zg + 1, c.zg + 2, by omega⟩, ?_⟩, ?_⟩
      · exact Or.inr (Or.inr (show c.zg < c.zg + 1 by omega))
      · show c.zg < c.zg + 1; omega

/-! ###########################################################################
    ## 分类A：中枢三态（两中枢关系 up/down/expansion）— 受限子类型双射
    ###########################################################################

  对象 = {pair : Center × Center // SameLevelNewCenterPair pair.1 pair.2}
  （核心已分离前提，排除中心定理一延伸/同枢）。
  ★有效域诚实标注（codex 审计）：延伸（中心定理一，核心重叠）在有效域外——
  全 Center×Center 上 total 不成立（trichotomy_predicates_total 须 SameLevelNewCenterPair）。
  诚实命名 = "核心已分离的两同级别新生中枢关系的完全分类"，非全 Center×Center。
-/

/-- 受限对象：核心已分离的两同级别新生中枢对。 -/
abbrev NewCenterPair := {pair : Center × Center // SameLevelNewCenterPair pair.1 pair.2}

/-- 中枢三态谓词族（在 NewCenterPair 上）。 -/
def RelPred : CenterRelation → NewCenterPair → Prop
  | CenterRelation.upContinuation   => fun p => IsUpContinuation p.val.1 p.val.2
  | CenterRelation.downContinuation => fun p => IsDownContinuation p.val.1 p.val.2
  | CenterRelation.levelExpansion   => fun p => IsLevelExpansion p.val.1 p.val.2

/-- 不变量（中枢三态）：从核心已分离对取关系标签。 -/
def relI (p : NewCenterPair) : CenterRelation := Formal.CenterTrichotomy.classify p.val.1 p.val.2

/-! ### complete 方向引理（谓词 → classify 标签）

  ★关键概念区分（不可用无条件 iff 掩盖）：
  `classify` 把"核心重叠的延伸"（中心定理一）也归到 `levelExpansion` 标签（因为延伸时
  外缘也重叠 ⟹ ¬up ∧ ¬down ⟹ classify=expansion）。但 `IsLevelExpansion` 谓词（中心定理二
  **原公式**）要求**核心分离**。⟹ `classify = levelExpansion ↔ IsLevelExpansion` 作为
  **无条件 iff 不成立**（mp 方向 classify=expansion → IsLevelExpansion 需核心分离前提，
  延伸是反例）。

  但 Classifies.complete 只需 **mpr 方向**（谓词 → classify 标签），这三个方向**无条件成立**：
  - IsUpContinuation → classify=up（classify_eq_up.mpr，无条件）
  - IsDownContinuation → classify=down（下证，无条件）
  - IsLevelExpansion → classify=expansion（用 disjoint：扩张 ⟹ ¬up ∧ ¬down ⟹ classify=expansion，无条件）

  这是诚实的：complete 只断言"谓词成立 ⟹ classify 给该标签"，不声称 classify 反过来忠实于谓词。
  延伸落在 NewCenterPair 有效域外（SameLevelNewCenterPair 排除核心重叠），不进入 total。
-/

/-- 下跌延续谓词 → classify=down（无条件，complete 用）。 -/
theorem down_imp_classify (prev next : Center) :
    IsDownContinuation prev next → Formal.CenterTrichotomy.classify prev next = CenterRelation.downContinuation := by
  intro hd
  unfold IsDownContinuation at hd
  have hp := prev.core_valid; have hpl := prev.outer_lo; have hph := prev.outer_hi
  have hn := next.core_valid; have hnl := next.outer_lo; have hnh := next.outer_hi
  unfold Formal.CenterTrichotomy.classify
  -- next.gg<prev.dd ⟹ ¬(next.dd>prev.gg)（由 dd≤zd<zg≤gg 不变量），故跳过 up 分支
  have h1 : ¬ next.dd > prev.gg := by omega
  simp only [h1, if_false, hd, if_true]

/--
  级别扩张谓词 → classify=expansion（无条件，complete 用）。
  用 disjoint：IsLevelExpansion ⟹ ¬IsUpContinuation ∧ ¬IsDownContinuation ⟹ classify=expansion。
-/
theorem expansion_imp_classify (prev next : Center) :
    IsLevelExpansion prev next → Formal.CenterTrichotomy.classify prev next = CenterRelation.levelExpansion := by
  intro hexp
  have hnotup : ¬ IsUpContinuation prev next := fun hup =>
    up_expansion_disjoint prev next ⟨hup, hexp⟩
  have hnotdown : ¬ IsDownContinuation prev next := fun hdown =>
    down_expansion_disjoint prev next ⟨hdown, hexp⟩
  unfold IsUpContinuation at hnotup
  unfold IsDownContinuation at hnotdown
  unfold Formal.CenterTrichotomy.classify
  simp only [hnotup, if_false, hnotdown, if_false]

/--
  ★中枢三态严格双射（Classifies on NewCenterPair 受限子类型，L0）。

  有效域 = 核心已分离的两同级别新生中枢对（排除中心定理一延伸/同枢——核心重叠）。
  五项义务：
  - total：trichotomy_predicates_total（**需 SameLevelNewCenterPair**，由子类型携带）。
  - sound：relI 给标签满足谓词（classify_eq_up.mp / 反推谓词）。
  - complete：谓词蕴含标签（classify_eq_up.mpr / down_imp_classify / expansion_imp_classify，无条件）。
  - disjoint：up/down/expansion 两两互斥（三 disjoint 定理）。
  - realized：三标签构造见证（含 levelExpansion：核心分离 + 外缘重叠）。

  ★诚实命名：这是"**核心已分离的两同级别新生中枢**关系的完全分类"，非全 Center×Center
  （延伸在有效域外，单独由中心定理一处理）。声明全域 = 有效域膨胀（codex 审计裁定4）。
-/
theorem center_relation_classifies :
    Strict.Classifies NewCenterPair CenterRelation relI RelPred where
  total := by
    intro p
    rcases trichotomy_predicates_total p.val.1 p.val.2 p.property with h | h | h
    · exact ⟨CenterRelation.upContinuation, h⟩
    · exact ⟨CenterRelation.downContinuation, h⟩
    · exact ⟨CenterRelation.levelExpansion, h⟩
  sound := by
    intro p
    unfold relI RelPred
    -- relI p = classify ..；对三态分类，RelPred (classify ..) 即对应谓词
    rcases center_trichotomy p.val.1 p.val.2 with h | h | h <;> rw [h]
    · exact (classify_eq_up p.val.1 p.val.2).mp h
    · -- classify=down → IsDownContinuation：由 down_imp_classify 的逆不直接，用谓词穷尽 + disjoint
      rcases trichotomy_predicates_total p.val.1 p.val.2 p.property with hu | hd | he
      · exact absurd (((classify_eq_up p.val.1 p.val.2).mpr hu).symm.trans h) (by decide)
      · exact hd
      · exact absurd ((expansion_imp_classify p.val.1 p.val.2 he).symm.trans h) (by decide)
    · -- classify=expansion → IsLevelExpansion：同上用穷尽 + disjoint
      rcases trichotomy_predicates_total p.val.1 p.val.2 p.property with hu | hd | he
      · exact absurd (((classify_eq_up p.val.1 p.val.2).mpr hu).symm.trans h) (by decide)
      · exact absurd ((down_imp_classify p.val.1 p.val.2 hd).symm.trans h) (by decide)
      · exact he
  complete := by
    intro p c hP
    cases c with
    | upContinuation   => exact (classify_eq_up p.val.1 p.val.2).mpr hP
    | downContinuation => exact down_imp_classify p.val.1 p.val.2 hP
    | levelExpansion   => exact expansion_imp_classify p.val.1 p.val.2 hP
  disjoint := by
    intro p c₁ c₂ hP1 hP2
    cases c₁ <;> cases c₂ <;>
      first
      | rfl
      | (exfalso; first
          | exact up_down_disjoint p.val.1 p.val.2 ⟨hP1, hP2⟩
          | exact up_down_disjoint p.val.1 p.val.2 ⟨hP2, hP1⟩
          | exact up_expansion_disjoint p.val.1 p.val.2 ⟨hP1, hP2⟩
          | exact up_expansion_disjoint p.val.1 p.val.2 ⟨hP2, hP1⟩
          | exact down_expansion_disjoint p.val.1 p.val.2 ⟨hP1, hP2⟩
          | exact down_expansion_disjoint p.val.1 p.val.2 ⟨hP2, hP1⟩)
  realized := by
    intro c
    cases c with
    | upContinuation   =>
      -- prev 核心[2,5]外缘[0,8]；next 核心[10,13]外缘[9,15]：next.dd=9>prev.gg=8 ⟹ up
      refine ⟨⟨(⟨0, 2, 5, 8, by decide, by decide, by decide⟩,
                ⟨9, 10, 13, 15, by decide, by decide, by decide⟩), ?_⟩, ?_⟩
      · -- SameLevelNewCenterPair: next.zd=10 > prev.zg=5
        show _ ∨ _; exact Or.inl (by decide)
      · show IsUpContinuation _ _; unfold IsUpContinuation; decide
    | downContinuation =>
      -- prev 核心[10,13]外缘[9,15]；next 核心[2,5]外缘[0,8]：next.gg=8<prev.dd=9 ⟹ down
      refine ⟨⟨(⟨9, 10, 13, 15, by decide, by decide, by decide⟩,
                ⟨0, 2, 5, 8, by decide, by decide, by decide⟩), ?_⟩, ?_⟩
      · -- SameLevelNewCenterPair: next.zg=5 < prev.zd=10
        show _ ∨ _; exact Or.inr (by decide)
      · show IsDownContinuation _ _; unfold IsDownContinuation; decide
    | levelExpansion   =>
      -- codex 见证：prev=(0,2,5,8), next=(4,6,9,11)：核心分离(next.zd=6>prev.zg=5)+外缘重叠(next.dd=4≤prev.gg=8)
      refine ⟨⟨(⟨0, 2, 5, 8, by decide, by decide, by decide⟩,
                ⟨4, 6, 9, 11, by decide, by decide, by decide⟩), ?_⟩, ?_⟩
      · -- SameLevelNewCenterPair: next.zd=6 > prev.zg=5
        show _ ∨ _; exact Or.inl (by decide)
      · -- IsLevelExpansion 支2: next.zd=6>prev.zg=5 ∧ next.dd=4≤prev.gg=8
        show IsLevelExpansion _ _; unfold IsLevelExpansion; exact Or.inr ⟨by decide, by decide⟩

end Strict.Center
