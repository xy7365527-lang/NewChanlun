//! Classifier bit-exact parity（acceptance #4，SG-2）——rust theta_v0 分类器 ↔ Lean Origin
//! 分类语义的**逐字段一致性证据**。
//!
//! ## 工位定位（SG-2，与 SG-1 `theta_v0_lean_parity.rs` 互斥不重叠）
//!
//! SG-1 核对的是 closed_loop 卖侧/transition 的 ledger delta 闭环（ThetaInstantiation/SellClosedLoop
//! 见证）。本文件核对的是 canonical §5 每级走势分类 `S^Chan=(τ,r,b,u)` 中**两个可严格判定的分量**
//! 相对 Lean Origin 分类语义的 bit-exact 一致：
//!
//! - **位置分量 r**（相对最后确认中枢）：rust `classifier::center::classify_position`
//!   ↔ Lean `Origin.CenterStates.classifyPosition`（CenterStates.lean:71-74）。
//! - **信号位 b 的第三类分量 B3/S3**：rust `classifier::bsp::{EndpointSituation,endpoint_to_bsp}`
//!   + `classifier::signal::extract_signals` ↔ Lean `Origin.BspClassification.{IsType3Buy,IsType3Sell}`
//!   （BspClassification.lean:111-121）。
//!
//! ## ★诚实覆盖标注（formalization-validity-domain 231号，强制）
//!
//! 本工位的 b 向量 parity 覆盖 = **B3/S3 only**（第三类买/卖）。
//! 第一类（B1/S1）、第二类（B2/S2）的 Lean↔Rust parity **不在本工位**（属 SG-5）——原因是诚实的
//! 结构边界，非遗漏：rust `signal::extract_signals`（classifier 顶层信号入口）当前**只产第三类**
//! （signal.rs:11-26 模块头声明 v0 只提取第三类，第一/二类需趋势确认时序/次级别递归，未实装）。
//! 因此 rust 在「从 confirmed 结构提取信号」这一端到端路径上**不存在** B1/B2 的输出，对其做
//! Lean↔Rust 端到端 parity 无对照标的。bsp.rs 的 `is_first/is_second` 谓词存在但 signal.rs 不
//! 调用它们产信号——故本工位核对 bsp.rs 第三类谓词，不核对一/二类谓词的端到端 parity。
//!
//! ## ★边界口径发现（no-workaround：定义层不一致，已显式记录非迁就）
//!
//! 第三类「不破 ZG/ZD」的**边界临界点** `retest == zg`（或 `retest == zd`）上，rust 与 Lean 口径相反：
//! - reference §36 原文：`回试低点 ≥ZG = 3买`（**含等号**，标注「等号允许设计选择」，docs/reference-theta-v0.md:36）。
//! - rust `signal::extract_signals`（signal.rs:71）：`retest.price >= c.zg`（**含等号**）——逐字符合 reference §36。
//! - Lean `IsType3Buy`（BspClassification.lean:113）：`e.center.zg < e.retracePrice`（**严格 >**，等号**排除**）——与 reference §36 相反。
//!
//! 本文件**不**写测试去迁就任一侧让其变绿：把临界点行为编码为显式记录矛盾的 `boundary_*` 测试，
//! 断言 rust **现状** = reference §36 含等号口径，并在断言旁标注 Lean 严格口径与之冲突（待 SG/Lead
//! 走矛盾上浮决定哪侧为准）。内部安全点（retest 严格大于 zg）两侧一致，正常 bit-exact 绿。

use newchan_rust::theta_v0::classifier::bsp::{endpoint_to_bsp, EndpointSituation};
use newchan_rust::theta_v0::classifier::center::{classify_position, RelativePosition};
use newchan_rust::theta_v0::classifier::signal::extract_signals;
use newchan_rust::theta_v0::types::{Center, Direction, Segment, Tick};

// ════════════════════════════════════════════════════════════════════════════
//  层 A — 位置分量 r：classify_position ↔ Lean Origin.CenterStates.classifyPosition
//
//  Lean classifyPosition (CenterStates.lean:71-74)：
//    if p < c.zd then below else if c.zg < p then above else within
//  Lean 见证 (CenterStates.lean:403-411，sampleCenter 核心 [10,20])：
//    classifyPosition core 5  = below   (5 < zd=10)
//    classifyPosition core 15 = within  (10 ≤ 15 ≤ 20)
//    classifyPosition core 30 = above   (20 < 30)
//  Lean 闭区间边界 (position_boundary_closed_core_within，CenterStates.lean:384-389)：
//    p = zd → within；p = zg → within（闭核心区间 [zd,zg]）。
// ════════════════════════════════════════════════════════════════════════════

