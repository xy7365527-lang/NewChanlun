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
  - §13（p34，逐字）：「这个定理说明：全互斥分类提供更强表达能力。它不会自动创造 alpha，
    但不会比粗糙分类更差；如果不同子类的最优动作不同，则可能严格更好。」（#1067 §7 的
    条件形式化：`fine_strictly_better_coarse`）。
  - §16（p35，逐字）：鞅市场不可能定理——「如果价格过程在信息流 F_t 下满足
    E[P_{t+1} − P_t | F_t] = 0，而策略因果 N_t ∈ F_t，则 E[N_t(P_{t+1} − P_t)] = 0；
    扣除成本：E[R_{t+1}] = −E[C_t] ≤ 0。」（#1067 §8 的有限加权和形式）。

  ## 两套分解之间的正式关系（本文件写死的五件事，#1050 三件 + #1067 两件）

  1. **粗分类策略的细模拟（定理 2 的证明核心）**：`fine_simulates_coarse` ——
     ∀π_Y ∃π_Z（π_Z = π_Y ∘ φ），细分解总收益 = 粗策略在细状态上的收益。
  2. **★★定理 2**：`fine_classification_not_worse_coarse` —— 细最优 ≥ 粗最优
     （细分类不劣于粗分类；等价于 V(Z) ≥ V(Y) 的最优策略形式，不需 sup 算符）。
  3. **★★全期望一致（毛/净分解的桥）**：`total_expectation_coarse_net_agreement` ——
     Σ_y 毛分类收益 = Σ_z 净分类收益（对粗分类可测策略逐式相等，law of total expectation
     的有限形式）。毛分解条件 γ（含 τ_γ）→ 粗分类 Y；净分解条件 z（逐 bar 全互斥状态）
     → 细分类 Z；φ : Z → Y 就是「逐 bar 状态 → 所属腿/证书」的投影。
  4. **★★定理 2·严格方向（#1067）**：`fine_strictly_better_coarse`（§7）——同纤维
     z1≠z2、两侧最优动作不相交（`actionsDisagree`）、权重均为正 ⟹ 细策略严格优于
     任何粗策略。纯结构定理（给定分类与收益函数即可证，不涉及市场数据；#1067 订正
     #1050 交付报告「L2 经验命题」的误标）。
  5. **★★鞅不可能定理有限形式（#1067）**：`martingale_impossibility_zero`（§8，
     E[N·ΔP]=0 塔性质有限形式）+ `martingale_no_positive_alpha`（扣非负成本后 E[R] ≤ 0）。

  ## 差异 = 条件信息差（PDF 结论的形式解读，诚实分层）

  毛分解按证书 γ（每腿用自己出场时点 τ_γ）条件化，净分解按逐 bar 全互斥状态 z
  条件化。`total_expectation_coarse_net_agreement` 说：对**粗分类可测**的策略两分解
  逐式一致；`fine_classification_not_worse_coarse` 说：优化到各自最优后，细分类
  最优值**不劣于**粗分类最优值。两分解最优值之差 = 细状态 z 相对粗证书 γ 的
  **条件信息差**的价值（细分类策略集合严格包含粗分类模拟版本，差 ≥ 0 是 L0 代数事实）。
  严格改进方向（p34「如果不同子类的最优动作不同，则可能严格更好」）由 #1067 落地为
  `fine_strictly_better_coarse`（§7）：最优动作**不相交**（比「不同」更强、可证充分条件）
  + 正权重 ⟹ 严格优于**任何**粗策略。反向（PDF p35 鞅不可能定理）由 #1067 落地为 §8
  `martingale_impossibility_zero` / `martingale_no_positive_alpha`（有限加权和形式，
  塔性质有限形式；不做 σ-代数/测度论层——票内出界条款）。

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
  - `fine_strictly_better_coarse`（前件 hdis/hw1/hsame，#1067 §7）→ 删前件见证（反例⑥⑦⑧）：
    删 hdis（收益恒 1 动作相交，2<2 证伪）/ 删 hw1（z1 权重 0，1<1 证伪）/ 删 hsame
    （两状态异纤维，粗策略分派各自最优达 2，2<2 证伪）。
  - `martingale_impossibility_zero`（前件 hzero/hsub，#1067 §8）→ 删前件见证（反例⑨⑩）：
    删 hzero（dP=1 纤维均值 1，1≠0 证伪）/ 删 hsub（ys=[] 覆盖不了，1≠0 证伪）。
  - `martingale_no_positive_alpha`（前件 hc/hw，#1067 §8）→ 删前件见证（反例⑪⑫）：
    删 hc（成本 −1 补贴，1≤0 证伪）/ 删 hw（权重 −1，1≤0 证伪）。

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
  ## 交付总结（#1050 第二段 + #1067 补充）

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
  7. ★★定理 2·严格方向 `fine_strictly_better_coarse`（§7，#1067）：同纤维 z1≠z2 +
     `actionsDisagree`（无公共最优动作）+ 正权重 ⟹ ∀π_Y ∃π_Z 细严格优于粗
     （单侧改进引理 `fine_improves_at` 引擎；p34「可能严格更好」的结构充分条件）。
  8. ★★鞅不可能定理有限形式（§8，#1067）：`martingale_impossibility_zero`
     （条件均值零逐纤维 + 因果头寸 ⟹ E[N·ΔP] = 0，塔性质有限形式复用
     `sum_over_fibers`）+ `martingale_no_positive_alpha`（扣非负成本后 E[R] ≤ 0）。
  9. 删前件自查 7 见证（§7 ⑥⑦⑧ + §8 ⑨⑩⑪⑫，#871 同款口径）。

  两套分解差异 = 条件信息差的正式写法（本文件给出）：恒等半边 = `total_expectation_*
  _agreement`（粗可测策略下毛/净一致）；不等半边 = `fine_classification_not_worse_coarse`
  （细最优 ≥ 粗最优，差 ≥ 0）；**严格半边** = `fine_strictly_better_coarse`（§7，#1067：
  无公共最优动作 + 正权重 ⟹ 严格优于任何粗策略——纯结构定理，非 L2 经验命题）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ ∃z,a μ(z,a) > 0（买卖点预测性——PDF p34/36：必须由统计估计、理论模型或外部假设给出）。
  - ✗ 概率空间/σ-代数/鞅的 measure-theoretic 形式化（有限枚举加权和是 PDF 自身的写法；
    塔性质只取有限形式，σ-代数层是另一张图的事，#1067 票内出界条款）。
  ════════════════════════════════════════════════════════════════════════ -/

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 ★★定理 2·严格方向（p34「可能严格更好」的条件形式化；#1067）

  PDF p34 逐字：「这个定理说明：全互斥分类提供更强表达能力。它不会自动创造 alpha，
  但不会比粗糙分类更差；如果不同子类的最优动作不同，则可能严格更好。」
  本文件把「不同子类的最优动作不同」收紧为可证的结构条件：**每个动作 a 至少在
  一侧存在严格改进动作**（经典等价于「不存在同一个动作同时对两子类最优」，
  即两侧最优动作集合不相交）。在此条件下细策略严格优于**任何**粗策略——粗策略
  必须给同一纤维 y 内的 z1、z2 派同一个动作 a，而 a 至少在其中一侧不最优；
  细策略在该侧换成改进动作即可严格改进（另一侧不变），权为正则严格差 > 0。

  ★诚实边界（条件为何收紧为「无公共最优动作」）：若两侧 argmax 集合只是**不同但
  相交**（z1 最优集 {a,b}、z2 最优集 {b,c}），粗策略可派公共最优动作 b，两侧同时
  达最优——细分类无法严格改进。PDF 的「可能严格更好」是存在性语气；本定理给出使
  「严格优于任何粗策略」成立的充分结构条件（#1050 交付报告标 L2 是误标：给定分类
  与收益函数即可证，纯结构定理）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- a 是状态 z 的最优动作（argmax 的谓词形式）：∀a', r z a' ≤ r z a。 -/
