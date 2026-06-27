/-
  Origin/RootSelDisambig.lean — 根声部方向选择 + 镜像反对称消歧 + 全局风控平仓
  （port 到 Origin canonical base）

  ★L3 声部根状态机层：RootSel 镜像反对称消歧（strict §9 / FULL 十）+ GlobalRiskClose
    全局风控平仓（strict §11 风险模式 μ_t + §9 根方向递归首触发 / FULL 十三 + 十）。

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**（formal/Origin/）。审计 B 判决策略侧
  四大支柱已港入（VoiceTree/RiskProj/StrategyFamily/ClassifierFamily），但**根声部状态机**
  （RootSel_Θ 根方向选择 + 镜像反对称消歧 + GlobalRiskClose 触发 + 根方向递归 σ̃_{r,t+1}）
  **MISSING in Origin，只在 legacy strict §9/§11**。本文件把它**重锚到 Origin 类型**：复用
  Origin `SourceAxioms` 的 `Side`（long/short）+ Origin `VoiceTree.flip`（赋格交替算子）作根
  方向翻转，定理挂 Origin 命名空间 `NewChanlun.Origin.RootSelDisambig`，**不** import legacy。

  ── 核心命题（strict §9 / FULL 十）────────────────────────────────────────────
  根方向选择函数 `RootSel_Θ : {0,1}² × D_t × ν_t → {-1,0,+1}`（strict §9 line 264）：
    RootSel(1,0)=+1, RootSel(0,1)=-1, RootSel(0,0)=0,  且  RootSel(M_D D) = -RootSel(D).
  **(1,1) 双触发消歧 = 镜像反对称的代数必然**：M_D(1,1)=(1,1) 是镜像不动点 ⟹
    RootSel(1,1) = RootSel(M_D(1,1)) = -RootSel(1,1) ⟹ RootSel(1,1) = 0.
  这不是人为裁决（如"买侧优先"），是严格镜像等变（strict §17 条件 7「多空镜像等变」）强制的
  唯一值——任何非 0 取值都破坏 `RootSel∘M_D = -RootSel`。

  根方向递归 σ̃_{r,t+1}（strict §9 line 278-284 / FULL 十 line 638-654，4 路 case）：
    0,              GlobalRiskClose_t                  （首触发：全局平根仓）
    0,              σ_{r,t}≠0 ∧ χ^{-σ_r}_{r,t}=1       （反向信号：先平后建）
    RootSel(χ⁺,χ⁻), σ_{r,t}=0                          （空仓：按候选选向）
    σ_{r,t},        其他                               （持仓延续）

  GlobalRiskClose_t = (μ_t ∈ {Insolvent, Liquidation})（strict §16 P1「破产或强平」line 535
  + §9 根方向递归首触发）。风险模式 μ_t 五态穷尽互斥（strict §11 line 369-374，
  Lean legacy `risk_mode_complete_unique`）。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数/枚举，不依赖数据）。机器可检验命题：
  - `rootSel_mirror_antisymmetric`：RootSel∘M_D = -RootSel（对全 4 种候选组合）。
  - `rootSel_double_trigger_flat`：RootSel(1,1)=neutral（镜像不动点反对称强制）。
  - `mirror_fixedpoint_forces_neutral`：任何满足反对称的 f 在镜像不动点处 = neutral。
  - `riskMode_complete_unique`：风险模式五态穷尽互斥（Σ𝟙=1）。
  - `globalRiskClose_iff`：GlobalRiskClose ⟺ μ ∈ {Insolvent, Liquidation}。
  - `rootDirNext_*`：根方向递归 4 路 case 的逐 case 取值。
  Lean build 通过 = 这些代数/逻辑命题正确（L0），**不**是「根方向选择在真实市场盈利/最优」
  （那是 L3 EmpiricalDomain，本文件不声称）。

  ── 诚实标注（formalization-validity-domain + no-patch-mentality）────────────
  ★μ_t 依赖账户层运行时输入（E_t/MM_t/B1/B2/LiqFlag），**非缠论或 Θ 可导**——是外部事件
    e_{t+1}/ν_t（strict §1:47 + §9:264）。本文件证「给定这些输入后 μ_t/GlobalRiskClose/σ̃
    唯一确定」，**不**证输入本身来自缠论，也**不**证平仓后必然回本/盈利（L3）。
  ★χ⁺/χ⁻（根触发候选）来自 recog 对 D_t 的折叠；`D_t × ν_t` 在 Θ v0 中不另参与根方向符号
    （ν_t 是执行层借券约束，不改根方向符号）——本文件 RootSel 签名取 {0,1}²，与 Rust
    `voice::root_sel(RootCandidates)` bit-exact 对齐。
  ★禁标 TrueCompleteClassification——根声部状态机是分类下游的操作语义，非分类定理本身。

  ── 依赖方向（单向无环，不 import #113 分类血肉、不 import legacy Strict）──────
  RootSelDisambig → {Origin.SourceAxioms（Side）, Origin.VoiceTree（flip + flip_flip）}。standalone。
  验证：`cd formal && lake env lean Origin/RootSelDisambig.lean`。禁 sorry/admit/axiom。

  谱系：legacy strict §9/§11（根声部状态机）→ #114（VoiceTree/RiskProj 港入 Origin）→
        本文件（L3 RootSel 镜像反对称消歧 + GlobalRiskClose 港入 Origin）。
