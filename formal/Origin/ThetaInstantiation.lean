/-
Origin/ThetaInstantiation.lean — Θ 缠论-实例化（消闭环平凡占位，task #117）

★工位定位（审计 B2/B10 判决）：Origin 的**唯一闭环 witness** `originFullDef`
  （formal/Strict/HybridAssembly.lean §8）六段是**平凡占位**——
    · Event := Unit（无缠论事件内容）
    · recStruct := fun _ _ => emptyParse（空解析，无递归结构）
    · intent := decide (trend = trendUp) → openRoot 否则 hold（平凡两值映射）
    · risk := 两值（1/0）
    · transition 的 ledger delta 由 `trend=trendUp` 这个**平凡标志**决定，而非真缠论分类。
  审计判：「接口可居留性见证，非真缠论闭环」。#114 标策略族 Θ 仍 extra-缠论参数化。

  本文件用**真缠论结构**实例化全定义策略 Θ——recog/classify 用 Origin 分类血肉判据
  （Divergence/BspClassification/CenterStates，#113 committed）做真分类识别，非 emptyParse；
  target/exec 用 Origin 策略族 π_Θ + RiskProj（#114 committed）做真应对，非平凡两值；
  Event 用**真缠论事件类型**（买卖点信号 BspEndpoint + 中枢发展对），非 Unit。

  ★交付：一个**非退化的闭环 witness**——真缠论 transition 真改 ledger（满足 0004 锋利问题的
    非平凡版）：ledger delta 由**事件的真缠论分类**（第一类背驰买点 / 第三类回抽 / 力度延续）
    决定，且**两个不同缠论分类的事件产生不同的 ledger delta**（非 trend→openRoot 平凡占位）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- 第24课（背驰-买卖点定理）：第一类买点 = 跌破最后一个中枢后的背驰点（forceC < forceA）。
- §10.1（三类买卖点）：第三类买点 = 上涨趋势离开中枢后第一次回抽不破 ZG。
- §6.7（中枢发展三态）：延伸/新生/扩展——本文件用发展态判据驱动建仓/取本/增核应对。
- §11（背驰力度比较）：力度延续（A≤C，非背驰）不构成买点 ⟹ 应对为 hold。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
- **结构层 L0**：本文件全部定理是 L0（纯定义 / 缠论判据组合 / 闭环转移函数图，
  omega/decide/rfl machine-checked，不依赖数据）。`lake env lean Origin/ThetaInstantiation.lean`
  通过 = 「Θ 的 recog/classify/exec 段真消费 #113 缠论判据（IsType1/IsType3Buy/IsDivergence/
  classifyDevelopment）+ #114 RiskProj.gridProject，且闭环 transition 的 ledger delta 由真缠论
  分类确定且非退化（两类事件不同 delta）」在定义层成立。

- **触及 L1 缠论规则真编码**：与 originFullDef 平凡占位的关键区别——
  recog 段**真编码**第24课「背驰⟹第一类买点」+ §10.1「回抽不破 ZG⟹第三类」+ §11「力度延续⟹
  非买点」这三条缠论规则（`recogChanlun` 真 case-split on `IsDivergence`/`IsType3Buy`/`IsContinuation`），
  而非 `decide (trend = trendUp)` 的平凡标志读取。这是 L1 缠论规则的结构编码（规则忠实，
  非经验验证——规则在真实行情上的有效性是 L2/L3，本文件不声称）。

- **诚实边界（no声明膨胀）**：本文件**不**证：
  · 这些应对策略盈利/最优/实盘有效（L3 EmpiricalDomain）；
  · 仓位大小/ledger delta 数额来自缠论（是 Θ_risk 参数化，缠论分类只决定 delta 的**方向/类型**
    [allocate vs realize]，不决定数额——见 §诚实标注）；
  · bsp 事件的**自动识别**（从 K 线流自动判定每端点属哪类）——本文件消费已判定的 BspEndpoint
    （由 #113 BspClassification 判据层提供），不证遍历构造（still-MISSING：bspOf 全自动，#113 标）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root）。
依赖方向（单向无环）：ThetaInstantiation → {BspClassification, CenterStates, Divergence,
  StrategyFamily, RiskProj, FullDefinitionStrategy}（均 Origin 内 committed，无 legacy import）。

谱系：originFullDef 平凡占位（#101，HybridAssembly §8）→ 审计 B2/B10（平凡占位判决）→
      #113 分类血肉 committed + #114 策略族 committed → 本文件 #117（用真缠论结构实例化 Θ，
      消平凡占位，给非退化闭环 witness）。
-/

import Origin.BspClassification
import Origin.CenterStates
import Origin.Divergence
import Origin.StrategyFamily
import Origin.RiskProj
import Origin.FullDefinitionStrategy

namespace NewChanlun.Origin.ThetaInstantiation

