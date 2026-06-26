/-
  Strict/Parse.lean — π_Θ 第2步：唯一递归解析 D_{ℓ,t}（Θ_parse 参数化，L0）
  ★cc-parse 工位（task #76；598/603/615/617 谱系 + 编排者 π_Θ 推导第2步）

  topo_address: swarm/pi-theta/cc-parse ｜ parent: Lead

  ── 核心理论结论（编排者 π_Θ 推导第2步） ──
  **递归解析的唯一性不是缠论的无参数真理——它是 缠论公理 + Θ_parse 的产物（617 精化）。**

  解析过程（standard §第2部分）：
  - D₀ = B_Θ(h)：基底解析（包含处理 → 分型 → 笔 → 线段），由 K 线历史 h 生成第 0 级结构。
  - D_{ℓ+1} = Φ_{ℓ,Θ}(D_ℓ)：递归升级（低级走势 → 本级中枢 → 本级走势类型）。
  - 每级状态 D_{ℓ,t} = (完成走势序列 confirmed, 未完成尾部 active)——
    **confirmed**（已闭合走势，交易可用）/ **active**（未完成尾部，仅预警，对接 615 第5部分 OpenTail）。

  ── 核心定理 `parse_unique`（本文件主交付） ──
  `∀ h ∀ ℓ, ∃! D_{ℓ,t}`——但**条件依赖 Θ_parse**。把 Θ_parse 作显式参数（`ParseParams`），
  证「**给定 Θ_parse ⟹ 解析唯一**」。Θ_parse 编码缠论公理外的全部边界约定：
  包含同向合并规则 / 相同极值取舍 / 笔与线段边界判据 / 中枢重叠开闭约定 /
  canonical 选择器 `Can_Θ = min_{≺_Θ}`（纤维内确定性截面选择）。

  ── 诚实标注（关键，formalization-validity-domain + no-patch-mentality + 615 gatekeeper） ──
  ★**无 Θ_parse 则合法分解可多值**（真多义性）——这不是 bug，是缠论的结构事实。
    引 `Strict.Decomp` 的 `decomp_ambiguity_witness`（gauge 前商 > 1，T₅₂ 真多义）
    作证「未参数化时多值」；引 `Decomp.gaugeFix`/`GaugeNormal`（按 level 最小、平级最左的
    纤维内确定性选择）作证「gauge 固定后唯一」。Θ_parse 在本文件**抽象**了 gauge-fixing 这一动作。
  ★唯一性**不是缠论无参数真理**：`parse_unique` 的前件必含 `ParseParams`（Θ_parse）。
    去掉 Θ_parse（用纯缠论公理）则 `parse_ambiguous_without_theta` 证多值（≥ 2 合法解析）。
  ★当前只形式化**结构骨架 + Θ_parse 作参数**：`ParseParams` 的具体边界规则（包含/极值/开闭判据）
    作字段**不全部实例化**（诚实：待实例化——这些是 Θ-参数化设计选择，不由缠论公理唯一钉死）。

  ── 标签声明（615 gatekeeper） ──
  ★主标签：**StructurePartitionOnly**（结构划分，非真完全分类——解析给结构骨架，
        不自动给语义不变量下的双射完备性）。
  ★Θ参数化前件：唯一性前件含 `ParseParams`（**ParameterizedUniqueness**，非无条件唯一）。
  ★**禁标 TrueCompleteClassification**——解析唯一性是「缠论公理 + Θ_parse」的产物，
    不是缠论内部的无参数分类定理（`parse_not_true_classification` 在本文件标签类型上证此）。

  ── 与现有 Strict 库的关系 ──
  - **复用**（不改）`Strict.Decomp` 的 gauge/多义结构作参考与对照（多义反例 + gaugeFix 截面唯一）。
  - **复用**（不改）`Strict.RecursiveKernel`（Classification.lean §结构6）的分级宇宙思想
    （U n = 第 n 级对象，Can/Boundary 升降级）——本文件的 D_{ℓ} 递归与之同构，但聚焦
    「带 Θ 的唯一性」而非 Kernel 的「规范化唯一边界」。
  - **复用**（不改）`Strict.OpenTailSystem`（§结构7）的「未完成走势 = 当下状态 + 分支集」
    思想——本文件 active 尾部对接 OpenTail 的 confirmed/active 区分。

  范式：纯 Prop/Type，不依赖 Mathlib（继承 Strict 库自包含约束，`∃!` 显式展开）。
  禁 sorry/admit/axiom。验证：`cd formal && lake env lean Strict/Parse.lean`。
-/

import Strict.Classification

namespace Strict.Parse

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 走势方向标签 + 每级解析状态 D_{ℓ,t}（confirmed / active 区分）

  缠论走势三型：U（上涨 Up）/ D（下跌 Down）/ P（盘整 Pivot/盘整）。
  每级解析状态 = 已完成走势序列（confirmed，交易可用）+ 未完成尾部（active，仅预警）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★走势方向 `MoveDir`：U（上涨）/ D（下跌）/ P（盘整）——standard 走势三型 {U, D, P}。
