/-
  payoff 层完全分类 = 成本阶段状态机（第31课）+ payoff 系数 c=C(...,L_confirm) 结构侧
  ★603 递归范式 reform：把 payoff 从「纯连续 L3 净收益（codex D4 立场，不做真完全分类）」
    重铸为「有限状态构造子穷尽 → datatype 真完全分类（L0）」+ 显式分离的连续净收益数值侧（L2/L3）。

  ═══════════════════════════════════════════════════════════════════════════
  编排者覆盖 codex D4（task message）：codex 判 payoff=纯 L3 不做真完全分类；编排者否定——
  「不是真完全分类的全部要 reform 成真完全分类」。本文件兑现此覆盖：payoff 的 **结构侧**
  （成本阶段 / 操作数量约束 / c 的自变量结构）是 datatype 真完全分类（L0，对构造子结构归纳）；
  仅 **纯连续净收益数值** payoffValue 是不可 datatype 化的 L2/L3，显式 opaque 与结构侧分开。
  ═══════════════════════════════════════════════════════════════════════════

  缠论定理（已结算·第31课原文）：
  - 成本阶段（031-第31课.md:24,26,28,30 + 注 36）三态穷尽：
    「当成本为0以前，要把成本变为0；当成本变成0以后，就要挣股票，直到股票见到历史性大顶。」
    ⟹ 三构造子 {降成本(cost>0), 归零(cost=0 边界), 挣股票(cost=0 持续)}。降成本与挣股票之间
       由「股票翻倍后出掉部分把成本降为0」的 **归零事件** 分隔（:28「找一个大级别的卖点出掉部分，
       把成本降为0」）。归零是阶段转移的不可逆边界（成本单调不增，:24「成本才可能真的降下来」）。
  - 操作数量约束（031:24,28 + 注 36/169）依成本阶段：
    · 成本>0（降成本阶段）：短差 **买入=卖出，仓位不增加**（:24「每次短差，一定不能增加股票的
      数量」；注169「成本为0前，只补进相同的数量，仓位不增加」）。
    · 成本=0（挣股票阶段）：**卖出后全额回补，股数增加**（:28「上面抛了以后，都全部回补，
      这样股票就越来越多，而成本还是0」；注169「成本为0后…全补进去，买回来的数量一定多了」）。
  - 终止：超大级别卖点（月线以上卖点=历史性大顶）一次性清仓（:28,30）。

  ★codex 019eff17（session 019eff17-74b7-7502-b17d-2fe792c20b00, gpt-5.5 xhigh）§4A：
    payoff 系数 c=C(走势配置, 中枢列表, 当前级别, L_confirm)，**c 非独立于 L_confirm**——
    实时操作语义里 L_confirm 参与「哪些中枢算已成立」。payoff 是单变量 payoff(c) 不推出
    c 独立于 L_confirm，而是 payoff(C(..., L_confirm))。本文件 §5 形式化此复合结构（结构侧 L0）。

  范式定位（603）：
  - 成本阶段状态机 = 有限状态机 = inductive 构造子穷尽 ⟹ **datatype 真完全分类**（对构造子结构归纳）。
  - 操作数量约束 = 由成本阶段状态读出的 **离散判定**（结构侧 L0），非连续净收益。
  - c 的自变量结构 = 复合函数 C(...) 的输入字段穷尽（结构侧 L0）。
  - 纯连续净收益数值 payoffValue = 显式 opaque（L2/L3，regime 依赖，数据阻塞），与结构侧分开。

  认识论分层（formalization-validity-domain）：
  - L0（本文件正文，by construction）：成本阶段三态穷尽 / 操作数量约束依阶段 / c 自变量复合结构。
  - L2/L3（本文件 §6 显式 opaque + 注记）：哪个 case 净收益正、具体配额数值、regime 是否溶解。
-/

import Formal.RecursiveConstruction
import Formal.BSPLabels

namespace Formal.Tlayers.Payoff

open Formal.TrendTrichotomy (Direction)
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.RecursiveConstruction (Move classifyMove)
open Formal.BSPLabels (BSPLabelSet)

/-! ## §1 成本阶段状态机（第31课，有限状态构造子穷尽 → datatype 真完全分类）

  第31课资金管理三态：降成本（cost>0，机动短差）→ 归零（cost=0 边界，翻倍出部分收回本金）
  → 挣股票（cost=0 持续，卖后全回补股数增）。这是 **有限状态机**——三个状态穷尽，
  无第四态（成本只有「>0」「=0」两种本体，归零是它们之间的不可逆转移边界）。
