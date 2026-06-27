/-
  Origin/VoiceThreeLevel.lean — 三级别声部 b_v⪰o_v⪰e_v + 父子许可 G_{v,t} 的结构层 L0 形式化
  ★task 编排者+Lead 裁定（补 PDF-only 2a：三级别声部 + 父子许可结构层）

  ── 存在论位置（结构细化，非重证从零）─────────────────────────────────────────
  Origin canonical 已有两层关于声部/级别的结构：
    (1) `Origin/VoiceTree.lean`（committed，本文件 owner 不改）：单级别声部树
        (parent/side/closed/depth + alternating + cascade_close)——声部**方向**的赋格交替
        + 级联关闭，但每声部只携带**一个** depth（单级别）。
    (2) §6 区间套（canonical 操作级 o_v=ℓ_0>…>ℓ_k=e_v 执行级，`Origin/SubLevelDescent.lean`
        已结构化）：操作级 descend 到执行级，级别严格递减（`descend_level_decreases`）。

  本文件是这两层的**结构细化**：把声部的单一 depth 细化为**三级别** (b_v, o_v, e_v)——
  背景判断级 b_v ⪰ 操作级 o_v ⪰ 区间套执行级 e_v（级别 Nat 序）。背景级 b_v 在操作级之上
  （比 §6 区间套 o_v→e_v 多出顶层背景判断级），三者构成级别全序链。

  ── 核心结构（PDF p18-23 转述 + §6 区间套 + Strict/Fugue.lean §4 同源记号）───────
  1. 三级别声部 (b_v, o_v, e_v)：级别 Nat，序约束 b_v ≥ o_v ≥ e_v。
     · 背景判断级 b_v：高级别，决定「**能不能做**」（市场背景 + 父声部状态）。
     · 操作级 o_v：中级别，决定操作方向（§6 区间套顶 ℓ_0）。
     · 区间套执行级 e_v：低级别，决定「**在哪做**」（§6 区间套底 ℓ_k，买卖点精确定位）。

  2. 父子许可 G_{v,t}（PDF p18-23）= 结构合取
        Normal ∧ x_{p(v)}>0 ∧ ¬C_{p(v)} ∧ Inside ∧ CostOK
     · Normal：市场可正常交易（非停牌/非熔断，背景级前置）。
     · x_{p(v)}>0：父声部有持仓（高级别决定能不能做——父须在场）。
     · ¬C_{p(v)}：父声部未平仓（父仓位仍开放）。
     · Inside：本声部在父声部级别区间套内（**结构定义**，CenterStates `IsWithin`：ZD≤p≤ZG）。
     · CostOK：成本通过 CostOK=1[A≥κ(Fee+Slip+Funding)]——**κ 阈值是 Θ-参数**（EmpiricalDomain），
       作 Θ-谓词接口**不 L0 discharge**（同 StrategyFamily `given_theta_total_unique` /
       ClassifierFamily `thetaFinitePartition` 的 GivenTheta 模式）。

  3. 结构原则「高级别决定能不能做、低级别区间套决定在哪做」：
     · 背景级 b_v 判 ParentValid（父有效性：Normal ∧ x_{p(v)}>0 ∧ ¬C_{p(v)}）——能不能做。
     · 执行级 e_v 判区间套 Inside（χ 落在父级别区间套内）——在哪做。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  本文件**严格分层**（诚实，不冒充 Θ-参数为 L0）：
    · **L0 结构**（纯定义/代数/归纳，不依赖数据，机器可检验）：
      - b_v ≥ o_v ≥ e_v 序良构（传递律给 b_v ≥ e_v）。
      - G_{v,t} 的结构合取**确定性**（给定 Θ-谓词 CostOK 后，G 由各分量唯一确定）。
      - 「高级别决定能否、低级别决定何处」的结构编码（ParentValid / Inside 分离 + G = ParentValid
        ∧ Inside ∧ CostOK 的合取分解）。
      - Inside 从结构定义（CenterStates `IsWithin`，闭区间 ZD≤p≤ZG，纯几何）。
      - 桥接 VoiceTree：三级别 refine 单级别（o_v ↦ VoiceTree.depth，refine 见证）。
    · **Θ-参数**（EmpiricalDomain，**不** L0 discharge）：
      - CostOK 的 κ 阈值（κ·(Fee+Slip+Funding) 的 κ）——作 Θ-谓词参数接口，本文件**不**证
        其取值，**不**证「CostOK 在真实成本数据上成立」（那是 L2+ EmpiricalDomain）。
    **不**声称任何 L1+ 经验有效性。三级别序 + G 合取结构是 L0；κ 阈值数值是 Θ。

  ── 诚实标注（no-patch-mentality / formalization-validity-domain 反声明膨胀）──────
  本文件证**结构层**：三级别 Nat 序良构 + G 结构合取确定性 + 高低级别分工编码 + VoiceTree refine。
  **不**冒充 CostOK 的 κ 阈值为 L0——κ 作 Θ-谓词接口，G 的「给定 CostOK 后确定」是 L0，CostOK
  本身的取值是 Θ。把 Θ-参数当 L0 discharge = 声明膨胀（禁止）。

  ── 依赖方向（单向无环）─────────────────────────────────────────────────────
  VoiceThreeLevel → {Origin.SourceAxioms（Side）, Origin.ChanlunElements（Center/Tick）,
    Origin.CenterStates（IsWithin 结构 Inside）, Origin.VoiceTree（单级别 refine 桥接）}。
  standalone（不 import legacy Strict、不 import #113 分类血肉构造层）。
  验证：`cd formal && lake env lean Origin/VoiceThreeLevel.lean`。禁 sorry/admit/axiom。

  谱系：Strict/Fugue.lean §4（G_v 同源记号，Θ_risk 标注）→ VoiceTree.lean（单级别港入 Origin）→
        本文件（三级别细化 b⪰o⪰e + G_{v,t} 结构层，CostOK Θ-谓词接口）。