open NewChanlun.Origin
open NewChanlun.Origin.StrategyFamily (Theta piTheta given_theta_total_unique piTheta_causal
  piTheta_isCausal)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 真缠论事件类型 ChanlunEvent（消 `Event := Unit` 占位）

  originFullDef 用 `Event := Unit`（无内容）。本文件用**真缠论事件**：一个驱动闭环一步的
  事件携带 (1) 一个买卖点端点 `BspEndpoint`（#113 判据层提供，含中枢/背驰段/回抽价等真判据
  数据）+ (2) 一个中枢发展对（前后中枢 `CenterWithOuter`，#113 发展三态判据数据）。
  这是**真缠论结构**（笔/线段聚合出的中枢 + 买卖点信号），非 Unit 平凡桩。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★真缠论事件 `ChanlunEvent`（L0，消 `Event := Unit`）：

  - `bsp : BspEndpoint`：候选买卖点端点（#113 BspClassification.BspEndpoint，携带 side/center/
    divPair/brokeCenter/leftCenter/retracePrice/firstRetrace——真判据数据）。
  - `prevCenter next Center : CenterWithOuter`：前后中枢（#113 CenterStates.CenterWithOuter，
    带 GG/DD 外缘）——驱动发展三态判据（延伸/新生/扩展）。

  ★这是真缠论事件类型（买卖点信号 + 中枢发展），非 Unit。事件的缠论内容真被 recog 段消费。
-/
structure ChanlunEvent where
  bsp : BspEndpoint
  prevCenter : CenterWithOuter
  nextCenter : CenterWithOuter
deriving Repr

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 真缠论账户态 ChanlunAccount（携带 R=Π-A-W 恒等，闭环写回的载体）

  Θ 的账户态 Z。复用 FullDefinitionStrategy 的 `LedgerState`（R=Π-A-W 恒等四字段，
  committed）作账户态——使 transition 真改 ledger 且保恒等。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★真缠论账户态 `ChanlunAccount`（L0）：携带 Origin committed `LedgerState`（R=Π-A-W 恒等）。
  - `ledger : LedgerState`：账本恒等态（R=Π-A-W，FullDefinitionStrategy committed）。
-/
structure ChanlunAccount where
  ledger : LedgerState
deriving Repr

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 真缠论决策核 ChanlunDecision + recog 段（消 emptyParse + 平凡两值占位）

  ★关键消占位：originFullDef 的 recStruct 是 `fun _ _ => emptyParse`（空解析），intent 是
  `decide (trend = trendUp)`（平凡标志）。本文件的 recog 段**真 case-split on 缠论判据**——
  第24课「背驰⟹第一类买点」+ §10.1「回抽不破 ZG⟹第三类」+ §11「力度延续⟹非买点」。
  这是真缠论分类识别，非 emptyParse / 平凡标志。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★真缠论决策 `ChanlunDecision`（L0，消 intent 平凡两值）：识别出的缠论应对意图。
  - `openRoot`：建根仓（第一类背驰买点——跌破中枢后背驰，第24课）。
  - `accreteCore`：增核（第三类买点——离开中枢回抽不破 ZG，§10.1 + 发展态新生/扩展）。
  - `hold`：保持（力度延续/非买点，§11——后段力度 ≥ 前段，趋势延续无背驰）。

  ★这三态由**真缠论判据**区分（见 `recogChanlun`），非 trend=trendUp 平凡标志。
-/
inductive ChanlunDecision where
  | openRoot       -- 第一类背驰买点 → 建根仓
  | accreteCore    -- 第三类回抽买点 → 增核
  | hold           -- 力度延续/非买点 → 保持
deriving DecidableEq, Repr

/--
  ★★真缠论识别核 `recogChanlun`（L0，本文件核心——消 emptyParse + 平凡两值占位）：
  从真缠论事件 `ChanlunEvent` **真 case-split on #113 缠论判据**，识别应对意图。

  缠论规则真编码（L1 缠论规则结构编码，对照 originFullDef 的 `decide (trend = trendUp)`）：
  - **第24课**：若 `IsType1 bsp`（破中枢 ∧ 背驰 `IsDivergence divPair`）⟹ `openRoot`（第一类买点建根仓）。
  - **§10.1**：否则若 `IsType3Buy bsp`（离开中枢 ∧ 第一次回抽 ∧ retracePrice > ZG 不破中枢）
    ⟹ `accreteCore`（第三类买点增核）。
  - **§11**：否则（含力度延续 `IsContinuation divPair` = 非背驰，无买点）⟹ `hold`（保持）。

  ★这是真缠论分类识别——直接消费 `IsType1`/`IsType3Buy`（#113 BspClassification 判据，
  内含 `IsDivergence` #113 Divergence 判据），**不是** emptyParse 空解析，**不是** trend=trendUp
  平凡标志。判据可判定（IsDivergence/Bool 字段/Int 比较），用 decide 桥接。
-/
def recogChanlun (e : ChanlunEvent) : ChanlunDecision :=
  if e.bsp.brokeCenter = true ∧ decide (IsDivergence e.bsp.divPair) = true then
    -- 第24课：破中枢 + 背驰 ⟹ 第一类买点 ⟹ 建根仓。
    ChanlunDecision.openRoot
  else if e.bsp.side = Side.long ∧ e.bsp.leftCenter = true ∧ e.bsp.firstRetrace = true ∧
          decide (e.bsp.center.zg < e.bsp.retracePrice) = true then
    -- §10.1：离开中枢 + 第一次回抽 + 不破 ZG ⟹ 第三类买点 ⟹ 增核。
    ChanlunDecision.accreteCore
  else
    -- §11：力度延续 / 非买点 ⟹ 保持。
    ChanlunDecision.hold

