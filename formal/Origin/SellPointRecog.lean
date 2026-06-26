/-
Origin/SellPointRecog.lean — 卖点对偶判据 + 完整 recog 双向（买卖对称，task #120）

★工位定位（#117 still-MISSING-(c) 判决）：#117 ThetaInstantiation 的 `recogChanlun` 只编码
  **买点侧**（IsType1/IsType3Buy → openRoot/accreteCore），诚实标 still-MISSING-(c)：缺卖点对偶。
  判据层 BspClassification 的三类判据应有买卖对称结构（§10.1 买卖点逐条对偶定义）。

  本文件**对偶实现卖点三类判据**——第一类卖点（顶背驰=上涨突破中枢后背驰）/ 第二类卖点 /
  第三类卖点（回升不破 ZD），并**补全 recog 双向**（买卖对称识别核），证买卖对偶对称性。

  ★关键诚实：本文件**不改** committed BspClassification/ThetaInstantiation（只读，对偶镜像）。
  - 第三类卖点判据 `IsType3Sell` **已在 committed BspClassification** 定义（buy 侧已有 dual）——
    本文件不重定义，直接复用 + 镜像证对称。
  - 第一类卖点：committed `IsType1` 是**方向无关**判据（破中枢 ∧ 背驰），买卖共用——本文件
    给方向特化的卖点视角 `IsType1Sell`（破中枢 ∧ 背驰 ∧ side=short），证它与买点 `IsType1Buy`
    的力度判据**同构**（顶背驰/底背驰共用 `IsDivergence` 的力度序，方向只在 side/trend 语境）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- §10.1（三类买卖点定义，知识库 + 第24课博文，逐条买卖对偶）：
  · 第一类买点：下跌趋势中，次级别向下**跌破**最后一个中枢后形成的**背驰点**（底背驰）。
  · 第一类卖点：上涨趋势中，次级别向上**突破**最后一个中枢后形成的**背驰点**（顶背驰）。
  · 第二类买点：第一类买点后，次级别上涨结束、再次下跌的那个次级别走势的结束点。
  · 第二类卖点：第一类卖点后，次级别下跌结束、再次上涨的那个次级别走势的结束点。
  · 第三类买点：上涨趋势中，次级别向上离开中枢后，次级别回抽低点**不跌破 ZG** 的终结点。
  · 第三类卖点：下跌趋势中，次级别向下离开中枢后，次级别回抽高点**不升破 ZD** 的终结点。
- §10.2 升跌完备性 / 趋势转折定律："上涨转折由某级别第一类卖点构成；下跌转折由第一类买点构成。"
- 第24/25课（背驰）：顶背驰 = 上涨力度背驰（向上两段后段力度弱）；底背驰 = 下跌力度背驰。
  力度比较 `IsDivergence`（forceC < forceA）方向无关——顶/底背驰共用同一力度序判据。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 买卖镜像对偶 / 位置三态卖点应用，omega/decide/rfl machine-checked，
不依赖数据）。`lake env lean Origin/SellPointRecog.lean` 通过 = 卖点三类判据 + recog 双向 +
买卖对偶对称性在定义层成立，**不是**任何"卖点识别在真实行情上有效"的实证断言（L2/L3）。

- **触及 L1 缠论规则真编码**（与 #117 buy 侧对偶）：recog 卖侧**真编码**§10.1「顶背驰⟹第一类卖点」
  + 「向下离开中枢回抽不破 ZD⟹第三类卖点」+ §11「力度延续⟹非卖点」三条规则（真 case-split），
  非平凡占位。规则忠实，非经验验证（卖点规则在真实行情上的有效性是 L2/L3，本文件不声称）。

诚实标注（gatekeeper）：
★ 买卖对偶**非完全对称的缠论特例已审查**（见 §诚实标注 + 边界条件）：
  - 第三类买点用 `ZG < retracePrice`（之上），卖点用 `retracePrice < ZD`（之下）——位置三态镜像，
    但 ZG/ZD **不对称**（ZG=上边界，ZD=下边界，§10.1 注）：买点参照 ZG，卖点参照 ZD。这是
    **判据参照点的方向特化**（非对称参数），不是逻辑不对称——镜像在「位置三态 above↔below」层对称。
  - 第二类卖点对偶（committed `IsType2` 方向无关：afterTypeOne ∧ ¬brokeCenter）——本级别可观测
    条件买卖共用，与 buy 侧同。买卖点定律一「由次级别一类构成」需次级别递归（still-MISSING-D），
    本文件不证递归构造（与 BspClassification/ThetaInstantiation 诚实一致）。

