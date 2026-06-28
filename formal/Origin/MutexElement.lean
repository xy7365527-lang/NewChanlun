/-
  Origin/MutexElement.lean — MW1 缠论元素五元组 + 级别关系标签 ρ_e（角色互斥分类链的根）

  ── 存在论位置（M04/M05，互斥分类 canonical spec §B 表 M04/M05 行 / §A 页2-3 §2）────────
  本文件形式化 **M04 元素五元组** + **M05 三种级别关系**：在 W7 C27 四元组
    e_C27 = (I_e, ε_e, ℓ_e, par(e))                       （`Origin.SyntaxElement`）
  基础上 **精化** 为缠论元素五元组
    e = (I_e, ℓ_e, ε_e, α_e, ρ_e)
  - α_e（`attached : Option SyntaxElement`）：操作语义中的 **依附对象**（该元素依附的上级
    声部/元素）。根级元素 α_e = ∅（none），非根元素携依附对象（some）。这是 M04 把 C27 的
    `par(e)`（父元素标识 `Option Nat`）**一般化** 为「依附对象」——依附对象可为 ∅（独立做多/
    做空根级），非根时携带上级元素本身（以便比较级别 ℓ_{α_e}）。
  - ρ_e（`LevelRelation`，Root/Same/Sub 三值标签）：元素相对其依附对象 α_e 的 **级别关系**
    （M05）。**不是独立硬编码字段**，而是由 α_e + 级别比较 `ℓ_e vs ℓ_{α_e}` **函数派生**
    （`levelRelation`）——这样 Root/Same/Sub 三分的「互斥穷尽」是结构定理（`levelRelation_*`
    全谱系定理），而非约定。派生规则（M05/M06/M07/M08）：
        α_e = ∅           ⟹ ρ_e = Root  （M06：根级，对应独立做多/做空）
        α_e = some p, ℓ_e = ℓ_p ⟹ ρ_e = Same  （M07：同级别，ℓ_e = ℓ_{α_e}）
        α_e = some p, ℓ_e < ℓ_p ⟹ ρ_e = Sub   （M08：次级别，ℓ_e < ℓ_{α_e}）
        α_e = some p, ℓ_e > ℓ_p ⟹ ρ_e = Same  （越级反向归并到同/上级语义——见 § 1 NB）

  ── 核心边界（mutex spec §E(b) 警示）：Same 在完全分类声部树中无位置，但在绝对级别坐标有位置 ──
  spec §E(b) 警示：**Same（同级别）在完全分类声部树（C16–C24，只父/子嵌套）无对应位置**。
  本文件的边界处理（no-workaround 严格区分）：
  - C27 的 `level : Nat` 是元素的 **绝对级别标量**（深度坐标，最低级笔 level=0，越高越粗）。
  - ρ_e（Root/Same/Sub）是元素 **相对依附对象的级别关系标签**——`ℓ_e` 与 `ℓ_{α_e}` 的 **比较**
    （=, <），不是绝对级别本身。
  - **关键判定**：Same 由 `ℓ_e = ℓ_{α_e}`（两元素 level 相等）在 **绝对级别坐标系** 中严格定义，
    完全有位置（不依赖声部树的父/子归属维度）。`Origin.RecursiveLevelSystem`（lift: n→n+1，
    **绝对级别提升**）只提供绝对级别坐标，**不约束** 同级别 vs 次级别的相对关系——故 Same 的
    `ℓ_e = ℓ_{α_e}` 与 RecursiveLevelSystem 的级别定义 **不冲突**（绝对坐标系下 level 相等是
    合法状态）。spec §E(b) 的「Same 无位置」仅指 **声部树覆盖归属维度**（C16–C24 只处理父/子
    嵌套），不指绝对级别坐标——本文件在绝对 `level : Nat` 上定义 ρ_e，Root/Same/Sub 三分
    互斥穷尽成立，**与 RecursiveLevelSystem 无定义冲突**（详见 § 3 冲突核对定理）。

  本文件 Root/Same/Sub 三值标签 **互斥穷尽**（`LevelRelation` 是三构造子归纳类型 + `DecidableEq`
  + `levelRelation_exhaustive`/`*_mutex_*` 全谱系）——为 MW2 级别关系判定 + MW4 角色互斥穷尽
  （M12 Σ指示=1）铺垫：角色 Role(e) 四分类（RootDir/SameDir/SubFollow/ShortDiff）的级别侧
  三分基底由本文件的 ρ_e 提供。

  ── owner（互斥铁律）─────────────────────────────────────────────────────────────
  **只建** 本文件（不扩展 `SyntaxElement.lean`——共享文件 owner 冲突）。import `Origin.SyntaxElement`
  复用其四元组（`SyntaxElement`/`ElementSet`/`Direction.sign`/`dirSign`/`level`/`isRoot`），
  新增 α_e/ρ_e 两维。不碰其他文件、不编辑 lakefile.toml。root 名 `Origin.MutexElement`。

  ── 认识论等级（formalization-validity-domain 231号强制）─────────────────────────
  全部 **L0**（纯结构定义 / 级别关系标签 = 纯组合）。五元组是 spec §A 页1-2 散文定义的忠实转录，
  ρ_e 三分是 `ℓ_e` 与 `ℓ_{α_e}` 的 Nat 比较的代数派生，互斥穷尽是三构造子归纳类型的结构必然。
  **信息增量为零的同义反复**（L0）：级别关系分类在「构造的元素集」上成立 ≠ 在真实市场有效。
  本文件 **不** 声称任何 L1+ 经验有效性——「操作语义必须唯一判定级别关系」（spec §A 页2）是
  内部操作语义假设，非现实市场假设（有效域 ⊊ 定义域，呼应 C15/C27 W7 同款标注）。

  ── 依赖方向（单向无环）+ 验证 ─────────────────────────────────────────────────
  MutexElement → Origin.SyntaxElement（W7，C27 四元组复用）→ {Origin.ChanlunElements, ...}。
  不碰其他文件（owner 互斥）。验证：`cd formal && lake env lean Origin/MutexElement.lean`。
  禁 sorry/admit/axiom。简体中文。

  谱系：M04/M05（互斥分类 PDF §1-§2 元素五元组 + 级别关系三分）→ MW1（角色互斥分类链的根）→
        下游 MW2（Root/Same/Sub 判定）/ MW3（角色四分类）/ MW4（M12 互斥穷尽 Σ指示=1）。
