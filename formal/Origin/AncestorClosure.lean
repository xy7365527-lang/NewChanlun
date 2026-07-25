/-
Origin/AncestorClosure.lean — 工作单元 MW7：祖先闭合 AncOK 活动集递归
的 L0 结构形式化（互斥分类条目 M16，§八 页7–8）。

★工位定位（cov: 推导完全互斥分类.pdf 19 页权威版 §八 页7–8）：

  互斥分类（M01–M30）相对完全分类第二部分的**真正新增**之一：在 C31/W9 的目标激活集
  `Ã_{t+1}=(A_t\D_t)∪B_t`（先关后开）之上，**加祖先闭合步**。M16 §八 boxed：

    B_t = {e : λ_e = t}（开始集）  /  D_t = {e : ρ_e = t}（结束集）  /  A_t ⊆ E（已激活集）
    先平后开：A^raw_{t+1} = (A_t \ D_t) ∪ B_t
    祖先闭合：AncOK(B) = { e ∈ B : Anc(e) ⊆ B }
    活动集递归：A_{t+1} = AncOK[ (A_t \ D_t) ∪ B_t ]

  §八续（页8）：祖先闭合保证 `e ∈ A_t ⟹ 所有祖先元素也在 A_t`。

  语义动机：一个元素 e 激活时，其全部祖先链 α_e → α_{α_e} → … → root 必须也在活动集，
  否则覆盖不闭合（子声部漂浮在不存在的父声部上）。M16 是 mutex spec 标的「W9 仅先关后开缺
  祖先闭合」的修正——互斥分类用 AncOK 强制「父关则子关」，比 C31 多一层。

  这是「**先关后开**（W9 `targetActiveSet`）→ **祖先闭合裁剪**（本文件 `ancestorClose`）」
  两步活动集递归的第二步。下游 W14 集成 M28（压缩式内嵌 AncOK）引本文件的
  `ancestorClose` / `AncOK` / `ancestorClose_anc_closed` 作为祖先闭合后激活集的 canonical 表示。

★owner 边界（互斥铁律）：本文件**新建**，**不**扩展 `FullDefinitionStrategy.lean` /
  `SeparateStrategyTarget.lean`（共享文件 owner 冲突）。import `SeparateStrategyTarget`（W9）
  仅为**复用其 `SyntaxElement` / `targetActiveSet` / `startingSet` / `endingSet`**——本文件在 W9
  的「先关后开」激活集上施加祖先闭合，不重定义 W9 任何对象。不碰 lakefile.toml；
  root 名 `Origin.AncestorClosure` 待 Lead 登记。

★诚实标注（231号 / formalization-validity-domain）：

  - 本文件 = **L0**（纯结构定义，不依赖任何数据）。祖先闭合是**树结构上的递归谓词**——从 §八
    定义直接转写 + 列表子集/传递闭包的代数恒等，信息增量为零（同义反复：PDF 文字 ↦ Lean 定义/引理）。
    祖先闭合是「语法层活动集的结构约束」（覆盖闭合性），**不是**实盘盈利保证、不是「净账户每笔盈利」。

  - **依附对象 α_e 的元素级表达**（§一 五元组 `e=(I_e,ℓ_e,ε_e,α_e,ρ_e)` 的 α_e）：本文件把
    祖先关系表示为**可分离参数** `par : SyntaxElement → Option SyntaxElement`（父元素，根元素
    `par e = none` 对应 §2.1 `ρ_e=Root ⟹ α_e=∅`）。W9 的 `SyntaxElement` 已留 `parent : Option Nat`
    作 schema 字段（C31/C32 不读父，见 W9 docstring §B），M16 的祖先闭合需要**元素级**依附关系
    （`Anc(e)⊆B` 中 Anc(e) 是元素集合，非索引集合）——故本文件以 `par`（元素级父函数）承载 §一 α_e，
    与 W9 把 `legAssignInjective` 作可分离谓词同构，**不耦进 `SyntaxElement` 构造**（no-workaround：
    本文件不改 W9 的 `SyntaxElement`，不写索引解析垫片，只严格定义 M16 自身需要的元素级祖先关系）。

  - **祖先链有限性**（§十九 元素树 `T=(E,par)` 树深有限）：祖先链 α_e→α_{α_e}→…→root 由树深界定，
    本文件用 **fuel 界定的上溯**（`ancestors par fuel e`，fuel = 树深上界）严格表达有限祖先链——
    fuel 充分大时收集 e 的全部祖先。这是 §十九「树深有限归纳」的结构前置，不是近似（fuel ≥ 树深时
    `ancestors` 已闭合，见 `ancestors_succ`）。

