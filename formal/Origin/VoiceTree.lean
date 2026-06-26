/-
  Origin/VoiceTree.lean — 赋格声部树（port 到 Origin canonical base）
  ★task #114, A′ 策略组件港入 Origin canonical base（审计 B：VoiceTree MISSING in Origin）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**。审计 B 判决：赋格声部树
  VoiceTree **MISSING in Origin，只在 legacy Strict/StrategyFamily.lean §5**。本文件把它
  **重锚到 Origin 类型**：复用 Origin `SourceAxioms` 的 `Side`（long/short 域，非 Bool 占位）
  作声部方向类型，定理挂 Origin 命名空间 `NewChanlun.Origin.VoiceTree`，**不** import legacy。

  ── 核心结构（继承 legacy §5）──────────────────────────────────────────────────
  多声部（赋格）结构：每个声部节点的方向 = 其父节点方向的翻转（σ_v = flip σ_{p(v)}）。
  级联关闭：父声部关闭 ⟹ 所有后代声部关闭（对无环 + 深度良基结构的祖先关系归纳）。
  多空双开：声部树**不禁止**同时存在 long 与 short 声部。

  ★相对 legacy 的 port 强化：legacy 用 `Bool`（true=long/false=short）编码方向；本文件用
    Origin canonical `Side`（`SourceAxioms.Side`，long/short 两构造子 + DecidableEq），方向翻转
    用 `Side.flip`（本文件定义，对合）。语义同构（两态对合无不动点），但锚 Origin 类型而非
    Bool 占位——这使「赋格交替」直接陈述在缠论方向域 Side 上（更忠实多空语义）。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数/归纳，不依赖数据）。机器可检验：交替律对合、相邻声部反向、
  祖先关闭传递闭包、无环（祖先深度严格降）、多空双开结构允许。
  **不**声称任何 L1+ 经验有效性（实际声部如何开/平是 Θ-参数化执行，属本文件有效域外）。

  ── 诚实标注（no-patch-mentality）──────────────────────────────────────────────
  本结构**公理强制的**只有「无环」（depth_decreasing ⟹ not_self_ancestor）+「parent 链深度
  严格降」。**不强制**「有限」（V 无 Finite 约束）也**不强制**「有根」（不证每节点经有限步
  parent 必达某 none 根）。级联关闭定理只需「无环 + 深度良基」即成立。

  ── 依赖方向（单向无环，不 import #113 分类血肉、不 import legacy Strict）──────
  VoiceTree → Origin.SourceAxioms（仅用 Side）。standalone。
  验证：`cd formal && lake env lean Origin/VoiceTree.lean`。禁 sorry/admit/axiom。

  谱系：legacy Strict/StrategyFamily.lean §5（cc-strategyfamily #69）→ #96/#97（A′ canonical）→
        本文件 #114（VoiceTree 港入 Origin）。
-/

import Origin.SourceAxioms

namespace NewChanlun.Origin.VoiceTree

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 声部方向翻转（赋格交替算子，锚 Origin Side）

  方向翻转 `Side.flip`：long ↔ short（Origin canonical Side 上的对合，赋格交替的算子）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★声部方向翻转 `flip`（赋格交替算子，L0）：long ↔ short（Origin Side 上的对合）。 -/
def flip : Side → Side
  | Side.long => Side.short
  | Side.short => Side.long

/-- ★翻转对合（L0）：翻两次还原（方向两态）。 -/
theorem flip_flip (s : Side) : flip (flip s) = s := by
  cases s <;> rfl

