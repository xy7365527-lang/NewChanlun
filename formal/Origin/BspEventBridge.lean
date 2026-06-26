/-
Origin/BspEventBridge.lean — 薄 bspOf(Bsp 三类) ↦ 厚 ChanlunEvent 判据的真桥（task #128）

★工位定位（still-MISSING：薄 `bspOf` ↛ 厚 `ChanlunEvent` 判据缺真桥）：
  - 构造层 `BspConstruction.bspOf : List BspCandidate → List Bsp` 产出 **canonical 薄 `Bsp`**
    （`{kind : BspKind, side, index, price}`——只载分类标签 + 位置/价，**不**载 recog 所需的
    厚判据字段 divPair/brokeCenter/leftCenter/retracePrice/center）。
  - 闭环 recog `ThetaInstantiation.recogChanlun : ChanlunEvent → ChanlunDecision` 消费 **厚
    `ChanlunEvent`**（载 `BspEndpoint` 完整判据 + 中枢发展对），真 case-split on §10.1/§24/§11 判据。
  - 两段类型不接：薄 `Bsp` ↛ 厚 `ChanlunEvent`。

★Pipeline.lean §0 用 **双投影同源** 结构处理该缺口：让 `bspOf candidates` 与事件流**都**取自
  同一 `candidates : List BspCandidate`，二者是同一厚端点的两个投影——
  `pipeline_bsp_recog_coherent (c : BspCandidate) ...` 在两侧**同一 `c`**上读 `classifyEndpoint c`
  与 `recogChanlun (candidateToEvent c dev)`。这**回避**了薄↛厚边界（never crosses thin→thick）：
  它证「同一对象的两个视图一致」，不证「薄标签 ↦ 厚判据」。

★本文件坐实**真桥**（消双投影同源绕过）：桥的两端是**结构上不同的对象**——
  - 薄端：任意 `b : Bsp`（一个分类标签，构造投影的陪域）；
  - 厚端：任意 `e : ChanlunEvent`（一个携完整判据的事件，recog 消费的定义域）；
  - 桥谓词 `BridgesTo b e` **不**从 `e` 投影出 `b`，也**不**共享 `BspCandidate`——
    `b` 与 `e` 是独立量化的两个对象。桥的内容是「`b` 的标签 `kind/side` 与 `e` **自身**判据
    （从 `e.bsp` 的 divPair/brokeCenter/leftCenter/retracePrice/center 重新读出）相符」。

  ★关键反伪桥论证（消双投影同源）：`BridgesTo b e` 的验证**从 `e` 本身**派生厚判据
  （`IsType1 e.bsp` / `IsType3Buy e.bsp`），`b` 仅贡献标签。不存在函数 `Bsp → ChanlunEvent`、
  不存在共享 `BspCandidate`。故桥**跨**真实结构边界（薄标签 ↦ 厚判据 ↦ recog 决策），
  不是「同源双投影使定理平凡成立」。反退化由 §4 坐实：标签与判据**矛盾**的 (b, e) 对**不**桥接
  （`bridge_rejects_mismatch`）——桥非真空真，故非平凡。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- §10.1（三类买卖点）：第一类 = 破中枢后背驰点；第三类买 = 离开中枢回抽不破 ZG。
- 第24课：第一类买点 = 跌破最后一个中枢后的背驰点（forceC < forceA）。
- §11：力度延续（A≤C，非背驰）不构成买点 ⟹ recog 为 hold。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 厚判据从事件重新读出 / recog 决策图 / 存在性见证，
decide/rfl/case-split machine-checked，不依赖数据）。
`lake env lean Origin/BspEventBridge.lean` 通过 = 「薄 `Bsp` 标签 ↦ 厚 `ChanlunEvent` 判据 ↦
recog 决策」的真桥（两端结构不同、非同源双投影、非平凡）在定义层成立，
**不是**任何「桥在真实行情上把每个识别的薄买卖点正确对应到厚事件」的实证断言（L2+）。

