/-
Origin/BuySellPredicate.lean — 买卖点谓词语法基底（BSP-L1，操作语义入口层 L0）

★工位定位（严格全互斥定义策略「操作语义入口层」的 Lean 形式化根）：
  互斥分类形式化（M01–M30）与完全分类（C01–C42）都从「元素方向 ε」当原始输入起步，
  **不暴露「买卖点判定」作为语法基底**。本文件补上内在语法层所缺的入口——
  买卖点谓词 `B_{i,ℓ}/S_{i,ℓ}` → 信号向量 `b_ℓ∈{0,1}^6` → 64 态穷尽。
  这是 spec §B P01/P02/P03 的 canonical 形式化（spec 文件
  `.chanlun/specs/2026-06-28-buy-sell-point-operational-semantics-pdf-extract.md`）。

  与既有 `BspClassification.lean` 的关系（复用，不重定义）：
  - `BspClassification` 提供**单端点厚判据** `IsType1/IsType2/IsType3Buy/IsType3Sell`
    （作用于单个 `BspEndpoint`）+ 完备性 + 单射裁定。
  - 本文件在其上构造**带级别 ℓ 索引的 6 谓词**（按 类别 i × 方向 buy/sell 组织）+
    把 6 谓词打包为信号向量 + 证 64 态穷尽。本文件**不重证**三类判据内容，只引用既有判据。
  - 与 `BspEventBridge.lean`（薄 Bsp ↦ 厚事件桥）、`CompleteStateEvent.lean`（状态 schema）
    不重叠：那两个不做「6 谓词 + 信号向量 + 64 态穷尽」这一层。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链 + spec PDF）
═══════════════════════════════════════════════════════════════════════════
- spec §B P01（§1, 页1）：对每级别 ℓ，`B_{i,ℓ}(x)∈{0,1}, S_{i,ℓ}(x)∈{0,1}, i=1,2,3`，
  每谓词「**全定义、单值、可判定、因果**」`∀x,ℓ,i, B_{i,ℓ}(x),S_{i,ℓ}(x)∈{0,1}`；
  **买卖点本身可重合**故不要求 `B_{i,ℓ}S_{j,ℓ}=0`。
- spec §B P02（§1, 页1–2）：信号向量 `b_ℓ(x)=(B_{1,ℓ},B_{2,ℓ},B_{3,ℓ},S_{1,ℓ},S_{2,ℓ},S_{3,ℓ})∈{0,1}^6`；
  每级买卖点状态严格分类为 `2^6=64` 种；`Σ_{u∈{0,1}^6} 1[b_ℓ(x)=u]=1`。
- spec §B P03（§1, 页1）：买卖点谓词全定义即可判定——因果：只依赖当前及历史状态，不依赖未来。
- maimai.md §10.1：三类买卖点定义（一类背驰点 / 二类回抽 / 三类离开中枢回抽不破）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain / 231号 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 谓词全定义因果结构 / 信号向量打包 / 64 态穷尽组合证明，
decide + List 显式枚举 machine-checked，不依赖任何数据）。
`lake env lean Origin/BuySellPredicate.lean` 通过 = 「6 买卖点谓词全定义因果 + 信号向量 64 态
穷尽」在定义层成立——**不是**任何「买卖点识别在真实行情上有 alpha」的实证断言（L2/L3）。

★诚实标注（gatekeeper 防膨胀）：
  - 买卖点择时 v1 已被**全窗 L3 8/8 否证**（n_beats_random=0/8，记忆
    `newchanlun-v1-fullwindow-l3-falsified`）。但 L3 否证的是 **v1 买卖点择时实盘盈利性
    （L2/L3 经验命题）**，本文件证的是 **6 谓词全定义 + 64 态穷尽的 L0 结构性质**——
    两者认识论等级不同、**不可互相否证**。本文件**不预判 alpha**。
  - 「64 态穷尽」是 `b_ℓ` 作为函数到 `{0,1}^6` 的单值性的直接推论（信息增量零的同义反复）——
    它说「每个状态在每级别落入恰好一个 6 维向量」，**不**说「真实市场会出现全部 64 态」。
  - **买卖点可重合**（不强制互斥）是本层的核心方法论（spec P25）：互斥推迟到角色层（BSP-L5），
    本层只穷尽不互斥。本文件**显式**编码这一点（`coincidence_allowed` 见证 2 类 3 类可重合）。

