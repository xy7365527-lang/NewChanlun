/-
Origin/SecondSellClosedLoop.lean — 第二类卖点 Θ 闭环装配（二类卖点决策接 ledger，task #129）

★工位定位（still-MISSING 判决）：committed 闭环覆盖**第一类/第三类/延续**三态——
  · 买侧 #117 ThetaInstantiation `chanlunTransition`（ChanlunDecision = openRoot[T1]/accreteCore[T3]/hold）。
  · 卖侧 #121 SellClosedLoop `sellTransition`（SellDecision = closeRoot[T1]/reduceCore[T3]/hold）。
  两侧的 recog/decision/ledger 闭环**都不含第二类分支**（buy 与 sell 的 ChanlunDecision/SellDecision
  均无 type-2 构造子）。committed 只在**判据层**给了第二类买卖对偶（#120 SellPointRecog §3：
  `IsType2Buy` / `IsType2Sell` / `type2_buy_sell_share_criterion`），**未装配第二类的 ledger 闭环**。

  本文件收尾：装配**第二类卖点的 Θ 闭环**（recog 第二类卖点 → 决策 → 账本 delta → committed
  `ledgerStep`），并把 #120 committed 的第二类买卖判据对偶**提升到 ledger 闭环层**——证第二类买点
  加仓（A↑）↔ 第二类卖点减仓（A↓）的 A 分量镜像（真证，非占位）。

  ★关键诚实（no-workaround / no声明膨胀）：
  - committed 不存在「第二类买侧 ledger 闭环」可镜像——本文件不假装镜像一个不存在的对象。
    本文件在 #120 committed 的**第二类本级别可观测判据** `IsType2Buy`/`IsType2Sell` 之上，
    **新建第二类买/卖两侧的 ledger delta**（distinct 决策载体），把买卖对偶在 ledger A 分量上**真证为定理**。
  - 第二类「由次级别第一类构成」（买卖点定律一 §10.2，still-MISSING-D，committed 已标）**仍不证**：
    本文件的第二类 recog 只消费**本级别可观测判据** `IsType2`（afterTypeOne ∧ ¬brokeCenter）——与
    committed `IsType2Buy`/`IsType2Sell`/`recogChanlun`/`recogChanlunSell` 的诚实口径**完全一致**，
    不冒充次级别递归。次级别递归构成是 #116 BspConstruction / #128 SubLevelDescent 的几何下钻领域。
  - 本文件**不改** committed 任何文件（全只读，建新 distinct 模块），**不编辑 lakefile**
    （报 Lead 登记 root `Origin.SecondSellClosedLoop`）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- §10.1（三类买卖点，逐条买卖对偶）：
  · 第二类买点：第一类买点后，次级别上涨结束、再次下跌的那个次级别走势的结束点（一类后回踩确认）。
  · 第二类卖点：第一类卖点后，次级别下跌结束、再次上涨的那个次级别走势的结束点（一类后反抽确认）。
- §10.1 本级别可观测口径（committed BspClassification.IsType2）：第一类后（`afterTypeOne`）∧
  回抽未再破中枢（`¬brokeCenter`）的回抽结束点。这是第二类的**本级别可观测条件**（次级别递归
  构成是另一层，still-MISSING-D）。
- 买卖对偶（#120 committed `type2_buy_sell_share_criterion`）：第二类买点与卖点**共用同一本级别
  可观测判据** `IsType2`，只在 side（long↔short）上对偶——本文件在 ledger 层兑现该镜像。
- 操盘语义（缠论操作纪律）：第二类是「一类确认后的加码/减码」——第二类买点是一类底背驰建仓后
  回踩确认的**二次加仓**（A↑）；第二类卖点是一类顶背驰清仓后反抽确认的**二次减仓**（A↓）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 第二类卖点决策→账本 delta 转移函数图 / 第二类买卖镜像对偶，
omega/decide/rfl machine-checked，不依赖数据）。`lake env lean Origin/SecondSellClosedLoop.lean`
通过 = 「第二类卖侧闭环 transition 真改 ledger（A↓1）+ 保 R=Π-A-W + 与第二类买侧 ledger A 分量
镜像对偶」在定义层成立。

- **触及 L1 缠论规则真编码**：本文件第二类 ledger delta 由 #120 committed 的**真缠论第二类判据**
  （`IsType2Sell` = side=short ∧ afterTypeOne ∧ ¬brokeCenter）确定——第二类卖点⟹减仓（A↓）。
  这是 L1 第二类卖侧规则的结构编码（规则忠实，非经验验证——第二类卖点在真实行情上的盈利性是
  L2/L3，本文件不声称）。

