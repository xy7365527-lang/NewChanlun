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

    // 影子评审 #622 S2 MEDIUM-3 修复：旧档之死不再只混在 delta 里逐条比对才能发现——
    // `step.killed` 显式带出被处死的旧档身份，`step.delta` 里也确实带着那条 CenterRebased 修订。
    assert_eq!(
        step.killed,
        Some(key_of(old, 3)),
        "本轮处死的旧档身份显式可查"
    );
    let killed_revision = step
        .delta
        .as_slice()
        .iter()
        .find(|revision| revision.key == key_of(old, 3))
        .expect("delta 内必含旧档的处死修订");
    assert_eq!(
        killed_revision.kind,
        RetraceRevisionKind::NotConstituted {
            reason: NotConstitutedReason::CenterRebased {
                registered_window: old,
                observed_window: Some(rebased),
            }
        },
        "delta 内旧档修订即处死记档，不是别的词汇"
    );
    assert_eq!(killed_revision.as_of, 700, "处死修订的知情时 = 处死拍");
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
        book.entry(&key_of(old, 3))
            .unwrap()
            .not_constituted_reason(),
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
    book.observe(&up_input(
        first,
        3,
        Some(RetraceOutcome::RetestReenters),
        500,
    ))
    .unwrap();

    let second = frame(1_400);
    book.observe(&up_input(second, 5, None, 700)).unwrap();
    let rebased = frame(1_600);
    book.observe(&up_input(rebased, 5, None, 900)).unwrap();

    assert_eq!(
        book.entry(&key_of(first, 3))
            .unwrap()
            .not_constituted_reason(),
        Some(NotConstitutedReason::RetestReentered),
        "首任判败原因码不被后续改口覆盖"
    );
    assert_eq!(
        book.entry(&key_of(second, 5))
            .unwrap()
            .not_constituted_reason(),
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

    let rejection = book
        .observe(&up_input(frame(1_400), 7, None, 700))
        .unwrap_err();
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
        .unwrap()
        .unwrap();
    assert_eq!(step.key, key_of(old, 3));
    assert_eq!(step.state, RetraceState::Invalidated);
    assert_eq!(
        step.killed,
        Some(key_of(old, 3)),
        "本通道 key 即被处死的档，killed 同样显式带出"
    );

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

    let step = book
        .reconcile_window(center.anchor(), None, 900)
        .unwrap()
        .unwrap();
    assert_eq!(step.state, RetraceState::Invalidated);
    assert_eq!(
        book.entry(&key_of(center, 3))
            .unwrap()
            .not_constituted_reason(),
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

    let step = book
        .reconcile_window(center.anchor(), Some(center), 900)
        .unwrap();
    assert!(step.is_none(), "同窗口重确认零动作：不产生任何 step");

    let after = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(after, &before, "条目一个 bit 不动（含门卫钟）");
    assert_eq!(
        book.journal(),
        journal_before.as_slice(),
        "日志一个 bit 不动"
    );
    assert_eq!(book.alarms().center_rebased, 0);
    settled(&book);
}

#[test]
fn reconcile_window_on_unregistered_anchor_is_none() {
    let mut book = ledger();
    let anchor = frame(1_200).anchor();
    assert!(book
        .reconcile_window(anchor, Some(frame(1_400)), 500)
        .unwrap()
        .is_none());
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

    let step = book
        .reconcile_window(center.anchor(), Some(frame(1_400)), 900)
        .unwrap();
    assert!(step.is_none(), "已终态候选：窗口核对与之无关");
    assert_eq!(book.entry(&key_of(center, 3)).unwrap(), &before);
    assert_eq!(book.alarms().center_rebased, 0);
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// 改口处死知情时护栏（票 #622 S2 修复轮 HIGH-1；影子评审探针 p9/p9b/p2）：`kill_as_rebased`
// 是唯一不经 `LedgerBook::admit` 倒退门的终态落账路径，落锤前须校验 `as_of ≥` 旧档门卫钟
// （`last_as_of`），否则可静默写出违反内核「出生钟 ≤ 终态钟」不变量的账本条目。两条通道
// （`observe` 碰撞分支、`reconcile_window`）都补护栏。
// ═══════════════════════════════════════════════════════════════════════════

/// 探针 p9b：`observe` 碰撞分支——陈旧知情时（早于旧档注册知情时，遑论门卫钟）不得处死旧档。
#[test]
fn observe_collision_rebase_with_as_of_behind_registration_is_rejected_fail_loud() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap(); // registered_as_of = last_as_of = 500

    let rebased = frame(1_400);
    let rejection = book.observe(&up_input(rebased, 3, None, 100)).unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::RebaseAsOfBehindGate {
            key: key_of(old, 3),
            gate_as_of: 500,
            as_of: 100,
        },
        "陈旧知情时 fail-loud 拒收，不得静默写坏落锤钟"
    );

    let entry = book.entry(&key_of(old, 3)).unwrap();
    assert_eq!(
        entry.state,
        RetraceState::Provisional,
        "旧档一个 bit 不动：处死零改写"
    );
    assert_eq!(entry.last_as_of, 500, "门卫钟不动");
    assert!(entry.terminal_as_of.is_none(), "落锤钟未被写坏");
    assert!(
        book.entry(&key_of(rebased, 3)).is_none(),
        "新档也未建仓——护栏在建仓前拦下"
    );
    assert_eq!(book.len(), 1, "只有旧档一条留档");
    assert_eq!(book.alarms().center_rebased, 0, "护栏拒收不是改口");
    assert_eq!(
        book.alarms().registration_rejected,
        1,
        "护栏拒收计入注册期拒收桶"
    );
    let record = book
        .audit_log()
        .last()
        .copied()
        .expect("护栏拒收落 audit 流");
    assert_eq!(
        record.event,
        RetraceAuditEvent::ResidualRejected {
            as_of: 100,
            code: RetraceRejectionCode::RebaseAsOfBehindGate,
        }
    );
    settled(&book);
}

