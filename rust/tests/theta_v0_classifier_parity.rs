//! Classifier bit-exact parity（acceptance #4，SG-2）——rust theta_v0 分类器 ↔ Lean Origin
//! 分类语义的**逐字段一致性证据**。
//!
//! ## 工位定位（SG-2，与 SG-1 `theta_v0_lean_parity.rs` 互斥不重叠——SG-1 已于 #181 随卖侧
//! SellDecision 死路径下线删除）
//!
//! SG-1 曾核对 closed_loop 卖侧/transition 的 ledger delta 闭环（ThetaInstantiation/SellClosedLoop
//! 见证；rust 卖侧 port 删除后，Lean `Origin/SellClosedLoop.lean` 形式化留存于 formal/）。本文件核对的是 canonical §5 每级走势分类 `S^Chan=(τ,r,b,u)` 中**两个可严格判定的分量**
//! 相对 Lean Origin 分类语义的 bit-exact 一致：
//!
//! - **位置分量 r**（相对最后确认中枢）：rust `classifier::center::classify_position`
//!   ↔ Lean `Origin.CenterStates.classifyPosition`（CenterStates.lean:71-74）。
//! - **信号位 b 的第一/三类分量 B1/S1 + B3/S3**：rust `classifier::bsp::{EndpointSituation,
//!   endpoint_to_bsp}` + `classifier::signal::extract_signals` ↔ Lean `Origin.BspClassification.
//!   {IsType1,IsType3Buy,IsType3Sell}`（BspClassification.lean:94-95,111-121）。
//! - **信号位 b 的第二类分量 B2/S2**：rust `classifier::signal::extract_second_signals`（递归组装层）
//!   ↔ Lean `Origin.RMoveCompose.{SecondTypeStructure,secondPointPrice}`（RMoveCompose.lean:197-215）。
//!
//! ## ★诚实覆盖标注（formalization-validity-domain 231号，强制）
//!
//! 本工位的 b 向量 parity 覆盖 = **B1/S1 + B3/S3 + B2/S2**（三类全买/卖）。
//! - **B3/S3**（第三类）：`extract_signals` 纯整数几何（离开后回试不破 ZG/ZD）↔ Lean
//!   `IsType3Buy/IsType3Sell`，**L0** bit-exact（层 B1/B2）。
//! - **B1/S1**（第一类）：`extract_signals` 破中枢几何（**L0**）∧ MACD 背驰真算（**L1**）↔ Lean
//!   `IsType1 = brokeCenter ∧ IsDivergence`（层 C）。★rust 领先 Origin：Lean `divPair` 是外部参数
//!   （still-MISSING-C 无 MACD 引擎），rust `divergence.rs` 已实装 MACD ⟹ 背驰分量真算非占位。
//! - **B2/S2**（第二类，#52 递归组装层）：`extract_second_signals` 消费 RMove 递归塔的
//!   `SecondTypeStructure`（第一类离开 m1 + 回拉 m2 不创新低/新高 + i1<i2）↔ Lean
//!   `Origin.RMoveCompose.SecondTypeStructure` + `secondPointPrice`（层 D，见 §层 D）。**L0/L1**
//!   bit-exact（结构层 L0，第一类离开背驰经 MACD = L1）。★诚实区分（与 B1/B3 不同）：B2/S2 的输入
//!   是 **RMove 递归塔**（非 L0 segment）——`extract_signals`（L0 签名层）不产 B2/S2
//!   （`signal_extraction_emits_no_second_class_only` 锁定该边界），按输入对象分工，非缺口。
//!   ★still-MISSING（坐标/塔）：RMove 无 source_index（坐标由 index_of 闭包提供），且 mod.rs 生产
//!   路径未构造 RMove 塔（still-MISSING-塔，上游接入），见 signal.rs 模块头。
//!
//! 「MACD 背驰预测在真实行情有效」（L2/L3 否证检验）**不在本工位**——本工位证 IsType1 结构合取
//! bit-exact（L1 管线正确性），非背驰预测的经验有效性。
//!
//! ## ★边界口径对齐（codex 裁决 2026-06-27：定义层矛盾已消解，三处统一严格口径）
//!
//! 第三类「不破 ZG/ZD」的**边界临界点** `retest == zg`（或 `retest == zd`）上，曾存在 rust/reference
//! 含等号 vs Lean 严格的口径矛盾（SG-2 §36 口径 BLOCKED）。codex 裁决统一为**严格口径**：
//! - **裁决依据**：第三类买点是**中枢终结点**；项目结算中枢=闭区间 `[ZD,ZG]` + 中心定理一（与
//!   `[ZD,ZG]` 重叠=中枢延伸，仅 `dn>ZG`/`gn<ZD` 才离开）。故 `retest == zg`=单点重叠=仍触及
//!   闭区间中枢=**非中枢终结=非第三类**。第21课「回抽不触及该中枢」支持。严格口径**正确**（Lean 严格<正确）。
//! - Lean `IsType3Buy`（BspClassification.lean:113）：`e.center.zg < e.retracePrice`（严格 `<`）——**正确，不动**。
//! - legacy `buysellpoint.rs:414`：`pullback_seg.low > zs.zg`（严格 `>`）——**已严格，不动**。
//! - rust `signal::extract_signals`（signal.rs:71）：原 `>= c.zg`（含等号，错口径）→ **已改严格 `> c.zg`**，对齐 Lean。
//! - reference §36：原 `≥ZG`（含等号，错口径）→ **已改严格 `>ZG`**，对齐 Lean。
//!
//! ⟹ 边界临界点 `retest == zg` 上 rust 与 Lean **现已一致**（两侧同判非第三类）。本文件 `boundary_*`
//! 测试断言 rust 严格口径 == Lean 严格口径（不再记录矛盾）。内部安全点（retest 严格大于 zg）两侧本就一致。