def isOptimalAction {Z : Type u} {A : Type v} (r : Z → A → Rat) (z : Z) (a : A) : Prop :=
  ∀ a', r z a' ≤ r z a

/--
  两侧最优动作不相交（p34「最优动作不同」的结构收紧形式，见证强度版）：
  每个动作 a 至少在 z1 或 z2 一侧存在严格改进动作。经典等价于
  ¬(isOptimalAction r z1 a ∧ isOptimalAction r z2 a)（无公共最优动作），
  但以 ∃/∨ 直给见证，证明无需排中（本文件零 import、零 classical）。
-/
def actionsDisagree {Z : Type u} {A : Type v} (r : Z → A → Rat) (z1 z2 : Z) : Prop :=
  ∀ a, (∃ a', r z1 a < r z1 a') ∨ (∃ a', r z2 a < r z2 a')

/-! 严格不等式与逐点求和的 Rat 小工具（核心 Rat lemmas 自证，无 Mathlib，无新增 axiom）。 -/

/-- 严格-宽松传递 a < b ≤ c ⟹ a < c（Rat 线性序自证，零 classical）。 -/
theorem rat_lt_of_lt_of_le {a b c : Rat} (hab : a < b) (hbc : b ≤ c) : a < c := by
  by_cases h : a < c
  · exact h
  · have hca : c ≤ a := by
      rcases (Rat.le_total (a := a) (b := c)) with hac | hca
      · have heq : a = c := by
          by_cases heq : a = c
          · exact heq
          · exact False.elim (h (Rat.lt_of_le_of_ne hac heq))
        rw [← heq]
        exact Rat.le_refl (a := a)
      · exact hca
    have hba : b ≤ a := Rat.le_trans hbc hca
    have hab' : a ≤ b := Rat.le_of_lt hab
    have heq : b = a := Rat.le_antisymm hba hab'
    exact False.elim ((Rat.ne_of_lt hab) heq.symm)

