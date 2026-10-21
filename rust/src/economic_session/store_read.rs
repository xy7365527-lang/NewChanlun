//! #1406 C5 H2：SQL 返回载荷的共同读取账；不是 SQLite 页级物理 I/O。
//! TEXT/BLOB 取返回字节长度，Integer/Real 取已返回标量的固定宽度，Null 为零。
//! 同一行重读字节照计。SQL 行不是六族/Delta 逻辑记录，不增加 records。
//! 先实记返回字节，再解析/校验/报到期；逻辑 records 仍由原业务访问点登记。
use crate::s_content::job::Job;
use rusqlite::{types::ValueRef, Connection, Params, Row, Rows};
use std::time::Instant;

pub(crate) struct ReadBudget<'a> {
    pub(crate) job: Option<&'a mut Job>,
    pub(crate) deadline: Option<Instant>,
}
impl<'a> ReadBudget<'a> {
    pub(crate) fn new(job: Option<&'a mut Job>, deadline: Option<Instant>) -> Self {
        Self { job, deadline }
    }
    pub(crate) fn before(&mut self) -> Result<(), String> {
        if let Some(job) = self.job.as_deref_mut() {
            job.begin_first_read()
                .map_err(|e| format!("Incomplete：SQL 读取起钟/停止门：{e}"))?;
        }
        super::store::check_operation_deadline(self.job.as_deref_mut(), self.deadline)
    }
    pub(crate) fn next<'r, 'stmt>(
        &mut self,
        rows: &'r mut Rows<'stmt>,
    ) -> Result<Option<&'r Row<'stmt>>, String> {
        self.before()?;
        let row = rows.next().map_err(super::store::err)?;
        if let Some(row) = row {
            let mut bytes = 0u64;
            for i in 0..row.as_ref().column_count() {
                let len = match row.get_ref(i).map_err(super::store::err)? {
                    ValueRef::Text(v) | ValueRef::Blob(v) => v.len(),
                    ValueRef::Integer(_) => std::mem::size_of::<i64>(),
                    ValueRef::Real(_) => std::mem::size_of::<f64>(),
                    ValueRef::Null => 0,
                };
                bytes = bytes
                    .checked_add(u64::try_from(len).map_err(super::store::err)?)
                    .ok_or("InvalidDomain：SQL 返回载荷字节溢出")?;
            }
            #[cfg(test)]
            if self.job.is_some() {
                if let Some(at) = super::tb03c_c5_read_tests::read_returned(bytes) {
                    self.deadline = Some(self.deadline.map_or(at, |old| old.min(at)));
                }
            }
            if let Some(job) = self.job.as_deref_mut() {
                job.charge_wire_bytes(bytes)
                    .map_err(|e| format!("Incomplete：SQL 实读累计：{e}"))?;
            }
        }
        // 已发生的返回先记账；外部 ingress 期限也不能抢在记账之前返回。
        self.before()?;
        Ok(row)
    }
    pub(crate) fn optional<P, T, F>(
        &mut self,
        c: &Connection,
        sql: &str,
        params: P,
        decode: F,
    ) -> Result<Option<T>, String>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.before()?;
        let mut statement = c.prepare(sql).map_err(super::store::err)?;
        let mut rows = statement.query(params).map_err(super::store::err)?;
        self.next(&mut rows)?
            .map(decode)
            .transpose()
            .map_err(super::store::err)
    }
    pub(crate) fn one<P, T, F>(
        &mut self,
        c: &Connection,
        sql: &str,
        params: P,
        decode: F,
    ) -> Result<T, String>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.optional(c, sql, params, decode)?
            .ok_or_else(|| super::store::err(rusqlite::Error::QueryReturnedNoRows))
    }
    pub(crate) fn all<P, T, F>(
        &mut self,
        c: &Connection,
        sql: &str,
        params: P,
        mut decode: F,
    ) -> Result<Vec<T>, String>
    where
        P: Params,
        F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.before()?;
        let mut statement = c.prepare(sql).map_err(super::store::err)?;
        let mut rows = statement.query(params).map_err(super::store::err)?;
        let mut result = Vec::new();
        while let Some(row) = self.next(&mut rows)? {
            result.push(decode(row).map_err(super::store::err)?);
        }
        Ok(result)
    }
}
