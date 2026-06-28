/-
  Origin/NestingCertificate.lean — 区间套证书 N^δ_{ℓ↓e} 的自相似递归（spec §6/§5, P5）
  —— **级别差 (ℓ-e):Nat 结构递归 + ∃!∈{0,1} + 级别平移不变** 的 L0 结构形式化。

  ═══════════════════════════════════════════════════════════════════════════
  唯一信源
  ═══════════════════════════════════════════════════════════════════════════
  权威 spec：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
  §6（P4 line 257-）+ §5（P5 line 273-313，含结果包）+ 形式化指引 Lean 表（line 1214）。
  逐字方框（P5 line 277-305）：

      N^δ_{ℓ↓e}(x) = ⎧ Conf^δ_e(x),                                              ℓ = e,
                      ⎩ Cand^δ_ℓ(x) ∧ [ J^δ_{ℓ-1}(x) ⊆ J^δ_ℓ(x) ] ∧ N^δ_{ℓ-1↓e}(x),  ℓ > e.
      Conf^+_e(x) = ⋁_{i=1}^3 B_{i,e}(x),   Conf^-_e(x) = ⋁_{i=1}^3 S_{i,e}(x).
      由于每次递归级别下降 ℓ > ℓ-1 > … > e，故有限终止。
      ∀ℓ,e,δ,x,  ∃! N^δ_{ℓ↓e}(x) ∈ {0,1}.
      平移不变：N^δ_{ℓ+k↓e+k}(S_k x) = N^δ_{ℓ↓e}(x)，其中 S_k(ℓ) = ℓ + k.

  ═══════════════════════════════════════════════════════════════════════════
  本工位 (lean-nest) 的产出 vs 既有锚点（去重 + no-patch 诚实标注）
  ═══════════════════════════════════════════════════════════════════════════
  既有 `Origin/IntervalNestCertificate.lean`（import Strict.Nest）已实装 N^δ 的**列表链递归**
  （`nestCertB`/`chiBool` 按 `List NestLevel` cons 展开 + Sel_Θ 全序选择器），证了
  χ^δ∈{0,1} + 无候选=0 + Sel_Θ 唯一。但它**没有**：
    (1) **级别差 (ℓ-e):Nat 结构递归**形式（它用列表链，本 spec §6 的方框是对 ℓ 递减、按级别差
        归纳——两者递归骨架不同：列表链 vs Nat 差归纳）；
    (2) **平移不变 N^δ_{ℓ+k↓e+k}(S_k x)=N^δ_{ℓ↓e}(x)**（S_k 级别平移算子；IntervalNestCertificate
        全文无此定理——这是本 spec §5 P5 自相似要求的核心，"区间套在任何级别看起来都一样"）；
    (3) **基例与 Conf^δ_e=⋁B/⋁S 的显式对接**（IntervalNestCertificate 终端用抽象 `confirmOK`
        Bool 标记，未显式写成三类谓词析取）。
  本文件**补**这三项：Nat 差结构递归 + 平移不变 + Conf=析取基例。**不重复** IntervalNestCertificate
  的列表链/Sel_Θ 层（那是另一套递归骨架，单一权威保留在该文件，避免双份漂移）。

  ★为何不 import `Origin.BuySellPredicate`（Conf^δ_e 的 canonical 家）：经实测
    `lake env lean Origin/BuySellPredicate.lean` **不通过**（line 117 Decidable synth 失败 + line 164
    free-variable 错误），且它用 `Fintype`/`Finset`（本工位硬约束**禁** Fintype/Finset + 纯 core）。
    import 一个不可构建且非纯 core 的文件会破坏本文件构建并违反硬约束。故本文件**不** import 它，
    改在纯 core 编码 Conf 的**结构形式**（⋁ 析取，spec P5 逐字），消费**信号向量 b_ℓ∈{0,1}^6 接口**
    （6 个 Bool = B_{1..3}/S_{1..3}）。买卖点谓词的**缠论语义内容**（B_{i,e}=IsType_i∧side）由
    BuySellPredicate/BspClassification 拥有——本文件**不重证**判据，只消费其 Bool 输出（接口契约）。

  ═══════════════════════════════════════════════════════════════════════════
  认识论等级（formalization-validity-domain / 231号 强制标注）
  ═══════════════════════════════════════════════════════════════════════════
  全部 **L0**（纯定义 / Nat 差结构递归 / Bool 值域 / 平移不变归纳，**不依赖任何数据**）。
  `lake env lean Origin/NestingCertificate.lean` 通过 = 「N^δ 级别差递归全定义、值域⊆{0,1}、
  唯一存在、级别平移不变」在**定义层**成立——**不是**任何「区间套定位在真实行情上有效/有 alpha」
  的实证断言（那是 L2/L3，需真实 K 线 + 各级别候选区间 + 力度的真实计算）。
  平移不变是纯结构性质（递归只依赖级别**差** d=ℓ-e，不依赖级别**绝对值**），信息增量 = 同义反复
  （L0），**不冒充** L1+。买卖点择时 v1 已被全窗 L3 8/8 否证（记忆
  `newchanlun-v1-fullwindow-l3-falsified`），但那否证的是 v1 实盘盈利性（L2/L3 经验命题），
  与本文件的 L0 结构定理**认识论等级不同、不可互相否证**——本文件不预判 alpha。

  范式：纯 Bool/Prop + Nat，**零** import（不依赖 Mathlib/Batteries/Std）。
  禁 sorry/admit/axiom。返回 Bool（天然 decidable + 唯一）。Nat 结构递归（终止性自动）。
  禁 Fintype/Finset（本文件全程未用）。不编辑 lakefile（报 Lead 登记 root `Origin.NestingCertificate`）。
