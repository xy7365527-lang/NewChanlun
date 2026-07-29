//! #641（N3）链证书语义锁。
//!
//! 夹具一律经 `CandidateEventBook` 产出真实事件（不手搓 `CandidateEvent` 字面量），使链侧读到的
//! 状态/区间/终态都由 N1 的修订协议产生——链的语义锁因此同时锁住「链读的是真事件」这件事。

use super::super::super::types::Side;
use super::super::cand_event::{
    CandidateEventBook, CandidateKey, CandidateKind, CandidateObservation, CandidateState,
    CandidateStreams, ParentFingerprint, StructuralPredicates, CANDIDATE_RULE_VERSION,
};
use super::*;

fn key(level: u32, c_start: usize) -> CandidateKey {
    CandidateKey {
        rule_version: CANDIDATE_RULE_VERSION,
        level,
        kind: CandidateKind::Trend,
        side: Side::Long,
        previous_center_start: Some(10),
        parent: ParentFingerprint {
            center_start: 20,
            zd: 100,
            zg: 110,
        },
        seg_a: (11, 19),
        c_start,
    }
}

/// Trend 域活候选观察（`Provisional`）。
fn obs(level: u32, c_start: usize, interval: (usize, usize)) -> CandidateObservation {
    CandidateObservation {
        key: key(level, c_start),
        kind: CandidateKind::Trend,
        center_ids: Some((10, 20)),
        candidate_group_id: 1,
        pair_id: 2,
        structural_predicates: StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: (11, 19),
        third_class_proof: None,
        interval,
        state: CandidateState::Provisional,
        first_provable_at: Some(interval.1),
        confirmed_at: None,
    }
}

/// Pan 域确认候选观察（`Confirmed`；键按 Pan 域口径不带前中枢）。
fn confirmed(level: u32, c_start: usize, interval: (usize, usize)) -> CandidateObservation {
    let mut observation = obs(level, c_start, interval);
    observation.key.kind = CandidateKind::Pan;
    observation.key.previous_center_start = None;
    observation.kind = CandidateKind::Pan;
    observation.center_ids = None;
    observation.state = CandidateState::Confirmed;
    observation
}

/// 一次性事件流（终态窗口投影口径：每次 fresh book，未观察到的身份直接不在流里）。
fn streams_of(observations: &[CandidateObservation], as_of: usize) -> CandidateStreams {
    let mut book = CandidateEventBook::default();
    book.advance(observations, as_of);
    book.streams()
}

fn head(book: &ChainCertificateBook, path: &[CandidateKey]) -> TowerChainCertificate {
    book.latest_of(&ChainKey::new(path.to_vec()))
        .expect("该链身份必须在簿中")
        .clone()
}

// ── 边 = 覆盖关系（不产传递闭包边） ──────────────────────────────────────────────────────

/// 能逐级就必须逐级：`L0 ⊆ L1 ⊆ L2` 时只有一条三节点链，**不**另产 `L2→L0` 的传递闭包边。
///
/// 反事实（把全部包含对都当边时）：`[L2,L1,L0]`、`[L2,L0]` 两条链并存，后者把在场且套得住的
/// L1 说成「跳过」——伪造的是「中间级缺失」这一事实本身（roadmap:64 护栏的镜像违规）。
#[test]
fn edges_are_cover_relation_not_transitive_closure() {
    let streams = streams_of(
        &[
            obs(2, 0, (0, 100)),
            obs(1, 10, (10, 60)),
            obs(0, 20, (20, 40)),
        ],
        100,
    );
    let paths = chain_paths(&streams);
    assert_eq!(
        paths,
        vec![vec![key(2, 0), key(1, 10), key(0, 20)]],
        "极大路径唯一，且必须经过在场的中间级"
    );
}

/// 中间级真缺时才出 skip 边；skip 边与相邻边**同权**（都由谓词裁、都构成链段）。
#[test]
fn skip_edge_and_adjacent_edge_are_equally_segments() {
    chain_probe::reset();
    // 左链：L2 ⊇ L0，L1 域整体无候选 ⟹ skip 边。右链：L2' ⊇ L1' ⟹ 相邻边。
    let streams = streams_of(
        &[
            obs(2, 0, (0, 100)),
            obs(0, 20, (20, 40)),
            obs(2, 200, (200, 300)),
            obs(1, 220, (220, 240)),
        ],
        300,
    );
    let mut book = ChainCertificateBook::default();
    book.advance(&streams, 300);

    let skip = head(&book, &[key(2, 0), key(0, 20)]);
    assert_eq!(skip.edges.len(), 1);
    assert_eq!(skip.edges[0].kind, ChainEdgeKind::Skip);
    assert!(skip.edges[0].is_segment(), "skip 边谓词判过即构成链段");
    assert_eq!(skip.segment_count(), 1);
    assert_eq!(skip.fact_edge_count(), 0);

    let adjacent = head(&book, &[key(2, 200), key(1, 220)]);
    assert_eq!(adjacent.edges[0].kind, ChainEdgeKind::Adjacent);
    assert!(adjacent.edges[0].is_segment());

    assert_eq!(
        skip.edges[0].is_segment(),
        adjacent.edges[0].is_segment(),
        "同权：链段资格只看谓词，不看边的级别形态"
    );
    let probe = chain_probe::snapshot();
    assert!(
        probe.skip_edges > 0 && probe.adjacent_edges > 0,
        "{probe:?}"
    );
}

