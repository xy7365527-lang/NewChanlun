//! #1419 3C U5：owner 生命周期、夹持观测与连接域核在 owner 连接上实际执行的 SQL 面。rusqlite
//! 连接（及其事务）与收费组件连接各实现一次；判据只有一份，留在调用方。
//!
//! Legacy 实现逐字调用原 rusqlite 方法（同一 SQL、同一取值类型、错误为原错误的 Display 文本），
//! 不新增 Rust 分配；调用方照原样给错误加前缀。
// 连接域核在所有构建使用；夹持观测与提交观察只在生产增量构建（unix）编译。
#![cfg_attr(not(all(feature = "s_incremental", unix)), allow(dead_code))]
use rusqlite::types::ValueRef;
use rusqlite::{Connection, Transaction};
use std::ffi::c_int;

/// 一行的列借用，只在 [`OwnerSql::rows`] 的回调期内有效。
pub(in crate::v3) trait Cells {
    fn len(&self) -> usize;
    fn value(&self, column: usize) -> Result<ValueRef<'_>, String>;
    fn integer(&self, column: usize) -> Result<i64, String>;
}

/// [`OwnerSql::rows`] 的失败：SQL 本身的错误（原错误文本），或回调返回的错误（原样）。
pub(in crate::v3) enum RowsError {
    Sql(String),
    Row(String),
    /// 行宽超过收费组件栈缓冲（准入 schema 至多 8 列，只可能来自外部改动的 schema）。
    #[cfg(all(test, feature = "s_incremental", unix))]
    Shape(String),
}

/// 提交观察回调：rusqlite 连接用 `wal_hook` 形式，收费连接用组件的 `WalFn` 形式。
pub(in crate::v3) struct CommitObserver {
    pub(in crate::v3) legacy: fn(&rusqlite::hooks::Wal, c_int) -> rusqlite::Result<()>,
    #[cfg(all(test, feature = "s_incremental", unix))]
    pub(in crate::v3) charged: newchan_rust::s_content::capture_allocation::sqlite_parent::WalFn,
}

pub(in crate::v3) trait OwnerSql {
    /// 单行首列整数。
    fn integer(&self, sql: &str) -> Result<i64, String>;
    /// 单行首列文本。
    fn text(&self, sql: &str) -> Result<String, String>;
    /// 单行前三列（`PRAGMA wal_checkpoint` 的形状）。
    fn int3(&self, sql: &str) -> Result<(i32, i32, i32), String>;
    /// `PRAGMA main.encoding`：无行为 `None`；UTF-8/UTF-16le/UTF-16be 为 1/2/3，其他为 0。
    fn main_encoding(&self) -> Result<Option<i64>, String>;
    /// 逐行交回调；回调在行借用期内执行，其中不得再执行 SQL。
    fn rows(
        &self,
        sql: &str,
        row: &mut dyn FnMut(&dyn Cells) -> Result<(), String>,
    ) -> Result<(), RowsError>;
    fn total_changes(&self) -> u64;
    /// 登记提交观察回调（替换 SQLite 默认的自动 checkpoint 回调）。
    fn install_commit_observer(&self, observer: &CommitObserver) -> Result<(), String>;
}

fn encoding_code(text: &str) -> i64 {
    match text {
        "UTF-8" => 1,
        "UTF-16le" => 2,
        "UTF-16be" => 3,
        _ => 0,
    }
}

impl Cells for rusqlite::Row<'_> {
    fn len(&self) -> usize {
        self.as_ref().column_count()
    }
    fn value(&self, column: usize) -> Result<ValueRef<'_>, String> {
        self.get_ref(column).map_err(|e| e.to_string())
    }
    fn integer(&self, column: usize) -> Result<i64, String> {
        self.get(column).map_err(|e| e.to_string())
    }
}

