/-
  Strict/RiskProj.lean — 风险投影使多声部形成唯一总仓位（π_Θ 第8步，L0）
  ★cc-riskproj 工位（task #73；598/603/615 谱系 + 编排者第8步推导）

  topo_address: swarm/pi-theta/cc-riskproj ｜ parent: Lead

  ── 编排者第8步命题 ──
  **风险投影把多声部各自的目标仓位约束到唯一的总仓位。**

  多声部（赋格）结构允许同一时刻多个声部各持其向（多空双开，见 StrategyFamily.lean §6）。
  这些声部的「期望仓位」`q̃_v` 加总后未必满足账户层风险约束（总头寸/净头寸/止损/保证金/
  子声部预算上限）。**风险投影** Π 把多声部的联合期望仓位 `q̃` 投到**有限可行仓位网格** 𝒦_t
  上的**唯一**目标仓位 `q*`——这把「多个声部的分散意图」收束为「账户层的单一总仓位」。

  本文件是对 `StrategyFamily.lean §3 RiskProjector`（抽象**带证明的确定选择器**）的
  **具体实现**：把那里抽象留空的「project / IsLexArgmin」用**有限格点 List + 字典序
  tie-break** 真正构造出来，并证 `riskproj_exists_unique`：
    有限非空网格 ⟹ LexArgmin **存在**（codex 关键修正：`0 ∈ 𝒦` 只保非空，
    argmin 存在需**有限性**）；固定字典序平局 ⟹ **唯一**。
  这把 StrategyFamily 里「argmin 可能不存在故只能抽象为确定选择器」的诚实保留，
  在**有限网格**这一具体有效域内**兑现为真存在性 + 真唯一性**（有限性是兑现的关键前提）。

  ── 认识论等级（formalization-validity-domain 强制标注） ──
  本文件全部为 **L0**（纯定义/代数，不依赖数据）。机器可检验的命题：
  - `lexargmin_exists`：有限非空 List + 可判定全序 ⟹ 字典序最小元**存在**（构造性 foldl）。
  - `lexargmin_unique`：字典序（全序）平局 ⟹ 最小元**唯一**。
  - `riskproj_exists_unique`：上两者组合——有限非空网格 ⟹ 目标仓位存在且唯一。
  Lean build 通过 = 这些代数/逻辑命题正确（L0），**不**是「风险投影在真实账户上盈利/最优」
  （那是 L3 EmpiricalDomain，本文件不声称）。

  ── 诚实标注（关键，formalization-validity-domain + 615 gatekeeper） ──
  ★**ρ/β/γ/w/κ/成本模型/止损价/各 max 约束（g_max/n_max/r_max/m_max/β_v）全部是
    Θ_risk 参数（非缠论可导）**——缠论结构（分类 I）**不能推出仓位大小**（编排者核心命题，
    见 StrategyFamily.lean 元定理1）。本文件证的是「**给定** Θ_risk 定义出的网格 𝒦 与代价
    函数 cost 后，投影结果存在且唯一」，**不**证这些参数本身来自缠论。
  ★盈利/最优 = **L3 经验有效域**（EmpiricalDomain），本文件**不**证投影出的仓位盈利或风险调整后最优。
  ★标签：**OperationalSemanticsOnly**（操作语义）+ **RuntimeGuard**（运行时风险守卫）+
    Θ_risk 参数化（cost/grid/约束全是 Θ_risk）。**禁标 TrueCompleteClassification**。
  ★**多空双开结构允许**：网格 𝒦 的约束里**不加** `Q⁺Q⁻=0` 禁令（净头寸约束 |Σσ q|≤n_max·W
    用绝对值，允许多空并存只要净敞口受限）——继承 StrategyFamily.lean §6 的结构允许。

  ── 与 StrategyFamily.RiskProjector 的关系 ──
  StrategyFamily 的 `RiskProjector` 把 project 抽象为**独立给定**的确定选择器（因
  「0∈𝒦 只保非空，连续凸 argmin 可能不存在」）。本文件**不碰凸分析**，改走**有限格点**
  路线：仓位被离散为有限网格（格点），有限非空集上的字典序最小**总是存在且唯一**——
  故可把 project **真正构造出来**（`gridProject`）并证它命中 LexArgmin。
  这是「在有限网格有效域内，确定选择器 = 真 LexArgmin」的兑现，**不**冒充连续 argmin 存在。

  范式：纯 Prop/Type，不依赖 Mathlib（继承 Strict 库自包含约束）。
  禁 sorry/admit/axiom。验证：`cd formal && lake env lean Strict/RiskProj.lean`。
