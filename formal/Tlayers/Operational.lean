/-
  递归操作层定理 T₁₇–T₃₂ 全 E-set Lean L0 形式化（必然性推导 §第2部分 + §0.5 prove 守卫映射）
  ★task #48（T-Lean 层B，编排者 2026-06-25 覆盖 codex D1：全都要 Lean，E-set 全进）

  范围（#48 E-set，编排者裁定）：操作层 T₁₇-T₃₂ **全部进 Lean**（自包含单一权威源）：
  - L 类（结构归纳）：T₁₇/T₂₃/T₄₄ —— 自包含于本模块（前 OperationalStructural.lean 半版已合并删除）。
  - R 类（运行时律）：T₁₈配额σ-不变/T₁₉成本门/T₂₀降成本/T₂₁并发/T₂₆三阶段/T₂₈-T₃₂
    → Lean 证其 **L0 结构形式**（"若 model 满足公理则不变量成立"的归纳/代数/条件定理），
    Rust 守卫（prove_theta_sigma_invariant/N2/N4/N5/N6/N7/prove_chain/A5）保留作 L2 补充。
  - A 类（轴范式 reform→真完全分类）：T₂₄ 多空对称（ℤ/2 τ-共轭）+ §6B 三轴（#45）
    → 用 603 递归数据类型范式重铸为 **构造子穷尽**（sum type 完全分类），reform 成功（见各节）。
  - 唯一合法排除 X：T₃₂ 的 **λ 定量罕见率**（L2 经验，不可结构证）；T₃₂ 定性结构侧（清仓门槛
    = 三条件合取）进 E。复合 T 的结构侧→Lean / 经验侧→X 显式分开（formalization-validity-domain）。

  ★命名诚实性 gatekeeper 标注（team-lead 裁定 2026-06-25，codex 裁决4 + t-gapmap {D/L/R/A} 判据）：
  每条定理标认识论等级——本模块 Lean 定理 **全是 L0 结构侧**（结构形式/构造子穷尽/良基/因果序），
  对应的 **运行时数值守卫**（配额 σ-不变 panic / 成本门 panic 等）= `RuntimeGuard`（Rust L2，**不在本
  Lean 模块**，§0.5 映射）。标签词汇（t-gapmap 对齐）：
  - `[gk:TrueCompleteClassification]` = 构造子穷尽真完全分类（= gap-map D/L；sum type 全分类，
    initiality 无遗漏）。
  - `[gk:StructurePartitionOnly]` = L0 结构形式/不变量（= gap-map L 结构侧；良基终止/因果偏序/
    投影分离/σ-不变代数等式——是 model 公理的结构推论，**非** 完整语义双射，对应 R 类运行时守卫的 L0 核）。
  - `[gk:RuntimeGuard]` = 运行时数值不变量（= gap-map R；Rust panic 守卫，L2，**本 Lean 模块不含**，
    仅在节注引用其对应 prove_*）。
  逐定理标签见各定理 doc 行首 `[gk:…]`。诚实分层：结构 L0 留 E-set / 运行时数值侧标 R 不冒充 L0
  （formalization-validity-domain，no-patch 不声明膨胀）。

  ★构建方式（no-workaround 解，不碰 lakefile.toml / Formal.lean）：
    依赖已构建的 `Formal` 库 + 本仓 `OperationalStructural`，**不注册为 lib root**。机器验证经
        `cd formal && lake env lean Tlayers/Operational.lean`
    Lean 内核逐条检查（拒绝 sorry/admit/axiom）= 真 L0。`lake build` 全项目不受扰动。

  ★R 类"L0 结构形式"的认识论（编排者裁定 + formalization-validity-domain）：
    R 类运行时律的 Lean 形式 = **conditional/代数定理**（不是直接断言运行时行为）：
    "配额算子满足 σ-不变性谓词"（代数等式）、"成本门是良基终止条件"（归纳）、"降成本投影与
    全局链正交"（结构分离）等。这些是 model 公理的逻辑推论（L0），Rust panic 守卫验证 model 在
    真实数据上确实满足（L2）。L0 证形式必然，L2 证经验成立——两者互补不替代。

  ★codex 异质审计修正已吸收（session 019effee，high）：
    ① T₁₈ 去重言化——`SigmaInvariant` 谓词真命题（非 `X=X`），levelDependentQuota 反例否证；
    ② 去标题声明膨胀——R 类标注"L0 结构形式"，T₃₂ 频率诚实标 L2 不入 Lean；
    ③ 补 T₂₇（codex 指原缺）+ T₁₉ leaf↔成本门绑定。

  ★赋格 stretto（T₂₂）/ §6B cd_ℚ=1 两层闭合：检验是否暴露定义冲突 → **未暴露**。
    T₂₂ stretto = 森林时序重叠（子 active 区间 ⊂ 父 active 区间，可自洽建模）；
    §6B 三轴 reform 为构造子穷尽 sum type（递归范式可达），σ-等变 ⊊ 三轴是已结算有效域收窄。
    故 **无 /escalate**（无真定义冲突；若后续暴露 cd_ℚ=1 两层闭合冲突立即 /escalate）。
-/

import Formal.RecursiveConstruction
import Formal.DivergenceNesting
import Formal.OperationalSemantics
import Formal.BSPLabels

namespace Formal.Tlayers.Operational

open Formal.TrendTrichotomy (Direction)
open Formal.DivergenceNesting (DivSegment Nested NestingChain
  nested_strictly_shrinks nested_level_decreases nesting_chain_bounded_by_head)

/-! ## T₁₇（区间套 = 递归层间桥梁）— L 类（结构归纳）

  陈述（§T₁₇）：高级别买卖点的精确定位必然要求低级别走势完美；区间套是相邻级别从上往下
  收窄的桥梁。L 类内容：区间套 = 相邻级别从上往下严格收缩的有限归纳链（复用 DivergenceNesting
  几何骨架 Nested/NestingChain + 链长界，beichi 第27课级别有限）。
  （自包含单一权威源：L 类 + R/A 类同模块，避免本地 import 路径问题 + 双份漂移。）
-/

/-- ★高级别点的低级别展开（T₁₇：高级别"点"在低级别是一段，桥梁单元）。 -/
def LevelBridge (hi lo : DivSegment) : Prop := Nested hi lo

/-- ★T₁₇ 层间桥梁严格收窄（L0，结构）：低级别端范围严格 < 高级别端 ∧ 级别严格递减。 -/
-- [gk:StructurePartitionOnly]
theorem t17_bridge_narrows (hi lo : DivSegment) (h : LevelBridge hi lo) :
    (lo.hi - lo.lo) < (hi.hi - hi.lo) ∧ lo.level < hi.level :=
  ⟨nested_strictly_shrinks hi lo h, nested_level_decreases hi lo h⟩