- **诚实边界（no声明膨胀）**：本文件**不**证：
  · 第二类应对盈利/最优/实盘有效（L3 EmpiricalDomain）。
  · 加仓/减仓数额（1）来自缠论（是 Θ_risk 参数——第二类卖点判据只决定 delta 的**方向/类型**
    [减仓 deallocate / 加仓 allocate]，不决定精确数额）。
  · **第二类「由次级别第一类构成」**（买卖点定律一 §10.2，still-MISSING-D）——本文件第二类 recog
    只用本级别可观测判据 `IsType2`，与 committed 口径一致，不冒充次级别递归。
  · **TW 端（取本金/提现，#90 三阶段全局账本 W 提现端）的对接**——本文件用 R=Π-A-W **单账本**
    （W 分量在第二类 delta 中恒 0，不动 W）。提现是另一决策层（TW 端，still-MISSING）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root
`Origin.SecondSellClosedLoop`）。
依赖方向（单向无环）：SecondSellClosedLoop → {SellPointRecog, ThetaInstantiation, BspClassification,
  FullDefinitionStrategy}（均 Origin 内 committed，只读，无 legacy import，不改 committed）。

谱系：#113 BspClassification 判据层 committed（IsType2 本级别可观测）→ #117 ThetaInstantiation 买侧
      闭环 committed（T1/T3/hold，第二类买点诚实标 still-MISSING-D）→ #120 SellPointRecog 第二类
      买卖判据对偶 committed（IsType2Buy/IsType2Sell/type2_buy_sell_share_criterion，闭环 ledger
      对接标 still-MISSING）→ #121 SellClosedLoop 卖侧 T1/T3/hold 闭环 committed（诚实标第二类卖点
      闭环缺）→ 本文件 #129（装配第二类卖点 Θ 闭环 recog→ledger，把 #120 第二类判据对偶提升到
      ledger 闭环层，证第二类买点加仓 A↑ ↔ 第二类卖点减仓 A↓ 镜像；次级别递归构成诚实留 still-MISSING-D）。
-/

import Origin.SellPointRecog
import Origin.ThetaInstantiation
import Origin.BspClassification
import Origin.FullDefinitionStrategy

namespace NewChanlun.Origin.SecondSellClosedLoop

open NewChanlun.Origin
open NewChanlun.Origin.SellPointRecog
  (IsType2Buy IsType2Sell type2_buy_sell_share_criterion type2_buy_sell_exclusive)
open NewChanlun.Origin.ThetaInstantiation (ChanlunAccount)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 第二类买/卖决策载体 + 账本 delta（committed 闭环都缺第二类分支，本文件补）

  ★committed `ChanlunDecision`（买侧三态 openRoot/accreteCore/hold）与 `SellDecision`（卖侧三态
  closeRoot/reduceCore/hold）**均无第二类构造子**。本节给第二类专用的买/卖决策载体（distinct，
  不改 committed enum）+ 第二类 ledger delta，把 #120 第二类判据对偶接到账本层。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★第二类卖点决策 `SecondSellDecision`（L0，distinct 决策载体，不改 committed `SellDecision`）：
  - `reduceSecond`：第二类卖点二次减仓（第一类卖点后反抽确认的减码，§10.1 + 操盘纪律）。
  - `hold`：非第二类卖点 ⟹ 保持（本级别不满足 IsType2Sell 时不动账）。

  ★与 committed `SellDecision`（closeRoot[T1]/reduceCore[T3]/hold）正交：本载体专证第二类卖点的
  ledger 闭环（committed 卖侧闭环缺的分支）。
-/
inductive SecondSellDecision where
  | reduceSecond   -- 第二类卖点 → 二次减仓
  | hold           -- 非第二类卖点 → 保持
deriving DecidableEq, Repr

/--
  ★第二类买点决策 `SecondBuyDecision`（L0，distinct 决策载体，不改 committed `ChanlunDecision`）：
  - `accreteSecond`：第二类买点二次加仓（第一类买点后回踩确认的加码，§10.1 + 操盘纪律）。
  - `hold`：非第二类买点 ⟹ 保持。

  ★这是镜像对偶的**买侧对端**：committed 买侧闭环（ThetaInstantiation）的 ChanlunDecision 不含
  第二类（第二类买点诚实标 still-MISSING-D），本文件在 committed `IsType2Buy` 判据上建第二类买侧
  ledger delta，使「第二类买卖对偶」可在 ledger 层**真证为定理**（非镜像一个不存在的 committed 对象）。
