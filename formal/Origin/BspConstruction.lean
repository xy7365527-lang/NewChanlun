/-
Origin/BspConstruction.lean — bsp 全自动构造（遍历识别三类 + 第二类本级别判据签名）+ 终止性 + 唯一性（task #116）
  + 632号路1 canonical 修复（brokeCenter/leftCenter L0 价格几何推导 + afterTypeOne 前序折叠 + divPair L2 开口）

★工位定位（#113 still-MISSING-D 缺口）：BspClassification.lean 形式化了买卖点三类**判据层**
  （每类中枢关系 + 背驰 + 单射裁定 + 完备性）；但 `bspOf : List BspEndpoint → List Bsp` 的
  **全自动遍历识别**（对每个端点判属哪类）的**终止性**与**输出唯一性**未证。本文件实装：
  (1) 遍历识别为结构递归（List 上结构终止）；
  (2) 第二类"由次级别第一类构成"（买卖点定律一，§10.2）的**单步本级别判据签名**
      （`secondTypeViaSublevel`：本级别 IsType2 ∧ 次级别第一类**占位**）。

  ★诚实声明（消声明膨胀，no-patch-mentality）：(2) **不是** well-founded 级别下降递归——
  其 `subLevelHasType1 n e` 丢弃级别索引 n、两分支同体、不自调，无下降递归结构。
  「次级别真下钻」（从次级别 ParseStruct 取真正的次级别第一类）是 **still-MISSING-D′**（未实装）。

═══════════════════════════════════════════════════════════════════════════
构造算法（§10.1 三类识别 + §10.2 买卖点定律一本级别判据签名）
═══════════════════════════════════════════════════════════════════════════
- 遍历识别：对每个候选端点，按 BspClassification 判据（IsType1/IsType2/IsType3）判类，
  产出 `Bsp`（type1/2/3 + side + index + price）。**终止性**：结构递归消费列头，结构终止。
- 第二类本级别判据（买卖点定律一）："任何级别的第二类买卖点都由次级别相应走势的第一类构成。"
  本文件把它装配为**单步本级别判据签名**：本级别(level n)第二类 = 本级别 IsType2 ∧ 次级别第一类
  **占位**（`subLevelHasType1`，丢弃 n）。次级别真下钻（级别真正递减到基底）是 still-MISSING-D′，
  **未实装**——本文件不冒充该下钻已成（见 SubLevelDescent.lean 的 descend 真级别递减结构基础）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
§1-6 全 **L0**（纯定义 / 结构递归终止 / Bool 全函数二歧 / 纯函数唯一性，不依赖数据）。
§7（632号路1 canonical 修复，codex 异质裁决）分层：
  · `brokeCenterOf`/`leftCenterOf`（§7.1）**L0**：从 `Move.endPrice` vs 中枢 [zd,zg] 用 608号
    位置三态（IsBelow/IsAbove）纯价格几何推导——`Move` 加 endPrice 字段后 brokeCenter/leftCenter
    **不再 oracle 注入**，真 L0 结构推导（`brokeCenterOf_iff` 坐实锚到 608号）。
  · `afterTypeOne`（§7.3）**L0**：在 `bspOfMovesAux` 遍历 `List Move` 时由前序状态折叠产生
    （列表级时序量，纯结构折叠），**不是**单 `Move` 原子字段。
  · `divPair`（`MoveForceJudgment`）**L2 开口**：背驰力度 `Force`/MACD 面积无 Origin 计算引擎
    （Divergence still-MISSING-C，需 EMA/DIF/DEA + 面积积分）——诚实不填，标 L2，不冒充已填。
`lake env lean Origin/BspConstruction.lean` 通过 = bspOf 全函数良定义（终止）+ 输出唯一（确定性）
+ brokeCenter/leftCenter 价格几何 L0 推导成立，**不是**任何"识别的买卖点真对应缠论权威标注"的
实证断言（L2+），**也不是** divPair 力度已计算（still-MISSING-C），**也不是**次级别真下钻终止性
证明（still-MISSING-D′）。

诚实标注（gatekeeper，no-patch-mentality）：
★ L0PriceGeometry + ListLevelFold + DivPairL2Open + SubLevelIsSingleStepPlaceholder ——
  本文件证 bspOf 遍历的**结构终止 + 唯一性** + brokeCenter/leftCenter 价格几何 L0 推导（632号路1）
  + afterTypeOne 列表级前序折叠。divPair 力度诚实留 L2 开口（still-MISSING-C，不冒充）。第二类
  「次级别第一类」用 level-indexed **单步占位判据**（`subLevelHasType1`：丢弃 n、两分支同体、不自调）
  ——**不是** well-founded 级别下降递归，**不**实装"从本级别 ParseStruct 下钻到次级别 ParseStruct
  重新跑 segmentsOf/centersOf"（still-MISSING-D′）。把单步占位判据冒充为完整下钻递归 / 把 divPair
  冒充为已 L0 推导 = 声明膨胀（禁止）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.Divergence
import Origin.BspClassification

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 三类判据的可判定封装（遍历识别需 Decidable）
    ═══════════════════════════════════════════════════════════════════════ -/

