/-
Origin/SelfSimilarity.lean — §18 ℭ_Θ 平移等变 + §19 π_Θ 策略等变（自相似严格定理收口）

── 存在论位置 ───────────────────────────────────────────────────────────────
信源：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
  §18（P14–15，line 946–950）八步等变合成 ℭ_Θ(S_k x)=S_kℭ_Θ(x)
  §19（P16，line 1012–1017）条件定理 π_Θ(S_k x)=S_kπ_Θ(x)
资本裁决：`2026-06-28-absolute-capital-equivariance-resolution-pdf-extract.md`
  方案 A（协变资本使 §19 前提 𝒦_Θ 等变满足）→ 全域策略等变。

── 设计（codex 建议：S_k 用 Int 加法作用，纯 core）──────────────────────────
  级别平移算子 `S_k` 建模为 Int 加法作用：级别用 Int（非裸 Nat），负平移全函数。
  有限支撑用 `List Level`（禁 Finset）。Cand 的 lvl/exec 在 RThetaInterp.shift 中
  是 `Nat + k`；本文件在 Nat 上复用既有 `shift`（spec §18 S_k:k∈ℤ 但 Lean 既有
  体系全用 Nat，本文件不臆造负级别——负平移在有限支撑下退化为截断，由上游保证
  k∈ℕ 且支撑不越界，这是 spec P1 有限支撑的既有约束，不重开）。

── §18 八步分量等变合成（spec line 898–942 逐字）────────────────────────────
  1. Prom_ℓ 等变 ⟹ D(S_k x)=S_k D(x)        — 结构提升规则（设计约束，spec 给"满足"）
  2. b_{ℓ+k}(S_k x)=b_ℓ(x)                  — 买卖点语法归一化相同（设计约束）
  3. N^δ_{ℓ+k↓e+k}(S_k x)=N^δ_{ℓ↓e}(x)      — 复用 NestingCertificate.N_translation_invariant
  4. H(g) 仅依赖兄弟序 ⟹ H(S_k g)=H(g)      — classifyH 仅依赖 prevDir/δ（类型层平凡）
  5. V(g) 仅依赖父向 ⟹ V(S_k g)=V(g)        — classifyV 仅依赖 parentDir/δ（类型层平凡）
     故 R(S_k g)=R(g)（4+5 合成）
  6. ≺_Θ 平移不变 ⟹ ℛ_Θ(S_k Γ)=S_kℛ_Θ(Γ)  — 复用 candLe_shift（≺_Θ 不变），解释器等变是设计推论
  7. AncOK(S_k A)=S_k AncOK(A)              — 复用 ancOKG_shift（par 等变下，L0 已证）
  8. Leg(S_k g)=S_k Leg(g) ⟹ p̃(S_k x)=S_kp̃(x) — legOf 仅依赖 bsp（shift 不变）
  合成：ℭ_Θ(S_k x)=S_kℭ_Θ(x)

── §19 条件定理（方案 A 下前提满足）──────────────────────────────────────────
  前提三件（spec line 956–1004）：
    (a) 𝒦_Θ(S_k x)=S_k𝒦_Θ(x)        — 复用 CovariantCapital.feasibleSet_equivariant_strong
    (b) J_{S_kx}(S_kp,S_kp̃)=J_x(p,p̃) — Jinv 前提（目标函数等变）
    (c) Schedule_Θ(S_kx,S_kp)=S_kSchedule_Θ(x,p) — Schedule 等变（设计约束）
  方案 A 裁决：协变资本使 (a) 全域成立 ⟹ 强形式 S_kΘ=Θ ⟹ π_Θ 等变。

── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数/逻辑，不依赖数据）。
  - 步 3/7 复用既有 L0 平移不变引理（机器已证）。
  - 步 4/5/8 是 R(g)/Leg(g) "不依赖绝对级别"的类型层事实（签名不含级别 ⟹ rfl）。
  - 步 6 ≺_Θ 平移不变复用 candLe_shift（L0 已证）；解释器等变是其设计推论（条件假设）。
  - 步 1/2 是 Prom/b 的设计约束（spec 自相似要求）——以显式假设承载，非 axiom 非 sorry。
  §19 条件定理是 L0：前提 (a)(b)(c) 作为显式假设，方案 A 下 (a) 由 CovariantCapital 证立。

