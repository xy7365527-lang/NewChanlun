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

// ── HIGH-1/HIGH-2 修复：episode 覆盖判据（评审 #670 回炉，第三轮 supersede 裁定①⑤） ────────

#[test]
fn first_class_multiple_points_in_same_episode_collapse_to_revision_history() {
    // 同一 episode（同 seg_a/c_start）内两个物理一类点（多段递进背驰，教义必然）——
    // 撞键自动消解为修订链，不需要任何区分量（第三轮 supersede 裁定①）。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 55))], 55);
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 55);

    assert_eq!(delta.len(), 2, "同 episode 两个物理一类点应产 2 条 revision（修订史，非撞键丢弃）");
    assert_eq!(delta[0].bsp_source_index, 40, "revision 0 = 较早物理点（source_index 升序处理）");
    assert_eq!(delta[1].bsp_source_index, 55, "revision 1 = 较晚物理点，成为链头");
    assert_eq!(delta[0].key, delta[1].key, "两条 revision 共享同一 BridgeKey（episode 身份）");
    assert_eq!(delta[1].revision, 1);
    assert_eq!(delta[1].supersedes_revision, Some(0));

    assert_eq!(bridge.edges().len(), 2, "append-only：两条物理点的历史都留痕");
    let heads = bridge.heads();
    assert_eq!(heads.len(), 1, "簿内该 episode 只有一条链头（身份唯一，不是两个身份）");
    assert_eq!(heads[0].bsp_source_index, 55, "链头 = 最新物理点（pivot 是修订载荷，不是身份分量）");
}

#[test]
fn trend_event_growth_does_not_orphan_earlier_first_class_point() {
    // MED-1 + HIGH-3 负控：C 段生长（growth_revision）不产生新的、恰好落在新右端上的物理点时，
    // 旧判据（右端等值）会从这一步起再也观察不到该点——边永远停在生长前的旧状态，此后事件转
    // Invalidated 也不会同步（观察缺失 ⟹ `apply` 从未被调用）。修复后的判据（episode 区间覆盖）
    // 会持续观察到该点：生长这一步本身载荷未变（同 `source_index`/同 `status`）⟹ 按幂等法则
    // 正确地零 Delta（不是漏观察）；真正的分辨力在下一步——事件失效必须能同步传导。旧判据在此
    // 分辨点上必然失败（观察从生长步起彻底消失，failure 会在最终 Invalidated 断言处炸出）。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let classification = Classification { levels: vec![level_with(vec![buy1_point(40, 20)])] };
    let mut bridge = BspBridgeBook::default();
    let first = bridge.advance(&classification, &book.streams(), 40);
    assert_eq!(first.len(), 1);
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Open);

    // C 段右端从 40 生长到 55，没有新的物理一类点落在新右端。载荷未变 ⟹ 幂等零 Delta
    // （不是漏观察——`apply` 仍被调用，只是投影相等而跳过 append）。
    book.advance(&[trend_observation(key, (25, 55))], 55);
    let grown = bridge.advance(&classification, &book.streams(), 55);
    assert!(grown.is_empty(), "载荷未变，幂等零 Delta（同 same_as_of_rerun_is_zero_delta 精神，跨 as_of 亦然）");
    assert_eq!(bridge.heads()[0].bsp_source_index, 40);
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Open);

    // 事件后续失效：边必须同步转终态，不能卡在生长前的旧状态（MED-1 核心断言）。
    book.advance(&[], 60);
    let invalidated = bridge.advance(&classification, &book.streams(), 60);
    assert_eq!(invalidated.len(), 1, "事件转 Invalidated 应追加一条新 revision");
    assert_eq!(
        bridge.heads()[0].status,
        BridgeStatus::Invalidated,
        "边必须跟随事件同步转终态（MED-1 修复验证）"
    );
}

#[test]
fn distinct_episodes_never_share_a_bridge_key() {
    // 同 parent 指纹、不同 seg_a/c_start 的两个 episode，各自的一类点不得被误合并到同一
    // BridgeKey（MED-2「同 BridgeKey 改指他点」缺口的正面对照：应始终各归各)。
    let key_a = trend_key(0, (6, 19), 25);
    let key_b = trend_key(0, (60, 79), 90);
    let mut book = CandidateEventBook::default();
    book.advance(
        &[trend_observation(key_a, (25, 40)), trend_observation(key_b, (90, 110))],
        110,
    );
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(110, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 110);

    assert_eq!(delta.len(), 2);
    let key_for_40 = &delta.iter().find(|e| e.bsp_source_index == 40).expect("point 40 应产边").key;
    let key_for_110 = &delta.iter().find(|e| e.bsp_source_index == 110).expect("point 110 应产边").key;
    assert_ne!(key_for_40, key_for_110, "不同 episode 必须产生不同 BridgeKey，不得因同 parent 而误合并");
    assert_eq!(key_for_40.event, key_a);
    assert_eq!(key_for_110.event, key_b);
}

