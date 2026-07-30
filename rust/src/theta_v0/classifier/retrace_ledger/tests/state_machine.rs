//! 三态状态机 + 观察适配器（AC①：状态迁移表驱动全盖 + 残废四桶拒收报错）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 注册（裁定一：注册拍四条边快照为身份）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn registration_pins_four_edge_snapshot_as_identity() {
    let mut book = ledger();
    let center = frame(1_200);
    let step = book.observe(&up_input(center, 3, None, 500)).unwrap();

    assert_eq!(step.key, key_of(center, 3));
    assert_eq!(step.state, RetraceState::Provisional);
    let entry = book.entry(&step.key).unwrap();
    assert_eq!(
        kinds(entry),
        vec![
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned
        ]
    );
    let registration = entry.registration().unwrap();
    assert_eq!(registration.frame, center, "身份锚 = 注册拍的四条边");
    assert_eq!(
        registration.side,
        RetraceSide::Buy,
        "向上离开 ⟹ 三类买点候选"
    );
    assert_eq!(entry.registered_as_of, 500);
    assert_eq!(
        step.key.pair().retest_move_index,
        4,
        "严格相邻 pair 是身份的纯函数"
    );
    settled(&book);
}

#[test]
fn down_departure_registers_sell_side_candidate() {
    let mut book = ledger();
    let center = frame(1_200);
    let step = book.observe(&down_input(center, 7, None, 500)).unwrap();
    assert_eq!(
        book.entry(&step.key).unwrap().side(),
        Some(RetraceSide::Sell)
    );
    settled(&book);
}

#[test]
fn pending_observations_keep_provisional_without_any_timeout() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    for as_of in [600, 700, 10_000, 1_000_000] {
        let step = book.observe(&up_input(center, 3, None, as_of)).unwrap();
        assert_eq!(
            step.state,
            RetraceState::Provisional,
            "缺席 ≠ 消失（裁定七）"
        );
        assert!(step.delta.is_empty(), "未决观察不产修订");
    }
    let entry = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(entry.terminal_as_of, None, "永不超时处死");
    assert_eq!(entry.revisions.len(), 2);
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 判胜 / 判败
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn success_settles_confirmed_with_evidence() {
    let mut book = ledger();
    let center = frame(1_200);
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();

    assert_eq!(step.state, RetraceState::Confirmed);
    let entry = book.entry(&step.key).unwrap();
    assert_eq!(
        kinds(entry),
        vec![
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
            RetraceRevisionKind::Confirmed
        ]
    );
    assert_eq!(entry.terminal_as_of, Some(500), "落锤钟 = 判决知情时");
    assert_eq!(entry.not_constituted_reason(), None, "正向终态无原因码");
    assert!(entry.terminal_evidence().unwrap().retest_end.is_some());
    settled(&book);
}