/// 被跳过的每一级都留痕，且「缺」与「断」分三格可读，不混同。
#[test]
fn skipped_level_separates_absent_level_from_broken_level() {
    // L3 ⊇ L0；L2 有候选但落在父区间外（断-其一）；L1 有候选且在父区间内但套不住子（断-其二）。
    let streams = streams_of(
        &[
            obs(3, 0, (0, 100)),
            obs(2, 500, (500, 600)),
            obs(1, 60, (60, 90)),
            obs(0, 20, (20, 40)),
        ],
        600,
    );
    let mut book = ChainCertificateBook::default();
    book.advance(&streams, 600);
    let certificate = head(&book, &[key(3, 0), key(0, 20)]);
    assert_eq!(certificate.edges[0].kind, ChainEdgeKind::Skip);
    assert_eq!(
        certificate.edges[0].skipped_levels,
        vec![
            SkippedLevel {
                level: 1,
                alive_at_level: 1,
                inside_parent: 1,
            },
            SkippedLevel {
                level: 2,
                alive_at_level: 1,
                inside_parent: 0,
            },
        ],
        "级别升序；level1 在场且在父内（断）、level2 在场但在父外（断）"
    );

    // 对照：L1 整体无候选 ⟹ alive_at_level = 0（缺，不是断）。
    let bare = streams_of(&[obs(2, 0, (0, 100)), obs(0, 20, (20, 40))], 100);
    let mut bare_book = ChainCertificateBook::default();
    bare_book.advance(&bare, 100);
    assert_eq!(
        head(&bare_book, &[key(2, 0), key(0, 20)]).edges[0].skipped_levels,
        vec![SkippedLevel {
            level: 1,
            alive_at_level: 0,
            inside_parent: 0,
        }],
    );
}

/// 相切（端点相等）判包含（#246 全域）——链段在相切边界上成立。
#[test]
fn touching_endpoints_still_form_a_segment() {
    for interval in [(0, 60), (40, 100), (0, 100)] {
        let streams = streams_of(&[obs(2, 0, (0, 100)), obs(1, 1, interval)], 100);
        let mut book = ChainCertificateBook::default();
        book.advance(&streams, 100);
        let certificate = head(&book, &[key(2, 0), key(1, 1)]);
        assert!(
            certificate.edges[0].is_segment(),
            "相切端点必须判包含：child={interval:?}"
        );
    }
}

// ── 谓词两侧：判过 = 链段；判不过 = 事实边 + 链 Invalidated ────────────────────────────

/// 生长使子端点越出父区间 ⟹ 该边转**事实边**（照实留在 edges 里、不构成链段）⟹ 链 `Invalidated`。
#[test]
fn predicate_failure_becomes_fact_edge_and_invalidates_chain() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 20))], 20),
        20,
    );
    assert_eq!(
        head(&book, &[key(2, 0), key(1, 10)]).status,
        ChainStatus::Open
    );

    // 同一身份（c_start 不变）右端生长到 40，越出父区间右端 30。
    book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 40))], 40),
        40,
    );
    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.status, ChainStatus::Invalidated);
    assert_eq!(certificate.invalidated_at, Some(40));
    assert_eq!(
        certificate.invalidation_cause,
        Some(ChainInvalidationCause::PredicateFailed),
        "判死成因分档：谓词判不过这一支"
    );
    assert_eq!(certificate.edges.len(), 1, "事实边照实留痕，不被删除");
    assert!(!certificate.edges[0].is_segment());
    assert_eq!(certificate.segment_count(), 0);
    assert_eq!(certificate.fact_edge_count(), 1);
    assert_eq!(certificate.revision, 1);
    assert_eq!(certificate.supersedes_revision, Some(0));

    // 事实边的指名见证：哪条谓词、哪两端点（含判定时读到的区间）、判不过的直接原因。
    let breach = certificate.edges[0]
        .breach
        .expect("谓词判不过的边必须携带指名见证");
    assert_eq!(breach.predicate, CHAIN_SEGMENT_PREDICATE);
    assert_eq!(breach.parent, certificate.edges[0].parent);
    assert_eq!(breach.child, certificate.edges[0].child);
    assert_eq!(breach.parent, key(2, 0));
    assert_eq!(breach.child, key(1, 10));
    assert_eq!(breach.parent_interval, (0, 30));
    assert_eq!(breach.child_interval, (10, 40));
    assert_eq!(
        breach.reason,
        PredicateBreachReason::RightOverhang,
        "子区间右端 40 越出父区间右端 30，左端未越出"
    );

    let probe = chain_probe::snapshot();
    assert!(
        probe.fact_edges > 0 && probe.to_invalidated > 0,
        "{probe:?}"
    );
}

