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
    ObservedState, ParentFingerprint, StructuralPredicates, CANDIDATE_RULE_VERSION,
};
use super::super::{Classification, LevelState};
use super::*;

fn parent_fp() -> ParentFingerprint {
    ParentFingerprint {
        center_start: 20,
        zd: 100,
        zg: 110,
    }
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
        structural_predicates: StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: key.seg_a,
        third_class_proof: None,
        interval,
        state: ObservedState::Provisional,
        first_provable_at: Some(interval.1),
        confirmed_at: None,
    }
}

fn center_at(start_index: usize) -> Center {
    Center {
        zd: 100,
        zg: 110,
        dd: 100,
        gg: 110,
        start_index,
        end_index: start_index + 3,
    }
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
        bits: BspBits {
            buy1: true,
            ..Default::default()
        },
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

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 40);

    assert_eq!(delta.len(), 1, "唯一一类点应恰好产 1 条边");
    let edge = &delta[0];
    assert_eq!(edge.key.event, key);
    assert_eq!(edge.key.bsp.class, BspPointClass::Buy1);
    assert_eq!(
        edge.key.bsp.anchor,
        vec![(6, 19), (25, 25)],
        "锚不含本点自身 source_index=40"
    );
    assert_eq!(edge.bsp_source_indices, vec![40]);
    assert_eq!(edge.head_source_index(), 40);
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

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 999)])],
    };
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

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20)])],
    };
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
    assert_eq!(
        bridge.edges().len(),
        2,
        "旧 revision 留痕，append-only 不删除"
    );
}

#[test]
fn same_as_of_rerun_is_zero_delta() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let first = bridge.advance(&classification, &streams, 40);
    assert_eq!(first.len(), 1);
    let second = bridge.advance(&classification, &streams, 40);
    assert!(
        second.is_empty(),
        "投影未变 ⟹ 幂等跳过，同 as_of 重跑零 Delta"
    );
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
    second.bits = BspBits {
        buy2: true,
        ..Default::default()
    };
    second.center = Some(OwnerRef::Type1Anchor(40));

    let classification = Classification {
        levels: vec![level_with(vec![anchor, second])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 70);

    assert_eq!(delta.len(), 2, "锚点自身一条一类边 + 二类点一条边");
    let second_edge = delta
        .iter()
        .find(|edge| edge.key.bsp.class == BspPointClass::Buy2)
        .expect("二类边必须存在");
    assert_eq!(second_edge.key.event, key, "二类边继承其一类锚的 N1 事件键");
    assert_eq!(second_edge.key.bsp.anchor, vec![(40, 40)]);
    assert_eq!(second_edge.bsp_source_indices, vec![70]);
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
        bits: BspBits {
            buy3: true,
            third_class_entry: Some(third_entry),
            ..Default::default()
        },
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center_at(20))),
        struct_break_dir: None,
        force: None,
    };
    let classification = Classification {
        levels: vec![level_with(vec![point])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 22);

    assert_eq!(delta.len(), 1);
    let edge = &delta[0];
    assert_eq!(
        edge.key.event, key,
        "三类边取离开段所在的 Trend 候选（不要求背驰确认）"
    );
    assert_eq!(edge.key.bsp.class, BspPointClass::Buy3);
    assert_eq!(
        edge.key.bsp.anchor,
        vec![(6, 15), (16, 16)],
        "回试段仅左端入锚，右端=自身 source_index 排除"
    );
    assert_eq!(edge.bsp_source_indices, vec![22]);
}

