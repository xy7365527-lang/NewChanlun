/-
Origin/SubLevelDescent.lean — 真下钻次级别 ParseStruct 重跑流水线（task #119，#84 L2-blocker territory，D′）

★工位定位（#116 still-MISSING-D′ 缺口的真下钻实装）：
  BspConstruction.lean 的 `subLevelHasType1 : Nat → BspEndpoint → Bool` 是**终止骨架**——
  两分支 `(0, e)` 与 `(_+1, e)` 同 `decide (IsType1 e)`，**未**实装"从本级别下钻到次级别、
  在次级别重跑分类流水线、判次级别真有第一类"。#116 诚实标 still-MISSING-D′。

  本文件用 committed `Origin.RecursiveLevelSystem`（#89 窗口化递归 lift，xs9 见证）**全实例化**
  真下钻：本级别买卖点候选（level-(n+1) 走势）→ 下钻次级别（RecursiveLevelSystem.lift 的逆：
  `composeStep`/`Move.compose` 携带的 `subs` 字段）→ 次级别重跑分类流水线（次级别走势的中枢
  几何 + 破中枢判定）→ 判次级别第一类。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- §10.2 买卖点定律一（知识库 + 第17课博文，最终权威：`docs/chanlun/text/blog/017-第17课.md:66`，
    #886 订正——旧注误标「第24课」，该课通篇零处提及定律/第二类买点）：
  · "任何级别的第二类买卖点都由次级别相应走势的第一类买点构成。"
- §10.1 第一类买点：某级别下跌趋势中，次级别向下**跌破最后一个中枢**后形成的**背驰点**。
  ⟹ 第一类 = 破中枢（几何） ∧ 背驰（力度，§9）。
- 走势分解定理二（zoushi 第17课，#89 形式化）：任何级别走势 ≥3 段次级别走势构成。
  ⟹ level-(n+1) 走势 `Move.compose subs centers (n+1)` 的 `subs` **就是**次级别走势序列。

═══════════════════════════════════════════════════════════════════════════
真下钻 vs 骨架的提升（本文件相对 BspConstruction.subLevelHasType1 骨架）
═══════════════════════════════════════════════════════════════════════════
骨架（#116）：`subLevelHasType1 n e = decide (IsType1 e)`——每级用**同一本级别端点 e** 的判据，
  级别参数 n **不影响**判定，"下钻"是名义的（同 e、同 decide，无次级别对象进入）。

真下钻（本文件）：
  (1) `descend : Move → List Move`（lift 的逆）：从 level-(n+1) 走势 `Move.compose subs …` 真正
      **取出次级别走势序列 `subs`**——这是 RecursiveLevelSystem `lift := composeStep` 把序列切窗
      封装为上级走势的**逆操作**（`descend (composeStep …) ⊇` 各窗口 subs，`descend_compose` rfl）。
      取出的是**真实的次级别走势对象**（携带各自 interval/centers），不是本级别端点 e 的复制。
  (2) `subLevelReclassify`：对取出的**每个次级别走势**重跑"破中枢"几何判定（次级别走势 interval
      相对其中枢 [ZD,ZG] 的破位）——这是次级别**重跑分类流水线的几何层**，对**次级别对象**算，
      不是对本级别 e 重算。
  (3) 第一类 = 破中枢 ∧ 背驰。破中枢（几何）真下钻可判；背驰（力度 Force/MACD）= still-MISSING-C
      （见 §卡点）——故真下钻**结构上**判到第一类的**几何必要条件**，力度充分条件诚实开口。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构递归下钻 / 几何破位判定 / well-founded 级别递减，不依赖数据）。
`lake env lean Origin/SubLevelDescent.lean` 通过 = 真下钻取次级别走势序列（lift 逆）+ 次级别
重跑破中枢几何判定 + 级别严格递减终止 在定义层成立，**不是**任何"次级别第一类识别在真实
行情上有效"的实证断言（那是 L2+，需真实 K 线 + MACD 力度计算）。