/// 探针 p2：较弱变体——`as_of` 早于旧档**门卫钟**但不早于**出生钟**（门卫钟已被中途一次未决
/// 观察前移）。此前只查「不算倒退」放行，会把落锤钟静默写倒退；护栏落地后同样 fail-loud。
#[test]
fn observe_collision_rebase_with_as_of_behind_advanced_gate_but_after_registration_is_rejected() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap(); // registered_as_of = 500
    book.observe(&up_input(old, 3, None, 900)).unwrap(); // 同身份未决观察前移门卫钟至 900

    let rebased = frame(1_400);
    let rejection = book.observe(&up_input(rebased, 3, None, 600)).unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::RebaseAsOfBehindGate {
            key: key_of(old, 3),
            gate_as_of: 900,
            as_of: 600,
        },
        "600 晚于出生钟 500 但早于门卫钟 900——弱变体同样须拒收，不能只查出生钟"
    );

    let entry = book.entry(&key_of(old, 3)).unwrap();
    assert_eq!(entry.registered_as_of, 500, "出生钟不动");
    assert_eq!(entry.last_as_of, 900, "门卫钟维持中途前移值，不被 600 覆盖");
    assert!(entry.terminal_as_of.is_none());
    assert_eq!(book.alarms().center_rebased, 0);
    settled(&book);
}

/// 探针 p9：`reconcile_window` 通道同样受护栏覆盖。
#[test]
fn reconcile_window_rejects_as_of_behind_gate_fail_loud() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap(); // registered_as_of = last_as_of = 500

    let rejection = book
        .reconcile_window(old.anchor(), Some(frame(1_500)), 100)
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::RebaseAsOfBehindGate {
            key: key_of(old, 3),
            gate_as_of: 500,
            as_of: 100,
        }
    );

    let entry = book.entry(&key_of(old, 3)).unwrap();
    assert_eq!(entry.state, RetraceState::Provisional, "零改写");
    assert_eq!(entry.last_as_of, 500);
    assert_eq!(book.alarms().center_rebased, 0);
    assert_eq!(book.alarms().registration_rejected, 1);
    settled(&book);
}