instance (e : BspEndpoint) : Decidable (IsType1 e) := by
  unfold IsType1; exact inferInstanceAs (Decidable (_ ∧ _))

instance (e : BspEndpoint) : Decidable (IsType3Buy e) := by
  unfold IsType3Buy
  simp only [Tick]
  exact inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _))

instance (e : BspEndpoint) : Decidable (IsType3Sell e) := by
  unfold IsType3Sell
  simp only [Tick]
  exact inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _))

instance (e : BspEndpoint) : Decidable (IsType3 e) := by
  unfold IsType3; exact inferInstanceAs (Decidable (_ ∨ _))

instance (e : BspEndpoint) : Decidable (IsType2 e) := by
  unfold IsType2; exact inferInstanceAs (Decidable (_ ∧ _))

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 单端点分类（识别优先级：一类 > 三类 > 二类）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **遍历输入端点** —— 候选端点 + 端点下标 + 端点价（产出 Bsp 所需位置数据）。
-/
structure BspCandidate where
  endpoint : BspEndpoint
  index : Index
  price : Tick
deriving Repr

/--
  **单端点分类（§10.1，识别优先级）** —— 一个候选端点 ↦ 至多一个 Bsp。
  优先级（处理 2/3 共存的 V 型反转，`no_exclusive_trichotomy`）：一类 > 三类 > 二类。
  ★这是 BspClassification 单射裁定的**操作侧消解**：判据层 2/3 可重合（互斥三分失败），
  构造层用确定性优先级使输出唯一——优先级是 still-MISSING-D 精化触发签名的最简实例。
  无类匹配 ⟹ none。
-/
def classifyEndpoint (c : BspCandidate) : Option Bsp :=
  let e := c.endpoint
  if IsType1 e then
    some { kind := BspKind.type1, side := e.side, index := c.index, price := c.price }
  else if IsType3 e then
    some { kind := BspKind.type3, side := e.side, index := c.index, price := c.price }
  else if IsType2 e then
    some { kind := BspKind.type2, side := e.side, index := c.index, price := c.price }
  else
    none

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. bspOf 遍历构造（结构递归，结构终止）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★bspOf 全自动遍历构造（task #116，结构终止）** —— `List BspCandidate → List Bsp`。
  遍历候选端点列，对每个端点 `classifyEndpoint` 判类，收集非 none 的结果。
  **终止性**：结构递归（消费列头 `c :: rest` → 递归 `rest`），结构终止（List.rec）——
  Lean 直接接受（无需 termination_by），消解 #113 still-MISSING-D 遍历终止性缺口。
  ★唯一性：纯全函数 ⟹ 输出唯一（`bspOf_total_unique`，§ 5）。
-/
def bspOf : List BspCandidate → List Bsp
  | [] => []
  | c :: rest =>
      match classifyEndpoint c with
      | some b => b :: bspOf rest
      | none => bspOf rest

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 第二类本级别判据（买卖点定律一，§10.2）：单步本级别签名（次级别真下钻 still-MISSING-D′）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **本级别第一类判据（单步，非次级别下钻）** —— 买卖点定律一："任何级别的第二类买卖点都由
  次级别相应走势的第一类买卖点构成。" 本函数**只**在端点 `e` 上判 `IsType1`——它对级别索引
  `n` 不做任何下钻：两个分支体逐字相同（`decide (IsType1 e)`），`n` 被丢弃，函数**不调用自身**。

  ★诚实声明（消声明膨胀，no-patch-mentality）：这**不是**「次级别真下钻」——「从本级别
  ParseStruct 下钻到次级别 ParseStruct 重跑 segmentsOf/centersOf 取次级别第一类」是
  **still-MISSING-D′**（未实装，需 RecursiveLevelSystem 全实例化，见 #113/§头部诚实标注）。
  本函数是该接口的**单级别占位判据**：在缺次级别 ParseStruct 时，用本级别 `IsType1` 充当
  «次级别第一类» 的可判定签名，级别索引 `n` 当前不携带信息（占位，待真下钻接入）。
-/
def subLevelHasType1 : Nat → BspEndpoint → Bool
  | 0, e => decide (IsType1 e)
  | _ + 1, e => decide (IsType1 e)  -- 占位：n 被丢弃，两分支同体，无次级别下钻（still-MISSING-D′）

/--
  **★第二类本级别判据（买卖点定律一，单步本级别——非次级别真下钻）** —— level n 的第二类签名 =
  本级别 `IsType2` 可观测条件成立 ∧ `subLevelHasType1 n e`（后者是上面的单级别占位判据，丢弃 n）。

  ★诚实声明（消声明膨胀，no-patch-mentality）：本函数对级别索引 `n` 做的是 `Nat.casesOn`
  （区分 0 / n+1 两个分支体），**不是** well-founded 级别下降递归——`subLevelHasType1 n e` 不下钻、
  不自调，故整体无「级别严格递减到基底」的递归结构。买卖点定律一的「次级别真下钻」
  （从次级别 ParseStruct 取真正的次级别第一类）是 **still-MISSING-D′**（未实装）。本函数只把
  «本级别 IsType2 ∧ 次级别第一类占位» 装配为单步判据签名，待真下钻接入后此处替换为真递归。
