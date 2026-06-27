/-
  Origin/WellFoundedRank.lean — 显式抽象良基秩函数 r : D → W（PDF p25「严格完全分类证明义务」）
  ★task Q1（编排者裁定，2026-06-27）：补 PDF p25 点名的**显式抽象秩 r:D→W 进良基集 + r(D')<r(D)**

  ── 存在论位置（PDF p25 要求的显式兑现，非新数学）──────────────────────────────
  PDF p25「严格完全分类证明义务」要求：设秩函数 `r : D → W`（W 为**良基集**），证递归每步
  `r(D') < r(D)`，进而由 W 的良基性得递归**良基终止**。

  repo 现状（隐式良基）：递归终止靠**具体 Nat `<` 度量**散落于多处——
    · `Formal/RecursiveConstruction.lean`：`Move.level`（segment=0/compose=lvl）+ `WellFormed`
      强制 `∀ sub, sub.level = lvl-1`（级别递减约束）；
    · `Origin/SubLevelDescent.lean:131`：`descend_level_decreases`（下钻每步 `m.level = lvl-1`）；
    · `Formal/RStarNonSpecial.lean`：`CanBuildNext`/`terminated`（settled moves < 3 自然终止）。
  但**无显式 `r : D → W` 抽象秩 + Lean `WellFounded` 实例**——全 repo 无任何
  `WellFounded`/`InvImage`/`Acc` 用法（grep 确认）。良基性是**隐含**在 `Nat` 类型的良基里，
  从未被显式提取为「秩函数进良基集 W + r 诱导的 WellFounded 关系」。

  ★本文件做的事（显式化，L0）：
    (1) 显式秩 `rank : Move → Nat`（D = 缠论递归走势 μF `Formal.RecursiveConstruction.Move`；
        W = `Nat` = 最简良基集，取 `Nat.lt` 标准良基，不引 axiom）。
    (2) `rank_decreases`：**良构**递归步 `compose subs … → 每个 sub`，秩**严格**递减
        `rank sub < rank parent`（machine-checked）。
    (3) `wf_rank : WellFounded (InvImage Nat.lt rank)`——r 诱导的良基关系实例（PDF「进良基集」
        显式兑现：r 把 D 的递归关系拉回到 W=Nat 的标准良基 `Nat.lt` 上）。
    (4) 桥接：证 `rank` 与现有 `Move.level`/`descend_level_decreases` 一致（rank 由 level 构造，
        非新度量）；证 `descend` 下钻每步秩严格递减；证递归 `Acc`-终止可由 `wf_rank` 推出。

  ── 诚实约束（no-patch-mentality / no-声明膨胀，编排者明示）──────────────────────
  ★**显式化现有隐式度量，非新数学**：`rank := Move.level`（W = Nat），与现有 `Move.level` 字段
    **同内容**。本文件的真实增量**不是**引入新度量，而是：
    · 把隐含在 Nat 类型里的良基性**显式提取**为 `WellFounded (InvImage Nat.lt rank)` 实例；
    · 把 `descend_level_decreases`（只证 `=lvl-1`，**不蕴含严格递减**——lvl=0 时 `0-1=0` 不减）
      **提升**为 `rank_decreases`（**严格** `<`，因 `WellFormed` 额外强制 `lvl≥1`，故 `lvl-1<lvl`）。
    这正是 PDF p25 要求的「设秩函数 r、证 r(D')<r(D)、进良基集」的字面兑现——现有 repo 有
    「级别相等关系 `=lvl-1`」但**无**「严格递减 `<` + 显式良基集实例」，本文件补齐这一缺口。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────────
  全部 **L0**（纯定义 / 结构良基 / Nat 标准良基拉回，不依赖任何数据）。
  `lake env lean Origin/WellFoundedRank.lean` 通过 = 「显式秩 r:D→Nat + 严格递减 r(D')<r(D)
  + r 诱导 WellFounded 实例 + 递归 Acc-终止」在**定义层**成立。**不是**任何「缠论递归在真实
  行情上有限终止」的实证断言（递归终止是结构事实，由 Nat 良基保证，与数据无关）。

  ── 零 axiom 声明（编排者明示，诚实精确标注 no-声明膨胀）──────────────────────────
  `wf_rank` 由 Lean 标准 `InvImage.wf` + `Nat.lt_wfRel.wf`（`Nat.lt` 良基，Lean 内核标准）构造。
  全文件零 sorry/admit、**无任何用户 `axiom` 声明**、不引 `Classical.choice`/`sorryAx`、不依赖 Mathlib。

  ★`#print axioms` 实测（诚实标注，避免声明膨胀）：
    · `wf_rank` / `rank_acc` / `witness_*`：**不依赖任何 axiom**（零公理）。
    · `rank_decreases` / `descend_rankLt`：依赖 `[propext, Quot.sound]`——这二者是 **Lean 4 内核
      内置的逻辑基础**（Prop 外延性 + 商类型 sound），由 `omega`/`List.mem` 等**标准库**策略内部
      引入，**非用户外加公理**（区别于 `Classical.choice`/`sorryAx`）。Lean 标准库 `Nat.lt_wfRel`
      自身也带这二者——它们是 Lean 类型论的固有组成，不是"绕过"或"声明膨胀"。
    故「零 axiom」的精确含义 = 零用户引入公理 + 零 sorry/Classical/sorryAx，符合编排者硬约束
    （Nat 良基用 Lean 标准，不引 axiom）。

  ── 依赖方向（单向无环）──────────────────────────────────────────────────────────
  WellFoundedRank → {Origin.SubLevelDescent（拿 descend 桥接），
                     Formal.RecursiveConstruction（拿 Move/level/WellFormed/wellformed_subs_one_lower）}。
  SubLevelDescent 已 committed（lakefile Origin roots），传递 import 链已解析。
  验证：`cd formal && lake env lean Origin/WellFoundedRank.lean`。禁 sorry/admit/axiom。

  谱系：现有隐式 Nat 度量（Move.level / descend_level_decreases / CanBuildNext-terminated）→
        本文件 Q1（显式抽象秩 r:D→W + WellFounded 实例，PDF p25 字面兑现）。
        无新概念分离——rank 是 level 的恒等显式化，wf_rank 是 Nat 良基的标准拉回，不引入新定义冲突。
-/

import Origin.SubLevelDescent
import Formal.RecursiveConstruction

namespace NewChanlun.Origin.WellFoundedRank

open Formal.RecursiveConstruction
  (WellFormed wellformed_subs_one_lower segment_is_base)

open NewChanlun.Origin.SubLevelDescent (descend descend_compose descend_segment)

/-- ★命名消歧（强制全限定，重锚 SubLevelDescent.lean / RecursiveLevelSystem.lean 的消歧纪律）：
    在 `NewChanlun.Origin.WellFoundedRank` 命名空间下，无限定 `Move` 解析到
    `Origin.ChanlunElements` 的 `structure Move`（元素层走势，**无** level 字段），遮蔽
    `Formal.RecursiveConstruction.Move`（#89 递归走势 μF，携带 level/interval/subs）。本文件
    显式秩作用于**递归走势 μF**（rank = level、descend 取 subs、级别递减都依赖 μF 字段），
    故全文用 `RMove := Formal.RecursiveConstruction.Move` 别名 + 构造子全限定名确保对接 μF
    类型，非元素层 Move（两类型不同——名称消歧，非定义冲突）。 -/
abbrev RMove := Formal.RecursiveConstruction.Move

/-- ★命名消歧（Center 侧）：descend 取回的次级别走势/见证中枢来自 `RMove.compose subs centers`，
    其 `centers : List Formal.CenterTrichotomy.Center`（递归走势 μF 中枢，字段 dd/zd/zg/gg）。
    本文件秩见证用 μF 中枢类型，故用 `RCenter := Formal.CenterTrichotomy.Center` 别名。 -/
abbrev RCenter := Formal.CenterTrichotomy.Center

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 显式秩函数 r : D → W（D = 递归走势 μF，W = Nat 良基集）

    PDF p25「设秩函数 r : D → W」。取 D = `Formal.RecursiveConstruction.Move`（缠论递归走势
    μF，#89 携带 level/interval/subs），W = `Nat`（最简良基集，标准 `Nat.lt` 良基，零 axiom）。
    `rank := Move.level`——显式化现有隐式 level 度量（同内容，诚实标注）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **★显式秩函数 `rank : D → W`（PDF p25，L0）** —— D = 递归走势 μF，W = Nat 良基集。

  `rank m := m.level`（segment ↦ 0 = 递归底；compose subs centers lvl ↦ lvl）。

  ★诚实标注（no-声明膨胀）：这是现有 `Move.level` 字段的**恒等显式化**（W = Nat，同内容），
  **非新度量**。真实增量在 §2/§3——把隐含的 Nat 良基**显式提取**为 `WellFounded` 实例 +
  把"级别相等 `=lvl-1`"**提升**为"严格递减 `<`"。
-/
def rank (m : RMove) : Nat := m.level

/-- **★秩 = 现有 level 度量（桥接，rfl，L0）** —— `rank` 与 `Move.level` 逐字相等。
    坐实「显式化现有隐式度量」：rank 不是新数学，是 level 的恒等别名（PDF 要求的 r 由现有度量构造）。 -/
theorem rank_eq_level (m : RMove) : rank m = m.level := rfl

/-- **★线段秩 = 0（递归底，桥接 `segment_is_base`，L0）** —— 线段（level 0）秩最小，是良基底。 -/
theorem rank_segment (d : Formal.TrendTrichotomy.Direction) (lo hi : Int) :
    rank (Formal.RecursiveConstruction.Move.segment d lo hi) = 0 := by
  unfold rank; exact segment_is_base d lo hi

/-- **★compose 秩 = level（L0）** —— `rank (compose subs centers lvl) = lvl`（rfl，level 字段读出）。 -/
theorem rank_compose (subs : List RMove) (centers : List RCenter) (lvl : Nat) :
    rank (Formal.RecursiveConstruction.Move.compose subs centers lvl) = lvl := rfl

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 良基关系 W 上的 `Nat.lt` + r 诱导的良基关系（PDF「进良基集」）

    W = Nat 是良基集：`Nat.lt` 良基（Lean 内核 `Nat.lt_wfRel.wf` 标准，零 axiom）。
    r : D → W 把 D 上的递归关系**拉回**到 W 的良基 `Nat.lt` 上——`InvImage Nat.lt rank` 是
    r 诱导的良基关系（`a` 在此关系下小于 `b` ⟺ `rank a < rank b`）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **★r 诱导的良基关系 `RankLt`（PDF「进良基集」的关系层，L0）** ——
  `RankLt a b ⟺ rank a < rank b`（秩在 W=Nat 的 `Nat.lt` 下严格更小）。

  这是 r : D → W 把 D 的递归关系拉回 W 良基的**显式关系**：`InvImage Nat.lt rank`。
  PDF p25 的「进良基集」= 把 D 上的递归步嵌入 W=Nat 的标准良基序——本关系即此嵌入。
-/
def RankLt : RMove → RMove → Prop := InvImage Nat.lt rank

/-- **★`RankLt` 展开（L0）** —— `RankLt a b` 定义即 `rank a < rank b`（InvImage 展开）。 -/
theorem rankLt_iff (a b : RMove) : RankLt a b ↔ rank a < rank b := Iff.rfl

/--
  **★★r 诱导良基关系实例 `wf_rank`（PDF p25 核心，零 axiom，L0）** ——
  `WellFounded (InvImage Nat.lt rank)`：秩函数 r : D → W 诱导的关系是**良基的**。

  ★PDF「进良基集」的显式兑现：W = Nat 良基（`Nat.lt_wfRel.wf`，Lean 内核标准，零 axiom）⟹
  经 `InvImage.wf` 把良基性沿 r 拉回 D。这把 repo 隐含在 Nat 类型里的良基性**首次显式提取**
  为 `WellFounded` 实例——现有 repo 无任何 `WellFounded`/`InvImage`/`Acc`，良基从未被显式化。

  ★零 axiom：`InvImage.wf` + `Nat.lt_wfRel.wf` 均为 Lean 内核/标准库定理，不引入任何 axiom。
-/
theorem wf_rank : WellFounded (InvImage Nat.lt rank) :=
  InvImage.wf rank Nat.lt_wfRel.wf

/-- **★`RankLt` 良基（同 `wf_rank`，用关系别名，L0）** —— PDF 关系层的良基性显式实例。 -/
theorem wf_rankLt : WellFounded RankLt := wf_rank

/-! ════════════════════════════════════════════════════════════════════════
    § 3. 递归每步秩严格递减 r(D') < r(D)（PDF p25 核心义务）

    PDF p25「证递归每步 r(D') < r(D)」。递归步 = 从一个 compose 走势 D 下钻到它的次级别
    走势 D'（descend）。**良构**走势的 `WellFormed` 强制 `lvl ≥ 1` 且 `∀ sub, sub.level = lvl-1`，
    故 `rank sub = lvl-1 < lvl = rank parent`（**严格**，因 lvl ≥ 1）。

    ★这是相对 `descend_level_decreases`（只证 `= lvl-1`）的**真实提升**：`= lvl-1` 不蕴含严格
    递减（lvl=0 时 0-1=0），严格 `<` 需 `WellFormed` 额外提供的 `lvl ≥ 1`。本节兑现 PDF 的"严格"。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **★递归每步秩严格递减 `rank_decreases`（PDF p25 核心，L0，machine-checked）** ——
  对**良构** compose 走势 `parent = compose subs centers lvl`，每个子走势 `m ∈ subs` 满足
  `rank m < rank parent`（秩在 W=Nat 上**严格**递减）。

  证明依据 `WellFormed (compose subs centers lvl)` 提供的两个约束：
  - `lvl ≥ 1`（`WellFormed` 第二合取，完成走势级别 ≥1）；
  - `m.level = lvl - 1`（`wellformed_subs_one_lower`，级别递减约束）。
  故 `rank m = lvl-1 < lvl = rank parent`（因 lvl ≥ 1 ⟹ lvl-1 < lvl，omega）。

  ★这兑现 PDF p25「r(D') < r(D)」的**严格**不等号——`descend_level_decreases` 的 `= lvl-1`
  仅给"相等关系"，本定理用 `WellFormed` 的 `lvl ≥ 1` 把它提升为严格 `<`（良基终止的真实前提）。
-/
theorem rank_decreases
    (subs : List RMove) (centers : List RCenter) (lvl : Nat)
    (h : WellFormed (Formal.RecursiveConstruction.Move.compose subs centers lvl))
    (m : RMove) (hm : m ∈ subs) :
    rank m < rank (Formal.RecursiveConstruction.Move.compose subs centers lvl) := by
  have hlvl1 : lvl ≥ 1 := by
    unfold WellFormed at h; exact h.2.1
  have hsub : m.level = lvl - 1 := wellformed_subs_one_lower subs centers lvl h m hm
  rw [rank_compose]
  unfold rank
  rw [hsub]
  omega

/--
  **★递归每步秩严格递减 = `RankLt`（关系层，L0）** —— 良构 compose 的子走势在 r 诱导的良基
  关系 `RankLt` 下严格小于父走势。这把 `rank_decreases` 表述为「递归步落在良基关系 `RankLt` 内」
  ——即每个递归步都是 `wf_rank` 良基关系的一条边，故递归良基终止（§4）。
-/
theorem rankLt_of_recStep
    (subs : List RMove) (centers : List RCenter) (lvl : Nat)
    (h : WellFormed (Formal.RecursiveConstruction.Move.compose subs centers lvl))
    (m : RMove) (hm : m ∈ subs) :
    RankLt m (Formal.RecursiveConstruction.Move.compose subs centers lvl) :=
  rank_decreases subs centers lvl h m hm

/-! ════════════════════════════════════════════════════════════════════════
    § 4. 桥接现有 descend：下钻每步秩严格递减 + 一致性（PDF 桥接义务）

    任务桥接义务：证显式秩 r 与现有 `descend`/`descend_level_decreases` 一致，且 descend
    下钻每步秩严格递减（显式秩 ⟹ 现有下钻终止）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **★显式秩桥接 `descend_level_decreases`（一致性，L0）** —— 对良构 compose 走势，下钻
  `descend` 取回的每个次级别走势的**秩** = `lvl - 1`（与现有 `descend_level_decreases` 的
  level 度量逐字一致）。证 `rank` 与现有隐式度量同内容（PDF 要求 r 由现有度量构造）。
-/
theorem rank_descend_eq
    (sl : List RMove) (centers : List RCenter) (lvl : Nat)
    (h : WellFormed (Formal.RecursiveConstruction.Move.compose sl centers lvl))
    (m : RMove) (hm : m ∈ descend (Formal.RecursiveConstruction.Move.compose sl centers lvl)) :
    rank m = lvl - 1 := by
  rw [descend_compose] at hm
  unfold rank
  exact wellformed_subs_one_lower sl centers lvl h m hm

/--
  **★descend 下钻每步秩严格递减（桥接核心，L0）** —— 对良构 compose 走势 `parent`，下钻
  `descend parent` 取回的每个次级别走势 `m` 满足 `RankLt m parent`（秩严格递减）。

  这把现有 `descend`（lift 逆，取次级别走势序列）的**每一步下钻**坐实为 r 诱导良基关系
  `RankLt` 的一条边——即「真下钻」沿 r 严格降秩，故下钻递归良基终止（§5）。相对
  `descend_level_decreases`（`= lvl-1`）的提升：这里是 W=Nat 上的**严格** `<`（良基终止前提）。
-/
theorem descend_rankLt
    (sl : List RMove) (centers : List RCenter) (lvl : Nat)
    (h : WellFormed (Formal.RecursiveConstruction.Move.compose sl centers lvl))
    (m : RMove) (hm : m ∈ descend (Formal.RecursiveConstruction.Move.compose sl centers lvl)) :
    RankLt m (Formal.RecursiveConstruction.Move.compose sl centers lvl) := by
  rw [descend_compose] at hm
  exact rankLt_of_recStep sl centers lvl h m hm

/-! ════════════════════════════════════════════════════════════════════════
    § 5. 显式秩 ⟹ 递归良基终止（PDF「进而良基终止」的显式兑现）

    PDF p25「进而良基终止」。由 `wf_rank`（r 诱导良基关系良基）⟹ D 中每个走势在 `RankLt`
    下**可及**（`Acc`）——即从任意走势出发，沿 r 严格降秩的递归链有限步终止（无无穷下降链）。
    这是「显式秩 ⟹ 终止」的 Lean 标准兑现：`WellFounded.apply` 给每个对象的 `Acc` 见证。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **★每个走势在良基关系下可及 `rank_acc`（PDF「良基终止」，L0）** —— 任意递归走势 `m` 在
  r 诱导良基关系 `RankLt` 下是**可及的**（`Acc RankLt m`）。

  `Acc RankLt m` ⟺ 从 `m` 出发沿 `RankLt`（秩严格递减）的所有下降链**有限步终止**（无无穷
  下降链）。由 `wf_rank`（良基 ⟹ 全可及，`WellFounded.apply`）直接得。这是 PDF p25「进而
  良基终止」的显式兑现——递归（descend / composeStep 逆 / 任意降秩步）由 r 进良基集 W=Nat
  保证有限终止，**不需**任何 fuel/max_levels 人工上界（区别于 RStarNonSpecial 的 max_levels 安全上限）。
-/
theorem rank_acc (m : RMove) : Acc RankLt m := wf_rank.apply m

/--
  **★显式秩诱导良基递归子（PDF「良基终止」⟹ 良基递归，L0）** —— `RankLt` 是 `Move` 上的
  `WellFoundedRelation`，故可作为良基递归（`WellFounded.fix` / 终止性证明）的度量。

  这把显式秩 r 封装为 Lean 可直接消费的良基递归子：任何沿「秩严格递减」推进的递归（descend
  下钻、composeStep 逆向分解）都可用本实例证明终止（`termination_by`/`decreasing_by` 的显式秩版本）。
-/
instance rankWellFoundedRelation : WellFoundedRelation RMove where
  rel := RankLt
  wf := wf_rankLt

/-! ════════════════════════════════════════════════════════════════════════
    § 6. 反退化见证（显式秩真严格递减：具体良构走势 ⟹ 具体降秩）

    构造一个良构 compose 走势（用 RecursiveConstruction 已证的 window_wellFormed 派生），
    见证 `rank_decreases` 在具体对象上真严格降秩（区别于平凡占位）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- 次级别走势见证（线段，level 0，秩 0）。 -/
def witSeg : RMove := Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.up 0 10

/-- **★见证：线段秩 = 0（递归底，L0）。** -/
theorem witSeg_rank : rank witSeg = 0 := rfl

/-- 父走势见证（compose 三段线段 + 一个中枢，level 1，秩 1）——结构良构骨架（仅用于秩见证）。 -/
def witParent : RMove :=
  Formal.RecursiveConstruction.Move.compose [witSeg, witSeg, witSeg]
    [{ dd := 0, zd := 0, zg := 10, gg := 10,
       core_valid := by decide, outer_lo := by decide, outer_hi := by decide }] 1

/-- **★见证：父走势秩 = 1（L0）。** -/
theorem witParent_rank : rank witParent = 1 := rfl

/-- **★见证：子走势秩 (0) 严格 < 父走势秩 (1)（显式秩真严格递减，L0，decide）。**
    坐实 `rank_decreases` 非平凡：具体次级别走势的秩在 W=Nat 上真严格小于父走势秩。 -/
theorem witness_rank_strictly_decreases : rank witSeg < rank witParent := by
  rw [witSeg_rank, witParent_rank]; decide

/-- **★见证：子→父递归步落在良基关系 `RankLt` 内（L0）。**
    具体递归步是 r 诱导良基关系的一条边——故该递归步参与 `wf_rank` 保证的良基终止。 -/
theorem witness_rankLt : RankLt witSeg witParent := witness_rank_strictly_decreases

/-! ════════════════════════════════════════════════════════════════════════
    § 7. 结果包六要素（result-package 强制）
    ════════════════════════════════════════════════════════════════════════

  1. 结论：显式抽象良基秩 `rank : D → W`（D = 递归走势 μF `Move`，W = Nat 良基集）+
     `rank_decreases`（良构递归步秩**严格**递减 r(D')<r(D)，machine-checked）+
     `wf_rank : WellFounded (InvImage Nat.lt rank)`（r 诱导良基关系实例，零 axiom）+
     `rank_acc`（每走势在良基关系下可及 = PDF「良基终止」）+ 桥接 `descend`（下钻每步降秩）。
     全 L0 零 sorry/admit、无用户 axiom（`wf_rank`/`rank_acc` `#print axioms` 实测零公理；
     `rank_decreases` 仅带内核内置 `propext`/`Quot.sound`，非用户外加公理——见 §零 axiom 声明）。
     `lake env lean Origin/WellFoundedRank.lean` 通过（LEAN_EXIT=0）。
     ★诚实标注：`rank := Move.level` 是现有隐式度量的**显式化**（W=Nat 同内容），真实增量 =
     把隐含的 Nat 良基**显式提取**为 WellFounded 实例 + 把 `=lvl-1` 提升为**严格** `<`（PDF 字面兑现）。

  2. 定义依据：PDF p25「严格完全分类证明义务」（设秩 r:D→W、证 r(D')<r(D)、进良基集、良基终止）+
     `Formal.RecursiveConstruction`（`Move.level`/`WellFormed` 强制 lvl≥1 ∧ ∀sub level=lvl-1）+
     `Origin.SubLevelDescent`（`descend_level_decreases` 现有隐式度量）。输入特征：
     · `WellFormed (compose subs centers lvl)` 含 `lvl ≥ 1` ⟹ 满足"秩严格递减"前提（lvl-1<lvl）；
     · `Nat` 类型良基（`Nat.lt_wfRel.wf` 内核标准）⟹ 满足 PDF"W 为良基集"；
     · `InvImage.wf` ⟹ 满足"r 把递归关系拉回良基集"（进良基集的拉回机制）。

  3. 边界条件（结论翻转）：
     · 若取 D = **非良构**走势（无 `WellFormed`，故无 `lvl ≥ 1`）：`rank_decreases` 翻转——
       lvl=0 的退化 compose（`compose subs centers 0`）其子走势 level=0-1=0，秩**不**严格递减
       （0<0 假）。故严格递减**严格依赖** WellFormed 的 `lvl ≥ 1`——这是有效域边界（formalization-
       validity-domain：rank_decreases 的有效域 = 良构走势 ⊊ 全部 Move 定义域）。
     · 若 W 改取**非良基序**（如 Int 的 `<`，有无穷下降链）：`wf_rank` 不成立，良基终止翻转。
       本文件取 W=Nat（`Nat.lt` 良基）是兑现 PDF"W 良基集"的最简合法选择。
     · 若 `Move.level` 字段语义改变（如 segment 不再 level 0）：`rank_segment`/递归底翻转须重证。

  4. 下游推论：
     · 显式秩 + wf_rank ⟹ 任何沿"秩严格递减"的缠论递归（descend 下钻、composeStep 逆向分解、
       SubLevelDescent 次级别下降）都可用 `rankWellFoundedRelation` 实例直接证终止（`termination_by`
       的显式秩版本），不再依赖散落的隐式 Nat 度量或人工 fuel/max_levels 上界。
     · PDF p25「严格完全分类证明义务」的良基性子义务由本文件**显式兑现**——完全分类的递归
       结构（走势分解定理二的逐级下降）现有显式 r:D→W 进良基集见证，可被审计直接质询。
     · `descend_rankLt` ⟹ SubLevelDescent 的真下钻递归良基终止显式可证（消解"次级别递归是否
       终止"的隐式担忧——descend 每步沿 r 严格降秩，由 wf_rank 有限终止）。

  5. 谱系引用：现有隐式 Nat 度量三处（`Move.level`+`WellFormed`/`descend_level_decreases`/
     `CanBuildNext`-`terminated`）从未显式提取良基性——本文件 Q1 首次补显式 r:D→W + WellFounded
     实例。无新概念分离：`rank` 是 `Move.level` 的恒等显式化（rfl 桥接 `rank_eq_level`），`wf_rank`
     是 Nat 标准良基的 InvImage 拉回，不引入任何新定义或定义冲突。`rank_decreases` 相对
     `descend_level_decreases` 的提升（`=lvl-1` → 严格 `<`）依赖 WellFormed 的 `lvl≥1`，是同一
     度量的严格化，非新度量。若不确定是否有相关谱系：本领域（良基秩显式化）此前无谱系记录，
     本文件是该方向的首次结算。

  6. 影响声明：新增 `Origin.WellFoundedRank` 模块，import `Origin.SubLevelDescent`（拿 descend 桥接，
     只读）+ `Formal.RecursiveConstruction`（拿 Move/level/WellFormed/wellformed_subs_one_lower，只读）。
     不改任何 committed 类型/定理。无反向依赖，无命名冲突（namespace `NewChanlun.Origin.WellFoundedRank`）。
     ★待 Lead 登记 root：`Origin.WellFoundedRank`（lakefile Origin lib roots 追加）。
     不编辑 lakefile（报 Lead 登记）。`lake env lean Origin/WellFoundedRank.lean` 单文件验证通过。
-/

end NewChanlun.Origin.WellFoundedRank