use newchan_rust::theta_v0::classifier::bsp::{endpoint_to_bsp, EndpointSituation};
use newchan_rust::theta_v0::classifier::center::{classify_position, RelativePosition};
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::rmove_compose::compose_move;
use newchan_rust::theta_v0::classifier::signal::{extract_second_signals, extract_signals, BspPoint};
use newchan_rust::theta_v0::config::MacdConfig;
use newchan_rust::theta_v0::types::{Center, Direction, Segment, Side, Tick};
use serde::Deserialize;

/// 第三类 parity 专用入口：传空 closes（第三类纯整数几何，不依赖 MACD）——第一类自然不产。
/// 第三类 Lean↔rust bit-exact 对照只核对几何判据，MACD 无关。
fn extract_third_only(centers: &[Center], segments: &[Segment]) -> Vec<BspPoint> {
    extract_signals(centers, segments, &[], &[], &MacdConfig::default())
}

// ════════════════════════════════════════════════════════════════════════════
//  §0 Lean #eval 导出 fixture 机器耦合（631 兑现，消手工转录漂移）
//
//  位置见证（classifyPosition sampleCenter.core 5/15/30 = below/within/above）由
//  `formal/Origin/ParityFixtureExport.lean` 的 #eval **机器导出**（Lean 真求值 classifyPosition），
//  序列化为 JSON。本文件用 include_str! 读 fixture 的 position 段，断言 rust classify_position 在
//  同一中枢同一价位产出 == fixture 中 Lean 导出的位置态。bit-exact 等级 = L0。
// ════════════════════════════════════════════════════════════════════════════

/// fixture JSON 中本文件 classifier 消费的子集（serde 镜像 ParityFixtureExport.fixtureJson）。
#[derive(Deserialize)]
struct ParityFixture {
    position: PositionFixture,
    buy_witness: BuyWitness,
}

#[derive(Deserialize)]
struct PositionFixture {
    at_5: String,
    at_15: String,
    at_30: String,
    zd: Tick,
    zg: Tick,
}

#[derive(Deserialize)]
struct BuyWitness {
    type3_retrace_price: Tick,
    type3_center_zg: Tick,
}

fn load_fixture() -> ParityFixture {
    let raw = include_str!("fixtures/theta_v0_parity.json");
    serde_json::from_str(raw).expect("fixture 必须是 Lean #eval 导出的合法 JSON")
}

