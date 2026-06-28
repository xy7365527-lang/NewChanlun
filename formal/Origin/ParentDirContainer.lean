/-
  Origin/ParentDirContainer.lean — σ_p(g) 来自父容器方向（spec §7.2 + 最高级别走势类型.pdf）

  认识论等级：L0（纯定义推导，不依赖市场数据）。

  ★口径修正（取代旧 ParentDirActive.lean「活动父腿」错口径）：
    spec line 367-381 §7.2：σ_{p(g)} = 父容器 p(g)∈C_{ℓ_g+1} 的方向 ∈ {-1,0,+1}。
    「最高级别走势类型.pdf」§2/§4/§5/§12 逐字确认：角色 R 不是裸元素单元函数 R(g)，
    而是相对「当前可见操作容器 c」的关系 R_t(γ)；c ∈ {确认走势,活动走势,候选中枢,胚元}，
    σ_t(c) 是容器自身方向，从当前自底向上结构 D_t 自上而下查得——**与是否持仓无关**。

    旧 ParentDirActive 用 `if memD p At`（父腿是否在持仓集 A_t）门控 σ_p，是把
    §7.2 的「σ 来源=父容器方向」与 §13 的「AncOK 持仓准入=父腿在 A_t」两个正交机制
    混为一谈。实质差异（非措辞）：父容器有向但未持仓的逆向次级点，错口径判 Ambient
    （→当独立根仓建 naked 逆势仓），正确口径判 ShortDiff（→AncOK 在未持父时剔除，不开仓）。
    错口径制造虚假独立逆势仓 = garbage trades。

  范式：纯 core，import Origin.ActiveSet（含 OperationRole18/CandidateSet 传递依赖）。
  禁 sorry/admit/native_decide/Mathlib/Finset/Fintype/lake clean。
-/

import Origin.ActiveSet

open NewChanlun.Origin (Side RoleV classifyV classifyV_germ_ambient)
open NewChanlun.Origin.CandidateSet (Cand)

namespace NewChanlun.Origin.ParentDirContainer

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 核心定义：父方向来自父容器（结构对象，非持仓）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **父方向来自父容器 `parentDirOfContainer`（L0 定义，spec §7.2 line 369）** ——
  给定结构父函数 `par : Cand → Option Cand`（spec p:C_ℓ→C_{ℓ+1}，候选所属走势元素在
  规范递归塔里的 RMove::Compose 父容器；`none` = 父为边界胚元 ∂，spec P4 去根化）、候选 `g`：
  - `par g = some p` → 返回 `some p.dir`（σ_p(g) = 父容器方向 δ(p)）
  - `par g = none`  → `none`（σ_p(g) = 0，父=胚元 ∂ → Ambient）

  **关键**：σ_p 只取 `par g`（结构父容器），**不引用任何持仓集 A_t**——与旧
  `parentDirFromActive` 的 `if memD p At` 门控相反。父容器从当前 D_t（≤t 的自底向上闭包）
  查得，活动走势的当前方向因果合法（只用 ≤t 数据），不需父被「持有」。 -/
def parentDirOfContainer (par : Cand → Option Cand) (g : Cand) : Option Side :=
  match par g with
  | none   => none
  | some p => some p.dir

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 主定理一：σ_p=0 ↔ 父=胚元 ∂（去根化边界，非持仓门控）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★**σ_p=none ↔ 父容器为胚元（L0）**——
  `parentDirOfContainer = none` 当且仅当 `par g = none`（父=边界胚元 ∂）。
  这是去根化边界（spec P4/P7）的精确刻画：σ_p=0 的唯一来源是「父=胚元」，
  **不是**「父未持有」（旧错口径的 `memD = false` 分支被删除）。 -/
theorem parentDirOfContainer_none_iff_germ
    (par : Cand → Option Cand) (g : Cand) :
    parentDirOfContainer par g = none ↔ par g = none := by
  cases hpar : par g with
  | none   => simp [parentDirOfContainer, hpar]
  | some p => simp [parentDirOfContainer, hpar]

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 主定理二：V=Ambient ↔ 父=胚元（接 classifyV，去根化）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★**V=Ambient ↔ 父容器为胚元（L0）**——
  `classifyV (parentDirOfContainer par g) δ = Ambient ↔ par g = none`。
  坐实「Ambient = 父容器无方向（胚元 ∂）时的正确垂直角色，非 Root 特例」（去根化）。
  与旧口径的关键差异：Ambient 的充要条件是「父=胚元」，**不含**「父有向但未持仓」。 -/
theorem classifyV_ambient_iff_germ
    (par : Cand → Option Cand) (g : Cand) (δ : Side) :
    classifyV (parentDirOfContainer par g) δ = RoleV.ambient ↔ par g = none := by
  rw [← parentDirOfContainer_none_iff_germ par g]
  constructor
  · intro h
    cases hv : parentDirOfContainer par g with
    | none => rfl
    | some s =>
      rw [hv] at h
      simp only [classifyV] at h
      by_cases hds : δ = s
      · simp [hds] at h
      · simp [hds] at h
  · intro hno
    rw [hno]
    exact classifyV_germ_ambient δ

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 主定理三：有向父容器 → V 完全确定（FollowParent/ShortDiff）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★**有向父容器时 V(g) 完全确定（L0，执行层接口）**——
  `par g = some p` → `classifyV (parentDirOfContainer par g) δ`
  = `if δ = p.dir then FollowParent else ShortDiff`。
  父容器有向（活动/确认走势）即足以定 V，**无需父被持有**——这正是修正口径的执行层接口：
  σ_p 来自 D_t 中父容器方向 δ(p)，直接喂 classifyV 产出垂直角色。
  ShortDiff（δ=-σ_p）的「持仓准入」是下游 §13 AncOK 的独立约束，不在本层。 -/
theorem classifyV_of_directional_container
    (par : Cand → Option Cand) (g p : Cand) (δ : Side)
    (hpar : par g = some p) :
    classifyV (parentDirOfContainer par g) δ =
      if δ = p.dir then RoleV.followParent else RoleV.shortDiff := by
  have hpd : parentDirOfContainer par g = some p.dir := by
    simp [parentDirOfContainer, hpar]
  rw [hpd]
  rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 主定理四：因果/结构独立性（σ_p 只依赖结构父，不依赖持仓/未来）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★★**因果合法性 = 结构独立性（L0，非平凡不变性）**：
  `parentDirOfContainer` 只依赖结构父 `par g`——若 `par₁ g = par₂ g` 则结果相同。

  这是对旧口径错误的根本纠正：旧 `parentDirFromActive` 把 σ_p 写成 `At`（持仓集）的函数，
  本定义**根本不接受 At 参数**——σ_p 不可能依赖持仓状态。父容器从 D_t（≤t 自底向上闭包）
  查得，故 σ_p 因果合法**by construction**：唯一输入 `par g` 是 D_t 的结构对象，
  只用 ≤t 数据，与未来数据/持仓账本均无关。 -/
theorem parentDirOfContainer_struct_only
    (par₁ par₂ : Cand → Option Cand) (g : Cand)
    (h : par₁ g = par₂ g) :
    parentDirOfContainer par₁ g = parentDirOfContainer par₂ g := by
  unfold parentDirOfContainer
  rw [h]

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 公理核验（#print axioms）
    ═══════════════════════════════════════════════════════════════════════ -/

#print axioms parentDirOfContainer_none_iff_germ
#print axioms classifyV_ambient_iff_germ
#print axioms classifyV_of_directional_container
#print axioms parentDirOfContainer_struct_only

end NewChanlun.Origin.ParentDirContainer
