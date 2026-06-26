/-
Origin/SegmentConstruction.lean — segmentsOf 全自动构造 + 终止性 + 唯一性（task #116 构造层）

★工位定位（#113 still-MISSING-A 缺口）：SegmentFeatureSeq.lean 形式化了线段划分的**判据层**
  （特征序列对偶/缺口/包含/两情况/顶高于底/新笔）；但 `segmentsOf : List Stroke → List Segment`
  的**全自动构造**——从原始笔序列递归切出线段序列——的**终止性**与**输出唯一性**未证。
  本文件实装该构造为 Lean 全函数（structurally/well-founded 递归），机器检查终止性，
  并由"纯函数 ⟹ 输出唯一"导出确定性。

═══════════════════════════════════════════════════════════════════════════
构造算法（特征序列法的终止性核心，第67/71/78课）
═══════════════════════════════════════════════════════════════════════════
线段划分递归处理一列笔。终止性的**严格形式**：每步消费 ≥1 根笔，递归参数（剩余笔数）
严格递减 ⟹ well-founded（在 `List.length` 上）。本文件不冒充"完整缠论特征序列分型识别"
（那需 SegmentFeatureSeq 判据 + 分型扫描，是判据层已 formalized 的内容）——本文件证的是
**切分递归的终止性骨架 + 确定性**：给定一个确定的"在哪切"决策函数（feature-seq 判据的
封装），切分递归终止且输出唯一。

切分语义（第78课"以向上笔开始的线段一定结束于向上笔"）：
  - 线段方向 = 起始笔方向；线段从一个转折点延伸到下一个同向转折确认点。
  - `nextSegmentEnd`：扫描笔列，返回本线段消费的笔数（≥1）——这是 feature-seq 判据
    （缺口/分型/两情况）的**确定性封装**。本文件给一个良构的具体实现（消费到下一个
    方向反转点，最少 1 根），并证它总返回 ≥1 ⟹ 递归严格递减 ⟹ 终止。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / well-founded 递归终止 / 纯函数唯一性，不依赖数据）。
`lake env lean Origin/SegmentConstruction.lean` 通过 = segmentsOf 作为全函数良定义（终止）+
输出唯一（确定性）在定义层成立，**不是**任何"切出的线段真对应缠论权威标注"的实证断言（L2+）。

诚实标注（gatekeeper，no-patch-mentality）：
★ TerminationAndDeterminismOnly —— 本文件证 segmentsOf 的**终止性 + 唯一性**（构造层骨架）。
  "在哪切"的决策（`nextSegmentEnd`）是 feature-seq 判据的封装：本文件给的具体实现刻画
  "切到下一个方向反转笔"，是**良构**的（总 ≥1，终止），但**不**等于"完整特征序列分型识别"
  （缺口走第一/第二种情况、非包含处理合并）——那是 SegmentFeatureSeq.lean 已 formalized 的
  判据层。完整判据驱动切分（把 SegmentEndUp/HasGap 接入 nextSegmentEnd）是 still-MISSING-A′
  （见文件尾）：本文件证骨架终止/唯一，不冒充判据完整接入。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。
-/

import Origin.ChanlunElements
import Origin.SegmentFeatureSeq

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 切分决策：nextSegmentEnd —— 本线段消费多少根笔（≥1）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **方向反转检测** —— 一根笔的方向是否与给定线段方向相反（第78课对偶：线段终结在反向序列
  出现对偶分型，最朴素的良构封装即"扫到下一个与起始方向相反的笔即段尾候选"）。
-/
def strokeReverses (segDir : Direction) (s : Stroke) : Bool :=
  s.direction == segDir.flip

/--
  **扫描子程序** —— 从 `acc`（1-based，已含起始笔）起扫 `rest`，遇第一个方向反转笔即返回该
  位置，否则消费整列。top-level def 使等式引理清晰可用于 bound 证明。
-/
def scanSegEnd (segDir : Direction) : List Stroke → Nat → Nat
  | [], acc => acc
  | t :: ts, acc =>
      if strokeReverses segDir t then acc + 1
      else scanSegEnd segDir ts (acc + 1)

/--
  **本线段消费的笔数（≥1）** —— 从笔列 `strokes` 起始（方向 = 第一根笔方向）扫描，
  返回本线段消费的笔数。良构性：**至少消费起始那根笔**（返回 ≥1），保证递归严格递减。

  具体语义（良构封装，非完整判据）：消费起始同向笔，直到遇到方向反转的"确认笔"——
  此处取"消费到（含）第一个反转笔之后的位置"的朴素实现，且对空/单元素列返回该列长度。
  ★关键不变量：返回值 ∈ [1, strokes.length]（§ 2 证），这是终止性的算术核心。
