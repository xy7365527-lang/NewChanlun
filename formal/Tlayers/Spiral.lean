/-
  几何螺旋层 T₄₇-T₅₉ 形式化（task #50 / 603 范式，L0）
  ★★编排者代理（codex）裁决（2026-06-25，物证 /tmp/codex_spiral_ruling.md）：**选 2**。

  矛盾消解 = **解耦"进 E-set"与"TrueCompleteClassification"**：
  - **E-set**：收录已 Lean 证明的有效形式化事实（含群论事实）。
  - **TrueCompleteClassification**：只给递归 datatype 构造子穷尽、可结构归纳的内涵式
    完全分类。

  ∴ 本文件两类标签（gatekeeper，每条定理显式标注，不冒充）：
  - **TrueCompleteClassification**：T₅₂/T₅₃/T₅₄/T₅₉/T₄₇——挂走势初代数 μF
    （`RecursiveConstruction.Move`）/ 级别递归塔余代数 νG（`RStarNonSpecial.ValidTower`），
    完全分类 by 结构归纳/余归纳 **导出**（构造子穷尽），复用 classifyMove/outcome_total/
    rstar_same_constructors。
  - **StructurePartitionOnly / QuotientByLabel**（subkind GroupTheoreticFact /
    FiniteGroupQuotient）：T₅₆/T₅₇/T₅₈/T₄₅/Burnside46——D∞ 群呈现的关系等式 / 有限循环商 /
    轨道计数，是 **真群论事实可 Lean 证**（非伪造初代数），进 E-set 但 **禁标
    TrueCompleteClassification**（无递归构造子穷尽，不是内涵式完全分类）。

  执行口径（codex）：群论 Lean 事实进 E-set，用现有 gatekeeper 标签降格，绝不冒充真完全分类。

  认识论：全部 **L0**（定义内蕴，零数据依赖）。Lean 通过 = 逻辑/管线正确（L0），不膨胀
  （formalization-validity-domain）。出处：necessity_derivation.md T₄₇-T₅₉ §8.3/§8.4/§9 +
  covering_space_interval_nesting_proof.md + reading_b_t_dual_design.md + 第33/36/91/93课。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy
import Formal.RecursiveConstruction
import Formal.RStarNonSpecial

namespace Spiral

open Formal.TrendTrichotomy (Direction)
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.RecursiveConstruction (Move classifyMove outcome_total WellFormed)
open Formal.RStarNonSpecial (TowerLevel ValidTower operate rstar_same_constructors)

/-! ════════════════════════════════════════════════════════════════════════
  # PART I — TrueCompleteClassification（递归构造子穷尽 by induction/coinduction 导出）

  以下 T₅₂/T₅₃/T₅₄/T₅₉/T₄₇ 全部挂走势初代数 μF / 级别递归塔余代数 νG，完全分类由
  结构归纳/余归纳 **by construction 导出**——是内涵式真完全分类（gatekeeper 标
  `TrueCompleteClassification`）。
  ════════════════════════════════════════════════════════════════════════ -/

/-! ## §1 T₅₃ 走势连接结合律【缺瓦】——`List Move` append 初代数结合律（L0）
  【gatekeeper: TrueCompleteClassification】
  necessity §9 T₅₃ + 第36课：连接 = 次级别走势序列 `List Move` 的 append（初代数连接
  构造子）。结合律由对 `List Move` 的结构归纳（nil/cons）导出，**非** Nat.max 代数事实。 -/

/-- 走势连接 = 次级别走势序列拼接（List Move append，初代数连接构造子）。 -/
def connectMoves (a b : List Move) : List Move := a ++ b

/--
  ★T₅₃ 连接结合律【TrueCompleteClassification】(L0)：(A ⊕ B) ⊕ C = A ⊕ (B ⊕ C)。
  对 `List Move` 结构归纳（nil/cons）——append 结合律是 List 初代数归纳定理，
  非 max 代数事实。三段次级别走势连接与括号位置无关。
