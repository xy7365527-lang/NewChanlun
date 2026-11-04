//! 中枢 reference v1 语义 parity（Lean #eval 机器导出 fixture ↔ rust 实装 bit-exact）。
//!
//! ## 工位定位（第Ⅱ类中枢 reference 语义，cov-rust-impl「中枢层无 parity=漂移风险」消除）
//!
//! `formal/Origin/CenterConstruct.lean` § 5.5 的 `refZhongshusFromComponents` 用 `#eval` **机器导出**
//! reference v1 中枢全字段（zd/zg/gg/dd/count/settled/break_index/break_up）为 JSON，落盘
//! `fixtures/theta_v0_center_parity.json`。本文件 `include_str!` 读同一 fixture + serde 反序列化，
//! 断言 rust `ref_zhongshus_from_components` 计算 == fixture 中 Lean 导出值。
//!
//! 机器耦合（非手工转录，631 铁律）：Lean 改 refZhongshusFromComponents ⟹ #eval 输出变 ⟹
//! fixture 变 ⟹ 本测试随之变（漂移被消除）。fixture 三 case 覆盖：核心计算（全三段 max3/min3）+
//! 弱接触延伸（count=4）+ 结算回退（settled+break）。★#1419 S3 另加四个判别例（Lean § 5.5.3b，
//! 期待值与 S0 oracle 冻结稿 `final_scan_lean_ref` 一致）：Zn 外缘 + last+2 续扫（UP/DOWN）、
//! 续扫锚 seed 相切 +1 与两单元连接段（E4）、D-A 方向判据（不交替输入）。
//!
//! `break_up` 只对拍已结算中枢（#1419 根裁定 r2 R4）：未结算中枢的离开方向未定义，reference 两侧都写
//! 占位值 `true`，S 事实层另行发布 `breakUp=false` 且 `breakUp_defined=false`。fixture 的
//! `meta.break_up_scope = "settled_only"` 声明这条范围，本文件读它并据此跳过，Lean 侧改范围时这里会红。
//!
//! ## 认识论（formalization-validity-domain 231号）
//!
//! L0：bit-exact 确认「rust 实装忠实于 Lean 形式化 / reference Python」。非 L2 行情有效断言。

use newchan_rust::theta_v0::classifier::ref_v1::{
    legacy_v0_interval, ref_v1_interval, ref_zhongshus_from_components, RefZhongshu,
};
use newchan_rust::theta_v0::types::{Direction, Segment, Tick};

use serde::Deserialize;

// ── fixture serde 镜像（CenterConstruct.lean refZhongshuJson / refV1FixtureJson）─────

/// fixture 中单个 RefZhongshu JSON（字段名严格对齐 Lean refZhongshuJson）。
#[derive(Debug, Deserialize)]
struct FixtureZs {
    zd: Tick,
    zg: Tick,
    gg: Tick,
    dd: Tick,
    start: usize,
    end: usize,
    count: usize,
    settled: bool,
    break_index: i64,
    break_up: bool,
}

/// fixture 中 v0/v1 区间对照段。
#[derive(Debug, Deserialize)]
struct FixtureV0V1 {
    v1_zd: Tick,
    v1_zg: Tick,
    v0_zd: Tick,
    v0_zg: Tick,
}

/// fixture `meta`（只取对拍范围声明）。
#[derive(Debug, Deserialize)]
struct FixtureMeta {
    break_up_scope: String,
}

/// 完整 fixture（CenterConstruct.lean refV1FixtureJson 镜像）。
#[derive(Debug, Deserialize)]
struct CenterFixture {
    meta: FixtureMeta,
    three_unsettled: Vec<FixtureZs>,
    four_extend: Vec<FixtureZs>,
    five_settled_break: Vec<FixtureZs>,
    e2_up_zn_c_resume: Vec<FixtureZs>,
    e2_down_zn_c_resume: Vec<FixtureZs>,
    e4_tangent_seed_connector: Vec<FixtureZs>,
    zn_direction_not_parity: Vec<FixtureZs>,
    v0_v1_interval: FixtureV0V1,
}

fn load_fixture() -> CenterFixture {
    let raw = include_str!("fixtures/theta_v0_center_parity.json");
    let fx: CenterFixture =
        serde_json::from_str(raw).expect("fixture 必须是 Lean #eval 导出的合法 JSON");
    assert_eq!(
        fx.meta.break_up_scope, "settled_only",
        "Lean 导出声明的 break_up 对拍范围变了，本文件的跳过规则须同步（#1419 R4）"
    );
    fx
}

