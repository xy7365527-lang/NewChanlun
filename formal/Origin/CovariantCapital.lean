/-
  Origin/CovariantCapital.lean — 方案 A：协变 / 无量纲资本（资本-等变冲突的严格修复）

  ── 存在论位置（编排者权威裁决 → 方案 A 钦定）───────────────────────────────
  信源：`.chanlun/specs/2026-06-28-absolute-capital-equivariance-resolution-pdf-extract.md`
  （commit 9b85c351a8）。三选二定理（P1）+ §1 不相容定理（P1–P2）已证：
    「非平凡尺度等变 + 固定绝对资本上限 + 全定义域策略等变 三者不能同时成立」。
  编排者钦定**方案 A**（资本协变 / 无量纲化，合 no-patch/161 号：务实=妥协被禁）——
  放弃「固定绝对资本」出口，使全域策略自相似 `π_Θ(S_k x)=S_k π_Θ(x)` 保留。

  本文件形式化方案 A 的数学骨架（spec §3 协变资本版严格等变定理 + §5 三阶段归一化）。
  **并存**（不原地改 ConstraintSystem.lean——下游 MainTheorem 契约锚定其绝对参数形式，
  原地改破 GREEN；本模块是协变资本的独立形式化，下游迁移由后续工位决定）。

  ── canonical 依据（spec 逐式，P2–P9）─────────────────────────────────────
    协变资本单位      U_ℓ(x) > 0,  U_{ℓ+k}(S_k x) = a_k U_ℓ(x),  a_k ≠ 1        （P2–P3 方框）
    无量纲化          Ī=I/U_ℓ, W̄=W/U_ℓ, R̄=R/U_ℓ, Ā=A/U_ℓ, L̄^wc=ℒ^wc/U_ℓ, m̄^unit=m^unit/U_ℓ
    尺度不变          S_k I / S_k U_ℓ = a_k I / a_k U_ℓ = I/U_ℓ ⟹ Φ(S_k x)=Φ(x)   （P3 方框）
    协变可行集        𝒦_Θ(x) = {p : Ψ_j(x,p) ≤ b_j, j=1..n}                       （P3）
    齐次等变          Ψ_j(S_k x, S_k p) = a_k^{d_j} Ψ_j(x,p)                       （P3 方框）
    边界协变          b_j(S_k Θ) = a_k^{d_j} b_j(Θ)                                （P4 方框）
    可行集等变        𝒦_{S_kΘ}(S_k x) = S_k 𝒦_Θ(x)                                 （P4 方框，§3 定理）
    目标函数不变      J_{S_k x}(S_k p, S_k p̃) = J_x(p, p̃)                         （P4 方框）
    投影等变          p*(S_k x) = S_k p*(x)                                         （P4 方框）
    强形式            S_k Θ = Θ ⟹ π_Θ(S_k x) = S_k π_Θ(x)                          （P4/P10 方框）

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义 / 代数 / 逻辑，不依赖数据）。等变是结构性质，可证为纯 core 引理。
  `U_ℓ` 协变性 `U_{ℓ+k}(S_k x)=a_k U_ℓ(x)`、约束齐次等变、边界协变是 **L0 假设（设计约束，
  非经验校准）**——它们是 §3 定理的前提（spec §3 原文「若 Ψ_j 齐次等变且 b_j 协变，则...」），
  在本文件中作为 structure 字段 / 定理假设承载，**不**用 Lean `axiom`（合 SourceAxioms 惯例）。
  资本参数的经验最优值（多大上限避免强平）仍是 L2，本裁决与本文件**均不**触及。

  机器可检验命题（Lean build 通过 = 这些代数 / 逻辑命题正确，L0）：
  - `dimensionless_invariant`：S_k 下无量纲比例不变（`(a_k X)/(a_k U) = X/U`，交叉相乘除法-free）。
  - `barred_invariant`：协变单位下带横线资本量在 S_k 下不变（spec P3 核心）。
  - `CovConstraint.holds_shift_iff`：单约束在 S_k 下成员性双条件（齐次等变 + 边界协变 + a_k^{d}>0）。
  - `feasibleSet_equivariant`：可行集等变 `𝒦_{S_kΘ}(S_k x)=S_k 𝒦_Θ(x)`（§3 定理，给证明）。
  - `feasibleSet_equivariant_strong`：无量纲 Θ（`S_kΘ=Θ`）⟹ 同一 Θ 强形式可行集等变。
  - `argmin_equivariant` / `proj_equivariant`：目标函数不变 + 唯一投影 ⟹ 订单等变 `π_Θ(S_k x)=S_k π_Θ(x)`。
  - `ReadyReturn_scale` / `StageOne_scale` / `StageThree_scale` / `phaseOf_scale`：§5 三阶段归一化尺度不变。
  - `leverage_ratio_invariant` / `grossNotional_scale`：杠杆比 `L^G=G/E`（d_j=0 无量纲）本就尺度不变（复用 LeverageCapital）。

  ── [需人工确认]（capital spec §3/§4 边界条件 + 末节汇总，采默认 + 标注，不臆造）────
  各约束尺度指数 `d_j`：spec §3/§4 用 `a_k^{d_j}` 但 PDF **未逐条给出**各约束的 `d_j` 值。
  本文件采**默认**并标注：杠杆比 `dLeverage = 0`（无量纲）、名义额 `dNominal = 1`、二次风险
  `dQuadraticRisk = 2`（spec 末节「合理推测，须坐实」）。`[需人工确认]` 各 K_Θ 约束的 `d_j` 实值。
  其余 [需人工确认]：binding regime 六类穷尽性、`LotCost` 离散颗粒在协变化下的残余尺度破缺。

  ── 复用（Origin.LeverageCapital）──────────────────────────────────────────
  复用 `Side`（SourceAxioms）、`VoicePosition` / `grossNotional`（LeverageCapital，§13）。
  杠杆比 `L^G=G/E` 本就无量纲（`d_j=0`，G 与 E 同 a_k 缩放 ⟹ 比值不变）——本文件证它满足方案 A
  （`leverage_ratio_invariant` + `grossNotional_scale`），坐实 spec「LeverageCapital 已部分满足 A」。

  ── 依赖方向（单向无环，纯 core，不 import legacy / 不改 ConstraintSystem）──────
  CovariantCapital → Origin.LeverageCapital → Origin.SourceAxioms。standalone。
  纯 Lean core：无 Mathlib / Batteries / Std；禁 Fintype / Finset；约束族用 `List` 逐条合取；
  比例用 Int 交叉相乘建模（避免实数除法）；a_k = a^k 用 Nat 幂；等变证明用 core Int 序引理。
  验证：`cd formal && lake env lean Origin/CovariantCapital.lean`。禁 sorry/admit/axiom。

  谱系：绝对资本.pdf 权威裁决（方案 A 钦定）→ 20 页 spec §15/§18/§19/§20 →
        三选二定理 / §1 不相容定理 → 本文件（方案 A 形式化骨架，资本协变化首次形式化裁决）。
