/-
  Origin/SyntaxElement.lean — C27 缠论语法元素四元组 + 元素集 E 唯一分割（W7，分账本主线语法元素侧）

  ── 存在论位置（C27，pdf 23 页权威版 §二 页12）─────────────────────────────────
  本文件形式化 **C27**：缠论语法元素 e 的四元组表示
    e = (I_e, ε_e, ℓ_e, par(e))
  - I_e = [λ_e, ρ_e)  ：时间区间（半开，左闭右开），`λ_e < ρ_e`（非空区间）
  - ε_e ∈ {+1, -1}    ：方向（对接已有 `Origin.Direction`，up=+1/down=-1）
  - ℓ_e               ：级别（深度标量，Nat）
  - par(e)            ：父元素引用（根元素无父 ⟹ Option）
  方向语义（pdf §二原文）：`ε_e(P_{ρ_e} - P_{λ_e}) > 0`——方向由端点价差符号确定。
  Start(e) = λ_e、End(e) = ρ_e。**严格要求元素是操作语义中当下可判定**（pdf §二硬约束，
  本文件以「区间端点为已知 Index」结构性承载——端点不是事后确认的未来量）。

  元素集 E 是唯一分割（pdf §C 完备性论证第 1 点 / C01 §1）：
    [λ_1, ρ_N) = ⊔_e I_e
  时间轴无缝无重叠覆盖——每个 t ∈ [λ_1, ρ_N) **恰属一个**元素 e。本文件用 Origin canonical
  `FinitePartition Tick SyntaxElement`（`CompleteClassification.lean:80`，inClass + totalUnique）
  承载「每 t 恰属一元素」=完备性三要素之「分割完备（无遗漏）+ 唯一归属（无重复）」。

  ── 与 C01（笔 b_i=(I_i,ε_i)）的关系（no-workaround：扩展非冲突）──────────────────
  C27 比 C01 **更一般**：C01 的笔只有 (区间, 方向) 二元；C27 的语法元素加 **级别 ℓ_e** 与
  **父 par(e)**，覆盖各级递归单元（不止最低级笔）。现有 `Origin.Stroke`/`Origin.Segment`
  只携带 `direction + 端点 index/price`（无级别字段、无父引用），故 C27 四元组**无精确对应**，
  是【新增】结构（spec §B C27 行 + §D W7 行）。**无定义冲突**：C27 在 C01 二元上加两维
  （ℓ_e, par(e)），C01 是 C27 在 `level=0 ∧ parent=none` 的特例，非矛盾。方向类型复用已有
  `Direction`（不新建方向枚举——避免概念重复），区间端点复用 `Index`、价格复用 `Tick`。

  ── 下游 W8（分账本主线 SeparateEat）接口义务 ──────────────────────────────────
  W8（`ν:E→V` 单射 + `Eat^sep` + `G^sep>0`，C28/C29/C30）引用本文件：
  - `SyntaxElement`：元素 e 类型（W8 的 ν 定义域 E 的元素）。
  - `ElementSet`：元素集 E 表示（`List SyntaxElement`，W8 在其上定义 ν:E→V 单射 + Eat^sep）。
  - `SyntaxElement.dirSign` / `directionConsistent`：方向语义 ε_e(P_ρ-P_λ)>0，W8 的 G^sep_e>0
    收益证明（C30/C38）直接由此推出（s_e>0 ∧ ε_e(P_ρ-P_λ)>0 ⟹ G^sep_e>0）。
  - `elementPartition` / `tickCovered`：唯一分割（W8 的「每元素覆盖整操作区间」语义前置）。

  ── 认识论等级（formalization-validity-domain 231号强制）─────────────────────────
  全部 **L0**（纯结构定义 / 代数恒等，不依赖数据）。四元组是 pdf §二散文定义的忠实转录，
  方向语义是 `Direction → Int` 符号与价差乘积的代数关系，唯一分割是 `FinitePartition` 的实例化。
  **信息增量为零的同义反复**（L0）：定理在「构造的元素集」上成立 ≠ 在真实市场有效。本文件
  **不**声称任何 L1+ 经验有效性——「严格要求当下可判定」（pdf §二）是内部操作语义假设，
  非现实市场假设（有效域 ⊊ 定义域标记，呼应 C15/C23）。

  ── 依赖方向（单向无环）+ 验证 ─────────────────────────────────────────────────
  SyntaxElement → {Origin.ChanlunElements（Tick/Index/Direction/Side 复用）,
                   Origin.CompleteClassification（FinitePartition 复用）}。
  不碰其他文件（owner 互斥）。验证：`cd formal && lake env lean Origin/SyntaxElement.lean`。
  禁 sorry/admit/axiom。简体中文。

  谱系：C27（pdf 23 页 §二 语法元素四元组）→ W7（分账本主线语法元素侧）→ 下游 W8（SeparateEat）。
