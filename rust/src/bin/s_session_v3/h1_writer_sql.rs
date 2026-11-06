//! #1419 3C：H1 写者交接连接（写者主库家族）实际使用的 SQL 面。原 rusqlite 连接与收费组件
//! 连接各实现一次；交接开库配置、正常关闭与 WAL 转换等生产函数对两者共用同一份代码。
use newchan_rust::s_content::capture_allocation::sqlite_parent::{
    ChargedConnection, DbFlag, OpenSpec, SqlGate, SqliteBinding,
};
use rusqlite::types::ValueRef;
use rusqlite::{config::DbConfig, ffi, Connection};
use std::fmt;
use std::path::Path;

pub(crate) trait H1WriterSql {
    type Error: fmt::Display;
    fn text(&self, sql: &str) -> Result<String, Self::Error>;
    fn integer(&self, sql: &str) -> Result<i64, Self::Error>;
    fn flag(&self, sql: &str) -> Result<bool, Self::Error>;
    fn batch(&self, sql: &str) -> Result<(), Self::Error>;
    /// 置 NO_CKPT_ON_CLOSE=1（交接期间关闭不检查点），返回 SQLite 读回值。
    fn disable_close_checkpoint(&self) -> Result<bool, Self::Error>;
    /// 收尾：置 NO_CKPT_ON_CLOSE=0（正常关闭），返回 SQLite 读回值。收费连接上首拒之后仍可执行。
    fn restore_close_checkpoint(&self) -> Result<bool, Self::Error>;
    /// 有未结束事务时回滚，没有时什么都不做。
    fn rollback_open(&self) -> Result<(), Self::Error>;
}

impl H1WriterSql for Connection {
    type Error = rusqlite::Error;
    fn text(&self, sql: &str) -> Result<String, Self::Error> {
        self.query_row(sql, [], |row| row.get(0))
    }
    fn integer(&self, sql: &str) -> Result<i64, Self::Error> {
        self.query_row(sql, [], |row| row.get(0))
    }
    fn flag(&self, sql: &str) -> Result<bool, Self::Error> {
        self.query_row(sql, [], |row| row.get(0))
    }
    fn batch(&self, sql: &str) -> Result<(), Self::Error> {
        self.execute_batch(sql)
    }
    fn disable_close_checkpoint(&self) -> Result<bool, Self::Error> {
        self.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)
    }
    fn restore_close_checkpoint(&self) -> Result<bool, Self::Error> {
        self.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, false)
    }
    fn rollback_open(&self) -> Result<(), Self::Error> {
        if self.is_autocommit() {
            Ok(())
        } else {
            self.execute_batch("ROLLBACK")
        }
    }
}

/// 原 `rusqlite::Transaction::new_unchecked` 的同语义守卫：`BEGIN …` 成功后，未提交或提交失败
/// 时在析构中回滚（忽略回滚自身的错误），显式回滚失败同样留给析构再试一次。
pub(crate) struct H1Txn<'s, S: H1WriterSql> {
    conn: &'s S,
    open: bool,
}

impl<'s, S: H1WriterSql> H1Txn<'s, S> {
    pub(crate) fn begin(conn: &'s S, sql: &str) -> Result<Self, S::Error> {
        conn.batch(sql)?;
        Ok(Self { conn, open: true })
    }
    pub(crate) fn commit(mut self) -> Result<(), S::Error> {
        self.conn.batch("COMMIT")?;
        self.open = false;
        Ok(())
    }
    pub(crate) fn rollback(mut self) -> Result<(), S::Error> {
        self.conn.batch("ROLLBACK")?;
        self.open = false;
        Ok(())
    }
}

impl<S: H1WriterSql> Drop for H1Txn<'_, S> {
    fn drop(&mut self) {
        if self.open {
            let _ = self.conn.rollback_open();
        }
    }
}

/// 收费组件连接上的拒绝；首个收费拒绝仍由 [`SqliteBinding::refusal`] 按类型给出。
#[allow(
    dead_code,
    reason = "#1419 3C：受管写者交接连接只由全新进程 release 锁消费，Native 一次启用波接入生产"
)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Refused(pub(crate) SqlGate);

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "受管SQLite拒绝：{:?}", self.0)
    }
}