依赖方向（单向无环，全 committed 只读）：AncestorClosure → SeparateStrategyTarget → FullDefinitionStrategy → …（纯 Origin）。

命名空间 NewChanlun.Origin.AncestorClosure。
-/

import Origin.SeparateStrategyTarget

namespace NewChanlun.Origin.AncestorClosure

open NewChanlun.Origin.SeparateStrategyTarget
  (SyntaxElement targetActiveSet startingSet endingSet mem_targetActiveSet)

/-! ## §A 依附对象 α_e 的元素级父函数（§一 五元组 α_e，§2.1 根 α_e=∅）

§一 line（页1–2）：元素五元组 `e=(I_e,ℓ_e,ε_e,α_e,ρ_e)`，`α_e` = 操作语义中的**依附对象**。
§2.1（页2）：`ρ_e=Root ⟹ α_e=∅`（根元素无依附对象）。

★本文件把 α_e 表示为**元素级父函数** `par : SyntaxElement → Option SyntaxElement`：
`par e = some p` 表示 e 依附于父元素 p（`α_e = p`）；`par e = none` 表示 e 是根（`α_e = ∅`，
即 §2.1 `ρ_e=Root`）。这是可分离参数（不耦进 W9 `SyntaxElement` 构造），承载 M16 祖先闭合
所需的元素级依附关系。-/

/-- 元素 e 是根元素（无依附对象，§2.1 `α_e=∅`）：`par e = none`。 -/
def IsRoot (par : SyntaxElement -> Option SyntaxElement) (e : SyntaxElement) : Prop :=
  par e = none

/-- 元素 e 直接依附于 p（`α_e = p`，§一）：`par e = some p`。 -/
def IsParent (par : SyntaxElement -> Option SyntaxElement)
    (p e : SyntaxElement) : Prop :=
  par e = some p

/-! ## §B 祖先链 Anc(e)（§一 α_e→α_{α_e}→…→root；§十九 树深有限）

§八（页7）：`Anc(e)` = e 的全部祖先元素集合（祖先链 α_e → α_{α_e} → … → root）。
§十九（页15）：元素树 `T=(E,par)` 树深有限——祖先链有限。

★用 **fuel 界定的上溯** `ancestors par fuel e` 严格表达有限祖先链：从 e 的父 `par e` 出发，
沿 par 链上溯至多 fuel 步，收集途经的祖先元素。fuel = 树深上界时收集全部祖先（`ancestors_succ`
给出递归展开，fuel ≥ 链长时已闭合）。`fuel=0` 时空链（尚未上溯）。-/

/--
祖先链 `Anc(e)`（§八，§一 α_e 链）：从 e 沿父函数 `par` 上溯至多 `fuel` 步收集的祖先列表。

- `fuel=0`：`[]`（不上溯，空祖先链）。
- `fuel=n+1`：若 `par e = some p` 则 `p :: ancestors par n p`（父 p + p 的祖先），否则 `[]`（e 是根）。

★`ancestors` 自顶向下：列表头是直接父 α_e，其后是 α_{α_e}、…、root。fuel 是树深上界
（§十九 树深有限），fuel ≥ 实际链长时收集全部祖先（`ancestors_succ` 展开见 §C）。
-/
def ancestors (par : SyntaxElement -> Option SyntaxElement) :
    Nat -> SyntaxElement -> List SyntaxElement
  | 0, _ => []
  | n + 1, e =>
    match par e with
    | none => []
    | some p => p :: ancestors par n p

/-- `fuel=0` 祖先链为空（不上溯）。 -/
theorem ancestors_zero (par : SyntaxElement -> Option SyntaxElement) (e : SyntaxElement) :
    ancestors par 0 e = [] :=
  rfl

/-- 根元素（`par e = none`，§2.1 α_e=∅）祖先链为空——根无祖先。 -/
theorem ancestors_root
    (par : SyntaxElement -> Option SyntaxElement) (n : Nat) (e : SyntaxElement)
    (hRoot : par e = none) :
    ancestors par (n + 1) e = [] := by
  simp [ancestors, hRoot]