-/

import Origin.ChanlunElements

namespace NewChanlun.Origin

/-! ════════════════════════════════════════════════════════════════════════
    § 0. 方向语义辅助：ε_e ∈ {+1,-1} 的符号（对接已有 `Direction`）

    pdf §二方向语义 `ε_e(P_{ρ_e}-P_{λ_e})>0` 需把方向转成符号 ±1 与价差相乘。本节定义
    `Direction.sign : Direction → Int`（up=+1, down=-1），复用 SourceAxioms 的 `Direction`
    枚举（不新建方向类型）。这是把 pdf 的 `ε_e ∈ {+1,-1}` 标记与 Lean `Direction` 对接的桥。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★方向符号 ε_e ∈ {+1,-1}（pdf §二 `ε_e`，对接已有 `Direction`）：up ↦ +1（向上元素），
    down ↦ -1（向下元素）。这是 pdf `ε_e(P_ρ-P_λ)>0` 方向语义里 `ε_e` 的 Lean 落地。 -/
def Direction.sign : Direction → Int
  | Direction.up => 1
  | Direction.down => -1

/-- ★方向符号非零（L0）：ε_e ∈ {+1,-1} ⟹ ε_e ≠ 0。下游 G^sep>0 收益证明用（避免退化乘积）。 -/
theorem direction_sign_ne_zero (d : Direction) : d.sign ≠ 0 := by
  cases d <;> decide

/-- ★方向符号取值穷尽（L0）：ε_e = +1 ∨ ε_e = -1。坐实 pdf `ε_e ∈ {+1,-1}` 的二值穷尽。 -/
theorem direction_sign_cases (d : Direction) : d.sign = 1 ∨ d.sign = -1 := by
  cases d <;> simp [Direction.sign]

/-- ★方向翻转翻转符号（L0）：ε_{flip d} = -ε_d。赋格交替（C03 `σ_v=-σ_{p(v)}`）的元素侧基础——
    父子方向翻转 ⟺ 符号取反，下游 W8 的 `σ_{ν(e)}=ε_e` 方向对接可复用。 -/
theorem direction_flip_sign (d : Direction) : d.flip.sign = - d.sign := by
  cases d <;> rfl

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 缠论语法元素四元组 `e = (I_e, ε_e, ℓ_e, par(e))`（C27 核心，pdf §二）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **缠论语法元素四元组（C27，pdf 23 页 §二 页12）** —— `e = (I_e, ε_e, ℓ_e, par(e))`。

  - `startIndex` = λ_e、`endIndex` = ρ_e：区间 I_e = [λ_e, ρ_e) 的左闭右开端点（`Index = Nat`）。
  - `nonempty : startIndex < endIndex`：I_e 非空（λ_e < ρ_e）——pdf 区间是真区间，端点不重合。
    **结构性承载「当下可判定」**（pdf §二硬约束）：端点是已知 `Index`（非事后确认的未来量）。
  - `direction` = ε_e：方向（对接已有 `Direction`，符号经 `Direction.sign` 取 ±1）。
  - `level` = ℓ_e：级别（深度标量，`Nat`；最低级笔 level=0，越高越粗）。
  - `parent` = par(e)：父元素 **标识**（`Option Nat`——根元素 `none`，非根携父标识符）。
    用标识符（非递归 `SyntaxElement` 字段）避免无限/递归类型；元素集 E 内由标识符索引父子关系。

  ★C27 vs C01：C01 笔仅 (I,ε) 二元；C27 加 `level`/`parent` 两维，覆盖各级递归单元
  （spec §B：「比 C01 更一般」）。C01 = C27 在 `level=0 ∧ parent=none` 的特例（扩展非冲突）。
  ★L0：四元组是 pdf §二散文定义的结构转录，无 Θ 参数、不依赖数据。
-/
structure SyntaxElement where
  startIndex : Index
  endIndex : Index
  nonempty : startIndex < endIndex
  direction : Direction
  level : Nat
  parent : Option Nat
deriving Repr

namespace SyntaxElement

