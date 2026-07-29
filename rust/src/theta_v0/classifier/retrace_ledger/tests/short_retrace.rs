//! 短差档门户（票 #623 AC②③）：判败盘背观测记录，亚型签固定 + 三锁 + 失败处置通知。

use super::*;

#[test]
fn short_retrace_portal_captures_retest_reentered_with_fixed_subtype() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(center, 3, Some(RetraceOutcome::RetestReenters), 640))
        .unwrap();

    let records = book.short_retrace_records();
    assert_eq!(records.len(), 1);
    let record = records[0];
    assert_eq!(record.identity, key_of(center, 3));
    assert_eq!(record.subtype, PanDivSubtype::RetestReentered, "亚型签固定");
    assert_eq!(record.side, RetraceSide::Buy);
    assert_eq!(record.frame, center, "判败：中枢未破坏，快照仍是原框（未转正）");
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
    book.observe(&up_input(center, 3, Some(RetraceOutcome::RetestReenters), 640))
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
    book.observe(&down_input(b, 11, Some(RetraceOutcome::RetestReenters), 760))
        .unwrap();

    let mut portal = ShortRetracePortal::new();
    portal.sync(&book);
    assert_eq!(portal.len(), 2, "账本两条判败，一一对应两条短差记录");
    portal.sync(&book);
    assert_eq!(portal.len(), 2, "重复 sync 不重复登记");
    settled(&book);
}

/// 三锁之三：账平断言——本档条数须等于账本判败条数（进验收行）。
#[test]
fn portal_balances_with_ledger_failure_count() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(center, 3, Some(RetraceOutcome::RetestReenters), 640))
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
    book.observe(&up_input(center, 3, Some(RetraceOutcome::RetestReenters), 640))
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
    book.observe(&up_input(center, 3, Some(RetraceOutcome::RetestReenters), 640))
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
    book.observe(&down_input(b, 11, Some(RetraceOutcome::RetestReenters), 760))
        .unwrap();

    let mut portal = ShortRetracePortal::new();
    portal.sync(&book);
    assert_eq!(
        portal.disposal_notices().len(),
        portal.len(),
        "登记多少条短差记录就发多少份通知"
    );
    assert_eq!(book.failure_disposal_notices().len(), 2, "直接派生口径同样两份");
    settled(&book);
}
