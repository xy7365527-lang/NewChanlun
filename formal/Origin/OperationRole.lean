/-
  Origin/OperationRole.lean — MW3 角色四分类 Role(e) + Side⊥Role 二维正交 + 短差绝对方向 + 短差非反手

  ── 存在论位置（M09/M10/M11/M13，互斥分类 canonical spec §B 表 + §A 页3-4 §3）──────────
  本文件形式化角色互斥分类链的「角色侧」四条 canonical 条目：
  - **M09 角色四分类** `Role(e) ∈ {RootDir, SameDir, SubFollow, ShortDiff}`：
        RootDir   ⟺ ρ_e = Root（根方向元素，独立做多/做空）
        SameDir   ⟺ ρ_e = Same（同级别元素，同层延续/反向切换/反手）
        SubFollow ⟺ ρ_e = Sub ∧ ε_e = σ_{α_e}（次级别顺势腿，非短差）
        ShortDiff ⟺ ρ_e = Sub ∧ ε_e = −σ_{α_e}（短差元素，父声部不动建独立反向子声部）
    Role 是级别关系 ρ_e（MW1 `levelRelation`，Root/Same/Sub）× 方向关系（ε_e vs σ_{α_e}，同/反）
    的 **纯组合派生函数**（单一真相源，非硬编码字段）——四分互斥穷尽是派生函数的结构定理。
  - **M10 Side ⊥ Role 二维正交**：方向 `Side(e) = ε_e ∈ {+1,-1}`（绝对做多/做空）与角色
    `Role(e)`（操作角色）是 **不同维度**——两者独立（同一 Side 可对应不同 Role，同一 Role 可对应
    不同 Side），无函数依赖。形式化为「Side 与 Role 的所有组合在结构上可表达」（正交见证）。
  - **M11 短差绝对方向（精化 C03 赋格交替）**：`ShortDiff ⟹ Side(e) = −σ_{α_e}`——父级多头
    （σ_{α_e}=+1）的短差是次级别做空（ε_e=−1）/ 父级空头（σ_{α_e}=−1）的短差是次级别做多
    （ε_e=+1）。短差的 **绝对方向由父级方向翻转决定**，这是 C03 赋格交替 `σ_v=−σ_{p(v)}` 在
    元素侧的精化（对接 `Origin.VoiceTree.flip` 的方向翻转语义）。
  - **★M13 短差非反手（防塌缩关键）**：短差（ShortDiff，ρ_e=Sub 父仓保持次级别反向双开）
    **≠ 同级别反向反手**（SameDir 反向，ρ_e=Same 关闭旧声部建新方向声部）。短差严格要求
    ℓ_e < ℓ_{α_e}（次级别），同级别反向是 ℓ_e = ℓ_{α_e}（同级别切换）——级别侧 ρ_e 不同
    （Sub vs Same）⟹ 角色不同（ShortDiff vs SameDir）。**这是 MW4 互斥穷尽（M12 Σ指示=1）
    的防塌缩支撑**：若把短差等同同级别反向反手，则 SameDir 与 ShortDiff 塌缩，互斥性崩溃。

  ── ★防塌缩边界（任务铁律：短差非反手是 MW4 互斥性的关键）────────────────────────────
  spec §C 强调两个角色不混同证明（防止四类塌缩）：
  - M25（同级别反向 ≠ 短差）：本文件 `sameDir_reverse_ne_shortDiff` —— 同级别反向（Same+反向）
    的角色是 SameDir 而非 ShortDiff（级别侧 Same≠Sub）。保证 SameDir 与 ShortDiff 互斥。
  - M11/M13（短差 ≠ 普通做空/反手）：本文件 `shortDiff_abs_dir`（短差绝对方向由父级翻转决定，
    与 RootDir 独立绝对方向区分）+ `shortDiff_ne_sameDir_reverse`（短差是次级别双开，反手是
    同级别切换，两者不同）。

  ── owner（互斥铁律）─────────────────────────────────────────────────────────────
  **只建** 本文件。import `Origin.MutexElement`（MW1 `MutexElement`/`LevelRelation`/`levelRelation`/
  `attached`/`level`/`dirSign`，已 GREEN）+ `Origin.VoiceTree`（C03 赋格交替 flip/Side，只读地基）+
  `Origin.SyntaxElement`（依附对象 σ_{α_e} = 依附 `SyntaxElement.dirSign`）。不碰其他文件、
  不编辑 lakefile.toml。root 名 `Origin.OperationRole`。

  ── 认识论等级（formalization-validity-domain 231号强制）─────────────────────────
  全部 **L0**（纯结构定义 / 角色分类 = 级别关系 ρ_e × 方向关系的纯组合）。Role 四分类是
  spec §3 散文「四类角色」的结构转录，由 `levelRelation`（MW1）+ 方向比较（ε_e vs σ_{α_e}）
  代数派生，四分互斥穷尽是派生函数的结构必然。**信息增量为零的同义反复**（L0）：角色分类
  在「构造的元素集」上成立 ≠ 在真实市场有效。本文件 **不** 声称任何 L1+ 经验有效性——
  「父级多头短差=做空」是内在操作语义命题（spec §3/§16），非现实市场盈利命题（有效域 ⊊
  定义域；不得膨胀为「短差实盘盈利」，那需 L2/L3）。M11 对接 VoiceTree.flip 是 L0 结构对应。

  ── 依赖方向（单向无环）+ 验证 ─────────────────────────────────────────────────
  OperationRole → {Origin.MutexElement（MW1），Origin.VoiceTree（C03 地基，只读），
                   Origin.SyntaxElement（依附对象方向）}。standalone，不碰其他文件（owner 互斥）。
  验证：`cd formal && lake env lean Origin/OperationRole.lean`。禁 sorry/admit/axiom。简体中文。

  谱系：M09/M10/M11/M13（互斥分类 PDF §3 角色四分类 + 二维正交 + 短差绝对方向 + 短差非反手）
        → MW3（角色侧）→ 下游 MW4（M12 互斥穷尽 Σ指示=1）/ MW5（M25 同级反向≠短差）/
        MW8（M26 三分归纳）。M11 精化 C03 赋格交替（VoiceTree.flip）。