-/

/--
  ★成本阶段（第31课，三构造子穷尽，L0）。

  - `reducing`：降成本阶段（成本 > 0）。机动资金短差，**仓位不增**（:24）。
  - `zeroing`：归零事件（成本 = 0 的达成边界）。翻倍出部分把成本降为 0（:28），收回全部本金。
  - `harvesting`：挣股票阶段（成本 = 0 持续）。卖后全回补，**股数增**（:28）。

  穷尽性根据（codex 019efff7 Q1 限定有效域）：本 datatype 分类的对象是 **活跃持仓的成本演化态**。
  成本相对本金只有「> 0」与「= 0」两种本体状态；归零是从前者到后者的不可逆转移（成本单调不增）。
  三态覆盖「活跃持仓的成本演化」的全部——降成本（趋近 0）/归零（达到 0）/挣股票（保持 0）。
  无第四 **成本态**（成本不会再变正——一旦归零，挣股票只增股数不动成本）。

  ★有效域边界（codex Q1）：「历史大顶清仓」（月线以上卖点，:28,30）是 **持仓终止态**，不是
  成本态——它结束整个持仓生命周期，不在本「成本演化」分类的论域内。本 datatype 是
  **活跃持仓成本管理** 的完全分类，不是「持仓生命周期」（建仓→活跃→清仓）的完全分类。
-/
inductive CostStage where
  | reducing    -- 成本 > 0：降成本，短差仓位不增
  | zeroing     -- 成本 = 0 达成边界：翻倍出部分，收回本金
  | harvesting  -- 成本 = 0 持续：挣股票，卖后全回补，股数增
deriving DecidableEq, Repr

namespace CostStage

/--
  ★成本阶段完全分类（L0，构造子穷尽）：任意成本阶段必属三构造子之一，无第四阶段。
  这是 datatype 的全函数判定完备——对构造子结构归纳，by construction。
-/
theorem stage_total (s : CostStage) :
    s = reducing ∨ s = zeroing ∨ s = harvesting := by
  cases s
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/--
  ★成本是否为 0（结构侧 L0）：reducing 成本 > 0（false），zeroing/harvesting 成本 = 0（true）。
  这把「成本本体二值（>0 / =0）」从三态状态机读出——归零是二值翻转的转移边界。
-/
def costIsZero : CostStage → Bool
  | reducing => false
  | zeroing => true
  | harvesting => true

/--
  ★阶段转移（第31课，单向不可逆，L0）：reducing → zeroing → harvesting → harvesting。
  成本单调不增（:24「成本才可能真的降下来」）⟹ 归零后不回降成本阶段；挣股票是吸收态
  （成本永远为 0，:28「成本永远为0」）。
-/
def nextStage : CostStage → CostStage
  | reducing => zeroing
  | zeroing => harvesting
  | harvesting => harvesting

/-- ★挣股票是吸收态（L0）：harvesting 转移到自身——成本永远为 0，不再回到正成本。 -/
theorem harvesting_absorbing : nextStage harvesting = harvesting := rfl

/-- ★归零不可逆（L0）：从 reducing 经 zeroing 后，成本恒为 0（不回 reducing）。 -/
theorem zeroing_irreversible :
    costIsZero (nextStage (nextStage reducing)) = true := rfl

/--
  ★成本单调不增（L0，第31课核心不变量）：转移后「成本为 0」单调不减——
  一旦 costIsZero 为 true，转移后仍为 true。形式化成本不可逆下降（归零后不回正）。
-/
theorem costZero_monotone (s : CostStage) (h : costIsZero s = true) :
    costIsZero (nextStage s) = true := by
  cases s <;> simp_all [costIsZero, nextStage]

end CostStage

/-! ## §2 操作数量约束依成本阶段（第31课硬约束，结构侧 L0）

  仓位数量变化由成本阶段 **离散决定**（不是连续净收益）：
  - reducing：买入=卖出，仓位不增（:24, 注169）。
  - harvesting：卖后全回补，股数增（:28, 注169）。
  这是 603 §诚实分层（codex Q2）说的「仓位数量依成本阶段状态，非仅由 BSP 标签决定」的
  **结构侧形式化**——数量变化的 **符号/方向** 由阶段钉死（L0），具体 **数值** 才是 L2/L3。
