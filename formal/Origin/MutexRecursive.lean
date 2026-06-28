/-
  Origin/MutexRecursive.lean — MW8 角色感知自相似递归 M26（三分互斥穷尽归纳）

  ── ★去根化偏离声明（20页 spec paradigm 对照，2026-06-28）──────────────────────────────
  **本文件属 15页 paradigm**（M26 三分互斥穷尽归纳在 `LevelRelation = Root/Same/Sub` 上进行，
  `Root` 是分类轴特例）。**20页权威 paradigm 在 `Origin.OperationRole18`**（18 类 R=H×V×δ，
  去根化：无 `Root` 构造子，归纳基础由边界胚元 ∂ + Ambient 承载）。

  - **有效域**：M26 三分归纳在 15页定义域内是 L0 结构定理。20页去根化表明归纳不应以 Root
    特例为分支——有效域严格小于 15页定义域声称（231号：有效域 ⊊ 定义域）。
  - **直接矛盾点**：M26 归纳的 `Root` 分支（α_e=∅）与 20页去根化**直接矛盾**。本文件保留
    该分支是 15paradigm 的历史存在。
  - **为何保留**（no-patch 保留契约锚，MEMORY newchanlun-no-patch-keep-primitive）：本文件被
    `MutexFinalTheorem` import 锚定，M26 是 15页时期已结算的 L0 定理。删除会破既有 GREEN。
    保留为**契约锚**（非权威 paradigm），诚实标注有效域——非兼容垫片。
  - **权威指引**：新模块应 import `Origin.OperationRole18`（去根化权威），不应 import 本文件。
  ──────────────────────────────────────────────────────────────────────────────────────

  ── 存在论位置（M26，互斥分类 canonical spec §B 表 M26 行 / §A 页15 §19 / §D MW8 行 / §F）────
  本文件把 W13 基础递归（`Origin.SeparateCoverRecursive`，父/子二分覆盖骨架）的归纳步从
  **父/子二分**（同向/反向，由 W8 `dir_consistent` 统一承载方向）**精化** 为 **角色三分互斥
  穷尽**（M26）：

      W13 二分归纳步：  父 SepCovered ⟹ 子 SepCovered（同向/反向不区分角色）
      MW8 三分归纳步：  非根元素 e 按 operationRole 三分覆盖（互斥穷尽）：
          SameDir   （ρ_e=Same，同级别保持——同层延续/反向切换/反手，**反向仍 SameDir 非短差**）
          SubFollow （ρ_e=Sub ∧ ε_e=σ_{α_e}，次级别顺势腿——顺父方向）
          ShortDiff （ρ_e=Sub ∧ ε_e=−σ_{α_e}，短差——父声部不动建独立反向子声部）

  ── 本文件证什么（M26，spec §19 自相似递归角色精化版）─────────────────────────────────
  ① **∀e Eat^sep 仍成立**（角色三分不破坏覆盖）：复用 W13 `recursive_eatSep`（覆盖侧不重证，
     角色三分是 **叠加在覆盖上的分类层**——覆盖递归走 W13 良基 `par` 树，角色分类走独立操作
     依附 `α` 边，两边在同一元素集协同）。
  ② **每元素腿分配尊重角色互斥分类**（MW4 Σ指示=1）：每个树元素 e 的角色 `operationRole` 满足
     `roleIndicatorSum = 1`（MW4 `roleIndicatorSum_eq_one`），且非根元素角色恰落三分之一（MW3
     `nonRoot_role_trichotomy`）——腿分配按角色 role-indexed（`RoleLegDuty`），不漏类不重类。
  ③ **★防塌缩（M13/M25 核心，MW8 相对 W13 的真增量）**：归纳步三分中 ShortDiff（父不动建反向
     子声部）**严格区别于** SameDir 反向（同级别关闭/重建）——由 MW3 `sameDir_reverse_ne_shortDiff`
     （同级别反向角色恒 SameDir≠ShortDiff）+ `shortDiff_not_same`（短差恒 Sub≠Same）。若三分塌缩
     回二分（把短差等同同级别反向反手），互斥性丢失（MW4 Σ指示=1 崩溃，Σ=2）。

  ── ★解耦桥接的严格根据（异质审查坐实，no-workaround）────────────────────────────────
  W13 `ElementTree.par`（覆盖递归/良基边，约束 `(par e).level < e.level` 驱动 Nat 强归纳下降）
  与 MW1/MW3 `attached`（操作依附边，缠论次级别 Sub = `e.level < attached.level` 依附对象级别
  **更高**）是 **两条不同语义的边**，level 比较方向相反。**冲突只发生在强行同一化
  `attached := par e` 时**——那会使 `e.level > (par e).level`（越级），MW1 把越级归并 Same，所有
  非根角色塌缩 SameDir，三分退化。

  本文件的严格解耦（不强行同一化）：MW8 给每个 W13 树元素 e **独立配置** 操作依附 `α e`（缠论
  依附对象，level 关系按缠论给定，与 W13 良基 `par` 解耦），升格为 `MutexElement {core:=e, attached:=α e}`
  ——并 **强制 `WellAttached`**（ℓ_e ≤ ℓ_{α_e}，划出合法操作依附有效域，**排除越级塌缩**）。覆盖
  递归仍走 W13 `par` 良基树（`recursive_eatSep` 不变），角色三分走 `α` 操作依附边——两结构在同
  一元素集协同，MW8 证 **角色三分叠加在覆盖上不破坏覆盖 + 互斥分配 + 防塌缩**。这不是补丁，是
  把「覆盖良基边」与「操作依附边」严格分离（两条边各管各的，不互相污染）。

  ── owner（互斥铁律）─────────────────────────────────────────────────────────────
  **只建** 本文件（**不** 扩展 W13——共享 owner 冲突；本文件 import W13）。import
  `Origin.SeparateCoverRecursive`（W13）+ `Origin.OperationRole`（MW3）+ `Origin.MutexExhaustive`
  （MW4）。不碰其他文件、不编辑 lakefile.toml。root 名 `Origin.MutexRecursive`。
  验证：`cd formal && lake env lean Origin/MutexRecursive.lean`。禁 sorry/admit/axiom。简体中文。

  ── 认识论等级（formalization-validity-domain 231号强制）─────────────────────────
  **全文件 L0**（纯结构/树归纳/角色三分代数，零数据依赖）。角色三分归纳 = `operationRole`
  四分穷尽（MW3）+ 非根三分（MW3 `nonRoot_role_trichotomy`）+ W13 良基覆盖（树深递减）的结构
  组合。`lake env lean` 通过 = 「W13 ∀e Eat^sep 覆盖 + 角色三分互斥穷尽分配（MW4 Σ指示=1）+
  ShortDiff≠SameDir反向防塌缩」的归纳命题正确。
  ★**∀e Eat^sep 角色版 = 语法覆盖（L0），NOT L2 实盘盈利**。角色三分是「构造的元素集上每元素
    恰属一操作角色 + 被规范腿覆盖」的结构分类（信息增量为零的同义反复），**非** 该角色分类在
    实盘择时有 alpha（全窗 L3 已 8/8 否证完整 v1 择时无 alpha——分类完备性 ≠ 择时盈利）。下游
    **不得** 把角色三分覆盖的有效域膨胀为「角色分类实盘可判定盈利」——那需 L2/L3，本文件不提供
    也不可由 L0 推出。
  ★有效域诚实声明：角色三分覆盖仅在 **覆盖可行域 X^cover_Θ**（W13 `StateMachineProvider` 承载
    W11 `coverFeasible`）+ **合法操作依附域 `WellAttached`**（ℓ_e≤ℓ_{α_e}）内成立。越级依附
    （ℓ_e>ℓ_{α_e}）在有效域外，角色三分精化不适用（继承 W13/MW1 有效域上界）。

  ── 依赖方向（单向无环，全 GREEN 只读）+ 谱系 ──────────────────────────────────────
  MutexRecursive → {SeparateCoverRecursive（W13 覆盖骨架），OperationRole（MW3 角色三分），
                    MutexExhaustive（MW4 Σ指示=1）}。standalone。
  谱系：M26（互斥分类 PDF §19 自相似递归角色精化）→ MW8（精化 W13 C39：二分→三分）→ 下游
        最终互斥定理集成（M29，引本文件角色三分递归 + 防塌缩）。精化 W13 `theorem_separate_recursive`
        （C39 二分）为角色三分互斥穷尽归纳——真增量 = ShortDiff≠SameDir反向（MW3 防塌缩引理）。