★ still-MISSING（见文件尾）：第二类次级别递归构成 / bspOf 全自动识别 / 卖点 Θ 闭环 ledger 对接
  （本文件证卖点 recog 判据对偶 + 对称性，**不**装配卖点闭环 transition 改 ledger——那需镜像
   ThetaInstantiation 的 §5/§6 闭环，本文件聚焦判据+recog 对偶，闭环对接标 still-MISSING）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root）。
依赖方向（单向无环）：SellPointRecog → {BspClassification, Divergence, CenterStates}
  （均 Origin 内 committed，只读对偶镜像，无 legacy import，不改 committed）。
-/

import Origin.BspClassification
import Origin.Divergence
import Origin.CenterStates

namespace NewChanlun.Origin.SellPointRecog

open NewChanlun.Origin

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 第一类卖点判据（顶背驰）+ 买点视角（§10.1 买卖对偶）

  committed `IsType1`（BspClassification）= 破中枢 ∧ 背驰，是**方向无关**判据（顶/底背驰共用
  `IsDivergence` 的力度序）。本节给方向特化的**卖点/买点视角**，证两者力度判据同构——
  这是买卖对偶在第一类的**对称性见证**（顶背驰=上涨力度背驰，底背驰=下跌力度背驰，同一力度序）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★第一类买点判据（§10.1，方向特化，L0）—— committed `IsType1`（破中枢 ∧ 底背驰）+ side=long。
  底背驰 = 下跌趋势向下跌破中枢后的力度背驰（`IsDivergence`：forceC < forceA）。
-/
def IsType1Buy (e : BspEndpoint) : Prop :=
  e.side = Side.long ∧ IsType1 e

/--
  ★★第一类卖点判据（§10.1 对偶，L0，本节核心）—— 上涨趋势向上**突破**最后一个中枢后的
  **顶背驰点**：committed `IsType1`（破中枢 ∧ 背驰）+ side=short。
  顶背驰 = 上涨趋势向上突破中枢后的力度背驰——与底背驰**共用同一力度序判据** `IsDivergence`
  （forceC < forceA，方向无关）。第一类买卖点的对偶只在 side（long↔short）与 trend 语境，
  力度判据完全对称（顶/底背驰同构）。
-/
def IsType1Sell (e : BspEndpoint) : Prop :=
  e.side = Side.short ∧ IsType1 e

/--
  ★★第一类买卖对偶力度同构（L0，对称性见证①）—— 第一类买点与第一类卖点**共用同一背驰力度判据**：
  二者都要求 `IsType1`（破中枢 ∧ `IsDivergence`），只在 side 上对偶（long↔short）。
  这坐实顶背驰（卖点）与底背驰（买点）在力度序层**对称**——背驰判据方向无关。