impl OwnerSql for Connection {
    fn integer(&self, sql: &str) -> Result<i64, String> {
        self.query_row(sql, [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }
    fn text(&self, sql: &str) -> Result<String, String> {
        self.query_row(sql, [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }
    fn int3(&self, sql: &str) -> Result<(i32, i32, i32), String> {
        self.query_row(sql, [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())
    }
    fn main_encoding(&self) -> Result<Option<i64>, String> {
        let mut stmt =
            newchan_rust::sqlite_checked::CheckedStatement::prepare(self, c"PRAGMA main.encoding")
                .map_err(|e| e.to_string())?;
        let code = match stmt.next().map_err(|e| e.to_string())? {
            None => None,
            Some(mut row) => Some(encoding_code(row.text(0).map_err(|e| e.to_string())?)),
        };
        stmt.finalize().map_err(|e| e.to_string())?;
        Ok(code)
    }
    fn rows(
        &self,
        sql: &str,
        row: &mut dyn FnMut(&dyn Cells) -> Result<(), String>,
    ) -> Result<(), RowsError> {
        let sql_error = |e: rusqlite::Error| RowsError::Sql(e.to_string());
        let mut statement = self.prepare(sql).map_err(sql_error)?;
        let mut rows = statement.query([]).map_err(sql_error)?;
        while let Some(r) = rows.next().map_err(sql_error)? {
            row(r).map_err(RowsError::Row)?;
        }
        Ok(())
    }
    fn total_changes(&self) -> u64 {
        Connection::total_changes(self)
    }
    fn install_commit_observer(&self, observer: &CommitObserver) -> Result<(), String> {
        self.wal_hook(Some(observer.legacy));
        Ok(())
    }
}

impl OwnerSql for Transaction<'_> {
    fn integer(&self, sql: &str) -> Result<i64, String> {
        OwnerSql::integer(&**self, sql)
    }
    fn text(&self, sql: &str) -> Result<String, String> {
        OwnerSql::text(&**self, sql)
    }
    fn int3(&self, sql: &str) -> Result<(i32, i32, i32), String> {
        OwnerSql::int3(&**self, sql)
    }
    fn main_encoding(&self) -> Result<Option<i64>, String> {
        OwnerSql::main_encoding(&**self)
    }
    fn rows(
        &self,
        sql: &str,
        row: &mut dyn FnMut(&dyn Cells) -> Result<(), String>,
    ) -> Result<(), RowsError> {
        OwnerSql::rows(&**self, sql, row)
    }
    fn total_changes(&self) -> u64 {
        Connection::total_changes(self)
    }
    fn install_commit_observer(&self, observer: &CommitObserver) -> Result<(), String> {
        OwnerSql::install_commit_observer(&**self, observer)
    }
}

// #1419 3C U5：收费 owner 一族（SQL 面、后端、连接与守卫）目前只由全新进程 release 锁消费（夹持
// `charged_tests`、生涯门 `step_reader`，两者都是测试编译、unix）。cfg 与消费者精确一致；Native
// 一次启用波接入生产时随生产调用点一起放宽，不靠 allow 掩盖。
#[cfg(all(test, feature = "s_incremental", unix))]
pub(super) use charged::ChargedOwnerSql;

#[cfg(all(test, feature = "s_incremental", unix))]
mod charged {
    use super::*;
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{
        Callback, ChargedConnection, SqlGate, SqliteBinding,
    };
    use std::ops::ControlFlow;

    /// 收费组件连接上的同一 SQL 面。每个操作经 `binding.run` 的逐操作门；首拒之后不再进入
    /// SQLite，首个收费拒绝仍由 [`SqliteBinding::refusal`] 按类型给出。
    #[derive(Clone, Copy)]
    pub(in super::super) struct ChargedOwnerSql<'b> {
        pub(in super::super) binding: &'b SqliteBinding,
        pub(in super::super) connection: &'b ChargedConnection,
    }

    fn gate(error: SqlGate) -> String {
        format!("受管SQLite拒绝：{error:?}")
    }

    struct SliceCells<'r, 'v>(&'r [ValueRef<'v>]);

    impl Cells for SliceCells<'_, '_> {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn value(&self, column: usize) -> Result<ValueRef<'_>, String> {
            self.0
                .get(column)
                .copied()
                .ok_or_else(|| format!("列 {column} 超出行宽"))
        }
        fn integer(&self, column: usize) -> Result<i64, String> {
            self.value(column)?.as_i64().map_err(|e| e.to_string())
        }
    }

    impl ChargedOwnerSql<'_> {
        fn one<T>(
            &self,
            sql: &str,
            f: impl FnOnce(ValueRef<'_>) -> Result<T, String>,
        ) -> Result<T, String> {
            self.binding
                .run(self.connection, |q| q.query_row(sql, [], |[v]| f(v)))
                .map_err(gate)?
        }
    }

    impl OwnerSql for ChargedOwnerSql<'_> {
        fn integer(&self, sql: &str) -> Result<i64, String> {
            self.one(sql, |v| v.as_i64().map_err(|e| e.to_string()))
        }
        fn text(&self, sql: &str) -> Result<String, String> {
            self.one(sql, |v| {
                v.as_str().map(str::to_owned).map_err(|e| e.to_string())
            })
        }
        fn int3(&self, sql: &str) -> Result<(i32, i32, i32), String> {
            let int = |v: ValueRef<'_>| -> Result<i32, String> {
                let n = v.as_i64().map_err(|e| e.to_string())?;
                i32::try_from(n).map_err(|e| e.to_string())
            };
            self.binding
                .run(self.connection, |q| {
                    q.query_row(sql, [], |[a, b, c]| -> Result<(i32, i32, i32), String> {
                        Ok((int(a)?, int(b)?, int(c)?))
                    })
                })
                .map_err(gate)?
        }
        fn main_encoding(&self) -> Result<Option<i64>, String> {
            let found = self.binding.run(self.connection, |q| {
                q.query_row("PRAGMA main.encoding", [], |[v]| {
                    v.as_str().map(encoding_code).map_err(|e| e.to_string())
                })
            });
            match found {
                Ok(code) => code.map(Some),
                Err(SqlGate::Sqlite(rusqlite::ffi::SQLITE_DONE)) => Ok(None),
                Err(e) => Err(gate(e)),
            }
        }
        fn rows(
            &self,
            sql: &str,
            row: &mut dyn FnMut(&dyn Cells) -> Result<(), String>,
        ) -> Result<(), RowsError> {
            let mut failed = None;
            self.binding
                .run(self.connection, |q| {
                    q.prepare(sql)?
                        .for_each_cells([], |cells| match row(&SliceCells(cells)) {
                            Ok(()) => ControlFlow::Continue(()),
                            Err(e) => {
                                failed = Some(e);
                                ControlFlow::Break(())
                            }
                        })
                })
                .map_err(|e| match e {
                    SqlGate::Sqlite(rusqlite::ffi::SQLITE_RANGE) => RowsError::Shape(gate(e)),
                    e => RowsError::Sql(gate(e)),
                })?;
            match failed {
                Some(e) => Err(RowsError::Row(e)),
                None => Ok(()),
            }
        }
        fn total_changes(&self) -> u64 {
            self.connection.total_changes()
        }
        fn install_commit_observer(&self, observer: &CommitObserver) -> Result<(), String> {
            let hook = Callback {
                callback: observer.charged,
                context: std::ptr::null_mut(),
            };
            // SAFETY: the observer keeps its state in a thread-local, not in the context.
            self.binding
                .run(self.connection, |q| unsafe { q.set_wal_hook(Some(hook)) })
                .map_err(gate)
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

// 可见性与 `AuditConnection`（v3 内消费）一致：默认后端 `LegacyOwner` 及其 trait 接口进入 v3 侧的
// 类型 `AuditConnection` 时不得比它更窄。
/// effects 节点 journal 的连接键：rusqlite 连接用句柄地址（原口径），收费连接用不复用的
/// [`ConnectionId`](newchan_rust::s_content::capture_allocation::sqlite_parent::ConnectionId)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::v3) enum JournalKey {
    Handle(usize),
    #[cfg(all(test, feature = "s_incremental", unix))]
    Charged(newchan_rust::s_content::capture_allocation::sqlite_parent::ConnectionId),
}

/// #1419 3C U5：owner 连接家族的后端（sealed，同台账 `Backend` 写法）。`AuditConnection` 与
/// `AuditTxn` 的生命周期判据只有一份，经本 trait 落到 rusqlite 或收费组件连接。
pub(in crate::v3) trait OwnerBackend: sealed::Sealed + Sized + 'static {
    type Conn;
    type Txn<'c>;
    fn conn_autocommit(conn: &Self::Conn) -> bool;
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "#1419 3C U5：只有生涯门（测试编译）按连接取 SQL 面"
        )
    )]
    fn conn_sql(conn: &Self::Conn) -> &dyn OwnerSql;
    /// 开事务；错误为原错误文本。
    fn begin(
        conn: &Self::Conn,
        behavior: rusqlite::TransactionBehavior,
    ) -> Result<Self::Txn<'_>, String>;
    fn txn_sql<'t>(txn: &'t Self::Txn<'_>) -> &'t dyn OwnerSql;
    /// 收尾前释放预备语句缓存（rusqlite 的缓存；组件不缓存语句）。
    fn flush_statements(conn: &Self::Conn);
    /// 回滚并结束守卫。
    fn rollback(txn: Self::Txn<'_>) -> Result<(), String>;
    /// COMMIT 并结束守卫；失败时不隐式回滚，由调用方核 autocommit 并经 [`Self::rollback_conn`] 收尾。
    fn commit(txn: Self::Txn<'_>) -> Result<(), String>;
    /// 守卫已结束而连接仍在事务内时的显式回滚（收尾）。
    fn rollback_conn(conn: &Self::Conn) -> Result<(), String>;
    /// 撤销本事务上登记的写回调（收尾，首拒之后仍可执行）。
    fn clear_write_hooks(txn: &Self::Txn<'_>) -> Result<(), String>;
    fn journal_key(txn: &Self::Txn<'_>) -> JournalKey;
    /// 连接自身初始化失败时的正常关闭（见 `h1_close_after_failure`）。
    fn close_after_failure<T>(conn: &Self::Conn, result: Result<T, String>) -> Result<T, String>;
}

/// 现行 owner：私有 rusqlite 连接。只作类型参数，不构造。
pub(in crate::v3) enum LegacyOwner {}
impl sealed::Sealed for LegacyOwner {}

impl OwnerBackend for LegacyOwner {
    type Conn = Connection;
    type Txn<'c> = Transaction<'c>;
    fn conn_autocommit(conn: &Connection) -> bool {
        conn.is_autocommit()
    }
    fn conn_sql(conn: &Connection) -> &dyn OwnerSql {
        conn
    }
    fn begin(
        conn: &Connection,
        behavior: rusqlite::TransactionBehavior,
    ) -> Result<Transaction<'_>, String> {
        Transaction::new_unchecked(conn, behavior).map_err(|e| e.to_string())
    }
    fn txn_sql<'t>(txn: &'t Transaction<'_>) -> &'t dyn OwnerSql {
        txn
    }
    fn flush_statements(conn: &Connection) {
        conn.flush_prepared_statement_cache();
    }
    fn rollback(txn: Transaction<'_>) -> Result<(), String> {
        txn.rollback().map_err(|e| e.to_string())
    }
    fn commit(mut txn: Transaction<'_>) -> Result<(), String> {
        txn.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        txn.commit().map_err(|e| e.to_string())
    }
    fn rollback_conn(conn: &Connection) -> Result<(), String> {
        conn.execute_batch("ROLLBACK").map_err(|e| e.to_string())
    }
    fn clear_write_hooks(txn: &Transaction<'_>) -> Result<(), String> {
        txn.update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
            .map_err(|e| e.to_string())
    }
    fn journal_key(txn: &Transaction<'_>) -> JournalKey {
        // SAFETY: the handle is only compared as an address, never dereferenced.
        JournalKey::Handle(unsafe { txn.handle() } as usize)
    }
    fn close_after_failure<T>(conn: &Connection, result: Result<T, String>) -> Result<T, String> {
        #[cfg(all(feature = "s_incremental", unix))]
        return crate::h1_close_after_failure(conn, result, false);
        #[cfg(not(all(feature = "s_incremental", unix)))]
        {
            let _ = conn;
            result
        }
    }
}

#[cfg(all(test, feature = "s_incremental", unix))]
pub(crate) use charged_owner::{ChargedOwner, ChargedOwnerConn, ChargedOwnerTxn};

#[cfg(all(test, feature = "s_incremental", unix))]
mod charged_owner {
    use super::*;
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{
        ChargedConnection, SqlGate, SqlOps, SqliteBinding,
    };
    use std::rc::Rc;

    /// 收费组件连接上的 owner（同父、逐操作门、首拒后只有窄收尾）。生产 Native 一次启用前只由
    /// 全新进程 release 锁消费。
    pub(crate) enum ChargedOwner {}
    impl sealed::Sealed for ChargedOwner {}

    pub(crate) struct ChargedOwnerConn {
        pub(crate) binding: Rc<SqliteBinding>,
        pub(crate) connection: ChargedConnection,
    }

    impl ChargedOwnerConn {
        fn sql(&self) -> ChargedOwnerSql<'_> {
            ChargedOwnerSql {
                binding: &self.binding,
                connection: &self.connection,
            }
        }
    }

    impl OwnerSql for ChargedOwnerConn {
        fn integer(&self, sql: &str) -> Result<i64, String> {
            self.sql().integer(sql)
        }
        fn text(&self, sql: &str) -> Result<String, String> {
            self.sql().text(sql)
        }
        fn int3(&self, sql: &str) -> Result<(i32, i32, i32), String> {
            self.sql().int3(sql)
        }
        fn main_encoding(&self) -> Result<Option<i64>, String> {
            self.sql().main_encoding()
        }
        fn rows(
            &self,
            sql: &str,
            row: &mut dyn FnMut(&dyn Cells) -> Result<(), String>,
        ) -> Result<(), RowsError> {
            self.sql().rows(sql, row)
        }
        fn total_changes(&self) -> u64 {
            self.connection.total_changes()
        }
        fn install_commit_observer(&self, observer: &CommitObserver) -> Result<(), String> {
            self.sql().install_commit_observer(observer)
        }
    }

    /// 事务守卫：`BEGIN …` 之后持有连接借用；提交与回滚都消费守卫。
    pub(crate) struct ChargedOwnerTxn<'c> {
        pub(crate) conn: &'c ChargedOwnerConn,
    }

    // #1419 3C effects S1：effects 与 `TxStore` 在守卫所在连接上的操作面（见 `owner_tx`）。
    impl<'c> ChargedOwnerTxn<'c> {
        /// 守卫所在连接上的一次普通操作：经组件逐操作门，首拒之后不进入 SQLite。
        pub(crate) fn run<R>(
            &self,
            f: impl FnOnce(&SqlOps<'_>) -> Result<R, SqlGate>,
        ) -> Result<R, SqlGate> {
            self.conn.binding.run(&self.conn.connection, f)
        }
        pub(crate) fn connection(&self) -> &'c ChargedConnection {
            &self.conn.connection
        }
        pub(crate) fn binding(&self) -> &'c SqliteBinding {
            &self.conn.binding
        }
    }

    fn gate(error: SqlGate) -> String {
        format!("受管SQLite拒绝：{error:?}")
    }

    impl OwnerBackend for ChargedOwner {
        type Conn = ChargedOwnerConn;
        type Txn<'c> = ChargedOwnerTxn<'c>;
        fn conn_autocommit(conn: &ChargedOwnerConn) -> bool {
            conn.connection.is_autocommit()
        }
        fn conn_sql(conn: &ChargedOwnerConn) -> &dyn OwnerSql {
            conn
        }
        fn begin(
            conn: &ChargedOwnerConn,
            behavior: rusqlite::TransactionBehavior,
        ) -> Result<ChargedOwnerTxn<'_>, String> {
            let sql = match behavior {
                rusqlite::TransactionBehavior::Deferred => "BEGIN DEFERRED",
                rusqlite::TransactionBehavior::Immediate => "BEGIN IMMEDIATE",
                rusqlite::TransactionBehavior::Exclusive => "BEGIN EXCLUSIVE",
                _ => return Err("未支持的事务方式".into()),
            };
            conn.binding
                .run(&conn.connection, |q| q.execute_batch(sql))
                .map_err(gate)?;
            Ok(ChargedOwnerTxn { conn })
        }
        fn txn_sql<'t>(txn: &'t ChargedOwnerTxn<'_>) -> &'t dyn OwnerSql {
            txn.conn
        }
        fn flush_statements(_: &ChargedOwnerConn) {}
        fn rollback(txn: ChargedOwnerTxn<'_>) -> Result<(), String> {
            txn.conn
                .binding
                .rollback(&txn.conn.connection)
                .map_err(gate)
        }
        fn commit(txn: ChargedOwnerTxn<'_>) -> Result<(), String> {
            txn.conn
                .binding
                .run(&txn.conn.connection, |q| q.execute_batch("COMMIT"))
                .map_err(gate)
        }
        fn rollback_conn(conn: &ChargedOwnerConn) -> Result<(), String> {
            conn.binding.rollback(&conn.connection).map_err(gate)
        }
        fn clear_write_hooks(txn: &ChargedOwnerTxn<'_>) -> Result<(), String> {
            txn.conn
                .binding
                .clear_write_hooks(&txn.conn.connection)
                .map_err(gate)
        }
        fn journal_key(txn: &ChargedOwnerTxn<'_>) -> JournalKey {
            JournalKey::Charged(txn.conn.connection.id())
        }
        fn close_after_failure<T>(
            conn: &ChargedOwnerConn,
            result: Result<T, String>,
        ) -> Result<T, String> {
            crate::h1_close_after_failure(
                &crate::v3::h1_writer_sql::ChargedWriter {
                    binding: &conn.binding,
                    connection: &conn.connection,
                },
                result,
                false,
            )
        }
    }
}
