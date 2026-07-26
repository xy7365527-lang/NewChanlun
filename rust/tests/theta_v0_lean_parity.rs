//! Lean ↔ Rust golden vector 交叉验证（acceptance #4）——`Origin.ThetaInstantiation` 闭环 +
//! `Origin.SellPointRecog`/`Origin.SellClosedLoop` 卖侧对偶的 **bit-exact 一致性证据**。
//!
//! ## 工位定位（SG-1，消 090 声明膨胀活跃点）
//!
//! `rust/src/theta_v0/closed_loop/sell.rs` 与 `transition.rs` 全程声称「port Lean」「bit-exact 对齐
//! Lean」「镜像 decisionLedgerDelta」，但这些断言此前全部在 **rust 自身手写的 golden** 上验证
//! （`sample_type1_sell()` 等是 rust 内部 fixture，不是从 Lean 见证逐字段提取）——即 rust 与 rust
//! 自洽，**零 Lean↔Rust 交叉验证**。本文件把 **Lean 侧已机器验证的具体见证**（含具体数值 + 已证
//! 定理结论）逐字段编码为 golden，跑 rust 实现，断言逐字段相等，使「bit-exact 对齐 Lean」从声明
//! 变为可执行证据。
//!
//! ## golden 来源（Lean 具体见证，file:line）
//!
//! 买侧（`formal/Origin/ThetaInstantiation.lean`，Lean 已 machine-check）：
//! - `eventType1`（:358-369）：`brokeCenter=true`，`divPair={forceA:=8, forceC:=2, isTrend:=true}`
//!   ⟹ `IsDivergence`（forceC<forceA：2<8）⟹ `eventType1_recog_openRoot`（:398）= `openRoot`。
//! - `eventType3`（:376-387）：`side=long`，`leftCenter=true`，`firstRetrace=true`，
//!   `center.zg=20`，`retracePrice=25`（20<25 ⟹ 不破 ZG）⟹ `eventType3_recog_accreteCore`（:402）
//!   = `accreteCore`。
//! - `decisionLedgerDelta`（:256-259）：openRoot⟹(0,1,0)，accreteCore⟹(0,2,0)，hold⟹(0,0,0)。
//! - `chanlunTransition_type1_allocates`（:319）：第一类 ⟹ ledger.A = base+1。
//! - `chanlunTransition_type3_allocates`（:331）：第三类 ⟹ ledger.A = base+2。
//! - `chanlun_transition_distinguishes_classes`（:420）：第一类 A=base+1 ≠ 第三类 A=base+2。
//!
//! 卖侧（`formal/Origin/SellPointRecog.lean` #120 + `SellClosedLoop.lean` #121，Lean 已 machine-check，
//! 经 rust 模块头 sell.rs:11-21 语义对应表锚定）：
//! - 第一类卖点 `sampleType1Sell`：`side=short`，破中枢，背驰 ⟹ recog=`closeRoot`，delta=(1,-1,0)。
//! - 第三类卖点 `sampleType3Sell`：`side=short`，离开中枢，第一次回升，`retrace<zd` ⟹ recog=`reduceCore`，delta=(0,-2,0)。
//! - `buy_sell_A_delta_mirror`：买 openRoot A+1 ↔ 卖 closeRoot A-1；买 accreteCore A+2 ↔ 卖 reduceCore A-2。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 本文件建立 **L0(Lean machine-checked 见证) ↔ L1(rust 实装) 对齐**。`cargo test` 绿 = rust
//! theta_v0 卖侧实现对 Lean 卖侧具体见证逐字段产出 bit-exact 相同的分类 + ledger delta；买侧 Lean
//! 见证经买↔卖 A 分量对偶（Lean `buy_sell_A_delta_mirror` 已证）间接交叉验证。这**不**提升经验等级
//! （不是缠论盈利/实盘有效声明 L2/L3）——它确认的是「rust 实装忠实于 Lean 形式化」这一管线一致性。
//!
//! ## 诚实边界（no声明膨胀，no-workaround）— 买侧 ThetaInstantiation 链的 rust 实现缺口
//!
//! ★**买侧 `Origin.ThetaInstantiation.recogChanlun` / `decisionLedgerDelta` / `chanlunTransition`
//!   在 rust 里没有独立实现**。grep `recogChanlun|ChanlunDecision|decisionLedgerDelta|chanlunTransition`
//!   于 `rust/src/` 仅命中卖侧对偶（sell.rs）+ 注释引用，无买侧函数。买侧 delta 在 sell.rs:406-407 是
//!   **硬编码常量** `open_root_da: i64 = 1`（不是调 rust 函数）。
//!
//! 因此本文件对买侧的「bit-exact」是 **经卖侧对偶间接验证**（Lean `buy_sell_A_delta_mirror` 已证
//! 买卖 A 分量严格相反号 ⟹ 验证卖侧 rust = Lean 卖侧见证 + 卖侧 = -买侧 Lean 见证 ⟹ 买侧 Lean 见证
//! 经对偶被覆盖）。要让买侧获得 **直接** bit-exact 交叉验证（rust 调买侧 `recog_chanlun_buy` 函数与
//! Lean `recogChanlun` 逐字段比），需先在 rust 实装买侧 ThetaInstantiation 链（当前缺口）——这是
//! #4 完全闭合的剩余工作，本文件诚实标注，不冒充已闭合。