/// 全边均为事实边时走 `PredicateFailed` 判死，不得同时计入「地板拦下」。
#[test]
fn all_fact_edges_do_not_increment_floor_blocked_probe() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 20))], 20),
        20,
    );

    let mut head_confirmed = obs(2, 0, (0, 30));
    head_confirmed.state = CandidateState::Confirmed;
    book.advance(&streams_of(&[head_confirmed, obs(1, 10, (10, 40))], 40), 40);

    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.status, ChainStatus::Invalidated);
    assert_eq!(
        certificate.invalidation_cause,
        Some(ChainInvalidationCause::PredicateFailed)
    );
    assert_eq!(certificate.segment_count(), 0);
    assert_eq!(certificate.fact_edge_count(), certificate.edges.len());
    assert_eq!(
        chain_probe::snapshot().floor_blocked,
        0,
        "谓词判不过的事实边链不属于「地板拦下」"
    );
}

/// 终态不复活：几何恢复也不把 `Invalidated` 拉回 `Open`。
#[test]
fn terminal_status_never_revives() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 20))], 20),
        20,
    );
    book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 40))], 40),
        40,
    );
    let revisions_before = book.certificates().len();

    // 父区间随后长到 50 ⟹ 几何上又套住了；终态链不得因此复活。
    let delta = book.advance(
        &streams_of(&[obs(2, 0, (0, 50)), obs(1, 10, (10, 40))], 50),
        50,
    );
    assert!(
        delta.is_empty(),
        "终态链的重评必须零 Delta，实得 {delta:#?}"
    );
    assert_eq!(book.certificates().len(), revisions_before);
    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.status, ChainStatus::Invalidated);
    assert_eq!(certificate.invalidated_at, Some(40), "失效钟不后移");
    assert!(chain_probe::snapshot().terminal_block > 0);
}

// ── 证伪节点：留痕、被跨过、不判死上级 ──────────────────────────────────────────────────

/// 中间节点被证伪 ⟹ 降为 `Falsified` 留痕、被新长出的 skip 边跨过；链的死活改由谓词在两侧存活
/// 端点间裁（本例判过 ⟹ 链继续 `Open`，上级不被判死）。
#[test]
fn falsified_middle_node_is_crossed_and_does_not_kill_the_head() {
    chain_probe::reset();
    let alive = [
        obs(2, 0, (0, 100)),
        obs(1, 10, (10, 60)),
        obs(0, 20, (20, 40)),
    ];
    let mut book = ChainCertificateBook::default();
    book.advance(&streams_of(&alive, 100), 100);
    let path = [key(2, 0), key(1, 10), key(0, 20)];
    assert_eq!(head(&book, &path).edges.len(), 2);

    // L1 身份消失 ⟹ N1 事件簿判其 `Invalidated`（缺席失效），链侧读到 `Falsified`。
    let mut candidates = CandidateEventBook::default();
    candidates.advance(&alive, 100);
    candidates.advance(&[obs(2, 0, (0, 100)), obs(0, 20, (20, 40))], 110);

    book.advance(&candidates.streams(), 110);
    let certificate = head(&book, &path);
    assert_eq!(certificate.nodes[1].status, ChainNodeStatus::Falsified);
    assert_eq!(
        certificate.nodes[1].state,
        Some(CandidateState::Invalidated),
        "证伪节点原样留痕，不被删除、不被伪造成别的状态"
    );
    assert_eq!(certificate.edges.len(), 1, "两端存活端点之间只剩一条边");
    assert_eq!(certificate.edges[0].crossed_nodes, vec![key(1, 10)]);
    assert_eq!(certificate.edges[0].kind, ChainEdgeKind::Skip);
    assert!(certificate.edges[0].is_segment());
    assert_eq!(
        certificate.status,
        ChainStatus::Open,
        "证伪节点不判死上级——链由谓词统一裁"
    );

    let probe = chain_probe::snapshot();
    assert!(
        probe.falsified_nodes > 0 && probe.crossed_by_edge > 0,
        "{probe:?}"
    );
}

/// 链头被证伪 ⟹ 链 `Invalidated`；其余节点的留痕原样保留（照实，不因链死而抹掉）。
#[test]
fn falsified_head_invalidates_chain_but_keeps_node_traces() {
    let alive = [obs(2, 0, (0, 100)), obs(1, 10, (10, 60))];
    let mut candidates = CandidateEventBook::default();
    candidates.advance(&alive, 100);
    let mut book = ChainCertificateBook::default();
    book.advance(&candidates.streams(), 100);

    candidates.advance(&[obs(1, 10, (10, 60))], 110);
    book.advance(&candidates.streams(), 110);

    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.status, ChainStatus::Invalidated);
    assert_eq!(
        certificate.invalidation_cause,
        Some(ChainInvalidationCause::HeadInvalidated),
        "判死成因分档：链头这一支"
    );
    assert_eq!(certificate.nodes[0].status, ChainNodeStatus::Falsified);
    assert_eq!(certificate.nodes[1].status, ChainNodeStatus::Alive);
    assert!(certificate.edges.is_empty(), "只剩一个存活端点 ⟹ 无边");
}