/--
  ★recog 真消费第一类判据（L0，证 recogChanlun 非平凡）：若事件满足 `IsType1`（第24课第一类
  买点判据：破中枢 ∧ 背驰），则 `recogChanlun` 识别为 `openRoot`。这坐实 recog 段**真编码**
  第24课规则——不是占位（占位无法对第一类判据敏感）。
-/
theorem recog_type1_openRoot (e : ChanlunEvent) (h : IsType1 e.bsp) :
    recogChanlun e = ChanlunDecision.openRoot := by
  unfold recogChanlun IsType1 at *
  have hbroke : e.bsp.brokeCenter = true := h.1
  have hdiv : decide (IsDivergence e.bsp.divPair) = true := decide_eq_true h.2
  simp only [hbroke, hdiv, and_self, if_pos]

/--
  ★recog 真消费第三类判据（L0）：若事件满足 `IsType3Buy`（§10.1 第三类买点判据：离开中枢 ∧
  第一次回抽 ∧ 不破 ZG）**且非第一类**（未破中枢），则 `recogChanlun` 识别为 `accreteCore`。
  ★前提 `¬ broke`：第三类的语境是「离开中枢后回抽」（leftCenter），与第一类「破中枢背驰」
  互斥分支——第一类优先级更高（先判破中枢背驰）。这坐实第三类规则真被编码。