-/
def nextSegmentEnd : List Stroke → Nat
  | [] => 0
  | [_] => 1
  | s :: rest => scanSegEnd s.direction rest 1

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. nextSegmentEnd 的核心不变量：空列返 0，非空列返 ≥1 且 ≤ length
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 内部 scan 的下界：acc ≤ scanSegEnd rest acc（acc 只增不减）。 -/
theorem scanSegEnd_lb (segDir : Direction) (rest : List Stroke) (acc : Nat) :
    acc ≤ scanSegEnd segDir rest acc := by
  induction rest generalizing acc with
  | nil => simp [scanSegEnd]
  | cons t ts ih =>
      unfold scanSegEnd
      split
      · omega
      · exact Nat.le_trans (Nat.le_succ acc) (ih (acc + 1))

/-- 内部 scan 的上界：scanSegEnd rest acc ≤ acc + rest.length。 -/
theorem scanSegEnd_ub (segDir : Direction) (rest : List Stroke) (acc : Nat) :
    scanSegEnd segDir rest acc ≤ acc + rest.length := by
  induction rest generalizing acc with
  | nil => simp [scanSegEnd]
  | cons t ts ih =>
      unfold scanSegEnd
      simp only [List.length_cons]
      split
      · omega
      · have := ih (acc + 1)
        omega

/-- **★段消费下界（终止性核心，L0）** —— 非空笔列 ⟹ nextSegmentEnd ≥ 1。 -/
theorem nextSegmentEnd_pos (strokes : List Stroke) (h : strokes ≠ []) :
    1 ≤ nextSegmentEnd strokes := by
  match strokes with
  | [] => exact absurd rfl h
  | [_] => simp [nextSegmentEnd]
  | s :: t :: rest =>
      unfold nextSegmentEnd
      exact scanSegEnd_lb s.direction (t :: rest) 1

/-- **★段消费上界（L0）** —— nextSegmentEnd ≤ length（不超出笔列）。 -/
theorem nextSegmentEnd_le (strokes : List Stroke) :
    nextSegmentEnd strokes ≤ strokes.length := by
  match strokes with
  | [] => simp [nextSegmentEnd]
  | [_] => simp [nextSegmentEnd]
  | s :: t :: rest =>
      unfold nextSegmentEnd
      have := scanSegEnd_ub s.direction (t :: rest) 1
      simp only [List.length_cons] at *
      omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 从消费的笔构造一个线段（端点几何，第78课顶高于底载体）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **从一段笔（已消费的前 k 根）构造线段** —— 线段方向 = 第一根笔方向；起点 = 第一根笔起点，
  终点 = 最后一根笔终点（第78课：线段从转折点到转折点）。
  对空列返回一个退化占位（不应被调用——构造保证非空，见 § 4）。
-/
def segmentOfStrokes : List Stroke → Segment
  | [] => { direction := Direction.up, startIndex := 0, endIndex := 0, startPrice := 0, endPrice := 0 }
  | s :: rest =>
      let last := (s :: rest).getLast (by simp)
      { direction := s.direction
        startIndex := s.startIndex
        endIndex := last.endIndex
        startPrice := s.startPrice
        endPrice := last.endPrice }

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. segmentsOf 全自动构造（well-founded 递归，终止性机器检查）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★segmentsOf 全自动构造（task #116，终止性核心）** —— `List Stroke → List Segment`。

  递归：消费 `k = nextSegmentEnd strokes` 根笔切出一个线段，对剩余 `strokes.drop k` 递归。
  **终止性**：`k ≥ 1`（`nextSegmentEnd_pos`）⟹ `(strokes.drop k).length < strokes.length`
  （非空时严格递减）⟹ well-founded 递归终止。Lean 的 `termination_by`/`decreasing_by`
  机器检查此递减——这是 #113 still-MISSING-A"终止性未证"的严格消解。

  ★唯一性：segmentsOf 是**纯全函数** ⟹ 输出由输入唯一确定（`segmentsOf_total_unique`，§ 5）。
-/
def segmentsOf (strokes : List Stroke) : List Segment :=
  match strokes with
  | [] => []
  | s :: rest =>
      let k := nextSegmentEnd (s :: rest)
      let consumed := (s :: rest).take k
      let seg := segmentOfStrokes consumed
      seg :: segmentsOf ((s :: rest).drop k)
  termination_by strokes.length
  decreasing_by
    -- (s::rest).drop k 严格短于 s::rest，因为 k ≥ 1
    have hpos : 1 ≤ nextSegmentEnd (s :: rest) :=
      nextSegmentEnd_pos (s :: rest) (by simp)
    have hub : nextSegmentEnd (s :: rest) ≤ (s :: rest).length :=
      nextSegmentEnd_le (s :: rest)
    rw [List.length_drop]
    have hl : (s :: rest).length = rest.length + 1 := by simp
    omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 输出唯一性（确定性）：纯全函数 ⟹ 输出唯一
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★segmentsOf 输出唯一性（task #116，确定性，L0）** —— 给定笔列，segmentsOf 的输出唯一确定。
  这是 ChanlunElements.segments_total_unique 平凡桩的**非平凡见证**：segmentsOf 不再是任意
  `total_unique_of_fun` 的抽象 P.segmentsOf，而是上面 well-founded 终止的**具体**全自动构造，
  其确定性由"纯函数对每输入恰有一个输出"导出。
-/
theorem segmentsOf_total_unique :
    TotalUnique (fun strokes out => segmentsOf strokes = out) :=
  total_unique_of_fun segmentsOf

