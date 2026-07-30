//! 两拍证据一致性守卫（票 #622；影子评审 #621 MEDIUM-2）：同一身份注册拍与落锤拍的侧 / 离开边
//! 不一致 ⟹ 报错拒收（裁定二 fail-loud 同族）。真实引擎两拍同值是既成事实（leave 笔已走完才有
//! departure）；本守卫只挡 provider 两拍改口这一异常通道。

use super::*;

/// 侧翻转（Buy → Sell）在落锤拍 ⟹ 拒收，零改写（账本状态、留档、门卫钟一个 bit 不动）。
///
/// 用 `down_input` 构造落锤拍——这是一条**独立合法**（自身能过残废四桶）但与注册拍矛盾的观察，
/// 而不是从 `up_input` 直接改方向字段（那样会先撞上「leave 未出中枢」，永远到不了两拍守卫）。
#[test]
fn side_flip_between_registration_and_settlement_is_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let before = book.entry(&key_of(center, 3)).unwrap().clone();

    let flipped = down_input(center, 3, Some(RetraceOutcome::Success), 600);

    let rejection = book.observe(&flipped).unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::TerminalEvidenceContradictsRegistration {
            key: key_of(center, 3),
            registered_side: RetraceSide::Buy,
            registered_leave_end: RetracePoint {
                index: center.end_index + 10,
                price: ZG + 50,
            },
            incoming_side: RetraceSide::Sell,
            incoming_leave_end: flipped.leave_end,
        }
    );

    let after = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(after, &before, "拒收零改写：条目一个 bit 不动");
    assert_eq!(after.state, RetraceState::Provisional, "未被误判成立/判败");
    assert_eq!(book.alarms().registration_rejected, 1);
    settled(&book);
}

/// 同侧但离开边坐标矛盾同样拒收——不仅比对 side，也比对 leave_end。
#[test]
fn leave_end_mismatch_between_registration_and_settlement_is_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let mut contradicted = up_input(center, 3, Some(RetraceOutcome::Success), 600);
    contradicted.leave_end = RetracePoint {
        index: center.end_index + 30,
        price: ZG + 80,
    };

    let rejection = book.observe(&contradicted).unwrap_err();
    match rejection {
        RetraceRejection::TerminalEvidenceContradictsRegistration {
            registered_leave_end,
            incoming_leave_end,
            ..
        } => {
            assert_ne!(registered_leave_end, incoming_leave_end);
        }
        other => panic!("应为两拍证据矛盾拒收：{other:?}"),
    }
    settled(&book);
}

/// 两拍同值（真实引擎既成事实）⟹ 照常判决，守卫零误伤。
#[test]
fn consistent_two_pass_evidence_settles_normally() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap();
    assert_eq!(step.state, RetraceState::Confirmed);
    assert_eq!(book.alarms().registration_rejected, 0, "同值不触发守卫");
    settled(&book);
}

/// 首次观察即带结局（register_new 路径）：注册快照与判案证据同源于同一次输入，天然一致，
/// 守卫不适用于本路径（守卫只检查 `advance_existing`）。
#[test]
fn first_observation_with_outcome_bypasses_the_guard_by_construction() {
    let mut book = ledger();
    let center = frame(1_200);
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    assert_eq!(step.state, RetraceState::Confirmed);
    assert_eq!(book.alarms().registration_rejected, 0);
    settled(&book);
}

/// 判败路径（RetestReenters）同样受两拍守卫覆盖，不只是 Success 路径。
#[test]
fn side_flip_on_failure_outcome_is_also_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let flipped = down_input(center, 3, Some(RetraceOutcome::RetestReenters), 600);

    let rejection = book.observe(&flipped).unwrap_err();
    assert!(matches!(
        rejection,
        RetraceRejection::TerminalEvidenceContradictsRegistration { .. }
    ));
    assert_eq!(
        book.entry(&key_of(center, 3)).unwrap().state,
        RetraceState::Provisional,
        "拒收不落判败"
    );
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 守卫有效域边界：终态后（票 #622 S2 修复轮 MEDIUM-1；影子评审探针 p8）
// ═══════════════════════════════════════════════════════════════════════════

/// **编排者字面裁定三优先**：终态后同身份迟到输入——无论证据是否与注册拍矛盾——一律走裁定三
/// 「静默吸收 + 警报」，两拍守卫不越过终态边界。本测试是「矛盾证据」这半边（一致证据的迟到吸收
/// 已由 [`super::dead_center::same_identity_late_input_after_confirmed_is_silently_absorbed_not_rejected`]
/// 覆盖）——此前守卫置于 `admit` 之前、无条件施加，会把这条本该静默的迟到输入误判成 `Err`。
#[test]
fn contradictory_late_evidence_after_terminal_is_silently_absorbed_not_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();

    // 迟到输入的侧翻转（Buy → Sell）：若守卫仍无条件施加，这里会拿到
    // `TerminalEvidenceContradictsRegistration` 的 `Err`；终态边界收缩后须静默吸收。
    let flipped = down_input(center, 3, Some(RetraceOutcome::Success), 600);
    let step = book
        .observe(&flipped)
        .expect("终态后矛盾证据须静默吸收，不是 Err");

    assert!(
        step.absorbed_late(),
        "终态后迟到 ⟹ 静默吸收（裁定三），不因证据矛盾变成拒收"
    );
    assert_eq!(step.state, RetraceState::Confirmed, "禁复活，原判维持");
    assert_eq!(book.alarms().late_absorbed, 1, "计入迟到吸收警报");
    assert_eq!(
        book.alarms().registration_rejected,
        0,
        "不是拒收——两拍守卫在终态边界外不生效"
    );

    let entry = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(
        entry.terminal_evidence().unwrap().side,
        RetraceSide::Buy,
        "落锤证据不被迟到矛盾污染"
    );
    settled(&book);
}