/// 构造中枢（外缘字段不影响位置判据——classifyPosition 只用 zd/zg）。
fn center(zd: Tick, zg: Tick) -> Center {
    Center { zd, zg, dd: zd - 5, gg: zg + 5, start_index: 0, end_index: 0 }
}

/// r 维度 6 个语义值对照：本工位任务列出 r∈{⊥,I,U⁰,U¹,D⁰,D¹}（canonical §5 含买卖侧细分），
/// 但 rust `RelativePosition` 与 Lean `CenterPosition` 都是**位置三态**（below/within/above）——
/// canonical 的 U⁰/U¹/D⁰/D¹ 细分（第几类买卖位）由 b 向量承载，不由位置态承载。位置态本身是
/// 三态，故此处穷举三态在 Lean sampleCenter 见证数值上的 bit-exact 对照。
#[test]
fn position_below_within_above_matches_lean_witness() {
    // 逐字段对照 Lean witness_below/within/above（CenterStates.lean:403-411，核心 [10,20]）。
    let c = center(10, 20);
    assert_eq!(classify_position(&c, 5), RelativePosition::Below, "Lean witness_below: 5<zd=10");
    assert_eq!(classify_position(&c, 15), RelativePosition::Within, "Lean witness_within: 10≤15≤20");
    assert_eq!(classify_position(&c, 30), RelativePosition::Above, "Lean witness_above: 20<30");
}

#[test]
fn position_closed_core_boundary_matches_lean() {
    // 逐字段对照 Lean position_boundary_closed_core_within（CenterStates.lean:384-389）：
    // p=zd 与 p=zg 都归 within（闭核心区间，Lean 严格不等式判据 p<zd / zg<p）。
    let c = center(10, 20);
    assert_eq!(classify_position(&c, 10), RelativePosition::Within, "Lean: p=zd → within（闭区间）");
    assert_eq!(classify_position(&c, 20), RelativePosition::Within, "Lean: p=zg → within（闭区间）");
}

