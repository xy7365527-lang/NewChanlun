/-
Origin/ParityFixtureExport.lean — parity fixture 机器导出（acceptance #4，消手工转录漂移，631 兑现）

★工位定位（SG-fixture-couple，631 膨胀基线兑现）：rust 的 3 个 parity 测试
  （theta_v0_buy_parity / theta_v0_classifier_parity / theta_v0_lean_parity）此前把 Lean
  machine-checked 的值**手工转录**为 rust 常量。631 裁断：手工转录 ≠ 机器耦合——人誊写可能漂移
  （Lean 改了值但 rust 常量没跟），bit-exact 声明被削弱。

  本文件用 Lean `#eval` 把 parity 用到的全部 machine-checked 值**真求值并序列化为 JSON 字符串**
  （`Lean.Json`），落盘为 `rust/tests/fixtures/theta_v0_parity.json`。rust parity 测试改用
  `include_str!` 读同一 fixture + serde 反序列化，断言 rust 计算结果 == fixture 中 Lean 导出值。
  机器耦合：Lean 改值 ⟹ #eval 输出变 ⟹ fixture 变 ⟹ rust 测试随之变（漂移被消除）。

★机器产 vs 手填（631 铁律）：本文件**所有数值/枚举名都来自真函数求值**，非手填——
  - delta 三元组：`decisionLedgerDelta`/`sellDecisionLedgerDelta` 真求值（不是写死 (0,1,0)）。
  - recog 枚举名：`decisionToStr (recogChanlun eventType1)` 先真跑 recogChanlun 得 openRoot，
    再 match 成 "openRoot"——字符串由机器决定（recogChanlun 改了输出 ⟹ 字符串跟着变）。
  - transition ledger.A：`(chanlunTransition z0 eventType1).ledger.A` 真跑闭环转移（不是写死 base+1）。
  - 见证字段：`eventType3.bsp.retracePrice` 等直接读 Lean def 字段（Lean 改字段 ⟹ 导出值变）。
  - classifyPosition：`classifyPosition sampleCenter.core 5` 真求值（不是写死 below）。

★认识论等级（formalization-validity-domain 231号，强制标注）：本 fixture 建立的是
  **L0(Lean machine-checked 见证) ↔ L1(rust 实装) bit-exact 代数一致**。fixture 导出值 = Lean
  对具体见证逐字段的求值结果。这**不**是 L2 经验断言（无真实行情数据，无缠论盈利/实盘有效声明）。
  rust include_str! 读 fixture 后断言 rust 计算 == Lean 导出，确认的是「rust 实装忠实于 Lean
  形式化」这一管线一致性，bit-exact 标注为 **L0**。

★导出方式（机器产证据）：`lake env lean Origin/ParityFixtureExport.lean` 的 #eval 输出整个
  fixture JSON（单行 compress），重定向落盘为 rust/tests/fixtures/theta_v0_parity.json。
  fixture 非手工编写——其内容由本文件 #eval 求值唯一确定。

权威来源（Lean machine-checked，file:line）：
- 买侧 ThetaInstantiation.lean：eventType1(:358)/eventType3(:376) 见证、recogChanlun(:151)、
  decisionLedgerDelta(:256-259)、chanlunTransition(:297)、定理 eventType1_recog_openRoot(:398)/
  eventType3_recog_accreteCore(:402)/chanlunTransition_type1_allocates(:319)/type3_allocates(:331)。
- 卖侧 SellPointRecog.lean：sampleType1Sell(:356)/sampleType3Sell(:370)、recogChanlunSell(:236)；
  SellClosedLoop.lean：sellDecisionLedgerDelta(:101)、sellTransition(:126)。
- classifier CenterStates.lean：classifyPosition(:71)、sampleCenter(:388)、见证 witness_below/within/above(:403-411)。
-/
import Origin.ThetaInstantiation
import Origin.CenterStates
import Origin.BspClassification
import Origin.SellPointRecog
import Origin.SellClosedLoop
import Origin.SegmentFeatureSeq
import Origin.BuySellPredicate
import Origin.RMoveCompose
import Origin.SubLevelDescent
import Lean.Data.Json

open NewChanlun.Origin
open NewChanlun.Origin.ThetaInstantiation
open NewChanlun.Origin.SellPointRecog
open NewChanlun.Origin.SellClosedLoop
open Lean (Json)

namespace NewChanlun.Origin.ParityFixtureExport

/-- 把 (Int × Int × Int) ledger delta 三元组序列化为 JSON 数组（数值来自真求值）。 -/
def tripleJson (t : Int × Int × Int) : Json :=
  Json.arr #[Json.num t.1, Json.num t.2.1, Json.num t.2.2]