use newchan_rust::theta_v0::closed_loop::sell::{
    is_type1_sell, is_type3_sell, recog_chanlun_sell, sell_decision_ledger_delta, sell_transition,
    SellDecision, SellEndpoint,
};
use newchan_rust::theta_v0::closed_loop::state::AssemblyState;
use newchan_rust::theta_v0::parser::segment::Interval;
use newchan_rust::theta_v0::strategy::ledger::LedgerComp;
use newchan_rust::theta_v0::types::Side;
use serde::Deserialize;

// ════════════════════════════════════════════════════════════════════════════
//  §0 Lean #eval 导出 fixture 机器耦合（631 兑现，消手工转录漂移）
//
//  卖侧 delta/recog/transition 与买侧 ledger delta（供 dual_mirror）由
//  `formal/Origin/ParityFixtureExport.lean` 的 #eval **机器导出**（Lean 真求值
//  sellDecisionLedgerDelta / recogChanlunSell / sellTransition / decisionLedgerDelta），序列化为
//  JSON。本文件用 include_str! 读同一 fixture，断言 rust 计算 == fixture 中 Lean 导出值。
//
//  ★631 点名修复：此前 lean_buy_side_delta_via_dual_mirror 用**硬编码** `lean_buy_open_root_a: i64=1`
//  （买侧无 rust 实装，只能手填）——这正是手工转录漂移风险点。现改为从 fixture 读 Lean 买侧
//  decisionLedgerDelta openRoot/accreteCore 的 A 分量（机器导出），手填常量被消除。bit-exact = L0。
// ════════════════════════════════════════════════════════════════════════════

/// fixture JSON 中本文件卖侧 + dual_mirror 消费的子集（serde 镜像 ParityFixtureExport.fixtureJson）。
#[derive(Deserialize)]
struct ParityFixture {
    buy_ledger_delta: BuyLedgerDelta,
    sell_ledger_delta: SellLedgerDelta,
    sell_recog: SellRecog,
    sell_transition: SellTransition,
    gap_overlap: GapOverlapSection,
}

/// #246 相切=重合裁定（ticket #248）fixture 段：单区间对的 Lean 机器见证
/// （`decide (HasGap ..)` / `decide (¬ HasGap ..)` 真求值；后者经已证 `gap_iff_not_overlap`
/// 与 Overlaps 严格互推——Overlaps 缺 Decidable 实例，补实例属证明项改动，ESCALATE 登记）。
#[derive(Deserialize)]
struct GapOverlapSection {
    tangent_a_high_eq_b_low: GapOverlapCase,
    tangent_b_high_eq_a_low: GapOverlapCase,
    strict_disjoint: GapOverlapCase,
    strict_disjoint_rev: GapOverlapCase,
    strict_overlap: GapOverlapCase,
}

#[derive(Deserialize)]
struct GapOverlapCase {
    has_gap: bool,
    overlaps: bool,
}

#[derive(Deserialize)]
struct BuyLedgerDelta {
    open_root: (i64, i64, i64),
    accrete_core: (i64, i64, i64),
}

#[derive(Deserialize)]
struct SellLedgerDelta {
    close_root: (i64, i64, i64),
    reduce_core: (i64, i64, i64),
    hold: (i64, i64, i64),
}