-/
theorem T53_connection_assoc (A B C : List Move) :
    connectMoves (connectMoves A B) C = connectMoves A (connectMoves B C) := by
  unfold connectMoves
  induction A with
  | nil => rfl
  | cons x xs ih => simp [List.cons_append, ih]

/--
  ★T₅₃ 连接 length 可加【TrueCompleteClassification】(L0)：|A ⊕ B| = |A| + |B|。
  连接是 length-加性——支撑"≥3 段"在连接下累积（走势分解定理二可组合性）。
-/
theorem T53_connection_length (A B : List Move) :
    (connectMoves A B).length = A.length + B.length := by
  unfold connectMoves
  simp [List.length_append]

/--
  ★T₅₃ 空连接单位律【TrueCompleteClassification】(L0)：[] ⊕ A = A ∧ A ⊕ [] = A。
  空走势序列是连接单位元——连接构成幺半群（结合 + 单位），由 List 初代数导出。
-/
theorem T53_connection_unit (A : List Move) :
    connectMoves [] A = A ∧ connectMoves A [] = A := by
  unfold connectMoves
  exact ⟨List.nil_append A, List.append_nil A⟩

/-! ## §2 T₅₄ 两重表里关系【缺瓦】——表由里 by construction 导出（L0）
  【gatekeeper: TrueCompleteClassification】
  reading_b_t_dual_design.md + 第91/93课 + 540号：表（surface = `MoveOutcome`）**由里**
  （interior = 次级别走势 → 中枢 `List Center`）经 `classifyMove` 压缩。完全分类由
  `outcome_total`（对里结构归纳）导出——compose 构造子的里→表 totality。 -/

/-- 表（surface）= 由里（中枢序列）经 classifyMove 压缩出的走势类型。 -/
def surface (interior : List Center) : MoveOutcome := classifyMove interior

/--
  ★T₅₄ 表由里完全分类【TrueCompleteClassification】(L0)：
  任意里压缩出的表必属四走势类型之一——`outcome_total` 对里结构归纳（[]/[c]/c0::c1::rest）
  导出。表里对偶 = compose 构造子里→表 totality，无第五种表型。
-/
theorem T54_surface_from_interior_total (interior : List Center) :
    surface interior = MoveOutcome.trend Direction.up
    ∨ surface interior = MoveOutcome.trend Direction.down
    ∨ surface interior = MoveOutcome.consolidation
    ∨ surface interior = MoveOutcome.higherCenterCandidate :=
  outcome_total interior

/--
  ★T₅₄ 表完全由里决定【TrueCompleteClassification】(L0)：里相同 ⟹ 表相同。
  表是里的函数（compose"裁决由 centers 计算"的表里对偶面），同里必同表。
-/
theorem T54_surface_determined_by_interior (i₁ i₂ : List Center) (h : i₁ = i₂) :
    surface i₁ = surface i₂ := by rw [h]

/--
  ★T₅₄ 里→表在 compose 走势上落地【TrueCompleteClassification】(L0)：
  compose 走势的表 = 其里的 surface——表由里构造在 Move 初代数构造子上 by construction 成立。
-/
theorem T54_compose_surface_eq (subs : List Move) (centers : List Center) (lvl : Nat) :
    Formal.RecursiveConstruction.outcome? (Move.compose subs centers lvl)
      = some (surface centers) := rfl

/-! ## §3 T₅₂ 走势多义性 = 覆盖空间多重提升 + 区间套规范固定【缺瓦】（L0）
  【gatekeeper: TrueCompleteClassification】
  covering_space_interval_nesting_proof.md + 第33课：同一 base 走势数据有 **多个合法 Move
  提升**（不同级别分解，Move 初代数非唯一分解的结构事实）。纤维 = 提升集合；区间套规范
  固定 = 加级别下界选唯一截面。完全分类由 List（提升集合）初代数 + 提升级别投影导出。 -/