/// 判死成因**只有裁定授权的两支**：中间节点失效不进任何判死档（它走「留痕 + 被跨过」）。
///
/// 反事实锁：若移植 B 侧的 `NodeInvalidated` 连坐支（2026-07-29 核定已 supersede），本例的链
/// 会被判死并落上该成因——它当场变红。
#[test]
fn invalidation_cause_has_only_the_two_ruled_branches() {
    let alive = [
        obs(2, 0, (0, 100)),
        obs(1, 10, (10, 60)),
        obs(0, 20, (20, 40)),
    ];
    let mut candidates = CandidateEventBook::default();
    candidates.advance(&alive, 100);
    let mut book = ChainCertificateBook::default();
    book.advance(&candidates.streams(), 100);

    candidates.advance(&[obs(2, 0, (0, 100)), obs(0, 20, (20, 40))], 110);
    book.advance(&candidates.streams(), 110);

    let certificate = head(&book, &[key(2, 0), key(1, 10), key(0, 20)]);
    assert_eq!(certificate.nodes[1].status, ChainNodeStatus::Falsified);
    assert_eq!(certificate.status, ChainStatus::Open);
    assert_eq!(
        certificate.invalidation_cause, None,
        "中间节点失效不判死 ⟹ 无成因可落"
    );
}

/// 路径节点在事件流中查无（终态窗口投影驱动）——单列为 `Absent`，不与 `Falsified` 混同。
#[test]
fn absent_node_is_recorded_separately_from_falsified() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );
    // fresh book ⟹ 上一轮的 L1 身份整个不在流里（不是 Invalidated，是查无）。
    book.advance(&streams_of(&[obs(2, 0, (0, 100))], 110), 110);

    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.nodes[1].status, ChainNodeStatus::Absent);
    assert_eq!(certificate.nodes[1].state, None);
    let probe = chain_probe::snapshot();
    assert!(
        probe.absent_node > 0 && probe.falsified_nodes == 0,
        "{probe:?}"
    );
}

/// 链头在 fresh 事件流中查无只表示投影抖动：落 `Open`，不判死。
#[test]
fn absent_head_stays_open_and_is_not_invalidated() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );

    // fresh book ⟹ 上一轮链头整个不在流里（不是 Invalidated，是查无）。
    book.advance(&streams_of(&[obs(1, 10, (10, 60))], 110), 110);

    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.status, ChainStatus::Open);
    assert_eq!(certificate.invalidation_cause, None);
    assert_eq!(certificate.nodes[0].status, ChainNodeStatus::Absent);
    assert!(
        chain_probe::snapshot().absent_node > 0,
        "必须真经过 Absent 节点分支"
    );
}

/// 链头**上一轮曾 `Confirmed`**、本轮查无——不得因「曾确认」残留而误判 `Closed`：状态只读
/// 当前这一轮的事件视图（status/state 同源），不带上一轮的记忆（#667 INFO-1 机器锁）。
///
/// 三节点夹具（#676-4 反事实收窄，见下方分辨力说明）：① 头未确认 ⟹ `Open`（基线，mid-leaf 间
/// 已有一条判过的边）。② 头转 `Confirmed`，同轮喂入一枚**不在路径内**的更低级候选令 leaf 保持
/// 可扩展（`extendable=true`）⟹ 仍 `Open`（非终态，被 resident 机制带入下一轮重评）。③ 头整个
/// 查无（`Absent`），mid/leaf 仍存活、其间保留一条判过的边（`has_segment == true`）⟹ 落
/// `Open`，非 `Closed` 非 `Invalidated`。
///
/// **分辨力**：两节点版本（原型）里头 Absent 后只剩一个存活节点，`alive_positions.windows(2)`
/// 为空 ⟹ 零边 ⟹ 地板条款（`mod.rs:602` 的 `has_segment`）单独封堵 Closed，`head_confirmed`
/// 是否正确读「当前」而非「历史」根本没被测到。三节点版本里 mid/leaf 两端仍活着、其间的边仍是
/// 有效链段，地板条款不再介入，`head_confirmed` 才是**唯一**封堵者——反事实：在 `mod.rs:599`
/// 把 `head_confirmed` 改成 `nodes[0].state == Some(Confirmed) || nodes[0].status ==
/// ChainNodeStatus::Absent`（把「曾确认」误判物化），本测试必须变红（`certificate.status`
/// 变 `Closed`）。
#[test]
fn absent_head_after_prior_confirmed_stays_open_and_is_not_invalidated() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    let path = [key(3, 0), key(2, 10), key(1, 20)];

    book.advance(
        &streams_of(
            &[
                obs(3, 0, (0, 100)),
                obs(2, 10, (10, 60)),
                obs(1, 20, (20, 40)),
            ],
            100,
        ),
        100,
    );
    let baseline = head(&book, &path);
    assert_eq!(baseline.status, ChainStatus::Open);
    assert!(
        baseline.segment_count() > 0,
        "基线必须已有有效链段，否则③轮的 Open 会被地板条款而非 head_confirmed 决定"
    );

    // 头转 Confirmed；同轮喂入一枚不在路径内的更低级候选令 leaf 保持可扩展——防止本轮本身先
    // 误判 Closed（这只是本用例站稳「头曾 Confirmed」这一前提的手段，不是待测结论）。
    let mut confirmed_head = obs(3, 0, (0, 100));
    confirmed_head.state = CandidateState::Confirmed;
    book.advance(
        &streams_of(
            &[
                confirmed_head,
                obs(2, 10, (10, 60)),
                obs(1, 20, (20, 40)),
                obs(0, 30, (25, 35)),
            ],
            105,
        ),
        105,
    );
    let mid_confirmed = head(&book, &path);
    assert_eq!(
        mid_confirmed.nodes[0].state,
        Some(CandidateState::Confirmed),
        "本轮头必须真读到 Confirmed（不是构造摆设）"
    );
    assert_eq!(mid_confirmed.status, ChainStatus::Open);

    // fresh book ⟹ 上一轮头整个不在流里（不是 Invalidated，是查无）；mid/leaf 仍存活，不再喂
    // level 0 候选 ⟹ extendable=false，其间保留一条判过的边 ⟹ has_segment=true——地板条款不
    // 介入，head_confirmed 是本轮 Closed 支唯一的封堵者。
    book.advance(
        &streams_of(&[obs(2, 10, (10, 60)), obs(1, 20, (20, 40))], 110),
        110,
    );

    let certificate = head(&book, &path);
    assert_eq!(
        certificate.status,
        ChainStatus::Open,
        "曾 Confirmed 不得残留成 Closed"
    );
    assert_eq!(certificate.invalidation_cause, None);
    assert_eq!(certificate.nodes[0].status, ChainNodeStatus::Absent);
    assert!(
        certificate.segment_count() > 0,
        "mid/leaf 必须仍构成有效链段，否则地板条款会重新顶替 head_confirmed 成为封堵者：{certificate:?}"
    );
    assert!(
        chain_probe::snapshot().absent_node > 0,
        "必须真经过 Absent 节点分支"
    );
}