★纯 core 硬约束（本工位 reconcile 重点）：
  本文件**只用 Lean 4 core**（List/Bool/Option/Int + decide），**禁 Mathlib**
  （禁 `Fintype`/`Finset`/`Fintype.card`/`∃!` 通知/`tauto`）。64 态穷尽用 `allBSP : List`
  显式枚举 + `List.filter ... |>.length`（指示函数 Σ 的 List 形式），不用 `Fintype.card`。

禁 sorry/admit/axiom。纯 Prop/Type + Bool/List/Int，不依赖 Mathlib。
**不编辑 lakefile**（报 Lead 登记 root `Origin.BuySellPredicate`）。
依赖方向（单向无环，全 committed 只读）：BuySellPredicate → {BspClassification, ChanlunElements}。
-/

import Origin.BspClassification

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 级别 + 买卖点类别索引（spec P01：i∈{1,2,3}，ℓ 级别）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **买卖点类别索引（spec P01）** —— `i∈{1,2,3}` 三类买卖点。与 `BspClass`（one/two/three）
  一一对应，本文件用 `Fin 3` 风格的独立枚举承载「谓词索引」语义（B_{i}/S_{i} 的下标 i）。
-/
inductive BspIndex where
  | i1  -- 第一类
  | i2  -- 第二类
  | i3  -- 第三类
deriving DecidableEq, Repr

/-- 把谓词索引映回 `BspClass`（既有 `BspClassification` 的判据类型）。 -/
def BspIndex.toClass : BspIndex → BspClass
  | .i1 => BspClass.one
  | .i2 => BspClass.two
  | .i3 => BspClass.three

/--
  **每级别买卖点端点视图（spec P01：因果状态投影）** —— 对一个固定级别 ℓ，状态 x 投影出
  该级别上每个类别 i 的「候选买卖点端点」（若该类未触发则 `none`）。

  ★因果性（spec P03）：本结构是状态 x 在级别 ℓ 的**当前投影**，只读当前及历史（端点判据
  字段如 brokeCenter/divPair/retracePrice 均由历史 K 线派生），**不依赖未来**。
  本文件不构造投影函数（那是 rust signal.rs 的实装层 / `bspOf` 全自动构造层，
  BspClassification still-MISSING-D）——本文件证「**给定**投影后 6 谓词全定义 + 信号向量穷尽」。
-/
structure LevelView where
  /-- 该级别第一类候选端点（none = 该级别本步无第一类候选）。 -/
  cand1 : Option BspEndpoint
  /-- 该级别第二类候选端点。 -/
  cand2 : Option BspEndpoint
  /-- 该级别第三类候选端点。 -/
  cand3 : Option BspEndpoint
deriving Repr

/-- 按索引取候选端点（全定义：每个 `BspIndex` 都有确定的 `Option` 输出）。 -/
def LevelView.cand (v : LevelView) : BspIndex → Option BspEndpoint
  | .i1 => v.cand1
  | .i2 => v.cand2
  | .i3 => v.cand3

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 六个买卖点谓词 B_{i,ℓ}/S_{i,ℓ}（spec P01）—— 全定义、单值、可判定、因果
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **三类判据可判定实例（纯 core）** —— `BspClassification` 把 `IsType1/IsType2/IsType3*`
  定义为 `def ... : Prop`（无 Decidable 实例），实例合成不会自动展开 `def`。本文件为它们补上
  **透明 term-mode** Decidable 实例（用 `inferInstanceAs` 锚到定义体的可判定原子：Bool 相等 +
  `IsDivergence`（既有实例）+ `Int <`（core DecidableLT）+ `Side` 相等（DecidableEq）），
  使 `decide` 在 kernel 可归约。这修复原稿 line117「Decidable synth 失败」根因
  （原 `cases i <;> unfold <;> infer_instance` 把 `match` 留作未归约的 matcher 应用，
  实例合成对 matcher 不归约 ⟹ 失败）。
-/
instance instDecIsType1 (e : BspEndpoint) : Decidable (IsType1 e) :=
  inferInstanceAs (Decidable (e.brokeCenter = true ∧ IsDivergence e.divPair))

instance instDecIsType2 (e : BspEndpoint) : Decidable (IsType2 e) :=
  inferInstanceAs (Decidable (e.afterTypeOne = true ∧ e.brokeCenter = false))