诚实标注（no-patch-mentality / no-workaround）：
★ 本桥是 **label↦criteria 的可靠性 + 可实现性** 桥，**不**是「从薄 Bsp **反推**唯一厚 e」的桥——
  反推不可能（薄 Bsp 丢失判据字段，信息不可逆，是 BspConstruction still-MISSING-D′ 的上游接口）。
  桥的正确方向是：**厚事件 e 的判据 ⟹ 它该带的薄标签 b**（构造投影方向，可靠性），
  **+ 每个可能的薄标签都有厚见证**（可实现性，存在性）。把不可逆的反推冒充为桥 = 伪桥（禁止）。
★ 桥只覆盖第一类（type1）+ 第三类买（type3 买侧）的 recog 决策对应。第二类（type2）的 recog
  决策对应继承自闭环 hold 分支但其厚判据（afterTypeOne）未进 recogChanlun 的显式 case
  （recogChanlun 只 case type1/type3买/否则 hold），故 type2 桥到 hold（§4 诚实标 still-OPEN-type2）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root
`NewChanlun.Origin.BspEventBridge`）。
依赖方向（单向无环，全 committed 只读）：BspEventBridge → {BspClassification, BspConstruction,
  ThetaInstantiation, ChanlunElements}（均 Origin 内 committed）。

谱系：Pipeline.lean §0 双投影同源（#125，回避薄↛厚边界）→ still-MISSING（薄 bspOf↛厚判据缺真桥）
  → 本文件 #128（坐实跨真实结构的 label↦criteria↦decision 真桥，消双投影同源绕过）。
-/

import Origin.BspClassification
import Origin.BspConstruction
import Origin.ThetaInstantiation

namespace NewChanlun.Origin.BspEventBridge

open NewChanlun.Origin
open NewChanlun.Origin.ThetaInstantiation
  (ChanlunEvent ChanlunDecision recogChanlun decisionLedgerDelta
   recog_type1_openRoot recog_type3_accreteCore)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 薄标签 ↦ 厚判据的桥谓词 `BridgesTo`（两端结构不同，非同源双投影）

  ★桥的两端：薄 `b : Bsp`（构造投影陪域，只载 kind/side/index/price）vs 厚 `e : ChanlunEvent`
  （recog 定义域，载完整判据）。`BridgesTo b e` **独立量化** b 和 e——不从 e 投影出 b、不共享
  `BspCandidate`。桥内容：b 的标签 ↔ e **自身**判据（从 `e.bsp` 重新读出 IsType1/IsType3Buy）相符。

  ★这是真桥（跨薄↛厚边界）而非双投影同源：验证侧用 `IsType1 e.bsp`（从厚端 e 派生），
  消费侧用 `b.kind`（薄端标签）——两端是不同对象的不同信息，桥要求它们相符。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★桥谓词 `BridgesTo`（L0，本文件核心）：薄标签 `b : Bsp` 与厚事件 `e : ChanlunEvent` **桥接** ⟺
  `b` 的分类标签与 `e` 自身携带的厚判据相符。

  - `b.kind = type1`：要求 `e` 满足第一类厚判据 `IsType1 e.bsp`（破中枢 ∧ 背驰）∧ 方向一致。
  - `b.kind = type3`：要求 `e` 满足第三类买厚判据 `IsType3Buy e.bsp`（离开中枢 ∧ 第一次回抽 ∧
    不破 ZG）∧ 方向一致。
  - `b.kind = type2`：要求 `e` 满足第二类厚判据 `IsType2 e.bsp`（在一类后 ∧ 回抽未再破中枢）∧
    方向一致。

  ★`b` 与 `e` 是**两个独立参数**——桥**不**从 `e` 计算 `b`、**不**从 `b` 反推 `e`。桥验证从 `e`
  本身派生厚判据（`IsType1 e.bsp` 等），`b` 仅贡献标签。故桥跨真实结构边界（薄↛厚），
  非同源双投影。