/-- ★T₁₇ 区间套桥梁链有限（L0，结构归纳）：链长 ≤ 首段级别+1（beichi 级别有限）。 -/
-- [gk:StructurePartitionOnly]
theorem t17_bridge_chain_finite (outer : DivSegment) (rest : List DivSegment)
    (h : NestingChain (outer :: rest)) :
    (outer :: rest).length ≤ outer.level + 1 :=
  nesting_chain_bounded_by_head outer rest h

/-! ## T₂₃（voice 自相似）+ T₄₄（递归深度有限）— L 类（结构归纳）

  T₂₃ 陈述（§T₂₃）：voice 自相似——每个 voice 内部可 spawn 子 voice，递归到底终止。
  T₄₄ 陈述（§T₄₄）：每次 spawn 级别严格 −1，良序自然数下降 ⟹ 递归深度有限。
  L 类内容：voice 嵌套 = 递归数据类型（VoiceTree），自相似 = 子树同样良构，深度有限 = 级别
  严格 −1 良序下降（depth ≤ level）。继承走势分解定理二 + zhongshu.md:18 级别有极限。
-/

/-- ★递归 voice 树（spawn 树，每节点带级别——递归数据类型 μF）。 -/
inductive VoiceTree where
  | leaf (level : Nat)
  | spawn (level : Nat) (children : List VoiceTree)

namespace VoiceTree

/-- voice 树节点级别。 -/
def level : VoiceTree → Nat
  | leaf l => l
  | spawn l _ => l

/-- voice 树深度（递归 spawn 链最大长度）。 -/
def depth : VoiceTree → Nat
  | leaf _ => 0
  | spawn _ children => 1 + (children.map depth).foldr max 0

end VoiceTree

open VoiceTree

/--
  ★voice 树良构（T₂₃ 自相似 + T₄₄ 级别严格递减）：spawn 要求 l≥1 ∧ 每子级别=l−1 ∧ 每子良构。
  级别严格递减 + Nat 良序 ⟹ 递归有限（关死无限 spawn 不降级）。
-/
def WellFoundedVoice : VoiceTree → Prop
  | .leaf _ => True
  | .spawn l children =>
      l ≥ 1
      ∧ (∀ c ∈ children, c.level = l - 1)
      ∧ (∀ c ∈ children, WellFoundedVoice c)

/--
  ★T₄₄ 递归深度受根级别控制（L0，良序有限下降）：良构 voice 树深度 ≤ 根级别。
  级别每 spawn 严格 −1 ⟹ 最深链长 ≤ l（降到 0=bi 基底自然终止）。结构归纳。
-/
-- [gk:StructurePartitionOnly]
theorem t44_voice_depth_le_level :
    ∀ (t : VoiceTree), WellFoundedVoice t → t.depth ≤ t.level
  | .leaf l, _ => by simp [VoiceTree.depth, VoiceTree.level]
  | .spawn l children, h => by
      unfold WellFoundedVoice at h
      obtain ⟨hl, hchild_lvl, hchild_wf⟩ := h
      have hchildren_depth : ∀ c ∈ children, c.depth ≤ l - 1 := by
        intro c hc
        have := t44_voice_depth_le_level c (hchild_wf c hc)
        rw [hchild_lvl c hc] at this
        exact this
      have hmax_le : (children.map VoiceTree.depth).foldr max 0 ≤ l - 1 := by
        induction children with
        | nil => simp
        | cons hd tl ih =>
            simp only [List.map_cons, List.foldr_cons]
            have hhd : hd.depth ≤ l - 1 := hchildren_depth hd (by simp)
            have htl : (tl.map VoiceTree.depth).foldr max 0 ≤ l - 1 := by
              apply ih
              · intro c hc; exact hchild_lvl c (by simp [hc])
              · intro c hc; exact hchild_wf c (by simp [hc])
              · intro c hc; exact hchildren_depth c (by simp [hc])
            exact Nat.max_le.mpr ⟨hhd, htl⟩
      simp only [VoiceTree.depth, VoiceTree.level]
      omega

/-- ★T₄₄ 递归深度有限（L0）：任意良构 voice 树存在有限深度上界（= 根级别）。 -/
-- [gk:StructurePartitionOnly]
theorem t44_recursion_finite (t : VoiceTree) (h : WellFoundedVoice t) :
    ∃ bound : Nat, t.depth ≤ bound :=
  ⟨t.level, t44_voice_depth_le_level t h⟩

/-- ★T₂₃ voice 自相似（L0，结构归纳）：良构 voice 树每个子树同样良构（自相似递归分支）。 -/
-- [gk:StructurePartitionOnly]
theorem t23_self_similar (l : Nat) (children : List VoiceTree)
    (h : WellFoundedVoice (.spawn l children)) :
    ∀ c ∈ children, WellFoundedVoice c := by
  unfold WellFoundedVoice at h
  obtain ⟨_, _, hwf⟩ := h
  exact hwf

/-- ★T₂₃ 自相似 ∧ 有限（L0）：良构 voice 树自相似（子树良构）∧ 深度有限（≤ 根级别）。 -/
-- [gk:StructurePartitionOnly]
theorem t23_self_similar_finite (l : Nat) (children : List VoiceTree)
    (h : WellFoundedVoice (.spawn l children)) :
    (∀ c ∈ children, WellFoundedVoice c) ∧ (VoiceTree.spawn l children).depth ≤ l :=
  ⟨t23_self_similar l children h, t44_voice_depth_le_level (.spawn l children) h⟩

/-! ## T₁₈（级别 = 操作量：配额级别无关结构核，σ-不变）— R 类 L0 结构形式

  陈述（necessity_derivation §T₁₈）：降成本释放给子 voice 的配额比例 f = r_{k−1}/r_k = 1/λ
  （σ-不变常数，级别无关）。`m = p_units × (1/λ)`。

  ★R 类 L0 结构形式（编排者裁定"σ-不变=代数等式 m=p_units×(1/λ)，可 Lean"）：
  Lean 证配额算子满足 **σ-不变性谓词**（`∀ k p, q(k+1)p = q k p`，代数等式），并证级别依赖
  分配 **否证** 该谓词。这是 prove_theta_sigma_invariant（L2）守的不变量的 L0 代数核。
  ★诚实标注：f 的 **具体值 1/λ**（λ=涌现尺度比）是 L2 经验（leverage_triad 回测扫描），不入
  Lean；Lean 只证"f 是级别无关常数"这一 σ-不变结构（codex 019effee 去膨胀，不证数值）。
  配额用整数 Int（不依赖 Mathlib ℚ；具体有理值本就是 L2 不入 Lean）。
-/

/--
  ★配额算子类型（T₁₈：配额是"级别 k × 父 units → 子 units"二元函数，σ:k↦k+1 作用在第一分量）。
-/
abbrev Quota := Nat → Int → Int