/-- 买侧 ChanlunDecision 枚举 → 字符串（match 真求值结果，非手填字符串）。 -/
def buyDecisionStr : ChanlunDecision → String
  | ChanlunDecision.openRoot    => "openRoot"
  | ChanlunDecision.accreteCore => "accreteCore"
  | ChanlunDecision.hold        => "hold"

/-- 卖侧 SellDecision 枚举 → 字符串（match 真求值结果，非手填字符串）。 -/
def sellDecisionStr : SellDecision → String
  | SellDecision.closeRoot  => "closeRoot"
  | SellDecision.reduceCore => "reduceCore"
  | SellDecision.hold       => "hold"

/-- CenterPosition 枚举 → 字符串（match 真求值结果，非手填字符串）。 -/
def positionStr : CenterPosition → String
  | CenterPosition.below  => "below"
  | CenterPosition.within => "within"
  | CenterPosition.above  => "above"

/-- 标准初始账户态（base A，Π=W=0；transition 见证用，与 rust parity 测试同口径）。 -/
def baseAccount (baseA : Int) : ChanlunAccount := { ledger := mkLedger 0 baseA 0 }

/-- #246 相切=重合裁定（2026-07-25，编排者裁定书
    `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`，ticket #248）
    单区间对的 HasGap/Overlaps 机器见证导出。真值字段来自 `decide` 机器求值（禁手填）。

    ★端点机器耦合（ticket #319，#316 影子评审 MED-1）：除两个真值外，本函数还导出用例的
    四个**输入端点**（`a.low`/`a.high`/`b.low`/`b.high`，直接读 `FeatureElem` 字段求值，
    非手填）。此前端点只存在于本导出器的 `feOf` 参数中、rust 两处（`parser/gap_overlap_fixture.rs`
    与主缝 `theta_v0_lean_parity.rs` §7）各自人工誊写一份：Lean 端点改而 rust 漏改，两侧都不会红。
    端点入 fixture 后，rust 两处改读 fixture，端点漂移沿「Lean 改端点 ⟹ fixture 变 ⟹ rust 用例输入变
    ⟹ rust 谓词结果与导出真值不符」即红。

    ★Overlaps 直化（ticket #296）：`Overlaps` 已补 `Decidable` instance
    （SegmentFeatureSeq.lean:114，与 :105 HasGap 同范式），本字段直接 `decide (Overlaps a b)`
    求值。历史（090 留痕）：#248 落地时 Overlaps 无 instance，曾绕道 `decide (¬ HasGap a b)`
    ——经已证 `gap_iff_not_overlap`（SegmentFeatureSeq.lean:118，`HasGap ↔ ¬ Overlaps`）
    严格互推，数学等价零 gap；本票是直化不是补缺，导出值不变。 -/
def gapOverlapJson (a b : FeatureElem) : Json :=
  Json.mkObj [
    ("a_low",    Json.num a.low),
    ("a_high",   Json.num a.high),
    ("b_low",    Json.num b.low),
    ("b_high",   Json.num b.high),
    ("has_gap",  Json.bool (decide (HasGap a b))),
    ("overlaps", Json.bool (decide (Overlaps a b)))
  ]

/-- gap/overlap 用例区间构造（`valid` 由 omega 直推；区间端点是用例输入，与
    `classifyPosition sampleCenter.core 5` 的 `5` 同性质——本文件是这组端点的**唯一权威源**，
    经 `gapOverlapJson` 读 `FeatureElem` 字段导出（#319），rust 侧不再誊写；
    导出字段 has_gap/overlaps 与四端点全部机器求值）。 -/
def feOf (lo hi : Int) (h : lo ≤ hi) : FeatureElem := { low := lo, high := hi, valid := h }

/-! ═══════════════════════════════════════════════════════════════════════
    § #1294 F-8 + #1289（G4 裁定 a）：BSP 谓词族向量级锁 fixture 段
    （IsType1 / IsType3Buy(firstRetrace) / SecondTypeStructure 存在性 + 见证级）

    先例 = gap/overlap 段（#248/#319）：Lean 提取值（`decide` 真求值）vs rust 输出逐位比对。
    本段把 lean_parity 复活到 BSP 谓词族：
    - `IsType1`（BspClassification :96）↔ rust `is_type1_buy`（closed_loop/buy.rs :112）；
    - `IsType3Buy`（BspClassification :117，含 `firstRetrace` 必要条件）↔ rust
      `is_type3_buy`（buy.rs :120）；
    - `SecondTypeStructure`（RMoveCompose :233，#1289 后 = 首个后继 i2=i1+1）↔ rust
      `find_second_type_structure`（rmove_compose.rs :205，首个后继 i2=i1+1）：
      · 存在性（`holds`）——`.is_some()` 与 Lean 可计算镜像 `secondTypeStructureHolds` 逐位比对；
      · 见证级（`witness`，#1289 补）——`(i1, i2=i1+1, second_point, retrace_breaks_extreme)`
        与 Lean first-match 见证镜像 `findSecondTypeStructureWitness` 逐位比对（同输入同见证点）。
    ═══════════════════════════════════════════════════════════════════════ -/

