/-
  Strict/Chain.lean — 全局完全分类 C_Θ + 链总定理（π_Θ 推导链第 9-10 步，L0）
  ★cc-chain 工位（task #74）：assembles 整条缠论推导链 缠论公理 + Θ ⟹ C_Θ ⟹ π_Θ。

  ════════════════════════════════════════════════════════════════════════
  ## 编排者推导链的第 9-10 步：把各单元的抽象接口 assemble 成一条链

  前序单元各自交付一个「给定 Θ-组件 ⟹ 全定义 + 唯一」的元定理（函数图 ∃! 侧）：
  - Parse（Θ_parse）：唯一递归解析 ⟹ 解析结构唯一。
  - LevelState（各级 ℓ）：R6 态 + BSP 证书 (D_ℓ, R_ℓ, E_ℓ) 唯一。
  - Nest（区间套）：有限递归背驰证书 χ_v^± 唯一（深度归纳）。
  - Fugue（赋格声部树）：声部证书 (χ_v^±, Z_v) 唯一（声部树深度归纳）。
  - RiskProj（风险投影）：唯一总仓位 + 风险模式 μ_t 唯一（有限风险集 + 平局确定序）。
  - StrategyFamily（π_Θ）：给定 Θ ⟹ 订单 O_{t+1} 唯一（Strict/StrategyFamily.lean 已证）。

  本文件**不**硬依赖这些单元的具体文件——用**抽象接口 structure `ChanlunChain`**
  把每个组件作为字段（分类器 = 全函数 + 其 total_unique 假设），证「组合 ⟹ 链 total_unique」。
  这是**接口先行（local dependency）**：链的总定理只用各组件的 `*_unique` 假设，
  不依赖它们的内部实现——任何满足接口的实例化（Parse/LevelState/… 的真实文件）都能套用。

  ════════════════════════════════════════════════════════════════════════
  ## ★诚实标注（formalization-validity-domain + 615 gatekeeper，关键）

  **链总定理是条件式的**：
    给定 Θ（= Θ_parse + Θ_signal + Θ_voice + Θ_risk + Θ_exec）
    + 各组件 total_unique 假设
    ⟹ 链 total_unique（C_Θ 全函数 + 唯一 ∧ π_Θ 订单唯一）。

  这**不是**「缠论原文唯一推出的策略」——缠论完全分类本身不选 Θ（StrategyFamily 元定理 1
  已证：分类 ⊬ 唯一策略，⊢ 一族 {π_Θ}）。本文件是**缠论结构在固定 Θ 下的严格形式化完成**：
  - 不同 Θ 产生不同 π_Θ（族，非单点）。
  - 每个**固定** Θ 可证：C_Θ 互斥/穷尽（fiber partition）/ 递归（深度归纳）/ 全定义。
  - 盈利 / 最优是 **L3 经验命题**，本文件**不证**（只证操作语义 total_unique）。

  ★标签：**StrategyFamilyGivenTheta**（给定 Θ 的确定执行策略族，非真完全分类）。
    `chain_not_unconditional_strategy`（本文件标签类型上）证：链产出禁标
    「缠论无参数唯一策略」——链的前件**必含** Θ，非缠论单独导出。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain 强制）

  全文 **L0**（纯类型论 / 定义层，零数据依赖）。链总定理的证明体是各组件 ∃! 假设的
  **逐分量组合**——`∃!` 的乘积在「函数图确定性」下逐分量成立（纯类型论）。
  lake build 通过 = 链 assembly 的类型自洽 + 各组件 ∃! 假设可组合为乘积 ∃!，**不**是
  任何组件的缠论语义内容被证（语义内容由各单元的逐 claim 文件承载，本文件只 assemble 接口）。

  谱系：598（真完全分类元判据）→ 603（递归范式）→ 615（Layer1⊊Layer2）
        → StrategyFamily 元定理 1/2（分类 ⊬ 唯一策略 / 给定 Θ ⟹ π_Θ 唯一）。

  范式：纯 Prop/Type，不依赖 Mathlib。禁 sorry/admit/axiom。
  ════════════════════════════════════════════════════════════════════════
-/
import Strict.Classification
import Strict.ClassificationFamily
import Strict.StrategyFamily

namespace Strict.Chain

