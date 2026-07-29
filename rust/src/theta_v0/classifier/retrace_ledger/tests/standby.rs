//! 备战档门户（票 #623 AC①）：未决可见可枚举；类型面隔离不可消费成买入信号。

use super::*;

#[test]
fn standby_portal_enumerates_pending_with_position_and_frame_snapshot() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();

    let watches = book.standby();
    assert_eq!(watches.len(), 1, "未决候选可枚举");
    let watch = watches[0];
    assert_eq!(watch.identity, key_of(center, 3));
    assert_eq!(watch.side, RetraceSide::Buy);
    assert_eq!(watch.frame, center, "四条边快照（活着期间不动）");
    assert_eq!(watch.leave_end.index, 1_210, "离开的边");
    assert_eq!(watch.registered_as_of, 500, "出生知情时");
    assert_eq!(book.standby_watch(&watch.identity), Some(watch));
    settled(&book);
}

#[test]
fn confirmed_and_failed_candidates_never_enter_the_standby_portal() {
    let mut book = ledger();
    let confirmed = frame(1_200);
    book.observe(&up_input(confirmed, 3, Some(RetraceOutcome::Success), 520))
        .unwrap();
    let failed = other_frame(300);
    book.observe(&down_input(
        failed,
        11,
        Some(RetraceOutcome::RetestReenters),
        540,
    ))
    .unwrap();

    assert!(
        book.standby().is_empty(),
        "S3 备战档只放 Provisional，不放 Confirmed / Invalidated"
    );
    assert_eq!(book.standby_watch(&key_of(confirmed, 3)), None);
    assert_eq!(book.standby_watch(&key_of(failed, 11)), None);
    settled(&book);
}

#[test]
fn standby_portal_is_deterministic_across_centers() {
    let mut book = ledger();
    let a = frame(1_200);
    let b = other_frame(300);
    book.observe(&up_input(b, 11, None, 520)).unwrap();
    book.observe(&up_input(a, 3, None, 540)).unwrap();

    let identities: Vec<_> = book.standby().iter().map(|watch| watch.identity).collect();
    let mut sorted = identities.clone();
    sorted.sort();
    assert_eq!(identities, sorted, "枚举按身份键序，无平局歧义");
    assert_eq!(identities.len(), 2);
    settled(&book);
}

/// 位置证据只含 leave_end：回抽尚未走完，`retest_end` 诚实缺席——[`StandbyWatch`] 结构上
/// 没有该字段，不是运行时留空。
#[test]
fn standby_watch_has_no_retest_position_field_by_construction() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let watch = book.standby()[0];
    // 编译期证据：下一行如果 `StandbyWatch` 带 `retest_end` 字段，本行会因“找不到字段”报错，
    // 但这里根本没有该字段可引用——结构本身就是证明。
    let StandbyWatch {
        identity: _,
        side: _,
        frame: _,
        leave_end: _,
        registered_as_of: _,
    } = watch;
    settled(&book);
}

/// 类型面隔离正面证据：[`ThirdPointPack`]（成立档）满足 `TradableSignal` 门闸；反面证据
/// （[`StandbyWatch`] 不满足）由本模块头的 `compile_fail` doctest 在编译期强制，非本测试覆盖。
#[test]
fn established_pack_satisfies_tradable_signal_gate_standby_watch_cannot() {
    fn place_order<T: TradableSignal>(signal: T) -> T {
        signal
    }

    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 860))
        .unwrap();
    let pack = book.established()[0];
    assert_eq!(place_order(pack), pack, "成立档通过 TradableSignal 门闸");
    settled(&book);
}