-/
def secondTypeViaSublevel : Nat → BspEndpoint → Bool
  | 0, e => decide (IsType2 e)
  | n + 1, e => decide (IsType2 e) && subLevelHasType1 n e

/--
  **★secondTypeViaSublevel 是 Bool 全函数（二歧，L0——非终止性证明）** —— 对任意级别 n + 端点，
  `secondTypeViaSublevel n e` 取值 true 或 false（`Bool` 只有这两个构造子）。

  ★诚实声明：这是 `Bool` 类型的**穷尽二歧**（cases 在 Bool 上），**不是**「级别下降递归终止性」
  的证明——`secondTypeViaSublevel` 本就无下降递归（`subLevelHasType1` 丢弃 n、不自调），无终止
  义务可证。本定理只坐实它是返回 Bool 的全函数（type-checks 即全），不冒充终止性论证。
-/
theorem secondType_is_bool (n : Nat) (e : BspEndpoint) :
    secondTypeViaSublevel n e = true ∨ secondTypeViaSublevel n e = false := by
  cases secondTypeViaSublevel n e
  · exact Or.inr rfl
  · exact Or.inl rfl

/--
  **★第二类次级别构成（买卖点定律一，L0）** —— 在 level n+1，第二类成立 ⟹ 次级别有第一类。
  这把"任何级别第二类由次级别第一类构成"形式化为蕴含（第二类 → 次级别第一类）。
-/
theorem secondType_needs_sublevel_type1 (n : Nat) (e : BspEndpoint)
    (h : secondTypeViaSublevel (n + 1) e = true) :
    subLevelHasType1 n e = true := by
  unfold secondTypeViaSublevel at h
  exact (Bool.and_eq_true _ _ |>.mp h).2

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 输出唯一性（确定性）：纯全函数 ⟹ 输出唯一
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★bspOf 输出唯一性（task #116，确定性，L0）** —— 给定候选端点列，bspOf 输出唯一确定。
  这是 ChanlunElements.bsp_total_unique 平凡桩的**非平凡见证**：bspOf 是上面结构终止的**具体**
  全自动遍历构造（带优先级分类），确定性由"纯函数对每输入恰有一个输出"导出。
-/
theorem bspOf_total_unique :
    TotalUnique (fun cands out => bspOf cands = out) :=
  total_unique_of_fun bspOf

/-- **★全性（终止性可观测推论，L0）** —— bspOf 对任意输入返回（不发散）。 -/
theorem bspOf_total :
    Total (fun cands out => bspOf cands = out) := by
  intro cands; exact ⟨bspOf cands, rfl⟩

/-- **★单值性（确定性，L0）** —— 同一候选列不产生两个不同买卖点序列。 -/
theorem bspOf_single_valued :
    SingleValued (fun cands out => bspOf cands = out) :=
  (total_and_single_of_total_unique bspOf_total_unique).2

/-- **★secondTypeViaSublevel 确定性（L0）** —— 本级别判据是纯全函数（非递归）⟹ 输出唯一。 -/
theorem secondType_total_unique :
    TotalUnique (fun (ne : Nat × BspEndpoint) (out : Bool) => secondTypeViaSublevel ne.1 ne.2 = out) :=
  total_unique_of_fun (fun ne : Nat × BspEndpoint => secondTypeViaSublevel ne.1 ne.2)

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 计算见证（反退化：具体端点 ⟹ 具体分类，非平凡）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 一类候选：破中枢 + 背驰（沿用 BspClassification.sampleType1）。 -/
def cand1 : BspCandidate :=
  { endpoint := sampleType1, index := 3, price := 5 }

/-- ★反退化见证：cand1 识别为第一类买卖点（type1）。 -/
theorem witness_classify_type1 :
    classifyEndpoint cand1 = some { kind := BspKind.type1, side := Side.long, index := 3, price := 5 } := by
  unfold classifyEndpoint cand1
  rw [if_pos witness_type1]
  rfl

/-- ★反退化见证：bspOf 对单候选列产出恰一个 type1 买卖点（遍历真跑通）。 -/
theorem witness_bspOf_single : (bspOf [cand1]).length = 1 := by
  unfold bspOf
  rw [witness_classify_type1]
  rfl

/-- ★反退化见证：bspOf 空候选列 ⟹ 空（边界）。 -/
theorem witness_bspOf_empty : bspOf [] = [] := by unfold bspOf; rfl

/-- ★反退化见证：第二类本级别判据在 level 0 / level 1 都返回 Bool（二歧全函数，非终止性见证）。 -/
theorem witness_secondType_is_bool :
    secondTypeViaSublevel 1 sampleType1 = true ∨ secondTypeViaSublevel 1 sampleType1 = false :=
  secondType_is_bool 1 sampleType1

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. ElementPipeline 桥接：Move → BspCandidate（brokeCenter/leftCenter L0 价格几何推导
         + afterTypeOne 列表级前序折叠 + divPair L2 开口）
    ═══════════════════════════════════════════════════════════════════════ -/