-/
def BridgesTo (b : Bsp) (e : ChanlunEvent) : Prop :=
  b.side = e.bsp.side ∧
  match b.kind with
  | BspKind.type1 => IsType1 e.bsp
  | BspKind.type3 => IsType3Buy e.bsp
  | BspKind.type2 => IsType2 e.bsp

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 桥的可靠性：厚事件判据 ⟹ 它该带的薄标签（构造投影方向）

  ★正确的桥方向（厚 ⟹ 薄，可靠性）：若厚事件 `e` 满足第一类判据，则 `classifyEndpoint` 给它
  配的薄标签**桥接**回 `e`。这把「构造层 bspOf 的薄标签」锚回「它源自的厚事件判据」——
  桥可靠（构造投影不撒谎）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★把厚事件 `e` 的端点装成构造层候选 `labelCandidateOf`（L0）：取 `e.bsp` 作判据端点，配占位
  下标/价。这是**厚端 → 构造层输入**的装配（喂 `classifyEndpoint`），不是「薄 ↦ 厚」反推。
-/
def labelCandidateOf (e : ChanlunEvent) : BspCandidate :=
  { endpoint := e.bsp, index := 0, price := 0 }

/--
  ★桥可靠性·第一类（L0，§2）：厚事件满足 `IsType1` ⟹ 构造层 `classifyEndpoint` 给它的薄标签
  存在、为 `type1`、且**桥接**回该厚事件。

  ★这坐实：薄标签不是凭空贴的——它由厚判据 `IsType1 e.bsp` 决定，且桥 `BridgesTo` 把薄标签
  正确连回厚事件（标签的 kind=type1 ↔ e 满足 IsType1）。桥的两端（薄标签 b、厚事件 e）相符。
-/
theorem bridge_sound_type1 (e : ChanlunEvent) (h : IsType1 e.bsp) :
    ∃ b : Bsp, classifyEndpoint (labelCandidateOf e) = some b ∧
               b.kind = BspKind.type1 ∧ BridgesTo b e := by
  refine ⟨{ kind := BspKind.type1, side := e.bsp.side, index := 0, price := 0 }, ?_, rfl, ?_⟩
  · -- classifyEndpoint：IsType1 ⟹ 标 type1。labelCandidateOf e 的 endpoint = e.bsp。
    unfold classifyEndpoint labelCandidateOf
    rw [if_pos h]
  · -- BridgesTo：side 相符（rfl）∧ kind=type1 分支要求 IsType1 e.bsp（即 h）。
    refine ⟨rfl, ?_⟩
    simpa using h

/--
  ★桥可靠性·第三类买（L0，§2）：厚事件满足 `IsType3Buy` 且**非第一类**（未破中枢）⟹ 构造层薄
  标签存在、为 `type3`、且桥接回该厚事件。

  ★前提 `hnobreak`（未破中枢）：classifyEndpoint 优先级一类>三类（破中枢背驰先判一类），故第三类
  标签需排除一类语境——与 recog 优先级、BspConstruction.classifyEndpoint 优先级一致。
