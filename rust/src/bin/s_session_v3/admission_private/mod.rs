//! #1527：私有 SQLite 仅通过固定只读 FD VFS。无源 RW、无默认 VFS 兜底、无文件 clone。
mod fd_wal;
use super::admission_anchor::AdmissionAccounting;
use super::admission_provider::{MeasuredDomain, Medium};

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
        let registration = fd_wal::Registration::new(main, wal, account)
            .map_err(|_| "PrivateVfsRegistrationFailed")?;
        let result = (|| {
            let session = registration
                .connect(true)
                .map_err(|_| "PrivateSqliteOpenFailed")?;
            let check = (|| {
                let mut stmt = session
                    .connection()
                    .prepare("PRAGMA quick_check")
                    .map_err(|_| "PrivateSqliteCheckFailed")?;
                let mut rows = stmt.query([]).map_err(|_| "PrivateSqliteCheckFailed")?;
                let mut seen = false;
                while let Some(row) = rows.next().map_err(|_| "PrivateSqliteCheckFailed")? {
                    let text: String = row.get(0).map_err(|_| "PrivateSqliteCheckFailed")?;
                    if text != "ok" || seen {
                        return Err("PrivateSqliteIntegrityRejected");
                    }
                    seen = true;
                }
                if !seen {
                    return Err("PrivateSqliteIntegrityEmpty");
                }
                Ok(())
            })();
            let closed = session.close().map_err(|_| "PrivateSqliteCloseFailed");
            check.and(closed)
        })();
        let cleanup = registration.finish().map_err(|_| "PrivateVfsCleanupFailed");
        result.and(cleanup).map_err(str::to_string)
    })();
    // SQLite/Registration 都先关闭，再结原预留；失败原 Job 快照仍由外层保留。
    let reconciled = account.reconcile(&grant, 0).map_err(|e| e.to_string());
    let budget = account.check_deadline().map_err(|e| e.to_string());
    budget.and(result).and(reconciled)
}