-/

import Origin.SeparateCoverRecursive
import Origin.OperationRole
import Origin.MutexExhaustive

namespace NewChanlun.Origin.MutexRecursive

open NewChanlun.Origin
open NewChanlun.Origin.SeparateLedger (Leg)
open NewChanlun.Origin.SeparateEat (LegAssignment EatSep SepCovered)
open NewChanlun.Origin.SeparateCoverRecursive
  (ElementTree StateMachineProvider recursive_eatSep recursive_cover
   theorem_separate_recursive recursive_unique_leg)
open MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    ## §1 角色感知元素树 `RoleAwareElementTree`（M26：W13 覆盖树 + 独立操作依附边 α）

    spec §19 / M26 自相似递归的「角色」侧：在 W13 `ElementTree`（覆盖递归良基树）之上，为每个
    元素配 **独立的操作依附对象 α**（缠论依附，与 W13 良基 `par` 解耦），并升格为 `MutexElement`
    以取角色 `operationRole`。**强制 `WellAttached`**（ℓ_e≤ℓ_{α_e}，排除越级塌缩——异质审查坐实
    的严格性约束）。这是把「覆盖良基边（W13 par）」与「操作依附边（α）」严格分离的载体。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★角色感知元素树 `RoleAwareElementTree`（L0，M26 角色侧）——W13 覆盖树 T + 独立操作依附边 α。

  - `T : ElementTree`：W13 覆盖递归良基树（`par` 覆盖边 + `parent_level_lt` 良基下降 +
    `interval_nested` 区间嵌套）。**覆盖侧由 W13 `recursive_eatSep` 承载，本文件不重证**。
  - `α : SyntaxElement → Option SyntaxElement`：**独立** 操作依附对象（缠论依附，与 W13 `par`
    解耦——`α e` 不必等于 `T.par e`）。根级元素 `α e = none`（ρ_e=Root），非根携依附对象 some。
  - `α_root_iff`：操作依附为空 ⟺ W13 树根（`α e = none ↔ e.isRoot`）——根级语义两边对齐
    （覆盖树根 = 操作依附根 = RootDir）。
  - **`wellAttached`（★防塌缩前置，异质审查坐实）**：每个树元素的升格 `MutexElement` 满足
    `WellAttached`（ℓ_e ≤ ℓ_{α_e}）——划出合法操作依附有效域，**排除越级**（ℓ_e>ℓ_{α_e} 会被
    MW1 归并 Same 致三分塌缩）。这是角色三分非退化的不可省前提。

  ★L0：纯结构（覆盖树 + 操作依附边 + 合法依附约束），无 Θ 参数、不依赖数据。
-/
structure RoleAwareElementTree where
  /-- W13 覆盖递归良基树（覆盖侧由 `recursive_eatSep` 承载，不重证）。 -/
  T : ElementTree
  /-- 独立操作依附对象 α（缠论依附，与 W13 `par` 解耦——不强行同一化）。 -/
  α : SyntaxElement → Option SyntaxElement
  /-- 操作依附根 ⟺ 覆盖树根（根级语义两边对齐：RootDir）。 -/
  α_root_iff : ∀ e, α e = none ↔ e.isRoot
  /-- ★合法操作依附（ℓ_e≤ℓ_{α_e}，排除越级塌缩——三分非退化前置）。 -/
  wellAttached : ∀ e, T.inTree e → WellAttached { core := e, attached := α e }