/-- ★翻转改变方向（L0）：flip s ≠ s（两态对合无不动点，赋格交替非平凡）。 -/
theorem flip_ne (s : Side) : flip s ≠ s := by
  cases s <;> simp [flip]

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 声部树结构（无环 + 深度良基 + 赋格交替 + 级联关闭）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部树 `VoiceTree V`（L0，port 到 Origin）：无环 + 深度良基的多声部父链结构。

  - `parent : V → Option V`：父声部（`none` = 局部根）。
  - `side : V → Side`：声部方向（Origin canonical Side：long/short）。
  - `closed : V → Bool`：声部是否已关闭（仓位平掉）。
  - `depth : V → Nat`：节点深度——**无环良基**编码（祖先深度严格更小）。
  - `alternating`：**赋格交替** `parent v = some p → side v = flip (side p)`（σ_v = -σ_{p(v)}）。
  - `depth_decreasing`：**无环 + 良基** `parent v = some p → depth p < depth v`（parent 链严格降）。
  - `cascade_close`：**级联关闭单步** `parent v = some p → closed p = true → closed v = true`。

  ★诚实标注：公理强制的只有「无环」+「parent 链深度严格降」。**不**强制「有限/有根」。
  级联关闭定理（ancestor_closed）只需「无环 + 深度良基」即成立。
-/
structure VoiceTree (V : Type) where
  parent : V → Option V
  side : V → Side
  closed : V → Bool
  depth : V → Nat
  /-- 赋格交替：子声部方向 = 父声部翻转（σ_v = -σ_{p(v)}）。 -/
  alternating : ∀ v p, parent v = some p → side v = flip (side p)
  /-- 无环 + 良基：父深度 < 子深度（parent 链深度严格降，故无环；不强制必达根）。 -/
  depth_decreasing : ∀ v p, parent v = some p → depth p < depth v
  /-- 级联关闭单步：父关 ⟹ 直接子关。 -/
  cascade_close : ∀ v p, parent v = some p → closed p = true → closed v = true

/--
  ★相邻声部反向（L0，赋格交替推论）：子声部方向 ≠ 父声部方向。
  `parent v = some p → side v ≠ side p`——相邻声部必反向（σ_v = -σ_{p(v)} ⟹ σ_v ≠ σ_{p(v)}）。
-/
theorem VoiceTree.adjacent_opposite {V : Type} (T : VoiceTree V)
    (v p : V) (hpar : T.parent v = some p) :
    T.side v ≠ T.side p := by
  rw [T.alternating v p hpar]
  exact flip_ne (T.side p)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 祖先关系 + 级联关闭传递闭包 + 无环见证
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★祖先关系 `IsAncestor`（L0）：`IsAncestor T a v` ⟺ a 是 v 的祖先（经 ≥1 步 parent 链可达）。
  归纳定义：直接父是祖先（base）；祖先的祖先是祖先（step）。
-/
inductive IsAncestor {V : Type} (T : VoiceTree V) : V → V → Prop where
  | base : ∀ v p, T.parent v = some p → IsAncestor T p v
  | step : ∀ v p a, T.parent v = some p → IsAncestor T a p → IsAncestor T a v

/--
  ★★级联关闭（传递闭包，L0，核心）：祖先关闭 ⟹ 后代关闭。
  `IsAncestor T a v → closed a = true → closed v = true`——任一祖先声部关闭，则该后代关闭
  （不只直接父，整条祖先链的关闭都级联到后代）。对 `IsAncestor` 归纳：
  - base（直接父）：`cascade_close` 单步律直接给。
  - step（父 p 的祖先）：归纳假设给 p 关闭，再 `cascade_close` 从 p 到 v。
-/
theorem VoiceTree.ancestor_closed {V : Type} (T : VoiceTree V)
    (a v : V) (hanc : IsAncestor T a v) (hclosed : T.closed a = true) :
    T.closed v = true := by
  induction hanc with
  | base v p hpar =>
      exact T.cascade_close v p hpar hclosed
  | step v p a hpar _hanc_a_p ih =>
      exact T.cascade_close v p hpar (ih hclosed)

/--
  ★祖先深度严格更小（L0，无环良基推论）：`IsAncestor T a v → depth a < depth v`。
  从 `depth_decreasing` 沿祖先链累积。保证 IsAncestor 无环，级联关闭归纳良基。