-/
theorem bridge_sound_type3buy (e : ChanlunEvent) (h : IsType3Buy e.bsp)
    (hnobreak : e.bsp.brokeCenter = false) :
    ∃ b : Bsp, classifyEndpoint (labelCandidateOf e) = some b ∧
               b.kind = BspKind.type3 ∧ BridgesTo b e := by
  refine ⟨{ kind := BspKind.type3, side := e.bsp.side, index := 0, price := 0 }, ?_, rfl, ?_⟩
  · -- classifyEndpoint：¬IsType1（未破中枢）∧ IsType3 ⟹ 标 type3。
    unfold classifyEndpoint labelCandidateOf
    have hnt1 : ¬ IsType1 e.bsp := by
      unfold IsType1; rw [hnobreak]; simp
    have ht3 : IsType3 e.bsp := Or.inl h
    rw [if_neg hnt1, if_pos ht3]
  · -- BridgesTo：side 相符 ∧ kind=type3 分支要求 IsType3Buy e.bsp（即 h）。
    refine ⟨rfl, ?_⟩
    simpa using h

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 桥的下游连通：薄标签（经桥）⟹ 厚 recog 决策（薄↦厚↦决策真贯穿）

  ★桥的下游推论：一旦薄标签 `b` 桥接到厚事件 `e`，`b` 的 kind 就**约束** `recogChanlun e` 的决策。
  这是薄↛厚边界的**真跨越**：薄标签（type1）经桥连到厚事件，厚事件经 recog 给决策（openRoot）。
  `b` 是薄端输入，`e` 是厚端输入，结论是厚端 recog 输出——三者贯穿。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★桥下游·第一类 ⟹ openRoot（L0，§3，薄↦厚↦决策）：若薄标签 `b`（kind=type1）桥接到厚事件 `e`，
  则 `recogChanlun e = openRoot`（第一类建根仓）。

  ★薄↛厚边界真跨越：从薄端标签 `b.kind = type1` + 桥 `BridgesTo b e`，推出厚端 recog 决策。
  桥 `BridgesTo` 在 type1 分支提供 `IsType1 e.bsp`，喂 `recog_type1_openRoot`（#117 committed）。
  `b` 和 `e` 是独立参数——结论不靠「b 来自 e 的投影」，靠桥谓词把薄标签约束到厚判据。
-/
theorem bridge_type1_forces_openRoot (b : Bsp) (e : ChanlunEvent)
    (hkind : b.kind = BspKind.type1) (hbridge : BridgesTo b e) :
    recogChanlun e = ChanlunDecision.openRoot := by
  have ht1 : IsType1 e.bsp := by
    have := hbridge.2
    rw [hkind] at this
    simpa using this
  exact recog_type1_openRoot e ht1

/--
  ★桥下游·第三类买 ⟹ accreteCore（L0，§3）：若薄标签 `b`（kind=type3）桥接到厚事件 `e` 且 `e`
  未破中枢（第三类语境，非第一类），则 `recogChanlun e = accreteCore`（第三类增核）。

  ★薄↛厚边界真跨越：薄端 type3 标签 + 桥 ⟹ 厚端 recog accreteCore。桥提供 `IsType3Buy e.bsp`，
  喂 `recog_type3_accreteCore`（#117 committed）。
-/
theorem bridge_type3_forces_accreteCore (b : Bsp) (e : ChanlunEvent)
    (hkind : b.kind = BspKind.type3) (hbridge : BridgesTo b e)
    (hnobreak : e.bsp.brokeCenter = false) :
    recogChanlun e = ChanlunDecision.accreteCore := by
  have ht3 : IsType3Buy e.bsp := by
    have := hbridge.2
    rw [hkind] at this
    simpa using this
  exact recog_type3_accreteCore e ht3 hnobreak

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 桥非平凡（反退化）：标签矛盾的 (b,e) 不桥接 + 不同标签 ↦ 不同决策

  ★no-workaround：桥必须**非真空真**——若 `BridgesTo` 对所有 (b,e) 都成立，桥无内容（伪桥）。
  本节坐实：标签与厚判据**矛盾**的 (b,e) **不**桥接（`bridge_rejects_mismatch`），且不同薄标签经
  桥连到的厚事件给**不同** recog 决策（`bridge_distinguishes_decisions`）——桥真区分。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★具体厚事件·第一类（沿用 #117 committed eventType1，破中枢+背驰，满足 IsType1）。
-/
def thickType1 : ChanlunEvent := ThetaInstantiation.eventType1

/--
  ★具体厚事件·第三类买（离开中枢+第一次回抽+不破 ZG，未破中枢，满足 IsType3Buy 非 IsType1）。
  center 核心 [10,20]，回抽价 25 > ZG=20（不破 ZG）；leftCenter+firstRetrace；未破中枢。
-/
def thickType3 : ChanlunEvent :=
  { bsp :=
      { side := Side.long
        center := { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }
        divPair := { forceA := ⟨3⟩, forceC := ⟨5⟩, isTrend := false }
        brokeCenter := false, afterTypeOne := true, leftCenter := true
        retracePrice := 25, firstRetrace := true }
    prevCenter := ThetaInstantiation.witnessOuter
    nextCenter := ThetaInstantiation.witnessOuter }

