/-
  Origin/LedgerReconciliation.lean — 毛净账目对账恒等式（#1050 第一段，ADR 0024 裁定三·形态 B）

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：毛腿收益求和 ≡ 净额逐 bar 求和（账目恒等式）

  权威源（逐字核对）：`docs/formal-chain/买卖点alpha2.pdf`：
  - §6「净额账户下为什么全覆盖不等于盈利」（p5）：
      p_t = (q_{v,t})_{v∈V}，σ_v ∈ {+1,-1}，N_t = Σ_v σ_v q_{v,t}，
      W_{t+1} = W_t + N_t(P_{t+1} - P_t) - C_t。
  - §7「更一般的抵消定理」（p6，逐式）：
      父腿方向 σ∈{+1,-1}、Q>0；子级短差腿 -σ、H≥0；ΔP = P_{t+1} - P_t；
      G_p = σQΔP；G_c = -σHΔP；G_p + G_c = σ(Q-H)ΔP；H=Q ⟹ G_p+G_c = 0；
      H<Q ⟹ N = σ(Q-H)，收益来自剩余净敞口。
  - §11「严格不可能定理」（p21）：无成本时净值递推 W_{t+1} = W_t + N_t ΔP_t 的
    逐 bar 求和是价格 PnL 的唯一来源（本文件的 telescoping 恒等式）。

  本文件证的两段（#1050 票内第一段）：
  1. **无成本时毛腿收益求和 ≡ 净额逐 bar 求和**：每条腿 v 用自己入出场 bar [λ_v,ρ_v)
     （λ_v = 入场 bar、ρ_v = 出场 bar，腿的毛收益 G_v = σ_v q_v (P_{ρ_v} - P_{λ_v})）；
     毛腿求和 Σ_v G_v = 逐 bar 净额收益求和 Σ_{t<T} N_t ΔP_t（telescoping 恒等式）。
     §7 的 G_p+G_c = σ(Q-H)ΔP 是其单 bar 特例，H=Q ⟹ 0 是其双开退化特例。
  2. **成本按腿分配后毛成本求和 ≡ 净成本求和**：每 bar 的净成本 C_t = Σ_v c_{v,t}
     （按腿分配的加性假设，hAlloc）⟹ Σ_v (Σ_t c_{v,t}) = Σ_t C_t（有限求和可换序）。
     无 hAlloc 时该等式不可证（附见证反例）。

  ════════════════════════════════════════════════════════════════════════
  ## 锚定 LeverageCapital 同族（票内指定）

  import Origin.LeverageCapital 复用其 `Side`（long/short，σ_v 方向域）与 `sign`
  （σ_v 的 ±1 编码）。本文件是 LeverageCapital 度量链（名义头寸 → 毛/净敞口）的
  **收益侧对偶**：LeverageCapital 证名义聚合 N≤G；本文件证收益聚合
  Σ_v 毛腿收益 = Σ_t 净额逐 bar 收益。两文件都用 `List` 承载有限声部族。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain 231号，强制标注）

  全部 **L0**（纯代数恒等式，不依赖数据、不依赖价格模型）。本文件**不**证：
  - 任何收益为正 / alpha（那是 L2/L3，PDF p21 鞅不可能定理：无预测假设时任何
    因果策略都不能被纯语法证明盈利）；
  - 成本 C_t ≥ 0 的实盘来源（手续费/滑点/融资是 Θ 层参数，本文件只证求和换序）。
  本文件证的是**账本自洽性**：毛账与净账在「收益」与「成本」两个求和上不分裂——
  对账锁（Rust 测试同款）的形式化根。

  ════════════════════════════════════════════════════════════════════════
  ## 删前件自查（#871 同款验收口径）

  每条带前件的新命题都附「删前件」见证反例：
  - `ledger_identity_no_cost`（前件 hclosed：所有腿在窗口 [0,T] 内闭合）→ 删前件
    见证 `identity_fails_without_closed_legs`：开窗外的腿 T=0 时毛收益 ≠ 0 而净额求和 = 0。
  - `hedged_two_leg_zero`（§7 H=Q 特例）→ 删约束见证 `two_leg_sum_needs_hedge`：
    Q=2,H=1,ΔP=1 时 G_p+G_c = 1 ≠ 0。
  - `remaining_net_nonzero`（前件 H<Q）→ 删前件见证 `remaining_net_needs_h_lt_q`：
    H=Q 时剩余净敞口 = 0。
  - `cost_reconciliation_with_net_cost`（前件 hAlloc：逐 bar 分配一致）→ 删前件
    见证 `cost_reconciliation_needs_allocation`：分配 1 而净成本记 0 时求和不等。
  - 无条件代数命题（`section7_two_leg_sum`/`cost_reconciliation`）无前件可删，
    其「特化即删约束」的退化版由上述见证覆盖。

  ## 公理足迹：#print axioms 零自定义 axiom（依赖仅 propext 等 Lean 核心）。

  依赖方向（单向无环）：LedgerReconciliation → Origin.LeverageCapital → Origin.SourceAxioms。
  验证：`cd formal && lake env lean Origin/LedgerReconciliation.lean`。禁 sorry/admit/axiom。

  谱系：买卖点alpha2.pdf §6/§7（p5–6）→ #1050（对账定理形式化，ADR 0024 裁定三·形态 B）
        → 本文件（毛净账目恒等式，锚 LeverageCapital 同族）。