#[test]
fn third_class_point_survives_trend_event_growth_via_episode_covering() {
    // MED 修复回归（第四轮 supersede）：三类判据迁移到 episode 区间覆盖后，Trend 候选生长
    // （growth_revision）不再使已确认的三类点失联——旧的 `leave_interval.1` 精确等值判据
    // 在候选生长后会让索引键漂移（索引按当前 `event.interval.1` 建，三类点自身记录的
    // `leave_interval.1` 是过去时），与一类 R2-HIGH-2 同一失效模式。
    let key = trend_key(0, (1, 5), 6);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (6, 15))], 15);

    let third_entry = ThirdClassEntryIdentity {
        center_si: 20,
        center_zd: 100,
        center_zg: 110,
        leave_interval: (6, 15),
        retest_interval: (16, 22),
    };
    let point = BspPoint {
        source_index: 22,
        bits: BspBits {
            buy3: true,
            third_class_entry: Some(third_entry),
            ..Default::default()
        },
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center_at(20))),
        struct_break_dir: None,
        force: None,
    };
    let classification = Classification {
        levels: vec![level_with(vec![point])],
    };
    let mut bridge = BspBridgeBook::default();
    let first = bridge.advance(&classification, &book.streams(), 22);
    assert_eq!(first.len(), 1);
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Open);

    // Trend 候选继续生长（右端从 15 → 30），三类点自身 leave_interval 不变（历史坐标）。
    book.advance(&[trend_observation(key, (6, 30))], 30);
    let grown = bridge.advance(&classification, &book.streams(), 30);
    assert!(
        grown.is_empty(),
        "载荷未变（覆盖集合/状态均不变），幂等零 Delta——不是漏观察"
    );
    assert_eq!(
        bridge.heads()[0].status,
        BridgeStatus::Open,
        "生长后三类边仍必须可观察，不能失联"
    );

    book.advance(&[], 40);
    let invalidated = bridge.advance(&classification, &book.streams(), 40);
    assert_eq!(
        invalidated.len(),
        1,
        "事件失效必须同步传导——若上一步已失联，本行在旧判据下恒为 0"
    );
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Invalidated);
}

#[test]
fn third_class_multiple_points_sharing_leave_segment_collapse_to_revision_history() {
    // 与一类同构（第四轮 supersede，MED 修复）：合成夹具——两个三类物理点共享同一
    // leave_interval/retest_interval 起点，验证折叠机制本身对三类同样生效（不声称生产数据
    // 必然产生这种共享，见 `chanlun/review-results/shadow-668-review2-20260729.md` R2-MED-1）。
    let key = trend_key(0, (1, 5), 6);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (6, 15))], 15);
    let streams = book.streams();

    let entry = ThirdClassEntryIdentity {
        center_si: 20,
        center_zd: 100,
        center_zg: 110,
        leave_interval: (6, 15),
        retest_interval: (16, 16),
    };
    let point_a = BspPoint {
        source_index: 18,
        bits: BspBits {
            buy3: true,
            third_class_entry: Some(entry),
            ..Default::default()
        },
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center_at(20))),
        struct_break_dir: None,
        force: None,
    };
    let mut point_b = point_a;
    point_b.source_index = 20;

    let classification = Classification {
        levels: vec![level_with(vec![point_a, point_b])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 20);

    assert_eq!(
        delta.len(),
        1,
        "共享 leave_interval/retest_interval 起点的两个三类物理点折叠为 1 条 revision"
    );
    assert_eq!(delta[0].bsp_source_indices, vec![18, 20]);
}

// ── 查询入口（#666 裁定⑦ 对拍用） ────────────────────────────────────────────────────────

#[test]
fn edges_for_bsp_point_finds_by_level_and_source_index() {
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &streams, 40);

    assert_eq!(bridge.edges_for_bsp_point(0, 40).len(), 1);
    assert!(bridge.edges_for_bsp_point(0, 999).is_empty());
    assert!(bridge.edges_for_bsp_point(1, 40).is_empty());
}

// ── HIGH-1/HIGH-2 修复：episode 覆盖判据（评审 #670 回炉，第三轮 supersede 裁定①⑤） ────────