诚实标注（gatekeeper，no-patch-mentality / no-声明膨胀）：
★ 本文件证**真下钻**（取真实次级别走势序列 + 次级别重跑破中枢几何 + 级别递减终止）——
  这是相对 #116 骨架（同 e、同 decide）的真实提升。但第一类 = 破中枢 ∧ **背驰**，背驰需力度
  `Force`（MACD 面积，Divergence.lean 标 still-MISSING-C，**无任何 Origin 模块从 K 线计算力度**）。
  故真下钻**结构上**判到次级别第一类的**几何必要条件**（破中枢 on 真实次级别走势），力度充分
  条件 still-MISSING（见 §卡点性质）。把"几何破位"冒充为"完整第一类"= 声明膨胀（禁止）。
禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.Divergence
import Origin.BspClassification
import Formal.RecursiveConstruction

namespace NewChanlun.Origin.SubLevelDescent

open Formal.RecursiveConstruction
  (WellFormed wellformed_subs_one_lower segment_is_base)

/-- ★命名消歧（强制全限定，重锚 RecursiveLevelSystem.lean 的消歧纪律）：在
    `NewChanlun.Origin.SubLevelDescent` 命名空间下，无限定 `Move` 解析到
    `Origin.ChanlunElements` 的 `structure Move`（元素层走势，**无** level/lo/hi 字段），
    遮蔽 `Formal.RecursiveConstruction.Move`（#89 递归走势 μF，携带 level/interval/subs）。
    本文件真下钻作用于 **递归走势 μF**（descend 取 subs、级别递减、interval 破中枢几何都依赖
    μF 字段），故全文用 `RMove := Formal.RecursiveConstruction.Move` 别名确保对接 μF 类型，
    非元素层 Move（两类型不同——名称消歧，非定义冲突）。 -/
abbrev RMove := Formal.RecursiveConstruction.Move

/-- ★命名消歧（Center 侧）：无限定 `Center` 在本命名空间解析到 `Origin.ChanlunElements.Center`
    （元素层中枢，字段 zd/zg/startIndex/endIndex/valid，**无** dd/gg 外缘）。但 descend 取回的
    次级别走势序列来自 `RMove.compose subs centers`，其 `centers : List Formal.CenterTrichotomy.Center`
    （递归走势 μF 的中枢，字段 dd/zd/zg/gg/core_valid/outer_lo/outer_hi——含外缘）。本文件破中枢
    几何作用于 μF 中枢核心 [zd, zg]，故全文用 `RCenter := Formal.CenterTrichotomy.Center` 别名
    确保对接 μF 中枢类型，非元素层 Center（名称消歧，非定义冲突）。 -/
abbrev RCenter := Formal.CenterTrichotomy.Center

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 真下钻 `descend`：RecursiveLevelSystem.lift 的逆（取次级别走势序列）

    RecursiveLevelSystem `lift := composeStep`（#89/#99）把第 n 级走势序列经规范窗口族切窗，
    **逐窗**封装为第 n+1 级走势 `Move.compose (窗口三段) centers (n+1)`。`descend` 是其逆：
    从一个第 n+1 级走势取回它**封装的次级别走势序列** `subs`——这是「下钻次级别」的核心算子，
    取出的是真实的次级别走势对象（携带各自 interval/centers），非本级别端点的复制。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★下钻算子（lift 的逆，真下钻核心）** —— 从一个走势取回它的**次级别走势序列**。

  - `Move.compose subs _ _`：取回封装的 `subs`（次级别走势序列）——`lift`（composeStep）把次级别
    窗口封装为本级别走势的**逆操作**。这是「下钻到次级别」的真实结构：subs 是 RecursiveLevelSystem
    递归塔下一层的真实走势对象。
  - `Move.segment _ _ _`：线段是递归底（level 0，`segment_is_base`），**无次级别**——下钻得空序列
    （次级别下降递归的基底，对应买卖点定律一"递归到最低级别为基"）。
