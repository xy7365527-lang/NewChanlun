//! 恢复后未决身份落锤显式护栏（票 #632；编排者 2026-07-29 裁方案②；影子评审 #621 MEDIUM-1
//! 修复）：[`RetraceLedger::fold_recovered`] 声明恢复点后，早于该点的知情时不得把仍处
//! Provisional 的身份落锤——门卫钟本身的退回语义（S1 现状）不动，本护栏是叠加的一层。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 影子探针 6 场景复现：恢复后陈旧知情时可落锤 ⟹ 加护栏后须拒收
// ═══════════════════════════════════════════════════════════════════════════

/// 未声明恢复点（普通 `fold`）：复现 #621 MEDIUM-1 的历史行为——陈旧知情时仍能落锤（订正
/// `log.rs` 声明所描述的现状，钉死「未升级调用方零约束」这条兼容承诺）。
#[test]
fn plain_fold_without_recovery_point_still_lets_stale_knowledge_settle() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap(); // 注册（进日志）
    live.observe(&up_input(center, 3, None, 4_000)).unwrap(); // 未决观察（不进日志，推门卫钟）
    let key = key_of(center, 3);
    assert_eq!(live.entry(&key).unwrap().last_as_of, 4_000);

    let mut restored = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    assert_eq!(restored.recovery_floor(), None, "普通 fold 不设恢复点");
    assert_eq!(
        restored.entry(&key).unwrap().last_as_of,
        500,
        "门卫钟退回留档下界（S1 不动）"
    );

    let step = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap();
    assert!(!step.retrograde_rejected());
    assert_eq!(
        step.state,
        RetraceState::Confirmed,
        "未设恢复点 ⟹ 零约束，历史行为不变"
    );
    settled(&restored);
}

/// 核心探针：声明恢复点（=重启前最后已知知情时 4000）后，同一条陈旧输入（600）改为 fail-loud
/// 拒收 + 警报，不落锤——恢复后未决身份不再能被陈旧知情时改判。
#[test]
fn recovered_settle_with_as_of_behind_recovery_point_is_rejected_fail_loud() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap(); // 注册（进日志）
    live.observe(&up_input(center, 3, None, 4_000)).unwrap(); // 未决观察（不进日志，推门卫钟）
    let key = key_of(center, 3);

    // 崩溃/重启：调用方独立维护的续跑断点声明恢复点 = 4_000（重启前已知到这里）。
    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 4_000).unwrap();
    assert_eq!(restored.recovery_floor(), Some(4_000));
    assert_eq!(
        restored.entry(&key).unwrap().last_as_of,
        500,
        "门卫钟仍退回留档下界，S1 语义不动"
    );
    let before = restored.entry(&key).unwrap().clone();

    let rejection = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::SettleBehindRecoveryPoint {
            key,
            recovery_as_of: 4_000,
            as_of: 600,
        },
        "陈旧知情时（600 < 恢复点 4000）fail-loud 拒收，不得落锤"
    );

    let entry = restored.entry(&key).unwrap();
    assert_eq!(entry, &before, "拒收零改写：身份一个 bit 不动");
    assert_eq!(entry.state, RetraceState::Provisional, "未决身份不落锤");
    assert!(entry.terminal_as_of.is_none(), "落锤钟未被写坏");
    assert_eq!(
        restored.alarms().registration_rejected,
        1,
        "护栏拒收计入注册期拒收桶"
    );
    let record = restored
        .audit_log()
        .last()
        .copied()
        .expect("护栏拒收落 audit 流");
    assert_eq!(
        record.event,
        RetraceAuditEvent::ResidualRejected {
            as_of: 600,
            code: RetraceRejectionCode::SettleBehindRecoveryPoint,
        }
    );
    settled(&restored);
}

/// 拒收之后，恢复点之后的正常知情时仍可正常落锤——护栏只挡陈旧区间，不冻结身份。
#[test]
fn identity_can_still_settle_normally_once_as_of_catches_up_to_recovery_point() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap();
    live.observe(&up_input(center, 3, None, 4_000)).unwrap();
    let key = key_of(center, 3);

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 4_000).unwrap();
    assert!(restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .is_err());

    let step = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 4_500))
        .unwrap();
    assert!(!step.retrograde_rejected());
    assert_eq!(
        step.state,
        RetraceState::Confirmed,
        "恢复点之后的知情时正常落锤"
    );
    assert_eq!(restored.entry(&key).unwrap().terminal_as_of, Some(4_500));
    settled(&restored);
}