-/

import Origin.MutexElement
import Origin.VoiceTree

namespace NewChanlun.Origin

open MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 角色四分类标签 `Role(e) ∈ {RootDir, SameDir, SubFollow, ShortDiff}`（M09）

    spec §A 页3 §3：`Role(e) ∈ {RootDir, SameDir, SubFollow, ShortDiff}` 四类角色定义。本节
    定义 `OperationRole` 为四构造子归纳类型——四分的「互斥穷尽」由归纳类型的结构必然保证
    （每个值恰是四构造子之一，无第五种、无重叠），配 `DecidableEq` 使「唯一判定」可机器判定。
    这是 MW4 角色互斥穷尽（M12 Σ指示=1）的角色侧基底。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **角色四分类标签 Role(e)（M09，spec §A 页3 §3）** —— 元素的操作角色，四值穷尽：
  - `rootDir`：根方向（M09，ρ_e=Root，独立做多/做空，无依附对象）。
  - `sameDir`：同级别（M09，ρ_e=Same，同层延续 / 反向切换/反手——**反向也是 SameDir 非短差**）。
  - `subFollow`：次级别顺势（M09，ρ_e=Sub ∧ ε_e=σ_{α_e}，顺父方向次级别腿，非短差）。
  - `shortDiff`：短差（M09，ρ_e=Sub ∧ ε_e=−σ_{α_e}，父声部不动建独立反向子声部）。

  ★四构造子归纳类型 ⟹ 互斥穷尽是结构必然（每值恰一构造子）。`DecidableEq` ⟹ 唯一判定可机器化。
  这是 MW4 角色互斥穷尽（M12 Σ指示=1）的角色侧基底。
  ★L0：四值标签是 spec §3 散文「四类角色」的结构转录，无 Θ 参数、不依赖数据。
-/
inductive OperationRole where
  /-- 根方向（M09）：ρ_e=Root，独立做多/做空。 -/
  | rootDir
  /-- 同级别（M09）：ρ_e=Same，同层延续或反向切换/反手（反向仍非短差）。 -/
  | sameDir
  /-- 次级别顺势（M09）：ρ_e=Sub ∧ ε_e=σ_{α_e}，顺父方向次级别腿。 -/
  | subFollow
  /-- 短差（M09）：ρ_e=Sub ∧ ε_e=−σ_{α_e}，父仓保持次级别反向双开。 -/
  | shortDiff
deriving DecidableEq, Repr

namespace OperationRole

/-- ★四值穷尽（L0，归纳类型结构必然）：任一 Role(e) 必是四构造子之一。坐实 spec M09 四类角色
    「穷尽」——无第五种角色（角色四分类完备）。MW4（M12 Σ指示=1）的角色侧穷尽基底。 -/
theorem exhaustive (R : OperationRole) :
    R = rootDir ∨ R = sameDir ∨ R = subFollow ∨ R = shortDiff := by
  cases R
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr rfl))

/-- ★rootDir ≠ sameDir（L0，互斥分量）。 -/
theorem rootDir_ne_sameDir : rootDir ≠ sameDir := by decide
/-- ★rootDir ≠ subFollow（L0，互斥分量）。 -/
theorem rootDir_ne_subFollow : rootDir ≠ subFollow := by decide
/-- ★rootDir ≠ shortDiff（L0，互斥分量）。 -/
theorem rootDir_ne_shortDiff : rootDir ≠ shortDiff := by decide
/-- ★sameDir ≠ subFollow（L0，互斥分量）。 -/
theorem sameDir_ne_subFollow : sameDir ≠ subFollow := by decide
/-- ★★sameDir ≠ shortDiff（L0，互斥分量 / M13 防塌缩关键）：同级别（含反向反手）≠ 短差。
    这是 M25「同级别反向≠短差」的角色侧根据——SameDir 与 ShortDiff 是不同构造子，类型层
    不可塌缩。MW4 互斥穷尽（M12）防止把短差等同同级别反手的具体支撑。 -/
theorem sameDir_ne_shortDiff : sameDir ≠ shortDiff := by decide
/-- ★subFollow ≠ shortDiff（L0，互斥分量）：次级别顺势（ε_e=σ_{α_e}）≠ 短差（ε_e=−σ_{α_e}）。 -/
theorem subFollow_ne_shortDiff : subFollow ≠ shortDiff := by decide