-/

import Origin.SourceAxioms
import Origin.VoiceTree

namespace NewChanlun.Origin.RootSelDisambig

open NewChanlun.Origin.VoiceTree (flip)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 根方向 RootDir（值域 {-1,0,+1}，含中性 0）

  strict §9 根方向 `σ_r ∈ {-1,0,+1}`（line 258）：比 Origin `Side`（仅 long/short 两态）多一个
  **中性 0**（空仓根）。本文件用三态 `RootDir`（long/short/neutral）承载，long↔+1 / short↔-1 /
  neutral↔0。镜像取负 `RootDir.neg`：long↔short（±1 互换），neutral 不变（0 = -0）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★根方向 `RootDir`（L0，strict §9 `σ_r ∈ {-1,0,+1}`）：三态——long(+1)/short(-1)/neutral(0)。 -/
inductive RootDir where
  | long    -- +1
  | short   -- -1
  | neutral -- 0（空仓根）
deriving DecidableEq, Repr

namespace RootDir

/-- ★根方向取负 `neg`（L0，镜像 +1↔-1，0=-0）：long↔short，neutral 不变。 -/
def neg : RootDir → RootDir
  | long => short
  | short => long
  | neutral => neutral

/-- ★取负对合（L0）：取两次还原（±1 两态对合 + 0 不动）。 -/
theorem neg_neg (d : RootDir) : neg (neg d) = d := by cases d <;> rfl

/-- ★neutral 是取负的唯一不动点（L0）：`neg d = d ↔ d = neutral`。
    long/short 取负改变（±1↔∓1），唯 neutral=0=-0 不变——镜像不动点性质。 -/
theorem neg_fixedpoint_iff (d : RootDir) : neg d = d ↔ d = neutral := by
  cases d <;> simp [neg]

/-- ★从 Origin `Side` 注入 RootDir（L0）：long→long, short→short（保方向，不引入 neutral）。 -/
def ofSide : Side → RootDir
  | Side.long => long
  | Side.short => short

/-- ★注入与翻转/取负相容（L0）：`ofSide (flip s) = neg (ofSide s)`——Side 翻转对应 RootDir 取负。 -/
theorem ofSide_flip (s : Side) : ofSide (flip s) = neg (ofSide s) := by
  cases s <;> rfl

end RootDir

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 根触发候选 (χ⁺,χ⁻) + 镜像算子 M_D

  根候选 `RootCandidates`：χ⁺=买侧触发，χ⁻=卖侧触发（strict §9 `{0,1}²`）。镜像算子 M_D
  作用于候选（strict §7：B_i ↔ S_i ⟹ χ⁺ ↔ χ⁻）：交换买卖触发。镜像对合（M²=id）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★根触发候选 `RootCandidates`（L0，strict §9 `{0,1}²`）：χ⁺=买侧触发 / χ⁻=卖侧触发。 -/
structure RootCandidates where
  longTrigger : Bool   -- χ⁺_{r,t}
  shortTrigger : Bool  -- χ⁻_{r,t}
deriving DecidableEq, Repr

namespace RootCandidates

/-- ★镜像算子 M_D 作用于候选（L0，strict §7 B_i↔S_i）：交换买卖触发 (χ⁺,χ⁻)→(χ⁻,χ⁺)。 -/
def mirror (c : RootCandidates) : RootCandidates :=
  { longTrigger := c.shortTrigger, shortTrigger := c.longTrigger }

