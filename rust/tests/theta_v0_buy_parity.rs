//! 买侧 Lean ↔ Rust **直接** bit-exact parity（acceptance #4 买侧闭合）——消间接对偶 vacuous gap。
//!
//! ## 工位定位（SG-1 买侧 vacuous gap，消 090 声明膨胀活跃点）
//!
//! `tests/theta_v0_lean_parity.rs` 模块头（:39-50）已诚实标注：买侧 `Origin.ThetaInstantiation` 链
//! 在 rust 此前**无独立实装**，买侧「bit-exact」全部经卖侧对偶 `buy_sell_A_delta_mirror` 间接验证
//! ——即「rust 卖侧 = Lean 卖侧见证」+「卖侧 = -买侧 Lean 见证」⟹ 买侧 Lean 见证经对偶被覆盖。
//! 这是**逻辑 vacuous**：买侧 rust 函数从未被独立断言，`lean_buy_side_delta_via_dual_mirror`（:184）
//! 的 `lean_buy_open_root_a: i64 = 1` 是**手填常量**，不是调买侧 rust 函数。
//!
//! 本文件把 `src/theta_v0/closed_loop/buy.rs`（独立买侧实装，port ThetaInstantiation.lean）的真函数
//! `recog_chanlun_buy` / `buy_decision_ledger_delta` / `buy_transition`，与 **Lean 买侧已机器验证的
//! 具体见证**（`eventType1`/`eventType3`/`decisionLedgerDelta`/`chanlunTransition_type*_allocates`/
//! `chanlun_transition_distinguishes_classes`）逐字段**直接**比对——买侧 bit-exact 从「经卖侧对偶
//! 间接」升级为「rust 买侧函数 ↔ Lean 买侧见证直接」，消除 vacuous。
//!
//! ## golden 来源（Lean 买侧具体见证，formal/Origin/ThetaInstantiation.lean，已 machine-check）
//!
//! - `eventType1`（:358-369）：`side=long`，`brokeCenter=true`，`divPair={forceA:=8, forceC:=2,
//!   isTrend:=true}` ⟹ `IsDivergence`（forceC<forceA：2<8）⟹ `eventType1_isType1`（:390）。
//!   `recogChanlun eventType1 = openRoot`（`eventType1_recog_openRoot` :398，已证定理）。
//! - `eventType3`（:376-387）：`side=long`，`brokeCenter=false`，`leftCenter=true`，`firstRetrace=true`，
//!   `center.zg=20`（`witnessCenter.zg` :348），`retracePrice=25`（20<25 ⟹ 不破 ZG）⟹ `eventType3_isType3Buy`
//!   （:394）。`recogChanlun eventType3 = accreteCore`（`eventType3_recog_accreteCore` :402，已证定理）。
//! - `decisionLedgerDelta`（:256-259）：openRoot⟹(0,1,0)，accreteCore⟹(0,2,0)，hold⟹(0,0,0)。
//! - `chanlunTransition_type1_allocates`（:319，已证）：第一类 ⟹ ledger.A = base+1。
//! - `chanlunTransition_type3_allocates`（:331，已证）：第三类 ⟹ ledger.A = base+2。
//! - `chanlun_transition_distinguishes_classes`（:420，已证）：第一类 A=base+1 ≠ 第三类 A=base+2。
//! - `chanlunTransition_preserves_ledger_inv`（:307，已证）：闭环转移后 R=Π-A-W。
//!
//! ★每个 golden 逐字段从 Lean 见证 def 字段值提取（非凭空手填）：`eventType1.bsp.brokeCenter=true`、
//!   `divPair.forceA=8/forceC=2`、`eventType3.bsp.retracePrice=25`、`witnessCenter.zg=20` 等，均
//!   Lean 源文件可逐行核对（行号标注于上）。背驰判据：rust `is_divergence` 字段直接给 Lean
//!   `IsDivergence{forceA:=8,forceC:=2}` 的判定结论 `true`（2<8），bit-exact 对齐 Lean 见证。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 本文件建立 **L0(Lean machine-checked 买侧见证) ↔ L1(rust 买侧实装) 直接对齐**。`cargo test` 绿 =
//! rust `buy.rs` 对 Lean `ThetaInstantiation` 买侧具体见证逐字段产出 bit-exact 相同的分类 + ledger
//! delta + 闭环转移 A 足迹。这**不**提升经验等级（非缠论盈利/实盘有效 L2/L3）——确认的是「rust 买侧
//! 实装忠实于 Lean 买侧形式化」这一管线一致性。**禁标 L2**（无真实行情数据，无可否证经验断言）。
//!
//! ## 诚实边界（no声明膨胀）— 本文件不证
//!
//! - ✗ 买侧策略盈利/最优/实盘有效（L3）。
//! - ✗ bsp 端点的自动识别（消费已判定 BuyEndpoint，#113 bspOf 全自动 still-MISSING）。
//! - ✗ 第二类买点（次级别递归，#113 still-MISSING-D）——买侧三态（第一类/第三类/延续）与 Lean
//!   recogChanlun :151 三态一致，第二类诚实留白。

