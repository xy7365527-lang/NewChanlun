/-
  Strict/Nest.lean — 区间套有限递归证书 χ_{v,t}^σ（task #72, RTAS 蜂群 cc-nest 工位）

  topo_address: swarm/pi-theta/cc-nest ｜ parent: Lead

  ── 工位定位 ────────────────────────────────────────────────────────────────
  本文件 import 标准内核库 `Classification`（Strict.* 结构定义），形式化 π_Θ 推导
  **第 3 步：区间套有限递归证书**。把缠论第 62-70 课「逐级进入低级别结构定位买卖点」
  的结构形式转写为带证明义务的 Lean 结构：

  - **操作级到执行级链** `o_v = ℓ₀ ≻ ℓ₁ ≻ … ≻ ℓ_k = e_v`：一条有限的级别下降链，
    从操作级别 ℓ₀（决策发生的级别）逐级进入到执行级别 e_v（实际下单的级别）。
  - **方向** `σ ∈ {+1 买, -1 卖}`：买卖点方向（`Dir.buy` / `Dir.sell`）。
  - **候选区间** `J_{ℓ_j,t}^σ`：第 j 级在时刻 t、方向 σ 下的候选定位区间。
  - **递归证书** `𝒩_{v,j}^σ`：逐级缩小的合取链（见结构 NestCertificate）。
  - **最终确认** `χ_{v,t}^σ := 𝒩_{v,k}^σ ∧ Confirm_{e_v}^σ`。

  ── 递归证书的精确形式（任务规格） ────────────────────────────────────────────
    𝒩_{v,0}^σ     := Candidate_{ℓ₀}^σ
    𝒩_{v,j+1}^σ   := 𝒩_{v,j}^σ ∧ Candidate_{ℓ_{j+1}}^σ ∧ (J_{ℓ_{j+1}}^σ ⊆ J_{ℓ_j}^σ)
    χ_{v,t}^σ     := 𝒩_{v,k}^σ ∧ Confirm_{e_v}^σ
  逐级 `⊆`（区间套缩小）= 缠论「在高级别买卖点区域内，进入低级别结构精确定位」的结构形式。

  ── 核心定理 nest_certificate_unique ─────────────────────────────────────────
  对区间套**深度结构归纳**，证满足 χ 的「定位见证」唯一——但**唯一性条件依赖
  `Sel_Θ` 选择器**（结束时间最新 → 开始时间最新 → 编号最小）。`Sel_Θ` 作**显式参数**：
  没有它，同一级别可能有多个 candidate 区间满足谓词（多值），唯一性不成立。这是
  **Θ-参数化前件**——缠论结构公理单独给不出唯一定位，需 Θ_signal 的选择器固定。

  ── 终端 2/3 类买卖点重合（对接 cc-levelstate bit-vector） ───────────────────
  终端 `Λ = {i : 第 i 类买卖点成立}`（bit-vector `BSPClass → Bool`），
  `Conf := 𝟙[Λ ≠ ∅]`。**不要求 |Λ| = 1**——允许 2/3 类买卖点在同一终端共存
  （codex / 编排者一致）。一个执行点同时是第 2 类和第 3 类买点是缠论合法情形，
  确认条件只问「至少一类成立」，不问「恰好一类」。

  ── 认识论等级（formalization-validity-domain 强制标注） ─────────────────────
  全部 **L0**（纯定义 / 结构归纳，不依赖数据）。`lake env lean Strict/Nest.lean` 通过
  = 区间套递归证书的良构性、`Sel_Θ`-固定下的唯一性、bit-vector 确认的 `|Λ|≥1` 语义
  在定义层成立，**不是**任何「区间套定位在真实行情上有效」的实证断言。唯一性定理是
  纯结构归纳，信息增量 = 同义反复（L0），**不冒充** L1+。

  ── 诚实标注（gatekeeper 标签） ──────────────────────────────────────────────
  ★**StructurePartitionOnly**：区间套 ⊆ 逐级缩小是缠论第 62-70 课「逐级进入低级别
    结构定位」的**结构形式**，本文件证的是这个结构的良构性与唯一性条件，**不证**
    某个具体 candidate 区间真的对应缠论买卖点的语义内容（语义由 BSP.lean 逐 claim 给）。
  ★**Θ-参数化前件**：χ 唯一性依赖 `Sel_Θ`（Θ_signal 选择器）。缠论结构公理单独
    **连唯一定位都给不出**——需 Sel_Θ 固定「同级别多 candidate 时取哪个」。本文件把
    Sel_Θ 参数化，**不**声称选择器规则由缠论结构推出（它是 Θ 的内容）。

  范式：纯 Prop/Type 结构定义，不依赖 Mathlib（与 Classification.lean 同范式）。
  禁 sorry/admit/axiom。

  谱系：598（真完全分类元判据）→ 615（Layer1 ⊊ Layer2）→ ClassificationFamily（C_Θ
        Θ-参数化）→ 本文件（定位证书 Θ-参数化：连唯一定位都需 Sel_Θ）。