/-- ★镜像对合（L0，strict §7 `M²=id`）：镜像两次还原。 -/
theorem mirror_mirror (c : RootCandidates) : mirror (mirror c) = c := by
  cases c; rfl

/-- ★(1,1) 与 (0,0) 是镜像不动点（L0）：买卖触发相等 ⟹ mirror 不变。 -/
theorem mirror_fixed_of_eq (c : RootCandidates) (h : c.longTrigger = c.shortTrigger) :
    mirror c = c := by
  cases c
  simp only [mirror] at *
  simp [h]

/-- ★双触发 (1,1) 是镜像不动点（L0）：`mirror ⟨true,true⟩ = ⟨true,true⟩`。 -/
theorem mirror_double_true : mirror ⟨true, true⟩ = ⟨true, true⟩ := rfl

/-- ★无触发 (0,0) 是镜像不动点（L0）：`mirror ⟨false,false⟩ = ⟨false,false⟩`。 -/
theorem mirror_double_false : mirror ⟨false, false⟩ = ⟨false, false⟩ := rfl

end RootCandidates

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 RootSel_Θ：根方向选择 + (1,1) 镜像反对称消歧

  基础规则（strict §9 line 270-272）：RootSel(1,0)=+1, RootSel(0,1)=-1, RootSel(0,0)=0。
  (1,1) 消歧由镜像反对称公理 `RootSel(M_D D) = -RootSel(D)`（line 273）强制——M_D(1,1)=(1,1)
  是不动点 ⟹ RootSel(1,1)=-RootSel(1,1) ⟹ =0（neutral）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★RootSel_Θ（L0，strict §9 line 264-273）：根方向选择，含 (1,1) 镜像反对称消歧为 neutral。 -/
def rootSel (c : RootCandidates) : RootDir :=
  match c.longTrigger, c.shortTrigger with
  | true, false => RootDir.long      -- RootSel(1,0) = +1
  | false, true => RootDir.short     -- RootSel(0,1) = -1
  | false, false => RootDir.neutral  -- RootSel(0,0) =  0
  | true, true => RootDir.neutral    -- RootSel(1,1) =  0（镜像不动点反对称强制）

/-- ★RootSel 基础规则 (1,0)→long（L0）。 -/
theorem rootSel_long_only : rootSel ⟨true, false⟩ = RootDir.long := rfl

/-- ★RootSel 基础规则 (0,1)→short（L0）。 -/
theorem rootSel_short_only : rootSel ⟨false, true⟩ = RootDir.short := rfl

/-- ★RootSel 基础规则 (0,0)→neutral（L0）。 -/
theorem rootSel_none : rootSel ⟨false, false⟩ = RootDir.neutral := rfl

/--
  ★★★(1,1) 双触发消歧为 neutral（L0，核心交付）★★★：
  `rootSel ⟨true,true⟩ = neutral`。**非买侧优先**——镜像反对称强制唯一值 0。
-/
theorem rootSel_double_trigger_flat : rootSel ⟨true, true⟩ = RootDir.neutral := rfl

/--
  ★★★镜像反对称律（L0，核心交付）★★★：`rootSel (mirror c) = neg (rootSel c)`，
  对**所有** 4 种候选组合成立——即 strict §9 line 273 `RootSel(M_D D) = -RootSel(D)`。
  这是 RootSel 满足 strict §17.7「多空镜像等变」的逐 case 见证。
-/
theorem rootSel_mirror_antisymmetric (c : RootCandidates) :
    rootSel (RootCandidates.mirror c) = RootDir.neg (rootSel c) := by
  obtain ⟨lt, st⟩ := c
  cases lt <;> cases st <;> rfl

/--
  ★★镜像不动点强制中性（L0，反对称消歧的一般定理）：**任何**满足镜像反对称
  `f (mirror c) = neg (f c)` 的根方向函数 f，在镜像不动点 c（mirror c = c）处必为 neutral。
  证：f c = f (mirror c) = neg (f c) ⟹ f c 是 neg 的不动点 ⟹ f c = neutral（`neg_fixedpoint_iff`）。

  这把 (1,1)（与 (0,0)）消歧为 neutral 从「RootSel 的具体定义」提升为「任何镜像等变根选择器的
  代数必然」——消歧不是 RootSel 这一个函数的设计选择，是镜像反对称公理在不动点处的逻辑后承。