#[test]
fn retest_reentry_settles_not_constituted_without_center_event() {
    let mut book = ledger();
    let center = frame(1_200);
    let step = book
        .observe(&up_input(
            center,
            3,
            Some(RetraceOutcome::RetestReenters),
            500,
        ))
        .unwrap();

    assert_eq!(step.state, RetraceState::Invalidated);
    let entry = book.entry(&step.key).unwrap();
    assert_eq!(
        entry.not_constituted_reason(),
        Some(NotConstitutedReason::RetestReentered),
        "判败名分 = 从未成立族（裁定三）"
    );
    assert!(
        book.death_certificate(step.key.anchor()).is_none(),
        "判败不派生任何中枢生命周期事件（中枢未破坏）"
    );
    assert!(book.established().is_empty(), "判败不进成立档");
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 终态吸收 + 迟到静默吸收 + 警报计数（裁定三）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn terminal_absorbs_late_input_silently_and_counts_alarm() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let before = book.entry(&key_of(center, 3)).unwrap().clone();

    for as_of in [501, 502, 600] {
        let late = book
            .observe(&up_input(
                center,
                3,
                Some(RetraceOutcome::RetestReenters),
                as_of,
            ))
            .unwrap();
        assert!(late.absorbed_late(), "终态后同身份迟到输入 ⟹ 静默吸收");
        assert!(late.delta.is_empty(), "静默 = 零修订");
        assert_eq!(late.state, RetraceState::Confirmed, "禁复活");
    }

    let after = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(after.revisions, before.revisions, "留档一个 bit 不动");
    assert_eq!(after.terminal_as_of, before.terminal_as_of, "落锤钟不后移");
    assert_eq!(book.alarms().late_absorbed, 3, "迟到吸收进警报计数");
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// Restart（裁定三：新档注册，谱系载荷记前任）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn restart_opens_new_entry_carrying_previous_identity() {
    let mut book = ledger();
    let first = frame(1_200);
    book.observe(&up_input(
        first,
        3,
        Some(RetraceOutcome::RetestReenters),
        500,
    ))
    .unwrap();

    // 重回 ⟹ 快照作废，下一代候选按延展后的新窗口重新拍照（裁定一）。
    let extended = frame(1_400);
    let step = book.observe(&up_input(extended, 5, None, 700)).unwrap();

    let entry = book.entry(&step.key).unwrap();
    assert_eq!(
        entry.restarted_from(),
        Some(key_of(first, 3)),
        "Restart 新档载荷记前任身份"
    );
    assert_eq!(
        kinds(entry),
        vec![
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
            RetraceRevisionKind::Restarted {
                previous: key_of(first, 3)
            }
        ]
    );
    assert_eq!(entry.state, RetraceState::Provisional, "新档从未决起步");
    assert_eq!(book.len(), 2, "历史身份全部留档不删");
    assert_eq!(
        book.entry(&key_of(first, 3)).unwrap().state,
        RetraceState::Invalidated,
        "前任条目一个 bit 不动"
    );
    settled(&book);
}

#[test]
fn unsettled_active_candidate_rejects_new_departure() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let rejection = book
        .observe(&up_input(frame(1_400), 5, None, 700))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::ActiveCandidateNotSettled {
            active: key_of(center, 3),
            incoming_departure_move_index: 5,
        },
        "同一中枢未判完而来新 departure ⟹ provider 有病，fail-loud（裁定二）"
    );
    assert_eq!(book.len(), 1, "拒收零建仓");
    assert_eq!(book.alarms().registration_rejected, 1);
    settled(&book);
}

#[test]
fn distinct_centers_keep_independent_active_candidates() {
    let mut book = ledger();
    let a = frame(1_200);
    let b = other_frame(300);
    book.observe(&up_input(a, 3, None, 500)).unwrap();
    book.observe(&up_input(b, 11, None, 520)).unwrap();

    assert_eq!(book.active_candidate(a.anchor()), Some(key_of(a, 3)));
    assert_eq!(book.active_candidate(b.anchor()), Some(key_of(b, 11)));
    assert_eq!(book.len(), 2, "不同中枢各有活跃候选，天然并存（裁定二）");
    settled(&book);
}