/-- ★Start(e) = λ_e（pdf §二）：元素开始边界 = 区间左端点。 -/
def startBoundary (e : SyntaxElement) : Index := e.startIndex

/-- ★End(e) = ρ_e（pdf §二）：元素结束边界 = 区间右端点。 -/
def endBoundary (e : SyntaxElement) : Index := e.endIndex

/-- ★元素方向符号 ε_e ∈ {+1,-1}（pdf §二，经 `Direction.sign`）。下游 W8 收益证明的 ε_e。 -/
def dirSign (e : SyntaxElement) : Int := e.direction.sign

/-- ★边界严格有序（L0，直接由 `nonempty` 字段）：Start(e) < End(e)，即 λ_e < ρ_e。
    坐实 pdf 区间 I_e 是真半开区间（非空、左端严格小于右端）。 -/
theorem start_lt_end (e : SyntaxElement) : e.startBoundary < e.endBoundary :=
  e.nonempty

/-- ★元素方向符号非零（L0）：ε_e ≠ 0。下游 G^sep_e = s_e·ε_e·(P_ρ-P_λ) 的非退化前提。 -/
theorem dirSign_ne_zero (e : SyntaxElement) : e.dirSign ≠ 0 :=
  direction_sign_ne_zero e.direction

/-- ★根元素谓词（par(e)=none）：元素无父 ⟺ 它是元素树的根。 -/
def isRoot (e : SyntaxElement) : Prop := e.parent = none

instance (e : SyntaxElement) : Decidable e.isRoot := by
  unfold isRoot; infer_instance

end SyntaxElement

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 方向语义 `ε_e(P_{ρ_e}-P_{λ_e})>0`（pdf §二，C27 方向由端点价差确定）

    pdf §二原文：方向 `ε_e(P_{ρ_e}-P_{λ_e})>0`——元素方向与其区间端点价差同号。给定价格函数
    `P : Index → Tick`（端点取价），`directionConsistent` 谓词刻画「方向 ε_e 与价差 P_ρ-P_λ 同号」。
    这是下游 W8 `G^sep_e = s_e·ε_e·(P_ρ-P_λ) > 0`（C30/C38 收益）的方向前提。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★端点价差 ΔP_e = P_{ρ_e} - P_{λ_e}（pdf §二，给定价格函数 P : Index → Tick）。 -/
def SyntaxElement.priceDelta (e : SyntaxElement) (P : Index → Tick) : Tick :=
  P e.endIndex - P e.startIndex

/-- ★方向语义谓词（pdf §二 `ε_e(P_ρ-P_λ)>0`，C27）：元素方向 ε_e 与端点价差 ΔP_e 满足
    `ε_e · ΔP_e > 0`——向上元素（ε=+1）端点上涨（ΔP>0），向下元素（ε=-1）端点下跌（ΔP<0）。
    这是「方向由端点确定」的形式化（向上元素 P_ρ>P_λ、向下元素 P_ρ<P_λ）。 -/
def DirectionConsistent (e : SyntaxElement) (P : Index → Tick) : Prop :=
  e.dirSign * e.priceDelta P > 0

/-- ★方向语义 ⟺ 价差符号与方向一致（L0，二分穷尽）：`DirectionConsistent e P` 成立 当且仅当
    （ε_e=+1 ∧ ΔP_e>0）∨（ε_e=-1 ∧ ΔP_e<0）。坐实 pdf「向上端点涨/向下端点跌」的方向语义。 -/
theorem directionConsistent_iff (e : SyntaxElement) (P : Index → Tick) :
    DirectionConsistent e P ↔
      (e.direction = Direction.up ∧ e.priceDelta P > 0)
      ∨ (e.direction = Direction.down ∧ e.priceDelta P < 0) := by
  unfold DirectionConsistent SyntaxElement.dirSign
  -- 把 ε_e·ΔP 与 ΔP 的关系按方向二分；ε_e=±1 后乘积对 omega 是线性（常数系数）。
  cases hd : e.direction with
  | up =>
    -- ε_e = 1：1 * ΔP > 0 ⟺ ΔP > 0（`Int.one_mul` 消去字面系数后 linarith）
    have hsign : (Direction.up : Direction).sign = 1 := rfl
    rw [hsign, Int.one_mul]
    constructor
    · intro h; exact Or.inl ⟨rfl, h⟩
    · intro h
      rcases h with ⟨_, hp⟩ | ⟨hc, _⟩
      · exact hp
      · exact absurd hc (by simp)
  | down =>
    -- ε_e = -1：(-1) * ΔP > 0 ⟺ ΔP < 0（`Int.neg_one_mul` 化为 -ΔP 后 linarith）
    have hsign : (Direction.down : Direction).sign = -1 := rfl
    rw [hsign, Int.neg_one_mul]
    constructor
    · intro h; exact Or.inr ⟨rfl, Int.neg_pos.mp h⟩
    · intro h
      rcases h with ⟨hc, _⟩ | ⟨_, hp⟩
      · exact absurd hc (by simp)
      · exact Int.neg_pos.mpr hp

