//! #1406 C5 H2：真实 /1 handle 的源捕获借入受控 Job，隔离验证 SQL 累计与失败。
//! 注入仅 cfg(test) 存在；不替代 /2 固定卷/内容节点容量验收，不修改原件/金额期望。
use super::tb03c_c5_tests::{apply_sources, transfer_documents, Session};
use crate::s_content::job::{Existing, Job, JobLimits, JobSnapshot, DEADLINE, LIMIT_EXCEEDED};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Hooks {
    ready: Option<Job>,
    completed: Option<Job>,
    snapshot: Option<JobSnapshot>,
    clock: Option<Rc<Cell<Instant>>>,
    active: bool,
    expire_after: Option<usize>,
    expire_external_after: Option<usize>,
    corrupt_proof: bool,
    reads: Vec<u64>,
}
thread_local! { static HOOKS: RefCell<Hooks> = RefCell::new(Hooks::default()); }
pub(super) struct SqlReturnDeadline;
impl Drop for SqlReturnDeadline {
    fn drop(&mut self) {
        HOOKS.with(|state| *state.borrow_mut() = Hooks::default());
    }
}
/// /2 内容 SQL 读口锁复用同一个“已返回、计费前”时钟触发点；不替换读取或节点。
pub(super) fn expire_on_next_sql_return(clock: Rc<Cell<Instant>>) -> SqlReturnDeadline {
    expire_after_sql_returns(clock, 1)
}
pub(super) fn expire_after_sql_returns(
    clock: Rc<Cell<Instant>>,
    returns: usize,
) -> SqlReturnDeadline {
    assert!(returns > 0);
    HOOKS.with(|state| {
        *state.borrow_mut() = Hooks {
            active: true,
            clock: Some(clock),
            expire_after: Some(returns),
            ..Hooks::default()
        }
    });
    SqlReturnDeadline
}
pub(super) struct InjectedJob {
    pub(super) job: Option<Job>,
}
impl Drop for InjectedJob {
    fn drop(&mut self) {
        HOOKS.with(|state| {
            let mut state = state.borrow_mut();
            state.snapshot = self.job.as_ref().map(Job::snapshot);
            state.completed = self.job.take();
            state.active = false;
        });
    }
}
pub(super) fn take_job() -> Option<InjectedJob> {
    HOOKS.with(|state| {
        let mut state = state.borrow_mut();
        let job = state.ready.take()?;
        state.active = true;
        Some(InjectedJob { job: Some(job) })
    })
}
pub(super) fn read_returned(bytes: u64) -> Option<Instant> {
    HOOKS.with(|state| {
        let mut state = state.borrow_mut();
        if !state.active {
            return None;
        }
        state.reads.push(bytes);
        if state.expire_after == Some(state.reads.len()) {
            let clock = state.clock.as_ref().unwrap();
            clock.set(clock.get() + Duration::from_millis(1));
        }
        (state.expire_external_after == Some(state.reads.len())).then(Instant::now)
    })
}
pub(super) fn proof_read(record: &mut Value) {
    HOOKS.with(|state| {
        let state = state.borrow();
        if state.active && state.corrupt_proof {
            record["b_source_debit_proof"]["previous_root"] = json!("testonly-proof-corruption");
        }
    });
}
fn limits() -> JobLimits {
    JobLimits {
        max_total_wire_bytes: 16_777_216,
        max_total_node_bytes: 16_777_216,
        max_total_nodes: 100_000,
        max_total_records: 100_000,
        max_total_requests: 100,
        max_elapsed_ms: 1,
        max_staged_bytes: 16_777_216,
        max_store_bytes: 16_777_216,
    }
}
fn arm(s: &Session, limit: JobLimits, expire_after: Option<usize>, corrupt_proof: bool) {
    let clock = Rc::new(Cell::new(Instant::now()));
    let clock_for_job = clock.clone();
    let job = Job::new(
        limit,
        Existing {
            store_bytes: s.allocated(2),
            staged_bytes: 0,
        },
        Box::new(move || clock_for_job.get()),
    )
    .unwrap();
    HOOKS.with(|state| {
        *state.borrow_mut() = Hooks {
            ready: Some(job),
            clock: Some(clock),
            expire_after,
            corrupt_proof,
            ..Hooks::default()
        }
    });
}
/// 重用失败前的同一个 Job，不新建、不改 clock、不清累计账。
fn rearm_same_job() {
    HOOKS.with(|state| {
        let mut state = state.borrow_mut();
        state.ready = state.completed.take();
        assert!(state.ready.is_some());
        state.corrupt_proof = false;
        state.expire_after = None;
        state.expire_external_after = None;
    });
}
fn checked_snapshot() -> JobSnapshot {
    HOOKS.with(|state| {
        let state = state.borrow();
        let snapshot = state.snapshot.clone().unwrap();
        assert_eq!(
            snapshot.wire_bytes,
            state.reads.iter().map(|n| u128::from(*n)).sum::<u128>()
        );
        // SQL trace 只校核 wire；/1 没有六族/Delta 逻辑记录访问。
        assert_eq!(snapshot.records, 0);
        snapshot
    })
}
#[derive(Clone, Copy, PartialEq)]
enum Source {
    Applied,
    Missing,
    Unmatched,
}
fn prepare(tag: &str, source_state: Source) -> (Session, Value, Value) {
    let (mut source, target, _) = transfer_documents();
    if source_state == Source::Unmatched {
        source["events"][3]["family_id"] = json!("other-source-family");
    }
    let mut s = Session::new(tag);
    apply_sources(&mut s, &source, 3);
    let outgoing = s.capture(&source, 3);
    assert_eq!(outgoing["kind"], "Recorded", "{outgoing}");
    if source_state != Source::Missing {
        let applied = s.apply(1, &outgoing);
        assert_eq!(
            applied["kind"],
            if source_state == Source::Applied {
                "Applied"
            } else {
                "Unmatched"
            },
            "{applied}"
        );
    }
    let incoming = s.capture(&target, 0);
    assert_eq!(incoming["kind"], "Recorded", "{incoming}");
    (s, outgoing, incoming)
}
fn assert_zero_target(s: &Session, before: &Value, rows: &[i64]) {
    assert_eq!(&s.image(2), before);
    assert_eq!(s.row_counts(2), rows);
    s.audit();
}