namespace RoleAwareElementTree

/-- ★元素升格为 `MutexElement`（L0，M26 角色取用）：树元素 e ⟶ `{core:=e, attached:=α e}`，
    以取角色 `operationRole`（覆盖侧仍用 W13 `e : SyntaxElement`，角色侧用此升格）。 -/
def roleElem (R : RoleAwareElementTree) (e : SyntaxElement) : MutexElement :=
  { core := e, attached := R.α e }

/-- ★升格保级别 + 方向（L0）：升格 `MutexElement` 的级别 ℓ_e、方向 ε_e 与 W13 元素一致
    （升格只加操作依附 α，不改 W7 已有 I/ε/ℓ/par）。 -/
theorem roleElem_preserves (R : RoleAwareElementTree) (e : SyntaxElement) :
    (R.roleElem e).level = e.level ∧ (R.roleElem e).core = e :=
  ⟨rfl, rfl⟩

/-- ★升格元素满足合法依附（L0，引 `wellAttached`）：树内元素升格后 `WellAttached`——排除越级，
    角色三分非退化前置。 -/
theorem roleElem_wellAttached (R : RoleAwareElementTree) {e : SyntaxElement}
    (he : R.T.inTree e) : (R.roleElem e).WellAttached :=
  R.wellAttached e he

/-- ★操作依附根 ⟺ 覆盖树根（L0，谓词包装 `α_root_iff`）：升格元素操作依附为空 ⟺ W13 树根。 -/
theorem roleElem_attached_none_iff (R : RoleAwareElementTree) (e : SyntaxElement) :
    (R.roleElem e).attached = none ↔ e.isRoot :=
  R.α_root_iff e

/-- ★角色（M26）：树元素 e 的操作角色 `Role(e) = operationRole(roleElem e)`（升格后取 MW3 角色）。 -/
def role (R : RoleAwareElementTree) (e : SyntaxElement) : OperationRole :=
  (R.roleElem e).operationRole

end RoleAwareElementTree

/-! ════════════════════════════════════════════════════════════════════════
    ## §2 根级 ⟺ RootDir / 非根 ⟺ 角色三分（M26：角色与树根对齐）

    spec §19 / M26：归纳基 = 根级（RootDir 规则覆盖）；归纳步 = 非根元素按三分覆盖。本节坐实
    「W13 树根 ⟺ RootDir」（归纳基对齐）+「非根 ⟺ 角色落 SameDir/SubFollow/ShortDiff 三分之一」
    （归纳步三分穷尽，引 MW3 `root_role_is_rootDir` / `nonRoot_role_trichotomy`）。
    ════════════════════════════════════════════════════════════════════════ -/

namespace RoleAwareElementTree

/-- ★根级元素角色为 RootDir（M26 归纳基，L0）：W13 树根（e.isRoot）⟹ Role(e)=RootDir
    （归纳基「根/最低级由 RootDir 规则覆盖」，spec §19）。由 `α_root_iff` 得操作依附 none，
    引 MW3 `root_role_is_rootDir`。 -/
theorem root_role_is_rootDir (R : RoleAwareElementTree) {e : SyntaxElement} (hr : e.isRoot) :
    R.role e = OperationRole.rootDir := by
  unfold role
  exact MutexElement.root_role_is_rootDir (R.roleElem e) ((R.α_root_iff e).2 hr)

/--
  ★★非根元素角色三分穷尽（M26 归纳步骨架，L0，**MW8 相对 W13 二分的核心精化**）：
  非根树元素 e（¬e.isRoot）⟹ Role(e) ∈ {SameDir, SubFollow, ShortDiff}（三分穷尽，RootDir 被
  非根排除）。这把 W13 归纳步的 **父/子二分**（同向/反向不区分角色）精化为 **角色三分互斥穷尽**
  （spec §19 M26「深度 k 元素按 Same/Sub∧σ/Sub∧−σ 三分覆盖」）。

  证明：非根 ⟹ α e ≠ none ⟹ ∃ p, α e = some p ⟹ MW3 `nonRoot_role_trichotomy`。 -/
theorem nonRoot_role_trichotomy (R : RoleAwareElementTree) {e : SyntaxElement} (hne : ¬ e.isRoot) :
    R.role e = OperationRole.sameDir
    ∨ R.role e = OperationRole.subFollow
    ∨ R.role e = OperationRole.shortDiff := by
  unfold role
  have hsome : R.α e ≠ none := fun h => hne ((R.α_root_iff e).1 h)
  obtain ⟨p, hp⟩ := Option.ne_none_iff_exists'.1 hsome
  exact MutexElement.nonRoot_role_trichotomy (R.roleElem e) p hp

end RoleAwareElementTree

/-! ════════════════════════════════════════════════════════════════════════
    ## §3 角色互斥分配（M26：每元素恰属一角色，MW4 Σ指示=1）

    spec §C / M12「角色互斥穷尽（Σ指示=1）」+ M26「三分互斥穷尽」。本节坐实「每元素腿分配尊重
    角色互斥分类」——每树元素的 `roleIndicatorSum = 1`（MW4，恰一角色），故角色 role-indexed
    腿分配（§4 `RoleLegDuty`）不漏类不重类。这是任务②「每元素腿分配尊重角色互斥分类」的形式化。
    ════════════════════════════════════════════════════════════════════════ -/

namespace RoleAwareElementTree

/-- ★★每元素角色互斥分配（M26 / MW4，L0）：每树元素 e 的角色指示函数求和 = 1
    （`roleIndicatorSum (roleElem e) = 1`）——恰属一操作角色（穷尽 + 互斥 + 唯一归属）。引 MW4
    `roleIndicatorSum_eq_one`。这坐实任务②「腿分配尊重角色互斥分类（MW4 Σ指示=1）」——每元素
    的腿按 **唯一** 角色 role-indexed 分配，无漏类（Σ≥1）无重类（Σ≤1）。 -/