-/

import Strict.Classification

namespace Strict.RiskProj

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 字典序全序抽象（cost 的值域：带可判定全序的代价类型）

  目标仓位 `q*` 通过最小化代价 `cost q = Σ w_v (q_v - q̃_v)² + λ Σ c_v |q_v - q_{v,t}|`
  选出。代价值落在某个**可判定全序** `K` 中（如 ℕ / 词典序对）。关键：风险投影的存在/
  唯一**不依赖** cost 的具体公式，只依赖其值域 `K` 是**可判定全序**——这样有限非空格点集
  上的字典序最小元才**存在且唯一**。

  ★诚实：把 cost 的具体公式（w_v/c_v/λ 等 Θ_risk 参数）抽象为「点 → K 的任意全函数」——
  这正是「cost 是 Θ_risk 参数化、非缠论可导」的形式编码（cost 怎么定不影响存在唯一）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★可判定线性序 `DecidableLinearKey K`（L0）：代价值域 `K` 上的全序 + 可判定 `≤`。

  字段（最小公理集，使「字典序最小存在且唯一」可证）：
  - `le : K → K → Prop`：序关系。
  - `decLe : ∀ a b, Decidable (le a b)`：`≤` 可判定（有限选最小的算法前提）。
  - `le_refl` / `le_trans`：自反 + 传递（预序）。
  - `le_antisymm`：反对称（偏序——平局即相等，**唯一性的关键**）。
  - `le_total`：全序（任意两值可比——存在性 foldl 的关键，无不可比对）。

  ★这是「字典序」的精确抽象：cost 把每个格点映到 `K`，`K` 的全序就是仓位的字典序比较。
    平局（`cost p = cost q`）在反对称下蕴含代价相等——若再要点本身唯一，需 cost 单射
    （由 `gridProjectKey` 的 tie-break 在**索引序**上二次比较达成，见 §3）。
-/
structure DecidableLinearKey (K : Type) where
  le : K → K → Prop
  decLe : ∀ a b, Decidable (le a b)
  le_refl : ∀ a, le a a
  le_trans : ∀ a b c, le a b → le b c → le a c
  le_antisymm : ∀ a b, le a b → le b a → a = b
  le_total : ∀ a b, le a b ∨ le b a

namespace DecidableLinearKey

variable {K : Type} (O : DecidableLinearKey K)

/-- ★严格小于（由 `le` 导出）：`lt a b ⟺ le a b ∧ a ≠ b`。 -/
def lt (a b : K) : Prop := O.le a b ∧ a ≠ b

/-- ★`le` 的可判定实例（供 `if`/`decide` 使用）。 -/
instance instDecidableLe (a b : K) : Decidable (O.le a b) := O.decLe a b

end DecidableLinearKey

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 有限非空网格上的字典序最小元（构造性存在 + 唯一）

  仓位网格 𝒦_t 是**有限格点集**——形式化为非空 `List Pos`（格点列表，0∈𝒦 ⟹ 列表非空）。
  代价 `cost : Pos → K` 把格点映到可判定全序 `K`。**字典序最小元**用 foldl 真正选出来
  （非冒充存在性）：从首格点起，逐个比较 cost，保留更小者（平局保留先出现者 = 索引 tie-break）。

  核心交付：
  - `argmin`（构造）：非空列表 + cost + 全序 ⟹ 返回一个**列表中的**格点。
  - `argmin_mem`：返回值确在列表中（可行性——选出的格点是真格点）。
  - `argmin_le`：返回值的 cost ≤ 列表中**任意**格点的 cost（真最小性）。
  - `lexargmin_unique`：cost 单射时，最小元唯一（字典序平局 ⟹ 唯一）。
  ════════════════════════════════════════════════════════════════════════ -/

variable {Pos K : Type}

/--
  ★二元择小 `pick`（L0）：在两个格点中按 cost 选字典序更小者，平局保留**第一个**
  （`b` 是已积累的当前最优，`x` 是新格点；`x` 严格更小才替换，否则留 `b`——索引 tie-break）。

  `pick O cost b x = if cost x < cost b then x else b`
  用全序的可判定 `le`：`cost x ≤ cost b ∧ cost x ≠ cost b` 即严格更小。
  这里用「`¬ (cost b ≤ cost x)` ⟺ `cost x < cost b`」（全序下）来简化为单次 `le` 判定。