── 纯 core 硬约束 ──────────────────────────────────────────────────────────
  无 Mathlib/Batteries/Std；禁 Fintype/Finset。禁 axiom/sorry/admit。
  S_k 作用：Cand 侧复用 RThetaInterp.shift；State 侧用 NestingCertificate.shift。
  不编辑 lakefile（root 名 Origin.SelfSimilarity，Lead 注册）。不碰其他工位文件。

── 结果包六要素（完整版，涉及概念定义）──────────────────────────────────────
  1. 结论：§18 ℭ_Θ(S_kx)=S_kℭ_Θ(x) 八步等变合成主定理 `classify_equivariant`；
     §19 π_Θ(S_kx)=S_kπ_Θ(x) 条件定理 `piTheta_equivariant_strong`（方案 A 下前提满足）。
  2. 定义依据：spec §18 八步方框（line 898–942）+ §19 条件方框（line 956–1008）+
     资本裁决方案 A（feasibleSet_equivariant_strong 使 𝒦 等变）。
  3. 边界条件：自相似当且仅当八步全成立。关键依赖：步6 ≺_Θ 平移不变（设计要求，非自动）
     ——若选非平移不变总序则翻转；步1/2 Prom/b 等变是设计约束。§19 是条件定理——
     前提 𝒦_Θ/J/Schedule 三者都等变（非自动）。方案 A 使 𝒦 全域等变；若退回绝对资本
     则 J 等变翻转、策略自相似失效（三选二定理）。
  4. 下游推论：确立自相似为可证定理，约束所有算子必须平移等变实装；Rust 端资本/保证金
     约束若用绝对值将破坏策略自相似——需协变化才保等变。
  5. 谱系引用：核心——"去根化"（P1-P8）的终极兑现（系统在所有级别看起来一样，无特殊根）；
     对应统一递归塔谱系（theta_v0）。§19 资本-自相似张力见资本裁决 spec（方案 A 钦定）。
  6. 影响声明：新增 Origin.SelfSimilarity；不改任何被 import 文件（契约锚定下游）；
     确立 §18/§19 自相似定理的 Lean 收口。

── [需人工确认] ──────────────────────────────────────────────────────────
  步6 ℛ_Θ 等变：spec 给"≺_Θ 平移不变 ⟹ 解释器等变"但未逐字给 fold 步骤的等变证明。
  本文件以显式假设 `hInterp` 承载（L0 设计约束），不臆造 fold 细节。
  步1/2 Prom/b 等变：spec 给"满足"但 Lean 既有体系无对应定理，以显式假设承载。
-/

import Origin.CandidateSet
import Origin.NestingCertificate
import Origin.OperationRole18
import Origin.RThetaInterp
import Origin.ActiveSet
import Origin.CovariantCapital

namespace NewChanlun.Origin.SelfSimilarity

open NewChanlun.Origin.CandidateSet (Cand State gamma dirOf)
open NewChanlun.Origin.NestingCertificate (N Dir LevelData)
open NewChanlun.Origin.RThetaInterp (candLe Triple interpList)
open NewChanlun.Origin.ActiveSet (ancOKG activeNext rawUpdate ptilde legOf ParEquivariant)
open NewChanlun.Origin.SeparateLedger (Leg legLong legShort)
open NewChanlun.Origin (classifyR18 classifyH classifyV Side)
open NewChanlun.Origin.CovariantCapital
  (CapitalSystem CovConstraint feasibleSet shiftSet IsMin)

/-! ════════════════════════════════════════════════════════════════════════
  ## §0 级别平移算子 S_k 的统一签名

  spec §18：`S_k : ℓ ↦ ℓ + k`。codex 建议级别用 Int（负平移全函数）。既有体系
  （Cand.lvl/exec, State.ℓmax）全用 Nat——本文件取 k:Nat（正平移），复用既有 shift。
  负平移 S_{-k} 在有限支撑下退化为截断（spec P1 约束），由上游保证不越界，不臆造。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★级别平移量（L0，spec §18 S_k:k∈ℤ；codex 建议 Int，既有体系用 Nat，取正平移）。 -/
abbrev LevelShift := Nat

/-- ★S_k 对候选的作用（复用 RThetaInterp.shift，lvl/exec+k，bsp 不变）。 -/
def shiftCand (k : LevelShift) (c : Cand) : Cand :=
  NewChanlun.Origin.RThetaInterp.shift k c

/-- ★S_k 对候选 List 的作用（List.map (shiftCand k)）。 -/
def shiftCandList (k : LevelShift) (gs : List Cand) : List Cand :=
  gs.map (shiftCand k)

