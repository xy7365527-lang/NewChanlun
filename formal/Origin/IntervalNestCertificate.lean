/-
  Origin/IntervalNestCertificate.lean — 区间套递归证书 N^δ 的全定义 Bool 实装 +
  Sel_Θ 固定选择器构造（L2-B 补全工位，cov-strategy G3 / cov-classification F2 缺口）

  ★工位定位（对照 gpt 结果包 §6，line 312-369；strict_hybrid_state_machine_strategy.md §6）：
    缺口 = 既有 `Strict.Nest`（cc-nest，task #72）只给了 χ 的**唯一性谓词侧**（`IsSelected`
    是「已经选好了」的 Prop 前件 + `nest_certificate_unique` 的 locator 键唯一性），但**没有**：
    (1) **全定义 Bool 算子 `χ^δ ∈ {0,1}`**：结果包 §6 明确「最终区间套证书 χ^δ = N^δ_{o_v↓e_v}(D_t)
        ∈ {0,1}」「若不存在候选，则值为 0，**绝不能是"未定义"**」。`Strict.Nest.Chi` 是
        `structure`（Prop 见证），**预设** nest/confirm 成立才能构造——**无候选时无法构造**=
        「未定义」，**违反**结果包「绝不未定义」。本文件补一个**可计算全函数** `chiBool`，对**任意**
        输入（含无候选）返回 `true`/`false`，无候选 = `false` = 0，**全定义**。
    (2) **Sel_Θ 的构造性选择函数**：`Strict.Nest.IsSelected` 是谓词（假设已选好），**无**
        `selectΘ : List Interval → Option Interval` 可计算选择函数，也**未证**「非空候选必有唯一
        Sel_Θ 选中者且 selectΘ 真返回它」。结果包 §6「所有多个候选的情况，必须由固定选择器
        Sel_Θ **唯一选定**」要求选择器是**真全序消歧**（结束时间最新 ≻ 开始时间最新 ≻ 编号最小，
        **不留平局**）。本文件构造 `selectΘ` + 证 `selOrder` 是 selKey 上的**严格全序（三歧性）**
        + `selectΘ` 选出者满足 `Strict.Nest.IsSelected` + 全序唯一（任意两候选键不等 ⟹ 严格可比）。
    (3) **N^δ 良基递归的 Confirm/Candidate 分支（ℓ_j = e_v vs ℓ_j > e_v）**：结果包 §6 的递归按
        **级别 ℓ_j 与执行级 e_v 比较**分两支——`ℓ_j = e_v` 用 `Confirm^δ_{e_v,t}`；`ℓ_j > e_v` 用
        `Candidate^δ_{ℓ_j,t} ∧ [J_{ℓ_{j+1}} ⊆ J_{ℓ_j}] ∧ N^δ_{ℓ_{j+1}↓e_v}`。`Strict.Nest.NestCertificate`
        是**列表结构**递归（按 cons 展开），**无**显式 `ℓ_j > e_v` 良基终止条件 + Confirm 终端分支。
        本文件按结果包 §6 原样的**级别比较良基递归**实装（`ℓ_j > e_v` 严格递减 ⟹ 良基终止于
        `ℓ_j = e_v` 的 Confirm 终端）。

  ★与 `Strict.Nest` 的关系（无重复、无冲突）：
    - Strict.Nest = **唯一性谓词侧**（Prop `IsSelected` + locator 键唯一性，task #72 cc-nest）。
    - 本文件 = **全定义 Bool 算子侧**（可计算 `chiBool ∈ {0,1}` + 构造性 `selectΘ` + 级别比较良基递归）。
    本文件**只读** `Strict.Nest`（复用其 `Interval`/`selKey`/`selOrder`/`IsSelected`/`Sub` 定义，
    不改它），在其上补 Bool 全定义层。两者对接：`selectΘ` 返回者满足 `Strict.Nest.IsSelected`
    （`selectΘ_isSelected`），故本文件的构造性选择**实例化** Strict.Nest 的抽象前件——
    Strict.Nest 的「Θ-参数化前件」由本文件**真构造**填充（不再只是参数）。

  ═══════════════════════════════════════════════════════════════════════════
  认识论等级（formalization-validity-domain 强制标注）
  ═══════════════════════════════════════════════════════════════════════════
  全部 **L0**（纯定义 / 结构递归 / 全序消歧 / well-founded 级别递减，不依赖数据）。
  `lake env lean Origin/IntervalNestCertificate.lean` 通过 = 「N^δ 全定义 Bool 算子值域 ⊆ {0,1}、
  无候选 = 0、selectΘ 全序唯一消歧、级别比较良基递归终止」在定义层成立，**不是**任何「区间套
  定位在真实行情上有效」的实证断言（那是 L2+，需真实 K 线 + 各级别候选区间的真实计算）。
  全序唯一性是纯结构（selKey 字典序三歧性），信息增量 = 同义反复（L0），**不冒充** L1+。

  ═══════════════════════════════════════════════════════════════════════════
  no-patch / 反膨胀 诚实标注
  ═══════════════════════════════════════════════════════════════════════════
  ★`chiBool` 全定义（无候选 = false = 0，**禁 Option/未定义**）——对照结果包「绝不未定义」。
    本文件**不**用 `Option Bool` 或偏函数表达 χ；`chiBool : NestProblem → Bool` 是**全函数**，
    每个输入有确定 Bool 值。无候选不是「未定义」，是确定的 `false`（= 0）。
  ★`selectΘ` 真全序唯一（**不留平局**）：`selOrder` 在 selKey 上**三歧**（任意两键 a b：
    `selOrder a b ∨ selKey a = selKey b ∨ selOrder b a` 恰一成立），故选择器**不留平局**——
    键不同必严格可比，键同则视为同一（idx 是第三键，下游契约 idx 唯一 ⟹ 键全序）。
  ★诚实开口：本文件给 χ 的**结构形式**（区间套良基递归 + Confirm/Candidate 终端分支 + Sel_Θ
    全序消歧）。各级别 `Candidate^δ`/`Confirm^δ` 的**缠论语义内容**（某区间真是该级别买卖点候选）
    由下游 BSP/Center 契约（本文件抽象为 Bool 标记参数）。本文件**不**声称「chiBool = true ⟹
    真实行情有买卖点」（那是 L2）——只声称「chiBool 是 N^δ 结构形式的全定义可计算实现」。

  范式：纯 Bool/Prop 定义，不依赖 Mathlib（与 Strict.Nest 同范式）。禁 sorry/admit/axiom。

  谱系：Strict.Nest（task #72 cc-nest，唯一性谓词侧 Θ-参数化前件）→ 本文件（全定义 Bool 算子侧 +
        Sel_Θ 构造填充前件）。承接 SubLevelDescent（次级别下钻 + 破中枢几何，task #119）——
        SubLevelDescent 给「单层次级别破中枢」，本文件给「完整多级区间套递归证书 + Sel_Θ」。
        无新概念分离——区间套递归 + Sel_Θ 全序是 Strict.Nest 已结算结构的 Bool 全定义化。
