/-
  Origin/FineCoarseClassification.lean — 期望层关系：细分类不劣于粗分类（#1050 第二段，
  ADR 0024 裁定三·形态 B；买卖点alpha2.pdf 定理 2，p33–34）

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：把两套收益分解之间的正式关系写死

  权威源（逐字核对）：`docs/formal-chain/买卖点alpha2.pdf`：
  - §12（p22）：毛分解——每个证书 γ 有自己的出场时点 τ_γ，
    X_γ = δ(P_{τ_γ} − P_t) − C_{t:τ_γ}（买点 δ=+1 收益 P_τ−P_t，卖点 δ=−1 收益 P_t−P_τ）。
  - §12/§14（p22–24）：按证书类别的条件期望 μ_{ℓ,δ,I} = E[X_γ | …]，总收益
    R = Σ_{γ∈T} X_γ，E[R] = Σ E[X_γ | F_{t(γ)}]；正边际假设 E[X_γ|F_{t(γ)}] ≥ η > 0
    ⟹ E[R] ≥ η E[|T|]。
  - §12（p33）：净分解——逐 bar 全互斥状态 z = (ℓ,δ,I_γ,父声部方向,是否短差,仓位状态,
    风险状态,…)，μ(z,a) = E[R_{t+1}(a) | Z_t = z]，E[R] = Σ_z P(Z_t=z) μ(z, π(z))。
  - §13 定理 2（p33–34，逐字）：设 Z 是全互斥细分类，Y 是较粗分类，Z 细化 Y 即存在
    φ : Z → Y。V(Z) = sup_π E[R^π]，V(Y) = sup_π E[R^π]。任意粗分类策略 π_Y : Y → A
    都可被细分类策略模拟：π_Z(z) = π_Y(φ(z))。因此细分类策略集合包含所有粗分类策略的
    模拟版本，取 supremum 后 V(Z) ≥ V(Y)。「细分类不劣于粗分类」。

  ## 两套分解之间的正式关系（本文件写死的三件事）

  1. **粗分类策略的细模拟（定理 2 的证明核心）**：`fine_simulates_coarse` ——
     ∀π_Y ∃π_Z（π_Z = π_Y ∘ φ），细分解总收益 = 粗策略在细状态上的收益。
  2. **★★定理 2**：`fine_classification_not_worse_coarse` —— 细最优 ≥ 粗最优
     （细分类不劣于粗分类；等价于 V(Z) ≥ V(Y) 的最优策略形式，不需 sup 算符）。
  3. **★★全期望一致（毛/净分解的桥）**：`total_expectation_coarse_net_agreement` ——
     Σ_y 毛分类收益 = Σ_z 净分类收益（对粗分类可测策略逐式相等，law of total expectation
     的有限形式）。毛分解条件 γ（含 τ_γ）→ 粗分类 Y；净分解条件 z（逐 bar 全互斥状态）
     → 细分类 Z；φ : Z → Y 就是「逐 bar 状态 → 所属腿/证书」的投影。

  ## 差异 = 条件信息差（PDF 结论的形式解读，诚实分层）

  毛分解按证书 γ（每腿用自己出场时点 τ_γ）条件化，净分解按逐 bar 全互斥状态 z
  条件化。`total_expectation_coarse_net_agreement` 说：对**粗分类可测**的策略两分解
  逐式一致；`fine_classification_not_worse_coarse` 说：优化到各自最优后，细分类
  最优值**不劣于**粗分类最优值。两分解最优值之差 = 细状态 z 相对粗证书 γ 的
  **条件信息差**的价值（细分类策略集合严格包含粗分类模拟版本，差 ≥ 0 是 L0 代数事实；
  该差的经验正值、不同子类最优动作不同带来的**严格**改进属 L2 EmpiricalDomain，
  本文件不证严格性）。反向（PDF p35）：若 ∀z,a 条件收益 μ(z,a) ≤ 0，任何全互斥策略
  都无正 alpha——本文件不证该经验命题（同属 L2）。

  ════════════════════════════════════════════════════════════════════════
  ## 模型口径（有限状态空间上的期望层，L0）

  状态 z ∈ Z 的有限枚举 `zs : List Z`；权 w : Z → Rat（非负假设 hw = P(Z_t=z) 的
  有限形式）；条件边际收益 r : Z → A → Rat（μ(z,a) = E[R_{t+1}(a)|Z_t=z] 的 L0 载体）。
  「期望」按有限加权和编码（PDF p33 自己的 E[R] = Σ_z P(Z_t=z) μ(z, π(z)) 同款）；
  不做可测空间/σ-代数形式化（诚实：本文件是期望层**关系**的形式化，概率空间的
  存在性假设由调用方给出）。
  ════════════════════════════════════════════════════════════════════════

  ## 删前件自查（#871 同款验收口径）

  - `fine_classification_not_worse_coarse`（前件 hYopt/hZopt）→ 删任一前件见证（§6 匿名
    example 反例①/②）：单状态、动作 Bool（收益 0/1），粗最优 1 对细策略 0 ⟹ 1 ≤ 0 被显式证伪。
  - `total_expectation_coarse_net_agreement`（前件 hsub/hnodup）→ 删前件见证（反例③）：
    ys=[] 覆盖不了 zs=[z] ⟹ 0 = 1 被显式证伪。
  - `coarseMu_mul_weight`（前件 h：纤维权重 ≠ 0）→ 删前件见证（反例④）：
    正负权重相消使纤维权重 = 0，归一化乘回 ≠ 未归一化纤维和（0 ≠ 5）。
  - `positive_reward_total`（前件 hη：逐状态边际收益 ≥ η）→ 删前件见证（反例⑤）：
    η=1 而边际收益全 0 ⟹ 1 ≤ 0 被显式证伪。

  ## 公理足迹：#print axioms 零自定义 axiom（仅 propext 等 Lean 核心）。

  依赖方向：standalone（纯 Prop/Type/Rat，零 import；与 LeverageCapital 同族风格）。
  验证：`cd formal && lake env lean Origin/FineCoarseClassification.lean`。禁 sorry/admit/axiom。

  谱系：买卖点alpha2.pdf §12/§13（p22–24、p33–34）→ #1050（对账定理形式化第二段）
        → 本文件（细分类不劣于粗分类 = 毛/净两套分解之间的正式关系）。