-/

import Strict.Classification

namespace Strict.Nest

/--
  **方向** σ ∈ {+1 买, -1 卖}。

  买卖点的方向：`buy`（做多 / +1）、`sell`（做空 / −1）。
-/
inductive Dir where
  | buy
  | sell
deriving DecidableEq, Repr

/--
  **买卖点类别** —— 缠论三类买卖点（对接 cc-levelstate 的 bit-vector 终端）。

  `first` 第一类（背驰）、`second` 第二类（回抽不破）、`third` 第三类（中枢离开后回抽不回）。
  终端 `Λ : BSPClass → Bool` 标记每一类是否成立，允许多类共存（如 2/3 类重合）。
-/
inductive BSPClass where
  | first
  | second
  | third
deriving DecidableEq, Repr

/--
  **候选区间载体的抽象接口** —— 一个候选定位区间 `J` 必须携带 `Sel_Θ` 排序所需的三键：

  - `endTime`：结束时间（第一排序键，最新者优先 ⟹ 数值最大优先）。
  - `startTime`：开始时间（第二排序键，最新者优先 ⟹ 数值最大优先）。
  - `idx`：编号（第三排序键，最小者优先 ⟹ 数值最小优先）。

  ★这是 `Sel_Θ` 选择器作用的最小数据——区间的几何内容（上下界）由下游 `Sub` 关系契约，
  本结构只暴露排序所需的三键，使「选择器固定 candidate」成为可机器检查的命题。
-/
structure Interval where
  endTime : Nat
  startTime : Nat
  idx : Nat
deriving DecidableEq, Repr

/--
  **Sel_Θ 字典序键** —— 把一个区间映射到其三键 `(endTime, startTime, idx)`。

  唯一性证明把「同级别多 candidate」的歧义压缩到这三键的字典序比较上。
-/
def selKey (J : Interval) : Nat × Nat × Nat :=
  (J.endTime, J.startTime, J.idx)

/--
  **Sel_Θ 选择器谓词** —— `IsSelected cands J` 表示在候选集合 `cands : List Interval`
  中，`J` 是 `Sel_Θ` 规则（结束时间最新 → 开始时间最新 → 编号最小）选出的那一个。

  形式化为：`J ∈ cands`，且 `J` 在 `selOrder` 字典序下**严格优于或等于**集合中每个候选，
  且**严格优于**每个与它键不同的候选——保证选出的对象在键上唯一。

  ★`selOrder` 编码「最新 endTime 优先、再最新 startTime、再最小 idx」：
    endTime 大者优 / startTime 大者优 / idx 小者优。
-/
def selOrder (a b : Interval) : Prop :=
  a.endTime > b.endTime
    ∨ (a.endTime = b.endTime ∧ a.startTime > b.startTime)
    ∨ (a.endTime = b.endTime ∧ a.startTime = b.startTime ∧ a.idx < b.idx)

/--
  `IsSelected cands J`：`J` 是 `cands` 中 `Sel_Θ` 选出的唯一最优候选。

  - `mem`：`J` 在候选集合中。
  - `best`：对集合中任意 `J'`，要么 `J' = J`（键相同），要么 `J` 在 `selOrder` 下严格优于 `J'`。

  ★`best` 同时给出存在（`J` 是最优）与唯一性骨架（任何非 `J` 候选都被 `J` 严格压制，
    故被选中者在三键上唯一）——下游 `selected_key_unique` 由此读出唯一性。
-/
structure IsSelected (cands : List Interval) (J : Interval) : Prop where
  mem : J ∈ cands
  best : ∀ J', J' ∈ cands → J' = J ∨ selOrder J J'

/--
  **selOrder 的反对称性（严格）** —— 若 `selOrder a b` 则 `¬ selOrder b a`。

  这是字典序严格序的基本性质，唯一性证明的核心引理：两个互相严格优的候选不可能共存。
-/
theorem selOrder_asymm {a b : Interval} (h : selOrder a b) : ¬ selOrder b a := by
  intro h'
  rcases h with he | ⟨hee, hs⟩ | ⟨hee, hse, hi⟩ <;>
    rcases h' with he' | ⟨hee', hs'⟩ | ⟨hee', hse', hi'⟩ <;> omega

