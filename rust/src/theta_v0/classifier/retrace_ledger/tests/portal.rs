//! 成立档门户（AC③）：Confirmed 可枚举；证据包含知情时 + 位置 + 四条边快照；死亡证明载荷齐。
//!
//! 本文件只覆盖成立档（S1 交付）。备战档见 [`super::standby`]；短差档见 [`super::short_retrace`]
//! （票 #623，S3）。

use super::*;

#[test]
fn established_portal_enumerates_confirmed_with_full_evidence() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 860))
        .unwrap();

    let packs = book.established();
    assert_eq!(packs.len(), 1, "Confirmed 可枚举");
    let pack = packs[0];
    assert_eq!(pack.identity, key_of(center, 3));
    assert_eq!(pack.side, RetraceSide::Buy);
    assert_eq!(pack.frame, center, "四条边快照（转正）");
    assert_eq!(pack.leave_end.index, 1_210, "离开的边");
    assert_eq!(pack.retest_end.index, 1_220, "三类点位置");
    assert_eq!(pack.registered_as_of, 500, "出生知情时");
    assert_eq!(pack.confirmed_as_of, 860, "落锤知情时");
    assert_eq!(book.established_pack(&pack.identity), Some(pack));
    settled(&book);
}

#[test]
fn death_certificate_carries_center_identity_and_issue_time() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 860))
        .unwrap();

    let certificate = book.death_certificate(center.anchor()).unwrap();
    assert_eq!(certificate.center, center, "中枢身份 = 快照转正后的四条边");
    assert_eq!(certificate.issued_as_of, 860, "开具时间 = 落锤知情时");
    assert_ne!(
        certificate.issued_as_of, certificate.center.end_index,
        "开具时间与中枢右边缘是两个时间，不混（裁定一）"
    );
    assert_eq!(
        book.established()[0].death_certificate,
        certificate,
        "成功事件即死亡证明"
    );
    settled(&book);
}

#[test]
fn pending_and_failed_candidates_never_enter_the_established_portal() {
    let mut book = ledger();
    let pending = frame(1_200);
    book.observe(&up_input(pending, 3, None, 500)).unwrap();
    let failed = other_frame(300);
    book.observe(&down_input(
        failed,
        11,
        Some(RetraceOutcome::RetestReenters),
        520,
    ))
    .unwrap();

    assert!(book.established().is_empty(), "S1 成立档只放 Confirmed");
    assert_eq!(book.established_pack(&key_of(pending, 3)), None);
    assert_eq!(book.established_pack(&key_of(failed, 11)), None);
    assert_eq!(book.death_certificate(pending.anchor()), None);
    assert_eq!(
        book.death_certificate(failed.anchor()),
        None,
        "判败：中枢未破坏 ⟹ 无死亡证明"
    );
    settled(&book);
}

#[test]
fn established_portal_is_deterministic_across_centers() {
    let mut book = ledger();
    let a = frame(1_200);
    let b = other_frame(300);
    book.observe(&up_input(b, 11, Some(RetraceOutcome::Success), 520))
        .unwrap();
    book.observe(&up_input(a, 3, Some(RetraceOutcome::Success), 540))
        .unwrap();

    let identities: Vec<_> = book.established().iter().map(|pack| pack.identity).collect();
    let mut sorted = identities.clone();
    sorted.sort();
    assert_eq!(identities, sorted, "枚举按身份键序，无平局歧义");
    assert_eq!(identities.len(), 2);
    settled(&book);
}

/// 死亡证明**不问迟到**（054:60 延迟合法、#583 在案）：落锤知情时远晚于位置也照开。
#[test]
fn death_certificate_is_issued_regardless_of_lateness() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let late = 9_000;
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), late))
        .unwrap();

    let pack = book.established_pack(&key_of(center, 3)).unwrap();
    assert_eq!(pack.death_certificate.issued_as_of, late);
    assert!(
        pack.retest_end.index < late,
        "位置早、知情晚——迟到过滤归交易层自理（#587），账本不进口外部状态"
    );
    settled(&book);
}