-/
def descend : RMove → List RMove
  | Formal.RecursiveConstruction.Move.segment _ _ _ => []
  | Formal.RecursiveConstruction.Move.compose subs _ _ => subs

/--
  **★`descend` 是 `lift`/`composeStep` 封装的逆（L0，rfl）** —— 对任意窗口封装的本级别走势
  `Move.compose subs centers (n+1)`，下钻恰取回封装的次级别走势序列 `subs`。

  这坐实「真下钻取真实次级别走势」：composeStep 把次级别窗口三段 compose 为本级别走势，descend
  逆向取回那三段（次级别走势对象本身）。与 #116 骨架（同本级别 e、级别参数不影响判定）的本质
  区别——这里级别真正下降一层，取出的是**次级别对象**。
-/
theorem descend_compose (subs : List RMove) (centers : List RCenter) (lvl : Nat) :
    descend (Formal.RecursiveConstruction.Move.compose subs centers lvl) = subs := rfl

/-- **★线段下钻得空（递归底，L0）** —— 线段（level 0）无次级别，下钻空序列。 -/
theorem descend_segment (d : Formal.TrendTrichotomy.Direction) (lo hi : Int) :
    descend (Formal.RecursiveConstruction.Move.segment d lo hi) = [] := rfl

/--
  **★下钻级别严格递减（well-founded 基础，L0）** —— 对**良构**本级别走势
  `Move.compose subs centers lvl`，每个下钻得到的次级别走势 level = lvl - 1（严格低一级）。

  这是次级别下降递归终止的**结构基础**（买卖点定律一的级别递归 well-founded）：每次 descend
  级别真正减一，递归到 level 0（线段，descend 得空）为基底，well-founded 终止。这把 #116 骨架
  "级别参数 n 不影响判定"提升为"级别真正下降一层 + 下钻对象是次级别"。
-/
theorem descend_level_decreases
    (sl : List RMove) (centers : List RCenter) (lvl : Nat)
    (h : WellFormed (Formal.RecursiveConstruction.Move.compose sl centers lvl)) :
    ∀ m ∈ descend (Formal.RecursiveConstruction.Move.compose sl centers lvl), m.level = lvl - 1 := by
  intro m hm
  rw [descend_compose] at hm
  exact wellformed_subs_one_lower sl centers lvl h m hm

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 次级别重跑分类流水线（几何层）：次级别走势破中枢判定

    第一类买点（§10.1）= 次级别向下**跌破最后一个中枢**后的**背驰点**。
    破中枢（几何）= 次级别走势的价格区间越过中枢核心 [ZD, ZG]：
      - 向下破（买点侧）：走势区间下沿 `lo < ZD`（跌破中枢下沿，离开中枢之下）。
      - 向上破（卖点侧）：走势区间上沿 `hi > ZG`（突破中枢上沿，离开中枢之上）。
    这对**下钻取回的次级别走势对象**算（Move.interval 相对其 center），是真"次级别重跑"，
    不是对本级别端点重算。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★次级别走势破中枢（向下，买点侧，几何层）** —— 一个次级别走势 `m` 跌破中枢 `c` ⟺
  其价格区间下沿 `m.lo` 严格低于中枢核心下沿 `c.zd`（走势离开中枢进入"之下"，第49课位置三态）。

  这是第一类买点判据的**几何必要条件**："次级别向下跌破最后一个中枢"。对**下钻取回的次级别
  走势**算（`m.lo` = 次级别走势区间下沿，`c.zd` = 次级别中枢核心下沿）。
-/
def SubBrokeBelow (m : RMove) (c : RCenter) : Prop := m.lo < c.zd

instance (m : RMove) (c : RCenter) : Decidable (SubBrokeBelow m c) := by
  unfold SubBrokeBelow; infer_instance