-/

import Origin.SyntaxElement

namespace NewChanlun.Origin

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 级别关系标签 `ρ_e ∈ {Root, Same, Sub}`（M05，三值互斥穷尽归纳类型）

    spec §A 页2 §2：`ρ_e ∈ {Root, Same, Sub}` 操作语义必须对每个元素 **唯一判定** 其相对
    依附对象 α_e 的级别关系。本节定义 `LevelRelation` 为三构造子归纳类型——三分的「互斥穷尽」
    由归纳类型的结构必然保证（每个值恰是三构造子之一，无第四种、无重叠），配 `DecidableEq`
    使「唯一判定」可机器判定。这是 MW2 级别关系判定 + MW4 角色互斥穷尽（M12）的级别侧基底。

    ★NB（越级 ℓ_e > ℓ_{α_e} 的归并，no-workaround 诚实声明）：spec M06/M07/M08 只显式给出
    α_e=∅⟹Root、ℓ_e=ℓ_{α_e}⟹Same、ℓ_e<ℓ_{α_e}⟹Sub 三情形（依附语义中元素只可能依附 **同级
    或更高级** 的对象，故 ℓ_e ≤ ℓ_{α_e} 是操作语义的内蕴约束）。对 ℓ_e > ℓ_{α_e}（元素级别
    **高于** 其依附对象）——这在「依附 = 依附上级」的语义下不应出现，但 Nat 比较在定义域上是
    全函数（必须对所有 (ℓ_e, ℓ_p) 给值，否则 `levelRelation` 非全函数）。本文件把该越界情形
    **归并为 Same**（非次级别——次级别严格要求 ℓ_e < ℓ_{α_e}，M08），并提供谓词 `WellAttached`
    （ℓ_e ≤ ℓ_{α_e}）标注「操作语义合法依附」的有效域，在 `WellAttached` 下 Same ⟺ ℓ_e = ℓ_{α_e}
    精确成立（`levelRelation_same_iff_of_wellAttached`）。这 **不是补丁**——是把全函数的定义域
    （所有 Nat 对）与操作语义有效域（ℓ_e ≤ ℓ_{α_e}）严格分离（231号有效域≠定义域）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **级别关系标签 ρ_e（M05，spec §A 页2 §2）** —— 元素相对其依附对象 α_e 的级别关系，三值穷尽：
  - `root`：根级（M06，α_e = ∅，对应独立做多/做空，无依附对象）。
  - `same`：同级别（M07，ℓ_e = ℓ_{α_e}，同向=延续/滚动，反向=同级别关闭/反向/重建——**仍不是短差**）。
  - `sub`：次级别（M08，ℓ_e < ℓ_{α_e}，同向=次级别顺势腿，反向=短差元素）。

  ★三构造子归纳类型 ⟹ 互斥穷尽是结构必然（每值恰一构造子）。`DecidableEq` ⟹ 唯一判定可机器化
  （spec「操作语义必须唯一判定」）。这是 MW4 角色互斥穷尽（M12 Σ指示=1）的级别侧基底。
  ★L0：三值标签是 spec §2 散文「三种级别关系」的结构转录，无 Θ 参数、不依赖数据。
