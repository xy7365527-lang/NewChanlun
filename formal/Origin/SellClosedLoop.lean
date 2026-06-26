/-
Origin/SellClosedLoop.lean — 卖侧 Θ 闭环装配（卖点决策接 ledger，task #121）

★工位定位（#120 still-MISSING-(a) 判决）：#120 SellPointRecog 做了**卖侧判据 + recog 对偶**
  （recogChanlunSell：顶背驰→closeRoot / 回升不破 ZD→reduceCore / §11 力度延续→hold），但诚实标
  still-MISSING-(a)：卖侧 recog **未接 ledger 闭环**——`ledgerClosureMissing` 标签。
  本文件收尾：镜像 #117 ThetaInstantiation §5/§6，把卖点决策接 committed `Origin.ledgerStep`，
  给**卖侧非退化闭环 witness** + **买卖闭环对偶对称见证**。

  ★关键诚实：本文件**不改** committed SellPointRecog / ThetaInstantiation / FullDefinitionStrategy
  （全只读，建新 distinct 模块）。只消费：
  - `recogChanlunSell` / `SellDecision` / `sampleType1Sell` / `sampleType3Sell`（#120 committed）。
  - `LedgerState` / `ledgerStep` / `mkLedger`（FullDefinitionStrategy committed，保 R=Π-A-W）。
  - `IsType1Sell` / `IsType3Sell`（#120/#113 committed 卖点判据）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- §10.1（三类买卖点，逐条买卖对偶）：
  · 第一类卖点：上涨趋势向上**突破**最后一个中枢后的**顶背驰点**（forceC < forceA）。
  · 第三类卖点：下跌趋势向下离开中枢后，回抽高点**不升破 ZD** 的终结点。
- §10.2 趋势转折定律：上涨转折由某级别第一类卖点构成（卖侧清仓的缠论根据）。
- §11（背驰力度比较）：力度延续（A≤C，非背驰）不构成卖点 ⟹ 应对为 hold（不动账）。
- 买卖对偶（#120 已证四层对称）：建仓增核（买，A↑）↔ 清仓减核（卖，A↓）——本文件在 ledger 层兑现该镜像。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 卖点决策→账本 delta 转移函数图 / 买卖镜像对偶，omega/decide/rfl
machine-checked，不依赖数据）。`lake env lean Origin/SellClosedLoop.lean` 通过 = 「卖侧闭环
transition 真改 ledger 且两类卖点不同 delta + 保 R=Π-A-W + 与买侧闭环镜像对偶」在定义层成立。

- **触及 L1 缠论规则真编码**（与 #117 买侧闭环对偶）：本文件的卖侧 ledger delta 由 #120
  `recogChanlunSell` 的**真缠论卖点分类**（第一类顶背驰 / 第三类回升 / 力度延续）确定——
  顶背驰⟹清仓（Π↑ 实现利润 + A↓ 减根仓位）、第三类⟹减核（A↓）、力度延续⟹不动账。
  这是 L1 卖侧规则的结构编码（规则忠实，非经验验证——卖点规则在真实行情上的盈利性是 L2/L3，
  本文件不声称）。

- **诚实边界（no声明膨胀）**：本文件**不**证：
  · 卖侧应对盈利/最优/实盘有效（L3 EmpiricalDomain）；
  · 仓位减量/利润实现数额来自缠论（是 Θ_risk 参数——缠论分类只决定 delta 的**方向/类型**
    [清仓 realize+deallocate / 减核 deallocate / hold noop]，不决定精确数额）；
  · **TW 端（取本金/提现，#90 三阶段全局账本的 W 提现端）的对接**——本文件用 R=Π-A-W **单账本**
    卖侧（W 在本文件的卖侧 delta 中恒 0，不动 W）。卖侧顶背驰的「实现利润」记在 Π↑（账面已实现
    损益），**不**触发 W 提现（提现是三阶段资金管理的另一决策层，#90 TW 端，本文件诚实留 MISSING，
    不把 TW 端硬塞进单账本卖侧）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root
`Origin.SellClosedLoop`）。
依赖方向（单向无环）：SellClosedLoop → {SellPointRecog, ThetaInstantiation, FullDefinitionStrategy,
  BspClassification}（均 Origin 内 committed，只读，无 legacy import，不改 committed）。

