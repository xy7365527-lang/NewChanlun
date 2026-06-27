/-
Origin/RMoveCompose.lean — 第二类走势递归组装层（task #51，L3 第二类递归组装层工位，D′）

★工位定位（#5 单声部根因「第二类结构不可产」的真完整态核心缺口）：
  L2 signal.rs 已产 B1/S1/B3/S3（第一类破中枢背驰 + 第三类离开回试），但 B2/S2（第二类）
  诚实标「递归组装层缺口」——第二类是中枢内部回拉结构，需走势递归组装（RMove::Compose +
  descend）才能产生。本文件形式化+实装该**第二类走势递归组装层**（结构层 L0）。

  本文件**不**做 B2/S2 信号提取层（留下一轮，依赖①迁主塔修正后的中枢区间口径②本文件产的
  递归结构）——本文件只产**第二类走势结构**（结构层），不碰 signal.rs。

═══════════════════════════════════════════════════════════════════════════
精确增量缺口定位（相对已有 composeStep / descend / SubLevelDescent）
═══════════════════════════════════════════════════════════════════════════
已有（本文件 build on，不重造）：
  · `composeStep`（ChanlunInstantiation）：RMove::Compose 递归组装核心——把第 n 级走势序列
    `List Move` 经规范窗口族切窗，逐窗 `Move.compose` 封装成第 n+1 级走势序列（走势分解
    定理二，#89/#99 窗口化 ≥3-witness 递归）。
  · `descend`（SubLevelDescent）：lift/composeStep 的逆——从本级别走势取回封装的次级别走势
    序列 `subs`（`descend_compose` rfl）+ 级别严格递减（`descend_level_decreases`，well-founded）。
  · `secondType_via_subLevel_type1`（SubLevelDescent）：买卖点定律一几何层——第二类 ⟸ 次级别
    有破中枢走势（次级别第一类的几何必要条件）。
  · `IsType2`（BspClassification）：第二类**本级别判据层**——`afterTypeOne ∧ ¬brokeCenter`
    （一类后回抽结束点，诚实标 still-MISSING-D：「由次级别一类构成」是构成性陈述未证递归构造）。

本文件**真增量**（缺失的「第二类走势结构层」，连接判据层与次级别构成层）：
  缺口 = 第二类买卖点作为**走势位置结构**本身的 canonical 形式化——它由 RMove::Compose
  递归组装产生，且满足「回拉不创新低/新高」位置约束。这是 `IsType2` 本级别判据
  （afterTypeOne ∧ ¬brokeCenter，无走势结构）与 `SubLevelDescent` 次级别第一类必要条件
  之间**缺失的中间层**：把第二类的本级别**回拉走势位置约束**（第15课「未创新低」）+ 它的
  **递归组装来源**（descend 取回的回拉次级别走势的第一类，第14课买点定律一）**统一**为一个
  走势结构定义，并证明它在 RMove::Compose 递归塔中由组装产生。

═══════════════════════════════════════════════════════════════════════════
canonical 依据（一级权威博文 + reference §5/§10）
═══════════════════════════════════════════════════════════════════════════
- 第14课（博文，一级权威，买点定律一原始出处）："这里给出一个缠中说禅买点定律：大级别的
  第二类买点由次一级别相应走势的第一类买点构成。例如，周线上的第二类买点由日线上相应走势的
  第一类买点构成。有了这个缠中说禅买点定律，所有的买点都可以归结到第一类买点。"
- 第15课（博文）："第二类买点都是第一次上0轴后回抽确认形成的"+「比较像背驰但**未创新低**」
  ——第二类的回抽**不创新低**（买点）/不创新高（卖点）。
- §10.1（reference）：第二类买点 = 第一类买点后，**次级别上涨结束、再次下跌的那个次级别走势
  的结束点**（BspClassification.lean:13 引）。⟹ 第二类点 = 回拉次级别走势的结束点。
- §五 走势递归（reference 第176行「自相似的走势递归核」+ 走势分解定理二）：高级别走势由次级别
  走势递归组装（`composeStep`），本级别第二类结构封装次级别回拉走势。
- 走势分解定理二（zoushi 第17课，#89）：任何级别走势 ≥3 段次级别走势构成——第二类的本级别
  走势 `Move.compose subs …` 的 `subs` 含「第一类离开走势 + 回拉走势」。