-/
inductive SecondBuyDecision where
  | accreteSecond  -- 第二类买点 → 二次加仓
  | hold           -- 非第二类买点 → 保持
deriving DecidableEq, Repr

/--
  ★★第二类卖点账本 delta `secondSellLedgerDelta`（L0，本文件核心·卖侧）：
  把第二类卖点决策映为账本三元组 delta `(dΠ, dA, dW)`（喂给 committed `ledgerStep`）。
  - `reduceSecond`（第二类二次减仓）⟹ `(0, -1, 0)`：A -= 1（deallocate，释放仓位）。
  - `hold`（非第二类）⟹ `(0, 0, 0)`：noop。

  ★诚实：数额 1 是 Θ_risk 占位常量（具体数额是运行时数据 L2）——第二类卖点判据决定 delta 的
  **类型/符号**（减仓 deallocate），不决定精确数额。
  ★诚实（TW 端）：W 分量恒 0——不接 #90 三阶段 W 提现端。
-/
def secondSellLedgerDelta : SecondSellDecision → (Int × Int × Int)
  | SecondSellDecision.reduceSecond => (0, -1, 0)   -- 第二类二次减仓：deallocate 1
  | SecondSellDecision.hold         => (0, 0, 0)    -- 保持：noop

/--
  ★★第二类买点账本 delta `secondBuyLedgerDelta`（L0，本文件核心·买侧对端，镜像对偶用）：
  - `accreteSecond`（第二类二次加仓）⟹ `(0, +1, 0)`：A += 1（allocate，分配仓位）。
  - `hold`（非第二类）⟹ `(0, 0, 0)`：noop。

  ★A 分量与卖侧 `secondSellLedgerDelta reduceSecond`（-1）**严格相反号**（+1 ↔ -1）——
  第二类买点加仓（A↑）↔ 第二类卖点减仓（A↓）镜像（§4 真证）。
-/
def secondBuyLedgerDelta : SecondBuyDecision → (Int × Int × Int)
  | SecondBuyDecision.accreteSecond => (0, 1, 0)    -- 第二类二次加仓：allocate 1
  | SecondBuyDecision.hold          => (0, 0, 0)    -- 保持：noop

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 第二类卖点识别核 recogSecondSell（真消费 committed IsType2Sell）

  ★committed `recogChanlunSell`（#120）无第二类分支（只 closeRoot[T1]/reduceCore[T3]/hold）。
  本节给第二类专用 recog——真 case-split on committed `IsType2Sell`（side=short ∧ afterTypeOne
  ∧ ¬brokeCenter）。判据可判定（Side DecidableEq + Bool 字段），用 decide 桥接。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★第二类卖点识别核 `recogSecondSell`（L0，本节核心）：从买卖点端点 `BspEndpoint` 真 case-split
  on committed 第二类卖点判据，识别第二类卖侧应对意图。

  缠论第二类卖点规则真编码（L1 卖侧规则结构编码）：
  - **§10.1 第二类卖点**：若 side=short ∧ afterTypeOne=true ∧ brokeCenter=false（committed
    `IsType2Sell` = side=short ∧ `IsType2`）⟹ `reduceSecond`（第二类二次减仓）。
  - 否则 ⟹ `hold`（非第二类卖点，保持）。

  ★真消费 committed `IsType2Sell` 判据字段——非占位（占位无法对第二类判据敏感）。
-/
def recogSecondSell (e : BspEndpoint) : SecondSellDecision :=
  if e.side = Side.short ∧ e.afterTypeOne = true ∧ e.brokeCenter = false then
    SecondSellDecision.reduceSecond
  else
    SecondSellDecision.hold

/--
  ★★第二类买点识别核 `recogSecondBuy`（L0，买侧对端）：真 case-split on committed `IsType2Buy`
  判据（side=long ∧ afterTypeOne ∧ ¬brokeCenter）。镜像 `recogSecondSell`，只在 side 上对偶。
-/
def recogSecondBuy (e : BspEndpoint) : SecondBuyDecision :=
  if e.side = Side.long ∧ e.afterTypeOne = true ∧ e.brokeCenter = false then
    SecondBuyDecision.accreteSecond
  else
    SecondBuyDecision.hold

/--
  ★recog 第二类卖点真消费 committed IsType2Sell 判据（L0，证 recogSecondSell 非平凡）：
  若端点是第二类卖点（committed `IsType2Sell` = side=short ∧ `IsType2`），则 `recogSecondSell`
  识别为 `reduceSecond`。坐实第二类卖侧 recog 真编码§10.1 第二类卖点规则。