-/

import Origin.LeverageCapital

namespace NewChanlun.Origin.CovariantCapital

open NewChanlun.Origin
open NewChanlun.Origin.LeverageCapital

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 级别平移的尺度因子 a_k = a^k（Nat 幂，非平凡 a_k ≠ 1）

  级别平移算子 `S_k` 非平凡地缩放头寸规模（spec §1）：`N(S_k p) = a_k N(p)`，`a_k ≠ 1`。
  本文件建模 per-level 基础缩放因子 `a : Nat`（`2 ≤ a`，非平凡），`a_k = a^k`。
  约束 j 的尺度因子是 `a_k^{d_j} = (a^k)^{d_j}`（§3 齐次等变指数）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★级别平移尺度因子 `aPow` a_k = a^k（L0，Int 值，spec §1 `N(S_k p)=a_k N(p)`）。 -/
def aPow (a k : Nat) : Int := ((a ^ k : Nat) : Int)

/-- ★约束 j 的尺度因子 `cFactor` a_k^{d_j} = (a^k)^{d_j}（L0，Int 值，spec §3 齐次等变指数）。 -/
def cFactor (a k d : Nat) : Int := (((a ^ k) ^ d : Nat) : Int)

/-- ★a_k = a^k > 0（L0）：非负基底的幂正（缩放因子恒正，比例分母合法）。 -/
theorem aPow_pos (a k : Nat) (ha : 0 < a) : 0 < aPow a k := by
  unfold aPow
  exact Int.natCast_pos.mpr (Nat.pow_pos ha)

/-- ★a_k^{d_j} > 0（L0）：嵌套幂正（齐次等变因子恒正，序乘可逆的前提）。 -/
theorem cFactor_pos (a k d : Nat) (ha : 0 < a) : 0 < cFactor a k d := by
  unfold cFactor
  exact Int.natCast_pos.mpr (Nat.pow_pos (Nat.pow_pos ha))

/-- ★★非平凡 a_k ≠ 1（L0，spec §1 方框 `a_k ≠ 1`）：`2 ≤ a ∧ k ≠ 0 ⟹ a^k ≠ 1`。
    这是「级别平移放大头寸规模」的形式坐实——`a_k=1`（平凡尺度）是三选二定理翻转条件之一。 -/
theorem levelScale_ne_one (a k : Nat) (ha : 2 ≤ a) (hk : k ≠ 0) : a ^ k ≠ 1 := by
  have h1 : 1 < a := by omega
  have : 1 < a ^ k := Nat.one_lt_pow hk h1
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 无量纲比例（除法-free 建模）+ 核心尺度不变性

  spec P3：`S_k I / S_k U_ℓ = a_k I / a_k U_ℓ = I / U_ℓ`。纯 core 禁实数除法，用**交叉相乘**
  建模比例相等：`I/U = X/Y ⟺ I·Y = X·U`（`RatioEq`）。S_k 下分子分母同乘 a_k，比例不变。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★比例相等 `RatioEq`（L0，除法-free）：`n₁/d₁ = n₂/d₂ ⟺ n₁·d₂ = n₂·d₁`（交叉相乘）。 -/
def RatioEq (n1 d1 n2 d2 : Int) : Prop := n1 * d2 = n2 * d1

