/-
Origin/BspConstruction.lean — bsp 全自动构造（遍历识别三类 + 第二类本级别判据签名）+ 终止性 + 唯一性（task #116）

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
全部 **L0**（纯定义 / 结构递归终止 / Bool 全函数二歧 / 纯函数唯一性，不依赖数据）。
`lake env lean Origin/BspConstruction.lean` 通过 = bspOf 作为全函数良定义（终止）+ 输出唯一
（确定性）+ 第二类本级别判据是返回 Bool 的全函数在定义层成立，**不是**任何"识别的买卖点真对应
缠论权威标注"的实证断言（L2+），**也不是**次级别真下钻终止性的证明（该下钻 still-MISSING-D′）。

诚实标注（gatekeeper，no-patch-mentality）：
★ TerminationAndDeterminismOnly + SubLevelIsSingleStepPlaceholder ——
  本文件证 bspOf 遍历的**结构终止 + 唯一性**。第二类「次级别第一类」用 level-indexed **单步占位
  判据**（`subLevelHasType1`：丢弃 n、两分支同体、不自调）——**不是** well-founded 级别下降递归，
  **不**实装"从本级别 ParseStruct 下钻到次级别 ParseStruct 重新跑 segmentsOf/centersOf"
  （那需 RecursiveLevelSystem 全实例化，still-MISSING-D′）。
  把单步占位判据冒充为完整次级别下钻递归 = 声明膨胀（禁止）。

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
    § 7. ElementPipeline 桥接：Move → BspCandidate oracle 接口（L1 真编码）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **走势判据注入（oracle 接口，外部数据契约）** —— 从元素层 `Move` **无法单独推导**的买卖点
  判据数据：`Move` 结构只携带 `kind/startIndex/endIndex/centers`，**不携带价格端点**
  （无 startPrice/endPrice），也不携带背驰段对。`bspOfMoves` 对这些**外部注入**的判据做遍历识别。

  ★诚实声明（no-patch-mentality + formalization-validity-domain + 632号矛盾上浮）：
  `MoveJudgment` 是 oracle **接口定义**——它声明"产买卖点判据需要哪些字段"，**不声明**"这些字段
  能从 `Move` 推导"。三个 Bool 字段的可推导性分层（632号逐字段分析）：

  · `brokeCenter`/`leftCenter`（价格几何）= **L0 结构**，但元素层 `Move` 缺价格端点：判定数据是
    «走势末端价 vs 中枢 [zd,zg]»（CenterStates.IsBelow/IsAbove，608号），价格在 `Segment.endPrice`
    层已算出（管线 strokesOf→segmentsOf）；是 `movesOf : List Segment → List Move` 构造 `Move` 时
    **丢弃**了 Segment 价格端点。对照 `SubLevelDescent.SubBrokeBelow`（对携带 `interval` 的 `RMove`
    L0 判破中枢）——破中枢本质 L0 几何，缺口在元素层 `Move` 无价格字段（canonical 契约缺陷）。
  · `afterTypeOne`（时序）需走势序列级前序分类上下文 + 递归依赖 `brokeCenter`，单 `Move` 不可推导。
  · `divPair`（背驰力度）= **真 L2 缺口**：`Force`/MACD 面积无 Origin 计算引擎（Divergence
    still-MISSING-C，需 EMA/DIF/DEA + 面积积分）。

  ★632号矛盾上浮：消除"从 Move 单独 L0 推导判据"的开口需 canonical 契约裁定（给 `Move` 加价格
  字段 / 改 `bspOf` 签名接 Segment / 接受 oracle 永久外部化）——触及 P1 owner `ChanlunElements`，
  待编排者 /ritual。本文件**不冒充**"Move 单独足以产 MoveJudgment"（删除原 dummyJudgmentFromMove
  零判据桩，见 §8 诚实化），只保留 oracle **接口契约**（judgments 由携带价格几何的上游注入）。
-/
structure MoveJudgment where
  side : Side
  center : Center
  divPair : DivergencePair
  brokeCenter : Bool
  afterTypeOne : Bool
  leftCenter : Bool
  retracePrice : Tick
  firstRetrace : Bool
  endIndex : Index
  endPrice : Tick
deriving Repr

/--
  **走势判据 → 买卖点候选端点（桥接函数，L1）** —— 将 `MoveJudgment` 组装为 `BspCandidate`。
  `Move` 的 `endIndex` 对应候选买卖点的下标（走势末端是买卖点候选位置）。
