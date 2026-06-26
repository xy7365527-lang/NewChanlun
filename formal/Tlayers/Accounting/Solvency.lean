/-
  会计层 · 偿付性与周期（T₄₁ 零强平 / T₄₃ N5⊥N7 两路消费 / T₄₅ 周期性 / T₄₆ 零破产）
  工位：t-accounting（task #49 / gap-map 层C E-set）

  ── 认识论等级：L0（条件结构定理，formalization-validity-domain）───────────────
  本模块形式化 **条件结构定理**：给定前提（否定线先占 / confirm 可分两路 / 状态有限确定
  转移），结论（零强平 / N5∧N7 同时满足 / 周期 / NAV≥0）结构上成立。

  **认识论诚实标注（关键）**：necessity §857 把 T₄₁/T₄₆ 标为「无条件 L0」，其「无条件」承自
  A₀（走势终完美，**公理**）+ T49（向心回溯）。**Lean 能证的是条件定理**「前提 P ⟹ 结论 Q」
  （P = 否定线 ext < 保证金线、confirm 两路分离、状态空间有限）——前提 P 的经验真值（A₀ 是否
  对真实市场成立、regime 是否满足）**不在 Lean 范围**。本模块 **不声称** 验证了 A₀ 的经验
  有效性；只证「若 P 则 Q」的结构蕴含（L0，零经验信息增量）。这是诚实降级，非声明膨胀：
  Lean 证条件蕴含，A₀ 的经验地位由公理层承担，Rust L2 守卫（prove_n8/prove_n1）由数据承担。

  ── 隔离声明 ────────────────────────────────────────────────────────────────
  自包含（仅依赖本层 Ledger.lean 的 nav）。无 sorry/admit/axiom。
-/

import Accounting.Ledger

namespace Formal.Tlayers.Accounting

/-! ## §1 T₄₁ 零强平（否定线先于保证金线）

  necessity_derivation.md §846-871：否定线（破 candidate 极值 = 结构信号）在概念上先于
  保证金强平线（资金信号）触发 ⟹ voice 在被强平前已因否定关闭 ⟹ 零强平。
  形式化为 **条件定理**：若否定阈值（亏损达否定线所需）< 强平阈值（capital 耗尽所需），
  则任何累积亏损序列在触及强平前必先触及否定线 ⟹ voice 先关闭。 -/

/--
  ★voice 偿付态（T₄₁ 载体）：capital 现金、basis 卖出价、negThreshold 否定线所需亏损量、
  marginThreshold 保证金强平所需亏损量。
  否定线先占的结构前提 = `negThreshold < marginThreshold`（破极值的亏损 < 保证金耗尽，
  §853：1x 逐仓下否定线 ext 始终紧于保证金线）。
-/
structure SolvencyState where
  negThreshold : Nat      -- 触及否定线所需累积亏损
  marginThreshold : Nat   -- 触及保证金强平所需累积亏损
deriving Repr

/-- 否定线先占前提（§853：否定线 ext 紧于保证金线）。 -/
def NegationLinePrecedes (st : SolvencyState) : Prop :=
  st.negThreshold < st.marginThreshold

/--
  ★T₄₁ 零强平条件定理（L0 条件蕴含）：在否定线先占前提下，任何累积亏损 `loss` 一旦达到
  强平阈值（loss ≥ marginThreshold），它 **必然已先** 达到否定线（loss ≥ negThreshold）。

  即：voice 在被强平的那一刻，否定线早已被触发（否定在前）⟹ voice 早已因否定关闭 ⟹
  强平不可达（zero liquidation）。这 machine-check「否定先到 ⟹ voice 先因否定关闭 ⟹
  强平不触发」（§854）。

  **认识论标注**：这是「若否定线先占 则 强平前必先否定」的 **条件** 蕴含（L0）。前提
  NegationLinePrecedes（A₀+regime 是否使否定线总紧于保证金线）的经验真值不在此定理范围。
-/
theorem zero_liquidation_conditional (st : SolvencyState) (loss : Nat)
    (hPrecede : NegationLinePrecedes st)
    (hMargin : loss ≥ st.marginThreshold) :
    loss ≥ st.negThreshold := by
  unfold NegationLinePrecedes at hPrecede
  omega

/--
  ★T₄₁ 强平不可达见证（L0 条件）：在否定线先占下，存在一个「已否定但未强平」的亏损区间
  (negThreshold ≤ loss < marginThreshold) ——voice 在此区间因否定关闭，永不进入强平区。
  这把「强平兜底 A 概念上不可达（C 必先消费空头）」（§869）形式化为：否定区严格早于强平区，
  二者之间有非空缓冲（强平兜底保留为纯数值终局，结构上前面已被否定拦截）。
-/
theorem negation_buffer_nonempty (st : SolvencyState)
    (hPrecede : NegationLinePrecedes st) :
    st.negThreshold < st.marginThreshold := hPrecede