-/

import Origin.LeverageCapital

namespace NewChanlun.Origin.LedgerReconciliation

open NewChanlun.Origin (Side)
open NewChanlun.Origin.LeverageCapital (sign)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 价格路径 + 单腿账目模型（p5 §6 的 p_t = (q_{v,t})_{v∈V}、σ_v、N_t）

  价格序列 P_t : Nat → Int（整数最小单位，逐 bar）。腿 = 声部的一条持仓：
  方向 σ_v（Side）、数量 q_v（ℕ，>0 时是真实持仓）、入场 bar λ_v、出场 bar ρ_v。
  腿在 bar t 活动 ⟺ λ_v ≤ t < ρ_v（半开区间，对齐元素生命期 I_e = [λ_e, ρ_e)）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★单腿账目 `LedgerLeg`（L0，p5 §6 单声部持仓 × 生命期）：
  - `side`：声部方向 σ_v（Long/Short）。
  - `q`：数量 Q_v（ℕ，>0 时真实持仓；=0 时零腿）。
  - `entry`：入场 bar λ_v（含，腿自此 bar 起吃 ΔP_{λ_v}）。
  - `exit`：出场 bar ρ_v（不含，腿吃 ΔP 到 bar ρ_v-1 为止）。

  ★诚实标注：q 是整数最小单位（对齐 LeverageCapital.notionalMag 口径）；价格 P 另由
  `Nat → Int` 给出（不臆造具体标的）。毛收益 = σ_v q_v (P_{ρ_v} - P_{λ_v})。
-/
structure LedgerLeg where
  side : Side
  q : Nat
  entry : Nat
  exit : Nat
deriving Repr

/-- ★bar 价差 `dP`（p6 §7：ΔP = P_{t+1} - P_t）。 -/
def dP (P : Nat → Int) (t : Nat) : Int := P (t + 1) - P t

/-- ★有符号腿数量 `signedQ`（p5：σ_v q_v；Long → +q，Short → -q）。 -/
def LedgerLeg.signedQ (l : LedgerLeg) : Int := sign l.side * (l.q : Int)

/-- ★腿在 bar t 活动（p5 生命期半开区间：λ_v ≤ t < ρ_v）。 -/
abbrev LedgerLeg.activeAt (l : LedgerLeg) (t : Nat) : Prop := l.entry ≤ t ∧ t < l.exit

/-- ★单腿毛收益 `entryPnl`（p6 §7 的逐腿形式）：G_v = σ_v q_v (P_{ρ_v} - P_{λ_v})。 -/
def LedgerLeg.entryPnl (l : LedgerLeg) (P : Nat → Int) : Int :=
  l.signedQ * (P l.exit - P l.entry)

/-- ★bar t 的净数量 `netQ`（p5：N_t = Σ_v σ_v q_{v,t}；非活动腿贡献 0）。 -/
def netQ (legs : List LedgerLeg) (t : Nat) : Int :=
  (legs.map (fun l => if l.activeAt t then l.signedQ else 0)).sum

/-- ★bar t 的净额收益（p5 净值递推的收益项）：N_t ΔP_t。 -/
def barNetPnl (legs : List LedgerLeg) (P : Nat → Int) (t : Nat) : Int :=
  netQ legs t * dP P t

/-- ★毛腿收益求和（Σ_v G_v）。 -/
def legPnlSum (legs : List LedgerLeg) (P : Nat → Int) : Int :=
  (legs.map (fun l => LedgerLeg.entryPnl l P)).sum