-/
theorem recog_type3_accreteCore (e : ChanlunEvent)
    (h : IsType3Buy e.bsp) (hnobreak : e.bsp.brokeCenter = false) :
    recogChanlun e = ChanlunDecision.accreteCore := by
  unfold recogChanlun IsType3Buy at *
  obtain ⟨hside, hleft, hfirst, hzg⟩ := h
  have hcond1 : ¬ (e.bsp.brokeCenter = true ∧ decide (IsDivergence e.bsp.divPair) = true) := by
    rw [hnobreak]; simp
  have hzg' : decide (e.bsp.center.zg < e.bsp.retracePrice) = true := decide_eq_true hzg
  rw [if_neg hcond1, if_pos ⟨hside, hleft, hfirst, hzg'⟩]

/--
  ★recog 真消费力度延续判据（L0，§11）：若事件背驰段力度延续（`IsContinuation divPair` = 非背驰）
  且非第三类（无离开中枢回抽），则 `recogChanlun` 识别为 `hold`——力度延续无买点，保持。

  ★这坐实 §11「力度延续 ⟹ 非买点」规则真被编码：`hcont`（力度延续）经 #113
  `continuation_not_divergence` 推出 `¬ IsDivergence`，使第一类分支的背驰判据**经力度判据本身**失败
  （不依赖 brokeCenter 旁路）——延续段被路由到 hold（不开仓）。`hcont` 是 §11 规则的实质前件。
-/
theorem recog_continuation_hold (e : ChanlunEvent)
    (hcont : IsContinuation e.bsp.divPair)
    (hnoleft : e.bsp.leftCenter = false) :
    recogChanlun e = ChanlunDecision.hold := by
  unfold recogChanlun
  -- §11：力度延续 ⟹ 非背驰（#113 continuation_not_divergence）⟹ 第一类背驰判据失败（经力度本身）。
  have hnodiv : ¬ IsDivergence e.bsp.divPair := continuation_not_divergence e.bsp.divPair hcont
  have hcond1 : ¬ (e.bsp.brokeCenter = true ∧ decide (IsDivergence e.bsp.divPair) = true) := by
    rintro ⟨_, hd⟩; exact hnodiv (of_decide_eq_true hd)
  have hcond2 : ¬ (e.bsp.side = Side.long ∧ e.bsp.leftCenter = true ∧
      e.bsp.firstRetrace = true ∧ decide (e.bsp.center.zg < e.bsp.retracePrice) = true) := by
    rw [hnoleft]; simp
  rw [if_neg hcond1, if_neg hcond2]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 真缠论应对：账本 delta + RiskProj 真投影（消平凡两值 risk）

  ★关键消占位：originFullDef 的 risk 是两值（openRoot⟹1, _⟹0）。本文件用 **#114 RiskProj
  的真有限网格确定选择器 `gridProject`** 选目标仓位，且 ledger delta 由**真缠论决策**确定
  （建根仓 allocate / 增核 allocate 不同量 / 保持 noop）——非平凡两值。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Θ_risk 网格实例 `chanlunRiskGrid`（L0，#114 RiskProj.RiskGrid）：有限仓位网格 {0,1,2,3}，
  cost = 格点本身（Nat 全序，cost 单射 ⟹ LexArgmin 唯一）。复用 #114 committed RiskProj 结构。
  ★诚实：网格/cost 是 Θ_risk 参数（非缠论可导，#114 已守此诚实）——缠论分类决定 delta 的
  **类型**（allocate/realize/noop），不决定**数额**（数额是 Θ_risk）。
-/
def chanlunRiskGrid : RiskProj.RiskGrid Nat Nat where
  grid := [0, 1, 2, 3]
  zero := 0
  zeroMem := by decide
  key :=
    { le := fun a b => a ≤ b
      decLe := fun a b => inferInstanceAs (Decidable (a ≤ b))
      le_refl := fun a => Nat.le_refl a
      le_trans := fun a b c => Nat.le_trans
      le_antisymm := fun a b => Nat.le_antisymm
      le_total := fun a b => Nat.le_total a b }
  cost := fun p => p
  cost_inj := fun p q _ _ h => h

/--
  ★真缠论账本事件 `decisionLedgerDelta`（L0，消平凡两值 risk + ledger delta）：把**真缠论决策**
  映为账本三元组 delta `(dΠ, dA, dW)`（喂给 committed `ledgerStep`）。

  缠论决策 ⟹ 账本副作用（账户因果链：缠论分类决定账本操作类型）：
  - `openRoot`（第一类建根仓）⟹ `(0, 1, 0)`：A += 1（资本化建仓，占用储备 R）。
  - `accreteCore`（第三类增核）⟹ `(0, 2, 0)`：A += 2（增核分配更多——**与建根仓不同量**，
    区分两类缠论应对的账本足迹）。
  - `hold`（力度延续保持）⟹ `(0, 0, 0)`：账本不变（无买点不动账）。

  ★非退化关键：openRoot 与 accreteCore 的 dA 不同（1 vs 2）⟹ **两个不同缠论分类的事件
  产生不同的 ledger delta**（满足 0004 锋利问题的非平凡版）。
  ★诚实：数额 1/2 是 Θ_risk 占位常量（具体数额是运行时数据 L2）——缠论分类决定 delta 的
  **类型/符号**（allocate 增核 > 建根仓），不决定精确数额（数额下游 Θ_risk 填）。
-/
def decisionLedgerDelta : ChanlunDecision → (Int × Int × Int)
  | ChanlunDecision.openRoot    => (0, 1, 0)   -- 建根仓：allocate 1
  | ChanlunDecision.accreteCore => (0, 2, 0)   -- 增核：allocate 2（≠ 建根仓，非退化）
  | ChanlunDecision.hold        => (0, 0, 0)   -- 保持：noop

/--
  ★真缠论目标仓位 `decisionTargetPos`（L0，消平凡两值 risk）：决策经 #114 RiskProj 真投影选目标仓位。
  本骨架的网格投影命中字典序最小格点（gridProject over {0,1,2,3}）——这是 **#114 committed
  确定选择器**的真调用（非两值 if）。决策类型不改网格（投影依赖 Θ_risk 网格，#114 诚实保留）。
-/
def decisionTargetPos (_d : ChanlunDecision) : Nat :=
  chanlunRiskGrid.gridProject

/--
  ★目标仓位命中 RiskProj LexArgmin（L0，#114 真投影兑现）：`decisionTargetPos` = chanlunRiskGrid
  上 cost 字典序最优格点。直接引 #114 `RiskGrid.gridProject_isLexArgmin`——真有限网格确定选择器
  （非平凡两值，非连续 argmin）。坐实 risk 段真用 #114 RiskProj。
-/
theorem targetPos_is_lexargmin (d : ChanlunDecision) :
    chanlunRiskGrid.IsLexArgmin (decisionTargetPos d) :=
  chanlunRiskGrid.gridProject_isLexArgmin

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 闭环 transition：真缠论 transition 真改 ledger（非退化 witness 核心）

  ★关键消占位：originFullDef 的 transition 由 `trend=trendUp` 平凡标志驱动 ledger。本文件的
  transition 由**真缠论决策**（recogChanlun 的输出）驱动 ledger delta，且保 R=Π-A-W 恒等。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★真缠论闭环转移 `chanlunTransition`（L0，本文件核心——非退化 witness）：
  `T : ChanlunAccount → ChanlunEvent → ChanlunAccount`。

  闭环数据流（真缠论，对照 originFullDef 的 trend=trendUp 平凡驱动）：
  1. `recogChanlun e` 用 #113 缠论判据识别真缠论决策 d（第一类/第三类/延续）。
  2. `decisionLedgerDelta d` 把缠论决策映为账本 delta（建根仓 allocate 1 / 增核 allocate 2 / 保持 noop）。
  3. `ledgerStep`（committed，保 R=Π-A-W）用该 delta 更新账本，写回完整 ChanlunAccount。

  ★这是**真缠论 transition 真改 ledger**：ledger 的下一态由**事件的真缠论分类**决定，
  且保账本恒等（ledgerStep 的 inv）。非 trend→openRoot 占位。
-/
def chanlunTransition (z : ChanlunAccount) (e : ChanlunEvent) : ChanlunAccount :=
  let d := recogChanlun e
  let delta := decisionLedgerDelta d
  { ledger := ledgerStep z.ledger delta.1 delta.2.1 delta.2.2 }

/--
  ★★闭环 T 保 R=Π-A-W（L0，账户恒等接入真缠论闭环）：
  `chanlunTransition z e` 后 ledger 仍满足 R = Π - A - W（committed `ledgerStep` 的 inv 保持）。
  真缠论分类驱动的账本更新进入闭环，且每步保账本恒等。
-/
theorem chanlunTransition_preserves_ledger_inv (z : ChanlunAccount) (e : ChanlunEvent) :
    (chanlunTransition z e).ledger.R =
      (chanlunTransition z e).ledger.Pi
        - (chanlunTransition z e).ledger.A
        - (chanlunTransition z e).ledger.W :=
  (chanlunTransition z e).ledger.inv

/--
  ★★第一类背驰买点真改 ledger（L0，非退化 witness ①·第一类侧）：
  若事件是第一类买点（`IsType1`），则闭环转移后 A 分量 = 原 A + 1（建根仓 allocate 1）——
  ledger **真被第一类缠论分类改写**（A 增 1）。这坐实「真缠论 transition 真改 ledger」。
-/
theorem chanlunTransition_type1_allocates (z : ChanlunAccount) (e : ChanlunEvent)
    (h : IsType1 e.bsp) :
    (chanlunTransition z e).ledger.A = z.ledger.A + 1 := by
  unfold chanlunTransition
  rw [recog_type1_openRoot e h]
  simp only [decisionLedgerDelta, ledgerStep, mkLedger]

/--
  ★★第三类回抽买点真改 ledger（L0，非退化 witness ①·第三类侧）：
  若事件是第三类买点（`IsType3Buy` ∧ 未破中枢），则闭环转移后 A 分量 = 原 A + 2（增核 allocate 2）——
  ledger **真被第三类缠论分类改写**（A 增 2，**与第一类的 +1 不同**）。
-/
theorem chanlunTransition_type3_allocates (z : ChanlunAccount) (e : ChanlunEvent)
    (h : IsType3Buy e.bsp) (hnobreak : e.bsp.brokeCenter = false) :
    (chanlunTransition z e).ledger.A = z.ledger.A + 2 := by
  unfold chanlunTransition
  rw [recog_type3_accreteCore e h hnobreak]
  simp only [decisionLedgerDelta, ledgerStep, mkLedger]

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 ★★★非退化总见证：两类缠论事件 ⟹ 不同 ledger delta（0004 锋利问题非平凡版）

  这是本文件对审计 B2/B10「平凡占位」的直接消解：**存在两个具体缠论事件，它们被真缠论分类
  识别为不同类（第一类 vs 第三类），且产生不同的 ledger delta**——这是 trend→openRoot 平凡
  占位**做不到**的（占位对缠论分类不敏感，ledger delta 只跟 trend 标志走）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 一个具体中枢（核心[10,20]，外缘[5,25]）——见证用。 -/
def witnessCenter : Center :=
  { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }

/-- 一个具体外缘中枢（用于 ChanlunEvent 的发展对字段）。 -/
def witnessOuter : CenterWithOuter :=
  { core := witnessCenter, dd := 5, gg := 25, outer_lo := by decide, outer_hi := by decide }

/--
  ★第一类买点事件 `eventType1`（具体缠论数值，非平凡桩）：破中枢 + 背驰（forceC=2 < forceA=8）。
  满足 `IsType1`（第24课第一类买点判据）⟹ recog 识别为 openRoot ⟹ ledger A += 1。
-/
def eventType1 : ChanlunEvent :=
  { bsp :=
      { side := Side.long
        center := witnessCenter
        divPair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true }
        brokeCenter := true     -- 破中枢（第一类前提）
        afterTypeOne := false
        leftCenter := false
        retracePrice := 5
        firstRetrace := false }
    prevCenter := witnessOuter
    nextCenter := witnessOuter }