#[test]
fn first_class_point_pairs_at_non_zero_level() {
    // MED-2 缺口：既有 7 例全部只在 level 0 产边，level 1 仅出现在 negative 断言里——补一个
    // level 1 的正例。
    let key = trend_key(1, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![]), level_with(vec![buy1_point(40, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 40);

    assert_eq!(delta.len(), 1, "level=1 的一类点应在该级别产边");
    assert_eq!(delta[0].bsp_level, 1);
    assert_eq!(delta[0].key.event, key);
    assert_eq!(delta[0].key.bsp.level, 1);
    assert!(bridge.edges_for_bsp_point(0, 40).is_empty(), "level 0 不应误配");
    assert_eq!(bridge.edges_for_bsp_point(1, 40).len(), 1);
}

// debug_assert! 在 release profile 编译为空操作——本测试只锁 debug 臂（`cargo test --lib`
// 基线口径），release 臂（`cargo test --lib --release`）天然跳过，不构成漏测（LOW-2 的静默
// 覆盖本身在 release 下确实不会 panic，这是 debug_assert 的既定语义，不是本测试的缺口）。
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "trend_index_by_interval_end 静默覆盖")]
fn trend_index_collision_trips_debug_assert() {
    // MED-2 缺口：`trend_index_by_interval_end`（三类专用）键冲突此前无留痕（LOW-2）。
    // 构造两个不同 episode 但共享 `(level, side, parent, interval.1)` 的病理输入，锁
    // debug_assert 确实会响，而不是静默覆盖。
    let key_a = trend_key(0, (6, 19), 25);
    let key_b = trend_key(0, (30, 44), 45);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key_a, (25, 60)), trend_observation(key_b, (45, 60))], 60);
    let streams = book.streams();
    let classification = Classification { levels: vec![level_with(vec![])] };
    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &streams, 60);
}

// ── HIGH-3 修复：跨 as_of 平价锁（对齐 N1 `..._full_replay_equals_incremental` / ────────────
// N3 `chain_certificate_book_incremental_equals_full_replay` 先例） ─────────────────────────

#[test]
fn bridge_book_incremental_equals_full_replay_across_as_of() {
    let key = trend_key(0, (6, 19), 25);
    let mut cand_book = CandidateEventBook::default();
    let mut inputs: Vec<(Classification, CandidateStreams, usize)> = Vec::new();

    cand_book.advance(&[trend_observation(key, (25, 40))], 40);
    inputs.push((
        Classification { levels: vec![level_with(vec![buy1_point(40, 20)])] },
        cand_book.streams(),
        40,
    ));

    // 步骤 2：C 段生长 + 新增第二个同 episode 物理点（修订史场景嵌入平价锁）。
    cand_book.advance(&[trend_observation(key, (25, 55))], 55);
    inputs.push((
        Classification { levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])] },
        cand_book.streams(),
        55,
    ));

    // 步骤 3：新增二类点（继承一类锚，锚点本身也已生长过一次）。
    let mut second = buy1_point(80, 20);
    second.bits = BspBits { buy2: true, ..Default::default() };
    second.center = Some(OwnerRef::Type1Anchor(55));
    inputs.push((
        Classification { levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20), second])] },
        cand_book.streams(),
        80,
    ));

    // 步骤 4：事件失效。
    cand_book.advance(&[], 90);
    inputs.push((
        Classification { levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20), second])] },
        cand_book.streams(),
        90,
    ));

    let mut incremental = BspBridgeBook::default();
    let mut snapshots = Vec::with_capacity(inputs.len());
    for (classification, streams, as_of) in &inputs {
        incremental.advance(classification, streams, *as_of);
        snapshots.push(incremental.clone());
    }

    for step in 1..=inputs.len() {
        let mut replay = BspBridgeBook::default();
        for (classification, streams, as_of) in &inputs[..step] {
            replay.advance(classification, streams, *as_of);
        }
        assert_eq!(
            &snapshots[step - 1],
            &replay,
            "step={step}: 增量推进的簿快照必须与从零全量重放逐字段相等（跨 as_of 平价锁）"
        );
    }
}

