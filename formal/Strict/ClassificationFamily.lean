/-
  Strict/ClassificationFamily.lean — C_Θ 分类器族 + fiber 原像划分（L0）
  ★cc-classfamily 工位（task #70；598/603/615 谱系 + 编排者推导链精化 + codex b7pb965r4 收敛）

  topo_address: swarm/严格完全分类/classfamily ｜ parent: Lead

  ── 核心理论结论（编排者 + codex 收敛） ──
  推导链 = **缠论结构公理 + Θ ⟹ C_Θ ⟹ fiber 原像划分（互斥穷尽自动）⟹ π_Θ**。

  关键洞察：**连分类器 C_Θ 本身都是 Θ 参数化的**，不只策略 π 需要 Θ。缠论结构公理
  单独**连唯一分类都给不出**——需要 Θ_parse（边界 / canonical 选择器）来固定「在哪里切、
  取哪个规范分解」。一旦固定 Θ，C_Θ : X → State θ 是 Lean 全函数，其 fiber
  自动构成 X 的互斥穷尽划分 X = ⊔_s C_Θ⁻¹(s)。

  本文件是 `Strict/StrategyFamily.lean` 的**上游对偶**：
  - StrategyFamily 证「给定 Θ ⟹ π_Θ（策略）全定义/唯一/因果」（标签**外**的执行语义）。
  - 本文件证「分类器 C 本身 Θ-参数化 ⟹ C_Θ 全定义/唯一 + fiber 自动互斥穷尽划分」
    （标签**本身**的产生机制）。
  两者合起来兑现完整推导链：Θ ⟹ C_Θ ⟹ fiber partition ⟹ π_Θ。

  ── 诚实标注（formalization-validity-domain + 615 精化，关键） ──
  原像划分**只证 partition（结构事实），不证**标签的缠论语义 / 递归正确 / 因果无前视。
  - 「fiber 互斥穷尽」是任何全函数的**纯类型论事实**（函数图平凡侧），不依赖 C 内容。
  - 逐 claim 证明（Trend.lean / Center.lean / BSP.lean / Decomp.lean）仍负责给 C_θ 字段的
    **缠论语义内容**——本文件给「框架」（partition 总成立），逐 claim 给「语义」（标签
    真的对应趋势/中枢/买卖点），二者**互补**，本文件**不冒充**逐 claim 的语义内容。
  - **realized 不自动**：若分类目标用 `im (C θ)`（值域）则每类非空自动；若用外部预设的
    大标签集 `State θ` 则可能有**空 fiber**，realized（每标签可实现）须**另证**，本文件诚实
    标此区分（`fibers_cover_image` 自动 vs `Realized` 须另证）。
  - **C_θ 唯一性条件依赖 Θ_parse**：`Classification.lean` 的 `DecompositionSystem.D : X → T`
    预设了一个**规范分解函数**，这不是缠论的无参数唯一性——它是一个 Θ_parse 选择
    （= `Decomp.lean` 的 GaugeNormal / gaugeFix 截面）。本文件注释引 Decomp.lean 的
    「gauge 前多义 / gaugeFix 后截面唯一」作证：唯一性是规范化选择**之后**才有的。

  ── 分层标注（formalization-validity-domain 强制） ──
  - **L0 可证**（machine-checked，本文件全部交付）：函数图全定义/唯一、fiber 互斥穷尽、
    fiberSetoid 等价、fiber 覆盖值域。这些是**纯类型论**事实，零数据依赖。
  - **Θ 参数**（非缠论可导，extra-缠论设计选择）：parse / level / signal / voice / risk /
    exec 的**具体选择**——在哪切边界、用哪级别、哪个信号阈值、哪种声部规则。这些**不由**
    缠论结构公理推出，是 Θ 的内容。本文件把它们参数化为 `Param`，**不**声称 Param 由缠论导出。
  - **L3 经验**（真实数据验证，不由本文件 L0 声称）：收益 / 最优性 / 阈值稳定性 / 成本模型。

  ★标签（gatekeeper）：**StructurePartitionOnly**（fiber 原像划分是**结构事实**，非语义分类）
        + **Θ-参数化前件**（分类器本身 Θ-参数化，缠论公理单独连唯一分类都给不出）。
  ★这是 **615 Layer2 的 Θ-参数化精化**，**非新真完全分类**——它不主张「证明了缠论完全分类」，
    只主张「任何固定 Θ 后的全函数 C_Θ 自动给 fiber partition；缠论唯一分类的前件是 Θ，非无参数」。

  范式：纯 Prop/Type，不依赖 Mathlib（继承 Strict 库自包含约束，`Equivalence` 来自 Lean core）。
  禁 sorry/admit/axiom。验证：`cd formal && lake env lean Strict/ClassificationFamily.lean`。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴）→ 615（Layer1 ⊊ Layer2）→
        616（完全分类 ⊬ 唯一策略，⊢ 策略族）→ 617 候选（C 与 π 都 Θ-参数化）。
