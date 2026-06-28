/-
  Origin/MutexExhaustive.lean — MW4 角色互斥穷尽顶点定理 M12（Σ指示=1）+ 完全互斥分类完备性

  ── 存在论位置（M12，互斥分类 canonical spec §B 表 M12 行 / §A 页4 §4 / §C 完备性论证）────
  本文件形式化 **M12 角色互斥穷尽定理**——goal「严格完全互斥分类」的 **形式化顶点**：
    ∀ e ∈ E, Σ_{R ∈ {RootDir, SameDir, SubFollow, ShortDiff}} 1[operationRole(e) = R] = 1
  即每元素的操作角色 **恰一角色成立**（指示函数求和=1 ⟺ 穷尽 + 互斥 + 唯一归属，标准「完全
  互斥分类」三要素）。这不是已有 `operationRole_mutex`（析取形式）的复述——本文件把互斥穷尽
  提升为 **算术 Σ指示=1 的 canonical 顶点陈述**（spec M12 原文 `Σ_R 1[Role(e)=R]=1`），并组装：

  - **角色侧（MW3 `Origin.OperationRole`）**：`operationRole_exhaustive`（四分穷尽）+
    `operationRole_mutex`（四分互斥）→ `roleIndicatorSum e = 1`（角色维度 Σ指示=1，M12 角色侧）。
  - **级别侧（MW1 `Origin.MutexElement`）**：`levelRelation_exhaustive`（三分穷尽）+
    `levelRelation_mutex`（三分互斥）→ `levelIndicatorSum e = 1`（Root/Same/Sub 维度 Σ指示=1）。
  - **合一（完全互斥分类完备性）**：`completeMutexClassificationSum e = 1`——每元素恰属一个
    (级别关系 ρ_e, 角色 Role(e)) 联合类（12 联合类指示函数求和=1）。这是 spec §C「级别三分穷尽
    × 角色四分穷尽 → 完全互斥分类」的完备性顶点（每元素恰一 (ρ_e, Role(e)) 归属）。

  spec §C 完备性三层（本文件全部坐实）：
    ① 级别关系侧三分完备（M05–M08）：`levelIndicatorSum = 1`（穷尽+互斥）。
    ② 方向关系侧二分完备（M08）：已在 MW3 `operationRole_*_iff` 坐实（Sub 下 ε=σ / ε=−σ 二分）。
    ③ 角色指示函数和=1（M12）：`roleIndicatorSum = 1`（四类互斥穷尽，本文件核心顶点）。

  ── ★防塌缩依据（M12 互斥性的具体支撑，来自 MW3）────────────────────────────────────
  M12 的「互斥」（Σ指示=1 中「恰一个」而非「至少一个」）依赖角色不混同：
  - M25（同级别反向 ≠ 短差，MW3 `sameDir_reverse_ne_shortDiff`）：保证 SameDir 与 ShortDiff 互斥。
  - M13（短差 ≠ 同级别反手，MW3 `shortDiff_ne_sameDir_reverse`）：防止 SameDir/ShortDiff 塌缩。
  本文件直接复用 MW3 的 `OperationRole.*_ne_*`（四角色两两不等，`DecidableEq` 派生）——这些
  不等是 Σ指示=1「恰一项为 1，余三项为 0」的算术根据。若任两角色塌缩，则会有两项同时为 1，
  Σ=2≠1，互斥性崩溃。故四角色两两不等 ⟺ 角色指示函数和=1（互斥穷尽的算术等价表述）。

  ── owner（互斥铁律）─────────────────────────────────────────────────────────────
  **只建** 本文件。import `Origin.OperationRole`（MW3，角色侧，已 GREEN）+ `Origin.MutexElement`
  （MW1，级别侧，已 GREEN，OperationRole 已 re-export 但显式 import 以求自包含）。不碰其他文件、
  不编辑 lakefile.toml。root 名 `Origin.MutexExhaustive`。

  ── 认识论等级（formalization-validity-domain 231号强制）─────────────────────────
  全部 **L0**（纯结构/代数：指示函数求和=1 是归纳类型穷尽互斥的代数恒等）。这是 goal「严格完全
  互斥分类」的 **形式化顶点**——但仍是 **L0 结构定理**（分类完备性），**非 L2**：本文件 **不**
  声称该角色分类在实盘有 alpha。全窗 L3 已 8/8 否证完整 v1 缠论择时无 alpha——**分类完备性
  ≠ 择时盈利**（有效域 ⊊ 定义域）。下游 **不得** 将 Σ指示=1 的有效域膨胀为「角色分类实盘可
  判定盈利」——那需 L2/L3 真实数据，本文件不提供也不可由 L0 推出。M12 是「构造的元素集上每
  元素恰属一角色」的代数恒等，信息增量为零的同义反复（L0）。

  ── 依赖方向（单向无环）+ 验证 ─────────────────────────────────────────────────
  MutexExhaustive → {Origin.OperationRole（MW3），Origin.MutexElement（MW1）}。standalone。
  验证：`cd formal && lake env lean Origin/MutexExhaustive.lean`。禁 sorry/admit/axiom。简体中文。

  谱系：M12（互斥分类 PDF §4 角色互斥穷尽 Σ指示=1）→ MW4（严格完全互斥分类形式化顶点）→
        下游 MW8（M26 三分归纳）/ 最终互斥定理集成。M12 = goal 形式化顶点之一（前言两证明目标
        中的「分类必须互斥穷尽」），与 M20（策略全定义 ∃!动作）共同构成 goal。