/// Lean 导出的 CenterPosition 枚举名字符串 → rust RelativePosition（期望值由 fixture 决定，非手填）。
fn position_from_lean(s: &str) -> RelativePosition {
    match s {
        "below" => RelativePosition::Below,
        "within" => RelativePosition::Within,
        "above" => RelativePosition::Above,
        other => panic!("fixture 出现未知位置态名「{other}」——Lean 枚举与 rust 镜像漂移"),
    }
}

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
    // 机器耦合：中枢边界 zd/zg 与三态期望从 fixture 读（Lean #eval classifyPosition sampleCenter.core
    // 5/15/30），非手填。Lean 改 sampleCenter 或 classifyPosition ⟹ fixture 变 ⟹ 本测试随之变。
    let fx = load_fixture();
    let c = center(fx.position.zd, fx.position.zg); // fixture: sampleCenter.core [zd=10, zg=20]
    assert_eq!(
        classify_position(&c, 5),
        position_from_lean(&fx.position.at_5),
        "rust classify_position(c,5) == Lean #eval classifyPosition sampleCenter.core 5"
    );
    assert_eq!(
        classify_position(&c, 15),
        position_from_lean(&fx.position.at_15),
        "rust classify_position(c,15) == Lean #eval classifyPosition sampleCenter.core 15"
    );
    assert_eq!(
        classify_position(&c, 30),
        position_from_lean(&fx.position.at_30),
        "rust classify_position(c,30) == Lean #eval classifyPosition sampleCenter.core 30"
    );
}

#[test]
fn position_closed_core_boundary_matches_lean() {
    // 逐字段对照 Lean position_boundary_closed_core_within（CenterStates.lean:384-389）：
    // p=zd 与 p=zg 都归 within（闭核心区间，Lean 严格不等式判据 p<zd / zg<p）。
    // 中枢边界 zd/zg 从 fixture 读（Lean sampleCenter.core），非手填 10/20。
    let fx = load_fixture();
    let c = center(fx.position.zd, fx.position.zg);
    assert_eq!(classify_position(&c, fx.position.zd), RelativePosition::Within, "Lean: p=zd → within（闭区间）");
    assert_eq!(classify_position(&c, fx.position.zg), RelativePosition::Within, "Lean: p=zg → within（闭区间）");
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

/// ★Lean 见证尺度 bit-exact（机器耦合）：直接用 fixture 导出的 Lean eventType3 见证值
/// （center.zg / retracePrice）跑 extract_signals，断言 rust 在 Lean 见证原始数值上产第三类买点。
/// 这是真正的 Lean 见证 parity（zg=20 < retrace=25 严格）——zg/retrace 从 fixture 读，非手填。
#[test]
fn type3_buy_lean_witness_scale_matches_fixture() {
    let fx = load_fixture();
    let zg = fx.buy_witness.type3_center_zg;       // fixture: Lean eventType3.bsp.center.zg = 20
    let retrace = fx.buy_witness.type3_retrace_price; // fixture: Lean eventType3.bsp.retracePrice = 25
    assert!(retrace > zg, "fixture 自洽前置：Lean eventType3 retrace 严格 > zg（不破 ZG）");
    // 用 Lean 见证尺度构造中枢与离开+回试线段：zd 任取低于 zg 的合法值，离开端点 > zg，回试 == retrace。
    let c = Center { zd: zg - 10, zg, dd: zg - 15, gg: zg + 5, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, zg - 5, zg + 30),      // 离开：端点 > zg
        seg(Direction::Down, 16, 20, zg + 30, retrace),   // 回试低点 = Lean retracePrice（> zg 严格）
    ];
    let points = extract_third_only(&[c], &segs);
    assert_eq!(points.len(), 1, "Lean 见证尺度 retrace>zg ⟹ 一个第三类买点（rust 与 Lean IsType3Buy 一致）");
    assert!(points[0].bits.buy3, "Lean eventType3 见证：rust 与 Lean IsType3Buy 同为真");
    // （#218 面 A 载体形态机械适配：Center 变体读出。）
    assert_eq!(
        points[0].center.and_then(|o| match o { newchan_rust::theta_v0::classifier::bsp::OwnerRef::Center(c) => Some(c.zg), _ => None }),
        Some(zg),
        "3 买止损=ZG（= Lean center.zg）"
    );
}

/// 内部安全点（scaled 路径覆盖，非 Lean 见证值）：回试低点严格大于 ZG。此处 zg=200/retrace=210 是
/// **测试自造的更大尺度**（覆盖 extract_signals 信号路径），不是 Lean eventType3 见证（zg=20/retrace=25，
/// 后者的 bit-exact 耦合见 type3_buy_lean_witness_scale_matches_fixture）。两尺度同判第三类成立。
#[test]
fn type3_buy_interior_point_matches_lean() {
    // 中枢核心 [100,200] end_index=12。向上线段离开（端点 250 > zg=200），向下回试低点 210（严格 > 200）。
    // 严格口径下 center.zg=200 < retracePrice=210 ⟹ 第三类成立（内部安全点，非边界）。
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),   // 离开：端点 250 > zg=200
        seg(Direction::Down, 16, 20, 250, 210), // 回试低点 210 > zg=200（严格，两侧一致）
    ];
    let points = extract_third_only(&[c], &segs);
    assert_eq!(points.len(), 1, "离开+回试不破 ⟹ 一个第三类买点（rust 与 Lean 一致）");
    assert!(points[0].bits.buy3, "内部安全点 retest>zg：rust 与 Lean IsType3Buy 同为真");
    // （#218 面 A 载体形态机械适配：Center 变体读出。）
    assert_eq!(
        points[0].center.and_then(|o| match o { newchan_rust::theta_v0::classifier::bsp::OwnerRef::Center(c) => Some(c.zg), _ => None }),
        Some(200),
        "3 买止损=ZG（BspClassification 止损语义）"
    );
}