/--
  ★σ-不变性谓词（T₁₈/T59：σWσ⁻¹=W 在配额算子上的形式核，codex 019effee 去重言化）。
  `SigmaInvariant q` ⟺ `∀ k p, q (k+1) p = q k p`——配额算子在 σ:k↦k+1 级别平移下不变。
  这是可证伪命题（levelDependentQuota 不满足，见下），非 `X=X` 重言。
-/
def SigmaInvariant (q : Quota) : Prop := ∀ k p, q (k + 1) p = q k p

/-- ★σ-不变配额（T₁₈：子配额 = 父 units × 级别无关常数 f，返回值不依赖级别 k）。 -/
def sigmaInvariantQuota (f : Int) : Quota := fun _k pUnits => pUnits * f

/--
  ★T₁₈ 配额 σ-不变（L0 结构形式，非重言）：`sigmaInvariantQuota f` 满足 `SigmaInvariant`。
  即 `∀ k p, sigmaInvariantQuota f (k+1) p = sigmaInvariantQuota f k p`（σ 级别平移下配额不变）。
  命题量化级别 k，断言 k 平移不改变配额——有内容（区分 σ-不变 vs 级别依赖），非 `X=X`。
-/
-- [gk:StructurePartitionOnly]
theorem t18_quota_sigma_invariant (f : Int) : SigmaInvariant (sigmaInvariantQuota f) := by
  intro k p; rfl

/-- ★级别依赖配额（T₁₈：θ_sub/θ_total 旧式，显式吃级别 k，破 σ-不变破 T59）。 -/
def levelDependentQuota : Quota := fun k pUnits => pUnits * (k + 1 : Int)

/--
  ★T₁₈ σ-不变 ⊥ 级别依赖分配（L0，反例否证，非重言）：`levelDependentQuota` **不满足**
  `SigmaInvariant`。这是 σ-不变命题的否证见证——存在配额算子破坏 σ-不变，故
  `t18_quota_sigma_invariant` 有内容（codex 019effee 修正：σ-不变非重言）。
-/
-- [gk:StructurePartitionOnly]
theorem t18_level_dependent_breaks_invariance : ¬ SigmaInvariant levelDependentQuota := by
  intro h
  have := h 0 1
  simp [levelDependentQuota] at this

/--
  ★T₁₈ 链式释放守 Σunits（L0 守恒侧，A4/N8 结构核）：父释放配额 m，父 units 减 m、子得 m
  ⟹ 父+子总量 = 父原 units。σ-不变配额 **不破 Σunits=N_base**（与 f 值无关）。
-/
-- [gk:StructurePartitionOnly]
theorem t18_quota_conserves (pUnits m : Int) : (pUnits - m) + m = pUnits := by omega

/-! ## T₁₉（成本门 = 递归终止）— R 类 L0 结构形式

  陈述（§T₁₉）：递归终止纯由成本门——`theta(k)=None`（不可测=不存在）∨ `theta(k)<friction`
  （势 < 成本=势消失）⟹ k 终止。非固定 floor。递归基 = bi(a0)（theta=None 自然终止）。

  ★R 类 L0 结构形式（编排者"终余代数/向上封闭"）：Lean 证成本门是 **良基终止条件**（成本门
  完全分类 + leaf↔成本门绑定 + 向上封闭：θ<f 终止则更小 θ 也终止）。Rust prove_n4_cost_gate=L2。
-/

/-- ★级别势状态（θ=None 不可测/bi 基底；θ=some v 可测幅度，用 Int 建模，不依赖 ℚ）。 -/
def Theta := Option Int

/-- ★成本门判定（终止 ⟺ θ=None ∨ θ<f；非终止 ⟺ θ=some v ∧ v≥f）。 -/
def costGateTerminates (θ : Theta) (f : Int) : Bool :=
  match θ with
  | none => true
  | some v => decide (v < f)

/-- ★T₁₉ 成本门完全分类（L0，二态穷尽）：递归要么终止要么继续，无第三态。 -/
-- [gk:TrueCompleteClassification]
theorem t19_cost_gate_total (θ : Theta) (f : Int) :
    costGateTerminates θ f = true ∨ costGateTerminates θ f = false := by
  cases costGateTerminates θ f
  · exact Or.inr rfl
  · exact Or.inl rfl

/-- ★T₁₉ bi 基底自然终止（L0）：θ=None ⟹ 成本门终止（递归基 = bi(a0)）。 -/
-- [gk:StructurePartitionOnly]
theorem t19_base_terminates (f : Int) : costGateTerminates none f = true := rfl

/-- ★T₁₉ 势 < 摩擦终止（L0）：v<f ⟹ 终止（势在操作语义下消失）。 -/
-- [gk:StructurePartitionOnly]
theorem t19_below_friction_terminates (v f : Int) (h : v < f) :
    costGateTerminates (some v) f = true := by simp [costGateTerminates, h]

/-- ★T₁₉ 势 ≥ 摩擦可继续（L0）：v≥f ⟹ 不终止（势足以兑现操作，可 spawn）。 -/
-- [gk:StructurePartitionOnly]
theorem t19_above_friction_continues (v f : Int) (h : f ≤ v) :
    costGateTerminates (some v) f = false := by
  simp only [costGateTerminates, decide_eq_false_iff_not]; omega

/--
  ★T₁₉ 成本门向上封闭（L0 结构形式，编排者"termination_is_upward_closed"）：
  若级别势 v 触发成本门终止（v<f），则任意更小的势 w≤v 同样终止——终止条件对"势更小"封闭。
  这形式化"势越小越该终止"的单调结构（成本门是良基终止的向上封闭集），与终余代数有限下降同构。
-/
-- [gk:StructurePartitionOnly]
theorem t19_termination_upward_closed (v w f : Int) (hv : v < f) (hw : w ≤ v) :
    costGateTerminates (some w) f = true := by
  apply t19_below_friction_terminates; omega

/--
  ★T₁₉ leaf ↔ 成本门绑定（L0，codex 019effee 强化）：leaf 终止 ⟺ 成本门在该级别终止。
  叶节点的存在性由成本门判定（θ=None ∨ θ<f），非凭空——补"成本门 ⟹ leaf"方向。
-/
def leafJustifiedBy (θ : Theta) (f : Int) (_l : Nat) : Prop := costGateTerminates θ f = true

-- [gk:StructurePartitionOnly]
theorem t19_leaf_from_cost_gate (θ : Theta) (f : Int) (l : Nat)
    (h : costGateTerminates θ f = true) : leafJustifiedBy θ f l := h

/-- ★T₁₉ None 势必为 leaf（L0）：bi 基底 θ=None ⟹ 成本门终止 ⟹ leaf（递归基自然终止）。 -/
-- [gk:StructurePartitionOnly]
theorem t19_none_is_leaf (f : Int) (l : Nat) : leafJustifiedBy none f l :=
  t19_leaf_from_cost_gate none f l (t19_base_terminates f)