#[allow(
    dead_code,
    reason = "#1419 3C：受管写者交接连接只由全新进程 release 锁消费，Native 一次启用波接入生产"
)]
pub(crate) struct ChargedWriter<'b> {
    pub(crate) binding: &'b SqliteBinding,
    pub(crate) connection: &'b ChargedConnection,
}

#[allow(
    dead_code,
    reason = "#1419 3C：受管写者交接连接只由全新进程 release 锁消费，Native 一次启用波接入生产"
)]
impl ChargedWriter<'_> {
    fn row<T>(&self, sql: &str, f: impl FnOnce(ValueRef<'_>) -> Option<T>) -> Result<T, Refused> {
        self.binding
            .run(self.connection, |q| q.query_row(sql, [], |[v]| f(v)))
            .map_err(Refused)?
            .ok_or(Refused(SqlGate::Sqlite(ffi::SQLITE_MISMATCH)))
    }
}

impl H1WriterSql for ChargedWriter<'_> {
    type Error = Refused;
    fn text(&self, sql: &str) -> Result<String, Self::Error> {
        self.row(sql, |v| v.as_str().ok().map(str::to_owned))
    }
    fn integer(&self, sql: &str) -> Result<i64, Self::Error> {
        self.row(sql, |v| v.as_i64().ok())
    }
    fn flag(&self, sql: &str) -> Result<bool, Self::Error> {
        self.row(sql, |v| v.as_i64().ok().map(|n| n != 0))
    }
    fn batch(&self, sql: &str) -> Result<(), Self::Error> {
        self.binding
            .run(self.connection, |q| q.execute_batch(sql))
            .map_err(Refused)
    }
    fn disable_close_checkpoint(&self) -> Result<bool, Self::Error> {
        self.binding
            .run(self.connection, |q| {
                q.set_db_flag(DbFlag::NoCheckpointOnClose, true)
            })
            .map_err(Refused)
    }
    fn restore_close_checkpoint(&self) -> Result<bool, Self::Error> {
        self.binding
            .restore_close_checkpoint(self.connection)
            .map_err(Refused)
    }
    fn rollback_open(&self) -> Result<(), Self::Error> {
        self.binding.rollback(self.connection).map_err(Refused)
    }
}

/// 原 `Connection::open_with_flags(db, OpenFlags::default() | PRIVATE_CACHE)` 的受管等价：
/// rusqlite 默认标志（读写、可创建、URI、无互斥）加私有缓存，及 rusqlite 开库时设置的 5 秒 busy 超时。
#[allow(
    dead_code,
    reason = "#1419 3C：受管写者交接连接只由全新进程 release 锁消费，Native 一次启用波接入生产"
)]
pub(crate) fn open_charged(
    binding: &SqliteBinding,
    db: &Path,
) -> Result<ChargedConnection, String> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(db.as_os_str().as_bytes())
        .map_err(|e| format!("打开 SQLite 失败：{e}"))?;
    let spec = OpenSpec {
        flags: ffi::SQLITE_OPEN_READWRITE
            | ffi::SQLITE_OPEN_CREATE
            | ffi::SQLITE_OPEN_URI
            | ffi::SQLITE_OPEN_NOMUTEX
            | ffi::SQLITE_OPEN_PRIVATECACHE,
        busy_timeout_ms: Some(5000),
        ..OpenSpec::default()
    };
    // SAFETY: the spec carries no caller callbacks, contexts or custom VFS.
    unsafe { binding.open_with(&path, &spec) }
        .map_err(|e| format!("打开 SQLite 失败：{}", Refused(e)))
}

/// #1419 3C：写者交接连接的真实生产函数（开库配置、WAL 转换、正常关闭）经收费组件运行。
/// allocator 只能在本进程首次 SQLite 初始化前安装，须全新进程 `--release --ignored --exact`
/// 单独运行（CI 经 `.github/1488-uo-release-locks.txt` 逐条执行）。
#[cfg(test)]
mod charged_tests {
    use super::super::audit_owner::native::Budget;
    use super::*;
    use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
    use newchan_rust::s_content::capture_allocation::sqlite_parent::SqliteParentAllocator;
    use std::cell::Cell;
    use std::ffi::c_int;

    const LIMIT: usize = 64 << 20;