-/

/--
  ★短差一回合的数量效应（第31课，结构侧 L0）。
  一个完整短差回合 = 卖 `sold` 股 + 回补 `bought` 股。第31课约束：
  - reducing 阶段：bought = sold（仓位不增，:24「每次短差，一定不能增加股票的数量」）。
  - harvesting 阶段：bought ≥ sold（卖后全额回补，金额回补 ⟹ 价跌时买回股数 ≥ 卖出，:28）。
-/
structure SwingRound where
  sold : Nat
  bought : Nat

/--
  ★短差回合合法（依成本阶段，第31课硬约束，L0）。
  - reducing：bought = sold（NetSharesDelta = 0，仓位不增）。
  - zeroing：归零事件本身是「出掉部分」（sold > 0, bought = 0，净减仓收回本金，:28）。
  - harvesting：bought ≥ sold（NetSharesDelta ≥ 0，股数不减，挣股票，:28）。
-/
def SwingRound.legalAt (r : SwingRound) : CostStage → Prop
  | CostStage.reducing => r.bought = r.sold
  | CostStage.zeroing => r.bought = 0 ∧ r.sold > 0
  | CostStage.harvesting => r.bought ≥ r.sold

/-- 短差回合净股数变化（bought − sold，Int 以容纳负）。 -/
def SwingRound.netSharesDelta (r : SwingRound) : Int :=
  (r.bought : Int) - (r.sold : Int)

/--
  ★降成本阶段仓位不增（L0，第31课:24）：reducing 合法回合的净股数变化 = 0。
  这是「成本为0前，只补进相同的数量，仓位不增加」（注169）的形式化——构造性钉死，非经验。
-/
theorem reducing_no_position_increase (r : SwingRound)
    (h : r.legalAt CostStage.reducing) :
    r.netSharesDelta = 0 := by
  unfold SwingRound.legalAt at h
  unfold SwingRound.netSharesDelta
  omega

/--
  ★挣股票阶段股数不减（L0，第31课:28）：harvesting 合法回合的净股数变化 ≥ 0。
  「上面抛了以后，都全部回补，这样股票就越来越多」——回补股数 ≥ 卖出股数。
-/
theorem harvesting_shares_nondecreasing (r : SwingRound)
    (h : r.legalAt CostStage.harvesting) :
    r.netSharesDelta ≥ 0 := by
  unfold SwingRound.legalAt at h
  unfold SwingRound.netSharesDelta
  omega

/--
  ★归零阶段严格净减仓（L0，第31课:28，codex 019efff7 Q2 收紧）：zeroing 合法回合
  （bought=0 ∧ sold>0）净股数变化 **< 0**（严格减，非仅 ≤0）。归零=「出掉部分把成本降为0」
  =只卖不补（sold>0），收回本金。codex Q2 指出原 `bought=0` 未含 `sold>0`，已补强为严格。
-/
theorem zeroing_net_reduce (r : SwingRound)
    (h : r.legalAt CostStage.zeroing) :
    r.netSharesDelta < 0 := by
  unfold SwingRound.legalAt at h
  unfold SwingRound.netSharesDelta
  omega

/--
  ★数量方向由成本阶段完全决定（L0，操作数量结构侧完全分类）：
  对任意合法短差回合，其净股数变化的符号由所处成本阶段 **唯一钉死**——
  reducing ⟹ =0（仓位不增）；zeroing ⟹ <0（严格净减，收回本金）；harvesting ⟹ ≥0（股数不减）。
  三态穷尽 ⟹ 数量方向钉死，无第四方向。注：本定理证「符号 ∈ {=0, <0, >0} 的析取」（三歧穷尽，
  对任意 Int 平凡真）；其 **信息内容** 在于上面三条 per-stage 定理（reducing=0/zeroing<0/harvesting≥0）
  把每个阶段的符号 **唯一** 钉死——这才是「数量依成本阶段」的结构侧 L0 内容（codex Q2）。
-/
theorem netDelta_sign_determined_by_stage (r : SwingRound) (s : CostStage)
    (h : r.legalAt s) :
    r.netSharesDelta = 0 ∨ r.netSharesDelta < 0 ∨ r.netSharesDelta > 0 := by
  rcases CostStage.stage_total s with hs | hs | hs
  · subst hs; exact Or.inl (reducing_no_position_increase r h)
  · subst hs; exact Or.inr (Or.inl (zeroing_net_reduce r h))
  · subst hs
    have hge : r.netSharesDelta ≥ 0 := harvesting_shares_nondecreasing r h
    omega