#[test]
fn first_class_multiple_points_observed_simultaneously_collapse_to_one_revision() {
    // 同一 episode（同 seg_a/c_start）内两个物理一类点**同时并存**于同一次 observe() 扫描——
    // 折叠为一条载荷集合观察，产 1 条 revision（第四轮 supersede，修复 #670 R2-HIGH-1：旧实现
    // 按物理点顺序逐个 `apply` 会把「并存」错判成「时间序修订」，见模块头「载荷形态」段）。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 55))], 55);
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 55);

    assert_eq!(
        delta.len(),
        1,
        "同时并存的两个物理点折叠为一条 revision，不是两条"
    );
    assert_eq!(
        delta[0].bsp_source_indices,
        vec![40, 55],
        "覆盖点集升序去重"
    );
    assert_eq!(
        bridge.edges().len(),
        1,
        "append-only：本次观察只留一条 revision"
    );
    let heads = bridge.heads();
    assert_eq!(heads.len(), 1, "簿内该 episode 只有一条链头（身份唯一）");
    assert_eq!(heads[0].bsp_source_indices, vec![40, 55]);
    assert_eq!(
        heads[0].head_source_index(),
        55,
        "链头 = 集合内最大 source_index（目前所见最新递进点）"
    );
}

#[test]
fn first_class_new_point_added_later_appends_revision_with_expanded_covered_set() {
    // 真正的「修订史」场景（与上一条「同时并存零折叠」区分）：点 40 先被观察到（revision 0），
    // 点 55 在**后续** as_of 才出现（revision 1，覆盖集合从 {40} 扩到 {40,55}）——新信息到达
    // 才追加 revision，append-only 留痕两条。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let mut bridge = BspBridgeBook::default();

    let first_delta = bridge.advance(
        &Classification {
            levels: vec![level_with(vec![buy1_point(40, 20)])],
        },
        &book.streams(),
        40,
    );
    assert_eq!(first_delta.len(), 1);
    assert_eq!(first_delta[0].bsp_source_indices, vec![40]);
    assert_eq!(first_delta[0].revision, 0);

    book.advance(&[trend_observation(key, (25, 55))], 55);
    let second_delta = bridge.advance(
        &Classification {
            levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])],
        },
        &book.streams(),
        55,
    );
    assert_eq!(
        second_delta.len(),
        1,
        "点集扩大 ⟹ 追加恰好 1 条 revision（不是逐点各追加一条）"
    );
    assert_eq!(second_delta[0].bsp_source_indices, vec![40, 55]);
    assert_eq!(second_delta[0].revision, 1);
    assert_eq!(second_delta[0].supersedes_revision, Some(0));
    assert_eq!(bridge.edges().len(), 2, "append-only：两条 revision 都留痕");
    assert_eq!(
        bridge.heads().len(),
        1,
        "身份唯一：仍是同一个 episode/BridgeKey"
    );
}

#[test]
fn multi_point_episode_same_as_of_rerun_is_zero_delta() {
    // R2-HIGH-1 回归锁：多物理点 episode 下，同一 `(classification, streams, as_of)` 重跑
    // 必须零 Delta——旧实现每次重跑会无条件 append len(covered) 条 churn revision（评审 #670
    // 复核探针 300k 窗实测每次 +12、边数 29→41→53 无上界增长）。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 55))], 55);
    let streams = book.streams();
    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let first = bridge.advance(&classification, &streams, 55);
    assert_eq!(first.len(), 1);
    assert_eq!(bridge.edges().len(), 1);

    let second = bridge.advance(&classification, &streams, 55);
    assert!(
        second.is_empty(),
        "同输入重跑，多物理点 episode 下仍须零 Delta"
    );
    let third = bridge.advance(&classification, &streams, 55);
    assert!(third.is_empty());
    assert_eq!(
        bridge.edges().len(),
        1,
        "重跑不追加任何 revision（无上界增长的回归锁）"
    );
}

