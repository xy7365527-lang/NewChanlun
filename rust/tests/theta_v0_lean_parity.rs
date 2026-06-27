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
use newchan_rust::theta_v0::strategy::ledger::LedgerComp;
use newchan_rust::theta_v0::types::Side;

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
    assert_eq!(
        recog_chanlun_sell(&lean_event_type1_dual()),
        SellDecision::CloseRoot,
        "Lean eventType1 ⟹ openRoot；卖侧对偶 ⟹ closeRoot（bit-exact）"
    );
}

/// Lean `eventType3_recog_accreteCore`（:402）的卖侧对偶：recog = `ReduceCore`（买 accreteCore 对偶）。
#[test]
fn lean_type3_recog_bit_exact() {
    assert_eq!(
        recog_chanlun_sell(&lean_event_type3_dual()),
        SellDecision::ReduceCore,
        "Lean eventType3 ⟹ accreteCore；卖侧对偶 ⟹ reduceCore（bit-exact）"
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
    assert_eq!(sell_decision_ledger_delta(SellDecision::CloseRoot), (1, -1, 0));
    assert_eq!(sell_decision_ledger_delta(SellDecision::ReduceCore), (0, -2, 0));
    assert_eq!(sell_decision_ledger_delta(SellDecision::Hold), (0, 0, 0));
}

/// ★买侧 ThetaInstantiation delta 经对偶间接验证（Lean decisionLedgerDelta + buy_sell_A_delta_mirror）。
///
/// 买侧 Lean `decisionLedgerDelta`：openRoot.A=+1，accreteCore.A=+2（ThetaInstantiation.lean:257-258，
/// Lean machine-checked 常量）。本测试断言 rust 卖侧 delta 的 A 分量是买侧 Lean 期望的严格相反号
/// ——即 rust 实装满足 Lean `buy_sell_A_delta_mirror`（SellClosedLoop §4，已证）。这是买侧 Lean
/// delta 在 rust 端的**间接** bit-exact 交叉验证（买侧无 rust 独立实现，见模块头诚实边界）。
#[test]
fn lean_buy_side_delta_via_dual_mirror() {
    // 买侧 Lean decisionLedgerDelta 的 A 分量（来自 ThetaInstantiation.lean:257-258，machine-checked）。
    let lean_buy_open_root_a: i64 = 1; // decisionLedgerDelta openRoot = (0, 1, 0)
    let lean_buy_accrete_core_a: i64 = 2; // decisionLedgerDelta accreteCore = (0, 2, 0)
    // rust 卖侧 A 分量。
    let rust_sell_close_root_a = sell_decision_ledger_delta(SellDecision::CloseRoot).1; // -1
    let rust_sell_reduce_core_a = sell_decision_ledger_delta(SellDecision::ReduceCore).1; // -2
    assert_eq!(
        lean_buy_open_root_a, -rust_sell_close_root_a,
        "Lean 买侧 openRoot A=+1 ↔ rust 卖侧 closeRoot A=-1（buy_sell_A_delta_mirror）"
    );
    assert_eq!(
        lean_buy_accrete_core_a, -rust_sell_reduce_core_a,
        "Lean 买侧 accreteCore A=+2 ↔ rust 卖侧 reduceCore A=-2（buy_sell_A_delta_mirror）"
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
    let base_a: i64 = 5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = sell_transition(&x0, &lean_event_type1_dual());
    // 卖侧第一类：A = base - 1（对偶 Lean 买侧 chanlunTransition_type1_allocates 的 base + 1）。
    assert_eq!(x1.ledger_state.a, base_a - 1, "卖侧第一类清根仓 A = base - 1");
    // 卖侧第一类额外实现利润 Π += 1（Lean sellTransition_type1_realizes；买侧无 Π 变化的时间不对称）。
    assert_eq!(x1.ledger_state.pi, 1, "卖侧第一类实现利润 Π += 1");
    // 闭环保 R=Π-A-W（Lean chanlunTransition_preserves_ledger_inv / sellTransition_preserves_ledger_inv）。
    assert!(x1.ledger_state.inv_holds(), "闭环转移保 R=Π-A-W");
}

/// 把 Lean `chanlunTransition_type3_allocates`（买侧 A=base+2）对偶为卖侧 A=base-2 的逐态验证。
#[test]
fn lean_type3_transition_ledger_bit_exact() {
    let base_a: i64 = 5;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = sell_transition(&x0, &lean_event_type3_dual());
    // 卖侧第三类：A = base - 2（对偶 Lean 买侧 chanlunTransition_type3_allocates 的 base + 2）。
    assert_eq!(x1.ledger_state.a, base_a - 2, "卖侧第三类减核 A = base - 2");
    assert!(x1.ledger_state.inv_holds(), "闭环转移保 R=Π-A-W");
}

/// ★★非退化总见证 bit-exact（对齐 Lean `chanlun_transition_distinguishes_classes` :420）。
///
/// Lean 买侧：第一类 A=base+1 ≠ 第三类 A=base+2（两类缠论分类 ⟹ 不同 ledger 足迹，非平凡占位）。
/// 卖侧对偶：第一类 A=base-1 ≠ 第三类 A=base-2。本测试用 Lean eventType1/eventType3 对偶见证从
/// **同一**初始态跑 rust 闭环，断言两类产生不同 A delta——bit-exact 兑现 Lean 非退化结论。
#[test]
fn lean_transition_distinguishes_classes_bit_exact() {
    let base_a: i64 = 10;
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: base_a, w: 0, r: -base_a },
        ..AssemblyState::initial(1_000_000)
    };
    let a_type1 = sell_transition(&x0, &lean_event_type1_dual()).ledger_state.a; // base - 1 = 9
    let a_type3 = sell_transition(&x0, &lean_event_type3_dual()).ledger_state.a; // base - 2 = 8
    assert_ne!(
        a_type1, a_type3,
        "Lean chanlun_transition_distinguishes_classes：两类缠论分类 ⟹ 不同 ledger A delta"
    );
    // 逐值锚定（卖侧对偶具体值；买侧 Lean 为 base+1=11 / base+2=12）。
    assert_eq!(a_type1, base_a - 1, "第一类 A = base - 1");
    assert_eq!(a_type3, base_a - 2, "第三类 A = base - 2");
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
    assert_eq!(recog_chanlun_sell(&cont), SellDecision::Hold, "力度延续 ⟹ hold");
    assert_eq!(sell_decision_ledger_delta(SellDecision::Hold), (0, 0, 0), "hold delta (0,0,0)");
    // hold 闭环不改账本（Lean hold delta 全零 ⟹ ledger 恒等）。
    let x0 = AssemblyState {
        ledger_state: LedgerComp { i0: 1_000_000, pi: 3, a: 5, w: 1, r: -3 },
        ..AssemblyState::initial(1_000_000)
    };
    let x1 = sell_transition(&x0, &cont);
    assert_eq!(x1.ledger_state, x0.ledger_state, "hold 闭环不改 ledger（bit-exact (0,0,0)）");
}