instance instDecIsType3Buy (e : BspEndpoint) : Decidable (IsType3Buy e) :=
  inferInstanceAs (Decidable
    (e.side = Side.long ∧ e.leftCenter = true ∧ e.firstRetrace = true ∧
      e.center.zg < e.retracePrice))

instance instDecIsType3Sell (e : BspEndpoint) : Decidable (IsType3Sell e) :=
  inferInstanceAs (Decidable
    (e.side = Side.short ∧ e.leftCenter = true ∧ e.firstRetrace = true ∧
      e.retracePrice < e.center.zd))

instance instDecIsType3 (e : BspEndpoint) : Decidable (IsType3 e) :=
  inferInstanceAs (Decidable (IsType3Buy e ∨ IsType3Sell e))

/--
  **类别 i 的判据满足（复用 `BspClassification` 厚判据）** —— 端点 e 是否满足第 i 类买卖点判据。
  i1 ⟹ `IsType1`（破中枢背驰点）；i2 ⟹ `IsType2`（一类后回抽结束）；i3 ⟹ `IsType3`（离开中枢回抽不破）。
-/
def endpointSatisfies : BspIndex → BspEndpoint → Prop
  | .i1, e => IsType1 e
  | .i2, e => IsType2 e
  | .i3, e => IsType3 e

/--
  **★全定义可判定（L0，纯 core，修复 line117）** —— 对每个 `(i, e)`，`endpointSatisfies i e`
  可判定。term-mode `match i` 实例：对具体构造子 i 归约到对应 `IsType_k e` 的实例（上方透明实例），
  kernel 可归约（避免原稿 `infer_instance` 对未归约 matcher 失败）。
-/
instance instDecEndpointSatisfies (i : BspIndex) (e : BspEndpoint) :
    Decidable (endpointSatisfies i e) :=
  match i with
  | .i1 => inferInstanceAs (Decidable (IsType1 e))
  | .i2 => inferInstanceAs (Decidable (IsType2 e))
  | .i3 => inferInstanceAs (Decidable (IsType3 e))

/--
  **买点谓词 `B_{i,ℓ}(x)`（spec P01，Bool 全定义因果）** —— 在级别视图 v、类别 i 上是否触发
  **买点**（long 方向）：该级别第 i 类候选端点存在、满足第 i 类判据、且方向为 long。

  ★全定义（spec P01/P03）：对任意 (v, i) 返回确定 `Bool`（`none` ⟹ false；判据不满足 ⟹ false）。
  ★因果：只读 v（当前级别投影），不依赖未来。
  ★可重合（spec P02/P25）：不要求与 `S` 或其他类别互斥——重合由 64 态向量编码，非本谓词排除。
-/
def B (v : LevelView) (i : BspIndex) : Bool :=
  match v.cand i with
  | none => false
  | some e => decide (endpointSatisfies i e) && decide (e.side = Side.long)

/--
  **卖点谓词 `S_{i,ℓ}(x)`（spec P01，对偶）** —— 该级别第 i 类候选端点存在、满足判据、方向为 short。
-/
def S (v : LevelView) (i : BspIndex) : Bool :=
  match v.cand i with
  | none => false
  | some e => decide (endpointSatisfies i e) && decide (e.side = Side.short)

/--
  **★P03 全定义即可判定（L0）** —— 每个买卖点谓词在每个 (级别视图, 类别) 都有确定 0/1 判定。
  形式化为：`B v i` 与 `S v i` 都是 `Bool`（要么 `true` 要么 `false`），无第三值、无未定义。
-/
theorem B_total (v : LevelView) (i : BspIndex) : B v i = true ∨ B v i = false := by
  cases h : B v i
  · exact Or.inr rfl
  · exact Or.inl rfl

theorem S_total (v : LevelView) (i : BspIndex) : S v i = true ∨ S v i = false := by
  cases h : S v i
  · exact Or.inr rfl
  · exact Or.inl rfl

/--
  **★P01 买卖点谓词单值（L0，修复 line164）** —— 同一类别 i 的买点谓词与卖点谓词**不能同时为真**
  （单个端点方向唯一：long ≠ short）。这刻画 spec P01「单值」——同一候选端点不会既算买点又算卖点。
  ★注意：这**不是**跨类别互斥（不同类别 i≠j 的 B_i 与 B_j 可同时真，买卖点可重合 spec P02）。

  ★修复 line164「Expected type must not contain free variables」根因：原稿
  `cases hc : v.cand i ... rw [hc] at hb; exact absurd hb (by decide)` 让 `by decide` 的期望类型
  保留未归约的 `match none with | none => .. | some e => decide (endpointSatisfies i e) ..`——其
  some 分支含自由变量 `i`，decide 无法对含自由变量的目标合成判定。改用 `split`（对 match 直接分裂
  并 iota-归约，none 分支得纯 `false = true`，无自由变量残留）。