-/

import Strict.Classification

namespace Strict.ClassificationFamily

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 ClassifierFamily：分类器是 Θ 参数化族（核心理论结论的 L0 兑现）

  **连分类器 C_Θ 本身都是 Θ 参数化的**。`ClassifierFamily X Param` 把「分类器」升级为
  按参数 `Param` 索引的族——每个 `θ : Param` 给出一个状态类型 `State θ` 与一个**全函数**
  分类器 `C θ : X → State θ`。状态类型 `State : Param → Type` 本身依赖 θ：不同 Θ_parse /
  Θ_level 选择给出**不同的标签集**（如不同级别下中枢三态 vs 多态），故 `State θ`，非固定 `S`。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分类器族 `ClassifierFamily X Param`（编排者推导链，L0）：

  - `State : Param → Type`：参数化状态/标签类型——**每个 θ 给一个标签集** `State θ`。
    （不同 Θ_parse / Θ_level 选择诱导不同标签集；故状态类型依赖 θ，不是固定 `S`。）
  - `C : (θ : Param) → X → State θ`：**Θ 参数化的全函数分类器族**——给定 θ，`C θ` 是
    对象类型 `X` 到标签集 `State θ` 的**全函数**（每个对象都被分类，无遗漏）。

  ★这是「分类器本身 Θ-参数化」的精确形式：缠论结构公理**不**单独给出 `C`（连唯一分类都
  给不出），必须配一个 Θ ∈ Param（边界 / canonical 选择器）才把分类器钉死为 `C θ`。
  Param **不由缠论导出**（Θ-参数化前件），但**给定** θ 后 `C θ` 是确定全函数（L0）。
-/
structure ClassifierFamily (X Param : Type) where
  State : Param → Type
  C : (θ : Param) → X → State θ

/--
  ★★分类器全定义 + 唯一（L0，编排者推导链「全定义义务」）：
  `classifier_total_unique` — 给定 θ，`C θ` 对每个对象**全定义且唯一**。

  对任意族 `F`、参数 `θ`、对象 `x`，**存在唯一**标签 `s` 使 `F.C θ x = s`。

  这是「完全分类的『全定义』义务」的 L0 兑现——每个对象 x 在分类器 C_θ 下得到
  恰好一个标签（全定义：∃；唯一：函数确定性）。

  ★诚实标注证明强度（避免声明膨胀，no-patch-mentality 禁令5）：这是**函数图的平凡侧**
  ——`F.C θ` 是 Lean 全函数，故「存在标签」（= `F.C θ x` 本身）与「唯一」（= 函数确定性）
  **自动成立**，证明体仅 `rfl` + 对称。它**不是**「证明了缠论分类完全」的实质命题，而是
  「一旦把分类器定为全函数，每对象恰好一个标签」的接口兑现。**实质的语义内容**（标签
  真对应缠论结构）由逐 claim 文件（Trend/Center/BSP/Decomp）承载，**不**由本定理兑现。
  本定理只显式列出「全定义」这一完全分类的必要义务，使其可被下游引用 / 质询。