-/
inductive LevelRelation where
  /-- 根级（M06）：α_e = ∅，独立做多/做空。 -/
  | root
  /-- 同级别（M07）：ℓ_e = ℓ_{α_e}，同层延续或反向切换（非短差）。 -/
  | same
  /-- 次级别（M08）：ℓ_e < ℓ_{α_e}，次级别顺势腿或短差。 -/
  | sub
deriving DecidableEq, Repr

namespace LevelRelation

/-- ★三值穷尽（L0，归纳类型结构必然）：任一 ρ_e 必是 root/same/sub 之一。坐实 spec M05
    「级别关系三分穷尽」——无第四种级别关系（操作语义必须唯一判定，且只在三值中判定）。 -/
theorem exhaustive (r : LevelRelation) : r = root ∨ r = same ∨ r = sub := by
  cases r
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/-- ★root ≠ same（L0，互斥分量）：根级与同级别两两不相交（完备性「互斥」之一支）。 -/
theorem root_ne_same : root ≠ same := by decide

/-- ★root ≠ sub（L0，互斥分量）：根级与次级别两两不相交。 -/
theorem root_ne_sub : root ≠ sub := by decide

/-- ★same ≠ sub（L0，互斥分量）：同级别与次级别两两不相交（M07/M08 严格区分——M25「同级别
    反向≠短差」的级别侧根据：Same 的 ℓ_e=ℓ_{α_e} 与 Sub 的 ℓ_e<ℓ_{α_e} 不可同时成立）。 -/
theorem same_ne_sub : same ≠ sub := by decide