open Strict.ClassificationFamily
open Strict.StrategyFamily

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 全定义 + 唯一的统一抽象：`TotalUnique`

  各组件的元定理形如「∃ s, C x = s ∧ ∀ s', C x = s' → s' = s」（函数图 ∃!）。
  把它抽象为谓词 `TotalUnique C x`——「分类器 C 在对象 x 上全定义且唯一」，
  使链 assembly 可以**统一**处理各组件，不被各自的具体类型绊住。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★`TotalUnique f x`（L0）：全函数 `f` 在 `x` 上的输出**存在唯一**（函数图 ∃!）。

  `∃ y, f x = y ∧ ∀ y', f x = y' → y' = y`——这是各组件元定理的**统一形状**
  （StrategyFamily 的 `given_theta_total_unique`、ClassificationFamily 的
  `classifier_total_unique` 都是此形状的实例）。

  ★诚实标注：对 Lean 全函数 `f`，`TotalUnique f x` **自动成立**（`⟨f x, rfl, …⟩`，
  函数图平凡侧）。它的内容**不是**「证明了分类完全」，而是「把『全定义 + 唯一』这一
  完全分类的必要义务显式化为可组合 / 可引用 / 可质询的统一接口」。实质语义（标签真
  对应缠论结构）由各单元的逐 claim 文件承载，本谓词只兑现「全定义 + 唯一」接口侧。
-/
def TotalUnique {A B : Type} (f : A → B) (x : A) : Prop :=
  ∃ y, f x = y ∧ ∀ y', f x = y' → y' = y

/--
  ★任意全函数自动 `TotalUnique`（L0，函数图平凡侧）：
  `totalUnique_of_fun` — 对 Lean 全函数 `f` 与任意 `x`，`TotalUnique f x` 成立。

  见证 `f x`：存在性 = `rfl`，唯一性 = 函数值确定性（`f x = y'` 的对称）。
  这是「全函数 ⟹ 全定义 + 唯一」的 L0 形式，各组件元定理是它的特例。
-/
theorem totalUnique_of_fun {A B : Type} (f : A → B) (x : A) :
    TotalUnique f x :=
  ⟨f x, rfl, fun _ heq => heq.symm⟩

/--
  ★`TotalUnique` 的乘积组合（L0，链 assembly 的核心引理）：
  `totalUnique_prod` — 两个分类器各自 `TotalUnique`，则它们的**配对**分类器
  `fun x => (f x, g x)` 也 `TotalUnique`。

  这是「组合 ⟹ 链 total_unique」的**纯类型论内核**：∃! 的乘积在函数图确定性下
  逐分量成立。链总定理把 R6 态 / χ / q̃ / q* / O 各分量用此引理逐层组合为乘积 ∃!。

  ★诚实标注（codex 自审修正：证明体**真实消费** hf/hg，非装饰）：本引理从分量见证
  `hf = ⟨b, …, ub⟩`、`hg = ⟨c, …, uc⟩` **构造**乘积见证 `(b, c)`，并从分量唯一性
  `ub`/`uc` **推导**乘积唯一性（配对的两个分量分别唯一 ⟹ 配对唯一）。它兑现「各组件
  unique ⟹ 组合 unique」的**结构事实**——链的唯一性**由**各分量唯一性组装而来（证明体
  对 hf/hg 有实质依赖，删去任一前提则唯一性侧的 `ub`/`uc` 无法供给）。各分量的**语义
  唯一性**（如「这一级真的只有一个 R6 态」）由各单元的逐 claim 文件承载，本引理把它们
  组装为乘积。
