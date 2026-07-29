/-
  Origin/ActiveSet.lean — 活动集递归更新 A_{t+1}=AncOK((A_t∖𝒟)∪ℬ) + 目标头寸 p̃（七链环6）

  ═══════════════════════════════════════════════════════════════════════════
  唯一信源
  ═══════════════════════════════════════════════════════════════════════════
  权威 spec：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
  §13（P11 活动集，line 657-687）+ §14（目标头寸 p̃，line 689-709）。逐字方框：

      §13（line 657-669）：
        A^{raw}_{t+1} = (A_t ∖ 𝒟_x) ∪ ℬ_x            （先去关闭、再加开启）
        A_{t+1} = AncOK(A^{raw}_{t+1})
        AncOK(A) = { a ∈ A : Anc(a) ⊆ A }
        a ∈ A_{t+1} ⟹ Anc(a) ⊆ A_{t+1}              （子腿存在则父容器存在）
        p : C_ℓ → C_{ℓ+1}                            （祖先只依赖相邻级别，自相似）
        S_k AncOK(A) = AncOK(S_k A)                   （级别平移不变）
      §14（line 691-703）：
        p̃_{t+1} = Σ_{g∈A_{t+1}} Leg(g)
                = Σ_{δ_g=+1} s_g e^+_{ν(g)} + Σ_{δ_g=-1} s_g e^-_{ν(g)}
        ∀x ∃! p̃_{t+1}.

  ★#247 缺口一：rust 生产实装的转移是**三来源** `AncOK[(A_t∖𝒟_x^†)∪ℬ_x∪ℛ_x]`（ℛ_x =
  RegistryRestore，从 persistent registry 恢复的操作祖先）。本文件的两来源 `rawUpdate` 是其
  `ℛ_x=∅` 限制——**对应关系与有效域声明见 §5′**。

  缺口矩阵 `.chanlun/specs/2026-06-28-lean-existing-vs-20page-gap-matrix.md`（环6a/6b）：
      环6a AncOK 判「已覆盖 → reconcile/复用 `Origin.AncestorClosure`，勿新建闭包」，只建 A^raw 更新；
      环6b p̃ 判「reconcile/补装 → 在 `SeparateLedger.Leg` 上 fold A_{t+1}→p̃，∃! 由 fold 确定性」。

  ═══════════════════════════════════════════════════════════════════════════
  ★类型谱系接缝（no-workaround：精确描述，不缝补假数据）—— 见 §7 still-MISSING + 结果包
  ═══════════════════════════════════════════════════════════════════════════
  七链（本 spec）的环5 输出 `RThetaInterp.Triple{D,B,K}` 三桶是 **`List Cand`**（候选三元组
  (lvl,exec,bsp)）。而既有 `AncestorClosure.ancOK` 是 **单态于 `SyntaxElement`**（互斥分类 M16 谱系，
  另一份 spec `推导完全互斥分类.pdf`）—— `Cand` 无 λ/ρ/parent-index，二者**不能跨类型直接组合**。

  缺口矩阵「复用 AncestorClosure，勿新建闭包」预设了二者类型对齐，而该预设**不成立**。严格（no-patch）
  的消解不是缝一个伪 `Cand→SyntaxElement` 桥（要凭空造 λ/ρ）、也不是改 AncestorClosure（非本工位/
  缺口矩阵要求复用），而是：
    (A) 把祖先闭合**多态化**（`ancestorsG/AncestorsInG/ancOKG` over `{α}[DecidableEq α]`），
    (B) **证多态闭合在 `α:=SyntaxElement` 上与 `AncestorClosure.ancOK` 逐点相等**
        （`ancOKG_eq_ancOK`）—— 这是「复用」的严格兑现：单一权威经**等价证明**抬升为多态，**非分叉**，
    (C) 在 `α:=Cand` 上实例化，喂入环5 三桶（消费 Triple）。
  这样既消费 `List Cand` 三桶（环5），又复用 AncestorClosure 的闭合逻辑（经等价），不造伪桥。

  ═══════════════════════════════════════════════════════════════════════════
  认识论等级（formalization-validity-domain / 231号 强制标注）
  ═══════════════════════════════════════════════════════════════════════════
  全部 **L0**（纯定义 / List filter·fold / 函数确定性，**不依赖任何数据**）。
  - A^raw 更新 = List 差集 ∪ 并集；A_{t+1}=AncOK 多态 filter；p̃ = List.map+fold。
  - 主性质 `a∈A_{t+1}⟹Anc(a)⊆A_{t+1}`（祖先闭合）、`∀ ∃! p̃`（fold 确定性）、`S_k AncOK=AncOK S_k`
    （平移不变）均为 L0 结构定理，信息增量 = 同义反复（spec 文字 ↦ Lean 定义/引理）。
  - **边界**：完整链闭合 `Anc(a)⊆A_{t+1}`（强于 `⊆A^raw`）在 **fuel 饱和**（fuel ≥ 树深，§十九 树深
    有限——AncestorClosure docstring 明示「fuel=树深上界」）下成立；平移不变在 **par 等变**（祖先关系
    级别平移自相似，spec p:C_ℓ→C_{ℓ+1}）下成立。两前提显式为假设，不静默假定（声明诚实）。
  - L0 唯一化/闭合与买卖点择时 v1 实盘盈利性（L2/L3，记忆 `newchanlun-v1-fullwindow-l3-falsified` 8/8
    否证）**认识论等级不同、不可互相否证**——本文件不预判 alpha。

  范式：纯 List/Bool/Nat/Int + Lean 核心 `Init`。仅 import 三纯 core 兄弟文件（RThetaInterp 环5 +
  AncestorClosure 环6a + SeparateLedger 环6b）——**无** Mathlib/Batteries/Std。禁 sorry/admit/axiom。
  禁 Fintype/Finset（活动集用 List）。不编辑 lakefile（报 Lead 登记 root `Origin.ActiveSet`）。
-/

import Origin.RThetaInterp
import Origin.AncestorClosure
import Origin.SeparateLedger

open NewChanlun.Origin (Side ExistsUnique)
open NewChanlun.Origin.CandidateSet (Cand State gamma dirOf)
open NewChanlun.Origin.RThetaInterp (Triple RΘ shift)
open NewChanlun.Origin.SeparateLedger
  (Leg legZero legLong legShort legNet Net SepPosition net_append)
open NewChanlun.Origin.SeparateStrategyTarget (SyntaxElement)