end LevelRelation

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 缠论元素五元组 `e = (I_e, ℓ_e, ε_e, α_e, ρ_e)`（M04，C27 四元组精化）

    在 W7 `SyntaxElement`（C27 四元组 I_e/ε_e/ℓ_e/par）基础上精化：新增 `attached`（α_e 依附
    对象）。ρ_e **不作为字段**，而由 `levelRelation`（α_e + 级别比较）派生——三分互斥穷尽是
    派生函数的结构定理（`levelRelation_*` 全谱系），不是约定。这避免「字段 ρ_e 与 (α_e, ℓ_e)
    不一致」的可能矛盾（单一真相源：ρ_e 永远 = (α_e, ℓ_e) 的函数）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **缠论元素五元组（M04，互斥分类 spec §A 页1-2 §1）** —— `e = (I_e, ℓ_e, ε_e, α_e, ρ_e)`，
  在 W7 C27 四元组 `SyntaxElement` 基础上 **精化** 新增依附对象 α_e。

  - `core : SyntaxElement`：复用 C27 四元组（I_e=[λ_e,ρ_e)、ε_e 方向、ℓ_e 级别、par 父标识）。
    **不重定义** 四元组——直接嵌入 W7 已 GREEN 的 `SyntaxElement`（owner 互斥：本文件只加 α_e）。
  - `attached : Option SyntaxElement`：依附对象 α_e（M04）。**根级 = none**（M06，独立做多/做空），
    **非根 = some p**（p 为依附的上级元素，携其 level 以判定 ρ_e）。这是 M04 把 C27 `par(e)`
    （`Option Nat` 父标识）一般化为「依附对象」——依附对象本身（非仅标识）便于比较 ℓ_{α_e}。

  ★ρ_e 不是字段：ρ_e 由 `levelRelation e`（依 attached + core.level）函数派生（§ 1 NB），
  保证「ρ_e = (α_e, ℓ_e) 的唯一函数」（单一真相源，无字段一致性矛盾）。
  ★M04 vs C27：C27 四元组（I,ε,ℓ,par）；M04 五元组加 α_e 依附对象（+ ρ_e 派生）。C27 是 M04 在
  「不区分依附级别关系」的特例——M04 是 C27 的真精化（加 α_e 维 + ρ_e 派生维），精化非冲突。
  ★L0：五元组是 spec §1 散文定义的结构转录，无 Θ 参数、不依赖数据。
-/
structure MutexElement where
  /-- C27 四元组核心（I_e/ε_e/ℓ_e/par，复用 W7 `SyntaxElement`，不重定义）。 -/
  core : SyntaxElement
  /-- 依附对象 α_e（M04）：根级 none（独立做多/做空），非根 some p（依附的上级元素）。 -/
  attached : Option SyntaxElement
deriving Repr

namespace MutexElement

/-- ★元素级别 ℓ_e（M04，透传 C27 `core.level`）。 -/
def level (e : MutexElement) : Nat := e.core.level

/-- ★元素方向符号 ε_e ∈ {+1,-1}（M04，透传 C27 `core.dirSign`）。下游 MW3 Side(e)=ε_e 的方向。 -/
def dirSign (e : MutexElement) : Int := e.core.dirSign

/-- ★依附对象级别 ℓ_{α_e}（M07/M08 比较用）：根级（attached=none）记 ℓ_{α_e} = ℓ_e（占位，
    根级不参与 Same/Sub 判定——见 `levelRelation` 先判 none⟹Root）；非根取依附对象 level。 -/
def attachedLevel (e : MutexElement) : Nat :=
  match e.attached with
  | none => e.level
  | some p => p.level

/--
  **级别关系派生函数 ρ_e = levelRelation(e)（M05/M06/M07/M08）** —— 从依附对象 α_e + 级别比较
  唯一派生 ρ_e（单一真相源，非硬编码字段）：
  - `attached = none`        ⟹ `root`（M06：根级，独立做多/做空）。
  - `attached = some p, ℓ_e < ℓ_p` ⟹ `sub`（M08：次级别，ℓ_e < ℓ_{α_e}）。
  - `attached = some p, ¬(ℓ_e < ℓ_p)`（即 ℓ_e ≥ ℓ_p）⟹ `same`（M07：同级别 ℓ_e=ℓ_{α_e}；
    越级 ℓ_e>ℓ_p 在 `WellAttached` 有效域外，归并 Same——§ 1 NB，非次级别因 Sub 严格要 ℓ_e<ℓ_p）。

  ★全函数（定义域 = 所有 (attached, ℓ_e) 对）：Nat 比较 `ℓ_e < ℓ_p` 可判定，对所有输入给值。
  ★唯一判定（spec §2「操作语义必须唯一判定」）：函数输出确定 ⟹ 每元素 ρ_e 唯一。 -/
