//! #1419 M8B：封闭的同事务、同父、同生涯读回；没有 stage miss 或第二来源。
use super::*;
use capture_allocation::scratch::{NativeReservation, ScratchFailure};
use capture_allocation::sqlite_parent::{ChargedConnection, SqlGate, SqliteBinding};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, ToSql, Transaction};
use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::ControlFlow;

/// #1419 3C：受管 reader 在借入事务上实际执行的全部 SQL：事务与变更计数观测、固定只读标量
/// PRAGMA、按摘要的节点单行。rusqlite 事务与收费组件事务各实现一次，reader 的判据只有一份。
pub trait ReaderTx {
    fn in_autocommit(&self) -> bool;
    fn change_count(&self) -> u64;
    /// 本事务所在连接上已记下的首个收费拒绝（rusqlite 事务没有，恒为 `None`）。签发方门报错时
    /// 据此按值把错误归为工作域的同类拒绝，不经文字。
    fn refusal(&self) -> Option<ScratchFailure>;
    fn scalar(&self, sql: &'static str, work: &WorkScope) -> Result<i64, CodecError>;
    /// 取摘要对应的一行，`(tag, body)` 只在回调期内借出；无行为 `None`。
    fn node<R>(
        &self,
        digest: &NodeDigest,
        work: &WorkScope,
        f: impl FnOnce(i64, &[u8]) -> Result<R, CodecError>,
    ) -> Result<Option<R>, CodecError>;
}

/// 与 rusqlite 实现的 `CheckedStatement` 同一条语句。
const NODE_SELECT: &str = "SELECT tag, body FROM s_content_nodes WHERE digest=?1";

impl ReaderTx for Transaction<'_> {
    fn in_autocommit(&self) -> bool {
        Connection::is_autocommit(self)
    }
    fn change_count(&self) -> u64 {
        Connection::total_changes(self)
    }
    fn refusal(&self) -> Option<ScratchFailure> {
        None
    }
    fn scalar(&self, sql: &'static str, _work: &WorkScope) -> Result<i64, CodecError> {
        self.query_row(sql, [], |row| row.get::<_, i64>(0))
            .map_err(storage_error)
    }
    fn node<R>(
        &self,
        digest: &NodeDigest,
        _work: &WorkScope,
        f: impl FnOnce(i64, &[u8]) -> Result<R, CodecError>,
    ) -> Result<Option<R>, CodecError> {
        // CheckedStatement 的 row.blob 只借 SQLite 原行，未复制为 Rust Vec。
        let mut statement = crate::sqlite_checked::CheckedStatement::prepare(
            self,
            c"SELECT tag, body FROM s_content_nodes WHERE digest=?1",
        )
        .map_err(storage_error)?;
        statement.bind_blob(1, digest).map_err(storage_error)?;
        #[cfg(test)]
        tests::observe(self, "prepared");
        let material = match statement.next().map_err(storage_error)? {
            None => None,
            Some(mut row) => {
                let tag = row.integer(0).map_err(storage_error)?;
                let bytes = row.blob(1).map_err(storage_error)?;
                let value = f(tag, bytes)?;
                #[cfg(test)]
                tests::observe(self, "row_and_rust_copy");
                Some(value)
            }
        };
        statement.finalize().map_err(storage_error)?;
        #[cfg(test)]
        tests::observe(self, "finalized");
        Ok(material)
    }
}

/// #1419 3C：收费组件连接上的事务视图。连接须处于签发方开启的事务内（签发时核
/// `autocommit`）；同连接别名由签发方排除，与 rusqlite 事务同一合同。SQLite 的 pager 与
/// 语句内部分配经组件向同一父账收费，补上原 rusqlite 路径“不由本许可覆盖”的 M8B ⑥ 缺口。
pub struct ChargedReadTx<'b> {
    binding: &'b SqliteBinding,
    connection: &'b ChargedConnection,
}

impl<'b> ChargedReadTx<'b> {
    pub fn new(binding: &'b SqliteBinding, connection: &'b ChargedConnection) -> Self {
        Self {
            binding,
            connection,
        }
    }
    /// 本事务视图所在连接的身份；签发方据此核对 reader 确在其私有连接上。
    pub fn connection_id(&self) -> capture_allocation::sqlite_parent::ConnectionId {
        self.connection.id()
    }
}

/// 组件门的拒绝按原工作域分类：收费首拒沿原类别，归属不符按 Ledger；SQLite 结果码按存储错误。
fn gate_error(error: SqlGate, work: &WorkScope) -> CodecError {
    match error {
        SqlGate::Refused(failure) => work.reject(failure),
        SqlGate::WrongParent => work.reject(ScratchFailure::Ledger),
        SqlGate::Sqlite(code) => storage_error(code),
    }
}