/-! ## §3 仓位数量 = 成本阶段轨迹的累积（结构侧 L0）

  实际持仓股数 = 初始股数 + 所有短差回合 netSharesDelta 之和。第31课的「股数越来越多」
  是 harvesting 阶段所有回合 netDelta ≥ 0 的累积结果（结构侧 L0，by construction）。
-/

/-- 一段成本阶段下的短差回合序列（同一阶段 s 内的多个回合）。 -/
def applyRounds (init : Int) (rounds : List SwingRound) : Int :=
  rounds.foldl (fun acc r => acc + r.netSharesDelta) init

/--
  ★挣股票阶段持仓单调不减（L0，第31课:28「股票越来越多」）：
  若一段回合序列全部在 harvesting 阶段合法，则持仓股数从 init 单调不减。
  这是「成本为0后股数越来越多」的累积形式化——by construction，非经验断言。
-/
theorem harvesting_position_monotone (init : Int) (rounds : List SwingRound)
    (h : ∀ r ∈ rounds, r.legalAt CostStage.harvesting) :
    applyRounds init rounds ≥ init := by
  unfold applyRounds
  induction rounds generalizing init with
  | nil => simp
  | cons r rest ih =>
      have hr : r.netSharesDelta ≥ 0 :=
        harvesting_shares_nondecreasing r (h r (by simp))
      have hrest : ∀ x ∈ rest, x.legalAt CostStage.harvesting :=
        fun x hx => h x (by simp [hx])
      have : applyRounds (init + r.netSharesDelta) rest ≥ init + r.netSharesDelta := by
        unfold applyRounds; exact ih (init + r.netSharesDelta) hrest
      unfold applyRounds at this
      simp only [List.foldl_cons]
      omega

/--
  ★降成本阶段持仓恒定（L0，第31课:24「仓位不增加」）：
  若一段回合序列全部在 reducing 阶段合法，则持仓股数始终 = init（不增不减）。
-/
theorem reducing_position_constant (init : Int) (rounds : List SwingRound)
    (h : ∀ r ∈ rounds, r.legalAt CostStage.reducing) :
    applyRounds init rounds = init := by
  unfold applyRounds
  induction rounds generalizing init with
  | nil => simp
  | cons r rest ih =>
      have hr : r.netSharesDelta = 0 :=
        reducing_no_position_increase r (h r (by simp))
      have hrest : ∀ x ∈ rest, x.legalAt CostStage.reducing :=
        fun x hx => h x (by simp [hx])
      simp only [List.foldl_cons, hr]
      rw [show init + (0 : Int) = init from by omega]
      exact ih init hrest

/-! ## §4 整合：payoff 操作态 = (成本阶段, 走势配置, 当前级别) 结构（结构侧 L0）

  把成本阶段状态机接到 603 递归数据类型的走势配置上——payoff 操作态由走势结构（Move/Center/BSP）
  + 成本阶段共同决定。这是「仓位数量纤维」在 603 范式下的结构表达。
-/

/--
  ★payoff 操作态（结构侧 datatype，L0）。
  - `stage`：成本阶段（第31课三态，§1）。
  - `outcome`：当前级别走势裁决（603 递归类型 classifyMove 三/四值）。
  - `bsp`：当前级别端点买卖点标签集（603 BSP 三类穷尽，决定操作是否触发）。
  - `levelIdx`：当前级别索引（c 的自变量之一，codex 019eff17）。
  - `lConfirm`：区间套确认深度 L_confirm（c 的自变量之一，codex 019eff17 §4A / 601 第四轴）。
  payoff 操作态完全由这些 **离散结构字段** 刻画（结构侧），无连续数值进入此 datatype。
-/
structure PayoffState where
  stage : CostStage
  outcome : MoveOutcome
  bsp : BSPLabelSet
  levelIdx : Nat
  lConfirm : Nat

/--
  ★payoff 操作态完全分类（L0，结构侧穷尽）：payoff 操作态的「成本阶段维度」属三构造子之一。
  结合 603 的走势三分（outcome_total）与 BSP 三类（bsp_type_completeness），payoff 操作态的
  全部离散结构维度由已结算缠论定理钉死——by construction 完全，无轴发现。