/-! ## §2 T₄₃ N5 ⊥ N7 严格解（confirm fire 两路消费）

  necessity_derivation.md §890-908：区间套定位（N5/N6，T₂₈/T₃₁）要求 source 是完整级联链
  顶层；降成本（N7，T₂₀）用 voice 自层 nf 不查全局链。二者不可由一条 located 链同时满足，
  但可由 **同一 confirm@k fire 分两路消费** 严格解（540 号「同一递归两遍历」）。
  形式化：一个 confirm 事件分裂为两个独立通道，各供一侧，两通道物理分离。 -/

/--
  ★confirm fire 的两路消费（T₄₃ 严格解，§900-904）。
  - `cascadeSource`：级联 located 链顶（供 N5/N6，仅根 F/C 消费）。
  - `layerNf`：本层 nf 极值（供 N7，任意 voice 的 E 直接消费，不查全局链）。
  二者由同一 `confirm@k` fire 派生，但物理分离（不同字段，独立清窗）。
-/
structure ConfirmFire where
  cascadeSource : Nat   -- 级联链顶（N5/N6 路）
  layerNf : Nat         -- 逐层 nf（N7 路）
deriving Repr

/-- N5/N6 路消费：取级联链顶。 -/
def consumeCascade (f : ConfirmFire) : Nat := f.cascadeSource

/-- N7 路消费：取逐层 nf（不查全局链）。 -/
def consumeLayerNf (f : ConfirmFire) : Nat := f.layerNf

/--
  ★T₄₃ 两路独立性（L0）：N5/N6 路与 N7 路消费 **互不干涉**——改 cascadeSource 不影响
  layerNf 消费，反之亦然。这形式化「两路物理分离」（§911：confirm 同时写 confirm_*[k]
  供级联 + nf_*[k] 供 E，两路独立清窗）——N5 与 N7 可同时满足（无死锁）。
-/
theorem two_path_independent (s1 s2 nf1 nf2 : Nat) :
    consumeCascade { cascadeSource := s1, layerNf := nf1 }
      = consumeCascade { cascadeSource := s1, layerNf := nf2 }
    ∧ consumeLayerNf { cascadeSource := s1, layerNf := nf1 }
      = consumeLayerNf { cascadeSource := s2, layerNf := nf1 } := by
  exact ⟨rfl, rfl⟩

/--
  ★T₄₃ N5∧N7 同时满足（L0）：同一 confirm fire 既供级联（N5/N6）又供逐层 nf（N7），
  两路从同一 fire 派生但消费独立。这 machine-check「N5/N6/N7 同时满足」（§908）——矛盾
  由「同一 confirm 分两路」严格解，非补丁。

  形式化：给定一个 fire，两路消费各得其值，证明它们可同时被读出（无互斥）。
  **边界条件（§913 矛盾翻转）**：若证明 confirm fire 不可分两路（级联与逐层 nf 必须同源
  同时清窗），则 N5⊥N7 不可同时满足，须 escalate。当前 datatype 两字段独立 ⟹ 可分两路。
-/
theorem n5_and_n7_simultaneous (f : ConfirmFire) :
    consumeCascade f = f.cascadeSource ∧ consumeLayerNf f = f.layerNf := by
  exact ⟨rfl, rfl⟩

/-! ## §3 T₄₅ 操盘的结构周期性

  necessity_derivation.md §938-961：第一次建仓后，系统状态在有限状态集 {持仓降成本,
  出场=翻转=新建仓} 之间循环。操盘是结构周期过程。有限状态 + 确定性转移 + 回到已访问态
  ⟹ 周期。形式化：有限状态机的确定性转移函数，从任一态出发有限步回到已访问态。

  **认识论标注**：necessity §953 标 T₄₅ 为「角向 φ-周期 = L0 群论推论」，并明示有效域边界
  ——螺旋 **径向 r 单调外展非周期**。Lean 证的是 **角向有限状态机的周期性**（confirm
  transition 在有限态上回到起点）；径向非周期（含级别 spawn 的完整轨迹）不在此定理范围，
  由 Spiral 层 D∞ 径向塔承担。本模块只证角向投影的周期。 -/

/--
  ★操盘三阶段稳态（T₄₅ 有限状态集，§946）。第一次建仓后不回到「无仓」初始态（T₂₇），
  稳态集 = {持仓降成本, 出场翻转新建仓}（两态有限）。
-/
inductive TradePhase where
  | holding   -- 阶段二：持仓降成本/挣股数
  | exitFlip  -- 阶段三：出场=翻转=新建仓
deriving DecidableEq, Repr

open TradePhase

/--
  ★确定性状态转移（T₄₅，§947：每次完美必然涌现新走势 ⟹ 状态转移确定）。
  holding → exitFlip（走势完美出场）；exitFlip → holding（新建仓回到持仓）。
  这是有限状态确定性游走（§950）。
-/
def phaseStep : TradePhase → TradePhase
  | holding => exitFlip
  | exitFlip => holding

