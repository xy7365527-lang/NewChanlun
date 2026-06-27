/-
  Origin/GlobalProductClassification.lean — 全局乘积分类的父子一致性 G_α
  （PDF-only 真缺口 2b：C^global_α = G_α ∧ ⋂_v C_{v,α_v}，∀x ∃!α）

  ── 存在论位置（PDF-math 审计精确定位）─────────────────────────────────────────
  当前 repo 全局分类已有 ⋂_v + ∃!α：
    Foundation/CompleteClassification.lean §GlobalVoice（global_class_complete_unique）+
    Strict/Chain.lean（globalClassify_total_unique / globalClassify_partition）。
  **但** `GlobalClassSpec`（CompleteClassification.lean:173）仅 `∀v, Local v x (g v)`——
  它把全局 α 标注当作**各声部 local 标注的任意笛卡尔积**（每声部独立 local 唯一，拼起来即得 ∃!g）。

  PDF p24 给的是更强的 `C^global_α = G_α ∧ ⋂_v C_{v,α_v}`：α 标注须额外满足**父子一致性 G_α**
  ——声部树 α 标注不是任意 local 标注的笛卡尔积，而要满足 σ_v = −σ_{p(v)}（赋格交替）+
  a_v ≤ a_{p(v)}（激活祖先闭合：子激活 ⟹ 父激活）+ 同单位数（父子声部同 unit 计数）。
  本文件**忠实编码 G_α** 并补回这条缺口（PDF p24 + canonical §8-9 多重赋格声部树 +
  祖先闭合 + 同单位数双开）。

  ── G_α 的忠实形式（PDF p24 + Strict/Fugue.lean G_v + Origin/VoiceTree 父子）────────
  α : V → LocalClass 是声部树 V 上的全局标注。从每个 local class 投影出三个分量：
    `dirOf  : LocalClass → Side`（声部方向 σ_v）
    `actOf  : LocalClass → Bool`（声部是否激活 a_v：是否有持仓 / 已开仓）
    `unitOf : LocalClass → Nat` （声部单位数 / 手数计数）
  G_α 是三条父子一致性合取（仅约束**直接父子**，由 VoiceTree.parent 给）：
    (G1 交替) parent v = some p → dirOf (α v) = flip (dirOf (α p))    （σ_v = −σ_{p(v)}）
    (G2 闭合) parent v = some p → actOf (α v) = true → actOf (α p) = true （a_v ≤ a_{p(v)}）
    (G3 同单) parent v = some p → actOf (α v) = true → unitOf (α v) = unitOf (α p） （激活子父同单位数）

  ── ∃! 保持的严格根据（无声明膨胀，no-patch §5）────────────────────────────────
  G_α 是**真约束**（缩小 α 空间——存在违反 G_α 的标注，见 `Gconsistency_nontrivial`）。
  关键：∃! 不被破坏，因为 G_α 约束的三分量在缠论结构下**由 local 唯一性 + 树结构确定性传播**，
  不是「从多个候选 α 中筛选」。具体地，当 local 分类的投影满足 §3 的**结构语义前提**
  （H_dir：dirOf 与树 alternating 一致；H_act / H_unit：激活/单位投影满足父子语义），
  逐声部唯一的 α = globalClass 自动满足 G_α（`globalClass_satisfies_G`）。故
    `global_product_complete_unique : ∀x ∃!α, C^global_α(x)`
  在该前提下成立（`refine_globalClassSpec`：C^global ⟹ GlobalClassSpec，子集 + ∃! 保持）。

  ★诚实分叉裁定（任务要求的两支）：
  - 不撞定义层。G_α 与现有 ∃!α **不冲突**——前提是结构语义条件（H_dir/H_act/H_unit）由
    VoiceTree 公理 + local 分类语义给定（非外加在 α 上的筛选）。在该前提下唯一 α 满足 G_α。
  - 若**去掉**结构语义前提（任意 dirOf/actOf/unitOf 投影 + 任意 Local），则确有 x 的唯一 α
    违反 G_α（存在性失败）——这正是 `Gconsistency_nontrivial` 见证的「G_α 非平凡」。本文件
    把这一支**显式编码为前提的必要性**（前提缺失 ⟹ 无解可表达），不掩盖、不 workaround。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数/逻辑，不依赖数据）。机器可检验：G_α 三合取定义、非平凡见证、
  唯一 α 在结构语义前提下满足 G_α、C^global ⟹ GlobalClassSpec 子集、∃! 保持。
  **不**声称任何 L1+ 经验有效性（实际声部激活/单位数由 Θ-参数化执行决定，属本文件有效域外）。

  ── 依赖方向（单向无环，owner = 本文件，不改 committed core）────────────────────
  GlobalProductClassification → Origin.{SourceAxioms, VoiceTree, CompleteClassification}。
  复用 SourceAxioms.Side（long/short）+ VoiceTree 父子结构 + CompleteClassification.GlobalVoice
  框架（ExistsUnique/GlobalClassSpec/globalClass/global_class_complete_unique）。
  **不**改 CompleteClassification.lean / Chain.lean（强化是新增 refine 定理，非改 core）。
  验证：`cd formal && lake env lean Origin/GlobalProductClassification.lean`。禁 sorry/admit/axiom。

  谱系：CompleteClassification §GlobalVoice（global_class_complete_unique，⋂_v + ∃!α）+
        VoiceTree（父子 alternating/cascade）+ Fugue（G_v 父子许可 Permit）→
        本文件（G_α 父子一致性补 C^global_α = G_α ∧ ⋂_v，PDF p24 缺口 2b）。
-/

import Origin.SourceAxioms
import Origin.VoiceTree
import CompleteClassification         -- Foundation lib 根（namespace NewChanlun）：GlobalVoice/GlobalClassSpec/globalClass/global_class_complete_unique

namespace NewChanlun.Origin.GlobalProductClassification

open NewChanlun                      -- GlobalClassSpec / globalClass / globalClass_spec / global_class_complete_unique（Foundation CompleteClassification 在 NewChanlun）
open NewChanlun.Origin.VoiceTree     -- flip / VoiceTree / IsAncestor（VoiceTree 在 NewChanlun.Origin.VoiceTree）

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 投影：从 local class 取声部三分量（方向 σ_v / 激活 a_v / 单位数）

  G_α 约束声部标注的三个分量。这些分量从 local class 投影（`dirOf`/`actOf`/`unitOf`）。
  投影是参数（每个具体分类系统给自己的 local class 编码 + 三投影），G_α 在投影上陈述。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部分量投影器 `VoiceProj`（L0）：从抽象 local class 取 G_α 所需三分量。
  - `dirOf  : LocalClass → Side`：声部方向 σ_v（long/short，缠论多空方向）。
  - `actOf  : LocalClass → Bool`：声部激活 a_v（是否有持仓 / 已开仓；true = 激活）。
  - `unitOf : LocalClass → Nat`：声部单位数（手数计数）。
  本结构纯投影承载——**不**约束三分量取值（取值由 local 分类语义决定），只声明它们可取。
-/
structure VoiceProj (LocalClass : Type) where
  dirOf  : LocalClass → Side
  actOf  : LocalClass → Bool
  unitOf : LocalClass → Nat

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 G_α 父子一致性谓词（PDF p24 三合取，仅约束直接父子）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★G1 赋格交替一致性（L0）：`parent v = some p → dirOf (α v) = flip (dirOf (α p))`。
  子声部方向 = 父声部方向翻转（σ_v = −σ_{p(v)}）——PDF p24「子声部方向 = −父」。
  用 VoiceTree.flip（Origin Side 上的对合，long↔short）。
-/
def GAlternating {V LocalClass : Type}
    (T : VoiceTree V) (proj : VoiceProj LocalClass) (α : V → LocalClass) : Prop :=
  ∀ v p, T.parent v = some p → proj.dirOf (α v) = VoiceTree.flip (proj.dirOf (α p))

/--
  ★G2 激活祖先闭合一致性（L0）：`parent v = some p → actOf (α v) = true → actOf (α p) = true`。
  子激活 ⟹ 父激活（a_v ≤ a_{p(v)} 的 Bool 形式）——PDF p24「子激活 ⟹ 父激活」。
  对应 Fugue.Permit 语义：子声部能开仓须父声部有持仓（q_p > 0）。
-/
def GActivationClosed {V LocalClass : Type}
    (T : VoiceTree V) (proj : VoiceProj LocalClass) (α : V → LocalClass) : Prop :=
  ∀ v p, T.parent v = some p → proj.actOf (α v) = true → proj.actOf (α p) = true

/--
  ★G3 同单位数一致性（L0）：`parent v = some p → actOf (α v) = true → unitOf (α v) = unitOf (α p)`。
  激活的子声部与父声部同单位数（同 unit 计数）——PDF p24「同单位数」。
  仅在子激活时约束（未激活子声部 unit 可为 0，不强制等于父）。
-/
def GSameUnit {V LocalClass : Type}
    (T : VoiceTree V) (proj : VoiceProj LocalClass) (α : V → LocalClass) : Prop :=
  ∀ v p, T.parent v = some p → proj.actOf (α v) = true → proj.unitOf (α v) = proj.unitOf (α p)

/--
  ★★父子一致性 G_α（L0，PDF p24 三合取）：G1 交替 ∧ G2 激活闭合 ∧ G3 同单位数。
  `Gconsistency T proj α` ⟺ α 标注满足全部三条父子约束（仅约束 VoiceTree.parent 给的直接父子）。
-/
def Gconsistency {V LocalClass : Type}
    (T : VoiceTree V) (proj : VoiceProj LocalClass) (α : V → LocalClass) : Prop :=
  GAlternating T proj α ∧ GActivationClosed T proj α ∧ GSameUnit T proj α

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 全局乘积分类 C^global_α = G_α ∧ ⋂_v C_{v,α_v}（PDF p24）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★全局乘积分类规格 `GlobalProductSpec`（L0，PDF p24）：
  `C^global_α(x) := G_α(α) ∧ (∀v, Local v x (α v))`。
  第一合取项 = 父子一致性 G_α；第二合取项 = ⋂_v C_{v,α_v}（= 现有 GlobalClassSpec）。

  ★相对 GlobalClassSpec（CompleteClassification.lean:173）的强化：加 `Gconsistency`——
  全局标注 α 不再是任意 local 标注的笛卡尔积，而须满足声部树父子一致性。
-/
def GlobalProductSpec {V X LocalClass : Type}
    (T : VoiceTree V) (proj : VoiceProj LocalClass)
    (Local : V → X → LocalClass → Prop)
    (x : X) (α : V → LocalClass) : Prop :=
  Gconsistency T proj α ∧ (∀ v : V, Local v x (α v))

/--
  ★C^global ⟹ GlobalClassSpec（L0，子集/强化方向）：满足全局乘积分类 ⟹ 满足现有 ⋂_v 规格。
  忠实编码「G_α ∧ ⋂_v」是「⋂_v」的**加强**（子集）——丢掉 G_α 合取项即得 GlobalClassSpec。
  这桥接本文件与 CompleteClassification.GlobalClassSpec（强化保持下游 ∃! 可引）。
-/
theorem product_refines_globalClassSpec {V X LocalClass : Type}
    (T : VoiceTree V) (proj : VoiceProj LocalClass)
    (Local : V → X → LocalClass → Prop)
    (x : X) (α : V → LocalClass)
    (h : GlobalProductSpec T proj Local x α) :
    GlobalClassSpec Local x α :=
  -- GlobalClassSpec Local x α = ∀ v, Local v x (α v) = h 的第二合取项。
  h.2

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 G_α 非平凡（真约束，缩小 α 空间——no-patch §5 反声明膨胀）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★G_α 非平凡见证（L0）：存在树 T、投影 proj、标注 α 使 `¬ Gconsistency T proj α`。
  即 G_α **不是恒真**——它真实地拒绝某些标注（缩小 α 空间）。这坐实 G_α 是约束而非装饰
  （对比 GlobalClassSpec 无此约束，任意 local 笛卡尔积通过）。

  构造：V = Bool，parent false = some true（false 是 true 的子），proj.dirOf = id（直取方向），
  α = id（标注即方向）。则子 (false) 方向 = short，父 (true) 方向 = long，
  flip (dirOf (α true)) = flip long = short = dirOf (α false)——G1 **满足**？需违反：
  改 α false ↦ 与父同向，则 dirOf (α false) = long ≠ flip long = short，G1 违反。
  这里用「子父同向」的 α 见证 G1（故 Gconsistency）失败。
-/
theorem Gconsistency_nontrivial :
    ∃ (T : VoiceTree Bool) (proj : VoiceProj Side) (α : Bool → Side),
      ¬ Gconsistency T proj α := by
  -- T：false 的父是 true（depth false = 1 > depth true = 0，满足 depth_decreasing）。
  -- T.side 取满足 VoiceTree 公理的值（子 short = flip(父 long)）——但 G_α 约束的是
  -- proj.dirOf (α ·)，**独立于 T.side**；故 proj.dirOf (α ·) 取「子父同向」违反 G1。
  let T : VoiceTree Bool := {
    parent := fun b => if b = false then some true else none
    side := fun b => if b = false then Side.short else Side.long
    closed := fun _ => false
    depth := fun b => if b = false then 1 else 0
    alternating := by
      intro v p hpar
      by_cases hv : v = false
      · subst hv; simp at hpar; subst hpar; rfl
      · simp [hv] at hpar
    depth_decreasing := by
      intro v p hpar
      by_cases hv : v = false
      · subst hv; simp at hpar; subst hpar; decide
      · simp [hv] at hpar
    cascade_close := by
      intro v p hpar hcl
      by_cases hv : v = false
      · subst hv; exact hcl
      · simp [hv] at hpar
  }
  -- proj.dirOf = id（标注即方向）；α = 子父都 long（同向）——违反 G1 交替。
  refine ⟨T, { dirOf := id, actOf := fun _ => true, unitOf := fun _ => 0 }, fun _ => Side.long, ?_⟩
  -- 证 ¬ Gconsistency：取 G1，对 v=false, p=true：dirOf(α false)=long，flip(dirOf(α true))=flip long=short。
  intro hG
  have hG1 := hG.1   -- GAlternating
  have hpar : T.parent false = some true := by simp [T]
  have hcontra := hG1 false true hpar
  -- hcontra : id (α false) = flip (id (α true))，即 long = flip long = short，矛盾。
  simp [VoiceTree.flip] at hcontra

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 ★★∃! 保持：结构语义前提下，唯一 α 自动满足 G_α

  核心定理。G_α 是真约束（§4），但**不破坏 ∃!**——因为当 local 分类的投影满足
  「结构语义前提」（H_dir/H_act/H_unit，由 VoiceTree 公理 + local 语义给定），
  逐声部唯一确定的 α = globalClass **自动满足** G_α（确定性传播，非候选筛选）。
  ════════════════════════════════════════════════════════════════════════ -/

section ExistenceUniqueness

variable {V X LocalClass : Type}
variable (T : VoiceTree V) (proj : VoiceProj LocalClass)
variable (Local : V → X → LocalClass → Prop)
-- 逐声部 local 唯一（= CompleteClassification.GlobalVoice 的 hLocal）。
variable (hLocal : ∀ (v : V) (x : X), ExistsUnique (fun c : LocalClass => Local v x c))
-- ★强制注入 hLocal：global_product_complete_unique 的 statement 不直接提及 hLocal
-- （它是 ∃! 的条件前提，由证明体消费），故须 include 使 section variable 注入签名。
include hLocal

/--
  ★结构语义前提 `StructuralSemantics`（L0）：local 分类的投影**在结构上**满足 G_α 的三分量约束。
  这是缠论结构（VoiceTree 父子 + Fugue Permit 语义）给定的前提——**不是**外加在 α 上的筛选，
  而是 local 分类自身的语义性质：对**任何**满足 local 规格的标注 c_v / c_p（父子对），
  - H_dir：子方向 = 父方向翻转（VoiceTree.alternating 在 dirOf∘local 上的投影）；
  - H_act：子激活 ⟹ 父激活（Fugue.Permit「子开仓须父有持仓」在 actOf∘local 上的投影）；
  - H_unit：激活子与父同单位数（赋格同 unit 在 unitOf∘local 上的投影）。

  ★存在论澄清（诚实分叉的关键）：StructuralSemantics 把「父子一致」定位为 **local 分类语义
  与树结构的联合性质**，不是 α 的额外自由度。给定该前提，唯一 α 满足 G_α（§5.2）；
  **缺**该前提（任意投影 + 任意 Local），G_α 才可能与逐声部唯一 α 冲突（§4 见证的非平凡支）。
  这把诚实分叉的「冲突支」显式编码为「前提的必要性」——前提在则无冲突，前提失则冲突可表达。
-/
structure StructuralSemantics (x : X) : Prop where
  /-- H_dir：父子对的任意合规 local 标注，方向投影满足赋格交替。 -/
  hdir : ∀ v p cv cp, T.parent v = some p →
    Local v x cv → Local p x cp → proj.dirOf cv = VoiceTree.flip (proj.dirOf cp)
  /-- H_act：父子对的任意合规 local 标注，子激活 ⟹ 父激活。 -/
  hact : ∀ v p cv cp, T.parent v = some p →
    Local v x cv → Local p x cp → proj.actOf cv = true → proj.actOf cp = true
  /-- H_unit：父子对的任意合规 local 标注，子激活 ⟹ 子父同单位数。 -/
  hunit : ∀ v p cv cp, T.parent v = some p →
    Local v x cv → Local p x cp → proj.actOf cv = true → proj.unitOf cv = proj.unitOf cp

/--
  ★★唯一 α 满足 G_α（L0，核心引理）：在结构语义前提下，`globalClass Local hLocal x`
  （逐声部唯一确定的标注）自动满足 `Gconsistency`。

  证明：G_α 三合取逐条由 StructuralSemantics 对应分量给——
  globalClass 的每声部值满足 local 规格（`globalClass_spec`），故对任意父子对 (v, p)，
  globalClass v 与 globalClass p 都是合规 local 标注，H_dir/H_act/H_unit 直接给三约束。
  **关键**：这不是「从候选中筛 α」，而是「对唯一确定的 α 验证它满足 G_α」——确定性传播。
-/
theorem globalClass_satisfies_G (x : X)
    (hSem : StructuralSemantics T proj Local x) :
    Gconsistency T proj (globalClass Local hLocal x) := by
  refine ⟨?_, ?_, ?_⟩
  · -- G1 交替
    intro v p hpar
    exact hSem.hdir v p _ _ hpar
      (globalClass_spec Local hLocal x v) (globalClass_spec Local hLocal x p)
  · -- G2 激活闭合
    intro v p hpar hact
    exact hSem.hact v p _ _ hpar
      (globalClass_spec Local hLocal x v) (globalClass_spec Local hLocal x p) hact
  · -- G3 同单位数
    intro v p hpar hact
    exact hSem.hunit v p _ _ hpar
      (globalClass_spec Local hLocal x v) (globalClass_spec Local hLocal x p) hact

/--
  ★★★全局乘积分类完整唯一（L0，PDF p24 缺口 2b 的兑现）★★★：
  `global_product_complete_unique` — 在逐声部 local 唯一（hLocal）+ 结构语义前提
  （StructuralSemantics）下，全局乘积分类 C^global_α 对每个 x **存在唯一** α：

    `∀ x, ∃! α, GlobalProductSpec T proj Local x α`（= G_α(α) ∧ ⋂_v Local v x (α v)）。

  ★证明骨架（∃! 三部分）：
  1. 存在性：witness = `globalClass Local hLocal x`（逐声部唯一确定的标注）。
     - 满足 G_α：`globalClass_satisfies_G`（§5.2，结构语义前提下确定性传播）。
     - 满足 ⋂_v：`globalClass_spec`（每声部满足 local 规格）。
  2. 唯一性：任意 α' 满足 GlobalProductSpec ⟹ α' 满足 GlobalClassSpec（丢 G_α 合取项，
     `product_refines_globalClassSpec`）⟹ α' = globalClass（CompleteClassification 的
     `global_class_complete_unique` 唯一性，逐声部 local 唯一蕴含全局标注唯一）。

  ★★诚实标注（formalization-validity-domain，关键，无声明膨胀）：
  - **条件式**：前件 = hLocal（逐声部 local 唯一）+ StructuralSemantics（结构语义前提）。
    StructuralSemantics 由 VoiceTree 公理 + local 分类语义给定（非外加 α 筛选）——见 §5.1 澄清。
  - **证什么**：在该前提下，G_α **不**破坏 ∃!——加 G_α 合取项后唯一 α 仍存在且唯一
    （G_α 由唯一 α 自动满足，唯一性由现有 global_class_complete_unique 保持）。
  - **不证什么**：(1) 无结构语义前提时 G_α 仍兼容 ∃!（§4 见证此时可冲突——前提是必要的）；
    (2) StructuralSemantics 对**所有**缠论 local 分类成立（那是各分类系统的语义义务，本文件
    把它作前提承载，不冒充已验证）；(3) 任何 L1+ 经验有效性（激活/单位数实时取值属 Θ 执行域）。
  - **不撞定义层**：G_α 与现有 ∃!α 不冲突——加 G_α 是**强化**（GlobalProductSpec ⊆ GlobalClassSpec）
    且在结构语义前提下 ∃! 保持。诚实分叉落在「不撞定义层」支。
-/
theorem global_product_complete_unique (x : X)
    (hSem : StructuralSemantics T proj Local x) :
    ExistsUnique (fun α : V → LocalClass => GlobalProductSpec T proj Local x α) := by
  refine ⟨globalClass Local hLocal x, ?_, ?_⟩
  · -- 存在性：globalClass 满足 G_α（§5.2）∧ ⋂_v（globalClass_spec）。
    refine ⟨globalClass_satisfies_G T proj Local hLocal x hSem, ?_⟩
    intro v
    exact globalClass_spec Local hLocal x v
  · -- 唯一性：任意 α' 满足 C^global ⟹ 满足 GlobalClassSpec ⟹ α' = globalClass。
    intro α' hα'
    have hGCS : GlobalClassSpec Local x α' :=
      product_refines_globalClassSpec T proj Local x α' hα'
    -- 现有 global_class_complete_unique：GlobalClassSpec 的唯一 g = globalClass，故 α' = globalClass。
    rcases global_class_complete_unique Local hLocal x with ⟨g, _hg, hUniqueG⟩
    have hα'Eq : α' = g := hUniqueG α' hGCS
    have hgcEq : globalClass Local hLocal x = g :=
      hUniqueG (globalClass Local hLocal x)
        (fun v => globalClass_spec Local hLocal x v)
    exact hα'Eq.trans hgcEq.symm

end ExistenceUniqueness

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★全局乘积分类标签 `GlobalProductTag`（gatekeeper，诚实分层）。
  - `ParentChildConsistencyL0`：G_α 三合取 + ∃! 保持是**结构事实**（L0 定义内蕴），非经验。
  - `ConditionalOnStructuralSemantics`：∃! 保持**条件依赖** StructuralSemantics 前提
    （前件必含结构语义条件——无该前提 G_α 可与 ∃! 冲突，§4 见证）。
  - `EmpiricalDomain`：激活/单位数实时取值 = L3，需真实数据，不由 L0 声称。
  ★**没有** `UnconditionalProductClassification` 构造子——类型层拒绝把 G_α∧∃! 标为无条件成立。
-/
inductive GlobalProductTag where
  | ParentChildConsistencyL0
  | ConditionalOnStructuralSemantics
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★全局乘积分类诚实标签包（L0 声明）。 -/
def globalProductLabels : List GlobalProductTag :=
  [GlobalProductTag.ParentChildConsistencyL0,
   GlobalProductTag.ConditionalOnStructuralSemantics,
   GlobalProductTag.EmpiricalDomain]

/-- ★禁标无条件成立（L0，gatekeeper 见证）：标签包必含 ConditionalOnStructuralSemantics
    （∃! 保持是条件式——前件含结构语义前提，不冒充无条件）。 -/
theorem globalProduct_is_conditional :
    GlobalProductTag.ConditionalOnStructuralSemantics ∈ globalProductLabels := by
  simp [globalProductLabels]

end NewChanlun.Origin.GlobalProductClassification