-/
theorem VoiceTree.ancestor_depth_lt {V : Type} (T : VoiceTree V)
    (a v : V) (hanc : IsAncestor T a v) :
    T.depth a < T.depth v := by
  induction hanc with
  | base v p hpar =>
      exact T.depth_decreasing v p hpar
  | step v p a hpar _hanc ih =>
      exact Nat.lt_trans ih (T.depth_decreasing v p hpar)

/--
  ★声部不是自己的祖先（L0，无环见证）：`¬ IsAncestor T v v`。
  若 v 是自己的祖先则 `depth v < depth v`（ancestor_depth_lt），矛盾。关死声部树的环。
-/
theorem VoiceTree.not_self_ancestor {V : Type} (T : VoiceTree V) (v : V) :
    ¬ IsAncestor T v v := by
  intro h
  exact Nat.lt_irrefl (T.depth v) (T.ancestor_depth_lt v v h)

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 多空双开结构允许（不加 Q⁺Q⁻=0 禁令）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★多空双开结构允许（L0）：声部树**不禁止**同时存在 long 与 short 声部。

  存在一棵声部树 T，含两个未关闭声部 vL（side=long）与 vS（side=short）——即结构上多空双开
  **可表达**（不被类型/约束排除）。与赋格交替不冲突：交替约束的是**父子**方向（σ_v = -σ_{p(v)}），
  不约束**非父子**声部的方向组合，故多空并存（不同子树或同层）结构合法。

  ★诚实标注：只证「结构允许」（双开可表达且满足所有 VoiceTree 公理）。**不**证「双开总可行」
  ——实际双开是否满足保证金/容量约束是 Θ-参数化风险约束（见 Origin/RiskProj.lean），属
  EmpiricalDomain，不由此 L0 声称。
-/
theorem long_short_both_open_allowed :
    ∃ (T : VoiceTree Side) (vL vS : Side),
      T.side vL = Side.long ∧ T.side vS = Side.short ∧
      T.closed vL = false ∧ T.closed vS = false := by
  -- 用 V = Side：vL = long（long 声部）, vS = short（short 声部），两者都是根（parent = none）
  refine ⟨{
    parent := fun _ => none          -- 两声部都是根（无父，故 alternating 真空满足）
    side := fun v => v                -- Side 自身即方向
    closed := fun _ => false          -- 都未关闭
    depth := fun _ => 0               -- 都在根层
    alternating := by intro v p hpar; simp at hpar
    depth_decreasing := by intro v p hpar; simp at hpar
    cascade_close := by intro v p hpar; simp at hpar
  }, Side.long, Side.short, rfl, rfl, rfl, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部树标签 `VoiceTreeTag`（gatekeeper，诚实分层）。
  StructuralFugueOnly（赋格交替 + 级联关闭是**结构事实**，非语义分类）+
  OperationalSemanticsOnly（声部开/平是给定 Θ 的操作语义）+ EmpiricalDomain（实际可行性 L3）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把声部树标为分类定理。
-/
inductive VoiceTreeTag where
  | StructuralFugueOnly
  | OperationalSemanticsOnly
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★声部树子类（gatekeeper）：AlternatingCascadeFugue（唯一子类）。 -/
inductive VoiceTreeSubkind where
  | AlternatingCascadeFugue
deriving DecidableEq, Repr

/-- ★声部树诚实标签包（L0 声明）。 -/
def voiceTreeLabels : List VoiceTreeTag × VoiceTreeSubkind :=
  ([VoiceTreeTag.StructuralFugueOnly, VoiceTreeTag.OperationalSemanticsOnly,
    VoiceTreeTag.EmpiricalDomain],
   VoiceTreeSubkind.AlternatingCascadeFugue)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：声部树子类必是 AlternatingCascadeFugue。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝冒充真完全分类（非跨库强保证）。 -/
theorem voiceTree_not_true_classification (k : VoiceTreeSubkind) :
    k = VoiceTreeSubkind.AlternatingCascadeFugue := by
  cases k; rfl

end NewChanlun.Origin.VoiceTree
