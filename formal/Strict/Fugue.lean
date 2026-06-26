/-
  Strict/Fugue.lean — 多重赋格声部树 + 4 互斥动作态 + 核心多头加短空二声部
  （task #75, RTAS 蜂群 cc-fugue 工位；π_Θ 推导第 5-7 步）

  上游：Strict/Classification.lean（StrictAction 等共享内核）+ Strict/StrategyFamily.lean §5
  的 VoiceTree 简版（parent/side/closed/depth + alternating + cascade_close）。
  本文件做**完整动作态推导**——比 StrategyFamily §5 更强：

  1. 声部方向用 σ_v ∈ {+1,-1}（Int），**从 depth 奇偶导出**：σ_v = (-1)^depth（`depth_parity`），
     并证赋格交替 σ_v = -σ_{p(v)}（`alternating`）。根多→子空→孙多…的精确形式。
  2. 父子许可 G_v：根 = MarketNormal；子 = `q_{p(v)}>0 ∧ ¬X_{p(v)} ∧ InsideParent`
     （高级别决定能不能做，低级别区间套决定在哪做）。
  3. 退出 X_v = Emergency ∨ AncestorClose ∨ Stop ∨ χ^{-σ_v}；级联关闭 `cascade_close`：
     父关 ⟹ 所有后代关（对 IsAncestor 传递闭包归纳）。
  4. ★核心定理 `voice_action_exhaustive_exclusive`：4 动作态 {close,open,hold,wait} `Σ𝟙=1`
     （恰一成立）；目标仓位 q̃ = 0(close)/b_v(open)/q_v(hold)/0(wait)。
  5. 二声部特例 `core_long_short_hedge`：根多头 + 子短空，证保留核心多头同时开短差空头/
     平短空/级联关闭三种推导；多空双开 Q⁺>0∧Q⁻>0 = 降低净多头暴露**非转空**。

  ★认识论等级（formalization-validity-domain 强制标注）：
  本文件全部为 **L0**（纯定义/结构层，不依赖数据）。所有定理是从声部树公理 +
  动作态定义推导的**逻辑必然**（穷尽/互斥/级联），lake build 通过 = 这些命题在
  定义内蕴层成立，**不**是任何实盘有效性声明。

  ★诚实标注（no-patch-mentality + 615 gatekeeper）：
  - 仓位大小 `b_v`（开仓手数）/止损价（`Stop` 的触发阈值）是 **Θ_risk 参数**（非缠论可导）——
    本文件把它们作为结构字段/谓词**承载**，不声称其取值由缠论推导。标 `OperationalSemanticsOnly`。
  - 4 动作态的穷尽互斥是 L0 定理（`Σ𝟙=1` 可证）；但「执行某动作能盈利」是 L3（EmpiricalDomain），
    本文件不声称。
  - 优先级链 全局强平 > 祖先关闭 > 本声部止损 > 反向区间套 > 同向区间套 > 保持：
    退出 X_v 把前四项 ∨ 合并为单一「关闭」态，与 open/hold/wait 互斥穷尽——无动作冲突。

  范式：纯 Prop/Type，不依赖 Mathlib（Int 取自 Lean core）。禁 sorry/admit/axiom。

  谱系：StrategyFamily §5（VoiceTree 简版）→ 本文件（depth_parity 强形式 + 4 动作态完整推导）。
        Classification.lean §结构 8（Strategy/StrictAction）→ 本文件（声部级 4 态细化）。
-/

import Strict.Classification

namespace Strict.Fugue

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 声部方向 σ ∈ {+1,-1}：赋格交替 + depth 奇偶
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★方向翻转 `flipDir`（赋格交替的算子）：+1 ↔ -1（用 `Int.neg`）。
  σ = +1 = 多（long），σ = -1 = 空（short）。
-/
def flipDir (σ : Int) : Int := -σ

/-- ★翻转对合（L0）：翻两次还原（方向两态）。 -/
theorem flipDir_flipDir (σ : Int) : flipDir (flipDir σ) = σ := by
  unfold flipDir; exact Int.neg_neg σ

/--
  ★从深度奇偶给方向 `dirOfDepth`（depth_parity 的构造）：
  `dirOfDepth d = (-1)^d`——偶深多（+1），奇深空（-1）。根多→子空→孙多…
-/
def dirOfDepth (d : Nat) : Int :=
  if d % 2 = 0 then 1 else -1

/-- ★方向恒为两态（L0）：`dirOfDepth d = 1 ∨ dirOfDepth d = -1`。 -/
theorem dirOfDepth_two_valued (d : Nat) :
    dirOfDepth d = 1 ∨ dirOfDepth d = -1 := by
  unfold dirOfDepth
  by_cases h : d % 2 = 0
  · left; rw [if_pos h]
  · right; rw [if_neg h]

/--
  ★深度+1 翻转方向（L0，赋格交替的奇偶根据）：`dirOfDepth (d+1) = flipDir (dirOfDepth d)`。
  相邻深度方向相反——这是 σ_v = -σ_{p(v)} 在 depth 严格+1（直接父子）时的算术核心。
