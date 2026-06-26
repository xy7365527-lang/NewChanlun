/-
  Strict/Trend.lean — 走势完全分类 Layer2（task #58, C1, 615 概念分离 Layer2）
  ★T-trend 工位（RTAS 严格完全分类蜂群）

  ════════════════════════════════════════════════════════════════════════
  范式背景（chanlun-strict-classification-standard.md 第一部分 A + codex#1/#2）
  ════════════════════════════════════════════════════════════════════════

  Layer1（Formal/TrendTrichotomy.lean，已有）：μF/inductive 给「语法生成无遗漏 +
  语法相等下构造子互斥」。`trend_trichotomy` / `trend_dichotomy_total` /
  `trend_consolidation_disjoint` 已证 `TrendKind` 三构造子穷尽互斥（L0）。

  codex#1 硬规则（严格性命名约束）：μF/inductive **不自动**给语义 ∼ 下的双射。
  凡只在 `x ∼ y ↔ I x = I y` 下成立的分类，必须显式命名为「按标签商分类」，
  不得冒充 `X/∼ ≅ P`。

  共享内核：import `Classification`（task #57 T-kernel 工位），复用
  `Strict.Classifies` / `Strict.SemanticQuotient`——不内联、不重复定义。

  ════════════════════════════════════════════════════════════════════════
  ★忠实性升级（codex 异质审计 session 019f0020，2026-06-25）
  ════════════════════════════════════════════════════════════════════════

  codex 审计裁定旧版本（`I` 用 `centers.length ≥ 2 ∧ 自由 dir 字段` 判趋势）**不合格**：
  - `dir` 是 WalkX 自由字段，违反 RecursiveConstruction R1#3「裁决由中枢计算，不自由携带」。
  - 旧反例 witness `[c0,c0]`（两个相同中枢）在真缠论判据下 `classifyMove [c0,c0] =
    higherCenterCandidate`（外缘不分离 = levelExpansion），**不是上涨趋势**——complete 失败
    反例对象非真趋势，注释「两个 upTrend 中枢数 2 vs 3」在缠论裁决层不成立。

  ⟹ 升级：`I` 由 **真缠论裁决** `RecursiveConstruction.classifyMove` 计算（不自由携带 dir）；
  `WalkX.kind` 由 `kindDerived` 约束钉死为 `classifyMove centers` 的桥接结果；反例换成
  **真上涨链**（外缘依次上移的中枢：c0=[0,3]/c1=[4,7]/c2=[8,11]，全链 upContinuation）。

  本模块 import `Formal.RecursiveConstruction`（下游文件，无循环——RecursiveConstruction
  import TrendTrichotomy，本文件 import RecursiveConstruction，方向一致）。

  ════════════════════════════════════════════════════════════════════════
  ★codex 编排者代理裁决（转向：精化非降级；/tmp/codex_planruling_answer.md §1.1/§2.1）
  ════════════════════════════════════════════════════════════════════════

  - **裁决1（∼ = B 结构等价）**：分类的数学对象是递归走势结构本身，不是标签。
    ⟹ ∼ := `SameChanStructure`（级别 + 中枢序列逐项相等；方向/种类已由中枢序列经
    真裁决 `classifyMove` 决定，不单列自由维度——盘整无方向 qushi 第31课）。

  - **裁决2（精化，不是降级！complete 失败不是目标，精化双射才是目标）**：
    `{趋势,盘整}` 只是完整走势类型树的 quotient/projection。若声称「完全分类」，必须把
    P **精化为完整走势类型树** `P_full = (级别 × 中枢序列)`（种类/方向/力度由中枢序列经
    真裁决派生），在结构等价 ∼ 下证 `X/∼ ≅ ValidP` **真双射**（complete **成立**）。
    `{趋势,盘整}` 作为粗投影保留（证良定义商映射 + 真多对一，QuotientByLabel）。

  - **标准第5部分对接（诚实边界，codex 复审 session 019f0025 第 (c) 点修正）**：
    `WalkX.kindDerived`（原名 `completed`，已诚实改名）= `classifyMove centers` 经
    `outcomeToKind` 桥接到 `kind` 成功（落 TrendKind 三态）。它**只保证 kind 由真裁决派生**，
    **不**保证「结构已封闭/已完成」。

    ★诚实裁定（撤回旧假声明）：旧注释声称「未完成走势不在定义域（无法构造 WalkX）」是
    **声明膨胀**——`RecursiveConstruction.CandidateMove.settled : Bool` 是**独立于 centers
    的维度**（`CandidateMove.state` 按 settled 路由 completed/pending），一个 `settled=false`
    的未完成候选其 centers 同样桥接成 trend up，故同一 centers 仍能构造 WalkX。即
    `kindDerived` 桥接成功 **不蕴含** settled=true。

    ⟹ **定义域排除的真正承载者是 `RecursiveConstruction.MoveState`**（completed ⊎ pending
    类型层分离，已全绿：`settled_gives_completed` / `pending_gives_pending` /
    `candidate_overreach_rejected`）——未完成走势在那里被类型层拒绝取得 `completed` 地位。
    本模块的 `WalkX` 是**结构层抽象**（中枢序列 + 其裁决标签），不重复承载 settled 维度
    （codex 裁决2 双射论域 `ValidP = (级别 × 中枢序列)` 不含 settled，故 WalkX 也不含——
    否则双射破坏：一个 ValidP 对应 settled=true/false 两个 WalkX）。`higherCenterCandidate`
    （0 中枢 / 级别扩张 / 中枢非依次同向）映 `none` ⟹ 这类**裁决退化**的 centers 不在
    本结构层定义域（其分类归 OpenTailSystem 状态层），但这是「裁决落三态」的约束，
    **不是**「已完成 settled」的约束。双射论域 `ValidP` = 裁决落三态的 P_full。

  ⟹ **三层定理（无声明膨胀）**：
      Layer1（`Formal.TrendTrichotomy.*`）：`TrendKind` 标签穷尽互斥，保留。
      Layer2-(a) `trend_classifies`：标签层真分区（`Classifies`，真裁决判据，见证用真链）= StructurePartitionOnly。
      Layer2-(b) `trend_full_semantic_quotient`：精化真双射 `WalkX/∼ ≅ ValidP`（`SemanticQuotient` 全证，complete **成立**）= TrueCompleteClassification。
      Layer2-(c) `proj_well_defined` + `trendKind_is_projection` + `proj_collapses_structure`：`{趋势,盘整}` 是 `P_full` 的良定义粗投影（真多对一）= QuotientByLabel。

  认识论等级：全部 L0（定义内蕴；精化双射是结构同构，非经验断言）。
  formalization-validity-domain：Lean build 通过 = 逻辑正确（L0），不膨胀为实证有效域。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy
import Formal.RecursiveConstruction
import Classification  -- task #57 T-kernel 共享内核（lake 模块名=Classification，命名空间=Strict）

namespace Strict.Trend

open Formal.TrendTrichotomy (TrendKind Direction)
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.RecursiveConstruction (classifyMove)

/-! ════════════════════════════════════════════════════════════════════════
    § 0. 真缠论裁决 → TrendKind 桥接（裁决落三态走势的 {趋势,盘整}；settled 由 MoveState 承载）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★`MoveOutcome → Option TrendKind` 桥接（不自由携带，由真裁决派生）。

  - `trend up` ↦ `upTrend`；`trend down` ↦ `downTrend`；`consolidation` ↦ `consolidation`。
  - `higherCenterCandidate` ↦ **none**：0 中枢 / 级别扩张 / 中枢非依次同向 = 本级未终结、
    交父级的**状态**，不是已完成走势的 {趋势,盘整} **结果**（标准第5部分）。
-/
def outcomeToKind : MoveOutcome → Option TrendKind
  | MoveOutcome.trend Direction.up => some TrendKind.upTrend
  | MoveOutcome.trend Direction.down => some TrendKind.downTrend
  | MoveOutcome.consolidation => some TrendKind.consolidation
  | MoveOutcome.higherCenterCandidate => none

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 走势的细结构 `WalkX`（codex#1：递归走势结构 + 真裁决派生 kind）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  走势对象 `WalkX`（缠论：走势由中枢序列 + 级别决定，zoushi.md 第17课；
  codex 审计升级：方向由中枢序列经真裁决 `classifyMove` 决定，不自由携带）。

  - `level`      ：走势所在级别（多级别共振——同标签可来自不同级别，体现力度区分）。
  - `centers`    ：构成该走势的中枢序列（走势 = 中枢序列的运动；方向由其依次同向性决定）。
  - `kind`       ：该走势的 {趋势,盘整} 类型——**不自由携带**，由 `kindDerived` 约束钉死。
  - `kindDerived`：**真裁决派生约束**（codex R1#3）——`classifyMove centers` 经 `outcomeToKind`
    成功落在 TrendKind 三态（= `some kind`）。它保证 kind 由真缠论裁决决定（不自由携带 dir）。

  ★诚实命名（codex 复审 session 019f0025 第 (c) 点）：字段名 `kindDerived`（非旧 `completed`）。
  它**只**断言「kind 由 centers 的真裁决桥接派生」，**不**断言「结构已封闭/已完成」——
  「已完成（settled）」是 `RecursiveConstruction.CandidateMove` 的**独立维度**，由
  `RecursiveConstruction.MoveState`（completed ⊎ pending）类型层承载，**不**由本结构层重复。
  `higherCenterCandidate`（裁决退化）映 `none` ⟹ 裁决不落三态的 centers 不在本定义域
  （其分类归 OpenTailSystem 状态层）——这是「裁决落三态」约束，非「settled」约束。

  这是比 `TrendKind` **严格更细** 的结构（codex#1「递归走势结构」），且 `kind` 忠实于
  RecursiveConstruction 内核的真裁决（R1#3：裁决由中枢计算）。
