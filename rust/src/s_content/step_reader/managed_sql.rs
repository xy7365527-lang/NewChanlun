//! #1419 M8B：封闭的同事务、同父、同生涯读回；没有 stage miss 或第二来源。
use super::*;
use capture_allocation::scratch::{NativeReservation, ScratchFailure};
use rusqlite::Transaction;
use std::cell::Cell;

/// 共享事务借用（#1419 R1）：生产材料切面只持 `&Transaction`（effects MaterialAccess），
/// 同一步写者也借同一事务。借用期内 Transaction 不能被 Commit 消费或替换；
/// 经共享句柄执行的写入由 total_changes 夹持拒绝，COMMIT/ROLLBACK 由 autocommit 拒绝，
/// DDL/PRAGMA/TEMP 影子由 schema 与 read_uncommitted 夹持拒绝；三者都在每次节点 SELECT 前核。
/// 外连接身份/生涯夹持由原 sample 门负责；原父许可只作 O(1) 域身份封印。
/// sample 必须借本请求原绝对 deadline/取消门，不能在这里重新起算期限。
pub(super) struct ManagedSqlStep<'tx, 'conn, 'work> {
    tx: &'tx Transaction<'conn>,
    work: &'work WorkScope,
    sample: &'work dyn Fn() -> Result<(), CodecError>,
    identity: NativeReservation,
    live: Cell<bool>,
    changes: u64,
    schema: (i64, i64),
    failure: Option<CodecError>,
    pub(super) reads: u64,
    // 固定域元数据查询独立计数；reads/SQL_READS 只计与原 TxStore 同口径的节点 SELECT。
    pub(super) metadata_reads: Cell<u64>,
}

impl<'tx, 'conn, 'work> ManagedSqlStep<'tx, 'conn, 'work> {
    pub(super) fn new(
        tx: &'tx Transaction<'conn>,
        work: &'work WorkScope,
        sample: &'work dyn Fn() -> Result<(), CodecError>,
    ) -> Result<Self, CodecError> {
        sample()?;
        work.check_current()?;
        if tx.is_autocommit() {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        let metadata_reads = Cell::new(0);
        let (read_uncommitted, main_schema, temp_schema) = sql_identity(tx, &metadata_reads)?;
        if read_uncommitted != 0 {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        let identity = work.reserve_native(0)?;
        let changes = tx.total_changes();
        Ok(Self {
            tx,
            work,
            sample,
            identity,
            live: Cell::new(true),
            changes,
            schema: (main_schema, temp_schema),
            failure: None,
            reads: 0,
            metadata_reads,
        })
    }

    /// R1 单一入口的同域核：只认构造时借入的同一活动工作域，不另签身份。
    #[cfg(test)]
    pub(super) fn bound_to(&self, work: &WorkScope) -> bool {
        std::ptr::eq(self.work, work)
    }

    fn check_domain(&self, tx: &Transaction<'_>, work: &WorkScope) -> Result<(), CodecError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        work.check_current()?;
        if !self.live.get()
            || self.tx.is_autocommit()
            || !std::ptr::eq(self.tx, tx)
            || !self.identity.belongs_to_parent(work)
            || self.tx.total_changes() != self.changes
        {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        (self.sample)()?;
        let (read_uncommitted, main_schema, temp_schema) =
            sql_identity(self.tx, &self.metadata_reads)?;
        if read_uncommitted != 0 || (main_schema, temp_schema) != self.schema {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        Ok(())
    }

    pub(super) fn read_checked<'lease>(
        &'lease mut self,
        reference: RefView<'lease>,
        mode: Mode,
    ) -> Result<SqlReadLease<'lease>, CodecError> {
        if let Err(error) = self.check_domain(self.tx, self.work) {
            if self.failure.is_none() {
                self.failure = Some(error.clone());
            }
            return Err(error);
        }
        // 使用真实 NodeSource/Verifier 核；第一次可达、Flavor 和四字段均由原核判。
        let verified = {
            let mut verifier = Verifier::with_context(
                CodecContext::Source(&mut *self),
                mode,
                AuditMemo::default(),
            );
            verifier.verify_refview(reference)
        };
        if let Err(error) = verified {
            if self.failure.is_none() {
                self.failure = Some(error.clone());
            }
            return Err(error);
        }
        let digest = reference.digest()?;
        let (tag, body) = read_scoped_checked(self, &digest)?;
        let VerifierBody::Scoped(body) = body else {
            unreachable!()
        };
        Ok(SqlReadLease {
            #[cfg(test)]
            reference,
            #[cfg(test)]
            mode,
            tag,
            body: Some(body),
            _reader: std::marker::PhantomData,
            #[cfg(test)]
            drop_observer: None,
        })
    }
}

fn storage_error(error: impl std::fmt::Display) -> CodecError {
    CodecError::new(code::SOURCE, format!("S3 内容存储：{error}"))
}

fn sql_identity(tx: &Transaction<'_>, reads: &Cell<u64>) -> Result<(i64, i64, i64), CodecError> {
    let scalar = |sql| {
        reads.set(reads.get().saturating_add(1));
        tx.query_row(sql, [], |row| row.get::<_, i64>(0))
            .map_err(storage_error)
    };
    Ok((
        scalar("PRAGMA read_uncommitted")?,
        scalar("PRAGMA main.schema_version")?,
        scalar("PRAGMA temp.schema_version")?,
    ))
}

impl NodeSource for ManagedSqlStep<'_, '_, '_> {
    fn codec_work(&self) -> Option<&WorkScope> {
        Some(self.work)
    }

    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        // 禁止将带许可正文剥成裸 Vec；所有 adapter 调用者使用 scoped 入口。
        let _ = digest;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let error = self.work.reject(ScratchFailure::Ledger);
        self.failure = Some(error.clone());
        self.live.set(false);
        Err(error)
    }