-/
theorem classifier_total_unique {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) (x : X) :
    ∃ s, F.C θ x = s ∧ ∀ s', F.C θ x = s' → s' = s :=
  ⟨F.C θ x, rfl, fun _ heq => heq.symm⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 fiber 原像划分：最干净的标准① 互斥穷尽（任何全函数自动给 partition）

  给定任意固定的全函数 `C : X → S`，其 fiber `C⁻¹(s) = {x | C x = s}` 自动构成 `X` 的
  **互斥穷尽划分** `X = ⊔_s C⁻¹(s)`：
  - **覆盖**（穷尽 / total）：每个 x 落在某个 fiber（`s = C x`）。
  - **互斥**（disjoint）：若 x 同时落在 fiber s₁ 与 s₂，则 s₁ = s₂（不重叠）。
  - **等价关系**：`FiberRel C x y := C x = C y` 是等价关系，诱导商集 `X / ∼ ≅ im C`。

  ★这是「任何全函数 C_θ 的 fiber 自动互斥穷尽划分 X = ⊔ C⁻¹(s)」——**纯类型论事实**，
  不依赖 C 的内容（缠论语义无关）。这正是「partition 给框架，逐 claim 给语义」的框架侧。

  ★工具定义在**固定全函数** `C : X → S`（S 固定）上；§3 再实例化到族成员 `F.C θ`
  （此时 S = `F.State θ`）。这样划分工具复用于任意 θ，不被依赖类型 `State θ` 绊住。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★fiber（原像 / 纤维）`Fiber C s x`（L0）：对象 `x` 落在标签 `s` 的 fiber ⟺ `C x = s`。
  即 `Fiber C s = C⁻¹(s) = {x | C x = s}`（标签 s 的原像集，以谓词形式）。
-/
def Fiber {X S : Type} (C : X → S) (s : S) (x : X) : Prop :=
  C x = s

/--
  ★fiber 覆盖（穷尽 / total，L0，标准① 的「穷尽」侧）：
  `fiber_total` — 每个对象至少落在一个 fiber（覆盖 X，0 遗漏）。

  `∀ x, ∃ s, Fiber C s x`——取 `s = C x` 即得（对象的标签就是它落入的 fiber）。
  这是「划分覆盖全空间」的 L0 形式：X = ⋃_s C⁻¹(s)。
-/
theorem fiber_total {X S : Type} (C : X → S) (x : X) :
    ∃ s, Fiber C s x :=
  ⟨C x, rfl⟩

/--
  ★fiber 互斥（disjoint，L0，标准① 的「互斥」侧）：
  `fiber_disjoint` — 一个对象不能落在两个不同标签的 fiber（不重叠，>1 重叠的否定）。

  `Fiber C s₁ x → Fiber C s₂ x → s₁ = s₂`——若 `C x = s₁` 且 `C x = s₂`，则 s₁ = s₂
  （函数值唯一）。这是「不同 fiber 不相交」的 L0 形式：s₁ ≠ s₂ ⟹ C⁻¹(s₁) ∩ C⁻¹(s₂) = ∅。
-/
theorem fiber_disjoint {X S : Type} (C : X → S) {s₁ s₂ : S} {x : X} :
    Fiber C s₁ x → Fiber C s₂ x → s₁ = s₂ := by
  intro h1 h2
  exact h1.symm.trans h2

/--
  ★fiber 等价关系 `FiberRel C x y`（L0）：两对象同 fiber ⟺ `C x = C y`（同标签）。
  这是 fiber 划分诱导的等价关系——同标签的对象归为一类（商集 X / ∼ 的等价核）。
-/
def FiberRel {X S : Type} (C : X → S) (x y : X) : Prop :=
  C x = C y

/--
  ★★fiber Setoid（L0，划分 = 等价关系的兑现）：
  `fiberSetoid` — `FiberRel C` 是 `X` 上的等价关系（自反 / 对称 / 传递）。

  这把「fiber 互斥穷尽划分」表达为 Lean 标准的 `Setoid X`——任何全函数 C 的 fiber 划分
  **就是**一个等价关系，诱导商集 `Quotient (fiberSetoid C) ≅ im C`。这是「划分 ⟺ 等价关系」
  对应（集合论基本事实）的 L0 形式，**对任意全函数 C 自动成立**，不依赖 C 的缠论语义。

  ★用 `def`（非 `instance`）：`S`/`C` 是显式参数，无法由 typeclass synthesis 从返回类型
  `Setoid X` 推断（同一个 X 可有多个不同 C 的 fiberSetoid），故显式构造而非实例注册。