-/

import Origin.MutexElement
import Origin.OperationRole

namespace NewChanlun.Origin

open MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 角色 / 级别关系的枚举列表（指示函数求和的载体）

    Σ指示=1 须有「求和的索引集」。本节给出 `OperationRole` 四角色 / `LevelRelation` 三级别关系
    的 canonical 枚举列表（含「枚举完备」引理：列表恰含全部构造子，无遗漏无重复）。这是
    spec M12 `Σ_{R ∈ {RootDir,SameDir,SubFollow,ShortDiff}}` 求和下标集的形式化。
    ════════════════════════════════════════════════════════════════════════ -/

namespace OperationRole

/-- ★角色枚举列表（M12 求和索引集）：`{RootDir, SameDir, SubFollow, ShortDiff}` 四角色的
    canonical 列表，作为 spec M12 `Σ_{R ∈ {...}}` 求和的下标集。 -/
def allRoles : List OperationRole := [rootDir, sameDir, subFollow, shortDiff]

/-- ★角色枚举完备（L0）：任一角色 R 必在 `allRoles` 中（列表恰含全部四构造子）。坐实求和
    索引集覆盖全部角色——Σ指示=1 的求和不遗漏任何角色。 -/
theorem mem_allRoles (R : OperationRole) : R ∈ allRoles := by
  cases R <;> simp [allRoles]

/-- ★角色枚举无重复（L0）：`allRoles` 四元素两两不同（`Nodup`）。保证指示函数求和中每角色
    只计一次——避免重复计数破坏 Σ指示=1。 -/
theorem allRoles_nodup : allRoles.Nodup := by decide

end OperationRole

namespace LevelRelation

/-- ★级别关系枚举列表（M12 级别侧求和索引集）：`{Root, Same, Sub}` 三级别关系的 canonical 列表。 -/
def allRelations : List LevelRelation := [root, same, sub]

/-- ★级别关系枚举完备（L0）：任一 ρ_e 必在 `allRelations` 中（列表恰含全部三构造子）。 -/
theorem mem_allRelations (r : LevelRelation) : r ∈ allRelations := by
  cases r <;> simp [allRelations]

/-- ★级别关系枚举无重复（L0）：`allRelations` 三元素两两不同（`Nodup`）。 -/
theorem allRelations_nodup : allRelations.Nodup := by decide

end LevelRelation

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 角色侧指示函数求和 `roleIndicatorSum`（M12 角色维度 Σ指示=1）

    spec M12 核心：`Σ_{R ∈ {RootDir,SameDir,SubFollow,ShortDiff}} 1[Role(e)=R] = 1`。本节定义
    指示函数求和 `roleIndicatorSum e = Σ_R 1[operationRole(e)=R]`（`List.sum` 形式，非析取），
    并证 = 1。这是 MW4 角色互斥穷尽的 **算术 canonical 顶点**——把 MW3 `operationRole_mutex`
    （析取「恰一分支成立」）提升为 spec M12 原文的 Σ指示=1 算术等式。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★角色指示函数（M12）：`1[operationRole(e) = R]`——元素 e 的角色等于 R 时为 1，否则为 0。
    这是 spec M12 求和 `Σ_R 1[Role(e)=R]` 的被加项。 -/