theorem role_indicator_sum_one (R : RoleAwareElementTree) (e : SyntaxElement) :
    (R.roleElem e).roleIndicatorSum = 1 :=
  MutexElement.roleIndicatorSum_eq_one (R.roleElem e)

/-- ★每元素恰属一联合类（M26 / MW4，L0）：每树元素 e 的 (级别关系 ρ_e, 角色 Role(e)) 联合分类
    指示函数求和 = 1（`completeMutexClassificationSum = 1`）——恰属一 (ρ_e, Role) 联合类。引 MW4
    `completeMutexClassificationSum_eq_one`。角色三分递归的每元素在级别×角色联合分类中恰一归属。 -/
theorem joint_classification_sum_one (R : RoleAwareElementTree) (e : SyntaxElement) :
    (R.roleElem e).completeMutexClassificationSum = 1 :=
  MutexElement.completeMutexClassificationSum_eq_one (R.roleElem e)

end RoleAwareElementTree

/-! ════════════════════════════════════════════════════════════════════════
    ## §4 角色 role-indexed 腿分配义务 `RoleLegDuty`（M26：把 Role 真正用进腿分配）

    ★异质审查坐实的真增量条件：角色三分的实质增量不在「漏类防护」（nonRoot_role_trichotomy 只
    防逻辑漏类），而在 **把 Role 真正用进腿分配规则**。本节定义 role-indexed 腿分配义务——每个
    角色对应一种腿分配语义（spec §7/§16）：
      RootDir   ⟹ 独立根级腿（无父，做多/做空绝对方向）。
      SameDir   ⟹ 同级别处理（同层延续/反向切换/反手——**反向仍同级别，不建反向子声部**）。
      SubFollow ⟹ 次级别顺势腿（顺父方向 ε_e=σ_{α_e}，父声部仍有效）。
      ShortDiff ⟹ 次级别反向短差腿（ε_e=−σ_{α_e}，**父声部不动建独立反向子声部**——父腿不删）。
    腿分配按 `role` 唯一路由（§3 Σ指示=1 ⟹ 每元素恰走一条 duty 分支）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★角色腿分配义务 `RoleLegDuty`（M26 role-indexed 腿分配，L0）——四角色各自的腿分配语义标签
  （spec §7/§16，把 Role 真正用进腿分配，非仅漏类防护）：
  - `rootLeg`：根级独立腿（RootDir，无父，绝对方向做多/做空）。
  - `sameLevelLeg`：同级别处理（SameDir，同层延续/反向切换——反向仍同级别不建子声部）。
  - `followLeg`：次级别顺势腿（SubFollow，顺父方向）。
  - `shortDiffLeg`：次级别反向短差腿（ShortDiff，父不动建独立反向子声部，父腿不删）。

  ★四标签与角色一一对应（`legDutyOfRole`）——`DecidableEq` 使路由可机器判定。
-/
inductive RoleLegDuty where
  /-- 根级独立腿（RootDir）：无父，绝对方向做多/做空。 -/
  | rootLeg
  /-- 同级别处理（SameDir）：同层延续/反向切换/反手——反向仍同级别，不建反向子声部。 -/
  | sameLevelLeg
  /-- 次级别顺势腿（SubFollow）：顺父方向 ε_e=σ_{α_e}，父声部仍有效。 -/
  | followLeg
  /-- 次级别反向短差腿（ShortDiff）：ε_e=−σ_{α_e}，父不动建独立反向子声部，父腿不删。 -/
  | shortDiffLeg
deriving DecidableEq, Repr

/-- ★角色 ⟶ 腿分配义务路由 `legDutyOfRole`（M26，L0）：每角色唯一对应一种腿分配义务
    （把 Role 真正用进腿分配，spec §7/§16）。 -/
def legDutyOfRole : OperationRole → RoleLegDuty
  | OperationRole.rootDir => RoleLegDuty.rootLeg
  | OperationRole.sameDir => RoleLegDuty.sameLevelLeg
  | OperationRole.subFollow => RoleLegDuty.followLeg
  | OperationRole.shortDiff => RoleLegDuty.shortDiffLeg

namespace RoleAwareElementTree

/-- ★树元素的腿分配义务（M26）：`legDuty e = legDutyOfRole (role e)`——按角色唯一路由腿分配。 -/
def legDuty (R : RoleAwareElementTree) (e : SyntaxElement) : RoleLegDuty :=
  legDutyOfRole (R.role e)

/-- ★根级 ⟹ 根级独立腿（M26 归纳基腿分配，L0）：W13 树根 ⟹ 腿分配义务 = rootLeg。 -/
theorem root_legDuty (R : RoleAwareElementTree) {e : SyntaxElement} (hr : e.isRoot) :
    R.legDuty e = RoleLegDuty.rootLeg := by
  unfold legDuty; rw [R.root_role_is_rootDir hr]; rfl

/--
  ★★非根 ⟹ 腿分配三分穷尽（M26 归纳步腿分配，L0，**把 Role 真正用进腿分配**）：非根树元素
  e ⟹ 腿分配义务 ∈ {sameLevelLeg, followLeg, shortDiffLeg}（角色三分穷尽 ⟹ 腿分配三分穷尽）。
  这坐实任务「角色三分约束腿分配」——非根元素的腿按角色三分路由（同级别/次级别顺势/次级别短差），
  不走 rootLeg。引 §2 `nonRoot_role_trichotomy`（角色三分）+ `legDutyOfRole` 路由。 -/
theorem nonRoot_legDuty_trichotomy (R : RoleAwareElementTree) {e : SyntaxElement}
    (hne : ¬ e.isRoot) :
    R.legDuty e = RoleLegDuty.sameLevelLeg
    ∨ R.legDuty e = RoleLegDuty.followLeg
    ∨ R.legDuty e = RoleLegDuty.shortDiffLeg := by
  unfold legDuty
  rcases R.nonRoot_role_trichotomy hne with h | h | h <;> rw [h]
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