use newchan_rust::theta_v0::closed_loop::buy::{
    buy_decision_ledger_delta, buy_transition, is_type1_buy, is_type3_buy, recog_chanlun_buy,
    BuyDecision, BuyEndpoint,
};
use newchan_rust::theta_v0::closed_loop::state::AssemblyState;
use newchan_rust::theta_v0::strategy::ledger::LedgerComp;
use newchan_rust::theta_v0::types::Side;

// ════════════════════════════════════════════════════════════════════════════
//  §1 Lean 买侧见证字段 → 逐字段直接编码为 rust BuyEndpoint（非对偶反推）
//
//  与 theta_v0_lean_parity.rs 的关键区别：那里把买侧 Lean 见证经买↔卖对偶编码为 SellEndpoint
//  （间接）；本文件直接编码为 BuyEndpoint，调买侧 rust 函数直接比 Lean 买侧见证（直接）。
// ════════════════════════════════════════════════════════════════════════════

/// Lean `eventType1`（ThetaInstantiation.lean:358-369）的**直接**买侧编码（逐字段，非对偶）。
///
/// Lean `eventType1.bsp`（:359-368）：
/// - `side := Side.long`（:360）
/// - `center := witnessCenter`（zg=20，:348）
/// - `divPair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := true }`（:362）⟹ IsDivergence（2<8）
/// - `brokeCenter := true`（:363）
/// - `leftCenter := false`（:365）
/// - `retracePrice := 5`（:366）
/// - `firstRetrace := false`（:367）
///
/// rust `is_divergence` 直接给 Lean `IsDivergence{forceA:=8, forceC:=2}` 的判定结论 `true`（2<8），
/// bit-exact 对齐 Lean 见证背驰判定（不另造 MACD 数据，是 Lean divPair 的 IsDivergence 结论）。
fn lean_event_type1_buy() -> BuyEndpoint {
    BuyEndpoint {
        side: Side::Long,        // Lean :360
        broke_center: true,      // Lean :363
        is_divergence: true,     // Lean IsDivergence{forceA:=8, forceC:=2} = (2 < 8) = true（:362）
        left_center: false,      // Lean :365
        first_retrace: false,    // Lean :367
        retrace_price: 5,        // Lean :366
        center_zg: 20,           // Lean witnessCenter.zg（:348）
    }
}