#[test]
fn invalidation_after_multi_point_episode_covers_all_points_and_stays_queryable() {
    // R2-HIGH-2 回归锁：多物理点 episode 失效后，链头不得回退到遍历序第一个物理点、且全部
    // 物理点必须继续可查（旧实现下：链头从 55 回退到 40，point 55 从 `edges_for_bsp_point`
    // 静默消失——评审 #670 复核探针 300k 窗实测 7/29 点查无）。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 55))], 55);
    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &book.streams(), 55);
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Open);
    assert_eq!(bridge.heads()[0].bsp_source_indices, vec![40, 55]);

    book.advance(&[], 60); // 缺席即失效。
    let invalidated = bridge.advance(&classification, &book.streams(), 60);
    assert_eq!(
        invalidated.len(),
        1,
        "失效应恰好追加 1 条 revision（覆盖集合不变，仅状态转终态）"
    );
    assert_eq!(bridge.edges().len(), 2);

    let head = &bridge.heads()[0];
    assert_eq!(head.status, BridgeStatus::Invalidated);
    assert_eq!(
        head.bsp_source_indices,
        vec![40, 55],
        "链头不应回退——全部物理点仍在同一条终态 revision 里"
    );
    assert_eq!(head.head_source_index(), 55);

    assert_eq!(
        bridge.edges_for_bsp_point(0, 40).len(),
        1,
        "point 40 失效后仍可查"
    );
    assert_eq!(
        bridge.edges_for_bsp_point(0, 55).len(),
        1,
        "point 55 失效后仍可查（旧实现在此静默丢失）"
    );
    assert_eq!(
        bridge.edges_for_bsp_point(0, 40)[0].status,
        BridgeStatus::Invalidated
    );
}

#[test]
fn trend_event_growth_does_not_orphan_earlier_first_class_point() {
    // MED-1 + HIGH-3 负控：C 段生长（growth_revision）不产生新的、恰好落在新右端上的物理点时，
    // 旧判据（右端等值）会从这一步起再也观察不到该点——边永远停在生长前的旧状态，此后事件转
    // Invalidated 也不会同步（观察缺失 ⟹ `apply` 从未被调用）。修复后的判据（episode 区间覆盖）
    // 会持续观察到该点：生长这一步本身载荷未变（同 `source_index` 集合/同 `status`）⟹ 按幂等
    // 法则正确地零 Delta（不是漏观察）；真正的分辨力在下一步——事件失效必须能同步传导。旧判据
    // 在此分辨点上必然失败（观察从生长步起彻底消失，failure 会在最终 Invalidated 断言处炸出）。
    let key = trend_key(0, (6, 19), 25);
    let mut book = CandidateEventBook::default();
    book.advance(&[trend_observation(key, (25, 40))], 40);
    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let first = bridge.advance(&classification, &book.streams(), 40);
    assert_eq!(first.len(), 1);
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Open);

    // C 段右端从 40 生长到 55，没有新的物理一类点落在新右端。载荷未变 ⟹ 幂等零 Delta
    // （不是漏观察——`apply` 仍被调用，只是投影相等而跳过 append）。
    book.advance(&[trend_observation(key, (25, 55))], 55);
    let grown = bridge.advance(&classification, &book.streams(), 55);
    assert!(
        grown.is_empty(),
        "载荷未变，幂等零 Delta（同 same_as_of_rerun_is_zero_delta 精神，跨 as_of 亦然）"
    );
    assert_eq!(bridge.heads()[0].bsp_source_indices, vec![40]);
    assert_eq!(bridge.heads()[0].status, BridgeStatus::Open);

    // 事件后续失效：边必须同步转终态，不能卡在生长前的旧状态（MED-1 核心断言）。
    book.advance(&[], 60);
    let invalidated = bridge.advance(&classification, &book.streams(), 60);
    assert_eq!(
        invalidated.len(),
        1,
        "事件转 Invalidated 应追加一条新 revision"
    );
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
        &[
            trend_observation(key_a, (25, 40)),
            trend_observation(key_b, (90, 110)),
        ],
        110,
    );
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(110, 20)])],
    };
    let mut bridge = BspBridgeBook::default();
    let delta = bridge.advance(&classification, &streams, 110);

    assert_eq!(delta.len(), 2);
    let key_for_40 = &delta
        .iter()
        .find(|e| e.bsp_source_indices == vec![40])
        .expect("point 40 应产边")
        .key;
    let key_for_110 = &delta
        .iter()
        .find(|e| e.bsp_source_indices == vec![110])
        .expect("point 110 应产边")
        .key;
    assert_ne!(
        key_for_40, key_for_110,
        "不同 episode 必须产生不同 BridgeKey，不得因同 parent 而误合并"
    );
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
    assert!(
        bridge.edges_for_bsp_point(0, 40).is_empty(),
        "level 0 不应误配"
    );
    assert_eq!(bridge.edges_for_bsp_point(1, 40).len(), 1);
}