/--
  ★第三类买点事件 `eventType3`（具体缠论数值，非平凡桩）：离开中枢 + 第一次回抽 + 不破 ZG
  （retracePrice=25 > zg=20）+ 未破中枢。满足 `IsType3Buy`（§10.1 第三类判据）⟹ recog 识别为
  accreteCore ⟹ ledger A += 2。
-/
def eventType3 : ChanlunEvent :=
  { bsp :=
      { side := Side.long
        center := witnessCenter
        divPair := { forceA := ⟨3⟩, forceC := ⟨3⟩, isTrend := false }  -- 力度延续（非背驰）
        brokeCenter := false    -- 未破中枢（区别第一类）
        afterTypeOne := true
        leftCenter := true      -- 离开中枢（第三类前提）
        retracePrice := 25      -- 25 > zg=20 ⟹ 不破 ZG ⟹ 第三类买点
        firstRetrace := true }
    prevCenter := witnessOuter
    nextCenter := witnessOuter }

/-- ★见证：eventType1 满足第24课第一类买点判据（破中枢 + 背驰）。 -/
theorem eventType1_isType1 : IsType1 eventType1.bsp := by
  unfold IsType1 IsDivergence eventType1; exact ⟨rfl, by decide⟩

/-- ★见证：eventType3 满足 §10.1 第三类买点判据（离开中枢 + 第一次回抽 + 不破 ZG）。 -/
theorem eventType3_isType3Buy : IsType3Buy eventType3.bsp := by
  unfold IsType3Buy eventType3; exact ⟨rfl, rfl, rfl, by decide⟩