impl ReaderTx for ChargedReadTx<'_> {
    fn in_autocommit(&self) -> bool {
        self.connection.is_autocommit()
    }
    fn change_count(&self) -> u64 {
        self.connection.total_changes()
    }
    fn refusal(&self) -> Option<ScratchFailure> {
        self.binding.refusal()
    }
    fn scalar(&self, sql: &'static str, work: &WorkScope) -> Result<i64, CodecError> {
        self.binding
            .run(self.connection, |q| {
                q.query_row(sql, [], |[v]| v.as_i64().ok())
            })
            .map_err(|e| gate_error(e, work))?
            .ok_or_else(|| storage_error("PRAGMA 结果不是整数"))
    }
    fn node<R>(
        &self,
        digest: &NodeDigest,
        work: &WorkScope,
        f: impl FnOnce(i64, &[u8]) -> Result<R, CodecError>,
    ) -> Result<Option<R>, CodecError> {
        let key: &[u8] = &digest[..];
        let mut f = Some(f);
        let mut row = None;
        self.binding
            .run(self.connection, |q| {
                q.prepare(NODE_SELECT)?
                    .for_each_row([&key as &dyn ToSql], |[tag, body]| {
                        row = Some(match (tag, body, f.take()) {
                            (ValueRef::Integer(tag), ValueRef::Blob(bytes), Some(f)) => {
                                f(tag, bytes)
                            }
                            _ => Err(storage_error("节点行类型不符")),
                        });
                        ControlFlow::Break(())
                    })
                    .map(drop)
            })
            .map_err(|e| gate_error(e, work))?;
        row.transpose()
    }
}

/// #1419 M8D 3A：签发方提供的生涯门。`check_live` 在每次域核（节点 SELECT 前）以本 reader
/// 独占的同一事务调用，由 owner 核其私有连接、快照/文件观测、材料与父身份；`sample` 只采样
/// 原绝对期限/取消。library 不构造生涯，也不接受“已认证”标志；判定全由签发方的真实核给出。
pub trait LiveGate<T: ?Sized> {
    fn check_live(&self, tx: &T) -> Result<(), CodecError>;
    fn sample(&self) -> Result<(), CodecError>;
}