def sideStr (s : Side) : String :=
  match s with
  | Side.long => "long"
  | Side.short => "short"

def dirStr (d : Formal.TrendTrichotomy.Direction) : String :=
  match d with
  | Formal.TrendTrichotomy.Direction.up => "up"
  | Formal.TrendTrichotomy.Direction.down => "down"

/-- 元素层中枢构造（对齐 `BspClassification.sampleType1.center`；valid 证由调用方给）。 -/
def mkBspCenter (zd zg : Tick) (h : zd ≤ zg) : Center :=
  { zd := zd, zg := zg, startIndex := 0, endIndex := 3, valid := h }

/-- 力度对构造（forceA/forceC 面积 + 趋势标记）。 -/
def mkDivPair (forceA forceC : Nat) (isTrend : Bool) : DivergencePair :=
  { forceA := { area := forceA }, forceC := { area := forceC }, isTrend := isTrend }

/-- 买卖点端点构造。 -/
def mkBspEndpoint (side : Side) (center : Center) (divPair : DivergencePair)
    (brokeCenter afterTypeOne leftCenter : Bool) (retracePrice : Tick)
    (firstRetrace : Bool) : BspEndpoint :=
  { side := side, center := center, divPair := divPair, brokeCenter := brokeCenter,
    afterTypeOne := afterTypeOne, leftCenter := leftCenter, retracePrice := retracePrice,
    firstRetrace := firstRetrace }

/-- 一条 BSP 端点向量的机器导出：输入字段（rust 重建 BuyEndpoint 用）+ 谓词真值（decide 求值）。 -/
def bspEndpointJson (name : String) (e : BspEndpoint) : Json :=
  Json.mkObj [
    ("name",           Json.str name),
    ("side",           Json.str (sideStr e.side)),
    ("broke_center",   Json.bool e.brokeCenter),
    ("is_divergence",  Json.bool (decide (IsDivergence e.divPair))),
    ("after_type_one", Json.bool e.afterTypeOne),
    ("left_center",    Json.bool e.leftCenter),
    ("first_retrace",  Json.bool e.firstRetrace),
    ("retrace_price",  Json.num e.retracePrice),
    ("center_zg",      Json.num e.center.zg),
    ("is_type1",       Json.bool (decide (IsType1 e))),
    ("is_type3_buy",   Json.bool (decide (IsType3Buy e)))
  ]

/-- BSP 端点向量集（固定输入）：正例 + 三处历史分歧/边界反例。
    - type1_broke_not_divergent = `witness_type1_needs_divergence`（破中枢但力度反超 ⟹ 非一类）；
    - type3_buy_not_first_retrace = §10.3 第三次类必须第一次回抽（firstRetrace 反例）；
    - type3_buy_reenter_zg = `type3Buy_rejects_reenter`（回抽回到 ZG ⟹ 非三类买点）。 -/
def bspEndpointVectors : List (String × BspEndpoint) := [
  ("type1_broke_divergent",
    mkBspEndpoint Side.long (mkBspCenter 10 20 (by decide)) (mkDivPair 8 2 true)
      true false false 5 false),
  ("type1_broke_not_divergent",
    mkBspEndpoint Side.long (mkBspCenter 10 20 (by decide)) (mkDivPair 2 8 true)
      true false false 5 false),
  ("type3_buy_first_retrace",
    mkBspEndpoint Side.long (mkBspCenter 10 20 (by decide)) (mkDivPair 3 3 false)
      false false true 25 true),
  ("type3_buy_not_first_retrace",
    mkBspEndpoint Side.long (mkBspCenter 10 20 (by decide)) (mkDivPair 3 3 false)
      false false true 25 false),
  ("type3_buy_reenter_zg",
    mkBspEndpoint Side.long (mkBspCenter 10 20 (by decide)) (mkDivPair 3 3 false)
      false false true 20 false),
  ("type3_sell_side_not_buy",
    mkBspEndpoint Side.short (mkBspCenter 10 20 (by decide)) (mkDivPair 3 3 false)
      false false true 25 true)
]

/-- 递归走势 μF 中枢构造（RMoveCompose 的 RCenter；证由调用方给）。 -/
def mkRCenter (dd zd zg gg : Int)
    (hcv : zd < zg) (hol : dd ≤ zd) (hoh : zg ≤ gg) : SubLevelDescent.RCenter :=
  { dd := dd, zd := zd, zg := zg, gg := gg, core_valid := hcv, outer_lo := hol, outer_hi := hoh }