-/
theorem payoffState_stage_total (ps : PayoffState) :
    ps.stage = CostStage.reducing
    ∨ ps.stage = CostStage.zeroing
    ∨ ps.stage = CostStage.harvesting :=
  CostStage.stage_total ps.stage

/--
  ★操作触发由 BSP 标签集 + 成本阶段共同决定（结构侧 L0）。
  操作是否「卖」：当前级别 BSP 标签集非空（有买卖点，603 升跌完备性）∧ 成本阶段决定回补规则。
  这形式化「r* 真顶减仓由 BSP 标签操作（L0），具体减多少依成本阶段（结构侧方向 L0 / 数值 L3）」。
-/
def PayoffState.operationTriggered (ps : PayoffState) : Bool :=
  ps.bsp.labels ≠ []

/-- ★操作触发恒真（L0，升跌完备性）：端点标签集非空 ⟹ 任意 payoff 态都有合法操作触发。 -/
theorem operation_always_triggered (ps : PayoffState) :
    ps.operationTriggered = true := by
  unfold PayoffState.operationTriggered
  exact decide_eq_true ps.bsp.nonempty

/-! ## §5 payoff 系数 c 的自变量结构含 L_confirm（结构侧 L0，codex 019eff17 §4A）

  ★codex 019efff7 Q3 FAIL 修正（no-patch / 声明膨胀 090）：本节 **不** 声称形式化了
  「c 非独立于 L_confirm」——那需要定义真 C 并证 `C(...,l₁) ≠ C(...,l₂)`，而真 C 的数值是
  L2/L3 不可由结构 L0 钉死。本节只形式化两件 **诚实可证** 的结构侧事实：
  (1) c 的自变量类型 CoeffArgs **含** lConfirm 字段（自变量结构含 L_confirm，非平凡：字段穷尽
      四个，与 codex 019eff17 逐字列举一致）；
  (2) **存在一个依赖 lConfirm 的 C 模式**（`coeffSchema`）使 `C(...,l₁) ≠ C(...,l₂)`——证明
      「c 依赖 L_confirm」是 **可实现的**（realizable），而非「c 独立于 L_confirm」（后者要求
      **任何** C 都不依赖 lConfirm，被一个反例模式否证）。这是 codex 019eff17 否定 bc-concepts
      「c 独立于 L_confirm」的诚实结构侧落地——不声称真 C 的数值，只证依赖可实现。

  codex 019eff17 §4A 逐字：c=C(走势配置, 中枢列表, 当前级别, L_confirm)，实时操作语义里
  L_confirm 参与「哪些中枢算已成立」。真 C 的数值响应是 L2/L3（§6 opaque），不在本节。
-/

/--
  ★payoff 系数 c 的自变量（codex 019eff17 §4A，结构侧 datatype，L0）。
  C 的四个输入字段穷尽（codex 019eff17 逐字）：
  - `trendConfig`：走势配置（当前级别走势裁决）。
  - `centers`：中枢列表（实时语义里「哪些中枢算已成立」依赖 L_confirm）。
  - `levelIdx`：当前级别。
  - `lConfirm`：确认深度（参与中枢成立判定，∴ 进入 c 自变量）。
-/
structure CoeffArgs where
  trendConfig : MoveOutcome
  centers : List Center
  levelIdx : Nat
  lConfirm : Nat

/--
  ★自变量结构含 L_confirm 字段（结构侧 L0，codex 019efff7 Q3 修正后的诚实表述）。
  CoeffArgs 的 lConfirm 字段 **结构上可独立变化**：存在两组仅 lConfirm 不同的自变量。
  这 **不是** 「c 非独立于 L_confirm」（那要证真 C 的响应，见 `coeffSchema_depends_on_lConfirm`），
  只是「lConfirm 是 CoeffArgs 的真字段（非冗余、可独立取值）」——自变量结构层的事实。
-/
theorem coeffArgs_has_lConfirm_field :
    ∃ a b : CoeffArgs,
      a.trendConfig = b.trendConfig
      ∧ a.centers = b.centers
      ∧ a.levelIdx = b.levelIdx
      ∧ a.lConfirm ≠ b.lConfirm := by
  refine ⟨⟨MoveOutcome.consolidation, [], 0, 0⟩,
          ⟨MoveOutcome.consolidation, [], 0, 1⟩, rfl, rfl, rfl, ?_⟩
  decide

