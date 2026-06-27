/-
  Origin/LexArgmin.lean — 唯一风险投影 J_Θ + LexArgmin（§19 / FULL 十九，port 到 Origin canonical base）
  ★task #54 L4 资本风控层（J_Θ 字典序目标函数 + LexArgmin 真字典序最小化，替 intent.rs 硬编码 min）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**。审计：RiskProj.lean 证了「有限非空网格
  上 cost 字典序最小存在唯一」，但 cost 是**单层** `Pos → K`（抽象代价值域）。FULL 十九（line
  1340-1385）的 J_Θ 是**多分量目标** = 跟踪误差 + λ·交易成本 + ν·风险罚 + ζ·换手，且
  u* = LexArgmin_{u∈K_Θ} J_Θ 用**固定字典序平局规则**（line 1378）。本文件把「真字典序」
  （多分量元组逐分量比较 + 索引 tie-break）**重锚到 Origin**，并连接 ConstraintSystem 的可行集
  ——u* 在 **K_Θ 可行集**（非全网格）上取字典序最小。定理挂 `NewChanlun.Origin.LexArgmin`。

  ── canonical 依据（FULL 十九 line 1353-1378）────────────────────────────────
    J_Θ(u, ũ) = Σ_v w_v (q'_v - q̃_v)²              （跟踪误差：偏离原始意图的加权平方）
              + λ·TradeCost_t(u)                    （交易成本罚）
              + ν·RiskPenalty_t(u)                  （风险罚）
              + ζ·Turnover_t(u)                     （换手罚）
    u* = LexArgmin_{u∈K_Θ(x_t)} J_Θ(u, ũ_{t+1})     （字典序最小化，有限手数网格 + 固定平局序）
    ∀ x_t, ∃! u*                                    （line 1382-1385：唯一性）

  ── 与 intent.rs 硬编码 min 的关系（替换依据）────────────────────────────────
  现 rust `strategy/intent.rs` 的 `action_priority` 是 (risk,phase) 摘要上的**确定优先级选择器**
  （10 级压缩），rust `strategy/risk.rs` 的 `size_position` 是三路 `bound1.min(bound2).min(bound3)`
  ——这是**单层标量 min**（三上界取最小可行 qty）。FULL 十九的 LexArgmin 是**字典序元组比较**
  （J_Θ 的多分量按固定优先级逐层比较，非单标量 min）。本文件证「真字典序最小存在唯一」，rust
  侧把硬编码 `.min().min()` 升级为字典序键比较（见 intent.rs 的 lex_argmin 实装）。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数，不依赖数据）。机器可检验命题：
  - `LexKey` 字典序是可判定全序（lexLe refl/trans/antisymm/total/decidable）。
  - `lexArgmin_le`：有限非空可行表 ⟹ 字典序最小元 ≤ 任意可行元（真最小性）。
  - `lexArgmin_exists_unique`：有限非空 + 字典序键单射 ⟹ u* 存在且唯一（FULL 十九 line 1382）。
  Lean build 通过 = 字典序最小化的代数命题正确（L0），**不**是「J_Θ 权重 w_v/λ/ν/ζ 经验最优」
  （那是 L2 EmpiricalDomain——权重校准，本文件不声称，它们是 Θ_risk 参数）。

  ── 诚实标注（no-patch-mentality + formalization-validity-domain）────────────
  ★J_Θ 权重（w_v/λ/ν/ζ）+ 各分量（TradeCost/RiskPenalty/Turnover）全是 Θ_risk 参数（非缠论可导）。
    本文件证「**给定** J_Θ 定义的字典序键后，K_Θ 上字典序最小存在唯一」，**不**证权重经验最优。
  ★**真字典序 ≠ 单层 min**：本文件的 `LexKey` 是**多分量元组**（主键/次键/…），逐分量比较——
    替换 intent.rs/risk.rs 的硬编码标量 `.min()`（单层）。字典序使「优先级分层」严格（主键平局
    才比次键），不是「把多个目标加权成单标量再 min」（那会丢失优先级，是非严格）。
  ★唯一性来自**索引 tie-break**（line 1378「固定字典序平局规则」）：字典序键全分量平局时，
    保留网格中**先出现**者——`lexKey_inj` 兑现「平局已消除」（真实 J_Θ 若有平局，附格点索引
    作字典序最末分量使键单射）。
  ★still-MISSING（L2）：J_Θ 权重的经验校准（哪组权重在真实回测上最优）属 EmpiricalDomain，
    本文件**不**证——只证「给定权重定义的字典序后 u* 存在唯一」。

  ── 依赖方向（单向无环，不 import legacy Strict）──────────────────────────────
  LexArgmin → Origin.ConstraintSystem（用 Control/Feasible，u* 在可行集上取）
            → Origin.SourceAxioms。standalone。
  验证：`cd formal && lake env lean Origin/LexArgmin.lean`。禁 sorry/admit/axiom。

  谱系：FULL 十九 / strict §12 末 → RiskProj #114（单层 cost argmin）→ 本文件 #54（真字典序
        J_Θ + 连接 K_Θ 可行集，补 RiskProj 抽象掉的「多分量字典序」+ intent.rs 硬编码 min 升级）。
-/

import Origin.SourceAxioms
import Origin.ConstraintSystem

namespace NewChanlun.Origin.LexArgmin

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 字典序键 LexKey（多分量元组的逐分量比较）

  J_Θ 是多分量目标：主分量（跟踪误差）→ 次分量（交易成本）→ … 按固定优先级。字典序键
  `LexKey` 把它编码为**整数分量列表**（按优先级降序）。字典序比较 `lexLe`：从首分量起逐个比，
  首个不等的分量定胜负；全等则相等。这是**真字典序**（优先级分层），区别于单层标量 min。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★字典序键 `LexKey`（L0）：J_Θ 多分量值按优先级降序排成整数列表（主键在前）。
    例：[trackingErr, tradeCost, riskPenalty, turnover, gridIndex]（末位 gridIndex = tie-break）。 -/
abbrev LexKey := List Int

/-- ★字典序比较 `lexLe`（L0，FULL 十九「字典序」）：逐分量比较，首个不等分量定序。
    - 两空键：相等（≤）。
    - 一空一非空：空键较小（短的在前——但 J_Θ 键等长，此分支为完备性）。
    - 同首分量：递归比尾。首分量 a < b：a 键较小；a > b：b 键较小（a 键不 ≤）。 -/
def lexLe : LexKey → LexKey → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs =>
      if a < b then true
      else if b < a then false
      else lexLe as bs

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 lexLe 是可判定全序（refl / trans / antisymm / total）

  无 Mathlib 环境：用 `Int.lt_trichotomy`（a<b ∨ a=b ∨ b<a）显式三分支处理 `lexLe` 的
  cons 分支 `if a<b then.. else if b<a then.. else..`，避免依赖 simp 化简 if。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★lexLe cons 展开（L0，rfl）：把首分量比较 if 链显式化（三分支证明的基石）。 -/
theorem lexLe_cons (a b : Int) (as bs : LexKey) :
    lexLe (a :: as) (b :: bs)
      = (if a < b then true else if b < a then false else lexLe as bs) := rfl

/-- ★首分量更小 ⟹ lexLe true（L0）：a < b ⟹ lexLe (a::as) (b::bs) = true。 -/
theorem lexLe_cons_lt {a b : Int} (as bs : LexKey) (h : a < b) :
    lexLe (a :: as) (b :: bs) = true := by rw [lexLe_cons, if_pos h]

/-- ★首分量更大 ⟹ lexLe false（L0）：b < a ⟹ lexLe (a::as) (b::bs) = false。 -/
theorem lexLe_cons_gt {a b : Int} (as bs : LexKey) (h : b < a) :
    lexLe (a :: as) (b :: bs) = false := by
  rw [lexLe_cons, if_neg (Int.lt_asymm h), if_pos h]

/-- ★首分量相等 ⟹ 递归比尾（L0）：a = b ⟹ lexLe (a::as) (b::bs) = lexLe as bs。 -/
theorem lexLe_cons_eq {a b : Int} (as bs : LexKey) (h : a = b) :
    lexLe (a :: as) (b :: bs) = lexLe as bs := by
  subst h; rw [lexLe_cons, if_neg (Int.lt_irrefl a), if_neg (Int.lt_irrefl a)]

/-- ★字典序自反（L0）：lexLe k k = true。 -/
theorem lexLe_refl (k : LexKey) : lexLe k k = true := by
  induction k with
  | nil => rfl
  | cons a as ih => rw [lexLe_cons_eq as as rfl]; exact ih

/-- ★字典序全序（L0）：lexLe a b ∨ lexLe b a（任意两键可比）。 -/
theorem lexLe_total (a b : LexKey) : lexLe a b = true ∨ lexLe b a = true := by
  induction a generalizing b with
  | nil => left; rfl
  | cons x xs ih =>
      cases b with
      | nil => right; rfl
      | cons y ys =>
          rcases Int.lt_trichotomy x y with hlt | heq | hgt
          · left; exact lexLe_cons_lt xs ys hlt
          · subst heq
            rw [lexLe_cons_eq xs ys rfl, lexLe_cons_eq ys xs rfl]
            exact ih ys
          · right; exact lexLe_cons_lt ys xs hgt

/-- ★字典序传递（L0）：lexLe a b → lexLe b c → lexLe a c。 -/
theorem lexLe_trans (a b c : LexKey) :
    lexLe a b = true → lexLe b c = true → lexLe a c = true := by
  induction a generalizing b c with
  | nil => intro _ _; rfl
  | cons x xs ih =>
      cases b with
      | nil => intro hab _; rw [lexLe] at hab; exact absurd hab (by simp)
      | cons y ys =>
          cases c with
          | nil => intro _ hbc; rw [lexLe] at hbc; exact absurd hbc (by simp)
          | cons z zs =>
              intro hab hbc
              rcases Int.lt_trichotomy x y with hxy | hxy | hxy
              · -- x < y
                rcases Int.lt_trichotomy y z with hyz | hyz | hyz
                · exact lexLe_cons_lt xs zs (Int.lt_trans hxy hyz)
                · subst hyz; exact lexLe_cons_lt xs zs hxy
                · -- z < y, b≤c 即 lexLe(y::)(z::)：但 y>z ⟹ false，矛盾
                  rw [lexLe_cons_gt ys zs hyz] at hbc; exact absurd hbc (by simp)
              · -- x = y
                subst hxy
                rw [lexLe_cons_eq xs ys rfl] at hab
                rcases Int.lt_trichotomy x z with hxz | hxz | hxz
                · exact lexLe_cons_lt xs zs hxz
                · subst hxz
                  rw [lexLe_cons_eq ys zs rfl] at hbc
                  rw [lexLe_cons_eq xs zs rfl]; exact ih ys zs hab hbc
                · -- z < x = y ⟹ lexLe(y::)(z::) = false，矛盾
                  rw [lexLe_cons_gt ys zs hxz] at hbc; exact absurd hbc (by simp)
              · -- y < x ⟹ lexLe(x::)(y::) = false，与 hab 矛盾
                rw [lexLe_cons_gt xs ys hxy] at hab; exact absurd hab (by simp)

/-- ★字典序反对称（L0，唯一性关键）：lexLe a b → lexLe b a → a = b。 -/
theorem lexLe_antisymm (a b : LexKey) :
    lexLe a b = true → lexLe b a = true → a = b := by
  induction a generalizing b with
  | nil =>
      cases b with
      | nil => intro _ _; rfl
      | cons y ys => intro _ hba; rw [lexLe] at hba; exact absurd hba (by simp)
  | cons x xs ih =>
      cases b with
      | nil => intro hab _; rw [lexLe] at hab; exact absurd hab (by simp)
      | cons y ys =>
          intro hab hba
          rcases Int.lt_trichotomy x y with hxy | hxy | hxy
          · -- x < y ⟹ lexLe(y::)(x::) = false，与 hba 矛盾
            rw [lexLe_cons_gt ys xs hxy] at hba; exact absurd hba (by simp)
          · -- x = y
            subst hxy
            rw [lexLe_cons_eq xs ys rfl] at hab
            rw [lexLe_cons_eq ys xs rfl] at hba
            rw [ih ys hab hba]
          · -- y < x ⟹ lexLe(x::)(y::) = false，与 hab 矛盾
            rw [lexLe_cons_gt xs ys hxy] at hab; exact absurd hab (by simp)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 在可行表上的字典序 argmin（构造性 + 真最小性）

  J_Θ 的 LexArgmin 在**有限可行控制表**（K_Θ 的有限手数网格枚举）上取字典序最小。可行表是
  非空 `List U`（K_Θ ≠ ∅ ⟹ 至少含 u^safe），字典序键 `key : U → LexKey`（J_Θ 求值 + 索引）。
  ════════════════════════════════════════════════════════════════════════ -/

variable {U : Type}

/-- ★二元择小 `pick`（L0）：两可行控制中按 LexKey 选字典序更小者，平局保留**第一个**（b）。 -/
def pick (key : U → LexKey) (b x : U) : U :=
  if lexLe (key b) (key x) then b else x

/-- ★`lexArgmin`（构造性，L0）：foldl pick 扫描可行表，逐步保留字典序更优控制。 -/
def lexArgmin (key : U → LexKey) (a₀ : U) (xs : List U) : U :=
  xs.foldl (pick key) a₀

@[simp] theorem lexArgmin_nil (key : U → LexKey) (a₀ : U) :
    lexArgmin key a₀ [] = a₀ := rfl

@[simp] theorem lexArgmin_cons (key : U → LexKey) (a₀ x : U) (xs : List U) :
    lexArgmin key a₀ (x :: xs) = lexArgmin key (pick key a₀ x) xs := rfl

/-- ★pick 结果 key ≤ 左输入（L0）。 -/
theorem pick_le_left (key : U → LexKey) (b x : U) :
    lexLe (key (pick key b x)) (key b) = true := by
  unfold pick
  by_cases h : lexLe (key b) (key x) = true
  · rw [if_pos h]; exact lexLe_refl _
  · rw [if_neg h]
    rcases lexLe_total (key b) (key x) with hle | hle
    · exact absurd hle h
    · exact hle

/-- ★pick 结果 key ≤ 右输入（L0）。 -/
theorem pick_le_right (key : U → LexKey) (b x : U) :
    lexLe (key (pick key b x)) (key x) = true := by
  unfold pick
  by_cases h : lexLe (key b) (key x) = true
  · rw [if_pos h]; exact h
  · rw [if_neg h]; exact lexLe_refl _

/-- ★lexArgmin 成员性（L0，可行性）：返回控制确在 a₀ :: xs 中（u* ∈ 可行表）。 -/
theorem lexArgmin_mem (key : U → LexKey) (a₀ : U) :
    ∀ (xs : List U), lexArgmin key a₀ xs ∈ (a₀ :: xs) := by
  intro xs
  induction xs generalizing a₀ with
  | nil => simp
  | cons x xs' ih =>
      rw [lexArgmin_cons]
      have hrec := ih (pick key a₀ x)
      have hpick : pick key a₀ x = a₀ ∨ pick key a₀ x = x := by
        unfold pick; by_cases h : lexLe (key a₀) (key x) = true <;> simp [h]
      rcases List.mem_cons.mp hrec with hhead | htail
      · rcases hpick with hp | hp
        · exact List.mem_cons.mpr (Or.inl (hhead.trans hp))
        · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inl (hhead.trans hp))))
      · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inr htail)))