// ═══════════════════════════════════════════════════════════════════════════
// `center_rebased` 现算一致性（票 #622 S2 修复轮 MEDIUM-2；影子评审探针 p5）：live 值与
// `fold(journal)` 折叠后的值须恒等——此前独立可变字段在 fold 后归零，与「已进日志的真实终态
// 事件」这一定位矛盾。
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// 恢复点护栏叠加 `kill_as_rebased`（票 #635，修复 #632 窄口）：`kill_as_rebased` 与
// `RebaseAsOfBehindGate` 各管各的门卫钟（该身份自己的 `last_as_of`），`guard_settle_after_recovery`
// 管的是调用方声明的恢复点（[`RetraceLedger::fold_recovered`]）——两者独立叠加。此前恢复点护栏
// 只挂在 `judge` 产出判胜/判败的两处入口，`kill_as_rebased` 这条独立终态落账路径未覆盖：一个
// 通过 `RebaseAsOfBehindGate`（`as_of ≥` 旧档门卫钟）但早于恢复点的陈旧 `as_of` 此前能悄悄处死
// 旧候选，与 #621 影子 MEDIUM-1 同形但更窄。两条调用路径（`observe` 碰撞分支、
// `reconcile_window`）都经由本函数，护栏落在此处两路同时覆盖。
// ═══════════════════════════════════════════════════════════════════════════

/// `observe` 碰撞分支：恢复点声明后，陈旧 `as_of`（晚于旧档门卫钟、早于恢复点）不得处死旧候选
/// ——护栏在建仓前拦下，新档也未建仓（零改写）。
#[test]
fn observe_collision_rebase_with_as_of_behind_recovery_point_is_rejected_fail_loud() {
    let mut live = ledger();
    let old = frame(1_200);
    live.observe(&up_input(old, 3, None, 500)).unwrap(); // registered_as_of = last_as_of = 500

    // 崩溃/重启：调用方声明恢复点 = 4_000（重启前已知到这里）。
    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 4_000).unwrap();
    let key = key_of(old, 3);
    let before = restored.entry(&key).unwrap().clone();

    let rebased = frame(1_400);
    let rejection = restored
        .observe(&up_input(rebased, 3, None, 600))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::SettleBehindRecoveryPoint {
            key,
            recovery_as_of: 4_000,
            as_of: 600,
        },
        "600 晚于旧档门卫钟 500（不触发 RebaseAsOfBehindGate）但早于恢复点 4000，仍须 fail-loud 拒收"
    );

    let entry = restored.entry(&key).unwrap();
    assert_eq!(entry, &before, "拒收零改写：旧档一个 bit 不动");
    assert_eq!(entry.state, RetraceState::Provisional, "旧候选未被处死");
    assert!(
        restored.entry(&key_of(rebased, 3)).is_none(),
        "新档也未建仓——护栏在建仓前拦下"
    );
    assert_eq!(restored.len(), 1, "只有旧档一条留档");
    assert_eq!(restored.alarms().center_rebased, 0, "护栏拒收不是改口");
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

/// 两门皆违反（`as_of` 同时早于旧档门卫钟与恢复点）：恢复点护栏先查，报出
/// `SettleBehindRecoveryPoint` 而非 `RebaseAsOfBehindGate`——钉死实现顺序（两判据各自独立，谁先
/// 查不影响拒收结论，仅影响错误变体）。
#[test]
fn observe_collision_rebase_violating_both_gates_reports_recovery_point_first() {
    let mut live = ledger();
    let old = frame(1_200);
    live.observe(&up_input(old, 3, None, 1_000)).unwrap(); // registered_as_of = last_as_of = 1_000
    live.observe(&up_input(old, 3, None, 2_000)).unwrap(); // 未决观察前移门卫钟至 2_000（不进日志）

    // fold_recovered 折回后门卫钟退回留档下界（1_000），恢复点声明为 5_000。
    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 5_000).unwrap();
    let key = key_of(old, 3);
    assert_eq!(
        restored.entry(&key).unwrap().last_as_of,
        1_000,
        "门卫钟退回留档下界"
    );

    let rebased = frame(1_400);
    let rejection = restored
        .observe(&up_input(rebased, 3, None, 500))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::SettleBehindRecoveryPoint {
            key,
            recovery_as_of: 5_000,
            as_of: 500,
        },
        "500 同时早于门卫钟 1000 与恢复点 5000——恢复点护栏先查，不报 RebaseAsOfBehindGate"
    );
    assert_eq!(
        restored.entry(&key).unwrap().state,
        RetraceState::Provisional,
        "零改写"
    );
    settled(&restored);
}