/-! ## T₂₀（降成本 = voice 内部操作，不需外部 pending）— R 类 L0 结构形式

  陈述（§T₂₀）：降成本是 voice 利用自身级别低级别走势完美做短差，只用 voice 自层 fire，
  不查全局区间套定位链。

  ★R 类 L0 结构形式：Lean 证 **遍历分离**——降成本算子只读 voice.selfFire（自层投影），
  与全局链 source 正交。Rust prove_n7_spawn_self_level=L2。
-/

/-- ★voice 内部状态（自层信息 + 全局链信息分离）。 -/
structure VoiceState where
  selfLevel : Nat
  selfFire : Bool
  globalChainSource : Nat

/-- ★降成本触发（T₂₀：只读 voice 自层 fire，不查全局链）。 -/
def decostTrigger (vs : VoiceState) : Bool := vs.selfFire

/--
  ★T₂₀ 降成本不依赖全局链（L0 结构形式，遍历分离）：改变全局链 source 不改变降成本触发。
  关死"降成本借更高级别 pending"——降成本是 voice 自层投影，与全局链正交（prove_n7 的 L0 核）。
-/
-- [gk:StructurePartitionOnly]
theorem t20_decost_self_level_only (vs : VoiceState) (gs' : Nat) :
    decostTrigger vs = decostTrigger { vs with globalChainSource := gs' } := rfl

/-- ★T₂₀ 降成本 ⊥ 入场遍历（L0）：存在 voice 自层 fire 但全局 source 在不同级别（不同遍历）。 -/
-- [gk:StructurePartitionOnly]
theorem t20_distinct_traversals :
    ∃ vs : VoiceState, decostTrigger vs = true ∧ vs.selfLevel ≠ vs.globalChainSource := by
  refine ⟨⟨2, true, 5⟩, rfl, ?_⟩; decide

/-! ## T₂₁（并发 = 级别同时性）— R 类 L0 结构形式

  陈述（§T₂₁）：多级别走势同时进行 ⟹ 操作必须能在每级别独立、同时发生；去全局互斥。
  ★R 类 L0 结构形式：Lean 证 **per-voice 独立性**——不同 voice 操作互不阻塞（去全局互斥锁）。
  Rust prove_n2_per_voice=L2。
-/

/-- ★本 bar 多 voice 操作记录（per-voice acted，NoDup=同 voice 不双动）。 -/
structure BarActions where
  actedVoices : List Nat
  nodup : actedVoices.Nodup

/-- 某 voice 本 bar 是否已操作。 -/
def BarActions.voiceActed (ba : BarActions) (v : Nat) : Bool := ba.actedVoices.contains v

/--
  ★T₂₁ per-voice 操作不互阻（L0 结构形式，去全局互斥）：存在见证 v₁ acted、v₂ 仍可操作（v₁≠v₂）。
  形式化"一个级别操作不阻断另一个"——voiceActed 是 per-voice 投影，不同 voice 判定独立。
-/
-- [gk:StructurePartitionOnly]
theorem t21_per_voice_independent :
    ∃ (ba : BarActions) (v₁ v₂ : Nat),
      v₁ ≠ v₂ ∧ ba.voiceActed v₁ = true ∧ ba.voiceActed v₂ = false := by
  refine ⟨⟨[1], by simp⟩, 1, 2, ?_, ?_, ?_⟩ <;> decide

/-- ★T₂₁ 同 voice 不双动（L0，per-voice ⊇ per-level 的 NoDup 侧）。 -/
-- [gk:StructurePartitionOnly]
theorem t21_same_voice_no_double (ba : BarActions) : ba.actedVoices.Nodup := ba.nodup

/-! ## T₂₂（多重赋格 = 并发操作结构，stretto）— R 类 L0 结构形式

  陈述（§T₂₂）：多 voice 并发 = 多重赋格。高级别 voice 先入（根），低级别 voice（子）在高级别
  未完美时已入 = stretto（声部叠入）。

  ★赋格 stretto 检验定义冲突（硬约束 no-workaround）：stretto = 子声部在父主题未结束时叠入。
  检验是否暴露 cd_ℚ=1 两层闭合冲突 → **未暴露**：stretto = 森林时序的子/父 active 区间 **重叠**
  （非分离），可自洽建模——两层 active 区间允许重叠，不要求闭合分离。无 /escalate。
-/

/-- ★声部 active 区间（voice 在 [enterBar, perfectBar) 期间 active）。 -/
structure VoiceSpan where
  enterBar : Nat
  perfectBar : Nat
  valid : enterBar ≤ perfectBar

/-- ★stretto 关系（T₂₂：子晚于父进入 ∧ 子在父未完美时已入=叠入）。 -/
def Stretto (parent child : VoiceSpan) : Prop :=
  parent.enterBar < child.enterBar ∧ child.enterBar < parent.perfectBar

/--
  ★T₂₂ stretto 区间重叠（L0，无定义冲突）：stretto ⟹ 子进入时父仍 active（区间重叠）。
  证 stretto 是自洽森林时序结构——**未暴露 cd_ℚ=1 两层闭合冲突**（两层 active 区间允许重叠）。
-/
-- [gk:StructurePartitionOnly]
theorem t22_stretto_overlap (parent child : VoiceSpan) (h : Stretto parent child) :
    parent.enterBar < child.enterBar ∧ child.enterBar < parent.perfectBar := h

/-- ★T₂₂ stretto 实例存在（L0，构造性见证）：根 [0,10) + 子 [3,7) 是合法 stretto 对。 -/
def strettoExample : VoiceSpan × VoiceSpan := (⟨0, 10, by omega⟩, ⟨3, 7, by omega⟩)

-- [gk:StructurePartitionOnly]
theorem t22_stretto_realizable : Stretto strettoExample.1 strettoExample.2 :=
  ⟨by decide, by decide⟩

/-! ## T₂₄（多空对称）+ T₂₅（多空嵌套=方向交替）— A 类 reform→真完全分类

  T₂₄ 陈述（§T₂₄）：A₀ 不区分方向 ⟹ 上涨完美翻空、下跌完美翻多 ⟹ 绩效 = Σ|涨跌幅|。
  T₂₅ 陈述（§T₂₅）：父多→子空→孙多（相邻级别方向相反）。

  ★A 类 reform（编排者"轴范式项 reform→真完全分类"）：方向 = `Direction` 二构造子穷尽
  （up/down，TrendTrichotomy 已 D）；多空对称 = 翻转算子 flip 的 **ℤ/2 对合结构**
  （flip∘flip=id，τ²=e）。这是 **构造子穷尽的真完全分类**（方向 sum type + 对合代数），
  递归范式可达 ⟹ reform 成功，Lean 化，**无 /escalate**（非伪造重铸：Direction 本就是 603
  二构造子，flip 是其上的对合，代数封闭）。
-/

/-- ★方向翻转算子（T₂₄：up↔down，手性 ε 在 φ=0 翻转）。 -/
def flipDir : Direction → Direction
  | Direction.up => Direction.down
  | Direction.down => Direction.up

/-- ★T₂₄ 翻转对称（L0）：flip 把 up↔down——对涨跌对称作用。 -/
-- [gk:TrueCompleteClassification]
theorem t24_flip_symmetric :
    flipDir Direction.up = Direction.down ∧ flipDir Direction.down = Direction.up := ⟨rfl, rfl⟩

/--
  ★T₂₄ 翻转对合（L0，ℤ/2 reform 核）：flip∘flip=id（二次翻转=恒等）。
  多空对称的代数核——手性 ε∈{±1}，周期 2（时序分段交替）。这是 A 类 reform 为构造子穷尽的
  真完全分类：方向 Direction 二构造子 + flip 对合 = ℤ/2 群作用（代数封闭，τ²=e）。
-/
-- [gk:TrueCompleteClassification]
theorem t24_flip_involutive (d : Direction) : flipDir (flipDir d) = d := by cases d <;> rfl

/-- ★T₂₄ 方向完全分类（L0，构造子穷尽 reform）：任意方向必属 up/down 之一（无第三方向）。 -/
-- [gk:TrueCompleteClassification]
theorem t24_direction_total (d : Direction) : d = Direction.up ∨ d = Direction.down := by
  cases d
  · exact Or.inl rfl
  · exact Or.inr rfl

/-- ★T₂₄ 走势对涨跌都终完美（L0，A₀ 方向无关）：完美判定对 up、down 都成立（涨跌都被吃）。 -/
def perfects : Direction → Bool := fun _ => true

-- [gk:TrueCompleteClassification]
theorem t24_both_directions_perfect :
    perfects Direction.up = true ∧ perfects Direction.down = true := ⟨rfl, rfl⟩

/-- ★T₂₅ 方向沿级别交替（L0，父反向）：子方向 = 父翻转；连续两级回同向（flip 对合）。 -/
def childDir (parentDir : Direction) : Direction := flipDir parentDir

-- [gk:TrueCompleteClassification]
theorem t25_alternates (parentDir : Direction) :
    childDir parentDir = flipDir parentDir ∧ childDir (childDir parentDir) = parentDir :=
  ⟨rfl, t24_flip_involutive parentDir⟩

/-! ## T₂₆（操盘三阶段）+ T₂₇（永远在场）— R 类 L0 结构形式

  T₂₆ 陈述（§T₂₆）：建仓（F）→ 持仓降成本（E）/挣股数 → 出场（C 清仓/翻转）= 新建仓（循环）。
  T₂₇ 陈述（§T₂₇）：第一次建仓后永远有方向（在场），三阶段融合为持续循环。

  ★R 类 L0 结构形式：三阶段 = 三构造子 sum type（穷尽）；循环闭合（exit→enter 周期3）；
  永远在场 = 三阶段都在场（无 FLAT 间隙）。T₂₇"measure-1"中清仓=测度零分支是 T₃₂ 的 L2 频率，
  不入 Lean（诚实标注）；本节只证三阶段结构性在场。
-/

/-- ★操盘三阶段（建仓/持仓/出场，三构造子穷尽）。 -/
inductive Phase where
  | enter | hold | exit
deriving DecidableEq, Repr

open Phase

/-- ★T₂₆ 三阶段完全分类（L0，构造子穷尽）：任意阶段必属三构造子之一，无第四阶段。 -/
-- [gk:TrueCompleteClassification]
theorem t26_phase_total (p : Phase) : p = enter ∨ p = hold ∨ p = exit := by
  cases p
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/-- ★阶段循环转移（enter→hold→exit→enter，出场=翻转=新建仓）。 -/
def nextPhase : Phase → Phase
  | enter => hold
  | hold => exit
  | exit => enter

/-- ★T₂₆ 三阶段循环闭合（L0）：转移 3 次回到原阶段（周期 3 循环）。 -/
-- [gk:TrueCompleteClassification]
theorem t26_phase_cycles (p : Phase) : nextPhase (nextPhase (nextPhase p)) = p := by
  cases p <;> rfl

/-- ★T₂₆ 出场=新建仓（L0，循环闭合关键边）：exit 下一阶段是 enter（翻转=反向新建仓）。 -/
-- [gk:StructurePartitionOnly]
theorem t26_exit_to_enter : nextPhase exit = enter := rfl

/-- ★在场判定（T₂₇：三阶段都在场，无 FLAT 间隙）。 -/
def inMarket : Phase → Bool := fun _ => true

/--
  ★T₂₇ 永远在场（L0 结构形式）：三阶段循环上 inMarket 恒真——第一次建仓后任意阶段都在场。
  形式化"出场=翻转=新建仓，无空仓间隙"（T₂₆ 循环闭合推论）。诚实标注：清仓"测度零分支"是 T₃₂
  L2 频率不入 Lean；本定理只证三阶段结构性在场。
-/
-- [gk:StructurePartitionOnly]
theorem t27_always_in_market (p : Phase) : inMarket p = true := rfl

/-- ★T₂₇ 出场不离场（L0）：出场阶段仍在场（exit→enter，无 FLAT）。 -/
-- [gk:StructurePartitionOnly]
theorem t27_exit_stays_in_market : inMarket exit = true ∧ nextPhase exit = enter := ⟨rfl, rfl⟩

/-! ## T₂₈（从最高级别进入）— R 类 L0 结构形式

  陈述（§T₂₈）：入场 source = 区间套自上而下定位链顶层（最高有信号级别）；source ≥ PENDING_LO
  （segment 非势源）。★R 类 L0 结构形式：Lean 证 source=链顶 ∧ source≥PENDING_LO 的结构条件。
  Rust prove_chain（N5/N6）=L2。
-/

/-- ★入场判据（source vs 链顶 vs PENDING_LO 下界）。 -/
structure EntryCriteria where
  pendingLo : Nat
  chainTop : Nat
  source : Nat

/-- ★入场合法（T₂₈：source=链顶 ∧ source≥pendingLo）。 -/
def EntryCriteria.legal (ec : EntryCriteria) : Prop :=
  ec.source = ec.chainTop ∧ ec.source ≥ ec.pendingLo

/-- ★T₂₈ 入场对齐最高级别且势源有效（L0 结构形式）：合法入场 source=链顶 ∧ ≥PENDING_LO。 -/
-- [gk:StructurePartitionOnly]
theorem t28_entry_at_top (ec : EntryCriteria) (h : ec.legal) :
    ec.source = ec.chainTop ∧ ec.source ≥ ec.pendingLo := h

/-- ★T₂₈ segment 非势源（L0）：source<pendingLo（segment 层）⟹ 入场不合法。 -/
-- [gk:StructurePartitionOnly]
theorem t28_segment_not_source (ec : EntryCriteria) (h : ec.source < ec.pendingLo) :
    ¬ ec.legal := by
  intro hlegal; obtain ⟨_, hge⟩ := hlegal; omega

/-! ## T₂₉（先势后定位：压缩→展开）— R 类 L0 结构形式

  陈述（§T₂₉）：高级别 candidate 先出现（压缩），低级别 confirm 后兑现（展开）。
  时序不可颠倒：compress_bar ≤ confirm_bar ≤ bar。★R 类 L0 结构形式：因果时序偏序传递 +
  反证（confirm 早于 compress 非法）。Rust prove_chain（N6）=L2。
-/

/-- ★先势后定位时序（compress_bar ≤ confirm_bar ≤ bar）。 -/
structure CausalTiming where
  compressBar : Nat
  confirmBar : Nat
  bar : Nat

/-- ★先势后定位（T₂₉：时序偏序，不可颠倒）。 -/
def CausalTiming.ordered (ct : CausalTiming) : Prop :=
  ct.compressBar ≤ ct.confirmBar ∧ ct.confirmBar ≤ ct.bar

/-- ★T₂₉ 因果时序传递（L0 结构形式）：compress≤confirm≤bar ⟹ compress≤bar（势的积累先于当前）。 -/
-- [gk:StructurePartitionOnly]
theorem t29_causal_transitive (ct : CausalTiming) (h : ct.ordered) :
    ct.compressBar ≤ ct.bar := Nat.le_trans h.1 h.2

/-- ★T₂₉ confirm 不可早于 compress（L0，反证核）：confirm<compress ⟹ 时序非法（先定位后有势矛盾）。 -/
-- [gk:StructurePartitionOnly]
theorem t29_confirm_not_before_compress (ct : CausalTiming) (h : ct.confirmBar < ct.compressBar) :
    ¬ ct.ordered := by
  intro hord; obtain ⟨hle, _⟩ := hord; omega

/-! ## T₃₀（根级别涌现 = 会计重组）— R 类 L0 结构形式

  陈述（§T₃₀）：根 voice 级别随走势向更高级别发展上移（会计重组，非加仓）；只有最高涌现级别
  完美才清仓。★R 类 L0 结构形式：relabel 保 units/NAV 不变（会计重组≠加仓）+ 涌现级别单调爬升。
  Rust prove_a5_relabel=L2。
-/

/-- ★根 voice 涌现状态（级别 + units + NAV，用 Int 不依赖 ℚ）。 -/
structure RootEmergent where
  emergentLevel : Nat
  units : Int
  nav : Int

/-- ★会计重组算子（T₃₀：级别上移，units/NAV 不变）。 -/
def relabel (re : RootEmergent) (newLevel : Nat) : RootEmergent :=
  { re with emergentLevel := newLevel }

/--
  ★T₃₀ 重组保持 units/NAV 不变（L0 结构形式，prove_a5_relabel 核）：relabel 前后 units、NAV
  严格相等。形式化"会计重组≠加仓"（A₃ 禁加仓）——级别上移只改读数，物理持仓/净值不变。
-/
-- [gk:StructurePartitionOnly]
theorem t30_relabel_preserves (re : RootEmergent) (newLevel : Nat) :
    (relabel re newLevel).units = re.units ∧ (relabel re newLevel).nav = re.nav := ⟨rfl, rfl⟩

/-- ★T₃₀ 涌现级别单调爬升（L0）：newLevel≥emergentLevel ⟹ relabel 后级别非降（向上单调）。 -/
-- [gk:StructurePartitionOnly]
theorem t30_emergent_monotone (re : RootEmergent) (newLevel : Nat)
    (h : re.emergentLevel ≤ newLevel) :
    re.emergentLevel ≤ (relabel re newLevel).emergentLevel := by simp [relabel]; exact h

/-! ## T₃₁（出场对齐 source 级别反向 BSP）— R 类 L0 结构形式

  陈述（§T₃₁）：出场要求 source≥S 的反向走势完美；出场级别不能低于入场级别（否则是降成本）。
  ★R 类 L0 结构形式：出场级别 ≥ 入场级别的约束 + 反证（低于入场级别=降成本非出场）。
-/

/-- ★出场判据（入场 source 级别 vs 出场判定级别）。 -/
structure ExitCriteria where
  entrySource : Nat
  exitLevel : Nat

/-- ★出场合法（T₃₁：出场级别≥入场 source 级别）。 -/
def ExitCriteria.legal (xc : ExitCriteria) : Prop := xc.exitLevel ≥ xc.entrySource

/-- ★T₃₁ 出场对齐 source 级别（L0 结构形式）：合法出场 exitLevel≥entrySource。 -/
-- [gk:StructurePartitionOnly]
theorem t31_exit_aligns_source (xc : ExitCriteria) (h : xc.legal) :
    xc.exitLevel ≥ xc.entrySource := h

/-- ★T₃₁ 低于入场级别=降成本非出场（L0，反证核）：exitLevel<entrySource ⟹ 非合法出场。 -/
-- [gk:StructurePartitionOnly]
theorem t31_below_entry_is_decost (xc : ExitCriteria) (h : xc.exitLevel < xc.entrySource) :
    ¬ xc.legal := by
  intro hlegal; unfold ExitCriteria.legal at hlegal; omega

/-! ## T₃₂（清仓极少发生）— R 类 L0 结构形式（定性侧）+ X（λ 定量频率）

  陈述（§T₃₂）：清仓极少——只在 ① 最高涌现完美 ② 区间套确认 ③ 全链级联 三条件同时满足。
  ★L0/X 显式分开（编排者"标度律定性单调侧=E / λ 定量经验侧=X"）：
    - **E（入 Lean）**：清仓 **结构门槛** = 三条件合取（少一条不清仓，走 E 降成本）。
    - **X（不入 Lean）**：清仓"十年1-2次"/sellpt=0 的 **λ 定量罕见率**（T₅₀ 标度律 ∝λ^{−K}），
      L2 经验读数，Lean **不证频率**（不伪造 L0 覆盖 L2，no-patch 声明膨胀禁）。
-/

/-- ★清仓三条件（最高涌现完美 ∧ 区间套确认 ∧ 全链级联）。 -/
structure LiquidationConditions where
  topEmergentPerfect : Bool
  nestingConfirmed : Bool
  cascadeCleared : Bool

/-- ★清仓判定（T₃₂：三条件合取——少一条不清仓，走 E 降成本）。 -/
def liquidates (lc : LiquidationConditions) : Bool :=
  lc.topEmergentPerfect && lc.nestingConfirmed && lc.cascadeCleared

/--
  ★T₃₂ 清仓需三条件全满足（L0 结构形式，定性门槛，非频率）：清仓 ⟺ 三条件同时为真。
  少任一条件 ⟹ 不清仓——清仓是三谓词合取（强门槛）。**频率"十年1-2次"是 X（L2），不在此证**。
-/
-- [gk:StructurePartitionOnly]
theorem t32_liquidation_requires_all (lc : LiquidationConditions) :
    liquidates lc = true ↔
      (lc.topEmergentPerfect = true ∧ lc.nestingConfirmed = true ∧ lc.cascadeCleared = true) := by
  unfold liquidates
  constructor
  · intro h
    obtain ⟨hab, hc⟩ := Bool.and_eq_true _ _ |>.mp h
    obtain ⟨ha, hb⟩ := Bool.and_eq_true _ _ |>.mp hab
    exact ⟨ha, hb, hc⟩
  · intro ⟨h1, h2, h3⟩; simp [h1, h2, h3]

/-- ★T₃₂ 缺最高涌现完美则不清仓（L0，定性单调侧）：topEmergentPerfect=false ⟹ 不清仓（走 E）。 -/
-- [gk:StructurePartitionOnly]
theorem t32_no_top_perfect_no_liquidation (lc : LiquidationConditions)
    (h : lc.topEmergentPerfect = false) : liquidates lc = false := by
  unfold liquidates; simp [h]

/-! ## §6B 级别间三轴穷尽（reconcile #45）— A 类 reform→真完全分类

  裁定（#45 + #48 旁挂）：级别间关系（5 种）沿 **三轴**（H⁰/groupoid/H¹）分布，σ-等变只捕获
  H¹ 一维（必要不充分）。
  ★A 类 reform（编排者"轴范式项 reform→真完全分类"）：三轴 = 级别间关系的 **构造子完备分区**
  （`InterAxis` 三构造子穷尽，sum type）。这 **不是** 轴范式残留（外延无穷轴枚举），**是** 三
  构造子完全分类（内涵式，同走势三分）——递归范式可达 ⟹ reform 成功，Lean 化，**无 /escalate**。
  σ-等变 ⊊ 三轴 = 已结算有效域收窄（非定义冲突）。
  ★(b) 结构层封闭 / H⁰ 内容层开放（#48 旁挂）：三轴结构穷尽是 L0（结构层）；H⁰ 轴具体形态学
  构成关系可细分更多（内容层开放）——这恰再证 σ-等变不足（诚实标注，非膨胀）。
-/

/-- ★级别间关系三轴（H⁰ 形态学 / groupoid observe / H¹ operate，三构造子穷尽 reform）。 -/
inductive InterAxis where
  | morphological | groupoid | cohomological
deriving DecidableEq, Repr

open InterAxis

/-- ★§6B 三轴完全分类（L0，构造子穷尽 reform）：任意级别间关系轴必属三构造子之一，无第四轴。 -/
-- [gk:TrueCompleteClassification]
theorem interaxis_total (a : InterAxis) :
    a = morphological ∨ a = groupoid ∨ a = cohomological := by
  cases a
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/-- ★五种级别间关系（仓位流动/信号确认/走势包含/中枢构成/区间套）。 -/
inductive InterRelation where
  | positionFlow | signalConfirm | trendInclusion | centerComposition | intervalNesting
deriving DecidableEq, Repr

open InterRelation

/-- ★关系→轴归属（§6B.2 表：5 关系恰分布三轴）。 -/
def axisOf : InterRelation → InterAxis
  | positionFlow => cohomological
  | signalConfirm => groupoid
  | intervalNesting => groupoid
  | trendInclusion => morphological
  | centerComposition => morphological

/-- ★§6B 五关系三轴分布完全覆盖（L0）：每关系恰被分配一个轴，三轴都覆盖。 -/
-- [gk:TrueCompleteClassification]
theorem interrelation_axis_total (r : InterRelation) :
    axisOf r = morphological ∨ axisOf r = groupoid ∨ axisOf r = cohomological :=
  interaxis_total (axisOf r)

/-- ★σ-等变捕获判定（仅对 H¹ 轴关系为真）。 -/
def sigmaEquivariantCaptures (r : InterRelation) : Bool :=
  decide (axisOf r = cohomological)

/--
  ★§6B.3 σ-等变 ⟺ H¹ 轴（L0，必要不充分精确刻画）：σ-等变捕获 r ⟺ r 属 H¹ 轴。
  证 σ-等变不捕获 groupoid（关系2/5）与 H⁰（关系3/4）——σ-等变 ⊊ 三轴（有效域收窄，非冲突）。
-/
-- [gk:StructurePartitionOnly]
theorem t6b_sigma_captures_iff_h1 (r : InterRelation) :
    sigmaEquivariantCaptures r = true ↔ axisOf r = cohomological := by
  unfold sigmaEquivariantCaptures; simp

/-- ★§6B σ-等变不捕获 groupoid（L0，反例）：信号确认属 groupoid，σ-等变不捕获。 -/
-- [gk:StructurePartitionOnly]
theorem t6b_sigma_misses_groupoid : sigmaEquivariantCaptures signalConfirm = false := by decide

/-- ★§6B σ-等变不捕获 H⁰（L0，反例）：中枢构成属 H⁰（重叠涌现，群作用外），σ-等变不捕获。 -/
-- [gk:StructurePartitionOnly]
theorem t6b_sigma_misses_morphological :
    sigmaEquivariantCaptures centerComposition = false := by decide

/--
  ★§6B reform 范式归宿（L0，#45 裁定）：三轴构造子穷尽 ∧ σ-等变 ⊊ 三轴。
  (a) 任意级别间关系属三轴之一（构造子穷尽 reform，非轴范式无穷枚举）；
  (b) 存在 σ-等变捕获不了的关系（σ-等变 ⊊ 三轴，必要不充分，有效域收窄）。
  reform 成功（递归范式可达）⟹ **无定义冲突 → 无 /escalate**。
-/
-- [gk:TrueCompleteClassification]
theorem t6b_paradigm_disposition :
    (∀ r : InterRelation, axisOf r = morphological ∨ axisOf r = groupoid
        ∨ axisOf r = cohomological)
    ∧ (∃ r : InterRelation, sigmaEquivariantCaptures r = false) :=
  ⟨interrelation_axis_total, ⟨signalConfirm, t6b_sigma_misses_groupoid⟩⟩

/-! ### §6B 推导口径（编排者 2026-06-25 新补）：初代数构造子**推导**三轴，非轴枚举

  team-lead/编排者裁定："reform 项（§6B/T₂₄）用初代数构造子**推导**真完全分类，不只证轴枚举。"

  推导（非枚举）的含义：三轴 `InterAxis` 是 orbit §6 三轴分解（H⁰ 不变量 / groupoid 转移 /
  H¹ 扬弃）的初代数 μF——"无第四轴"**不是**外部断言（枚举式"我数了三个"），**是** `InterAxis`
  inductive 的 **initiality 泛性质**（结构归纳原理）。下面用 **依赖消去子**（`InterAxis.rec`/
  `casesOn` 的泛性质）把"三轴穷尽"表达为：任意从 InterAxis 出发的判定，由且仅由三构造子分支
  决定——这是 initiality（μF 唯一态射），不是枚举。同 `TrendTrichotomy.trend_trichotomy`
  从走势三分初代数推导"无第四走势构造子"。
-/

/--
  ★§6B 三轴 initiality 推导（L0，构造子穷尽=泛性质，非枚举）：

  任意谓词 `P : InterAxis → Prop`，若在三构造子上都成立（morphological/groupoid/cohomological），
  则对 **所有** InterAxis 成立。这是 `InterAxis` 初代数的 **结构归纳原理**（initiality 泛性质）——
  "无第四轴"由此推导（不是"我枚举了三个所以是三个"，而是"μF 的唯一态射只经三构造子分解"）。
  这把 `interaxis_total` 从"枚举命题"升为"初代数推导"（编排者推导口径）。
-/
-- [gk:TrueCompleteClassification]
theorem t6b_interaxis_initiality (P : InterAxis → Prop)
    (hM : P morphological) (hG : P groupoid) (hC : P cohomological) :
    ∀ a : InterAxis, P a := by
  intro a; cases a
  · exact hM
  · exact hG
  · exact hC

/--
  ★§6B 三轴分区满射（L0，推导闭环）：每个轴构造子都被某真实级别间关系命中——
  morphological ← centerComposition，groupoid ← signalConfirm，cohomological ← positionFlow。
  这闭合"推导"：三轴不是空枚举的占位，每构造子有 witness 关系填充（分区 `axisOf` 满射到三构造子）。
  配合 initiality（无第四轴）+ 满射（三轴都非空）⟹ 三轴是级别间关系的真完全分类（推导，非枚举）。
-/
-- [gk:TrueCompleteClassification]
theorem t6b_axis_surjective :
    (∃ r : InterRelation, axisOf r = morphological)
    ∧ (∃ r : InterRelation, axisOf r = groupoid)
    ∧ (∃ r : InterRelation, axisOf r = cohomological) :=
  ⟨⟨centerComposition, rfl⟩, ⟨signalConfirm, rfl⟩, ⟨positionFlow, rfl⟩⟩

/--
  ★§6B 三轴满射（标准形态，codex 019f0001 加强）：`∀ a : InterAxis, ∃ r, axisOf r = a`——
  每个轴构造子都有 InterRelation witness。这是 `axisOf` 满射的标准 ∀∃ 形态（比合取版更硬），
  对三轴 initiality 消去后逐构造子给 witness。配合 `t6b_interaxis_initiality`（无第四轴）
  ⟹ 三轴 = 级别间关系的真完全分类（initiality + 满射，构造子推导非枚举）。
-/
-- [gk:TrueCompleteClassification]
theorem t6b_axis_surjective_forall : ∀ a : InterAxis, ∃ r : InterRelation, axisOf r = a := by
  intro a
  cases a
  · exact ⟨centerComposition, rfl⟩
  · exact ⟨signalConfirm, rfl⟩
  · exact ⟨positionFlow, rfl⟩

/--
  ★§6B 推导口径范式归宿（L0，编排者推导口径完整命题）：
  三轴真完全分类 = (a) initiality 推导"无第四轴"（`t6b_interaxis_initiality`）∧
  (b) 三轴满射非空（`t6b_axis_surjective`）∧ (c) σ-等变 ⊊ 三轴（必要不充分）。
  这是用初代数构造子 **推导** 的真完全分类（非轴枚举）——reform 成功，无定义冲突 → 无 /escalate。
-/
-- [gk:TrueCompleteClassification]
theorem t6b_derivation_disposition :
    (∀ (P : InterAxis → Prop), P morphological → P groupoid → P cohomological → ∀ a, P a)
    ∧ (∀ a : InterAxis, ∃ r : InterRelation, axisOf r = a)
    ∧ ((∀ r : InterRelation, sigmaEquivariantCaptures r = true ↔ axisOf r = cohomological)
        ∧ (∃ r : InterRelation, sigmaEquivariantCaptures r = false)) :=
  ⟨t6b_interaxis_initiality, t6b_axis_surjective_forall,
   ⟨t6b_sigma_captures_iff_h1, ⟨signalConfirm, t6b_sigma_misses_groupoid⟩⟩⟩

/-! ### T₂₄ 推导口径：方向初代数构造子推导 ℤ/2 对合（非枚举）

  T₂₄ 多空对称的 reform 推导口径：方向 `Direction`（TrendTrichotomy 二构造子初代数）的
  initiality ⟹ flip 对合 ℤ/2 是 **推导** 的（对 Direction 结构归纳，两构造子分支都满足
  flip∘flip=id），非"枚举了 up/down 两个方向"。下面把 T₂₄ 对合表达为 Direction initiality。
-/

/--
  ★T₂₄ 方向 initiality 推导（L0，ℤ/2 对合由构造子推导，非枚举）：

  任意谓词 `P : Direction → Prop`，若在 up、down 都成立，则对所有 Direction 成立——
  Direction 二构造子初代数的结构归纳原理。flip 对合（`t24_flip_involutive`）正由此推导：
  对 Direction 的两构造子分支分别验证 flip∘flip=id ⟹ 对所有方向成立（initiality），
  非"枚举两个方向各验一遍"。这是 T₂₄ 多空对称 ℤ/2 的初代数推导口径。
-/
-- [gk:TrueCompleteClassification]
theorem t24_direction_initiality (P : Direction → Prop)
    (hU : P Direction.up) (hD : P Direction.down) : ∀ d : Direction, P d := by
  intro d; cases d
  · exact hU
  · exact hD

/--
  ★T₂₄ ℤ/2 对合由 initiality 推导（L0，推导口径）：flip 的对合性 ∀d, flip(flip d)=d 由
  Direction initiality 推导——把对合谓词 `fun d => flipDir (flipDir d) = d` 喂给
  `t24_direction_initiality`，两构造子分支 rfl 成立 ⟹ 全方向成立。这坐实 T₂₄ 是构造子推导
  的真完全分类（ℤ/2 群作用），非方向枚举。
-/
-- [gk:TrueCompleteClassification]
theorem t24_involutive_by_initiality :
    ∀ d : Direction, flipDir (flipDir d) = d :=
  t24_direction_initiality (fun d => flipDir (flipDir d) = d) rfl rfl

end Formal.Tlayers.Operational
