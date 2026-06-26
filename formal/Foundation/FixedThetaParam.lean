/-
  Foundation/FixedThetaParam.lean

  固定参数 Θ 的类型化部件结构（task #95，架构吸收：codex Origin/SourceAxioms.lean）

  本文件把 codex Origin/SourceAxioms 中 `FixedTheta` 结构的 12 个类型部件
  （data/parse/scale/signal/mirror/voice/phase/ledger/leverage/risk/execution/tieBreak）
  迁入我们的 Foundation 作严格地基，并补齐 Foundation 此前缺失的关系性基础引理
  （`Total`/`SingleValued`/`TotalUnique`/`total_unique_of_fun`）。

  ## 与 Foundation/CompleteClassification 的关系（复用 vs 补充）

  Foundation/CompleteClassification.lean 已定义 `NewChanlun.ExistsUnique`——本文件
  `import CompleteClassification` 直接复用它，**不重定义**（no-patch-mentality：不重复已有定义）。
  Foundation 此前**没有** `Total`/`SingleValued`/`TotalUnique`/`total_unique_of_fun`
  （这些只存在于 codex Origin/SourceAxioms，未进我们 Foundation），故本文件在
  `NewChanlun.Foundation` 命名空间补齐它们，建立在复用的 `NewChanlun.ExistsUnique` 之上。

  ## FixedTheta：参数化的固定参数容器

  `FixedTheta` 是一个把 Θ 引擎的 12 个职责拆成 12 个类型部件的结构。
  它**不约束**这些类型的内容（每个字段就是一个 `Type`），它形式化的是
  「Θ 引擎由这 12 个相互正交的职责类型组成」这一架构事实。
  具体缠论实例化（笔/线段/中枢填入 data/parse 等）由下游 #88 完成，本文件只立架构骨架。

  ## 认识论等级：L0（纯定义/类型构造）

  全部内容定义内蕴：结构声明 + 关系引理的纯逻辑证明，不依赖任何数据。
  Lean build 通过 = 类型/引理逻辑正确（L0），不是 Θ 引擎经验有效性的声明
  （formalization-validity-domain）。

  standalone：不 import HybridAssembly（避免与 #93 耦合）。仅 import CompleteClassification。
-/

import CompleteClassification

namespace NewChanlun.Foundation

universe u v

/-! ## 关系性基础引理（补 Foundation 缺口）

  Foundation/CompleteClassification 只提供 `NewChanlun.ExistsUnique`（下方直接复用）。
  以下 `Total`/`SingleValued`/`TotalUnique`/`total_unique_of_fun` 是 Foundation 此前
  缺失的部件，原存于 codex Origin/SourceAxioms——迁入此处补齐。
-/

/-- 复用 Foundation/CompleteClassification 的 `ExistsUnique`，避免重定义。
    `NewChanlun.ExistsUnique p := ∃ x, p x ∧ ∀ y, p y → y = x`。 -/
abbrev ExistsUnique {A : Type u} (p : A → Prop) : Prop := NewChanlun.ExistsUnique p

/-- 关系 `R` 是全的：每个 `a` 至少有一个 `b` 与之关联。 -/
def Total {A : Type u} {B : Type v} (R : A → B → Prop) : Prop :=
  ∀ a, ∃ b, R a b

/-- 关系 `R` 是单值的：同一个 `a` 关联的 `b` 至多一个。 -/
def SingleValued {A : Type u} {B : Type v} (R : A → B → Prop) : Prop :=
  ∀ a b c, R a b → R a c → b = c

/-- 关系 `R` 是全且唯一的：每个 `a` 恰好有一个 `b`（= 全 + 单值的合取形式）。 -/
def TotalUnique {A : Type u} {B : Type v} (R : A → B → Prop) : Prop :=
  ∀ a, ExistsUnique (fun b => R a b)

/-- 任何函数 `f` 诱导的关系 `fun a b => f a = b` 都是全且唯一的。 -/
theorem total_unique_of_fun {A : Type u} {B : Type v} (f : A → B) :
    TotalUnique (fun a b => f a = b) := by
  intro a
  refine ⟨f a, rfl, ?_⟩
  intro y hy
  exact hy.symm

/-- 全且唯一 ⟹ 全 ∧ 单值（两种刻画等价的一个方向）。 -/
theorem total_and_single_of_total_unique {A : Type u} {B : Type v}
    {R : A → B → Prop} (h : TotalUnique R) :
    Total R ∧ SingleValued R := by
  constructor
  · intro a
    rcases h a with ⟨b, hb, _⟩
    exact ⟨b, hb⟩
  · intro a b c hb hc
    rcases h a with ⟨d, _, hd⟩
    exact (hd b hb).trans (hd c hc).symm

/-! ## FixedTheta：Θ 引擎的 12 类型部件 -/

/-- 固定参数 Θ 的类型化容器：把 Θ 引擎拆成 12 个相互正交的职责类型。

    字段对应 codex Origin/SourceAxioms.FixedTheta（架构吸收，逐字段对齐）：
    - `data`：原始市场数据类型
    - `parse`：解析（K线→分型→笔→线段→走势→中枢）产物类型
    - `scale`：级别（递归层级）类型
    - `signal`：信号（买卖点/背驰）类型
    - `mirror`：镜像（多空对称）变换类型
    - `voice`：声部（赋格多级别声部）类型
    - `phase`：资本阶段（取本金三阶段）类型
    - `ledger`：账本（R=Π-A-W）类型
    - `leverage`：杠杆类型
    - `risk`：风险投影类型
    - `execution`：执行（订单/进出场）类型
    - `tieBreak`：决胜（同优先级仲裁）类型

    每个字段是一个裸 `Type`：本结构只声明「Θ 由这 12 个职责类型组成」，
    不约束其内容。具体缠论实例化由 #88 下游填入。 -/
structure FixedTheta where
  data : Type
  parse : Type
  scale : Type
  signal : Type
  mirror : Type
  voice : Type
  phase : Type
  ledger : Type
  leverage : Type
  risk : Type
  execution : Type
  tieBreak : Type

/-- FixedTheta 的部件计数语法记录：恰好 12 个职责类型。
    这是对结构字段数的显式断言，若未来增删字段需同步更新此处（可观测约束）。 -/
def fixedThetaPartCount : Nat := 12

/-- FixedTheta 部件计数等于 12（与结构字段数一致的语法记录）。 -/
theorem fixedTheta_part_count_eq : fixedThetaPartCount = 12 := rfl

end NewChanlun.Foundation