#[test]
fn position_total_and_exclusive_matches_lean() {
    // 对照 Lean position_total + position_disjoint（CenterStates.lean:80-95）：任一点恰落一态。
    // 穷举一段整数点，验证 rust 判定与 Lean 三态定义（p<zd / zd≤p≤zg / zg<p）逐点一致。
    let c = center(0, 10);
    for p in -20..=30 {
        let r = classify_position(&c, p);
        // Lean 谓词定义（IsBelow/IsWithin/IsAbove，CenterStates.lean:64-68）。
        let lean_below = p < c.zd;
        let lean_above = c.zg < p;
        let lean_within = c.zd <= p && p <= c.zg;
        // 恰一态成立（Lean position_total ∧ position_disjoint）。
        let count = [lean_below, lean_within, lean_above].iter().filter(|&&b| b).count();
        assert_eq!(count, 1, "p={p}: Lean 三态恰一成立");
        match r {
            RelativePosition::Below => assert!(lean_below, "p={p}: rust=Below ⟺ Lean IsBelow"),
            RelativePosition::Within => assert!(lean_within, "p={p}: rust=Within ⟺ Lean IsWithin"),
            RelativePosition::Above => assert!(lean_above, "p={p}: rust=Above ⟺ Lean IsAbove"),
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  层 B1 — 第三类判据（bsp 谓词层）：endpoint_to_bsp ↔ Lean IsType3Buy/IsType3Sell 的结构合取
//
//  Lean IsType3Buy (BspClassification.lean:111-113)：
//    side=long ∧ leftCenter=true ∧ firstRetrace=true ∧ center.zg < retracePrice
//  rust 把价位条件 (center.zg < retracePrice) 折叠为布尔 retrace_not_reenter，由 signal.rs 上游计算；
//  bsp.rs is_third = leftCenter ∧ retrace_not_reenter（结构合取，价位已折叠）。
//  Lean 见证 x_2b3b (BspClassification.lean:172-186)：leftCenter=true, retracePrice=25 > zg=20
//    ⟹ x_2b3b_is_type3（IsType3Buy 成立，BspClassification.lean:185-186）。
// ════════════════════════════════════════════════════════════════════════════

/// 端点语义构造（对齐 bsp.rs::EndpointSituation 字段）。
fn situ(left: bool, retrace_ok: bool, sell: bool) -> EndpointSituation {
    EndpointSituation {
        after_first_buy: false,
        is_pullback_end: false,
        left_center: left,
        retrace_not_reenter: retrace_ok,
        below_last_center: false,
        is_sell_side: sell,
    }
}

#[test]
fn type3_buy_predicate_matches_lean_x_2b3b() {
    // 对照 Lean x_2b3b_is_type3（BspClassification.lean:185）：leftCenter=true ∧ 不破ZG ⟹ 第三类买。
    // rust 侧：left_center=true ∧ retrace_not_reenter=true（价位折叠后）⟹ is_third ⟹ buy3=1。
    let e = situ(true, true, false);
    let bits = endpoint_to_bsp(&e);
    assert!(bits.buy3, "Lean IsType3Buy 成立 ⟺ rust buy3 置位");
    assert!(!bits.sell3, "买侧 ⟹ sell3 不置位");
}

#[test]
fn type3_sell_predicate_mirrors_lean() {
    // 对照 Lean IsType3Sell（BspClassification.lean:119-121）：side=short ∧ leftCenter ∧ retrace<zd。
    // rust 镜像：is_sell_side=true ∧ left_center ∧ retrace_not_reenter ⟹ sell3=1。
    let e = situ(true, true, true);
    let bits = endpoint_to_bsp(&e);
    assert!(bits.sell3, "Lean IsType3Sell 成立 ⟺ rust sell3 置位");
    assert!(!bits.buy3, "卖侧 ⟹ buy3 不置位");
}

#[test]
fn type3_buy_rejected_when_reenter_matches_lean() {
    // 对照 Lean type3Buy_rejects_reenter（BspClassification.lean:152-158）：retrace 破 ZG（重入中枢）
    // ⟹ ¬IsType3Buy。rust 侧 retrace_not_reenter=false（价位折叠为「破」）⟹ is_third=false ⟹ buy3=0。
    let e = situ(true, false, false);
    let bits = endpoint_to_bsp(&e);
    assert!(!bits.buy3, "Lean ¬IsType3Buy（回抽破ZG重入）⟺ rust buy3 不置位");
}

#[test]
fn type3_requires_left_center_matches_lean() {
    // Lean IsType3Buy 要求 leftCenter=true（BspClassification.lean:112）。未离开中枢 ⟹ ¬第三类。
    let e = situ(false, true, false);
    let bits = endpoint_to_bsp(&e);
    assert!(!bits.buy3, "Lean leftCenter=false ⟹ ¬IsType3Buy ⟺ rust buy3 不置位");
}

// ════════════════════════════════════════════════════════════════════════════
//  层 B2 — 第三类信号提取（价位比较层）：signal::extract_signals 的价位折叠
//          ↔ Lean IsType3Buy/IsType3Sell 的价位比较
//
//  内部安全点（retest 严格大于 zg / 严格小于 zd）：rust 与 Lean 一致，正常 bit-exact 绿。
//  Lean eventType3 见证 (ThetaInstantiation.lean:376-387)：center.zg=20, retracePrice=25（25>20 严格）。
// ════════════════════════════════════════════════════════════════════════════

fn seg(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Segment {
    Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
}

/// 内部安全点：回试低点严格大于 ZG（Lean eventType3 见证口径 retracePrice=25 > zg=20）。
/// 此点 rust（>=）与 Lean（严格 >）两侧一致 ⟹ bit-exact 绿。
#[test]
fn type3_buy_interior_point_matches_lean() {
    // 中枢核心 [100,200] end_index=12。向上线段离开（端点 250 > zg=200），向下回试低点 210（严格 > 200）。
    // Lean IsType3Buy: center.zg=200 < retracePrice=210 ⟹ 成立（内部安全点，非边界）。
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),   // 离开：端点 250 > zg=200
        seg(Direction::Down, 16, 20, 250, 210), // 回试低点 210 > zg=200（严格，两侧一致）
    ];
    let points = extract_signals(&[c], &segs);
    assert_eq!(points.len(), 1, "离开+回试不破 ⟹ 一个第三类买点（rust 与 Lean 一致）");
    assert!(points[0].bits.buy3, "内部安全点 retest>zg：rust 与 Lean IsType3Buy 同为真");
    assert_eq!(points[0].center.map(|c| c.zg), Some(200), "3 买止损=ZG（BspClassification 止损语义）");
}

/// 内部安全点：回试明确破 ZG（210→190，190 < zg=200）。Lean ¬IsType3Buy，rust 也 ¬buy3 ⟹ 一致。
#[test]
fn type3_buy_clear_reenter_matches_lean() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),
        seg(Direction::Down, 16, 20, 250, 190), // 回试 190 < zg=200（明确重入）
    ];
    let points = extract_signals(&[c], &segs);
    assert!(points.is_empty(), "回试明确破 ZG ⟹ rust 与 Lean 同为 ¬第三类");
}

