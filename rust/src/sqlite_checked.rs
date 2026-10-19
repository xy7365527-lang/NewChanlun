//! #1394：真实只读 SELECT 的 checked FFI 边界；不授予完整 SQL 分配资格。
//! 文本转换先检查 SQLite NOMEM，再开放借用；业务复制及预算仍由调用者持有。
use rusqlite::{ffi, types::Type, Connection, Error, Result};
use std::{ffi::CStr, marker::PhantomData, ptr::NonNull};

fn sql_error(code: i32) -> Error {
    Error::SqliteFailure(ffi::Error::new(code), None)
}
fn check(code: i32) -> Result<()> {
    if code == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(sql_error(code))
    }
}

/// 唯一拥有 statement 的只读连接借用。不会从连接枚举或猜测别人的 statement。
pub struct CheckedStatement<'conn> {
    _connection: &'conn Connection,
    statement: NonNull<ffi::sqlite3_stmt>,
    finished: bool,
    readonly: bool,
}
impl<'conn> CheckedStatement<'conn> {
    /// 借用静态 CStr 或调用者已有的 CStr；返回后无需保留 SQL 借用，连接必须仍活着。
    pub fn prepare(connection: &'conn Connection, sql: &CStr) -> Result<Self> {
        Self::prepare_with_mode(connection, sql, true)
    }

    /// 仅供调用方固定副本 INSERT；来源 SQL 仍须使用只读 prepare。
    /// 不允许另一个语句或 RETURNING 行借只读 next 执行写入。
    pub fn prepare_insert(connection: &'conn Connection, sql: &CStr) -> Result<Self> {
        if !sql.to_bytes().starts_with(b"INSERT INTO ") {
            return Err(sql_error(ffi::SQLITE_MISUSE));
        }
        Self::prepare_with_mode(connection, sql, false)
    }

