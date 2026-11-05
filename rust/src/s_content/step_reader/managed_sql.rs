//! #1419 M8B：封闭的同事务、同父、同生涯读回；没有 stage miss 或第二来源。
use super::*;
use capture_allocation::scratch::{NativeReservation, ScratchFailure};
use rusqlite::Transaction;
use std::cell::Cell;

/// #1419 M8D 3A：签发方提供的生涯门。`check_live` 在每次域核（节点 SELECT 前）以本 reader
/// 独占的同一事务调用，由 owner 核其私有连接、快照/文件观测、材料与父身份；`sample` 只采样
/// 原绝对期限/取消。library 不构造生涯，也不接受“已认证”标志；判定全由签发方的真实核给出。
#[cfg(feature = "s_capture_native_testonly")]
pub trait LiveGate {
    fn check_live(&self, tx: &Transaction<'_>) -> Result<(), CodecError>;
    fn sample(&self) -> Result<(), CodecError>;
}

/// 签发方在同一调用位次上只有一个门：M8B 测试差分沿用原期限回调（外生涯由其调用方 sample
/// 门负责），owner 签发的 reader 只用 `LiveGate`。两者不在同一 if-else 上互为兜底。
#[derive(Clone, Copy)]
enum Gate<'w> {
    Sample(&'w dyn Fn() -> Result<(), CodecError>),
    #[cfg(feature = "s_capture_native_testonly")]
    Live(&'w dyn LiveGate),
}
impl Gate<'_> {
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(unused_variables))]
    fn live(&self, tx: &Transaction<'_>) -> Result<(), CodecError> {
        match self {
            Gate::Sample(sample) => sample(),
            #[cfg(feature = "s_capture_native_testonly")]
            Gate::Live(gate) => gate.check_live(tx),
        }
    }
    fn sample(&self) -> Result<(), CodecError> {
        match self {
            Gate::Sample(sample) => sample(),
            #[cfg(feature = "s_capture_native_testonly")]
            Gate::Live(gate) => gate.sample(),
        }
    }
}

/// 独占事务借用（#1419 R1-01 返修，回到 M8B 原合同）：reader 存活期内任何 Rust 调用方都拿不到
/// 同一 Transaction 的另一借用，不能 Commit、换事务或执行事务控制 SQL；读者自身也不暴露 tx。
/// 事务能力的封闭来自所有权而不是统计观测：底层 Connection 的别名只能由签发方排除，
/// 生产签发方是持私有连接的 owner（M8D §1.2 生涯封印，3A），非测试调用点在 3C 之前不接。
/// total_changes/schema/隔离夹持只拒测试内部经本 reader 注入的同事务变化。
/// 外连接身份/生涯夹持由签发方的门负责（M8B 回调或 3A owner `LiveGate`）；原父许可只作 O(1)
/// 域身份封印。门必须借本请求原绝对 deadline/取消，不能在这里重新起算期限。
pub(super) struct ManagedSqlStep<'tx, 'conn, 'work> {
    tx: &'tx mut Transaction<'conn>,
    work: &'work WorkScope,
    gate: Gate<'work>,
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
        tx: &'tx mut Transaction<'conn>,
        work: &'work WorkScope,
        sample: &'work dyn Fn() -> Result<(), CodecError>,
    ) -> Result<Self, CodecError> {
        Self::issue(tx, work, Gate::Sample(sample))
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn with_gate(
        tx: &'tx mut Transaction<'conn>,
        work: &'work WorkScope,
        gate: &'work dyn LiveGate,
    ) -> Result<Self, CodecError> {
        Self::issue(tx, work, Gate::Live(gate))
    }

    fn issue(
        tx: &'tx mut Transaction<'conn>,
        work: &'work WorkScope,
        gate: Gate<'work>,
    ) -> Result<Self, CodecError> {
        gate.live(tx)?;
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
            gate,
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

    /// 测试内部注入入口：只给同 crate 测试取得 reader 独占的事务；生产没有 tx getter。
    #[cfg(test)]
    pub(super) fn tx_for_test(&self) -> &Transaction<'conn> {
        self.tx
    }

    /// 只核签发时独占借入的事务；原 `ptr::eq(self.tx, tx)` 恒以 self.tx 自比，已删去，
    /// 同一事务由独占借用的类型保证，不再传入第二个事务参数。
    fn check_domain(&self, work: &WorkScope) -> Result<(), CodecError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        work.check_current()?;
        if !self.live.get()
            || self.tx.is_autocommit()
            || !self.identity.belongs_to_parent(work)
            || self.tx.total_changes() != self.changes
        {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        self.gate.live(self.tx)?;
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
        if let Err(error) = self.check_domain(self.work) {
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
        self.check_domain(self.work)?;
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
                self.gate.sample()?;
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
        self.gate.sample()?;
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
    tx: &mut Transaction<'_>,
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
    tx: &mut Transaction<'_>,
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

/// #1419 M8D 3A：owner 工厂签发的受管 SQL reader。只能由持有 `&mut Transaction` 的签发方
/// 构造，门由签发方给出；不暴露 tx/Connection，不提供 Clone、跨步句柄或正文 into。
#[cfg(feature = "s_capture_native_testonly")]
pub struct GatedSqlReader<'tx, 'conn, 'work> {
    inner: ManagedSqlStep<'tx, 'conn, 'work>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl<'tx, 'conn, 'work> GatedSqlReader<'tx, 'conn, 'work> {
    pub fn new(
        tx: &'tx mut Transaction<'conn>,
        work: &'work WorkScope,
        gate: &'work dyn LiveGate,
    ) -> Result<Self, CodecError> {
        Ok(Self {
            inner: ManagedSqlStep::with_gate(tx, work, gate)?,
        })
    }
    /// 单帧点查：先过签发方生涯门，再同事务 SELECT、读前许可与 F；首错保持，失活后零 SELECT。
    pub fn frame<T>(
        &mut self,
        digest: &NodeDigest,
        f: impl FnOnce(u8, &[u8]) -> T,
    ) -> Result<Option<T>, CodecError> {
        self.inner
            .checked_read_scoped(digest)
            .map(|frame| frame.map(|frame| f(frame.tag(), frame.body())))
    }
    /// 根租约：原 Verifier 完整核后交付；正文只在回调借用期内可见。
    pub fn root<T>(
        &mut self,
        reference: RefView<'_>,
        mode: Mode,
        f: impl FnOnce(u8, &[u8]) -> T,
    ) -> Result<T, CodecError> {
        let lease = self.inner.read_checked(reference, mode)?;
        Ok(f(lease.tag, &lease))
    }
    pub fn node_selects(&self) -> u64 {
        self.inner.reads
    }
    pub fn metadata_reads(&self) -> u64 {
        self.inner.metadata_reads.get()
    }
}

#[cfg(test)]
mod tests;