-/

import Origin.SourceAxioms
import Origin.ChanlunElements
import Origin.CenterStates
import Origin.VoiceTree

namespace NewChanlun.Origin.VoiceThreeLevel

open NewChanlun.Origin (Center Tick IsWithin)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 三级别声部 (b_v, o_v, e_v)：背景级 ⪰ 操作级 ⪰ 执行级（级别 Nat 序）

  PDF p18-23：声部 v 携带三个级别——背景判断级 b_v（最高）、操作级 o_v（中）、区间套执行级
  e_v（最低）。级别用 Nat 编码（数大=级别高，即更粗的时间框架/更大的趋势级别），序约束
  b_v ≥ o_v ≥ e_v（背景判断在操作之上，操作在区间套执行之上）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★三级别声部 `ThreeLevelVoice`（L0，结构层）：每声部携带 (b_v, o_v, e_v) 三级别 + 序约束。

  - `bg : Nat`：背景判断级 b_v（最高级别，决定「能不能做」的市场背景级别）。
  - `op : Nat`：操作级 o_v（中级别，§6 区间套顶 ℓ_0，决定操作方向）。
  - `ex : Nat`：区间套执行级 e_v（最低级别，§6 区间套底 ℓ_k，决定「在哪做」的买卖点定位级别）。
  - `bg_ge_op`：背景级 ⪰ 操作级（b_v ≥ o_v，背景判断在操作之上）。
  - `op_ge_ex`：操作级 ⪰ 执行级（o_v ≥ e_v，操作在区间套执行之上）。

  ★诚实标注：级别值 (bg, op, ex) 本身是结构参数（哪个级别做背景/操作/执行由配置给定，
  Θ-参数化的级别选择），本结构承载它们 + 序约束以使三级别分工可表达。序约束 b≥o≥e 是 L0
  结构事实（一旦给定级别值，序由 Nat 比较确定）。
-/
structure ThreeLevelVoice where
  bg : Nat
  op : Nat
  ex : Nat
  /-- 背景级 ⪰ 操作级（b_v ≥ o_v）。 -/
  bg_ge_op : bg ≥ op
  /-- 操作级 ⪰ 执行级（o_v ≥ e_v）。 -/
  op_ge_ex : op ≥ ex

/--
  ★三级别序良构（L0，传递律）：背景级 ⪰ 执行级（b_v ≥ e_v）。
  由 `bg_ge_op`（b≥o）+ `op_ge_ex`（o≥e）经 Nat 序传递给出——三级别构成全序链 b⪰o⪰e。
-/
theorem ThreeLevelVoice.bg_ge_ex (v : ThreeLevelVoice) : v.bg ≥ v.ex :=
  Nat.le_trans v.op_ge_ex v.bg_ge_op