═══════════════════════════════════════════════════════════════════════════
第二类走势结构定义（本文件 canonical 形式化）
═══════════════════════════════════════════════════════════════════════════
第二类走势结构 `SecondTypeStructure side parent center divPair`：本级别走势 `parent`
（RMove::Compose 组装，携带次级别走势序列 `descend parent`）的第二类买卖点结构 ⟺
  (1) **第一类离开走势**：`descend parent` 中存在次级别走势 `m1`，是完整次级别第一类
      （破中枢 ∧ 背驰，`SubLevelType1`，SubLevelDescent）——第一类锚（买点定律一的「次级别
      第一类」，第14课）。
  (2) **回拉走势**：`descend parent` 中 m1 **之后**存在次级别走势 `m2`（反向回拉）。
  (3) **不创新低/新高**：回拉走势 m2 的极值（买点侧=m2.lo 回拉低点 / 卖点侧=m2.hi 回拉高点）
      **不突破** m1 的极值（买点：`m2.lo ≥ m1.lo` 不创新低 / 卖点：`m2.hi ≤ m1.hi` 不创新高，
      第15课「未创新低」）。
第二类买卖点 = 回拉走势 m2 的结束点（§10.1「次级别再次下跌走势的结束点」）。

★它如何由递归组装产生：parent 由 `composeStep`/`Move.compose` 把次级别走势序列组装为本级别
  走势（RMove::Compose 递归组装层），`descend parent` 取回该序列。第二类结构在**取回的次级别
  序列内部**识别「第一类离开 m1 + 回拉 m2 不创新低」——这是中枢内部回拉结构的真递归组装形式
  （第一类离开是次级别趋势破中枢，回拉是次级别反向走势，二者都在 parent 封装的 subs 中）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯结构定义 / descend 递归取回 / 回拉极值几何比较 / 级别递减，不依赖数据）。
`lake env lean Origin/RMoveCompose.lean` 通过 = 第二类走势结构（第一类离开 + 回拉不创新低/新高
+ 递归组装来源）在定义层成立，**不是**「第二类识别在真实行情上有效」的实证断言（那是 L2+，
需真实 K 线 + MACD 力度计算 still-MISSING-C）。

诚实开口（gatekeeper，no-patch / no-声明膨胀）：
★ 本文件证**第二类走势结构层**（第一类离开走势 + 回拉走势 + 不创新低/新高 + 由 descend 取回
  的递归组装来源 + 买卖点定律一连接）。但第一类的**背驰力度**（`SubLevelType1` 的 `divPair`）
  靠外部提供（MACD 面积 still-MISSING-C，无 Origin 引擎从 K 线算力度，承接 SubLevelDescent
  same 卡点）。把「回拉几何结构」冒充为「完整第二类识别」= 声明膨胀（禁止）——本文件诚实把
  `divPair` 设为显式参数。
★ B2/S2 **信号提取层**（从本级别 parent 自动提取第二类端点 BspEndpoint）留下一轮（依赖迁主塔
  中枢区间口径定稿，no-patch 不在未定口径上构建提取层）。本文件只产**走势结构**，提取层缺口
  精确边界见 §卡点。
禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。
-/

import Origin.SubLevelDescent
import Origin.BspClassification
import Formal.RecursiveConstruction

namespace NewChanlun.Origin.RMoveCompose

open Formal.RecursiveConstruction (WellFormed)
open NewChanlun.Origin.SubLevelDescent
  (RMove RCenter descend descend_compose descend_segment descend_level_decreases
   SubBrokeBelow SubBrokeAbove SubLevelType1 subLevelType1_imp_broke
   subReclassifyBroke subLevelHasBrokenCenter type1_needs_subBrokenCenter
   secondType_via_subLevel_type1)

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. RMove::Compose 递归组装层（组装本级别走势 + descend 取回次级别序列）

    `composeStep`（ChanlunInstantiation）/`Move.compose`（RecursiveConstruction）是 RMove::Compose
    递归组装的核心——把次级别走势序列封装成本级别走势。本节给「组装-取回」对偶的接口引理：
    `descend (Move.compose subs centers lvl) = subs`（descend_compose 重锚），坐实 RMove::Compose
    组装的次级别序列可被 descend 真取回——这是第二类结构在 subs 内部识别的前提。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★RMove::Compose 组装的本级别走势（组装算子别名，重锚 RecursiveConstruction.Move.compose）：
  把次级别走势序列 `subs` + 派生中枢 `centers` 封装成级别 `lvl` 的本级别走势。这是 `composeStep`
  逐窗 map 用的单走势组装子——第二类的本级别走势由它组装（subs 含第一类离开 + 回拉走势）。
-/
def composeMove (subs : List RMove) (centers : List RCenter) (lvl : Nat) : RMove :=
  Formal.RecursiveConstruction.Move.compose subs centers lvl

/--
  ★组装-取回对偶（L0，rfl）：`descend (composeMove subs centers lvl) = subs`——RMove::Compose
  组装的次级别走势序列可被 descend 真取回。这坐实第二类结构在「取回的次级别序列内部」识别
  （第一类离开 + 回拉）是良定义的：组装进去的 subs 恰是 descend 取回的 subs。
