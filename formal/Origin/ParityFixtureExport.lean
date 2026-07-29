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
    ])
  ]

end NewChanlun.Origin.ParityFixtureExport

-- ★机器导出入口：#eval 用 IO.println 输出**裸** fixture JSON（单行 compress，非 String 字面量），
-- `lake env lean Origin/ParityFixtureExport.lean` 抓 stdout 重定向落盘为 fixture 文件。
-- 注：doc-comment `/-- -/` 必须紧跟声明，不能跟 #eval command，故此处用普通行注释。
#eval IO.println NewChanlun.Origin.ParityFixtureExport.fixtureJson.compress