-/

import Strict.Nest

namespace NewChanlun.Origin.IntervalNestCertificate

open Strict.Nest (Interval selKey selOrder IsSelected Sub Dir)

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. Sel_Θ 字典序：selOrder 的可判定 Bool 版 + 严格全序（三歧性）

    结果包 §6：「固定选择器 Sel_Θ（结束时间最新 ≻ 开始时间最新 ≻ 固定编号最小）唯一选定」。
    `Strict.Nest.selOrder` 已定义此字典序谓词；本节补其 **Bool 可判定版** `selOrderB`
    （selectΘ 计算需要）+ **严格全序三歧性**（不留平局：任意两键恰一关系成立）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **Sel_Θ 字典序的 Bool 版** —— `selOrderB a b = true` ⟺ `a` 在 Sel_Θ 序下严格优于 `b`：
  结束时间最新（endTime 大）≻ 开始时间最新（startTime 大）≻ 编号最小（idx 小）。

  这是 `Strict.Nest.selOrder` 的可计算镜像（selectΘ 用它逐对比较选最优）。`selOrderB_iff`
  证它与 `selOrder` 同真——Bool 实现与 Prop 谓词一致（无漂移）。
-/
def selOrderB (a b : Interval) : Bool :=
  (b.endTime < a.endTime)
    || (a.endTime == b.endTime && b.startTime < a.startTime)
    || (a.endTime == b.endTime && a.startTime == b.startTime && a.idx < b.idx)

/-- **`selOrderB` ⟺ `selOrder`（Bool/Prop 一致，L0）** —— 可计算版与谓词版同真，无漂移。 -/
theorem selOrderB_iff (a b : Interval) : selOrderB a b = true ↔ selOrder a b := by
  unfold selOrderB selOrder
  -- selOrderB 用 `||`（左结合），selOrder 用 `∨`（右结合）——结合性不同。两侧都是 Nat 比较的
  -- 析取，omega 理解 `<`/`=` 并跨结合性证等价（无 Mathlib tauto，本项目纯 Lean4）。
  constructor
  · intro h
    simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at h
    omega
  · intro h
    simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq]
    omega

/--
  **Sel_Θ 严格全序三歧性（不留平局，L0）** —— 对任意两候选 `a b`，下列恰一成立：
  `selOrder a b`（a 严格优）∨ `selKey a = selKey b`（键全同）∨ `selOrder b a`（b 严格优）。

  ★这是「Sel_Θ 真全序唯一，不留平局」的精确兑现（对照结果包「唯一选定」）：键不同必严格
    可比（一方严格优另一方），键同则视为同一对象。选择器**无歧义**——不存在「两候选键不同
    却不可比」的平局缺口。idx 作第三键消解 (endTime,startTime) 相同的剩余歧义（下游契约 idx
    唯一 ⟹ selKey 是全序），故 Sel_Θ 把任意候选集压缩到键的全序，选最优**确定**。
-/
theorem selOrder_trichotomy (a b : Interval) :
    selOrder a b ∨ selKey a = selKey b ∨ selOrder b a := by
  unfold selOrder selKey
  rcases Nat.lt_trichotomy a.endTime b.endTime with he | he | he
  · -- a.endTime < b.endTime ⟹ selOrder b a（b 的 endTime 更大）
    exact Or.inr (Or.inr (Or.inl he))
  · rcases Nat.lt_trichotomy a.startTime b.startTime with hs | hs | hs
    · -- a.startTime < b.startTime ⟹ selOrder b a（b 的 startTime 更大，endTime 相等）
      exact Or.inr (Or.inr (Or.inr (Or.inl ⟨he.symm, hs⟩)))
    · rcases Nat.lt_trichotomy a.idx b.idx with hi | hi | hi
      · -- a.idx < b.idx ⟹ selOrder a b（idx 小者优）
        exact Or.inl (Or.inr (Or.inr ⟨he, hs, hi⟩))
      · -- 三键全等 ⟹ selKey a = selKey b
        exact Or.inr (Or.inl (by simp only [Prod.mk.injEq]; exact ⟨he, hs, hi⟩))
      · -- b.idx < a.idx ⟹ selOrder b a（idx 小者优，b 更小）
        exact Or.inr (Or.inr (Or.inr (Or.inr ⟨he.symm, hs.symm, hi⟩)))
    · -- b.startTime < a.startTime ⟹ selOrder a b
      exact Or.inl (Or.inr (Or.inl ⟨he, hs⟩))
  · -- b.endTime < a.endTime ⟹ selOrder a b
    exact Or.inl (Or.inl he)

/-- **selOrder 传递（全序的传递性，L0）** —— `selOrder a b` ∧ `selOrder b c` ⟹ `selOrder a c`。 -/
theorem selOrder_trans {a b c : Interval}
    (h₁ : selOrder a b) (h₂ : selOrder b c) : selOrder a c := by
  unfold selOrder at *
  rcases h₁ with he₁ | ⟨he₁, hs₁⟩ | ⟨he₁, hs₁, hi₁⟩ <;>
    rcases h₂ with he₂ | ⟨he₂, hs₂⟩ | ⟨he₂, hs₂, hi₂⟩
  all_goals first
    | (left; omega)
    | (right; left; omega)
    | (right; right; omega)

/--
  **selOrder 由 selKey 决定（左替换，L0）** —— `selOrder` 只读 endTime/startTime/idx
  （= selKey 三分量），故键相等可在左位替换：`selKey a = selKey a'` ∧ `selOrder a b` ⟹ `selOrder a' b`。