end OperationRole

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 方向关系（ε_e vs σ_{α_e}）+ 角色派生函数 `Role(e) = operationRole(e)`（M09）

    Role 不作为字段，而由 `operationRole`（依 `levelRelation` + 方向比较 ε_e vs σ_{α_e}）派生
    （单一真相源，非硬编码字段）。四分互斥穷尽是派生函数的结构定理。这避免「字段 Role 与
    (ρ_e, ε_e, σ_{α_e}) 不一致」的可能矛盾。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★绝对方向 Side(e) = ε_e ∈ {+1,-1}（M10，透传 MW1 `dirSign`）。这是元素的 **绝对做多/做空**
    方向，与角色 Role(e) 是不同维度（M10 Side⊥Role）。 -/
def side (e : MutexElement) : Int := e.dirSign

/-- ★依附对象方向 σ_{α_e} ∈ {+1,-1,0}（M07/M08/M11 比较用）：根级（attached=none）记 σ_{α_e}=0
    （占位，根级不参与同/反向判定——`operationRole` 先判 Root）；非根取依附对象 `SyntaxElement.dirSign`。 -/
def attachedDirSign (e : MutexElement) : Int :=
  match e.attached with
  | none => 0
  | some p => p.dirSign

/--
  **方向同向谓词 `sameDirection`（ε_e = σ_{α_e}，M08 次级别顺势侧）** —— 元素方向与依附对象方向
  相同（次级别顺势腿 SubFollow 的方向条件）。仅在非根（attached=some）时有意义；根级
  （attached=none，σ_{α_e}=0）下 `dirSign ∈ {+1,-1} ≠ 0`，故 `sameDirection` 为假（不影响——
  根级角色由 ρ_e=Root 先判为 RootDir）。 -/
def sameDirection (e : MutexElement) : Prop := e.dirSign = e.attachedDirSign

instance (e : MutexElement) : Decidable e.sameDirection := by
  unfold sameDirection; infer_instance

/--
  **方向反向谓词 `oppositeDirection`（ε_e = −σ_{α_e}，M08 短差侧）** —— 元素方向与依附对象方向
  相反（短差 ShortDiff 的方向条件）。同 `sameDirection`，仅非根有意义。 -/
def oppositeDirection (e : MutexElement) : Prop := e.dirSign = - e.attachedDirSign

instance (e : MutexElement) : Decidable e.oppositeDirection := by
  unfold oppositeDirection; infer_instance

/--
  **角色派生函数 Role(e) = operationRole(e)（M09，spec §3）** —— 从级别关系 ρ_e（MW1
  `levelRelation`）+ 方向关系（ε_e vs σ_{α_e}）唯一派生 Role（单一真相源，非硬编码字段）：
  - `ρ_e = Root`                ⟹ `rootDir`（M09：根方向，独立做多/做空）。
  - `ρ_e = Same`                ⟹ `sameDir`（M09：同级别，同向延续或反向切换/反手，**反向仍 SameDir**）。
  - `ρ_e = Sub, ε_e = σ_{α_e}`  ⟹ `subFollow`（M09：次级别顺势腿）。
  - `ρ_e = Sub, ε_e ≠ σ_{α_e}`  ⟹ `shortDiff`（M09：次级别反向 = 短差；ε_e=−σ_{α_e} 由方向二值得）。

  ★全函数（定义域 = 所有元素）：`levelRelation` 全函数 + Nat/Int 比较可判定。
  ★唯一判定（spec §3「操作语义必须唯一判定」）：函数输出确定 ⟹ 每元素 Role 唯一。
  ★Sub 下的方向二分：方向 ε_e ∈ {+1,-1} 二值，σ_{α_e} ∈ {+1,-1}（非根），故 ε_e=σ_{α_e}（顺势）
  与 ε_e≠σ_{α_e}（即 ε_e=−σ_{α_e}，短差）互斥穷尽——这是 spec §C「方向二分完备」。 -/
def operationRole (e : MutexElement) : OperationRole :=
  match e.levelRelation with
  | LevelRelation.root => OperationRole.rootDir
  | LevelRelation.same => OperationRole.sameDir
  | LevelRelation.sub =>
      if e.dirSign = e.attachedDirSign then OperationRole.subFollow else OperationRole.shortDiff

/-! ── Role 四分类的级别关系判据（M09 四分式坐实）──────────────────────────────────── -/

/-- ★M09 根方向判据（L0）：Role(e) = RootDir ⟺ ρ_e = Root。坐实 spec M09「RootDir ⟺ ρ_e=Root」。 -/
theorem operationRole_rootDir_iff (e : MutexElement) :
    e.operationRole = OperationRole.rootDir ↔ e.levelRelation = LevelRelation.root := by
  unfold operationRole
  cases h : e.levelRelation with
  | root => simp
  | same => simp
  | sub => by_cases hd : e.dirSign = e.attachedDirSign <;> simp [hd]

/-- ★M09 同级别判据（L0）：Role(e) = SameDir ⟺ ρ_e = Same。坐实 spec M09「SameDir ⟺ ρ_e=Same」。
    ★注意：Same 的同向/反向都映射到 SameDir（同级别反向是反手/切换，**仍 SameDir 非短差**——
    M13/M25 防塌缩）。 -/