-/
theorem descend_composeMove (subs : List RMove) (centers : List RCenter) (lvl : Nat) :
    descend (composeMove subs centers lvl) = subs := by
  unfold composeMove; exact descend_compose subs centers lvl

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 回拉走势「不创新低/新高」位置约束（第15课「未创新低」几何层）

    第二类的回拉走势 m2（次级别再次下跌/上涨）其极值**不突破**第一类离开走势 m1 的极值：
      - 买点侧（long）：回拉低点 `m2.lo ≥ m1.lo`（**不创新低**——回拉没跌破第一类的低点）。
      - 卖点侧（short）：回拉高点 `m2.hi ≤ m1.hi`（**不创新高**——回抽没涨破第一类的高点）。
    这是 §10.1「次级别再次下跌走势的结束点」+ 第15课「未创新低」的几何刻画——对 descend 取回的
    **真实次级别回拉走势对象**算（m2.lo/m2.hi = 回拉走势区间极值，m1.lo/m1.hi = 第一类走势极值）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★回拉不创新低（买点侧，几何层）—— 回拉走势 `m2` 的低点 `m2.lo` 不低于第一类走势 `m1` 的
  低点 `m1.lo`（`m1.lo ≤ m2.lo`）。第二类买点的核心位置约束（第15课「未创新低」）：第一类
  探底后，回拉再次下跌但**没创新低**——回拉低点守住第一类低点上方（含相等，临界不创新低）。
-/
def NoNewLow (m1 m2 : RMove) : Prop := m1.lo ≤ m2.lo

instance (m1 m2 : RMove) : Decidable (NoNewLow m1 m2) := by
  unfold NoNewLow; infer_instance

/--
  ★回拉不创新高（卖点侧，几何层）—— 回拉走势 `m2` 的高点 `m2.hi` 不高于第一类走势 `m1` 的
  高点 `m1.hi`（`m2.hi ≤ m1.hi`）。第二类卖点对偶位置约束：第一类探顶后，回抽再次上涨但
  **没创新高**——回抽高点守住第一类高点下方（含相等，临界不创新高）。
-/
def NoNewHigh (m1 m2 : RMove) : Prop := m2.hi ≤ m1.hi

instance (m1 m2 : RMove) : Decidable (NoNewHigh m1 m2) := by
  unfold NoNewHigh; infer_instance

/--
  ★回拉位置约束（按方向，几何层）—— 第二类的回拉走势 `m2` 相对第一类走势 `m1` 不创极值：
  买点侧判不创新低，卖点侧判不创新高。这把「未创新低/新高」按买卖方向统一。
-/
def RetraceNoBreak (side : Side) (m1 m2 : RMove) : Prop :=
  match side with
  | Side.long  => NoNewLow m1 m2
  | Side.short => NoNewHigh m1 m2

instance (side : Side) (m1 m2 : RMove) : Decidable (RetraceNoBreak side m1 m2) := by
  unfold RetraceNoBreak; cases side <;> infer_instance

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 第二类走势结构（canonical 形式化：第一类离开 + 回拉不创新低/新高 + 递归组装来源）

    第二类买卖点作为**走势位置结构**：本级别走势 `parent`（RMove::Compose 组装，subs =
    descend parent）的第二类结构 ⟺ subs 内部存在「第一类离开走势 m1 + 其后回拉走势 m2，m2
    不创新低/新高」。第二类点 = 回拉走势 m2 的结束点（§10.1）。

    ★m1 在 m2 之前（时间顺序，§10.1「第一类后、再次下跌」）：用 `descend parent` 的列表索引
    `i1 < i2` 刻画——m1 在 i1 位置（第一类离开），m2 在 i2 位置（回拉），i1 < i2（回拉在第一类
    之后）。这是「第一类后回拉」的时间序约束（与 RecursiveConstruction `StrictlyIncreasing`
    窗口起点序同精神：subs 内部时间有序）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★第二类走势结构（canonical 形式化，L0 结构定义）—— 本级别走势 `parent` 的第二类买卖点结构。

  `SecondTypeStructure side parent c1 divPair`：parent（RMove::Compose 组装）的第二类结构 ⟺
  存在两个次级别走势 m1（第一类离开）、m2（回拉）+ 索引 i1 < i2，使：
  - `m1 ∈ descend parent` 在索引 i1，`m2 ∈ descend parent` 在索引 i2，`i1 < i2`
    （第一类离开在前、回拉在后，§10.1「第一类后再次下跌」时间序）；
  - `SubLevelType1 side m1 c1 divPair`：m1 是完整次级别第一类（破中枢 ∧ 背驰，第14课买点
    定律一「由次级别第一类构成」+ §10.1）。`divPair` 显式参数（力度 still-MISSING-C 外部提供）；
  - `RetraceNoBreak side m1 m2`：回拉走势 m2 不创新低/新高（第15课「未创新低」，相对第一类 m1）。

  第二类买卖点 = 回拉走势 m2 的结束点（§10.1）。这是「中枢内部回拉结构」的真递归组装形式——
  第一类离开是次级别破中枢趋势走势，回拉是次级别反向走势，二者都在 parent 由 RMove::Compose
  封装的 subs（= descend parent）中，第二类结构在取回的次级别序列内部识别。