-/
theorem B_S_same_index_exclusive (v : LevelView) (i : BspIndex) :
    ¬ (B v i = true ∧ S v i = true) := by
  rintro ⟨hb, hs⟩
  unfold B at hb
  unfold S at hs
  split at hb
  · exact absurd hb (by decide)
  · next e heq =>
    rw [heq] at hs
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hb hs
    have h1 : e.side = Side.long := hb.2
    have h2 : e.side = Side.short := hs.2
    rw [h1] at h2
    exact Side.noConfusion h2

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 信号向量 b_ℓ ∈ {0,1}^6（spec P02）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **信号向量 `b_ℓ(x) ∈ {0,1}^6`（spec P02）** —— 六个买卖点谓词打包为 6 维 Bool 向量，
  分量顺序按 spec P02：`(B_{1,ℓ}, B_{2,ℓ}, B_{3,ℓ}, S_{1,ℓ}, S_{2,ℓ}, S_{3,ℓ})`。
  用 6 个具名 Bool 字段精确对齐 spec 的 6 分量（{0,1}^6 = Bool^6）。
-/
structure SignalVector where
  b1 : Bool  -- B_{1,ℓ}
  b2 : Bool  -- B_{2,ℓ}
  b3 : Bool  -- B_{3,ℓ}
  s1 : Bool  -- S_{1,ℓ}
  s2 : Bool  -- S_{2,ℓ}
  s3 : Bool  -- S_{3,ℓ}
deriving DecidableEq, Repr

/--
  **信号向量构造 `b_ℓ(x)`（spec P02）** —— 从级别视图 v 计算 6 维信号向量。
  ★这是从市场状态（级别投影 v）到 `{0,1}^6` 的**函数**——函数单值性即 64 态穷尽（见下）。
-/
def signalVector (v : LevelView) : SignalVector :=
  { b1 := B v .i1, b2 := B v .i2, b3 := B v .i3
    s1 := S v .i1, s2 := S v .i2, s3 := S v .i3 }

/--
  **{0,1}^6 全枚举（纯 core，替代 `Fintype.card`）** —— 显式列出全部 `2^6 = 64` 个信号向量
  （6 个 Bool 字段的笛卡尔积，List Monad do-记法 = 嵌套 flatMap）。这是 spec P02「2^6=64 种」
  的 List 显式见证——**不用 `Fintype`/`Finset`**（纯 core 硬约束）。
-/
def allBSP : List SignalVector :=
  [false, true].flatMap (fun b1 =>
  [false, true].flatMap (fun b2 =>
  [false, true].flatMap (fun b3 =>
  [false, true].flatMap (fun s1 =>
  [false, true].flatMap (fun s2 =>
  [false, true].map (fun s3 =>
    { b1 := b1, b2 := b2, b3 := b3, s1 := s1, s2 := s2, s3 := s3 }))))))

/--
  **★P02 信号向量恰 64 个（L0，替代 `Fintype.card SignalVector = 64`）** —— 全枚举 `allBSP`
  的长度恰为 `2^6 = 64`。这坐实「每级买卖点状态严格分类为 64 种」，用 `List.length` 而非
  `Fintype.card`（纯 core）。
-/
theorem allBSP_length_64 : allBSP.length = 64 := by decide

/--
  **★P02 全枚举无重复（L0）** —— `allBSP` 64 个信号向量两两不同（`List.Nodup`）。
  配合 `allBSP_length_64` + `allBSP_complete` 坐实「恰 64 个互异状态」（穷尽且不重叠）。
-/
theorem allBSP_nodup : allBSP.Nodup := by decide

/--
  **★P02 全枚举穷尽（L0）** —— 每个可能的信号向量 `u` 都在 `allBSP` 中（`{0,1}^6` 无遗漏）。
  按 6 个 Bool 字段全展开（2^6=64 分支）逐分支 `decide` 成员判定。
-/
theorem allBSP_complete (u : SignalVector) : u ∈ allBSP := by
  obtain ⟨b1, b2, b3, s1, s2, s3⟩ := u
  cases b1 <;> cases b2 <;> cases b3 <;> cases s1 <;> cases s2 <;> cases s3 <;> decide