-/

namespace NewChanlun.Origin.NestingCertificate

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 方向 δ ∈ {+1,-1} + 区间 J + 每级别数据 + 全市场状态
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **方向 `δ ∈ {+1, -1}`（spec §6 line 261-265）** —— `buy = +1`（买入方向），`sell = -1`（卖出方向）。

  ★对应既有 `NewChanlun.Origin.Side`（long/short，SourceAxioms.lean）：buy↔long↔+1，sell↔short↔-1。
    本文件用本地 `Dir`（不 import SourceAxioms）保持**零依赖纯 core**——`Dir` 是 spec δ 的最小载体，
    语义与 Side 同构（两构造子，无第三态）。下游对接 Side 时一一映射。
-/
inductive Dir where
  | buy   -- δ = +1
  | sell  -- δ = -1
deriving DecidableEq, Repr

/--
  **区间套区间 `J^δ_ℓ(x)`（spec P5 line 279 的 J）** —— 一级别 ℓ 上、方向 δ 的区间套区间，
  用一对有序坐标 `lo ≤ hi` 表示（坐标的缠论语义——价格区间 / 时间×价格区域——由下游契约提供，
  本文件抽象为单调坐标对）。
-/
structure Interval where
  lo : Nat
  hi : Nat
deriving DecidableEq, Repr

/--
  **每级别 ℓ 的数据 `LevelData`（spec §6 一级别的全部输入）** —— 在级别 ℓ 上：
  - `b1 b2 b3`：买点谓词 `B_{1,ℓ}, B_{2,ℓ}, B_{3,ℓ}` 的 Bool 值（信号向量 b_ℓ 的前 3 分量）。
  - `s1 s2 s3`：卖点谓词 `S_{1,ℓ}, S_{2,ℓ}, S_{3,ℓ}` 的 Bool 值（信号向量 b_ℓ 的后 3 分量）。
  - `candBuy candSell`：候选谓词 `Cand^δ_ℓ`（δ=buy/sell 各一支，**抽象** Bool，见 §4 诚实标注）。
  - `jBuy jSell`：区间套区间 `J^δ_ℓ`（δ=buy/sell 各一支）。

  ★`b_i/s_i` 是**信号向量接口**（b_ℓ∈{0,1}^6，spec P02）：本文件**消费**它（Conf = 其析取），
    其缠论语义内容（B_{i,ℓ}=IsType_i∧side=long 等）由 BuySellPredicate/BspClassification 拥有
    （契约锚），本文件**不重证**。
-/
structure LevelData where
  b1 : Bool
  b2 : Bool
  b3 : Bool
  s1 : Bool
  s2 : Bool
  s3 : Bool
  candBuy : Bool
  candSell : Bool
  jBuy : Interval
  jSell : Interval