/--
  ★一个依赖 lConfirm 的 C 模式（结构侧 L0，codex 019efff7 Q3：证依赖可实现）。
  `coeffSchema` 是 **一个** 合法的 C 模式（不是真 C——真 C 的数值是 L2/L3）：它把自变量投到一个
  整数标识，**显式让 lConfirm 影响输出**。它的存在证明「c 依赖 L_confirm」是可实现的——
  这正是否定「c 独立于 L_confirm」所需（独立 = 任何 C 都不依赖 lConfirm；一个依赖反例即否之）。
-/
def coeffSchema (a : CoeffArgs) : Int := (a.levelIdx : Int) * 10 + (a.lConfirm : Int)

/--
  ★C 模式真依赖 lConfirm（结构侧 L0，codex 019efff7 Q3 核心修正）：
  存在两组仅 lConfirm 不同的自变量，使 `coeffSchema` 输出 **不同**——
  即 `C(...,l₁) ≠ C(...,l₂)`。这才是「c 依赖 L_confirm 可实现」的真证明（区别于
  `coeffArgs_has_lConfirm_field` 的字段存在性）。否定 bc-concepts「c 独立于 L_confirm」
  （独立要求对 **所有** C 成立，被本反例模式否证）。真 C 的具体数值仍 L2/L3（§6）。
-/
theorem coeffSchema_depends_on_lConfirm :
    ∃ a b : CoeffArgs,
      a.trendConfig = b.trendConfig
      ∧ a.centers = b.centers
      ∧ a.levelIdx = b.levelIdx
      ∧ a.lConfirm ≠ b.lConfirm
      ∧ coeffSchema a ≠ coeffSchema b := by
  refine ⟨⟨MoveOutcome.consolidation, [], 0, 0⟩,
          ⟨MoveOutcome.consolidation, [], 0, 1⟩, rfl, rfl, rfl, ?_, ?_⟩
  · decide
  · decide

/--
  ★c 的结构表示保留 L_confirm（L0）：把 CoeffArgs 投为离散结构标识，第四分量 = lConfirm。
-/
def coeffStructure (a : CoeffArgs) : MoveOutcome × Nat × Nat × Nat :=
  (a.trendConfig, a.centers.length, a.levelIdx, a.lConfirm)

/--
  ★c 的结构标识保留 L_confirm（L0）：coeffStructure 的第四分量 = lConfirm。
  ⟹ 仅 lConfirm 不同的两组自变量有不同的 c 结构标识——c 结构对 L_confirm 敏感（非折叠）。
  这是「L_confirm 必然进入 c 自变量」的结构侧见证（601 第四轴 + codex 019eff17 一致）。
-/
theorem coeffStructure_retains_lConfirm (a b : CoeffArgs)
    (h : a.lConfirm ≠ b.lConfirm) :
    coeffStructure a ≠ coeffStructure b := by
  unfold coeffStructure
  intro heq
  exact h (congrArg (fun x => x.2.2.2) heq)

/-! ## §6 纯连续净收益数值侧（L2/L3，显式 opaque，与结构侧分开）

  ★诚实分层的核心（formalization-validity-domain + 编排者覆盖 D4）：
  结构侧（§1-§5）是 **活跃持仓成本管理的结构侧 L0 完全分类**（成本阶段三态穷尽 + 数量方向钉死
  + c 自变量含 L_confirm）。**纯连续净收益数值** 是不可 datatype 化的
  L2/L3——它依赖 regime（强牛 vs 震荡）、具体阈值、跨标的鲁棒性，需真实数据验证（L2/L3）。
  这里 **不** 给它任何 L0 计算定义（那会是声明膨胀 090），只声明它是一个 opaque 实值函数，
  把「哪个 case 净收益正」「配额具体数值」「regime 是否溶解」显式标注为本文件 **不裁定** 的
  L2/L3 开放问题（数据阻塞，继承 597 pending_verification / 561 b2/563）。
-/