end RoleAwareElementTree

/-! ════════════════════════════════════════════════════════════════════════
    ## §5 ★防塌缩（M13/M25，MW8 相对 W13 的真增量）：ShortDiff ≠ SameDir 反向

    ★任务铁律：归纳步三分中 ShortDiff（父不动建反向子声部）**必须严格区别于** SameDir 反向
    （同级别关闭/重建），否则三分塌缩回二分 = 互斥性丢失（MW4 Σ指示=1 崩溃）。这是 MW8 相对
    W13 的真增量——W13 二分骨架经 W8 `dir_consistent` 统一承载方向，**不区分** 短差（次级别反向）
    与同级别反向反手；MW8 用 MW3 `sameDir_reverse_ne_shortDiff` / `shortDiff_not_same` 把两者
    在级别维度严格分开（短差严格要 ℓ_e<ℓ_{α_e}=Sub，同级别反向是 ℓ_e=ℓ_{α_e}=Same）。
    ════════════════════════════════════════════════════════════════════════ -/

namespace RoleAwareElementTree

/-- ★★同级别元素的腿分配恒为 sameLevelLeg（M25 / §17 防塌缩，L0）：升格元素若 ρ_e=Same（同级别，
    **含反向反手** ε_e=−σ_{α_e}）⟹ 腿分配义务 = sameLevelLeg（**非** shortDiffLeg）。坐实
    「同级别反向也走同级别腿，不建反向子声部」——同级别反向不因方向相反而变短差腿。引 MW3
    `same_role_is_sameDir`。 -/
theorem same_level_legDuty_is_sameLevelLeg (R : RoleAwareElementTree) {e : SyntaxElement}
    (h : (R.roleElem e).levelRelation = LevelRelation.same) :
    R.legDuty e = RoleLegDuty.sameLevelLeg := by
  unfold legDuty role
  rw [MutexElement.same_role_is_sameDir (R.roleElem e) h]; rfl

/-- ★★★同级别反向 ≠ 短差腿（M25 防塌缩核心，L0，**MW8 真增量**）：升格元素若 ρ_e=Same（同级别，
    含反向 ε_e=−σ_{α_e}）⟹ 腿分配义务 **≠ shortDiffLeg**。坐实任务铁律「ShortDiff 严格区别于
    SameDir 反向」——同级别反向走同级别腿（关闭/重建），**不** 走短差腿（父不动建反向子声部）。
    引 MW3 `sameDir_reverse_ne_shortDiff`（同级别反向角色≠ShortDiff）。

    ★若无此区分，同级别反向会被误归短差腿 ⟹ SameDir/ShortDiff 塌缩 ⟹ MW4 Σ指示=1 崩溃（Σ=2）。 -/
theorem same_level_legDuty_ne_shortDiffLeg (R : RoleAwareElementTree) {e : SyntaxElement}
    (h : (R.roleElem e).levelRelation = LevelRelation.same) :
    R.legDuty e ≠ RoleLegDuty.shortDiffLeg := by
  rw [R.same_level_legDuty_is_sameLevelLeg h]; decide

/-- ★★短差腿 ⟹ 非同级别（M13 防塌缩对偶，L0）：升格元素若腿分配义务 = shortDiffLeg ⟹ ρ_e ≠ Same
    （且 ρ_e=Sub，次级别）。坐实「短差是次级别父不动反向双开，**非** 同级别反手」——从短差腿侧
    确认它不可能是同级别处理。引 MW3 `shortDiff_not_same`。 -/
theorem shortDiffLeg_not_same_level (R : RoleAwareElementTree) {e : SyntaxElement}
    (h : R.legDuty e = RoleLegDuty.shortDiffLeg) :
    (R.roleElem e).levelRelation ≠ LevelRelation.same := by
  -- legDuty=shortDiffLeg ⟹ role=shortDiff（legDutyOfRole 单射于 shortDiff）
  have hrole : R.role e = OperationRole.shortDiff := by
    unfold legDuty legDutyOfRole at h
    rcases MutexElement.operationRole_exhaustive (R.roleElem e) with hr | hr | hr | hr <;>
      unfold role at h ⊢ <;> rw [hr] at h ⊢ <;> first | rfl | (simp at h) | exact hr
  exact MutexElement.shortDiff_not_same (R.roleElem e) hrole

/--
  ★★★短差腿 ≠ 同级别反手（M13 防塌缩主定理，L0，**MW8 相对 W13 的真增量顶点**）：不存在树元素
  同时腿分配为短差腿（shortDiffLeg）且级别关系为同级别（ρ_e=Same，反手的级别关系）。即短差腿
  （次级别父不动反向双开 Sub）与同级别反手（同级别关闭切换 Same）在级别维度 **不可同存**。

  ★这是任务铁律「ShortDiff 严格区别于 SameDir 反向，否则三分塌缩回二分」的形式化顶点——
    短差（Sub）≠ 同级别反向（Same），保证角色三分不塌缩，MW4 Σ指示=1 互斥性成立。引 MW3
    `shortDiff_ne_sameDir_reverse`（无元素同时 ShortDiff ∧ Same）。 -/
theorem shortDiffLeg_ne_same_level_reverse (R : RoleAwareElementTree) (e : SyntaxElement) :
    ¬ (R.legDuty e = RoleLegDuty.shortDiffLeg
       ∧ (R.roleElem e).levelRelation = LevelRelation.same) := by
  rintro ⟨hduty, hsame⟩
  exact (R.same_level_legDuty_ne_shortDiffLeg hsame) hduty

end RoleAwareElementTree