    fn no_checkpoint_on_close(w: &ChargedWriter<'_>) -> c_int {
        w.binding
            .run(w.connection, |q| {
                let mut out: c_int = -1;
                // SAFETY: read-only db_config query (-1) on the live handle inside this run.
                let rc = unsafe {
                    ffi::sqlite3_db_config(
                        q.handle(),
                        ffi::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
                        -1 as c_int,
                        &mut out as *mut c_int,
                    )
                };
                assert_eq!(rc, ffi::SQLITE_OK);
                Ok(out)
            })
            .unwrap()
    }

    fn seed(binding: &SqliteBinding, db: &Path) {
        let conn = open_charged(binding, db).unwrap();
        binding
            .run(&conn, |q| {
                q.execute_batch(
                    "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT);
                     INSERT INTO meta VALUES('rule_revision', 'r');",
                )
            })
            .unwrap();
        binding.close(conn).map_err(|(_, e)| e).unwrap();
    }

    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn h1_writer_handoff_charged_keeps_original_config_conversion_and_close() {
        // SAFETY: this test runs alone in a fresh process before any SQLite use.
        let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
        let root = Budget::fresh(LIMIT).unwrap();
        let mut current = root.bind_frozen(1, LIMIT, 0, 0).unwrap();
        let bind = |budget: &Budget| {
            let (parent, origin) = budget.sqlite_parent();
            allocator.bind(parent, origin).expect("bind")
        };
        let dir = std::env::temp_dir().join(format!("h1-writer-charged-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let converted_db = dir.join("converted.sqlite3");
        let refused_db = dir.join("refused.sqlite3");
        let wal = |db: &Path| {
            let mut name = db.as_os_str().to_os_string();
            name.push("-wal");
            std::path::PathBuf::from(name)
        };
        let wide = bind(&current);
        seed(&wide, &converted_db);
        seed(&wide, &refused_db);

        let base = current.live();
        let conn = crate::open_db_after_h1_certification_charged(&wide, &converted_db).unwrap();
        let w = ChargedWriter {
            binding: &wide,
            connection: &conn,
        };
        assert_eq!(
            w.integer("PRAGMA synchronous").unwrap(),
            2,
            "synchronous=FULL"
        );
        #[cfg(target_os = "macos")]
        assert_eq!(w.integer("PRAGMA fullfsync").unwrap(), 1);
        assert_eq!(
            no_checkpoint_on_close(&w),
            1,
            "handoff disables close checkpoint"
        );
        assert!(
            current.live() > base,
            "connection bytes charged to the parent"
        );

        let integrity = Cell::new(false);
        let calls = Cell::new(0);
        let token = crate::prepare_h1_wal_handoff(
            &w,
            || {
                calls.set(calls.get() + 1);
                Ok(())
            },
            || {
                calls.set(calls.get() + 10);
                Ok(7_u8)
            },
            &integrity,
        )
        .unwrap();
        assert_eq!((token, calls.get(), integrity.get()), (Some(7), 11, false));
        assert!(w
            .text("PRAGMA main.journal_mode")
            .unwrap()
            .eq_ignore_ascii_case("wal"));
        assert_eq!(
            crate::prepare_h1_wal_handoff(
                &w,
                || -> Result<(), String> { panic!("WAL input must not authenticate again") },
                || -> Result<u8, String> { panic!("WAL input must not snapshot again") },
                &integrity,
            ),
            Ok(None)
        );
        crate::h1_normal_close(&w).unwrap();
        assert_eq!(
            no_checkpoint_on_close(&w),
            0,
            "normal close restores the checkpoint"
        );
        wide.close(conn).map_err(|(_, e)| e).unwrap();
        assert!(
            !wal(&converted_db).exists(),
            "normal close checkpointed and removed the WAL"
        );
        assert_eq!(current.live(), base, "all connection bytes returned");

        let conn = crate::open_db_after_h1_certification_charged(&wide, &refused_db).unwrap();
        let w = ChargedWriter {
            binding: &wide,
            connection: &conn,
        };
        let integrity = Cell::new(false);
        let refused = crate::prepare_h1_wal_handoff(
            &w,
            || Err("SourceIdentityChanged".to_string()),
            || -> Result<u8, String> { panic!("refused input must not snapshot") },
            &integrity,
        );
        assert_eq!(refused, Err("SourceIdentityChanged".to_string()));
        assert!(integrity.get(), "source refusal is an integrity refusal");
        assert!(w
            .text("PRAGMA main.journal_mode")
            .unwrap()
            .eq_ignore_ascii_case("delete"));
        assert_eq!(
            w.integer("SELECT COUNT(*) FROM main.meta").unwrap(),
            1,
            "exclusive transaction rolled back, source unchanged"
        );
        assert_eq!(
            no_checkpoint_on_close(&w),
            1,
            "integrity refusal keeps the evidence mode"
        );
        let failed = crate::prepare_h1_wal_handoff(
            &w,
            || Err("temporary".to_string()),
            || -> Result<u8, String> { panic!("failed input must not snapshot") },
            &integrity,
        );
        assert_eq!(failed, Err("temporary".to_string()));
        assert_eq!(
            no_checkpoint_on_close(&w),
            0,
            "own failure restores normal close"
        );
        wide.close(conn).map_err(|(_, e)| e).unwrap();
        assert_eq!(current.live(), base);

        drop(wide);
        current = current.handoff(2, 0).expect("zero-allowance frozen domain");
        let tight = bind(&current);
        let base = current.live();
        let error = crate::open_db_after_h1_certification_charged(&tight, &converted_db)
            .err()
            .expect("open refused in a zero-allowance domain");
        assert!(error.contains("Refused(Budget)"), "{error}");
        assert_eq!(tight.refusal(), Some(ScratchFailure::Budget));
        assert_eq!(current.live(), base, "refused open leaves no charged bytes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// dc5 独审 F1：开库并已禁关闭检查点之后，业务中途遇到收费首拒；收尾仍回滚事务、经窄收尾
    /// 恢复关闭检查点并正常关闭，首拒保留，字节返还。
    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn h1_writer_budget_refusal_after_open_still_restores_normal_close() {
        // SAFETY: this test runs alone in a fresh process before any SQLite use.
        let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
        let root = Budget::fresh(LIMIT).unwrap();
        let current = root.bind_frozen(1, LIMIT, 0, 0).unwrap();
        let (parent, origin) = current.sqlite_parent();
        let binding = allocator.bind(parent, origin).expect("bind");
        let dir = std::env::temp_dir().join(format!("h1-writer-refusal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("writer.sqlite3");
        seed(&binding, &db);

        let base = current.live();
        let conn = crate::open_db_after_h1_certification_charged(&binding, &db).unwrap();
        let w = ChargedWriter {
            binding: &binding,
            connection: &conn,
        };
        assert_eq!(
            no_checkpoint_on_close(&w),
            1,
            "handoff disabled close checkpoint"
        );
        let integrity = Cell::new(false);
        let in_transaction = Cell::new(false);
        let result = crate::prepare_h1_wal_handoff(
            &w,
            || Ok(()),
            || -> Result<u8, String> {
                in_transaction.set(!conn.is_autocommit());
                w.integer("SELECT length(randomblob(134217728))")
                    .map_err(|e| e.to_string())?;
                Ok(0)
            },
            &integrity,
        );
        let budget = Refused(SqlGate::Refused(ScratchFailure::Budget)).to_string();
        assert_eq!(
            result,
            Err(budget),
            "original error, normal close appended nothing"
        );
        assert!(
            in_transaction.get(),
            "the failure happened inside the handoff transaction"
        );
        assert!(
            !integrity.get(),
            "own resource failure is not an integrity refusal"
        );
        assert!(conn.is_autocommit(), "handoff transaction rolled back");
        assert_eq!(binding.refusal(), Some(ScratchFailure::Budget));
        binding.close(conn).map_err(|(_, e)| e).unwrap();
        assert!(
            !wal_path(&db).exists(),
            "close checkpoint restored: normal close checkpointed and removed the WAL"
        );
        assert_eq!(
            binding.refusal(),
            Some(ScratchFailure::Budget),
            "first refusal kept"
        );
        assert_eq!(current.live(), base, "all connection bytes returned");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn wal_path(db: &Path) -> std::path::PathBuf {
        let mut name = db.as_os_str().to_os_string();
        name.push("-wal");
        std::path::PathBuf::from(name)
    }
}