/-- ★比例自反（L0）：`X/U = X/U`。 -/
theorem ratioEq_refl (n d : Int) : RatioEq n d n d := rfl

/--
  ★★无量纲尺度不变性（L0，spec P3 核心方框 `S_k I / S_k U_ℓ = a_k I / a_k U_ℓ = I/U_ℓ`）：
  分子分母同乘 a_k 后比例不变——`(a_k·X)/(a_k·U) = X/U`。证：交叉相乘 `(a_k·X)·U = X·(a_k·U)`
  仅用乘法交换 + 结合（a_k 在两侧对消，无需 a_k 可逆 / 实数除法）。这是方案 A「无量纲化使 S_k 下
  带横线变量不变」的代数根。
-/
theorem dimensionless_invariant (ak X U : Int) : RatioEq (ak * X) (ak * U) X U := by
  unfold RatioEq
  calc (ak * X) * U = (X * ak) * U := by rw [Int.mul_comm ak X]
    _ = X * (ak * U) := Int.mul_assoc X ak U

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 协变资本单位 U_ℓ(x) + 带横线变量的 S_k 不变性

  spec P2–P3：引入每级资本 / 风险单位 `U_ℓ(x) > 0`，协变 `U_{ℓ+k}(S_k x) = a_k U_ℓ(x)`；
  资本量 `q`（I/W/R/A/ℒ^wc/m^unit）同 a_k 齐次缩放 `q(S_k x) = a_k q(x)`（d_j=1 线性资本量）。
  则带横线量 `q̄ = q/U_ℓ` 在 S_k 下不变。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★协变资本单位 `CovUnit`（L0 假设包，spec P2–P3 方框）：bundle 级别平移作用 + 协变律。
  - `a`/`ha`：per-level 基础缩放因子（`2 ≤ a`，a_k = a^k ≠ 1 非平凡）。
  - `shiftX`：级别平移算子 S_k 对状态的作用。
  - `U`：协变资本单位 `U_ℓ(x)`，`pos` 即 `U_ℓ(x) > 0`。
  - `covariant`：协变律 `U_{ℓ+k}(S_k x) = a_k U_ℓ(x)`（**L0 设计约束**，§3 定理前提，非 axiom）。
-/
structure CovUnit (X : Type) where
  a : Nat
  ha : 2 ≤ a
  shiftX : Nat → X → X
  U : Nat → X → Int
  pos : ∀ ℓ x, 0 < U ℓ x
  covariant : ∀ ℓ k x, U (ℓ + k) (shiftX k x) = aPow a k * U ℓ x

/-- ★资本单位正（L0）：`U_ℓ(x) > 0`（比例分母合法）。 -/
theorem CovUnit.unit_pos {X : Type} (CU : CovUnit X) (ℓ : Nat) (x : X) :
    0 < CU.U ℓ x := CU.pos ℓ x

/--
  ★★带横线资本量 S_k 不变（L0，spec P3 `Φ(S_k x)=Φ(x)` 的逐量根据）：
  设资本量 `q` 同 a_k 齐次缩放（`q(S_k x) = a_k q(x)`，d_j=1），协变单位 `U_{ℓ+k}(S_k x)=a_k U_ℓ(x)`，
  则带横线量 `q̄ = q/U` 在 S_k 下不变：`q(S_k x) / U_{ℓ+k}(S_k x) = q(x) / U_ℓ(x)`（`RatioEq`）。
  证：分子分母同被 a_k 缩放，`dimensionless_invariant` 对消。
-/
theorem barred_invariant {X : Type} (CU : CovUnit X) (ℓ k : Nat) (x : X)
    (q : X → Int) (qcov : ∀ k x, q (CU.shiftX k x) = aPow CU.a k * q x) :
    RatioEq (q (CU.shiftX k x)) (CU.U (ℓ + k) (CU.shiftX k x)) (q x) (CU.U ℓ x) := by
  rw [qcov, CU.covariant]
  exact dimensionless_invariant (aPow CU.a k) (q x) (CU.U ℓ x)

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 协变资本版严格等变定理（spec §3，P3–P4）

  协变可行集 `𝒦_Θ(x) = {p : Ψ_j(x,p) ≤ b_j, j=1..n}`。若每约束齐次等变
  `Ψ_j(S_k x, S_k p) = a_k^{d_j} Ψ_j(x,p)` 且边界协变 `b_j(S_k Θ) = a_k^{d_j} b_j(Θ)`，
  则可行集等变 `𝒦_{S_kΘ}(S_k x) = S_k 𝒦_Θ(x)`（**给证明**）。约束族用 `List`（禁 Finset）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★协变资本系统 `CapitalSystem`（L0 假设包）：级别平移 S_k 对 状态 / 头寸 / 参数包 三空间的作用。
  - `a`/`ha`：基础缩放因子（a_k = a^k 非平凡）。
  - `shiftX`/`shiftP`/`shiftΘ`：S_k 对 状态 X / 头寸 P / 参数包 Θ 的作用。
  - `shiftP_surj`：S_k 在头寸空间满射（∃ S_{-k} q 的前像）——spec §3「反向用 S_{-k}」的纯 core
    建模（只需满射给出前像，即可证可行集等变的反向包含，无需显式逆映射）。