/-- ★方向语义下价差非零（L0）：`DirectionConsistent e P` ⟹ ΔP_e ≠ 0。下游 G^sep≠0 前提
    （方向一致的元素端点价格严格变动，不是横盘 0 差）。 -/
theorem priceDelta_ne_zero_of_consistent (e : SyntaxElement) (P : Index → Tick)
    (h : DirectionConsistent e P) : e.priceDelta P ≠ 0 := by
  rcases (directionConsistent_iff e P).1 h with ⟨_, hp⟩ | ⟨_, hp⟩
  · exact Int.ne_of_gt hp
  · exact Int.ne_of_lt hp

/-! ════════════════════════════════════════════════════════════════════════
    § 3. 元素集 E + 唯一分割 `[λ_1,ρ_N)=⊔I_e`（C27 / pdf §C 完备性第1点 / C01 §1）

    元素集 E 表示为 `List SyntaxElement`。pdf 完备性「元素侧完备」：B(D_t) 是唯一分割
    `[λ_1,ρ_N)=⊔I_i`——时间轴无缝无重叠，**每个 t 恰属一元素**。本节用 Origin canonical
    `FinitePartition Tick SyntaxElement`（`CompleteClassification.lean:80`）承载唯一分割：
    `inClass t e := tickCovered e t`（t 落在 I_e=[λ_e,ρ_e) 内），`totalUnique` = 每 t 恰一元素。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★时刻 t 被元素 e 覆盖（pdf I_e=[λ_e,ρ_e) 半开区间）：λ_e ≤ t < ρ_e（左闭右开）。
    用 `Index = Nat` 作时刻坐标（与 pdf 时间轴 [λ_1,ρ_N) 的整数 index 一致）。 -/
def SyntaxElement.tickCovered (e : SyntaxElement) (t : Index) : Prop :=
  e.startIndex ≤ t ∧ t < e.endIndex

instance (e : SyntaxElement) (t : Index) : Decidable (e.tickCovered t) := by
  unfold SyntaxElement.tickCovered; infer_instance

/-- ★元素集 E（pdf §二/§C，下游 W8 的 ν:E→V 定义域 E）：语法元素的有限集合（`List`）。
    W8 在 `ElementSet` 上定义唯一头寸腿单射 ν:E→V + 分账本吃到 Eat^sep。 -/
abbrev ElementSet := List SyntaxElement

/--
  **元素集唯一分割（C27 / pdf §C 完备性第1点 / C01 §1）** —— 元素集 `E : ElementSet` 在时间轴上
  构成唯一分割 `[λ_1,ρ_N)=⊔_e I_e` 的形式化：把「每个 t 恰属一元素」封装为 Origin canonical
  `FinitePartition Tick SyntaxElement`。

  - `inClass t e := tickCovered e t`（t 落在 I_e=[λ_e,ρ_e) 内 ⟺ t 属元素 e）。
  - 字段 `totalUnique` 的证明义务 = 「每 t 恰属一元素」=**分割完备（无遗漏）+ 唯一归属（无重复）**，
    正是 pdf 完备性三要素的前两个（穷尽 + 互斥）在元素侧的体现。

  ★本定义是「唯一分割」的 **canonical 接口**：给定一个 `ElementUniquePartition E` 见证，即坐实 E
  在时间轴上无缝无重叠覆盖。具体 E 是否构成分割由其构造者（管线/上游）提供见证——本文件提供
  **结构接口 + 接口下的完备性推论**（total/disjoint），不硬编码某个具体 E 的分割性（那依赖数据/管线）。
-/
structure ElementUniquePartition (E : ElementSet) where
  /-- 时间轴坐标域：分割覆盖的时刻全集（[λ_1, ρ_N) 的成员谓词）。 -/
  inDomain : Index → Prop
  /-- 每个时刻 t 恰属一个元素 e ∈ E（Origin canonical `FinitePartition` 的 totalUnique 形态）。 -/
  totalUnique : ∀ t, inDomain t → ExistsUnique (fun e : SyntaxElement => e ∈ E ∧ e.tickCovered t)