谱系：#117 ThetaInstantiation 买侧闭环 committed（chanlunTransition + chanlun_transition_distinguishes_classes）
      → #120 SellPointRecog 卖侧判据 + recog 对偶 committed（诚实标 still-MISSING-(a) 卖侧 ledger 闭环）
      → 本文件 #121（镜像 #117 §5/§6，把卖点决策接 ledger，给卖侧非退化闭环 + 买卖闭环对偶对称，
        消 #120 still-MISSING-(a)；TW 端对接诚实留 still-MISSING）。
-/

import Origin.SellPointRecog
import Origin.ThetaInstantiation
import Origin.FullDefinitionStrategy
import Origin.BspClassification

namespace NewChanlun.Origin.SellClosedLoop

open NewChanlun.Origin
open NewChanlun.Origin.SellPointRecog
  (SellDecision recogChanlunSell IsType1Sell sampleType1Sell sampleType3Sell
   sampleType1Sell_isType1Sell sampleType3Sell_isType3Sell
   recogSell_type1_closeRoot recogSell_type3_reduceCore)
open NewChanlun.Origin.ThetaInstantiation (ChanlunAccount ChanlunDecision decisionLedgerDelta)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 卖点决策 → 账本 delta（消 #120 still-MISSING-(a)：卖侧决策接 ledger）

  ★关键收尾：#120 的 `recogChanlunSell` 输出 `SellDecision`（closeRoot/reduceCore/hold）但
  **未映为账本 delta**。本节给卖侧的决策→delta 映射，与 #117 买侧 `decisionLedgerDelta`
  （openRoot (0,+1,0) / accreteCore (0,+2,0) / hold (0,0,0)）严格镜像对偶。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★卖点账本事件 `sellDecisionLedgerDelta`（L0，本文件核心——消 #120 still-MISSING-(a)）：
  把**真缠论卖点决策**映为账本三元组 delta `(dΠ, dA, dW)`（喂给 committed `ledgerStep`）。

  卖点决策 ⟹ 账本副作用（账户因果链：卖点分类决定账本操作类型，与买侧镜像对偶）：
  - `closeRoot`（第一类顶背驰清根仓）⟹ `(1, -1, 0)`：Π += 1（实现利润，账面已实现损益）
    ∧ A -= 1（清根仓位，释放占用）。**与买侧 openRoot (0,+1,0) 在 A 分量镜像对偶**
    （建根仓 A+1 ↔ 清根仓 A-1），并额外携 Π↑（卖出实现利润——买侧建仓无 Π 变化，对偶非完全对称，
    见 §诚实标注：卖出实现损益是买入的时间不对称，缠论买卖在「方向」对称但在「损益实现时点」不对称）。
  - `reduceCore`（第三类回升减核）⟹ `(0, -2, 0)`：A -= 2（减核，释放更多——**与第一类清仓不同量**，
    区分两类卖点应对的账本足迹）。**与买侧 accreteCore (0,+2,0) 在 A 分量严格镜像**（增核 A+2 ↔ 减核 A-2）。
  - `hold`（力度延续保持）⟹ `(0, 0, 0)`：账本不变（无卖点不动账，与买侧 hold 共用）。

  ★非退化关键：closeRoot 与 reduceCore 的 dA 不同（-1 vs -2）⟹ **两个不同缠论卖点分类的事件
  产生不同的 ledger delta**（卖侧非退化，对偶 #117 买侧）。
  ★诚实：数额 1/2 是 Θ_risk 占位常量（具体数额是运行时数据 L2）——缠论卖点分类决定 delta 的
  **类型/符号**（清仓 realize+deallocate / 减核 deallocate），不决定精确数额（数额下游 Θ_risk 填）。
  ★诚实（TW 端）：W 分量恒 0——本文件**不**接 #90 三阶段取本金/提现（W 提现端）。卖侧顶背驰的
  「实现利润」记 Π↑（账面），提现（W↑）是另一决策层（TW 端，still-MISSING），不硬塞进单账本卖侧。