-/
structure CapitalSystem (X P Θ : Type) where
  a : Nat
  ha : 2 ≤ a
  shiftX : Nat → X → X
  shiftP : Nat → P → P
  shiftΘ : Nat → Θ → Θ
  shiftP_surj : ∀ (k : Nat) (q : P), ∃ p : P, shiftP k p = q

/-- ★系统基础缩放因子正（L0）：`0 < a`（由 `2 ≤ a`）。 -/
theorem CapitalSystem.a_pos {X P Θ : Type} (S : CapitalSystem X P Θ) : 0 < S.a := by
  have := S.ha; omega

/--
  ★协变约束 `CovConstraint`（L0 假设包，spec §3）：单条约束 + 齐次等变 + 边界协变。
  - `Ψ`：约束函数 `Ψ_j(x, p)`。 `b`：约束边界 `b_j(Θ)`。 `d`：尺度指数 `d_j`。
  - `homog`：齐次等变 `Ψ_j(S_k x, S_k p) = a_k^{d_j} Ψ_j(x,p)`（**L0 设计约束**）。
  - `bcov`：边界协变 `b_j(S_k Θ) = a_k^{d_j} b_j(Θ)`（**L0 设计约束**）。
-/
structure CovConstraint {X P Θ : Type} (S : CapitalSystem X P Θ) where
  Ψ : X → P → Int
  b : Θ → Int
  d : Nat
  homog : ∀ (k : Nat) (x : X) (p : P),
    Ψ (S.shiftX k x) (S.shiftP k p) = cFactor S.a k d * Ψ x p
  bcov : ∀ (k : Nat) (θ : Θ), b (S.shiftΘ k θ) = cFactor S.a k d * b θ

/-- ★单约束成员谓词 `holds`（L0）：`Ψ_j(x,p) ≤ b_j(Θ)`（`p ∈ 约束 j 的可行域`）。 -/
def CovConstraint.holds {X P Θ : Type} {S : CapitalSystem X P Θ}
    (c : CovConstraint S) (θ : Θ) (x : X) (p : P) : Prop :=
  c.Ψ x p ≤ c.b θ

/--
  ★★单约束 S_k 成员性双条件（L0，spec §3 证明链的逐约束核心）：
  `Ψ_j(S_k x, S_k p) ≤ b_j(S_k Θ) ⟺ Ψ_j(x,p) ≤ b_j(Θ)`。
  证：齐次等变 + 边界协变把两侧同写成 `a_k^{d_j}·(·)`，再由 `a_k^{d_j} > 0` 序乘可逆对消。
  这同时给出 spec §3 的正向（`p∈𝒦 ⟹ S_k p∈𝒦_{shift}`）与反向（S_{-k}）两个包含。
-/
theorem CovConstraint.holds_shift_iff {X P Θ : Type} {S : CapitalSystem X P Θ}
    (c : CovConstraint S) (k : Nat) (θ : Θ) (x : X) (p : P) :
    c.holds (S.shiftΘ k θ) (S.shiftX k x) (S.shiftP k p) ↔ c.holds θ x p := by
  unfold CovConstraint.holds
  rw [c.homog k x p, c.bcov k θ]
  exact Int.mul_le_mul_left (cFactor_pos S.a k c.d S.a_pos)

/--
  ★d_j=0（杠杆比）⟹ 约束纯等变（L0，spec「杠杆比 L^G=G/E 本就无量纲 d_j=0」）：
  d=0 ⟹ a_k^0 = 1 ⟹ `Ψ_j(S_k x, S_k p) = Ψ_j(x,p)`（约束值本身 S_k 不变，最强等变）。 -/
theorem CovConstraint.homog_d0 {X P Θ : Type} {S : CapitalSystem X P Θ}
    (c : CovConstraint S) (hd : c.d = 0) (k : Nat) (x : X) (p : P) :
    c.Ψ (S.shiftX k x) (S.shiftP k p) = c.Ψ x p := by
  rw [c.homog k x p, hd]
  unfold cFactor
  rw [Nat.pow_zero, Int.natCast_one, Int.one_mul]

/-- ★协变可行集 `feasibleSet` 𝒦_Θ(x)（L0）：满足全部约束的头寸（`p → Prop`，禁 Finset）。 -/
def feasibleSet {X P Θ : Type} {S : CapitalSystem X P Θ}
    (cs : List (CovConstraint S)) (θ : Θ) (x : X) : P → Prop :=
  fun p => ∀ c ∈ cs, c.holds θ x p

/-- ★S_k 像集 `shiftSet` S_k K（L0）：`{S_k p : p ∈ K}`（头寸集的平移像，定义可行集等变 RHS）。 -/
def shiftSet {X P Θ : Type} (S : CapitalSystem X P Θ) (k : Nat)
    (K : P → Prop) : P → Prop :=
  fun q => ∃ p : P, K p ∧ S.shiftP k p = q

/--
  ★可行集 S_k 成员性双条件（L0）：`p ∈ 𝒦_{S_kΘ}(S_k x) ⟺ p ∈ 𝒦_Θ(x)`（逐约束 `holds_shift_iff` 合取）。 -/