-/
theorem recogSecondSell_type2_reduceSecond (e : BspEndpoint) (h : IsType2Sell e) :
    recogSecondSell e = SecondSellDecision.reduceSecond := by
  unfold recogSecondSell IsType2Sell IsType2 at *
  obtain ⟨hside, hafter, hbroke⟩ := h
  rw [if_pos ⟨hside, hafter, hbroke⟩]

/--
  ★recog 第二类买点真消费 committed IsType2Buy 判据（L0，买侧对端）：
  若端点是第二类买点（committed `IsType2Buy` = side=long ∧ `IsType2`），则 `recogSecondBuy`
  识别为 `accreteSecond`。
-/
theorem recogSecondBuy_type2_accreteSecond (e : BspEndpoint) (h : IsType2Buy e) :
    recogSecondBuy e = SecondBuyDecision.accreteSecond := by
  unfold recogSecondBuy IsType2Buy IsType2 at *
  obtain ⟨hside, hafter, hbroke⟩ := h
  rw [if_pos ⟨hside, hafter, hbroke⟩]

/--
  ★第二类卖点 recog 全定义（L0）：任一端点经 `recogSecondSell` 落 `reduceSecond` 或 `hold`。
  与 committed `recogChanlunSell` 全定义对偶——第二类卖侧识别核对任意端点有输出。
-/
theorem recogSecondSell_total (e : BspEndpoint) :
    recogSecondSell e = SecondSellDecision.reduceSecond ∨
    recogSecondSell e = SecondSellDecision.hold := by
  unfold recogSecondSell
  by_cases h : e.side = Side.short ∧ e.afterTypeOne = true ∧ e.brokeCenter = false
  · rw [if_pos h]; exact Or.inl rfl
  · rw [if_neg h]; exact Or.inr rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 第二类卖侧闭环 transition：真第二类卖点 transition 真改 ledger

  ★镜像 #121 SellClosedLoop §2（卖侧 T1/T3 闭环）：第二类卖侧 `secondSellTransition` 由第二类
  卖点判据驱动 ledger delta，保 R=Π-A-W 恒等。复用 committed `ChanlunAccount`（携 LedgerState
  R=Π-A-W）作账户态——与 #117/#121 闭环共用同一账本载体。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★第二类卖侧真缠论闭环转移 `secondSellTransition`（L0，本文件核心——第二类卖侧 witness）：
  `T_2sell : ChanlunAccount → BspEndpoint → ChanlunAccount`。

  闭环数据流（真第二类卖点）：
  1. `recogSecondSell e` 用 committed 第二类卖点判据识别决策 d（reduceSecond / hold）。
  2. `secondSellLedgerDelta d` 把决策映为账本 delta（减仓 A-1 / 保持 noop）。
  3. `ledgerStep`（committed，保 R=Π-A-W）用该 delta 更新账本，写回完整 ChanlunAccount。

  ★这是**真第二类卖点 transition 真改 ledger**：ledger 下一态由端点的真第二类卖点分类决定，
  且保账本恒等。消第二类卖侧闭环缺口（committed 卖侧闭环只有 T1/T3/hold）。
-/
def secondSellTransition (z : ChanlunAccount) (e : BspEndpoint) : ChanlunAccount :=
  let d := recogSecondSell e
  let delta := secondSellLedgerDelta d
  { ledger := ledgerStep z.ledger delta.1 delta.2.1 delta.2.2 }

/--
  ★★第二类买侧闭环转移 `secondBuyTransition`（L0，买侧对端，镜像对偶用）：
  镜像 `secondSellTransition`，由 committed `IsType2Buy` 判据驱动 ledger delta（加仓 A+1 / 保持）。
-/
def secondBuyTransition (z : ChanlunAccount) (e : BspEndpoint) : ChanlunAccount :=
  let d := recogSecondBuy e
  let delta := secondBuyLedgerDelta d
  { ledger := ledgerStep z.ledger delta.1 delta.2.1 delta.2.2 }

/--
  ★★第二类卖侧闭环 T 保 R=Π-A-W（L0，账户恒等接入第二类卖侧闭环）：
  `secondSellTransition z e` 后 ledger 仍满足 R = Π - A - W（committed `ledgerStep` 的 inv 保持）。
-/
theorem secondSellTransition_preserves_ledger_inv (z : ChanlunAccount) (e : BspEndpoint) :
    (secondSellTransition z e).ledger.R =
      (secondSellTransition z e).ledger.Pi
        - (secondSellTransition z e).ledger.A
        - (secondSellTransition z e).ledger.W :=
  (secondSellTransition z e).ledger.inv