-/
def sellDecisionLedgerDelta : SellDecision → (Int × Int × Int)
  | SellDecision.closeRoot  => (1, -1, 0)   -- 清根仓：realize 利润 Π+1 ∧ deallocate A-1
  | SellDecision.reduceCore => (0, -2, 0)   -- 减核：deallocate A-2（≠ 清根仓，非退化）
  | SellDecision.hold       => (0, 0, 0)    -- 保持：noop

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 卖侧闭环 transition：真缠论卖点 transition 真改 ledger（非退化 witness 核心）

  ★镜像 #117 §5：#117 买侧 `chanlunTransition` 由 `recogChanlun`（买点）驱动 ledger。本文件
  卖侧 `sellTransition` 由 #120 `recogChanlunSell`（卖点）驱动 ledger delta，保 R=Π-A-W 恒等。
  复用 #117 committed `ChanlunAccount`（携 LedgerState R=Π-A-W）作账户态——买卖共用同一账本载体。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★卖侧真缠论闭环转移 `sellTransition`（L0，本文件核心——卖侧非退化 witness）：
  `T_sell : ChanlunAccount → BspEndpoint → ChanlunAccount`。

  闭环数据流（真缠论卖点，镜像 #117 买侧）：
  1. `recogChanlunSell e` 用 #120 卖点判据识别真缠论卖点决策 d（第一类清仓/第三类减核/延续保持）。
  2. `sellDecisionLedgerDelta d` 把卖点决策映为账本 delta（清仓 Π+1/A-1 / 减核 A-2 / 保持 noop）。
  3. `ledgerStep`（committed，保 R=Π-A-W）用该 delta 更新账本，写回完整 ChanlunAccount。

  ★这是**真缠论卖点 transition 真改 ledger**：ledger 下一态由**端点的真缠论卖点分类**决定，
  且保账本恒等（ledgerStep 的 inv）。消 #120 still-MISSING-(a)：卖侧 recog 现已接 ledger 闭环。
-/
def sellTransition (z : ChanlunAccount) (e : BspEndpoint) : ChanlunAccount :=
  let d := recogChanlunSell e
  let delta := sellDecisionLedgerDelta d
  { ledger := ledgerStep z.ledger delta.1 delta.2.1 delta.2.2 }

/--
  ★★卖侧闭环 T 保 R=Π-A-W（L0，账户恒等接入卖侧真缠论闭环）：
  `sellTransition z e` 后 ledger 仍满足 R = Π - A - W（committed `ledgerStep` 的 inv 保持）。
  真缠论卖点分类驱动的账本更新进入卖侧闭环，且每步保账本恒等。
-/
theorem sellTransition_preserves_ledger_inv (z : ChanlunAccount) (e : BspEndpoint) :
    (sellTransition z e).ledger.R =
      (sellTransition z e).ledger.Pi
        - (sellTransition z e).ledger.A
        - (sellTransition z e).ledger.W :=
  (sellTransition z e).ledger.inv

/--
  ★★第一类顶背驰卖点真改 ledger·A 侧（L0，卖侧非退化 witness ①·清仓侧）：
  若端点是第一类卖点（`IsType1Sell`），则卖侧闭环转移后 A 分量 = 原 A - 1（清根仓 deallocate 1）——
  ledger **真被第一类卖点缠论分类改写**（A 减 1）。坐实「卖侧真缠论 transition 真改 ledger」。
-/
theorem sellTransition_type1_deallocates (z : ChanlunAccount) (e : BspEndpoint)
    (h : IsType1Sell e) :
    (sellTransition z e).ledger.A = z.ledger.A - 1 := by
  unfold sellTransition
  rw [recogSell_type1_closeRoot e h]
  simp only [sellDecisionLedgerDelta, ledgerStep, mkLedger]
  omega

/--
  ★★第一类顶背驰卖点真实现利润·Π 侧（L0，卖侧非退化 witness ①·实现利润侧）：
  若端点是第一类卖点，则卖侧闭环转移后 Π 分量 = 原 Π + 1（实现利润，账面已实现损益）——
  这是卖侧**特有**的账本足迹（买侧建仓 openRoot 不改 Π，卖侧清仓 closeRoot 实现 Π↑）。
-/
theorem sellTransition_type1_realizes (z : ChanlunAccount) (e : BspEndpoint)
    (h : IsType1Sell e) :
    (sellTransition z e).ledger.Pi = z.ledger.Pi + 1 := by
  unfold sellTransition
  rw [recogSell_type1_closeRoot e h]
  simp only [sellDecisionLedgerDelta, ledgerStep, mkLedger]

/--
  ★★第三类回升卖点真改 ledger（L0，卖侧非退化 witness ①·减核侧）：
  若端点是第三类卖点（`IsType3Sell` ∧ 未破中枢），则卖侧闭环转移后 A 分量 = 原 A - 2（减核 deallocate 2）——
  ledger **真被第三类卖点缠论分类改写**（A 减 2，**与第一类清仓的 -1 不同**）。