/--
祖先链上溯一步（§八，§一 α_e 链展开）：若 `par e = some p`（e 依附父 p），则
`Anc(e) = p :: Anc(p)`——e 的祖先 = 直接父 p ∪ p 的祖先。这是 §十九 树深归纳的递归步。
-/
theorem ancestors_succ
    (par : SyntaxElement -> Option SyntaxElement) (n : Nat) (e p : SyntaxElement)
    (hPar : par e = some p) :
    ancestors par (n + 1) e = p :: ancestors par n p := by
  simp [ancestors, hPar]

/--
直接父在祖先链中（§八）：若 `par e = some p`（fuel ≥ 1），则 p ∈ Anc(e)——直接父是祖先。
这是 §八祖先闭合「e∈A ⟹ par(e)∈A」证明的关键：父恒在祖先链头。
-/
theorem parent_mem_ancestors
    (par : SyntaxElement -> Option SyntaxElement) (n : Nat) (e p : SyntaxElement)
    (hPar : par e = some p) :
    p ∈ ancestors par (n + 1) e := by
  rw [ancestors_succ par n e p hPar]
  exact List.mem_cons_self

/--
父的祖先包含于 e 的祖先（§八，祖先链传递）：若 `par e = some p`（fuel=n+1），则
p 的祖先（`ancestors par n p`）全部是 e 的祖先（`ancestors par (n+1) e`）。
这是「父关则子关」的传递闭包侧：e 的祖先 ⊇ {p} ∪ p 的祖先。
-/
theorem parent_ancestors_subset
    (par : SyntaxElement -> Option SyntaxElement) (n : Nat) (e p : SyntaxElement)
    (hPar : par e = some p) (a : SyntaxElement) (ha : a ∈ ancestors par n p) :
    a ∈ ancestors par (n + 1) e := by
  rw [ancestors_succ par n e p hPar]
  exact List.mem_cons_of_mem p ha

/-! ## §C 祖先闭合谓词 AncOK 与成员判据（§八 `AncOK(B)={e∈B:Anc(e)⊆B}`）

§八 boxed：`AncOK(B) = { e ∈ B : Anc(e) ⊆ B }`——激活集 B 中只保留「全部祖先也在 B」的元素。

★用 fuel-bounded `ancestors` 表达 `Anc(e)⊆B`：`ancestors par fuel e` 的每个元素都在 B 中。
`AncestorsIn par fuel B e` = 「e 的（fuel 界内）全部祖先都在 B」的谓词。
`ancOK par fuel B` = B 过滤出满足 `AncestorsIn` 的元素（§八 AncOK(B) 的可计算实现）。-/

/--
谓词「e 的祖先全部在 B 中」（§八 `Anc(e) ⊆ B`）：e 的 fuel 界内每个祖先都是 B 的成员。
对应 §八 AncOK 的成员条件 `Anc(e) ⊆ B`。
-/
def AncestorsIn
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) (e : SyntaxElement) : Prop :=
  ∀ a, a ∈ ancestors par fuel e -> a ∈ B

/-- `AncestorsIn` 可判定（祖先链有限 + `SyntaxElement` 有 `DecidableEq`），故 AncOK 可计算。 -/
instance
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) (e : SyntaxElement) :
    Decidable (AncestorsIn par fuel B e) := by
  unfold AncestorsIn
  exact List.decidableBAll (fun a => a ∈ B) (ancestors par fuel e)

/--
祖先闭合算子 `AncOK(B)`（§八 boxed）：B 中保留「全部祖先也在 B」的元素。

`ancOK par fuel B = B.filter (AncestorsIn par fuel B ·)`——逐元素检查其祖先链是否完全落在 B。
注意 `AncestorsIn` 的 B 参数是**原始 B**（§八 `Anc(e)⊆B` 检查的是原集合，非过滤后），
故这是一次性裁剪（保留祖先齐全的元素）。
-/
def ancOK
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) : List SyntaxElement :=
  B.filter (fun e => decide (AncestorsIn par fuel B e))