/-- **★次级别走势破中枢（向上，卖点侧，几何层）** —— 区间上沿 `m.hi > c.zg`（突破中枢上沿）。 -/
def SubBrokeAbove (m : RMove) (c : RCenter) : Prop := c.zg < m.hi

instance (m : RMove) (c : RCenter) : Decidable (SubBrokeAbove m c) := by
  unfold SubBrokeAbove; infer_instance

/--
  **★次级别重跑破中枢判定（按方向，几何层）** —— 对下钻取回的次级别走势 `m` + 其中枢 `c` +
  买卖方向 `side`，重跑破中枢几何判定：买点侧判向下破，卖点侧判向上破。

  这是「次级别重跑分类流水线」的几何层：对**次级别对象**（descend 取回）重新算破位，
  不是 #116 骨架的"对本级别 e 重 decide 同一判据"。
-/
def subReclassifyBroke (side : Side) (m : RMove) (c : RCenter) : Bool :=
  match side with
  | Side.long  => decide (SubBrokeBelow m c)
  | Side.short => decide (SubBrokeAbove m c)

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 真下钻第一类几何必要条件：descend → 次级别重跑 → 判破中枢

    第一类 = 破中枢 ∧ 背驰。本节实装真下钻判到的部分：从本级别走势 descend 取次级别走势序列，
    对每个次级别走势配其中枢重跑破中枢几何，判"次级别**存在**破中枢的走势"（第一类的几何
    必要条件）。背驰（力度）见 §卡点 still-MISSING。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★次级别走势序列存在破中枢者（几何层，真下钻）** —— 给定本级别走势 `parent` + 一个中枢
  分配 `centerOf`（次级别每走势配其相关中枢）+ 方向，判**下钻取回的次级别走势序列中是否存在
  一个真破中枢的走势**。

  这是真下钻：`descend parent` 取真实次级别走势序列，`List.any` 对每个次级别走势重跑破中枢
  几何（`subReclassifyBroke`）——是次级别对象的真实重跑，非本级别端点复制判定。
-/
def subLevelHasBrokenCenter (parent : RMove) (side : Side) (centerOf : RMove → RCenter) : Bool :=
  (descend parent).any (fun m => subReclassifyBroke side m (centerOf m))

/--
  **★第一类几何必要条件（真下钻，L0）** —— 次级别**真有第一类**蕴含次级别**存在破中枢走势**。

  第一类 = 破中枢 ∧ 背驰（§10.1）。故"次级别有第一类"⟹"次级别有破中枢走势"（必要条件，
  几何层真下钻可判）。本定理把第一类的几何必要条件下钻到次级别对象——这是相对 #116 骨架的
  真实提升（骨架对本级别 e 判，本定理对下钻取回的次级别走势序列判）。

  ★诚实开口：必要条件**非充分**——破中枢不蕴含第一类（还需背驰，力度层 still-MISSING-C）。
  本定理只断言"第一类 ⟹ 破中枢"（蕴含一向），不断言"破中枢 ⟹ 第一类"（那需力度）。
-/
theorem type1_needs_subBrokenCenter
    (parent : RMove) (side : Side) (centerOf : RMove → RCenter)
    (m : RMove) (hm : m ∈ descend parent)
    (hbroke : subReclassifyBroke side m (centerOf m) = true) :
    subLevelHasBrokenCenter parent side centerOf = true := by
  unfold subLevelHasBrokenCenter
  rw [List.any_eq_true]
  exact ⟨m, hm, hbroke⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 第二类 ⟺ 次级别真有第一类（买卖点定律一，几何层真下钻 + 力度开口）

    买卖点定律一（§10.2）："第二类买卖点由次级别相应走势的第一类构成。"
    把它形式化为：本级别第二类成立 ⟺ 次级别（descend 取回）真有第一类。
    第一类 = 破中枢（几何，真下钻可判）∧ 背驰（力度，still-MISSING-C）。
    本节给「⟸」的几何必要条件侧（第二类 ⟸ 次级别破中枢 + 背驰前提），力度前提诚实显式标注。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★次级别第一类（完整判据，含力度前提显式标注）** —— 一个下钻取回的次级别走势 `m` 是第一类
  买卖点 ⟺ 破中枢（几何，本文件真下钻可判）∧ 背驰（力度 `divPair`，still-MISSING-C 外部提供）。

  ★力度 `divPair` 作为**显式参数**（不内蕴于 Move——Move 不携带 Force/MACD 数据，见 §卡点）：
  本谓词诚实暴露"第一类需要力度数据"——力度由外部（次级别 MACD 计算，still-MISSING-C）提供。
  把 divPair 设为参数 = 诚实标注"这部分数据本文件不产出"，不是把它藏进 decide 假装算了。