namespace NewChanlun.Origin.ActiveSet

/-! ═══════════════════════════════════════════════════════════════════════
    § 0. 小工具（纯 core，无 Mathlib）
    ═══════════════════════════════════════════════════════════════════════
    纯 core 下 `a ∈ (l : List Cand)`（Prop/List.Mem）**无**自动 `Decidable` 实例（CandidateSet
    docstring 明示，故全程用 `DecidableEq` 派生的 Bool 成员 `memD`，不依赖 List.Mem 的判定实例）。 -/

/-- Bool 外延：两 Bool 同真值 ⟹ 相等（纯 core 经 `cases` 归约）。 -/
theorem bool_ext {b1 b2 : Bool} (h : b1 = true ↔ b2 = true) : b1 = b2 := by
  cases b1 <;> cases b2 <;> simp_all

/-- Bool 成员判定 `memD a l`（DecidableEq 派生，绕开缺失的 List.Mem 判定实例）：l 中存在等于 a 者。 -/
def memD {α : Type} [DecidableEq α] (a : α) (l : List α) : Bool :=
  l.any (fun b => decide (a = b))

/-- `memD a l = true ↔ a ∈ l`（Bool 成员 ↔ Prop 成员）。 -/
theorem memD_iff {α : Type} [DecidableEq α] (a : α) (l : List α) :
    memD a l = true ↔ a ∈ l := by
  unfold memD
  rw [List.any_eq_true]
  constructor
  · rintro ⟨b, hb, hab⟩
    rw [decide_eq_true_eq] at hab
    subst hab; exact hb
  · intro h
    exact ⟨a, h, decide_eq_true rfl⟩

/-- `memD a l = false ↔ a ∉ l`。 -/
theorem memD_false_iff {α : Type} [DecidableEq α] (a : α) (l : List α) :
    memD a l = false ↔ a ∉ l := by
  rw [← memD_iff a l]
  cases memD a l <;> simp

/-- map/filter 交换（纯 core，避免依赖 `List.filter_congr` 等版本差异）：
    `(l.filter p).map f = (l.map f).filter q`，前提 `∀ x, q (f x) = p x`。 -/
theorem map_filter_comm {α β : Type} (f : α → β) (p : α → Bool) (q : β → Bool)
    (h : ∀ x, q (f x) = p x) : ∀ l : List α, (l.filter p).map f = (l.map f).filter q := by
  intro l
  induction l with
  | nil => rfl
  | cons a as ih =>
    simp only [List.filter_cons, List.map_cons, h a]
    by_cases hp : p a = true
    · simp [hp, ih]
    · simp only [Bool.not_eq_true] at hp
      simp [hp, ih]

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 多态祖先闭合（`{α}[DecidableEq α]`）—— AncestorClosure 逻辑的保守多态化
    ═══════════════════════════════════════════════════════════════════════
    与 `AncestorClosure.ancestors/AncestorsIn/ancOK` 结构同源，仅把载体 `SyntaxElement`
    抽象为参数 `α`。§3 证 `α:=SyntaxElement` 上与 AncestorClosure 逐点相等（复用兑现）。 -/

/-- 多态祖先链 `Anc(e)`（fuel 界定上溯，对齐 `AncestorClosure.ancestors`）。 -/
def ancestorsG {α : Type} (par : α → Option α) : Nat → α → List α
  | 0, _ => []
  | n + 1, e =>
    match par e with
    | none => []
    | some p => p :: ancestorsG par n p

/-- 谓词「e 的祖先全在 B」（§13 `Anc(e) ⊆ A`，Prop 形式，供定理陈述）。 -/
def AncestorsInG {α : Type} [DecidableEq α] (par : α → Option α) (fuel : Nat)
    (B : List α) (e : α) : Prop :=
  ∀ a, a ∈ ancestorsG par fuel e → a ∈ B

/-- 「e 的祖先全在 B」的 Bool 判定（用 `memD`，绕开缺失的 List.Mem 判定实例）。 -/
def AncestorsInB {α : Type} [DecidableEq α] (par : α → Option α) (fuel : Nat)
    (B : List α) (e : α) : Bool :=
  (ancestorsG par fuel e).all (fun a => memD a B)

/-- `AncestorsInB = true ↔ AncestorsInG`（Bool 判定 ↔ Prop 谓词）。 -/
theorem AncestorsInB_iff {α : Type} [DecidableEq α] (par : α → Option α) (fuel : Nat)
    (B : List α) (e : α) :
    AncestorsInB par fuel B e = true ↔ AncestorsInG par fuel B e := by
  unfold AncestorsInB AncestorsInG
  rw [List.all_eq_true]
  constructor
  · intro h a ha; exact (memD_iff a B).mp (h a ha)
  · intro h a ha; exact (memD_iff a B).mpr (h a ha)

/-- 多态祖先闭合算子 `AncOK(A) = {a∈A : Anc(a)⊆A}`（§13 boxed，对齐 `AncestorClosure.ancOK`）。 -/
def ancOKG {α : Type} [DecidableEq α] (par : α → Option α) (fuel : Nat)
    (B : List α) : List α :=
  B.filter (fun e => AncestorsInB par fuel B e)

/-- AncOK 成员判据（§13）：`e∈AncOK(B) ↔ (e∈B ∧ Anc(e)⊆B)`。 -/
theorem mem_ancOKG {α : Type} [DecidableEq α] (par : α → Option α) (fuel : Nat)
    (B : List α) (e : α) :
    e ∈ ancOKG par fuel B ↔ (e ∈ B ∧ AncestorsInG par fuel B e) := by
  unfold ancOKG
  rw [List.mem_filter, AncestorsInB_iff]

/-- AncOK 只裁剪：`AncOK(B) ⊆ B`。 -/
theorem ancOKG_subset {α : Type} [DecidableEq α] (par : α → Option α) (fuel : Nat)
    (B : List α) (e : α) (he : e ∈ ancOKG par fuel B) : e ∈ B :=
  ((mem_ancOKG par fuel B e).mp he).1

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. fuel 饱和下的祖先链不动点 + 传递性（§十九 树深有限 = fuel 饱和）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- fuel 饱和（fuel ≥ 树深，§十九）：多上溯一步不增祖先。AncestorClosure docstring 明示
    「fuel = 树深上界」即此正则区。 -/