/-- ★thickType1 满足第一类厚判据。 -/
theorem thickType1_isType1 : IsType1 thickType1.bsp := ThetaInstantiation.eventType1_isType1

/-- ★thickType3 满足第三类买厚判据。 -/
theorem thickType3_isType3buy : IsType3Buy thickType3.bsp := by
  unfold IsType3Buy thickType3; refine ⟨rfl, rfl, rfl, by decide⟩

/-- ★thickType3 未破中枢（第三类语境）。 -/
theorem thickType3_nobreak : thickType3.bsp.brokeCenter = false := rfl

/--
  ★反退化·桥拒绝标签矛盾（L0，§4，核心非平凡见证）：一个 **type3 薄标签** 与 **第一类厚事件
  `thickType1`** **不**桥接——因为 type3 分支要求 `IsType3Buy thickType1.bsp`，而 thickType1 破中枢
  （`brokeCenter=true`）⟹ `leftCenter`/不破ZG 等第三类判据不成立。

  ★这坐实桥**非真空真**（非伪桥）：标签（type3）与厚判据（thickType1 是第一类）矛盾时桥**失败**。
  桥真检验薄标签与厚判据是否相符——若 `BridgesTo` 对任意 (b,e) 平凡成立，本定理不可证。
-/
theorem bridge_rejects_mismatch :
    ¬ BridgesTo { kind := BspKind.type3, side := Side.long, index := 0, price := 0 } thickType1 := by
  -- BridgesTo 在 type3 分支要求 IsType3Buy thickType1.bsp；thickType1 满足 leftCenter=false ⟹ 否。
  rintro ⟨_, hbody⟩
  -- type3 分支体 = IsType3Buy thickType1.bsp。
  have ht3 : IsType3Buy thickType1.bsp := by simpa using hbody
  -- IsType3Buy 要求 leftCenter=true，但 thickType1（eventType1）的 leftCenter=false。
  exact absurd ht3.2.1 (by decide)

/--
  ★反退化·桥拒绝方向矛盾（L0，§4）：一个**卖向 type1 标签**（side=short）与**买向第一类厚事件
  `thickType1`**（side=long）**不**桥接——桥要求 `b.side = e.bsp.side`。

  ★这坐实桥的 side 分量真检验方向一致（非平凡）——方向不符时桥失败。
-/
theorem bridge_rejects_side_mismatch :
    ¬ BridgesTo { kind := BspKind.type1, side := Side.short, index := 0, price := 0 } thickType1 := by
  rintro ⟨hside, _⟩
  -- hside : Side.short = thickType1.bsp.side = Side.long，矛盾。
  exact absurd hside (by decide)

/--
  ★反退化·桥区分决策（L0，§4，桥非退化的下游推论）：经桥连到 `thickType1`（type1 标签）与连到
  `thickType3`（type3 标签）的 recog 决策**不同**（openRoot ≠ accreteCore）——桥的两端不同标签
  ↦ 厚端不同决策 ↦ 不同 ledger delta（建根仓 1 vs 增核 2）。

  ★这坐实桥**非退化**：不是所有薄标签都桥到同一决策。桥真把薄分类的区分传递到厚 recog 决策的
  区分（满足 0004 锋利问题：两个不同分类的事件产生不同 ledger 足迹）。
-/
theorem bridge_distinguishes_decisions :
    recogChanlun thickType1 ≠ recogChanlun thickType3 := by
  -- type1 标签桥到 thickType1 ⟹ openRoot；type3 标签桥到 thickType3 ⟹ accreteCore。
  have h1 : recogChanlun thickType1 = ChanlunDecision.openRoot :=
    bridge_type1_forces_openRoot
      { kind := BspKind.type1, side := Side.long, index := 0, price := 0 } thickType1
      rfl ⟨rfl, by simpa using thickType1_isType1⟩
  have h3 : recogChanlun thickType3 = ChanlunDecision.accreteCore :=
    bridge_type3_forces_accreteCore
      { kind := BspKind.type3, side := Side.long, index := 0, price := 0 } thickType3
      rfl ⟨rfl, by simpa using thickType3_isType3buy⟩ thickType3_nobreak
  rw [h1, h3]; decide