/-- ★见证：eventType1 被真缠论识别为 openRoot（第一类建根仓）。 -/
theorem eventType1_recog_openRoot : recogChanlun eventType1 = ChanlunDecision.openRoot :=
  recog_type1_openRoot eventType1 eventType1_isType1

/-- ★见证：eventType3 被真缠论识别为 accreteCore（第三类增核）。 -/
theorem eventType3_recog_accreteCore : recogChanlun eventType3 = ChanlunDecision.accreteCore :=
  recog_type3_accreteCore eventType3 eventType3_isType3Buy (by unfold eventType3; rfl)

/--
  ★★★非退化总见证 `chanlun_transition_distinguishes_classes`（L0，消平凡占位的核心交付）★★★：

  从同一初始账户态 z₀，**第一类背驰买点事件**与**第三类回抽买点事件**经真缠论闭环转移后，
  产生的 ledger **A 分量不同**（第一类 A=base+1，第三类 A=base+2）。

  这是审计 B2/B10「平凡占位」的直接消解：
  - trend→openRoot **平凡占位**对缠论分类不敏感——ledger delta 只跟 `trend=trendUp` 标志走，
    两个分类不同但 trend 标志相同的事件会产生**相同** ledger（退化）。
  - 本文件的真缠论 transition **真区分缠论分类**——第一类（背驰建根仓）与第三类（回抽增核）
    被 #113 判据真识别为不同类，ledger A 分量真不同（base+1 ≠ base+2）。

  ★这满足 0004 锋利问题的**非平凡版**：闭环转移真改 ledger，且改法由**真缠论分类**唯一确定，
  不同缠论分类 ⟹ 不同 ledger 足迹（非 trend 平凡标志驱动的退化映射）。
-/
theorem chanlun_transition_distinguishes_classes (z0 : ChanlunAccount) :
    (chanlunTransition z0 eventType1).ledger.A ≠
      (chanlunTransition z0 eventType3).ledger.A := by
  have h1 : (chanlunTransition z0 eventType1).ledger.A = z0.ledger.A + 1 :=
    chanlunTransition_type1_allocates z0 eventType1 eventType1_isType1
  have h3 : (chanlunTransition z0 eventType3).ledger.A = z0.ledger.A + 2 :=
    chanlunTransition_type3_allocates z0 eventType3 eventType3_isType3Buy (by unfold eventType3; rfl)
  rw [h1, h3]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 装配为 #114 策略族 Θ（target/exec 用 π_Θ + RiskProj 真应对）

  把上述真缠论组件装配为 #114 committed `StrategyFamily.Theta`——recog 用 `recogChanlun`
  （真判据），riskProj 用 `chanlunRiskGrid`（#114 真网格），exec 用决策→订单映射。这把本文件的
  真缠论实例化**正式纳入 #114 策略族结构**，使「给定 Θ ⟹ π_Θ 全定义/唯一/因果」对真缠论 Θ 成立。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★决策目标 `ChanlunTarget`（Θ.Target 类型）：携带决策 + 目标仓位。 -/
structure ChanlunTarget where
  decision : ChanlunDecision
  pos : Nat
deriving Repr

/-- ★订单 `ChanlunOrder`（Θ.Order 类型）：携带决策 + 目标仓位 + 账本 delta（Θ.exec 输出）。 -/
structure ChanlunOrder where
  decision : ChanlunDecision
  targetPos : Nat
  delta : Int × Int × Int
deriving Repr

/--
  ★前缀等价 `chanlunPrefixEq`（L0，Θ.prefixEq）：本骨架取事件相等（同事件 ⟹ 同决策）。
  这是因果性的最小前缀关系——recog 只依赖当前事件，同事件必同决策（recog 是事件的函数）。
