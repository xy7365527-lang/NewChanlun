//! #1527：私有 SQLite 仅通过固定只读 FD VFS。无源 RW、无默认 VFS 兜底、无文件 clone。
mod fd_wal;
use super::admission_anchor::AdmissionAccounting;
use super::admission_provider::{MeasuredDomain, Medium};
use super::audit_budget::Budget;
use newchan_rust::s_content::{self as sc, job::CheckedFrame, NodeDigest};
use std::cell::RefCell;

/// Budget 的 &self 与 VFS 的回调共享一个受限门面；每次借用在返回前结束，
/// 不跨 SQL prepare/step/rollback/close，也不实现 Access::owner_job。
struct SharedAccounting<'s, 'j> {
    account: RefCell<&'s mut AdmissionAccounting<'j>>,
}
impl Budget for SharedAccounting<'_, '_> {
    fn precheck(&self) -> Result<(), sc::CodecError> {
        self.account
            .try_borrow_mut()
            .map_err(|_| reentrant())?
            .pre_io_check()
    }
    fn prepare_node(&self) -> Result<(), sc::CodecError> {
        self.precheck()?;
        self.account
            .try_borrow_mut()
            .map_err(|_| reentrant())?
            .begin_first_read()
    }
    fn accept_prepared_frame(&self, frame: &CheckedFrame) -> Result<(), sc::CodecError> {
        self.account
            .try_borrow_mut()
            .map_err(|_| reentrant())?
            .accept_checked_usage(frame.usage())
            .map(|_| ())
    }
    fn node(&self, digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.prepare_node()?;
        let usage = sc::job::CheckedNodeUsage::checked(digest, tag, body)?;
        self.account
            .try_borrow_mut()
            .map_err(|_| reentrant())?
            .accept_checked_usage(&usage)
            .map(|_| ())
    }
    fn record(&self) -> Result<(), sc::CodecError> {
        self.account
            .try_borrow_mut()
            .map_err(|_| reentrant())?
            .charge_records(1)
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
        self.account
            .try_borrow_mut()
            .map_err(|_| reentrant())?
            .charge_wire_bytes(actual)
            .map_err(Into::into)
    }
}

fn audit_business(
    conn: &rusqlite::Connection,
    account: &SharedAccounting<'_, '_>,
) -> Result<(), String> {
    account.precheck().map_err(|e| e.to_string())?;
    let deadline = account
        .account
        .try_borrow_mut()
        .map_err(|_| reentrant().to_string())?
        .first_read_deadline()
        .map_err(|e| e.to_string())?;
    // rusqlite 的安全共享借用事务口；conn 的独占生命周期由 Session 保持。
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let result = (|| match crate::meta_get_opt(&tx, "protocol_revision")?.as_deref() {
        Some(super::PROTOCOL_REVISION) => {
            super::state::audit_with_budget(&tx, Some(deadline), None, Some(account)).map(|_| ())
        }
        None | Some(super::PREVIOUS_REVISION) => {
            // 完整原判据原样运行；旧证据图不是 s-content 分帧域，不能伪造 CheckedNodeUsage。
            crate::v2::audit_private_startup(&tx)?;
            Err("LegacyAdmissionBudgetAdapterMissing：完整旧域审计已调用，但旧证据图节点与Snapshot/Delta尚无Job同源费用适配，禁止签发准入".into())
        }
        _ => Err("PrivateAuditProtocolUnsupported".into()),
    })();
    // 显式结束事务后再关闭 connection；原审计错误优先，清理错误不会变成成功。
    // 仅撤下 VM 计算门完成回滚/关闭；VFS 实际 I/O 门仍在，不能再作业务查询。
    conn.progress_handler(0, None::<fn() -> bool>);
    let ended = tx.rollback().map_err(|e| e.to_string());
    let gate = account.precheck().map_err(|e| e.to_string());
    gate.and(result).and(ended)
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

pub(super) fn validate(
    source: &MeasuredDomain,
    account: &mut AdmissionAccounting<'_>,
) -> Result<(), String> {
    account.pre_io_check().map_err(|e| e.to_string())?;
    match source.medium {
        Medium::RollbackJournal => return Err("D2ProductionCapacityMissing：JobLimits8/SResources7没有已裁的overlay payload、metadata与SQLite heap额度映射；禁止采用D2-SMALL六参数默认".into()),
        Medium::MainOnly | Medium::Wal => {}
    }
    // D1 的所有 xWrite/xDelete/xTruncate 都拒绝，未提供 xShmMap，不创建临时磁盘文件。
    // 因而此机制的新增磁盘 allocation_upper 精确为 0，不是未知上界填 0。
    let grant = account.reserve_staged(0).map_err(|e| e.to_string())?;
    let result = (|| {
        let main = source.main_fd()?;
        let wal = source.wal_fd()?;
        let shared = SharedAccounting {
            account: RefCell::new(&mut *account),
        };
        let mut io = &shared;
        let registration = fd_wal::Registration::new(main, wal, &mut io)
            .map_err(|_| "PrivateVfsRegistrationFailed")?;
        let result = (|| {
            let session = registration
                .connect(true)
                .map_err(|_| "PrivateSqliteOpenFailed")?;
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
                audit_business(session.connection(), &shared)
            })();
            let closed = session
                .close()
                .map_err(|_| "PrivateSqliteCloseFailed".to_string());
            check.and(closed)
        })();
        let cleanup = registration
            .finish()
            .map_err(|_| "PrivateVfsCleanupFailed".to_string());
        result.and(cleanup)
    })();
    // SQLite/Registration 都先关闭，再结原预留；失败原 Job 快照仍由外层保留。
    let reconciled = account.reconcile(&grant, 0).map_err(|e| e.to_string());
    let budget = account.check_deadline().map_err(|e| e.to_string());
    budget.and(result).and(reconciled)
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
            let shared = SharedAccounting {
                account: RefCell::new(&mut account),
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
            (sc::DOMAIN_SEPARATOR.len() + 9 + 4) as u128
        );
        drop(account);
        drop(started);
        drop(source);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