/--
AncOK 成员判据（§八 `AncOK(B)={e∈B:Anc(e)⊆B}` 的可判定展开）：
`e ∈ AncOK(B) ↔ (e ∈ B ∧ e 的全部祖先 ∈ B)`。下游 W14 引此判定祖先闭合后元素是否激活。
-/
theorem mem_ancOK
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) (e : SyntaxElement) :
    e ∈ ancOK par fuel B ↔ (e ∈ B ∧ AncestorsIn par fuel B e) := by
  unfold ancOK
  rw [List.mem_filter]
  constructor
  · rintro ⟨hB, hdec⟩
    exact ⟨hB, of_decide_eq_true hdec⟩
  · rintro ⟨hB, hAnc⟩
    exact ⟨hB, decide_eq_true hAnc⟩

/--
AncOK 是子集（§八）：`AncOK(B) ⊆ B`——祖先闭合只裁剪不新增（保留祖先齐全者）。
-/
theorem ancOK_subset
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) (e : SyntaxElement)
    (he : e ∈ ancOK par fuel B) :
    e ∈ B :=
  ((mem_ancOK par fuel B e).mp he).1

/-! ## §D 祖先闭合活动集递归 `A_{t+1}=AncOK[(A_t\D_t)∪B_t]`（§八 boxed 总式）

§八 boxed：`A_{t+1} = AncOK[ (A_t \ D_t) ∪ B_t ]`——先关后开（W9 `targetActiveSet`）后施祖先闭合。

★`ancestorClose` = 对 W9 的「先关后开」激活集 `targetActiveSet active ending starting`
（= `(A_t\D_t)∪B_t`）施加 `ancOK`。两步活动集递归（W9 先关后开 → 本文件祖先闭合裁剪）的合成。

★票#247（rust 对应实装的声明一致，纯注释不改任何语句）：rust 生产路径
（`rust/src/theta_v0/strategy/coverage.rs` 环6 `coverage_step_from_buckets`）的活动集转移含
**显式第三来源** `∪RegistryRestore`——persistent registry 中本已在场的持久祖先（LiveDetached）
经 `restore_ancestor_chain_from_registry` 在 per-bar 因果树上**物化**入 raw（anc.pdf §11 归纳：
每条未关闭腿的操作父 live ⟹ depth<d 祖先全在 raw ⟹ AncOK 通过）。物化机制**非新数学来源**——
Lean 理想式 `AncOK[(A_t\D_t)∪B_t]` 不变（物化不改递归的数学内容，只把 persistent 在场的祖先
展示进 raw 使 `AncestorsIn` 可判）。-/

/--
祖先闭合活动集 `A_{t+1} = AncOK[(A_t\D_t)∪B_t]`（§八 boxed 总式，M16 顶点定义）。

合成两步：
1. **先关后开**（W9）：`targetActiveSet active ending starting = (A_t\D_t)∪B_t`。
2. **祖先闭合**（本文件）：`ancOK par fuel (·)` 裁剪掉祖先不齐的元素。

参数 `active`（A_t）/ `ending`（D_t）/ `starting`（B_t）来自 W9；`par`（α_e 元素级父函数）/
`fuel`（树深上界，§十九）来自本文件。这是互斥分类活动集递归相对 C31「先关后开」的精化顶点。
-/
def ancestorClose
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (active ending starting : List SyntaxElement) : List SyntaxElement :=
  ancOK par fuel (targetActiveSet active ending starting)

/--
祖先闭合活动集成员判据（§八 + W9 `mem_targetActiveSet` 合成）：
`e ∈ A_{t+1} ↔ (先关后开成员 ∧ 祖先齐全)`，展开为
`e ∈ A_{t+1} ↔ ((e∈A_t∧e∉D_t)∨e∈B_t) ∧ Anc(e)⊆[(A_t\D_t)∪B_t]`。

★这是 M16 的核心成员刻画：一个元素激活 ⟺ 它先关后开后在场 **且** 它的全部祖先也先关后开后在场。
下游 W14（M28 压缩式 `LegTarget(AncOK[(A\D)∪B])`）引此判定目标头寸非零元素。
-/
theorem mem_ancestorClose
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (active ending starting : List SyntaxElement) (e : SyntaxElement) :
    e ∈ ancestorClose par fuel active ending starting ↔
      (((e ∈ active ∧ e ∉ ending) ∨ e ∈ starting)
        ∧ AncestorsIn par fuel (targetActiveSet active ending starting) e) := by
  unfold ancestorClose
  rw [mem_ancOK]
  rw [mem_targetActiveSet]