deriving Repr

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 方向选择器 + 基例 Conf^δ_e = ⋁B / ⋁S（spec P5 line 283-285）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **基例确认 `Conf^δ_e(x)`（spec P5 line 283-285）** —— 执行级 e 的方向 δ 终端确认：
  `Conf^+_e = B_{1,e} ∨ B_{2,e} ∨ B_{3,e}`（买入 = 三类买点析取），
  `Conf^-_e = S_{1,e} ∨ S_{2,e} ∨ S_{3,e}`（卖出 = 三类卖点析取）。

  ★这是 spec P5 line 283-285 的逐字 Bool 形式（⋁ = `||`）。与 BuySellPredicate.ConfBuy/ConfSell
    同结构（同一接口的纯 core 镜像）——本文件消费信号向量 b_ℓ 的 6 分量，**不**重证买卖点判据。
-/
def Conf (δ : Dir) (d : LevelData) : Bool :=
  match δ with
  | .buy  => d.b1 || d.b2 || d.b3
  | .sell => d.s1 || d.s2 || d.s3

/-- **候选谓词 `Cand^δ_ℓ` 的方向选择**（spec P5 line 279；抽象 Bool，见 §4）。 -/
def candOf (δ : Dir) (d : LevelData) : Bool :=
  match δ with
  | .buy  => d.candBuy
  | .sell => d.candSell

/-- **区间套区间 `J^δ_ℓ` 的方向选择**（spec P5 line 279）。 -/
def jOf (δ : Dir) (d : LevelData) : Interval :=
  match δ with
  | .buy  => d.jBuy
  | .sell => d.jSell

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 区间套包含 J^δ_{ℓ-1} ⊆ J^δ_ℓ（**子⊆父**，spec P5 line 279 + 边界条件 line 310）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **区间套包含关系 `Sub child parent`（Prop）** —— 子区间 `child = J^δ_{ℓ-1}` 被父区间
  `parent = J^δ_ℓ` 包含：`parent.lo ≤ child.lo ∧ child.hi ≤ parent.hi`（子区间更窄，落在父内）。

  ★**方向关键（spec P5 边界条件 line 310）**：包含方向是 **J^δ_{ℓ-1} ⊆ J^δ_ℓ（子⊆父）**——
    级别更低（ℓ-1）的区间套**被包含于**级别更高（ℓ）的区间。方向不可反（反则证书失效）。
-/
def Sub (child parent : Interval) : Prop :=
  parent.lo ≤ child.lo ∧ child.hi ≤ parent.hi

/-- **区间套包含的 Bool 判定 `subB`** —— `Sub` 的可计算镜像（子⊆父）。 -/
def subB (child parent : Interval) : Bool :=
  decide (parent.lo ≤ child.lo) && decide (child.hi ≤ parent.hi)

/-- **`subB` ⟺ `Sub`（Bool/Prop 一致，L0）** —— 可计算判定与谓词同真，无漂移。 -/
theorem subB_iff_Sub (child parent : Interval) :
    subB child parent = true ↔ Sub child parent := by
  unfold subB Sub
  simp only [Bool.and_eq_true, decide_eq_true_eq]

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. N^δ_{ℓ↓e} 区间套证书：对级别差 (ℓ-e):Nat 结构递归（spec P5 line 277-279）

    ★递归骨架（与 IntervalNestCertificate 的列表链递归区分）：按级别**差** d = ℓ-e 归纳。
      d = 0 ⟺ ℓ = e ⟹ 基例 Conf^δ_e。
      d+1 ⟺ ℓ = e+(d+1) > e ⟹ Cand^δ_ℓ ∧ [J^δ_{ℓ-1}⊆J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}（ℓ-1 = e+d）。
      每步 d 严格递减 → 有限终止（Nat 结构递归，终止性自动）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **全市场状态 = 级别 → 该级别数据的函数 `f : Nat → LevelData`** 上的区间套证书递归核。

  `nestCert δ f e d`：执行级 `e`、级别差 `d`（当前级别 ℓ = e + d）、方向 δ 的区间套证书值。
  - `d = 0`（ℓ = e）：基例 `Conf^δ_e(x)` = `Conf δ (f e)`。
  - `d+1`（ℓ = e+d+1 > e）：`Cand^δ_ℓ(x) ∧ [J^δ_{ℓ-1} ⊆ J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}(x)`
    = 本级候选 `candOf δ (f (e+d+1))` ∧ 子⊆父 `subB (jOf δ (f (e+d))) (jOf δ (f (e+d+1)))`
      ∧ 递归 `nestCert δ f e d`（次级别 ℓ-1 = e+d）。

  ★**全定义**（对任意 (δ,f,e,d) 有确定 Bool 值，绝不未定义）+ **天然唯一 ∈{0,1}**（返回 Bool）。
  ★**结构递归于 d**（最后一参，d+1 ↦ d 严格递减）⟹ Lean 自动判定终止（对照 spec「级别下降故有限终止」）。