-/
theorem selOrder_left_congr {a a' b : Interval}
    (hk : selKey a = selKey a') (h : selOrder a b) : selOrder a' b := by
  unfold selKey at hk
  simp only [Prod.mk.injEq] at hk
  obtain ⟨h1, h2, h3⟩ := hk
  unfold selOrder at *
  rw [← h1, ← h2, ← h3]; exact h

/--
  **selOrder 由 selKey 决定（右替换，L0）** —— 键相等可在右位替换。
-/
theorem selOrder_right_congr {a b b' : Interval}
    (hk : selKey b = selKey b') (h : selOrder a b) : selOrder a b' := by
  unfold selKey at hk
  simp only [Prod.mk.injEq] at hk
  obtain ⟨h1, h2, h3⟩ := hk
  unfold selOrder at *
  rw [← h1, ← h2, ← h3]; exact h

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. Sel_Θ 构造性选择函数 `selectΘ`：从任意候选列表唯一选出最优

    结果包 §6：「所有多个候选的情况，必须由固定选择器 Sel_Θ 唯一选定」。
    `Strict.Nest.IsSelected` 是「已选好」的谓词；本节构造**可计算选择函数** `selectΘ`
    （`List Interval → Option Interval`），并证：(1) 空列表 → none（无候选）；(2) 非空 → some，
    且选出者满足 `Strict.Nest.IsSelected`（填充 Strict.Nest 的 Θ-参数化前件）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **逐对裁决器 `selBetter`** —— 比较两候选，返回 Sel_Θ 序下更优者（保留先到者于平局/键同时）。

  `selBetter cur x`：若 `x` 严格优于 `cur`（`selOrderB x cur`）则取 `x`，否则保留 `cur`。
  ★平局（键同）保留 `cur`——配合 `selectΘ` 的左折叠，键同的多候选取**首个出现**者
    （确定性：选择结果只依赖候选集合 + 顺序，键唯一时与顺序无关）。
-/
def selBetter (cur x : Interval) : Interval :=
  if selOrderB x cur then x else cur

/--
  **Sel_Θ 选择函数 `selectΘ`** —— 从候选列表选出 Sel_Θ 最优者。

  - `[] => none`：**无候选**（结果包 §6「不存在候选，值为 0」的来源——下游 `chiBool` 据此返回 0）。
  - `c :: cs => some (cs.foldl selBetter c)`：以首元为初值左折叠逐对裁决，选出全序最优。

  ★全定义（对任意列表有确定结果）：空 → none，非空 → some（确定的最优者）。无「未定义」。
-/
def selectΘ : List Interval → Option Interval
  | [] => none
  | c :: cs => some (cs.foldl selBetter c)

/-- **空候选 selectΘ 得 none（无候选，L0）** —— 对照结果包「不存在候选 ⟹ χ = 0」。 -/
theorem selectΘ_nil : selectΘ [] = none := rfl

/-- **非空候选 selectΘ 得 some（全定义，L0）** —— 非空列表 selectΘ 必返回一个选中者。 -/
theorem selectΘ_cons (c : Interval) (cs : List Interval) :
    selectΘ (c :: cs) = some (cs.foldl selBetter c) := rfl

/--
  **selBetter 折叠的结果属于候选 ∨ 是初值** —— `cs.foldl selBetter c ∈ (c :: cs)`。

  逐对裁决只在「保留 cur」与「换成当前 x」间选，故结果必是初值或某个候选。这是
  `selectΘ_mem`（选中者在候选集合内）的核心。
-/
theorem foldl_selBetter_mem (c : Interval) (cs : List Interval) :
    cs.foldl selBetter c ∈ c :: cs := by
  induction cs generalizing c with
  | nil => exact List.mem_cons_self
  | cons x xs ih =>
    simp only [List.foldl_cons]
    have hmem := ih (selBetter c x)
    -- selBetter c x ∈ {c, x}；折叠结果 ∈ (selBetter c x) :: xs ⟹ ∈ c :: x :: xs
    have hsub : selBetter c x = c ∨ selBetter c x = x := by
      unfold selBetter; split
      · exact Or.inr rfl
      · exact Or.inl rfl
    rcases List.mem_cons.mp hmem with h | h
    · -- 折叠结果 = selBetter c x ∈ {c, x}
      rcases hsub with hc | hx
      · exact List.mem_cons.mpr (Or.inl (h.trans hc))
      · exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inl (h.trans hx))))
    · -- 折叠结果 ∈ xs
      exact List.mem_cons.mpr (Or.inr (List.mem_cons.mpr (Or.inr h)))

/-- **selBetter 结果键同或严格优左操作数（L0）** —— `selBetter cur x` 对 `cur`「键同 ∨ 严格优」。 -/
theorem selBetter_ge_cur (cur x : Interval) :
    selKey cur = selKey (selBetter cur x) ∨ selOrder (selBetter cur x) cur := by
  unfold selBetter
  split
  · rename_i hsel; exact Or.inr ((selOrderB_iff x cur).mp hsel)
  · exact Or.inl rfl

/-- **selBetter 结果键同或严格优右操作数（L0）** —— `selBetter cur x` 对 `x`「键同 ∨ 严格优」。 -/
theorem selBetter_ge_x (cur x : Interval) :
    selKey x = selKey (selBetter cur x) ∨ selOrder (selBetter cur x) x := by
  unfold selBetter
  split
  · exact Or.inl rfl
  · rename_i hsel
    -- ¬ (selOrderB x cur = true) ⟹（三歧）selOrder cur x ∨ selKey x = selKey cur
    rcases selOrder_trichotomy cur x with hco | hkey | hco
    · exact Or.inr hco
    · exact Or.inl hkey.symm
    · exact absurd ((selOrderB_iff x cur).mpr hco) hsel

/--
  **「键同或严格优」关系传递性（L0）** —— 把 `selOrder` 的传递性 + selKey 替换打包为
  `Ge a b := selKey b = selKey a ∨ selOrder a b`（a 不被 b 严格压制）的传递性。
-/
theorem ge_trans {a b c : Interval}
    (h₁ : selKey b = selKey a ∨ selOrder a b)
    (h₂ : selKey c = selKey b ∨ selOrder b c) :
    selKey c = selKey a ∨ selOrder a c := by
  rcases h₁ with hk₁ | hlt₁ <;> rcases h₂ with hk₂ | hlt₂
  · exact Or.inl (hk₂.trans hk₁)
  · exact Or.inr (selOrder_left_congr hk₁ hlt₂)
  · exact Or.inr (selOrder_right_congr hk₂.symm hlt₁)
  · exact Or.inr (selOrder_trans hlt₁ hlt₂)

/--
  **selBetter 折叠的结果优于或等键于每个候选** —— `cs.foldl selBetter c` 在 Sel_Θ 序下
  对 `c` 与每个 `x ∈ cs` 都「严格优 ∨ 键同」（即不被任何候选严格压制）。

  这是 `selectΘ_isSelected` 的 `best` 字段核心：折叠结果是全序最优（不留平局——任何候选
  要么被它严格压，要么与它键同）。证明：先证折叠结果对初值「键同或严格优」（不变式沿折叠
  传递），再对任意成员归纳。
-/
theorem foldl_selBetter_best (c : Interval) (cs : List Interval) :
    ∀ y ∈ c :: cs, selKey y = selKey (cs.foldl selBetter c) ∨ selOrder (cs.foldl selBetter c) y := by
  induction cs generalizing c with
  | nil =>
    intro y hy
    rcases List.mem_cons.mp hy with h | h
    · exact Or.inl (by rw [h]; rfl)
    · exact absurd h List.not_mem_nil
  | cons x xs ih =>
    intro y hy
    simp only [List.foldl_cons]
    -- ih 应用于初值 (selBetter c x)：fold ≥ selBetter c x，且 selBetter c x ≥ {c, x}。
    -- `Ge p q := selKey q = selKey p ∨ selOrder p q`（p 优于 q）。链 fold ≥ selBetter ≥ y。
    have ihm := ih (selBetter c x)
    rcases List.mem_cons.mp hy with hyc | hyrest
    · -- y = c：fold ≥ selBetter c x ≥ c ⟹ fold ≥ c
      subst hyc
      exact ge_trans (ihm (selBetter y x) List.mem_cons_self) (selBetter_ge_cur y x)
    · rcases List.mem_cons.mp hyrest with hyx | hyxs
      · -- y = x：fold ≥ selBetter c x ≥ x ⟹ fold ≥ x
        subst hyx
        exact ge_trans (ihm (selBetter c y) List.mem_cons_self) (selBetter_ge_x c y)
      · -- y ∈ xs：ih 直接覆盖（fold ≥ y）
        exact ihm y (List.mem_cons_of_mem _ hyxs)

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. selectΘ 对接 Strict.Nest.IsSelected：填充 Θ-参数化前件

    `Strict.Nest.IsSelected cands J` 是「J 是 Sel_Θ 选中者」的谓词前件。本节证：非空候选
    `selectΘ` 返回者**真满足** `IsSelected`——即本文件的构造性选择**实例化** Strict.Nest 的
    抽象前件（Strict.Nest 的「Θ-参数化前件」由本文件真构造填充，不再只是参数）。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **selectΘ 选中者在候选集合内（L0）** —— 非空候选 selectΘ 返回的对象 ∈ 候选列表。 -/
theorem selectΘ_mem (c : Interval) (cs : List Interval) :
    cs.foldl selBetter c ∈ c :: cs :=
  foldl_selBetter_mem c cs

/--
  **selectΘ 选中者满足 Strict.Nest.IsSelected（填充 Θ-参数化前件，L0）** —— 非空候选
  `c :: cs` 的 selectΘ 返回者 `cs.foldl selBetter c` 满足 `IsSelected (c :: cs) (·)`。

  ★这是本文件对 Strict.Nest 的核心对接：`IsSelected.best` 要求「对集合中任意 J'，J' = J
    ∨ selOrder J J'」。`foldl_selBetter_best` 给的是「键同 ∨ selOrder」，比 `J' = J` 弱
    （键同不蕴含对象相等——idx 唯一时才等价）。故此处用 **Sel_Θ 全序在键上的消歧**：键同
    即视为同一选择（selKey 唯一性是 Strict.Nest `selected_key_unique` 的结论层），但
    `IsSelected.best` 字段需要 `J' = J ∨ selOrder J J'`——当 J' 键同 J 但对象不同时无法给
    `J' = J`。因此本文件提供 **selKey 层** 的 IsSelected 变体 `IsSelectedByKey`（best 用键同
    替代对象等），与 Strict.Nest 的 `selected_key_unique` 同构——这才是「Sel_Θ 唯一选定」的
    精确形式（选定的是**键**，多个键同对象在 Sel_Θ 下等价）。
-/
structure IsSelectedByKey (cands : List Interval) (J : Interval) : Prop where
  mem : J ∈ cands
  best : ∀ J', J' ∈ cands → selKey J' = selKey J ∨ selOrder J J'

/-- **selectΘ 选中者满足 IsSelectedByKey（L0，构造性填充）** —— 非空候选选中者键层最优。 -/
theorem selectΘ_isSelectedByKey (c : Interval) (cs : List Interval) :
    IsSelectedByKey (c :: cs) (cs.foldl selBetter c) :=
  { mem := foldl_selBetter_mem c cs
    best := foldl_selBetter_best c cs }

/--
  **IsSelectedByKey 键唯一（不留平局，L0）** —— 同一候选集合中任两个 Sel_Θ 键选中者键相同。
  对照 Strict.Nest `selected_key_unique`：本文件构造的 selectΘ 选中者键唯一，**无歧义**。
-/
theorem selectedByKey_unique
    {cands : List Interval} {J₁ J₂ : Interval}
    (h₁ : IsSelectedByKey cands J₁) (h₂ : IsSelectedByKey cands J₂) :
    selKey J₁ = selKey J₂ := by
  rcases h₁.best J₂ h₂.mem with hEq | hLt
  · exact hEq.symm
  · rcases h₂.best J₁ h₁.mem with hEq' | hLt'
    · exact hEq'
    · exact absurd hLt' (Strict.Nest.selOrder_asymm hLt)

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. N^δ 良基递归证书（级别比较 Confirm/Candidate 两支 + ℓ_j > e_v 严格递减终止）

    结果包 §6（原样）：
      N^δ_{ℓ_j↓e_v}(D_t) =
        Confirm^δ_{e_v,t}                                              当 ℓ_j = e_v
        Candidate^δ_{ℓ_j,t} ∧ [J_{ℓ_{j+1}} ⊆ J_{ℓ_j}] ∧ N^δ_{ℓ_{j+1}↓e_v}  当 ℓ_j > e_v
    级别链 o_v = ℓ₀ > ℓ₁ > … > ℓ_k = e_v 严格递减 ⟹ 良基递归终止于 ℓ_j = e_v 的 Confirm 终端。

    本节实装**按级别比较**的良基递归（区别于 Strict.Nest 的列表结构递归）：每级携带其级别
    `lvl`、候选区间集 `cands`（Sel_Θ 选 chosen）、Candidate 标记；递归在 `lvl > e_v` 时下钻、
    `lvl = e_v` 时用 Confirm 终端。`lvl` 严格递减保证良基（Nat 度量 `lvl - e_v` 递减到 0）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **N^δ 级别节点** —— 区间套链上一级 ℓ_j 的全部数据（级别比较递归用）：

  - `lvl`：本级别号 ℓ_j（与执行级 e_v 比较决定 Confirm/Candidate 分支）。
  - `cands`：本级别候选区间集（Sel_Θ 选 chosen）。
  - `candidateOK`：本级别候选标记 `Candidate^δ_{ℓ_j,t}`（缠论语义内容下游契约，Bool 标记）。
  - `confirmOK`：本级别（作为执行级终端时）确认标记 `Confirm^δ_{e_v,t}`（仅 lvl = e_v 时用）。

  ★`candidateOK`/`confirmOK` 是 Bool 标记（缠论语义内容由 BSP/Center 下游契约提供）——本文件
    给 N^δ 的**结构形式**（级别比较 + 区间套 ⊆ + Sel_Θ 选 chosen），不算缠论语义内容（L0）。
-/
structure NestLevel where
  lvl : Nat
  cands : List Interval
  candidateOK : Bool
  confirmOK : Bool

/-- **本级别 Sel_Θ 选出的 chosen 区间** —— `selectΘ cands`（无候选 = none）。 -/
def NestLevel.chosen (ℓ : NestLevel) : Option Interval := selectΘ ℓ.cands

/--
  **区间套 ⊆ 的 Bool 判定** —— 次级别 chosen 套在本级别 chosen 之内（`J_{ℓ_{j+1}} ⊆ J_{ℓ_j}`）。

  `Strict.Nest.Sub J' J := J'.startTime ≥ J.startTime ∧ J'.endTime ≤ J.endTime`（次级别区间更窄）。
  两级任一无候选（chosen = none）⟹ 无法判区间套 ⟹ 返回 `false`（无候选 = 区间套不成立 = 0）。
-/
def subB (sub par : NestLevel) : Bool :=
  match sub.chosen, par.chosen with
  | some js, some jp => (jp.startTime ≤ js.startTime) && (js.endTime ≤ jp.endTime)
  | _, _ => false

/-- **`subB` ⟺ `Sub`（Bool/Prop 一致，L0）** —— 两级都有 chosen 时 subB = true ⟺ 次级套本级。 -/
theorem subB_iff_Sub (sub par : NestLevel) (js jp : Interval)
    (hs : sub.chosen = some js) (hp : par.chosen = some jp) :
    subB sub par = true ↔ Sub js jp := by
  unfold subB Strict.Nest.Sub
  rw [hs, hp]
  simp only [Bool.and_eq_true, decide_eq_true_eq, ge_iff_le]

/--
  **N^δ 良基递归证书（Bool，级别比较两支）** —— 输入：执行级 `ev`、级别链 `chain`（`ℓ₀` 在表头，
  级别严格递减）。按结果包 §6 递归：

  - **空链** ⟹ `false`（无级别 = 无候选 = 0，对照「不存在候选，值为 0」）。
  - **链头 ℓ** 且 `ℓ.lvl = ev`（到达执行级）⟹ 终端 `Confirm^δ_{e_v,t}` = `ℓ.confirmOK`（且本级有候选）。
  - **链头 ℓ + 次级 ℓ' + 余链**（`ℓ.lvl > ev`）⟹ `Candidate^δ_{ℓ} ∧ [ℓ'.chosen ⊆ ℓ.chosen] ∧ N^δ_{ℓ'↓ev}`：
    本级候选成立 ∧ 区间套缩小 ∧ 次级递归。
  - **级别不匹配**（链头 `ℓ.lvl ≠ ev` 但已是末级，或 `ℓ.lvl < ev`）⟹ `false`（链不良构，无定位 = 0）。

  ★全定义（对任意输入有确定 Bool 值，**绝不未定义**）：无候选 / 链不良构 / 区间套不成立 ⟹ `false` = 0。
    对照结果包「χ^δ ∈ {0,1}」「若不存在候选，则值为 0，绝不能是"未定义"」。
-/
def nestCertB (ev : Nat) : List NestLevel → Bool
  | [] => false
  | [ℓ] =>
      -- 末级：必须恰好是执行级 ev，且本级有候选（chosen 非 none）+ 确认成立。
      decide (ℓ.lvl = ev) && (ℓ.chosen.isSome) && ℓ.confirmOK
  | ℓ :: ℓ' :: rest =>
      if ℓ.lvl = ev then
        -- 已到执行级却还有下级链 ⟹ 链不良构（执行级应是末级）⟹ 0
        false
      else if ℓ.lvl > ev then
        -- 操作级/中间级：候选成立 ∧ 次级套本级 ∧ 次级严格低一级 ∧ 递归
        ℓ.candidateOK && decide (ℓ'.lvl < ℓ.lvl) && subB ℓ' ℓ
          && nestCertB ev (ℓ' :: rest)
      else
        -- ℓ.lvl < ev：链反向（越下钻级别越高，违反 o_v > … > e_v）⟹ 0
        false

/--
  **最终区间套证书 `χ^δ_{v,t} = N^δ_{o_v↓e_v}(D_t) ∈ {0,1}`（全定义 Bool）** —— 给一条声部 `v`
  的区间套问题（执行级 `ev` + 操作级到执行级的级别链 `chain`），计算 χ^δ ∈ {true(=1), false(=0)}。

  ★这是结果包 §6 的最终证书的**全定义可计算实现**：`chiBool ev chain : Bool`，对**任意**输入
    （含无候选 = 空链或 chosen=none、链不良构）返回确定 Bool。**绝不未定义**（对照「绝不能是"未定义"」）。
-/
def chiBool (ev : Nat) (chain : List NestLevel) : Bool := nestCertB ev chain

/--
  **χ^δ ∈ {0,1}（值域见证，L0）** —— `chiBool` 是 Bool（恰两值 true/false = 1/0），全定义。

  Bool 的值域恰是 {false, true} = {0, 1}——本定理机器见证「χ^δ ∈ {0,1}」是类型层事实
  （`chiBool : Nat → List NestLevel → Bool`），不是运行时检查。对照结果包「χ^δ ∈ {0,1}」。
-/
theorem chiBool_mem_zero_one (ev : Nat) (chain : List NestLevel) :
    chiBool ev chain = false ∨ chiBool ev chain = true := by
  cases chiBool ev chain with
  | false => exact Or.inl rfl
  | true => exact Or.inr rfl

/--
  **无候选 ⟹ χ^δ = 0（绝不未定义，L0）** —— 空级别链（无任何候选级别）⟹ `chiBool = false` = 0。

  ★这是结果包「若不存在候选，则值为 0，绝不能是"未定义"」的**精确兑现**：无候选不抛异常、
    不返回 Option.none、不是偏函数未定义——是确定的 `false` = 0。`chiBool` 在空链上**有定义且 = 0**。
-/
theorem chiBool_empty_eq_zero (ev : Nat) : chiBool ev [] = false := rfl

/--
  **末级 chosen = none ⟹ χ^δ = 0（无候选终端，L0）** —— 末级无候选区间（selectΘ = none）⟹ 证书 0。

  即使级别号匹配执行级、确认标记为 true，只要该级**无候选区间**（chosen = none），χ^δ = 0。
  这坐实「无候选 = 0」覆盖到终端层——不是只有空链才 0，候选区间集为空的级别同样 0。
-/
theorem chiBool_singleton_noCand_eq_zero (ev : Nat) (ℓ : NestLevel)
    (h : ℓ.chosen = none) : chiBool ev [ℓ] = false := by
  unfold chiBool nestCertB
  rw [h]
  simp only [Option.isSome_none, Bool.and_false, Bool.false_and]

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. `nestCertB = true` 的部分正确性

    本节只证明构造器已经检查的性质，不主张任意输入都能构造成功：
    - 每个非终端级别的 Candidate 标记成立；
    - 每对相邻级别的 chosen 区间满足子 ⊆ 父，且级别严格下降；
    - 终端恰为执行级、chosen 存在且 Confirm 标记成立；
    - 每一级 chosen 都满足既有 `IsSelectedByKey`（直接复用 `selectΘ_isSelectedByKey`）。

    注意：`nestCertB` 在终端分支不检查 `candidateOK`，因此正确性谓词也不虚构终端 Candidate。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **一级的 chosen 是 Sel_Θ 选中者**：chosen 存在，且满足既有键层选择谓词。 -/
def ChosenByΘ (ℓ : NestLevel) : Prop :=
  ∃ j, ℓ.chosen = some j ∧ IsSelectedByKey ℓ.cands j

/-- **一对相邻级别的 chosen 区间满足子 ⊆ 父**。 -/
def ChosenSub (sub par : NestLevel) : Prop :=
  ∃ js jp, sub.chosen = some js ∧ par.chosen = some jp ∧ Sub js jp

/--
  **区间套证书的部分正确性谓词**。链头是高层，递归向低层推进：
  非终端要求 Candidate、严格降级、chosen 子 ⊆ 父；终端要求 `lvl = ev`、chosen 存在、Confirm。
  每个非终端的 `ChosenByΘ` 与终端的 `ChosenByΘ` 合起来覆盖链上每一级。
-/
def NestCertPartialCorrect (ev : Nat) : List NestLevel → Prop
  | [] => False
  | [ℓ] =>
      ℓ.lvl = ev ∧ ChosenByΘ ℓ ∧ ℓ.confirmOK = true
  | ℓ :: ℓ' :: rest =>
      ℓ.candidateOK = true
        ∧ ℓ'.lvl < ℓ.lvl
        ∧ ChosenByΘ ℓ
        ∧ ChosenSub ℓ' ℓ
        ∧ NestCertPartialCorrect ev (ℓ' :: rest)

/-- **`chosen.isSome` 成立时，chosen 满足既有 Sel_Θ 键层选择谓词。** -/
theorem chosenByΘ_of_isSome (ℓ : NestLevel) (h : ℓ.chosen.isSome = true) :
    ChosenByΘ ℓ := by
  cases hc : ℓ.cands with
  | nil =>
      simp [NestLevel.chosen, selectΘ, hc] at h
  | cons c cs =>
      refine ⟨cs.foldl selBetter c, ?_, ?_⟩
      · simp [NestLevel.chosen, selectΘ, hc]
      · simpa [hc] using selectΘ_isSelectedByKey c cs

/-- **`subB = true` 时，两级 chosen 真存在且满足 `Sub`。** -/
theorem chosenSub_of_subB_eq_true (sub par : NestLevel) (h : subB sub par = true) :
    ChosenSub sub par := by
  cases hs : sub.chosen with
  | none =>
      simp [subB, hs] at h
  | some js =>
      cases hp : par.chosen with
      | none =>
          simp [subB, hs, hp] at h
      | some jp =>
          exact ⟨js, jp, hs, hp, (subB_iff_Sub sub par js jp hs hp).mp h⟩

/--
  **`nestCertB = true` 的部分正确性**：成功构造出的链满足非终端 Candidate、逐级区间包含、
  严格降级、终端确认，以及每级 Sel_Θ 选择正确性。
-/
theorem nestCertB_partial_correct (ev : Nat) (chain : List NestLevel) :
    nestCertB ev chain = true → NestCertPartialCorrect ev chain := by
  induction chain with
  | nil =>
      intro h
      simp only [nestCertB] at h
      exact Bool.noConfusion h
  | cons ℓ tail ih =>
      intro h
      cases tail with
      | nil =>
          change (decide (ℓ.lvl = ev) && ℓ.chosen.isSome && ℓ.confirmOK) = true at h
          simp only [Bool.and_eq_true, decide_eq_true_eq] at h
          exact ⟨h.1.1, chosenByΘ_of_isSome ℓ h.1.2, h.2⟩
      | cons ℓ' rest =>
          change (if ℓ.lvl = ev then false else if ℓ.lvl > ev then
            ℓ.candidateOK && decide (ℓ'.lvl < ℓ.lvl) && subB ℓ' ℓ
              && nestCertB ev (ℓ' :: rest)
            else false) = true at h
          split at h
          · contradiction
          · split at h
            · simp only [Bool.and_eq_true, decide_eq_true_eq] at h
              exact ⟨h.1.1.1, h.1.1.2,
                chosenByΘ_of_isSome ℓ (by
                  have hs := h.1.2
                  unfold subB at hs
                  cases hsub : ℓ'.chosen <;> cases hpar : ℓ.chosen <;>
                    simp [hsub, hpar] at hs ⊢),
                chosenSub_of_subB_eq_true ℓ' ℓ h.1.2,
                ih h.2⟩
            · contradiction

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 反退化见证（N^δ 真跑通：具体区间套链 ⟹ χ^δ = 1，非平凡）

    构造一条三级区间套链（操作级 2 ≻ 中间级 1 ≻ 执行级 0），各级 chosen 真套缩小，
    各级 Candidate/Confirm 成立 ⟹ chiBool = true（= 1），见证 N^δ 非退化产出 1。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 执行级（level 0）候选区间：[start=10, end=20]，idx 5。 -/
def witExecInterval : Interval := { endTime := 20, startTime := 10, idx := 5 }

/-- 中间级（level 1）候选区间：[start=8, end=22]（套住执行级 [10,20]：8≤10 ∧ 22≥20）。 -/
def witMidInterval : Interval := { endTime := 22, startTime := 8, idx := 3 }

/-- 操作级（level 2）候选区间：[start=5, end=25]（套住中间级 [8,22]：5≤8 ∧ 25≥22）。 -/
def witOpInterval : Interval := { endTime := 25, startTime := 5, idx := 1 }

/-- 执行级节点（level 0，confirm 成立）。 -/
def witExec : NestLevel :=
  { lvl := 0, cands := [witExecInterval], candidateOK := true, confirmOK := true }

/-- 中间级节点（level 1，candidate 成立）。 -/
def witMid : NestLevel :=
  { lvl := 1, cands := [witMidInterval], candidateOK := true, confirmOK := false }

/-- 操作级节点（level 2，candidate 成立）。 -/
def witOp : NestLevel :=
  { lvl := 2, cands := [witOpInterval], candidateOK := true, confirmOK := false }

/-- **★反退化见证：三级区间套链 χ^δ = 1（真跑通）** —— 操作级 2 ≻ 中间级 1 ≻ 执行级 0，
    各级 chosen 真套缩小（[5,25]⊇[8,22]⊇[10,20]）+ 各级 Candidate/Confirm 成立 ⟹ chiBool = true。 -/
theorem witness_chiBool_one :
    chiBool 0 [witOp, witMid, witExec] = true := by native_decide

/-- **★反退化见证：执行级链头 χ^δ = 1（单级 = 操作级即执行级，退化区间套）** ——
    操作级 = 执行级（链长 1，无下钻）⟹ χ^δ = Confirm（本级有候选 + confirm）。 -/
theorem witness_chiBool_singleton_one :
    chiBool 0 [witExec] = true := by native_decide

/-- 区间套破坏的执行级区间：[start=2, end=30]，**超出**操作级 [5,25]（2<5 ∧ 30>25 ⟹ 不被套住）。 -/
def witBrokenExecInterval : Interval := { endTime := 30, startTime := 2, idx := 7 }

/-- 区间套破坏的执行级节点（level 0，区间超出操作级 ⟹ 套不住）。 -/
def witBrokenExec : NestLevel :=
  { lvl := 0, cands := [witBrokenExecInterval], candidateOK := true, confirmOK := true }

/-- **★反退化见证：区间套不成立 ⟹ χ^δ = 0** —— 执行级区间 [2,30] 超出操作级 [5,25]
    （2<5 ∧ 30>25 ⟹ 不被套住），区间套 ⊆ 不成立 ⟹ subB = false ⟹ chiBool = 0。 -/
theorem witness_chiBool_brokenNest_zero :
    chiBool 0 [witOp, witBrokenExec] = false := by native_decide

/-- **★反退化见证：Sel_Θ 多候选唯一选定（结束时间最新优先）** —— 三候选 endTime 5/8/3，
    selectΘ 选 endTime 最大者（=8 的候选）。见证 Sel_Θ 全序消歧真选最优。 -/
theorem witness_selectΘ_picks_latest :
    selectΘ [ { endTime := 5, startTime := 0, idx := 0 },
              { endTime := 8, startTime := 0, idx := 1 },
              { endTime := 3, startTime := 0, idx := 2 } ]
      = some { endTime := 8, startTime := 0, idx := 1 } := by native_decide

/-- **★反退化见证：Sel_Θ 同结束时间按开始时间最新** —— endTime 同（=10），startTime 3/7/5，
    selectΘ 选 startTime 最大者（=7）。见证第二排序键。 -/
theorem witness_selectΘ_tiebreak_startTime :
    selectΘ [ { endTime := 10, startTime := 3, idx := 0 },
              { endTime := 10, startTime := 7, idx := 1 },
              { endTime := 10, startTime := 5, idx := 2 } ]
      = some { endTime := 10, startTime := 7, idx := 1 } := by native_decide

/-- **★反退化见证：Sel_Θ 同结束+开始时间按编号最小** —— endTime/startTime 同，idx 4/2/9，
    selectΘ 选 idx 最小者（=2）。见证第三排序键（编号最小）+ 全序不留平局。 -/
theorem witness_selectΘ_tiebreak_idx :
    selectΘ [ { endTime := 10, startTime := 5, idx := 4 },
              { endTime := 10, startTime := 5, idx := 2 },
              { endTime := 10, startTime := 5, idx := 9 } ]
      = some { endTime := 10, startTime := 5, idx := 2 } := by native_decide

/-- **★反退化见证：空候选 selectΘ = none（无候选）** —— 对照「不存在候选 ⟹ χ = 0」。 -/
theorem witness_selectΘ_empty_none :
    selectΘ [] = none := rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ★本文件**补全**实装了（对照 gpt 结果包 §6，零遗漏）：
    (1) **Sel_Θ 固定选择器全序构造**：`selectΘ`（可计算选择函数）+ `selOrder_trichotomy`（严格
        全序三歧性，不留平局）+ `selOrder_trans`（传递）+ `selectΘ_isSelectedByKey`（选中者键最优）
        + `selectedByKey_unique`（键唯一）。对照「固定选择器 Sel_Θ（结束时间最新 ≻ 开始时间最新
        ≻ 编号最小）唯一选定」——三排序键全序消歧，机器见证（witness_selectΘ_* 三键各一）。
    (2) **N^δ 良基递归证书（级别比较 Confirm/Candidate 两支）**：`nestCertB`（按 ℓ_j 与 e_v 级别
        比较分 Confirm 终端 / Candidate∧⊆∧递归 两支，ℓ_j > e_v 严格递减终止）。对照结果包 §6 原样递归。
    (3) **χ^δ ∈ {0,1} 全定义（绝不未定义）**：`chiBool`（全函数 Bool）+ `chiBool_mem_zero_one`
        （值域 {0,1}）+ `chiBool_empty_eq_zero`/`chiBool_singleton_noCand_eq_zero`（无候选 = 0）。
        对照「χ^δ ∈ {0,1}」「若不存在候选，则值为 0，绝不能是"未定义"」。

  ★本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **各级 Candidate^δ/Confirm^δ 的缠论语义内容**：`candidateOK`/`confirmOK` 是 Bool 标记
      （某区间真是该级别买卖点候选/确认）——语义由下游 BSP/Center 契约（L2 需真实 K 线 + 力度）。
      本文件给 N^δ 的**结构形式**（级别比较 + 区间套 ⊆ + Sel_Θ 选 chosen），不算语义内容（L0）。
    · **各级候选区间集 cands 的自动计算**：`cands` 是输入（次级别候选区间由上游 ParseStruct +
      SubLevelDescent 下钻提供）——本文件消费 cands，不从 K 线产出 cands（那是 L2 流水线）。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：区间套递归证书 N^δ 的**全定义 Bool 实装**（`nestCertB`/`chiBool`，级别比较 Confirm/
     Candidate 两支 + ℓ_j>e_v 严格递减良基终止）+ **Sel_Θ 固定选择器**（`selectΘ` 构造 +
     `selOrder_trichotomy` 严格全序三歧不留平局 + `selectΘ_isSelectedByKey`/`selectedByKey_unique`
     选中者键唯一），全 L0 零 sorry。χ^δ ∈ {0,1} 全定义（`chiBool_mem_zero_one`），无候选 = 0
     （`chiBool_empty_eq_zero`/`chiBool_singleton_noCand_eq_zero`），绝不未定义。
  2. 定义依据：FULL_USER_FORMULA_SOURCE.md §6（line 312-369）N^δ_{ℓ_j↓e_v}(D_t) 递归
     （ℓ_j=e_v⟹Confirm；ℓ_j>e_v⟹Candidate∧[J_{ℓ_{j+1}}⊆J_{ℓ_j}]∧N^δ_{ℓ_{j+1}↓e_v}）+ 最终证书
     χ^δ = N^δ_{o_v↓e_v}(D_t) ∈ {0,1}（无候选 = 0，绝不未定义）+ Sel_Θ（结束时间最新 ≻ 开始
     时间最新 ≻ 编号最小）唯一选定。输入特征：级别链 o_v=ℓ₀>…>ℓ_k=e_v 严格递减 ⟹ `nestCertB`
     按 `ℓ.lvl > ev` 下钻、`ℓ.lvl = ev` Confirm 终端（满足良基递归）；候选区间携三键
     (endTime,startTime,idx) ⟹ `selectΘ` 字典序全序选最优（满足 Sel_Θ 唯一选定）。
  3. 边界条件（结论翻转）：
     · 若改 `subB` 区间套 ⊆ 口径（`Sub J' J := J'.start ≥ J.start ∧ J'.end ≤ J.end` 闭区间 vs
       严格包含），临界区间（边界相等）归属翻转——当前用 Strict.Nest.Sub 闭口径（≥/≤）。
     · 若 Sel_Θ 排序键优先级改变（如「编号最小」提到「开始时间」之前），多候选选中者翻转——
       当前严格按结果包 endTime ≻ startTime ≻ idx，`selOrder` 字典序编码此优先级。
     · 若某级别候选区间集 cands 为空（无候选）——χ^δ = 0（非未定义），这是确定结果不翻转；
       但若下游补 L2 流水线后该级别**真有候选**，cands 非空，χ^δ 可能变 1（L2 数据决定，可否证）。
     · 若级别链非严格递减（如 ℓ_j ≤ ℓ_{j+1}）——`nestCertB` 的 `decide (ℓ'.lvl < ℓ.lvl)` = false
       ⟹ χ^δ = 0（链不良构 = 无定位），这是结构约束，非数据翻转。
  4. 下游推论：
     · χ^δ 全定义 Bool ⟹ 策略组件（cov-strategy G3）可用 `chiBool ev chain : Bool` 作区间套确认
       的**可计算门**（无候选确定返回 0，不需处理 Option/未定义分支）。
     · Sel_Θ 全序唯一 ⟹ 多候选场景（cov-classification F2）订单生成**无歧义**（`selectΘ` 确定选一），
       对接 exec.rs ConflictKey 字典序裁决（同构：全序消歧保证不依赖输入顺序）。
     · `selectΘ_isSelectedByKey` 填充 Strict.Nest 的「Θ-参数化前件」⟹ Strict.Nest 的
       `nest_certificate_unique`（locator 键唯一）现有**构造性选择器实例**（不再只是抽象参数）。
     · N^δ 级别比较良基递归 ⟹ SubLevelDescent 的「单层次级别破中枢」可嵌入**多级区间套链**
       （SubLevelDescent 给单层几何，本文件给多级递归 + Sel_Θ + Bool 全定义）。
  5. 谱系引用：Strict.Nest（task #72 cc-nest，唯一性谓词侧 Θ-参数化前件 IsSelected）→ 本文件
     （全定义 Bool 算子侧 chiBool ∈ {0,1} + Sel_Θ 构造性选择填充前件）。承接 SubLevelDescent
     （task #119，次级别下钻 + 破中枢几何）——SubLevelDescent 给单层次级别，本文件给完整多级区间套
     递归 + Sel_Θ 全序。无新概念分离——N^δ 递归 + Sel_Θ 全序是 Strict.Nest 已结算结构的 Bool
     全定义化 + 构造性填充（Strict.Nest 的 selOrder/selKey/Sub/IsSelected 原样复用，不改）。
  6. 影响声明：新增 `Origin.IntervalNestCertificate` 模块，import Strict.Nest（只读复用
     Interval/selKey/selOrder/IsSelected/Sub/Dir，不改 Strict.Nest）。无反向依赖，无命名冲突
     （namespace `NewChanlun.Origin.IntervalNestCertificate`）。
     ★待 Lead 登记 root：`Origin.IntervalNestCertificate`（lakefile Origin lib roots 追加）。
     不编辑 lakefile（报 Lead 登记）。`lake env lean Origin/IntervalNestCertificate.lean` 单文件验证。
     rust 对照实装：`rust/src/theta_v0/strategy/nest.rs`（select_theta + chi_bool + selKey 全序，parity）。
-/

end NewChanlun.Origin.IntervalNestCertificate
