//! #668（N4）桥接对象语义锁。
//!
//! N1 事件侧夹具经 `CandidateEventBook::advance` 产出真实事件（同 `chain_cert::tests` 先例，
//! 不手搓 `CandidateEvent` 字面量——终态/生命史钟必须走簿的修订协议产生）。BSP 侧
//! （`BspPoint`/`Classification`）是纯数据记录（无判定逻辑、无生命史协议），手搓字面量夹具
//! 直接测试本模块**自己的**相关/折叠逻辑，不冒充判据测试。

use std::rc::Rc;

use super::super::super::types::{BspBits, Center, Side, ThirdClassEntryIdentity};
use super::super::bsp::{BspPoint, OwnerRef};
use super::super::cand_event::{
    CandidateEventBook, CandidateKey, CandidateKind, CandidateObservation, CandidateState,
    ParentFingerprint, StructuralPredicates, CANDIDATE_RULE_VERSION,
};
use super::super::{Classification, LevelState};
use super::*;

fn parent_fp() -> ParentFingerprint {
    ParentFingerprint { center_start: 20, zd: 100, zg: 110 }
}

fn trend_key(level: u32, seg_a: (usize, usize), c_start: usize) -> CandidateKey {
    CandidateKey {
        rule_version: CANDIDATE_RULE_VERSION,
        level,
        kind: CandidateKind::Trend,
        side: Side::Long,
        previous_center_start: Some(5),
        parent: parent_fp(),
        seg_a,
        c_start,
    }
}

fn trend_observation(key: CandidateKey, interval: (usize, usize)) -> CandidateObservation {
    CandidateObservation {
        key,
        kind: CandidateKind::Trend,
        center_ids: Some((5, 20)),
        candidate_group_id: 1,
        pair_id: 2,
        structural_predicates: StructuralPredicates { direction: true, comparable: true, extreme: true },
        extreme_proof: key.seg_a,
        third_class_proof: None,
        interval,
        state: CandidateState::Provisional,
        first_provable_at: Some(interval.1),
        confirmed_at: None,
    }
}

fn center_at(start_index: usize) -> Center {
    Center { zd: 100, zg: 110, dd: 100, gg: 110, start_index, end_index: start_index + 3 }
}

fn level_with(points: Vec<BspPoint>) -> LevelState {
    LevelState {
        moves: Vec::new(),
        centers: Rc::new(Vec::new()),
        cp_ownership: Rc::new(Vec::new()),
        bsp: Rc::new(points),
        pan_div: Rc::new(Vec::new()),
        level_projection: None,
    }
}

fn buy1_point(source_index: usize, center_start: usize) -> BspPoint {
    BspPoint {
        source_index,
        bits: BspBits { buy1: true, ..Default::default() },
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center_at(center_start))),
        struct_break_dir: Some(Side::Long),
        force: None,
    }
}

// ── 一类：命中/查无 ──────────────────────────────────────────────────────────────────────

#[test]
fn first_class_point_pairs_with_matching_trend_event() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification { levels: vec![level_with(vec![buy1_point(40, 20)])] };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 40);

    assert_eq!(delta.len(), 1, "唯一一类点应恰好产 1 条边");
    let edge = &delta[0];
    assert_eq!(edge.key.event, key);
    assert_eq!(edge.key.bsp.class, BspPointClass::Buy1);
    assert_eq!(edge.key.bsp.anchor, vec![(6, 19), (25, 25)], "锚不含本点自身 source_index=40");
    assert_eq!(edge.bsp_source_index, 40);
    assert_eq!(edge.status, BridgeStatus::Open);
    assert_eq!(edge.revision, 0);
}

#[test]
fn no_matching_event_is_absent_not_an_error() {
    // 中枢指纹与任何候选事件都不同 ⟹ 查无——advance 静默跳过，不产边、不 panic。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification { levels: vec![level_with(vec![buy1_point(40, 999)])] };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 40);

    assert!(delta.is_empty(), "Absent 非证伪：查无 N1 事件不产边");
    assert!(bridge.heads().is_empty());
}

// ── 端死边死 + append-only ───────────────────────────────────────────────────────────────