/-- ★净额逐 bar 求和（Σ_{t<T} N_t ΔP_t）。 -/
def barPnlSum (legs : List LedgerLeg) (P : Nat → Int) (T : Nat) : Int :=
  ((List.range T).map (fun t => barNetPnl legs P t)).sum

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 §7 单 bar 抵消定理（p6，逐式 G_p+G_c=σ(Q-H)ΔP；H=Q ⟹ 0）

  先证 PDF §7 的单 bar 形式（毛腿逐字），telescoping 恒等式在 §3 作多 bar 推广。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★父腿毛收益（p6 §7 逐字）：G_p = σQΔP。 -/
def parentLegPnl (side : Side) (Q : Nat) (d : Int) : Int :=
  sign side * (Q : Int) * d

/-- ★子级短差腿毛收益（p6 §7 逐字）：G_c = -σHΔP。 -/
def childLegPnl (side : Side) (H : Nat) (d : Int) : Int :=
  -(sign side) * (H : Int) * d

/-- ★剩余净敞口（p6 §7）：N = σ(Q-H)。 -/
def remainingNet (side : Side) (Q H : Nat) : Int :=
  sign side * ((Q : Int) - (H : Int))

/--
  ★§7 两腿合计（p6 逐式）：G_p + G_c = σ(Q-H)ΔP。
  无条件代数恒等式（Int.mul_sub/sub_mul/neg_mul 重写）——父腿 σQ 与子腿 -σH 的
  符号抵消后只剩净敞口 σ(Q-H) 吃 ΔP。
-/
theorem section7_two_leg_sum (side : Side) (Q H : Nat) (d : Int) :
    parentLegPnl side Q d + childLegPnl side H d =
      sign side * ((Q : Int) - (H : Int)) * d := by
  unfold parentLegPnl childLegPnl
  rw [Int.mul_sub, Int.sub_mul]
  rw [Int.neg_mul, Int.neg_mul]
  rfl

/--
  ★§7 收益来自剩余净敞口（p6 逐句「此时收益不是来自吃到每个元素，而是来自剩余净敞口」）：
  G_p + G_c = N·ΔP，其中 N = σ(Q-H)。
-/
theorem section7_pnl_from_remaining_net (side : Side) (Q H : Nat) (d : Int) :
    parentLegPnl side Q d + childLegPnl side H d = remainingNet side Q H * d := by
  unfold remainingNet
  exact section7_two_leg_sum side Q H d

/--
  ★§7 双开退化（p6 逐式）：H=Q ⟹ G_p + G_c = 0。
  同单位数双开的毛腿求和为零——两腿毛收益互相抵消，坐实「腿级全覆盖成立、净值级收益为零」
  （p5 §6 末行）。这是 telescoping 恒等式 H=Q 的单 bar 特例。
-/
theorem hedged_two_leg_zero (side : Side) (Q : Nat) (d : Int) :
    parentLegPnl side Q d + childLegPnl side Q d = 0 := by
  rw [section7_two_leg_sum]
  have h : (Q : Int) - (Q : Int) = 0 := by omega
  rw [h]
  simp

/--
  ★§7 剩余净方向（p6）：H < Q ⟹ N = σ(Q-H) ≠ 0（仍有净方向，收益来自剩余净敞口）。
-/
theorem remaining_net_nonzero (side : Side) (Q H : Nat) (hHQ : H < Q) :
    remainingNet side Q H ≠ 0 := by
  unfold remainingNet
  have hQH : (Q : Int) - (H : Int) ≠ 0 := by omega
  cases side <;> simp only [sign, Int.one_mul, Int.neg_one_mul] <;> omega

/--
  ★剩余净敞口在 H=Q 时归零（p6 反方向）：N = σ(Q-Q) = 0。
  这是 `remaining_net_nonzero` 前件 H<Q **不可删**的见证（删后 H=Q 即反例）。
-/
theorem remaining_net_zero_when_equal (side : Side) (Q : Nat) :
    remainingNet side Q Q = 0 := by
  unfold remainingNet
  cases side <;> simp only [sign, Int.one_mul, Int.neg_one_mul] <;> omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 ★★telescoping 恒等式（§7 的多 bar 推广，核心产出）

  Σ_v σ_v q_v (P_{ρ_v} - P_{λ_v}) = Σ_{t<T} N_t ΔP_t（无成本）。
  证明路径：有限求和换序（sum_swap）→ 每腿的 bar 指示子求和 telescoping 成
  P_{ρ_v} - P_{λ_v}（telescope_indicator）。全部在 List/Int core 上完成。
  ════════════════════════════════════════════════════════════════════════ -/

/-! 辅助引理：Int 求和换序 / 常因子提取 / 指示子求和（core List 上的自证引理）。 -/