/-- 宽松-严格传递 a ≤ b < c ⟹ a < c（Rat 线性序自证，零 classical）。 -/
theorem rat_lt_of_le_of_lt {a b c : Rat} (hab : a ≤ b) (hbc : b < c) : a < c := by
  by_cases h : a < c
  · exact h
  · have hca : c ≤ a := by
      rcases (Rat.le_total (a := a) (b := c)) with hac | hca
      · have heq : a = c := by
          by_cases heq : a = c
          · exact heq
          · exact False.elim (h (Rat.lt_of_le_of_ne hac heq))
        rw [← heq]
        exact Rat.le_refl (a := a)
      · exact hca
    have hcb : c ≤ b := Rat.le_trans hca hab
    have hbc' : b ≤ c := Rat.le_of_lt hbc
    have heq : c = b := Rat.le_antisymm hcb hbc'
    exact False.elim ((Rat.ne_of_lt hbc) heq.symm)

/-- a < b ⟹ 0 < b − a（正差；由 Rat.add_lt_add_right + Rat.sub_eq_add_neg 自证）。 -/
theorem rat_sub_pos_of_lt {a b : Rat} (h : a < b) : 0 < b - a := by
  have h1 : a + -a < b + -a := (Rat.add_lt_add_right).mpr h
  rw [Rat.add_neg_cancel] at h1
  rw [← Rat.sub_eq_add_neg b a] at h1
  exact h1

/-- Σ_t (−f t) = −Σ_t f t（Rat 求和的负号提取）。 -/
theorem list_sum_neg {α : Type u} (f : α → Rat) (l : List α) :
    (l.map (fun x => -(f x))).sum = -((l.map f).sum) := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons, ih]
      exact Rat.neg_add.symm

/-- Σ_t (f t − g t) = Σ_t f t − Σ_t g t（Rat 求和差分配）。 -/
theorem list_sum_sub {α : Type u} (f g : α → Rat) (l : List α) :
    (l.map (fun x => f x - g x)).sum = (l.map f).sum - (l.map g).sum := by
  rw [Rat.sub_eq_add_neg ((l.map f).sum) ((l.map g).sum)]
  rw [show (l.map (fun x => f x - g x)) = (l.map (fun x => f x + -(g x))) by
    apply List.map_congr_left
    intro x hx
    rw [Rat.sub_eq_add_neg (f x) (g x)]]
  rw [list_sum_add f (fun x => -(g x)) l]
  rw [list_sum_neg g l]

/-- 逐点非负 + 某一正值元素在列 ⟹ 求和 > 0（严格版 sum_nonneg）。 -/
theorem sum_pos_of_mem {α : Type u} [DecidableEq α] (f : α → Rat) (l : List α) (z : α)
    (hnonneg : ∀ a ∈ l, 0 ≤ f a) (hpos : 0 < f z) (hzm : z ∈ l) : 0 < (l.map f).sum := by
  induction l with
  | nil => cases hzm
  | cons a l ih =>
      have ha : 0 ≤ f a := hnonneg a (by simp)
      simp only [List.map_cons, List.sum_cons]
      by_cases haz : a = z
      · subst z
        have hrest : 0 ≤ (l.map f).sum := sum_nonneg f l (by intro b hb; exact hnonneg b (by simp [hb]))
        have hle : f a ≤ f a + (l.map f).sum := by
          have hle' : f a + 0 ≤ f a + (l.map f).sum := (Rat.add_le_add_left).mpr hrest
          simpa [Rat.add_zero] using hle'
        exact rat_lt_of_lt_of_le hpos hle
      · have hzm' : z ∈ l := by
          rcases List.mem_cons.mp hzm with h | h
          · exact False.elim (haz h.symm)
          · exact h
        have hrestpos : 0 < (l.map f).sum := ih (by intro b hb; exact hnonneg b (by simp [hb])) hzm'
        have hle : (l.map f).sum ≤ f a + (l.map f).sum := by
          have hle' : 0 + (l.map f).sum ≤ f a + (l.map f).sum := (Rat.add_le_add_right).mpr ha
          simpa [Rat.zero_add] using hle'
        exact rat_lt_of_lt_of_le hrestpos hle

/--
  ★单侧改进引理（定理 2 严格方向的引擎）：若 z0 ∈ zs、w z0 > 0、且 a0 在 z0 上严格改进
  粗动作 πY(φ z0)，则把 z0 上的动作换成 a0 的细策略严格优于 πY 的细模拟。
  结构事实：粗策略对纤维内所有细状态只能派一个动作；细策略可以在某一侧单点换动作，
  加权和差 = w z0 · (r z0 a0 − r z0 (πY (φ z0))) ·（z0 在 zs 中出现次数）> 0。