def Saturated {α : Type} (par : α → Option α) (fuel : Nat) : Prop :=
  ∀ e, ancestorsG par (fuel + 1) e = ancestorsG par fuel e

/-- 饱和下祖先链不动点展开：`par a = some p ⟹ Anc(a) = p :: Anc(p)`（同 fuel，无 off-by-one）。 -/
theorem ancestorsG_unfold_sat {α : Type} (par : α → Option α) (fuel : Nat)
    (hSat : Saturated par fuel) (a p : α) (hpar : par a = some p) :
    ancestorsG par fuel a = p :: ancestorsG par fuel p := by
  rw [← hSat a]
  simp only [ancestorsG, hpar]

/-- 祖先链传递性（饱和下，§13 「祖先的祖先仍是祖先」）：`b∈Anc(a) ⟹ Anc(b)⊆Anc(a)`。
    按搜索深度 d 归纳，闭合 fuel 保持饱和不变（避免 fuel-归纳与定 fuel 假设冲突）。 -/
theorem ancestorsG_trans {α : Type} (par : α → Option α) (fuel : Nat)
    (hSat : Saturated par fuel) :
    ∀ d a b, b ∈ ancestorsG par d a →
      ∀ c, c ∈ ancestorsG par fuel b → c ∈ ancestorsG par fuel a := by
  intro d
  induction d with
  | zero => intro a b hb; simp only [ancestorsG] at hb; exact absurd hb (List.not_mem_nil)
  | succ k ih =>
    intro a b hb c hc
    cases hpar : par a with
    | none => simp only [ancestorsG, hpar] at hb; exact absurd hb (List.not_mem_nil)
    | some p =>
      simp only [ancestorsG, hpar] at hb
      have hsub : ancestorsG par fuel p ⊆ ancestorsG par fuel a := by
        rw [ancestorsG_unfold_sat par fuel hSat a p hpar]
        exact fun x hx => List.mem_cons_of_mem p hx
      rcases List.mem_cons.mp hb with hbp | hbtail
      · rw [hbp] at hc
        exact hsub hc
      · exact hsub (ih p b hbtail c hc)

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 复用兑现：多态闭合在 SyntaxElement 上 = AncestorClosure.ancOK（单一权威经等价）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- `ancestorsG` 在 `SyntaxElement` 上逐项等于 `AncestorClosure.ancestors`（fuel 归纳）。 -/
theorem ancestorsG_eq_anc (par : SyntaxElement → Option SyntaxElement) :
    ∀ fuel e, ancestorsG par fuel e = NewChanlun.Origin.AncestorClosure.ancestors par fuel e := by
  intro fuel
  induction fuel with
  | zero => intro e; rfl
  | succ n ih =>
    intro e
    cases hpe : par e with
    | none =>
      simp only [ancestorsG, NewChanlun.Origin.AncestorClosure.ancestors, hpe]
    | some p =>
      simp only [ancestorsG, NewChanlun.Origin.AncestorClosure.ancestors, hpe, ih p]

/-- `AncestorsInG ↔ AncestorClosure.AncestorsIn`（由祖先链相等）。 -/
theorem AncestorsInG_iff_anc (par : SyntaxElement → Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) (e : SyntaxElement) :
    AncestorsInG par fuel B e ↔
      NewChanlun.Origin.AncestorClosure.AncestorsIn par fuel B e := by
  unfold AncestorsInG NewChanlun.Origin.AncestorClosure.AncestorsIn
  rw [ancestorsG_eq_anc par fuel e]

/-- **★复用兑现（§13 环6a）**：多态 `ancOKG` 在 `SyntaxElement` 上**逐点等于** 既有
    `AncestorClosure.ancOK`。这坐实本文件不是分叉的第二份闭合，而是单一权威（AncestorClosure）
    经等价证明抬升为多态——「勿新建闭包」的严格形式。 -/
theorem ancOKG_eq_ancOK (par : SyntaxElement → Option SyntaxElement) (fuel : Nat)
    (B : List SyntaxElement) :
    ancOKG par fuel B = NewChanlun.Origin.AncestorClosure.ancOK par fuel B := by
  unfold ancOKG NewChanlun.Origin.AncestorClosure.ancOK
  have hpred : (fun e => AncestorsInB par fuel B e)
      = (fun e => decide (NewChanlun.Origin.AncestorClosure.AncestorsIn par fuel B e)) := by
    funext e
    apply bool_ext
    rw [AncestorsInB_iff, AncestorsInG_iff_anc, decide_eq_true_eq]
  rw [hpred]

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. A^raw 原始更新（§13 (A_t∖𝒟_x)∪ℬ_x，消费环5 Triple{D,B,K}）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 列表差集 `X ∖ D`（去关闭，§13 A_t∖𝒟_x）：保留不在 D 的元素。 -/
def listDiff (X D : List Cand) : List Cand :=
  X.filter (fun a => !memD a D)

/-- 列表并集 `X ∪ Y`（加开启，去重不重复 X 中已有，集合语义）。 -/
def listUnion (X Y : List Cand) : List Cand :=
  X ++ Y.filter (fun b => !memD b X)

theorem mem_listDiff (X D : List Cand) (a : Cand) :
    a ∈ listDiff X D ↔ (a ∈ X ∧ a ∉ D) := by
  unfold listDiff
  rw [List.mem_filter]
  constructor
  · rintro ⟨hX, hd⟩
    refine ⟨hX, (memD_false_iff a D).mp ?_⟩
    simpa using hd
  · rintro ⟨hX, hd⟩
    refine ⟨hX, ?_⟩
    simpa using (memD_false_iff a D).mpr hd

theorem mem_listUnion (X Y : List Cand) (a : Cand) :
    a ∈ listUnion X Y ↔ (a ∈ X ∨ a ∈ Y) := by
  unfold listUnion
  rw [List.mem_append, List.mem_filter]
  constructor
  · rintro (hX | ⟨hY, _⟩)
    · exact Or.inl hX
    · exact Or.inr hY
  · rintro (hX | hY)
    · exact Or.inl hX
    · cases hmx : memD a X with
      | true => exact Or.inl ((memD_iff a X).mp hmx)
      | false => exact Or.inr ⟨hY, rfl⟩

/-- **★A^raw_{t+1} = (A_t ∖ 𝒟_x) ∪ ℬ_x（§13 原始更新，★消费环5 三桶 Triple）** ——
    `t.D = 𝒟_x`（应关闭，去掉）、`t.B = ℬ_x`（应开启，加入）。`t.K`（记录不执行）**不进**活动集。 -/