/-- 次级别线段构造（RMove.segment，递归底 level 0）。 -/
def mkSeg (dir : Formal.TrendTrichotomy.Direction) (lo hi : Int) :
    SubLevelDescent.RMove :=
  Formal.RecursiveConstruction.Move.segment dir lo hi

/-- 单条次级别线段 → JSON（方向/lo/hi；rust 重建 RMove::Segment 用）。 -/
def segLegJson (m : SubLevelDescent.RMove) : Json :=
  match m with
  | Formal.RecursiveConstruction.Move.segment d lo hi =>
      Json.mkObj [
        ("direction", Json.str (dirStr d)),
        ("lo",        Json.num lo),
        ("hi",        Json.num hi)
      ]
  | Formal.RecursiveConstruction.Move.compose _ _ _ => Json.null

/-- `SubLevelType1`（破中枢 ∧ 背驰）的 Bool 判定镜像——`subReclassifyBroke` 已给几何
    分量（Bool），力度分量 `IsDivergence` 由 `decide` 求值；两分量 `&&` 恰等于
    `decide (SubLevelType1 side m c1 divPair)`（`SubLevelType1` 无 Decidable 实例，
    此处不展开 def 直判，用同形 Bool 原子）。 -/
def subLevelType1Holds (side : Side) (m : SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) : Bool :=
  SubLevelDescent.subReclassifyBroke side m c1 && decide (IsDivergence divPair)

/-- 腿级镜像等价（L0）：`subLevelType1Holds` 的 Bool 判定 ⇔ 规范 `SubLevelType1` Prop。
    这是 `secondTypeStructureHolds` 可计算判定的正确性锚点——每腿判定忠实于规范谓词；
    列表级「存在一个带后继（i2=i1+1 存在）的破中枢背驰腿」与 `SecondTypeStructure`
    （首个后继 i2=i1+1，#1289 G4 裁定 a）的等价性见 `secondTypeStructureHolds_iff`。 -/
theorem subLevelType1Holds_iff (side : Side) (m : SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) :
    subLevelType1Holds side m c1 divPair = true ↔
      SubLevelDescent.SubLevelType1 side m c1 divPair := by
  unfold subLevelType1Holds SubLevelDescent.SubLevelType1
  cases side <;> simp [SubLevelDescent.subReclassifyBroke]

/-- `SecondTypeStructure` 存在性的**可计算判定镜像**（Rust `find_second_type_structure`
    循环的同形）：在 `subs` 上找「第一个有后继（i2=i1+1 存在）且 `SubLevelType1` 的次级别
    走势」。与 `SecondTypeStructure`（首个后继 i2=i1+1，#1289 G4 裁定 a）在存在性上等价——
    `SubLevelType1` 只约束 m1，i2=i1+1 存在 ⟺ `i1 ≤ subs.length-2`。 -/
def secondTypeStructureHolds (side : Side) (subs : List SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) : Bool :=
  match subs with
  | [] => false
  | [_] => false
  | m :: rest =>
      subLevelType1Holds side m c1 divPair
        || secondTypeStructureHolds side rest c1 divPair

/-- `SecondTypeStructure` 存在性等价（机器证，L0）：`secondTypeStructureHolds` 可计算镜像
    ⇔ 规范 `SecondTypeStructure` Prop（列表级，首个后继 i2=i1+1，#1289 G4 裁定 a）。这是锁 2
    的正确性锚点——fixture 的 `holds` 字段来自本镜像，本定理证明镜像与规范谓词**同真同假**
    （非手写断言）；腿级等价由 `subLevelType1Holds_iff` 已证，本定理把腿级等价提升到列表级
    （`SubLevelType1` 只约束 m1，i2=i1+1 存在即后继存在）。 -/
theorem secondTypeStructureHolds_iff (side : Side) (subs : List SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) :
    secondTypeStructureHolds side subs c1 divPair = true ↔
      ∃ (i1 : Nat) (m1 m2 : SubLevelDescent.RMove),
        subs[i1]? = some m1 ∧ subs[i1 + 1]? = some m2 ∧
        SubLevelDescent.SubLevelType1 side m1 c1 divPair := by
  induction subs with
  | nil =>
      simp [secondTypeStructureHolds]
  | cons m rest ih =>
      cases rest with
      | nil =>
          simp only [secondTypeStructureHolds]
          constructor
          · intro h; cases h
          · intro h
            rcases h with ⟨i1, m1, m2, hg1, hg2, hty⟩
            have ⟨hb2, _⟩ := (List.getElem?_eq_some_iff).1 hg2
            simp at hb2
      | cons m2 rest2 =>
          simp only [secondTypeStructureHolds, Bool.or_eq_true, ih]
          rw [subLevelType1Holds_iff]
          constructor
          · intro h
            rcases h with hleft | hright
            · refine ⟨0, m, m2, ?_, ?_, hleft⟩
              · exact List.getElem?_cons_zero
              · simp
            · rcases hright with ⟨i1, a1, a2, hg1, hg2, hty⟩
              refine ⟨i1 + 1, a1, a2, ?_, ?_, hty⟩
              · simpa [List.getElem?_cons_succ] using hg1
              · simpa [List.getElem?_cons_succ] using hg2
          · intro h
            rcases h with ⟨i1, a1, a2, hg1, hg2, hty⟩
            cases i1 with
            | zero =>
                left
                have hm : m = a1 := by
                  have h' : some m = some a1 := by
                    simpa [List.getElem?_cons_zero] using hg1
                  exact Option.some.inj h'
                exact hm ▸ hty
            | succ i =>
                right
                refine ⟨i, a1, a2, ?_, ?_, hty⟩
                · simpa [List.getElem?_cons_succ] using hg1
                · simpa [List.getElem?_cons_succ, Nat.add_assoc, Nat.add_comm, Nat.add_left_comm] using hg2