/-! ─── § 7.1 brokeCenter / leftCenter：从 Move 价格端点 vs 中枢的 L0 几何推导（608号位置三态）─── -/

/--
  **★破中枢判据（L0 价格几何，608号位置三态）** —— 走势末端价 `m.endPrice` 落在中枢 `c` 核心
  `[zd,zg]` **之外**（之下或之上）⟺ 走势已跌破/突破该中枢。

  ★632号路1 兑现：原 `brokeCenter` 是 oracle Bool 字段（无法从无价格端点的 `Move` 推导）；
  `Move` 加 `endPrice`（透传自 `Segment.endPrice`）后，破中枢从 608号 CenterStates 的
  `IsBelow`/`IsAbove`（点相对 [zd,zg] 的位置三态）**纯 L0 结构推导**，消除 oracle 注入。
  「之外」= `IsBelow ∨ IsAbove` = `classifyPosition ≠ within`——位置三态的直接应用。

  ★认识论等级 L0：纯整数比较（价格几何），不依赖经验数据/MACD/Θ 参数。
-/
def brokeCenterOf (m : Move) (c : Center) : Bool :=
  decide (m.endPrice < c.zd ∨ c.zg < m.endPrice)

/--
  **★离开中枢判据（L0 价格几何，608号位置三态）** —— 走势末端价落在中枢核心 `[zd,zg]` 之外
  ⟺ 走势已离开该中枢。与 `brokeCenterOf` 同几何基准（末端价 vs [zd,zg]），区别在语境：
  `brokeCenter` 用于第一类（破中枢后背驰），`leftCenter` 用于第三类（离开中枢后回抽）——
  二者在「末端价是否在中枢之外」这一**位置三态判据**上同构（608号 IsBelow/IsAbove）。

  ★认识论等级 L0：与 brokeCenterOf 同为价格几何结构推导，不依赖经验数据。
-/
def leftCenterOf (m : Move) (c : Center) : Bool :=
  decide (m.endPrice < c.zd ∨ c.zg < m.endPrice)

/--
  **★破中枢 L0 推导坐实（位置三态应用，L0）** —— `brokeCenterOf m c = true` ⟺ 末端价在中枢之外
  （608号位置三态 `IsBelow c p ∨ IsAbove c p`：之下或之上 = ¬之中）。坐实 L0 几何推导非桩——
  `brokeCenterOf` 的底层整数析取 `endPrice < zd ∨ zg < endPrice` 按定义即 `IsBelow ∨ IsAbove`。
-/
theorem brokeCenterOf_iff (m : Move) (c : Center) :
    brokeCenterOf m c = true ↔ (IsBelow c m.endPrice ∨ IsAbove c m.endPrice) := by
  unfold brokeCenterOf IsBelow IsAbove; exact decide_eq_true_iff

/-! ─── § 7.2 走势判据 oracle 接口（仅 divPair/retracePrice/firstRetrace 留 L2 开口）─── -/

/--
  **走势力度判据注入（L2 oracle 接口，仅背驰相关）** —— 632号路1 后，`Move` 携价格端点，
  价格几何字段（brokeCenter/leftCenter）已可 L0 推导；时序字段（afterTypeOne）由列表级前序
  折叠产生。**只剩**「背驰力度」相关数据无法从 `Move` 几何推导，需上游真实计算注入：

  - `divPair`（背驰段对）= **真 L2 缺口**：`Force`/MACD 面积无 Origin 计算引擎
    （Divergence still-MISSING-C，需 EMA/DIF/DEA + 面积积分）。不填、诚实标注 L2 开口。
  - `retracePrice`/`firstRetrace`（第三类回抽极值/是否首抽）：回抽极值价是次级别走势的几何量，
    本级别 `Move` 末端价不直接等于回抽极值——保留为上游注入（携带次级别几何的载体提供）。

  ★诚实声明（no-patch-mentality + 632号路1 兑现）：本结构**只**声明「无法从本级别 Move 几何
  L0 推导的力度/回抽字段」——brokeCenter/leftCenter（价格几何，§7.1 L0 推导）+ afterTypeOne
  （时序，§7.3 前序折叠）**不再**在此结构中（已从 oracle 降为推导/折叠产物，632号路1）。
  把 divPair 力度冒充为已填 = 声明膨胀（禁止）——`divPair` still-MISSING-C，诚实留 L2。
-/
structure MoveForceJudgment where
  side : Side
  divPair : DivergencePair
  retracePrice : Tick
  firstRetrace : Bool
deriving Repr

/--
  **走势 + 中枢 + 力度判据 + 前序时序 → 买卖点候选端点（桥接函数，L0 几何 + L2 力度开口）** ——
  组装 `BspEndpoint`：brokeCenter/leftCenter 从 `Move` 价格端点 vs `center` L0 推导（§7.1），
  afterTypeOne 由调用方传入的前序状态 `afterT1` 折叠产生（§7.3），力度/回抽字段从 `MoveForceJudgment`
  注入（L2 开口）。`Move.endIndex`/`Move.endPrice` 对应候选买卖点的下标/价。