-/
inductive MoveDir where
  | U  -- 上涨走势
  | D  -- 下跌走势
  | P  -- 盘整走势
deriving DecidableEq, Repr

/--
  ★完成走势 `ConfirmedMove`（confirmed，L0）：一个**已闭合**的本级走势。

  - `dir : MoveDir`：方向（U/D/P）。
  - `lo hi : Int`：走势区间下界/上界（极值，用 Int 作离散价格刻度，避免 Mathlib 实数依赖）。

  ★confirmed = 走势已完成（终点已确认）。**交易只用 confirmed**（standard §第5部分裁决）。
-/
structure ConfirmedMove where
  dir : MoveDir
  lo : Int
  hi : Int
deriving DecidableEq, Repr

/--
  ★未完成尾部 `ActiveTail`（active，L0，对接 615 标准第5部分 OpenTail）：

  当前级别**正在延伸、尚未闭合**的走势尾部。它**不能**被强行分类为最终走势结果
  （standard §第5部分核心修正：未完成走势只能唯一分类为「当下状态」，不能唯一分类为「最终结果」）。

  - `provDir : MoveDir`：当下**临时**方向（provisional——可能随未来延伸翻转，不是终局）。
  - `anchor : Int`：尾部起点锚（最后一个 confirmed 走势的终点）。

  ★active **仅预警，不交易**（standard §第5部分裁决：交易只用 confirmed）。
    `ActiveTail` 对应 `Strict.OpenTailSystem.current`（当下状态，唯一可分类），未来延伸的
    分支集（互斥穷尽）由 OpenTailSystem 的 `BranchPred` 承载，**不**在本结构内压成最终结果。
-/
structure ActiveTail where
  provDir : MoveDir
  anchor : Int
deriving DecidableEq, Repr

/--
  ★每级解析状态 `ParseState`（D_{ℓ,t}，L0）：第 ℓ 级在时刻 t 的解析状态。

  - `confirmed : List ConfirmedMove`：**已完成走势序列**（交易可用，唯一确定）。
  - `active : Option ActiveTail`：**未完成尾部**（`none` = 尾部恰好闭合无悬挂；
    `some τ` = 有一段正在延伸的 active 走势，仅预警）。

  ★这是 standard §第2部分「D_{ℓ,t} = (完成走势序列, 未完成尾部 τ)」的精确结构形式。
    confirmed/active 的**分离**是 standard §第5部分裁决的落实——把「当下能唯一确定的部分」
    （confirmed）与「未来未定的部分」（active，分支集）在类型层切开。
-/
structure ParseState where
  confirmed : List ConfirmedMove
  active : Option ActiveTail
deriving DecidableEq, Repr

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 Θ_parse 参数（ParseParams）——边界约定的显式参数化

  Θ_parse 编码缠论公理**外**的全部解析边界约定。给定 Θ_parse，基底解析 B_Θ 与递归升级
  Φ_{ℓ,Θ} 全定义且确定；去掉 Θ_parse 则合法分解可多值（§4 多义反例）。

  ★诚实：`ParseParams` 的字段是「确定性选择器 + 边界判据」的**抽象签名**——具体的
    包含合并/极值取舍/笔线段边界/中枢开闭规则作字段类型出现，但**不全部实例化**为具体算法
    （它们是 Θ-参数化设计选择，待下游按标的/周期实例化；本文件只证「给定它们 ⟹ 唯一」）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★K 线 `Kline`（最小占位事件型，L0）：包含处理前的原始 K 线（high/low）。

  用 Int 离散刻度（避免 Mathlib 实数）。`high ≥ low` 不在类型层强制（边界由 Θ 的包含规则处理）。
-/
structure Kline where
  high : Int
  low : Int
deriving DecidableEq, Repr

/--
  ★★Θ_parse 参数结构 `ParseParams`（L0，本文件核心参数）。

  把「缠论公理外」的全部解析边界约定参数化为**确定性选择器**（全函数）。给定一个
  `ParseParams`，整条解析链（基底 B_Θ + 递归 Φ_{ℓ,Θ}）成为**全定义确定**函数 ⟹ 解析唯一。

  字段（standard §第2部分列举的 Θ_parse 内容，全部作确定性选择器）：
  - `baseParse : List Kline → ParseState`：**基底解析 B_Θ**——把 K 线历史确定地解析为第 0 级状态
    （内部封装：包含同向合并 + 分型识别 + 笔划分 + 线段划分，全部按 Θ 的边界规则确定）。
  - `recogStep : ParseState → ParseState`：**递归升级 Φ_{ℓ,Θ}**——把第 ℓ 级状态确定地升级为
    第 ℓ+1 级状态（内部封装：低级走势聚合 → 本级中枢识别 → 本级走势类型，按 Θ 的开闭约定确定）。

  证明义务（确定性 = 全函数性，全部 L0）：
  - `baseParse` / `recogStep` 是 **Lean 全函数** ⟹ 对每个输入产出**唯一**输出（本结构无需额外
    证明义务字段——全函数性自动编码「给定 Θ ⟹ 确定」，这正是 parse_unique 的依据）。

  ★诚实标注（formalization-validity-domain，避免声明膨胀 090 号）：
    `baseParse`/`recogStep` 的**具体内容**（用什么包含合并规则、极值取舍、笔线段边界判据、
    中枢重叠开闭约定、canonical 选择器 `Can_Θ = min_{≺_Θ}`）全部是 **Θ-参数化，非 L0 可导**——
    它们不由缠论公理唯一推出，是 extra-缠论的边界设计选择。本结构**只**把它们抽象为确定性
    选择器签名，**不全部实例化**为具体算法（待下游按标的/周期实例化，见 `ParseParams.refl_*` 注释）。
    本文件证的是「给定这些 Θ-选择器 ⟹ 组合出的解析 D_{ℓ,t} 全定义/唯一」，**不**证「这些选择器
    的具体取值由缠论唯一确定」（后者正是 §4 否证的——无 Θ 则多值）。