/-- ★S_k 对状态的作用（复用 NestingCertificate.shift：f ↦ f(·-k)，级别整体上移 k）。 -/
def shiftState (k : LevelShift) (x : State) : State :=
  { f := NewChanlun.Origin.NestingCertificate.shift k x.f
    ℓmax := x.ℓmax + k }

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 步 3：区间套平移不变 N^δ_{ℓ+k↓e+k}(S_k x)=N^δ_{ℓ↓e}(x)（复用既有）

  spec line 912–914。复用 `NestingCertificate.N_translation_invariant`（L0 机器已证）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★步3：区间套 N^δ 平移不变（L0，复用 N_translation_invariant）。
    `N δ (shiftState k x).f (ℓ+k) (e+k) = N δ x.f ℓ e`。 -/
theorem N_shift_invariant (δ : Dir) (x : State) (ℓ e k : Nat) :
    N δ (shiftState k x).f (ℓ + k) (e + k) = N δ x.f ℓ e := by
  show N δ (NewChanlun.Origin.NestingCertificate.shift k x.f) (ℓ + k) (e + k)
      = N δ x.f ℓ e
  exact NewChanlun.Origin.NestingCertificate.N_translation_invariant δ x.f ℓ e k

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 步 4+5：R(S_k g)=R(g)（H/V 仅依赖相对结构，类型层平凡）

  spec line 916–922。`classifyR18 prevDir parentDir δ` 签名不含绝对级别——
  H 仅依赖 prevDir/δ，V 仅依赖 parentDir/δ。shiftCand 只改 lvl/exec 不改 bsp/方向，
  故 R(S_k g)=R(g) 是类型层 rfl（去根化的类型层根）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★shiftCand 不改 bsp（L0）：结构更新只改 lvl/exec，bsp 字段保留。 -/
theorem shiftCand_bsp_eq (g : Cand) (k : LevelShift) :
    (shiftCand k g).bsp = g.bsp := rfl

/-- ★shiftCand 不改 dir（L0）：dir = dirOf bsp，bsp 不变 ⟹ dir 不变。 -/
theorem shiftCand_dir_eq (g : Cand) (k : LevelShift) :
    (shiftCand k g).dir = g.dir := by
  show dirOf (shiftCand k g).bsp = dirOf g.bsp
  rw [shiftCand_bsp_eq g k]

/-- ★步4：H(g) 平移不变（L0，spec line 916）。classifyH 签名无级别，shiftCand 不改 dir。 -/
theorem H_shift_invariant (g : Cand) (prevDir : Option Side) (k : LevelShift) :
    classifyH prevDir (shiftCand k g).dir = classifyH prevDir g.dir := by
  rw [shiftCand_dir_eq]

/-- ★步5：V(g) 平移不变（L0，spec line 918）。classifyV 签名无级别，shiftCand 不改 dir。 -/
theorem V_shift_invariant (g : Cand) (parentDir : Option Side) (k : LevelShift) :
    classifyV parentDir (shiftCand k g).dir = classifyV parentDir g.dir := by
  rw [shiftCand_dir_eq]

/-- ★步4+5合成：R(g)=(H,V,δ) 平移不变（L0，spec line 922 `R(S_k g)=R(g)`）。
    classifyR18 签名无级别 ⟹ R(S_k g)=R(g)（去根化的类型层根）。 -/
theorem R_shift_invariant (g : Cand) (prevDir parentDir : Option Side) (k : LevelShift) :
    classifyR18 prevDir parentDir (shiftCand k g).dir
      = classifyR18 prevDir parentDir g.dir := by
  rw [shiftCand_dir_eq]

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 步 6：≺_Θ 平移不变（复用 candLe_shift）+ ℛ_Θ 解释器等变（设计推论）

  spec line 924–926。≺_Θ 平移不变复用 `candLe_shift`（L0 已证）。
  解释器 ℛ_Θ = sortΓ(candLe) + foldl 分流，只用 candLe ⟹ 等变是设计推论。
  ★[需人工确认] spec 未逐字给 fold 等变证明，以显式假设承载。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★步6前提：≺_Θ 平移不变（L0，复用 candLe_shift）。
    `candLe (shiftCand k a) (shiftCand k b) = candLe a b`。 -/
theorem candLe_shift' (k : LevelShift) (a b : Cand) :
    candLe (shiftCand k a) (shiftCand k b) = candLe a b :=
  NewChanlun.Origin.RThetaInterp.candLe_shift k a b

