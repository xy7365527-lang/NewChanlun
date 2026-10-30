//! #1527：私有 SQLite 仅通过固定只读 FD VFS。无源 RW、无默认 VFS 兜底、无文件 clone。
mod fd_wal;
mod mixed;
mod mixed_accounting;
mod mixed_sql;
use super::admission_anchor::AdmissionAccounting;
use super::admission_provider::{MeasuredDomain, Medium};
use super::audit_budget::Budget;
use newchan_rust::s_content::{self as sc, job::CheckedFrame, NodeDigest};
use std::cell::RefCell;

#[cfg(test)]
pub(crate) mod s4_read_trace {
    use std::cell::RefCell;
    #[derive(Default)]
    struct Trace {
        enabled: bool,
        phase: usize,
        bytes: [u64; 4],
        coverage: [u64; 2],
        last: Option<(&'static str, Option<i64>)>,
    }
    thread_local! { static TRACE: RefCell<Trace> = RefCell::new(Trace::default()); }
    pub(crate) fn enable() { TRACE.with(|t| *t.borrow_mut() = Trace { enabled: true, ..Trace::default() }); }
    pub(crate) fn phase(phase: usize) { TRACE.with(|t| t.borrow_mut().phase = phase); }
    pub(crate) fn read(_: i32, _: i64, len: usize) {
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled { let phase = t.phase; t.bytes[phase] += len as u64; }
        });
    }
    pub(crate) fn coverage(index: usize) {
        // 只比较同一次admission业务审计；Driver后续恢复也会解码Template。
        TRACE.with(|t| { let mut t = t.borrow_mut(); if t.enabled && t.phase == 2 { t.coverage[index] += 1; } });
    }
    pub(crate) fn audit_event(label: &'static str, generation: Option<i64>) {
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled && t.phase == 2 {
                t.last = Some((label, generation));
                eprintln!("S4_AUDIT_EVENT phase={} generation={:?} bytes={} coverage={:?}", label, generation, t.bytes[2], t.coverage);
                eprintln!("S4_AUDIT_SQL phase={} generation={:?} bytes={} node_sql_reads={}", label, generation, t.bytes[2], crate::content_store::sql_reads_on_thread());
            }
        });
    }
    fn cache_counts(conn: &rusqlite::Connection) -> [(i32, i32, i32); 2] {
        use rusqlite::ffi;
        [ffi::SQLITE_DBSTATUS_CACHE_HIT, ffi::SQLITE_DBSTATUS_CACHE_MISS].map(|verb| {
            let (mut current, mut highwater) = (0, 0);
            // SAFETY：同步借用有效连接与栈内输出；resetFlag=0，不改累计值或连接配置。
            let rc = unsafe {
                ffi::sqlite3_db_status(conn.handle(), verb, &mut current, &mut highwater, 0)
            };
            (rc, current, highwater)
        })
    }
    #[test]
    fn history_public_coverage_diagnostics_count_sql_without_probe_and_do_not_reset_pager() {
        use crate::content_store::{init_schema, sql_reads_on_thread, ReadStore};
        use newchan_rust::s_content::NodeSource;
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::v2::SCHEMA_V2).unwrap();
        conn.execute(
            "INSERT INTO s_protocol_meta VALUES(1, 's-session/3', '1', 'test-clock', '{}', 1, 'accept')",
            [],
        ).unwrap();
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let mut source = ReadStore::new(&tx).unwrap();
        let before = sql_reads_on_thread();
        source.read(&[0; 32]).unwrap();
        source.read(&[0; 32]).unwrap();
        assert_eq!(sql_reads_on_thread().checked_sub(before), Some(2));
        let first = cache_counts(&tx);
        assert_eq!(first[0].0, rusqlite::ffi::SQLITE_OK);
        assert_eq!(first[1].0, rusqlite::ffi::SQLITE_OK);
        assert!(first[0].1 > 0);
        assert_eq!(cache_counts(&tx), first);
    }
    pub(crate) fn audit_window(
        conn: &rusqlite::Connection,
        boundary: &'static str,
        edge: &'static str,
        outcome: &'static str,
    ) {
        TRACE.with(|t| {
            let t = t.borrow();
            if t.enabled && t.phase == 2 {
                let [hit, miss] = cache_counts(conn);
                eprintln!("S4_AUDIT_WINDOW boundary={} edge={} outcome={} bytes={} node_sql_reads={} cache_hit={:?} cache_miss={:?}",
                    boundary, edge, outcome, t.bytes[2],
                    crate::content_store::sql_reads_on_thread(), hit, miss);
            }
        });
    }
    pub(crate) fn begin(conn: &rusqlite::Connection, boundary: &'static str) {
        audit_window(conn, boundary, "begin", "pending");
    }
    pub(crate) fn end<T, E>(conn: &rusqlite::Connection, boundary: &'static str, result: &Result<T, E>) {
        audit_window(conn, boundary, "end", if result.is_ok() { "ok" } else { "error" });
    }
    pub(crate) fn snapshot(label: &str, bytes: u128) {
        TRACE.with(|t| { if t.borrow().enabled { eprintln!("S4_READ_ACCOUNT {} {}", label, bytes); } });
    }
    pub(crate) fn measured_bytes() -> [u64; 4] {
        TRACE.with(|t| t.borrow().bytes)
    }
    pub(crate) fn finish() -> [u64; 2] {
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled {
                eprintln!("S4_READ_SUMMARY bytes={:?} coverage={:?} last={:?}", t.bytes, t.coverage, t.last);
                eprintln!("S4_AUDIT_SQL_SUMMARY bytes={} node_sql_reads={}", t.bytes[2], crate::content_store::sql_reads_on_thread());
            }
            t.enabled = false;
            t.coverage
        })
    }
}