-/
structure ParseParams where
  /-- 基底解析 B_Θ：K 线历史 → 第 0 级解析状态（包含/分型/笔/线段，全按 Θ 边界规则确定）。 -/
  baseParse : List Kline → ParseState
  /-- 递归升级 Φ_{ℓ,Θ}：第 ℓ 级状态 → 第 ℓ+1 级状态（中枢/走势类型，全按 Θ 开闭约定确定）。 -/
  recogStep : ParseState → ParseState

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 递归解析 D_{ℓ,t}：D₀ = B_Θ(h)，D_{ℓ+1} = Φ_{ℓ,Θ}(D_ℓ)

  给定 Θ_parse，从 K 线历史 h 递归生成每级解析状态。这是 standard §第2部分递归方程的
  Lean 实现——**全定义**（每个 ℓ 都有状态）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★递归解析 `parse`（D_{ℓ,t}，L0）：给定 Θ_parse θ 与 K 线历史 h，产出第 ℓ 级解析状态。

  - `parse θ h 0 = θ.baseParse h`（D₀ = B_Θ(h)，基底解析）。
  - `parse θ h (ℓ+1) = θ.recogStep (parse θ h ℓ)`（D_{ℓ+1} = Φ_{ℓ,Θ}(D_ℓ)，递归升级）。

  这是 standard §第2部分递归方程的精确 Lean 形式。**全定义**：对每个 (θ, h, ℓ) 都给出唯一状态
  （θ.baseParse / θ.recogStep 是全函数，递归对 ℓ 良基）。
-/
def parse (θ : ParseParams) (h : List Kline) : Nat → ParseState
  | 0 => θ.baseParse h
  | (ℓ + 1) => θ.recogStep (parse θ h ℓ)

/--
  ★基底解析展开（L0）：`parse θ h 0 = θ.baseParse h`（D₀ = B_Θ(h)）。
-/
@[simp] theorem parse_zero (θ : ParseParams) (h : List Kline) :
    parse θ h 0 = θ.baseParse h := rfl

/--
  ★递归升级展开（L0）：`parse θ h (ℓ+1) = θ.recogStep (parse θ h ℓ)`（D_{ℓ+1} = Φ_{ℓ,Θ}(D_ℓ)）。
-/
@[simp] theorem parse_succ (θ : ParseParams) (h : List Kline) (ℓ : Nat) :
    parse θ h (ℓ + 1) = θ.recogStep (parse θ h ℓ) := rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 核心定理 parse_unique：给定 Θ_parse ⟹ 解析唯一（条件依赖 Θ_parse）

  ★★★这是本文件主交付。诚实强度：唯一性的前件**必含** ParseParams（Θ_parse）——
  它是「函数图 ∃!」（parse 是 Lean 全函数，给定 θ 后确定），但关键的诚实内容在于
  **前件不可去掉 Θ_parse**（§5 证去掉则多值）。唯一性 = 缠论公理 + Θ_parse，非缠论无参数真理。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★核心定理 `parse_unique`（给定 Θ_parse ⟹ 解析唯一）★★★：

  **给定** Θ_parse `θ`、K 线历史 `h`、级别 `ℓ`，**存在唯一** 解析状态 `D` 使 `parse θ h ℓ = D`。

  这是 standard §第2部分「∀ h ∀ ℓ, ∃! D_{ℓ,t}」的 Lean 形式——但**条件依赖 Θ_parse**：
  定理的**第一个显式参数就是** `θ : ParseParams`（Θ_parse）。

  ★诚实标注证明强度（codex 自审，避免声明膨胀 090 号 / no-patch-mentality 禁令5）：
  这是「函数图的 ∃!」——`parse θ h ℓ` 是 Lean 全函数（θ.baseParse/θ.recogStep 全函数 +
  对 ℓ 良基递归），故全定义（= 存在）与唯一（= 函数确定性）由 θ 固定后**自动成立**，证明体
  仅 `rfl` + 对称。它的**实质内容不在「∃!」本身**（那是接口兑现），而在于：
  **唯一性的前件不可去掉 `θ`**——`parse` 没有 `θ` 就无法定义（baseParse/recogStep 是 θ 的字段），
  且去掉 θ 用纯缠论合法分解谓词则**多值**（§5 `parse_ambiguous_without_theta`）。
  这精确兑现「解析唯一性 = 缠论公理 + Θ_parse 的产物（617 精化）」——**不是**缠论无参数真理。