/// Lean `eventType3`（ThetaInstantiation.lean:376-387）的**直接**买侧编码（逐字段，非对偶）。
///
/// Lean `eventType3.bsp`（:377-385）：
/// - `side := Side.long`（:378）
/// - `center := witnessCenter`（zg=20，:348）
/// - `divPair := { forceA := ⟨3⟩, forceC := ⟨3⟩, isTrend := false }`（:380）⟹ 力度延续（非背驰）
/// - `brokeCenter := false`（:381）
/// - `leftCenter := true`（:383）
/// - `retracePrice := 25`（:384）（25 > zg=20 ⟹ 不破 ZG ⟹ 第三类）
/// - `firstRetrace := true`（:385）
fn lean_event_type3_buy() -> BuyEndpoint {
    BuyEndpoint {
        side: Side::Long,        // Lean :378
        broke_center: false,     // Lean :381（未破中枢，区别第一类）
        is_divergence: false,    // Lean divPair{forceA:=3,forceC:=3} 力度延续（非背驰，:380）
        left_center: true,       // Lean :383
        first_retrace: true,     // Lean :385
        retrace_price: 25,       // Lean :384（25 > zg=20 ⟹ 不破 ZG）
        center_zg: 20,           // Lean witnessCenter.zg（:348）
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §2 买点判据 bit-exact 直接对齐 Lean（rust 有买侧 port ⟹ 直接交叉验证）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `eventType1_isType1`（:390）：rust `is_type1_buy(eventType1) = true`（直接比 Lean 见证）。
#[test]
fn lean_type1_witness_matches_rust_is_type1() {
    assert!(
        is_type1_buy(&lean_event_type1_buy()),
        "Lean eventType1（破中枢+背驰）⟹ rust is_type1_buy 必为 true（直接 parity）"
    );
}

/// Lean `eventType3_isType3Buy`（:394）：rust `is_type3_buy(eventType3) = true`（直接比 Lean 见证）。
#[test]
fn lean_type3_witness_matches_rust_is_type3() {
    assert!(
        is_type3_buy(&lean_event_type3_buy()),
        "Lean eventType3（离开中枢+回抽不破 ZG）⟹ rust is_type3_buy 必为 true（直接 parity）"
    );
    // 互斥：第三类见证不满足第一类判据（与 Lean eventType3 brokeCenter=false 一致，:381）。
    assert!(
        !is_type1_buy(&lean_event_type3_buy()),
        "Lean eventType3 未破中枢 ⟹ 非第一类（对齐分支互斥）"
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  §3 买侧识别核 recog bit-exact 直接对齐 Lean recogChanlun（:151）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `eventType1_recog_openRoot`（:398，已证定理）：rust `recog_chanlun_buy(eventType1) = OpenRoot`。
#[test]
fn lean_type1_recog_bit_exact() {
    assert_eq!(
        recog_chanlun_buy(&lean_event_type1_buy()),
        BuyDecision::OpenRoot,
        "Lean eventType1_recog_openRoot ⟹ rust recog = OpenRoot（直接 bit-exact）"
    );
}

/// Lean `eventType3_recog_accreteCore`（:402，已证定理）：rust `recog_chanlun_buy(eventType3) = AccreteCore`。
#[test]
fn lean_type3_recog_bit_exact() {
    assert_eq!(
        recog_chanlun_buy(&lean_event_type3_buy()),
        BuyDecision::AccreteCore,
        "Lean eventType3_recog_accreteCore ⟹ rust recog = AccreteCore（直接 bit-exact）"
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  §4 买侧 ledger delta bit-exact 直接对齐 Lean decisionLedgerDelta（:256-259）
//
//  Lean 买侧 decisionLedgerDelta（:256-259）：openRoot=(0,1,0)，accreteCore=(0,2,0)，hold=(0,0,0)。
//  ★直接调 rust buy_decision_ledger_delta，非 theta_v0_lean_parity.rs:184 的硬编码 `i64 = 1`。
// ════════════════════════════════════════════════════════════════════════════

/// 买侧 delta bit-exact 直接对齐 Lean `decisionLedgerDelta`（:256-259，逐字段从 Lean 源提取）。
///
/// ★消 vacuous：此前 `theta_v0_lean_parity.rs::lean_buy_side_delta_via_dual_mirror`（:184）用
/// 硬编码 `lean_buy_open_root_a: i64 = 1`（买侧无 rust 实装，只能手填）。本测试**调真 rust 买侧函数**
/// 直接断言三态 delta 等于 Lean machine-checked 常量——买侧 bit-exact 从手填升级为真函数交叉验证。
#[test]
fn lean_buy_ledger_delta_bit_exact() {
    // Lean decisionLedgerDelta openRoot = (0, 1, 0)（:257）。
    assert_eq!(buy_decision_ledger_delta(BuyDecision::OpenRoot), (0, 1, 0));
    // Lean decisionLedgerDelta accreteCore = (0, 2, 0)（:258）。
    assert_eq!(buy_decision_ledger_delta(BuyDecision::AccreteCore), (0, 2, 0));
    // Lean decisionLedgerDelta hold = (0, 0, 0)（:259）。
    assert_eq!(buy_decision_ledger_delta(BuyDecision::Hold), (0, 0, 0));
}

// ════════════════════════════════════════════════════════════════════════════
//  §5 买侧闭环 transition bit-exact 直接对齐 Lean chanlunTransition 系列（:297-428）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `chanlunTransition_type1_allocates`（:319，已证）：第一类 ⟹ ledger.A = base+1（直接 parity）。
///
/// 从具体初始账本态（base A=5）跑 Lean eventType1 直接见证经 rust `buy_transition`，断言 ledger.A
/// = base+1（第一类建根仓 allocate 1）。逐态兑现 Lean 已证定理的 A 足迹。
#[test]
fn lean_type1_transition_ledger_bit_exact() {
    let base_a: i64 = 5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = buy_transition(&x0, &lean_event_type1_buy());
    // 买侧第一类：A = base + 1（Lean chanlunTransition_type1_allocates :319）。
    assert_eq!(x1.ledger_state.a, base_a + 1, "买侧第一类建根仓 A = base + 1");
    // ★买侧 Π 不变（买入无实现损益，Lean openRoot dΠ=0；对照卖侧 closeRoot dΠ=+1 的时间不对称）。
    assert_eq!(x1.ledger_state.pi, 0, "买侧第一类 Π 不变（dΠ=0）");
    // 闭环保 R=Π-A-W（Lean chanlunTransition_preserves_ledger_inv :307）。
    assert!(x1.ledger_state.inv_holds(), "闭环转移保 R=Π-A-W");
}

/// Lean `chanlunTransition_type3_allocates`（:331，已证）：第三类 ⟹ ledger.A = base+2（直接 parity）。
#[test]
fn lean_type3_transition_ledger_bit_exact() {
    let base_a: i64 = 5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = buy_transition(&x0, &lean_event_type3_buy());
    // 买侧第三类：A = base + 2（Lean chanlunTransition_type3_allocates :331）。
    assert_eq!(x1.ledger_state.a, base_a + 2, "买侧第三类增核 A = base + 2");
    assert!(x1.ledger_state.inv_holds(), "闭环转移保 R=Π-A-W");
}

/// ★★非退化总见证 bit-exact 直接对齐 Lean `chanlun_transition_distinguishes_classes`（:420，已证）。
///
/// Lean 买侧：第一类 A=base+1 ≠ 第三类 A=base+2（两类缠论分类 ⟹ 不同 ledger 足迹，非平凡占位）。
/// 本测试用 Lean eventType1/eventType3 **直接**买侧见证从**同一**初始态跑 rust `buy_transition`，
/// 断言两类产生不同 A delta——bit-exact 直接兑现 Lean 非退化结论（非经卖侧对偶）。
#[test]
fn lean_transition_distinguishes_classes_bit_exact() {
    let base_a: i64 = 10;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let a_type1 = buy_transition(&x0, &lean_event_type1_buy()).ledger_state.a; // base + 1 = 11
    let a_type3 = buy_transition(&x0, &lean_event_type3_buy()).ledger_state.a; // base + 2 = 12
    assert_ne!(
        a_type1, a_type3,
        "Lean chanlun_transition_distinguishes_classes：两类缠论分类 ⟹ 不同 ledger A delta"
    );
    // 逐值锚定 Lean 买侧（base+1=11 / base+2=12）。
    assert_eq!(a_type1, base_a + 1, "第一类 A = base + 1");
    assert_eq!(a_type3, base_a + 2, "第三类 A = base + 2");
}

// ════════════════════════════════════════════════════════════════════════════
//  §6 hold bit-exact + ★负向探针（证 parity 非平凡——断言非空过，可否证）
// ════════════════════════════════════════════════════════════════════════════

/// Lean `recog_continuation_hold`（:199）：力度延续（非背驰 ∧ 未离开中枢）⟹ hold，delta (0,0,0)。
#[test]
fn lean_continuation_hold_bit_exact() {
    let cont = BuyEndpoint {
        side: Side::Long,
        broke_center: false,
        is_divergence: false, // 力度延续（IsContinuation = 非背驰）
        left_center: false,   // 未离开中枢
        first_retrace: false,
        retrace_price: 0,
        center_zg: 20,
    };
    assert_eq!(recog_chanlun_buy(&cont), BuyDecision::Hold, "力度延续 ⟹ hold");
    assert_eq!(buy_decision_ledger_delta(BuyDecision::Hold), (0, 0, 0), "hold delta (0,0,0)");
    // hold 闭环不改账本（Lean hold delta 全零 ⟹ ledger 恒等）。
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 3, a: 5, w: 1, r: -3 },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = buy_transition(&x0, &cont);
    assert_eq!(x1.ledger_state, x0.ledger_state, "hold 闭环不改 ledger（bit-exact (0,0,0)）");
}

/// ★负向探针（证 parity 可否证、非空过）：rust 买侧函数对**违反** Lean 判据的端点产出与第一类/
/// 第三类**不同**的决策——若 recog 是退化常量（恒返回 OpenRoot），这些断言会失败。
///
/// 这坐实 §2-§5 的肯定 parity 不是 vacuously true（端点字段真被消费）：
/// - 破 ZG 的「第三类候选」⟹ 非 AccreteCore（hold）——retrace 字段真被判据消费。
/// - side=Short 的「第三类候选」⟹ 非 AccreteCore（hold）——side 字段真被判据消费（Lean :155）。
/// - 非背驰且未破中枢且未离开中枢 ⟹ hold（≠ OpenRoot ≠ AccreteCore）——三态真区分。
#[test]
fn buy_recog_negative_probes_non_vacuous() {
    // 破 ZG（retrace=15 ≤ zg=20）的第三类候选 ⟹ 非第三类（Lean :156 不破 ZG 失败）⟹ hold。
    let break_zg = BuyEndpoint { retrace_price: 15, ..lean_event_type3_buy() };
    assert_ne!(recog_chanlun_buy(&break_zg), BuyDecision::AccreteCore, "破 ZG ⟹ 非增核（判据真消费 retrace）");
    assert_eq!(recog_chanlun_buy(&break_zg), BuyDecision::Hold, "破 ZG 的第三类候选 ⟹ hold");

    // side=Short 的第三类候选 ⟹ 非第三类（Lean :155 检查 side=long）⟹ hold。
    let short_t3 = BuyEndpoint { side: Side::Short, ..lean_event_type3_buy() };
    assert_ne!(recog_chanlun_buy(&short_t3), BuyDecision::AccreteCore, "side=Short ⟹ 非增核（判据真消费 side）");

    // 非背驰的第一类候选（broke_center=true 但 is_divergence=false）⟹ 非第一类（Lean :152 背驰失败）。
    let no_div = BuyEndpoint { is_divergence: false, ..lean_event_type1_buy() };
    assert_ne!(recog_chanlun_buy(&no_div), BuyDecision::OpenRoot, "非背驰 ⟹ 非建根仓（判据真消费 divergence）");

    // ★delta 非平凡：三态 delta 两两不同（OpenRoot/AccreteCore/Hold 的 A 分量 1/2/0 互异）。
    let d_open = buy_decision_ledger_delta(BuyDecision::OpenRoot).1;
    let d_accrete = buy_decision_ledger_delta(BuyDecision::AccreteCore).1;
    let d_hold = buy_decision_ledger_delta(BuyDecision::Hold).1;
    assert_ne!(d_open, d_accrete, "建根仓(1) ≠ 增核(2)（非退化）");
    assert_ne!(d_open, d_hold, "建根仓(1) ≠ 保持(0)");
    assert_ne!(d_accrete, d_hold, "增核(2) ≠ 保持(0)");
}
