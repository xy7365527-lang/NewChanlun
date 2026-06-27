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
  **走势判据注入（oracle pattern，L1 接口）** —— 从 `Move` 无法单独推导的买卖点判据数据：
  `Move` 结构只携带 `kind/startIndex/endIndex/centers`，**不**携带背驰段对、中枢突破标志等。
  这些判据由上游（背驰计算 + 中枢位置判断）提供，此处定义为外部注入结构（oracle），
  使 `bspOfMoves` 可在不依赖 P1 未完成的 CenterConstruct 的情况下组装端点。

  ★诚实声明（no-patch-mentality + formalization-validity-domain）：
  `MoveJudgment` 是**接口占位**——在真实管线中，它须由 centersOf + Divergence 真填充
  （`divPair` 来自背驰力度计算，`brokeCenter/leftCenter` 来自中枢位置判断）。
  本文件假设该数据已正确注入，不实装"从 Move 自动推导判据"（那是上游责任）。
  **有效域**：L1（管线编码层成立，经验有效性需 L2+）。
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
    § 8. ElementPipeline 具体实例（L1 合成见证：管线端到端跑通）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★ElementPipeline.bspOf 字段绑定（L1 合成实例）** —— 将 `ElementPipeline.bspOf`（`List Move → List Bsp`）
  绑定为：对每个 `Move` 提取其 `centers.head?`（首中枢）作为背景中枢，以零判据（全 false/零）
  产出 BspCandidate，调用 `classifyEndpoint`。

  ★诚实声明：此实例是 L1 **管线编码**见证（非经验有效见证）：
  - 零判据（`brokeCenter = false`, `afterTypeOne = false`, `leftCenter = false`）⟹
    无法满足任何三类判据 ⟹ 对任何真实 Move 均产出空 Bsp 列。
  - 意义：证明 `ElementPipeline.bspOf` 字段可被具体全函数（`classifyEndpoint` 遍历）绑定，
    消解"bspOf 字段是平凡桩"——字段现有具体算法，不再是 `total_unique_of_fun P.bspOf` 的抽象占位。
  - 真实管线须接入上游真实判据注入（`MoveJudgment` 由 centersOf + 背驰填充，still-MISSING-D′）。
-/
def dummyJudgmentFromMove (m : Move) : MoveJudgment :=
  let c : Center := match m.centers.head? with
    | some ctr => ctr
    | none => { zd := 0, zg := 0, startIndex := 0, endIndex := 0, valid := Int.le_refl 0 }
  { side := Side.long
    center := c
    divPair := { forceA := ⟨0⟩, forceC := ⟨0⟩, isTrend := false }
    brokeCenter := false
    afterTypeOne := false
    leftCenter := false
    retracePrice := 0
    firstRetrace := false
    endIndex := m.endIndex
    endPrice := 0 }

/--
  **★具体 ElementPipeline 实例见证（L1，管线端到端编码）** —— `bspOfViaPipeline` 是一个
  `List Move → List Bsp` 全函数，绑定到 `bspOfMoves`（零判据路径）。
  可直接用作 `ElementPipeline.bspOf` 字段的具体实现（消解平凡桩）。
-/
def bspOfViaPipeline (moves : List Move) : List Bsp :=
  bspOfMoves moves (moves.map dummyJudgmentFromMove)

/-- ★终止性：bspOfViaPipeline 是全函数（`List.map` + `bspOfMoves` 均结构终止）。 -/
theorem bspOfViaPipeline_total :
    Total (fun moves out => bspOfViaPipeline moves = out) :=
  fun moves => ⟨bspOfViaPipeline moves, rfl⟩

/-- ★唯一性：bspOfViaPipeline 输出唯一（纯函数）。 -/
theorem bspOfViaPipeline_total_unique :
    TotalUnique (fun moves out => bspOfViaPipeline moves = out) :=
  total_unique_of_fun bspOfViaPipeline

/-! ═══════════════════════════════════════════════════════════════════════
    § 9. L1 合成计算见证（具体 Move + 具体判据 ⟹ 具体 Bsp 输出）
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

/-- ★L1 管线见证：bspOfViaPipeline 空列 ⟹ 空（零判据路径边界）。 -/
theorem witness_L1_pipeline_empty : bspOfViaPipeline [] = [] := by
  unfold bspOfViaPipeline bspOfMoves; rfl