theorem feasible_shift_iff {X P Θ : Type} {S : CapitalSystem X P Θ}
    (cs : List (CovConstraint S)) (k : Nat) (θ : Θ) (x : X) (p : P) :
    feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x) (S.shiftP k p) ↔ feasibleSet cs θ x p := by
  unfold feasibleSet
  constructor
  · intro h c hc
    exact (c.holds_shift_iff k θ x p).mp (h c hc)
  · intro h c hc
    exact (c.holds_shift_iff k θ x p).mpr (h c hc)

/--
  ★★协变资本版严格等变定理（L0，spec §3 / P4 方框 `𝒦_{S_kΘ}(S_k x) = S_k 𝒦_Θ(x)`，**给证明**）：
  齐次等变 + 边界协变 ⟹ 可行集等变。
  证（spec §3 逐字）：
  - **正向（S_k K ⊆ 𝒦_{shift}）**：`q = S_k p`，`p ∈ 𝒦_Θ(x)` ⟹ `S_k p ∈ 𝒦_{S_kΘ}(S_k x)`
    （`feasible_shift_iff.mpr`）。
  - **反向（𝒦_{shift} ⊆ S_k K，spec「反向用 S_{-k}」）**：`q ∈ 𝒦_{S_kΘ}(S_k x)`，取前像
    `p`（`shiftP_surj`：`S_k p = q`），由 `feasible_shift_iff.mp` 得 `p ∈ 𝒦_Θ(x)`，故 `q ∈ S_k K`。
-/
theorem feasibleSet_equivariant {X P Θ : Type} {S : CapitalSystem X P Θ}
    (cs : List (CovConstraint S)) (k : Nat) (θ : Θ) (x : X) :
    ∀ q : P,
      feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x) q ↔ shiftSet S k (feasibleSet cs θ x) q := by
  intro q
  unfold shiftSet
  constructor
  · intro hq
    obtain ⟨p, hp⟩ := S.shiftP_surj k q
    refine ⟨p, ?_, hp⟩
    rw [← hp] at hq
    exact (feasible_shift_iff cs k θ x p).mp hq
  · intro hq
    obtain ⟨p, hpfeas, hpq⟩ := hq
    rw [← hpq]
    exact (feasible_shift_iff cs k θ x p).mpr hpfeas

/--
  ★★强形式可行集等变（L0，spec P4/P10 `S_k Θ = Θ ⟹ 同一 Θ` 强形式）：
  若参数包无量纲（`S_k Θ = Θ`），则 `𝒦_Θ(S_k x) = S_k 𝒦_Θ(x)`（同一个 Θ，spec 钦定的「去根化」强形式）。
  这是 spec「若全资本参数无量纲 ⟹ S_k Θ = Θ ⟹ 恢复强形式」的形式坐实。
-/
theorem feasibleSet_equivariant_strong {X P Θ : Type} {S : CapitalSystem X P Θ}
    (cs : List (CovConstraint S)) (k : Nat) (θ : Θ) (x : X) (hθ : S.shiftΘ k θ = θ) :
    ∀ q : P,
      feasibleSet cs θ (S.shiftX k x) q ↔ shiftSet S k (feasibleSet cs θ x) q := by
  intro q
  have h := feasibleSet_equivariant cs k θ x q
  rw [hθ] at h
  exact h

/-! ════════════════════════════════════════════════════════════════════════
  ## §4b 投影 / 订单等变（spec §3 / P4：目标函数不变 ⟹ 唯一投影等变 ⟹ 订单等变）

  spec P4：可行集等变 + 目标头寸等变 + 目标函数不变 `J_{S_k x}(S_k p)=J_x(p)` ⟹ 唯一投影
  `p*(S_k x)=S_k p*(x)` ⟹ 订单等变 `π_{S_kΘ}(S_k x)=S_k π_Θ(x)`。
  唯一性由 LexArgmin（字典序严格最小）给出——本文件取其为 §3 所述前提（`hstrict`），
  不重证 lex；证 argmin 等变（实质内容）+ 唯一性 ⟹ 投影函数等变。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★可行集上目标 J 的最小者 `IsMin`（L0）：`p ∈ K ∧ ∀ q ∈ K, J p ≤ J q`（投影 p* 的判据）。 -/
def IsMin {P : Type} (K : P → Prop) (J : P → Int) (p : P) : Prop :=
  K p ∧ ∀ q : P, K q → J p ≤ J q

/--
  ★★argmin 等变（L0，spec P4 `p*(S_k x)=S_k p*(x)` 的实质内容）：
  目标函数不变 `J(S_k x, S_k p) = J(x, p)` ⟹ 若 `p` 是 `(𝒦_Θ(x), J_x)` 的最小者，
  则 `S_k p` 是 `(𝒦_{S_kΘ}(S_k x), J_{S_k x})` 的最小者。
  证：成员性由 `feasibleSet_equivariant`；最小性由前像 `S_{-k} q`（`shiftP_surj`）+ J 不变 + 原最小性。