-/
theorem sellTransition_type3_deallocates (z : ChanlunAccount) (e : BspEndpoint)
    (h : IsType3Sell e) (hnobreak : e.brokeCenter = false) :
    (sellTransition z e).ledger.A = z.ledger.A - 2 := by
  unfold sellTransition
  rw [recogSell_type3_reduceCore e h hnobreak]
  simp only [sellDecisionLedgerDelta, ledgerStep, mkLedger]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 ★★★卖侧非退化总见证：两类卖点 ⟹ 不同 ledger delta（镜像 #117 §6）

  对偶 #117 `chanlun_transition_distinguishes_classes`：**存在两个具体卖点端点，它们被真缠论
  卖点分类识别为不同类（第一类 vs 第三类），且产生不同的 ledger delta**——这是卖侧闭环对缠论
  卖点分类敏感的直接见证（消 #120 still-MISSING-(a)：卖侧 recog 接 ledger 且非退化）。
  复用 #120 committed 见证端点 `sampleType1Sell`（突破中枢+顶背驰）/`sampleType3Sell`（离开中枢回升不破 ZD）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★见证：sampleType1Sell（#120 committed）经卖侧闭环转移后 A = base - 1（第一类清根仓）。 -/
theorem sampleType1Sell_transition_A (z0 : ChanlunAccount) :
    (sellTransition z0 sampleType1Sell).ledger.A = z0.ledger.A - 1 :=
  sellTransition_type1_deallocates z0 sampleType1Sell sampleType1Sell_isType1Sell

/-- ★见证：sampleType3Sell（#120 committed）经卖侧闭环转移后 A = base - 2（第三类减核）。 -/
theorem sampleType3Sell_transition_A (z0 : ChanlunAccount) :
    (sellTransition z0 sampleType3Sell).ledger.A = z0.ledger.A - 2 :=
  sellTransition_type3_deallocates z0 sampleType3Sell sampleType3Sell_isType3Sell
    (by unfold sampleType3Sell; rfl)

/--
  ★★★卖侧非退化总见证 `sell_transition_distinguishes_classes`（L0，消 #120 still-MISSING-(a) 核心交付）★★★：

  从同一初始账户态 z₀，**第一类顶背驰卖点端点**与**第三类回升卖点端点**经真缠论卖侧闭环转移后，
  产生的 ledger **A 分量不同**（第一类 A=base-1，第三类 A=base-2）。

  这是 #120 still-MISSING-(a)「卖侧 recog 未接 ledger 闭环」的直接消解：
  - #120 卖侧 recog 输出 SellDecision 但**未改 ledger**（ledgerClosureMissing 标签）。
  - 本文件的卖侧闭环 transition **真改 ledger 且真区分卖点分类**——第一类（顶背驰清仓）与第三类
    （回升减核）被 #120 判据真识别为不同类，ledger A 分量真不同（base-1 ≠ base-2）。

  ★这是 #117 买侧 `chanlun_transition_distinguishes_classes`（base+1 ≠ base+2）的**卖侧对偶**：
  买侧两类建仓增核 A↑ 不同量 ↔ 卖侧两类清仓减核 A↓ 不同量。卖侧闭环非退化兑现。
-/
theorem sell_transition_distinguishes_classes (z0 : ChanlunAccount) :
    (sellTransition z0 sampleType1Sell).ledger.A ≠
      (sellTransition z0 sampleType3Sell).ledger.A := by
  rw [sampleType1Sell_transition_A z0, sampleType3Sell_transition_A z0]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 ★★★买卖闭环对偶对称：建仓增核(A↑) ↔ 清仓减核(A↓) 镜像（接 #120 对偶）

  本节是本文件区别于「仅卖侧闭环」的核心交付：证**买侧闭环（#117）与卖侧闭环（本文件）在 ledger
  A 分量严格镜像对偶**——买侧 openRoot 建根仓 A+1 ↔ 卖侧 closeRoot 清根仓 A-1；买侧 accreteCore
  增核 A+2 ↔ 卖侧 reduceCore 减核 A-2。这把 #120 的 `sell_recog_dual_correspondence`（recog 决策层
  对偶）提升到 **ledger 闭环层对偶**——买卖在账本足迹上严格镜像。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★买卖账本 delta·A 分量镜像（L0，对偶对称见证·决策层）——
  买侧 `decisionLedgerDelta`（#117 committed）与卖侧 `sellDecisionLedgerDelta`（本文件）的 A 分量
  严格相反号：
  - 买 openRoot dA=+1 ↔ 卖 closeRoot dA=-1（建根仓 ↔ 清根仓）。
  - 买 accreteCore dA=+2 ↔ 卖 reduceCore dA=-2（增核 ↔ 减核）。
  - 买 hold dA=0 ↔ 卖 hold dA=0（保持 ↔ 保持）。

  ★这坐实买卖在账本 A（仓位）足迹上的镜像：买入分配（allocate, A↑）↔ 卖出释放（deallocate, A↓）。