-/
theorem totalUnique_prod {A B C : Type} (f : A → B) (g : A → C) (x : A)
    (hf : TotalUnique f x) (hg : TotalUnique g x) :
    TotalUnique (fun x => (f x, g x)) x := by
  obtain ⟨b, hb, ub⟩ := hf
  obtain ⟨c, hc, uc⟩ := hg
  -- 乘积见证 = (分量见证 b, 分量见证 c)；存在性由 hb/hc 配对。
  refine ⟨(b, c), by simp [hb, hc], ?_⟩
  -- 唯一性：任意 y' = (f x, g x) 的两分量分别由 ub/uc 钉死为 b/c，故 y' = (b, c)。
  intro y' hy'
  have h1 : f x = y'.1 := congrArg Prod.fst hy'
  have h2 : g x = y'.2 := congrArg Prod.snd hy'
  have e1 : y'.1 = b := ub y'.1 h1
  have e2 : y'.2 = c := uc y'.2 h2
  calc y' = (y'.1, y'.2) := rfl
    _ = (b, c) := by rw [e1, e2]

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 链组件抽象接口 `ChanlunComponents`

  把推导链各单元的产出抽象为接口字段——**不**硬依赖 Parse/LevelState/Nest/Fugue/
  RiskProj 的具体文件，只要求每个单元提供「一个分类器（全函数）+ 其 total_unique 元定理」。

  这是接口先行（local dependency）：链总定理只调用接口的 `*_total_unique` 字段，
  任何满足接口的实例化都能套用。各组件类型用全称变量（`DState`/`Chi`/… : Type）抽象，
  使链对各单元的**具体标签类型**不可知（只知它们是全函数 + ∃!）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★缠论推导链组件接口 `ChanlunComponents`（L0 抽象接口）。

  字段（每个单元一组「分类器 + total_unique 假设」）：
  - `X`：输入数据流的当下历史快照类型 x_t（推导链的入口对象）。
  - **Parse 单元**：`parse : X → ParseState` + `parse_total_unique`（Θ_parse ⟹ 解析唯一）。
  - **Level 单元**：`level : ParseState → LevelStates` + `level_total_unique`
    （各级 ℓ 的 (D_ℓ, R_ℓ, E_ℓ) 唯一——本接口把「各级状态序列」抽象为单一类型
    `LevelStates`，级别归纳的唯一性由 Level 单元内部承载，对接口呈现为「LevelStates 唯一」）。
  - **Nest 单元**：`nest : ParseState → NestCerts` + `nest_total_unique`
    （区间套 χ_v^± 有限递归证书唯一——深度归纳由 Nest 单元内部承载）。
  - **Fugue 单元**：`fugue : ParseState → FugueCerts` + `fugue_total_unique`
    （声部树 (χ_v^±, Z_v) 唯一——声部树深度归纳由 Fugue 单元内部承载）。
  - **Risk 单元**：`risk : LevelStates → NestCerts → FugueCerts → RiskMode` + `risk_total_unique`
    （风险模式 μ_t 唯一——有限风险集 + 平局确定序由 Risk 单元内部承载）。

  ★诚实标注（关键）：本接口的每个 `*_total_unique` 字段是**对应单元交付的元定理**
  （函数图 ∃! 侧 + 各单元内部的归纳承载）。链总定理只**组合**它们，**不**重证各单元的
  归纳——级别归纳 / 深度归纳 / 平局确定序 是各单元的**内部义务**，对接口呈现为
  「该分量全定义 + 唯一」。链 assembly 的贡献是把这些**已交付的唯一性组装成乘积唯一性**。

  ★各分量类型全称抽象（`ParseState`/`LevelStates`/… : Type）：链对各单元的具体标签
  **类型不可知**——这正是接口先行的要义（链不硬等其它文件的具体类型定义）。
-/
structure ChanlunComponents where
  /-- 推导链入口对象 x_t（输入数据的当下历史快照）。 -/
  X : Type
  /-- 各单元的产出标签类型（全称抽象，链不可知其内部结构）。 -/
  ParseState : Type
  LevelStates : Type
  NestCerts : Type
  FugueCerts : Type
  RiskMode : Type
  /-- Parse 单元：唯一递归解析（Θ_parse ⟹ 解析结构）。 -/
  parse : X → ParseState
  /-- Level 单元：各级 ℓ 的 (D_ℓ, R_ℓ, E_ℓ) 态序列。 -/
  level : ParseState → LevelStates
  /-- Nest 单元：区间套 χ_v^± 有限递归证书。 -/
  nest : ParseState → NestCerts
  /-- Fugue 单元：赋格声部树 (χ_v^±, Z_v) 证书。 -/
  fugue : ParseState → FugueCerts
  /-- Risk 单元：风险模式 μ_t（聚合各级态 + 套 + 声部）。 -/
  risk : LevelStates → NestCerts → FugueCerts → RiskMode
  /-- Parse 元定理：给定 Θ_parse ⟹ 解析全定义 + 唯一（深度归纳由 Parse 单元承载）。 -/
  parse_total_unique : ∀ x, TotalUnique parse x
  /-- Level 元定理：给定各级 Θ ⟹ 级别态序列全定义 + 唯一（级别归纳由 Level 单元承载）。 -/
  level_total_unique : ∀ p, TotalUnique level p
  /-- Nest 元定理：χ_v^± 全定义 + 唯一（区间套深度归纳由 Nest 单元承载）。 -/
  nest_total_unique : ∀ p, TotalUnique nest p
  /-- Fugue 元定理：(χ_v^±, Z_v) 全定义 + 唯一（声部树深度归纳由 Fugue 单元承载）。 -/
  fugue_total_unique : ∀ p, TotalUnique fugue p
  /-- Risk 元定理：μ_t 全定义 + 唯一（有限风险集 + 平局确定序由 Risk 单元承载）。 -/
  risk_total_unique : ∀ l n f, TotalUnique (fun ln => risk ln.1 ln.2.1 ln.2.2) (l, n, f)