-/
def fiberSetoid {X S : Type} (C : X → S) : Setoid X where
  r := FiberRel C
  iseqv := ⟨fun _ => rfl, fun h => h.symm, fun h1 h2 => h1.trans h2⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 族成员的 fiber 划分 + realized 的诚实区分（codex 收敛关键）

  把 §2 的 fiber 工具实例化到族成员 `F.C θ : X → F.State θ`，得到「给定 θ ⟹ C_θ 的 fiber
  自动互斥穷尽划分 X」。同时**诚实区分** realized 的两种情形：
  - 用值域 `im (F.C θ)`：每类**非空自动**（`fibers_cover_image`，L0 自动）。
  - 用外部预设大标签集 `F.State θ`：可能有**空 fiber**，realized **须另证**（`Realized` 谓词，
    本文件**不**默认它成立——诚实标注它非自动）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★族成员 fiber `θFiber F θ s x`（L0）：对象 `x` 落在族成员 `F.C θ` 的标签 `s` 的 fiber。
  即把 §2 的 `Fiber` 实例化到固定 θ 的全函数 `F.C θ : X → F.State θ`。
-/
def θFiber {X Param : Type} (F : ClassifierFamily X Param)
    (θ : Param) (s : F.State θ) (x : X) : Prop :=
  Fiber (F.C θ) s x

/--
  ★★族成员 fiber 划分（L0，编排者推导链「fiber partition」侧的兑现）：
  `theta_fiber_partition` — 给定 θ，`F.C θ` 的 fiber **互斥穷尽划分** X。

  返回三项：
  - 覆盖（total）：`∀ x, ∃ s, θFiber F θ s x`（每对象落某 fiber）。
  - 互斥（disjoint）：`∀ x s₁ s₂, θFiber F θ s₁ x → θFiber F θ s₂ x → s₁ = s₂`。

  这兑现「**任何全函数 C_θ 的 fiber 自动互斥穷尽划分** X = ⊔_s C_θ⁻¹(s)」——纯类型论事实，
  对每个 θ 成立，不依赖 C_θ 的缠论语义内容（partition 给框架，逐 claim 给语义）。
-/
theorem theta_fiber_partition {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) :
    (∀ x, ∃ s, θFiber F θ s x) ∧
    (∀ x (s₁ s₂ : F.State θ), θFiber F θ s₁ x → θFiber F θ s₂ x → s₁ = s₂) :=
  ⟨fun x => fiber_total (F.C θ) x,
   fun _ _ _ h1 h2 => fiber_disjoint (F.C θ) h1 h2⟩

/--
  ★★fiber 覆盖值域 ⟹ 值域内每类非空（L0，realized 的**自动**侧）：
  `fibers_cover_image` — 若把分类目标取为**值域** `im (F.C θ)`（即「实际出现过的标签」），
  则每个这样的标签 `s = F.C θ x` 的 fiber **非空自动**（x 自己是见证）。

  形式：`∀ x, ∃ x', θFiber F θ (F.C θ x) x'`——对值域中任一标签 `F.C θ x`，存在对象 x'
  （取 x' = x）落入其 fiber。故**限制到值域时 realized 自动成立**（无空类）。

  ★诚实标注（codex 收敛）：这**只在分类目标 = 值域 `im (F.C θ)` 时**自动。它**不**保证
  外部预设大标签集 `F.State θ` 的**每个** s 都非空——见 `Realized` 谓词，那须另证。
-/
theorem fibers_cover_image {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) (x : X) :
    ∃ x', θFiber F θ (F.C θ x) x' :=
  ⟨x, rfl⟩

/--
  ★可实现谓词 `Realized F θ`（L0 谓词，realized 的**须另证**侧）：
  `Realized F θ` ⟺ `F.State θ` 的**每个**标签 s 都有对象见证（`∀ s, ∃ x, F.C θ x = s`）。

  ★诚实标注（codex 收敛，关键区分 vs `fibers_cover_image`）：
  - 若分类目标 = **值域** `im (F.C θ)`，realized 由 `fibers_cover_image` **自动**给出。
  - 若分类目标 = **外部预设大标签集** `F.State θ`（如「中枢三态」预设了所有理论上可能的态，
    但某态在某 θ 下从不出现），则可能有**空 fiber**，`Realized F θ` **不自动成立，须另证**。
  本文件把 `Realized` 定义为**显式谓词**而**不**证明它对任意 F/θ 成立——这是诚实标注：
  realized 是分类目标选择（im vs 大标签集）的函数，不是 partition 的自动推论。
  下游若用大标签集，**必须**为其 C_θ 单独证 `Realized F θ`（提供每标签的对象见证）。