-/
theorem parse_unique (θ : ParseParams) (h : List Kline) (ℓ : Nat) :
    ∃ D, parse θ h ℓ = D ∧ ∀ D', parse θ h ℓ = D' → D' = D :=
  ⟨parse θ h ℓ, rfl, fun _ heq => heq.symm⟩

/--
  ★解析确定性（L0）：同一 Θ_parse、同一历史、同一级别 ⟹ 解析状态相等。
  `parse θ h ℓ = parse θ h ℓ`——这是 parse_unique 的「确定性」侧的直接形式（同输入同输出）。
  关键：**θ 固定**是前提；不同 θ 可给不同解析（§5），故确定性条件依赖 Θ_parse。
-/
theorem parse_deterministic (θ : ParseParams) (h : List Kline) (ℓ : Nat) :
    parse θ h ℓ = parse θ h ℓ := rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 诚实标注：无 Θ_parse 则多值（真多义性，引 Decomp 多义/gaugeFix 对照）

  ★关键诚实内容：**去掉 Θ_parse（用纯缠论合法分解谓词）则合法解析可多值**。
  本节给出形式化见证：(1) 不同 θ 给不同解析（θ 是非平凡选择）；(2) 抽象的「合法分解」
  谓词在无 gauge-fixing 时可被 ≥ 2 个状态满足（同构于 Decomp.decomp_ambiguity_witness 的
  gauge 前商 > 1）。Θ_parse 在本文件抽象了 Decomp.gaugeFix 这一确定性截面选择动作。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★不同 Θ_parse 给不同解析（L0，θ 的非平凡选择见证）：

  存在两个 Θ_parse `θ₁ ≠ θ₂` 与一个历史 h，使 `parse θ₁ h 0 ≠ parse θ₂ h 0`——
  即**解析结果随 Θ_parse 选择而变**。这坐实「唯一性条件依赖 Θ_parse」：θ 不是冗余参数，
  去掉它（或换一个 θ）解析结果就变。

  ★含义：缠论公理**不**唯一钉死 θ——`θ₁` 与 `θ₂` 都可以是「合法的缠论解析」（满足缠论公理），
  但给出**不同**的第 0 级状态。选哪个是 extra-缠论的边界约定（Θ_parse）。这与
  `Strict.Decomp.decomp_ambiguity_witness`（同 base 两个 ≈ₙ-不等合法分解）同构。
-/
theorem parse_varies_with_theta :
    ∃ (θ₁ θ₂ : ParseParams) (h : List Kline),
      parse θ₁ h 0 ≠ parse θ₂ h 0 := by
  -- θ₁ 把任意历史解析为「空 confirmed + none active」；θ₂ 解析为「空 confirmed + some 尾部」
  -- 这模拟两套不同边界约定（如：是否把末段悬挂记为 active 尾部）给出不同 D₀。
  refine ⟨
    { baseParse := fun _ => { confirmed := [], active := none }
      recogStep := fun s => s },
    { baseParse := fun _ => { confirmed := [], active := some { provDir := MoveDir.U, anchor := 0 } }
      recogStep := fun s => s },
    [], ?_⟩
  -- 两 θ 在空历史上的 D₀ 的 active 字段不同（none ≠ some _）⟹ ParseState 不等
  intro hcontra
  simp only [parse_zero] at hcontra
  -- 从 ParseState 相等提取 active 字段相等，得 none = some _，矛盾
  have h : (none : Option ActiveTail) = some { provDir := MoveDir.U, anchor := 0 } :=
    congrArg ParseState.active hcontra
  exact nomatch h

/--
  ★抽象「合法解析」谓词 `LegalParse`（L0，无 Θ_parse 的「仅约束、不选择」侧）：

  `LegalParse Legal h D` 表示「`D` 满足一个**给定的合法性约束** `Legal`」——`Legal` 是
  **任意**关系参数，**不含** Θ_parse 的确定性选择。这是「去掉 Θ_parse」的形式化：合法性是
  「约束」（多个 D 可同时合法），不是「选择器」（选出唯一 D）。选择器是 Θ_parse 的工作（§2-§4）。

  ★诚实标注（codex 自审，避免声明膨胀 090 号）：`Legal` 是**抽象关系参数**，**不是**缠论公理的
    具体实例（本文件**不**实例化「包含/分型/笔/线段/中枢公理约束」为具体 `Legal`——那属下游）。
    因此基于它的多义性见证（下方）是**抽象玩具见证**，**不**是「具体缠论合法解析多值」的证明。
-/
def LegalParse (Legal : List Kline → ParseState → Prop) (h : List Kline) (D : ParseState) : Prop :=
  Legal h D