/-! ════════════════════════════════════════════════════════════════════════
    ## §6 角色三分非空性见证（异质审查坐实：trichotomy 不自动给非空性）

    ★异质审查坐实：`nonRoot_role_trichotomy` 三分穷尽 **不保证** 三个分支在某具体树上都非空——
    非空性需要 **具体配置见证**。本节给出三分支各一具体元素见证（SameDir/SubFollow/ShortDiff
    各非空），坐实角色三分是 **真三分**（非退化为某分支空集 = 实质二分/一分）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★角色三分非空见证（M26，L0，三分支各非空）：存在三个升格元素，角色分别为 SameDir /
    SubFollow / ShortDiff（且腿分配义务分别为 sameLevelLeg / followLeg / shortDiffLeg）——坐实
    角色三分是 **真三分**（每分支非空，非退化）。这是异质审查要求的「三分非空性需具体配置见证」。

    构造（固定方向 up，依附对象配置覆盖三角色）：
    - SameDir：依附同级 up（ℓ_e=ℓ_p=0，ρ_e=Same）。
    - SubFollow：依附高级 up（ℓ_e=0<ℓ_p=1，ε_e=σ_p=+1 同向）。
    - ShortDiff：依附高级 down（ℓ_e=0<ℓ_p=1，ε_e=+1=−σ_p，σ_p=−1 反向）。 -/
theorem role_trichotomy_nonempty :
    ∃ (eSame eFollow eShort : MutexElement),
      eSame.operationRole = OperationRole.sameDir
      ∧ eFollow.operationRole = OperationRole.subFollow
      ∧ eShort.operationRole = OperationRole.shortDiff
      ∧ legDutyOfRole eSame.operationRole = RoleLegDuty.sameLevelLeg
      ∧ legDutyOfRole eFollow.operationRole = RoleLegDuty.followLeg
      ∧ legDutyOfRole eShort.operationRole = RoleLegDuty.shortDiffLeg := by
  -- up 元素核（ε=+1），区间 [0,1)
  let coreUp : SyntaxElement :=
    { startIndex := 0, endIndex := 1, nonempty := by decide,
      direction := Direction.up, level := 0, parent := none }
  -- 父：同级 up（level 0）⟹ SameDir
  let parSameUp : SyntaxElement :=
    { startIndex := 0, endIndex := 5, nonempty := by decide,
      direction := Direction.up, level := 0, parent := none }
  -- 父：高级 up（level 1）⟹ SubFollow（子同向 up）
  let parSubUp : SyntaxElement :=
    { startIndex := 0, endIndex := 5, nonempty := by decide,
      direction := Direction.up, level := 1, parent := none }
  -- 父：高级 down（level 1）⟹ ShortDiff（子 up = −σ_p）
  let parSubDown : SyntaxElement :=
    { startIndex := 0, endIndex := 5, nonempty := by decide,
      direction := Direction.down, level := 1, parent := none }
  exact ⟨
    { core := coreUp, attached := some parSameUp },
    { core := coreUp, attached := some parSubUp },
    { core := coreUp, attached := some parSubDown },
    by decide, by decide, by decide, by decide, by decide, by decide⟩

/-! ════════════════════════════════════════════════════════════════════════
    ## §7 ★★★角色感知自相似递归主定理（M26，覆盖 + 角色三分互斥 + 防塌缩合一）

    spec §19 / M26：把 W13 `theorem_separate_recursive`（C39 二分覆盖）精化为 **角色三分互斥
    穷尽** 版——∀e Eat^sep 仍成立（覆盖侧复用 W13）+ 每元素恰属一角色（MW4 Σ指示=1）+ 角色
    三分（非根 SameDir/SubFollow/ShortDiff）+ ShortDiff≠SameDir反向防塌缩。
    ════════════════════════════════════════════════════════════════════════ -/

namespace RoleAwareElementTree

/--
  ★★★★M26 角色感知自相似递归主定理 `theorem_role_aware_recursive`★★★★（L0，spec §19，
  **W13 二分覆盖 → 角色三分互斥穷尽精化**）：

  对角色感知元素树 R 内 **每个** 元素 e（覆盖状态机承载器 smp = W13/W11 投影），同时成立：

      ① 覆盖（∀e Eat^sep，复用 W13）：`EatSep L q e` —— e 被唯一规范头寸腿吃到（角色三分不破坏
        覆盖，覆盖递归走 W13 良基 par 树）。
      ② 角色互斥分配（MW4 Σ指示=1）：`(roleElem e).roleIndicatorSum = 1` —— e 恰属一操作角色。
      ③ 角色三分（M26 归纳步）：e 根级 ⟹ Role=RootDir（腿分配 rootLeg）；非根 ⟹ Role 落
        {SameDir,SubFollow,ShortDiff} 三分之一（腿分配 {sameLevelLeg,followLeg,shortDiffLeg}）。
      ④ ★防塌缩（M13/M25，MW8 真增量）：若 ρ_e=Same（同级别反向反手）⟹ 腿分配 ≠ shortDiffLeg
        （ShortDiff 严格区别于 SameDir 反向，三分不塌缩回二分）。

  ★这是 spec §19 M26「自相似递归归纳步按 Same/SubFollow/ShortDiff 三分互斥穷尽覆盖」的形式化——
    W13 父/子二分覆盖骨架（`recursive_eatSep`）+ 角色三分分类层（MW3/MW4）+ 防塌缩（MW3
    `sameDir_reverse_ne_shortDiff`）的归纳合一。覆盖侧 **不重证 W13**（复用 `recursive_eatSep`），
    真增量在角色三分互斥分配 + 防塌缩。
  ★L0 诚实边界：①覆盖=语法层全元素覆盖、②③④=角色分类结构必然——**NOT** 实盘每笔盈利（全窗 L3
    已 8/8 否证 v1 择时无 alpha；分类完备性≠择时盈利）。仅覆盖可行域 + 合法依附域 WellAttached
    内成立。 -/