-/
def nestCert (δ : Dir) (f : Nat → LevelData) (e : Nat) : Nat → Bool
  | 0 => Conf δ (f e)
  | d + 1 =>
      candOf δ (f (e + d + 1))
        && subB (jOf δ (f (e + d))) (jOf δ (f (e + d + 1)))
        && nestCert δ f e d

/--
  **区间套证书 `N^δ_{ℓ↓e}(x)`（spec P5 方框）** —— 给级别 `ℓ`、执行级 `e`（前置约束 e ≤ ℓ），
  对级别差 `ℓ - e` 跑结构递归。`N δ f ℓ e := nestCert δ f e (ℓ - e)`。

  ★前置约束 `e ≤ ℓ`（spec P4 §6）：当 ℓ < e 时 Nat 截断 `ℓ - e = 0`，N 退化为基例 Conf^δ_e
    （超出递归定义域，由上游保证 e ≤ ℓ；本文件不臆造 ℓ<e 的语义，只如实标注截断行为）。
-/
def N (δ : Dir) (f : Nat → LevelData) (ℓ e : Nat) : Bool :=
  nestCert δ f e (ℓ - e)

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 基例 + 递归步 的方程刻画（spec P5 方框分段，逐条机器见证）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **基例 ℓ = e ⟹ N = Conf^δ_e（spec P5 line 277 上支，L0）**。 -/
theorem N_base (δ : Dir) (f : Nat → LevelData) (e : Nat) :
    N δ f e e = Conf δ (f e) := by
  unfold N
  rw [Nat.sub_self]
  simp only [nestCert]

/--
  **递归步 ℓ = e+(d+1) > e ⟹ N = Cand ∧ [子⊆父] ∧ N_{ℓ-1↓e}（spec P5 line 279 下支，L0）**。
  级别 ℓ = e+(d+1)，次级别 ℓ-1 = e+d。子⊆父 = `subB (jOf δ (f (e+d))) (jOf δ (f (e+(d+1))))`。
-/
theorem N_step (δ : Dir) (f : Nat → LevelData) (e d : Nat) :
    N δ f (e + d + 1) e
      = (candOf δ (f (e + d + 1))
          && subB (jOf δ (f (e + d))) (jOf δ (f (e + d + 1)))
          && N δ f (e + d) e) := by
  unfold N
  have h1 : e + d + 1 - e = d + 1 := by omega
  have h2 : e + d - e = d := by omega
  rw [h1, h2]
  simp only [nestCert]

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. ∃! ∈ {0,1}（spec P5 line 295：∀ℓ,e,δ,x, ∃! N^δ_{ℓ↓e}(x) ∈ {0,1}）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **N^δ ∈ {0,1}（值域见证，L0）** —— `N` 是 Bool（恰两值 true/false = 1/0）。Bool 的值域恰是
  {false,true}={0,1}——类型层事实（`N : Dir → … → Bool`），对照 spec line 295「∈{0,1}」。
-/
theorem N_mem_zero_one (δ : Dir) (f : Nat → LevelData) (ℓ e : Nat) :
    N δ f ℓ e = false ∨ N δ f ℓ e = true := by
  cases N δ f ℓ e
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  **∃! N^δ_{ℓ↓e}(x) ∈ {0,1}（唯一存在，spec P5 line 295，L0）** —— 对任意 δ,f,ℓ,e，存在**唯一**
  Bool 值 u 使 `N δ f ℓ e = u`。返回 Bool ⟹ 唯一性天然（函数单值 + Bool 值域），对照 spec ∃! 方框。

  ★形式：`ExistsUnique` 的展开式 `∃ u, (N = u) ∧ ∀ v, (N = v) → v = u`。本项目纯 core
    **无** Mathlib/Std 的 `∃!` 记号（`ExistsUnique`），故用展开式直写——语义等同 spec 的 ∃!。