/--
  ★★存在一个允许多值的合法性约束（L0，抽象玩具见证，非具体缠论多义证明）：

  **存在**一个约束谓词 `Legal` 与历史 `h`，使 **≥ 2 个不同状态** `D₁ ≠ D₂` 都 `LegalParse Legal h`——
  即「仅靠约束 + 无 Θ_parse 选择器」时，合法解析**可以**多值。

  ★诚实强度（codex 自审，关键降级）：本定理证的是**存在性**——「**存在一个**宽松约束允许多值」，
  **不是**「**所有**缠论合法性约束都多值」，也**不是**「具体缠论公理（包含/分型/笔/线段/中枢）下
  多值」（那需把缠论公理实例化为具体 `Legal`，本文件**未做**，属下游）。它的诚实内容仅为：
  **唯一性不能仅从「约束谓词」一般地推出**——至少需要确定性选择器（Θ_parse）才能从「可多值的约束」
  收缩到单值。这**足以**说明 `parse_unique` 的唯一性**不是约束本身自带的**（来自 Θ_parse 的选择器），
  但**不足以**精确否证「具体缠论公理无 Θ 时必多值」（后者是更强命题，未证）。

  ★与 `Strict.Decomp` 的**注释层对照**（任务要求，非 import 桥接）：本见证与 Decomp.lean 的
  `decomp_ambiguity_witness`（gauge 前 `|𝒟ₙ(h)/≈ₙ| ≥ 2`，T₅₂ 真多义）**同构于「未固定 gauge 则多值」**
  的结构模式；§2-§4 的 `ParseParams` 确定性选择器与 `Decomp.gaugeFix`/`GaugeNormal`（按 level 最小、
  平级最左的纤维内确定性截面选择）**同构于「gauge-fixing 钉单值」**。★但本文件**不 import** Decomp，
  对照仅在注释层——Decomp 的多义见证用的是具体 `Move`/`StructEqN`，本文件用抽象 `Legal`，
  二者是**结构同构**而非**形式桥接**（建形式桥接需 import Decomp 并构造 ParseState↔Move 映射，未做）。
-/
theorem parse_ambiguous_without_theta :
    ∃ (Legal : List Kline → ParseState → Prop) (h : List Kline) (D₁ D₂ : ParseState),
      LegalParse Legal h D₁ ∧ LegalParse Legal h D₂ ∧ D₁ ≠ D₂ := by
  -- Legal := 「任何 confirmed 为空的状态都合法」——这模拟「缠论公理只约束 confirmed 部分，
  -- 不钉死 active 尾部的方向」⟹ 同一历史下，active=none 与 active=some(...) 都合法（多义）。
  refine ⟨
    fun _ D => D.confirmed = [],
    [],
    { confirmed := [], active := none },
    { confirmed := [], active := some { provDir := MoveDir.U, anchor := 0 } },
    rfl, rfl, ?_⟩
  -- 两状态 active 字段不同（none ≠ some _）⟹ 不等
  intro hcontra
  have h : (none : Option ActiveTail) = some { provDir := MoveDir.U, anchor := 0 } :=
    congrArg ParseState.active hcontra
  exact nomatch h

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 confirmed / active 区分（615 标准第5部分，结构同构对接 ≠ 形式桥接）

  把「confirmed 交易可用、active 仅预警」的裁决落实为可下游引用的投影 + 见证定理。

  ★诚实标注（codex 自审，避免膨胀）：本节实现的是 **confirmed/active 的类型层切分**
    （`ParseState` 两字段 + 投影 + 互补定理 + active 临时性见证）。它与
    `Strict.OpenTailSystem`（confirmed ≅ 已闭合走势序列，active ≅ `current` 当下状态，
    未来延伸 ≅ `Ext`=⊔ⱼBⱼ 分支集）是**结构同构对接**——但本文件**不 import** OpenTail，
    **未**构造 `ActiveTail ↔ OpenTailSystem.current` 的形式映射，也**未**证未来分支集的
    互斥穷尽（那由 OpenTail.lean 的 `branch_total`/`branch_disjoint` 承载，不在本文件重证）。
    「对接」= 概念结构一致（注释层），**非** Lean 形式桥接（待下游建 import 映射）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★交易可用部分投影 `tradableMoves`（L0，615 第5部分）：解析状态的**已完成走势序列**。
  交易决策**只读** confirmed（已闭合走势），**不读** active（未完成尾部）。
-/
def tradableMoves (D : ParseState) : List ConfirmedMove := D.confirmed

/--
  ★预警尾部投影 `warningTail`（L0，615 第5部分）：解析状态的**未完成尾部**（`Option ActiveTail`）。
  active 尾部**仅预警**（提示可能形成的走势），**不**进入交易决策。
-/
def warningTail (D : ParseState) : Option ActiveTail := D.active