    fn prepare_with_mode(
        connection: &'conn Connection,
        sql: &CStr,
        readonly: bool,
    ) -> Result<Self> {
        let mut statement = std::ptr::null_mut();
        let mut tail = std::ptr::null();
        // SAFETY: borrowed Connection is live; sql has a terminating NUL, out pointers valid.
        let code = unsafe {
            ffi::sqlite3_prepare_v2(
                connection.handle(),
                sql.as_ptr(),
                -1,
                &mut statement,
                &mut tail,
            )
        };
        if code != ffi::SQLITE_OK {
            if !statement.is_null() {
                unsafe {
                    ffi::sqlite3_finalize(statement);
                }
            }
            return Err(sql_error(code));
        }
        let statement = NonNull::new(statement).ok_or_else(|| sql_error(ffi::SQLITE_MISUSE))?;
        let out = Self {
            _connection: connection,
            statement,
            finished: false,
            readonly,
        };
        if (unsafe { ffi::sqlite3_stmt_readonly(statement.as_ptr()) } != 0) != readonly {
            return Err(sql_error(ffi::SQLITE_READONLY));
        }
        // Like rusqlite::prepare, permit whitespace/comments, reject a second statement.
        if !tail.is_null() && !unsafe { CStr::from_ptr(tail) }.to_bytes().is_empty() {
            let mut extra = std::ptr::null_mut();
            let code = unsafe {
                ffi::sqlite3_prepare_v2(
                    connection.handle(),
                    tail,
                    -1,
                    &mut extra,
                    std::ptr::null_mut(),
                )
            };
            let multiple = !extra.is_null();
            if multiple {
                unsafe {
                    ffi::sqlite3_finalize(extra);
                }
            }
            check(code)?;
            if multiple {
                return Err(Error::MultipleStatement);
            }
        }
        Ok(out)
    }
    /// 元数据借用也只属于本 statement；空结果仍保留完整列名。
    pub fn column_count(&self) -> usize {
        unsafe { ffi::sqlite3_column_count(self.statement.as_ptr()) as usize }
    }
    pub fn column_name(&mut self, column: usize) -> Result<&str> {
        if column >= self.column_count() {
            return Err(Error::InvalidColumnIndex(column));
        }
        let pointer = unsafe { ffi::sqlite3_column_name(self.statement.as_ptr(), column as i32) };
        let code = unsafe { ffi::sqlite3_errcode(self._connection.handle()) };
        // UTF-8列名转换可能分配；NOMEM检查先于指针借用。
        if code & 255 == ffi::SQLITE_NOMEM {
            return Err(sql_error(code));
        }
        if pointer.is_null() {
            return Err(sql_error(ffi::SQLITE_MISUSE));
        }
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .map_err(|cause| Error::Utf8Error(column, cause))
    }
    /// 参数索引从 1 开始；TRANSIENT 副本由 SQLite 分配，不复制成 Rust 业务文本。
    pub fn bind_text(&mut self, index: i32, value: &str) -> Result<()> {
        let len = i32::try_from(value.len()).map_err(|_| sql_error(ffi::SQLITE_TOOBIG))?;
        check(unsafe {
            ffi::sqlite3_bind_text(
                self.statement.as_ptr(),
                index,
                value.as_ptr().cast(),
                len,
                ffi::SQLITE_TRANSIENT(),
            )
        })
    }
    pub fn bind_integer(&mut self, index: i32, value: i64) -> Result<()> {
        check(unsafe { ffi::sqlite3_bind_int64(self.statement.as_ptr(), index, value) })
    }
    pub fn bind_real(&mut self, index: i32, value: f64) -> Result<()> {
        check(unsafe { ffi::sqlite3_bind_double(self.statement.as_ptr(), index, value) })
    }
    pub fn bind_blob(&mut self, index: i32, value: &[u8]) -> Result<()> {
        let len = i32::try_from(value.len()).map_err(|_| sql_error(ffi::SQLITE_TOOBIG))?;
        check(unsafe {
            ffi::sqlite3_bind_blob(
                self.statement.as_ptr(),
                index,
                value.as_ptr().cast(),
                len,
                ffi::SQLITE_TRANSIENT(),
            )
        })
    }
    pub fn bind_null(&mut self, index: i32) -> Result<()> {
        check(unsafe { ffi::sqlite3_bind_null(self.statement.as_ptr(), index) })
    }
    pub fn clear_bindings(&mut self) -> Result<()> {
        check(unsafe { ffi::sqlite3_clear_bindings(self.statement.as_ptr()) })
    }
    /// 下一行借用独占本 statement；列索引从 0 开始。
    /// 行或列借用活着时不能 step/reset/finalize。
    /// DONE 检查 reset 并返回 None；成功调用者须显式 finalize 检查终结，或 reset 重用。
    ///
    /// ```compile_fail
    /// use newchan_rust::sqlite_checked::CheckedStatement;
    /// fn invalid(s: &mut CheckedStatement<'_>) -> rusqlite::Result<()> {
    ///     let mut row = s.next()?.unwrap();
    ///     let text = row.text(0)?;
    ///     s.next()?;
    ///     println!("{text}");
    ///     Ok(())
    /// }
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<CheckedRow<'_>>> {
        if !self.readonly {
            return Err(sql_error(ffi::SQLITE_READONLY));
        }
        if self.finished {
            return Ok(None);
        }
        let code = unsafe { ffi::sqlite3_step(self.statement.as_ptr()) };
        match code {
            ffi::SQLITE_ROW => Ok(Some(CheckedRow {
                statement: self.statement,
                _borrow: PhantomData,
            })),
            ffi::SQLITE_DONE => {
                self.finished = true;
                check(unsafe { ffi::sqlite3_reset(self.statement.as_ptr()) })?;
                Ok(None)
            }
            cause => {
                self.finished = true;
                // Cleanup cannot replace the original step failure with reset's result.
                unsafe {
                    ffi::sqlite3_reset(self.statement.as_ptr());
                }
                Err(sql_error(cause))
            }
        }
    }
    /// 固定 INSERT 的唯一执行口；不开放写 statement 的行借用。
    pub fn execute(&mut self) -> Result<usize> {
        if self.readonly {
            return Err(sql_error(ffi::SQLITE_READONLY));
        }
        let code = unsafe { ffi::sqlite3_step(self.statement.as_ptr()) };
        if code != ffi::SQLITE_DONE {
            unsafe {
                ffi::sqlite3_reset(self.statement.as_ptr());
            }
            return Err(if code == ffi::SQLITE_ROW {
                Error::ExecuteReturnedResults
            } else {
                sql_error(code)
            });
        }
        let changed = unsafe { ffi::sqlite3_changes(self._connection.handle()) } as usize;
        check(unsafe { ffi::sqlite3_reset(self.statement.as_ptr()) })?;
        self.finished = true;
        Ok(changed)
    }
    pub fn reset(&mut self) -> Result<()> {
        let code = unsafe { ffi::sqlite3_reset(self.statement.as_ptr()) };
        self.finished = false;
        check(code)
    }
    /// 消耗唯一 guard，之后原 handle 不可再用。正常终结可检查返回值；
    /// 错误路径直接 Drop，保留调用者已取得的最初 cause。
    pub fn finalize(self) -> Result<()> {
        let out = std::mem::ManuallyDrop::new(self);
        check(unsafe { ffi::sqlite3_finalize(out.statement.as_ptr()) })
    }
}
impl Drop for CheckedStatement<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_finalize(self.statement.as_ptr());
        }
    }
}