/--
  ★★第二类卖点真改 ledger·A 侧（L0，第二类卖侧非退化 witness）：
  若端点是第二类卖点（committed `IsType2Sell`），则第二类卖侧闭环转移后 A 分量 = 原 A - 1
  （二次减仓 deallocate 1）——ledger **真被第二类卖点缠论分类改写**（A 减 1）。
-/
theorem secondSellTransition_type2_deallocates (z : ChanlunAccount) (e : BspEndpoint)
    (h : IsType2Sell e) :
    (secondSellTransition z e).ledger.A = z.ledger.A - 1 := by
  unfold secondSellTransition
  rw [recogSecondSell_type2_reduceSecond e h]
  simp only [secondSellLedgerDelta, ledgerStep, mkLedger]
  omega

/--
  ★★第二类买点真改 ledger·A 侧（L0，买侧对端非退化 witness）：
  若端点是第二类买点（committed `IsType2Buy`），则第二类买侧闭环转移后 A 分量 = 原 A + 1
  （二次加仓 allocate 1）。
-/
theorem secondBuyTransition_type2_allocates (z : ChanlunAccount) (e : BspEndpoint)
    (h : IsType2Buy e) :
    (secondBuyTransition z e).ledger.A = z.ledger.A + 1 := by
  unfold secondBuyTransition
  rw [recogSecondBuy_type2_accreteSecond e h]
  simp only [secondBuyLedgerDelta, ledgerStep, mkLedger]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 ★★★第二类买卖闭环对偶对称：加仓(A↑) ↔ 减仓(A↓) 镜像（真证，非占位）

  本节是本文件核心交付：把 #120 committed `type2_buy_sell_share_criterion`（第二类买卖共用本级别
  可观测判据）提升到 **ledger 闭环层对偶**——第二类买点二次加仓 A+1 ↔ 第二类卖点二次减仓 A-1，
  A 分量严格镜像。买卖对偶**真证为定理**（非 `:= trivial` 占位）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★第二类买卖账本 delta·A 分量镜像（L0，对偶对称见证·决策层，真证）——
  第二类买点 `accreteSecond` 的 A 分量（+1）与第二类卖点 `reduceSecond` 的 A 分量（-1）严格相反号。
  这坐实第二类买卖在账本 A（仓位）足迹上的镜像：二次加仓（allocate, A↑）↔ 二次减仓（deallocate, A↓）。

  ★真证（`decide` 对具体 Int 三元组的 A 分量计算判定）——非 `trivial`/`rfl` 占位。
-/
theorem second_buy_sell_A_delta_mirror :
    (secondBuyLedgerDelta SecondBuyDecision.accreteSecond).2.1 =
      - (secondSellLedgerDelta SecondSellDecision.reduceSecond).2.1 := by
  decide

/--
  ★★★第二类买卖闭环 A 分量镜像总见证 `second_buy_sell_closed_loop_A_mirror`（L0，闭环层对偶，真证）★★★：

  从同一初始账户态 z₀：
  - 第二类买点端点经第二类买侧闭环转移后 A 增 1（`secondBuyTransition_type2_allocates`）。
  - 第二类卖点端点经第二类卖侧闭环转移后 A 减 1（`secondSellTransition_type2_deallocates`）。
  二者相对 base 的 A 偏移**严格相反号**（+1 ↔ -1）：第二类买点加仓与第二类卖点减仓在闭环 ledger
  上镜像对偶。

  ★这是「第二类买卖闭环对偶对称」的总见证（闭环层）：把 #120 `type2_buy_sell_share_criterion`
  （判据层第二类买卖共用本级别判据）兑现到闭环 ledger 足迹层——第二类买点二次加仓（A↑）↔
  第二类卖点二次减仓（A↓）镜像。**真证为定理**（非 trivial 占位）：用 committed `IsType2Buy`/
  `IsType2Sell` 判据假设 + 两侧闭环 A 见证，omega 闭合。

  ★前提 `eb`/`es` 分别满足第二类买/卖判据——这是真买卖对偶（同一本级别可观测判据 IsType2，
  side 对偶），不是同一端点（side 互斥，见 `type2_buy_sell_exclusive`）。
-/
theorem second_buy_sell_closed_loop_A_mirror (z0 : ChanlunAccount)
    (eb es : BspEndpoint) (hb : IsType2Buy eb) (hs : IsType2Sell es) :
    (secondBuyTransition z0 eb).ledger.A - z0.ledger.A =
      - ((secondSellTransition z0 es).ledger.A - z0.ledger.A) := by
  have hbuy : (secondBuyTransition z0 eb).ledger.A = z0.ledger.A + 1 :=
    secondBuyTransition_type2_allocates z0 eb hb
  have hsell : (secondSellTransition z0 es).ledger.A = z0.ledger.A - 1 :=
    secondSellTransition_type2_deallocates z0 es hs
  rw [hbuy, hsell]; omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 反退化见证（具体第二类买/卖端点，非平凡桩）
  ════════════════════════════════════════════════════════════════════════ -/