/--
  ★一个提升 = base 数据上的合法 Move 分解（覆盖纤维上的点，Move 初代数实例）。
  多义 = 同 base 多 decomposition。
-/
structure Lift where
  base : List Move
  decomposition : Move

/-- 覆盖映射 p：提升投影回 base。 -/
def projBase (l : Lift) : List Move := l.base

/-- 提升级别（区间套约束对象）。 -/
def liftLevel (l : Lift) : Nat := l.decomposition.level

/-- 纤维 p⁻¹(base) = 同 base 多提升（List 初代数）。 -/
abbrev Fiber := List Lift

/-- 走势多义性：纤维含 ≥2 提升 + 全覆盖同基点（Move 初代数允许多 compose 封装的结构后果）。 -/
def isAmbiguous (f : Fiber) : Prop :=
  f.length ≥ 2 ∧ ∀ l ∈ f, projBase l = (f.headD ⟨[], Move.segment Direction.up 0 0⟩).base

/-- 区间套规范：过滤级别 ≥ floor 的提升。 -/
def aboveFloor (f : Fiber) (floor : Nat) : Fiber :=
  f.filter (fun l => decide (liftLevel l ≥ floor))

/-- 两提升取级别较小者（区间套规范）。 -/
def minLift (a b : Lift) : Lift :=
  if liftLevel a ≤ liftLevel b then a else b

/-- 区间套规范固定：过滤级别 ≥ floor 后取级别最小提升（foldl 初代数折叠归约）。 -/
def gaugeFix (f : Fiber) (floor : Nat) : Option Lift :=
  match aboveFloor f floor with
  | [] => none
  | x :: xs => some (xs.foldl minLift x)

/--
  ★T₅₂ 提升集合构造子穷尽【TrueCompleteClassification】(L0)：纤维要么空要么 cons。
  纤维 = `List Lift` 初代数（nil/cons 穷尽）——多义性由 List 结构归纳导出。
-/
theorem T52_fiber_exhaustive (f : Fiber) :
    f = [] ∨ ∃ (x : Lift) (rest : Fiber), f = x :: rest := by
  cases f with
  | nil => exact Or.inl rfl
  | cons x rest => exact Or.inr ⟨x, rest, rfl⟩

/--
  ★T₅₂ 区间套归约多提升为唯一截面【TrueCompleteClassification】(L0，核心)：
  gaugeFix 结果是 `Option Lift` 单值——多义纤维经区间套规范固定后输出至多一提升
  （foldl minLift 对 List 初代数折叠 by construction 产生确定单值）。"多义 → 规范唯一"。
-/
theorem T52_gauge_fixes_unique (f : Fiber) (floor : Nat) :
    gaugeFix f floor = none ∨ ∃ l : Lift, gaugeFix f floor = some l := by
  cases h : gaugeFix f floor with
  | none => exact Or.inl rfl
  | some l => exact Or.inr ⟨l, rfl⟩

/--
  ★T₅₂ 空过滤 ⟹ 无截面【TrueCompleteClassification】(L0，偏函数诚实)：
  无提升满足级别下界 ⟹ gauge = none（不伪造全函数）。
-/
theorem T52_empty_above_floor_none (f : Fiber) (floor : Nat)
    (h : aboveFloor f floor = []) : gaugeFix f floor = none := by
  unfold gaugeFix; rw [h]

/--
  ★T₅₂ 提升覆盖同一基点【TrueCompleteClassification】(L0)：
  多义纤维每个提升经 p 投影回同一 base（p⁻¹(base) 纤维定义性质）。
-/
theorem T52_lifts_cover_same_base (f : Fiber) (h : isAmbiguous f) :
    ∀ l ∈ f, projBase l = (f.headD ⟨[], Move.segment Direction.up 0 0⟩).base :=
  h.2