-/
def Realized {X Param : Type} (F : ClassifierFamily X Param) (θ : Param) : Prop :=
  ∀ s : F.State θ, ∃ x, F.C θ x = s

/--
  ★值域分类目标下 Realized 自动（L0，封口 `fibers_cover_image` 的全称侧）：
  `realized_on_image` — 若把标签集**就取为值域**（每个出现的标签都来自某对象），则
  对「形如 `F.C θ x` 的标签」Realized 条件自动满足。

  本定理形式化「限制到值域 ⟹ 每个（出现的）标签可实现」：对任意 `x`，标签 `F.C θ x`
  有见证 `x`。这是 `fibers_cover_image` 的等价重述（值域内 realized 自动），与 `Realized`
  对**全** `F.State θ` 的要求形成对照——后者对外部大标签集**不自动**。
-/
theorem realized_on_image {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) (x : X) :
    ∃ x', F.C θ x' = F.C θ x :=
  ⟨x, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 Θ_parse 唯一性前件（C_θ 唯一性依赖规范化选择器，615 精化）

  C_θ 的「唯一性」**不是**缠论的无参数唯一性——它依赖一个 Θ_parse（规范分解选择器）。
  `Classification.lean` 的 `DecompositionSystem.D : X → T` 预设了一个规范分解函数 `D`，
  这个 `D` **就是**一个 Θ_parse 选择（= `Decomp.lean` 的 GaugeNormal / gaugeFix 截面）。
  本节把「分类器唯一性以一个选择器 D 为前件」形式化为 `ParseDependentClassifier`，
  诚实标注：唯一性是规范化选择**之后**才有的（gauge 前多义，gaugeFix 后截面唯一）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Θ_parse 依赖的分类器 `ParseDependentClassifier X Cand Param`（615 精化，L0）：

  分类器的唯一性**以一个 parse 选择器为前件**。结构：
  - `candidates : X → Cand → Prop`：对象 x 的**候选**分解集（gauge 前可能**多义**——
    多个 Cand 都是 x 的合法候选，对应 Decomp.lean「gauge 前真多义性」）。
  - `parse : Param → X → Cand`：**Θ_parse 选择器族**——给定 Θ ∈ Param，从候选中选出
    **规范**分解 `parse θ x`（= GaugeNormal / gaugeFix 截面，「level 最小、平级最左」等）。
  - `parse_valid`：选出的规范分解确实是候选之一（`candidates x (parse θ x)`）——
    选择器不凭空造分解，只在已有候选中选。

  ★诚实标注（615 精化 + Decomp.lean 引证）：
  - 唯一规范分解 `parse θ x` 的**唯一性来自 θ（选择器），不来自缠论无参数唯一性**。
    没有 θ，`candidates x` 可能多元（Decomp.lean 已证 gauge 前真多义，见该文件
    「gauge 前多义 / gaugeFix 后截面唯一」）。
  - 当前 `parse` 只抽象「存在一个选择器」；其**具体内容**（处理包含、相同极值取舍、
    开闭边界、未完成尾部）是 **Θ-参数化，非缠论可导**——本结构**不**声称 parse 由缠论唯一钉死。
-/
structure ParseDependentClassifier (X Cand Param : Type) where
  candidates : X → Cand → Prop
  parse : Param → X → Cand
  parse_valid : ∀ θ x, candidates x (parse θ x)