/// rust RefZhongshu == fixture 中 Lean 导出 RefZhongshu（逐字段 bit-exact；`break_up` 只比已结算中枢）。
fn assert_zs_eq(rust: &RefZhongshu, lean: &FixtureZs, ctx: &str) {
    assert_eq!(rust.zd, lean.zd, "{ctx}: zd 漂移");
    assert_eq!(rust.zg, lean.zg, "{ctx}: zg 漂移");
    assert_eq!(rust.gg, lean.gg, "{ctx}: gg 漂移");
    assert_eq!(rust.dd, lean.dd, "{ctx}: dd 漂移");
    assert_eq!(rust.start, lean.start, "{ctx}: start 漂移");
    assert_eq!(rust.end, lean.end, "{ctx}: end 漂移");
    assert_eq!(rust.count, lean.count, "{ctx}: count 漂移");
    assert_eq!(rust.settled, lean.settled, "{ctx}: settled 漂移");
    assert_eq!(
        rust.break_index, lean.break_index,
        "{ctx}: break_index 漂移"
    );
    if lean.settled {
        assert_eq!(rust.break_up, lean.break_up, "{ctx}: break_up 漂移");
    }
}

// ── fixture 三 case 的 rust 输入（必须与 Lean refSeg0..refSeg4Break 同价位编码）─────

/// 段构造（high/low 由 start/end price 编码：up 段 end>start，down 段 start>end）。
fn seg(si: usize, ei: usize, dir: Direction, sp: Tick, ep: Tick) -> Segment {
    Segment {
        direction: dir,
        start_index: si,
        end_index: ei,
        start_price: sp,
        end_price: ep,
    }
}

// Lean refSeg0/refSeg1/refSeg2/refSeg3Ext/refSeg4Break 对应（CenterConstruct.lean § 5.5.2/5.5.3）。
fn ref_seg0() -> Segment {
    seg(0, 1, Direction::Up, 10, 20)
} // [low=10, high=20]
fn ref_seg1() -> Segment {
    seg(1, 2, Direction::Down, 15, 12)
} // [low=12, high=15]
fn ref_seg2() -> Segment {
    seg(2, 3, Direction::Up, 11, 18)
} // [low=11, high=18]
fn ref_seg3_ext() -> Segment {
    seg(3, 4, Direction::Down, 16, 13)
} // [low=13, high=16] 弱接触延伸
fn ref_seg4_break() -> Segment {
    seg(4, 5, Direction::Up, 22, 25)
} // [low=22, high=25] 结算段