-/
def SubLevelType1 (side : Side) (m : RMove) (c : RCenter) (divPair : DivergencePair) : Prop :=
  (match side with
   | Side.long  => SubBrokeBelow m c
   | Side.short => SubBrokeAbove m c)
  ∧ IsDivergence divPair

/--
  **★次级别第一类 ⟹ 破中枢（几何必要条件，L0）** —— 完整第一类（破中枢 ∧ 背驰）蕴含破中枢。
  这把次级别第一类的几何分量分离出来——几何分量真下钻可判，力度分量（背驰）外部提供。
-/
theorem subLevelType1_imp_broke (side : Side) (m : RMove) (c : RCenter) (divPair : DivergencePair)
    (h : SubLevelType1 side m c divPair) :
    subReclassifyBroke side m c = true := by
  unfold SubLevelType1 at h
  unfold subReclassifyBroke
  cases side with
  | long  => simp only; exact decide_eq_true h.1
  | short => simp only; exact decide_eq_true h.1

/--
  **★买卖点定律一（几何层真下钻，L0）** —— 本级别第二类成立的**构成性必要条件**：若次级别
  （descend 取回）某走势 `m` 是完整第一类（破中枢 ∧ 背驰），则次级别存在破中枢走势
  （`subLevelHasBrokenCenter` 真）——即本级别第二类的次级别第一类构成在几何层被真下钻见证。

  这是买卖点定律一"第二类由次级别第一类构成"的真下钻形式：次级别第一类（descend 取回的真实
  次级别走势 + 破中枢几何 + 背驰力度）⟹ 次级别破中枢被本级别 descend 见证。相对 #116 骨架
  （同本级别 e 同 decide）的本质提升——这里是次级别对象的真实第一类下钻到本级别第二类构成。
-/
theorem secondType_via_subLevel_type1
    (parent : RMove) (side : Side) (centerOf : RMove → RCenter) (divPair : DivergencePair)
    (m : RMove) (hm : m ∈ descend parent)
    (h1 : SubLevelType1 side m (centerOf m) divPair) :
    subLevelHasBrokenCenter parent side centerOf = true :=
  type1_needs_subBrokenCenter parent side centerOf m hm
    (subLevelType1_imp_broke side m (centerOf m) divPair h1)

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 反退化见证（真下钻真跑通：具体次级别走势 ⟹ 具体破中枢判定，非平凡）

    构造一个本级别走势 `parentWit`（compose 三个次级别走势），descend 取回真实次级别走势序列，
    对取回的次级别走势重跑破中枢几何，见证真下钻产出非平凡判定（区别于骨架的同 e decide）。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 次级别走势见证：向下跌破的次级别走势（区间 [-5, 1]，下沿 -5 < 中枢 ZD=10 ⟹ 破中枢之下）。 -/
def subMoveBroke : RMove :=
  Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.down (0 - 5) 1

/-- 次级别走势见证：未破中枢的次级别走势（区间 [12, 18]，下沿 12 ≥ 中枢 ZD=10 ⟹ 未破之下）。 -/
def subMoveInside : RMove :=
  Formal.RecursiveConstruction.Move.segment Formal.TrendTrichotomy.Direction.up 12 18