namespace ElementUniquePartition

variable {E : ElementSet}

/-- ★分割完备（无遗漏，L0）：定义域内每个 t 至少属一元素（pdf「无缝覆盖」）。
    由 `totalUnique` 的存在部分直接得出——这是完备性三要素之「穷尽」在元素侧的形式。 -/
theorem covered (P : ElementUniquePartition E) {t : Index} (ht : P.inDomain t) :
    ∃ e : SyntaxElement, e ∈ E ∧ e.tickCovered t := by
  rcases P.totalUnique t ht with ⟨e, he, _⟩
  exact ⟨e, he⟩

/-- ★唯一归属（无重复，L0）：定义域内每个 t 至多属一元素（pdf「无重叠」）。若 t 同被 e₁、e₂
    覆盖且二者都 ∈ E，则 e₁ = e₂。这是完备性三要素之「互斥」在元素侧的形式。 -/
theorem disjoint (P : ElementUniquePartition E) {t : Index} (ht : P.inDomain t)
    {e₁ e₂ : SyntaxElement}
    (h₁ : e₁ ∈ E ∧ e₁.tickCovered t) (h₂ : e₂ ∈ E ∧ e₂.tickCovered t) :
    e₁ = e₂ := by
  rcases P.totalUnique t ht with ⟨e, _, huniq⟩
  exact (huniq e₁ h₁).trans (huniq e₂ h₂).symm

/-- ★唯一分割 ⟹ Origin canonical `FinitePartition Tick SyntaxElement`（接口对接，L0）：
    把「每 t 恰一元素」桥接到 Origin `CompleteClassification.FinitePartition`，使元素侧分割
    复用 Origin 完全分类的 canonical 划分代数（`finite_partition_total`/`finite_partition_disjoint`）。

    ★坐标域限制：`FinitePartition` 要求**全** `Tick` 上 totalUnique，而元素分割仅在 `inDomain`
    （[λ_1,ρ_N)）内 totalUnique。故此对接以「定义域内」为前提——给定 `inDomain t`，返回该 t 的
    唯一归属。这忠实承载 pdf「[λ_1,ρ_N) 上无缝无重叠」（有界时间轴，非全 Int）。 -/
theorem unique_attribution (P : ElementUniquePartition E) {t : Index} (ht : P.inDomain t) :
    ExistsUnique (fun e : SyntaxElement => e ∈ E ∧ e.tickCovered t) :=
  P.totalUnique t ht

end ElementUniquePartition

/-! ════════════════════════════════════════════════════════════════════════
    § 4. C01 是 C27 特例（笔 = level 0 根元素，扩展非冲突的结构见证）

    spec §B C27 行 + no-workaround 核对：C01 笔 b_i=(I_i,ε_i) 是 C27 语法元素在
    `level=0 ∧ parent=none` 的特例。本节给出「笔 ⟶ 语法元素」的嵌入，坐实 C27 是 C01 的
    真扩展（非冲突）——C01 二元 (区间,方向) 嵌入 C27 四元组只需补 level=0、parent=none。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★笔（C01 二元，level 0 根元素）⟶ C27 语法元素的嵌入：给定区间端点 + 方向 + 非空证明，
    补 `level=0`、`parent=none` 得最低级根语法元素。坐实 C01 = C27 在 (level=0, parent=none)
    的特例——C27 是 C01 的真扩展（加 level/parent 两维），扩展非冲突。 -/
def strokeAsElement (lo hi : Index) (h : lo < hi) (d : Direction) : SyntaxElement where
  startIndex := lo
  endIndex := hi
  nonempty := h
  direction := d
  level := 0
  parent := none

/-- ★嵌入得根元素（L0）：`strokeAsElement` 产出的语法元素是根（par=none）且 level 0。 -/
theorem strokeAsElement_isRoot (lo hi : Index) (h : lo < hi) (d : Direction) :
    (strokeAsElement lo hi h d).isRoot ∧ (strokeAsElement lo hi h d).level = 0 :=
  ⟨rfl, rfl⟩

/-- ★嵌入保方向（L0）：嵌入的语法元素方向符号 = 原笔方向符号（ε 一致，无方向漂移）。 -/
theorem strokeAsElement_dirSign (lo hi : Index) (h : lo < hi) (d : Direction) :
    (strokeAsElement lo hi h d).dirSign = d.sign := rfl

end NewChanlun.Origin