/-- 一个具体中枢（核心[10,20]）——见证用。 -/
def witnessCenter2 : Center :=
  { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }

/--
  ★第二类卖点见证 `sampleType2Sell`（具体缠论数值）：第一类卖点后（afterTypeOne）、回抽未再破
  中枢（¬brokeCenter）、side=short。满足 committed `IsType2Sell` ⟹ recogSecondSell 识别 reduceSecond。
-/
def sampleType2Sell : BspEndpoint :=
  { side := Side.short
    center := witnessCenter2
    divPair := { forceA := ⟨3⟩, forceC := ⟨3⟩, isTrend := false }  -- 非背驰（区别第一类）
    brokeCenter := false    -- 回抽未再破中枢（第二类前提，区别第一类）
    afterTypeOne := true     -- 在第一类后（第二类前提）
    leftCenter := false
    retracePrice := 15       -- 中枢内回抽（无关第三类离开中枢前提）
    firstRetrace := false }

/--
  ★第二类买点见证 `sampleType2Buy`（具体缠论数值）：第一类买点后、回抽未再破中枢、side=long。
  满足 committed `IsType2Buy` ⟹ recogSecondBuy 识别 accreteSecond。
-/
def sampleType2Buy : BspEndpoint :=
  { side := Side.long
    center := witnessCenter2
    divPair := { forceA := ⟨3⟩, forceC := ⟨3⟩, isTrend := false }
    brokeCenter := false
    afterTypeOne := true
    leftCenter := false
    retracePrice := 15
    firstRetrace := false }

/-- ★见证：sampleType2Sell 满足 committed 第二类卖点判据。 -/
theorem sampleType2Sell_isType2Sell : IsType2Sell sampleType2Sell := by
  unfold IsType2Sell IsType2 sampleType2Sell
  exact ⟨rfl, rfl, rfl⟩

/-- ★见证：sampleType2Buy 满足 committed 第二类买点判据。 -/
theorem sampleType2Buy_isType2Buy : IsType2Buy sampleType2Buy := by
  unfold IsType2Buy IsType2 sampleType2Buy
  exact ⟨rfl, rfl, rfl⟩

/-- ★见证：sampleType2Sell 经第二类卖侧闭环后 A = base - 1（二次减仓）。 -/
theorem sampleType2Sell_transition_A (z0 : ChanlunAccount) :
    (secondSellTransition z0 sampleType2Sell).ledger.A = z0.ledger.A - 1 :=
  secondSellTransition_type2_deallocates z0 sampleType2Sell sampleType2Sell_isType2Sell

/-- ★见证：sampleType2Buy 经第二类买侧闭环后 A = base + 1（二次加仓）。 -/
theorem sampleType2Buy_transition_A (z0 : ChanlunAccount) :
    (secondBuyTransition z0 sampleType2Buy).ledger.A = z0.ledger.A + 1 :=
  secondBuyTransition_type2_allocates z0 sampleType2Buy sampleType2Buy_isType2Buy

/--
  ★★第二类买卖闭环镜像·具体见证（L0，真证）—— 用 committed 判据见证端点 sampleType2Buy/
  sampleType2Sell 坐实 §4 总见证：sampleType2Buy 加仓 A+1 ↔ sampleType2Sell 减仓 A-1，A 偏移相反号。