/--
  **selOrder 蕴含键不等** —— 若 `selOrder a b` 则 `selKey a ≠ selKey b`。

  严格优 ⟹ 三键不全相等。用于从 `IsSelected.best` 读出「被选中者键唯一」。
-/
theorem selOrder_key_ne {a b : Interval} (h : selOrder a b) : selKey a ≠ selKey b := by
  intro hk
  simp only [selKey, Prod.mk.injEq] at hk
  obtain ⟨h1, h2, h3⟩ := hk
  rcases h with he | ⟨_, hs⟩ | ⟨_, _, hi⟩
  · omega
  · omega
  · omega

/--
  **selected 的键唯一性** —— 同一候选集合中，任何两个被 `Sel_Θ` 选中的对象键相同。

  这是 `nest_certificate_unique` 的逐级核心：每级别的 candidate 在 `Sel_Θ` 下键唯一，
  故沿区间套链各级别选择确定，整条定位见证唯一。

  ★依赖 `Sel_Θ`（`IsSelected`）作为前件——**这正是 Θ-参数化前件**：去掉选择器，
    同级别多 candidate 时此唯一性不成立（best 假设无法建立）。
-/
theorem selected_key_unique
    {cands : List Interval} {J₁ J₂ : Interval}
    (h₁ : IsSelected cands J₁) (h₂ : IsSelected cands J₂) :
    selKey J₁ = selKey J₂ := by
  rcases h₁.best J₂ h₂.mem with hEq | hLt
  · exact congrArg selKey hEq.symm
  · rcases h₂.best J₁ h₁.mem with hEq' | hLt'
    · exact congrArg selKey hEq'
    · exact absurd hLt' (selOrder_asymm hLt)

/--
  **区间套关系 `Sub`** —— `Sub J' J` 表示候选区间 `J'`（低级别）套在 `J`（高级别）之内，
  即 `J_{ℓ_{j+1}}^σ ⊆ J_{ℓ_j}^σ`（逐级缩小定位）。

  抽象为载体上的二元关系（具体几何含义由下游 BSP/Center 实例契约），本文件证其
  作为区间套链节点的**传递性**（嵌套可链式传播）。