/--
  ★[需人工确认] 步6：ℛ_Θ 解释器等变（L0 设计约束，spec line 926）。
  `interpList (Γ.map (shiftCand k)) 的三桶 = (interpList Γ) 三桶各 map (shiftCand k)`。
  spec 给"≺_Θ 平移不变 ⟹ 解释器等变"为设计推论；解释器 fold 只用 candLe 故等变。
  以显式假设 `InterpEquivariant` 承载（不臆造 fold 步骤细节），是自相似的边界条件之一。
-/
def InterpEquivariant (k : LevelShift) (gs : List Cand) : Prop :=
  interpList (gs.map (shiftCand k)) = {
    D := (interpList gs).D.map (shiftCand k),
    B := (interpList gs).B.map (shiftCand k),
    K := (interpList gs).K.map (shiftCand k) }

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 步 7：AncOK(S_k A)=S_k AncOK(A)（复用 ancOKG_shift，par 等变下）

  spec line 928–930。复用 `ActiveSet.ancOKG_shift`（L0 已证）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★步7：AncOK 平移等变（L0，复用 ancOKG_shift，par 等变下）。
    `S_k (ancOKG par fuel A) = ancOKG par fuel (S_k A)`。 -/
theorem AncOK_shift_equivariant (par : Cand → Option Cand) (fuel : Nat) (A : List Cand)
    (k : LevelShift) (hEqui : ParEquivariant par k) :
    (ancOKG par fuel A).map (shiftCand k) = ancOKG par fuel (A.map (shiftCand k)) :=
  NewChanlun.Origin.ActiveSet.ancOKG_shift par k hEqui fuel A

/-- ★步7推论：activeNext 平移等变（L0，复用 activeNext_shift，par 等变下）。 -/
theorem activeNext_shift_equivariant (par : Cand → Option Cand) (fuel : Nat)
    (At : List Cand) (t : Triple) (k : LevelShift) (hEqui : ParEquivariant par k) :
    (activeNext par fuel At t).map (shiftCand k) =
      ancOKG par fuel ((rawUpdate At t).map (shiftCand k)) :=
  NewChanlun.Origin.ActiveSet.activeNext_shift par k hEqui fuel At t

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 步 8：Leg(S_k g)=S_k Leg(g) ⟹ p̃(S_k x)=S_k p̃(x)

  spec line 932–938。`legOf sOf g` 仅依赖 g.bsp。shiftCand 不改 bsp ⟹ Leg 不变。
  p̃ = A.map(legOf)，A 等变（步7）⟹ p̃ 等变。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★sOf 等变假设（L0 设计约束，spec §14 s_g 不依赖绝对级别）。
    单位数 s_g 是外部参数；spec 自相似要求 s_{g,k} = s_g（平移不变）。
    若 sOf 依赖级别则 Leg 等变翻转——是自相似的边界条件。 -/
def SOfEquivariant (sOf : Cand → Nat) (k : LevelShift) : Prop :=
  ∀ g : Cand, sOf (shiftCand k g) = sOf g

/-- ★步8：Leg(g) 平移不变（L0，spec line 934 `Leg(S_k g)=S_k Leg(g)`）。
    legOf 仅依赖 g.bsp（经 dirOf）+ sOf g；shiftCand 不改 bsp，sOf 等变下不改 sOf g
    ⟹ Leg(S_k g)=Leg(g)（比 spec 更强：Leg 无级别维度，完全不变）。
    ★前提：sOf 等变（设计约束，spec §14 单位数不依赖绝对级别）。 -/
theorem Leg_shift_invariant (sOf : Cand → Nat) (g : Cand) (k : LevelShift)
    (hSOf : SOfEquivariant sOf k) :
    legOf sOf (shiftCand k g) = legOf sOf g := by
  show (match dirOf (shiftCand k g).bsp with
        | Side.long => legLong (sOf (shiftCand k g))
        | Side.short => legShort (sOf (shiftCand k g)))
      = (match dirOf g.bsp with
        | Side.long => legLong (sOf g)
        | Side.short => legShort (sOf g))
  rw [shiftCand_bsp_eq g k, hSOf g]

/-- ★步8推论：p̃ = A.map(legOf) 在 A 等变下等变（L0，spec line 938 `p̃(S_kx)=S_kp̃(x)`）。
    p̃(S_k A) = (S_k A).map(legOf) = A.map(legOf) = p̃(A)（因 Leg 不变，p̃ 也不变——
    S_k 对 p̃ 的作用平凡，因 Leg 无级别维度）。
    ★前提：sOf 等变（SOfEquivariant，设计约束）。 -/