/--
  ★净收益数值（L2/L3，opaque，本文件不给 L0 定义）。
  payoffValue 把一个 payoff 操作态映到实值净收益。它依赖 regime / 阈值 / 标的——
  是 L2/L3 经验对象，**没有 L0 by-construction 定义**。这里用 `opaque` 显式声明它存在但
  其值不由本文件的形态层结构钉死（诚实分层：不为未验证的净收益预测声明 L0 能力）。

  ★为什么 opaque 而非 def：给它任何具体 def（如 `fun _ => 0`）= 声明膨胀（090）——
  会假装净收益由结构 L0 决定。opaque 如实表达「值存在但 L0 不可计算，待 L2/L3 数据」。
-/
opaque payoffValue : PayoffState → Int

/--
  ★结构侧 ⊥ 数值侧（诚实分层定理，L0）：payoffValue 是 opaque——本文件证 **不出** 它的任何
  具体取值或符号。这形式化「净收益数值不由形态层结构钉死」（编排者覆盖 D4 的边界）：
  结构侧完全分类（§1-§5 L0）成立，**不蕴含** 净收益数值可计算（L2/L3 数据阻塞）。

  本定理是一个 **元层诚实声明**：它陈述 payoffValue 的存在（类型正确）而不声明其值——
  下面没有任何 `payoffValue x = 具体值` 的定理，正是 opaque 的诚实之处。
-/
theorem payoffValue_is_opaque_witness (ps : PayoffState) :
    ∃ v : Int, payoffValue ps = v :=
  ⟨payoffValue ps, rfl⟩

/-! ## §7 总判定（活跃持仓成本管理·结构侧 L0 完全分类，数值侧 L2/L3 分开）

  ★reform 结论（编排者覆盖 codex D4 的兑现，codex 019efff7 Q5 降级后的诚实表述）：
  payoff 层 **不是** 「纯连续 L3 净收益、无任何 datatype 结构」（codex D4「payoff 纯 L3」立场太强，
  被覆盖）；但本文件也 **不** 声称「payoff 层完全分类」（那也太强——payoff 含真 C 数值 + 持仓
  生命周期，均 L2/L3 或本论域外）。诚实结论：**活跃持仓成本管理的结构侧** ——成本阶段（§1 三态
  穷尽）/ 操作数量方向（§2 钉死）/ c 自变量结构（§5 含 L_confirm，依赖可实现）——是 L0 完全分类
  （对构造子结构归纳，by construction，继承第31课 + 603 + codex 019eff17）。真 C 的数值响应 +
  纯连续净收益数值（§6 payoffValue）+ 持仓清仓终止态（§1 有效域注）= L2/L3 或本论域外，显式分开。
-/

/--
  ★活跃持仓成本管理·结构侧完全分类总判定（L0）：
  对任意 payoff 操作态 ps，其三个离散结构维度全部由已结算定理穷尽：
  (1) 成本阶段三态（第31课活跃持仓成本演化，stage_total）；
  (2) 走势裁决三/四值（603 outcome_total，经 classifyMove）；
  (3) 操作触发由 BSP 标签集非空决定（603 升跌完备性，operation_always_triggered）。
  三维度 by construction 穷尽 ⟹ payoff 操作态结构侧 L0 完全分类（无第四成本态、无轴发现）。
  注（codex Q5）：这是 **活跃持仓成本管理结构侧** 的完全分类，不是「payoff 层」整体的完全分类
  （真 C 数值 + 净收益 + 清仓终止态在 §6/§1 有效域注里显式分到 L2/L3 或论域外）。
-/
theorem payoff_structural_complete (ps : PayoffState) :
    (ps.stage = CostStage.reducing
      ∨ ps.stage = CostStage.zeroing
      ∨ ps.stage = CostStage.harvesting)
    ∧ (ps.outcome = MoveOutcome.trend Formal.TrendTrichotomy.Direction.up
        ∨ ps.outcome = MoveOutcome.trend Formal.TrendTrichotomy.Direction.down
        ∨ ps.outcome = MoveOutcome.consolidation
        ∨ ps.outcome = MoveOutcome.higherCenterCandidate)
    ∧ ps.operationTriggered = true := by
  refine ⟨payoffState_stage_total ps, ?_, operation_always_triggered ps⟩
  cases h : ps.outcome with
  | trend d =>
      cases d with
      | up => exact Or.inl rfl
      | down => exact Or.inr (Or.inl rfl)
  | consolidation => exact Or.inr (Or.inr (Or.inl rfl))
  | higherCenterCandidate => exact Or.inr (Or.inr (Or.inr rfl))

end Formal.Tlayers.Payoff