theorem operationRole_sameDir_iff (e : MutexElement) :
    e.operationRole = OperationRole.sameDir ↔ e.levelRelation = LevelRelation.same := by
  unfold operationRole
  cases h : e.levelRelation with
  | root => simp
  | same => simp
  | sub => by_cases hd : e.dirSign = e.attachedDirSign <;> simp [hd]

/-- ★M09 次级别顺势判据（L0）：Role(e) = SubFollow ⟺ ρ_e = Sub ∧ ε_e = σ_{α_e}。
    坐实 spec M09「SubFollow ⟺ ρ_e=Sub ∧ ε_e=σ_{α_e}」（次级别顺势腿）。 -/
theorem operationRole_subFollow_iff (e : MutexElement) :
    e.operationRole = OperationRole.subFollow
      ↔ e.levelRelation = LevelRelation.sub ∧ e.dirSign = e.attachedDirSign := by
  unfold operationRole
  cases h : e.levelRelation with
  | root => simp
  | same => simp
  | sub => by_cases hd : e.dirSign = e.attachedDirSign <;> simp [hd]

/-- ★M09 短差判据（L0）：Role(e) = ShortDiff ⟺ ρ_e = Sub ∧ ε_e ≠ σ_{α_e}。
    坐实 spec M09「ShortDiff ⟺ ρ_e=Sub ∧ ε_e=−σ_{α_e}」（方向二值下 ε_e≠σ_{α_e} ⟺ ε_e=−σ_{α_e}，
    见 `shortDiff_iff_opposite`）。 -/
theorem operationRole_shortDiff_iff (e : MutexElement) :
    e.operationRole = OperationRole.shortDiff
      ↔ e.levelRelation = LevelRelation.sub ∧ e.dirSign ≠ e.attachedDirSign := by
  unfold operationRole
  cases h : e.levelRelation with
  | root => simp
  | same => simp
  | sub => by_cases hd : e.dirSign = e.attachedDirSign <;> simp [hd]

/-! ── Role 四分类穷尽 + 唯一 + 互斥（M09 / MW4 M12 Σ指示=1 的角色侧基底）─────────────── -/

/-- ★Role 四分穷尽（L0，M09）：每元素 Role 必是四角色之一。由 `OperationRole` 归纳类型穷尽 +
    `operationRole` 全函数。坐实 spec §C「角色四分类穷尽」——MW4（M12 Σ指示=1）穷尽基底。 -/
theorem operationRole_exhaustive (e : MutexElement) :
    e.operationRole = OperationRole.rootDir
    ∨ e.operationRole = OperationRole.sameDir
    ∨ e.operationRole = OperationRole.subFollow
    ∨ e.operationRole = OperationRole.shortDiff :=
  OperationRole.exhaustive e.operationRole

/-- ★Role 四分唯一判定（L0，spec §3「操作语义必须唯一判定」）：Role 由 `operationRole` 函数
    确定，故对每元素唯一。 -/
theorem operationRole_unique (e : MutexElement) :
    ∀ R₁ R₂, e.operationRole = R₁ → e.operationRole = R₂ → R₁ = R₂ := by
  intro R₁ R₂ h₁ h₂; rw [← h₁, ← h₂]

/-- ★★Role 四分互斥（L0，M09 / §C「互斥」/ M12 Σ指示=1）：RootDir/SameDir/SubFollow/ShortDiff
    两两不相交——元素的 Role 恰一个为真（指示函数和=1，恰一个分支成立）。这是 MW4 角色互斥穷尽
    定理（M12）的核心结构——四个互斥分支，每元素恰落一个。 -/
theorem operationRole_mutex (e : MutexElement) :
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
       ∧ e.operationRole = OperationRole.shortDiff) := by
  rcases operationRole_exhaustive e with h | h | h | h
  · exact Or.inl ⟨h, by rw [h]; decide, by rw [h]; decide, by rw [h]; decide⟩
  · exact Or.inr (Or.inl ⟨by rw [h]; decide, h, by rw [h]; decide, by rw [h]; decide⟩)
  · exact Or.inr (Or.inr (Or.inl ⟨by rw [h]; decide, by rw [h]; decide, h, by rw [h]; decide⟩))
  · exact Or.inr (Or.inr (Or.inr ⟨by rw [h]; decide, by rw [h]; decide, by rw [h]; decide, h⟩))

end MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 3. Side ⊥ Role 二维正交（M10）

    spec §A 页3-4 §3：`Side(e) = ε_e ∈ {做多,做空}` 与 `Role(e) ∈ {根方向,同级别,顺势次级别,短差}`
    是 **不同维度**（M10）。本节形式化正交性——Side 与 Role 之间无函数依赖（同 Side 不同 Role，
    同 Role 不同 Side 都可表达），坐实「方向 ⊥ 角色」的二维分解（互斥分类的关键洞察）。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★Side 取值穷尽（L0，M10）：Side(e) = ε_e = +1（做多）∨ = −1（做空）。透传 `dirSign` 二值
    （`direction_sign_cases`）。绝对方向只有做多/做空两值。 -/
theorem side_cases (e : MutexElement) : e.side = 1 ∨ e.side = -1 := by
  unfold side dirSign SyntaxElement.dirSign
  exact direction_sign_cases e.core.direction