-/
theorem dirOfDepth_succ (d : Nat) :
    dirOfDepth (d + 1) = flipDir (dirOfDepth d) := by
  unfold dirOfDepth flipDir
  by_cases h : d % 2 = 0
  · -- d 偶 ⟹ d+1 奇
    have h1 : (d + 1) % 2 = 1 := by omega
    rw [if_pos h, if_neg (by omega : ¬ (d + 1) % 2 = 0)]
  · -- d 奇 ⟹ d+1 偶
    have hd : d % 2 = 1 := by omega
    rw [if_neg h, if_pos (by omega : (d + 1) % 2 = 0)]
    decide

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 声部树 𝒯=(V,p)：有限有根 + 赋格交替 + depth_parity
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部 `Voice`（v=(p(v),σ_v,b_v,o_v,e_v) 的字段载体）+ 声部树 `FugueTree V`（L0，强形式）。

  每个声部 `v : V` 携带：
  - `parent : V → Option V`：父声部（`none` = 根，全树唯一根由 `rooted` 强制）。
  - `depth : V → Nat`：节点深度（根深度 0；直接子深度 = 父深度 + 1，由 `depth_child` 强制）。
  - `σ : V → Int`：声部方向（**从 depth 导出**，由 `depth_parity` 钉死 = `dirOfDepth (depth v)`）。
  - `b : V → Nat`：开仓基准手数 b_v（**Θ_risk 参数**，结构承载，非缠论可导；b_v ≥ 1 由 `b_pos`）。
  - `q : V → Nat`：当前持仓手数 q_v（声部当下仓位，q_v = 0 = 空仓）。

  公理（赋格强形式，比 StrategyFamily §5 多 `rooted`/`depth_child`/`depth_parity`）：
  - `rooted`：**有根** —— 存在唯一根 `root`，`parent root = none`，且唯一 `none` 父者是 root。
  - `depth_root`：根深度 0。
  - `depth_child`：**直接子深度 = 父 + 1**（比 §5 的 `depth p < depth v` 强：严格 +1，编码"级别逐层下降一级"）。
  - `depth_parity`：**σ_v = (-1)^depth_v** = `dirOfDepth (depth v)`（σ 由深度奇偶定，根多偶层多/奇层空）。

  ★诚实：`b`/`q` 是仓位量（Θ_risk + 运行时状态），其**取值**不由缠论推导；
  本结构只承载它们以使动作态推导可表达。`σ`/`depth`/`parent` 是缠论结构（级别套嵌 + 赋格交替）。
-/
structure FugueTree (V : Type) where
  parent : V → Option V
  depth : V → Nat
  σ : V → Int
  b : V → Nat
  q : V → Nat
  root : V
  /-- 有根：root 无父。 -/
  parent_root : parent root = none
  /-- 有根唯一：唯一无父者是 root（全树单根，排除多个不连通局部根）。 -/
  rooted : ∀ v, parent v = none → v = root
  /-- 根深度 0。 -/
  depth_root : depth root = 0
  /-- 直接子深度 = 父深度 + 1（级别逐层降一级；蕴含无环 + 良基 + 父深度 < 子深度）。 -/
  depth_child : ∀ v p, parent v = some p → depth v = depth p + 1
  /-- 赋格 + 奇偶：σ_v = (-1)^depth_v（根多 → 子空 → 孙多…）。 -/
  depth_parity : ∀ v, σ v = dirOfDepth (depth v)
  /-- 开仓基准手数为正（b_v ≥ 1，开仓必有正手数；Θ_risk 参数）。 -/
  b_pos : ∀ v, 1 ≤ b v

/--
  ★父深度严格小于子（L0，由 `depth_child` 直接给）：`parent v = some p → depth p < depth v`。
  这是 StrategyFamily §5 `depth_decreasing` 的导出版（本文件的 `depth_child` 更强）。
-/
theorem FugueTree.depth_decreasing {V : Type} (T : FugueTree V)
    (v p : V) (hpar : T.parent v = some p) :
    T.depth p < T.depth v := by
  rw [T.depth_child v p hpar]; omega

/--
  ★★赋格交替 `alternating`（L0，核心，σ_v = -σ_{p(v)}）：
  `parent v = some p → σ v = flipDir (σ p)`——子声部方向 = 父声部翻转。

  证明：σ_v = (-1)^depth_v（depth_parity）；depth_v = depth_p + 1（depth_child）；
  (-1)^(depth_p+1) = flip((-1)^depth_p)（dirOfDepth_succ）= flip σ_p（depth_parity）。
-/
theorem FugueTree.alternating {V : Type} (T : FugueTree V)
    (v p : V) (hpar : T.parent v = some p) :
    T.σ v = flipDir (T.σ p) := by
  rw [T.depth_parity v, T.depth_parity p, T.depth_child v p hpar, dirOfDepth_succ]

/--
  ★相邻声部反向（L0，赋格交替推论）：`parent v = some p → σ v ≠ σ p`。
  σ_v = -σ_p 且 σ_p ∈ {+1,-1}，故 σ_v ≠ σ_p（两态翻转无不动点）。
-/
theorem FugueTree.adjacent_opposite {V : Type} (T : FugueTree V)
    (v p : V) (hpar : T.parent v = some p) :
    T.σ v ≠ T.σ p := by
  rw [T.alternating v p hpar]
  -- flipDir σp = -σp ≠ σp，因 σp ∈ {+1,-1} 非零
  rcases T.depth_parity p ▸ dirOfDepth_two_valued (T.depth p) with hp | hp
  · rw [hp]; unfold flipDir; decide
  · rw [hp]; unfold flipDir; decide

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 祖先关系 + 级联关闭
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★祖先关系 `IsAncestor`（L0）：`IsAncestor T a v` ⟺ a 经 ≥1 步 parent 链可达 v。
  归纳定义：直接父是祖先（base）；祖先的祖先是祖先（step）。
-/
inductive IsAncestor {V : Type} (T : FugueTree V) : V → V → Prop where
  | base : ∀ v p, T.parent v = some p → IsAncestor T p v
  | step : ∀ v p a, T.parent v = some p → IsAncestor T a p → IsAncestor T a v

/--
  ★祖先深度严格更小（L0，无环良基）：`IsAncestor T a v → depth a < depth v`。
  从 `depth_decreasing` 沿祖先链累积——保证 IsAncestor 无环。
-/
theorem FugueTree.ancestor_depth_lt {V : Type} (T : FugueTree V)
    (a v : V) (hanc : IsAncestor T a v) :
    T.depth a < T.depth v := by
  induction hanc with
  | base v p hpar => exact T.depth_decreasing v p hpar
  | step v p a hpar _hanc ih => exact Nat.lt_trans ih (T.depth_decreasing v p hpar)

/--
  ★声部不是自己的祖先（L0，无环见证）：`¬ IsAncestor T v v`。
  否则 `depth v < depth v`（`ancestor_depth_lt`），矛盾。