-/
def moveToCandidate (m : Move) (center : Center) (force : MoveForceJudgment) (afterT1 : Bool) :
    BspCandidate :=
  { endpoint :=
      { side := force.side
        center := center
        divPair := force.divPair
        brokeCenter := brokeCenterOf m center      -- L0 价格几何推导（608号位置三态）
        afterTypeOne := afterT1                     -- 列表级前序折叠产物（§7.3）
        leftCenter := leftCenterOf m center         -- L0 价格几何推导（608号位置三态）
        retracePrice := force.retracePrice
        firstRetrace := force.firstRetrace }
    index := m.endIndex
    price := m.endPrice }

/-! ─── § 7.3 bspOfMoves：列表级前序折叠（afterTypeOne 由前序「是否已出现第一类」产生）─── -/

/--
  **★bspOfMoves（L0 价格几何 + 列表级前序折叠 afterTypeOne + L2 力度开口）** ——
  `List Move → List Center → List MoveForceJudgment → List Bsp`。遍历 `(Move, Center, Force)`
  三元组，**维护前序状态** `afterT1`（前序是否已出现第一类买卖点），对每个走势：
  (1) 用 `moveToCandidate`（brokeCenter/leftCenter L0 推导 + afterTypeOne := 当前前序状态）组装端点；
  (2) `classifyEndpoint` 判类；
  (3) **折叠更新前序状态**：若本走势识别为第一类 ⟹ 后续 `afterT1 := true`（第一类已出现）。

  ★afterTypeOne 前序折叠（632号路1 codex 裁决）：`afterTypeOne` **不是**单个 `Move` 的原子字段，
  而是走势**序列**遍历时由前序状态折叠产生的**列表级时序量**——第二类「在第一类之后」是序列级
  时序关系，非单走势内蕴属性。本函数把它装配为 fold：前序出现第一类 ⟹ 翻转 afterT1。

  ★终止性：结构递归消费三元组列头，结构终止。
  ★认识论等级：brokeCenter/leftCenter L0（价格几何）；afterTypeOne L0（列表级折叠，纯结构）；
  divPair L2 开口（force 注入时携带的背驰力度数据，still-MISSING-C，本函数不产生力度信息增量）。
-/
def bspOfMovesAux :
    List Move → List Center → List MoveForceJudgment → Bool → List Bsp
  | [], _, _, _ => []
  | _, [], _, _ => []
  | _, _, [], _ => []
  | m :: ms, c :: cs, f :: fs, afterT1 =>
      let cand := moveToCandidate m c f afterT1
      match classifyEndpoint cand with
      | some b =>
          let afterT1' := afterT1 || decide (b.kind = BspKind.type1)
          b :: bspOfMovesAux ms cs fs afterT1'
      | none => bspOfMovesAux ms cs fs afterT1

/--
  **★bspOfMoves（顶层，前序状态初始 false）** —— 走势序列起始时尚未出现第一类（afterT1 := false）。
-/
def bspOfMoves (moves : List Move) (centers : List Center) (forces : List MoveForceJudgment) :
    List Bsp :=
  bspOfMovesAux moves centers forces false

/-- ★bspOfMoves 输出唯一性（L0，确定性：纯全函数 ⟹ 输出唯一）。 -/
theorem bspOfMoves_total_unique :
    TotalUnique (fun (t : List Move × List Center × List MoveForceJudgment) (out : List Bsp) =>
      bspOfMoves t.1 t.2.1 t.2.2 = out) :=
  total_unique_of_fun
    (fun t : List Move × List Center × List MoveForceJudgment => bspOfMoves t.1 t.2.1 t.2.2)

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. 632号路1 兑现：brokeCenter/leftCenter L0 消桩 + afterTypeOne 前序折叠 + divPair L2 开口

    ★629号识别"未验证 oracle 冒充完整分类"开口；632号精确化根因：元素层 `Move` **缺价格端点**
    （`movesOf` 丢弃了 `Segment.startPrice/endPrice`），brokeCenter/leftCenter（价格几何判据）
    无法从 `Move` 单独推导，只能 oracle 注入——这是 canonical 契约缺陷（codex 异质裁决）。

    **632号路1 修复（codex 裁决，acceptance #2）**：给 canonical `Move` 加 `startPrice/endPrice`
    （透传自 `Segment` 端点，ChanlunElements.lean）。修复后逐字段消解：

    · `brokeCenter`/`leftCenter`（§7.1）：从 `Move.endPrice` vs 中枢 [zd,zg] 用 608号位置三态
      `IsBelow/IsAbove` **纯 L0 几何推导**（`brokeCenterOf`/`leftCenterOf`），**不再 oracle 注入**。
      原 oracle 注入的价格几何分量已删除，替换为 608 真结构推导（`brokeCenterOf_iff` 坐实）。
    · `afterTypeOne`（§7.3）：**不是**单个 `Move` 的原子字段，改为在 `bspOfMovesAux` 遍历
      `List Move` 时由**前序状态折叠**产生（前序出现第一类 ⟹ afterT1 翻转）——列表级时序量。
      原 `MoveJudgment.afterTypeOne` Bool 字段已删除（不再当单走势内蕴属性）。
    · `divPair`（背驰力度）：保持**明确 L2 开口**（`MoveForceJudgment.divPair`，still-MISSING-C：
      MACD/EMA 引擎无 Origin 计算）——不填、诚实标注，不与 L0 几何捆成大 oracle（codex 裁决）。

    原 `MoveJudgment`（把 brokeCenter/leftCenter/afterTypeOne 当 oracle Bool 字段）+
    `moveJudgmentToCandidate` + `bspSampleJudgment1` + 旧 `bspOfMoves`（zip+filterMap，无前序折叠）
    已**删除/重构**——它们把可 L0 推导的价格几何（632号路1 后）+ 可折叠的时序冒充为 oracle 注入。
    新 `MoveForceJudgment` **只**保留无法从 `Move` 几何推导的力度/回抽字段（divPair L2 开口）。
    ═══════════════════════════════════════════════════════════════════════ -/