-/
def moveJudgmentToCandidate (j : MoveJudgment) : BspCandidate :=
  { endpoint :=
      { side := j.side
        center := j.center
        divPair := j.divPair
        brokeCenter := j.brokeCenter
        afterTypeOne := j.afterTypeOne
        leftCenter := j.leftCenter
        retracePrice := j.retracePrice
        firstRetrace := j.firstRetrace }
    index := j.endIndex
    price := j.endPrice }

/--
  **★bspOfMoves（L1 真编码）** —— `List Move → List MoveJudgment → List Bsp`。
  对每个 `(Move, MoveJudgment)` 对，提取候选端点并调用 `classifyEndpoint`，收集非 none 结果。
  oracle 注入（`judgments`）是 L1 接口约束：判据数据由外部提供，本函数只做遍历识别。

  ★终止性：结构递归消费 `pairs` 列头（`_ :: rest` → 递归 `rest`），结构终止。
  ★唯一性：纯函数 ⟹ 输出唯一（`bspOfMoves_total_unique`）。
  ★有效域诚实标注：L1（管线编码，judgments 是合成注入时不产生信息增量；
  L2+ 需真实判据数据，即上游真实 centersOf + 背驰计算）。
-/
def bspOfMoves (moves : List Move) (judgments : List MoveJudgment) : List Bsp :=
  let pairs := moves.zip judgments
  pairs.filterMap (fun (_, j) => classifyEndpoint (moveJudgmentToCandidate j))

/-- ★bspOfMoves 输出唯一性（L0，确定性）。 -/
theorem bspOfMoves_total_unique :
    TotalUnique (fun (pair : List Move × List MoveJudgment) (out : List Bsp) =>
      bspOfMoves pair.1 pair.2 = out) :=
  total_unique_of_fun (fun p : List Move × List MoveJudgment => bspOfMoves p.1 p.2)

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. 诚实开口（632号）：bspOf 从元素层 Move 单独不可 L0 实装——无桩冒充绑定

    ★629号识别"未验证 oracle 冒充完整分类"开口；632号精确化：元素层 `Move`（无价格端点）
    单独不足以产 `MoveJudgment`（brokeCenter/leftCenter 需走势末端价 vs 中枢几何，价格在
    `Segment` 层已算但被 `movesOf` 丢弃；divPair 需 MACD 力度 = L2 still-MISSING-C）。

    原 `dummyJudgmentFromMove`（对每个 Move 产零判据：全 false / 零价）+ `bspOfViaPipeline`
    （map 零判据 → 恒产空 Bsp 列）已**删除**——它们冒充"绑定 ElementPipeline.bspOf 字段、消解
    平凡桩"，实际零判据恒产空列 = 退化冒充（no-patch-mentality / 090 声明膨胀，acceptance#2 要
    "无未验证 oracle 冒充完整分类"）。`bspOf : List Move → List Bsp` 的具体绑定需 canonical 契约
    裁定（632号上浮三路：给 Move 加价格字段 / 改 bspOf 签名接 Segment / 接受 oracle 永久外部化），
    触及 P1 owner ChanlunElements，待编排者 /ritual——本文件**不冒充该绑定已成**。

    保留的诚实部分：`MoveJudgment`（oracle 接口契约）+ `bspOfMoves`（真遍历，judgments 由携带
    价格几何的上游外部注入，无冒充）+ §9 用真判据 `bspSampleJudgment1`（复用 sampleType1，
    携带真破中枢+真背驰）的 L1 管线见证——judgments 真实（非零判据），是诚实的 L1 编码见证。
    ═══════════════════════════════════════════════════════════════════════ -/

/-! ═══════════════════════════════════════════════════════════════════════
    § 9. L1 合成计算见证（具体 Move + 具体真判据 ⟹ 具体 Bsp 输出）
    ═══════════════════════════════════════════════════════════════════════ -/

-- L1 合成见证用 Move（趋势上涨，单中枢；endIndex=3 对应 sampleType1 见证）。
-- 避免与 CenterStates.sampleCenter（CenterWithOuter 类型）名称冲突，中枢值内联。
def bspSampleMove : Move :=
  { kind := MoveKind.trendUp, startIndex := 0, endIndex := 3
    centers := [sampleType1.center] }

-- L1 合成见证用判据：复用 BspClassification.sampleType1（第一类买点，已有 witness_type1）。
def bspSampleJudgment1 : MoveJudgment :=
  { side := sampleType1.side
    center := sampleType1.center
    divPair := sampleType1.divPair
    brokeCenter := sampleType1.brokeCenter
    afterTypeOne := sampleType1.afterTypeOne
    leftCenter := sampleType1.leftCenter
    retracePrice := sampleType1.retracePrice
    firstRetrace := sampleType1.firstRetrace
    endIndex := 3
    endPrice := 5 }