// ── 终态三态 ────────────────────────────────────────────────────────────────────────────

/// `Closed` 三条件缺一不可：链头 `Confirmed` + 不可再扩展 + 全链段谓词判过。
#[test]
fn closed_requires_confirmed_head_and_no_extension() {
    // 头未确认 ⟹ Open（另两条件都满足）。
    let mut open_book = ChainCertificateBook::default();
    open_book.advance(
        &streams_of(&[obs(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );
    let opened = head(&open_book, &[key(2, 0), key(1, 10)]);
    assert_eq!(opened.status, ChainStatus::Open);
    assert!(!opened.extendable);
    assert_eq!(opened.segment_count(), 1);

    // 头确认 + 不可扩展 ⟹ Closed。
    let mut closed_book = ChainCertificateBook::default();
    closed_book.advance(
        &streams_of(&[confirmed(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );
    let certificate = head(
        &closed_book,
        &[
            {
                let mut root = key(2, 0);
                root.kind = CandidateKind::Pan;
                root.previous_center_start = None;
                root
            },
            key(1, 10),
        ],
    );
    assert_eq!(certificate.status, ChainStatus::Closed);
    assert_eq!(certificate.closed_at, Some(100));
}

/// 后来长出的子节点 = **路径扩展**：新 `ChainKey` + `extends` 指**簿内**最长真前缀；
/// 已 `Closed` 的旧链不回写、不复活（E2E-L「原谱系及其形成钟不回写」的机器载体）。
///
/// 同时锁住 `extends` 的**命中侧**：真前缀 `[L2,L1]` 在 as_of=100 已作为链落过簿 ⟹ 查簿命中 ⟹
/// 写 `extends`（否定侧见 `extends_is_none_when_the_structural_prefix_never_materialized`）。
#[test]
fn downward_extension_creates_new_key_and_leaves_closed_chain_untouched() {
    chain_probe::reset();
    let mut root = key(2, 0);
    root.kind = CandidateKind::Pan;
    root.previous_center_start = None;
    let short = [root, key(1, 10)];
    let long = [root, key(1, 10), key(0, 20)];

    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[confirmed(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );
    assert_eq!(head(&book, &short).status, ChainStatus::Closed);

    book.advance(
        &streams_of(
            &[
                confirmed(2, 0, (0, 100)),
                obs(1, 10, (10, 60)),
                obs(0, 20, (20, 40)),
            ],
            110,
        ),
        110,
    );

    let old = head(&book, &short);
    assert_eq!(old.status, ChainStatus::Closed);
    assert_eq!(old.closed_at, Some(100), "旧链形成钟不回写");
    assert_eq!(old.revision, 0, "旧链不因扩展而追加修订");

    let new = head(&book, &long);
    assert_eq!(new.extends, Some(ChainKey::new(short.to_vec())));
    assert_eq!(new.status, ChainStatus::Closed);
    assert_eq!(new.closed_at, Some(110));
    assert_eq!(
        old.extends, None,
        "两节点链的 proper-prefix 是单节点，不是链"
    );
    let probe = chain_probe::snapshot();
    assert_eq!(
        (probe.extends_resolved, probe.extends_not_materialized),
        (1, 0),
        "查簿命中侧真被走过：{probe:?}"
    );
}

/// ★#641 `extends` 负控（取舍裁定 comment-5121793896 第 3 条）：结构上的真前缀若**从未**作为
/// 链在簿中物化过，不得写 `extends`。
///
/// 旧实装取纯结构派生（去掉 leaf）而不查簿，BTC 100k 实测 12 条 `extends` **全部**指向从未存在
/// 的链。本例复现该形态：三节点链一次成型，其两节点真前缀 `[L2,L1]` 因 L1 有存活子而从来不是
/// 极大路径 ⟹ 簿内查无 ⟹ `extends` 必须为 `None`。
#[test]
fn extends_is_none_when_the_structural_prefix_never_materialized() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(
            &[
                obs(2, 0, (0, 100)),
                obs(1, 10, (10, 60)),
                obs(0, 20, (20, 40)),
            ],
            100,
        ),
        100,
    );

    let long = head(&book, &[key(2, 0), key(1, 10), key(0, 20)]);
    assert_eq!(long.key.path.len(), 3, "结构上确实存在两节点真前缀");
    assert!(
        book.latest_of(&ChainKey::new(vec![key(2, 0), key(1, 10)]))
            .is_none(),
        "该真前缀从未落簿"
    );
    assert_eq!(
        long.extends, None,
        "簿内查无 ⟹ 不写 extends（禁把结构派生冒充谱系事实）"
    );
    let probe = chain_probe::snapshot();
    assert_eq!(
        (probe.extends_resolved, probe.extends_not_materialized),
        (0, 1),
        "幽灵前缀被计数、不被静默写入：{probe:?}"
    );
}

/// 库内只读读数口 [`ChainCertificateBook::summarize`] / [`ChainCertificateBook::digest`]：
/// 与簿内对象逐格自洽，且 `digest` 对同一簿确定、对不同簿相异。
///
/// 地板条款监视格 `closed_with_zero_segments` 在本例（与全部真实窗口）恒 0——它是「地板被绕过」
/// 的机器警报，不是可选统计。
#[test]
fn summarize_and_digest_are_book_internal_readouts() {
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(
            &[
                confirmed(2, 0, (0, 100)),
                obs(1, 10, (10, 60)),
                obs(0, 20, (20, 40)),
                obs(2, 200, (200, 300)),
                obs(0, 220, (220, 240)),
            ],
            300,
        ),
        300,
    );

    let summary = book.summarize();
    assert_eq!(summary.revisions, book.certificates().len());
    assert_eq!(summary.chains, book.heads().len());
    assert_eq!(
        summary.open + summary.closed + summary.invalidated,
        summary.chains,
        "三态划分链身份集合"
    );
    assert_eq!(
        summary.invalidated_head + summary.invalidated_predicate,
        summary.invalidated,
        "判死成因分档合计恒等于 Invalidated 数"
    );
    assert_eq!(summary.adjacent_edges + summary.skip_edges, summary.edges);
    assert_eq!(summary.segments + summary.fact_edges, summary.edges);
    assert_eq!(
        summary.closed_with_zero_segments, 0,
        "地板条款监视格：`Closed` 且零链段恒 0"
    );
    assert!(
        summary.skip_edges > 0,
        "本夹具含 L2→L0 的 skip 边：{summary:?}"
    );
    assert_eq!(summary.by_path_len.values().sum::<usize>(), summary.chains);
    assert_eq!(
        summary.by_root_level.values().sum::<usize>(),
        summary.chains
    );
    assert_eq!(
        summary.nodes_alive + summary.nodes_falsified + summary.nodes_absent,
        book.heads()
            .iter()
            .map(|certificate| certificate.nodes.len())
            .sum::<usize>()
    );

    let same = book.digest();
    assert_eq!(same, book.digest(), "同一簿的摘要确定");
    let empty = ChainCertificateBook::default();
    assert_ne!(same, empty.digest(), "空簿与非空簿的摘要必须相异");
}

/// 非终态链在子节点出现后转为**可扩展**：状态仍 `Open`，但载荷变 ⟹ 追加修订（不是幂等跳过）。
#[test]
fn resident_open_chain_becomes_extendable_and_appends_a_payload_revision() {
    chain_probe::reset();
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );
    book.advance(
        &streams_of(
            &[
                obs(2, 0, (0, 100)),
                obs(1, 10, (10, 60)),
                obs(0, 20, (20, 40)),
            ],
            110,
        ),
        110,
    );
    let certificate = head(&book, &[key(2, 0), key(1, 10)]);
    assert_eq!(certificate.status, ChainStatus::Open);
    assert!(certificate.extendable, "leaf 内部长出存活候选 ⟹ 可再扩展");
    assert_eq!(certificate.revision, 1);
    assert!(chain_probe::snapshot().payload_revision > 0);
}

/// ★#641 地板条款（comment-5121572134，2026-07-29 编排者裁定）：链必须**至少一条有效链段**
/// 才能 `Closed`；链头独活（全下级证伪/缺失 ⟹ 链段集合为空）= **永远 Open**。
///
/// 本测试是该裁定点名要翻转的那一枚（原名
/// `head_only_survivor_closes_by_vacuous_segment_condition`，原钉的行为是「真空成立 ⟹ 可
/// Closed」）。翻转后钉的是裁定字面：**不可 Closed**。与 E2E-L 原型 `UnresolvedFloor` 语义对齐
/// （原型 §5:188/§6.1，git `640609071d`）。
///
/// floor 的**完整**口径（`CloseFloor_at` 三型、级别地板）仍留 fog 另裁（取舍裁定
/// comment-5121793896 第 5 条），本测试只钉「≥1 有效链段」这一格。
#[test]
fn head_only_survivor_stays_open_by_floor_conjunct() {
    chain_probe::reset();
    // 非终态起手：头未确认 ⟹ 首轮 Open。
    let mut candidates = CandidateEventBook::default();
    candidates.advance(&[obs(2, 400, (400, 500)), obs(1, 410, (410, 460))], 500);
    let mut book = ChainCertificateBook::default();
    book.advance(&candidates.streams(), 500);
    let path = [key(2, 400), key(1, 410)];
    assert_eq!(head(&book, &path).status, ChainStatus::Open);
    assert_eq!(head(&book, &path).segment_count(), 1);

    // 头转 `Confirmed`（同一身份，区间不变）+ 唯一下级缺席失效 ⟹ 链段集合为空。
    let mut head_confirmed = obs(2, 400, (400, 500));
    head_confirmed.state = CandidateState::Confirmed;
    candidates.advance(&[head_confirmed], 510);
    book.advance(&candidates.streams(), 510);

    let certificate = head(&book, &path);
    assert_eq!(certificate.nodes[0].status, ChainNodeStatus::Alive);
    assert_eq!(certificate.nodes[0].state, Some(CandidateState::Confirmed));
    assert_eq!(certificate.nodes[1].status, ChainNodeStatus::Falsified);
    assert!(certificate.edges.is_empty(), "只剩链头存活 ⟹ 无边");
    assert_eq!(certificate.segment_count(), 0);
    assert!(
        !certificate.extendable,
        "链头内部已无存活候选 ⟹ 旧口径的另两条 Closed 条件都满足"
    );
    assert_eq!(
        certificate.status,
        ChainStatus::Open,
        "地板条款：链段集合为空 ⟹ 不可 Closed（真空成立不算数）"
    );
    assert_eq!(certificate.closed_at, None, "被地板拦下 ⟹ 形成钟不落");

    let probe = chain_probe::snapshot();
    assert!(
        probe.floor_blocked > 0,
        "地板合取必须真被走到（不是靠注释声明）：{probe:?}"
    );
}

/// 地板条款只挡「零链段」这一格：有链段的链不受影响，`Closed` 照常。
///
/// 反事实锁：若把地板合取写成级别地板（如「必须降到 L0 才能 Close」），本例的 L2→L1 两节点链
/// 会被一并挡住——它当场变红。floor 完整口径留 fog 另裁，本模块只落「≥1 有效链段」。
#[test]
fn floor_conjunct_does_not_block_a_chain_that_has_a_segment() {
    let mut root = key(2, 0);
    root.kind = CandidateKind::Pan;
    root.previous_center_start = None;
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[confirmed(2, 0, (0, 100)), obs(1, 10, (10, 60))], 100),
        100,
    );
    let certificate = head(&book, &[root, key(1, 10)]);
    assert_eq!(certificate.segment_count(), 1);
    assert_eq!(
        certificate.leaf_level, 1,
        "未降到 L0 也可 Closed（无级别地板）"
    );
    assert_eq!(certificate.status, ChainStatus::Closed);
    assert_eq!(certificate.closed_at, Some(100));
}

// ── 幂等 / append-only ──────────────────────────────────────────────────────────────────

/// 同一 `as_of`、同一事件流重跑 ⟹ 零 Delta（投影相同即幂等跳过）。
#[test]
fn same_as_of_replay_is_idempotent() {
    chain_probe::reset();
    let streams = streams_of(
        &[
            obs(2, 0, (0, 100)),
            obs(1, 10, (10, 60)),
            obs(0, 20, (20, 40)),
        ],
        100,
    );
    let mut book = ChainCertificateBook::default();
    let first = book.advance(&streams, 100);
    assert_eq!(first.len(), 1);
    let second = book.advance(&streams, 100);
    assert!(
        second.is_empty(),
        "同 as_of 重跑必须零 Delta，实得 {second:#?}"
    );
    assert_eq!(book.certificates().len(), 1);
    assert!(chain_probe::snapshot().idempotent_skip > 0);
}

/// append-only：修订只追加，旧 revision 原样保留（禁删除模拟失效）。
///
/// 同时锁住**链载荷只引用 EventKey**这一口径（E2E-L §S8：`node_event_keys[]` 只存 key 引用，
/// 节点几何由 validator 回联事件 head 取）：节点 C 段右端生长而拓扑与谓词结论不变时，链侧零
/// Delta——否则每根 bar 的活段生长都会给每条链刷一次修订，链簿变成事件簿的影子。
#[test]
fn revisions_are_append_only_and_node_growth_alone_is_not_a_chain_revision() {
    let mut book = ChainCertificateBook::default();
    book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 20))], 20),
        20,
    );

    let growth_only = book.advance(
        &streams_of(&[obs(2, 0, (0, 30)), obs(1, 10, (10, 25))], 25),
        25,
    );
    assert!(
        growth_only.is_empty(),
        "节点区间生长不改链拓扑与谓词结论 ⟹ 链零 Delta，实得 {growth_only:#?}"
    );

    // 长出子节点 ⟹ 可扩展性翻转（链载荷真变）⟹ 追加修订。
    book.advance(
        &streams_of(
            &[
                obs(2, 0, (0, 30)),
                obs(1, 10, (10, 25)),
                obs(0, 12, (12, 18)),
            ],
            30,
        ),
        30,
    );
    // 子端点生长越出父区间 ⟹ 事实边 ⟹ 终态。
    book.advance(
        &streams_of(
            &[
                obs(2, 0, (0, 30)),
                obs(1, 10, (10, 40)),
                obs(0, 12, (12, 18)),
            ],
            40,
        ),
        40,
    );

    let history: Vec<_> = book
        .certificates()
        .iter()
        .filter(|certificate| certificate.key.path == vec![key(2, 0), key(1, 10)])
        .collect();
    assert_eq!(history.len(), 3);
    assert_eq!(
        history.iter().map(|c| c.revision).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(history[0].status, ChainStatus::Open, "旧 revision 不被改写");
    assert!(!history[0].extendable);
    assert_eq!(history[1].status, ChainStatus::Open);
    assert!(history[1].extendable);
    assert_eq!(history[2].status, ChainStatus::Invalidated);
    assert_eq!(history[0].observed_at, 20);
    assert_eq!(history[2].observed_at, 20, "入簿钟不后移");
}