// debug_assert! 在 release profile 编译为空操作——本测试只锁 debug 臂（`cargo test --lib`
// 基线口径），release 臂（`cargo test --lib --release`）天然跳过，不构成漏测。
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "episode 归属应唯一")]
fn overlapping_episodes_trip_find_episode_debug_assert() {
    // 两个不同 episode（不同 seg_a/c_start）但区间重叠，覆盖同一个第三类点的 `leave_interval.1`
    // ——`find_episode` 的 `debug_assert` 应响，而不是静默择一（第四轮 supersede：替代已随
    // `trend_index_by_interval_end` 一并移除的 `trend_index_collision_trips_debug_assert`——
    // 三类判据迁移到统一的 `find_episode` 反查后，静默覆盖风险同样迁移到这一处，LOW-2 精神延续）。
    let key_a = trend_key(0, (6, 19), 25);
    let key_b = trend_key(0, (10, 24), 30); // c_start=30，区间 [30,60] 与 key_a 的 [25,60] 重叠
    let mut book = CandidateEventBook::default();
    book.advance(
        &[
            trend_observation(key_a, (25, 60)),
            trend_observation(key_b, (30, 60)),
        ],
        60,
    );
    let streams = book.streams();

    let third_entry = ThirdClassEntryIdentity {
        center_si: 20,
        center_zd: 100,
        center_zg: 110,
        leave_interval: (6, 45), // .1=45 落在两个 episode 区间交集 [30,60] 内
        retest_interval: (46, 50),
    };
    let point = BspPoint {
        source_index: 50,
        bits: BspBits {
            buy3: true,
            third_class_entry: Some(third_entry),
            ..Default::default()
        },
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center_at(20))),
        struct_break_dir: None,
        force: None,
    };
    let classification = Classification {
        levels: vec![level_with(vec![point])],
    };
    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &streams, 60);
}

// debug_assert! 在 release profile 编译为空操作——本测试只锁 debug 臂，release 臂天然跳过。
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "episode 归属应唯一")]
fn overlapping_episodes_trip_find_episode_debug_assert_via_first_class_path() {
    // 一类路径回归锁（第五轮 supersede，修复 #670 R2-HIGH-3）：`resolve_first_class_episode_points`
    // 此前绕过 `find_episode` 内联同款过滤逻辑，判据相同但机器保证覆盖不到——一个一类点同时落入
    // 两个区间重叠的 episode 时旧实现会安静产两条边，零机器信号。现改为逐点调用 `find_episode`
    // 反查，同一夹具（两个区间重叠的 episode + 一个落在交集内的一类点）下 `debug_assert` 必须响。
    let key_a = trend_key(0, (6, 19), 25);
    let key_b = trend_key(0, (10, 24), 30); // c_start=30，区间 [30,60] 与 key_a 的 [25,60] 重叠
    let mut book = CandidateEventBook::default();
    book.advance(
        &[
            trend_observation(key_a, (25, 60)),
            trend_observation(key_b, (30, 60)),
        ],
        60,
    );
    let streams = book.streams();

    let classification = Classification {
        levels: vec![level_with(vec![buy1_point(50, 20)])],
    }; // 50 ∈ [25,60] ∩ [30,60]
    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &streams, 60);
}

// ── HIGH-4 修复：跨 as_of 真平价锁（对齐 N1 `..._full_replay_equals_incremental` 先例——────────
// 两个不同驱动：增量推进序列 vs 仅用终态输入的单次全新簿，不是同一输入序列跑两遍） ─────────────