/-- Σ_t (f t + g t) = Σ_t f t + Σ_t g t（求和加法分配）。 -/
theorem list_sum_add {α : Type u} (f g : α → Int) (l : List α) :
    (l.map (fun x => f x + g x)).sum = (l.map f).sum + (l.map g).sum := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons, ih]
      omega

/-- Σ_t 0 = 0（零函数求和）。 -/
theorem list_sum_zero {α : Type u} (l : List α) :
    (l.map (fun _ => (0 : Int))).sum = 0 := by
  induction l with
  | nil => simp
  | cons a l ih => simp [ih]

/-- 逐点为零 ⟹ 求和为零（指示子恒假的推广）。 -/
theorem list_sum_zero_of {α : Type u} (f : α → Int) (l : List α)
    (h : ∀ a ∈ l, f a = 0) : (l.map f).sum = 0 := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons, h a (List.mem_cons_self),
        ih (fun b hb => h b (List.mem_cons_of_mem a hb))]
      omega

/-- (Σ_t f t)·c = Σ_t (f t·c)（右乘分配进求和）。 -/
theorem sum_mul {α : Type u} (f : α → Int) (c : Int) (l : List α) :
    (l.map f).sum * c = (l.map (fun x => f x * c)).sum := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons]
      rw [Int.add_mul, ih]

/-- Σ_t (c·f t) = c·Σ_t f t（左乘分配进求和）。 -/
theorem list_sum_mul {α : Type u} (c : Int) (f : α → Int) (l : List α) :
    (l.map (fun x => c * f x)).sum = c * (l.map f).sum := by
  induction l with
  | nil => simp
  | cons a l ih =>
      simp only [List.map_cons, List.sum_cons, ih]
      rw [Int.mul_add]

/-- Σ_t Σ_v f v t = Σ_v Σ_t f v t（有限求和换序，对账核心）。 -/
theorem sum_swap (legs : List LedgerLeg) (f : LedgerLeg → Nat → Int) (T : Nat) :
    ((List.range T).map (fun t => (legs.map (fun l => f l t)).sum)).sum
    = (legs.map (fun l => ((List.range T).map (fun t => f l t)).sum)).sum := by
  induction legs with
  | nil => simp [list_sum_zero]
  | cons l legs ih =>
      have hmap : ((List.range T).map (fun t => ((l :: legs).map (fun x => f x t)).sum))
          = ((List.range T).map (fun t => f l t + (legs.map (fun x => f x t)).sum)) := by
        apply List.map_congr_left
        intro t _
        simp only [List.map_cons, List.sum_cons]
      rw [hmap, list_sum_add]
      simp only [List.map_cons, List.sum_cons, ih]

/-- Σ_{t<T} [t=n]·f t = f n（n<T 时指示子求和退化为单点）。 -/
theorem sum_at_point (T n : Nat) (f : Nat → Int) (hnT : n < T) :
    ((List.range T).map (fun t => if t = n then f t else 0)).sum = f n := by
  induction T with
  | zero => omega
  | succ m ih =>
      rw [List.range_succ, List.map_append, List.sum_append]
      by_cases h : n = m
      · rw [h]
        have hzero : ((List.range m).map (fun t => if t = m then f t else 0)).sum = 0 := by
          refine list_sum_zero_of _ _ ?_
          intro t ht
          have ht' : t < m := (List.mem_range.mp ht)
          have htne : ¬ (t = m) := by omega
          simp only [if_neg htne]
        rw [hzero]
        change 0 + ((if m = m then f m else 0) + 0) = f m
        rw [if_pos rfl]
        omega
      · have hnT' : n < m := by omega
        have hih := ih hnT'
        rw [hih]
        have hTn : ¬ (m = n) := by omega
        simp only [List.map_cons, List.map_nil, List.sum_cons, List.sum_nil]
        rw [if_neg hTn]
        omega

/-- 指示子拆分 [λ≤t<n+1] = [λ≤t<n] + [t=n]（λ≤n 时逐点成立）。 -/
theorem indicator_split_add (la n t : Nat) (X : Int) (hln : la ≤ n) :
    (if la ≤ t ∧ t < n + 1 then X else 0)
    = (if la ≤ t ∧ t < n then X else 0) + (if t = n then X else 0) := by
  by_cases h1 : la ≤ t ∧ t < n + 1
  · by_cases h2 : la ≤ t ∧ t < n
    · have hne : ¬ (t = n) := by omega
      simp only [if_pos h1, if_pos h2, if_neg hne]
      omega
    · have htn : t = n := by omega
      have hneg2 : ¬ (la ≤ t ∧ t < n) := by omega
      simp only [if_pos h1, if_neg hneg2, if_pos htn]
      omega
  · have hneg2 : ¬ (la ≤ t ∧ t < n) := by omega
    have hne : ¬ (t = n) := by omega
    simp only [if_neg h1, if_neg hneg2, if_neg hne]
    omega