/-- 列表级镜像 ⟺ 规范 `SecondTypeStructure`（对任意 parent，`descend parent` 即列表）。 -/
theorem secondTypeStructureHolds_iff_secondTypeStructure (side : Side)
    (parent : SubLevelDescent.RMove) (c1 : SubLevelDescent.RCenter)
    (divPair : DivergencePair) :
    secondTypeStructureHolds side (SubLevelDescent.descend parent) c1 divPair = true ↔
      RMoveCompose.SecondTypeStructure side parent c1 divPair := by
  unfold RMoveCompose.SecondTypeStructure
  exact secondTypeStructureHolds_iff side (SubLevelDescent.descend parent) c1 divPair

/-- `find_second_type_structure`（rmove_compose.rs :205）的 **first-match 见证镜像**（#1289 见证级
    锁的正确性源）：在 `subs` 上找**第一个** `SubLevelType1` 的次级别走势（回拉 = 首个后继
    i2=i1+1），返回 `(i1, m1, m2)`。末腿才 type1（无后继）⟹ none——与 rust
    `i1+1 >= len → return None` 同形停止；不找后面的腿（first-match）。 -/
def findSecondTypeStructureWitness (side : Side) (subs : List SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) :
    Option (Nat × SubLevelDescent.RMove × SubLevelDescent.RMove) :=
  match subs with
  | [] => none
  | [_] => none
  | m1 :: m2 :: rest =>
      if subLevelType1Holds side m1 c1 divPair then
        some (0, m1, m2)
      else
        match findSecondTypeStructureWitness side (m2 :: rest) c1 divPair with
        | none => none
        | some (i, a1, a2) => some (i + 1, a1, a2)

/-- 见证镜像可靠（机器证，L0，#1289）：`findSecondTypeStructureWitness` 返回的每个
    `(i1, m1, m2)` 都是规范 `SecondTypeStructure`（首个后继 i2=i1+1）的合法见证——
    `subs[i1]? = some m1 ∧ subs[i1+1]? = some m2 ∧ SubLevelType1 side m1 c1 divPair`。
    fixture 的 `witness` 字段来自本镜像，本定理证明镜像产出的见证**忠实于规范谓词**
    （非手写断言）。 -/
theorem findSecondTypeStructureWitness_spec (side : Side) (subs : List SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) :
    ∀ i1 m1 m2, findSecondTypeStructureWitness side subs c1 divPair = some (i1, m1, m2) →
      subs[i1]? = some m1 ∧ subs[i1 + 1]? = some m2 ∧
      SubLevelDescent.SubLevelType1 side m1 c1 divPair := by
  induction subs with
  | nil => intro i1 m1 m2 h; simp [findSecondTypeStructureWitness] at h
  | cons m rest ih =>
      cases rest with
      | nil => intro i1 m1 m2 h; simp [findSecondTypeStructureWitness] at h
      | cons m2 rest2 =>
          intro i1 a1 a2 h
          by_cases hty : subLevelType1Holds side m c1 divPair = true
          · -- 首腿 type1：some (0, m, m2)
            simp [findSecondTypeStructureWitness, hty] at h
            rcases h with ⟨rfl, rfl, rfl⟩
            have hty' := (subLevelType1Holds_iff side m c1 divPair).1 hty
            constructor
            · exact List.getElem?_cons_zero
            · constructor
              · simp
              · exact hty'
          · -- 首腿非 type1：递归 tail 的见证 +1 平移
            have hfalse : subLevelType1Holds side m c1 divPair = false := by
              exact Bool.eq_false_iff.2 hty
            simp [findSecondTypeStructureWitness, hfalse] at h
            cases hrec : findSecondTypeStructureWitness side (m2 :: rest2) c1 divPair with
            | none => simp only [hrec] at h; cases h
            | some w =>
                rcases w with ⟨i, w2⟩
                rcases w2 with ⟨b1, b2⟩
                simp only [hrec] at h
                have heq := Option.some.inj h
                injection heq with hi1 heq2
                injection heq2 with ha1 ha2
                rw [← hi1, ← ha1, ← ha2]
                have hspec := ih i b1 b2 hrec
                rcases hspec with ⟨hb1, hb2, hty2⟩
                constructor
                · simpa [List.getElem?_cons_succ] using hb1
                · constructor
                  · simpa [List.getElem?_cons_succ] using hb2
                  · exact hty2