/// 孤立候选（既无存活父也无存活子）不成链，但单列计数不静默丢。
#[test]
fn isolated_candidates_are_counted_not_silently_dropped() {
    chain_probe::reset();
    // 两条区间互不相含 ⟹ 各自既无父也无子，两个都是孤立根。
    let streams = streams_of(&[obs(2, 0, (0, 10)), obs(1, 500, (500, 600))], 600);
    assert!(chain_paths(&streams).is_empty());
    assert_eq!(chain_probe::snapshot().isolated_roots, 2);
}

/// 单节点不是链（`ChainKey::new` 当场失败，不产出没有任何链段的畸形证书）。
#[test]
#[should_panic(expected = "链身份至少两个节点")]
fn single_node_path_is_not_a_chain() {
    let _ = ChainKey::new(vec![key(1, 10)]);
}

/// `breach_reason` 只读 `interval_is_sub` 已判过的端点值（不复算），构造直接的 [`CandidateEvent`]
/// 覆盖四个失败合取项，逐项核验 `breach_reason` 命中的档确是 `candidate_is_sub` 判假的直接原因。
/// 一致性锁（#653 影子评审 LOW-2）：若未来 `interval_is_sub` 口径变动（相切规则、容差、
/// 半开区间），此测试须同步改写，否则会当场变红——防止两侧口径静默漂移。
fn candidate_event_for_breach(level: u32, interval: (usize, usize)) -> CandidateEvent {
    CandidateEvent {
        key: key(level, interval.0),
        kind: CandidateKind::Trend,
        event_level: level,
        center_ids: Some((10, 20)),
        candidate_group_id: 1,
        pair_id: 2,
        structural_predicates: StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: (11, 19),
        third_class_proof: None,
        interval,
        state: CandidateState::Provisional,
        observed_at: 0,
        first_provable_at: Some(interval.1),
        confirmed_at: None,
        invalidated_at: None,
        revision: 0,
        supersedes_revision: None,
        revision_at: 0,
    }
}

#[test]
fn breach_reason_matches_interval_is_sub_conjuncts() {
    let parent_level = 2;
    let child_level = 1;
    let cases: &[((usize, usize), (usize, usize), PredicateBreachReason)] = &[
        (
            (5, 3),
            (0, 10),
            PredicateBreachReason::ChildIntervalDegenerate,
        ),
        (
            (2, 4),
            (10, 8),
            PredicateBreachReason::ParentIntervalDegenerate,
        ),
        ((0, 5), (2, 10), PredicateBreachReason::LeftOverhang),
        ((5, 15), (2, 10), PredicateBreachReason::RightOverhang),
        ((0, 15), (2, 10), PredicateBreachReason::BothEndsOverhang),
    ];

    for &(child_interval, parent_interval, expected) in cases {
        let child = candidate_event_for_breach(child_level, child_interval);
        let parent = candidate_event_for_breach(parent_level, parent_interval);

        assert!(
            !candidate_is_sub(&child, &parent),
            "夹具用例必须真实判假：child={child_interval:?} parent={parent_interval:?}"
        );
        assert_eq!(
            breach_reason(&child, &parent),
            expected,
            "child={child_interval:?} parent={parent_interval:?}"
        );
    }
}