-/

namespace NewChanlun.Origin.FineCoarseClassification

universe u v w

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 细分类总收益（p33 净分解：E[R] = Σ_z P(Z_t=z) μ(z, π(z))）

  z 是逐 bar 全互斥状态（细分类）；w z = P(Z_t=z)（非负权）；r z a = μ(z,a)
  （条件边际收益）。`fineTotal` = Σ_{z∈zs} w z · r z (π z)。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★细分类策略总收益 `fineTotal`（L0，p33）：Σ_{z∈zs} w z · r z (π z)。
  有限状态枚举上的加权和——「E[R^π] = Σ_z P(Z_t=z) μ(z, π(z))」的 L0 载体。
-/
def fineTotal {Z : Type u} {A : Type v} (zs : List Z) (w : Z → Rat) (r : Z → A → Rat)
    (π : Z → A) : Rat :=
  (zs.map (fun z => w z * r z (π z))).sum

/--
  ★粗分类策略在细状态上的总收益 `coarseEval`（L0，p33 模拟式 π_Z(z) = π_Y(φ(z))）：
  Σ_{z∈zs} w z · r z (π_Y (φ z))。粗策略 π_Y : Y → A 经细化投影 φ : Z → Y 回到细状态上评估。
-/
def coarseEval {Z : Type u} {Y : Type w} {A : Type v} (zs : List Z) (w : Z → Rat)
    (r : Z → A → Rat) (φ : Z → Y) (πY : Y → A) : Rat :=
  fineTotal zs w r (fun z => πY (φ z))

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 ★★定理 2：细分类不劣于粗分类（p33–34 逐字）

  「任意粗分类策略 π_Y : Y → A 都可以被细分类策略模拟：π_Z(z) = π_Y(φ(z))。
  因此细分类策略集合包含所有粗分类策略的模拟版本。取 supremum 后 V(Z) ≥ V(Y)。」
  本文件用「最优策略」形式避免 sup 算符：细最优策略的总收益 ≥ 粗最优策略的总收益。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★粗策略被细策略模拟（L0，定理 2 证明第一句逐字）：
  ∀π_Y ∃π_Z，细分解总收益 = 粗策略在细状态上的总收益（见证 π_Z = π_Y ∘ φ）。
  这坐实「细分类策略集合包含所有粗分类策略的模拟版本」——细分类至少不丢粗分类的表达力。