def levelRelation (e : MutexElement) : LevelRelation :=
  match e.attached with
  | none => LevelRelation.root
  | some p => if e.level < p.level then LevelRelation.sub else LevelRelation.same

/-- ★合法依附谓词 `WellAttached`（操作语义有效域，§ 1 NB / 231号）：依附对象级别 ≥ 元素级别
    （ℓ_e ≤ ℓ_{α_e}）——元素只依附 **同级或更高级** 对象（操作语义内蕴约束）。根级（none）
    平凡满足。`WellAttached` 划出「同级或次级别」的有效域，越级 ℓ_e>ℓ_{α_e} 在有效域外。 -/
def WellAttached (e : MutexElement) : Prop :=
  match e.attached with
  | none => True
  | some p => e.level ≤ p.level

instance (e : MutexElement) : Decidable e.WellAttached := by
  unfold WellAttached; cases e.attached <;> infer_instance

/-! ── ρ_e 派生的三谱系定理（M06/M07/M08 各自判据 + 互斥穷尽）─────────────────────── -/

/-- ★M06 根级判据（L0）：α_e = ∅ ⟺ ρ_e = Root。坐实 spec M06「ρ_e=Root ⟺ α_e=∅」
    （独立做多/做空，无依附对象）。 -/
theorem levelRelation_root_iff (e : MutexElement) :
    e.levelRelation = LevelRelation.root ↔ e.attached = none := by
  cases h : e.attached with
  | none => simp [levelRelation, h]
  | some p =>
    simp only [levelRelation, h]
    by_cases hlt : e.level < p.level <;> simp [hlt]

/-- ★M08 次级别判据（L0）：ρ_e = Sub ⟺ ∃ 依附对象 p 且 ℓ_e < ℓ_p（ℓ_e < ℓ_{α_e}）。
    坐实 spec M08「ρ_e=Sub ⟺ ℓ_e<ℓ_{α_e}」（次级别，区间严格包含的级别侧条件）。 -/
theorem levelRelation_sub_iff (e : MutexElement) :
    e.levelRelation = LevelRelation.sub ↔ ∃ p, e.attached = some p ∧ e.level < p.level := by
  cases h : e.attached with
  | none => simp [levelRelation, h]
  | some p =>
    simp only [levelRelation, h, Option.some.injEq]
    by_cases hlt : e.level < p.level
    · simp only [hlt, if_true]
      exact ⟨fun _ => ⟨p, rfl, hlt⟩, fun _ => trivial⟩
    · simp only [hlt, if_false]
      constructor
      · intro hr; exact absurd hr (by decide)
      · rintro ⟨p', rfl, hlt'⟩; exact absurd hlt' hlt

/-- ★M07 同级别判据（L0）：ρ_e = Same ⟺ ∃ 依附对象 p 且 ¬(ℓ_e < ℓ_p)（ℓ_e ≥ ℓ_{α_e}）。
    在 `WellAttached`（ℓ_e ≤ ℓ_{α_e}）有效域内，¬(ℓ_e<ℓ_p) ∧ ℓ_e≤ℓ_p ⟹ ℓ_e=ℓ_p，得
    spec M07「ρ_e=Same ⟺ ℓ_e=ℓ_{α_e}」（见 `levelRelation_same_iff_of_wellAttached`）。 -/
theorem levelRelation_same_iff (e : MutexElement) :
    e.levelRelation = LevelRelation.same ↔ ∃ p, e.attached = some p ∧ ¬ e.level < p.level := by
  cases h : e.attached with
  | none => simp [levelRelation, h]
  | some p =>
    simp only [levelRelation, h, Option.some.injEq]
    by_cases hlt : e.level < p.level
    · simp only [hlt, if_true]
      constructor
      · intro hr; exact absurd hr (by decide)
      · rintro ⟨p', rfl, hge⟩; exact absurd hlt hge
    · simp only [hlt, if_false]
      exact ⟨fun _ => ⟨p, rfl, hlt⟩, fun _ => trivial⟩