/--
  ★三级别链良构（L0，合取形式）：b_v ≥ o_v ∧ o_v ≥ e_v ∧ b_v ≥ e_v（完整序链一次性给出）。
  这是 b⪰o⪰e 区间套序的结构编码——三级别按 Nat 序严格分层（背景在操作之上，操作在执行之上）。
-/
theorem ThreeLevelVoice.chain_wellformed (v : ThreeLevelVoice) :
    v.bg ≥ v.op ∧ v.op ≥ v.ex ∧ v.bg ≥ v.ex :=
  ⟨v.bg_ge_op, v.op_ge_ex, v.bg_ge_ex⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 父子许可 G_{v,t}：结构合取 + CostOK Θ-谓词接口

  PDF p18-23：G_{v,t} = Normal ∧ x_{p(v)}>0 ∧ ¬C_{p(v)} ∧ Inside ∧ CostOK。
  - Normal / x_{p(v)}>0 / ¬C_{p(v)}：父有效性（ParentValid）——背景级 b_v 判，「能不能做」。
  - Inside：本声部在父级别区间套内——执行级 e_v 判，「在哪做」。**结构定义**（CenterStates IsWithin）。
  - CostOK = 1[A≥κ(Fee+Slip+Funding)]：成本通过——**κ 阈值是 Θ-参数**，作 Θ-谓词接口**不 discharge**。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部许可环境 `PermitEnv`（L0，结构层）：承载父子许可 G_{v,t} 各分量的当下判定。

  - `normal : Bool`：市场可正常交易 Normal（背景级前置；非停牌/非熔断）。
  - `parentPos : Int`：父声部持仓量 x_{p(v)}（>0 = 父有仓位；高级别决定能不能做——父须在场）。
  - `parentClosed : Bool`：父声部已平仓 C_{p(v)}（true = 父已关，则不许子开新仓）。
  - `parentCenter : Center`：父声部级别区间套对应的中枢（Inside 判定的几何基准 [ZD, ZG]）。
  - `nestTick : Tick`：本声部当下在父级别区间套中的位置 χ（落入 [ZD,ZG] 内 ⟺ Inside）。

  ★诚实：`normal`/`parentClosed`/`nestTick`/`parentCenter` 的**实时取值**来自数据流（区间套/
  级别套嵌的运行时判定），非 L0 可导；本结构作为给定值承载，G_{v,t} 推导在给定环境下是 L0 逻辑
  必然。CostOK 不在此结构内——它作 Θ-谓词参数显式传入（见 `G`），诚实暴露「成本阈值是 Θ」。
-/
structure PermitEnv where
  normal : Bool
  parentPos : Int
  parentClosed : Bool
  parentCenter : Center
  nestTick : Tick

/--
  ★父有效性 `ParentValid`（L0，背景级 b_v 判——「能不能做」）：
  `Normal ∧ x_{p(v)}>0 ∧ ¬C_{p(v)}`（市场正常 ∧ 父有持仓 ∧ 父未平仓）。

  ★语义：这是「高级别（背景级 b_v）决定**能不能做**」的结构编码——父声部须在场（有仓位）且
  市场背景正常且父未退出，才允许子声部考虑开仓。完全由结构判定（无 Θ-参数），是 L0。
-/
def ParentValid (env : PermitEnv) : Prop :=
  env.normal = true ∧ env.parentPos > 0 ∧ env.parentClosed = false

instance (env : PermitEnv) : Decidable (ParentValid env) := by
  unfold ParentValid; infer_instance

/--
  ★区间套内 `Inside`（L0，执行级 e_v 判——「在哪做」）：本声部当下位置 χ 落在父级别区间套
  （父中枢 [ZD, ZG]）内 —— **结构定义**，复用 CenterStates `IsWithin`（闭区间 ZD ≤ χ ≤ ZG）。

  ★语义：这是「低级别区间套（执行级 e_v）决定**在哪做**」的结构编码——本声部须在父级别区间套
  之内（买卖点定位落在父中枢区间），才是合法的子声部开仓位置。纯几何（Tick 整数闭区间），是 L0。
-/
def Inside (env : PermitEnv) : Prop :=
  IsWithin env.parentCenter env.nestTick

instance (env : PermitEnv) : Decidable (Inside env) := by
  unfold Inside IsWithin; infer_instance