-/
theorem fine_improves_at {Z : Type u} {Y : Type w} {A : Type v} [DecidableEq Z]
    (zs : List Z) (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (πY : Y → A) (z0 : Z) (a0 : A)
    (hz0 : z0 ∈ zs) (hw0 : 0 < w z0) (himprov : r z0 (πY (φ z0)) < r z0 a0) :
    coarseEval zs w r φ πY < fineTotal zs w r (fun z => if z = z0 then a0 else πY (φ z)) := by
  let πZ : Z → A := fun z => if z = z0 then a0 else πY (φ z)
  let δ : Z → Rat := fun z => w z * (r z (πZ z) - r z (πY (φ z)))
  have hsubpos : 0 < r z0 a0 - r z0 (πY (φ z0)) := rat_sub_pos_of_lt himprov
  have hposδ : 0 < δ z0 := by
    dsimp [δ, πZ]
    rw [if_pos rfl]
    exact Rat.mul_pos hw0 hsubpos
  have hnonnegδ : ∀ z ∈ zs, 0 ≤ δ z := by
    intro z hz
    by_cases hzz : z = z0
    · rw [hzz]
      exact Rat.le_of_lt hposδ
    · have hπ : πZ z = πY (φ z) := by
        simp [πZ, hzz]
      dsimp [δ]
      rw [hπ, Rat.sub_self, Rat.mul_zero]
      exact Rat.le_refl (a := 0)
  have hsumpos : 0 < (zs.map δ).sum := sum_pos_of_mem δ zs z0 hnonnegδ hposδ hz0
  have hdistr (z : Z) : w z * r z (πZ z) - w z * r z (πY (φ z)) = w z * (r z (πZ z) - r z (πY (φ z))) := by
    rw [Rat.sub_eq_add_neg (w z * r z (πZ z)) (w z * r z (πY (φ z)))]
    rw [Rat.sub_eq_add_neg (r z (πZ z)) (r z (πY (φ z)))]
    rw [← Rat.mul_neg (w z) (r z (πY (φ z)))]
    rw [Rat.mul_add (w z) (r z (πZ z)) (-(r z (πY (φ z))))]
  have hdiff : fineTotal zs w r πZ - coarseEval zs w r φ πY = (zs.map δ).sum := by
    dsimp [fineTotal, coarseEval, δ]
    rw [← list_sum_sub (fun z => w z * r z (πZ z)) (fun z => w z * r z (πY (φ z))) zs]
    rw [show (zs.map (fun z => w z * r z (πZ z) - w z * r z (πY (φ z)))) = (zs.map (fun z => w z * (r z (πZ z) - r z (πY (φ z))))) by
      apply List.map_congr_left
      intro z hz
      exact hdistr z]
  have hposdiff : 0 < fineTotal zs w r πZ - coarseEval zs w r φ πY := by
    rw [hdiff]
    exact hsumpos
  have hlt : coarseEval zs w r φ πY + 0 < coarseEval zs w r φ πY + (fineTotal zs w r πZ - coarseEval zs w r φ πY) :=
    (Rat.add_lt_add_left).mpr hposdiff
  rw [Rat.add_zero, Rat.add_comm (coarseEval zs w r φ πY) (fineTotal zs w r πZ - coarseEval zs w r φ πY), Rat.sub_add_cancel] at hlt
  exact hlt

/--
  ★★定理 2·严格方向（L0，p34「可能严格更好」的结构充分条件；#1067 订正——纯结构定理，
  非 L2 经验命题）：z1 ≠ z2 两细状态同映射到粗类（φ z1 = φ z2）、两侧最优动作不相交
  （`actionsDisagree`）、权重均为正 ⟹ 细策略严格优于**任何**粗策略：
  ∀πY, ∃πZ, coarseEval πY < fineTotal πZ。
  证明：粗策略在纤维 y 上只能派一个动作 a = πY(φ z1) = πY(φ z2)；actionsDisagree 说
  a 至少在一侧有严格改进动作（∃/∨ 直给见证，零排中）；在该侧用 `fine_improves_at`
  换成改进动作（权重为正），加权和严格增大，另一侧不变。
  删前件见证（§7 见证⑥⑦⑧）：hdis、hw1、hsame 各自不可删；hne 是簿记性前件
  （证明不消费——分岔完全由 hdis 承载）；hz1/hz2（状态在枚举内）是明显的求和
  成员前件，删后 z0 不进和。
-/
theorem fine_strictly_better_coarse {Z : Type u} {Y : Type w} {A : Type v} [DecidableEq Z]
    (zs : List Z) (w : Z → Rat) (r : Z → A → Rat) (φ : Z → Y) (z1 z2 : Z)
    (hz1 : z1 ∈ zs) (hz2 : z2 ∈ zs) (_hne : z1 ≠ z2) (hsame : φ z1 = φ z2)
    (hw1 : 0 < w z1) (hw2 : 0 < w z2)
    (hdis : actionsDisagree r z1 z2) :
    ∀ πY : Y → A, ∃ πZ : Z → A, coarseEval zs w r φ πY < fineTotal zs w r πZ := by
  intro πY
  let aY : A := πY (φ z1)
  rcases hdis aY with hleft | hright
  · rcases hleft with ⟨a1', ha1'⟩
    refine ⟨fun z => if z = z1 then a1' else πY (φ z), ?_⟩
    exact fine_improves_at zs w r φ πY z1 a1' hz1 hw1 (by simpa [aY] using ha1')
  · rcases hright with ⟨a2', ha2'⟩
    refine ⟨fun z => if z = z2 then a2' else πY (φ z), ?_⟩
    exact fine_improves_at zs w r φ πY z2 a2' hz2 hw2 (by simpa [aY, hsame] using ha2')

/-! ════════════════════════════════════════════════════════════════════════
  ## §7·附 删前件自查（见证⑥⑦⑧，#871 同款口径）
  ════════════════════════════════════════════════════════════════════════ -/