/// 内部安全点：回试明确破 ZG（210→190，190 < zg=200）。Lean ¬IsType3Buy，rust 也 ¬buy3 ⟹ 一致。
#[test]
fn type3_buy_clear_reenter_matches_lean() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),
        seg(Direction::Down, 16, 20, 250, 190), // 回试 190 < zg=200（明确重入）
    ];
    let points = extract_third_only(&[c], &segs);
    assert!(points.is_empty(), "回试明确破 ZG ⟹ rust 与 Lean 同为 ¬第三类");
}

// ─────────────────────────────────────────────────────────────────────────────
//  ★边界口径对齐（codex 裁决 2026-06-27：rust 严格 == Lean 严格，矛盾已消解）
//
//  临界点 retest == zg：第三类买点=中枢终结点；retest==zg=单点重叠=仍触及闭区间中枢 [ZD,ZG]
//  =中枢延伸（中心定理一）=非终结=非第三类。严格口径正确（第21课「回抽不触及该中枢」支持）。
//  rust signal.rs 原 `>=`（含等号，错）已改严格 `>`，与 Lean IsType3Buy（`zg < retracePrice` 严格）
//  现一致；reference §36 同步改严格 `>ZG`。本测试断言 rust 严格 == Lean 严格（不再记录矛盾）。
// ─────────────────────────────────────────────────────────────────────────────

/// 边界临界点（买侧）：回试低点 **恰等于** ZG（retest == zg）。
/// - rust（signal.rs `retest.price > c.zg`，严格）：**不**算第三类（buy3=0）。
/// - Lean IsType3Buy（`zg < retracePrice`，严格）：**不**算第三类。
/// 两侧在此临界点结论**一致**（同判非第三类）——矛盾已由 codex 裁决消解。
#[test]
fn boundary_retest_eq_zg_rust_aligned_to_lean_strict() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Up, 12, 16, 150, 250),
        seg(Direction::Down, 16, 20, 250, 200), // 回试 == zg=200（边界临界点）
    ];
    let points = extract_third_only(&[c], &segs);
    // rust 严格口径（>）⟹ retest==zg 触及闭区间中枢=非终结=非第三类。
    // 对齐 Lean IsType3Buy（严格 zg<retracePrice 在 retest==zg 时为假）⟹ 两侧同判 points 空。
    assert!(points.is_empty(), "rust 严格（>）：retest==zg 触及中枢=非第三类，对齐 Lean IsType3Buy 严格口径");
}

/// 边界临界点（卖侧镜像）：回抽高点 **恰等于** ZD（retest == zd）。
/// - rust（signal.rs `retest.price < c.zd`，严格）：**不**算第三类（sell3=0）。
/// - Lean IsType3Sell（`retracePrice < center.zd`，严格）：**不**算第三类。
/// 两侧一致——矛盾已由 codex 裁决消解。
#[test]
fn boundary_retest_eq_zd_rust_aligned_to_lean_strict() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 12 };
    let segs = vec![
        seg(Direction::Down, 12, 16, 150, 50), // 离开：端点 50 < zd=100
        seg(Direction::Up, 16, 20, 50, 100),   // 回抽 == zd=100（边界临界点）
    ];
    let points = extract_third_only(&[c], &segs);
    // rust 严格口径（<）⟹ retest==zd 触及闭区间中枢=非终结=非第三类，对齐 Lean IsType3Sell 严格口径。
    assert!(points.is_empty(), "rust 严格（<）：retest==zd 触及中枢=非第三类，对齐 Lean IsType3Sell 严格口径");
}

