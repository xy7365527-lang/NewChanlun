/-
Origin/CenterAutoAssign.lean — 中枢自动分配（centersOf 输出 → 相邻中枢发展对 → 自动喂下游），task #130

★工位定位（#130 still-MISSING：中枢自动分配缺口）：
  CenterConstruction.lean 的 `centersOf : List Segment → List CenterFull` 全自动识别中枢序列
  （终止 + 唯一，#116 committed）；CenterFull.lean 提供 `CenterFull.developmentWith prev next :
  CenterDevelopment`（把 CenterStates.classifyDevelopment 抬到 CenterFull 上，发展三态：延伸/
  新生/扩展）。**但 centersOf 的输出从未被自动喂给下游**——Pipeline.lean 的 `originPipeline`
  把 ChanlunEvent 所需的中枢发展对 `dev : CenterWithOuter` 作为**显式参数手工传入**
  （Pipeline.lean 第 158-166 行 candidateToEvent/pipelineEvents 对每个候选端点用**同一个**手工
  占位 dev；第 529-531 行诚实标注："dev 参数当前显式传入——完整版从构造链 centers 相邻
  CenterFull 对自动读出（CenterFull.developmentWith）"）。

  本文件坐实那条"自动读出"——把 `centersOf` 输出的 `List CenterFull` **自动**转为下游所需的
  中枢发展态序列（相邻中枢对 → developmentWith → CenterDevelopment）+ 自动事件发展对序列
  （相邻中枢对 → CenterWithOuter 对），消去手工分配占位。证：自动分配是 centersOf 的**纯函数
  后继**（终止 + 唯一 + 长度关系 = 中枢数 - 1），且自动产出的中枢对**恰是** centersOf 输出的
  相邻对（无手工注入、无占位替换）。

═══════════════════════════════════════════════════════════════════════════
自动分配构造（相邻对提取 = 中枢序列的发展态串接）
═══════════════════════════════════════════════════════════════════════════
中枢发展态由**相邻两中枢**的关系决定（CenterStates §6.7：前中枢 prev + 后中枢 next → 延伸/
新生/扩展）。一条中枢序列 [c₀, c₁, …, cₙ] 的发展态串 = 对每个相邻对 (cᵢ, cᵢ₊₁) 应用
classifyDevelopment——共 n 个发展态（中枢数 - 1）。

自动分配 = 把这个"相邻对 → 发展态"映射作为 centersOf 的纯函数后继：
  segs ──centersOf──▶ [c₀,…,cₙ] ──adjacentCenterPairs──▶ [(c₀,c₁),…,(cₙ₋₁,cₙ)]
                                  ──map developmentWith──▶ [d₀,…,dₙ₋₁]
不再有"手工传入一个 dev 占位"——发展对**从 centersOf 输出结构性读出**。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构递归终止 / 纯函数唯一性 / List.length 归纳，rfl/omega/induction
machine-checked，不依赖数据）。`lake env lean Origin/CenterAutoAssign.lean` 通过 =
「自动分配作为 centersOf 的纯函数后继良定义（终止）+ 输出唯一（确定性）+ 自动产出的相邻对
恰是 centersOf 输出的相邻对（无手工占位）+ 发展态串长度 = 中枢数 - 1」在定义层成立，**不是**
任何「自动分配的发展态在真实行情上对应权威标注」的实证断言（L2+）。

诚实标注（gatekeeper，no-patch-mentality）：
★ AutoAssignFromCentersOnly —— 本文件证自动分配是 centersOf 输出的**结构性后继**（相邻对提取
  + developmentWith 映射），消去 Pipeline 手工 dev 占位。**不**冒充：
  - 发展态判据本身的正确性（继承 CenterStates classifyDevelopment 的有效域 = 核心已分离对，
    谱系 256/264——延伸支为 ¬分离，本文件原样继承，不重证）。
  - 每个买卖点端点应配哪个相邻中枢对（端点 ↔ 中枢对的**索引对齐**是 still-MISSING-align，
    见文件尾——本文件产出中枢序列的全部相邻发展对，端点到发展对的索引选择是上游接口）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root
`Origin.CenterAutoAssign`）。**不改** Pipeline.lean / CenterConstruction.lean / CenterFull.lean
（distinct 新模块，只读 import）。
依赖方向（单向无环，全 committed 只读）：CenterAutoAssign → {CenterConstruction, CenterFull,
  CenterStates, ChanlunElements}（均 Origin 内 committed + 已登记 root）。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.CenterFull
import Origin.CenterConstruction

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 相邻中枢对提取（中枢序列 → 相邻对序列）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★相邻中枢对提取 `adjacentCenterPairs`（L0，自动分配核心）** —— `List CenterFull →
  List (CenterFull × CenterFull)`。把一条中枢序列转为其**全部相邻对**：
  `[c₀,c₁,…,cₙ]` ↦ `[(c₀,c₁),(c₁,c₂),…,(cₙ₋₁,cₙ)]`。

  这是发展态串接的结构骨架——中枢发展态由相邻两中枢的关系决定（CenterStates §6.7），故
  发展态序列 = 相邻对序列上逐对应用 classifyDevelopment。

  **终止性**：结构递归（消费列头 `c1 :: c2 :: rest` → 递归 `c2 :: rest`），结构终止（List.rec）——
  Lean 直接接受（无需 termination_by）。< 2 个中枢 ⟹ 无相邻对 ⟹ []。
-/
def adjacentCenterPairs : List CenterFull → List (CenterFull × CenterFull)
  | [] => []
  | [_] => []
  | c1 :: c2 :: rest => (c1, c2) :: adjacentCenterPairs (c2 :: rest)

/--
  **相邻对数 = 中枢数 - 1（L0，长度关系）** —— `n` 个中枢恰有 `n - 1` 个相邻对（n ≥ 1 时）；
  0 个中枢 0 对。结构归纳。这坐实自动分配的发展态串长度由中枢数唯一确定（无手工增删）。
-/
theorem adjacentCenterPairs_length (cs : List CenterFull) :
    (adjacentCenterPairs cs).length = cs.length - 1 := by
  induction cs with
  | nil => rfl
  | cons c1 rest ih =>
    cases rest with
    | nil => rfl
    | cons c2 rest' =>
      simp only [adjacentCenterPairs, List.length_cons]
      -- (adjacentCenterPairs (c2 :: rest')).length + 1 = (rest'.length + 1 + 1) - 1
      rw [ih]
      simp only [List.length_cons]
      omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 自动发展态串接（相邻对 → developmentWith → CenterDevelopment）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★自动发展态串 `autoDevelopmentsOfCenters`（L0，自动分配主体）** —— `List CenterFull →
  List CenterDevelopment`。对中枢序列的每个相邻对应用 `CenterFull.developmentWith`
  （= CenterStates.classifyDevelopment ∘ toOuter 双投影）。

  `[c₀,…,cₙ]` ↦ `[developmentWith c₀ c₁, …, developmentWith cₙ₋₁ cₙ]`。

  ★这是手工 dev 占位的结构性替代：Pipeline 原来对每端点用同一手工 dev；这里发展态**从 centersOf
  输出的相邻中枢对逐对自动算出**——发展对来源 = 构造链中枢流（非手工传入）。
-/
def autoDevelopmentsOfCenters (cs : List CenterFull) : List CenterDevelopment :=
  (adjacentCenterPairs cs).map (fun p => CenterFull.developmentWith p.1 p.2)

/--
  **★自动发展对（CenterWithOuter 对）`autoDevPairsOfCenters`（L0）** —— `List CenterFull →
  List (CenterWithOuter × CenterWithOuter)`。把每个相邻 CenterFull 对投影为 ChanlunEvent
  所需的 `CenterWithOuter` 对（prev/next）。

  ★这是 Pipeline.candidateToEvent 的 `dev : CenterWithOuter` 占位的结构性来源：ChanlunEvent 需要
  `prevCenter next Center : CenterWithOuter`；本函数从 centersOf 输出的相邻 CenterFull 对自动产出
  这些 (prev, next) 对（CenterFull.toOuter 投影），无手工注入。
-/
def autoDevPairsOfCenters (cs : List CenterFull) : List (CenterWithOuter × CenterWithOuter) :=
  (adjacentCenterPairs cs).map (fun p => (p.1.toOuter, p.2.toOuter))

/--
  **发展态串长度 = 中枢数 - 1（L0）** —— 继承 adjacentCenterPairs_length（map 保长）。
  自动分配产出的发展态数由中枢数唯一确定。
-/
theorem autoDevelopments_length (cs : List CenterFull) :
    (autoDevelopmentsOfCenters cs).length = cs.length - 1 := by
  unfold autoDevelopmentsOfCenters
  rw [List.length_map, adjacentCenterPairs_length]

/--
  **发展对串长度 = 中枢数 - 1（L0）** —— 继承 adjacentCenterPairs_length（map 保长）。
-/
theorem autoDevPairs_length (cs : List CenterFull) :
    (autoDevPairsOfCenters cs).length = cs.length - 1 := by
  unfold autoDevPairsOfCenters
  rw [List.length_map, adjacentCenterPairs_length]

/--
  **★自动发展态串接定义一致（L0）** —— 自动发展态串的第 i 项 = 对应相邻对的 developmentWith。
  本定理把"自动分配逐对算发展态"显式化为可观测等式（map 的逐项语义），坐实发展态来自相邻对
  （非手工 dev 占位的常量串）。
-/
theorem autoDevelopments_eq_map (cs : List CenterFull) :
    autoDevelopmentsOfCenters cs =
      (adjacentCenterPairs cs).map (fun p => CenterFull.developmentWith p.1 p.2) := rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 自动分配是 centersOf 的纯函数后继（segs → 自动发展态/发展对）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★端到端自动分配 `autoDevAssign`（L0，#130 核心交付）** —— `List Segment →
  List CenterDevelopment`。从线段序列**全自动**产出中枢发展态串：
  `segs ──centersOf──▶ List CenterFull ──autoDevelopmentsOfCenters──▶ List CenterDevelopment`。

  ★这坐实 centersOf 输出**自动可用于下游**：无需手工分配——发展态从 centersOf 输出的中枢序列
  结构性读出。这是 #130 still-MISSING（中枢自动分配）的直接消解。
-/
def autoDevAssign (segs : List Segment) : List CenterDevelopment :=
  autoDevelopmentsOfCenters (centersOf segs)

/--
  **★端到端自动发展对 `autoEventDevPairs`（L0）** —— `List Segment →
  List (CenterWithOuter × CenterWithOuter)`。从线段序列全自动产出 ChanlunEvent 所需的发展对串：
  `segs ──centersOf──▶ List CenterFull ──autoDevPairsOfCenters──▶ List (CenterWithOuter × CenterWithOuter)`。

  ★这是 Pipeline.pipelineEvents 的手工 `dev` 参数的结构性替代来源：每个事件的发展对从这里的
  自动串按索引取（Lead 接线点，见文件尾），不再手工传同一占位 dev。
-/
def autoEventDevPairs (segs : List Segment) : List (CenterWithOuter × CenterWithOuter) :=
  autoDevPairsOfCenters (centersOf segs)

/--
  **★自动分配 = centersOf 后继（类型对接 well-defined，L0）** —— autoDevAssign 的输入恰是
  centersOf 的输出：`autoDevAssign segs = autoDevelopmentsOfCenters (centersOf segs)`。
  本定理把"centersOf 输出自动喂给发展态分配"显式化为可观测等式——centersOf 输出 ⊑
  autoDevelopmentsOfCenters 输入（逐段对接，无手工中介）。
-/
theorem autoDevAssign_feeds_centersOf (segs : List Segment) :
    autoDevAssign segs = autoDevelopmentsOfCenters (centersOf segs) := rfl

/--
  **★自动发展对 = centersOf 后继（类型对接 well-defined，L0）** —— 同上，发展对串从 centersOf 输出读出。
-/
theorem autoEventDevPairs_feeds_centersOf (segs : List Segment) :
    autoEventDevPairs segs = autoDevPairsOfCenters (centersOf segs) := rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 终止性 + 唯一性（确定性）：纯全函数后继
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★自动分配输出唯一性（确定性，L0）** —— 给定线段列，autoDevAssign 输出唯一确定。
  纯全函数复合（centersOf 后继 adjacentCenterPairs 后继 map developmentWith）⟹ 输出由输入唯一确定。
  复用 #116 committed centersOf 唯一性 + 本文件结构递归终止 + map 纯函数性。
-/
theorem autoDevAssign_total_unique :
    TotalUnique (fun segs out => autoDevAssign segs = out) :=
  total_unique_of_fun autoDevAssign

/-- **★全性（终止性可观测推论，L0）** —— autoDevAssign 对任意输入返回（不发散）。 -/
theorem autoDevAssign_total :
    Total (fun segs out => autoDevAssign segs = out) := by
  intro segs; exact ⟨autoDevAssign segs, rfl⟩

/-- **★单值性（确定性，L0）** —— 同一线段列不产生两个不同发展态串。 -/
theorem autoDevAssign_single_valued :
    SingleValued (fun segs out => autoDevAssign segs = out) :=
  (total_and_single_of_total_unique autoDevAssign_total_unique).2

/-- **★自动发展对唯一性（确定性，L0）** -/
theorem autoEventDevPairs_total_unique :
    TotalUnique (fun segs out => autoEventDevPairs segs = out) :=
  total_unique_of_fun autoEventDevPairs

/--
  **★相邻对提取唯一性（确定性，L0）** —— adjacentCenterPairs 是纯全函数 ⟹ 输出唯一。
-/
theorem adjacentCenterPairs_total_unique :
    TotalUnique (fun cs out => adjacentCenterPairs cs = out) :=
  total_unique_of_fun adjacentCenterPairs

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 自动产出的相邻对恰是 centersOf 输出的相邻对（无手工占位，反伪造）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★发展态串第一项 = 头两个中枢的 developmentWith（L0，无手工占位见证）** —— 当 centersOf
  输出至少两个中枢 `c0 :: c1 :: rest` 时，自动发展态串的头 = `developmentWith c0 c1`
  （**c0, c1 真是 centersOf 输出的头两个中枢**，非手工注入的占位）。

  ★这坐实"自动产出的发展对取自 centersOf 输出的相邻中枢"——与 Pipeline 手工 dev 占位（对每端点
  用同一外部传入的 CenterWithOuter）的本质区别：这里发展对的两个分量 c0/c1 来自 centersOf 输出
  结构，无外部注入。
-/
theorem autoDev_head_from_adjacent (c0 c1 : CenterFull) (rest : List CenterFull) :
    autoDevelopmentsOfCenters (c0 :: c1 :: rest) =
      CenterFull.developmentWith c0 c1
        :: autoDevelopmentsOfCenters (c1 :: rest) := by
  unfold autoDevelopmentsOfCenters
  simp only [adjacentCenterPairs, List.map_cons]

/--
  **★发展对串第一项 = 头两个中枢的 toOuter 对（L0，无手工占位见证）** —— 自动发展对串的头分量
  恰是 centersOf 输出头两个中枢的 CenterWithOuter 投影（prev = c0.toOuter, next = c1.toOuter）。
-/
theorem autoDevPair_head_from_adjacent (c0 c1 : CenterFull) (rest : List CenterFull) :
    autoDevPairsOfCenters (c0 :: c1 :: rest) =
      (c0.toOuter, c1.toOuter) :: autoDevPairsOfCenters (c1 :: rest) := by
  unfold autoDevPairsOfCenters
  simp only [adjacentCenterPairs, List.map_cons]

/--
  **★退化边界：单中枢 ⟹ 无发展态（L0）** —— 只有一个中枢时无相邻对 ⟹ 发展态串为空
  （发展态需要前后两中枢，单中枢无发展态可分配）。
-/
theorem autoDev_singleton_empty (c : CenterFull) :
    autoDevelopmentsOfCenters [c] = [] := rfl

/--
  **★退化边界：空中枢序列 ⟹ 无发展态（L0）** -/
theorem autoDev_empty_empty :
    autoDevelopmentsOfCenters [] = [] := rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 反退化计算见证（具体线段列 → 自动发展态，整链真跑通）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  两个相邻 CenterFull（核心分离 + 外缘分离 ⟹ 新生）用于见证自动分配真算发展态。
  c0 核心[10,20] 外缘[5,25]；c1 核心[30,40] 外缘[28,45]（后dd=28 > 前gg=25 ⟹ 上涨新生）。
-/
def cfA : CenterFull :=
  { core := { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }
    dd := 5, gg := 25, outer_lo := by decide, outer_hi := by decide }

def cfB : CenterFull :=
  { core := { zd := 30, zg := 40, startIndex := 4, endIndex := 7, valid := by decide }
    dd := 28, gg := 45, outer_lo := by decide, outer_hi := by decide }

/-- ★反退化见证：两中枢序列 [cfA, cfB] 自动产出恰一个发展态（中枢数2 - 1 = 1）。 -/
theorem witness_autoDev_length_one :
    (autoDevelopmentsOfCenters [cfA, cfB]).length = 1 := by
  unfold autoDevelopmentsOfCenters cfA cfB
  rfl

/-- ★反退化见证：[cfA, cfB] 自动产出的发展态 = 新生（上涨趋势，外缘分离，真算出非占位）。 -/
theorem witness_autoDev_newBirth :
    autoDevelopmentsOfCenters [cfA, cfB] = [CenterDevelopment.newBirth] := by
  unfold autoDevelopmentsOfCenters CenterFull.developmentWith CenterFull.toOuter
    classifyDevelopment cfA cfB
  decide

/-- ★反退化见证：[cfA, cfB] 自动产出的发展对头分量 = (cfA.toOuter, cfB.toOuter)（取自相邻中枢，非占位）。 -/
theorem witness_autoDevPair_head :
    autoDevPairsOfCenters [cfA, cfB] = [(cfA.toOuter, cfB.toOuter)] := by
  unfold autoDevPairsOfCenters
  simp only [adjacentCenterPairs, List.map_cons, List.map_nil]

/-- ★反退化见证：端到端自动分配在 CenterConstruction 见证线段列上跑通（三段重叠 ⟹ 一个中枢 ⟹ 无发展态）。
    [ovSeg1,ovSeg2,ovSeg3] ──centersOf──▶ 恰一个中枢 ⟹ autoDevAssign 空（单中枢无相邻对）。 -/
theorem witness_autoDevAssign_single_center_empty :
    autoDevAssign [ovSeg1, ovSeg2, ovSeg3] = [] := by
  unfold autoDevAssign autoDevelopmentsOfCenters
  -- centersOf [ovSeg1,ovSeg2,ovSeg3] 恰一个中枢（witness_centersOf_one），单中枢无相邻对。
  rw [centersOf.eq_def]
  dsimp only []
  rw [dif_pos overlapping_holds]
  -- 余项 centersOf [] = []（well-founded 递归，用 witness_centersOf_too_few 显式归约）。
  rw [witness_centersOf_too_few]
  -- 中枢序列 = [单中枢]，adjacentCenterPairs [单] = []（[_] 分支），map [] = []。
  rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 诚实标签（gatekeeper：禁标完整/盈利） + 自动分配子类
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★自动分配标签 `AutoAssignTag`（gatekeeper，诚实分层）。
  - `autoFromCentersOf`：发展态/发展对从 centersOf 输出的相邻中枢对结构性读出（消手工 dev 占位）。
  - `terminating`：自动分配是结构递归终止的纯全函数（adjacentCenterPairs 结构终止）。
  - `deterministic`：自动分配输出唯一（纯全函数后继，复用 centersOf 唯一性）。
  - `lengthDetermined`：发展态数 = 中枢数 - 1（由中枢序列唯一确定，无手工增删）。
  - `endpointAlignMissing`：端点 ↔ 发展对的**索引选择**（哪个端点配哪个相邻中枢对）是上游接口，
    本文件产全部相邻发展对，不定端点到发展对的对齐（still-MISSING-align）。
  - `devCriterionInherited`：发展态判据正确性继承 CenterStates classifyDevelopment（有效域=核心
    已分离对，谱系 256/264），本文件不重证判据。
  - `empiricalDomain`：自动发展态对应权威标注 = L2+，不由本 L0 声称。
-/
inductive AutoAssignTag where
  | autoFromCentersOf
  | terminating
  | deterministic
  | lengthDetermined
  | endpointAlignMissing
  | devCriterionInherited
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★自动分配子类（gatekeeper）：AutoAssignFromCentersOnly（唯一构造子，禁标完整/盈利）。 -/
inductive AutoAssignSubkind where
  | autoAssignFromCentersOnly
deriving DecidableEq, Repr

/-- ★诚实标签包（L0 声明）。 -/
def autoAssignLabels : List AutoAssignTag × AutoAssignSubkind :=
  ([AutoAssignTag.autoFromCentersOf, AutoAssignTag.terminating, AutoAssignTag.deterministic,
    AutoAssignTag.lengthDetermined, AutoAssignTag.endpointAlignMissing,
    AutoAssignTag.devCriterionInherited, AutoAssignTag.empiricalDomain],
   AutoAssignSubkind.autoAssignFromCentersOnly)

/-- ★禁标完整/盈利（L0，gatekeeper 见证）：子类必是 AutoAssignFromCentersOnly。 -/
theorem autoAssign_subkind_is_from_centers (k : AutoAssignSubkind) :
    k = AutoAssignSubkind.autoAssignFromCentersOnly := by
  cases k; rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. 交付总结 + still-MISSING + 边界 + 下游 + Lead 接线依赖 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  本文件**证**（L0 结构层，machine-checked，无 sorry/admit/axiom）：

  1. ★相邻中枢对提取 `adjacentCenterPairs`（§1）：中枢序列 → 全部相邻对，结构递归终止；
     相邻对数 = 中枢数 - 1（`adjacentCenterPairs_length`）。
  2. ★自动发展态串 `autoDevelopmentsOfCenters` + 自动发展对 `autoDevPairsOfCenters`（§2）：
     对每相邻对应用 CenterFull.developmentWith / toOuter 投影；长度 = 中枢数 - 1。
  3. ★端到端自动分配 `autoDevAssign` + `autoEventDevPairs`（§3，#130 核心交付）：
     `List Segment ──centersOf──▶ List CenterFull ──自动──▶ 发展态/发展对串`——
     centersOf 输出**自动喂下游**（`autoDevAssign_feeds_centersOf` 坐实类型对接）。
  4. ★终止 + 确定性（§4）：`autoDevAssign_total` + `autoDevAssign_total_unique` +
     `autoDevAssign_single_valued`（复用 #116 centersOf 唯一性 + 结构递归终止）。
  5. ★无手工占位见证（§5）：`autoDev_head_from_adjacent`（发展态头取自 centersOf 输出头两中枢）+
     `autoDevPair_head_from_adjacent`（发展对头分量 = 头两中枢 toOuter）+ 退化边界（单/空中枢无发展态）。
  6. ★反退化计算见证（§6）：[cfA,cfB] 自动算出 newBirth（非占位）+ 端到端 autoDevAssign 在
     CenterConstruction 见证线段列上跑通。
  7. 诚实标签（§7）`autoAssignLabels` + `autoAssign_subkind_is_from_centers`（禁标完整/盈利）。

  本文件**不证**（no声明膨胀 / no-workaround，诚实边界）：
  - ✗ **端点 ↔ 发展对索引对齐**（still-MISSING-align，endpointAlignMissing）：本文件产出中枢序列的
       **全部**相邻发展对串；但"第 k 个买卖点端点应配哪个相邻中枢对"（端点到发展对的索引选择）
       是上游接口——依赖端点在中枢序列中的位置定位（ParseStruct 留存每端点的中枢归属），#113/#116
       still-open。本文件**不**冒充已定该对齐——只坐实发展对串从 centersOf 自动产出。
  - ✗ **发展态判据正确性**（devCriterionInherited）：继承 CenterStates classifyDevelopment 的有效域
       （= 核心已分离对，谱系 256/264 重锚），本文件不重证判据，只把它映射到 centersOf 输出。
  - ✗ 自动发展态对应真实行情权威标注（L2+ empiricalDomain）。

  ★边界条件（结论翻转）：
    - 相邻对提取按**列表相邻**（cᵢ, cᵢ₊₁）。若某口径要求"跨非中枢段的相邻"（中枢间隔着非中枢
      走势段时相邻定义改变），adjacentCenterPairs 须改，长度关系须重证。当前 centersOf 输出已是
      纯中枢序列（滑窗已消费非中枢段），故列表相邻 = 中枢相邻。
    - 发展态串长度 = 中枢数 - 1 依赖"发展态由相邻两中枢决定"。若某口径用三中枢窗口定发展态
      （而非两中枢对），长度关系翻转为中枢数 - 2，须重构。当前 CenterStates §6.7 用两中枢对。
    - 单中枢 ⟹ 空发展态（发展态需前后两中枢）。若某口径定义"单中枢自身的发展态"（如延伸到
      自身），退化边界翻转。当前裁定单中枢无发展态（与 §6.7 前后对一致）。

  ★下游推论：
    - 端到端自动分配 ⟹ Pipeline.lean 的手工 `dev` 占位可由本文件自动串替代：管线不再需要外部
      传入 CenterWithOuter 占位，发展对从 autoEventDevPairs (centersOf segs) 按端点位置取。
    - centersOf 输出自动可用 ⟹ 构造链 segments → centers → **发展态** 全自动贯通（原来 centers 到
      发展态有手工断点，现已接通）。

  ★★★【已接线·#130 consolidation 完成】Lead 已在 consolidation 把本模块接进 Pipeline.lean（下方
    五步方案已全部执行：import CenterAutoAssign + autoEventDevPairs 替代 dev 参数 + candidateToEvent
    拆 prev/next + pipelineEvents 用 List.zip + pipeline_preserves_ledger_inv 去 dev 重证通过）。
    以下接线方案为执行记录（保留作谱系）。

    Pipeline.lean 接线前的手工占位接线点（精确定位）：
    - **接线点 1**（Pipeline.lean 第 158-159 行 `candidateToEvent`）：
        `def candidateToEvent (c : BspCandidate) (dev : CenterWithOuter) : ChanlunEvent :=`
        `  { bsp := c.endpoint, prevCenter := dev, nextCenter := dev }`
      手工占位：prev/next 都用同一外部传入 dev。
    - **接线点 2**（Pipeline.lean 第 165-166 行 `pipelineEvents`）：
        `def pipelineEvents (cands : List BspCandidate) (dev : CenterWithOuter) : List ChanlunEvent :=`
        `  cands.map (fun c => candidateToEvent c dev)`
      手工占位：对每端点用同一 dev。
    - **接线点 3**（Pipeline.lean 第 180-181 行 `pipelineAccount` / 第 215-219 行 `originPipeline`
        的 `dev : CenterWithOuter` 参数）：dev 是 originPipeline 的显式输入参数。

    **具体接线方案**（Lead 在 consolidation 执行，把本模块接入 Pipeline.lean）：
    1. Pipeline.lean 增 `import Origin.CenterAutoAssign`（本模块已登记 root 后）。
    2. 用本文件 `autoEventDevPairs (pipelineSegments inp) : List (CenterWithOuter × CenterWithOuter)`
       替代 originPipeline 的显式 `dev` 参数——发展对从构造链 centers 自动读出。
    3. `candidateToEvent` 改签名为接 `(prev next : CenterWithOuter)`（拆 prev/next，不再 dev 共用），
       `prevCenter := prev, nextCenter := next`。
    4. `pipelineEvents` 用 `List.zip cands (autoEventDevPairs segs)` 把每端点配其对应相邻发展对
       （端点到发展对的**索引对齐**是 still-MISSING-align——本模块产全部相邻对，对齐策略由 Lead
       裁定或标 still-open；最简对齐 = zip 截断，端点 k 配第 k 个相邻发展对）。
    5. 重证 Pipeline 的 `pipeline_preserves_ledger_inv` 等（fold 结构不变，dev 来源改为自动串不影响
       R=Π-A-W 贯穿——ledger inv 是结构内蕴，与 dev 来源无关）。

    ★这是明确的 Lead 接线依赖：本模块**不擅自改 Pipeline.lean**（共享文件），自动分配构造已就位，
      接入由 Lead 在 consolidation 按上述方案执行。本模块自身 `lake env lean` 自检通过、零 sorry、
      不依赖 Pipeline.lean。

  ★影响声明：
    - 新增 Origin.CenterAutoAssign 模块（中枢自动分配层），import {ChanlunElements, CenterStates,
      CenterFull, CenterConstruction}（均 committed + 已登记 root，只读）。
    - **不改** 任何 committed 类型/模块（Pipeline.lean / CenterConstruction.lean / CenterFull.lean
      均只读 import），无反向依赖，无命名冲突（新定义 adjacentCenterPairs/autoDevelopmentsOfCenters/
      autoDevPairsOfCenters/autoDevAssign/autoEventDevPairs 均 Origin 命名空间内首现）。
    - 待 Lead 登记 root：`Origin.CenterAutoAssign`（无工具自登记，报 Lead 代置）。
    - 待 Lead 接线：把自动分配接进 Pipeline.lean（具体方案见上「Lead 接线依赖」）。

  ★谱系引用：消解 #130 still-MISSING（中枢自动分配——centersOf 输出未自动喂下游）；接续
    CenterConstruction #116（centersOf 终止+唯一）+ CenterFull #116（developmentWith）+ CenterStates
    #113（classifyDevelopment 发展三态，谱系 256/264 有效域重锚）。无概念分离谱系涉及（纯结构
    后继：把 centersOf 输出结构性映射为发展态串，不引入新定义冲突）。
  ═══════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin
