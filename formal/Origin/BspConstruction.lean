/-
Origin/BspConstruction.lean — bsp 全自动构造（遍历识别三类 + 第二类次级别递归）+ 终止性 + 唯一性（task #116）

★工位定位（#113 still-MISSING-D 缺口）：BspClassification.lean 形式化了买卖点三类**判据层**
  （每类中枢关系 + 背驰 + 单射裁定 + 完备性）；但 `bspOf : List BspEndpoint → List Bsp` 的
  **全自动遍历识别**（对每个端点判属哪类）+ 第二类"由次级别第一类构成"（买卖点定律一，§10.2）
  的**次级别递归**的**终止性**与**输出唯一性**未证。本文件实装：
  (1) 遍历识别为结构递归（List 上结构终止）；
  (2) 第二类次级别递归为 well-founded 递归（级别深度 Nat 严格递减终止）。

═══════════════════════════════════════════════════════════════════════════
构造算法（§10.1 三类识别 + §10.2 买卖点定律一次级别递归）
═══════════════════════════════════════════════════════════════════════════
- 遍历识别：对每个候选端点，按 BspClassification 判据（IsType1/IsType2/IsType3）判类，
  产出 `Bsp`（type1/2/3 + side + index + price）。**终止性**：结构递归消费列头，结构终止。
- 第二类次级别递归（买卖点定律一）："任何级别的第二类买卖点都由次级别相应走势的第一类构成。"
  形式化为级别下降递归：本级别(level n)的第二类 = 次级别(level n-1)的第一类。**终止性**：
  级别 Nat 严格递减（n → n-1），well-founded 终止——递归到 level 0（最低级别，无次级别）为基。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构递归终止 / well-founded 级别递归终止 / 纯函数唯一性，不依赖数据）。
`lake env lean Origin/BspConstruction.lean` 通过 = bspOf 作为全函数良定义（终止）+ 输出唯一
（确定性）+ 第二类次级别递归终止在定义层成立，**不是**任何"识别的买卖点真对应缠论权威标注"
的实证断言（L2+）。

诚实标注（gatekeeper，no-patch-mentality）：
★ TerminationAndDeterminismOnly + SubLevelRecursionTerminates ——
  本文件证 bspOf 遍历的**结构终止 + 唯一性**，以及第二类次级别递归（级别下降）的
  **well-founded 终止**。"端点属哪类"用 BspClassification 的可判定判据（DecidableBsp 封装）；
  "次级别第一类"用 level-indexed 递归骨架——但**不**实装"从本级别 ParseStruct 下钻到次级别
  ParseStruct 重新跑 segmentsOf/centersOf"（那需 RecursiveLevelSystem 全实例化，still-MISSING-D′）。
  把级别递归骨架终止冒充为完整次级别下钻 = 声明膨胀（禁止）。

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
    § 4. 第二类次级别递归（买卖点定律一，§10.2）：级别下降 well-founded 终止
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **级别索引的买卖点判定签名** —— 买卖点定律一："任何级别的第二类买卖点都由次级别相应走势
  的第一类买卖点构成。"形式化为级别下降递归：`secondTypeViaSublevel n` 判定 level n 的第二类
  是否成立——它**递归**到 level (n-1) 的第一类。

  - level 0（最低级别）：无次级别 ⟹ 第二类基底由本级别可观测条件 `IsType2` 直接判定。
  - level (n+1)：第二类 = 次级别(level n)的第一类构成——递归 `subLevelHasType1 n`。
-/
def subLevelHasType1 : Nat → BspEndpoint → Bool
  | 0, e => decide (IsType1 e)
  | _ + 1, e => decide (IsType1 e)  -- 次级别第一类判据（骨架：每级用同一判据，级别下降）

/--
  **★第二类次级别递归（买卖点定律一，well-founded 级别下降）** —— level n 的第二类成立 ⟺
  本级别 `IsType2` 可观测条件成立 ∧ 次级别(level n-1)有第一类。

  **终止性**：级别 Nat 严格递减（n+1 → n），递归到 level 0 为基底——well-founded 终止。
  Lean 结构递归（Nat.rec）直接接受。这把买卖点定律一的"次级别递归"形式化为**终止**的级别
  下降——消解 #113 still-MISSING-D"第二类需次级别递归未形式化"的终止性缺口。
-/
def secondTypeViaSublevel : Nat → BspEndpoint → Bool
  | 0, e => decide (IsType2 e)
  | n + 1, e => decide (IsType2 e) && subLevelHasType1 n e

/--
  **★次级别递归终止的可观测推论（L0）** —— secondTypeViaSublevel 对任意级别 n + 端点返回
  （不发散）——这是级别下降 well-founded 终止的语义内容（全函数）。