-/
theorem N_exists_unique (δ : Dir) (f : Nat → LevelData) (ℓ e : Nat) :
    ∃ u : Bool, N δ f ℓ e = u ∧ ∀ v : Bool, N δ f ℓ e = v → v = u :=
  ⟨N δ f ℓ e, rfl, fun _ h => h.symm⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 级别平移不变 N^δ_{ℓ+k↓e+k}(S_k x) = N^δ_{ℓ↓e}(x)（spec P5 line 299-305）

    ★本工位核心新增（IntervalNestCertificate 无此定理）。S_k 级别平移算子：S_k(ℓ)=ℓ+k，
      对状态 f 的作用 `(S_k f)(m) = f(m-k)`（在级别 m 看到原级别 m-k 的数据；level 上移 k）。
      自相似要求"区间套在任何级别看起来都一样" ⟺ N 只依赖级别**差** d=ℓ-e，不依赖绝对级别。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **级别平移算子 `S_k` 对状态的作用（spec P5 line 301）** —— `(S_k f)(m) = f(m - k)`：
  平移后的状态在级别 m 处呈现的是原状态在级别 m-k 处的数据（级别整体上移 k）。
  这正是 `S_k(ℓ) = ℓ + k`（原级别 ℓ 的数据移到级别 ℓ+k）的状态侧表达。
-/
def shift (k : Nat) (f : Nat → LevelData) : Nat → LevelData :=
  fun m => f (m - k)

/-- **`shift` 应用化简（L0）** —— `shift k f m = f (m - k)`（定义展开，simp 用）。 -/
@[simp] theorem shift_apply (k : Nat) (f : Nat → LevelData) (m : Nat) :
    shift k f m = f (m - k) := rfl

/--
  **平移不变核心引理（对级别差 d 归纳，L0）** —— 对任意级别差 `d`：
  `nestCert δ (S_k f) (e+k) d = nestCert δ f e d`。

  ★证明骨架：递归只用级别**差** d（基例读 f(e)；递归步读 f(e+d), f(e+d+1)），平移把执行级
    e→e+k、各级别读取点同步 +k，差 d 不变 ⟹ 逐级数据对应相等 ⟹ 证书值相等。归纳于 d。
-/
theorem nestCert_shift (δ : Dir) (f : Nat → LevelData) (e k : Nat) :
    ∀ d, nestCert δ (shift k f) (e + k) d = nestCert δ f e d := by
  intro d
  induction d with
  | zero =>
      -- 基例：Conf δ ((S_k f)(e+k)) = Conf δ (f ((e+k)-k)) = Conf δ (f e)
      simp only [nestCert, shift_apply]
      have he : e + k - k = e := by omega
      rw [he]
  | succ d ih =>
      -- 递归步：各读取点 (e+k)+d+1, (e+k)+d 平移回 e+d+1, e+d；递归段用 ih
      simp only [nestCert, shift_apply]
      have h1 : e + k + d + 1 - k = e + d + 1 := by omega
      have h2 : e + k + d - k = e + d := by omega
      rw [h1, h2, ih]

/--
  **★级别平移不变 `N^δ_{ℓ+k↓e+k}(S_k x) = N^δ_{ℓ↓e}(x)`（spec P5 line 299，L0）** ——
  平移后状态 `S_k f` 在平移后级别 (ℓ+k ↓ e+k) 上的区间套证书 = 原状态 f 在原级别 (ℓ↓e) 上的证书。

  ★这机器见证 spec P5「区间套在任何级别看起来都一样」（自相似要求方框）。证明：级别差不变
    `(ℓ+k)-(e+k) = ℓ-e`，再用 `nestCert_shift`。这是**纯结构** L0（不依赖任何数据）。