-/
theorem FugueTree.not_self_ancestor {V : Type} (T : FugueTree V) (v : V) :
    ¬ IsAncestor T v v := by
  intro h
  exact Nat.lt_irrefl (T.depth v) (T.ancestor_depth_lt v v h)

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 退出条件 X_v + 父子许可 G_v + 进场许可 E_v
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部动态环境 `VoiceEnv V`（L0）：承载每个声部在某一时刻 t 的**布尔判定**。

  每个声部 v 的退出/进场依赖以下原子谓词（均为运行时可判定的 Bool）：
  - `emergency : V → Bool`：全局强平信号（Emergency，最高优先级，对全树一致）。
  - `stop : V → Bool`：本声部止损触发（χ 止损价穿越；阈值是 Θ_risk 参数）。
  - `reverseNest : V → Bool`：反向区间套确认 χ^{-σ_v}（次级别出现反向方向的区间套 ⟹ 平本声部）。
  - `sameNest : V → Bool`：同向区间套确认 χ^{+σ_v}（次级别同向区间套 ⟹ 可在此开子声部）。
  - `marketNormal : Bool`：市场可正常交易（根声部许可的前置；非停牌/非熔断）。
  - `insideParent : V → Bool`：本声部处于父声部的级别区间套内（InsideParent，低级别"在哪做"）。

  ★诚实：`stop` 的阈值、`b` 的手数是 Θ_risk；`reverseNest`/`sameNest`/`insideParent`/
  `marketNormal` 是缠论结构判定（区间套/级别套嵌），但其**实时取值**来自数据流，非 L0 可导。
  本结构把它们作为给定 Bool 承载，动作态推导在给定环境下是 L0 逻辑必然。
-/
structure VoiceEnv (V : Type) where
  emergency : V → Bool
  stop : V → Bool
  reverseNest : V → Bool
  sameNest : V → Bool
  marketNormal : Bool
  insideParent : V → Bool

/--
  ★祖先关闭信号 `ancestorClosing`（L0）：是否存在某祖先 a 触发了 emergency 或 stop。
  这里以"祖先 a 的 emergency 或 stop 为真"作为祖先关闭的**触发源**——
  级联关闭定理把祖先的这两类退出源沿后代传播（见 `exit_cascade`）。

  注：用 `∃ a, IsAncestor T a v ∧ (emergency a ∨ stop a)` 作 Prop 判定。

  ★★能力边界（codex 异质审计修订，no-patch §5 反声明膨胀）：
  `ancestorClosing` **只**捕获祖先的 emergency（全局强平）与 stop（止损）两类退出源，
  **不**捕获祖先因 `reverseNest`（反向区间套止盈）平仓的级联。故级联定理 `exit_cascade`
  的精确语义是「父因 emergency/stop 关 ⟹ 后代关」，**不是**无条件的「父关 ⟹ 所有后代关」。

  为何不把 reverseNest 纳入级联（诚实留白，非回避）：父声部因止盈（反向区间套）平仓后，
  子对冲声部是否应级联平掉，是**有缠论实质内容的语义判断**（止盈父仓 ≠ 风险事件，
  对冲子仓可能仍需保留或独立了结）——这超出本 L0 结构工位的裁定范围，属 Θ_risk/缠论
  操作语义。本文件**不**默认它级联，把判断显式留给下游，而非用更宽的级联定义掩盖空白。
  注：父平仓使 `q p = 0`，子声部的 `Permit`（要求 `q p > 0`）自动失效——子无法再开新仓，
  但已有子仓的了结由子自身的 Exit 条件决定，不经 ancestorClosing 强制级联。
