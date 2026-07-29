//! 引擎改口处死（票 #622；#574 裁定一）：窗口变 / 中枢消失 → `NotConstituted{CenterRebased}`
//! + 警报计数 + 证据新旧窗口对照；同窗口重确认零动作。两条通道：`observe` 碰撞路径（同一
//! departure、不同窗口）+ `reconcile_window` 显式核对路径（专供「无新 departure 也要核对」与
//! 「中枢消失」——后者没有天然的 `observe` 输入可以携带）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// `observe` 碰撞路径：同一 departure、不同窗口 ⟹ 改口
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn same_departure_different_window_kills_old_as_rebased_and_registers_new() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap();

    let rebased = frame(1_400);
    let step = book.observe(&up_input(rebased, 3, None, 700)).unwrap();

    assert_eq!(step.key, key_of(rebased, 3), "本轮焦点是新注册的身份");
    assert_eq!(step.state, RetraceState::Provisional);

    let old_entry = book.entry(&key_of(old, 3)).unwrap();
    assert_eq!(old_entry.state, RetraceState::Invalidated, "旧候选处死");
    match old_entry.not_constituted_reason().unwrap() {
        NotConstitutedReason::CenterRebased {
            registered_window,
            observed_window,
        } => {
            assert_eq!(registered_window, old, "旧窗口 = 注册拍快照");
            assert_eq!(observed_window, Some(rebased), "新窗口 = 引擎当拍报的窗口");
        }
        other => panic!("判败原因码应为 CenterRebased：{other:?}"),
    }
    assert_eq!(old_entry.terminal_as_of, Some(700), "落锤钟 = 处死知情时");

    let new_entry = book.entry(&key_of(rebased, 3)).unwrap();
    assert_eq!(
        new_entry.restarted_from(),
        Some(key_of(old, 3)),
        "谱系载荷记前任（改口与判败重回共用 Restart 谱系机制，anchor 级）"
    );
    assert_eq!(book.alarms().center_rebased, 1, "改口进警报计数");
    assert_eq!(
        book.alarms().registration_rejected,
        0,
        "改口不是拒收——不进 registration_rejected 桶"
    );
    assert_eq!(book.len(), 2, "新旧两条身份全部留档");
    settled(&book);
}

/// 改口触发观察若同时携带结局，新档照常判决（改口只处死旧档，不影响新档判决路径）。
#[test]
fn rebase_collision_observation_still_judges_the_new_identity() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap();

    let rebased = frame(1_400);
    let step = book
        .observe(&up_input(rebased, 3, Some(RetraceOutcome::Success), 700))
        .unwrap();

    assert_eq!(step.state, RetraceState::Confirmed, "新档带结局照常判决");
    assert!(book.established_pack(&key_of(rebased, 3)).is_some());
    assert_eq!(
        book.entry(&key_of(old, 3)).unwrap().not_constituted_reason(),
        Some(NotConstitutedReason::CenterRebased {
            registered_window: old,
            observed_window: Some(rebased),
        })
    );
    settled(&book);
}

/// 中枢改口发生在**判败之后**的重回场景不受影响：先判败（RetestReentered），再改口注册新档，
/// 前任判败原因码不因后续改口而被覆盖（append-only：前任的判败修订早已固化）。
#[test]
fn rebase_after_retest_reentered_predecessor_leaves_its_own_reason_intact() {
    let mut book = ledger();
    let first = frame(1_200);
    book.observe(&up_input(first, 3, Some(RetraceOutcome::RetestReenters), 500))
        .unwrap();

    let second = frame(1_400);
    book.observe(&up_input(second, 5, None, 700)).unwrap();
    let rebased = frame(1_600);
    book.observe(&up_input(rebased, 5, None, 900)).unwrap();

    assert_eq!(
        book.entry(&key_of(first, 3)).unwrap().not_constituted_reason(),
        Some(NotConstitutedReason::RetestReentered),
        "首任判败原因码不被后续改口覆盖"
    );
    assert_eq!(
        book.entry(&key_of(second, 5)).unwrap().not_constituted_reason(),
        Some(NotConstitutedReason::CenterRebased {
            registered_window: second,
            observed_window: Some(rebased),
        })
    );
    assert_eq!(
        book.entry(&key_of(rebased, 5)).unwrap().restarted_from(),
        Some(key_of(second, 5)),
        "谱系只记直接前任"
    );
    assert_eq!(book.alarms().center_rebased, 1);
    settled(&book);
}