/-- 一条第二类走势结构向量的机器导出：腿序列 + 中枢 + 力度 + 存在性真值 + 见证级
    （i1/i2/second_point/retrace_breaks_extreme，first-match，机器求值）。 -/
def secondTypeStructureJson (name : String) (side : Side) (subs : List SubLevelDescent.RMove)
    (c1 : SubLevelDescent.RCenter) (divPair : DivergencePair) : Json :=
  let w := findSecondTypeStructureWitness side subs c1 divPair
  let witnessJson :=
    match w with
    | none => Json.mkObj [
        ("present",               Json.bool false),
        ("i1",                    Json.num 0),
        ("i2",                    Json.num 0),
        ("second_point",          Json.num 0),
        ("retrace_breaks_extreme", Json.bool false)
      ]
    | some (i1, m1, m2) => Json.mkObj [
        ("present",                Json.bool true),
        ("i1",                     Json.num i1),
        ("i2",                     Json.num (i1 + 1 : Nat)),
        ("second_point",           Json.num (RMoveCompose.secondPointPrice side m2)),
        ("retrace_breaks_extreme", Json.bool (decide (RMoveCompose.RetraceBreaksExtreme side m1 m2)))
      ]
  Json.mkObj [
    ("name",          Json.str name),
    ("side",          Json.str (sideStr side)),
    ("center_zd",     Json.num c1.zd),
    ("center_zg",     Json.num c1.zg),
    ("center_dd",     Json.num c1.dd),
    ("center_gg",     Json.num c1.gg),
    ("is_divergence", Json.bool (decide (IsDivergence divPair))),
    ("legs",          Json.arr (subs.map segLegJson).toArray),
    ("holds",         Json.bool (secondTypeStructureHolds side subs c1 divPair)),
    ("witness",       witnessJson)
  ]

/-- 第二类走势结构向量集（固定输入）：首腿破中枢背驰（有后继 ⟹ holds，见证 i1=0/i2=1）/
    末腿才破中枢（无后继 ⟹ 不 holds）/ 全无破中枢（不 holds）/ 力度反超（几何破中枢但非背驰 ⟹
    不 holds）/ 两处 type1（首腿 + 第三腿；first-match ⟹ 见证取首腿 i1=0 非 i1=2，#1289 锁）。 -/
def secondTypeStructureVectors : List (String × Side × List SubLevelDescent.RMove ×
    SubLevelDescent.RCenter × DivergencePair) := [
  ("first_leg_type1_has_successor",
    Side.long,
    [ mkSeg Formal.TrendTrichotomy.Direction.down (-5) 1
    , mkSeg Formal.TrendTrichotomy.Direction.up 12 18
    , mkSeg Formal.TrendTrichotomy.Direction.down 14 17 ],
    mkRCenter 5 10 20 25 (by decide) (by decide) (by decide),
    mkDivPair 8 2 true),
  ("first_match_two_type1_legs",
    Side.long,
    [ mkSeg Formal.TrendTrichotomy.Direction.down (-5) 1
    , mkSeg Formal.TrendTrichotomy.Direction.up 12 18
    , mkSeg Formal.TrendTrichotomy.Direction.down (-3) 2
    , mkSeg Formal.TrendTrichotomy.Direction.up 13 16 ],
    mkRCenter 5 10 20 25 (by decide) (by decide) (by decide),
    mkDivPair 8 2 true),
  ("last_leg_type1_no_successor",
    Side.long,
    [ mkSeg Formal.TrendTrichotomy.Direction.up 12 18
    , mkSeg Formal.TrendTrichotomy.Direction.down (-5) 1 ],
    mkRCenter 5 10 20 25 (by decide) (by decide) (by decide),
    mkDivPair 8 2 true),
  ("no_leg_breaks_center",
    Side.long,
    [ mkSeg Formal.TrendTrichotomy.Direction.up 12 18
    , mkSeg Formal.TrendTrichotomy.Direction.down 14 17
    , mkSeg Formal.TrendTrichotomy.Direction.up 13 16 ],
    mkRCenter 5 10 20 25 (by decide) (by decide) (by decide),
    mkDivPair 8 2 true),
  ("first_leg_breaks_but_not_divergent",
    Side.long,
    [ mkSeg Formal.TrendTrichotomy.Direction.down (-5) 1
    , mkSeg Formal.TrendTrichotomy.Direction.up 12 18 ],
    mkRCenter 5 10 20 25 (by decide) (by decide) (by decide),
    mkDivPair 2 8 true)
]