/--
  **★P02 64 态穷尽 `Σ_u 1[b_ℓ(x)=u] = 1`（L0，单值推论）** —— 对任意级别视图 v，存在**唯一**的
  信号向量 u 使 `signalVector v = u`（`∃!` 的纯 core 展开形式，不用 Mathlib `∃!` 记法）。
  这是 spec P02 的 `Σ_{u∈{0,1}^6} 1[b_ℓ(x)=u]=1`：指示函数在 64 个向量上求和恒为 1
  ⟺ 函数单值（每个状态落入恰好一个向量）。

  ★诚实标注：这是 `signalVector` 作为**函数**的单值性的直接推论（L0，零信息增量）——
  它说「分类穷尽且不重叠」（64 态划分全状态空间），**不**说「64 态都会在真实市场出现」。
-/
theorem signalVector_unique (v : LevelView) :
    ∃ u : SignalVector, signalVector v = u ∧ (∀ w : SignalVector, signalVector v = w → u = w) :=
  ⟨signalVector v, rfl, fun _ h => h⟩

/--
  **指示函数 Σ 的 List 引理（L0，纯 core）** —— 对任意信号向量 w，在全枚举 `allBSP`（64 元）上
  「等于 w」的元素恰好 1 个（`List.filter ... |>.length = 1`）。这是 spec P02
  `Σ_{u} 1[b=u]=1` 的字面 List 形式（指示函数 = `decide (w = u)`，Σ = filter 后 length），
  **不用 `Finset.univ`/`Finset.card`**（纯 core）。按 w 的 6 个 Bool 字段全展开逐分支 `decide`。
-/
theorem allBSP_indicator_length_one (w : SignalVector) :
    (allBSP.filter (fun u => decide (w = u))).length = 1 := by
  obtain ⟨b1, b2, b3, s1, s2, s3⟩ := w
  cases b1 <;> cases b2 <;> cases b3 <;> cases s1 <;> cases s2 <;> cases s3 <;> decide

/--
  **★P02 64 态分类的指示函数和 = 1（L0，显式 Σ 形式，替代 Finset 版）** —— 对任意 v，遍历全部
  64 个信号向量 `allBSP`，「`signalVector v = u`」为真的 u 恰好 1 个。这是 spec P02
  `Σ_{u} 1[b_ℓ(x)=u]=1` 的字面形式（List 版）：在 `allBSP`（64 元）上谓词
  `signalVector v = ·` 的计数为 1。直接由 `allBSP_indicator_length_one` 实例化。
-/
theorem signalVector_indicator_sum_one (v : LevelView) :
    (allBSP.filter (fun u => decide (signalVector v = u))).length = 1 :=
  allBSP_indicator_length_one (signalVector v)

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 买卖点可重合（spec P02/P25）—— 本层不互斥，互斥推迟到角色层
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★P02/P25 买卖点可重合见证（L0）** —— 存在一个级别视图，其第二类买点与第三类买点
  **同时为真**（`B v .i2 = true ∧ B v .i3 = true`）——信号向量同时有 `b2=true` 与 `b3=true`。

  ★这是 spec P25 核心方法论的形式化：「**不要把买卖点直接当互斥类别**」——买卖点可重合
  （maimai.md 第21课「第二类与第三类买点可重合」，V 型反转），重合被编码为 64 态向量中
  `b2=1∧b3=1` 的状态，而非被某个互斥约束排除。复用 `BspClassification.x_2b3b`
  （已证同占二、三类）作为第二、三类同时触发的端点见证。
-/
def coincidentView : LevelView :=
  { cand1 := none
    cand2 := some x_2b3b   -- 满足第二类（afterTypeOne ∧ ¬brokeCenter）
    cand3 := some x_2b3b }  -- 同时满足第三类买点（leftCenter ∧ firstRetrace ∧ retrace > zg）

/--
  ★`coincidence_allowed` 修复 line257/262 根因：原稿用 `rw [decide_eq_true ...]` 后
  `unfold x_2b3b` 留下 metavariable 致 unfold 失败。改用 `decide`——`coincidentView`/`x_2b3b`
  全字段具体（闭项），透明 Decidable 实例（§2）使 `B coincidentView .i2/.i3` 在 kernel 整体归约。