def roleIndicator (e : MutexElement) (R : OperationRole) : Nat :=
  if e.operationRole = R then 1 else 0

/--
  **角色侧指示函数求和（M12 角色维度，spec M12 顶点）** ——
  `roleIndicatorSum e = Σ_{R ∈ allRoles} 1[operationRole(e) = R]`（`List.sum` 形式）。

  这是 spec M12 `Σ_{R ∈ {RootDir,SameDir,SubFollow,ShortDiff}} 1[Role(e)=R]` 的形式化。下面
  `roleIndicatorSum_eq_one` 证其 = 1（恰一角色成立 = 穷尽 + 互斥 + 唯一）。 -/
def roleIndicatorSum (e : MutexElement) : Nat :=
  (OperationRole.allRoles.map (fun R => e.roleIndicator R)).sum

/--
  ★★★M12 角色互斥穷尽顶点定理（角色维度，L0，goal 形式化顶点）：
    `∀ e, Σ_{R ∈ {RootDir,SameDir,SubFollow,ShortDiff}} 1[operationRole(e) = R] = 1`
  即每元素的操作角色 **恰一角色成立**（指示函数求和=1）。这是 spec M12 原文的 canonical 算术
  形式——「完全互斥分类」三要素（穷尽 + 互斥 + 唯一归属）在角色维度的实现：
  - **穷尽**：求和 ≥ 1（至少一角色，由 `operationRole_exhaustive` / `OperationRole.exhaustive`）。
  - **互斥**：求和 ≤ 1（至多一角色，由四角色两两不等 `OperationRole.*_ne_*`）。
  - **唯一**：求和 = 1（恰一角色）。

  ★证明：`operationRole e` 是 `OperationRole` 四构造子之一（穷尽）。对每个具体角色代入，
  `roleIndicatorSum` 是闭项（四个指示函数求和），由 `DecidableEq OperationRole`（四角色两两
  不等）`decide` 出 = 1（恰一项为 1，余三项为 0）。这把 MW3 的互斥（`*_ne_*`）+ 穷尽
  （`exhaustive`）组装为 Σ指示=1。 -/
theorem roleIndicatorSum_eq_one (e : MutexElement) : e.roleIndicatorSum = 1 := by
  unfold roleIndicatorSum roleIndicator OperationRole.allRoles
  rcases operationRole_exhaustive e with h | h | h | h <;> rw [h] <;> decide

/-- ★M12 角色「恰一角色」展开（L0，Σ指示=1 的可读形式）：`roleIndicatorSum = 1` 等价于
    四个指示函数中恰一个为 1、其余三个为 0。把 Σ指示=1 与 MW3 `operationRole_mutex` 析取
    形式桥接（同一互斥穷尽事实的两种表述：算术 Σ=1 ⟺ 析取恰一分支）。 -/
theorem roleIndicator_exactly_one (e : MutexElement) :
    (e.roleIndicator OperationRole.rootDir = 1 ∧ e.roleIndicator OperationRole.sameDir = 0
       ∧ e.roleIndicator OperationRole.subFollow = 0 ∧ e.roleIndicator OperationRole.shortDiff = 0)
    ∨ (e.roleIndicator OperationRole.rootDir = 0 ∧ e.roleIndicator OperationRole.sameDir = 1
       ∧ e.roleIndicator OperationRole.subFollow = 0 ∧ e.roleIndicator OperationRole.shortDiff = 0)
    ∨ (e.roleIndicator OperationRole.rootDir = 0 ∧ e.roleIndicator OperationRole.sameDir = 0
       ∧ e.roleIndicator OperationRole.subFollow = 1 ∧ e.roleIndicator OperationRole.shortDiff = 0)
    ∨ (e.roleIndicator OperationRole.rootDir = 0 ∧ e.roleIndicator OperationRole.sameDir = 0
       ∧ e.roleIndicator OperationRole.subFollow = 0 ∧ e.roleIndicator OperationRole.shortDiff = 1) := by
  unfold roleIndicator
  rcases operationRole_exhaustive e with h | h | h | h <;> rw [h] <;> decide

/-! ════════════════════════════════════════════════════════════════════════
    § 3. 级别关系侧指示函数求和 `levelIndicatorSum`（M12 Root/Same/Sub 维度 Σ指示=1）

    spec §C 完备性论证 ① 级别关系侧三分完备：每元素 ρ_e 唯一判定为 Root/Same/Sub 之一。本节
    给出级别侧 Σ指示=1（与角色侧同构，三分而非四分），作为 M12 联合完备性的级别维度基底。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★级别关系指示函数（M12 级别侧）：`1[levelRelation(e) = r]`。 -/