-/
def SecondTypeStructure (side : Side) (parent : RMove) (c1 : RCenter)
    (divPair : DivergencePair) : Prop :=
  ∃ (i1 i2 : Nat) (m1 m2 : RMove),
    i1 < i2
    ∧ (descend parent : List RMove)[i1]? = some m1
    ∧ (descend parent : List RMove)[i2]? = some m2
    ∧ SubLevelType1 side m1 c1 divPair
    ∧ RetraceNoBreak side m1 m2

/--
  ★第二类点 = 回拉走势的结束点（§10.1，L0 投影）—— 给定第二类走势结构的 m2（回拉走势），
  第二类买卖点价位 = 回拉走势的极值结束点（买点 = 回拉低点 m2.lo，卖点 = 回拉高点 m2.hi）。

  这把「第二类买卖点」从走势结构投影到具体价位——§10.1「次级别再次下跌走势的结束点」。
-/
def secondPointPrice (side : Side) (m2 : RMove) : Int :=
  match side with
  | Side.long  => m2.lo
  | Side.short => m2.hi

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 买卖点定律一连接（第二类走势结构 ⟹ 次级别有第一类，第14课真下钻）

    第二类走势结构含第一类离开走势 m1（`SubLevelType1`）⟹ 次级别**真有第一类**（descend 取回
    的真实次级别走势 + 破中枢 + 背驰）⟹ 经 SubLevelDescent 的 `secondType_via_subLevel_type1`
    见证「次级别有破中枢走势」（买卖点定律一几何必要条件）。这把本文件的「第二类走势结构」与
    SubLevelDescent 已证的「买卖点定律一几何层」**真正连接**——第二类结构 ⟹ 买卖点定律一前件成立。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★第二类走势结构 ⟹ 次级别有破中枢走势（买卖点定律一连接，L0）。

  第二类走势结构含第一类离开走势 m1（`SubLevelType1 side m1 c1 divPair`，descend 取回的真实
  次级别走势）⟹ 经 SubLevelDescent `secondType_via_subLevel_type1`，次级别存在破中枢走势
  （`subLevelHasBrokenCenter` 真）——买卖点定律一「第二类由次级别第一类构成」的几何必要条件
  在第二类走势结构上**机器成立**。

  这坐实本文件的「第二类走势结构」与 SubLevelDescent 的「买卖点定律一几何层」连贯：第二类
  走势结构（含第一类离开 m1）⟹ 买卖点定律一前件（次级别有破中枢走势）。`centerOf` 取回拉
  结构的第一类中枢分配函数（次级别每走势配其相关中枢，still-MISSING-B 外部提供）。
-/
theorem secondTypeStructure_imp_subBrokenCenter
    (side : Side) (parent : RMove) (c1 : RCenter) (divPair : DivergencePair)
    (centerOf : RMove → RCenter)
    (hstruct : SecondTypeStructure side parent c1 divPair)
    (hcenter : ∀ m1 ∈ descend parent, centerOf m1 = c1) :
    subLevelHasBrokenCenter parent side centerOf = true := by
  obtain ⟨_i1, _i2, m1, _m2, _hlt, hm1, _hm2, h1, _hretrace⟩ := hstruct
  have hmem : m1 ∈ descend parent := List.mem_of_getElem? hm1
  have hcof : centerOf m1 = c1 := hcenter m1 hmem
  -- h1 : SubLevelType1 side m1 c1 divPair；目标需 SubLevelType1 side m1 (centerOf m1) divPair
  have h1' : SubLevelType1 side m1 (centerOf m1) divPair := hcof ▸ h1
  exact secondType_via_subLevel_type1 parent side centerOf divPair m1 hmem h1'

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. descend 下降递归终止（well-founded，重锚 SubLevelDescent + 级别递减）

    第二类走势结构递归到次级别（descend 取回的 m1/m2 是次级别走势，级别 = parent.level - 1）。
    买卖点定律一「所有买点归结到第一类」是级别递归（第二类 → 次级别第一类 → 次次级别…），
    well-founded 终止由级别严格递减保证（descend_level_decreases）：每次 descend 级别真减一，
    递归到 level 0（线段，descend 得空，无第二类结构——递归底）为基。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★第二类结构的次级别走势级别递减（well-founded 基础，L0）—— 对**良构** parent
  （`Move.compose subs centers lvl`），其第二类结构的第一类离开走势 m1（∈ descend parent）
  级别 = lvl - 1（严格低一级）。

  这是买卖点定律一级别递归 well-founded 的结构基础：第二类（本级别 lvl）→ 第一类离开走势
  （次级别 lvl-1）→ 该次级别走势再递归…级别每次真减一，递归到 level 0（线段，descend 得空，
  无第二类结构）为基。重锚 SubLevelDescent `descend_level_decreases`。