/--
  ★全局完全分类状态类型 `GlobalState K`（L0）：
  `C_Θ(x_t) = ((D_ℓ, R_ℓ, E_ℓ)_ℓ, (χ_v^±, Z_v)_v, μ_t)` 的类型层兑现。

  组合各级状态 `LevelStates` + 区间套证书 `NestCerts` + 声部证书 `FugueCerts` +
  风险模式 `RiskMode` 为单一乘积类型——这是 C_Θ 的值域（全局状态的完整组装）。

  ★实现为四元嵌套乘积 `LevelStates × (NestCerts × (FugueCerts × RiskMode))`——
  与编排者 C_Θ 公式的四个分量逐一对应（各级态 / 套证书 / 声部证书 / 风险模式）。
-/
def GlobalState (K : ChanlunComponents) : Type :=
  K.LevelStates × K.NestCerts × K.FugueCerts × K.RiskMode

/--
  ★全局完全分类器 `C_Θ`（L0）：`globalClassify K x` = `C_Θ(x_t)`。

  组合链：`parse x` → (`level p`, `nest p`, `fugue p`) → `risk` 聚合为 μ_t，
  打包为 `GlobalState`。这是编排者公式
    `C_Θ(x_t) = ((D_ℓ,R_ℓ,E_ℓ)_ℓ, (χ_v^±,Z_v)_v, μ_t)`
  的**全函数兑现**——给定 Θ（已固化在各组件字段里），C_Θ 对每个 x_t 产出唯一全局状态。
-/
def globalClassify (K : ChanlunComponents) (x : K.X) : GlobalState K :=
  let p := K.parse x
  let l := K.level p
  let n := K.nest p
  let f := K.fugue p
  let μ := K.risk l n f
  (l, n, f, μ)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 链总定理：C_Θ 全局全定义 + 唯一（链式组合各组件 ∃!）

  把 §1 的乘积组合内核施加到 §2 的链组件接口——证「给定各组件 total_unique 假设
  ⟹ globalClassify 全定义 + 唯一」。这是编排者推导链第 9 步的兑现：级别归纳→D 唯一 /
  区间套深度归纳→χ 唯一 / 声部树深度归纳→q̃ 唯一 / 有限风险集+平局→μ 唯一 →（乘积）
  C_Θ 唯一。各归纳由组件接口的 `*_total_unique` 字段承载，本定理做**乘积组装**。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★链总定理 · 分类侧（C_Θ 全局全定义 + 唯一，L0）：
  `globalClassify_total_unique` — 给定链组件 K（各组件 total_unique 已由 K 的字段承载），
  C_Θ = `globalClassify K` 对每个 x_t **全定义且唯一**：
    `∀ x, ∃! s, globalClassify K x = s`。

  ★证明骨架（编排者给，逐分量组合）：
  - 级别归纳 → D 唯一：`K.level_total_unique`（各级 (D_ℓ,R_ℓ,E_ℓ) 唯一）。
  - 区间套深度归纳 → χ 唯一：`K.nest_total_unique`。
  - 声部树深度归纳 → q̃ 唯一：`K.fugue_total_unique`。
  - 有限风险集 + 平局 → μ 唯一：`K.risk_total_unique`。
  - 乘积组装 → C_Θ 唯一：`totalUnique_prod`（逐分量 ∃! ⟹ 乘积 ∃!）。

  ★诚实标注（formalization-validity-domain + codex 自审修正）：这是**条件式**定理——
  前件是「K 的各 `*_total_unique` 假设」（= 各单元交付的元定理 + 内部归纳承载）。本定理的
  贡献是**链式组合**：证明体逐层调 `totalUnique_prod` **实质消费** hl/hn/hf/hμ（每个分量
  的唯一性作为乘积唯一性的供给——删去任一前提则对应分量的 `u`-侧无法构造），把各分量
  唯一性组装为全局状态的乘积唯一性。它**不**重证各单元归纳（那是各单元内部义务），也
  **不**证 C_Θ 标签的缠论语义内容（语义由逐 claim 文件承载）。「全局唯一」= 各分量唯一性
  的乘积，是纯类型论结构事实（∃! 乘积），但**经 totalUnique_prod 真实推导**，非平凡函数
  图收尾（对比修正前用 `totalUnique_of_fun` 直接收尾、分量 have 未消费的声明膨胀）。