def levelIndicator (e : MutexElement) (r : LevelRelation) : Nat :=
  if e.levelRelation = r then 1 else 0

/-- **级别关系侧指示函数求和（M12 Root/Same/Sub 维度）** ——
    `levelIndicatorSum e = Σ_{r ∈ allRelations} 1[levelRelation(e) = r]`。spec §C 级别三分完备的
    Σ指示=1 形式。 -/
def levelIndicatorSum (e : MutexElement) : Nat :=
  (LevelRelation.allRelations.map (fun r => e.levelIndicator r)).sum

/-- ★★M12 级别关系互斥穷尽（级别维度 Σ指示=1，L0，spec §C ①）：
    `∀ e, Σ_{r ∈ {Root,Same,Sub}} 1[levelRelation(e) = r] = 1`——每元素 ρ_e 恰一级别关系成立。
    组装 MW1 `levelRelation_exhaustive`（三分穷尽）+ 三级别关系两两不等（`LevelRelation.*_ne_*`，
    互斥）。这是 M12 联合完备性的级别维度基底（角色侧 `roleIndicatorSum_eq_one` 的级别对偶）。 -/
theorem levelIndicatorSum_eq_one (e : MutexElement) : e.levelIndicatorSum = 1 := by
  unfold levelIndicatorSum levelIndicator LevelRelation.allRelations
  rcases levelRelation_exhaustive e with h | h | h <;> rw [h] <;> decide

/-! ════════════════════════════════════════════════════════════════════════
    § 4. 完全互斥分类完备性顶点 `completeMutexClassificationSum`（M12 合一）

    spec §C「**级别三分穷尽 × 方向二分穷尽 → 四类角色互斥穷尽（Σ指示=1）**」+ §E 第4要素
    「M12 是 goal 形式化顶点」。本节合一角色侧 + 级别侧为 **联合分类完备性**：每元素恰属一个
    (级别关系 ρ_e, 角色 Role(e)) 联合类——12 个联合类（3 级别 × 4 角色）的指示函数求和=1。
    这是「严格完全互斥分类」的 canonical 顶点（每元素在联合分类中恰一归属）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★联合分类指示函数（M12 合一）：`1[(levelRelation(e), operationRole(e)) = (r, R)]`——元素 e
    的 (级别关系, 角色) 联合等于 (r, R) 时为 1。这是 spec §C「(级别关系, 角色) 联合类」的指示。 -/
def jointIndicator (e : MutexElement) (r : LevelRelation) (R : OperationRole) : Nat :=
  if e.levelRelation = r ∧ e.operationRole = R then 1 else 0

/--
  **完全互斥分类完备性求和（M12 合一顶点，spec §C / §E 第4要素）** ——
  `completeMutexClassificationSum e = Σ_{r ∈ allRelations} Σ_{R ∈ allRoles}
      1[(levelRelation(e), operationRole(e)) = (r, R)]`
  即对全部 12 个 (级别关系, 角色) 联合类求指示函数和。`completeMutexClassificationSum_eq_one`
  证其 = 1——每元素 **恰属一个联合类**（严格完全互斥分类的 canonical 顶点）。 -/
def completeMutexClassificationSum (e : MutexElement) : Nat :=
  (LevelRelation.allRelations.map
    (fun r => (OperationRole.allRoles.map (fun R => e.jointIndicator r R)).sum)).sum

/--
  ★★★★M12 完全互斥分类完备性顶点定理（合一，L0，goal「严格完全互斥分类」形式化顶点）：
    `∀ e, Σ_{(r,R) ∈ {Root,Same,Sub} × {RootDir,SameDir,SubFollow,ShortDiff}}
            1[(levelRelation(e), operationRole(e)) = (r, R)] = 1`
  即每元素 **恰属一个 (级别关系 ρ_e, 角色 Role(e)) 联合类**。这是 spec §C「级别三分穷尽 ×
  角色四分穷尽 → 完全互斥分类」的 canonical 完备性顶点——「严格完全互斥分类」的形式化顶点
  （标准三要素：穷尽 + 互斥 + 唯一归属，在联合分类维度的实现）。

  ★证明：`levelRelation e` ∈ {Root,Same,Sub}（三分穷尽，MW1）∧ `operationRole e` ∈
  {RootDir,SameDir,SubFollow,ShortDiff}（四分穷尽，MW3）。对两侧各代入具体值后，
  `completeMutexClassificationSum` 是闭项（12 联合指示函数求和），由 `DecidableEq`（联合类
  两两不等）`decide` 出 = 1（恰一联合类的指示=1，余 11 个=0）。这组装级别侧穷尽互斥
  （`levelIndicatorSum_eq_one`）+ 角色侧穷尽互斥（`roleIndicatorSum_eq_one`）为联合完备性。 -/