-/
theorem mirror_fixedpoint_forces_neutral
    (f : RootCandidates → RootDir)
    (hanti : ∀ c, f (RootCandidates.mirror c) = RootDir.neg (f c))
    (c : RootCandidates) (hfix : RootCandidates.mirror c = c) :
    f c = RootDir.neutral := by
  have hkey : f (RootCandidates.mirror c) = RootDir.neg (f c) := hanti c
  rw [hfix] at hkey
  -- hkey : f c = neg (f c)
  exact (RootDir.neg_fixedpoint_iff (f c)).mp hkey.symm

/--
  ★RootSel 在 (1,1) 处中性 = 镜像不动点强制中性的实例（L0）：把一般定理应用到 RootSel + (1,1)。
  坐实「RootSel(1,1)=0 不是 rootSel 定义的偶然，是 rootSel 满足镜像反对称 ⟹ (1,1) 不动点 ⟹ 必中性」。
-/
theorem rootSel_double_trigger_is_forced :
    rootSel ⟨true, true⟩ = RootDir.neutral :=
  mirror_fixedpoint_forces_neutral rootSel rootSel_mirror_antisymmetric
    ⟨true, true⟩ RootCandidates.mirror_double_true

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 风险模式 μ_t（五态穷尽互斥）+ GlobalRiskClose

  strict §11 line 369-374 风险模式 μ_t ∈ {Insolvent, Liquidation, Deleverage, CloseOnly, Normal}，
  按 M0>M1>M2>M3>M4 优先级穷尽互斥（Σ𝟙=1，legacy `risk_mode_complete_unique`）。
  GlobalRiskClose_t = (μ_t ∈ {Insolvent, Liquidation})（strict §16 P1「破产或强平」+ §9 首触发）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★风险模式 `RiskMode`（L0，strict §11 line 364 / FULL 十三 line 1016-1022）：五态。 -/
inductive RiskMode where
  | insolvent     -- M0：E_t ≤ 0（破产）
  | liquidation   -- M1：LiqFlag ∨ E_t < MM_t（强平）
  | deleverage    -- M2：E_t < MM_t + B1（去杠杆）
  | closeOnly     -- M3：E_t < MM_t + B2（只平仓）
  | normal        -- M4：以上皆否（正常）
deriving DecidableEq, Repr

/--
  ★风险模式输入（L0，strict §11 line 370-373）：账户层运行时量（非缠论可导）。
  - `equity` = E_t、`maintMargin` = MM_t、`buffer1` = B1、`buffer2` = B2（实数）；`liqFlag` = LiqFlag。
  ★诚实标注：全部是外部事件 e_{t+1}/ν_t（strict §1:47），不由缠论/Θ 推导。
-/
structure RiskModeInput where
  equity : Int        -- E_t（用 Int 承载美元 tick；不依赖浮点序避免可判定性问题）
  maintMargin : Int   -- MM_t
  buffer1 : Int       -- B1
  buffer2 : Int       -- B2
  liqFlag : Bool      -- LiqFlag_t

/--
  ★风险模式 μ_t（L0，strict §11 line 369-374，bit-exact 对齐 Rust `risk::risk_mode`）：
  按 M0>M1>M2>M3>M4 短路判定（`if/else` 链实现 `¬M0 ∧ … ∧ ¬M_{i-1}` 前缀）。
-/
def riskMode (x : RiskModeInput) : RiskMode :=
  if x.equity ≤ 0 then RiskMode.insolvent                                  -- M0
  else if x.liqFlag ∨ x.equity < x.maintMargin then RiskMode.liquidation   -- M1
  else if x.equity < x.maintMargin + x.buffer1 then RiskMode.deleverage     -- M2
  else if x.equity < x.maintMargin + x.buffer2 then RiskMode.closeOnly      -- M3
  else RiskMode.normal                                                      -- M4

/--
  ★★风险模式五态穷尽（L0，strict §11 / FULL 十三 line 1061 `Σ𝟙=1`）：riskMode 必返回五态之一。
  函数全定义 ⟹ 必落入某态（穷尽）；返回值是单一 RiskMode ⟹ 互斥（恰一态）。