#[test]
fn c5_h2_missing_receipt_reads_count_and_retry_reuses_the_same_job() {
    let (mut s, outgoing, incoming) = prepare("c5-h2-missing", Source::Missing);
    let before = s.image(2);
    let rows = s.row_counts(2);
    arm(&s, limits(), None, false);
    let rejected = s.apply(2, &incoming);
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingDependency"),
        "{rejected}"
    );
    let first = checked_snapshot();
    assert!(first.wire_bytes > 0);
    assert_eq!(first.requests, 1);
    assert_eq!(first.stopped, None);
    assert_zero_target(&s, &before, &rows);
    assert_eq!(s.apply(1, &outgoing)["kind"], "Applied");
    rearm_same_job();
    let applied = s.apply(2, &incoming);
    assert_eq!(applied["kind"], "Applied", "{applied}");
    let second = checked_snapshot();
    assert!(second.wire_bytes > first.wire_bytes);
    assert_eq!(second.requests, 2);
    s.recover_all();
}

#[test]
fn c5_h2_unmatched_receipt_preserves_audited_read_cost_and_zero_target() {
    let (mut s, _, incoming) = prepare("c5-h2-unmatched", Source::Unmatched);
    let before = s.image(2);
    let rows = s.row_counts(2);
    arm(&s, limits(), None, false);
    let rejected = s.apply(2, &incoming);
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingDependency"),
        "{rejected}"
    );
    let snapshot = checked_snapshot();
    assert!(snapshot.wire_bytes > 0);
    assert_eq!(snapshot.stopped, None);
    assert_zero_target(&s, &before, &rows);
    s.recover_all();
}

#[test]
fn c5_h2_proof_failure_keeps_full_read_cost_then_healthy_retry_can_apply() {
    let (mut s, _, incoming) = prepare("c5-h2-proof", Source::Applied);
    let before = s.image(2);
    let rows = s.row_counts(2);
    arm(&s, limits(), None, true);
    let rejected = s.apply(2, &incoming);
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .starts_with("IdentityConflict"),
        "{rejected}"
    );
    let failed = checked_snapshot();
    assert!(failed.wire_bytes > 0);
    assert_eq!(failed.stopped, None);
    assert_zero_target(&s, &before, &rows);
    rearm_same_job();
    assert_eq!(s.apply(2, &incoming)["kind"], "Applied");
    let healthy = checked_snapshot();
    // 同一源快照的二次实读全额再计，未按根或字节去重。
    assert_eq!(healthy.wire_bytes, failed.wire_bytes * 2);
    assert_eq!(healthy.records, failed.records * 2);
    assert_eq!(healthy.requests, 2);
    s.recover_all();
}

