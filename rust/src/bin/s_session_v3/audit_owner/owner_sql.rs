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
pub(super) trait Cells {
    fn len(&self) -> usize;
    fn value(&self, column: usize) -> Result<ValueRef<'_>, String>;
    fn integer(&self, column: usize) -> Result<i64, String>;
}

/// [`OwnerSql::rows`] 的失败：SQL 本身的错误（原错误文本），或回调返回的错误（原样）。
pub(super) enum RowsError {
    Sql(String),
    Row(String),
}

/// 提交观察回调：rusqlite 连接用 `wal_hook` 形式，收费连接用组件的 `WalFn` 形式。
pub(super) struct CommitObserver {
    pub(super) legacy: fn(&rusqlite::hooks::Wal, c_int) -> rusqlite::Result<()>,
    #[cfg(feature = "s_incremental")]
    pub(super) charged: newchan_rust::s_content::capture_allocation::sqlite_parent::WalFn,
}

pub(super) trait OwnerSql {
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

#[cfg(feature = "s_incremental")]
pub(super) use charged::ChargedOwnerSql;

#[cfg(feature = "s_incremental")]
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
                .map_err(|e| RowsError::Sql(gate(e)))?;
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