-/
theorem riskMode_complete (x : RiskModeInput) :
    riskMode x = RiskMode.insolvent ∨ riskMode x = RiskMode.liquidation ∨
    riskMode x = RiskMode.deleverage ∨ riskMode x = RiskMode.closeOnly ∨
    riskMode x = RiskMode.normal := by
  unfold riskMode
  by_cases h0 : x.equity ≤ 0
  · simp [h0]
  · by_cases h1 : x.liqFlag = true ∨ x.equity < x.maintMargin
    · simp [h0, h1]
    · by_cases h2 : x.equity < x.maintMargin + x.buffer1
      · simp [h0, h1, h2]
      · by_cases h3 : x.equity < x.maintMargin + x.buffer2
        · simp [h0, h1, h2, h3]
        · simp [h0, h1, h2, h3]

/-- ★M0 优先（L0）：E_t ≤ 0 ⟹ insolvent（不论 LiqFlag/MM——最高优先级吸收）。 -/
theorem riskMode_insolvent_of_nonpos_equity (x : RiskModeInput) (h : x.equity ≤ 0) :
    riskMode x = RiskMode.insolvent := by
  unfold riskMode; simp [h]

/-- ★GlobalRiskClose_t（L0，strict §9 line 280 / §16 P1）：μ ∈ {insolvent, liquidation}。 -/
def globalRiskClose (m : RiskMode) : Bool :=
  match m with
  | RiskMode.insolvent => true
  | RiskMode.liquidation => true
  | _ => false

/--
  ★★GlobalRiskClose 等价刻画（L0，strict §16 P1「破产或强平」）：
  `globalRiskClose m = true ↔ (m = insolvent ∨ m = liquidation)`。
  Deleverage/CloseOnly/Normal **不**触发全局平仓（它们限增仓不强制平仓，strict §12 `G(q')≤G(q_t)`）。
-/
theorem globalRiskClose_iff (m : RiskMode) :
    globalRiskClose m = true ↔ (m = RiskMode.insolvent ∨ m = RiskMode.liquidation) := by
  cases m <;> simp [globalRiskClose]

/-- ★去杠杆/只平仓/正常不触发全局平仓（L0，strict §16 P1 边界）。 -/
theorem globalRiskClose_false_of_deleverage : globalRiskClose RiskMode.deleverage = false := rfl
theorem globalRiskClose_false_of_closeOnly : globalRiskClose RiskMode.closeOnly = false := rfl
theorem globalRiskClose_false_of_normal : globalRiskClose RiskMode.normal = false := rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 根方向递归 σ̃_{r,t+1}（4 路 case 全定义）

  strict §9 line 278-284 / FULL 十 line 638-654：4 路 case 按优先级短路。
  - case 1：GlobalRiskClose_t ⟹ neutral（全局平根仓，最高优先级）。
  - case 2：持仓 + 反向触发 ⟹ neutral（先平后建，不假设反手）。
  - case 3：空仓 ⟹ RootSel(χ⁺,χ⁻)（按候选选向，含 (1,1) 消歧）。
  - case 4：其他（持仓 + 无反向）⟹ 延续当前方向。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★反向信号判定（L0，strict §9 line 281 `χ^{-σ_r}=1`）：持仓方向遇反向触发。
    long 根遇 χ⁻=1（shortTrigger）；short 根遇 χ⁺=1（longTrigger）；neutral 无反向。 -/
def reverseSignal (current : RootDir) (c : RootCandidates) : Bool :=
  match current with
  | RootDir.long => c.shortTrigger
  | RootDir.short => c.longTrigger
  | RootDir.neutral => false

/--
  ★根方向递归 σ̃_{r,t+1}（L0，strict §9 line 278-284，bit-exact 对齐 Rust `risk::root_dir_next`）：
  4 路 case 短路。
-/
def rootDirNext (m : RiskMode) (current : RootDir) (c : RootCandidates) : RootDir :=
  if globalRiskClose m then RootDir.neutral                            -- case 1
  else if current ≠ RootDir.neutral ∧ reverseSignal current c then
    RootDir.neutral                                                    -- case 2
  else if current = RootDir.neutral then rootSel c                     -- case 3
  else current                                                         -- case 4

/-- ★case 1（L0）：GlobalRiskClose ⟹ neutral（覆盖一切，最高优先级）。 -/
theorem rootDirNext_global_close (m : RiskMode) (current : RootDir) (c : RootCandidates)
    (h : globalRiskClose m = true) :
    rootDirNext m current c = RootDir.neutral := by
  unfold rootDirNext; simp [h]