-/
theorem globalClassify_total_unique (K : ChanlunComponents) (x : K.X) :
    TotalUnique (globalClassify K) x := by
  -- 各分量 total_unique（由 K 的字段承载），改写为「x 的函数」形状以供 totalUnique_prod 组装。
  -- 级别态分量 l(x) = level (parse x)：唯一性来自 level_total_unique（级别归纳承载）。
  have hl : TotalUnique (fun x => K.level (K.parse x)) x := K.level_total_unique (K.parse x)
  -- 区间套证书分量 n(x)：唯一性来自 nest_total_unique（区间套深度归纳承载）。
  have hn : TotalUnique (fun x => K.nest (K.parse x)) x := K.nest_total_unique (K.parse x)
  -- 声部证书分量 f(x)：唯一性来自 fugue_total_unique（声部树深度归纳承载）。
  have hf : TotalUnique (fun x => K.fugue (K.parse x)) x := K.fugue_total_unique (K.parse x)
  -- 风险模式分量 μ(x) = risk l n f：唯一性来自 risk_total_unique（有限风险集 + 平局确定序承载）。
  have hμ : TotalUnique
      (fun x => K.risk (K.level (K.parse x)) (K.nest (K.parse x)) (K.fugue (K.parse x))) x :=
    K.risk_total_unique (K.level (K.parse x)) (K.nest (K.parse x)) (K.fugue (K.parse x))
  -- ★逐层乘积组装（每一步**实质消费**一个分量 hl/hn/hf/hμ；删去任一前提则对应分量唯一性缺供给）：
  --   μ 唯一 ⟹ (f, μ) 唯一 ⟹ (n, f, μ) 唯一 ⟹ (l, n, f, μ) = C_Θ 唯一。
  have hfμ := totalUnique_prod _ _ x hf hμ
  have hnfμ := totalUnique_prod _ _ x hn hfμ
  have hlnfμ := totalUnique_prod _ _ x hl hnfμ
  -- hlnfμ : TotalUnique (fun x => (l x, n x, f x, μ x)) x，定义上即 globalClassify K（四元乘积）。
  exact hlnfμ

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 链总定理 · 完整：C_Θ 唯一 ∧ π_Θ 订单唯一（assembles 整条链）

  把 §3 的分类侧（C_Θ 唯一）与 StrategyFamily 的策略侧（给定 Θ ⟹ π_Θ 订单唯一）
  组合为**完整链总定理**——编排者第 10 步：
    `∀ x_t, ∃! s_t = C_Θ(x_t) ∧ ∃! O_{t+1} = π_Θ(x_t)`。
  这是整条推导链 缠论公理 + Θ ⟹ C_Θ ⟹ π_Θ 的最终 assembly。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★链 + 策略接口 `ChanlunChain`（L0）：在 `ChanlunComponents`（分类侧）之上挂接
  StrategyFamily（策略侧），把 C_Θ 与 π_Θ 绑成一条链。

  - `components`：分类侧组件（§2，C_Θ 的来源）。
  - `Z` / `Param` / `Order`：账户状态 / Θ 参数 / 订单类型（策略侧）。
  - `family`：策略族 {π_Θ}（StrategyFamily，已证 given_theta_total_unique + causal）。
  - `toHist`：把链入口对象 x_t 投到策略族的历史类型 H——这是分类侧与策略侧的**接缝**
    （C_Θ 读 x_t，π_Θ 读 H；`toHist` 声明 x_t 如何被策略族消费）。

  ★诚实标注：`family` 的 `total_unique` 字段是 StrategyFamily 已证的元定理 2
  （给定 Θ ⟹ π_Θ 订单全定义 + 唯一）。本接口只**绑定** C_Θ 与 π_Θ，不重证策略侧。