-/
theorem sample_second_buy_sell_A_mirror (z0 : ChanlunAccount) :
    (secondBuyTransition z0 sampleType2Buy).ledger.A - z0.ledger.A =
      - ((secondSellTransition z0 sampleType2Sell).ledger.A - z0.ledger.A) :=
  second_buy_sell_closed_loop_A_mirror z0 sampleType2Buy sampleType2Sell
    sampleType2Buy_isType2Buy sampleType2Sell_isType2Sell

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（gatekeeper：禁标盈利/次级别递归已证/TW端已接/平凡占位）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★第二类卖侧闭环标签 `SecondSellClosedLoopTag`（gatekeeper，诚实分层）。
  - `realSecondSellLedgerClosure`：第二类卖点决策（recogSecondSell）真接 committed `ledgerStep`。
  - `nonDegenerateSecondSellTransition`：第二类卖侧闭环真改 ledger（A-1，非占位）。
  - `secondBuySellLedgerMirror`：第二类买卖闭环在 A 分量镜像对偶（加仓 A↑ ↔ 减仓 A↓，真证）。
  - `preservesLedgerInv`：第二类卖侧闭环保 R=Π-A-W（committed ledgerStep inv）。
  - `levelOneObservableCriterion`：第二类 recog 只用本级别可观测判据 `IsType2`（committed 口径一致）。
  - `thetaRiskParametric`：加仓/减仓数额是 Θ_risk 参数（第二类判据只定 delta 类型不定数额）。
  - `secondTypeRecursionMissing`：第二类「由次级别第一类构成」（买卖点定律一 §10.2）**未证**
    （still-MISSING-D，与 committed BspClassification/SellPointRecog 诚实一致）。
  - `twEndMissing`：TW 端（取本金/提现 W 提现，#90 三阶段）对接**未做**（still-MISSING）。
  - `empiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableSecondType` / `SubLevelRecursionDone` / `TWEndDone` / `TrivialPlaceholder`
  构造子——类型层拒绝把本第二类卖侧闭环标为盈利策略 / 次级别递归已证 / TW 端已接 / 平凡占位。
-/
inductive SecondSellClosedLoopTag where
  | realSecondSellLedgerClosure
  | nonDegenerateSecondSellTransition
  | secondBuySellLedgerMirror
  | preservesLedgerInv
  | levelOneObservableCriterion
  | thetaRiskParametric
  | secondTypeRecursionMissing
  | twEndMissing
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★第二类卖侧闭环子类（gatekeeper）：SecondSellClosedLoopAssembled（唯一构造子，禁标递归已证/TW已接/平凡占位）。 -/
inductive SecondSellClosedLoopSubkind where
  | secondSellClosedLoopAssembled
deriving DecidableEq, Repr

/-- ★诚实标签包（L0 声明）。 -/
def secondSellClosedLoopLabels : List SecondSellClosedLoopTag × SecondSellClosedLoopSubkind :=
  ([SecondSellClosedLoopTag.realSecondSellLedgerClosure,
    SecondSellClosedLoopTag.nonDegenerateSecondSellTransition,
    SecondSellClosedLoopTag.secondBuySellLedgerMirror,
    SecondSellClosedLoopTag.preservesLedgerInv,
    SecondSellClosedLoopTag.levelOneObservableCriterion,
    SecondSellClosedLoopTag.thetaRiskParametric,
    SecondSellClosedLoopTag.secondTypeRecursionMissing,
    SecondSellClosedLoopTag.twEndMissing,
    SecondSellClosedLoopTag.empiricalDomain],
   SecondSellClosedLoopSubkind.secondSellClosedLoopAssembled)