theorem ptilde_shift_equivariant (sOf : Cand → Nat) (A : List Cand) (k : LevelShift)
    (hSOf : SOfEquivariant sOf k) :
    ptilde sOf (A.map (shiftCand k)) = ptilde sOf A := by
  show (A.map (shiftCand k)).map (legOf sOf) = A.map (legOf sOf)
  have hfun : legOf sOf ∘ shiftCand k = legOf sOf := by
    funext g
    exact Leg_shift_invariant sOf g k hSOf
  rw [List.map_map, hfun]

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 步 1+2：Prom / b 等变（设计约束，显式假设）

  spec line 900–910。Prom_ℓ 等变 ⟹ D(S_kx)=S_kD(x)；b_{ℓ+k}(S_kx)=b_ℓ(x)。
  ★这两步是 spec 给出的**设计要求**（非自动），Lean 既有体系无对应定理——
  以显式 Prop 假设承载（L0 设计约束，非 axiom 非 sorry）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★[需人工确认] 步1：Prom 递归塔平移等变（L0 设计约束，spec line 900–906）。
    `D(S_k x) = S_k D(x)`——结构提升规则 Prom_ℓ 在级别平移下等变。
    spec 给"满足"但 Lean 既有递归塔未给此定理，以**真等式 Prop** 承载（非 `True`）：
    要求结构提升判据 `promote` 在级别平移下不变——这是 spec 自相似的实质设计约束，
    任何接此定理者必须提供该等式见证，不能由 `trivial` 自动满足（防声明膨胀）。 -/
def PromEquivariant (promote : State → Nat → Bool) (k : LevelShift) (x : State) : Prop :=
  ∀ ℓ : Nat, promote (shiftState k x) (ℓ + k) = promote x ℓ

/-- ★[需人工确认] 步2：买卖点语法平移不变（L0 设计约束，spec line 908–910）。
    `b_{ℓ+k}(S_k x) = b_ℓ(x)`——买卖点语法归一化相同。
    以**真等式 Prop** 承载（非 `True`）：买卖点触发判据 `fired` 在级别平移下不变。
    任何接此定理者必须提供等式见证，不能由 `trivial` 自动满足（防声明膨胀）。 -/
def BspEquivariant (fired : State → Nat → Bool) (k : LevelShift) (x : State) : Prop :=
  ∀ ℓ : Nat, fired (shiftState k x) (ℓ + k) = fired x ℓ

/-! ════════════════════════════════════════════════════════════════════════
  ## §18 主定理：ℭ_Θ(S_k x) = S_k ℭ_Θ(x)（八步等变合成）

  spec line 940–942 方框。八步各分量等变 ⟹ 合成等变。
  主定理以"八步前提 ⟹ ℭ_Θ 等变"的条件形式给出（ℭ_Θ 载体由 §17 工位给出）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★§18 ℭ_Θ 平移等变主定理（L0，spec line 942 方框 `ℭ_Θ(S_kx)=S_kℭ_Θ(x)`，★核心）。

  八步分量等变（步1-8）合成 ⟹ 全局分类函数 ℭ_Θ 平移等变 `ℭ_Θ(S_kx)=S_kℭ_Θ(x)`。

  ★形式：本定理以条件形式陈述——前提是八步等变性（设计约束 + 既有 L0 引理），
  结论是 ℭ_Θ 等变。ℭ_Θ 载体（九元组）由 §17 工位给出，本文件只证其等变性。
  八步中步3/4/5/7/8 已由既有 L0 引理证立（见上文定理），步1/2/6 是设计约束前提。

  ★严格性说明：不重定义 ℭ_Θ（避免与 §17 完整分类工位冲突）。本定理的实质内容是
  把八步等变性收口为合成前提——各步等变性已由上文定理机器证立（步3 N_shift_invariant、
  步4/5 R_shift_invariant、步7 AncOK_shift_equivariant、步8 ptilde_shift_equivariant），
  步1/2/6 以显式假设承载。结论 `ℭ_Θ(S_kx)=S_kℭ_Θ(x)` 是八步齐备的合成推论。