// ════════════════════════════════════════════════════════════════════════════
//  层 C — 第一类（B1/S1）：extract_signals 破中枢几何 L0 ∧ MACD 背驰 L1 真算
//          ↔ Lean Origin.BspClassification.IsType1 = brokeCenter ∧ IsDivergence(divPair)
//
//  Lean IsType1 (BspClassification.lean:94-95)：brokeCenter=true ∧ IsDivergence divPair。
//  Lean IsDivergence (Divergence.lean:83)：d.forceC.area < d.forceA.area（后段力度严格小于前段）。
//  rust 侧：破中枢=段端点出中枢核心（L0 整数几何，对齐 descend.rs sub_broke_below）；
//  IsDivergence=破中枢段 vs 前同向段 MACD 面积严格变小（divergence::segments_diverge 真算，L1）。
//  ★rust 领先 Origin：Lean divPair 是外部参数（still-MISSING-C 无 MACD 引擎），rust MACD 已实装。
// ════════════════════════════════════════════════════════════════════════════

/// closes + close_src 辅助（连续 source_index，无合并跳跃）。
fn closes_seq(vals: &[Tick]) -> (Vec<f64>, Vec<usize>) {
    let c: Vec<f64> = vals.iter().map(|&v| v as f64).collect();
    let src: Vec<usize> = (0..vals.len()).collect();
    (c, src)
}

/// 第一类买点 Lean↔rust parity：趋势背驰（A/B/C 框架）⟹ rust buy1 置位，与 Lean IsType1 同。
///
/// ★fixture 对齐 tau 框架权威（divergence.rs trend_class，b8667b968f 收紧 B1=趋势背驰 A/B/C 框架）：
/// 旧 fixture 用单中枢（Consolidation τ）产 B1 是退化语义——tau 门控下盘整背驰**不产第一类**
/// （beichi #4 + maimai.md:56 已结算）。现对齐 `signal.rs:680-711` 的 `[c0, c1]` 两全链同向向下
/// 中枢模式（c1.gg < c0.dd ⟹ 下跌趋势 τ=Trend(Down)），B1=趋势背驰（C 段破最后中枢 ∧ C<A 面积）。
/// Lean IsType1 = brokeCenter ∧ IsDivergence（divPair 外参，rust MACD 真算 forceC.area < forceA.area）。
#[test]
fn type1_buy_broke_and_diverge_matches_lean_istype1() {
    // 两依次向下中枢（下跌趋势 τ=Trend(Down)），对齐 signal.rs:687-705 `first_buy_extracted_with_trend_divergence`。
    // c1.gg=210 < c0.dd=290 ⟹ 下跌延续 ⟹ Trend(Down) ⟹ B1 可产（趋势背驰，非盘整背驰）。
    let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
    let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
    let segs = vec![
        seg(Direction::Down, 3, 5, 350, 250),   // A 段：C0 离开段（破 C0 下沿），MACD 面积大（强势=Lean forceA）
        seg(Direction::Up, 5, 7, 250, 280),     // B 段：中间反向连接（构成 C1）
        seg(Direction::Down, 9, 11, 150, 80),   // C 段：破 C1 下沿（< 100）∧ C<A 趋势背驰 ⟹ B1（=Lean forceC < forceA）
    ];
    // closes（merged_bars 序列，与 segment 抽象端点价解耦——结构判定在 tick 域，MACD 在浮点域，两者
    // 不必逐 bar 一致）：A 段 bar[3,5] 急跌 hist 大=强力度（forceA.area 大），B 段 bar[5,7] 盘整让 EMA 收敛，
    // C 段 bar[9,11] 缓动 hist 小=力度衰减=趋势背驰（手算 A area[3,5]=23.26 > C area[9,11]=9.09 ⟹ C<A 成立）。
    let (closes, src) = closes_seq(&[
        300, 300, 300,        // 0..2 C0 区（预热）
        300, 100, 250,        // 3..5 A 段：急跌（hist 绝对值大=强力度）
        250, 250, 250,        // 6..8 B 段：盘整让 EMA 收敛（hist 回拉 0 轴）
        248, 246, 244,        // 9..11 C 段：缓动（hist 小=力度衰减=趋势背驰）
    ]);
    let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
    let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
    assert_eq!(buy1.len(), 1, "Lean IsType1（brokeCenter ∧ IsDivergence）⟺ rust buy1 置位");
    assert_eq!(buy1[0].pivot_low, 80, "1 买止损=pivot_low（C 段破中枢端点，reference:46）");
}