theorem completeMutexClassificationSum_eq_one (e : MutexElement) :
    e.completeMutexClassificationSum = 1 := by
  unfold completeMutexClassificationSum jointIndicator
    LevelRelation.allRelations OperationRole.allRoles
  rcases levelRelation_exhaustive e with hl | hl | hl <;>
    rcases operationRole_exhaustive e with hr | hr | hr | hr <;>
    rw [hl, hr] <;> decide

/-! ── M12 顶点的存在唯一性表述（每元素恰存在唯一 (ρ_e, Role(e)) 归属）──────────────── -/

/-- ★★M12 唯一归属（存在唯一性形式，L0，spec §C「唯一归属」三要素之一）：每元素 **恰存在唯一**
    一对 (级别关系 r, 角色 R) 使其 (levelRelation, operationRole) 联合等于 (r, R)。这是
    `completeMutexClassificationSum_eq_one`（Σ指示=1）的存在唯一性等价表述——Σ指示=1 ⟺ 恰一
    联合类归属。spec「完全互斥分类」= 每元素在 (级别×角色) 联合分类中恰一归属。

    ★存在唯一性用 `Exists` + 合取显式表达（core Lean 4.31 无 Mathlib `∃!` notation；本项目
    `lakefile.toml` 声明不依赖 Mathlib）：存在一对 (ρ_e, Role(e)) 满足联合等式，且任何满足者
    必等于它（唯一）。 -/
theorem exists_unique_joint_class (e : MutexElement) :
    ∃ rR : LevelRelation × OperationRole,
      (e.levelRelation = rR.1 ∧ e.operationRole = rR.2)
      ∧ ∀ rR' : LevelRelation × OperationRole,
          (e.levelRelation = rR'.1 ∧ e.operationRole = rR'.2) → rR' = rR := by
  refine ⟨(e.levelRelation, e.operationRole), ⟨rfl, rfl⟩, ?_⟩
  rintro ⟨r, R⟩ ⟨hr, hR⟩
  exact Prod.ext hr.symm hR.symm

/-- ★M12 联合类恰一指示为 1（L0，互斥的算术坐实）：在全部 12 个 (r, R) 联合类中，恰有一个
    `jointIndicator e r R = 1`（即 (r,R) = (levelRelation e, operationRole e)），其余 11 个 = 0。
    这是 `completeMutexClassificationSum_eq_one` 的「恰一项为 1」的逐项坐实——互斥（无两项
    同时为 1）+ 穷尽（至少一项为 1）的算术等价。 -/
theorem jointIndicator_eq_one_iff (e : MutexElement) (r : LevelRelation) (R : OperationRole) :
    e.jointIndicator r R = 1 ↔ (e.levelRelation = r ∧ e.operationRole = R) := by
  unfold jointIndicator
  by_cases h : e.levelRelation = r ∧ e.operationRole = R <;> simp [h]

end MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 5. M12 顶点与 MW3/MW1 析取形式的桥接（互斥穷尽的两种等价表述）

    M12 Σ指示=1（算术）与 MW3 `operationRole_mutex` / MW1 `levelRelation_mutex`（析取「恰一
    分支成立」）是 **同一互斥穷尽事实的两种表述**。本节给出桥接定理，坐实 Σ指示=1 顶点不是
    新增公理而是 MW3/MW1 已结算互斥穷尽的算术提升（无新增假设，纯组装）。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★M12 顶点 ⟸ MW3 析取互斥（桥接，L0）：MW3 `operationRole_mutex`（角色恰一分支成立的析取
    形式）⟹ 角色侧 Σ指示=1（`roleIndicatorSum_eq_one`）。坐实 Σ指示=1 由 MW3 已结算互斥穷尽
    导出，非新增假设——M12 是 MW3 的算术提升，nullary 信息增量（L0，同一事实换表述）。 -/