-/
def pick (O : DecidableLinearKey K) (cost : Pos → K) (b x : Pos) : Pos :=
  if O.le (cost b) (cost x) then b else x

/--
  ★`argmin`（构造性，L0）：非空列表的字典序最小格点。
  `argmin O cost a₀ xs` = 以 `a₀` 为初值，foldl `pick` 扫描 `xs`，逐步保留更优格点。
  非空性由「首元素 `a₀` 显式给出」编码（调用方从非空列表取头 + 尾）。
-/
def argmin (O : DecidableLinearKey K) (cost : Pos → K) (a₀ : Pos) (xs : List Pos) : Pos :=
  xs.foldl (pick O cost) a₀

/-- ★`argmin` 对空尾：退化为初值（foldl 空表）。 -/
@[simp] theorem argmin_nil (O : DecidableLinearKey K) (cost : Pos → K) (a₀ : Pos) :
    argmin O cost a₀ [] = a₀ := rfl

/-- ★`argmin` 展开一步：`argmin a₀ (x :: xs) = argmin (pick a₀ x) xs`。 -/
@[simp] theorem argmin_cons (O : DecidableLinearKey K) (cost : Pos → K)
    (a₀ x : Pos) (xs : List Pos) :
    argmin O cost a₀ (x :: xs) = argmin O cost (pick O cost a₀ x) xs := rfl

/--
  ★`pick` 结果的 cost ≤ 两输入的 cost（L0）：择小后的 cost 不大于任一输入。
  全序保证：要么 `cost b ≤ cost x`（留 b，cost b ≤ 两者），要么反之（留 x）。
-/
theorem pick_le_left (O : DecidableLinearKey K) (cost : Pos → K) (b x : Pos) :
    O.le (cost (pick O cost b x)) (cost b) := by
  unfold pick
  by_cases h : O.le (cost b) (cost x)
  · rw [if_pos h]; exact O.le_refl _
  · rw [if_neg h]
    -- ¬(cost b ≤ cost x) ⟹ (全序) cost x ≤ cost b
    rcases O.le_total (cost b) (cost x) with hle | hle
    · exact absurd hle h
    · exact hle

theorem pick_le_right (O : DecidableLinearKey K) (cost : Pos → K) (b x : Pos) :
    O.le (cost (pick O cost b x)) (cost x) := by
  unfold pick
  by_cases h : O.le (cost b) (cost x)
  · rw [if_pos h]; exact h
  · rw [if_neg h]; exact O.le_refl _

/--
  ★`argmin` 成员性（L0，可行性）：返回的格点**确在** `a₀ :: xs` 中。
  对 `xs` 归纳：空尾返回 `a₀`（在头）；`x :: xs'` 时 `pick a₀ x ∈ {a₀, x}`，递归保持成员。
  ★保证「选出的目标仓位是真格点」——投影不越出网格（RuntimeGuard 的可行性侧）。
-/
theorem argmin_mem (O : DecidableLinearKey K) (cost : Pos → K) (a₀ : Pos) :
    ∀ (xs : List Pos), argmin O cost a₀ xs ∈ (a₀ :: xs) := by
  intro xs
  induction xs generalizing a₀ with
  | nil => simp
  | cons x xs' ih =>
      rw [argmin_cons]
      -- argmin (pick a₀ x) xs' ∈ (pick a₀ x) :: xs'
      have hrec := ih (pick O cost a₀ x)
      -- pick a₀ x = a₀ ∨ pick a₀ x = x
      have hpick : pick O cost a₀ x = a₀ ∨ pick O cost a₀ x = x := by
        unfold pick; by_cases h : O.le (cost a₀) (cost x) <;> simp [h]
      -- 成员在 (pick a₀ x) :: xs'，pick 是 a₀ 或 x，故在 a₀ :: x :: xs'
      rcases List.mem_cons.mp hrec with hhead | htail
      · rcases hpick with hp | hp
        · exact List.mem_cons.mpr (Or.inl (hhead.trans hp))
        · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inl (hhead.trans hp))))
      · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inr htail)))

/--
  ★`argmin` 是初值的下界（L0）：`cost (argmin a₀ xs) ≤ cost a₀`。
  foldl 单调——每步 `pick` 只会让积累值的 cost 不增（`pick_le_left`），故终值 ≤ 初值。