/--
  ★telescoping 引理（指示子版）：Σ_{t<T} [λ≤t<λ+d]·ΔP_t = P_{λ+d} - P_λ（λ+d ≤ T）。
  半开区间上的价差求和 telescoping 成端点价差——逐 bar 收益与腿收益的桥梁。
-/
theorem telescope_indicator (P : Nat → Int) (la d T : Nat) (h : la + d ≤ T) :
    ((List.range T).map (fun t => if la ≤ t ∧ t < la + d then dP P t else 0)).sum
    = P (la + d) - P la := by
  induction d generalizing T with
  | zero =>
      have hzero : ((List.range T).map (fun t => if la ≤ t ∧ t < la + 0 then dP P t else 0)).sum
          = 0 := by
        refine list_sum_zero_of _ _ ?_
        intro t ht
        by_cases hc : la ≤ t ∧ t < la + 0
        · omega
        · simp only [if_neg hc]
      rw [hzero]
      simp only [Nat.add_zero]
      omega
  | succ d ih =>
      have hsucc : la + (d + 1) = (la + d) + 1 := by omega
      have hmap : ((List.range T).map (fun t => if la ≤ t ∧ t < la + (d + 1) then dP P t else 0))
          = ((List.range T).map (fun t =>
              (if la ≤ t ∧ t < la + d then dP P t else 0)
              + (if t = la + d then dP P t else 0))) := by
        apply List.map_congr_left
        intro t _
        have hs := indicator_split_add la (la + d) t (dP P t) (by omega)
        simpa [hsucc] using hs
      rw [hmap, list_sum_add]
      have hih : ((List.range T).map (fun t => if la ≤ t ∧ t < la + d then dP P t else 0)).sum
          = P (la + d) - P la := ih T (by omega)
      rw [hih]
      have hpoint := sum_at_point T (la + d) (fun t => dP P t) (by omega)
      rw [hpoint]
      rw [hsucc]
      unfold dP
      omega

/--
  ★单腿 bar 指示子求和 = 腿毛收益（telescoping 的单腿形式）：
  Σ_{t<T} [λ≤t<ρ]·ΔP_t = P_ρ - P_λ（λ≤ρ≤T）。
-/
theorem leg_telescope (l : LedgerLeg) (P : Nat → Int) (T : Nat)
    (hle : l.entry ≤ l.exit) (hρT : l.exit ≤ T) :
    ((List.range T).map (fun t => if l.entry ≤ t ∧ t < l.exit then dP P t else 0)).sum
    = P l.exit - P l.entry := by
  have hd : l.entry + (l.exit - l.entry) = l.exit := Nat.add_sub_of_le hle
  have hdT : l.entry + (l.exit - l.entry) ≤ T := by simpa [hd] using hρT
  have htel := telescope_indicator P l.entry (l.exit - l.entry) T hdT
  simpa [hd] using htel

/-- (if c then a else 0)·b = a·(if c then b else 0)（指示子与常因子换位）。 -/
theorem if_zero_mul (c : Prop) [Decidable c] (a b : Int) :
    (if c then a else 0) * b = a * (if c then b else 0) := by
  by_cases h : c <;> simp [h]

/--
  ★单腿贡献引理（telescoping × 常因子提取）：
  Σ_{t<T} [腿活动]·σ_v q_v·ΔP_t = σ_v q_v (P_{ρ_v} - P_{λ_v}) = 腿毛收益。
-/
theorem leg_bar_contribution (l : LedgerLeg) (P : Nat → Int) (T : Nat)
    (hle : l.entry ≤ l.exit) (hρT : l.exit ≤ T) :
    ((List.range T).map (fun t => (if l.activeAt t then l.signedQ else 0) * dP P t)).sum
    = LedgerLeg.entryPnl l P := by
  unfold LedgerLeg.entryPnl
  have hmul : ((List.range T).map (fun t => (if l.activeAt t then l.signedQ else 0) * dP P t))
      = ((List.range T).map (fun t => l.signedQ * (if l.activeAt t then dP P t else 0))) := by
    apply List.map_congr_left
    intro t _
    exact if_zero_mul (l.activeAt t) l.signedQ (dP P t)
  rw [hmul, list_sum_mul]
  have ht := leg_telescope l P T hle hρT
  unfold LedgerLeg.activeAt at ht ⊢
  rw [ht]