-/
structure WalkX where
  level : Nat
  centers : List Center
  kind : TrendKind
  kindDerived : outcomeToKind (classifyMove centers) = some kind

/--
  走势类型投影 `I : WalkX → TrendKind`（zoushi.md 第17课走势三分法）。
  `I x := x.kind`——由 `kindDerived` 约束，`kind` 由真缠论裁决 `classifyMove centers` 唯一决定。
-/
def I (x : WalkX) : TrendKind := x.kind

/-! ════════════════════════════════════════════════════════════════════════
    § 1.1 真链见证中枢（codex 审计建议：外缘依次上/下移，全链同向延续）
    ════════════════════════════════════════════════════════════════════════ -/

/-- 真链中枢 c0：核心 [1,2]，外缘 [0,3]。 -/
def c0 : Center := ⟨0, 1, 2, 3, by decide, by decide, by decide⟩
/-- 真链中枢 c1：核心 [5,6]，外缘 [4,7]（c1.dd=4 > c0.gg=3 ⟹ 上涨延续）。 -/
def c1 : Center := ⟨4, 5, 6, 7, by decide, by decide, by decide⟩
/-- 真链中枢 c2：核心 [9,10]，外缘 [8,11]（c2.dd=8 > c1.gg=7 ⟹ 上涨延续）。 -/
def c2 : Center := ⟨8, 9, 10, 11, by decide, by decide, by decide⟩

/-- 2 枢真上涨趋势（全链 upContinuation）：classifyMove [c0,c1] = trend up。 -/
theorem chain2_up : classifyMove [c0, c1] = MoveOutcome.trend Direction.up := by decide
/-- 3 枢真上涨趋势：classifyMove [c0,c1,c2] = trend up。 -/
theorem chain3_up : classifyMove [c0, c1, c2] = MoveOutcome.trend Direction.up := by decide
/-- 单枢真盘整：classifyMove [c0] = consolidation。 -/
theorem chain1_cons : classifyMove [c0] = MoveOutcome.consolidation := by decide
/-- 2 枢真下跌趋势（外缘依次下移）：classifyMove [c2,c0] = trend down。 -/
theorem chain2_down : classifyMove [c2, c0] = MoveOutcome.trend Direction.down := by decide

/-- ★walkUp2：2 枢真上涨趋势（kind=upTrend 由真裁决 chain2_up 钉死）。 -/
def walkUp2 : WalkX where
  level := 0
  centers := [c0, c1]
  kind := TrendKind.upTrend
  kindDerived := by decide