def rawUpdate (At : List Cand) (t : Triple) : List Cand :=
  listUnion (listDiff At t.D) t.B

/-- A^raw 成员判据（§13）：`a∈A^raw ↔ ((a∈A_t∧a∉𝒟_x)∨a∈ℬ_x)`。 -/
theorem mem_rawUpdate (At : List Cand) (t : Triple) (a : Cand) :
    a ∈ rawUpdate At t ↔ (((a ∈ At ∧ a ∉ t.D) ∨ a ∈ t.B)) := by
  unfold rawUpdate
  rw [mem_listUnion, mem_listDiff]

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. A_{t+1} = AncOK(A^raw)（§13 祖先闭合，实例化多态闭合于 Cand）+ 闭合保证
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **★A_{t+1} = AncOK[(A_t∖𝒟_x)∪ℬ_x]（§13 顶点定义，★实例化于 Cand）** ——
    `par`（候选祖先 p:C_ℓ→C_{ℓ+1}，元素级父函数，可分离参数）/ `fuel`（树深上界，§十九）。

    ★票#247（rust 对应实装的声明一致，纯注释不改任何语句）：rust 生产路径
    （`rust/src/theta_v0/strategy/coverage.rs` 环6 `coverage_step_from_buckets`）的转移含
    **显式第三来源** `∪RegistryRestore`——persistent registry 持久祖先（LiveDetached）经
    `restore_ancestor_chain_from_registry` 在 per-bar 因果树上物化入 raw（anc.pdf §11），
    **非新数学来源**；本定义的理想式 `AncOK[(A_t∖𝒟_x)∪ℬ_x]` 不变。 -/
def activeNext (par : Cand → Option Cand) (fuel : Nat) (At : List Cand) (t : Triple) : List Cand :=
  ancOKG par fuel (rawUpdate At t)

/-- **★A_{t+1} 直接消费环5 状态 x（spec (𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))）** —— 喂入 `RΘ x` 三桶。 -/
def activeFromState (par : Cand → Option Cand) (fuel : Nat) (At : List Cand) (x : State) : List Cand :=
  activeNext par fuel At (RΘ x)

/-- A_{t+1} 成员判据（§13 + 环5）。 -/
theorem mem_activeNext (par : Cand → Option Cand) (fuel : Nat) (At : List Cand)
    (t : Triple) (e : Cand) :
    e ∈ activeNext par fuel At t ↔
      ((((e ∈ At ∧ e ∉ t.D) ∨ e ∈ t.B))
        ∧ AncestorsInG par fuel (rawUpdate At t) e) := by
  unfold activeNext
  rw [mem_ancOKG, mem_rawUpdate]

/-- A_{t+1} ⊆ A^raw（AncOK 只裁剪）。 -/
theorem activeNext_subset_raw (par : Cand → Option Cand) (fuel : Nat) (At : List Cand)
    (t : Triple) (e : Cand) (he : e ∈ activeNext par fuel At t) :
    e ∈ rawUpdate At t :=
  ancOKG_subset par fuel (rawUpdate At t) e he

/-- **祖先闭合（⊆ A^raw 侧，无条件，对齐 `AncestorClosure.ancestorClose_anc_closed`）** ——
    `a∈A_{t+1} ⟹ Anc(a)⊆A^raw`。这是「用既有 anc_closed」的 Cand 实例（robust，无 fuel 前提）。 -/
theorem activeNext_anc_subset_raw (par : Cand → Option Cand) (fuel : Nat) (At : List Cand)
    (t : Triple) (e : Cand) (he : e ∈ activeNext par fuel At t) :
    ∀ a, a ∈ ancestorsG par fuel e → a ∈ rawUpdate At t := by
  unfold activeNext at he
  exact ((mem_ancOKG par fuel (rawUpdate At t) e).mp he).2

/-- **★祖先闭合（直接父侧，§13「子腿存在则父容器存在」）** —— `a∈A_{t+1}` 且 `par a = some p`
    （a 有父容器 p），饱和下则 `p∈A_{t+1}`。子声部激活 ⟹ 父容器在活动集（覆盖不漂浮）。 -/
theorem parent_mem_activeNext (par : Cand → Option Cand) (fuel : Nat)
    (hSat : Saturated par fuel) (At : List Cand) (t : Triple) (e p : Cand)
    (he : e ∈ activeNext par fuel At t) (hpar : par e = some p) :
    p ∈ activeNext par fuel At t := by
  have hraw := activeNext_anc_subset_raw par fuel At t e he
  rw [ancestorsG_unfold_sat par fuel hSat e p hpar] at hraw
  unfold activeNext
  rw [mem_ancOKG]
  refine ⟨hraw p List.mem_cons_self, ?_⟩
  intro a ha
  exact hraw a (List.mem_cons_of_mem p ha)

/-- **★祖先闭合（完整链侧，§13 boxed `a∈A_{t+1}⟹Anc(a)⊆A_{t+1}`）** —— 饱和下（fuel≥树深，§十九）
    激活元素的**整条祖先链都在 A_{t+1}**。强于 ⊆A^raw：经传递性 `ancestorsG_trans` 把每个祖先也判为
    祖先齐全（故被 AncOK 保留）。 -/
theorem activeNext_anc_closed (par : Cand → Option Cand) (fuel : Nat)
    (hSat : Saturated par fuel) (At : List Cand) (t : Triple) (e : Cand)
    (he : e ∈ activeNext par fuel At t) :
    ∀ a, a ∈ ancestorsG par fuel e → a ∈ activeNext par fuel At t := by
  intro b hb
  have hrawe := activeNext_anc_subset_raw par fuel At t e he
  unfold activeNext
  rw [mem_ancOKG]
  refine ⟨hrawe b hb, ?_⟩
  intro c hc
  exact hrawe c (ancestorsG_trans par fuel hSat fuel e b hb c hc)