// ═══════════════════════════════════════════════════════════════════════════
// 边界：恰等于恢复点放行（非严格不等式）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn settle_exactly_at_recovery_point_is_admitted() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap();
    let key = key_of(center, 3);

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 600).unwrap();
    let step = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap();
    assert!(!step.retrograde_rejected());
    assert_eq!(
        step.state,
        RetraceState::Confirmed,
        "恰等于恢复点 ⟹ 非降语义放行（同门卫钟纪律）"
    );
    assert_eq!(restored.entry(&key).unwrap().terminal_as_of, Some(600));
    assert_eq!(restored.alarms().registration_rejected, 0);
    settled(&restored);
}

/// 边界的镜像：恢复点前一格（`recovery_as_of - 1`）仍须拒收——护栏不是"约等于"。
#[test]
fn settle_one_below_recovery_point_is_still_rejected() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap();
    let key = key_of(center, 3);

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 600).unwrap();
    let rejection = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 599))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::SettleBehindRecoveryPoint {
            key,
            recovery_as_of: 600,
            as_of: 599,
        }
    );
    settled(&restored);
}

// ═══════════════════════════════════════════════════════════════════════════
// 终态身份不受影响：护栏只挡「即将落锤的未决身份」，已终态身份走既有迟到吸收路径
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn already_terminal_identity_is_unaffected_by_recovery_point_late_absorption_still_applies() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let key = key_of(center, 3);
    assert_eq!(live.entry(&key).unwrap().state, RetraceState::Confirmed);

    // 恢复点设得比落锤钟还高——若护栏误伤终态身份，这里会被当成"未决落锤"拒收；
    // 正确行为是终态吸收路径完全不查恢复点，走既有的静默吸收 + 警报。
    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 9_000).unwrap();
    let step = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 501))
        .unwrap();
    assert!(
        step.absorbed_late(),
        "终态后迟到输入仍走既有静默吸收，不经恢复点护栏"
    );
    assert_eq!(
        restored.entry(&key).unwrap().terminal_as_of,
        Some(500),
        "落锤钟不动"
    );
    assert_eq!(restored.alarms().late_absorbed, 1);
    assert_eq!(
        restored.alarms().registration_rejected,
        0,
        "终态身份的迟到输入不应被恢复点护栏计入拒收——护栏只管未决身份的落锤"
    );
    settled(&restored);
}

// ═══════════════════════════════════════════════════════════════════════════
// 活账（无重启）行为不动：`new()` 与未升级的 `fold()` 调用方零约束
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn live_ledger_has_no_recovery_point_and_settles_unconstrained() {
    let mut book = ledger();
    assert_eq!(book.recovery_floor(), None);
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 501))
        .unwrap();
    assert_eq!(step.state, RetraceState::Confirmed);
    assert_eq!(book.alarms().registration_rejected, 0);
    settled(&book);
}

#[test]
fn plain_fold_leaves_recovery_floor_unset() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap();
    let folded = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    assert_eq!(
        folded.recovery_floor(),
        None,
        "fold_recovered 是纯增设，fold 不受影响"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// register_new 通道：首次观察即带结局，同样受恢复点护栏覆盖
// ═══════════════════════════════════════════════════════════════════════════

/// 恢复点声明后，一个从未在日志中出现过的**全新身份**若首次观察就带陈旧结局（早于恢复点），
/// 同样 fail-loud 拒收——护栏覆盖两个入口（`advance_existing` 与 `register_new`），不只挡
/// 「被恢复的旧身份」。
#[test]
fn brand_new_identity_settling_below_recovery_point_on_first_observation_is_rejected() {
    let mut live = ledger();
    // journal 里放一条无关记录，只为让 fold_recovered 有内容可折（身份本身与新候选无关）。
    live.observe(&up_input(other_frame(300), 11, None, 100))
        .unwrap();

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 4_000).unwrap();
    let center = frame(1_200);
    let key = key_of(center, 3);
    let rejection = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::SettleBehindRecoveryPoint {
            key,
            recovery_as_of: 4_000,
            as_of: 600,
        }
    );
    assert!(
        restored.entry(&key).is_none(),
        "护栏在建仓前拦下，新身份未建仓（零改写）"
    );
    assert_eq!(restored.len(), 1, "只有 journal 里那条无关身份");
    settled(&restored);
}

/// `register_new` 通道的镜像正例：首次观察带的结局知情时不早于恢复点 ⟹ 正常建仓并落锤。
#[test]
fn brand_new_identity_settling_at_or_after_recovery_point_on_first_observation_succeeds() {
    let mut live = ledger();
    live.observe(&up_input(other_frame(300), 11, None, 100))
        .unwrap();

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 4_000).unwrap();
    let center = frame(1_200);
    let step = restored
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 4_000))
        .unwrap();
    assert_eq!(step.state, RetraceState::Confirmed);
    settled(&restored);
}