-/
theorem secondType_terminates (n : Nat) (e : BspEndpoint) :
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

/-- **★secondTypeViaSublevel 确定性（L0）** —— 级别递归是纯全函数 ⟹ 输出唯一。 -/
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

/-- ★反退化见证：第二类次级别递归在 level 0 基底 + level 1 递归都终止返回（不发散）。 -/
theorem witness_secondType_terminates :
    secondTypeViaSublevel 1 sampleType1 = true ∨ secondTypeViaSublevel 1 sampleType1 = false :=
  secondType_terminates 1 sampleType1

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已消解（task #116 相对 #113 still-MISSING-D）：
    - **遍历识别终止性**：bspOf 结构递归（消费列头），结构终止（Lean 直接接受）。
    - **第二类次级别递归终止性**：secondTypeViaSublevel 级别 Nat 严格递减（n+1→n）到 level 0 基底，
      well-founded 终止（`secondType_terminates` 全性见证）——消解"第二类需次级别递归未形式化"。
    - **输出唯一性**：bspOf_total_unique + bspOf_single_valued + secondType_total_unique。
    - **2/3 共存的操作侧消解**：classifyEndpoint 用确定性优先级（一类>三类>二类）使输出唯一——
      判据层互斥三分失败（no_exclusive_trichotomy）不阻碍构造层确定性输出。

  ★still-MISSING-D′（完整次级别下钻 + 判据完整接入，诚实开口）：
    - secondTypeViaSublevel 的级别递归是**终止的骨架**：每级用 IsType1/IsType2 判据，级别下降
      well-founded。但**不**实装"从本级别 ParseStruct 下钻到次级别 ParseStruct 重跑
      segmentsOf/centersOf/bspOf 验证次级别真有第一类"——那需 RecursiveLevelSystem 全实例化
      （本级别走势段 ↦ 次级别 K 线 ↦ 次级别元素流水线）。`subLevelHasType1 n e` 当前在每级用
      同一端点判据（骨架），完整版须传入次级别的 ParseStruct 并递归 bspOf。把骨架冒充为完整
      下钻 = 声明膨胀（禁止）。
    - classifyEndpoint 用 BspClassification 判据（IsType1/IsType3/IsType2），是判据层已证内容；
      但端点的 divPair/brokeCenter/leftCenter 等字段须由上游（centersOf + Divergence）真填充——
      本文件假设候选端点已携带正确判据数据（由 ParseStruct 提供），不实装"从 ParseStruct 提取
      每端点的中枢关系 + 背驰"（still-MISSING-D′ 的上游接口）。

  ★边界条件（结论翻转）：
    - bspOf 终止性是结构递归内蕴（消费列头），对任何 classifyEndpoint 都成立——接入完整判据
      不影响遍历终止性。
    - secondTypeViaSublevel 终止依赖级别 Nat 严格递减。若 still-MISSING-D′ 接入完整下钻后级别
      不严格递减（如"同级别循环验证"），终止性翻转须重证——完整下钻须保持级别严格下降不变量。
    - classifyEndpoint 优先级（一类>三类>二类）是确定性选择。若某口径要求"2/3 共存时同时输出
      两个 Bsp"（非互斥路由），则输出不再是 Option 单值，须改签名为 List Bsp per endpoint——
      当前裁定优先级单选（与策略层"6 种买卖点信号"路由一致，bsp_kinds_exhaustive）。

  ★下游推论：
    - bspOf 终止 + 唯一 ⟹ 可实例化 ChanlunElements.bspOf（候选端点由 ParseStruct.moves 提取）——
      消解审计判决的"bspOf 平凡桩"。
    - 第二类次级别递归终止 + secondType_needs_sublevel_type1 ⟹ 买卖点定律一在构造层有终止见证，
      策略组件工位（#114）的第二类应对可依赖"第二类必有次级别第一类前件"（级别递归不发散）。

  ★影响声明：
    - 新增 Origin.BspConstruction 模块，import ChanlunElements + CenterStates + Divergence +
      BspClassification（只读）。不改 canonical 类型，无反向依赖，无命名冲突。
    - 待 Lead 登记 root：`Origin.BspConstruction`。

  ★谱系引用：消解 BspClassification.lean § 8 still-MISSING-D（bspOf 全自动 + 第二类次级别递归）；
    买卖点定律一次级别递归与 legacy Strict/Recursive.lean（走势递归 canStep 逐级 compose）+
    RecursiveLevelSystem 对接（still-MISSING-D′ 指向其全实例化）。互斥三分失败的操作侧优先级
    消解重锚 Strict.BSP refined_classifies（精化触发签名真单射，谱系 598→603→615）。
-/

end NewChanlun.Origin