-/
def Sub (J' J : Interval) : Prop :=
  J'.startTime ≥ J.startTime ∧ J'.endTime ≤ J.endTime

/-- `Sub` 自反。 -/
theorem Sub_refl (J : Interval) : Sub J J := ⟨Nat.le_refl _, Nat.le_refl _⟩

/-- `Sub` 传递——区间套逐级缩小可链式传播。 -/
theorem Sub_trans {J₃ J₂ J₁ : Interval} (h₁ : Sub J₃ J₂) (h₂ : Sub J₂ J₁) : Sub J₃ J₁ :=
  ⟨Nat.le_trans h₂.1 h₁.1, Nat.le_trans h₁.2 h₂.2⟩

/--
  **级别节点** —— 区间套链上一级 ℓ_j 的全部数据：

  - `cands`：该级别在 (t, σ) 下的候选区间集合。
  - `chosen`：`Sel_Θ` 从 `cands` 选出的候选（由 `selected` 字段见证）。
  - `Candidate`：该级别候选谓词 `Candidate_{ℓ_j}^σ`（缠论语义内容由下游契约，本文件抽象为命题）。
-/
structure LevelNode where
  cands : List Interval
  chosen : Interval
  selected : IsSelected cands chosen
  Candidate : Prop

/--
  **递归证书 `𝒩`** —— 对级别链（`LevelNode` 列表，`ℓ₀` 在表头）逐级构造的合取链。

      𝒩 [ℓ₀]            := Candidate_{ℓ₀}
      𝒩 (ℓ₀ :: rest)    := Candidate_{ℓ₀} ∧ 𝒩 rest ∧ (J_{rest.head} ⊆ J_{ℓ₀})

  ★实现上以**列表递归**展开任务规格的 `𝒩_{v,j+1} := 𝒩_{v,j} ∧ Candidate_{ℓ_{j+1}} ∧ ⊆`：
    每深入一级，追加该级 candidate 谓词 + 与上一级的区间套 `Sub` 约束。空链为 `True`
    （无级别 ⟹ 无约束），单级链退化为该级 candidate 谓词。
-/
def NestCertificate : List LevelNode → Prop
  | [] => True
  | [ℓ] => ℓ.Candidate
  | ℓ₀ :: ℓ₁ :: rest =>
      ℓ₀.Candidate ∧ Sub ℓ₁.chosen ℓ₀.chosen ∧ NestCertificate (ℓ₁ :: rest)

/--
  **终端确认 bit-vector `Λ`** —— 执行级 e_v 处三类买卖点的成立标记。

  `Λ c = true` 表示第 c 类买卖点在该终端成立。允许多个 `true`（2/3 类共存）。
-/
def BSPVector := BSPClass → Bool

/--
  **`Λ ≠ ∅` 判定** —— 至少一类买卖点成立。

  `nonEmpty Λ := Λ first ∨ Λ second ∨ Λ third`（任一类为 true）。
-/
def nonEmpty (Λ : BSPVector) : Prop :=
  Λ BSPClass.first = true ∨ Λ BSPClass.second = true ∨ Λ BSPClass.third = true

/--
  **终端确认 `Conf := 𝟙[Λ ≠ ∅]`** —— 确认谓词只问「至少一类买卖点成立」，
  **不要求 |Λ| = 1**。

  ★这正是「2/3 类买卖点允许重合」的兑现：`Confirm` 对 `Λ first = Λ second = Λ third = true`
    （三类全中）同样成立，对恰好一类成立也成立——唯一被排除的是 `Λ = ∅`（全 false）。
-/
def Confirm (Λ : BSPVector) : Prop := nonEmpty Λ

set_option linter.unusedVariables false in
/--
  **2/3 类共存合法性见证** —— 当第 2 类与第 3 类同时成立（`Λ second = Λ third = true`）时，
  `Confirm Λ` 成立。

  ★机器检查地兑现「不要求 |Λ|=1」：此定理对 |Λ|≥2 的终端给出 Confirm，证明确认条件
    不排斥多类共存（codex / 编排者一致）。`h₃` 是语义前件（声明第 3 类也成立），
    虽证明体只用 `h₂`，但删之即丢失「2/3 共存」的签名语义——故局部关闭 unused linter。
-/
theorem confirm_of_two_three
    (Λ : BSPVector) (h₂ : Λ BSPClass.second = true) (h₃ : Λ BSPClass.third = true) :
    Confirm Λ := by
  exact Or.inr (Or.inl h₂)

set_option linter.unusedVariables false in
/--
  **三类全中亦确认** —— `|Λ| = 3` 时 `Confirm` 仍成立（共存不设上限）。
-/
theorem confirm_of_all_three
    (Λ : BSPVector)
    (h₁ : Λ BSPClass.first = true) (h₂ : Λ BSPClass.second = true)
    (h₃ : Λ BSPClass.third = true) :
    Confirm Λ :=
  Or.inl h₁

/--
  **空 Λ 不确认** —— `Λ = ∅`（全 false）时 `¬ Confirm Λ`。

  确认条件的下边界：唯一被排除的是无任何买卖点成立的终端。
-/
theorem not_confirm_of_empty
    (Λ : BSPVector)
    (h₁ : Λ BSPClass.first = false) (h₂ : Λ BSPClass.second = false)
    (h₃ : Λ BSPClass.third = false) :
    ¬ Confirm Λ := by
  intro h
  rcases h with h | h | h
  · rw [h₁] at h; exact Bool.noConfusion h
  · rw [h₂] at h; exact Bool.noConfusion h
  · rw [h₃] at h; exact Bool.noConfusion h

/--
  **最终确认证书 `χ_{v,t}^σ := 𝒩_{v,k}^σ ∧ Confirm_{e_v}^σ`** —— 区间套递归证书与终端确认的合取。

  - `chain`：操作级→执行级的级别链（`ℓ₀` 在表头，`ℓ_k` 在表尾 = 执行级 e_v）。
  - `dir`：方向 σ。
  - `terminal`：执行级终端 bit-vector Λ。
  - `nest`：递归证书 `𝒩` 成立（逐级 candidate + 区间套 ⊆）。
  - `confirm`：终端确认 `Conf := 𝟙[Λ ≠ ∅]` 成立。
-/
structure Chi where
  chain : List LevelNode
  dir : Dir
  terminal : BSPVector
  nest : NestCertificate chain
  confirm : Confirm terminal

/--
  **定位见证（locator）** —— 一条 χ 在各级别选出的区间序列（每级 `Sel_Θ` 的 `chosen`）。

  这是「χ 唯一」所唯一的对象：给定级别链结构（`cands` 集合固定），各级 `chosen` 由
  `Sel_Θ` 确定，故整条 locator 在键上唯一。
-/
def locatorKeys (chain : List LevelNode) : List (Nat × Nat × Nat) :=
  chain.map (fun ℓ => selKey ℓ.chosen)

/--
  **逐级 chosen 键由 cands 唯一确定** —— 两条 χ 若各级别候选集合 `cands` 对应相等，
  则各级 `Sel_Θ` 选出的 `chosen` 键逐级相等。

  这是 `nest_certificate_unique` 的列表版核心：对链长度归纳，逐节点调用
  `selected_key_unique`（每级 `Sel_Θ` 键唯一）。
-/
theorem locatorKeys_unique :
    ∀ (c₁ c₂ : List LevelNode),
      c₁.map (·.cands) = c₂.map (·.cands) →
      (∀ i (h₁ : i < c₁.length) (h₂ : i < c₂.length),
        IsSelected (c₂.get ⟨i, h₂⟩).cands (c₁.get ⟨i, h₁⟩).chosen) →
      locatorKeys c₁ = locatorKeys c₂
  | [], [], _, _ => rfl
  | [], _ :: _, h, _ => by simp at h
  | _ :: _, [], h, _ => by simp at h
  | ℓ₁ :: rest₁, ℓ₂ :: rest₂, hcands, hsel => by
    simp only [List.map_cons, List.cons.injEq] at hcands
    obtain ⟨hcandHead, hcandTail⟩ := hcands
    have hHead : selKey ℓ₁.chosen = selKey ℓ₂.chosen := by
      have hsel0 : IsSelected ℓ₂.cands ℓ₁.chosen := by
        have := hsel 0 (by simp) (by simp)
        simpa using this
      have h2sel : IsSelected ℓ₂.cands ℓ₂.chosen := ℓ₂.selected
      exact selected_key_unique hsel0 h2sel
    have hTail : locatorKeys rest₁ = locatorKeys rest₂ := by
      apply locatorKeys_unique rest₁ rest₂ hcandTail
      intro i h₁ h₂
      have := hsel (i + 1) (by simpa using Nat.succ_lt_succ h₁)
        (by simpa using Nat.succ_lt_succ h₂)
      simpa using this
    simp only [locatorKeys, List.map_cons, List.cons.injEq]
    exact ⟨hHead, hTail⟩

/--
  **核心定理 `nest_certificate_unique`** —— 在 `Sel_Θ` 选择器固定下，区间套递归证书
  的**定位见证（locator 键序列）唯一**。

  陈述：给定两条 χ，若它们的级别链结构对应相等（各级别 `cands` 候选集合逐级相同），
  且每级满足交叉 `Sel_Θ` 选择条件，则两条 χ 的 locator 键序列相等
  （`locatorKeys c₁ = locatorKeys c₂`）。

  ★**这是「χ 唯一」的精确形式**：χ 唯一 = 在固定级别链结构下，`Sel_Θ` 确定的定位见证唯一。
    证明对区间套深度（链长度）结构归纳，逐级归约到 `selected_key_unique`。

  ★**Θ-参数化前件（诚实标注）**：唯一性**依赖** `hsel`（`Sel_Θ` 交叉选择条件）。去掉它，
    同级别多 candidate 时无法建立逐级键相等——缠论结构公理单独**给不出唯一定位**。
    这与 ClassificationFamily「C_θ 唯一性依赖 Θ_parse」同构：定位证书的唯一性是
    选择器固定**之后**才有的，不是缠论的无参数唯一性。
-/
theorem nest_certificate_unique
    (χ₁ χ₂ : Chi)
    (hcands : χ₁.chain.map (·.cands) = χ₂.chain.map (·.cands))
    (hsel : ∀ i (h₁ : i < χ₁.chain.length) (h₂ : i < χ₂.chain.length),
      IsSelected (χ₂.chain.get ⟨i, h₂⟩).cands (χ₁.chain.get ⟨i, h₁⟩).chosen) :
    locatorKeys χ₁.chain = locatorKeys χ₂.chain :=
  locatorKeys_unique χ₁.chain χ₂.chain hcands hsel

set_option linter.unusedVariables false in
/--
  **退化情形：单级别链 χ 唯一** —— 当操作级 = 执行级（链长 1，无区间套缩小）时，
  唯一性退化为单级 `Sel_Θ` 键唯一。

  ★边界条件见证：k = 0（无下降级别）时区间套递归证书退化为单一 candidate，
    χ 唯一性 = 该级 `Sel_Θ` 选择唯一。
-/
theorem nest_certificate_unique_singleton
    (ℓ₁ ℓ₂ : LevelNode)
    (hcands : ℓ₁.cands = ℓ₂.cands)
    (hsel : IsSelected ℓ₂.cands ℓ₁.chosen) :
    selKey ℓ₁.chosen = selKey ℓ₂.chosen :=
  selected_key_unique hsel ℓ₂.selected

end Strict.Nest