-/
theorem secondType_m1_level_decreases
    (sl : List RMove) (centers : List RCenter) (lvl : Nat)
    (h : WellFormed (composeMove sl centers lvl))
    (m1 : RMove) (hm1 : m1 ∈ descend (composeMove sl centers lvl)) :
    m1.level = lvl - 1 := by
  unfold composeMove at h hm1
  exact descend_level_decreases sl centers lvl h m1 hm1

/--
  ★线段无第二类结构（递归底，L0）—— 线段（level 0）descend 得空序列，无任何次级别走势，
  故无第一类离开走势 m1（`descend (segment …) = []`，无索引可取）⟹ 无第二类走势结构。

  这是买卖点定律一级别递归的**终止基底**：递归到 level 0 线段时，descend 得空，第二类结构的
  存在量词（需 `(descend parent)[i1]? = some m1`）无解 ⟹ 无第二类。递归在线段终止（无限下降被
  级别递减 + 线段底关死，well-founded）。
-/
theorem segment_no_secondType (side : Side) (d : Formal.TrendTrichotomy.Direction)
    (lo hi : Int) (c1 : RCenter) (divPair : DivergencePair) :
    ¬ SecondTypeStructure side (Formal.RecursiveConstruction.Move.segment d lo hi) c1 divPair := by
  intro ⟨i1, _i2, m1, _m2, _hlt, hm1, _hm2, _h1, _hretrace⟩
  rw [descend_segment d lo hi] at hm1
  simp at hm1

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 反退化见证（具体第二类走势结构：真递归组装 + 第一类离开 + 回拉不创新低）

    构造一个本级别走势 `parentWit2`（RMove::Compose 组装：第一类离开走势 + 回拉走势 + 收尾），
    descend 取回真实次级别走势序列，第一类离开真破中枢 + 背驰，回拉真不创新低——见证第二类
    走势结构真跑通（非平凡：回拉不创新低是真约束，破之即非第二类）。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 第一类离开走势（向下跌破中枢的次级别走势，区间 [-10, -2]，下沿 -10 < 中枢 ZD=0 ⟹ 破中枢）。 -/
def m1Wit : RMove :=
  Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.down (0 - 10) (0 - 2)

/-- 回拉走势（再次下跌但不创新低，区间 [-8, 3]，下沿 -8 ≥ 第一类下沿 -10 ⟹ 不创新低）。 -/
def m2Wit : RMove :=
  Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.up (0 - 8) 3

/-- 收尾走势（第三段，凑满 ≥3 段走势分解定理二）。 -/
def m3Wit : RMove :=
  Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.up 1 5

/-- 第二类结构的次级别中枢（核心 [ZD, ZG] = [0, 4]）。 -/
def c1Wit : RCenter :=
  { dd := 0 - 2, zd := 0, zg := 4, gg := 6,
    core_valid := by decide, outer_lo := by decide, outer_hi := by decide }

/-- 本级别走势见证：RMove::Compose 组装三个次级别走势（第一类离开 + 回拉 + 收尾）+ 中枢，级别 1。 -/
def parentWit2 : RMove :=
  composeMove [m1Wit, m2Wit, m3Wit] [c1Wit] 1

/-- 背驰力度见证（forceC=2 < forceA=8 ⟹ 背驰，第一类力度前提，still-MISSING-C 外部提供）。 -/
def divWit2 : DivergencePair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true }

/-- **★反退化见证：RMove::Compose 组装 + descend 取回真实次级别走势序列（非空，含第一类离开+回拉）。** -/
theorem witness_descend_parentWit2 :
    descend parentWit2 = [m1Wit, m2Wit, m3Wit] := by
  unfold parentWit2; exact descend_composeMove [m1Wit, m2Wit, m3Wit] [c1Wit] 1

/-- **★反退化见证：第一类离开走势真跌破中枢（m1.lo=-10 < zd=0）。** -/
theorem witness_m1_broke : SubBrokeBelow m1Wit c1Wit := by
  unfold SubBrokeBelow m1Wit c1Wit
    Formal.RecursiveConstruction.Move.lo Formal.RecursiveConstruction.Move.interval
  decide

/-- **★反退化见证：回拉走势真不创新低（m2.lo=-8 ≥ m1.lo=-10）。** -/
theorem witness_retrace_noNewLow : NoNewLow m1Wit m2Wit := by
  unfold NoNewLow m1Wit m2Wit
    Formal.RecursiveConstruction.Move.lo Formal.RecursiveConstruction.Move.interval
  decide

/-- **★反退化见证（非平凡）：若回拉创新低则非第二类位置——回拉低点 -12 < 第一类低点 -10 ⟹ ¬NoNewLow。**
    这证「不创新低」是真约束（破之即非第二类买点位置，第15课「未创新低」非平凡桩）。 -/
