//! 死人挂号拒收（票 #622；#574 裁定四）：Success 后同一中枢新 departure fail-loud 拒收，判据
//! 统一为 `death_certificate(anchor)` 查法（修复影子评审 #621 MEDIUM-4：S1 旧查法「前一注册是否
//! Confirmed」在隔代序列上漏计）。与同身份迟到静默吸收（裁定三，幂等问题）严格区分——两路不混，
//! 各有测试。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 死人挂号拒收：新身份挂号
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn new_identity_at_dead_center_is_rejected_with_certificate() {
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
        }
    );
    assert!(book.entry(&attempted).is_none(), "拒收零建仓");
    assert_eq!(book.len(), 1, "只有原始 Confirmed 身份在账");
    assert_eq!(book.alarms().dead_center_registrations, 1);
    settled(&book);
}

/// 精确复现影子评审 #621 探针 2 的隔代序列：Confirmed → 同锚新 departure。
///
/// S1 旧查法「前一注册是否 Confirmed」在「Confirmed → 判败 → 再新注册」的隔代序列上会漏计
/// （前一注册是判败的那代，不是 Confirmed 的那代）。S2 统一为 `death_certificate(anchor)` 查法
/// 后，**这个隔代序列本身在活路上已不可构造**——死亡证明存在时，同锚的第二代新 departure 在
/// 注册期就被拦下，压根没有机会先注册成功再走到判败，第三代更无从谈起。本测试直接证明这一点：
/// 死后任意新 departure，无论第几次尝试，一律同样的拒收，账本状态一个 bit 不动。
#[test]
fn dead_center_rejection_is_uniform_across_repeated_attempts_no_intervening_generation_possible() {
    let mut book = ledger();
    let f1 = frame(1_200);
    book.observe(&up_input(f1, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let certificate = book.death_certificate(f1.anchor()).unwrap();

    for (departure, as_of) in [(5usize, 600usize), (7, 700)] {
        let attempted = key_of(f1, departure);
        let rejection = book
            .observe(&up_input(f1, departure, None, as_of))
            .unwrap_err();
        assert_eq!(
            rejection,
            RetraceRejection::DeadCenterReentry {
                anchor: f1.anchor(),
                attempted,
                death_certificate: certificate,
            },
            "每次尝试都拿到同一张死亡证明，从未有机会先注册成功"
        );
    }
    assert_eq!(
        book.len(),
        1,
        "账本自始至终只有原始 Confirmed 一条——无隔代可言"
    );
    assert_eq!(book.alarms().dead_center_registrations, 2);
    settled(&book);
}

/// 拒收多次 ⟹ 计数与拒收次数恒等（MEDIUM-4 修复的核心断言：不再可能出现「计数 ≠ 拒收数」）。
#[test]
fn dead_center_counter_equals_actual_rejection_count_across_multiple_attempts() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();

    for (departure, as_of) in [(5usize, 600usize), (7, 700), (9, 800)] {
        assert!(book
            .observe(&up_input(frame(1_300 + departure), departure, None, as_of))
            .is_err());
    }
    assert_eq!(
        book.alarms().dead_center_registrations,
        3,
        "三次拒收，计数恰好三"
    );
    assert_eq!(book.len(), 1);
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 与同身份迟到静默吸收严格区分（两路不混）
// ═══════════════════════════════════════════════════════════════════════════

/// 同身份（同 key）迟到输入走终态吸收——**不是**死人挂号，即使该锚已死。
#[test]
fn same_identity_late_input_after_confirmed_is_silently_absorbed_not_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();

    let late = book
        .observe(&up_input(
            center,
            3,
            Some(RetraceOutcome::RetestReenters),
            600,
        ))
        .unwrap();
    assert!(late.absorbed_late(), "同身份迟到 ⟹ 静默吸收");
    assert_eq!(late.state, RetraceState::Confirmed, "禁复活");
    assert_eq!(book.alarms().late_absorbed, 1);
    assert_eq!(
        book.alarms().dead_center_registrations,
        0,
        "同身份迟到不是死人挂号——两路不混"
    );
    settled(&book);
}

/// 死人挂号（新身份，新 departure）与迟到吸收（同身份）在同一测试里对照，锁死两路分野。
#[test]
fn dead_center_reentry_and_late_absorption_are_mutually_exclusive_paths() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();

    // 路一：同身份迟到 ⟹ 吸收。
    let absorbed = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap();
    assert!(absorbed.absorbed_late());

    // 路二：新身份挂号 ⟹ 拒收。
    let rejected = book.observe(&up_input(frame(1_400), 5, None, 700));
    assert!(rejected.is_err());

    assert_eq!(book.alarms().late_absorbed, 1);
    assert_eq!(book.alarms().dead_center_registrations, 1);
    settled(&book);
}