theorem theorem_role_aware_recursive {V : Type} (R : RoleAwareElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider R.T L q P) :
    ∀ e, R.T.inTree e →
      -- ① 覆盖（∀e Eat^sep，复用 W13）
      EatSep L q e
      -- ② 角色互斥分配（MW4 Σ指示=1）
      ∧ (R.roleElem e).roleIndicatorSum = 1
      -- ③ 角色三分（根级 RootDir / 非根三分穷尽）
      ∧ ((e.isRoot → R.role e = OperationRole.rootDir)
         ∧ (¬ e.isRoot → R.role e = OperationRole.sameDir
                          ∨ R.role e = OperationRole.subFollow
                          ∨ R.role e = OperationRole.shortDiff))
      -- ④ ★防塌缩（ρ_e=Same ⟹ 腿分配 ≠ shortDiffLeg）
      ∧ ((R.roleElem e).levelRelation = LevelRelation.same
          → R.legDuty e ≠ RoleLegDuty.shortDiffLeg) := by
  intro e he
  refine ⟨?_, ?_, ?_, ?_⟩
  -- ① 覆盖：复用 W13 recursive_eatSep（覆盖侧不重证，角色三分叠加在覆盖上不破坏覆盖）
  · exact recursive_eatSep R.T L q P smp e he
  -- ② 角色互斥分配：MW4 Σ指示=1
  · exact R.role_indicator_sum_one e
  -- ③ 角色三分：根级 RootDir + 非根三分穷尽
  · exact ⟨fun hr => R.root_role_is_rootDir hr, fun hne => R.nonRoot_role_trichotomy hne⟩
  -- ④ 防塌缩：同级别反向 ⟹ 腿分配 ≠ shortDiffLeg（MW3 sameDir_reverse_ne_shortDiff）
  · exact fun hsame => R.same_level_legDuty_ne_shortDiffLeg hsame

/--
  ★★★M26 角色感知递归 ⟹ 全称覆盖（覆盖侧导出，L0）：角色感知元素树内每元素 e 被分账本规范腿
  吃到 `EatSep L q e`（∀e Eat^sep，角色三分版）。由 `theorem_role_aware_recursive` 第①合取项
  ——坐实「角色三分精化不破坏 W13 全称覆盖」（spec §19 M26「三分互斥穷尽覆盖 ⟹ ∀e∈E Eat^sep」）。 -/
theorem role_aware_eatSep {V : Type} (R : RoleAwareElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider R.T L q P) :
    ∀ e, R.T.inTree e → EatSep L q e :=
  fun e he => (R.theorem_role_aware_recursive L q P smp e he).1

/--
  ★★M26 角色感知递归 ⟹ 全称角色互斥分配（MW4 Σ指示=1 导出，L0）：每元素 e 恰属一操作角色
  （`roleIndicatorSum = 1`）。由 `theorem_role_aware_recursive` 第②合取项——任务②「每元素腿分配
  尊重角色互斥分类（MW4 Σ指示=1）」的全称形式。 -/
theorem role_aware_mutex_assignment {V : Type} (R : RoleAwareElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider R.T L q P) :
    ∀ e, R.T.inTree e → (R.roleElem e).roleIndicatorSum = 1 :=
  fun e he => (R.theorem_role_aware_recursive L q P smp e he).2.1

/--
  ★★M26 角色感知递归 ⟹ 全称腿分配三分（M26 归纳步导出，L0）：根级 ⟹ rootLeg，非根 ⟹ 腿分配落
  {sameLevelLeg, followLeg, shortDiffLeg} 三分之一。把角色三分（③）路由到 role-indexed 腿分配
  ——「把 Role 真正用进腿分配」的全称形式（spec §7/§16）。 -/
theorem role_aware_legDuty_trichotomy (R : RoleAwareElementTree) (e : SyntaxElement) :
    (e.isRoot → R.legDuty e = RoleLegDuty.rootLeg)
    ∧ (¬ e.isRoot → R.legDuty e = RoleLegDuty.sameLevelLeg
                    ∨ R.legDuty e = RoleLegDuty.followLeg
                    ∨ R.legDuty e = RoleLegDuty.shortDiffLeg) :=
  ⟨fun hr => R.root_legDuty hr, fun hne => R.nonRoot_legDuty_trichotomy hne⟩

end RoleAwareElementTree

/-! ════════════════════════════════════════════════════════════════════════
    ## §8 诚实标签（formalization-validity-domain gatekeeper，角色三分覆盖 ≠ 实盘盈利）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★角色感知递归裁定标签 `RoleAwareRecursiveVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `allElementsRoleClassifiedAndEatenInCoverDomain`——类型层钉死「覆盖可行域 + 合法依附
  域内，元素树每元素恰属一操作角色（角色三分互斥穷尽）且被唯一规范腿吃到（语法全覆盖）」。
  ★**没有** `NetProfitGuaranteed` / `RoleProfitableInLive` / `EatenInAnyState` 构造子——拒绝三类
    声明膨胀：
    (1)「角色三分覆盖 ⟹ 净账户每笔盈利」（覆盖是语法层，角色是结构分类——C40/C42 否定净账户每笔
        盈利，双开净额退化 G_net=0）；
    (2)「角色分类在实盘择时盈利」（全窗 L3 已 8/8 否证 v1 择时无 alpha；分类完备性 ≠ 择时盈利）；
    (3)「角色三分在任意状态成立」（仅覆盖可行域 X^cover_Θ + 合法依附域 WellAttached 内成立；
        越级依附或 q̄∉K_Θ 时不适用——继承 W13/MW1 有效域上界）。
-/
inductive RoleAwareRecursiveVerdict where
  | allElementsRoleClassifiedAndEatenInCoverDomain
deriving DecidableEq, Repr

/-- ★裁定见证（L0，gatekeeper）：角色感知递归裁定必是「覆盖可行域 + 合法依附域内每元素恰属一
    角色且被唯一规范腿吃到」。支撑：§7 `theorem_role_aware_recursive`（覆盖 + 角色互斥分配 +
    三分 + 防塌缩）+ W13 良基覆盖 + MW4 Σ指示=1 + MW3 防塌缩。角色三分覆盖是覆盖域内语法层结论
    ——**不** 蕴含实盘盈利、**不** 在覆盖域/合法依附域外成立。 -/