-/
theorem coincidence_allowed :
    B coincidentView .i2 = true ∧ B coincidentView .i3 = true := by
  decide

/--
  **★P02 重合 ⟹ 信号向量同时置位（L0）** —— `coincidentView` 的信号向量满足 `b2 = true ∧ b3 = true`。
  这显式坐实「重合作为不同语法状态」：64 态向量空间能区分「单类触发」与「多类重合」。
-/
theorem coincident_signalVector :
    (signalVector coincidentView).b2 = true ∧ (signalVector coincidentView).b3 = true := by
  decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. P04 接口锚点（区间套终端确认 Conf^±_e = ⋁ B/S，BSP-L2 用）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **买入方向终端确认 `Conf^+_e(x) = ⋁_{i=1}^3 B_{i,e}(x)`（spec P04 接口）** ——
  执行级别 e 的买入终端确认 = 三类买点谓词的析取。这是区间套递归 `N^δ_{ℓ↓e}` 的**终端底**
  （spec P04），由本文件提供，BSP-L2（`IntervalNestCertificate.lean` 扩展）作为递归底消费。
  ★本文件只提供终端确认接口（⋁B），**不**实装区间套递归（那是 BSP-L2）。
-/
def ConfBuy (v : LevelView) : Bool :=
  B v .i1 || B v .i2 || B v .i3

/-- **卖出方向终端确认 `Conf^-_e(x) = ⋁_{i=1}^3 S_{i,e}(x)`（spec P04 对偶）**。 -/
def ConfSell (v : LevelView) : Bool :=
  S v .i1 || S v .i2 || S v .i3

/--
  **★P04 终端确认全定义（L0）** —— `Conf^±_e` 对任意级别视图返回确定 Bool。
  这保证区间套递归底（BSP-L2 用）全定义——递归终端不会卡在未定义状态。
-/
theorem ConfBuy_total (v : LevelView) : ConfBuy v = true ∨ ConfBuy v = false := by
  cases h : ConfBuy v
  · exact Or.inr rfl
  · exact Or.inl rfl

theorem ConfSell_total (v : LevelView) : ConfSell v = true ∨ ConfSell v = false := by
  cases h : ConfSell v
  · exact Or.inr rfl
  · exact Or.inl rfl

/--
  **★P04 终端确认与谓词的析取关系（L0，反退化）** —— `Conf^+_e = true` 当且仅当
  至少一类买点谓词为真。这坐实 `Conf` 是 `⋁B` 而非平凡桩（任一 `B_i` 真 ⟹ Conf 真；全假 ⟹ Conf 假）。
  ★修复 line311「unknown tactic」根因：原稿 `tauto` 是 Mathlib 战术（纯 core 无），改用
  `simp only [Bool.or_eq_true, or_assoc]`（core 引理：Bool 析取展开 + ∨ 结合律）。
-/
theorem ConfBuy_iff_exists (v : LevelView) :
    ConfBuy v = true ↔ (B v .i1 = true ∨ B v .i2 = true ∨ B v .i3 = true) := by
  unfold ConfBuy
  simp only [Bool.or_eq_true, or_assoc]

