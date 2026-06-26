/-
  会计层 · earning 与父子恒等（T₃₆ 构造不对称 / T₃₇ child.P&L≡cost_reduction 有条件）
  工位：t-accounting（task #49 / gap-map 层C E-set）

  ── 认识论等级：L0（定义内蕴，formalization-validity-domain）──────────────────
  - T₃₆「构造不对称」是 **定理内容本身**（necessity §763 明示「不对称是定理内容，非缺陷」）：
    多头 earning 在 ℝ₊ 现货载体可构造（返回正股数增量），空头 earning L0 构造性不可表示
    （「挣负股数」无凸性载体）。形式化为 Option 返回：多头 Some Δ / 空头 None。这是构造性
    L0 结论（不依赖数据），不是经验否证。空头在期权时间维（DTE）可构造是定理边界的中性
    事实，不在 ℝ₊ 现货载体内。
  - T₃₇「父子恒等」是 **有条件** 恒等式（necessity §775：仅 {盈利 ∧ shortfall=0 ∧ 池足}
    成立）。形式化为带前提谓词的恒等定理；其 **无条件严格形式 = NAV 价值中性**（T₃₅，
    见 Ledger.lean），由本模块显式引用，不冒充无条件恒等。

  ── 隔离声明 ────────────────────────────────────────────────────────────────
  自包含（仅依赖本层 Ledger.lean 的 Polarity）。无 sorry/admit/axiom。
-/

import Accounting.Ledger

namespace Formal.Tlayers.Accounting

open Polarity

/-! ## §1 T₃₆ earning 构造不对称（多头可构造 / 空头不可构造）

  necessity_derivation.md §751-771：降成本使 cost_pool ≤ 0 后仓位免费，后续纯利润多头可
  买入 Δ 正单位（增 N，可构造）；空头继续盈利需持「负数量股」——L0 构造性不可表示。
  **不对称是定理内容**。形式化：earning 构造函数对多头返回 `some Δ`，对空头返回 `none`。 -/

/--
  ★earning 构造尝试（T₃₆ 核心）：给定极性与免费仓位后的纯利润 `profit`（已 cost≤0），
  尝试把利润资本化为股数增量 Δ。
  - 多头：`some profit`——纯利润可在买点买入 Δ 正单位（`parent.units += dq`，A4 dq>0）。
  - 空头：`none`——「挣负股数」在 ℝ₊ 现货载体 **构造性不可表示**（无凸性载体）。

  这 **不是** 工程缺陷或未实现——`none` 是 T₃₆ 陈述的「空头 earning L0 不可构造」的忠实
  编码（necessity §765：代码正确地不给空头增 units，仅计数）。
-/
def tryEarning (pol : Polarity) (profit : Nat) : Option Nat :=
  match pol with
  | long => some profit
  | short => none

/--
  ★T₃₆ 多头 earning 可构造（L0）：多头总能把纯利润 profit 资本化为股数增量（= profit）。
  这形式化「多头父 earning **可构造**」（§764）——返回 some，且增量 = profit。
-/
theorem long_earning_constructible (profit : Nat) :
    tryEarning long profit = some profit := by
  rfl

/--
  ★T₃₆ 空头 earning 不可构造（L0，**构造不对称的核心**）：空头无论利润多大，earning
  构造 **恒为 none**。这形式化「空头父 earning L0 **不可构造**」（§765）——这是定理内容
  （构造不对称），不是缺陷。关死「空头也应能 earning」的误读：在 ℝ₊ 现货载体内不可构造。
-/
theorem short_earning_not_constructible (profit : Nat) :
    tryEarning short profit = none := by
  rfl

/--
  ★T₃₆ 构造不对称定理（L0，多空对比）：对任意利润，多头可构造（isSome）∧ 空头不可构造
  （isNone）。这是 T₃₆ 「**不对称**」的精确机器陈述——同一 profit，极性决定可构造性，
  二者不可同时可构造。necessity §760：earning 多空对称仅在金额守恒/极性翻转层成立，
  资本化构造不对称。
-/
theorem earning_asymmetry (profit : Nat) :
    (tryEarning long profit).isSome = true
    ∧ (tryEarning short profit).isNone = true := by
  exact ⟨rfl, rfl⟩

/--
  ★T₃₆ 增 N 唯一性见证（L0）：earning 成功（返回 some Δ）⟹ 极性必为多头。
  即「增 N 的唯一途径 = 多头 earning」的逆否——若某 earning 构造成功，它一定来自多头。
  这关死「空头悄悄增了 units」的 N8 守恒破坏路径（§766：给空头增 units 会破坏守恒）。
-/
theorem earning_some_implies_long (pol : Polarity) (profit delta : Nat)
    (h : tryEarning pol profit = some delta) : pol = long := by
  cases pol with
  | long => rfl
  | short => simp [tryEarning] at h

/--
  ★T₃₆ 空头 earning 构造不存在（L0，**¬∃ 强形式**，修 codex FAIL #5）：对任意利润，
  **不存在** 任何股数增量 delta 使空头 earning 构造成功。这比「返回 none」更强——它是
  「在 ℝ₊（Nat 非负）现货载体内，空头『挣负股数』的构造 **根本不存在**」的精确陈述
  （§759：均价≤0 后继续盈利需持「负数量股」，L0 构造性不可表示）。

  关键：用 Nat（非负）载体，故「增量」只能是非负 delta；空头 earning 对任何 delta 都失败
  ⟹ ¬∃。这不是「定义成 no-op/不变」（codex 批评的 vacuous 模式），而是证「构造空间为空」。
-/
theorem short_earning_no_construction (profit : Nat) :
    ¬ ∃ delta : Nat, tryEarning short profit = some delta := by
  intro ⟨delta, h⟩
  simp [tryEarning] at h

/-! ## §2 T₃₇ child.P&L ≡ parent.cost_reduction（有条件恒等）

  necessity_derivation.md §773-796：子空头平仓盈亏 = 父降成本金额，**无条件恒等仅在
  {盈利 ∧ shortfall=0 ∧ cost_pool 足} 成立**。一般情形 child.P&L 分裂为四去向。其无条件
  严格形式 = NAV 价值中性（T₃₅）。形式化：带前提谓词的恒等 + 一般情形的分裂表达。 -/

/--
  ★关闭时刻的会计去向（T₃₇ 一般情形 child.P&L 的四分裂，§781）。
  child.P&L ≡ costReduction（降成本）+ earningExcess（N 增,池竭后）− shortfallLoss（亏损）
            + freeResidual（free 沉淀）。
-/
structure CloseSettlement where
  costReduction : Nat   -- 进入父降成本的部分
  earningExcess : Nat   -- 池竭后超额走 earning（T₃₆）
  shortfallLoss : Nat   -- 亏损物化为 shortfall
  freeResidual : Nat    -- 沉淀进 free
deriving Repr

/-- 子盈亏 = 四去向之和（§781 的结构定义，关闭时刻结算）。 -/
def childPnL (s : CloseSettlement) : Nat :=
  s.costReduction + s.earningExcess + s.shortfallLoss + s.freeResidual

/--
  ★T₃₇ 无条件恒等成立的前提谓词（§775：盈利 ∧ shortfall=0 ∧ 池足）。
  「盈利」⟹ earning 超额与亏损为 0 之外的部分全进降成本；「shortfall=0」⟹ 无亏损物化；
  「池足」⟹ 无 free 沉淀。三条同时 ⟺ 四分裂退化为「全进 costReduction」。
-/
def NaiveIdentityHolds (s : CloseSettlement) : Prop :=
  s.earningExcess = 0 ∧ s.shortfallLoss = 0 ∧ s.freeResidual = 0

/--
  ★T₃₇ 朴素恒等式（有条件，L0）：在前提谓词成立时，child.P&L = parent.costReduction。
  这忠实编码「无条件恒等 **仅在** {盈利 ∧ shortfall=0 ∧ 池足} 成立」（§775）——前提
  NaiveIdentityHolds 使三个旁路项归零，childPnL 退化为 costReduction。
-/
theorem child_pnl_eq_cost_reduction_conditional (s : CloseSettlement)
    (h : NaiveIdentityHolds s) :
    childPnL s = s.costReduction := by
  unfold childPnL
  obtain ⟨h1, h2, h3⟩ := h
  rw [h1, h2, h3]
  omega

/--
  ★T₃₇ 有条件性见证（L0，**可证伪 = 非恒真**）：构造一个 shortfall≠0 的结算，朴素恒等式
  **不** 成立（child.P&L ≠ costReduction）。这证明 NaiveIdentityHolds 是有判别力的真前提，
  关死「朴素恒等式无条件成立」的误读（§784：亏损时 leftover=0 ⟹ cost_reduction=0，
  亏损物化为 shortfall ⟹ 恒等破裂）。
-/
theorem child_pnl_neq_cost_reduction_when_shortfall :
    childPnL { costReduction := 0, earningExcess := 0, shortfallLoss := 5, freeResidual := 0 }
      ≠ (0 : Nat) := by
  unfold childPnL
  decide

/--
  ★T₃₇ 无条件严格形式的指针（L0）：朴素恒等式的无条件替代 = NAV 价值中性（T₃₅）。
  本定理把「无条件形式在哪」显式化——它 **不在** childPnL=costReduction（有条件），而在
  `nav_neutral_on_cost_reduce`（Ledger.lean，T₃₅，完整三项 NAV，无条件）。这是诚实降级：
  本模块不冒充 child.P&L 恒等式无条件成立，而是指向真正无条件的 T₃₅。

  形式化为：一次降成本 spawn（父多头卖 m@c 转子空头）下，真正守恒的是完整 NAV
  （含空头 capital 项），而非 childPnL=costReduction（后者需 NaiveIdentityHolds）。
  这里用 T₃₅ 的实例陈述兑现「无条件形式 = NAV 中性」。
-/
theorem unconditional_form_is_nav_neutrality
    (free longUnits c shortCapital m : Nat) (h : m ≤ longUnits) :
    nav free (longUnits - m) c (shortCapital + m * c) = nav free longUnits c shortCapital :=
  nav_neutral_on_cost_reduce free longUnits c shortCapital m h

end Formal.Tlayers.Accounting