/-! ═══════════════════════════════════════════════════════════════════════
    § 5′. ★rust 生产路径对应关系：**第三来源 `ℛ_x = RegistryRestore`**（#247 缺口一）
    ═══════════════════════════════════════════════════════════════════════

    **本节是有效域声明，不是定理**（无 Lean 代码——诚实标注 Lean↔rust 的域差，不缝兼容垫片）。

    §13 / 本文件 `rawUpdate` 的转移是**两来源**：

        A^raw_{t+1} = (A_t ∖ 𝒟_x) ∪ ℬ_x

    rust 生产实装（`rust/src/theta_v0/strategy/coverage.rs::coverage_step_from_buckets_sep`）的
    转移是**三来源**：

        A_{t+1} = AncOK[ (A_t ∖ 𝒟_x^†) ∪ ℬ_x ∪ ℛ_x ]
        ℛ_x = RegistryRestore(A_t, ℬ_x) ⊆ Pi        （persistent registry 恢复的操作祖先）

    `ℛ_x` 的元素**既不在 A_t 的腿里，也不是 ℛ_Θ(Γ(x)) 落入 ℬ_x/𝒦_x 的候选**——它们由
    `restore_ancestor_chain_from_registry` 从持久注册表 Pi（anc.pdf §4）沿 `structural_parent_id`
    链恢复入 A^raw，两条注入路径：(i) held 路——Stale/LiveDetached 持仓腿的 `op_parent` 祖先链；
    (ii) open 路——open 候选父 carrier 的祖先链（当 bar 关闭种子处中断）。
    恢复元素**进 A_{t+1} 并生成目标腿计入 p̃**（非仅作祖先在场性判据）。

    ★对应关系（域差方向）：本文件的 `rawUpdate`/`activeNext` 是 rust 三来源转移在
    **`ℛ_x = ∅` 上的限制**。故本文件全部 L0 定理（`mem_activeNext` / `parent_mem_activeNext` /
    `activeNext_anc_closed` / 平移不变）对生产路径的适用性以 `ℛ_x=∅` 为前提；`ℛ_x≠∅` 的 bar
    （rust 探针 `AncokProbe.restore_calls>0`）**在本文件定义域之外**——不是被本文件否证，也不被
    本文件担保（231号：有效域 ⊊ 定义域，显式声明不静默膨胀）。

    ★形式化 `ℛ_x` 的前提（未做，诚实列出）：Pi 是**跨 bar 持久状态**，`ℛ_x` 依赖 (Pi, A_t, ℬ_x)
    三者而非仅 (A_t, 三桶) ⟹ 需先把 `Cand` 升格为携持久身份 pid + `par` 升格为 Pi 上的结构父函数
    （anc.pdf §5/§6 I1-I5），非本文件（纯 List/无 Mathlib）单文件可容。列为 #59 交接项。

    ═══════════════════════════════════════════════════════════════════════
    § 6. 平移不变 S_k AncOK(A) = AncOK(S_k A)（§13 自相似，par 等变下）
    ═══════════════════════════════════════════════════════════════════════
    S_k 作用于活动集 = `List.map (shift k)`（复用环5 `RThetaInterp.shift`：lvl/exec 同加 k）。 -/

/-- 级别平移 `shift k` 单射（lvl/exec 同加 k 可逆，bsp 不变）。 -/
theorem shift_inj (k : Nat) (a b : Cand) (h : shift k a = shift k b) : a = b := by
  obtain ⟨al, ae, ab⟩ := a
  obtain ⟨bl, be, bb⟩ := b
  simp only [shift, Cand.mk.injEq] at h
  obtain ⟨h1, h2, h3⟩ := h
  simp only [Cand.mk.injEq]
  exact ⟨by omega, by omega, h3⟩

/-- par **等变**（祖先关系级别平移自相似，spec p:C_ℓ→C_{ℓ+1}）：父容器随候选一同平移。 -/
def ParEquivariant (par : Cand → Option Cand) (k : Nat) : Prop :=
  ∀ g, par (shift k g) = Option.map (shift k) (par g)

/-- 祖先链与平移交换（par 等变下）：`Anc(S_k g) = S_k Anc(g)`。 -/
theorem ancestorsG_shift (par : Cand → Option Cand) (k : Nat)
    (hEqui : ParEquivariant par k) :
    ∀ fuel g, ancestorsG par fuel (shift k g) = (ancestorsG par fuel g).map (shift k) := by
  intro fuel
  induction fuel with
  | zero => intro g; rfl
  | succ n ih =>
    intro g
    cases hp : par g with
    | none =>
      have hpn : par (shift k g) = none := by rw [hEqui g, hp]; rfl
      simp only [ancestorsG, hpn, hp, List.map_nil]
    | some p =>
      have hps : par (shift k g) = some (shift k p) := by rw [hEqui g, hp]; rfl
      simp only [ancestorsG, hps, hp, List.map_cons, ih p]

/-- 祖先齐全谓词与平移交换（par 等变 + shift 单射）。 -/
theorem AncestorsInG_shift (par : Cand → Option Cand) (k : Nat)
    (hEqui : ParEquivariant par k) (fuel : Nat) (A : List Cand) (g : Cand) :
    AncestorsInG par fuel A g ↔ AncestorsInG par fuel (A.map (shift k)) (shift k g) := by
  unfold AncestorsInG
  rw [ancestorsG_shift par k hEqui fuel g]
  constructor
  · intro h y hy
    rw [List.mem_map] at hy
    obtain ⟨x, hx, rfl⟩ := hy
    exact List.mem_map.mpr ⟨x, h x hx, rfl⟩
  · intro h x hx
    have hmem : shift k x ∈ A.map (shift k) :=
      h (shift k x) (List.mem_map.mpr ⟨x, hx, rfl⟩)
    rw [List.mem_map] at hmem
    obtain ⟨z, hz, hzeq⟩ := hmem
    have : z = x := shift_inj k z x hzeq
    rw [← this]; exact hz

/-- **★平移不变 S_k AncOK(A) = AncOK(S_k A)（§13 boxed 自相似）** —— par 等变下，祖先闭合与级别
    平移交换。S_k = `List.map (shift k)`。坐实「各级别共用一份闭合规则，无顶层特例」（去根化）。 -/
theorem ancOKG_shift (par : Cand → Option Cand) (k : Nat) (hEqui : ParEquivariant par k)
    (fuel : Nat) (A : List Cand) :
    (ancOKG par fuel A).map (shift k) = ancOKG par fuel (A.map (shift k)) := by
  unfold ancOKG
  exact map_filter_comm (shift k)
    (fun e => AncestorsInB par fuel A e)
    (fun e => AncestorsInB par fuel (A.map (shift k)) e)
    (fun x => bool_ext (by
      rw [AncestorsInB_iff, AncestorsInB_iff]
      exact (AncestorsInG_shift par k hEqui fuel A x).symm)) A