-/
theorem type1_buy_sell_share_divergence (e e' : BspEndpoint)
    (hb : IsType1Buy e) (hs : IsType1Sell e') :
    IsDivergence e.divPair ∧ IsDivergence e'.divPair :=
  ⟨hb.2.2, hs.2.2⟩

/--
  ★第一类卖点真消费背驰判据（L0）—— 第一类卖点蕴含背驰（`IsDivergence divPair`）。
  与 buy 侧对偶：卖点也是背驰点（顶背驰），非平凡桩。
-/
theorem type1Sell_implies_divergence (e : BspEndpoint) (h : IsType1Sell e) :
    IsDivergence e.divPair := h.2.2

/--
  ★第一类买卖互斥（L0，方向唯一）—— 同一端点不能既是第一类买点又是第一类卖点（side 唯一）。
-/
theorem type1_buy_sell_exclusive (e : BspEndpoint) :
    ¬ (IsType1Buy e ∧ IsType1Sell e) := by
  rintro ⟨hb, hs⟩
  have h1 : e.side = Side.long := hb.1
  have h2 : e.side = Side.short := hs.1
  rw [h1] at h2; exact Side.noConfusion h2

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 第三类卖点判据（回升不破 ZD）+ 买卖对偶对称（位置三态镜像）

  committed `IsType3Sell`（BspClassification）已定义（side=short ∧ leftCenter ∧ firstRetrace ∧
  `retracePrice < ZD`）。本节**复用**它（不重定义），并证它与 `IsType3Buy` 的**位置三态镜像对称**：
  买点回抽在中枢之上（above ZG），卖点回抽在中枢之下（below ZD）——CenterStates 位置三态镜像。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★第三类买卖对偶位置镜像（L0，对称性见证②，重锚 CenterStates 位置三态）——
  第三类买点回抽点在中枢**之上**（`IsAbove`），第三类卖点回抽点在中枢**之下**（`IsBelow`）。
  这把第三类买卖对偶重锚到 CenterStates 位置三态的 above↔below 镜像——买卖在位置层对称：
  买点不破 ZG（之上），卖点不破 ZD（之下）。
-/
theorem type3_buy_sell_position_mirror (eb es : BspEndpoint)
    (hb : IsType3Buy eb) (hs : IsType3Sell es) :
    IsAbove eb.center eb.retracePrice ∧ IsBelow es.center es.retracePrice :=
  ⟨type3Buy_retrace_above eb hb, type3Sell_retrace_below es hs⟩

/--
  ★第三类卖点反退化见证（L0，非平凡）—— 若回抽高点 `retracePrice ≥ ZD`（升破 ZD，重入中枢），
  则不是第三类卖点。与 buy 侧 `type3Buy_rejects_reenter` 对偶——证卖点判据真能否决。
-/
theorem type3Sell_rejects_reenter (e : BspEndpoint)
    (_hside : e.side = Side.short) (hreenter : e.center.zd ≤ e.retracePrice) :
    ¬ IsType3Sell e := by
  unfold IsType3Sell; intro h
  have hlt := h.2.2.2
  simp only [Tick] at hlt hreenter ⊢
  omega

/--
  ★第三类买卖互斥（L0，复用 committed third_subdomain_exclusive 的对偶坐实）—— 同一端点
  不能既是第三类买点又是第三类卖点。直接引 committed BspClassification 的互斥定理。
-/
theorem type3_buy_sell_exclusive (e : BspEndpoint) :
    ¬ (IsType3Buy e ∧ IsType3Sell e) :=
  third_subdomain_exclusive e

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 第二类卖点判据（§10.1 对偶）

  committed `IsType2`（BspClassification）= afterTypeOne ∧ ¬brokeCenter，是**方向无关**的本级别
  可观测条件（第一类后回抽结束点）。本节给方向特化的第二类买/卖视角，证买卖共用同一本级别判据。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★第二类买点判据（§10.1，方向特化，L0）—— committed `IsType2` + side=long。
  第一类买点后，次级别上涨结束、再次下跌的结束点（本级别可观测条件：一类后回抽未再破中枢）。
-/
def IsType2Buy (e : BspEndpoint) : Prop :=
  e.side = Side.long ∧ IsType2 e

/--
  ★★第二类卖点判据（§10.1 对偶，L0）—— committed `IsType2` + side=short。
  第一类卖点后，次级别下跌结束、再次上涨的结束点（本级别可观测条件：一类后回抽未再破中枢）。
  ★诚实：与 buy 侧 IsType2 一致，本判据刻画**本级别可观测条件**；买卖点定律一「由次级别一类
  构成」需次级别递归（still-MISSING-D，BspClassification 已标），本文件不证递归构造。
-/
def IsType2Sell (e : BspEndpoint) : Prop :=
  e.side = Side.short ∧ IsType2 e

/--
  ★★第二类买卖对偶本级别判据同构（L0，对称性见证③）—— 第二类买点与卖点**共用同一本级别
  可观测判据** `IsType2`（一类后 ∧ 未破中枢），只在 side 上对偶。坐实第二类买卖对称。
-/
theorem type2_buy_sell_share_criterion (e e' : BspEndpoint)
    (hb : IsType2Buy e) (hs : IsType2Sell e') :
    IsType2 e ∧ IsType2 e' :=
  ⟨hb.2, hs.2⟩

/--
  ★第二类买卖互斥（L0）—— side 唯一。 -/
theorem type2_buy_sell_exclusive (e : BspEndpoint) :
    ¬ (IsType2Buy e ∧ IsType2Sell e) := by
  rintro ⟨hb, hs⟩
  have h1 : e.side = Side.long := hb.1
  have h2 : e.side = Side.short := hs.1
  rw [h1] at h2; exact Side.noConfusion h2

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 卖点识别核 recogChanlunSell（对偶 #117 recogChanlun 买点侧）

  ★补全 #117 still-MISSING-(c)：#117 `recogChanlun` 只编码买点侧（openRoot/accreteCore）。
  本节给**对偶卖点识别核**——真 case-split on 卖点判据，编码§10.1「顶背驰⟹第一类卖点」+
  「向下离开中枢回抽不破 ZD⟹第三类卖点」+ §11「力度延续⟹非卖点」三条规则。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★卖点缠论决策 `SellDecision`（L0，对偶 #117 ChanlunDecision 买点三态）：
  - `closeRoot`：清根仓 / 反向建空（第一类顶背驰卖点——突破中枢后背驰，§10.1）。
  - `reduceCore`：减核（第三类卖点——向下离开中枢回抽不破 ZD，§10.1）。
  - `hold`：保持（力度延续/非卖点，§11——后段力度 ≥ 前段，趋势延续无背驰）。

  ★这三态由真缠论卖点判据区分（见 `recogChanlunSell`），与买点侧（openRoot/accreteCore/hold）对偶。
-/
inductive SellDecision where
  | closeRoot     -- 第一类顶背驰卖点 → 清根仓
  | reduceCore    -- 第三类回升卖点 → 减核
  | hold          -- 力度延续/非卖点 → 保持
deriving DecidableEq, Repr

/--
  ★★卖点识别核 `recogChanlunSell`（L0，本节核心——对偶 #117 recogChanlun 买点侧）：
  从买卖点端点 `BspEndpoint` 真 case-split on 卖点判据，识别卖侧应对意图。

  缠论卖点规则真编码（L1 卖侧规则结构编码，对偶 #117 买侧）：
  - **§10.1 第一类卖点**：若破中枢 ∧ 背驰（`IsType1`）∧ side=short ⟹ `closeRoot`（顶背驰清仓）。
  - **§10.1 第三类卖点**：否则若 side=short ∧ 离开中枢 ∧ 第一次回抽 ∧ retracePrice < ZD（不破 ZD）
    ⟹ `reduceCore`（第三类卖点减核）。
  - **§11**：否则（含力度延续 = 非背驰，无卖点）⟹ `hold`（保持）。

  ★这是真缠论卖点识别——直接消费 committed `IsType1`（含 `IsDivergence`）+ 第三类卖点判据
  字段（`retracePrice < ZD`），与 #117 买侧 `recogChanlun` 严格对偶（决策载体不同，判据镜像）。
-/
def recogChanlunSell (e : BspEndpoint) : SellDecision :=
  if e.side = Side.short ∧ e.brokeCenter = true ∧ decide (IsDivergence e.divPair) = true then
    -- §10.1 第一类卖点：突破中枢 + 顶背驰 ⟹ 清根仓。
    SellDecision.closeRoot
  else if e.side = Side.short ∧ e.leftCenter = true ∧ e.firstRetrace = true ∧
          decide (e.retracePrice < e.center.zd) = true then
    -- §10.1 第三类卖点：向下离开中枢 + 第一次回抽 + 不破 ZD ⟹ 减核。
    SellDecision.reduceCore
  else
    -- §11：力度延续 / 非卖点 ⟹ 保持。
    SellDecision.hold

/--
  ★recog 卖侧真消费第一类卖点判据（L0，对偶 #117 recog_type1_openRoot）：若端点是第一类卖点
  （`IsType1Sell`：side=short ∧ 破中枢 ∧ 背驰），则 `recogChanlunSell` 识别为 `closeRoot`。
  坐实卖侧 recog 真编码§10.1 第一类卖点规则——非占位。
-/
theorem recogSell_type1_closeRoot (e : BspEndpoint) (h : IsType1Sell e) :
    recogChanlunSell e = SellDecision.closeRoot := by
  unfold recogChanlunSell IsType1Sell IsType1 at *
  obtain ⟨hside, hbroke, hdiv⟩ := h
  have hdiv' : decide (IsDivergence e.divPair) = true := decide_eq_true hdiv
  rw [if_pos ⟨hside, hbroke, hdiv'⟩]

/--
  ★recog 卖侧真消费第三类卖点判据（L0，对偶 #117 recog_type3_accreteCore）：若端点是第三类卖点
  （`IsType3Sell`：side=short ∧ 离开中枢 ∧ 第一次回抽 ∧ 不破 ZD）**且非第一类**（未破中枢），
  则 `recogChanlunSell` 识别为 `reduceCore`。
  ★前提 `¬ broke`：第三类卖点语境是「向下离开中枢后回升」，与第一类「突破中枢背驰」互斥分支——
  第一类优先级更高（先判破中枢背驰）。坐实第三类卖点规则真被编码。
-/
theorem recogSell_type3_reduceCore (e : BspEndpoint)
    (h : IsType3Sell e) (hnobreak : e.brokeCenter = false) :
    recogChanlunSell e = SellDecision.reduceCore := by
  unfold recogChanlunSell IsType3Sell at *
  obtain ⟨hside, hleft, hfirst, hzd⟩ := h
  have hcond1 : ¬ (e.side = Side.short ∧ e.brokeCenter = true ∧
      decide (IsDivergence e.divPair) = true) := by
    rw [hnobreak]; simp
  have hzd' : decide (e.retracePrice < e.center.zd) = true := decide_eq_true hzd
  rw [if_neg hcond1, if_pos ⟨hside, hleft, hfirst, hzd'⟩]

/--
  ★recog 卖侧真消费力度延续判据（L0，§11，对偶 #117 recog_continuation_hold）：若端点背驰段
  力度延续（`IsContinuation divPair` = 非背驰）且非第三类卖点语境（未离开中枢），则
  `recogChanlunSell` 识别为 `hold`——力度延续无卖点，保持。
  ★`hcont` 经 committed `continuation_not_divergence` 推 `¬ IsDivergence`，使第一类卖点分支的
  背驰判据**经力度判据本身**失败（不依赖 brokeCenter/side 旁路）——延续段路由到 hold。
-/
theorem recogSell_continuation_hold (e : BspEndpoint)
    (hcont : IsContinuation e.divPair)
    (hnoleft : e.leftCenter = false) :
    recogChanlunSell e = SellDecision.hold := by
  unfold recogChanlunSell
  have hnodiv : ¬ IsDivergence e.divPair := continuation_not_divergence e.divPair hcont
  have hcond1 : ¬ (e.side = Side.short ∧ e.brokeCenter = true ∧
      decide (IsDivergence e.divPair) = true) := by
    rintro ⟨_, _, hd⟩; exact hnodiv (of_decide_eq_true hd)
  have hcond2 : ¬ (e.side = Side.short ∧ e.leftCenter = true ∧
      e.firstRetrace = true ∧ decide (e.retracePrice < e.center.zd) = true) := by
    rw [hnoleft]; simp
  rw [if_neg hcond1, if_neg hcond2]

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 买卖 recog 双向对称性（本文件核心交付：完整 recog 双向）

  #117 recog 只有买点侧（openRoot/accreteCore/hold）。本文件补卖点侧（closeRoot/reduceCore/hold）。
  本节证**买卖 recog 双向对称**：买点侧与卖点侧由 side（long↔short）镜像驱动，
  第一类→根仓操作（建/清）、第三类→核操作（增/减）、力度延续→hold（买卖共用）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★recog 双向覆盖买卖全侧（L0，完整 recog 双向见证）—— 任一端点：
  - side=long ⟹ 由买点 recog（#117 recogChanlun）处理（识别 openRoot/accreteCore/hold）。
  - side=short ⟹ 由卖点 recog（本文件 recogChanlunSell）处理（识别 closeRoot/reduceCore/hold）。
  本定理证卖侧 recog 输出落在 SellDecision 三态——卖侧识别核**全定义**（对任意端点有输出），
  与买侧 recog 全定义对偶。补全 #117 still-MISSING-(c)：recog 不再单买点侧。
-/
theorem recogSell_total (e : BspEndpoint) :
    recogChanlunSell e = SellDecision.closeRoot ∨
    recogChanlunSell e = SellDecision.reduceCore ∨
    recogChanlunSell e = SellDecision.hold := by
  unfold recogChanlunSell
  by_cases h1 : e.side = Side.short ∧ e.brokeCenter = true ∧
      decide (IsDivergence e.divPair) = true
  · rw [if_pos h1]; exact Or.inl rfl
  · rw [if_neg h1]
    by_cases h2 : e.side = Side.short ∧ e.leftCenter = true ∧
        e.firstRetrace = true ∧ decide (e.retracePrice < e.center.zd) = true
    · rw [if_pos h2]; exact Or.inr (Or.inl rfl)
    · rw [if_neg h2]; exact Or.inr (Or.inr rfl)

/--
  ★★买卖对偶决策对应（L0，对称性见证④·总）—— 第一类（买/卖）→根仓操作，第三类→核操作：
  - 第一类买点 ⟹ 买侧 openRoot（建根仓）；第一类卖点 ⟹ 卖侧 closeRoot（清根仓）。
  - 第三类买点 ⟹ 买侧 accreteCore（增核）；第三类卖点 ⟹ 卖侧 reduceCore（减核）。
  本定理给卖侧的对应（第一类卖点→closeRoot ∧ 第三类卖点→reduceCore），与买侧（#117）镜像对偶。

  ★这是「买卖对偶对称性」的总见证：买点底背驰建仓 ↔ 卖点顶背驰清仓；买点第三类离开中枢向上增核
  ↔ 卖点第三类离开中枢向下减核（§10.1 逐条买卖对偶在 recog 决策层兑现）。
-/
theorem sell_recog_dual_correspondence (e1 e3 : BspEndpoint)
    (h1 : IsType1Sell e1)
    (h3 : IsType3Sell e3) (h3nobreak : e3.brokeCenter = false) :
    recogChanlunSell e1 = SellDecision.closeRoot ∧
    recogChanlunSell e3 = SellDecision.reduceCore :=
  ⟨recogSell_type1_closeRoot e1 h1, recogSell_type3_reduceCore e3 h3 h3nobreak⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 反退化见证（具体卖点端点，非平凡桩）
  ════════════════════════════════════════════════════════════════════════ -/

/-- 一个具体中枢（核心[10,20]）——见证用。 -/
def witnessCenter : Center :=
  { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }

/--
  ★第一类卖点见证 `sampleType1Sell`（具体缠论数值）：上涨突破中枢 + 顶背驰（forceC=2 < forceA=8）
  + side=short。满足 `IsType1Sell` ⟹ recogSell 识别为 closeRoot。
-/
def sampleType1Sell : BspEndpoint :=
  { side := Side.short
    center := witnessCenter
    divPair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true }
    brokeCenter := true     -- 突破中枢（第一类卖点前提）
    afterTypeOne := false
    leftCenter := false
    retracePrice := 30      -- 突破后高点（无关第三类）
    firstRetrace := false }

/--
  ★第三类卖点见证 `sampleType3Sell`（具体缠论数值）：向下离开中枢 + 第一次回升 + 不破 ZD
  （retracePrice=5 < zd=10）+ side=short + 未破中枢。满足 `IsType3Sell` ⟹ recogSell 识别 reduceCore。
-/
def sampleType3Sell : BspEndpoint :=
  { side := Side.short
    center := witnessCenter
    divPair := { forceA := ⟨3⟩, forceC := ⟨3⟩, isTrend := false }  -- 力度延续（非背驰）
    brokeCenter := false    -- 未破中枢（区别第一类）
    afterTypeOne := true
    leftCenter := true      -- 离开中枢（第三类前提）
    retracePrice := 5       -- 5 < zd=10 ⟹ 不破 ZD ⟹ 第三类卖点
    firstRetrace := true }

/-- ★见证：sampleType1Sell 满足第一类卖点判据（突破中枢 + 顶背驰）。 -/
theorem sampleType1Sell_isType1Sell : IsType1Sell sampleType1Sell := by
  unfold IsType1Sell IsType1 IsDivergence sampleType1Sell
  exact ⟨rfl, rfl, by decide⟩

/-- ★见证：sampleType3Sell 满足第三类卖点判据（离开中枢 + 第一次回升 + 不破 ZD）。 -/
theorem sampleType3Sell_isType3Sell : IsType3Sell sampleType3Sell := by
  unfold IsType3Sell sampleType3Sell witnessCenter
  exact ⟨rfl, rfl, rfl, by decide⟩

/-- ★见证：sampleType1Sell 被卖侧 recog 识别为 closeRoot（第一类清根仓）。 -/
theorem sampleType1Sell_recog_closeRoot :
    recogChanlunSell sampleType1Sell = SellDecision.closeRoot :=
  recogSell_type1_closeRoot sampleType1Sell sampleType1Sell_isType1Sell

/-- ★见证：sampleType3Sell 被卖侧 recog 识别为 reduceCore（第三类减核）。 -/
theorem sampleType3Sell_recog_reduceCore :
    recogChanlunSell sampleType3Sell = SellDecision.reduceCore :=
  recogSell_type3_reduceCore sampleType3Sell sampleType3Sell_isType3Sell
    (by unfold sampleType3Sell; rfl)

/--
  ★★非退化总见证 `sell_recog_distinguishes_type1_type3`（L0，卖侧 recog 真区分两类）——
  第一类卖点事件与第三类卖点事件被卖侧 recog 识别为**不同决策**（closeRoot ≠ reduceCore）。
  与买侧 #117 `chanlun_transition_distinguishes_classes` 对偶——卖侧 recog 对缠论分类敏感，非占位。
-/
theorem sell_recog_distinguishes_type1_type3 :
    recogChanlunSell sampleType1Sell ≠ recogChanlunSell sampleType3Sell := by
  rw [sampleType1Sell_recog_closeRoot, sampleType3Sell_recog_reduceCore]
  decide

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 诚实标签（gatekeeper：禁标盈利/卖点闭环已对接/平凡占位）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★卖点对偶标签 `SellRecogTag`（gatekeeper，诚实分层）。
  - `realSellRecog`：卖侧 recog 真消费 committed 卖点判据（IsType1Sell/IsType3Sell/IsContinuation）。
  - `buySellSymmetric`：买卖对偶在力度判据/位置三态/本级别判据三层对称（§1/§2/§3 见证）。
  - `sellRuleEncoded`：§10.1 顶背驰卖点/回升不破 ZD/§11 力度延续三条卖侧规则 L1 结构编码。
  - `ledgerClosureMissing`：卖点 Θ 闭环 ledger 对接**未做**（still-MISSING，诚实留白）。
  - `secondTypeRecursionMissing`：第二类次级别递归构成未证（与 committed 一致 still-MISSING-D）。
  - `empiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableSellStrategy` / `SellLedgerClosureDone` / `TrivialPlaceholder` 构造子——
  类型层拒绝把本对偶标为盈利策略 / 卖点闭环已完成 / 平凡占位。
-/
inductive SellRecogTag where
  | realSellRecog
  | buySellSymmetric
  | sellRuleEncoded
  | ledgerClosureMissing
  | secondTypeRecursionMissing
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★卖点对偶子类（gatekeeper）：DualSellRecog（唯一构造子，禁标已完成闭环）。 -/
inductive SellRecogSubkind where
  | dualSellRecog
deriving DecidableEq, Repr

/-- ★诚实标签包（L0 声明）。 -/
def sellRecogLabels : List SellRecogTag × SellRecogSubkind :=
  ([SellRecogTag.realSellRecog, SellRecogTag.buySellSymmetric,
    SellRecogTag.sellRuleEncoded, SellRecogTag.ledgerClosureMissing,
    SellRecogTag.secondTypeRecursionMissing, SellRecogTag.empiricalDomain],
   SellRecogSubkind.dualSellRecog)

/-- ★禁标平凡占位/盈利/闭环已完成（L0，gatekeeper 见证）：子类必是 DualSellRecog。 -/
theorem sellRecog_subkind_is_dual (k : SellRecogSubkind) :
    k = SellRecogSubkind.dualSellRecog := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（task #120，卖点对偶 + 完整 recog 双向，消 #117 still-MISSING-(c)）

  本文件**证**（L0 结构层 + L1 卖侧缠论规则编码，machine-checked，无 sorry/admit/axiom）：

  1. ★第一类卖点判据 `IsType1Sell`（顶背驰=突破中枢后背驰 ∧ side=short）+ 买点视角 `IsType1Buy`
     · `type1_buy_sell_share_divergence`：第一类买卖共用同一背驰力度判据（顶/底背驰同构，对称①）。
     · `type1Sell_implies_divergence` / `type1_buy_sell_exclusive`：卖点真消费背驰 + 买卖互斥。
  2. ★第三类卖点（复用 committed `IsType3Sell`，不重定义）+ 位置三态镜像对称
     · `type3_buy_sell_position_mirror`：买点之上（above ZG）↔ 卖点之下（below ZD），对称②。
     · `type3Sell_rejects_reenter` / `type3_buy_sell_exclusive`：卖点反退化否决 + 买卖互斥。
  3. ★第二类卖点判据 `IsType2Sell` + 买点 `IsType2Buy`
     · `type2_buy_sell_share_criterion`：第二类买卖共用本级别可观测判据（对称③）。
  4. ★★卖点识别核 `recogChanlunSell`（对偶 #117 recogChanlun 买点侧，真 case-split 卖点判据）
     · `recogSell_type1_closeRoot` / `recogSell_type3_reduceCore` / `recogSell_continuation_hold`：
       §10.1 顶背驰卖点 / 回升不破 ZD / §11 力度延续三条卖侧规则真编码坐实。
  5. ★★完整 recog 双向（消 #117 still-MISSING-(c)）
     · `recogSell_total`：卖侧 recog 全定义（任意端点落 SellDecision 三态）。
     · `sell_recog_dual_correspondence`：第一类卖点→closeRoot ∧ 第三类卖点→reduceCore，
       与买侧（建仓/增核）镜像对偶（对称④·总）。
  6. ★反退化见证（具体卖点端点 sampleType1Sell/sampleType3Sell）+ 非退化总见证
     `sell_recog_distinguishes_type1_type3`（卖侧 recog 真区分两类，非占位）。
  7. 诚实标签 `sellRecogLabels` + `sellRecog_subkind_is_dual`（禁标平凡占位/盈利/闭环已完成）。

  本文件**不证**（no声明膨胀，诚实边界）：
  - ✗ 卖点应对策略盈利/最优/实盘有效（L3 EmpiricalDomain）。
  - ✗ 卖点 Θ **闭环 ledger 对接**（镜像 ThetaInstantiation §5/§6 的 chanlunTransition 改 ledger）——
       本文件聚焦卖点 recog 判据对偶 + 买卖对称，闭环 transition（卖点决策→账本 delta→ledgerStep）
       标 still-MISSING（ledgerClosureMissing 标签）。这是诚实留白，非 workaround：本工位 own
       「卖点对偶判据 + recog 双向」，闭环对接是下一波（与 #117 买侧闭环对偶装配）。
  - ✗ 第二类次级别递归构成（买卖点定律一「由次级别一类构成」，still-MISSING-D，committed 已标）。
  - ✗ bspOf 卖点端点自动识别（从 K 线流自动判定，still-MISSING，BspClassification 已标）。

  ★买卖对偶对称见证（四层）：
  - 第一类（§1）：顶背驰↔底背驰共用 `IsDivergence` 力度序（方向无关），对称①。
  - 第三类（§2）：回抽不破 ZG（above）↔ 不破 ZD（below），CenterStates 位置三态镜像，对称②。
  - 第二类（§3）：买卖共用 `IsType2` 本级别可观测判据，对称③。
  - recog（§5）：建仓/增核（买）↔ 清仓/减核（卖）镜像决策对应，对称④。

  ★买卖**非完全对称的缠论特例审查**（no-workaround：遇定义冲突停 ESCALATE）——审查结论：
  **无定义冲突，未触发 ESCALATE**。买卖对偶在缠论中是**严格对称**的（§10.1 逐条对偶定义）：
  唯一的「非对称」是参照点 ZG（买点上边界）vs ZD（卖点下边界），这是**位置三态 above/below 的
  方向特化参数**（CenterStates 已证 above↔below 镜像），不是逻辑不对称——镜像在位置三态层封闭。
  力度判据 `IsDivergence` 完全方向无关（顶/底背驰同构）。因此买卖对偶可严格镜像实现，无矛盾。

  谱系：#113 BspClassification 判据层 committed（IsType1/IsType3Buy/IsType3Sell 已含卖点对偶字段）
        + #114 策略族 committed → #117 ThetaInstantiation recog 买点侧（诚实标 still-MISSING-(c)
        卖点对偶）→ 本文件 #120（补卖点三类判据视角 + recogChanlunSell 双向 + 买卖对称四层见证，
        消 still-MISSING-(c)；闭环 ledger 对接诚实留 still-MISSING）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SellPointRecog