    fn checked_read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::ScopedFrame>, CodecError> {
        let result = self.read_frame(digest);
        if let Err(error) = &result {
            if self.failure.is_none() {
                self.failure = Some(error.clone());
            }
            self.live.set(false);
        }
        result
    }
}
impl ManagedSqlStep<'_, '_, '_> {
    fn read_frame(&mut self, digest: &NodeDigest) -> Result<Option<job::ScopedFrame>, CodecError> {
        self.check_domain(self.tx, self.work)?;
        self.reads = self.reads.saturating_add(1);
        #[cfg(feature = "s_capture_native_testonly")]
        SQL_READS.with(|reads| reads.set(reads.get().saturating_add(1)));
        // CheckedStatement 的 row.blob 只借 SQLite 原行，未复制为 Rust Vec。
        // SQLite pager/statement 内部 allocation 不由本许可覆盖，仍是 M8B ⑥ 未证明项。
        let mut statement = crate::sqlite_checked::CheckedStatement::prepare(
            self.tx,
            c"SELECT tag, body FROM s_content_nodes WHERE digest=?1",
        )
        .map_err(storage_error)?;
        statement.bind_blob(1, digest).map_err(storage_error)?;
        let material = match statement.next().map_err(storage_error)? {
            None => None,
            Some(mut row) => {
                let tag = row.integer(0).map_err(storage_error)?;
                let bytes = row.blob(1).map_err(storage_error)?;
                // 读前许可：首次 Rust 行正文 allocation 之前沿原 work/origin 预留。
                // CheckedFrame/CheckedNodeUsage 内联在栈和租约中，不另开 heap 证明表。
                (self.sample)()?;
                let permit = self.work.reserve_native(bytes.len())?;
                let mut body = Vec::new();
                body.try_reserve_exact(bytes.len())
                    .map_err(|_| self.work.reject(ScratchFailure::Allocation))?;
                body.extend_from_slice(bytes);
                Some(SqlMaterial { tag, body, permit })
            }
        };
        statement.finalize().map_err(storage_error)?;
        let result = material
            .map(|material| {
                let tag = u8::try_from(material.tag)
                    .map_err(|_| CodecError::new(code::NODE_DIGEST, "节点 tag 超过一个字节"))?;
                let SqlMaterial { body, permit, .. } = material;
                let frame = job::CheckedFrame::checked(digest, tag, body)?;
                Ok(job::ScopedFrame::new(frame, Some(permit)))
            })
            .transpose()?;
        // F 后采样原绝对门，失败直接丢弃本次 allocation，不换源或重试。
        (self.sample)()?;
        Ok(result)
    }
}

#[cfg(feature = "s_capture_native_testonly")]
thread_local! { static SQL_READS: Cell<u64> = const { Cell::new(0) }; }
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn sql_reads_on_thread() -> u64 {
    SQL_READS.with(Cell::get)
}

/// 独占借 reader，不提供 Clone/into_body。正文真析构后才归还原 origin 许可。
pub(super) struct SqlReadLease<'lease> {
    #[cfg(test)]
    pub(super) reference: RefView<'lease>,
    #[cfg(test)]
    pub(super) mode: Mode,
    pub(super) tag: u8,
    body: Option<job::ScopedBody>,
    _reader: std::marker::PhantomData<&'lease mut ()>,
    #[cfg(test)]
    drop_observer: Option<&'lease std::sync::atomic::AtomicBool>,
}

// 错误路径保持聚合所有权，tag 转换或 finalize 失败时正文也先于许可析构。
struct SqlMaterial {
    tag: i64,
    body: Vec<u8>,
    permit: NativeReservation,
}
impl std::ops::Deref for SqlReadLease<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.body.as_ref().expect("live SQL lease")
    }
}
impl Drop for SqlReadLease<'_> {
    fn drop(&mut self) {
        if let Some(body) = self.body.take() {
            let (bytes, permit) = body.into_parts();
            drop(bytes);
            #[cfg(test)]
            if let Some(observer) = self.drop_observer {
                observer.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            drop(permit);
        }
    }
}

#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn with_sql_frame<T>(
    tx: &Transaction<'_>,
    work: &WorkScope,
    sample: &dyn Fn() -> Result<(), CodecError>,
    digest: &NodeDigest,
    f: impl FnOnce(u8, &[u8]) -> T,
) -> Result<Option<T>, CodecError> {
    let mut reader = ManagedSqlStep::new(tx, work, sample)?;
    reader
        .checked_read_scoped(digest)
        .map(|frame| frame.map(|frame| f(frame.tag(), frame.body())))
}

#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn with_sql_root<T>(
    tx: &Transaction<'_>,
    work: &WorkScope,
    sample: &dyn Fn() -> Result<(), CodecError>,
    reference: RefView<'_>,
    mode: Mode,
    f: impl FnOnce(u8, &[u8]) -> T,
) -> Result<T, CodecError> {
    let mut reader = ManagedSqlStep::new(tx, work, sample)?;
    let lease = reader.read_checked(reference, mode)?;
    Ok(f(lease.tag, &lease))
}

#[cfg(test)]
mod tests;