// ──────────────────────────────────────────────────────────────────────────
//  Case 1: 三段 → 恰一中枢，末 unsettled（全三段 max3/min3 核心计算 bit-exact）
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn parity_three_unsettled() {
    let fx = load_fixture();
    let rust = ref_zhongshus_from_components(&[ref_seg0(), ref_seg1(), ref_seg2()]);
    assert_eq!(
        rust.len(),
        fx.three_unsettled.len(),
        "中枢数漂移（rust {} vs Lean {}）",
        rust.len(),
        fx.three_unsettled.len()
    );
    for (r, l) in rust.iter().zip(fx.three_unsettled.iter()) {
        assert_zs_eq(r, l, "three_unsettled");
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Case 2: 四段 → 弱接触延伸 count=4（reference:130-134）
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn parity_four_extend() {
    let fx = load_fixture();
    let rust = ref_zhongshus_from_components(&[ref_seg0(), ref_seg1(), ref_seg2(), ref_seg3_ext()]);
    assert_eq!(rust.len(), fx.four_extend.len(), "中枢数漂移");
    for (r, l) in rust.iter().zip(fx.four_extend.iter()) {
        assert_zs_eq(r, l, "four_extend");
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Case 3: 五段 → 结算 + break up（reference:136-145）
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn parity_five_settled_break() {
    let fx = load_fixture();
    let rust = ref_zhongshus_from_components(&[
        ref_seg0(),
        ref_seg1(),
        ref_seg2(),
        ref_seg3_ext(),
        ref_seg4_break(),
    ]);
    assert_eq!(rust.len(), fx.five_settled_break.len(), "中枢数漂移");
    for (r, l) in rust.iter().zip(fx.five_settled_break.iter()) {
        assert_zs_eq(r, l, "five_settled_break");
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Case 4: v0/v1 核心区间对照（裁决 v0 错，对齐 Origin.CenterConstruct + NewChanlunEngineAudit）
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn parity_v0_v1_interval() {
    let fx = load_fixture();
    // rust v1 全三段 == Lean 导出 v1。
    let (v1_zd, v1_zg) = ref_v1_interval(10, 20, 12, 15, 11, 18);
    assert_eq!(v1_zd, fx.v0_v1_interval.v1_zd, "v1_zd 漂移");
    assert_eq!(v1_zg, fx.v0_v1_interval.v1_zg, "v1_zg 漂移");
    // rust v0 首尾段 == Lean 导出 v0。
    let (v0_zd, v0_zg) = legacy_v0_interval(10, 20, 12, 15, 11, 18);
    assert_eq!(v0_zd, fx.v0_v1_interval.v0_zd, "v0_zd 漂移");
    assert_eq!(v0_zg, fx.v0_v1_interval.v0_zg, "v0_zg 漂移");
    // 裁决：v0 ≠ v1（同 fixture），故 v0 不是 v1 的正确实现。
    assert_ne!((v0_zd, v0_zg), (v1_zd, v1_zg), "v0 必须 ≠ v1（反例裁决）");
}

// ──────────────────────────────────────────────────────────────────────────
//  ★#1419 S3 判别例（Lean § 5.5.3b `segsOfEndpoints` / `dirExtSegs` 同价位编码）
// ──────────────────────────────────────────────────────────────────────────

/// 端点序列 → 相连段序列（`u_k = P_k → P_{k+1}`，方向由端点升降定；Lean `segsOfEndpoints`）。
fn segs_from_endpoints(p: &[Tick]) -> Vec<Segment> {
    p.windows(2)
        .enumerate()
        .map(|(k, w)| {
            let dir = if w[1] > w[0] {
                Direction::Up
            } else {
                Direction::Down
            };
            seg(k, k + 1, dir, w[0], w[1])
        })
        .collect()
}

/// S0 oracle UP 流 K12（Lean `e2UpEndpoints`）。
const E2_UP: [Tick; 14] = [10, 40, 20, 35, 30, 38, 21, 25, 5, 15, 2, 12, 4, 9];
/// S0 oracle E4 流 K10（Lean `e4Endpoints`）。
const E4: [Tick; 12] = [10, 40, 20, 35, 5, 15, 2, 18, 15, 17, 1, 8];

fn assert_case(rust: &[RefZhongshu], lean: &[FixtureZs], ctx: &str) {
    assert_eq!(rust.len(), lean.len(), "{ctx}: 中枢数漂移（rust {rust:?}）");
    for (r, l) in rust.iter().zip(lean.iter()) {
        assert_zs_eq(r, l, ctx);
    }
}

/// UP：A = u0..u7（u7 ↓ 非 Zn ⟹ DD=10），突破段 u8 后自 last+2 = u9 起 B = u9..u12。
#[test]
fn parity_e2_up_zn_c_resume() {
    let fx = load_fixture();
    let rust = ref_zhongshus_from_components(&segs_from_endpoints(&E2_UP));
    assert_case(&rust, &fx.e2_up_zn_c_resume, "e2_up_zn_c_resume");
}

/// DOWN 镜像（p' = 45 − p）：A 的 GG = 35。
#[test]
fn parity_e2_down_zn_c_resume() {
    let fx = load_fixture();
    let mirrored: Vec<Tick> = E2_UP.iter().map(|p| 45 - p).collect();
    let rust = ref_zhongshus_from_components(&segs_from_endpoints(&mirrored));
    assert_case(&rust, &fx.e2_down_zn_c_resume, "e2_down_zn_c_resume");
}

/// E4：续扫锚 u5 的 seed ZD = ZG 相切不成立 ⟹ +1，B = u6..u9，连接段 u4、u5。
#[test]
fn parity_e4_tangent_seed_connector() {
    let fx = load_fixture();
    let rust = ref_zhongshus_from_components(&segs_from_endpoints(&E4));
    assert_case(
        &rust,
        &fx.e4_tangent_seed_connector,
        "e4_tangent_seed_connector",
    );
}

/// D-A：不交替输入上 Zn = 与首段同向的段（Lean `dirExtSegs`）。
#[test]
fn parity_zn_direction_not_parity() {
    let fx = load_fixture();
    let segs = [
        seg(0, 1, Direction::Up, 10, 40),
        seg(1, 2, Direction::Up, 20, 50),
        seg(2, 3, Direction::Down, 30, 0),
        seg(3, 4, Direction::Down, 25, 5),
        seg(4, 5, Direction::Up, 15, 60),
    ];
    let rust = ref_zhongshus_from_components(&segs);
    assert_case(
        &rust,
        &fx.zn_direction_not_parity,
        "zn_direction_not_parity",
    );
}
