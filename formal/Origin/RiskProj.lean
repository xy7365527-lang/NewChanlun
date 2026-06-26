/-
  Origin/RiskProj.lean — 风险投影使多声部形成唯一总仓位（port 到 Origin canonical base）
  ★task #114, A′ 策略组件港入 Origin canonical base（审计 B：S_Θ 组件 incomplete in Origin）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**（formal/Origin/）。审计 B 判决：
  闭环六段接口 + R=Π-A-W 已忠实，但策略侧四大支柱（π_Θ / C_Θ / RiskProj / VoiceTree）
  **全部 MISSING in Origin，只在 legacy Strict**。本文件把 legacy `Strict/RiskProj.lean`
  的「风险投影 = 有限网格字典序最小总仓位」**重锚到 Origin 类型**：定理挂 Origin 命名空间
  `NewChanlun.Origin.RiskProj`，复用 Origin `SourceAxioms` 的 `ExistsUnique`/`SingleValued`，
  **不** import legacy Strict（避免把 Origin 锚回 legacy）。

  ── 编排者命题（继承 legacy）──────────────────────────────────────────────────
  **风险投影把多声部各自的目标仓位约束到唯一的总仓位。**
  多声部（赋格，见 Origin/VoiceTree.lean）允许同一时刻多声部各持其向（多空双开）。这些声部
  的「期望仓位」加总后未必满足账户层风险约束。**风险投影** Π 把联合期望仓位投到**有限可行
  仓位网格** 𝒦 上的**唯一**目标仓位 q*——把「多声部分散意图」收束为「账户层单一总仓位」。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数，不依赖数据）。机器可检验命题：
  - `argmin_le`：有限非空 List + 可判定全序 ⟹ 字典序最小元 cost 不大于任意格点。
  - `riskproj_exists_unique`：有限非空网格 ⟹ 目标仓位存在且唯一（有限性 + cost 单射）。
  Lean build 通过 = 这些代数/逻辑命题正确（L0），**不**是「投影在真实账户上盈利/最优」
  （那是 L3 EmpiricalDomain，本文件不声称）。

  ── 诚实标注（formalization-validity-domain + no-patch-mentality）────────────
  ★cost/grid/约束上限全部是 **Θ_risk 参数（非缠论可导）**——缠论结构（Origin CompleteClassifier）
    **不能推出仓位大小**（编排者核心命题，见 Origin/StrategyFamily.lean 元定理1）。本文件证的是
    「**给定** Θ_risk 定义出的网格与代价后，投影结果存在且唯一」，**不**证参数本身来自缠论。
  ★存在性来自**有限性 + 非空**：0∈𝒦 只保非空，连续 argmin 可能不存在；有限网格的字典序
    最小**总存在**。唯一性来自 cost 单射（固定字典序 tie-break，平局已消除）。
  ★盈利/最优 = L3 EmpiricalDomain，本文件**不**证。**禁标 TrueCompleteClassification**。
  ★**多空双开结构允许**：网格约束**不加** Q⁺Q⁻=0 禁令（净头寸约束用绝对值）。

  ── 依赖方向（单向无环，不 import #113 分类血肉、不 import legacy Strict）──────
  RiskProj → Origin.SourceAxioms（仅用 ExistsUnique/SingleValued 风格）。standalone。
  验证：`cd formal && lake env lean Origin/RiskProj.lean`。禁 sorry/admit/axiom。

  谱系：legacy Strict/RiskProj.lean（cc-riskproj #73）→ #96/#97（A′ Origin canonical base）→
        本文件 #114（RiskProj 港入 Origin）。
-/

import Origin.SourceAxioms

namespace NewChanlun.Origin.RiskProj

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 可判定线性序（cost 值域：带可判定全序的代价类型）

  目标仓位 q* 通过最小化代价 cost 选出。代价值落在某个**可判定全序** K 中。风险投影的
  存在/唯一**不依赖** cost 的具体公式，只依赖其值域 K 是**可判定全序**——这样有限非空
  格点集上的字典序最小元才**存在且唯一**。把 cost 具体公式抽象为「点 → K 的任意全函数」
  正是「cost 是 Θ_risk 参数化、非缠论可导」的形式编码。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★可判定线性序 `DecidableLinearKey K`（L0）：代价值域 K 上的全序 + 可判定 ≤。

  最小公理集（使「字典序最小存在且唯一」可证）：refl/trans（预序）+ antisymm（反对称——
  平局即相等，唯一性关键）+ total（全序——存在性 foldl 关键，无不可比对）+ decLe（可判定）。
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

/-- ★`le` 的可判定实例（供 `if`/`decide` 使用）。 -/
instance instDecidableLe (a b : K) : Decidable (O.le a b) := O.decLe a b