/// #1527：单线程观察顺序只由本次 validate 持有；不拥有或替换 Job。
/// 每个固定阶段留首个次错，不积累回调轨迹；返回原首错为前缀。
#[derive(Clone, Copy)]
enum ErrorPhase {
    Business,
    Rollback,
    Gate,
    Close,
    Registration,
    Reconcile,
    Deadline,
}
impl ErrorPhase {
    fn name(self) -> &'static str {
        match self {
            Self::Business => "业务返回",
            Self::Rollback => "回滚",
            Self::Gate => "回滚后门",
            Self::Close => "关闭连接",
            Self::Registration => "撤销注册",
            Self::Reconcile => "结原预留",
            Self::Deadline => "收尾期限",
        }
    }
}
struct ObservedErrors {
    phase: ErrorPhase,
    first: Option<String>,
    later: [Option<String>; 7],
    returned_errors: [Option<String>; 7],
    unaccepted_wire_bytes: u128,
    unaccepted_wire_overflow: bool,
}
impl ObservedErrors {
    fn new() -> Self {
        Self {
            phase: ErrorPhase::Business,
            first: None,
            later: std::array::from_fn(|_| None),
            returned_errors: std::array::from_fn(|_| None),
            unaccepted_wire_bytes: 0,
            unaccepted_wire_overflow: false,
        }
    }
    fn note(&mut self, error: String) {
        if let Some(first) = &self.first {
            if first != &error && self.later[self.phase as usize].is_none() {
                self.later[self.phase as usize] = Some(error);
            }
        } else {
            self.first = Some(error);
        }
    }
    fn returned(&mut self, phase: ErrorPhase, result: &Result<(), String>) {
        if let Err(error) = result {
            if self.first.is_none() {
                self.first = Some(error.clone());
            } else if self.first.as_ref() != Some(error)
                && !self
                    .later
                    .iter()
                    .chain(self.returned_errors.iter())
                    .any(|e| e.as_ref() == Some(error))
                && self.returned_errors[phase as usize].is_none()
            {
                // 原业务/SQL String 与更早回调原因分槽；收尾结果亦不被同阶段门覆盖。
                self.returned_errors[phase as usize] = Some(error.clone());
            }
        }
    }
    fn unaccepted_wire(&mut self, actual: u128) {
        match self.unaccepted_wire_bytes.checked_add(actual) {
            Some(total) => self.unaccepted_wire_bytes = total,
            None => {
                self.unaccepted_wire_bytes = u128::MAX;
                self.unaccepted_wire_overflow = true;
            }
        }
    }
    fn finish(self) -> Result<(), String> {
        let Some(mut first) = self.first else {
            return Ok(());
        };
        for ((phase, callback), returned) in [
            ErrorPhase::Business,
            ErrorPhase::Rollback,
            ErrorPhase::Gate,
            ErrorPhase::Close,
            ErrorPhase::Registration,
            ErrorPhase::Reconcile,
            ErrorPhase::Deadline,
        ]
        .into_iter()
        .zip(self.later)
        .zip(self.returned_errors)
        {
            for (origin, error) in [("回调", callback), ("返回", returned)] {
                if let Some(error) = error {
                    use std::fmt::Write;
                    let _ = write!(first, "；次错[{}/{origin}]={error}", phase.name());
                }
            }
        }
        if self.unaccepted_wire_bytes != 0 || self.unaccepted_wire_overflow {
            use std::fmt::Write;
            let _ = write!(
                first,
                "；未接纳实读wire={}；拒收计数溢出={}",
                self.unaccepted_wire_bytes, self.unaccepted_wire_overflow
            );
        }
        Err(first)
    }
}