/-- ★walkUp3：3 枢真上涨趋势（与 walkUp2 同标签 upTrend，结构不同）。 -/
def walkUp3 : WalkX where
  level := 0
  centers := [c0, c1, c2]
  kind := TrendKind.upTrend
  kindDerived := by decide

/-- ★walkCons：单枢真盘整（kind=consolidation）。 -/
def walkCons : WalkX where
  level := 0
  centers := [c0]
  kind := TrendKind.consolidation
  kindDerived := by decide

/-- ★walkDown：2 枢真下跌趋势（kind=downTrend）。 -/
def walkDown : WalkX where
  level := 0
  centers := [c2, c0]
  kind := TrendKind.downTrend
  kindDerived := by decide

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 标签层真互斥穷尽分区（StructurePartitionOnly，codex#1 标准 §A 分情况分类）

    谓词族 `IsKind : TrendKind → WalkX → Prop` 用**真裁决判据**（classifyMove 桥接）刻画
    每个标签，证 `Strict.Classifies WalkX TrendKind I IsKind`——total/sound/complete/
    disjoint/realized 五件套。这**不是**重言（谓词不是 `I x = c`，而是「真裁决桥接 = c」），
    故是标签层的**真分区**（穷尽 + 互斥），见证用真链（非伪造中枢）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  走势类型的**真裁决判据**谓词族（独立于 `I`，非重言）：
  某标签 `c` 刻画走势 `x` ⟺ `x` 的中枢序列经真缠论裁决 `classifyMove` 桥接到 `some c`。
  这把"标签满足"绑定到 RecursiveConstruction 内核的真判据（全链依次同向），非自由 dir。
-/
def IsKind (c : TrendKind) (x : WalkX) : Prop :=
  outcomeToKind (classifyMove x.centers) = some c

/-- `I` 与真裁决判据一致（sound 的核心引理）：`I x` 满足其真裁决判据。 -/
theorem I_sound (x : WalkX) : IsKind (I x) x := x.kindDerived

/--
  ★Layer2-标签层 真互斥穷尽分区（StructurePartitionOnly，L0）。

  `Strict.Classifies WalkX TrendKind I IsKind` 五件套全证：
  - total   ：每个走势至少落入一类（`⟨I x, I_sound x⟩`）。
  - sound   ：`I_sound`——`I x` 的标签满足其真裁决判据。
  - complete：满足某类真裁决判据 ⟹ `I x` 等于该类（裁决桥接是函数，故标签由 centers 唯一决定）。
  - disjoint：互斥——不可同时满足两类真裁决判据（`outcomeToKind ∘ classifyMove` 是函数，
    `some c₁ = some c₂ ⟹ c₁ = c₂`）。
  - realized：每类有 **真链** 走势见证（walkUp2/walkDown/walkCons，非伪造中枢）。

  ★诚实命名：这是**标签层的真分区**（穷尽+互斥），证明 `TrendKind` 是 `WalkX` 的良定义
  粗分类。但它**不**蕴含语义结构层的双射——见 §3 complete 失败。
-/
theorem trend_classifies : Strict.Classifies WalkX TrendKind I IsKind where
  total x := ⟨I x, I_sound x⟩
  sound := I_sound
  complete := by
    intro x c hc
    -- IsKind c x : outcomeToKind (classifyMove x.centers) = some c
    -- I_sound x  : IsKind (I x) x = outcomeToKind (classifyMove x.centers) = some (I x)
    have h : outcomeToKind (classifyMove x.centers) = some (I x) := I_sound x
    unfold IsKind at hc
    rw [hc] at h
    -- h : some c = some (I x)
    exact ((Option.some.injEq _ _).mp h).symm
  disjoint := by
    intro x c₁ c₂ h1 h2
    unfold IsKind at h1 h2
    rw [h1] at h2
    exact (Option.some.injEq _ _).mp h2
  realized := by
    intro c
    cases c with
    | consolidation => exact ⟨walkCons, walkCons.kindDerived⟩
    | upTrend        => exact ⟨walkUp2, walkUp2.kindDerived⟩
    | downTrend      => exact ⟨walkDown, walkDown.kindDerived⟩