/-- ★M07 同级别精确判据（在合法依附有效域内，L0 / 231号有效域）：`WellAttached e` ⟹
    （ρ_e = Same ⟺ ∃ p, α_e = some p ∧ ℓ_e = ℓ_p）。这是 spec M07「ρ_e=Same ⟺ ℓ_e=ℓ_{α_e}」
    的精确形式——只在操作语义合法依附（ℓ_e ≤ ℓ_{α_e}）的有效域内成立。越级（ℓ_e>ℓ_p，有效域外）
    虽也归 Same（§ 1 NB），但那是定义域全函数的边界归并，非操作语义的同级别。 -/
theorem levelRelation_same_iff_of_wellAttached (e : MutexElement) (hwa : e.WellAttached) :
    e.levelRelation = LevelRelation.same ↔ ∃ p, e.attached = some p ∧ e.level = p.level := by
  rw [levelRelation_same_iff]
  unfold WellAttached at hwa
  constructor
  · rintro ⟨p', hp', hge⟩
    rw [hp'] at hwa
    exact ⟨p', hp', Nat.le_antisymm hwa (Nat.not_lt.mp hge)⟩
  · rintro ⟨p', hp', heq⟩
    exact ⟨p', hp', by omega⟩

/-! ── 级别关系三分互斥穷尽（M05，MW4 / M12 角色互斥穷尽的级别侧基底）─────────────── -/

/-- ★ρ_e 三分穷尽（L0，M05）：每元素的 ρ_e 必是 Root/Same/Sub 之一。直接由 `LevelRelation`
    归纳类型穷尽 + `levelRelation` 全函数。坐实 spec §C「级别关系侧三分完备（穷尽）」。 -/
theorem levelRelation_exhaustive (e : MutexElement) :
    e.levelRelation = LevelRelation.root
    ∨ e.levelRelation = LevelRelation.same
    ∨ e.levelRelation = LevelRelation.sub :=
  LevelRelation.exhaustive e.levelRelation

/-- ★ρ_e 三分唯一判定（L0，spec §2「操作语义必须唯一判定」）：ρ_e 由 `levelRelation` 函数
    确定，故对每元素唯一。形式化为「levelRelation 是函数 ⟹ 任两次求值相等」（自反，承载
    唯一性语义：同一元素不会判出两个级别关系）。 -/
theorem levelRelation_unique (e : MutexElement) :
    ∀ r₁ r₂, e.levelRelation = r₁ → e.levelRelation = r₂ → r₁ = r₂ := by
  intro r₁ r₂ h₁ h₂; rw [← h₁, ← h₂]

/-- ★ρ_e 三分互斥（L0，M05 / §C「互斥」）：Root/Same/Sub 两两不相交——元素的 ρ_e 不能同时是
    其中两个。形式化为指示函数和 = 1（恰一个为真）：在三个判据谓词上恰一个成立。 -/
theorem levelRelation_mutex (e : MutexElement) :
    (e.levelRelation = LevelRelation.root ∧ e.levelRelation ≠ LevelRelation.same
       ∧ e.levelRelation ≠ LevelRelation.sub)
    ∨ (e.levelRelation ≠ LevelRelation.root ∧ e.levelRelation = LevelRelation.same
       ∧ e.levelRelation ≠ LevelRelation.sub)
    ∨ (e.levelRelation ≠ LevelRelation.root ∧ e.levelRelation ≠ LevelRelation.same
       ∧ e.levelRelation = LevelRelation.sub) := by
  rcases levelRelation_exhaustive e with h | h | h
  · exact Or.inl ⟨h, by rw [h]; decide, by rw [h]; decide⟩
  · exact Or.inr (Or.inl ⟨by rw [h]; decide, h, by rw [h]; decide⟩)
  · exact Or.inr (Or.inr ⟨by rw [h]; decide, by rw [h]; decide, h⟩)

/-! ── 根级谓词对接 W7 isRoot（M06，扩展非冲突）──────────────────────────────────── -/

/-- ★元素根级谓词（M06）：ρ_e = Root（依 `levelRelation`，⟺ α_e=∅）。 -/
def isRootRel (e : MutexElement) : Prop := e.levelRelation = LevelRelation.root

instance (e : MutexElement) : Decidable e.isRootRel := by
  unfold isRootRel; infer_instance

/-- ★根级 ⟺ 依附对象为空（M06，L0）：isRootRel ⟺ attached = none（= `levelRelation_root_iff`
    的谓词包装）。坐实 spec M06「ρ_e=Root ⟹ α_e=∅」（独立做多/做空）。 -/
theorem isRootRel_iff_attached_none (e : MutexElement) :
    e.isRootRel ↔ e.attached = none :=
  levelRelation_root_iff e

end MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 3. Same 与 RecursiveLevelSystem 级别定义的冲突核对（spec §E(b) 边界，no-workaround）

    spec §E(b) 警示 + 任务铁律要求：核对 Same（同级别 ℓ_e=ℓ_{α_e}）是否与现有
    `Origin.RecursiveLevelSystem` 的级别定义不可弥合。本节给出冲突核对定理（无冲突的结构见证）。

    `Origin.RecursiveLevelSystem`（`TrendCompleteClassification.lean`）：
      structure RecursiveLevelSystem where
        State : Nat → Type           ← 绝对级别 n 的状态类型
        lift  : (n) → State n → State (n+1)  ← 绝对级别提升 n → n+1
    它提供 **绝对级别坐标**（Nat n），lift 把第 n 级 **提升** 到第 n+1 级。它 **不约束** 同一
    绝对级别内两元素的相对级别关系（Same/Sub）——同级别（ℓ_e = ℓ_{α_e}，两元素 level 相等，
    `level = n` 同坐标）是 RecursiveLevelSystem 绝对坐标系下的 **合法状态**，不需要 lift 跨级。

    结论（与 RecursiveLevelSystem 无定义冲突）：Same 的 ℓ_e = ℓ_{α_e} 是「两元素绝对 level
    相等」，RecursiveLevelSystem 的 level（Nat n）正是这个绝对坐标——Same 在该坐标系完全有
    位置（level 相等是合法），不需要也不违反 lift（lift 是跨级 n→n+1，Same 是同级 n=n）。
    spec §E(b)「Same 在声部树无位置」仅指声部树覆盖归属（父/子嵌套），不指绝对级别坐标。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★Same 在绝对级别坐标系有位置（spec §E(b) 边界核对，L0，无冲突见证）：在合法依附有效域内，
    ρ_e = Same ⟺ 元素与依附对象的 **绝对级别相等**（ℓ_e = ℓ_{α_e}）。这是 RecursiveLevelSystem
    绝对级别坐标（`level : Nat`）下的合法状态——两元素同 level（同绝对级别 n），不需 lift 跨级。
    故 Same 与 RecursiveLevelSystem「lift: n→n+1」**不冲突**：lift 是跨级提升，Same 是同级
    （n=n），两者作用于不同维度（绝对提升 vs 同级关系），无定义矛盾。 -/
theorem same_is_equal_absolute_level (e : MutexElement) (hwa : e.WellAttached)
    (p : SyntaxElement) (hp : e.attached = some p) :
    e.levelRelation = LevelRelation.same ↔ e.level = p.level := by
  rw [levelRelation_same_iff_of_wellAttached e hwa]
  constructor
  · rintro ⟨p', hp', heq⟩
    obtain rfl : p = p' := by rw [hp] at hp'; injection hp'
    exact heq
  · intro heq; exact ⟨p, hp, heq⟩

/-- ★Sub 严格低于依附对象绝对级别（spec §E(b) 边界核对，L0）：ρ_e = Sub ⟺ ℓ_e < ℓ_{α_e}
    （元素绝对 level 严格小于依附对象）——这正是 RecursiveLevelSystem 绝对级别坐标下的「次级别」
    （n < m），对应 lift 可从 ℓ_e 级逐步提升到 ℓ_{α_e} 级（Sub 是跨级关系，与 lift 同向）。
    Same（n=n）与 Sub（n<m）在绝对坐标系 **互斥**（n=n 与 n<m 不可同时），坐实级别三分在
    RecursiveLevelSystem 坐标系下的互斥性（与 §2 `levelRelation_mutex` 一致，无冲突）。 -/
theorem sub_is_strictly_below_absolute_level (e : MutexElement)
    (p : SyntaxElement) (hp : e.attached = some p) :
    e.levelRelation = LevelRelation.sub ↔ e.level < p.level := by
  rw [levelRelation_sub_iff]
  constructor
  · rintro ⟨p', hp', hlt⟩
    obtain rfl : p = p' := by rw [hp] at hp'; injection hp'
    exact hlt
  · intro hlt; exact ⟨p, hp, hlt⟩

end MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 4. C27 是 M04 特例（C27 四元素 = M04 在「依附对象 = 父标识对应元素」的特例见证）

    spec §B M04 行 + no-workaround 核对：C27 四元组（`SyntaxElement`）是 M04 五元组在「不显式
    携带依附对象级别关系」的退化。本节给出「C27 元素 ⟶ M04 元素」的嵌入（补 attached），坐实
    M04 是 C27 的真精化（非冲突）——C27 加 α_e 维即得 M04，根级 C27（par=none）嵌入为 Root。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★C27 根元素（par=none）⟶ M04 根级元素的嵌入：给定 C27 `SyntaxElement` 且为根（par=none），
    补 attached=none 得 M04 根级元素（ρ_e=Root）。坐实根级 C27 = M04 在 (attached=none) 的特例
    ——M04 是 C27 的真精化（加依附对象维），精化非冲突。 -/
def rootElementAsMutex (e : SyntaxElement) : MutexElement where
  core := e
  attached := none

/-- ★嵌入得根级（L0，M06）：`rootElementAsMutex` 产出的 M04 元素 ρ_e = Root（attached=none）。 -/
theorem rootElementAsMutex_isRoot (e : SyntaxElement) :
    (rootElementAsMutex e).levelRelation = LevelRelation.root := by
  unfold rootElementAsMutex MutexElement.levelRelation; rfl

/-- ★嵌入保级别 + 方向（L0）：嵌入的 M04 元素级别 ℓ_e = C27 元素级别，方向符号 ε_e 一致
    （无级别/方向漂移——M04 精化只加 α_e，不改 C27 已有 I/ε/ℓ/par）。 -/
theorem rootElementAsMutex_preserves (e : SyntaxElement) :
    (rootElementAsMutex e).level = e.level
    ∧ (rootElementAsMutex e).dirSign = e.dirSign :=
  ⟨rfl, rfl⟩

/-- ★依附嵌入：C27 元素 + 依附对象 p ⟶ M04 元素（attached=some p）。给定 C27 元素与其依附的
    上级元素 p，得携依附对象的 M04 元素，ρ_e 由 `levelRelation` 依 ℓ_e vs ℓ_p 派生（Same/Sub）。 -/
def attachedElementAsMutex (e : SyntaxElement) (p : SyntaxElement) : MutexElement where
  core := e
  attached := some p

/-- ★依附嵌入的级别关系（L0）：ℓ_e < ℓ_p ⟹ Sub、否则 Same（直接由 `levelRelation` 派生规则）。
    坐实依附嵌入的 ρ_e 由级别比较唯一确定（M07/M08）。 -/
theorem attachedElementAsMutex_rel (e : SyntaxElement) (p : SyntaxElement) :
    (attachedElementAsMutex e p).levelRelation
      = if e.level < p.level then LevelRelation.sub else LevelRelation.same := by
  unfold attachedElementAsMutex MutexElement.levelRelation MutexElement.level
  rfl

end NewChanlun.Origin