/// 第一类反退化（Lean IsType1 需 IsDivergence）：破中枢但后段面积 >= 前段（¬IsDivergence）⟹ ¬IsType1。
/// 对照 Lean：IsType1 = brokeCenter ∧ IsDivergence；IsDivergence 假 ⟹ 整体假（合取）。
#[test]
fn type1_rejected_without_divergence_matches_lean() {
    let c = Center { zd: 100, zg: 200, dd: 95, gg: 205, start_index: 0, end_index: 2 };
    let segs = vec![
        seg(Direction::Down, 3, 5, 150, 90),
        seg(Direction::Down, 6, 8, 120, 80),
    ];
    // 前段小幅、后段大幅 ⟹ 后段面积 > 前段 ⟹ ¬IsDivergence（力度延续）。
    let (closes, src) = closes_seq(&[100, 100, 100, 100, 98, 102, 100, 50, 150]);
    let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
    assert!(points.iter().all(|p| !p.bits.buy1), "Lean ¬IsDivergence ⟹ ¬IsType1 ⟺ rust ¬buy1");
}

// ════════════════════════════════════════════════════════════════════════════
//  覆盖确认（extract_signals L0 层产 B1/S1 + B3/S3；B2/S2 由 extract_second_signals 递归组装层产）
// ════════════════════════════════════════════════════════════════════════════

/// ★L0 签名层边界（编码为可执行断言）：extract_signals 产**第一类 + 第三类**，但对任意 L0 segment
/// 输入**永不产第二类**（buy2/sell2 恒假）——第二类=次级别第一类构成（买卖点定律一），需 RMove
/// 递归结构（descend.rs），L0 segment 是递归底（descend 得空）⟹ 结构上不可在本签名层产出。
/// B2/S2 由平行的递归组装层入口 `extract_second_signals` 产（消费 RMove 塔，见 §层 D）——按输入对象
/// 分工（L0 segment vs RMove 递归塔），非 extract_signals 缺口。本测试锁定 L0 层覆盖边界（B1/B3）。
///
/// ★fixture 对齐 tau 框架权威（divergence.rs trend_class，b8667b968f 收紧 B1=趋势背驰 A/B/C 框架）：
/// 旧 fixture 用单中枢（Consolidation τ）产 B1 是退化语义——tau 门控下盘整背驰**不产第一类**
/// （beichi #4 + maimai.md:56 已结算）。现对齐 `signal.rs:687-705` 的 `[c0, c1]` 两全链同向向下
/// 中枢模式（c1.gg < c0.dd ⟹ 下跌趋势 τ=Trend(Down)），B1=趋势背驰（C 段破最后中枢 ∧ C<A 面积）。
#[test]
fn signal_extraction_emits_no_second_class_only() {
    // 两依次向下中枢（下跌趋势 τ=Trend(Down)），对齐 signal.rs:687-705 `first_buy_extracted_with_trend_divergence`。
    // c1.gg=210 < c0.dd=290 ⟹ 下跌延续 ⟹ Trend(Down) ⟹ B1 可产（趋势背驰，非盘整背驰）。
    let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
    let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
    let segs = vec![
        seg(Direction::Down, 3, 5, 350, 250),   // A 段：C0 离开段（破 C0 下沿），MACD 面积大（强势）
        seg(Direction::Up, 5, 7, 250, 280),     // B 段：中间反向连接（构成 C1）
        seg(Direction::Down, 9, 11, 150, 80),   // C 段：破 C1 下沿（< 100）∧ C<A 趋势背驰 ⟹ B1
        seg(Direction::Up, 11, 13, 80, 260),    // 离开 C1 上方（端点 > c1.zg=200）
        seg(Direction::Down, 13, 15, 260, 210), // 回试低点 210 > c1.zg=200（不破 ZG）⟹ B3
    ];
    // closes（merged_bars 序列，与 segment 抽象端点价解耦——结构判定在 tick 域，MACD 在浮点域，两者
    // 不必逐 bar 一致）：A 段 bar[3,5] 急跌 hist 大=强力度，B 段 bar[5,7] 盘整让 EMA 收敛，C 段 bar[9,11]
    // 缓动 hist 小=力度衰减=趋势背驰（手算 A area[3,5]=23.26 > C area[9,11]=9.09 ⟹ C<A 成立）。
    let (closes, src) = closes_seq(&[
        300, 300, 300,        // 0..2 C0 区（预热）
        300, 100, 250,        // 3..5 A 段：急跌（hist 绝对值大=强力度）
        250, 250, 250,        // 6..8 B 段：盘整让 EMA 收敛（hist 回拉 0 轴）
        248, 246, 244,        // 9..11 C 段：缓动（hist 小=力度衰减=趋势背驰）
        244, 250, 256,        // 11..13 离开上段：价格回升过 c1.zg=200（端点 256 > 200）
        256, 230, 210,        // 13..15 回试：低点 210 > c1.zg=200（不破 ZG ⟹ B3）
    ]);
    let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
    for p in &points {
        assert!(!p.bits.buy2, "extract_signals（L0 层）不产第二类买点——B2 由 extract_second_signals 递归组装层产");
        assert!(!p.bits.sell2, "extract_signals（L0 层）不产第二类卖点——S2 由 extract_second_signals 递归组装层产");
    }
    // 多声部确认：B1（趋势背驰 A/B/C 框架）+ B3（离开后回试不破）均产出（消解「单声部 L0 第三类」根因）。
    assert!(points.iter().any(|p| p.bits.buy1), "产第一类（趋势背驰：C 段破最后中枢 ∧ C<A 面积）");
    assert!(points.iter().any(|p| p.bits.buy3), "产第三类（离开后回试不破 ZG）");
}