/// 裁定四：Success 后同中枢新注册——**S2 落地拒收**（票 #622，修复影子评审 #621 MEDIUM-4：
/// 判据统一为 `death_certificate(anchor)` 查法，本计数与实际拒收次数恒等）。
#[test]
fn confirmed_center_new_departure_is_rejected_as_dead_center_reentry() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let certificate = book.death_certificate(center.anchor()).unwrap();

    let attempted = key_of(frame(1_400), 5);
    let rejection = book
        .observe(&up_input(frame(1_400), 5, None, 700))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::DeadCenterReentry {
            anchor: center.anchor(),
            attempted,
            death_certificate: certificate,
        },
        "死人挂号：给死人挂号 fail-loud 拒收（裁定四）"
    );
    assert_eq!(
        book.alarms().dead_center_registrations,
        1,
        "计数与拒收次数恒等"
    );
    assert_eq!(book.len(), 1, "拒收零建仓，新档从未进账");
    assert!(book.entry(&attempted).is_none());
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 残废四桶：注册期拒收报错，错误与终态分家（裁定三）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn malformed_frame_rejected_at_registration() {
    let mut book = ledger();
    let broken = CenterFrame {
        zd: ZG,
        zg: ZD,
        start_index: ANCHOR_START,
        end_index: 1_200,
    };
    assert_eq!(
        book.observe(&up_input(broken, 3, None, 500)).unwrap_err(),
        RetraceRejection::MalformedFrame { frame: broken }
    );

    let degenerate = CenterFrame {
        zd: ZD,
        zg: ZG,
        start_index: ANCHOR_START,
        end_index: ANCHOR_START,
    };
    assert_eq!(
        book.observe(&up_input(degenerate, 3, None, 500))
            .unwrap_err(),
        RetraceRejection::MalformedFrame { frame: degenerate }
    );
    settled(&book);
}

#[test]
fn missing_retest_position_rejected_at_registration() {
    let mut book = ledger();
    let center = frame(1_200);
    let mut input = up_input(center, 3, Some(RetraceOutcome::Success), 500);
    input.retest_end = None;
    assert_eq!(
        book.observe(&input).unwrap_err(),
        RetraceRejection::MissingRetestPosition { as_of: 500 },
        "给了结局就必须给判案位置"
    );
    settled(&book);
}

#[test]
fn leave_not_outside_center_rejected_at_registration() {
    let mut book = ledger();
    let center = frame(1_200);
    let mut input = up_input(center, 3, None, 500);
    input.leave_end = RetracePoint {
        index: 1_210,
        price: ZG,
    };
    assert_eq!(
        book.observe(&input).unwrap_err(),
        RetraceRejection::LeaveNotOutsideCenter {
            frame: center,
            leave_end: input.leave_end,
            leave_direction: Direction::Up,
        },
        "中枢边界闭区间 ⟹ 等值不算出中枢"
    );
    settled(&book);
}

#[test]
fn same_direction_retest_rejected_at_registration() {
    let mut book = ledger();
    let mut input = up_input(frame(1_200), 3, None, 500);
    input.retest_direction = Direction::Up;
    assert_eq!(
        book.observe(&input).unwrap_err(),
        RetraceRejection::RetestSameDirection {
            direction: Direction::Up
        }
    );
    settled(&book);
}

#[test]
fn non_adjacent_pair_rejected_at_registration() {
    let mut book = ledger();
    let mut input = up_input(frame(1_200), 3, None, 500);
    input.pair = StrictCompletedPair {
        leave_move_index: 3,
        retest_move_index: 5,
    };
    assert_eq!(
        book.observe(&input).unwrap_err(),
        RetraceRejection::NotAdjacent {
            leave_move_index: 3,
            retest_move_index: 5,
        },
        "补充十五：任一失败即 None，禁止后扫替代配对"
    );
    settled(&book);
}