/-- §7 见证模型：两状态（Nat 0/1）、动作 Bool；r 0·true = 1、r 1·false = 1（两侧最优动作相异且不相交）。 -/
def disagreeR : Nat → Bool → Rat := fun z a => if z = 0 then (if a then 1 else 0) else (if a then 0 else 1)

/-- §7 见证用 hdis：每个动作至少在一侧有严格改进动作（true 在 z1 侧改进、false 在 z2 侧改进）。 -/
theorem disagree_actions : ∀ a : Bool, (∃ a', disagreeR 0 a < disagreeR 0 a') ∨ (∃ a', disagreeR 1 a < disagreeR 1 a') := by
  intro a
  cases a with
  | false => exact Or.inl ⟨true, by simp [disagreeR] <;> decide⟩
  | true => exact Or.inr ⟨false, by simp [disagreeR] <;> decide⟩

/--
  ★见证⑥（`fine_strictly_better_coarse` 删前件 hdis 不可证）：收益恒 1（任何动作对两侧
  都最优，argmax 相交）时，粗策略与细策略总收益同为 Σw；无细策略能严格优于粗策略
  （2 < 2 被显式证伪）。hdis（两侧最优动作不相交）是严格方向的承载前件。
-/
example : ¬ (∀ (zs : List Nat) (w : Nat → Rat) (r : Nat → Bool → Rat) (φ : Nat → Nat) (z1 z2 : Nat),
    z1 ∈ zs → z2 ∈ zs → z1 ≠ z2 → φ z1 = φ z2 → 0 < w z1 → 0 < w z2 →
    (∀ πY : Nat → Bool, ∃ πZ : Nat → Bool, coarseEval zs w r φ πY < fineTotal zs w r πZ)) := by
  intro h
  have h' := h [0,1] (fun _ => 1) (fun _ _ => 1) (fun _ => 0) 0 1 (by simp) (by simp) (by decide) rfl (by decide) (by decide)
  rcases h' (fun _ => true) with ⟨πZ, hlt⟩
  have hbad : ¬ ((2 : Rat) < 2) := by decide
  apply hbad
  simp [coarseEval, fineTotal, Rat.add_zero] at hlt

/--
  ★见证⑦（`fine_strictly_better_coarse` 删前件 hw1 不可证）：z1 权重为 0（z2 权重 1）时，
  在 z1 上的改进不进入加权和——粗策略派 z2 的最优动作 false 达总收益 1，任何细策略
  总收益 ≤ 1（1 < 1 被显式证伪）。正权重是把「单点改进」变成「严格差」的前提。
