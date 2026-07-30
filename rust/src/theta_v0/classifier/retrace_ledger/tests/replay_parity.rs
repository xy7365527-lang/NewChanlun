//! 旧模块 `first_retrace_replay` fixtures → 新账本行为等价面对拍（票 #624 S4 ①）。
//!
//! # 基线表与差异全枚举
//!
//! 九条 fixture 的逐条基线（输入 / 旧输出 / 新账本对应面 / 判定）落在
//! `chanlun/review-results/issue624-fixture-replay-diff-20260729.md`。本文件只承载**新账本侧
//! 确有对应面**的那四条（F4 / F6 / F7 / F8）：
//!
//! | fixture | 旧输出 | 新账本对应面 | 关系 |
//! |---|---|---|---|
//! | F4 `first_pair_collapses_into_same_completed_c2_move_later` | `StrictPairError::SameMove` | [`RetraceRejection::NotAdjacent`] | 有意改变（错误码合并） |
//! | F6 `same_identity_late_success_is_rejected_after_reentry` | `RetraceLifecycleError::FirstRetraceConsumed` | 终态静默吸收 + 警报计数 | 有意改变（#574 裁定三） |
//! | F7 `new_departure_supersedes_then_restarts_with_new_identity` | `Supersede(old) → Restart(new)` 事件对 | 新档注册 + [`RetraceRevisionKind::Restarted`] 谱系载荷 | 有意改变（无 Supersede 事件名） |
//! | F7' 同上（错误面） | `RetraceLifecycleError::PairDepartureMismatch` | [`RetraceRejection::ActiveCandidateNotSettled`] | 有意改变（错误码改名） |
//! | F8 `without_strict_pairs_emits_no_lifecycle_events` | `Ok(vec![])` | 空账本 + 空日志 | 等价 |
//!
//! 其余五条不在本文件：F1/F2/F9 是 `level_view` / `decompose` 面（F1 迁至
//! `level_view/tests/projection_pairing.rs`，F2 已被该文件既有测试覆盖，F9 是同义反复夹具），
//! F3/F5 是 D1 seed → 唯一 CompletedMove 映射面——裁定 A 删除该原语后新账本**不覆盖**
//! （账本入口只接已配对 pair，映射职责在 provider / `level_view` 侧）。报告 §四逐条登记。

use super::*;

/// F4：leave / retest 落同一 CompletedMove。
///
/// 旧 `strict_completed_pair` 先判 `SameMove` 再判 `NotAdjacent`（两个独立错误码）；新账本适配器
/// 只有紧邻一条判据（`retest == leave + 1`，补充十五），`leave == retest` 落进同一码。
#[test]
fn f4_leave_and_retest_on_the_same_move_is_rejected_as_not_adjacent() {
    let mut book = ledger();
    let mut input = up_input(frame(1_200), 3, None, 500);
    input.pair = StrictCompletedPair {
        leave_move_index: 3,
        retest_move_index: 3,
    };
    assert_eq!(
        book.observe(&input).unwrap_err(),
        RetraceRejection::NotAdjacent {
            leave_move_index: 3,
            retest_move_index: 3,
        },
        "同一 move 当 leave/retest ⟹ 不紧邻桶拒收（旧 SameMove 的对应面）"
    );
    assert!(book.is_empty(), "拒收零建仓");
    settled(&book);
}

/// F6：同身份在判败后晚到成功。
///
/// 旧模块 `FirstRetraceConsumed` **报错**；新账本按 #574 裁定三**静默吸收 + 警报计数**——迟到
/// 输入改不动任何既成事实，判它 `Err` 只会把调用方拖进本该静默的既成事实（`book.rs`
/// `advance_existing` 文档）。消费一次性本身未变：留档一个 bit 不动，终态不复活。
#[test]
fn f6_same_identity_late_success_after_reentry_is_absorbed_not_errored() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        500,
    ))
    .unwrap();
    let before = book.entry(&key_of(center, 3)).unwrap().clone();

    let late = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap();
    assert!(late.absorbed_late(), "晚到成功 ⟹ 静默吸收，不是 Err");
    assert!(late.delta.is_empty(), "静默 = 零修订");
    assert_eq!(late.state, RetraceState::Invalidated, "禁复活");

    let after = book.entry(&key_of(center, 3)).unwrap();
    assert_eq!(
        after.revisions, before.revisions,
        "消费一次性：留档一个 bit 不动"
    );
    assert_eq!(
        after.not_constituted_reason(),
        Some(NotConstitutedReason::RetestReentered),
        "判败名分不被晚到成功改写"
    );
    assert_eq!(
        book.alarms().late_absorbed,
        1,
        "旧模块的报错面 ⟹ 新账本的警报计数"
    );
    settled(&book);
}