/--
  ★父子许可 `G`（L0 结构合取 + CostOK Θ-谓词参数接口）：
  `G_{v,t} = ParentValid ∧ Inside ∧ CostOK`，其中
    - `ParentValid env`（Normal ∧ x_{p(v)}>0 ∧ ¬C_{p(v)}）：结构判定（背景级，能不能做）。
    - `Inside env`（χ ∈ [ZD,ZG]）：结构判定（执行级区间套，在哪做）。
    - `costOK : Prop`：**Θ-谓词参数**（CostOK=1[A≥κ(Fee+Slip+Funding)]，κ 是 Θ-参数）——
      作显式参数传入，**不**在本文件 discharge。

  ★诚实标注（formalization-validity-domain）：`costOK` 设为 `Prop` 参数 = 诚实暴露「成本通过
  与否依赖 Θ-参数 κ，本文件不产出该判定」。把 costOK 设为参数（同 StrategyFamily piTheta 的
  Θ 参数、ClassifierFamily thetaFinitePartition 的 θ 参数），而**不**内蕴一个 κ 常量假装算了——
  那才是声明膨胀。G 的「给定 costOK 后由三分量合取唯一确定」是 L0；costOK 本身的取值是 Θ。
-/
def G (env : PermitEnv) (costOK : Prop) : Prop :=
  ParentValid env ∧ Inside env ∧ costOK

/--
  ★G 结构合取确定性（L0，核心）：给定 Θ-谓词 `costOK` 后，G 由三分量的合取**唯一确定**。
  `G env costOK ↔ (ParentValid env ∧ Inside env ∧ costOK)`——G 不是别的，恰是三分量合取
  （rfl 级确定性）。这坐实「给定 Θ-谓词接口后，G 的真值由结构分量 + costOK 唯一定」。

  ★语义：一旦 Θ 侧把 costOK 钉死（给定 κ 阈值后 1[A≥κ(…)] 求值），G 的真假由 ParentValid /
  Inside / costOK 三者合取**确定**——无第二种 G。这是 G 作为「结构合取 + Θ-谓词接口」的确定性
  保证（结构层 L0：合取算子确定；Θ 层：costOK 取值由 κ 给定，不在此 discharge）。
-/
theorem G_iff (env : PermitEnv) (costOK : Prop) :
    G env costOK ↔ (ParentValid env ∧ Inside env ∧ costOK) := Iff.rfl

/--
  ★G 的高低级别分工分解（L0，结构原则编码）：
  `G env costOK → ParentValid env ∧ Inside env`——G 成立蕴含「能不能做（ParentValid，背景级）」
  与「在哪做（Inside，执行级区间套）」**同时**成立。

  这是「高级别决定能不能做、低级别区间套决定在哪做」的结构编码：G 把背景级判定（ParentValid）
  与执行级判定（Inside）合取——二者缺一则 G 不成立（高级别否决 ⟹ 不做；区间套外 ⟹ 没位置做）。
-/
theorem G_split (env : PermitEnv) (costOK : Prop) (h : G env costOK) :
    ParentValid env ∧ Inside env :=
  ⟨h.1, h.2.1⟩

/--
  ★高级别否决 ⟹ G 否（L0，「能不能做」的结构必要性）：父无效（背景级否决）⟹ G 不成立。
  `¬ ParentValid env → ¬ G env costOK`——无论区间套内外、无论成本是否通过，高级别（背景级）
  否决即整体许可否决。这编码「高级别决定**能不能做**」：能不能做的否定支配整体。
-/
theorem G_false_of_not_parentValid (env : PermitEnv) (costOK : Prop)
    (h : ¬ ParentValid env) : ¬ G env costOK := by
  intro hG; exact h hG.1

/--
  ★区间套外 ⟹ G 否（L0，「在哪做」的结构必要性）：不在父区间套内（执行级无位置）⟹ G 不成立。
  `¬ Inside env → ¬ G env costOK`——即使高级别许可（能做），若本声部不在父级别区间套内（无合法
  买卖点位置），整体许可仍否决。这编码「低级别区间套决定**在哪做**」：没位置则不开。
-/
theorem G_false_of_not_inside (env : PermitEnv) (costOK : Prop)
    (h : ¬ Inside env) : ¬ G env costOK := by
  intro hG; exact h hG.2.1

/--
  ★G 成立的充分构造（L0）：三分量齐全 ⟹ G 成立。
  `ParentValid env → Inside env → costOK → G env costOK`——背景级许可（能做）∧ 执行级区间套内
  （有位置）∧ 成本通过（Θ 给定 costOK 真）⟹ 父子许可 G 成立。三分量是 G 的充要构成。