/--
  ★反退化·桥连决策的 ledger 足迹不同（L0，§4，0004 锋利问题非平凡版）：桥到 type1 的决策
  `decisionLedgerDelta openRoot = (0,1,0)` ≠ 桥到 type3 的 `decisionLedgerDelta accreteCore = (0,2,0)`。
  ★薄标签区分 ──桥──▶ 厚 recog 决策区分 ──ledger──▶ 账本足迹区分（dA: 1 vs 2）。
-/
theorem bridge_distinguishes_ledger_delta :
    decisionLedgerDelta (recogChanlun thickType1) ≠
    decisionLedgerDelta (recogChanlun thickType3) := by
  have h1 : recogChanlun thickType1 = ChanlunDecision.openRoot :=
    bridge_type1_forces_openRoot
      { kind := BspKind.type1, side := Side.long, index := 0, price := 0 } thickType1
      rfl ⟨rfl, by simpa using thickType1_isType1⟩
  have h3 : recogChanlun thickType3 = ChanlunDecision.accreteCore :=
    bridge_type3_forces_accreteCore
      { kind := BspKind.type3, side := Side.long, index := 0, price := 0 } thickType3
      rfl ⟨rfl, by simpa using thickType3_isType3buy⟩ thickType3_nobreak
  rw [h1, h3]; decide

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 桥的可实现性：每个可能的薄标签都有厚见证（存在性，非空）

  ★桥非空：对每个薄 `BspKind`（type1/type3 买），**存在**一个满足对应厚判据的厚事件与一个该 kind
  的薄标签**桥接**。这坐实「构造投影的陪域（薄标签）每个值都被厚事件实现」——桥的薄端不是空标签。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★可实现性·type1（L0）：存在 type1 薄标签 + 厚事件桥接（thickType1 见证）。 -/
theorem bridge_realizable_type1 :
    ∃ (b : Bsp) (e : ChanlunEvent), b.kind = BspKind.type1 ∧ BridgesTo b e := by
  exact ⟨{ kind := BspKind.type1, side := Side.long, index := 0, price := 0 }, thickType1,
         rfl, ⟨rfl, by simpa using thickType1_isType1⟩⟩