/--
  ★规范分解给定 Θ 后唯一（L0，615 精化「gaugeFix 后截面唯一」侧）：
  `parse_unique_given_theta` — 给定 Θ_parse 选择器 θ，规范分解 `parse θ x` 对每个 x **唯一**。

  `∀ x, ∃! t, parse θ x = t`（展开为 ∃ + 唯一）——这是「**截面唯一**」（gaugeFix 之后）的
  L0 形式。注意唯一性**条件于 θ**：固定选择器后唯一，**这正是「唯一性依赖 Θ_parse」**。

  ★诚实标注：这是函数图唯一（`parse θ` 是全函数），平凡侧；它的**内容不是**「缠论给出
  唯一分解」，而是「**一旦选定 Θ_parse 选择器** θ，规范分解就唯一」。**缠论单独**（无 θ）
  对应 `candidates x` 可能**多元**（Decomp.lean gauge 前多义），本结构不主张那是唯一的。
-/
theorem parse_unique_given_theta {X Cand Param : Type}
    (P : ParseDependentClassifier X Cand Param) (θ : Param) (x : X) :
    ∃ t, P.parse θ x = t ∧ ∀ t', P.parse θ x = t' → t' = t :=
  ⟨P.parse θ x, rfl, fun _ heq => heq.symm⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（gatekeeper：StructurePartitionOnly + Θ-参数化前件，诚实分层）

  把本文件产出的认识论标签钉为可下游引用的结构——**禁标 TrueCompleteClassification**。
  本文件证 partition（结构事实）+ C 的 Θ-参数化前件，**不**冒充逐 claim 的缠论语义内容。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分类器族产出标签种类 `FamilyTag`（gatekeeper，诚实分层）。

  - `StructurePartitionOnly`：**fiber 原像划分是结构事实**——任何全函数自动给互斥穷尽划分，
    **不是**缠论语义分类。这是本文件的主标签。
  - `ThetaParametricPremise`：**Θ-参数化前件**——分类器 C 本身 Θ-参数化，缠论结构公理
    单独连唯一分类都给不出（需 Θ_parse / canonical 选择器）。
  - `EmpiricalDomain`：经验有效域（收益 / 最优 / 阈值稳定 = L3，需真实数据，不由 L0 声称）。

  ★**没有** `TrueCompleteClassification` 构造子——类型层就拒绝把本文件产出标为真完全分类。
  本文件是 **615 Layer2 的 Θ-参数化精化**（C 与 π 都 Θ-参数化），**非新真完全分类**。
-/
inductive FamilyTag where
  | StructurePartitionOnly
  | ThetaParametricPremise
  | EmpiricalDomain
deriving DecidableEq, Repr

/--
  ★分类器族子类标记 `FamilySubkind`（gatekeeper）：Layer2ThetaRefinement。
  唯一子类——615 Layer2 的 Θ-参数化精化。**没有** TrueCompleteClassification 子类（类型层拒绝冒充）。
-/
inductive FamilySubkind where
  | Layer2ThetaRefinement
deriving DecidableEq, Repr

/--
  ★分类器族的诚实标签包 `classFamilyLabels`（L0 声明）：
  主标签 StructurePartitionOnly + 附标签 ThetaParametricPremise + 收益/最优 EmpiricalDomain，
  子类 Layer2ThetaRefinement。下游引用此标签即知「这是结构 partition + C 的 Θ-参数化前件，
  非真完全分类，缠论语义内容由逐 claim 文件给，收益性需 L3 验证」。
-/
def classFamilyLabels : List FamilyTag × FamilySubkind :=
  ([FamilyTag.StructurePartitionOnly, FamilyTag.ThetaParametricPremise, FamilyTag.EmpiricalDomain],
   FamilySubkind.Layer2ThetaRefinement)

/--
  ★禁标真完全分类（L0，gatekeeper 见证）：分类器族产出的子类标记必是 Layer2ThetaRefinement。
  `FamilySubkind` 只有 `Layer2ThetaRefinement` 一个构造子——任意子类标记必是它。

  ★诚实标注强度（避免膨胀）：这是**本文件本地标签类型**的平凡枚举定理——它保证「在
  `FamilySubkind` 这个类型里，本文件产出不可能被标成真完全分类」（因该类型不含
  TrueCompleteClassification 构造子）。它**不是**「全项目/跨库禁止冒充」的强保证。本定理的
  诚实内容：本文件标签 API **不提供**冒充真完全分类的途径——fiber partition 自我声明为
  StructurePartitionOnly（结构事实），C 的唯一性自我声明为 ThetaParametricPremise（依赖 Θ）。