/// Budget 的 &self 与 VFS 的回调共享一个受限门面；每次借用在返回前结束，
/// 不跨 SQL prepare/step/rollback/close，也不实现 Access::owner_job。
struct SharedAccounting<'s, 'j> {
    account: RefCell<&'s mut AdmissionAccounting<'j>>,
    errors: &'s RefCell<ObservedErrors>,
    // 准入先于任何写：S3 schema 核在此记下源库是否仍是 #1602 迁移前形状。
    delta: &'s std::cell::Cell<crate::DeltaShape>,
}
impl SharedAccounting<'_, '_> {
    fn observed<T>(&self, result: Result<T, sc::CodecError>) -> Result<T, sc::CodecError> {
        if let Err(error) = &result {
            self.errors.borrow_mut().note(error.to_string());
        }
        result
    }
    fn with_account<T>(
        &self,
        operation: impl FnOnce(&mut AdmissionAccounting<'_>) -> Result<T, sc::CodecError>,
    ) -> Result<T, sc::CodecError> {
        // 原 Job/VFS 门的错误在回调返回前锁存；不能事后用新 deadline 猜先后。
        let result = self
            .account
            .try_borrow_mut()
            .map_err(|_| reentrant())
            .and_then(|mut account| operation(&mut account));
        self.observed(result)
    }
}
impl Budget for SharedAccounting<'_, '_> {
    fn precheck(&self) -> Result<(), sc::CodecError> {
        self.with_account(|account| account.pre_io_check())
    }
    fn prepare_node(&self) -> Result<(), sc::CodecError> {
        self.precheck()?;
        self.with_account(|account| account.begin_first_read())
    }
    fn accept_prepared_frame(&self, frame: &CheckedFrame) -> Result<(), sc::CodecError> {
        self.with_account(|account| account.accept_checked_usage(frame.usage()).map(|_| ()))
    }
    fn node(&self, digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.prepare_node()?;
        let usage = self.observed(sc::job::CheckedNodeUsage::checked(digest, tag, body))?;
        self.with_account(|account| account.accept_checked_usage(&usage).map(|_| ()))
    }
    fn record(&self) -> Result<(), sc::CodecError> {
        self.with_account(|account| account.charge_records(1))
    }
    fn before_write(&self) -> Option<&std::cell::Cell<crate::DeltaShape>> {
        Some(self.delta)
    }
}
fn reentrant() -> sc::CodecError {
    sc::CodecError::new("admission_reentrant", "私有审计记账重入，拒绝继续")
}
impl IoAccounting for &SharedAccounting<'_, '_> {
    fn before_io(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.precheck().map_err(Into::into)
    }
    fn after_read(&mut self, actual: u64) -> Result<(), Box<dyn std::error::Error>> {
        let result = match self.account.try_borrow_mut() {
            Ok(mut account) => {
                let before = account.snapshot().wire_bytes;
                let result = account.charge_wire_bytes(actual);
                let accepted = account.snapshot().wire_bytes - before;
                // 越限那次 wire 已先入账；仅真正未接纳的实读量进入诊断，绝不补写 Job。
                self.errors
                    .borrow_mut()
                    .unaccepted_wire(u128::from(actual).saturating_sub(accepted));
                result
            }
            Err(_) => {
                self.errors.borrow_mut().unaccepted_wire(u128::from(actual));
                Err(reentrant())
            }
        };
        self.observed(result).map_err(Into::into)
    }
}

/// #1527 rejection-only 前缀；成功不携带可转移资格，实际连接仍完整重核。
pub(super) enum RejectionPrefix<'a> {
    None,
    OrdinaryStartup {
        epoch: &'a str,
    },
    NativeStartup {
        epoch: &'a str,
        context: Option<&'a mut super::request_audit::RequestAuditContext>,
        work: Option<&'a sc::capture_allocation::scratch::WorkScope>,
    },
    UnboundRecover,
    LegacyServe {
        db: &'a std::path::Path,
        epoch: &'a str,
        clock: &'a crate::v2::ClockPlan,
        profile: &'a std::path::Path,
    },
}