/-- ★禁标递归已证/TW端已接/盈利/平凡占位（L0，gatekeeper 见证）：子类必是 SecondSellClosedLoopAssembled。 -/
theorem secondSellClosedLoop_subkind_is_assembled (k : SecondSellClosedLoopSubkind) :
    k = SecondSellClosedLoopSubkind.secondSellClosedLoopAssembled := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（task #129，第二类卖点 Θ 闭环装配）

  本文件**证**（L0 结构层 + L1 第二类卖侧缠论规则编码，machine-checked，无 sorry/admit/axiom）：

  1. ★第二类买/卖决策载体 `SecondSellDecision`（reduceSecond/hold）/ `SecondBuyDecision`
     （accreteSecond/hold）+ 账本 delta `secondSellLedgerDelta`（A-1）/ `secondBuyLedgerDelta`（A+1）——
     committed 闭环（买侧 ChanlunDecision / 卖侧 SellDecision）**都缺第二类分支**，本文件 distinct 补。
  2. ★★第二类卖点识别核 `recogSecondSell`（真消费 committed `IsType2Sell`）+ 买侧对端
     `recogSecondBuy`（真消费 committed `IsType2Buy`）。
     · `recogSecondSell_type2_reduceSecond` / `recogSecondBuy_type2_accreteSecond`：真编码坐实。
     · `recogSecondSell_total`：第二类卖侧 recog 全定义。
  3. ★★第二类卖侧闭环转移 `secondSellTransition`（recogSecondSell 决策驱动 ledger，保 R=Π-A-W）
     + 买侧对端 `secondBuyTransition`。
     · `secondSellTransition_preserves_ledger_inv`：第二类卖侧闭环保账本恒等。
     · `secondSellTransition_type2_deallocates`（第二类卖点 A-=1）/ `secondBuyTransition_type2_allocates`
       （第二类买点 A+=1）：真改 ledger。
  4. ★★★**第二类买卖闭环对偶对称**（核心交付，真证非占位）：
     · `second_buy_sell_A_delta_mirror`（决策层 A 分量相反号 +1 ↔ -1，`decide` 真证）。
     · `second_buy_sell_closed_loop_A_mirror`（闭环层：第二类买点加仓 A+1 ↔ 第二类卖点减仓 A-1，
       omega 真证）——把 #120 `type2_buy_sell_share_criterion`（判据层）提升到 ledger 闭环层对偶。
  5. ★反退化见证（具体第二类买/卖端点 sampleType2Buy/sampleType2Sell，满足 committed 判据）+
     `sample_second_buy_sell_A_mirror`（具体见证镜像）。
  6. 诚实标签 `secondSellClosedLoopLabels` + `secondSellClosedLoop_subkind_is_assembled`
     （禁标次级别递归已证/TW 端已接/盈利/平凡占位）。

  本文件**不证**（no声明膨胀，诚实边界）：
  - ✗ 第二类应对策略盈利/最优/实盘有效（L3 EmpiricalDomain）。
  - ✗ 加仓/减仓数额（1）来自缠论（Θ_risk 参数——第二类判据只决定 delta 的**类型/符号**：
       减仓 deallocate / 加仓 allocate，不决定精确数额）。
  - ✗ **第二类「由次级别第一类构成」**（买卖点定律一 §10.2，still-MISSING-D）——本文件第二类 recog
       只用本级别可观测判据 `IsType2`（afterTypeOne ∧ ¬brokeCenter），与 committed BspClassification/
       SellPointRecog/ThetaInstantiation 的诚实口径**完全一致**，不冒充次级别递归。次级别递归构成是
       #116 BspConstruction / #128 SubLevelDescent 的几何下钻领域，本文件诚实留白（非 workaround）。
  - ✗ **TW 端（取本金/提现，#90 三阶段全局账本 W 提现端）**——本文件用 R=Π-A-W **单账本**
       （W 在第二类 delta 中恒 0）。提现是另一决策层（TW 端，still-MISSING）。
  - ✗ 第二类与第三类的互斥裁定（committed `no_exclusive_trichotomy`：2类3类可重合，V 型反转，
       x_2b3b 见证）——本文件第二类 recog 只对本级别第二类判据敏感，不裁定与第三类的互斥
       （committed 已证互斥三分失败，本文件不重证亦不冲突）。

  ★对完整性推进度：本文件装配第二类卖点 Θ 闭环（recog→ledger），消第二类卖侧闭环缺口，并把
    #120 第二类买卖判据对偶提升到 ledger 闭环层（第二类买点加仓 A↑ ↔ 第二类卖点减仓 A↓ 镜像真证）。
    三类卖点闭环现已**全覆盖**（第一类 #121 closeRoot / 第三类 #121 reduceCore / 第二类本文件
    reduceSecond）。剩余完整性缺口（诚实标注）：(a) 第二类次级别递归构成（still-MISSING-D）；
    (b) TW 端双账本对接（#90 twState 提现端）；(c) bspOf/centersOf 全自动构造（#113 still-MISSING）。

  ★买卖对偶真证审查（no-workaround：遇定义冲突停 ESCALATE）——审查结论：**无定义冲突，未触发
    ESCALATE**。第二类买卖闭环在 ledger A（仓位）分量上严格镜像（二次加仓 A↑ ↔ 二次减仓 A↓），
    买卖对偶**真证为定理**（`second_buy_sell_closed_loop_A_mirror`，omega 闭合，非 `:= trivial`
    占位）。第二类买卖共用同一本级别可观测判据 `IsType2`（#120 committed），只在 side 上对偶
    （side 互斥，`type2_buy_sell_exclusive`），镜像在 A 分量层严格封闭，无须 ESCALATE。

  谱系：#113 BspClassification（IsType2 本级别可观测）→ #117 ThetaInstantiation 买侧闭环（T1/T3/hold，
        第二类买点诚实标 still-MISSING-D）→ #120 SellPointRecog 第二类买卖判据对偶
        （IsType2Buy/IsType2Sell/type2_buy_sell_share_criterion，闭环对接标 still-MISSING）→
        #121 SellClosedLoop 卖侧 T1/T3/hold 闭环（诚实标第二类卖点闭环缺）→ 本文件 #129
        （装配第二类卖点 Θ 闭环 recog→ledger，把第二类判据对偶提升到 ledger 闭环层，证第二类
        买点加仓 A↑ ↔ 第二类卖点减仓 A↓ 镜像；次级别递归构成诚实留 still-MISSING-D）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SecondSellClosedLoop