/-- ★Side ≠ 0（L0，M10）：绝对方向非零（避免退化）。 -/
theorem side_ne_zero (e : MutexElement) : e.side ≠ 0 := by
  rcases side_cases e with h | h <;> rw [h] <;> decide

end MutexElement

/-- ★Side ⊥ Role 正交（L0，M10，正交见证）：方向 Side 与角色 Role 是独立维度——存在四个元素，
    **同一 Side（做多 ε=+1）对应四个不同 Role**（RootDir/SameDir/SubFollow/ShortDiff）。这坐实
    Side 不决定 Role（同 Side 可任意 Role），是「方向 ⊥ 角色」二维分解的正交性见证之一支。

    构造：固定方向 up（Side=+1），通过依附对象与级别配置覆盖四角色：
    - RootDir：attached=none（ρ_e=Root）。
    - SameDir：attached=some p, ℓ_e=ℓ_p（ρ_e=Same）。
    - SubFollow：attached=some p, ℓ_e<ℓ_p, ε_e=σ_p=+1（同向，p 也 up）。
    - ShortDiff：attached=some p, ℓ_e<ℓ_p, ε_e=−σ_p（p 为 down，σ_p=−1，ε_e=+1=−σ_p）。 -/
theorem side_orthogonal_role_same_side :
    ∃ (e₁ e₂ e₃ e₄ : MutexElement),
      e₁.side = 1 ∧ e₂.side = 1 ∧ e₃.side = 1 ∧ e₄.side = 1 ∧
      e₁.operationRole = OperationRole.rootDir ∧
      e₂.operationRole = OperationRole.sameDir ∧
      e₃.operationRole = OperationRole.subFollow ∧
      e₄.operationRole = OperationRole.shortDiff := by
  -- up 元素核（ε=+1），区间 [0,1)
  let coreUp : SyntaxElement :=
    { startIndex := 0, endIndex := 1, nonempty := by decide,
      direction := Direction.up, level := 0, parent := none }
  -- 父：同级 up（level 0），用于 SameDir
  let parSameUp : SyntaxElement :=
    { startIndex := 0, endIndex := 5, nonempty := by decide,
      direction := Direction.up, level := 0, parent := none }
  -- 父：高级 up（level 1），用于 SubFollow（子同向 up）
  let parSubUp : SyntaxElement :=
    { startIndex := 0, endIndex := 5, nonempty := by decide,
      direction := Direction.up, level := 1, parent := none }
  -- 父：高级 down（level 1），用于 ShortDiff（子 up = −σ_p）
  let parSubDown : SyntaxElement :=
    { startIndex := 0, endIndex := 5, nonempty := by decide,
      direction := Direction.down, level := 1, parent := none }
  refine ⟨
    { core := coreUp, attached := none },
    { core := coreUp, attached := some parSameUp },
    { core := coreUp, attached := some parSubUp },
    { core := coreUp, attached := some parSubDown },
    ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  -- Side = +1（四元素同核 coreUp）
  · decide
  · decide
  · decide
  · decide
  -- RootDir（attached=none）
  · decide
  -- SameDir（ℓ_e=ℓ_p=0）
  · decide
  -- SubFollow（ℓ_e=0<ℓ_p=1, ε_e=+1=σ_p）
  · decide
  -- ShortDiff（ℓ_e=0<ℓ_p=1, ε_e=+1=−σ_p, σ_p=−1）
  · decide

/-! ════════════════════════════════════════════════════════════════════════
    § 4. 短差绝对方向（M11，精化 C03 赋格交替，对接 VoiceTree.flip）

    spec §A 页4 §3 / 页13-14 §16：`ShortDiff ⟹ Side(e) = −σ_{α_e}`——父级多头（σ_{α_e}=+1）的
    短差是次级别做空（ε_e=−1）/ 父级空头（σ_{α_e}=−1）的短差是次级别做多（ε_e=+1）。短差的
    绝对方向由父级方向 **翻转** 决定，这是 C03 赋格交替 `σ_v=−σ_{p(v)}`（`Origin.VoiceTree.flip`）
    在元素侧的精化。本节坐实短差绝对方向定理 + 与 VoiceTree.flip 的语义对应。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★短差方向二值化（L0）：Role(e)=ShortDiff ⟹ ε_e ≠ σ_{α_e}（短差判据的方向部分）。 -/
theorem shortDiff_dir_ne (e : MutexElement) (h : e.operationRole = OperationRole.shortDiff) :
    e.dirSign ≠ e.attachedDirSign :=
  ((operationRole_shortDiff_iff e).1 h).2

/-- ★短差是次级别（L0）：Role(e)=ShortDiff ⟹ ρ_e = Sub（短差判据的级别部分）。 -/
theorem shortDiff_isSub (e : MutexElement) (h : e.operationRole = OperationRole.shortDiff) :
    e.levelRelation = LevelRelation.sub :=
  ((operationRole_shortDiff_iff e).1 h).1

/-- ★依附对象方向二值（L0，非根）：非根元素（attached=some p）的 σ_{α_e} = ±1（依附对象方向符号，
    `SyntaxElement.dirSign` 二值）。短差是次级别（ρ_e=Sub）必非根，故 σ_{α_e} ∈ {+1,-1}。 -/