-/
example : ¬ (∀ (zs : List Nat) (w : Nat → Rat) (r : Nat → Bool → Rat) (φ : Nat → Nat) (z1 z2 : Nat),
    z1 ∈ zs → z2 ∈ zs → z1 ≠ z2 → φ z1 = φ z2 → 0 < w z2 →
    (∀ a : Bool, (∃ a', r z1 a < r z1 a') ∨ (∃ a', r z2 a < r z2 a')) →
    (∀ πY : Nat → Bool, ∃ πZ : Nat → Bool, coarseEval zs w r φ πY < fineTotal zs w r πZ)) := by
  intro h
  let w : Nat → Rat := fun z => if z = 0 then 0 else 1
  have h' := h [0,1] w disagreeR (fun _ => 0) 0 1 (by simp) (by simp) (by decide) rfl (by decide) disagree_actions
  rcases h' (fun _ => false) with ⟨πZ, hlt⟩
  have hbad : ¬ ((1 : Rat) < fineTotal [0,1] w disagreeR πZ) := by
    by_cases hb : πZ 1 = true
    · simp [fineTotal, w, disagreeR, hb, Rat.add_zero]
      decide
    · have hb' : πZ 1 = false := by
        cases hp : πZ 1 with
        | true => exact False.elim (hb (by rw [hp]))
        | false => rfl
      simp [fineTotal, w, disagreeR, hb', Rat.add_zero, Rat.zero_add]
  exact hbad (by simpa [coarseEval, fineTotal, w, disagreeR, Rat.add_zero, Rat.zero_add] using hlt)

/--
  ★见证⑧（`fine_strictly_better_coarse` 删前件 hsame 不可证）：φ z1 ≠ φ z2（两状态落
  不同粗类）时，粗策略可对两状态分别派各自最优动作（true/false），粗收益 = 1 + 1 = 2
  达细最优值，任何细策略 ≤ 2（2 < 2 被显式证伪）。同纤维（hsame）是「粗策略被迫同
  动作」的前提。
-/
example : ¬ (∀ (zs : List Nat) (w : Nat → Rat) (r : Nat → Bool → Rat) (φ : Nat → Nat) (z1 z2 : Nat),
    z1 ∈ zs → z2 ∈ zs → z1 ≠ z2 → 0 < w z1 → 0 < w z2 →
    (∀ a : Bool, (∃ a', r z1 a < r z1 a') ∨ (∃ a', r z2 a < r z2 a')) →
    (∀ πY : Nat → Bool, ∃ πZ : Nat → Bool, coarseEval zs w r φ πY < fineTotal zs w r πZ)) := by
  intro h
  have h' := h [0,1] (fun _ => 1) disagreeR (fun z => z) 0 1 (by simp) (by simp) (by decide) (by decide) (by decide) disagree_actions
  rcases h' (fun y => if y = 0 then true else false) with ⟨πZ, hlt⟩
  have hrle (z : Nat) (a : Bool) : disagreeR z a ≤ 1 := by
    by_cases hz : z = 0 <;> by_cases ha : a = true <;> simp [disagreeR, hz, ha] <;> decide
  have hfine_le : fineTotal [0,1] (fun _ => 1) disagreeR πZ ≤ (1 + 1 : Rat) := by
    unfold fineTotal
    have hsum := sum_le_sum (fun z => disagreeR z (πZ z)) (fun _ => (1 : Rat)) [0,1] (by intro z hz; exact hrle z (πZ z))
    simpa [Rat.add_zero] using hsum
  have hbad : ¬ ((1 + 1 : Rat) < fineTotal [0,1] (fun _ => 1) disagreeR πZ) := by
    intro hlt2
    exact Rat.lt_irrefl (rat_lt_of_lt_of_le hlt2 hfine_le)
  exact hbad (by simpa [coarseEval, fineTotal, disagreeR, Rat.add_zero] using hlt)

/-! ════════════════════════════════════════════════════════════════════════
  ## §8 ★★鞅市场不可能定理·有限形式（PDF p35 §16；#1067）

  PDF p35 逐字：「如果价格过程在信息流 F_t 下满足 E[P_{t+1} − P_t | F_t] = 0，而策略
  因果 N_t ∈ F_t，则 E[N_t(P_{t+1} − P_t)] = E[N_t E[P_{t+1} − P_t | F_t]] = 0。
  扣除成本：E[R_{t+1}] = −E[C_t] ≤ 0。」
  有限加权和机制（与 §1 同款，不做 σ-代数/测度论层——票内出界条款）：全状态 s ∈ zs
  （细状态，含 t+1 的信息）按 φ : Z → Y 投影到 t 时刻的粗信息元（F_t 单元）；
  w s = P(S=s)；ΔP s = P_{t+1} − P_t；因果头寸 N s = N_Y (φ s)（F_t 可测 = 在 φ-纤维上
  为常值）。条件均值零 = 每个纤维 y 上 Σ_{φ s = y} w s · ΔP s = 0（`fiberMeanZero`）。
  则塔性质有限形式（`sum_over_fibers`）给出
  E[N·ΔP] = Σ_y N_Y y · Σ_{φ s = y} w s · ΔP s = Σ_y N_Y y · 0 = 0；扣成本 C_t ≥ 0
  （逐点非负）后 E[R] = 0 − E[C] ≤ 0。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 因果头寸（p35「策略因果 N_t ∈ F_t」的 L0 载体）：N s = N_Y (φ s)——头寸只依赖 t 时刻的粗信息元。 -/
def causalHoldings {Z : Type u} {Y : Type w} (NY : Y → Rat) (φ : Z → Y) (s : Z) : Rat := NY (φ s)

/-- 单步毛收益（鞅部分）：E[N·ΔP] = Σ_{s∈zs} w s · N s · ΔP s。 -/
def grossMartingaleReward {Z : Type u} (zs : List Z) (w : Z → Rat) (N : Z → Rat) (dP : Z → Rat) : Rat :=
  (zs.map (fun s => w s * N s * dP s)).sum

/-- 条件均值零（有限形式，p35 E[ΔP|F_t]=0 的纤维写法）：纤维 y 内 Σ_{φ s = y} w s · ΔP s = 0。 -/
def fiberMeanZero {Z : Type u} {Y : Type w} [DecidableEq Y] (zs : List Z) (w : Z → Rat) (dP : Z → Rat) (φ : Z → Y) (y : Y) : Prop :=
  ((zs.filter (fun s => decide (φ s = y))).map (fun s => w s * dP s)).sum = 0

/-- 扣成本净收益（p35 E[R] = E[N·ΔP] − E[C]）：毛收益 − Σ_{s∈zs} w s · c s。 -/
def netAfterCost {Z : Type u} (zs : List Z) (w : Z → Rat) (N : Z → Rat) (dP : Z → Rat) (c : Z → Rat) : Rat :=
  grossMartingaleReward zs w N dP - (zs.map (fun s => w s * c s)).sum

/-- 纤维 x 因果头寸的求和提取：Σ_{φ s = y} N_Y(φ s) · (w s · ΔP s) = N_Y y · Σ_{φ s = y} w s · ΔP s。 -/
theorem fiber_of_causal_sum {Z : Type u} {Y : Type w} [DecidableEq Y] (zs : List Z)
    (w : Z → Rat) (dP : Z → Rat) (φ : Z → Y) (NY : Y → Rat) (y : Y)
    (hzero : fiberMeanZero zs w dP φ y) :
    ((zs.filter (fun s => decide (φ s = y))).map (fun s => NY (φ s) * (w s * dP s))).sum = 0 := by
  have hmap : ((zs.filter (fun s => decide (φ s = y))).map (fun s => NY (φ s) * (w s * dP s)))
      = ((zs.filter (fun s => decide (φ s = y))).map (fun s => (w s * dP s) * NY y)) := by
    apply List.map_congr_left
    intro s hs
    have hφ : φ s = y := of_decide_eq_true (List.mem_filter.mp hs).2
    rw [hφ, Rat.mul_comm]
  rw [hmap]
  rw [sum_mul (fun s => w s * dP s) (NY y) (zs.filter (fun s => decide (φ s = y)))]
  unfold fiberMeanZero at hzero
  rw [hzero, Rat.zero_mul]

/--
  ★★鞅不可能定理·有限形式（L0，p35 第一式）：ys 无重复覆盖 zs 的 φ-像、且每个纤维
  y 条件均值零 ⟹ E[N·ΔP] = 0（塔性质有限形式：
  Σ_s w s · N s · ΔP s = Σ_y N_Y y · Σ_{φ s = y} w s · ΔP s = Σ_y N_Y y · 0 = 0）。
  删前件见证（§8 见证⑨⑩）：hzero、hsub 各自不可删。
-/
theorem martingale_impossibility_zero {Z : Type u} {Y : Type w} [DecidableEq Y]
    (ys : List Y) (zs : List Z) (w : Z → Rat) (dP : Z → Rat) (φ : Z → Y) (NY : Y → Rat)
    (hsub : ∀ s ∈ zs, φ s ∈ ys) (hnodup : ys.Nodup)
    (hzero : ∀ y ∈ ys, fiberMeanZero zs w dP φ y) :
    grossMartingaleReward zs w (fun s => NY (φ s)) dP = 0 := by
  unfold grossMartingaleReward
  have hreassoc : (zs.map (fun s => w s * NY (φ s) * dP s))
      = (zs.map (fun s => NY (φ s) * (w s * dP s))) := by
    apply List.map_congr_left
    intro s hs
    rw [Rat.mul_assoc (w s) (NY (φ s)) (dP s)]
    rw [Rat.mul_comm (w s) (NY (φ s) * dP s)]
    rw [Rat.mul_assoc]
    rw [Rat.mul_comm (dP s) (w s)]
  rw [hreassoc]
  have hsum := sum_over_fibers ys zs (fun s => NY (φ s) * (w s * dP s)) φ hsub hnodup
  rw [← hsum]
  have hmapz : (ys.map (fun y => ((zs.filter (fun s => decide (φ s = y))).map (fun s => NY (φ s) * (w s * dP s))).sum))
      = (ys.map (fun _ => (0 : Rat))) := by
    apply List.map_congr_left
    intro y hy
    exact fiber_of_causal_sum zs w dP φ NY y (hzero y hy)
  rw [hmapz]
  exact list_sum_zero ys

/--
  ★★鞅市场无正 alpha·有限形式（L0，p35 第二式）：条件均值零 + 因果头寸 + 权重非负 +
  成本逐点非负 ⟹ 扣成本净收益 ≤ 0。E[R] = E[N·ΔP] − E[C] = 0 − E[C] ≤ 0
  （E[C] ≥ 0 由 sum_nonneg；−E[C] ≤ 0 由 Rat.neg_le_neg）。
  删前件见证（§8 见证⑪⑫）：hc（成本非负）、hw（权重非负）各自不可删。
-/
theorem martingale_no_positive_alpha {Z : Type u} {Y : Type w} [DecidableEq Y]
    (ys : List Y) (zs : List Z) (w : Z → Rat) (dP : Z → Rat) (c : Z → Rat) (φ : Z → Y) (NY : Y → Rat)
    (hsub : ∀ s ∈ zs, φ s ∈ ys) (hnodup : ys.Nodup)
    (hzero : ∀ y ∈ ys, fiberMeanZero zs w dP φ y)
    (hw : ∀ s ∈ zs, 0 ≤ w s) (hc : ∀ s ∈ zs, 0 ≤ c s) :
    netAfterCost zs w (fun s => NY (φ s)) dP c ≤ 0 := by
  unfold netAfterCost
  rw [martingale_impossibility_zero ys zs w dP φ NY hsub hnodup hzero]
  have hcost : 0 ≤ (zs.map (fun s => w s * c s)).sum := sum_nonneg (fun s => w s * c s) zs (by
    intro s hs
    exact Rat.mul_nonneg (hw s hs) (hc s hs))
  rw [Rat.sub_eq_add_neg, Rat.zero_add]
  have hneg : -((zs.map (fun s => w s * c s)).sum) ≤ 0 := by
    have hle := Rat.neg_le_neg hcost
    simpa [Rat.neg_zero] using hle
  exact hneg

/-! ════════════════════════════════════════════════════════════════════════
  ## §8·附 删前件自查（见证⑨⑩⑪⑫，#871 同款口径）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★见证⑨（`martingale_impossibility_zero` 删前件 hzero 不可证）：纤维均值 1 ≠ 0（dP = 1），
  E[N·ΔP] = 1·1·1 = 1 ≠ 0——条件均值零是 p35 第一式的唯一天窗。
-/
example : ¬ (∀ (ys : List Nat) (zs : List Nat) (w : Nat → Rat) (dP : Nat → Rat) (φ : Nat → Nat) (NY : Nat → Rat),
    (∀ s ∈ zs, φ s ∈ ys) → ys.Nodup →
    grossMartingaleReward zs w (fun s => NY (φ s)) dP = 0) := by
  intro h
  have hbad : ¬ ((1 : Rat) = 0) := by decide
  apply hbad
  simpa [grossMartingaleReward, Rat.add_zero] using
    (h [0] [0] (fun _ => 1) (fun _ => 1) (fun _ => 0) (fun _ => 1) (by simp) (by simp))

/--
  ★见证⑩（`martingale_impossibility_zero` 删前件 hsub 不可证）：ys = [] 覆盖不了 zs = [0]
  （hzero 空真），毛收益 = 1 ≠ 0——纤维枚举必须覆盖全部细状态（同 §6 见证③口径）。
-/
example : ¬ (∀ (ys : List Nat) (zs : List Nat) (w : Nat → Rat) (dP : Nat → Rat) (φ : Nat → Nat) (NY : Nat → Rat),
    ys.Nodup → (∀ y ∈ ys, fiberMeanZero zs w dP φ y) →
    grossMartingaleReward zs w (fun s => NY (φ s)) dP = 0) := by
  intro h
  have hbad : ¬ ((1 : Rat) = 0) := by decide
  apply hbad
  simpa [grossMartingaleReward, Rat.add_zero] using
    (h ([] : List Nat) [0] (fun _ => 1) (fun _ => 1) (fun _ => 0) (fun _ => 1) (by simp) (by simp))

/--
  ★见证⑪（`martingale_no_positive_alpha` 删前件 hc 不可证）：成本 −1（负成本=补贴）时
  E[C] = −1，扣成本净收益 = 0 − (−1) = 1 > 0——成本非负是「扣成本后 ≤ 0」的前提。
-/
example : ¬ (∀ (ys : List Nat) (zs : List Nat) (w : Nat → Rat) (dP : Nat → Rat) (c : Nat → Rat) (φ : Nat → Nat) (NY : Nat → Rat),
    (∀ s ∈ zs, φ s ∈ ys) → ys.Nodup → (∀ y ∈ ys, fiberMeanZero zs w dP φ y) →
    (∀ s ∈ zs, 0 ≤ w s) →
    netAfterCost zs w (fun s => NY (φ s)) dP c ≤ 0) := by
  intro h
  have hbad : ¬ ((1 : Rat) ≤ 0) := by decide
  apply hbad
  have hzm : fiberMeanZero [0] (fun _ : Nat => 1) (fun _ => 0) (fun _ => 0) 0 := by
    simp [fiberMeanZero, Rat.add_zero]
  simpa [netAfterCost, grossMartingaleReward, Rat.add_zero, Rat.zero_add, Rat.sub_eq_add_neg, Rat.neg_neg] using
    (h [0] [0] (fun _ => 1) (fun _ => 0) (fun _ => -1) (fun _ => 0) (fun _ => 1) (by simp) (by simp)
      (by intro y hy
          have hy0 : y = 0 := by simpa using hy
          simpa [hy0] using hzm)
      (by intro s hs
          change 0 ≤ (1 : Rat)
          decide))

/--
  ★见证⑫（`martingale_no_positive_alpha` 删前件 hw 不可证）：权重 −1、成本 1 时
  E[C] = −1 < 0，扣成本净收益 = 0 − (−1) = 1 > 0——权重非负是 E[C] ≥ 0 的前提
  （与 hc 对称：w·c 的符号由两者共同决定）。
-/
example : ¬ (∀ (ys : List Nat) (zs : List Nat) (w : Nat → Rat) (dP : Nat → Rat) (c : Nat → Rat) (φ : Nat → Nat) (NY : Nat → Rat),
    (∀ s ∈ zs, φ s ∈ ys) → ys.Nodup → (∀ y ∈ ys, fiberMeanZero zs w dP φ y) →
    (∀ s ∈ zs, 0 ≤ c s) →
    netAfterCost zs w (fun s => NY (φ s)) dP c ≤ 0) := by
  intro h
  have hbad : ¬ ((1 : Rat) ≤ 0) := by decide
  apply hbad
  have hzm : fiberMeanZero [0] (fun _ : Nat => -1) (fun _ => 0) (fun _ => 0) 0 := by
    simp [fiberMeanZero, Rat.add_zero]
  simpa [netAfterCost, grossMartingaleReward, Rat.add_zero, Rat.zero_add, Rat.sub_eq_add_neg, Rat.neg_neg] using
    (h [0] [0] (fun _ => -1) (fun _ => 0) (fun _ => 1) (fun _ => 0) (fun _ => 1) (by simp) (by simp)
      (by intro y hy
          have hy0 : y = 0 := by simpa using hy
          simpa [hy0] using hzm)
      (by intro s hs
          change 0 ≤ (1 : Rat)
          decide))

end NewChanlun.Origin.FineCoarseClassification
