//! 三钟纪律（AC②后半：首写不改 / 倒退拒收 + 警报 / 每条修订带知情时 / 知情时 ≠ 位置）。

use super::*;

#[test]
fn birth_clock_is_written_once_and_never_moves() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    for as_of in [500, 501, 900] {
        book.observe(&up_input(center, 3, None, as_of)).unwrap();
    }
    let entry = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(entry.registered_as_of, 500, "出生钟首写永不改（裁定五）");
    assert_eq!(entry.last_as_of, 900, "门卫钟随推进前移");
    settled(&book);
}

#[test]
fn terminal_clock_is_written_once_and_never_moves() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 640))
        .unwrap();
    for as_of in [641, 700, 5_000] {
        book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), as_of))
            .unwrap();
    }
    let entry = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(entry.terminal_as_of, Some(640), "落锤钟首写永不改（裁定五）");
    assert_eq!(entry.last_as_of, 5_000, "门卫钟照常前移（吸收也过门）");
    settled(&book);
}

#[test]
fn gate_clock_rejects_retrograde_with_zero_mutation_and_alarm() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let before = book.entry(&key_of(center, 3)).unwrap().clone();

    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 400))
        .unwrap();
    assert!(step.retrograde_rejected(), "per-identity 知情时非降（裁定五）");
    assert!(step.delta.is_empty());

    let after = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(after, &before, "倒退喂入零改写");
    assert_eq!(book.alarms().retrograde_rejected, 1, "倒退拒收进警报计数");
    settled(&book);
}

#[test]
fn equal_knowledge_time_is_not_retrograde() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    assert!(!step.retrograde_rejected(), "非降 ⟹ 相等准入");
    assert_eq!(step.state, RetraceState::Confirmed);
    assert_eq!(book.alarms().retrograde_rejected, 0);
    settled(&book);
}

#[test]
fn every_revision_carries_its_knowledge_time() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(center, 3, Some(RetraceOutcome::RetestReenters), 800))
        .unwrap();
    book.observe(&up_input(frame(1_400), 5, None, 900)).unwrap();

    let stamps: Vec<_> = book
        .journal()
        .iter()
        .map(|record| (record.revision.kind, record.revision.as_of))
        .collect();
    assert_eq!(
        stamps,
        vec![
            (RetraceRevisionKind::Registered, 500),
            (RetraceRevisionKind::SnapshotPinned, 500),
            (
                RetraceRevisionKind::NotConstituted {
                    reason: NotConstitutedReason::RetestReentered
                },
                800
            ),
            (RetraceRevisionKind::Registered, 900),
            (RetraceRevisionKind::SnapshotPinned, 900),
            (
                RetraceRevisionKind::Restarted {
                    previous: key_of(center, 3)
                },
                900
            ),
        ]
    );
    settled(&book);
}

/// 裁定五：位置进证据载荷，**不是钟**——回抽周二到最低点、引擎周四才确认走完。
#[test]
fn position_lives_in_evidence_not_in_clocks() {
    let mut book = ledger();
    let center = frame(1_200);
    let input = up_input(center, 3, Some(RetraceOutcome::Success), 500);
    let step = book.observe(&input).unwrap();

    let entry = book.entry(&step.key).unwrap();
    let evidence = entry.terminal_evidence().unwrap();
    assert_eq!(evidence.retest_end.unwrap().index, 1_220, "位置 = 回抽笔终点源坐标");
    assert_eq!(evidence.leave_end.index, 1_210, "位置 = 离开笔终点源坐标");
    assert_eq!(entry.terminal_as_of, Some(500), "知情时是另一个数");
    assert_ne!(
        evidence.retest_end.unwrap().index,
        entry.terminal_as_of.unwrap(),
        "知情时 ≠ 位置：回测不用未来信息靠知情时保证"
    );
    settled(&book);
}

/// 未决时回抽位置**诚实缺席**（材料缺就是缺，不编造）。
#[test]
fn pending_registration_reports_absent_retest_position() {
    let mut book = ledger();
    let center = frame(1_200);
    let step = book.observe(&up_input(center, 3, None, 500)).unwrap();
    let registration = book.entry(&step.key).unwrap().registration().unwrap();
    assert_eq!(registration.retest_end, None);
    assert_eq!(registration.leave_end.price, ZG + 50);
    settled(&book);
}