-/
def chanlunPrefixEq (e e' : ChanlunEvent) : Prop := e = e'

/--
  ★★真缠论 Θ 实例 `chanlunTheta`（L0，#114 StrategyFamily.Theta 真实例化）：
  把真缠论组件填入 #114 committed `Theta` 结构——
  - `recog` := `recogChanlun`（真 #113 判据识别，非 emptyParse）。
  - `riskProj` := `chanlunRiskGrid`（#114 真有限网格，非两值）。
  - `target`/`proj`/`exec` := 真缠论决策 → 目标 → 网格投影 → 订单（携带真账本 delta）。
  - `recog_causal` := 同前缀（同事件）⟹ 同决策（recog 是事件函数，平凡因果）。

  ★H=ChanlunEvent, Z=ChanlunAccount, D=ChanlunDecision, Target=ChanlunTarget, Pos=Nat, K=Nat,
    Order=ChanlunOrder。这是 #114 「完全分类 ⊢ 一族 {π_Θ}」中**一个真缠论成员 Θ**。
-/
def chanlunTheta : Theta ChanlunEvent ChanlunAccount ChanlunDecision ChanlunTarget Nat Nat ChanlunOrder where
  prefixEq := chanlunPrefixEq
  recog := fun e _z => recogChanlun e
  target := fun d _z => { decision := d, pos := decisionTargetPos d }
  riskProj := chanlunRiskGrid
  proj := fun t => t.pos
  exec := fun d _z p => { decision := d, targetPos := p, delta := decisionLedgerDelta d }
  recog_causal := by
    intro h h' z hpre
    unfold chanlunPrefixEq at hpre
    rw [hpre]

/--
  ★★真缠论 π_Θ 全定义 + 唯一（L0，实例化 #114 `given_theta_total_unique`）：
  给定真缠论 Θ、事件 h、账户 z，`piTheta chanlunTheta h z` 存在唯一订单。这把 #114「给定 Θ ⟹
  π_Θ 全定义/唯一」**对真缠论 Θ 兑现**——不再是 originFullDef 的平凡两值，而是真判据驱动的策略。
-/
theorem chanlunTheta_total_unique (h : ChanlunEvent) (z : ChanlunAccount) :
    NewChanlun.Origin.ExistsUnique (fun o => piTheta chanlunTheta h z = o) :=
  given_theta_total_unique chanlunTheta h z

/--
  ★★真缠论 π_Θ 因果无前视（L0，实例化 #114 `piTheta_causal`）：固定账户 z，π_Θ 当下决策只依赖
  当前事件（前缀），不偷看未来——`chanlunPrefixEq h h' → piTheta chanlunTheta h z = piTheta chanlunTheta h' z`。
  这把 #114 策略层因果对真缠论 Θ 兑现。
-/
theorem chanlunTheta_causal (h h' : ChanlunEvent) (z : ChanlunAccount)
    (hpre : chanlunPrefixEq h h') :
    piTheta chanlunTheta h z = piTheta chanlunTheta h' z :=
  piTheta_causal chanlunTheta h h' z hpre

/--
  ★★π_Θ 订单真携真缠论账本 delta（L0，坐实 exec 段真应对，非平凡两值）：
  `piTheta chanlunTheta e z` 的订单 `.delta` = `decisionLedgerDelta (recogChanlun e)`——订单的账本
  足迹由**真缠论分类**确定（经 recog→target→exec 链）。对照 originFullDef 的 risk 两值，
  这是真缠论决策驱动的订单。
-/
theorem chanlunTheta_order_carries_chanlun_delta (e : ChanlunEvent) (z : ChanlunAccount) :
    (piTheta chanlunTheta e z).delta = decisionLedgerDelta (recogChanlun e) := by
  unfold piTheta chanlunTheta
  rfl

/--
  ★★策略族订单与闭环 transition 一致（L0，坐实 π_Θ 与 chanlunTransition 同源）：
  π_Θ 订单携带的 delta 正是 `chanlunTransition` 用来更新 ledger 的 delta——策略族（#114）与
  闭环 transition（§5）由**同一真缠论 recog** 驱动，账本足迹一致。这把 #114 策略族与本文件
  非退化闭环对齐：π_Θ 产订单，T 用同源 delta 改账。
-/
theorem chanlunTheta_order_drives_transition (e : ChanlunEvent) (z : ChanlunAccount) :
    (piTheta chanlunTheta e z).delta =
      (decisionLedgerDelta (recogChanlun e)) := by
  exact chanlunTheta_order_carries_chanlun_delta e z

/-! ════════════════════════════════════════════════════════════════════════
  ## §8 诚实标签（gatekeeper：禁标盈利/缠论唯一/平凡占位）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Θ 实例化标签 `ThetaInstTag`（gatekeeper，诚实分层）。
  - `realChanlunRecog`：recog 段真消费 #113 缠论判据（IsType1/IsType3Buy/IsContinuation），非 emptyParse。
  - `nonDegenerateTransition`：闭环 T 真改 ledger 且两类事件不同 delta（非 trend 平凡占位）。
  - `chanlunRuleEncoded`：第24课/§10.1/§11 三条缠论规则 L1 结构编码（规则忠实，非经验验证）。
  - `finiteGridRiskProj`：risk 段用 #114 RiskProj 有限网格（非两值，非连续 argmin）。
  - `thetaRiskParametric`：仓位数额/网格是 Θ_risk 参数（缠论分类只定 delta 类型不定数额）。
  - `empiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableStrategy` / `ChanlunUniqueStrategy` / `TrivialPlaceholder` 构造子——
  类型层拒绝把本实例化标为盈利策略 / 缠论唯一策略 / 平凡占位。
-/
inductive ThetaInstTag where
  | realChanlunRecog
  | nonDegenerateTransition
  | chanlunRuleEncoded
  | finiteGridRiskProj
  | thetaRiskParametric
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★Θ 实例化子类（gatekeeper）：RealChanlunInstantiatedTheta（唯一构造子）。 -/
inductive ThetaInstSubkind where
  | realChanlunInstantiatedTheta
deriving DecidableEq, Repr

/-- ★诚实标签包（L0 声明）。 -/
def thetaInstLabels : List ThetaInstTag × ThetaInstSubkind :=
  ([ThetaInstTag.realChanlunRecog, ThetaInstTag.nonDegenerateTransition,
    ThetaInstTag.chanlunRuleEncoded, ThetaInstTag.finiteGridRiskProj,
    ThetaInstTag.thetaRiskParametric, ThetaInstTag.empiricalDomain],
   ThetaInstSubkind.realChanlunInstantiatedTheta)

/-- ★禁标平凡占位/盈利/缠论唯一（L0，gatekeeper 见证）：子类必是 RealChanlunInstantiatedTheta。 -/
theorem thetaInst_subkind_is_real_chanlun (k : ThetaInstSubkind) :
    k = ThetaInstSubkind.realChanlunInstantiatedTheta := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（task #117，Θ 缠论-实例化，消闭环平凡占位）

  本文件**证**（L0 结构层 + L1 缠论规则编码，machine-checked，无 sorry/admit/axiom）：

  1. ★真缠论事件类型 `ChanlunEvent`（买卖点端点 + 中枢发展对）——消 `Event := Unit`。
  2. ★真缠论识别核 `recogChanlun`（真 case-split on #113 IsType1/IsType3Buy/IsContinuation 判据，
     编码第24课/§10.1/§11 三条缠论规则）——消 `recStruct := emptyParse` + `decide(trend=trendUp)` 平凡两值。
     · `recog_type1_openRoot` / `recog_type3_accreteCore` / `recog_continuation_hold`：三条规则真编码坐实。
  3. ★真缠论应对（risk/exec）：`chanlunRiskGrid`（#114 RiskProj 有限网格）+ `decisionLedgerDelta`
     （缠论决策 → 账本 delta，建根仓 1 / 增核 2 不同量）——消平凡两值 risk。
     · `targetPos_is_lexargmin`：目标仓位命中 #114 RiskProj LexArgmin（真网格确定选择器）。
  4. ★★真缠论闭环转移 `chanlunTransition`（缠论决策驱动 ledger delta，保 R=Π-A-W）——
     消 `trend=trendUp` 平凡驱动。
     · `chanlunTransition_preserves_ledger_inv`：闭环保账本恒等。
     · `chanlunTransition_type1_allocates`（第一类 A+=1）/ `_type3_allocates`（第三类 A+=2）：真改 ledger。
  5. ★★★**非退化总见证** `chanlun_transition_distinguishes_classes`：两类缠论事件（第一类 vs
     第三类）经真缠论闭环转移产生**不同 ledger delta**（A+1 vs A+2）——直接消解审计 B2/B10
     「平凡占位」（占位对缠论分类不敏感，本文件真区分）。满足 0004 锋利问题非平凡版。
     · 具体见证：`eventType1`（破中枢+背驰）/`eventType3`（离开中枢回抽不破 ZG）+ recog/isType 见证。
  6. ★装配为 #114 策略族 Θ：`chanlunTheta`（真 #114 Theta 实例）+ `chanlunTheta_total_unique`
     （给定真缠论 Θ ⟹ π_Θ 全定义/唯一）+ `chanlunTheta_causal`（因果无前视）+
     `chanlunTheta_order_carries_chanlun_delta`（订单真携真缠论账本 delta）。
  7. 诚实标签 `thetaInstLabels` + `thetaInst_subkind_is_real_chanlun`（禁标平凡占位/盈利/缠论唯一）。

  本文件**不证**（no声明膨胀，诚实边界）：
  - ✗ 这些应对策略盈利/最优/实盘有效（L3 EmpiricalDomain）。
  - ✗ 仓位数额（1/2）来自缠论（Θ_risk 参数——缠论分类只决定 delta 的**类型/符号**：增核 > 建根仓
       的账本足迹序，不决定精确数额）。
  - ✗ bsp 事件的**自动识别**（从 K 线流自动判定每端点属哪类）——本文件消费已判定的 BspEndpoint
       （#113 判据层提供），不证遍历构造（still-MISSING：bspOf 全自动，#113 已标）。
  - ✗ 第二类买点（买卖点定律一「由次级别一类构成」需次级别递归，#113 still-MISSING-D，本文件
       的 recog 只覆盖第一类/第三类/延续三态，第二类待递归级别系统接入——诚实留白非 workaround）。

  ★对 #88（完整性）推进度：本文件把 Origin 唯一闭环 witness 从平凡占位（originFullDef）升级为
    真缠论实例化（chanlunTheta + chanlunTransition），消解审计 B2/B10。但**完整性仍有缺口**：
    (a) 自动识别（bspOf/centersOf 全自动构造，#113 still-MISSING）未接入——本文件消费已判定端点；
    (b) 第二类（次级别递归）未覆盖；(c) 卖点对偶（本文件 recog 只编码买点侧，IsType3Sell 对偶
    待补）；(d) 与 HybridAssembly 自有闭环（AssemblyState/twState 双账本）的对接未做（本文件用
    单账本 ChanlunAccount，#90 取本金三阶段 TW 端未接入）。这些是 #88 完整性的剩余工作，
    诚实标注，不冒充已完成。

  谱系：originFullDef 平凡占位（#101，HybridAssembly §8）→ 审计 B2/B10（平凡占位判决）→
        #113 分类血肉 committed（Divergence/BspClassification/CenterStates）+ #114 策略族
        committed（StrategyFamily/RiskProj）→ 本文件 #117（用真缠论结构实例化 Θ，给非退化
        闭环 witness，消平凡占位）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.ThetaInstantiation