/-- 平移不变在 A_{t+1} 层（A^raw 也随 S_k 平移时）：`S_k A_{t+1} = AncOK(S_k A^raw)`。 -/
theorem activeNext_shift (par : Cand → Option Cand) (k : Nat) (hEqui : ParEquivariant par k)
    (fuel : Nat) (At : List Cand) (t : Triple) :
    (activeNext par fuel At t).map (shift k) = ancOKG par fuel ((rawUpdate At t).map (shift k)) := by
  unfold activeNext
  exact ancOKG_shift par k hEqui fuel (rawUpdate At t)

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 目标头寸 p̃_{t+1} = Σ_{g∈A_{t+1}} Leg(g)（§14，fold 求和 + ∀ ∃!）
    ═══════════════════════════════════════════════════════════════════════
    复用 `SeparateLedger.Leg`（多空双坐标 qPlus/qMinus）/ `legLong`/`legShort`/`Net`/`net_append`。
    腿的单位数 s_g（§14，Cand 不携带）作可分离参数 `sOf : Cand → Nat`（外部数据，不臆造）。 -/

/-- **★单候选的头寸腿 Leg(g)（§14 s_g e^{δ_g}_{ν(g)}，复用 SeparateLedger.Leg）** ——
    方向 δ_g = `dirOf g.bsp`：买点(long) ⟹ 多头腿 `legLong (s_g)`；卖点(short) ⟹ 空头腿 `legShort (s_g)`。 -/
def legOf (sOf : Cand → Nat) (g : Cand) : Leg :=
  match dirOf g.bsp with
  | Side.long  => legLong (sOf g)
  | Side.short => legShort (sOf g)

/-- **★目标头寸 p̃_{t+1} = Σ_{g∈A_{t+1}} Leg(g)（§14，活动集 fold→分账本头寸 SepPosition）** ——
    `A.map (legOf sOf)`：活动集每候选一条腿，构成 `SepPosition`（= List Leg，C25 直积）。 -/
def ptilde (sOf : Cand → Nat) (A : List Cand) : SepPosition :=
  A.map (legOf sOf)

/-- p̃ 在活动集 A_{t+1} 上（§14 整链组装：环5→环6a→环6b）。 -/
def ptildeActive (par : Cand → Option Cand) (fuel : Nat) (At : List Cand)
    (t : Triple) (sOf : Cand → Nat) : SepPosition :=
  ptilde sOf (activeNext par fuel At t)

/-- **★∀ ∃! p̃（§14 boxed ∀x ∃! p̃_{t+1}，★核心）** —— p̃ 由活动集确定性 fold 出，故唯一。
    L0：`ptilde` 是全函数（List.map），像唯一。前提 A_{t+1} 唯一（§13）+ Leg 确定（§9 sOf）。 -/
theorem ptilde_exists_unique (sOf : Cand → Nat) (A : List Cand) :
    ExistsUnique (fun p => ptilde sOf A = p) :=
  ⟨ptilde sOf A, rfl, fun _ hy => hy.symm⟩

/-- p̃ 在活动集上 ∀ ∃!（§14 + §13）。 -/
theorem ptildeActive_exists_unique (par : Cand → Option Cand) (fuel : Nat)
    (At : List Cand) (t : Triple) (sOf : Cand → Nat) :
    ExistsUnique (fun p => ptildeActive par fuel At t sOf = p) :=
  ⟨ptildeActive par fuel At t sOf, rfl, fun _ hy => hy.symm⟩

/-- p̃ 对活动集拼接可加（§14 求和结合性，map 分配 append）。 -/
theorem ptilde_append (sOf : Cand → Nat) (A B : List Cand) :
    ptilde sOf (A ++ B) = ptilde sOf A ++ ptilde sOf B := by
  unfold ptilde
  rw [List.map_append]

/-- **★p̃ 净额可加（§14 + C26，复用 SeparateLedger.net_append）** —— 多空分组后净额线性叠加：
    `Net(p̃(A++B)) = Net(p̃ A) + Net(p̃ B)`。坐实 p̃ 与分账本净额映射一致。 -/
theorem net_ptilde_append (sOf : Cand → Nat) (A B : List Cand) :
    Net (ptilde sOf (A ++ B)) = Net (ptilde sOf A) + Net (ptilde sOf B) := by
  rw [ptilde_append, net_append]

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. 反退化见证（整链真跑通：环5三桶→A^raw→AncOK 剪孤儿→p̃，纯 core decide）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 见证候选：三级买入区间套链 c0(级0,根)→c1(级1)→c2(级2)，par 上溯。 -/
def wc0 : Cand := ⟨0, 0, .b1⟩
def wc1 : Cand := ⟨1, 0, .b1⟩
def wc2 : Cand := ⟨2, 0, .b1⟩

/-- 见证父函数：c2→c1→c0→根（祖先链，p:C_ℓ→C_{ℓ+1} 的具体实例）。 -/
def witPar : Cand → Option Cand
  | ⟨2, 0, .b1⟩ => some wc1
  | ⟨1, 0, .b1⟩ => some wc0
  | _ => none

/-- **★见证：AncOK 剪孤儿（L0，★祖先闭合全部价值）** —— 仅含 c1 但缺父 c0 ⟹ c1 被裁剪（孤儿不漂浮）。 -/
theorem witness_ancOK_drops_orphan : ancOKG witPar 3 [wc1] = [] := by decide

/-- **★见证：祖先齐全保留（L0）** —— c0,c1 父子俱全 ⟹ 全保留。 -/
theorem witness_ancOK_keeps_closed : ancOKG witPar 3 [wc0, wc1] = [wc0, wc1] := by decide

/-- **★见证：A^raw 更新（L0，消费三桶）** —— A_t=[c0]，𝒟=[]，ℬ=[c1] ⟹ A^raw=[c0,c1]。 -/
theorem witness_rawUpdate : rawUpdate [wc0] ⟨[], [wc1], []⟩ = [wc0, wc1] := by decide

/-- **★见证：A_{t+1} 整链（L0）** —— A_t=[c0] + ℬ=[c1]（父 c0 在）⟹ AncOK 保留全部 ⟹ [c0,c1]。 -/
theorem witness_activeNext_closed :
    activeNext witPar 3 [wc0] ⟨[], [wc1], []⟩ = [wc0, wc1] := by decide