/--
  ★confirmed/active 投影互补（L0）：`tradableMoves` 与 `warningTail` 是 ParseState 的两个独立字段，
  解析状态由「交易可用 confirmed + 仅预警 active」**完全分解**（无第三部分）。

  这坐实 615 第5部分裁决的结构落实：当下能唯一确定的（confirmed，交易用）与未来未定的
  （active，分支集预警）在类型层**切开**——`ParseState.mk (tradableMoves D) (warningTail D) = D`。
-/
theorem parse_state_split (D : ParseState) :
    ParseState.mk (tradableMoves D) (warningTail D) = D := by
  cases D
  rfl

/--
  ★active 尾部不强行定性（L0，615 第5部分核心修正的见证）：

  存在解析状态 `D` 其 `active = some τ`（有未完成尾部），但该 τ 的 `provDir` 是**临时**方向——
  存在另一组「相同 confirmed 但不同 active provDir」的状态，见证 active 部分的「最终方向」
  **在结构上不被本类型唯一定性**（同一 confirmed 可配不同 active provDir）。

  ★诚实强度（codex 自审）：本定理是**存在性见证**——「存在两个 confirmed 相同、active provDir
  不同的 ParseState」。它**不**证「同一历史 + 同一 Θ_parse 下 active 可多值」（那与 `parse_unique`
  矛盾——给定 θ 后 active 也唯一）；它证的是**更弱**的命题：`ActiveTail.provDir` 作为类型字段
  **不被 confirmed 字段决定**（两者独立），故「active 的临时方向」是一个**独立自由度**，与
  「最终走势方向需未来延伸才定」的语义一致。这与 `Strict.OpenTailSystem` 的 `current`（当下状态）
  /分支集结构**同构**（注释层对照，非 import 桥接，见 §6 标题诚实标注）。

  ★与 §5/§4 的关系：confirmed 部分（交易用）由 Θ_parse 唯一钉死（`parse_unique`，给定 θ）；
  本定理说的是**跨状态**的结构自由度（active 字段独立于 confirmed 字段），不是同一 (θ,h,ℓ) 下的
  多值——后者不存在（`parse_unique`）。两者不矛盾：唯一性是「给定输入定输出」，自由度是「字段间
  无强制依赖」。
-/
theorem active_tail_provisional :
    ∃ (cf : List ConfirmedMove) (D₁ D₂ : ParseState),
      D₁.confirmed = cf ∧ D₂.confirmed = cf ∧
      D₁.active.isSome = true ∧ D₂.active.isSome = true ∧
      D₁.active ≠ D₂.active := by
  -- 相同 confirmed=[]，但 active 的临时方向不同（U vs D）——未完成尾部方向未定（仅预警）
  refine ⟨
    [],
    { confirmed := [], active := some { provDir := MoveDir.U, anchor := 0 } },
    { confirmed := [], active := some { provDir := MoveDir.D, anchor := 0 } },
    rfl, rfl, rfl, rfl, ?_⟩
  -- 两 active 的 provDir 不同（U ≠ D）⟹ active 字段不等
  intro hcontra
  -- some {U,0} = some {D,0} ⟹ {U,0} = {D,0} ⟹ U = D，矛盾
  have h1 : ({ provDir := MoveDir.U, anchor := 0 } : ActiveTail)
          = { provDir := MoveDir.D, anchor := 0 } := Option.some.inj hcontra
  have h2 : MoveDir.U = MoveDir.D := congrArg ActiveTail.provDir h1
  exact MoveDir.noConfusion h2

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 标签声明（615 gatekeeper）——禁标 TrueCompleteClassification

  把解析唯一性的认识论标签钉为可下游引用的结构——主标签 StructurePartitionOnly，
  唯一性前件含 ParseParams（ParameterizedUniqueness），禁标 TrueCompleteClassification。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★解析标签种类 `ParseTag`（615 gatekeeper，诚实分层）。

  - `StructurePartitionOnly`：结构划分（解析给结构骨架 D_{ℓ,t}），**不是真完全分类**
    （不自动给语义不变量下的双射完备性——那需 `Strict.Classifies` 的五项证明义务）。
  - `ParameterizedUniqueness`：参数化唯一性（唯一性前件含 ParseParams，**非无条件唯一**）。
  - `ConfirmedActiveSplit`：confirmed/active 区分（交易用 confirmed，预警用 active，615 第5部分）。

  ★**没有** `TrueCompleteClassification` 构造子——类型层就拒绝把解析唯一性标为真完全分类。
  解析唯一性是「缠论公理 + Θ_parse」的产物（条件唯一），不是缠论内部的无参数分类定理。
-/
inductive ParseTag where
  | StructurePartitionOnly
  | ParameterizedUniqueness
  | ConfirmedActiveSplit
deriving DecidableEq, Repr

/--
  ★解析子类标记 `ParseSubkind`（615 gatekeeper）：UniqueGivenThetaParse。
  唯一子类——给定 Θ_parse 的唯一递归解析。**没有** TrueCompleteClassification 子类（类型层拒绝冒充）。
-/
inductive ParseSubkind where
  | UniqueGivenThetaParse
deriving DecidableEq, Repr