/-! ═══════════════════════════════════════════════════════════════════════
    § 9. L0/L2 合成计算见证（具体 Move 价格端点 ⟹ L0 几何推导 brokeCenter ⟹ 具体 Bsp 输出）
    ═══════════════════════════════════════════════════════════════════════ -/

-- 合成见证用 Move（趋势上涨，单中枢；endIndex=3 对应 sampleType1 见证）。
-- 避免与 CenterStates.sampleCenter（CenterWithOuter 类型）名称冲突，中枢值内联。
-- ★endPrice=5 < zd=10 ⟹ 末端价在中枢之下 ⟹ brokeCenterOf 推导出 true（L0 价格几何，非 oracle）。
def bspSampleMove : Move :=
  { kind := MoveKind.trendUp, startIndex := 0, endIndex := 3
    startPrice := 22         -- 起点价：中枢 [10,20] 之上（趋势上涨起点在中枢上沿外）
    endPrice := 5            -- 末端价：5 < zd=10 ⟹ 之下 ⟹ 破中枢（L0 推导，与 sampleType1 第一类一致）
    centers := [sampleType1.center] }

-- 合成见证用力度判据：复用 BspClassification.sampleType1 的背驰/回抽字段（L2 开口部分）。
-- ★brokeCenter/leftCenter **不在此**（已从 oracle 降为 L0 推导，632号路1）；afterTypeOne 也不在
--   （已降为前序折叠产物）。本结构只携带无法从 Move 几何推导的力度数据（divPair still-MISSING-C）。
def bspSampleForce1 : MoveForceJudgment :=
  { side := sampleType1.side
    divPair := sampleType1.divPair
    retracePrice := sampleType1.retracePrice
    firstRetrace := sampleType1.firstRetrace }

/-- ★L0 反退化见证：bspSampleMove 末端价 5 在中枢 [10,20] 之下 ⟹ brokeCenterOf 推导出 true
    （价格几何 L0 推导，非 oracle 注入——这是 632号路1 消桩的核心兑现）。 -/
theorem witness_L0_brokeCenter :
    brokeCenterOf bspSampleMove sampleType1.center = true := by
  unfold brokeCenterOf bspSampleMove sampleType1
  decide

/-- ★L0 反退化见证：moveToCandidate 组装的端点（brokeCenter L0 推导 + afterT1=false）识别为 type1。
    afterT1=false 不阻碍第一类（第一类只需 brokeCenter ∧ 背驰，不依赖 afterTypeOne）。 -/
theorem witness_L0_type1_from_move :
    classifyEndpoint (moveToCandidate bspSampleMove sampleType1.center bspSampleForce1 false) =
      some { kind := BspKind.type1, side := Side.long, index := 3, price := 5 } := by
  have ht1 : IsType1 (moveToCandidate bspSampleMove sampleType1.center bspSampleForce1 false).endpoint := by
    unfold IsType1 moveToCandidate bspSampleForce1
    refine ⟨?_, ?_⟩
    · exact witness_L0_brokeCenter
    · unfold IsDivergence; decide
  unfold classifyEndpoint
  rw [if_pos ht1]
  unfold moveToCandidate bspSampleForce1 sampleType1
  rfl

/-- ★管线见证：bspOfMoves 对 [bspSampleMove] + [center] + [force] 产出恰一个 type1 买卖点
    （brokeCenter 从 Move 价格端点 L0 推导，divPair 从 force 注入，afterTypeOne 前序折叠初始 false）。 -/
theorem witness_bspOfMoves_single :
    (bspOfMoves [bspSampleMove] [sampleType1.center] [bspSampleForce1]).length = 1 := by
  simp only [bspOfMoves, bspOfMovesAux, witness_L0_type1_from_move]
  rfl