/// 错误与终态分家：拒收既不建仓、也不产修订、更不写 `NotConstituted`。
#[test]
fn rejections_never_touch_the_ledger() {
    let mut book = ledger();
    let mut broken = up_input(frame(1_200), 3, None, 500);
    broken.retest_direction = Direction::Up;
    for _ in 0..4 {
        assert!(book.observe(&broken).is_err());
    }
    assert!(book.is_empty(), "残废件根本没资格进账本");
    assert!(book.journal().is_empty(), "拒收不进日志（不改状态）");
    assert_eq!(
        book.alarms().registration_rejected,
        4,
        "拒收另记 audit 计数"
    );
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 迁移表全盖（起态 × 输入 → 终态 + 本轮修订词汇）
// ═══════════════════════════════════════════════════════════════════════════

/// 起态。
#[derive(Debug, Clone, Copy)]
enum From {
    Absent,
    Provisional,
    Confirmed,
    NotConstituted,
}

/// 迁移表一行。
struct Row {
    from: From,
    outcome: Option<RetraceOutcome>,
    expect_state: RetraceState,
    expect_new_kinds: &'static [RetraceRevisionKind],
}

const TRANSITIONS: &[Row] = &[
    Row {
        from: From::Absent,
        outcome: None,
        expect_state: RetraceState::Provisional,
        expect_new_kinds: &[
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
        ],
    },
    Row {
        from: From::Absent,
        outcome: Some(RetraceOutcome::Success),
        expect_state: RetraceState::Confirmed,
        expect_new_kinds: &[
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
            RetraceRevisionKind::Confirmed,
        ],
    },
    Row {
        from: From::Absent,
        outcome: Some(RetraceOutcome::RetestReenters),
        expect_state: RetraceState::Invalidated,
        expect_new_kinds: &[
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
            RetraceRevisionKind::NotConstituted {
                reason: NotConstitutedReason::RetestReentered,
            },
        ],
    },
    Row {
        from: From::Provisional,
        outcome: None,
        expect_state: RetraceState::Provisional,
        expect_new_kinds: &[],
    },
    Row {
        from: From::Provisional,
        outcome: Some(RetraceOutcome::Success),
        expect_state: RetraceState::Confirmed,
        expect_new_kinds: &[RetraceRevisionKind::Confirmed],
    },
    Row {
        from: From::Provisional,
        outcome: Some(RetraceOutcome::RetestReenters),
        expect_state: RetraceState::Invalidated,
        expect_new_kinds: &[RetraceRevisionKind::NotConstituted {
            reason: NotConstitutedReason::RetestReentered,
        }],
    },
    Row {
        from: From::Confirmed,
        outcome: None,
        expect_state: RetraceState::Confirmed,
        expect_new_kinds: &[],
    },
    Row {
        from: From::Confirmed,
        outcome: Some(RetraceOutcome::Success),
        expect_state: RetraceState::Confirmed,
        expect_new_kinds: &[],
    },
    Row {
        from: From::Confirmed,
        outcome: Some(RetraceOutcome::RetestReenters),
        expect_state: RetraceState::Confirmed,
        expect_new_kinds: &[],
    },
    Row {
        from: From::NotConstituted,
        outcome: None,
        expect_state: RetraceState::Invalidated,
        expect_new_kinds: &[],
    },
    Row {
        from: From::NotConstituted,
        outcome: Some(RetraceOutcome::Success),
        expect_state: RetraceState::Invalidated,
        expect_new_kinds: &[],
    },
    Row {
        from: From::NotConstituted,
        outcome: Some(RetraceOutcome::RetestReenters),
        expect_state: RetraceState::Invalidated,
        expect_new_kinds: &[],
    },
];

fn seed(book: &mut RetraceLedger, center: CenterFrame, from: From) {
    let seeded = match from {
        From::Absent => return,
        From::Provisional => None,
        From::Confirmed => Some(RetraceOutcome::Success),
        From::NotConstituted => Some(RetraceOutcome::RetestReenters),
    };
    book.observe(&up_input(center, 3, seeded, 500)).unwrap();
}

#[test]
fn transition_table_is_total_over_state_by_outcome() {
    for row in TRANSITIONS {
        let mut book = ledger();
        let center = frame(1_200);
        seed(&mut book, center, row.from);
        let step = book
            .observe(&up_input(center, 3, row.outcome, 600))
            .unwrap();
        let produced: Vec<_> = step
            .delta
            .as_slice()
            .iter()
            .map(|revision| revision.kind)
            .collect();
        assert_eq!(
            step.state, row.expect_state,
            "起态 {:?} + 结局 {:?} 的终态",
            row.from, row.outcome
        );
        assert_eq!(
            produced, row.expect_new_kinds,
            "起态 {:?} + 结局 {:?} 的本轮修订",
            row.from, row.outcome
        );
        settled(&book);
    }
}