theorem attachedDirSign_cases_of_some (e : MutexElement) (p : SyntaxElement)
    (hp : e.attached = some p) : e.attachedDirSign = 1 ∨ e.attachedDirSign = -1 := by
  unfold attachedDirSign
  rw [hp]
  exact direction_sign_cases p.direction

/-- ★次级别必非根（L0）：ρ_e = Sub ⟹ ∃ p, attached = some p（次级别有依附对象）。 -/
theorem sub_has_attached (e : MutexElement) (h : e.levelRelation = LevelRelation.sub) :
    ∃ p, e.attached = some p := by
  rcases (levelRelation_sub_iff e).1 h with ⟨p, hp, _⟩
  exact ⟨p, hp⟩

/--
  ★★短差绝对方向定理（M11，L0，精化 C03 赋格交替）：Role(e)=ShortDiff ⟹ `Side(e) = −σ_{α_e}`
  （ε_e = −σ_{α_e}）——短差的绝对方向由父级方向 **翻转** 决定。

  证明：短差 ⟹ ρ_e=Sub（非根，σ_{α_e}=±1）∧ ε_e≠σ_{α_e}。ε_e ∈ {+1,-1}，σ_{α_e} ∈ {+1,-1}，
  二值下 ε_e≠σ_{α_e} ⟺ ε_e=−σ_{α_e}。坐实 spec M11「父多头短差=做空 / 父空头短差=做多」。 -/
theorem shortDiff_abs_dir (e : MutexElement) (h : e.operationRole = OperationRole.shortDiff) :
    e.side = - e.attachedDirSign := by
  unfold side
  have hsub : e.levelRelation = LevelRelation.sub := shortDiff_isSub e h
  have hne : e.dirSign ≠ e.attachedDirSign := shortDiff_dir_ne e h
  obtain ⟨p, hp⟩ := sub_has_attached e hsub
  have hatt : e.attachedDirSign = 1 ∨ e.attachedDirSign = -1 :=
    attachedDirSign_cases_of_some e p hp
  have heps : e.dirSign = 1 ∨ e.dirSign = -1 := by
    unfold dirSign SyntaxElement.dirSign
    exact direction_sign_cases e.core.direction
  rcases heps with he | he <;> rcases hatt with ha | ha <;>
    rw [he, ha] <;> rw [he, ha] at hne <;> first | rfl | (exact absurd rfl hne)

/-- ★父级多头短差=做空（M11，L0，spec §3/§16）：父级方向 σ_{α_e}=+1（多头）⟹ 短差方向
    ε_e=−1（次级别做空）。`shortDiff_abs_dir` 在 σ_{α_e}=+1 的实例。 -/
theorem shortDiff_long_parent_is_short (e : MutexElement)
    (h : e.operationRole = OperationRole.shortDiff) (hp : e.attachedDirSign = 1) :
    e.side = -1 := by
  rw [shortDiff_abs_dir e h, hp]

/-- ★父级空头短差=做多（M11，L0，spec §3/§16）：父级方向 σ_{α_e}=−1（空头）⟹ 短差方向
    ε_e=+1（次级别做多）。`shortDiff_abs_dir` 在 σ_{α_e}=−1 的实例。 -/
theorem shortDiff_short_parent_is_long (e : MutexElement)
    (h : e.operationRole = OperationRole.shortDiff) (hp : e.attachedDirSign = -1) :
    e.side = 1 := by
  rw [shortDiff_abs_dir e h, hp]; rfl

/-- ★短差判据方向等价（M09 短差判据的方向二值精化，L0）：在短差（ρ_e=Sub, σ_{α_e}=±1）下，
    `ε_e ≠ σ_{α_e}`（operationRole 用的判据）⟺ `ε_e = −σ_{α_e}`（spec M09 原文）。坐实
    `operationRole_shortDiff_iff` 的 `≠` 形式与 spec 的 `=−σ` 形式在方向二值下等价。 -/
theorem shortDiff_iff_opposite (e : MutexElement) (p : SyntaxElement)
    (hp : e.attached = some p) :
    (e.dirSign ≠ e.attachedDirSign) ↔ (e.dirSign = - e.attachedDirSign) := by
  have hatt : e.attachedDirSign = 1 ∨ e.attachedDirSign = -1 :=
    attachedDirSign_cases_of_some e p hp
  have heps : e.dirSign = 1 ∨ e.dirSign = -1 := by
    unfold dirSign SyntaxElement.dirSign
    exact direction_sign_cases e.core.direction
  rcases heps with he | he <;> rcases hatt with ha | ha <;>
    rw [he, ha] <;> constructor <;> intro hh <;>
    first | rfl | (exact absurd rfl hh) | (exact absurd hh (by decide)) | decide

end MutexElement

/-! ── M11 对接 VoiceTree 赋格交替（C03 地基，flip 语义对应）─────────────────────────── -/

/-- ★Side(Int) ⟷ Origin Side 对应：Int 方向符号 +1/−1 ⟷ Origin canonical `Side.long`/`Side.short`
    （VoiceTree 的方向域，来自 `SourceAxioms`）。把元素侧绝对方向（Int）映射到 C03 赋格交替的
    载体方向域，用于陈述 M11 短差绝对方向与赋格交替 `flip` 的语义对应。 -/