/-- ★管线见证：bspOfMoves 空走势列 ⟹ 空（边界，真遍历，无桩）。 -/
theorem witness_bspOfMoves_empty : bspOfMoves [] [] [] = [] := by
  unfold bspOfMoves bspOfMovesAux; rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 10. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已消解（task #116 + 632号路1 canonical 修复，acceptance #2）：
    - **遍历识别终止性**：bspOf/bspOfMovesAux 结构递归（消费列头），结构终止（Lean 直接接受）。
    - **第二类本级别判据签名**：secondTypeViaSublevel 是返回 Bool 的全函数（`secondType_is_bool`
      二歧见证）——把买卖点定律一装配为「本级别 IsType2 ∧ 次级别第一类占位」单步判据。
      ★诚实：这**不是**次级别递归终止性证明——`subLevelHasType1` 丢弃级别索引 n、两分支同体、
      不自调，无下降递归结构，故无终止义务可证（次级别真下钻 still-MISSING-D′，见下）。
    - **输出唯一性**：bspOf_total_unique + bspOf_single_valued + secondType_total_unique +
      bspOfMoves_total_unique。
    - **2/3 共存的操作侧消解**：classifyEndpoint 用确定性优先级（一类>三类>二类）使输出唯一——
      判据层互斥三分失败（no_exclusive_trichotomy）不阻碍构造层确定性输出。
    - **★632号路1 canonical 修复（codex 异质裁决，brokeCenter/leftCenter L0 消桩）**：
      给 canonical `Move` 加 `startPrice/endPrice`（ChanlunElements.lean，透传自 `Segment` 端点）。
      · `brokeCenterOf`/`leftCenterOf`（§7.1）：从 `Move.endPrice` vs 中枢 [zd,zg] 用 608号位置三态
        `IsBelow/IsAbove` **纯 L0 几何推导**——`brokeCenter`/`leftCenter` **不再 oracle 注入**。
        `brokeCenterOf_iff` 坐实推导锚到 608号位置三态（之外 = IsBelow ∨ IsAbove）。**L0**。
      · `afterTypeOne`（§7.3）：在 `bspOfMovesAux` 遍历 `List Move` 时由**前序状态折叠**产生
        （前序出现第一类 ⟹ afterT1 翻转），**不是**单 `Move` 原子字段——列表级时序量。**L0**（纯折叠）。
      · `divPair`（背驰力度）：保持**明确 L2 开口**（`MoveForceJudgment.divPair`，still-MISSING-C：
        MACD/EMA 引擎无 Origin 计算）——不填、诚实标注，不与 L0 几何捆成大 oracle。
      · 计算见证：`witness_L0_brokeCenter`（末端价 L0 推导破中枢=true）+ `witness_L0_type1_from_move`
        （Move 价格端点 → L0 brokeCenter → type1）+ `witness_bspOfMoves_single`（管线产出 length=1）+
        `witness_bspOfMoves_empty`（空列边界）——brokeCenter 真 L0 推导（非 oracle），divPair 真 L2 注入。
    - **L 等级诚实标注**：§7.1 brokeCenter/leftCenter **L0**（价格几何，608号位置三态结构推导，
      信息增量真——从 Move 价格端点真推导，非 oracle 给定）；§7.3 afterTypeOne **L0**（列表级折叠，
      纯结构）；divPair **L2 开口**（still-MISSING-C，背驰力度需 MACD 引擎，诚实不填）。

  ★still-MISSING-D′ + divPair L2 开口（divPair 力度仍需 Origin 计算引擎）：
    - `subLevelHasType1` 丢弃级别索引 n、两分支同体、不自调——**不是** well-founded 级别下降递归。
      把单步占位判据冒充为完整下钻递归 = 声明膨胀（禁止）。
    - **divPair（背驰力度）= 真 L2 缺口（still-MISSING-C）**：632号路1 消解了价格几何 oracle
      （brokeCenter/leftCenter）与时序 oracle（afterTypeOne），但 `divPair` 的 `Force`/MACD 面积
      **无法**从 `Move` 价格端点几何推导（需 EMA/DIF/DEA + 面积积分，Divergence still-MISSING-C）。
      `MoveForceJudgment.divPair` 是诚实的 L2 开口——本文件**不冒充** divPair 已可 L0 推导。
      第一类判据 `IsType1 = brokeCenter ∧ IsDivergence` 中：brokeCenter 现 L0（632号路1），
      IsDivergence(divPair) 仍 L2——第一类完整识别仍依赖 L2 力度引擎接入。

  ★边界条件（结论翻转）：
    - `brokeCenterOf`/`leftCenterOf` L0 推导依赖 `Move.endPrice`（632号路1 加的字段）。若
      `movesOf` 实例**不**从 `Segment` 端点透传真价格（违反 ElementPipeline 价格透传契约），
      则 `Move.endPrice` 是垃圾值，brokeCenter 推导虽 type-check 但语义失效——透传契约是 L0 推导
      有效的前提（接口义务，ChanlunElements.lean 文档）。
    - `witness_bspOfMoves_single` 成立依赖 `bspSampleMove.endPrice=5 < zd=10`（末端价破中枢）+
      `bspSampleForce1` 背驰（IsDivergence）。若末端价改到中枢内（之中），brokeCenterOf 推导 false ⟹
      非第一类 ⟹ 输出空——见证翻转，须新见证。brokeCenter 翻转现由**价格几何**驱动（非 oracle 改值）。
    - `afterTypeOne` 前序折叠：第一类不依赖 afterTypeOne（`IsType1` 无 afterTypeOne 项），故前序状态
      不影响第一类识别；但第二类 `IsType2 = afterTypeOne ∧ ¬brokeCenter` 依赖前序折叠——若走势序列
      中无前序第一类，则后续走势 afterT1 恒 false ⟹ 无第二类。这是买卖点定律一的列表级时序兑现。
    - `divPair` 留 L2 开口：若 Divergence still-MISSING-C 被填（MACD 引擎接入），则 `MoveForceJudgment`
      可从真实力度计算产生（L2→L3 视数据覆盖），本文件接口不需改动（force 注入点已留）。
    - `classifyEndpoint` 优先级（一类>三类>二类）是确定性选择。若某口径要求"2/3 共存时同时输出
      两个 Bsp"（非互斥路由），须改 `bspOfMoves` 签名为 `List (List Bsp)` per move——
      当前裁定优先级单选（与策略层"6 种买卖点信号"路由一致）。

  ★下游推论（632号路1 对 acceptance #2 MET）：
    - **价格几何 L0 真推导消冒充**：brokeCenter/leftCenter 从 `Move` 价格端点真 L0 推导（608号
      位置三态），**不再**是"未验证 oracle 冒充完整分类"——acceptance #2 的价格几何分量真兑现。
    - **divPair 诚实 L2 开口**：背驰力度未冒充已填，明确标 still-MISSING-C（MACD 引擎）——
      第一类完整识别 = L0 破中枢 ∧ L2 背驰，分层诚实（不把 L2 力度说成 L0）。
    - `movesOf` 实例上游须**从 Segment 端点透传价格**（ElementPipeline 价格透传契约）——这是
      `Move` 携价格端点后的接口义务（不是丢弃，632号路1）。`MoveForceJudgment` 仅需上游提供力度
      （背驰段对），不再需提供价格几何/时序（已内化为 L0 推导/折叠）。
    - 策略组件工位（#114）的第二类应对依赖 `bspOfMovesAux` 前序折叠产生的 afterTypeOne（列表级
      时序），+ `secondType_needs_sublevel_type1` 前件结构（次级别下钻占位，still-MISSING-D′）。

  ★影响声明：
    - **ChanlunElements.lean**（canonical，632号路1）：`Move` 加 `startPrice/endPrice` 字段；
      `ElementPipeline` 加价格透传契约文档（`movesOf` 须从 Segment 端点透传）。
    - **BspConstruction.lean §7（632号路1 重构）**：
      · 新增 `brokeCenterOf`/`leftCenterOf`（608号位置三态 L0 价格几何推导）+ `brokeCenterOf_iff`。
      · 新增 `MoveForceJudgment`（**只**保留 divPair/retracePrice/firstRetrace L2 力度开口字段）+
        `moveToCandidate`（brokeCenter/leftCenter L0 推导 + afterTypeOne 折叠传入 + force L2 注入）。
      · 新增 `bspOfMovesAux`（列表级前序折叠 afterTypeOne）+ `bspOfMoves`（顶层 afterT1 初始 false）+
        `bspOfMoves_total_unique`。
      · **删除** 旧 `MoveJudgment`（把价格几何/时序当 oracle Bool 字段）+ `moveJudgmentToCandidate`
        + `bspSampleJudgment1` + 旧 `bspOfMoves`（zip+filterMap，无前序折叠）。
    - **BspConstruction.lean §9（见证重构）**：`bspSampleMove` 加价格端点（endPrice=5 破中枢）；
      新增 `bspSampleForce1`（L2 力度判据）+ `witness_L0_brokeCenter`/`witness_L0_type1_from_move`/
      `witness_bspOfMoves_single`/`witness_bspOfMoves_empty`。删除旧 `witness_L1_*_from_judgment`。
    - 不改 BspClassification.lean / CenterStates.lean / Divergence.lean（只读，用其判据/位置三态/背驰）。

  ★谱系引用：消解 BspClassification.lean § 8 still-MISSING-D（bspOf 全自动遍历终止 + 唯一）；
    **632号路1 兑现**（codex 异质裁决，编排者委托）——629号开口③精确化的 canonical 契约缺陷
    （movesOf 丢 Segment 价格）通过给 `Move` 加价格端点修复：brokeCenter/leftCenter 从 oracle 降为
    608号位置三态 L0 推导，afterTypeOne 从单走势字段降为列表级前序折叠，divPair 诚实留 L2 开口
    （still-MISSING-C）。互斥三分失败的操作侧优先级消解重锚 Strict.BSP refined_classifies
    （精化触发签名真单射，谱系 598→603→615）。
    认识论等级：§1-7.1/7.3 全 L0（价格几何推导 + 列表折叠）；divPair L2 开口（背驰力度 still-MISSING-C）。
-/

end NewChanlun.Origin