-/
theorem buy_sell_A_delta_mirror :
    (decisionLedgerDelta ChanlunDecision.openRoot).2.1 =
      - (sellDecisionLedgerDelta SellDecision.closeRoot).2.1 ∧
    (decisionLedgerDelta ChanlunDecision.accreteCore).2.1 =
      - (sellDecisionLedgerDelta SellDecision.reduceCore).2.1 ∧
    (decisionLedgerDelta ChanlunDecision.hold).2.1 =
      - (sellDecisionLedgerDelta SellDecision.hold).2.1 := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

/--
  ★★买卖闭环 A 分量镜像（L0，对偶对称见证·闭环层·总）——
  从同一初始账户态 z₀：
  - 买侧第一类建根仓后 A 增 1（#117 `chanlunTransition_type1_allocates`）。
  - 卖侧第一类清根仓后 A 减 1（本文件 `sellTransition_type1_deallocates`）。
  二者相对 base 的 A 偏移严格相反（+1 ↔ -1）：买侧建仓与卖侧清仓在闭环 ledger 上镜像对偶。

  ★这是「买卖闭环对偶对称」的总见证（闭环层）：买侧建仓增核（A↑）↔ 卖侧清仓减核（A↓）镜像，
  把 #120 `sell_recog_dual_correspondence`（recog 决策对偶）兑现到闭环 ledger 足迹层。
  对买侧的具体见证用 #117 committed `eventType1`（第一类买点），对卖侧用 #120 committed
  `sampleType1Sell`（第一类卖点）——两个真缠论第一类端点（买/卖）经各自闭环，A 偏移相反号。
-/
theorem buy_sell_closed_loop_A_mirror (z0 : ChanlunAccount) :
    (ThetaInstantiation.chanlunTransition z0 ThetaInstantiation.eventType1).ledger.A
        - z0.ledger.A =
      - ((sellTransition z0 sampleType1Sell).ledger.A - z0.ledger.A) := by
  have hbuy : (ThetaInstantiation.chanlunTransition z0 ThetaInstantiation.eventType1).ledger.A
      = z0.ledger.A + 1 :=
    ThetaInstantiation.chanlunTransition_type1_allocates z0 ThetaInstantiation.eventType1
      ThetaInstantiation.eventType1_isType1
  have hsell : (sellTransition z0 sampleType1Sell).ledger.A = z0.ledger.A - 1 :=
    sampleType1Sell_transition_A z0
  rw [hbuy, hsell]; omega

/--
  ★★买卖闭环第三类 A 分量镜像（L0，对偶对称见证·闭环层·第三类）——
  买侧第三类增核后 A 增 2（#117 `chanlunTransition_type3_allocates`）↔ 卖侧第三类减核后 A 减 2
  （本文件 `sellTransition_type3_deallocates`）：增核（A↑2）↔ 减核（A↓2）镜像。
  对买侧用 #117 committed `eventType3`，对卖侧用 #120 committed `sampleType3Sell`。