/-- **★见证：开启孤儿被剪（L0，★环5→环6 闭合）** —— A_t=[] + ℬ=[c1]（父 c0 缺）⟹ AncOK 剪 c1 ⟹ []。
    坐实「子腿（c1）开启但父容器（c0）不在 ⟹ 子腿不进活动集」。 -/
theorem witness_activeNext_drops_orphan :
    activeNext witPar 3 [] ⟨[], [wc1], []⟩ = [] := by decide

/-- **★见证：去关闭（L0）** —— A_t=[c0,c1]，𝒟=[c1]（关闭 c1）⟹ A^raw=[c0]，AncOK 保留 ⟹ [c0]。 -/
theorem witness_activeNext_close :
    activeNext witPar 3 [wc0, wc1] ⟨[wc1], [], []⟩ = [wc0] := by decide

/-- **★见证：p̃ 头寸（L0，全多头买入腿）** —— 活动集 [c0,c1]，单位 1 ⟹ p̃=[legLong 1, legLong 1]。 -/
theorem witness_ptilde :
    ptilde (fun _ => 1) [wc0, wc1] = [legLong 1, legLong 1] := by decide

/-- **★见证：p̃ 净额（L0）** —— 两条多头腿净额 = 2（多头记正）。 -/
theorem witness_ptilde_net :
    Net (ptilde (fun _ => 1) [wc0, wc1]) = 2 := by decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 9. still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ★本文件**实装**（对照 spec §13/§14 / line 657-709 环6，零遗漏）：
    (1) **A^raw_{t+1}=(A_t∖𝒟_x)∪ℬ_x**（`rawUpdate`/`mem_rawUpdate`，消费环5 `RThetaInterp.Triple` 三桶
        D=𝒟_x/B=ℬ_x，K 不进活动集）。
    (2) **A_{t+1}=AncOK(A^raw)**（`activeNext`/`activeFromState`，实例化多态闭合 `ancOKG` 于 Cand）+
        **复用兑现** `ancOKG_eq_ancOK`（多态闭合在 SyntaxElement 上 = `AncestorClosure.ancOK`，单一权威经
        等价抬升，非分叉）。
    (3) **祖先闭合保证**：`activeNext_anc_subset_raw`（⊆A^raw，无条件，对齐既有 anc_closed）+
        `parent_mem_activeNext`（直接父侧，§13 子腿存在则父在）+ `activeNext_anc_closed`
        （完整链 a∈A_{t+1}⟹Anc(a)⊆A_{t+1}，饱和下，§13 boxed）。
    (4) **平移不变 S_k AncOK=AncOK S_k**（`ancOKG_shift`，par 等变下；`activeNext_shift`）。
    (5) **p̃_{t+1}=Σ_{g∈A_{t+1}}Leg(g)**（`legOf`/`ptilde`/`ptildeActive`，复用 SeparateLedger.Leg）+
        **∀ ∃! p̃**（`ptilde_exists_unique`/`ptildeActive_exists_unique`，fold 确定性）+ `net_ptilde_append`
        （复用 net_append）。
    (6) **反退化见证**（§8 八条 decide）：AncOK 剪孤儿 / 整链保留 / 开启孤儿被剪 / 去关闭 / p̃ 净额。

  ★本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **`par`（候选祖先 p:C_ℓ→C_{ℓ+1}）的具体定义**：本文件取 `par : Cand → Option Cand` 为**可分离参数**
      （同 AncestorClosure 取 par 为参数）。spec §13 仅给「祖先只依赖相邻级别 p:C_ℓ→C_{ℓ+1}」性质，**未**
      逐字钉死「候选 c 的父容器是哪一个候选」。[需人工确认] par 的具体构造（区间套上一级的对应候选）属
      环1/环2 区间套塔的下游桥，本文件不臆造父函数内容（只取其为参数 + 等变性质）。
    · **`sOf`（腿单位数 s_g，§14 s_g>0）**：本文件取 `sOf : Cand → Nat` 为可分离参数（Cand 不携带 s_g）。
      [需人工确认] s_g 的具体来源（§15 LexArgmin 手数网格 / §三 LegAssign 的 s 字段）属环7 风控层 +
      SeparateStrategyTarget LegAssign，单一权威不在本文件——不臆造单位数。
    · **腿的声部坐标 ν(g)（§14 e^{δ}_{ν(g)} 的 ν）**：本文件 `ptilde` 以**活动集列表本身**为索引（每候选
      一条 `Leg` 入 `SepPosition`），**未**按 ν(g) 分组聚合同声部腿。[需人工确认] §14 展开式按 ν(g) 分组
      （同声部多候选腿相加）；本文件取「每候选一腿」的 List 承载（SepPosition=List Leg），∃! 不依赖分组
      （fold 确定即唯一）。按 ν(g) 真分组属 SeparateStrategyTarget LegAssign（nu 字段）下游，本文件不臆造
      声部映射。
    · **跨谱系类型桥 Cand ↔ SyntaxElement**：本文件经 `ancOKG_eq_ancOK` 证多态闭合在 SyntaxElement 上 =
      AncestorClosure（复用兑现），但**未**建 Cand→SyntaxElement 的具体嵌入（要造 λ/ρ/parent-index 假数据，
      属 patch，no-workaround 禁）。七链（Cand）与互斥分类（SyntaxElement）的语义级桥是**独立工位**——
      本文件多态化使两谱系的闭合**算子层**统一（经等价），语义层对齐不属本工位。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：活动集递归更新与目标头寸的 **L0 结构形式化**——两步活动集更新 (1)`rawUpdate`：
     A^raw=(A_t∖𝒟_x)∪ℬ_x（消费环5 `RThetaInterp.Triple` 三桶）；(2)`activeNext`=`ancOKG par fuel A^raw`
     （祖先闭合，多态闭合实例化于 Cand）。主性质：`activeNext_anc_closed`（a∈A_{t+1}⟹Anc(a)⊆A_{t+1}，
     饱和下）、`ancOKG_shift`（S_k AncOK=AncOK S_k，par 等变下）、`ptilde_exists_unique`（∀ ∃! p̃）。
     **复用兑现** `ancOKG_eq_ancOK`（= AncestorClosure.ancOK at SyntaxElement）。全 L0，零 sorry/admit/axiom。
  2. 定义依据：spec `2026-06-28-...-bsp-pdf-extract.md` §13（line 657-687：A^raw 方框/AncOK 方框/AncOK 定义/
     蕴含方框/p:C_ℓ→C_{ℓ+1}/平移方框）+ §14（line 691-703：p̃ 求和方框/展开式/∀ ∃! 方框）+ 结果包
     line 682-687、705-709。输入特征：环5 `Triple{D,B,K}:List Cand`（D=𝒟_x 应关闭、B=ℬ_x 应开启、K 记录
     不执行不进活动集）满足 §13「去关闭加开启」；候选祖先关系（par）满足 §13「祖先只依赖相邻级别」；
     方向 δ=dirOf bsp（§14）⟹ legLong/legShort 分流。
  3. 边界条件（结论翻转）：
     · **完整链闭合依赖 fuel 饱和**（核心）：`activeNext_anc_closed`（Anc(a)⊆A_{t+1}）需 `Saturated par fuel`
       （fuel≥树深，§十九）。若 fuel<树深，单次 AncOK 不达不动点，深祖先可能漏判 ⟹ 完整链闭合**翻转**为仅
       `⊆A^raw`（`activeNext_anc_subset_raw`，无条件成立）。AncestorClosure docstring 明示 fuel=树深上界即
       此正则区。
     · **平移不变依赖 par 等变**：`ancOKG_shift`（S_k AncOK=AncOK S_k）需 `ParEquivariant par k`
       （par(S_k g)=S_k par(g)，祖先关系级别平移自相似）。若 par 非等变（如父函数依赖绝对级别而非相对），
       平移不变**翻转**。spec p:C_ℓ→C_{ℓ+1} 自相似 ⟹ 等变成立。
     · **∃! p̃ 依赖上游唯一**：`ptilde_exists_unique` 唯一性依赖 A_{t+1} 唯一（§13，来自环5 ∃! 三桶 +
       确定性闭合）与 Leg 确定（sOf 函数）。任一上游非唯一 ⟹ p̃ 唯一性翻转。
     · **par/sOf/ν 为可分离参数（设计选择）**：本文件不钉死 par 内容、s_g 来源、ν 分组——取其为参数。
       具体构造换不同实例，活动集成员/p̃ 数值随之变，但结构定理（闭合/平移/∃!）不变（依赖参数性质，非具体值）。
  4. 下游推论：
     · `activeNext`/`activeFromState` 全定义 ⟹ 环7 全定义策略（§15 𝒦_Θ、J_x=‖p−p̃‖²_W）可消费 p̃ 作目标
       头寸；`ConstraintSystem.c2_ancestorClose`（约束化身）与本文件 `activeNext_anc_closed` 同源（父闭合约束）。
     · `ptilde`（SepPosition）⟹ 环7 风控层范数 ‖p−p̃‖²_W 的 p̃ 项就绪；`net_ptilde_append` ⟹ 与
       `SeparateLedger.Net` 净额视角一致（多空抵消 C26 在头寸聚合层成立）。
     · `ancOKG_eq_ancOK` ⟹ 七链（Cand）与互斥分类（SyntaxElement）的祖先闭合**算子层统一**（单一权威），
       下游 §16 七链穿线可统一引用一份闭合语义，避免两谱系闭合漂移。
     · `ancOKG_shift` ⟹ 升级3a 分类自相似 §18 的步7（AncOK 平移）就绪（缺口矩阵列为缺口）。
  5. 谱系引用：
     · 缺口矩阵 `2026-06-28-lean-existing-vs-20page-gap-matrix.md` 环6a「reconcile/复用 AncestorClosure，
       勿新建闭包」+ 环6b「在 SeparateLedger.Leg 上 fold→p̃」——本文件遵守：A_{t+1} 经 `ancOKG_eq_ancOK`
       复用 AncestorClosure（非分叉）；p̃ 复用 `SeparateLedger.Leg/legLong/legShort/Net/net_append`。
     · ★**纠正缺口矩阵两处不精确**（no-patch 诚实标注，建议 source-auditor/genealogist 核）：
       (a) 缺口矩阵称「`ancestorClose`=迭代不动点」——经核 `AncestorClosure.ancestorClose` 实为**单次** AncOK
           （`ancOK par fuel (targetActiveSet ...)`，非迭代）。完整链闭合靠 **fuel 饱和**（fuel≥树深）而非迭代。
       (b) 缺口矩阵预设 AncOK 可直接复用于环5 输出，但 `AncestorClosure.ancOK` **单态于 SyntaxElement**，环5
           三桶是 **List Cand**——**跨谱系类型不对齐**。本文件经多态化 + 等价证明消解（见顶部类型谱系接缝）。
     · 触及记忆 `newchanlun-b2s2-still-missing-tower`（嵌套对冲需真嵌套塔）：本文件 `activeNext_anc_closed`
       保证「子腿存在则父容器存在」= 嵌套结构完整（子必有父=真塔），与该谱系呼应（覆盖闭合性的 Lean 形式）。
     · 触及记忆 `theta-v0-trades-vs-closedloop-disjoint-paths` / recognize 硬编码 exit:false：本文件 𝒟_x（去关闭）
       是 rust exit 的来源——A^raw 的 `listDiff At t.D` 为该谱系提供「应关闭即移出活动集」的 Lean 语义
       （rust 侧重写属下游工位，不属本工位）。
     · 触及记忆 `newchanlun-v1-fullwindow-l3-falsified`：本文件 L0 闭合/∃! 与 v1 实盘盈利性（L2/L3）认识论等级
       不同、不可互相否证——不预判 alpha。
     · **不确定**是否有「七链环6 活动集递归 + p̃」的更早专属谱系——本工位不臆造，明确标注此不确定性。
  6. 影响声明：新建 `Origin/ActiveSet.lean`，import 三纯 core 兄弟文件（`Origin.RThetaInterp` 环5 +
     `Origin.AncestorClosure` 环6a + `Origin.SeparateLedger` 环6b），**无** Mathlib/Batteries/Std。
     **不改**任何既有 Lean/rust/定义/spec（不碰 AncestorClosure/RThetaInterp/SeparateLedger/ConstraintSystem
     ——后者是 lean-capital/lean-strategy 地盘）。namespace `NewChanlun.Origin.ActiveSet`（无命名冲突）。
     下游：为环7 全定义策略提供活动集 A_{t+1} 与目标头寸 p̃。不碰 lakefile（报 Lead 登记 root
     `Origin.ActiveSet`）。`lake env lean Origin/ActiveSet.lean` 单文件验证。
-/

end NewChanlun.Origin.ActiveSet