/--
祖先闭合后激活集 ⊆ 先关后开激活集（§八，AncOK 只裁剪）：
`A_{t+1} ⊆ (A_t\D_t)∪B_t`——祖先闭合不引入 W9 激活集之外的元素。
-/
theorem ancestorClose_subset_raw
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (active ending starting : List SyntaxElement) (e : SyntaxElement)
    (he : e ∈ ancestorClose par fuel active ending starting) :
    e ∈ targetActiveSet active ending starting :=
  ancOK_subset par fuel (targetActiveSet active ending starting) e he

/-! ## §E 祖先闭合保证「e∈A ⟹ par(e)∈A」（§八续 页8 核心性质）

§八续（页8）：祖先闭合保证 `e ∈ A_t ⟹ 所有祖先元素也在 A_t`。

★这是 M16 相对 C31 的**全部价值所在**：先关后开（C31）不保证父在子在，祖先闭合（M16）强制之。
本节给出该性质的精确形式——「直接父在」（`ancestorClose_parent_mem`）+「全祖先在」
（`ancestorClose_anc_closed`）。W14 集成 M28 须引此保证覆盖不漂浮。-/

/--
**祖先闭合核心保证（直接父侧）**（§八续 页8）：
若 e ∈ A_{t+1}（祖先闭合活动集，fuel=n+1）且 `par e = some p`（e 有直接父 p），则
`p ∈ (A_t\D_t)∪B_t`（W9 先关后开激活集）。「子激活 ⟹ 直接父在场」。

这是 §八「e∈A_t ⟹ 祖先在 A_t」的直接父实例。证明：e∈AncOK ⟹ Anc(e)⊆(A\D)∪B（祖先齐全
条件），而 p=par(e)∈Anc(e)（`parent_mem_ancestors`），故 p∈(A\D)∪B。
-/
theorem parent_in_raw_of_mem_ancestorClose
    (par : SyntaxElement -> Option SyntaxElement) (n : Nat)
    (active ending starting : List SyntaxElement) (e p : SyntaxElement)
    (he : e ∈ ancestorClose par (n + 1) active ending starting)
    (hPar : par e = some p) :
    p ∈ targetActiveSet active ending starting := by
  rw [mem_ancestorClose] at he
  obtain ⟨_, hAnc⟩ := he
  exact hAnc p (parent_mem_ancestors par n e p hPar)

/--
**祖先闭合核心保证（全祖先侧）**（§八续 页8 原文「所有祖先元素也在 A_t」）：
若 e ∈ A_{t+1}（祖先闭合活动集），则 e 的**全部祖先**都在 W9 先关后开激活集 `(A_t\D_t)∪B_t` 中。

这是 §八续页8 的精确 Lean 形式：祖先闭合 ⟹ 激活元素的整条祖先链在场（覆盖不漂浮）。
证明：e∈AncOK ⟹ `AncestorsIn`（祖先齐全条件）直接给出 ∀ 祖先 ∈ (A\D)∪B。
-/
theorem ancestorClose_anc_closed
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (active ending starting : List SyntaxElement) (e : SyntaxElement)
    (he : e ∈ ancestorClose par fuel active ending starting) :
    ∀ a, a ∈ ancestors par fuel e -> a ∈ targetActiveSet active ending starting := by
  rw [mem_ancestorClose] at he
  exact he.2

/--
**祖先闭合的幂等性引理（祖先齐全元素被保留）**（§八）：若 e 在先关后开激活集且其全部祖先也在，
则 e 在祖先闭合活动集 `A_{t+1}`——祖先齐全的元素不被裁剪。这是 AncOK「只裁剪祖先不齐者」的
正面陈述（与 `ancestorClose_subset_raw` 的负面裁剪互补）。
-/
theorem mem_ancestorClose_of_anc_closed
    (par : SyntaxElement -> Option SyntaxElement) (fuel : Nat)
    (active ending starting : List SyntaxElement) (e : SyntaxElement)
    (hRaw : e ∈ targetActiveSet active ending starting)
    (hAnc : ∀ a, a ∈ ancestors par fuel e -> a ∈ targetActiveSet active ending starting) :
    e ∈ ancestorClose par fuel active ending starting := by
  rw [mem_ancestorClose]
  refine ⟨?_, hAnc⟩
  rw [mem_targetActiveSet] at hRaw
  exact hRaw

end NewChanlun.Origin.AncestorClosure