/--
  ★解析的诚实标签包 `parseLabels`（L0 声明）：
  主标签 StructurePartitionOnly + ParameterizedUniqueness + ConfirmedActiveSplit，
  子类 UniqueGivenThetaParse。这是 Parse.lean 产出的认识论自我声明——下游引用此标签即知
  「这是给定 Θ_parse 的结构划分唯一性，非缠论无参数真完全分类，唯一性前件含 Θ_parse」。
-/
def parseLabels : List ParseTag × ParseSubkind :=
  ([ParseTag.StructurePartitionOnly, ParseTag.ParameterizedUniqueness, ParseTag.ConfirmedActiveSplit],
   ParseSubkind.UniqueGivenThetaParse)

/--
  ★禁标真完全分类（L0，615 gatekeeper 见证）：解析的子类标记必是 UniqueGivenThetaParse。
  `ParseSubkind` 只有 `UniqueGivenThetaParse` 一个构造子——任意子类标记必是它。

  ★诚实标注强度（codex 自审，避免膨胀）：这是**本文件本地标签类型**的平凡枚举定理——
  它保证「在 `ParseSubkind` 这个类型里，解析唯一性不可能被标成真完全分类」（因 `ParseSubkind`
  根本不含 TrueCompleteClassification 构造子）。它**不是**「全项目/跨库禁止冒充」的强保证
  （`TrueCompleteClassification` 是 `Strict.Classifies` 域的概念，不在此枚举内）。本定理的诚实
  内容：本文件的标签 API **不提供**冒充真完全分类的途径——解析唯一性自我声明为
  UniqueGivenThetaParse（给定 Θ_parse 的条件唯一），而非缠论无参数分类定理。
-/
theorem parse_not_true_classification (k : ParseSubkind) :
    k = ParseSubkind.UniqueGivenThetaParse := by
  cases k; rfl

/--
  ★解析唯一性非无条件（L0，617 精化的标签层见证）：

  唯一性标签必是 `ParameterizedUniqueness`（前件含 Θ_parse），**不是**无条件唯一。
  本文件证「∃ θ ⟹ 唯一」（`parse_unique`，前件含 θ）+「无 θ 则多值」（`parse_ambiguous_without_theta`），
  二者合起来精确兑现「解析唯一性 = 缠论公理 + Θ_parse」——故标签必是参数化唯一性。

  ★诚实强度：这是本地标签类型的见证——`ParameterizedUniqueness` ∈ `parseLabels.1`。它把
  「唯一性条件依赖 Θ_parse」这一**已证内容**（parse_unique 前件含 θ + parse_ambiguous_without_theta）
  反映到标签层，使下游引用 `parseLabels` 即知唯一性非无条件。
-/
theorem parse_uniqueness_is_parameterized :
    ParseTag.ParameterizedUniqueness ∈ parseLabels.1 := by
  simp [parseLabels]