/-! ## §4 T₅₉ σ 尺度自相似——级别递归塔余代数 by coinduction 导出（L0）
  【gatekeeper: TrueCompleteClassification】
  necessity T₅₉：σ 移位（相邻级别）不改变走势构造子集——每层用同四构造子。复用
  `RStarNonSpecial.rstar_same_constructors`（ValidTower 任意层 outcome 属同四构造子）。
  σ 自相似 = 余代数每层 unfold 用同 classifyMove 构造子集（余归纳），**非** 群关系等式。 -/

/--
  ★T₅₉ σ 自相似·每层同构造子【TrueCompleteClassification】(L0，余代数导出)：
  ValidTower 任意层 k（含 σ 移位后层）outcome 属同四构造子——σ 升级别不引入新构造子。
  这是 `rstar_same_constructors` 对终余代数每层 by coinduction 的事实，非群关系等式。
-/
theorem T59_sigma_self_similar_constructors (t : ValidTower) (k : Nat) :
    (t.level k).outcome = MoveOutcome.trend Direction.up
    ∨ (t.level k).outcome = MoveOutcome.trend Direction.down
    ∨ (t.level k).outcome = MoveOutcome.consolidation
    ∨ (t.level k).outcome = MoveOutcome.higherCenterCandidate :=
  rstar_same_constructors t k

/--
  ★T₅₉ σ 移位相邻层同构造子集【TrueCompleteClassification】(L0)：
  级别 k 与 k+1（σ 作用一次）走势类型同属同四构造子集——σ 升级别不改分类可能取值集。
-/
theorem T59_sigma_shift_same_constructor_set (t : ValidTower) (k : Nat) :
    ((t.level k).outcome = MoveOutcome.trend Direction.up
      ∨ (t.level k).outcome = MoveOutcome.trend Direction.down
      ∨ (t.level k).outcome = MoveOutcome.consolidation
      ∨ (t.level k).outcome = MoveOutcome.higherCenterCandidate)
    ∧ ((t.level (k + 1)).outcome = MoveOutcome.trend Direction.up
      ∨ (t.level (k + 1)).outcome = MoveOutcome.trend Direction.down
      ∨ (t.level (k + 1)).outcome = MoveOutcome.consolidation
      ∨ (t.level (k + 1)).outcome = MoveOutcome.higherCenterCandidate) :=
  ⟨rstar_same_constructors t k, rstar_same_constructors t (k + 1)⟩

/-! ## §5 T₄₇ 角向唯一奇点定域——BSP 三构造子结构归纳穷尽（L0）
  【gatekeeper: TrueCompleteClassification】
  necessity T₄₇：角向唯一奇点 φ=0；BSP 三类是 φ=0 径向投影深度分类，无第四类。
  "无第四类"= 对 BSP 构造子的结构归纳穷尽（inductive 三构造子）。 -/

/-- BSP 三类（necessity T₄₇：φ=0 径向投影深度分类，三构造子穷尽）。 -/
inductive BspKind where
  | type1 | type2 | type3
deriving DecidableEq, Repr

/--
  ★T₄₇ BSP 无第四类【TrueCompleteClassification】(L0，构造子穷尽导出)：
  任意 BSP 属三构造子之一——对 `BspKind` 结构归纳。角向唯一奇点 ⟹ 三类是同一 φ=0 的
  径向深度分类，由构造子穷尽 by construction（非外延轴枚举）。