/// 只由成功 SQLITE_ROW 构造；不开放 raw handle 或外来成功 token。
pub struct CheckedRow<'stmt> {
    statement: NonNull<ffi::sqlite3_stmt>,
    _borrow: PhantomData<&'stmt mut ffi::sqlite3_stmt>,
}
impl CheckedRow<'_> {
    pub fn column_count(&self) -> usize {
        unsafe { ffi::sqlite3_column_count(self.statement.as_ptr()) as usize }
    }
    fn kind(&self, column: usize) -> Result<Type> {
        if column >= self.column_count() {
            return Err(Error::InvalidColumnIndex(column));
        }
        let db = unsafe { ffi::sqlite3_db_handle(self.statement.as_ptr()) };
        let kind = unsafe { ffi::sqlite3_column_type(self.statement.as_ptr(), column as i32) };
        let code = unsafe { ffi::sqlite3_errcode(db) };
        if code & 255 == ffi::SQLITE_NOMEM {
            return Err(sql_error(code));
        }
        Ok(match kind {
            ffi::SQLITE_NULL => Type::Null,
            ffi::SQLITE_INTEGER => Type::Integer,
            ffi::SQLITE_FLOAT => Type::Real,
            ffi::SQLITE_TEXT => Type::Text,
            ffi::SQLITE_BLOB => Type::Blob,
            _ => return Err(sql_error(ffi::SQLITE_MISUSE)),
        })
    }
    fn require(&self, column: usize, expected: Type) -> Result<()> {
        let actual = self.kind(column)?;
        if actual == expected {
            Ok(())
        } else {
            // Empty column-name diagnostic owns no allocated business text.
            Err(Error::InvalidColumnType(column, String::new(), actual))
        }
    }
    /// 只读运行时类型；沿同一索引与 NOMEM 门，不开放 statement handle。
    pub fn column_type(&self, column: usize) -> Result<Type> {
        self.kind(column)
    }
    /// 严格 TEXT 的 checked 借用字节；UTF-8 语义判据留给调用者。
    pub fn text_bytes(&mut self, column: usize) -> Result<&[u8]> {
        self.require(column, Type::Text)?;
        unsafe { checked_column_text(self.statement.as_ptr(), column as i32) }?
            .ok_or_else(|| sql_error(ffi::SQLITE_MISUSE))
    }
    pub fn text(&mut self, column: usize) -> Result<&str> {
        let bytes = self.text_bytes(column)?;
        std::str::from_utf8(bytes).map_err(|cause| Error::Utf8Error(column, cause))
    }
    pub fn optional_text(&mut self, column: usize) -> Result<Option<&str>> {
        if self.kind(column)? == Type::Null {
            return Ok(None);
        }
        self.text(column).map(Some)
    }
    pub fn integer(&mut self, column: usize) -> Result<i64> {
        self.require(column, Type::Integer)?;
        Ok(unsafe { ffi::sqlite3_column_int64(self.statement.as_ptr(), column as i32) })
    }
    /// 严格 REAL：先校验类型与 NOMEM，原 double 位由调用者保留。
    pub fn real(&mut self, column: usize) -> Result<f64> {
        self.require(column, Type::Real)?;
        let value = unsafe { ffi::sqlite3_column_double(self.statement.as_ptr(), column as i32) };
        let db = unsafe { ffi::sqlite3_db_handle(self.statement.as_ptr()) };
        let code = unsafe { ffi::sqlite3_errcode(db) };
        if code & 255 == ffi::SQLITE_NOMEM {
            return Err(sql_error(code));
        }
        Ok(value)
    }
    pub fn optional_integer(&mut self, column: usize) -> Result<Option<i64>> {
        if self.kind(column)? == Type::Null {
            return Ok(None);
        }
        self.integer(column).map(Some)
    }
    pub fn blob(&mut self, column: usize) -> Result<&[u8]> {
        self.require(column, Type::Blob)?;
        let raw = self.statement.as_ptr();
        let db = unsafe { ffi::sqlite3_db_handle(raw) };
        let pointer = unsafe { ffi::sqlite3_column_blob(raw, column as i32) };
        let code = unsafe { ffi::sqlite3_errcode(db) };
        if code & 255 == ffi::SQLITE_NOMEM {
            return Err(sql_error(code));
        }
        let len = unsafe { ffi::sqlite3_column_bytes(raw, column as i32) };
        let code = unsafe { ffi::sqlite3_errcode(db) };
        if code & 255 == ffi::SQLITE_NOMEM {
            return Err(sql_error(code));
        }
        if len < 0 || (pointer.is_null() && len != 0) {
            return Err(sql_error(ffi::SQLITE_MISUSE));
        }
        if len == 0 {
            return Ok(&[]);
        }
        Ok(unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), len as usize) })
    }
    pub fn optional_blob(&mut self, column: usize) -> Result<Option<&[u8]>> {
        if self.kind(column)? == Type::Null {
            return Ok(None);
        }
        self.blob(column).map(Some)
    }
}