/// F7：判败后不同 departure 到达。
///
/// 旧模块产 `Supersede(old) → Restart(new)` 一对事件；新账本没有 `Supersede` 事件名——旧候选在
/// 判败那拍已落锤成终态（事件即 `NotConstituted{RetestReentered}`），「退位」不需要第二条事件。
/// 新一代的谱系由新档的 [`RetraceRevisionKind::Restarted`] 载荷记前任（#574 裁定三：Restart 是
/// 新档，不走桥迁移）。
#[test]
fn f7_new_departure_after_reentry_opens_restarted_entry_instead_of_supersede_event() {
    let mut book = ledger();
    let first = frame(1_200);
    book.observe(&up_input(
        first,
        3,
        Some(RetraceOutcome::RetestReenters),
        500,
    ))
    .unwrap();

    let step = book
        .observe(&up_input(
            frame(1_400),
            5,
            Some(RetraceOutcome::Success),
            700,
        ))
        .unwrap();
    let entry = book.entry(&step.key).unwrap();
    assert_eq!(
        entry.restarted_from(),
        Some(key_of(first, 3)),
        "新档谱系载荷记前任 = 旧 Supersede→Restart 的等价面"
    );
    assert_eq!(
        kinds(entry),
        vec![
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
            RetraceRevisionKind::Restarted {
                previous: key_of(first, 3)
            },
            RetraceRevisionKind::Confirmed,
        ],
        "新档词汇序：建项 → 钉快照 → 谱系 → 判胜"
    );
    assert_eq!(
        book.entry(&key_of(first, 3)).unwrap().state,
        RetraceState::Invalidated,
        "前任档一个 bit 不动（历史身份全部留档不删）"
    );
    settled(&book);
}

/// F7'：旧候选未判完就来新 departure。
///
/// 旧模块 `restart_allowed == false` ⟹ `PairDepartureMismatch`；新账本 ⟹
/// [`RetraceRejection::ActiveCandidateNotSettled`]（#574 裁定二：provider 有病，fail-loud）。
/// 两者判据同构（未获重启许可即换 departure），只是错误码改名、并按裁定二归入注册期拒收计数。
#[test]
fn f7_new_departure_before_the_old_one_settles_is_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let rejection = book
        .observe(&up_input(
            frame(1_400),
            5,
            Some(RetraceOutcome::Success),
            700,
        ))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::ActiveCandidateNotSettled {
            active: key_of(center, 3),
            incoming_departure_move_index: 5,
        },
        "未判完即换 departure ⟹ 拒收（旧 PairDepartureMismatch 的对应面）"
    );
    assert_eq!(book.len(), 1, "拒收零建仓");
    assert_eq!(book.alarms().registration_rejected, 1);
    settled(&book);
}

/// F8：零观察。
///
/// 旧模块 `replay_first_retrace(identity, &[]) == Ok(vec![])`；新账本零观察 ⟹ 空账本 + 空日志 +
/// 零警报。**等价**（唯一差别是新账本连身份都不预置——旧模块要求先给一个 `initial` 身份）。
#[test]
fn f8_without_any_observation_the_ledger_and_log_stay_empty() {
    let book = ledger();
    assert!(book.is_empty(), "零观察零建仓");
    assert!(book.journal().is_empty(), "零观察零日志");
    assert!(book.established().is_empty());
    assert_eq!(book.alarms(), RetraceAlarms::default());
    settled(&book);
}

/// F7 同中枢补测（影子评审 #624 T3 MEDIUM-1）。
///
/// 上面的 [`f7_new_departure_after_reentry_opens_restarted_entry_instead_of_supersede_event`]
/// 用 `frame(1_200)` → `frame(1_400)`（同锚异框）；旧 fixture
/// `lv_case2_new_departure_supersedes_then_restarts_with_new_identity` 的真实输入面是
/// `new = RetraceIdentity { center: old.center, departure_move_index: 5 }`——**同一个 `center`**，
/// 只有 `departure` 从 3 变 5。本测试逐条复刻这条输入面（同框，非同锚异框），补齐 MEDIUM-1
/// 指出的「旧 fixture 的真实输入一条测试都没跑」缺口。
#[test]
fn f7_same_center_new_departure_after_reentry_opens_restarted_entry_instead_of_supersede_event() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(
        center,
        3,
        Some(RetraceOutcome::RetestReenters),
        500,
    ))
    .unwrap();

    let step = book
        .observe(&up_input(center, 5, Some(RetraceOutcome::Success), 700))
        .unwrap();
    let entry = book.entry(&step.key).unwrap();
    assert_eq!(
        entry.restarted_from(),
        Some(key_of(center, 3)),
        "同中枢：新档谱系载荷记前任 = 旧 Supersede→Restart 的等价面"
    );
    assert_eq!(
        kinds(entry),
        vec![
            RetraceRevisionKind::Registered,
            RetraceRevisionKind::SnapshotPinned,
            RetraceRevisionKind::Restarted {
                previous: key_of(center, 3)
            },
            RetraceRevisionKind::Confirmed,
        ],
        "同中枢：新档词汇序不因异框/同框而变"
    );
    assert_eq!(
        book.entry(&key_of(center, 3)).unwrap().state,
        RetraceState::Invalidated,
        "同中枢：前任档一个 bit 不动"
    );
    settled(&book);
}

/// F7' 同中枢补测（影子评审 #624 T3 MEDIUM-1）。
///
/// 复刻 [`f7_new_departure_before_the_old_one_settles_is_rejected`] 的判据，但换成与旧
/// `RetraceIdentity { center: old.center, .. }` 一致的同一 `center`——验证「未获重启许可即换
/// departure」的拒收判据不依赖 frame 是否同框。
#[test]
fn f7_prime_same_center_new_departure_before_the_old_one_settles_is_rejected() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let rejection = book
        .observe(&up_input(center, 5, Some(RetraceOutcome::Success), 700))
        .unwrap_err();
    assert_eq!(
        rejection,
        RetraceRejection::ActiveCandidateNotSettled {
            active: key_of(center, 3),
            incoming_departure_move_index: 5,
        },
        "同中枢：未判完即换 departure ⟹ 拒收（旧 PairDepartureMismatch 的对应面）"
    );
    assert_eq!(book.len(), 1, "拒收零建仓");
    assert_eq!(book.alarms().registration_rejected, 1);
    settled(&book);
}