-/
theorem family_not_true_classification (k : FamilySubkind) :
    k = FamilySubkind.Layer2ThetaRefinement := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 有效域诚实标注总结（formalization-validity-domain + no-patch-mentality）

  本文件**忠实交付**（L0，machine-checked）：
  1. ClassifierFamily：分类器是 Θ 参数化族（`State : Param → Type`，`C : (θ)→X→State θ`）+
     `classifier_total_unique`（给定 θ ⟹ C_θ 全定义/唯一，函数图平凡侧，显式列为「全定义」义务）。
  2. fiber 原像划分（标准① 最干净形式）：`Fiber` + `fiber_total`（覆盖）+ `fiber_disjoint`（互斥）
     + `fiberSetoid`（等价关系 / 商集）——**任何全函数自动给互斥穷尽划分**，纯类型论事实。
  3. 族成员划分 + realized 诚实区分：`theta_fiber_partition`（给定 θ ⟹ fiber partition）+
     `fibers_cover_image`/`realized_on_image`（**值域内** realized 自动）vs `Realized` 谓词
     （**外部大标签集** realized **须另证**，本文件不默认成立）。
  4. Θ_parse 唯一性前件（615 精化）：`ParseDependentClassifier`（候选可多义 + parse 选择器 +
     parse_valid）+ `parse_unique_given_theta`（**给定 θ** 后规范分解唯一——唯一性条件于 Θ_parse）。
  5. 诚实标签：`classFamilyLabels`（StructurePartitionOnly + ThetaParametricPremise +
     EmpiricalDomain + Layer2ThetaRefinement）+ `family_not_true_classification`（禁标真完全分类）。

  本文件**不声明**（有效域边界，诚实标注，避免膨胀）：
  - ✗ fiber partition = 缠论语义分类——partition 是**纯类型论结构事实**（函数图），
       **不**证标签的缠论语义 / 递归正确 / 因果无前视。逐 claim 文件（Trend/Center/BSP/Decomp）
       仍负责给 C_θ 字段的**语义内容**（partition 给框架，逐 claim 给语义，**互补**）。
  - ✗ realized 自动——只在**分类目标 = 值域 `im (C θ)`** 时自动（`fibers_cover_image`）；
       用**外部预设大标签集** `State θ` 时可能有**空 fiber**，`Realized` **须另证**，本文件诚实标。
  - ✗ C_θ 唯一性是缠论无参数唯一性——唯一性**依赖 Θ_parse**（`ParseDependentClassifier` 的
       `parse` 选择器 = `Decomp.lean` 的 GaugeNormal / gaugeFix 截面）。缠论单独 `candidates`
       可**多义**（Decomp.lean gauge 前多义），**固定 θ 后才**截面唯一（gaugeFix 后）。
  - ✗ Param 由缠论导出——Θ（parse / level / signal / voice / risk / exec 的具体选择）是
       **extra-缠论设计选择**（Θ-参数化前件），**不由**缠论结构公理推出。本文件把它参数化，
       **不**声称 Param 来自缠论。
  - ✗ 收益 / 最优 / 阈值稳定——L3 EmpiricalDomain，需真实数据，**不由此 L0 声称**。
  - ✗ 本文件 = 新真完全分类——这是 **615 Layer2 的 Θ-参数化精化**（C 与 π 都 Θ-参数化），
       `family_not_true_classification` 证**禁标** TrueCompleteClassification。

  ★与现有 Strict 库的关系：
  - **import** `Strict.Classification`（共享内核）；与 `Strict.StrategyFamily` 是**对偶**——
    StrategyFamily 证「Θ ⟹ π_Θ（标签**外**执行）」，本文件证「C 本身 Θ-参数化 ⟹ C_Θ +
    fiber partition（标签**本身**产生）」。合起来兑现完整链 Θ ⟹ C_Θ ⟹ partition ⟹ π_Θ。
  - 引证 `Decomp.lean`（gauge 前多义 / gaugeFix 后截面唯一）作为「C_θ 唯一性依赖 Θ_parse」
    的既有 L0 证据（DecompositionSystem.D = 一个 Θ_parse 选择）。
-/

end Strict.ClassificationFamily