/-! ════════════════════════════════════════════════════════════════════════
  ## §8 有效域诚实标注总结（formalization-validity-domain + no-patch-mentality）

  本文件**忠实交付**（L0，machine-checked）：
  1. 结构骨架：`MoveDir`（U/D/P）+ `ConfirmedMove`/`ActiveTail`/`ParseState`（D_{ℓ,t}）。
  2. Θ_parse 参数化：`ParseParams`（baseParse B_Θ + recogStep Φ_{ℓ,Θ}，确定性选择器抽象签名，
     具体边界规则**不全部实例化**，诚实标 Θ-参数化）。
  3. 递归解析：`parse`（D₀=B_Θ(h)，D_{ℓ+1}=Φ_{ℓ,Θ}(D_ℓ)）+ `parse_zero`/`parse_succ` 展开。
  4. ★核心定理 `parse_unique`（给定 Θ_parse ⟹ ∃! D_{ℓ,t}）+ `parse_deterministic`
     （确定性，θ 固定为前提）。
  5. ★诚实多义性：`parse_varies_with_theta`（不同 θ 给不同解析，**实证**）+
     `parse_ambiguous_without_theta`（**存在**一个宽松约束允许多值 ≥ 2，**抽象玩具见证**——
     非「具体缠论公理无 Θ 时必多值」的证明；与 `Strict.Decomp.decomp_ambiguity_witness`
     **结构同构**，注释层对照，非 import 形式桥接）。
  6. confirmed/active 区分（615 第5部分，**类型层切分** + 见证）：`tradableMoves`/`warningTail`/
     `parse_state_split`（状态完全分解为「交易用 confirmed + 预警 active」）+
     `active_tail_provisional`（active provDir 字段独立于 confirmed，存在性见证）。
     ★与 `Strict.OpenTailSystem` 是**结构同构对接**（注释层），**未** import OpenTail、
     **未**建 `ActiveTail ↔ current` 形式映射、**未**重证分支集互斥穷尽（见 §6 标题诚实标注）。
  7. 标签声明：`parseLabels`（StructurePartitionOnly + ParameterizedUniqueness +
     ConfirmedActiveSplit + UniqueGivenThetaParse）+ `parse_not_true_classification`
     （禁标 TrueCompleteClassification）+ `parse_uniqueness_is_parameterized`（唯一性非无条件）。

  本文件**不声明**（有效域边界，诚实标注，避免膨胀）：
  - ✗ 解析唯一性是缠论无参数真理——`parse_unique` 前件**必含** ParseParams（Θ_parse）；
       `parse_ambiguous_without_theta` 证去掉 Θ_parse 则多值。唯一性 = **缠论公理 + Θ_parse**
       （617 精化），非缠论内部无参数分类定理。
  - ✗ Θ_parse 由缠论唯一确定——`parse_varies_with_theta` 证不同 θ（不同边界约定）给不同解析，
       缠论公理**不**唯一钉死 θ。包含合并/极值取舍/笔线段边界/中枢开闭/canonical 选择器
       全部 Θ-参数化，**待实例化**（本文件只证「给定 θ ⟹ 唯一」，不证「θ 取值由缠论唯一确定」）。
  - ✗ `ParseParams` 的具体边界规则已实例化——`baseParse`/`recogStep` 是**抽象确定性选择器签名**，
       具体算法（包含/分型/笔/线段/中枢的判定细节）**不在本文件实例化**（Θ-参数化设计选择）。
  - ✗ 解析是真完全分类——`parse_not_true_classification` 证**禁标** TrueCompleteClassification。
       解析给**结构划分**（StructurePartitionOnly），不自动给语义不变量下的双射完备性
       （后者需 `Strict.Classifies` 的 total/sound/complete/disjoint/realized 五项，本文件不证）。
  - ✗ active 尾部可定最终走势——`active_tail_provisional` 证 active 方向**临时不定**（仅预警），
       未来延伸落入哪个最终走势是分支集（`Strict.OpenTailSystem.BranchPred`，本文件不预判）。

  ★与现有 Strict 库的关系：本文件**只** import `Strict.Classification`（复用 ParseState 等结构
    定义范式 + `Strict.RecursiveKernel`/`OpenTailSystem` 的概念）。对 `Strict.Decomp`（多义/gaugeFix）
    与 `Strict.OpenTail`（current/Ext/BranchPred）的引用**仅在注释层**（结构同构对照），**不** import
    它们，**未**建任何 ParseState↔Move / ActiveTail↔current 的形式桥接（保持最小依赖；建桥待下游）。
    本文件是 π_Θ 推导**第2步**（唯一递归解析），上承第1步分类（Classification/ClassificationFamily），
    下接第3步策略族（StrategyFamily）——三步共同构成「分类 → 解析 → 策略」的 π_Θ 链。

  ── 结果包六要素（result-package 强制） ──
  1. **结论**：Parse.lean 形式化 π_Θ 第2步唯一递归解析，核心定理 `parse_unique`
     （给定 Θ_parse ⟹ ∃! D_{ℓ,t}，前件含 ParseParams，无无参数冒充）；confirmed/active 在类型层
     切分（与 615 第5部分 OpenTail **结构同构对接**，非 import 形式桥接）；全文件 L0 无 sorry/axiom。
  2. **定义依据**：standard §第2部分递归方程（D₀=B_Θ(h)，D_{ℓ+1}=Φ_{ℓ,Θ}(D_ℓ)）；§第5部分
     confirmed/active 裁决（交易只用 confirmed，active 仅预警）；617 精化（唯一性=公理+Θ_parse）。
  3. **边界条件**：唯一性**翻转**当且仅当去掉 Θ_parse 前件——定理 `parse_unique` 的第一参数即
     `θ : ParseParams`，去掉则 `parse` 无定义（baseParse/recogStep 是 θ 字段）。
     `parse_ambiguous_without_theta` **进一步**给出（抽象玩具见证）：仅靠约束谓词、无确定性选择器时
     合法解析**可**多值（≥2）——故唯一性来自 Θ_parse 选择器，非约束自带。★诚实：此见证是**存在性**
     （存在一个宽松约束多值），**非**「具体缠论公理无 Θ 必多值」的证明。
  4. **下游推论**：π_Θ 第3步（StrategyFamily）的输入 S（结构状态）= 本文件 `parse θ h ℓ` 的输出
     D_{ℓ,t}；交易决策只读 `tradableMoves D`（confirmed），不读 active；Θ_parse 与 StrategyFamily
     的 Θ（风险公理）是**不同**的 Θ（前者管解析边界，后者管执行风险），下游不可混用。
  5. **谱系引用**：615（Layer1⊊Layer2 概念分离，本文件标 StructurePartitionOnly 非真完全分类）；
     617（唯一性=缠论公理+Θ_parse 精化，本文件核心兑现）；T₅₂（真多义性，对照 Decomp.lean
     `decomp_ambiguity_witness`）。
  6. **影响声明**：新增 `formal/Strict/Parse.lean`（不改任何现有文件，不碰 lakefile）；提供
     π_Θ 第2步的结构骨架 + Θ_parse 参数化 + parse_unique 供下游（StrategyFamily 等）引用。
-/

end Strict.Parse