theorem ConfSell_iff_exists (v : LevelView) :
    ConfSell v = true ↔ (S v .i1 = true ∨ S v .i2 = true ∨ S v .i3 = true) := by
  unfold ConfSell
  simp only [Bool.or_eq_true, or_assoc]

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. still-MISSING 诚实声明 + 边界条件 + 下游推论
    ═══════════════════════════════════════════════════════════════════════

  ★still-MISSING（本文件不冒充的层）：
    - 本文件证「**给定**级别投影 `LevelView` 后，6 谓词全定义因果 + 信号向量 64 态穷尽」。
      它**不**构造投影函数 `状态 x → LevelView`（从 K 线/走势自动识别每级别每类候选端点）——
      那是 rust `signal.rs` 的 extract_signals + `bspOf` 全自动构造层
      （BspClassification still-MISSING-D）。把判据层冒充为构造层 = 声明膨胀（禁止）。
    - **第二类谓词 B_{2}/S_{2} 的有效域诚实标注（GAP-1，spec D.2）**：rust 当前实装仅产
      B1/S1/B3/S3（记忆 `theta-v0-tower-3seg-window-blocks-l0-l1-b2`：B2/S2 只在 L1→L2 几何
      路径产，L0→L1 三段窗口结构上不产，codex 坐实非缺陷）。本文件在**定义层**给出 B_{2}/S_{2}
      全定义谓词（满足 spec P01「6 谓词全定义」），但其**实装侧有效域** < 定义域（rust 暂缺）——
      本 L0 谓词成立不等于 rust 6 谓词全部接通。这是 spec §F' L0 与 rust 实装 L1 的等级差。

  ★边界条件（结论翻转）：
    - 「64 态穷尽」依赖 `signalVector` 是**函数**（单值）。若买卖点谓词改为关系（一个状态映多个
      向量），则 `signalVector_unique`/`signalVector_indicator_sum_one` 翻转——但 spec P01 要求
      谓词「单值」，故当前是函数，不翻转。
    - 6 谓词的内容（`endpointSatisfies` 的判据）直接复用 `BspClassification` 的 `IsType1/2/3`。
      若那里的第三类边界 `>ZG`(`<`严格) 与 reference/rust(`>=`含等号) 的既有冲突
      （记忆 `theta-v0-type3-boundary-reference-lean-conflict`）被裁决改向，则 B_{3}/S_{3} 的
      触发集随之变——但**本文件不引入新冲突**（spec E.3：P01 只要求谓词全定义因果，不取定 >/>=
      边界），既有 type3 冲突仍是待裁决项，不属本工位 escalate 范围。
    - 买卖点可重合（`coincidence_allowed`）依赖 `x_2b3b`（2/3 类可重合见证，BspClassification）。
      若某口径强制 2/3 类不可重合，则重合见证消失——但 maimai.md 第21课 + 第24课支持 2/3 可重合
      （V 型反转），当前不翻转。

  ★下游推论：
    - 6 谓词全定义（B_total/S_total）+ 信号向量 64 态穷尽（signalVector_indicator_sum_one）
      ⟹ BSP-L3 冲突解释器 R_Θ 的候选事件集 `Γ(x)` 有限（每级别至多 6 个谓词触发，级别有限）。
    - 终端确认 `Conf^±_e = ⋁B/S`（ConfBuy/ConfSell）⟹ BSP-L2 区间套递归
      （`IntervalNestCertificate.lean` 扩展）有全定义的递归底——区间套终止性证明可锚到 `Conf` 全定义。
    - 买卖点可重合（coincidence_allowed）⟹ 互斥**不在本层**（BSP-L1）——互斥推迟到角色层
      （BSP-L5 `OperationRole` 4→5 细化），与 spec P25 方法论「互斥发生在角色层」一致。

  ★谱系引用：
    - 买卖点互斥三分失败 + 2/3 类可重合已在 legacy Strict/BSP.lean（谱系 598→603→615）+
      `BspClassification.lean`（`no_exclusive_trichotomy`/`x_2b3b`）形式化。本文件**复用**其
      重合见证，**不重证**——把「可重合」从「互斥分类的反退化」提升为「64 态穷尽的语法状态」。
    - **不确定**是否有「买卖点谓词作为语法基底 + 信号向量 64 态」的更早专属谱系记录
      （spec §E' 273 行建议 genealogist 核 `.chanlun/genealogy/` 确认 P01 是否首次概念分离）——
      本工位不臆造谱系，明确标注此不确定性。
    - 触及 230号谱系（直积退化）：64 态是 `{0,1}^6` 直积，但本层只在**计数/穷尽**意义上用直积
      （`allBSP.length = 64`，List 显式枚举），**未**声明 64 态在概率/净额度量下保持 6 自由度——
      避免 230号膨胀。

  ★影响声明：本文件 reconcile `Origin/BuySellPredicate.lean`（孤儿草稿 → 纯 core，待 Lead 登记 root）。
    删除全部 `Fintype`/`Finset`/`Fintype.card`/`∃!`/`tauto` 用法（违反纯 core 硬约束），改为
    `allBSP : List` 显式枚举 + `List.filter ... |>.length`（指示函数 Σ 的 List 形式）。
    **不改**任何既有 Lean/rust/定义/spec。只读复用 `BspClassification`
    （IsType1/2/3, BspEndpoint, BspClass, x_2b3b, Side）+ `ChanlunElements`（间接）。下游：
    为 BSP-L2（区间套底 Conf）/ BSP-L3（解释器 Γ(x)）/ BSP-L5（角色细化）提供 6 谓词 +
    信号向量 + 64 态穷尽的形式化根。不碰 lakefile/mod.lean。
-/

end NewChanlun.Origin