/-- ★L1 反退化见证：bspSampleJudgment1 端点识别为 type1。 -/
theorem witness_L1_type1_from_judgment :
    classifyEndpoint (moveJudgmentToCandidate bspSampleJudgment1) =
      some { kind := BspKind.type1, side := Side.long, index := 3, price := 5 } := by
  -- moveJudgmentToCandidate bspSampleJudgment1 展开后 endpoint = sampleType1（字段投影，不产新 proof）
  -- witness_type1 已证 IsType1 sampleType1，用于 if_pos
  have heq : (moveJudgmentToCandidate bspSampleJudgment1).endpoint = sampleType1 := by
    unfold moveJudgmentToCandidate bspSampleJudgment1 sampleType1; rfl
  have ht1 : IsType1 (moveJudgmentToCandidate bspSampleJudgment1).endpoint := heq ▸ witness_type1
  unfold classifyEndpoint
  rw [if_pos ht1]
  unfold moveJudgmentToCandidate bspSampleJudgment1 sampleType1
  rfl

/-- ★L1 管线见证：bspOfMoves 对 [bspSampleMove] + [bspSampleJudgment1] 产出恰一个 type1 买卖点。 -/
theorem witness_L1_bspOfMoves_single :
    (bspOfMoves [bspSampleMove] [bspSampleJudgment1]).length = 1 := by
  unfold bspOfMoves
  simp only [List.zip_cons_cons, List.zip_nil_right, List.filterMap_cons, List.filterMap_nil]
  rw [witness_L1_type1_from_judgment]
  rfl