// ════════════════════════════════════════════════════════════════════════════
//  层 D — 第二类（B2/S2，#52 递归组装层）：extract_second_signals 消费 RMove 递归塔的
//          SecondTypeStructure ↔ Lean Origin.RMoveCompose.{SecondTypeStructure,secondPointPrice}
//
//  Lean RMoveCompose.lean 已 machine-checked 见证（直接对照值，非 fixture，同 type3 内部点尺度精神）：
//  - m1Wit（:301-302）：向下破中枢，区间 [-10,-2]，lo=-10 < c1Wit.zd=0 ⟹ SubBrokeBelow（:330）。
//  - m2Wit（:305-306）：回拉不创新低，区间 [-8,3]，lo=-8 ≥ m1.lo=-10 ⟹ NoNewLow（:336）。
//  - c1Wit（:313-315）：次级别中枢核心 [zd,zg]=[0,4]（B 口径核心区间）。
//  - witness_secondTypeStructure（:352）：i1=0 < i2=1，第一类离开破中枢∧背驰 + 回拉不创新低 ⟹
//    SecondTypeStructure Side.long parentWit2 成立（已证）。
//  - witness_secondPoint_price（:374）：secondPointPrice Side.long m2Wit = -8（回拉低点 = m2.lo，已证）。
//  - witness_newLow_breaks_type2（:343）：回拉 lo=-12 < m1.lo=-10 ⟹ ¬NoNewLow（不创新低是真约束，已证）。
//
//  rust extract_second_signals 用 Lean 见证原始数值（c1Wit/m1Wit/m2Wit）构造 RMove 塔，断言产出
//  与 Lean 见证 bit-exact 一致（second_point = -8 = Lean secondPointPrice）。
//  ★坐标：Lean Move μF 无 source_index（同 rust RMove）——本层对照结构判定 + second_point 价位，
//  source_index 由 rust 测试侧 index_of 提供（坐标 still-MISSING，对照不依赖坐标，见 signal.rs 模块头）。
// ════════════════════════════════════════════════════════════════════════════

/// 第二类见证中枢（Lean c1Wit，RMoveCompose.lean:313-315，B 口径核心区间 [zd,zg]=[0,4]）。
fn second_center_wit() -> Center {
    Center { zd: 0, zg: 4, dd: -2, gg: 6, start_index: 0, end_index: 0 }
}

/// Lean m1Wit（:301-302）：第一类离开走势（向下破中枢，区间 [-10,-2]）。
fn m1_wit_rust() -> RMove {
    RMove::Segment { direction: Direction::Down, lo: -10, hi: -2 }
}

/// Lean m2Wit（:305-306）：回拉走势（不创新低，区间 [-8,3]）。
fn m2_wit_rust() -> RMove {
    RMove::Segment { direction: Direction::Up, lo: -8, hi: 3 }
}

/// Lean m3Wit（:309-310）：收尾走势（区间 [1,5]）。
fn m3_wit_rust() -> RMove {
    RMove::Segment { direction: Direction::Up, lo: 1, hi: 5 }
}