theorem witness_newLow_breaks_type2 :
    ¬ NoNewLow m1Wit (Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.up (0 - 12) 3) := by
  unfold NoNewLow m1Wit
    Formal.RecursiveConstruction.Move.lo Formal.RecursiveConstruction.Move.interval
  decide

/-- **★反退化见证主定理：具体第二类走势结构真跑通（RMove::Compose 组装 + 第一类离开破中枢背驰
    + 回拉不创新低 + i1=0 < i2=1 时间序）。**
    背驰力度由外部 divWit2（forceC=2 < forceA=8）提供，几何破中枢 + 不创新低真下钻判定。 -/
theorem witness_secondTypeStructure :
    SecondTypeStructure Side.long parentWit2 c1Wit divWit2 := by
  refine ⟨0, 1, m1Wit, m2Wit, ?_, ?_, ?_, ?_, ?_⟩
  · decide  -- i1=0 < i2=1（第一类离开在前、回拉在后）
  · rw [witness_descend_parentWit2]; rfl  -- (descend)[0]? = some m1Wit
  · rw [witness_descend_parentWit2]; rfl  -- (descend)[1]? = some m2Wit
  · -- SubLevelType1：第一类离开破中枢 ∧ 背驰
    unfold SubLevelType1
    refine ⟨?_, ?_⟩
    · simp only; exact witness_m1_broke
    · unfold IsDivergence divWit2; decide
  · -- RetraceNoBreak（long）：回拉不创新低
    show NoNewLow m1Wit m2Wit; exact witness_retrace_noNewLow

/-- **★反退化见证：第二类走势结构 ⟹ 次级别有破中枢走势（买卖点定律一连接真跑通）。**
    从具体第二类结构经 `secondTypeStructure_imp_subBrokenCenter`，见证买卖点定律一前件成立。 -/
theorem witness_secondType_imp_brokenCenter :
    subLevelHasBrokenCenter parentWit2 Side.long (fun _ => c1Wit) = true :=
  secondTypeStructure_imp_subBrokenCenter Side.long parentWit2 c1Wit divWit2
    (fun _ => c1Wit) witness_secondTypeStructure (fun _ _ => rfl)