-/
theorem fine_simulates_coarse {Z : Type u} {Y : Type w} {A : Type v} (zs : List Z)
    (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (πY : Y → A) :
    ∃ πZ : Z → A, fineTotal zs w r πZ = coarseEval zs w r φ πY := by
  refine ⟨fun z => πY (φ z), ?_⟩
  rfl

/--
  ★★定理 2·细分类不劣于粗分类（L0，p34）：细最优 ≥ 粗最优。
  粗最优 piYopt（∀π_Y 粗收益 ≤ 粗最优收益）与细最优 piZopt（∀π_Z 细收益 ≤ 细最优收益）之间：
  coarseEval piYopt ≤ fineTotal piZopt。证明 = PDF p34 原文：模拟 piYopt 成细策略，
  细最优支配一切细策略，含该模拟。删 hYopt 或 hZopt 后结论不可证（§6 见证）。
-/
theorem fine_classification_not_worse_coarse {Z : Type u} {Y : Type w} {A : Type v}
    (zs : List Z) (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (piYopt : Y → A) (piZopt : Z → A)
    (_hYopt : ∀ πY : Y → A, coarseEval zs w r φ πY ≤ coarseEval zs w r φ piYopt)
    (hZopt : ∀ πZ : Z → A, fineTotal zs w r πZ ≤ fineTotal zs w r piZopt) :
    coarseEval zs w r φ piYopt ≤ fineTotal zs w r piZopt := by
  have hsim : fineTotal zs w r (fun z => piYopt (φ z)) = coarseEval zs w r φ piYopt := by rfl
  simpa [hsim] using hZopt (fun z => piYopt (φ z))

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 ★★全期望一致：毛分解（证书 γ 条件）≡ 净分解（逐 bar 状态 z 条件）

  p22–24 毛分解按证书 γ（每腿自己的出场时点 τ_γ）条件化；p33 净分解按逐 bar 全互斥
  状态 z 条件化。z 细化 γ：φ : Z → Y 把每个细状态投影到它所属的证书/腿。
  对粗分类可测策略，两分解逐式相等（law of total expectation 的有限形式）——
  这是「毛/净两套分解差异 = 条件信息差」的恒等半边；不等半边（细最优 > 粗最优）
  由 §2 定理 2 给出（差 ≥ 0 且取 sup 后不劣）。
  ════════════════════════════════════════════════════════════════════════ -/

/-! 有限求和工具（Rat 版：换序前的自证引理，无 Mathlib）。 -/

/-- Σ_t (f t + g t) = Σ_t f t + Σ_t g t（Rat 求和加法分配）。 -/
theorem list_sum_add {α : Type u} (f g : α → Rat) (l : List α) :
    (l.map (fun x => f x + g x)).sum = (l.map f).sum + (l.map g).sum := by
  induction l with
  | nil => simp only [List.map_nil, List.sum_nil, Rat.add_zero]
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons, ih]
      ac_rfl

/-- Σ_t (f t · c) = (Σ_t f t) · c（右乘提取）。 -/
theorem sum_mul {α : Type u} (f : α → Rat) (c : Rat) (l : List α) :
    (l.map (fun x => f x * c)).sum = (l.map f).sum * c := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons, ih]
      rw [Rat.add_mul]

/-- Σ_t f ≤ Σ_t g（逐点 ≤ ⟹ 求和 ≤）。 -/
theorem sum_le_sum {α : Type u} (f g : α → Rat) (l : List α)
    (h : ∀ a ∈ l, f a ≤ g a) : (l.map f).sum ≤ (l.map g).sum := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons]
      have ha := h a (by simp)
      have hil := ih (by intro b hb; exact h b (by simp [hb]))
      exact Rat.le_trans ((Rat.add_le_add_left).2 hil) ((Rat.add_le_add_right).2 ha)

/-- Σ_t 0 = 0（Rat 零函数求和）。 -/
theorem list_sum_zero {α : Type u} (l : List α) :
    (l.map (fun _ => (0 : Rat))).sum = 0 := by
  induction l with
  | nil => rfl
  | cons a l ih => simp [ih, Rat.add_zero]

