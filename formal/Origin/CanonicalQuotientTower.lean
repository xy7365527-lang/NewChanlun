/-
  Origin/CanonicalQuotientTower.lean — PDF-only 级别递归的「Can 商化基础」形式化
  （A′ 包补全工位，组A，codex 异质裁决 (b)）

  ── 存在论位置（codex 裁决 (b)：Can 商化是基础，∃! 是推论）────────────────────
  编排者委托 codex 异质裁决「PDF Can 商化路径 vs FULL ∃! 字面唯一路径」的概念分叉，
  裁 (b)：**PDF 的 Can 商化是更严格的基础，FULL 的 ∃! 唯一分解是其推论**。

  本文件忠实形式化 A′ 包三个截图 PDF（pdftotext 全空，视觉读是唯一途径）的级别递归
  公理化的**商化基础**，而非直接声明 ∃!。对照源（结果包视觉读地面真相）：
    - pdf-26p page-6（核心页）：递归核 `U_{n+1} := Can_{Φ_n}(⊔_{m≥3} U_n^m)`、
      最关键项 `∑_{c∈C_n} 1[P_{n,c}(d_n(h))]=1`、商集判据 `|D_n(h)/∼_n|=1`
    - pdf-15p/26p page-7：结合律括号多义性 `(a+b)+c` vs `a+(b+c)` →∼_n 消解、
      压缩总式 `C(h)=Rec(B,f_1,f_2)`, `∀n: |D_n(h)/∼_n|=1`, `∑ 1[P_{n,c}]=1`
    - pdf-6p page-6：同 26p page-6 内容（同源截图）

  PDF 原文（page-6/7 框）「缠论完全分类的最严格整定义」四要素：
    (1) 从最低级别开始的良基递归生成
    (2) 每一级存在且唯一的规范结构和结构等价类
    (3) 每一级类别互斥穷尽
    (4) 新数据到来时整个层级结构唯一因果地递归更新

  ── 本文件做的事（构造商化，推出 ∃!，L0）──────────────────────────────────
  禁止把 `∃!D_ℓ` 当公理直接声明（那是被 codex 裁决否定的路径 (c)）。本文件：
    1. `Decomposition`：一个第 n+1 级结构的「分解」= 一个 ≥3 段的下级结构序列
       （载体 `List U`，对应 PDF `⊔_{m≥3} U_n^m`，`m≥3` 是构成律）。
    2. `LegalDecomp Φ`：合法分解 = 满足 Φ_n 规则 + `m≥3` 段约束的分解
       （Φ_n 在 PDF 是「能否组成上级结构的全部规则」的抽象规则族——忠实参数化，
        不臆造具体规则，避免 no-patch 违规）。
    3. `DecompSetoid`：等价关系 `∼_n`（结合律括号多义性消解）——两个合法分解若
       规范化到同一代表元则 `∼_n` 等价（PDF：`(a+b)+c ∼_n a+(b+c)`）。
    4. `Can_Φ`（`CanonicalForm`）：规范化算子，把任一合法分解送到其等价类代表元
       （PDF `Can_{Φ_n}`，「负责确定唯一分解*」）。满足幂等 + 等价保持。
    5. `QuotientSingleton`：商集判据 `|D_n/∼_n|=1`（所有合法分解互相 `∼_n` 等价）。
    6. **`∃!` 作为 normalized representative theorem**（`canonical_representative_unique`）：
       从 `Can` 商化 + `|D_n/∼_n|=1` **推出**「存在唯一规范代表 d_ℓ 使一切合法分解
       规范化到它」。这是 codex 裁 (b) 核心——∃! 是推论，不是公理。
    7. `CanonicalTower`：`U_n := Can(⊔_{m≥3} U_n^m)` 直积塔（级别递归塔），
       `lift_n` 把第 n 级商化结构升到第 n+1 级。对接 Origin `RecursiveLevelSystem`。

  ── 与 Origin 既有路径的关系（重要：不删 ∃! 路径，给出商化基础）────────────
  Origin `RecursiveLevelSystem`（`TrendCompleteClassification.lean:137`）与 #89/#99
  `composeStep` 走 FULL `∃!D` 字面唯一路径。本文件给出 PDF 的**商化基础**层，并证明
  FULL 的 ∃! 可从商化 + `Can` 推出（`existsUnique_decomp_of_quotient_singleton`）——
  即两条路径在「Can 是恒等选择子（字面唯一）」特例下重合（`literal_unique_is_canonical`）。
  这正是 codex 裁 (b)：FULL ∃! 是 Can 商化在「∼_n 退化为相等」时的推论。

  ── ★契约锚（#239 实写，#236 裁定）：rust 对应物 `rust/src/theta_v0/classifier/
  recursive_tower.rs`——基础层 vs 实例层。本文件是**基础层**（Can 商化基础，∃! 是推论）；
  rust 递归塔是**生产实例层**（窗口化 compose + 坐标传递），其实例层契约锚已由
  `Origin.RecursiveLevelSystem`（①留件）承载。另：RLS 已降级语义近邻（#240 终裁）——
  本文件固定三格 `lift` ≠ 生产动态窗 `compose` lift，勿逐字段对拍。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────
  全部 **L0**（结构 / 定义内蕴，信息增量为零——商集单点 ⟹ ∃! 是纯逻辑推导，
  不依赖任何市场数据）。本文件**不**声称任何 L1+ 经验有效性：Φ_n 规则族是抽象
  参数，`|D_n/∼_n|=1` 在真实行情下是否成立（合法分解是否真的互相等价）是 L2 经验
  问题，本文件只证「若商单点则 ∃! 规范代表」的 L0 蕴含，不证前件经验成立。

  ── 谱系引用 ────────────────────────────────────────────────────────────────
  - 230号「直积退化」：`⊔_{m≥3} U_n^m` 直积塔在 27 配置代数成立、概率度量下退化为
    <3 自由度。本文件标注：直积塔的有效域声明须带 230 号 L2 退化警告——商化 `Can`
    正是**防直积退化误判的唯一机制**（把代数上区分但经验上同一的分解收缩为一个等价类，
    避免把退化自由度当独立配置计数）。
  - OQ-9 扩维消解模式：商化 `Can_Φ` 的「合法分解（rawDecomp）→ 规范代表（legalRep）」
    双层结构正是 OQ-9 的 rawStep/legalStep 双层——入口证书（`LegalDecomp`）+ 正交维度
    （`∼_n` 商）消解「字面多分解 vs 同一结构」矛盾，非事后判违规。

  ── 严格性声明（no-workaround / no-patch / 反膨胀）──────────────────────────
  本文件**零 sorry / 零 admit / 零 axiom**。Φ_n 作为抽象规则族参数化（PDF 未给具体规则，
  忠实保留为参数，不臆造）。`∃!` 严格从 `Can` 商化推出（`canonical_representative_unique`），
  非直接声明。验证：`lake env lean Origin/CanonicalQuotientTower.lean`（禁 lake clean）。

  ── 依赖方向（单向无环）──────────────────────────────────────────────────────
  CanonicalQuotientTower → {Origin.TrendCompleteClassification（RecursiveLevelSystem 接口）,
  Origin.ChanlunElements（仅读，结构常量）}。只读，不改这两个文件。
  root 名：`Origin.CanonicalQuotientTower`（报 Lead 登记 lakefile roots，本工位不改 lakefile）。

  谱系：codex 裁 (b)（Can 商化是基础）→ 本文件（PDF-only 商化级别递归公理化，
        ∃! 作 normalized representative theorem 输出）。
-/

import Origin.TrendCompleteClassification

namespace NewChanlun.Origin.CanonicalQuotient

universe u

/-! ## 1. 分解载体（PDF `⊔_{m≥3} U_n^m` 直积塔的一个元素）

一个第 n+1 级结构的「分解」= 一个下级结构（`U`）的有限序列。PDF 的
`U_{n+1} := Can_{Φ_n}(⊔_{m≥3} U_n^m)`：`⊔_{m≥3} U_n^m` 是「所有 ≥3 段下级序列」
的直和（不定段数 m≥3 的直积幂的并），载体即 `List U`，段数 `= List.length`。 -/

/-- 一个分解 = 下级结构序列（PDF 直积塔 `⊔_{m≥3} U_n^m` 的一个候选元素）。 -/
abbrev Decomposition (U : Type u) := List U

/-- 段数 `m`（PDF：`m ≥ 3` 高级结构至少由三个低级结构构成）。 -/
def Decomposition.segments {U : Type u} (d : Decomposition U) : Nat := d.length

/-! ## 2. Φ_n 规则族 + 合法分解（入口证书，OQ-9 rawDecomp→legalRep 双层）

`Φ_n` 在 PDF page-6 是「这些第 n 级结构能否组成第 n+1 级结构*的**全部规则**」——
一个抽象规则族。忠实参数化为谓词 `Phi : Decomposition U → Prop`，**不臆造**具体
缠论组成律（缠论的「三段重叠中枢」等具体律是 reference 实装层 II 类，非本 PDF 层）。

合法分解 = 满足 Φ_n + `m ≥ 3` 段约束（PDF 构成律）。 -/

/-- Φ_n 规则族：判定一个下级序列能否组成一个上级结构。PDF 抽象「全部规则」的忠实参数化。 -/
abbrev PhiRule (U : Type u) := Decomposition U → Prop

/-- 合法分解谓词：满足 Φ_n 规则 ∧ 段数 ≥ 3（PDF `m ≥ 3` 构成律）。
    这是 OQ-9 的「入口证书」——分解先过证书才进入商化，非事后判违规。 -/
def LegalDecomp {U : Type u} (Φ : PhiRule U) (d : Decomposition U) : Prop :=
  Φ d ∧ d.segments ≥ 3

theorem legalDecomp_segments_ge_three {U : Type u} {Φ : PhiRule U}
    {d : Decomposition U} (h : LegalDecomp Φ d) : d.segments ≥ 3 :=
  h.2

theorem legalDecomp_phi {U : Type u} {Φ : PhiRule U}
    {d : Decomposition U} (h : LegalDecomp Φ d) : Φ d :=
  h.1

/-! ## 3. 规范化算子 `Can_{Φ_n}`（PDF page-6「负责确定唯一分解*」）

PDF page-6/7：「`Can` 规范化划分，负责确定唯一分解*」。结合律括号多义性
`(a+b)+c` vs `a+(b+c)`「字面分解不同，但可以表示同一结构」。

规范化算子 `Can : Decomposition U → Decomposition U` 把任一分解送到一个**代表元**。
PDF 要求 `Can` 至少满足：
  - **幂等**（`Can (Can d) = Can d`）：代表元自身的规范化是自己（确定唯一）。
这是「Can 是良定义的规范化选择子」的最小公理（PDF 未给 Can 的内部算法，忠实抽象）。 -/

/-- 规范化算子 `Can_{Φ_n}` 的接口：一个把分解送到代表元的函数 + 幂等律。
    PDF 「负责确定唯一分解*」= 幂等的规范化选择子。`Φ` 是其参数化的规则族。 -/
structure CanonicalForm (U : Type u) where
  /-- Φ_n 规则族（PDF：组成上级结构的全部规则）。 -/
  Φ : PhiRule U
  /-- 规范化算子 `Can_{Φ_n}`：把任一分解送到等价类代表元。 -/
  can : Decomposition U → Decomposition U
  /-- 幂等：`Can (Can d) = Can d`（PDF「确定唯一分解」——代表元规范化是自身）。 -/
  idem : ∀ d, can (can d) = can d

/-! ## 4. 等价关系 `∼_n`（结合律括号多义性消解，PDF page-6/7）

PDF：「合法分解的结合、重组等价关系下只有一个等价类」「`(a+b)+c ∼_n a+(b+c)`」。
等价关系定义：两个分解 `∼_n` 等价 ⟺ 规范化到同一代表元（`Can d₁ = Can d₂`）。
这是「商化由规范化诱导」——纯结构定义，自反/对称/传递机器可证。 -/

/-- 等价关系 `∼_n`：两个分解规范化到同一代表元则等价（PDF 结合律括号消解）。 -/
def CanonicalForm.equiv {U : Type u} (C : CanonicalForm U)
    (d₁ d₂ : Decomposition U) : Prop :=
  C.can d₁ = C.can d₂

/-- `∼_n` 自反。 -/
theorem CanonicalForm.equiv_refl {U : Type u} (C : CanonicalForm U)
    (d : Decomposition U) : C.equiv d d :=
  rfl

/-- `∼_n` 对称。 -/
theorem CanonicalForm.equiv_symm {U : Type u} (C : CanonicalForm U)
    {d₁ d₂ : Decomposition U} (h : C.equiv d₁ d₂) : C.equiv d₂ d₁ :=
  h.symm

/-- `∼_n` 传递。 -/
theorem CanonicalForm.equiv_trans {U : Type u} (C : CanonicalForm U)
    {d₁ d₂ d₃ : Decomposition U}
    (h₁ : C.equiv d₁ d₂) (h₂ : C.equiv d₂ d₃) : C.equiv d₁ d₃ :=
  h₁.trans h₂

/-- 规范化与原分解 `∼_n` 等价（`Can d ∼_n d`，由幂等推出）。 -/
theorem CanonicalForm.can_equiv_self {U : Type u} (C : CanonicalForm U)
    (d : Decomposition U) : C.equiv (C.can d) d :=
  C.idem d

/-! ## 5. 商集判据 `|D_n/∼_n| = 1`（PDF page-6/7 框）

PDF：`|D_n(h)/∼_n| = 1`——「合法分解在结合、重组等价关系下只有一个等价类」。
形式化：所有合法分解互相 `∼_n` 等价（商集是单点）。 -/

/-- 商集单点判据 `|D_n/∼_n| = 1`：所有合法分解互相 `∼_n` 等价。
    `legal` 是「至少存在一个合法分解」的见证（商集非空，单点不是空集）。 -/
structure QuotientSingleton {U : Type u} (C : CanonicalForm U) where
  /-- 见证分解：商集非空（存在至少一个合法分解）。 -/
  witness : Decomposition U
  /-- 见证合法性。 -/
  witness_legal : LegalDecomp C.Φ witness
  /-- 单点：任意两个合法分解 `∼_n` 等价（PDF `|D_n/∼_n| = 1`）。 -/
  collapse : ∀ d₁ d₂, LegalDecomp C.Φ d₁ → LegalDecomp C.Φ d₂ → C.equiv d₁ d₂

/-! ## 6. ∃! 作为 normalized representative theorem（codex 裁 (b) 核心）

**从 Can 商化 + `|D_n/∼_n| = 1` 推出 `∃!` 规范代表**。
这是 codex 裁决 (b) 的核心：FULL 的 `∃!D_ℓ` 不是公理，是 PDF 商化基础的**推论**。

`canonical_representative_unique`：存在唯一规范代表 `r`（= `Can witness`），使得
**每一个**合法分解 `d` 都规范化到 `r`（`Can d = r`）。这就是「规范代表的存在唯一性」——
PDF「每一级存在且唯一的规范结构」。 -/

/-- **∃! 规范代表定理**（normalized representative theorem，codex 裁 (b)）。

    给定 Can 商化 `C` 与商集单点见证 `Q`（`|D_n/∼_n| = 1`），**推出**：存在唯一规范
    代表 `r`，使得每个合法分解都规范化到 `r`。∃! 在此是**推论**（不是公理）。 -/
theorem canonical_representative_unique {U : Type u}
    (C : CanonicalForm U) (Q : QuotientSingleton C) :
    ExistsUnique (fun r : Decomposition U =>
      C.can Q.witness = r ∧
      (∀ d, LegalDecomp C.Φ d → C.can d = r)) := by
  refine ⟨C.can Q.witness, ⟨rfl, ?_⟩, ?_⟩
  · -- 存在性：每个合法分解都规范化到 `Can witness`。
    intro d hd
    -- d ∼_n witness（商单点）⟹ Can d = Can witness。
    exact Q.collapse d Q.witness hd Q.witness_legal
  · -- 唯一性：任何满足条件的 r 必等于 `Can witness`。
    intro r hr
    exact hr.1.symm

/-- 推论：合法分解的规范代表唯一（任两个合法分解规范化结果相等）。
    这是 PDF「每一级存在且唯一的规范结构」的直接形式（∃! 的简化投影）。 -/
theorem legal_decomp_canonical_form_unique {U : Type u}
    (C : CanonicalForm U) (Q : QuotientSingleton C)
    {d₁ d₂ : Decomposition U}
    (h₁ : LegalDecomp C.Φ d₁) (h₂ : LegalDecomp C.Φ d₂) :
    C.can d₁ = C.can d₂ :=
  Q.collapse d₁ d₂ h₁ h₂

/-! ## 7. FULL `∃!` 路径 = Can 商化的「∼_n 退化为相等」特例（两路径重合证明）

codex 裁 (b)：FULL `∃!D` 字面唯一是 Can 商化在「`Can` 是恒等选择子」时的推论。
当 `Can = id`（字面唯一，`∼_n` 退化为 `=`）时，商单点 ⟺ 字面唯一存在。 -/

/-- 字面唯一规范化（`Can = id`）：FULL `∃!D` 路径对应的 `CanonicalForm`。
    `∼_n` 退化为相等（`d₁ ∼_n d₂ ⟺ d₁ = d₂`，PDF page-7 末「若规定最规范化规则使分解本身唯一」）。 -/
def literalUnique {U : Type u} (Φ : PhiRule U) : CanonicalForm U where
  Φ := Φ
  can := id
  idem := fun _ => rfl

/-- 字面唯一特例下 `∼_n` 退化为相等（PDF page-7：`d ∼_n d' ⟺ d = d'`）。 -/
theorem literal_unique_is_canonical {U : Type u} (Φ : PhiRule U)
    (d₁ d₂ : Decomposition U) :
    (literalUnique Φ).equiv d₁ d₂ ↔ d₁ = d₂ :=
  Iff.rfl

/-- **FULL `∃!` 从 Can 商化推出**：字面唯一特例下，商单点直接给出 `∃!` 合法分解
    本身唯一（不只是规范化结果唯一）。这是 FULL `∃!D_ℓ` 路径 = PDF Can 商化在
    `∼_n` 退化为 `=` 时的推论（codex 裁 (b) 的两路径重合）。 -/
theorem existsUnique_decomp_of_quotient_singleton {U : Type u}
    (Φ : PhiRule U) (Q : QuotientSingleton (literalUnique Φ)) :
    ExistsUnique (fun d : Decomposition U => LegalDecomp Φ d) := by
  refine ⟨Q.witness, Q.witness_legal, ?_⟩
  intro d hd
  -- 字面唯一：Q.collapse 给出 `id d = id witness`，即 `d = witness`。
  have := Q.collapse d Q.witness hd Q.witness_legal
  -- `(literalUnique Φ).equiv d witness` 即 `id d = id witness` 即 `d = witness`。
  simpa [CanonicalForm.equiv, literalUnique] using this

/-! ## 8. 规范化商塔 `U_n := Can(⊔_{m≥3} U_n^m)`（级别递归塔）

PDF page-6 递归核：`U_0 := A`, `U_{n+1} := Can_{Φ_n}(⊔_{m≥3} U_n^m)`。
塔的每一级是「规范化后的合法 ≥3 段下级序列」的载体。`lift` 把第 n 级结构升到第 n+1 级
（取一个合法分解的规范代表）。对接 Origin `RecursiveLevelSystem`。 -/

/-- 规范化商塔：逐级 `CanonicalForm`（每级有自己的 Φ_n 与 Can_{Φ_n}）。
    `State n` 是第 n 级结构类型；`form n` 是第 n 级的规范化算子（作用于第 n 级序列分解）。 -/
structure CanonicalTower where
  /-- 第 n 级结构类型（PDF `U_n`）。`State 0 = A`（最低级原始构件）。 -/
  State : Nat → Type
  /-- 第 n 级的规范化算子 `Can_{Φ_n}`（作用于 `State n` 的序列分解）。 -/
  form : (n : Nat) → CanonicalForm (State n)
  /-- 升级算子：取一个第 n 级合法分解的规范代表，产出第 n+1 级结构。
      PDF：`U_{n+1} := Can_{Φ_n}(⊔_{m≥3} U_n^m)`。 -/
  lift : (n : Nat) → Decomposition (State n) → State (n + 1)
  /-- 升级只消费规范代表：`lift n d` 由 `Can_{Φ_n} d` 决定（规范化前置，防直积退化）。
      这是 230 号「直积退化」的防护——lift 看到的是商化后的代表元，不是原始直积幂的全部配置。 -/
  lift_canonical : ∀ n d, lift n d = lift n ((form n).can d)

/-- 塔的级别递归：从 base 逐级 lift（取规范代表分解升级）。
    `step n` 给出第 n 级到第 n+1 级的一个规范分解选择。 -/
def CanonicalTower.atLevel (T : CanonicalTower)
    (base : T.State 0) (step : (n : Nat) → T.State n → Decomposition (T.State n)) :
    (n : Nat) → T.State n
  | 0 => base
  | n + 1 => T.lift n (step n (T.atLevel base step n))

/-- 塔自相似递归（PDF `a_{n+1} = f_2(a_n)`，Origin `recursive_level_self_similar` 类比）：
    第 n+1 级结构 = lift 第 n 级结构的一个规范分解。 -/
theorem CanonicalTower.atLevel_self_similar (T : CanonicalTower)
    (base : T.State 0) (step : (n : Nat) → T.State n → Decomposition (T.State n))
    (n : Nat) :
    T.atLevel base step (n + 1)
      = T.lift n (step n (T.atLevel base step n)) :=
  rfl

/-- 塔升级的规范化不变性（防直积退化，230 号）：lift 的结果只依赖规范代表，
    不依赖分解的具体括号化（`(a+b)+c` 与 `a+(b+c)` 经 Can 升级到同一上级结构）。 -/
theorem CanonicalTower.lift_bracket_invariant (T : CanonicalTower)
    (n : Nat) (d₁ d₂ : Decomposition (T.State n))
    (h : (T.form n).equiv d₁ d₂) :
    T.lift n d₁ = T.lift n d₂ := by
  rw [T.lift_canonical n d₁, T.lift_canonical n d₂]
  -- h : (form n).can d₁ = (form n).can d₂，故 lift 后相等。
  rw [show (T.form n).can d₁ = (T.form n).can d₂ from h]

/-! ## 9. 对接 Origin `RecursiveLevelSystem`（商化基础 → Origin 递归接口）

把规范化商塔嵌入 Origin `RecursiveLevelSystem`（`State : Nat → Type` + `lift`）。
这把 PDF 的商化级别递归塔表达为 Origin canonical 递归接口的一个实例，使 PDF 路径
与 Origin/#89 FULL 路径在同一接口下可比较（codex 裁 (b)：两路径同接口，商化是基础）。 -/

open NewChanlun.Origin in
/-- 规范化商塔诱导一个 Origin `RecursiveLevelSystem`（给定逐级分解选择 `step`）。
    `State := T.State`，`lift n s := T.lift n (step n s)`（取规范分解升级）。 -/
def CanonicalTower.toRecursiveLevelSystem (T : CanonicalTower)
    (step : (n : Nat) → T.State n → Decomposition (T.State n)) :
    NewChanlun.Origin.RecursiveLevelSystem where
  State := T.State
  lift := fun n s => T.lift n (step n s)

open NewChanlun.Origin in
/-- 商塔的级别递归与 Origin `RecursiveLevelSystem.at` 逐级相等（接口对接无语义漂移，rfl）。 -/
theorem CanonicalTower.toRLS_at_eq (T : CanonicalTower)
    (base : T.State 0) (step : (n : Nat) → T.State n → Decomposition (T.State n))
    (n : Nat) :
    (T.toRecursiveLevelSystem step).at base n = T.atLevel base step n := by
  induction n with
  | zero => rfl
  | succ k ih =>
      show (T.toRecursiveLevelSystem step).lift k
              ((T.toRecursiveLevelSystem step).at base k)
            = T.lift k (step k (T.atLevel base step k))
      rw [ih]
      rfl

open NewChanlun.Origin in
/-- 商塔诱导的 Origin 系统满足 Origin 自相似递归（特化 `recursive_level_self_similar`）。 -/
theorem CanonicalTower.toRLS_self_similar (T : CanonicalTower)
    (base : T.State 0) (step : (n : Nat) → T.State n → Decomposition (T.State n))
    (n : Nat) :
    (T.toRecursiveLevelSystem step).at base (n + 1)
      = (T.toRecursiveLevelSystem step).lift n
          ((T.toRecursiveLevelSystem step).at base n) :=
  NewChanlun.Origin.recursive_level_self_similar (T.toRecursiveLevelSystem step) base n

end NewChanlun.Origin.CanonicalQuotient