/-- **★反退化见证：第二类买卖点价位 = 回拉走势结束点（买点 = 回拉低点 m2.lo = -8）。** -/
theorem witness_secondPoint_price :
    secondPointPrice Side.long m2Wit = (0 - 8 : Int) := by
  unfold secondPointPrice m2Wit
    Formal.RecursiveConstruction.Move.lo Formal.RecursiveConstruction.Move.interval
  decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 卡点性质 + still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ═══════════════════════════════════════════════════════════════════════
  ★卡点性质（task #51 核心交付：实现难 or 定义冲突？）
  ═══════════════════════════════════════════════════════════════════════
  判定：**实现难度 + 上游数据缺失（力度 still-MISSING-C + 信号提取层依赖迁主塔口径），
  不是定义冲突**。

  论证（formalization-validity-domain 有效域 ⊊ 定义域）：
  - **第二类走势结构层无定义冲突**：RMove::Compose 组装（`composeStep`/`Move.compose`）+ descend
    取回（`descend_composeMove` rfl）+ 回拉不创新低/新高几何（`RetraceNoBreak`）+ 第一类离开
    （`SubLevelType1`，SubLevelDescent）+ 时间序 i1<i2 + 买卖点定律一连接
    （`secondTypeStructure_imp_subBrokenCenter`）+ 级别递减终止（`segment_no_secondType` 递归底）
    ——全部定义自洽且实装，零 sorry。第二类的两个 canonical 来源（第14课「由次级别第一类构成」
    + 第15课「未创新低」）在结构层**统一无矛盾**：第一类离开走势 m1 承载「次级别第一类构成」，
    回拉走势 m2 不创新低承载「未创新低」，二者在 descend 取回的同一 subs 序列内部 i1<i2 共存。

  - **卡点1 = 第一类的力度分量无数据来源（still-MISSING-C，承接 SubLevelDescent）**：
    第二类结构含第一类离开 m1，第一类 = 破中枢（几何，本文件 `SubBrokeBelow/Above` 真下钻可判）
    ∧ 背驰（力度 `divPair`，MACD 面积无 Origin 引擎，Divergence.lean still-MISSING-C）。故
    `SubLevelType1` 的 `divPair` 必须**外部提供**（本文件设为 `SecondTypeStructure` 的显式参数，
    诚实标注数据来源在外，不藏进 decide 假装算了）。

  - **卡点2 = B2/S2 信号提取层依赖迁主塔中枢区间口径（实现难/上游缺失，非定义冲突）**：
    本文件产**第二类走势结构**（结构层），未产 B2/S2 **信号提取**（从本级别 parent 自动提取
    第二类端点 `BspEndpoint` + 注入 signal.rs）。提取层依赖①迁主塔修正后的中枢区间口径
    （本轮迁主塔工位并行改中枢核心区间，no-patch 不在未定口径上构建提取层）②本文件产的递归
    结构。补上中枢口径定稿后，提取层可消费本文件的 `SecondTypeStructure` 产 B2/S2——结构层
    不需改动，只需补提取适配器。

  - **为何不是定义冲突**：定义冲突 = 接受 A 则 B 矛盾，无法弥合（no-workaround）。此处无矛盾——
    第二类走势结构（第一类离开 + 回拉不创新低 + 递归组装来源 + 买卖点定律一连接）三者定义自洽
    且全实装；缺的只是**力度数据引擎**（MACD 面积，L2 数值层）+ **信号提取适配器**（依赖迁主塔
    口径，L2 工程层），那是**实现/上游缺失**，不是定义之间打架。
    （对比真定义冲突：若「第二类要求回拉不创新低」与「§10.1 次级别再次下跌走势」矛盾，那才是
    冲突——但二者一致：再次下跌（回拉方向）+ 不创新低（回拉幅度受限），同一回拉走势的方向与
    幅度约束，无矛盾。）

  ═══════════════════════════════════════════════════════════════════════
  ★still-MISSING（诚实开口，no-声明膨胀）
  ═══════════════════════════════════════════════════════════════════════
  本文件**真递归组装**实装了：
    (1) RMove::Compose 组装-取回对偶（`descend_composeMove` rfl，组装的 subs 可 descend 真取回）；
    (2) 回拉不创新低/新高几何（`NoNewLow`/`NoNewHigh`/`RetraceNoBreak`，对真实次级别走势对象算）；
    (3) 第二类走势结构 canonical 定义（`SecondTypeStructure`：第一类离开 m1 + 回拉 m2 不创极值
        + i1<i2 时间序 + 由 descend 取回的递归组装来源）；
    (4) 买卖点定律一连接（`secondTypeStructure_imp_subBrokenCenter`：第二类结构 ⟹ 次级别破中枢）；
    (5) 级别递减终止（`secondType_m1_level_decreases` + `segment_no_secondType` 递归底）；
    (6) 反退化见证（`witness_secondTypeStructure`：具体第二类结构真跑通 + 回拉创新低非第二类非平凡）。
  本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **背驰力度计算**（still-MISSING-C，承接 SubLevelDescent / Divergence）：`SubLevelType1` 的
      `divPair` 靠外部参数。故第二类结构的第一类离开走势「真有背驰」的**充分判定**未闭合（力度
      需 MACD 引擎，L2）。本文件证「第二类结构（含 SubLevelType1）⟹ 次级别破中枢」（几何必要），
      未证「破中枢 + X ⟹ 完整第二类」的可计算充分性。
    · **B2/S2 信号提取层**（依赖迁主塔中枢区间口径，留下一轮）：从本级别 parent 自动提取第二类
      端点 `BspEndpoint`（side/center/retracePrice/...）+ 注入 `signal.rs` 多声部——本文件产走势
      结构 `SecondTypeStructure`，未产提取适配器。提取层精确依赖：①迁主塔修正后中枢核心区间口径
      （`Move.interval`/`Center` 核心 [zd,zg] 定稿）②本文件 `SecondTypeStructure` + `secondPointPrice`。
    · **中枢分配 `centerOf`**：次级别每走势配其相关中枢（centersOf GG-DD 自动）未在本文件实装——
      `centerOf` 设为参数（次级别中枢由上游 centersOf 提供，CenterStates still-MISSING-B）。
    · **回拉走势的「第一次」约束**：本文件回拉 m2 只约束「m1 之后 i1<i2 + 不创新低」，未约束
      「第一次回拉」（第三类用 firstRetrace，第二类 §10.1 是「再次下跌的那个次级别走势」——
      本文件用 i1<i2 取 m1 后某回拉，未强制是紧邻第一个回拉。这是结构层的弱化，提取层可加
      「i2 = m1 后第一个反向走势」强化，留提取层）。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：第二类走势递归组装层 canonical 形式化——RMove::Compose 组装-取回对偶
     （`descend_composeMove`）+ 第二类走势结构 `SecondTypeStructure`（第一类离开走势 m1 +
     回拉走势 m2 不创新低/新高 + i1<i2 时间序 + descend 取回的递归组装来源）+ 买卖点定律一连接
     （`secondTypeStructure_imp_subBrokenCenter`）+ 级别递减终止（`segment_no_secondType` 递归底）
     + 反退化见证（具体第二类结构真跑通 + 创新低非第二类非平凡），全 L0 零 sorry。
  2. 定义依据：第14课买点定律一（第二类由次级别第一类构成）+ 第15课（未创新低）+ §10.1（次级别
     再次下跌走势的结束点）+ §五走势递归（高级别走势由次级别递归组装）+ 走势分解定理二（#89
     parent 由次级别走势 compose）。输入特征：parent = RMove::Compose 组装 ⟹ descend 取回 subs
     （满足「递归组装来源」）；subs 内 m1 破中枢+背驰 ⟹ 第一类离开（满足「由次级别第一类构成」）；
     m2.lo ≥ m1.lo（买）/ m2.hi ≤ m1.hi（卖）⟹ 不创新低/新高（满足「未创新低」）；i1<i2 ⟹ 回拉
     在第一类后（满足「第一类后再次下跌」时间序）。
  3. 边界条件（结论翻转）：
     · 若改「不创新低/新高」判据的闭/开区间口径（`m1.lo ≤ m2.lo` 含等号 vs `m1.lo < m2.lo`
       严格），临界回拉（m2.lo = m1.lo，恰平第一类低点）归属翻转。当前用含等号（`NoNewLow :=
       m1.lo ≤ m2.lo`，第15课「未创新低」含「恰平不创」）——若 reference 改严格「必须高于」则
       临界重裁（与 BspClassification 第三类 zg/zd 口径裁决同精神，见该谱系）。
     · 若 still-MISSING-C 补上 MACD 力度引擎后，`divPair` 由真实次级别力度填充——若某标的次级别
       第一类离开走势「破中枢但不背驰」，则非第一类 ⟹ 该 parent 无第二类走势结构（几何必要非
       充分在 L2 数据上被检验）。这是**否定性结果的入口**（formalization-validity-domain L2 可否证）。
     · 若 §10.1「相应走势」被定为「紧邻第一个回拉」（强 firstRetrace 约束），则本文件「i1<i2 取
       m1 后任一回拉」需收紧为「i2 = m1 后第一个反向走势」——临界多回拉场景归属翻转，留提取层。
     · 若 descend 的递归底改变（线段也可下钻），`segment_no_secondType` 终止翻转须重证——当前底
       在线段 level 0（`descend_segment` 得空），明确。
  4. 下游推论：
     · 第二类走势结构成立 ⟹ B2/S2 信号提取层（留下一轮）可消费 `SecondTypeStructure` +
       `secondPointPrice` 产第二类端点，注入 signal.rs 多声部（消解 #5 单声部根因「第二类结构
       不可产」——结构层已产，提取层待迁主塔口径定稿）。
     · 第二类走势结构 ⟹ 买卖点定律一前件（次级别有破中枢走势，`secondTypeStructure_imp_subBrokenCenter`）
       ⟹ 策略组件的第二类应对可依赖「第二类必有次级别第一类离开 + 回拉不创新低前件」（真递归
       组装见证，非 IsType2 本级别判据的名义 afterTypeOne）。
     · 卡点定位在 MACD 力度引擎（still-MISSING-C）+ 信号提取层（依赖迁主塔口径）⟹ 形式化主线
       下一缺口明确指向 L2 数值引擎层 + B2/S2 提取适配器，不是 L0 结构定义层。
  5. 谱系引用：第二类「由次级别第一类构成」在 BspClassification.IsType2 是本级别判据层（标
     still-MISSING-D「不证递归构造」）；SubLevelDescent 用 descend 真下钻证「第二类 ⟸ 次级别破
     中枢」几何必要条件（still-MISSING-D′ 几何部分）；本文件用 RMove::Compose 组装-取回对偶 +
     第二类走势结构（第一类离开 + 回拉不创新低）补全「第二类走势结构层」（连接 IsType2 判据层与
     SubLevelDescent 次级别构成层，消解 #5 的结构层缺口）。力度卡点承接 Divergence.lean
     still-MISSING-C；中枢分配承接 CenterStates still-MISSING-B；信号提取层依赖迁主塔中枢区间
     口径（本轮并行迁主塔工位 637 修正中枢核心区间）。无新概念分离——RMove::Compose（composeStep）
     + descend 是已结算组装/逆操作，第二类走势结构是其上的结构刻画，不引入新定义冲突。
  6. 影响声明：新增 `Origin.RMoveCompose` 模块，import SubLevelDescent + BspClassification +
     Formal.RecursiveConstruction（只读，不改 committed 类型）。无反向依赖，无命名冲突（namespace
     `NewChanlun.Origin.RMoveCompose`）。**不碰** signal.rs（B2/S2 提取层留下轮）/center.rs/
     voice.rs/risk.rs/exec.rs/closed_loop。
     ★待 Lead 登记 root：`Origin.RMoveCompose`（lakefile Origin lib roots 追加）。
     不编辑 lakefile（报 Lead 登记）。`lake env lean Origin/RMoveCompose.lean` 单文件验证。
-/

end NewChanlun.Origin.RMoveCompose