-/
theorem G_of_parts (env : PermitEnv) (costOK : Prop)
    (hp : ParentValid env) (hi : Inside env) (hc : costOK) : G env costOK :=
  ⟨hp, hi, hc⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 桥接 VoiceTree：三级别 refine 单级别 ℓ_v

  `Origin/VoiceTree.lean` 的声部树每节点携带**单一** depth（单级别 ℓ_v）。本文件三级别声部
  refine 它：操作级 o_v 对应 VoiceTree 的 depth（操作级是声部树主链的级别），背景级 b_v / 执行级
  e_v 在操作级之上/之下细化。证「三级别 refine 单级别」——三级别坍缩到操作级即恢复 VoiceTree 单级别。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★三级别 refine 单级别的对应 `refineLevel`（L0）：三级别声部的**操作级** o_v 即 VoiceTree 单级别
  ℓ_v 对应的级别——操作级是声部树主链上的级别，背景级（之上）/执行级（之下）是它的三级别细化。

  这给出 refine 的对应映射：单级别 ℓ_v ↦ 三级别 (b_v, o_v=ℓ_v, e_v)，坍缩（取操作级）恢复单级别。
-/
def refineLevel (v : ThreeLevelVoice) : Nat := v.op

/--
  ★三级别 refine 单级别（L0，refine 见证）：三级别声部坍缩到操作级 = 该声部在单级别视角的级别。
  `refineLevel v = v.op`——三级别 (b,o,e) 在单级别 VoiceTree 视角下坍缩为操作级 o_v（rfl）。

  这坐实「三级别是单级别的 refine」：VoiceTree 的 depth/ℓ_v 是三级别声部的操作级分量，三级别
  在操作级之上（背景级）与之下（执行级）增加结构，坍缩（取 op）即恢复 VoiceTree 单级别视角。
-/
theorem refineLevel_eq_op (v : ThreeLevelVoice) : refineLevel v = v.op := rfl

/--
  ★refine 保序（L0）：三级别声部的操作级（= refine 到的单级别）被背景级上界、执行级下界夹住。
  `v.ex ≤ refineLevel v ∧ refineLevel v ≤ v.bg`——单级别 ℓ_v（=操作级）落在三级别区间 [e_v, b_v]
  内。这保证 refine 的单级别与三级别序兼容：三级别细化不破坏单级别的级别定位（操作级居中）。
-/
theorem refineLevel_bounds (v : ThreeLevelVoice) :
    v.ex ≤ refineLevel v ∧ refineLevel v ≤ v.bg :=
  ⟨v.op_ge_ex, v.bg_ge_op⟩

/--
  ★refine 提升单级别声部到三级别（L0，refine 的另一向）：给定 VoiceTree 单级别声部（一个 depth
  级别 `ℓ`），可平凡提升为三级别声部（b=o=e=ℓ，退化区间套——背景=操作=执行同级）。

  这给出 refine 的「单级别 ↪ 三级别」嵌入：单级别声部是三级别声部的退化情形（三级别坍缩成一点）。
  序约束 b≥o≥e 在 b=o=e=ℓ 时平凡满足（自反）。这与 VoiceTree 的单级别视角一致——三级别细化
  是单级别的**严格扩展**（单级别 = 三级别的退化对角线 b=o=e），不丢失单级别可表达的任何声部。
-/
def liftSingleLevel (ℓ : Nat) : ThreeLevelVoice :=
  { bg := ℓ, op := ℓ, ex := ℓ, bg_ge_op := Nat.le_refl ℓ, op_ge_ex := Nat.le_refl ℓ }

/-- ★提升保级别（L0，refine 见证）：单级别 ℓ 提升的三级别声部，其操作级 refine 回 ℓ（rfl）。
    `refineLevel (liftSingleLevel ℓ) = ℓ`——单级别 ↪ 三级别 ↠ 单级别 是恒等（refine 往返保级别）。 -/
theorem refineLevel_liftSingleLevel (ℓ : Nat) : refineLevel (liftSingleLevel ℓ) = ℓ := rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 反退化见证（三级别 + G 真跑通：具体声部 ⟹ 具体许可判定，非平凡）
  ════════════════════════════════════════════════════════════════════════ -/

/-- 三级别声部见证：背景级 5 ⪰ 操作级 3 ⪰ 执行级 1（严格三层，非退化对角线）。 -/
def witVoice : ThreeLevelVoice :=
  { bg := 5, op := 3, ex := 1, bg_ge_op := by decide, op_ge_ex := by decide }