/// 唯一 checked TEXT primitive；转换后立即检查 NOMEM，先于 bytes/NULL解释。
/// # Safety
/// `stmt` 是已 step 到 SQLITE_ROW 的活 statement，column 在范围内；调用者把返回
/// 借用限制在该 row 生命周期内，借用活着时不得转换该列或 step/reset/finalize。
pub unsafe fn checked_column_text<'a>(
    stmt: *mut ffi::sqlite3_stmt,
    column: i32,
) -> Result<Option<&'a [u8]>> {
    let db = unsafe { ffi::sqlite3_db_handle(stmt) };
    let pointer = unsafe { ffi::sqlite3_column_text(stmt, column) };
    let code = unsafe { ffi::sqlite3_errcode(db) };
    if code & 255 == ffi::SQLITE_NOMEM {
        return Err(sql_error(code));
    }
    let len = unsafe { ffi::sqlite3_column_bytes(stmt, column) };
    let code = unsafe { ffi::sqlite3_errcode(db) };
    if code & 255 == ffi::SQLITE_NOMEM {
        return Err(sql_error(code));
    }
    if len < 0 || (pointer.is_null() && len != 0) {
        return Err(sql_error(ffi::SQLITE_MISUSE));
    }
    if pointer.is_null() {
        return Ok(None);
    }
    Ok(Some(unsafe {
        std::slice::from_raw_parts(pointer, len as usize)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_types_bindings_utf8_and_readonly_guard() {
        let connection = Connection::open_in_memory().unwrap();
        let mut query = CheckedStatement::prepare(&connection, c"SELECT ?1, ?2, ?3, ?4").unwrap();
        query.bind_text(1, "缠论\0UTF8").unwrap();
        query.bind_integer(2, i64::MAX).unwrap();
        query.bind_blob(3, &[0, 255]).unwrap();
        query.bind_null(4).unwrap();
        let mut row = query.next().unwrap().unwrap();
        assert_eq!(row.text(0).unwrap(), "缠论\0UTF8");
        assert_eq!(row.integer(1).unwrap(), i64::MAX);
        assert_eq!(row.blob(2).unwrap(), &[0, 255]);
        assert_eq!(row.optional_text(3).unwrap(), None);
        assert_eq!(row.optional_integer(3).unwrap(), None);
        assert_eq!(row.optional_blob(3).unwrap(), None);
        assert!(matches!(row.text(1), Err(Error::InvalidColumnType(..))));
        assert!(matches!(row.integer(0), Err(Error::InvalidColumnType(..))));
        assert!(matches!(row.blob(0), Err(Error::InvalidColumnType(..))));
        assert!(matches!(row.text(4), Err(Error::InvalidColumnIndex(4))));
        drop(row);
        assert!(query.next().unwrap().is_none());
        query.reset().unwrap();
        assert_eq!(
            query.next().unwrap().unwrap().text(0).unwrap(),
            "缠论\0UTF8"
        );
        query.finalize().unwrap();
        assert!(matches!(
            CheckedStatement::prepare(&connection, c"CREATE TABLE forbidden(x)"),
            Err(Error::SqliteFailure(..))
        ));
        assert!(matches!(
            CheckedStatement::prepare(&connection, c"SELECT 1; SELECT 2"),
            Err(Error::MultipleStatement)
        ));
    }

    #[cfg(feature = "s_allocation_sqlite_testonly")]
    #[test]
    #[ignore = "only this exact test in a fresh isolated SQLite process"]
    fn isolated_utf16_text_conversion_nomem_keeps_shared_owners() {
        use crate::s_content::capture_allocation::{
            sqlite::TestOnlySqliteAllocator, CaptureSlab, OwnedBudget,
        };
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let guard = unsafe { TestOnlySqliteAllocator::install(&budget) }.unwrap();
        let control = guard.control();
        let rust_owner = CaptureSlab::new(0, 4096, &budget).unwrap();
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "PRAGMA encoding='UTF-16le'; CREATE TABLE text_input(value TEXT NOT NULL);",
            )
            .unwrap();
        // The original tiny TEXT failure remains in lib-probe-02-r2.log. Its successful
        // access did not prove a refused global allocation: translation may fit lookaside.
        // This named fixture is real stored UTF16 TEXT; no CAST/BLOB or lookaside changes.
        const UTF16_FIXTURE_REPETITIONS: usize = 16_384;
        let fixture = "真实UTF16缠论".repeat(UTF16_FIXTURE_REPETITIONS);
        connection
            .execute(
                "INSERT INTO text_input(value) VALUES(?1)",
                [fixture.as_str()],
            )
            .unwrap();
        let encoding: String = connection
            .query_row("PRAGMA encoding", [], |row| row.get(0))
            .unwrap();
        assert_eq!(encoding, "UTF-16le");
        let mut query =
            CheckedStatement::prepare(&connection, c"SELECT value FROM text_input").unwrap();
        let mut row = query.next().unwrap().unwrap();
        assert_eq!(row.column_type(0).unwrap(), Type::Text);
        fn lookaside_size_misses(connection: &Connection) -> i32 {
            let mut current = 0;
            let mut highwater = 0;
            let code = unsafe {
                ffi::sqlite3_db_status(
                    connection.handle(),
                    ffi::SQLITE_DBSTATUS_LOOKASIDE_MISS_SIZE,
                    &mut current,
                    &mut highwater,
                    0,
                )
            };
            assert_eq!(code, ffi::SQLITE_OK);
            highwater
        }
        let miss_size_before_conversion = lookaside_size_misses(&connection);
        let before = (budget.snapshot(), budget.ledger_health());
        assert!(before.1.sqlite_live_layout_bytes > 0);
        assert!(before.0.live_requested_layout_bytes > before.1.sqlite_live_layout_bytes);
        // Real TEXT has been stepped, but no UTF-8 column accessor has run yet.
        control.refuse_sqlite_allocations_for_testonly();
        let error = row.text(0).unwrap_err();
        assert_eq!(error.sqlite_error_code(), Some(ffi::ErrorCode::OutOfMemory));
        assert_eq!(
            error.sqlite_error().unwrap().extended_code & 255,
            ffi::SQLITE_NOMEM
        );
        let refused = (budget.snapshot(), budget.ledger_health());
        let miss_size_after_conversion = lookaside_size_misses(&connection);
        assert!(
            miss_size_after_conversion > miss_size_before_conversion,
            "the actual conversion request must miss this runtime lookaside slot size"
        );
        assert!(refused.1.sqlite_rejected_layouts > before.1.sqlite_rejected_layouts);
        let request = budget.allocation_request_probe();
        assert!(request.refused_kind.starts_with("SQLite "));
        assert!(request.refused_bytes > fixture.len());
        assert_eq!(refused.0.allocated_layouts, before.0.allocated_layouts);
        assert_eq!(
            refused.0.live_requested_layout_bytes,
            before.0.live_requested_layout_bytes
        );
        drop(row);
        // SQLite may invalidate the errored VDBE/converted value; recover using a fresh SELECT.
        drop(query);
        control.clear_refusal_for_testonly();
        let mut retry =
            CheckedStatement::prepare(&connection, c"SELECT value FROM text_input").unwrap();
        assert_eq!(retry.next().unwrap().unwrap().text(0).unwrap(), fixture);
        retry.finalize().unwrap();
        drop(connection);
        drop(rust_owner);
        unsafe { guard.finish() }.unwrap();
        drop(control);
        let after = (budget.snapshot(), budget.ledger_health());
        assert_eq!(after.0.live_requested_layout_bytes, 0);
        assert_eq!(after.0.allocated_layouts, after.0.released_layouts);
        assert_eq!(after.1.sqlite_live_layout_bytes, 0);
        assert!(!after.1.invariant_broken);
        println!(
            "CHECKED_UTF16_TEXT {}",
            serde_json::json!({"fixture":"utf16_text_above_runtime_lookaside_16384_units",
                "fixture_repetitions":UTF16_FIXTURE_REPETITIONS,"fixture_utf8_bytes":fixture.len(),
                "lookaside_size_misses_before_conversion":miss_size_before_conversion,
                "lookaside_size_misses_after_conversion":miss_size_after_conversion,"request":request,
                "before":before,"refused":refused,"after":after,"error_code":ffi::SQLITE_NOMEM,
                "testonly_limit_bytes":50*1024*1024,"lookaside_configuration_changed":false,
                "original_small_fixture_log":"lib-probe-02-r2.log",
                "scope":"synthetic real UTF16 TEXT conversion at checked SELECT; does not replace actual UTF8 caller acceptance",
                "whole_audit_allocation_admitted":false})
        );
    }
}