def intSignToVoiceSide (s : Int) : Side :=
  if s = 1 then Side.long else Side.short

/-- ★Int 符号翻转 ⟷ VoiceTree.flip 对应（L0，M11 精化 C03 见证）：对 ±1 符号，`intSignToVoiceSide`
    把「Int 取反 −s」对应到 VoiceTree「flip」（赋格交替算子）。坐实短差绝对方向 ε_e=−σ_{α_e}
    （Int 翻转）= C03 赋格交替 σ_v=flip σ_{p(v)}（VoiceTree.flip）在元素侧的精化。 -/
theorem intSignToVoiceSide_neg (s : Int) (hs : s = 1 ∨ s = -1) :
    intSignToVoiceSide (- s) = VoiceTree.flip (intSignToVoiceSide s) := by
  rcases hs with h | h <;> subst h <;> rfl

/-- ★★短差绝对方向 = 父方向赋格翻转（M11，L0，精化 C03）：Role(e)=ShortDiff ⟹ 元素绝对方向
    （映射到 VoiceTree.Side）= 父级方向的 `flip`（赋格交替翻转）。这是 M11「短差绝对方向由父级
    方向翻转决定」与 C03 赋格交替 `σ_v=flip σ_{p(v)}`（`Origin.VoiceTree`）的语义对应——短差是
    赋格交替在元素操作角色侧的精化（spec M11 精化 C03）。 -/
theorem shortDiff_voiceSide_eq_flip_parent (e : MutexElement) (p : SyntaxElement)
    (h : e.operationRole = OperationRole.shortDiff) (hp : e.attached = some p) :
    intSignToVoiceSide e.side = VoiceTree.flip (intSignToVoiceSide e.attachedDirSign) := by
  have hside : e.side = - e.attachedDirSign := MutexElement.shortDiff_abs_dir e h
  have hatt : e.attachedDirSign = 1 ∨ e.attachedDirSign = -1 :=
    MutexElement.attachedDirSign_cases_of_some e p hp
  rw [hside, intSignToVoiceSide_neg e.attachedDirSign hatt]

/-! ════════════════════════════════════════════════════════════════════════
    § 5. 短差非反手（M13，防塌缩关键）+ 同级别反向≠短差（M25 支撑）

    spec §A 页3/页7 §2.3/§7.4（M13）+ 页14 §17（M25）：**短差（ShortDiff，ρ_e=Sub 次级别父仓
    保持反向双开）≠ 同级别反向反手（SameDir 反向，ρ_e=Same 关闭旧声部建新方向声部）**。这是
    防止 SameDir/ShortDiff 塌缩、保证 MW4 角色互斥（M12）成立的关键。本节坐实：
    - 短差严格要求 ρ_e=Sub（ℓ_e<ℓ_{α_e}），同级别反向是 ρ_e=Same（ℓ_e=ℓ_{α_e}）⟹ 角色不同。
    - 「同级别反向」（Same + ε_e=−σ_{α_e}）的角色仍是 SameDir（非 ShortDiff）——M25 核心。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★同级别元素角色恒为 SameDir（L0，M25 / §17）：ρ_e=Same ⟹ Role(e)=SameDir，**无论方向**
    （同向延续 ε_e=σ_{α_e} 或反向反手 ε_e=−σ_{α_e}）。坐实 spec M25「同级别反向也是 SameDir」
    ——同级别反向不因方向相反而变成短差（短差严格要 ρ_e=Sub）。 -/
theorem same_role_is_sameDir (e : MutexElement) (h : e.levelRelation = LevelRelation.same) :
    e.operationRole = OperationRole.sameDir :=
  (operationRole_sameDir_iff e).2 h

/-- ★★同级别反向 ≠ 短差（M25，L0，防塌缩核心）：ρ_e=Same（同级别，含反向 ε_e=−σ_{α_e}）⟹
    Role(e) ≠ ShortDiff。坐实 spec §17/M25「同级别反向≠短差」——同级别反向的角色是 SameDir
    而非 ShortDiff，因短差定义严格要求 ℓ_e<ℓ_{α_e}（Sub），同级别反向是 ℓ_e=ℓ_{α_e}（Same）。
    **MW4 互斥（M12）的具体支撑**：Same 与 Sub 级别侧不可混淆 ⟹ SameDir 与 ShortDiff 角色互斥。 -/
theorem sameDir_reverse_ne_shortDiff (e : MutexElement) (h : e.levelRelation = LevelRelation.same) :
    e.operationRole ≠ OperationRole.shortDiff := by
  rw [same_role_is_sameDir e h]
  exact OperationRole.sameDir_ne_shortDiff

/-- ★★短差 ⟹ 非同级别（M13，L0，防塌缩对偶）：Role(e)=ShortDiff ⟹ ρ_e ≠ Same（且 ρ_e=Sub）。
    坐实 spec M13「短差非反手」——短差是次级别（Sub），反手是同级别（Same），两者级别关系不同。
    与 `sameDir_reverse_ne_shortDiff` 对偶：从短差侧确认它不可能是同级别反手。 -/