-/
theorem buy_sell_closed_loop_A_mirror_type3 (z0 : ChanlunAccount) :
    (ThetaInstantiation.chanlunTransition z0 ThetaInstantiation.eventType3).ledger.A
        - z0.ledger.A =
      - ((sellTransition z0 sampleType3Sell).ledger.A - z0.ledger.A) := by
  have hbuy : (ThetaInstantiation.chanlunTransition z0 ThetaInstantiation.eventType3).ledger.A
      = z0.ledger.A + 2 :=
    ThetaInstantiation.chanlunTransition_type3_allocates z0 ThetaInstantiation.eventType3
      ThetaInstantiation.eventType3_isType3Buy (by unfold ThetaInstantiation.eventType3; rfl)
  have hsell : (sellTransition z0 sampleType3Sell).ledger.A = z0.ledger.A - 2 :=
    sampleType3Sell_transition_A z0
  rw [hbuy, hsell]; omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 诚实标签（gatekeeper：禁标盈利/TW端已接/平凡占位）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★卖侧闭环标签 `SellClosedLoopTag`（gatekeeper，诚实分层）。
  - `realSellLedgerClosure`：卖侧 recog 决策（#120 recogChanlunSell）真接 committed `ledgerStep`（消 #120 (a)）。
  - `nonDegenerateSellTransition`：卖侧闭环真改 ledger 且两类卖点不同 delta（非占位）。
  - `buySellLedgerMirror`：买卖闭环在 A 分量镜像对偶（建仓增核 A↑ ↔ 清仓减核 A↓）。
  - `preservesLedgerInv`：卖侧闭环保 R=Π-A-W（committed ledgerStep inv）。
  - `thetaRiskParametric`：仓位减量/利润数额是 Θ_risk 参数（缠论卖点分类只定 delta 类型不定数额）。
  - `twEndMissing`：TW 端（取本金/提现 W 提现，#90 三阶段）对接**未做**（still-MISSING，单账本卖侧不塞 TW）。
  - `empiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableSellStrategy` / `TWEndDone` / `TrivialPlaceholder` 构造子——
  类型层拒绝把本卖侧闭环标为盈利策略 / TW 端已对接 / 平凡占位。
-/
inductive SellClosedLoopTag where
  | realSellLedgerClosure
  | nonDegenerateSellTransition
  | buySellLedgerMirror
  | preservesLedgerInv
  | thetaRiskParametric
  | twEndMissing
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★卖侧闭环子类（gatekeeper）：SellClosedLoopAssembled（唯一构造子，禁标 TW 端已接/平凡占位）。 -/
inductive SellClosedLoopSubkind where
  | sellClosedLoopAssembled
deriving DecidableEq, Repr

/-- ★诚实标签包（L0 声明）。 -/
def sellClosedLoopLabels : List SellClosedLoopTag × SellClosedLoopSubkind :=
  ([SellClosedLoopTag.realSellLedgerClosure, SellClosedLoopTag.nonDegenerateSellTransition,
    SellClosedLoopTag.buySellLedgerMirror, SellClosedLoopTag.preservesLedgerInv,
    SellClosedLoopTag.thetaRiskParametric, SellClosedLoopTag.twEndMissing,
    SellClosedLoopTag.empiricalDomain],
   SellClosedLoopSubkind.sellClosedLoopAssembled)