/-! ════════════════════════════════════════════════════════════════════════
    § 3. 忠实结构等价 ∼（codex 裁决1 选 B：同级别 + 同中枢序列；方向由中枢决定）

    `SameChanStructure x y := level + centers 逐项相等`。方向/种类已由 centers 经真裁决
    `classifyMove` 决定（盘整无方向），不单列。这是 §4 精化双射的等价关系（invariant 即
    `trend_invariant`，complete 在精化靶子 `ValidP` 上成立——裁决2 主目标）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★语义结构等价 ∼（codex 裁决1，选 B 结构等价）。

  两走势「同缠结构」⟺ 级别、中枢序列**逐项**相等。方向由中枢序列经真裁决决定，
  故 centers 相等 ⟹ 方向相等，不需单列 dir 维度。
  这是 codex#1 的 `WalkEqSemantic`——分类的数学对象是递归走势结构本身，不是标签投影。
-/
def SameChanStructure (x y : WalkX) : Prop :=
  x.level = y.level ∧ x.centers = y.centers

/-- ∼ 是等价关系（逐分量相等的合取，自反/对称/传递显然）。 -/
theorem sameChanStructure_equiv : Equivalence SameChanStructure where
  refl _ := ⟨rfl, rfl⟩
  symm := fun ⟨h1, h2⟩ => ⟨h1.symm, h2.symm⟩
  trans := fun ⟨h1, h2⟩ ⟨g1, g2⟩ => ⟨h1.trans g1, h2.trans g2⟩

/--
  ★invariant 成立（同结构必同标签，L0）：`SameChanStructure x y ⟹ I x = I y`。

  结构等价要求 centers 相等，而 `kind`（=I）由 `classifyMove centers` 经 `kindDerived` 钉死，
  故 centers 相等 ⟹ 同裁决 ⟹ 同标签。这是双射三件套里**唯一**对语义 ∼ 成立的一件。
-/
theorem trend_invariant {x y : WalkX} (h : SameChanStructure x y) : I x = I y := by
  obtain ⟨_, hcen⟩ := h
  -- kind 由 classifyMove centers 决定（kindDerived），centers 相等 ⟹ kind 相等。
  have hx : outcomeToKind (classifyMove x.centers) = some (I x) := x.kindDerived
  have hy : outcomeToKind (classifyMove y.centers) = some (I y) := y.kindDerived
  rw [hcen] at hx
  -- hx : outcomeToKind (classifyMove y.centers) = some (I x)
  -- hy : outcomeToKind (classifyMove y.centers) = some (I y)
  rw [hx] at hy
  -- hy : some (I x) = some (I y)
  exact (Option.some.injEq _ _).mp hy

/-! ════════════════════════════════════════════════════════════════════════
    § 4. Layer2-(b) 精化真双射（codex 裁决2 主目标：X/∼ ≅ ValidP，TrueCompleteClassification）

    ★裁决2 转向（精化，非降级）：complete 失败不是目标，精化双射才是目标。
    精化靶子 `P_full := Nat × List Center`（级别 × 中枢序列）= 完整走势类型树的载体
    （种类/方向/力度全由中枢序列经真裁决 classifyMove 派生，已含于 centers）。
    `I_full x := (x.level, x.centers)` 完整捕获 ∼ ⟹ complete 成立 ⟹ 真双射。
    论域 = 合法 P_full（centers 经 classifyMove 落 TrendKind 三态）——非法 centers 无走势实现。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  完整走势类型树载体 `P_full := Nat × List Center`（级别 × 中枢序列）。

  ★缠论忠实性：走势的完整结构 = 级别 + 中枢序列（方向、种类、力度全由中枢序列的依次
  同向性 + 数量经真裁决 `classifyMove` 派生）。故 (level, centers) 是完整走势类型树的
  无冗余载体——比携带自由 dir 的字段更忠实（dir 是派生量，不应自由）。
-/
abbrev P_full := Nat × List Center

/-- 完整结构投影 `I_full : WalkX → P_full`（裁决2 精化不变量，完整捕获结构）。 -/
def I_full (x : WalkX) : P_full := (x.level, x.centers)