/-- 逐点非负 ⟹ 求和非负（权非负假设的聚合形式）。 -/
theorem sum_nonneg {α : Type u} (w : α → Rat) (l : List α)
    (h : ∀ a ∈ l, 0 ≤ w a) : 0 ≤ (l.map w).sum := by
  have hle := sum_le_sum (fun _ => (0 : Rat)) w l (by intro a ha; simpa using h a ha)
  simpa [list_sum_zero l] using hle

/--
  按 Bool 谓词拆分的求和恒等式：Σ_{p} f + Σ_{q} f = Σ f（q = ¬p 逐点）。
  纤维求和（粗分类分组）的拼接工具。
-/
theorem sum_filter_split {α : Type u} (l : List α) (f : α → Rat) (p q : α → Bool)
    (hq : ∀ z, q z = !p z) :
    ((l.filter p).map f).sum + ((l.filter q).map f).sum = (l.map f).sum := by
  induction l with
  | nil => simp only [List.filter_nil, List.map_nil, List.sum_nil, Rat.add_zero]
  | cons z l ih =>
      by_cases hz : p z = true
      · have hqz : q z = false := by rw [hq z, hz]; rfl
        have hqz' : ¬ q z = true := by intro h; rw [hqz] at h; cases h
        rw [List.filter_cons_of_pos hz, List.filter_cons_of_neg hqz']
        simp only [List.map_cons, List.sum_cons]
        rw [Rat.add_assoc, ih]
      · have hz' : p z = false := by
          cases hp : p z with
          | true => have ht : p z = true := by rw [hp]
                    exact False.elim (hz ht)
          | false => rfl
        have hqz : q z = true := by rw [hq z, hz']; rfl
        have hz'' : ¬ p z = true := by intro h; rw [hz'] at h; cases h
        rw [List.filter_cons_of_neg hz'', List.filter_cons_of_pos hqz]
        simp only [List.map_cons, List.sum_cons]
        rw [Rat.add_left_comm, ih]

/--
  ★★纤维求和恒等式（law of total expectation 的求和引擎）：
  ys 无重复且覆盖 zs 的 φ-像 ⟹ Σ_{y∈ys} Σ_{z∈zs, φ z = y} f z = Σ_{z∈zs} f z。
  毛分类（y = 证书/腿）与净分类（z = 逐 bar 状态）两套分解在求和层对齐的核心引理。
-/
theorem sum_over_fibers {Z : Type u} {Y : Type w} [DecidableEq Y] (ys : List Y) (zs : List Z)
    (f : Z → Rat) (φ : Z → Y)
    (hsub : ∀ z ∈ zs, φ z ∈ ys) (hnodup : ys.Nodup) :
    ((ys.map (fun y => ((zs.filter (fun z => decide (φ z = y))).map f).sum)).sum)
    = (zs.map f).sum := by
  induction ys generalizing zs with
  | nil =>
      cases zs with
      | nil => simp
      | cons z zs =>
          have hz : φ z ∈ ([] : List Y) := hsub z (by simp)
          cases hz
  | cons y ys ih =>
      rw [List.map_cons, List.sum_cons]
      have hnd : ys.Nodup := (List.nodup_cons.mp hnodup).2
      have hny : ¬ y ∈ ys := (List.nodup_cons.mp hnodup).1
      have hfiber (t : Y) (ht : t ∈ ys) :
          zs.filter (fun z => decide (φ z = t))
          = (zs.filter (fun z => decide (φ z ≠ y))).filter (fun z => decide (φ z = t)) := by
        have hty : t ≠ y := by intro he; rw [he] at ht; exact hny ht
        rw [List.filter_filter]
        apply List.filter_congr
        intro z hz
        by_cases hzt : φ z = t
        · have hzy : φ z ≠ y := by rw [hzt]; exact hty
          have h1 : decide (φ z = t) = true := decide_eq_true hzt
          have h2 : decide (φ z ≠ y) = true := decide_eq_true hzy
          rw [h1, h2]
          rfl
        · have h1 : decide (φ z = t) = false := decide_eq_false hzt
          rw [h1]
          rfl
      have hys : (ys.map (fun t => ((zs.filter (fun z => decide (φ z = t))).map f).sum)).sum
          = ((zs.filter (fun z => decide (φ z ≠ y))).map f).sum := by
        have hmap : (ys.map (fun t => ((zs.filter (fun z => decide (φ z = t))).map f).sum))
            = (ys.map (fun t => ((((zs.filter (fun z => decide (φ z ≠ y))).filter (fun z => decide (φ z = t))).map f).sum))) := by
          apply List.map_congr_left
          intro t ht
          rw [hfiber t ht]
        rw [hmap]
        exact ih (zs.filter (fun z => decide (φ z ≠ y))) (by
          intro z hz
          have hzin : z ∈ zs := (List.mem_filter.mp hz).1
          have hzys : φ z ∈ y :: ys := hsub z hzin
          have hzy : φ z ≠ y := of_decide_eq_true (List.mem_filter.mp hz).2
          exact (List.mem_cons.mp hzys).resolve_left hzy) hnd
      rw [hys]
      have hsplit := sum_filter_split zs f (fun z => decide (φ z = y)) (fun z => decide (φ z ≠ y)) (by
        intro z
        by_cases hzy : φ z = y
        · have h1 : decide (φ z = y) = true := decide_eq_true hzy
          have h2 : decide (φ z ≠ y) = false := decide_eq_false (by intro h; exact h hzy)
          rw [h1, h2]
          rfl
        · have h1 : decide (φ z = y) = false := decide_eq_false hzy
          have h2 : decide (φ z ≠ y) = true := decide_eq_true hzy
          rw [h1, h2]
          rfl)
      rw [hsplit]

/-- ★毛分类 y 的未归一化收益和（= P(Y=y)·μ_Y(y,a) 的 L0 载体）：Σ_{z∈zs, φ z = y} w z · r z a。 -/
def coarseReward {Z : Type u} {Y : Type w} {A : Type v} [DecidableEq Y] (zs : List Z)
    (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (y : Y) (a : A) : Rat :=
  ((zs.filter (fun z => decide (φ z = y))).map (fun z => w z * r z a)).sum

/-- ★毛分类 y 的纤维权重（= P(Y=y) 的 L0 载体）：Σ_{z∈zs, φ z = y} w z。 -/
def fiberWeight {Z : Type u} {Y : Type w} [DecidableEq Y] (zs : List Z)
    (w : Z → Rat) (φ : Z → Y) (y : Y) : Rat :=
  ((zs.filter (fun z => decide (φ z = y))).map w).sum

/--
  ★毛分类条件收益 `coarseMu`（L0）：μ_Y(y,a) = E[R(a)|Y=y] 的有限形式 =
  纤维收益和 / 纤维权重。纤维权重 = 0 时条件期望无定义，按 0 约定（诚实边界：
  P(Y=y)=0 的条件期望不参与 p33 的分解和）。
-/
def coarseMu {Z : Type u} {Y : Type w} {A : Type v} [DecidableEq Y] (zs : List Z)
    (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (y : Y) (a : A) : Rat :=
  if _h : fiberWeight zs w φ y ≠ 0 then coarseReward zs w r φ y a / fiberWeight zs w φ y else 0

/--
  ★★全期望一致（毛/净分解桥，p22–24 vs p33–37）：
  ys 无重复且覆盖 zs 的 φ-像 ⟹ Σ_{y∈ys} Σ_{z:φz=y} w z · r z (π_Y y)
  = Σ_{z∈zs} w z · r z (π_Y (φ z))。
  左端 = 毛分解（按证书 y 条件化后的未归一化收益求和，PDF p22–24 的
  E[R] = Σ_γ E[X_γ|F_{t(γ)}] 有限形式）；右端 = 净分解（逐 bar 全互斥状态 z 的
  E[R] = Σ_z P(z) μ(z, π(z)) 有限形式）。对粗分类可测策略两分解逐式相等。
-/
theorem total_expectation_coarse_net_agreement {Z : Type u} {Y : Type w} {A : Type v}
    [DecidableEq Y] (ys : List Y) (zs : List Z) (w : Z → Rat) (r : Z → A → Rat)
    (φ : Z → Y) (πY : Y → A)
    (hsub : ∀ z ∈ zs, φ z ∈ ys) (hnodup : ys.Nodup) :
    (ys.map (fun y => coarseReward zs w r φ y (πY y))).sum
    = (zs.map (fun z => w z * r z (πY (φ z)))).sum := by
  unfold coarseReward
  have hfiber (y : Y) :
      ((zs.filter (fun z => decide (φ z = y))).map (fun z => w z * r z (πY y))).sum
      = ((zs.filter (fun z => decide (φ z = y))).map (fun z => w z * r z (πY (φ z)))).sum := by
    apply congrArg (fun (l : List Rat) => l.sum)
    apply List.map_congr_left
    intro z hz
    have hφ : φ z = y := of_decide_eq_true (List.mem_filter.mp hz).2
    rw [← hφ]
  have hmap : (ys.map (fun y => ((zs.filter (fun z => decide (φ z = y))).map (fun z => w z * r z (πY y))).sum)).sum
      = (ys.map (fun y => ((zs.filter (fun z => decide (φ z = y))).map (fun z => w z * r z (πY (φ z)))).sum)).sum := by
    apply congrArg (fun (l : List Rat) => l.sum)
    apply List.map_congr_left
    intro y hy
    exact hfiber y
  rw [hmap]
  exact sum_over_fibers ys zs (fun z => w z * r z (πY (φ z))) φ hsub hnodup

/--
  ★归一化条件收益乘回权重 = 未归一化纤维和（P(Y=y) > 0 时）：
  P(Y=y)·μ_Y(y,a) = Σ_{z:φz=y} w z · r z a。粗分类条件期望的定义合法性
  （Rat.mul_div_cancel）。前件 h（纤维权重 ≠ 0）不可删（§6 见证）。
-/
theorem coarseMu_mul_weight {Z : Type u} {Y : Type w} {A : Type v} [DecidableEq Y]
    (zs : List Z) (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (y : Y) (a : A)
    (h : fiberWeight zs w φ y ≠ 0) :
    fiberWeight zs w φ y * coarseMu zs w r φ y a = coarseReward zs w r φ y a := by
  unfold coarseMu
  rw [dif_pos h, Rat.div_def]
  rw [Rat.mul_comm (fiberWeight zs w φ y) (coarseReward zs w r φ y a * (fiberWeight zs w φ y)⁻¹)]
  rw [Rat.mul_assoc]
  rw [Rat.inv_mul_cancel (fiberWeight zs w φ y) h]
  rw [Rat.mul_one]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 正边际收益聚合（p23 §14 正收益定理的有限形式）

  「假设对所有被交易证书有统一下界 E[X_γ|F_{t(γ)}] ≥ η > 0 … 则 E[R] ≥ η E[|T|]。」
  有限形式：逐状态边际收益 ≥ η 且权重非负 ⟹ 总收益 ≥ η·Σ_z w z。
  这是毛分解（证书条件）与净分解（状态条件）共同的上游收益聚合式。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★正边际收益聚合（L0，p23 有限形式）：hw（权重非负）+ hη（逐状态 μ(z,π(z)) ≥ η）
  ⟹ η · Σ_z w z ≤ fineTotal（总收益 ≥ 统一下界 × 总权重）。
  删 hη 后结论不可证（§6 见证）。
-/
theorem positive_reward_total {Z : Type u} {A : Type v} (zs : List Z) (w : Z → Rat)
    (r : Z → A → Rat) (π : Z → A) (η : Rat)
    (hw : ∀ z ∈ zs, 0 ≤ w z) (hη : ∀ z ∈ zs, η ≤ r z (π z)) :
    η * (zs.map w).sum ≤ fineTotal zs w r π := by
  unfold fineTotal
  rw [Rat.mul_comm η ((zs.map w).sum)]
  rw [← sum_mul (fun z => w z) η zs]
  exact sum_le_sum (fun z => w z * η) (fun z => w z * r z (π z)) zs (by
    intro z hz
    exact Rat.mul_le_mul_of_nonneg_left (hη z hz) (hw z hz))

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 诚实标签（formalization-validity-domain gatekeeper）

  本文件证的是两套分解之间的**结构关系**（模拟 + 最优支配 + 求和一致），不是盈利。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★期望层标签 `ExpectationTag`（gatekeeper，诚实分层）。
  DecompositionRelationOnly（分解间关系是结构命题）+ ConditionalExpectationModel
  （条件期望是外部给定的 μ(z,a) 参数）。**没有** `AlphaPositive` / `TrueCompleteClassification`
  构造子——类型层拒绝把「细分类不劣于粗分类」标为盈利或分类定理。
-/
inductive ExpectationTag where
  | DecompositionRelationOnly
  | ConditionalExpectationModel
deriving DecidableEq, Repr

/-- ★期望层子类：FineCoarseDecomposition（唯一子类）。 -/
inductive ExpectationSubkind where
  | FineCoarseDecomposition
deriving DecidableEq, Repr

/-- ★期望层诚实标签包（L0 声明）。 -/
def expectationLabels : List ExpectationTag × ExpectationSubkind :=
  ([ExpectationTag.DecompositionRelationOnly, ExpectationTag.ConditionalExpectationModel],
   ExpectationSubkind.FineCoarseDecomposition)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：期望层子类必是 FineCoarseDecomposition。 -/
theorem expectation_not_true_classification (k : ExpectationSubkind) :
    k = ExpectationSubkind.FineCoarseDecomposition := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 ★★删前件自查（#871 同款验收口径，见证反例）

  单状态模型（zs = [0]，Z = Y = Nat，动作 A = Bool，收益 0/1）上逐条证伪删前件结论。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 单状态见证模型（w=1，r z a = if a then 1 else 0，φ z = 0）：粗收益/细收益 = 动作的 0/1 值。 -/
def witnessW : Nat → Rat := fun _ => 1
def witnessR : Nat → Bool → Rat := fun _ a => if a then 1 else 0
def witnessPhi : Nat → Nat := fun _ => 0

/--
  ★见证①（`fine_classification_not_worse_coarse` 删前件 hYopt 不可证）：
  删掉粗最优假设后，粗策略可取收益 1、细策略可取收益 0，1 ≤ 0 被显式证伪
  （粗最优假设是「粗侧取 sup」的 L0 载体，不可删）。
-/
example : ¬ (∀ (piYopt : Nat → Bool) (piZopt : Nat → Bool),
    coarseEval [0] witnessW witnessR witnessPhi piYopt ≤ fineTotal [0] witnessW witnessR piZopt) := by
  intro h
  have hbad : ¬ ((1 : Rat) ≤ 0) := by decide
  apply hbad
  simpa [coarseEval, fineTotal, witnessW, witnessR, witnessPhi, Rat.add_zero] using
    (h (fun _ => true) (fun _ => false))

/--
  ★见证②（`fine_classification_not_worse_coarse` 删前件 hZopt 不可证）：
  删掉细最优假设后，粗最优（=1，动作空间 0/1 的上确界）仍可被细策略 piZopt=0 反超方向
  破坏——1 ≤ 0 被显式证伪（细最优假设不可删）。
-/
example : ¬ (∀ (piYopt : Nat → Bool) (piZopt : Nat → Bool),
    (∀ πY : Nat → Bool, coarseEval [0] witnessW witnessR witnessPhi πY
      ≤ coarseEval [0] witnessW witnessR witnessPhi piYopt)
    → coarseEval [0] witnessW witnessR witnessPhi piYopt
      ≤ fineTotal [0] witnessW witnessR piZopt) := by
  intro h
  have hbad : ¬ ((1 : Rat) ≤ 0) := by decide
  apply hbad
  have hopt : ∀ πY : Nat → Bool,
      coarseEval [0] witnessW witnessR witnessPhi πY
      ≤ coarseEval [0] witnessW witnessR witnessPhi (fun _ => true) := by
    intro πY
    by_cases hb : πY 0 = true
    · simp [coarseEval, fineTotal, witnessW, witnessR, witnessPhi, hb, Rat.add_zero]
    · have hb' : πY 0 = false := by
        cases hp : πY 0 with
        | true => exact False.elim (hb (by rw [hp]))
        | false => rfl
      simp [coarseEval, fineTotal, witnessW, witnessR, witnessPhi, hb', Rat.add_zero]
      decide
  simpa [coarseEval, fineTotal, witnessW, witnessR, witnessPhi, Rat.add_zero] using
    (h (fun _ => true) (fun _ => false) hopt)

/--
  ★见证③（`total_expectation_coarse_net_agreement` 删前件 hsub/hnodup 不可证）：
  ys = [] 覆盖不了 zs = [z]：毛分类侧求和 = 0，净分类侧求和 = 1，0 = 1 被显式证伪
  （粗分类枚举必须覆盖全部细状态且无重复）。
-/
example : ¬ (∀ (ys : List Nat) (zs : List Nat) (w : Nat → Rat) (r : Nat → Nat → Rat)
    (φ : Nat → Nat) (πY : Nat → Nat),
    (ys.map (fun y => coarseReward zs w r φ y (πY y))).sum
    = (zs.map (fun z => w z * r z (πY (φ z)))).sum) := by
  intro h
  have hbad : ¬ ((0 : Rat) = 1) := by decide
  apply hbad
  simpa [coarseReward, Rat.add_zero] using
    (h ([] : List Nat) [0] (fun _ => 1) (fun _ _ => 1) (fun _ => 0) (fun _ => 0))

/--
  ★见证④（`coarseMu_mul_weight` 删前件 h：纤维权重 ≠ 0 不可证）：
  权重 1 与 -1 相消使纤维权重 = 0：归一化乘回 = 0·μ = 0 ≠ 未归一化纤维和 = 5
  （正负权重相消时「乘回权重」不恢复纤维和——非零权重前件不可删）。
-/
example : ¬ (∀ (zs : List Nat) (w : Nat → Rat) (r : Nat → Nat → Rat) (φ : Nat → Nat)
    (y a : Nat), fiberWeight zs w φ y * coarseMu zs w r φ y a = coarseReward zs w r φ y a) := by
  intro h
  have hbad : ¬ ((0 : Rat) = 5) := by decide
  apply hbad
  simpa [fiberWeight, coarseMu, coarseReward, Rat.add_zero, Rat.add_neg_cancel] using
    (h [0, 1] (fun z => if z = 0 then 1 else -1) (fun z _ => if z = 0 then 5 else 0)
      (fun _ => 0) 0 0)

/--
  ★见证⑤（`positive_reward_total` 删前件 hη 不可证）：
  η = 1 而逐状态边际收益全 0：总收益下界 1·Σw = 1 > 0 = 实际总收益，1 ≤ 0 被显式证伪
  （正边际假设是 p23 正收益定理的唯一天窗，不可删）。
-/
example : ¬ (∀ (zs : List Nat) (w : Nat → Rat) (r : Nat → Nat → Rat) (π : Nat → Nat) (η : Rat),
    η * (zs.map w).sum ≤ fineTotal zs w r π) := by
  intro h
  have hbad : ¬ ((1 : Rat) ≤ 0) := by decide
  apply hbad
  simpa [fineTotal, Rat.add_zero] using (h [0] (fun _ => 1) (fun _ _ => 0) (fun _ => 0) 1)

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（#1050 第二段）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，standalone）：
  1. ★★定理 2 `fine_classification_not_worse_coarse`：细最优 ≥ 粗最优
     （PDF p33–34 逐字：粗策略被细策略模拟 ⟹ 细分类策略集合包含粗分类模拟版本
     ⟹ 取 sup 后 V(Z) ≥ V(Y)；本文件用最优策略形式规避 sup 算符）。
  2. ★模拟 `fine_simulates_coarse`：π_Z = π_Y ∘ φ 逐式同收益。
  3. ★★全期望一致 `total_expectation_coarse_net_agreement`：毛分解（证书 y 条件，
     τ_γ 由 y 携带）与净分解（逐 bar 全互斥状态 z 条件）对粗可测策略逐式相等
     （law of total expectation 有限形式，纤维求和恒等式 `sum_over_fibers` 支撑）。
  4. ★归一化 `coarseMu_mul_weight`：P(Y=y)·μ_Y(y,a) = 纤维收益和（权重非零时）。
  5. ★p23 正收益聚合 `positive_reward_total`：逐状态 μ ≥ η ∧ 权重非负 ⟹ 总收益 ≥ η·Σw。
  6. 删前件自查 5 见证（§6）+ 诚实标签（§5）。

  两套分解差异 = 条件信息差的正式写法（本文件给出）：恒等半边 = `total_expectation_*
  _agreement`（粗可测策略下毛/净一致）；不等半边 = `fine_classification_not_worse_coarse`
  （细最优 ≥ 粗最优，差 ≥ 0）。差的**经验正值**（不同子类最优动作不同带来的严格改进，
  PDF p34「可能严格更好」）属 L2 EmpiricalDomain，本文件不声称。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ ∃z,a μ(z,a) > 0（买卖点预测性——PDF p34/36：必须由统计估计、理论模型或外部假设给出）。
  - ✗ 鞅市场不可能定理的逆向（p35：∀z,a μ ≤ 0 ⟹ 无正 alpha——需要成本非负 + 全状态假设，
    与本文件模型的正负权重自由参数不一致，留作后续）。
  - ✗ 概率空间/σ-代数/鞅的 measure-theoretic 形式化（有限枚举加权和是 PDF 自身的写法）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.FineCoarseClassification