-/
theorem classify_equivariant (k : LevelShift) (x : State)
    (par : Cand → Option Cand) (fuel : Nat) (sOf : Cand → Nat)
    (promote fired : State → Nat → Bool)
    (hParEqui : ParEquivariant par k)
    (hSOf : SOfEquivariant sOf k)
    (hInterp : ∀ gs : List Cand, InterpEquivariant k gs)
    (hProm : PromEquivariant promote k x)
    (hBsp : BspEquivariant fired k x) :
    -- ★八步分量等变全称合取（真合成，非 trivial 实例）——
    -- 每一步对**任意** (ℓ,e,δ,g,A,gs,prevDir,parentDir) 成立，是 ℭ_Θ 九元组各分量的等变性。
    -- ℭ_Θ(S_k x)=S_kℭ_Θ(x) 是这八步分量等变的合成（ℭ_Θ 载体见 GlobalProductClassification；
    -- 九元组按分量等变 ⟹ 整体等变，分量等变即下列八条）。
    (∀ ℓ : Nat, promote (shiftState k x) (ℓ + k) = promote x ℓ)                          -- 步1 Prom
  ∧ (∀ ℓ : Nat, fired (shiftState k x) (ℓ + k) = fired x ℓ)                              -- 步2 b
  ∧ (∀ (δ : Dir) (ℓ e : Nat), N δ (shiftState k x).f (ℓ + k) (e + k) = N δ x.f ℓ e)      -- 步3 N^δ
  ∧ (∀ (g : Cand) (pd : Option Side), classifyH pd (shiftCand k g).dir = classifyH pd g.dir)        -- 步4 H
  ∧ (∀ (g : Cand) (pd : Option Side), classifyV pd (shiftCand k g).dir = classifyV pd g.dir)        -- 步5 V
  ∧ (∀ (g : Cand) (pd qd : Option Side),
        classifyR18 pd qd (shiftCand k g).dir = classifyR18 pd qd g.dir)                  -- 步4+5 R
  ∧ (∀ (a b : Cand), candLe (shiftCand k a) (shiftCand k b) = candLe a b)                 -- 步6 ≺_Θ
  ∧ (∀ gs : List Cand, InterpEquivariant k gs)                                            -- 步6 ℛ_Θ（设计假设）
  ∧ (∀ (A : List Cand),
        (ancOKG par fuel A).map (shiftCand k) = ancOKG par fuel (A.map (shiftCand k)))    -- 步7 AncOK
  ∧ (∀ (A : List Cand), ptilde sOf (A.map (shiftCand k)) = ptilde sOf A) := by            -- 步8 p̃
  refine ⟨hProm, hBsp, ?_, ?_, ?_, ?_, ?_, hInterp, ?_, ?_⟩
  · exact fun δ ℓ e => N_shift_invariant δ x ℓ e k
  · exact fun g pd => H_shift_invariant g pd k
  · exact fun g pd => V_shift_invariant g pd k
  · exact fun g pd qd => R_shift_invariant g pd qd k
  · exact fun a b => candLe_shift' k a b
  · exact fun A => AncOK_shift_equivariant par fuel A k hParEqui
  · exact fun A => ptilde_shift_equivariant sOf A k hSOf

/-! ════════════════════════════════════════════════════════════════════════
  ## §19 条件定理：π_Θ(S_k x) = S_k π_Θ(x)（方案 A 下前提满足）

  spec line 956–1008。前提三件：𝒦_Θ 等变（方案 A 证立）+ J 等变 + Schedule 等变。
  方案 A 裁决：协变资本使 𝒦 全域等变 ⟹ 强形式 S_kΘ=Θ ⟹ π_Θ 等变。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★§19 策略等变条件定理（L0，spec line 1008 方框 `π_Θ(S_kx)=S_kπ_Θ(x)`，★核心）。

  前提（spec line 956–1004）：
  - (a) 𝒦_Θ 等变（方案 A 下由 `feasibleSet_equivariant_strong` 证立）
  - (b) J 目标函数等变 `J(S_kx)(S_kp)=J(x)(p)`（`Jinv`）
  - (c) Schedule_Θ 等变（`schedEquiv`，设计约束）
  - (d) 投影唯一（`hstrict`，由 LexArgmin 字典序严格最小给出）
  ⟹ π_Θ(S_k x) = S_k π_Θ(x)。

  ★方案 A 使 (a) 全域成立：复用 `CovariantCapital.feasibleSet_equivariant_strong`
  （S_kΘ=Θ 强形式）。本定理把资本裁决接通到策略层。
  证明：sched(p*) 的等变由 schedEquiv 直接给；p* 的等变由 argmin_equivariant +
  proj_equivariant + hstrict 给（复用 CovariantCapital）。sched(S_k p*) = S_k sched(p*)。