/// 不同 departure（真新离开）而旧候选未判完 ⟹ 仍是裁定二 fail-loud，**不是**改口——回归防护：
/// 改口分支只在 departure 相同时触发，不得吞掉真正的 provider 有病场景。
#[test]
fn different_departure_while_unsettled_is_still_fail_loud_not_rebase() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let rejection = book.observe(&up_input(frame(1_400), 7, None, 700)).unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::ActiveCandidateNotSettled {
            active: key_of(center, 3),
            incoming_departure_move_index: 7,
        }
    );
    assert_eq!(book.alarms().center_rebased, 0, "不是改口");
    assert_eq!(book.alarms().registration_rejected, 1);
    assert_eq!(
        book.entry(&key_of(center, 3)).unwrap().state,
        RetraceState::Provisional,
        "旧候选一个 bit 不动"
    );
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// `reconcile_window` 显式核对路径（中枢消失 / 无新 departure 时的窗口核对）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reconcile_window_kills_as_rebased_when_window_changes() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap();

    let new_window = frame(1_500);
    let step = book
        .reconcile_window(old.anchor(), Some(new_window), 800)
        .unwrap();
    assert_eq!(step.key, key_of(old, 3));
    assert_eq!(step.state, RetraceState::Invalidated);

    let entry = book.entry(&key_of(old, 3)).unwrap();
    assert_eq!(
        entry.not_constituted_reason(),
        Some(NotConstitutedReason::CenterRebased {
            registered_window: old,
            observed_window: Some(new_window),
        })
    );
    assert_eq!(entry.terminal_as_of, Some(800));
    assert_eq!(book.alarms().center_rebased, 1);
    settled(&book);
}

#[test]
fn reconcile_window_kills_as_rebased_when_center_vanishes() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let step = book.reconcile_window(center.anchor(), None, 900).unwrap();
    assert_eq!(step.state, RetraceState::Invalidated);
    assert_eq!(
        book.entry(&key_of(center, 3)).unwrap().not_constituted_reason(),
        Some(NotConstitutedReason::CenterRebased {
            registered_window: center,
            observed_window: None,
        }),
        "中枢消失：observed_window = None"
    );
    assert_eq!(book.alarms().center_rebased, 1);
    settled(&book);
}

#[test]
fn reconcile_window_same_window_is_zero_action() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let before = book.entry(&key_of(center, 3)).unwrap().clone();
    let journal_before = book.journal().to_vec();

    let step = book.reconcile_window(center.anchor(), Some(center), 900);
    assert!(step.is_none(), "同窗口重确认零动作：不产生任何 step");

    let after = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(after, &before, "条目一个 bit 不动（含门卫钟）");
    assert_eq!(book.journal(), journal_before.as_slice(), "日志一个 bit 不动");
    assert_eq!(book.alarms().center_rebased, 0);
    settled(&book);
}

#[test]
fn reconcile_window_on_unregistered_anchor_is_none() {
    let mut book = ledger();
    let anchor = frame(1_200).anchor();
    assert!(book.reconcile_window(anchor, Some(frame(1_400)), 500).is_none());
    assert!(book.is_empty());
    settled(&book);
}

#[test]
fn reconcile_window_on_already_terminal_candidate_is_zero_action() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let before = book.entry(&key_of(center, 3)).unwrap().clone();

    let step = book.reconcile_window(center.anchor(), Some(frame(1_400)), 900);
    assert!(step.is_none(), "已终态候选：窗口核对与之无关");
    assert_eq!(book.entry(&key_of(center, 3)).unwrap(), &before);
    assert_eq!(book.alarms().center_rebased, 0);
    settled(&book);
}