/--
  ★完整 parity fixture（所有字段来自真函数求值，机器产，非手填）。

  fixture schema（rust serde 端镜像）：
  - meta：导出来源 + 认识论等级标注。
  - buy_ledger_delta：买侧三态 ledger delta（decisionLedgerDelta 真求值）。
  - buy_recog：买侧两见证 recog 结果（recogChanlun 真求值后 match 枚举名）。
  - buy_witness：买侧 eventType1/eventType3 见证关键字段（直接读 Lean def 字段）。
  - buy_transition：买侧闭环 transition 后 ledger.A（base=5/10，chanlunTransition 真跑）。
  - sell_ledger_delta：卖侧三态 ledger delta（sellDecisionLedgerDelta 真求值）。
  - sell_recog：卖侧两见证 recog 结果（recogChanlunSell 真求值后 match 枚举名）。
  - sell_transition：卖侧闭环 transition 后 ledger.A/Pi（base=5，sellTransition 真跑）。
  - position：classifyPosition 在 sampleCenter 三见证点的结果（真求值后 match 枚举名）。
-/
def fixtureJson : Json :=
  Json.mkObj [
    ("meta", Json.mkObj [
      ("source", Json.str "formal/Origin/ParityFixtureExport.lean #eval (machine-exported)"),
      ("epistemic_level", Json.str "L0"),
      ("note", Json.str "bit-exact Lean machine-checked 见证; 非 L2 经验断言")
    ]),
    -- 买侧 ledger delta（ThetaInstantiation.decisionLedgerDelta :256-259，真求值）
    ("buy_ledger_delta", Json.mkObj [
      ("open_root",    tripleJson (decisionLedgerDelta ChanlunDecision.openRoot)),
      ("accrete_core", tripleJson (decisionLedgerDelta ChanlunDecision.accreteCore)),
      ("hold",         tripleJson (decisionLedgerDelta ChanlunDecision.hold))
    ]),
    -- 买侧 recog 见证（recogChanlun 真跑 eventType1/eventType3，:398/:402 定理对应）
    ("buy_recog", Json.mkObj [
      ("event_type1", Json.str (buyDecisionStr (recogChanlun eventType1))),
      ("event_type3", Json.str (buyDecisionStr (recogChanlun eventType3)))
    ]),
    -- 买侧见证关键字段（直接读 Lean eventType1/eventType3 def 字段 :358-387）
    ("buy_witness", Json.mkObj [
      ("type1_broke_center",   Json.bool eventType1.bsp.brokeCenter),
      ("type1_is_divergence",  Json.bool (decide (IsDivergence eventType1.bsp.divPair))),
      ("type1_retrace_price",  Json.num eventType1.bsp.retracePrice),
      ("type1_center_zg",      Json.num eventType1.bsp.center.zg),
      ("type3_broke_center",   Json.bool eventType3.bsp.brokeCenter),
      ("type3_is_divergence",  Json.bool (decide (IsDivergence eventType3.bsp.divPair))),
      ("type3_left_center",    Json.bool eventType3.bsp.leftCenter),
      ("type3_first_retrace",  Json.bool eventType3.bsp.firstRetrace),
      ("type3_retrace_price",  Json.num eventType3.bsp.retracePrice),
      ("type3_center_zg",      Json.num eventType3.bsp.center.zg)
    ]),
    -- 买侧闭环 transition 后 ledger.A（base=5：type1→6/type3→7；base=10：type1→11/type3→12）
    ("buy_transition", Json.mkObj [
      ("base_a_5",  Json.num 5),
      ("type1_a_at_base5", Json.num (chanlunTransition (baseAccount 5) eventType1).ledger.A),
      ("type3_a_at_base5", Json.num (chanlunTransition (baseAccount 5) eventType3).ledger.A),
      ("type1_pi_at_base5", Json.num (chanlunTransition (baseAccount 5) eventType1).ledger.Pi),
      ("base_a_10", Json.num 10),
      ("type1_a_at_base10", Json.num (chanlunTransition (baseAccount 10) eventType1).ledger.A),
      ("type3_a_at_base10", Json.num (chanlunTransition (baseAccount 10) eventType3).ledger.A)
    ]),
    -- 卖侧 ledger delta（SellClosedLoop.sellDecisionLedgerDelta :101，真求值）
    ("sell_ledger_delta", Json.mkObj [
      ("close_root",  tripleJson (sellDecisionLedgerDelta SellDecision.closeRoot)),
      ("reduce_core", tripleJson (sellDecisionLedgerDelta SellDecision.reduceCore)),
      ("hold",        tripleJson (sellDecisionLedgerDelta SellDecision.hold))
    ]),
    -- 卖侧 recog 见证（recogChanlunSell 真跑 sampleType1Sell/sampleType3Sell）
    ("sell_recog", Json.mkObj [
      ("sample_type1", Json.str (sellDecisionStr (recogChanlunSell sampleType1Sell))),
      ("sample_type3", Json.str (sellDecisionStr (recogChanlunSell sampleType3Sell)))
    ]),
    -- 卖侧闭环 transition 后 ledger.A/Pi（base=5：type1→A=4/Pi=1，type3→A=3）
    ("sell_transition", Json.mkObj [
      ("base_a_5", Json.num 5),
      ("type1_a_at_base5",  Json.num (sellTransition (baseAccount 5) sampleType1Sell).ledger.A),
      ("type1_pi_at_base5", Json.num (sellTransition (baseAccount 5) sampleType1Sell).ledger.Pi),
      ("type3_a_at_base5",  Json.num (sellTransition (baseAccount 5) sampleType3Sell).ledger.A)
    ]),
    -- classifier 位置见证（classifyPosition 真跑 sampleCenter.core，CenterStates :403-411）
    ("position", Json.mkObj [
      ("at_5",  Json.str (positionStr (classifyPosition sampleCenter.core 5))),
      ("at_15", Json.str (positionStr (classifyPosition sampleCenter.core 15))),
      ("at_30", Json.str (positionStr (classifyPosition sampleCenter.core 30))),
      ("zd",    Json.num sampleCenter.core.zd),
      ("zg",    Json.num sampleCenter.core.zg)
    ]),
    -- #246 相切=重合裁定机器见证（ticket #248，gapOverlapJson 真值 decide 真求值 + 四端点
    -- 机器导出（#319，rust 两处改读 fixture 端点，端点转录漂移已消））。
    -- 用例：相切两形态（a.high==b.low / b.high==a.low）+ 严格分离（正/反向）+ 严格重叠对照。
    -- 三笔形态（max(lows)==min(highs)）不适用：Lean 形式化层无三笔重合谓词（Overlaps 为两区间版），
    -- 其三笔相切语义与两区间形态 a.high==b.low 同构，已由 tangent_a_high_eq_b_low 覆盖；
    -- rust 侧 three_stroke_overlap 的口径由 crate 内单测 three_stroke_overlap_lean_fixture_bit_exact
    -- 读本段真值锁定（ticket #312，退化三笔 (a,b,b) 降到两区间语义）；三笔**互异**相切形态在 Lean
    -- 侧仍无见证（#276 MED-1 已知缺角），rust 侧由手填单测
    -- three_stroke_overlap_distinct_triple_tangent 覆盖，非机器耦合。
    ("gap_overlap", Json.mkObj [
      ("tangent_a_high_eq_b_low", gapOverlapJson (feOf 5 10 (by omega)) (feOf 10 20 (by omega))),
      ("tangent_b_high_eq_a_low", gapOverlapJson (feOf 10 20 (by omega)) (feOf 5 10 (by omega))),
      ("strict_disjoint",         gapOverlapJson (feOf 5 10 (by omega)) (feOf 11 20 (by omega))),
      ("strict_disjoint_rev",     gapOverlapJson (feOf 11 20 (by omega)) (feOf 5 10 (by omega))),
      ("strict_overlap",          gapOverlapJson (feOf 5 12 (by omega)) (feOf 8 20 (by omega)))
    ]),
    -- #1294 F-8：BSP 谓词族向量级锁（IsType1 / IsType3Buy(firstRetrace) /
    -- SecondTypeStructure）——固定输入向量集，Lean `decide` 真求值 vs rust 输出逐位比对
    -- （先例：上方 gap/overlap 段）。
    ("bsp_predicates", Json.mkObj [
      ("endpoints", Json.arr (bspEndpointVectors.map (fun (name, e) => bspEndpointJson name e)).toArray),
      ("second_type_structures", Json.arr
        (secondTypeStructureVectors.map (fun (name, side, subs, c1, dp) =>
          secondTypeStructureJson name side subs c1 dp)).toArray)
    ])
  ]

end NewChanlun.Origin.ParityFixtureExport

-- ★机器导出入口：#eval 用 IO.println 输出**裸** fixture JSON（单行 compress，非 String 字面量），
-- `lake env lean Origin/ParityFixtureExport.lean` 抓 stdout 重定向落盘为 fixture 文件。
-- 注：doc-comment `/-- -/` 必须紧跟声明，不能跟 #eval command，故此处用普通行注释。
#eval IO.println NewChanlun.Origin.ParityFixtureExport.fixtureJson.compress