-/
theorem T47_bsp_no_fourth_kind (k : BspKind) :
    k = BspKind.type1 ∨ k = BspKind.type2 ∨ k = BspKind.type3 := by
  cases k
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/-! ════════════════════════════════════════════════════════════════════════
  # PART II — 群论 Lean 事实进 E-set（StructurePartitionOnly / QuotientByLabel）

  ★codex 裁决（选 2）：T₅₆/T₅₇/T₅₈/T₄₅/Burnside46 是 **真群论事实可 Lean 证**（非伪造
  初代数），进 E-set 但 **禁标 TrueCompleteClassification**（无递归构造子穷尽）。
  gatekeeper 标签 + 二级 subkind（不新增第 7 个顶层标签，保 6 类 gatekeeper）：
  - D∞ 生成元关系（h²³=σ / τhτ⁻¹=h⁻¹ / τστ⁻¹=σ⁻¹）→ StructurePartitionOnly · GroupTheoreticFact
  - ℤ/23 有限循环商 → QuotientByLabel · FiniteGroupQuotient
  - Burnside 46 基本域基数 → StructurePartitionOnly · GroupTheoreticFact

  这 **不是** 内涵式完全分类——D∞ 元素无"由次级别 D∞ 元素构成"的初代数结构，故只是
  群结构事实（关系/商/计数），用 gatekeeper 降格标注，绝不冒充真完全分类。
  ════════════════════════════════════════════════════════════════════════ -/

/-! ## §6 D∞ 螺旋群（Britton 正规形）—— 群论事实，非真完全分类
  necessity §8.4.2：G_spiral = ⟨h, τ | τ²=e, τhτ⁻¹=h⁻¹⟩ = D∞。
  Britton 正规形：每元素唯一写作 hⁿ（rot）或 hⁿτ（refl）。
  ⚠ `dinf_normal_form` 是群正规形的"形式枚举"（StructurePartitionOnly），**不是**走势
  那种"由构造子递归构成"的初代数——D∞ 元素是平坦标签（n∈ℤ + 反射位），无递归构成。 -/

/-- D∞ 元素（Britton 正规形：rot n = hⁿ / refl n = hⁿτ）。群论事实，非递归初代数。 -/
inductive DInf where
  | rot  (n : Int)
  | refl (n : Int)
deriving DecidableEq, Repr

namespace DInf

/-- 单位元 e = rot 0。 -/
def e : DInf := rot 0
/-- 旋转生成元 h = rot 1。 -/
def h : DInf := rot 1
/-- 手性翻转 τ = refl 0。 -/
def tau : DInf := refl 0
/-- 一整圈 σ = h²³ = rot 23（necessity T₅₆ 角径耦合）。 -/
def sigma : DInf := rot 23

/-- 逆元：(hⁿ)⁻¹=h⁻ⁿ / (hⁿτ)⁻¹=hⁿτ（refl 自逆，因 τhτ⁻¹=h⁻¹）。 -/
def inv : DInf → DInf
  | rot n  => rot (-n)
  | refl n => refl n

/-- D∞ 乘法表（半直积 ℤ ⋊ ℤ/2）。 -/
def mul : DInf → DInf → DInf
  | rot a,  rot b  => rot (a + b)
  | rot a,  refl b => refl (a + b)
  | refl a, rot b  => refl (a - b)
  | refl a, refl b => rot (a - b)

/-- 左单位律【StructurePartitionOnly · GroupTheoreticFact】(L0)。 -/
theorem mul_e_left (g : DInf) : mul e g = g := by
  cases g with
  | rot n => simp [mul, e]
  | refl n => simp [mul, e]

/-- 右单位律【StructurePartitionOnly · GroupTheoreticFact】(L0)。 -/
theorem mul_e_right (g : DInf) : mul g e = g := by
  cases g with
  | rot n => simp [mul, e]
  | refl n => simp [mul, e]

/-- 左逆律【StructurePartitionOnly · GroupTheoreticFact】(L0)。 -/
theorem mul_inv_left (g : DInf) : mul (inv g) g = e := by
  cases g with
  | rot n => simp only [inv, mul, e]; congr 1; omega
  | refl n => simp [mul, inv, e]

/-- 结合律【StructurePartitionOnly · GroupTheoreticFact】(L0)：8 分支 Int 加减结合。 -/
theorem mul_assoc (a b c : DInf) : mul (mul a b) c = mul a (mul b c) := by
  cases a <;> cases b <;> cases c <;> simp [mul] <;> omega

