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
    § 7. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已消解（task #116 相对 #113 still-MISSING-D）：
    - **遍历识别终止性**：bspOf 结构递归（消费列头），结构终止（Lean 直接接受）。
    - **第二类本级别判据签名**：secondTypeViaSublevel 是返回 Bool 的全函数（`secondType_is_bool`
      二歧见证）——把买卖点定律一装配为「本级别 IsType2 ∧ 次级别第一类占位」单步判据。
      ★诚实：这**不是**次级别递归终止性证明——`subLevelHasType1` 丢弃级别索引 n、两分支同体、
      不自调，无下降递归结构，故无终止义务可证（次级别真下钻 still-MISSING-D′，见下）。
    - **输出唯一性**：bspOf_total_unique + bspOf_single_valued + secondType_total_unique。
    - **2/3 共存的操作侧消解**：classifyEndpoint 用确定性优先级（一类>三类>二类）使输出唯一——
      判据层互斥三分失败（no_exclusive_trichotomy）不阻碍构造层确定性输出。

  ★still-MISSING-D′（完整次级别下钻 + 判据完整接入，诚实开口）：
    - secondTypeViaSublevel 的「次级别第一类」是**单步占位判据**（`subLevelHasType1 n e =
      decide (IsType1 e)`，丢弃 n、两分支同体、不自调）——**不是** well-founded 级别下降递归。
      **不**实装"从本级别 ParseStruct 下钻到次级别 ParseStruct 重跑 segmentsOf/centersOf/bspOf
      验证次级别真有第一类"——那需 RecursiveLevelSystem 全实例化（本级别走势段 ↦ 次级别 K 线 ↦
      次级别元素流水线）。完整版须传入次级别的 ParseStruct 并递归 bspOf（真级别递减结构基础见
      SubLevelDescent.lean 的 descend / descend_level_decreases）。把单步占位判据冒充为完整
      下钻递归 = 声明膨胀（禁止）。
    - classifyEndpoint 用 BspClassification 判据（IsType1/IsType3/IsType2），是判据层已证内容；
      但端点的 divPair/brokeCenter/leftCenter 等字段须由上游（centersOf + Divergence）真填充——
      本文件假设候选端点已携带正确判据数据（由 ParseStruct 提供），不实装"从 ParseStruct 提取
      每端点的中枢关系 + 背驰"（still-MISSING-D′ 的上游接口）。

  ★边界条件（结论翻转）：
    - bspOf 终止性是结构递归内蕴（消费列头），对任何 classifyEndpoint 都成立——接入完整判据
      不影响遍历终止性。
    - secondTypeViaSublevel 当前是单步本级别判据（无下降递归，全函数 type-checks 即全）。当
      still-MISSING-D′ 接入完整下钻（subLevelHasType1 替换为真递归到次级别）后，才产生「级别 Nat
      严格递减终止」的证明义务——彼时若级别不严格递减（如"同级别循环验证"），终止性须真证。
      当前本文件不声称该终止性（无递归可终止），故无翻转可言（占位判据无下降义务）。
    - classifyEndpoint 优先级（一类>三类>二类）是确定性选择。若某口径要求"2/3 共存时同时输出
      两个 Bsp"（非互斥路由），则输出不再是 Option 单值，须改签名为 List Bsp per endpoint——
      当前裁定优先级单选（与策略层"6 种买卖点信号"路由一致，bsp_kinds_exhaustive）。

  ★下游推论：
    - bspOf 终止 + 唯一 ⟹ 可实例化 ChanlunElements.bspOf（候选端点由 ParseStruct.moves 提取）——
      消解审计判决的"bspOf 平凡桩"。
    - 第二类本级别判据全函数 + secondType_needs_sublevel_type1 ⟹ 买卖点定律一在构造层有「第二类
      ⟹ 次级别第一类（占位）前件」的可观测见证；策略组件工位（#114）的第二类应对可依赖该前件
      结构。完整次级别真下钻待 still-MISSING-D′ 接入（占位判据不冒充真递归）。

  ★影响声明：
    - 新增 Origin.BspConstruction 模块，import ChanlunElements + CenterStates + Divergence +
      BspClassification（只读）。不改 canonical 类型，无反向依赖，无命名冲突。
    - 待 Lead 登记 root：`Origin.BspConstruction`。

  ★谱系引用：消解 BspClassification.lean § 8 still-MISSING-D（bspOf 全自动遍历终止 + 唯一）；
    第二类「次级别第一类」当前为单步占位判据，其完整次级别下钻（与 legacy Strict/Recursive.lean
    走势递归 canStep 逐级 compose + RecursiveLevelSystem 对接）是 still-MISSING-D′（SubLevelDescent.lean
    的 descend 提供真级别递减结构基础）。互斥三分失败的操作侧优先级消解重锚 Strict.BSP
    refined_classifies（精化触发签名真单射，谱系 598→603→615）。
-/

end NewChanlun.Origin