/// 边界：`as_of` 恰等于恢复点 ⟹ 非降语义放行（同门卫钟纪律），改口照常处死旧候选、注册新档。
#[test]
fn observe_collision_rebase_with_as_of_exactly_at_recovery_point_is_admitted() {
    let mut live = ledger();
    let old = frame(1_200);
    live.observe(&up_input(old, 3, None, 500)).unwrap();

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 600).unwrap();
    let rebased = frame(1_400);
    let step = restored.observe(&up_input(rebased, 3, None, 600)).unwrap();

    assert_eq!(
        step.killed,
        Some(key_of(old, 3)),
        "恰等于恢复点 ⟹ 放行，旧档正常处死"
    );
    let old_entry = restored.entry(&key_of(old, 3)).unwrap();
    assert_eq!(old_entry.state, RetraceState::Invalidated);
    assert_eq!(old_entry.terminal_as_of, Some(600));
    assert_eq!(
        restored
            .entry(&key_of(rebased, 3))
            .unwrap()
            .restarted_from(),
        Some(key_of(old, 3))
    );
    assert_eq!(restored.alarms().center_rebased, 1);
    assert_eq!(restored.alarms().registration_rejected, 0);
    settled(&restored);
}

/// `reconcile_window` 通道的镜像：同一恢复点场景经显式核对路径同样 fail-loud 拒收，零改写。
#[test]
fn reconcile_window_rejects_as_of_behind_recovery_point_fail_loud() {
    let mut live = ledger();
    let old = frame(1_200);
    live.observe(&up_input(old, 3, None, 500)).unwrap();

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 4_000).unwrap();
    let key = key_of(old, 3);
    let before = restored.entry(&key).unwrap().clone();

    let rejection = restored
        .reconcile_window(old.anchor(), Some(frame(1_500)), 600)
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::SettleBehindRecoveryPoint {
            key,
            recovery_as_of: 4_000,
            as_of: 600,
        }
    );

    let entry = restored.entry(&key).unwrap();
    assert_eq!(entry, &before, "拒收零改写");
    assert_eq!(entry.state, RetraceState::Provisional);
    assert_eq!(restored.alarms().center_rebased, 0);
    assert_eq!(restored.alarms().registration_rejected, 1);
    settled(&restored);
}

/// `reconcile_window` 边界：恰等于恢复点放行，处死照常发生。
#[test]
fn reconcile_window_admits_as_of_exactly_at_recovery_point() {
    let mut live = ledger();
    let old = frame(1_200);
    live.observe(&up_input(old, 3, None, 500)).unwrap();

    let mut restored = RetraceLedger::fold_recovered(provenance(), live.journal(), 600).unwrap();
    let step = restored
        .reconcile_window(old.anchor(), Some(frame(1_500)), 600)
        .unwrap()
        .unwrap();

    assert_eq!(step.state, RetraceState::Invalidated);
    assert_eq!(
        restored.entry(&key_of(old, 3)).unwrap().terminal_as_of,
        Some(600)
    );
    assert_eq!(restored.alarms().center_rebased, 1);
    assert_eq!(restored.alarms().registration_rejected, 0);
    settled(&restored);
}

/// 未声明恢复点（普通 `fold`）：历史行为不变——陈旧知情时仍能处死旧候选（订正兼容承诺，未升级
/// 调用方零约束）。
#[test]
fn plain_fold_without_recovery_point_still_lets_stale_as_of_kill_as_rebased() {
    let mut live = ledger();
    let old = frame(1_200);
    live.observe(&up_input(old, 3, None, 500)).unwrap();

    let mut restored = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    assert_eq!(restored.recovery_floor(), None, "普通 fold 不设恢复点");

    let rebased = frame(1_400);
    let step = restored.observe(&up_input(rebased, 3, None, 600)).unwrap();
    assert_eq!(
        step.killed,
        Some(key_of(old, 3)),
        "未设恢复点 ⟹ 零约束，历史行为不变"
    );
    assert_eq!(restored.alarms().center_rebased, 1);
    settled(&restored);
}

#[test]
fn center_rebased_survives_fold_journal_round_trip() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap();
    book.observe(&up_input(frame(1_400), 3, None, 700)).unwrap(); // 改口处死旧档

    assert_eq!(book.alarms().center_rebased, 1, "live 计数照常累加");

    let folded = RetraceLedger::fold(provenance(), book.journal()).expect("自产日志必可折叠回状态");
    assert_eq!(
        folded.alarms().center_rebased,
        1,
        "fold 后现算值必须与 live 一致——已进日志的真实终态事件，不应归零"
    );
    settled(&book);
}