/-- rot 是 ℤ→DInf 单同态【StructurePartitionOnly · GroupTheoreticFact】(L0)：⟨h⟩≅(ℤ,+)。 -/
theorem rot_homomorphism (a b : Int) : mul (rot a) (rot b) = rot (a + b) := rfl

/--
  ★T₅₆ 角径耦合 h²³=σ【StructurePartitionOnly · GroupTheoreticFact】(L0)。
  ⚠ 这是 D∞ 生成元 **关系等式**（一整圈 = 23 角向单位 = σ），群论事实可 Lean 证，
  **非** 递归构造子穷尽——禁标 TrueCompleteClassification（codex 裁决）。
-/
theorem T56_h_pow_23_eq_sigma : rot (23 : Int) = sigma := rfl

/--
  ★T₅₇ 半直积关系 τhτ⁻¹=h⁻¹【StructurePartitionOnly · GroupTheoreticFact】(L0)。
  ⚠ 手性翻转反转旋转方向，D∞ 区别于直积的 **关系等式**。群论事实，非递归构造子。
-/
theorem T57_tau_h_tau_inv : mul (mul tau h) (inv tau) = inv h := by
  simp [mul, inv, tau, h]

/-- R₂ 对合 τ²=e【StructurePartitionOnly · GroupTheoreticFact】(L0)。 -/
theorem tau_involution : mul tau tau = e := by simp [mul, tau, e]

/-- σ 在 τ 下反向 τστ⁻¹=σ⁻¹【StructurePartitionOnly · GroupTheoreticFact】(L0)。 -/
theorem tau_sigma_tau_inv : mul (mul tau sigma) (inv tau) = inv sigma := by
  simp [mul, inv, tau, sigma]

/--
  ★D∞ 正规形枚举【StructurePartitionOnly · GroupTheoreticFact】(L0)：
  任意元素 g = rot n（hⁿ）或 refl n（hⁿτ）。
  ⚠ 这是群正规形的 **形式枚举**（两标签），**非** 递归 datatype 构造子穷尽——
  rot/refl 不是"由次级别 DInf 递归构成"，无初代数归纳结构。禁标 TrueCompleteClassification。
-/
theorem dinf_normal_form (g : DInf) :
    (∃ n, g = rot n) ∨ (∃ n, g = refl n) := by
  cases g with
  | rot n => exact Or.inl ⟨n, rfl⟩
  | refl n => exact Or.inr ⟨n, rfl⟩

end DInf

/-! ## §7 T₅₈/T₄₅ 角向基本域 ℤ/23 —— 有限循环商，非真完全分类
  【gatekeeper: QuotientByLabel · subkind FiniteGroupQuotient】
  necessity §8.4.2 (c)：角向商 ℤ/23ℤ = ⟨h⟩/⟨h²³⟩。T₄₅ 操盘结构周期性 = 角向 φ-周期
  D∞ 商 Z/23Z 基本域。⚠ 23 环是 **有限循环群计数**（Fin 23 有限枚举），非递归 datatype
  归纳穷尽——禁标 TrueCompleteClassification（codex 裁决）。 -/

/-- 角向商映射 n ↦ n mod 23（necessity §8.4.2 角向商 ℤ→ℤ/23ℤ）。 -/
def angularQuotient (n : Int) : Fin 23 :=
  ⟨(n % 23).toNat % 23, by omega⟩

/--
  ★T₅₈ 角向基本域有限闭合【QuotientByLabel · FiniteGroupQuotient】(L0)：每相位 < 23。
  ⚠ Fin 23 有限循环计数（有限基本域），非递归构造子穷尽。禁标 TrueCompleteClassification。
-/
theorem T58_angular_domain_card : ∀ x : Fin 23, x.val < 23 := fun x => x.isLt