/-- ★反退化见证：三级别序链良构（5 ≥ 3 ≥ 1 且 5 ≥ 1，传递律真跑通）。 -/
theorem witVoice_chain : witVoice.bg ≥ witVoice.op ∧ witVoice.op ≥ witVoice.ex ∧ witVoice.bg ≥ witVoice.ex :=
  witVoice.chain_wellformed

/-- ★反退化见证：refine 到操作级 = 3（三级别坍缩到单级别 ℓ_v=3）。 -/
theorem witVoice_refine : refineLevel witVoice = 3 := rfl

/-- 许可环境见证：市场正常 + 父持仓 10 手 + 父未平仓 + 父中枢 [10,20] + 本声部位置 χ=15（区间套内）。 -/
def witEnv : PermitEnv :=
  { normal := true, parentPos := 10, parentClosed := false,
    parentCenter := { zd := 10, zg := 20, startIndex := 0, endIndex := 1, valid := by decide },
    nestTick := 15 }

/-- ★反退化见证：父有效（市场正常 ∧ 父持仓>0 ∧ 父未平仓）——背景级许可「能做」真跑通。 -/
theorem witEnv_parentValid : ParentValid witEnv := by
  unfold ParentValid witEnv; refine ⟨rfl, ?_, rfl⟩; decide

/-- ★反退化见证：在父区间套内（χ=15 ∈ [10,20]）——执行级区间套「有位置」真跑通。 -/
theorem witEnv_inside : Inside witEnv := by
  unfold Inside witEnv IsWithin; constructor <;> decide

/-- ★反退化见证：父子许可 G 成立（背景级许可 ∧ 区间套内 ∧ Θ 给定 costOK=True 真）。
    costOK 由 Θ 侧给定（此见证取 `True` 代表成本通过的 Θ-判定为真），三分量齐全 ⟹ G 成立。 -/
theorem witEnv_G : G witEnv True :=
  G_of_parts witEnv True witEnv_parentValid witEnv_inside trivial

/-- ★反退化见证：父无效 ⟹ G 否（背景级否决支配整体——「能不能做」否定即整体否）。
    构造父已平仓的环境（parentClosed=true ⟹ ¬ParentValid），G 对任意 costOK 否决。 -/
theorem witEnv_G_false_parentClosed :
    ¬ G { witEnv with parentClosed := true } True := by
  apply G_false_of_not_parentValid
  intro h; exact absurd h.2.2 (by decide)

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★三级别许可标签 `VoiceThreeLevelTag`（gatekeeper，诚实分层）。
  StructuralThreeLevelOnly（b⪰o⪰e 序 + G 结构合取是**结构事实**，非语义分类）+
  ThetaCostThreshold（CostOK 的 κ 阈值是 Θ-参数，**不** L0 discharge）+
  EmpiricalDomain（成本在真实数据上是否通过 = L2+）。
  ★**没有** `TrueCompleteClassification`，**没有** `CostThresholdDischarged` 构造子——
  类型层拒绝把 κ 阈值标为 L0 已证（诚实暴露 Θ-参数边界）。
-/
inductive VoiceThreeLevelTag where
  | StructuralThreeLevelOnly
  | ThetaCostThreshold
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★三级别许可子类（gatekeeper）：ThreeLevelPermitGivenCostTheta（唯一子类，Θ-成本参数化）。 -/
inductive VoiceThreeLevelSubkind where
  | ThreeLevelPermitGivenCostTheta
deriving DecidableEq, Repr

/-- ★三级别许可诚实标签包（L0 声明）。 -/
def voiceThreeLevelLabels : List VoiceThreeLevelTag × VoiceThreeLevelSubkind :=
  ([VoiceThreeLevelTag.StructuralThreeLevelOnly, VoiceThreeLevelTag.ThetaCostThreshold,
    VoiceThreeLevelTag.EmpiricalDomain],
   VoiceThreeLevelSubkind.ThreeLevelPermitGivenCostTheta)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：三级别许可子类必是 ThreeLevelPermitGivenCostTheta。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝冒充 κ 阈值已 L0 discharge（CostOK 是 Θ）。 -/
theorem voiceThreeLevel_costTheta_not_discharged (k : VoiceThreeLevelSubkind) :
    k = VoiceThreeLevelSubkind.ThreeLevelPermitGivenCostTheta := by
  cases k; rfl

end NewChanlun.Origin.VoiceThreeLevel