-/
theorem argmin_equivariant {X P Θ : Type} {S : CapitalSystem X P Θ}
    (cs : List (CovConstraint S)) (k : Nat) (θ : Θ) (x : X)
    (J : X → P → Int)
    (Jinv : ∀ (k : Nat) (x : X) (p : P), J (S.shiftX k x) (S.shiftP k p) = J x p)
    (p : P) (hp : IsMin (feasibleSet cs θ x) (J x) p) :
    IsMin (feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x)) (J (S.shiftX k x)) (S.shiftP k p) := by
  obtain ⟨hpmem, hpmin⟩ := hp
  constructor
  · -- 成员性：S_k p ∈ 𝒦_{S_kΘ}(S_k x)
    exact (feasibleSet_equivariant cs k θ x (S.shiftP k p)).mpr ⟨p, hpmem, rfl⟩
  · -- 最小性：∀ q ∈ 𝒦_{S_kΘ}(S_k x), J(S_k x)(S_k p) ≤ J(S_k x) q
    intro q hq
    obtain ⟨q', hq'⟩ := S.shiftP_surj k q
    have hq'feas : feasibleSet cs θ x q' := by
      have : feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x) (S.shiftP k q') := by
        rw [hq']; exact hq
      exact (feasible_shift_iff cs k θ x q').mp this
    calc J (S.shiftX k x) (S.shiftP k p)
        = J x p := Jinv k x p
      _ ≤ J x q' := hpmin q' hq'feas
      _ = J (S.shiftX k x) (S.shiftP k q') := (Jinv k x q').symm
      _ = J (S.shiftX k x) q := by rw [hq']

/--
  ★★投影 / 订单等变（L0，spec P4 `π_{S_kΘ}(S_k x) = S_k π_Θ(x)`）：
  目标函数不变 + 投影唯一（`hstrict`，spec §3「唯一投影」前提，由 LexArgmin 字典序严格最小给出）
  ⟹ 移位问题的最小者 `q` 等于 `S_k p`（原最小者的平移）。即 `p*(S_k x) = S_k p*(x)`。
  这接通 spec §3 链：可行集等变 + J 不变 + 唯一 ⟹ 订单等变 ⟹（无量纲 Θ）`π_Θ(S_k x)=S_k π_Θ(x)`。
-/
theorem proj_equivariant {X P Θ : Type} {S : CapitalSystem X P Θ}
    (cs : List (CovConstraint S)) (k : Nat) (θ : Θ) (x : X)
    (J : X → P → Int)
    (Jinv : ∀ (k : Nat) (x : X) (p : P), J (S.shiftX k x) (S.shiftP k p) = J x p)
    (p : P) (hp : IsMin (feasibleSet cs θ x) (J x) p)
    (q : P) (hq : IsMin (feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x)) (J (S.shiftX k x)) q)
    (hstrict : ∀ a b : P,
      IsMin (feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x)) (J (S.shiftX k x)) a →
      IsMin (feasibleSet cs (S.shiftΘ k θ) (S.shiftX k x)) (J (S.shiftX k x)) b → a = b) :
    q = S.shiftP k p :=
  hstrict q (S.shiftP k p) hq (argmin_equivariant cs k θ x J Jinv p hp)

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 三阶段资本约束的正确自相似写法（spec §5，P8–P9）

  spec §5：三阶段状态不写绝对形式 `W_t < I_0`，改归一化形式。所有量被 U_t 归一化，S_k 下
  `U_t ↦ a_k U_t`、`W,R,I,A,g,ℒ,m ↦ a_k(·)` ⟹ **带横线变量不变** ⟹ 三阶段分类保持自相似。
  纯 core 建模：共享同一单位 U 的带横线比较 `W̄ < Ī_0 ⟺ W < I_0`（U>0 对消），S_k 下分子同乘
  a_k ⟹ 比较不变（序乘可逆）。直接在原始 Int 量上证 `Φ(S_k x) = Φ(x)`。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★安全取本 `ReadyReturn`（L0，spec §5 方框 `D̄≥Ī_0-W̄ ∧ R̄-(Ī_0-W̄)≥R̄_⋆+L̄^wc`）。
    共享单位 U 对消 ⟹ 等价于原始量比较（spec 钦定的归一化形式）。 -/
@[reducible] def ReadyReturn (W I0 R Rstar Lwc D : Int) : Prop :=
  (I0 - W) ≤ D ∧ (Rstar + Lwc) ≤ R - (I0 - W)

/-- ★阶段一 `StageOne`（L0，spec §5 方框 `Φ_t=I ⟺ W̄_t<Ī_0 ∧ ¬ReadyReturn_t`）。 -/
@[reducible] def StageOne (W I0 R Rstar Lwc D : Int) : Prop :=
  W < I0 ∧ ¬ ReadyReturn W I0 R Rstar Lwc D

/-- ★阶段三可增单位 `StageThree`（L0，spec §5 方框 `Φ_t=III ⟺ W̄_t≥Ī_0 ∧ R̄_t≥R̄_⋆+L̄^wc+m̄^unit`）。 -/
@[reducible] def StageThree (W I0 R Rstar Lwc munit : Int) : Prop :=
  I0 ≤ W ∧ (Rstar + Lwc + munit) ≤ R

/-- ★三阶段相位 `Phase`（L0）：阶段一 / 阶段三可增单位 / 其它。 -/
inductive Phase where
  | stageI
  | stageIII
  | other
deriving DecidableEq, Repr

/-- ★三阶段分类器 `phaseOf` Φ_t（L0，spec §5 三阶段状态）：归一化量 ⟹ 相位。 -/
def phaseOf (W I0 R Rstar Lwc munit D : Int) : Phase :=
  if StageOne W I0 R Rstar Lwc D then Phase.stageI
  else if StageThree W I0 R Rstar Lwc munit then Phase.stageIII
  else Phase.other

/--
  ★★安全取本尺度不变（L0，spec §5 `所有带横线变量不变`）：
  全部资本量同乘 a_k（`> 0`）⟹ `ReadyReturn` 谓词不变。
  证：`a_k·I_0 - a_k·W = a_k·(I_0-W)`、`a_k·R_⋆+a_k·L^wc = a_k·(R_⋆+L^wc)` 提取公因子后序乘可逆对消。
-/
theorem ReadyReturn_scale (ak : Int) (hak : 0 < ak) (W I0 R Rstar Lwc D : Int) :
    ReadyReturn (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * D)
      ↔ ReadyReturn W I0 R Rstar Lwc D := by
  unfold ReadyReturn
  rw [show ak * I0 - ak * W = ak * (I0 - W) from (Int.mul_sub ak I0 W).symm,
      show ak * Rstar + ak * Lwc = ak * (Rstar + Lwc) from (Int.mul_add ak Rstar Lwc).symm,
      show ak * R - ak * (I0 - W) = ak * (R - (I0 - W)) from (Int.mul_sub ak R (I0 - W)).symm,
      Int.mul_le_mul_left hak, Int.mul_le_mul_left hak]

/-- ★★阶段一尺度不变（L0，spec §5 `Φ_t=I` 自相似）：`StageOne` 谓词在 S_k 下不变。 -/
theorem StageOne_scale (ak : Int) (hak : 0 < ak) (W I0 R Rstar Lwc D : Int) :
    StageOne (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * D)
      ↔ StageOne W I0 R Rstar Lwc D := by
  unfold StageOne
  rw [Int.mul_lt_mul_left hak, ReadyReturn_scale ak hak W I0 R Rstar Lwc D]

/-- ★★阶段三尺度不变（L0，spec §5 `Φ_t=III` 自相似）：`StageThree` 谓词在 S_k 下不变。 -/
theorem StageThree_scale (ak : Int) (hak : 0 < ak) (W I0 R Rstar Lwc munit : Int) :
    StageThree (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * munit)
      ↔ StageThree W I0 R Rstar Lwc munit := by
  unfold StageThree
  have hsum : ak * Rstar + ak * Lwc + ak * munit = ak * (Rstar + Lwc + munit) := by
    rw [Int.mul_add, Int.mul_add]
  rw [hsum, Int.mul_le_mul_left hak, Int.mul_le_mul_left hak]

/--
  ★★★三阶段分类 S_k 不变 `Φ(S_k x) = Φ(x)`（L0，spec P3/P9 核心方框「三阶段分类与策略保持自相似」）：
  全部资本量同乘 a_k（`> 0`）⟹ `phaseOf` 不变。这是 spec §5「带横线变量不变 ⟹ Φ(S_k x)=Φ(x)」的
  完整形式坐实（资本层尺度不变 ⟹ 三阶段层在方案 A 下全域自相似）。
-/
theorem phaseOf_scale (ak : Int) (hak : 0 < ak) (W I0 R Rstar Lwc munit D : Int) :
    phaseOf (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * munit) (ak * D)
      = phaseOf W I0 R Rstar Lwc munit D := by
  unfold phaseOf
  by_cases h1 : StageOne W I0 R Rstar Lwc D
  · have h1' : StageOne (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * D) :=
      (StageOne_scale ak hak W I0 R Rstar Lwc D).mpr h1
    rw [if_pos h1', if_pos h1]
  · have h1' : ¬ StageOne (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * D) :=
      fun h => h1 ((StageOne_scale ak hak W I0 R Rstar Lwc D).mp h)
    by_cases h3 : StageThree W I0 R Rstar Lwc munit
    · have h3' : StageThree (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * munit) :=
        (StageThree_scale ak hak W I0 R Rstar Lwc munit).mpr h3
      rw [if_neg h1', if_pos h3', if_neg h1, if_pos h3]
    · have h3' : ¬ StageThree (ak * W) (ak * I0) (ak * R) (ak * Rstar) (ak * Lwc) (ak * munit) :=
        fun h => h3 ((StageThree_scale ak hak W I0 R Rstar Lwc munit).mp h)
      rw [if_neg h1', if_neg h3', if_neg h1, if_neg h3]

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 d_j 尺度指数默认 + 杠杆比无量纲（d_j=0）+ 复用 LeverageCapital

  spec §3/§4 用 `a_k^{d_j}` 但未逐条给出 d_j。本节采**默认**（[需人工确认]）：杠杆比 d=0、
  名义额 d=1、二次风险 d=2，并证杠杆比 `L^G=G/E`（d=0）本就尺度不变（复用 LeverageCapital）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★[需人工确认] 杠杆比尺度指数 `dLeverage = 0`（无量纲，spec「L^G=G/E 本就无量纲 d_j=0」）。 -/
def dLeverage : Nat := 0

/-- ★[需人工确认] 名义额尺度指数 `dNominal = 1`（线性资本量，spec 末节默认）。 -/
def dNominal : Nat := 1

/-- ★[需人工确认] 二次风险尺度指数 `dQuadraticRisk = 2`（方差 / 二次型，spec 末节默认）。 -/
def dQuadraticRisk : Nat := 2

/-- ★d=0 因子 = 1（L0）：`a_k^0 = 1`（杠杆比约束值 S_k 不变）。 -/
theorem cFactor_dLeverage (a k : Nat) : cFactor a k dLeverage = 1 := by
  unfold cFactor dLeverage
  rw [Nat.pow_zero, Int.natCast_one]

/-- ★d=1 因子 = a_k（L0）：`a_k^1 = a_k`（名义额线性缩放）。 -/
theorem cFactor_dNominal (a k : Nat) : cFactor a k dNominal = aPow a k := by
  unfold cFactor dNominal aPow
  rw [Nat.pow_one]

/-- ★d=2 因子 = a_k·a_k（L0）：`a_k^2 = a_k·a_k`（二次风险二次缩放）。 -/
theorem cFactor_dQuadraticRisk (a k : Nat) :
    cFactor a k dQuadraticRisk = aPow a k * aPow a k := by
  unfold cFactor dQuadraticRisk aPow
  rw [Nat.pow_two, Int.natCast_mul]

/--
  ★★杠杆比 S_k 不变（L0，spec「杠杆比 L^G=G/E 本就无量纲 d_j=0」）：
  毛敞口 G 与权益 E 同乘 a_k ⟹ 比值 `G/E` 不变（`RatioEq`）。d_j=0 = 无量纲 = 天然满足方案 A。 -/
theorem leverage_ratio_invariant (ak G E : Int) : RatioEq (ak * G) (ak * E) G E :=
  dimensionless_invariant ak G E

/-- ★按 Nat 因子缩放单声部名义大小（L0，S_k 下 |n_v| ↦ a_k|n_v|）：复用 LeverageCapital.VoicePosition。 -/
def VoicePosition.scaleMag (m : Nat) (p : VoicePosition) : VoicePosition :=
  { side := p.side, notionalMag := m * p.notionalMag }

/--
  ★★毛敞口齐次缩放（L0，复用 LeverageCapital.grossNotional）：所有声部名义同乘 a_k ⟹ `G_t ↦ a_k G_t`
  （`grossNotional (scale ps) = a_k · grossNotional ps`）。这坐实毛敞口是 d_j=1 线性量，配合权益 E
  同缩放 ⟹ 杠杆比 G/E（d_j=0）不变（`leverage_ratio_invariant`）。
-/
theorem grossNotional_scale (m : Nat) (ps : List VoicePosition) :
    grossNotional (ps.map (VoicePosition.scaleMag m)) = m * grossNotional ps := by
  unfold grossNotional
  induction ps with
  | nil => simp
  | cons p ps' ih =>
      simp only [List.map_cons, List.sum_cons]
      show m * p.notionalMag + ((ps'.map (VoicePosition.scaleMag m)).map
            VoicePosition.notionalMag).sum
          = m * (p.notionalMag + (ps'.map VoicePosition.notionalMag).sum)
      rw [ih, Nat.mul_add]

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 诚实标签（formalization-validity-domain gatekeeper，L0 声明）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★协变资本标签 `CapitalTag`（gatekeeper，诚实分层）：
  CovariantUnitNormalization（协变单位归一化）+ DimensionlessRatio（无量纲比例）+
  EquivarianceL0（等变是 L0 结构定理）+ ThetaCapitalParametric（资本参数是 Θ 层，非缠论可导）。
  ★**没有** `EmpiricalallyValidated` 构造子——类型层拒绝把 L0 结构定理标为经验有效（L2）。
-/
inductive CapitalTag where
  | CovariantUnitNormalization
  | DimensionlessRatio
  | EquivarianceL0
  | ThetaCapitalParametric
deriving DecidableEq, Repr

/-- ★协变资本子类（gatekeeper）：CovariantDimensionlessCapital（方案 A 唯一子类）。 -/
inductive CapitalSubkind where
  | CovariantDimensionlessCapital
deriving DecidableEq, Repr

/-- ★协变资本诚实标签包（L0 声明）。 -/
def capitalLabels : List CapitalTag × CapitalSubkind :=
  ([CapitalTag.CovariantUnitNormalization, CapitalTag.DimensionlessRatio,
    CapitalTag.EquivarianceL0, CapitalTag.ThetaCapitalParametric],
   CapitalSubkind.CovariantDimensionlessCapital)

/-- ★禁标经验有效（L0，gatekeeper 见证）：协变资本子类必是 CovariantDimensionlessCapital（纯 L0）。 -/
theorem capital_subkind_unique (k : CapitalSubkind) :
    k = CapitalSubkind.CovariantDimensionlessCapital := by
  cases k; rfl

end NewChanlun.Origin.CovariantCapital