theorem role_aware_verdict_is_classified_and_eaten (v : RoleAwareRecursiveVerdict) :
    v = RoleAwareRecursiveVerdict.allElementsRoleClassifiedAndEatenInCoverDomain := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
    ## 交付总结（MW8 工位，M26 角色感知自相似递归三分互斥穷尽归纳）

    本文件**证**（L0，machine-checked，无 sorry/admit/axiom，import W13/MW3/MW4 链）：

    1. §1 角色感知元素树 `RoleAwareElementTree`（M26）：W13 覆盖树 T + **独立操作依附边 α**
       （与 W13 良基 par 解耦，异质审查坐实的严格解耦）+ **`wellAttached`（排除越级塌缩）** +
       `α_root_iff`（操作依附根 ⟺ 覆盖树根）。升格 `roleElem` 取 MW3 角色。

    2. §2 角色与树根对齐：`root_role_is_rootDir`（根级⟹RootDir，归纳基）+
       ★`nonRoot_role_trichotomy`（**非根⟹角色三分 SameDir/SubFollow/ShortDiff，W13 二分→三分的
       核心精化**，引 MW3）。

    3. §3 角色互斥分配（MW4 Σ指示=1）：★`role_indicator_sum_one`（每元素 roleIndicatorSum=1）+
       `joint_classification_sum_one`（联合分类 Σ=1）——任务②形式化。

    4. §4 role-indexed 腿分配 `RoleLegDuty`（把 Role 真正用进腿分配，异质审查坐实的真增量条件）：
       `legDutyOfRole` 路由 + `root_legDuty`（根级⟹rootLeg）+
       ★`nonRoot_legDuty_trichotomy`（非根⟹腿分配三分穷尽）。

    5. ★★§5 防塌缩（M13/M25，**MW8 相对 W13 的真增量**）：
       - `same_level_legDuty_is_sameLevelLeg`：同级别⟹同级别腿（含反向反手）。
       - ★★★`same_level_legDuty_ne_shortDiffLeg`：**同级别反向 ≠ 短差腿**（引 MW3
         `sameDir_reverse_ne_shortDiff`）。
       - `shortDiffLeg_not_same_level`：短差腿⟹非同级别（引 MW3 `shortDiff_not_same`）。
       - ★★★`shortDiffLeg_ne_same_level_reverse`：**短差腿 ≠ 同级别反手**（防塌缩顶点，引 MW3
         `shortDiff_ne_sameDir_reverse`）。

    6. §6 三分非空见证 `role_trichotomy_nonempty`（异质审查坐实：trichotomy 不自动给非空性——
       三分支各具体配置见证 SameDir/SubFollow/ShortDiff 非空，真三分非退化）。

    7. ★★★★§7 角色感知自相似递归主定理（核心产出）：
       - ★★★★`theorem_role_aware_recursive`：∀e，**①覆盖(复用 W13)∧②角色互斥分配(MW4 Σ=1)∧
         ③角色三分(根级RootDir/非根三分)∧④防塌缩(Same⟹≠shortDiffLeg)**——M26 角色三分互斥穷尽
         归纳合一（W13 二分→三分精化）。
       - `role_aware_eatSep`：∀e EatSep（覆盖侧导出，角色三分不破坏覆盖）。
       - `role_aware_mutex_assignment`：∀e roleIndicatorSum=1（角色互斥分配全称）。
       - `role_aware_legDuty_trichotomy`：根级 rootLeg / 非根腿分配三分（把 Role 用进腿分配全称）。

    8. §8 诚实标签 `role_aware_verdict_is_classified_and_eaten`（裁定=覆盖域+合法依附域内每元素
       恰属一角色且被吃到，无「净盈利保证」/「实盘择时盈利」/「任意状态成立」构造子）。

    本文件**不证**（formalization-validity-domain 诚实边界）：
    - ✗ 角色三分覆盖 ⟹ 实盘盈利 / 净账户每笔盈利（覆盖是 L0 语法层；C40/C42 否定净账户每笔盈利）。
    - ✗ 角色分类在实盘择时盈利（全窗 L3 已 8/8 否证 v1 无 alpha；分类完备性≠择时盈利）。
    - ✗ 角色三分在合法依附域外（越级 ℓ_e>ℓ_{α_e}）或覆盖可行域外成立（继承 W13/MW1 有效域上界）。
    - ✗ W13 覆盖递归内部（`recursive_eatSep` 复用不重证）/ W11 q*=q̄（StateMachineProvider 承载前提）。

    ★下游引用（最终互斥定理 M29 集成直接引）：
    - 角色三分递归：`theorem_role_aware_recursive`（M26 覆盖+角色互斥+三分+防塌缩合一）。
    - 覆盖侧：`role_aware_eatSep`（∀e EatSep 角色版）。
    - 互斥分配：`role_aware_mutex_assignment`（∀e Σ指示=1）。
    - ★防塌缩：`shortDiffLeg_ne_same_level_reverse`（短差≠同级别反手，M29 互斥性顶点引）/
      `same_level_legDuty_ne_shortDiffLeg`。
    - 角色感知树：`RoleAwareElementTree`（M29 在其上集成）/ `legDutyOfRole`（腿分配路由）。

    谱系：M26（互斥分类 PDF §19 自相似递归角色精化）→ W13（C39 二分覆盖骨架，覆盖侧复用）+
          MW3（角色三分 + 防塌缩引理）+ MW4（Σ指示=1）→ 本文件 MW8（二分→三分互斥穷尽精化）→
          下游 M29 最终互斥集成。真增量 = ShortDiff≠SameDir反向防塌缩（W13 二分骨架无此区分）。
          有效域上界 = 覆盖可行域 + 合法依附域内角色三分覆盖（C40/C42 净资产盈利 + L3 实盘择时
          盈利均被否定）。
    ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.MutexRecursive