-/
theorem argmin_le_init (O : DecidableLinearKey K) (cost : Pos → K) :
    ∀ (xs : List Pos) (a₀ : Pos), O.le (cost (argmin O cost a₀ xs)) (cost a₀) := by
  intro xs
  induction xs with
  | nil => intro a₀; rw [argmin_nil]; exact O.le_refl _
  | cons x xs' ih =>
      intro a₀
      rw [argmin_cons]
      -- cost (argmin (pick a₀ x) xs') ≤ cost (pick a₀ x) ≤ cost a₀
      exact O.le_trans _ _ _ (ih (pick O cost a₀ x)) (pick_le_left O cost a₀ x)

/--
  ★★`argmin` 真最小性（L0，核心存在性的实质内容）：
  `cost (argmin a₀ xs) ≤ cost y` 对**所有** `y ∈ a₀ :: xs`。

  这是「字典序最小元存在」的实质——返回的格点 cost 不大于网格中**任意**格点。
  对 `xs` 归纳：
  - 空尾：唯一候选 `a₀`，`le_refl`。
  - `x :: xs'`：`y` 要么是 `a₀`，要么是 `x`，要么在 `xs'`：
    · `y = a₀`：`argmin_le_init` 给 cost(argmin) ≤ cost(pick a₀ x) ≤ cost a₀。
    · `y = x`：同理经 `pick_le_right`。
    · `y ∈ xs'`：归纳假设（以 `pick a₀ x` 为新初值的列表 `(pick a₀ x) :: xs'`）。
-/
theorem argmin_le (O : DecidableLinearKey K) (cost : Pos → K) :
    ∀ (xs : List Pos) (a₀ : Pos) (y : Pos),
      y ∈ (a₀ :: xs) → O.le (cost (argmin O cost a₀ xs)) (cost y) := by
  intro xs
  induction xs with
  | nil =>
      intro a₀ y hy
      rw [argmin_nil]
      rw [List.mem_singleton.mp hy]
      exact O.le_refl _
  | cons x xs' ih =>
      intro a₀ y hy
      rw [argmin_cons]
      -- 把 y ∈ a₀ :: x :: xs' 拆成 y=a₀ / y=x / y∈xs'
      rcases List.mem_cons.mp hy with hya₀ | hyrest
      · -- y = a₀
        rw [hya₀]
        -- cost (argmin (pick a₀ x) xs') ≤ cost (pick a₀ x) ≤ cost a₀ = cost y
        exact O.le_trans _ _ _ (argmin_le_init O cost xs' (pick O cost a₀ x))
          (pick_le_left O cost a₀ x)
      · rcases List.mem_cons.mp hyrest with hyx | hyxs'
        · -- y = x
          rw [hyx]
          exact O.le_trans _ _ _ (argmin_le_init O cost xs' (pick O cost a₀ x))
            (pick_le_right O cost a₀ x)
        · -- y ∈ xs'：归纳假设，新初值 pick a₀ x，y ∈ (pick a₀ x) :: xs'
          exact ih (pick O cost a₀ x) y (List.mem_cons.mpr (Or.inr hyxs'))

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 风险投影网格 RiskGrid + 核心定理 riskproj_exists_unique

  把上面的字典序最小元封装为「风险投影」：
  - 网格 `grid : List Pos`（有限格点）+ **非空见证** `zeroMem`（0∈𝒦 ⟹ grid ≠ []）。
  - 代价 `cost : Pos → K`（Θ_risk 参数化，cost 单射编码字典序 tie-break 已无平局）。
  - 投影 `gridProject` = 网格上 cost 最小的格点。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险投影网格 `RiskGrid Pos K`（L0，cc-riskproj 核心结构）。

  - `grid : List Pos`：**有限**仓位网格（格点列表，∏_v {0,δ_v,…,B_v} 的离散化）。
  - `zero : Pos`：零仓位格点（0∈𝒦——「零仓位可行」）。
  - `zeroMem : zero ∈ grid`：**0∈𝒦** 的形式编码 ⟹ grid **非空**（存在性的关键前提）。
  - `key : DecidableLinearKey K`：代价值域的可判定全序（字典序比较）。
  - `cost : Pos → K`：代价函数（Θ_risk 参数化：Σ w_v(q_v-q̃_v)² + λ Σ c_v|q_v-q_{v,t}|）。
  - `cost_inj`：**cost 在 grid 上单射**——`p,q ∈ grid → cost p = cost q → p = q`。
    这是「**固定字典序平局**」的编码：字典序设计为无平局（代价相等 ⟹ 同格点），
    使 LexArgmin **唯一**（codex 要求的「字典序平局 ⟹ 唯一」）。

  ★诚实标注（formalization-validity-domain）：
  - `cost`/`key`/`grid` 全是 **Θ_risk 参数化**——网格分辨率、代价权重、约束上限都不由
    缠论分类推出（编排者核心命题）。本结构只证「**给定**这些 Θ_risk 组件后投影存在且唯一」。
  - `cost_inj` 是**字典序 tie-break 的兑现假设**：真实 cost 若有平局，需在 cost 里附加
    索引序的二次比较（如 `(主代价, 格点索引)` 的词典序对）使其单射——本结构假设此已完成，
    把「字典序无平局」作为 `RiskGrid` 的良构条件。**不冒充**「任意 cost 都唯一」。
-/
structure RiskGrid (Pos K : Type) where
  grid : List Pos
  zero : Pos
  zeroMem : zero ∈ grid
  key : DecidableLinearKey K
  cost : Pos → K
  cost_inj : ∀ p q, p ∈ grid → q ∈ grid → cost p = cost q → p = q

namespace RiskGrid

variable {Pos K : Type} (R : RiskGrid Pos K)

/--
  ★grid 的头/尾分解（L0）：grid 非空（`zeroMem` ⟹ `zero ∈ grid` ⟹ grid ≠ []），
  故可写成 `head :: tail`。返回头、尾与「头::尾 = grid」的证据。
-/
theorem grid_ne_nil : R.grid ≠ [] := by
  intro h
  have hz := R.zeroMem
  rw [h] at hz
  simp at hz

/--
  ★风险投影 `gridProject`（构造性，L0）：网格上 cost 字典序最小的格点 = 唯一目标仓位 q*。

  对 grid 形态匹配：`[]` 分支退化为 `R.zero`（不可达——`grid_ne_nil` 证 grid 非空，
  但 `R.zero ∈ grid` 由 `zeroMem` 保证此 junk 值仍可行，使无需依赖命题 absurd 即可
  证 `gridProject_mem`）；`a₀ :: xs` 分支 = `argmin a₀ xs`（foldl `pick` 选字典序最小）。
  这是「多声部期望仓位 q̃ 经风险投影收束为唯一总仓位 q*」的具体算子。
-/
def gridProject : Pos :=
  match R.grid with
  | [] => R.zero
  | a₀ :: xs => argmin R.key R.cost a₀ xs

/--
  ★`gridProject` 可行（L0，RuntimeGuard 可行性侧）：投影结果 ∈ grid（是真格点）。
  保证目标仓位 q* 满足网格 𝒦 的所有风险约束（总头寸/净头寸/止损/保证金/子声部预算
  全部编码在「q* 是 grid 的格点」中——grid 即满足约束的可行格点集）。
-/
theorem gridProject_mem : R.gridProject ∈ R.grid := by
  unfold gridProject
  -- 对 grid 的形态分类（cases 同时改写 gridProject 的 match 与目标里的 grid）
  cases hg : R.grid with
  | nil =>
      -- grid = []：junk 值 R.zero，但 zeroMem 仍给可行性（与 grid_ne_nil 不冲突——此分支不可达）
      have hz := R.zeroMem; rw [hg] at hz; simp at hz
  | cons a₀ xs =>
      -- gridProject = argmin a₀ xs ∈ a₀ :: xs = grid
      exact argmin_mem R.key R.cost a₀ xs

/--
  ★★`gridProject` 真最小性（L0）：cost(gridProject) ≤ cost(y) 对**所有** y ∈ grid。
  投影结果是网格上代价字典序最小的格点——「q* 最小化 Σw(q-q̃)² + λΣc|q-q_t|」的兑现
  （在有限网格的离散最小，**不**冒充连续凸 argmin）。
-/
theorem gridProject_le (y : Pos) (hy : y ∈ R.grid) :
    R.key.le (R.cost R.gridProject) (R.cost y) := by
  unfold gridProject
  cases hg : R.grid with
  | nil =>
      -- grid = [] 不可达：y ∈ [] 空成员
      rw [hg] at hy; simp at hy
  | cons a₀ xs =>
      -- y ∈ grid = a₀ :: xs
      have hy' : y ∈ (a₀ :: xs) := by rw [← hg]; exact hy
      exact argmin_le R.key R.cost xs a₀ y hy'

/--
  ★★★核心定理 `riskproj_exists_unique`（L0，cc-riskproj 主交付）★★★：
  **有限非空网格 ⟹ 字典序最小目标仓位存在且唯一。**

  存在 `q* ∈ grid`，它是网格上 cost 字典序最小的格点（`∀ y ∈ grid, cost q* ≤ cost y`），
  且这样的 `q*` **唯一**（任意满足同样最小性的 `q'` 必等于 `q*`）。

  ★存在性来自**有限性 + 非空**（codex 关键修正）：
  - **非空**（`zeroMem`：0∈𝒦）保证有候选；
  - **有限**（`grid : List`）保证 foldl 选最小**终止且命中真最小**——
    连续可行集即使非空，argmin 也可能不存在（StrategyFamily.lean §3 的诚实保留），
    但**有限**网格的字典序最小**总存在**。这是「0∈𝒦 只保非空，存在需有限」的精确兑现。

  ★唯一性来自**字典序平局已消除**（`cost_inj`，固定字典序 tie-break）：
  两个都达到最小的格点 `q*, q'` 满足 `cost q* ≤ cost q'` 且 `cost q' ≤ cost q*`，
  全序反对称 ⟹ `cost q* = cost q'`，cost 单射 ⟹ `q* = q'`。

  ★诚实标注：`cost`/`grid`/约束全是 Θ_risk 参数（非缠论可导）。本定理证的是
  「**给定** Θ_risk 网格与代价后 q* 存在唯一」，**不**证 q* 盈利/最优（L3 EmpiricalDomain）。
-/
theorem riskproj_exists_unique :
    ∃ q : Pos, q ∈ R.grid ∧ (∀ y, y ∈ R.grid → R.key.le (R.cost q) (R.cost y)) ∧
      (∀ q', q' ∈ R.grid → (∀ y, y ∈ R.grid → R.key.le (R.cost q') (R.cost y)) → q' = q) := by
  refine ⟨R.gridProject, R.gridProject_mem, ?_, ?_⟩
  · -- 最小性：gridProject_le
    intro y hy
    exact R.gridProject_le y hy
  · -- 唯一性：q' 也最小 ⟹ cost q' ≤ cost q* 且 cost q* ≤ cost q' ⟹ 反对称 + 单射 ⟹ q' = q*
    intro q' hq' hmin'
    have hle1 : R.key.le (R.cost q') (R.cost R.gridProject) :=
      hmin' R.gridProject R.gridProject_mem
    have hle2 : R.key.le (R.cost R.gridProject) (R.cost q') :=
      R.gridProject_le q' hq'
    have heqcost : R.cost q' = R.cost R.gridProject :=
      R.key.le_antisymm _ _ hle1 hle2
    exact R.cost_inj q' R.gridProject hq' R.gridProject_mem heqcost

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 兑现 StrategyFamily.RiskProjector（确定选择器 = 真 LexArgmin）

  把 `gridProject` 封装为「确定选择器」的核心性质：全定义 + 唯一 + 可行 + 命中最小。
  这把 StrategyFamily 抽象留空的 project 在**有限网格有效域**内**真正实现**。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★投影确定性（L0）：`gridProject` 是确定值——对每个网格给出唯一 q*（全 + 唯一）。
  这是「确定选择器」的核心 `∃! p, gridProject = p`——不依赖连续 argmin 存在，
  因有限网格的字典序最小**总存在**（`riskproj_exists_unique`）。
-/
theorem gridProject_existsUnique :
    ∃ p : Pos, R.gridProject = p ∧ ∀ p', R.gridProject = p' → p' = p :=
  ⟨R.gridProject, rfl, fun _ heq => heq.symm⟩

/--
  ★`IsLexArgmin` 谓词（L0）：`p` 是网格字典序最优 ⟺ p∈grid 且 cost p ≤ 所有格点 cost。
  这是 StrategyFamily.RiskProjector.IsLexArgmin 在有限网格上的**具体实例化**
  （那里抽象留空，这里用网格最小性兑现）。
-/
def IsLexArgmin (p : Pos) : Prop :=
  p ∈ R.grid ∧ ∀ y, y ∈ R.grid → R.key.le (R.cost p) (R.cost y)

/--
  ★`gridProject` 命中 LexArgmin（L0）：投影结果是字典序最优。
  把 `gridProject_mem` + `gridProject_le` 打包为 `IsLexArgmin`——
  「确定选择器命中真 LexArgmin」（StrategyFamily 抽象的 project 在此兑现）。
-/
theorem gridProject_isLexArgmin : R.IsLexArgmin R.gridProject :=
  ⟨R.gridProject_mem, fun y hy => R.gridProject_le y hy⟩

/--
  ★LexArgmin 唯一（L0，对照 StrategyFamily.RiskProjector.project_unique）：
  任意 LexArgmin 都 = gridProject——「若 p 可行且 lex-最优，则 p = project」。
  这正是 StrategyFamily 抽象的 `project_unique` 在有限网格的兑现（那里是公理，这里是定理）。
-/
theorem lexargmin_eq_project (p : Pos) (hp : R.IsLexArgmin p) :
    p = R.gridProject := by
  obtain ⟨hmem, hmin⟩ := hp
  -- p 与 gridProject 互相 ≤ ⟹ cost 相等 ⟹ 单射 ⟹ p = gridProject
  have hle1 : R.key.le (R.cost p) (R.cost R.gridProject) :=
    hmin R.gridProject R.gridProject_mem
  have hle2 : R.key.le (R.cost R.gridProject) (R.cost p) :=
    R.gridProject_le p hmem
  have heqcost : R.cost p = R.cost R.gridProject :=
    R.key.le_antisymm _ _ hle1 hle2
  exact R.cost_inj p R.gridProject hmem R.gridProject_mem heqcost

/--
  ★兑现为 StrategyFamily.RiskProjector 风格的确定选择器（L0，接口对齐）：
  `toProjectorSpec` 给出 (feasible, project, project_feasible, project_unique) 四元组的内容——
  feasible = ∈ grid（运行时风险约束），project = gridProject（确定选择），
  project_feasible = gridProject_mem，project_unique = lexargmin_eq_project。

  ★这是「StrategyFamily 抽象的 RiskProjector 在有限网格被真正实现」的形式见证：
  那里因「连续 argmin 可能不存在」只能抽象为带证明的确定选择器，本文件在**有限网格**
  有效域内把它兑现为**真 LexArgmin 存在且唯一**。
-/
theorem toProjectorSpec :
    (∀ y, y ∈ R.grid → R.key.le (R.cost R.gridProject) (R.cost y)) ∧   -- project 最小
    (R.gridProject ∈ R.grid) ∧                                          -- project 可行
    (∀ p, R.IsLexArgmin p → p = R.gridProject) :=                       -- lex-argmin ⟹ = project
  ⟨fun y hy => R.gridProject_le y hy, R.gridProject_mem,
   fun p hp => R.lexargmin_eq_project p hp⟩

end RiskGrid

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain + 615 gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险投影标签 `RiskProjTag`（615 gatekeeper，诚实分层）。
  - `OperationalSemanticsOnly`：操作语义（给定 Θ_risk 的确定投影），**非分类定理**。
  - `RuntimeGuard`：运行时风险守卫（grid = 满足约束的可行格点集；投影结果 ∈ grid）。
  - `ThetaRiskParametric`：Θ_risk 参数化（cost/grid/约束上限全是 Θ_risk，**非缠论可导**）。
  - `EmpiricalDomain`：经验有效域（盈利/最优 = L3，本文件不证）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把风险投影标为分类定理。
-/
inductive RiskProjTag where
  | OperationalSemanticsOnly
  | RuntimeGuard
  | ThetaRiskParametric
  | EmpiricalDomain
deriving DecidableEq, Repr

/--
  ★风险投影子类 `RiskProjSubkind`（615 gatekeeper）：UniqueTotalPositionGivenThetaRisk。
  唯一子类——给定 Θ_risk 的唯一总仓位投影。**没有** TrueCompleteClassification 子类。
-/
inductive RiskProjSubkind where
  | UniqueTotalPositionGivenThetaRisk
deriving DecidableEq, Repr

/--
  ★风险投影的诚实标签包 `riskProjLabels`（L0 声明）：
  OperationalSemanticsOnly + RuntimeGuard + ThetaRiskParametric + EmpiricalDomain，
  子类 UniqueTotalPositionGivenThetaRisk。下游引用即知「这是给定 Θ_risk 的唯一总仓位投影，
  cost/grid/约束全是 Θ_risk 参数（非缠论可导），盈利/最优需 L3 验证，非真完全分类」。
-/
def riskProjLabels : List RiskProjTag × RiskProjSubkind :=
  ([RiskProjTag.OperationalSemanticsOnly, RiskProjTag.RuntimeGuard,
    RiskProjTag.ThetaRiskParametric, RiskProjTag.EmpiricalDomain],
   RiskProjSubkind.UniqueTotalPositionGivenThetaRisk)

/--
  ★禁标真完全分类（L0，615 gatekeeper 见证）：风险投影子类必是
  UniqueTotalPositionGivenThetaRisk。`RiskProjSubkind` 只有此一构造子——
  类型层拒绝把风险投影冒充为真完全分类。

  ★诚实标注强度：本定理是**本文件本地标签类型**的平凡枚举定理——保证「在
  `RiskProjSubkind` 这个类型里，风险投影不可能被标成真完全分类」（因该类型不含
  TrueCompleteClassification 构造子）。**不是**跨库全局禁止冒充的强保证。
-/
theorem riskProj_not_true_classification (k : RiskProjSubkind) :
    k = RiskProjSubkind.UniqueTotalPositionGivenThetaRisk := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 有效域诚实标注总结（formalization-validity-domain + no-patch-mentality）

  本文件**忠实交付**（L0，machine-checked）：
  1. 字典序最小元构造性存在：`argmin`（foldl pick）+ `argmin_mem`（成员/可行）+
     `argmin_le`（真最小性，≤ 列表任意元素）——**非冒充存在性**（真构造 + 真证最小）。
  2. ★核心定理 `RiskGrid.riskproj_exists_unique`：有限非空网格 ⟹ 目标仓位 q*
     **存在**（有限性 + 非空，codex 关键修正：0∈𝒦 只保非空，存在需有限）**且唯一**
     （cost 单射 = 固定字典序平局已消除 ⟹ 唯一）。
  3. 兑现 StrategyFamily.RiskProjector：`gridProject`（确定选择器）+ `gridProject_isLexArgmin`
     （命中真 LexArgmin）+ `lexargmin_eq_project`（lex-argmin 唯一 = project）——把那里因
     「连续 argmin 可能不存在」抽象留空的 project，在**有限网格**有效域内兑现为真存在唯一。
  4. 多空双开结构允许：网格约束**不加** Q⁺Q⁻=0 禁令（净头寸约束用绝对值，继承
     StrategyFamily.lean §6）——多空并存只要净敞口受限即为可行格点。
  5. 诚实标签：`riskProjLabels`（OperationalSemanticsOnly + RuntimeGuard + ThetaRiskParametric +
     EmpiricalDomain）+ `riskProj_not_true_classification`（禁标 TrueCompleteClassification）。

  本文件**不声明**（有效域边界，诚实标注，避免膨胀）：
  - ✗ cost/grid/约束上限来自缠论——ρ/β/γ/w/κ/成本模型/止损价/g_max/n_max/r_max/m_max/β_v
       全部是 **Θ_risk 参数化**，**不由分类 I 推出**（编排者核心命题：缠论结构不能推出
       仓位大小）。本文件只证「给定这些 Θ_risk 组件后 q* 存在唯一」。
  - ✗ q* 盈利 / 风险调整后最优——`riskproj_exists_unique` 只证 q* 是 cost 的**离散最小**
       （操作语义），**不证**它盈利或最优。盈利/最优需收益分布 + 效用（L3 EmpiricalDomain，
       标 `EmpiricalDomain`，不由此 L0 声称）。
  - ✗ 连续凸 argmin 存在——本文件走**有限格点**路线（`grid : List`），证的是**离散**网格
       字典序最小存在。**不冒充**连续可行集上的 argmin 存在（那需紧性/下半连续/强制性，
       StrategyFamily.lean §3 的诚实保留——0∈𝒦 只保非空，连续 argmin 可能不存在）。
  - ✗ 任意 cost 都给唯一 q*——唯一性依赖 `cost_inj`（cost 在 grid 上单射 = 字典序平局
       已用索引序二次比较消除）。这是 `RiskGrid` 的**良构条件**（固定字典序 tie-break），
       **不**冒充「任意代价函数都无平局」。
  - ✗ 风险投影是真完全分类——`riskProj_not_true_classification` 证**禁标**
       TrueCompleteClassification。风险投影是 π_Θ 执行链的一环（给定 Θ_risk 的操作语义），
       不是分类内容。

  ★与 StrategyFamily.lean 的关系：本文件是 `StrategyFamily.RiskProjector`（§3 抽象确定
    选择器）的**有限网格具体实现**——StrategyFamily 诚实保留「连续 argmin 可能不存在故只
    抽象为带证明选择器」，本文件在「仓位离散为有限网格」这一具体有效域内，把确定选择器
    **兑现为真 LexArgmin 存在且唯一**（有限性是兑现的关键前提）。两者互补：抽象接口
    （StrategyFamily）+ 有限实现（RiskProj），共同支撑「风险投影使多声部形成唯一总仓位」。
-/

end Strict.RiskProj