#[test]
fn bridge_book_incremental_final_state_equals_fresh_full_replay_from_empty() {
    // R2-HIGH-4 修复：旧版本「同一 `inputs` 列表跑两遍」是 `f(x)==f(x)` 重言式，结构上不可能
    // 失败（对齐 N3 最弱一面，且未抄 N3 的三条非真空锁）。真平价对齐 N1 先例——两个不同驱动：
    // 驱动 A = 逐 as_of 递进推进的增量簿（历经生长/新增二类点/失效四个中间态）；
    // 驱动 B = 仅用**终态**一步 `(classification, streams, as_of)`、从空簿单次 `advance`
    // （不是同一序列跑两遍——只喂最后一步）。二者在终态时刻的 head 载荷投影必须一致：若 A 因
    // 幂等/终态挡实现错误而把某个物理点错误地挤出/滞留在错误状态（R2-HIGH-1/R2-HIGH-2 那类
    // 偏差），B（从 ∅ 直接观察终态数据）不会重复 A 的错误，二者会分道，本锁会变红。
    let key = trend_key(0, (6, 19), 25);
    let mut cand_book = CandidateEventBook::default();
    let mut inputs: Vec<(Classification, CandidateStreams, usize)> = Vec::new();

    cand_book.advance(&[trend_observation(key, (25, 40))], 40);
    inputs.push((
        Classification {
            levels: vec![level_with(vec![buy1_point(40, 20)])],
        },
        cand_book.streams(),
        40,
    ));

    // 步骤 2：C 段生长 + 新增第二个同 episode 物理点（多物理点场景嵌入平价锁）。
    cand_book.advance(&[trend_observation(key, (25, 55))], 55);
    inputs.push((
        Classification {
            levels: vec![level_with(vec![buy1_point(40, 20), buy1_point(55, 20)])],
        },
        cand_book.streams(),
        55,
    ));

    // 步骤 3：新增二类点（继承一类锚，锚点本身也已生长过一次）。
    let mut second = buy1_point(80, 20);
    second.bits = BspBits {
        buy2: true,
        ..Default::default()
    };
    second.center = Some(OwnerRef::Type1Anchor(55));
    inputs.push((
        Classification {
            levels: vec![level_with(vec![
                buy1_point(40, 20),
                buy1_point(55, 20),
                second,
            ])],
        },
        cand_book.streams(),
        80,
    ));

    // 步骤 4：事件失效。
    cand_book.advance(&[], 90);
    inputs.push((
        Classification {
            levels: vec![level_with(vec![
                buy1_point(40, 20),
                buy1_point(55, 20),
                second,
            ])],
        },
        cand_book.streams(),
        90,
    ));

    assert!(
        inputs.len() > 1,
        "非真空锁①：跨多 as_of 步骤（对齐 N3 distinct_as_of>1）"
    );

    // 驱动 A：增量推进。
    let mut incremental = BspBridgeBook::default();
    let mut total_revisions = 0usize;
    for (classification, streams, as_of) in &inputs {
        total_revisions += incremental.advance(classification, streams, *as_of).len();
    }
    assert!(
        total_revisions > 1,
        "非真空锁②：增量序列必须产生 >1 条 revision（对齐 N3 with_edges>0——否则无法证明\
         「增量」真的经过了中间态，不是一步到位）"
    );
    assert!(
        !incremental.heads().is_empty(),
        "非真空锁③：终态簿非空（对齐 N3 certificates 非空）"
    );

    // 驱动 B：仅用终态输入，从空簿单次 advance（不是同一序列跑两遍——只喂最后一步）。
    let (final_classification, final_streams, final_as_of) = inputs.last().unwrap();
    let mut fresh = BspBridgeBook::default();
    fresh.advance(final_classification, final_streams, *final_as_of);

    assert_eq!(
        fresh.heads().len(),
        incremental.heads().len(),
        "两驱动在终态时刻的 key 集合基数必须一致"
    );
    let mut keys: Vec<_> = incremental
        .heads()
        .iter()
        .map(|edge| edge.key.clone())
        .collect();
    keys.sort();
    for key in &keys {
        let a = incremental
            .latest_of(key)
            .expect("驱动 A 必须持有该 key")
            .projection();
        let b = fresh
            .latest_of(key)
            .unwrap_or_else(|| panic!("驱动 B 必须观察到相同的 key={key:?}（否则两驱动分道）"))
            .projection();
        assert_eq!(a, b, "key={key:?}：增量终态投影 ≠ 全量重放自 ∅ 投影");
    }
}