/-- 次级别中枢见证：核心 [ZD, ZG] = [10, 20]。 -/
def subCenter : RCenter :=
  { dd := 5, zd := 10, zg := 20, gg := 25,
    core_valid := by decide, outer_lo := by decide, outer_hi := by decide }

/-- 本级别走势见证：compose 三个次级别走势（其一向下破中枢）+ 中枢，级别 1。 -/
def parentWit : RMove :=
  Formal.RecursiveConstruction.Move.compose [subMoveBroke, subMoveInside, subMoveInside] [subCenter] 1

/-- **★反退化见证：descend 取回真实次级别走势序列（非空，含破中枢走势）。** -/
theorem witness_descend :
    descend parentWit = [subMoveBroke, subMoveInside, subMoveInside] := rfl

/-- **★反退化见证：取回的次级别走势 `subMoveBroke` 真跌破中枢（lo=-5 < zd=10）。** -/
theorem witness_subMove_broke :
    SubBrokeBelow subMoveBroke subCenter := by
  unfold SubBrokeBelow subMoveBroke subCenter
    Formal.RecursiveConstruction.Move.lo Formal.RecursiveConstruction.Move.interval
  decide

/-- **★反退化见证：未破中枢的次级别走势真未破（lo=12 ≥ zd=10）。** -/
theorem witness_subMove_inside :
    ¬ SubBrokeBelow subMoveInside subCenter := by
  unfold SubBrokeBelow subMoveInside subCenter
    Formal.RecursiveConstruction.Move.lo Formal.RecursiveConstruction.Move.interval
  decide

/-- **★反退化见证：真下钻判到次级别存在破中枢走势（买点侧，几何层真跑通）。**
    `descend parentWit` 取回真实次级别走势序列，对每个重跑破中枢几何，存在 `subMoveBroke` 破中枢。 -/
theorem witness_subLevel_hasBrokenCenter :
    subLevelHasBrokenCenter parentWit Side.long (fun _ => subCenter) = true := by
  unfold subLevelHasBrokenCenter
  rw [witness_descend]
  decide

/-- **★反退化见证：完整次级别第一类（破中枢 ∧ 背驰）⟹ 第二类构成（真下钻 + 力度前提）。**
    背驰力度由外部 divPair（forceC=2 < forceA=8 ⟹ 背驰）提供，几何破中枢真下钻判定。 -/