/-- **★终止性的可观测推论（L0）** —— segmentsOf 对任意输入返回（全函数，不发散）。
  形式化：对任意 strokes 存在输出（全性）——这是 well-founded 终止的语义内容。 -/
theorem segmentsOf_total :
    Total (fun strokes out => segmentsOf strokes = out) := by
  intro strokes; exact ⟨segmentsOf strokes, rfl⟩

/-- **★单值性（确定性，L0）** —— 同一笔列不会产生两个不同线段序列。 -/
theorem segmentsOf_single_valued :
    SingleValued (fun strokes out => segmentsOf strokes = out) :=
  (total_and_single_of_total_unique segmentsOf_total_unique).2

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 计算见证（反退化：具体笔列 ⟹ 具体线段序列，非平凡）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 三根笔：上(10→20) 下(20→15) 上(15→25)。第一段上，第二根反转处切。 -/
def sampleStrokes : List Stroke :=
  [ { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 },
    { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 15 },
    { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 15, endPrice := 25 } ]

/-- ★反退化见证：sampleStrokes 上的 segmentsOf 终止并产出非空线段序列（构造真跑通）。 -/
theorem witness_segmentsOf_nonempty : segmentsOf sampleStrokes ≠ [] := by
  rw [segmentsOf.eq_def]
  simp only [sampleStrokes, ne_eq, reduceCtorEq, not_false_eq_true]

/-- ★反退化见证：空笔列 ⟹ 空线段序列（边界）。 -/
theorem witness_segmentsOf_empty : segmentsOf [] = [] := by
  rw [segmentsOf.eq_def]

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已消解（task #116 相对 #113 still-MISSING-A）：
    - **终止性**：segmentsOf 用 well-founded 递归（`termination_by strokes.length` +
      `decreasing_by` 机器检查），由 `nextSegmentEnd_pos`（消费 ≥1 根笔）保证严格递减。
      #113"非包含处理递归 + 分型识别递归的终止性证明未证"——本文件证了切分递归的终止性骨架。
    - **输出唯一性**：`segmentsOf_total_unique`（纯全函数 ⟹ 输出唯一）+ `segmentsOf_single_valued`。
      这是确定性（同输入同输出），是 #113"输出唯一性未证"的消解。

  ★still-MISSING-A′（判据完整接入，诚实开口）：
    本文件的 `nextSegmentEnd` 是 feature-seq 判据的**良构封装**（扫到方向反转笔即切），它满足
    终止性所需的"消费 ≥1"不变量，但**不**等于 SegmentFeatureSeq.lean 的完整判据驱动切分——
    完整版须：(1) 按方向对偶提取特征序列元素（FeatureElem.ofStroke），(2) 递归非包含处理
    （mergeInclusion）得标准特征序列，(3) 识别顶/底分型（IsTopFractal），(4) 按缺口有无走
    SegmentEndUp 第一/第二种情况确认。把 (1)-(4) 接入 nextSegmentEnd 使切分点 = 真特征序列
    确认点，是 still-MISSING-A′。**本文件不声称**切出的线段真对应缠论权威标注——只声称
    切分递归终止 + 确定。把骨架冒充为完整判据接入 = 声明膨胀（禁止）。

  ★边界条件（结论翻转）：
    - 终止性依赖 `nextSegmentEnd ≥ 1`（非空列）。若 nextSegmentEnd 在某输入返回 0（消费 0 根），
      递归不递减 ⟹ 不终止。`nextSegmentEnd_pos` 排除此情形（非空列总 ≥1）——故终止性对**当前**
      nextSegmentEnd 成立；若 still-MISSING-A′ 接入完整判据后某判据允许"消费 0 根"，须重证终止
      （完整判据须保持"消费 ≥1"不变量，否则终止性翻转）。
    - 唯一性是 segmentsOf 作为函数的内蕴性质，对任何确定性 nextSegmentEnd 都成立——接入完整
      判据不影响唯一性（只要判据本身确定）。

  ★下游推论：
    - CenterConstruction.lean（centersOf）以 segmentsOf 输出为输入——线段序列构造终止 ⟹
      centersOf 的输入良定义。
    - segmentsOf 终止 + 唯一 ⟹ ChanlunElements.ElementPipeline.segmentsOf 可用本文件具体实例化
      （非 total_unique_of_fun 平凡桩）——消解审计判决的"segmentsOf 是平凡桩"。

  ★影响声明：
    - 新增 Origin.SegmentConstruction 模块，import ChanlunElements + SegmentFeatureSeq（只读）。
    - 不改 canonical 类型，无反向依赖，无命名冲突。待 Lead 登记 root：`Origin.SegmentConstruction`。

  ★谱系引用：消解 SegmentFeatureSeq.lean § 8 still-MISSING-A 的终止性/唯一性缺口（构造层）；
    `.chanlun/genealogy/settled/001-degenerate-segment.md`（退化线段：本文件 nextSegmentEnd
    保证非退化消费 ≥1，与退化线段谱系一致——不产生 0 长度消费）。
-/

end NewChanlun.Origin