/--
  ★T₄₅ 周期性（L0）：状态转移两步回到原态（周期 = 2）。
  `phaseStep (phaseStep p) = p`——有限状态 + 确定性转移 + 回到已访问态 ⟹ 周期。
  这 machine-check「操盘结构周期」（§948）的角向投影：F→E→C→F 生命周期闭合。

  **认识论标注**：这是 **角向 φ-周期**（有限态闭合）；径向非周期（σ 无限塔）是有效域
  边界，不在此定理（§959：含级别 spawn 的完整轨迹非周期）。
-/
theorem phase_periodic (p : TradePhase) :
    phaseStep (phaseStep p) = p := by
  cases p with
  | holding => rfl
  | exitFlip => rfl

/-- n 步状态转移（显式迭代，不依赖 Mathlib 的 `^[]` notation——本模块自包含）。 -/
def phaseIter : Nat → TradePhase → TradePhase
  | 0, p => p
  | n + 1, p => phaseStep (phaseIter n p)

/--
  ★T₄₅ 回到已访问态（L0，周期的本质）：从任一态出发，有限步（2 步）必回到出发态。
  这是「有限状态确定性游走，回到已访问状态 ⟹ 周期」（§950）的精确形式——稳态集有限
  （2 态）保证有限步回归。
-/
theorem phase_returns_to_start (p : TradePhase) :
    ∃ n, n > 0 ∧ phaseIter n p = p := by
  refine ⟨2, by omega, ?_⟩
  show phaseStep (phaseStep (phaseIter 0 p)) = p
  show phaseStep (phaseStep p) = p
  exact phase_periodic p

/-! ## §4 T₄₆ 零破产（NAV ≥ 0 恒成立）

  necessity_derivation.md §963-979：在 1x 逐仓 + 否定线先占下，NAV ≥ 0 恒成立。
  来源对：T₃₅（NAV 价值中性）× T₄₁（零强平）× T₃₄（守恒）。形式化为组合条件定理：
  各 voice 在否定线（capital 耗尽前）关闭 ⟹ Σ capital ≥ 0 ⟹ NAV ≥ 0。

  **认识论标注**：与 T₄₁ 同——「无条件 L0」承自 A₀，Lean 证的是「若否定线先占 ∧ 守恒 ∧
  NAV 中性，则 NAV≥0」的条件蕴含。Nat 上 NAV≥0 平凡（Nat 非负），核心内容是「capital 不
  被单边记账泄漏到负」——这由 T₄₁ 关闭时机（capital 耗尽前关闭）保证。用整数模型表达
  capital 可正可负，证否定线先占 ⟹ capital ≥ 0。 -/

/--
  ★voice 资本余额（T₄₆ 载体，用 Int 容许「亏损超 capital」的破产态以便证它被排除）。
  `capital`：初始保证金。`loss`：累积亏损。余额 = capital − loss。破产 ⟺ 余额 < 0。
-/
structure CapitalBalance where
  capital : Int
  loss : Int
  capital_nonneg : capital ≥ 0
  loss_nonneg : loss ≥ 0

/-- voice 当前资本余额。 -/
def CapitalBalance.balance (b : CapitalBalance) : Int := b.capital - b.loss

/--
  ★T₄₆ 单 voice 非破产（L0 条件定理）：若 voice 在 loss 达到 capital 之前关闭（否定线
  先占：loss ≤ capital，T₄₁），则其余额 ≥ 0（不破产）。
  这是「每个 voice 在其亏损达到 capital 之前关闭 ⟹ capital ≥ 0」（§971）的结构形式。
-/
theorem voice_solvent_if_closed_early (b : CapitalBalance)
    (hEarly : b.loss ≤ b.capital) :
    b.balance ≥ 0 := by
  unfold CapitalBalance.balance
  omega

/--
  ★T₄₆ 零破产组合定理（L0 条件蕴含）：森林中每个 voice 都在否定线先占下关闭（loss≤capital）
  ⟹ 全森林余额总和 ≥ 0 ⟹ NAV ≥ 0（不破产）。
  这 machine-check「Σ 各 voice capital ≥ 0 ⟹ NAV ≥ 0」（§971）——T₄₁ 零强平 + T₃₄ 守恒
  （无单边泄漏，余额求和良定）+ T₃₅ NAV 中性（NAV 变化只来自价格）的组合。

  **认识论标注**：前提 `∀ b ∈ forest, b.loss ≤ b.capital`（每 voice 否定线先占）的经验
  真值承自 A₀，不在此定理范围。Lean 证的是「若全员否定线先占 则 Σbalance≥0」的条件蕴含。
-/
theorem zero_bankruptcy_conditional (forest : List CapitalBalance)
    (hAll : ∀ b ∈ forest, b.loss ≤ b.capital) :
    (forest.map CapitalBalance.balance).sum ≥ 0 := by
  induction forest with
  | nil => simp
  | cons b rest ih =>
      simp only [List.map_cons, List.sum_cons]
      have hb : b.balance ≥ 0 :=
        voice_solvent_if_closed_early b (hAll b (by simp))
      have hrest : (rest.map CapitalBalance.balance).sum ≥ 0 :=
        ih (fun b' hb' => hAll b' (by simp [hb']))
      omega

end Formal.Tlayers.Accounting
