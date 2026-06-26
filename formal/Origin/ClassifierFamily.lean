/-
  Origin/ClassifierFamily.lean — C_Θ 分类器族 + fiber 原像划分（port 到 Origin canonical base）
  ★task #114, A′ 策略组件港入 Origin canonical base（审计 B：C_Θ/fiber MISSING in Origin）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**。审计 B 判决：分类器族 C_Θ/fiber
  **MISSING in Origin，只在 legacy Strict/ClassificationFamily.lean**。本文件把它**重锚到
  Origin 类型**：定理挂 Origin 命名空间 `NewChanlun.Origin.ClassifierFamily`，并把 fiber 划分
  **对接** Origin canonical `FinitePartition`/`ClassifierPartition`（`CompleteClassification.lean`），
  **不** import legacy Strict、**不**依赖 #113 分类血肉具体实现（只对 Origin 抽象接口编程）。

  ── 核心理论结论（编排者 + codex 收敛，继承 legacy）──────────────────────────
  推导链 = **缠论结构公理 + Θ ⟹ C_Θ ⟹ fiber 原像划分（互斥穷尽自动）⟹ π_Θ**。
  关键洞察：**连分类器 C_Θ 本身都是 Θ 参数化的**。缠论结构公理单独**连唯一分类都给不出**
  ——需 Θ_parse（边界 / canonical 选择器）固定「在哪切、取哪个规范分解」。一旦固定 Θ，
  C_Θ : X → State θ 是 Lean 全函数，其 fiber 自动构成互斥穷尽划分 X = ⊔_s C_Θ⁻¹(s)。

  本文件是 `Origin/StrategyFamily.lean` 的**上游对偶**：
  - StrategyFamily 证「给定 Θ ⟹ π_Θ 全定义/唯一/因果」（标签**外**执行语义）。
  - 本文件证「分类器 C 本身 Θ-参数化 ⟹ C_Θ 全定义/唯一 + fiber 自动互斥穷尽划分」（标签**本身**产生机制）。

  ── 与 Origin CompleteClassifier 的对接（GPT 链「Θ⟹C_Θ⟹fiber partition⟹π_Θ」族侧）──
  Origin `CompleteClassification.lean` 已有 `FinitePartition X Class`（inClass + totalUnique）
  与 `ClassifierPartition`（从 `CompleteClassifier` 造 partition）。本文件证「**任何**族成员全函数
  `F.C θ` 诱导一个 Origin `FinitePartition X (State θ)」（`thetaFinitePartition`）——把族侧的
  fiber 划分**正式对接**到 Origin canonical partition 接口，非另起炉灶。

  ── 诚实标注（formalization-validity-domain + gatekeeper）──────────────────────
  原像划分**只证 partition（结构事实），不证**标签的缠论语义 / 递归正确 / 因果无前视。
  - 「fiber 互斥穷尽」是任何全函数的**纯类型论事实**，不依赖 C 内容。
  - **realized 不自动**：用值域 `im (C θ)` 则每类非空自动；用外部大标签集 `State θ` 可能有
    **空 fiber**，realized 须**另证**（`fibers_cover_image` 自动 vs `Realized` 须另证）。
  - **C_θ 唯一性条件依赖 Θ_parse**：`ParseDependentClassifier` 编码——candidates 可多义
    （gauge 前），parse 选择器固定后截面唯一（gaugeFix 后）。

  ── 分层标注 ──────────────────────────────────────────────────────────────────
  - **L0 可证**：函数图全定义/唯一、fiber 互斥穷尽、fiberSetoid 等价、对接 Origin partition。
  - **Θ 参数**（非缠论可导）：parse/level/signal/voice/risk/exec 的**具体选择**。
  - **L3 经验**（不由本文件 L0 声称）：收益 / 最优 / 阈值稳定。

  ── 依赖方向（单向无环，不 import #113、不 import legacy Strict）──────────────
  ClassifierFamily → Origin.CompleteClassification（用 FinitePartition/ClassifierPartition）。standalone。
  验证：`cd formal && lake env lean Origin/ClassifierFamily.lean`。禁 sorry/admit/axiom。

  谱系：legacy Strict/ClassificationFamily.lean（cc-classfamily #70）→ #96/#97（A′ canonical）→
        本文件 #114（C_Θ/fiber 港入 Origin）。
-/

import Origin.CompleteClassification

namespace NewChanlun.Origin.ClassifierFamily

universe u v

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 ClassifierFamily：分类器是 Θ 参数化族（核心理论结论的 L0 兑现）

  **连分类器 C_Θ 本身都是 Θ 参数化的**。`ClassifierFamily X Param` 把「分类器」升级为按
  参数 `Param` 索引的族——每个 θ 给出状态类型 `State θ` 与**全函数**分类器 `C θ : X → State θ`。
  状态类型 `State : Param → Type` 本身依赖 θ：不同 Θ_parse/Θ_level 选择给出**不同标签集**。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分类器族 `ClassifierFamily X Param`（编排者推导链，L0，port 到 Origin）：

  - `State : Param → Type`：参数化状态/标签类型——每个 θ 给一个标签集 `State θ`。
  - `C : (θ : Param) → X → State θ`：**Θ 参数化的全函数分类器族**。

  ★缠论结构公理**不**单独给出 C（连唯一分类都给不出），必须配 Θ ∈ Param（边界 / canonical
  选择器）才把分类器钉死为 C θ。Param **不由缠论导出**（Θ-参数化前件），但**给定** θ 后
  C θ 是确定全函数（L0）。
-/
structure ClassifierFamily (X Param : Type) where
  State : Param → Type
  C : (θ : Param) → X → State θ

/--
  ★★分类器全定义 + 唯一（L0）：`classifier_total_unique` — 给定 θ，C θ 对每对象全定义且唯一。
  用 Origin canonical `ExistsUnique`（`SourceAxioms.ExistsUnique`）陈述——对接 Origin 总-唯一词汇。

  ★诚实标注证明强度（避免声明膨胀）：函数图平凡侧——C θ 是 Lean 全函数，故「存在标签」与
  「唯一」自动成立。**不是**「证明了缠论分类完全」，而是「一旦分类器是全函数，每对象恰好一个
  标签」的接口兑现。语义内容由逐 claim 文件承载，**不**由本定理兑现。
-/
theorem classifier_total_unique {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) (x : X) :
    ExistsUnique (fun s => F.C θ x = s) :=
  ⟨F.C θ x, rfl, fun _ heq => heq.symm⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 fiber 原像划分：任何全函数自动给互斥穷尽 partition

  给定任意固定全函数 `C : X → S`，其 fiber `C⁻¹(s) = {x | C x = s}` 自动构成 X 的**互斥
  穷尽划分** X = ⊔_s C⁻¹(s)：覆盖（穷尽）+ 互斥（disjoint）+ 等价关系（FiberRel）。这是
  **纯类型论事实**，不依赖 C 的缠论语义。工具定义在固定全函数上；§3 实例化到族成员 F.C θ。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★fiber（原像 / 纤维）`Fiber C s x`（L0）：`x` 落在标签 `s` 的 fiber ⟺ `C x = s`。 -/
def Fiber {X S : Type} (C : X → S) (s : S) (x : X) : Prop :=
  C x = s

/-- ★fiber 覆盖（穷尽 / total，L0）：每对象至少落在一个 fiber（取 s = C x）。 -/
theorem fiber_total {X S : Type} (C : X → S) (x : X) :
    ∃ s, Fiber C s x :=
  ⟨C x, rfl⟩

/-- ★fiber 互斥（disjoint，L0）：一对象不落在两个不同标签的 fiber（函数值唯一）。 -/
theorem fiber_disjoint {X S : Type} (C : X → S) {s₁ s₂ : S} {x : X} :
    Fiber C s₁ x → Fiber C s₂ x → s₁ = s₂ := by
  intro h1 h2
  exact h1.symm.trans h2

/-- ★fiber 等价关系 `FiberRel C x y`（L0）：两对象同 fiber ⟺ `C x = C y`（同标签）。 -/
def FiberRel {X S : Type} (C : X → S) (x y : X) : Prop :=
  C x = C y

/--
  ★★fiber Setoid（L0，划分 = 等价关系兑现）：`FiberRel C` 是 X 上的等价关系（自反/对称/传递）。
  任何全函数 C 的 fiber 划分**就是**等价关系，诱导商集 `Quotient (fiberSetoid C) ≅ im C`。
  用 `def`（非 instance）：S/C 是显式参数，无法由 typeclass synthesis 从返回类型推断。
-/
def fiberSetoid {X S : Type} (C : X → S) : Setoid X where
  r := FiberRel C
  iseqv := ⟨fun _ => rfl, fun h => h.symm, fun h1 h2 => h1.trans h2⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 族成员 fiber 划分 + 对接 Origin FinitePartition + realized 诚实区分
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★族成员 fiber `θFiber F θ s x`（L0）：把 §2 `Fiber` 实例化到固定 θ 的全函数 `F.C θ`。 -/
def θFiber {X Param : Type} (F : ClassifierFamily X Param)
    (θ : Param) (s : F.State θ) (x : X) : Prop :=
  Fiber (F.C θ) s x

/--
  ★★族成员 fiber 划分（L0）：`theta_fiber_partition` — 给定 θ，F.C θ 的 fiber 互斥穷尽划分 X。
  返回覆盖（total）+ 互斥（disjoint）。纯类型论事实，对每个 θ 成立，不依赖 C_θ 缠论语义。
-/
theorem theta_fiber_partition {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) :
    (∀ x, ∃ s, θFiber F θ s x) ∧
    (∀ x (s₁ s₂ : F.State θ), θFiber F θ s₁ x → θFiber F θ s₂ x → s₁ = s₂) :=
  ⟨fun x => fiber_total (F.C θ) x,
   fun _ _ _ h1 h2 => fiber_disjoint (F.C θ) h1 h2⟩

/--
  ★★对接 Origin canonical `FinitePartition`（L0，A′ port 的核心对接，GPT 链族侧）：
  `thetaFinitePartition` — 任何族成员全函数 `F.C θ` 诱导一个 Origin `FinitePartition X (State θ)`。

  这把族侧 fiber 划分**正式对接**到 Origin `CompleteClassification.lean` 的 `FinitePartition`
  接口（`inClass x s := C x = s`，totalUnique 由分类器全定义/唯一给出）——非另起炉灶，而是
  填入 Origin canonical partition 接口。下游可经此与 Origin `ClassifierPartition` 同构使用。

  ★诚实标注：这只对接「**结构 partition**」，不对接「缠论语义分类」。Origin `CompleteClassifier`
  的 `complete`（fiber = 行为等价类）是**更强**的语义义务，本文件**不**为 F.C θ 提供它
  （那需 trace + BehEquiv，属逐 claim 文件 / Origin TrendCompleteClassification 承载）。
-/
def thetaFinitePartition {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) :
    FinitePartition X (F.State θ) where
  inClass x s := F.C θ x = s
  totalUnique x := ⟨F.C θ x, rfl, fun _ heq => heq.symm⟩

/--
  ★对接见证（L0）：`thetaFinitePartition` 的 inClass 恰是 θFiber（rfl，接口对接无语义漂移）。
  证 Origin partition 的 inClass 字段被族 fiber 逐字段填充——族 fiber 划分 = Origin partition。
-/
theorem thetaFinitePartition_inClass_eq {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) (x : X) (s : F.State θ) :
    (thetaFinitePartition F θ).inClass x s ↔ θFiber F θ s x :=
  Iff.rfl

/--
  ★★fiber 覆盖值域 ⟹ 值域内每类非空（L0，realized 的**自动**侧）：
  `fibers_cover_image` — 取分类目标为**值域** `im (F.C θ)`，则每个标签 `s = F.C θ x` 的 fiber
  **非空自动**（x 自己是见证）。★诚实：**只在分类目标 = 值域时**自动，不保证外部大标签集
  `F.State θ` 的每个 s 都非空（见 `Realized`，须另证）。
-/
theorem fibers_cover_image {X Param : Type}
    (F : ClassifierFamily X Param) (θ : Param) (x : X) :
    ∃ x', θFiber F θ (F.C θ x) x' :=
  ⟨x, rfl⟩

/--
  ★可实现谓词 `Realized F θ`（L0 谓词，realized 的**须另证**侧）：
  `Realized F θ` ⟺ `F.State θ` 的**每个**标签 s 都有对象见证（`∀ s, ∃ x, F.C θ x = s`）。

  ★诚实标注（关键区分 vs fibers_cover_image）：若分类目标 = 值域则 realized 由
  `fibers_cover_image` 自动；若 = 外部预设大标签集（某态在某 θ 下从不出现）则可能有空 fiber，
  `Realized` **不自动成立，须另证**。本文件把它定义为显式谓词而**不**证它对任意 F/θ 成立——
  这是诚实标注：realized 是分类目标选择的函数，不是 partition 的自动推论。
-/
def Realized {X Param : Type} (F : ClassifierFamily X Param) (θ : Param) : Prop :=
  ∀ s : F.State θ, ∃ x, F.C θ x = s

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 Θ_parse 唯一性前件（C_θ 唯一性依赖规范化选择器）

  C_θ 的「唯一性」**不是**缠论无参数唯一性——它依赖一个 Θ_parse（规范分解选择器）。
  `ParseDependentClassifier` 编码：candidates 可多义（gauge 前），parse 选择器固定后截面唯一
  （gaugeFix 后）。诚实标注唯一性是规范化选择**之后**才有的。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Θ_parse 依赖的分类器 `ParseDependentClassifier X Cand Param`（L0，port 到 Origin）：

  - `candidates : X → Cand → Prop`：对象 x 的**候选**分解集（gauge 前可能多义）。
  - `parse : Param → X → Cand`：**Θ_parse 选择器族**——给定 Θ 从候选中选规范分解。
  - `parse_valid`：选出的规范分解确是候选之一（选择器不凭空造分解）。

  ★诚实标注：唯一规范分解的**唯一性来自 θ（选择器），不来自缠论无参数唯一性**。没有 θ，
  candidates x 可能多元。parse 的**具体内容**（包含处理、相同极值取舍、开闭边界、未完成尾部）
  是 **Θ-参数化，非缠论可导**——本结构**不**声称 parse 由缠论唯一钉死。
-/
structure ParseDependentClassifier (X Cand Param : Type) where
  candidates : X → Cand → Prop
  parse : Param → X → Cand
  parse_valid : ∀ θ x, candidates x (parse θ x)

/--
  ★规范分解给定 Θ 后唯一（L0，「gaugeFix 后截面唯一」侧）：
  `parse_unique_given_theta` — 给定 Θ_parse 选择器 θ，规范分解 `parse θ x` 对每 x **唯一**。
  用 Origin `ExistsUnique` 陈述。唯一性**条件于 θ**——这正是「唯一性依赖 Θ_parse」。

  ★诚实标注：函数图唯一（parse θ 是全函数），平凡侧；内容**不是**「缠论给唯一分解」，而是
  「**一旦选定** Θ_parse 选择器 θ，规范分解就唯一」。缠论单独（无 θ）对应 candidates 可多元。
-/
theorem parse_unique_given_theta {X Cand Param : Type}
    (P : ParseDependentClassifier X Cand Param) (θ : Param) (x : X) :
    ExistsUnique (fun t => P.parse θ x = t) :=
  ⟨P.parse θ x, rfl, fun _ heq => heq.symm⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（gatekeeper：StructurePartitionOnly + Θ-参数化前件，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分类器族产出标签 `FamilyTag`（gatekeeper，诚实分层）。
  StructurePartitionOnly（fiber 划分是结构事实，非缠论语义分类）+ ThetaParametricPremise
  （分类器 C 本身 Θ-参数化）+ EmpiricalDomain（收益/最优 L3）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把本文件产出标为真完全分类。
-/
inductive FamilyTag where
  | StructurePartitionOnly
  | ThetaParametricPremise
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★分类器族子类（gatekeeper）：ThetaRefinement（唯一子类，Θ-参数化精化，非新真完全分类）。 -/
inductive FamilySubkind where
  | ThetaRefinement
deriving DecidableEq, Repr

/-- ★分类器族诚实标签包（L0 声明）。 -/
def classFamilyLabels : List FamilyTag × FamilySubkind :=
  ([FamilyTag.StructurePartitionOnly, FamilyTag.ThetaParametricPremise, FamilyTag.EmpiricalDomain],
   FamilySubkind.ThetaRefinement)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：分类器族子类必是 ThetaRefinement。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝冒充真完全分类（非跨库强保证）。 -/
theorem family_not_true_classification (k : FamilySubkind) :
    k = FamilySubkind.ThetaRefinement := by
  cases k; rfl

end NewChanlun.Origin.ClassifierFamily