fn audit_legacy_serve_prefix(
    conn: &rusqlite::Connection,
    account: &SharedAccounting<'_, '_>,
    db: &std::path::Path,
    epoch: &str,
    clock: &crate::v2::ClockPlan,
    profile: &std::path::Path,
) -> Result<(), String> {
    crate::v2::verify_serve_startup(conn, db, epoch, clock, Some(account.delta), || {
        let bytes = {
            let mut access = account
                .account
                .try_borrow_mut()
                .map_err(|_| reentrant().to_string())?;
            super::admission::read_startup_accounted(&mut **access, profile, None)?
        };
        let text = String::from_utf8(bytes).map_err(|_| "StartupProfileUtf8Invalid")?;
        crate::parse_profile_text(&text)
    })
}

fn audit_business(
    conn: &rusqlite::Connection,
    account: &SharedAccounting<'_, '_>,
    prefix: &mut RejectionPrefix<'_>,
) -> Result<(), String> {
    account.precheck().map_err(|e| e.to_string())?;
    let deadline = account
        .with_account(|account| account.first_read_deadline())
        .map_err(|e| e.to_string())?;
    // rusqlite 的安全共享借用事务口；conn 的独占生命周期由 Session 保持。
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let result = (|| match crate::meta_get_opt(&tx, "protocol_revision")?.as_deref() {
        Some(super::PROTOCOL_REVISION) => {
            match prefix {
                RejectionPrefix::NativeStartup { context, work, .. } => {
                    if let Some(context) = context.as_deref_mut() {
                        super::state::audit_native_in_request_measured(
                            &tx,
                            context,
                            *work,
                            Some(deadline),
                            Some(account),
                        )?;
                    } else {
                        super::state::audit_native_current_measured(
                            &tx,
                            Some(deadline),
                            Some(account),
                        )?;
                    }
                }
                _ => {
                    super::state::audit_with_budget(&tx, Some(deadline), None, Some(account))?;
                }
            }
            match prefix {
                RejectionPrefix::None => Ok(()),
                RejectionPrefix::LegacyServe {
                    db,
                    epoch,
                    clock,
                    profile,
                } => audit_legacy_serve_prefix(&tx, account, db, epoch, clock, profile),
                RejectionPrefix::OrdinaryStartup { epoch }
                | RejectionPrefix::NativeStartup { epoch, .. } => {
                    crate::verify_writer_epoch(&tx, epoch)?;
                    super::require_current_writer_rule(
                        &crate::meta_get_opt(&tx, "rule_revision")?
                            .ok_or_else(|| super::audit_owner::invalid("缺rule_revision"))?,
                    )
                }
                RejectionPrefix::UnboundRecover => super::require_current_writer_rule(
                    &crate::meta_get_opt(&tx, "rule_revision")?
                        .ok_or_else(|| super::audit_owner::invalid("缺rule_revision"))?,
                ),
            }
        }
        None | Some(super::PREVIOUS_REVISION) => {
            // 完整原判据原样运行；旧证据图不是 s-content 分帧域，不能伪造 CheckedNodeUsage。
            crate::v2::audit_private_startup(&tx, Some(account.delta))?;
            if let RejectionPrefix::LegacyServe {
                db,
                epoch,
                clock,
                profile,
            } = prefix
            {
                audit_legacy_serve_prefix(&tx, account, db, epoch, clock, profile)?;
            }
            Err("LegacyAdmissionBudgetAdapterMissing：完整旧域审计已调用，但旧证据图节点与Snapshot/Delta尚无Job同源费用适配，禁止签发准入".into())
        }
        _ => Err("PrivateAuditProtocolUnsupported".into()),
    })();
    // #1527：先记录业务实际返回；此前回调观察到的首错仍居前。
    account
        .errors
        .borrow_mut()
        .returned(ErrorPhase::Business, &result);
    account.errors.borrow_mut().phase = ErrorPhase::Rollback;
    // 显式结束事务后再关闭 connection；原审计错误优先，清理错误不会变成成功。
    // 仅撤下 VM 计算门完成回滚/关闭；VFS 实际 I/O 门仍在，不能再作业务查询。
    let disabled = conn
        .progress_handler(0, None::<fn() -> bool>)
        .map_err(|e| e.to_string());
    account
        .errors
        .borrow_mut()
        .returned(ErrorPhase::Rollback, &disabled);
    let ended = tx.rollback().map_err(|e| e.to_string());
    account
        .errors
        .borrow_mut()
        .returned(ErrorPhase::Rollback, &ended);
    account.errors.borrow_mut().phase = ErrorPhase::Gate;
    let gate = account.precheck().map_err(|e| e.to_string());
    account
        .errors
        .borrow_mut()
        .returned(ErrorPhase::Gate, &gate);
    result.and(disabled).and(ended).and(gate)
}