/-- ★case 2（L0）：非平仓模式 + 持仓 + 反向触发 ⟹ neutral（先平后建）。 -/
theorem rootDirNext_reverse_close (m : RiskMode) (current : RootDir) (c : RootCandidates)
    (hg : globalRiskClose m = false) (hc : current ≠ RootDir.neutral)
    (hr : reverseSignal current c = true) :
    rootDirNext m current c = RootDir.neutral := by
  unfold rootDirNext; simp [hg, hc, hr]

/-- ★case 3（L0）：非平仓 + 空仓 ⟹ RootSel(χ⁺,χ⁻)（按候选选向，含 (1,1) 消歧）。 -/
theorem rootDirNext_neutral_uses_rootSel (m : RiskMode) (c : RootCandidates)
    (hg : globalRiskClose m = false) :
    rootDirNext m RootDir.neutral c = rootSel c := by
  unfold rootDirNext
  simp [hg, reverseSignal]

/-- ★case 4（L0）：非平仓 + 持仓 + 无反向触发 ⟹ 延续当前方向。 -/
theorem rootDirNext_hold (m : RiskMode) (current : RootDir) (c : RootCandidates)
    (hg : globalRiskClose m = false) (hc : current ≠ RootDir.neutral)
    (hr : reverseSignal current c = false) :
    rootDirNext m current c = current := by
  unfold rootDirNext
  simp [hg, hc, hr]

/-- ★根方向递归全定义（L0，strict §9 4-case 穷尽）：rootDirNext 对任意输入有确定值（函数全定义）。 -/
theorem rootDirNext_total (m : RiskMode) (current : RootDir) (c : RootCandidates) :
    ∃ d : RootDir, rootDirNext m current c = d :=
  ⟨rootDirNext m current c, rfl⟩

/--
  ★★空仓遇双触发 (1,1) ⟹ neutral（L0，根方向递归 + RootSel 消歧的合成结论）：
  非平仓模式下，空仓根遇 (χ⁺,χ⁻)=(1,1) ⟹ rootDirNext = RootSel(1,1) = neutral（不开根仓）。
  这把「根方向递归 case 3」与「RootSel (1,1) 镜像反对称消歧」串联——双触发在空仓态唯一消歧为不开。
-/
theorem rootDirNext_neutral_double_trigger (m : RiskMode)
    (hg : globalRiskClose m = false) :
    rootDirNext m RootDir.neutral ⟨true, true⟩ = RootDir.neutral := by
  rw [rootDirNext_neutral_uses_rootSel m ⟨true, true⟩ hg]
  exact rootSel_double_trigger_flat

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★根声部状态机标签 `RootSelTag`（gatekeeper，诚实分层）。
  OperationalSemanticsOnly（根方向选择是分类下游操作语义）+ MirrorEquivariant（镜像反对称消歧
  是结构事实）+ AccountRuntimeParametric（μ_t 依赖账户运行时输入，非缠论可导）+
  EmpiricalDomain（平仓后回本/盈利 L3）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把根声部状态机标为分类定理。
-/
inductive RootSelTag where
  | OperationalSemanticsOnly
  | MirrorEquivariant
  | AccountRuntimeParametric
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★根声部状态机子类（gatekeeper）：MirrorAntisymmetricRootSelector（唯一子类）。 -/
inductive RootSelSubkind where
  | MirrorAntisymmetricRootSelector
deriving DecidableEq, Repr

/-- ★根声部状态机诚实标签包（L0 声明）。 -/
def rootSelLabels : List RootSelTag × RootSelSubkind :=
  ([RootSelTag.OperationalSemanticsOnly, RootSelTag.MirrorEquivariant,
    RootSelTag.AccountRuntimeParametric, RootSelTag.EmpiricalDomain],
   RootSelSubkind.MirrorAntisymmetricRootSelector)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：根声部状态机子类必是 MirrorAntisymmetricRootSelector。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝冒充真完全分类（非跨库强保证）。 -/
theorem rootSel_not_true_classification (k : RootSelSubkind) :
    k = RootSelSubkind.MirrorAntisymmetricRootSelector := by
  cases k; rfl

end NewChanlun.Origin.RootSelDisambig