/--
  ★★毛净账目对账恒等式（#1050 第一段核心，L0）：
  所有腿在窗口 [0,T] 内闭合（λ_v ≤ ρ_v ≤ T）时，无成本毛腿收益求和 ≡ 净额逐 bar 求和：
      Σ_v σ_v q_v (P_{ρ_v} - P_{λ_v}) = Σ_{t<T} N_t ΔP_t。
  这是 PDF §7 G_p+G_c=σ(Q-H)ΔP 的 telescoping 推广（§7 单 bar 是 T=1 特例，见 §2）；
  H=Q ⟹ 0 是双开退化特例。删前件（hclosed）后结论不可证，见证见 §5。
-/
theorem ledger_identity_no_cost (legs : List LedgerLeg) (P : Nat → Int) (T : Nat)
    (hclosed : ∀ l ∈ legs, l.entry ≤ l.exit ∧ l.exit ≤ T) :
    legPnlSum legs P = barPnlSum legs P T := by
  unfold legPnlSum barPnlSum barNetPnl
  have hstep1 : ((List.range T).map (fun t => netQ legs t * dP P t))
      = ((List.range T).map (fun t =>
          (legs.map (fun l => (if l.activeAt t then l.signedQ else 0) * dP P t)).sum)) := by
    apply List.map_congr_left
    intro t _
    unfold netQ
    exact sum_mul (fun l => if l.activeAt t then l.signedQ else 0) (dP P t) legs
  rw [hstep1]
  rw [sum_swap legs (fun l t => (if l.activeAt t then l.signedQ else 0) * dP P t) T]
  apply congrArg (fun (l : List Int) => l.sum)
  apply List.map_congr_left
  intro l hl
  have hleg := hclosed l hl
  exact (leg_bar_contribution l P T hleg.1 hleg.2).symm

/--
  ★★双开腿对在窗口内毛收益求和为零（§7 H=Q ⟹ 0 的多 bar telescoping 特例）：
  同窗口同数量的一多一空两腿（父 σQ + 子 -σQ），毛腿收益逐 bar 抵消，合计 0。
  这是 p5 §6「H=Q ⟹ N=0 ⟹ NΔP=0」在整生命期上的形式坐实。
-/
theorem hedged_pair_pnl_zero (side : Side) (Q : Nat) (P : Nat → Int) (entry exit : Nat) :
    legPnlSum [{ side := side, q := Q, entry := entry, exit := exit },
               { side := (match side with | Side.long => Side.short | Side.short => Side.long),
                 q := Q, entry := entry, exit := exit }] P = 0 := by
  unfold legPnlSum
  unfold LedgerLeg.entryPnl LedgerLeg.signedQ
  cases side <;>
    simp only [List.map_cons, List.map_nil, List.sum_cons, List.sum_nil, sign,
      Int.one_mul, Int.neg_mul] <;> omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 成本按腿分配：毛成本求和 ≡ 净成本求和（#1050 第一段第二式）

  p5 §6：W_{t+1} = W_t + N_t ΔP_t - C_t，C_t 是手续费/滑点/融资/借券/资金费。
  成本按腿分配：c_{v,t} = 腿 v 在 bar t 分到的成本；逐 bar 净成本
  C_t = Σ_v c_{v,t}（hAlloc）。毛成本求和（先按腿聚合 Σ_t c_{v,t} 再求和 Σ_v）
  与净成本求和（先按 bar 聚合 Σ_v c_{v,t} 再求和 Σ_t）相等 = 有限求和换序。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★腿 v 的全期成本（Σ_t c_{v,t}）。 -/
def legCostSum (c : LedgerLeg → Nat → Int) (l : LedgerLeg) (T : Nat) : Int :=
  ((List.range T).map (fun t => c l t)).sum

/-- ★毛成本求和（Σ_v Σ_t c_{v,t}）。 -/
def grossCostSum (legs : List LedgerLeg) (c : LedgerLeg → Nat → Int) (T : Nat) : Int :=
  (legs.map (fun l => legCostSum c l T)).sum

/-- ★净成本求和（Σ_t Σ_v c_{v,t}）。 -/
def netCostSum (legs : List LedgerLeg) (c : LedgerLeg → Nat → Int) (T : Nat) : Int :=
  ((List.range T).map (fun t => (legs.map (fun l => c l t)).sum)).sum

