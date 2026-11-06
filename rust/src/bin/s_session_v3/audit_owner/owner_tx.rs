//! #1419 3C effects S1：effects 写记录与内容节点存储（`TxStore`）在 owner 事务上实际执行的
//! SQL 面。
//!
//! `Legacy` 就是现行事务所在的 rusqlite 连接：调用方在 Legacy 分支里逐字保留原 rusqlite 调用，
//! SQL、次序与错误文本不变。`Charged` 是受管 owner 守卫（U5），目前只由全新进程 release 锁消费，
//! cfg 与 `owner_sql` 收费一族一致；Native 一次启用波随生产调用点一起放宽。
//!
//! 本面不持有任何 Rust 堆：两个变体都是借用，行视图借自语句当前行。
// 非增量构建没有 effects 消费者（与 `owner_sql` 同一写法）。
#![cfg_attr(not(feature = "s_incremental"), allow(dead_code))]
#[cfg(feature = "s_incremental")]
use rusqlite::types::{FromSql, ValueRef};
use rusqlite::{Connection, Transaction};

/// owner 事务所在连接（借用）。
#[derive(Clone, Copy)]
pub(crate) enum OwnerTx<'a> {
    Legacy(&'a Connection),
    #[cfg(all(test, feature = "s_incremental", unix))]
    Charged(&'a super::owner_sql::ChargedOwnerTxn<'a>),
}

impl<'a> From<&'a Transaction<'_>> for OwnerTx<'a> {
    fn from(tx: &'a Transaction<'_>) -> Self {
        Self::Legacy(tx)
    }
}

impl<'a> From<&'a Connection> for OwnerTx<'a> {
    fn from(conn: &'a Connection) -> Self {
        Self::Legacy(conn)
    }
}

impl OwnerTx<'_> {
    /// 连接打开以来的累计变更行数。只读连接状态，不执行 SQL、不分配。
    pub(crate) fn total_changes(&self) -> u64 {
        match self {
            Self::Legacy(conn) => conn.total_changes(),
            #[cfg(all(test, feature = "s_incremental", unix))]
            Self::Charged(txn) => txn.connection().total_changes(),
        }
    }
    /// 最近一条完成语句的直接变更行数。只读连接状态，不执行 SQL、不分配。
    #[cfg(feature = "s_incremental")]
    pub(crate) fn changes(&self) -> u64 {
        match self {
            Self::Legacy(conn) => conn.changes(),
            #[cfg(all(test, feature = "s_incremental", unix))]
            Self::Charged(txn) => txn.connection().changes(),
        }
    }
    /// effects 节点 journal 的连接键：rusqlite 连接为句柄地址（原口径），受管连接为不复用的
    /// `ConnectionId`。
    #[cfg(feature = "s_incremental")]
    pub(in crate::v3) fn journal_key(&self) -> super::owner_sql::JournalKey {
        match self {
            // SAFETY: the handle is only compared as an address, never dereferenced.
            Self::Legacy(conn) => {
                super::owner_sql::JournalKey::Handle(unsafe { conn.handle() } as usize)
            }
            #[cfg(all(test, feature = "s_incremental", unix))]
            Self::Charged(txn) => super::owner_sql::JournalKey::Charged(txn.connection().id()),
        }
    }
}

/// 一行的列借用：rusqlite 行，或受管语句当前行的列值。只在取行回调期内有效。
#[cfg(feature = "s_incremental")]
pub(crate) enum OwnerRow<'r> {
    Legacy(&'r rusqlite::Row<'r>),
    #[cfg(all(test, feature = "s_incremental", unix))]
    Charged(&'r [ValueRef<'r>]),
}

#[cfg(feature = "s_incremental")]
impl OwnerRow<'_> {
    /// 按列取值。Legacy 即 `rusqlite::Row::get`；受管行按同一 `FromSql` 转换并按 rusqlite 同一
    /// 映射报错（受管语句不交出列名，类型错误中的列名为空）。
    pub(crate) fn get<T: FromSql>(&self, index: usize) -> rusqlite::Result<T> {
        match self {
            Self::Legacy(row) => row.get(index),
            #[cfg(all(test, feature = "s_incremental", unix))]
            Self::Charged(cells) => {
                let value = cells
                    .get(index)
                    .copied()
                    .ok_or(rusqlite::Error::InvalidColumnIndex(index))?;
                T::column_result(value).map_err(|error| conversion_error(index, value, error))
            }
        }
    }
}

/// `rusqlite::Row::get` 的同一错误映射（rusqlite 0.40.2 `row.rs:288-302`）。
#[cfg(all(test, feature = "s_incremental", unix))]
fn conversion_error(
    index: usize,
    value: ValueRef<'_>,
    error: rusqlite::types::FromSqlError,
) -> rusqlite::Error {
    use rusqlite::types::FromSqlError;
    match error {
        FromSqlError::InvalidType => {
            rusqlite::Error::InvalidColumnType(index, String::new(), value.data_type())
        }
        FromSqlError::OutOfRange(n) => rusqlite::Error::IntegralValueOutOfRange(index, n),
        FromSqlError::Utf8Error(error) => rusqlite::Error::Utf8Error(index, error),
        FromSqlError::Other(error) => {
            rusqlite::Error::FromSqlConversionFailure(index, value.data_type(), error)
        }
        error => {
            rusqlite::Error::FromSqlConversionFailure(index, value.data_type(), Box::new(error))
        }
    }
}