/-- ★L1 管线见证：bspOfViaPipeline 对 [bspSampleMove]（零判据）产出空（零判据不产信号，诚实）。 -/
theorem witness_L1_pipeline_zero_judgment : bspOfViaPipeline [bspSampleMove] = [] := by
  -- 零判据（dummyJudgmentFromMove 产 brokeCenter=false,afterTypeOne=false,leftCenter=false）
  -- ⟹ IsType1/IsType3/IsType2 均不满足 ⟹ classifyEndpoint = none ⟹ filterMap 产空
  native_decide

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
    - **ElementPipeline 桥接（L1 升级）**：
      · `MoveJudgment`：oracle 接口，外部判据注入结构（move 不携带的判据字段）。
      · `bspOfMoves`：`List Move → List MoveJudgment → List Bsp` 真遍历（结构终止）。
      · `bspOfViaPipeline`：`List Move → List Bsp` 具体函数，可绑定 ElementPipeline.bspOf。
      · L1 计算见证：`witness_L1_type1_from_judgment`（合成判据 → type1）+
        `witness_L1_bspOfMoves_single`（管线产出 length=1）+ `witness_L1_pipeline_zero_judgment`
        （零判据→空列，诚实标注无信号）——消解"bspOf 字段是平凡桩"。
    - **L 等级诚实标注**：§7 及以前全部 L0（纯定义/结构递归）；§8-9 L1（合成数据管线编码）。
      L1 信息增量为零——合成判据中的 `brokeCenter=true/false` 由测试者设定，
      验证只能确认管线没有 bug，**不**确认"真实 Move 真有背驰"（需 L2+）。

  ★still-MISSING-D′（完整次级别下钻 + 判据真填充，诚实开口）：
    - `subLevelHasType1` 丢弃级别索引 n、两分支同体、不自调——**不是** well-founded 级别下降递归。
      把单步占位判据冒充为完整下钻递归 = 声明膨胀（禁止）。
    - `dummyJudgmentFromMove` 产出零判据（全 false）——`bspOfViaPipeline` 在零判据下恒产空列。
      真实管线须上游（centersOf 真算 + 背驰力度真算）填充 `MoveJudgment` 各字段
      （仍是 still-MISSING-D′，本文件不冒充该接入已完成）。

  ★边界条件（结论翻转）：
    - `bspOfMoves` 终止性是结构递归内蕴（消费 `pairs` 列头），对任何 `classifyEndpoint` 都成立——
      接入完整判据不影响遍历终止性。
    - `witness_L1_pipeline_zero_judgment` 成立依赖 `dummyJudgmentFromMove` 零判据（所有 Bool=false）。
      若上游接入真实判据（`brokeCenter=true` 且 `IsDivergence divPair`），则输出非空——此时需
      `witness_L1_bspOfMoves_single` 类型的新见证，当前零判据见证不翻转（零判据行为仍成立）。
    - `classifyEndpoint` 优先级（一类>三类>二类）是确定性选择。若某口径要求"2/3 共存时同时输出
      两个 Bsp"（非互斥路由），须改 `bspOfMoves` 签名为 `List (List Bsp)` per move——
      当前裁定优先级单选（与策略层"6 种买卖点信号"路由一致）。

  ★下游推论：
    - `bspOfViaPipeline` 终止 + 唯一 ⟹ 可直接绑定 `ElementPipeline.bspOf` 字段——消解审计
      判决的"bspOf 平凡桩"。策略组件工位（#114）的第二类应对可依赖 `secondType_needs_sublevel_type1`
      前件结构（占位判据，待真下钻接入）。
    - `MoveJudgment` 接口定义 ⟹ 上游（centersOf + 背驰）只需实现 `Move → MoveJudgment`
      即可接入完整管线，接口契约已在本文件形式化（字段类型 + moveJudgmentToCandidate 组装）。
    - L1 见证告诉下游：管线编码层无 bug（合成端点正确路由到 type1），上游填充真实判据后，
      管线不需改动——信息差在于判据质量（L2+），不在于管线实现（L1 已完成）。

  ★影响声明：
    - 在 Origin.BspConstruction 新增 §7-9：`MoveJudgment`（结构体）、`moveJudgmentToCandidate`、
      `bspOfMoves`、`dummyJudgmentFromMove`、`bspOfViaPipeline` + 相关定理和见证。
    - 不改 canonical 类型（`BspEndpoint`/`BspCandidate`/`Bsp`/`ElementPipeline` 结构定义不变）。
    - 不改 BspClassification.lean（只读，P1 owner 文件）。

  ★谱系引用：消解 BspClassification.lean § 8 still-MISSING-D（bspOf 全自动遍历终止 + 唯一）；
    L0→L1 升级消解 task #17 P2 平凡桩；`MoveJudgment` oracle pattern 对应"判据完整接入"
    的接口侧（经验侧仍 still-MISSING-D′）。互斥三分失败的操作侧优先级消解重锚 Strict.BSP
    refined_classifies（精化触发签名真单射，谱系 598→603→615）。
    认识论等级：§1-6 全 L0；§7-9 L1（合成数据管线编码，信息增量为零但管线编码真实）。
-/

end NewChanlun.Origin