/--
  ★★成本对账（无条件求和换序）：毛成本求和 ≡ 净成本求和。
  任意分配矩阵 c 都成立（有限双求和的换序性）——「按腿分配」的聚合口径不产生账差。
-/
theorem cost_reconciliation (legs : List LedgerLeg) (c : LedgerLeg → Nat → Int) (T : Nat) :
    grossCostSum legs c T = netCostSum legs c T := by
  unfold grossCostSum netCostSum legCostSum
  exact (sum_swap legs c T).symm

/--
  ★★成本对账（带逐 bar 净成本 C_t 的分配一致形式）：
  分配一致（hAlloc：每 bar 净成本 C_t = Σ_v c_{v,t}）⟹
  毛成本求和 = Σ_t C_t = 净值递推 W_{t+1}=W_t+N_tΔP_t-C_t 里的成本总扣除。
  hAlloc 是本命题的唯一前件——删掉后结论不可证（见证 §5）。
-/
theorem cost_reconciliation_with_net_cost (legs : List LedgerLeg) (c : LedgerLeg → Nat → Int)
    (C : Nat → Int) (T : Nat)
    (hAlloc : ∀ t < T, (legs.map (fun l => c l t)).sum = C t) :
    grossCostSum legs c T = ((List.range T).map C).sum := by
  rw [cost_reconciliation legs c T]
  unfold netCostSum
  apply congrArg (fun (l : List Int) => l.sum)
  apply List.map_congr_left
  intro t ht
  exact hAlloc t (List.mem_range.mp ht)

/--
  ★★全对账（收益 + 成本，无成本恒等式的自然推广）：
  腿闭合 + 成本分配一致 ⟹ 毛账净额（毛腿收益 − 毛成本）≡ 净账净额
  （逐 bar 净收益 − 逐 bar 净成本）。毛账与净账在净额层不分裂。
-/
theorem ledger_identity_with_cost (legs : List LedgerLeg) (P : Nat → Int)
    (c : LedgerLeg → Nat → Int) (C : Nat → Int) (T : Nat)
    (hclosed : ∀ l ∈ legs, l.entry ≤ l.exit ∧ l.exit ≤ T)
    (hAlloc : ∀ t < T, (legs.map (fun l => c l t)).sum = C t) :
    legPnlSum legs P - grossCostSum legs c T
    = barPnlSum legs P T - ((List.range T).map C).sum := by
  rw [ledger_identity_no_cost legs P T hclosed,
      cost_reconciliation_with_net_cost legs c C T hAlloc]

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 ★★删前件自查（#871 同款验收口径：每条带前件命题删前件后结论不可证，附见证）

  四个见证反例，分别坐实 §2–§4 四个命题的前件不可删。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★见证①（`ledger_identity_no_cost` 删前件 hclosed 不可证）：
  腿开在窗口外（exit=1 > T=0）：毛腿收益 = P_1 - P_0 = 1 ≠ 0，而逐 bar 求和 Σ_{t<0} = 0。
  删掉「腿在窗口内闭合」前件后恒等式即被显式证伪。
-/
example : ¬ (∀ (legs : List LedgerLeg) (P : Nat → Int) (T : Nat),
    legPnlSum legs P = barPnlSum legs P T) := by
  intro h
  have hc : (1 : Int) = 0 := by
    have := h [{ side := Side.long, q := 1, entry := 0, exit := 1 }]
      (fun t => (t : Int)) 0
    unfold legPnlSum barPnlSum at this
    simp only [List.range_zero, List.map_nil, List.sum_nil, List.map_cons, List.sum_cons] at this
    unfold LedgerLeg.entryPnl LedgerLeg.signedQ at this
    simp only [sign] at this
    omega
  omega

/--
  ★见证②（§7 双开退化删约束 H=Q 不可证）：
  父 σ=long、Q=2，子 -σ、H=1，ΔP=1：G_p+G_c = 2 + (-1) = 1 ≠ 0。
  不约束 H=Q 时两腿毛收益求和不必为零——抵消定理需要「同单位数」前件。
-/
example : ¬ (∀ (side : Side) (Q H : Nat) (d : Int),
    parentLegPnl side Q d + childLegPnl side H d = 0) := by
  intro h
  have hc : (1 : Int) = 0 := by
    have := h Side.long 2 1 1
    unfold parentLegPnl childLegPnl at this
    simp only [sign, Int.one_mul, Int.neg_one_mul] at this
    omega
  omega

/--
  ★见证③（`remaining_net_nonzero` 删前件 H<Q 不可证）：
  H=Q 时剩余净敞口 N = σ(Q-Q) = 0（`remaining_net_zero_when_equal`），
  故「剩余净方向非零」没有 H<Q 前件即被证伪。