#[test]
fn type2_buy_matches_lean_witness_second_point() {
    // ★对照 Lean witness_secondTypeStructure（:352）+ witness_secondPoint_price（:374）：
    // RMove 塔（m1 破中枢∧背驰 + m2 回拉不创新低 + i1<i2）⟹ rust 产 B2，second_point = m2.lo = -8
    // （= Lean secondPointPrice Side.long m2Wit）。
    let parent = compose_move(vec![m1_wit_rust(), m2_wit_rust(), m3_wit_rust()], vec![second_center_wit()], 1);
    let points = extract_second_signals(
        &parent,
        Side::Long,
        &second_center_wit(),
        |m| m.lo() == -10,        // Lean divWit2：仅第一类离开 m1（含 lo=-10）背驰
        |m| if m.lo() == -8 { 99 } else { 0 }, // 回拉走势 m2 坐标（still-MISSING，测试侧提供）
    );
    assert_eq!(points.len(), 1, "Lean witness_secondTypeStructure 成立 ⟹ rust 产一个 B2");
    assert!(points[0].bits.buy2, "Lean SecondTypeStructure Side.long ⟺ rust buy2 置位");
    assert!(!points[0].bits.buy1 && !points[0].bits.buy3, "第二类不置 1/3 类 bit");
    // ★second_point bit-exact：= Lean secondPointPrice Side.long m2Wit = -8（回拉低点 = m2.lo）。
    assert_eq!(points[0].pivot_low, -8, "rust B2 止损源 pivot_low == Lean secondPointPrice m2Wit（-8）");
}

#[test]
fn type2_buy_rejected_new_low_matches_lean() {
    // ★对照 Lean witness_newLow_breaks_type2（:343）：回拉创新低（lo=-12 < m1.lo=-10）⟹ ¬NoNewLow
    // ⟹ ¬SecondTypeStructure ⟹ rust 无 B2（不创新低是真约束，rust 与 Lean 同判）。
    let m2_break = RMove::Segment { direction: Direction::Up, lo: -12, hi: 3 };
    let parent = compose_move(vec![m1_wit_rust(), m2_break], vec![second_center_wit()], 1);
    let points = extract_second_signals(&parent, Side::Long, &second_center_wit(), |m| m.lo() == -10, |_m| 0);
    assert!(points.is_empty(), "Lean ¬NoNewLow（回拉创新低）⟹ rust 无 B2（与 Lean 同判非第二类）");
}

#[test]
fn type2_sell_mirror_matches_lean() {
    // ★S2 镜像对照 Lean NoNewHigh（RMoveCompose.lean:152）：第一类向上探顶 [2,10]，回抽 [1,8]，
    // hi=8 ≤ m1.hi=10 ⟹ NoNewHigh ⟹ SecondTypeStructure Side.short ⟹ rust 产 S2，
    // second_point = secondPointPrice Side.short m2 = m2.hi = 8。
    let m1_sell = RMove::Segment { direction: Direction::Up, lo: 2, hi: 10 };
    let m2_sell = RMove::Segment { direction: Direction::Down, lo: 1, hi: 8 };
    let m3_sell = RMove::Segment { direction: Direction::Down, lo: 0, hi: 6 };
    let parent = compose_move(vec![m1_sell, m2_sell, m3_sell], vec![second_center_wit()], 1);
    let points = extract_second_signals(
        &parent,
        Side::Short,
        &second_center_wit(),
        |m| m.hi() == 10,
        |m| if m.hi() == 8 { 7 } else { 0 },
    );
    assert_eq!(points.len(), 1, "Lean NoNewHigh ⟹ rust 产一个 S2");
    assert!(points[0].bits.sell2 && !points[0].bits.buy2, "Lean SecondTypeStructure Side.short ⟺ rust sell2 置位");
    assert_eq!(points[0].pivot_high, 8, "rust S2 止损源 pivot_high == Lean secondPointPrice Side.short m2（m2.hi=8）");
}

/// ★诚实边界（编码为可执行断言，formalization-validity-domain）：B2/S2 输入是 RMove 递归塔，
/// 不是 L0 segment。L0 线段（RMove::Segment，递归底）descend 得空 ⟹ extract_second_signals 诚实空
/// （对照 Lean segment_no_secondType，RMoveCompose.lean:285）——按输入对象分工，B2/S2 不在 L0 segment 层产。
#[test]
fn type2_empty_for_l0_segment_matches_lean() {
    let seg = RMove::Segment { direction: Direction::Down, lo: -10, hi: -2 };
    let points = extract_second_signals(&seg, Side::Long, &second_center_wit(), |_m| true, |_m| 0);
    assert!(points.is_empty(), "Lean segment_no_secondType ⟺ rust L0 线段无 B2（递归底诚实空）");
}