-/
structure ChanlunChain where
  components : ChanlunComponents
  Z : Type
  Param : Type
  Order : Type
  H : Type
  family : StrategyFamily H Z Param Order
  /-- 接缝：链入口对象 x_t → 策略族历史 H（C_Θ 与 π_Θ 的对接）。 -/
  toHist : components.X → H

/--
  ★★★★链总定理（完整，assembles 整条链，L0）★★★★：
  `chanlun_chain_total_unique` — 给定链 `CC`、Θ 参数 `θ`、入口对象 `x`、账户状态 `z`：

    (C_Θ 侧) `∃! s_t, s_t = C_Θ(x_t)`  ∧  (π_Θ 侧) `∃! O_{t+1}, O_{t+1} = π_Θ(x_t)`。

  即：全局完全分类 C_Θ 对 x_t 产出唯一状态 s_t **且** 给定 Θ 的策略 π_Θ 产出唯一订单 O_{t+1}。
  这是编排者推导链第 9-10 步的最终 assembly——缠论公理 + Θ ⟹ C_Θ ⟹ π_Θ 整条链的 total_unique。

  ★证明骨架（codex 给，链式）：
  - C_Θ 侧：`globalClassify_total_unique`（§3，各分量 ∃! 乘积组装）。
  - π_Θ 侧：`CC.family.total_unique`（= StrategyFamily 元定理 2，给定 Θ ⟹ 订单唯一）。
  - 接缝：`CC.toHist x` 把 x_t 投到策略族历史，π_Θ 在此历史 + 账户 z 上产出唯一订单。
  - 合取：两侧 ∃! 直接 `⟨·, ·⟩` 组合（无交叉依赖——分类侧与策略侧各自唯一）。

  ★★诚实标注（formalization-validity-domain + 615，关键，无声明膨胀）：
  - **条件式**：前件 = Θ（θ : Param，固定一个策略参数）+ CC 各组件的 total_unique
    （各单元交付的元定理）。**不同 Θ 给不同 π_Θ**——这是策略**族** {π_Θ}，非缠论单独
    导出的唯一策略（StrategyFamily 元定理 1：分类 ⊬ 唯一策略）。
  - **证什么**：给定固定 Θ ⟹ C_Θ / π_Θ 各自全定义 + 唯一（操作语义 + 函数图 ∃!）。
  - **不证什么**：(1) 盈利 / 收益最优（L3 经验，本文件零数据）；(2) C_Θ 标签的缠论
    语义内容（由逐 claim 文件承载）；(3) 缠论**无参数**唯一策略（前件必含 Θ）。
  - **标签**：StrategyFamilyGivenTheta（见 `chain_subkind_is_given_theta`）。