-/
example : ¬ (∀ (side : Side) (Q H : Nat), remainingNet side Q H ≠ 0) := by
  intro h
  exact (h Side.long 1 1) (remaining_net_zero_when_equal Side.long 1)

/--
  ★见证④（`cost_reconciliation_with_net_cost` 删前件 hAlloc 不可证）：
  单腿、单 bar，按腿分配 c = 1 而净成本记账 C_0 = 0（分配不一致）：
  毛成本求和 = 1 ≠ 0 = Σ_t C_t。没有「逐 bar 分配一致」前件，成本对账即被显式证伪。
-/
example : ¬ (∀ (legs : List LedgerLeg) (c : LedgerLeg → Nat → Int) (C : Nat → Int) (T : Nat),
    grossCostSum legs c T = ((List.range T).map C).sum) := by
  intro h
  have hc : (1 : Int) = 0 := by
    have := h [{ side := Side.long, q := 1, entry := 0, exit := 1 }]
      (fun _ _ => 1) (fun _ => 0) 1
    unfold grossCostSum legCostSum at this
    simp only [List.range_succ, List.range_zero, List.map_append, List.map_nil, List.sum_append,
      List.sum_nil, List.map_cons, List.sum_cons] at this
    omega
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（formalization-validity-domain gatekeeper）

  对账恒等式是**账本自洽性**（operational semantics），不是分类/盈利定理。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★对账标签 `ReconciliationTag`（gatekeeper，诚实分层）。
  OperationalSemanticsOnly（毛净账目聚合是操作语义）+ ThetaLeverageParametric
  （价格/成本是 Θ 层输入）。**没有** `TrueCompleteClassification` / `AlphaPositive`
  构造子——类型层拒绝把账目对账标为分类定理或盈利声明。
-/
inductive ReconciliationTag where
  | OperationalSemanticsOnly
  | ThetaLeverageParametric
deriving DecidableEq, Repr

/-- ★对账子类：LedgerReconciliationIdentity（唯一子类）。 -/
inductive ReconciliationSubkind where
  | LedgerReconciliationIdentity
deriving DecidableEq, Repr

/-- ★对账诚实标签包（L0 声明）。 -/
def reconciliationLabels : List ReconciliationTag × ReconciliationSubkind :=
  ([ReconciliationTag.OperationalSemanticsOnly, ReconciliationTag.ThetaLeverageParametric],
   ReconciliationSubkind.LedgerReconciliationIdentity)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：对账子类必是 LedgerReconciliationIdentity。 -/
theorem reconciliation_not_true_classification (k : ReconciliationSubkind) :
    k = ReconciliationSubkind.LedgerReconciliationIdentity := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（#1050 第一段）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. §7 单 bar 抵消：`section7_two_leg_sum`（G_p+G_c=σ(Q-H)ΔP，p6 逐式）、
     `section7_pnl_from_remaining_net`（收益来自剩余净敞口 N=σ(Q-H)）、
     `hedged_two_leg_zero`（H=Q ⟹ 0）、`remaining_net_nonzero`（H<Q ⟹ N≠0）。
  2. ★★telescoping 恒等式 `ledger_identity_no_cost`：无成本时毛腿收益求和 ≡
     净额逐 bar 求和（§7 的多 bar 推广）；`hedged_pair_pnl_zero`（H=Q 特例的多 bar 形式）。
  3. ★★成本对账 `cost_reconciliation` / `cost_reconciliation_with_net_cost`：
     成本按腿分配后毛成本求和 ≡ 净成本求和（求和换序 + hAlloc）。
  4. ★★全对账 `ledger_identity_with_cost`：毛账净额（毛收益−毛成本）≡ 净账净额。
  5. 删前件自查 4 见证（§5）+ 诚实标签（§6）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 任何收益为正 / alpha / Sharpe>0（PDF p21 鞅不可能定理：需预测性假设，L2/L3）。
  - ✗ 成本参数的经验标定（Θ_leverage 层，本文件只证求和换序）。
  - ✗ 期望层关系（#1050 第二段：毛分解 τ_γ 与净分解逐 bar 的条件信息差 + 定理 2
    细分类不劣于粗分类）——留给独立文件/后续实装。

  Rust 对账测试锁：rust/src/theta_v0/strategy/coverage/leg_tests.rs 的
  `ledger_reconciliation_*` 测试以本文件恒等式为契约锚（毛腿收益求和 vs 净额逐 bar 求和）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.LedgerReconciliation
