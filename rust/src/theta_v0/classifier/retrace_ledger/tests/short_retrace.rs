//! 短差档门户（票 #623 AC②③）：判败盘背观测记录，亚型签固定 + 三锁 + 失败处置通知。

use super::*;

#[test]
fn short_retrace_portal_captures_retest_reentered_with_fixed_subtype() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();

    let records = book.short_retrace_records();
    assert_eq!(records.len(), 1);
    let record = records[0];
    assert_eq!(record.identity, key_of(center, 3));
    assert_eq!(record.subtype, PanDivSubtype::RetestReentered, "亚型签固定");
    assert_eq!(record.side, RetraceSide::Buy);
    assert_eq!(
        record.frame, center,
        "判败：中枢未破坏，快照仍是原框（未转正）"
    );
    assert_eq!(record.leave_end.index, 1_210);
    assert_eq!(record.retest_end.index, 1_220, "判败位置=回抽笔终点");
    assert_eq!(record.registered_as_of, 500);
    assert_eq!(record.judged_as_of, 640, "落锤知情时");
    settled(&book);
}

#[test]
fn success_and_pending_never_enter_short_retrace_portal() {
    let mut book = ledger();
    let pending = frame(1_200);
    book.observe(&up_input(pending, 3, None, 500)).unwrap();
    let confirmed = other_frame(300);
    book.observe(&up_input(confirmed, 11, Some(RetraceOutcome::Success), 520))
        .unwrap();

    assert!(
        book.short_retrace_records().is_empty(),
        "S3 短差档只放 NotConstituted{{RetestReentered}}"
    );
    settled(&book);
}

/// 三锁之一：键唯一——同一身份重复登记冲突拒绝。
#[test]
fn admit_rejects_duplicate_identity() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();
    let record = book.short_retrace_records()[0];

    let mut portal = ShortRetracePortal::new();
    assert_eq!(portal.admit(record), Ok(()));
    assert_eq!(
        portal.admit(record),
        Err(ShortRetraceRejection::DuplicateIdentity(record.identity)),
        "键唯一：重复登记冲突拒绝"
    );
    assert_eq!(portal.len(), 1, "冲突拒绝不改变已登记条数");
    settled(&book);
}

/// 三锁之二：一一对应去重——`sync` 与账本判败集合逐条对应，重复 `sync` 不重复登记。
#[test]
fn sync_is_one_to_one_with_ledger_failures() {
    let mut book = ledger();
    let a = frame(1_200);
    book.observe(&up_input(a, 3, None, 500)).unwrap();
    book.observe(&up_input(a, 3, Some(RetraceOutcome::RetestReenters), 640))
        .unwrap();
    let b = other_frame(300);
    book.observe(&down_input(b, 11, None, 700)).unwrap();
    book.observe(&down_input(
        b,
        11,
        Some(RetraceOutcome::RetestReenters),
        760,
    ))
    .unwrap();

    let mut portal = ShortRetracePortal::new();
    portal.sync(&book);
    assert_eq!(portal.len(), 2, "账本两条判败，一一对应两条短差记录");
    portal.sync(&book);
    assert_eq!(portal.len(), 2, "重复 sync 不重复登记");
    settled(&book);
}

/// 三锁之三：账平断言——本档记录与账本判败一一对应（bijection：条数 + 身份 + 内容三重比对，进验收行）。
/// （2026-07-29 修复轮：由「条数相等」升级为内容级一一对应，影子评审 M1。）
#[test]
fn portal_balances_with_ledger_failure_count() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();

    let mut portal = ShortRetracePortal::new();
    assert!(
        !portal.balances_with(&book),
        "同步前：0 != 账本 1 条判败，账不平"
    );
    portal.sync(&book);
    assert!(portal.balances_with(&book), "同步后：账平断言成立");
    settled(&book);
}

/// 幂等消费：同一身份重复消费不重复处置。
#[test]
fn consume_is_idempotent_and_does_not_double_process() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();
    let mut portal = ShortRetracePortal::new();
    portal.sync(&book);
    let identity = key_of(center, 3);

    assert!(portal.consume(&identity).is_some(), "首次消费返回记录");
    assert_eq!(portal.consume(&identity), None, "重复消费幂等：不重复处置");
    settled(&book);
}

/// 失败处置通知：载荷 = 身份 + 位置 + 知情时；名义「通知非信号」（不代表已入场，不代表要处置）。
#[test]
fn failure_disposal_notice_carries_identity_position_and_knowledge_time() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();

    let notices = book.failure_disposal_notices();
    assert_eq!(notices.len(), 1);
    let notice = notices[0];
    assert_eq!(notice.identity, key_of(center, 3), "身份");
    assert_eq!(notice.position.index, 1_220, "位置=判败回抽笔终点");
    assert_eq!(notice.known_as_of, 640, "知情时=落锤知情时");
    settled(&book);
}