/-- ★禁标平凡占位/盈利/TW端已接（L0，gatekeeper 见证）：子类必是 SellClosedLoopAssembled。 -/
theorem sellClosedLoop_subkind_is_assembled (k : SellClosedLoopSubkind) :
    k = SellClosedLoopSubkind.sellClosedLoopAssembled := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（task #121，卖侧 Θ 闭环装配，消 #120 still-MISSING-(a)）

  本文件**证**（L0 结构层 + L1 卖侧缠论规则编码，machine-checked，无 sorry/admit/axiom）：

  1. ★卖点决策 → 账本 delta `sellDecisionLedgerDelta`（消 #120 still-MISSING-(a) 的核心映射）：
     清根仓 (Π+1, A-1) / 减核 (A-2) / 保持 noop——与 #117 买侧 `decisionLedgerDelta` 镜像对偶。
  2. ★★卖侧真缠论闭环转移 `sellTransition`（#120 recogChanlunSell 决策驱动 ledger delta，
     保 R=Π-A-W）——镜像 #117 §5 买侧 `chanlunTransition`。
     · `sellTransition_preserves_ledger_inv`：卖侧闭环保账本恒等（R=Π-A-W）。
     · `sellTransition_type1_deallocates`（第一类 A-=1）/ `_type1_realizes`（第一类 Π+=1 实现利润）/
       `_type3_deallocates`（第三类 A-=2）：真改 ledger。
  3. ★★★**卖侧非退化总见证** `sell_transition_distinguishes_classes`：两类卖点（第一类顶背驰 vs
     第三类回升）经卖侧闭环转移产生**不同 ledger delta**（A-1 vs A-2）——消 #120 still-MISSING-(a)
     （#120 recog 未接 ledger，本文件接且非退化）。对偶 #117 买侧 distinguishes_classes。
     · 具体见证：#120 committed `sampleType1Sell`/`sampleType3Sell` 端点 + 闭环 A 见证。
  4. ★★★**买卖闭环对偶对称** `buy_sell_A_delta_mirror`（决策层 A 分量相反号）+
     `buy_sell_closed_loop_A_mirror`（闭环层·第一类：买建仓 A+1 ↔ 卖清仓 A-1）+
     `buy_sell_closed_loop_A_mirror_type3`（闭环层·第三类：买增核 A+2 ↔ 卖减核 A-2）——
     把 #120 `sell_recog_dual_correspondence`（recog 决策对偶）提升到 **ledger 闭环层对偶**：
     建仓增核（A↑）↔ 清仓减核（A↓）镜像兑现。
  5. 诚实标签 `sellClosedLoopLabels` + `sellClosedLoop_subkind_is_assembled`（禁标 TW 端已接/盈利/平凡占位）。

  本文件**不证**（no声明膨胀，诚实边界）：
  - ✗ 卖侧应对策略盈利/最优/实盘有效（L3 EmpiricalDomain）。
  - ✗ 仓位减量/利润数额（1/2/1）来自缠论（Θ_risk 参数——缠论卖点分类只决定 delta 的**类型/符号**：
       清仓 realize+deallocate / 减核 deallocate，不决定精确数额）。
  - ✗ **TW 端（取本金/提现，#90 三阶段全局账本 W 提现端）的对接**——本文件用 R=Π-A-W **单账本**卖侧
       （W 在卖侧 delta 中恒 0）。卖侧顶背驰「实现利润」记 Π↑（账面已实现损益），**不**触发 W 提现
       （提现是三阶段资金管理另一决策层，TW 端，still-MISSING，待 codex 对接 #90 双账本 twState；
       本文件诚实留白，不把 TW 端硬塞进单账本卖侧——no-workaround）。
  - ✗ 第二类卖点闭环（committed `IsType2Sell` 本级别可观测，买卖点定律一「由次级别一类构成」需
       次级别递归 still-MISSING-D；本文件卖侧闭环覆盖第一类/第三类/延续三态，与 #117 买侧对偶一致）。
  - ✗ bspOf 卖点端点全自动识别（从 K 线流自动判定，still-MISSING，BspClassification 已标）——
       本文件消费已判定的 BspEndpoint（#113 判据层提供）。

  ★对 #88（完整性）推进度：本文件消解 #120 still-MISSING-(a)（卖侧 ledger 闭环），把卖点决策
    接入 committed `ledgerStep`，给卖侧非退化闭环 + 买卖闭环对偶对称。买卖闭环现已**双向对偶完整**
    （买侧 #117 + 卖侧本文件，A 分量镜像）。剩余完整性缺口（诚实标注，不冒充已完成）：
    (a) TW 端双账本对接（#90 twState 提现端，待 codex）；(b) 第二类次级别递归；
    (c) bspOf/centersOf 全自动构造（#113 still-MISSING）。

  ★买卖**非完全对称的缠论特例审查**（no-workaround：遇定义冲突停 ESCALATE）——审查结论：
  **无定义冲突，未触发 ESCALATE**。买卖闭环在 ledger A（仓位）分量上严格镜像（建仓 A↑ ↔ 清仓 A↓）。
  唯一的「非对称」是卖侧 closeRoot 额外携 Π↑（实现利润）而买侧 openRoot 不改 Π——这**不是逻辑不对称**，
  而是**损益实现的时间不对称**（买入是占用储备建仓，无已实现损益；卖出是平仓，实现账面损益 Π↑）。
  这与缠论买卖「方向对偶」一致（§10.1），是会计语义的真实结构（买入分配/卖出实现），不是判据矛盾——
  镜像在「仓位 A 分量」层严格封闭，Π 的不对称是单账本会计的正确表达，无须 ESCALATE。

  谱系：#117 ThetaInstantiation 买侧闭环 committed（chanlunTransition + distinguishes_classes）
        → #120 SellPointRecog 卖侧判据 + recog 对偶 committed（诚实标 still-MISSING-(a) 卖侧 ledger 闭环，
          ledgerClosureMissing 标签）→ 本文件 #121（镜像 #117 §5/§6 把卖点决策接 ledger，给卖侧非退化
          闭环 + 买卖闭环对偶对称四见证，消 still-MISSING-(a)；TW 端对接诚实留 still-MISSING）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SellClosedLoop