#[derive(Deserialize)]
struct SellRecog {
    sample_type1: String,
    sample_type3: String,
}

#[derive(Deserialize)]
struct SellTransition {
    base_a_5: i64,
    type1_a_at_base5: i64,
    type1_pi_at_base5: i64,
    type3_a_at_base5: i64,
}

fn load_fixture() -> ParityFixture {
    let raw = include_str!("fixtures/theta_v0_parity.json");
    serde_json::from_str(raw).expect("fixture 必须是 Lean #eval 导出的合法 JSON")
}

/// Lean 导出的 SellDecision 枚举名字符串 → rust SellDecision（期望值由 fixture 决定，非手填）。
fn sell_decision_from_lean(s: &str) -> SellDecision {
    match s {
        "closeRoot" => SellDecision::CloseRoot,
        "reduceCore" => SellDecision::ReduceCore,
        "hold" => SellDecision::Hold,
        other => panic!("fixture 出现未知卖侧 decision 名「{other}」——Lean 枚举与 rust 镜像漂移"),
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §1 Lean 买侧见证字段 → 经买↔卖对偶编码为 rust SellEndpoint
//
//  对偶规则（Lean SellPointRecog 模块头 + rust sell.rs:82-96）：
//  - 买侧第一类（brokeCenter ∧ IsDivergence ∧ side=long）⟺ 卖侧第一类（broke_center ∧ is_divergence ∧ side=short）。
//  - 买侧第三类（leftCenter ∧ firstRetrace ∧ zg < retracePrice ∧ side=long）⟺
//    卖侧第三类（left_center ∧ first_retrace ∧ retrace_price < zd ∧ side=short）。
//    位置镜像：买侧「不破 ZG = retracePrice 高于 zg」⟺ 卖侧「不破 ZD = retrace_price 低于 zd」。
// ════════════════════════════════════════════════════════════════════════════

/// Lean `eventType1`（ThetaInstantiation.lean:358-369）的卖侧对偶编码。
///
/// Lean 买侧 `eventType1`：`brokeCenter=true`，`divPair={forceA:=8, forceC:=2}` ⟹ `IsDivergence`
/// （2<8）成立。对偶到卖侧：`side=Short`，`broke_center=true`，`is_divergence=true`。
/// 注：rust `SellEndpoint.is_divergence` 是 MACD 真算后的 bool（sell.rs:64-66）——这里直接给出
/// Lean `IsDivergence(forceA=8,forceC=2)` 的判定结果 `true`（2<8），bit-exact 对齐 Lean 见证的
/// 背驰判定（不是另造 MACD 数据，是 Lean 见证 `divPair` 的 IsDivergence 结论）。
fn lean_event_type1_dual() -> SellEndpoint {
    SellEndpoint {
        side: Side::Short,
        broke_center: true,   // Lean eventType1.bsp.brokeCenter = true
        is_divergence: true,  // Lean IsDivergence{forceA:=8, forceC:=2} = (2 < 8) = true
        left_center: false,   // Lean eventType1.bsp.leftCenter = false
        first_retrace: false, // Lean eventType1.bsp.firstRetrace = false
        // 第三类无关字段：买侧 retracePrice=5, center.zg=20（5<20 ⟹ 不满足买侧第三类「zg<retracePrice」）。
        // 对偶卖侧：retrace_price 高于 center_zd ⟹ 不满足卖侧第三类「retrace<zd」（保持第一类纯净）。
        retrace_price: 20,
        center_zd: 10,
    }
}

/// Lean `eventType3`（ThetaInstantiation.lean:376-387）的卖侧对偶编码。
///
/// Lean 买侧 `eventType3`：`side=long`，`leftCenter=true`，`firstRetrace=true`，`center.zg=20`，
/// `retracePrice=25`（20<25 ⟹ 不破 ZG）；`brokeCenter=false`（区别第一类）。对偶到卖侧：位置镜像
/// 「不破 ZD」⟹ `retrace_price < center_zd`。用对偶位置 `center_zd=20`，`retrace_price=15`（15<20 ⟹
/// 不破 ZD），保持与 Lean 见证同构的「第一次离开中枢回试不入」结构。
fn lean_event_type3_dual() -> SellEndpoint {
    SellEndpoint {
        side: Side::Short,
        broke_center: false,  // Lean eventType3.bsp.brokeCenter = false（未破中枢，区别第一类）
        is_divergence: false, // Lean eventType3.bsp.divPair 力度延续（forceA=3,forceC=3 ⟹ 非背驰）
        left_center: true,    // Lean eventType3.bsp.leftCenter = true
        first_retrace: true,  // Lean eventType3.bsp.firstRetrace = true
        // 位置镜像：买侧「center.zg=20 < retracePrice=25」⟺ 卖侧「retrace_price=15 < center_zd=20」。
        retrace_price: 15,
        center_zd: 20,
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §2 卖侧判据 bit-exact 对齐 Lean（rust 有 port ⟹ 直接交叉验证）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `eventType1` 对偶 ⟹ rust `is_type1_sell` = true（对齐 Lean `eventType1_isType1` :390）。
#[test]
fn lean_type1_witness_matches_rust_is_type1() {
    assert!(
        is_type1_sell(&lean_event_type1_dual()),
        "Lean eventType1（破中枢+背驰）对偶后 rust is_type1_sell 必为 true"
    );
}

/// Lean `eventType3` 对偶 ⟹ rust `is_type3_sell` = true（对齐 Lean `eventType3_isType3Buy` :394）。
#[test]
fn lean_type3_witness_matches_rust_is_type3() {
    assert!(
        is_type3_sell(&lean_event_type3_dual()),
        "Lean eventType3（离开中枢+回试不入）对偶后 rust is_type3_sell 必为 true"
    );
    // 互斥：第三类见证不满足第一类判据（与 Lean eventType3 brokeCenter=false 一致）。
    assert!(
        !is_type1_sell(&lean_event_type3_dual()),
        "Lean eventType3 未破中枢 ⟹ 非第一类（对齐分支互斥）"
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  §3 卖侧识别核 recog bit-exact（对齐 Lean recogChanlun 买侧 + recogChanlunSell 卖侧）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `eventType1_recog_openRoot`（:398）的卖侧对偶：recog = `CloseRoot`（买 openRoot 对偶）。
#[test]
fn lean_type1_recog_bit_exact() {
    let fx = load_fixture();
    // 期望值从 fixture 读（Lean #eval recogChanlunSell sampleType1Sell = "closeRoot"），非手填。
    assert_eq!(
        recog_chanlun_sell(&lean_event_type1_dual()),
        sell_decision_from_lean(&fx.sell_recog.sample_type1),
        "rust recog_chanlun_sell == Lean #eval recogChanlunSell sampleType1Sell（bit-exact）"
    );
}

/// Lean `eventType3_recog_accreteCore`（:402）的卖侧对偶：recog = `ReduceCore`（买 accreteCore 对偶）。
#[test]
fn lean_type3_recog_bit_exact() {
    let fx = load_fixture();
    // 期望值从 fixture 读（Lean #eval recogChanlunSell sampleType3Sell = "reduceCore"），非手填。
    assert_eq!(
        recog_chanlun_sell(&lean_event_type3_dual()),
        sell_decision_from_lean(&fx.sell_recog.sample_type3),
        "rust recog_chanlun_sell == Lean #eval recogChanlunSell sampleType3Sell（bit-exact）"
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  §4 ledger delta bit-exact（对齐 Lean decisionLedgerDelta + sellDecisionLedgerDelta）
//
//  Lean 买侧 decisionLedgerDelta（:256-259）：openRoot=(0,1,0)，accreteCore=(0,2,0)，hold=(0,0,0)。
//  rust 卖侧 sell_decision_ledger_delta（sell.rs:135-141）：closeRoot=(1,-1,0)，reduceCore=(0,-2,0)，hold=(0,0,0)。
//  A 分量镜像（Lean buy_sell_A_delta_mirror）：买 +1 ↔ 卖 -1；买 +2 ↔ 卖 -2。
// ════════════════════════════════════════════════════════════════════════════

/// 卖侧 delta bit-exact 对齐 Lean `sellDecisionLedgerDelta`（#121）。
#[test]
fn lean_sell_ledger_delta_bit_exact() {
    let fx = load_fixture();
    let d = &fx.sell_ledger_delta;
    // 期望值从 fixture 读（Lean #eval sellDecisionLedgerDelta closeRoot/reduceCore/hold），非手填三元组。
    assert_eq!(
        sell_decision_ledger_delta(SellDecision::CloseRoot),
        d.close_root,
        "rust closeRoot delta == Lean #eval sellDecisionLedgerDelta closeRoot"
    );
    assert_eq!(
        sell_decision_ledger_delta(SellDecision::ReduceCore),
        d.reduce_core,
        "rust reduceCore delta == Lean #eval sellDecisionLedgerDelta reduceCore"
    );
    assert_eq!(
        sell_decision_ledger_delta(SellDecision::Hold),
        d.hold,
        "rust hold delta == Lean #eval sellDecisionLedgerDelta hold"
    );
}

/// ★买侧 ThetaInstantiation delta 经对偶间接验证（Lean decisionLedgerDelta + buy_sell_A_delta_mirror）。
///
/// ★631 点名修复：原硬编码 `lean_buy_open_root_a: i64 = 1`（手填，转录漂移风险）已替换为从 fixture
/// 读 Lean 买侧 decisionLedgerDelta openRoot/accreteCore 的 A 分量（机器导出）。Lean 改买侧 delta
/// ⟹ #eval 输出变 ⟹ fixture 变 ⟹ 本断言随之变，漂移被消除。
///
/// 本测试断言 rust 卖侧 delta 的 A 分量是买侧 Lean 导出 A 分量的严格相反号——即 rust 实装满足 Lean
/// `buy_sell_A_delta_mirror`（SellClosedLoop §4，已证）。这是买侧 Lean delta 在 rust 端的**间接**
/// bit-exact 交叉验证（买侧无 rust 独立实现于本卖侧文件；买侧直接 parity 见 theta_v0_buy_parity.rs）。
#[test]
fn lean_buy_side_delta_via_dual_mirror() {
    let fx = load_fixture();
    // 买侧 Lean decisionLedgerDelta 的 A 分量——从 fixture 读（机器导出），非硬编码常量。
    let lean_buy_open_root_a = fx.buy_ledger_delta.open_root.1; // Lean openRoot delta.A = 1
    let lean_buy_accrete_core_a = fx.buy_ledger_delta.accrete_core.1; // Lean accreteCore delta.A = 2
    // rust 卖侧 A 分量。
    let rust_sell_close_root_a = sell_decision_ledger_delta(SellDecision::CloseRoot).1; // -1
    let rust_sell_reduce_core_a = sell_decision_ledger_delta(SellDecision::ReduceCore).1; // -2
    assert_eq!(
        lean_buy_open_root_a, -rust_sell_close_root_a,
        "Lean 买侧 openRoot A ↔ rust 卖侧 closeRoot A 严格相反号（buy_sell_A_delta_mirror，A 分量从 fixture 读）"
    );
    assert_eq!(
        lean_buy_accrete_core_a, -rust_sell_reduce_core_a,
        "Lean 买侧 accreteCore A ↔ rust 卖侧 reduceCore A 严格相反号（buy_sell_A_delta_mirror，A 分量从 fixture 读）"
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  §5 闭环 transition bit-exact（对齐 Lean chanlunTransition 系列 + sellTransition）
// ════════════════════════════════════════════════════════════════════════════

/// 把 Lean `chanlunTransition_type1_allocates`（买侧 A=base+1）对偶为卖侧 A=base-1 的逐态验证。
///
/// 从具体初始账本态（base A=5）出发，跑 Lean eventType1 对偶见证经 rust `sell_transition`，
/// 断言 ledger.A = base-1（卖侧第一类清根仓；对偶买侧 base+1，A 分量镜像）。
#[test]
fn lean_type1_transition_ledger_bit_exact() {
    let fx = load_fixture();
    let t = &fx.sell_transition;
    // base_a 与期望 A/Pi 从 fixture 读（Lean #eval sellTransition base=5 后 ledger.A/Pi），非手填。
    let base_a: i64 = t.base_a_5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = sell_transition(&x0, &lean_event_type1_dual());
    // 卖侧第一类：A == Lean #eval (sellTransition base5 sampleType1Sell).ledger.A（=base-1=4）。
    assert_eq!(
        x1.ledger_state.a, t.type1_a_at_base5,
        "rust 卖侧第一类 A == Lean #eval sellTransition A（base=5）"
    );
    // 卖侧第一类 Π == Lean #eval ...ledger.Pi（=1，实现利润；买侧 Π 不变的时间不对称）。
    assert_eq!(
        x1.ledger_state.pi, t.type1_pi_at_base5,
        "rust 卖侧第一类 Π == Lean #eval sellTransition Pi（base=5，实现利润 Π+=1）"
    );
    // 闭环保 R=Π-A-W（Lean chanlunTransition_preserves_ledger_inv / sellTransition_preserves_ledger_inv）。
    assert!(x1.ledger_state.inv_holds(), "闭环转移保 R=Π-A-W");
}

/// 把 Lean `chanlunTransition_type3_allocates`（买侧 A=base+2）对偶为卖侧 A=base-2 的逐态验证。
#[test]
fn lean_type3_transition_ledger_bit_exact() {
    let fx = load_fixture();
    let t = &fx.sell_transition;
    let base_a: i64 = t.base_a_5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = sell_transition(&x0, &lean_event_type3_dual());
    // 卖侧第三类：A == Lean #eval (sellTransition base5 sampleType3Sell).ledger.A（=base-2=3）。
    assert_eq!(
        x1.ledger_state.a, t.type3_a_at_base5,
        "rust 卖侧第三类 A == Lean #eval sellTransition A（base=5）"
    );
    assert!(x1.ledger_state.inv_holds(), "闭环转移保 R=Π-A-W");
}

/// ★★非退化总见证 bit-exact（对齐 Lean `chanlun_transition_distinguishes_classes` :420）。
///
/// Lean 买侧：第一类 A=base+1 ≠ 第三类 A=base+2（两类缠论分类 ⟹ 不同 ledger 足迹，非平凡占位）。
/// 卖侧对偶：第一类 A=base-1 ≠ 第三类 A=base-2。本测试用 Lean eventType1/eventType3 对偶见证从
/// **同一**初始态跑 rust 闭环，断言两类产生不同 A delta——bit-exact 兑现 Lean 非退化结论。
#[test]
fn lean_transition_distinguishes_classes_bit_exact() {
    let fx = load_fixture();
    let t = &fx.sell_transition;
    // base 与两类期望 A 从 fixture 读（Lean #eval sellTransition base=5：type1 A=4 ≠ type3 A=3），非手填。
    let base_a: i64 = t.base_a_5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let a_type1 = sell_transition(&x0, &lean_event_type1_dual()).ledger_state.a;
    let a_type3 = sell_transition(&x0, &lean_event_type3_dual()).ledger_state.a;
    assert_ne!(
        a_type1, a_type3,
        "Lean chanlun_transition_distinguishes_classes：两类缠论分类 ⟹ 不同 ledger A delta"
    );
    // 逐值锚定 Lean 导出（卖侧对偶具体值，从 fixture 读）。
    assert_eq!(a_type1, t.type1_a_at_base5, "rust 第一类 A == Lean #eval sellTransition A（base=5）");
    assert_eq!(a_type3, t.type3_a_at_base5, "rust 第三类 A == Lean #eval sellTransition A（base=5）");
}

// ════════════════════════════════════════════════════════════════════════════
//  §6 hold bit-exact（对齐 Lean recog_continuation_hold + hold delta (0,0,0)）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `recog_continuation_hold`（:199）对偶：力度延续（非背驰 ∧ 未离开中枢）⟹ hold，delta (0,0,0)。
#[test]
fn lean_continuation_hold_bit_exact() {
    let cont = SellEndpoint {
        side: Side::Short,
        broke_center: false,
        is_divergence: false, // 力度延续（IsContinuation = 非背驰）
        left_center: false,   // 未离开中枢
        first_retrace: false,
        retrace_price: 0,
        center_zd: 10,
    };
    let fx = load_fixture();
    assert_eq!(recog_chanlun_sell(&cont), sell_decision_from_lean("hold"), "力度延续 ⟹ hold");
    // hold delta 从 fixture 读（Lean #eval sellDecisionLedgerDelta hold），非手填 (0,0,0)。
    assert_eq!(
        sell_decision_ledger_delta(SellDecision::Hold),
        fx.sell_ledger_delta.hold,
        "rust hold delta == Lean #eval sellDecisionLedgerDelta hold"
    );
    // hold 闭环不改账本（Lean hold delta 全零 ⟹ ledger 恒等）。
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 3, a: 5, w: 1, r: -3 },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = sell_transition(&x0, &cont);
    assert_eq!(x1.ledger_state, x0.ledger_state, "hold 闭环不改 ledger（bit-exact (0,0,0)）");
}

// ════════════════════════════════════════════════════════════════════════════
//  §7 #246 相切=重合裁定机器见证（ticket #248，2026-07-25 裁定书
//     chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md）
//
//  裁定：相切（两区间只有一个公共端点）算「有重合区间」⟹ 无缺口，全域生效。
//  Lean 侧 `HasGap`（严格 `<`，SegmentFeatureSeq.lean:102）与 `Overlaps`（`≤`，:111）
//  本已符合裁定且 `gap_iff_not_overlap`（:115）已证互补；rust 侧 `Interval::overlaps`（≤）
//  与 `Interval::gap`（!overlaps）与之逐字对齐。本测试断言：对同一批区间对用例，
//  rust 既有原语计算 == fixture 中 Lean `decide` 机器导出值。
//
//  用例区间端点为双侧镜像的**用例输入**（与 §1 SellEndpoint 手编码输入同性质）；
//  期望值（has_gap/overlaps）全部从 fixture 读（Lean #eval 机器产，禁手填）。
//  Overlaps 真值经 `decide (¬ HasGap ..)` 读出（Overlaps 缺 Decidable 实例；补实例属
//  证明项级改动，ESCALATE 登记于 tangency-impact-quantification-20260725.md §8）。
//  三笔形态（max(lows)==min(highs)）不适用：Lean 无三笔重合谓词，其相切语义与两区间
//  形态同构；rust 三笔谓词相切行为由 segment.rs 单测 three_stroke_overlap_tangent_counts 覆盖。
// ════════════════════════════════════════════════════════════════════════════

/// rust `Interval::overlaps`/`gap` == Lean `decide(HasGap)`/`decide(¬HasGap)` 逐用例 bit-exact。
#[test]
fn lean_gap_overlap_tangent_bit_exact() {
    let fx = load_fixture();
    // (a, b, Lean 机器导出期望, 用例名)——区间端点与 ParityFixtureExport gap_overlap 段逐一镜像。
    let cases: [(Interval, Interval, &GapOverlapCase, &str); 5] = [
        (
            Interval { lo: 5, hi: 10 },
            Interval { lo: 10, hi: 20 },
            &fx.gap_overlap.tangent_a_high_eq_b_low,
            "tangent_a_high_eq_b_low（[5,10] 与 [10,20] 相切）",
        ),
        (
            Interval { lo: 10, hi: 20 },
            Interval { lo: 5, hi: 10 },
            &fx.gap_overlap.tangent_b_high_eq_a_low,
            "tangent_b_high_eq_a_low（反向相切）",
        ),
        (
            Interval { lo: 5, hi: 10 },
            Interval { lo: 11, hi: 20 },
            &fx.gap_overlap.strict_disjoint,
            "strict_disjoint（严格分离）",
        ),
        (
            Interval { lo: 11, hi: 20 },
            Interval { lo: 5, hi: 10 },
            &fx.gap_overlap.strict_disjoint_rev,
            "strict_disjoint_rev（反向严格分离）",
        ),
        (
            Interval { lo: 5, hi: 12 },
            Interval { lo: 8, hi: 20 },
            &fx.gap_overlap.strict_overlap,
            "strict_overlap（严格重叠对照）",
        ),
    ];
    for (a, b, expected, name) in cases {
        assert_eq!(
            a.gap(&b),
            expected.has_gap,
            "{name}: rust Interval::gap == Lean decide(HasGap)（bit-exact）"
        );
        assert_eq!(
            a.overlaps(&b),
            expected.overlaps,
            "{name}: rust Interval::overlaps == Lean decide(¬HasGap)（经 gap_iff_not_overlap ↔ Overlaps）"
        );
        // 互补性在 Bool 层自证：fixture 导出值自身须满足 has_gap == !overlaps。
        assert_eq!(
            expected.has_gap, !expected.overlaps,
            "{name}: fixture 导出值违反 gap_iff_not_overlap 互补（Lean 侧漂移）"
        );
    }
}