#[test]
fn invalidated_event_freezes_edge_terminal_with_history_preserved() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);

    let classification = Classification { levels: vec![level_with(vec![buy1_point(40, 20)])] };
    let mut bridge = BspBridgeBook::default();
    let opened = bridge.advance(&classification, &book.streams(), 40);
    assert_eq!(opened[0].status, BridgeStatus::Open);

    // 缺席即失效（N1 CandidateEventBook 自有纪律）：下一次 advance 不再观察该 key。
    book.advance(&[], 50);
    let invalidated = bridge.advance(&classification, &book.streams(), 50);
    assert_eq!(invalidated.len(), 1, "事件转 Invalidated 应产生新 revision");
    let edge = &invalidated[0];
    assert_eq!(edge.status, BridgeStatus::Invalidated);
    assert_eq!(edge.invalidated_at, Some(50));
    assert_eq!(edge.revision, 1);
    assert_eq!(edge.supersedes_revision, Some(0));

    // 终态不复活：再 advance 一次（同一份已终态的流），不应再追加 revision。
    let rerun = bridge.advance(&classification, &book.streams(), 60);
    assert!(rerun.is_empty(), "终态边不复活，advance 静默跳过");
    assert_eq!(bridge.edges().len(), 2, "旧 revision 留痕，append-only 不删除");
}

#[test]
fn same_as_of_rerun_is_zero_delta() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification { levels: vec![level_with(vec![buy1_point(40, 20)])] };
    let mut bridge = BspBridgeBook::default();
    let first = bridge.advance(&classification, &streams, 40);
    assert_eq!(first.len(), 1);
    let second = bridge.advance(&classification, &streams, 40);
    assert!(second.is_empty(), "投影未变 ⟹ 幂等跳过，同 as_of 重跑零 Delta");
}

// ── 二类：继承一类锚的事件键 ──────────────────────────────────────────────────────────────

#[test]
fn second_class_point_inherits_anchor_first_class_event_key() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let anchor = buy1_point(40, 20);
    let mut second = buy1_point(70, 20);
    second.bits = BspBits { buy2: true, ..Default::default() };
    second.center = Some(OwnerRef::Type1Anchor(40));

    let classification = Classification { levels: vec![level_with(vec![anchor, second])] };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 70);

    assert_eq!(delta.len(), 2, "锚点自身一条一类边 + 二类点一条边");
    let second_edge = delta
        .iter()
        .find(|edge| edge.key.bsp.class == BspPointClass::Buy2)
        .expect("二类边必须存在");
    assert_eq!(second_edge.key.event, key, "二类边继承其一类锚的 N1 事件键");
    assert_eq!(second_edge.key.bsp.anchor, vec![(40, 40)]);
    assert_eq!(second_edge.bsp_source_index, 70);
}

// ── 三类：离开段命中 Trend 候选（不要求背驰确认） ─────────────────────────────────────────

#[test]
fn third_class_point_pairs_with_leave_segment_trend_event() {
    let key = trend_key(0, (1, 5), 6);
    let mut book = CandidateEventBook::default();
    // 离开段候选：仅几何破中枢即可入簿（结构宽候选，不要求背驰——Provisional 已足够）。
    book.advance(&[trend_observation(key, (6, 15))], 15);
    let streams = book.streams();

    let third_entry = ThirdClassEntryIdentity {
        center_si: 20,
        center_zd: 100,
        center_zg: 110,
        leave_interval: (6, 15),
        retest_interval: (16, 22),
    };
    let point = BspPoint {
        source_index: 22,
        bits: BspBits { buy3: true, third_class_entry: Some(third_entry), ..Default::default() },
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center_at(20))),
        struct_break_dir: None,
        force: None,
    };
    let classification = Classification { levels: vec![level_with(vec![point])] };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 22);

    assert_eq!(delta.len(), 1);
    let edge = &delta[0];
    assert_eq!(edge.key.event, key, "三类边取离开段所在的 Trend 候选（不要求背驰确认）");
    assert_eq!(edge.key.bsp.class, BspPointClass::Buy3);
    assert_eq!(edge.key.bsp.anchor, vec![(6, 15), (16, 16)], "回试段仅左端入锚，右端=自身 source_index 排除");
    assert_eq!(edge.bsp_source_index, 22);
}

// ── 查询入口（#666 裁定⑦ 对拍用） ────────────────────────────────────────────────────────

#[test]
fn edges_for_bsp_point_finds_by_level_and_source_index() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification { levels: vec![level_with(vec![buy1_point(40, 20)])] };
    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &streams, 40);

    assert_eq!(bridge.edges_for_bsp_point(0, 40).len(), 1);
    assert!(bridge.edges_for_bsp_point(0, 999).is_empty());
    assert!(bridge.edges_for_bsp_point(1, 40).is_empty());
}