/-- ★L1 管线见证：bspOfMoves 空走势列 + 空判据列 ⟹ 空（边界，真遍历，无桩）。 -/
theorem witness_L1_bspOfMoves_empty : bspOfMoves [] [] = [] := by
  unfold bspOfMoves; rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 10. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已消解（task #116 + task #17 P2 构造层升级 L0→L1）：
    - **遍历识别终止性**：bspOf 结构递归（消费列头），结构终止（Lean 直接接受）。
    - **第二类本级别判据签名**：secondTypeViaSublevel 是返回 Bool 的全函数（`secondType_is_bool`
      二歧见证）——把买卖点定律一装配为「本级别 IsType2 ∧ 次级别第一类占位」单步判据。
      ★诚实：这**不是**次级别递归终止性证明——`subLevelHasType1` 丢弃级别索引 n、两分支同体、
      不自调，无下降递归结构，故无终止义务可证（次级别真下钻 still-MISSING-D′，见下）。
    - **输出唯一性**：bspOf_total_unique + bspOf_single_valued + secondType_total_unique。
    - **2/3 共存的操作侧消解**：classifyEndpoint 用确定性优先级（一类>三类>二类）使输出唯一——
      判据层互斥三分失败（no_exclusive_trichotomy）不阻碍构造层确定性输出。
    - **ElementPipeline 桥接（oracle 接口侧，无桩冒充）**：
      · `MoveJudgment`：oracle 接口契约——声明产买卖点判据需要的字段，**不声明**这些字段能从
        元素层 `Move`（无价格端点）推导（632号）。
      · `bspOfMoves`：`List Move → List MoveJudgment → List Bsp` 真遍历（结构终止）——judgments 由
        携带价格几何的上游外部注入，本函数只做遍历识别，无冒充。
      · L1 计算见证：`witness_L1_type1_from_judgment`（**真判据** sampleType1 → type1）+
        `witness_L1_bspOfMoves_single`（真判据管线产出 length=1）+ `witness_L1_bspOfMoves_empty`
        （空列边界）——judgments 真实（携真破中枢+真背驰，非零判据），是诚实 L1 编码见证。
    - **L 等级诚实标注**：§7 及以前全部 L0（纯定义/结构递归）；§9 L1（用真判据 sampleType1 的
      管线编码见证）。L1 信息增量为零——判据 `brokeCenter=true` 由 sampleType1 给定，验证只确认
      管线没有 bug，**不**确认"真实 Move 真有破中枢/背驰"（需 L2+，且元素层 Move 缺价格，632号）。

  ★still-MISSING-D′ + 632号诚实开口（bspOf 从元素层 Move 单独不可 L0 实装）：
    - `subLevelHasType1` 丢弃级别索引 n、两分支同体、不自调——**不是** well-founded 级别下降递归。
      把单步占位判据冒充为完整下钻递归 = 声明膨胀（禁止）。
    - **632号**：原 `dummyJudgmentFromMove`（零判据桩）+ `bspOfViaPipeline`（恒产空列）已**删除**——
      它们冒充"绑定 ElementPipeline.bspOf、消解平凡桩"，实际零判据恒产空 = 退化冒充（acceptance#2
      要"无未验证 oracle 冒充完整分类"）。`MoveJudgment` 的三 Bool 字段不可从元素层 `Move` 纯 L0
      结构推导：brokeCenter/leftCenter（价格几何，L0 结构但 `Move` 缺价格端点——`movesOf` 丢弃了
      `Segment` 的 startPrice/endPrice）、afterTypeOne（时序，需走势序列上下文）、divPair（背驰力度，
      L2 still-MISSING-C）。`bspOf` 具体绑定需 canonical 契约裁定（632号上浮三路），待编排者 /ritual。

  ★边界条件（结论翻转）：
    - `bspOfMoves` 终止性是结构递归内蕴（消费 `pairs` 列头），对任何 `classifyEndpoint` 都成立——
      接入完整判据不影响遍历终止性。
    - `witness_L1_bspOfMoves_single` 成立依赖 `bspSampleJudgment1`（真判据：brokeCenter=true 且
      IsDivergence divPair）。若判据改为非破中枢/非背驰，则输出空——见证翻转，须新见证。
    - 632号修复路裁定翻转下游：若编排者裁定"给 `Move` 加价格字段"（路1），则 brokeCenter/leftCenter
      可用 608号位置三态 L0 结构推导消桩，本文件可新增真结构推导函数（替代已删的零判据桩），
      仅 divPair 力度留 L2 开口；若裁定"接受 oracle 永久外部化"（路3），则 oracle 接口即终态。
    - `classifyEndpoint` 优先级（一类>三类>二类）是确定性选择。若某口径要求"2/3 共存时同时输出
      两个 Bsp"（非互斥路由），须改 `bspOfMoves` 签名为 `List (List Bsp)` per move——
      当前裁定优先级单选（与策略层"6 种买卖点信号"路由一致）。

  ★下游推论：
    - **632号**：`bspOf : List Move → List Bsp` 从元素层 `Move` 单独**不可 L0 实装**（Move 缺价格
      几何 + 力度），不能宣称"消解 bspOf 平凡桩"——具体绑定待 canonical 契约裁定。策略组件工位
      （#114）的第二类应对可依赖 `secondType_needs_sublevel_type1` 前件结构（占位判据，待真下钻接入）。
    - `MoveJudgment` 接口契约 ⟹ 上游须实现「携带价格几何的载体（Segment/RMove）+ 背驰 → MoveJudgment」
      才能注入（不是 `Move → MoveJudgment`，因元素层 Move 信息不足，632号）。
    - L1 见证告诉下游：管线遍历层无 bug（真判据端点正确路由到 type1），上游填充真实判据后管线不需
      改动——但"上游填充"需先解决 632号 canonical 契约（Move 取价格的途径），非纯上游责任。

  ★影响声明：
    - §7：`MoveJudgment`（oracle 接口契约，文档诚实化标 632号）+ `moveJudgmentToCandidate` + `bspOfMoves`。
    - §8（632号诚实化）：**删除** `dummyJudgmentFromMove`（零判据桩）+ `bspOfViaPipeline`
      + `bspOfViaPipeline_total` + `bspOfViaPipeline_total_unique` + `witness_L1_pipeline_empty`
      + `witness_L1_pipeline_zero_judgment`（它们冒充"绑定 bspOf 字段"，零判据恒产空 = 退化冒充）。
      新增 `witness_L1_bspOfMoves_empty`（空列边界，真遍历无桩）。保留 `bspOfMoves`（真遍历）+
      §9 真判据 L1 见证。
    - 不改 canonical 类型（`BspEndpoint`/`BspCandidate`/`Bsp`/`Move`/`ElementPipeline` 结构定义不变）。
    - 不改 BspClassification.lean / ChanlunElements.lean（只读，P1 owner 文件）。

  ★谱系引用：消解 BspClassification.lean § 8 still-MISSING-D（bspOf 全自动遍历终止 + 唯一）；
    **632号**精确化 629号开口③——MoveJudgment 消桩在元素层 Move 上不可纯 L0（根因 movesOf 丢
    Segment 价格），区分 L0 结构缺口（价格几何）与 L2 数据缺口（背驰力度，still-MISSING-C），
    canonical 契约修复待编排者 /ritual。互斥三分失败的操作侧优先级消解重锚 Strict.BSP
    refined_classifies（精化触发签名真单射，谱系 598→603→615）。
    认识论等级：§1-7 全 L0；§9 L1（真判据管线编码见证，信息增量为零但遍历编码真实）。
-/

end NewChanlun.Origin