theorem roleIndicatorSum_one_of_mutex (e : MutexElement) :
    (e.operationRole = OperationRole.rootDir
       ∧ e.operationRole ≠ OperationRole.sameDir
       ∧ e.operationRole ≠ OperationRole.subFollow
       ∧ e.operationRole ≠ OperationRole.shortDiff)
    ∨ (e.operationRole ≠ OperationRole.rootDir
       ∧ e.operationRole = OperationRole.sameDir
       ∧ e.operationRole ≠ OperationRole.subFollow
       ∧ e.operationRole ≠ OperationRole.shortDiff)
    ∨ (e.operationRole ≠ OperationRole.rootDir
       ∧ e.operationRole ≠ OperationRole.sameDir
       ∧ e.operationRole = OperationRole.subFollow
       ∧ e.operationRole ≠ OperationRole.shortDiff)
    ∨ (e.operationRole ≠ OperationRole.rootDir
       ∧ e.operationRole ≠ OperationRole.sameDir
       ∧ e.operationRole ≠ OperationRole.subFollow
       ∧ e.operationRole = OperationRole.shortDiff)
    → e.roleIndicatorSum = 1 := by
  intro _; exact roleIndicatorSum_eq_one e

/-- ★M12 角色 Σ指示=1 ⟺ MW3 析取恰一（双向桥接，L0）：角色侧 Σ指示=1 永真（`= 1`），
    与 MW3 `operationRole_mutex`（恒成立的析取）同为「角色互斥穷尽」的真表述——两者都无条件成立。
    本定理坐实 M12 顶点（Σ指示=1）与 MW3 互斥析取在「同一元素」上一致（同一互斥穷尽事实）。 -/
theorem roleIndicatorSum_one_iff_mutex (e : MutexElement) :
    e.roleIndicatorSum = 1 ↔
      ((e.operationRole = OperationRole.rootDir
         ∧ e.operationRole ≠ OperationRole.sameDir
         ∧ e.operationRole ≠ OperationRole.subFollow
         ∧ e.operationRole ≠ OperationRole.shortDiff)
      ∨ (e.operationRole ≠ OperationRole.rootDir
         ∧ e.operationRole = OperationRole.sameDir
         ∧ e.operationRole ≠ OperationRole.subFollow
         ∧ e.operationRole ≠ OperationRole.shortDiff)
      ∨ (e.operationRole ≠ OperationRole.rootDir
         ∧ e.operationRole ≠ OperationRole.sameDir
         ∧ e.operationRole = OperationRole.subFollow
         ∧ e.operationRole ≠ OperationRole.shortDiff)
      ∨ (e.operationRole ≠ OperationRole.rootDir
         ∧ e.operationRole ≠ OperationRole.sameDir
         ∧ e.operationRole ≠ OperationRole.subFollow
         ∧ e.operationRole = OperationRole.shortDiff)) :=
  ⟨fun _ => operationRole_mutex e, fun _ => roleIndicatorSum_eq_one e⟩

end MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 6. 防膨胀 gatekeeper（231号有效域 ⊊ 定义域，L0 ≠ L2）

    M12 是 goal「严格完全互斥分类」的形式化顶点，但仍是 L0 结构定理。本节以 #check / 注释
    显式记录有效域边界——防止下游把 Σ指示=1 的有效域膨胀为「角色分类实盘可判定盈利」（那需
    L2/L3，全窗 L3 已 8/8 否证完整 v1 择时无 alpha）。分类完备性 ≠ 择时盈利。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★gatekeeper（L0 有效域锚点）：M12 顶点定理的类型签名是「∀ e : MutexElement, ... = 1」——
    定义域是 **构造的元素集**（`MutexElement`），非真实市场数据。Σ指示=1 在该定义域上代数成立
    （L0 同义反复），**不蕴含** 任何 L1+ 经验有效性。下游引用 M12 时须保此有效域：M12 证的是
    「每元素恰属一角色/联合类」（结构完备性），非「该分类在实盘择时盈利」（L2/L3 经验命题，
    本文件不提供）。把 M12 有效域膨胀为实盘盈利 = 声明膨胀（090号）+ 有效域膨胀（231号）。 -/
theorem m12_validity_domain_is_constructed_elements :
    ∀ e : MutexElement, e.completeMutexClassificationSum = 1 :=
  MutexElement.completeMutexClassificationSum_eq_one

end NewChanlun.Origin