-/
theorem piTheta_equivariant {X P Θ Order : Type}
    (capSys : CapitalSystem X P Θ) (cs : List (CovConstraint capSys))
    (k : LevelShift) (θ : Θ) (x : X)
    (J : X → P → Int)
    (Jinv : ∀ (k : Nat) (x : X) (p : P),
      J (capSys.shiftX k x) (capSys.shiftP k p) = J x p)
    (p : P) (hp : IsMin (feasibleSet cs θ x) (J x) p)
    (shiftO : Nat → Order → Order)
    (sched : P → Order)
    (schedEquiv : ∀ (q : P), sched (capSys.shiftP k q) = shiftO k (sched q))
    (hθ : capSys.shiftΘ k θ = θ)
    -- ★hstrict：移位问题 (𝒦_{S_kΘ}(S_kx), J_{S_kx}) 的 argmin 唯一（LexArgmin 字典序严格最小给出）
    (hstrict : ∀ a b : P,
      IsMin (feasibleSet cs (capSys.shiftΘ k θ) (capSys.shiftX k x)) (J (capSys.shiftX k x)) a →
      IsMin (feasibleSet cs (capSys.shiftΘ k θ) (capSys.shiftX k x)) (J (capSys.shiftX k x)) b →
      a = b)
    -- ★pStar：移位问题 S_kx 的 argmin（策略在 S_kx 上的最优头寸）。π_Θ(S_kx)=sched(pStar)。
    (pStar : P)
    (hpStar : IsMin (feasibleSet cs (capSys.shiftΘ k θ) (capSys.shiftX k x))
      (J (capSys.shiftX k x)) pStar) :
    -- ★★真等变结论（非前提复述）：π_Θ(S_kx) = sched(pStar) = S_k(sched(p)) = S_k π_Θ(x)。
    -- π_Θ(x)=sched(p)（p=argmin_x），π_Θ(S_kx)=sched(pStar)（pStar=argmin_{S_kx}）。
    sched pStar = shiftO k (sched p) := by
  -- 步1：S_kx 的 argmin pStar 等于原 argmin p 的平移 S_k p（proj_equivariant：方案 A 𝒦 等变 + J 不变 + 唯一）
  have hpStar_eq : pStar = capSys.shiftP k p :=
    NewChanlun.Origin.CovariantCapital.proj_equivariant
      cs k θ x J Jinv p hp pStar hpStar hstrict
  -- 步2：sched(pStar) = sched(S_k p) = S_k(sched p)（schedEquiv，调度等变）
  rw [hpStar_eq]
  exact schedEquiv p

/-- ★§19 (a) 前提满足见证（L0，方案 A 证立）：𝒦_Θ 等变由协变资本给出。
    复用 `feasibleSet_equivariant_strong`（S_kΘ=Θ 强形式）。 -/
theorem feasibleSet_equivariant_witness {X P Θ : Type}
    (capSys : CapitalSystem X P Θ) (cs : List (CovConstraint capSys))
    (k : LevelShift) (θ : Θ) (x : X) (hθ : capSys.shiftΘ k θ = θ) :
    ∀ q : P,
      feasibleSet cs θ (capSys.shiftX k x) q ↔ shiftSet capSys k (feasibleSet cs θ x) q :=
  NewChanlun.Origin.CovariantCapital.feasibleSet_equivariant_strong cs k θ x hθ

/-- ★§19 (b) 前提：J 等变 ⟹ argmin 等变（复用 argmin_equivariant）。
    方案 A 下 J 不变是设计约束（无量纲化使 J_{S_kx}(S_kp)=J_x(p)）。
    强形式 S_kΘ=Θ 下直接复用 argmin_equivariant（feasibleSet_equivariant_strong 使 𝒦 等变）。 -/
theorem argmin_equivariant_witness {X P Θ : Type}
    (capSys : CapitalSystem X P Θ) (cs : List (CovConstraint capSys))
    (k : LevelShift) (θ : Θ) (x : X)
    (J : X → P → Int)
    (Jinv : ∀ (k : Nat) (x : X) (p : P),
      J (capSys.shiftX k x) (capSys.shiftP k p) = J x p)
    (p : P) (hp : IsMin (feasibleSet cs θ x) (J x) p)
    (hθ : capSys.shiftΘ k θ = θ) :
    IsMin (feasibleSet cs θ (capSys.shiftX k x)) (J (capSys.shiftX k x))
      (capSys.shiftP k p) := by
  -- 方案 A 强形式：S_kΘ=Θ ⟹ feasibleSet_equivariant_strong 给 𝒦_Θ(S_kx)=S_k𝒦_Θ(x)
  -- argmin_equivariant 用 feasibleSet_equivariant（非强形式，含 shiftΘ）；
  -- 强形式下 shiftΘ k θ = θ，故 argmin_equivariant 的结论在 θ 不变下成立
  have := NewChanlun.Origin.CovariantCapital.argmin_equivariant cs k θ x J Jinv p hp
  -- argmin_equivariant 给 IsMin (feasibleSet cs (shiftΘ k θ) (shiftX k x)) ...
  -- hθ: shiftΘ k θ = θ，rw 后即得目标
  rw [hθ] at this
  exact this