end DecidableLinearKey

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 有限非空网格上的字典序最小元（构造性存在 + 真最小性）

  仓位网格 𝒦 是有限格点集——形式化为非空 `List Pos`（0∈𝒦 ⟹ 列表非空）。代价
  `cost : Pos → K` 把格点映到可判定全序 K。字典序最小元用 foldl 真正选出（非冒充存在性）：
  从首格点起逐个比较 cost，保留更小者（平局保留先出现者 = 索引 tie-break）。
  ════════════════════════════════════════════════════════════════════════ -/

variable {Pos K : Type}

/--
  ★二元择小 `pick`（L0）：两格点中按 cost 选字典序更小者，平局保留**第一个**（b 是已积累
  当前最优，x 是新格点）：`pick O cost b x = if le (cost b) (cost x) then b else x`。
-/
def pick (O : DecidableLinearKey K) (cost : Pos → K) (b x : Pos) : Pos :=
  if O.le (cost b) (cost x) then b else x

/-- ★`argmin`（构造性，L0）：以 a₀ 为初值，foldl `pick` 扫描 xs，逐步保留更优格点。 -/
def argmin (O : DecidableLinearKey K) (cost : Pos → K) (a₀ : Pos) (xs : List Pos) : Pos :=
  xs.foldl (pick O cost) a₀

/-- ★`argmin` 对空尾：退化为初值（foldl 空表）。 -/
@[simp] theorem argmin_nil (O : DecidableLinearKey K) (cost : Pos → K) (a₀ : Pos) :
    argmin O cost a₀ [] = a₀ := rfl

/-- ★`argmin` 展开一步：`argmin a₀ (x :: xs) = argmin (pick a₀ x) xs`。 -/
@[simp] theorem argmin_cons (O : DecidableLinearKey K) (cost : Pos → K)
    (a₀ x : Pos) (xs : List Pos) :
    argmin O cost a₀ (x :: xs) = argmin O cost (pick O cost a₀ x) xs := rfl

/-- ★`pick` 结果 cost ≤ 左输入 cost（L0）：全序保证择小后 cost 不大于 b。 -/
theorem pick_le_left (O : DecidableLinearKey K) (cost : Pos → K) (b x : Pos) :
    O.le (cost (pick O cost b x)) (cost b) := by
  unfold pick
  by_cases h : O.le (cost b) (cost x)
  · rw [if_pos h]; exact O.le_refl _
  · rw [if_neg h]
    rcases O.le_total (cost b) (cost x) with hle | hle
    · exact absurd hle h
    · exact hle

/-- ★`pick` 结果 cost ≤ 右输入 cost（L0）。 -/
theorem pick_le_right (O : DecidableLinearKey K) (cost : Pos → K) (b x : Pos) :
    O.le (cost (pick O cost b x)) (cost x) := by
  unfold pick
  by_cases h : O.le (cost b) (cost x)
  · rw [if_pos h]; exact h
  · rw [if_neg h]; exact O.le_refl _

/--
  ★`argmin` 成员性（L0，可行性）：返回的格点**确在** `a₀ :: xs` 中。
  保证「选出的目标仓位是真格点」——投影不越出网格（RuntimeGuard 可行性侧）。
-/
theorem argmin_mem (O : DecidableLinearKey K) (cost : Pos → K) (a₀ : Pos) :
    ∀ (xs : List Pos), argmin O cost a₀ xs ∈ (a₀ :: xs) := by
  intro xs
  induction xs generalizing a₀ with
  | nil => simp
  | cons x xs' ih =>
      rw [argmin_cons]
      have hrec := ih (pick O cost a₀ x)
      have hpick : pick O cost a₀ x = a₀ ∨ pick O cost a₀ x = x := by
        unfold pick; by_cases h : O.le (cost a₀) (cost x) <;> simp [h]
      rcases List.mem_cons.mp hrec with hhead | htail
      · rcases hpick with hp | hp
        · exact List.mem_cons.mpr (Or.inl (hhead.trans hp))
        · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inl (hhead.trans hp))))
      · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inr htail)))

/-- ★`argmin` 是初值的下界（L0）：`cost (argmin a₀ xs) ≤ cost a₀`（foldl 单调，每步不增）。 -/
theorem argmin_le_init (O : DecidableLinearKey K) (cost : Pos → K) :
    ∀ (xs : List Pos) (a₀ : Pos), O.le (cost (argmin O cost a₀ xs)) (cost a₀) := by
  intro xs
  induction xs with
  | nil => intro a₀; rw [argmin_nil]; exact O.le_refl _
  | cons x xs' ih =>
      intro a₀
      rw [argmin_cons]
      exact O.le_trans _ _ _ (ih (pick O cost a₀ x)) (pick_le_left O cost a₀ x)