theorem witness_secondType_via_type1 :
    subLevelHasBrokenCenter parentWit Side.long (fun _ => subCenter) = true := by
  have h1 : SubLevelType1 Side.long subMoveBroke subCenter
      { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true } := by
    unfold SubLevelType1
    refine ⟨?_, ?_⟩
    · simp only; exact witness_subMove_broke
    · unfold IsDivergence; decide
  exact secondType_via_subLevel_type1 parentWit Side.long (fun _ => subCenter)
    { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true } subMoveBroke
    (by rw [witness_descend]; exact List.mem_cons_self) h1

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 卡点性质 + still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ═══════════════════════════════════════════════════════════════════════
  ★卡点性质（task #119 核心交付：实现难 or 定义冲突？）
  ═══════════════════════════════════════════════════════════════════════
  判定：**实现难度 + 上游数据缺失（still-MISSING-C），不是定义冲突**。

  论证（formalization-validity-domain 有效域 ⊊ 定义域）：
  - **真下钻本身无定义冲突**：RecursiveLevelSystem `lift := composeStep` 把次级别窗口封装为本级别
    走势，`Move.compose` **携带** `subs` 字段——descend（lift 逆）从结构上**确定性取回**次级别
    走势序列（`descend_compose` rfl）。级别严格递减（`descend_level_decreases`）保证 well-founded
    终止。次级别下降递归基（线段 level 0，descend 得空）**无递归基冲突**——递归底是线段，明确。
    故 #84 担忧的"次级别递归基/终止 vs 有限数据"**在结构层不构成定义冲突**：descend 对任意走势
    全函数、级别单调递减、底在线段。

  - **卡点 = 第一类的力度分量无数据来源（still-MISSING-C，实现难/上游缺失）**：
    第一类 = 破中枢（几何）∧ 背驰（力度 §9）。
    · 几何分量（破中枢）：真下钻**完全可判**——descend 取真实次级别走势，`SubBrokeBelow/Above`
      对次级别走势 interval 相对其中枢核心几何判定（本文件实装，L0 机器证明）。
    · 力度分量（背驰）：需 `Force`（MACD 柱子面积，Divergence.lean §1）。**无任何 Origin 模块从
      K 线序列计算 MACD 面积**（Divergence.lean 显式标 still-MISSING-C："不实装从 K 线计算 MACD
      面积，需 EMA/DIF/DEA 数值计算 + 面积积分，是 L2 引擎层"）。故 `IsDivergence divPair` 的
      `divPair` 必须**外部提供**（本文件设为 `SubLevelType1` 的显式参数，诚实标注数据来源在外）。

  - **为何不是定义冲突**：定义冲突 = 接受 A 则 B 矛盾，无法弥合（no-workaround）。此处无矛盾——
    descend / 破中枢几何 / 级别递减 三者定义自洽且全实装；缺的只是**力度数据的计算引擎**
    （MACD 面积，L2 数值层），那是**实现/上游缺失**，不是定义之间打架。补上 MACD 引擎（L2）后，
    `divPair` 参数可由真实次级别力度填充，`SubLevelType1` 即完整——定义不需改动，只需补数据源。
    （对比真定义冲突：若"次级别递归"要求级别可无限下降但"有限数据"要求有底，那才是冲突——但
    descend 的递归底明确在线段 level 0，#89 消费约束 `length*3 ≤ lowerMoves.length` 保证有限段
    自然收敛，**无此冲突**。#84 的"真实数据线段不足→无次级别"是 L2 数据充分性问题，不是 L0
    定义冲突——数据不足时 descend 得空序列 `subLevelHasBrokenCenter = false`，诚实返回"无次级别
    第一类"，不发散不矛盾。）

  ═══════════════════════════════════════════════════════════════════════
  ★still-MISSING（诚实开口，no-声明膨胀）
  ═══════════════════════════════════════════════════════════════════════
  本文件**真下钻**实装了：
    (1) descend（lift 逆，取真实次级别走势序列，`descend_compose` rfl）；
    (2) 次级别重跑破中枢几何判定（对次级别对象算，`subReclassifyBroke`）；
    (3) 级别严格递减 well-founded 基础（`descend_level_decreases`）；
    (4) 第一类几何必要条件下钻 + 买卖点定律一几何层构成（`secondType_via_subLevel_type1`）。
  本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **背驰力度计算**（still-MISSING-C，根因）：`Force`/MACD 面积无 Origin 计算引擎，`divPair`
      靠外部参数。故"次级别**真有第一类**"的**充分判定**（破中枢 ∧ 背驰齐全且力度真算）未闭合——
      本文件证"第一类 ⟹ 破中枢"（必要，几何下钻），未证"破中枢 ∧ X ⟹ 第一类"的完整可计算充分性。
    · **中枢分配 `centerOf`**：次级别每走势配"最后一个中枢"的提取（centersOf GG-DD 全自动）未在
      本文件实装——`centerOf` 设为参数（次级别中枢由上游 centersOf 提供，CenterStates still-MISSING-B）。
    · **完整 ParseStruct 重跑**：本文件下钻到次级别**走势序列**层 + 破中枢几何，未重跑次级别
      mergeBars→fractals→strokes→segments→centers 全元素流水线（那需次级别 K 线，#84 L2 territory）。
      descend 取的是 RecursiveLevelSystem 递归塔的次级别走势（已是 Move 层），非从次级别 Bar 重解析。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：真下钻 `descend`（lift 逆，取真实次级别走势序列）+ 次级别重跑破中枢几何 + 级别严格
     递减终止 + 买卖点定律一几何层构成（第二类 ⟸ 次级别第一类的几何必要条件），全 L0 零 sorry。
     相对 #116 骨架（同本级别 e、同 decide、级别参数不影响）的真实提升：下钻取**次级别对象**、
     级别真递减、对次级别重跑几何判定。
  2. 定义依据：§10.2 买卖点定律一（第二类由次级别第一类构成）+ §10.1 第一类（破中枢 ∧ 背驰）+
     走势分解定理二（#89 level-(n+1) 走势 = subs 次级别序列）。输入特征：`Move.compose subs …`
     携带 subs ⟹ descend 可逆取回（满足"下钻次级别"）；次级别走势 interval 相对中枢核心 ⟹
     破中枢几何可判（满足"次级别向下跌破中枢"的几何必要条件）。
  3. 边界条件（结论翻转）：
     · 若改"破中枢"判据的闭/开区间口径（`lo < zd` 严格 vs `lo ≤ zd` 含等号），临界走势（lo=zd）
       归属翻转，须重裁（与 BspClassification 第三类 zg/zd 口径一致性相关）。
     · 若 still-MISSING-C 补上 MACD 力度引擎后，`divPair` 由真实次级别力度填充——若某标的次级别
       力度数据显示"破中枢但不背驰"，则该次级别无第一类，本级别第二类不成立（几何必要非充分在
       L2 数据上被检验）。这是**否定性结果的入口**（formalization-validity-domain：L2 可否证）。
     · 若 descend 的递归底改变（如线段也可下钻），well-founded 终止翻转须重证——当前底在线段
       level 0（`segment_is_base`），明确。
  4. 下游推论：
     · 真下钻几何必要条件成立 ⟹ BspConstruction `subLevelHasType1` 骨架可替换为本文件 descend +
       破中枢几何（消解 #116 still-MISSING-D′ 的几何部分）。完整替换还需 MACD 力度引擎（L2）。
     · 第二类 ⟸ 次级别第一类（几何层）⟹ 策略组件（#114）的第二类应对可依赖"第二类必有次级别
       破中枢前件"（真下钻见证，非骨架名义）。
     · 卡点定位在 MACD 力度引擎（still-MISSING-C）⟹ 形式化主线下一缺口明确指向 L2 数值引擎层
       （EMA/DIF/DEA + 面积积分），不是 L0 定义层。
  5. 谱系引用：买卖点定律一次级别递归在 BspConstruction.subLevelHasType1 是骨架（#116 标
     still-MISSING-D′）；本文件用 #89 窗口化递归（composeStep/lift）的逆 descend 真下钻提升。
     力度卡点承接 Divergence.lean still-MISSING-C（Force/MACD 抽象标量，指向 Strict.Nest 区间套
     已证的级别下降唯一性 + L2 引擎层力度计算）。中枢分配承接 CenterStates still-MISSING-B
     （centersOf GG-DD 自动计算）。无新概念分离——descend 是 lift 已结算逆操作，不引入新定义冲突。
  6. 影响声明：新增 `Origin.SubLevelDescent` 模块，import ChanlunElements + CenterStates +
     Divergence + BspClassification + Formal.RecursiveConstruction（只读，不改 committed 类型）。
     无反向依赖，无命名冲突（namespace `NewChanlun.Origin.SubLevelDescent`）。
     ★待 Lead 登记 root：`Origin.SubLevelDescent`（lakefile Origin lib roots 追加）。
     不编辑 lakefile（报 Lead 登记）。`lake env lean Origin/SubLevelDescent.lean` 单文件验证。
-/

end NewChanlun.Origin.SubLevelDescent