#[test]
fn disposal_notices_are_emitted_for_every_registered_short_retrace_record() {
    let mut book = ledger();
    let a = frame(1_200);
    book.observe(&up_input(a, 3, None, 500)).unwrap();
    book.observe(&up_input(a, 3, Some(RetraceOutcome::RetestReenters), 640))
        .unwrap();
    let b = other_frame(300);
    book.observe(&down_input(b, 11, None, 700)).unwrap();
    book.observe(&down_input(
        b,
        11,
        Some(RetraceOutcome::RetestReenters),
        760,
    ))
    .unwrap();

    let mut portal = ShortRetracePortal::new();
    portal.sync(&book);
    assert_eq!(
        portal.disposal_notices().len(),
        portal.len(),
        "登记多少条短差记录就发多少份通知"
    );
    assert_eq!(
        book.failure_disposal_notices().len(),
        2,
        "直接派生口径同样两份"
    );
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 影子评审 #623 S3 修复轮：HIGH-1 / MEDIUM-1 / MEDIUM-2 / MEDIUM-3 正式测试化
// （原探针 P3/P4/P5/P6/P7，见 chanlun/review-results/shadow-623-s3-review-20260729.md）
// ═══════════════════════════════════════════════════════════════════════════

/// HIGH-1 订正（P3）：`consume` 对尚未登记的身份返回 `None` 且不写入 `consumed`——此后 `sync`
/// 补登的真记录仍可正常消费。旧实现会先无条件插入 `consumed` 再查 `records`，把该身份永久
/// 毒化，`sync` 之后再也取不出来。
#[test]
fn consume_on_unregistered_identity_does_not_poison_future_sync() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();
    let identity = key_of(center, 3);

    let mut portal = ShortRetracePortal::new();
    assert_eq!(portal.consume(&identity), None, "尚未登记：显式拒绝");
    portal.sync(&book);
    assert_eq!(portal.len(), 1, "记录已登记");
    assert!(portal.balances_with(&book), "账平断言照样成立");
    assert!(
        portal.consume(&identity).is_some(),
        "未登记时的 consume 不得永久毒化该身份：sync 补登后仍可正常消费"
    );
    assert_eq!(
        portal.consume(&identity),
        None,
        "真正消费过一次后，幂等锁照常生效"
    );
    settled(&book);
}

/// MEDIUM-1 订正（P4）：账平断言按身份 + 内容双重比对，伪造身份混入即判不平（旧实现只比
/// `len()`，两边身份集合完全不相交时仍会判「平」）。
#[test]
fn balances_with_rejects_bogus_identity_absent_from_ledger() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();
    let real = book.short_retrace_records()[0];

    let bogus = ShortRetraceRecord {
        identity: key_of(other_frame(9_000), 99),
        ..real
    };
    let mut portal = ShortRetracePortal::new();
    portal.admit(bogus).unwrap();
    assert!(
        !portal.balances_with(&book),
        "门户里是伪造身份、账本里是另一条真判败，条数相等但身份集合不相交——账不平"
    );
    settled(&book);
}

/// MEDIUM-1 订正（P5）：`sync` 以账本为真相——已登记身份若内容偏离账本（如被篡改的
/// `retest_end`），重新 `sync` 后订正为账本值，不再静默保留旧值；订正后下发的处置通知
/// 不再带错位置。
#[test]
fn sync_corrects_identity_whose_recorded_content_deviates_from_ledger() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();
    let real = book.short_retrace_records()[0];

    let tampered = ShortRetraceRecord {
        retest_end: RetracePoint {
            index: 999_999,
            price: -1,
        },
        ..real
    };
    let mut portal = ShortRetracePortal::new();
    portal.admit(tampered).unwrap();
    assert!(!portal.balances_with(&book), "同步前：内容偏离账本，账不平");

    portal.sync(&book);
    assert!(portal.balances_with(&book), "同步后：订正为账本值，账平");
    let notices = portal.disposal_notices();
    assert_eq!(notices.len(), 1);
    assert_eq!(
        notices[0].position, real.retest_end,
        "通知位置已订正，不再带错位置"
    );
    settled(&book);
}

/// MEDIUM-2 订正（P6）：判败缺回抽位置的重放态在 `assert_invariants()` 处 fail-loud，不再留到
/// 只读派生函数 `short_retrace_records()` 才 panic——与判胜侧既有的镜像不变量对称。
#[test]
#[should_panic(expected = "判败必带回抽位置")]
fn assert_invariants_catches_tampered_failure_without_retest_position() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();

    let mut journal = book.journal().to_vec();
    let target = journal
        .iter_mut()
        .find(|record| {
            matches!(
                record.revision.kind,
                RetraceRevisionKind::NotConstituted {
                    reason: NotConstitutedReason::RetestReentered
                }
            )
        })
        .expect("判败落锤记录必在日志中");
    target.revision.evidence.as_mut().unwrap().retest_end = None;

    let tampered = RetraceLedger::fold(provenance(), &journal).unwrap();
    tampered.assert_invariants(); // ← 应在此 fail-loud，而不是等到 short_retrace_records() 才 panic
}

/// MEDIUM-3 订正（P7）：`disposal_notices` 与 `consume` 共享同一 `consumed` 判重——记录被消费后
/// 不再重复吐出通知。旧实现两路完全不相交：`consume` 幂等成立之后，`disposal_notices` 仍会
/// 无条件重复发放同一条通知。
#[test]
fn disposal_notices_stop_after_the_record_is_consumed() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        640,
    ))
    .unwrap();
    let identity = key_of(center, 3);

    let mut portal = ShortRetracePortal::new();
    portal.sync(&book);
    assert_eq!(portal.disposal_notices().len(), 1, "消费前：通知照发");

    assert!(portal.consume(&identity).is_some(), "首次消费成功");
    assert_eq!(portal.consume(&identity), None, "幂等：重复消费不重复处置");
    assert!(
        portal.disposal_notices().is_empty(),
        "已消费的记录不再重复吐出通知——通知路与消费锁共享同一判重"
    );
    assert_eq!(portal.len(), 1, "消费不删除记录，只标记：记录仍在档");
    settled(&book);
}