/// C 回调只看受限门面；不得取得 Job、StartedAdmission 或结束资格。
pub(super) trait IoAccounting {
    fn before_io(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn after_read(&mut self, actual: u64) -> Result<(), Box<dyn std::error::Error>>;
}
impl IoAccounting for AdmissionAccounting<'_> {
    fn before_io(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.pre_io_check().map_err(Into::into)
    }
    fn after_read(&mut self, actual: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.charge_wire_bytes(actual).map_err(Into::into)
    }
}

/// 成功返回源库的 structure_deltas 形状；`PrePolicyRecords` 由 scope 带到开库迁移。
pub(super) fn validate_prefix(
    source: &MeasuredDomain,
    account: &mut AdmissionAccounting<'_>,
    mut prefix: RejectionPrefix<'_>,
) -> Result<crate::DeltaShape, String> {
    let result = validate_prefix_inner(source, account, &mut prefix);
    if result.is_err() {
        if let RejectionPrefix::NativeStartup {
            context: Some(context),
            ..
        } = &mut prefix
        {
            drop(context.take());
        }
    }
    result
}

fn validate_prefix_inner(
    source: &MeasuredDomain,
    account: &mut AdmissionAccounting<'_>,
    prefix: &mut RejectionPrefix<'_>,
) -> Result<crate::DeltaShape, String> {
    #[cfg(test)]
    s4_read_trace::snapshot("prelude_start", account.snapshot().wire_bytes);
    account.pre_io_check().map_err(|e| e.to_string())?;
    if account.is_mixed() {
        // #1527：父账已可消费，旧域SQL/walker完整闭合前仍不发布服务准入。
        return Err("MixedAdmissionNotEnabled".into());
    }
    match source.medium {
        Medium::RollbackJournal => return Err("D2ProductionCapacityMissing：JobLimits8/SResources7没有已裁的overlay payload、metadata与SQLite heap额度映射；禁止采用D2-SMALL六参数默认".into()),
        Medium::MainOnly | Medium::Wal => {}
    }
    // D1 的所有 xWrite/xDelete/xTruncate 都拒绝，未提供 xShmMap，不创建临时磁盘文件。
    // 因而此机制的新增磁盘 allocation_upper 精确为 0，不是未知上界填 0。
    let grant = account.reserve_staged(0).map_err(|e| e.to_string())?;
    let errors = RefCell::new(ObservedErrors::new());
    let delta = std::cell::Cell::new(crate::DeltaShape::Current);
    let result = (|| {
        let main = source.main_fd()?;
        let wal = source.wal_fd()?;
        let shared = SharedAccounting {
            account: RefCell::new(&mut *account),
            errors: &errors,
            delta: &delta,
        };
        let mut io = &shared;
        let registration = fd_wal::Registration::new(main, wal, &mut io)
            .map_err(|_| "PrivateVfsRegistrationFailed")?;
        let result = (|| {
            let session = registration
                .connect(true)
                .map_err(|_| "PrivateSqliteOpenFailed")?;
            #[cfg(test)]
            { s4_read_trace::snapshot("quick_start", shared.account.borrow().snapshot().wire_bytes); s4_read_trace::phase(1); }
            let check: Result<(), String> = (|| {
                let mut stmt = session
                    .connection()
                    .prepare("PRAGMA quick_check")
                    .map_err(|_| "PrivateSqliteCheckFailed")?;
                let mut rows = stmt.query([]).map_err(|_| "PrivateSqliteCheckFailed")?;
                let mut seen = false;
                while let Some(row) = rows.next().map_err(|_| "PrivateSqliteCheckFailed")? {
                    let text: String = row.get(0).map_err(|_| "PrivateSqliteCheckFailed")?;
                    if text != "ok" || seen {
                        return Err("PrivateSqliteIntegrityRejected".into());
                    }
                    seen = true;
                }
                if !seen {
                    return Err("PrivateSqliteIntegrityEmpty".into());
                }
                drop(rows);
                drop(stmt);
                #[cfg(test)]
                { s4_read_trace::snapshot("audit_start", shared.account.borrow().snapshot().wire_bytes); s4_read_trace::phase(2); }
                #[cfg(test)]
                s4_read_trace::audit_window(session.connection(), "audit_business", "begin", "pending");
                let result = audit_business(session.connection(), &shared, prefix);
                #[cfg(test)]
                s4_read_trace::audit_window(
                    session.connection(), "audit_business", "end",
                    if result.is_ok() { "ok" } else { "error" },
                );
                result
            })();
            // quick_check/业务结果在任何 close 回调之前保存。
            errors.borrow_mut().returned(ErrorPhase::Business, &check);
            #[cfg(test)]
            s4_read_trace::phase(3);
            errors.borrow_mut().phase = ErrorPhase::Close;
            let closed = session
                .close()
                .map_err(|_| "PrivateSqliteCloseFailed".to_string());
            errors.borrow_mut().returned(ErrorPhase::Close, &closed);
            check.and(closed)
        })();
        errors.borrow_mut().returned(ErrorPhase::Business, &result);
        errors.borrow_mut().phase = ErrorPhase::Registration;
        let cleanup = registration
            .finish()
            .map_err(|_| "PrivateVfsCleanupFailed".to_string());
        errors
            .borrow_mut()
            .returned(ErrorPhase::Registration, &cleanup);
        result.and(cleanup)
    })();
    // SQLite/Registration 都先关闭，再结原预留；失败原 Job 快照仍由外层保留。
    errors.borrow_mut().returned(ErrorPhase::Business, &result);
    errors.borrow_mut().phase = ErrorPhase::Reconcile;
    let reconciled = account.reconcile(&grant, 0).map_err(|e| e.to_string());
    errors
        .borrow_mut()
        .returned(ErrorPhase::Reconcile, &reconciled);
    errors.borrow_mut().phase = ErrorPhase::Deadline;
    let budget = account.check_deadline().map_err(|e| e.to_string());
    errors.borrow_mut().returned(ErrorPhase::Deadline, &budget);
    #[cfg(test)]
    s4_read_trace::snapshot("end", account.snapshot().wire_bytes);
    errors.into_inner().finish().map(|()| delta.get())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    // NOT_RUN：不调用 SQLite。测试真实受限 Budget 方法与 VFS 回调门面的交替借用。
    #[test]
    fn nodes_records_and_wire_share_the_original_job_without_reentrant_borrows() {
        let dir = std::env::temp_dir().join(format!(
            "s1527-budget-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("main");
        std::fs::write(&db, b"test source, no sqlite").unwrap();
        let consumed = super::super::admission_anchor::test_prepare(&db)
            .consume()
            .unwrap();
        let source = MeasuredDomain::measure(&consumed).unwrap();
        let mut started = consumed.into_job(source.existing).unwrap();
        let mut account = started.accounting();
        let before = account.snapshot();
        {
            let errors = RefCell::new(ObservedErrors::new());
            let shared = SharedAccounting {
                account: RefCell::new(&mut account),
                errors: &errors,
                delta: &std::cell::Cell::new(crate::DeltaShape::Current),
            };
            let mut io = &shared;
            io.before_io().unwrap();
            io.after_read(7).unwrap();
            let body = b"node";
            let digest = sc::frame(sc::tag::BYTES_CHUNK, body);
            shared.node(&digest, sc::tag::BYTES_CHUNK, body).unwrap();
            shared.node(&digest, sc::tag::BYTES_CHUNK, body).unwrap();
            shared.record().unwrap();
            shared.record().unwrap();
            io.before_io().unwrap();
            io.after_read(3).unwrap();
            let held = shared.account.borrow_mut();
            assert_eq!(shared.precheck().unwrap_err().code(), "admission_reentrant");
            drop(held);
            shared.precheck().unwrap();
        }
        let after = account.snapshot();
        assert_eq!(after.wire_bytes - before.wire_bytes, 10);
        assert_eq!(after.nodes - before.nodes, 1);
        assert_eq!(after.records - before.records, 2);
        assert_eq!(
            after.node_bytes - before.node_bytes,
            (sc::DOMAIN_SEPARATOR.len() + 9 + 4) as u64
        );
        drop(account);
        drop(started);
        drop(source);
        std::fs::remove_dir_all(dir).unwrap();
    }

    // NOT_RUN：只验证窄错误载体，不执行 SQLite 或恢复。
    #[test]
    fn business_error_precedes_late_deadline_and_keeps_all_cleanup_results() {
        let mut errors = ObservedErrors::new();
        errors.returned(ErrorPhase::Business, &Err("original business".into()));
        for phase in [
            ErrorPhase::Rollback,
            ErrorPhase::Gate,
            ErrorPhase::Close,
            ErrorPhase::Registration,
            ErrorPhase::Reconcile,
            ErrorPhase::Deadline,
        ] {
            errors.phase = phase;
            errors.note(format!("callback {}", phase.name()));
            errors.returned(phase, &Err(format!("returned {}", phase.name())));
        }
        assert_eq!(errors.first.as_deref(), Some("original business"));
        let result = errors.finish().unwrap_err();
        assert!(result.starts_with("original business；"));
        for phase in [
            ErrorPhase::Rollback,
            ErrorPhase::Gate,
            ErrorPhase::Close,
            ErrorPhase::Registration,
            ErrorPhase::Reconcile,
            ErrorPhase::Deadline,
        ] {
            assert!(result.contains(&format!("callback {}", phase.name())));
            assert!(result.contains(&format!("returned {}", phase.name())));
        }
    }

    #[test]
    fn earlier_vfs_budget_error_precedes_sql_wrapper() {
        let mut errors = ObservedErrors::new();
        errors.note("original wire cap".into());
        errors.note("stopped wrapper".into());
        errors.returned(ErrorPhase::Business, &Err("original SQL wrapper".into()));
        assert_eq!(errors.first.as_deref(), Some("original wire cap"));
        let result = errors.finish().unwrap_err();
        assert!(result.starts_with("original wire cap；"));
        assert!(result.contains("original SQL wrapper"));
        assert!(result.contains("stopped wrapper"));
    }

    #[test]
    fn unconstrained_original_error_is_byte_exact_and_success_stays_success() {
        let mut errors = ObservedErrors::new();
        errors.returned(ErrorPhase::Business, &Err("原业务错误：逐字".into()));
        errors.returned(ErrorPhase::Rollback, &Ok(()));
        assert_eq!(errors.finish(), Err("原业务错误：逐字".into()));
        assert_eq!(ObservedErrors::new().finish(), Ok(()));
    }

    // NOT_RUN：用原 Job 的真实 charge 口检查拒收量；不发起 SQLite/业务 I/O。
    #[test]
    fn callback_keeps_first_cause_and_only_counts_actually_unaccepted_wire() {
        let dir = std::env::temp_dir().join(format!(
            "s1527-firsterror-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("main");
        std::fs::write(&db, b"test source, no sqlite").unwrap();
        let consumed = super::super::admission_anchor::test_prepare(&db)
            .consume()
            .unwrap();
        let source = MeasuredDomain::measure(&consumed).unwrap();
        let mut started = consumed.into_job(source.existing).unwrap();
        let mut account = started.accounting();
        let grant = account.reserve_staged(0).unwrap();
        let before = account.snapshot();
        let actual = before.limits.max_total_wire_bytes + 1;
        let errors = RefCell::new(ObservedErrors::new());
        let first;
        {
            let shared = SharedAccounting {
                account: RefCell::new(&mut account),
                errors: &errors,
                delta: &std::cell::Cell::new(crate::DeltaShape::Current),
            };
            let mut io = &shared;
            first = io.after_read(actual).unwrap_err().to_string();
            errors
                .borrow_mut()
                .returned(ErrorPhase::Business, &Err("SQL wrapper".into()));
            errors.borrow_mut().phase = ErrorPhase::Rollback;
            // 模拟已返回的清理观测；不调用 before_io 来授权一次新业务读取。
            io.after_read(3).unwrap_err();
        }
        let after = account.snapshot();
        assert_eq!(after.wire_bytes, before.wire_bytes + u128::from(actual));
        assert_eq!(after.open_reservations, before.open_reservations);
        assert_eq!(after.nodes, before.nodes);
        assert_eq!(after.records, before.records);
        assert_eq!(after.requests, before.requests);
        assert_eq!(errors.borrow().unaccepted_wire_bytes, 3);
        assert_eq!(errors.borrow().first.as_deref(), Some(first.as_str()));
        // 原预留继续按旧规则结清；停止状态和原量不重置。
        account.reconcile(&grant, 0).unwrap();
        assert_eq!(account.snapshot().stopped, after.stopped);
        assert_eq!(account.snapshot().wire_bytes, after.wire_bytes);
        let result = errors.into_inner().finish().unwrap_err();
        assert!(result.starts_with(&first));
        assert!(result.contains("SQL wrapper"));
        assert!(result.contains("未接纳实读wire=3"));
        drop(account);
        drop(started);
        drop(source);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
pub(crate) fn test_progressive_mixed_sql_policy<T>(
    conn: &rusqlite::Connection,
    observer: &dyn super::audit_budget::LegacyObserver,
    run: impl FnOnce(&dyn super::audit_budget::LegacyObserver) -> T,
) -> T {
    mixed_sql::test_progressive_policy(conn, observer, run)
}

pub(crate) fn mixed_fixed_metadata_bytes() -> usize {
    mixed_accounting::fixed_metadata_bytes() + std::mem::size_of::<MixedSealedPrefix>()
}

pub(crate) use mixed::SealedPrefix as MixedSealedPrefix;
pub(crate) fn validate_mixed_prefix(
    source: &MeasuredDomain,
    account: &mut AdmissionAccounting<'_>,
    prefix: &RejectionPrefix<'_>,
) -> Result<MixedSealedPrefix, super::admission_anchor::terminal::FixedFailure> {
    let enabled = false;
    #[cfg(test)]
    let enabled = enabled || MIXED_TEST_ENTRY.with(|value| value.get());
    if !enabled {
        return Err(super::admission_anchor::terminal::FixedFailure::new(
            "MixedAdmissionNotEnabled",
            "business",
            "MixedAdmissionNotEnabled",
        ));
    }
    let sealed = mixed::validate(source, account)?;
    sealed.require(prefix)?;
    Ok(sealed)
}
#[cfg(test)]
thread_local! {static MIXED_TEST_ENTRY:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
#[cfg(test)]
pub(crate) fn test_with_mixed_entry<T>(run: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            MIXED_TEST_ENTRY.with(|v| v.set(false));
        }
    }
    MIXED_TEST_ENTRY.with(|v| assert!(!v.replace(true)));
    let _reset = Reset;
    run()
}