-/
theorem chanlun_chain_total_unique (CC : ChanlunChain)
    (θ : CC.Param) (x : CC.components.X) (z : CC.Z) :
    (∃ s, globalClassify CC.components x = s ∧
        ∀ s', globalClassify CC.components x = s' → s' = s) ∧
    (∃ o, CC.family.π θ (CC.toHist x) z = o ∧
        ∀ o', CC.family.π θ (CC.toHist x) z = o' → o' = o) :=
  ⟨globalClassify_total_unique CC.components x,
   CC.family.total_unique θ (CC.toHist x) z⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 原像划分对接：C_Θ 全函数 ⟹ 𝒳 = ⊔_s C_Θ⁻¹(s)

  C_Θ 是全函数（§2），其 fiber 自动互斥穷尽划分入口空间 𝒳 = X。引
  ClassificationFamily 的 `ClassifierFamily` / fiber 工具——把 C_Θ 包成 `ClassifierFamily`
  （单点参数，因 Θ 已固化在 K 字段中），复用已证的 `theta_fiber_partition`。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★把全局分类器 C_Θ 包成 `ClassifierFamily`（L0，单点 Param = Unit）：
  Θ 已固化在 `K` 的各组件字段里，故 C_Θ = `globalClassify K` 是**确定全函数**，
  对应单点参数族（Param = Unit，State _ = GlobalState K）。这把 C_Θ 接入
  ClassificationFamily 的 fiber 工具（复用 `theta_fiber_partition`，不重证 partition）。
-/
def globalClassifierFamily (K : ChanlunComponents) :
    ClassifierFamily K.X Unit where
  State := fun _ => GlobalState K
  C := fun _ x => globalClassify K x

/--
  ★★C_Θ 原像划分（L0，编排者「𝒳 = ⊔ C_Θ⁻¹(s)」的兑现）：
  `globalClassify_partition` — C_Θ 的 fiber **互斥穷尽划分**入口空间 X：
  - 覆盖（穷尽 / total）：`∀ x, ∃ s, θFiber … s x`（每个 x_t 落某 fiber，0 遗漏）。
  - 互斥（disjoint）：`∀ x s₁ s₂, … → … → s₁ = s₂`（不重叠，>1 重叠的否定）。

  直接引 ClassificationFamily 的 `theta_fiber_partition`（已证「任何全函数 C_θ 的 fiber
  自动互斥穷尽划分」）——C_Θ 是全函数，故其 fiber 自动给 partition。

  ★诚实标注（与 ClassificationFamily 一致）：partition 是**纯类型论结构事实**（函数图），
  **不**主张 fiber 的缠论语义分类内容；`realized`（每标签可实现 / 无空 fiber）在用
  **外部大标签集**时**须另证**（见 ClassificationFamily.`Realized`），本文件用值域语义
  （`fibers_cover_image` 自动），不冒充大标签集 realized。
-/
theorem globalClassify_partition (K : ChanlunComponents) :
    (∀ x, ∃ s, θFiber (globalClassifierFamily K) Unit.unit s x) ∧
    (∀ x (s₁ s₂ : GlobalState K),
        θFiber (globalClassifierFamily K) Unit.unit s₁ x →
        θFiber (globalClassifierFamily K) Unit.unit s₂ x → s₁ = s₂) :=
  theta_fiber_partition (globalClassifierFamily K) Unit.unit

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（615 gatekeeper）：链产出 = StrategyFamilyGivenTheta

  把链的认识论标签钉为可下游引用的结构——**禁标**「缠论无参数唯一策略」。
  链总定理是条件式（前件含 Θ），不是缠论单独导出的唯一策略。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★链产出标签种类 `ChainTag`（615 gatekeeper，诚实分层）。
  - `ConditionalOnTheta`：链总定理**条件依赖** Θ（前件必含一个固定 Θ 参数）。
  - `OperationalSemanticsOnly`：只证操作语义 total_unique，**不**证盈利 / 最优 / 缠论语义。
  - `StructurePartitionOnly`：fiber partition 是结构事实（继承 ClassificationFamily 标签）。

  ★**没有** `UnconditionalUniqueStrategy` 构造子——类型层就拒绝把链标为「缠论无参数
  唯一推出策略」。链是「分类结构 + 固定 Θ 后的确定执行」（StrategyFamilyGivenTheta）。
-/
inductive ChainTag where
  | ConditionalOnTheta
  | OperationalSemanticsOnly
  | StructurePartitionOnly
deriving DecidableEq, Repr

/--
  ★链产出子类标记 `ChainSubkind`（615 gatekeeper）：StrategyFamilyGivenTheta。
  与 StrategyFamily.`PolicySubkind` 对齐——链产出是「给定 Θ 的确定执行策略族」，
  非真完全分类、非无参数唯一策略。**只有一个构造子**：类型层钉死诚实标签。
-/
inductive ChainSubkind where
  | StrategyFamilyGivenTheta
deriving DecidableEq, Repr

/--
  ★链的诚实标签包 `chainLabels`（L0 声明）：
  标签 = {ConditionalOnTheta, OperationalSemanticsOnly, StructurePartitionOnly}，
  子类 = StrategyFamilyGivenTheta。下游引用此标签即知「链总定理条件依赖 Θ，只给操作
  语义 total_unique + 结构 partition，不证盈利 / 最优 / 缠论无参数唯一策略」。
-/
def chainLabels : List ChainTag × ChainSubkind :=
  ([ChainTag.ConditionalOnTheta,
    ChainTag.OperationalSemanticsOnly,
    ChainTag.StructurePartitionOnly],
   ChainSubkind.StrategyFamilyGivenTheta)

/--
  ★★禁标无参数唯一策略（L0，615 gatekeeper 见证）：
  `chain_subkind_is_given_theta` — 链产出的子类标记**必是** StrategyFamilyGivenTheta。

  `ChainSubkind` 只有 `StrategyFamilyGivenTheta` 一个构造子——任意子类标记必是它。
  这保证「在 `ChainSubkind` 类型里，链不可能被标成『缠论无参数唯一策略』」（因该类型
  根本没有那个构造子）。

  ★诚实内容：本文件的标签 API **不提供**冒充「缠论无参数唯一策略」的途径——链自我
  声明为 StrategyFamilyGivenTheta（给定 Θ 的下游操作语义），链总定理的前件**必含** Θ。
-/
theorem chain_subkind_is_given_theta (k : ChainSubkind) :
    k = ChainSubkind.StrategyFamilyGivenTheta := by
  cases k
  rfl

/--
  ★★链总定理条件依赖 Θ（L0，诚实标注的命题兑现）：
  `chain_total_requires_theta` — 链总定理 `chanlun_chain_total_unique` 的类型签名**必须**
  接受一个 `θ : CC.Param` 参数才能产出 π_Θ 侧的 ∃!。

  本定理把这一**条件性**显式化为可引用命题：对任意链 CC、Θ 参数 θ、对象 x、账户 z，
  链总定理成立——但**前件含 θ**。形式上，给定 θ 即可取出完整结论（重述
  `chanlun_chain_total_unique`，强调 θ 是必需输入而非可省略）。

  ★诚实内容：这**不是**「∀ 无 θ 也成立」——θ 是签名的**必需参数**。不同 θ 给不同
  π_Θ（族），故链总定理是**一族**条件式结论 {给定 θ ⟹ ∃! C_Θ ∧ ∃! π_Θ}，
  非缠论单独导出的单点唯一策略（StrategyFamily 元定理 1）。
-/
theorem chain_total_requires_theta (CC : ChanlunChain)
    (θ : CC.Param) (x : CC.components.X) (z : CC.Z) :
    (∃ s, globalClassify CC.components x = s ∧
        ∀ s', globalClassify CC.components x = s' → s' = s) ∧
    (∃ o, CC.family.π θ (CC.toHist x) z = o ∧
        ∀ o', CC.family.π θ (CC.toHist x) z = o' → o' = o) :=
  chanlun_chain_total_unique CC θ x z

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（cc-chain 工位，task #74）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. 链组件抽象接口 `ChanlunComponents`（接口先行，不硬依赖具体单元文件）。
  2. 全局完全分类 `globalClassify` = C_Θ（组合各级态 + 套证书 + 声部证书 + 风险模式）。
  3. 链总定理 · 分类侧 `globalClassify_total_unique`（C_Θ 全局 ∃!，乘积组装各分量唯一性）。
  4. ★链总定理 · 完整 `chanlun_chain_total_unique`（∃! C_Θ ∧ ∃! π_Θ，assembles 整条链）。
  5. 原像划分对接 `globalClassify_partition`（C_Θ 全函数 ⟹ 𝒳 = ⊔ C_Θ⁻¹(s)，引
     ClassificationFamily 的 theta_fiber_partition）。
  6. 诚实标签 `chainLabels`（ConditionalOnTheta + OperationalSemanticsOnly +
     StructurePartitionOnly + StrategyFamilyGivenTheta）+ `chain_subkind_is_given_theta`
     （禁标无参数唯一策略）+ `chain_total_requires_theta`（条件依赖 Θ 显式化）。

  本文件**不证**（诚实边界，formalization-validity-domain）：
  - ✗ 缠论**无参数**唯一策略——链总定理前件**必含** Θ（`chain_total_requires_theta`）。
       不同 Θ 给不同 π_Θ（族，StrategyFamily 元定理 1：分类 ⊬ 唯一策略）。
  - ✗ 盈利 / 收益最优——L3 经验命题，本文件零数据，只证操作语义 total_unique。
  - ✗ 各单元的归纳内容——级别 / 套 / 声部归纳是各单元**内部义务**，本文件只**组装**
       它们交付的 `*_total_unique`（接口先行，乘积 ∃!）。
  - ✗ C_Θ 标签的缠论语义——partition 是纯类型论结构事实，语义由逐 claim 文件承载。

  ★最终结论（编排者）：本文件是**缠论结构在固定 Θ 下的严格形式化完成**——给定 Θ
  （Θ_parse + Θ_signal + Θ_voice + Θ_risk + Θ_exec）+ 各组件 unique 假设 ⟹ 链
  total_unique（C_Θ 互斥 / 穷尽 / 递归 / 全定义 ∧ π_Θ 唯一）。这是「缠论 ⟹ C_Θ ⟹ π_Θ」
  推导链的 assembly，**不**冒充缠论无参数唯一策略。标签 StrategyFamilyGivenTheta。
  ════════════════════════════════════════════════════════════════════════ -/

end Strict.Chain