-/
def ancestorClosing {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  ∃ a, IsAncestor T a v ∧ (env.emergency a = true ∨ env.stop a = true)

/--
  ★★退出条件 X_v（L0，优先级链的 ∨ 合并）：
  `Exit T env v` := Emergency ∨ AncestorClose ∨ Stop ∨ χ^{-σ_v}（反向区间套）。

  四个析取项对应优先级链的前四级（全局强平 > 祖先关闭 > 本声部止损 > 反向区间套）——
  ∨ 合并为单一「关闭」态，使 close 与 open/hold/wait 互斥穷尽（无优先级冲突）。
  同向区间套 χ^{+σ_v}（`sameNest`）**不**进退出（它触发开子声部，不平本声部）。
-/
def Exit {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  env.emergency v = true ∨ ancestorClosing T env v ∨
    env.stop v = true ∨ env.reverseNest v = true

/--
  ★父子许可 G_v（L0，高级别决定能不能做）：
  - 根：`marketNormal = true`（市场可正常交易）。
  - 非根（parent v = some p）：`q p > 0 ∧ ¬ Exit p ∧ insideParent v`
    （父声部有持仓 + 父未退出 + 本声部在父级别区间套内）。

  ★语义：高级别（父）决定**能不能做**（父须有仓位且未退出），低级别区间套（insideParent）
  决定**在哪做**。这是缠论"大级别定方向，小级别定买卖点"的结构形式。
-/
def Permit {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  match T.parent v with
  | none => env.marketNormal = true
  | some p => T.q p > 0 ∧ ¬ Exit T env p ∧ env.insideParent v = true

/--
  ★进场许可 E_v（L0，可以开仓的总条件）：`Permit T env v ∧ sameNest v`。
  父子许可成立（能做）+ 本级别同向区间套确认（出现该方向买卖点）⟹ 可开本声部。
  这是 open 动作态的进场门（区别 hold：hold 是已有仓位继续持，open 是从空仓建仓）。
-/
def EnterOK {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  Permit T env v ∧ env.sameNest v = true

/--
  ★★退出级联（L0，核心）：祖先 a 触发退出源（emergency/stop）⟹ 后代 v 退出。
  `IsAncestor T a v → (emergency a ∨ stop a) → Exit T env v`。

  证明：a 是 v 的祖先且 a 触发 emergency/stop ⟹ `ancestorClosing T env v` 成立
  ⟹ Exit 的第二析取项成立。这是"父因 emergency/stop 关 ⟹ 后代关"的精确传递形式
  （对 IsAncestor 的传递闭包——IsAncestor 本身已编码任意深祖先）。

  ★能力边界（见 `ancestorClosing` 注释）：仅 emergency/stop 两类祖先退出源经此级联；
  祖先因 reverseNest（止盈）平仓**不**触发本定理（诚实留白，非"父关⟹所有后代关"的无条件版）。
-/
theorem exit_cascade {V : Type} (T : FugueTree V) (env : VoiceEnv V)
    (a v : V) (hanc : IsAncestor T a v)
    (htrig : env.emergency a = true ∨ env.stop a = true) :
    Exit T env v := by
  right; left
  exact ⟨a, hanc, htrig⟩

/--
  ★全局强平级联（L0，特例）：根触发 emergency ⟹ 所有后代退出。
  根 emergency = 全局强平信号——对每个以 root 为祖先的声部 v，Exit 成立。
-/
theorem emergency_cascade_from_root {V : Type} (T : FugueTree V) (env : VoiceEnv V)
    (v : V) (hanc : IsAncestor T T.root v) (hemg : env.emergency T.root = true) :
    Exit T env v :=
  exit_cascade T env T.root v hanc (Or.inl hemg)

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 ★核心：4 互斥动作态 {close, open, hold, wait} —— Σ𝟙 = 1
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★4 动作态枚举 `ActState`（L0）：close / open / hold / wait。
  对应 Classification.lean 的 `StrictAction`（这里聚焦声部级 4 态分支，见 `toStrict`）。
-/
inductive ActState where
  | close  -- 平仓（退出）
  | open   -- 开仓（从空仓建仓 b_v）
  | hold   -- 持仓（已有仓位继续持有 q_v）
  | wait   -- 等待（空仓观望）
deriving DecidableEq, Repr

/--
  ★动作态定义（4 个互斥 Prop，L0）：
  - `close`  := Exit v                                  （退出优先，吸收前四优先级）
  - `open`   := ¬Exit ∧ q_v = 0 ∧ EnterOK v             （未退出 + 空仓 + 进场许可）
  - `hold`   := ¬Exit ∧ q_v > 0                          （未退出 + 已有仓位）
  - `wait`   := ¬Exit ∧ q_v = 0 ∧ ¬EnterOK v            （未退出 + 空仓 + 无进场许可）

  ★注意 open 与 wait 都要求 q_v = 0（空仓），用 EnterOK 区分；hold 要求 q_v > 0。
  4 态对 (Exit, q_v=0?, EnterOK?) 三元判定做完全划分——见 `voice_action_exhaustive_exclusive`。
-/
def isClose {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  Exit T env v

def isOpen {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  ¬ Exit T env v ∧ T.q v = 0 ∧ EnterOK T env v

def isHold {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  ¬ Exit T env v ∧ T.q v > 0

def isWait {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) : Prop :=
  ¬ Exit T env v ∧ T.q v = 0 ∧ ¬ EnterOK T env v

/--
  ★选出当前动作态 `actState`（L0，可判定全函数）：基于 (Exit, q=0, EnterOK) 三元分类。
  需要 Exit / EnterOK 的可判定实例——这里以 Classical 决定（纯 Prop 层，不影响 L0 证明）。
-/
noncomputable def actState {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) :
    ActState := by
  classical
  exact
    if Exit T env v then ActState.close
    else if T.q v = 0 then
      (if EnterOK T env v then ActState.open else ActState.wait)
    else ActState.hold

/--
  ★★★核心定理 `voice_action_exhaustive_exclusive`（L0，Σ𝟙 = 1）：
  对每个声部 v，4 动作态 {close, open, hold, wait} **恰一成立**——

  穷尽（∨）：`isClose ∨ isOpen ∨ isHold ∨ isWait`；
  互斥（两两 ¬∧）：任两个不同态不同时成立。

  这把"完全应对策略"在声部级落实为带证明的穷尽互斥划分（无优先级冲突、无未定义动作）。
  对应优先级链：close 吸收前四级（全局强平/祖先关闭/止损/反向区间套）；
  其余三态在"未退出"分支内按 (空仓?, 进场许可?) 二次划分。
-/
theorem voice_action_exhaustive_exclusive {V : Type} (T : FugueTree V)
    (env : VoiceEnv V) (v : V) :
    -- 穷尽
    (isClose T env v ∨ isOpen T env v ∨ isHold T env v ∨ isWait T env v) ∧
    -- 互斥（6 对）
    (¬ (isClose T env v ∧ isOpen T env v)) ∧
    (¬ (isClose T env v ∧ isHold T env v)) ∧
    (¬ (isClose T env v ∧ isWait T env v)) ∧
    (¬ (isOpen  T env v ∧ isHold T env v)) ∧
    (¬ (isOpen  T env v ∧ isWait T env v)) ∧
    (¬ (isHold  T env v ∧ isWait T env v)) := by
  classical
  unfold isClose isOpen isHold isWait
  by_cases hX : Exit T env v
  · -- close 成立；其余三态均含 ¬Exit，与 hX 矛盾
    refine ⟨Or.inl hX, ?_, ?_, ?_, ?_, ?_, ?_⟩
    · rintro ⟨_, hnX, _⟩; exact hnX hX
    · rintro ⟨_, hnX, _⟩; exact hnX hX
    · rintro ⟨_, hnX, _⟩; exact hnX hX
    · rintro ⟨⟨hnX, _⟩, _⟩; exact hnX hX
    · rintro ⟨⟨hnX, _⟩, _⟩; exact hnX hX
    · rintro ⟨⟨hnX, _⟩, _⟩; exact hnX hX
  · -- ¬Exit：按 (q=0?, EnterOK?) 划分 open/hold/wait
    by_cases hq : T.q v = 0
    · by_cases hE : EnterOK T env v
      · -- open
        refine ⟨Or.inr (Or.inl ⟨hX, hq, hE⟩), ?_, ?_, ?_, ?_, ?_, ?_⟩
        · rintro ⟨hXc, _⟩; exact hX hXc
        · rintro ⟨hXc, _⟩; exact hX hXc
        · rintro ⟨hXc, _⟩; exact hX hXc
        · rintro ⟨⟨_, hqz, _⟩, ⟨_, hqp⟩⟩; omega   -- q=0 ∧ q>0 矛盾
        · rintro ⟨⟨_, _, hEo⟩, ⟨_, _, hEw⟩⟩; exact hEw hEo  -- EnterOK ∧ ¬EnterOK
        · rintro ⟨⟨_, hqp⟩, ⟨_, hqz, _⟩⟩; omega
      · -- wait
        refine ⟨Or.inr (Or.inr (Or.inr ⟨hX, hq, hE⟩)), ?_, ?_, ?_, ?_, ?_, ?_⟩
        · rintro ⟨hXc, _⟩; exact hX hXc
        · rintro ⟨hXc, _⟩; exact hX hXc
        · rintro ⟨hXc, _⟩; exact hX hXc
        · rintro ⟨⟨_, hqz, _⟩, ⟨_, hqp⟩⟩; omega
        · rintro ⟨⟨_, _, hEo⟩, ⟨_, _, hEw⟩⟩; exact hEw hEo
        · rintro ⟨⟨_, hqp⟩, ⟨_, hqz, _⟩⟩; omega
    · -- q > 0：hold
      have hqp : T.q v > 0 := Nat.pos_of_ne_zero hq
      refine ⟨Or.inr (Or.inr (Or.inl ⟨hX, hqp⟩)), ?_, ?_, ?_, ?_, ?_, ?_⟩
      · rintro ⟨hXc, _⟩; exact hX hXc
      · rintro ⟨hXc, _⟩; exact hX hXc
      · rintro ⟨hXc, _⟩; exact hX hXc
      · rintro ⟨⟨_, hqz, _⟩, ⟨_, hqp'⟩⟩; omega
      · rintro ⟨⟨_, hqz, _⟩, ⟨_, hqz', _⟩⟩; omega
      · rintro ⟨⟨_, hqp'⟩, ⟨_, hqz, _⟩⟩; omega

/--
  ★`actState` 与谓词一致（L0，sound）：`actState` 选出的态正是成立的那个动作态。
  这把可判定全函数 `actState` 与 4 个谓词 `isClose/isOpen/isHold/isWait` 对齐。
-/
theorem actState_sound {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) :
    (actState T env v = ActState.close ↔ isClose T env v) ∧
    (actState T env v = ActState.open  ↔ isOpen  T env v) ∧
    (actState T env v = ActState.hold  ↔ isHold  T env v) ∧
    (actState T env v = ActState.wait  ↔ isWait  T env v) := by
  classical
  -- 只对 actState（LHS）做条件归约；isClose/.. 谓词保持不展开（避免 simp 把命题改写成 True/False）
  have hAct : actState T env v =
      (if Exit T env v then ActState.close
       else if T.q v = 0 then
         (if EnterOK T env v then ActState.open else ActState.wait)
       else ActState.hold) := rfl
  by_cases hX : Exit T env v
  · rw [hAct, if_pos hX]
    refine ⟨?_, ?_, ?_, ?_⟩
    · exact ⟨fun _ => hX, fun _ => rfl⟩
    · exact ⟨fun h => absurd h (by decide), fun ho => absurd hX ho.1⟩
    · exact ⟨fun h => absurd h (by decide), fun hh => absurd hX hh.1⟩
    · exact ⟨fun h => absurd h (by decide), fun hw => absurd hX hw.1⟩
  · by_cases hq : T.q v = 0
    · by_cases hE : EnterOK T env v
      · rw [hAct, if_neg hX, if_pos hq, if_pos hE]
        refine ⟨?_, ?_, ?_, ?_⟩
        · exact ⟨fun h => absurd h (by decide), fun hc => absurd hc hX⟩
        · exact ⟨fun _ => ⟨hX, hq, hE⟩, fun _ => rfl⟩
        · exact ⟨fun h => absurd h (by decide), fun hh => by have h2 := hh.2; omega⟩
        · exact ⟨fun h => absurd h (by decide), fun hw => absurd hE hw.2.2⟩
      · rw [hAct, if_neg hX, if_pos hq, if_neg hE]
        refine ⟨?_, ?_, ?_, ?_⟩
        · exact ⟨fun h => absurd h (by decide), fun hc => absurd hc hX⟩
        · exact ⟨fun h => absurd h (by decide), fun ho => absurd ho.2.2 hE⟩
        · exact ⟨fun h => absurd h (by decide), fun hh => by have h2 := hh.2; omega⟩
        · exact ⟨fun _ => ⟨hX, hq, hE⟩, fun _ => rfl⟩
    · have hqp : T.q v > 0 := Nat.pos_of_ne_zero hq
      rw [hAct, if_neg hX, if_neg hq]
      refine ⟨?_, ?_, ?_, ?_⟩
      · exact ⟨fun h => absurd h (by decide), fun hc => absurd hc hX⟩
      · exact ⟨fun h => absurd h (by decide), fun ho => absurd ho.2.1 hq⟩
      · exact ⟨fun _ => ⟨hX, hqp⟩, fun _ => rfl⟩
      · exact ⟨fun h => absurd h (by decide), fun hw => absurd hw.2.1 hq⟩

/--
  ★目标仓位 `targetPos`（L0）：q̃_{v,t+1} = 0(close)/b_v(open)/q_v(hold)/0(wait)。
  与 4 动作态一一对应——close/wait 归零，open 建基准手数 b_v，hold 维持当前 q_v。
-/
noncomputable def targetPos {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) :
    Nat :=
  match actState T env v with
  | ActState.close => 0
  | ActState.open  => T.b v
  | ActState.hold  => T.q v
  | ActState.wait  => 0

/--
  ★目标仓位与动作态一致（L0）：列出 4 态下 targetPos 的取值。
  close → 0，open → b_v，hold → q_v，wait → 0。
-/
theorem targetPos_spec {V : Type} (T : FugueTree V) (env : VoiceEnv V) (v : V) :
    (isClose T env v → targetPos T env v = 0) ∧
    (isOpen  T env v → targetPos T env v = T.b v) ∧
    (isHold  T env v → targetPos T env v = T.q v) ∧
    (isWait  T env v → targetPos T env v = 0) := by
  classical
  obtain ⟨hc, ho, hh, hw⟩ := actState_sound T env v
  unfold targetPos
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro h; rw [hc.mpr h]
  · intro h; rw [ho.mpr h]
  · intro h; rw [hh.mpr h]
  · intro h; rw [hw.mpr h]

/--
  ★声部 4 态 → StrictAction（L0，对齐 Classification.lean §结构 8）：
  close → `StrictAction.close`，open → `buy`/`sell`（按 σ_v 方向），
  hold → `StrictAction.hold`，wait → `StrictAction.wait`。

  ★注：open 的买/卖由声部方向 σ_v 决定（σ=+1 多 → buy，σ=-1 空 → sell）——
  这是声部级动作到标准七动作集的方向化映射。add/reduce（加减仓）是 b_v 调整的细分，
  本文件聚焦 4 主态（开/平/持/等），不展开加减仓（属 Θ_risk 的仓位管理细节）。
-/
def toStrict {V : Type} (T : FugueTree V) (a : ActState) (v : V) : StrictAction :=
  match a with
  | ActState.close => StrictAction.close
  | ActState.open  => if T.σ v = 1 then StrictAction.buy else StrictAction.sell
  | ActState.hold  => StrictAction.hold
  | ActState.wait  => StrictAction.wait

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 ★二声部特例：核心多头 + 子短空（core_long_short_hedge）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★二声部赋格 `TwoVoiceHedge`（L0）：恰两个声部——
  根 `r`（σ=+1 多头，主级别核心多头）+ 子 `s`（σ=-1 短空，次级别对冲，parent s = some r）。

  字段是对 `FugueTree Bool` 的实例约束封装（V = Bool：true=r 根多, false=s 子空）：
  - `T : FugueTree Bool`，`r s : Bool`，`r = true`，`s = false`，`parent s = some r`，
    `parent r = none`（r 是根）。
  这把"核心多头 + 子短空二声部"钉为具体可推导对象。
-/
structure TwoVoiceHedge where
  T : FugueTree Bool
  r : Bool
  s : Bool
  hr : r = true
  hs : s = false
  hrs : T.parent s = some r
  hroot : T.parent r = none

/-- ★根方向为多（L0）：σ_r = +1（根深度 0，dirOfDepth 0 = 1）。 -/
theorem TwoVoiceHedge.root_long (H : TwoVoiceHedge) : H.T.σ H.r = 1 := by
  rw [H.T.depth_parity H.r]
  have : H.T.depth H.r = 0 := by
    -- r 是根：parent r = none ⟹ r = root ⟹ depth r = 0
    have := H.T.rooted H.r H.hroot
    rw [this, H.T.depth_root]
  rw [this]; rfl

/-- ★子方向为空（L0，赋格交替）：σ_s = -1（= flipDir σ_r = -(+1)）。 -/
theorem TwoVoiceHedge.child_short (H : TwoVoiceHedge) : H.T.σ H.s = -1 := by
  rw [H.T.alternating H.s H.r H.hrs, H.root_long]; rfl

/--
  ★★保留核心多头同时开短差空头（L0，core_long_short_hedge 第一推导）：

  前提（高级别多头成立 + 低级别卖区间套确认）：
  - 根多头有持仓且未退出：`q r > 0`、`¬ Exit r`；
  - 子声部空仓 + 未退出：`q s = 0`、`¬ Exit s`；
  - 子声部进场许可成立：`EnterOK s`（= 父子许可 + 子方向同向区间套 `sameNest s`；
    注意子 σ_s = -1，子的"同向区间套"= 卖方向区间套 χ^{-σ_r} = 对根而言的反向，
    即"低级别卖区间套确认"）。
  结论：
  - 根声部动作态 = hold（保留核心多头）；
  - 子声部动作态 = open（开短差空头）。

  ⟹ 多空双开：根 hold（Q⁺=q_r>0 保留）+ 子 open（Q⁻=b_s>0 建立）。
  这是"保留核心多头同时开短差空头"的精确结构形式。

  ★为何 `¬ Exit s` 是显式前提（no-patch-mentality，非 workaround）：
  `isOpen s` 按 §5 定义本就含 `¬ Exit s`；而 `EnterOK s`（Permit ∧ sameNest）**不蕴含**
  `¬ Exit s`——Exit（emergency/止损/反向区间套）与 Permit（父有仓+父未退出+在父区间内）是
  **独立判定**。开短空声部时，该声部自身不处于退出态是开仓的合法前提，必须显式声明，
  不能从进场许可偷渡。这与核心定理 `voice_action_exhaustive_exclusive` 的 open 分支一致。
-/
theorem core_long_short_hedge_open (H : TwoVoiceHedge)
    (env : VoiceEnv Bool)
    (hqr : H.T.q H.r > 0) (hXr : ¬ Exit H.T env H.r)
    (hqs : H.T.q H.s = 0) (hXs : ¬ Exit H.T env H.s)
    (hEs : EnterOK H.T env H.s) :
    isHold H.T env H.r ∧ isOpen H.T env H.s :=
  ⟨⟨hXr, hqr⟩, ⟨hXs, hqs, hEs⟩⟩

/--
  ★★平短空（L0，core_long_short_hedge 第二推导）：

  前提（低级别买区间套确认 ⟹ 平短空）：
  - 根多头仍持仓未退出：`q r > 0`、`¬ Exit r`（核心多头不动）；
  - 子短空有持仓 `q s > 0`；
  - 子声部触发退出：`Exit s`——子方向 σ_s = -1，对子而言"买区间套确认"= χ^{-σ_s} = χ^{+1}
    （根方向同向的区间套）触发子的**反向区间套** `reverseNest s`，落入子 Exit。
  结论：
  - 根声部动作态 = hold（保留核心多头不变）；
  - 子声部动作态 = close（平短空）。

  ⟹ 平掉对冲空头、保留核心多头。这是"低级别买区间套确认 ⟹ 平短空"的精确结构形式。
  ★注：子的 Exit 由 `reverseNest s = true`（买区间套对空头是反向）给出——这正是退出 X_v
  第四析取项 χ^{-σ_v}。本定理以 `Exit s` 为前提（不论由哪个析取项触发），结论 close 成立。
-/
theorem core_long_short_hedge_close (H : TwoVoiceHedge)
    (env : VoiceEnv Bool)
    (hqr : H.T.q H.r > 0) (hXr : ¬ Exit H.T env H.r)
    (hXs : Exit H.T env H.s) :
    isHold H.T env H.r ∧ isClose H.T env H.s :=
  ⟨⟨hXr, hqr⟩, hXs⟩

/--
  ★★高级别失效 ⟹ 先关子后关根（L0，core_long_short_hedge 第三推导，级联关闭）：

  前提（根触发退出源 emergency 或 stop）：`env.emergency r = true ∨ env.stop r = true`。
  结论：
  - 子声部 close（先关子）：根触发退出源 ⟹ 子 `Exit s`（`exit_cascade`，根是子的祖先）；
  - 根声部 close（后关根）：根自身 `Exit r`（emergency/stop 落入根 Exit 第一/第三析取项）。

  ★"先关子后关根"的结构含义：子的关闭**经由根的退出源级联**触发（`ancestorClosing s` 成立，
  因 root 是 s 的祖先且 root 触发 emergency/stop）——即子的平仓**依赖**根的失效信号，
  逻辑上根失效是因、子关闭是果。两者最终都 close，但子的 close 由级联导出，根的 close 由自身导出。
  这与操盘"高级别多头失效时，先平掉低级别对冲，再处理核心仓"的顺序一致。
-/
theorem core_long_short_hedge_cascade_close (H : TwoVoiceHedge)
    (env : VoiceEnv Bool)
    (htrig : env.emergency H.r = true ∨ env.stop H.r = true) :
    isClose H.T env H.s ∧ isClose H.T env H.r := by
  constructor
  · -- 子 close：根是子的直接父（IsAncestor.base），级联触发子 Exit
    have hanc : IsAncestor H.T H.r H.s := IsAncestor.base H.s H.r H.hrs
    exact exit_cascade H.T env H.r H.s hanc htrig
  · -- 根 close：根自身 Exit（emergency 第一析取项 / stop 第三析取项）
    show Exit H.T env H.r
    rcases htrig with hemg | hstop
    · exact Or.inl hemg
    · exact Or.inr (Or.inr (Or.inl hstop))

/--
  ★多空双开 = 降低净多头暴露，非转空（L0，core_long_short_hedge 净敞口语义）：

  当根 hold（持仓 q_r，σ_r=+1 多）+ 子 open（建仓 b_s，σ_s=-1 空）时，
  净敞口 = Q⁺ - Q⁻ = q_r - b_s（多头手数 - 空头手数）。多空双开 `q_r > 0 ∧ b_s > 0`
  **不**等于转空——只要 `q_r ≥ b_s`，净敞口 ≥ 0（仍是净多或中性），是**降低净多头暴露**。

  本定理证：方向相反（σ_r = +1 ≠ σ_s = -1，赋格交替保证）且双方均正手数（双开），
  净敞口 `(q_r : Int) - b_s` 在 `q_r ≥ b_s` 时 ≥ 0——双开降低而非反转净多头。

  ★`_hqr`/`_hbs`（q_r>0 / b_s>0）是"双开"的**陈述层语义前提**（双开 = 两边均正手数），
  下划线标记证明层未直接引用（结论 `q_r ≥ b_s` 已足够推净敞口非负）——保留前提以使
  定理陈述完整表达"双开"语义，非冗余（删之则定理退化为不含双开语义的纯算术命题）。

  ★诚实：`q_r ≥ b_s`（核心仓 ≥ 对冲仓）是**Θ_risk 的仓位约束**（不由缠论导出），
  本定理在该约束下证净敞口非负；约束不成立（b_s > q_r）则净敞口转负 = 实质转空，
  此时已超出"对冲"语义——这是边界条件，由 Θ_risk 把守，不在 L0 默认声称。
-/
theorem hedge_reduces_not_reverses (H : TwoVoiceHedge)
    (qr bs : Nat) (_hqr : qr > 0) (_hbs : bs > 0) (hcover : qr ≥ bs) :
    -- 方向相反（赋格交替）
    H.T.σ H.r ≠ H.T.σ H.s ∧
    -- 净敞口非负（仍净多 / 中性，非转空）
    (qr : Int) - (bs : Int) ≥ 0 := by
  constructor
  · exact H.T.adjacent_opposite H.s H.r H.hrs |>.symm
  · have : (bs : Int) ≤ (qr : Int) := by exact_mod_cast hcover
    omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 标签声明 + 自审清单（formalization-validity-domain + 615 gatekeeper）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Fugue 标签种类 `FugueTag`（615 gatekeeper，诚实分层）。

  - `OperationalSemanticsOnly`：操作语义（给定声部树 + 环境的 4 动作态划分），**不是分类定理**。
  - `ThetaParametrized`：Θ-参数化（仓位大小 b_v / 止损阈值是 Θ_risk，非缠论可导）。
  - `RuntimeGuard`：运行时守卫（区间套/许可的实时取值由数据流决定，env 字段在运行时填充）。
  - `EmpiricalDomain`：经验有效域（双开盈利/对冲有效性 = L3，需真实数据，不由 L0 声称）。

  ★**没有** `TrueCompleteClassification` 构造子——4 动作态的穷尽互斥是 L0 操作语义划分，
  不冒充缠论走势的真完全分类。
-/
inductive FugueTag where
  | OperationalSemanticsOnly
  | ThetaParametrized
  | RuntimeGuard
  | EmpiricalDomain
deriving DecidableEq, Repr

/--
  ★本文件的认识论标签（L0 自标注）：4 动作态推导是操作语义 + Θ-参数化，非真完全分类。
-/
def fugueTag : FugueTag := FugueTag.OperationalSemanticsOnly

/-
  ════════════════════════════════════════════════════════════════════════
  ## 自审清单（codex 自审靶子 + result-package 六要素）

  【结论】Strict/Fugue.lean 形式化了 π_Θ 推导第 5-7 步：
    1. depth_parity 强形式：σ_v = (-1)^depth_v（`FugueTree.depth_parity`）+ 赋格交替
       σ_v = -σ_{p(v)}（`FugueTree.alternating`，从 dirOfDepth_succ 推导）+ 相邻反向。
    2. 父子许可 `Permit`（根=marketNormal；子=q_p>0 ∧ ¬Exit_p ∧ insideParent）+
       进场许可 `EnterOK`（Permit ∧ sameNest）。
    3. 退出 `Exit` = emergency ∨ ancestorClosing ∨ stop ∨ reverseNest；级联关闭
       `exit_cascade`（祖先触发 ⟹ 后代 Exit，对 IsAncestor）+ `emergency_cascade_from_root`。
    4. ★核心 `voice_action_exhaustive_exclusive`：4 动作态 {close,open,hold,wait} Σ𝟙=1
       （穷尽 ∨ + 6 对互斥 ¬∧，全证）；`actState_sound`（全函数与谓词对齐）；
       `targetPos_spec`（q̃=0/b_v/q_v/0）；`toStrict`（→ Classification.StrictAction）。
    5. 二声部 hedge：`core_long_short_hedge_open`（根 hold + 子 open，保留核心多头开短空）/
       `core_long_short_hedge_close`（低级别买区间套 ⟹ 平短空）/
       `core_long_short_hedge_cascade_close`（高级别失效 ⟹ 先关子后关根，级联）/
       `hedge_reduces_not_reverses`（多空双开 = 降低净多头暴露非转空，在 q_r≥b_s 约束下）。

  【定义依据】
    - Classification.lean §结构 8 `StrictAction`（buy/sell/add/reduce/hold/close/wait）——
      本文件 4 主态 {close,open,hold,wait} 经 `toStrict` 方向化映射到它（open→buy/sell 按 σ_v）。
    - StrategyFamily §5 VoiceTree（alternating/cascade_close/IsAncestor）——本文件 depth_parity
      是其强化（σ 从 depth 导出，depth_child 严格+1，rooted 强制单根）。

  【边界条件（结论翻转点）】
    - 4 动作态 Σ𝟙=1 依赖 Exit/EnterOK 可判定（用 classical）——纯 Prop 层，不影响 L0。
    - `hedge_reduces_not_reverses` 的"非转空"结论依赖 `q_r ≥ b_s`（核心仓 ≥ 对冲仓，Θ_risk）；
      若 b_s > q_r 则净敞口转负 = 实质转空，超出对冲语义（边界，Θ_risk 把守）。
    - `core_long_short_hedge_open` 需显式 `¬ Exit s`——EnterOK 不蕴含 ¬Exit（独立判定），
      若子声部自身处于退出态则 open 不成立（落 close）。
    - depth_parity/alternating 依赖 depth_child 严格 +1——若级别非逐层降一级（跳级）则
      σ 奇偶推导失效（但 FugueTree 公理强制 depth_child，跳级不在定义域内）。

  【下游推论】
    - 任何实例化 FugueTree 的工位（多声部赋格执行器）自动获得 4 动作态穷尽互斥 + 级联关闭，
      无需重证"动作不冲突 / 父关后代关"。
    - π_Θ 的声部级动作产出 = 每声部 actState（全函数），与 StrategyFamily 的 π 单声部策略
      组合为多声部策略族。
    - 多空双开结构允许（承自 StrategyFamily `long_short_both_open_allowed`）在本文件细化为
      "根 hold + 子 open"的具名 hedge 推导，且证净敞口语义（降暴露非转空）。

  【谱系引用】
    - StrategyFamily §5（VoiceTree 赋格交替 + 级联关闭，简版）→ 本文件（depth_parity 强形式 +
      4 动作态完整推导 + 二声部 hedge 具名定理）。本文件是 §5 的真子集推进，无概念分离冲突。
    - Classification §结构 8（Strategy/StrictAction，单声部全函数策略）→ 本文件（声部级 4 主态划分）。
    - 多空双开不禁令承自 StrategyFamily §6（不加 Q⁺Q⁻=0 禁令）——本文件 `hedge_reduces_not_reverses`
      给净敞口语义但仍不禁双开（只在 b_s>q_r 边界标 Θ_risk 把守）。

  【影响声明】
    - 新增独立文件 Strict/Fugue.lean（namespace Strict.Fugue），import Strict.Classification。
    - 不修改任何现有文件；lakefile roots 是否加入 Fugue 由 Lead 决定（本文件不碰 lakefile）。
    - 认识论等级：全文件 L0（OperationalSemanticsOnly + ThetaParametrized）——
      4 动作态 Σ𝟙=1 / 级联关闭 / hedge 推导是定义内蕴逻辑必然，非实盘有效性声明。

  ★诚实分层（formalization-validity-domain）：
    - L0（可证）：赋格交替、depth_parity、级联关闭、4 动作态穷尽互斥、净敞口非负（给定约束）。
    - Θ_risk（承载非导出）：b_v 手数、stop 阈值、q_r≥b_s 覆盖约束。
    - L3（不声称）：双开盈利、对冲降低实际回撤、4 动作态实盘最优——需真实数据，EmpiricalDomain。
  ════════════════════════════════════════════════════════════════════════
-/

end Strict.Fugue