#[test]
fn c5_h2_read_completed_at_deadline_counts_before_stopping() {
    let (mut s, _, incoming) = prepare("c5-h2-deadline", Source::Applied);
    let before = s.image(2);
    let rows = s.row_counts(2);
    arm(&s, limits(), Some(1), false);
    let rejected = s.apply(2, &incoming);
    assert!(
        rejected["error"].as_str().unwrap().contains(DEADLINE),
        "{rejected}"
    );
    let snapshot = checked_snapshot();
    assert_eq!(snapshot.stopped, Some(DEADLINE));
    assert_eq!(snapshot.records, 0);
    assert!(snapshot.wire_bytes > 0);
    assert_zero_target(&s, &before, &rows);
    rearm_same_job();
    assert!(s.apply(2, &incoming)["error"]
        .as_str()
        .unwrap()
        .contains(DEADLINE));
    assert_eq!(checked_snapshot().wire_bytes, snapshot.wire_bytes);
    assert_eq!(checked_snapshot().records, snapshot.records);
}

#[test]
fn c5_h2_low_wire_limit_keeps_failed_bytes_and_does_not_charge_sql_rows_as_records() {
    let (mut s, _, incoming) = prepare("c5-h2-limit", Source::Applied);
    let before = s.image(2);
    let rows = s.row_counts(2);
    let mut limit = limits();
    limit.max_total_wire_bytes = 1;
    limit.max_total_records = 1;
    arm(&s, limit, None, false);
    let rejected = s.apply(2, &incoming);
    assert!(
        rejected["error"].as_str().unwrap().contains(LIMIT_EXCEEDED),
        "{rejected}"
    );
    let snapshot = checked_snapshot();
    assert_eq!(snapshot.stopped, Some(LIMIT_EXCEEDED));
    assert!(snapshot.wire_bytes > 1);
    assert_eq!(snapshot.records, 0);
    assert_zero_target(&s, &before, &rows);
    rearm_same_job();
    assert!(s.apply(2, &incoming)["error"].is_string());
    assert_eq!(checked_snapshot().wire_bytes, snapshot.wire_bytes);
    assert_eq!(checked_snapshot().records, 0);
}

#[test]
fn c5_h2_external_deadline_after_return_does_not_erase_read_accounting() {
    let (mut s, _, incoming) = prepare("c5-h2-ingress", Source::Applied);
    let before = s.image(2);
    let rows = s.row_counts(2);
    arm(&s, limits(), None, false);
    HOOKS.with(|state| state.borrow_mut().expire_external_after = Some(1));
    let rejected = s.apply(2, &incoming);
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .contains("最早绝对期限已过"),
        "{rejected}"
    );
    let snapshot = checked_snapshot();
    assert_eq!(snapshot.records, 0);
    assert!(snapshot.wire_bytes > 0);
    // Job 自身期限尚未到，先到的是外部 ingress；不能拿另一个钟覆盖它。
    assert_eq!(snapshot.stopped, None);
    assert_zero_target(&s, &before, &rows);
}

#[test]
fn c5_h2_sql_no_payload_errors_differ_from_parse_failure_after_payload() {
    use super::store::{metered_row, ReadBudget};
    let c = rusqlite::Connection::open_in_memory().unwrap();
    c.execute_batch(
        "CREATE TABLE facts(k TEXT PRIMARY KEY,json TEXT); INSERT INTO facts VALUES('bad','{');",
    )
    .unwrap();
    let now = Instant::now();
    // 本测试只有内存 DB，没有磁盘暂存/内容存储，0 是实际基线。
    let mut job = Job::new(
        limits(),
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(move || now),
    )
    .unwrap();
    let mut budget = ReadBudget::new(Some(&mut job), None);
    let prepare_error: Result<String, String> =
        budget.one(&c, "SELECT absent_column", [], |r| r.get(0));
    assert!(prepare_error.is_err());
    let step_error: Result<i64, String> =
        budget.one(&c, "SELECT abs(-9223372036854775808)", [], |r| r.get(0));
    assert!(step_error.is_err());
    assert_eq!(budget.job.as_ref().unwrap().snapshot().wire_bytes, 0);
    assert_eq!(budget.job.as_ref().unwrap().snapshot().records, 0);
    assert!(metered_row(&c, "facts", "bad", &mut budget)
        .unwrap_err()
        .starts_with("SchemaUnsupported"));
    assert_eq!(budget.job.as_ref().unwrap().snapshot().wire_bytes, 1);
    assert_eq!(budget.job.as_ref().unwrap().snapshot().records, 0);
    // 不带 Job 也要在真实 SQL 前拒绝已经到达的外部期限。
    let mut no_job = ReadBudget::new(None, Some(Instant::now()));
    let expired: Result<i64, String> = no_job.one(&c, "SELECT absent_column", [], |r| r.get(0));
    assert!(expired.unwrap_err().contains("最早绝对期限已过"));
}