/--
  合法 P_full 谓词（双射的精确论域）：中枢序列经真裁决 `classifyMove` 落在 TrendKind
  三态（= **裁决落三态**的走势，**非** 关于 settled 的"已完成"——见 WalkX.kindDerived
  诚实命名）。裁决退化 centers（裁决 = higherCenterCandidate = 0 中枢 / 级别扩张 /
  中枢非依次同向）无 TrendKind 实现——排除空标签（这是「裁决落三态」约束，非「settled」约束）。
-/
def ValidPFull (p : P_full) : Prop :=
  ∃ k : TrendKind, outcomeToKind (classifyMove p.2) = some k

/-- `I_full x` 总是合法 P_full（由 `x.kindDerived` 保证 centers 落三态）。 -/
theorem I_full_valid (x : WalkX) : ValidPFull (I_full x) :=
  ⟨x.kind, x.kindDerived⟩

/-- 合法靶子 `ValidP := {p : P_full // ValidPFull p}`（codex 裁决2 双射的精确论域）。 -/
abbrev ValidP := {p : P_full // ValidPFull p}

/-- 精化不变量到合法靶子 `I_valid : WalkX → ValidP`。 -/
def I_valid (x : WalkX) : ValidP := ⟨I_full x, I_full_valid x⟩

/--
  ★Layer2-(b) 精化真双射（codex 裁决2 主目标，TrueCompleteClassification，L0）。

  `Strict.SemanticQuotient WalkX ValidP I_valid SameChanStructure` 四件套全证：
  - equiv     ：∼ 是等价关系（`sameChanStructure_equiv`）。
  - invariant ：`x ∼ y ⟹ I_valid x = I_valid y`（∼ 即 level+centers 相等 ⟹ 子类型相等）。
  - complete  ：`I_valid x = I_valid y ⟹ x ∼ y`（子类型相等 ⟹ 值相等 ⟹ 分量相等 ⟹ ∼）。
    **这是裁决2 的核心**——精化后 complete **成立**（与旧 `TrendKind` 太粗 complete 失败相反），
    因 `P_full` 捕获完整结构（级别 + 中枢序列）。
  - realized  ：`∀ p : ValidP, ∃ x, I_valid x = p`——由合法性 `ValidPFull p` 取出 kind 见证，
    重建 WalkX（`kindDerived` 由合法性证明给出）。

  ⟹ `WalkX/∼ ≅ ValidP`（完整走势类型树）真双射。这是 codex 裁决2 的「精化双射才是目标」。

  ★诚实认识论标注（formalization-validity-domain，防声明膨胀）：本双射中 ∼（=level+centers
  相等）恰好是 `I_full` 的核，故 complete 在此是**定义性展开**（精化到 ∼ 粒度的必然结果），
  非独立的经验/结构 complete 验证。这是裁决2 主动选择的代价——为达 TrueCompleteClassification，
  P 须精化到与 ∼ 对应的粒度，此时双射定义性成立。**真正的数学内容不在此双射本身**，而在：
  (1) §2 `trend_classifies` 标签层真分区（真裁决判据 `classifyMove`，非重言）；
  (2) §5 `proj_collapses_structure` 粗投影的真多对一（`{趋势,盘整}` 相对 `P_full` 的真信息损失）。
  realized 论域限制为 `ValidP`（合法 = **裁决落三态** P_full，**非** settled「已完成」——
  见 §3.2 ValidPFull 与 WalkX.kindDerived 诚实命名）——裁决退化 centers（higherCenterCandidate）
  无 TrendKind 实现，故全 P_full 上 realized 不成立，论域限制为「裁决落三态」是诚实标注，非声明膨胀。
-/
theorem trend_full_semantic_quotient :
    Strict.SemanticQuotient WalkX ValidP I_valid SameChanStructure where
  equiv := sameChanStructure_equiv
  invariant := by
    intro x y h
    obtain ⟨hl, hc⟩ := h
    apply Subtype.ext
    show I_full x = I_full y
    unfold I_full; rw [hl, hc]
  complete := by
    intro x y h
    have hval : I_full x = I_full y := congrArg Subtype.val h
    unfold I_full at hval
    exact ⟨congrArg Prod.fst hval, congrArg Prod.snd hval⟩
  realized := by
    intro p
    obtain ⟨⟨lvl, cs⟩, k, hk⟩ := p
    refine ⟨⟨lvl, cs, k, hk⟩, ?_⟩
    -- I_valid ⟨lvl, cs, k, hk⟩ = ⟨(lvl, cs), _⟩ = p（子类型，值相等 + 证明无关）
    apply Subtype.ext
    rfl

/-! ════════════════════════════════════════════════════════════════════════
    § 5. Layer2-(c) 粗投影（codex 裁决2：{趋势,盘整} 是 P_full 的良定义商映射）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  粗投影 `proj : P_full → Option TrendKind`（完整走势类型树 → 三分标签）。
  由真裁决 `classifyMove` 经 `outcomeToKind` 派生（合法 P_full 上得 `some`）。
  ★这是**多对一**映射（不同 centers 的趋势同映 some upTrend）——真投影，非恒等。
-/
def proj (p : P_full) : Option TrendKind := outcomeToKind (classifyMove p.2)

/-- 旧三分标签不变量 `I = proj ∘ I_full` 的兑现（{趋势,盘整} 是 P_full 的投影）。 -/
theorem trendKind_is_projection (x : WalkX) : proj (I_full x) = some (I x) :=
  x.kindDerived

/--
  ★粗投影良定义（QuotientByLabel，codex 裁决2：proj 是 ∼ 不变的商映射，L0）。

  ∼-等价的走势经 `I`（=proj∘I_full）映到同一 `TrendKind`：`SameChanStructure x y ⟹ I x = I y`。
  这证明 `{趋势,盘整}` 是 `WalkX/∼`（≅ `ValidP`，§4 真双射）上的**良定义商映射**——
  粗投影从精化双射诱导下来，诚实命名为 QuotientByLabel（标签层粗分类）。
  （= 原 `trend_invariant`，此处重述为「粗投影良定义」的语义。）
-/
theorem proj_well_defined {x y : WalkX} (h : SameChanStructure x y) : I x = I y :=
  trend_invariant h

/--
  ★粗投影是真多对一（非单射，L0 见证）：完整走势类型树的两个不同合法 `P_full`
  （walkUp2 的 [c0,c1] vs walkUp3 的 [c0,c1,c2]）经 `proj` 同映 `some upTrend`，但 P_full 不等。

  这刻画"为何 `{趋势,盘整}` 是真粗投影"——投影坍缩了中枢数（=力度/级别结构）信息。
  这**不是** complete 失败（§4 精化双射 complete 已成立）——是粗投影 `proj` 本身的多对一，
  即 `{趋势,盘整}` 相对完整走势类型树 `P_full` 的信息损失（裁决2：标签是 projection）。
  与原文 blog/017-第17课.md:78,82-84 缠师亲证一致：力度/级别不同的两上涨趋势，同标签异结构。
-/
theorem proj_collapses_structure :
    proj (I_full walkUp2) = proj (I_full walkUp3)
    ∧ I_full walkUp2 ≠ I_full walkUp3 := by
  refine ⟨by decide, ?_⟩
  intro h
  -- I_full walkUp2 = (0, [c0,c1])；I_full walkUp3 = (0, [c0,c1,c2])，中枢序列长度 2 ≠ 3。
  have : ([c0, c1] : List Center).length = ([c0, c1, c2] : List Center).length :=
    congrArg (fun p => p.2.length) h
  simp at this

/-! ════════════════════════════════════════════════════════════════════════
    § 6. Layer1 桥接（保留：粗投影与既有标签穷尽一致）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★粗投影 `I` 落入 `TrendKind` 三构造子之一（Layer1 标签穷尽，桥接 `trend_trichotomy`）。
  精化双射的粗投影仍满足旧的标签穷尽——Layer1 在 Layer2 之上保持一致。
-/
theorem I_trichotomy (x : WalkX) :
    I x = TrendKind.consolidation ∨ I x = TrendKind.upTrend ∨ I x = TrendKind.downTrend :=
  Formal.TrendTrichotomy.trend_trichotomy (I x)

end Strict.Trend
