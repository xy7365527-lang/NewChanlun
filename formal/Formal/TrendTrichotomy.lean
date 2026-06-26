/-
  走势三分完全分类·Layer1（005 走势分解定理一 + qushi/zoushi + 603 §三）

  缠论定理（已结算）：
  - 走势分解定理一（zoushi.md:26 / qushi.md:28，第17课 / blog/017-第17课.md:14）：
    "任何级别的所有走势，都能分解成趋势与盘整两类，而趋势又分为上涨与下跌两类。"
  - 走势类型三种穷尽：上涨 ∪ 下跌 ∪ 盘整 = 全集。

  范式重铸（603）：完全性 **不是** "走势特征向量轨道穷尽"（外延式 → 无穷回归），
  **是** 走势类型 sum type 的构造子穷尽（内涵式）。走势类型作为代数数据类型，
  其构造子集 {Consolidation, UpTrend, DownTrend} 由缠论三分法定理钉死。

  形式化对象：`TrendKind` 是一个三构造子 inductive（与 Rust `recursive_t::types::TrendKind`
  逐字对应：Consolidation / UpTrend / DownTrend）。

  ════════════════════════════════════════════════════════════════════════
  ★两层定理（615 概念分离，codex#1 硬规则）——本文件只承载 Layer1
  ════════════════════════════════════════════════════════════════════════

  本文件证明的真完全分类 = **Layer1·标签商分类（构造子穷尽）**：
  对 `TrendKind` 结构归纳，任意走势类型必属三构造子之一，无第四构造子（这是 initiality
  泛性质 = 结构归纳原理的直接推论，机器可检验，L0）。

  **codex#1 硬规则（命名诚实）**：μF/inductive **只**证「语法生成无遗漏 + 语法相等下
  构造子互斥」，**不自动**给语义 ∼ 下的双射。本文件的 `trend_trichotomy` /
  `trend_dichotomy_total` / `trend_consolidation_disjoint` 是 **P 侧（标签集）** 的覆盖+互斥，
  **不冒充** 语义双射 `X/∼ ≅ P`。

  **Layer2（语义侧）见 `Strict/Trend.lean`**：对象 `WalkX`（细结构）+ 语义等价
  `SameChanStructure`（级别+方向+中枢序列）+ 不变量 `I`，证 invariant 成立 / realized 成立 /
  **complete 失败**（两个趋势可中枢数不同却同标签）⟹ 诚实裁定 {趋势,盘整} 只分类
  「最高层走势类型」，不是语义双射。Layer2 必须放下游文件（import RecursiveConstruction），
  因 RecursiveConstruction 已 import 本文件——若本文件反向 import 则循环依赖。

  认识论等级：本文件全部 L0（定义内蕴，从 inductive 结构归纳推导，无经验数据）。
-/

namespace Formal.TrendTrichotomy

/-- 走势方向（趋势的定义属性；盘整无方向，见 qushi.md 第31课）。 -/
inductive Direction where
  | up
  | down
deriving DecidableEq, Repr

/--
  走势类型分类（第17课走势三分法）。

  与 Rust `recursive_t::types::TrendKind` 逐字同构：
  - `consolidation` ↔ `Consolidation`（盘整：恰好 1 个中枢）
  - `upTrend`       ↔ `UpTrend`（上涨趋势：≥2 个依次向上中枢）
  - `downTrend`     ↔ `DownTrend`（下跌趋势：≥2 个依次向下中枢）

  这是一个 **sum type 三构造子**——构造子集穷尽由走势分解定理一钉死。
  "无第四构造子"不是断言，是 `inductive` 的结构归纳原理（initiality 泛性质）。
-/
inductive TrendKind where
  /-- 盘整：某完成的走势类型只包含 1 个中枢（zoushi.md 第17课）。 -/
  | consolidation
  /-- 上涨趋势：≥2 个依次同向（向上）中枢（zoushi.md 第17课）。 -/
  | upTrend
  /-- 下跌趋势：≥2 个依次同向（向下）中枢（zoushi.md 第17课）。 -/
  | downTrend
deriving DecidableEq, Repr

open TrendKind

/--
  趋势 = 上涨 ∪ 下跌（zoushi 第17课答疑 L606："上涨、下跌合起来叫趋势"）。
  二分法（趋势 / 盘整）与三分法（上涨 / 下跌 / 盘整）的桥接谓词。
-/
def isTrend : TrendKind → Bool
  | consolidation => false
  | upTrend => true
  | downTrend => true

/-- 盘整 = 非趋势（二分法的另一侧）。 -/
def isConsolidation (t : TrendKind) : Bool := !(isTrend t)

/--
  ★Layer1·真完全分类（构造子穷尽，L0）：任意走势类型必属三构造子之一。

  这是对 `TrendKind` 的结构归纳（穷举所有构造子）。证明用 `cases` 穷举三个构造子——
  Lean 接受证明 = 构造子集 **穷尽**（无遗漏分支）。这正是 603 的核心命题：
  "无第四构造子"是 initiality 泛性质（结构归纳）的定理，不是经验断言。

  **命名诚实（codex#1）**：这是 P 侧（标签集）的穷尽，**不** 蕴含语义双射 X/∼≅P
  （那是 Layer2 的事，见 Strict/Trend.lean，且 complete 在此分类下失败）。
-/
theorem trend_trichotomy (t : TrendKind) :
    t = consolidation ∨ t = upTrend ∨ t = downTrend := by
  cases t
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/--
  ★Layer1·二分法 = 三分法（走势分解定理一 + qushi/009号"二分法与三分法中的盘整是同一对象"）。

  每个走势类型要么是趋势（上涨/下跌），要么是盘整——二者穷尽且互斥（标签层覆盖）。
-/
theorem trend_dichotomy_total (t : TrendKind) :
    isTrend t = true ∨ isConsolidation t = true := by
  cases t <;> simp [isTrend, isConsolidation]

/-- ★Layer1·趋势与盘整互斥（不能同时为真，标签层互斥）。 -/
theorem trend_consolidation_disjoint (t : TrendKind) :
    ¬ (isTrend t = true ∧ isConsolidation t = true) := by
  cases t <;> simp [isTrend, isConsolidation]

/--
  方向只属于趋势（qushi.md 第31课 L390："盘整哪里有什么方向，只有趋势才有方向"）。
  把方向作为趋势构造子的字段建模——盘整构造子不携带方向。
-/
def trendDirection : TrendKind → Option Direction
  | consolidation => none
  | upTrend => some Direction.up
  | downTrend => some Direction.down

/-- 趋势必有方向，盘整必无方向（构造子字段的完全分类）。 -/
theorem direction_iff_trend (t : TrendKind) :
    (trendDirection t).isSome = isTrend t := by
  cases t <;> rfl

end Formal.TrendTrichotomy