-/
theorem N_translation_invariant (δ : Dir) (f : Nat → LevelData) (ℓ e k : Nat) :
    N δ (shift k f) (ℓ + k) (e + k) = N δ f ℓ e := by
  unfold N
  have hd : ℓ + k - (e + k) = ℓ - e := by omega
  rw [hd]
  exact nestCert_shift δ f e k (ℓ - e)

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. 反退化见证（N^δ 真跑通：具体三级区间套链 ⟹ N = 1，非平凡桩）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 占位空数据（未触及级别用；全 false / 平凡区间）。 -/
def emptyLevel : LevelData :=
  { b1 := false, b2 := false, b3 := false, s1 := false, s2 := false, s3 := false
    candBuy := false, candSell := false
    jBuy := { lo := 0, hi := 0 }, jSell := { lo := 0, hi := 0 } }

/--
  反退化见证状态 `witField`：执行级 0 ≺ 中间级 1 ≺ 操作级 2（买入方向）。
  各级 jBuy 真套缩小：level2 [5,25] ⊇ level1 [8,22] ⊇ level0 [10,20]（子⊆父）。
  level0 b1=true（Conf^+_0 = true）；level1/2 candBuy=true。
-/
def witField : Nat → LevelData
  | 0 => { emptyLevel with b1 := true, candBuy := true, jBuy := { lo := 10, hi := 20 } }
  | 1 => { emptyLevel with candBuy := true, jBuy := { lo := 8, hi := 22 } }
  | 2 => { emptyLevel with candBuy := true, jBuy := { lo := 5, hi := 25 } }
  | _ => emptyLevel

/-- **★反退化见证：三级区间套链 N^+_{2↓0} = 1（真跑通，L0）** —— 操作级 2 ≻ 中间级 1 ≻ 执行级 0，
    各级 jBuy 真套缩小（子⊆父）+ 候选/确认成立 ⟹ N = true。证 N 非平凡桩（能产 1）。 -/
theorem witness_N_three_level_one :
    N Dir.buy witField 2 0 = true := by native_decide

/-- **★反退化见证：基例 ℓ=e ⟹ N = Conf（单级退化区间套，L0）** —— 操作级 = 执行级 0
    ⟹ N^+_{0↓0} = Conf^+_0 = b1 = true。 -/
theorem witness_N_base_one :
    N Dir.buy witField 0 0 = true := by native_decide

/--
  区间套破坏状态：执行级 0 的 jBuy = [2,30] **超出**中间级 1 的 [8,22]（2<8 ∧ 30>22 ⟹ 子⊄父）。
-/
def witBrokenField : Nat → LevelData
  | 0 => { emptyLevel with b1 := true, candBuy := true, jBuy := { lo := 2, hi := 30 } }
  | 1 => { emptyLevel with candBuy := true, jBuy := { lo := 8, hi := 22 } }
  | 2 => { emptyLevel with candBuy := true, jBuy := { lo := 5, hi := 25 } }
  | _ => emptyLevel

/-- **★反退化见证：区间套不成立 ⟹ N = 0（L0）** —— 执行级区间 [2,30] 超出中间级 [8,22]
    （子⊄父）⟹ subB = false ⟹ N = 0。坐实 subB 子⊆父方向真起门控作用（非平凡真桩）。 -/
theorem witness_N_broken_nest_zero :
    N Dir.buy witBrokenField 2 0 = false := by native_decide

/-- **★反退化见证：级别平移不变具体实例（L0）** —— 把 witField 整体上移 k=3，
    在级别 (2+3 ↓ 0+3) 上的证书 = 原 (2↓0) 证书（= true）。机器见证平移不变非空洞。 -/
theorem witness_translation_concrete :
    N Dir.buy (shift 3 witField) (2 + 3) (0 + 3) = N Dir.buy witField 2 0 :=
  N_translation_invariant Dir.buy witField 2 0 3