/// 签发方在同一调用位次上只有一个门：M8B 测试差分沿用原期限回调（外生涯由其调用方 sample
/// 门负责），owner 签发的 reader 只用 `LiveGate`。两者不在同一 if-else 上互为兜底。
enum Gate<'w, T: ?Sized> {
    #[cfg(any(test, feature = "s_capture_native_testonly"))]
    Sample(&'w dyn Fn() -> Result<(), CodecError>),
    Live(&'w dyn LiveGate<T>),
}
impl<T: ?Sized> Clone for Gate<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for Gate<'_, T> {}
impl<T: ?Sized> Gate<'_, T> {
    fn live(&self, tx: &T) -> Result<(), CodecError> {
        match self {
            #[cfg(any(test, feature = "s_capture_native_testonly"))]
            Gate::Sample(sample) => sample(),
            Gate::Live(gate) => gate.check_live(tx),
        }
    }
    fn sample(&self) -> Result<(), CodecError> {
        match self {
            #[cfg(any(test, feature = "s_capture_native_testonly"))]
            Gate::Sample(sample) => sample(),
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
pub(super) struct ManagedSqlStep<'tx, 'conn, 'work, T: ReaderTx = Transaction<'conn>> {
    tx: &'tx mut T,
    _conn: PhantomData<&'conn ()>,
    work: &'work WorkScope,
    gate: Gate<'work, T>,
    identity: NativeReservation,
    live: Cell<bool>,
    changes: u64,
    schema: (i64, i64),
    failure: Option<CodecError>,
    pub(super) reads: u64,
    // 固定域元数据查询独立计数；reads/SQL_READS 只计与原 TxStore 同口径的节点 SELECT。
    pub(super) metadata_reads: Cell<u64>,
}

impl<'tx, 'conn, 'work, T: ReaderTx> ManagedSqlStep<'tx, 'conn, 'work, T> {
    #[cfg(any(test, feature = "s_capture_native_testonly"))]
    pub(super) fn new(
        tx: &'tx mut T,
        work: &'work WorkScope,
        sample: &'work dyn Fn() -> Result<(), CodecError>,
    ) -> Result<Self, CodecError> {
        Self::issue(tx, work, Gate::Sample(sample))
    }

    pub(super) fn with_gate(
        tx: &'tx mut T,
        work: &'work WorkScope,
        gate: &'work dyn LiveGate<T>,
    ) -> Result<Self, CodecError> {
        Self::issue(tx, work, Gate::Live(gate))
    }

    fn issue(
        tx: &'tx mut T,
        work: &'work WorkScope,
        gate: Gate<'work, T>,
    ) -> Result<Self, CodecError> {
        gate.live(tx).map_err(|e| typed_gate_error(e, &*tx, work))?;
        work.check_current()?;
        if tx.in_autocommit() {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        let metadata_reads = Cell::new(0);
        let (read_uncommitted, main_schema, temp_schema) =
            sql_identity(&*tx, work, &metadata_reads)?;
        if read_uncommitted != 0 {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        let identity = work.reserve_native(0)?;
        let changes = tx.change_count();
        Ok(Self {
            tx,
            _conn: PhantomData,
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
    pub(super) fn tx_for_test(&self) -> &T {
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
            || self.tx.in_autocommit()
            || !self.identity.belongs_to_parent(work)
            || self.tx.change_count() != self.changes
        {
            return Err(work.reject(ScratchFailure::Ledger));
        }
        self.gate
            .live(self.tx)
            .map_err(|e| typed_gate_error(e, &*self.tx, work))?;
        let (read_uncommitted, main_schema, temp_schema) =
            sql_identity(&*self.tx, work, &self.metadata_reads)?;
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

/// 签发方门报错时，事务所在连接若已记下收费首拒，按值归为工作域的同类拒绝（与节点读的
/// `gate_error` 同一归类）；否则原样返回门的错误。rusqlite 事务没有收费首拒，原样返回。
fn typed_gate_error<T: ReaderTx>(error: CodecError, tx: &T, work: &WorkScope) -> CodecError {
    match tx.refusal() {
        Some(failure) => work.reject(failure),
        None => error,
    }
}

fn storage_error(error: impl std::fmt::Display) -> CodecError {
    CodecError::new(code::SOURCE, codec_detail!("S3 内容存储：{error}"))
}

fn sql_identity<T: ReaderTx>(
    tx: &T,
    work: &WorkScope,
    reads: &Cell<u64>,
) -> Result<(i64, i64, i64), CodecError> {
    let scalar = |sql| {
        reads.set(reads.get().saturating_add(1));
        tx.scalar(sql, work)
    };
    Ok((
        scalar("PRAGMA read_uncommitted")?,
        scalar("PRAGMA main.schema_version")?,
        scalar("PRAGMA temp.schema_version")?,
    ))
}

impl<T: ReaderTx> NodeSource for ManagedSqlStep<'_, '_, '_, T> {
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
impl<T: ReaderTx> ManagedSqlStep<'_, '_, '_, T> {
    fn read_frame(&mut self, digest: &NodeDigest) -> Result<Option<job::ScopedFrame>, CodecError> {
        self.check_domain(self.work)?;
        self.reads = self.reads.saturating_add(1);
        #[cfg(feature = "s_capture_native_testonly")]
        SQL_READS.with(|reads| reads.set(reads.get().saturating_add(1)));
        // SQLite pager/statement 内部 allocation：rusqlite 事务上不由本许可覆盖（M8B ⑥ 未证明项）；
        // 收费组件事务上经组件向同一父账收费。
        let (gate, work) = (self.gate, self.work);
        let material = self.tx.node(digest, work, |tag, bytes| {
            // 读前许可：首次 Rust 行正文 allocation 之前沿原 work/origin 预留。
            // CheckedFrame/CheckedNodeUsage 内联在栈和租约中，不另开 heap 证明表。
            gate.sample()?;
            let permit = work.reserve_native(bytes.len())?;
            let mut body = Vec::new();
            body.try_reserve_exact(bytes.len())
                .map_err(|_| work.reject(ScratchFailure::Allocation))?;
            body.extend_from_slice(bytes);
            Ok(SqlMaterial { tag, body, permit })
        })?;
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
pub struct GatedSqlReader<'tx, 'conn, 'work, T: ReaderTx = Transaction<'conn>> {
    inner: ManagedSqlStep<'tx, 'conn, 'work, T>,
}
impl<'tx, 'conn, 'work, Tx: ReaderTx> GatedSqlReader<'tx, 'conn, 'work, Tx> {
    pub fn new(
        tx: &'tx mut Tx,
        work: &'work WorkScope,
        gate: &'work dyn LiveGate<Tx>,
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
    /// 步出口（M8D §1.2(4)）：交付前再过一次同一域核与签发方门（期限/取消、生涯）；
    /// 已有首错原样返回，出口失败同样记为首错，之后零 SELECT。
    pub fn exit(&mut self) -> Result<(), CodecError> {
        let result = self.inner.check_domain(self.inner.work);
        if let Err(error) = &result {
            if self.inner.failure.is_none() {
                self.inner.failure = Some(error.clone());
            }
            self.inner.live.set(false);
        }
        result
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