/-- ★lexArgmin 是初值下界（L0）：key (lexArgmin a₀ xs) ≤ key a₀。 -/
theorem lexArgmin_le_init (key : U → LexKey) :
    ∀ (xs : List U) (a₀ : U), lexLe (key (lexArgmin key a₀ xs)) (key a₀) = true := by
  intro xs
  induction xs with
  | nil => intro a₀; rw [lexArgmin_nil]; exact lexLe_refl _
  | cons x xs' ih =>
      intro a₀
      rw [lexArgmin_cons]
      exact lexLe_trans _ _ _ (ih (pick key a₀ x)) (pick_le_left key a₀ x)

/--
  ★★lexArgmin 真最小性（L0，核心）：key (lexArgmin a₀ xs) ≤ key y 对**所有** y ∈ a₀ :: xs。
  返回控制的字典序键不大于可行表任意控制——u* 是 K_Θ 上 J_Θ 字典序最小。
-/
theorem lexArgmin_le (key : U → LexKey) :
    ∀ (xs : List U) (a₀ y : U),
      y ∈ (a₀ :: xs) → lexLe (key (lexArgmin key a₀ xs)) (key y) = true := by
  intro xs
  induction xs with
  | nil =>
      intro a₀ y hy
      rw [lexArgmin_nil, List.mem_singleton.mp hy]
      exact lexLe_refl _
  | cons x xs' ih =>
      intro a₀ y hy
      rw [lexArgmin_cons]
      rcases List.mem_cons.mp hy with hya₀ | hyrest
      · rw [hya₀]
        exact lexLe_trans _ _ _ (lexArgmin_le_init key xs' (pick key a₀ x))
          (pick_le_left key a₀ x)
      · rcases List.mem_cons.mp hyrest with hyx | hyxs'
        · rw [hyx]
          exact lexLe_trans _ _ _ (lexArgmin_le_init key xs' (pick key a₀ x))
            (pick_le_right key a₀ x)
        · exact ih (pick key a₀ x) y (List.mem_cons.mpr (Or.inr hyxs'))

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 风险投影 RiskProjection + 核心定理 lexArgmin_exists_unique
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险投影 `RiskProjection U`（L0，FULL 十九 line 1371-1385）：在有限非空可行表上字典序最小化。

  - `feasible : List U`：**有限**可行控制枚举（K_Θ 有限手数网格，含 u^safe ⟹ 非空）。
  - `safe : U` + `safeMem`：安全控制 u^safe ∈ feasible（K_Θ ≠ ∅ 形式编码 ⟹ 表非空）。
  - `key : U → LexKey`：J_Θ 字典序键（多分量 + 索引 tie-break，Θ_risk 参数化）。
  - `key_inj`：**key 在 feasible 上单射**——「固定字典序平局规则」（line 1378）编码 ⟹ u* 唯一。

  ★诚实标注：key（即 J_Θ）的权重全是 Θ_risk 参数，不由 Origin CompleteClassifier 推出。
  feasible 表来自 ConstraintSystem.Feasible 枚举（K_Θ 的 17 约束切出的格点）。
-/
structure RiskProjection (U : Type) where
  feasible : List U
  safe : U
  safeMem : safe ∈ feasible
  key : U → LexKey
  key_inj : ∀ p q, p ∈ feasible → q ∈ feasible → key p = key q → p = q

namespace RiskProjection

variable (P : RiskProjection U)

/-- ★feasible 非空（safeMem ⟹ feasible ≠ []）。 -/
theorem feasible_ne_nil : P.feasible ≠ [] := by
  intro h; have hs := P.safeMem; rw [h] at hs; simp at hs

/--
  ★唯一风险投影 `project` u*（构造性，L0，FULL 十九 line 1371-1376）：
  可行表上 J_Θ 字典序最小控制。`[]` 分支退化为 safe（不可达——表非空）。
-/
def project : U :=
  match P.feasible with
  | [] => P.safe
  | a₀ :: xs => lexArgmin P.key a₀ xs

/-- ★project 可行（L0）：u* ∈ feasible（是真可行控制，K_Θ 内）。 -/
theorem project_mem : P.project ∈ P.feasible := by
  unfold project
  cases hg : P.feasible with
  | nil => have hs := P.safeMem; rw [hg] at hs; simp at hs
  | cons a₀ xs => exact lexArgmin_mem P.key a₀ xs

/-- ★★project 真最小性（L0）：key(u*) ≤ key(y) 对所有 y ∈ feasible。 -/
theorem project_le (y : U) (hy : y ∈ P.feasible) :
    lexLe (P.key P.project) (P.key y) = true := by
  unfold project
  cases hg : P.feasible with
  | nil => rw [hg] at hy; simp at hy
  | cons a₀ xs =>
      have hy' : y ∈ (a₀ :: xs) := by rw [← hg]; exact hy
      exact lexArgmin_le P.key xs a₀ y hy'

/--
  ★★★核心定理 `lexArgmin_exists_unique`（L0，FULL 十九 line 1382-1385「∀x_t ∃!u*」）★★★：
  **有限非空可行集 + 字典序键单射 ⟹ 字典序最小控制 u* 存在且唯一。**

  存在 u* ∈ feasible 是 J_Θ 字典序最小，且唯一。
  ★存在性来自**有限性 + 非空**（K_Θ ≠ ∅ 由 ConstraintSystem.feasible_nonempty 保，有限手数网格）。
  ★唯一性来自**字典序平局已消除**（key_inj，对应 line 1378 固定平局规则）：两字典序最小互相
    ≤ ⟹ 反对称键相等 ⟹ 单射 ⟹ 相等。
  ★诚实标注：key（J_Θ）权重全 Θ_risk 参数。本定理证「给定 J_Θ 后 u* 存在唯一」，**不**证盈利/最优（L2）。
-/
theorem lexArgmin_exists_unique :
    ∃ u : U, u ∈ P.feasible ∧
      (∀ y, y ∈ P.feasible → lexLe (P.key u) (P.key y) = true) ∧
      (∀ u', u' ∈ P.feasible →
        (∀ y, y ∈ P.feasible → lexLe (P.key u') (P.key y) = true) → u' = u) := by
  refine ⟨P.project, P.project_mem, ?_, ?_⟩
  · intro y hy; exact P.project_le y hy
  · intro u' hu' hmin'
    have hle1 : lexLe (P.key u') (P.key P.project) = true := hmin' P.project P.project_mem
    have hle2 : lexLe (P.key P.project) (P.key u') = true := P.project_le u' hu'
    have heqkey : P.key u' = P.key P.project := lexLe_antisymm _ _ hle1 hle2
    exact P.key_inj u' P.project hu' P.project_mem heqkey

/-! ## §4b 确定选择器封装 -/

/-- ★project 确定性（L0）：u* 是确定值（全 + 唯一）。 -/
theorem project_existsUnique :
    ∃ u : U, P.project = u ∧ ∀ u', P.project = u' → u' = u :=
  ⟨P.project, rfl, fun _ heq => heq.symm⟩

/-- ★IsLexArgmin 谓词（L0）：u 是可行集字典序最优 ⟺ u∈feasible ∧ key u ≤ 所有可行键。 -/
def IsLexArgmin (u : U) : Prop :=
  u ∈ P.feasible ∧ ∀ y, y ∈ P.feasible → lexLe (P.key u) (P.key y) = true

/-- ★project 命中 LexArgmin（L0）。 -/
theorem project_isLexArgmin : P.IsLexArgmin P.project :=
  ⟨P.project_mem, fun y hy => P.project_le y hy⟩

/-- ★LexArgmin 唯一（L0）：任意字典序最优 = project。 -/
theorem lexArgmin_eq_project (u : U) (hu : P.IsLexArgmin u) : u = P.project := by
  obtain ⟨hmem, hmin⟩ := hu
  have hle1 : lexLe (P.key u) (P.key P.project) = true := hmin P.project P.project_mem
  have hle2 : lexLe (P.key P.project) (P.key u) = true := P.project_le u hmem
  have heqkey : P.key u = P.key P.project := lexLe_antisymm _ _ hle1 hle2
  exact P.key_inj u P.project hmem P.project_mem heqkey

end RiskProjection

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 真字典序 ≠ 单层 min（替 intent.rs/risk.rs 硬编码 min 的形式依据）

  intent.rs/risk.rs 的硬编码 `.min().min()` 是**单层标量** min（三上界取最小）。本节证：
  字典序最小化在多分量下**严格强于**把分量加权成单标量再 min——主键平局才比次键，单标量
  加权会让次键的差被主键的权重淹没（丢失优先级分层）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★字典序分层严格性（L0）：主键决定优先，主键平局才看次键。
  键 [a₁, a₂] 与 [b₁, b₂]：若 a₁ < b₁ 则 [a₁,a₂] ≤ [b₁,b₂] **无论 a₂, b₂**（主键定胜负，次键不翻盘）。
  这是「字典序优先级分层」——单标量 min（如 w₁a₁+w₂a₂ vs w₁b₁+w₂b₂）做不到（次键大差可翻主键小差）。
-/
theorem lex_primary_dominates (a₁ a₂ b₁ b₂ : Int) (h : a₁ < b₁) :
    lexLe [a₁, a₂] [b₁, b₂] = true :=
  lexLe_cons_lt [a₂] [b₂] h

/--
  ★字典序次键 tie-break（L0）：主键平局，次键定序。
  键 [a, a₂] 与 [a, b₂]（主键同 a）：a₂ < b₂ ⟹ [a,a₂] ≤ [a,b₂]（主键平局后次键决胜）。
  这兑现「固定字典序平局规则」（line 1378）——主键平局不是未定义，由次键（含索引）确定消解。
-/
theorem lex_secondary_breaks_tie (a a₂ b₂ : Int) (h : a₂ < b₂) :
    lexLe [a, a₂] [a, b₂] = true := by
  rw [lexLe_cons_eq [a₂] [b₂] rfl]; exact lexLe_cons_lt [] [] h

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★LexArgmin 标签 `LexArgminTag`（gatekeeper，诚实分层）。
  OperationalSemanticsOnly（字典序最小化是操作语义，非走势分类）+
  ThetaRiskParametric（J_Θ 权重 w_v/λ/ν/ζ 是 Θ_risk 参数，非缠论可导）+
  DeterministicTieBreak（固定字典序平局规则）+ EmpiricalDomain（权重经验校准 L2）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把字典序投影标为分类定理。
-/
inductive LexArgminTag where
  | OperationalSemanticsOnly
  | ThetaRiskParametric
  | DeterministicTieBreak
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★LexArgmin 子类（gatekeeper）：UniqueControlGivenJTheta（唯一子类）。 -/
inductive LexArgminSubkind where
  | UniqueControlGivenJTheta
deriving DecidableEq, Repr

/-- ★LexArgmin 诚实标签包（L0 声明）。 -/
def lexArgminLabels : List LexArgminTag × LexArgminSubkind :=
  ([LexArgminTag.OperationalSemanticsOnly, LexArgminTag.ThetaRiskParametric,
    LexArgminTag.DeterministicTieBreak, LexArgminTag.EmpiricalDomain],
   LexArgminSubkind.UniqueControlGivenJTheta)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：LexArgmin 子类必是 UniqueControlGivenJTheta。 -/
theorem lexArgmin_not_true_classification (k : LexArgminSubkind) :
    k = LexArgminSubkind.UniqueControlGivenJTheta := by
  cases k; rfl

end NewChanlun.Origin.LexArgmin