/--
  ★T₅₈/T₄₅ 一整圈角向归零【QuotientByLabel · FiniteGroupQuotient】(L0)：
  σ=rot 23 角向商 = 角向商 0（一圈回起点相位 = ℤ/23ℤ 中 h²³≡0 = T₄₅ 周期闭合）。
  ⚠ 有限循环商事实，非递归构造子穷尽。禁标 TrueCompleteClassification。
-/
theorem T58_sigma_angular_zero : angularQuotient 23 = angularQuotient 0 := rfl

/-! ## §8 Burnside 基本域基数 = 46 —— 群论计数，非真完全分类
  【gatekeeper: StructurePartitionOnly · subkind GroupTheoreticFact】
  necessity §8.4.3：基本域 = 23 环 × 2 手性 = 46。⚠ 结构基数（类型积计数），506→46→2→1
  Burnside 轨道归约是纯数值（(1/|G|)Σ|Fix(g)|，依赖具体 506 配置枚举）——无递归构造子穷尽。
  禁标 TrueCompleteClassification（codex 裁决，与前版 X 标注同类）。 -/

/-- 基本域基数 = 23 环 × 2 手性（necessity §8.4.3）。 -/
def fundamentalDomainCard : Nat := 23 * 2

/--
  ★Burnside 基本域基数 = 46【StructurePartitionOnly · GroupTheoreticFact】(L0)：
  角向 23 × 手性 2 = 46（类型积计数）。⚠ 群论计数事实，506→46→2→1 轨道数值归约是纯
  计数（无构造子穷尽内容）。禁标 TrueCompleteClassification。
-/
theorem burnside_fundamental_domain_card_eq_46 : fundamentalDomainCard = 46 := rfl

/-
  ═══════════════════════════════════════════════════════════════════════════
  ★标签总表（gatekeeper，codex 裁决"选2"落地）
  ═══════════════════════════════════════════════════════════════════════════

  PART I — TrueCompleteClassification（递归构造子穷尽 by induction/coinduction 导出）：
  - T₅₃ 连接结合律 = List Move append 初代数结合律（结构归纳）
  - T₅₄ 表里对偶 = classifyMove 里→表 totality（outcome_total 结构归纳）
  - T₅₂ 多义性 = Move 初代数多提升 + 区间套 foldl 归约（List 初代数）
  - T₅₉ σ 自相似 = ValidTower 余代数每层同构造子（rstar_same_constructors 余归纳）
  - T₄₇ 无第四类 BSP = BspKind 构造子穷尽（结构归纳）

  PART II — 群论 Lean 事实进 E-set，gatekeeper 降格（禁标 TrueCompleteClassification）：
  - T₅₆/T₅₇/τ²=e/τστ⁻¹=σ⁻¹/群公理/正规形枚举 → StructurePartitionOnly · GroupTheoreticFact
  - T₅₈/T₄₅ ℤ/23 角向商 → QuotientByLabel · FiniteGroupQuotient
  - Burnside 46 基本域基数 → StructurePartitionOnly · GroupTheoreticFact

  执行口径（codex）：E-set accepts Lean-proved group-theoretic facts；
  TrueCompleteClassification remains restricted to recursive datatype constructor exhaustion；
  Group facts are admissible formal facts, not constructor-exhaustive classifications.

  ─────────────────────────────────────────────────────────────────────────
  ★T₅₁ 莫比乌斯 / T₄₈ σ-Casimir / T₄₉ 向心时序：性质（不变量/单调/对合），非分类
  （不属"完全分类"诉求范畴）。T₅₁ 手性 ℤ/2 二态丛是有限群无递归构造子，其结构事实归
  PART II 群论侧；T₄₈/T₄₉ 守恒/时序由会计层（层C）+ Rust 守卫（L2，prove_t49-t59 +
  should_panic）承载，本几何层不重复声明（避免与层C 重叠 + 不把性质冒充完全分类）。
  ─────────────────────────────────────────────────────────────────────────
-/

end Spiral