/-- **★反退化见证：卖出方向 Conf^-_e = ⋁S（基例方向对偶，L0）** —— 卖出基例读 s_i 析取。 -/
theorem witness_conf_sell :
    Conf Dir.sell { emptyLevel with s2 := true } = true := by native_decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 9. still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ★本文件**实装**（对照 spec P5 方框，零遗漏）：
    (1) **基例 ℓ=e ⟹ Conf^δ_e = ⋁B/⋁S**（`Conf` + `N_base`，spec line 277/283-285 逐字 ⋁ 析取）。
    (2) **递归步 ℓ>e ⟹ Cand ∧ [子⊆父] ∧ N_{ℓ-1↓e}**（`nestCert`/`N` 对 (ℓ-e):Nat 结构递归 +
        `N_step` 方程刻画，spec line 279；`Sub`/`subB` 子⊆父方向 spec line 310）。
    (3) **∃! ∈ {0,1}**（`N_mem_zero_one` 值域 + `N_exists_unique` 唯一存在，spec line 295）。
    (4) **级别平移不变**（`N_translation_invariant` + `nestCert_shift`，spec line 299-305；
        本工位核心新增，IntervalNestCertificate 无此定理）。
    (5) **有限终止**（Nat 结构递归 d+1↦d，Lean 自动判定，对照 spec line 287-291「级别下降故有限终止」）。

  ★本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **Cand^δ_ℓ 的独立定义式**：spec 疑点2（line ~258/279）`Cand^δ_ℓ` 在 PDF **仅作符号**，
      **未给独立定义式**。本文件形式化为**抽象 Bool 谓词**（`candBuy`/`candSell` 字段，`candOf` 选择）
      ——**不臆造定义**。[需人工确认] Cand^δ_ℓ 的精确判据（候选买卖点的级别相关条件）由下游/spec 后续补。
    · **买卖点谓词 B_{i,e}/S_{i,e} 的缠论语义内容**：`b1..b3`/`s1..s3` 是信号向量 b_ℓ∈{0,1}^6 的
      **Bool 接口**（spec P02）；其语义（B_{i,e}=IsType_i∧side）由 BuySellPredicate/BspClassification
      拥有（契约锚），本文件**消费**不重证。[需人工确认] 第三类谓词边界：BspClassification 的
      `IsType3` 用严格 `<`（ZG），reference §36 含等号 `≥`（记忆
      `theta-v0-type3-boundary-reference-lean-conflict`）——此冲突属买卖点谓词层（B_3/S_3 的内容），
      **本文件不引入新冲突**（Conf 只对 b_ℓ 的 Bool 输出做析取，不取定 >/≥ 边界），既有 type3 冲突
      仍是待裁决项，不属本工位 escalate 范围。
    · **状态 f 的自动构造**：`f : Nat → LevelData`（每级别的信号/候选/区间）是**输入**（上游
      ParseStruct + 各级别识别提供），本文件消费 f，不从 K 线产出 f（那是 L2 流水线）。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：区间套证书 `N^δ_{ℓ↓e}` 的 **L0 结构形式化**——对级别差 (ℓ-e):Nat 结构递归
     （`nestCert`/`N`，基例 Conf^δ_e=⋁B/⋁S + 递归步 Cand∧[子⊆父]∧N_{ℓ-1↓e}）+ **∃!∈{0,1}**
     （`N_mem_zero_one`/`N_exists_unique`）+ **级别平移不变** `N^δ_{ℓ+k↓e+k}(S_k x)=N^δ_{ℓ↓e}(x)`
     （`N_translation_invariant`）+ 有限终止（Nat 结构递归自动）。全 L0，零 sorry/admit/axiom。
     主定理：`N_exists_unique`（∃!∈{0,1}）+ `N_translation_invariant`（平移不变）。
  2. 定义依据：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md` §6（P4 line 257-）
     + §5（P5 line 277-313）。逐字方框：N^δ_{ℓ↓e} 分段（ℓ=e⟹Conf^δ_e；ℓ>e⟹Cand^δ_ℓ∧[J^δ_{ℓ-1}⊆J^δ_ℓ]
     ∧N^δ_{ℓ-1↓e}）+ Conf^±_e=⋁_{i=1}^3 B/S（line 283-285）+ ∃!∈{0,1}（line 295）+ 平移不变
     N^δ_{ℓ+k↓e+k}(S_k x)=N^δ_{ℓ↓e}(x), S_k(ℓ)=ℓ+k（line 299-305）。输入特征：级别链 ℓ>ℓ-1>…>e
     严格递减 ⟹ 级别差 d=ℓ-e 上 Nat 结构递归（满足有限终止）；包含方向**子⊆父**（line 310 边界条件）
     ⟹ `Sub child parent := parent.lo≤child.lo ∧ child.hi≤parent.hi`。
  3. 边界条件（结论翻转）：
     · **包含方向**：当前 `Sub` = 子⊆父（J^δ_{ℓ-1}⊆J^δ_ℓ，spec line 310）。若反向（父⊆子）则
       `subB` 翻转，临界（边界相等）归属翻转——方向不可反（spec 明确"反则证书失效"）。
     · **闭/开区间口径**：`Sub` 用 `≤`（闭口径，含边界相等）。若改严格 `<`（开区间），边界相等的
       区间套归属翻转——当前含等号（与 subB decide(≤) 一致）。
     · **Conf 析取 vs 互斥**：基例 Conf^δ_e 用 `||`（析取，任一买/卖点即确认，spec line 283-285）。
       若改互斥（要求恰一类），Conf 语义翻转——当前不互斥（spec P25 买卖点可重合，互斥推迟到角色层）。
     · **执行级前置 e≤ℓ**：ℓ<e 时 Nat 截断 ℓ-e=0，N 退化为 Conf^δ_e（超定义域，上游保证 e≤ℓ）。
     · **Cand^δ_ℓ 抽象**：当前 `candOf` 是抽象 Bool。下游补 Cand 独立定义式后，candOf 实例化，
       N 的触发集随 Cand 内容变（L2 数据/定义层决定，可否证）——但平移不变/∃!/值域**不**随之翻转
       （它们只依赖 N 是 Bool 函数 + 递归只用级别差，与 Cand 内容无关）。
  4. 下游推论：
     · `N : Dir→(Nat→LevelData)→Nat→Nat→Bool` 全定义 ⟹ 策略层可用 `N δ f ℓ e : Bool` 作区间套
       确认的**可计算门**（无需处理 Option/未定义分支）。
     · **平移不变** ⟹ 同一区间套语法在所有级别复用（spec 自相似）：级别 e 训练的区间套门可平移到
       任意级别 e+k 而证书值不变——支撑"统一归一化语法 b_*"（spec §5 P4 line 245 自相似方框）。
     · `N_base`（Conf 终端）⟹ 与 BuySellPredicate.ConfBuy/ConfSell 接口对齐：本文件消费信号向量
       b_ℓ∈{0,1}^6（BuySellPredicate 产出），区间套递归底锚到 Conf 全定义。
     · 与 `IntervalNestCertificate.nestCertB`（列表链递归 + Sel_Θ）**互补不冲突**：本文件给 Nat 差
       递归骨架 + 平移不变；IntervalNestCertificate 给列表链 + Sel_Θ 全序消歧。两套递归骨架各自
       单一权威（避免双份漂移）；[需人工确认] 是否后续需证两骨架等价（同一 N^δ 的两种实现）。
  5. 谱系引用：
     · 既有 `Origin/IntervalNestCertificate.lean`（task #72/#119 系，列表链 + Sel_Θ）→ 本文件
       （Nat 差结构递归 + 平移不变 + Conf 析取基例）。无新概念分离——同一 spec §6 区间套递归的
       **另一递归骨架（按级别差）+ 自相似平移不变**的形式化。
     · 触及记忆 `theta-v0-type3-boundary-reference-lean-conflict`（第三类边界 reference 含等号 vs
       Lean 严格<）：本文件 Conf 只对 b_ℓ 的 Bool 输出析取，**不**触碰 B_3/S_3 的 >/≥ 边界，
       **不引入新冲突**（该冲突仍属买卖点谓词层待裁决项）。
     · **不确定**是否有「区间套自相似平移不变」的更早专属谱系记录——本工位不臆造谱系，明确标注此
       不确定性（建议 genealogist 核 `.chanlun/genealogy/` 确认 spec §5 P5 平移不变是否首次形式化）。
  6. 影响声明：新建 `Origin/NestingCertificate.lean`，**零 import**（不依赖任何既有 Lean/Mathlib/
     Batteries/Std），**不改**任何既有 Lean/rust/定义/spec。namespace
     `NewChanlun.Origin.NestingCertificate`（无命名冲突）。下游：为策略层区间套确认门 + 自相似级别
     复用提供 Bool 全定义证书 + 平移不变定理。不碰 lakefile（报 Lead 登记 root `Origin.NestingCertificate`）。
     `lake env lean Origin/NestingCertificate.lean` 单文件验证。
-/

end NewChanlun.Origin.NestingCertificate