theorem shortDiff_not_same (e : MutexElement) (h : e.operationRole = OperationRole.shortDiff) :
    e.levelRelation ≠ LevelRelation.same := by
  have hsub : e.levelRelation = LevelRelation.sub := shortDiff_isSub e h
  rw [hsub]
  exact LevelRelation.same_ne_sub.symm

/--
  ★★短差 ≠ 同级别反手（M13，L0，防塌缩主定理）：不存在元素同时是短差（Role=ShortDiff）且是
  同级别（ρ_e=Same，反手的级别关系）。即短差与同级别反手在级别维度上不可同存——短差是次级别
  父仓保持双开（Sub），同级别反手是同级别关闭切换（Same），spec §C「防止四类塌缩」的核心。 -/
theorem shortDiff_ne_sameDir_reverse (e : MutexElement) :
    ¬ (e.operationRole = OperationRole.shortDiff ∧ e.levelRelation = LevelRelation.same) := by
  rintro ⟨hrole, hsame⟩
  exact (sameDir_reverse_ne_shortDiff e hsame) hrole

end MutexElement

/-! ════════════════════════════════════════════════════════════════════════
    § 6. 角色三分归纳支撑（M26 / MW8：Same/SubFollow/ShortDiff 三分互斥穷尽）

    spec §A 页15 §19 / M26：自相似递归归纳步按 ρ_e=Same / Sub∧σ / Sub∧−σ **三分别由
    SameDir/SubFollow/ShortDiff 规则覆盖，三种情况互斥穷尽**。本节为 MW8 角色感知自相似递归
    提供「非根元素角色三分」的互斥穷尽引理（根级 RootDir 是归纳基；非根元素恰落三角色之一）。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexElement

/-- ★非根元素角色三分穷尽（M26 / MW8，L0）：非根元素（attached=some p，ρ_e≠Root）的角色必是
    SameDir / SubFollow / ShortDiff 之一（RootDir 已被 attached=some 排除）。坐实 spec M26 归纳步
    「深度 k 元素按 Same/Sub∧σ/Sub∧−σ 三分覆盖」的三分穷尽——MW8 自相似递归的归纳步骨架。 -/
theorem nonRoot_role_trichotomy (e : MutexElement) (p : SyntaxElement)
    (hp : e.attached = some p) :
    e.operationRole = OperationRole.sameDir
    ∨ e.operationRole = OperationRole.subFollow
    ∨ e.operationRole = OperationRole.shortDiff := by
  -- 非根 ⟹ ρ_e ≠ Root ⟹ Role ≠ RootDir，由四分穷尽得余下三分
  have hnotRoot : e.levelRelation ≠ LevelRelation.root := by
    rw [Ne, levelRelation_root_iff e, hp]; simp
  have hnotRootDir : e.operationRole ≠ OperationRole.rootDir := by
    rw [Ne, operationRole_rootDir_iff e]; exact hnotRoot
  rcases operationRole_exhaustive e with h | h | h | h
  · exact absurd h hnotRootDir
  · exact Or.inl h
  · exact Or.inr (Or.inl h)
  · exact Or.inr (Or.inr h)

/-- ★根级元素角色为 RootDir（M26 / MW8 归纳基，L0）：根级（attached=none）⟹ Role(e)=RootDir。
    坐实 spec M26 归纳基「最低级/根元素由 RootDir 规则覆盖」。 -/
theorem root_role_is_rootDir (e : MutexElement) (h : e.attached = none) :
    e.operationRole = OperationRole.rootDir := by
  rw [operationRole_rootDir_iff e, levelRelation_root_iff e]; exact h

/-- ★角色完全决定于（attached, ρ_e, 方向关系）（M26 / MW8，L0，单一真相源收尾）：根级⟹RootDir、
    同级别⟹SameDir、次级别顺势⟹SubFollow、次级别反向⟹ShortDiff——四情形互斥穷尽覆盖所有元素。
    这把 spec M26 自相似递归归纳步的「三分（+ 根级基）穷尽覆盖」收为一条全覆盖引理。 -/
theorem role_total_coverage (e : MutexElement) :
    (e.attached = none ∧ e.operationRole = OperationRole.rootDir)
    ∨ (e.levelRelation = LevelRelation.same ∧ e.operationRole = OperationRole.sameDir)
    ∨ (e.levelRelation = LevelRelation.sub ∧ e.dirSign = e.attachedDirSign
        ∧ e.operationRole = OperationRole.subFollow)
    ∨ (e.levelRelation = LevelRelation.sub ∧ e.dirSign ≠ e.attachedDirSign
        ∧ e.operationRole = OperationRole.shortDiff) := by
  rcases levelRelation_exhaustive e with hr | hs | hsub
  · exact Or.inl ⟨(levelRelation_root_iff e).1 hr, (operationRole_rootDir_iff e).2 hr⟩
  · exact Or.inr (Or.inl ⟨hs, (operationRole_sameDir_iff e).2 hs⟩)
  · by_cases hd : e.dirSign = e.attachedDirSign
    · exact Or.inr (Or.inr (Or.inl ⟨hsub, hd, (operationRole_subFollow_iff e).2 ⟨hsub, hd⟩⟩))
    · exact Or.inr (Or.inr (Or.inr ⟨hsub, hd, (operationRole_shortDiff_iff e).2 ⟨hsub, hd⟩⟩))

end MutexElement

end NewChanlun.Origin