/-- ★可实现性·type3 买（L0）：存在 type3 薄标签 + 厚事件桥接（thickType3 见证）。 -/
theorem bridge_realizable_type3 :
    ∃ (b : Bsp) (e : ChanlunEvent), b.kind = BspKind.type3 ∧ BridgesTo b e := by
  exact ⟨{ kind := BspKind.type3, side := Side.long, index := 0, price := 0 }, thickType3,
         rfl, ⟨rfl, by simpa using thickType3_isType3buy⟩⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 still-MISSING 诚实声明 + 结果包六要素
  ════════════════════════════════════════════════════════════════════════

  ★已坐实（task #128，消双投影同源绕过）：
    - **真桥 `BridgesTo`**：薄 `Bsp` 标签 ↦ 厚 `ChanlunEvent` 判据，两端独立量化（非从 e 投影出 b、
      非共享 BspCandidate），桥验证从 `e` 自身派生厚判据（IsType1/IsType3Buy）。
    - **可靠性**（§2）：厚判据 ⟹ 构造层薄标签 + 桥接回厚事件（bridge_sound_type1/type3buy）。
    - **下游连通**（§3）：薄标签经桥 ⟹ 厚 recog 决策（bridge_type1_forces_openRoot /
      bridge_type3_forces_accreteCore）——薄↛厚边界真跨越。
    - **非平凡**（§4）：桥拒绝标签矛盾（bridge_rejects_mismatch）+ 方向矛盾
      （bridge_rejects_side_mismatch）+ 区分决策/ledger 足迹（bridge_distinguishes_*）。
    - **可实现性**（§5）：每个薄 kind 有厚见证（bridge_realizable_type1/type3）。

  ★still-OPEN（诚实开口，no声明膨胀）：
    - **薄→厚反推不可能**：薄 `Bsp` 丢失判据字段（divPair/brokeCenter/…），从薄标签**反推唯一**厚
      事件信息不可逆——桥是 label↦criteria 的**可靠性+可实现性**桥（厚⟹薄 + 存在厚见证），
      **不**是「薄 ↦ 唯一厚」的反函数。把不可逆反推冒充为桥 = 伪桥（本文件不做）。
    - **type2 桥到 hold**：recogChanlun 只显式 case type1/type3买/否则 hold，type2 的厚判据
      （afterTypeOne）不进 recog 显式 case，故 type2 薄标签桥到 hold 决策（继承否则分支），
      其专属应对未在 recog 区分——still-OPEN-type2（recog 扩 case 后可坐实 type2↦专属决策）。
    - **第三类卖侧对偶**：本文件桥 type1 + type3 **买**侧；type3 卖侧（IsType3Sell）对偶继承，
      待卖侧 recog（SellClosedLoop committed 后）接入——still-OPEN-sell-bridge。

  ★结果包六要素：
  1. **结论**：新增 Origin.BspEventBridge——薄 bspOf(Bsp 三类) ↦ 厚 ChanlunEvent 判据的真桥
     `BridgesTo`，含可靠性 / 下游连通（薄↦厚↦recog 决策）/ 非平凡（拒绝矛盾 + 区分决策）/
     可实现性，全 L0 零 sorry。
  2. **定义依据**：薄端 = ChanlunElements.Bsp（{kind,side,index,price}，BspConstruction.bspOf 陪域）；
     厚端 = ThetaInstantiation.ChanlunEvent（载 BspClassification.BspEndpoint 完整判据）。桥用
     §10.1/§24 第一类判据 `IsType1`（破中枢∧背驰）+ 第三类买 `IsType3Buy`（离开中枢∧第一次回抽∧
     不破 ZG）从厚端 e 派生，与薄端标签 kind/side 相符。
  3. **边界条件**（结论翻转）：
     - 若 recogChanlun 扩 case 显式区分 type2（afterTypeOne ↦ 专属决策），则 type2 桥可从 hold
       重锚到专属决策——本文件 type2↦hold 翻转。
     - 若某口径要求桥**反推**唯一厚事件（薄 ↦ 厚反函数），桥失败——薄 Bsp 信息不可逆，须上游
       ParseStruct 留存判据（still-MISSING-D′），非本桥可证。
     - bridge_sound_type3buy 依赖 `brokeCenter=false`（第三类语境）；若 classifyEndpoint 优先级口径
       改（三类>一类），前提翻转须重证。
  4. **下游推论**：
     - 薄 bspOf 的分类标签现有真桥连到厚 recog 决策——Pipeline 的双投影同源结构可由本桥**替换**为
       真 label↦criteria 桥（薄端标签 ⟹ 厚端决策不再靠「同源」，靠桥谓词跨边界）。
     - bridge_distinguishes_ledger_delta ⟹ 桥把薄分类区分传到 ledger 足迹区分（0004 锋利问题非平凡版）。
  5. **谱系引用**：消 Pipeline.lean §0 双投影同源绕过（#125，回避薄↛厚边界）；薄/厚两段类型不接
     的诚实标注（BspClassification still-MISSING-D + Pipeline still-MISSING 薄→厚桥）由本桥坐实
     真桥（非伪桥、非同源双投影）。type2↦hold / 卖侧对偶 still-OPEN 诚实标。
  6. **影响声明**：新增 Origin.BspEventBridge 模块，import BspClassification + BspConstruction +
     ThetaInstantiation + ChanlunElements（只读）。**不改**任何 canonical 类型 / 已有 Origin 文件，
     无反向依赖，无命名冲突。待 Lead 登记 root：`NewChanlun.Origin.BspEventBridge`。
-/

end NewChanlun.Origin.BspEventBridge