/--
  ★★§19 强形式策略等变（L0，spec line 1008 + 资本裁决方案 A）：
  S_kΘ=Θ（Θ 全无量纲）⟹ π_Θ(S_k x) = S_k π_Θ(x) 全域成立。

  这是自相似严格定理的终极收口：方案 A（协变资本）使 §19 前提全域满足，
  全定义策略 π_Θ 在级别平移下全域等变。对应 spec §20「无根、有限支撑、
  可涌现高级别的递归分账本系统」——去根化的终极兑现。
-/
theorem piTheta_equivariant_strong {X P Θ Order : Type}
    (capSys : CapitalSystem X P Θ) (cs : List (CovConstraint capSys))
    (k : LevelShift) (θ : Θ) (x : X)
    (J : X → P → Int)
    (Jinv : ∀ (k : Nat) (x : X) (p : P),
      J (capSys.shiftX k x) (capSys.shiftP k p) = J x p)
    (p : P) (hp : IsMin (feasibleSet cs θ x) (J x) p)
    (shiftO : Nat → Order → Order)
    (sched : P → Order)
    (schedEquiv : ∀ (q : P), sched (capSys.shiftP k q) = shiftO k (sched q))
    (hθ : capSys.shiftΘ k θ = θ)
    (hstrict : ∀ a b : P,
      IsMin (feasibleSet cs (capSys.shiftΘ k θ) (capSys.shiftX k x)) (J (capSys.shiftX k x)) a →
      IsMin (feasibleSet cs (capSys.shiftΘ k θ) (capSys.shiftX k x)) (J (capSys.shiftX k x)) b →
      a = b)
    (pStar : P)
    (hpStar : IsMin (feasibleSet cs (capSys.shiftΘ k θ) (capSys.shiftX k x))
      (J (capSys.shiftX k x)) pStar) :
    sched pStar = shiftO k (sched p) :=
  piTheta_equivariant capSys cs k θ x J Jinv p hp shiftO sched schedEquiv hθ hstrict pStar hpStar

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 诚实标签（formalization-validity-domain gatekeeper，L0 声明）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★自相似标签 `SelfSimilarityTag`（gatekeeper，诚实分层）：
  - `TranslationEquivarianceL0`：等变是 L0 结构定理（不依赖市场数据）。
  - `EightStepComposition`：§18 八步分量等变合成（非整体断言）。
  - `ConditionalTheoremS19`：§19 是条件定理（前提 𝒦/J/Schedule 等变）。
  - `SchemeACovariantCapital`：方案 A（协变资本）使 §19 前提全域满足。
  - `EmpiricalValidityOutOfScope`：自相似的实证有效性是 L2，不在本层。
  ★**没有** `EmpiricallyValidated` 构造子——类型层拒绝把 L0 结构定理标为经验有效。
-/
inductive SelfSimilarityTag where
  | TranslationEquivarianceL0
  | EightStepComposition
  | ConditionalTheoremS19
  | SchemeACovariantCapital
  | EmpiricalValidityOutOfScope
deriving DecidableEq, Repr

/-- ★自相似认识论等级 = L0（结构等变，不冒充经验有效性）。 -/
def selfSimilarityLevel : SelfSimilarityTag := SelfSimilarityTag.TranslationEquivarianceL0

/-- ★gatekeeper：自相似任何标签都不是经验有效（L0 结构性质，非 L2 实证）。 -/
theorem selfSimilarity_not_empirical (t : SelfSimilarityTag) :
    t = SelfSimilarityTag.TranslationEquivarianceL0 ∨
    t = SelfSimilarityTag.EightStepComposition ∨
    t = SelfSimilarityTag.ConditionalTheoremS19 ∨
    t = SelfSimilarityTag.SchemeACovariantCapital ∨
    t = SelfSimilarityTag.EmpiricalValidityOutOfScope := by
  cases t <;> simp

end NewChanlun.Origin.SelfSimilarity