/--
  ★★`argmin` 真最小性（L0，核心存在性的实质内容）：
  `cost (argmin a₀ xs) ≤ cost y` 对**所有** `y ∈ a₀ :: xs`——返回格点 cost 不大于网格任意格点。
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
      rcases List.mem_cons.mp hy with hya₀ | hyrest
      · rw [hya₀]
        exact O.le_trans _ _ _ (argmin_le_init O cost xs' (pick O cost a₀ x))
          (pick_le_left O cost a₀ x)
      · rcases List.mem_cons.mp hyrest with hyx | hyxs'
        · rw [hyx]
          exact O.le_trans _ _ _ (argmin_le_init O cost xs' (pick O cost a₀ x))
            (pick_le_right O cost a₀ x)
        · exact ih (pick O cost a₀ x) y (List.mem_cons.mpr (Or.inr hyxs'))

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 风险投影网格 RiskGrid + 核心定理 riskproj_exists_unique
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险投影网格 `RiskGrid Pos K`（L0，cc-riskproj 核心结构 port 到 Origin）。

  - `grid : List Pos`：**有限**仓位网格（格点列表，∏_v {0,δ_v,…,B_v} 离散化）。
  - `zero : Pos` + `zeroMem`：**0∈𝒦** 形式编码 ⟹ grid **非空**（存在性关键前提）。
  - `key : DecidableLinearKey K`：代价值域可判定全序（字典序比较）。
  - `cost : Pos → K`：代价函数（Θ_risk 参数化）。
  - `cost_inj`：**cost 在 grid 上单射**——「固定字典序平局」编码，使 LexArgmin **唯一**。

  ★诚实标注：cost/key/grid 全是 Θ_risk 参数化，不由 Origin CompleteClassifier 推出。
  `cost_inj` 是字典序 tie-break 的兑现假设（真实 cost 若有平局需附索引序二次比较使其单射）。
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

/-- ★grid 非空（`zeroMem` ⟹ `zero ∈ grid` ⟹ grid ≠ []）。 -/
theorem grid_ne_nil : R.grid ≠ [] := by
  intro h
  have hz := R.zeroMem
  rw [h] at hz
  simp at hz

/--
  ★风险投影 `gridProject`（构造性，L0）：网格上 cost 字典序最小格点 = 唯一目标仓位 q*。
  `[]` 分支退化为 `R.zero`（不可达——grid 非空，但 zeroMem 保证此 junk 值仍可行）；
  `a₀ :: xs` 分支 = `argmin a₀ xs`。这是「多声部期望仓位经风险投影收束为唯一总仓位」的算子。
-/
def gridProject : Pos :=
  match R.grid with
  | [] => R.zero
  | a₀ :: xs => argmin R.key R.cost a₀ xs

/-- ★`gridProject` 可行（L0，RuntimeGuard 可行性侧）：投影结果 ∈ grid（是真格点）。 -/
theorem gridProject_mem : R.gridProject ∈ R.grid := by
  unfold gridProject
  cases hg : R.grid with
  | nil =>
      have hz := R.zeroMem; rw [hg] at hz; simp at hz
  | cons a₀ xs =>
      exact argmin_mem R.key R.cost a₀ xs

/-- ★★`gridProject` 真最小性（L0）：cost(gridProject) ≤ cost(y) 对**所有** y ∈ grid。 -/
theorem gridProject_le (y : Pos) (hy : y ∈ R.grid) :
    R.key.le (R.cost R.gridProject) (R.cost y) := by
  unfold gridProject
  cases hg : R.grid with
  | nil =>
      rw [hg] at hy; simp at hy
  | cons a₀ xs =>
      have hy' : y ∈ (a₀ :: xs) := by rw [← hg]; exact hy
      exact argmin_le R.key R.cost xs a₀ y hy'

/--
  ★★★核心定理 `riskproj_exists_unique`（L0，cc-riskproj 主交付 port 到 Origin）★★★：
  **有限非空网格 ⟹ 字典序最小目标仓位存在且唯一。**

  存在 `q* ∈ grid` 是网格上 cost 字典序最小格点，且这样的 q* **唯一**。
  ★存在性来自**有限性 + 非空**（0∈𝒦 只保非空，存在需有限——有限网格字典序最小总存在）。
  ★唯一性来自**字典序平局已消除**（cost_inj）：两最小格点互相 ≤ ⟹ 反对称 cost 相等 ⟹ 单射 ⟹ 相等。
  ★诚实标注：cost/grid/约束全 Θ_risk 参数。本定理证「给定 Θ_risk 后 q* 存在唯一」，**不**证盈利/最优（L3）。
-/
theorem riskproj_exists_unique :
    ∃ q : Pos, q ∈ R.grid ∧ (∀ y, y ∈ R.grid → R.key.le (R.cost q) (R.cost y)) ∧
      (∀ q', q' ∈ R.grid → (∀ y, y ∈ R.grid → R.key.le (R.cost q') (R.cost y)) → q' = q) := by
  refine ⟨R.gridProject, R.gridProject_mem, ?_, ?_⟩
  · intro y hy
    exact R.gridProject_le y hy
  · intro q' hq' hmin'
    have hle1 : R.key.le (R.cost q') (R.cost R.gridProject) :=
      hmin' R.gridProject R.gridProject_mem
    have hle2 : R.key.le (R.cost R.gridProject) (R.cost q') :=
      R.gridProject_le q' hq'
    have heqcost : R.cost q' = R.cost R.gridProject :=
      R.key.le_antisymm _ _ hle1 hle2
    exact R.cost_inj q' R.gridProject hq' R.gridProject_mem heqcost

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 确定选择器封装（gridProject 全定义 + 唯一 + 可行 + 命中最小）
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★投影确定性（L0）：`gridProject` 是确定值——`∃! p, gridProject = p`（全 + 唯一）。 -/
theorem gridProject_existsUnique :
    ∃ p : Pos, R.gridProject = p ∧ ∀ p', R.gridProject = p' → p' = p :=
  ⟨R.gridProject, rfl, fun _ heq => heq.symm⟩

/-- ★`IsLexArgmin` 谓词（L0）：p 是网格字典序最优 ⟺ p∈grid 且 cost p ≤ 所有格点 cost。 -/
def IsLexArgmin (p : Pos) : Prop :=
  p ∈ R.grid ∧ ∀ y, y ∈ R.grid → R.key.le (R.cost p) (R.cost y)

/-- ★`gridProject` 命中 LexArgmin（L0）：投影结果是字典序最优。 -/
theorem gridProject_isLexArgmin : R.IsLexArgmin R.gridProject :=
  ⟨R.gridProject_mem, fun y hy => R.gridProject_le y hy⟩

/-- ★LexArgmin 唯一（L0）：任意 LexArgmin 都 = gridProject（互相 ≤ ⟹ cost 相等 ⟹ 单射 ⟹ 相等）。 -/
theorem lexargmin_eq_project (p : Pos) (hp : R.IsLexArgmin p) :
    p = R.gridProject := by
  obtain ⟨hmem, hmin⟩ := hp
  have hle1 : R.key.le (R.cost p) (R.cost R.gridProject) :=
    hmin R.gridProject R.gridProject_mem
  have hle2 : R.key.le (R.cost R.gridProject) (R.cost p) :=
    R.gridProject_le p hmem
  have heqcost : R.cost p = R.cost R.gridProject :=
    R.key.le_antisymm _ _ hle1 hle2
  exact R.cost_inj p R.gridProject hmem R.gridProject_mem heqcost

end RiskGrid

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险投影标签 `RiskProjTag`（gatekeeper，诚实分层）。
  OperationalSemanticsOnly（操作语义）+ RuntimeGuard（运行时风险守卫）+
  ThetaRiskParametric（Θ_risk 参数化，非缠论可导）+ EmpiricalDomain（盈利/最优 L3）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把风险投影标为分类定理。
-/
inductive RiskProjTag where
  | OperationalSemanticsOnly
  | RuntimeGuard
  | ThetaRiskParametric
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★风险投影子类（gatekeeper）：UniqueTotalPositionGivenThetaRisk（唯一子类）。 -/
inductive RiskProjSubkind where
  | UniqueTotalPositionGivenThetaRisk
deriving DecidableEq, Repr

/-- ★风险投影诚实标签包（L0 声明）。 -/
def riskProjLabels : List RiskProjTag × RiskProjSubkind :=
  ([RiskProjTag.OperationalSemanticsOnly, RiskProjTag.RuntimeGuard,
    RiskProjTag.ThetaRiskParametric, RiskProjTag.EmpiricalDomain],
   RiskProjSubkind.UniqueTotalPositionGivenThetaRisk)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：风险投影子类必是 UniqueTotalPositionGivenThetaRisk。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝冒充真完全分类（不是跨库强保证）。 -/
theorem riskProj_not_true_classification (k : RiskProjSubkind) :
    k = RiskProjSubkind.UniqueTotalPositionGivenThetaRisk := by
  cases k; rfl

end NewChanlun.Origin.RiskProj