// ─────────────────────────────────────────────────────────────────────────────
//  ★边界口径矛盾（no-workaround：显式记录，非迁就）
//
//  临界点 retest == zg：reference §36 含等号（≥ZG=3买）；rust signal.rs `>=` 与之一致；
//  Lean IsType3Buy `zg < retracePrice` 严格排除等号，与 reference §36 相反。
//  本测试断言 rust **现状** = reference §36 含等号口径（rust 正确忠实 spec），并标注 Lean 冲突。
//  这不是 rust bug——rust 逐字符合 reference §36；矛盾在 Lean IsType3Buy 的严格 `<` 偏离了 spec
//  的「等号允许」。待矛盾上浮裁决统一口径（改 Lean 为 ≤/< 含等号 或 reference 改为不含等号）。
// ─────────────────────────────────────────────────────────────────────────────

/// 边界临界点（买侧）：回试低点 **恰等于** ZG（retest == zg）。
/// - rust 现状（signal.rs `retest.price >= c.zg`）：算第三类（buy3=1）——合 reference §36「≥ZG」。
/// - Lean IsType3Buy（`zg < retracePrice`，严格）：**不**算第三类——与 reference §36 相反。
/// 本测试锁定 rust 现状 = reference §36 口径；rust 与 Lean 在此临界点结论相反（记录矛盾）。
#[test]
fn boundary_retest_eq_zg_rust_follows_reference_lean_diverges() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),
        seg(Direction::Down, 16, 20, 250, 200), // 回试 == zg=200（边界临界点）
    ];
    let points = extract_signals(&[c], &segs);
    // rust 现状：含等号口径 ⟹ 算第三类（合 reference §36「≥ZG 等号允许设计选择」）。
    assert_eq!(points.len(), 1, "rust 现状（>=）：retest==zg 算第三类（合 reference §36）");
    assert!(points[0].bits.buy3, "rust buy3 置位");
    // ★矛盾记录：Lean IsType3Buy（严格 zg<retracePrice）在 retest==zg 时为假——
    //   若 rust 改用 Lean 严格口径，本断言的 points.len() 应为 0。两侧在此临界点结论相反。
    //   见模块头「边界口径发现」——待矛盾上浮裁决（reference §36 含等号 vs Lean 严格 <）。
}

/// 边界临界点（卖侧镜像）：回抽高点 **恰等于** ZD（retest == zd）。
/// - rust 现状（signal.rs `retest.price <= c.zd`）：算第三类（sell3=1）——合 reference §36「≤ZD」。
/// - Lean IsType3Sell（`retracePrice < center.zd`，严格）：**不**算——与 reference §36 相反。
#[test]
fn boundary_retest_eq_zd_rust_follows_reference_lean_diverges() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Down, 12, 16, 150, 50), // 离开：端点 50 < zd=100
        seg(Direction::Up, 16, 20, 50, 100),   // 回抽 == zd=100（边界临界点）
    ];
    let points = extract_signals(&[c], &segs);
    assert_eq!(points.len(), 1, "rust 现状（<=）：retest==zd 算第三类（合 reference §36）");
    assert!(points[0].bits.sell3, "rust sell3 置位");
    // ★矛盾记录：Lean IsType3Sell（严格 retracePrice<zd）在 retest==zd 时为假——两侧结论相反。
}

// ════════════════════════════════════════════════════════════════════════════
//  诚实覆盖确认（B3/S3 only，第一/二类不在本工位）
// ════════════════════════════════════════════════════════════════════════════

/// ★诚实标注（编码为可执行断言）：signal::extract_signals（classifier 顶层信号入口）当前只产
/// 第三类——对任意 confirmed 结构输入，产出的 BspBits 中 buy1/buy2/sell1/sell2 恒为假。
/// 故第一/二类的 Lean↔Rust 端到端 parity **无对照标的**（属 SG-5）。本测试锁定该覆盖边界。
#[test]
fn signal_extraction_emits_third_class_only() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),
        seg(Direction::Down, 16, 20, 250, 210),
    ];
    let points = extract_signals(&[c], &segs);
    for p in &points {
        assert!(!p.bits.buy1 && !p.bits.buy2, "signal v0 不产第一/二类买点（覆盖=B3 only，SG-5 域）");
        assert!(!p.bits.sell1 && !p.bits.sell2, "signal v0 不产第一/二类卖点（覆盖=S3 only，SG-5 域）");
    }
    assert!(points.iter().any(|p| p.bits.buy3 || p.bits.sell3), "本输入确产第三类（覆盖标的存在）");
}
